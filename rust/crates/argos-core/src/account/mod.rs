//! Compte Argos : clé de 24 mots en français (aucun courriel, aucun serveur).
//! Compatible avec le compte actuel du site : mêmes mots, même somme de contrôle, mêmes dérivations PBKDF2.

use pbkdf2::pbkdf2_hmac;
use sha2::{Digest, Sha256};
use unicode_normalization::UnicodeNormalization;

const WORDS_FR: &str = include_str!("mots-fr.txt");
const WORDS_BIP39_EN: &str = include_str!("bip39-en.txt");
const ITERATIONS: u32 = 210_000;

/// Enlève les accents, met en minuscules, enlève les espaces autour (comme `deacc` en JS).
pub fn deaccent(w: &str) -> String {
    w.nfd()
        .filter(|c| !('\u{0300}'..='\u{036f}').contains(c))
        .collect::<String>()
        .to_lowercase()
        .trim()
        .to_string()
}

fn words_fr() -> Vec<String> {
    WORDS_FR.lines().map(deaccent).collect()
}

/// Pourquoi une phrase est refusée.
#[derive(Debug, Clone, PartialEq)]
pub enum PhraseError {
    /// Ça ressemble à une phrase de wallet crypto (anglais) : ne jamais la taper dans Argos.
    LooksLikeWallet,
    WrongCount(usize),
    UnknownWord(String),
    BadChecksum,
}

impl std::fmt::Display for PhraseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LooksLikeWallet => f.write_str(
                "⚠️ Ça ressemble à une phrase de WALLET crypto (en anglais). Ne la mets JAMAIS dans Argos ni ailleurs : ta clé Argos est en français.",
            ),
            Self::WrongCount(n) => write!(f, "Il faut 24 mots (tu en as {n})."),
            Self::UnknownWord(w) => write!(f, "Mot inconnu : {w}"),
            Self::BadChecksum => f.write_str("Un mot est mal écrit ou dans le mauvais ordre (somme de contrôle invalide)."),
        }
    }
}

/// Somme de contrôle (11 bits) des 23 premiers mots : 24e mot de la phrase.
fn checksum(words: &[String], idx: &[usize]) -> usize {
    let text = idx.iter().map(|&i| words[i].as_str()).collect::<Vec<_>>().join(" ");
    let h = Sha256::digest(text.as_bytes());
    (((h[0] as usize) << 3) | ((h[1] as usize) >> 5)) & 2047
}

/// Fabrique une phrase de 24 mots à partir de 23 nombres aléatoires (l'appelant fournit le hasard sécuritaire).
pub fn phrase_from_entropy(random: [u16; 23]) -> Vec<&'static str> {
    let fr: Vec<&'static str> = WORDS_FR.lines().collect();
    let norm = words_fr();
    let mut idx: Vec<usize> = random.iter().map(|&r| (r & 2047) as usize).collect();
    idx.push(checksum(&norm, &idx));
    idx.into_iter().map(|i| fr[i]).collect()
}

/// Vérifie une phrase tapée par l'usager. Renvoie la forme normalisée (sans accents), qui sert à toutes les dérivations.
pub fn check_phrase(text: &str) -> Result<String, PhraseError> {
    let typed: Vec<String> = text
        .split(|c: char| c.is_whitespace() || c == ',')
        .map(deaccent)
        .filter(|w| !w.is_empty())
        .collect();
    let norm = words_fr();
    let known = |w: &String| norm.iter().position(|x| x == w);
    let en: std::collections::HashSet<&str> = WORDS_BIP39_EN.lines().collect();
    if !typed.is_empty() && typed.iter().all(|w| en.contains(w.as_str())) && !typed.iter().all(|w| known(w).is_some()) {
        return Err(PhraseError::LooksLikeWallet);
    }
    if typed.len() != 24 {
        return Err(PhraseError::WrongCount(typed.len()));
    }
    let mut idx = Vec::with_capacity(24);
    for w in &typed {
        match known(w) {
            Some(i) => idx.push(i),
            None => return Err(PhraseError::UnknownWord(w.clone())),
        }
    }
    if checksum(&norm, &idx[..23]) != idx[23] {
        return Err(PhraseError::BadChecksum);
    }
    Ok(idx.iter().map(|&i| norm[i].as_str()).collect::<Vec<_>>().join(" "))
}

fn derive(norm: &str, salt: &str, out: &mut [u8]) {
    pbkdf2_hmac::<Sha256>(norm.as_bytes(), salt.as_bytes(), ITERATIONS, out);
}

/// Identité du compte : un identifiant public + la clé qui chiffre les sauvegardes. Rien de ça ne quitte l'appareil.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountKeys {
    /// 16 caractères hex (8 octets), identifiant du compte sur l'appareil.
    pub id: String,
    /// Clé AES-256 des sauvegardes chiffrées.
    pub backup_key: [u8; 32],
}

pub fn derive_account(norm: &str) -> AccountKeys {
    let mut bits = [0u8; 48];
    derive(norm, "argos-compte-v1", &mut bits);
    let mut backup_key = [0u8; 32];
    backup_key.copy_from_slice(&bits[16..48]);
    AccountKeys {
        id: hex::encode(&bits[..8]),
        backup_key,
    }
}

/// Clé Nostr (secp256k1 / Schnorr) dérivée des 24 mots.
#[derive(Clone, PartialEq, Eq)]
pub struct NostrKeys {
    pub secret_hex: String,
    pub public_hex: String,
}

impl std::fmt::Debug for NostrKeys {
    // la clé privée n'apparaît jamais dans un journal
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NostrKeys {{ public_hex: {} }}", self.public_hex)
    }
}

