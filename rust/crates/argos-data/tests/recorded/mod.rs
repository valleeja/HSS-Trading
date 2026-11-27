//! 📼 The answers of the exchanges kept in a text file: how they are written and read back, how they are taken
//! while the app's own code asks its questions, and how they are given again to that same code in place of the
//! network. And the questions themselves: one list, asked the same way the day the answers are taken and every time
//! a test reads them.
//!
//! The file is made of entries. An entry is one header line, one count line and the answer itself, byte for byte:
//!
//! ```text
//! @ binance candles-1m GET https://fapi.binance.com/fapi/v1/klines?symbol=BTCUSDT&interval=1m&limit=60
//! ok 13
//! [[1700000000]]
//! ```
//!
//! The header says who answered (an exchange's id, or the name of a book), which kind of data it is and what was
//! asked: `GET address`, `POST address body` or `LIVE address subscription` (a message of a live stream). The
//! count line says `ok` or `failed` (the answer is then the reason) and the number of bytes that follow. A line that
//! starts with `#` between two entries is a note; the note `# taken 1700000000000` says when the answers were taken.

// each test file uses its own part of this one
#![allow(dead_code)]

use argos_core::feed::{Feed, Market};
use argos_core::murs::VenueBook;
use argos_core::{ArgosError, Result, Timeframe};
use argos_data::exchanges::Request;
use argos_data::http::Http;
use argos_data::live::LiveSub;
use argos_data::murs::Carnet;
use argos_data::{Checked, Exchange, ExchangeFeed};
use std::cell::Cell;
use std::sync::{Arc, Mutex};

/// An answer, or why there was none.
pub type Answer = std::result::Result<String, String>;

/// On one line (a header is one line).
fn one(s: &str) -> String {
    s.replace(['\n', '\r'], " ")
}

/// What was asked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Asked {
    Get(String),
    Post {
        url: String,
        body: String,
    },
    /// A live stream: its address and what is sent to subscribe (several messages: joined by « | »).
    Live {
        url: String,
        subscribe: String,
    },
}

impl Asked {
    pub fn get(url: &str) -> Self {
        Asked::Get(one(url))
    }

    pub fn post(url: &str, body: &str) -> Self {
        Asked::Post {
            url: one(url),
            body: one(body),
        }
    }

    pub fn request(r: &Request) -> Self {
        match r {
            Request::Get(url) => Asked::get(url),
            Request::PostJson { url, body } => Asked::post(url, body),
        }
    }

    pub fn live(sub: &LiveSub) -> Self {
        Asked::Live {
            url: one(&sub.url),
            subscribe: sub.subscribe.lines().filter(|l| !l.is_empty()).collect::<Vec<_>>().join(" | "),
        }
    }

    /// As the header writes it.
    fn line(&self) -> String {
        match self {
            Asked::Get(url) => format!("GET {url}"),
            Asked::Post { url, body } => format!("POST {url} {body}"),
            Asked::Live { url, subscribe } => format!("LIVE {url} {subscribe}"),
        }
    }
}

/// One answer: who gave it, which kind of data, what was asked, and the answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub who: String,
    pub kind: String,
    pub asked: Asked,
    pub answer: Answer,
}

/// The entries as the text of a file.
pub fn write(entries: &[Entry]) -> String {
    let mut out = String::new();
    for e in entries {
        let word = |s: &str| s.replace(char::is_whitespace, "-");
        let (said, text) = match &e.answer {
            Ok(t) => ("ok", t),
            Err(t) => ("failed", t),
        };
        out.push_str(&format!(
            "@ {} {} {}\n{said} {}\n{text}\n",
            word(&e.who),
            word(&e.kind),
            e.asked.line(),
            text.len()
        ));
    }
    out
}

/// A whole file: what it holds, when the answers were taken (milliseconds), and the entries.
pub fn file(about: &str, taken_ms: i64, entries: &[Entry]) -> String {
    format!("# {}\n# taken {taken_ms}\n{}", one(about), write(entries))
}

/// When the answers of a file were taken (its note `# taken`), in milliseconds.
pub fn taken(text: &str) -> Option<i64> {
    text.lines()
        .take_while(|l| l.starts_with('#'))
        .find_map(|l| l.strip_prefix("# taken ")?.split(' ').next()?.parse().ok())
}

