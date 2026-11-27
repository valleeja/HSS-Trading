//! Bybit (perps USDT « linear », symbole « BTCUSDT »). API publique v5.

use super::Request;
use crate::parse::{bars, json, num, ok_code};
use crate::parse::{decimals, text};
use argos_core::feed::Market;
use argos_core::{ArgosError, Bar, Result, Timeframe};
use argos_core::{Instrument, MarketKind};

const REST: &str = "https://api.bybit.com/v5/market";
const WS: &str = "wss://stream.bybit.com/v5/public/linear";

/// Direct des bougies (`kline.30.BTCUSDT`).
pub fn live_sub(base: &str, tf: Timeframe) -> crate::live::LiveSub {
    crate::live::LiveSub {
        url: WS.into(),
        subscribe: format!(r#"{{"op":"subscribe","args":["kline.{}.{}USDT"]}}"#, interval(tf), base.to_uppercase()),
        ping: r#"{"op":"ping"}"#.into(),
        ..Default::default()
    }
}

/// Bougie d'un message du direct (`{"topic":"kline.…","data":[{"start":…,"open":…}]}`).
pub fn parse_live(msg: &str) -> Option<Bar> {
    let v = json(msg).ok()?;
    if !v["topic"].as_str()?.starts_with("kline.") {
        return None;
    }
    crate::parse::valid(crate::parse::bar_from_object(v["data"].as_array()?.last()?))
}

fn interval(tf: Timeframe) -> &'static str {
    match tf {
        Timeframe::M1 => "1",
        Timeframe::M3 => "3",
        Timeframe::M5 => "5",
        Timeframe::M15 => "15",
        Timeframe::M30 => "30",
        Timeframe::H1 => "60",
        Timeframe::H2 => "120",
        Timeframe::H4 => "240",
        Timeframe::H12 => "720",
        Timeframe::D1 => "D",
        Timeframe::W1 => "W",
    }
}

/// `end_ms` : seulement les bougies d'avant ce moment (ms), pour paginer vers le passé ; `None` = les plus récentes.
pub fn candles_request(base: &str, tf: Timeframe, limit: usize, end_ms: Option<i64>) -> Result<Request> {
    let end = end_ms.map_or(String::new(), |e| format!("&end={}", e));
    Ok(Request::Get(format!(
        "{REST}/kline?category=linear&symbol={base}USDT&interval={}&limit={}{end}",
        interval(tf),
        limit.min(1000)
    )))
}

pub fn parse_candles(body: &str) -> Result<Vec<Bar>> {
    let v = json(body)?;
    ok_code(&v, "retCode", "Bybit")?;
    Ok(bars(&v["result"]["list"]))
}

pub fn markets_request() -> Request {
    Request::Get(format!("{REST}/tickers?category=linear"))
}

pub fn parse_markets(body: &str) -> Result<Vec<Market>> {
    let v = json(body)?;
    ok_code(&v, "retCode", "Bybit")?;
    let rows = v["result"]["list"].as_array().ok_or_else(|| ArgosError::Feed("Bybit : liste absente".into()))?;
    Ok(rows
        .iter()
        .filter_map(|t| {
            let base = t["symbol"].as_str()?.strip_suffix("USDT")?;
            Some(Market {
                base: base.to_string(),
                price: num(&t["lastPrice"]),
                prev: num(&t["prevPrice24h"]),
            })
        })
        .collect())
}

/// Catalogue : les perps linéaires (USDT, USDC), page par page (`cursor` = page suivante).
pub fn instruments_request(cursor: Option<&str>) -> Request {
    let next = cursor.filter(|c| !c.is_empty()).map_or(String::new(), |c| format!("&cursor={c}"));
    Request::Get(format!("{REST}/instruments-info?category=linear&limit=1000{next}"))
}

/// (symboles, page suivante).
pub fn parse_instruments(body: &str) -> Result<(Vec<Instrument>, Option<String>)> {
    let v = json(body)?;
    ok_code(&v, "retCode", "Bybit")?;
    let rows = v["result"]["list"].as_array().ok_or_else(|| ArgosError::Feed("Bybit : liste absente".into()))?;
    let list = rows
        .iter()
        .filter(|s| s["contractType"] == "LinearPerpetual")
        .map(|s| Instrument {
            exchange: "bybit".into(),
            symbol: text(&s["symbol"]),
            base: text(&s["baseCoin"]),
            quote: text(&s["quoteCoin"]),
            kind: MarketKind::Perp,
            group: "Crypto".into(),
            price_decimals: decimals(&s["priceFilter"]["tickSize"]),
            size_decimals: decimals(&s["lotSizeFilter"]["qtyStep"]),
            growth: false,
            active: s["status"] == "Trading",
        })
        .collect();
    let next = v["result"]["nextPageCursor"].as_str().filter(|c| !c.is_empty()).map(str::to_string);
    Ok((list, next))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requete_et_lecture() {
        assert_eq!(
            candles_request("SOL", Timeframe::H1, 500, None).unwrap(),
            Request::Get("https://api.bybit.com/v5/market/kline?category=linear&symbol=SOLUSDT&interval=60&limit=500".into())
        );
        let b =
            parse_candles(r#"{"retCode":0,"retMsg":"OK","result":{"category":"linear","list":[["1700000000000","100","101","99","100.5","1000","100000"]]}}"#)
                .unwrap();
        assert_eq!(b[0].high, 101.0);
        assert!(parse_candles(r#"{"retCode":10001,"retMsg":"params error","result":{}}"#).is_err());
        let m = parse_markets(r#"{"retCode":0,"result":{"list":[{"symbol":"SOLUSDT","lastPrice":"100","prevPrice24h":"95"}]}}"#).unwrap();
        assert_eq!(m[0].base, "SOL");
    }
}

#[cfg(test)]
mod catalog_tests {
    use super::*;

    #[test]
    fn catalogue() {
        let body = r#"{"retCode":0,"result":{"list":[{"symbol":"0GUSDT","contractType":"LinearPerpetual","status":"Trading","baseCoin":"0G","quoteCoin":"USDT","priceFilter":{"tickSize":"0.0001"},"lotSizeFilter":{"qtyStep":"0.1"}},
            {"symbol":"BTCUSDT-26DEC26","contractType":"LinearFutures","status":"Trading","baseCoin":"BTC","quoteCoin":"USDT","priceFilter":{"tickSize":"0.5"},"lotSizeFilter":{"qtyStep":"0.001"}}],"nextPageCursor":"abc"}}"#;
        let (l, next) = parse_instruments(body).unwrap();
        assert_eq!(l.len(), 1);
        assert_eq!((l[0].id().as_str(), l[0].price_decimals, l[0].size_decimals), ("bybit:0GUSDT", 4, 1));
        assert_eq!(next.as_deref(), Some("abc"));
        let Request::Get(u) = instruments_request(Some("abc")) else { panic!() };
        assert!(u.ends_with("&cursor=abc"));
    }
}
