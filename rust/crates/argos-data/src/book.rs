//! 📖 Carnet d'ordres, ⚡ trades et 💸 funding en direct (modules de la colonne), **Hyperliquid** : flux public sans
//! clé (`l2Book` + `trades` de la pièce du graphique) et historique du funding (`fundingHistory`). Lecture seule.

use crate::live::LiveSub;
use serde_json::Value;

/// Un niveau du carnet : prix, quantité, nombre d'ordres.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Level {
    pub px: f64,
    pub sz: f64,
    pub n: u32,
}

/// Le carnet : achats (du meilleur au moins bon) et ventes (du meilleur au moins bon).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Book {
    pub coin: String,
    pub time_ms: i64,
    pub bids: Vec<Level>,
    pub asks: Vec<Level>,
}

impl Book {
    /// Écart entre la meilleure vente et le meilleur achat (prix, %).
    pub fn spread(&self) -> Option<(f64, f64)> {
        let (b, a) = (self.bids.first()?.px, self.asks.first()?.px);
        Some((a - b, (a - b) / ((a + b) / 2.0) * 100.0))
    }
}

/// Flux Hyperliquid sur **une seule connexion** : carnet et trades de `coin` (s'il y en a un ; carnet regroupé à `sig`
/// chiffres significatifs, 2 à 5, plus large autour du prix pour la heatmap ; `None` : détaillé), plus les trades
/// seuls de chaque pièce d'`autres` (🧮 marchés enregistrés en continu). Un abonnement par ligne.
pub fn sub(coin: Option<&str>, sig: Option<u8>, autres: &[String]) -> LiveSub {
    let trades = |c: &str| format!("{{\"method\":\"subscribe\",\"subscription\":{{\"type\":\"trades\",\"coin\":\"{c}\"}}}}");
    let mut lignes = vec![];
    if let Some(coin) = coin {
        lignes.push(match sig {
            Some(n) => format!(
                "{{\"method\":\"subscribe\",\"subscription\":{{\"type\":\"l2Book\",\"coin\":\"{coin}\",\"nSigFigs\":{}}}}}",
                n.clamp(2, 5)
            ),
            None => format!("{{\"method\":\"subscribe\",\"subscription\":{{\"type\":\"l2Book\",\"coin\":\"{coin}\"}}}}"),
        });
        lignes.push(trades(coin));
    }
    lignes.extend(autres.iter().filter(|c| Some(c.as_str()) != coin).map(|c| trades(c)));
    LiveSub {
        url: "wss://api.hyperliquid.xyz/ws".into(),
        subscribe: lignes.join("\n"),
        ping: r#"{"method":"ping"}"#.into(),
        ..Default::default()
    }
}

/// The book of an `l2Book` message (`None`: something else). A level that cannot be (a price or a size that is not
/// a number above zero, `feed::price_size_ok`) is left out.
pub fn parse_book(msg: &str) -> Option<Book> {
    let v: Value = serde_json::from_str(msg).ok()?;
    if v["channel"] != "l2Book" {
        return None;
    }
    let d = &v["data"];
    let side = |k: usize| -> Vec<Level> {
        d["levels"][k]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|l| {
                Some(Level {
                    px: l["px"].as_str()?.parse().ok()?,
                    sz: l["sz"].as_str()?.parse().ok()?,
                    n: l["n"].as_u64().unwrap_or(0) as u32,
                })
            })
            .filter(|l| argos_core::feed::price_size_ok(l.px, l.sz))
            .collect()
    };
    Some(Book {
        coin: d["coin"].as_str()?.to_string(),
        time_ms: d["time"].as_i64().unwrap_or(0),
        bids: side(0),
        asks: side(1),
    })
}

/// Un funding passé : moment (ms) et taux horaire.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Funding {
    pub time_ms: i64,
    pub rate: f64,
}

/// Lit la réponse `fundingHistory` d'Hyperliquid (dans l'ordre du temps).
pub fn parse_funding(raw: &str) -> Vec<Funding> {
    let Ok(v) = serde_json::from_str::<Value>(raw) else { return vec![] };
    let mut l: Vec<Funding> = v
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|f| {
            Some(Funding {
                time_ms: f["time"].as_i64()?,
                rate: f["fundingRate"].as_str()?.parse().ok()?,
            })
        })
        .collect();
    l.sort_by_key(|f| f.time_ms);
    l
}

/// Fundings des dernières 24 h de `coin` (Hyperliquid, public).
#[cfg(any(feature = "native", feature = "web"))]
pub async fn funding_history(coin: &str, now_ms: i64) -> argos_core::Result<Vec<Funding>> {
    use crate::http::Http;
    let body = format!(r#"{{"type":"fundingHistory","coin":"{coin}","startTime":{}}}"#, now_ms - 24 * 3600 * 1000);
    let raw = crate::rt::Client::default()
        .post_json("https://api.hyperliquid.xyz/info", &body)
        .await
        .map_err(|e| e.to_string())?;
    Ok(parse_funding(&raw))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn carnet_et_funding() {
        let msg = r#"{"channel":"l2Book","data":{"coin":"BTC","time":1793050000000,"levels":[
            [{"px":"80000.0","sz":"1.5","n":3},{"px":"79999.0","sz":"0.2","n":1}],
            [{"px":"80001.0","sz":"0.7","n":2},{"px":"80003.0","sz":"4.0","n":5}]]}}"#;
        let b = parse_book(msg).unwrap();
        assert_eq!((b.bids.len(), b.asks[1].sz, b.bids[0].n), (2, 4.0, 3));
        let (s, pct) = b.spread().unwrap();
        assert_eq!(s, 1.0);
        assert!((pct - 0.00125).abs() < 1e-6);
        assert!(parse_book(r#"{"channel":"trades","data":[]}"#).is_none());
        let s = sub(Some("ETH"), None, &[]);
        assert_eq!(s.subscribe.lines().count(), 2);
        assert!(s.subscribe.lines().all(|l| serde_json::from_str::<Value>(l).is_ok()));
        // 🧮 marchés enregistrés : leurs trades seuls, sur la même connexion (pas de carnet, pas de doublon)
        let s = sub(Some("ETH"), Some(3), &["BTC".into(), "ETH".into(), "SOL".into()]);
        assert_eq!(s.subscribe.lines().count(), 4);
        assert_eq!(s.subscribe.matches("l2Book").count(), 1);
        assert!(s.subscribe.contains(r#""nSigFigs":3"#) && s.subscribe.contains(r#""coin":"SOL""#));
        assert_eq!(sub(None, None, &["BTC".into()]).subscribe.lines().count(), 1, "seulement les trades du BTC");
        let f = parse_funding(
            r#"[{"coin":"BTC","fundingRate":"0.0000125","premium":"0.0001","time":1793050000000},{"coin":"BTC","fundingRate":"-0.00001","premium":"0","time":1793046400000}]"#,
        );
        assert_eq!((f.len(), f[0].rate), (2, -0.00001), "dans l'ordre du temps");
    }
}
