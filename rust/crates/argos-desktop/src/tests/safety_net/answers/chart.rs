//! 🕸 The messages of the chart: its candles (loaded, older, repaired), its live stream, and what other sources lay
//! on it or put in its place: the books of several exchanges, the real order flow, a DEX pool, a series of
//! CoinMarketCap.

use super::super::furniture::{self, deliver, COIN};
use super::super::scenes::{laid_out, only, tool_open, List};
use super::super::universe::{id, id_of};
use super::super::windows::{footprint_exchange, footprint_panel};
use super::super::Form;
use super::{book, lit, loaded, on, plus_panel, stream, ADDRESS};
use crate::run::Candles;
use crate::*;
use argos_core::orderflow::{Cle, FlowBar};
use argos_render::dock::{DockPos, Module};
use argos_render::indicators::Ind;
use winit::dpi::PhysicalPosition;

/// Every scene of the chart's messages.
pub(super) fn scenes(l: &mut List) {
    history(l);
    live(l);
    books(l);
    bitfinex(l);
    against_btc(l);
    order_flow(l);
    merged_footprint(l);
    dex(l);
    market_cap(l);
    small_candles(l);
}

/// The view on the oldest candles, as after a long drag to the right.
fn oldest(a: &mut App) {
    let view = &mut a.chart.viewport;
    let seen = view.plot_w() / view.bar_spacing;
    let back = (a.chart.slots.total() - seen + view.right_offset).max(0.0);
    view.scroll_px(back * view.bar_spacing, &a.chart.slots);
}

/// `count` candles before the first one of the chart, each one a little lower than the next.
fn before(a: &App, count: i64) -> Vec<Bar> {
    let Some(first) = a.chart.raw.first().copied() else { return vec![] };
    let every = a.chart.interval.secs();
    (1..=count)
        .rev()
        .map(|k| {
            let close = first.open - (k - 1) as f64 * 0.5;
            Bar {
                time: first.time - k * every,
                open: close - 0.5,
                high: close + 1.0,
                low: close - 1.5,
                close,
                volume: 20.0,
            }
        })
        .collect()
}

/// 🕯 The candles: a market just opened receives them (from its exchange, from another one in its place, or the
/// failure), the past arrives when the view goes back to it (or its end, or the failure), holes are repaired.
fn history(l: &mut List) {
    let banner = [id("app", "banner")];
    l.both("answer/Loaded", &[], |a| on(a, Exchange::Binance, COIN));
    l.both("answer/Loaded/secours", &banner, |a| {
        a.open_live(Exchange::Kucoin, COIN.into());
        let mut answer = loaded(a, Exchange::Kucoin, COIN);
        (answer.primary, answer.secours) = (false, Some(Exchange::Binance));
        answer.banner = Some("Bougies d'une bourse de secours (essai)".into());
        deliver(a, UserEvent::Loaded(stream(a), Ok(answer)));
    });
    // (a hole the exchange had: its candles are refused by the check, noted to be repaired)
    l.both("answer/Loaded/holes", &[], |a| {
        a.open_live(Exchange::Binance, COIN.into());
        let mut answer = loaded(a, Exchange::Binance, COIN);
        answer.a_reparer = answer.bars.drain(170..176).map(|b| b.time).collect();
        deliver(a, UserEvent::Loaded(stream(a), Ok(answer)));
    });
    l.both("answer/Loaded-err", &banner, |a| {
        a.open_live(Exchange::Binance, COIN.into());
        deliver(a, UserEvent::Loaded(stream(a), Err("refusé (essai)".into())));
    });
    // (the candles of the last session stay, and the chart says it is offline)
    l.both("answer/Loaded-err/session", &banner, |a| {
        deliver(a, UserEvent::Loaded(stream(a), Err("refusé (essai)".into())));
    });
    l.both("answer/Older", &[], |a| {
        let key = a.older_key_sync();
        let older = before(a, 80);
        deliver(a, UserEvent::Older(key, Ok(older)));
        oldest(a);
    });
    // (nothing older on the chart's exchange: the rest comes from Binance Futures, said once)
    l.both("answer/Older/via", &banner, |a| {
        let key = a.older_key_sync();
        deliver(a, UserEvent::Older(key, Ok(vec![])));
    });
    l.both("answer/Older/start", &banner, |a| {
        let key = a.older_key_sync();
        deliver(a, UserEvent::Older(key.clone(), Ok(vec![])));
        deliver(a, UserEvent::Older(key, Ok(vec![])));
    });
    l.both("answer/Older-err", &banner, |a| {
        let key = a.older_key_sync();
        deliver(a, UserEvent::Older(key.clone(), Ok(vec![])));
        deliver(a, UserEvent::Older(key, Err("refusé (essai)".into())));
    });
    // 🕳 six candles missing: three are found again, the exchange does not have the three others
    l.both("answer/Repaired", &[], |a| {
        use argos_data::trous::{Reparation, Trou};
        let Some(tf) = a.source_tf() else { return };
        let missing: Vec<Bar> = a.chart.raw.drain(170..176).collect();
        a.rebuild();
        // (what the app notes when it asks for a repair: the market and the unit of time of the holes)
        let key = format!("{}|{}", a.main_coin(), tf.seconds());
        a.chart.trous_connus = (key.clone(), vec![]);
        let never = Trou {
            debut: missing[3].time,
            fin: missing[5].time + tf.seconds(),
        };
        let repair = Reparation {
            bars: missing[..3].to_vec(),
            irreparables: vec![never],
        };
        deliver(a, UserEvent::Repaired(key, repair));
    });
}

