//! 🆘 Rescue code of the encrypted journal: the device key (32 bytes, kept in the system keyring) written as readable
//! groups with a checksum, so that the journal (`sensible.redb`) can still be opened after the keyring is lost or the
//! computer is reinstalled. The code is shown only on an explicit request and is never written to a file or a log;
//! restoring checks that the code really opens the journal before putting the key back in the keyring.
//!
//! Format: the 32 key bytes, then the first 2 bytes of their SHA-256 (checksum), in Crockford base 32 (no I, L, O, U;
//! read with O = 0 and I = L = 1), 55 characters in 11 groups of 5 separated by « - ».

use crate::journal;
use crate::sensible::Cle;
use std::path::Path;

/// Crockford base 32: digits and letters without I, L, O, U.
const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
/// Characters of a code (34 bytes = 272 bits, 5 bits per character).
const LONGUEUR: usize = 55;
/// Characters per group.
const GROUPE: usize = 5;

/// The 2 checksum bytes of a key.
fn controle(cle: &Cle) -> [u8; 2] {
    let h = argos_core::sha256(cle);
    [h[0], h[1]]
}

/// The rescue code of `cle`.
pub fn code(cle: &Cle) -> String {
    let mut octets = cle.to_vec();
    octets.extend(controle(cle));
    let mut lettres = String::new();
    let (mut acc, mut bits) = (0u32, 0u32);
    for o in octets {
        acc = (acc << 8) | u32::from(o);
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            lettres.push(ALPHABET[((acc >> bits) & 31) as usize] as char);
        }
    }
    if bits > 0 {
        lettres.push(ALPHABET[((acc << (5 - bits)) & 31) as usize] as char);
    }
    lettres
        .as_bytes()
        .chunks(GROUPE)
        .map(|g| String::from_utf8_lossy(g).into_owned())
        .collect::<Vec<_>>()
        .join("-")
}

/// The key of a rescue code, as typed (spaces, dashes and lower case accepted; O read as 0, I and L as 1). A wrong
/// length or a wrong checksum: a clear error, never a wrong key.
pub fn cle(code: &str) -> Result<Cle, String> {
    let mut valeurs = vec![];
    for c in code.chars().filter(|c| !c.is_whitespace() && *c != '-') {
        let c = match c.to_ascii_uppercase() {
            'O' => '0',
            'I' | 'L' => '1',
            c => c,
        };
        let v = ALPHABET
            .iter()
            .position(|&a| a as char == c)
            .ok_or_else(|| format!("code de secours : caractère « {c} » inconnu"))?;
        valeurs.push(v as u32);
    }
    if valeurs.len() != LONGUEUR {
        return Err(format!("code de secours : {} caractères au lieu de {LONGUEUR}", valeurs.len()));
    }
    let mut octets = vec![];
    let (mut acc, mut bits) = (0u32, 0u32);
    for v in valeurs {
        acc = (acc << 5) | v;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            octets.push(((acc >> bits) & 0xff) as u8);
        }
    }
    let (cle, somme) = octets.split_at(32);
    let cle: Cle = cle.try_into().map_err(|_| "code de secours illisible".to_string())?;
    if somme != controle(&cle) || acc & ((1 << bits) - 1) != 0 {
        return Err("code de secours mal recopié (la somme de contrôle ne correspond pas)".into());
    }
    Ok(cle)
}

/// The key of `code`, once checked against the journal at `chemin`: the journal exists and opens with it.
pub fn verifier(chemin: &Path, code: &str) -> Result<Cle, String> {
    let cle = cle(code)?;
    if !chemin.exists() {
        return Err("pas de journal chiffré sur cet appareil : rien à restaurer".into());
    }
    journal::un_moment(chemin, &cle, journal::lire).map_err(|_| "ce code n'ouvre pas ce journal".to_string())?;
    Ok(cle)
}

/// The rescue code of the key kept in the keyring (`gardee`), once checked against the journal at `chemin` (a key
/// that does not open it would give a useless code). No journal yet: the code of the key that will write it.
pub fn code_de_l_appareil(chemin: &Path, gardee: impl Fn() -> Result<Option<Cle>, String>) -> Result<String, String> {
    let cle = gardee()?.ok_or("aucune clé du journal sur cet appareil")?;
    if chemin.exists() {
        journal::un_moment(chemin, &cle, journal::lire)
            .map_err(|_| "la clé de cet appareil n'ouvre pas le journal : restaure-la d'abord avec ton ancien code".to_string())?;
    }
    Ok(code(&cle))
}

/// Restores the device key from `code`: checked against the journal (`verifier`), then put in the system keyring.
pub fn restaurer(chemin: &Path, code: &str) -> Result<(), String> {
    restaurer_avec(chemin, code, journal::cle_gardee, journal::garder_cle)
}

