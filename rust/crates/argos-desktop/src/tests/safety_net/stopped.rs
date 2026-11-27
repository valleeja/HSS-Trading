//! 🕸 A source that stops (the data reliability tests, test 5): what the screen shows when a source of the market's
//! data stops answering, and what the lists hold when it is back.
//!
//! The rule: a source that stops is shown stopped; data that no longer move are never left on screen as if they were
//! live. And after a reconnection nothing is missing and nothing is there twice.
//!
//! Each line of [`sources`] is one source on a screen that shows its data, already arrived (a scene of the net). The
//! screen is drawn, the app receives what it receives when that source stops (its stream cut, its reading refused:
//! the messages of [`answers`](super::answers), made up), and the screen is drawn again; where the source tells the
//! app it is back, it comes back and the screen is drawn a third time. Nothing else happens between the images (the
//! clock is frozen, nothing arrives): a pixel that changes is what the app says of the stop.
//!
//! - **The stop shows**: the second image is not the first one.
//! - **The sign leaves with the stop**: where the source tells it is back, the third image is the first one again.
//! - **What was seen** ([`SEEN`]): the sources whose stop changes nothing on the screen today: their last data stay
//!   as they were, with no sign. They are written down, not excused: the test fails on the day one of them shows its
//!   stop, so that its line leaves the list and the rule holds that source from then on.
//! - **Nothing forgotten**: every kind of message of the net's inventory that tells a failure (`universe::ANSWERS`:
//!   « -err », « -Disconnected »…) is delivered by a line of [`sources`], or is listed in [`NOT_A_STOP`] with the
//!   reason why it is not the stop of a source of the market's data. A new source gets its line.
//!
//! What no line can show: a source that stops without the app being told. The prices of the watchlist
//! (`UserEvent::Quotes`, `UserEvent::Prices`) have no message for a reading that failed: the app receives nothing,
//! so it has nothing to show. And a stream that goes silent while its connection stays open sends nothing either
//! (`argos_data::live`: the desktop's connection is made again after a minute of silence, the browser's is not).
//!
//! The scenes are drawn in a copy of the test program that runs one test alone ([`alone_in_a_copy`]), as the scenes
//! of the net are: no other test changes the theme or the language between two images, and the folders an app at
//! rest empties are the copy's own. `ARGOS_CAPTURES=<folder>` writes the images of every source there.

use super::furniture::{deliver, trade, COIN};
use super::scenes::only;
use super::{copy, fresh, on_its_own, plain, save_png, universe, Form};
use crate::actions::hyperliquid::HlLu;
use crate::*;
use argos_render::dock::{DockPos, ModSize, Module};
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::atomic::Ordering;

/// What tells a copy of the test program that it is one, and where it writes the proof that it did its work.
const COPY: &str = "ARGOS_STOPPED_COPY";

/// The endings of the kinds of messages that tell a failure (`universe::ANSWERS`): a reading refused, a stream cut,
/// a stream that cannot open, an answer with nothing in it, and « any other message of that stream » (the cut of the
/// liquidations is one of those).
const FAILURES: [&str; 6] = ["-err", "-Disconnected", "-Indisponible", "-cut", "-none", "-other"];

/// A source of the market's data, on a screen that shows them.
struct Source {
    /// What stops, in plain words: the name the messages of the test and [`SEEN`] use.
    name: &'static str,
    /// The scene of the net whose screen shows the source's data, already arrived (« pc/answer/Murs »).
    scene: &'static str,
    /// What is opened on that screen so that the data are read whole (a module alone in the column).
    open: fn(&mut App),
    /// What the app receives when the source stops.
    stops: fn(&mut App) -> Vec<UserEvent>,
    /// What it receives when the source is back, where that alone must take the sign of the stop away.
    back: Option<fn(&mut App) -> Vec<UserEvent>>,
}

impl Source {
    fn of(name: &'static str, scene: &'static str, stops: fn(&mut App) -> Vec<UserEvent>) -> Source {
        Source {
            name,
            scene,
            open: |_| {},
            stops,
            back: None,
        }
    }

