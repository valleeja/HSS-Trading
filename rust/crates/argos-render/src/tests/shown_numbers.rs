//! 🔢 Data reliability, test 1: a number written on the screen is the number that came.
//!
//! What it protects: the safety net (`argos-desktop`, `safety_net`) keeps every screen as it was drawn; it does not
//! say that a number drawn is the right one. Here numbers come as an exchange writes them (plain decimal text, in
//! the exchange's own message where the app has a reader for it), go through the app's own readers and writers, and
//! the **text the screen writes** is read back ([`crate::text::ecrits`]) and compared with what came.
//!
//! - **The rule**: when the screen has as many decimals as the number that came, it writes the same digits, grouped
//!   and marked as the chosen number format says; when it has fewer, it writes the number rounded, never further
//!   than half a unit of its last decimal. Never another digit, never another sign.
//! - **What came is text, and stays text here**: the expected text is made from the received text ([`written`]),
//!   and a text of the screen is read back digit by digit ([`read_in`]) into an exact decimal ([`Dec`]); no float
//!   and none of the app's own functions sit on that side of the comparison.
//! - **What cannot be is left out**: a level of the Book or a trade of the Trades module whose price or size is not
//!   a number above zero ([`MAD`]) is not written, and the screen is the one it would be had it not come.
//! - **Where**: the number formats of the top bar's 🌐 (`argos_core::nombres`); the chart (the legend
//!   O H L C · Vol, the label of the last price, the pills of the highest and the lowest price); the Book, the Trades
//!   and the Watch modules of the column; the price of a watchlist row; the funding rate of the market details.
//! - **Made-up data**: a fixed draw ([`Draw`]), the same on every computer and at every run, and a few markets written
//!   by hand. Nothing here is real or anyone's.
//! - **What is seen and not right yet** is not hidden: [`SEEN`] lists it, with what the screen writes today and
//!   what was expected. A fix makes `what_was_seen_is_still_there` fail on purpose: the entry leaves the list and
//!   its case joins the test of its screen.
//!
//! Not reached from here: what `argos-desktop` writes by hand inside its drawing (the rows of the Funding module);
//! the drawn text can only be read back inside this crate.
//!
//! The screens are drawn with the default number format (« 1 234,56 »): the chosen format is one setting for the whole
//! program, so no test may change it; the other formats are checked on the writers themselves.

use super::*;
use argos_core::nombres::{format_money_in, format_number_in, format_pct_in, format_price_in, kf_in};

/// The number formats of the top bar's 🌐, as their examples read (« 1 234,56 », « 1,234.56 », « 1.234,56 »,
/// « 1234,56 », « 1,234.567 »): (thousands separator, decimal mark, extra decimals on prices). Written by hand: it is
/// what the user chose, not what the code says. The settings keep the index: never reorder.
const FORMATS: [(&str, char, usize); 5] = [("\u{202f}", ',', 0), (",", '.', 0), (".", ',', 0), ("", ',', 0), (",", '.', 1)];

/// The default format, the one of every screen drawn here.
const DEFAULT: (&str, char) = (FORMATS[0].0, FORMATS[0].1);

/// The true minus sign of the screen (never the hyphen).
const MINUS: char = '−';

/// An exact decimal number, `m` × 10^-`scale`: what an exchange writes and what the screen writes, without a float.
#[derive(Debug, Clone, Copy)]
struct Dec {
    m: i128,
    scale: u32,
}

impl Dec {
    /// A plain decimal text (« -12.50 », « 0.00000712 »): an optional hyphen, digits, an optional point and digits.
    fn of(s: &str) -> Dec {
        let (neg, s) = s.strip_prefix('-').map_or((false, s), |r| (true, r));
        let (int, frac) = s.split_once('.').unwrap_or((s, ""));
        assert!(
            !int.is_empty() && int.chars().chain(frac.chars()).all(|c| c.is_ascii_digit()),
            "not a plain decimal: « {s} »"
        );
        let m: i128 = format!("{int}{frac}").parse().unwrap();
        Dec {
            m: if neg { -m } else { m },
            scale: frac.len() as u32,
        }
    }

    /// The whole number of 10^-`scale` this number is worth (`scale` at least its own).
    fn at(self, scale: u32) -> i128 {
        self.m * 10i128.pow(scale - self.scale)
    }

    /// The same number, whatever the zeros written at the end.
    fn same(self, o: Dec) -> bool {
        let s = self.scale.max(o.scale);
        self.at(s) == o.at(s)
    }

    /// Is `shown` this number rounded to the decimals `shown` has: never further than half a unit of its last one?
    fn rounds_to(self, shown: Dec) -> bool {
        let s = self.scale.max(shown.scale);
        2 * (self.at(s) - shown.at(s)).abs() <= 10i128.pow(s - shown.scale)
    }

    fn plus(self, o: Dec) -> Dec {
        let s = self.scale.max(o.scale);
        Dec {
            m: self.at(s) + o.at(s),
            scale: s,
        }
    }

    fn minus(self, o: Dec) -> Dec {
        self.plus(Dec { m: -o.m, scale: o.scale })
    }

    /// As a float, for the checks that allow a tolerance only (a percentage worked out from two prices).
    fn f64(self) -> f64 {
        self.m as f64 / 10f64.powi(self.scale as i32)
    }

    /// Back to a plain decimal text.
    fn plain(self) -> String {
        let digits = format!("{:0>w$}", self.m.abs(), w = self.scale as usize + 1);
        let (int, frac) = digits.split_at(digits.len() - self.scale as usize);
        format!("{}{int}{}{frac}", if self.m < 0 { "-" } else { "" }, if frac.is_empty() { "" } else { "." })
    }
}

/// What the screen must write for a number that came as `received` (plain decimal text) when it shows `decimals`
/// decimals, at least those that came: the same digits, zeros added at the end, thousands grouped by `sep`, `mark`
/// before the decimals, the true minus. Text only.
fn written(received: &str, decimals: usize, (sep, mark): (&str, char)) -> String {
    let (neg, s) = received.strip_prefix('-').map_or((false, received), |r| (true, r));
    let (int, frac) = s.split_once('.').unwrap_or((s, ""));
    assert!(
        frac.len() <= decimals,
        "« {received} » has more than {decimals} decimals: the screen would round it"
    );
    let mut out = String::new();
    if neg && s.chars().any(|c| c.is_ascii_digit() && c != '0') {
        out.push(MINUS);
    }
    for (k, c) in int.chars().enumerate() {
        if k > 0 && (int.len() - k) % 3 == 0 {
            out.push_str(sep);
        }
        out.push(c);
    }
    if decimals > 0 {
        out.push(mark);
        out.push_str(frac);
        out.extend(std::iter::repeat_n('0', decimals - frac.len()));
    }
    out
}

