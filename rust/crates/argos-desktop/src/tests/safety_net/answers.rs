//! 🕸 The messages of the safety net: every kind of message the app receives from its other threads (`UserEvent`:
//! candles, live, books, accounts, answers of the sources), made up, on a screen that shows what the app does with
//! it.
//!
//! The app at rest already received the ones of an ordinary start ([`furniture::arrive`]). Here each other kind
//! arrives; a kind no scene delivers makes `every_screen_has_a_scene` fail (`universe::ANSWERS`). A test has no
//! network, so the app asked for nothing: where it only takes an answer it is waiting for, the scene first notes what
//! the app notes when it asks (a key, an address, a market). Nothing here is real or anyone's.

use super::furniture::{self, deliver, COIN};
use super::scenes::{corner, laid_out, only, tool_open, touch, List};
use super::states::reach;
use super::universe::{id, id_of};
use super::{candles, Form};
use crate::run::Candles;
use crate::*;
use argos_core::murs::VenueBook;
use argos_data::murs::Carnet;
use argos_render::dock::{DockPos, Module};
use argos_render::indicators::Ind;
use std::sync::atomic::Ordering;

mod app;
mod chart;
mod hyperliquid;
mod news;
mod synced;
mod typed;

/// A public address that is nobody's: « essai » written with the figures of an address, eight times.
const ADDRESS: &str = "0xe55a1e55a1e55a1e55a1e55a1e55a1e55a1e55a1";
/// The public address of a made-up whale: « baleine », the same way.
const WHALE: &str = "0xba1e1ba1e1ba1e1ba1e1ba1e1ba1e1ba1e1ba1e1";

/// Every scene of the messages.
pub(super) fn scenes(l: &mut List) {
    layers(l);
    wallet(l);
    whales(l);
    markets(l);
    assistant(l);
    panes(l);
    chart::scenes(l);
    hyperliquid::scenes(l);
    news::scenes(l);
    app::scenes(l);
    synced::scenes(l);
    typed::scenes(l);
}

/// The stream of the chart's market: the app only takes the candles and the live of the market it shows now.
fn stream(a: &App) -> u64 {
    a.gen.load(Ordering::SeqCst)
}

/// The candles of the chart's market (`base` on `exchange`), as its exchange answers.
fn loaded(a: &App, exchange: Exchange, base: &str) -> Candles {
    Candles {
        bars: candles(),
        title: exchange.chart_title(base, &a.chart.interval.short()),
        banner: None,
        primary: true,
        secours: None,
        a_reparer: vec![],
    }
}

/// The chart on another market, picked as the app does it (`App::open_live`), its candles arrived.
fn on(a: &mut App, exchange: Exchange, base: &str) {
    a.open_live(exchange, base.into());
    let answer = loaded(a, exchange, base);
    deliver(a, UserEvent::Loaded(stream(a), Ok(answer)));
}

/// An indicator of the chart, lit.
fn lit(a: &mut App, indicator: Ind) {
    if !a.ind.on.contains(&indicator) {
        a.toggle_indicator(indicator);
    }
}

/// ＋ The panel that adds a symbol, opened by the button of the watchlist.
fn plus_panel(a: &mut App) {
    tool_open(a, wl::Tool::Watchlist);
    laid_out(a);
    reach::<super::windows::Side>(a, "Add", None);
}

/// ƒx The Indicators page: the volet of the PC's top bar, the page of the phone's pill.
fn indicators_page(a: &mut App) {
    if a.top.phone {
        touch(a, argos_render::ecrans::CellHit::Fx);
    } else {
        a.fx_volet();
    }
}

/// The last price of the chart.
fn price(a: &App) -> f64 {
    a.chart.bars.last().map_or(0.0, |b| b.close)
}

/// The settings of an indicator, read by name.
fn inputs<'a>(params: &'a [argos_script::inputs::Param], values: &'a [f64]) -> argos_script::inputs::Inputs<'a> {
    argos_script::inputs::Inputs { params, vals: values }
}

