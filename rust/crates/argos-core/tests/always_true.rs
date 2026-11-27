//! 🔢 Data reliability, test 2: rules that are always true, whatever the data.
//!
//! Made-up series (a fixed draw: the same on every computer and at every run) go through the functions every chart
//! goes through: the cleaning of the candles an exchange answers, their regrouping into a longer period, the Heikin
//! Ashi candles, the replay, and the order flow built from the trades. Each rule is said here once, the long way, and
//! must hold for every series: a rule is never worked out with the function it checks.
//!
//! - **A candle is coherent** ([`coherent`]): it has a time, its prices are numbers above zero, its high is not under
//!   its open or its close, its low is not above them, its volume is a number and is not negative.
//! - **A regrouped candle is the fold of its parts**: the open of the first, the highest high, the lowest low, the
//!   close of the last and the sum of the volumes, dated at the start of its period (the hour on the hour, the day
//!   at 00:00 UTC, the week on Monday, the month on the 1st).
//! - **Nothing is lost and nothing is counted twice**: the volume of the regrouped candles is the volume that came;
//!   a trade given twice is counted once; a trade that is not a number is not counted.
//! - **Never the future**: what a replay shows at an hour is made of what came before that hour.
//!
//! Nothing here is real or anyone's. See also test 1 (`argos-render`, `tests/shown_numbers.rs`): a number written
//! on the screen is the number that came.

use argos_core::feed::clean_bars;
use argos_core::orderflow::{regrouper, Agregateur, FlowBar, TradeFlux};
use argos_core::{aggregate, au_replay, bucket_end, bucket_start, heikin_ashi, se_regroupe, Bar, MONTH_SECONDS};
use std::collections::BTreeMap;

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
    fn below(&mut self, n: u64) -> i64 {
        (self.next() % n) as i64
    }

    /// The same things in another order.
    fn shuffle<T>(&mut self, v: &mut [T]) {
        for k in (1..v.len()).rev() {
            v.swap(k, self.below(k as u64 + 1) as usize);
        }
    }
}

/// A minute, an hour, a day and a week, in seconds.
const MINUTE: i64 = 60;
const HOUR: i64 = 3600;
const DAY: i64 = 86_400;
const WEEK: i64 = 7 * DAY;

/// The periods a chart regroups candles into: the timeframes of the app, then one, three and twelve months.
const PERIODS: [i64; 14] = [
    MINUTE,
    3 * MINUTE,
    5 * MINUTE,
    15 * MINUTE,
    30 * MINUTE,
    HOUR,
    2 * HOUR,
    4 * HOUR,
    12 * HOUR,
    DAY,
    WEEK,
    MONTH_SECONDS,
    3 * MONTH_SECONDS,
    12 * MONTH_SECONDS,
];

/// The rule of a coherent candle, said by hand.
fn coherent(b: &Bar) -> bool {
    let number = |v: f64| v.is_finite() && v > 0.0;
    b.time > 0
        && number(b.open)
        && number(b.high)
        && number(b.low)
        && number(b.close)
        && b.high >= b.open
        && b.high >= b.close
        && b.low <= b.open
        && b.low <= b.close
        && b.volume.is_finite()
        && b.volume >= 0.0
}

/// The first second (UTC) of every month from January 1970 to January 2110, the long way: 365 days a year, one more
/// in a leap year (every 4 years, not every 100, yet every 400).
fn months() -> Vec<i64> {
    let mut out = vec![];
    let mut day = 0;
    for year in 1970..2110 {
        let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
        for days in [31, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31] {
            out.push(day * DAY);
            day += days;
        }
    }
    out.push(day * DAY);
    out
}

/// The period of `sec` seconds that holds the second `t`, the long way: (its first second, the first second of the
/// next). A month starts on the 1st, three months in January, April, July or October, twelve months in January; a
/// week starts on Monday (1 January 1970 was a Thursday); every other period is counted from 00:00 UTC.
fn period(t: i64, sec: i64, months: &[i64]) -> (i64, i64) {
    if sec % MONTH_SECONDS == 0 {
        let n = (sec / MONTH_SECONDS) as usize;
        let k = months.partition_point(|m| *m <= t) - 1;
        let k = k - k % n;
        (months[k], months[k + n])
    } else if sec == WEEK {
        let day = t.div_euclid(DAY);
        let monday = (day - (day + 3).rem_euclid(7)) * DAY;
        (monday, monday + WEEK)
    } else {
        let start = t - t.rem_euclid(sec);
        (start, start + sec)
    }
}

