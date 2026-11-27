//! 🧮 Symboles calculés gratuits, sans clé : XAUGBP, XAUEUR, XAGEUR et le DXY.
//!
//! - **Or / argent en livre ou en euro** : l'or (ou l'argent) en dollars ÷ la livre (ou l'euro) en dollars, bougie par
//!   bougie (même heure d'ouverture), avec les marchés Hyperliquid HIP-3 déjà dans Argos (`xyz:GOLD`, `xyz:SILVER`,
//!   `xyz:GBP`, `xyz:EUR`). Avec un jeton OANDA sur le desktop, l'app prend plutôt les vrais `XAU_GBP`…
//! - **DXY** : le vrai indice (Yahoo `DX-Y.NYB` : direct sur le desktop, relais `/api/quote` du site dans le
//!   navigateur). Si Yahoo ne répond pas (téléphone sur le serveur local, sans relais), un **DXY calculé,
//!   approximatif** avec la formule de l'ICE et 5 devises de Kraken (EUR, JPY, GBP, CAD, CHF) : aucune source gratuite
//!   d'Argos n'a la couronne suédoise, elle est donc suivie à travers l'euro (`EURSEK_REF`).

use argos_core::Bar;

/// (symbole, nom, instrument OANDA natif, jambes Hyperliquid HIP-3 « numérateur ÷ dénominateur »).
pub const SYNTH: [(&str, &str, Option<&str>, Option<(&str, &str)>); 4] = [
    ("XAUGBP", "Or / Livre", Some("XAU_GBP"), Some(("xyz:GOLD", "xyz:GBP"))),
    ("XAUEUR", "Or / Euro", Some("XAU_EUR"), Some(("xyz:GOLD", "xyz:EUR"))),
    ("XAGEUR", "Argent / Euro", Some("XAG_EUR"), Some(("xyz:SILVER", "xyz:EUR"))),
    ("DXY", "Dollar US (indice DXY)", None, None),
];

/// Formule officielle du DXY (ICE) : constante, puis (paire Kraken, exposant) pour EURUSD, USDJPY, GBPUSD, USDCAD,
/// USDCHF. USDSEK (exposant 0,042) n'a pas de source gratuite : voir `EURSEK_REF`.
pub const DXY_K: f64 = 50.14348112;
pub const DXY_LEGS: [(&str, f64); 5] = [("EURUSD", -0.576), ("USDJPY", 0.136), ("GBPUSD", -0.119), ("USDCAD", 0.091), ("USDCHF", 0.036)];
pub const SEK_EXP: f64 = 0.042;
/// EUR/SEK de référence (Yahoo `EURSEK=X`, 11,0128 le 2026-11-05) : USDSEK ≈ EURSEK_REF ÷ EURUSD. Une dérive de 5 %
/// de l'EUR/SEK ne déplace le DXY calculé que d'environ 0,2 point (exposant 0,042), d'où « approximatif ».
pub const EURSEK_REF: f64 = 11.0128;

/// Identifiant de watchlist : « synth:XAUGBP ».
pub fn id(sym: &str) -> String {
    format!("synth:{sym}")
}

/// Relit `id` : le symbole s'il est connu.
pub fn parse_id(id: &str) -> Option<&'static str> {
    let s = id.strip_prefix("synth:")?;
    SYNTH.iter().find(|x| x.0 == s).map(|x| x.0)
}

/// Nom affiché d'un symbole calculé.
pub fn name(sym: &str) -> &'static str {
    SYNTH.iter().find(|x| x.0 == sym).map_or("", |x| x.1)
}

/// Instrument OANDA natif (préféré avec un jeton).
pub fn oanda_of(sym: &str) -> Option<&'static str> {
    SYNTH.iter().find(|x| x.0 == sym).and_then(|x| x.2)
}

/// Jambes Hyperliquid HIP-3 (numérateur, dénominateur).
pub fn legs(sym: &str) -> Option<(&'static str, &'static str)> {
    SYNTH.iter().find(|x| x.0 == sym).and_then(|x| x.3)
}

/// Symboles calculés qui répondent à la recherche (« xau », « gbp », « dxy », « or », « dollar »…).
pub fn search(q: &str) -> Vec<&'static str> {
    let q = q.trim().to_lowercase().replace(['/', ' ', '_'], "");
    if q.len() < 2 {
        return vec![];
    }
    SYNTH
        .iter()
        .filter(|x| x.0.to_lowercase().contains(&q) || x.1.to_lowercase().replace([' ', '/'], "").contains(&q))
        .map(|x| x.0)
        .collect()
}

