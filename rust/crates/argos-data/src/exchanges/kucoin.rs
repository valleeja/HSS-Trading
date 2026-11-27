//! KuCoin (spot, « BTC-USDT »). API publique v1 (testée en direct le 2026-11-05, sans clé ; le navigateur est refusé :
//! passe par le relais). Le direct passe par ses trades (`trade`), sur un flux dont l'adresse demande un jeton public
//! (`whales::kucoin_preparer`, redemandé à chaque connexion).

use super::Request;
use crate::parse::{decimals, json, num, text};
use argos_core::feed::Market;
use argos_core::{ArgosError, Bar, Result, Timeframe};
use argos_core::{Instrument, MarketKind};

const REST: &str = "https://api.kucoin.com/api";

fn kind(tf: Timeframe) -> &'static str {
    match tf {
        Timeframe::M1 => "1min",
        Timeframe::M3 => "3min",
        Timeframe::M5 => "5min",
        Timeframe::M15 => "15min",
        Timeframe::M30 => "30min",
        Timeframe::H1 => "1hour",
        Timeframe::H2 => "2hour",
        Timeframe::H4 => "4hour",
        Timeframe::H12 => "12hour",
        Timeframe::D1 => "1day",
        Timeframe::W1 => "1week",
    }
}

fn ok(v: &serde_json::Value) -> Result<()> {
    if v["code"] == "200000" {
        Ok(())
    } else {
        Err(ArgosError::Feed(format!("KuCoin : {}", v["msg"].as_str().unwrap_or("erreur"))))
    }
}

/// `end_ms` : seulement les bougies d'avant ce moment (ms) ; KuCoin rend 1500 bougies au plus.
pub fn candles_request(base: &str, tf: Timeframe, limit: usize, end_ms: Option<i64>) -> Result<Request> {
    let n = limit.min(1500) as i64;
    let range = match end_ms {
        Some(e) => {
            let end = e / 1000 - 1;
            format!("&startAt={}&endAt={end}", end - tf.seconds() * n)
        }
        None => String::new(),
    };
    Ok(Request::Get(format!(
        "{REST}/v1/market/candles?type={}&symbol={}{range}",
        kind(tf),
        // ₿ « XRP/BTC » → XRP-BTC; else the USDT pair
        crate::paire::symbole(base, "-", "USDT")
    )))
}

/// `[temps, ouverture, CLÔTURE, haut, bas, volume, montant]` (attention à l'ordre), les plus récentes d'abord.
pub fn parse_candles(body: &str) -> Result<Vec<Bar>> {
    let v = json(body)?;
    ok(&v)?;
    Ok(v["data"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|k| Bar {
                    time: crate::parse::secs(&k[0]),
                    open: num(&k[1]),
                    close: num(&k[2]),
                    high: num(&k[3]),
                    low: num(&k[4]),
                    volume: num(&k[5]),
                })
                .collect()
        })
        .unwrap_or_default())
}

pub fn markets_request() -> Request {
    Request::Get(format!("{REST}/v1/market/allTickers"))
}

pub fn parse_markets(body: &str) -> Result<Vec<Market>> {
    let v = json(body)?;
    ok(&v)?;
    let rows = v["data"]["ticker"]
        .as_array()
        .ok_or_else(|| ArgosError::Feed("KuCoin : liste absente".into()))?;
    Ok(rows
        .iter()
        .filter_map(|t| {
            let price = num(&t["last"]);
            let rate = num(&t["changeRate"]);
            let s = t["symbol"].as_str()?;
            // the USDT pairs, and ₿ the BTC pairs (base « XRP/BTC »)
            let base = match s.strip_suffix("-USDT") {
                Some(b) => b.to_string(),
                None => crate::paire::base(s.strip_suffix("-BTC").filter(|b| !b.is_empty())?, "BTC"),
            };
            Some(Market {
                base,
                price,
                // without the change: the price 24 h ago is not known (0, shown « — »), not a 0 % made up
                prev: if rate.is_finite() && rate > -1.0 { price / (1.0 + rate) } else { 0.0 },
            })
        })
        .collect())
}

pub fn instruments_request() -> Request {
    Request::Get(format!("{REST}/v2/symbols"))
}

pub fn parse_instruments(body: &str) -> Result<Vec<argos_core::Instrument>> {
    let v = json(body)?;
    ok(&v)?;
    let rows = v["data"].as_array().ok_or_else(|| ArgosError::Feed("KuCoin : liste absente".into()))?;
    Ok(rows
        .iter()
        // the USDT pairs, and ₿ the BTC pairs
        .filter(|s| s["quoteCurrency"] == "USDT" || s["quoteCurrency"] == "BTC")
        .map(|s| Instrument {
            exchange: "kucoin".into(),
            symbol: text(&s["symbol"]),
            base: text(&s["baseCurrency"]),
            quote: text(&s["quoteCurrency"]),
            kind: MarketKind::Spot,
            group: "Crypto".into(),
            price_decimals: decimals(&s["priceIncrement"]),
            size_decimals: decimals(&s["baseIncrement"]),
            growth: false,
            active: s["enableTrading"] != false,
        })
        .collect())
}