/// Identifiant de passkey du compte : les 32 octets de sa clé publique Nostr (comme le site).
pub fn nostr_user_id(public_hex: &str) -> Vec<u8> {
    hex::decode(public_hex).unwrap_or_default()
}

/// Clé publique x-only (BIP-340) d'une clé privée hex, ou None si la clé n'est pas valide.
pub fn nostr_public(secret_hex: &str) -> Option<String> {
    let bytes = hex::decode(secret_hex).ok()?;
    let key = k256::schnorr::SigningKey::from_slice(&bytes).ok()?;
    Some(hex::encode(key.verifying_key().to_bytes()))
}

pub fn derive_nostr(norm: &str) -> NostrKeys {
    let mut bits = [0u8; 32];
    derive(norm, "argos-nostr-v1", &mut bits);
    let mut sk = hex::encode(bits);
    // même règle que le site : si la clé tombe hors de la courbe (extrêmement rare), on la re-hache
    for _ in 0..8 {
        if let Some(public_hex) = nostr_public(&sk) {
            return NostrKeys { secret_hex: sk, public_hex };
        }
        sk = hex::encode(Sha256::digest(sk.as_bytes()));
    }
    panic!("clé Nostr impossible à dériver");
}

/// 🆔 Nom d'un ID Argos (`@nom#tag`), comme le site : sans accents, minuscules, espaces → `_`, seulement a-z 0-9 _ .,
/// 20 caractères au plus.
pub fn handle_norm(x: &str) -> String {
    deaccent(x)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("_")
        .chars()
        .filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '_' || *c == '.')
        .take(20)
        .collect()
}

/// Noms réservés (comme le site).
const RESERVED: [&str; 22] = [
    "admin",
    "administrateur",
    "argos",
    "support",
    "aide",
    "help",
    "mod",
    "moderateur",
    "moderator",
    "staff",
    "officiel",
    "official",
    "root",
    "system",
    "systeme",
    "claude",
    "anthropic",
    "hyperliquid",
    "blofin",
    "api",
    "null",
    "undefined",
];

/// Pourquoi un nom (déjà passé par `handle_norm`) est refusé ; `None` : il est bon.
pub fn handle_error(n: &str) -> Option<&'static str> {
    let alnum = |c: Option<char>| c.is_some_and(|c| c.is_ascii_alphanumeric());
    if n.is_empty() {
        Some("Choisis un nom")
    } else if n.len() < 3 {
        Some("Minimum 3 caractères")
    } else if !alnum(n.chars().next()) || !alnum(n.chars().last()) {
        Some("Commence et finis par une lettre ou un chiffre")
    } else if n.contains("..") || n.contains("__") || n.contains("._") || n.contains("_.") {
        Some("Pas deux . ou _ de suite")
    } else if RESERVED.contains(&n) {
        Some("Ce nom est réservé")
    } else {
        None
    }
}

/// `#tag` d'un ID : les 6 premiers caractères de la clé publique Nostr.
pub fn tag(public_hex: &str) -> &str {
    &public_hex[..public_hex.len().min(6)]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Vecteur calculé par le site actuel (JavaScript, WebCrypto + vendor/nostr.js) : le compte Rust doit donner exactement la même chose.
    #[test]
    fn compatible_avec_le_site() {
        let p = "aborder gélule acquérir zoologie abandon algue pleurer agréable abeille vexer abrupt citerne favori réagir aboyer abrasif abreuver abriter incendie décrire boiser appuyer viaduc bustier";
        let n = check_phrase(p).unwrap();
        let a = derive_account(&n);
        assert_eq!(a.id, "8be22d57c919ce10");
        assert_eq!(hex::encode(a.backup_key), "8256d7ac64ae6fc4594727811dfe1b0a3f0f899cee5482845b30a4ca6fab9c3a");
        assert_eq!(derive_nostr(&n).public_hex, "d9507ebddac5521eec6acb977a697b73c2c7cb83c78b320e49ec17a81657b145");
    }

    #[test]
    fn noms_comme_le_site() {
        assert_eq!(handle_norm("  Jée Trader "), "jee_trader");
        assert_eq!(handle_norm("a!b@c#"), "abc");
        assert_eq!(handle_error("jee"), None);
        assert_eq!(handle_error("je"), Some("Minimum 3 caractères"));
        assert_eq!(handle_error("_jee"), Some("Commence et finis par une lettre ou un chiffre"));
        assert_eq!(handle_error("je..e"), Some("Pas deux . ou _ de suite"));
        assert_eq!(handle_error("argos"), Some("Ce nom est réservé"));
        assert_eq!(tag("abcdef0123"), "abcdef");
    }

    #[test]
    fn listes_completes() {
        assert_eq!(WORDS_FR.lines().count(), 2048);
        assert_eq!(WORDS_BIP39_EN.lines().count(), 2048);
    }

    #[test]
    fn phrase_generee_valide_et_accents_toleres() {
        let p = phrase_from_entropy([7; 23]).join(" ");
        assert!(check_phrase(&p).is_ok());
        // tapée en majuscules et avec des virgules : même résultat
        assert_eq!(check_phrase(&p.to_uppercase().replace(' ', ", ")), check_phrase(&p));
    }

    #[test]
    fn erreurs_claires() {
        assert_eq!(check_phrase("abaisser abandon"), Err(PhraseError::WrongCount(2)));
        let wallet = "abandon ability able about above absent absorb abstract absurd abuse access accident";
        assert_eq!(check_phrase(wallet), Err(PhraseError::LooksLikeWallet));
        let mut w: Vec<&str> = phrase_from_entropy([1; 23]);
        w.swap(0, 1);
        if w[0] != w[1] {
            assert_eq!(check_phrase(&w.join(" ")), Err(PhraseError::BadChecksum));
        }
    }
}