/// DXY calculé à partir des bougies des 5 paires (dans l'ordre de `DXY_LEGS`), alignées à la même heure.
/// Ouverture et clôture : la formule sur les ouvertures et les clôtures ; haut et bas : chaque paire prend son haut ou
/// son bas selon le signe de son exposant, puis élargis pour contenir l'ouverture et la clôture. Pas de volume.
pub fn dxy_bars(legs: &[Vec<Bar>; 5]) -> Vec<Bar> {
    use std::collections::HashMap;
    let maps: Vec<HashMap<i64, &Bar>> = legs.iter().map(|l| l.iter().map(|b| (b.time, b)).collect()).collect();
    let f = |pick: &dyn Fn(usize, &Bar) -> f64, bars: &[&Bar]| -> Option<f64> {
        let mut v = DXY_K;
        let mut eur = 0.0;
        for (k, b) in bars.iter().enumerate() {
            let x = pick(k, b);
            if !(x.is_finite() && x > 0.0) {
                return None;
            }
            v *= x.powf(DXY_LEGS[k].1);
            if k == 0 {
                eur = x;
            }
        }
        Some(v * (EURSEK_REF / eur).powf(SEK_EXP))
    };
    let mut out: Vec<Bar> = legs[0]
        .iter()
        .filter_map(|b0| {
            let bars: Vec<&Bar> = maps.iter().map(|m| m.get(&b0.time).copied()).collect::<Option<_>>()?;
            // l'exposant de l'euro inclut celui de la couronne (suivie à travers l'euro) : négatif dans les deux cas
            let up = |k: usize| DXY_LEGS[k].1 > 0.0;
            let open = f(&|_, b| b.open, &bars)?;
            let close = f(&|_, b| b.close, &bars)?;
            let hi = f(&|k, b| if up(k) { b.high } else { b.low }, &bars)?;
            let lo = f(&|k, b| if up(k) { b.low } else { b.high }, &bars)?;
            Some(Bar {
                time: b0.time,
                open,
                high: hi.max(open).max(close),
                low: lo.min(open).min(close),
                close,
                volume: 0.0,
            })
        })
        .collect();
    out.sort_by_key(|b| b.time);
    out
}

/// Bougies d'une réponse Yahoo (`chart.result[0]`, desktop) : trous (null) laissés de côté.
pub fn parse_yahoo_bars(raw: &str) -> Vec<Bar> {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(raw) else {
        return vec![];
    };
    let r = &v["chart"]["result"][0];
    let (Some(ts), q) = (r["timestamp"].as_array(), &r["indicators"]["quote"][0]) else {
        return vec![];
    };
    let col = |k: &str, i: usize| q[k][i].as_f64().filter(|x| x.is_finite());
    ts.iter()
        .enumerate()
        .filter_map(|(i, t)| {
            Some(Bar {
                time: t.as_i64()?,
                open: col("open", i)?,
                high: col("high", i)?,
                low: col("low", i)?,
                close: col("close", i)?,
                volume: col("volume", i).unwrap_or(0.0),
            })
        })
        .collect()
}

/// Bougies d'une réponse du relais du site (`{bars:[[t,o,h,l,c,v]]}`).
pub fn parse_relay_bars(raw: &str) -> Vec<Bar> {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(raw) else {
        return vec![];
    };
    v["bars"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|b| {
            Some(Bar {
                time: b[0].as_i64()?,
                open: b[1].as_f64()?,
                high: b[2].as_f64()?,
                low: b[3].as_f64()?,
                close: b[4].as_f64()?,
                volume: b[5].as_f64().unwrap_or(0.0),
            })
        })
        .collect()
}

/// Intervalle Yahoo pour une unité de temps (les autres sont refaites à partir de la plus proche plus petite),
/// et la période demandée.
pub fn yahoo_iv(tf_s: i64) -> (&'static str, &'static str, i64) {
    match tf_s {
        s if s < 300 => ("1m", "7d", 60),
        s if s < 900 => ("5m", "60d", 300),
        s if s < 1800 => ("15m", "60d", 900),
        s if s < 3600 => ("30m", "60d", 1800),
        s if s < 86_400 => ("60m", "730d", 3600),
        s if s < 604_800 => ("1d", "10y", 86_400),
        _ => ("1wk", "max", 604_800),
    }
}

/// 🌐 Vrai DXY (Yahoo) à l'unité `tf_s` : desktop en direct, navigateur par le relais du site.
#[cfg(any(feature = "native", feature = "web"))]
pub async fn dxy_yahoo(tf_s: i64) -> argos_core::Result<Vec<Bar>> {
    use crate::http::Http;
    let (iv, range, src) = yahoo_iv(tf_s);
    let c = crate::rt::Client::default();
    let bars = if crate::direct() {
        let url = format!("https://query1.finance.yahoo.com/v8/finance/chart/DX-Y.NYB?interval={iv}&range={range}");
        parse_yahoo_bars(&c.get(&url).await.map_err(|e| e.to_string())?)
    } else {
        parse_relay_bars(&c.get(&format!("/api/quote?sym=DXY&iv={iv}")).await.map_err(|e| e.to_string())?)
    };
    if bars.is_empty() {
        return Err("Yahoo ne répond pas".into());
    }
    Ok(if tf_s > src { argos_core::aggregate(&bars, tf_s) } else { bars })
}

