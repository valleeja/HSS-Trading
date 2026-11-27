//! Aster (perps USDT, « BTCUSDT »). Son API publique est celle de Binance Futures (mêmes formats, autre adresse) :
//! on réutilise les lecteurs de `binance`. Testée en direct le 2026-11-05, sans clé ; l'ancien site l'utilisait déjà
//! (`src/23-…js`, fapi.asterdex.com).

use super::Request;
use argos_core::feed::Market;
use argos_core::{Bar, Instrument, Result, Timeframe};

const REST: &str = "https://fapi.asterdex.com/fapi/v1";
const WS: &str = "wss://fstream.asterdex.com/ws";

pub fn candles_request(base: &str, tf: Timeframe, limit: usize, end_ms: Option<i64>) -> Result<Request> {
    let end = end_ms.map_or(String::new(), |e| format!("&endTime={e}"));
    Ok(Request::Get(format!(
        "{REST}/klines?symbol={}USDT&interval={}&limit={}{end}",
        base.to_uppercase(),
        tf.id(),
        limit.min(1500)
    )))
}

pub fn parse_candles(body: &str) -> Result<Vec<Bar>> {
    super::binance::parse_candles(body)
}

pub fn markets_request() -> Request {
    Request::Get(format!("{REST}/ticker/24hr"))
}

pub fn parse_markets(body: &str) -> Result<Vec<Market>> {
    super::binance::parse_markets(body)
}

pub fn instruments_request() -> Request {
    Request::Get(format!("{REST}/exchangeInfo"))
}

/// Même catalogue que Binance, rangé sous « aster ».
pub fn parse_instruments(body: &str) -> Result<Vec<Instrument>> {
    let mut l = super::binance::parse_instruments(body)?;
    for i in &mut l {
        i.exchange = "aster".into();
    }
    Ok(l)
}

pub fn live_sub(base: &str, tf: Timeframe) -> crate::live::LiveSub {
    crate::live::LiveSub {
        url: WS.into(),
        subscribe: format!(r#"{{"method":"SUBSCRIBE","params":["{}usdt@kline_{}"],"id":1}}"#, base.to_lowercase(), tf.id()),
        ..Default::default()
    }
}

/// 🧮 Trades des perps de chaque pièce (`btcusdt@aggTrade`, format Binance Futures), sur une connexion : pour
/// l'orderflow réel ; lus par le décodeur des aggTrades de Binance (`m` = l'acheteur est le teneur).
pub fn trades_sub(coins: &[String]) -> crate::live::LiveSub {
    crate::whales::aggtrades(WS, coins)
}

pub fn parse_live(msg: &str) -> Option<Bar> {
    super::binance::parse_live(msg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aster_comme_binance() {
        assert_eq!(
            candles_request("btc", Timeframe::M30, 2, None).unwrap(),
            Request::Get("https://fapi.asterdex.com/fapi/v1/klines?symbol=BTCUSDT&interval=30m&limit=2".into())
        );
        let b = parse_candles(r#"[[1793914200000,"75986.6","76143.5","75770.1","75854.0","87.000",1793915999999,"6607411.1",735,"42.8","3250859.0","0"]]"#)
            .unwrap();
        assert_eq!((b[0].open, b[0].close), (75986.6, 75854.0));
        let i = parse_instruments(r#"{"symbols":[{"symbol":"BTCUSDT","baseAsset":"BTC","quoteAsset":"USDT","contractType":"PERPETUAL","status":"TRADING","pricePrecision":1,"quantityPrecision":3,"filters":[]}]}"#).unwrap();
        assert_eq!(i[0].id(), "aster:BTCUSDT");
    }
}