/// `restaurer` with the keyring given (`gardee`: the key kept, `garder`: keep a key; tests use a made-up keyring).
/// Refused when the key already kept opens the journal and differs: nothing to repair, never replace a good key.
pub fn restaurer_avec(
    chemin: &Path,
    code: &str,
    gardee: impl Fn() -> Result<Option<Cle>, String>,
    garder: impl Fn(&Cle) -> Result<(), String>,
) -> Result<(), String> {
    let cle = verifier(chemin, code)?;
    if let Some(actuelle) = gardee()? {
        if actuelle == cle {
            return Ok(());
        }
        if journal::un_moment(chemin, &actuelle, journal::lire).is_ok() {
            return Err("la clé de cet appareil ouvre déjà le journal : rien à restaurer".into());
        }
    }
    garder(&cle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal::Changements;
    use argos_core::journal::JTrade;

    /// 🧪 Made-up test keys (never the real device key).
    const CLE: Cle = [7; 32];
    const AUTRE: Cle = [9; 32];

    fn cle_variee() -> Cle {
        let mut c = [0u8; 32];
        for (i, o) in c.iter_mut().enumerate() {
            *o = (i as u8).wrapping_mul(37).wrapping_add(11);
        }
        c
    }

    /// The code is readable (11 groups of 5 from the alphabet) and gives the key back, typed in any way.
    #[test]
    fn code_aller_retour() {
        for k in [CLE, AUTRE, cle_variee(), [0; 32], [255; 32]] {
            let c = code(&k);
            assert_eq!(c.split('-').count(), 11, "{c}");
            assert!(c.split('-').all(|g| g.len() == GROUPE && g.bytes().all(|b| ALPHABET.contains(&b))), "{c}");
            assert_eq!(cle(&c), Ok(k));
            let tape = c.to_lowercase().replace('-', " ").replace('0', "o").replace('1', "l");
            assert_eq!(cle(&tape), Ok(k), "lower case, spaces, O for 0, L for 1");
        }
    }

    /// One character changed, swapped, missing or unknown: refused with a clear message, never another key.
    #[test]
    fn code_mal_recopie_refuse() {
        let c = code(&cle_variee());
        let lettres: Vec<char> = c.chars().filter(|&x| x != '-').collect();
        for i in 0..lettres.len() {
            let mut faux = lettres.clone();
            faux[i] = if faux[i] == 'A' { 'B' } else { 'A' };
            let faux: String = faux.into_iter().collect();
            assert!(cle(&faux).is_err(), "changed at {i}");
        }
        let mut inverse = lettres.clone();
        let i = (0..inverse.len() - 1).find(|&i| inverse[i] != inverse[i + 1]).unwrap();
        inverse.swap(i, i + 1);
        assert!(cle(&inverse.into_iter().collect::<String>()).is_err(), "two characters swapped");
        assert!(cle(&c[..c.len() - 1]).unwrap_err().contains("54 caractères"));
        assert!(cle(&c.replacen('-', "U", 1)).unwrap_err().contains("« U » inconnu"));
    }

    /// Restoring checks the journal first: no journal, or a code of another key, refused; the right code opens it.
    #[test]
    fn verifier_contre_le_journal() {
        let d = argos_core::dossiers::de_test("journal-secours");
        let p = d.join("sensible.redb");
        let _ = std::fs::remove_dir_all(&d);
        assert_eq!(
            verifier(&p, &code(&CLE)).unwrap_err(),
            "pas de journal chiffré sur cet appareil : rien à restaurer"
        );
        assert!(!p.exists(), "checking never creates a journal");
        journal::un_moment(&p, &CLE, |b| {
            journal::appliquer(
                b,
                &Changements {
                    mettre: vec![JTrade {
                        id: "t1".into(),
                        ..Default::default()
                    }],
                    ..Default::default()
                },
            )
        })
        .unwrap();
        assert_eq!(verifier(&p, &code(&AUTRE)).unwrap_err(), "ce code n'ouvre pas ce journal");
        assert_eq!(verifier(&p, &code(&CLE)), Ok(CLE));
        // a made-up keyring: what restoring keeps in it
        let garde = std::cell::Cell::new(None::<Cle>);
        let garder = |c: &Cle| {
            garde.set(Some(*c));
            Ok(())
        };
        // new computer: no key, or a new key that does not open the journal → the code's key is kept
        assert_eq!(restaurer_avec(&p, &code(&CLE), || Ok(None), garder), Ok(()));
        assert_eq!(garde.take(), Some(CLE));
        assert_eq!(restaurer_avec(&p, &code(&CLE), || Ok(Some(AUTRE)), garder), Ok(()));
        assert_eq!(garde.take(), Some(CLE));
        // the kept key already opens the journal: nothing replaced
        assert_eq!(restaurer_avec(&p, &code(&CLE), || Ok(Some(CLE)), garder), Ok(()));
        assert_eq!(garde.take(), None, "same key: nothing written");
        // a wrong code: refused before touching the keyring
        assert!(restaurer_avec(&p, &code(&AUTRE), || Ok(None), garder).is_err());
        assert_eq!(garde.take(), None);
        // an empty journal opens with any key: the kept key works, it is never replaced
        let vide = d.join("vide.redb");
        journal::un_moment(&vide, &AUTRE, journal::lire).unwrap();
        assert_eq!(
            restaurer_avec(&vide, &code(&CLE), || Ok(Some(AUTRE)), garder).unwrap_err(),
            "la clé de cet appareil ouvre déjà le journal : rien à restaurer"
        );
        assert_eq!(garde.take(), None);
        std::fs::remove_dir_all(&d).unwrap();
    }
}
