//! 📼 Test 3 of the data reliability: the answers the exchanges gave on 24 Nov. 2026 (`tests/recorded/2026-11-24/`),
//! read again by the app's own code with no network, agree with one another. No number is written by hand: an hour
//! is checked against its sixty minutes and its twelve candles of five minutes, a day against its twenty-four hours
//! and its six candles of four hours, a market against the same market on the other exchanges.
//!
//! What differs is named (the same name from one run to the next). A difference seen on that day and understood is
//! listed in `SEEN` with its reason: a new one fails, and so does one that is gone (the list then says too much).

mod recorded;

use argos_core::Bar;
use argos_data::murs::Carnet;
use argos_data::{Checked, Exchange};
use recorded::{Entry, Replay, Window, CANDLES};
use std::collections::BTreeMap;

/// The day the answers were taken: the name of their folder.
const DAY: &str = "2026-11-24";

/// The entries of one file of that day, and the moment they were taken (milliseconds).
fn kept(name: &str) -> (i64, Vec<Entry>) {
    let path = format!("{}/tests/recorded/{DAY}/answers-{name}.txt", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let taken = recorded::taken(&text).unwrap_or_else(|| panic!("{path}: no note « taken »"));
    (taken, recorded::read(&text).unwrap_or_else(|e| panic!("{path}: {e}")))
}

/// The entries of one question: who answered, which kind of data.
fn of(entries: &[Entry], who: &str, kind: &str) -> Vec<Entry> {
    entries.iter().filter(|e| e.who == who && e.kind == kind).cloned().collect()
}

/// What the checks found: a name that stays the same from one run to the next, and what was seen.
#[derive(Default)]
struct Found(BTreeMap<String, String>);

impl Found {
    /// Notes what was seen under `name` (after what was already seen under that name).
    fn note(&mut self, name: impl Into<String>, what: impl Into<String>) {
        let what = what.into();
        self.0
            .entry(name.into())
            .and_modify(|w| {
                w.push_str(" ; ");
                w.push_str(&what);
            })
            .or_insert(what);
    }

    /// The kept answers one question did not use, and what it asked that was not kept.
    fn replay(&mut self, name: &str, r: &Replay) {
        if !r.unknown().is_empty() {
            self.note(format!("{name}: asks what was not asked that day"), format!("{:?}", r.unknown()));
        }
        let left: Vec<String> = r.left().iter().map(|e| format!("{:?}", e.asked)).collect();
        if !left.is_empty() {
            self.note(format!("{name}: an answer of that day is not asked again"), left.join(" ; "));
        }
    }

    /// Fails unless what was found is exactly what `seen` lists, each name once.
    fn agrees_with(&self, seen: &[(&str, &str)]) {
        let report: Vec<String> = self.0.iter().map(|(n, w)| format!("{n} · {w}")).collect();
        let new: Vec<&String> = self.0.keys().filter(|n| !seen.iter().any(|(s, _)| s == n)).collect();
        let gone: Vec<&str> = seen.iter().map(|(s, _)| *s).filter(|s| !self.0.contains_key(*s)).collect();
        assert!(
            new.is_empty() && gone.is_empty(),
            "\nnew differences: {new:#?}\nlisted as seen, not found any more: {gone:#?}\neverything found:\n{}",
            report.join("\n")
        );
    }
}

/// `a` and `b` apart by more than `tol` (a share of `b`).
fn apart(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() > tol * b.abs()
}

/// The middle value.
fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(f64::total_cmp);
    match v.len() {
        0 => f64::NAN,
        n if n % 2 == 1 => v[n / 2],
        n => (v[n / 2 - 1] + v[n / 2]) / 2.0,
    }
}

/// An hour of the day of the answers, as people read it (UTC); « -1d » the day before.
fn at(t: i64, day: i64) -> String {
    let back = if t < day {
        format!(" -{}d", (day - t + 86_399) / 86_400)
    } else {
        String::new()
    };
    format!("{:02}:{:02}{back}", t.rem_euclid(86_400) / 3600, t.rem_euclid(3600) / 60)
}

