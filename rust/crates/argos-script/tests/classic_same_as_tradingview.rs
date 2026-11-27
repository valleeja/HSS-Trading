//! MACD, Stochastic, Williams %R and Parabolic SAR the same as TradingView's, bar for bar, from the first bar, on
//! Jee's export of BINANCE:BTCUSDT 1D (`fixtures/binance_btcusdt_1d.csv`, 1 187 bars).
//!
//! Jee's captures and exports do not show these four, so the reference is TradingView's own definition of each,
//! written in Argos Script with the `ta.*` functions already checked against his exports (`ta.ema` in the Cipher and
//! TVD Pro, `ta.sma` in the Bollinger bands, `ta.stoch` in the Cipher, `ta.highest` / `ta.lowest` in Donchian), and,
//! for the SAR, the step-by-step algorithm the Pine reference manual gives for `ta.sar`: the reversal is read on the
//! SAR before it is held back by the two previous bars (Wilder and TA-Lib hold it back first).

use argos_core::Bar;
use argos_script::indicators::{classic, textbook};
use argos_script::lang;

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

fn run(src: &str, bars: &[Bar]) -> lang::Output {
    lang::run(src, bars, 86_400, &Default::default()).unwrap_or_else(|e| panic!("{e}"))
}

fn plot<'a>(out: &'a lang::Output, title: &str) -> &'a [f64] {
    &out.plots.iter().find(|p| p.title == title).unwrap_or_else(|| panic!("plot {title}")).values
}

/// Every bar the same: both empty, or equal to 1e-9 (relative). Says which bars differ.
fn same(what: &str, got: &[f64], want: &[f64]) {
    assert_eq!(got.len(), want.len(), "{what}: lengths");
    let off: Vec<usize> = (0..got.len())
        .filter(|&i| match (got[i].is_finite(), want[i].is_finite()) {
            (true, true) => (got[i] - want[i]).abs() > 1e-9 * want[i].abs().max(1.0),
            (a, b) => a != b,
        })
        .collect();
    assert!(
        off.is_empty(),
        "{what}: {} bars differ from TradingView, the first {:?} (Argos {:?}, TradingView {:?})",
        off.len(),
        &off[..off.len().min(8)],
        off.first().map(|&i| got[i]),
        off.first().map(|&i| want[i])
    );
}

/// TradingView's MACD: two `ta.ema` of the close, and the `ta.ema` of their gap. The line starts at the 26th bar,
/// the signal and the histogram at the 34th; before (B-093), Argos started its averages on the first close and drew
/// all three from the first bar: 268 bars apart from TradingView's on these bars, about 150 by more than half a cent.
#[test]
fn macd_like_tradingview_from_the_first_bar() {
    let bars = bars();
    let tv = run(
        "//@version=6
indicator('MACD as TradingView')
gap = ta.ema(close, 12) - ta.ema(close, 26)
signal = ta.ema(gap, 9)
plot(gap, 'line')
plot(signal, 'signal')
plot(gap - signal, 'hist')
",
        &bars,
    );
    let m = classic::macd_n(&bars, 12, 26, 9);
    same("MACD line", &m.line, plot(&tv, "line"));
    same("MACD signal", &m.signal, plot(&tv, "signal"));
    same("MACD histogram", &m.hist, plot(&tv, "hist"));
    assert!(m.line[24].is_nan() && m.line[25].is_finite());
    assert!(m.signal[32].is_nan() && m.signal[33].is_finite());
}

/// TradingView's Stochastic: `ta.stoch` over 14 bars, smoothed by `ta.sma`. Nothing before the 14th bar; before
/// (B-094), Argos took a shorter window there and drew 13 values TradingView does not have.
#[test]
fn stochastic_like_tradingview_from_the_first_bar() {
    let bars = bars();
    for smooth in [1usize, 3] {
        let tv = run(
            &format!(
                "//@version=6
indicator('Stochastic as TradingView')
k = ta.sma(ta.stoch(close, high, low, 14), {smooth})
plot(k, 'k')
plot(ta.sma(k, 3), 'd')
"
            ),
            &bars,
        );
        let (k, d) = classic::stochastic_n(&bars, 14, smooth, 3);
        same(&format!("Stochastic 14, {smooth}, 3: %K"), &k, plot(&tv, "k"));
        same(&format!("Stochastic 14, {smooth}, 3: %D"), &d, plot(&tv, "d"));
        assert!(k[12 + smooth - 1].is_nan() && k[13 + smooth - 1].is_finite());
    }
}

