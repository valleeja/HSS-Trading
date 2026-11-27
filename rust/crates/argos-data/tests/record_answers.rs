//! 🎙 Takes the answers of the exchanges on the real network, once (ignored by default). To run it at home:
//!   cargo test --workspace --locked --test record_answers -- --ignored --nocapture
//!
//! It asks every exchange what the app asks (candles, markets, order books, funding rates, live streams), through
//! the app's own code: read-only, public addresses of the allow-list, no account. What was answered is written as
//! text files under `target/tmp/recorded-answers/`; nothing is sent anywhere else. Wallets named by public trades
//! are written as zeros. The offline tests then read the files kept in `tests/recorded/`.
//!
//! Six to ten minutes: one question at a time to an exchange, with twice its own pause between two questions. What
//! is taken is written as it goes, with `summary.txt`: what was asked, what was answered, what the app made of it.

#![cfg(feature = "native")]

mod recorded;

use argos_core::Bar;
use argos_data::http::NativeHttp;
use argos_data::live::{LiveEvent, LiveSub};
use argos_data::murs::Carnet;
use argos_data::Exchange;
use recorded::{Asked, Entry, Recording, Stream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// A live stream is listened to this long at most; liquidations longer: a quiet market has one from time to time.
const LISTEN: Duration = Duration::from_secs(20);
const LISTEN_TO_LIQUIDATIONS: Duration = Duration::from_secs(60);
/// No more than this many messages of a stream are kept (a book: fewer, its messages are long; liquidations: fewer,
/// they are waited for), nor this many bytes.
const MOST_MESSAGES: usize = 12;
const MOST_MESSAGES_OF_A_BOOK: usize = 6;
const MOST_LIQUIDATIONS: usize = 5;
const MOST_BYTES: usize = 256 * 1024;
/// An exchange is asked its candles for this long at most. One that says « too many requests » is waited for, as
/// the app waits; past this, its next questions are not asked.
const MOST_FOR_AN_EXCHANGE: Duration = Duration::from_secs(150);
/// Streams listened to together, each on its own connection.
const TOGETHER: usize = 12;

/// What a stream said so far.
#[derive(Default)]
struct Heard {
    connected: bool,
    lost: Option<String>,
    messages: Vec<String>,
    bytes: usize,
    stop: bool,
}

type Ears = Arc<Mutex<Heard>>;

/// The two halves the app's live connection is given: the first keeps each message as it came, the second says
/// when to stop (enough messages, the stream fell, or `stop` was asked).
fn ears(
    most: usize,
) -> (
    Ears,
    impl Fn(&str) -> Option<LiveEvent> + Send + 'static,
    impl Fn(LiveEvent) -> bool + Send + 'static,
) {
    let heard = Ears::default();
    let (kept, told) = (heard.clone(), heard.clone());
    let keep = move |text: &str| {
        if let Ok(mut h) = kept.lock() {
            if !h.stop && h.messages.len() < most && h.bytes + text.len() <= MOST_BYTES {
                h.bytes += text.len();
                h.messages.push(text.to_string());
            }
        }
        // every message goes on to the second half, which says when to stop
        Some(LiveEvent::Trades(vec![]))
    };
    let go_on = move |event: LiveEvent| {
        let Ok(mut h) = told.lock() else { return false };
        match event {
            LiveEvent::Connected => h.connected = true,
            // one try: a stream that falls is not opened again
            LiveEvent::Disconnected(why) | LiveEvent::Indisponible(why) => {
                h.lost = Some(why);
                h.stop = true;
            }
            _ => {}
        }
        !h.stop && h.messages.len() < most
    };
    (heard, keep, go_on)
}

/// Opens a stream with the app's own connection and listens.
fn listen(sub: LiveSub, most: usize) -> Ears {
    let (heard, keep, go_on) = ears(most);
    argos_data::live::spawn(sub, keep, go_on);
    heard
}

fn most_of(kind: &str) -> usize {
    match kind {
        "live-book" | "live-depth" => MOST_MESSAGES_OF_A_BOOK,
        "live-liquidations" => MOST_LIQUIDATIONS,
        _ => MOST_MESSAGES,
    }
}

fn listened_for(kind: &str) -> Duration {
    if kind == "live-liquidations" {
        LISTEN_TO_LIQUIDATIONS
    } else {
        LISTEN
    }
}

/// Has this stream nothing more to say (enough messages, fallen, stopped)?
fn finished(h: &Ears, most: usize) -> bool {
    h.lock().map_or(true, |h| h.stop || h.messages.len() >= most)
}

/// What a stream said, as entries: its messages, or why there is none.
fn entries_of(s: &Stream, h: &Ears) -> Vec<Entry> {
    let (messages, connected, lost) = match h.lock() {
        Ok(mut h) => {
            h.stop = true;
            (std::mem::take(&mut h.messages), h.connected, h.lost.clone())
        }
        Err(_) => (vec![], false, Some("the listening stopped".to_string())),
    };
    let entry = |answer| Entry {
        who: s.who.clone(),
        kind: s.kind.to_string(),
        asked: Asked::live(&s.sub),
        answer,
    };
    if messages.is_empty() {
        let why = lost.unwrap_or_else(|| {
            format!(
                "{} in {} s",
                if connected { "connected, no message" } else { "no connection" },
                listened_for(s.kind).as_secs()
            )
        });
        return vec![entry(Err(recorded::kept(&why)))];
    }
    messages.into_iter().map(|m| entry(Ok(recorded::kept(&m)))).collect()
}

/// Listens to the streams, a few together, and gives what they said.
async fn listen_to(streams: &[Stream], told: &mut Told) -> Vec<Entry> {
    let mut all = vec![];
    for wave in streams.chunks(TOGETHER) {
        let heard: Vec<Ears> = wave.iter().map(|s| listen(s.sub.clone(), most_of(s.kind))).collect();
        let start = Instant::now();
        // until each stream of the wave has said enough, has fallen, or was listened to long enough
        while !wave
            .iter()
            .zip(&heard)
            .all(|(s, h)| finished(h, most_of(s.kind)) || start.elapsed() >= listened_for(s.kind))
        {
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
        for (s, h) in wave.iter().zip(&heard) {
            let entries = entries_of(s, h);
            told.say(format!("{:<15} {:<18} {}", s.who, s.kind, said(&entries)));
            all.extend(entries);
        }
    }
    all
}

/// One line about what was answered: how many answers, their size, or the reason of the first failure.
fn said(entries: &[Entry]) -> String {
    let bytes: usize = entries.iter().filter_map(|e| e.answer.as_ref().ok()).map(String::len).sum();
    match entries.iter().find_map(|e| e.answer.as_ref().err()) {
        Some(why) => format!("FAILED · {}", why.chars().take(160).collect::<String>().replace('\n', " ")),
        None => format!("{} answer(s) · {bytes} bytes", entries.len()),
    }
}

/// « 19:00 → 19:59 · close 97123.5 », UTC.
fn span(bars: &[Bar]) -> String {
    let hm = |t: i64| format!("{:02}:{:02}", t.rem_euclid(86_400) / 3600, t.rem_euclid(3600) / 60);
    match (bars.first(), bars.last()) {
        (Some(a), Some(z)) => format!(" · {} → {} UTC · close {}", hm(a.time), hm(z.time), z.close),
        _ => String::new(),
    }
}

/// Between two questions to the same exchange: twice its own pause between two pages (the app may be open on the
/// same computer, and shares what the exchange allows), 300 ms at least.
async fn pause(ex: Exchange) {
    tokio::time::sleep(Duration::from_millis((2 * ex.page_pause_ms()).max(300))).await;
}

/// What the recorder says: printed as it goes, and kept for the summary written beside the answers.
#[derive(Default)]
struct Told(Vec<String>);

impl Told {
    fn say(&mut self, line: String) {
        println!("{line}");
        self.0.push(line);
    }

    /// One line for one question: who, which kind, what was answered and what the app made of it (said once when
    /// the app only repeats why there was no answer).
    fn question(&mut self, who: &str, kind: &str, asked: &[Entry], read: &str) {
        self.say(one_line(who, kind, &said(asked), read));
    }

    fn save(&self, dir: &Path) {
        let _ = std::fs::write(dir.join("summary.txt"), self.0.join("\n") + "\n");
    }
}

fn one_line(who: &str, kind: &str, said: &str, read: &str) -> String {
    match read.strip_prefix(THE_APP_SAYS) {
        Some(why) if said.contains(why) => format!("{who:<15} {kind:<18} {said}"),
        _ => format!("{who:<15} {kind:<18} {said} · {read}"),
    }
}

const THE_APP_SAYS: &str = "the app says: ";

/// What the app says of an answer it does not take, on one short line.
fn refused(e: &argos_core::ArgosError) -> String {
    let why = recorded::kept(&e.to_string()).chars().take(120).collect::<String>();
    format!("{THE_APP_SAYS}{}", why.replace('\n', " "))
}

/// If the app's own code falls on an answer.
const FELL: &str = "THE APP'S CODE PANICKED ON THIS ANSWER";

/// Asks, and does not fall with the app: an answer is kept before it is read, so an answer that makes the app's
/// code panic is in the file, and the recorder goes on to the next question.
async fn ask<T>(question: impl std::future::Future<Output = T>) -> Option<T> {
    use futures_util::FutureExt;
    std::panic::AssertUnwindSafe(question).catch_unwind().await.ok()
}

/// Writes a file of answers, reads it back, and says how big it is. The file says when its answers were asked:
/// `since` is when the first one was, and the last one is asked now.
fn save(dir: &Path, name: &str, about: &str, since: Duration, entries: &[Entry]) -> String {
    let now = Duration::from_millis((system_ms() - recorded::clock()).max(0) as u64);
    let about = format!("{about} · asked {} to {} s after the moment below", since.as_secs(), now.as_secs());
    let text = recorded::file(&about, recorded::clock(), entries);
    let failed = entries.iter().filter(|e| e.answer.is_err()).count();
    let back = match recorded::read(&text) {
        Ok(back) if back == entries => "read back the same".to_string(),
        Ok(_) => "NOT READ BACK THE SAME".to_string(),
        Err(why) => format!("NOT READABLE: {why}"),
    };
    match std::fs::write(dir.join(name), &text) {
        Ok(()) => format!("{name:<22} {:>9} bytes · {} answers, {failed} failed · {back}", text.len(), entries.len()),
        Err(e) => format!("{name:<22} NOT WRITTEN: {e}"),
    }
}

/// The computer's own clock, in milliseconds.
fn system_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as i64)
}

