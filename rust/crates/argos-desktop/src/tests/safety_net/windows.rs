//! 🪟 The windows over the chart, control by control.
//!
//! [`super::scenes`] opens every window the code lists; here each control of an open window is pressed, as a click of
//! the app (a touch on the phone), and what a press brings (a list that opens, a tab) is pressed in turn. Where the
//! controls are is not written here: a copy of the window is asked what a press does every few points
//! ([`super::states::controls`]), so a control added to a window is pressed too.
//!
//! Not pressed: the box of a key and the box of the 24 words (a scene never asks the app to keep a key or to make an
//! account); the rows of the search (the app reads its real window to place a click in them: they are reached by
//! the keys of the page, `answers::typed`).

use super::scenes::laid_out;
use super::states::{asked, controls, reach, reach_last, tried, Around, Page};
use super::Next;
use crate::*;
use argos_render::colorpick::ColorPick;
use argos_render::drawalert::{DaClick, DrawAlert};
use argos_render::editor::Editor;
use argos_render::inddlg::IndDialog;
use argos_render::journal::JForm;
use argos_render::marches::Panneau;
use argos_render::objtree::ObjTree;
use argos_render::reglages::Reglages;
use argos_render::splits;
use argos_render::support::Support;
use argos_render::text::Text;
use std::sync::Arc;
use winit::dpi::PhysicalPosition;
use winit::event::MouseScrollDelta;

/// The kind of a press on a window, from what it changes: each field and its new value, without the row or the
/// number (« pop: Some(Src(0)) » and « pop: Some(Src(1)) » are alike), a long value left out; and what the window
/// asks of the app, without what it carries.
fn alike(changed: &[(String, String)]) -> String {
    let sort = |is: &str| is.chars().take_while(|c| !c.is_ascii_digit() && *c != '"').collect::<String>();
    let one = |(path, is): &(String, String)| match path.rsplit('.').next() {
        Some("") | None => sort(is),
        Some(name) if is.len() <= 14 => format!("{name}-{}", sort(is)),
        Some(name) => name.to_string(),
    };
    changed.iter().map(one).collect::<Vec<_>>().join("-")
}

/// Notches of the app's wheel in one turn over a window that scrolls (a notch moves it by `step` points).
const NOTCHES: f64 = 8.0;

/// One turn of the app's wheel, the mouse in the middle of this window.
fn wheel_over(a: &mut App, area: Rect) {
    a.mouse_moved(PhysicalPosition::new(area.cx(), area.cy()));
    a.mouse_wheel(MouseScrollDelta::LineDelta(0.0, -NOTCHES as f32));
}

/// A window over the chart as a [`Page`]: a copy of the open one, where it is, what a press does to the copy. The
/// first and the last control of each kind are pressed, and one of the controls a press brings. With `scrolls`
/// (the points of one notch of the wheel), the window is looked at further down too; `aside`: a field of the window
/// that is no change of it; `kinds`: what makes two presses alike, where it is not [`alike`]; `deep`: how many of the
/// controls a press brings are pressed, where each of them does something else (the rows of a menu's part).
macro_rules! window {
    (@turns) => {
        0
    };
    (@turns $step:expr) => {
        3
    };
    (@deep) => {
        1
    };
    (@deep $deep:expr) => {
        $deep
    };
    (@kind $changed:ident) => {
        alike($changed)
    };
    (@kind $changed:ident, $kinds:expr) => {
        $kinds($changed)
    };
    ($type:ty, |$a:ident| $copy:expr, |$w:ident, $place:ident| $area:expr, |$p:ident, $around:ident, $x:ident, $y:ident| $press:expr
        $(, scrolls: $step:expr, |$s:ident, $dy:ident| $wheel:expr)? $(, aside: $aside:expr)? $(, kinds: $kinds:expr)? $(, deep: $deep:expr)?) => {
        impl Page for $type {
            const TURNS: usize = window!(@turns $($step)?);
            const ALIKE: usize = 2;
            const SPREAD: bool = true;
            const DEEP: usize = window!(@deep $($deep)?);
            fn copy($a: &mut App) -> Option<Self> {
                $copy
            }
            fn area(&self, $place: &Around) -> Rect {
                let $w = self;
                $area
            }
            fn press(&mut self, $around: &Around, $x: f64, $y: f64) -> String {
                let $p = self;
                $press
            }
            #[allow(unused_variables)]
            fn wheel(&mut self, _: &Around, dy: f64) {
                $(
                    let ($s, $dy) = (self, dy);
                    $wheel;
                )?
            }
            #[allow(unused_variables)]
            fn turn(&mut self, around: &Around) {
                $(Page::wheel(self, around, NOTCHES * $step);)?
            }
            fn turned(a: &mut App, around: &Around) {
                if let Some(window) = Self::copy(a) {
                    wheel_over(a, window.area(around));
                }
            }
            fn grid(_: f64) -> (f64, f64) {
                (8.0, 6.0)
            }
            fn kind(_: &str, changed: &[(String, String)]) -> String {
                window!(@kind changed $(, $kinds)?)
            }
            $(
                fn aside(path: &str) -> bool {
                    path.ends_with("scroll") || path.ends_with($aside)
                }
            )?
        }
    };
}