/// 🧱 A book around `px`: one coin every `step`, `levels` of them on each side, and its walls (price, coins).
fn book(px: f64, step: f64, levels: i64, walls: &[(f64, f64)]) -> VenueBook {
    let mut book = VenueBook {
        venue: "essai".into(),
        ..Default::default()
    };
    let middle = (px / step).round() as i64;
    for k in (middle - levels)..=(middle + levels) {
        let p = k as f64 * step;
        let size = 1.0 + walls.iter().filter(|w| w.0 == p).map(|w| w.1).sum::<f64>();
        if p > px {
            book.asks.push((p, size));
        } else if p < px {
            book.bids.push((p, size));
        }
    }
    book
}

/// 🧱 Argos Murs with its books read twice (a wall seen twice is shown), one exchange refused, the far book, and
/// between the two photos a level told by Binance's stream.
fn walls(a: &mut App) {
    a.toggle_indicator(Ind::ArgosMurs);
    // (a wall shows at once: a scene cannot wait the minutes it must hold)
    let mut values = argos_render::indicators::values(&a.ind.set, Ind::ArgosMurs);
    if let Some(hold) = argos_render::murs::PARAMS.iter().position(|p| p.key == "holdMin") {
        values[hold] = 0.0;
    }
    a.ind.set.insert(Ind::ArgosMurs, values);
    a.murs_refresh();
    let (px, Some(step)) = (price(a), a.feeds.murs.as_ref().map(|m| m.step)) else {
        return;
    };
    let at = |percent: f64| (px * (1.0 + percent / 100.0) / step).round() * step;
    let photo = || -> Vec<(Carnet, Result<VenueBook, String>)> {
        let perp = book(px, step, 40, &[(at(1.0), 50.0), (at(-0.5), 30.0)]);
        let mut books: Vec<_> = Carnet::PERP.iter().map(|c| (*c, Ok(perp.clone()))).collect();
        books[3].1 = Err("refusé (essai)".into());
        books.extend(Carnet::SPOT.iter().map(|c| (*c, Ok(book(px, step, 40, &[(at(-1.0), 40.0)])))));
        books
    };
    let far = VenueBook {
        venue: "Hyperliquid".into(),
        bids: vec![(at(-6.0), 900.0)],
        asks: vec![(at(8.0), 700.0)],
    };
    deliver(a, UserEvent::Murs(COIN.into(), photo(), Some(Ok(far.clone())), Some(100)));
    let stream = a.feeds.murs_flux_gen.load(Ordering::SeqCst);
    let told = argos_data::murs::MajProfondeur {
        pu: 150,
        u: 200,
        asks: vec![(at(2.0), 400.0)],
        ..Default::default()
    };
    deliver(a, UserEvent::MursFlux(stream, LiveEvent::Profondeur(Box::new(told))));
    deliver(a, UserEvent::Murs(COIN.into(), photo(), Some(Ok(far)), Some(300)));
}

/// 💥 The liquidation map with the data of the exchanges its settings ask for (`refused`: each one fails).
fn liquidation_map(a: &mut App, refused: bool) {
    use argos_core::liqmap::OiBar;
    use argos_data::liqdata::Venue;
    use argos_render::liqview::{HOURS, PARAMS};
    a.toggle_indicator(Ind::LiqMap);
    // (what the app notes when it asks: the coin, the period and the exchanges of the indicator's settings)
    let values = argos_render::indicators::values(&a.ind.set, Ind::LiqMap);
    let p = inputs(&PARAMS, &values);
    let (hours, venue) = (HOURS[p.pick("period").min(HOURS.len() - 1)], p.pick("venue"));
    let key = format!("{COIN}|{hours}|{venue}");
    a.feeds.liq_key = key.clone();
    // the open interest climbs while the price falls and comes back down: positions opened, then closed
    let recent: Vec<Bar> = a.chart.bars.iter().rev().take(96).rev().copied().collect();
    let bars = |oi: f64| -> Vec<OiBar> {
        recent
            .iter()
            .enumerate()
            .map(|(k, b)| OiBar {
                time: b.time,
                open: b.open,
                high: b.high,
                low: b.low,
                close: b.close,
                oi: oi * (1.0 + ((k * 7) % 13) as f64 / 40.0),
            })
            .collect()
    };
    let asked: &[Venue] = match venue {
        0 => &[Venue::Binance],
        1 => &[Venue::Okx],
        2 => &[],
        _ => &[Venue::Binance, Venue::Okx],
    };
    let answer = |v: &Venue| (*v, if refused { Err("refusé (essai)".into()) } else { Ok(bars(4.0e6)) });
    let hyperliquid = (venue >= 2 && !refused).then(|| (recent.iter().map(|b| (b.time, b.open, b.high, b.low, b.close)).collect(), Some(2.5e6)));
    deliver(a, UserEvent::LiqMap(key, asked.iter().map(answer).collect(), hyperliquid));
}

