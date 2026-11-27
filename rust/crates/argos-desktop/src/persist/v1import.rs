//! 📥 Reprise des données d'Argos v1 : watchlists, dessins (par symbole), scripts Argos, journal, compte démo.
//!
//! D'où elles viennent :
//! - **navigateur** (même site que la v1) : lues directement dans le stockage de la page (`argos-chart`,
//!   `argos-draw-*`, `argos-journal`, `argos-paper`) ;
//! - **desktop** : le fichier `argos-v1.json` de tes Téléchargements, fait par le bouton « 📦 Tout exporter pour Argos
//!   Rust » de la v1.
//!
//! Jamais de clés, de wallet ni de clé Nostr : seulement les clés de la liste `KEYS`. Rien n'est écrasé : ce qui existe
//! déjà dans Argos Rust est gardé, la reprise ajoute à côté (et une 2e reprise n'ajoute pas de doublons).

use argos_core::journal::JTrade;
use argos_core::portfolio::Portfolio;
use argos_data::Exchange;
use argos_render::drawings::Drawing;
use serde_json::{json, Value};
use std::collections::HashMap;

/// Clés de la v1 reprises (les autres, dont les clés API et le wallet, sont ignorées).
pub const KEYS: [&str; 3] = ["argos-chart", "argos-journal", "argos-paper"];
/// Préfixe des dessins par symbole.
pub const DRAW: &str = "argos-draw-";
/// Nom du fichier d'export de la v1.
pub const FILE: &str = "argos-v1.json";

/// Ce qu'il y a à reprendre.
#[derive(Debug, Default)]
pub struct Plan {
    /// Listes : (nom, [(section, id Argos Rust)]).
    pub lists: Vec<(String, Vec<(String, String)>)>,
    /// Dessins : (fichier de sauvegarde du symbole, dessins).
    pub drawings: Vec<(String, Vec<Drawing>)>,
    /// Scripts : (nom, code, sur la chart).
    pub scripts: Vec<(String, String, bool)>,
    pub journal: Vec<JTrade>,
    pub demo: Option<Portfolio>,
    /// Ce qui n'a pas pu être repris (et pourquoi).
    pub skipped: Vec<String>,
}

/// Les données de la v1 : `{clé: valeur JSON}` (seulement les clés permises). Navigateur : le stockage de la page.
#[cfg_attr(not(any(target_arch = "wasm32", test)), allow(dead_code))]
pub fn bundle(pairs: impl IntoIterator<Item = (String, String)>) -> HashMap<String, Value> {
    pairs
        .into_iter()
        .filter(|(k, _)| KEYS.contains(&k.as_str()) || k.starts_with(DRAW))
        .filter_map(|(k, v)| Some((k, serde_json::from_str(&v).ok()?)))
        .collect()
}

/// Lit le fichier d'export de la v1 (`{"format":"argos-v1-export","data":{…}}`).
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub fn from_export(raw: &str) -> Option<HashMap<String, Value>> {
    let v: Value = serde_json::from_str(raw).ok()?;
    if v["format"] != "argos-v1-export" {
        return None;
    }
    let data = v["data"].as_object()?;
    Some(
        data.iter()
            .filter(|(k, _)| KEYS.contains(&k.as_str()) || k.starts_with(DRAW))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect(),
    )
}

/// Symbole de la v1 ou ticker TradingView → (id Argos Rust « exchange:symbole », nom de l'exchange, base).
/// Accepte « BINANCE:BTCUSDT », « binance:btcusdt.p » (perpétuel), « BINANCE:XRPBTC » (paire calculée).
/// `None` : pas dans Argos Rust (OANDA, COINBASE…).
pub fn coin(v1: &str) -> Option<(String, &'static str, String)> {
    use argos_data::tv::{self, Tv};
    let tv_prefix = match v1.split_once(':') {
        Some((e, _)) => {
            e.chars().all(|c| c.is_ascii_uppercase()) || e.eq_ignore_ascii_case("hyperliquid") || Exchange::ALL.iter().any(|x| x.id().eq_ignore_ascii_case(e))
        }
        None => false,
    };
    if !tv_prefix {
        return Some((format!("hl:{v1}"), "Hyperliquid", v1.to_string())); // Hyperliquid : « BTC », « xyz:GOLD »
    }
    let Tv::Ticker(t) = tv::parse(v1) else { return None };
    // la v1 n'avait que des perpétuels : « OKX:SOLUSDT » reste le perp d'OKX
    let ex = t.exchange?.parent();
    let base = t.base.clone();
    if t.is_btc_pair() {
        return Some((tv::ratio_id(ex, &base, "BTC"), ex.name(), base));
    }
    let quote = t.quote.clone().unwrap_or_else(|| "USDT".into());
    let id = match ex {
        Exchange::Hyperliquid => format!("hl:{base}"),
        Exchange::Okx => format!("okx:{base}-{quote}-SWAP"),
        Exchange::Blofin => format!("blofin:{base}-{quote}"),
        Exchange::Bitstamp => format!("bitstamp:{}{}", base.to_lowercase(), quote.to_lowercase()),
        Exchange::Binance | Exchange::Bybit | Exchange::Weex | Exchange::Bitunix | Exchange::Bitget | Exchange::Aster => {
            format!("{}:{base}{quote}", ex.id())
        }
        Exchange::Gate | Exchange::Mexc => format!("{}:{base}_{quote}", ex.id()),
        Exchange::Kucoin => format!("kucoin:{base}-{quote}"),
        Exchange::Coinbase => format!("coinbase:{base}-USD"),
        Exchange::Kraken => format!("kraken:{base}/USD"),
        Exchange::OkxSpot => format!("okx-spot:{base}-{quote}"),
        Exchange::BinanceSpot | Exchange::BybitSpot | Exchange::BitgetSpot => format!("{}:{base}{quote}", ex.id()),
        // (v1 never had Bitfinex: its perpetual, as Bitfinex writes it)
        Exchange::Bitfinex | Exchange::BitfinexSpot => {
            format!("bitfinex:{}", argos_data::exchanges::bitfinex::symbole(&base, true).trim_start_matches('t'))
        }
    };
    Some((id, ex.name(), base))
}

