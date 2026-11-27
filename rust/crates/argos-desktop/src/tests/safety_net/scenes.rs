//! 🕸 The scenes of the safety net: every screen of the app, opened the way the app opens it.
//!
//! A scene is a name (« pc/tool/Trading »), what it must show (ids of [`universe::shown`]) and what opens it on the
//! app at rest. The families follow the code's own lists (tools, modules, indicators, candle styles, themes, sheet
//! actions, chart actions, buttons of the top bar, list menus, drawing tools…), so a new entry in one of them gets
//! its scene without a line here. A scene can also be followed ([`Scene::follow`]): each row of the menu it opened
//! is clicked, each zone of its screen, each tile of its sheet, each button of its bar; what there is to press is
//! read on the drawn screen.

use super::states::controls;
use super::universe::{self, every, id, id_of};
use super::{Form, Next, Open, Scene};
use crate::actions::pilule::PKind;
use crate::tests::{clic, clic_chart, toucher, START};
use crate::*;
use argos_render::actions::{ChartAction, Flag};
use argos_render::avis::AvisClick;
use argos_render::colorpick::ColorPick;
use argos_render::dock::{DockPos, Module};
use argos_render::drawalert::DrawAlert;
use argos_render::drawings::Point;
use argos_render::ecrans::CellHit;
use argos_render::inddlg::IndDialog;
use argos_render::indicators::Ind;
use argos_render::journal::JForm;
use argos_render::listmenu::MenuClick;
use argos_render::marches::{Filtre, Panneau};
use argos_render::objtree::ObjTree;
use argos_render::reglages::Reglages;
use argos_render::settings::ScaleMode;
use argos_render::sheet::Act;
use argos_render::support::Support;
use argos_render::topbar::{TopClick, TopMenu};
use std::sync::Arc;
use winit::dpi::PhysicalPosition;

/// The scenes being listed.
pub(super) struct List(Vec<Scene>);

impl List {
    /// A scene of one form: its name after the form's (« tool/Trading »), what it shows, what opens it.
    pub(super) fn add(&mut self, form: Form, name: &str, shows: &[String], open: impl Fn(&mut App) + Send + Sync + 'static) -> &mut Scene {
        self.0.push(Scene {
            name: format!("{}/{}", form.tag(), super::plain(name)),
            form,
            open: Arc::new(open),
            shows: shows.to_vec(),
            follow: None,
            then: None,
            empty: false,
        });
        self.0.last_mut().expect("the scene just added")
    }

    /// The same scene on the PC and on the phone.
    pub(super) fn both(&mut self, name: &str, shows: &[String], open: impl Fn(&mut App) + Send + Sync + Clone + 'static) {
        self.both_followed(name, shows, open, None);
    }

    /// The same scene on the PC and on the phone, followed.
    pub(super) fn both_followed(
        &mut self,
        name: &str,
        shows: &[String],
        open: impl Fn(&mut App) + Send + Sync + Clone + 'static,
        follow: Option<fn(&mut App) -> Next>,
    ) {
        for form in Form::ALL {
            self.add(form, name, shows, open.clone()).follow = follow;
        }
    }
}

/// Every scene, always in the same order (the shares of the copies of the test program are cut in it).
pub(super) fn all() -> Vec<Scene> {
    let mut l = List(vec![]);
    chart(&mut l);
    tools(&mut l);
    modules(&mut l);
    sheet_actions(&mut l);
    chart_actions(&mut l);
    top_bar(&mut l);
    pc_screen(&mut l);
    phone_zones(&mut l);
    phone_sheets(&mut l);
    lists(&mut l);
    drawings(&mut l);
    windows(&mut l);
    editors(&mut l);
    legends(&mut l);
    volets_pressed(&mut l);
    super::alerts::scenes(&mut l);
    super::answers::scenes(&mut l);
    super::states::scenes(&mut l);
    super::tables::scenes(&mut l);
    empty(&mut l);
    tablet(&mut l);
    let names: std::collections::BTreeSet<&str> = l.0.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names.len(), l.0.len(), "two scenes have the same name");
    l.0
}

/// 📐 The families of the PC's screens a tablet draws its own way: what a narrower window lays out differently (the
/// volets, the menus, the pages, the windows), and what the touch mode of the drawing changes (`actions::tactile`:
/// the rows of the menus, the rail, the legend, the settings). A name alone is one scene, a name ending with « / »
/// a family. Not there: the column of modules (windows from 1 200 points wide), and what is drawn in the chart
/// itself, the same on every form (the styles of candles, the themes, the indicators, the drawings).
const TABLET_FAMILIES: [&str; 45] = [
    "rest",
    "used",
    "traded",
    "home",
    "search",
    "empty/rest",
    "tool/",
    "top/",
    "menu/",
    "rail",
    "rail-unfolded",
    "list/",
    "page/",
    "volet/",
    "volet-wide/",
    "drawing-bar",
    "favourites",
    "indicator-settings/Rsi",
    "indicator-settings/AnchoredVwap",
    "box/",
    "markets",
    "journal/",
    "argos-ai/",
    "editor/",
    "script/",
    "legend/",
    "act/",
    "drawing-settings/",
    "drawing-alert",
    "alert-edited",
    "alert-selected",
    "alert-delete",
    "object-tree",
    "alerts/",
    "strategy/",
    "news/",
    "calendar/",
    "zen",
    "full-screen/",
    "pill-folded",
    "shortcuts",
    "replay",
    "simulator-date",
    "ticket-drawn",
    "demo-band/",
];

/// 📐 The bars a finger presses button by button on the tablet too (the scenes of the PC with what follows them): the
/// rail of the drawing tools, the favourites of the toolbar, the bar of a selected drawing (its layout by the finger,
/// `App::drawbar`).
const TABLET_FOLLOWED: [&str; 3] = ["rail", "favourites", "drawing-bar"];

/// 📐 The tablet (Jee's, lying down and upright: [`Form::TABLET`]): the bars of the PC, by the finger. The screens of
/// [`TABLET_FAMILIES`] are opened as the PC's scenes open them and must show the same; only those of
/// [`TABLET_FOLLOWED`] are followed (each control of the others is pressed on the PC and on the phone already). Then
/// what a tablet alone goes through: the panel as a drawer. (🟠 Its DÉMO band is in every one of its scenes since
/// paires-btc-1: a tablet in the PC's frame, which has no bar under the chart, keeps the band at the top, under the
/// top bar and right of the price legend when the first row has no room for it, `App::bandeau_place`.)
fn tablet(l: &mut List) {
    let family = |name: &str| TABLET_FAMILIES.iter().any(|f| name == *f || (f.ends_with('/') && name.starts_with(f)));
    let of_pc = |f: &str| {
        l.0.iter()
            .any(|scene| scene.form == Form::Pc && scene.name.strip_prefix("pc/").is_some_and(family) && scene.name[3..].starts_with(f))
    };
    let none: Vec<&str> = TABLET_FAMILIES.into_iter().filter(|f| !of_pc(f)).collect();
    assert!(none.is_empty(), "no scene of the PC in the families {none:?} of TABLET_FAMILIES");
    let mut derived = vec![];
    for form in Form::TABLET {
        for scene in l.0.iter().filter(|scene| scene.form == Form::Pc) {
            let Some(name) = scene.name.strip_prefix("pc/").filter(|name| family(name)) else {
                continue;
            };
            derived.push(Scene {
                name: format!("{}/{name}", form.tag()),
                form,
                open: scene.open.clone(),
                shows: scene.shows.clone(),
                follow: scene.follow.filter(|_| TABLET_FOLLOWED.contains(&name)),
                then: None,
                empty: scene.empty,
            });
        }
    }
    l.0.extend(derived);
    for form in Form::TABLET.into_iter().filter(|form| form.choice().drawer()) {
        // 📐 the panel as a drawer: opened over the chart, a touch of the chart folds it again
        l.add(form, "drawer/closed-by-the-chart", &[], |a| {
            tool_open(a, wl::Tool::Watchlist);
            laid_out(a);
            let (x, y) = (a.rw() + 80.0, a.top_h() + a.chart.viewport.plot_h() * 0.5);
            press(a, x, y);
        });
    }
}

/// The name part of an id (« style:Line » → « Line »): the end of a scene's name.
fn tail(id: &str) -> &str {
    id.split_once(':').map_or(id, |(_, name)| name)
}

/// The values a family lists have one of each name of the code's list (`named!` in `universe`).
fn complete<T>(family: &str, names: &[&str], values: &[T], name_of: impl Fn(&T) -> &'static str) {
    let without: Vec<&&str> = names.iter().filter(|n| !values.iter().any(|v| name_of(v) == **n)).collect();
    assert!(without.is_empty(), "{family}: no scene for {without:?}");
}

/// A click (PC) or a touch (phone, and a tablet used by the finger) at this point of the window.
pub(super) fn press(a: &mut App, x: f64, y: f64) {
    if a.top.phone || a.ui.form.min_target >= argos_render::formfactor::TOUCH_TARGET {
        toucher(a, x, y);
    } else {
        clic(a, x, y);
    }
}

/// 👆 A finger of the page: put down, moved, lifted, taken back by the system (a call, a gesture of the system).
#[derive(Clone, Copy)]
pub(super) enum Phase {
    Down,
    Moved,
    Lifted,
    Cancelled,
}

/// 👆 What a finger does arrives (`UserEvent::Finger`, the fingers the page reads). The gestures have a clock no test
/// can freeze (`touch::now_ms`): a scene never lifts a finger far from where it was put down (the speed a list keeps
/// after it is told with that clock) and never holds it for a long press.
pub(super) fn finger(a: &mut App, which: u64, phase: Phase, (x, y): (f64, f64)) {
    super::furniture::deliver(a, UserEvent::Finger(which, phase as u8, x, y, crate::input::touch::now_ms()));
}

