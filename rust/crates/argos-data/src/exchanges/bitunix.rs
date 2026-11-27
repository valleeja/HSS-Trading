//! Bitunix (perps USDT, symbole « BTCUSDT »). API publique futures : https://fapi.bitunix.com/api/v1/futures/market
//! Bitunix bloque les navigateurs (CORS) : sur le web, relais Argos (/api/exchange) ; sur desktop, en direct.

use super::Request;
use crate::parse::{bars, field, json, num, ok_code};
use crate::parse::{decimals, text};
use argos_core::feed::Market;
use argos_core::{ArgosError, Bar, Result, Timeframe};
use argos_core::{Instrument, MarketKind};

const REST: &str = "https://fapi.bitunix.com/api/v1/futures/market";

/// `end_ms` : seulement les bougies d'avant ce moment (ms), pour paginer vers le passé ; `None` = les plus récentes.
pub fn candles_request(base: &str, tf: Timeframe, limit: usize, end_ms: Option<i64>) -> Result<Request> {
    let end = end_ms.map_or(String::new(), |e| format!("&endTime={}", e));
    Ok(Request::Get(format!(
        "{REST}/kline?symbol={base}USDT&interval={}&limit={}{end}",
        tf.id(),
        limit.min(200)
    )))
}

pub fn parse_candles(body: &str) -> Result<Vec<Bar>> {
    let v = json(body)?;
    ok_code(&v, "code", "Bitunix")?;
    Ok(bars(&v["data"]).into_iter().map(open_in_candle).collect())
}

/// 🕯 Bitunix opens each candle at the close of the one before (every candle of its answers recorded on 23 Nov. 2026,
/// history and live), but its high and low hold only the trades of the candle: after a jump of the price between two
/// candles, the open falls outside the candle and the app's check refuses it (`argos_core::feed::clean_bars`), and a
/// whole answer with too many of them. Jee's choice (23 Nov. 2026): the high and the low stay Bitunix's, the open
/// goes to the nearest edge of the candle. Only an open no further from the candle than [`GAP_MAX`], in a candle whose
/// low is not above its high: anything else (an open that cannot be read, at zero or below, too far; a low above the
/// high, or one that cannot be read) is not Bitunix's way of writing a candle, stays as it came, and is refused as
/// before. (A number that cannot be read holds no comparison: it never passes.)
fn open_in_candle(mut b: Bar) -> Bar {
    if b.low <= b.high {
        let edge = b.open.clamp(b.low, b.high);
        if (b.open - edge).abs() <= edge * GAP_MAX {
            b.open = edge;
        }
    }
    b
}

/// 🕯 The furthest an open may be from its candle and still be the close of the candle before (a tenth of the price;
/// on 23 Nov. 2026 the gaps were of one step of price).
const GAP_MAX: f64 = 0.1;

pub fn markets_request() -> Request {
    Request::Get(format!("{REST}/tickers"))
}

pub fn parse_markets(body: &str) -> Result<Vec<Market>> {
    let v = json(body)?;
    ok_code(&v, "code", "Bitunix")?;
    let rows = v["data"].as_array().ok_or_else(|| ArgosError::Feed("Bitunix : liste absente".into()))?;
    Ok(rows
        .iter()
        .filter_map(|t| {
            let base = t["symbol"].as_str()?.strip_suffix("USDT")?;
            Some(Market {
                base: base.to_string(),
                price: num(field(t, &["lastPrice", "last"])),
                prev: num(field(t, &["open", "openPrice"])),
            })
        })
        .collect())
}

/// Catalogue : les perps (`trading_pairs`).
pub fn instruments_request() -> Request {
    Request::Get(format!("{REST}/trading_pairs"))
}