/// Unité Kraken (minutes) la plus proche sous `tf_s` : Kraken a 1, 5, 15, 30, 60, 240, 1440 et 10080 minutes.
pub fn kraken_minutes(tf_s: i64) -> i64 {
    [10_080, 1440, 240, 60, 30, 15, 5, 1].into_iter().find(|m| m * 60 <= tf_s).unwrap_or(1)
}

/// 🧮 DXY calculé (5 paires Kraken, couronne suivie à travers l'euro) à l'unité `tf_s`.
#[cfg(any(feature = "native", feature = "web"))]
pub async fn dxy_calc(tf_s: i64) -> argos_core::Result<Vec<Bar>> {
    let minutes = kraken_minutes(tf_s);
    use crate::http::Http;
    let c = crate::rt::Client::default();
    let mut legs: [Vec<Bar>; 5] = Default::default();
    for (k, (pair, _)) in DXY_LEGS.iter().enumerate() {
        let url = format!("https://api.kraken.com/0/public/OHLC?pair={pair}&interval={minutes}");
        let body = c.get(&url).await.map_err(|e| format!("{pair} : {e}"))?;
        legs[k] = crate::exchanges::kraken::parse_candles(&body).map_err(|e| format!("{pair} : {e}"))?;
    }
    let bars = dxy_bars(&legs);
    if bars.is_empty() {
        return Err("les 5 devises n'ont aucune bougie à la même heure".into());
    }
    Ok(if tf_s > minutes * 60 { argos_core::aggregate(&bars, tf_s) } else { bars })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar(t: i64, c: f64) -> Bar {
        Bar {
            time: t,
            open: c,
            high: c,
            low: c,
            close: c,
            volume: 1.0,
        }
    }

    #[test]
    fn dxy_formule_proche_du_vrai() {
        // prix Kraken et Yahoo du 2026-11-05 (DXY réel 101,566)
        let px = [1.12209, 159.641, 1.34537, 1.40206, 0.81573];
        let legs: [Vec<Bar>; 5] = std::array::from_fn(|k| vec![bar(100, px[k]), bar(200, px[k])]);
        let d = dxy_bars(&legs);
        assert_eq!(d.len(), 2);
        assert!((d[0].close - 101.566).abs() < 0.5, "DXY calculé {} trop loin du vrai", d[0].close);
        // une heure absente d'une paire : la bougie n'est pas faite
        let mut legs2 = legs.clone();
        legs2[3].remove(0);
        assert_eq!(dxy_bars(&legs2).len(), 1);
    }

    #[test]
    fn xaugbp_or_divise_par_la_livre_bougie_par_bougie() {
        // prix HIP-3 du 2026-11-05 : or 4090, livre 1,3457 → XAUGBP ≈ 3039,3
        let gold = vec![bar(100, 4090.0), bar(200, 4100.0), bar(300, 4110.0)];
        let gbp = vec![bar(100, 1.3457), bar(300, 1.35)];
        let x = crate::tv::ratio_bars(&gold, &gbp);
        assert_eq!(x.len(), 2, "seules les heures présentes des deux côtés");
        assert!((x[0].close - 4090.0 / 1.3457).abs() < 1e-9);
        assert_eq!((x[1].time, x[1].volume), (300, 0.0), "pas de volume pour une paire calculée");
        assert!(x.iter().all(|b| b.low <= b.open.min(b.close) && b.high >= b.open.max(b.close)));
    }

    #[test]
    fn ids_recherche_et_jambes() {
        assert_eq!(parse_id(&id("XAUGBP")), Some("XAUGBP"));
        assert_eq!(parse_id("synth:BTC"), None);
        assert_eq!(legs("XAGEUR"), Some(("xyz:SILVER", "xyz:EUR")));
        assert_eq!(oanda_of("XAUEUR"), Some("XAU_EUR"));
        assert_eq!(search("xau"), ["XAUGBP", "XAUEUR"]);
        assert_eq!(search("dxy"), ["DXY"]);
        assert_eq!(search("xau/gbp"), ["XAUGBP"]);
        assert!(search("x").is_empty());
        let y = r#"{"chart":{"result":[{"timestamp":[1,2],"indicators":{"quote":[{"open":[1.0,null],"high":[2.0,2.0],"low":[0.5,1.0],"close":[1.5,1.5],"volume":[0,0]}]}}]}}"#;
        assert_eq!(parse_yahoo_bars(y).len(), 1, "trou laissé de côté");
        assert_eq!(parse_relay_bars(r#"{"bars":[[1,1,2,0.5,1.5,0]]}"#)[0].high, 2.0);
        assert_eq!(yahoo_iv(14_400), ("60m", "730d", 3600), "4 h : refaite à partir de l'heure");
        assert_eq!((kraken_minutes(14_400), kraken_minutes(7200), kraken_minutes(60)), (240, 60, 1));
    }
}