/// A place for a menu opened without its button: at the top left of the chart.
pub(super) fn corner(a: &App) -> Rect {
    Rect::new(a.rw() + 60.0, a.top_h() + 40.0, 120.0, 24.0)
}

/// The app draws itself once, the chart whole as at each second of its clock: for a scene whose next step depends on
/// where things were drawn (the image the app keeps while only the mouse moves would leave some of them where they
/// were, in one order of the scenes and not in another).
pub(super) fn laid_out(a: &mut App) {
    a.ui.fond = None;
    a.draw();
}

/// The middle of the candles, in the window.
pub(super) fn middle(a: &App) -> (f64, f64) {
    (a.rw() + a.chart.viewport.plot_w() * 0.5, a.top_h() + a.chart.viewport.plot_h() * 0.5)
}

/// A point of the chart: this candle, this price.
pub(super) fn point(bar: i64, p: f64) -> Point {
    Point { t: START + bar * 1800, p }
}

/// The price of the alert of [`used`].
const ALERT: f64 = 195.0;

/// What the app holds after a little use: two drawings (the second one, a trend line, selected) and a price alert,
/// all three in view on the PC and on the phone.
pub(super) fn used(a: &mut App) {
    a.chart.drawings.push(Drawing::hline(180.0));
    a.chart.drawings.push(Drawing::new("trend", vec![point(165, 150.0), point(190, 205.0)]));
    a.selected = Some(1);
    a.add_price_alert(ALERT);
    a.banner = None;
}

/// The id of the price alert of [`used`].
fn alert(a: &App) -> u64 {
    a.alerts.list.last().map_or(0, |x| x.id)
}

/// A point of the card of the closed trade (« Argos AI · ton trade ») where a press asks this, in the window: of the
/// points tried, the one nearest the middle of those that ask it (the card lays its text out at each point tried:
/// few of them).
fn on_card(a: &App, asks: fn(AvisClick) -> bool) -> (f64, f64) {
    const APART: f64 = 10.0;
    let text = argos_render::text::Text::default();
    let (card, place) = a.avis_place(&text).expect("the card of the closed trade has a place");
    let each = |length: f64| (0..(length / APART) as usize).map(|k| (k as f64 + 0.5) * APART);
    let found: Vec<(f64, f64)> = each(place.h)
        .flat_map(|y| each(place.w).map(move |x| (x, y)))
        .filter(|(x, y)| asks(card.click(&text, place.w, *x, *y)))
        .collect();
    let middle = |of: fn(&(f64, f64)) -> f64| found.iter().map(of).sum::<f64>() / found.len().max(1) as f64;
    let (mx, my) = (middle(|p| p.0), middle(|p| p.1));
    let far = |(x, y): &(f64, f64)| (x - mx).hypot(y - my);
    let (x, y) = found.iter().min_by(|p, q| far(p).total_cmp(&far(q))).expect("the card has such a place");
    (place.x + x, place.y + y)
}

/// 📱 A point of the Trading sheet where a touch ticks « Reduce-only », over the card of the closed trade that the
/// sheet hides.
fn reduce_only_over_card(a: &mut App) -> (f64, f64) {
    const APART: f64 = 6.0;
    let (_, card) = a
        .avis_place(&argos_render::text::Text::default())
        .expect("the card of the closed trade has a place");
    let was = a.watch.panels.trading.reduce_only;
    let each = |from: f64, length: f64| (0..(length / APART) as usize).map(move |k| from + (k as f64 + 0.5) * APART);
    let mut found = vec![];
    for (x, y) in each(card.y, card.h).flat_map(|y| each(card.x, card.w).map(move |x| (x, y))) {
        a.mouse_moved(PhysicalPosition::new(x, y));
        let Some((sx, sy)) = a.pointer.side_mouse else { continue };
        let mut column = a.watch.clone();
        column.click(sx, sy, a.side_h(sx));
        if column.panels.trading.reduce_only != was {
            found.push((x, y));
        }
    }
    *found.get(found.len() / 2).expect("« Reduce-only » lies over the card")
}

/// A demo account that has traded: a limit order waiting under the price, a position open at the market, and a trade
/// closed with a gain (its card shows).
pub(super) fn traded(a: &mut App) {
    {
        let t = &mut a.watch.panels.trading;
        (t.buy, t.sl_pips, t.risk, t.guardian, t.plan) = (true, 10.0, 100.0, false, None);
    }
    let last = a.chart.bars.last().map_or(0.0, |b| b.close);
    a.brouillon_nouveau(last - 10.0, true);
    a.ordre_clic(argos_render::ordres::Prise::BrEnvoyer);
    a.place_demo_order();
    let order = argos_core::portfolio::Order {
        base: "BTC".into(),
        side: argos_core::Side::Long,
        qty: 1.0,
        price: 100.0,
        stop: Some(90.0),
        target: None,
        couts: None,
        perp: None,
    };
    let t0 = a.chart.bars[150].time;
    let closed = a.demo.open(order, t0).and_then(|id| a.demo.close(id, 105.0, t0 + 1800, "objectif"));
    a.demo_closed(&[closed.expect("a demo trade opened and closed")]);
    a.banner = None;
}

/// One more trade of the demo account of [`traded`], opened without a stop and closed by hand with a loss: its card
/// replaces the one before, and has a lesson of the Academy.
fn lost(a: &mut App) {
    let order = argos_core::portfolio::Order {
        base: "BTC".into(),
        side: argos_core::Side::Long,
        qty: 1.0,
        price: 100.0,
        stop: None,
        target: None,
        couts: None,
        perp: None,
    };
    let t0 = a.chart.bars[160].time;
    let closed = a.demo.open(order, t0).and_then(|id| a.demo.close(id, 97.0, t0 + 1800, "fermé à la main"));
    a.demo_closed(&[closed.expect("a demo trade opened and closed")]);
    a.banner = None;
}

/// The few indicators that have, between them, every kind of input an indicator of the code has (a number, a box to
/// tick, a list, a colour, a price…), an input with a help text, a greyed one, inputs on one line, each of these on a
/// first tab too (a press reaches it at once there, and what the press opens is pressed in turn), and several tabs:
/// the ones whose settings window is pressed control by control. Each one is the indicator that brings the most of
/// what the ones before it lack.
fn rich() -> Vec<Ind> {
    let has = |i: &Ind| {
        let (tabs, inputs) = i.params();
        let of = |p: &argos_script::inputs::Param| {
            let kind = format!("{:?}", p.kind);
            let mut all = vec![kind.split('(').next().unwrap_or_default().to_string()];
            let more = [
                ("help", !p.tip.is_empty()),
                ("greyed", !p.active.is_empty()),
                ("one line", !p.inline.is_empty()),
            ];
            all.extend(more.iter().filter(|(_, yes)| *yes).map(|(what, _)| what.to_string()));
            if p.group == 0 {
                all.extend(all.clone().iter().map(|what| format!("{what}, on the first tab")));
            }
            all
        };
        let mut all: Vec<String> = inputs.iter().flat_map(of).collect();
        if tabs.len() > 1 {
            all.push("tabs".to_string());
        }
        all
    };
    let (mut chosen, mut lacking): (Vec<Ind>, Vec<String>) = (vec![], Ind::ALL.iter().flat_map(has).collect());
    lacking.sort();
    lacking.dedup();
    while !lacking.is_empty() {
        let brings = |i: &Ind| {
            has(i)
                .iter()
                .filter(|what| lacking.contains(what))
                .collect::<std::collections::BTreeSet<_>>()
                .len()
        };
        // (of two that bring as much, the first one of the code's list)
        let Some(best) = Ind::ALL.iter().rev().max_by_key(|i| brings(i)).copied() else {
            break;
        };
        let brought = has(&best);
        lacking.retain(|what| !brought.contains(what));
        chosen.push(best);
    }
    chosen
}

/// 📈 The chart: at rest, after a little use, each indicator alone and its settings window, each candle style, each
/// theme.
fn chart(l: &mut List) {
    let rich = rich();
    l.both("rest", &[], |_| {});
    l.both("used", &[], used);
    l.both("traded", &[id("ui", "avis")], traded);
    // 🤖 the card of the closed trade: its ✕, its lesson of the Academy, a press on the card itself (what is under it
    // does not answer)
    let parts: [(&str, fn(AvisClick) -> bool); 2] = [("closed", |asks| asks == AvisClick::Fermer), ("pressed", |asks| asks == AvisClick::Rien)];
    for (name, part) in parts {
        l.both(&format!("traded/card/{name}"), &[], move |a| {
            traded(a);
            let (x, y) = on_card(a, part);
            press(a, x, y);
        });
    }
    // (the card of a trade without a stop has a lesson of the Academy: the card, then its lesson pressed)
    l.both("traded/card/no-stop", &[], |a| {
        traded(a);
        lost(a);
    });
    l.both("traded/card/lesson", &[], |a| {
        traded(a);
        lost(a);
        let (x, y) = on_card(a, |asks| matches!(asks, AvisClick::Lecon(_)));
        press(a, x, y);
    });
    // 📱 the Trading sheet opened over the card, and « Reduce-only » touched where the card lies under the sheet (the
    // hidden card takes the touch)
    l.add(Form::Phone, "traded/card/under-sheet", &[], |a| {
        traded(a);
        tool_open(a, wl::Tool::Trading);
        laid_out(a);
        let (x, y) = reduce_only_over_card(a);
        press(a, x, y);
    });
    for i in Ind::ALL {
        let shows = id_of("ind", &i);
        l.both(&format!("indicator/{}", tail(&shows)), std::slice::from_ref(&shows), move |a| {
            a.toggle_indicator(i)
        });
        // (control by control for the few indicators that have, between them, every kind of input)
        let pressed = rich.contains(&i).then_some(controls::<IndDialog> as fn(&mut App) -> Next);
        let settings = move |a: &mut App| {
            a.toggle_indicator(i);
            a.open_ind_dlg(i);
        };
        l.both_followed(&format!("indicator-settings/{}", tail(&shows)), &[id("ui", "ind_dlg")], settings, pressed);
        // (and their tab of styles: the colours, the lines)
        if rich.contains(&i) {
            let styles = move |a: &mut App| {
                settings(a);
                super::windows::styles(a);
            };
            l.both_followed(
                &format!("indicator-styles/{}", tail(&shows)),
                &[id("ui", "ind_dlg")],
                styles,
                Some(super::windows::inputs),
            );
        }
    }
    for (k, style) in CandleStyle::all().iter().enumerate() {
        let shows = id("style", style.name());
        // (as a pick in the list of chart types)
        l.both(&format!("style/{}", tail(&shows)), std::slice::from_ref(&shows), move |a| {
            a.switch_style(k as isize - a.chart.style as isize)
        });
    }
    for (k, theme) in argos_render::Theme::NAMES.iter().enumerate() {
        let shows = id("theme", theme);
        // (as a pick in the list of themes)
        l.both(&format!("theme/{k}-{}", tail(&shows)), std::slice::from_ref(&shows), move |a| {
            a.top.prefs.theme = k;
            argos_render::set_theme(k);
            a.save_top();
        });
    }
}