/// The candle that follows the last one of the chart: the one in progress at the frozen clock.
pub(super) fn next_bar(a: &App) -> Bar {
    let last = a.chart.raw.last().copied().expect("the candles of a scene");
    Bar {
        time: last.time + a.chart.interval.secs(),
        open: last.close,
        high: last.close + 2.5,
        low: last.close - 1.0,
        close: last.close + 1.5,
        volume: 12.0,
    }
}

/// ⚡ The live stream of the chart: connected, a candle, trades (the exchanges without a stream of candles), a cut.
fn live(l: &mut List) {
    l.both("answer/Live-Connected", &[], |a| deliver(a, UserEvent::Live(stream(a), LiveEvent::Connected)));
    l.both("answer/Live-Bar", &[], |a| {
        let bar = next_bar(a);
        deliver(a, UserEvent::Live(stream(a), LiveEvent::Bar(bar)));
        // (then the same candle moves: only the last candle changes)
        let moved = Bar { close: bar.close - 2.0, ..bar };
        deliver(a, UserEvent::Live(stream(a), LiveEvent::Bar(moved)));
    });
    // (a candle that arrives while the candles of a market are still loading waits for them)
    l.both("answer/Live-Bar/loading", &[], |a| {
        a.open_live(Exchange::Binance, COIN.into());
        let mut answer = loaded(a, Exchange::Binance, COIN);
        let late = answer.bars.pop().expect("the candles of a scene");
        deliver(a, UserEvent::Live(stream(a), LiveEvent::Bar(late)));
        deliver(a, UserEvent::Loaded(stream(a), Ok(answer)));
    });
    l.both("answer/Live-Trades", &[], |a| {
        let (start, px) = (next_bar(a).time, super::price(a));
        let trade = |after: i64, price: f64| live::Trade {
            time_ms: (start + after) * 1000,
            price,
            size: 2.0,
        };
        let trades = vec![trade(5, px), trade(60, px + 3.0), trade(200, px - 1.5), trade(540, px + 1.0)];
        deliver(a, UserEvent::Live(stream(a), LiveEvent::Trades(trades)));
    });
    l.both("answer/Live-Disconnected", &[], |a| {
        deliver(a, UserEvent::Live(stream(a), LiveEvent::Connected));
        deliver(a, UserEvent::Live(stream(a), LiveEvent::Disconnected("essai".into())));
    });
}

