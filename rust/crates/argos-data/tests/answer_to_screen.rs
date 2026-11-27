//! 📼 Test 4 of the data reliability: from the raw answer of an exchange to the candles a chart draws, with the
//! answers of 24 Nov. 2026 (`tests/recorded/2026-11-24/`) and no network. The steps are the app's own, in the order
//! of its first screen and of what follows it (`collecte::premier_ecran`, `collecte::completer`): the supervisor
//! (checks, fallback), the cache on disk (written, read again, joined to fresh candles), the regrouping into a
//! bigger interval, the holes found and asked again. Here they run with the clock of the answers. What the screen
//! writes from those candles is the subject of test 1 (`argos-render`): the number shown is the number that came.
//!
//! A hole stays a hole (no candle is made up), and a candle given twice or out of order changes nothing. A
//! difference seen on that day and understood is listed in `SEEN` with its reason, as in `answers_agree.rs`.

mod recorded;

use argos_core::{Bar, Timeframe};
use argos_data::supervisor::{Config, Supervisor};
use argos_data::{cache, trous, Exchange, ExchangeFeed};
use recorded::{Candles, Entry, Replay, Window, CANDLES};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::PathBuf;

/// The day the answers were taken: the name of their folder.
const DAY: &str = "2026-11-24";

/// The candles of that day and the moment they were taken (milliseconds).
fn kept_candles() -> (i64, Vec<Entry>) {
    let path = format!("{}/tests/recorded/{DAY}/answers-candles.txt", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let taken = recorded::taken(&text).unwrap_or_else(|| panic!("{path}: no note « taken »"));
    (taken, recorded::read(&text).unwrap_or_else(|e| panic!("{path}: {e}")))
}

/// The entries of one question: who answered, which kind of data.
fn of(entries: &[Entry], who: &str, kind: &str) -> Vec<Entry> {
    entries.iter().filter(|e| e.who == who && e.kind == kind).cloned().collect()
}

/// The question of the kit named `kind`.
fn question(kind: &str) -> &'static Candles {
    CANDLES.iter().find(|c| c.kind == kind).expect("a question of the kit")
}

/// A folder of its own for the cache of a test (emptied first, removed by the test when it ends).
fn folder(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the folder of the cache");
    dir
}

/// What the checks found: a name that stays the same from one run to the next, and what was seen.
#[derive(Default)]
struct Found(BTreeMap<String, String>);

impl Found {
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

    /// Fails unless what was found is exactly what `seen` lists.
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

/// The fields of candle `a` that are not those of `b`: prices to the digit, volumes to a thousandth (an exchange may
/// round the volume of each candle).
fn differ(a: &Bar, b: &Bar) -> Vec<String> {
    let apart = |x: f64, y: f64, tol: f64| (x - y).abs() > tol * y.abs();
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
        if apart(x, y, 1e-9) {
            out.push(format!("{name} {x} ≠ {y}"));
        }
    }
    if apart(a.volume, b.volume, 1e-3) {
        out.push(format!("volume {} ≠ {}", a.volume, b.volume));
    }
    out
}

/// Candles regrouped by the app compared with the exchange's own: the differences, candle by candle.
fn compare(made: &[Bar], theirs: &[Bar]) -> Vec<String> {
    if made.len() != theirs.len() {
        return vec![format!("{} candles made, {} given", made.len(), theirs.len())];
    }
    made.iter().zip(theirs).flat_map(|(m, t)| differ(m, t)).collect()
}

/// The candles `ex` gave to the question `kind` of the kit, read by the app's code; `None` when they are not the
/// candles asked, one after the other at the hours asked (`answers_agree.rs` says why).
async fn read(entries: &[Entry], ex: Exchange, kind: &str) -> Option<Vec<Bar>> {
    let c = question(kind);
    let bars = recorded::candles(Replay::new(&of(entries, ex.id(), kind)), ex, c).await.ok()?.bars;
    let (hour, day) = recorded::hour_and_day();
    let sec = c.tf.seconds();
    let first = match c.window {
        Window::Newest(_) => bars.first()?.time,
        Window::BeforeTheHour(n) => hour - n as i64 * sec,
        Window::SinceYesterday => day - 86_400,
        Window::BeforeToday(n) => day - n as i64 * sec,
        Window::BeforeThisWeek(n) => recorded::this_week() - n as i64 * sec,
    };
    let n = match c.window {
        Window::BeforeTheHour(n) | Window::BeforeToday(n) | Window::BeforeThisWeek(n) | Window::Newest(n) => n as i64,
        Window::SinceYesterday => (hour - first) / sec,
    };
    let asked = bars.len() as i64 == n && bars.iter().zip(0..).all(|(b, k)| b.time == first + k * sec);
    asked.then_some(bars)
}