#[tokio::test]
#[ignore]
async fn record_the_answers_of_the_exchanges() {
    // just after an hour began an exchange may still be closing its last candle: the hour must be 90 s old
    let young = 90_000 - system_ms().rem_euclid(3_600_000);
    if young > 0 {
        println!("the hour just began: waiting {} s", young / 1000 + 1);
        tokio::time::sleep(Duration::from_millis(young as u64)).await;
    }
    let taken = system_ms();
    recorded::set_clock(taken);
    let start = Instant::now();
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("recorded-answers");
    std::fs::create_dir_all(&dir).expect("the folder of the answers");
    println!("the answers go to {}", dir.display());
    let net = Recording::new(NativeHttp::default());
    let (mut told, mut files) = (Told::default(), vec![]);
    told.say(format!("taken at {taken} (milliseconds, UTC)"));

    told.say("\n== candles ==".into());
    let (name, about) = ("answers-candles.txt", "candles of every exchange, asked as a chart asks them");
    let (since, mut entries) = (start.elapsed(), vec![]);
    for ex in Exchange::ALL {
        let began = Instant::now();
        for c in &recorded::CANDLES {
            if began.elapsed() > MOST_FOR_AN_EXCHANGE {
                let long = began.elapsed().as_secs();
                told.say(format!("{:<15} {:<18} NOT ASKED · this exchange already took {long} s", ex.id(), c.kind));
                continue;
            }
            let read = match ask(recorded::candles(net.clone(), ex, c)).await {
                Some(Ok(k)) => format!("{} candles, {} refused{}", k.bars.len(), k.a_reparer.len(), span(&k.bars)),
                Some(Err(e)) => refused(&e),
                None => FELL.to_string(),
            };
            let asked = net.take(ex.id(), c.kind);
            told.question(ex.id(), c.kind, &asked, &read);
            entries.extend(asked);
            pause(ex).await;
        }
        // what is taken is written as it goes
        save(&dir, name, about, since, &entries);
        told.save(&dir);
    }
    files.push(save(&dir, name, about, since, &entries));

    told.say(format!("\n== markets == ({} s)", start.elapsed().as_secs()));
    let (since, mut entries) = (start.elapsed(), vec![]);
    for ex in Exchange::ALL {
        let read = match ask(recorded::markets(net.clone(), ex)).await {
            Some(Ok(m)) => format!("{} markets", m.len()),
            Some(Err(e)) => refused(&e),
            None => FELL.to_string(),
        };
        let asked = net.take(ex.id(), "markets");
        told.question(ex.id(), "markets", &asked, &read);
        entries.extend(asked);
        pause(ex).await;
    }
    files.push(save(&dir, "answers-markets.txt", "markets and prices of every exchange", since, &entries));
    told.save(&dir);

    told.say(format!("\n== order books and last trades == ({} s)", start.elapsed().as_secs()));
    let (since, mut entries) = (start.elapsed(), vec![]);
    for c in Carnet::PERP.into_iter().chain(Carnet::SPOT) {
        let who = format!("{c:?}");
        let read = match ask(recorded::book(&net, c)).await {
            Some(Ok(b)) => format!("{} bids, {} asks", b.bids.len(), b.asks.len()),
            Some(Err(e)) => refused(&e),
            None => FELL.to_string(),
        };
        let asked = net.take(&who, "book");
        told.question(&who, "book", &asked, &read);
        entries.extend(asked);
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    for (ex, perp) in [(Exchange::Bitfinex, true), (Exchange::BitfinexSpot, false)] {
        pause(ex).await;
        let read = match ask(recorded::bitfinex_book(&net, perp)).await {
            Some(Ok(b)) => format!("{} bids, {} asks", b.bids.len(), b.asks.len()),
            Some(Err(e)) => refused(&e),
            None => FELL.to_string(),
        };
        let asked = net.take(ex.id(), "book");
        told.question(ex.id(), "book", &asked, &read);
        entries.extend(asked);
        pause(ex).await;
        let read = match ask(recorded::bitfinex_trades(&net, perp)).await {
            Some(Ok(t)) => format!("{} trades", t.len()),
            Some(Err(e)) => refused(&e),
            None => FELL.to_string(),
        };
        let asked = net.take(ex.id(), "trades");
        told.question(ex.id(), "trades", &asked, &read);
        entries.extend(asked);
    }
    let about = "order books of Argos Murs, Bitfinex's book and last trades";
    files.push(save(&dir, "answers-books.txt", about, since, &entries));
    told.save(&dir);

    told.say(format!("\n== funding == ({} s)", start.elapsed().as_secs()));
    let (since, mut entries) = (start.elapsed(), vec![]);
    for ex in Exchange::ALL {
        if argos_data::perp::url_funding(ex, recorded::COIN).is_none() {
            continue;
        }
        let read = match ask(recorded::funding(&net, ex)).await {
            Some(Some(rate)) => format!("rate for 8 hours {rate}"),
            Some(None) => "the app reads no rate".to_string(),
            None => FELL.to_string(),
        };
        let asked = net.take(ex.id(), "funding");
        told.question(ex.id(), "funding", &asked, &read);
        entries.extend(asked);
        pause(ex).await;
    }
    files.push(save(&dir, "answers-funding.txt", "current funding rates", since, &entries));
    told.save(&dir);

    told.say(format!("\n== live streams == ({} s)", start.elapsed().as_secs()));
    let since = start.elapsed();
    let entries = listen_to(&recorded::streams(), &mut told).await;
    files.push(save(&dir, "answers-live.txt", "first messages of the live streams", since, &entries));

    told.say("\n== files ==".into());
    for f in files {
        told.say(f);
    }
    told.say(format!("== the end == ({} s)", start.elapsed().as_secs()));
    told.save(&dir);
}

/// The listening for real, on a small local server: each stream is opened by the app's own connection and
/// subscribed to; its first messages come back as entries, and a stream that falls or cannot be opened says why.
#[tokio::test]
async fn the_messages_of_a_stream_come_back_as_entries() {
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::Message;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("a local port");
    let port = listener.local_addr().expect("its number").port();
    // a port nobody listens on: taken, then given back
    let nobody = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("a local port");
    let closed = nobody.local_addr().expect("its number").port();
    drop(nobody);
    tokio::spawn(async move {
        loop {
            let Ok((tcp, _)) = listener.accept().await else { return };
            tokio::spawn(async move {
                let Ok(mut ws) = tokio_tungstenite::accept_async(tcp).await else { return };
                // what the stream was sent to subscribe decides what it does
                let Some(Ok(Message::Text(first))) = ws.next().await else { return };
                if first.as_str() == "close" {
                    ws.close(None).await.ok();
                    return;
                }
                let Some(Ok(Message::Text(second))) = ws.next().await else { return };
                ws.send(Message::Text(format!("asked {first} + {second}"))).await.ok();
                for i in 0..20 {
                    ws.send(Message::Text(format!("message {i}"))).await.ok();
                }
                // stays open until the listener leaves
                while let Some(Ok(_)) = ws.next().await {}
            });
        }
    });
    let stream = |who: &str, port: u16, subscribe: &str| Stream {
        who: who.to_string(),
        kind: "live-tape",
        sub: LiveSub {
            url: format!("ws://127.0.0.1:{port}"),
            subscribe: subscribe.to_string(),
            ..Default::default()
        },
    };
    let streams = [stream("talks", port, "a\nb"), stream("closes", port, "close"), stream("nobody", closed, "a")];
    let start = Instant::now();
    let mut told = Told::default();
    let entries = listen_to(&streams, &mut told).await;
    assert!(
        start.elapsed() < LISTEN,
        "the listening ends as soon as every stream has said enough or has fallen"
    );

    let of = |who: &str| entries.iter().filter(|e| e.who == who).collect::<Vec<_>>();
    let mut expected = vec!["asked a + b".to_string()];
    expected.extend((0..MOST_MESSAGES - 1).map(|i| format!("message {i}")));
    assert_eq!(
        of("talks").iter().map(|e| e.answer.clone()).collect::<Vec<_>>(),
        expected.into_iter().map(Ok).collect::<Vec<_>>()
    );
    assert!(of("talks").iter().all(|e| e.kind == "live-tape"
        && e.asked
            == Asked::Live {
                url: format!("ws://127.0.0.1:{port}"),
                subscribe: "a | b".into()
            }));
    for who in ["closes", "nobody"] {
        assert!(
            matches!(of(who).as_slice(), [e] if e.answer.is_err()),
            "{who}: one entry that says why, {:?}",
            of(who)
        );
    }
    assert_eq!(told.0.len(), 3, "one line for each stream: {:?}", told.0);
    assert!(
        told.0[0].starts_with("talks           live-tape          12 answer(s) · ") && told.0[2].contains("FAILED · "),
        "{:?}",
        told.0
    );
    // through the file and back
    let text = recorded::file("a trial", 1, &entries);
    assert_eq!(recorded::read(&text), Ok(entries));
}

/// The listening itself, with no network: the two halves are driven as the app's connection drives them.
#[test]
fn a_stream_is_listened_to_until_enough_was_said() {
    // enough messages: the first three are kept, in order, and the third says stop
    let (heard, keep, go_on) = ears(3);
    assert!(go_on(LiveEvent::Connected), "connected: go on");
    let mut stopped_at = None;
    for (i, text) in ["one", "two", "three", "four"].into_iter().enumerate() {
        let event = keep(text).expect("every message goes on to the second half");
        if !go_on(event) && stopped_at.is_none() {
            stopped_at = Some(i);
        }
    }
    let h = heard.lock().expect("heard");
    assert_eq!(
        (h.messages.clone(), h.connected, h.lost.clone(), stopped_at),
        (vec!["one".to_string(), "two".into(), "three".into()], true, None, Some(2))
    );
    drop(h);

    // a stream that falls is not opened again, and what it said before is kept
    let (heard, keep, go_on) = ears(3);
    assert!(go_on(LiveEvent::Connected));
    assert!(go_on(keep("one").expect("goes on")));
    assert!(!go_on(LiveEvent::Disconnected("closed by the exchange".into())), "fallen: stop");
    assert!(!go_on(keep("late").expect("goes on")), "nothing is kept after the fall");
    let h = heard.lock().expect("heard");
    assert_eq!(
        (h.messages.clone(), h.lost.clone()),
        (vec!["one".to_string()], Some("closed by the exchange".to_string()))
    );
    drop(h);

    // too long a message is left out, the next ones are kept; a stop asked from outside is obeyed
    let (heard, keep, go_on) = ears(3);
    assert!(go_on(keep(&"x".repeat(MOST_BYTES + 1)).expect("goes on")));
    assert!(go_on(keep("short").expect("goes on")));
    heard.lock().expect("heard").stop = true;
    assert!(!go_on(keep("after the stop").expect("goes on")));
    assert_eq!(heard.lock().expect("heard").messages, ["short"]);

    // what is written of a stream: its messages with the wallets as zeros, or why there is none
    let s = Stream {
        who: "hl".into(),
        kind: "live-trades",
        sub: LiveSub {
            url: "wss://example.invalid/ws".into(),
            subscribe: "{\"a\":1}\n{\"b\":2}".into(),
            ..Default::default()
        },
    };
    let (heard, keep, _) = ears(3);
    keep(&format!(r#"{{"users":["0x{}","0x{}"]}}"#, "ab".repeat(20), "cd".repeat(20)));
    let entries = entries_of(&s, &heard);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].answer, Ok(format!(r#"{{"users":["0x{}","0x{}"]}}"#, "0".repeat(40), "0".repeat(40))));
    assert_eq!(
        entries[0].asked,
        Asked::Live {
            url: "wss://example.invalid/ws".into(),
            subscribe: r#"{"a":1} | {"b":2}"#.into()
        }
    );
    let (heard, _, go_on) = ears(3);
    assert_eq!(entries_of(&s, &heard)[0].answer, Err("no connection in 20 s".to_string()));
    let (heard2, _, go_on2) = ears(3);
    go_on2(LiveEvent::Connected);
    assert_eq!(entries_of(&s, &heard2)[0].answer, Err("connected, no message in 20 s".to_string()));
    go_on(LiveEvent::Disconnected("refused by 203.0.113.7".into()));
    assert_eq!(entries_of(&s, &heard)[0].answer, Err("refused by 0.0.0.0".to_string()));
    assert!(finished(&heard, 3) && most_of("live-book") < most_of("live-tape"));
    assert!(listened_for("live-liquidations") > listened_for("live-tape") && most_of("live-liquidations") < most_of("live-tape"));

    // a line of the summary: what the app says is not said twice when it only repeats why there was no answer
    let failed = [Entry {
        answer: Err("source: no route to the exchange".into()),
        ..entries[0].clone()
    }];
    let e = argos_core::ArgosError::Other("source: no route to the exchange".into());
    assert_eq!(
        one_line("hl", "candles-1m", &said(&failed), &refused(&e)),
        "hl              candles-1m         FAILED · source: no route to the exchange"
    );
    assert_eq!(
        one_line("hl", "candles-1m", &said(&entries), &refused(&e)),
        "hl              candles-1m         1 answer(s) · 101 bytes · the app says: source: no route to the exchange"
    );
    assert_eq!(
        one_line("hl", "candles-1m", &said(&entries), "60 candles, 0 refused"),
        "hl              candles-1m         1 answer(s) · 101 bytes · 60 candles, 0 refused"
    );
    // (two wallets of 42 characters between quotes, and the 13 characters around them)
    assert_eq!(said(&entries), "1 answer(s) · 101 bytes");
}
