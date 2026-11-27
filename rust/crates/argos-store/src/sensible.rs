//! 🔐 La base **sensible** du desktop : `sensible.redb`, à côté d'`argos.redb`, où chaque valeur est **chiffrée**
//! (AES-256-GCM) avec la clé propre à l'appareil, donnée à l'ouverture (32 octets ; le desktop la garde dans le
//! trousseau du système). Pour le journal de trading et toute donnée personnelle ; jamais en clair sur le disque.
//!
//! - Une valeur = du JSON rangé sous (table, ligne). Chaque écriture tire un nonce neuf de 12 octets (`getrandom`).
//! - La valeur est **liée** à sa table et à sa ligne (données associées du chiffrement) : recopiée sous une autre
//!   ligne, elle ne s'ouvre pas. Les noms de table et de ligne, eux, restent lisibles.
//! - Mauvaise clé, valeur abîmée ou recopiée : une erreur claire, jamais une panique.
//!
//! Format d'une valeur : version (1 octet), nonce (12 octets), texte chiffré et son étiquette (16 octets).

use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Nonce};
#[cfg(test)]
use redb::backends::InMemoryBackend;
use redb::{Database, ReadableDatabase, TableDefinition};
use serde_json::Value;
use std::path::{Path, PathBuf};

/// Une clé de chiffrement : 32 octets.
pub type Cle = [u8; 32];

/// Version du format d'une valeur.
const VERSION: u8 = 1;
const NONCE: usize = 12;
/// Séparateur entre la table et la ligne dans la clé redb (interdit dans leurs noms).
const SEP: char = '\u{1f}';
const VALEURS: TableDefinition<&str, &[u8]> = TableDefinition::new("valeurs");

/// Emplacement de la base sensible : à côté d'`argos.redb` (`…/argos-hss-trading/sensible.redb`).
pub fn chemin() -> Option<PathBuf> {
    Some(crate::default_path()?.with_file_name("sensible.redb"))
}

/// Ce qui peut mal tourner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Erreur {
    /// Le fichier de la base (ouverture, lecture, écriture).
    Base(String),
    /// Mauvaise clé, ou valeur abîmée ou recopiée sous une autre ligne : elle ne s'ouvre pas.
    Dechiffrement,
    /// Nom de table ou de ligne refusé, ou JSON illisible.
    Format(String),
}

impl std::fmt::Display for Erreur {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Erreur::Base(e) => write!(f, "base sensible : {e}"),
            Erreur::Dechiffrement => write!(f, "base sensible : valeur illisible (mauvaise clé, ou valeur abîmée)"),
            Erreur::Format(e) => write!(f, "base sensible : {e}"),
        }
    }
}

fn base(e: impl Into<redb::Error>) -> Erreur {
    Erreur::Base(e.into().to_string())
}

/// La base sensible ouverte avec sa clé.
pub struct Sensible {
    db: Database,
    chiffre: Aes256Gcm,
}

impl Sensible {
    /// Ouvre (ou crée) la base à cet endroit, lisible seulement par toi (dossier 0700, fichier 0600).
    pub fn ouvrir(path: &Path, cle: &Cle) -> Result<Self, Erreur> {
        if let Some(d) = path.parent() {
            std::fs::create_dir_all(d).map_err(|e| Erreur::Base(format!("dossier : {e}")))?;
            #[cfg(unix)]
            std::fs::set_permissions(d, std::os::unix::fs::PermissionsExt::from_mode(0o700)).map_err(|e| Erreur::Base(format!("dossier : {e}")))?;
        }
        let db = Database::create(path).map_err(base)?;
        #[cfg(unix)]
        std::fs::set_permissions(path, std::os::unix::fs::PermissionsExt::from_mode(0o600)).map_err(|e| Erreur::Base(format!("fichier : {e}")))?;
        Self::avec(db, cle)
    }

    /// Base en mémoire (tests).
    #[cfg(test)]
    pub fn en_memoire(cle: &Cle) -> Result<Self, Erreur> {
        Self::avec(Database::builder().create_with_backend(InMemoryBackend::new()).map_err(base)?, cle)
    }

    fn avec(db: Database, cle: &Cle) -> Result<Self, Erreur> {
        // the table is created once: a write (and its disk sync) at every opening, even for a read, made each short
        // opening of the journal (`journal::un_moment`) much longer under load, so the other program waited longer
        let existe = db.begin_read().map_err(base)?.open_table(VALEURS).is_ok();
        if !existe {
            let tx = db.begin_write().map_err(base)?;
            tx.open_table(VALEURS).map_err(base)?;
            tx.commit().map_err(base)?;
        }
        let chiffre = Aes256Gcm::new_from_slice(cle).map_err(|_| Erreur::Dechiffrement)?;
        Ok(Self { db, chiffre })
    }

