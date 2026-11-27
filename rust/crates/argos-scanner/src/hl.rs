//! Lecture des chandelles Hyperliquid (API publique, aucune clé requise).

use serde::Deserialize;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const API: &str = "https://api.hyperliquid.xyz/info";

pub use argos_scanner::candle::Candle;

/// Format brut renvoyé par Hyperliquid (les prix sont des chaînes).
#[derive(Deserialize)]
struct RawCandle {
    t: i64,
    o: String,
    h: String,
    l: String,
    c: String,
}

/// Convertit le JSON brut de Hyperliquid en chandelles.
pub fn parse(json: &str) -> Result<Vec<Candle>, String> {
    let raw: Vec<RawCandle> = serde_json::from_str(json).map_err(|e| format!("JSON invalide : {e}"))?;
    raw.into_iter()
        .map(|r| {
            let p = |s: &str| s.parse::<f64>().map_err(|e| format!("prix invalide {s} : {e}"));
            Ok(Candle {
                t: r.t,
                o: p(&r.o)?,
                h: p(&r.h)?,
                l: p(&r.l)?,
                c: p(&r.c)?,
            })
        })
        .collect()
}

pub struct Client {
    http: reqwest::blocking::Client,
}

impl Client {
    pub fn new() -> Result<Self, String> {
        let http = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(20))
            .user_agent("argos-scanner/0.1")
            .build()
            .map_err(|e| e.to_string())?;
        Ok(Self { http })
    }

    /// Chandelles de `coin` à l'intervalle `interval` ("1d", "4h"…) sur les `days` derniers jours.
    pub fn candles(&self, coin: &str, interval: &str, days: i64) -> Result<Vec<Candle>, String> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| e.to_string())?.as_millis() as i64;
        let body = serde_json::json!({
            "type": "candleSnapshot",
            "req": { "coin": coin, "interval": interval, "startTime": now - days * 86_400_000 }
        });
        let resp = self.http.post(API).json(&body).send().map_err(|e| format!("{coin} {interval} : {e}"))?;
        if !resp.status().is_success() {
            return Err(format!("{coin} {interval} : HTTP {}", resp.status()));
        }
        let text = resp.text().map_err(|e| e.to_string())?;
        parse(&text)
    }
}