/// ₿ The catalogues of two exchanges that have markets against Bitcoin, as they arrive (made up): XRP and ETH
/// against BTC and a small token on Bitfinex Spot; XRP against BTC, and against USDT (never among them), on Binance
/// Spot.
pub(super) fn btc_catalogues(a: &mut App) {
    let market = |exchange: Exchange, symbol: &str, base: &str, quote: &str| argos_core::instrument::Instrument {
        exchange: exchange.id().into(),
        symbol: symbol.into(),
        base: base.into(),
        quote: quote.into(),
        kind: argos_core::instrument::MarketKind::Spot,
        group: "Crypto".into(),
        price_decimals: 8,
        size_decimals: 2,
        growth: false,
        active: true,
    };
    let (bitfinex, binance) = (Exchange::BitfinexSpot, Exchange::BinanceSpot);
    let on_bitfinex = [("XRPBTC", "XRP"), ("ETHBTC", "ETH"), ("ESSAI:BTC", "ESSAI")].map(|(symbol, base)| market(bitfinex, symbol, base, "BTC"));
    super::furniture::deliver(a, UserEvent::Catalog(bitfinex, on_bitfinex.to_vec()));
    let on_binance = vec![market(binance, "XRPBTC", "XRP", "BTC"), market(binance, "XRPUSDT", "XRP", "USDT")];
    super::furniture::deliver(a, UserEvent::Catalog(binance, on_binance));
}

/// A tool of the right column (PC) and of the sheets (phone): its panel opens, or what its button does.
pub(super) fn tool_open(a: &mut App, tool: wl::Tool) {
    if tool.action() {
        let button = Rect::new(a.win_w() - 40.0, a.top_h() + 60.0, 32.0, 32.0);
        a.outil_barre(tool, button);
    } else {
        (a.watch.prefs.open, a.watch.prefs.panel) = (true, tool);
    }
}

/// 🏷 Each button of the legend of the indicators, pressed. What a point of the chart does there is asked of the app
/// (`App::legende_bouton`) every few points; each thing a press can do is done once, in the middle of its button.
fn legend(a: &mut App) -> Next {
    use argos_render::indleg::LegClick;
    let ((x0, y0), viewport) = ((a.rw(), a.top_h()), &a.chart.viewport);
    let (w, h) = (viewport.plot_w(), viewport.plot_h());
    let mut found: Vec<(LegClick, [f64; 4])> = vec![];
    let mut y = 2.0;
    while y < h {
        let mut x = 2.0;
        while x < w {
            if let Some(click) = a.legende_bouton(x, y) {
                match found.iter_mut().find(|(known, _)| *known == click) {
                    Some((_, b)) => *b = [b[0].min(x), b[1].min(y), b[2].max(x), b[3].max(y)],
                    None => found.push((click, [x, y, x, y])),
                }
            }
            x += 4.0;
        }
        y += 4.0;
    }
    found
        .into_iter()
        .map(|(click, b)| {
            let (x, y) = (x0 + (b[0] + b[2]) / 2.0, y0 + (b[1] + b[3]) / 2.0);
            let gesture: Open = Arc::new(move |a: &mut App| press(a, x, y));
            (format!("{click:?}"), gesture)
        })
        .collect()
}

/// 🗂 Each tool of the right column (PC) and of the sheets (phone): its panel, or what its button opens; then with a
/// demo account that has traded, and each control of that panel pressed. Then the volets of the PC's top bar.
fn tools(l: &mut List) {
    for tool in universe::tools() {
        // 🎯 (the Stratégie volet is in the bar while a strategy is on the chart: two of them, its pill offers the
        // choice)
        let open = move |a: &mut App| {
            if tool == wl::Tool::Strategie {
                super::states::strategies(a);
            }
            tool_open(a, tool);
        };
        l.both(&format!("tool/{tool:?}"), &[], open);
        l.both_followed(
            &format!("traded/tool/{tool:?}"),
            &[],
            move |a| {
                traded(a);
                // (📱 the Trading sheet alone covers the card of the closed trade, and the hidden card takes the
                // touches meant for the sheet where it lies, see `traded/card/under-sheet`: closed first, as its ✕
                // does (`traded/card/closed`), each control of the sheet answers)
                if a.top.phone && tool == wl::Tool::Trading {
                    a.ui.avis = None;
                }
                open(a);
            },
            Some(super::windows::panel),
        );
    }
    type Volet = (&'static str, &'static str, fn(&mut App), Option<fn(&mut App) -> Next>);
    let volets: [Volet; 5] = [
        ("Symboles", "ui:symboles", App::symboles_volet, None),
        ("Indicateurs", "ui:fx_page", App::fx_volet, None),
        ("Alertes", "panel:Alertes", |a| a.notifs_pop(Rect::new(270.0, 10.0, 30.0, 30.0)), None),
        ("Reglages", "ui:reglages", App::reglages_ouvrir, Some(controls::<Reglages>)),
        ("Support", "ui:support", App::support_ouvrir, Some(controls::<Support>)),
    ];
    for (name, shows, open, pressed) in volets {
        l.add(Form::Pc, &format!("volet/{name}"), &[shows.to_string()], open).follow = pressed;
        // (the wide volet: the setting kept in interface-2.json, applied as at startup)
        l.add(
            Form::Pc,
            &format!("volet-wide/{name}"),
            &[shows.to_string(), id("ui", "volet_large")],
            move |a| {
                let wide = crate::actions::Interface2 {
                    volet_large: true,
                    ..Default::default()
                };
                a.appliquer_cadre(&wide);
                open(a);
            },
        );
    }
    // (the phone reads the same kept setting: its sheets are already as wide as the screen)
    l.add(Form::Phone, "volet-wide", &[id("ui", "volet_large")], |a| {
        let wide = crate::actions::Interface2 {
            volet_large: true,
            ..Default::default()
        };
        a.appliquer_cadre(&wide);
        tool_open(a, wl::Tool::Watchlist);
    });
}

/// 🪟 The volets and windows whose controls were not pressed one by one before: the volet of the symbols, the page of
/// the indicators, the feed of the alerts, « Aller à ». Each is opened the way a person opens it, then every control
/// it offers is pressed ([`super::windows`]).
fn volets_pressed(l: &mut List) {
    // (🖥 the volet of the symbols and the volet of the alerts are the PC's: the phone has its own sheets,
    // `universe::PC_ONLY`)
    l.add(Form::Pc, "pressed/symbols", &["ui:symboles".to_string()], |a| {
        a.symboles_volet();
        laid_out(a);
    })
    .follow = Some(super::windows::symbols);
    l.both_followed(
        "pressed/indicators",
        &["ui:fx_page".to_string()],
        |a| {
            a.fx_volet();
            laid_out(a);
        },
        Some(super::windows::indicators),
    );
    // 🔔 the feed of the alerts: the volet, then its ⚙ and the row that shows the feed (`notifs::RA_FIL`)
    l.add(Form::Pc, "pressed/feed", &["panel:Alertes".to_string()], |a| {
        let button = Rect::new(270.0, 10.0, 30.0, 30.0);
        a.notifs_pop(button);
        laid_out(a);
        a.reglages_alertes_choix(5, button);
    })
    .follow = Some(super::windows::feed);
    // 📅 « Aller à »: the window over the 📅 of the bottom bar
    l.both_followed(
        "pressed/goto",
        &[id("ui", "goto")],
        |a| {
            used(a);
            a.run_action(ChartAction::GoTo);
            laid_out(a);
        },
        Some(super::windows::goto),
    );
    // 😀 the grid of the emoji, which the icon tool opens (`App::set_tool`)
    l.both_followed(
        "pressed/pop-emoji",
        &[id("ui", "pop")],
        |a| {
            a.pick_tool("icon");
            laid_out(a);
        },
        Some(super::windows::pop),
    );
    // 👥 an idea of the Communauté opened (its detail at the top of the menu): the row of the idea picked
    l.both("pressed/idea-open", &[id("ui", "idea_open")], |a| {
        let at = Rect::new(600.0, 40.0, 200.0, 10.0);
        a.idea_typed("long 150 sl 140 tp 190 essai");
        a.banner = None;
        a.open_list(ListKind::Community, at);
        laid_out(a);
        // (👥 the first idea of the list: `community::IDEA0`; the menu is built again with the detail of the idea
        // at its top, as `App::list_click` does after the pick)
        if a.community_pick(100, at) {
            let (ww, wh) = (a.win_w(), f64::from(a.area.1));
            let menu = argos_render::listmenu::ListMenu::new(a.community_menu(), vec![], at, ww, wh);
            a.ui.list = Some((ListKind::Community, menu));
        }
    });
    // ＋ the panel that adds a market, from the ＋ of the watchlist
    l.both_followed(
        "pressed/add",
        &[id("ui", "add")],
        |a| {
            tool_open(a, wl::Tool::Watchlist);
            laid_out(a);
            super::states::reach::<super::windows::Side>(a, "Add", None);
            laid_out(a);
        },
        Some(super::windows::add_panel),
    );
}

