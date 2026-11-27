//! Gate.io (perps USDT, contrat « BTC_USDT »). API publique v4 (testée en direct le 2026-11-05, sans clé).

use super::Request;
use crate::parse::{bars, decimals, json, num};
use argos_core::feed::Market;
use argos_core::{ArgosError, Bar, Result, Timeframe};
use argos_core::{Instrument, MarketKind};

const REST: &str = "https://api.gateio.ws/api/v4/futures/usdt";
const WS: &str = "wss://fx-ws.gateio.ws/v4/ws/usdt";

fn int(tf: Timeframe) -> Result<&'static str> {
    Ok(match tf {
        Timeframe::M1 => "1m",
        Timeframe::M5 => "5m",
        Timeframe::M15 => "15m",
        Timeframe::M30 => "30m",
        Timeframe::H1 => "1h",
        Timeframe::H4 => "4h",
        Timeframe::D1 => "1d",
        Timeframe::W1 => "7d",
        t => return Err(ArgosError::Feed(format!("Gate.io : pas de bougies {}", t.id()))),
    })
}

/// `end_ms` : seulement les bougies d'avant ce moment (ms) ; `None` = les plus récentes.
pub fn candles_request(base: &str, tf: Timeframe, limit: usize, end_ms: Option<i64>) -> Result<Request> {
    let n = limit.min(1999);
    let end = match end_ms {
        // avec une fin, Gate demande aussi un début
        Some(e) => {
            let to = e / 1000 - 1;
            format!("&from={}&to={to}", to - tf.seconds() * n as i64)
        }
        None => format!("&limit={n}"),
    };
    Ok(Request::Get(format!(
        "{REST}/candlesticks?contract={}_USDT&interval={}{end}",
        base.to_uppercase(),
        int(tf)?
    )))
}

pub fn parse_candles(body: &str) -> Result<Vec<Bar>> {
    let v = json(body)?;
    if v.is_object() {
        return Err(ArgosError::Feed(format!("Gate.io : {}", v["message"].as_str().unwrap_or("erreur"))));
    }
    Ok(bars(&v))
}

pub fn markets_request() -> Request {
    Request::Get(format!("{REST}/tickers"))
}

pub fn parse_markets(body: &str) -> Result<Vec<Market>> {
    let v = json(body)?;
    let rows = v.as_array().ok_or_else(|| ArgosError::Feed("Gate.io : liste absente".into()))?;
    Ok(rows
        .iter()
        .filter_map(|t| {
            let price = num(&t["last"]);
            let pct = num(&t["change_percentage"]);
            Some(Market {
                base: t["contract"].as_str()?.strip_suffix("_USDT")?.to_string(),
                price,
                // without the change: the price 24 h ago is not known (0, shown « — »), not a 0 % made up
                prev: if pct.is_finite() && pct > -100.0 { price / (1.0 + pct / 100.0) } else { 0.0 },
            })
        })
        .collect())
}

pub fn instruments_request() -> Request {
    Request::Get(format!("{REST}/contracts"))
}

pub fn parse_instruments(body: &str) -> Result<Vec<argos_core::Instrument>> {
    let v = json(body)?;
    let rows = v.as_array().ok_or_else(|| ArgosError::Feed("Gate.io : liste absente".into()))?;
    Ok(rows
        .iter()
        .filter_map(|s| {
            let name = s["name"].as_str()?;
            let base = name.strip_suffix("_USDT")?;
            Some(Instrument {
                exchange: "gate".into(),
                symbol: name.to_string(),
                base: base.to_string(),
                quote: "USDT".into(),
                kind: MarketKind::Perp,
                group: "Crypto".into(),
                price_decimals: decimals(&s["order_price_round"]),
                size_decimals: 0,
                growth: false,
                active: s["in_delisting"] != true,
            })
        })
        .collect())
}

/// 📏 The size of one contract of `base` (its perp counts candles and trades in contracts).
pub fn contract_request(base: &str) -> Request {
    Request::Get(format!("{REST}/contracts/{}_USDT", base.to_uppercase()))
}

/// The coins in one contract: `{"name":"BTC_USDT","quanto_multiplier":"0.0001",…}`.
pub fn parse_contract(body: &str) -> Result<f64> {
    let v = json(body)?;
    let k = num(&v["quanto_multiplier"]);
    if k.is_finite() && k > 0.0 {
        Ok(k)
    } else {
        Err(ArgosError::Feed(format!(
            "Gate.io : {}",
            v["message"].as_str().unwrap_or("taille du contrat absente")
        )))
    }
}

/// Direct des bougies (`futures.candlesticks`).
pub fn live_sub(base: &str, tf: Timeframe) -> Option<crate::live::LiveSub> {
    Some(crate::live::LiveSub {
        url: WS.into(),
        subscribe: format!(
            r#"{{"time":0,"channel":"futures.candlesticks","event":"subscribe","payload":["{}","{}_USDT"]}}"#,
            int(tf).ok()?,
            base.to_uppercase()
        ),
        ping: r#"{"time":0,"channel":"futures.ping"}"#.into(),
        ..Default::default()
    })
}

/// `{"channel":"futures.candlesticks","event":"update","result":[{"t":…,"o":…,"h":…,"l":…,"c":…,"v":…,"a":…,
/// "n":"1m_BTC_USDT"}]}`: `v` counts contracts (`a`: dollars), given in coins by the size of the contract (read with
/// the candles, `ExchangeFeed`); unknown size: the message is left out.
pub fn parse_live(msg: &str) -> Option<Bar> {
    let v = json(msg).ok()?;
    if v["channel"] != "futures.candlesticks" || v["event"] != "update" {
        return None;
    }
    let last = v["result"].as_array()?.last()?;
    let (_, contract) = last["n"].as_str()?.split_once('_')?;
    let k = crate::contracts::contract("Gate.io", contract.strip_suffix("_USDT")?)?;
    let b = crate::parse::bar_from_object(last);
    crate::parse::valid(Bar {
        volume: num(&last["v"]) * k,
        ..b
    })
}