/// 📖 The books of several exchanges (the ones ticked in the heatmap's ⚙ instead of the chart's own), read and
/// summed: one reading per candle for the last twelve, which arrive from the live stream one after the other; a
/// wall that stays, a wall that follows the price, one exchange refused.
fn summed_books(a: &mut App) {
    lit(a, Ind::BookHeat);
    let mut values = argos_render::indicators::values(&a.ind.set, Ind::BookHeat);
    if let Some(source) = argos_render::bookheat::PARAMS.iter().position(|p| p.key == "source") {
        values[source] = 1.0;
    }
    a.ind.set.insert(Ind::BookHeat, values);
    let late = a.chart.raw.split_off(a.chart.raw.len().saturating_sub(12));
    a.rebuild();
    for bar in late {
        deliver(a, UserEvent::Live(stream(a), LiveEvent::Bar(bar)));
        // (what the app notes when it asks: the coin and the books chosen)
        a.carnets_refresh();
        let (Some(key), Some(chosen)) = (a.feeds.carnets.as_ref().map(|s| s.key.clone()), a.carnets_choix()) else {
            return;
        };
        let walls = [(230.0, 60.0), (195.0, 45.0), ((bar.close + 6.0).round(), 30.0)];
        let read = chosen.carnets.iter().enumerate().map(|(k, (venue, money))| {
            let answer = if k == 3 {
                Err("refusé (essai)".to_string())
            } else {
                Ok(book(bar.close, 0.25, 100, &walls))
            };
            (*venue, *money, answer)
        });
        deliver(a, UserEvent::Carnets(key, read.collect()));
    }
}

/// 📖 The summed books: as the heatmap of the chart, and in the order-book module of the column.
fn books(l: &mut List) {
    l.both("answer/Carnets", &[id_of("ind", &Ind::BookHeat)], summed_books);
    l.add(Form::Pc, "answer/Carnets/module", &[id_of("module", &Module::Book)], |a| {
        summed_books(a);
        only(a, &[Module::Book], DockPos::Right);
    });
}

/// ₿ The chart on a pair against Bitcoin (« XRP/BTC », an exchange's spot): its prices are in BTC and the header
/// says so; the demo account counts in dollars, so the Trading sheet says there is no ticket yet.
fn against_btc(l: &mut List) {
    l.both("answer/Loaded/against-btc", &[], |a| on(a, Exchange::BinanceSpot, "XRP/BTC"));
    l.both("answer/Loaded/against-btc/Trading", &[id_of("panel", &wl::Tool::Trading)], |a| {
        on(a, Exchange::BinanceSpot, "XRP/BTC");
        tool_open(a, wl::Tool::Trading);
    });
}

/// ⚡ The chart on a Bitfinex perpetual, its heatmap lit: Bitfinex has its own book, never the summed ones. What the
/// app notes when it asks is noted (the market of the book and of the figures): the key of the book it waits for.
fn on_bitfinex(a: &mut App) -> String {
    on(a, Exchange::Bitfinex, COIN);
    lit(a, Ind::BookHeat);
    a.carnets_refresh();
    a.feeds.carnets.as_ref().map(|asked| asked.key.clone()).unwrap_or_default()
}

/// 📖 Bitfinex's book around `px`, as it answers: a price every `step`, `levels` of them on each side, the twelfth
/// of each side a wall.
fn bitfinex_book(px: f64, step: f64, levels: i64) -> argos_data::book::Book {
    use argos_data::book::{Book, Level};
    let middle = (px / step).round() as i64;
    let level = |k: i64| Level {
        px: (middle + k) as f64 * step,
        sz: if k.abs() == 12 { 45.0 } else { 0.5 + (k.abs() % 5) as f64 * 0.75 },
        n: 1 + (k.abs() % 3) as u32,
    };
    Book {
        coin: COIN.into(),
        time_ms: now_s() * 1000,
        bids: (1..=levels).map(|k| level(-k)).collect(),
        asks: (1..=levels).map(level).collect(),
    }
}

