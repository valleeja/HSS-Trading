//! 🕸 The messages of the Explorer Hyperliquid (`UserEvent::Hl`): the readings of a public address (its account, its
//! orders, its trades, its fundings, its PnL) and the account of a followed address, read in the background for the
//! alerts.
//!
//! A test has no network: the address is pasted in the field as a user does it (the app notes it and asks nothing),
//! then each reading arrives, made up. The addresses are nobody's (`ADDRESS`, `WHALE`).

use super::super::furniture::{deliver, COIN};
use super::super::scenes::{laid_out, only, press, tool_open, List};
use super::super::universe::{id, id_of};
use super::super::windows::panel;
use super::super::Form;
use super::{ADDRESS, WHALE};
use crate::actions::hyperliquid::HlLu;
use crate::*;
use argos_data::explorer as hx;
use argos_render::dock::{DockPos, Module};
use argos_render::hlexplorer::ExClick;

/// Every scene of the Explorer Hyperliquid.
pub(super) fn scenes(l: &mut List) {
    readings(l);
    failures(l);
    followed(l);
    lines(l);
}

/// 🧭 The made-up address pasted in the Explorer's field: the app notes it and waits for its readings.
fn pasted(a: &mut App) {
    // (what a click on the field does: it takes the keyboard)
    a.watch.explorer.saisie = Some(String::new());
    deliver(a, UserEvent::Paste(ADDRESS.into()));
}

/// 🧭 The Explorer's panel on the made-up address.
fn read(a: &mut App) {
    tool_open(a, wl::Tool::Explorer);
    pasted(a);
}

/// A reading of the made-up address arrives.
fn arrives(a: &mut App, reading: HlLu) {
    deliver(a, UserEvent::Hl(ADDRESS.into(), reading));
}

/// A tab of the Explorer opened, as a click on it does («  Positions », « Ordres », « Trades », « Funding »).
fn tab(a: &mut App, k: usize) {
    a.watch.explorer.onglet = k;
    a.explorer_click(ExClick::Onglet(k));
}

/// A position: its market, its signed size, its entry, the price now, its leverage, its liquidation price.
fn position(coin: &str, size: f64, entry: f64, price: f64, leverage: f64, liq: Option<f64>) -> hx::Position {
    hx::Position {
        coin: coin.into(),
        size,
        entry,
        price,
        value: size.abs() * price,
        pnl: size * (price - entry),
        leverage,
        liq,
    }
}

/// An account with these positions, the biggest first as the app sorts them.
fn account_of(mut positions: Vec<hx::Position>) -> hx::Compte {
    hx::trier(&mut positions);
    hx::Compte {
        value: 12_500.0,
        ntl: positions.iter().map(|p| p.value).sum(),
        margin_used: 4_250.0,
        positions,
    }
}

/// 🧭 The account of the made-up address: three positions, the one of the chart's market the biggest (« Voir sur la
/// chart › » shows the biggest), one short without a liquidation price, one on a HIP-3 market.
fn account() -> hx::Compte {
    account_of(vec![
        position(COIN, 40.0, 210.0, 222.0, 5.0, Some(172.5)),
        position("ETH", -40.0, 90.5, 88.25, 10.0, None),
        position("xyz:NVDA", 100.0, 13.5, 14.25, 3.0, Some(9.75)),
    ])
}

/// 🧭 Its orders waiting.
fn orders() -> Vec<hx::Order> {
    let order = |coin: &str, buy: bool, px: f64, size: f64, kind: &str| hx::Order {
        coin: coin.into(),
        buy,
        px,
        size,
        kind: kind.into(),
    };
    vec![
        order(COIN, true, 205.0, 4.0, "Limit"),
        order(COIN, false, 240.0, 40.0, "Take Profit Market"),
        order("ETH", true, 95.0, 40.0, "Stop Market"),
    ]
}

/// 🧭 Its trades, the most recent first: two of the last day (one closed with a gain), one of the last month (closed
/// with a loss), one older.
fn fills() -> Vec<hx::Fill> {
    let fill = |coin: &str, buy: bool, px: f64, size: f64, ago: i64, dir: &str, closed_pnl: f64| hx::Fill {
        coin: coin.into(),
        buy,
        px,
        size,
        time_ms: (now_s() - ago) * 1000,
        dir: dir.into(),
        closed_pnl,
    };
    vec![
        fill(COIN, true, 210.0, 40.0, 3_600, "Open Long", 0.0),
        fill("SOL", false, 15.25, 300.0, 7_200, "Close Long", 75.5),
        fill("ETH", false, 90.5, 40.0, 5 * 86_400, "Open Short", 0.0),
        fill("HYPE", true, 4.25, 2_000.0, 9 * 86_400, "Close Short", -310.0),
        fill("xyz:NVDA", true, 13.5, 100.0, 40 * 86_400, "Open Long", 0.0),
    ]
}

