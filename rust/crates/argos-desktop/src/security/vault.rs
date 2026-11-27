//! 🔐 Coffres d'Argos : un secret chiffré (AES-256-GCM) avec une clé tirée de **ton code** (PBKDF2-SHA256,
//! 200 000 tours, sel au hasard). Seul le chiffré est gardé (dossier de config sur desktop, stockage de la page dans
//! le navigateur) ; le secret en clair reste seulement en mémoire, le temps de la session, et le code n'est jamais gardé.
//! Deux coffres : la clé Claude (`claude-key.json`) et les clés du compte Argos (`account-key.json`).

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::{engine::general_purpose::STANDARD as B64, Engine};

/// Coffre de la clé Claude.
pub const FILE: &str = "claude-key.json";
/// Coffre des clés du compte Argos (clé Nostr et clé des sauvegardes, tirées des 24 mots).
pub const ACCOUNT_FILE: &str = "account-key.json";
const ROUNDS: u32 = 200_000;

/// Le coffre tel qu'il est gardé.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Sealed {
    pub salt: String,
    pub nonce: String,
    pub data: String,
}

fn derive(code: &str, salt: &[u8]) -> [u8; 32] {
    let mut k = [0u8; 32];
    pbkdf2::pbkdf2_hmac::<sha2::Sha256>(code.as_bytes(), salt, ROUNDS, &mut k);
    k
}

/// Chiffre `secret` avec `code`.
pub fn seal(secret: &str, code: &str) -> Result<Sealed, String> {
    let (mut salt, mut nonce) = ([0u8; 16], [0u8; 12]);
    getrandom::fill(&mut salt).map_err(|e| e.to_string())?;
    getrandom::fill(&mut nonce).map_err(|e| e.to_string())?;
    let cipher = Aes256Gcm::new_from_slice(&derive(code, &salt)).map_err(|e| e.to_string())?;
    let data = cipher
        .encrypt(Nonce::from_slice(&nonce), secret.as_bytes())
        .map_err(|_| "chiffrement impossible".to_string())?;
    Ok(Sealed {
        salt: B64.encode(salt),
        nonce: B64.encode(nonce),
        data: B64.encode(data),
    })
}

/// Déchiffre avec `code` ; `Err` si le code n'est pas le bon.
pub fn open(s: &Sealed, code: &str) -> Result<String, String> {
    let dec = |x: &str| B64.decode(x).map_err(|_| "coffre abîmé".to_string());
    let (salt, nonce, data) = (dec(&s.salt)?, dec(&s.nonce)?, dec(&s.data)?);
    if nonce.len() != 12 {
        return Err("coffre abîmé".into());
    }
    let cipher = Aes256Gcm::new_from_slice(&derive(code, &salt)).map_err(|e| e.to_string())?;
    let plain = cipher
        .decrypt(Nonce::from_slice(&nonce), data.as_ref())
        .map_err(|_| "code incorrect".to_string())?;
    String::from_utf8(plain).map_err(|_| "coffre abîmé".into())
}

/// 📱 Dans la coque Tauri, aucun secret dans la page (même chiffré) : il ira dans le coffre du système, côté cœur Rust
/// (étape T2). Ailleurs (desktop, navigateur normal) : `false`.
fn dans_la_coque() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        argos_data::coque::active()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        false
    }
}

/// Le coffre `file` gardé sur cet appareil (s'il y en a un ; jamais dans la coque Tauri avant l'étape T2).
pub fn load(file: &str) -> Option<Sealed> {
    if dans_la_coque() {
        return None;
    }
    serde_json::from_str(&crate::platform::read(file)?).ok()
}

/// Garde le coffre `file` (refusé dans la coque Tauri avant l'étape T2 : aucun secret dans la page).
pub fn store(file: &str, s: &Sealed) -> Result<(), String> {
    if dans_la_coque() {
        return Err(argos_data::coque_sans_cle());
    }
    crate::platform::write(file, &serde_json::to_string(s).map_err(|e| e.to_string())?)
}

/// Oublie le secret : le coffre `file` est remplacé par un fichier vide.
pub fn forget(file: &str) -> Result<(), String> {
    crate::platform::write(file, "")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coffre() {
        let s = seal("sk-ant-test-0123456789", "1234").unwrap();
        assert!(!s.data.contains("sk-ant"), "jamais en clair");
        assert_eq!(open(&s, "1234").unwrap(), "sk-ant-test-0123456789");
        assert_eq!(open(&s, "4321").unwrap_err(), "code incorrect");
        // deux coffres de la même clé ne se ressemblent pas (sel et nonce au hasard)
        assert_ne!(seal("sk-ant-test-0123456789", "1234").unwrap().data, s.data);
    }

    #[test]
    fn cle_tiree_du_code_stable() {
        // PBKDF2-HMAC-SHA256, 200 000 tours (valeur de référence calculée hors d'Argos) : les coffres déjà gardés
        // s'ouvrent toujours après une mise à jour de pbkdf2 / sha2
        let k: String = derive("1234", b"argos-sel-test!!").iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(k, "96bae7919e590937676fb0d8db24eadc51590026eca2bb1c722e43c97de9c534");
    }
}
