//! AI Trend aligné sur le Pine « Ultimate Ai Trend All Div Pro » (TradingView) : 3000 bougies 30m réelles (fixture).
//! On vérifie le comportement Pine : preset Auto → Day Trade en 30m, ligne AI bornée 0–100, achats / ventes = croisements
//! de la ligne signal, divergences cohérentes, zones extrêmes au-delà du Very Red / Very Blue.

use argos_core::Bar;
use argos_script::indicators::ai_trend::{compute, LowerBars, Settings};
use serde_json::Value;

fn bars() -> Vec<Bar> {
    let raw: Value = serde_json::from_str(include_str!("fixtures/ai_trend_js.json")).unwrap();
    let f = |b: &Value, k: &str| b[k].as_f64().unwrap();
    raw["bars"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| Bar {
            time: b["time"].as_i64().unwrap(),
            open: f(b, "open"),
            high: f(b, "high"),
            low: f(b, "low"),
            close: f(b, "close"),
            volume: f(b, "volume"),
        })
        .collect()
}

#[test]
fn ai_trend_comme_pine() {
    let bars = bars();
    let s = Settings::default();
    let r = compute(&bars, 1800, &s, &LowerBars::new());
    assert_eq!(r.preset, "Day Trade");
    assert_eq!(r.ai.len(), bars.len());

    // ligne AI : bornée 0–100 dès qu'elle existe, et elle existe sur presque toute la série
    let ok: Vec<f64> = r.ai.iter().copied().filter(|v| !v.is_nan()).collect();
    assert!(ok.len() > bars.len() - 100);
    assert!(ok.iter().all(|v| (-1e-9..=100.0 + 1e-9).contains(v)));

    // achats / ventes : l'AI croise la ligne signal
    assert!(!r.buy.is_empty() && !r.sell.is_empty());
    for &i in &r.buy {
        assert!(r.ai[i] > r.signal[i] && r.ai[i - 1] <= r.signal[i - 1], "achat {i}");
    }
    for &i in &r.sell {
        assert!(r.ai[i] < r.signal[i] && r.ai[i - 1] >= r.signal[i - 1], "vente {i}");
    }

    // divergences : pivot précédent < pivot < confirmation
    for d in r.divs.bull.iter().chain(&r.divs.bear).chain(&r.divs.hidden_bull).chain(&r.divs.hidden_bear) {
        assert!(d.prev < d.pivot && d.pivot <= d.confirmed && d.confirmed < bars.len());
    }
    assert!(!r.divs.bull.is_empty() || !r.divs.bear.is_empty());

    // zones extrêmes (fond rouge / bleu)
    for &i in &r.extreme_high {
        assert!(r.ai[i] >= 85.0 - 1e-9);
    }
    for &i in &r.extreme_low {
        assert!(r.ai[i] <= 20.0 + 1e-9);
    }
    assert!(r.htf.is_some());
}

/// Comme le Pine : flèches (l'AI sort de la survente / du surachat, filtre EMA 9 / 21), ligne signal et divergences
/// cachées calculées même masquées, 17 alertes.
#[test]
fn ai_trend_alertes_comme_pine() {
    let bars = bars();
    let s = Settings {
        signal_show: false,
        div_hidden: false,
        ..Settings::default()
    };
    let r = compute(&bars, 1800, &s, &LowerBars::new());
    assert!(!r.buy.is_empty(), "croisements de la ligne signal même masquée");
    assert!(
        !r.divs.hidden_bull.is_empty() || !r.divs.hidden_bear.is_empty(),
        "cachées calculées même masquées"
    );
    let close: Vec<f64> = bars.iter().map(|b| b.close).collect();
    let (ef, es) = (argos_script::ta::ema(&close, 9), argos_script::ta::ema(&close, 21));
    let want: Vec<usize> = (1..bars.len()).filter(|&i| r.ai[i] > 25.0 && r.ai[i - 1] <= 25.0 && ef[i] > es[i]).collect();
    assert_eq!(r.arrow_buy, want);
    let al = r.alerts(&bars);
    assert_eq!(al.len(), 17);
    let ones = |k: usize| al[k].1.iter().filter(|v| **v == 1.0).count();
    assert_eq!(ones(10), r.buy.len());
    assert_eq!(ones(14), r.arrow_buy.len());
    assert_eq!(ones(6), r.divs.bull.len());
}