/// The column of modules at this place, with these modules only.
pub(super) fn only(a: &mut App, on: &[Module], pos: DockPos) {
    (a.watch.dock.on, a.watch.dock.pos) = (true, pos);
    for item in a.watch.dock.items.iter_mut() {
        item.on = on.contains(&item.module);
    }
}

/// 🧩 The column of modules (PC): each module alone, then with a demo account that has traded, then the column at the
/// left and under the chart.
fn modules(l: &mut List) {
    for m in Module::ALL {
        l.add(Form::Pc, &format!("module/{m:?}"), &[id_of("module", &m)], move |a| {
            only(a, &[m], DockPos::Right)
        });
        l.add(Form::Pc, &format!("traded/module/{m:?}"), &[id_of("module", &m)], move |a| {
            traded(a);
            only(a, &[m], DockPos::Right);
        });
    }
    for pos in [DockPos::Left, DockPos::Bottom] {
        let three = [Module::Book, Module::Trades, Module::Funding];
        let shows: Vec<String> = three.iter().map(|m| id_of("module", m)).collect();
        l.add(Form::Pc, &format!("modules/{pos:?}"), &shows, move |a| only(a, &three, pos));
    }
}

/// ☰ Each action of the Argos menu and of the sheets of tiles (`Act`), as a touch on its tile.
fn sheet_actions(l: &mut List) {
    let acts = every!(Act: Indicators, Demo, Journal, Agenda, Eco, Whales, Modules, Live, Chat, New, Save, Open, Replay,
        Tpo, Footprint, PatternsAuto, MultiChart, CreateInd, Heatmap, Backtest, Alerts, IndTemplates, ChartType,
        ObjectTree, Exchanges, ChartSettings, Timeframe, OpenPanels, Account, Params, Theme, ImportV1, AccueilLancement,
        RevoirAccueil, Raccourcis, SonAlertes, TryDemo, CreateAccount, HaveKey, Passkey);
    for act in acts {
        // (an action that opens a window of tiles: each tile pressed)
        l.both_followed(&format!("act/{act:?}"), &[], move |a| a.sheet_act(act), Some(controls::<Sheet>));
    }
}

/// ⌨️ Each action of the chart (`ChartAction`: keyboard and menus of the chart), on a chart with two drawings and an
/// alert, the mouse over the candles; then the four menus of the chart.
fn chart_actions(l: &mut List) {
    use argos_render::context_menu::Source;
    use ChartAction as C;
    let flags = every!(Flag: LastPriceLabel, LastPriceLine, Grid, SymbolName, PrevCloseLabel, PrevCloseLine, HighLowLabel,
        HighLowLine, HighLowChart, Countdown, PlusButton, SessionBreaks, EconEvents, EconNews, PropFirmWindows, MainOnly,
        TpoLetters, TpoBlocks, BidAskLabel, BidAskLine);
    let modes = every!(ScaleMode: Normal, Percent, Indexed, Log);
    let mut actions = vec![
        C::ResetView,
        C::ResetTime,
        C::ZoomIn,
        C::ZoomOut,
        C::ScrollLeft,
        C::ScrollRight,
        C::ScrollLeftFast,
        C::ScrollRightFast,
        C::GoStart,
        C::GoEnd,
        C::ShowRange(40, 120),
        C::CenterOn(100),
        C::ResetPrice,
        C::AutoScale,
        C::LockRatio,
        C::Invert,
        C::TogglePercent,
        C::ToggleLog,
        C::SetTimezone("America/New_York"),
        C::TpoBlock(1800),
        C::TpoVa(70),
        C::TpoIb(3600),
        C::GoTo,
        C::WatchPrev,
        C::WatchNext,
        C::Reload,
        C::NextStyle,
        C::PrevStyle,
        C::Undo,
        C::Redo,
        C::CloseMenus,
        C::HLineAtMouse,
        C::HLine(200.0),
        C::AddAlert(240.0),
        C::Brouillon(240.0, true),
        C::Brouillon(240.0, false),
        C::CopyPrice(200.0),
        C::ClearDrawings,
        C::CopyDrawing,
        C::PasteDrawing,
        C::ObjectTree,
        C::PleinEcran,
    ];
    actions.extend(modes.map(C::Mode));
    actions.extend(flags.map(C::Toggle));
    complete("chart action", universe::CHART_ACTIONS, &actions, universe::chart_action_name);
    for action in actions {
        l.both(&format!("chart/{action:?}"), &[], move |a| {
            used(a);
            let (x, y) = middle(a);
            a.mouse_moved(PhysicalPosition::new(x, y));
            a.run_action(action);
        });
    }
    // the menus of the chart: right-click (long press) on the candles, on a scale, ⚙ and ＋ of the price scale
    for source in every!(Source: PriceAxis, TimeAxis, Chart, Plus) {
        l.both(&format!("menu/{source:?}"), &[id_of("menu", &source)], move |a| {
            used(a);
            let (x, y) = middle(a);
            a.mouse_moved(PhysicalPosition::new(x, y));
            a.open_menu(source, if source == Source::Plus { Anchor::Below(y) } else { Anchor::Cursor(x, y) });
        });
    }
}

/// 🖥 Each button of the PC's top bar (`TopClick`).
fn top_bar(l: &mut List) {
    use TopClick as T;
    let button = Rect::new(600.0, 8.0, 36.0, 30.0);
    let menus = every!(TopMenu: Argos, Layouts, Lang, Settings, Split, App);
    let mut clicks = vec![
        (String::new(), T::None),
        (String::new(), T::ReplierBandeaux),
        (String::new(), T::Recherche),
        (String::new(), T::Style(button)),
        (String::new(), T::Indicateurs(button)),
        (String::new(), T::Alertes(button)),
        (String::new(), T::Partager),
        (String::new(), T::Gardien),
        (String::new(), T::Support),
        (String::new(), T::Changed),
        (String::new(), T::Open("BTC".into())),
        (String::new(), T::Symbol),
        (String::new(), T::Home),
        (String::new(), T::Intervals(button)),
        ("-1h".to_string(), T::Interval(3600)),
    ];
    clicks.extend(menus.map(|m| (format!("-{m:?}"), T::Menu(m, button))));
    complete("top bar", universe::TOP_CLICKS, &clicks, |(_, c)| universe::top_click_name(c));
    for (which, click) in clicks {
        let name = format!("top/{}{which}", universe::top_click_name(&click));
        // (the menus that are windows: each of their rows clicked)
        let pressed: Option<fn(&mut App) -> Next> = match click {
            T::Menu(TopMenu::Argos | TopMenu::App, _) => Some(controls::<topmenu::ArgosMenu>),
            T::Menu(TopMenu::Split, _) => Some(super::windows::split_screens),
            _ => None,
        };
        l.add(Form::Pc, &name, &[], move |a| a.top_click_do(click.clone())).follow = pressed;
    }
}

/// 🖥 Each named zone of the PC's screen: the mouse over its middle, a click there, a right-click there.
fn zones(a: &mut App) -> Next {
    let mut next: Next = vec![];
    for (k, (zone, _, r)) in a.zones_pc().into_iter().enumerate() {
        let (x, y) = (r.cx(), r.cy());
        let zone = format!("{k:02}-{}", zone.replace(' ', "_"));
        next.push((
            format!("{zone}/mouse-over"),
            Arc::new(move |a: &mut App| a.mouse_moved(PhysicalPosition::new(x, y))),
        ));
        next.push((format!("{zone}/click"), Arc::new(move |a: &mut App| clic(a, x, y))));
        next.push((
            format!("{zone}/right-click"),
            Arc::new(move |a: &mut App| {
                a.mouse_moved(PhysicalPosition::new(x, y));
                a.right_click();
            }),
        ));
    }
    next
}

/// 🖥 Each button of the left bar, clicked (the mouse first comes over the bar, which unfolds).
fn rail_buttons(a: &mut App) -> Next {
    let top = a.top_h();
    rail::layout(a.area.1 as f64)
        .into_iter()
        .map(|(r, button)| {
            let (x, y) = (r.cx(), r.cy() + top);
            let gesture: Open = Arc::new(move |a: &mut App| {
                a.mouse_moved(PhysicalPosition::new(x, y));
                clic(a, x, y);
            });
            (format!("{button:?}"), gesture)
        })
        .collect()
}

/// ⭐ Each button of the favourites bar, pressed.
fn favourites(a: &mut App) -> Next {
    let (_, buttons) = toolbar::layout(&a.tools, &argos_render::text::Text::default());
    buttons
        .into_iter()
        .map(|(r, button)| {
            let gesture: Open = Arc::new(move |a: &mut App| a.tool_click(button, r.cx(), r.cy()));
            (format!("{button:?}"), gesture)
        })
        .collect()
}

/// 🖍 Each button of the selected drawing's bar, pressed.
fn drawing_bar(a: &mut App) -> Next {
    use argos_render::drawbar::DbBtn;
    let Some((_, bar)) = a.drawbar() else { return vec![] };
    every!(DbBtn: Handle, Template, LineColor, TextColor, Width, Style, Settings, Alert, Lock, Trash, More)
        .into_iter()
        .filter_map(|button| {
            let r = bar.at(button)?;
            let gesture: Open = Arc::new(move |a: &mut App| {
                a.drawbar_click(r.cx(), r.cy());
            });
            Some((format!("{button:?}"), gesture))
        })
        .collect()
}