/// ⚡ 📖 A Bitfinex perpetual: its own book in the heatmap and in the order-book module (one reading per candle for
/// the last twelve, which arrive from the live stream one after the other), then a reading refused (what was read
/// stays; the module says so); its figures in the header (mark, open interest, funding and the time left before it
/// is paid), then an answer without figures and an answer refused (the figures read stay).
fn bitfinex(l: &mut List) {
    let read = |a: &mut App| {
        let key = on_bitfinex(a);
        let late = a.chart.raw.split_off(a.chart.raw.len().saturating_sub(12));
        a.rebuild();
        for bar in late {
            deliver(a, UserEvent::Live(stream(a), LiveEvent::Bar(bar)));
            deliver(a, UserEvent::CarnetBitfinex(key.clone(), Ok(bitfinex_book(bar.close, 0.25, 100))));
        }
        key
    };
    let refused = move |a: &mut App| {
        let key = read(a);
        deliver(a, UserEvent::CarnetBitfinex(key, Err("refusé (essai)".into())));
    };
    let (heat, module) = ([id_of("ind", &Ind::BookHeat)], [id_of("module", &Module::Book)]);
    l.both("answer/CarnetBitfinex", &heat, move |a| {
        read(a);
    });
    l.add(Form::Pc, "answer/CarnetBitfinex/module", &module, move |a| {
        read(a);
        only(a, &[Module::Book], DockPos::Right);
    });
    l.both("answer/CarnetBitfinex-err", &heat, refused);
    l.add(Form::Pc, "answer/CarnetBitfinex-err/module", &module, move |a| {
        refused(a);
        only(a, &[Module::Book], DockPos::Right);
    });
    let figures = |a: &mut App| {
        on_bitfinex(a);
        let figures = argos_data::exchanges::bitfinex::Statut {
            mark: super::price(a) + 0.5,
            funding: 0.000_653,
            open_interest: 12_345.0,
            next_funding_ms: (now_s() + 3 * 3600 + 12 * 60 + 45) * 1000,
        };
        deliver(a, UserEvent::StatutBitfinex(COIN.into(), Ok(Some(figures))));
    };
    l.both("answer/StatutBitfinex", &[], figures);
    l.both("answer/StatutBitfinex-none", &[], move |a| {
        figures(a);
        deliver(a, UserEvent::StatutBitfinex(COIN.into(), Ok(None)));
    });
    l.both("answer/StatutBitfinex-err", &[], move |a| {
        figures(a);
        deliver(a, UserEvent::StatutBitfinex(COIN.into(), Err("refusé (essai)".into())));
    });
}

/// 🧮 Footprint and CVD on the chart, their legend open: it says « estimé » as long as no real trade is there.
fn flow_read(a: &mut App) {
    lit(a, Ind::Footprint);
    lit(a, Ind::Cvd);
    a.ind.leg.0 = true;
}

/// 🧮 The minutes of real trades of the chart's last day (one minute after the other, three prices each, one candle
/// out of nine with a minute missed), `above` the chart's prices and `times` as big.
fn minutes(a: &App, above: f64, times: f64) -> Vec<FlowBar> {
    const STEP: f64 = 0.5;
    let mut minutes = vec![];
    for (k, bar) in a.chart.raw.iter().enumerate().rev().take(48) {
        for minute in 0..30 {
            // (the price goes from the open of its candle to its close)
            let px = bar.open + (bar.close - bar.open) * minute as f64 / 29.0 + above;
            let row = (px / STEP).floor() as i64;
            let (buys, sells) = (times * (1.0 + ((k * 7 + minute * 3) % 5) as f64), times * (1.0 + ((k * 5 + minute) % 4) as f64));
            minutes.push(FlowBar {
                time: bar.time + minute as i64 * 60,
                pas: STEP,
                rangees: vec![(row - 1, sells, buys / 2.0), (row, buys, sells), (row + 1, buys / 2.0, 0.5 * times)],
                complete: k % 9 != 0 || minute != 7,
            });
        }
    }
    minutes
}

/// 🧮 The app's own keeping of the real order flow receives these minutes of this market.
fn kept(market: &Cle, minutes: &[FlowBar]) {
    use argos_core::orderflow::FlowStore;
    if let Some(mut kept) = crate::actions::orderflow::store() {
        kept.ecrire(market, minutes).expect("the order flow a test keeps");
    }
}

/// 🧮 The real order flow of the chart's market, as the app kept it during the last day, read again as when the app
/// starts; then the candle in progress and its trades arrive.
fn real_flow(a: &mut App, exchange: Exchange) {
    flow_read(a);
    let Some(market) = a.orderflow_cle() else { return };
    kept(&market, &minutes(a, 0.0, 1.0));
    // (a market already followed is not read again: the app forgets what it follows, as when it starts)
    a.feeds.orderflow.clear();
    a.orderflow_suivre();
    let bar = next_bar(a);
    deliver(a, UserEvent::Live(stream(a), LiveEvent::Bar(bar)));
    let trades: Vec<_> = [(50, 0.0, 6.0, true), (35, 0.5, 4.0, false), (20, 1.0, 9.0, true), (5, 1.5, 3.0, false)]
        .into_iter()
        .map(|(ago, above, size, buy)| furniture::trade(COIN, ago, bar.open + above, size, buy, exchange.name()))
        .collect();
    if exchange == Exchange::Hyperliquid {
        deliver(a, UserEvent::Flow(LiveEvent::Tape(trades)));
    } else {
        deliver(a, UserEvent::Orderflow(exchange, LiveEvent::Whales(trades)));
    }
}

