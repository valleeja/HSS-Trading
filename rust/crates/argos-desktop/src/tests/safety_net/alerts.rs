//! 🕸 🔔 The Alertes panel: on the PC the volet beside the right column, on the phone a bottom sheet.
//!
//! Its two tabs, with an alert of each kind the panel writes and a history of firings; each of its places pressed
//! (the panel says where they are, `alertes::Etat::zones`: a place added to it is pressed too); what a press brings
//! (the menu of an alert, of « + », of ⚙, « Trade » in a lit row, the box that asks before the history is emptied);
//! and behind its ⚙ the feed of notifications, which was the whole 🔔 before the panel.
//!
//! The firings of the chart's market are real ones (the alerts are shown a price that crosses them, at made-up
//! times); the older ones, on other markets, are written as alerts.json keeps them: with the id of the chart they
//! were made on (« binance:SOL », the exchange and the base: `App::main_coin`), which is not the id of the catalogue
//! (« binance:SOLUSDT »). No name, no address, no key.

use super::scenes::{finger, laid_out, press, row, touch, used, List, Phase};
use super::universe::id;
use super::{Next, Open};
use crate::*;
use argos_core::alerts::{decimal, Cond, Declenchement, Dir, Kind, Op, Sens, Src};
use argos_render::alertes::AZone;
use argos_render::confirmer::Places;
use argos_render::ecrans::CellHit;
use argos_render::text::Text;
use std::sync::Arc;

const HOUR: i64 = 3600;
const DAY: i64 = 24 * HOUR;

/// 🔔 Alerts of each kind the panel writes, on the chart's market, and their past. The alerts: a crossing (the one of
/// [`used`]), « au-dessus » each time, two conditions on the candles, « en dessous » paused by hand, one that fired
/// (it was for once: « Déclenchée ») and one past its validity (« Expirée »). The history, newest first: the three
/// firings of those alerts, and three older ones on other markets (one of them kept before the panel: only its
/// symbol is known; the two others as the app keeps the alert of a chart of Binance and of OKX).
pub(super) fn alerts(a: &mut App) {
    used(a);
    let (sym, tf, now) = (a.symbol_label(), a.chart.interval.secs(), now_s());
    let price = |price: f64, dir: Dir| Kind::Price { price, dir };
    let cond = |l: Src, op: Op, r: Src| Cond { l, op, r, r2: None };
    let two = Kind::Multi {
        tf,
        all: true,
        conds: vec![cond(Src::Close, Op::Above, Src::Ema(20)), cond(Src::Rsi, Op::Below, Src::Num(70.0))],
    };
    a.add_alert(&sym, price(290.0, Dir::Up), Some(300), None, "");
    a.add_alert(&sym, two, None, None, "");
    let paused = a.add_alert(&sym, price(120.0, Dir::Down), None, None, "");
    a.add_alert(&sym, price(210.0, Dir::Up), None, None, "");
    a.add_alert(&sym, price(130.0, Dir::Down), None, Some(now - HOUR), "");
    a.alerts.expire(now);
    if let Some(alert) = a.alerts.list.iter_mut().find(|alert| alert.id == paused) {
        alert.on = false;
    }
    // the price of the market crosses 290 twice, three days ago then yesterday, and 210 an hour ago
    for (ago, seen) in [
        (3 * DAY, 285.0),
        (3 * DAY, 295.0),
        (26 * HOUR, 285.0),
        (26 * HOUR, 295.0),
        (2 * HOUR, 205.0),
        (HOUR, 215.0),
    ] {
        a.alerts.check_price(&sym, seen, now - ago);
    }
    assert_eq!(a.alerts.declenchements.len(), 3, "the made-up alerts fired three times");
    let older = |coin: &str, sym: &str, tf: Option<i64>, ago: i64, sens: Sens, (target, at): (f64, f64)| Declenchement {
        id: 0,
        ts: now - ago,
        sym: sym.into(),
        coin: coin.into(),
        tf,
        sens,
        target: decimal(target),
        price: decimal(at),
        every: false,
    };
    // (two days ago: between the two crossings of 290)
    let between = older("binance:SOL", "SOL", Some(HOUR), 2 * DAY, Sens::Down, (31.25, 31.2));
    a.alerts.declenchements.insert(2, between);
    a.alerts.declenchements.extend([
        older("okx:ETH", "ETH", Some(4 * HOUR), 9 * DAY, Sens::Up, (1980.0, 1981.5)),
        older("", "XAUUSD", None, 20 * DAY, Sens::Cross, (2488.0, 2488.4)),
    ]);
    a.save_alerts();
    a.banner = None;
}