// 🔔 the alert of a drawing, a price alert edited (the app fits the window in its own again after a press that only
// changes the window)
window!(
    DrawAlert,
    |a| a.ui.draw_alert.as_ref().map(|(_, window)| window.clone()),
    |w, _place| w.rect,
    |w, around, x, y| {
        let asks = w.click(x, y);
        if asks == DaClick::None {
            w.refit(around.win.0, around.win.1);
        }
        asked(&asks)
    }
);

/// ⚙ The settings of a drawing: the window and the drawing it changes (where the last press was, which the window
/// keeps for its sliders, is no change).
#[derive(Clone, PartialEq, Debug)]
pub(super) struct Settings {
    window: ObjDialog,
    drawing: Drawing,
}

window!(
    Settings,
    |a| {
        let window = a.ui.obj_dlg.clone()?;
        let drawing = a.chart.drawings.get(window.index)?.clone();
        Some(Settings { window, drawing })
    },
    |w, _place| w.window.rect,
    |w, _around, x, y| asked(&w.window.click(x, y, &mut w.drawing)),
    aside: "last_x"
);

/// ⚙ The controls of the first tab of these settings, with what every tab shows (the tabs, the name of the drawing,
/// « ⋯ », the buttons at the bottom). What another tab holds is pressed in the scene of that tab ([`tab`]).
pub(super) fn first_tab(a: &mut App) -> Next {
    controls::<Settings>(a)
        .into_iter()
        .filter(|(name, _)| !name.starts_with("tab-") || !name.contains('/'))
        .collect()
}

/// ⚙ The controls of another tab of these settings: what the tab alone holds.
pub(super) fn tab(a: &mut App) -> Next {
    const EVERYWHERE: [&str; 4] = ["tab-", "focus-Some(Rename)", "pop-Some(More)", "Close("];
    controls::<Settings>(a)
        .into_iter()
        .filter(|(name, _)| !EVERYWHERE.iter().any(|common| name.starts_with(common)))
        .collect()
}

/// The input of these settings that takes what is typed, when there is one. (The window keeps it to itself: it is
/// read in what the window writes of itself.)
fn typed_in(window: &IndDialog) -> Option<usize> {
    let said = format!("{window:?}");
    said.split("edit: Some((").nth(1)?.split(',').next()?.parse().ok()
}

// ⚙ the settings of an indicator (a notch of the wheel: 42 points). A box to tick, a number, a list, a colour… each
// changes a value of the indicator, and a field takes what is typed: the kind of the input says which control was
// pressed (the window of a script of the user has the inputs of the script, not the ones of an indicator of the app)
window!(IndDialog, |a| a.ui.ind_dlg.clone(), |w, _place| w.rect, |w, _around, x, y| {
    let (was, typed) = (w.vals.clone(), typed_in(w).filter(|_| w.typing()));
    let asks = asked(&w.click(x, y, &Text::default()));
    let kind = |k: usize| match &w.script {
        Some((_, inputs)) => inputs.get(k).map(|input| input.kind.to_string()),
        None => w.ind.params().1.get(k).map(|input| format!("{:?}", input.kind).split(['(', ' ']).next().unwrap_or_default().to_string()),
    };
    let changed = was.iter().zip(&w.vals).position(|(was, is)| was != is);
    match changed.and_then(kind) {
        Some(kind) => format!("{asks}-{kind}"),
        None if asks.is_empty() && w.typing() => {
            let now = typed_in(w).filter(|now| typed != Some(*now));
            now.and_then(kind).map_or(asks, |kind| format!("typing-{kind}"))
        }
        None => asks,
    }
}, scrolls: 42.0, |w, dy| w.wheel(dy, &Text::default()), deep: 6);

/// ⚙ These settings at the tab of the styles: the last tab of the PC's window, the second one of the phone's sheet.
pub(super) fn styles(a: &mut App) {
    let mut tabs = controls::<IndDialog>(a)
        .into_iter()
        .filter(|(name, _)| name.starts_with("tab-") && !name.contains('/'));
    let style = if a.top.phone { tabs.next() } else { tabs.next_back() };
    if let Some((_, press)) = style {
        press(a);
    }
}

/// ⚙ The controls of a tab of these settings that is not the first: what the tab alone holds.
pub(super) fn inputs(a: &mut App) -> Next {
    const EVERYWHERE: [&str; 3] = ["tab-", "menu_defauts-", "Close("];
    controls::<IndDialog>(a)
        .into_iter()
        .filter(|(name, _)| !EVERYWHERE.iter().any(|common| name.starts_with(common)))
        .collect()
}

// 📓 the form of a trade of the journal
window!(JForm, |a| a.ui.j_form.clone(), |w, _place| w.rect, |w, _around, x, y| asked(&w.click(
    x,
    y,
    &Text::default()
)));

