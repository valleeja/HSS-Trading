//! 👛 Wallet en **lecture seule** : l'état public d'une adresse Hyperliquid (valeur du compte, positions, soldes
//! spot, ordres ouverts, derniers trades), lu par l'API publique `info` (sans clé). Aucune clé privée, aucune
//! signature, aucun ordre : l'adresse publique suffit, comme sur un explorateur.
//!
//! Formats relevés en direct le 2026-11-05 (`clearinghouseState`, `spotClearinghouseState`, `frontendOpenOrders`,
//! `userFills`).

use serde_json::Value;

#[cfg(any(feature = "native", feature = "web"))]
const INFO: &str = "https://api.hyperliquid.xyz/info";

/// Une position perp.
#[derive(Debug, Clone, PartialEq)]
pub struct Pos {
    pub coin: String,
    /// Taille signée (négative : short).
    pub size: f64,
    pub entry: f64,
    pub upnl: f64,
    pub leverage: f64,
    pub liq: Option<f64>,
}

/// Un ordre ouvert.
#[derive(Debug, Clone, PartialEq)]
pub struct Order {
    pub coin: String,
    pub buy: bool,
    pub px: f64,
    pub size: f64,
    pub kind: String,
}

/// Un trade (fill).
#[derive(Debug, Clone, PartialEq)]
pub struct Fill {
    pub coin: String,
    pub buy: bool,
    pub px: f64,
    pub size: f64,
    pub time_ms: i64,
    /// « Open Long », « Close Short »…
    pub dir: String,
    pub closed_pnl: f64,
}

/// L'état d'une adresse.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Snapshot {
    pub value: f64,
    pub withdrawable: f64,
    pub margin_used: f64,
    pub positions: Vec<Pos>,
    /// Soldes spot : (jeton, total, bloqué).
    pub balances: Vec<(String, f64, f64)>,
    pub orders: Vec<Order>,
    /// Les plus récents d'abord.
    pub fills: Vec<Fill>,
}

impl Snapshot {
    /// PnL latent de toutes les positions.
    pub fn upnl(&self) -> f64 {
        self.positions.iter().map(|p| p.upnl).sum()
    }
}

fn f(v: &Value) -> f64 {
    v.as_str().and_then(|s| s.parse::<f64>().ok()).or_else(|| v.as_f64()).unwrap_or(0.0)
}

/// Une adresse publique Hyperliquid (0x + 40 caractères hexadécimaux) ; refuse tout ce qui ressemble à une clé privée.
pub fn is_address(a: &str) -> bool {
    crate::whales::is_address(a.trim())
}

/// `clearinghouseState` : valeur, retirable, marge, positions (la plus grosse d'abord).
pub fn parse_perps(raw: &str, s: &mut Snapshot) -> Option<()> {
    let v: Value = serde_json::from_str(raw).ok()?;
    let m = &v["marginSummary"];
    s.value = f(&m["accountValue"]);
    s.margin_used = f(&m["totalMarginUsed"]);
    s.withdrawable = f(&v["withdrawable"]);
    s.positions = v["assetPositions"]
        .as_array()?
        .iter()
        .map(|a| &a["position"])
        .filter_map(|p| {
            Some(Pos {
                coin: p["coin"].as_str()?.to_string(),
                size: f(&p["szi"]),
                entry: f(&p["entryPx"]),
                upnl: f(&p["unrealizedPnl"]),
                leverage: f(&p["leverage"]["value"]),
                liq: Some(f(&p["liquidationPx"])).filter(|x| *x > 0.0),
            })
        })
        .filter(|p| p.size != 0.0)
        .collect();
    s.positions.sort_by(|a, b| (b.size.abs() * b.entry).total_cmp(&(a.size.abs() * a.entry)));
    Some(())
}

/// `spotClearinghouseState` : soldes non nuls.
pub fn parse_spot(raw: &str, s: &mut Snapshot) -> Option<()> {
    let v: Value = serde_json::from_str(raw).ok()?;
    s.balances = v["balances"]
        .as_array()?
        .iter()
        .filter_map(|b| Some((b["coin"].as_str()?.to_string(), f(&b["total"]), f(&b["hold"]))))
        .filter(|b| b.1 > 0.0)
        .collect();
    Some(())
}

/// `frontendOpenOrders`.
pub fn parse_orders(raw: &str, s: &mut Snapshot) -> Option<()> {
    let v: Value = serde_json::from_str(raw).ok()?;
    s.orders = v
        .as_array()?
        .iter()
        .filter_map(|o| {
            Some(Order {
                coin: o["coin"].as_str()?.to_string(),
                buy: o["side"] == "B",
                px: f(&o["limitPx"]),
                size: f(&o["sz"]),
                kind: o["orderType"].as_str().unwrap_or("Limit").to_string(),
            })
        })
        .collect();
    Some(())
}