    fn opened(mut self, open: fn(&mut App)) -> Source {
        self.open = open;
        self
    }

    fn back(mut self, back: fn(&mut App) -> Vec<UserEvent>) -> Source {
        self.back = Some(back);
        self
    }
}

/// A stream cut.
fn cut() -> LiveEvent {
    LiveEvent::Disconnected("essai".into())
}

/// A stream that cannot open.
fn cannot_open() -> LiveEvent {
    LiveEvent::Indisponible("essai".into())
}

/// A reading refused.
fn refused<T>() -> Result<T, String> {
    Err("refusé (essai)".into())
}

/// The stream of the chart's market: the app only takes the candles and the live of the market it shows now.
fn stream(a: &App) -> u64 {
    a.gen.load(Ordering::SeqCst)
}

/// One module alone in the column of the right, as tall as the column: its data are read whole.
fn alone(a: &mut App, module: Module) {
    only(a, &[module], DockPos::Right);
    for item in a.watch.dock.items.iter_mut().filter(|item| item.module == module) {
        item.size = ModSize::Fill;
    }
}

/// The figures of the crypto market as they were, with this failure of their source (or none).
fn market(a: &App, error: Option<&str>) -> Vec<UserEvent> {
    let mut market = a.watch.market.clone();
    market.error = error.map(String::from);
    vec![UserEvent::Market(Box::new(market))]
}

/// The address the Hyperliquid panel reads receives this.
fn account(a: &App, answer: HlLu) -> Vec<UserEvent> {
    let read = a.watch.explorer.adresse.clone();
    read.map(|address| UserEvent::Hl(address, answer)).into_iter().collect()
}

/// Every book the app asked the summed order book for, refused.
fn books_refused(a: &App) -> Vec<UserEvent> {
    let (Some(asked), Some(chosen)) = (a.feeds.carnets.as_ref(), a.carnets_choix()) else {
        return vec![];
    };
    let all = chosen.carnets.iter().map(|(venue, money)| (*venue, *money, refused())).collect();
    vec![UserEvent::Carnets(asked.key.clone(), all)]
}

/// Bitfinex's own book, refused.
fn bitfinex_refused(a: &App) -> Vec<UserEvent> {
    let asked = a.feeds.carnets.as_ref();
    asked.map(|asked| UserEvent::CarnetBitfinex(asked.key.clone(), refused())).into_iter().collect()
}

/// The stream of the walls receives this.
fn walls(a: &App, event: LiveEvent) -> Vec<UserEvent> {
    vec![UserEvent::MursFlux(a.feeds.murs_flux_gen.load(Ordering::SeqCst), event)]
}