/// Made-up candles `step` seconds apart from `start`, coherent, with a hole of one to twenty candles now and then.
/// Prices are quarters and volumes whole numbers: their sums are exact, in any order.
fn series(d: &mut Draw, n: usize, start: i64, step: i64) -> Vec<Bar> {
    let (mut t, mut last) = (start, 4000 + d.below(4000));
    (0..n)
        .map(|_| {
            if d.below(15) == 0 {
                t += step * (1 + d.below(20));
            }
            let open = (last + d.below(9) - 4).max(1);
            let close = (open + d.below(41) - 20).max(1);
            let (high, low) = (open.max(close) + d.below(10), (open.min(close) - d.below(10)).max(1));
            last = close;
            t += step;
            Bar {
                time: t - step,
                open: open as f64 / 4.0,
                high: high as f64 / 4.0,
                low: low as f64 / 4.0,
                close: close as f64 / 4.0,
                volume: d.below(5001) as f64,
            }
        })
        .collect()
}

/// The candles of `bars` regrouped by period of `sec` seconds, the long way: one candle for each period that has
/// candles, in the order of time.
fn folded(bars: &[Bar], sec: i64, months: &[i64]) -> Vec<Bar> {
    let mut by: BTreeMap<i64, Vec<&Bar>> = BTreeMap::new();
    for b in bars {
        by.entry(period(b.time, sec, months).0).or_default().push(b);
    }
    by.into_iter()
        .map(|(time, parts)| Bar {
            time,
            open: parts[0].open,
            high: parts.iter().map(|b| b.high).fold(f64::MIN, f64::max),
            low: parts.iter().map(|b| b.low).fold(f64::MAX, f64::min),
            close: parts[parts.len() - 1].close,
            volume: parts.iter().map(|b| b.volume).sum(),
        })
        .collect()
}

/// Three made-up markets: minutes over a few days, hours over a few months, days over a few years (1 January 2024
/// was a Monday at 00:00 UTC: 1 704 067 200).
fn markets(d: &mut Draw) -> Vec<(i64, Vec<Bar>)> {
    let start = 1_704_067_200;
    vec![
        (MINUTE, series(d, 6000, start + 7 * HOUR, MINUTE)),
        (HOUR, series(d, 4000, start, HOUR)),
        (DAY, series(d, 1500, start, DAY)),
    ]
}

/// 🧹 The candles kept from an answer are coherent, in the order of time, one per time (the last one received),
/// and no coherent candle is lost, whatever the answer holds: candles out of order, twice, with a price that is not
/// a number, zero or negative, a high under the close, a low above the open, a volume that cannot be, no time.
#[test]
fn the_candles_kept_are_coherent_and_none_is_lost() {
    let mut d = Draw(0x5EED_0101);
    for round in 0..200 {
        let mut answer = series(&mut d, 300, 1_704_067_200, HOUR);
        for _ in 0..40 {
            let mut b = answer[d.below(answer.len() as u64) as usize];
            let mad = [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 0.0, -1.0, -0.0, f64::MAX, 1e300][d.below(8) as usize];
            match d.below(10) {
                0 => b.open = mad,
                1 => b.high = mad,
                2 => b.low = mad,
                3 => b.close = mad,
                4 => b.volume = [f64::NAN, f64::INFINITY, -1.0, -1e-9][d.below(4) as usize],
                5 => b.high = b.close - 0.25,
                6 => b.low = b.open + 0.25,
                7 => b.time = [0, -1, i64::MIN][d.below(3) as usize],
                // the same time once more, coherent: the last one received is the one kept
                8 => b.volume += 1.0,
                _ => (b.open, b.close) = (b.close, b.open),
            }
            answer.push(b);
        }
        d.shuffle(&mut answer);
        let mut want: BTreeMap<i64, Bar> = BTreeMap::new();
        for b in answer.iter().filter(|b| coherent(b)) {
            want.insert(b.time, *b);
        }
        let kept = clean_bars(answer.clone());
        assert!(kept.iter().all(coherent), "round {round}: a candle that is not coherent is kept");
        assert!(
            kept.windows(2).all(|w| w[0].time < w[1].time),
            "round {round}: the candles are not in the order of time"
        );
        assert_eq!(kept, want.into_values().collect::<Vec<_>>(), "round {round}");
    }
}

