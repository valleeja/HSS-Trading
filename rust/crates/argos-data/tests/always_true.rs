//! 🔢 Data reliability, test 2, the live side: rules that are always true, whatever the streams send.
//!
//! The same method as `argos-core/tests/always_true.rs` (made-up data from a fixed draw, the same on every computer
//! and at every run; each rule said once, the long way, never worked out with the function it checks), on what the
//! data goes through after it is read and before the screen: a candle of the live stream applied to the candles of
//! the chart, trades folded into candles (the exchanges without a candle stream, and the charts in seconds), the
//! candles found again for a hole, and the order books of several exchanges summed into one.
//!
//! - **The live never changes the past**: a candle of the stream moves the candle in progress or starts a new one;
//!   an older one, or one that is not coherent, changes nothing.
//! - **A candle made from trades is the fold of its trades**: the price of the first, the highest, the lowest, the
//!   price of the last, the sum of the sizes.
//! - **A hole is filled and nothing else moves.**
//! - **After a cut the candles missed come back, each once** (test 5): when its stream is back the chart is the
//!   exchange's own series again, with no hole, no candle twice and none left as it was at the cut.
//! - **A summed book keeps every size**, its prices in order, and is not crossed when no book that came is.
//! - **What cannot be is left out where it is read**: a trade, a level of an order book or a liquidation whose price
//!   or size is not a number above zero never comes out of a reader, whatever the exchange; what came with it stays.
//!
//! Nothing here is real or anyone's.

use argos_core::murs::VenueBook;
use argos_core::{Bar, Timeframe};
use argos_data::cache::{merge, missing};
use argos_data::live::{apply, fold_trade, parse_trades, Trade};
use argos_data::murs::{parse_maj, somme, Carnet};
use argos_data::trous::combler;
use argos_data::Exchange;
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
    fn below(&mut self, n: i64) -> i64 {
        (self.next() % n as u64) as i64
    }

    /// The same things in another order.
    fn shuffle<T>(&mut self, v: &mut [T]) {
        for k in (1..v.len()).rev() {
            v.swap(k, self.below(k as i64 + 1) as usize);
        }
    }
}

/// Monday 1 January 2024, 00:00 UTC.
const START: i64 = 1_704_067_200;

/// The rule of a coherent candle, said by hand: it has a time, its prices are numbers above zero, its high is not
/// under its open or its close, its low is not above them, its volume is a number and is not negative.
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

/// A made-up coherent candle at `time`, near the price `last` (counted in quarters; the volume is a whole number).
fn candle(d: &mut Draw, time: i64, last: &mut i64) -> Bar {
    let open = (*last + d.below(9) - 4).max(1);
    let close = (open + d.below(41) - 20).max(1);
    let (high, low) = (open.max(close) + d.below(10), (open.min(close) - d.below(10)).max(1));
    *last = close;
    Bar {
        time,
        open: open as f64 / 4.0,
        high: high as f64 / 4.0,
        low: low as f64 / 4.0,
        close: close as f64 / 4.0,
        volume: d.below(5001) as f64,
    }
}

/// Are two candles the same, number for number (bit for bit: a candle that took a « not a number » is equal to
/// nothing, itself included, with `==`)?
fn same(a: &Bar, b: &Bar) -> bool {
    let bits = |b: &Bar| (b.time, [b.open, b.high, b.low, b.close, b.volume].map(f64::to_bits));
    bits(a) == bits(b)
}

/// The same candle with one thing that cannot be.
fn mad(d: &mut Draw, mut b: Bar) -> Bar {
    let no = [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 0.0, -1.0][d.below(5) as usize];
    match d.below(8) {
        0 => b.open = no,
        1 => b.high = no,
        2 => b.low = no,
        3 => b.close = no,
        4 => b.volume = [f64::NAN, f64::INFINITY, -1.0][d.below(3) as usize],
        5 => b.high = b.open.max(b.close) - 0.25,
        6 => b.low = b.open.min(b.close) + 0.25,
        _ => b.time = [0, -60][d.below(2) as usize],
    }
    assert!(!coherent(&b), "{b:?} was meant not to be coherent");
    b
}

/// ⚡ The live never changes the past. A candle of the stream that is coherent and not older than the last candle
/// becomes the last candle (it moves the candle in progress, or starts a new one) and nothing before it moves; any
/// other (older, or not coherent: a price that is not a number, zero or negative, a high under the close…) changes
/// nothing at all.
#[test]
fn the_live_never_changes_the_past() {
    let mut d = Draw(0x5EED_0201);
    let (mut taken, mut left_out) = (0, 0);
    for round in 0..300 {
        let step = [60, 300, 3600, 86_400][d.below(4) as usize];
        let mut last = 4000 + d.below(4000);
        let mut bars: Vec<Bar> = (0..50).map(|k| candle(&mut d, START + k * step, &mut last)).collect();
        for _ in 0..300 {
            let newest = bars[bars.len() - 1].time;
            let time = match d.below(20) {
                // the candle in progress moves
                0..=11 => newest,
                // a new candle, sometimes after a hole
                12..=14 => newest + step * (1 + d.below(3)),
                // an old candle, sent again
                _ => newest - step * (1 + d.below(40)),
            };
            let mut came = candle(&mut d, time, &mut last);
            if d.below(6) == 0 {
                came = mad(&mut d, came);
            }
            let before = bars.clone();
            let began = apply(&mut bars, came);
            if coherent(&came) && came.time >= newest {
                let kept = before.len() - usize::from(came.time == newest);
                assert_eq!(bars[bars.len() - 1], came, "round {round}: the candle that came is the last one");
                assert_eq!(bars[..bars.len() - 1], before[..kept], "round {round}: a candle before {came:?} moved");
                assert_eq!(began, came.time > newest, "round {round}: {came:?} after {newest}");
                taken += 1;
            } else {
                assert_eq!(bars, before, "round {round}: {came:?} is old or not coherent, yet something changed");
                assert!(!began, "round {round}: {came:?} is old or not coherent, yet a candle began");
                left_out += 1;
            }
        }
        assert!(bars.iter().all(coherent) && bars.windows(2).all(|w| w[0].time < w[1].time), "round {round}");
    }
    assert!(
        taken > 20_000 && left_out > 20_000,
        "{taken} candles taken and {left_out} left out: both must be seen"
    );
}

