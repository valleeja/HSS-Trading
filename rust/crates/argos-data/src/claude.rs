//! ✨ Assistant Argos : un appel à l'API de Claude (Messages) avec **ta** clé, qui ne quitte jamais l'appareil
//! autrement que vers `api.anthropic.com`. Recherche web intégrée (outil `web_search` d'Anthropic) pour les infos
//! récentes ; les pages web sont des données, jamais des instructions.
//!
//! Desktop : `reqwest` (2 min de délai, les réponses avec recherche web sont longues) ; navigateur : `fetch`.

use serde_json::{json, Value};

/// Modèle utilisé (le même que l'assistant d'Argos v1).
pub const MODEL: &str = "claude-sonnet-5-5";
#[cfg_attr(not(any(feature = "native", feature = "web")), allow(dead_code))]
pub(crate) const URL: &str = "https://api.anthropic.com/v1/messages";

/// Un message de la conversation.
#[derive(Debug, Clone, PartialEq)]
pub struct Msg {
    /// « user » ou « assistant ».
    pub role: &'static str,
    pub text: String,
}

/// Réponse : le texte, et les sources trouvées par la recherche web (titre, lien).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Answer {
    pub text: String,
    pub sources: Vec<(String, String)>,
}

/// Corps de la requête.
pub fn body(system: &str, msgs: &[Value]) -> String {
    json!({
        "model": MODEL,
        "max_tokens": 3000,
        "system": system,
        "messages": msgs,
        "tools": [{ "type": "web_search_20250305", "name": "web_search", "max_uses": 5 }],
    })
    .to_string()
}

/// Lit une réponse de l'API : (texte, sources, contenu brut, faut-il continuer (`pause_turn`)).
pub fn parse(resp: &str) -> argos_core::Result<(String, Vec<(String, String)>, Value, bool)> {
    let v: Value = serde_json::from_str(resp).map_err(|e| format!("réponse illisible : {e}"))?;
    if let Some(e) = v.get("error") {
        return Err(e["message"].as_str().unwrap_or("erreur de Claude").into());
    }
    let blocks = v["content"].as_array().cloned().unwrap_or_default();
    let mut text = String::new();
    let mut sources = vec![];
    for b in &blocks {
        match b["type"].as_str() {
            Some("text") => text.push_str(b["text"].as_str().unwrap_or("")),
            Some("web_search_tool_result") => {
                for r in b["content"].as_array().into_iter().flatten() {
                    if let (Some(u), t) = (r["url"].as_str(), r["title"].as_str()) {
                        if !sources.iter().any(|(_, x)| x == u) {
                            sources.push((t.unwrap_or(u).to_string(), u.to_string()));
                        }
                    }
                }
            }
            _ => {}
        }
    }
    let more = v["stop_reason"].as_str() == Some("pause_turn");
    Ok((text, sources, Value::Array(blocks), more))
}

/// Message d'erreur clair selon le statut HTTP.
#[cfg_attr(not(any(feature = "native", feature = "web")), allow(dead_code))]
fn status_msg(status: u16, body: &str) -> String {
    let detail = serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|v| v["error"]["message"].as_str().map(str::to_string))
        .unwrap_or_default();
    match status {
        401 => "🔑 Clé refusée par Claude : vérifie-la (bouton 🔑 de l'assistant)".into(),
        429 => "⏳ Limite de Claude atteinte : réessaie dans une minute".into(),
        529 | 503 => "😴 Claude est surchargé : réessaie dans un moment".into(),
        _ => format!("Claude : erreur {status} {detail}"),
    }
}

/// Demande à Claude (avec la recherche web) ; continue tout seul si la réponse est en pause (`pause_turn`).
#[cfg(any(feature = "native", feature = "web"))]
pub async fn ask(key: &str, system: &str, history: &[Msg]) -> argos_core::Result<Answer> {
    let mut msgs: Vec<Value> = history.iter().map(|m| json!({ "role": m.role, "content": m.text })).collect();
    let mut out = Answer::default();
    for _ in 0..4 {
        let (status, resp) = post(key, &body(system, &msgs)).await?;
        if !(200..300).contains(&status) {
            return Err(status_msg(status, &resp).into());
        }
        let (text, sources, blocks, more) = parse(&resp)?;
        if !text.is_empty() {
            if !out.text.is_empty() {
                out.text.push_str("\n\n");
            }
            out.text.push_str(&text);
        }
        for s in sources {
            if !out.sources.iter().any(|x| x.1 == s.1) {
                out.sources.push(s);
            }
        }
        if !more {
            break;
        }
        msgs.push(json!({ "role": "assistant", "content": blocks }));
    }
    Ok(out)
}