/// 📓 The controls of the form of a new trade that ask something of the app (close, save). The form itself is not
/// compared (it shows the hour it really is, `universe::real_clock`): a control that only changes the form would be a
/// scene that shows nothing; it is seen in the form of a noted trade (`journal/Edit`).
pub(super) fn new_trade(a: &mut App) -> Next {
    controls::<JForm>(a).into_iter().filter(|(name, _)| !name.ends_with("Changed")).collect()
}

// 🎨 the colours of a drawing
window!(
    ColorPick,
    |a| a.ui.colorpick.as_ref().map(|(.., palette)| palette.clone()),
    |w, _place| w.rect,
    |w, _around, x, y| asked(&w.click(x, y))
);

// 🛟 the Support
window!(Support, |a| a.ui.support.clone(), |w, _place| w.rect(), |w, _around, x, y| asked(&w.click(x, y)),
    scrolls: 40.0, |w, dy| w.wheel(dy));

// 🌐 the markets panel (its list rolls under the wheel and the finger: the chips of « Tous les filtres » are under
// its rows)
window!(Panneau, |a| a.ui.marches.clone(), |w, _place| w.rect, |w, _around, x, y| asked(&w.click(x, y)),
    scrolls: 40.0, |w, dy| w.wheel(dy));

/// 🏦 The ⚙ of the Footprint, the panel of the exchanges: what it shows (the app builds it at each image from the
/// settings of the indicator), where it is, and whether it is the phone's sheet.
#[derive(Clone, PartialEq, Debug)]
pub(super) struct FootprintPanel {
    panel: argos_render::fpmulti::Reglages,
    place: Rect,
    sheet: bool,
}

window!(
    FootprintPanel,
    |a| {
        let (panel, place) = a.fp_place(&Text::default())?;
        Some(FootprintPanel {
            panel,
            place,
            sheet: a.top.phone,
        })
    },
    |w, _place| w.place,
    |w, _around, x, y| match w.panel.click(&Text::default(), w.place.w, w.sheet, x - w.place.x, y - w.place.y) {
        argos_render::fpmulti::Clic::Rien => String::new(),
        asks => asked(&asks),
    }
);

/// 🏦 Each control of the panel of the exchanges.
pub(super) fn footprint_panel(a: &mut App) -> Next {
    controls::<FootprintPanel>(a)
}

/// 🔎 The volet of the symbols: the window, what it shows (the app builds it at each image from the watchlist and the
/// recents) and where it is. A press reads the list, never changes it: two copies are the same when their windows are.
#[derive(Clone, Debug)]
pub(super) struct Symbols {
    window: argos_render::symboles::Symboles,
    shows: argos_render::symboles::Donnees,
    place: Rect,
}

impl PartialEq for Symbols {
    fn eq(&self, other: &Self) -> bool {
        self.window == other.window && self.place == other.place
    }
}

window!(
    Symbols,
    |a| Some(Symbols {
        window: a.ui.symboles.clone()?,
        shows: a.sym_donnees(),
        place: a.sym_rect()?,
    }),
    |w, _place| w.place,
    |w, _around, x, y| {
        let (place, shows) = (w.place, w.shows.clone());
        asked(&w.window.click(x - place.x, y - place.y, place.w, place.h, &shows))
    },
    scrolls: 40.0,
    |w, dy| {
        let (place, shows) = (w.place, w.shows.clone());
        w.window.wheel(dy, place.w, place.h, &shows);
    }
);

/// 🔎 Each control of the volet of the symbols.
pub(super) fn symbols(a: &mut App) -> Next {
    controls::<Symbols>(a)
}

/// 🅘 The page of the indicators (the PC's volet, the whole window on the phone): the window, the indicators it marks,
/// and where it is. A press reads them, never changes them.
#[derive(Clone, Debug)]
pub(super) struct Indicators {
    window: argos_render::fxpage::FxPage,
    shows: argos_render::fxpage::Donnees,
    place: Option<Rect>,
    /// The size the app gives the page when it is not in the volet: the width of the window, and its height
    /// without the bar at the bottom (`App::fx_click`, `App::mouse_wheel`).
    whole: (f64, f64),
}

impl PartialEq for Indicators {
    fn eq(&self, other: &Self) -> bool {
        self.window == other.window && self.place == other.place
    }
}

window!(
    Indicators,
    |a| Some(Indicators {
        window: a.ui.fx_page.clone()?,
        shows: a.fx_donnees(),
        place: a.fx_rect(),
        whole: (a.win_w(), a.top_h() + f64::from(a.area.1) + argos_render::ecrans::dock_h()),
    }),
    |w, place| w.place.unwrap_or(Rect::new(0.0, 0.0, place.w, place.h)),
    |w, _around, x, y| {
        // (in the volet: the page's own points and its width; the whole window otherwise, as `App::fx_click` reads it)
        let (dx, dy, width) = w.place.map_or((0.0, 0.0, w.whole.0), |r| (r.x, r.y, r.w));
        let shows = w.shows.clone();
        asked(&w.window.click(x - dx, y - dy, width, &shows))
    },
    scrolls: 40.0,
    |w, dy| {
        let height = w.place.map_or(w.whole.1, |r| r.h);
        let shows = w.shows.clone();
        w.window.wheel(dy, height, &shows);
    }
);

