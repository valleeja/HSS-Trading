//! WEEX (perps USDT, symbole « BTCUSDT »). API publique contrats v3 : https://api-contract.weex.com/capi/v3/market
//! WEEX bloque les navigateurs (CORS) : sur le web, la requête passe par le relais Argos (/api/exchange) ; sur desktop, en direct.

use super::Request;
use crate::parse::{bars, field, json, num};
use crate::parse::{decimals, text};
use argos_core::feed::Market;
use argos_core::{ArgosError, Bar, Result, Timeframe};
use argos_core::{Instrument, MarketKind};

const REST: &str = "https://api-contract.weex.com/capi/v3/market";

/// `end_ms` : seulement les bougies d'avant ce moment (ms), pour paginer vers le passé ; `None` = les plus récentes.
/// WEEX ignore `endTime` sur `klines` (toujours les plus récentes) : vers le passé, c'est `historyKlines`, qui demande
/// `startTime` **et** `endTime`, et **100 bougies au plus** par demande (vérifié en direct le 2026-11-07).
pub fn candles_request(base: &str, tf: Timeframe, limit: usize, end_ms: Option<i64>) -> Result<Request> {
    Ok(Request::Get(match end_ms {
        None => format!("{REST}/klines?symbol={base}USDT&interval={}&limit={}", tf.id(), limit.clamp(1, 300)),
        Some(e) => {
            let n = limit.clamp(1, 100);
            let start = e - tf.seconds() * 1000 * n as i64;
            format!(
                "{REST}/historyKlines?symbol={base}USDT&interval={}&limit={n}&startTime={start}&endTime={e}",
                tf.id()
            )
        }
    }))
}

pub fn parse_candles(body: &str) -> Result<Vec<Bar>> {
    let v = json(body)?;
    // la liste peut arriver directement ou dans « data »
    let list = if v.is_array() { &v } else { &v["data"] };
    if !list.is_array() {
        return Err(ArgosError::Feed(format!(
            "WEEX : {}",
            field(&v, &["msg", "message"]).as_str().unwrap_or("réponse inattendue")
        )));
    }
    Ok(bars(list))
}

pub fn markets_request() -> Request {
    Request::Get(format!("{REST}/ticker/24hr"))
}

pub fn parse_markets(body: &str) -> Result<Vec<Market>> {
    let v = json(body)?;
    let list = if v.is_array() { &v } else { &v["data"] };
    let rows = list.as_array().ok_or_else(|| ArgosError::Feed("WEEX : liste absente".into()))?;
    Ok(rows
        .iter()
        .filter_map(|t| {
            let base = t["symbol"].as_str()?.strip_suffix("USDT")?;
            let price = num(&t["lastPrice"]);
            let open = num(&t["openPrice"]);
            let prev = if open.is_finite() {
                open
            } else {
                price / (1.0 + num(&t["priceChangePercent"]) / 100.0)
            };
            Some(Market {
                base: base.to_string(),
                price,
                prev,
            })
        })
        .collect())
}

/// Catalogue : les perps (`exchangeInfo` v3).
pub fn instruments_request() -> Request {
    Request::Get(format!("{REST}/exchangeInfo"))
}