/// Every source, on the screen that shows its data.
fn sources() -> Vec<Source> {
    use argos_data::liqdata::Venue;
    use argos_data::murs::Carnet;
    let of = Source::of;
    vec![
        // 🕯 the chart
        of("the chart's live stream, cut", "pc/answer/Live-Connected", |a| {
            vec![UserEvent::Live(stream(a), cut())]
        })
        .back(|a| vec![UserEvent::Live(stream(a), LiveEvent::Connected)]),
        of("the chart's live stream, cut (phone)", "cell/answer/Live-Connected", |a| {
            vec![UserEvent::Live(stream(a), cut())]
        })
        .back(|a| vec![UserEvent::Live(stream(a), LiveEvent::Connected)]),
        of("the chart's candles, refused", "pc/rest", |a| vec![UserEvent::Loaded(stream(a), refused())]),
        of("the chart's candles, refused (phone)", "cell/rest", |a| {
            vec![UserEvent::Loaded(stream(a), refused())]
        }),
        of("the past of the chart, refused", "pc/rest", |a| {
            vec![UserEvent::Older(a.older_key_sync(), refused())]
        }),
        of("a cell of the split screen, its candles refused", "pc/answer/Pane", |a| {
            a.panes.iter().map(|cell| UserEvent::Pane(cell.demande(), refused())).collect()
        }),
        // 📖 the order books
        of("the order book of the column, its stream cut", "pc/rest", |_| vec![UserEvent::Flow(cut())]).opened(|a| alone(a, Module::Book)),
        of("the order book, its stream cut (phone)", "cell/tool/Carnet", |_| vec![UserEvent::Flow(cut())]),
        of("the heatmap of several exchanges, every book refused", "pc/answer/Carnets", |a| {
            books_refused(a)
        }),
        of("the order book of several exchanges, every book refused", "pc/answer/Carnets/module", |a| {
            books_refused(a)
        }),
        of("the heatmap of Bitfinex's book, refused", "pc/answer/CarnetBitfinex", |a| bitfinex_refused(a)),
        of("Bitfinex's order book, refused", "pc/answer/CarnetBitfinex/module", |a| bitfinex_refused(a)),
        of("Bitfinex's figures, refused", "pc/answer/StatutBitfinex", |_| {
            vec![UserEvent::StatutBitfinex(COIN.into(), refused())]
        }),
        of("Bitfinex's figures, an answer without them", "pc/answer/StatutBitfinex", |_| {
            vec![UserEvent::StatutBitfinex(COIN.into(), Ok(None))]
        }),
        of("the walls of the order books, their stream cut", "pc/answer/Murs", |a| walls(a, cut())),
        of("the walls of the order books, every book refused", "pc/answer/Murs", |_| {
            let all = Carnet::PERP.iter().chain(Carnet::SPOT.iter()).map(|book| (*book, refused())).collect();
            vec![UserEvent::Murs(COIN.into(), all, Some(refused()), None)]
        }),
        // ⚡ the trades
        of("the trades of the column, their stream cut", "pc/rest", |_| vec![UserEvent::Flow(cut())]).opened(|a| alone(a, Module::Trades)),
        of("the real order flow of an exchange, its stream cut", "pc/answer/Orderflow", |_| {
            vec![UserEvent::Orderflow(Exchange::Binance, cut())]
        }),
        of(
            "the trades of an exchange in the column, their stream cut",
            "pc/answer/Orderflow/module",
            |_| vec![UserEvent::Orderflow(Exchange::Binance, cut())],
        ),
        of("the real order flow of an exchange, its stream cannot open", "pc/answer/Orderflow", |_| {
            vec![UserEvent::Orderflow(Exchange::Binance, cannot_open())]
        }),
        // 💥 the liquidations
        of("the liquidations, their stream cut", "pc/tool/Liquidations", |_| vec![UserEvent::Liq(cut())]),
        of("the liquidation map, every exchange refused", "pc/answer/LiqMap", |a| {
            let all = vec![(Venue::Binance, refused()), (Venue::Okx, refused())];
            vec![UserEvent::LiqMap(a.feeds.liq_key.clone(), all, None)]
        }),
        // 🐋 the whales
        of("the whales with transactions on screen, their stream cut", "pc/tool/Whales", |_| {
            vec![UserEvent::Whales(cut())]
        }),
        of("the whales with transactions on screen, their stream cannot open", "pc/tool/Whales", |_| {
            vec![UserEvent::Whales(cannot_open())]
        }),
        of(
            "the whales with no transaction on screen, their stream cut",
            "pc/answer/Whales-Connected",
            |_| vec![UserEvent::Whales(cut())],
        )
        .back(|_| vec![UserEvent::Whales(LiveEvent::Connected)]),
        of("the flow of all the orders (whales), refused", "pc/answer/WhaleFlow", |a| {
            vec![UserEvent::WhaleFlow(a.watch.whales.flow_coin.clone(), refused())]
        }),
        of("the positions of a followed whale, refused", "pc/answer/WhaleAccount", |a| {
            let followed = a.watch.whales.follow.clone();
            followed.map(|address| UserEvent::WhaleAccount(address, refused())).into_iter().collect()
        }),
        // 📈 the markets
        of("the 24 h figures of an exchange, refused", "pc/tool/Watchlist", |_| {
            vec![UserEvent::Tickers(Exchange::Binance, refused())]
        }),
        of("all the markets of an exchange, refused", "pc/answer/Tous", |a| {
            vec![UserEvent::Tous(a.tous_bourse(), refused())]
        }),
        of("the funding history, refused", "pc/rest", |_| {
            vec![UserEvent::FundingHist(COIN.into(), refused())]
        })
        .opened(|a| alone(a, Module::Funding)),
        of("the funding rate of the demo account, no answer", "pc/answer/FundingDemo", |_| {
            vec![UserEvent::FundingDemo(Exchange::Binance, COIN.into(), None)]
        }),
        of("the figures of the crypto market, refused", "pc/tool/Market", |a| {
            market(a, Some("CoinGecko : refusé (essai)"))
        })
        .back(|a| market(a, None)),
        of("CoinMarketCap's reading, refused", "pc/answer/Cmc/series", |_| vec![UserEvent::Cmc(refused())]),
        of("the macro markets, refused", "pc/rest", |_| vec![UserEvent::World(refused())]).opened(|a| alone(a, Module::World)),
        of("the calendar, refused", "pc/tool/Agenda", |_| vec![UserEvent::Agenda(refused())]),
        of("the news, refused", "pc/tool/News", |_| vec![UserEvent::News(refused())]),
        // 👛 the accounts read (read-only)
        of("the account of a Hyperliquid address, refused", "pc/answer/Hl-Compte", |a| {
            account(a, HlLu::Compte(refused()))
        }),
        of("the orders of a Hyperliquid address, refused", "pc/answer/Hl-Ordres", |a| {
            account(a, HlLu::Ordres(refused()))
        }),
        of("the trades of a Hyperliquid address, refused", "pc/answer/Hl-Fills", |a| {
            account(a, HlLu::Fills(refused()))
        }),
        of("the fundings of a Hyperliquid address, refused", "pc/answer/Hl-Funding", |a| {
            account(a, HlLu::Funding(refused()))
        }),
        of("a followed address read in the background, refused", "pc/answer/Hl-Suivi/list", |a| {
            let followed = a.watch.explorer.suivies.first().cloned();
            followed.map(|address| UserEvent::Hl(address, HlLu::Suivi(refused()))).into_iter().collect()
        }),
        of("the wallet, refused", "pc/answer/Wallet/module", |a| {
            vec![UserEvent::Wallet(a.feeds.wallet_addr.clone(), refused())]
        }),
    ]
}