/// POST vers l'API : (statut, texte).
#[cfg(feature = "native")]
async fn post(key: &str, body: &str) -> Result<(u16, String), String> {
    crate::net::verifier(URL).map_err(|e| e.to_string())?;
    let c = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())?;
    let r = c
        .post(URL)
        .header("content-type", "application/json")
        .header("x-api-key", key)
        .header("anthropic-version", "2023-06-01")
        .body(body.to_string())
        .send()
        .await
        .map_err(|e| format!("réseau : {e}"))?;
    let status = r.status().as_u16();
    Ok((status, r.text().await.map_err(|e| e.to_string())?))
}

#[cfg(all(feature = "web", not(feature = "native")))]
async fn post(key: &str, body: &str) -> Result<(u16, String), String> {
    crate::net::verifier(URL).map_err(|e| e.to_string())?;
    // 📱 coque Tauri : aucune clé dans la page (étape T2 : coffre du système, appels par le cœur Rust)
    if crate::coque::active() {
        return Err(crate::coque_sans_cle());
    }
    use wasm_bindgen::{JsCast, JsValue};
    let err = |e: JsValue| e.as_string().unwrap_or_else(|| "réseau".into());
    let init = web_sys::RequestInit::new();
    init.set_method("POST");
    init.set_body(&JsValue::from_str(body));
    let h = web_sys::Headers::new().map_err(err)?;
    for (k, v) in [
        ("content-type", "application/json"),
        ("x-api-key", key),
        ("anthropic-version", "2023-06-01"),
        ("anthropic-dangerous-direct-browser-access", "true"),
    ] {
        h.set(k, v).map_err(err)?;
    }
    init.set_headers(&h);
    let w = web_sys::window().ok_or("pas de fenêtre")?;
    let resp: web_sys::Response = wasm_bindgen_futures::JsFuture::from(w.fetch_with_str_and_init(URL, &init))
        .await
        .map_err(err)?
        .dyn_into()
        .map_err(err)?;
    let text = wasm_bindgen_futures::JsFuture::from(resp.text().map_err(err)?)
        .await
        .map_err(err)?
        .as_string()
        .unwrap_or_default();
    Ok((resp.status(), text))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requete_et_reponse() {
        let b: Value = serde_json::from_str(&body("Tu es l'assistant d'Argos.", &[json!({"role": "user", "content": "Analyse 4H"})])).unwrap();
        assert_eq!(b["model"], MODEL);
        assert_eq!(b["messages"][0]["content"], "Analyse 4H");
        assert_eq!(b["tools"][0]["name"], "web_search");
        let resp = r#"{"content":[{"type":"server_tool_use","id":"s1","name":"web_search","input":{"query":"btc"}},
            {"type":"web_search_tool_result","tool_use_id":"s1","content":[{"type":"web_search_result","url":"https://x.com/a","title":"A"}]},
            {"type":"text","text":"BTC tient le support."}],"stop_reason":"end_turn"}"#;
        let (t, s, blocks, more) = parse(resp).unwrap();
        assert_eq!(t, "BTC tient le support.");
        assert_eq!(s, vec![("A".to_string(), "https://x.com/a".to_string())]);
        assert_eq!(blocks.as_array().unwrap().len(), 3);
        assert!(!more);
        assert!(parse(r#"{"stop_reason":"pause_turn","content":[]}"#).unwrap().3);
        assert!(parse(r#"{"type":"error","error":{"type":"x","message":"mauvaise clé"}}"#)
            .unwrap_err()
            .to_string()
            .contains("mauvaise clé"));
        assert!(status_msg(401, "").contains("Clé refusée"));
    }
}