/// The number a text writes in the format (`sep`, `mark`), read back digit by digit; `None` when the text is not a
/// number of that format (another separator, a group that is not three digits, a hyphen for the minus, a letter).
fn read_in(text: &str, (sep, mark): (&str, char)) -> Option<Dec> {
    let (neg, s) = text.strip_prefix(MINUS).map_or((false, text), |r| (true, r));
    let (int, frac) = match s.split_once(mark) {
        Some((i, f)) if !f.is_empty() => (i, f),
        Some(_) => return None,
        None => (s, ""),
    };
    let digits = |g: &str| !g.is_empty() && g.chars().all(|c| c.is_ascii_digit());
    let groups: Vec<&str> = if sep.is_empty() { vec![int] } else { int.split(sep).collect() };
    // a format that groups: one to three digits, then groups of three (« 1234 » is not written by it)
    let grouped = sep.is_empty() || (groups[0].len() <= 3 && groups[1..].iter().all(|g| g.len() == 3));
    if !(groups.iter().all(|g| digits(g)) && grouped && (frac.is_empty() || digits(frac))) {
        return None;
    }
    let plain = format!(
        "{}{}{}{frac}",
        if neg { "-" } else { "" },
        groups.concat(),
        if frac.is_empty() { "" } else { "." }
    );
    Some(Dec::of(&plain))
}

/// [`read_in`] with the default format, or the test stops and says which text is not a number of the screen.
fn read(text: &str) -> Dec {
    read_in(text, DEFAULT).unwrap_or_else(|| panic!("« {text} » is not a number written in the default format"))
}

/// A number written without the number format (a point, a hyphen, a plus sign), as a few places still write it
/// ([`SEEN`]): read all the same, so that its value is checked.
fn read_raw(text: &str) -> Dec {
    let t = text.replace(MINUS, "-");
    Dec::of(t.strip_prefix('+').unwrap_or(&t))
}

/// A fixed draw (xorshift64*): the same numbers on every computer and at every run.
struct Draw(u64);

impl Draw {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// A whole number below `n`.
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }

    /// A number as an exchange writes it: `int` digits before the point (« 0 » when none), `decimals` after.
    fn decimal(&mut self, int: u32, decimals: u32) -> String {
        let mut s = String::new();
        for k in 0..int {
            s.push(char::from(b'0' + if k == 0 { 1 + self.below(9) } else { self.below(10) } as u8));
        }
        if int == 0 {
            s.push('0');
        }
        if decimals > 0 {
            s.push('.');
            for _ in 0..decimals {
                s.push(char::from(b'0' + self.below(10) as u8));
            }
        }
        s
    }
}

/// Is every digit of `s` a zero (the number is written as zero)?
fn zero(s: &str) -> bool {
    !s.chars().any(|c| c.is_ascii_digit() && c != '0')
}

/// A price or a size that cannot be, as a message could write it: not a number, infinite (« 1e999 » is read as
/// infinite), zero, negative.
const MAD: [&str; 10] = ["NaN", "nan", "inf", "-inf", "Infinity", "1e999", "0", "0.000", "-1", "-0.5"];

/// Can this price or this size be? Said by hand, on the text that came: a plain number (digits and a point) that is
/// not zero. No sign, no letter.
fn can_be(s: &str) -> bool {
    s.chars().any(|c| c.is_ascii_digit()) && s.chars().all(|c| c.is_ascii_digit() || c == '.') && !zero(s)
}

/// One of [`MAD`].
fn mad(d: &mut Draw) -> String {
    MAD[d.below(MAD.len() as u64) as usize].to_string()
}

/// 🧪 The tools of these tests catch a wrong number themselves: a text that is not a number of the format is refused,
/// a number further than half a unit of the last decimal is refused.
#[test]
fn the_checks_catch_a_wrong_number() {
    // what can be and what cannot
    assert!(["97123.5", "0.00016", "1234", "0.5", "10"].iter().all(|s| can_be(s)));
    assert!(MAD.iter().chain(&["", ".", "-97123.5", "0.0", "00", "1e-5", "+1", "1 000"]).all(|s| !can_be(s)));
    // what the screen must write
    assert_eq!(written("1234567.891", 3, DEFAULT), "1\u{202f}234\u{202f}567,891");
    assert_eq!(written("-12.5", 2, DEFAULT), "−12,50");
    assert_eq!(written("-0.00", 2, DEFAULT), "0,00", "zero has no sign");
    assert_eq!(written("97000", 0, DEFAULT), "97\u{202f}000");
    assert_eq!(written("0.00000712", 8, (",", '.')), "0.00000712");
    // a text read back
    assert!(read_in("1\u{202f}234,56", DEFAULT).is_some_and(|d| d.same(Dec::of("1234.56"))));
    assert!(read_in("−0,5", DEFAULT).is_some_and(|d| d.same(Dec::of("-0.5"))));
    for wrong in [
        "1234,56",
        "1 234,56",
        "1,234.56",
        "-0,5",
        "+0,5",
        "12,",
        ",5",
        "1\u{202f}23,4",
        "NaN",
        "inf",
        "",
        "1,2,3",
        "1e5",
    ] {
        assert!(read_in(wrong, DEFAULT).is_none(), "« {wrong} » is not a number of the default format");
    }
    assert!(read_in("1234,56", ("", ',')).is_some(), "the format without a separator");
    // the same number, a number rounded
    assert!(Dec::of("97123.50").same(Dec::of("97123.5")) && !Dec::of("97123.5").same(Dec::of("97123.4")));
    let came = Dec::of("0.00015");
    assert!(
        came.rounds_to(Dec::of("0.0001")) && came.rounds_to(Dec::of("0.0002")),
        "a tie: either neighbour"
    );
    assert!(!came.rounds_to(Dec::of("0.0003")) && !came.rounds_to(Dec::of("0.0000")) && !Dec::of("0.00016").rounds_to(Dec::of("0.0001")));
    assert_eq!(
        (Dec::of("-0.05").plain(), Dec::of("12").minus(Dec::of("0.25")).plain()),
        ("-0.05".into(), "11.75".into())
    );
}

