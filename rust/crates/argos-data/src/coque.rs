//! 📱 Version navigateur (WASM) **dans la coque Tauri** (étape T1) : la page n'a aucun réseau (CSP de la coque). Tout
//! ce qui sortirait vers Internet passe par le **cœur Rust** de la coque, par l'IPC de Tauri :
//! - les requêtes HTTP (`reseau`) : le cœur vérifie la liste blanche (`net.rs`) et les fait avec le client natif ;
//! - le direct (`direct_ouvrir` / `direct_fermer`) : le cœur ouvre le WebSocket et renvoie chaque message par un canal.
//!
//! Hors de la coque (navigateur normal), rien ne change : `active()` est faux et la page fait comme avant.

use wasm_bindgen::{JsCast, JsValue};

fn internes() -> Option<JsValue> {
    let w = web_sys::window()?;
    js_sys::Reflect::get(&w, &JsValue::from_str("__TAURI_INTERNALS__"))
        .ok()
        .filter(|v| v.is_object())
}

/// La page tourne-t-elle dans la coque Tauri ?
pub fn active() -> bool {
    internes().is_some()
}

/// Un appel au cœur Rust de la coque (`__TAURI_INTERNALS__.invoke`) : la réponse, ou le texte de l'erreur.
pub async fn invoke(cmd: &str, args: &JsValue) -> Result<JsValue, String> {
    let i = internes().ok_or("pas dans la coque")?;
    let f: js_sys::Function = js_sys::Reflect::get(&i, &JsValue::from_str("invoke"))
        .map_err(|_| "IPC absente")?
        .dyn_into()
        .map_err(|_| "IPC absente")?;
    let p: js_sys::Promise = f
        .call2(&i, &JsValue::from_str(cmd), args)
        .map_err(|e| e.as_string().unwrap_or_else(|| "IPC".into()))?
        .dyn_into()
        .map_err(|_| "IPC : pas de promesse")?;
    wasm_bindgen_futures::JsFuture::from(p)
        .await
        .map_err(|e| e.as_string().unwrap_or_else(|| "erreur du cœur".into()))
}

/// Un objet JS `{ clé: valeur }` à partir de textes.
pub fn objet(champs: &[(&str, JsValue)]) -> JsValue {
    let o = js_sys::Object::new();
    for (k, v) in champs {
        let _ = js_sys::Reflect::set(&o, &JsValue::from_str(k), v);
    }
    o.into()
}

/// Un canal de Tauri (`new window.__TAURI__.core.Channel()`) qui appelle `on` à chaque message.
pub fn canal(on: impl FnMut(JsValue) + 'static) -> Option<JsValue> {
    let w = web_sys::window()?;
    let tauri = js_sys::Reflect::get(&w, &JsValue::from_str("__TAURI__")).ok()?;
    let core = js_sys::Reflect::get(&tauri, &JsValue::from_str("core")).ok()?;
    let ctor: js_sys::Function = js_sys::Reflect::get(&core, &JsValue::from_str("Channel")).ok()?.dyn_into().ok()?;
    let ch = js_sys::Reflect::construct(&ctor, &js_sys::Array::new()).ok()?;
    let cb = wasm_bindgen::closure::Closure::<dyn FnMut(JsValue)>::new(on);
    js_sys::Reflect::set(&ch, &JsValue::from_str("onmessage"), cb.as_ref()).ok()?;
    cb.forget();
    Some(ch)
}