/// 🕯 A regrouped candle is the fold of its parts, for every period of the app and every market: the open of the
/// first candle, the highest high, the lowest low, the close of the last, the sum of the volumes, dated at the start
/// of its period. The regrouped candles are coherent, in the order of time, and their volume is the volume that came.
#[test]
fn a_regrouped_candle_is_the_fold_of_its_parts() {
    let months = months();
    let mut d = Draw(0x5EED_0102);
    for round in 0..8 {
        for (step, bars) in markets(&mut d) {
            for sec in PERIODS.into_iter().filter(|sec| *sec >= step) {
                let got = aggregate(&bars, sec);
                assert_eq!(got, folded(&bars, sec, &months), "round {round}: candles of {step} s regrouped by {sec} s");
                assert!(
                    got.iter().all(coherent),
                    "round {round}, {step} s by {sec} s: a regrouped candle is not coherent"
                );
                assert!(
                    got.windows(2).all(|w| w[0].time < w[1].time),
                    "round {round}, {step} s by {sec} s: not in the order of time"
                );
                let (came, kept): (f64, f64) = (bars.iter().map(|b| b.volume).sum(), got.iter().map(|b| b.volume).sum());
                assert_eq!(came, kept, "round {round}, {step} s by {sec} s: the volume that came is not the volume kept");
            }
            // the same period: nothing changes
            assert_eq!(aggregate(&bars, step), bars, "round {round}: candles of {step} s regrouped by {step} s");
        }
    }
}

/// 🕯 Regrouping in two steps is regrouping in one, wherever the app says a period regroups into another
/// (`se_regroupe`): minutes into hours then days is minutes into days.
#[test]
fn regrouping_in_two_steps_is_regrouping_in_one() {
    let mut d = Draw(0x5EED_0103);
    let mut pairs = 0;
    for (step, bars) in markets(&mut d) {
        for fine in PERIODS.into_iter().filter(|p| *p >= step) {
            for sec in PERIODS.into_iter().filter(|p| *p > fine && se_regroupe(fine, *p) && se_regroupe(step, fine)) {
                pairs += 1;
                assert_eq!(
                    aggregate(&aggregate(&bars, fine), sec),
                    aggregate(&bars, sec),
                    "candles of {step} s by {fine} s then by {sec} s"
                );
            }
        }
    }
    assert!(pairs > 60, "{pairs} pairs of periods: the hours into days, the days into weeks and months");
    // a week does not fit in a month, nor a quarter of an hour in five minutes, nor two hours in three
    assert!(!se_regroupe(WEEK, MONTH_SECONDS) && !se_regroupe(15 * MINUTE, 5 * MINUTE) && !se_regroupe(2 * HOUR, 3 * HOUR));
}

/// 🗓 The periods tile time: every second belongs to one period, which starts where the long way says (the hour on
/// the hour, the day at 00:00 UTC, the week on Monday, the month on the 1st) and ends where the next one starts.
#[test]
fn the_periods_tile_time() {
    let months = months();
    let mut d = Draw(0x5EED_0104);
    // by hand, from a calendar: Tuesday 29 February 2028, 15:30:00 UTC is 1 835 451 000
    let t = 1_835_451_000;
    assert_eq!(
        (bucket_start(t, HOUR), bucket_start(t, DAY)),
        (1_835_449_200, 1_835_395_200),
        "15:00, then 00:00 the same day"
    );
    assert_eq!(
        (bucket_start(t, WEEK), bucket_start(t, MONTH_SECONDS)),
        (1_835_308_800, 1_832_976_000),
        "Monday 28 February, 1 February"
    );
    assert_eq!(
        (bucket_start(t, 3 * MONTH_SECONDS), bucket_start(t, 12 * MONTH_SECONDS)),
        (1_830_297_600, 1_830_297_600),
        "1 January 2028"
    );
    assert_eq!(bucket_end(1_832_976_000, MONTH_SECONDS), 1_835_481_600, "February 2028 has 29 days: 1 March");
    for _ in 0..20_000 {
        // from 1970 to 2100
        let t = d.below(4_102_444_800);
        for sec in PERIODS {
            let (start, end) = period(t, sec, &months);
            assert!(start <= t && t < end, "{t} in a period of {sec} s");
            assert_eq!(bucket_start(t, sec), start, "the period of {sec} s that holds {t}");
            assert_eq!(bucket_end(start, sec), end, "the end of the period of {sec} s that starts at {start}");
        }
    }
}