/// 🅘 Each control of the page of the indicators.
pub(super) fn indicators(a: &mut App) -> Next {
    controls::<Indicators>(a)
}

/// 🔔 The feed of the alerts: the panel, the events it shows (the app builds them at each image) and the size the app
/// gives it (the window without the bar at the bottom).
#[derive(Clone, Debug)]
pub(super) struct Feed {
    panel: argos_render::notifs::Notifs,
    events: Vec<argos_render::notifs::Evenement>,
    place: Rect,
}

impl PartialEq for Feed {
    fn eq(&self, other: &Self) -> bool {
        self.panel == other.panel && self.place == other.place
    }
}

window!(
    Feed,
    |a| {
        let panel = a.ui.notifs.clone().filter(|n| n.fil)?;
        let height = a.top_h() + f64::from(a.area.1) + argos_render::ecrans::dock_h();
        Some(Feed {
            panel,
            events: a.notif_evenements(),
            place: Rect::new(0.0, 0.0, a.win_w(), height),
        })
    },
    |w, _place| w.place,
    |w, _around, x, y| {
        let (place, events) = (w.place, w.events.clone());
        asked(&w.panel.click(x, y, place.w, place.h, &Text::default(), &events))
    },
    scrolls: 40.0,
    |w, dy| {
        let events = w.events.clone();
        w.panel.wheel(dy, &events);
    }
);

/// 🔔 Each control of the feed of the alerts.
pub(super) fn feed(a: &mut App) -> Next {
    controls::<Feed>(a)
}

/// 📅 « Aller à »: the window and the view it is laid on (the app lays it over the 📅 of the bottom bar).
///
/// The window does not say where it is (its layout is its own), so the presses are tried in the band it stands in:
/// its width, from the top of the chart down to the bottom bar.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Goto {
    window: argos_render::goto::GoTo,
    view: argos_render::viewport::Viewport,
    zone: i64,
    /// Where the chart begins in the window: the app gives this window the points of the chart, not of the window
    /// (`pointer.mouse`).
    chart: (f64, f64),
}

/// The width « Aller à » is drawn with (`goto::W`, kept here: the window does not say it).
const GOTO_W: f64 = 360.0;

window!(
    Goto,
    |a| Some(Goto {
        window: a.ui.goto.clone()?,
        view: a.chart.viewport,
        zone: a.tz_offset_s(),
        chart: (a.rw(), a.top_h()),
    }),
    |w, _place| {
        let left = argos_render::bottombar::goto_rect(&w.view)
            .map_or((w.view.plot_w() - GOTO_W) / 2.0, |icon| icon.x.min(f64::from(w.view.width) - GOTO_W - 4.0))
            .max(4.0);
        Rect::new(w.chart.0 + left, w.chart.1 + 4.0, GOTO_W, w.view.plot_h())
    },
    |w, _around, x, y| {
        let (view, zone, chart) = (w.view, w.zone, w.chart);
        asked(&w.window.click(&view, x - chart.0, y - chart.1, zone))
    }
);

/// 📅 Each control of « Aller à ».
pub(super) fn goto(a: &mut App) -> Next {
    controls::<Goto>(a)
}

// 🎨 a window stuck under its button of the drawing bar (the colours, the width, the kind of chart, the unit of
// time, the emoji of an icon…)
window!(argos_render::toolbar::Pop, |a| a.ui.pop.clone(), |w, _place| w.rect, |w, _around, x, y| asked(
    &w.click(x, y)
));

/// 🎨 Each control of the window under a button of the drawing bar.
pub(super) fn pop(a: &mut App) -> Next {
    controls::<argos_render::toolbar::Pop>(a)
}

// ✏️ the box that asks for a name (a list, a section, a unit of time, the reason of an unlock…): its two buttons
// (its field takes the keys of the page, `answers::typed`)
window!(
    argos_render::wlmenu::NameBox,
    |a| a.ui.name_box.clone(),
    |w, _place| w.rect,
    |w, _around, x, y| asked(&w.click(x, y))
);

/// ✏️ The buttons of the box that asks for a name.
pub(super) fn name_box(a: &mut App) -> Next {
    controls::<argos_render::wlmenu::NameBox>(a)
}

// ＋ the panel that adds a market to a list: its chips and its rows (nothing in the app scrolls it today: the wheel
// over it goes to what is under it)
window!(argos_render::wlmenu::AddPanel, |a| a.ui.add.clone(), |w, _place| w.rect, |w, _around, x, y| {
    asked(&w.click(x, y, &Text::default()))
});

/// ＋ Each control of the panel that adds a market.
pub(super) fn add_panel(a: &mut App) -> Next {
    controls::<argos_render::wlmenu::AddPanel>(a)
}

/// 🏦 The chip of an exchange pressed in the panel of the exchanges (`which`: its place in the app's list,
/// `actions::fpmulti::BOURSES`).
pub(super) fn footprint_exchange(a: &mut App, which: usize) {
    reach::<FootprintPanel>(a, &format!("Bourse({which})"), None);
}

