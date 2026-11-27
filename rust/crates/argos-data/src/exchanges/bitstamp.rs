//! Bitstamp (spot en USD, paire « btcusd »). API publique v2.

use super::Request;
use crate::parse::{bars, json, num};
use crate::parse::{decimals, text};
use argos_core::feed::Market;
use argos_core::{ArgosError, Bar, Result, Timeframe};
use argos_core::{Instrument, MarketKind};

const REST: &str = "https://www.bitstamp.net/api/v2";
/// Pas offerts par Bitstamp (secondes).
const STEPS: [i64; 12] = [60, 180, 300, 900, 1800, 3600, 7200, 14400, 21600, 43200, 86400, 259200];

/// `end_ms` : seulement les bougies d'avant ce moment (ms), pour paginer vers le passé ; `None` = les plus récentes.
pub fn candles_request(base: &str, tf: Timeframe, limit: usize, end_ms: Option<i64>) -> Result<Request> {
    let end = end_ms.map_or(String::new(), |e| format!("&end={}", e / 1000));
    if !STEPS.contains(&tf.seconds()) {
        return Err(ArgosError::Invalid(format!("Bitstamp n'offre pas l'unité de temps {tf}")));
    }
    Ok(Request::Get(format!(
        "{REST}/ohlc/{}/?step={}&limit={}{end}",
        // ₿ « XRP/BTC » → xrpbtc; else the US dollar pair
        crate::paire::symbole(base, "", "USD").to_lowercase(),
        tf.seconds(),
        limit.min(1000)
    )))
}

pub fn parse_candles(body: &str) -> Result<Vec<Bar>> {
    let v = json(body)?;
    if !v["errors"].is_null() || !v["code"].is_null() {
        return Err(ArgosError::Feed(format!("Bitstamp : {}", v["message"].as_str().unwrap_or("erreur"))));
    }
    Ok(bars(&v["data"]["ohlc"]))
}

pub fn markets_request() -> Request {
    Request::Get(format!("{REST}/ticker/"))
}

pub fn parse_markets(body: &str) -> Result<Vec<Market>> {
    let v = json(body)?;
    let rows = v.as_array().ok_or_else(|| ArgosError::Feed("Bitstamp : liste absente".into()))?;
    Ok(rows
        .iter()
        .filter_map(|t| {
            let pair = t["pair"].as_str()?;
            // the US dollar pairs, and ₿ the BTC pairs (base « XRP/BTC »)
            let base = match pair.strip_suffix("/USD") {
                Some(b) => b.to_string(),
                None => crate::paire::base(pair.strip_suffix("/BTC").filter(|b| !b.is_empty())?, "BTC"),
            };
            Some(Market {
                base,
                price: num(&t["last"]),
                prev: num(&t["open_24"]),
            })
        })
        .collect())
}

/// Catalogue : les paires au comptant (`markets`).
pub fn instruments_request() -> Request {
    Request::Get(format!("{REST}/markets/"))
}