/// 🕯 Heikin Ashi candles keep the time, the volume and the shape of a candle: as many candles, each coherent, its
/// high not under the real high and its low not above the real low.
#[test]
fn heikin_ashi_candles_keep_the_shape() {
    let mut d = Draw(0x5EED_0105);
    for (step, bars) in markets(&mut d) {
        let ha = heikin_ashi(&bars);
        assert_eq!(ha.len(), bars.len(), "candles of {step} s");
        for (h, b) in ha.iter().zip(&bars) {
            assert!(coherent(h), "candles of {step} s: {h:?}");
            assert!(
                (h.time, h.volume) == (b.time, b.volume) && h.high >= b.high && h.low <= b.low,
                "candles of {step} s: {h:?} for {b:?}"
            );
        }
    }
}

/// ⏪ A replay never shows the future: at an hour, each candle it shows is made of the candles that came before that
/// hour and of nothing else; every candle finished by then is shown, then the candle in progress.
#[test]
fn a_replay_never_shows_the_future() {
    let months = months();
    let mut d = Draw(0x5EED_0106);
    let mut in_progress = 0;
    for (step, bars) in markets(&mut d) {
        for sec in PERIODS.into_iter().filter(|p| *p > step && se_regroupe(step, *p)) {
            let own = aggregate(&bars, sec);
            for _ in 0..60 {
                // the hour of the replay: the end of a candle that came
                let hour = bars[d.below(bars.len() as u64) as usize].time + step;
                let before: Vec<Bar> = bars.iter().filter(|b| b.time < hour).copied().collect();
                let past = folded(&before, sec, &months);
                let shown = au_replay(&own, sec, &bars, hour);
                for b in &shown {
                    assert!(past.contains(b), "candles of {step} s by {sec} s at {hour}: {b:?} is not made of the past only");
                }
                let finished: Vec<&Bar> = past.iter().filter(|b| period(b.time, sec, &months).1 <= hour).collect();
                assert!(
                    finished.iter().all(|b| shown.contains(b)),
                    "candles of {step} s by {sec} s at {hour}: a finished candle is missing"
                );
                // the candle in progress, when its first candle came: shown last, made of what came before the hour
                let start = period(hour, sec, &months).0;
                if start < hour && before.iter().any(|b| b.time == start) {
                    assert_eq!(shown.last(), past.last(), "candles of {step} s by {sec} s at {hour}: the candle in progress");
                    in_progress += 1;
                }
                assert!(
                    shown.windows(2).all(|w| w[0].time < w[1].time),
                    "candles of {step} s by {sec} s at {hour}: not in the order of time"
                );
            }
        }
    }
    assert!(in_progress > 500, "{in_progress} replays stopped inside a candle");
}

/// Made-up trades of one market over about ten minutes, in the order of time: prices are quarters around 100 and
/// sizes sixty-fourths, so sums are exact; `numbered`: each has a number that only grows, as some exchanges give.
fn trades(d: &mut Draw, n: usize, numbered: bool, first_id: u64) -> Vec<TradeFlux> {
    let mut ms = 1_704_067_200_000 + d.below(60_000);
    (0..n)
        .map(|k| {
            ms += d.below(1500);
            TradeFlux {
                time_ms: ms,
                prix: (380 + d.below(41)) as f64 / 4.0,
                taille: (1 + d.below(640)) as f64 / 64.0,
                achat: d.below(2) == 0,
                id: numbered.then_some(first_id + k as u64),
            }
        })
        .collect()
}

/// What trades are worth, the long way: for each minute, for each price row of half a unit, (purchases, sales).
fn counted(trades: &[TradeFlux]) -> BTreeMap<i64, BTreeMap<i64, (f64, f64)>> {
    let mut by: BTreeMap<i64, BTreeMap<i64, (f64, f64)>> = BTreeMap::new();
    for t in trades {
        // a row is half a unit: 99.75 is in the row 199, 100.00 in the row 200
        let row = (t.prix * 2.0).floor() as i64;
        let e = by.entry(t.time_ms.div_euclid(60_000) * 60).or_default().entry(row).or_default();
        if t.achat {
            e.0 += t.taille;
        } else {
            e.1 += t.taille;
        }
    }
    by
}