/// `userFills` : les plus récents d'abord, au plus `max`.
pub fn parse_fills(raw: &str, s: &mut Snapshot, max: usize) -> Option<()> {
    let v: Value = serde_json::from_str(raw).ok()?;
    let mut fills: Vec<Fill> = v
        .as_array()?
        .iter()
        .filter_map(|x| {
            Some(Fill {
                coin: x["coin"].as_str()?.to_string(),
                buy: x["side"] == "B",
                px: f(&x["px"]),
                size: f(&x["sz"]),
                time_ms: x["time"].as_i64()?,
                dir: x["dir"].as_str().unwrap_or("").to_string(),
                closed_pnl: f(&x["closedPnl"]),
            })
        })
        .collect();
    fills.sort_by_key(|x| std::cmp::Reverse(x.time_ms));
    fills.truncate(max);
    s.fills = fills;
    Some(())
}

/// Tout l'état public d'une adresse (4 lectures). Lecture seule : rien n'est signé ni envoyé d'autre que l'adresse.
#[cfg(any(feature = "native", feature = "web"))]
pub async fn snapshot(address: &str) -> argos_core::Result<Snapshot> {
    use crate::http::Http;
    let a = address.trim();
    if !is_address(a) {
        return Err("adresse invalide (0x suivi de 40 caractères)".into());
    }
    let http = crate::rt::Client::default();
    let ask = |t: &str| format!(r#"{{"type":"{t}","user":"{a}"}}"#);
    let mut s = Snapshot::default();
    let perps = http.post_json(INFO, &ask("clearinghouseState")).await.map_err(|e| e.to_string())?;
    parse_perps(&perps, &mut s).ok_or("réponse illisible (positions)")?;
    if let Ok(b) = http.post_json(INFO, &ask("spotClearinghouseState")).await {
        parse_spot(&b, &mut s);
    }
    if let Ok(b) = http.post_json(INFO, &ask("frontendOpenOrders")).await {
        parse_orders(&b, &mut s);
    }
    if let Ok(b) = http.post_json(INFO, &ask("userFills")).await {
        parse_fills(&b, &mut s, 20);
    }
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PERPS: &str = r#"{"marginSummary":{"accountValue":"4494.5036","totalNtlPos":"6138.4586","totalRawUsd":"10632.9622","totalMarginUsed":"1227.69172"},"withdrawable":"3266.81188","assetPositions":[{"type":"oneWay","position":{"coin":"BTC","szi":"-0.07633","leverage":{"type":"cross","value":5},"entryPx":"80422.5","positionValue":"6138.4586","unrealizedPnl":"0.192069","liquidationPx":"137582.7467801441","marginUsed":"1227.69172"}},{"type":"oneWay","position":{"coin":"ETH","szi":"0.5","leverage":{"type":"cross","value":10},"entryPx":"2400","unrealizedPnl":"-3.5","liquidationPx":null}}]}"#;
    const SPOT: &str =
        r#"{"balances":[{"coin":"USDC","token":0,"hold":"10.0","total":"125.5","entryNtl":"0.0"},{"coin":"PURR","token":1,"hold":"0.0","total":"0.0"}]}"#;
    const ORDERS: &str = r#"[{"coin":"BTC","isTrigger":false,"limitPx":"79000.0","oid":1,"orderType":"Limit","origSz":"0.01","reduceOnly":false,"side":"B","sz":"0.01","timestamp":1793855400000,"triggerPx":"0.0"}]"#;
    const FILLS: &str = r#"[{"coin":"BTC","px":"80418.0","sz":"0.00016","side":"B","time":1793855400021,"dir":"Close Short","closedPnl":"0.00072","fee":"-0.000386"},{"coin":"BTC","px":"80500.0","sz":"0.01","side":"A","time":1793855500000,"dir":"Open Short","closedPnl":"0.0"}]"#;

    #[test]
    fn lit_l_etat_public_d_une_adresse() {
        let mut s = Snapshot::default();
        parse_perps(PERPS, &mut s).unwrap();
        assert_eq!((s.value, s.withdrawable), (4494.5036, 3266.81188));
        assert_eq!(s.positions.len(), 2);
        assert_eq!(
            (s.positions[0].coin.as_str(), s.positions[0].size < 0.0, s.positions[0].leverage),
            ("BTC", true, 5.0),
            "la plus grosse d'abord, short"
        );
        assert_eq!(s.positions[1].liq, None, "sans prix de liquidation");
        assert!((s.upnl() - (0.192069 - 3.5)).abs() < 1e-9);
        parse_spot(SPOT, &mut s).unwrap();
        assert_eq!(s.balances, vec![("USDC".to_string(), 125.5, 10.0)], "soldes nuls retirés");
        parse_orders(ORDERS, &mut s).unwrap();
        assert!(s.orders[0].buy && s.orders[0].px == 79000.0);
        parse_fills(FILLS, &mut s, 20).unwrap();
        assert_eq!(s.fills[0].dir, "Open Short", "le plus récent d'abord");
    }

    #[test]
    fn adresse_publique_seulement() {
        assert!(is_address("0xbb160cb61e694570f9d570f9fcde7c61dd2143ad"));
        assert!(is_address(" 0xbb160cb61e694570f9d570f9fcde7c61dd2143ad "));
        // une clé privée fait 64 caractères hexadécimaux : refusée
        assert!(!is_address("0x4c0883a69102937d6231471b5dbb6204fe5129617082792ae468d01a3f362318"));
        assert!(!is_address("bonjour"));
    }
}
