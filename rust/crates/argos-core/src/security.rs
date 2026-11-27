//! Sécurité et sandbox d'Argos.
//! - La session reste verrouillée tant que la clé gardée ne correspond pas au compte (rien n'est signé).
//! - Le wallet est en lecture seule et doit être reconfirmé à chaque ouverture, sinon : mode démo.
//! - Les requêtes sensibles au site sont signées NIP-98 (le serveur ne reçoit jamais les 24 mots ni la clé privée).

use crate::account::nostr_public;
use crate::types::{ArgosError, Result};
use k256::schnorr::{Signature, SigningKey, VerifyingKey};
use sha2::{Digest, Sha256};

/// État de la session sur l'appareil.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Session {
    /// Aucun compte : mode invité / démo.
    Guest,
    /// Compte connu mais clé absente ou invalide : démo, rien n'est signé.
    Locked { public_hex: String },
    /// Clé vérifiée : peut signer.
    Unlocked { public_hex: String },
}

impl Session {
    /// Vérifie, sur l'appareil et sans réseau, que la clé gardée correspond bien à la clé publique du compte.
    pub fn check(account_public_hex: Option<&str>, stored_secret_hex: Option<&str>) -> Session {
        match (account_public_hex, stored_secret_hex) {
            (None, _) => Session::Guest,
            (Some(p), Some(sk)) if nostr_public(sk).as_deref() == Some(p) => Session::Unlocked { public_hex: p.to_string() },
            (Some(p), _) => Session::Locked { public_hex: p.to_string() },
        }
    }
    pub fn can_sign(&self) -> bool {
        matches!(self, Session::Unlocked { .. })
    }
}

/// Wallet : Argos lit seulement l'adresse publique. À chaque ouverture, le wallet doit la reconfirmer
/// (demande silencieuse, sans signature). Une adresse collée à la main n'est jamais confirmée.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WalletState {
    None,
    Demo { address: String },
    Confirmed { address: String },
}

pub fn confirm_wallet(saved_address: Option<&str>, pasted_by_hand: bool, accounts_reported_by_wallet: &[String]) -> WalletState {
    let Some(addr) = saved_address else { return WalletState::None };
    let ok = !pasted_by_hand && accounts_reported_by_wallet.iter().any(|a| a.eq_ignore_ascii_case(addr));
    if ok {
        WalletState::Confirmed { address: addr.to_string() }
    } else {
        WalletState::Demo { address: addr.to_string() }
    }
}

// ---------- Événements Nostr signés ----------