/// The fields of candle `a` that are not those of `b`: prices apart by more than `tol`, volumes by more than `tol_vol`.
fn differ(a: &Bar, b: &Bar, tol: f64, tol_vol: f64) -> Vec<String> {
    let mut out = vec![];
    if a.time != b.time {
        out.push(format!("time {} ≠ {}", a.time, b.time));
    }
    for (name, x, y) in [
        ("open", a.open, b.open),
        ("high", a.high, b.high),
        ("low", a.low, b.low),
        ("close", a.close, b.close),
    ] {
        if apart(x, y, tol) {
            out.push(format!("{name} {x} ≠ {y}"));
        }
    }
    if apart(a.volume, b.volume, tol_vol) {
        out.push(format!("volume {} ≠ {}", a.volume, b.volume));
    }
    out
}

/// The candles of every exchange, by (exchange, question): only those that are the candles asked.
type Series = BTreeMap<(&'static str, &'static str), Vec<Bar>>;

/// Every candle question of every exchange, read again by the app's code. A question not read, or whose candles are
/// not the ones asked, is noted once here and kept out of the checks that follow.
async fn every_candle(entries: &[Entry], found: &mut Found) -> Series {
    let (hour, day) = recorded::hour_and_day();
    let week = recorded::this_week();
    let now = recorded::clock() / 1000;
    let mut all = BTreeMap::new();
    for ex in Exchange::ALL {
        for c in &CANDLES {
            let name = format!("{} {}", ex.id(), c.kind);
            let replay = Replay::new(&of(entries, ex.id(), c.kind));
            let got = recorded::candles(replay.clone(), ex, c).await;
            found.replay(&name, &replay);
            let Checked { bars, a_reparer } = match got {
                Ok(k) => k,
                Err(e) => {
                    found.note(format!("{name}: not read"), e.to_string());
                    continue;
                }
            };
            if !a_reparer.is_empty() {
                let times: Vec<String> = a_reparer.iter().map(|t| at(*t, day)).collect();
                found.note(format!("{name}: candles refused"), times.join(" "));
            }
            let sec = c.tf.seconds();
            let want: Vec<i64> = match c.window {
                Window::Newest(n) => {
                    let last = bars.last().map_or(0, |b| b.time);
                    if !(now - 2 * sec..=now + 300).contains(&last) {
                        found.note(format!("{name}: the newest candle is not new"), at(last, day));
                    }
                    (0..n as i64).rev().map(|k| last - k * sec).collect()
                }
                Window::BeforeTheHour(n) => (1..=n as i64).rev().map(|k| hour - k * sec).collect(),
                Window::SinceYesterday => (day - 86_400..hour).step_by(sec as usize).collect(),
                Window::BeforeToday(n) => (1..=n as i64).rev().map(|k| day - k * sec).collect(),
                Window::BeforeThisWeek(n) => (1..=n as i64).rev().map(|k| week - k * sec).collect(),
            };
            let times: Vec<i64> = bars.iter().map(|b| b.time).collect();
            if times != want {
                let missing: Vec<String> = want.iter().filter(|t| !times.contains(t)).map(|t| at(*t, day)).collect();
                let more: Vec<String> = times.iter().filter(|t| !want.contains(t)).map(|t| at(*t, day)).collect();
                found.note(
                    format!("{name}: not the candles asked"),
                    format!("{} candles, missing {missing:?}, more {more:?}", times.len()),
                );
                continue;
            }
            all.insert((ex.id(), c.kind), bars);
        }
    }
    all
}

/// The candle of `kind` that begins at `t`.
fn one(all: &Series, id: &'static str, kind: &'static str, t: i64) -> Option<Bar> {
    all.get(&(id, kind))?.iter().find(|b| b.time == t).copied()
}

/// One candle of `sec` seconds made of the candles of `kind` that begin in [from, from + sec).
fn made_of(all: &Series, id: &'static str, kind: &'static str, from: i64, sec: i64) -> Option<Bar> {
    let bars: Vec<Bar> = all.get(&(id, kind))?.iter().filter(|b| (from..from + sec).contains(&b.time)).copied().collect();
    argos_core::aggregate(&bars, sec).into_iter().next()
}

