//! Backtest de conformité : le Cipher Rust doit donner les mêmes vagues et les mêmes points que le Cipher du site
//! (fixture générée par le code JavaScript de `src/10-createur-d-indicateurs.js`, 600 bougies).
//! Le site démarre ses moyennes un peu autrement que Pine au tout début : on compare après 200 bougies de mise en route.

use argos_core::Bar;
use argos_script::indicators::cipher::{compute, Settings};

const WARMUP: usize = 200;

fn load() -> (Vec<Bar>, serde_json::Value) {
    let raw: serde_json::Value = serde_json::from_str(include_str!("fixtures/cipher_js.json")).unwrap();
    let bars = raw["bars"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| Bar {
            time: b["time"].as_i64().unwrap(),
            open: b["open"].as_f64().unwrap(),
            high: b["high"].as_f64().unwrap(),
            low: b["low"].as_f64().unwrap(),
            close: b["close"].as_f64().unwrap(),
            volume: b["volume"].as_f64().unwrap(),
        })
        .collect();
    (bars, raw)
}

fn same_series(name: &str, ours: &[f64], js: &serde_json::Value) {
    for (i, v) in ours.iter().enumerate().skip(WARMUP) {
        let j = js[i].as_f64().unwrap_or(f64::NAN);
        assert!((v - j).abs() < 1e-6 * (1.0 + j.abs()), "{name} différent à la bougie {i} : Rust {v} / site {j}");
    }
}

fn same_events(name: &str, ours: &[usize], js: &serde_json::Value) {
    same_events_but(name, ours, js, &[], &[]);
}

/// B-027: the site looked for troughs with plain 5-bar windows; Pine (so TradingView) hides from the trough pattern
/// the bar where a top is confirmed (`f_top_fractal(src) ? 1 : f_bot_fractal(src) ? -1 : 0`). These points differ on
/// purpose and nowhere else (the site's list is the plain-window finder's, bar for bar); TradingView itself is checked
/// in `cipher_same_as_tradingview.rs`.
const PINE_RULE_DIV_BULL: (&[usize], &[usize]) = (&[283], &[276, 279, 433, 451, 474]);
const PINE_RULE_GOLD: (&[usize], &[usize]) = (&[441], &[435, 453]);

/// Like `same_events`, but `only_ours` and `only_site` are the points that must differ, and the only ones.
fn same_events_but(name: &str, ours: &[usize], js: &serde_json::Value, only_ours: &[usize], only_site: &[usize]) {
    let mut js: Vec<usize> = js
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_u64().unwrap() as usize)
        .filter(|&i| i >= WARMUP + 20)
        .collect();
    let ours: Vec<usize> = ours.iter().copied().filter(|&i| i >= WARMUP + 20).collect();
    assert!(
        only_site.iter().all(|i| js.contains(i)) && only_ours.iter().all(|i| !js.contains(i)),
        "{name} : écarts attendus"
    );
    js.retain(|i| !only_site.contains(i));
    js.extend(only_ours);
    js.sort_unstable();
    assert_eq!(ours, js, "{name} : points différents");
}

#[test]
fn cipher_identique_au_site() {
    let (bars, js) = load();
    let c = compute(&bars, &Settings::default());
    same_series("WT1", &c.wt1, &js["wt1"]);
    same_series("WT2", &c.wt2, &js["wt2"]);
    same_series("RSI", &c.rsi, &js["rsi"]);
    same_series("RSI+MFI", &c.mfi, &js["mfi"]);
    same_series("Stoch K", &c.stoch_k, &js["k"]);
    same_series("Stoch D", &c.stoch_d, &js["d"]);
    same_events("croisements haussiers", &c.cross_up, &js["buy"]);
    same_events("croisements baissiers", &c.cross_down, &js["sell"]);
    same_events("achats en zone", &c.buy, &js["bz"]);
    same_events("ventes en zone", &c.sell, &js["sz"]);
    let (ours_only, site_only) = PINE_RULE_DIV_BULL;
    same_events_but("points de divergence haussière", &c.div_bull_dots, &js["divb"], ours_only, site_only);
    same_events("points de divergence baissière", &c.div_bear_dots, &js["divs"]);
    let (ours_only, site_only) = PINE_RULE_GOLD;
    same_events_but("Golden dots", &c.gold, &js["gold"], ours_only, site_only);
    same_events("Purple dots", &c.purple, &js["purple"]);
}

/// Confluence /4 comme le Pine TradingView : à chaque creux (sommet) MFI confirmé, nombre de divergences régulières
/// WT + RSI + Stoch + MFI confirmées sur la même bougie ; étiquette au pivot dès « Confluence Min. » ; alertes à 2+/4.
#[test]
fn cipher_confluence_comme_tradingview() {
    let (bars, _) = load();
    let c = compute(&bars, &Settings::default());
    let n = c.wt2.len();
    let count = |i: usize, bull: bool| {
        [&c.wt_divs, &c.rsi_divs, &c.stoch_divs, &c.mfi_divs]
            .iter()
            .filter(|d| (if bull { &d.bull } else { &d.bear }).iter().any(|x| x.confirmed == i))
            .count()
    };
    for i in 0..n {
        assert_eq!(c.bull_count[i] as usize, count(i, true), "bougie {i} : bullDivCount");
        assert_eq!(c.bear_count[i] as usize, count(i, false), "bougie {i} : bearDivCount");
    }
    let want: Vec<(usize, usize)> = c
        .mfi_divs
        .bottoms
        .iter()
        .filter(|&&j| j + 2 < n && count(j + 2, true) >= 2)
        .map(|&j| (j, count(j + 2, true)))
        .collect();
    assert_eq!(c.confluence_buy, want);
    let alerts = c.alerts();
    assert_eq!(alerts.len(), 9);
    let conf_bull = &alerts[7].1;
    assert!((0..n).all(|i| (conf_bull[i] == 1.0) == (count(i, true) >= 2)));
    assert_eq!(alerts[0].1.iter().filter(|v| **v == 1.0).count(), c.buy.len());
}
