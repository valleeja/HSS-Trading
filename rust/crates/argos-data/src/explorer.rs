//! 🧭 Explorer Hyperliquid (Jee's mock-up « explorer hyperliquid v1 », 22 Nov 2026): any public Hyperliquid address
//! (a wallet or a vault), read only, through the public `info` API (`https://api.hyperliquid.xyz/info`, already in the
//! white list): `clearinghouseState` (value, margin, positions), `frontendOpenOrders`, `userFills`, `userFunding` and
//! `portfolio` (PnL over a day and a month). Nothing is signed, nothing is sent but the address.
//!
//! 🚦 Hyperliquid's budget is 1,200 points a minute per IP for every screen of the house. Weights (Hyperliquid's
//! documentation): `clearinghouseState` 2, `frontendOpenOrders` and `portfolio` 20, `userFills` and `userFunding` 20
//! plus 1 per 20 items returned (up to 2,000 fills: 120 at most; 500 funding rows: 45 at most). See [`POIDS`].
//!
//! The public `info` API gives no list of vaults by name: the field takes an address only.

use serde_json::Value;

pub use crate::wallet::{Fill, Order};

/// 🚦 The weight of each request (Hyperliquid's documentation; `userFills` / `userFunding`: their most).
pub const POIDS: [(&str, u32); 5] = [
    ("clearinghouseState", 2),
    ("frontendOpenOrders", 20),
    ("portfolio", 20),
    ("userFills", 120),
    ("userFunding", 45),
];

/// An open position of the address.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Position {
    pub coin: String,
    /// Signed size (negative: short).
    pub size: f64,
    pub entry: f64,
    /// Price now (the position's value over its size).
    pub price: f64,
    /// Value in dollars.
    pub value: f64,
    /// Unrealised PnL (dollars).
    pub pnl: f64,
    pub leverage: f64,
    /// Liquidation price (`None`: none).
    pub liq: Option<f64>,
}

impl Position {
    pub fn long(&self) -> bool {
        self.size > 0.0
    }
}

/// The account of the address.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Compte {
    /// Account value (dollars).
    pub value: f64,
    /// Total notional of the positions (dollars).
    pub ntl: f64,
    pub margin_used: f64,
    /// The biggest first.
    pub positions: Vec<Position>,
}

impl Compte {
    /// Average leverage: the positions' notional over the account's value (`None`: an empty account).
    pub fn levier_moyen(&self) -> Option<f64> {
        (self.value > 0.0).then(|| self.ntl / self.value)
    }
}

/// A funding payment (`userFunding`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Funding {
    pub time_ms: i64,
    pub coin: String,
    /// Paid (negative) or received (positive), dollars.
    pub usdc: f64,
    /// Hourly rate (fraction).
    pub rate: f64,
    /// Position size at the time.
    pub szi: f64,
}

fn f(v: &Value) -> f64 {
    v.as_str().and_then(|s| s.parse::<f64>().ok()).or_else(|| v.as_f64()).unwrap_or(0.0)
}

/// Sorts positions: the biggest (value in dollars) first, then by name.
pub fn trier(positions: &mut [Position]) {
    positions.sort_by(|a, b| b.value.total_cmp(&a.value).then_with(|| a.coin.cmp(&b.coin)));
}

/// `clearinghouseState`.
pub fn parse_compte(raw: &str) -> Option<Compte> {
    let v: Value = serde_json::from_str(raw).ok()?;
    let m = &v["marginSummary"];
    let mut positions: Vec<Position> = v["assetPositions"]
        .as_array()?
        .iter()
        .map(|a| &a["position"])
        .filter_map(|p| {
            let size = f(&p["szi"]);
            let entry = f(&p["entryPx"]);
            let value = f(&p["positionValue"]);
            Some(Position {
                coin: p["coin"].as_str()?.to_string(),
                size,
                entry,
                price: if size != 0.0 && value > 0.0 { value / size.abs() } else { entry },
                value: if value > 0.0 { value } else { size.abs() * entry },
                pnl: f(&p["unrealizedPnl"]),
                leverage: f(&p["leverage"]["value"]),
                liq: Some(f(&p["liquidationPx"])).filter(|x| *x > 0.0),
            })
        })
        .filter(|p| p.size != 0.0)
        .collect();
    trier(&mut positions);
    let value = f(&m["accountValue"]);
    let ntl = match f(&m["totalNtlPos"]) {
        n if n > 0.0 => n,
        _ => positions.iter().map(|p| p.value).sum(),
    };
    Some(Compte {
        value,
        ntl,
        margin_used: f(&m["totalMarginUsed"]),
        positions,
    })
}