pub fn parse_instruments(body: &str) -> Result<Vec<Instrument>> {
    let v = json(body)?;
    ok_code(&v, "code", "Bitunix")?;
    let rows = v["data"].as_array().ok_or_else(|| ArgosError::Feed("Bitunix : liste absente".into()))?;
    Ok(rows
        .iter()
        .map(|s| Instrument {
            exchange: "bitunix".into(),
            symbol: text(&s["symbol"]),
            base: text(&s["base"]),
            quote: text(&s["quote"]),
            kind: MarketKind::Perp,
            group: "Crypto".into(),
            price_decimals: decimals(&s["quotePrecision"]),
            size_decimals: decimals(&s["basePrecision"]),
            growth: false,
            active: s["symbolStatus"].as_str().is_none_or(|st| st == "OPEN"),
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requete_et_lecture() {
        assert_eq!(
            candles_request("BTC", Timeframe::M5, 600, None).unwrap(),
            Request::Get("https://fapi.bitunix.com/api/v1/futures/market/kline?symbol=BTCUSDT&interval=5m&limit=200".into())
        );
        let b = parse_candles(r#"{"code":0,"data":[{"time":1700000000000,"open":"1","high":"2","low":"0.5","close":"1.5","baseVol":"10","quoteVol":"15"}]}"#)
            .unwrap();
        assert_eq!(b[0].volume, 10.0);
        assert!(parse_candles(r#"{"code":2,"msg":"System error","data":null}"#).is_err());
        let m = parse_markets(r#"{"code":0,"data":[{"symbol":"ETHUSDT","lastPrice":"2300","open":"2350"}]}"#).unwrap();
        assert_eq!(
            m[0],
            Market {
                base: "ETH".into(),
                price: 2300.0,
                prev: 2350.0
            }
        );
    }

    fn ohlc(b: &Bar) -> [f64; 4] {
        [b.open, b.high, b.low, b.close]
    }

    /// 🕯 Two candles of five minutes as Bitunix gave them on 23 Nov. 2026 (newest first): the older one opens at the
    /// close of the one before it (61189.2), above its own high (61188.1). Its open goes to its high, nothing else
    /// moves, and the answer is kept whole (it was refused: one candle in twelve is more than 5 %).
    #[test]
    fn l_ouverture_d_une_bougie_a_trou_est_ramenee_dans_la_bougie() {
        let body = r#"{"code":0,"data":[{"open":"61070.1","high":"61269.3","low":"61038.1","close":"61210.4","quoteVol":"6118047.42106","baseVol":"99.9873","time":"1795467300000"},{"open":"61189.2","high":"61188.1","low":"61046","close":"61070.1","quoteVol":"5029550.87308","baseVol":"82.3347","time":"1795467000000"}]}"#;
        let (bars, refused) = crate::Exchange::Bitunix.parse_candles_checked(body).expect("kept");
        assert!(refused.is_empty(), "{refused:?}");
        let got: Vec<[f64; 4]> = bars.iter().map(ohlc).collect();
        assert_eq!(got, [[61188.1, 61188.1, 61046.0, 61070.1], [61070.1, 61269.3, 61038.1, 61210.4]]);
        assert_eq!(bars.iter().map(|b| b.volume).collect::<Vec<_>>(), [82.3347, 99.9873]);
        // below its low: to its low (DOGE, one minute, the same day)
        let doge = r#"{"code":0,"data":[{"open":"0.05655","high":"0.05663","low":"0.05656","close":"0.05663","quoteVol":"2993.66022","baseVol":"52905","time":"1795467420000"}]}"#;
        assert_eq!(ohlc(&parse_candles(doge).expect("read")[0]), [0.05656, 0.05663, 0.05656, 0.05663]);
        // the live stream: the same
        let live = r#"{"ch":"market_kline_1min","symbol":"BTCUSDT","ts":1795470428856,"data":{"o":"61100","c":"61087.6","h":"61087.6","l":"61048","b":"10.4136","q":"635839.58183"}}"#;
        let b = parse_live(live).expect("a candle");
        assert_eq!((b.time, ohlc(&b)), (1_795_470_420, [61087.6, 61087.6, 61048.0, 61087.6]));
        assert_eq!(
            ohlc(&parse_live(&live.replace("61100", "61050.1")).expect("a candle"))[0],
            61050.1,
            "inside: unchanged"
        );
    }

    /// 🕯 What is not Bitunix's way of writing a candle stays as it came, and is refused as before: an open that
    /// cannot be read, at zero or below, too far from its candle, or a low above the high.
    #[test]
    fn une_bougie_illisible_reste_refusee() {
        let candle = |o: &str, h: &str, l: &str| {
            format!(r#"{{"code":0,"data":[{{"open":"{o}","high":"{h}","low":"{l}","close":"100","baseVol":"1","time":"1700000000000"}}]}}"#)
        };
        for (o, h, l) in [
            ("x", "101", "99"),
            ("0", "101", "99"),
            ("-100", "101", "99"),
            ("112", "101", "99"),
            ("89", "101", "99"),
            ("100", "99", "101"),
        ] {
            let body = candle(o, h, l);
            let b = parse_candles(&body).expect("read");
            assert!(b[0].open.is_nan() || b[0].open.to_string() == o, "{o} {h} {l}: {:?}", ohlc(&b[0]));
            assert!(
                crate::Exchange::Bitunix.parse_candles_checked(&body).is_err(),
                "{o} {h} {l}: refused, as before"
            );
        }
        // a tenth of the price away is still the close of the candle before
        assert_eq!(parse_candles(&candle("111", "101", "99")).expect("read")[0].open, 101.0);
        let live = r#"{"ch":"market_kline_1min","ts":1700000000000,"data":{"o":"0","c":"100","h":"101","l":"99","b":"1"}}"#;
        assert_eq!(parse_live(live).map(|b| b.open), Some(0.0), "the live candle keeps it too");
    }
}

#[cfg(test)]
mod catalog_tests {
    use super::*;

    #[test]
    fn catalogue() {
        let body = r#"{"code":0,"data":[{"symbol":"BTCUSDT","base":"BTC","quote":"USDT","basePrecision":4,"quotePrecision":1,"symbolStatus":"OPEN"},{"symbol":"OLDUSDT","base":"OLD","quote":"USDT","basePrecision":0,"quotePrecision":4,"symbolStatus":"STOP"}]}"#;
        let l = parse_instruments(body).unwrap();
        assert_eq!(
            (l[0].id().as_str(), l[0].price_decimals, l[0].size_decimals, l[0].active),
            ("bitunix:BTCUSDT", 1, 4, true)
        );
        assert!(!l[1].active);
    }
}

const WS: &str = "wss://fapi.bitunix.com/public/";

/// Nom du canal de bougies Bitunix (`market_kline_1min`, `…_60min`, `…_1day`…).
const CHANNELS: [(Timeframe, &str); 11] = [
    (Timeframe::M1, "1min"),
    (Timeframe::M3, "3min"),
    (Timeframe::M5, "5min"),
    (Timeframe::M15, "15min"),
    (Timeframe::M30, "30min"),
    (Timeframe::H1, "60min"),
    (Timeframe::H2, "2h"),
    (Timeframe::H4, "4h"),
    (Timeframe::H12, "12h"),
    (Timeframe::D1, "1day"),
    (Timeframe::W1, "1week"),
];

/// Direct : `market_kline_<unité>` ; `{"op":"ping"}` garde le lien ouvert.
pub fn live_sub(base: &str, tf: Timeframe) -> crate::live::LiveSub {
    let ch = CHANNELS.iter().find(|c| c.0 == tf).map_or("1min", |c| c.1);
    crate::live::LiveSub {
        url: WS.into(),
        subscribe: format!(
            r#"{{"op":"subscribe","args":[{{"symbol":"{}USDT","ch":"market_kline_{ch}"}}]}}"#,
            base.to_uppercase()
        ),
        ping: r#"{"op":"ping","ping":0}"#.into(),
        // 💤 nothing comes back to this ping (`LiveSub::ping_unanswered`)
        ping_unanswered: true,
        ..Default::default()
    }
}

/// Bougie d'un message du direct. Bitunix n'écrit pas l'heure de la bougie : c'est l'heure du message (`ts`)
/// ramenée au début de sa bougie (semaine : le lundi 00:00 UTC). Volume : `b` (en BTC).
pub fn parse_live(msg: &str) -> Option<Bar> {
    let v = json(msg).ok()?;
    let ch = v["ch"].as_str()?.strip_prefix("market_kline_")?;
    let tf = CHANNELS.iter().find(|c| c.1 == ch)?.0;
    let secs = tf.seconds();
    // l'heure Unix 0 tombe un jeudi : les semaines partent du lundi (4 jours plus tard)
    let shift = if tf == Timeframe::W1 { 4 * 86_400 } else { 0 };
    let t = v["ts"].as_i64()? / 1000;
    let d = &v["data"];
    // (the same open as the history: `open_in_candle`)
    crate::parse::valid(open_in_candle(Bar {
        time: (t - shift).div_euclid(secs) * secs + shift,
        open: num(&d["o"]),
        high: num(&d["h"]),
        low: num(&d["l"]),
        close: num(&d["c"]),
        volume: num(&d["b"]),
    }))
}
