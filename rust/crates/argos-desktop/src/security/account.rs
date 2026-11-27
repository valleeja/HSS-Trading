//! ✨ Compte Argos sur cet appareil : créé à partir d'une **clé de 24 mots** (aucun courriel, aucun serveur), comme le site.
//! Les mots donnent l'identifiant du compte, la clé des sauvegardes et la clé Nostr (`argos_core::account`, mêmes
//! dérivations que le site : le même compte partout). Ce qui est gardé :
//! - `account.json` : le public seulement (identifiant, nom, clé publique Nostr, date) ;
//! - `account-key.json` : la clé Nostr et la clé des sauvegardes, **chiffrées avec ton code** (coffre `vault`).
//!
//! Les 24 mots ne sont jamais écrits : ils vivent en mémoire le temps de la création, puis sont effacés.

use argos_core::account as core;
use base64::{engine::general_purpose::STANDARD as B64, Engine};

/// Le compte (partie publique), gardé dans `account.json`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Account {
    /// Identifiant du compte (16 caractères hex), le même que sur le site.
    pub id: String,
    /// Nom de l'ID (`@nom#tag`).
    pub handle: String,
    /// Clé publique Nostr (hex) : son début fait le `#tag`.
    pub pub_hex: String,
    /// Création (secondes Unix).
    pub created: i64,
}

impl Account {
    /// `@nom#tag`.
    pub fn id_full(&self) -> String {
        format!("@{}#{}", self.handle, core::tag(&self.pub_hex))
    }
}

/// Fichier du compte (public).
pub const FILE: &str = "account.json";

/// Création en cours : le nom, le code et les 24 mots, en mémoire seulement (effacés à la fin, même en cas d'erreur).
pub struct Draft {
    pub handle: String,
    code: String,
    pub words: Vec<String>,
    /// Les 3 mots demandés à la vérification (0 à 23).
    pub ask: [usize; 3],
}

impl Drop for Draft {
    fn drop(&mut self) {
        for s in self.words.iter_mut().chain(std::iter::once(&mut self.code)) {
            let n = s.len();
            s.clear();
            s.extend(std::iter::repeat_n('0', n));
            s.clear();
        }
    }
}

/// Hasard du système (23 nombres de 11 bits utiles, plus 3 pour choisir les mots à vérifier).
fn random_u16<const N: usize>() -> Result<[u16; N], String> {
    let mut b = vec![0u8; N * 2];
    getrandom::fill(&mut b).map_err(|e| format!("hasard du système : {e}"))?;
    let mut out = [0u16; N];
    for (i, v) in out.iter_mut().enumerate() {
        *v = u16::from_le_bytes([b[2 * i], b[2 * i + 1]]);
    }
    Ok(out)
}

/// Étape 1 : nom et code vérifiés, puis 24 mots tirés au hasard (et les 3 qu'on redemandera).
pub fn begin(handle: &str, code: &str, confirm: &str) -> Result<Draft, String> {
    let handle = core::handle_norm(handle);
    if let Some(e) = core::handle_error(&handle) {
        return Err(e.into());
    }
    if code.chars().count() < 4 {
        return Err("Ton code : 4 caractères ou plus".into());
    }
    if code != confirm {
        return Err("Les deux codes ne sont pas pareils".into());
    }
    let words = core::phrase_from_entropy(random_u16::<23>()?).into_iter().map(str::to_string).collect();
    // 3 numéros différents, en ordre
    let mut ask: Vec<usize> = vec![];
    while ask.len() < 3 {
        let [r] = random_u16::<1>()?;
        let i = r as usize % 24;
        if !ask.contains(&i) {
            ask.push(i);
        }
    }
    ask.sort_unstable();
    Ok(Draft {
        handle,
        code: code.to_string(),
        words,
        ask: [ask[0], ask[1], ask[2]],
    })
}

/// Les mots retapés sont-ils les bons (accents et majuscules tolérés) ?
pub fn verify(draft: &Draft, typed: &[String]) -> bool {
    typed.len() == 3 && draft.ask.iter().zip(typed).all(|(&i, t)| core::deaccent(t) == core::deaccent(&draft.words[i]))
}