/// ⚡ A candle made from trades is the fold of its trades, for every period a chart fills from trades (a second to a
/// day): it starts at the start of its period, opens at the price of its first trade, closes at the price of its
/// last, its high and low are the highest and lowest price, its volume the sum of the sizes. The trades first complete
/// the candle in progress; a trade older than the candle in progress is left out, and so is a trade that cannot be
/// (a price or a size that is not a number above zero): it changes nothing at all, whenever it comes.
#[test]
fn a_candle_made_from_trades_is_the_fold_of_its_trades() {
    let mut d = Draw(0x5EED_0202);
    let (mut late, mut begun, mut left_out) = (0, 0, 0);
    for round in 0..160 {
        let secs = [1, 5, 15, 60, 300, 3600, 14_400, 86_400][round % 8];
        // the candle in progress when the trades start
        let t0 = START + d.below(1000) * secs;
        let first = Bar {
            time: t0,
            open: 100.0,
            high: 101.5,
            low: 99.25,
            close: 100.5,
            volume: 7.0,
        };
        let mut bars = vec![first];
        let mut counted: BTreeMap<i64, Vec<Trade>> = BTreeMap::from([(t0, vec![])]);
        let (mut ms, mut newest) = (t0 * 1000 + d.below(secs * 300), t0);
        for _ in 0..600 {
            ms += d.below(secs * 400 + 1);
            // now and then a trade arrives late: one to three periods back
            let time_ms = if d.below(12) == 0 { ms - secs * 1000 * (1 + d.below(3)) } else { ms };
            let mut t = Trade {
                time_ms,
                price: (380 + d.below(41)) as f64 / 4.0,
                size: (1 + d.below(640)) as f64 / 64.0,
            };
            // now and then a trade that cannot be, at any time (late, in the candle in progress, or after it)
            if d.below(10) == 0 {
                let no = [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 0.0, -0.0, -1.0][d.below(6) as usize];
                if d.below(2) == 0 {
                    t.price = no;
                } else {
                    t.size = no;
                }
                let before = bars.clone();
                assert!(!fold_trade(&mut bars, t, secs), "round {round}, {secs} s: {t:?} cannot be, yet a candle began");
                assert!(
                    bars.len() == before.len() && bars.iter().zip(&before).all(|(a, b)| same(a, b)),
                    "round {round}, {secs} s: {t:?} cannot be, yet the candles went from {before:?} to {bars:?}"
                );
                left_out += 1;
                continue;
            }
            // by hand: the period that holds the second of the trade
            let second = time_ms.div_euclid(1000);
            let start = second - second.rem_euclid(secs);
            if start >= newest {
                counted.entry(start).or_default().push(t);
            } else {
                late += 1;
            }
            let n = bars.len();
            let began = fold_trade(&mut bars, t, secs);
            assert_eq!(
                (began, bars.len()),
                (start > newest, n + usize::from(start > newest)),
                "round {round}, {secs} s: {t:?}"
            );
            begun += usize::from(began);
            newest = newest.max(start);
        }
        let folded: Vec<Bar> = counted
            .iter()
            .map(|(&time, trades)| {
                let open = trades.first().map_or(0.0, |t| t.price);
                let mut c = if time == t0 {
                    first
                } else {
                    Bar {
                        time,
                        open,
                        high: open,
                        low: open,
                        close: open,
                        volume: 0.0,
                    }
                };
                for t in trades {
                    (c.high, c.low, c.close, c.volume) = (c.high.max(t.price), c.low.min(t.price), t.price, c.volume + t.size);
                }
                c
            })
            .collect();
        assert_eq!(bars, folded, "round {round}, candles of {secs} s");
        assert!(
            bars.iter().all(coherent) && bars.windows(2).all(|w| w[0].time < w[1].time),
            "round {round}, {secs} s"
        );
    }
    assert!(
        late > 1000 && begun > 10_000 && left_out > 5000,
        "{late} late trades, {begun} candles begun and {left_out} trades that cannot be: each must be seen"
    );
}

/// 🩹 A hole is filled and nothing else moves: after a repair the chart has its candles and the coherent candles
/// found again, in the order of time, one per time (for a time it already had, the exchange's latest word); a candle
/// found again that is not coherent changes nothing.
#[test]
fn a_hole_is_filled_and_nothing_else_moves() {
    let mut d = Draw(0x5EED_0203);
    for round in 0..200 {
        let mut last = 4000 + d.below(4000);
        let whole: Vec<Bar> = (0..300).map(|k| candle(&mut d, START + k * 3600, &mut last)).collect();
        let (mut have, mut found) = (vec![], vec![]);
        for b in &whole {
            match d.below(10) {
                0..=5 => have.push(*b),
                // a hole, found again
                6 => found.push(*b),
                // a hole that stays a hole
                7 => {}
                // a candle the chart has, found again with another volume
                8 => {
                    have.push(*b);
                    found.push(Bar { volume: b.volume + 1.0, ..*b });
                }
                // a candle the chart has, found again mad
                _ => {
                    have.push(*b);
                    found.push(mad(&mut d, *b));
                }
            }
        }
        d.shuffle(&mut found);
        let mut want: BTreeMap<i64, Bar> = have.iter().map(|b| (b.time, *b)).collect();
        want.extend(found.iter().filter(|b| coherent(b)).map(|b| (b.time, *b)));
        let got = combler(&have, &found);
        assert_eq!(got, want.into_values().collect::<Vec<_>>(), "round {round}");
        assert!(got.len() > have.len() && got.len() < whole.len(), "round {round}: holes filled, holes left");
    }
}