/// What was seen (23 Nov. 2026, on the tree of `interface-2` at 97d603cf): the sources whose stop changes nothing on
/// the screen, and what stays there. Written down, not excused: on the day one of them shows its stop the test asks
/// for its line to go, and the rule holds it from then on.
const SEEN: &[(&str, &str)] = &[
    (
        "a cell of the split screen, its candles refused",
        "the cell keeps the candles it had (the failure is only written in the terminal)",
    ),
    (
        "the order book of the column, its stream cut",
        "the book stays as it was (a cut of Hyperliquid's stream only marks the order flow)",
    ),
    ("the order book, its stream cut (phone)", "the book stays as it was"),
    ("the heatmap of several exchanges, every book refused", "the heatmap keeps its last reading"),
    (
        "the order book of several exchanges, every book refused",
        "the last sum stays; the title counts the books that answered, but the width of the column cuts that count off",
    ),
    ("the heatmap of Bitfinex's book, refused", "the heatmap keeps its last reading"),
    ("Bitfinex's figures, refused", "the figures read before stay in the header"),
    ("Bitfinex's figures, an answer without them", "the figures read before stay in the header"),
    (
        "the walls of the order books, their stream cut",
        "the walls drawn stay (what the stream alone told is forgotten for the next reading)",
    ),
    ("the walls of the order books, every book refused", "the walls drawn stay"),
    ("the trades of the column, their stream cut", "the list stays as it was"),
    (
        "the real order flow of an exchange, its stream cut",
        "the footprint and the CVD stay (the minute of the cut is kept as not complete)",
    ),
    ("the trades of an exchange in the column, their stream cut", "the list stays as it was"),
    (
        "the liquidations, their stream cut",
        "the list stays as it was (a message that is not a liquidation is left out)",
    ),
    (
        "the whales with transactions on screen, their stream cut",
        "the list stays: the note of the cut is only written in place of an empty list",
    ),
    (
        "the whales with transactions on screen, their stream cannot open",
        "the list stays: the note is only written in place of an empty list",
    ),
    ("the positions of a followed whale, refused", "the positions read before stay"),
    ("the 24 h figures of an exchange, refused", "the figures read before stay"),
    ("the funding history, refused", "the history read before stays"),
    ("the funding rate of the demo account, no answer", "the rate read before stays"),
    (
        "the macro markets, refused",
        "the tiles stay: the note is only written in place of an empty list",
    ),
    ("the calendar, refused", "the list stays: the note is only written in place of an empty list"),
    ("the news, refused", "the list stays: the note is only written in place of an empty list"),
    ("the orders of a Hyperliquid address, refused", "the orders read before stay"),
    ("the trades of a Hyperliquid address, refused", "the trades read before stay"),
    ("the fundings of a Hyperliquid address, refused", "the fundings read before stay"),
    (
        "a followed address read in the background, refused",
        "nothing: its alerts wait for the next reading",
    ),
    ("the wallet, refused", "the state read before stays"),
];