    /// La clé redb et les données associées d'une valeur (table + ligne).
    fn cle_de(table: &str, ligne: &str) -> Result<(String, String), Erreur> {
        let ok = |s: &str| !s.is_empty() && s.len() <= 200 && !s.contains(SEP);
        if !ok(table) || !ok(ligne) {
            return Err(Erreur::Format(format!("nom refusé : « {table} » / « {ligne} »")));
        }
        Ok((
            format!("{table}{SEP}{ligne}"),
            format!("argos-hss-trading/sensible/{VERSION}{SEP}{table}{SEP}{ligne}"),
        ))
    }

    /// Écrit (ou remplace) la valeur JSON de (table, ligne), chiffrée avec un nonce neuf.
    #[cfg(test)]
    pub fn ecrire_json(&self, table: &str, ligne: &str, valeur: &Value) -> Result<(), Erreur> {
        self.ecrire_lot(&[(table, ligne, Some(valeur))])
    }

    /// Plusieurs écritures (`Some`) et retraits (`None`) en **une** transaction : tout ou rien.
    pub fn ecrire_lot(&self, lot: &[(&str, &str, Option<&Value>)]) -> Result<(), Erreur> {
        let mut prets: Vec<(String, Option<Vec<u8>>)> = Vec::with_capacity(lot.len());
        for &(table, ligne, valeur) in lot {
            let (cle, aad) = Self::cle_de(table, ligne)?;
            prets.push((cle, valeur.map(|v| self.chiffrer(&aad, v)).transpose()?));
        }
        let tx = self.db.begin_write().map_err(base)?;
        {
            let mut t = tx.open_table(VALEURS).map_err(base)?;
            for (cle, v) in &prets {
                match v {
                    Some(v) => t.insert(cle.as_str(), v.as_slice()).map(|_| ()).map_err(base)?,
                    None => t.remove(cle.as_str()).map(|_| ()).map_err(base)?,
                }
            }
        }
        tx.commit().map_err(base)
    }

    /// Une valeur chiffrée : version, nonce neuf, texte chiffré lié à `aad`.
    fn chiffrer(&self, aad: &str, valeur: &Value) -> Result<Vec<u8>, Erreur> {
        let texte = serde_json::to_vec(valeur).map_err(|e| Erreur::Format(e.to_string()))?;
        let mut nonce = [0u8; NONCE];
        getrandom::fill(&mut nonce).map_err(|e| Erreur::Base(format!("hasard : {e}")))?;
        let chiffre = self
            .chiffre
            .encrypt(
                Nonce::from_slice(&nonce),
                Payload {
                    msg: &texte,
                    aad: aad.as_bytes(),
                },
            )
            .map_err(|_| Erreur::Dechiffrement)?;
        let mut v = Vec::with_capacity(1 + NONCE + chiffre.len());
        v.push(VERSION);
        v.extend_from_slice(&nonce);
        v.extend_from_slice(&chiffre);
        Ok(v)
    }

    /// La valeur JSON de (table, ligne), déchiffrée ; `None` si elle n'existe pas.
    pub fn lire_json(&self, table: &str, ligne: &str) -> Result<Option<Value>, Erreur> {
        let (cle, aad) = Self::cle_de(table, ligne)?;
        let tx = self.db.begin_read().map_err(base)?;
        let t = tx.open_table(VALEURS).map_err(base)?;
        let Some(brut) = t.get(cle.as_str()).map_err(base)? else { return Ok(None) };
        let v = brut.value();
        if v.len() < 1 + NONCE || v[0] != VERSION {
            return Err(Erreur::Dechiffrement);
        }
        let texte = self
            .chiffre
            .decrypt(
                Nonce::from_slice(&v[1..1 + NONCE]),
                Payload {
                    msg: &v[1 + NONCE..],
                    aad: aad.as_bytes(),
                },
            )
            .map_err(|_| Erreur::Dechiffrement)?;
        serde_json::from_slice(&texte).map(Some).map_err(|e| Erreur::Format(e.to_string()))
    }

    /// Les lignes d'une table, dans l'ordre.
    pub fn lister(&self, table: &str) -> Result<Vec<String>, Erreur> {
        let (debut, _) = Self::cle_de(table, "x")?;
        let prefixe = &debut[..debut.len() - 1];
        let tx = self.db.begin_read().map_err(base)?;
        let t = tx.open_table(VALEURS).map_err(base)?;
        let mut out = vec![];
        for e in t.range(prefixe..).map_err(base)? {
            let (k, _) = e.map_err(base)?;
            let Some(ligne) = k.value().strip_prefix(prefixe).map(str::to_string) else {
                break;
            };
            out.push(ligne);
        }
        Ok(out)
    }