/// 📈 The layers other modules draw on the chart, with their data: the walls of the order books, the liquidation
/// map.
fn layers(l: &mut List) {
    l.both("answer/Murs", &[id_of("ind", &Ind::ArgosMurs)], walls);
    l.both("answer/LiqMap", &[id_of("ind", &Ind::LiqMap)], |a| liquidation_map(a, false));
    l.both("answer/LiqMap/refused", &[id_of("ind", &Ind::LiqMap)], |a| liquidation_map(a, true));
}

/// 👛 The state of a public address: two positions, two balances, an order waiting, three trades.
fn snapshot() -> argos_data::wallet::Snapshot {
    use argos_data::wallet::{Fill, Order, Pos, Snapshot};
    let position = |coin: &str, size: f64, entry: f64, upnl: f64, liq: Option<f64>| Pos {
        coin: coin.into(),
        size,
        entry,
        upnl,
        leverage: 5.0,
        liq,
    };
    let fill = |coin: &str, buy: bool, px: f64, size: f64, ago: i64, dir: &str, closed_pnl: f64| Fill {
        coin: coin.into(),
        buy,
        px,
        size,
        time_ms: (now_s() - ago) * 1000,
        dir: dir.into(),
        closed_pnl,
    };
    Snapshot {
        value: 12_500.0,
        withdrawable: 8_250.0,
        margin_used: 4_250.0,
        positions: vec![position(COIN, 12.0, 210.0, 144.0, Some(172.5)), position("ETH", -40.0, 90.5, -62.0, None)],
        balances: vec![("USDC".into(), 1_500.0, 250.0), ("HYPE".into(), 320.0, 0.0)],
        orders: vec![Order {
            coin: COIN.into(),
            buy: true,
            px: 205.0,
            size: 4.0,
            kind: "Limit".into(),
        }],
        fills: vec![
            fill(COIN, true, 210.0, 12.0, 3_600, "Open Long", 0.0),
            fill("ETH", false, 90.5, 40.0, 7_200, "Open Short", 0.0),
            fill("SOL", false, 15.25, 300.0, 90_000, "Close Long", 75.5),
        ],
    }
}

/// 👛 The wallet in read-only: its address typed in the app's box, then its state arrives (or the failure): the
/// menu, and the module of the column.
fn wallet(l: &mut List) {
    let typed = |a: &mut App| {
        a.wallet_typed(ADDRESS);
        a.banner = None;
    };
    let menu = [id("list", "Wallet"), id("ui", "list")];
    l.both("answer/Wallet", &menu, move |a| {
        typed(a);
        let at = corner(a);
        a.open_wallet(at);
        deliver(a, UserEvent::Wallet(ADDRESS.into(), Ok(snapshot())));
    });
    l.both("answer/Wallet-err", &menu, move |a| {
        typed(a);
        let at = corner(a);
        a.open_wallet(at);
        deliver(a, UserEvent::Wallet(ADDRESS.into(), Err("HTTP 429 (essai)".into())));
    });
    l.add(Form::Pc, "answer/Wallet/module", &[id_of("module", &Module::Wallet)], move |a| {
        typed(a);
        deliver(a, UserEvent::Wallet(ADDRESS.into(), Ok(snapshot())));
        only(a, &[Module::Wallet], DockPos::Right);
    });
}

/// 🐋 The Baleines panel on the chart's coin, as the app notes it when it starts listening.
fn whales_open(a: &mut App) {
    tool_open(a, wl::Tool::Whales);
    let w = &mut a.watch.whales;
    (w.coin, w.flow_coin) = (COIN.into(), COIN.into());
}

