//! 🏦 Symboles OANDA populaires d'Argos v1 (forex, métaux, énergie, indices, taux), la liste de départ « OANDA 🥇 »
//! et la **source OANDA** (bougies v20 avec le jeton de l'utilisateur, compte démo ou réel).
//! Sans jeton, Argos montre leurs équivalents Hyperliquid HIP-3 (`xyz:GOLD`…), proches d'OANDA mais pas identiques,
//! comme v1. Le jeton est entré par l'utilisateur lui-même ; il part seulement chez OANDA (desktop) ou dans l'en-tête
//! du relais `/api/oanda` du site (navigateur), qui ne le garde ni ne l'écrit.

/// (instrument OANDA, nom, catégorie, taille d'un lot standard, équivalent Hyperliquid HIP-3).
pub const OANDA: [(&str, &str, &str, f64, Option<&str>); 28] = [
    ("EUR_USD", "Euro / Dollar US", "forex", 100_000.0, Some("xyz:EUR")),
    ("GBP_USD", "Livre / Dollar US", "forex", 100_000.0, Some("xyz:GBP")),
    ("USD_JPY", "Dollar US / Yen", "forex", 100_000.0, Some("xyz:JPY")),
    ("USD_CAD", "Dollar US / Dollar canadien", "forex", 100_000.0, None),
    ("AUD_USD", "Dollar australien / Dollar US", "forex", 100_000.0, None),
    ("NZD_USD", "Dollar NZ / Dollar US", "forex", 100_000.0, None),
    ("USD_CHF", "Dollar US / Franc suisse", "forex", 100_000.0, None),
    ("EUR_GBP", "Euro / Livre", "forex", 100_000.0, None),
    ("EUR_JPY", "Euro / Yen", "forex", 100_000.0, None),
    ("GBP_JPY", "Livre / Yen", "forex", 100_000.0, None),
    ("AUD_JPY", "Dollar australien / Yen", "forex", 100_000.0, None),
    ("EUR_CAD", "Euro / Dollar canadien", "forex", 100_000.0, None),
    ("CAD_JPY", "Dollar canadien / Yen", "forex", 100_000.0, None),
    ("XAU_USD", "Or / Dollar US", "commodity", 100.0, Some("xyz:GOLD")),
    ("XAG_USD", "Argent / Dollar US", "commodity", 5000.0, Some("xyz:SILVER")),
    ("XCU_USD", "Cuivre", "commodity", 1.0, Some("xyz:COPPER")),
    ("WTICO_USD", "Pétrole WTI", "commodity", 1.0, Some("xyz:CL")),
    ("BCO_USD", "Pétrole Brent", "commodity", 1.0, Some("xyz:BRENTOIL")),
    ("NATGAS_USD", "Gaz naturel", "commodity", 1.0, Some("xyz:NATGAS")),
    ("XPT_USD", "Platine", "commodity", 1.0, Some("xyz:PLATINUM")),
    ("SPX500_USD", "S&P 500", "index", 1.0, Some("xyz:SP500")),
    ("NAS100_USD", "Nasdaq 100", "index", 1.0, Some("mkts:USTECH")),
    ("US30_USD", "Dow Jones 30", "index", 1.0, None),
    ("US2000_USD", "Russell 2000", "index", 1.0, Some("mkts:SMALL2000")),
    ("DE30_EUR", "DAX 40 (Allemagne)", "index", 1.0, None),
    ("UK100_GBP", "FTSE 100", "index", 1.0, None),
    ("JP225_USD", "Nikkei 225", "index", 1.0, Some("xyz:JP225")),
    ("USB10Y_USD", "Obligation US 10 ans", "economic", 1.0, None),
];

/// Nom de la liste de départ.
pub const LIST_NAME: &str = "OANDA";