/// The kinds of messages that tell a failure and are not the stop of a source of the market's data, and why.
const NOT_A_STOP: &[(&str, &str)] = &[
    ("Live-other", "the chart's stream carries candles and trades: any other message is left out"),
    ("Whales-other", "the stream of the whales carries transactions: any other message is left out"),
    (
        "Flow-other",
        "Hyperliquid's stream carries its book and its trades: any other message is left out",
    ),
    (
        "Orderflow-other",
        "the stream of an exchange's trades carries trades: any other message is left out",
    ),
    ("LangPack-err", "a language pack: not the market's data"),
    ("Ai-err", "the assistant's answer: said in its own panel"),
    ("OandaTest-err", "the answer to a key just tried: said at once, nothing was on screen before"),
    ("CmcTest-err", "the answer to a key just tried: said at once, nothing was on screen before"),
    ("CleCmc-err", "the reading of a key: not the market's data"),
    ("Passkey-err", "the answer to a login: not the market's data"),
    ("Support-err", "the tickets of the Support: not the market's data"),
    ("DexFound-err", "the answer to a search just typed: said at once, nothing was on screen before"),
    ("OandaCatalog-err", "a catalogue of symbols: it has no prices"),
    (
        "OrderflowContrats-err",
        "the sizes of an exchange's contracts, asked once: without them the footprint stays « estimé » and says so",
    ),
    ("RelaisBourse-none", "no relay for the exchanges on this page: a setting, not data"),
    ("Hl-Portfolio-err", "the profits of the account are then counted from its trades"),
];

/// Runs the `work` of the test `name` of this file alone in a copy of the test program: the test program starts the
/// copy and waits for it (what the copy said is printed if it failed); the copy does the work, then writes the proof
/// that it did. As for the scenes of the net ([`super::results`]): what the app keeps for the whole program (theme,
/// language, glass) is not changed by another test between two images, and the folders an app at rest empties are
/// the copy's own.
fn alone_in_a_copy(name: &str, work: impl FnOnce()) {
    if let Some(proof) = std::env::var_os(COPY) {
        on_its_own();
        work();
        std::fs::write(proof, name).expect("the proof that the copy did its work");
        return;
    }
    let test = format!("{}::{name}", module_path!().split_once("::").map_or("", |(_, module)| module));
    let folder = argos_core::dossiers::de_test(&format!("stopped-{name}"));
    let _ = std::fs::remove_dir_all(&folder);
    let nowhere = folder.join("nowhere");
    std::fs::create_dir_all(&nowhere).expect("the empty folder of the copy");
    let (log, proof) = (folder.join("copy.log"), folder.join("done"));
    let mut child = copy(&test, &nowhere, &log)
        .env(COPY, &proof)
        .spawn()
        .expect("a copy of the test program starts");
    // (its input stays open as long as this parent lives: `wait` would close it first)
    let input = child.stdin.take();
    let status = child.wait().expect("a copy of the test program ends");
    drop(input);
    let said = std::fs::read_to_string(&log).unwrap_or_default();
    let done = std::fs::read_to_string(&proof).ok();
    let _ = std::fs::remove_dir_all(&folder);
    assert!(status.success(), "{name}, in its copy of the test program ({status}):\n{said}");
    assert_eq!(done.as_deref(), Some(name), "the copy of the test program did not run {test}:\n{said}");
}

