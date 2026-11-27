//! Coinbase Exchange (spot en dollars, « BTC-USD »). API publique (testée en direct le 2026-11-05, sans clé). Pas de
//! canal de bougies chez Coinbase : le direct passe par ses trades (`trades_sub`), comme Bitstamp.

use super::Request;
use crate::parse::{decimals, json, num, text};
use argos_core::feed::Market;
use argos_core::{ArgosError, Bar, Result, Timeframe};
use argos_core::{Instrument, MarketKind};

const REST: &str = "https://api.exchange.coinbase.com";

fn gran(tf: Timeframe) -> Result<i64> {
    Ok(match tf {
        Timeframe::M1 => 60,
        Timeframe::M5 => 300,
        Timeframe::M15 => 900,
        Timeframe::H1 => 3600,
        Timeframe::D1 => 86_400,
        t => return Err(ArgosError::Feed(format!("Coinbase : pas de bougies {}", t.id()))),
    })
}

fn iso(s: i64) -> String {
    chrono::DateTime::from_timestamp(s, 0).map_or(String::new(), |d| d.format("%Y-%m-%dT%H:%M:%SZ").to_string())
}

/// `end_ms` : seulement les bougies d'avant ce moment (ms) ; Coinbase rend 300 bougies au plus.
pub fn candles_request(base: &str, tf: Timeframe, limit: usize, end_ms: Option<i64>) -> Result<Request> {
    let g = gran(tf)?;
    let range = match end_ms {
        Some(e) => {
            let end = e / 1000 - 1;
            format!("&start={}&end={}", iso(end - g * limit.min(300) as i64), iso(end))
        }
        None => String::new(),
    };
    Ok(Request::Get(format!(
        "{REST}/products/{}/candles?granularity={g}{range}",
        // ₿ « XRP/BTC » → XRP-BTC; else the US dollar pair
        crate::paire::symbole(base, "-", "USD")
    )))
}

/// `[temps, BAS, HAUT, ouverture, clôture, volume]` (attention à l'ordre), les plus récentes d'abord.
pub fn parse_candles(body: &str) -> Result<Vec<Bar>> {
    let v = json(body)?;
    if v.is_object() {
        return Err(ArgosError::Feed(format!("Coinbase : {}", v["message"].as_str().unwrap_or("erreur"))));
    }
    Ok(v.as_array()
        .map(|a| {
            a.iter()
                .map(|k| Bar {
                    time: crate::parse::secs(&k[0]),
                    low: num(&k[1]),
                    high: num(&k[2]),
                    open: num(&k[3]),
                    close: num(&k[4]),
                    volume: num(&k[5]),
                })
                .collect()
        })
        .unwrap_or_default())
}

pub fn markets_request() -> Request {
    Request::Get(format!("{REST}/products/stats"))
}

/// `{"BTC-USD":{"stats_24hour":{"open","last"}}, …}`.
pub fn parse_markets(body: &str) -> Result<Vec<Market>> {
    let v = json(body)?;
    let rows = v.as_object().ok_or_else(|| ArgosError::Feed("Coinbase : liste absente".into()))?;
    Ok(rows
        .iter()
        .filter_map(|(id, s)| {
            // the US dollar pairs, and ₿ the BTC pairs (base « XRP/BTC »)
            let base = match id.strip_suffix("-USD") {
                Some(b) => b.to_string(),
                None => crate::paire::base(id.strip_suffix("-BTC").filter(|b| !b.is_empty())?, "BTC"),
            };
            Some(Market {
                base,
                price: num(&s["stats_24hour"]["last"]),
                prev: num(&s["stats_24hour"]["open"]),
            })
        })
        .collect())
}

pub fn instruments_request() -> Request {
    Request::Get(format!("{REST}/products"))
}

pub fn parse_instruments(body: &str) -> Result<Vec<argos_core::Instrument>> {
    let v = json(body)?;
    let rows = v.as_array().ok_or_else(|| ArgosError::Feed("Coinbase : liste absente".into()))?;
    Ok(rows
        .iter()
        // the US dollar pairs, and ₿ the BTC pairs
        .filter(|s| s["quote_currency"] == "USD" || s["quote_currency"] == "BTC")
        .map(|s| Instrument {
            exchange: "coinbase".into(),
            symbol: text(&s["id"]),
            base: text(&s["base_currency"]),
            quote: text(&s["quote_currency"]),
            kind: MarketKind::Spot,
            group: "Crypto".into(),
            price_decimals: decimals(&s["quote_increment"]),
            size_decimals: decimals(&s["base_increment"]),
            growth: false,
            active: s["status"] == "online",
        })
        .collect())
}