/// 🧮 The real order flow: the trades of an exchange other than Hyperliquid on its own connection (its trades in
/// the module of the column too), a cut, a stream that cannot open and comes back; Hyperliquid's trades come with
/// its book; and an exchange in contracts whose contract sizes were asked for.
fn order_flow(l: &mut List) {
    let shows = [id_of("ind", &Ind::Footprint), id_of("ind", &Ind::Cvd)];
    let binance = |a: &mut App| {
        on(a, Exchange::Binance, COIN);
        real_flow(a, Exchange::Binance);
    };
    l.both("answer/Orderflow", &shows, binance);
    l.add(Form::Pc, "answer/Orderflow/module", &[id_of("module", &Module::Trades)], move |a| {
        binance(a);
        only(a, &[Module::Trades], DockPos::Right);
    });
    l.both("answer/Orderflow-Disconnected", &shows, move |a| {
        binance(a);
        deliver(a, UserEvent::Orderflow(Exchange::Binance, LiveEvent::Disconnected("essai".into())));
    });
    l.both("answer/Orderflow-Indisponible", &shows, move |a| {
        binance(a);
        deliver(a, UserEvent::Orderflow(Exchange::Binance, LiveEvent::Indisponible("essai".into())));
    });
    l.both("answer/Orderflow-Connected", &shows, move |a| {
        binance(a);
        deliver(a, UserEvent::Orderflow(Exchange::Binance, LiveEvent::Indisponible("essai".into())));
        deliver(a, UserEvent::Orderflow(Exchange::Binance, LiveEvent::Connected));
    });
    l.both("answer/Flow-cut", &shows, |a| {
        real_flow(a, Exchange::Hyperliquid);
        deliver(a, UserEvent::Flow(LiveEvent::Disconnected("essai".into())));
    });
    // (MEXC trades in contracts: the size of HYPE's contract is not known, so its footprint stays estimated; the one
    // of BTC is known, so its real order flow goes on and nothing is said)
    for (name, answer) in [("OrderflowContrats", Ok(())), ("OrderflowContrats-err", Err("refusé (essai)".to_string()))] {
        l.both(
            &format!("answer/{name}"),
            &[id("app", "banner"), shows[0].clone(), shows[1].clone()],
            move |a| {
                on(a, Exchange::Mexc, "HYPE");
                flow_read(a);
                deliver(a, UserEvent::OrderflowContrats(Exchange::Mexc, answer.clone()));
            },
        );
    }
    l.both("answer/OrderflowContrats/known", &shows, |a| {
        on(a, Exchange::Mexc, COIN);
        real_flow(a, Exchange::Mexc);
        deliver(a, UserEvent::OrderflowContrats(Exchange::Mexc, Ok(())));
    });
}

/// 🏦 The footprint of several exchanges: the app kept the minutes of real trades of the chart's market and the ones
/// of the same crypto on three other exchanges (Binance a dollar higher and twice as big, Bybit half a dollar lower
/// and half as big, Coinbase two dollars higher and a quarter as big), and reads them again as when it starts.
/// (« Auto » takes OKX too, which counts in contracts: without the network the size of its contract is never read and
/// OKX is left out, `App::fp_minutes`; the app's own tests sum it.)
fn merged_flow(a: &mut App) {
    flow_read(a);
    // (the same crypto on each exchange: the market's name is the crypto's, `App::fp_cles`)
    let market = a.orderflow_cle().expect("the real order flow of the chart's market");
    for (exchange, above, times) in [(Exchange::Binance, 1.0, 2.0), (Exchange::Bybit, -0.5, 0.5), (Exchange::Coinbase, 2.0, 0.25)] {
        let theirs = Cle {
            exchange: exchange.id().to_string(),
            ..market.clone()
        };
        kept(&theirs, &minutes(a, above, times));
    }
    real_flow(a, Exchange::Hyperliquid);
}

/// 🔍 The last candles seen from close: wide and tall enough for the cells of their footprint to be read.
fn close_up(a: &mut App) {
    let last = a.chart.raw.last().map_or(0.0, |bar| bar.close);
    // (📱 the room left at the right of the last candle counts in candles: five of these are wider than the phone)
    (a.chart.viewport.bar_spacing, a.chart.viewport.right_offset) = if a.top.phone { (66.0, 0.5) } else { (86.0, 1.5) };
    a.chart.settings.scale.manual = Some((last - 6.0, last + 6.0));
}