/// Dérive les clés d'une phrase vérifiée (`norm`), les garde chiffrées avec le code, et rend le compte (public).
fn seal_and_save(norm: &str, handle: &str, code: &str, now: i64) -> Result<Account, String> {
    let keys = core::derive_account(norm);
    let nostr = core::derive_nostr(norm);
    let secret = serde_json::json!({ "v": 1, "id": keys.id, "nostr_sk": nostr.secret_hex, "backup_key": B64.encode(keys.backup_key) }).to_string();
    let sealed = super::vault::seal(&secret, code)?;
    super::vault::store(super::vault::ACCOUNT_FILE, &sealed)?;
    let acc = Account {
        id: keys.id,
        handle: handle.to_string(),
        pub_hex: nostr.public_hex,
        created: now,
    };
    crate::platform::write(FILE, &serde_json::to_string_pretty(&acc).map_err(|e| e.to_string())?)?;
    Ok(acc)
}

/// Étape finale de la création : les clés des 24 mots, gardées chiffrées avec le code.
pub fn create(draft: &Draft, now: i64) -> Result<Account, String> {
    let norm = core::check_phrase(&draft.words.join(" ")).map_err(|e| e.to_string())?;
    seal_and_save(&norm, &draft.handle, &draft.code, now)
}

/// 🔑 « J'ai déjà une clé » : retrouve le compte de ces 24 mots (même compte que sur le site ou un autre appareil),
/// avec le nom de l'ID et un code pour cet appareil. Une phrase de wallet crypto (anglais) est refusée.
pub fn restore(phrase: &str, handle: &str, code: &str, confirm: &str, now: i64) -> Result<Account, String> {
    let norm = core::check_phrase(phrase).map_err(|e| e.to_string())?;
    let handle = core::handle_norm(handle);
    if let Some(e) = core::handle_error(&handle) {
        return Err(e.into());
    }
    if code.chars().count() < 4 {
        return Err("Ton code : 4 caractères ou plus".into());
    }
    if code != confirm {
        return Err("Les deux codes ne sont pas pareils".into());
    }
    seal_and_save(&norm, &handle, code, now)
}

/// Le compte gardé sur cet appareil (s'il y en a un).
pub fn load() -> Option<Account> {
    serde_json::from_str(&crate::platform::read(FILE)?).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creation_pas_a_pas() {
        assert_eq!(begin("je", "1234", "1234").err().as_deref(), Some("Minimum 3 caractères"));
        assert_eq!(begin("jee", "12", "12").err().as_deref(), Some("Ton code : 4 caractères ou plus"));
        assert_eq!(begin("jee", "1234", "4321").err().as_deref(), Some("Les deux codes ne sont pas pareils"));
        let d = begin(" Jée ", "1234", "1234").unwrap();
        assert_eq!(d.handle, "jee");
        assert_eq!(d.words.len(), 24);
        assert!(core::check_phrase(&d.words.join(" ")).is_ok(), "somme de contrôle du site");
        assert!(d.ask[0] < d.ask[1] && d.ask[1] < d.ask[2] && d.ask[2] < 24);
        let good: Vec<String> = d.ask.iter().map(|&i| d.words[i].to_uppercase()).collect();
        assert!(verify(&d, &good), "majuscules tolérées");
        let bad: Vec<String> = vec!["x".into(), "y".into(), "z".into()];
        assert!(!verify(&d, &bad));
    }

    #[test]
    fn retrouver_refuse_les_mauvaises_phrases() {
        let wallet = "abandon ability able about above absent absorb abstract absurd abuse access accident account accuse achieve acid acoustic acquire across act action actor actress actual";
        assert!(restore(wallet, "jee", "1234", "1234", 0).unwrap_err().contains("WALLET"));
        assert!(restore("abaisser abandon", "jee", "1234", "1234", 0).unwrap_err().contains("24 mots"));
    }
}