pub fn parse_instruments(body: &str) -> Result<Vec<Instrument>> {
    let v = json(body)?;
    let rows = v["symbols"]
        .as_array()
        .ok_or_else(|| ArgosError::Feed("WEEX : liste des symboles absente".into()))?;
    Ok(rows
        .iter()
        .filter(|s| s["contractType"] == "PERPETUAL")
        .map(|s| Instrument {
            exchange: "weex".into(),
            symbol: text(&s["symbol"]),
            base: text(&s["baseAsset"]),
            quote: text(&s["quoteAsset"]),
            kind: MarketKind::Perp,
            group: "Crypto".into(),
            price_decimals: decimals(&s["pricePrecision"]),
            size_decimals: decimals(&s["quantityPrecision"]),
            growth: false,
            active: true,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requete_et_lecture() {
        assert_eq!(
            candles_request("BTC", Timeframe::M30, 1000, None).unwrap(),
            Request::Get("https://api-contract.weex.com/capi/v3/market/klines?symbol=BTCUSDT&interval=30m&limit=300".into())
        );
        // format type Binance (tableau de tableaux) ou enveloppé dans « data »
        assert_eq!(parse_candles(r#"[[1700000000000,"1","2","0.5","1.5","10"]]"#).unwrap()[0].close, 1.5);
        assert_eq!(
            parse_candles(r#"{"data":[{"openTime":1700000000000,"open":"1","high":"2","low":"0.5","close":"1.6","volume":"10"}]}"#).unwrap()[0].close,
            1.6
        );
        assert!(parse_candles(r#"{"code":"40001","msg":"symbol invalide"}"#).is_err());
        let m = parse_markets(r#"[{"symbol":"BTCUSDT","lastPrice":"80000","priceChangePercent":"0"}]"#).unwrap();
        assert_eq!(m[0].prev, 80000.0);
    }

    /// Vers le passé : `historyKlines` avec `startTime` et `endTime` (WEEX ignore `endTime` sur `klines`).
    #[test]
    fn passe_par_history_klines_vers_le_passe() {
        let Request::Get(u) = candles_request("BTC", Timeframe::H1, 300, Some(1_791_432_000_000)).unwrap() else {
            panic!("GET attendu")
        };
        let start = 1_791_432_000_000i64 - 100 * 3_600_000;
        assert!(
            u.contains("/historyKlines?") && u.contains("limit=100&") && u.contains(&format!("startTime={start}")),
            "{u}"
        );
        assert!(u.ends_with("&endTime=1791432000000"), "{u}");
    }
}

#[cfg(test)]
mod catalog_tests {
    use super::*;

    #[test]
    fn catalogue() {
        let body = r#"{"assets":[],"symbols":[{"symbol":"BTCUSDT","baseAsset":"BTC","quoteAsset":"USDT","contractType":"PERPETUAL","pricePrecision":1,"quantityPrecision":4}]}"#;
        let l = parse_instruments(body).unwrap();
        assert_eq!((l[0].id().as_str(), l[0].price_decimals, l[0].size_decimals), ("weex:BTCUSDT", 1, 4));
    }
}

const WS: &str = "wss://ws-contract.weex.com/v3/ws/public";

/// Direct : bougies v3 (`btcusdt@kline_1m`) ; `{"method":"PING"}` garde le lien ouvert.
pub fn live_sub(base: &str, tf: Timeframe) -> crate::live::LiveSub {
    crate::live::LiveSub {
        url: WS.into(),
        subscribe: format!(r#"{{"method":"SUBSCRIBE","params":["{}usdt@kline_{}"],"id":1}}"#, base.to_lowercase(), tf.id()),
        ping: r#"{"method":"PING"}"#.into(),
        ..Default::default()
    }
}

/// Bougie d'un message du direct (`{"e":"kline","d":[{"t":…,"o":…}]}`) ; l'instantané du départ est ignoré
/// (l'historique vient du REST).
pub fn parse_live(msg: &str) -> Option<Bar> {
    let v = json(msg).ok()?;
    (v["e"] == "kline")
        .then(|| crate::parse::bar_from_object(&v["d"][0]))
        .and_then(crate::parse::valid)
}

/// 🧮 Trades des perps de chaque pièce (`btcusdt@trade`), sur une connexion : pour l'orderflow réel.
pub fn trades_sub(coins: &[String]) -> crate::live::LiveSub {
    let params: Vec<String> = coins.iter().map(|c| format!(r#""{}usdt@trade""#, c.to_lowercase())).collect();
    crate::live::LiveSub {
        url: WS.into(),
        subscribe: format!(r#"{{"method":"SUBSCRIBE","params":[{}],"id":1}}"#, params.join(",")),
        ping: r#"{"method":"PING"}"#.into(),
        ..Default::default()
    }
}

/// 🔎 Trades d'un message du direct : `{"e":"trade","s":"BTCUSDT","d":[{"T":ms,"t":"…","p","q","v","m":false}]}` (et
/// l'instantané du départ, `tradeSnapshot`). Relevé sur le flux public le 2026-11-15 : `m` = l'acheteur est le teneur,
/// donc `m: false` = le preneur achète (143 sur 158 au prix vendeur du carnet, et 138 ventes sur 145 au prix acheteur) ;
/// `q` en pièces (`v` = `p` × `q` en dollars) ; `t` est un texte, pas un numéro ; un lot n'est pas trié par heure.
pub fn trades(v: &serde_json::Value) -> Vec<crate::whales::Whale> {
    if !(v["e"] == "trade" || v["e"] == "tradeSnapshot") {
        return vec![];
    }
    let coin = crate::whales::coin_of(v["s"].as_str().unwrap_or(""));
    let num = |t: &serde_json::Value, k: &str| t[k].as_str().and_then(|s| s.parse::<f64>().ok());
    v["d"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|t| {
            Some(crate::whales::Whale {
                coin: coin.clone(),
                time_ms: t["T"].as_i64()?,
                price: num(t, "p")?,
                size: num(t, "q")?,
                buy: !t["m"].as_bool()?,
                src: "WEEX",
                wallet: String::new(),
                id: None,
            })
        })
        .filter(|w| !w.coin.is_empty())
        .collect()
}
