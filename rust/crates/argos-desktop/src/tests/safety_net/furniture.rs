//! 🕸 The data of the safety net: what the exchanges and the other sources would send, all of it made up.
//!
//! The scenes are drawn on an app in use, not on an empty one: a Hyperliquid chart, its store (in memory) with the
//! lists of a first launch, and the answers of the network, delivered the way the app receives them
//! (`App::on_user_event`): prices, order book, trades, funding, liquidations, big transactions, calendar, news,
//! markets. Nothing here is real or anyone's: no price, no address, no account.

use super::universe;
use crate::*;
use argos_data::whales::Whale;
use std::cell::RefCell;
use std::collections::BTreeSet;

thread_local! {
    /// The kinds of messages the app of this scene received (a scene has its own thread).
    static ARRIVED: RefCell<BTreeSet<&'static str>> = const { RefCell::new(BTreeSet::new()) };
}

/// A message arrives: the app receives it as it does from its window's event loop (`App::on_user_event`), and its
/// kind is noted for the scene.
pub(super) fn deliver(a: &mut App, answer: UserEvent) {
    ARRIVED.with(|kinds| kinds.borrow_mut().insert(universe::answer_name(&answer)));
    a.on_user_event(answer);
}

/// The ids of the kinds of messages the app of this scene received (« answer:Wallet »).
pub(super) fn arrived() -> Vec<String> {
    ARRIVED.with(|kinds| kinds.borrow().iter().map(|kind| universe::id("answer", kind)).collect())
}

/// The market of the chart.
pub(super) const COIN: &str = "BTC";

/// A made-up market: its name, its price, its price 24 h before, its volume (M$).
const MARKETS: [(&str, f64, f64, f64); 14] = [
    ("BTC", 222.0, 213.5, 4_200.0),
    ("ETH", 88.25, 90.1, 2_100.0),
    ("SOL", 15.5, 14.2, 910.0),
    ("HYPE", 4.125, 4.0, 640.0),
    ("PAXG", 301.0, 300.4, 55.0),
    ("XRP", 0.5125, 0.53, 480.0),
    ("DOGE", 0.0725, 0.07, 350.0),
    ("BNB", 61.5, 61.5, 220.0),
    ("AVAX", 3.25, 3.4, 130.0),
    ("LINK", 1.75, 1.6, 120.0),
    ("SUI", 0.3125, 0.29, 95.0),
    ("LTC", 9.5, 9.75, 60.0),
    ("xyz:NVDA", 14.25, 13.9, 75.0),
    ("xyz:GOLD", 30.5, 30.25, 40.0),
];

/// The app in use, before its window is drawn: on a Hyperliquid market, with its store and its lists.
pub(super) fn in_use(a: &mut App) {
    let exchange = Exchange::Hyperliquid;
    a.title = exchange.chart_title(COIN, &a.chart.interval.short());
    a.chart.source = Source::Live { exchange, base: COIN.into() };
    a.store = Some(Store::in_memory().expect("a store in memory"));
    a.load_lists();
}

/// A trade or a big transaction, `ago` seconds before the frozen clock.
pub(super) fn trade(coin: &str, ago: i64, price: f64, size: f64, buy: bool, src: &'static str) -> Whale {
    Whale {
        coin: coin.into(),
        time_ms: (now_s() - ago) * 1000,
        price,
        size,
        buy,
        src,
        wallet: String::new(),
        id: None,
    }
}

/// The answers of the network arrive: the app receives them as it does from its window's event loop. (They come
/// before a scene opens its screen: today a menu left open closes when the 24 h figures of an exchange arrive,
/// `UserEvent::Tickers`.)
pub(super) fn arrive(a: &mut App) {
    // (what the app notes when it asks: the market whose order book and fundings it waits for)
    a.feeds.flow_coin = a.book_coin();
    a.feeds.funding_at = Some((COIN.into(), web_time::Instant::now()));
    // (and what it copies from its settings each time it asks, `App::refresh_eco`: the ⚙ of the Actualités shows the
    // sources, the language and « ⚡ sur la chart » as they are kept)
    let (off, language, lightning) = (
        a.watch.prefs.news_sources_off.clone(),
        a.watch.prefs.news_langue.min(2),
        a.chart.settings.econ_news,
    );
    (a.watch.eco.sources_off, a.watch.eco.langue, a.watch.eco.eclairs) = (off, language, lightning);
    for answer in answers() {
        deliver(a, answer);
    }
}

