//! Croisement EMA, EMA multiples and Argos TVD Pro the same as TradingView's, bar for bar, from the first bar, on
//! Jee's export of BINANCE:BTCUSDT 1D (`fixtures/binance_btcusdt_1d.csv`, 1 187 bars) and on the 504 HYPE 50 min bars
//! of the old site's tests (`fixtures/hype_hl_50m.json`).
//!
//! The reference is `ta.ema` as Argos Script runs it: it starts on the simple mean of its first `length` values, as
//! TradingView's does (the same start as the MACD of B-093, whose values were redone apart, in Python, from TradingView's
//! definition). Argos Script computes it bar after bar (`Smooth` in `lang::interp`), not with the series function the
//! indicators take (`ta::ema`): two separate pieces of code. For Argos TVD Pro, the reference is Jee's own Pine script
//! of it (`pine/CCQc_TVD_Pro.pine`), run as it is. Before B-096 to B-098, these three indicators started their
//! averages on the first value (the EMA of the first Argos, `classic::ema`), as the MACD did before B-093.
//!
//! A third chart, Jee's bars with two flat stretches, holds what a market where nobody trades gives: TVD Pro's AI mix
//! with holes (B-101) and an AI Index that holds its value a few bars (B-102).

use argos_core::Bar;
use argos_script::indicators::{classic, tvd};
use argos_script::inputs::defaults;
use argos_script::lang;

fn bars() -> Vec<Bar> {
    let mut lines = include_str!("fixtures/binance_btcusdt_1d.csv").lines();
    assert_eq!(lines.next(), Some("time,open,high,low,close"));
    lines
        .enumerate()
        .map(|(k, l)| {
            let v: Vec<f64> = l.split(',').map(|x| x.parse().unwrap()).collect();
            Bar {
                time: v[0] as i64,
                open: v[1],
                high: v[2],
                low: v[3],
                close: v[4],
                // the export has no volume: a volume that moves, so that TVD Pro's vector candles (volume against its
                // 20-bar mean) say something
                volume: 1000.0 + ((k * 7919) % 5000) as f64,
            }
        })
        .collect()
}

/// The 504 HYPE 50 min bars of the old site's tests.
fn hype() -> Vec<Bar> {
    let raw: serde_json::Value = serde_json::from_str(include_str!("fixtures/hype_hl_50m.json")).unwrap();
    raw.as_array()
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
        .collect()
}

/// Jee's bars with two stretches of 10 flat bars (open = high = low = close, as a market where nobody trades): bars 4
/// to 13, and 600 to 609. Over a flat window TVD Pro's Stochastic has no range and is empty, so its AI mix has a hole
/// at bars 13 to 15, inside the first values its AI Index starts on (B-101), and another one later.
fn flat_stretches() -> Vec<Bar> {
    let mut b = bars();
    for (from, to) in [(4, 13), (600, 609)] {
        let p = b[from - 1].close;
        for x in &mut b[from..=to] {
            (x.open, x.high, x.low, x.close) = (p, p, p, p);
        }
    }
    b
}

/// The charts, by name.
fn charts() -> [(&'static str, Vec<Bar>); 3] {
    [
        ("BTCUSDT 1D", bars()),
        ("HYPE 50 min", hype()),
        ("BTCUSDT 1D with flat stretches", flat_stretches()),
    ]
}

fn run(src: &str, bars: &[Bar]) -> lang::Output {
    lang::run(src, bars, 86_400, &Default::default()).unwrap_or_else(|e| panic!("{e}"))
}

fn plot<'a>(out: &'a lang::Output, title: &str) -> &'a [f64] {
    &out.plots.iter().find(|p| p.title == title).unwrap_or_else(|| panic!("plot {title}")).values
}

/// Every bar the same: both empty, or equal to 1e-9 (relative). Writes into `report` how many bars differ and the
/// first ones, so that one run shows every series that is off.
fn same(report: &mut Vec<String>, what: &str, got: &[f64], want: &[f64]) {
    assert_eq!(got.len(), want.len(), "{what}: lengths");
    let off: Vec<usize> = (0..got.len())
        .filter(|&i| match (got[i].is_finite(), want[i].is_finite()) {
            (true, true) => (got[i] - want[i]).abs() > 1e-9 * want[i].abs().max(1.0),
            (a, b) => a != b,
        })
        .collect();
    if let Some(&i) = off.first() {
        report.push(format!(
            "{what}: {} bars differ from the reference, the first {:?} (bar {i}: Argos {}, reference {})",
            off.len(),
            &off[..off.len().min(8)],
            got[i],
            want[i]
        ));
    }
}