/// 🏦 The mouse (📱 the finger's crosshair) on the busiest row of the footprint of the candle before the last one.
fn row_pointed(a: &mut App) {
    laid_out(a);
    let footprint = a.ind.out.footprint.clone().expect("the footprint of the chart");
    let before_last = a.chart.bars.len() - 2;
    let candle = footprint
        .bars
        .iter()
        .find(|candle| candle.i == before_last)
        .expect("the footprint of that candle");
    let busiest = candle
        .rows
        .iter()
        .max_by(|x, y| (x.1 + x.2).total_cmp(&(y.1 + y.2)))
        .expect("a row of the footprint");
    let frame = a.frame_now().expect("the chart's frame");
    let geo = frame.geo();
    let (x, y) = (
        geo.x_of(a.chart.bars[before_last].time) + 10.0,
        geo.y_of((busiest.0 as f64 + 0.5) * footprint.step),
    );
    a.mouse_moved(PhysicalPosition::new(a.rw() + x, a.top_h() + y));
    if a.top.phone {
        a.croix_poser();
    }
}

/// 🏦 The footprint of several exchanges seen from close (its legend names them); the row under the mouse (📱 under
/// the finger's crosshair) with the bubble of what each exchange sold and bought there; its ⚙ of the legend, the
/// panel of the exchanges with each control pressed, and the chip of the chart's exchange, which stays (the app says
/// why).
fn merged_footprint(l: &mut List) {
    let shows = [id_of("ind", &Ind::Footprint)];
    let seen = |a: &mut App| {
        merged_flow(a);
        close_up(a);
    };
    l.both("footprint/merged", &shows, seen);
    l.both("footprint/merged/row", &shows, move |a| {
        seen(a);
        row_pointed(a);
    });
    let settings = move |a: &mut App| {
        seen(a);
        a.legend_click(argos_render::indleg::LegClick::Settings(Ind::Footprint));
    };
    l.both_followed("footprint/settings", &[id("ui", "fp_reglages")], settings, Some(footprint_panel));
    l.both("footprint/settings/chart-exchange", &[id("app", "banner")], move |a| {
        settings(a);
        laid_out(a);
        let chart = crate::actions::fpmulti::BOURSES.iter().position(|exchange| *exchange == Exchange::Hyperliquid);
        footprint_exchange(a, chart.expect("the chart's exchange in the list of the footprint"));
    });
}

/// 🦎 The ＋ panel of the watchlist, where « dex:essai » was pasted.
fn dex_asked(a: &mut App) {
    plus_panel(a);
    deliver(a, UserEvent::Paste("dex:essai".into()));
}

