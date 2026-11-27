//! 🔐 **Face ID / empreinte** (passkey, navigateur et cell seulement) : comme le site, la passkey (gardée par iCloud,
//! Google ou le cell) donne un secret PRF ; ce secret chiffre (AES-256-GCM) les clés du compte. Le paquet chiffré
//! (`passkey.json`) reste sur l'appareil ; Face ID le rouvre, sans les 24 mots ni le code. Aucun serveur.
//! Sur desktop, pas de passkey du système : « bientôt » (proposition : un code QR scanné avec le cell).

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::{engine::general_purpose::STANDARD as B64, Engine};

/// Paquet chiffré gardé sur l'appareil.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub const FILE: &str = "passkey.json";

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Packet {
    pub v: u32,
    /// Identifiant de la passkey (base64).
    pub cred: String,
    pub iv: String,
    pub ct: String,
}

/// Chiffre `payload` avec le secret PRF de la passkey `cred`.
pub fn seal(cred: &[u8], prf: &[u8], payload: &str) -> Result<Packet, String> {
    let mut iv = [0u8; 12];
    getrandom::fill(&mut iv).map_err(|e| e.to_string())?;
    let cipher = Aes256Gcm::new_from_slice(prf).map_err(|_| "secret de la passkey invalide".to_string())?;
    let ct = cipher
        .encrypt(Nonce::from_slice(&iv), payload.as_bytes())
        .map_err(|_| "chiffrement impossible".to_string())?;
    Ok(Packet {
        v: 1,
        cred: B64.encode(cred),
        iv: B64.encode(iv),
        ct: B64.encode(ct),
    })
}

/// Rouvre le paquet avec le secret PRF ; la passkey doit être celle du paquet.
pub fn open(p: &Packet, cred: &[u8], prf: &[u8]) -> Result<String, String> {
    if p.cred != B64.encode(cred) {
        return Err("ce n'est pas la passkey de ce compte sur cet appareil".into());
    }
    let dec = |x: &str| B64.decode(x).map_err(|_| "paquet abîmé".to_string());
    let cipher = Aes256Gcm::new_from_slice(prf).map_err(|_| "secret de la passkey invalide".to_string())?;
    let out = cipher
        .decrypt(Nonce::from_slice(&dec(&p.iv)?), dec(&p.ct)?.as_ref())
        .map_err(|_| "Face ID n'ouvre pas ce paquet".to_string())?;
    String::from_utf8(out).map_err(|_| "paquet abîmé".to_string())
}

/// Le paquet gardé sur cet appareil (s'il y en a un).
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub fn load() -> Option<Packet> {
    serde_json::from_str(&crate::platform::read(FILE)?).ok()
}

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub fn store(p: &Packet) -> Result<(), String> {
    crate::platform::write(FILE, &serde_json::to_string(p).map_err(|e| e.to_string())?)
}

#[cfg(target_arch = "wasm32")]
mod web {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen(inline_js = r#"
const SALT = () => new TextEncoder().encode("argos-passkey-v1");
const ok = () => !!(window.PublicKeyCredential && navigator.credentials && window.isSecureContext);
export async function argos_pk_get(credId) {
  if (!ok()) throw new Error("passkeys pas disponibles dans ce navigateur");
  const opt = { publicKey: { challenge: crypto.getRandomValues(new Uint8Array(32)), userVerification: "required", timeout: 60000, extensions: { prf: { eval: { first: SALT() } } } } };
  if (credId && credId.length) opt.publicKey.allowCredentials = [{ type: "public-key", id: credId }];
  const cr = await navigator.credentials.get(opt);
  const r = cr.getClientExtensionResults ? cr.getClientExtensionResults() : {};
  const out = r.prf && r.prf.results && r.prf.results.first;
  if (!out) throw new Error("ton appareil ou ton navigateur ne supporte pas encore cette fonction (PRF) : garde tes 24 mots");
  return [new Uint8Array(cr.rawId), new Uint8Array(out)];
}
export async function argos_pk_create(userId, name) {
  if (!ok()) throw new Error("passkeys pas disponibles dans ce navigateur");
  const cr = await navigator.credentials.create({ publicKey: { challenge: crypto.getRandomValues(new Uint8Array(32)), rp: { name: "Argos" },
    user: { id: userId, name, displayName: "Argos " + name }, pubKeyCredParams: [{ type: "public-key", alg: -7 }, { type: "public-key", alg: -257 }],
    authenticatorSelection: { residentKey: "required", userVerification: "required" }, timeout: 60000, extensions: { prf: {} } } });
  const ext = cr.getClientExtensionResults ? cr.getClientExtensionResults() : {};
  if (ext.prf && ext.prf.enabled === false) throw new Error("ton appareil a créé la passkey mais ne supporte pas PRF : garde tes 24 mots");
  return argos_pk_get(new Uint8Array(cr.rawId));
}
"#)]
    extern "C" {
        #[wasm_bindgen(catch)]
        async fn argos_pk_get(cred: &[u8]) -> Result<JsValue, JsValue>;
        #[wasm_bindgen(catch)]
        async fn argos_pk_create(user: &[u8], name: &str) -> Result<JsValue, JsValue>;
    }

    fn pair(v: JsValue) -> (Vec<u8>, Vec<u8>) {
        let a = js_sys::Array::from(&v);
        (js_sys::Uint8Array::new(&a.get(0)).to_vec(), js_sys::Uint8Array::new(&a.get(1)).to_vec())
    }

    fn msg(e: JsValue) -> String {
        js_sys::Reflect::get(&e, &"message".into())
            .ok()
            .and_then(|m| m.as_string())
            .unwrap_or_else(|| "Face ID annulé".into())
    }

    /// Crée la passkey du compte (`user` = 32 octets, sa clé publique Nostr) : (identifiant, secret PRF).
    pub async fn create(user: &[u8], name: &str) -> Result<(Vec<u8>, Vec<u8>), String> {
        argos_pk_create(user, name).await.map(pair).map_err(msg)
    }

    /// Face ID / empreinte avec une passkey d'Argos (`cred` vide : celle que la personne choisit).
    pub async fn get(cred: &[u8]) -> Result<(Vec<u8>, Vec<u8>), String> {
        argos_pk_get(cred).await.map(pair).map_err(msg)
    }
}

#[cfg(target_arch = "wasm32")]
pub use web::{create, get};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paquet() {
        let (cred, prf) = (b"cred-1".to_vec(), [7u8; 32]);
        let p = seal(&cred, &prf, "secret du compte").unwrap();
        assert_eq!(open(&p, &cred, &prf).unwrap(), "secret du compte");
        assert!(open(&p, b"autre", &prf).is_err(), "autre passkey");
        assert!(open(&p, &cred, &[8u8; 32]).is_err(), "autre secret");
    }
}