/// Fails with every line of `report`.
fn none(report: Vec<String>) {
    assert!(report.is_empty(), "\n{}", report.join("\n"));
}

/// The bars where a 0 / 1 plot is 1.
fn ones(v: &[f64]) -> Vec<usize> {
    (0..v.len()).filter(|&i| v[i] == 1.0).collect()
}

/// TradingView's EMA Cross: two `ta.ema` of the close and the bars where they cross. Before (B-096), Argos started
/// both averages on the first close: they differed from TradingView's at the first bars, and so could the crossings.
#[test]
fn ema_cross_like_tradingview_from_the_first_bar() {
    let mut report = Vec::new();
    for (chart, bars) in charts() {
        for (f, s) in [(9usize, 21usize), (20, 50)] {
            let tv = run(
                &format!(
                    "//@version=6
indicator('EMA Cross as TradingView', overlay=true)
fast = ta.ema(close, {f})
slow = ta.ema(close, {s})
plot(fast, 'fast')
plot(slow, 'slow')
plot(ta.crossover(fast, slow) ? 1 : 0, 'up')
plot(ta.crossunder(fast, slow) ? 1 : 0, 'down')
"
                ),
                &bars,
            );
            let e = classic::ema_cross_n(&bars, f, s);
            let what = format!("{chart}, EMA Cross {f}/{s}");
            same(&mut report, &format!("{what}: fast"), &e.fast, plot(&tv, "fast"));
            same(&mut report, &format!("{what}: slow"), &e.slow, plot(&tv, "slow"));
            for (way, got, want) in [("up", &e.up, ones(plot(&tv, "up"))), ("down", &e.down, ones(plot(&tv, "down")))] {
                if *got != want {
                    report.push(format!("{what}: the crossings {way}, Argos {got:?}, TradingView {want:?}"));
                }
            }
        }
    }
    none(report);
}

/// EMA multiples: up to six `ta.ema` of the close (or of hl2). Before (B-097), each started on the first close and
/// was hidden until its length: from there, still apart from TradingView's for tens to hundreds of bars.
#[test]
fn ema_multiples_like_tradingview_from_the_first_bar() {
    let mut report = Vec::new();
    let mut vals = defaults(&argos_script::v1_inputs::EMA_MULTI_PARAMS);
    // the four of the defaults (13, 50, 100, 200), and the 5th and 6th set too
    vals[4] = 9.0;
    vals[5] = 21.0;
    for (chart, bars) in charts() {
        for hl2 in [false, true] {
            vals[6] = if hl2 { 1.0 } else { 0.0 };
            let src = if hl2 { "hl2" } else { "close" };
            let tv = run(
                &format!(
                    "//@version=6
indicator('EMA multiples as TradingView', overlay=true)
plot(ta.ema({src}, 13), 'e1')
plot(ta.ema({src}, 50), 'e2')
plot(ta.ema({src}, 100), 'e3')
plot(ta.ema({src}, 200), 'e4')
plot(ta.ema({src}, 9), 'e5')
plot(ta.ema({src}, 21), 'e6')
"
                ),
                &bars,
            );
            let m = classic::ema_multiples(&bars, &vals);
            assert_eq!(m.len(), 6);
            for (k, e) in m.iter().enumerate() {
                let len = [13, 50, 100, 200, 9, 21][k];
                let what = format!("{chart}, EMA multiples ({src}): EMA {len}");
                same(&mut report, &what, e, plot(&tv, &format!("e{}", k + 1)));
            }
        }
    }
    none(report);
}