/// 🖥 The screen by its parts: the PC's zones and its left bar, the favourites bar, the bar of a selected drawing;
/// then Zen.
fn pc_screen(l: &mut List) {
    l.add(Form::Pc, "zone", &[], |a| {
        used(a);
        a.toggle_indicator(Ind::Rsi);
    })
    .follow = Some(zones);
    l.add(Form::Pc, "rail", &[], used).follow = Some(rail_buttons);
    l.both_followed("favourites", &[], used, Some(favourites));
    l.both_followed("drawing-bar", &[], used, Some(drawing_bar));
    // the mouse over the folded left bar: it unfolds
    l.add(Form::Pc, "rail-unfolded", &[id("ui", "rail_survol")], |a| {
        a.mouse_moved(PhysicalPosition::new(14.0, 400.0))
    });
    // Zen, and the mouse at the top of the screen in Zen: the top bar comes back
    l.both("zen", &[id("ui", "zen")], App::toggle_zen);
    // 🪟 every layout of the split screens, in Zen: the chart is as wide as the window there
    for key in super::windows::layouts() {
        l.add(Form::Pc, &format!("split-zen/{key}"), &[id("ui", "zen")], move |a| {
            a.split.layout = key.clone();
            a.toggle_zen();
        });
    }
    // 🪟 with the bars around the chart, each layout on every form, the candles of its cells arrived; then, in a
    // layout of two columns and in one of two rows, each button of the navigation pill of a cell pressed
    for key in super::windows::layouts() {
        for form in Form::ALL.into_iter().chain(Form::TABLET) {
            let layout = key.clone();
            let split = l.add(form, &format!("split/{key}"), &[], move |a| {
                a.split.layout = layout.clone();
                a.sync_panes();
                super::answers::cells(a);
            });
            split.follow = matches!(key.as_str(), "2c" | "2r").then_some(cell_navigation as fn(&mut App) -> Next);
        }
    }
    l.add(Form::Pc, "zen-top", &[id("ui", "zen"), id("ui", "zen_haut")], |a| {
        a.toggle_zen();
        laid_out(a);
        a.mouse_moved(PhysicalPosition::new(700.0, 3.0));
    });
}

/// 🪟 Each button of the navigation pill of the first cell of a split screen (not the main chart's), where the app
/// draws it: the PC shows the pill once the mouse is there. A button the pill has outside a narrow cell (📱 the
/// first one, in a layout of two columns) is not there to press.
fn cell_navigation(a: &mut App) -> Next {
    let cells = argos_render::splits::cells(&a.split.layout);
    let (Some(cell), false) = (cells.get(1), a.panes.is_empty()) else {
        return vec![];
    };
    let (width, height) = (a.area.0 as f64, a.area.1 as f64);
    let (x, y) = (a.rw() + cell[0] * width, a.top_h() + cell[1] * height);
    let (view, _) = a.pane_view_de(0, &a.pane_bars(0), (cell[2] * width).round(), (cell[3] * height).round(), None);
    let buttons = argos_render::navbar::centers(&view, view.plot_h(), false);
    let pressed = |(k, (cx, cy)): (usize, (f64, f64))| {
        let gesture: Open = Arc::new(move |a: &mut App| press(a, x + cx, y + cy));
        (format!("Navigation({k})"), gesture)
    };
    let inside = |(_, (cx, _)): &(usize, (f64, f64))| (0.0..=view.plot_w()).contains(cx);
    buttons.into_iter().enumerate().filter(inside).map(pressed).collect()
}

/// A touch on a zone of the phone's main screen (where the zone is, or the default place the plan of the app uses).
pub(super) fn touch(a: &mut App, hit: CellHit) {
    let zones = argos_render::ecrans::zones(a.win_w(), a.cell_h(), &a.cell_etat());
    let zone = zones.iter().find(|(_, h)| *h == hit).map_or(Rect::new(0.0, 0.0, a.win_w(), 46.0), |(r, _)| *r);
    a.cell_click(hit, zone);
}

/// 📱 Each zone of the phone's main screen (`CellHit`), touched; the wheels of the pill, the pill folded, the finger's
/// crosshair.
fn phone_zones(l: &mut List) {
    use argos_render::phonebar::Icone;
    use CellHit as H;
    let icons = every!(Icone: Trading, Demo, PropFirm, Baleines, Modules, Script, Academie, Reglages, Communaute, Macro,
        Marketplace);
    let mut hits = vec![
        H::Accueil,
        H::Argos,
        H::MiseEnPage,
        H::Alertes,
        H::Disposition,
        H::Partager,
        H::Recherche,
        H::Plan,
        H::Demo,
        H::Flux,
        H::Symbole,
        H::SymbolePrec,
        H::SymboleSuiv,
        H::Unite,
        H::UnitePrec,
        H::UniteSuiv,
        H::Fx,
        H::Ai,
        H::Autres,
        H::Dessins,
        H::Trading,
        H::Plus,
        H::Replier,
        H::Rouvrir,
    ];
    hits.extend(icons.map(H::Icone));
    complete("phone zone", universe::CELL_HITS, &hits, universe::cell_hit_name);
    for hit in hits {
        // (the sheets that are windows: each of their rows and tiles touched)
        let pressed: Option<fn(&mut App) -> Next> = match hit {
            H::Argos => Some(controls::<topmenu::ArgosMenu>),
            H::Disposition => Some(super::windows::split_screens),
            _ => None,
        };
        l.add(Form::Phone, &format!("touch/{hit:?}"), &[], move |a| touch(a, hit)).follow = pressed;
    }
    // (a finger held on the symbol, on the unit of time: its wheel)
    for hit in [H::Symbole, H::Unite] {
        l.add(Form::Phone, &format!("wheel/{hit:?}"), &[id("ui", "roue")], move |a| {
            let zones = argos_render::ecrans::zones(a.win_w(), a.cell_h(), &a.cell_etat());
            if let Some((r, _)) = zones.iter().find(|(_, h)| *h == hit) {
                a.mouse_moved(PhysicalPosition::new(r.cx(), r.cy()));
                a.roue_poser(hit, *r);
            }
        });
    }
    // (the pill folded: the setting kept in interface-2.json, applied as at startup; a wide window reads it too)
    l.both("pill-folded", &[id("ui", "pilule_repliee")], |a| {
        let folded = crate::actions::Interface2 {
            pilule_repliee: true,
            ..Default::default()
        };
        a.appliquer_cadre(&folded);
    });
    l.add(Form::Phone, "crosshair", &[id("ui", "croix")], |a| {
        let (x, y) = middle(a);
        a.mouse_moved(PhysicalPosition::new(x, y));
        a.croix_poser();
    });
    // (⚡ a Bitfinex perpetual: its Funding pill at rest, then its card opened by a touch on the pill)
    l.add(Form::Phone, "funding-pill", &[], |a| {
        perp_bitfinex(a);
    });
    l.add(Form::Phone, "funding-card", &[id("ui", "funding_carte")], |a| {
        perp_bitfinex(a);
        if let Some((_, z)) = a.funding_zones() {
            toucher(a, z.pilule.cx(), z.pilule.cy());
        }
    });
}

/// ⚡ The chart on Bitfinex's XRPUSDT perpetual, with its `status/deriv` answer of 23 Nov 2026 (the PC's row reads it).
fn perp_bitfinex(a: &mut App) {
    a.chart.source = Source::Live {
        exchange: Exchange::Bitfinex,
        base: "XRP".into(),
    };
    let s = argos_data::exchanges::bitfinex::parse_status(
        r#"[["tXRPF0:USTF0",1795454314000,null,0.97122,0.97145,null,59332707.22187445,null,1795478400000,-0.00001989,1500,null,0.0001,null,null,0.971198,null,null,2274368.17012306]]"#,
    )
    .ok()
    .flatten()
    // (the next payment 5 h 49 min 44 s after the frozen clock, as in the mock-up: never the day's time)
    .map(|mut s| {
        s.next_funding_ms = (now_s() + 20_984) * 1000;
        s
    });
    a.feeds.bitfinex_statut = Some(crate::actions::carnets::StatutBitfinex {
        base: "XRP".into(),
        statut: s,
        ..Default::default()
    });
    a.refresh_top();
}

/// 📱 Each zone of the open sheet of the pill, touched (its tiles, its tabs, its card, its foot).
fn tiles(a: &mut App) -> Next {
    let (w, h) = a.pfeuille_fenetre();
    let Some(sheet) = a.pfeuille_vue() else { return vec![] };
    sheet
        .zones(&argos_render::text::Text::default(), w, h)
        .into_iter()
        .enumerate()
        .map(|(k, (zone, touched))| {
            let gesture: Open = Arc::new(move |a: &mut App| a.pfeuille_toucher(zone.cx(), zone.cy()));
            (format!("{k:02}-{touched:?}"), gesture)
        })
        .collect()
}

/// 📱 Each sheet of the phone's pill (`PKind`), then a touch on each of its zones.
fn phone_sheets(l: &mut List) {
    let mut kinds = vec![PKind::Argos, PKind::Dessins, PKind::Trading, PKind::Reglages, PKind::Croix(200.0)];
    kinds.extend(crate::actions::pilule::DUOS.iter().map(|duo| PKind::Duo(duo.0)));
    complete("pill sheet", universe::PILLS, &kinds, universe::pill_name);
    for kind in kinds {
        let name = match &kind {
            PKind::Duo(duo) => format!("sheet/Duo-{duo}"),
            other => format!("sheet/{}", universe::pill_name(other)),
        };
        let shows = [id("pill", universe::pill_name(&kind)), id("ui", "pfeuille")];
        l.add(Form::Phone, &name, &shows, move |a| a.pfeuille_ouvrir(kind.clone())).follow = Some(tiles);
    }
    // 🎯 (a strategy on the chart: the sheet « Argos » has one more tile, the first of its tools, touched)
    let shows = [id("pill", universe::pill_name(&PKind::Argos)), id("ui", "pfeuille")];
    let with_strategy = |a: &mut App| {
        super::states::strategy(a);
        a.pfeuille_ouvrir(PKind::Argos);
    };
    l.add(Form::Phone, "sheet/Argos/strategy", &shows, with_strategy).follow = Some(|a| {
        let strategy = |(name, _): &(String, Open)| name.to_lowercase().contains("strategie");
        tiles(a).into_iter().filter(strategy).collect()
    });
}

