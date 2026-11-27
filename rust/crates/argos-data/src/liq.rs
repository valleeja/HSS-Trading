//! 💥 Liquidations en direct (module de la colonne), deux sources publiques et sans clé :
//! - Binance Futures, tous les marchés (`!forceOrder@arr`) : une vente forcée = une position **longue** liquidée, un
//!   achat forcé = un **short** ;
//! - Bybit (`allLiquidation.<symbole>`, les cryptos principales) : `S = Buy` = position **longue** liquidée (sens de la
//!   position, à l'inverse de Binance) ; son prix est le prix de faillite, le montant en $ est donc une estimation.
//!
//! Les deux adresses (`fstream.binance.com`, `stream.bybit.com`) sont dans la liste blanche (`net.rs`).

use crate::live::LiveSub;
use serde_json::Value;

/// Une liquidation.
#[derive(Debug, Clone, PartialEq)]
pub struct Liq {
    pub coin: String,
    pub time_ms: i64,
    pub price: f64,
    pub qty: f64,
    /// `true` : position longue liquidée (vente forcée) ; `false` : short liquidé.
    pub long: bool,
    /// Exchange de la liquidation (« Binance », « Bybit »).
    pub venue: &'static str,
}

impl Liq {
    pub fn usd(&self) -> f64 {
        self.price * self.qty
    }
}

/// Flux Binance Futures des liquidations de tous les marchés (Binance pingue lui-même).
pub fn sub() -> LiveSub {
    LiveSub {
        url: "wss://fstream.binance.com/market/ws".into(),
        subscribe: r#"{"method":"SUBSCRIBE","params":["!forceOrder@arr"],"id":9}"#.into(),
        ping: String::new(),
        ..Default::default()
    }
}

/// Liquidation d'un message `forceOrder` (vide : autre chose).
pub fn parse(msg: &str) -> Vec<Liq> {
    let Ok(v) = serde_json::from_str::<Value>(msg) else { return vec![] };
    if v["e"] != "forceOrder" {
        return vec![];
    }
    let o = &v["o"];
    let num = |k: &str| o[k].as_str().and_then(|s| s.parse::<f64>().ok()).filter(|x| x.is_finite() && *x > 0.0);
    (|| {
        let s = o["s"].as_str()?;
        let coin = s.strip_suffix("USDT").or_else(|| s.strip_suffix("USDC")).unwrap_or(s).to_string();
        Some(Liq {
            coin,
            time_ms: o["T"].as_i64()?,
            price: num("ap").or_else(|| num("p"))?,
            qty: num("z").or_else(|| num("q"))?,
            long: o["S"] == "SELL",
            venue: "Binance",
        })
    })()
    .into_iter()
    .collect()
}

/// Cryptos suivies sur Bybit (un sujet par symbole ; 10 sujets par message d'abonnement).
pub const BYBIT_COINS: &[&str] = &[
    "BTC", "ETH", "SOL", "XRP", "DOGE", "BNB", "ADA", "SUI", "LINK", "AVAX", "HYPE", "ZEC", "LTC", "NEAR", "ENA", "AAVE", "UNI", "ARB", "ONDO", "TRX",
];

/// Flux Bybit des liquidations des cryptos `coins` (contrats USDT) ; Bybit veut un ping toutes les 20 s.
pub fn bybit_sub(coins: &[&str]) -> LiveSub {
    let topics: Vec<String> = coins.iter().map(|c| format!("\"allLiquidation.{c}USDT\"")).collect();
    let subscribe = topics
        .chunks(10)
        .map(|t| format!(r#"{{"op":"subscribe","args":[{}]}}"#, t.join(",")))
        .collect::<Vec<_>>()
        .join("\n");
    LiveSub {
        url: "wss://stream.bybit.com/v5/public/linear".into(),
        subscribe,
        ping: r#"{"op":"ping"}"#.into(),
        ..Default::default()
    }
}

/// Liquidations d'un message `allLiquidation` de Bybit (vide : autre chose, comme la réponse au ping).
pub fn parse_bybit(msg: &str) -> Vec<Liq> {
    let Ok(v) = serde_json::from_str::<Value>(msg) else { return vec![] };
    if !v["topic"].as_str().is_some_and(|t| t.starts_with("allLiquidation.")) {
        return vec![];
    }
    let Some(list) = v["data"].as_array() else { return vec![] };
    list.iter()
        .filter_map(|d| {
            let num = |k: &str| d[k].as_str().and_then(|s| s.parse::<f64>().ok()).filter(|x| x.is_finite() && *x > 0.0);
            let s = d["s"].as_str()?;
            Some(Liq {
                coin: s.strip_suffix("USDT").unwrap_or(s).to_string(),
                time_ms: d["T"].as_i64()?,
                price: num("p")?,
                qty: num("v")?,
                long: d["S"] == "Buy",
                venue: "Bybit",
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn liquidations() {
        let m = r#"{"e":"forceOrder","E":1793050000100,"o":{"s":"ETHUSDT","S":"SELL","o":"LIMIT","f":"IOC","q":"12.5","p":"3990.10","ap":"3995.00","X":"FILLED","l":"12.5","z":"12.5","T":1793050000000}}"#;
        let l = parse(m);
        assert_eq!((l[0].coin.as_str(), l[0].long, l[0].usd()), ("ETH", true, 49_937.5));
        assert!(parse(r#"{"result":null,"id":9}"#).is_empty());
        assert!(serde_json::from_str::<Value>(&sub().subscribe).is_ok());
        assert_eq!(l[0].venue, "Binance");
    }

    #[test]
    fn liquidations_bybit() {
        let m = r#"{"topic":"allLiquidation.BTCUSDT","type":"snapshot","ts":1793050000200,"data":[{"T":1793050000100,"s":"BTCUSDT","S":"Buy","v":"0.5","p":"80000"},{"T":1793050000150,"s":"BTCUSDT","S":"Sell","v":"0.25","p":"80100"}]}"#;
        let l = parse_bybit(m);
        assert_eq!(l.len(), 2);
        assert_eq!((l[0].coin.as_str(), l[0].long, l[0].usd(), l[0].venue), ("BTC", true, 40_000.0, "Bybit"));
        assert!(!l[1].long, "Sell = short liquidé");
        assert!(parse_bybit(r#"{"success":true,"ret_msg":"pong","op":"ping"}"#).is_empty());
        let s = bybit_sub(BYBIT_COINS);
        let lines: Vec<&str> = s.subscribe.lines().collect();
        assert_eq!(lines.len(), 2, "20 sujets : 2 messages de 10");
        for l in lines {
            let v: Value = serde_json::from_str(l).unwrap();
            assert_eq!(v["args"].as_array().unwrap().len(), 10);
        }
        assert!(crate::net::verifier(&s.url).is_ok());
    }
}