    /// Retire (table, ligne) ; `false` si elle n'existait pas.
    pub fn retirer(&self, table: &str, ligne: &str) -> Result<bool, Erreur> {
        let (cle, _) = Self::cle_de(table, ligne)?;
        let tx = self.db.begin_write().map_err(base)?;
        let la = tx.open_table(VALEURS).map_err(base)?.remove(cle.as_str()).map_err(base)?.is_some();
        tx.commit().map_err(base)?;
        Ok(la)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const CLE: Cle = [7; 32];

    /// Un dossier de test dans le dossier de compilation du projet (jamais /tmp), effacé à la fin du test.
    fn dossier(nom: &str) -> PathBuf {
        let d = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/tmp")
            .join(format!("sensible-{nom}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn chiffrer_puis_dechiffrer() {
        let s = Sensible::en_memoire(&CLE).unwrap();
        let v = json!({"trades": [{"sym": "BTC", "pnl": 120.5}], "cfg": {"tilt_n": 3}});
        s.ecrire_json("journal", "principal", &v).unwrap();
        assert_eq!(s.lire_json("journal", "principal").unwrap(), Some(v));
        assert_eq!(s.lire_json("journal", "absent").unwrap(), None);
        s.ecrire_json("journal", "autre", &json!(1)).unwrap();
        s.ecrire_json("reglages", "a", &json!(2)).unwrap();
        assert_eq!(s.lister("journal").unwrap(), vec!["autre", "principal"], "seulement cette table");
        assert!(s.retirer("journal", "autre").unwrap());
        assert!(!s.retirer("journal", "autre").unwrap());
        assert!(matches!(s.ecrire_json("jour\u{1f}nal", "x", &json!(1)), Err(Erreur::Format(_))));
    }

    /// Sur le disque : rien du journal en clair, fichier 0600, une mauvaise clé ne lit rien, sans panique.
    #[test]
    fn sur_disque_jamais_en_clair_et_mauvaise_cle() {
        let d = dossier("disque");
        let p = d.join("sensible.redb");
        {
            let s = Sensible::ouvrir(&p, &CLE).unwrap();
            s.ecrire_json("journal", "principal", &json!({"notes": "SECRET-TRADE-XAUUSD"})).unwrap();
        }
        let brut = std::fs::read(&p).unwrap();
        assert!(
            !brut.windows(19).any(|w| w == b"SECRET-TRADE-XAUUSD"),
            "le texte n'est jamais en clair sur le disque"
        );
        #[cfg(unix)]
        assert_eq!(
            std::os::unix::fs::PermissionsExt::mode(&std::fs::metadata(&p).unwrap().permissions()) & 0o777,
            0o600
        );
        let autre = Sensible::ouvrir(&p, &[8; 32]).unwrap();
        assert_eq!(
            autre.lire_json("journal", "principal"),
            Err(Erreur::Dechiffrement),
            "mauvaise clé : erreur claire"
        );
        drop(autre);
        let bonne = Sensible::ouvrir(&p, &CLE).unwrap();
        assert_eq!(bonne.lire_json("journal", "principal").unwrap(), Some(json!({"notes": "SECRET-TRADE-XAUUSD"})));
        drop(bonne);
        std::fs::remove_dir_all(&d).unwrap();
    }

    /// Deux écritures identiques : deux nonces différents ; une valeur abîmée ou recopiée sur une autre ligne ne
    /// s'ouvre pas.
    #[test]
    fn nonces_neufs_valeur_abimee_ou_recopiee() {
        let s = Sensible::en_memoire(&CLE).unwrap();
        let brut = |s: &Sensible, ligne: &str| -> Vec<u8> {
            let tx = s.db.begin_read().unwrap();
            let t = tx.open_table(VALEURS).unwrap();
            t.get(format!("journal{SEP}{ligne}").as_str()).unwrap().unwrap().value().to_vec()
        };
        let ecrire_brut = |s: &Sensible, ligne: &str, v: &[u8]| {
            let tx = s.db.begin_write().unwrap();
            tx.open_table(VALEURS).unwrap().insert(format!("journal{SEP}{ligne}").as_str(), v).unwrap();
            tx.commit().unwrap();
        };
        s.ecrire_json("journal", "a", &json!("même valeur")).unwrap();
        let un = brut(&s, "a");
        s.ecrire_json("journal", "a", &json!("même valeur")).unwrap();
        let deux = brut(&s, "a");
        assert_ne!(un[1..1 + NONCE], deux[1..1 + NONCE], "un nonce neuf à chaque écriture");
        assert_ne!(un, deux);
        // recopiée telle quelle sous une autre ligne : refusée
        ecrire_brut(&s, "b", &deux);
        assert_eq!(s.lire_json("journal", "b"), Err(Erreur::Dechiffrement));
        // un octet changé : refusée ; trop courte : refusée
        let mut abimee = deux.clone();
        let fin = abimee.len() - 1;
        abimee[fin] ^= 1;
        ecrire_brut(&s, "a", &abimee);
        assert_eq!(s.lire_json("journal", "a"), Err(Erreur::Dechiffrement));
        ecrire_brut(&s, "a", &[VERSION, 1, 2]);
        assert_eq!(s.lire_json("journal", "a"), Err(Erreur::Dechiffrement));
    }
}
