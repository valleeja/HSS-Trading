//! 📼 The answers of the exchanges, read with no network. The answers are taken once by `record_answers.rs` (on the
//! real network, ignored by default); here the app's own code asks the same questions again and gets the kept
//! answers. This file first holds the kit itself to its word: a file is read back as it was written, wallets and
//! network addresses never get into a file, and the kept answers go to the questions that were asked, to no other.

mod recorded;

use argos_core::{ArgosError, Result};
use argos_data::http::Http;
use argos_data::Exchange;
use recorded::{Asked, Entry, Recording, Replay, Window};

/// A client that answers like Binance does, from nothing but the question: its candles are made of their own
/// times. Everything else fails, as a network that is not there.
#[derive(Clone)]
struct Made;

/// The value of `name` in the address `url` (« interval » in « …?symbol=BTCUSDT&interval=1m »).
fn value<'a>(url: &'a str, name: &str) -> Option<&'a str> {
    url.split(['?', '&']).find_map(|p| p.strip_prefix(name)?.strip_prefix('='))
}

impl Http for Made {
    async fn get(&self, url: &str) -> Result<String> {
        let failed = || {
            Err(ArgosError::Feed(format!(
                "no network in a test, asked by 203.0.113.7: {}",
                url.chars().take(60).collect::<String>()
            )))
        };
        if !url.starts_with("https://fapi.binance.com/fapi/v1/klines?symbol=") {
            return failed();
        }
        let seconds = match value(url, "interval") {
            Some("1m") => 60,
            Some("5m") => 300,
            Some("1h") => 3600,
            Some("4h") => 14_400,
            Some("12h") => 43_200,
            Some("1d") => 86_400,
            Some("1w") => 604_800,
            _ => return failed(),
        };
        let limit: i64 = value(url, "limit").and_then(|l| l.parse().ok()).unwrap_or(0);
        // the candles that began at or before `endTime` (none given: now), the newest last
        let end_ms: i64 = value(url, "endTime").and_then(|e| e.parse().ok()).unwrap_or_else(recorded::clock);
        // (a week begins on Monday 00:00 UTC, as Binance's)
        let last = argos_core::bucket_start(end_ms.div_euclid(1000), seconds);
        let rows: Vec<String> = (0..limit)
            .rev()
            .map(|k| {
                let t = last - k * seconds;
                let open = 97_000.0 + (t / 60 % 500) as f64 * 0.5;
                format!(
                    r#"[{},"{open}","{}","{}","{}","{}"]"#,
                    t * 1000,
                    open + 3.0,
                    open - 2.0,
                    open + 0.5,
                    seconds / 60
                )
            })
            .collect();
        Ok(format!("[{}]", rows.join(",")))
    }

    async fn post_json(&self, url: &str, _body: &str) -> Result<String> {
        Err(ArgosError::Feed(format!("no network in a test: {url}")))
    }
}

/// A client that says the same thing to every question: an answer, or the reason there is none.
#[derive(Clone)]
struct Says(recorded::Answer);

impl Http for Says {
    async fn get(&self, _url: &str) -> Result<String> {
        self.0.clone().map_err(ArgosError::Other)
    }

    async fn post_json(&self, _url: &str, _body: &str) -> Result<String> {
        self.0.clone().map_err(ArgosError::Other)
    }
}

/// The moment of the trial: Monday 23 November 2026, 19:37:12 UTC.
const MOMENT: i64 = 1_795_462_632_000;

