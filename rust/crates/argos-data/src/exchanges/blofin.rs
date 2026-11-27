//! BloFin (perps USDT, instId « BTC-USDT »). API publique v1.

use super::Request;
use crate::parse::{decimals, text};
use crate::parse::{json, num, ok_code};
use argos_core::feed::Market;
use argos_core::{ArgosError, Bar, Result, Timeframe};
use argos_core::{Instrument, MarketKind};

const REST: &str = "https://openapi.blofin.com/api/v1/market";
const WS: &str = "wss://openapi.blofin.com/ws/public";

/// Direct des bougies (canal `candle30m`, instId `BTC-USDT`).
pub fn live_sub(base: &str, tf: Timeframe) -> crate::live::LiveSub {
    crate::live::LiveSub {
        url: WS.into(),
        subscribe: format!(
            r#"{{"op":"subscribe","args":[{{"channel":"candle{}","instId":"{}-USDT"}}]}}"#,
            bar(tf),
            base.to_uppercase()
        ),
        ping: "ping".into(),
        ..Default::default()
    }
}

/// Where a BloFin candle gives its volume in coins: index 6 (`volCurrency`). Its index 5 counts contracts (0.001 BTC
/// each, seen on 23 Nov. 2026).
const COINS: usize = 6;

/// Bougie d'un message du direct (`{"arg":{"channel":"candle…"},"data":[["ts","o","h","l","c","vol","volCurrency",…]]}`).
pub fn parse_live(msg: &str) -> Option<Bar> {
    let v = json(msg).ok()?;
    if !v["arg"]["channel"].as_str()?.starts_with("candle") {
        return None;
    }
    crate::parse::valid(crate::parse::bar_from_array_vol(v["data"].as_array()?.first()?, COINS))
}

fn bar(tf: Timeframe) -> &'static str {
    match tf {
        Timeframe::H1 => "1H",
        Timeframe::H2 => "2H",
        Timeframe::H4 => "4H",
        Timeframe::H12 => "12H",
        Timeframe::D1 => "1D",
        Timeframe::W1 => "1W",
        t => t.id(),
    }
}

/// `end_ms` : seulement les bougies d'avant ce moment (ms), pour paginer vers le passé ; `None` = les plus récentes.
pub fn candles_request(base: &str, tf: Timeframe, limit: usize, end_ms: Option<i64>) -> Result<Request> {
    let end = end_ms.map_or(String::new(), |e| format!("&after={}", e));
    Ok(Request::Get(format!(
        "{REST}/candles?instId={base}-USDT&bar={}&limit={}{end}",
        bar(tf),
        limit.min(1440)
    )))
}

pub fn parse_candles(body: &str) -> Result<Vec<Bar>> {
    let v = json(body)?;
    ok_code(&v, "code", "BloFin")?;
    let list = v["data"].as_array().map(Vec::as_slice).unwrap_or_default();
    Ok(list.iter().map(|a| crate::parse::bar_from_array_vol(a, COINS)).collect())
}

pub fn markets_request() -> Request {
    Request::Get(format!("{REST}/tickers"))
}

pub fn parse_markets(body: &str) -> Result<Vec<Market>> {
    let v = json(body)?;
    ok_code(&v, "code", "BloFin")?;
    let rows = v["data"].as_array().ok_or_else(|| ArgosError::Feed("BloFin : liste absente".into()))?;
    Ok(rows
        .iter()
        .filter_map(|t| {
            let base = t["instId"].as_str()?.strip_suffix("-USDT")?;
            Some(Market {
                base: base.to_string(),
                price: num(&t["last"]),
                prev: num(&t["open24h"]),
            })
        })
        .collect())
}

/// Catalogue : les perps (SWAP) linéaires (USDT, USDC).
pub fn instruments_request() -> Request {
    Request::Get(format!("{REST}/instruments?instType=SWAP"))
}

pub fn parse_instruments(body: &str) -> Result<Vec<Instrument>> {
    let v = json(body)?;
    ok_code(&v, "code", "BloFin")?;
    let rows = v["data"].as_array().ok_or_else(|| ArgosError::Feed("BloFin : liste absente".into()))?;
    Ok(rows
        .iter()
        .filter(|s| s["contractType"] == "linear")
        .map(|s| Instrument {
            exchange: "blofin".into(),
            symbol: text(&s["instId"]),
            base: text(&s["baseCurrency"]),
            quote: text(&s["quoteCurrency"]),
            kind: MarketKind::Perp,
            group: s["assetClass"].as_str().filter(|c| !c.is_empty()).unwrap_or("Crypto").into(),
            price_decimals: decimals(&s["tickSize"]),
            size_decimals: decimals(&s["lotSize"]),
            growth: false,
            active: s["state"] == "live",
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 🔒 Reliable data: BloFin's real answer of 23 Nov. 2026: its volume in contracts of 0.001 BTC at index 5 (57,
    /// whole contracts) and in coins at 6 (0.0577): the coins are read, from the candles and their live messages.
    #[test]
    fn blofin_volume_in_coins() {
        let body = r#"{"code":"0","msg":"success","data":[["1795470240000","61263.5","61282.1","61263.5","61282.1","57","0.0577","3535.35397","0"]]}"#;
        assert_eq!(parse_candles(body).unwrap()[0].volume, 0.0577);
        let live = r#"{"arg":{"channel":"candle1m","instId":"BTC-USDT"},"data":[["1795470240000","61263.5","61282.1","61263.5","61282.1","57","0.0577","3535.35397","0"]]}"#;
        assert_eq!(parse_live(live).map(|b| b.volume), Some(0.0577));
    }

    #[test]
    fn requete_et_lecture() {
        assert_eq!(
            candles_request("BTC", Timeframe::H4, 3000, None).unwrap(),
            Request::Get("https://openapi.blofin.com/api/v1/market/candles?instId=BTC-USDT&bar=4H&limit=1440".into())
        );
        let b = parse_candles(r#"{"code":"0","msg":"success","data":[["1700000000000","80000","80200","79900","80100","12","x","y","1"]]}"#).unwrap();
        assert_eq!(b[0].open, 80000.0);
        let m = parse_markets(r#"{"code":"0","data":[{"instId":"BTC-USDT","last":"80000","open24h":"79000"}]}"#).unwrap();
        assert_eq!(m[0].base, "BTC");
    }
}

#[cfg(test)]
mod catalog_tests {
    use super::*;

    #[test]
    fn catalogue() {
        let body = r#"{"code":"0","data":[{"instId":"BTC-USDC","baseCurrency":"BTC","quoteCurrency":"USDC","assetClass":"Crypto","lotSize":"1","tickSize":"0.1","contractType":"linear","state":"live"},
            {"instId":"BTC-USD","baseCurrency":"BTC","quoteCurrency":"USD","assetClass":"Crypto","lotSize":"1","tickSize":"0.1","contractType":"inverse","state":"live"}]}"#;
        let l = parse_instruments(body).unwrap();
        assert_eq!(l.len(), 1);
        assert_eq!((l[0].id().as_str(), l[0].quote.as_str(), l[0].price_decimals), ("blofin:BTC-USDC", "USDC", 1));
    }
}