/// `frontendOpenOrders`.
pub fn parse_ordres(raw: &str) -> Option<Vec<Order>> {
    let mut s = crate::wallet::Snapshot::default();
    crate::wallet::parse_orders(raw, &mut s)?;
    Some(s.orders)
}

/// `userFills`: the `max` most recent first.
pub fn parse_fills(raw: &str, max: usize) -> Option<Vec<Fill>> {
    let mut s = crate::wallet::Snapshot::default();
    crate::wallet::parse_fills(raw, &mut s, max)?;
    Some(s.fills)
}

/// `userFunding`: the most recent first.
pub fn parse_funding(raw: &str) -> Option<Vec<Funding>> {
    let v: Value = serde_json::from_str(raw).ok()?;
    let mut out: Vec<Funding> = v
        .as_array()?
        .iter()
        .filter_map(|x| {
            let d = &x["delta"];
            Some(Funding {
                time_ms: x["time"].as_i64()?,
                coin: d["coin"].as_str()?.to_string(),
                usdc: f(&d["usdc"]),
                rate: f(&d["fundingRate"]),
                szi: f(&d["szi"]),
            })
        })
        .collect();
    out.sort_by_key(|x| std::cmp::Reverse(x.time_ms));
    Some(out)
}

/// `portfolio`: the PnL over a day and over a month (the last point of `pnlHistory` minus its first; `None`: the
/// period is missing).
pub fn parse_portfolio(raw: &str) -> Option<(Option<f64>, Option<f64>)> {
    let v: Value = serde_json::from_str(raw).ok()?;
    let periodes = v.as_array()?;
    let pnl = |nom: &str| -> Option<f64> {
        let p = periodes.iter().find(|p| p[0].as_str() == Some(nom))?;
        let h = p[1]["pnlHistory"].as_array()?;
        let (a, b) = (h.first()?, h.last()?);
        Some(f(&b[1]) - f(&a[1]))
    };
    Some((pnl("day"), pnl("month")))
}

/// PnL made from the fills of the last `periode_ms` (closed PnL minus fees): when `portfolio` cannot be read.
pub fn pnl_des_fills(fills: &[Fill], now_ms: i64, periode_ms: i64) -> f64 {
    fills.iter().filter(|x| now_ms - x.time_ms <= periode_ms).map(|x| x.closed_pnl).sum()
}

/// The coins that opened or closed between two readings of an address (« ⭐ Suivre »): (opened, closed).
pub fn changements(avant: &Compte, apres: &Compte) -> (Vec<String>, Vec<String>) {
    let a: Vec<&str> = avant.positions.iter().map(|p| p.coin.as_str()).collect();
    let b: Vec<&str> = apres.positions.iter().map(|p| p.coin.as_str()).collect();
    (
        b.iter().filter(|c| !a.contains(c)).map(|c| c.to_string()).collect(),
        a.iter().filter(|c| !b.contains(c)).map(|c| c.to_string()).collect(),
    )
}

/// Short address: « 0x7a3f…c91e ».
pub fn court(a: &str) -> String {
    if a.len() > 12 {
        format!("{}…{}", &a[..6], &a[a.len() - 4..])
    } else {
        a.to_string()
    }
}

#[cfg(any(feature = "native", feature = "web"))]
mod lire {
    use super::*;
    use crate::http::Http;

    const INFO: &str = "https://api.hyperliquid.xyz/info";

    async fn ask(body: String) -> argos_core::Result<String> {
        crate::rt::Client::default().post_json(INFO, &body).await.map_err(|e| e.to_string().into())
    }

