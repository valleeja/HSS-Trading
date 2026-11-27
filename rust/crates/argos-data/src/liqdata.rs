//! 💥 Données publiques de la carte des liquidations (bougies + intérêt ouvert), sans clé :
//! - Binance Futures : klines + `/futures/data/openInterestHist` (30 jours au plus) ;
//! - OKX : `history-candles` + `rubik/stat/contracts/open-interest-history` (quelques jours en 5 min) ;
//! - Hyperliquid : pas d'historique public de l'intérêt ouvert ; l'app note l'intérêt ouvert courant
//!   (`metaAndAssetCtxs`) au fil du temps (historique en construction).

use crate::http::Http;
use argos_core::liqmap::OiBar;
use serde_json::Value;
use std::collections::BTreeMap;

/// Bourses de la carte.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Venue {
    Binance,
    Okx,
    Hyperliquid,
}

impl Venue {
    pub const ALL: [Venue; 3] = [Venue::Binance, Venue::Okx, Venue::Hyperliquid];
    pub fn name(self) -> &'static str {
        match self {
            Venue::Binance => "Binance",
            Venue::Okx => "OKX",
            Venue::Hyperliquid => "Hyperliquid",
        }
    }
}

fn num(v: &Value) -> Option<f64> {
    v.as_f64().or_else(|| v.as_str()?.parse().ok())
}

/// Bougies `[[ouverture ms, o, h, l, c, …], …]` (Binance et OKX) → (temps en s, o, h, l, c).
pub fn parse_candles(raw: &str) -> Vec<(i64, f64, f64, f64, f64)> {
    let v: Value = serde_json::from_str(raw).unwrap_or(Value::Null);
    let rows = v.get("data").unwrap_or(&v);
    let mut out: Vec<_> = rows
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|r| Some((num(&r[0])? as i64 / 1000, num(&r[1])?, num(&r[2])?, num(&r[3])?, num(&r[4])?)))
        .collect();
    out.sort_by_key(|r| r.0);
    out.dedup_by_key(|r| r.0);
    out
}

/// Historique de l'intérêt ouvert en $ → (temps en s, OI $). Binance : objets `sumOpenInterestValue` ; OKX : lignes
/// `[ts, oi, oiCcy, oiUsd]`.
pub fn parse_oi(raw: &str) -> Vec<(i64, f64)> {
    let v: Value = serde_json::from_str(raw).unwrap_or(Value::Null);
    let rows = v.get("data").unwrap_or(&v);
    let mut out: Vec<(i64, f64)> = rows
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|r| {
            if r.is_array() {
                Some((num(&r[0])? as i64 / 1000, num(&r[3])?))
            } else {
                Some((num(&r["timestamp"])? as i64 / 1000, num(&r["sumOpenInterestValue"])?))
            }
        })
        .collect();
    out.sort_by_key(|r| r.0);
    out.dedup_by_key(|r| r.0);
    out
}

/// Bougies + intérêt ouvert, appariés : l'OI noté à la clôture de la bougie (ou à son ouverture), sinon le dernier
/// connu. Les bougies d'avant le premier OI sont laissées de côté.
pub fn join(candles: &[(i64, f64, f64, f64, f64)], oi: &[(i64, f64)], step: i64) -> Vec<OiBar> {
    let m: BTreeMap<i64, f64> = oi.iter().copied().collect();
    candles
        .iter()
        .filter_map(|&(t, o, h, l, c)| {
            let v = m
                .get(&(t + step))
                .or_else(|| m.get(&t))
                .copied()
                .or_else(|| m.range(..=t + step).next_back().map(|x| *x.1))?;
            Some(OiBar {
                time: t,
                open: o,
                high: h,
                low: l,
                close: c,
                oi: v,
            })
        })
        .collect()
}

/// Intérêt ouvert courant (en $) d'une pièce Hyperliquid, depuis `metaAndAssetCtxs`.
pub fn parse_hl_oi(raw: &str, coin: &str) -> Option<f64> {
    let v: Value = serde_json::from_str(raw).ok()?;
    let i = v[0]["universe"].as_array()?.iter().position(|u| u["name"] == coin)?;
    let c = &v[1][i];
    Some(num(&c["openInterest"])? * num(&c["markPx"])?)
}

/// Période des données : 5 min jusqu'à 48 h, 1 h au-delà (moins de requêtes, et l'OI d'OKX en 5 min ne remonte pas
/// assez loin).
pub fn step_for(hours: i64) -> i64 {
    if hours <= 48 {
        300
    } else {
        3600
    }
}

/// Bougies + OI de Binance Futures sur les `hours` dernières heures (symbole « BTCUSDT »).
pub async fn binance(http: &impl Http, sym: &str, hours: i64, now: i64) -> argos_core::Result<Vec<OiBar>> {
    let step = step_for(hours);
    let (p, n) = if step == 300 { ("5m", hours * 12) } else { ("1h", hours) };
    let start = (now - hours * 3600) * 1000;
    let mut oi = vec![];
    let mut from = start;
    for _ in 0..30 {
        let url = format!("https://fapi.binance.com/futures/data/openInterestHist?symbol={sym}&period={p}&limit=500&startTime={from}");
        let page = parse_oi(&http.get(&url).await.map_err(|e| e.to_string())?);
        let Some(last) = page.last().map(|x| x.0) else { break };
        oi.extend(page);
        if (last + step) * 1000 >= now * 1000 || oi.len() as i64 >= n {
            break;
        }
        from = (last + step) * 1000;
        http.wait(150).await;
    }
    let url = format!(
        "https://fapi.binance.com/fapi/v1/klines?symbol={sym}&interval={p}&limit={}&startTime={start}",
        n.clamp(1, 1500)
    );
    let mut candles = parse_candles(&http.get(&url).await.map_err(|e| e.to_string())?);
    if n > 1500 {
        if let Some(last) = candles.last().map(|c| c.0) {
            let url = format!(
                "https://fapi.binance.com/fapi/v1/klines?symbol={sym}&interval={p}&limit=1500&startTime={}",
                (last + step) * 1000
            );
            candles.extend(parse_candles(&http.get(&url).await.map_err(|e| e.to_string())?));
        }
    }
    Ok(join(&candles, &oi, step))
}

