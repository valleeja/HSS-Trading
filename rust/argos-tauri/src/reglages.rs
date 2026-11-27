//! 🔄 Session partagée dans la coque (étape 4 de synchro-direct) : les mêmes fichiers de réglages que l'app de bureau
//! (`~/.config/argos/charts`), lus et écrits par le cœur Rust ; mêmes règles que le serveur local
//! (`argos-outils serveur`, `/api/reglages`) :
//! - seulement les réglages de session (`nom.json` simple) ; jamais compte, passkey, clés, portefeuille ;
//! - 1 Mio au plus, JSON valide (ou vide), fichier temporaire puis renommage, 0600, dossier 0700 ;
//! - un numéro de version par réglage ; `changements` dit ce qui a changé depuis (attend jusqu'à 25 s), y compris
//!   les fichiers écrits par l'app de bureau ou le serveur local.

#![forbid(unsafe_code)]

use serde::Serialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime};

pub const MAX_REGLAGE: u64 = 1 << 20;
const INTERDITS: [&str; 7] = [
    "account.json",
    "passkey.json",
    "claude-key.json",
    "account-key.json",
    "wallet.json",
    "cmc_credits.json",
    // 🔐 chiffré sur le PC : jamais réécrit en clair par la synchro
    "journal.json",
];

/// Le dossier des réglages de l'app de bureau, propre à Argos HSS Trading : `$XDG_CONFIG_HOME/argos-hss-trading/charts`,
/// sinon `~/.config/argos-hss-trading/charts` ; au premier appel, l'ancien `~/.config/argos/charts` y est copié (sans
/// l'ancien journal en clair, ancien dossier intact).
pub fn dossier() -> Option<PathBuf> {
    let d = argos_core::dossiers::reglages()?;
    if let Some(a) = argos_core::dossiers::reglages_ancien() {
        let _ = argos_core::dossiers::preparer(&d, &a);
    }
    Some(d)
}

/// Un réglage de session : « nom.json » simple, pas un fichier interdit, pas de sous-dossier.
pub fn nom_permis(nom: &str) -> bool {
    let Some(tige) = nom.strip_suffix(".json") else { return false };
    nom.len() <= 100
        && !INTERDITS.contains(&nom)
        && tige.chars().next().is_some_and(|c| c.is_ascii_alphanumeric())
        && tige.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Ce que la page reçoit (même forme que le serveur local).
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Reponse {
    pub e: String,
    pub v: u64,
    pub fichiers: BTreeMap<String, String>,
    pub retires: Vec<String>,
}

fn lire(d: &Path, nom: &str) -> Option<String> {
    let p = d.join(nom);
    if std::fs::metadata(&p).ok()?.len() > MAX_REGLAGE {
        return None;
    }
    std::fs::read_to_string(p).ok()
}

/// Écrit un réglage (JSON valide ou vide) : fichier temporaire puis renommage, 0600, dossier 0700.
pub fn ecrire(d: &Path, nom: &str, texte: &str) -> Result<(), String> {
    if !nom_permis(nom) {
        return Err("réglage refusé".into());
    }
    if texte.len() as u64 > MAX_REGLAGE {
        return Err("réglage trop gros".into());
    }
    if !texte.trim().is_empty() && serde_json::from_str::<serde_json::Value>(texte).is_err() {
        return Err("ce n'est pas un réglage JSON".into());
    }
    let e = |x: std::io::Error| x.to_string();
    std::fs::create_dir_all(d).map_err(e)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(d, std::fs::Permissions::from_mode(0o700));
    }
    let p = d.join(nom);
    let tmp = d.join(format!("{nom}.{}.coque.tmp", std::process::id()));
    let mut o = std::fs::OpenOptions::new();
    o.write(true).create(true).truncate(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut o, 0o600);
    let res = (|| {
        use std::io::Write;
        let mut f = o.open(&tmp)?;
        f.write_all(texte.as_bytes())?;
        f.sync_all()?;
        std::fs::rename(&tmp, &p)
    })();
    if res.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    res.map_err(e)
}

/// 🔢 Les numéros de version des réglages d'un dossier (relu à chaque question).
pub struct Journal {
    pub dossier: PathBuf,
    pub epoque: String,
    etat: Mutex<(u64, BTreeMap<String, (SystemTime, u64)>, BTreeMap<String, u64>)>,
}