/// 🌐 The number formats: a number that came with `k` decimals and is written with at least `k` is written with the
/// digits that came, in each of the five formats; a price gets the format's extra decimal; an amount its dollar; a
/// number written with fewer decimals is rounded, never further than half a unit of the last one.
#[test]
fn the_number_formats_write_the_digits_that_came() {
    let mut d = Draw(0x5EED_0001);
    let (mut exact, mut rounded) = (0, 0);
    for _ in 0..4000 {
        // a float keeps 15 digits: what the app reads from an exchange always fits
        let int = d.below(10) as u32;
        let k = (d.below(9) as u32).min(15 - int);
        let mut received = d.decimal(int, k);
        if d.below(5) == 0 && !zero(&received) {
            received.insert(0, '-');
        }
        let v: f64 = received.parse().unwrap();
        for (f, &(sep, mark, extra)) in FORMATS.iter().enumerate() {
            for more in 0..3 {
                let n = (k + more) as usize;
                if int as usize + n + extra > 15 {
                    continue;
                }
                exact += 1;
                let want = written(&received, n, (sep, mark));
                assert_eq!(format_number_in(v, n, f), want, "« {received} », {n} decimals, format {f}");
                let price = if n > 0 { written(&received, n + extra, (sep, mark)) } else { want.clone() };
                assert_eq!(format_price_in(v, n, f), price, "the price « {received} », {n} decimals, format {f}");
                let money = match want.strip_prefix(MINUS) {
                    Some(abs) => format!("{MINUS}${abs}"),
                    None => format!("${want}"),
                };
                assert_eq!(format_money_in(v, n, f), money, "the amount « {received} », {n} decimals, format {f}");
            }
            for n in 0..k as usize {
                rounded += 1;
                let text = format_number_in(v, n, f);
                let shown = read_in(&text, (sep, mark)).unwrap_or_else(|| panic!("« {text} » for « {received} », format {f}"));
                assert_eq!(shown.scale as usize, n, "« {text} »: {n} decimals asked");
                assert!(
                    Dec::of(&received).rounds_to(shown),
                    "« {received} » written « {text} » with {n} decimals, format {f}"
                );
            }
        }
    }
    assert!(exact > 20_000 && rounded > 20_000, "{exact} exact, {rounded} rounded: the draw covers both");
}

/// 🌐 A percentage: two decimals, the format's decimal mark, « + » or the true minus, and no sign on what is written
/// as zero; never further than half a hundredth from what came. Short volumes (« 7,7K »): the number that came,
/// rounded to what is written.
#[test]
fn percentages_and_short_volumes_are_what_came_rounded() {
    let mut d = Draw(0x5EED_0002);
    for _ in 0..4000 {
        let received = {
            let (int, k) = (d.below(4) as u32, d.below(7) as u32);
            let r = d.decimal(int, k);
            if d.below(2) == 0 && !zero(&r) {
                format!("-{r}")
            } else {
                r
            }
        };
        let v: f64 = received.parse().unwrap();
        for (f, &(sep, mark, _)) in FORMATS.iter().enumerate() {
            let text = format_pct_in(v, f);
            let body = text.strip_suffix(" %").unwrap_or_else(|| panic!("« {text} »: a percentage ends with « % »"));
            let (sign, digits) = match body.chars().next() {
                Some('+') => (1, &body[1..]),
                Some(MINUS) => (-1, &body[MINUS.len_utf8()..]),
                _ => (0, body),
            };
            let shown = read_in(digits, (sep, mark)).unwrap_or_else(|| panic!("« {text} » for « {received} », format {f}"));
            assert_eq!(shown.scale, 2, "« {text} »: two decimals");
            let want = if shown.m == 0 { 0 } else { v.signum() as i32 };
            assert_eq!(sign, want, "« {received} » written « {text} »: the sign of what is written");
            let abs = Dec::of(received.trim_start_matches('-'));
            assert!(abs.rounds_to(shown), "« {received} » written « {text} », format {f}");
        }
    }
    for _ in 0..4000 {
        let (int, k) = (1 + d.below(11) as u32, d.below(3) as u32);
        let received = d.decimal(int, k);
        let v: f64 = received.parse().unwrap();
        for (f, &(_, mark, _)) in FORMATS.iter().enumerate() {
            let text = kf_in(v, f);
            let (digits, unit) = match text.chars().last() {
                Some('K') => (&text[..text.len() - 1], 3),
                Some('M') => (&text[..text.len() - 1], 6),
                Some('B') => (&text[..text.len() - 1], 9),
                _ => (text.as_str(), 0),
            };
            let shown = read_in(digits, ("", mark)).unwrap_or_else(|| panic!("« {text} » for « {received} », format {f}"));
            // « 1,2K » is 1 200, to the nearest hundred: the unit moves the decimals
            let came = Dec::of(&received);
            let one = 10i128.pow(unit + came.scale - shown.scale);
            assert!(2 * (came.m - shown.m * one).abs() <= one, "« {received} » written « {text} », format {f}");
        }
    }
}

/// 📈 The last candle of a market as it came (open, high, low, close as the exchange writes them) and its volume.
const MARKETS: [(&str, [&str; 4], &str); 16] = [
    ("BTC, one decimal", ["97123.5", "97200.0", "97000.5", "97150.5"], "12.345"),
    ("BTC, two decimals", ["97123.45", "97200.10", "97000.05", "97150.55"], "1234.5"),
    ("BTC, whole dollars", ["97123", "97200", "97000", "97150"], "0.5"),
    ("ETH", ["3456.78", "3460.01", "3449.99", "3458.20"], "845.2"),
    ("SOL, three decimals", ["187.345", "188.001", "186.999", "187.500"], "12345678.9"),
    ("XRP, four decimals", ["0.5234", "0.5301", "0.5199", "0.5250"], "1234567890"),
    ("DOGE, five decimals", ["0.12345", "0.12399", "0.12301", "0.12377"], "950"),
    ("a coin at a cent, six decimals", ["0.012345", "0.012399", "0.012301", "0.012377"], "5000"),
    (
        "a coin at a fraction of a cent, eight decimals",
        ["0.00002345", "0.00002360", "0.00002301", "0.00002351"],
        "123456789",
    ),
    (
        "a pair against BTC, eight decimals",
        ["0.00001631", "0.00001640", "0.00001622", "0.00001635"],
        "7700",
    ),
    ("EUR/USD, five decimals", ["1.08345", "1.08402", "1.08299", "1.08377"], "0"),
    ("USD/JPY, three decimals", ["151.234", "151.900", "150.875", "151.500"], "1"),
    ("gold", ["2345.67", "2350.00", "2340.25", "2349.75"], "999.99"),
    ("an index in the millions", ["15123456", "15200000", "15100001", "15150505"], "42"),
    // every digit that came, under 1 (Jee, 23 Nov 2026: « Tous les chiffres reçus »)
    ("a price with seven decimals", ["0.0123456", "0.0123999", "0.0123001", "0.0123777"], "1"),
    (
        "XRPBTC at Bitfinex, five digits that count",
        ["0.000016312", "0.000016409", "0.000016221", "0.000016357"],
        "1",
    ),
];

