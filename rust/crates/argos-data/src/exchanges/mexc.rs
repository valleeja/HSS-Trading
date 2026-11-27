//! MEXC (perps USDT, « BTC_USDT »). API publique des contrats (testée en direct le 2026-11-05, sans clé ; le
//! navigateur est refusé : passe par le relais).

use super::Request;
use crate::parse::{decimals, json, num, text};
use argos_core::feed::Market;
use argos_core::{ArgosError, Bar, Result, Timeframe};
use argos_core::{Instrument, MarketKind};

const REST: &str = "https://contract.mexc.com/api/v1/contract";
const WS: &str = "wss://contract.mexc.com/edge";

fn int(tf: Timeframe) -> Result<&'static str> {
    Ok(match tf {
        Timeframe::M1 => "Min1",
        Timeframe::M5 => "Min5",
        Timeframe::M15 => "Min15",
        Timeframe::M30 => "Min30",
        Timeframe::H1 => "Min60",
        Timeframe::H4 => "Hour4",
        Timeframe::D1 => "Day1",
        Timeframe::W1 => "Week1",
        t => return Err(ArgosError::Feed(format!("MEXC : pas de bougies {}", t.id()))),
    })
}

/// `end_ms` : seulement les bougies d'avant ce moment (ms) ; `None` = les plus récentes (MEXC rend 2000 bougies au plus).
pub fn candles_request(base: &str, tf: Timeframe, limit: usize, end_ms: Option<i64>) -> Result<Request> {
    let n = limit.min(2000) as i64;
    let range = match end_ms {
        Some(e) => {
            let end = e / 1000 - 1;
            format!("&start={}&end={end}", end - tf.seconds() * n)
        }
        None => String::new(),
    };
    Ok(Request::Get(format!("{REST}/kline/{}_USDT?interval={}{range}", base.to_uppercase(), int(tf)?)))
}

/// Bougies en colonnes : `{"data":{"time":[…],"open":[…],"high":[…],"low":[…],"close":[…],"vol":[…]}}`.
pub fn parse_candles(body: &str) -> Result<Vec<Bar>> {
    let v = json(body)?;
    if v["success"] != true {
        return Err(ArgosError::Feed(format!("MEXC : {}", v["message"].as_str().unwrap_or("erreur"))));
    }
    let d = &v["data"];
    let col = |k: &str| d[k].as_array().cloned().unwrap_or_default();
    let (t, o, h, l, c, vol) = (col("time"), col("open"), col("high"), col("low"), col("close"), col("vol"));
    Ok((0..t.len())
        .map(|i| Bar {
            time: crate::parse::secs(&t[i]),
            open: o.get(i).map_or(f64::NAN, num),
            high: h.get(i).map_or(f64::NAN, num),
            low: l.get(i).map_or(f64::NAN, num),
            close: c.get(i).map_or(f64::NAN, num),
            volume: vol.get(i).map_or(0.0, num),
        })
        .collect())
}

pub fn markets_request() -> Request {
    Request::Get(format!("{REST}/ticker"))
}

pub fn parse_markets(body: &str) -> Result<Vec<Market>> {
    let v = json(body)?;
    let rows = v["data"].as_array().ok_or_else(|| ArgosError::Feed("MEXC : liste absente".into()))?;
    Ok(rows
        .iter()
        .filter_map(|t| {
            let price = num(&t["lastPrice"]);
            // the change over 24 hours is the first of `riseFallRatesOfTimezone`; `riseFallRate` (= its third) counts
            // from 00:00 in Asia (16:00 UTC), the second from 00:00 UTC (seen on 23 Nov. 2026). Without it: the price
            // 24 h ago is not known (0, shown « — »), never a change of 0 % made up
            let rate = num(&t["riseFallRatesOfTimezone"][0]);
            Some(Market {
                base: t["symbol"].as_str()?.strip_suffix("_USDT")?.to_string(),
                price,
                prev: if rate.is_finite() && rate > -1.0 { price / (1.0 + rate) } else { 0.0 },
            })
        })
        .collect())
}

