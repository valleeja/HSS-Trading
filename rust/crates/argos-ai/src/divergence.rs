//! 🤖 Idea 3: classic divergences between the price and an oscillator (RSI, MACD line, MFI). Bearish: the price makes a
//! higher high, the oscillator a lower high; bullish: the price makes a lower low, the oscillator a higher low. Only
//! closed bars and confirmed highs and lows; a divergence that is too old or that the price has already gone past
//! is not announced.

use crate::format::bars as n_bars;
use argos_core::Bar;
use argos_lens::explain::Sentence;

/// Bars on each side that confirm a high or a low: the high of bar `i` is above the highs of the `PIVOT_SIDE` bars
/// before and after it (likewise for lows). A high is therefore confirmed `PIVOT_SIDE` bars after it.
pub(crate) const PIVOT_SIDE: usize = 3;
/// How far back the two highs (or lows) are looked for, in closed bars.
pub(crate) const WINDOW: usize = 60;
/// A divergence whose last high (or low) is older than this, in bars, is no longer announced.
pub(crate) const STALE_AFTER: usize = 20;

/// A classic divergence found on the last closed bars.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Divergence {
    /// Bearish (higher high, lower oscillator) or bullish (lower low, higher oscillator).
    pub bearish: bool,
    /// Age in bars of the two highs (or lows), the older first.
    pub ages: (usize, usize),
    /// The price at the two highs (or lows).
    pub prices: (f64, f64),
    /// The oscillator at the same bars.
    pub values: (f64, f64),
    /// Bars since the last high (or low) was confirmed.
    pub confirmed: usize,
}

/// The bars whose `v` stands strictly above (`high`) or below its `PIVOT_SIDE` neighbours before it, and at least as
/// high (or as low) as its `PIVOT_SIDE` neighbours after it (a bar that opens where the high was does not hide it);
/// confirmed and inside the window.
fn pivots(v: &[f64], high: bool) -> Vec<usize> {
    let n = v.len();
    let beyond = |a: f64, b: f64| if high { a > b } else { a < b };
    let from = n.saturating_sub(WINDOW).max(PIVOT_SIDE);
    (from..n.saturating_sub(PIVOT_SIDE))
        .filter(|&i| (1..=PIVOT_SIDE).all(|k| beyond(v[i], v[i - k]) && !beyond(v[i + k], v[i])))
        .collect()
}

/// The divergence between the last two confirmed highs or lows of `bars` and `osc` (same length), if any is still
/// current: the more recent of the two kinds, not older than `STALE_AFTER`, and not gone past by the price since.
pub(crate) fn find(bars: &[Bar], osc: &[f64]) -> Option<Divergence> {
    let n = bars.len().min(osc.len());
    let (bars, osc) = (&bars[..n], &osc[..n]);
    let mut found: Vec<(usize, Divergence)> = vec![];
    for bearish in [true, false] {
        let price: Vec<f64> = bars.iter().map(|b| if bearish { b.high } else { b.low }).collect();
        let p = pivots(&price, bearish);
        let [.., a, b] = p[..] else { continue };
        let (oa, ob) = (osc[a], osc[b]);
        if !(oa.is_finite() && ob.is_finite()) {
            continue;
        }
        let diverges = if bearish {
            price[b] > price[a] && ob < oa
        } else {
            price[b] < price[a] && ob > oa
        };
        let age = n - 1 - b;
        // gone past: since the last high, a bar went above it (below the last low, for a bullish one)
        let gone_past = price[b + 1..].iter().any(|&x| if bearish { x > price[b] } else { x < price[b] });
        if diverges && age <= STALE_AFTER && !gone_past {
            found.push((
                b,
                Divergence {
                    bearish,
                    ages: (n - 1 - a, age),
                    prices: (price[a], price[b]),
                    values: (oa, ob),
                    confirmed: age - PIVOT_SIDE,
                },
            ));
        }
    }
    found.into_iter().max_by_key(|(b, _)| *b).map(|(_, d)| d)
}

/// The note of a reading: its zone and its divergence together.
pub(crate) fn note(zone: Option<&'static str>, d: Option<&Divergence>) -> Option<&'static str> {
    match (zone, d.map(|d| d.bearish)) {
        (z, None) => z,
        (None, Some(true)) => Some("divergence baissière"),
        (None, Some(false)) => Some("divergence haussière"),
        (Some("en surachat"), Some(true)) => Some("en surachat, divergence baissière"),
        (Some("en surachat"), Some(false)) => Some("en surachat, divergence haussière"),
        (Some("en survente"), Some(true)) => Some("en survente, divergence baissière"),
        (Some("en survente"), Some(false)) => Some("en survente, divergence haussière"),
        (Some(z), Some(_)) => Some(z),
    }
}

/// The sentence of a divergence of `osc_name` (« le RSI 14 »), with the price and oscillator values written by `px`
/// and `ov`, and the sentence that goes with it in « Comment le lire ».
pub(crate) fn sentences(d: &Divergence, osc_name: &str, px: impl Fn(f64) -> String, ov: impl Fn(f64) -> String) -> (Sentence, Sentence) {
    let (kind, price_move, osc_move) = if d.bearish {
        ("baissière", "un plus haut plus haut", "un plus haut plus bas")
    } else {
        ("haussière", "un plus bas plus bas", "un plus bas plus haut")
    };
    let when = if d.confirmed == 0 {
        "confirmée sur la dernière bougie".to_string()
    } else {
        format!("confirmée il y a {}", n_bars(d.confirmed))
    };
    let now = Sentence {
        id: "divergence",
        text: format!(
            "Divergence {kind} : le prix a fait {price_move} ({} il y a {}, puis {} il y a {}), mais {osc_name} {osc_move} ({} puis {}) ; {when}.",
            px(d.prices.0),
            n_bars(d.ages.0),
            px(d.prices.1),
            n_bars(d.ages.1),
            ov(d.values.0),
            ov(d.values.1)
        ),
    };
    let how = Sentence {
        id: "how_divergence",
        text: "Une divergence signale un élan qui faiblit, pas un retournement certain : elle peut durer longtemps ou échouer.".into(),
    };
    (now, how)
}