/// Where two series of candles stop being the same, said in a line (`None`: they are the same).
fn differ(got: &[Bar], want: &[Bar]) -> Option<String> {
    let k = got.iter().zip(want).position(|(g, w)| g != w).unwrap_or(got.len().min(want.len()));
    (got != want).then(|| {
        format!(
            "{} candles for {} wanted; the first that differs is the candle {k}: {:?} for {:?}",
            got.len(),
            want.len(),
            got.get(k),
            want.get(k)
        )
    })
}

/// 🔌 After a cut the candles missed come back, each once (test 5: after a reconnection, no hole and nothing twice).
/// There are two ways to miss candles. The stream of the chart is cut and comes back: the candle in progress at the
/// cut moved on, and the chart then asks for as many candles as it keeps. Or the app was closed and is opened again:
/// the chart starts from the candles it saved (closed ones only) and asks for a first screen at most. Both times the
/// exchange went on meanwhile: candles closed, and it may have corrected the last candles the chart had. The chart
/// asks for what it missed ([`missing`]: every candle since its last one, and its last ones again), lets the stream
/// move its newest candle while the answer is on its way, joins the answer to what it had ([`merge`]) and gives the
/// candles of the stream their place again ([`apply`]).
///
/// After that the chart is the exchange's own series as it is now, from the chart's first candle to the newest (the
/// newest `keeps` of them): no hole, no candle twice, and none left as it was. When too many candles were missed for
/// the answer to reach back to the chart's, the chart starts again from the answer: it never joins candles over a
/// hole.
///
/// The one exception, seen here and written down: when the app stayed closed for about as many periods as it asks for
/// at once, the answer just meets the candles saved and does not give the last of them again; if the exchange
/// corrected one of those since, it stays as it was saved.
#[test]
fn after_a_cut_the_candles_missed_come_back_once() {
    // the last candles of the chart that are asked again with the ones missed: the exchange may have corrected them
    const AGAIN: usize = 3;
    let mut d = Draw(0x5EED_0206);
    let (mut blips, mut short, mut long, mut over_a_hole, mut begun, mut corrected, mut left_as_saved) = (0, 0, 0, 0, 0, 0, 0);
    for round in 0..4000 {
        let tf = [Timeframe::M1, Timeframe::M5, Timeframe::H1, Timeframe::D1][d.below(4) as usize];
        let step = tf.seconds();
        // the most candles the chart keeps (5 000 in the app), and the most it asks for at once: as many when its
        // stream comes back, a first screen (500 in the app) when the app is opened again
        let reopened = d.below(3) == 0;
        let keeps = 6 + d.below(60) as usize;
        let screen = if reopened { 4 + d.below(keeps as i64 - 3) as usize } else { keeps };
        // the exchange has `older` candles from before the chart's first one; the chart has `had` candles
        let (older, had) = (4 + d.below(10) as usize, 1 + d.below(80) as usize);
        // how many periods are missed: none (a blip of the stream), a few, many, or about as many as the chart asks for
        let missed = match d.below(10) {
            0..=2 => 0,
            3..=6 => 1 + d.below(6) as usize,
            7 => d.below(3 * screen as i64) as usize,
            _ => (screen + d.below(8) as usize).saturating_sub(AGAIN + 4),
        }
        .max(usize::from(reopened));
        let time = |k: usize| START + k as i64 * step;
        let (then, newest) = (older + had - 1, older + had - 1 + missed);
        // the exchange's candles once they are closed, up to the one that begins after the return
        let mut last = 4000 + d.below(4000);
        let closed: Vec<Bar> = (0..=newest + 1).map(|k| candle(&mut d, time(k), &mut last)).collect();
        // another state of the candle `k`: in progress, or as the stream last said it before the exchange corrected it
        let state = |d: &mut Draw, k: usize| {
            let mut near = (closed[k].close * 4.0) as i64;
            candle(d, time(k), &mut near)
        };
        // what the chart has: the exchange's candles; some of the last ones as the stream last said them; and, when
        // the stream was cut, the newest one in progress
        let mut chart: Vec<Bar> = closed[older..=then].to_vec();
        for back in 0..=AGAIN.min(had - 1) {
            let in_progress = back == 0 && !reopened;
            if in_progress || d.below(3) == 0 {
                chart[had - 1 - back] = state(&mut d, then - back);
                corrected += usize::from(!in_progress);
            }
        }
        let had_then = chart.clone();
        // the exchange's series when it answers: every candle closed since, and the newest one in progress
        let mut theirs: Vec<Bar> = closed[..newest].to_vec();
        theirs.push(state(&mut d, newest));
        let asked = missing(&chart, time(newest) + d.below(step), tf, keeps).min(screen);
        assert_eq!(
            asked,
            (missed + 1 + AGAIN).min(screen),
            "round {round}: the {missed} candles since the chart's last one, that one and the {AGAIN} before it"
        );
        let answer = theirs[theirs.len() - asked..].to_vec();
        // the stream is there before the answer: it moves the newest candle, and sometimes the next one begins
        let mut stream: Vec<Bar> = (0..d.below(3)).map(|_| state(&mut d, newest)).collect();
        if d.below(4) == 0 {
            stream.extend([state(&mut d, newest), closed[newest + 1]]);
            begun += 1;
        }
        let mut waiting = vec![];
        for b in &stream {
            apply(&mut waiting, *b);
            apply(&mut chart, *b);
        }
        let mut got = merge(&chart, answer.clone(), tf, keeps);
        for b in waiting {
            apply(&mut got, b);
        }
        // said by hand, from the exchange's series. The answer reaches back to the chart when no candle of the
        // exchange is missing between the two: then the chart goes on from its first candle (or from the first one
        // of the answer, when that one is older); if not, it starts again from the answer
        let reached = answer[0].time <= time(then) + step;
        let from = if reached { had_then[0].time.min(answer[0].time) } else { answer[0].time };
        let mut want: BTreeMap<i64, Bar> = theirs.iter().filter(|b| b.time >= from).map(|b| (b.time, *b)).collect();
        // (a candle the chart had and that the answer does not give again stays as the chart had it)
        want.extend(had_then.iter().filter(|b| (from..answer[0].time).contains(&b.time)).map(|b| (b.time, *b)));
        while want.len() > keeps {
            want.pop_first();
        }
        // then the words of the stream: the last word for a candle is the one that stays, a candle that began is one more
        want.extend(stream.iter().map(|b| (b.time, *b)));
        assert_eq!(
            differ(&got, &want.into_values().collect::<Vec<_>>()),
            None,
            "round {round}: {had} candles, {missed} periods missed, at most {screen} asked and {keeps} kept"
        );
        // which is the rule: no hole and no candle twice…
        assert!(
            got.windows(2).all(|w| w[1].time - w[0].time == step),
            "round {round}: a hole, or a candle twice"
        );
        // … and none left as it was: every candle from before the answer is the exchange's closed one (the exception:
        // the answer was cut short and just meets the candles the chart saved)
        let left = got
            .iter()
            .filter(|b| b.time < answer[0].time && **b != closed[((b.time - START) / step) as usize])
            .count();
        if left > 0 {
            assert!(
                reopened && asked < missed + 1 + AGAIN,
                "round {round}: {left} candle(s) left as they were, though the answer gave the chart's last candles again"
            );
            left_as_saved += 1;
        }
        match (missed, reached) {
            (0, _) => blips += 1,
            (_, false) => over_a_hole += 1,
            _ if asked < screen => short += 1,
            _ => long += 1,
        }
    }
    assert!(
        blips > 300 && short > 300 && long > 300 && over_a_hole > 100 && begun > 300 && corrected > 300,
        "{blips} blips, {short} short absences, {long} long ones, {over_a_hole} too long for the answer to reach the chart, {begun} candles begun and {corrected} candles corrected: each must be seen"
    );
    assert!(
        left_as_saved > 0,
        "no candle is left as it was saved any more: take the exception out of this rule, it holds for every absence now"
    );
}