/// 🧭 Its fundings of the last hours, the most recent first: paid on the long, received on the short.
fn fundings() -> Vec<hx::Funding> {
    (0..6)
        .map(|k| {
            let (coin, size, paid) = if k % 2 == 0 { (COIN, 40.0, -1.11) } else { ("ETH", -40.0, 0.44) };
            hx::Funding {
                time_ms: (now_s() - (k / 2 + 1) * 3_600) * 1000,
                coin: coin.into(),
                usdc: paid,
                rate: 0.0000125,
                szi: size,
            }
        })
        .collect()
}

/// 🧭 Everything the Explorer reads of the address arrived.
fn all_read(a: &mut App) {
    arrives(a, HlLu::Compte(Ok(account())));
    arrives(a, HlLu::Portfolio(Ok((Some(312.5), Some(-1_840.0)))));
    arrives(a, HlLu::Ordres(Ok(orders())));
    arrives(a, HlLu::Fills(Ok(fills())));
    arrives(a, HlLu::Funding(Ok(fundings())));
}

/// 🧭 Each reading arrives, on the tab that shows it; then the whole account with each control of its panel pressed
/// (⚙ and its followed address, ⭐, « Voir sur la chart › », each tab, a row), and the module of the column.
fn readings(l: &mut List) {
    let shows = [id_of("panel", &wl::Tool::Explorer)];
    l.both("answer/Hl-Compte", &shows, |a| {
        read(a);
        arrives(a, HlLu::Compte(Ok(account())));
    });
    l.both("answer/Hl-Portfolio", &shows, |a| {
        read(a);
        arrives(a, HlLu::Compte(Ok(account())));
        arrives(a, HlLu::Portfolio(Ok((Some(312.5), Some(-1_840.0)))));
    });
    type Tab = (&'static str, usize, fn() -> HlLu);
    let tabs: [Tab; 3] = [
        ("Ordres", 1, || HlLu::Ordres(Ok(orders()))),
        ("Fills", 2, || HlLu::Fills(Ok(fills()))),
        ("Funding", 3, || HlLu::Funding(Ok(fundings()))),
    ];
    for (name, k, reading) in tabs {
        l.both(&format!("answer/Hl-{name}"), &shows, move |a| {
            read(a);
            arrives(a, HlLu::Compte(Ok(account())));
            tab(a, k);
            arrives(a, reading());
        });
    }
    let whole = |a: &mut App| {
        a.watch.explorer.suivies.push(WHALE.into());
        read(a);
        all_read(a);
    };
    l.both_followed("answer/Hl/read", &shows, whole, Some(panel));
    l.add(Form::Pc, "answer/Hl/module", &[id_of("module", &Module::Explorer)], |a| {
        only(a, &[Module::Explorer], DockPos::Right);
        // (drawn once: the column of modules is laid out, and its field can take what is typed)
        laid_out(a);
        pasted(a);
        all_read(a);
    });
}

/// 🧭 Each reading fails. The account: the panel says it. The PnL: the tiles take it from the trades, which the app
/// reads then. The orders: the ones read before stay. The trades and the fundings: their tab keeps waiting.
fn failures(l: &mut List) {
    let shows = [id_of("panel", &wl::Tool::Explorer)];
    let refused = || -> String { "HTTP 429 (essai)".into() };
    l.both("answer/Hl-Compte-err", &shows, move |a| {
        read(a);
        arrives(a, HlLu::Compte(Err(refused())));
    });
    l.both("answer/Hl-Portfolio-err", &shows, move |a| {
        read(a);
        arrives(a, HlLu::Compte(Ok(account())));
        arrives(a, HlLu::Portfolio(Err(refused())));
        tab(a, 2);
        arrives(a, HlLu::Fills(Ok(fills())));
    });
    l.both("answer/Hl-Ordres-err", &shows, move |a| {
        read(a);
        arrives(a, HlLu::Compte(Ok(account())));
        tab(a, 1);
        arrives(a, HlLu::Ordres(Ok(orders())));
        arrives(a, HlLu::Ordres(Err(refused())));
    });
    l.both("answer/Hl-Fills-err", &shows, move |a| {
        read(a);
        arrives(a, HlLu::Compte(Ok(account())));
        tab(a, 2);
        arrives(a, HlLu::Fills(Err(refused())));
    });
    l.both("answer/Hl-Funding-err", &shows, move |a| {
        read(a);
        arrives(a, HlLu::Compte(Ok(account())));
        tab(a, 3);
        arrives(a, HlLu::Funding(Err(refused())));
    });
}

/// ⭐ The whale followed, its account read in the background a first time: one position.
fn whale_followed(a: &mut App) {
    a.watch.explorer.suivies.push(WHALE.into());
    let before = account_of(vec![position(COIN, 9_000.0, 205.0, 222.0, 10.0, Some(188.0))]);
    deliver(a, UserEvent::Hl(WHALE.into(), HlLu::Suivi(Ok(before))));
}

/// ⭐ A followed address opens a position and closes another between two readings: the banner says it, the alert
/// rings, the 🔔 list keeps it. A reading that fails changes nothing. The address the panel shows, followed too,
/// alerts the same way when its own reading shows a position closed.
fn followed(l: &mut List) {
    let moved = |a: &mut App| {
        whale_followed(a);
        let after = account_of(vec![position("ETH", -12_000.0, 91.0, 88.25, 10.0, None)]);
        deliver(a, UserEvent::Hl(WHALE.into(), HlLu::Suivi(Ok(after))));
    };
    l.both("answer/Hl-Suivi", &[id("app", "banner")], moved);
    // (the feed of notifications, behind the ⚙ of the alerts panel)
    l.both("answer/Hl-Suivi/list", &[id("notifs", "fil")], move |a| {
        moved(a);
        super::super::alerts::feed(a);
    });
    // (⚙ open: the followed addresses are listed there)
    l.both("answer/Hl-Suivi-err", &[id_of("panel", &wl::Tool::Explorer)], |a| {
        whale_followed(a);
        deliver(a, UserEvent::Hl(WHALE.into(), HlLu::Suivi(Err("HTTP 429 (essai)".into()))));
        tool_open(a, wl::Tool::Explorer);
        a.watch.explorer.reglages = true;
    });
    l.both("answer/Hl-Compte/followed", &[id("app", "banner"), id_of("panel", &wl::Tool::Explorer)], |a| {
        read(a);
        arrives(a, HlLu::Compte(Ok(account())));
        a.explorer_click(ExClick::Suivre(ADDRESS.into()));
        let mut after = account();
        after.positions.retain(|p| p.coin != "ETH");
        arrives(a, HlLu::Compte(Ok(after)));
    });
}

/// 📈 « Voir sur la chart › » pressed on the account of the made-up address: its entry and its liquidation, each a
/// line with its pill.
fn seen_on_chart(a: &mut App) {
    read(a);
    arrives(a, HlLu::Compte(Ok(account())));
    a.explorer_click(ExClick::Voir(ADDRESS.into(), COIN.into()));
}

/// The ✕ of the first pill of the lines (window points), found as the app finds it under a press: asked every two
/// points of the chart, from the top, until the rows that hold it end.
fn close_button(a: &App) -> Option<(f64, f64)> {
    let (w, h) = (a.chart.viewport.plot_w(), a.chart.viewport.plot_h());
    let mut found: Option<[f64; 4]> = None;
    let mut y = 1.0;
    while y < h {
        let mut x = 1.0;
        let mut here = false;
        while x < w {
            if argos_render::hlexplorer::fermer_sous(x, y) {
                let b = found.get_or_insert([x, y, x, y]);
                *b = [b[0].min(x), b[1].min(y), b[2].max(x), b[3].max(y)];
                here = true;
            }
            x += 2.0;
        }
        if found.is_some() && !here {
            break;
        }
        y += 2.0;
    }
    found.map(|b| (a.rw() + (b[0] + b[2]) / 2.0, a.top_h() + (b[1] + b[3]) / 2.0))
}

/// 📈 The lines of an address on the chart: shown (the phone's sheet goes down), moved with the position at the next
/// reading, gone when the position is closed, removed by the ✕ of a pill.
fn lines(l: &mut List) {
    let shows = [id("ui", "hl_lignes")];
    l.both("answer/Hl-Compte/lines", &shows, seen_on_chart);
    l.both("answer/Hl-Compte/lines/moved", &shows, |a| {
        seen_on_chart(a);
        let mut after = account();
        if let Some(p) = after.positions.iter_mut().find(|p| p.coin == COIN) {
            (p.size, p.entry, p.liq) = (60.0, 214.0, Some(186.5));
        }
        arrives(a, HlLu::Compte(Ok(after)));
    });
    l.both("answer/Hl-Compte/lines/gone", &[], |a| {
        seen_on_chart(a);
        let mut after = account();
        after.positions.retain(|p| p.coin != COIN);
        arrives(a, HlLu::Compte(Ok(after)));
    });
    l.both("answer/Hl-Compte/lines/closed", &[], |a| {
        seen_on_chart(a);
        // (drawn once: the app knows where it laid the pills)
        laid_out(a);
        if let Some((x, y)) = close_button(a) {
            press(a, x, y);
        }
    });
}