/// 🗂 The right column as a [`Page`]: its icons and the open panel (the PC's floating volet, the phone's sheet). A copy
/// of the column with all its panels, as the app holds it; and what the app says of each point a press is tried at.
#[derive(Clone, PartialEq)]
pub(super) struct Side {
    column: wl::Watchlist,
    places: Places,
    /// The height the app gives the open panel when the wheel turns over it (no panel open: no wheel).
    tall: Option<f64>,
}

/// Where the presses of a [`Side`] are tried ([`tried`]: across, down), and for each of these points, row by row,
/// what the app says: where the pointer is in the column there, and how tall the column is. The app is asked,
/// because the column is not in one piece (the icons, the panel that floats, the sheet of the phone) and each piece
/// counts from its own corner.
#[derive(Clone)]
struct Places {
    area: Rect,
    across: Arc<Vec<f64>>,
    down: Arc<Vec<f64>>,
    held: Arc<Vec<Option<[f64; 3]>>>,
}

/// (Where the column is in the window is no state of the column: two copies differ by what they hold.)
impl PartialEq for Places {
    fn eq(&self, _: &Places) -> bool {
        true
    }
}

/// (What a press changes is read in what the column writes of itself.)
impl std::fmt::Debug for Side {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.column.fmt(f)
    }
}

impl Side {
    /// Whether the app says the pointer is the column's at this point of the window: where in the column, and the
    /// column's height there (as `App::mouse_button` reads them for a click).
    fn held_at(a: &mut App, x: f64, y: f64) -> Option<[f64; 3]> {
        a.mouse_moved(PhysicalPosition::new(x, y));
        // (the card of a closed trade takes the press at its place, even hidden under the phone's Trading sheet)
        let column = a.pointer.side_mouse.filter(|_| !a.avis_sous(x, y));
        column.map(|(sx, sy)| [sx, sy, a.side_h(sx)])
    }

    /// A point of the window over the open panel (the pointer is left there), and the panel's height.
    fn over_panel(a: &mut App) -> Option<f64> {
        const TRIED: [(f64, f64); 6] = [(0.9, 0.5), (0.5, 0.75), (0.75, 0.5), (0.5, 0.5), (0.9, 0.75), (0.9, 0.25)];
        let around = Around::of(a);
        TRIED.iter().find_map(|(across, down)| {
            let [sx, _, tall] = Self::held_at(a, around.w * across, around.h * down)?;
            (a.watch.prefs.open && sx < a.watch.pw()).then_some(tall)
        })
    }
}

impl Side {
    /// Whether a press does one of these things: it changes one of these fields of the column, or asks the app
    /// something that begins so.
    fn does(changed: &[(String, String)], fields: &[&str], asks: &[&str]) -> bool {
        changed.iter().any(|(path, is)| match path.is_empty() {
            true => asks.iter().any(|ask| is.starts_with(ask)),
            false => fields.contains(&path.as_str()),
        })
    }

    /// A press on what the column shows around every panel: an icon that opens another panel or something else, the
    /// column folded, the panel closed.
    fn around_panels(changed: &[(String, String)]) -> bool {
        const FIELDS: [&str; 4] = ["prefs.panel", "prefs.open", "prefs.folded", "col_repliee"];
        const ASKS: [&str; 5] = ["Outil(", "Marketplace", "PersonnaliserBarre", "FermerFeuille", "ReplierBarre"];
        Self::does(changed, &FIELDS, &ASKS)
    }

    /// A press on the list of symbols, which a panel without a screen of its own shows too.
    fn on_list(changed: &[(String, String)]) -> bool {
        const FIELDS: [&str; 5] = ["picker", "prefs.flags", "selected", "prefs.sort", "prefs.collapsed"];
        Self::does(changed, &FIELDS, &["Add(", "Lists(", "Liste("])
    }
}