/// 🔔 The same with a longer past than the panel shows: twelve older firings of alerts of the chart's market that
/// are no more (one a day, three to four weeks ago).
fn long(a: &mut App) {
    alerts(a);
    let (coin, sym, tf, now) = (a.main_coin(), a.symbol_label(), a.chart.interval.secs(), now_s());
    let old = |k: i64| Declenchement {
        id: 0,
        ts: now - (21 + k) * DAY,
        sym: sym.clone(),
        coin: coin.clone(),
        tf: Some(tf),
        sens: if k % 2 == 0 { Sens::Up } else { Sens::Down },
        target: decimal(150.0 + 5.0 * k as f64),
        price: decimal(150.5 + 5.0 * k as f64),
        every: k % 3 == 0,
    };
    a.alerts.declenchements.extend((0..12).map(old));
    a.save_alerts();
}

/// 🔔 The panel opened as the app opens it: 🖥 the 🔔 of the top bar, 📱 the 🔔 of the screen.
pub(super) fn panel(a: &mut App) {
    if a.top.phone {
        touch(a, CellHit::Alertes);
    } else {
        a.notifs_pop(Rect::new(270.0, 10.0, 30.0, 30.0));
    }
}

/// The places of the drawn panel, in the window (with the mouse where it is, as the app asks for them: the row it
/// lights has its « Trade »).
fn places(a: &App) -> Vec<(Rect, AZone)> {
    let Some(panel) = a.ui.notifs.as_ref().filter(|panel| !panel.fil) else {
        return vec![];
    };
    let (w, h) = (a.win_w(), a.top_h() + a.area.1 as f64 + argos_render::ecrans::dock_h());
    let mouse = (!a.pointer.doigt_leve).then_some(a.pointer.win_mouse);
    panel.alertes.zones(&Text::default(), panel.alertes_place(w, h), &a.alertes_donnees(), mouse)
}

/// A press in the middle of this place of the drawn panel.
pub(super) fn pressed(a: &mut App, place: AZone) {
    let found = places(a).into_iter().find(|(_, known)| *known == place);
    let (r, _) = found.unwrap_or_else(|| panic!("the alerts panel has no place {place:?}"));
    press(a, r.cx(), r.cy());
}

/// 🔔 The history of the panel: its second tab pressed, once the panel is drawn (`made`: the alerts and their past).
fn history_of(a: &mut App, made: fn(&mut App)) {
    made(a);
    panel(a);
    laid_out(a);
    pressed(a, AZone::Onglet(1));
}

/// 🔔 The history of the panel, with the alerts of [`alerts`].
fn history(a: &mut App) {
    history_of(a, alerts);
}

/// 🔔 Each place of the drawn panel, pressed (the open tab aside: it is there already).
fn each_place(a: &mut App) -> Next {
    let open = a.ui.notifs.as_ref().map_or(0, |panel| panel.alertes.onglet);
    let gesture = |(r, place): (Rect, AZone)| {
        let press: Open = Arc::new(move |a: &mut App| press(a, r.cx(), r.cy()));
        (format!("{place:?}"), press)
    };
    places(a).into_iter().filter(|(_, place)| *place != AZone::Onglet(open)).map(gesture).collect()
}

/// 🔔 « Trade » of the lit row of the history, pressed.
fn trade(a: &mut App) -> Next {
    let lit = |(_, place): &(Rect, AZone)| matches!(place, AZone::Trade(_));
    let gesture = |(r, place): (Rect, AZone)| {
        let press: Open = Arc::new(move |a: &mut App| press(a, r.cx(), r.cy()));
        (format!("{place:?}"), press)
    };
    places(a).into_iter().filter(lit).map(gesture).collect()
}

/// ✅ Each button of the open box that asks (« Supprimer cette alerte? », « Vider l'historique? »), pressed: the one
/// that does it, « Annuler », its ✕.
pub(super) fn answers(a: &mut App) -> Next {
    let Some((asked, _)) = a.ui.confirmer.clone() else { return vec![] };
    let (w, h) = a.taille_points();
    let Places { ok, annuler, fermer, .. } = asked.places(&Text::default(), w, h);
    let gesture = |(name, r): (&str, Rect)| {
        let press: Open = Arc::new(move |a: &mut App| press(a, r.cx(), r.cy()));
        (name.to_string(), press)
    };
    [("ok", ok), ("annuler", annuler), ("fermer", fermer)].into_iter().map(gesture).collect()
}