/// 🔎 LE décodeur d'un trade du direct de KuCoin (bougies, Baleines et orderflow réel), canal `/market/match` :
/// `{"type":"message","subject":"trade.l3match","data":{"price","size","side":"buy","symbol":"BTC-USDT",
/// "time":"ns","tradeId"}}`. `side` = le côté du preneur ; `tradeId` toujours croissant ; heure en nanosecondes ;
/// prix en USDT, taille en pièces.
pub fn trade(v: &serde_json::Value) -> Option<crate::whales::Whale> {
    if v["type"] != "message" || v["subject"] != "trade.l3match" {
        return None;
    }
    let d = &v["data"];
    let n = |k: &str| d[k].as_str().and_then(|s| s.parse::<f64>().ok());
    Some(crate::whales::Whale {
        coin: crate::whales::coin_of(d["symbol"].as_str()?),
        time_ms: d["time"].as_str()?.parse::<i64>().ok()? / 1_000_000,
        price: n("price")?,
        size: n("size")?,
        buy: d["side"] == "buy",
        src: "KuCoin",
        id: d["tradeId"].as_str().and_then(|s| s.parse().ok()),
        wallet: String::new(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kucoin_ordre_ouverture_cloture_haut_bas() {
        let b = parse_candles(r#"{"code":"200000","data":[["1793908800","76364","76090.4","76400","75957.5","50.7","3863153.4"]]}"#).unwrap();
        assert_eq!((b[0].open, b[0].close, b[0].high, b[0].low), (76364.0, 76090.4, 76400.0, 75957.5));
        let r = candles_request("btc", Timeframe::H2, 10, None).unwrap();
        assert_eq!(r, Request::Get(format!("{REST}/v1/market/candles?type=2hour&symbol=BTC-USDT")));
        let m = parse_markets(
            r#"{"code":"200000","data":{"ticker":[{"symbol":"BTC-USDT","last":"110","changeRate":"0.1"},{"symbol":"BTC-EUR","last":"1","changeRate":"0"}]}}"#,
        )
        .unwrap();
        assert_eq!(m.len(), 1);
        assert!((m[0].prev - 100.0).abs() < 1e-9);
        let m = parse_markets(r#"{"code":"200000","data":{"ticker":[{"symbol":"BTC-USDT","last":"110"}]}}"#).unwrap();
        assert_eq!(m[0].change_pct(), None, "no change given: « — », not 0 %");
        let i = parse_instruments(r#"{"code":"200000","data":[{"symbol":"BTC-USDT","baseCurrency":"BTC","quoteCurrency":"USDT","priceIncrement":"0.1","baseIncrement":"0.00000001","enableTrading":true}]}"#).unwrap();
        assert_eq!((i[0].id().as_str(), i[0].kind), ("kucoin:BTC-USDT", MarketKind::Spot));
    }

    /// ₿ KuCoin's BTC pairs (« XRP-BTC »): catalogue, prices, candles; the USDT pairs unchanged.
    #[test]
    fn kucoin_paires_contre_btc() {
        let r = candles_request("XRP/BTC", Timeframe::H2, 10, None).unwrap();
        assert_eq!(r, Request::Get(format!("{REST}/v1/market/candles?type=2hour&symbol=XRP-BTC")));
        let m = parse_markets(r#"{"code":"200000","data":{"ticker":[{"symbol":"XRP-BTC","last":"0.0000234","changeRate":"0"},{"symbol":"XRP-USDT","last":"2","changeRate":"0"}]}}"#)
            .unwrap();
        assert_eq!(m.iter().map(|x| x.base.as_str()).collect::<Vec<_>>(), ["XRP/BTC", "XRP"]);
        let i = parse_instruments(
            r#"{"code":"200000","data":[{"symbol":"XRP-BTC","baseCurrency":"XRP","quoteCurrency":"BTC","priceIncrement":"0.0000000001","baseIncrement":"0.0001","enableTrading":true},{"symbol":"XRP-EUR","baseCurrency":"XRP","quoteCurrency":"EUR"}]}"#,
        )
        .unwrap();
        assert_eq!((i.len(), i[0].quote.as_str(), i[0].price_decimals), (1, "BTC", 10));
    }
}