/// What the network answers, in the order it arrives.
fn answers() -> Vec<UserEvent> {
    use argos_data::watchlist::{Quote, Section};
    let now = now_s();
    let mut answers = vec![];
    // 📡 the prices of the watchlist (Hyperliquid's sections) and of two other exchanges
    let quote = |(coin, price, prev, volume): &(&str, f64, f64, f64)| Quote {
        coin: coin.to_string(),
        mark: format!("{price}"),
        prev: *prev,
        volume: volume * 1e6,
        growth: coin.starts_with("xyz:"),
        oracle: *price,
        oi: volume * 1e4,
        funding: 0.0000125,
    };
    let (crypto, hip3): (Vec<_>, Vec<_>) = MARKETS.iter().partition(|m| !m.0.contains(':'));
    let section = |name: &str, markets: &[&(&str, f64, f64, f64)]| Section {
        name: name.into(),
        quotes: markets.iter().map(|m| quote(m)).collect(),
    };
    let sections = vec![section("Crypto", &crypto), section("HIP-3", &hip3)];
    answers.push(UserEvent::Quotes(sections, MARKETS.iter().map(quote).collect()));
    for (k, exchange) in [Exchange::Binance, Exchange::Okx].into_iter().enumerate() {
        let market = |(coin, price, prev, _): &&(&str, f64, f64, f64)| argos_core::feed::Market {
            base: coin.to_string(),
            price: price + k as f64 * 0.25,
            prev: *prev,
        };
        answers.push(UserEvent::Prices(exchange, crypto.iter().map(market).collect()));
        // (never the same figures on two exchanges: the lists sorted by them have one order only)
        let ticker = |(coin, price, prev, volume): &&(&str, f64, f64, f64)| argos_data::tickers::Ticker {
            ex: exchange,
            base: coin.to_string(),
            price: price + k as f64 * 0.25,
            prev: *prev,
            vol_usd: volume * 1e6 / (1 + k) as f64,
        };
        answers.push(UserEvent::Tickers(exchange, Ok(crypto.iter().map(ticker).collect())));
    }
    // 📇 Hyperliquid's catalogue (what the search of symbols finds)
    let instrument = |(coin, ..): &(&str, f64, f64, f64)| argos_core::instrument::Instrument {
        exchange: Exchange::Hyperliquid.id().into(),
        symbol: coin.to_string(),
        base: coin.to_string(),
        quote: "USDC".into(),
        kind: argos_core::instrument::MarketKind::Perp,
        group: if coin.contains(':') { "HIP-3" } else { "Crypto" }.into(),
        price_decimals: 2,
        size_decimals: 3,
        growth: coin.contains(':'),
        active: true,
    };
    answers.push(UserEvent::Catalog(Exchange::Hyperliquid, MARKETS.iter().map(instrument).collect()));
    // 🌐 the crypto market: the coin of the chart, fear and greed, the capitalisation
    let market = argos_data::market::MarketInfo {
        coin: Some(argos_data::market::Coin {
            id: "bitcoin".into(),
            name: "Bitcoin".into(),
            symbol: COIN.into(),
            rank: Some(1),
            price: 222.0,
            chg: [Some(0.4), Some(3.98), Some(-2.5), Some(12.0), Some(-8.0)],
            market_cap: 4.4e9,
            volume: 4.2e9,
            fdv: Some(4.66e9),
            low_24h: Some(211.0),
            high_24h: Some(224.0),
            ath: Some(290.0),
            ath_chg: Some(-23.4),
        }),
        fng: Some(argos_data::market::Fng {
            value: 62,
            label: "Greed".into(),
            yesterday: Some(58),
            week_ago: Some(41),
        }),
        global: Some(argos_data::market::Global {
            total: 8.25e9,
            total_chg_24h: Some(1.75),
            btc_d: Some(53.5),
            eth_d: Some(17.25),
            usdt_d: Some(5.5),
            usdc_d: Some(2.25),
            volume: 9.5e8,
        }),
        alt_season: Some(37),
        error: None,
        cmc_cle: false,
        cmc: None,
    };
    answers.push(UserEvent::Market(Box::new(market)));
    // 📅 the calendar of the week and 📰 the news
    let event = |title: &str, country: &str, impact: &str, hours: i64, numbers: [&str; 3]| argos_data::eco::Event {
        title: title.into(),
        country: country.into(),
        impact: impact.into(),
        ts: now + hours * 3600,
        forecast: numbers[0].into(),
        previous: numbers[1].into(),
        actual: numbers[2].into(),
    };
    answers.push(UserEvent::Agenda(Ok(vec![
        event("Retail Sales m/m", "USD", "Medium", -20, ["0.3%", "0.1%", "0.4%"]),
        event("CPI m/m", "USD", "High", 3, ["0.3%", "0.2%", ""]),
        event("BOC Rate Statement", "CAD", "High", 5, ["", "", ""]),
        event("German ZEW", "EUR", "Low", 26, ["12.0", "10.3", ""]),
        event("Unemployment Claims", "USD", "Medium", 50, ["225K", "219K", ""]),
        event("BOJ Outlook Report", "JPY", "High", 74, ["", "", ""]),
    ])));
    let news = |title: &str, source: &str, minutes: i64| argos_data::eco::News {
        title: title.into(),
        link: format!("https://example.invalid/{minutes}"),
        ts: now - minutes * 60,
        source: source.into(),
        lang: if source.ends_with(" FR") { "fr" } else { "en" }.into(),
    };
    answers.push(UserEvent::News(Ok(vec![
        news("A made-up headline about rates, long enough to be cut at the edge of its module", "Essai", 4),
        news("Another made-up headline", "Essai", 38),
        news("Markets close higher (made up)", "Exemple", 190),
    ])));
    // 🌍 the macro markets
    let tile = |name: &str, price: f64, chg_pct: f64| argos_data::world::Tile {
        name: name.into(),
        price,
        chg_pct,
        spark: (0..24).map(|k| price * (1.0 + ((k * 7) % 11) as f64 / 400.0 - 0.0125)).collect(),
    };
    answers.push(UserEvent::World(Ok(vec![
        tile("S&P 500", 512.25, 0.75),
        tile("Nasdaq", 181.5, 1.25),
        tile("DXY", 10.375, -0.25),
        tile("Gold", 30.5, 0.5),
        tile("Oil", 7.75, -1.5),
        tile("US 10Y", 4.125, 0.0),
    ])));
    // 📖 the order book of the chart's market and ⚡ its trades
    let level = |px: f64, k: usize| argos_data::book::Level {
        px,
        sz: 0.5 + ((k * 5) % 7) as f64 * 1.5,
        n: 1 + (k % 4) as u32,
    };
    answers.push(UserEvent::Flow(LiveEvent::Book(argos_data::book::Book {
        coin: COIN.into(),
        time_ms: now * 1000,
        bids: (0..20).map(|k| level(221.75 - k as f64 * 0.25, k)).collect(),
        asks: (0..20).map(|k| level(222.25 + k as f64 * 0.25, k + 3)).collect(),
    })));
    let tape = (0..24).map(|k| trade(COIN, 2 + k * 9, 222.0 - (k % 5) as f64 * 0.25, 0.25 + (k % 4) as f64, k % 3 != 0, "Hyperliquid"));
    answers.push(UserEvent::Flow(LiveEvent::Tape(tape.collect())));
    // 💸 its fundings of the last 24 h
    let funding = |k: i64| argos_data::book::Funding {
        time_ms: (now - k * 3600) * 1000,
        rate: [0.0000125, 0.00001, -0.0000075, 0.00002][(k % 4) as usize],
    };
    answers.push(UserEvent::FundingHist(COIN.into(), Ok((0..24).rev().map(funding).collect())));
    // 💥 the liquidations and 🐋 the big transactions (under 1 M$ on the chart's market: no banner comes with them)
    let liq = |coin: &str, ago: i64, price: f64, qty: f64, long: bool, venue: &'static str| argos_data::liq::Liq {
        coin: coin.into(),
        time_ms: (now - ago) * 1000,
        price,
        qty,
        long,
        venue,
    };
    answers.push(UserEvent::Liq(LiveEvent::Liqs(vec![
        liq(COIN, 40, 221.5, 900.0, true, "Binance"),
        liq("ETH", 95, 88.0, 2_500.0, false, "Bybit"),
        liq(COIN, 310, 219.0, 4_000.0, true, "Okx"),
        liq("SOL", 1_200, 15.25, 30_000.0, false, "Binance"),
    ])));
    answers.push(UserEvent::Whales(LiveEvent::Whales(vec![
        trade(COIN, 30, 222.0, 1_500.0, true, "Hyperliquid"),
        trade("ETH", 200, 88.25, 9_000.0, false, "Binance"),
        trade(COIN, 900, 220.5, 2_600.0, false, "Okx"),
        trade("SOL", 2_400, 15.5, 80_000.0, true, "Bybit"),
        trade(COIN, 5_000, 218.0, 4_000.0, true, "Binance"),
    ])));
    // 📡 the scanner
    let line = |coin: &str, dir: i8, rr: Option<f64>| argos_render::dock::ScanLine {
        coin: coin.into(),
        dir,
        rr,
        ex: "Hyperliquid".into(),
        also: vec!["Binance".into()],
    };
    answers.push(UserEvent::Scan(
        vec![line("ETH", 1, Some(2.5)), line("SOL", -1, Some(1.75)), line("HYPE", 1, None)],
        vec!["PAXG".into()],
    ));
    answers
}