/// Differences seen that day, and why.
const SEEN_FIRST_SCREEN: &[(&str, &str)] = &[];

/// ⚡ The first screen of a chart in one minute: the supervisor takes the exchange chosen, and its newest candles
/// pass its checks (not stale, no impossible price, no hole); a fallback is the exception, and says why.
#[tokio::test]
async fn the_chart_of_an_exchange_shows_its_own_candles() {
    let (taken, entries) = kept_candles();
    recorded::set_clock(taken);
    let newest: Vec<Entry> = entries.iter().filter(|e| e.kind == "candles-1m-newest").cloned().collect();
    // each answer as many times as the supervisor asks it (a first try and its new tries), as the network would
    // give it again a second later
    let tries = 1 + Config::default().retry_ms.len();
    let again: Vec<Entry> = newest.iter().flat_map(|e| std::iter::repeat_n(e.clone(), tries)).collect();
    let mut found = Found::default();
    for ex in Exchange::ALL {
        // every exchange's newest candles are there for the fallbacks, as the network would give them
        let sup = Supervisor::with_clock(Replay::new(&again), Config::default(), recorded::clock);
        match sup.candles(ex, recorded::COIN, Timeframe::M1, 5).await {
            Ok(s) if s.is_fallback() => {
                let why: Vec<String> = s.problems.iter().map(|(e, p)| format!("{}: {p}", e.id())).collect();
                found.note(
                    format!("{}: the chart shows another exchange's candles", ex.id()),
                    format!("{} ({})", s.source.id(), why.join(", ")),
                );
            }
            Ok(s) if !s.problems.is_empty() => {
                let why: Vec<String> = s.problems.iter().map(|(e, p)| format!("{}: {p}", e.id())).collect();
                found.note(format!("{}: the chart warns", ex.id()), why.join(", "));
            }
            Ok(_) => {}
            Err(e) => found.note(format!("{}: no candles at all", ex.id()), e.to_string()),
        }
    }
    found.agrees_with(SEEN_FIRST_SCREEN);
}

/// Differences seen that day, and why: the series the kit could not check are left out here (`answers_agree.rs`).
const SEEN_CHAIN: &[(&str, &str)] = &[(
    "bitunix 5m made of 1m",
    "Bitunix's minute of 04:20 opens at 60439.4, above its own high 60439.3: the app keeps it with its open at that \
     high (bitunix-bougies-1), one step under the open of Bitunix's own five minutes",
)];