/// The minutes of an order flow that hold trades, as [`counted`] writes them.
fn rows(bars: &[FlowBar]) -> BTreeMap<i64, BTreeMap<i64, (f64, f64)>> {
    bars.iter()
        .filter(|b| !b.rangees.is_empty())
        .map(|b| (b.time, b.rangees.iter().map(|&(r, buys, sells)| (r, (buys, sells))).collect()))
        .collect()
}

/// Every minute of an order flow fed with `batches`, closed two minutes after the last trade.
fn flow(batches: &[&[TradeFlux]]) -> Vec<FlowBar> {
    let mut a = Agregateur::new(Some(0.5));
    for b in batches {
        a.ajouter(b);
    }
    let last = batches.iter().flat_map(|b| b.iter()).map(|t| t.time_ms).max().unwrap_or(0);
    a.tout_fermer(last + 120_000)
}

/// The same as the app does it: the trades arrive in lots, and after each lot the minutes that are closed (2 s after
/// their end) are taken out, then the rest two minutes after the last trade.
fn flow_as_it_comes(d: &mut Draw, came: &[TradeFlux]) -> Vec<FlowBar> {
    let mut a = Agregateur::new(Some(0.5));
    let (mut out, mut k) = (vec![], 0);
    while k < came.len() {
        let n = (1 + d.below(40) as usize).min(came.len() - k);
        a.ajouter(&came[k..k + n]);
        k += n;
        out.extend(a.fermees(came[k - 1].time_ms + d.below(1500)));
    }
    out.extend(a.tout_fermer(came[came.len() - 1].time_ms + 120_000));
    out
}

/// 🦶 The order flow counts every trade once, in its minute and in its price row, purchases and sales apart: its
/// delta is the purchases minus the sales that came. A lot given twice, or in another order, or with trades that are
/// not numbers, counts the same; after a reconnection, the last trades the exchange gives again count once.
#[test]
fn the_order_flow_counts_every_trade_once() {
    let mut d = Draw(0x5EED_0107);
    for round in 0..150 {
        for numbered in [true, false] {
            let came = trades(&mut d, 400, numbered, 1000);
            let want = counted(&came);
            let bars = flow(&[&came]);
            assert_eq!(rows(&bars), want, "round {round}, numbered {numbered}");
            assert!(
                bars.iter().all(|b| b.time % 60 == 0 && b.pas == 0.5),
                "round {round}: a bar a minute, rows of half a unit"
            );
            let (buys, sells) = came
                .iter()
                .fold((0.0, 0.0), |(b, s), t| if t.achat { (b + t.taille, s) } else { (b, s + t.taille) });
            assert_eq!(
                bars.iter().map(FlowBar::delta).sum::<f64>(),
                buys - sells,
                "round {round}: the delta is the purchases minus the sales"
            );
            // in several lots, as the trades arrive: each minute is given once, when it is closed
            let (a, b) = came.split_at(d.below(came.len() as u64) as usize);
            assert_eq!(rows(&flow(&[a, b])), want, "round {round}, numbered {numbered}: in two lots");
            let live = flow_as_it_comes(&mut d, &came);
            assert!(
                live.windows(2).all(|w| w[0].time < w[1].time),
                "round {round}, numbered {numbered}: a minute is given twice"
            );
            assert_eq!(rows(&live), want, "round {round}, numbered {numbered}: as it comes");
            // trades that are not numbers, or of no size, or at no price: not counted
            let mut mad = came.clone();
            for (k, v) in [f64::NAN, f64::INFINITY, 0.0, -1.0, f64::NEG_INFINITY].into_iter().enumerate() {
                let mut t = came[k * 7];
                if k % 2 == 0 {
                    t.prix = v;
                } else {
                    t.taille = v;
                }
                t.id = numbered.then_some(5000 + k as u64);
                mad.push(t);
            }
            assert_eq!(
                rows(&flow(&[&mad])),
                want,
                "round {round}, numbered {numbered}: with trades that are not numbers"
            );
            if numbered {
                // the same lot once more, and a lot from the most recent to the oldest: counted once, the same
                assert_eq!(rows(&flow(&[&came, &came])), want, "round {round}: the same lot twice");
                let reversed: Vec<TradeFlux> = came.iter().rev().copied().collect();
                assert_eq!(rows(&flow(&[&reversed])), want, "round {round}: from the most recent to the oldest");
            } else {
                // a reconnection: the exchange gives its last trades again, then new ones
                let (old, new) = came.split_at(250);
                let mut a = Agregateur::new(Some(0.5));
                a.ajouter(old);
                a.coupure(old[old.len() - 1].time_ms);
                let again: Vec<TradeFlux> = old[200..]
                    .iter()
                    .chain(new.iter().filter(|t| t.time_ms > old[old.len() - 1].time_ms))
                    .copied()
                    .collect();
                a.ajouter(&again);
                let kept: Vec<TradeFlux> = old
                    .iter()
                    .chain(new.iter().filter(|t| t.time_ms > old[old.len() - 1].time_ms))
                    .copied()
                    .collect();
                assert_eq!(
                    rows(&a.tout_fermer(came[came.len() - 1].time_ms + 120_000)),
                    counted(&kept),
                    "round {round}: after a reconnection"
                );
            }
        }
    }
}