/// Liste de départ « OANDA » de v1, dans le même ordre : (section, instruments).
pub const SEED: [(&str, &[&str]); 6] = [
    ("Métaux", &["XAU_USD", "XAG_USD", "XPT_USD", "XCU_USD"]),
    ("Énergie", &["WTICO_USD", "BCO_USD", "NATGAS_USD"]),
    (
        "Indices",
        &["SPX500_USD", "NAS100_USD", "US30_USD", "US2000_USD", "DE30_EUR", "UK100_GBP", "JP225_USD"],
    ),
    ("Forex majeurs", &["EUR_USD", "GBP_USD", "USD_JPY", "USD_CAD", "AUD_USD", "USD_CHF", "NZD_USD"]),
    ("Forex croisés", &["EUR_GBP", "EUR_JPY", "GBP_JPY", "AUD_JPY", "EUR_CAD", "CAD_JPY"]),
    ("Taux", &["USB10Y_USD"]),
];

/// Ligne de watchlist d'un instrument : son équivalent Hyperliquid (`hl:xyz:GOLD`), ou `oanda:XAU_USD` s'il n'en a pas.
pub fn item(instrument: &str) -> String {
    match OANDA.iter().find(|o| o.0 == instrument).and_then(|o| o.4) {
        Some(px) => format!("hl:{px}"),
        None => format!("oanda:{instrument}"),
    }
}

/// Nom OANDA à montrer pour une ligne (« XAUUSD » pour `hl:xyz:GOLD` ou `oanda:XAU_USD`) ; `None` : pas un symbole OANDA.
pub fn label(item: &str) -> Option<String> {
    let (ex, sym) = item.split_once(':')?;
    let ins = match ex {
        "oanda" => OANDA.iter().find(|o| o.0 == sym),
        "hl" => OANDA.iter().find(|o| o.4 == Some(sym)),
        _ => None,
    }?;
    Some(ins.0.replace('_', ""))
}

/// Pastille émoji comme v1 (🥇 or, 🥈 argent, 🛢 pétrole, 📊 indices, 💱 forex…) ; `None` : pas un symbole OANDA.
pub fn emoji(item: &str) -> Option<&'static str> {
    let (ex, sym) = item.split_once(':')?;
    let ins = OANDA.iter().find(|o| (ex == "oanda" && o.0 == sym) || (ex == "hl" && o.4 == Some(sym)))?;
    Some(match ins.0.split('_').next().unwrap_or("") {
        "XAU" => "🥇",
        "XAG" => "🥈",
        "WTICO" | "BCO" => "🛢",
        "NATGAS" => "🔥",
        "XCU" => "🟤",
        "XPT" => "⚪",
        "USB10Y" => "🏛",
        _ if ins.2 == "index" => "📊",
        _ => "💱",
    })
}

/// Nom complet (« Or / Dollar US »).
#[cfg(test)]
pub fn full_name(item: &str) -> Option<&'static str> {
    let (ex, sym) = item.split_once(':')?;
    OANDA
        .iter()
        .find(|o| (ex == "oanda" && o.0 == sym) || (ex == "hl" && o.4 == Some(sym)))
        .map(|o| o.1)
}

/// Instrument OANDA d'une ligne de watchlist (`oanda:XAU_USD` ou son équivalent `hl:xyz:GOLD`) ; `None` : pas OANDA.
/// Toute ligne `oanda:…` est un instrument OANDA (le catalogue complet de ton compte) ; `hl:…` seulement s'il a un
/// équivalent connu.
pub fn instrument_of(item: &str) -> Option<String> {
    let (ex, sym) = item.split_once(':')?;
    if ex == "oanda" && !sym.is_empty() {
        return Some(sym.to_string());
    }
    OANDA.iter().find(|o| ex == "hl" && o.4 == Some(sym)).map(|o| o.0.to_string())
}

/// Granularité OANDA d'une unité de temps (`None` : OANDA ne l'a pas, ex. 3 minutes).
pub fn granularity(tf: argos_core::Timeframe) -> Option<&'static str> {
    use argos_core::Timeframe::*;
    Some(match tf {
        M1 => "M1",
        M3 => return None,
        M5 => "M5",
        M15 => "M15",
        M30 => "M30",
        H1 => "H1",
        H2 => "H2",
        H4 => "H4",
        H12 => "H12",
        D1 => "D",
        W1 => "W",
    })
}