/// `ta::ema` and `ta::rma`, the averages of the indicators, skip a missing value among their first values, as `ta.ema`
/// and `ta.rma` of Argos Script do. Before (B-101), one missing value there left the average empty on the whole chart.
#[test]
fn ta_ema_and_rma_skip_a_gap_in_their_first_values_as_argos_script() {
    let mut report = Vec::new();
    for (chart, bars) in charts() {
        // the close with holes: two among the first values, one later
        let holes = [1usize, 3, 100];
        let x: Vec<f64> = bars
            .iter()
            .enumerate()
            .map(|(i, b)| if holes.contains(&i) { f64::NAN } else { b.close })
            .collect();
        let script = run(
            "//@version=6
indicator('averages with holes')
x = bar_index == 1 or bar_index == 3 or bar_index == 100 ? na : close
plot(ta.ema(x, 9), 'ema 9')
plot(ta.ema(x, 2), 'ema 2')
plot(ta.rma(x, 14), 'rma 14')
",
            &bars,
        );
        let ta = argos_script::ta::ema;
        same(&mut report, &format!("{chart}: ta::ema 9 with holes"), &ta(&x, 9), plot(&script, "ema 9"));
        same(&mut report, &format!("{chart}: ta::ema 2 with holes"), &ta(&x, 2), plot(&script, "ema 2"));
        same(
            &mut report,
            &format!("{chart}: ta::rma 14 with holes"),
            &argos_script::ta::rma(&x, 14),
            plot(&script, "rma 14"),
        );
    }
    none(report);
}

/// Jee's Pine script of Argos TVD Pro, run as it is, plus two plots of its AI Index and signal line.
fn tvd_pine(bars: &[Bar]) -> lang::Output {
    let src = format!(
        "{}\nplot(aiIndex, 'ai index')\nplot(aiSignal, 'ai signal')\n",
        include_str!("../../../../pine/CCQc_TVD_Pro.pine")
    );
    run(&src, bars)
}

/// Argos TVD Pro draws the six averages of its Pine script: the AI Index and its signal line (`ta.ema` of the AI
/// mix), and the EMA 13, 50, 100 and 200. Before (B-098), the native TVD Pro took the EMA of the first Argos, which
/// starts on the first value: apart from the Pine's at the first bars of every series, and, on Jee's chart, at all the
/// bars of the EMA 200.
#[test]
fn tvd_pro_averages_like_its_pine_script() {
    let mut report = Vec::new();
    for (chart, bars) in charts() {
        let pine = tvd_pine(&bars);
        let t = tvd::compute(&bars, &defaults(&tvd::PARAMS));
        for (what, got, title) in [
            ("EMA Fast (13)", &t.fast, "EMA Fast"),
            ("EMA Slow (50)", &t.slow, "EMA Slow"),
            ("EMA Mid (100)", &t.mid, "EMA Mid"),
            ("EMA Trend (200)", &t.trend, "EMA Trend"),
            ("AI Index", &t.ai, "ai index"),
            ("AI signal", &t.signal, "ai signal"),
        ] {
            same(&mut report, &format!("{chart}, TVD Pro: {what}"), got, plot(&pine, title));
        }
    }
    none(report);
}

/// Argos TVD Pro raises its alerts at the bars where Jee's Pine script raises them, all eleven (Trend, GOLD, AI,
/// vector candle, the four divergences): most of them read the averages above.
#[test]
fn tvd_pro_alerts_like_its_pine_script() {
    let mut report = Vec::new();
    for (chart, bars) in charts() {
        let pine = tvd_pine(&bars);
        let t = tvd::compute(&bars, &defaults(&tvd::PARAMS));
        assert_eq!(pine.alertconditions.len(), tvd::ALERTS.len());
        for (k, name) in tvd::ALERTS.iter().enumerate() {
            let (title, _, on) = &pine.alertconditions[k];
            assert_eq!(title, name, "the alerts in the same order");
            let native: Vec<usize> = t.alert_at.iter().filter(|a| a.1 == k).map(|a| a.0).collect();
            apart(&mut report, &format!("{chart}, TVD Pro alert « {name} »"), &native, &ones(on));
        }
    }
    none(report);
}

