//! 🔐 Le journal de trading dans la base sensible, **partagé** par l'app de bureau et le serveur local
//! (`argos-serveur`, qui le sert au cell) : la même logique des deux côtés, jamais copiée.
//! - Rangement : les réglages (table `journal`, ligne `cfg`), un trade par ligne (table `journal-trades`, ligne = son
//!   identifiant), et un numéro de version (table `journal`, ligne `version`) qui monte à chaque écriture.
//! - Écritures **entrée par entrée** (`argos_core::journal::Changements`), en une transaction : la dernière écriture
//!   gagne pour chaque entrée, et une entrée qu'un appareil n'a pas touchée n'est jamais écrasée.
//! - **Un seul programme à la fois** sur le fichier (redb) : chacun l'ouvre seulement le temps d'une lecture ou d'une
//!   écriture (`un_moment`), avec quelques essais courts si l'autre la tient, puis une erreur claire.
//! - **Clé de l'appareil** : 32 octets aléatoires créés au premier usage, dans le trousseau du système (service
//!   « argos-hss-trading », nom « cle-appareil ») ; jamais dans un fichier, un log ni le dépôt.
//! - **Migration** : l'ancien `journal.json` en clair est relu, écrit chiffré, relu et comparé, puis effacé seulement
//!   si tout est identique (sinon il reste, rien n'est perdu).

use crate::sensible::{Cle, Sensible};
pub use argos_core::journal::{Changements, Donnees};
use argos_core::journal::{JCfg, JTrade};
use serde_json::Value;
use std::path::Path;

/// L'ancien fichier en clair (dossier des réglages), migré une fois puis effacé.
pub const ANCIEN: &str = "journal.json";
const T_JOURNAL: &str = "journal";
const T_TRADES: &str = "journal-trades";
/// Longest wait while the other program holds the database (a duration, not a number of tries: under load each try
/// takes longer), then a clear error; never an endless wait.
pub const ATTENTE: std::time::Duration = std::time::Duration::from_secs(3);
/// Pause between two opening tries.
const PAUSE: std::time::Duration = std::time::Duration::from_millis(10);
/// ⚖️ A request not refreshed for this long is forgotten. A waiting program refreshes it at every try (every `PAUSE`,
/// 10 ms), so this only happens to a program stopped or frozen while waiting. Short on purpose: the app opens the
/// journal on its screen thread, so a request left behind must never freeze the screen for long (and outside Linux,
/// where a stopped program cannot be recognised by its process number, it is the only way to forget its request).
const OUBLI: std::time::Duration = std::time::Duration::from_secs(2);

/// ⚖️ The request file of a waiting program (`sensible.redb.attente`, holding its process number). Without it, a
/// program chaining operations took the database back as soon as it released it, and the other one, which only tries
/// now and then, could miss every short free moment and give up after `ATTENTE` (seen under load). With it, the turn
/// is handed over, whatever the timing: the program that finds the database held writes the request (one at a time),
/// nobody else takes the database while the request is alive, the waiting one removes it once it has the database
/// (or when it gives up).
fn demande(chemin: &Path) -> std::path::PathBuf {
    let mut nom = chemin.as_os_str().to_owned();
    nom.push(".attente");
    nom.into()
}

/// Is a request waiting for its turn? Yes while its program runs and refreshes it (`OUBLI`); a request left behind by a
/// program that stopped is ignored at once.
fn demande_active(d: &Path) -> bool {
    let fraiche = std::fs::metadata(d)
        .and_then(|m| m.modified())
        .is_ok_and(|t| t.elapsed().map_or(true, |age| age < OUBLI));
    fraiche && proprietaire_vivant(d)
}

/// The program that wrote the request still runs (Linux: `/proc/<n°>`; elsewhere, or number not written yet: yes).
fn proprietaire_vivant(d: &Path) -> bool {
    let numero = std::fs::read_to_string(d).ok().and_then(|s| s.trim().parse::<u32>().ok());
    match numero {
        Some(n) if cfg!(target_os = "linux") => Path::new("/proc").join(n.to_string()).exists(),
        _ => true,
    }
}

/// Writes this program's request (`false`: another live one is there). A request left behind is replaced.
fn demander(d: &Path) -> bool {
    if !demande_active(d) {
        let _ = std::fs::remove_file(d);
    }
    let Ok(mut f) = std::fs::OpenOptions::new().write(true).create_new(true).open(d) else {
        return false;
    };
    std::io::Write::write_all(&mut f, std::process::id().to_string().as_bytes()).is_ok()
}

