//! 🌍 Marchés macro (module de la colonne) : S&P 500, Nasdaq 100, Dow Jones, dollar (DXY), or, pétrole, taux US 10 ans
//! et VIX — dernier prix, variation du jour et courbe des 5 derniers jours. Source : Yahoo Finance (desktop : direct ;
//! navigateur : relais du site `/api/quote`, Yahoo refusant les pages web). Lecture seule, aucune clé.

use serde_json::Value;

/// (clé du relais, symbole Yahoo, nom affiché).
pub const ASSETS: [(&str, &str, &str); 8] = [
    ("SPX500_USD", "^GSPC", "S&P 500"),
    ("NAS100_USD", "^NDX", "Nasdaq 100"),
    ("US30_USD", "^DJI", "Dow Jones"),
    ("DXY", "DX-Y.NYB", "Dollar (DXY)"),
    ("XAU_USD", "GC=F", "Or"),
    ("WTICO_USD", "CL=F", "Pétrole WTI"),
    ("USB10Y_USD", "^TNX", "Taux US 10 ans"),
    ("VIX", "^VIX", "VIX"),
];

/// Une tuile : nom, dernier prix, variation depuis la clôture de la veille (%), clôtures des 5 derniers jours.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Tile {
    pub name: String,
    pub price: f64,
    pub chg_pct: f64,
    pub spark: Vec<f64>,
}

/// Clôtures (heure, prix) d'une réponse Yahoo (`chart.result[0]`).
pub fn parse_yahoo(raw: &str) -> Vec<(i64, f64)> {
    let Ok(v) = serde_json::from_str::<Value>(raw) else { return vec![] };
    let r = &v["chart"]["result"][0];
    let (Some(ts), Some(cl)) = (r["timestamp"].as_array(), r["indicators"]["quote"][0]["close"].as_array()) else {
        return vec![];
    };
    ts.iter()
        .zip(cl)
        .filter_map(|(t, c)| Some((t.as_i64()?, c.as_f64().filter(|x| x.is_finite())?)))
        .collect()
}

/// Clôtures (heure, prix) d'une réponse du relais (`{bars:[[t,o,h,l,c,v]]}`).
pub fn parse_relay(raw: &str) -> Vec<(i64, f64)> {
    let Ok(v) = serde_json::from_str::<Value>(raw) else { return vec![] };
    v["bars"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|b| Some((b[0].as_i64()?, b[4].as_f64()?)))
        .collect()
}

/// Tuile tirée des clôtures : variation contre la dernière clôture d'un jour (UTC) avant celui du dernier prix.
pub fn tile(name: &str, closes: &[(i64, f64)]) -> Option<Tile> {
    let &(t, price) = closes.last()?;
    let day = t.div_euclid(86_400);
    let prev = closes
        .iter()
        .rev()
        .find(|(x, _)| x.div_euclid(86_400) < day)
        .map(|c| c.1)
        .unwrap_or(closes[0].1);
    let chg_pct = if prev != 0.0 { (price / prev - 1.0) * 100.0 } else { 0.0 };
    Some(Tile {
        name: name.into(),
        price,
        chg_pct,
        spark: closes.iter().map(|c| c.1).collect(),
    })
}

/// Toutes les tuiles (celles qui ne répondent pas sont laissées de côté).
#[cfg(any(feature = "native", feature = "web"))]
pub async fn tiles() -> argos_core::Result<Vec<Tile>> {
    use crate::http::Http;
    let c = crate::rt::Client::default();
    let mut out = vec![];
    for (key, sym, name) in ASSETS {
        let closes = if crate::direct() {
            let url = format!(
                "https://query1.finance.yahoo.com/v8/finance/chart/{}?interval=30m&range=5d",
                sym.replace('^', "%5E").replace('=', "%3D")
            );
            c.get(&url).await.map(|r| parse_yahoo(&r)).unwrap_or_default()
        } else {
            c.get(&format!("/api/quote?sym={key}&iv=spark"))
                .await
                .map(|r| parse_relay(&r))
                .unwrap_or_default()
        };
        if let Some(t) = tile(name, &closes) {
            out.push(t);
        }
    }
    if out.is_empty() {
        return Err("Yahoo Finance ne répond pas".into());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tuiles_macro() {
        let y = r#"{"chart":{"result":[{"timestamp":[1793000000,1793003600,1793090000],"indicators":{"quote":[{"close":[5000.0,5050.0,null]}]}}]}}"#;
        assert_eq!(parse_yahoo(y), [(1793000000, 5000.0), (1793003600, 5050.0)], "trou (null) laissé de côté");
        let r = r#"{"bars":[[1793000000,1,1,1,100.0,0],[1793090000,1,1,1,102.0,0]]}"#;
        let c = parse_relay(r);
        let t = tile("Or", &c).unwrap();
        assert_eq!((t.price, (t.chg_pct * 100.0).round() / 100.0, t.spark.len()), (102.0, 2.0, 2));
        assert!(tile("x", &[]).is_none());
    }
}