/// Jeton valide (forme seulement : lettres, chiffres et tirets, comme OANDA).
pub fn token_ok(token: &str) -> bool {
    (20..=100).contains(&token.len()) && token.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

/// Bougies de la réponse v20 d'OANDA (`price=M`, heures UNIX) : prix du milieu.
pub fn parse_v20(json: &str) -> argos_core::Result<Vec<argos_core::Bar>> {
    let v: serde_json::Value = serde_json::from_str(json).map_err(|e| format!("réponse OANDA illisible : {e}"))?;
    let num = |x: &serde_json::Value| x.as_str().and_then(|s| s.parse::<f64>().ok()).or_else(|| x.as_f64());
    Ok(v["candles"]
        .as_array()
        .map(|list| {
            list.iter()
                .filter_map(|c| {
                    let m = &c["mid"];
                    Some(argos_core::Bar {
                        time: num(&c["time"])? as i64,
                        open: num(&m["o"])?,
                        high: num(&m["h"])?,
                        low: num(&m["l"])?,
                        close: num(&m["c"])?,
                        volume: num(&c["volume"]).unwrap_or(0.0),
                    })
                })
                .collect()
        })
        .unwrap_or_default())
}

/// Bougies de la réponse du relais `/api/oanda` du site : `{"bars": [[t, o, h, l, c, v], …]}`.
pub fn parse_relay(json: &str) -> argos_core::Result<Vec<argos_core::Bar>> {
    let v: serde_json::Value = serde_json::from_str(json).map_err(|e| format!("réponse illisible : {e}"))?;
    if let Some(e) = v["error"].as_str() {
        return Err(e.into());
    }
    Ok(v["bars"]
        .as_array()
        .map(|list| {
            list.iter()
                .filter_map(|b| {
                    let f = |k: usize| b.get(k).and_then(|x| x.as_f64());
                    Some(argos_core::Bar {
                        time: f(0)? as i64,
                        open: f(1)?,
                        high: f(2)?,
                        low: f(3)?,
                        close: f(4)?,
                        volume: f(5).unwrap_or(0.0),
                    })
                })
                .collect()
        })
        .unwrap_or_default())
}

/// Serveur OANDA : compte réel ou démo (practice).
#[cfg(feature = "native")]
fn host(live: bool) -> &'static str {
    if live {
        "https://api-fxtrade.oanda.com"
    } else {
        "https://api-fxpractice.oanda.com"
    }
}

/// GET avec le jeton : desktop = directement chez OANDA (`path` de l'API v20).
#[cfg(feature = "native")]
async fn get(token: &str, live: bool, path: &str) -> Result<String, String> {
    let url = format!("{}{path}", host(live));
    crate::net::verifier(&url).map_err(|e| e.to_string())?;
    let c = reqwest::Client::builder()
        .user_agent("Argos/2.0")
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;
    let r = c
        .get(url)
        .bearer_auth(token)
        .header("accept-datetime-format", "UNIX")
        .send()
        .await
        .map_err(|e| format!("OANDA ne répond pas : {e}"))?;
    let status = r.status();
    let body = r.text().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        let msg = serde_json::from_str::<serde_json::Value>(&body)
            .ok()
            .and_then(|v| v["errorMessage"].as_str().map(str::to_string))
            .unwrap_or_else(|| format!("HTTP {status}"));
        return Err(msg);
    }
    Ok(body)
}

/// Navigateur : par le relais du site (OANDA ne répond pas directement à une page web) ; le jeton part dans l'en-tête.
#[cfg(all(feature = "web", not(feature = "native")))]
async fn relay(token: &str, query: &str) -> Result<String, String> {
    use wasm_bindgen::{JsCast, JsValue};
    // 📱 coque Tauri : aucune clé dans la page (étape T2 : coffre du système, appels par le cœur Rust)
    if crate::coque::active() {
        return Err(crate::coque_sans_cle());
    }
    let err = |e: JsValue| e.as_string().unwrap_or_else(|| "réseau".into());
    let init = web_sys::RequestInit::new();
    init.set_method("GET");
    let h = web_sys::Headers::new().map_err(err)?;
    h.set("x-oanda-token", token).map_err(err)?;
    init.set_headers(&h);
    let w = web_sys::window().ok_or("pas de fenêtre")?;
    let resp: web_sys::Response = wasm_bindgen_futures::JsFuture::from(w.fetch_with_str_and_init(&format!("/api/oanda?{query}"), &init))
        .await
        .map_err(err)?
        .dyn_into()
        .map_err(err)?;
    let text = wasm_bindgen_futures::JsFuture::from(resp.text().map_err(err)?)
        .await
        .map_err(err)?
        .as_string()
        .unwrap_or_default();
    if !resp.ok() {
        let msg = serde_json::from_str::<serde_json::Value>(&text)
            .ok()
            .and_then(|v| v["error"].as_str().map(str::to_string))
            .unwrap_or_else(|| format!("HTTP {}", resp.status()));
        return Err(msg);
    }
    Ok(text)
}