/// The entries of a file's text; `Err`: where it stops being readable.
pub fn read(text: &str) -> std::result::Result<Vec<Entry>, String> {
    let mut entries = vec![];
    let mut rest = text;
    while !rest.is_empty() {
        let at = text.len() - rest.len();
        let (line, after) = rest.split_once('\n').unwrap_or((rest, ""));
        if line.is_empty() || line.starts_with('#') {
            rest = after;
            continue;
        }
        let bad = |what: &str| format!("byte {at}: {what}: « {} »", line.chars().take(80).collect::<String>());
        let mut parts = line.splitn(5, ' ');
        let (Some("@"), Some(who), Some(kind), Some(verb), Some(tail)) = (parts.next(), parts.next(), parts.next(), parts.next(), parts.next()) else {
            return Err(bad("not the header of an entry"));
        };
        // nothing after the address: an empty body, an empty subscription
        let (url, more) = tail.split_once(' ').unwrap_or((tail, ""));
        let asked = match verb {
            "GET" if more.is_empty() => Asked::Get(url.to_string()),
            "POST" => Asked::Post {
                url: url.to_string(),
                body: more.to_string(),
            },
            "LIVE" => Asked::Live {
                url: url.to_string(),
                subscribe: more.to_string(),
            },
            _ => return Err(bad("not GET address, POST address body or LIVE address subscription")),
        };
        // (the last end of line of a file may be lost)
        let (count, after) = match after.split_once('\n') {
            Some(x) => x,
            None if !after.is_empty() => (after, ""),
            None => return Err(bad("no count line")),
        };
        let (said, n) = count.split_once(' ').ok_or_else(|| bad("the count line is not « ok N » or « failed N »"))?;
        let n: usize = n.parse().map_err(|_| bad("the count is not a number"))?;
        // the answer, then the end of its line (or of the file)
        if after.len() < n || !after.is_char_boundary(n) || after.as_bytes().get(n).is_some_and(|c| *c != b'\n') {
            return Err(bad("the answer does not have the length its count says"));
        }
        let answer = match said {
            "ok" => Ok(after[..n].to_string()),
            "failed" => Err(after[..n].to_string()),
            _ => return Err(bad("the count line is not « ok N » or « failed N »")),
        };
        entries.push(Entry {
            who: who.to_string(),
            kind: kind.to_string(),
            asked,
            answer,
        });
        rest = after.get(n + 1..).unwrap_or("");
    }
    Ok(entries)
}