/// The screen drawn whole until two images in a row are the same, as the net draws a scene: that image. What leaves
/// by itself stays (a banner goes after 8 s: a slow computer draws the same image as a fast one), and what shows the
/// hour it really is is painted over.
fn settled(a: &mut App, form: Form) -> Result<Vec<u32>, String> {
    let later = web_time::Instant::now() + std::time::Duration::from_secs(3600);
    if let Some(banner) = a.banner.clone() {
        a.ui.banner_since = Some((banner, later));
    }
    let mut before = vec![];
    for _ in 0..super::MAX_FRAMES {
        a.ui.fond = None;
        a.draw();
        let mut image = std::mem::take(&mut a.ui.image_test);
        if let Some(clock) = universe::real_clock(a) {
            super::blank(&mut image, form.size(), clock);
        }
        if image == before {
            return Ok(image);
        }
        before = image;
    }
    Err(format!("its screen is still moving after {} images", super::MAX_FRAMES))
}

/// What a source gave: the kinds of messages its stop delivered, and its three images (its data on screen, after it
/// stopped, after it came back where it tells so).
struct Watched {
    kinds: Vec<&'static str>,
    alive: Vec<u32>,
    stopped: Vec<u32>,
    back: Option<Vec<u32>>,
}

/// Draws a source: its screen with its data, then stopped, then back.
fn watch(source: &Source, scene: &super::Scene) -> Result<Watched, String> {
    let form = scene.form;
    fresh(form, scene.empty, |a| {
        (scene.open)(a);
        (source.open)(a);
        let alive = settled(a, form)?;
        let stops = (source.stops)(a);
        let kinds: Vec<&'static str> = stops.iter().map(universe::answer_name).collect();
        for message in stops {
            deliver(a, message);
        }
        let stopped = settled(a, form)?;
        let back = match source.back {
            Some(back) => {
                for message in back(a) {
                    deliver(a, message);
                }
                Some(settled(a, form)?)
            }
            None => None,
        };
        // (as for a scene of the net: past 180 images in a row the app changes its glass by itself)
        assert!(a.ui.frames < 150, "{} images drawn for one source", a.ui.frames);
        Ok(Watched { kinds, alive, stopped, back })
    })
    .and_then(|watched| watched)
}