/// Argos TVD Pro puts its marks (Trend, AI, reversal, GOLD) at the bars where Jee's Pine script draws them.
#[test]
fn tvd_pro_marks_like_its_pine_script() {
    use tvd::MarkKind::*;
    let mut report = Vec::new();
    for (chart, bars) in charts() {
        let pine = tvd_pine(&bars);
        let t = tvd::compute(&bars, &defaults(&tvd::PARAMS));
        for (kind, title) in [
            (TrendBuy, "Trend BUY"),
            (TrendSell, "Trend SELL"),
            (AiBuy, "AI BUY"),
            (AiSell, "AI SELL"),
            (RevBull, "Bull Reversal"),
            (RevBear, "Bear Reversal"),
            (GoldSell, "GOLD SELL"),
            (GoldBuy, "GOLD BUY"),
        ] {
            let native: Vec<usize> = t.marks.iter().filter(|m| m.kind == kind).map(|m| m.index).collect();
            let want: Vec<usize> = pine.shapes.iter().filter(|s| s.title == title).map(|s| s.bar).collect();
            apart(&mut report, &format!("{chart}, TVD Pro mark « {title} »"), &native, &want);
        }
    }
    none(report);
}

/// Argos TVD Pro draws its divergences (line from the previous pivot, label at the new one) where Jee's Pine script
/// draws them, with the same ends. Its pivots are those of `ta.pivothigh` / `ta.pivotlow`: above (below) the bars before,
/// at least as high (low) as the bars after. Before (B-102), a tie on the right refused the pivot: on an AI Index that
/// holds its value a few bars (the flat stretch at bar 600 of the third chart), a hidden bullish divergence was missing.
#[test]
fn tvd_pro_divergences_like_its_pine_script() {
    use tvd::MarkKind::*;
    let mut report = Vec::new();
    for (chart, bars) in charts() {
        let pine = tvd_pine(&bars);
        let t = tvd::compute(&bars, &defaults(&tvd::PARAMS));
        let name = |k| match k {
            Bull => "Bull",
            HBull => "H Bull",
            Bear => "Bear",
            _ => "H Bear",
        };
        let mut native: Vec<String> = t
            .divs
            .iter()
            .map(|d| format!("{} {} {} → {} {}", name(d.kind), d.i0, d.p0, d.i1, d.p1))
            .collect();
        let lines: Vec<_> = pine.drawings.iter().filter(|d| d.kind == "line").collect();
        let labels: Vec<_> = pine.drawings.iter().filter(|d| d.kind == "label").collect();
        assert_eq!(lines.len(), labels.len(), "{chart}: one label for each line");
        let mut want: Vec<String> = lines
            .iter()
            .zip(&labels)
            .map(|(l, b)| {
                assert_eq!((l.x2, l.y2), (b.x1, b.y1), "{chart}: the label at the end of its line");
                format!("{} {} {} → {} {}", b.text, l.x1, l.y1, l.x2, l.y2)
            })
            .collect();
        native.sort();
        want.sort();
        let only_argos: Vec<&String> = native.iter().filter(|x| !want.contains(x)).collect();
        let only_pine: Vec<&String> = want.iter().filter(|x| !native.contains(x)).collect();
        if !only_argos.is_empty() || !only_pine.is_empty() {
            report.push(format!(
                "{chart}, TVD Pro divergences: in Argos only {only_argos:?}, in the Pine only {only_pine:?} (of {})",
                want.len()
            ));
        }
        assert!(want.len() > 20 || chart.starts_with("HYPE"), "{chart}: the Pine draws divergences");
    }
    none(report);
}

/// Writes into `report` the bars only Argos has and the bars only the Pine has, if any.
fn apart(report: &mut Vec<String>, what: &str, native: &[usize], want: &[usize]) {
    let only_argos: Vec<usize> = native.iter().filter(|i| !want.contains(i)).copied().collect();
    let only_pine: Vec<usize> = want.iter().filter(|i| !native.contains(i)).copied().collect();
    if !only_argos.is_empty() || !only_pine.is_empty() {
        report.push(format!(
            "{what}: {} bars in Argos only {:?}, {} in the Pine only {:?} (of {} in the Pine)",
            only_argos.len(),
            &only_argos[..only_argos.len().min(8)],
            only_pine.len(),
            &only_pine[..only_pine.len().min(8)],
            want.len()
        ));
    }
}