/// The same text with every wallet address and transaction hash written as zeros (« 0x » and forty hexadecimal
/// digits or more): a public trade names the wallets of two people, and no test reads them.
pub fn without_wallets(text: &str) -> String {
    let b = text.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'0' && b.get(i + 1) == Some(&b'x') {
            let n = b[i + 2..].iter().take_while(|c| c.is_ascii_hexdigit()).count();
            if n >= 40 {
                out.extend_from_slice(b"0x");
                out.resize(out.len() + n, b'0');
                i += 2 + n;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    // only ASCII bytes were replaced by ASCII bytes
    String::from_utf8(out).unwrap_or_else(|_| text.to_string())
}

/// The same text with every network address of a computer written as zeros (« 203.0.113.7 » becomes « 0.0.0.0 », an
/// IPv6 address becomes « :: »): an exchange that refuses a request may name the computer that asked.
pub fn without_addresses(text: &str) -> String {
    // four numbers under 256 between three points (a point that ends a sentence is not part of it)
    let v4 = |word: &str| {
        let core = word.trim_matches('.');
        let parts: Vec<&str> = core.split('.').collect();
        (parts.len() == 4 && parts.iter().all(|p| p.len() <= 3 && p.parse::<u8>().is_ok())).then(|| word.replacen(core, "0.0.0.0", 1))
    };
    // hexadecimal groups between colons: five groups or more, or a « :: » (an hour, « 19:26:26 », is neither)
    let v6 = |word: &str| {
        let colons = word.matches(':').count();
        (colons >= 4 || (colons >= 2 && word.contains("::"))).then(|| "::".to_string())
    };
    let digits = rewritten(text, |c| c.is_ascii_digit() || c == '.', v4);
    rewritten(&digits, |c| c.is_ascii_hexdigit() || c == ':', v6)
}

/// What is kept of an answer, or of the reason there is none: no wallet, no network address. (No number of a
/// market is written as four groups between three points, nor as five between colons: its figures stay as they came.)
pub fn kept(text: &str) -> String {
    without_addresses(&without_wallets(text))
}

/// `text` with each run of the characters `part` accepts given to `new`: `Some` replaces the run.
fn rewritten(text: &str, part: impl Fn(char) -> bool, new: impl Fn(&str) -> Option<String>) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while !rest.is_empty() {
        let n = rest.find(&part).unwrap_or(rest.len());
        out.push_str(&rest[..n]);
        rest = &rest[n..];
        let n = rest.find(|c| !part(c)).unwrap_or(rest.len());
        out.push_str(&new(&rest[..n]).unwrap_or_else(|| rest[..n].to_string()));
        rest = &rest[n..];
    }
    out
}

type Seen = Arc<Mutex<Vec<(Asked, Answer)>>>;

/// 🎙 A client that lets the app's code ask the real one, and keeps every question with its answer.
#[derive(Clone)]
pub struct Recording<H> {
    net: H,
    seen: Seen,
}

impl<H> Recording<H> {
    pub fn new(net: H) -> Self {
        Self { net, seen: Seen::default() }
    }

    /// What was asked since the last time, in order, filed under `who` and `kind`. Wallets and network addresses
    /// are written as zeros, in an answer as in the reason of a failure.
    pub fn take(&self, who: &str, kind: &str) -> Vec<Entry> {
        let seen = self.seen.lock().map(|mut s| std::mem::take(&mut *s)).unwrap_or_default();
        seen.into_iter()
            .map(|(asked, answer)| Entry {
                who: who.to_string(),
                kind: kind.to_string(),
                asked,
                answer: answer.map(|t| kept(&t)).map_err(|t| kept(&t)),
            })
            .collect()
    }

    fn keep(&self, asked: Asked, answer: &Result<String>) {
        if let Ok(mut s) = self.seen.lock() {
            s.push((asked, answer.clone().map_err(|e| e.to_string())));
        }
    }
}

impl<H: Http> Http for Recording<H> {
    async fn get(&self, url: &str) -> Result<String> {
        let answer = self.net.get(url).await;
        self.keep(Asked::get(url), &answer);
        answer
    }

    async fn post_json(&self, url: &str, body: &str) -> Result<String> {
        let answer = self.net.post_json(url, body).await;
        self.keep(Asked::post(url, body), &answer);
        answer
    }

    async fn wait(&self, ms: u64) {
        self.net.wait(ms).await;
    }
}

/// 📼 A client that never goes on the network: it gives the kept answers again, each one once, to whoever asks
/// exactly what was asked the day they were taken. A question with no kept answer fails, and is noted.
#[derive(Clone, Default)]
pub struct Replay {
    kept: Arc<Mutex<Vec<Entry>>>,
    unknown: Arc<Mutex<Vec<Asked>>>,
}

impl Replay {
    pub fn new(entries: &[Entry]) -> Self {
        Self {
            kept: Arc::new(Mutex::new(entries.iter().filter(|e| !matches!(e.asked, Asked::Live { .. })).cloned().collect())),
            unknown: Arc::default(),
        }
    }

    /// The questions that had no kept answer.
    pub fn unknown(&self) -> Vec<Asked> {
        self.unknown.lock().map(|u| u.clone()).unwrap_or_default()
    }

    /// The kept answers nobody asked for.
    pub fn left(&self) -> Vec<Entry> {
        self.kept.lock().map(|k| k.clone()).unwrap_or_default()
    }

    fn answer(&self, asked: Asked) -> Result<String> {
        let found = self
            .kept
            .lock()
            .ok()
            .and_then(|mut k| k.iter().position(|e| e.asked == asked).map(|i| k.remove(i)));
        match found {
            // a failure is told again in the words it was told (they hold the code a retry looks for)
            Some(e) => e.answer.map_err(ArgosError::Other),
            None => {
                let why = format!("no kept answer for {asked:?}");
                if let Ok(mut u) = self.unknown.lock() {
                    u.push(asked);
                }
                Err(ArgosError::Feed(why))
            }
        }
    }
}

impl Http for Replay {
    async fn get(&self, url: &str) -> Result<String> {
        self.answer(Asked::get(url))
    }

    async fn post_json(&self, url: &str, body: &str) -> Result<String> {
        self.answer(Asked::post(url, body))
    }
}

thread_local! {
    /// The moment the answers are or were taken (milliseconds), for the code that asks « what time is it? ».
    static TAKEN: Cell<i64> = const { Cell::new(0) };
}

/// Says when the answers are taken (the day they are taken: now) or were taken (a test: the note of the file), for
/// the questions asked from this thread.
pub fn set_clock(ms: i64) {
    TAKEN.with(|t| t.set(ms));
}

/// The clock given to the app's code: the moment of the answers, never the computer's own.
pub fn clock() -> i64 {
    TAKEN.with(Cell::get)
}

/// The market asked everywhere, and a market with a small price.
pub const COIN: &str = "BTC";
pub const SMALL: &str = "DOGE";

/// Which candles of a market.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Window {
    /// The newest ones (the last one is still moving).
    Newest(usize),
    /// The last ones closed before the hour in progress.
    BeforeTheHour(usize),
    /// Every one from yesterday 00:00 UTC to the hour in progress.
    SinceYesterday,
    /// The last ones closed before today 00:00 UTC.
    BeforeToday(usize),
    /// The last ones closed before this week began (Monday 00:00 UTC).
    BeforeThisWeek(usize),
}