/// 📴 A source that stops is shown stopped, and its sign leaves when it is back: see the top of this file.
#[test]
fn a_source_that_stops_is_shown_stopped() {
    alone_in_a_copy("a_source_that_stops_is_shown_stopped", || {
        let captures = std::env::var_os("ARGOS_CAPTURES").map(PathBuf::from);
        let scenes = super::scenes::all();
        let sources = sources();
        let names: BTreeSet<&str> = sources.iter().map(|source| source.name).collect();
        assert_eq!(names.len(), sources.len(), "two sources have the same name");
        let mut problems: Vec<String> = vec![];
        let mut delivered: BTreeSet<&'static str> = BTreeSet::new();
        let mut silent: BTreeSet<&str> = BTreeSet::new();
        let mut unknown: BTreeSet<&str> = BTreeSet::new();
        for (k, source) in sources.iter().enumerate() {
            let watched = match scenes.iter().find(|scene| scene.name == source.scene) {
                Some(scene) => watch(source, scene).map(|watched| (watched, scene.form.size())),
                None => Err(format!("the net has no scene « {} »", source.scene)),
            };
            let (watched, size) = match watched {
                Ok(watched) if !watched.0.kinds.is_empty() => watched,
                Ok(_) => {
                    unknown.insert(source.name);
                    problems.push(format!("{}: nothing was delivered (its screen does not ask for that source)", source.name));
                    continue;
                }
                Err(why) => {
                    unknown.insert(source.name);
                    problems.push(format!("{}: {why}", source.name));
                    continue;
                }
            };
            delivered.extend(watched.kinds.iter().copied());
            if let Some(folder) = &captures {
                std::fs::create_dir_all(folder).expect("the folder of the captures");
                let file = |step: &str| folder.join(format!("stopped-{:02}-{}-{step}.png", k + 1, plain(&source.name.replace(' ', "-"))));
                save_png(&file("1-alive"), &watched.alive, size, &[]);
                save_png(&file("2-stopped"), &watched.stopped, size, &[]);
                if let Some(back) = &watched.back {
                    save_png(&file("3-back"), back, size, &[]);
                }
            }
            if watched.stopped == watched.alive {
                silent.insert(source.name);
            } else if watched.back.as_ref().is_some_and(|back| *back != watched.alive) {
                problems.push(format!("{}: back, its screen is not the one it had before the stop", source.name));
            }
        }
        // what was seen is still there, and nothing else is silent
        let seen: BTreeSet<&str> = SEEN.iter().map(|(name, _)| *name).collect();
        assert_eq!(seen.len(), SEEN.len(), "two lines of SEEN name the same source");
        for name in seen.difference(&names) {
            problems.push(format!("SEEN names « {name} », which is not a source of this test"));
        }
        for name in silent.difference(&seen) {
            problems.push(format!(
                "{name}: its stop changes nothing on the screen (its last data stay there as if they were live)"
            ));
        }
        for name in seen.intersection(&names).filter(|name| !silent.contains(*name) && !unknown.contains(*name)) {
            problems.push(format!("{name}: its stop shows now. Take its line out of SEEN: the rule holds it from now on"));
        }
        // nothing forgotten: every kind of message that tells a failure is a stop tried here, or is said not to be one
        let apart: BTreeSet<&str> = NOT_A_STOP.iter().map(|(kind, _)| *kind).collect();
        assert_eq!(apart.len(), NOT_A_STOP.len(), "two lines of NOT_A_STOP name the same kind of message");
        for kind in universe::ANSWERS.iter().filter(|kind| FAILURES.iter().any(|end| kind.ends_with(end))) {
            match (delivered.contains(kind), apart.contains(kind)) {
                (false, false) => problems.push(format!(
                    "« {kind} »: a kind of message that tells a failure, and no source of this test delivers it. A source \
                     of the market's data gets a line in `sources` (what does its screen show when it stops?); anything \
                     else gets a line in NOT_A_STOP, with the reason"
                )),
                (true, true) => problems.push(format!("« {kind} » is delivered by a source and listed in NOT_A_STOP: take that line out")),
                _ => {}
            }
        }
        for kind in apart.iter().filter(|kind| !universe::ANSWERS.contains(kind)) {
            problems.push(format!("NOT_A_STOP names « {kind} », which is not a kind of message of the app"));
        }
        assert!(problems.is_empty(), "{} problem(s):\n- {}", problems.len(), problems.join("\n- "));
    });
}

/// A trade as the lists tell two of them apart: its exchange, its market, its moment, its price, its size, its side.
fn same(t: &argos_data::whales::Whale) -> (&'static str, String, i64, u64, u64, bool) {
    (t.src, t.coin.clone(), t.time_ms, t.price.to_bits(), t.size.to_bits(), t.buy)
}

/// How many entries of a list are there more than once.
fn twice(list: &[argos_data::whales::Whale]) -> usize {
    let mut once = BTreeSet::new();
    list.iter().filter(|t| !once.insert(same(t))).count()
}

/// The lists that hold twice what an exchange gives again after a reconnection, each with what was seen. As for
/// [`SEEN`]: on the day one of them holds it once, the test asks for its line to go. None is left: the trades of the
/// column, seen on 23 Nov. 2026 (the trades given again went on top of the list a second time), hold them once since
/// `App::tape_ajouter` leaves out of the first lot after a cut what the list already holds.
const LISTED_TWICE: &[(&str, &str)] = &[];

