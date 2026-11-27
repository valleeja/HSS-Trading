//! Kraken (spot en dollars, « BTC/USD » ; Kraken écrit XBT pour BTC et XDG pour DOGE). API publique (testée en direct
//! le 2026-11-05, sans clé). Kraken ne rend que les 720 dernières bougies (pas de page plus ancienne).

use super::Request;
use crate::parse::{json, num, text};
use argos_core::feed::Market;
use argos_core::{ArgosError, Bar, Result, Timeframe};
use argos_core::{Instrument, MarketKind};

const REST: &str = "https://api.kraken.com/0/public";
const WS: &str = "wss://ws.kraken.com/v2";

fn minutes(tf: Timeframe) -> Result<i64> {
    Ok(match tf {
        Timeframe::M1 => 1,
        Timeframe::M5 => 5,
        Timeframe::M15 => 15,
        Timeframe::M30 => 30,
        Timeframe::H1 => 60,
        Timeframe::H4 => 240,
        Timeframe::D1 => 1440,
        Timeframe::W1 => 10_080,
        t => return Err(ArgosError::Feed(format!("Kraken : pas de bougies {}", t.id()))),
    })
}

/// Nom Kraken d'un jeton (BTC → XBT, DOGE → XDG) et l'inverse.
pub fn kraken_base(base: &str) -> String {
    match base.to_uppercase().as_str() {
        "BTC" => "XBT".into(),
        "DOGE" => "XDG".into(),
        b => b.into(),
    }
}

pub fn argos_base(k: &str) -> String {
    match k {
        "XBT" => "BTC".into(),
        "XDG" => "DOGE".into(),
        b => b.into(),
    }
}

/// Les 720 dernières bougies (Kraken n'a pas de page plus ancienne : `end_ms` est ignoré).
pub fn candles_request(base: &str, tf: Timeframe, _limit: usize, _end_ms: Option<i64>) -> Result<Request> {
    // ₿ « XRP/BTC » → XRPXBT (Kraken writes XBT); else the US dollar pair
    let (a, q) = crate::paire::parts(base);
    let q = q.map_or("USD".to_string(), kraken_base);
    Ok(Request::Get(format!("{REST}/OHLC?pair={}{q}&interval={}", kraken_base(a), minutes(tf)?)))
}

/// `{"result":{"XXBTZUSD":[[t,o,h,l,c,vwap,volume,n],…],"last":…}}`.
pub fn parse_candles(body: &str) -> Result<Vec<Bar>> {
    let v = json(body)?;
    if let Some(e) = v["error"].as_array().and_then(|a| a.first()) {
        return Err(ArgosError::Feed(format!("Kraken : {}", e.as_str().unwrap_or("erreur"))));
    }
    let rows = v["result"]
        .as_object()
        .and_then(|o| o.iter().find(|(k, _)| *k != "last").map(|(_, r)| r.clone()));
    Ok(rows
        .and_then(|r| r.as_array().cloned())
        .unwrap_or_default()
        .iter()
        .map(|k| Bar {
            time: crate::parse::secs(&k[0]),
            open: num(&k[1]),
            high: num(&k[2]),
            low: num(&k[3]),
            close: num(&k[4]),
            volume: num(&k[6]),
        })
        .collect())
}

pub fn markets_request() -> Request {
    Request::Get(format!("{REST}/Ticker"))
}

/// Jeton d'une clé de paire en dollars (« XXBTZUSD » → BTC, « SOLUSD » → SOL), ₿ ou base d'une paire contre BTC
/// (« XXRPXXBT », « SOLXBT » → « XRP/BTC », « SOL/BTC »), sinon `None`.
fn base_of_pair(key: &str) -> Option<String> {
    if let Some(b) = key.strip_suffix("XXBT").or_else(|| key.strip_suffix("XBT")) {
        let b = match b.strip_prefix('X') {
            Some(x) if b.len() == 4 => x,
            _ => b,
        };
        return (!b.is_empty()).then(|| crate::paire::base(&argos_base(b), "BTC"));
    }
    let b = match key.strip_suffix("ZUSD") {
        Some(b) if b.len() == 4 && b.starts_with('X') => b[1..].to_string(),
        _ => key.strip_suffix("USD")?.to_string(),
    };
    (!b.is_empty()).then(|| argos_base(&b))
}