/// Bougies d'un instrument (`XAU_USD`) à cette unité de temps, les plus récentes (5000 au plus, la limite d'OANDA).
#[cfg(any(feature = "native", feature = "web"))]
pub async fn candles(instrument: &str, tf: argos_core::Timeframe, token: &str, live: bool) -> argos_core::Result<Vec<argos_core::Bar>> {
    let gran = granularity(tf).ok_or("OANDA n'a pas cette unité de temps")?;
    #[cfg(feature = "native")]
    return parse_v20(
        &get(
            token,
            live,
            &format!("/v3/instruments/{instrument}/candles?price=M&granularity={gran}&count=5000"),
        )
        .await?,
    );
    #[cfg(all(feature = "web", not(feature = "native")))]
    return parse_relay(&relay(token, &format!("env={}&inst={instrument}&gran={gran}", if live { "live" } else { "practice" })).await?);
}

/// 🏦 Catalogue OANDA complet de ton compte (`/v3/accounts/{id}/instruments`) : forex, métaux, indices, matières
/// premières, obligations… Une ligne par instrument, rangée dans son groupe.
pub fn parse_instruments(body: &str) -> argos_core::Result<Vec<argos_core::Instrument>> {
    let v: serde_json::Value = serde_json::from_str(body).map_err(|e| e.to_string())?;
    let rows = v["instruments"].as_array().ok_or("OANDA : liste des instruments absente")?;
    let mut out: Vec<argos_core::Instrument> = rows
        .iter()
        .filter_map(|i| {
            let name = i["name"].as_str()?.to_string();
            let (base, quote) = name
                .split_once('_')
                .map_or((name.clone(), String::new()), |(b, q)| (b.to_string(), q.to_string()));
            let group = match i["type"].as_str().unwrap_or("") {
                "CURRENCY" => "Forex",
                "METAL" => "Métaux",
                _ => match name.as_str() {
                    n if n.contains("USB") || n.contains("BUND") || n.contains("GILT") => "Obligations",
                    n if ["WTICO", "BCO", "NATGAS", "CORN", "SOYBN", "SUGAR", "WHEAT", "XCU"]
                        .iter()
                        .any(|c| n.starts_with(c)) =>
                    {
                        "Matières premières"
                    }
                    _ => "Indices",
                },
            };
            Some(argos_core::Instrument {
                exchange: "oanda".into(),
                symbol: name,
                base,
                quote,
                kind: argos_core::MarketKind::Spot,
                group: group.into(),
                price_decimals: i["displayPrecision"].as_u64().unwrap_or(5) as u32,
                size_decimals: 0,
                growth: false,
                active: true,
            })
        })
        .collect();
    out.sort_by(|a, b| a.symbol.cmp(&b.symbol));
    out.dedup_by(|a, b| a.symbol == b.symbol);
    Ok(out)
}

/// 🏦 Catalogue complet : le premier compte du jeton, puis ses instruments (desktop : le jeton reste au trousseau).
#[cfg(feature = "native")]
pub async fn instruments(token: &str, live: bool) -> argos_core::Result<Vec<argos_core::Instrument>> {
    let acc = get(token, live, "/v3/accounts").await?;
    let v: serde_json::Value = serde_json::from_str(&acc).map_err(|e| e.to_string())?;
    let id = v["accounts"][0]["id"].as_str().ok_or("aucun compte OANDA pour ce jeton")?;
    parse_instruments(&get(token, live, &format!("/v3/accounts/{id}/instruments")).await?)
}