pub fn parse_instruments(body: &str) -> Result<Vec<Instrument>> {
    let v = json(body)?;
    let rows = v.as_array().ok_or_else(|| ArgosError::Feed("Bitstamp : liste des paires absente".into()))?;
    Ok(rows
        .iter()
        .map(|s| Instrument {
            exchange: "bitstamp".into(),
            symbol: text(&s["market_symbol"]),
            base: text(&s["base_currency"]),
            quote: text(&s["counter_currency"]),
            kind: MarketKind::Spot,
            group: "Spot".into(),
            price_decimals: decimals(&s["counter_decimals"]),
            size_decimals: decimals(&s["base_decimals"]),
            growth: false,
            active: s["trading"] == "Enabled",
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requete_et_lecture() {
        assert_eq!(
            candles_request("BTC", Timeframe::M30, 300, None).unwrap(),
            Request::Get("https://www.bitstamp.net/api/v2/ohlc/btcusd/?step=1800&limit=300".into())
        );
        assert!(candles_request("BTC", Timeframe::W1, 300, None).is_err());
        let b = parse_candles(
            r#"{"data":{"pair":"BTC/USD","ohlc":[{"timestamp":"1700000000","open":"80000","high":"80200","low":"79900","close":"80100","volume":"3.2"}]}}"#,
        )
        .unwrap();
        assert_eq!(
            b[0],
            Bar {
                time: 1_700_000_000,
                open: 80000.0,
                high: 80200.0,
                low: 79900.0,
                close: 80100.0,
                volume: 3.2
            }
        );
        let m = parse_markets(r#"[{"pair":"BTC/USD","last":"80000","open_24":"79000"},{"pair":"BTC/EUR","last":"1","open_24":"1"}]"#).unwrap();
        assert_eq!(m.len(), 1);
        // ₿ the BTC pairs: « xrpbtc », base « XRP/BTC »
        assert_eq!(
            candles_request("XRP/BTC", Timeframe::H4, 300, None).unwrap(),
            Request::Get("https://www.bitstamp.net/api/v2/ohlc/xrpbtc/?step=14400&limit=300".into())
        );
        let m = parse_markets(r#"[{"pair":"XRP/BTC","last":"0.0000234","open_24":"0.000023"}]"#).unwrap();
        assert_eq!(m[0].base, "XRP/BTC");
        assert!(trades_sub(&["XRP/BTC".into()]).subscribe.contains("live_trades_xrpbtc"));
    }
}

#[cfg(test)]
mod catalog_tests {
    use super::*;

    #[test]
    fn catalogue() {
        let body = r#"[{"name":"BTC/USD","market_symbol":"btcusd","base_currency":"BTC","base_decimals":8,"counter_currency":"USD","counter_decimals":0,"trading":"Enabled","market_type":"SPOT"}]"#;
        let l = parse_instruments(body).unwrap();
        assert_eq!(
            (l[0].id().as_str(), l[0].kind, l[0].price_decimals, l[0].size_decimals),
            ("bitstamp:btcusd", MarketKind::Spot, 0, 8)
        );
    }
}

/// Direct : Bitstamp n'a pas de canal de bougies, seulement les **trades** (`live_trades_btcusd`) : Argos construit
/// les bougies avec (`live::fold_trade`), comme TradingView ; l'orderflow réel les compte. Une seule connexion, un
/// canal par pièce (contre le dollar US).
pub fn trades_sub(bases: &[String]) -> crate::live::LiveSub {
    let canaux: Vec<String> = bases
        .iter()
        .map(|b| {
            format!(
                r#"{{"event":"bts:subscribe","data":{{"channel":"live_trades_{}"}}}}"#,
                crate::paire::symbole(b, "", "USD").to_lowercase()
            )
        })
        .collect();
    crate::live::LiveSub {
        url: "wss://ws.bitstamp.net".into(),
        subscribe: canaux.join("\n"),
        ping: r#"{"event":"bts:heartbeat"}"#.into(),
        ..Default::default()
    }
}

/// 🔎 LE décodeur d'un trade du direct de Bitstamp (bougies, Baleines et orderflow réel) :
/// `{"channel":"live_trades_btcusd","event":"trade","data":{"id":…,"amount":…,"price":…,"type":0,"microtimestamp":"…",
/// "buy_order_id":…,"sell_order_id":…}}`. Relevé sur le flux public le 2026-11-15 : `type` 0 = le preneur achète,
/// 1 = il vend (dans chaque trade relevé, c'est bien le côté dont l'ordre est le plus récent) ; `id` toujours
/// croissant ; prix en dollars US, `amount` en pièces, heure en microsecondes.
pub fn trade(v: &serde_json::Value) -> Option<crate::whales::Whale> {
    if v["event"] != "trade" {
        return None;
    }
    let d = &v["data"];
    let coin = v["channel"].as_str()?.strip_prefix("live_trades_")?.strip_suffix("usd")?.to_uppercase();
    Some(crate::whales::Whale {
        coin,
        time_ms: d["microtimestamp"].as_str()?.parse::<i64>().ok()? / 1000,
        price: d["price"].as_f64()?,
        size: d["amount"].as_f64()?,
        buy: match d["type"].as_u64()? {
            0 => true,
            1 => false,
            _ => return None,
        },
        src: "Bitstamp",
        wallet: String::new(),
        id: d["id"].as_u64(),
    })
}

/// Trade d'un message du direct, pour les bougies.
pub fn parse_trades(msg: &str) -> Vec<crate::live::Trade> {
    json(msg).ok().as_ref().and_then(trade).map_or(vec![], |w| {
        vec![crate::live::Trade {
            time_ms: w.time_ms,
            price: w.price,
            size: w.size,
        }]
    })
}
