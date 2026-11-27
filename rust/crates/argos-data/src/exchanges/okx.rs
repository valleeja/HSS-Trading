//! OKX (perps USDT, instId « BTC-USDT-SWAP »). API publique v5.

use super::Request;
use crate::parse::{decimals, text};
use crate::parse::{json, num, ok_code};
use argos_core::feed::Market;
use argos_core::{ArgosError, Bar, Result, Timeframe};
use argos_core::{Instrument, MarketKind};

const REST: &str = "https://www.okx.com/api/v5";
const WS: &str = "wss://ws.okx.com:8443/ws/v5/business";

/// Direct des bougies (canal `candle30m`, instId `BTC-USDT-SWAP`).
pub fn live_sub(base: &str, tf: Timeframe) -> crate::live::LiveSub {
    crate::live::LiveSub {
        url: WS.into(),
        subscribe: format!(
            r#"{{"op":"subscribe","args":[{{"channel":"candle{}","instId":"{}-USDT-SWAP"}}]}}"#,
            bar(tf),
            base.to_uppercase()
        ),
        ping: "ping".into(),
        ..Default::default()
    }
}

/// Where an OKX candle gives its volume in coins. A perp gives it at index 6 (`volCcy`, « the quantity of base
/// currency »), its index 5 counts contracts (`vol`: 0.01 BTC each, seen on 23 Nov. 2026: 484 074 for 4 840 BTC in
/// an hour). A spot pair gives coins at 5 (its 6 is in the quote currency).
pub const COINS_PERP: usize = 6;
pub const COINS_SPOT: usize = 5;

/// Bougie d'un message du direct (`{"arg":{"channel":"candle…"},"data":[["ts","o","h","l","c","vol","volCcy",…]]}`).
pub fn parse_live(msg: &str) -> Option<Bar> {
    parse_live_in(msg, COINS_PERP)
}

/// [`parse_live`], the volume in coins read at `coins`.
pub fn parse_live_in(msg: &str, coins: usize) -> Option<Bar> {
    let v = json(msg).ok()?;
    if !v["arg"]["channel"].as_str()?.starts_with("candle") {
        return None;
    }
    crate::parse::valid(crate::parse::bar_from_array_vol(v["data"].as_array()?.first()?, coins))
}

fn bar(tf: Timeframe) -> &'static str {
    match tf {
        Timeframe::H1 => "1H",
        Timeframe::H2 => "2H",
        Timeframe::H4 => "4H",
        // 12 h, days and weeks counted from 00:00 UTC: OKX's own « 12H » begins at 00:00 in Hong Kong (16:00 UTC)
        Timeframe::H12 => "12Hutc",
        Timeframe::D1 => "1Dutc",
        Timeframe::W1 => "1Wutc",
        t => t.id(),
    }
}

/// `end_ms` : seulement les bougies d'avant ce moment (ms), pour paginer vers le passé ; `None` = les plus récentes.
/// Les plus récentes viennent de `/market/candles` (300 max), le passé de `/market/history-candles` (100 max).
pub fn candles_request(base: &str, tf: Timeframe, limit: usize, end_ms: Option<i64>) -> Result<Request> {
    Ok(Request::Get(match end_ms {
        None => format!("{REST}/market/candles?instId={base}-USDT-SWAP&bar={}&limit={}", bar(tf), limit.min(300)),
        Some(e) => format!(
            "{REST}/market/history-candles?instId={base}-USDT-SWAP&bar={}&after={e}&limit={}",
            bar(tf),
            limit.min(100)
        ),
    }))
}

pub fn parse_candles(body: &str) -> Result<Vec<Bar>> {
    parse_candles_in(body, COINS_PERP)
}

/// [`parse_candles`], the volume in coins read at `coins`.
pub fn parse_candles_in(body: &str, coins: usize) -> Result<Vec<Bar>> {
    let v = json(body)?;
    ok_code(&v, "code", "OKX")?;
    let list = v["data"].as_array().map(Vec::as_slice).unwrap_or_default();
    Ok(list.iter().map(|a| crate::parse::bar_from_array_vol(a, coins)).collect())
}

pub fn markets_request() -> Request {
    Request::Get(format!("{REST}/market/tickers?instType=SWAP"))
}

pub fn parse_markets(body: &str) -> Result<Vec<Market>> {
    let v = json(body)?;
    ok_code(&v, "code", "OKX")?;
    let rows = v["data"].as_array().ok_or_else(|| ArgosError::Feed("OKX : liste absente".into()))?;
    Ok(rows
        .iter()
        .filter_map(|t| {
            let base = t["instId"].as_str()?.strip_suffix("-USDT-SWAP")?;
            Some(Market {
                base: base.to_string(),
                price: num(&t["last"]),
                prev: num(&t["open24h"]),
            })
        })
        .collect())
}

/// Catalogue : les perps (SWAP) linéaires.
pub fn instruments_request() -> Request {
    Request::Get(format!("{REST}/public/instruments?instType=SWAP"))
}