/// A made-up book of one exchange around the price `mid` (counted in halves): bids under it, asks above it, in no
/// order; sizes are sixty-fourths, so sums are exact.
fn venue(d: &mut Draw, name: &str, mid: i64) -> VenueBook {
    let mut side = |sign: i64| -> Vec<(f64, f64)> {
        let mut away: Vec<i64> = (1..=400).collect();
        d.shuffle(&mut away);
        away.truncate(1 + d.below(60) as usize);
        away.into_iter()
            .map(|k| ((mid + sign * k) as f64 / 2.0, (1 + d.below(6400)) as f64 / 64.0))
            .collect()
    };
    VenueBook {
        venue: name.into(),
        bids: side(-1),
        asks: side(1),
    }
}

/// 📖 A summed book keeps every size. The books of one to three exchanges summed by price steps: every size that
/// came is in its step (a bid in the step at or under its price, an ask in the step at or above), the bids go from
/// the best (highest) down and the asks from the best (lowest) up, one level per step, and the book is not crossed:
/// the best bid is under the best ask, as in every book that came.
#[test]
fn a_summed_book_keeps_every_size() {
    let mut d = Draw(0x5EED_0204);
    for round in 0..300 {
        let mid = 190_000 + d.below(20_000);
        let books: Vec<VenueBook> = ["A", "B", "C"][..1 + round % 3].iter().map(|name| venue(&mut d, name, mid)).collect();
        // the step, counted in halves: 0.5, 1, 2.5, 10, 50
        for step in [1, 2, 5, 20, 100] {
            let book = somme("BTC", &books, step as f64 / 2.0, 1);
            // by hand, in whole numbers of halves
            let (mut bids, mut asks): (BTreeMap<i64, f64>, BTreeMap<i64, f64>) = (BTreeMap::new(), BTreeMap::new());
            for b in &books {
                for &(px, sz) in &b.bids {
                    let halves = (px * 2.0) as i64;
                    *bids.entry(halves - halves.rem_euclid(step)).or_default() += sz;
                }
                for &(px, sz) in &b.asks {
                    let halves = (px * 2.0) as i64;
                    *asks.entry(halves + (step - halves.rem_euclid(step)) % step).or_default() += sz;
                }
            }
            let levels = |l: &[argos_data::book::Level]| -> Vec<(f64, f64)> { l.iter().map(|l| (l.px, l.sz)).collect() };
            let want_bids: Vec<(f64, f64)> = bids.iter().rev().map(|(k, sz)| (*k as f64 / 2.0, *sz)).collect();
            let want_asks: Vec<(f64, f64)> = asks.iter().map(|(k, sz)| (*k as f64 / 2.0, *sz)).collect();
            assert_eq!(levels(&book.bids), want_bids, "round {round}, step {step}: the bids");
            assert_eq!(levels(&book.asks), want_asks, "round {round}, step {step}: the asks");
            let came = |side: fn(&VenueBook) -> &Vec<(f64, f64)>| -> f64 { books.iter().flat_map(side).map(|l| l.1).sum() };
            let kept = |l: &[argos_data::book::Level]| -> f64 { l.iter().map(|l| l.sz).sum() };
            assert_eq!(
                (kept(&book.bids), kept(&book.asks)),
                (came(|b| &b.bids), came(|b| &b.asks)),
                "round {round}, step {step}: the sizes"
            );
            let (bid, ask) = (book.bids[0].px, book.asks[0].px);
            assert!(
                bid < mid as f64 / 2.0 && mid as f64 / 2.0 < ask,
                "round {round}, step {step}: the book is crossed ({bid} / {ask})"
            );
            assert_eq!(book.spread().map(|s| s.0), Some(ask - bid), "round {round}, step {step}: the spread");
        }
    }
}