/// 📋 Each row of the open list menu that does something, clicked: its line, its star, its ✕. A menu longer than its
/// window is scrolled first, a page at a time, to reach the rows further down.
pub(super) fn rows(a: &mut App) -> Next {
    const PAGES: usize = 12;
    let Some((_, menu)) = &a.ui.list else { return vec![] };
    let r = menu.rect;
    let page = (r.h / argos_render::listmenu::ROW - 1.0).floor().max(1.0);
    // (what a click does is asked of a copy of the menu, every few points: the middle of each answer is its row)
    let mut found: Vec<(MenuClick, usize, [f64; 4])> = vec![];
    let mut probe = menu.clone();
    for scrolled in 0..PAGES {
        let mut y = r.y + 2.0;
        while y < r.y + r.h {
            let mut x = r.x + 4.0;
            while x < r.x + r.w {
                let click = probe.clone().click(x, y);
                if !matches!(click, MenuClick::Outside | MenuClick::Inside) {
                    match found.iter_mut().find(|(c, ..)| *c == click) {
                        Some((_, p, b)) if *p == scrolled => *b = [b[0].min(x), b[1].min(y), b[2].max(x), b[3].max(y)],
                        Some(_) => {}
                        None => found.push((click, scrolled, [x, y, x, y])),
                    }
                }
                x += 12.0;
            }
            y += 5.0;
        }
        let before = probe.scroll;
        probe.wheel(-page);
        if probe.scroll == before {
            break;
        }
    }
    found
        .into_iter()
        .map(|(click, scrolled, b)| {
            let (x, y) = ((b[0] + b[2]) / 2.0, (b[1] + b[3]) / 2.0);
            let gesture: Open = Arc::new(move |a: &mut App| {
                if scrolled > 0 {
                    a.mouse_moved(PhysicalPosition::new(r.cx(), r.cy()));
                    a.mouse_wheel(winit::event::MouseScrollDelta::LineDelta(0.0, -(page * scrolled as f64) as f32));
                }
                press(a, x, y);
            });
            (format!("{click:?}"), gesture)
        })
        .collect()
}

/// A press on the row of the open list menu whose name contains `label` (the menu is scrolled to it first).
pub(super) fn row(a: &mut App, label: &str) {
    let Some((_, menu)) = &a.ui.list else { panic!("no list menu is open") };
    let named = menu.groups.iter().flat_map(|group| &group.items).find(|item| item.label.contains(label));
    let wanted = named.map(|item| MenuClick::Pick(item.id));
    let (r, mut probe) = (menu.rect, menu.clone());
    for _ in 0..12 {
        let mut y = r.y + 2.0;
        while y < r.y + r.h {
            if Some(probe.clone().click(r.cx(), y)) == wanted {
                if let Some((_, menu)) = a.ui.list.as_mut() {
                    menu.scroll = probe.scroll;
                }
                return press(a, r.cx(), y + 2.0);
            }
            y += 4.0;
        }
        probe.wheel(-3.0);
    }
    panic!("no row « {label} » in the open menu");
}

/// Opens a list menu of this kind on a chart with two drawings and an alert, with what the menu is about (an
/// indicator selected, a second layout, a split screen…). The alert's id in `kind` is replaced by the app's.
pub(super) fn list_open(a: &mut App, kind: ListKind) {
    use crate::actions::layouts::{Layout, Layouts, FICHIER};
    used(a);
    let at = corner(a);
    match kind {
        ListKind::AlertLine(_) => a.open_list(ListKind::AlertLine(alert(a)), at),
        ListKind::SonAlerte(_) => a.open_list(ListKind::SonAlerte(alert(a)), at),
        ListKind::Section => {
            a.ui.section = Some("Crypto".into());
            a.open_list(kind, at);
        }
        ListKind::IndPlus => {
            a.toggle_indicator(Ind::Rsi);
            a.ind.sel = Some(Ok(Ind::Rsi));
            a.open_list(kind, at);
        }
        ListKind::PaneIntervals(_) => {
            a.split.layout = "4g".into();
            a.sync_panes();
            a.open_list(kind, at);
        }
        ListKind::Community => {
            a.idea_typed("long 150 sl 140 tp 190 essai");
            a.banner = None;
            a.open_list(kind, at);
        }
        ListKind::Curseurs => a.open_rail_list(kind, at),
        // 🎯 (the menu of the pill of the Stratégie volet, pressed: the two strategies on the chart. 🖥 Today the
        // app closes the volet behind this menu, and it stays closed once a strategy is chosen.)
        ListKind::Strategies => {
            super::states::strategies(a);
            tool_open(a, wl::Tool::Strategie);
            laid_out(a);
            super::states::reach::<super::windows::Side>(a, "Strategie(Choisir", None);
        }
        // 📖 (a pill of the order-book volet pressed, on the DOM tab for « Échangé »: the volet stays under its menu)
        ListKind::Carnet(m) => {
            if m == crate::actions::carnet::CarnetMenu::Echange {
                a.watch.prefs.carnet.onglet = argos_render::carnet::Onglet::Dom;
            }
            tool_open(a, wl::Tool::Carnet);
            laid_out(a);
            a.carnet_livre();
            a.open_list(kind, at);
        }
        // 🔔 (⚙ of the alerts panel, pressed: the panel stays under its menu)
        ListKind::ReglagesAlertes => {
            super::alerts::panel(a);
            laid_out(a);
            super::alerts::pressed(a, argos_render::alertes::AZone::Reglages);
        }
        ListKind::LangConfirm(k) => a.ask_lang(k, at),
        ListKind::AddApi => a.exchange_action(argos_render::exchanges::ExAction::Add),
        ListKind::TuileLayout(k) | ListKind::SupprimerLayout(k) => {
            let layout = |nom: &str| Layout {
                nom: nom.into(),
                ..Default::default()
            };
            let two = Layouts {
                ouverte: "Argos".into(),
                liste: vec![layout("Argos"), layout("Or")],
            };
            App::write_prefs(FICHIER, &two);
            a.top.prefs.layout_name = "Argos".into();
            a.open_accueil();
            a.menu_tuile(k, at);
            // (« Supprimer… », the last row of the tile's menu, asks first in its own little menu)
            if let (ListKind::SupprimerLayout(_), Some((_, delete))) = (kind, rows(a).pop()) {
                delete(a);
            }
        }
        _ => a.open_list(kind, at),
    }
}

/// 📋 Each list menu (`ListKind`), then each of its rows clicked.
fn lists(l: &mut List) {
    use argos_render::drawbar::DbBtn;
    use ListKind as K;
    let mut kinds = vec![
        K::DockAutres,
        K::Accueil,
        K::Curseurs,
        K::Intervals,
        K::Section,
        K::IndPlus,
        K::PaneIntervals(0),
        K::Styles,
        K::Theme,
        K::Lists,
        K::TousBourse,
        K::TousTri,
        K::Timezone,
        K::Drawing(1),
        K::AddApi,
        K::ToolMenu(None),
        K::ToolMenu(Some("trend")),
        K::LangConfirm(argos_render::i18n::CATALOG.len() - 1),
        K::Alerts,
        K::Accounts,
        K::Movers,
        K::Backtest,
        K::AiMemory,
        K::Wallet,
        K::IconBar(None),
        K::IconBar(Some(wl::Tool::Trading)),
        K::Community,
        K::Modules,
        K::TuileLayout(1),
        K::SupprimerLayout(1),
        K::AlertLine(0),
        K::SonAlerte(0),
        K::SonAlertes,
        K::Strategies,
        K::ReglagesAlertes,
    ];
    // 📖 (the four pills of the order-book volet: exchange, step, coins traded, ⚙)
    {
        use crate::actions::carnet::CarnetMenu as M;
        kinds.extend([M::Bourse, M::Pas, M::Echange, M::Reglages].map(K::Carnet));
    }
    kinds.extend(rail::group_ids().map(K::RailGroup));
    kinds.extend([rail::Btn::Magnet, rail::Btn::Lock, rail::Btn::Eye, rail::Btn::Trash].map(K::RailButton));
    kinds.extend([DbBtn::Template, DbBtn::Width, DbBtn::Style, DbBtn::More].map(|b| K::DrawBar(1, b)));
    // (of the top bar's menus, two are lists: the layouts and the languages)
    kinds.extend([TopMenu::Layouts, TopMenu::Lang].map(K::Top));
    complete("list menu", universe::LISTS, &kinds, universe::list_name);
    for kind in kinds {
        let shows = [id("list", universe::list_name(&kind)), id("ui", "list")];
        for form in Form::ALL.into_iter().filter(|form| universe::has(*form, &shows[0])) {
            l.add(form, &format!("list/{kind:?}"), &shows, move |a| list_open(a, kind)).follow = Some(rows);
        }
    }
}

/// ✏️ Each drawing tool (`tools::TOOLS`): picked (its help shows), drawn with as many points as it takes, and its
/// first point laid (the drawing in progress).
fn drawings(l: &mut List) {
    const PRICES: [f64; 7] = [150.0, 210.0, 170.0, 225.0, 190.0, 215.0, 160.0];
    for tool in argos_render::tools::TOOLS.iter() {
        l.add(Form::Pc, &format!("tool-picked/{}", tool.id), &[], move |a| a.pick_tool(tool.id));
        if tool.points == 0 {
            continue;
        }
        // (a tool of as many points as wanted: four)
        let points = if tool.points == 99 { 4 } else { tool.points as usize };
        l.add(Form::Pc, &format!("drawing/{}", tool.id), &[], move |a| {
            let pts = (0..points).map(|k| point(100 + 13 * k as i64, PRICES[k % PRICES.len()])).collect();
            a.chart.drawings.push(Drawing::new(tool.id, pts));
        });
        if tool.points > 1 {
            l.add(Form::Pc, &format!("drawing-started/{}", tool.id), &[id("app", "draft")], move |a| {
                a.pick_tool(tool.id);
                clic_chart(a, 420.0, 300.0);
                a.mouse_moved(PhysicalPosition::new(a.rw() + 620.0, a.top_h() + 380.0));
            });
        }
    }
    // (the phone lays a drawing with the finger: a trend line started)
    l.add(Form::Phone, "drawing-started/trend", &[id("app", "draft")], |a| {
        a.pick_tool("trend");
        let (x, y) = middle(a);
        toucher(a, x - 40.0, y);
    });
}