/// One question about candles.
#[derive(Debug, Clone, Copy)]
pub struct Candles {
    pub kind: &'static str,
    pub coin: &'static str,
    pub tf: Timeframe,
    pub window: Window,
}

/// The candles asked to every exchange. Together they can be checked against one another with no answer written
/// by hand: the last closed hour and its sixty minutes, its twelve candles of five minutes, yesterday and its
/// twenty-four hours, its six candles of four hours, its two halves of twelve hours; last week and its seven days.
pub const CANDLES: [Candles; 10] = [
    Candles {
        kind: "candles-1m-newest",
        coin: COIN,
        tf: Timeframe::M1,
        window: Window::Newest(5),
    },
    Candles {
        kind: "candles-1m",
        coin: COIN,
        tf: Timeframe::M1,
        window: Window::BeforeTheHour(60),
    },
    Candles {
        kind: "candles-5m",
        coin: COIN,
        tf: Timeframe::M5,
        window: Window::BeforeTheHour(12),
    },
    Candles {
        kind: "candles-1h",
        coin: COIN,
        tf: Timeframe::H1,
        window: Window::SinceYesterday,
    },
    Candles {
        kind: "candles-4h",
        coin: COIN,
        tf: Timeframe::H4,
        window: Window::BeforeToday(6),
    },
    Candles {
        kind: "candles-1d",
        coin: COIN,
        tf: Timeframe::D1,
        window: Window::BeforeToday(2),
    },
    Candles {
        kind: "candles-1m-small",
        coin: SMALL,
        tf: Timeframe::M1,
        window: Window::BeforeTheHour(30),
    },
    Candles {
        kind: "candles-12h",
        coin: COIN,
        tf: Timeframe::H12,
        window: Window::BeforeToday(2),
    },
    Candles {
        kind: "candles-1d-week",
        coin: COIN,
        tf: Timeframe::D1,
        window: Window::BeforeThisWeek(7),
    },
    Candles {
        kind: "candles-1w",
        coin: COIN,
        tf: Timeframe::W1,
        window: Window::BeforeThisWeek(2),
    },
];

/// The start of the hour in progress and of today (UTC), in seconds, at the moment of the answers.
pub fn hour_and_day() -> (i64, i64) {
    let now = clock() / 1000;
    (now - now.rem_euclid(3600), now - now.rem_euclid(86_400))
}

/// The start of this week (Monday 00:00 UTC), in seconds, at the moment of the answers.
pub fn this_week() -> i64 {
    argos_core::bucket_start(clock() / 1000, Timeframe::W1.seconds())
}

/// Asks `ex` the candles `c` the way a chart does (pages, checks, regrouping), at the moment of the answers.
pub async fn candles<H: Http>(http: H, ex: Exchange, c: &Candles) -> Result<Checked> {
    let (hour, day) = hour_and_day();
    let feed = ExchangeFeed::with_clock(ex, http, clock);
    match c.window {
        Window::Newest(n) => feed.candles_checked(c.coin, c.tf, n).await,
        Window::BeforeTheHour(n) => feed.older_checked(c.coin, c.tf, hour, n).await,
        Window::SinceYesterday => feed.older_checked(c.coin, c.tf, hour, 24 + ((hour - day) / 3600) as usize).await,
        Window::BeforeToday(n) => feed.older_checked(c.coin, c.tf, day, n).await,
        Window::BeforeThisWeek(n) => feed.older_checked(c.coin, c.tf, this_week(), n).await,
    }
}

