//! ⚓ **Argos Anchored VWAP**, comme l'indicateur Anchored VWAP de TradingView (exemple de `input.time` avec
//! `confirm` dans la doc de Pine v6) : VWAP qui démarre à une bougie choisie sur la chart, avec jusqu'à 3 paires de
//! bandes (écart-type pondéré par le volume × multiplicateur).

use crate::inputs::{check, color, color_t, num, price_series, source, time, Inputs, Param, Src};
use argos_core::Bar;

pub const GROUPS: [&str; 2] = ["⚓ Ancrage", "🎨 Bandes et style"];
pub const PARAMS: [Param; 11] = [
    time("anchor", 0, "Ancrage", 0.0)
        .confirm()
        .tip("Bougie où le VWAP démarre.\nGlisse la ligne sur la chart, ou clique 📍."),
    source("src", 0, "Source", 5),
    check("b1", 1, "Bandes #1", true).inline("b1"),
    num("m1", 1, "× écart-type", 1.0, 0.1, 10.0, 0.1).inline("b1").active_if("b1"),
    check("b2", 1, "Bandes #2", false).inline("b2"),
    num("m2", 1, "× écart-type", 2.0, 0.1, 10.0, 0.1).inline("b2").active_if("b2"),
    check("b3", 1, "Bandes #3", false).inline("b3"),
    num("m3", 1, "× écart-type", 3.0, 0.1, 10.0, 0.1).inline("b3").active_if("b3"),
    color("cv", 1, "VWAP", 0x2962ff),
    color_t("cb", 1, "Bandes", 0x4caf50, 0),
    color_t("cf", 1, "Remplissage", 0x4caf50, 90),
];

/// Le VWAP depuis l'ancrage et ses bandes (`NaN` avant l'ancrage).
#[derive(Debug, Clone, PartialEq)]
pub struct AnchoredVwap {
    /// Bougie de l'ancrage.
    pub from: usize,
    pub vwap: Vec<f64>,
    /// Écart-type pondéré par le volume (les bandes = vwap ± m × écart-type).
    pub dev: Vec<f64>,
}

/// `ext` : série d'une source qui vient d'un autre indicateur (`Src::Ext`), sinon `None`.
pub fn compute(bars: &[Bar], vals: &[f64], ext: &dyn Fn(usize, usize) -> Option<Vec<f64>>) -> AnchoredVwap {
    let p = Inputs { params: &PARAMS, vals };
    let n = bars.len();
    let src = match p.source("src") {
        Src::Price(k) => price_series(bars, k),
        Src::Ext(i, o) => ext(i, o).filter(|s| s.len() == n).unwrap_or_else(|| price_series(bars, 5)),
    };
    // ancrage pas encore choisi : 200 bougies en arrière
    let anchor = p.get("anchor") as i64 / 1000;
    let from = if anchor <= 0 {
        n.saturating_sub(200)
    } else {
        bars.partition_point(|b| b.time < anchor).min(n.saturating_sub(1))
    };
    let (mut vwap, mut dev) = (vec![f64::NAN; n], vec![f64::NAN; n]);
    let (mut sv, mut spv, mut sp2v) = (0.0, 0.0, 0.0);
    for i in from..n {
        let (v, x) = (bars[i].volume.max(0.0), src[i]);
        if !x.is_finite() {
            continue;
        }
        sv += v;
        spv += x * v;
        sp2v += x * x * v;
        if sv > 0.0 {
            let m = spv / sv;
            vwap[i] = m;
            dev[i] = (sp2v / sv - m * m).max(0.0).sqrt();
        } else {
            vwap[i] = x;
            dev[i] = 0.0;
        }
    }
    AnchoredVwap { from, vwap, dev }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vwap_depuis_l_ancrage() {
        let bars: Vec<Bar> = (0..10)
            .map(|i| Bar {
                time: i * 60,
                open: 10.0,
                high: 10.0 + i as f64,
                low: 10.0,
                close: 10.0 + i as f64,
                volume: 1.0 + i as f64,
            })
            .collect();
        let mut v = crate::inputs::defaults(&PARAMS);
        v[0] = 300_000.0; // bougie de 300 s (index 5)
        let a = compute(&bars, &v, &|_, _| None);
        assert_eq!(a.from, 5);
        assert!(a.vwap[4].is_nan());
        // 1re bougie : vwap = son hlc3 ; ensuite pondéré par le volume
        let h = |i: usize| (bars[i].high + bars[i].low + bars[i].close) / 3.0;
        assert!((a.vwap[5] - h(5)).abs() < 1e-12 && a.dev[5] == 0.0);
        let w = (h(5) * 6.0 + h(6) * 7.0) / 13.0;
        assert!((a.vwap[6] - w).abs() < 1e-12);
        // source d'un autre indicateur
        v[1] = crate::inputs::source_ext(0, 0);
        let a = compute(&bars, &v, &|_, _| Some(vec![42.0; 10]));
        assert_eq!(a.vwap[9], 42.0);
        // pas d'ancrage choisi : 200 bougies en arrière (ici : depuis le début)
        assert_eq!(compute(&bars, &crate::inputs::defaults(&PARAMS), &|_, _| None).from, 0);
    }
}