/// 💾 The candles go through the cache unchanged (only closed ones are written), fresh candles take the place of
/// the cached ones from their first hour, and the regroupings the chart makes from them are the exchange's own
/// candles: five minutes from one minute, an hour from one minute, four hours and a day from one hour.
#[tokio::test]
async fn the_candles_go_through_the_cache_and_the_regroupings_unchanged() {
    let (taken, entries) = kept_candles();
    recorded::set_clock(taken);
    let now = taken / 1000;
    let (hour, day) = recorded::hour_and_day();
    let dir = folder("answer-to-screen-cache");
    let mut found = Found::default();
    for ex in Exchange::ALL {
        let id = ex.id();
        let at = |tf: Timeframe| dir.join(cache::file_name(ex, recorded::COIN, tf));

        // the newest candles: only the closed ones reach the disk (closed: their end, plus 2 minutes for the clock of
        // the computer, is past); the one in progress never does
        if let Some(newest) = read(&entries, ex, "candles-1m-newest").await {
            let closed: Vec<Bar> = newest.iter().filter(|b| b.time + 60 + 120 <= now).copied().collect();
            let _ = cache::write(&at(Timeframe::M1), &newest, Timeframe::M1, now);
            let back = cache::read(&at(Timeframe::M1), Timeframe::M1, now).unwrap_or_default();
            if back != closed {
                found.note(
                    format!("{id} newest: the cache"),
                    format!("{} candles read again, {} closed", back.len(), closed.len()),
                );
            }
        }

        // the minutes of the last closed hour: written, read again, the same; regrouped, the exchange's own
        let minutes = read(&entries, ex, "candles-1m").await;
        if let Some(m) = &minutes {
            let _ = cache::write(&at(Timeframe::M1), m, Timeframe::M1, now);
            let back = cache::read(&at(Timeframe::M1), Timeframe::M1, now);
            if back.as_ref() != Some(m) {
                found.note(format!("{id} minutes: the cache"), format!("{:?} candles read again", back.map(|b| b.len())));
            }
            if let Some(fives) = read(&entries, ex, "candles-5m").await {
                let diff = compare(&argos_core::aggregate(m, 300), &fives);
                if !diff.is_empty() {
                    found.note(format!("{id} 5m made of 1m"), diff.join(", "));
                }
            }
        }

        // the hours since yesterday: the cache cut in two and joined again, the fresh ones winning; four hours and
        // the day made of them
        let Some(hours) = read(&entries, ex, "candles-1h").await else { continue };
        let split = hours.len() / 2;
        let mut cached = hours[..split + cache::RELUES].to_vec();
        // a cached candle the exchange corrected since: the fresh one wins
        cached[split].close = cached[split].high;
        let _ = cache::write(&at(Timeframe::H1), &cached, Timeframe::H1, now);
        let back = cache::read(&at(Timeframe::H1), Timeframe::H1, now).unwrap_or_default();
        let joined = cache::merge(&back, hours[split..].to_vec(), Timeframe::H1, argos_data::DEFAULT_CANDLES);
        if joined != hours {
            found.note(
                format!("{id} hours: cache and fresh joined"),
                format!("{} candles, {} given", joined.len(), hours.len()),
            );
        }
        let yesterday: Vec<Bar> = hours.iter().filter(|b| b.time < day).copied().collect();
        if let Some(fours) = read(&entries, ex, "candles-4h").await {
            let diff = compare(&argos_core::aggregate(&yesterday, 14_400), &fours);
            if !diff.is_empty() {
                found.note(format!("{id} 4h made of 1h"), diff.join(", "));
            }
        }
        if let Some(days) = read(&entries, ex, "candles-1d").await {
            let diff = compare(&argos_core::aggregate(&yesterday, 86_400), &days[days.len() - 1..]);
            if !diff.is_empty() {
                found.note(format!("{id} day made of 1h"), diff.join(", "));
            }
        }
        if let Some(m) = &minutes {
            let made = argos_core::aggregate(m, 3600);
            let theirs: Vec<Bar> = hours.iter().filter(|b| b.time == hour - 3600).copied().collect();
            let diff = compare(&made, &theirs);
            if !diff.is_empty() {
                found.note(format!("{id} hour made of 1m"), diff.join(", "));
            }
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
    found.agrees_with(SEEN_CHAIN);
}

/// A moment in milliseconds, written as a number or as text, in seconds.
fn ms(v: &Value) -> i64 {
    v.as_i64().or_else(|| v.as_str()?.parse().ok()).unwrap_or(0) / 1000
}

/// Where the candles are in the answer of `who`, and the moment (seconds) of one of them.
fn candle_list(who: &str) -> (&'static str, fn(&Value) -> i64) {
    match who {
        "binance" => ("", |c| ms(&c[0])),
        "hl" => ("", |c| ms(&c["t"])),
        "okx" => ("/data", |c| ms(&c[0])),
        "bybit" => ("/result/list", |c| ms(&c[0])),
        _ => unreachable!("not one of the four ways of writing candles"),
    }
}

/// The answer `body` of `who` with its list of candles changed by `change` (written again by serde: the change is
/// not « as it came » any more, the candles in it are).
fn changed(who: &str, body: &str, change: impl Fn(&mut Vec<Value>, fn(&Value) -> i64)) -> String {
    let (at, time) = candle_list(who);
    let mut v: Value = serde_json::from_str(body).expect("json");
    let list = v.pointer_mut(at).and_then(Value::as_array_mut).expect("the list of candles");
    change(list, time);
    v.to_string()
}

/// 🕳 A hole stays a hole: the candles around it are the same, it is found, asked again, and stays empty when the
/// exchange has nothing; nothing is made up to fill it. A candle given twice or the candles out of order change
/// nothing. Four ways of writing candles: a list (Binance), objects (Hyperliquid), the newest first (OKX, Bybit).
#[tokio::test]
async fn a_hole_stays_a_hole_and_twice_or_out_of_order_changes_nothing() {
    let (taken, entries) = kept_candles();
    recorded::set_clock(taken);
    let now = taken / 1000;
    let (hour, _) = recorded::hour_and_day();
    let gone = hour - 1800;
    let dir = folder("answer-to-screen-holes");
    let c = question("candles-1m");
    for (ex, who) in [
        (Exchange::Binance, "binance"),
        (Exchange::Hyperliquid, "hl"),
        (Exchange::Okx, "okx"),
        (Exchange::Bybit, "bybit"),
    ] {
        let asked = of(&entries, who, "candles-1m");
        assert_eq!(asked.len(), 1, "{who}: one answer");
        let body = asked[0].answer.clone().expect("an answer");
        let with = |text: String| {
            vec![Entry {
                answer: Ok(text),
                ..asked[0].clone()
            }]
        };
        let clean = recorded::candles(Replay::new(&with(body.clone())), ex, c).await.expect("read").bars;
        assert_eq!(clean.len(), 60, "{who}");

        // twice, and out of order: the same candles
        let twice = changed(who, &body, |l, t| {
            let i = l.iter().position(|x| t(x) == gone).expect("the candle");
            l.insert(i, l[i].clone());
        });
        let upside = changed(who, &body, |l, _| l.reverse());
        for (what, text) in [("twice", twice), ("out of order", upside)] {
            let got = recorded::candles(Replay::new(&with(text)), ex, c).await.expect("read");
            assert_eq!(got.bars, clean, "{who} {what}");
            assert!(got.a_reparer.is_empty(), "{who} {what}: {:?}", got.a_reparer);
        }

        // a hole: every other candle as it came, the hole found and asked again, still empty, nothing made up (to
        // keep sixty candles the app may show one more from before, taken from the same answer)
        let holed = changed(who, &body, |l, t| l.retain(|x| t(x) != gone));
        let replay = Replay::new(&with(holed));
        let got = recorded::candles(replay.clone(), ex, c).await.expect("read").bars;
        let given = ex.parse_candles_checked(&body).expect("the whole answer").0;
        assert!(
            got.iter().all(|b| b.time != gone && given.contains(b)),
            "{who}: only candles that came: {got:?}"
        );
        assert!(
            clean.iter().all(|b| b.time == gone || got.contains(b)),
            "{who}: the candles around the hole: {got:?}"
        );
        // (with fewer candles than asked, the app asks one page further back: nothing was kept for it)
        assert!(replay.unknown().len() <= 1, "{who}: {:?}", replay.unknown());
        let path = dir.join(cache::file_name(ex, recorded::COIN, Timeframe::M1));
        cache::write(&path, &got, Timeframe::M1, now).expect("the cache");
        let back = cache::read(&path, Timeframe::M1, now).expect("the cache read again");
        assert_eq!(back, got, "{who}: the hole is kept in the cache");
        let holes = trous::trouver(&back, Timeframe::M1, &[], trous::calendrier(ex, recorded::COIN), &[]);
        assert_eq!(holes, [trous::Trou { debut: gone, fin: gone + 60 }], "{who}");
        let feed = ExchangeFeed::with_clock(ex, Replay::default(), recorded::clock);
        let r = trous::reparer(&feed, recorded::COIN, Timeframe::M1, &holes, 10).await;
        assert!(r.bars.is_empty() && r.irreparables == holes, "{who}: {r:?}");
        let shown = trous::combler(&back, &r.bars);
        assert_eq!(shown, got, "{who}: nothing made up");
        // the hour made of them is short of that minute's volume, and says nothing else
        let hour_of = |bars: &[Bar]| -> Vec<Bar> {
            let within: Vec<Bar> = bars.iter().filter(|b| (hour - 3600..hour).contains(&b.time)).copied().collect();
            argos_core::aggregate(&within, 3600)
        };
        let (made, full) = (hour_of(&shown), hour_of(&clean));
        let lost = clean.iter().find(|b| b.time == gone).map_or(0.0, |b| b.volume);
        assert_eq!(made.len(), 1, "{who}");
        assert!(
            (made[0].volume + lost - full[0].volume).abs() <= 1e-9 * full[0].volume,
            "{who}: {made:?} {full:?}"
        );
        let same = |a: &Bar, b: &Bar| (a.time, a.open, a.close) == (b.time, b.open, b.close);
        assert!(same(&made[0], &full[0]), "{who}: the hour opens and closes as before: {made:?} {full:?}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}
