//! Numbers written for the screen in the number format chosen in 🌐 (`argos_core::nombres`: the same state and
//! functions as the chart): « 64 812,50 » by default. The decimals of a price stay the Argos Lens card's
//! (`argos_lens::decimales_prix`).

use argos_core::nombres::{format_number_in, format_price_in, number_format};
use argos_lens::{decimales_prix, Decimal};

/// The decimals of the price `x` on the Argos Lens card (2 from 1 000, more for small prices); `None` when `x` is not
/// a writable number.
fn price_decimals(x: f64) -> Option<usize> {
    Decimal::try_from(x).ok().map(|d| decimales_prix(d) as usize)
}

/// A true minus sign in front of `s` when `x` is negative and `s` is not a zero (never « −0,0 »).
fn with_sign(x: f64, s: String) -> String {
    if x < 0.0 && s.chars().any(|c| c.is_ascii_digit() && c != '0') {
        format!("−{s}")
    } else {
        s
    }
}

/// A price in the chosen format (« 64 812,50 »).
pub(crate) fn price(x: f64) -> String {
    price_in(x, number_format())
}

/// A price in the format `fmt`: 2 decimals from 1 000, more for small prices (plus the format's extra decimal).
pub(crate) fn price_in(x: f64, fmt: usize) -> String {
    price_decimals(x).map_or_else(|| "?".to_string(), |dp| format_price_in(x, dp, fmt))
}

/// A value with `dp` decimals in the chosen format (« 64,3 », « −80,0 » with a true minus sign).
pub(crate) fn num(x: f64, dp: u32) -> String {
    num_in(x, dp, number_format())
}

/// A value with `dp` decimals in the format `fmt`.
pub(crate) fn num_in(x: f64, dp: u32, fmt: usize) -> String {
    if !x.is_finite() {
        return "?".to_string();
    }
    with_sign(x, format_number_in(x.abs(), dp as usize, fmt))
}

/// A value in price units that can be negative (MACD), with the decimals of the price `reference` and a true minus
/// sign (« −154,16 »).
pub(crate) fn signed(x: f64, reference: f64) -> String {
    if !x.is_finite() {
        return "?".to_string();
    }
    let dp = price_decimals(reference).unwrap_or(2);
    with_sign(x, format_price_in(x.abs(), dp, number_format()))
}

/// A signed percentage with 2 decimals (« +0,42 % », « −1,10 % »).
pub(crate) fn percent(x: f64) -> String {
    let s = num(x.abs(), 2);
    if x < 0.0 {
        format!("−{s} %")
    } else {
        format!("+{s} %")
    }
}

/// « 1 bougie », « 3 bougies ».
pub(crate) fn bars(n: usize) -> String {
    if n == 1 {
        "1 bougie".to_string()
    } else {
        format!("{n} bougies")
    }
}