/// 🪟 The windows and boxes over the chart that no family above opens: the settings of a drawing (each tab), its
/// alert, its colours, the journal's buttons, each place of the plan of the app, the full screens, what Argos AI
/// reads, the markets panel, the boxes that ask (a reason, a key, a confirmation), an alert selected.
fn windows(l: &mut List) {
    use crate::actions::argos_ai_ecran::Ensemble;
    use crate::actions::plein_ecran::PleinEcran;
    use argos_render::journal::JClick;
    use argos_render::objdlg::{Tab, Which};
    use argos_render::plan::Lieu;
    // ⚙ the settings window of a drawing: each tab, on a drawing that has it
    for tab in every!(Tab: Style, Text, Coords, Vis, Inputs, Icon) {
        let (tool, points) = match tab {
            Tab::Inputs => ("long", 3),
            Tab::Icon => ("icon", 1),
            _ => ("trend", 2),
        };
        let settings = move |a: &mut App| {
            let pts = [point(165, 150.0), point(190, 205.0), point(190, 140.0)];
            a.chart.drawings.push(Drawing::new(tool, pts[..points].to_vec()));
            a.open_obj_dlg(0, Some(tab));
        };
        // (each control of the tab pressed; what all the tabs show, in the first one only)
        let pressed: fn(&mut App) -> Next = if tab == Tab::Style { super::windows::first_tab } else { super::windows::tab };
        l.both_followed(&format!("drawing-settings/{tab:?}"), &[id("ui", "obj_dlg")], settings, Some(pressed));
    }
    // 🔔 the alert of a drawing, a price alert edited, an alert about to be deleted
    let drawn = |a: &mut App| {
        used(a);
        a.open_draw_alert(1);
    };
    l.both_followed("drawing-alert", &[id("ui", "draw_alert")], drawn, Some(controls::<DrawAlert>));
    let edited = |a: &mut App| {
        used(a);
        a.ouvrir_edition_alerte(alert(a));
    };
    l.both_followed("alert-edited", &[id("ui", "draw_alert")], edited, Some(controls::<DrawAlert>));
    let deleted = |a: &mut App| {
        used(a);
        a.demander_suppression_alerte(alert(a), "essai 260");
    };
    l.both_followed("alert-delete", &[id("ui", "confirmer")], deleted, Some(super::alerts::answers));
    // 🎨 the colours of a drawing: its line, its stop, its target, its text
    for which in every!(Which: Line, Stop, Target, Text) {
        let colours = move |a: &mut App| {
            used(a);
            let at = corner(a);
            a.open_colours(1, which, at, which == Which::Line);
        };
        l.both_followed(
            &format!("drawing-colours/{which:?}"),
            &[id("ui", "colorpick")],
            colours,
            Some(controls::<ColorPick>),
        );
    }
    // 📓 the journal: each of its buttons (the form of a new trade shows the hour it really is: its scene is compared
    // without the form, `universe::real_clock`), and the form of a trade the demo account noted
    let journal = [
        ("New", JClick::New),
        ("ExportCsv", JClick::Export(true)),
        ("ExportJson", JClick::Export(false)),
        ("Import", JClick::Import),
        ("Secours", JClick::Secours),
    ];
    for (name, click) in journal {
        // (the volet is laid out before its button is pressed, as on a screen someone looks at: the form and the box
        // of the journal take their place from the width of the window, `App::win_w`, which counts the volet)
        let button = move |a: &mut App| {
            (a.watch.prefs.open, a.watch.prefs.panel) = (true, wl::Tool::Journal);
            laid_out(a);
            a.journal_click(click);
        };
        // (the form of a new trade: its controls that ask something of the app)
        let pressed = (click == JClick::New).then_some(super::windows::new_trade as fn(&mut App) -> Next);
        l.both_followed(&format!("journal/{name}"), &[], button, pressed);
    }
    let noted = |a: &mut App| {
        traded(a);
        (a.watch.prefs.open, a.watch.prefs.panel) = (true, wl::Tool::Journal);
        laid_out(a);
        a.journal_click(JClick::Edit(0));
    };
    l.both_followed("journal/Edit", &[id("ui", "j_form")], noted, Some(controls::<JForm>));
    // 🗺 each place the plan of the app opens (the plan is the phone's: `touch/Plan`)
    let mut places = vec![
        Lieu::Chart,
        Lieu::Outils,
        Lieu::Indicateurs,
        Lieu::Alertes,
        Lieu::Trading,
        Lieu::Watchlist,
        Lieu::Recherche,
        Lieu::Reglages,
        Lieu::Argos,
        Lieu::Appli,
        Lieu::Splits,
        Lieu::Accueil,
        Lieu::Raccourcis,
        Lieu::Simulateur,
    ];
    places.extend(argos_render::accueil::Section::ALL.map(Lieu::Section));
    for place in places {
        l.add(Form::Phone, &format!("plan/{place:?}"), &[], move |a| a.plan_ouvrir(place));
    }
    // ⛶ the full screens: the prices alone, one indicator panel alone
    l.both("full-screen/Prix", &[id("ui", "plein_ecran")], |a| a.entrer_plein_ecran(PleinEcran::Prix));
    l.both("full-screen/Panneau", &[id("ui", "plein_ecran")], |a| {
        a.toggle_indicator(Ind::Rsi);
        laid_out(a);
        a.entrer_plein_ecran(PleinEcran::Panneau(Ind::Rsi));
    });
    // 🤖 Argos AI: its panel, what it reads on the chart, its rules
    for what in every!(Ensemble: Tout, Accord, Volatilite, NiveauxJour) {
        l.both(&format!("argos-ai/{what:?}"), &[], move |a| {
            a.toggle_indicator(Ind::Rsi);
            a.lire_ensemble(what);
        });
    }
    l.both("argos-ai/panel", &[], App::ouvrir_panneau_ai);
    l.both("argos-ai/rules", &[], App::regles_expliquer);
    // 🌐 the markets panel: under its pill (PC), as a sheet (phone)
    l.add(Form::Pc, "markets", &[id("ui", "marches")], |a| {
        a.marches_ouvrir(Rect::new(20.0, 86.0, 120.0, 24.0), false)
    })
    .follow = Some(controls::<Panneau>);
    l.add(Form::Phone, "markets", &[id("ui", "marches")], |a| {
        a.marches_ouvrir(Rect::new(20.0, 86.0, 120.0, 24.0), true)
    })
    .follow = Some(controls::<Panneau>);
    // ₿ the markets against Bitcoin, with the catalogues of two exchanges that have some: the « btc » chip under
    // « ⚙ Tous les filtres » (CONTRE BTC, the most traded first), and a pair typed whole (the real pairs of each
    // exchange, then the computed one)
    for (form, sheet) in [(Form::Pc, false), (Form::Phone, true)] {
        l.add(form, "markets/against-btc", &[id("ui", "marches")], move |a| {
            btc_catalogues(a);
            a.marches_ouvrir(Rect::new(20.0, 86.0, 120.0, 24.0), sheet);
            let Some(filters) = a.ui.marches.as_ref().map(Panneau::filtres_rect) else {
                return;
            };
            a.marches_click(filters.cx(), filters.cy());
            // (the chips come under the rows: the wheel, or the finger, brings « btc » above « Tous les filtres »)
            let chip = |a: &App| a.ui.marches.as_ref().and_then(|panel| panel.puce_rect(Filtre::Quote, "btc"));
            a.mouse_moved(PhysicalPosition::new(filters.cx(), filters.y - 60.0));
            for _ in 0..40 {
                if chip(a).is_none_or(|at| at.y + at.h <= filters.y - 6.0) {
                    break;
                }
                a.mouse_wheel(winit::event::MouseScrollDelta::LineDelta(0.0, -1.0));
            }
            if let Some(at) = chip(a) {
                a.marches_click(at.cx(), at.cy());
            }
        });
        l.add(form, "markets/pair-typed", &[id("ui", "marches")], move |a| {
            btc_catalogues(a);
            a.marches_ouvrir(Rect::new(20.0, 86.0, 120.0, 24.0), sheet);
            let asked = a.ui.marches.as_mut().map(|panel| panel.taper("xrp btc"));
            if let Some(asked) = asked {
                a.marches_faire(asked);
            }
        });
    }
    // 🟠 the DÉMO band of the first row: a press opens the Simulateur, its ↺ opens it on the confirmation of the reset
    // (📐 on a tablet too since paires-btc-1, where `App::bandeau_place` puts it: `TABLET_FAMILIES`)
    for (name, reset) in [("demo-band/pressed", false), ("demo-band/reset", true)] {
        l.add(Form::Pc, name, &[id("ui", "sim")], move |a| {
            laid_out(a);
            let band = a.bandeau_rect();
            press(a, if reset { band.x + band.w - 10.0 } else { band.x + 20.0 }, band.cy());
        });
    }
    // the boxes that ask: the reason to unlock the day, the CoinMarketCap key, the wallet
    // (✏️ its two buttons pressed; its field takes the keys of the page, `answers::typed`)
    l.both_followed(
        "box/unlock-reason",
        &[id("ui", "name_box")],
        App::guard_unlock_demander,
        Some(super::windows::name_box),
    );
    l.both("box/cmc-key", &[], App::cmc_key_box);
    l.both("box/wallet", &[], |a| {
        let at = corner(a);
        a.open_wallet(at);
    });
    // 🔔 an alert selected on the chart: a click (a touch) on its line, beside the label the mouse over it shows
    l.both("alert-selected", &[id("ui", "alerte_sel")], |a| {
        used(a);
        laid_out(a);
        if let Some(scale) = a.scale_now() {
            let (x0, y0, y) = (a.rw(), a.top_h(), scale.y_of(ALERT));
            a.mouse_moved(PhysicalPosition::new(x0 + 60.0, y0 + y));
            let x = a.etiquette_alerte().map_or(60.0, |(.., label)| label.boite.x + label.boite.w + 10.0);
            a.pointer.last_chart_click = None;
            press(a, x0 + x, y0 + y);
        }
    });
    // 🎫 the ticket drawn on the chart (the position follows the ticket)
    l.both("ticket-drawn", &[id("app", "ticket_draw")], |a| {
        (a.watch.prefs.open, a.watch.prefs.panel) = (true, wl::Tool::Trading);
        a.drawing_from_ticket();
    });
    // 🧪 the backtest run on the chart: its menu again (it now has the report), each row clicked
    l.both_followed(
        "backtest-run",
        &[id("list", "Backtest")],
        |a| {
            a.run_backtest();
            a.banner = None;
            list_open(a, ListKind::Backtest);
        },
        Some(rows),
    );
    // 🔐 the 24 words of a new account (made-up ones: the app draws them at random, `account::begin`)
    l.both("box/words", &[id("session", "words_box")], |a| {
        let words = (1..=24).map(|k| format!("essai{k:02}")).collect();
        let (w, h) = (a.win_w(), a.area.1 as f64 + a.top_h());
        a.session.words_box = Some(WordsBox::new(words, w, h));
    });
    // 🔎 the search, the shortcuts, the home page, the replay, the simulator's date, the tree of objects
    l.both("search", &[id("ui", "palette")], App::open_palette);
    l.both("shortcuts", &[id("ui", "raccourcis")], App::toggle_raccourcis);
    l.both("home", &[id("ui", "accueil")], App::open_accueil);
    l.both("replay", &[id("app", "replay")], App::replay_start);
    l.both("simulator-date", &[], App::sim_replay_date);
    let tree = |a: &mut App| {
        used(a);
        a.open_tree();
    };
    l.both_followed("object-tree", &[id("ui", "obj_tree")], tree, Some(controls::<ObjTree>));
}