/// Chaîne JSON identique à `JSON.stringify` (nécessaire pour que l'identifiant d'événement soit le même partout).
fn json_str(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            '\u{08}' => o.push_str("\\b"),
            '\u{0c}' => o.push_str("\\f"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

/// Un événement Nostr signé.
#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    pub id: String,
    pub pubkey: String,
    pub created_at: i64,
    pub kind: u32,
    pub tags: Vec<Vec<String>>,
    pub content: String,
    pub sig: String,
}

impl Event {
    fn serialize(pubkey: &str, created_at: i64, kind: u32, tags: &[Vec<String>], content: &str) -> String {
        let tags = tags
            .iter()
            .map(|t| format!("[{}]", t.iter().map(|s| json_str(s)).collect::<Vec<_>>().join(",")))
            .collect::<Vec<_>>()
            .join(",");
        format!("[0,{},{},{},[{}],{}]", json_str(pubkey), created_at, kind, tags, json_str(content))
    }

    /// Signe un événement. Refuse si la session n'est pas déverrouillée.
    pub fn sign(session: &Session, secret_hex: &str, created_at: i64, kind: u32, tags: Vec<Vec<String>>, content: &str, aux_rand: [u8; 32]) -> Result<Event> {
        let Session::Unlocked { public_hex } = session else {
            return Err(ArgosError::Denied("compte verrouillé : reconnecte-toi avec tes 24 mots".into()));
        };
        let key_bytes = hex::decode(secret_hex).map_err(|_| ArgosError::Invalid("clé privée".into()))?;
        let key = SigningKey::from_slice(&key_bytes).map_err(|_| ArgosError::Invalid("clé privée".into()))?;
        if hex::encode(key.verifying_key().to_bytes()) != *public_hex {
            return Err(ArgosError::Denied("la clé ne correspond pas au compte".into()));
        }
        let id = Sha256::digest(Self::serialize(public_hex, created_at, kind, &tags, content).as_bytes());
        let sig = key.sign_raw(&id, &aux_rand).map_err(|_| ArgosError::Invalid("signature".into()))?;
        Ok(Event {
            id: hex::encode(id),
            pubkey: public_hex.clone(),
            created_at,
            kind,
            tags,
            content: content.to_string(),
            sig: hex::encode(sig.to_bytes()),
        })
    }

    /// Vérifie l'identifiant et la signature Schnorr (BIP-340).
    pub fn verify(&self) -> bool {
        let id = Sha256::digest(Self::serialize(&self.pubkey, self.created_at, self.kind, &self.tags, &self.content).as_bytes());
        if hex::encode(id) != self.id {
            return false;
        }
        let (Ok(pk), Ok(sig)) = (hex::decode(&self.pubkey), hex::decode(&self.sig)) else {
            return false;
        };
        let (Ok(vk), Ok(sig)) = (VerifyingKey::from_slice(&pk), Signature::try_from(sig.as_slice())) else {
            return false;
        };
        vk.verify_raw(&id, &sig).is_ok()
    }

    /// Le tag `k` (premier élément) → sa valeur.
    pub fn tag(&self, k: &str) -> Option<&str> {
        self.tags
            .iter()
            .find(|t| t.first().map(String::as_str) == Some(k))
            .and_then(|t| t.get(1))
            .map(String::as_str)
    }
}

/// NIP-98 : vérifie qu'un événement autorise CETTE requête (kind 27235, ≤ 60 s, même chemin et méthode).
pub fn nip98_check(ev: &Event, method: &str, path_and_query: &str, now: i64) -> Result<String> {
    if ev.kind != 27235 {
        return Err(ArgosError::Denied("mauvais type d'événement".into()));
    }
    if (now - ev.created_at).abs() > 60 {
        return Err(ArgosError::Denied("signature expirée".into()));
    }
    let u = ev.tag("u").unwrap_or("");
    let path = u.find("://").and_then(|i| u[i + 3..].find('/').map(|j| &u[i + 3 + j..])).unwrap_or(u);
    if path != path_and_query {
        return Err(ArgosError::Denied("la signature ne correspond pas à cette adresse".into()));
    }
    if !ev.tag("method").unwrap_or("").eq_ignore_ascii_case(method) {
        return Err(ArgosError::Denied("la signature ne correspond pas à cette méthode".into()));
    }
    if !ev.verify() {
        return Err(ArgosError::Denied("signature invalide".into()));
    }
    Ok(ev.pubkey.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::{check_phrase, derive_nostr, phrase_from_entropy};

    fn keys() -> crate::account::NostrKeys {
        derive_nostr(&check_phrase(&phrase_from_entropy([3; 23]).join(" ")).unwrap())
    }

    #[test]
    fn session_verrouillee_sans_bonne_cle() {
        let k = keys();
        assert!(Session::check(Some(&k.public_hex), Some(&k.secret_hex)).can_sign());
        assert!(!Session::check(Some(&k.public_hex), None).can_sign());
        assert!(!Session::check(Some(&k.public_hex), Some(&"11".repeat(32))).can_sign());
        assert_eq!(Session::check(None, None), Session::Guest);
    }

    #[test]
    fn wallet_demo_tant_que_pas_confirme() {
        let a = "0xAbC0000000000000000000000000000000000001";
        assert_eq!(
            confirm_wallet(Some(a), false, &[a.to_lowercase()]),
            WalletState::Confirmed { address: a.into() }
        );
        assert_eq!(confirm_wallet(Some(a), false, &[]), WalletState::Demo { address: a.into() });
        assert_eq!(confirm_wallet(Some(a), true, &[a.into()]), WalletState::Demo { address: a.into() });
    }

    #[test]
    fn nip98_signe_et_verifie() {
        let k = keys();
        let s = Session::check(Some(&k.public_hex), Some(&k.secret_hex));
        let tags = vec![vec!["u".into(), "https://argos.app/api/me".into()], vec!["method".into(), "GET".into()]];
        let ev = Event::sign(&s, &k.secret_hex, 1_000, 27235, tags, "", [9; 32]).unwrap();
        assert_eq!(nip98_check(&ev, "GET", "/api/me", 1_030), Ok(k.public_hex.clone()));
        assert!(nip98_check(&ev, "POST", "/api/me", 1_030).is_err());
        assert!(nip98_check(&ev, "GET", "/api/ai", 1_030).is_err());
        assert!(nip98_check(&ev, "GET", "/api/me", 1_200).is_err());
        let mut bad = ev.clone();
        bad.content = "x".into();
        assert!(!bad.verify());
        // compte verrouillé : refus de signer
        assert!(Event::sign(
            &Session::Locked {
                public_hex: k.public_hex.clone()
            },
            &k.secret_hex,
            1,
            1,
            vec![],
            "",
            [0; 32]
        )
        .is_err());
    }

    #[test]
    fn json_comme_javascript() {
        assert_eq!(json_str("a\"b\\c\n\u{1}é"), "\"a\\\"b\\\\c\\n\\u0001é\"");
    }
}