/// 🦶 A regrouped order flow keeps every trade: the rows of a longer bar are the sums of the rows of its minutes, at
/// the start of its period. A minute is complete unless the connection was cut during it or it is still open; a
/// longer bar is complete only when each of its minutes is there and complete.
#[test]
fn a_regrouped_order_flow_keeps_every_trade() {
    let mut d = Draw(0x5EED_0108);
    let (mut whole, mut partial) = (0, 0);
    for round in 0..40 {
        // two hours and ten minutes of trades from an hour on the hour, a trade every 2 s or so: the first hour is
        // whole; in the second, two minutes where nothing came and a connection that is cut
        let start = 1_704_067_200_000;
        let nothing = [65 + d.below(25), 95 + d.below(25)];
        let mut ms = start + d.below(1000);
        let mut came = vec![];
        while ms < start + 130 * 60_000 {
            if !nothing.contains(&((ms - start) / 60_000)) {
                came.push(TradeFlux {
                    time_ms: ms,
                    prix: (380 + d.below(41)) as f64 / 4.0,
                    taille: (1 + d.below(640)) as f64 / 64.0,
                    achat: d.below(2) == 0,
                    id: Some(came.len() as u64 + 1),
                });
            }
            ms += 1 + d.below(4000);
        }
        let cut = start + (62 + d.below(60)) * 60_000 + d.below(60_000);
        let (before, after) = came.split_at(came.partition_point(|t| t.time_ms < cut));
        let mut a = Agregateur::new(Some(0.5));
        a.ajouter(before);
        a.coupure(cut);
        a.ajouter(after);
        let end = came[came.len() - 1].time_ms + 120_000;
        let minutes = a.tout_fermer(end);
        assert_eq!(rows(&minutes), counted(&came), "round {round}: every trade is in its minute");
        // by hand: the minutes that are not complete are the minute of the cut and the minute still open at the end
        let broken = [cut.div_euclid(60_000) * 60, end.div_euclid(60_000) * 60];
        let seen: Vec<i64> = minutes.iter().filter(|m| !m.complete).map(|m| m.time).collect();
        assert_eq!(seen, broken, "round {round}: the minutes that are not complete");
        let there: Vec<i64> = minutes.iter().map(|m| m.time).collect();
        for tf in [5 * MINUTE, 15 * MINUTE, HOUR] {
            let got = regrouper(&minutes, tf);
            let mut want: BTreeMap<i64, BTreeMap<i64, (f64, f64)>> = BTreeMap::new();
            for (minute, r) in counted(&came) {
                let period = want.entry(minute - minute.rem_euclid(tf)).or_default();
                for (row, (buys, sells)) in r {
                    let e = period.entry(row).or_default();
                    (e.0, e.1) = (e.0 + buys, e.1 + sells);
                }
            }
            assert_eq!(rows(&got), want, "round {round}: minutes regrouped by {tf} s");
            assert!(
                got.windows(2).all(|w| w[0].time < w[1].time),
                "round {round}: bars of {tf} s not in the order of time"
            );
            for b in &got {
                assert!(b.time % tf == 0 && b.pas == 0.5, "round {round}: a bar of {tf} s at {}", b.time);
                // by hand: each minute of the bar is there, and none of them is broken
                let all = (0..tf / MINUTE)
                    .map(|k| b.time + k * MINUTE)
                    .all(|m| there.contains(&m) && !broken.contains(&m));
                assert_eq!(b.complete, all, "round {round}: the bar of {tf} s at {}", b.time);
                if all {
                    whole += 1;
                } else {
                    partial += 1;
                }
            }
        }
    }
    assert!(whole > 400 && partial > 200, "{whole} whole bars and {partial} partial ones: both must be seen");
}