/// Un dessin de la v1 (`{type, pts:[{t,p}], st:{color, width, dash, locked, hidden, text, tsize, tcolor, tfs}}`).
fn drawing(d: &Value) -> Option<Drawing> {
    let st = &d["st"];
    let pts: Vec<Value> = d["pts"]
        .as_array()?
        .iter()
        .map(|q| json!({ "t": q["t"].as_f64().unwrap_or(0.0) as i64, "p": q["p"].as_f64().unwrap_or(0.0) }))
        .collect();
    let mut style = json!({});
    let set = |s: &mut Value, k: &str, v: Value| s[k] = v;
    if let Some(c) = st["color"].as_str() {
        set(&mut style, "color", json!(c));
    }
    if let Some(w) = st["width"].as_f64() {
        set(&mut style, "width", json!(w));
    }
    match st["dash"].as_str() {
        Some("dashed") => set(&mut style, "dash", json!(true)),
        Some("dotted") => set(&mut style, "dotted", json!(true)),
        _ => {}
    }
    for k in ["locked", "hidden"] {
        if let Some(b) = st[k].as_bool() {
            set(&mut style, k, json!(b));
        }
    }
    if let Some(t) = st["text"].as_str() {
        set(&mut style, "text", json!(t));
    }
    if let Some(t) = st["tsize"].as_f64() {
        set(&mut style, "tsize", json!(t));
    }
    if let Some(t) = st["tcolor"].as_str() {
        set(&mut style, "tcolor", json!(t));
    }
    if let Some(tfs) = st["tfs"].as_array() {
        let secs: Vec<i64> = tfs.iter().filter_map(|t| argos_core::Interval::parse(t.as_str()?)).map(|i| i.secs()).collect();
        set(&mut style, "intervals", json!(secs));
    }
    // le reste du style prend ses valeurs par défaut
    let mut full = serde_json::to_value(argos_render::drawings::Style::default()).ok()?;
    for (k, v) in style.as_object()? {
        full[k] = v.clone();
    }
    serde_json::from_value(json!({ "tool": d["type"].as_str()?, "pts": pts, "style": full })).ok()
}