/// The chart's Stochastic starts with TradingView's settings, 14, 1, 3 (Jee's choice, 26 Nov 2026; it was 14, 3, 3):
/// added as it comes, it draws TradingView's curves.
#[test]
fn the_default_stochastic_is_tradingview_s() {
    use argos_script::classic_inputs::STOCH_PARAMS;
    use argos_script::inputs::{defaults, Inputs};
    let vals = defaults(&STOCH_PARAMS);
    let p = Inputs {
        params: &STOCH_PARAMS,
        vals: &vals,
    };
    assert_eq!((p.len("len"), p.len("smooth"), p.len("d")), (14, 1, 3));
    let bars = bars();
    let tv = run(
        "//@version=6
indicator(title='Stochastic', shorttitle='Stoch')
periodK = input.int(14, title='%K Length', minval=1)
smoothK = input.int(1, title='%K Smoothing', minval=1)
periodD = input.int(3, title='%D Smoothing', minval=1)
k = ta.sma(ta.stoch(close, high, low, periodK), smoothK)
plot(k, title='%K')
plot(ta.sma(k, periodD), title='%D')
",
        &bars,
    );
    let (k, d) = classic::stochastic_n(&bars, p.len("len"), p.len("smooth"), p.len("d"));
    same("Stochastic by default: %K", &k, plot(&tv, "%K"));
    same("Stochastic by default: %D", &d, plot(&tv, "%D"));
}

/// TradingView's Williams %R: 100 × (close − highest) / (highest − lowest) over 14 bars. Already the same before this
/// lot; this test keeps it so.
#[test]
fn williams_r_like_tradingview_from_the_first_bar() {
    let bars = bars();
    let tv = run(
        "//@version=6
indicator('Williams %R as TradingView')
hi = ta.highest(high, 14)
lo = ta.lowest(low, 14)
plot(100 * (close - hi) / (hi - lo), 'r')
",
        &bars,
    );
    same("Williams %R", &textbook::williams_r_n(&bars, 14), plot(&tv, "r"));
}

/// The step-by-step `ta.sar` of the Pine reference manual (0.02, 0.02, 0.2), and the `ta.sar` of Argos Script.
const SAR: &str = "//@version=5
indicator('Parabolic SAR, step by step as the Pine reference manual')
var float sar = na
var float ext = na
var float af = na
var bool below = false
bool fresh = false
if bar_index == 1
    if close > close[1]
        below := true
        ext := high
        sar := low[1]
    else
        below := false
        ext := low
        sar := high[1]
    fresh := true
    af := 0.02
sar := sar + af * (ext - sar)
if below
    if sar > low
        fresh := true
        below := false
        sar := math.max(high, ext)
        ext := low
        af := 0.02
else
    if sar < high
        fresh := true
        below := true
        sar := math.min(low, ext)
        ext := high
        af := 0.02
if not fresh
    if below
        if high > ext
            ext := high
            af := math.min(af + 0.02, 0.2)
    else
        if low < ext
            ext := low
            af := math.min(af + 0.02, 0.2)
if below
    sar := math.min(sar, low[1])
    if bar_index > 1
        sar := math.min(sar, low[2])
else
    sar := math.max(sar, high[1])
    if bar_index > 1
        sar := math.max(sar, high[2])
plot(sar, 'reference')
plot(ta.sar(0.02, 0.02, 0.2), 'builtin')
";

/// The SAR of the chart (« SAR parabolique ») follows TradingView's steps. Before (B-091), it was held back by the
/// two previous bars before the reversal was read, and after a reversal it started from the extreme alone: on these
/// bars it differed from TradingView at 228 bars, 45 of them on the other side of the price.
#[test]
fn parabolic_sar_like_tradingview_from_the_first_bar() {
    let bars = bars();
    let tv = run(SAR, &bars);
    same("SAR parabolique", &textbook::psar_n(&bars, 0.02, 0.02, 0.2), plot(&tv, "reference"));
}

/// `ta.sar` in a script (an imported Pine script) follows TradingView's steps too. Before (B-092), it was held back
/// by the previous bar only, before the reversal was read: 469 bars differed, 61 on the other side of the price.
#[test]
fn ta_sar_like_tradingview_from_the_first_bar() {
    let bars = bars();
    let tv = run(SAR, &bars);
    same("ta.sar", plot(&tv, "builtin"), plot(&tv, "reference"));
}

/// Wherever it is, TradingView's SAR is never inside a bar: under its low or above its high. Before (B-091, B-092),
/// a reversal bar that also made a new extreme got a SAR inside it.
#[test]
fn the_sar_never_sits_inside_a_bar() {
    let bars = bars();
    let tv = run(SAR, &bars);
    for (what, sar) in [
        ("SAR parabolique", textbook::psar_n(&bars, 0.02, 0.02, 0.2)),
        ("ta.sar", plot(&tv, "builtin").to_vec()),
    ] {
        let inside: Vec<usize> = (1..bars.len()).filter(|&i| sar[i] > bars[i].low && sar[i] < bars[i].high).collect();
        assert!(
            inside.is_empty(),
            "{what}: inside the bar at {} bars, the first {:?}",
            inside.len(),
            &inside[..inside.len().min(8)]
        );
    }
}
