//! Bitget (perps USDT, « BTCUSDT »). API publique v2 (testée en direct le 2026-11-05, sans clé).

use super::Request;
use crate::parse::{bars, decimals, json, num, text};
use argos_core::feed::Market;
use argos_core::{ArgosError, Bar, Result, Timeframe};
use argos_core::{Instrument, MarketKind};

const REST: &str = "https://api.bitget.com/api/v2/mix/market";
const WS: &str = "wss://ws.bitget.com/v2/ws/public";

fn gran(tf: Timeframe) -> Result<&'static str> {
    Ok(match tf {
        Timeframe::M1 => "1m",
        Timeframe::M3 => "3m",
        Timeframe::M5 => "5m",
        Timeframe::M15 => "15m",
        Timeframe::M30 => "30m",
        Timeframe::H1 => "1H",
        Timeframe::H4 => "4H",
        // 12 h, days and weeks from 00:00 UTC, for the candles and their live channel: Bitget's own « 1D » begins at
        // 00:00 in Asia (16:00 UTC, seen on 23 Nov. 2026)
        Timeframe::H12 => "12Hutc",
        Timeframe::D1 => "1Dutc",
        Timeframe::W1 => "1Wutc",
        Timeframe::H2 => return Err(ArgosError::Feed("Bitget : pas de bougies 2 h".into())),
    })
}

fn ok(v: &serde_json::Value) -> Result<()> {
    if v["code"] == "00000" {
        Ok(())
    } else {
        Err(ArgosError::Feed(format!("Bitget : {}", v["msg"].as_str().unwrap_or("erreur"))))
    }
}

/// `end_ms` : seulement les bougies d'avant ce moment (ms) ; `None` = les plus récentes.
pub fn candles_request(base: &str, tf: Timeframe, limit: usize, end_ms: Option<i64>) -> Result<Request> {
    let end = end_ms.map_or(String::new(), |e| format!("&endTime={e}"));
    Ok(Request::Get(format!(
        "{REST}/candles?symbol={}USDT&productType=USDT-FUTURES&granularity={}&limit={}{end}",
        base.to_uppercase(),
        gran(tf)?,
        limit.min(1000)
    )))
}

pub fn parse_candles(body: &str) -> Result<Vec<Bar>> {
    let v = json(body)?;
    ok(&v)?;
    Ok(bars(&v["data"]))
}

pub fn markets_request() -> Request {
    Request::Get(format!("{REST}/tickers?productType=USDT-FUTURES"))
}

pub fn parse_markets(body: &str) -> Result<Vec<Market>> {
    let v = json(body)?;
    ok(&v)?;
    let rows = v["data"].as_array().ok_or_else(|| ArgosError::Feed("Bitget : liste absente".into()))?;
    Ok(rows
        .iter()
        .filter_map(|t| {
            Some(Market {
                base: t["symbol"].as_str()?.strip_suffix("USDT")?.to_string(),
                price: num(&t["lastPr"]),
                prev: num(&t["open24h"]),
            })
        })
        .collect())
}

pub fn instruments_request() -> Request {
    Request::Get(format!("{REST}/contracts?productType=USDT-FUTURES"))
}

pub fn parse_instruments(body: &str) -> Result<Vec<argos_core::Instrument>> {
    let v = json(body)?;
    ok(&v)?;
    let rows = v["data"].as_array().ok_or_else(|| ArgosError::Feed("Bitget : liste absente".into()))?;
    Ok(rows
        .iter()
        .map(|s| Instrument {
            exchange: "bitget".into(),
            symbol: text(&s["symbol"]),
            base: text(&s["baseCoin"]),
            quote: text(&s["quoteCoin"]),
            kind: MarketKind::Perp,
            group: "Crypto".into(),
            price_decimals: text(&s["pricePlace"]).parse().unwrap_or(2),
            size_decimals: text(&s["volumePlace"]).parse().unwrap_or_else(|_| decimals(&s["minTradeNum"])),
            growth: false,
            active: s["symbolStatus"].as_str().is_none_or(|x| x == "normal"),
        })
        .collect())
}

/// Direct des bougies (canal `candle30m`).
pub fn live_sub(base: &str, tf: Timeframe) -> Option<crate::live::LiveSub> {
    Some(crate::live::LiveSub {
        url: WS.into(),
        subscribe: format!(
            r#"{{"op":"subscribe","args":[{{"instType":"USDT-FUTURES","channel":"candle{}","instId":"{}USDT"}}]}}"#,
            gran(tf).ok()?,
            base.to_uppercase()
        ),
        ping: "ping".into(),
        ..Default::default()
    })
}

pub fn parse_live(msg: &str) -> Option<Bar> {
    let v = json(msg).ok()?;
    if !v["arg"]["channel"].as_str()?.starts_with("candle") {
        return None;
    }
    crate::parse::valid(crate::parse::bar_from_array(v["data"].as_array()?.last()?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bitget_bougies_marches_catalogue() {
        let r = candles_request("btc", Timeframe::M30, 3000, Some(1_700_000_000_000)).unwrap();
        assert_eq!(
            r,
            Request::Get(format!(
                "{REST}/candles?symbol=BTCUSDT&productType=USDT-FUTURES&granularity=30m&limit=1000&endTime=1700000000000"
            ))
        );
        assert!(candles_request("BTC", Timeframe::H2, 10, None).is_err());
        let b = parse_candles(r#"{"code":"00000","data":[["1793912400000","76211.2","76213.1","75741.2","75980.8","802.9","60966630.1"]]}"#).unwrap();
        assert_eq!((b[0].time, b[0].open, b[0].close), (1_793_912_400, 76211.2, 75980.8));
        let m = parse_markets(r#"{"code":"00000","data":[{"symbol":"BTCUSDT","lastPr":"75947.3","open24h":"80469"}]}"#).unwrap();
        assert_eq!(m[0].base, "BTC");
        let i = parse_instruments(
            r#"{"code":"00000","data":[{"symbol":"BTCUSDT","baseCoin":"BTC","quoteCoin":"USDT","pricePlace":"1","volumePlace":"4","symbolStatus":"normal"}]}"#,
        )
        .unwrap();
        assert_eq!((i[0].id().as_str(), i[0].price_decimals), ("bitget:BTCUSDT", 1));
        let l = parse_live(
            r#"{"action":"update","arg":{"instType":"USDT-FUTURES","channel":"candle30m","instId":"BTCUSDT"},"data":[["1793912400000","1","2","0.5","1.5","10","0","0"]]}"#,
        );
        assert_eq!(l.map(|b| b.close), Some(1.5));
    }

    /// 🔒 Reliable data: Bitget's own days begin at 00:00 in Asia (16:00 UTC, seen on 23 Nov. 2026): its 12 hours,
    /// days and weeks are asked from 00:00 UTC, the candles and their live channel.
    #[test]
    fn bitget_days_in_utc() {
        for (tf, name) in [(Timeframe::H12, "12Hutc"), (Timeframe::D1, "1Dutc"), (Timeframe::W1, "1Wutc")] {
            let r = candles_request("BTC", tf, 5, None).unwrap();
            assert!(matches!(&r, Request::Get(u) if u.contains(&format!("&granularity={name}&"))), "{r:?}");
            let sub = live_sub("BTC", tf).expect("live").subscribe;
            assert!(sub.contains(&format!(r#""channel":"candle{name}""#)), "{sub}");
        }
        let r = candles_request("BTC", Timeframe::H4, 5, None).unwrap();
        assert!(matches!(&r, Request::Get(u) if u.contains("&granularity=4H&")), "unchanged: {r:?}");
    }
}