impl Page for Side {
    // (past the end of the longest panels, the keys of the exchanges and the list of the indicators: a row added to
    // one of them must not leave its last rows without a press)
    const TURNS: usize = 6;
    // (the tabs of a panel are each a screen of their own: all of them; of the rows of a list, a few)
    const ALIKE: usize = 5;
    const SPREAD: bool = true;
    const DEEP: usize = 6;
    const ONCE: bool = true;
    const EXACT: bool = true;
    fn copy(a: &mut App) -> Option<Self> {
        let (around, column, was) = (Around::of(a), a.watch.clone(), a.pointer.win_mouse);
        // roughly where the column is in the window, then each point a press is tried at there
        const ROUGH: f64 = 24.0;
        let rough = |length: f64| (0..(length / ROUGH) as usize).map(|k| (k as f64 + 0.5) * ROUGH);
        let mut edges: Option<[f64; 4]> = None;
        for (x, y) in rough(around.h).flat_map(|y| rough(around.w).map(move |x| (x, y))) {
            if Self::held_at(a, x, y).is_some() {
                let [x0, y0, x1, y1] = edges.unwrap_or([x, y, x, y]);
                edges = Some([x0.min(x), y0.min(y), x1.max(x), y1.max(y)]);
            }
        }
        let [x0, y0, x1, y1] = edges?;
        let area = Rect::new(x0 - ROUGH, y0 - ROUGH, x1 - x0 + 2.0 * ROUGH, y1 - y0 + 2.0 * ROUGH);
        let (across, down) = tried(area, &around, Self::grid(around.w), None);
        let mut held = Vec::with_capacity(across.len() * down.len());
        for (x, y) in down.iter().flat_map(|y| across.iter().map(move |x| (*x, *y))) {
            held.push(Self::held_at(a, x, y));
        }
        let tall = Self::over_panel(a);
        a.mouse_moved(PhysicalPosition::new(was.0, was.1));
        Some(Side {
            column,
            places: Places {
                area,
                across: Arc::new(across),
                down: Arc::new(down),
                held: Arc::new(held),
            },
            tall,
        })
    }
    fn area(&self, _: &Around) -> Rect {
        self.places.area
    }
    fn grid(_: f64) -> (f64, f64) {
        (12.0, 8.0)
    }
    fn press(&mut self, _: &Around, x: f64, y: f64) -> String {
        let places = &self.places;
        let (column, row) = (places.across.iter().position(|at| *at == x), places.down.iter().position(|at| *at == y));
        let held = column.zip(row).and_then(|(column, row)| places.held[row * places.across.len() + column]);
        held.map_or(String::new(), |[sx, sy, tall]| asked(&self.column.click(sx, sy, tall)))
    }
    /// (The column goes where the wheel sends it image after image: here at once, as [`Page::turned`] does.)
    fn wheel(&mut self, _: &Around, dy: f64) {
        if let Some(tall) = self.tall.filter(|_| self.column.prefs.open) {
            self.column.wheel(-dy / SIDE_NOTCH, tall);
            while self.column.glide() {}
        }
    }
    fn turn(&mut self, around: &Around) {
        Page::wheel(self, around, NOTCHES * SIDE_NOTCH);
    }
    fn turned(a: &mut App, _: &Around) {
        if Self::over_panel(a).is_some() {
            a.mouse_wheel(MouseScrollDelta::LineDelta(0.0, -NOTCHES as f32));
            while a.watch.glide() {}
        }
    }
    /// (Two presses are alike when they change the same fields and ask the same thing of the app, whatever the row
    /// or the name it carries: « IndSettings(Rsi) » and « IndSettings(Macd) », « Exchange(Key("hl")) » and
    /// « Exchange(Key("okx")) ».)
    fn kind(_: &str, changed: &[(String, String)]) -> String {
        let asks = |is: &str| is.rfind('(').map_or(is.to_string(), |carried| is[..=carried].to_string());
        let (fields, asked): (Vec<_>, Vec<_>) = changed.iter().cloned().partition(|(path, _)| !path.is_empty());
        let asked: Vec<String> = asked.iter().map(|(_, is)| asks(is)).collect();
        [alike(&fields), asked.join("-")].join("-")
    }
    /// (Where the column is scrolled to, and its scroll bar held while the button is down, are no change of it.)
    fn aside(path: &str) -> bool {
        path.ends_with("scroll") || ["scroll_to", "panel_to", "bar_grab"].contains(&path)
    }
    /// (What the column shows around every panel, and the list of symbols a panel without a screen of its own shows,
    /// are pressed in the scenes of the watchlist. A press that only says the column changed, and changes nothing a
    /// scene would show, is left out.)
    fn offered(&self, changed: &[(String, String)], brought: Option<&[(String, String)]>) -> bool {
        let watchlist = self.column.prefs.open && self.column.prefs.panel == wl::Tool::Watchlist;
        let pressed = brought.unwrap_or(changed);
        let nothing = pressed.iter().all(|(path, is)| path.is_empty() && is == "Changed");
        !nothing && (watchlist || !(Self::around_panels(pressed) || Self::on_list(pressed)))
    }
    /// (What another panel holds is pressed in its own scenes.)
    fn deepens(&self, changed: &[(String, String)]) -> bool {
        !Self::around_panels(changed)
    }
}

/// Points a notch of the wheel moves an open panel of the column by (`Watchlist::wheel`).
const SIDE_NOTCH: f64 = 56.0;

/// 🗂 Each control of the open panel of the right column (on the phone, of its sheet), pressed, and what a press
/// brings pressed in turn.
pub(super) fn panel(a: &mut App) -> Next {
    controls::<Side>(a)
}

// 🌳 the tree of objects
window!(ObjTree, |a| a.ui.obj_tree.clone(), |w, _place| w.rect, |w, around, x, y| asked(&w.click(
    &around.tree,
    x,
    y
)));

/// Each press is of its own kind: all of them are kept.
fn each(changed: &[(String, String)]) -> String {
    changed.iter().map(|(path, is)| format!("{path} {is}")).collect::<Vec<_>>().join(", ")
}

// 🪟 the menu of the split screens, the phone's sheet (each layout is a screen of its own; the app opens and leaves
// the sheet's « Synchro » itself)
window!(splits::Menu, |a| a.ui.split_menu.clone(), |w, _place| w.rect, |w, _around, x, y| {
    let asks = w.click(x, y);
    if let splits::MenuClick::Synchro(open) = asks {
        w.synchro = open;
    }
    asked(&asks)
}, scrolls: 40.0, |w, dy| w.wheel(dy), kinds: each, deep: 8);

