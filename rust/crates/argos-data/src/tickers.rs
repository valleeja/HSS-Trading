//! 📊 Tickers 24 h des perps crypto **Binance, OKX et Hyperliquid** (API publiques, sans clé) : prix, prix d'il y a
//! 24 h et volume 24 h en dollars, pour le Top gagnants / perdants et le Scanner. Un même symbole sur plusieurs
//! exchanges est dédoublonné : le plus liquide est gardé, les autres sont notés à côté.
//!
//! Une requête par exchange pour tous ses marchés (Binance `fapi/v1/ticker/24hr`, OKX `market/tickers?instType=SWAP`),
//! relue au plus une fois par minute par l'app : bien sous les limites de débit des deux.

use crate::exchanges::Request;
use crate::http::Http;
use crate::Exchange;
use argos_core::{ArgosError, Result};
use serde_json::Value;

/// Un marché perp et ses chiffres sur 24 h.
#[derive(Debug, Clone, PartialEq)]
pub struct Ticker {
    pub ex: Exchange,
    /// Jeton de base (« BTC »), celui qu'ouvre la chart de l'exchange.
    pub base: String,
    pub price: f64,
    pub prev: f64,
    pub vol_usd: f64,
}

impl Ticker {
    /// Variation sur 24 h en %.
    pub fn pct(&self) -> Option<f64> {
        (self.prev > 0.0 && self.price > 0.0).then(|| (self.price / self.prev - 1.0) * 100.0)
    }
}

/// Clé d'un symbole pour le dédoublonnage : Hyperliquid écrit « kPEPE » pour 1000 PEPE (même jeton, autre unité).
pub fn norm(base: &str) -> String {
    let b = base
        .strip_prefix('k')
        .filter(|r| r.chars().next().is_some_and(|c| c.is_ascii_uppercase()))
        .unwrap_or(base);
    b.to_uppercase()
}

/// Le plus liquide de chaque symbole, avec les autres exchanges qui l'ont aussi (du plus liquide au moins liquide).
pub fn best_by_base(list: &[Ticker]) -> Vec<(Ticker, Vec<Exchange>)> {
    let mut by: std::collections::BTreeMap<String, Vec<&Ticker>> = Default::default();
    for t in list.iter().filter(|t| t.price > 0.0) {
        by.entry(norm(&t.base)).or_default().push(t);
    }
    by.into_values()
        .map(|mut v| {
            v.sort_by(|a, b| b.vol_usd.total_cmp(&a.vol_usd));
            let others = v[1..].iter().map(|t| t.ex).collect();
            (v[0].clone(), others)
        })
        .collect()
}

fn num(v: &Value) -> f64 {
    v.as_f64().or_else(|| v.as_str().and_then(|s| s.parse().ok())).unwrap_or(0.0)
}

/// Binance Futures `ticker/24hr` (contrats USDT).
pub fn parse_binance(body: &str) -> Result<Vec<Ticker>> {
    let v: Value = serde_json::from_str(body).map_err(|e| ArgosError::Feed(format!("Binance : {e}")))?;
    let rows = v.as_array().ok_or_else(|| ArgosError::Feed("Binance : liste absente".into()))?;
    Ok(rows
        .iter()
        .filter_map(|t| {
            Some(Ticker {
                ex: Exchange::Binance,
                base: t["symbol"].as_str()?.strip_suffix("USDT")?.to_string(),
                price: num(&t["lastPrice"]),
                prev: num(&t["openPrice"]),
                vol_usd: num(&t["quoteVolume"]),
            })
        })
        .collect())
}

