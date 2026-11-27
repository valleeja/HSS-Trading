//! ⚡ Each exchange's perpetual terms for the demo account (maximum leverage, maintenance margin rate, hours between
//! two funding payments), and the current funding rate read from its public API.
//!
//! Terms read in November 2026 from the exchanges' contract pages, first tier only (a large position gets a lower
//! maximum leverage on a real exchange) and rounded; an exchange can change them at any time. These numbers only feed
//! the demo account, never a real order.

use crate::http::Http;
use crate::Exchange;
use argos_core::perp::Bareme;

/// ⚡ The perpetual terms of `base` on `e`; `None` for a spot market (Coinbase, Kraken, Bitstamp, KuCoin, the spot
/// venues): no leverage, no liquidation, no funding.
pub fn bareme(e: Exchange, base: &str) -> Option<Bareme> {
    if e.market_kind() != "Perpétuel" {
        return None;
    }
    let b = base.to_uppercase();
    let (levier_max, maintenance, funding_h) = match e {
        // HIP-3 markets (« xyz:NVDA »): ×10 at most, funding every hour
        Exchange::Hyperliquid if base.contains(':') => (10.0, 0.05, 1.0),
        // Hyperliquid: maintenance margin = half the initial margin at the maximum leverage; funding every hour
        Exchange::Hyperliquid => {
            let max = match b.as_str() {
                "BTC" => 40.0,
                "ETH" => 25.0,
                "SOL" | "XRP" => 20.0,
                _ => 10.0,
            };
            (max, 0.5 / max, 1.0)
        }
        // centralised exchanges: ×100 on BTC and ETH, ×50 on the rest, 0.5 % maintenance, funding every 8 hours
        _ => (if b == "BTC" || b == "ETH" { 100.0 } else { 50.0 }, 0.005, 8.0),
    };
    Some(Bareme {
        levier_max,
        maintenance,
        funding_h,
    })
}

/// ⚡ The address of `base`'s current funding rate on `e` (`None`: no public rate read by Argos for that exchange;
/// Hyperliquid's comes with its prices).
pub fn url_funding(e: Exchange, base: &str) -> Option<String> {
    let s = e.ticker(base);
    Some(match e {
        Exchange::Binance => format!("https://fapi.binance.com/fapi/v1/premiumIndex?symbol={s}"),
        Exchange::Aster => format!("https://fapi.asterdex.com/fapi/v1/premiumIndex?symbol={s}"),
        Exchange::Bybit => format!("https://api.bybit.com/v5/market/tickers?category=linear&symbol={s}"),
        Exchange::Okx => format!("https://www.okx.com/api/v5/public/funding-rate?instId={s}"),
        Exchange::Bitget => format!("https://api.bitget.com/api/v2/mix/market/current-fund-rate?symbol={s}&productType=USDT-FUTURES"),
        Exchange::Gate => format!("https://api.gateio.ws/api/v4/futures/usdt/contracts/{s}"),
        _ => return None,
    })
}

/// ⚡ Reads an answer of [`url_funding`]: the rate of one payment and the hours between two payments.
pub fn lire_funding(e: Exchange, raw: &str) -> Option<(f64, f64)> {
    let v: serde_json::Value = serde_json::from_str(raw).ok()?;
    let nombre = |x: &serde_json::Value| x.as_str().and_then(|s| s.parse::<f64>().ok()).or_else(|| x.as_f64());
    let (taux, heures) = match e {
        Exchange::Binance | Exchange::Aster => (nombre(&v["lastFundingRate"])?, 8.0),
        Exchange::Bybit => {
            let t = &v["result"]["list"][0];
            (nombre(&t["fundingRate"])?, nombre(&t["fundingIntervalHour"]).unwrap_or(8.0))
        }
        Exchange::Okx => {
            let d = &v["data"][0];
            let ms = nombre(&d["nextFundingTime"]).zip(nombre(&d["fundingTime"])).map(|(a, b)| (a - b) / 3_600_000.0);
            (nombre(&d["fundingRate"])?, ms.filter(|h| *h >= 1.0).unwrap_or(8.0))
        }
        Exchange::Bitget => {
            let d = &v["data"][0];
            (nombre(&d["fundingRate"])?, nombre(&d["fundingRateInterval"]).unwrap_or(8.0))
        }
        Exchange::Gate => (nombre(&v["funding_rate"])?, nombre(&v["funding_interval"]).map_or(8.0, |s| s / 3600.0)),
        _ => return None,
    };
    (taux.is_finite() && taux.abs() < 0.05 && heures >= 1.0).then_some((taux, heures))
}