pub fn instruments_request() -> Request {
    Request::Get(format!("{REST}/detail"))
}

pub fn parse_instruments(body: &str) -> Result<Vec<argos_core::Instrument>> {
    let v = json(body)?;
    let rows = v["data"].as_array().ok_or_else(|| ArgosError::Feed("MEXC : liste absente".into()))?;
    Ok(rows
        .iter()
        .filter(|s| s["quoteCoin"] == "USDT")
        .map(|s| Instrument {
            exchange: "mexc".into(),
            symbol: text(&s["symbol"]),
            base: text(&s["baseCoin"]),
            quote: "USDT".into(),
            kind: MarketKind::Perp,
            group: "Crypto".into(),
            price_decimals: decimals(&s["priceUnit"]),
            size_decimals: 0,
            growth: false,
            active: s["state"] == 0,
        })
        .collect())
}

/// 📏 The size of one contract of `base` (its perp counts candles and trades in contracts).
pub fn contract_request(base: &str) -> Request {
    Request::Get(format!("{REST}/detail?symbol={}_USDT", base.to_uppercase()))
}

/// The coins in one contract: `{"success":true,"data":{"symbol":"BTC_USDT","contractSize":0.0001,…}}`.
pub fn parse_contract(body: &str) -> Result<f64> {
    let v = json(body)?;
    let k = num(&v["data"]["contractSize"]);
    if v["success"] == true && k.is_finite() && k > 0.0 {
        Ok(k)
    } else {
        Err(ArgosError::Feed(format!(
            "MEXC : {}",
            v["message"].as_str().unwrap_or("taille du contrat absente")
        )))
    }
}

/// Direct des bougies (`sub.kline`).
pub fn live_sub(base: &str, tf: Timeframe) -> Option<crate::live::LiveSub> {
    Some(crate::live::LiveSub {
        url: WS.into(),
        subscribe: format!(
            r#"{{"method":"sub.kline","param":{{"symbol":"{}_USDT","interval":"{}"}}}}"#,
            base.to_uppercase(),
            int(tf).ok()?
        ),
        ping: r#"{"method":"ping"}"#.into(),
        ..Default::default()
    })
}

