//! 🌐 Numbers written as on the screen: the number formats of the top bar's 🌐 (as in Argos v1), the format chosen
//! (one global, set by the app from the saved settings) and the functions that write with it. The `…_in` functions
//! take the format's index (tests, or a caller that has its own); the others read the chosen format. The chart,
//! Argos AI and the summary tables all write their numbers here, so they read the same.

/// The number formats (🌐 of the top bar, as Argos v1): (example, (thousands separator, decimal mark), extra
/// decimals on prices). The index is saved in the settings: never reorder, only append.
pub const NUMBER_FORMATS: [(&str, (&str, &str), usize); 5] = [
    ("1 234,56", ("\u{202f}", ","), 0),
    ("1,234.56", (",", "."), 0),
    ("1.234,56", (".", ","), 0),
    ("1234,56", ("", ","), 0),
    ("1,234.567", (",", "."), 1),
];

static NUMBER_FORMAT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// The chosen format (index in `NUMBER_FORMATS`).
pub fn number_format() -> usize {
    NUMBER_FORMAT.load(std::sync::atomic::Ordering::Relaxed).min(NUMBER_FORMATS.len() - 1)
}

/// Chooses the format `k` (an unknown index: the last one).
pub fn set_number_format(k: usize) {
    NUMBER_FORMAT.store(k.min(NUMBER_FORMATS.len() - 1), std::sync::atomic::Ordering::Relaxed);
}

/// The format `fmt` (an unknown index: the last one).
fn format(fmt: usize) -> (&'static str, (&'static str, &'static str), usize) {
    NUMBER_FORMATS[fmt.min(NUMBER_FORMATS.len() - 1)]
}

/// A number with the separators of the format `fmt` and exactly `decimals` decimals (« 1 234,56 »; negative:
/// « −12,50 »).
pub fn format_number_in(p: f64, decimals: usize, fmt: usize) -> String {
    let (_, (sep, dec), _) = format(fmt);
    let s = format!("{:.*}", decimals, p.abs());
    let (int, frac) = s.split_once('.').map_or((s.as_str(), ""), |(a, b)| (a, b));
    let mut grouped = String::new();
    for (k, ch) in int.chars().enumerate() {
        if k > 0 && (int.len() - k) % 3 == 0 {
            grouped.push_str(sep);
        }
        grouped.push(ch);
    }
    let sign = if p < 0.0 { "−" } else { "" };
    if frac.is_empty() {
        format!("{sign}{grouped}")
    } else {
        format!("{sign}{grouped}{dec}{frac}")
    }
}

/// A price as on the chart's scale: `decimals` decimals (plus the format's extra one when there are decimals; whole
/// numbers stay whole).
pub fn format_price_in(p: f64, decimals: usize, fmt: usize) -> String {
    let extra = format(fmt).2;
    format_number_in(p, if decimals > 0 { decimals + extra } else { 0 }, fmt)
}

/// A dollar amount as v1 (« $100,000.00 », « −$12.50 »): the format's separators, exactly `decimals` decimals.
pub fn format_money_in(v: f64, decimals: usize, fmt: usize) -> String {
    let s = format_number_in(v.abs(), decimals, fmt);
    format!("{}${s}", if v < 0.0 { "−" } else { "" })
}

/// Short volume as v1's `kf` (« 950 », « 1,2K », « 3,4M », « 2,1B »), with the decimal mark of the format `fmt`.
pub fn kf_in(v: f64, fmt: usize) -> String {
    let a = v.abs();
    let (x, s) = if a >= 1e9 {
        (v / 1e9, "B")
    } else if a >= 1e6 {
        (v / 1e6, "M")
    } else if a >= 1e3 {
        (v / 1e3, "K")
    } else {
        return format!("{v:.0}");
    };
    let n = if x.abs() >= 100.0 {
        format!("{x:.0}")
    } else {
        format!("{x:.1}").replace('.', format(fmt).1 .1)
    };
    format!("{n}{s}")
}

/// A signed percentage with 2 decimals and the format's decimal mark (« +1,59 % », « −0,42 % »); the sign is decided
/// on the number as written, so « 0,00 % » has none (never « -0,00 % »).
pub fn format_pct_in(p: f64, fmt: usize) -> String {
    let abs = format_number_in(p.abs(), 2, fmt);
    let sign = if !abs.chars().any(|c| c.is_ascii_digit() && c != '0') {
        ""
    } else if p < 0.0 {
        "−"
    } else {
        "+"
    };
    format!("{sign}{abs} %")
}

/// [`format_pct_in`] with the chosen format.
pub fn format_pct(p: f64) -> String {
    format_pct_in(p, number_format())
}

/// [`format_number_in`] with the chosen format.
pub fn format_number(p: f64, decimals: usize) -> String {
    format_number_in(p, decimals, number_format())
}

/// [`format_price_in`] with the chosen format.
pub fn format_price(p: f64, decimals: usize) -> String {
    format_price_in(p, decimals, number_format())
}

/// [`format_money_in`] with the chosen format.
pub fn format_money(v: f64, decimals: usize) -> String {
    format_money_in(v, decimals, number_format())
}

/// [`kf_in`] with the chosen format.
pub fn kf(v: f64) -> String {
    kf_in(v, number_format())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 🌐 Each format writes the same number its own way; prices get the format's extra decimal; an unknown index is
    /// the last format.
    #[test]
    fn chaque_format() {
        let n = 1234567.891;
        let w: Vec<String> = (0..NUMBER_FORMATS.len()).map(|f| format_number_in(n, 2, f)).collect();
        assert_eq!(w, ["1\u{202f}234\u{202f}567,89", "1,234,567.89", "1.234.567,89", "1234567,89", "1,234,567.89"]);
        assert_eq!(format_number_in(-12.5, 2, 0), "−12,50");
        assert_eq!(format_number_in(950.0, 0, 1), "950");
        assert_eq!(
            (format_price_in(62035.0, 2, 0), format_price_in(62035.0, 2, 4)),
            ("62\u{202f}035,00".into(), "62,035.000".into())
        );
        assert_eq!(format_price_in(62035.0, 0, 4), "62,035", "whole numbers stay whole");
        assert_eq!(
            (format_money_in(-12.5, 2, 1), format_money_in(100000.0, 2, 1)),
            ("−$12.50".into(), "$100,000.00".into())
        );
        assert_eq!(format_number_in(1.5, 1, 99), "1.5");
    }

    /// 📈 Percentages (Jee, 22 Nov: « +1.59 % » with a point in the « Variation 24 h » box): the format's decimal mark,
    /// the true minus, no sign on what is written as zero.
    #[test]
    fn pourcentages() {
        assert_eq!(
            [
                format_pct_in(1.594, 0),
                format_pct_in(-0.42, 0),
                format_pct_in(-0.001, 0),
                format_pct_in(12.0, 1)
            ],
            ["+1,59 %", "−0,42 %", "0,00 %", "+12.00 %"]
        );
    }

    /// 📋 Short volume: v1's steps, the format's decimal mark (« 7,7K » by default, « 7.7K » in English).
    #[test]
    fn volume_court() {
        assert_eq!(
            [kf_in(950.0, 0), kf_in(7700.0, 0), kf_in(7700.0, 1), kf_in(123_456.0, 0)],
            ["950", "7,7K", "7.7K", "123K"]
        );
        assert_eq!([kf_in(3_456_789.0, 2), kf_in(2.1e9, 3), kf_in(-1500.0, 1)], ["3,5M", "2,1B", "-1.5K"]);
    }
}