/// A price or a size that cannot be, as a message could write it in a text: not a number, infinite (« 1e999 » is
/// read as infinite), zero, negative.
const MAD: [&str; 10] = ["NaN", "nan", "inf", "-inf", "Infinity", "1e999", "0", "0.000", "-1", "-0.5"];

/// The same where a message writes a JSON number: it can only be zero or negative.
const MAD_NUMBERS: [&str; 5] = ["0", "0.0", "-0.0", "-1", "-0.5"];

/// Is every digit of `s` a zero?
fn zero(s: &str) -> bool {
    !s.chars().any(|c| c.is_ascii_digit() && c != '0')
}

/// What a reader of the app reads, and for what.
enum Read {
    /// `live::parse_trades`: the trades a chart folds into candles.
    Candles(Exchange),
    /// `whales::parse_all`: the trades of the Trades module, of the whales and of the order flow.
    Trades,
    /// `book::parse_book`: the live book of the Book module (here, its purchases).
    LiveBook,
    /// `Carnet::parse`: the book of one exchange, for Argos Murs and the summed book (here, its purchases).
    Book(Carnet),
    /// `liq::parse` and `liq::parse_bybit`: the liquidations.
    Liquidations,
    LiquidationsBybit,
}

/// One message format of one exchange, and the reader it goes to.
struct Reader {
    name: &'static str,
    read: Read,
    /// The message. Between « and », one item (a trade, a level, a liquidation) with its price PX and its size SZ:
    /// the items of one message are written there one after the other. A message that is one item whole (nothing
    /// around « and ») carries one item: several items are several messages.
    message: &'static str,
    /// Coins per unit of size where the exchange counts in contracts (BTC, read on the exchanges in Nov 2026); 1
    /// elsewhere.
    coins: f64,
    /// The sign of the size says the side (a sale is negative): a negative size can be.
    signed: bool,
}

impl Reader {
    /// The messages that carry these items, (price, size) as the exchange writes them.
    fn messages(&self, items: &[(&str, &str)]) -> Vec<String> {
        let (head, rest) = self.message.split_once('«').expect("an item between « and »");
        let (item, tail) = rest.split_once('»').expect("an item between « and »");
        let items: Vec<String> = items.iter().map(|(px, sz)| item.replace("PX", px).replace("SZ", sz)).collect();
        if head.is_empty() && tail.is_empty() {
            items
        } else {
            vec![format!("{head}{}{tail}", items.join(","))]
        }
    }

    /// What the reader gives for these items: the (price, size) of each, in the order they came.
    fn read(&self, items: &[(&str, &str)]) -> Vec<(f64, f64)> {
        let one = |m: &String| -> Vec<(f64, f64)> {
            match &self.read {
                Read::Candles(ex) => parse_trades(*ex, m).iter().map(|t| (t.price, t.size)).collect(),
                Read::Trades => argos_data::whales::parse_all(m).iter().map(|w| (w.price, w.size)).collect(),
                Read::LiveBook => argos_data::book::parse_book(m).map_or(vec![], |b| b.bids.iter().map(|l| (l.px, l.sz)).collect()),
                Read::Book(c) => c.parse(m, 1.0).map_or(vec![], |b| b.bids),
                Read::Liquidations => argos_data::liq::parse(m).iter().map(|l| (l.price, l.qty)).collect(),
                Read::LiquidationsBybit => argos_data::liq::parse_bybit(m).iter().map(|l| (l.price, l.qty)).collect(),
            }
        };
        self.messages(items).iter().flat_map(one).collect()
    }
}

