//! Hyperliquid (perps). API publique : POST https://api.hyperliquid.xyz/info

use super::Request;
use crate::parse::{json, num, secs};
use argos_core::feed::Market;
use argos_core::{ArgosError, Bar, Result, Timeframe};
use argos_core::{Instrument, MarketKind};

const INFO: &str = "https://api.hyperliquid.xyz/info";

/// `end_ms` : seulement les bougies d'avant ce moment (ms) ; `None` = jusqu'à maintenant. 5000 bougies max par requête.
/// Nom exact pour l'API : un marché HIP-3 garde son dex en minuscules (« XYZ:GOLD » → « xyz:GOLD ») ;
/// Hyperliquid refuse sinon (« null »). Les autres noms ne changent pas.
pub fn coin(base: &str) -> String {
    match base.split_once(':') {
        Some((dex, asset)) => format!("{}:{asset}", dex.to_lowercase()),
        None => base.to_string(),
    }
}

pub fn candles_request(base: &str, tf: Timeframe, limit: usize, now_ms: i64, end_ms: Option<i64>) -> Result<Request> {
    let base = coin(base);
    let end = end_ms.unwrap_or(now_ms);
    let start = end - tf.seconds() * 1000 * limit.min(5000) as i64;
    let body = format!(
        r#"{{"type":"candleSnapshot","req":{{"coin":"{base}","interval":"{}","startTime":{start},"endTime":{end}}}}}"#,
        tf.id()
    );
    Ok(Request::PostJson { url: INFO.into(), body })
}

pub fn parse_candles(body: &str) -> Result<Vec<Bar>> {
    let v = json(body)?;
    let rows = v.as_array().ok_or_else(|| ArgosError::Feed("Hyperliquid : réponse inattendue".into()))?;
    Ok(rows
        .iter()
        .map(|k| Bar {
            time: secs(&k["t"]),
            open: num(&k["o"]),
            high: num(&k["h"]),
            low: num(&k["l"]),
            close: num(&k["c"]),
            volume: num(&k["v"]),
        })
        .collect())
}

pub fn markets_request() -> Request {
    Request::PostJson {
        url: INFO.into(),
        body: r#"{"type":"metaAndAssetCtxs"}"#.into(),
    }
}

pub fn parse_markets(body: &str) -> Result<Vec<Market>> {
    let v = json(body)?;
    let (meta, ctx) = (&v[0]["universe"], &v[1]);
    let list = meta
        .as_array()
        .ok_or_else(|| ArgosError::Feed("Hyperliquid : liste des marchés absente".into()))?;
    Ok(list
        .iter()
        .enumerate()
        .filter(|(_, u)| !u["isDelisted"].as_bool().unwrap_or(false))
        .map(|(i, u)| Market {
            base: u["name"].as_str().unwrap_or("").to_string(),
            price: num(&ctx[i]["markPx"]),
            prev: num(&ctx[i]["prevDayPx"]),
        })
        .collect())
}