/// Every layout of the split screens the PC's menu offers (asked of the menu itself, point by point), without the
/// grids « à toi » that give a layout the menu names.
pub(super) fn layouts() -> Vec<String> {
    const STEP: f64 = 4.0;
    let menu = splits::Menu::new(Rect::new(1000.0, 0.0, 40.0, 48.0), 1440.0, 900.0);
    let mut keys: Vec<String> = vec![];
    let (columns, rows) = ((menu.rect.w / STEP) as usize, (menu.rect.h / STEP) as usize);
    for (row, column) in (0..rows).flat_map(|row| (0..columns).map(move |column| (row, column))) {
        let (x, y) = (menu.rect.x + STEP * (column as f64 + 0.5), menu.rect.y + STEP * (row as f64 + 0.5));
        if let splits::MenuClick::Layout(key) = menu.click(x, y) {
            if !keys.iter().any(|known| splits::cells(known) == splits::cells(&key)) {
                keys.push(key);
            }
        }
    }
    keys
}

/// 🪟 The controls of the menu of the split screens (the phone's sheet). Left out: a grid « à toi » that gives a
/// layout the menu names. (The lines between the cells are drawn on whole points since paires-btc-1: every layout is
/// drawn in a test build, with the bars around the chart too.)
pub(super) fn split_screens(a: &mut App) -> Next {
    let named = layouts();
    let kept = |name: &str| match name.split('"').nth(1) {
        Some(key) if name.starts_with("Layout(") => named.iter().any(|known| known == key),
        _ => true,
    };
    controls::<splits::Menu>(a).into_iter().filter(|(name, _)| kept(name)).collect()
}

// ☰ the Argos menu and the menu of the app
window!(topmenu::ArgosMenu, |a| a.ui.argos.clone(), |w, _place| w.rect, |w, _around, x, y| asked(&w.click(x, y)),
    scrolls: 40.0, |w, dy| w.wheel(dy), deep: 16);

// the windows of tiles (panels, account)
window!(Sheet, |a| a.ui.sheet.clone(), |w, _place| w.rect, |w, _around, x, y| asked(&w.click(x, y)),
    scrolls: 40.0, |w, dy| w.wheel(dy));

// ⚙ the settings window
window!(Reglages, |a| a.ui.reglages.clone(), |w, _place| w.rect(), |w, _around, x, y| asked(&w.click(x, y)),
    scrolls: 40.0, |w, dy| w.wheel(dy));

/// ✎ The editor of scripts as a window: the editor, and where the app puts it.
#[derive(Clone, Debug)]
pub(super) struct Code {
    editor: Editor,
    place: Rect,
}

impl PartialEq for Code {
    /// (field by field: the editor also keeps, in a cell, the rows of its last image, which no press changes)
    fn eq(&self, other: &Self) -> bool {
        let small = |e: &Editor| (e.cur, e.top, e.slot, e.err_line, e.menu, e.armed, e.lecture_seule, e.tout);
        let (a, b) = (&self.editor, &other.editor);
        small(a) == small(b) && a.extra == b.extra && a.name == b.name && a.lines == b.lines && a.log == b.log && self.place == other.place
    }
}

// ✎ the editor of scripts (a notch of the app's wheel moves its code by 3 lines); under the menu of its name, each
// of its six rows
window!(Code, |a| Some(Code { editor: a.ui.editor.clone()?, place: a.ed_rect() }), |w, _place| w.place, |w, _around, x, y| {
    asked(&w.editor.click(&Text::default(), x - w.place.x, y - w.place.y, w.place.w, w.place.h))
}, scrolls: 3.0, |w, dy| w.editor.scroll(dy.round() as i64), deep: 6);

/// 📖 The Doc of the editor of scripts as a window: the editor with its Doc in place of the code, and where the app
/// puts it.
#[derive(Clone)]
pub(super) struct DocPage {
    editor: Editor,
    place: Rect,
}

impl DocPage {
    /// What the Doc writes of itself, up to the block last copied: what comes after is no state a scene shows (a copy
    /// and its notice read the real clock) or changes (the place of its last image, the layout it keeps, long to
    /// write).
    fn told(&self) -> String {
        const END: &str = ", copie:";
        struct Start(String);
        impl std::fmt::Write for Start {
            fn write_str(&mut self, said: &str) -> std::fmt::Result {
                self.0.push_str(said);
                if self.0.contains(END) {
                    Err(std::fmt::Error)
                } else {
                    Ok(())
                }
            }
        }
        let mut start = Start(String::new());
        let _ = std::fmt::write(&mut start, format_args!("{:?}", self.editor.doc));
        format!("{} }}", start.0.split(END).next().unwrap_or_default())
    }