/// `{"result":{"XXBTZUSD":{"c":["prix",…],"o":"ouverture"},…}}`.
///
/// Kraken gives no price of 24 h ago: its « o » is the price at 00:00 UTC (seen 23 Nov. 2026, on PEPE, which moved
/// that night). The price 24 h ago stays unknown (0): the change is shown as « — », never the change since midnight
/// shown as one of 24 hours (Jee's choice, 24 Nov. 2026).
pub fn parse_markets(body: &str) -> Result<Vec<Market>> {
    let v = json(body)?;
    let rows = v["result"].as_object().ok_or_else(|| ArgosError::Feed("Kraken : liste absente".into()))?;
    let mut out: Vec<Market> = rows
        .iter()
        .filter_map(|(k, t)| {
            Some(Market {
                base: base_of_pair(k)?,
                price: num(&t["c"][0]),
                prev: 0.0,
            })
        })
        .collect();
    // une seule fois par jeton (Kraken a parfois deux clés pour la même paire)
    out.sort_by(|a, b| a.base.cmp(&b.base));
    out.dedup_by(|a, b| a.base == b.base);
    Ok(out)
}

pub fn instruments_request() -> Request {
    Request::Get(format!("{REST}/AssetPairs"))
}

pub fn parse_instruments(body: &str) -> Result<Vec<argos_core::Instrument>> {
    let v = json(body)?;
    let rows = v["result"].as_object().ok_or_else(|| ArgosError::Feed("Kraken : liste absente".into()))?;
    let mut out: Vec<Instrument> = rows
        .values()
        .filter_map(|p| {
            let ws = p["wsname"].as_str()?;
            // the US dollar pairs, and ₿ the BTC pairs (« XRP/XBT » → XRP/BTC)
            let (k, quote) = match ws.strip_suffix("/USD") {
                Some(k) => (k, "USD"),
                None => (ws.strip_suffix("/XBT")?, "BTC"),
            };
            let base = argos_base(k);
            Some(Instrument {
                exchange: "kraken".into(),
                symbol: format!("{base}/{quote}"),
                base,
                quote: quote.into(),
                kind: MarketKind::Spot,
                group: "Crypto".into(),
                price_decimals: p["pair_decimals"].as_u64().unwrap_or(2) as u32,
                size_decimals: p["lot_decimals"].as_u64().unwrap_or(8) as u32,
                growth: false,
                active: p["status"].as_str().is_none_or(|s| s == "online"),
            })
        })
        .collect();
    out.sort_by(|a, b| a.symbol.cmp(&b.symbol));
    out.dedup_by(|a, b| a.symbol == b.symbol);
    Ok(out)
}

/// Direct des bougies (canal `ohlc` v2).
pub fn live_sub(base: &str, tf: Timeframe) -> Option<crate::live::LiveSub> {
    Some(crate::live::LiveSub {
        url: WS.into(),
        subscribe: format!(
            r#"{{"method":"subscribe","params":{{"channel":"ohlc","symbol":["{}"],"interval":{}}}}}"#,
            crate::paire::symbole(base, "/", "USD"),
            minutes(tf).ok()?
        ),
        ping: r#"{"method":"ping"}"#.into(),
        ..Default::default()
    })
}