/// The waiting program shows it is still there (its request is never forgotten while it tries); `false`: the request
/// is gone (forgotten and replaced while this program was frozen).
fn rafraichir(d: &Path) -> bool {
    std::fs::OpenOptions::new()
        .write(true)
        .open(d)
        .is_ok_and(|f| f.set_modified(std::time::SystemTime::now()).is_ok())
}

/// La clé de l'appareil, dans le trousseau du système (créée au premier usage).
pub fn cle_du_trousseau() -> Result<Cle, String> {
    if let Some(cle) = cle_gardee()? {
        return Ok(cle);
    }
    let mut cle = [0u8; 32];
    getrandom::fill(&mut cle).map_err(|e| format!("hasard : {e}"))?;
    garder_cle(&cle)?;
    Ok(cle)
}

/// The device key entry of the system keyring.
fn entree_trousseau() -> Result<keyring::Entry, String> {
    keyring::Entry::new("argos-hss-trading", "cle-appareil").map_err(|e| format!("trousseau : {e}"))
}

/// The device key kept in the keyring, without creating one (`None`: no key on this device yet).
pub fn cle_gardee() -> Result<Option<Cle>, String> {
    match entree_trousseau()?.get_password() {
        Ok(hex) => cle_du_hex(&hex).map(Some),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(format!("trousseau : {e}")),
    }
}

/// Keeps `cle` in the keyring as the device key (replaces the one there).
pub fn garder_cle(cle: &Cle) -> Result<(), String> {
    let hex: String = cle.iter().map(|b| format!("{b:02x}")).collect();
    entree_trousseau()?.set_password(&hex).map_err(|e| format!("trousseau : {e}"))
}

/// The key written in hexadecimal in the keyring.
fn cle_du_hex(hex: &str) -> Result<Cle, String> {
    let octets: Vec<u8> = (0..hex.len())
        .step_by(2)
        .filter_map(|i| hex.get(i..i + 2).and_then(|x| u8::from_str_radix(x, 16).ok()))
        .collect();
    octets.try_into().map_err(|_| "trousseau : clé de l'appareil illisible".to_string())
}

/// Opens the database for the time of `f`, then closes it (the other program can take it). Another program is waiting
/// for its turn (`demande`): it goes first, and that wait is not counted. Held by the other one: short tries for at
/// most `ATTENTE` (counted from our own first try), with our request written, then a clear error.
pub fn un_moment<T>(chemin: &Path, cle: &Cle, f: impl FnOnce(&Sensible) -> Result<T, String>) -> Result<T, String> {
    let d = demande(chemin);
    let mut a_demande = false;
    let mut fin = None;
    loop {
        if !a_demande && demande_active(&d) {
            // someone else's turn: it ends when they have the database or give up; our own wait starts after it
            fin = None;
            std::thread::sleep(PAUSE);
            continue;
        }
        let fin = *fin.get_or_insert_with(|| std::time::Instant::now() + ATTENTE);
        let derniere = match Sensible::ouvrir(chemin, cle) {
            Ok(b) => {
                if a_demande {
                    let _ = std::fs::remove_file(&d);
                }
                return f(&b);
            }
            Err(e) => e,
        };
        // (our request was removed while we were frozen: we ask again)
        a_demande = (a_demande && rafraichir(&d)) || demander(&d);
        if std::time::Instant::now() >= fin {
            if a_demande {
                let _ = std::fs::remove_file(&d);
            }
            return Err(format!("journal occupé ou illisible ({derniere})"));
        }
        std::thread::sleep(PAUSE);
    }
}

/// Le numéro de version de la base (0 : journal jamais écrit).
pub fn version(b: &Sensible) -> Result<u64, String> {
    Ok(b.lire_json(T_JOURNAL, "version")
        .map_err(|e| e.to_string())?
        .and_then(|v| v.as_u64())
        .unwrap_or(0))
}

/// Le journal de la base.
pub fn lire(b: &Sensible) -> Result<Donnees, String> {
    let cfg: JCfg = match b.lire_json(T_JOURNAL, "cfg").map_err(|e| e.to_string())? {
        Some(v) => serde_json::from_value(v).map_err(|e| format!("journal illisible : {e}"))?,
        None => JCfg::default(),
    };
    let mut trades: Vec<JTrade> = vec![];
    for id in b.lister(T_TRADES).map_err(|e| e.to_string())? {
        if let Some(v) = b.lire_json(T_TRADES, &id).map_err(|e| e.to_string())? {
            trades.push(serde_json::from_value(v).map_err(|e| format!("journal illisible : {e}"))?);
        }
    }
    trades.sort_by_key(|t| t.entry_ts);
    Ok(Donnees {
        cfg,
        trades,
        version: version(b)?,
    })
}