/// Bougies + OI d'OKX sur les `hours` dernières heures (instrument « BTC-USDT-SWAP »).
pub async fn okx(http: &impl Http, inst: &str, hours: i64, now: i64) -> argos_core::Result<Vec<OiBar>> {
    let step = step_for(hours);
    let p = if step == 300 { "5m" } else { "1H" };
    let start = now - hours * 3600;
    let (mut oi, mut candles) = (vec![], vec![]);
    let mut end = now * 1000;
    for _ in 0..30 {
        let url = format!("https://www.okx.com/api/v5/rubik/stat/contracts/open-interest-history?instId={inst}&period={p}&limit=100&end={end}");
        let page = parse_oi(&http.get(&url).await.map_err(|e| e.to_string())?);
        let Some(first) = page.first().map(|x| x.0) else { break };
        oi.extend(page);
        if first <= start {
            break;
        }
        end = (first - 1) * 1000;
        http.wait(150).await;
    }
    let mut after = now * 1000;
    for _ in 0..30 {
        let url = format!("https://www.okx.com/api/v5/market/history-candles?instId={inst}&bar={p}&limit=100&after={after}");
        let page = parse_candles(&http.get(&url).await.map_err(|e| e.to_string())?);
        let Some(first) = page.first().map(|x| x.0) else { break };
        candles.extend(page);
        if first <= start {
            break;
        }
        after = first * 1000;
        http.wait(150).await;
    }
    candles.sort_by_key(|c| c.0);
    candles.dedup_by_key(|c| c.0);
    candles.retain(|c| c.0 >= start);
    oi.sort_by_key(|r| r.0);
    oi.dedup_by_key(|r| r.0);
    Ok(join(&candles, &oi, step))
}

/// Bougies 5 min de Hyperliquid et l'intérêt ouvert courant (en $) : l'app y ajoute ses relevés.
pub async fn hyperliquid(http: &impl Http, coin: &str, hours: i64, now: i64) -> argos_core::Result<(Vec<(i64, f64, f64, f64, f64)>, Option<f64>)> {
    let step = step_for(hours);
    let body = format!(
        r#"{{"type":"candleSnapshot","req":{{"coin":"{coin}","interval":"{}","startTime":{},"endTime":{}}}}}"#,
        if step == 300 { "5m" } else { "1h" },
        (now - hours * 3600) * 1000,
        now * 1000
    );
    let raw = http.post_json("https://api.hyperliquid.xyz/info", &body).await.map_err(|e| e.to_string())?;
    let v: Value = serde_json::from_str(&raw).unwrap_or(Value::Null);
    let candles = v
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|c| Some((num(&c["t"])? as i64 / 1000, num(&c["o"])?, num(&c["h"])?, num(&c["l"])?, num(&c["c"])?)))
        .collect();
    let ctx = http
        .post_json("https://api.hyperliquid.xyz/info", r#"{"type":"metaAndAssetCtxs"}"#)
        .await
        .map_err(|e| e.to_string())?;
    Ok((candles, parse_hl_oi(&ctx, coin)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lecture_et_appariement() {
        let bn = r#"[{"symbol":"BTCUSDT","sumOpenInterest":"1","sumOpenInterestValue":"7737822459.4","timestamp":1793829600000},{"symbol":"BTCUSDT","sumOpenInterest":"1","sumOpenInterestValue":"7743519821.0","timestamp":1793829900000}]"#;
        assert_eq!(parse_oi(bn), vec![(1_793_829_600, 7_737_822_459.4), (1_793_829_900, 7_743_519_821.0)]);
        let ok =
            r#"{"code":"0","data":[["1793829900000","3361176.4","33608.3","2697310655.0"],["1793829600000","3361176.4","33611.7","2696641599.2"]],"msg":""}"#;
        assert_eq!(parse_oi(ok)[0], (1_793_829_600, 2_696_641_599.2), "trié par heure");
        let k = r#"[[1793829600000,"80000","80100","79900","80050","1"],[1793829900000,"80050","80200","80000","80150","1"]]"#;
        let c = parse_candles(k);
        assert_eq!(c[1], (1_793_829_900, 80_050.0, 80_200.0, 80_000.0, 80_150.0));
        let j = join(&c, &parse_oi(bn), 300);
        assert_eq!(j.len(), 2);
        assert_eq!(j[0].oi, 7_743_519_821.0, "OI à la clôture de la première bougie");
        assert!(join(&c, &[], 300).is_empty(), "sans OI : rien");
        let hl = r#"[{"universe":[{"name":"ETH"},{"name":"BTC"}]},[{"openInterest":"1","markPx":"2"},{"openInterest":"10","markPx":"80000"}]]"#;
        assert_eq!(parse_hl_oi(hl, "BTC"), Some(800_000.0));
        assert_eq!(step_for(24), 300);
        assert_eq!(step_for(168), 3600);
    }
}