/// `{"channel":"ohlc","data":[{"open","high","low","close","volume","interval_begin":"ISO"}]}`.
pub fn parse_live(msg: &str) -> Option<Bar> {
    let v = json(msg).ok()?;
    if v["channel"] != "ohlc" {
        return None;
    }
    let d = v["data"].as_array()?.last()?;
    let t = chrono::DateTime::parse_from_rfc3339(&text(&d["interval_begin"])).ok()?.timestamp();
    crate::parse::valid(Bar {
        time: t,
        open: num(&d["open"]),
        high: num(&d["high"]),
        low: num(&d["low"]),
        close: num(&d["close"]),
        volume: num(&d["volume"]),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kraken_xbt_et_bougies() {
        assert_eq!(
            candles_request("btc", Timeframe::M30, 1, None).unwrap(),
            Request::Get(format!("{REST}/OHLC?pair=XBTUSD&interval=30"))
        );
        let b = parse_candles(r#"{"error":[],"result":{"XXBTZUSD":[[1793901600,"75836.6","76141.2","75583.9","76058.1","75861.5","162.4",5935]],"last":1}}"#)
            .unwrap();
        assert_eq!((b[0].open, b[0].close, b[0].volume), (75836.6, 76058.1, 162.4));
        let m = parse_markets(
            r#"{"error":[],"result":{"XXBTZUSD":{"c":["110","1"],"o":"100"},"SOLUSD":{"c":["2","1"],"o":"1"},"XETHZEUR":{"c":["1","1"],"o":"1"}}}"#,
        )
        .unwrap();
        assert_eq!(
            m.iter().map(|x| x.base.as_str()).collect::<Vec<_>>(),
            ["BTC", "SOL"],
            "XBT = BTC, une fois chacun, dollars seulement"
        );
        assert_eq!((m[0].price, m[0].prev), (110.0, 0.0), "« o » is midnight UTC, not 24 hours before");
        assert_eq!(m[0].change_pct(), None, "the change of Kraken is « — »");
        let i = parse_instruments(
            r#"{"error":[],"result":{"XXBTZUSD":{"wsname":"XBT/USD","pair_decimals":1,"lot_decimals":8,"status":"online"},"XBTUSD.d":{"wsname":"XBT/USD"}}}"#,
        )
        .unwrap();
        assert_eq!(i.len(), 1, "pas de doublon");
        assert_eq!(i[0].id(), "kraken:BTC/USD");
        let l = parse_live(
            r#"{"channel":"ohlc","type":"update","data":[{"symbol":"BTC/USD","open":1,"high":2,"low":0.5,"close":1.5,"volume":3,"interval_begin":"2026-11-05T20:00:00.000000000Z","interval":30}]}"#,
        );
        assert_eq!(l.map(|b| b.close), Some(1.5));
    }

    /// ₿ Kraken writes XBT for BTC: « XRP/XBT » is shown as XRP/BTC, its candles ask for XRPXBT.
    #[test]
    fn kraken_xbt_devient_btc() {
        assert_eq!(
            candles_request("XRP/BTC", Timeframe::H4, 1, None).unwrap(),
            Request::Get(format!("{REST}/OHLC?pair=XRPXBT&interval=240"))
        );
        let m = parse_markets(
            r#"{"error":[],"result":{"XXRPXXBT":{"c":["0.0000171","1"],"o":"0.000017"},"SOLXBT":{"c":["0.002","1"],"o":"0.002"},"XXBTZUSD":{"c":["110","1"],"o":"100"}}}"#,
        )
        .unwrap();
        assert_eq!(m.iter().map(|x| x.base.as_str()).collect::<Vec<_>>(), ["BTC", "SOL/BTC", "XRP/BTC"]);
        let i = parse_instruments(
            r#"{"error":[],"result":{"XXRPXXBT":{"wsname":"XRP/XBT","pair_decimals":8,"lot_decimals":8,"status":"online"},"XXBTZUSD":{"wsname":"XBT/USD","pair_decimals":1}}}"#,
        )
        .unwrap();
        let ids: Vec<String> = i.iter().map(|x| x.id()).collect();
        assert_eq!(ids, ["kraken:BTC/USD", "kraken:XRP/BTC"]);
        assert_eq!((i[1].base.as_str(), i[1].quote.as_str(), i[1].price_decimals), ("XRP", "BTC", 8));
        assert!(live_sub("XRP/BTC", Timeframe::H1).unwrap().subscribe.contains(r#"["XRP/BTC"]"#));
        assert!(
            live_sub("BTC", Timeframe::H1).unwrap().subscribe.contains(r#"["BTC/USD"]"#),
            "dollars unchanged"
        );
    }
}