/// Differences seen that day, and why: facts of the exchanges first, then defects of the app shown to Jee on
/// 24 Nov. 2026 (a defect that is corrected leaves this list in the same change).
const SEEN: &[(&str, &str)] = &[
    (
        "bitfinex candles-1m-small: not the candles asked",
        "Bitfinex sends no candle for a minute without a trade (DOGE trades little there): the chart leaves it empty",
    ),
    ("bitfinex-spot candles-1m-small: not the candles asked", "the same on Bitfinex's spot market"),
    (
        "bitstamp candles-1m-newest: not the candles asked",
        "the same on Bitstamp (no trade of bitcoin in the minute of 05:42 that day), which sends the minute in progress",
    ),
    (
        "bitstamp live-candles: last price",
        "its newest candles are left out above (a minute without a trade): nothing to compare the live price with",
    ),
    (
        "bitfinex-spot week: low far from the others",
        "the week of 16 Nov. 2026 fell to about 55 100 in a quick drop; on Bitfinex's spot market its low stopped at 55 501",
    ),
    ("bitstamp candles-1w: not read", "Bitstamp has no candles of one week"),
    ("coinbase candles-4h: not read", "Coinbase has no candles of 4 hours"),
    ("coinbase candles-12h: not read", "Coinbase has no candles of 12 hours"),
    ("coinbase candles-1w: not read", "Coinbase has no candles of one week"),
    (
        "gate candles-12h: not read",
        "the app does not ask Gate.io for candles of 12 hours: none in Gate.io's documentation (to try in a recording)",
    ),
    ("kraken candles-12h: not read", "Kraken has no candles of 12 hours"),
    ("mexc candles-12h: not read", "MEXC has no candles of 12 hours"),
    (
        "coinbase markets: price far from the others",
        "DOGE priced in bitcoin: one step of its price, 0.00000001, is 1 % of it",
    ),
    (
        "kucoin markets: price 24 h ago far from the others",
        "the same step of price, in DOGE priced in bitcoin",
    ),
    (
        "coinbase markets: price 24 h ago far from the others",
        "SOL moved fast 24 hours before, and the exchanges do not take their price 24 h ago at the same second \
         (Coinbase 71.77, Binance 71.96, OKX 72.15)",
    ),
    (
        "kraken markets: no price 24 h ago",
        "Kraken gives its price at 00:00 UTC, not 24 hours before: the app shows « — » (Jee's choice, 24 Nov. 2026)",
    ),
    (
        "mexc markets: price 24 h ago far from the others",
        "MEXC counts its 24 hours from the start of the hour 24 hours before (05:00 for an answer at 05:46): prices \
         rose 0.6 to 1.8 % in those 46 minutes that day; the app shows MEXC's own figure",
    ),
    (
        "gate candles-1w: not the candles asked",
        "Gate.io's weeks begin on Thursday at 00:00 UTC (« 7d », counted from 1 Jan. 1970, a Thursday), not on Monday: \
         the app makes its weeks of days (the chart, AI Trend, the scripts), from Monday",
    ),
    ("hl candles-1w: not the candles asked", "the same on Hyperliquid (« 1w »)"),
    ("kraken candles-1w: not the candles asked", "the same on Kraken (10 080 minutes)"),
    ("kucoin candles-1w: not the candles asked", "the same on KuCoin (« 1week »)"),
];