pub fn parse_instruments(body: &str) -> Result<Vec<Instrument>> {
    let v = json(body)?;
    ok_code(&v, "code", "OKX")?;
    let rows = v["data"].as_array().ok_or_else(|| ArgosError::Feed("OKX : liste absente".into()))?;
    Ok(rows
        .iter()
        .filter(|s| s["ctType"] == "linear")
        .map(|s| Instrument {
            exchange: "okx".into(),
            symbol: text(&s["instId"]),
            base: text(&s["ctValCcy"]),
            quote: text(&s["settleCcy"]),
            kind: MarketKind::Perp,
            group: "Crypto".into(),
            price_decimals: decimals(&s["tickSz"]),
            size_decimals: decimals(&s["lotSz"]),
            growth: false,
            active: s["state"] == "live",
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requete_et_lecture() {
        assert_eq!(
            candles_request("ZEC", Timeframe::D1, 600, None).unwrap(),
            Request::Get("https://www.okx.com/api/v5/market/candles?instId=ZEC-USDT-SWAP&bar=1Dutc&limit=300".into())
        );
        let b = parse_candles(
            r#"{"code":"0","msg":"","data":[["1700001800000","2","3","1","2.5","100","x","y","1"],["1700000000000","1","2","0.5","2","50","x","y","1"]]}"#,
        )
        .unwrap();
        assert_eq!(b.len(), 2);
        assert_eq!(b[1].close, 2.0);
        assert!(parse_candles(r#"{"code":"51001","msg":"Instrument ID does not exist","data":[]}"#).is_err());
        let m = parse_markets(
            r#"{"code":"0","data":[{"instId":"BTC-USDT-SWAP","last":"80000","open24h":"79000"},{"instId":"BTC-USD-SWAP","last":"1","open24h":"1"}]}"#,
        )
        .unwrap();
        assert_eq!(
            m,
            vec![Market {
                base: "BTC".into(),
                price: 80000.0,
                prev: 79000.0
            }]
        );
    }

    /// 🔒 Reliable data: OKX's real answers of 23 Nov. 2026. A perp gives its volume in contracts of 0.01 BTC at
    /// index 5 (422.64) and in coins at 6 (4.2264): the coins are read, from the candles and their live messages; a
    /// spot pair gives coins at 5 (its 6 is in dollars). The 12 hours, days and weeks are asked from 00:00 UTC
    /// (« 12Hutc »…): OKX's own « 12H » begins at 00:00 in Hong Kong.
    #[test]
    fn okx_volumes_in_coins_and_days_in_utc() {
        let perp = r#"{"code":"0","data":[["1795470240000","61254.2","61270.8","61254.2","61270.8","422.64","4.2264","258886.49955","0"]]}"#;
        assert_eq!(parse_candles(perp).unwrap()[0].volume, 4.2264);
        let live = r#"{"arg":{"channel":"candle1m","instId":"BTC-USDT-SWAP"},"data":[["1795470240000","61254.2","61270.8","61254.2","61270.8","422.64","4.2264","258886.49955","0"]]}"#;
        assert_eq!(parse_live(live).map(|b| b.volume), Some(4.2264));
        let spot = r#"{"code":"0","data":[["1795470240000","61288.3","61313","61253.2","61266.2","5.31868739","325962.867454732","325962.867454732","0"]]}"#;
        assert_eq!(parse_candles_in(spot, COINS_SPOT).unwrap()[0].volume, 5.31868739);
        for (tf, name) in [(Timeframe::H12, "12Hutc"), (Timeframe::D1, "1Dutc"), (Timeframe::W1, "1Wutc")] {
            let r = candles_request("BTC", tf, 5, None).unwrap();
            assert!(matches!(&r, Request::Get(u) if u.contains(&format!("&bar={name}&"))), "{r:?}");
            let sub = live_sub("BTC", tf).subscribe;
            assert!(sub.contains(&format!(r#""channel":"candle{name}""#)), "{sub}");
        }
    }
}

#[cfg(test)]
mod catalog_tests {
    use super::*;

    #[test]
    fn catalogue() {
        let body = r#"{"code":"0","data":[{"instId":"BTC-USDT-SWAP","ctType":"linear","ctValCcy":"BTC","settleCcy":"USDT","tickSz":"0.1","lotSz":"0.01","state":"live"},
            {"instId":"BTC-USD-SWAP","ctType":"inverse","ctValCcy":"USD","settleCcy":"BTC","tickSz":"0.1","lotSz":"1","state":"live"}]}"#;
        let l = parse_instruments(body).unwrap();
        assert_eq!(l.len(), 1, "les inverses sont exclus");
        assert_eq!(
            (
                l[0].id().as_str(),
                l[0].base.as_str(),
                l[0].quote.as_str(),
                l[0].price_decimals,
                l[0].size_decimals
            ),
            ("okx:BTC-USDT-SWAP", "BTC", "USDT", 1, 2)
        );
    }
}