impl Journal {
    pub fn new(dossier: PathBuf) -> Self {
        let epoque = format!(
            "coque-{:x}",
            SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).map_or(0, |d| d.as_nanos() as u64)
        );
        let j = Journal {
            dossier,
            epoque,
            etat: Mutex::new((0, BTreeMap::new(), BTreeMap::new())),
        };
        j.scanner(true);
        j
    }

    fn vus(&self) -> BTreeMap<String, (SystemTime, u64)> {
        std::fs::read_dir(&self.dossier)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|e| {
                let nom = e.file_name().into_string().ok()?;
                let m = e.metadata().ok()?;
                (nom_permis(&nom) && m.is_file() && m.len() <= MAX_REGLAGE).then_some((nom, (m.modified().ok()?, m.len())))
            })
            .collect()
    }

    /// Relit le dossier ; rend le numéro de version courant.
    pub fn scanner(&self, premier: bool) -> u64 {
        let vus = self.vus();
        let Ok(mut g) = self.etat.lock() else { return 0 };
        let (v, anciens, versions) = &mut *g;
        let noms: Vec<String> = vus.keys().chain(anciens.keys()).cloned().collect();
        for nom in noms {
            if vus.get(&nom) != anciens.get(&nom) {
                if !premier {
                    *v += 1;
                }
                versions.insert(nom, *v);
            }
        }
        *anciens = vus;
        *v
    }

    /// Tous les réglages de session.
    pub fn tout(&self) -> Reponse {
        let v = self.scanner(false);
        let fichiers = self.vus().keys().filter_map(|n| Some((n.clone(), lire(&self.dossier, n)?))).collect();
        Reponse {
            e: self.epoque.clone(),
            v,
            fichiers,
            retires: vec![],
        }
    }

    /// Ce qui a changé après `depuis` (attend au plus `attente`) ; une autre époque : tout.
    pub fn changements(&self, epoque: &str, depuis: u64, attente: Duration) -> Reponse {
        let fin = Instant::now() + attente.min(Duration::from_secs(25));
        loop {
            let v = self.scanner(false);
            let (noms, presents) = {
                let Ok(g) = self.etat.lock() else { return Reponse::default() };
                let tout = epoque != self.epoque;
                let noms: Vec<String> = g.2.iter().filter(|(_, k)| tout || **k > depuis).map(|(n, _)| n.clone()).collect();
                (noms, g.1.keys().cloned().collect::<Vec<_>>())
            };
            if !noms.is_empty() || Instant::now() >= fin {
                let mut r = Reponse {
                    e: self.epoque.clone(),
                    v,
                    ..Default::default()
                };
                for nom in noms {
                    match presents.contains(&nom).then(|| lire(&self.dossier, &nom)).flatten() {
                        Some(t) => {
                            r.fichiers.insert(nom, t);
                        }
                        None if !presents.contains(&nom) => r.retires.push(nom),
                        None => {}
                    }
                }
                return r;
            }
            std::thread::sleep(Duration::from_millis(500).min(fin.saturating_duration_since(Instant::now())));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(n: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("argos-coque-reglages-{n}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn noms_permis() {
        for ok in ["split.json", "dock.json", "hyperliquid-btc.json", "indicator_inputs.json"] {
            assert!(nom_permis(ok), "{ok}");
        }
        for non in [
            "account.json",
            "passkey.json",
            "claude-key.json",
            "account-key.json",
            "wallet.json",
            "journal.json",
            "../x.json",
            "a/b.json",
            "x.txt",
            ".json",
            "-x.json",
            "é.json",
        ] {
            assert!(!nom_permis(non), "{non}");
        }
    }

    #[test]
    fn ecrire_relire_et_refus() {
        let d = tmp("ecrire");
        ecrire(&d, "split.json", "{\"n\":2}").unwrap();
        assert_eq!(lire(&d, "split.json").as_deref(), Some("{\"n\":2}"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(std::fs::metadata(d.join("split.json")).unwrap().permissions().mode() & 0o777, 0o600);
            assert_eq!(std::fs::metadata(&d).unwrap().permissions().mode() & 0o777, 0o700);
        }
        for (nom, t) in [("account.json", "{}"), ("../x.json", "{}"), ("x.json", "pas json")] {
            assert!(ecrire(&d, nom, t).is_err(), "{nom}");
        }
        assert!(ecrire(&d, "x.json", &" ".repeat(MAX_REGLAGE as usize + 1)).is_err());
        let restes: Vec<_> = std::fs::read_dir(&d).unwrap().flatten().map(|e| e.file_name()).collect();
        assert_eq!(restes, ["split.json"], "aucun fichier temporaire ne reste");
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn versions_et_changements() {
        let d = tmp("journal");
        ecrire(&d, "dock.json", "{}").unwrap();
        std::fs::write(d.join("account.json"), "{\"secret\":1}").unwrap();
        let j = Journal::new(d.clone());
        let t = j.tout();
        assert_eq!(t.fichiers.keys().collect::<Vec<_>>(), ["dock.json"], "jamais le compte");
        assert!(j.changements(&j.epoque, t.v, Duration::ZERO).fichiers.is_empty());
        assert_eq!(j.changements("autre", t.v, Duration::ZERO).fichiers.len(), 1, "autre époque : tout");
        // écrit par l'app de bureau (directement) : vu ; retiré : annoncé
        std::fs::write(d.join("split.json"), "{\"n\":3}").unwrap();
        let c = j.changements(&j.epoque, t.v, Duration::ZERO);
        assert_eq!(c.fichiers.get("split.json").map(String::as_str), Some("{\"n\":3}"));
        std::fs::remove_file(d.join("dock.json")).unwrap();
        let c2 = j.changements(&j.epoque, c.v, Duration::ZERO);
        assert_eq!(c2.retires, ["dock.json"]);
        // attente : un changement pendant l'attente répond tout de suite
        let d2 = d.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(300));
            ecrire(&d2, "alerts.json", "[]").unwrap();
        });
        let t0 = Instant::now();
        let c3 = j.changements(&j.epoque, c2.v, Duration::from_secs(10));
        assert!(c3.fichiers.contains_key("alerts.json") && t0.elapsed() < Duration::from_secs(3));
        std::fs::remove_dir_all(&d).ok();
    }
}