/// 🧮 Trades des perps de chaque pièce (`futures.trades`), sur une connexion : pour l'orderflow réel.
pub fn trades_sub(coins: &[String]) -> crate::live::LiveSub {
    let contrats: Vec<String> = coins.iter().map(|c| format!(r#""{}_USDT""#, c.to_uppercase())).collect();
    crate::live::LiveSub {
        url: WS.into(),
        subscribe: format!(
            r#"{{"time":0,"channel":"futures.trades","event":"subscribe","payload":[{}]}}"#,
            contrats.join(",")
        ),
        ping: r#"{"time":0,"channel":"futures.ping"}"#.into(),
        ..Default::default()
    }
}

/// 🔎 Trades d'un message du direct : `{"channel":"futures.trades","event":"update","result":[{"id":…,"size":-4,
/// "create_time_ms":…,"price":"…","contract":"BTC_USDT"}]}`. Relevé sur le flux public le 2026-11-15 : `size` > 0 = le
/// preneur achète, < 0 = il vend (avec le carnet `futures.book_ticker` juste avant : 70 achats sur 82 au prix vendeur,
/// 30 ventes sur 30 au prix acheteur) ; `size` en contrats (BTC : 0,0001 BTC), convertie en pièces par la taille du
/// contrat, sinon le trade est ignoré ; `id` toujours croissant.
pub fn trades(v: &serde_json::Value) -> Vec<crate::whales::Whale> {
    if v["channel"] != "futures.trades" || v["event"] != "update" {
        return vec![];
    }
    v["result"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|t| {
            let coin = crate::whales::coin_of(t["contract"].as_str()?);
            let k = crate::contracts::contract("Gate.io", &coin)?;
            let size = t["size"].as_f64()?;
            Some(crate::whales::Whale {
                time_ms: t["create_time_ms"].as_i64()?,
                price: t["price"].as_str()?.parse().ok()?,
                size: size.abs() * k,
                buy: size > 0.0,
                src: "Gate.io",
                wallet: String::new(),
                id: t["id"].as_u64(),
                coin,
            })
        })
        .filter(|w| w.size > 0.0)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gate_bougies_marches_catalogue() {
        assert_eq!(
            candles_request("btc", Timeframe::M30, 2, None).unwrap(),
            Request::Get(format!("{REST}/candlesticks?contract=BTC_USDT&interval=30m&limit=2"))
        );
        assert!(candles_request("BTC", Timeframe::H2, 2, None).is_err());
        let b = parse_candles(r#"[{"o":"76195.8","v":26633301,"t":1793912400,"c":"75967","l":"75726","h":"76195.9"}]"#).unwrap();
        assert_eq!((b[0].time, b[0].high, b[0].close), (1_793_912_400, 76195.9, 75967.0));
        let m = parse_markets(r#"[{"contract":"BTC_USDT","last":"110","change_percentage":"10"}]"#).unwrap();
        assert!((m[0].prev - 100.0).abs() < 1e-9);
        let m = parse_markets(r#"[{"contract":"BTC_USDT","last":"110"}]"#).unwrap();
        assert_eq!(m[0].change_pct(), None, "no change given: « — », not 0 %");
        let i = parse_instruments(r#"[{"name":"BTC_USDT","order_price_round":"0.1","in_delisting":false}]"#).unwrap();
        assert_eq!((i[0].id().as_str(), i[0].price_decimals), ("gate:BTC_USDT", 1));
    }

    /// 🔒 Reliable data: Gate.io counts its volumes in contracts (BTC: 0.0001 BTC each); a candle gives them in coins,
    /// with the size of the contract read from Gate.io (real answers of 23 Nov. 2026: 19 290 contracts = 1.929 BTC,
    /// 117 812 dollars at 61 080). A coin whose contract size is not known yet gives no live candle.
    #[test]
    fn gate_volumes_in_coins() {
        assert_eq!(contract_request("doge"), Request::Get(format!("{REST}/contracts/DOGE_USDT")));
        let detail = r#"{"name":"ZZGATE_USDT","type":"direct","quanto_multiplier":"0.0001","leverage_min":"1","order_price_round":"0.1"}"#;
        assert_eq!(parse_contract(detail).unwrap(), 0.0001);
        assert!(parse_contract(r#"{"label":"CONTRACT_NOT_FOUND","message":"Contract not found"}"#).is_err());
        let msg = r#"{"time":1795470429,"time_ms":1795470429716,"channel":"futures.candlesticks","event":"update","result":[{"t":1795470420,"c":"61082.1","h":"61092.4","l":"61046.7","o":"61048.2","a":"117811.98997","n":"1m_BTC_USDT","w":false,"v":19290}]}"#;
        let b = parse_live(msg).unwrap();
        assert!((b.volume - 1.929).abs() < 1e-9, "{}", b.volume);
        assert_eq!((b.time, b.open, b.close), (1_795_470_420, 61048.2, 61082.1));
        assert_eq!(parse_live(&msg.replace("BTC_USDT", "ZZNOSIZE_USDT")), None, "no size: no candle");
        crate::contracts::set_contracts("Gate.io", vec![("ZZGATELIVE".into(), 10.0)]);
        let b = parse_live(&msg.replace("BTC_USDT", "ZZGATELIVE_USDT")).unwrap();
        assert!((b.volume - 192_900.0).abs() < 1e-6);
    }
}