    fn adresse(a: &str) -> argos_core::Result<String> {
        let a = a.trim().to_lowercase();
        if crate::whales::is_address(&a) {
            Ok(a)
        } else {
            Err("adresse invalide (0x suivi de 40 caractères)".into())
        }
    }

    /// `clearinghouseState` (weight 2).
    pub async fn compte(a: &str) -> argos_core::Result<Compte> {
        let a = adresse(a)?;
        let raw = ask(format!(r#"{{"type":"clearinghouseState","user":"{a}"}}"#)).await?;
        parse_compte(&raw).ok_or_else(|| "réponse illisible".into())
    }

    /// `frontendOpenOrders` (weight 20).
    pub async fn ordres(a: &str) -> argos_core::Result<Vec<Order>> {
        let a = adresse(a)?;
        let raw = ask(format!(r#"{{"type":"frontendOpenOrders","user":"{a}"}}"#)).await?;
        parse_ordres(&raw).ok_or_else(|| "réponse illisible".into())
    }

    /// `userFills`: the 50 most recent (weight 20 + 1 per 20 fills returned).
    pub async fn fills(a: &str) -> argos_core::Result<Vec<Fill>> {
        let a = adresse(a)?;
        let raw = ask(format!(r#"{{"type":"userFills","user":"{a}"}}"#)).await?;
        parse_fills(&raw, 50).ok_or_else(|| "réponse illisible".into())
    }

    /// `userFunding` over the last 30 days (weight 20 + 1 per 20 rows returned).
    pub async fn funding(a: &str, now_ms: i64) -> argos_core::Result<Vec<Funding>> {
        let a = adresse(a)?;
        let debut = now_ms - 30 * 86_400_000;
        let raw = ask(format!(r#"{{"type":"userFunding","user":"{a}","startTime":{debut}}}"#)).await?;
        parse_funding(&raw).ok_or_else(|| "réponse illisible".into())
    }

    /// `portfolio`: PnL 24 h and 30 days (weight 20).
    pub async fn portfolio(a: &str) -> argos_core::Result<(Option<f64>, Option<f64>)> {
        let a = adresse(a)?;
        let raw = ask(format!(r#"{{"type":"portfolio","user":"{a}"}}"#)).await?;
        parse_portfolio(&raw).ok_or_else(|| "réponse illisible".into())
    }
}

#[cfg(any(feature = "native", feature = "web"))]
pub use lire::{compte, fills, funding, ordres, portfolio};

#[cfg(test)]
mod tests {
    use super::*;

    const ETAT: &str = r#"{"marginSummary":{"accountValue":"184350.0","totalNtlPos":"848010.0","totalRawUsd":"0","totalMarginUsed":"90000"},"withdrawable":"1000","assetPositions":[
        {"type":"oneWay","position":{"coin":"ETH","szi":"-38.0","leverage":{"type":"cross","value":5},"entryPx":"2088.5","positionValue":"77577.6","unrealizedPnl":"1797.0","liquidationPx":"2455.0"}},
        {"type":"oneWay","position":{"coin":"BTC","szi":"2.4","leverage":{"type":"cross","value":10},"entryPx":"59820","positionValue":"146496","unrealizedPnl":"2928","liquidationPx":"54310"}},
        {"type":"oneWay","position":{"coin":"SOL","szi":"0","leverage":{"type":"cross","value":8},"entryPx":"116.1","positionValue":"0","unrealizedPnl":"0","liquidationPx":null}},
        {"type":"oneWay","position":{"coin":"HYPE","szi":"1500","leverage":{"type":"isolated","value":3},"entryPx":"33.4","positionValue":"46875","unrealizedPnl":"-3225","liquidationPx":null}}]}"#;

    /// 🧭 clearinghouseState: positions sorted (biggest first), closed ones left out, price from the value, short,
    /// no liquidation price, average leverage.
    #[test]
    fn lit_le_compte() {
        let c = parse_compte(ETAT).unwrap();
        assert_eq!(c.value, 184_350.0);
        assert_eq!(c.positions.iter().map(|p| p.coin.as_str()).collect::<Vec<_>>(), ["BTC", "ETH", "HYPE"]);
        let btc = &c.positions[0];
        assert!(btc.long() && (btc.price - 61_040.0).abs() < 1e-6 && btc.liq == Some(54_310.0) && btc.leverage == 10.0);
        assert!(!c.positions[1].long());
        assert_eq!(c.positions[2].liq, None);
        assert!((c.levier_moyen().unwrap() - 4.6).abs() < 0.01);
        assert_eq!(Compte::default().levier_moyen(), None);
        assert!(parse_compte("{}").is_none());
    }

    #[test]
    fn tri_des_positions() {
        let p = |c: &str, v: f64| Position {
            coin: c.into(),
            value: v,
            ..Default::default()
        };
        let mut v = vec![p("SOL", 10.0), p("BTC", 300.0), p("ETH", 10.0)];
        trier(&mut v);
        assert_eq!(
            v.iter().map(|p| p.coin.as_str()).collect::<Vec<_>>(),
            ["BTC", "ETH", "SOL"],
            "same value: by name"
        );
    }

    #[test]
    fn ordres_trades_funding_portfolio() {
        let o = parse_ordres(r#"[{"coin":"BTC","limitPx":"58000.0","orderType":"Limit","side":"B","sz":"0.5","timestamp":1}]"#).unwrap();
        assert!(o[0].buy && o[0].px == 58_000.0 && o[0].kind == "Limit");
        let fills: String = format!(
            "[{}]",
            (0..60)
                .map(|i| format!(
                    r#"{{"coin":"ETH","px":"2000","sz":"1","side":"A","time":{},"dir":"Close Long","closedPnl":"10"}}"#,
                    1000 + i
                ))
                .collect::<Vec<_>>()
                .join(",")
        );
        let f = parse_fills(&fills, 50).unwrap();
        assert_eq!((f.len(), f[0].time_ms), (50, 1059), "the 50 most recent");
        assert_eq!(pnl_des_fills(&f, 1059, 9), 100.0, "the last 10 ms");
        let fu = parse_funding(
            r#"[{"time":1000,"hash":"0x0","delta":{"type":"funding","coin":"ETH","usdc":"-3.6","szi":"38.0","fundingRate":"0.0000125","nSamples":null}},
                {"time":2000,"hash":"0x0","delta":{"type":"funding","coin":"BTC","usdc":"1.2","szi":"-2.4","fundingRate":"-0.00001","nSamples":null}}]"#,
        )
        .unwrap();
        assert_eq!((fu[0].coin.as_str(), fu[0].usdc, fu[1].rate), ("BTC", 1.2, 0.0000125), "the most recent first");
        let pf = r#"[["day",{"accountValueHistory":[[1,"100"],[2,"110"]],"pnlHistory":[[1,"0.0"],[2,"3120.0"]],"vlm":"0"}],
                     ["week",{"accountValueHistory":[],"pnlHistory":[],"vlm":"0"}],
                     ["month",{"accountValueHistory":[[1,"1"]],"pnlHistory":[[1,"500.0"],[5,"-8440.0"]],"vlm":"0"}]]"#;
        assert_eq!(parse_portfolio(pf), Some((Some(3120.0), Some(-8940.0))));
        assert_eq!(parse_portfolio(r#"[["allTime",{"pnlHistory":[]}]]"#), Some((None, None)));
    }

    /// ⭐ Suivre: the coins opened and closed between two readings.
    #[test]
    fn ouvertures_et_fermetures() {
        let a = parse_compte(ETAT).unwrap();
        let mut b = a.clone();
        b.positions.retain(|p| p.coin != "ETH");
        b.positions.push(Position {
            coin: "SOL".into(),
            size: 1.0,
            ..Default::default()
        });
        assert_eq!(changements(&a, &b), (vec!["SOL".to_string()], vec!["ETH".to_string()]));
        assert_eq!(court("0x7a3f00000000000000000000000000000000c91e"), "0x7a3f…c91e");
    }
}
