//! Binance Futures (perps USDT-M, symbole « BTCUSDT »). API publique fapi.

use super::Request;
use crate::parse::{bars, json, num};
use crate::parse::{decimals, text};
use argos_core::feed::Market;
use argos_core::{ArgosError, Bar, Result, Timeframe};
use argos_core::{Instrument, MarketKind};

const REST: &str = "https://fapi.binance.com/fapi/v1";
/// Direct des marchés : la route `/market` (l'ancienne `/ws` accepte l'abonnement mais n'envoie plus les bougies —
/// vérifié le 25 oct. 2026).
const WS: &str = "wss://fstream.binance.com/market/ws";

/// Direct des bougies (flux `btcusdt@kline_30m`). Binance envoie lui-même les pings : rien à envoyer.
pub fn live_sub(base: &str, tf: Timeframe) -> crate::live::LiveSub {
    crate::live::LiveSub {
        url: WS.into(),
        subscribe: format!(r#"{{"method":"SUBSCRIBE","params":["{}usdt@kline_{}"],"id":1}}"#, base.to_lowercase(), tf.id()),
        ping: String::new(),
        ..Default::default()
    }
}

/// Bougie d'un message du direct (`{"e":"kline","k":{"t":…,"o":…}}`).
pub fn parse_live(msg: &str) -> Option<Bar> {
    let v = json(msg).ok()?;
    (v["e"] == "kline")
        .then(|| crate::parse::bar_from_object(&v["k"]))
        .and_then(crate::parse::valid)
}

/// `end_ms` : seulement les bougies d'avant ce moment (ms), pour paginer vers le passé ; `None` = les plus récentes.
pub fn candles_request(base: &str, tf: Timeframe, limit: usize, end_ms: Option<i64>) -> Result<Request> {
    let end = end_ms.map_or(String::new(), |e| format!("&endTime={}", e));
    Ok(Request::Get(format!(
        "{REST}/klines?symbol={base}USDT&interval={}&limit={}{end}",
        tf.id(),
        limit.min(1500)
    )))
}

pub fn parse_candles(body: &str) -> Result<Vec<Bar>> {
    let v = json(body)?;
    if v.is_object() {
        return Err(ArgosError::Feed(format!("Binance : {}", v["msg"].as_str().unwrap_or("erreur"))));
    }
    Ok(bars(&v))
}

pub fn markets_request() -> Request {
    Request::Get(format!("{REST}/ticker/24hr"))
}

pub fn parse_markets(body: &str) -> Result<Vec<Market>> {
    let v = json(body)?;
    let rows = v.as_array().ok_or_else(|| ArgosError::Feed("Binance : liste absente".into()))?;
    Ok(rows
        .iter()
        .filter_map(|t| {
            let base = t["symbol"].as_str()?.strip_suffix("USDT")?;
            Some(Market {
                base: base.to_string(),
                price: num(&t["lastPrice"]),
                prev: num(&t["openPrice"]),
            })
        })
        .collect())
}

/// Catalogue : tous les perps (`exchangeInfo`). Les perps « TradFi » (actions, or…) vont dans leur groupe.
pub fn instruments_request() -> Request {
    Request::Get(format!("{REST}/exchangeInfo"))
}

pub fn parse_instruments(body: &str) -> Result<Vec<Instrument>> {
    let v = json(body)?;
    let rows = v["symbols"]
        .as_array()
        .ok_or_else(|| ArgosError::Feed("Binance : liste des symboles absente".into()))?;
    Ok(rows
        .iter()
        .filter_map(|s| {
            let kind = s["contractType"].as_str()?;
            if kind != "PERPETUAL" && kind != "TRADIFI_PERPETUAL" {
                return None;
            }
            let tick = s["filters"]
                .as_array()?
                .iter()
                .find(|f| f["filterType"] == "PRICE_FILTER")
                .map(|f| &f["tickSize"]);
            Some(Instrument {
                exchange: "binance".into(),
                symbol: text(&s["symbol"]),
                base: text(&s["baseAsset"]),
                quote: text(&s["quoteAsset"]),
                kind: MarketKind::Perp,
                group: if kind == "TRADIFI_PERPETUAL" { "TradFi" } else { "Crypto" }.into(),
                price_decimals: tick.map_or_else(|| decimals(&s["pricePrecision"]), decimals),
                size_decimals: decimals(&s["quantityPrecision"]),
                growth: false,
                active: s["status"] == "TRADING",
            })
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requete_et_lecture() {
        assert_eq!(
            candles_request("BTC", Timeframe::H4, 2000, None).unwrap(),
            Request::Get("https://fapi.binance.com/fapi/v1/klines?symbol=BTCUSDT&interval=4h&limit=1500".into())
        );
        let b = parse_candles(r#"[[1700000000000,"80000","80200","79900","80100","12.5",1700001799999,"1","2","3","4","0"]]"#).unwrap();
        assert_eq!(b[0].volume, 12.5);
        assert!(parse_candles(r#"{"code":-1121,"msg":"Invalid symbol."}"#).is_err());
        let m = parse_markets(r#"[{"symbol":"ETHUSDT","lastPrice":"2300","openPrice":"2350"},{"symbol":"BTCUSDC","lastPrice":"1","openPrice":"1"}]"#).unwrap();
        assert_eq!(
            m,
            vec![Market {
                base: "ETH".into(),
                price: 2300.0,
                prev: 2350.0
            }]
        );
    }
}

#[cfg(test)]
mod catalog_tests {
    use super::*;

    #[test]
    fn catalogue() {
        let body = r#"{"symbols":[{"symbol":"BTCUSDT","contractType":"PERPETUAL","status":"TRADING","baseAsset":"BTC","quoteAsset":"USDT","pricePrecision":2,"quantityPrecision":3,"filters":[{"filterType":"PRICE_FILTER","tickSize":"0.10"}]},
            {"symbol":"TSLAUSDT","contractType":"TRADIFI_PERPETUAL","status":"TRADING","baseAsset":"TSLA","quoteAsset":"USDT","pricePrecision":2,"quantityPrecision":2,"filters":[]},
            {"symbol":"BTCUSDT_261225","contractType":"CURRENT_QUARTER","status":"TRADING","baseAsset":"BTC","quoteAsset":"USDT","filters":[]},
            {"symbol":"OLDUSDT","contractType":"PERPETUAL","status":"SETTLING","baseAsset":"OLD","quoteAsset":"USDT","filters":[]}]}"#;
        let l = parse_instruments(body).unwrap();
        assert_eq!(l.len(), 3, "les trimestriels sont exclus");
        assert_eq!(
            (l[0].id().as_str(), l[0].price_decimals, l[0].size_decimals, l[0].active),
            ("binance:BTCUSDT", 1, 3, true)
        );
        assert_eq!(l[1].group, "TradFi");
        assert!(!l[2].active, "en règlement : inactif");
    }
}