#[tokio::test]
async fn the_answers_of_one_day_agree_with_one_another() {
    let mut found = Found::default();

    // 🕯 the candles: each question read again, the same questions as that day, the candles asked
    let (taken, entries) = kept("candles");
    recorded::set_clock(taken);
    let (hour, day) = recorded::hour_and_day();
    let all = every_candle(&entries, &mut found).await;

    // ⏱ within one exchange: the last closed hour is its sixty minutes and its twelve candles of five minutes;
    // yesterday is its twenty-four hours, its six candles of four hours and its two halves of twelve hours (each
    // twelve of its hours); last week is its seven days (a week begins on Monday 00:00 UTC)
    // (the prices of a candle are the prices of its parts, to the digit; its volume is their sum to a thousandth: an
    // exchange may round the volume of each candle, BloFin counts whole contracts)
    let (h, d, w) = (hour - 3600, day - 86_400, recorded::this_week() - 604_800);
    for ex in Exchange::ALL {
        let id = ex.id();
        let checks = [
            ("hour", one(&all, id, "candles-1h", h), "60 x 1m", made_of(&all, id, "candles-1m", h, 3600)),
            ("hour", one(&all, id, "candles-1h", h), "12 x 5m", made_of(&all, id, "candles-5m", h, 3600)),
            ("day", one(&all, id, "candles-1d", d), "24 x 1h", made_of(&all, id, "candles-1h", d, 86_400)),
            ("day", one(&all, id, "candles-1d", d), "6 x 4h", made_of(&all, id, "candles-4h", d, 86_400)),
            (
                "12 hours",
                one(&all, id, "candles-12h", d),
                "12 x 1h",
                made_of(&all, id, "candles-1h", d, 43_200),
            ),
            (
                "12 hours",
                one(&all, id, "candles-12h", d + 43_200),
                "12 x 1h",
                made_of(&all, id, "candles-1h", d + 43_200, 43_200),
            ),
            (
                "week",
                one(&all, id, "candles-1w", w),
                "7 x 1d",
                made_of(&all, id, "candles-1d-week", w, 604_800),
            ),
        ];
        for (what, theirs, parts, made) in checks {
            // (a question not read, or not answered with the candles asked, is already noted)
            let (Some(theirs), Some(made)) = (theirs, made) else {
                continue;
            };
            let diff = differ(&theirs, &made, 1e-9, 1e-3);
            if !diff.is_empty() {
                found.note(format!("{id} {what} ≠ {parts}"), diff.join(", "));
            }
        }
    }

    // 🌍 across exchanges: the same hour, the same day and the last minute of the small coin, near the others; the
    // volume of the hour and of the day in the same unit (an exchange trades 1 000 times less than another, never
    // 50 times more than the middle one)
    for (what, kind, t) in [
        ("hour", "candles-1h", h),
        ("day", "candles-1d", d),
        ("12 hours", "candles-12h", d + 43_200),
        ("week", "candles-1w", w),
        ("DOGE minute", "candles-1m-small", hour - 60),
    ] {
        let bars: Vec<(&str, Bar)> = Exchange::ALL.iter().filter_map(|ex| Some((ex.id(), one(&all, ex.id(), kind, t)?))).collect();
        for (field, get) in [
            ("open", (|b: &Bar| b.open) as fn(&Bar) -> f64),
            ("high", |b| b.high),
            ("low", |b| b.low),
            ("close", |b| b.close),
        ] {
            let mid = median(bars.iter().map(|(_, b)| get(b)).collect());
            for (id, b) in &bars {
                if apart(get(b), mid, 0.005) {
                    found.note(format!("{id} {what}: {field} far from the others"), format!("{} (the others: {mid})", get(b)));
                }
            }
        }
        if what == "DOGE minute" {
            continue;
        }
        let mid = median(bars.iter().map(|(_, b)| b.volume).collect());
        for (id, b) in &bars {
            if b.volume > 50.0 * mid || b.volume < mid / 1000.0 {
                found.note(
                    format!("{id} candles: volume far from the others (another unit?)"),
                    format!("{what} {} (the middle one: {mid})", b.volume),
                );
            }
        }
    }

    // 🏷 the markets: each list read again; a coin's price and its price 24 h ago within 0.5 % of the other
    // exchanges' (a dollar and a tether differ by a tenth of that; a change of 24 h read the wrong way, by twice the change).
    // A price 24 h ago not given (0, shown « — ») is not compared: an exchange without it for the coin is noted.
    let (taken, entries) = kept("markets");
    recorded::set_clock(taken);
    let mut prices: BTreeMap<String, Vec<(&str, f64, f64)>> = BTreeMap::new();
    for ex in Exchange::ALL {
        let name = format!("{} markets", ex.id());
        let replay = Replay::new(&of(&entries, ex.id(), "markets"));
        let got = recorded::markets(replay.clone(), ex).await;
        found.replay(&name, &replay);
        match got {
            Ok(list) => {
                if !list.iter().any(|m| m.base == recorded::COIN) {
                    found.note(format!("{name}: no {}", recorded::COIN), format!("{} markets", list.len()));
                }
                for m in list {
                    if m.base == recorded::COIN && m.prev <= 0.0 {
                        found.note(format!("{name}: no price 24 h ago"), format!("{} {}", m.base, m.prev));
                    }
                    prices.entry(m.base.clone()).or_default().push((ex.id(), m.price, m.prev));
                }
            }
            Err(e) => found.note(format!("{name}: not read"), e.to_string()),
        }
    }
    for (base, list) in prices.iter().filter(|(_, l)| l.len() >= 2) {
        let known: Vec<f64> = list.iter().map(|l| l.2).filter(|q| *q > 0.0).collect();
        let (price, prev) = (median(list.iter().map(|l| l.1).collect()), median(known.clone()));
        for (id, p, q) in list {
            if apart(*p, price, 0.005) {
                found.note(format!("{id} markets: price far from the others"), format!("{base} {p} (the others: {price})"));
            }
            if *q > 0.0 && known.len() >= 2 && apart(*q, prev, 0.005) {
                found.note(
                    format!("{id} markets: price 24 h ago far from the others"),
                    format!("{base} {q} (the others: {prev})"),
                );
            }
        }
    }

    // 📚 the order books: each one read again, its best bid under its best ask, its middle near the others'
    let (taken, entries) = kept("books");
    recorded::set_clock(taken);
    let mut mids: Vec<(String, f64)> = vec![];
    for c in Carnet::PERP.into_iter().chain(Carnet::SPOT) {
        let who = format!("{c:?}");
        let replay = Replay::new(&of(&entries, &who, "book"));
        let got = recorded::book(&replay, c).await;
        found.replay(&format!("{who} book"), &replay);
        match got {
            Ok(b) => match (b.bids.first(), b.asks.first()) {
                (Some(bid), Some(ask)) if bid.0 < ask.0 => mids.push((who, (bid.0 + ask.0) / 2.0)),
                (bid, ask) => found.note(format!("{who} book: no best bid under the best ask"), format!("{bid:?} {ask:?}")),
            },
            Err(e) => found.note(format!("{who} book: not read"), e.to_string()),
        }
    }
    for (ex, perp) in [(Exchange::Bitfinex, true), (Exchange::BitfinexSpot, false)] {
        let replay = Replay::new(&of(&entries, ex.id(), "book"));
        let got = recorded::bitfinex_book(&replay, perp).await;
        found.replay(&format!("{} book", ex.id()), &replay);
        match got {
            Ok(b) => match (b.bids.first(), b.asks.first()) {
                (Some(bid), Some(ask)) if bid.px < ask.px => mids.push((ex.id().to_string(), (bid.px + ask.px) / 2.0)),
                (bid, ask) => found.note(format!("{} book: no best bid under the best ask", ex.id()), format!("{bid:?} {ask:?}")),
            },
            Err(e) => found.note(format!("{} book: not read", ex.id()), e.to_string()),
        }
        let replay = Replay::new(&of(&entries, ex.id(), "trades"));
        let got = recorded::bitfinex_trades(&replay, perp).await;
        found.replay(&format!("{} trades", ex.id()), &replay);
        match got {
            Ok(t) if !t.is_empty() => mids.push((format!("{} trades", ex.id()), median(t.iter().map(|t| t.price).collect()))),
            Ok(_) => found.note(format!("{} trades: none", ex.id()), ""),
            Err(e) => found.note(format!("{} trades: not read", ex.id()), e.to_string()),
        }
    }
    let middle = median(mids.iter().map(|m| m.1).collect());
    for (who, mid) in &mids {
        if apart(*mid, middle, 0.005) {
            found.note(format!("{who}: price far from the others"), format!("{mid} (the others: {middle})"));
        }
    }

    // 💸 the funding rates: each one read again, a rate for 8 hours that can be
    let (taken, entries) = kept("funding");
    recorded::set_clock(taken);
    for ex in Exchange::ALL {
        let asked = of(&entries, ex.id(), "funding");
        if asked.is_empty() {
            continue;
        }
        let replay = Replay::new(&asked);
        let got = recorded::funding(&replay, ex).await;
        found.replay(&format!("{} funding", ex.id()), &replay);
        match got {
            Some(r) if r.is_finite() && r.abs() < 0.003 => {}
            r => found.note(format!("{} funding: no rate that can be", ex.id()), format!("{r:?}")),
        }
    }

    // 📡 the live streams of the charts: each message read by the app's code, the last price near the exchange's own
    // newest candle
    let (_, entries) = kept("live");
    for ex in Exchange::ALL {
        let newest = all.get(&(ex.id(), "candles-1m-newest")).and_then(|b| b.last()).map(|b| b.close);
        for kind in ["live-candles", "live-trades"] {
            let messages = of(&entries, ex.id(), kind);
            if messages.is_empty() {
                continue;
            }
            let name = format!("{} {kind}", ex.id());
            let mut last = None;
            for m in messages.iter().filter_map(|m| m.answer.as_ref().ok()) {
                match argos_data::live::parse_event(ex, m) {
                    Some(argos_data::live::LiveEvent::Bar(b)) => last = Some(b.close),
                    Some(argos_data::live::LiveEvent::Trades(t)) => last = t.last().map(|t| t.price).or(last),
                    _ => {}
                }
            }
            match (last, newest) {
                (Some(p), Some(n)) if !apart(p, n, 0.005) => {}
                (p, n) => found.note(format!("{name}: last price"), format!("{p:?} (its newest candle: {n:?})")),
            }
        }
    }

    found.agrees_with(SEEN);
}