/// Sixty candles alike, as [`MARKETS`] gives the last one.
fn candles(ohlc: [&str; 4], volume: &str) -> Vec<Bar> {
    let p = |s: &str| -> f64 { s.parse().unwrap() };
    (0..60)
        .map(|i| Bar {
            time: 1_700_000_000 + i * 1800,
            open: p(ohlc[0]),
            high: p(ohlc[1]),
            low: p(ohlc[2]),
            close: p(ohlc[3]),
            volume: p(volume),
        })
        .collect()
}

/// Everything a chart of these candles writes, the legend shown.
fn chart_texts(b: &[Bar]) -> Vec<String> {
    let mut c = chart(b, CandleStyle::Classic);
    c.legende = true;
    (c.viewport.width, c.viewport.height) = (900, 600);
    let _ = crate::text::ecrits();
    let _ = render(&c);
    crate::text::ecrits()
}

/// The legend's first line (« O …  H …  L …  C …  ±x % ») cut into its five values.
fn legend(texts: &[String]) -> [String; 5] {
    let line = texts.iter().find(|s| s.starts_with("O ")).unwrap_or_else(|| panic!("no legend in {texts:?}"));
    let v: Vec<&str> = line.split("  ").collect();
    assert_eq!(v.len(), 5, "« {line} »");
    let cut = |k: usize, name: &str| v[k].strip_prefix(name).unwrap_or_else(|| panic!("« {line} »: « {name} » expected")).to_string();
    [cut(0, "O "), cut(1, "H "), cut(2, "L "), cut(3, "C "), v[4].to_string()]
}

/// 📈 The chart writes the prices that came: the legend's open, high, low and close, the label of the last price on
/// the scale and the pills of the highest and the lowest price are the numbers of the exchange, digit for digit; the
/// change is the one of the candle and the volume is the one that came, rounded to what is written.
#[test]
fn the_chart_writes_the_prices_that_came() {
    for (market, ohlc, volume) in MARKETS {
        let texts = chart_texts(&candles(ohlc, volume));
        let l = legend(&texts);
        for (k, name) in ["open", "high", "low", "close"].iter().enumerate() {
            let shown = read(&l[k]);
            assert!(
                Dec::of(ohlc[k]).same(shown),
                "{market}: the {name} came as « {} », the legend writes « {} »",
                ohlc[k],
                l[k]
            );
        }
        // the same texts, outside the legend: the last price on the scale, the highest and the lowest price
        let outside: Vec<&String> = texts.iter().filter(|s| !s.starts_with("O ")).collect();
        for (k, what) in [
            (3, "the label of the last price"),
            (1, "the pill of the highest price"),
            (2, "the pill of the lowest price"),
        ] {
            assert!(outside.contains(&&l[k]), "{market}: {what}, « {} », is not written: {texts:?}", l[k]);
        }
        // the candle's change, to a hundredth of a percent (its decimal mark: see SEEN)
        let (o, c) = (Dec::of(ohlc[0]).f64(), Dec::of(ohlc[3]).f64());
        let pct = read_raw(l[4].strip_suffix(" %").unwrap_or_else(|| panic!("« {} »", l[4])));
        assert!((pct.f64() - (c / o - 1.0) * 100.0).abs() <= 0.005 + 1e-9, "{market}: the change, « {} »", l[4]);
        // the volume: « Vol 12,35 », « Vol 1,23 K », « Vol 12,35 M », « Vol 1,23 B »
        let vol = texts
            .iter()
            .find_map(|s| s.strip_prefix("Vol "))
            .unwrap_or_else(|| panic!("{market}: no volume in {texts:?}"));
        let (digits, unit) = [(" K", 1e3), (" M", 1e6), (" B", 1e9)]
            .iter()
            .find_map(|(u, x)| vol.strip_suffix(u).map(|d| (d, *x)))
            .unwrap_or((vol, 1.0));
        let shown = read(digits);
        assert_eq!(shown.scale, 2, "{market}: « Vol {vol} », two decimals");
        assert!(
            (shown.f64() * unit - Dec::of(volume).f64()).abs() <= 0.005 * unit * (1.0 + 1e-9),
            "{market}: the volume came as « {volume} », the legend writes « {vol} »"
        );
    }
}