/// Catalogue d'un dex (`""` = Hyperliquid lui-même, sinon un dex HIP-3).
pub fn instruments_request(dex: &str) -> Request {
    let body = if dex.is_empty() {
        r#"{"type":"meta"}"#.to_string()
    } else {
        format!(r#"{{"type":"meta","dex":"{dex}"}}"#)
    };
    Request::PostJson { url: INFO.into(), body }
}

/// La liste des dex HIP-3.
pub fn dexes_request() -> Request {
    Request::PostJson {
        url: INFO.into(),
        body: r#"{"type":"perpDexs"}"#.into(),
    }
}

/// Noms des dex HIP-3 (le premier élément, `null`, est Hyperliquid lui-même).
pub fn parse_dexes(body: &str) -> Result<Vec<String>> {
    let v = json(body)?;
    let list = v.as_array().ok_or_else(|| ArgosError::Feed("Hyperliquid perpDexs : liste absente".into()))?;
    Ok(list
        .iter()
        .filter_map(|d| d["name"].as_str().map(str::to_string))
        .filter(|n| !n.is_empty())
        .collect())
}

/// Symboles d'un dex. Décimales du prix : 6 − décimales de la taille (règle d'Hyperliquid pour les perps).
pub fn parse_instruments(body: &str, dex: &str) -> Result<Vec<Instrument>> {
    let v = json(body)?;
    let list = v["universe"]
        .as_array()
        .ok_or_else(|| ArgosError::Feed("Hyperliquid : liste des marchés absente".into()))?;
    Ok(list
        .iter()
        .filter_map(|u| {
            let name = u["name"].as_str()?;
            let symbol = if dex.is_empty() || name.contains(':') {
                name.to_string()
            } else {
                format!("{dex}:{name}")
            };
            let base = symbol.rsplit(':').next().unwrap_or(&symbol).to_string();
            let sz = u["szDecimals"].as_u64().unwrap_or(0) as u32;
            let g = &u["growthMode"];
            Some(Instrument {
                exchange: "hl".into(),
                base,
                symbol,
                quote: "USDC".into(),
                kind: MarketKind::Perp,
                group: if dex.is_empty() { "Crypto".to_string() } else { format!("HIP-3 {dex}") },
                price_decimals: 6u32.saturating_sub(sz),
                size_decimals: sz,
                growth: g.as_str() == Some("enabled") || g.as_bool() == Some(true),
                active: !u["isDelisted"].as_bool().unwrap_or(false),
            })
        })
        .collect())
}

#[cfg(test)]
mod tests {

    #[test]
    fn nom_hip3_garde_son_dex_en_minuscules() {
        assert_eq!(coin("XYZ:GOLD"), "xyz:GOLD", "le flux met tout en majuscules : Hyperliquid refuse « XYZ:GOLD »");
        assert_eq!(coin("BTC"), "BTC");
        let Request::PostJson { body, .. } = candles_request("XYZ:GBP", Timeframe::H1, 10, 1_000_000_000, None).unwrap() else {
            panic!("POST attendu")
        };
        assert!(body.contains(r#""coin":"xyz:GBP""#), "{body}");
    }

    use super::*;

    #[test]
    fn requete_et_lecture() {
        let Request::PostJson { body, .. } = candles_request("BTC", Timeframe::M30, 2, 10_000_000, None).unwrap() else {
            panic!()
        };
        assert!(body.contains(r#""coin":"BTC""#) && body.contains(r#""interval":"30m""#) && body.contains(r#""startTime":6400000"#));
        let b =
            parse_candles(r#"[{"t":1700000000000,"T":1700001799999,"s":"BTC","i":"30m","o":"80000","c":"80100","h":"80200","l":"79900","v":"12.5","n":100}]"#)
                .unwrap();
        assert_eq!(
            b[0],
            Bar {
                time: 1_700_000_000,
                open: 80000.0,
                high: 80200.0,
                low: 79900.0,
                close: 80100.0,
                volume: 12.5
            }
        );
        let m = parse_markets(r#"[{"universe":[{"name":"BTC"},{"name":"OLD","isDelisted":true},{"name":"ETH"}]},[{"markPx":"80000","prevDayPx":"79000"},{},{"markPx":"2300","prevDayPx":"2350"}]]"#).unwrap();
        assert_eq!(m.len(), 2);
        assert_eq!(
            m[1],
            Market {
                base: "ETH".into(),
                price: 2300.0,
                prev: 2350.0
            }
        );
    }
}

#[cfg(test)]
mod catalog_tests {
    use super::*;

    #[test]
    fn catalogue() {
        let main = parse_instruments(
            r#"{"universe":[{"name":"BTC","szDecimals":5},{"name":"OLD","szDecimals":0,"isDelisted":true}]}"#,
            "",
        )
        .unwrap();
        assert_eq!(
            (main[0].id().as_str(), main[0].price_decimals, main[0].size_decimals, main[0].group.as_str()),
            ("hl:BTC", 1, 5, "Crypto")
        );
        assert!(!main[1].active);
        let hip = parse_instruments(
            r#"{"universe":[{"szDecimals":4,"name":"xyz:XYZ100","growthMode":"enabled"},{"szDecimals":2,"name":"xyz:GOLD"}]}"#,
            "xyz",
        )
        .unwrap();
        assert_eq!(
            (hip[0].id().as_str(), hip[0].base.as_str(), hip[0].group.as_str(), hip[0].growth),
            ("hl:xyz:XYZ100", "XYZ100", "HIP-3 xyz", true)
        );
        assert!(!hip[1].growth);
        assert_eq!(parse_dexes(r#"[null,{"name":"xyz"},{"name":"flx"}]"#).unwrap(), vec!["xyz", "flx"]);
    }
}
