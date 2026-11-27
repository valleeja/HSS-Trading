//! 🆘 Rescue code of the encrypted journal, in the terminal of the computer only (never through an HTTP route: the key
//! and its code never go through the browser nor the network):
//! - `argos-serveur code-secours`: shows the code once, after a typed confirmation;
//! - `argos-serveur restaurer-code`: reads a code typed on the keyboard, checks it opens the journal, keeps the key in
//!   the keyring.
//!
//! Nothing is written to a file or a log. The database is opened only for the time of an operation
//! (`journal::un_moment`), so these commands work while the server on port 8000 is running.

use argos_store::secours;
use argos_store::sensible::Cle;
use std::io::{BufRead, Write};
use std::path::Path;

/// The word to type to show the code.
const MOT: &str = "AFFICHER";

/// One line typed on the keyboard (end of input: empty).
fn ligne(entree: &mut impl BufRead) -> Result<String, String> {
    let mut l = String::new();
    entree.read_line(&mut l).map_err(|e| format!("clavier : {e}"))?;
    Ok(l.trim().to_string())
}

/// `code-secours`: after the typed confirmation, shows the code of the key kept in the keyring (`gardee`), checked
/// against the journal at `chemin` (`secours::code_de_l_appareil`).
pub fn afficher(entree: &mut impl BufRead, sortie: &mut impl Write, chemin: &Path, gardee: impl Fn() -> Result<Option<Cle>, String>) -> Result<(), String> {
    let ecrire = |s: &mut dyn Write, t: &str| writeln!(s, "{t}").map_err(|e| format!("écran : {e}"));
    ecrire(
        sortie,
        "🆘 Code de secours du journal chiffré : avec lui, ton journal se rouvre même si le trousseau est perdu.",
    )?;
    ecrire(
        sortie,
        "Note-le sur papier et garde-le à l'abri. Jamais en photo, jamais dans un fichier ni un message.",
    )?;
    write!(sortie, "Tape {MOT} pour le voir (Entrée seule : annuler) : ").map_err(|e| format!("écran : {e}"))?;
    sortie.flush().map_err(|e| format!("écran : {e}"))?;
    if ligne(entree)? != MOT {
        return ecrire(sortie, "Annulé : rien n'est affiché.");
    }
    let code = secours::code_de_l_appareil(chemin, gardee).map_err(|e| if e.contains("n'ouvre pas") { format!("{e} (restaurer-code)") } else { e })?;
    ecrire(sortie, &format!("\n    {code}\n"))?;
    ecrire(sortie, "Recopie-le sans faute : 11 groupes de 5 caractères.")
}

/// `restaurer-code`: reads a code on the keyboard and restores the key (`secours::restaurer_avec`).
pub fn restaurer(
    entree: &mut impl BufRead,
    sortie: &mut impl Write,
    chemin: &Path,
    gardee: impl Fn() -> Result<Option<Cle>, String>,
    garder: impl Fn(&Cle) -> Result<(), String>,
) -> Result<(), String> {
    write!(sortie, "🆘 Tape ton code de secours (espaces et tirets permis) : ").map_err(|e| format!("écran : {e}"))?;
    sortie.flush().map_err(|e| format!("écran : {e}"))?;
    let code = ligne(entree)?;
    secours::restaurer_avec(chemin, &code, gardee, garder)?;
    writeln!(sortie, "✅ Clé du journal restaurée dans le trousseau : relance l'app et le serveur.").map_err(|e| format!("écran : {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    /// 🧪 Made-up test keys (never the real device key).
    const CLE: Cle = [3; 32];
    const AUTRE: Cle = [5; 32];

    /// A journal written with `cle`, in the build folder of the project.
    fn journal_de_test(nom: &str, cle: &Cle) -> std::path::PathBuf {
        let d = argos_core::dossiers::de_test(nom);
        let _ = std::fs::remove_dir_all(&d);
        let p = d.join("sensible.redb");
        argos_store::journal::un_moment(&p, cle, |b| {
            argos_store::journal::appliquer(
                b,
                &argos_store::journal::Changements {
                    mettre: vec![argos_core::journal::JTrade {
                        id: "t1".into(),
                        ..Default::default()
                    }],
                    ..Default::default()
                },
            )
        })
        .unwrap();
        p
    }

    fn texte(v: Vec<u8>) -> String {
        String::from_utf8(v).unwrap()
    }

    /// The code shows only after « AFFICHER » typed; anything else cancels; a key that does not open the journal
    /// shows nothing.
    #[test]
    fn afficher_seulement_apres_confirmation() {
        let p = journal_de_test("serveur-secours-voir", &CLE);
        let mut sortie = vec![];
        afficher(&mut Cursor::new("AFFICHER\n"), &mut sortie, &p, || Ok(Some(CLE))).unwrap();
        assert!(texte(sortie).contains(&secours::code(&CLE)));
        for refus in ["\n", "afficher\n", "oui\n", ""] {
            let mut sortie = vec![];
            afficher(&mut Cursor::new(refus), &mut sortie, &p, || Ok(Some(CLE))).unwrap();
            let t = texte(sortie);
            assert!(t.contains("Annulé") && !t.contains(&secours::code(&CLE)), "{refus:?} : {t}");
        }
        let mut sortie = vec![];
        let e = afficher(&mut Cursor::new("AFFICHER\n"), &mut sortie, &p, || Ok(Some(AUTRE))).unwrap_err();
        assert!(e.contains("n'ouvre pas le journal") && !texte(sortie).contains(&secours::code(&AUTRE)));
        let e = afficher(&mut Cursor::new("AFFICHER\n"), &mut vec![], &p, || Ok(None)).unwrap_err();
        assert_eq!(e, "aucune clé du journal sur cet appareil");
        std::fs::remove_dir_all(p.parent().unwrap()).unwrap();
    }

    /// The code typed (with spaces) restores the key in the keyring; a wrong code keeps nothing.
    #[test]
    fn restaurer_au_clavier() {
        let p = journal_de_test("serveur-secours-restaurer", &CLE);
        let garde = std::cell::Cell::new(None::<Cle>);
        let garder = |c: &Cle| {
            garde.set(Some(*c));
            Ok(())
        };
        let tape = format!("{}\n", secours::code(&CLE).replace('-', " "));
        let mut sortie = vec![];
        restaurer(&mut Cursor::new(tape), &mut sortie, &p, || Ok(Some(AUTRE)), garder).unwrap();
        assert!(texte(sortie).contains("✅ Clé du journal restaurée"));
        assert_eq!(garde.take(), Some(CLE));
        let faux = format!("{}\n", secours::code(&AUTRE));
        assert_eq!(
            restaurer(&mut Cursor::new(faux), &mut vec![], &p, || Ok(None), garder).unwrap_err(),
            "ce code n'ouvre pas ce journal"
        );
        assert_eq!(garde.take(), None);
        std::fs::remove_dir_all(p.parent().unwrap()).unwrap();
    }
}