/// ⚡ The current funding rate of `base` on `e`, brought to 8 hours (the unit of the demo account); `None` when the
/// exchange has no public rate read by Argos or does not answer (the demo then uses the standard rate).
pub async fn taux_8h(http: &impl Http, e: Exchange, base: &str) -> Option<f64> {
    let url = url_funding(e, base)?;
    let raw = http.get(&url).await.ok()?;
    lire_funding(e, &raw).map(|(t, h)| t * 8.0 / h)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn baremes() {
        let b = bareme(Exchange::Hyperliquid, "BTC").unwrap();
        assert_eq!((b.levier_max, b.funding_h), (40.0, 1.0));
        assert!((b.maintenance - 0.0125).abs() < 1e-12);
        assert_eq!(bareme(Exchange::Hyperliquid, "xyz:NVDA").unwrap().levier_max, 10.0);
        assert_eq!(bareme(Exchange::Hyperliquid, "DOGE").unwrap().levier_max, 10.0);
        let b = bareme(Exchange::Binance, "btc").unwrap();
        assert_eq!((b.levier_max, b.maintenance, b.funding_h), (100.0, 0.005, 8.0));
        assert_eq!(bareme(Exchange::Bybit, "PEPE").unwrap().levier_max, 50.0);
        for e in [
            Exchange::Coinbase,
            Exchange::Kraken,
            Exchange::Bitstamp,
            Exchange::Kucoin,
            Exchange::BinanceSpot,
        ] {
            assert!(bareme(e, "BTC").is_none(), "{e:?} : spot");
        }
        // every perpetual market has its terms; the leverage at the maximum keeps the maintenance below the margin
        for e in Exchange::ALL.into_iter().filter(|e| e.market_kind() == "Perpétuel") {
            let b = bareme(e, "SOL").unwrap();
            assert!(b.maintenance < 1.0 / b.levier_max, "{e:?}");
        }
    }

    #[test]
    fn lit_les_taux() {
        let binance = r#"{"symbol":"BTCUSDT","markPrice":"91000.1","lastFundingRate":"0.00010000","nextFundingTime":1795161600000}"#;
        assert_eq!(lire_funding(Exchange::Binance, binance), Some((0.0001, 8.0)));
        let bybit = r#"{"retCode":0,"result":{"category":"linear","list":[{"symbol":"BTCUSDT","fundingRate":"-0.00005","fundingIntervalHour":"4"}]}}"#;
        assert_eq!(lire_funding(Exchange::Bybit, bybit), Some((-0.00005, 4.0)));
        let okx = r#"{"code":"0","data":[{"instId":"BTC-USDT-SWAP","fundingRate":"0.0002","fundingTime":"1795161600000","nextFundingTime":"1795190400000"}]}"#;
        assert_eq!(lire_funding(Exchange::Okx, okx), Some((0.0002, 8.0)));
        let bitget = r#"{"code":"00000","data":[{"symbol":"BTCUSDT","fundingRate":"0.0001","fundingRateInterval":"8"}]}"#;
        assert_eq!(lire_funding(Exchange::Bitget, bitget), Some((0.0001, 8.0)));
        let gate = r#"{"name":"BTC_USDT","funding_rate":"0.000075","funding_interval":28800}"#;
        assert_eq!(lire_funding(Exchange::Gate, gate), Some((0.000075, 8.0)));
        // nonsense is refused
        assert_eq!(lire_funding(Exchange::Binance, r#"{"lastFundingRate":"7"}"#), None);
        assert_eq!(lire_funding(Exchange::Binance, "<html>"), None);
        assert!(url_funding(Exchange::Weex, "BTC").is_none());
        for e in [
            Exchange::Binance,
            Exchange::Aster,
            Exchange::Bybit,
            Exchange::Okx,
            Exchange::Bitget,
            Exchange::Gate,
        ] {
            assert!(crate::net::autorise(&url_funding(e, "BTC").unwrap()), "{e:?}");
        }
    }
}
