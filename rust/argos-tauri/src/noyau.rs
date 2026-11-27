//! 🧩 Le cœur de la coque, sans Tauri (testable seul) : le module EMA vérifié puis lancé dans la bulle du kit.
//!
//! Mêmes vérifications que l'app native :
//! - manifeste du kit lu et vérifié (`trading/…`, interface v2, champ inconnu = refus, réseau et IA refusés) ;
//! - `.wasm` vérifié sans le lancer (imports `argos::*` seulement, exports v2) ;
//! - empreinte SHA-384 = celle **acceptée** (`approbations.json`, l'accord de Jee) : un autre fichier est refusé ;
//! - droits : aucun accordé (refus par défaut) ; carburant, mémoire et temps limités par la bulle.
//!
//! Les bougies viennent de la **collecte** d'Argos (étape 8 du plan : `argos_data::collecte`, le même code que l'app
//! de bureau) : le cache du dossier propre à Argos HSS Trading, puis seulement ce qui manque (premier écran, haute
//! priorité), puis en arrière-plan le passé jusqu'à 5000 bougies et les trous réparés (basse priorité). Sans réseau :
//! le cache (« hors ligne ») ; sans cache non plus : un petit échantillon fixe (marqué comme tel). Le module reçoit la
//! fenêtre de l'étape 7 (50 000 bougies et 4 Mio au plus). Tout se passe ici, dans le cœur Rust : jamais dans la page.

#![forbid(unsafe_code)]

use argos_module_host::{engine::Engine, fenetre, lines_of, scan, sha384, Manifest, RawBar};
use serde::Serialize;

pub const EMA_WASM: &[u8] = include_bytes!("../../../app/modules/ema.wasm");
pub const EMA_JSON: &str = include_str!("../../../app/modules/ema.json");
/// L'accord : l'empreinte SHA-384 (hexadécimal) acceptée pour chaque module.
pub const APPROBATIONS: &str = include_str!("../approbations.json");

/// Ce que la page affiche.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Vue {
    pub titre: String,
    /// Clôtures et heures des bougies.
    pub temps: Vec<i64>,
    pub cloture: Vec<f64>,
    /// La courbe du module (`null` = vide au début).
    pub ligne: String,
    pub valeurs: Vec<Option<f64>>,
    /// Empreinte vérifiée (début) et source des bougies.
    pub empreinte: String,
    pub source: String,
}

fn hex48(s: &str) -> Option<[u8; 48]> {
    let s = s.trim();
    if s.len() != 96 {
        return None;
    }
    let mut out = [0u8; 48];
    for (i, b) in out.iter_mut().enumerate() {
        *b = u8::from_str_radix(s.get(2 * i..2 * i + 2)?, 16).ok()?;
    }
    Some(out)
}

/// L'empreinte acceptée pour ce chemin de module, ou `None` (pas d'accord = refus).
pub fn approuve(path: &str) -> Option<[u8; 48]> {
    let v: serde_json::Value = serde_json::from_str(APPROBATIONS).ok()?;
    hex48(v.get(path)?.as_str()?)
}

/// Vérifie puis lance le module (`wasm`, `manifeste`) sur ces bougies, dans la bulle du kit, sans aucun droit.
pub fn lancer(wasm: &[u8], manifeste: &str, symbole: &str, tf_s: i64, bars: &[RawBar], longueur: i64) -> Result<(String, Vec<Option<f64>>), String> {
    let m = Manifest::parse(manifeste)?;
    m.check()?;
    scan(wasm)?;
    let accord = approuve(&m.path).ok_or_else(|| format!("{} : aucun accord (approbations.json)", m.path))?;
    let longueur = longueur.clamp(1, 500);
    // 🪟 fenêtre de l'étape 7 : les plus récentes qui tiennent dans les limites du kit Argos
    let input = fenetre(symbole, tf_s, bars, vec![longueur, 0]).input;
    let dossier = std::env::temp_dir().join("argos-tauri-modules");
    let out = Engine::new()?.run(&m, wasm, &accord, &[], &dossier, &[], &input)?;
    let (nom, v) = lines_of(&out).into_iter().next().ok_or("le module n'a rendu aucune courbe")?;
    Ok((nom, v.into_iter().map(|x| x.is_finite().then_some(x)).collect()))
}

/// Échantillon hors ligne : 200 bougies d'une heure, une vague régulière (pas un vrai marché).
pub fn echantillon() -> Vec<RawBar> {
    (0..200)
        .map(|i| {
            let x = i as f64;
            let c = 100.0 + 8.0 * (x / 17.0).sin() + 3.0 * (x / 5.0).cos() + x * 0.05;
            let o = c - 0.6 * (x / 3.0).sin();
            (1_793_000_000 + i * 3600, o, c.max(o) + 0.8, c.min(o) - 0.8, c, 10.0)
        })
        .collect()
}