/// `{"symbol":"BTC_USDT","data":{"t":…,"o":…,"h":…,"l":…,"c":…,"a":…,"q":…},"channel":"push.kline"}`: `q` counts
/// contracts (`a`: dollars), given in coins by the size of the contract (read with the candles, `ExchangeFeed`);
/// unknown size: the message is left out.
pub fn parse_live(msg: &str) -> Option<Bar> {
    let v = json(msg).ok()?;
    if v["channel"] != "push.kline" {
        return None;
    }
    let d = &v["data"];
    let symbol = d["symbol"].as_str().or(v["symbol"].as_str())?;
    let k = crate::contracts::contract("MEXC", symbol.strip_suffix("_USDT")?)?;
    crate::parse::valid(Bar {
        time: crate::parse::secs(&d["t"]),
        open: num(&d["o"]),
        high: num(&d["h"]),
        low: num(&d["l"]),
        close: num(&d["c"]),
        volume: num(&d["q"]) * k,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mexc_bougies_en_colonnes() {
        assert_eq!(
            candles_request("btc", Timeframe::M30, 5, None).unwrap(),
            Request::Get(format!("{REST}/kline/BTC_USDT?interval=Min30"))
        );
        let b = parse_candles(r#"{"success":true,"code":0,"data":{"time":[1793901600,1793903400],"open":[75920.2,76140.0],"close":[76140.0,76066.7],"high":[76222.2,76248.4],"low":[75669.4,75950.0],"vol":[1.7e7,8.3e6]}}"#).unwrap();
        assert_eq!(b.len(), 2);
        assert_eq!((b[1].time, b[1].open, b[1].close), (1_793_903_400, 76140.0, 76066.7));
        let m = parse_markets(r#"{"data":[{"symbol":"BTC_USDT","lastPrice":110,"riseFallRate":0.5,"riseFallRatesOfTimezone":[0.1,0.3,0.5]}]}"#).unwrap();
        assert!((m[0].prev - 100.0).abs() < 1e-9, "24 hours: the first rate");
        let l = parse_live(r#"{"channel":"push.kline","data":{"t":1793901600,"o":1,"h":2,"l":0.5,"c":1.5,"q":3},"symbol":"BTC_USDT"}"#);
        assert_eq!(l.map(|b| b.close), Some(1.5));
    }

    /// 🔒 Reliable data: MEXC's real answer of 23 Nov. 2026. Its « riseFallRate » counts from 16:00 UTC (−2.77 % for
    /// BTC that evening): the change shown is the one of 24 hours (+0.38 %), the first of « riseFallRatesOfTimezone ».
    #[test]
    fn mexc_change_over_24_hours() {
        let body = r#"{"success":true,"code":0,"data":[{"contractId":10,"symbol":"BTC_USDT","lastPrice":61123.2,"bid1":61119.3,"ask1":61119.4,"volume24":529300684,"amount24":3280148314.05792,"riseFallRate":-0.0277,"riseFallValue":-1744,"timestamp":1795470380905,"riseFallRates":{"zone":"UTC+8","r":-0.0277,"v":-1744},"riseFallRatesOfTimezone":[0.0038,-0.0018,-0.0277]}]}"#;
        let m = parse_markets(body).unwrap();
        let change = (m[0].price / m[0].prev - 1.0) * 100.0;
        assert!((change - 0.38).abs() < 1e-9, "+0.38 % over 24 hours, not −2.77 %: {change}");
        // an answer without the 24 hours: the change is not known (« — »), never the one from 16:00 UTC
        let m = parse_markets(r#"{"data":[{"symbol":"BTC_USDT","lastPrice":110,"riseFallRate":0.1}]}"#).unwrap();
        assert_eq!((m[0].prev, m[0].change_pct()), (0.0, None));
    }

    /// 🔒 Reliable data: MEXC counts its volumes in contracts (BTC: 0.0001 BTC each); a candle gives them in coins,
    /// with the size of the contract read from MEXC (real answers of 23 Nov. 2026: 39 770 contracts = 3.977 BTC,
    /// 242 838 dollars at 61 070). A coin whose contract size is not known yet gives no live candle.
    #[test]
    fn mexc_volumes_in_coins() {
        assert_eq!(contract_request("doge"), Request::Get(format!("{REST}/detail?symbol=DOGE_USDT")));
        let detail = r#"{"success":true,"code":0,"data":{"symbol":"ZZMEXC_USDT","displayName":"ZZMEXC_USDT","baseCoin":"ZZMEXC","quoteCoin":"USDT","contractSize":0.0001,"minVol":1,"maxVol":1250000}}"#;
        assert_eq!(parse_contract(detail).unwrap(), 0.0001);
        assert!(parse_contract(r#"{"success":false,"code":1001,"message":"contract not exists"}"#).is_err());
        assert!(parse_contract(r#"{"success":true,"code":0,"data":{"symbol":"ZZMEXC_USDT","contractSize":0}}"#).is_err());
        let msg = r#"{"symbol":"BTC_USDT","data":{"symbol":"BTC_USDT","interval":"Min1","t":1795470420,"o":61050,"c":61087.6,"h":61087.6,"l":61048,"a":242838.30713,"q":39770,"ro":61050.1,"rc":61087.6,"rh":61087.6,"rl":61048},"channel":"push.kline","ts":1795470428547}"#;
        let b = parse_live(msg).unwrap();
        assert!((b.volume - 3.977).abs() < 1e-9, "{}", b.volume);
        let unknown = msg.replace("BTC_USDT", "ZZNOSIZE_USDT");
        assert_eq!(parse_live(&unknown), None, "no size: no candle");
        crate::contracts::set_contracts("MEXC", vec![("ZZMEXCLIVE".into(), 10.0)]);
        let b = parse_live(&msg.replace("BTC_USDT", "ZZMEXCLIVE_USDT")).unwrap();
        assert!((b.volume - 397_700.0).abs() < 1e-6);
    }
}