/// Teste le jeton : nombre de comptes OANDA qu'il ouvre.
#[cfg(any(feature = "native", feature = "web"))]
pub async fn test(token: &str, live: bool) -> argos_core::Result<usize> {
    #[cfg(feature = "native")]
    let body = get(token, live, "/v3/accounts").await?;
    #[cfg(all(feature = "web", not(feature = "native")))]
    let body = relay(token, &format!("env={}&test=1", if live { "live" } else { "practice" })).await?;
    let v: serde_json::Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    Ok(v["accounts"].as_array().map_or_else(|| v["accounts"].as_u64().unwrap_or(0) as usize, Vec::len))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalogue_oanda_complet_par_groupe() {
        let body = r#"{"instruments":[{"name":"EUR_USD","type":"CURRENCY","displayPrecision":5},{"name":"XAU_USD","type":"METAL","displayPrecision":3},{"name":"SPX500_USD","type":"CFD","displayPrecision":1},{"name":"WTICO_USD","type":"CFD","displayPrecision":3},{"name":"EUR_USD","type":"CURRENCY","displayPrecision":5}]}"#;
        let l = parse_instruments(body).unwrap();
        assert_eq!(l.len(), 4, "pas de doublon");
        let g = |s: &str| l.iter().find(|i| i.symbol == s).map(|i| i.group.clone()).unwrap();
        assert_eq!(
            (g("EUR_USD"), g("XAU_USD"), g("SPX500_USD"), g("WTICO_USD")),
            ("Forex".into(), "Métaux".into(), "Indices".into(), "Matières premières".into())
        );
        assert_eq!(l.iter().find(|i| i.symbol == "EUR_USD").unwrap().id(), "oanda:EUR_USD");
    }

    #[test]
    fn source_oanda() {
        assert_eq!(instrument_of("hl:xyz:GOLD").as_deref(), Some("XAU_USD"));
        assert_eq!(instrument_of("oanda:USD_CAD").as_deref(), Some("USD_CAD"));
        assert_eq!(
            instrument_of("oanda:DE30_EUR").as_deref(),
            Some("DE30_EUR"),
            "catalogue complet : tout instrument OANDA"
        );
        assert_eq!(instrument_of("hl:BTC"), None);
        assert_eq!(granularity(argos_core::Timeframe::D1), Some("D"));
        assert_eq!(granularity(argos_core::Timeframe::M3), None);
        assert!(token_ok("abcdef0123456789abcdef-0123456789abcdef") && !token_ok("trop court") && !token_ok("a b c d e f g h i j k l m n"));
        let v20 = r#"{"candles":[{"complete":true,"volume":12,"time":"1700000000.000000000","mid":{"o":"1990.1","h":"1995.5","l":"1988.0","c":"1993.2"}}]}"#;
        let b = parse_v20(v20).unwrap();
        assert_eq!((b[0].time, b[0].close, b[0].volume), (1_700_000_000, 1993.2, 12.0));
        assert_eq!(parse_relay(r#"{"bars":[[1700000000,1,2,0.5,1.5,7]]}"#).unwrap()[0].high, 2.0);
        assert_eq!(
            parse_relay(r#"{"error":"token OANDA manquant ou invalide"}"#).unwrap_err().to_string(),
            "token OANDA manquant ou invalide"
        );
    }

    #[test]
    fn liste_oanda() {
        assert_eq!(item("XAU_USD"), "hl:xyz:GOLD");
        assert_eq!(item("USD_CAD"), "oanda:USD_CAD");
        assert_eq!(label("hl:xyz:GOLD").as_deref(), Some("XAUUSD"));
        assert_eq!(label("oanda:USD_CAD").as_deref(), Some("USDCAD"));
        assert_eq!(label("hl:BTC"), None);
        assert_eq!(full_name("hl:xyz:GOLD"), Some("Or / Dollar US"));
        assert_eq!(
            (emoji("hl:xyz:GOLD"), emoji("oanda:USD_CAD"), emoji("hl:xyz:SP500"), emoji("hl:BTC")),
            (Some("🥇"), Some("💱"), Some("📊"), None)
        );
        // chaque symbole de la liste existe dans la table, une seule fois
        let all: Vec<&str> = SEED.iter().flat_map(|(_, l)| l.iter().copied()).collect();
        assert_eq!(all.len(), 28);
        assert!(all.iter().all(|s| OANDA.iter().filter(|o| o.0 == *s).count() == 1));
    }
}