/// Every candles question to two exchanges (one answers, one does not), as text: what the app made of the answers.
async fn asked_to<H: Http + Clone>(http: &H, mut each: impl FnMut(Exchange, &'static str)) -> Vec<String> {
    let mut read = vec![];
    for ex in [Exchange::Binance, Exchange::Okx] {
        for c in &recorded::CANDLES {
            read.push(format!(
                "{} {}: {:?}",
                ex.id(),
                c.kind,
                recorded::candles(http.clone(), ex, c).await.map_err(|e| e.to_string())
            ));
            each(ex, c.kind);
        }
    }
    read
}

#[tokio::test]
async fn the_kept_answers_go_to_the_questions_that_were_asked() {
    // the day the answers are taken: the app's code asks, every question and its answer are kept
    recorded::set_clock(MOMENT);
    let net = Recording::new(Made);
    let mut entries = vec![];
    let first = asked_to(&net, |ex, kind| entries.extend(net.take(ex.id(), kind))).await;
    let n = recorded::CANDLES.len();
    assert_eq!(first.len(), 2 * n);
    assert!(
        first[..n].iter().all(|r| r.contains("Ok(")) && first[n..].iter().all(|r| r.contains("Err(")),
        "{first:#?}"
    );
    assert_eq!(entries.len(), 2 * n, "one request for each question: {entries:#?}");
    assert!(entries.iter().all(|e| e.answer.is_ok() == (e.who == "binance")));
    // the reason of a failure is kept without the network address it named
    assert!(
        matches!(&entries[n].answer, Err(why) if why.contains("asked by 0.0.0.0") && !why.contains("203.")),
        "{:?}",
        entries[n].answer
    );

    // the candles are the ones each question describes: at 19:37 the last closed hour began at 18:00
    let (hour, day) = recorded::hour_and_day();
    assert_eq!((hour, day), (MOMENT / 1000 - 37 * 60 - 12, MOMENT / 1000 - 19 * 3600 - 37 * 60 - 12));
    assert_eq!(recorded::this_week(), day, "that Monday");
    for (c, e) in recorded::CANDLES.iter().zip(&entries) {
        let k = recorded::candles(Replay::new(std::slice::from_ref(e)), Exchange::Binance, c)
            .await
            .expect("answered");
        let (first, last) = (k.bars.first().expect("candles").time, k.bars.last().expect("candles").time);
        let sec = c.tf.seconds();
        let expected = match c.window {
            Window::Newest(n) => (MOMENT / 1000 / sec * sec - (n as i64 - 1) * sec, MOMENT / 1000 / sec * sec, n),
            Window::BeforeTheHour(n) => (hour - n as i64 * sec, hour - sec, n),
            Window::SinceYesterday => (day - 86_400, hour - sec, 24 + 19),
            Window::BeforeToday(n) => (day - n as i64 * sec, day - sec, n),
            Window::BeforeThisWeek(n) => (day - n as i64 * sec, day - sec, n),
        };
        assert_eq!((first, last, k.bars.len()), expected, "{}", c.kind);
        assert!(
            k.a_reparer.is_empty() && k.bars.windows(2).all(|w| w[1].time - w[0].time == sec),
            "{}: no hole",
            c.kind
        );
    }

    // the file, written and read back
    let text = recorded::file("a trial", MOMENT, &entries);
    assert!(text.starts_with(
        "# a trial\n# taken 1795462632000\n@ binance candles-1m-newest GET https://fapi.binance.com/fapi/v1/klines?symbol=BTCUSDT&interval=1m&limit=5\nok "
    ));
    let back = recorded::read(&text).expect("the file is readable");
    assert_eq!(back, entries);

    // any other day, with no network: the clock of the file, the same questions, the same answers, nothing more
    recorded::set_clock(0);
    recorded::set_clock(recorded::taken(&text).expect("the file says when it was taken"));
    let replay = Replay::new(&back);
    let again = asked_to(&replay, |_, _| ()).await;
    // (the reason of a failure comes back as it was kept: without the network address)
    let told: Vec<String> = first.iter().map(|r| recorded::without_addresses(r)).collect();
    assert_eq!(again, told, "the app reads the kept answers as it read the real ones");
    assert!(first != told && first[..n] == told[..n]);
    assert_eq!(
        (replay.unknown(), replay.left()),
        (vec![], vec![]),
        "every kept answer was asked for, and nothing else was asked"
    );

    // a question that was not asked that day has no answer, and is noted; an answer is given once
    let other = Replay::new(&back[..1]);
    assert!(other
        .get("https://fapi.binance.com/fapi/v1/klines?symbol=ETHUSDT&interval=1m&limit=5")
        .await
        .is_err());
    let Asked::Get(url) = &back[0].asked else { panic!("a GET") };
    assert!(other.get(url).await.is_ok() && other.get(url).await.is_err());
    assert_eq!(other.unknown().len(), 2);
    assert!(other.left().is_empty());

    // with another clock the questions are no longer the ones that were asked: the test says so instead of passing
    recorded::set_clock(MOMENT + 3_600_000);
    let late = Replay::new(&back);
    let _ = asked_to(&late, |_, _| ()).await;
    assert!(!late.unknown().is_empty() && !late.left().is_empty());
}

/// Letters of more than one byte.
const LETTERS: &str = "prix « 97 123,5 » → ₿";

#[test]
fn a_file_is_read_back_as_it_was_written() {
    let entry = |who: &str, kind: &str, asked: Asked, answer: recorded::Answer| Entry {
        who: who.into(),
        kind: kind.into(),
        asked,
        answer,
    };
    let entries = vec![
        entry(
            "binance",
            "candles-1m",
            Asked::get("https://example.invalid/klines?symbol=BTCUSDT&interval=1m"),
            Ok("[[1700000000]]".into()),
        ),
        // an answer of several lines, with a line that looks like a header, a note and a count; an empty answer
        entry(
            "hl",
            "book",
            Asked::post("https://example.invalid/info", r#"{"type":"l2Book","coin":"BTC"}"#),
            Ok("a\n@ x y GET z\n# note\nok 3\n\n".into()),
        ),
        entry("hl", "markets", Asked::post("https://example.invalid/info", ""), Ok(String::new())),
        entry(
            "okx",
            "funding",
            Asked::get("https://example.invalid/funding"),
            Err("HTTP 429 (Retry-After 2 s) : trop de requêtes".into()),
        ),
        // letters of more than one byte: the count is in bytes
        entry(
            "kraken",
            "live-trades",
            Asked::Live {
                url: "wss://example.invalid/v2".into(),
                subscribe: r#"{"a":1} | {"b":2}"#.into(),
            },
            Ok(LETTERS.into()),
        ),
        entry(
            "bitstamp",
            "live-trades",
            Asked::Live {
                url: "wss://example.invalid".into(),
                subscribe: String::new(),
            },
            Err(String::new()),
        ),
    ];
    let text = recorded::write(&entries);
    assert_eq!(recorded::read(&text), Ok(entries.clone()));
    assert_eq!((LETTERS.chars().count(), LETTERS.len()), (21, 27));
    assert!(
        text.contains(&format!(
            "@ kraken live-trades LIVE wss://example.invalid/v2 {{\"a\":1}} | {{\"b\":2}}\nok 27\n{LETTERS}\n"
        )),
        "{text}"
    );

    // with its notes, with the last end of line lost, with empty lines between two entries
    let whole = recorded::file("a trial\nof the file", 1_795_462_632_000, &entries);
    assert!(whole.starts_with("# a trial of the file\n# taken 1795462632000\n@ binance "));
    assert_eq!(recorded::taken(&whole), Some(1_795_462_632_000));
    assert_eq!(recorded::taken(&text), None);
    assert_eq!(recorded::read(&whole), Ok(entries.clone()));
    assert_eq!(recorded::read(whole.trim_end_matches('\n')), Ok(entries.clone()));
    assert_eq!(
        recorded::read(&format!("\n\n{}\n\n# the end\n", recorded::write(&entries[..1]))),
        Ok(entries[..1].to_vec())
    );
    assert_eq!(recorded::read(""), Ok(vec![]));

    // what is asked is written on one line, and compared as it is written
    assert_eq!(
        Asked::post("https://example.invalid/a", "{\n\"b\":1\n}"),
        Asked::Post {
            url: "https://example.invalid/a".into(),
            body: "{ \"b\":1 }".into()
        }
    );
    let sub = argos_data::live::LiveSub {
        url: "wss://example.invalid".into(),
        subscribe: "one\n\ntwo\n".into(),
        ..Default::default()
    };
    assert_eq!(
        Asked::live(&sub),
        Asked::Live {
            url: "wss://example.invalid".into(),
            subscribe: "one | two".into()
        }
    );

    // a file that was cut or changed by hand is refused, and the place is said
    for (broken, why) in [
        ("@ binance candles GET https://example.invalid\nok 5\nabc\n", "length"),
        ("@ binance candles GET https://example.invalid\nok 3\nabcd\n", "length"),
        ("@ binance candles GET https://example.invalid\nyes 3\nabc\n", "count line"),
        ("@ binance candles GET https://example.invalid\nok three\nabc\n", "not a number"),
        ("@ binance candles GET https://example.invalid", "no count line"),
        ("@ binance candles FETCH https://example.invalid\nok 0\n\n", "not GET"),
        ("@ binance candles GET https://example.invalid more\nok 0\n\n", "not GET"),
        ("binance candles GET https://example.invalid\nok 0\n\n", "not the header"),
        ("@ binance candles\nok 0\n\n", "not the header"),
        ("@ a b GET c\nok 2\n€\n", "length"),
    ] {
        let said = recorded::read(broken).expect_err(broken);
        assert!(said.contains(why) && said.starts_with("byte 0: "), "{broken:?}: {said}");
    }
    let second = format!("{}@ binance candles GET x\nok 9\nabc\n", recorded::write(&entries[..1]));
    assert!(recorded::read(&second)
        .expect_err("cut")
        .starts_with(&format!("byte {}: ", recorded::write(&entries[..1]).len())));
}

#[tokio::test]
async fn wallets_and_network_addresses_never_get_into_a_file() {
    use recorded::{without_addresses, without_wallets};
    // a public trade of Hyperliquid names two wallets and a transaction: forty hexadecimal digits or more
    let wallet = format!("0x{}", "aB3d".repeat(10));
    let hash = format!("0x{}", "9f".repeat(32));
    let trade = format!(r#"{{"coin":"BTC","px":"97123.5","sz":"0.25","hash":"{hash}","tid":1042,"users":["{wallet}","{wallet}"]}}"#);
    let clean = without_wallets(&trade);
    assert_eq!(
        clean,
        format!(
            r#"{{"coin":"BTC","px":"97123.5","sz":"0.25","hash":"0x{}","tid":1042,"users":["0x{z}","0x{z}"]}}"#,
            "0".repeat(64),
            z = "0".repeat(40)
        )
    );
    assert_eq!(clean.len(), trade.len(), "the same length: the rest of the answer is where it was");
    // what is not a wallet stays: a short hexadecimal number, a price, a text with no « 0x »
    for same in [
        "0x1f",
        r#"{"px":"0.00001631","id":"0xabc","n":10000000000000000000000000000000000000000}"#,
        &"ab".repeat(40),
        "prix « 0x » ₿",
        "",
    ] {
        assert_eq!(without_wallets(same), same);
    }
    assert_eq!(without_wallets(&format!("0x{}", "f".repeat(39))), format!("0x{}", "f".repeat(39)));
    assert_eq!(without_wallets(&format!("₿0x{}₿0x12", "F".repeat(41))), format!("₿0x{}₿0x12", "0".repeat(41)));

    // an exchange that refuses a request may name the computer that asked
    for (told, kept) in [
        (
            "Way too many requests; IP(203.0.113.7) banned until 1795462632000.",
            "Way too many requests; IP(0.0.0.0) banned until 1795462632000.",
        ),
        ("refused from 198.51.100.24:443.", "refused from 0.0.0.0:443."),
        ("blocked 2001:db8:85a3:0:0:8a2e:370:7334 at 19:26:26", "blocked :: at 19:26:26"),
        ("blocked [2001:db8::1] and ::1", "blocked [::] and ::"),
        ("::ffff:192.0.2.1", "::.0.0.0"),
        // what is not an address stays: a price, a version, an hour, a date, a JSON text
        (
            "close 97123.5, 0.25 BTC, v1.2.3, 19:26:26, 2026.11.23.19, 1.2.3.4.5, 300.1.1.1",
            "close 97123.5, 0.25 BTC, v1.2.3, 19:26:26, 2026.11.23.19, 1.2.3.4.5, 300.1.1.1",
        ),
        (
            r#"{"code":-1003,"msg":"a:b","data":[1.5,2.5,3.5,4.5]}"#,
            r#"{"code":-1003,"msg":"a:b","data":[1.5,2.5,3.5,4.5]}"#,
        ),
        (
            "HTTP 429 (Retry-After 12 s) : trop de requêtes « ₿ »",
            "HTTP 429 (Retry-After 12 s) : trop de requêtes « ₿ »",
        ),
        ("", ""),
    ] {
        assert_eq!(without_addresses(told), kept);
    }

    // what a recording keeps went through both, an answer as the reason there is none
    let said = format!(r#"{{"msg":"asked by 203.0.113.7","px":"97123.5","users":["{wallet}"]}}"#);
    let clean = format!(r#"{{"msg":"asked by 0.0.0.0","px":"97123.5","users":["0x{}"]}}"#, "0".repeat(40));
    for answer in [Ok(said.clone()), Err(said.clone())] {
        let net = Recording::new(Says(answer.clone()));
        assert_eq!(
            net.get("https://example.invalid/a").await.map_err(|e| e.to_string()),
            answer,
            "the app is told all"
        );
        let _ = net.post_json("https://example.invalid/b", "{}").await;
        let kept: Vec<recorded::Answer> = net.take("x", "book").into_iter().map(|e| e.answer).collect();
        let expected = if answer.is_ok() { Ok(clean.clone()) } else { Err(clean.clone()) };
        assert_eq!(kept, [expected.clone(), expected]);
        assert!(net.take("x", "book").is_empty(), "what was taken is taken once");
    }
    assert_eq!(recorded::kept(&said), clean);
}

#[test]
fn every_stream_is_listened_to_once_on_an_address_of_the_allow_list() {
    let streams = recorded::streams();
    for (i, s) in streams.iter().enumerate() {
        assert!(
            s.sub.url.starts_with("wss://") && argos_data::net::verifier(&s.sub.url).is_ok(),
            "{} {}: {}",
            s.who,
            s.kind,
            s.sub.url
        );
        assert!(!s.who.contains(char::is_whitespace) && s.kind.starts_with("live-"), "{} {}", s.who, s.kind);
        assert!(
            streams[..i].iter().all(|other| other.sub != s.sub),
            "{} {}: the same stream twice",
            s.who,
            s.kind
        );
        // the header of its entries is read back as it is written
        let e = Entry {
            who: s.who.clone(),
            kind: s.kind.to_string(),
            asked: Asked::live(&s.sub),
            answer: Ok("{}".into()),
        };
        assert_eq!(recorded::read(&recorded::write(std::slice::from_ref(&e))), Ok(vec![e]));
    }
    let count = |kind: &str| streams.iter().filter(|s| s.kind == kind).count();
    // every exchange has live candles but KuCoin (a ticket first); the trades of the Trades module: its list but KuCoin
    assert_eq!(count("live-candles"), Exchange::ALL.len() - 1);
    assert!(count("live-tape") >= 10 && count("live-tape") < argos_data::whales::EXCHANGES.len());
    assert_eq!(
        (count("live-book"), count("live-figures"), count("live-depth"), count("live-liquidations")),
        (1, 1, 1, 2)
    );
    // 🔒 the live candles of 12 hours, a day and a week of the exchanges that name them by their hour of UTC
    let utc = recorded::LIVE_UTC.len();
    assert_eq!((count("live-candles-12h"), count("live-candles-1d"), count("live-candles-1w")), (utc, utc, utc));
    assert!(streams
        .iter()
        .filter(|s| s.kind.starts_with("live-candles-"))
        .all(|s| s.sub.subscribe.contains("utc\"")));
    assert_eq!(streams.len(), count("live-candles") + count("live-trades") + count("live-tape") + 5 + 3 * utc);
}