/// OKX `market/tickers?instType=SWAP` (contrats USDT) ; `volCcy24h` est en jeton de base : × dernier prix = dollars.
pub fn parse_okx(body: &str) -> Result<Vec<Ticker>> {
    let v: Value = serde_json::from_str(body).map_err(|e| ArgosError::Feed(format!("OKX : {e}")))?;
    if v["code"].as_str().is_some_and(|c| c != "0") {
        return Err(ArgosError::Feed(format!("OKX : {}", v["msg"].as_str().unwrap_or("erreur"))));
    }
    let rows = v["data"].as_array().ok_or_else(|| ArgosError::Feed("OKX : liste absente".into()))?;
    Ok(rows
        .iter()
        .filter_map(|t| {
            let last = num(&t["last"]);
            Some(Ticker {
                ex: Exchange::Okx,
                base: t["instId"].as_str()?.strip_suffix("-USDT-SWAP")?.to_string(),
                price: last,
                prev: num(&t["open24h"]),
                vol_usd: num(&t["volCcy24h"]) * last,
            })
        })
        .collect())
}

/// Tous les tickers d'un exchange (Binance ou OKX ; Hyperliquid vient du flux de la watchlist).
pub async fn fetch<H: Http>(http: &H, ex: Exchange) -> Result<Vec<Ticker>> {
    let url = match ex.markets_request() {
        Request::Get(u) => u,
        Request::PostJson { url, .. } => url,
    };
    let body = http.get(&url).await?;
    match ex {
        Exchange::Binance => parse_binance(&body),
        Exchange::Okx => parse_okx(&body),
        other => Err(ArgosError::Feed(format!("{} : tickers non gérés ici", other.name()))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // formats relevés en direct le 2026-11-05 (raccourcis)
    const BN: &str = r#"[{"symbol":"BTCUSDT","lastPrice":"80600","openPrice":"79000","quoteVolume":"12000000000"},{"symbol":"ETHUSDT","lastPrice":"2430","openPrice":"2500","quoteVolume":"6000000000"},{"symbol":"BTCUSDC","lastPrice":"80600","openPrice":"79000","quoteVolume":"1"}]"#;
    const OKX: &str = r#"{"code":"0","data":[{"instId":"BTC-USDT-SWAP","last":"80590","open24h":"79100","volCcy24h":"40000"},{"instId":"ZEC-USDT-SWAP","last":"1300","open24h":"1200","volCcy24h":"200000"},{"instId":"BTC-USD-SWAP","last":"80590","open24h":"79100","volCcy24h":"1"}]}"#;

    #[test]
    fn lit_binance_et_okx() {
        let b = parse_binance(BN).unwrap();
        assert_eq!(b.len(), 2, "contrats USDT seulement");
        assert_eq!((b[0].base.as_str(), b[0].vol_usd), ("BTC", 12e9));
        assert!((b[0].pct().unwrap() - 2.025).abs() < 0.01);
        let o = parse_okx(OKX).unwrap();
        assert_eq!(o.len(), 2);
        assert_eq!((o[0].base.as_str(), o[0].vol_usd), ("BTC", 40_000.0 * 80_590.0), "volume en $ = jetons × prix");
        assert!(parse_okx(r#"{"code":"50011","msg":"Too Many Requests","data":[]}"#).is_err());
    }

    #[test]
    fn le_plus_liquide_d_abord_les_autres_notes() {
        let mut all = parse_binance(BN).unwrap();
        all.extend(parse_okx(OKX).unwrap());
        all.push(Ticker {
            ex: Exchange::Hyperliquid,
            base: "BTC".into(),
            price: 80_610.0,
            prev: 79_050.0,
            vol_usd: 5e9,
        });
        let best = best_by_base(&all);
        let btc = best.iter().find(|(t, _)| t.base == "BTC").unwrap();
        assert_eq!(btc.0.ex, Exchange::Binance, "12 G$ chez Binance");
        assert_eq!(
            btc.1,
            vec![Exchange::Hyperliquid, Exchange::Okx],
            "les autres, du plus liquide au moins liquide"
        );
        assert_eq!(best.len(), 3, "BTC, ETH, ZEC");
        assert_eq!((norm("kPEPE"), norm("KAITO"), norm("kaito")), ("PEPE".into(), "KAITO".into(), "KAITO".into()));
    }
}