/// Direct : Coinbase n'a pas de canal de bougies, seulement les **trades** (canal public `matches`, « BTC-USD ») :
/// Argos construit les bougies avec (`live::fold_trade`), comme TradingView ; les Baleines et l'orderflow réel les
/// comptent. Une seule connexion pour toutes les pièces (contre le dollar US) ; Coinbase pingue lui-même.
pub fn trades_sub(bases: &[String]) -> crate::live::LiveSub {
    let ids: Vec<String> = bases.iter().map(|b| format!(r#""{}""#, crate::paire::symbole(b, "-", "USD"))).collect();
    crate::live::LiveSub {
        url: "wss://ws-feed.exchange.coinbase.com".into(),
        subscribe: format!(r#"{{"type":"subscribe","product_ids":[{}],"channels":["matches"]}}"#, ids.join(",")),
        ..Default::default()
    }
}

/// 🔎 LE décodeur d'un trade du direct de Coinbase (bougies, Baleines et orderflow réel) : `{"type":"match",
/// "trade_id":…,"side":…,"size":"…","price":"…","product_id":"BTC-USD","time":"…Z"}`, et `last_match`, le dernier
/// trade d'avant l'abonnement. `side` = le côté du teneur de marché : le preneur fait l'inverse ; `trade_id` toujours
/// croissant ; prix en dollars US, taille en pièces.
pub fn trade(v: &serde_json::Value) -> Option<crate::whales::Whale> {
    if v["type"] != "match" && v["type"] != "last_match" {
        return None;
    }
    let n = |k: &str| v[k].as_str().and_then(|s| s.parse::<f64>().ok());
    Some(crate::whales::Whale {
        coin: crate::whales::coin_of(v["product_id"].as_str()?),
        time_ms: crate::whales::iso_ms(v["time"].as_str()?)?,
        price: n("price")?,
        size: n("size")?,
        buy: v["side"] == "sell",
        src: "Coinbase",
        id: v["trade_id"].as_u64(),
        wallet: String::new(),
    })
}

/// Trade d'un message du direct, pour les bougies (`last_match` non : d'avant l'abonnement, il est déjà dans les
/// bougies lues).
pub fn parse_trades(msg: &str) -> Vec<crate::live::Trade> {
    json(msg).ok().filter(|v| v["type"] == "match").as_ref().and_then(trade).map_or(vec![], |w| {
        vec![crate::live::Trade {
            time_ms: w.time_ms,
            price: w.price,
            size: w.size,
        }]
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coinbase_ordre_bas_haut_ouverture_cloture() {
        let b = parse_candles(r#"[[1793847600,80440.52,80750,80495.99,80456.58,135.7]]"#).unwrap();
        assert_eq!((b[0].low, b[0].high, b[0].open, b[0].close), (80440.52, 80750.0, 80495.99, 80456.58));
        assert!(candles_request("BTC", Timeframe::M30, 10, None).is_err(), "pas de 30 min chez Coinbase");
        let r = candles_request("btc", Timeframe::H1, 300, Some(1_793_851_200_000)).unwrap();
        let (s, e) = (iso(1_793_851_199 - 300 * 3600), iso(1_793_851_199));
        assert_eq!(r, Request::Get(format!("{REST}/products/BTC-USD/candles?granularity=3600&start={s}&end={e}")));
        assert!(e.ends_with('Z') && e.len() == 20);
        let m = parse_markets(r#"{"BTC-USD":{"stats_24hour":{"open":"100","last":"110"}},"BTC-EUR":{"stats_24hour":{"open":"1","last":"1"}}}"#).unwrap();
        assert_eq!((m.len(), m[0].base.as_str(), m[0].price), (1, "BTC", 110.0));
    }

    /// ₿ Coinbase's BTC pairs (« XRP-BTC »): catalogue, prices, candles; the dollar pairs unchanged.
    #[test]
    fn coinbase_paires_contre_btc() {
        let r = candles_request("XRP/BTC", Timeframe::H1, 300, None).unwrap();
        assert_eq!(r, Request::Get(format!("{REST}/products/XRP-BTC/candles?granularity=3600")));
        let m = parse_markets(r#"{"ETH-BTC":{"stats_24hour":{"open":"0.03","last":"0.031"}},"ETH-USD":{"stats_24hour":{"open":"1","last":"1"}}}"#).unwrap();
        let mut b: Vec<&str> = m.iter().map(|x| x.base.as_str()).collect();
        b.sort();
        assert_eq!(b, ["ETH", "ETH/BTC"]);
        let i = parse_instruments(
            r#"[{"id":"ETH-BTC","base_currency":"ETH","quote_currency":"BTC","quote_increment":"0.00001","base_increment":"0.00000001","status":"online"},{"id":"ETH-EUR","base_currency":"ETH","quote_currency":"EUR"}]"#,
        )
        .unwrap();
        assert_eq!((i.len(), i[0].quote.as_str(), i[0].price_decimals), (1, "BTC", 5));
        assert!(trades_sub(&["XRP/BTC".into(), "BTC".into()]).subscribe.contains(r#"["XRP-BTC","BTC-USD"]"#));
    }
}