    /// The Doc lays its open tab out, at the width of this window, and keeps it: the copies of this one have it too.
    /// (A copy pressed on the other tab knows the first one only: each press then tried on it, in its text or on its
    /// list of parts, would lay the whole tab out again, every line of it measured.)
    fn laid(&self) {
        // (the top of the Doc in the editor is no part of what it keeps)
        self.editor.doc.nombre_trouves(&Text::default(), self.place.w, self.place.h, 0.0);
    }
}

impl PartialEq for DocPage {
    fn eq(&self, other: &Self) -> bool {
        self.editor.doc_ouverte == other.editor.doc_ouverte && self.place == other.place && self.told() == other.told()
    }
}

impl std::fmt::Debug for DocPage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DocPage {{ open: {}, doc: {} }}", self.editor.doc_ouverte, self.told())
    }
}

// 📖 the Doc of the editor (a notch of the app's wheel moves it by 40 points); under « ☰ Parties » on the phone, the
// rows of its sheet
window!(
    DocPage,
    |a| {
        let editor = a.ui.editor.clone().filter(|editor| editor.doc_ouverte)?;
        Some(DocPage { editor, place: a.ed_rect() })
    },
    |w, _place| w.place,
    |w, _around, x, y| {
        let asks = asked(&w.editor.click(&Text::default(), x - w.place.x, y - w.place.y, w.place.w, w.place.h));
        w.laid();
        asks
    },
    scrolls: 40.0,
    |w, dy| w.editor.doc.wheel(dy),
    deep: 2
);

/// 📖 Each control of the Doc of the editor: its two tabs, its search, its parts (📱 their sheet), the wheel, the way
/// back to the code and the ✕. Not its copies (« ⧉ Copier », « ✨ Copier pour l'IA »: their notice reads the real
/// clock, and a test has no clipboard), nor the other buttons of the editor's header, pressed in the scenes of the
/// code ([`editor`]).
pub(super) fn doc(a: &mut App) -> Next {
    const KEPT: [&str; 2] = ["Btn(Help)", "Btn(Close)"];
    // (« ‹ Code » brings the code back, on the phone in a window of another size than the Doc's: what a press does
    // there is in the scenes of the code)
    let own = |name: &str| !name.contains("Doc(Copier") && !name.starts_with("open-false/");
    let kept = |name: &str| own(name) && (!name.contains("Btn(") || KEPT.iter().any(|kept| name.ends_with(kept)));
    controls::<DocPage>(a).into_iter().filter(|(name, _)| kept(name)).collect()
}

/// 📖 The search of the Doc takes the keyboard: a press on its field.
pub(super) fn doc_search(a: &mut App) {
    reach::<DocPage>(a, "saisie-true", None);
}

/// 📱 The last part of the Doc's open tab, chosen in the sheet of « ☰ Parties » (of the rows of the sheet, [`doc`]
/// presses the first one, where the Doc already is).
pub(super) fn doc_last_part(a: &mut App) {
    reach::<DocPage>(a, "feuille-true", None);
    laid_out(a);
    reach_last::<DocPage>(a, "feuille-false-vise");
}

/// One step in the editor: a press on the control of this name, or a text typed (the hidden field of the page).
#[derive(Clone, Copy)]
enum Step {
    Press(&'static str),
    Type(&'static str),
}

/// These steps done one after the other in the open editor; the app draws itself between two of them.
fn walk(a: &mut App, steps: &[Step]) {
    for (k, step) in steps.iter().enumerate() {
        if k > 0 {
            laid_out(a);
        }
        match *step {
            Step::Press(name) => _ = reach::<Code>(a, name, None),
            Step::Type(text) => a.paste_text(text.into()),
        }
    }
}

/// ✎ The controls of the editor of scripts, then what the app itself brings after a row of the menu of its name,
/// pressed in turn: the box of a name validated (a copy made; the script renamed, a word typed after its name), the
/// second press that deletes, the oldest version of the history taken back. A code that is only read ({ }) has the
/// copy alone; a script never saved has no version.
pub(super) fn editor(a: &mut App) -> Next {
    use Step::{Press, Type};
    const MENU: Step = Press("menu-true");
    let Some(editor) = a.ui.editor.clone() else { return vec![] };
    let versions = editor.slot.and_then(|slot| a.scripts.get(slot)).map_or(0, |script| script.versions.len());
    let mut paths = vec![vec![MENU, Press("menu-false-Item(Copy)"), Press("PromptOk")]];
    if !editor.lecture_seule {
        paths.push(vec![MENU, Press("menu-false-Item(Rename)"), Type(" bis"), Press("PromptOk")]);
        paths.push(vec![MENU, Press("Item(Delete)"), Press("menu-false-Item(Delete)")]);
        if versions > 0 {
            let oldest: &'static str = ["Version(0)", "Version(1)"][(versions - 1).min(1)];
            paths.push(vec![MENU, Press("menu-false-Item(History)"), Press(oldest)]);
        }
    }
    let mut next = controls::<Code>(a);
    for steps in paths {
        let name: Vec<&str> = steps.iter().map(|step| if let Press(name) = step { *name } else { "typed" }).collect();
        next.push((name.join("/"), Arc::new(move |a: &mut App| walk(a, &steps))));
    }
    next
}