/// A script of the user that asks for one setting of each kind (`input.*`), in a panel under the price.
const ASKS: &str = r#"//@version=6
indicator("Essai réglages")
length = input.int(14, "Longueur", minval = 1, maxval = 200)
factor = input.float(1.5, "Facteur", step = 0.1, tooltip = "Un réglage fabriqué pour les essais")
shown = input.bool(true, "Afficher")
kind = input.string("SMA", "Moyenne", options = ["SMA", "EMA"])
note = input.string("essai", "Texte")
long = input.text_area("un essai sur une ligne", "Note")
source = input.source(close, "Source")
tint = input.color(color.orange, "Couleur")
hours = input.session("0930-1600", "Séance")
market = input.symbol("BTC", "Symbole")
unit = input.timeframe("60", "Unité de temps")
since = input.time(1700179200000, "Depuis")
level = input.price(100, "Niveau")
base = kind == "SMA" ? ta.sma(source, length) : ta.ema(source, length)
plot(shown ? base * factor : na, "essai", tint)
hline(level)
"#;

/// 🏷 The legend of the indicators, each of its buttons pressed: folded, then unfolded, on a chart that has an
/// indicator on the price whose code can be read (the { }), Argos Murs (« Cadrer »), two panels under the price
/// (↑ ↓ ▾; Argos Vague keeps its buttons in its pill), a script of the user on the price and one in a panel. On the
/// PC, the mouse over a panel shows the buttons of a folded legend. On the phone a name opens a bar of actions: the
/// one of an indicator, the one of a script, and the rows of the « ⋯ » of a script. Then the settings window the app
/// makes from the `input.*` of a script, each kind of input pressed.
fn legends(l: &mut List) {
    use argos_render::indleg::LegClick;
    let lit = |a: &mut App| {
        for i in [Ind::Sma, Ind::ArgosMurs, Ind::Rsi, Ind::ArgosVague] {
            a.toggle_indicator(i);
        }
        super::states::script(a, "Essai", WRITTEN);
        super::states::script(a, "Essai réglages", ASKS);
    };
    let unfolded = move |a: &mut App| {
        lit(a);
        a.legend_click(LegClick::Toggle);
    };
    l.both_followed("legend", &[], lit, Some(legend));
    l.both_followed("legend/unfolded", &[], unfolded, Some(legend));
    l.add(Form::Pc, "legend/panel-hover", &[], move |a| {
        lit(a);
        laid_out(a);
        let under = a.ind.out.bands(a.chart.viewport.plot_h()).first().map_or(0.0, |(_, top)| *top);
        a.mouse_moved(PhysicalPosition::new(a.rw() + 400.0, a.top_h() + under + 30.0));
    })
    .follow = Some(legend);
    let bars: [(&str, LegClick); 2] = [("indicator", LegClick::Nom(Ind::ArgosMurs)), ("script", LegClick::NomScript(0))];
    for (name, touched) in bars {
        let bar = move |a: &mut App| {
            unfolded(a);
            a.legend_click(touched);
        };
        l.add(Form::Phone, &format!("legend/bar/{name}"), &[], bar).follow = Some(legend);
    }
    l.add(Form::Phone, "legend/bar/script/more", &[id("list", "IndPlus")], move |a| {
        unfolded(a);
        a.legend_click(LegClick::NomScript(0));
        a.legend_click(LegClick::PlusScript(0));
    })
    .follow = Some(rows);
    let settings = |a: &mut App| {
        super::states::script(a, "Essai réglages", ASKS);
        a.legend_click(LegClick::ScriptSettings(0));
    };
    l.both_followed("script-settings", &[id("ui", "ind_dlg")], settings, Some(controls::<IndDialog>));
}

/// A script of the user as it was saved first: an average on the price.
const WRITTEN_FIRST: &str = "//@version=6\nindicator(\"Essai\", overlay = true)\nplot(ta.sma(close, 10), \"moyenne\", color.orange)\n";

/// The same script as it is now: two averages, and a mark where they cross.
const WRITTEN: &str = r#"//@version=6
indicator("Essai", overlay = true)
// (un script fabriqué pour les essais)
fast = ta.sma(close, 10)
slow = ta.sma(close, 30)
plot(fast, "rapide", color.orange)
plot(slow, "lente", color.blue, 2)
plotshape(ta.crossover(fast, slow), "croisement", shape.triangleup, location.belowbar, color.green)
"#;

/// ✎ The editor of scripts, each of its controls pressed: on a script of the chart that was saved twice (its menu
/// has two versions to take back), on a script never saved (« Nouveau script »), on the code of an indicator of the
/// app, which is only read (the { } of the legend). The editor that waits for a script to paste (a test has no
/// clipboard to paste from). And its Doc (📖) in place of the code, each of its controls pressed.
fn editors(l: &mut List) {
    use argos_render::panels::ScriptOpen;
    let shows = [id("ui", "editor")];
    let pressed = Some(super::windows::editor as fn(&mut App) -> Next);
    let saved = |a: &mut App| {
        super::states::script(a, "Essai", WRITTEN);
        a.scripts[0].versions = vec![(START + 190 * 1800, WRITTEN.into()), (START + 150 * 1800, WRITTEN_FIRST.into())];
        a.editor_on(Some(0), None);
    };
    l.both_followed("editor/saved", &shows, saved, pressed);
    l.both_followed("editor/new", &shows, |a| a.open_editor(ScriptOpen::New), pressed);
    let read = |a: &mut App| {
        a.toggle_indicator(Ind::Sma);
        a.legend_click(argos_render::indleg::LegClick::Code(Ind::Sma));
    };
    l.both_followed("editor/read-only", &shows, read, pressed);
    l.both("editor/import", &shows, |a| a.open_editor(ScriptOpen::Import));
    l.both_followed("editor/doc", &shows, doc_open, Some(super::windows::doc as fn(&mut App) -> Next));
    l.add(Form::Phone, "editor/doc/last-part", &shows, |a| {
        doc_open(a);
        laid_out(a);
        super::windows::doc_last_part(a);
    });
}

/// 📖 The Doc of the editor of scripts, opened as its button of the editor's header asks, on a new script. (The button
/// itself is pressed by `editor/new/BtnHelp`. 📱 The header leaves it out when the name of the script is too long for
/// the window: asked of the app here, the Doc opens whatever the header has room for.)
pub(super) fn doc_open(a: &mut App) {
    use argos_render::editor::{EdBtn, EdClick};
    a.open_editor(argos_render::panels::ScriptOpen::New);
    a.editor_click(EdClick::Btn(EdBtn::Help));
}

/// 🫙 The app of a first launch, before any data arrived (a file of candles, no store, « Chargement… »): at rest, each
/// tool, each module of the column.
fn empty(l: &mut List) {
    for form in Form::ALL {
        l.add(form, "empty/rest", &[], |_| {}).empty = true;
        // (🎯 the Stratégie volet is in the bar with a strategy on the chart only: never at a first launch)
        for tool in universe::tools().into_iter().filter(|tool| *tool != wl::Tool::Strategie) {
            l.add(form, &format!("empty/tool/{tool:?}"), &[], move |a| tool_open(a, tool)).empty = true;
        }
    }
    for m in Module::ALL {
        l.add(Form::Pc, &format!("empty/module/{m:?}"), &[id_of("module", &m)], move |a| {
            only(a, &[m], DockPos::Right)
        })
        .empty = true;
    }
}