/// 🔔 The feed of notifications (news, whales, followed addresses…) in place of the panel: ⚙ of the panel pressed,
/// then its line « Fil des notifications ».
pub(super) fn feed(a: &mut App) {
    panel(a);
    laid_out(a);
    pressed(a, AZone::Reglages);
    row(a, "Fil des notifications");
}

/// 🔔 The search of the panel takes the keyboard, on the history: its field pressed.
pub(super) fn search(a: &mut App) {
    history(a);
    laid_out(a);
    pressed(a, AZone::Champ);
}

/// 🔔 Every scene of the Alertes panel.
pub(super) fn scenes(l: &mut List) {
    let shows = [id("ui", "notifs"), id("notifs", "alertes")];
    // without an alert and without a firing: what the panel says then, on each tab
    l.both("alerts/none", &shows, panel);
    l.both("alerts/none/history", &shows, |a| {
        panel(a);
        laid_out(a);
        pressed(a, AZone::Onglet(1));
    });
    let active = |a: &mut App| {
        alerts(a);
        panel(a);
    };
    l.both_followed("alerts/active", &shows, active, Some(each_place));
    l.both_followed("alerts/history", &shows, history, Some(each_place));
    // a row of the history lit (📱 touched, 🖥 clicked): its « Trade », pressed, opens its market and the demo ticket
    // (the row of another market than the chart's, then the row of the chart's)
    for (name, of) in [("other-market", 2), ("same-market", 0)] {
        let lit = move |a: &mut App| {
            history(a);
            laid_out(a);
            pressed(a, AZone::Ligne(of));
        };
        l.both_followed(&format!("alerts/history/lit-{name}"), &shows, lit, Some(trade));
    }
    // 🖥 the mouse over a row lights it too (nothing pressed)
    l.add(super::Form::Pc, "alerts/history/hovered", &shows, |a| {
        history(a);
        laid_out(a);
        let over = places(a).into_iter().find(|(_, place)| *place == AZone::Ligne(1));
        let (r, _) = over.expect("the second row of the history");
        a.mouse_moved(winit::dpi::PhysicalPosition::new(r.cx(), r.cy()));
    });
    // « Vider l'historique »: the box that asks, each of its answers
    let emptied = |a: &mut App| {
        history(a);
        laid_out(a);
        pressed(a, AZone::Vider);
    };
    l.both_followed("alerts/history/emptied", &[id("ui", "confirmer")], emptied, Some(answers));
    // a history longer than the panel: as it opens, then further down (🖥 the wheel over the list; 📱 a finger put
    // on the list goes up and stays down: the list follows it)
    l.both("alerts/history/long", &shows, |a| history_of(a, long));
    l.both("alerts/history/long/scrolled", &shows, |a| {
        history_of(a, long);
        laid_out(a);
        let third = places(a).into_iter().find(|(_, place)| *place == AZone::Ligne(2));
        let (r, _) = third.expect("the third row of the history");
        if a.top.phone {
            finger(a, 1, Phase::Down, (r.cx(), r.cy()));
            finger(a, 1, Phase::Moved, (r.cx(), r.cy() - 60.0));
            finger(a, 1, Phase::Moved, (r.cx(), r.cy() - 120.0));
        } else {
            a.mouse_moved(winit::dpi::PhysicalPosition::new(r.cx(), r.cy()));
            a.mouse_wheel(winit::event::MouseScrollDelta::LineDelta(0.0, -6.0));
        }
    });
    // the feed of notifications, behind ⚙ (its line says how many were not read; once opened, none)
    l.both("alerts/feed", &[id("ui", "notifs"), id("notifs", "fil")], |a| {
        alerts(a);
        feed(a);
    });
    // (the feed once read, the panel left and opened again: the line of its ⚙ has no count any more)
    l.both("alerts/feed/read", &[id("list", "ReglagesAlertes")], |a| {
        alerts(a);
        feed(a);
        a.ui.notifs = None;
        panel(a);
        laid_out(a);
        pressed(a, AZone::Reglages);
    });
}