/// Ce qu'il y a à reprendre dans les données de la v1.
pub fn plan(data: &HashMap<String, Value>) -> Plan {
    let mut p = Plan::default();
    let state = data.get("argos-chart").cloned().unwrap_or(Value::Null);
    // watchlists : [{name, items:[{sec}, "BTC", "BINANCE:BTCUSDT"…]}]
    for l in state["lists"].as_array().into_iter().flatten() {
        let name = l["name"].as_str().unwrap_or("Liste v1").to_string();
        let mut sec = String::new();
        let mut items = vec![];
        for it in l["items"].as_array().into_iter().flatten() {
            if let Some(s) = it["sec"].as_str() {
                sec = s.to_string();
            } else if let Some(c) = it.as_str() {
                match coin(c) {
                    Some((id, _, _)) => items.push((sec.clone(), id)),
                    None => p.skipped.push(format!("{c} (liste « {name} ») : OANDA pas encore dans Argos Rust")),
                }
            }
        }
        if !items.is_empty() {
            p.lists.push((format!("{name} (v1)"), items));
        }
    }
    // scripts Argos (pas les indicateurs natifs du créateur)
    for c in state["cinds"].as_array().into_iter().flatten() {
        if let Some(src) = c["script"].as_str().filter(|s| !s.trim().is_empty()) {
            p.scripts.push((
                c["name"].as_str().unwrap_or("Script v1").to_string(),
                src.to_string(),
                c["on"].as_bool().unwrap_or(false),
            ));
        }
    }
    // dessins par symbole
    let mut keys: Vec<&String> = data.keys().filter(|k| k.starts_with(DRAW)).collect();
    keys.sort();
    for k in keys {
        let c = &k[DRAW.len()..];
        let list: Vec<Drawing> = data[k].as_array().into_iter().flatten().filter_map(drawing).collect();
        if list.is_empty() {
            continue;
        }
        match coin(c) {
            Some((_, ex, base)) => p.drawings.push((super::saved::file_name(&format!("{ex}-{base}")), list)),
            None => p.skipped.push(format!("dessins de {c} : OANDA pas encore dans Argos Rust")),
        }
    }
    // journal : déjà au format de v1
    if let Some(j) = data.get("argos-journal") {
        p.journal = serde_json::from_value(j.clone()).unwrap_or_default();
    }
    // compte démo actif
    let paper = data.get("argos-paper").cloned().unwrap_or(Value::Null);
    let accts = paper["accounts"].as_array().cloned().unwrap_or_default();
    let active = accts.iter().find(|a| a["id"] == paper["active"]).or(accts.first());
    if let Some(a) = active {
        let base = |s: &Value| s.as_str().and_then(coin).map_or_else(|| s.as_str().unwrap_or("?").to_string(), |c| c.2);
        let side = |s: &Value| if s.as_str() == Some("sell") { "Short" } else { "Long" };
        let positions: Vec<Value> = a["positions"].as_array().into_iter().flatten().enumerate().map(|(k, q)| json!({
            "id": k as u64 + 1, "base": base(&q["sym"]), "side": side(&q["side"]), "qty": q["units"].as_f64().unwrap_or(0.0),
            "entry": q["entry"].as_f64().unwrap_or(0.0), "stop": q["sl"].as_f64(), "target": q["tp"].as_f64(), "opened_at": q["openTs"].as_i64().unwrap_or(0) / 1000,
        })).collect();
        let history: Vec<Value> = a["history"].as_array().into_iter().flatten().map(|h| json!({
            "base": base(&h["sym"]), "side": side(&h["side"]), "qty": h["units"].as_f64().unwrap_or(0.0), "entry": h["entry"].as_f64().unwrap_or(0.0),
            "exit": h["exit"].as_f64().unwrap_or(0.0), "pnl": h["pnl"].as_f64().unwrap_or(0.0), "fees": 0.0,
            "opened_at": h["openTs"].as_i64().unwrap_or(0) / 1000, "closed_at": h["closeTs"].as_i64().unwrap_or(0) / 1000, "reason": h["why"].as_str().unwrap_or(""),
        })).collect();
        let n = positions.len() as u64 + 1;
        let start = a["start"].as_f64().unwrap_or(100_000.0);
        p.demo = serde_json::from_value(json!({
            "name": a["name"].as_str().unwrap_or("Démo v1"), "start": start, "balance": a["balance"].as_f64().unwrap_or(start),
            "fee_rate": 0.0, "positions": positions, "history": history, "next_id": n,
        }))
        .ok();
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v1() -> HashMap<String, Value> {
        bundle([
            ("argos-chart".to_string(), r##"{"coin":"BTC","lists":[{"name":"Hyperliquid ⚡","items":[{"sec":"Majeurs"},"BTC","xyz:GOLD",{"sec":"Altcoins"},"ZEC"]},
                {"name":"Mixte","items":["OKX:SOLUSDT","BINANCE:ETHUSDT","OANDA:XAU_USD"]}],
                "cinds":[{"name":"RSI v1","script":"indicator(\"RSI\")\nplot(rsi(close, 14))","on":true},{"name":"Natif","native":"tbo"}]}"##.to_string()),
            ("argos-draw-BTC".to_string(), r##"[{"type":"hline","pts":[{"t":1793000000,"p":80000}],"st":{"color":"#ff0000","dash":"dashed","locked":true,"tfs":["4h","1h"]}},
                {"type":"trend","pts":[{"t":1793000000,"p":79000},{"t":1793010000,"p":81000}]}]"##.to_string()),
            ("argos-draw-OANDA:XAU_USD".to_string(), r#"[{"type":"hline","pts":[{"t":1,"p":4000}]}]"#.to_string()),
            ("argos-journal".to_string(), r#"[{"id":"j1","sym":"BTC","side":"long","entry":80000,"sl":79000,"tp":82000,"pnl":150,"entryTs":1793000000000,"exitTs":1793003600000,"conf":[],"notes":""}]"#.to_string()),
            ("argos-paper".to_string(), r#"{"active":"a2","accounts":[{"id":"a1","name":"Vieux"},{"id":"a2","name":"PropFirm 100K","start":100000,"balance":101500,
                "positions":[{"sym":"BTC","side":"buy","units":0.5,"entry":80000,"sl":79000,"openTs":1793000000000}],
                "history":[{"sym":"BINANCE:ETHUSDT","side":"sell","units":2,"entry":2400,"exit":2300,"pnl":200,"openTs":1792900000000,"closeTs":1792990000000,"why":"objectif"}]}]}"#.to_string()),
            // jamais repris : clés, wallet, Nostr
            ("argos-claude-key".to_string(), r#""sk-ant-xxx""#.to_string()),
            ("argos-nostr-v1".to_string(), r#"{"sk":"secret"}"#.to_string()),
        ])
    }

    #[test]
    fn symboles() {
        assert_eq!(coin("BTC").unwrap().0, "hl:BTC");
        assert_eq!(coin("xyz:GOLD").unwrap().0, "hl:xyz:GOLD");
        assert_eq!(coin("OKX:SOLUSDT").unwrap().0, "okx:SOL-USDT-SWAP");
        assert_eq!(coin("BLOFIN:BTCUSDT").unwrap().0, "blofin:BTC-USDT");
        assert_eq!(coin("BINANCE:ETHUSDT").unwrap(), ("binance:ETHUSDT".to_string(), "Binance", "ETH".to_string()));
        assert!(coin("OANDA:XAU_USD").is_none());
        // format TradingView (guide des tickers) : « .P », minuscules, paires contre BTC, préfixes pas gérés
        assert_eq!(
            coin("BINANCE:BTCUSDT.P").unwrap(),
            ("binance:BTCUSDT".to_string(), "Binance", "BTC".to_string())
        );
        assert_eq!(coin("bybit:ethusdt.p").unwrap().0, "bybit:ETHUSDT");
        assert_eq!(coin("OKX:BTCUSDT.P").unwrap().0, "okx:BTC-USDT-SWAP");
        assert_eq!(coin("BITSTAMP:BTCUSD").unwrap().0, "bitstamp:btcusd");
        assert_eq!(coin("HYPERLIQUID:BTCUSDC.P").unwrap().0, "hl:BTC");
        assert_eq!(coin("BINANCE:XRPBTC").unwrap().0, "ratio:binance:XRP:BTC");
        assert_eq!(coin("COINBASE:BTCUSD").unwrap().0, "coinbase:BTC-USD", "Coinbase branché (spot en dollars)");
        assert!(coin("CME:BTC1!").is_none());
    }

    #[test]
    fn reprise() {
        let data = v1();
        assert!(
            !data.contains_key("argos-claude-key") && !data.contains_key("argos-nostr-v1"),
            "jamais les clés"
        );
        let p = plan(&data);
        assert_eq!(p.lists.len(), 2);
        assert_eq!(p.lists[0].0, "Hyperliquid ⚡ (v1)");
        assert_eq!(
            p.lists[0].1,
            vec![
                ("Majeurs".into(), "hl:BTC".into()),
                ("Majeurs".into(), "hl:xyz:GOLD".into()),
                ("Altcoins".into(), "hl:ZEC".into())
            ]
        );
        assert_eq!(p.lists[1].1.len(), 2, "OANDA laissé de côté");
        assert!(p.skipped.iter().any(|s| s.contains("OANDA")));
        assert_eq!(
            p.scripts,
            vec![("RSI v1".to_string(), "indicator(\"RSI\")\nplot(rsi(close, 14))".to_string(), true)]
        );
        assert_eq!(p.drawings.len(), 1);
        let (file, d) = &p.drawings[0];
        assert_eq!(file, "hyperliquid-btc.json");
        assert_eq!(
            (d[0].tool.as_str(), d[0].style.color.as_str(), d[0].style.dash, d[0].style.locked),
            ("hline", "#ff0000", true, true)
        );
        assert_eq!(d[0].style.intervals, vec![14400, 3600]);
        assert_eq!(d[1].pts.len(), 2);
        assert_eq!(p.journal.len(), 1);
        let demo = p.demo.unwrap();
        assert_eq!(
            (demo.name.as_str(), demo.balance, demo.positions.len(), demo.history.len()),
            ("PropFirm 100K", argos_core::Money::from_f64(101_500.0), 1, 1)
        );
        assert_eq!(demo.positions[0].opened_at, 1_793_000_000);
        assert_eq!(demo.history[0].base, "ETH");
        // le fichier d'export de la v1
        let raw = json!({ "format": "argos-v1-export", "data": { "argos-chart": data["argos-chart"], "argos-wallet": {"x": 1} } }).to_string();
        let back = from_export(&raw).unwrap();
        assert!(back.contains_key("argos-chart") && !back.contains_key("argos-wallet"));
        assert!(from_export("{}").is_none());
    }
}
