//! The Cipher's points against TradingView itself, from Jee's export of the Cipher's Pine on BINANCE:BTCUSDT 1D
//! (25 Nov 2026, 1 187 closed days from 26 Aug 2023, the live day left out). Only the exchange's candles are kept here
//! (`fixtures/binance_btcusdt_1d.csv`) and, written below, the bars where TradingView drew each point: none of the
//! values TradingView computed is in the repository. TradingView draws each point with
//! `offset = -2`, so a bar below is the point's pivot bar. The export itself stays in Jee's folder, out of the repository.

use argos_core::Bar;
use argos_script::indicators::cipher::{compute, find_divs, Settings};
use std::collections::BTreeSet;

/// The bars (from 0, the file's first candle) where TradingView drew « Divergence buy circle ».
const TV_DIV_BUY: [usize; 33] = [
    11, 147, 250, 314, 316, 347, 378, 379, 493, 527, 543, 562, 563, 564, 591, 650, 732, 778, 802, 883, 894, 909, 913, 946, 1008, 1034, 1036, 1037, 1072, 1140,
    1147, 1163, 1182,
];
/// The bars where TradingView drew « Divergence sell circle ».
const TV_DIV_SELL: [usize; 57] = [
    25, 37, 42, 76, 78, 81, 86, 91, 92, 130, 187, 191, 198, 200, 219, 229, 270, 285, 286, 330, 338, 399, 421, 432, 454, 455, 466, 471, 473, 511, 514, 516, 524,
    545, 615, 624, 625, 631, 635, 636, 754, 773, 831, 874, 904, 971, 972, 974, 985, 1055, 1094, 1097, 1104, 1106, 1132, 1136, 1137,
];
/// The bars where TradingView drew « Gold buy gold circle ».
const TV_GOLD: [usize; 4] = [316, 562, 591, 1034];

/// From this bar on, the Cipher's averages have settled: its WT2, RSI and Stoch K equal TradingView's (checked against
/// the export: 0.000000), so the points can be compared.
const WARMUP: usize = 300;

fn bars() -> Vec<Bar> {
    let mut lines = include_str!("fixtures/binance_btcusdt_1d.csv").lines();
    assert_eq!(lines.next(), Some("time,open,high,low,close"));
    lines
        .map(|l| {
            let v: Vec<f64> = l.split(',').map(|x| x.parse().unwrap()).collect();
            Bar {
                time: v[0] as i64,
                open: v[1],
                high: v[2],
                low: v[3],
                close: v[4],
                volume: 1.0,
            }
        })
        .collect()
}

fn from(v: impl IntoIterator<Item = usize>, start: usize) -> BTreeSet<usize> {
    v.into_iter().filter(|&i| i >= start).collect()
}

/// The divergence finder alone, fed the Cipher's WT2, RSI and Stoch K: the same buy and sell dots as TradingView, bar
/// for bar. Pine runs the bottom pattern only on bars without a top pattern (`f_fractalize`'s ternary): plain 5-bar
/// windows gave buy dots TradingView does not have (411, 526, 542, 649, 1162) and missed some it has (B-027).
#[test]
fn divergence_dots_like_tradingview_on_its_waves() {
    let bars = bars();
    assert_eq!(bars.len(), 1187, "the candles of the export");
    let s = Settings::default();
    let c = compute(&bars, &s);
    let found = [
        find_divs(&c.wt2, &bars, s.wt_div_bear_min, s.wt_div_bull_min, true),
        find_divs(&c.wt2, &bars, s.wt_div2_bear, s.wt_div2_bull, true),
        find_divs(&c.rsi, &bars, s.rsi_div_bear_min, s.rsi_div_bull_min, true),
        find_divs(&c.stoch_k, &bars, 0.0, 0.0, false),
    ];
    let buy = from(found.iter().flat_map(|d| d.bull.iter().map(|x| x.pivot)), WARMUP);
    let sell = from(found.iter().flat_map(|d| d.bear.iter().map(|x| x.pivot)), WARMUP);
    assert_eq!(buy, from(TV_DIV_BUY, WARMUP), "Divergence buy circle");
    assert_eq!(sell, from(TV_DIV_SELL, WARMUP), "Divergence sell circle");
}

/// The whole Cipher from the candles: once its averages have settled, the same divergence dots and golden dots as
/// TradingView, each on the bar where TradingView draws it (the golden dot 2 bars before its signal, B-028).
#[test]
fn cipher_points_like_tradingview() {
    let c = compute(&bars(), &Settings::default());
    for (name, ours, theirs) in [
        ("Divergence buy circle", &c.div_bull_dots, &TV_DIV_BUY[..]),
        ("Divergence sell circle", &c.div_bear_dots, &TV_DIV_SELL[..]),
        ("Gold buy gold circle", &c.gold_dots, &TV_GOLD[..]),
    ] {
        let theirs = from(theirs.iter().copied(), WARMUP);
        assert!(!theirs.is_empty(), "{name}: TradingView has points after the warm-up");
        assert_eq!(from(ours.iter().copied(), WARMUP), theirs, "{name}");
    }
    // the signal itself stays on the bar where TradingView raises it (alerts, « il y a n barres »)
    assert!(c.gold.iter().zip(&c.gold_dots).all(|(g, d)| g - 2 == *d));
}
