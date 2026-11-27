//! 🌉 Étape T1 : la chart complète (la version WASM de `/app/`) tourne dans la coque. La page n'a aucun réseau (CSP) ;
//! ce qu'elle ferait elle-même passe ici, dans le cœur Rust, sans Tauri (testable seul) :
//! - `reseau` : une requête HTTP(S) de la liste blanche d'Argos (`net.rs`), avec le client natif (taille maximale) ;
//! - `module` : un module du kit Argos lancé dans la bulle du cœur, au lieu du Web Worker de la page. Mêmes
//!   vérifications que l'app native : manifeste vérifié, `.wasm` vérifié sans le lancer, empreinte SHA-384 = celle
//!   acceptée par Jee dans la page (`identifiant@empreinte`) ; **aucun droit** accordé dans la coque (ni fichiers, ni
//!   réseau, ni IA), carburant, mémoire et temps limités par la bulle ;
//! - le direct (`lib.rs`) : le cœur ouvre le WebSocket et renvoie chaque message par un canal.
//!
//! Les clés n'arrivent qu'à l'étape T2 (coffre du système) : dans la coque, rien ne lit ni ne garde de clé.

#![forbid(unsafe_code)]

use argos_data::Http;
use argos_module_host::{engine::Engine, scan, sha384, Input, Manifest};
use std::collections::HashMap;
use std::sync::Mutex;

/// 🌐 Une requête de la page : GET, ou POST JSON si `corps` est donné. Refusée hors de la liste blanche.
pub async fn reseau(url: &str, corps: Option<&str>) -> Result<String, String> {
    argos_data::net::verifier(url).map_err(|e| e.to_string())?;
    let c = argos_data::rt::Client::default();
    match corps {
        Some(b) => c.post_json(url, b).await,
        None => c.get(url).await,
    }
    .map_err(|e| e.to_string())
}

/// Les `.wasm` déjà reçus, par `identifiant@empreinte` (la page n'envoie le fichier qu'une fois).
#[derive(Default)]
pub struct Modules(Mutex<HashMap<String, Vec<u8>>>);

/// Au plus 64 modules différents gardés en mémoire.
const MAX_MODULES: usize = 64;

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn hex48(s: &str) -> Option<[u8; 48]> {
    if s.len() != 96 {
        return None;
    }
    let mut out = [0u8; 48];
    for (i, b) in out.iter_mut().enumerate() {
        *b = u8::from_str_radix(s.get(2 * i..2 * i + 2)?, 16).ok()?;
    }
    Some(out)
}

impl Modules {
    /// 🧩 Un appel de module : `cle` = `identifiant@empreinte` acceptée ; `wasm` (base64) vide si déjà envoyé ;
    /// `entree` = l'entrée du kit (base64). Rend la sortie du kit (base64), que la page vérifie à son tour.
    pub fn lancer(&self, cle: &str, wasm: &str, manifeste: &str, entree: &str) -> Result<String, String> {
        let (id, empreinte) = cle.rsplit_once('@').ok_or("clé du module illisible")?;
        let accord = hex48(empreinte).ok_or("accord du module illisible : réinstalle-le")?;
        let fichier = if wasm.is_empty() {
            let m = self.0.lock().map_err(|_| "bulle indisponible")?;
            // « recharger » : la page renvoie le fichier (le cœur a été relancé)
            m.get(cle).cloned().ok_or("module à recharger")?
        } else {
            let w = b64_decode(wasm).ok_or("fichier du module illisible")?;
            if sha384(&w) != accord {
                return Err("le fichier du module a changé depuis ton accord : réinstalle-le".into());
            }
            scan(&w)?;
            let mut m = self.0.lock().map_err(|_| "bulle indisponible")?;
            if m.len() >= MAX_MODULES {
                m.clear();
            }
            m.insert(cle.to_string(), w.clone());
            w
        };
        let m = Manifest::parse(manifeste)?;
        m.check()?;
        if m.path != id {
            return Err(format!("{id} : le manifeste parle d'un autre module ({})", m.path));
        }
        let input = Input::decode(&b64_decode(entree).ok_or("entrée illisible")?).map_err(|e| format!("entrée refusée ({e:?})"))?;
        // aucun droit dans la coque : le dossier privé n'est jamais créé ; `~/.argos-signature` reste protégé
        let dossier = std::env::temp_dir().join("argos-tauri-modules");
        let protege: Vec<std::path::PathBuf> = std::env::var_os("HOME")
            .map(|h| std::path::PathBuf::from(h).join(".argos-signature"))
            .into_iter()
            .collect();
        let out = Engine::new()?.run(&m, &fichier, &accord, &[], &dossier, &protege, &input)?;
        Ok(b64_encode(&out.encode()))
    }
}