/// Asks `ex` its markets with their prices (the list a chart opens).
pub async fn markets<H: Http>(http: H, ex: Exchange) -> Result<Vec<Market>> {
    ExchangeFeed::with_clock(ex, http, clock).markets().await
}

/// Asks one order book of Argos Murs (Hyperliquid: its finest grouping).
pub async fn book<H: Http>(http: &H, c: Carnet) -> Result<VenueBook> {
    argos_data::murs::fetch(http, c, COIN, 0.0, 0.0).await
}

/// Asks the current funding rate of `ex`, brought to 8 hours (`None`: no public rate read, or no answer).
pub async fn funding<H: Http>(http: &H, ex: Exchange) -> Option<f64> {
    argos_data::perp::taux_8h(http, ex, COIN).await
}

/// Sends a request of the app.
pub async fn send<H: Http>(http: &H, r: Request) -> Result<String> {
    match r {
        Request::Get(url) => http.get(&url).await,
        Request::PostJson { url, body } => http.post_json(&url, &body).await,
    }
}

/// Asks Bitfinex its order book (the Book module reads it by REST), perps or spot.
pub async fn bitfinex_book<H: Http>(http: &H, perp: bool) -> Result<argos_data::book::Book> {
    let body = send(http, argos_data::exchanges::bitfinex::book_request(COIN, perp)).await?;
    argos_data::exchanges::bitfinex::parse_book(&body, COIN, clock())
}

/// Asks Bitfinex its last trades (the Trades module reads them by REST), perps or spot.
pub async fn bitfinex_trades<H: Http>(http: &H, perp: bool) -> Result<Vec<argos_data::whales::Whale>> {
    let body = send(http, argos_data::exchanges::bitfinex::trades_request(COIN, perp)).await?;
    argos_data::exchanges::bitfinex::parse_trades(&body, COIN)
}

/// A live stream the app listens to: who sends it, which kind of data, where it is and what is sent to subscribe.
#[derive(Debug, Clone, PartialEq)]
pub struct Stream {
    pub who: String,
    pub kind: &'static str,
    pub sub: LiveSub,
}

/// The live candles of twelve hours, a day and a week of the exchanges that name them by their hour of UTC
/// (« 1Dutc »): their names are asked as the app asks them.
pub const LIVE_UTC: [Exchange; 4] = [Exchange::Okx, Exchange::OkxSpot, Exchange::Bitget, Exchange::BitgetSpot];

/// Every live stream listened to the day the answers are taken, each one once: the candles of a chart in one
/// minute (and in twelve hours, a day and a week, `LIVE_UTC`), the trades of a chart in seconds, the trades of the
/// Trades module, Hyperliquid's book and figures, Binance's depth and the liquidations. (KuCoin's stream needs a
/// ticket asked first: not taken here.)
pub fn streams() -> Vec<Stream> {
    let coins = [COIN.to_string()];
    let mut all: Vec<Stream> = vec![];
    let mut add = |who: &str, kind: &'static str, sub: LiveSub| {
        if !all.iter().any(|s| s.sub == sub) {
            all.push(Stream {
                who: who.to_string(),
                kind,
                sub,
            });
        }
    };
    for ex in Exchange::ALL {
        if let Some(sub) = argos_data::live::subscription(ex, COIN, Timeframe::M1) {
            add(ex.id(), "live-candles", sub);
        }
    }
    for ex in LIVE_UTC {
        for (kind, tf) in [
            ("live-candles-12h", Timeframe::H12),
            ("live-candles-1d", Timeframe::D1),
            ("live-candles-1w", Timeframe::W1),
        ] {
            if let Some(sub) = argos_data::live::subscription(ex, COIN, tf) {
                add(ex.id(), kind, sub);
            }
        }
    }
    for ex in Exchange::ALL {
        if let Some(sub) = argos_data::live::trades_subscription(ex, COIN) {
            add(ex.id(), "live-trades", sub);
        }
    }
    for name in argos_data::whales::EXCHANGES {
        if let Some(sub) = argos_data::whales::sub_of(name, &coins) {
            add(name, "live-tape", sub);
        }
    }
    add("hl", "live-book", argos_data::book::sub(Some(COIN), None, &[]));
    add("hl", "live-figures", argos_data::watchlist::ctx_sub(&coins));
    add("binance", "live-depth", argos_data::murs::flux_binance(COIN));
    add("binance", "live-liquidations", argos_data::liq::sub());
    add("bybit", "live-liquidations", argos_data::liq::bybit_sub(argos_data::liq::BYBIT_COINS));
    all
}