/// 🦎 A DEX pair: found by the search of the ＋ panel (or none, or the failure), then its pool on the chart with its
/// card in the column.
fn dex(l: &mut List) {
    use argos_data::dex::{Pair, PoolInfo, TokenInfo, Trade};
    const PAIR: &str = "ESSAI/WETH";
    let add = [id("ui", "add")];
    l.both("answer/DexFound", &add, |a| {
        dex_asked(a);
        let pair = |chain: &str, quote: &str, liquidity: f64| Pair {
            chain: chain.into(),
            dex: "essai-swap".into(),
            pool: ADDRESS.into(),
            base: "ESSAI".into(),
            base_addr: ADDRESS.into(),
            quote: quote.into(),
            price_usd: Some(0.0125),
            liq_usd: Some(liquidity),
            vol_24h: Some(liquidity / 8.0),
            buys_24h: 120,
            sells_24h: 80,
        };
        let found = vec![pair("ethereum", "WETH", 3.3e6), pair("solana", "SOL", 4.5e5)];
        deliver(a, UserEvent::DexFound("essai".into(), Ok(found)));
    });
    l.both("answer/DexFound/none", &add, |a| {
        dex_asked(a);
        deliver(a, UserEvent::DexFound("essai".into(), Ok(vec![])));
    });
    l.both("answer/DexFound-err", &add, |a| {
        dex_asked(a);
        deliver(a, UserEvent::DexFound("essai".into(), Err("refusé (essai)".into())));
    });
    let pool = |a: &mut App| {
        a.open_dex("eth".into(), ADDRESS.into(), PAIR.into());
        let answer = Candles {
            title: crate::actions::dex_title(PAIR, "eth", &a.chart.interval.short()),
            ..loaded(a, Exchange::Hyperliquid, COIN)
        };
        deliver(a, UserEvent::Loaded(stream(a), Ok(answer)));
        let info = PoolInfo {
            name: PAIR.into(),
            base_addr: ADDRESS.into(),
            price_usd: Some(222.0),
            liq_usd: Some(3.3e6),
            vol_24h: Some(4.2e5),
            chg_24h: Some(3.98),
            buys: 120,
            sells: 80,
            buyers: 64,
            sellers: 51,
            fdv: Some(9.5e7),
            mcap: Some(4.4e7),
            // (the row of the pool's age counts its days on the real clock, `argos_render::dock`, not from the moment
            //  the net holds still: forty days and a half before the real one, « 40 j » whatever the day the net runs)
            created_s: Some(crate::platform::maintenant_ms() / 1000 - 40 * 86_400 - 43_200),
            chg_5m: Some(0.25),
            chg_1h: Some(-1.5),
            chg_6h: Some(2.75),
            vol_1h: Some(2.1e4),
            vol_6h: Some(1.3e5),
            buys_1h: 9,
            sells_1h: 6,
            locked_pct: Some(62.5),
            token: Some(TokenInfo {
                holders: Some(1_250),
                top10_pct: Some(37.5),
                gt_score: Some(71.0),
                verified: true,
                honeypot: Some(false),
                mint_authority: false,
                freeze_authority: true,
                twitter: "essai".into(),
                website: "https://example.invalid/essai".into(),
            }),
        };
        let trade = |ago: i64, buy: bool, usd: f64| Trade {
            time_s: now_s() - ago,
            buy,
            usd,
            price_usd: 222.0,
        };
        let trades = vec![trade(20, true, 1_250.0), trade(95, false, 640.0), trade(400, true, 12_500.0)];
        deliver(a, UserEvent::DexPool(a.chart.source.key(), Some(Box::new(info)), trades));
    };
    l.both("answer/DexPool", &[], pool);
    l.add(Form::Pc, "answer/DexPool/module", &[id_of("module", &Module::Dex)], move |a| {
        pool(a);
        only(a, &[Module::Dex], DockPos::Right);
    });
}

/// 📈 A reading of the whole market by CoinMarketCap.
fn reading(ago: i64, usdt: f64) -> argos_data::cmcap::CapPoint {
    argos_data::cmcap::CapPoint {
        t: now_s() - ago,
        total: 2.5e12,
        btc_d: 55.5,
        eth_d: 12.25,
        usdt_d: Some(usdt),
        usdc_d: Some(2.25),
        volume: Some(9.5e10),
        chg_24h: Some(1.5),
        fng: Some((62, "Avidité".into())),
        alt_season: Some(40),
        caps: [("HYPE".to_string(), 1.375e9), ("DOGE".to_string(), 1.05e10)].into(),
    }
}

/// 📈 CoinMarketCap: a reading arrives in the Marché panel of an app that has a key (only that there is one is
/// noted, never the key), and on the chart of one of its series, made of the readings the app kept; or the failure.
fn market_cap(l: &mut List) {
    let series = |a: &mut App| {
        // (what the app kept of its earlier readings, one every five minutes, going up and down between 5,50 and
        // 5,60: `cmc_history.json`)
        let wave = |k: i64| 5.5 + (30 - (k % 60 - 30).abs()) as f64 / 300.0;
        a.feeds.cmc_hist = (1..=180).rev().map(|k| reading(k * 300, wave(k))).collect();
        a.open_cap("USDT.D");
        a.banner = None;
    };
    l.both("answer/Cmc", &[id_of("panel", &wl::Tool::Market)], |a| {
        tool_open(a, wl::Tool::Market);
        deliver(a, UserEvent::CleCmc(Ok(true), false));
        deliver(a, UserEvent::Cmc(Ok(reading(0, 5.5))));
    });
    l.both("answer/Cmc/series", &[], move |a| {
        series(a);
        deliver(a, UserEvent::Cmc(Ok(reading(0, 5.58))));
    });
    l.both("answer/Cmc-err", &[id("app", "banner")], move |a| {
        series(a);
        deliver(a, UserEvent::Cmc(Err("refusé (essai)".into())));
    });
}