/// An order book as Hyperliquid sends it (`l2Book`): the levels (price, size) of each side, the best first.
fn l2book(bids: &[(String, String)], asks: &[(String, String)]) -> String {
    let side = |l: &[(String, String)]| {
        l.iter()
            .map(|(px, sz)| format!(r#"{{"px":"{px}","sz":"{sz}","n":1}}"#))
            .collect::<Vec<_>>()
            .join(",")
    };
    format!(
        r#"{{"channel":"l2Book","data":{{"coin":"COIN","time":1793050000000,"levels":[[{}],[{}]]}}}}"#,
        side(bids),
        side(asks)
    )
}

/// A made-up book around a price with `dec` decimals: `n` levels a side, the best first, sizes below `size` digits
/// before the point. Now and then a level cannot be: its price or its size is one of [`MAD`]; a size drawn as zero
/// and, around a price close to zero, a purchase at a price of zero or under cannot be either.
fn made_up_book(d: &mut Draw, dec: u32, n: usize, size: u32) -> (Vec<(String, String)>, Vec<(String, String)>) {
    let int = 1 + d.below(5) as u32;
    let mid = Dec::of(&d.decimal(int, dec));
    let tick = |k: i128| Dec { m: k, scale: dec };
    let level = |d: &mut Draw, px: Dec| {
        let (int, decimals) = (d.below(size as u64 + 1) as u32, d.below(6) as u32);
        let (px, sz) = (px.plain(), d.decimal(int, decimals));
        match d.below(16) {
            0 => (mad(d), sz),
            1 => (px, mad(d)),
            _ => (px, sz),
        }
    };
    let mut step = 0;
    let bids = (0..n)
        .map(|_| {
            step += 1 + d.below(40) as i128;
            level(d, mid.minus(tick(step)))
        })
        .collect();
    step = 0;
    let asks = (0..n)
        .map(|_| {
            step += 1 + d.below(40) as i128;
            level(d, mid.plus(tick(step)))
        })
        .collect();
    (bids, asks)
}

/// 📖 The Book module writes the levels that came, read from the exchange's own message: each price digit for digit;
/// each size rounded to the decimals the book shows (whole numbers from 1 000, two decimals from 10, four below: one
/// format for the whole book); each total the sum of the sizes down to its row; the spread the best sale minus the
/// best purchase, and its percentage of the middle price. A level that cannot be (its price or its size is not a
/// number above zero) is left out: the book is the one it would be had that level not come.
#[test]
fn the_book_writes_the_levels_that_came() {
    // by hand: a purchase of 1,5 at 97 123 and a sale of 12,5 at 97 126
    let without = book_texts("");
    assert_eq!(
        without,
        [
            "Prix",
            "Taille",
            "Total",
            "97\u{202f}126,0",
            "12,50",
            "12,50",
            "Écart 3,0 · 0,003 %",
            "97\u{202f}123,0",
            "1,50",
            "1,50"
        ]
    );
    // a better sale that can be is written, and counts in the total and in the spread
    assert_eq!(
        book_texts(r#"{"px":"97125.0","sz":"2.5","n":1}"#)[3..10],
        ["97\u{202f}125,0", "2,50", "2,50", "97\u{202f}126,0", "12,50", "15,00", "Écart 2,0 · 0,002 %"]
    );
    // one that cannot be is left out, and nothing else moves (« NaN » with « inf » was seen written on 23 Nov 2026)
    assert_eq!(book_texts(r#"{"px":"NaN","sz":"inf","n":1}"#), without);
    for no in MAD {
        for level in [
            format!(r#"{{"px":"{no}","sz":"2.5","n":1}}"#),
            format!(r#"{{"px":"97125.0","sz":"{no}","n":1}}"#),
        ] {
            assert_eq!(book_texts(&level), without, "the level {level} cannot be: the book is written without it");
        }
    }
    // made-up books
    let mut d = Draw(0x5EED_0003);
    let text = crate::text::Text::default();
    let (mut left_out, mut whole, mut one_side, mut none) = (0, 0, 0, 0);
    for round in 0..400 {
        // nine rows a side fit in 400 points: every level of these books is shown
        let (dec, levels, size) = (d.below(7) as u32, 1 + d.below(9) as usize, 1 + d.below(6) as u32);
        let came = made_up_book(&mut d, dec, levels, size);
        let book = argos_data::book::parse_book(&l2book(&came.0, &came.1)).expect("the reader of the book reads its own exchange");
        let mut pm = crate::px::new(320, 400).unwrap();
        let _ = crate::text::ecrits();
        crate::dock::draw_book(
            &mut pm,
            &text,
            320.0,
            400.0,
            Some(&book),
            dec as usize,
            &Theme::default(),
            &crate::dock::CarnetSource::Hyperliquid,
        );
        let e = crate::text::ecrits();
        // by hand: the levels that can be, in the order they came
        let kept = |l: &[(String, String)]| -> Vec<(String, String)> { l.iter().filter(|(px, sz)| can_be(px) && can_be(sz)).cloned().collect() };
        let (bids, asks) = (kept(&came.0), kept(&came.1));
        left_out += came.0.len() + came.1.len() - bids.len() - asks.len();
        if bids.is_empty() && asks.is_empty() {
            assert_eq!(e, ["Connexion au carnet d'Hyperliquid…"], "round {round}: no level that can be came");
            none += 1;
            continue;
        }
        assert_eq!(e[..3], ["Prix", "Taille", "Total"], "round {round}: {e:?}");
        assert_eq!(
            e.len(),
            4 + 3 * (asks.len() + bids.len()),
            "round {round}: a row for each level that can be, {e:?}"
        );
        let (rows_asks, rest) = e[3..].split_at(3 * asks.len());
        let (spread, rows_bids) = rest.split_first().unwrap_or_else(|| panic!("round {round}: {e:?}"));
        let largest = bids.iter().chain(&asks).map(|(_, sz)| Dec::of(sz)).max_by_key(|s| s.at(5)).unwrap();
        let size_decimals = if largest.at(5) >= 1000 * 100_000 {
            0
        } else if largest.at(5) >= 10 * 100_000 {
            2
        } else {
            4
        };
        for (side, levels, rows) in [("sale", &asks, rows_asks), ("purchase", &bids, rows_bids)] {
            let mut total = Dec { m: 0, scale: 0 };
            for (k, (px, sz)) in levels.iter().enumerate() {
                let [price, size, sum] = [&rows[3 * k], &rows[3 * k + 1], &rows[3 * k + 2]];
                assert_eq!(price, &written(px, dec as usize, DEFAULT), "round {round}, {side} {k}: the price « {px} »");
                total = total.plus(Dec::of(sz));
                for (what, came, text) in [("size", Dec::of(sz), size), ("total", total, sum)] {
                    let shown = read(text);
                    assert_eq!(
                        shown.scale, size_decimals,
                        "round {round}, {side} {k}: the {what} « {text} », one format for the whole book"
                    );
                    assert!(
                        came.rounds_to(shown),
                        "round {round}, {side} {k}: the {what} came as {}, the book writes « {text} »",
                        came.plain()
                    );
                }
            }
        }
        let (Some(bid), Some(ask)) = (bids.first(), asks.first()) else {
            assert_eq!(spread, "—", "round {round}: no spread with one side only, {e:?}");
            one_side += 1;
            continue;
        };
        let (bid, ask) = (Dec::of(&bid.0), Dec::of(&ask.0));
        let gap = ask.minus(bid);
        let (abs, pct) = spread
            .strip_prefix("Écart ")
            .and_then(|s| s.strip_suffix(" %"))
            .and_then(|s| s.split_once(" · "))
            .unwrap_or_else(|| panic!("round {round}: « {spread} »"));
        assert_eq!(
            abs,
            written(&gap.plain(), dec as usize, DEFAULT),
            "round {round}: the spread of {} and {}",
            bid.plain(),
            ask.plain()
        );
        let middle = (ask.f64() + bid.f64()) / 2.0;
        let shown = read(pct);
        assert_eq!(shown.scale, 3, "round {round}: « {spread} »");
        assert!(
            (shown.f64() - gap.f64() / middle * 100.0).abs() <= 0.0005 + 1e-9,
            "round {round}: « {spread} » for {} and {}",
            bid.plain(),
            ask.plain()
        );
        whole += 1;
    }
    assert!(
        left_out > 300 && whole > 250 && one_side > 10 && none > 0,
        "{left_out} levels left out, {whole} books with two sides, {one_side} with one, {none} with none: each must be seen"
    );
}

/// ⚡ The Trades module writes the trades that came, read from the exchange's own message: the price digit for
/// digit, the size rounded to what is written (four decimals; whole numbers from 1 000), the hour of the trade in the
/// user's time zone. A trade that cannot be (its price or its size is not a number above zero) is left out: the list
/// is the one it would be had that trade not come.
#[test]
fn the_tape_writes_the_trades_that_came() {
    let text = crate::text::Text::default();
    let draw = |msg: &str, dec: usize, tz: i64| {
        let trades = argos_data::whales::parse_all(msg);
        let mut pm = crate::px::new(320, 600).unwrap();
        let _ = crate::text::ecrits();
        crate::dock::draw_tape(&mut pm, &text, 320.0, &trades, dec, tz, None, ("Hyperliquid", None), &Theme::default());
        crate::text::ecrits()
    };
    let trade = |px: &str, sz: &str, ms: i64, buy: bool| {
        format!(
            r#"{{"coin":"COIN","side":"{}","px":"{px}","sz":"{sz}","time":{ms},"hash":"0x0","tid":1,"users":["0xa","0xb"]}}"#,
            if buy { "B" } else { "A" }
        )
    };
    let message = |trades: &[String]| format!(r#"{{"channel":"trades","data":[{}]}}"#, trades.join(","));
    // three trades written by hand, with the hour a clock in Toronto shows in winter (UTC − 5 h)
    let by_hand = message(&[
        trade("97123.5", "0.00016", 1_793_050_123_456, true),
        trade("97120.0", "1234.5", 1_793_053_723_999, false),
        trade("97119.5", "12.5", 1_793_059_199_000, true),
    ]);
    let e = draw(&by_hand, 1, -5 * 3600);
    assert_eq!(e[..4], ["Hyperliquid", "Prix", "Taille", "Heure"], "{e:?}");
    assert_eq!(
        e[4..],
        [
            "97\u{202f}123,5",
            "0,0002",
            "16:28:43",
            "97\u{202f}120,0",
            "1\u{202f}234",
            "17:28:43",
            "97\u{202f}119,5",
            "12,5000",
            "18:59:59"
        ]
    );
    // by hand: a trade that cannot be is left out, before or after one that can, and nothing else moves (« NaN » with
    // a size of « -3 » was seen written on 23 Nov 2026); alone, it leaves the list waiting
    let can = ("97123.5", "0.25");
    let without = tape_texts(&[can]);
    assert_eq!(without, ["Hyperliquid", "Prix", "Taille", "Heure", "97\u{202f}123,5", "0,2500", "21:28:43"]);
    let waiting = ["Hyperliquid", "En attente des trades de Hyperliquid…"];
    assert_eq!(tape_texts(&[]), waiting);
    assert_eq!(tape_texts(&[("NaN", "-3"), can]), without);
    for no in MAD {
        for cannot in [(no, "0.25"), ("97120.0", no)] {
            assert_eq!(
                tape_texts(&[cannot, can]),
                without,
                "the trade {cannot:?} cannot be: the list is written without it"
            );
            assert_eq!(
                tape_texts(&[can, cannot]),
                without,
                "the trade {cannot:?} cannot be: the list is written without it"
            );
            assert_eq!(tape_texts(&[cannot]), waiting, "the trade {cannot:?} cannot be: nothing is written");
        }
    }
    // made-up trades; now and then one cannot be (a price or a size of [`MAD`], or a size drawn as zero)
    let mut d = Draw(0x5EED_0004);
    let (mut left_out, mut written_trades, mut none) = (0, 0, 0);
    for round in 0..300 {
        let (dec, tz) = (d.below(7) as u32, (d.below(27) as i64 - 12) * 3600);
        let came: Vec<(String, String, i64, bool)> = (0..1 + d.below(20))
            .map(|_| {
                let (int, size_int, size_decimals) = (1 + d.below(6) as u32, d.below(6) as u32, d.below(6) as u32);
                let (px, sz) = (d.decimal(int, dec), d.decimal(size_int, size_decimals));
                let (px, sz) = match d.below(16) {
                    0 => (mad(&mut d), sz),
                    1 => (px, mad(&mut d)),
                    _ => (px, sz),
                };
                (px, sz, 1_700_000_000_000 + d.below(900_000_000_000) as i64, d.below(2) == 0)
            })
            .collect();
        let e = draw(
            &message(&came.iter().map(|(px, sz, ms, buy)| trade(px, sz, *ms, *buy)).collect::<Vec<_>>()),
            dec as usize,
            tz,
        );
        // by hand: the trades that can be, in the order they came
        let kept: Vec<&(String, String, i64, bool)> = came.iter().filter(|(px, sz, ..)| can_be(px) && can_be(sz)).collect();
        left_out += came.len() - kept.len();
        if kept.is_empty() {
            assert_eq!(e, waiting, "round {round}: no trade that can be came");
            none += 1;
            continue;
        }
        assert_eq!(e[..4], ["Hyperliquid", "Prix", "Taille", "Heure"], "round {round}: {e:?}");
        assert_eq!(e.len(), 4 + 3 * kept.len(), "round {round}: a row for each trade that can be, {e:?}");
        for (k, (px, sz, ms, _)) in kept.iter().enumerate() {
            let [price, size, hour] = [&e[4 + 3 * k], &e[5 + 3 * k], &e[6 + 3 * k]];
            assert_eq!(price, &written(px, dec as usize, DEFAULT), "round {round}, trade {k}: the price « {px} »");
            let shown = read(size);
            let size_came = Dec::of(sz);
            assert_eq!(
                shown.scale,
                if size_came.at(5) >= 1000 * 100_000 { 0 } else { 4 },
                "round {round}, trade {k}: « {size} »"
            );
            assert!(
                size_came.rounds_to(shown),
                "round {round}, trade {k}: the size came as « {sz} », the tape writes « {size} »"
            );
            let s = (ms.div_euclid(1000) + tz).rem_euclid(86_400);
            assert_eq!(
                hour,
                &format!("{:02}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60),
                "round {round}, trade {k}: {ms} ms, {tz} s"
            );
            written_trades += 1;
        }
    }
    assert!(
        left_out > 300 && written_trades > 2000 && none > 0,
        "{left_out} trades left out, {written_trades} written, {none} lists left waiting: each must be seen"
    );
}

/// 👁 The Watch module writes the market that came: the exchange's price, the change against the price of 24 hours
/// ago, the highest and the lowest of the day's candles, and the compact book (the best sales, the spread, the best
/// purchases) read from the exchange's own message.
#[test]
fn the_watch_card_writes_the_market_that_came() {
    // 48 candles of one hour: the day is the last 24
    let came: Vec<[String; 4]> = (0..48)
        .map(|i| {
            [
                format!("{}.5", 97_000 + i * 3),
                format!("{}.5", 97_020 + i * 3),
                format!("{}.0", 96_980 + i * 3),
                format!("{}.0", 97_010 + i * 3),
            ]
        })
        .collect();
    let p = |s: &str| -> f64 { s.parse().unwrap() };
    let bars: Vec<Bar> = came
        .iter()
        .enumerate()
        .map(|(i, r)| Bar {
            time: 1_700_000_000 + i as i64 * 3600,
            open: p(&r[0]),
            high: p(&r[1]),
            low: p(&r[2]),
            close: p(&r[3]),
            volume: 2.0,
        })
        .collect();
    let (mark, prev) = ("97155.5", "96000.0");
    let live = crate::dock::Live {
        mark: p(mark),
        prev: p(prev),
        volume: 1_234_567_890.0,
    };
    let bids = [("97155.0", "0.25"), ("97154.5", "1.5"), ("97150.0", "12.125"), ("97100.0", "99")];
    let asks = [("97156.0", "0.7"), ("97156.5", "4.001"), ("97160.0", "0.333"), ("97200.0", "99")];
    let own = |l: &[(&str, &str)]| l.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect::<Vec<_>>();
    let book = argos_data::book::parse_book(&l2book(&own(&bids), &own(&asks))).unwrap();
    let rows = crate::dock::watch_rows(&bars, 3600, Some(live), Some(&book), 1);
    let row = |name: &str| &rows.iter().find(|r| r.0 == name).unwrap_or_else(|| panic!("no row « {name} » in {rows:?}")).1;
    assert_eq!(row("Prix"), &written(mark, 1, DEFAULT));
    // +1,20 %: 97 155,5 against 96 000,0 is +1,2036… %
    assert_eq!(row("Variation 24 h"), "+1,20 %");
    // the day's candles are the last 24: the highest high is the last one's, the lowest low the 25th's
    assert_eq!(row("Plus haut 24 h"), &written(&came[47][1], 1, DEFAULT));
    assert_eq!(row("Plus bas 24 h"), &written(&came[24][2], 1, DEFAULT));
    assert_eq!(row("Volume 24 h"), "1,23 G$");
    let names: Vec<&str> = rows.iter().map(|r| r.0.as_str()).collect();
    assert_eq!(
        names[5..],
        [
            "Vente 97\u{202f}160,0",
            "Vente 97\u{202f}156,5",
            "Vente 97\u{202f}156,0",
            "Écart",
            "Achat 97\u{202f}155,0",
            "Achat 97\u{202f}154,5",
            "Achat 97\u{202f}150,0"
        ],
        "the three best levels of each side, the best next to the spread"
    );
    let sizes: Vec<&str> = rows[5..].iter().map(|r| r.1.as_str()).collect();
    assert_eq!(sizes, ["0,333", "4,001", "0,700", "1,0 · 0,001 %", "0,250", "1,500", "12,125"]);
}

/// ⭐ A watchlist row writes its price as the exchange wrote it: the same digits, the same decimals.
#[test]
fn the_watchlist_writes_the_price_as_the_exchange_wrote_it() {
    let by_hand = [
        ("61373.0", "61\u{202f}373,0"),
        ("0.00000712", "0,00000712"),
        ("1234567.89", "1\u{202f}234\u{202f}567,89"),
        ("97000", "97\u{202f}000"),
    ];
    for (mark, want) in by_hand {
        assert_eq!(crate::watchlist::Row::new("COIN", "COIN-USDC", mark, 1.0).prix_affiche(), want, "« {mark} »");
    }
    let mut d = Draw(0x5EED_0005);
    for _ in 0..4000 {
        let (int, k) = (d.below(8) as u32, d.below(11) as u32);
        if int + k > 15 {
            continue;
        }
        let mark = d.decimal(int, k);
        let row = crate::watchlist::Row::new("COIN", "COIN-USDC", &mark, 1.0);
        assert_eq!(row.prix_affiche(), written(&mark, k as usize, DEFAULT), "« {mark} »");
    }
    assert_eq!(
        crate::watchlist::Row::new("COIN", "COIN-USDC", "", 1.0).prix_affiche(),
        "",
        "no price yet: nothing written"
    );
}

/// 💸 The funding rate of the market details is the hourly rate that came, in percent, rounded to its four decimals,
/// and the time left before the next payment, on the hour (its decimal mark: see [`SEEN`]).
#[test]
fn the_funding_rate_is_the_rate_that_came() {
    let mut d = Draw(0x5EED_0006);
    for _ in 0..2000 {
        let rate = format!("{}0.000{}", if d.below(2) == 0 { "-" } else { "" }, d.decimal(1, 5).replace('.', ""));
        let now = 1_700_000_000 + d.below(1_000_000) as i64;
        let text = crate::topbar::funding_text(rate.parse().unwrap(), now);
        let (pct, left) = text.split_once(" % · ").unwrap_or_else(|| panic!("« {text} »"));
        let shown = read_raw(pct);
        assert_eq!(shown.scale, 4, "« {text} »: four decimals");
        // the rate is a fraction of 1: in percent, two decimals fewer
        let came = Dec::of(&rate);
        let came = Dec {
            m: came.m,
            scale: came.scale - 2,
        };
        assert!(came.rounds_to(shown), "the rate came as « {rate} », written « {text} »");
        let s = 3600 - now.rem_euclid(3600);
        assert_eq!(left, format!("{:02}:{:02}", s / 60, s % 60), "« {text} » at {now}");
    }
}

/// 🔎 What these tests saw on the screen that is not what was expected: (what, what the screen writes today, what
/// was expected). Each stays here until it is decided and fixed; the day one is fixed, the test below fails and says
/// so: remove the entry here and add its case to the test of its screen. Nothing here is hidden from Jee, who
/// decides each line (the list was delivered with the tests, 23 Nov 2026).
const SEEN: [(&str, fn() -> String, &str, &str); 14] = [
    // 1. a negative number too small for the decimals written keeps its minus: « −0,00 »
    ("a number written as zero keeps a minus", || format_number_in(-0.001, 2, 0), "−0,00", "0,00"),
    ("an amount written as zero keeps a minus", || format_money_in(-0.001, 2, 1), "−$0.00", "$0.00"),
    ("a short volume written as zero keeps a hyphen", || kf_in(-0.2, 0), "-0", "0"),
    (
        "a funding rate written as zero keeps a hyphen",
        || crate::topbar::funding_text(-0.000_000_1, 1),
        "-0.0000 % · 59:59",
        "0,0000 % · 59:59",
    ),
    // 2. numbers written by hand, without the number format: a point where the format has a comma, a hyphen for the
    // minus (Jee, 22 Nov, on another screen: « +1.59 % »)
    (
        "the change of the legend has a point",
        || legend(&chart_texts(&candles(MARKETS[1].1, "1")))[4].clone(),
        "+0.03 %",
        "+0,03 %",
    ),
    (
        "the funding rate has a point",
        || crate::topbar::funding_text(0.000_012_5, 1),
        "0.0013 % · 59:59",
        "0,0013 % · 59:59",
    ),
    (
        "a negative funding rate has a hyphen",
        || crate::topbar::funding_text(-0.000_1, 1),
        "-0.0100 % · 59:59",
        "−0,0100 % · 59:59",
    ),
    ("a negative short volume has a hyphen", || kf_in(-1500.0, 0), "-1,5K", "−1,5K"),
    (
        "the 24 h range of the key indicators has a point",
        || key_row("Amplitude 24 h"),
        "2.00 %",
        "2,00 %",
    ),
    // 3. just under a thousand of the unit, the number is rounded up but the unit does not follow
    ("a short volume just under a thousand", || kf_in(999.6, 0), "1000", "1,0K"),
    ("a short volume just under a million", || kf_in(999_950.0, 0), "1000K", "1,0M"),
    (
        "the legend's volume just under a million",
        || crate::viewport::format_volume(999_999.0),
        "1\u{202f}000,00 K",
        "1,00 M",
    ),
    (
        "a big number just under a million",
        || crate::topbar::big(999_950.0),
        "1\u{202f}000,0 k",
        "1,0 M",
    ),
    // 4. (fixed 24 Nov 2026, branch interface-perps-1: every digit that came under 1, see MARKETS)
    // 5. a size that is not zero is written as zero when the book's largest level sets few decimals
    (
        "a small size is written as zero in the book",
        || book_texts(r#"{"px":"97125.0","sz":"0.00015","n":1}"#)[4].clone(),
        "0,00",
        "a size that is not zero",
    ),
];

/// The row `name` of the key indicators of 48 one-hour candles between 99 and 101, closed at 100.
fn key_row(name: &str) -> String {
    let bars: Vec<Bar> = (0..48)
        .map(|i| Bar {
            time: 1_700_000_000 + i * 3600,
            open: 100.0,
            high: 101.0,
            low: 99.0,
            close: 100.0,
            volume: 1.0,
        })
        .collect();
    let rows = crate::dock::key_stats(&bars, 3600);
    rows.iter()
        .find(|r| r.0 == name)
        .unwrap_or_else(|| panic!("no row « {name} » in {rows:?}"))
        .1
        .clone()
}

/// The texts of a Book whose best sale is `level` (an `l2Book` level; none when empty), next to a level of 12,5 at
/// 97 126 and a purchase.
fn book_texts(level: &str) -> Vec<String> {
    let comma = if level.is_empty() { "" } else { "," };
    let msg = format!(
        r#"{{"channel":"l2Book","data":{{"coin":"COIN","time":1,"levels":[[{{"px":"97123.0","sz":"1.5","n":1}}],[{level}{comma}{{"px":"97126.0","sz":"12.5","n":1}}]]}}}}"#
    );
    let book = argos_data::book::parse_book(&msg).unwrap();
    let mut pm = crate::px::new(320, 400).unwrap();
    let _ = crate::text::ecrits();
    crate::dock::draw_book(
        &mut pm,
        &crate::text::Text::default(),
        320.0,
        400.0,
        Some(&book),
        1,
        &Theme::default(),
        &crate::dock::CarnetSource::Hyperliquid,
    );
    crate::text::ecrits()
}

/// The texts of a tape with these trades, (price, size) as Hyperliquid writes them.
fn tape_texts(trades: &[(&str, &str)]) -> Vec<String> {
    let trades: Vec<String> = trades
        .iter()
        .map(|(px, sz)| format!(r#"{{"coin":"COIN","side":"B","px":"{px}","sz":"{sz}","time":1793050123456,"users":["0xa","0xb"]}}"#))
        .collect();
    let msg = format!(r#"{{"channel":"trades","data":[{}]}}"#, trades.join(","));
    let trades = argos_data::whales::parse_all(&msg);
    let mut pm = crate::px::new(320, 200).unwrap();
    let _ = crate::text::ecrits();
    crate::dock::draw_tape(
        &mut pm,
        &crate::text::Text::default(),
        320.0,
        &trades,
        1,
        0,
        None,
        ("Hyperliquid", None),
        &Theme::default(),
    );
    crate::text::ecrits()
}

/// 🔎 [`SEEN`] says the truth: each thing seen is still written as the list says. A failure here is good news or a
/// change to look at: the screen writes something else than on the day it was seen.
#[test]
fn what_was_seen_is_still_there() {
    for (what, now, today, expected) in SEEN {
        let got = now();
        assert_eq!(
            got, today,
            "« {what} »: the screen now writes « {got} » (it wrote « {today} »; expected: « {expected} »). If it is fixed, remove it from SEEN and add its case to the test of its screen."
        );
        assert_ne!(today, expected, "« {what} »: listed as seen, yet it is what was expected");
    }
}