/// L'empreinte d'un `.wasm`, comme la page l'écrit dans la clé du module.
pub fn empreinte(wasm: &[u8]) -> String {
    hex(&sha384(wasm))
}

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Base64 standard (avec `=`), comme la page.
pub fn b64_encode(data: &[u8]) -> String {
    let mut s = String::with_capacity(data.len().div_ceil(3) * 4);
    for c in data.chunks(3) {
        let n = (u32::from(c[0]) << 16) | (u32::from(*c.get(1).unwrap_or(&0)) << 8) | u32::from(*c.get(2).unwrap_or(&0));
        for (k, shift) in [18, 12, 6, 0].into_iter().enumerate() {
            s.push(if k <= c.len() { B64[(n >> shift) as usize & 63] as char } else { '=' });
        }
    }
    s
}

/// Base64 standard ; `None` si le texte n'en est pas.
pub fn b64_decode(s: &str) -> Option<Vec<u8>> {
    let s = s.as_bytes();
    if !s.len().is_multiple_of(4) {
        return None;
    }
    let val = |c: u8| B64.iter().position(|&b| b == c).map(|p| p as u32);
    let mut out = Vec::with_capacity(s.len() / 4 * 3);
    for (i, q) in s.chunks(4).enumerate() {
        let fin = i + 1 == s.len() / 4;
        let pad = q.iter().rev().take_while(|&&c| c == b'=').count();
        if pad > 2 || (pad > 0 && !fin) {
            return None;
        }
        let mut n = 0u32;
        for &c in &q[..4 - pad] {
            n = (n << 6) | val(c)?;
        }
        n <<= 6 * pad as u32;
        out.extend_from_slice(&n.to_be_bytes()[1..4 - pad]);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::noyau::{echantillon, EMA_JSON, EMA_WASM};

    #[test]
    fn base64_aller_retour() {
        for (d, e) in [("", ""), ("f", "Zg=="), ("fo", "Zm8="), ("foo", "Zm9v"), ("foobar", "Zm9vYmFy")] {
            assert_eq!(b64_encode(d.as_bytes()), e);
            assert_eq!(b64_decode(e).as_deref(), Some(d.as_bytes()));
        }
        let tout: Vec<u8> = (0..=255).collect();
        assert_eq!(b64_decode(&b64_encode(&tout)), Some(tout));
        for mauvais in ["Zg=", "Z===", "Zg==Zg==", "Zm9!"] {
            assert_eq!(b64_decode(mauvais), None, "{mauvais}");
        }
    }

    fn entree() -> String {
        b64_encode(&argos_module_host::fenetre("BTC", 3600, &echantillon(), vec![20, 0]).input.encode())
    }

    #[test]
    fn module_lance_une_fois_puis_depuis_la_memoire() {
        let mods = Modules::default();
        let cle = format!("trading/ema@{}", empreinte(EMA_WASM));
        // sans fichier ni mémoire : la page doit le renvoyer
        assert!(mods.lancer(&cle, "", EMA_JSON, &entree()).unwrap_err().contains("recharger"));
        let a = mods.lancer(&cle, &b64_encode(EMA_WASM), EMA_JSON, &entree()).expect("module vérifié et lancé");
        let b = mods.lancer(&cle, "", EMA_JSON, &entree()).expect("depuis la mémoire");
        assert_eq!(a, b);
        let out = argos_module_host::Output::decode(&b64_decode(&a).unwrap(), 200).expect("sortie du kit");
        assert_eq!(argos_module_host::lines_of(&out)[0].0, "EMA 20");
    }

    #[test]
    fn module_refuse_si_fichier_manifeste_ou_droits_changent() {
        let mods = Modules::default();
        let cle = format!("trading/ema@{}", empreinte(EMA_WASM));
        let mut abime = EMA_WASM.to_vec();
        let n = abime.len();
        abime[n - 1] ^= 1;
        assert!(mods.lancer(&cle, &b64_encode(&abime), EMA_JSON, &entree()).unwrap_err().contains("changé"));
        let autre = format!("trading/autre@{}", empreinte(EMA_WASM));
        assert!(mods
            .lancer(&autre, &b64_encode(EMA_WASM), EMA_JSON, &entree())
            .unwrap_err()
            .contains("autre module"));
        let reseau = EMA_JSON.replace("\"capabilities\": []", "\"capabilities\": [\"Network\"], \"domains\": [\"x.com\"]");
        assert!(mods.lancer(&cle, &b64_encode(EMA_WASM), &reseau, &entree()).unwrap_err().contains("réseau"));
        assert!(mods.lancer("sans-arobase", "", EMA_JSON, &entree()).is_err());
    }

    #[test]
    fn reseau_hors_liste_blanche_refuse() {
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        for url in ["https://exemple.invalid/x", "http://api.binance.com/api/v3/time", "file:///etc/passwd"] {
            assert!(rt.block_on(reseau(url, Some("{}"))).is_err(), "{url}");
            assert!(rt.block_on(reseau(url, None)).is_err(), "{url}");
        }
    }
}