/// 🔁 After a reconnection nothing is listed twice and nothing new is missing. A stream that comes back gives its
/// last messages again (the exchanges send their recent trades when a subscription starts: the order flow counts
/// them once, `Agregateur::ajouter`): each list of the screen holds what it held, once, and what is new. The chart's
/// candles too: the candle in progress given again is the same candle, and the next one follows it.
#[test]
fn after_a_reconnection_nothing_is_listed_twice() {
    alone_in_a_copy("after_a_reconnection_nothing_is_listed_twice", || {
        let back = |list: Vec<argos_data::whales::Whale>| [cut(), LiveEvent::Connected, LiveEvent::Whales(list)];
        let lists = fresh(Form::Pc, false, |a| {
            let mut lists: Vec<(&str, usize, bool)> = vec![];
            // ⚡ the trades of the column: Hyperliquid's stream is cut, comes back, gives its six last trades again
            // and a new one
            let before = a.tape_vue().to_vec();
            assert!(
                before.len() >= 6 && twice(&before) == 0,
                "the trades of the app at rest: {} of them",
                before.len()
            );
            let new = trade(COIN, 1, 222.25, 3.0, true, "Hyperliquid");
            let again: Vec<_> = before.iter().take(6).cloned().chain([new.clone()]).collect();
            for message in [cut(), LiveEvent::Connected, LiveEvent::Tape(again)] {
                deliver(a, UserEvent::Flow(message));
            }
            let after = a.tape_vue();
            lists.push(("the trades of the column", twice(after), after.iter().any(|t| same(t) == same(&new))));
            // 🐋 the whales: their stream is cut, comes back, gives its three last transactions again and a new one
            let before = a.watch.whales.list.clone();
            assert!(
                before.len() >= 3 && twice(&before) == 0,
                "the whales of the app at rest: {} of them",
                before.len()
            );
            let new = trade(COIN, 1, 222.25, 2_000.0, false, "Hyperliquid");
            let again: Vec<_> = before.iter().take(3).cloned().chain([new.clone()]).collect();
            for message in back(again) {
                deliver(a, UserEvent::Whales(message));
            }
            let after = &a.watch.whales.list;
            lists.push(("the whales", twice(after), after.iter().any(|t| same(t) == same(&new))));
            // 🕯 the chart: its stream is cut, comes back, gives the last candle again, then the next one
            let last = a.chart.raw.last().copied().expect("the candles of the app at rest");
            let next = Bar {
                time: last.time + a.chart.interval.secs(),
                open: last.close,
                high: last.close + 1.0,
                low: last.close - 1.0,
                close: last.close + 0.5,
                volume: 3.0,
            };
            let (candles, stream) = (a.chart.raw.len(), stream(a));
            for message in [cut(), LiveEvent::Connected, LiveEvent::Bar(last), LiveEvent::Bar(next)] {
                deliver(a, UserEvent::Live(stream, message));
            }
            let raw = &a.chart.raw;
            let once = raw.len() == candles + 1 && raw.windows(2).all(|pair| pair[0].time < pair[1].time);
            let newest = raw.last().is_some_and(|b| b.time == next.time && b.close == next.close);
            lists.push(("the chart's candles", usize::from(!once), newest));
            lists
        })
        .expect("the app at rest");
        let mut problems: Vec<String> = vec![];
        for (name, twice, new) in lists {
            let seen = LISTED_TWICE.iter().any(|(list, _)| *list == name);
            match (twice > 0, seen) {
                (true, false) => problems.push(format!("{name}: after the reconnection, {twice} of its entries are there twice")),
                (false, true) => problems.push(format!(
                    "{name}: nothing is there twice now. Take its line out of LISTED_TWICE: the rule holds it from now on"
                )),
                _ => {}
            }
            if !new {
                problems.push(format!("{name}: what came after the reconnection is not there"));
            }
        }
        assert!(problems.is_empty(), "{} problem(s):\n- {}", problems.len(), problems.join("\n- "));
    });
}