/// 🐋 The Baleines panel at « 1 M$ »: no transaction that big arrived, so its list is empty and it says its note
/// (the panel only says a note in place of an empty list).
fn whales_quiet(a: &mut App) {
    whales_open(a);
    a.watch.whales.level = argos_render::whales::LEVELS.len() - 1;
}

/// 🐋 The whales: the flow of all the orders, the account of a followed wallet, the top of each exchange (which only
/// chooses the coins listened to: nothing of it is drawn), a cut, a stream refused, a transaction of more than a
/// million on the chart's coin.
fn whales(l: &mut List) {
    use argos_data::whales::{Account, Flow, Position};
    let panel = [id_of("panel", &wl::Tool::Whales)];
    let flow = |every: i64, count: i64| -> Vec<Flow> {
        (0..count)
            .map(|k| Flow {
                t_ms: (now_s() - (count - k) * every) * 1000,
                buy_usd: 2.0e6 + ((k * 5) % 7) as f64 * 4.0e5,
                sell_usd: 1.5e6 + ((k * 3) % 11) as f64 * 3.0e5,
            })
            .collect()
    };
    l.both("answer/WhaleFlow", &panel, move |a| {
        whales_open(a);
        deliver(a, UserEvent::WhaleFlow(COIN.into(), Ok((flow(3600, 168), flow(300, 288)))));
    });
    l.both("answer/WhaleFlow-err", &panel, |a| {
        whales_quiet(a);
        deliver(a, UserEvent::WhaleFlow(COIN.into(), Err("refusé (essai)".into())));
    });
    let followed = |a: &mut App| {
        whales_open(a);
        a.watch.whales.follow = Some(WHALE.into());
    };
    l.both("answer/WhaleAccount", &panel, move |a| {
        followed(a);
        let position = |coin: &str, size: f64, entry: f64, value: f64, pnl: f64, liq: f64| Position {
            coin: coin.into(),
            size,
            entry,
            value,
            pnl,
            leverage: 10.0,
            liq,
        };
        let account = Account {
            value: 4.2e6,
            positions: vec![
                position(COIN, 9_000.0, 205.0, 1_998_000.0, 153_000.0, 188.0),
                position("ETH", -12_000.0, 91.0, 1_059_000.0, -33_000.0, 0.0),
            ],
        };
        deliver(a, UserEvent::WhaleAccount(WHALE.into(), Ok(account)));
    });
    l.both("answer/WhaleAccount-err", &panel, move |a| {
        followed(a);
        deliver(a, UserEvent::WhaleAccount(WHALE.into(), Err("refusé (essai)".into())));
    });
    l.both("answer/WhaleTop", &panel, |a| {
        whales_open(a);
        let top = vec![
            ("Binance".to_string(), Ok(vec!["BTC".to_string(), "ETH".into(), "SOL".into()])),
            ("Bybit".to_string(), Err("refusé (essai)".to_string())),
        ];
        deliver(a, UserEvent::WhaleTop(top));
    });
    let notes: [(&str, fn() -> LiveEvent); 3] = [
        ("Disconnected", || LiveEvent::Disconnected("essai".into())),
        ("Indisponible", || LiveEvent::Indisponible("essai".into())),
        ("Connected", || LiveEvent::Connected),
    ];
    for (name, event) in notes {
        l.both(&format!("answer/Whales-{name}"), &panel, move |a| {
            whales_quiet(a);
            // (a stream that comes back clears the note of its cut)
            if name == "Connected" {
                deliver(a, UserEvent::Whales(LiveEvent::Disconnected("essai".into())));
            }
            deliver(a, UserEvent::Whales(event()));
        });
    }
    l.both("answer/Whales/million", &[id("app", "banner")], |a| {
        a.watch.whales.coin = COIN.into();
        let big = furniture::trade(COIN, 5, 222.0, 9_000.0, true, "Hyperliquid");
        deliver(a, UserEvent::Whales(LiveEvent::Whales(vec![big])));
    });
}