/// Every reader of trades, of order books and of liquidations, with a message of each exchange it reads (the
/// formats are those of the exchanges' public streams and answers, as the app's own readers say them).
fn readers() -> Vec<Reader> {
    let r = |name, read, message, coins, signed| Reader {
        name,
        read,
        message,
        coins,
        signed,
    };
    const HYPERLIQUID_TRADES: &str =
        r#"{"channel":"trades","data":[«{"coin":"BTC","side":"B","px":"PX","sz":"SZ","time":1704067207000,"tid":1,"users":["0xa","0xb"]}»]}"#;
    const BITSTAMP: &str = r#"«{"channel":"live_trades_btcusd","event":"trade","data":{"id":1,"amount":SZ,"price":PX,"type":0,"microtimestamp":"1704067207000000","buy_order_id":2,"sell_order_id":1}}»"#;
    const COINBASE: &str =
        r#"«{"type":"match","trade_id":1,"side":"sell","size":"SZ","price":"PX","product_id":"BTC-USD","sequence":1,"time":"2024-01-01T00:00:07.000000Z"}»"#;
    const KUCOIN: &str = r#"«{"type":"message","topic":"/market/match:BTC-USDT","subject":"trade.l3match","data":{"price":"PX","size":"SZ","side":"buy","symbol":"BTC-USDT","time":"1704067207000000000","tradeId":"1"}}»"#;
    const BITFINEX: &str = r#"«[17,"te",[1,1704067207000,SZ,PX]]»"#;
    const BINANCE_BOOK: &str = r#"{"lastUpdateId":1,"bids":[«["PX","SZ"]»],"asks":[["200000","1"]]}"#;
    const BYBIT_BOOK: &str =
        r#"{"retCode":0,"retMsg":"OK","result":{"s":"BTCUSDT","b":[«["PX","SZ"]»],"a":[["200000","1"]],"ts":1,"u":1},"retExtInfo":{},"time":1}"#;
    const OKX_BOOK: &str = r#"{"code":"0","msg":"","data":[{"bids":[«["PX","SZ","0","1"]»],"asks":[["200000","1","0","1"]],"ts":"1"}]}"#;
    const HYPERLIQUID_BOOK: &str = r#"{"coin":"BTC","time":1,"levels":[[«{"px":"PX","sz":"SZ","n":1}»],[{"px":"200000","sz":"1","n":1}]]}"#;
    vec![
        // the trades a chart folds into candles
        r("candles, Hyperliquid", Read::Candles(Exchange::Hyperliquid), HYPERLIQUID_TRADES, 1.0, false),
        r("candles, Bitstamp", Read::Candles(Exchange::Bitstamp), BITSTAMP, 1.0, false),
        r("candles, Coinbase", Read::Candles(Exchange::Coinbase), COINBASE, 1.0, false),
        r("candles, KuCoin", Read::Candles(Exchange::Kucoin), KUCOIN, 1.0, false),
        r("candles, Bitfinex", Read::Candles(Exchange::Bitfinex), BITFINEX, 1.0, true),
        r("candles, Bitfinex spot", Read::Candles(Exchange::BitfinexSpot), BITFINEX, 1.0, true),
        // the trades of the Trades module, of the whales and of the order flow
        r("trades, Hyperliquid", Read::Trades, HYPERLIQUID_TRADES, 1.0, false),
        r(
            "trades, WEEX",
            Read::Trades,
            r#"{"e":"trade","E":1704067207000,"s":"BTCUSDT","d":[«{"T":1704067207000,"t":"a","p":"PX","q":"SZ","v":"1","m":false}»]}"#,
            1.0,
            false,
        ),
        r(
            "trades, Binance and Aster",
            Read::Trades,
            r#"«{"e":"aggTrade","E":1704067207000,"s":"BTCUSDT","a":1,"p":"PX","q":"SZ","f":1,"l":1,"T":1704067207000,"m":false}»"#,
            1.0,
            false,
        ),
        r(
            "trades, BloFin",
            Read::Trades,
            r#"{"arg":{"channel":"trades","instId":"BTC-USDT"},"data":[«{"instId":"BTC-USDT","tradeId":"1","price":"PX","size":"SZ","side":"buy","ts":"1704067207000"}»]}"#,
            0.001,
            false,
        ),
        r(
            "trades, OKX",
            Read::Trades,
            r#"{"arg":{"channel":"trades","instId":"BTC-USDT"},"data":[«{"instId":"BTC-USDT","tradeId":"1","px":"PX","sz":"SZ","side":"buy","ts":"1704067207000"}»]}"#,
            1.0,
            false,
        ),
        r(
            "trades, Bybit",
            Read::Trades,
            r#"{"topic":"publicTrade.BTCUSDT","type":"snapshot","ts":1704067207000,"data":[«{"T":1704067207000,"s":"BTCUSDT","S":"Buy","v":"SZ","p":"PX","i":"1"}»]}"#,
            1.0,
            false,
        ),
        r(
            "trades, Bitget",
            Read::Trades,
            r#"{"action":"update","arg":{"instType":"USDT-FUTURES","channel":"trade","instId":"BTCUSDT"},"data":[«{"ts":"1704067207000","price":"PX","size":"SZ","side":"buy","tradeId":"1"}»]}"#,
            1.0,
            false,
        ),
        r("trades, Coinbase", Read::Trades, COINBASE, 1.0, false),
        r(
            "trades, Kraken",
            Read::Trades,
            r#"{"channel":"trade","type":"update","data":[«{"symbol":"BTC/USD","side":"buy","price":PX,"qty":SZ,"ord_type":"market","trade_id":1,"timestamp":"2024-01-01T00:00:07.000000Z"}»]}"#,
            1.0,
            false,
        ),
        r(
            "trades, Gate.io perps",
            Read::Trades,
            r#"{"time":1704067207,"channel":"futures.trades","event":"update","result":[«{"id":1,"size":SZ,"create_time_ms":1704067207000,"price":"PX","contract":"BTC_USDT"}»]}"#,
            0.0001,
            true,
        ),
        r(
            "trades, Gate.io spot",
            Read::Trades,
            r#"«{"time":1704067207,"channel":"spot.trades","event":"update","result":{"id":1,"create_time_ms":"1704067207000.0","side":"buy","currency_pair":"BTC_USDT","amount":"SZ","price":"PX"}}»"#,
            1.0,
            false,
        ),
        r(
            "trades, MEXC",
            Read::Trades,
            r#"{"symbol":"BTC_USDT","data":[«{"p":PX,"v":SZ,"T":1,"O":1,"M":1,"t":1704067207000,"i":"1"}»],"channel":"push.deal","ts":1704067207000}"#,
            0.0001,
            false,
        ),
        r("trades, Bitstamp", Read::Trades, BITSTAMP, 1.0, false),
        r("trades, KuCoin", Read::Trades, KUCOIN, 1.0, false),
        r(
            "trades, HTX",
            Read::Trades,
            r#"{"ch":"market.btcusdt.trade.detail","ts":1704067207000,"tick":{"id":1,"ts":1704067207000,"data":[«{"id":1,"ts":1704067207000,"tradeId":1,"amount":SZ,"price":PX,"direction":"buy"}»]}}"#,
            1.0,
            false,
        ),
        r(
            "trades, Crypto.com",
            Read::Trades,
            r#"{"id":-1,"method":"subscribe","code":0,"result":{"channel":"trade","instrument_name":"BTC_USDT","data":[«{"t":1704067207000,"p":"PX","q":"SZ","s":"BUY","i":"BTC_USDT","d":"1"}»]}}"#,
            1.0,
            false,
        ),
        r(
            "trades, Bitunix",
            Read::Trades,
            r#"{"ch":"trade","symbol":"BTCUSDT","ts":1704067207000,"data":[«{"t":"2024-01-01T00:00:07Z","p":"PX","v":"SZ","s":"buy"}»]}"#,
            1.0,
            false,
        ),
        // the order books
        r(
            "live book, Hyperliquid",
            Read::LiveBook,
            r#"{"channel":"l2Book","data":{"coin":"BTC","time":1,"levels":[[«{"px":"PX","sz":"SZ","n":1}»],[{"px":"200000","sz":"1","n":1}]]}}"#,
            1.0,
            false,
        ),
        r("book, Binance perp", Read::Book(Carnet::BinancePerp), BINANCE_BOOK, 1.0, false),
        r("book, Binance spot", Read::Book(Carnet::BinanceSpot), BINANCE_BOOK, 1.0, false),
        r(
            "book, Binance perp, written as numbers",
            Read::Book(Carnet::BinancePerp),
            r#"{"lastUpdateId":1,"bids":[«[PX,SZ]»],"asks":[[200000,1]]}"#,
            1.0,
            false,
        ),
        r("book, Bybit perp", Read::Book(Carnet::BybitPerp), BYBIT_BOOK, 1.0, false),
        r("book, Bybit spot", Read::Book(Carnet::BybitSpot), BYBIT_BOOK, 1.0, false),
        r("book, OKX perp", Read::Book(Carnet::OkxPerp), OKX_BOOK, 1.0, false),
        r("book, OKX spot", Read::Book(Carnet::OkxSpot), OKX_BOOK, 1.0, false),
        r(
            "book, Bitunix perp",
            Read::Book(Carnet::BitunixPerp),
            r#"{"code":0,"data":{"bids":[«["PX","SZ"]»],"asks":[["200000","1"]]},"msg":"Success"}"#,
            1.0,
            false,
        ),
        r("book, Hyperliquid perp", Read::Book(Carnet::HyperliquidPerp), HYPERLIQUID_BOOK, 1.0, false),
        r(
            "book, Coinbase spot",
            Read::Book(Carnet::CoinbaseSpot),
            r#"{"bids":[«["PX","SZ",1]»],"asks":[["200000","1",1]],"sequence":1,"auction_mode":false,"auction":null,"time":"2024-01-01T00:00:07Z"}"#,
            1.0,
            false,
        ),
        // the liquidations
        r(
            "liquidations, Binance",
            Read::Liquidations,
            r#"«{"e":"forceOrder","E":1704067207000,"o":{"s":"BTCUSDT","S":"SELL","o":"LIMIT","f":"IOC","q":"SZ","p":"PX","ap":"PX","X":"FILLED","l":"SZ","z":"SZ","T":1704067207000}}»"#,
            1.0,
            false,
        ),
        r(
            "liquidations, Bybit",
            Read::LiquidationsBybit,
            r#"{"topic":"allLiquidation.BTCUSDT","type":"snapshot","ts":1704067207000,"data":[«{"T":1704067207000,"s":"BTCUSDT","S":"Buy","v":"SZ","p":"PX"}»]}"#,
            1.0,
            false,
        ),
    ]
}