/// Écrit des changements (entrée par entrée, une seule transaction, version + 1) et rend le journal à jour.
pub fn appliquer(b: &Sensible, ch: &Changements) -> Result<Donnees, String> {
    if ch.vide() {
        return lire(b);
    }
    let err = |e: serde_json::Error| e.to_string();
    let cfg = ch.cfg.as_ref().map(serde_json::to_value).transpose().map_err(err)?;
    let trades: Vec<(String, Value)> = ch
        .mettre
        .iter()
        .map(|t| Ok((t.id.clone(), serde_json::to_value(t).map_err(err)?)))
        .collect::<Result<_, String>>()?;
    let version = Value::from(version(b)? + 1);
    let mut lot: Vec<(&str, &str, Option<&Value>)> = vec![(T_JOURNAL, "version", Some(&version))];
    if let Some(c) = &cfg {
        lot.push((T_JOURNAL, "cfg", Some(c)));
    }
    lot.extend(trades.iter().map(|(id, v)| (T_TRADES, id.as_str(), Some(v))));
    lot.extend(ch.retirer.iter().map(|id| (T_TRADES, id.as_str(), None)));
    b.ecrire_lot(&lot).map_err(|e| e.to_string())?;
    lire(b)
}

/// Migre l'ancien journal en clair `ancien` (s'il existe et que la base n'a encore aucun journal) : écrit chiffré,
/// relu et comparé, puis effacé. Un ancien fichier illisible n'est ni migré ni effacé (erreur).
pub fn migrer(b: &Sensible, ancien: &Path) -> Result<(), String> {
    if version(b)? > 0 || !ancien.exists() {
        return Ok(());
    }
    let texte = std::fs::read_to_string(ancien).map_err(|e| format!("ancien journal : {e}"))?;
    let vieux: Donnees = if texte.trim().is_empty() {
        Donnees::default()
    } else {
        serde_json::from_str(&texte).map_err(|e| format!("ancien journal illisible, gardé tel quel : {e}"))?
    };
    let ch = Changements {
        cfg: Some(vieux.cfg.clone()),
        mettre: vieux.trades.clone(),
        retirer: vec![],
    };
    let mut attendu = vieux.trades.clone();
    attendu.sort_by_key(|t| t.entry_ts);
    let relu = appliquer(b, &ch)?;
    if relu.cfg != vieux.cfg || relu.trades != attendu {
        return Err("relecture du journal chiffré différente : l'ancien fichier est gardé".into());
    }
    std::fs::remove_file(ancien).map_err(|e| format!("ancien journal migré mais pas effacé : {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    const CLE: Cle = [42; 32];

    /// Un ancien `journal.json` réel (format de l'app avant le chiffrement, réglages du Gardien de l'étape 2).
    const ANCIEN_JOURNAL: &str = r#"{"tab":0,"period":3,"strat":null,"sym":null,"trades":[{"id":"demo-BTC-1793000000","sym":"BTC","side":"long","tf":"1h","strat":"Breakout","entry":95000.0,"sl":94000.0,"tp":97000.0,"exit":96500.0,"risk":500.0,"fees":null,"pnl":750.0,"rate":null,"entryTs":1793000000000,"exitTs":1793003600000,"conf":["RSI"],"emo":"revenge","notes":"NOTE-PRIVEE-XAU","shot":""},{"id":"a2","sym":"ETH","side":"short","entryTs":1792000000000,"pnl":-80.0}],"cfg":{"risk":500.0,"account":100000.0,"warn":80.0,"tilt_n":3,"tilt_min":45,"tilt_revenge":true,"pause_until":0}}"#;

    /// Un dossier de test dans le dossier de compilation du projet (jamais /tmp), effacé à la fin du test.
    fn dossier(nom: &str) -> PathBuf {
        let d = argos_core::dossiers::de_test(&format!("journal-{nom}"));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn trade(id: &str, pnl: f64) -> JTrade {
        JTrade {
            id: id.into(),
            pnl: Some(pnl),
            ..Default::default()
        }
    }

    /// Migration sans perte d'un vrai ancien fichier, effacé après relecture ; relance ; rien en clair sur le disque.
    #[test]
    fn migration_sans_perte_puis_relance() {
        let d = dossier("migration");
        let (ancien, p) = (d.join(ANCIEN), d.join("sensible.redb"));
        std::fs::write(&ancien, ANCIEN_JOURNAL).unwrap();
        let vieux: Donnees = serde_json::from_str(ANCIEN_JOURNAL).unwrap();
        let j = un_moment(&p, &CLE, |b| {
            migrer(b, &ancien)?;
            lire(b)
        })
        .unwrap();
        assert!(!ancien.exists(), "effacé après relecture identique");
        assert_eq!(j.cfg, vieux.cfg);
        assert_eq!(
            j.trades.iter().map(|t| t.id.as_str()).collect::<Vec<_>>(),
            ["a2", "demo-BTC-1793000000"],
            "tous là, dans l'ordre"
        );
        assert_eq!((j.cfg.tilt_n, j.cfg.tilt_min, j.cfg.tilt_revenge, j.version), (3, 45, true, 1));
        // relance : relu de la base (plus d'ancien fichier), rien de migré deux fois
        let j2 = un_moment(&p, &CLE, |b| {
            migrer(b, &ancien)?;
            lire(b)
        })
        .unwrap();
        assert_eq!(j2, j);
        let brut = std::fs::read(&p).unwrap();
        assert!(!brut.windows(15).any(|w| w == b"NOTE-PRIVEE-XAU"), "aucun texte du journal en clair");
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn ancien_illisible_garde() {
        let d = dossier("illisible");
        let ancien = d.join(ANCIEN);
        std::fs::write(&ancien, "{ pas du json").unwrap();
        let b = Sensible::en_memoire(&CLE).unwrap();
        assert!(migrer(&b, &ancien).is_err());
        assert!(ancien.exists(), "illisible : gardé tel quel");
        assert_eq!(lire(&b).unwrap(), Donnees::default());
        std::fs::remove_dir_all(&d).unwrap();
    }

    /// Entrée par entrée : un retrait, un ajout et les réglages en une écriture ; la version monte.
    #[test]
    fn appliquer_entree_par_entree() {
        let b = Sensible::en_memoire(&CLE).unwrap();
        let j = appliquer(
            &b,
            &Changements {
                mettre: vec![trade("a", 1.0), trade("b", 2.0)],
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!((j.trades.len(), j.version), (2, 1));
        let j = appliquer(
            &b,
            &Changements {
                cfg: Some(JCfg { tilt_n: 4, ..JCfg::default() }),
                mettre: vec![trade("c", 3.0)],
                retirer: vec!["a".into()],
            },
        )
        .unwrap();
        assert_eq!(j.trades.iter().map(|t| t.id.as_str()).collect::<Vec<_>>(), ["b", "c"]);
        assert_eq!((j.cfg.tilt_n, j.version), (4, 2));
        assert_eq!(appliquer(&b, &Changements::default()).unwrap().version, 2, "rien à écrire : version inchangée");
    }

    /// Deux programmes (deux poignées) qui alternent écritures et lectures sur le même fichier : chacun l'ouvre le
    /// temps d'une opération, rien n'est perdu, rien ne bloque (l'autre attend quelques essais).
    #[test]
    fn deux_programmes_qui_alternent() {
        let d = dossier("deux");
        let p = d.join("sensible.redb");
        let p2 = p.clone();
        let autre = std::thread::spawn(move || {
            for k in 0..15 {
                un_moment(&p2, &CLE, |b| {
                    appliquer(
                        b,
                        &Changements {
                            mettre: vec![trade(&format!("cell-{k}"), 1.0)],
                            ..Default::default()
                        },
                    )
                })
                .unwrap();
            }
        });
        for k in 0..15 {
            un_moment(&p, &CLE, |b| {
                appliquer(
                    b,
                    &Changements {
                        mettre: vec![trade(&format!("pc-{k}"), 2.0)],
                        ..Default::default()
                    },
                )
            })
            .unwrap();
            un_moment(&p, &CLE, lire).unwrap();
        }
        autre.join().unwrap();
        let j = un_moment(&p, &CLE, lire).unwrap();
        assert_eq!((j.trades.len(), j.version), (30, 30), "toutes les écritures des deux côtés");
        // tenue par l'un : l'autre attend un peu puis rend une erreur claire, sans bloquer
        let tenue = Sensible::ouvrir(&p, &CLE).unwrap();
        let t0 = std::time::Instant::now();
        let e = un_moment(&p, &CLE, lire).unwrap_err();
        assert!(e.starts_with("journal occupé"), "{e}");
        assert!(t0.elapsed() < std::time::Duration::from_secs(10), "attente bornée");
        drop(tenue);
        std::fs::remove_dir_all(&d).unwrap();
    }

    /// ⚖️ A program that chains slow operations (30 ms each, like a disk write under load) for longer than `ATTENTE`
    /// never locks the other one out: thanks to the request file (`demande`), every write of the other program goes
    /// through. Without it, the chaining program took the database back as soon as it released it and the other one
    /// gave up after `ATTENTE` with « Database already open » (seen in `make check` under load). The request file is
    /// gone at the end, and a request left behind is ignored after `ATTENTE`.
    #[test]
    fn un_programme_qui_enchaine_ne_bloque_pas_l_autre() {
        let d = dossier("enchaine");
        let p = d.join("sensible.redb");
        let p2 = p.clone();
        let debut = std::time::Instant::now();
        let enchaine = std::thread::spawn(move || {
            let mut n = 0;
            while debut.elapsed() < ATTENTE + std::time::Duration::from_millis(500) {
                un_moment(&p2, &CLE, |b| {
                    std::thread::sleep(std::time::Duration::from_millis(30));
                    appliquer(
                        b,
                        &Changements {
                            mettre: vec![trade(&format!("pc-{n}"), 1.0)],
                            ..Default::default()
                        },
                    )
                })
                .unwrap();
                n += 1;
            }
            n
        });
        std::thread::sleep(std::time::Duration::from_millis(50));
        for k in 0..5 {
            un_moment(&p, &CLE, |b| {
                appliquer(
                    b,
                    &Changements {
                        mettre: vec![trade(&format!("cell-{k}"), 2.0)],
                        ..Default::default()
                    },
                )
            })
            .unwrap_or_else(|e| panic!("write {k} of the waiting program: {e}"));
        }
        let n = enchaine.join().unwrap();
        let j = un_moment(&p, &CLE, lire).unwrap();
        assert_eq!(j.trades.len(), n + 5, "every write of both programs");
        assert!(!demande(&p).exists(), "the request file is removed once served");
        // a fresh request left behind by a program that stopped (its process number is gone): ignored, the database
        // is opened while the file is still there
        let mut fini = std::process::Command::new("true").spawn().unwrap();
        fini.wait().unwrap();
        std::fs::write(demande(&p), fini.id().to_string()).unwrap();
        assert!(
            un_moment(&p, &CLE, |_| Ok(demande(&p).exists())).unwrap(),
            "opened without waiting for a dead program"
        );
        // a request of a live program not refreshed for `OUBLI` (frozen or stuck): forgotten too, so the screen of the
        // app never waits longer than that for it
        std::fs::write(demande(&p), std::process::id().to_string()).unwrap();
        let oubliee = std::fs::OpenOptions::new().write(true).open(demande(&p)).unwrap();
        oubliee
            .set_modified(std::time::SystemTime::now() - OUBLI - std::time::Duration::from_secs(1))
            .unwrap();
        assert!(
            un_moment(&p, &CLE, |_| Ok(demande(&p).exists())).unwrap(),
            "a forgotten request does not hold the turn"
        );
        std::fs::remove_dir_all(&d).unwrap();
    }

    /// ⚖️ A waiting program that waits longer than `ATTENTE` (refreshing its request) keeps its turn: the other program
    /// never opens the database before the request is gone, and that wait is not taken from its own `ATTENTE`, so it
    /// still gets the database when it is held a moment after. Before: the request was ignored after `ATTENTE` (the
    /// other one jumped the queue), and a wait for a request used up the whole `ATTENTE`, leaving a single try
    /// (« Database already open » under load).
    #[test]
    fn une_demande_vivante_garde_son_tour() {
        let d = dossier("tour");
        let p = d.join("sensible.redb");
        un_moment(&p, &CLE, lire).unwrap();
        // a live waiting program (this test process), refreshing its request like `un_moment` does
        std::fs::write(demande(&p), std::process::id().to_string()).unwrap();
        let p2 = p.clone();
        let autre = std::thread::spawn(move || un_moment(&p2, &CLE, |_| Ok(std::time::Instant::now())));
        // longer than `ATTENTE`: the other program's whole budget would be gone if this wait counted
        let debut = std::time::Instant::now();
        while debut.elapsed() < ATTENTE + std::time::Duration::from_millis(200) {
            std::thread::sleep(std::time::Duration::from_millis(100));
            assert!(rafraichir(&demande(&p)));
        }
        // the waiting program: it has the database, its request is removed, it holds it a moment
        let tenue = Sensible::ouvrir(&p, &CLE).unwrap();
        std::fs::remove_file(demande(&p)).unwrap();
        let rendue = std::time::Instant::now();
        std::thread::sleep(std::time::Duration::from_millis(300));
        drop(tenue);
        let ouverte = autre.join().unwrap().unwrap_or_else(|e| panic!("the other program gave up: {e}"));
        assert!(ouverte > rendue, "the other program waited for the live request");
        std::fs::remove_dir_all(&d).unwrap();
    }
}