/// 🌐 « Tous les marchés » of the watchlist: the markets of the exchange shown and their market caps, or the failure.
fn markets(l: &mut List) {
    use argos_data::tous::Ligne;
    let line = |base: &str, price: f64, prev: f64, millions: f64| Ligne {
        base: base.into(),
        symbol: base.into(),
        price,
        prev,
        vol_usd: Some(millions * 1e6),
        high: Some(price * 1.03),
        low: Some(price * 0.96),
        oi_usd: Some(millions * 4e5),
        cap: None,
    };
    let all = |a: &mut App| {
        tool_open(a, wl::Tool::Watchlist);
        a.watch.prefs.tous = true;
    };
    let panel = [id_of("panel", &wl::Tool::Watchlist)];
    l.both("answer/Tous", &panel, move |a| {
        all(a);
        let lines = vec![
            line(COIN, 222.0, 213.5, 4_200.0),
            line("ETH", 88.25, 90.1, 2_100.0),
            line("SOL", 15.5, 14.2, 910.0),
            line("HYPE", 4.125, 4.0, 640.0),
            line("DOGE", 0.0725, 0.07, 350.0),
        ];
        deliver(a, UserEvent::Tous(a.tous_bourse(), Ok(lines)));
        let caps = [(COIN, 4.4e9), ("ETH", 1.06e9), ("SOL", 7.5e8)];
        deliver(a, UserEvent::TousCaps(caps.into_iter().map(|(coin, cap)| (coin.to_string(), cap)).collect()));
    });
    l.both("answer/Tous-err", &panel, move |a| {
        all(a);
        deliver(a, UserEvent::Tous(a.tous_bourse(), Err("refusé (essai)".into())));
    });
    // 🪙 the logo of a coin of the watchlist: a made-up image (a red square)
    l.both("answer/Logo", &panel, |a| {
        tool_open(a, wl::Tool::Watchlist);
        let mut logo = tiny_skia::Pixmap::new(32, 32).expect("a small image");
        logo.fill(tiny_skia::Color::from_rgba8(200, 40, 40, 255));
        deliver(a, UserEvent::Logo(COIN.into(), logo.encode_png().expect("a PNG")));
    });
}

/// ✨ Argos AI: its answer (with its sources, with a note it proposes to keep), or the failure.
fn assistant(l: &mut List) {
    use argos_data::claude::Answer;
    let asked = |a: &mut App| {
        a.ouvrir_panneau_ai();
        a.ai.msgs.push((true, "Que dit la chart ? (essai)".into()));
        a.ai.busy = true;
    };
    let shows = [id("ui", "ai_open")];
    l.both("answer/Ai", &shows, move |a| {
        asked(a);
        let answer = Answer {
            text: "Une réponse fabriquée, assez longue pour tenir sur deux lignes du panneau de l'assistant.".into(),
            sources: vec![("Une source fabriquée".into(), "https://example.invalid/source".into())],
        };
        deliver(a, UserEvent::Ai(Ok(answer)));
    });
    l.both("answer/Ai/note", &shows, move |a| {
        asked(a);
        let answer = Answer {
            text: "Une réponse fabriquée.\n🧠 RETENIR : les essais sont courts (fabriqué)".into(),
            sources: vec![],
        };
        deliver(a, UserEvent::Ai(Ok(answer)));
    });
    l.both("answer/Ai-err", &shows, move |a| {
        asked(a);
        deliver(a, UserEvent::Ai(Err("refusé (essai)".into())));
    });
}

/// 🪟 The candles of each other cell of the split screen arrive.
pub(super) fn cells(a: &mut App) {
    let asked: Vec<(String, i64)> = a.panes.iter().map(|p| (p.demande(), p.interval.secs())).collect();
    for (key, seconds) in asked {
        deliver(a, UserEvent::Pane(key, Ok(argos_core::aggregate(&a.chart.raw, seconds))));
    }
}

/// 🪟 The split screen: the candles of each of its other cells arrive (or the failure: the cell keeps saying
/// « Chargement… »).
fn panes(l: &mut List) {
    let split = |a: &mut App| {
        a.split.layout = "4g".into();
        a.sync_panes();
    };
    l.both("answer/Pane", &[], move |a| {
        split(a);
        cells(a);
    });
    l.both("answer/Pane-err", &[], move |a| {
        split(a);
        if let Some(key) = a.panes.first().map(|p| p.demande()) {
            deliver(a, UserEvent::Pane(key, Err("refusé (essai)".into())));
        }
    });
}