/// The candle in progress of a chart in 15 seconds (100 · 101 · 99 · 100.5, volume 7) after these trades came, seven
/// seconds into it, as one Hyperliquid message says them: « open high low close volume ».
fn candle_after(trades: &[(&str, &str)]) -> String {
    let trades: Vec<String> = trades
        .iter()
        .map(|(px, sz)| format!(r#"{{"coin":"BTC","side":"B","px":"{px}","sz":"{sz}","time":1704067207000,"tid":1}}"#))
        .collect();
    let msg = format!(r#"{{"channel":"trades","data":[{}]}}"#, trades.join(","));
    let mut bars = vec![Bar {
        time: START,
        open: 100.0,
        high: 101.0,
        low: 99.0,
        close: 100.5,
        volume: 7.0,
    }];
    for t in parse_trades(Exchange::Hyperliquid, &msg) {
        fold_trade(&mut bars, t, 15);
    }
    let b = bars[bars.len() - 1];
    format!("{} {} {} {} {}", b.open, b.high, b.low, b.close, b.volume)
}

/// The bids of the summed book (step 1) after Binance's photo came with these bids: « price:size », best first.
fn bids_after(bids: &str) -> String {
    let body = format!(r#"{{"lastUpdateId":1,"bids":{bids},"asks":[["101","1"]]}}"#);
    let Ok(book) = Carnet::BinancePerp.parse(&body, 1.0) else {
        return "not read".into();
    };
    let levels: Vec<String> = somme("BTC", &[book], 1.0, 1).bids.iter().map(|l| format!("{}:{}", l.px, l.sz)).collect();
    levels.join(" ")
}

/// 🧹 What cannot be is left out where it is read. A trade, a level of an order book or a liquidation whose price or
/// size is not a number above zero (not a number, infinite, zero, negative) never comes out of a reader, on any
/// exchange; what can be comes out as it came, and what came in the same message as something mad stays. So nothing
/// mad reaches a candle, a book, a list or a sum. (Two things are not mad: where the sign of the size says the side,
/// a negative size is a sale; in an update of a book kept live, a size of zero says the level is gone.)
#[test]
fn what_cannot_be_is_left_out_where_it_is_read() {
    let readers = readers();
    let nothing: [(f64, f64); 0] = [];
    let (mut left_out, mut kept) = (0, 0);
    for r in &readers {
        let name = r.name;
        // is the price, is the size, written as a text or as a JSON number?
        let (px_text, sz_text) = (r.message.contains("\"PX\""), r.message.contains("\"SZ\""));
        assert!(
            !(r.signed && sz_text),
            "{name}: a size whose sign says the side is a number in every stream read here"
        );
        // what can be comes out as it came, in the order it came (counted in coins where the exchange counts in
        // contracts)
        let (first, second) = (("97120.5", "25"), ("97121", "150"));
        let (a, b) = ((97120.5, 25.0 * r.coins), (97121.0, 150.0 * r.coins));
        assert_eq!(r.read(&[first, second]), [a, b], "{name}: two items that can be");
        assert_eq!(r.read(&[second, first]), [b, a], "{name}: two items that can be, the other way round");
        if r.signed {
            assert_eq!(r.read(&[first, ("97121", "-150")]), [a, b], "{name}: a negative size is a sale");
        }
        kept += 2;
        // what cannot be is left out, alone, before or after an item that can be
        let no_px: &[&str] = if px_text { &MAD } else { &MAD_NUMBERS };
        let no_sz: Vec<&str> = (if sz_text { &MAD[..] } else { &MAD_NUMBERS[..] })
            .iter()
            .copied()
            .filter(|s| !r.signed || zero(s))
            .collect();
        let cannot: Vec<(&str, &str)> = no_px.iter().map(|px| (*px, "150")).chain(no_sz.iter().map(|sz| ("97121", *sz))).collect();
        for no in cannot {
            assert_eq!(r.read(&[no]), nothing, "{name}: {no:?} cannot be");
            assert_eq!(r.read(&[first, no]), [a], "{name}: {no:?} cannot be, what came before it stays");
            assert_eq!(r.read(&[no, first]), [a], "{name}: {no:?} cannot be, what came after it stays");
            assert_eq!(r.read(&[no, first, no, second, no]), [a, b], "{name}: {no:?} cannot be, the rest stays");
            left_out += 1;
        }
    }
    assert!(
        readers.len() == 36 && left_out > 600 && kept == 72,
        "{} readers, {left_out} items left out, {kept} kept: each must be seen",
        readers.len()
    );

    // 📖 an update of the book kept live (Binance): a size of zero says the level is gone, and is read; a price or a
    // size that cannot be leaves its level out, and the update stays
    let update = |levels: &str| {
        let msg = format!(r#"{{"e":"depthUpdate","E":1,"T":1,"s":"BTCUSDT","U":1,"u":7,"pu":5,"b":[{levels}],"a":[{levels}]}}"#);
        let u = parse_maj(&msg).expect("an update of the book");
        assert_eq!((u.pu, u.u), (5, 7), "{levels}");
        assert_eq!(u.bids, u.asks, "{levels}: the same levels were written on both sides");
        u.bids
    };
    assert_eq!(
        update(r#"["97120.5","25"],["97121","0"],["97122","0.000"]"#),
        [(97120.5, 25.0), (97121.0, 0.0), (97122.0, 0.0)]
    );
    for no in MAD {
        assert_eq!(
            update(&format!(r#"["{no}","150"],["97120.5","25"]"#)),
            [(97120.5, 25.0)],
            "an update at the price {no}"
        );
        if !zero(no) {
            assert_eq!(
                update(&format!(r#"["97120.5","25"],["97121","{no}"]"#)),
                [(97120.5, 25.0)],
                "an update of size {no}"
            );
        }
    }

    // ⚡ so a candle in progress never takes a trade that cannot be (the first three were seen taken on 23 Nov 2026)
    let untouched = "100 101 99 100.5 7";
    assert_eq!(candle_after(&[]), untouched);
    assert_eq!(candle_after(&[("100.75", "1")]), "100 101 99 100.75 8", "a trade that can be moves the candle");
    assert_eq!(candle_after(&[("101.5", "0.5"), ("98", "0.25")]), "100 101.5 98 98 7.75");
    for no in [("NaN", "1"), ("0", "1"), ("100.75", "-3")] {
        assert_eq!(candle_after(&[no]), untouched, "the trade {no:?} cannot be");
    }
    for no in MAD {
        for trade in [(no, "1"), ("100.75", no)] {
            assert_eq!(candle_after(&[trade]), untouched, "the trade {trade:?} cannot be");
            assert_eq!(
                candle_after(&[trade, ("100.75", "1"), trade]),
                "100 101 99 100.75 8",
                "the trade {trade:?} cannot be"
            );
        }
    }

    // 📖 and a summed book never takes a level that cannot be (the first two were seen taken on 23 Nov 2026)
    assert_eq!(
        bids_after(r#"[["100.5","1"],["100.2","3"]]"#),
        "100:4",
        "two levels that can be, in the same step"
    );
    assert_eq!(bids_after(r#"[["100","1"],["NaN","2"]]"#), "100:1");
    assert_eq!(bids_after(r#"[["100.5","1"],["100.2","-3"]]"#), "100:1");
    assert_eq!(
        bids_after(r#"[["NaN","2"],["100.2","inf"]]"#),
        "",
        "no bid can be: the book is read without bids"
    );
    assert!(
        Carnet::BinancePerp
            .parse(r#"{"lastUpdateId":1,"bids":[["NaN","2"]],"asks":[["101","0"]]}"#, 1.0)
            .is_err(),
        "a book where nothing can be is not read"
    );
}