/// 🔬 The candles of `seconds` inside these candles of `every` seconds: each candle cut in equal parts that go from its
/// open to its close, the part of the middle reaching its high and its low, its volume shared.
fn smaller(bars: &[Bar], every: i64, seconds: i64) -> Vec<Bar> {
    let parts = (every / seconds).max(1);
    bars.iter()
        .flat_map(|b| {
            let at = move |k: i64| b.open + (b.close - b.open) * k as f64 / parts as f64;
            (0..parts).map(move |k| {
                let (open, close, middle) = (at(k), at(k + 1), k == parts / 2);
                Bar {
                    time: b.time + k * seconds,
                    open,
                    high: if middle { b.high } else { open.max(close) },
                    low: if middle { b.low } else { open.min(close) },
                    close,
                    volume: b.volume / parts as f64,
                }
            })
        })
        .collect()
}

/// A script that reads the candles of 1 minute inside each candle of the chart, and counts them.
const READS_MINUTES: &str = "//@version=6\nindicator(\"Essai\")\nminutes = request.security_lower_tf(syminfo.tickerid, \"1\", close)\nplot(array.size(minutes), \"Bougies de 1 min\")\n";

/// 🔬 That script run in the editor, on the chart. Gives the key of the candles it asks for, as the app notes it
/// when it asks: the market, the unit of time of the chart, the smaller ones wanted. (A test has no network: the
/// app asked for nothing, and the editor first says the candles of 1 minute are not there.)
fn minutes_asked(a: &mut App) -> String {
    use crate::state::{EtatPetites, ScriptMtf};
    crate::tests::tester_dans_l_editeur(a, READS_MINUTES);
    a.refresh_indicators();
    let key = format!("{}-{COIN}-{}-{:?}", Exchange::Hyperliquid.name(), a.chart.interval.secs(), [60]);
    a.feeds.script_mtf = Some(ScriptMtf {
        key: key.clone(),
        exchange: Exchange::Hyperliquid,
        at: None,
        en_cours: true,
        echecs: 0,
        etats: [(60, EtatPetites::Chargement)].into_iter().collect(),
    });
    key
}

/// 🔬 The candles smaller than the chart's, loaded for what reads them: AI Trend (the divergences of its slots; its
/// first slot is set on 5 minutes, the others are bigger than the chart's 30 minutes), and a script that asks for
/// the candles of 1 minute (they arrive: the editor says so and the script counts 30 of them a candle; or the
/// failure).
fn small_candles(l: &mut List) {
    l.both("answer/MtfBars", &[id_of("ind", &Ind::AiTrend)], |a| {
        use argos_script::indicators::ai_trend;
        let mut values = argos_render::indicators::values(&a.ind.set, Ind::AiTrend);
        for (key, value) in [("m1_on", 1.0), ("m1_tf", 0.0)] {
            if let Some(setting) = ai_trend::PARAMS.iter().position(|p| p.key == key) {
                values[setting] = value;
            }
        }
        let every = a.chart.interval.secs();
        let wanted = ai_trend::lower_needed(&ai_trend::Settings::from_inputs(&values), every);
        a.ind.set.insert(Ind::AiTrend, values);
        lit(a, Ind::AiTrend);
        // (what the app notes when it asks: the market, the unit of time of the chart, the smaller ones wanted)
        let key = format!("{}-{COIN}-{every}-{wanted:?}", Exchange::Hyperliquid.name());
        a.feeds.mtf_at = Some((key.clone(), web_time::Instant::now()));
        let bars = wanted.iter().map(|seconds| (*seconds, smaller(&a.chart.bars, every, *seconds))).collect();
        deliver(a, UserEvent::MtfBars(key, bars));
    });
    let editor = [id("ui", "editor")];
    l.both("answer/ScriptBars", &editor, |a| {
        let key = minutes_asked(a);
        let minutes = smaller(&a.chart.bars, a.chart.interval.secs(), 60);
        deliver(a, UserEvent::ScriptBars(key, vec![(60, Ok(minutes))]));
    });
    l.both("answer/ScriptBars/refused", &editor, |a| {
        let key = minutes_asked(a);
        deliver(a, UserEvent::ScriptBars(key, vec![(60, Err("refusé (essai)".into()))]));
    });
}