/// Les bougies BTC 1 h par la collecte d'Argos (cache, puis ce qui manque) ; le passé et les trous se complètent
/// ensuite en arrière-plan (un fil à part, basse priorité) et vont au cache : le calcul suivant les a. `None` s'il
/// n'y a ni réseau ni cache.
pub fn bougies_en_ligne() -> Option<(Vec<RawBar>, String)> {
    use argos_core::Timeframe;
    use argos_data::{collecte, Exchange};
    let (ex, base, tf) = (Exchange::Hyperliquid, "BTC", Timeframe::H1);
    let dossier = argos_data::cache::dir();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64);
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().ok()?;
    let c = rt.block_on(collecte::premier_ecran(
        argos_data::rt::Client::default(),
        ex,
        base,
        tf,
        dossier.as_deref(),
        now,
    ));
    if c.bars.is_empty() {
        return None;
    }
    if !c.hors_ligne {
        let (bars, a_reparer) = (c.bars.clone(), c.a_reparer.clone());
        std::thread::spawn(move || {
            if let Ok(rt) = tokio::runtime::Builder::new_current_thread().enable_all().build() {
                rt.block_on(collecte::completer(
                    argos_data::rt::Client::default(),
                    ex,
                    base,
                    tf,
                    dossier.as_deref(),
                    bars,
                    &a_reparer,
                    now,
                ));
            }
        });
    }
    let raw: Vec<RawBar> = c.bars.iter().map(|b| (b.time, b.open, b.high, b.low, b.close, b.volume)).collect();
    let src = if c.hors_ligne {
        format!("BTC 1 h · 📴 hors ligne · {} bougies du cache", raw.len())
    } else {
        format!("BTC 1 h · {} · {} bougies ({} du cache)", c.source, raw.len(), c.depuis_cache)
    };
    Some((raw, src))
}

/// La vue complète : bougies (en ligne, sinon l'échantillon), puis le module EMA vérifié.
pub fn vue_ema(longueur: i64, en_ligne: bool) -> Result<Vue, String> {
    let (bars, source) = en_ligne
        .then(bougies_en_ligne)
        .flatten()
        .unwrap_or_else(|| (echantillon(), "hors ligne · échantillon fixe (pas un vrai marché)".into()));
    let (ligne, valeurs) = lancer(EMA_WASM, EMA_JSON, "BTC", 3600, &bars, longueur)?;
    let hex: String = sha384(EMA_WASM).iter().take(8).map(|b| format!("{b:02x}")).collect();
    Ok(Vue {
        titre: "Module EMA (kit Argos) dans la coque Tauri".into(),
        temps: bars.iter().map(|b| b.0).collect(),
        cloture: bars.iter().map(|b| b.4).collect(),
        ligne,
        valeurs,
        empreinte: format!("SHA-384 {hex}… acceptée"),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accord_egal_au_fichier_du_module() {
        assert_eq!(
            approuve("trading/ema"),
            Some(sha384(EMA_WASM)),
            "approbations.json = empreinte de app/modules/ema.wasm"
        );
        assert_eq!(approuve("trading/autre"), None);
    }

    #[test]
    fn ema_hors_ligne_egale_au_calcul_attendu() {
        let v = vue_ema(20, false).expect("module vérifié et lancé");
        assert_eq!(v.ligne, "EMA 20");
        assert_eq!(v.valeurs.len(), 200);
        assert!(v.valeurs[..19].iter().all(Option::is_none), "vide avant 20 bougies");
        // même EMA que l'intégrée (moyenne simple au départ, puis 2 / (n + 1)), à l'arrondi de la sortie
        let c = &v.cloture;
        let mut e = c[..20].iter().sum::<f64>() / 20.0;
        for (i, (x, got)) in c.iter().zip(&v.valeurs).enumerate().skip(20) {
            e += (x - e) * 2.0 / 21.0;
            assert!((got.unwrap() - e).abs() < 1e-3, "bougie {i}");
        }
    }

    #[test]
    fn fichier_change_ou_droits_refuses() {
        let bars = echantillon();
        let mut abime = EMA_WASM.to_vec();
        let n = abime.len();
        abime[n - 1] ^= 1;
        assert!(
            lancer(&abime, EMA_JSON, "BTC", 3600, &bars, 20).is_err(),
            "autre fichier que celui accepté : refusé"
        );
        let reseau = EMA_JSON.replace("\"capabilities\": []", "\"capabilities\": [\"Network\"], \"domains\": [\"x.com\"]");
        assert!(lancer(EMA_WASM, &reseau, "BTC", 3600, &bars, 20).unwrap_err().contains("réseau"));
        let autre = EMA_JSON.replace("trading/ema", "trading/inconnu");
        assert!(lancer(EMA_WASM, &autre, "BTC", 3600, &bars, 20).unwrap_err().contains("accord"));
    }
}
