//! 🕸 The states of a screen that no list of the code names: what a script of the user can draw, and each control of
//! the full pages pressed.
//!
//! The scenes of [`super::scenes`] open every screen the code lists; these ones bring a screen to the states it only
//! reaches with its own data or after a press: every shape, label, line, box and table of Argos Script; the home
//! page, the three screens of a first launch and each place they lead to; the guides, the tabs and the amounts of the
//! portfolio; the sheet of an indicator of the Marketplace; a pupil of the Academy, a lesson and its quiz; the tabs
//! of the backtest report.

use super::scenes::{laid_out, list_open, press, row, tool_open, traded, List};
use super::universe::id;
use super::{Form, Next};
use crate::*;
use argos_render::academy::{lecons, AcademyPage, Progress, View};
use argos_render::accueil::{AccueilPage, Carte};
use argos_render::btpage::BtPage;
use argos_render::indicators::Ind;
use argos_render::marketplace::MkPage;
use argos_render::portfolio::PfPage;
use argos_render::text::Text;
use std::sync::Arc;

/// Every scene of this file.
pub(super) fn scenes(l: &mut List) {
    scripts(l);
    pages(l);
}

/// 🔺 Every shape and place of `plotshape`, a letter of `plotchar`, the arrows of `plotarrow`: on the last candles
/// (the phone shows them too), with a text of one and of two lines, in each size.
const SHAPES: &str = r#"//@version=6
indicator("Essai formes", overlay = true)
k = last_bar_index - bar_index
plotshape(k == 30, "a", shape.triangleup, location.belowbar, color.green, text = "deux\nlignes", size = size.tiny)
plotshape(k == 30, "b", shape.triangledown, location.abovebar, color.red, text = "deux\nlignes", size = size.small)
plotshape(k == 26, "c", shape.arrowup, location.belowbar, color.lime, size = size.normal)
plotshape(k == 26, "d", shape.arrowdown, location.abovebar, color.maroon, size = size.large)
plotshape(k == 22, "e", shape.circle, location.top, color.blue, text = "haut", size = size.huge)
plotshape(k == 22, "f", shape.square, location.bottom, color.orange, text = "bas")
plotshape(k == 18 ? low - 4 : na, "g", shape.diamond, location.absolute, color.purple, text = "au prix")
plotshape(k == 14, "h", shape.cross, location.abovebar, color.teal)
plotshape(k == 14, "i", shape.xcross, location.belowbar, color.fuchsia)
plotshape(k == 10, "j", shape.flag, location.abovebar, color.navy, size = size.large)
plotshape(k == 6, "k", shape.labelup, location.belowbar, color.green, text = "essai", textcolor = color.white)
plotshape(k == 6, "l", shape.labeldown, location.abovebar, color.red, text = "essai", textcolor = color.white)
plotchar(k == 3, "m", "é", location.abovebar, color.yellow, size = size.large)
plotarrow(k == 1 ? 1 : k == 0 ? -1 : na, "n")
"#;

/// 🏷 Every style of `label.new`: its ten shapes above the candles, its bubbles under them (the tip on each side,
/// then none), one placed by the hour of a candle, and two after the last candle: the short one fits before the
/// price scale, the long one does not and is not drawn.
const LABELS: &str = r#"//@version=6
indicator("Essai étiquettes", overlay = true)
if barstate.islast
    b = bar_index
    label.new(b - 33, high[33] + 3, "rond", color = color.blue, style = label.style_circle, textcolor = color.white, size = size.tiny)
    label.new(b - 30, high[30] + 3, "losange", color = color.purple, style = label.style_diamond, textcolor = color.white, size = size.small)
    label.new(b - 27, high[27] + 3, "carré", color = color.orange, style = label.style_square, textcolor = color.white, size = size.normal)
    label.new(b - 24, high[24] + 3, "haut", color = color.green, style = label.style_triangleup, textcolor = color.white, size = size.large)
    label.new(b - 21, high[21] + 3, "bas", color = color.red, style = label.style_triangledown, textcolor = color.white, size = size.huge)
    label.new(b - 18, high[18] + 3, "", color = color.lime, style = label.style_arrowup)
    label.new(b - 15, high[15] + 3, "", color = color.maroon, style = label.style_arrowdown)
    label.new(b - 12, high[12] + 3, "croix", color = color.teal, style = label.style_cross, textcolor = color.white)
    label.new(b - 9, high[9] + 3, "x", color = color.fuchsia, style = label.style_xcross, textcolor = color.white)
    label.new(b - 6, high[6] + 3, "drapeau", color = color.navy, style = label.style_flag, textcolor = color.white)
    label.new(b - 2, high[2] + 2, "dessus")
    label.new(b - 34, low[34] - 2, "dessous", color = color.green, style = label.style_label_up)
    label.new(b - 28, low[28] - 6, "gauche", color = color.red, style = label.style_label_left)
    label.new(b - 25, low[25] - 6, "droite", color = color.orange, style = label.style_label_right)
    label.new(b - 22, low[22] - 6, "centre", color = color.purple, style = label.style_label_center)
    label.new(b - 19, low[19] - 6, "sans bulle", style = label.style_none, textcolor = color.yellow)
    label.new(b - 16, low[16] - 6, "contour", style = label.style_text_outline, textcolor = color.aqua)
    label.new(b - 13, low[13] - 6, "h-g", color = color.teal, style = label.style_label_upper_left)
    label.new(b - 10, low[10] - 6, "h-d", color = color.teal, style = label.style_label_upper_right)
    label.new(b - 7, low[7] - 6, "b-g", color = color.navy, style = label.style_label_lower_left)
    label.new(b - 4, low[4] - 6, "b-d\ndeux lignes", color = color.navy, style = label.style_label_lower_right)
    label.new(time[20], high[20] + 10, "à l'heure", xloc = xloc.bar_time, color = color.gray)
    label.new(b + 2, close, "après", color = color.blue, style = label.style_label_left)
    label.new(b + 2, close - 8, "une étiquette trop longue pour tenir avant l'échelle des prix", style = label.style_label_left)
"#;

/// 📏 The lines (each style, each arrow, extended to each side, one placed by the hour that ends six candles after
/// the last one), the area between two lines, the boxes (a border and a text, one extended to the right, one without
/// a border, one that crosses the top of its pane, one above its pane: nothing) and a table in a corner.
const LINES: &str = r#"//@version=6
indicator("Essai lignes", overlay = true)
if barstate.islast
    b = bar_index
    line.new(b - 34, low[34], b - 28, low[34], color = color.red, width = 2)
    line.new(b - 34, low[34] - 3, b - 28, low[34] - 3, color = color.orange, style = line.style_dashed)
    line.new(b - 34, low[34] - 6, b - 28, low[34] - 6, color = color.yellow, style = line.style_dotted)
    line.new(b - 26, low[26] - 2, b - 20, low[26] - 2, color = color.green, style = line.style_arrow_right, width = 2)
    line.new(b - 26, low[26] - 5, b - 20, low[26] - 5, color = color.teal, style = line.style_arrow_left, width = 2)
    line.new(b - 26, low[26] - 8, b - 20, low[26] - 8, color = color.aqua, style = line.style_arrow_both, width = 2)
    line.new(b - 16, high[16] + 2, b - 12, high[12] + 6, extend = extend.left, color = color.purple)
    line.new(b - 10, high[10] + 6, b - 6, high[6] + 2, extend = extend.right, color = color.fuchsia)
    line.new(b - 18, low[18] - 9, b - 8, low[8] - 12, extend = extend.both, color = color.gray, style = line.style_dashed)
    line.new(time[8], low[8] - 2, time + 6 * 1800000, low[8] - 2, xloc = xloc.bar_time, color = color.white, width = 2)
    l1 = line.new(b - 34, high[34] + 8, b - 24, high[24] + 12, color = color.lime)
    l2 = line.new(b - 34, high[34] + 3, b - 24, high[24] + 4, color = color.lime)
    linefill.new(l1, l2, color.new(color.lime, 75))
    box.new(b - 22, high[22] + 12, b - 16, high[22] + 5, border_color = color.orange, border_width = 2, border_style = line.style_dashed, bgcolor = color.new(color.orange, 80), text = "boîte", text_color = color.white)
    box.new(b - 5, low[5] - 3, b - 2, low[5] - 8, border_color = color.red, extend = extend.right, bgcolor = color.new(color.red, 85), text = "étendue", text_color = color.white)
    box.new(b - 14, low[14] - 2, b - 11, low[14] - 6, border_width = 0, bgcolor = color.new(color.teal, 60))
    box.new(b - 4, high[4] + 100000, b - 1, high[4] + 4, border_color = color.blue, bgcolor = color.new(color.blue, 85))
    box.new(b - 30, high + 300000, b - 20, high + 200000, border_color = color.blue)
var t = table.new(position.bottom_left, 2, 3, bgcolor = color.new(color.black, 30), frame_color = color.gray, frame_width = 1, border_color = color.gray, border_width = 1)
table.cell(t, 0, 0, "Essai", text_color = color.white)
table.cell(t, 1, 0, "valeur", text_color = color.gray, text_size = size.small)
table.cell(t, 0, 1, "Clôture", text_color = color.white)
table.cell(t, 1, 1, str.tostring(close), text_color = color.white, bgcolor = close > open ? color.green : color.red)
table.cell(t, 0, 2, "Volume", text_color = color.white)
table.cell(t, 1, 2, str.tostring(volume), text_color = color.yellow)
"#;

/// 📈 A script in its own pane: a plot of each style that is a line (plain, cut where it has no value, in steps, an
/// area, circles, crosses, shifted five candles, hidden), two levels, what is filled between two plots and between
/// two levels, the background of a candle, the colour of the candles, marks at the top and at the bottom of the pane,
/// a label and, on the price, what the script forces there.
const PLOTS: &str = r#"//@version=6
indicator("Essai tracés")
k = last_bar_index - bar_index
m = ta.sma(close, 5)
haut = plot(m + 6, "ligne", color.blue, 2)
bas = plot(m - 6, "marches", color.green, style = plot.style_stepline)
fill(haut, bas, color = m > m[1] ? color.new(color.green, 85) : color.new(color.red, 85))
plot(bar_index % 7 < 5 ? m + 10 : na, "coupée", color.red, 2, plot.style_linebr)
plot(m - 12, "aire", color.new(color.purple, 20), style = plot.style_area)
plot(bar_index % 3 == 0 ? m + 14 : na, "ronds", color.orange, 2, plot.style_circles)
plot(m + 18, "croix", bar_index % 2 == 0 ? color.teal : na, style = plot.style_cross)
plot(m, "décalée", color.gray, offset = 5)
plot(m + 30, "cachée", color.white, display = display.none)
h1 = hline(180, "bas", color.gray, hline.style_dashed)
h2 = hline(230, "haut", color.red, hline.style_dotted)
fill(h1, h2, color.new(color.blue, 92))
bgcolor(bar_index % 10 == 0 ? color.new(color.yellow, 85) : na)
barcolor(close > open ? color.aqua : na)
plotshape(k % 12 == 0, "en haut", shape.circle, location.top, color.red)
plotshape(k % 12 == 6, "en bas", shape.square, location.bottom, color.green)
plotshape(k % 12 == 3, "au-dessus", shape.triangledown, location.abovebar, color.orange)
plotshape(k % 12 == 9, "en dessous", shape.triangleup, location.belowbar, color.lime)
plotshape(k == 4, "sur le prix", shape.flag, location.abovebar, color.yellow, size = size.large, force_overlay = true)
if barstate.islast
    label.new(bar_index - 10, m, "dans la bande")
    line.new(bar_index - 30, low[30], bar_index, low, color = color.white, force_overlay = true)
"#;

/// 📊 A script in its own pane, around zero: columns, a histogram, an area down to zero.
const COLUMNS: &str = r#"//@version=6
indicator("Essai colonnes")
d = close - ta.sma(close, 10)
plot(d, "colonnes", d >= 0 ? color.green : color.red, style = plot.style_columns)
plot(d / 2, "histogramme", color.blue, 2, plot.style_histogram)
plot(ta.ema(d, 5), "aire", color.new(color.orange, 30), style = plot.style_area)
hline(0, "zéro", color.gray)
"#;

/// 🎯 A strategy: two quick averages that cross often on the made-up candles, each trade with its stop and its
/// target (enough closed trades for every kind of difference with TradingView's list, [`tradingview_list`]).
const STRATEGY: &str = r#"//@version=6
strategy("Essai stratégie", overlay = true, initial_capital = 10000, default_qty_type = strategy.percent_of_equity, default_qty_value = 10, commission_type = strategy.commission.percent, commission_value = 0.05)
rapide = ta.ema(close, 3)
lente = ta.ema(close, 8)
plot(rapide, "rapide", color.aqua)
plot(lente, "lente", color.orange)
if ta.crossover(rapide, lente)
    strategy.entry("Achat", strategy.long)
    strategy.exit("Sortie achat", "Achat", stop = close * 0.99, limit = close * 1.02)
if ta.crossunder(rapide, lente)
    strategy.entry("Vente", strategy.short)
    strategy.exit("Sortie vente", "Vente", stop = close * 1.01, limit = close * 0.98)
"#;

/// A script of the user on the chart, as the editor saves it (the first one under a made-up id, the next ones under
/// the ids after it); it ran without an error.
pub(super) fn script(a: &mut App, name: &str, src: &str) {
    a.scripts.push(crate::types::SavedScript {
        id: format!("e55a1e55a1e55a{:02x}", 0x1e + a.scripts.len()),
        name: name.into(),
        src: src.into(),
        on: true,
        ..Default::default()
    });
    a.ind.key = Default::default();
    a.refresh_indicators();
    let error = a.ind.out.scripts.iter().find(|view| view.name == name).map(|view| view.error.clone());
    assert_eq!(error, Some(None), "the made-up script « {name} » runs on the chart");
}

/// 📜 What a script of the user can draw on the chart: each of the five scripts above, alone.
fn scripts(l: &mut List) {
    for (name, src) in [("shapes", SHAPES), ("labels", LABELS), ("lines", LINES), ("plots", PLOTS), ("columns", COLUMNS)] {
        l.both(&format!("script/{name}"), &[], move |a| script(a, name, src));
    }
    l.both("script/strategy", &[], strategy);
    strategy_volet(l);
}

/// 🎯 The Stratégie volet (🖥 a volet, 📱 a sheet) with one strategy on the chart (its pill offers no choice; two of
/// them: `tool/Strategie`). 🖥 A strategy just added opens it by itself: `script/strategy`. Then after « ⬆ Comparer
/// avec TradingView »: the report page read TradingView's made-up list (the newest CSV of the downloads of the
/// test) and is closed by its ✕, the volet shows the comparison on its third tab; each control of that volet
/// pressed. And the strategy taken off the chart by the ✕ of its legend: the volet leaves with it.
fn strategy_volet(l: &mut List) {
    use argos_render::strategie::StratClick;
    let shows = [super::universe::id_of("panel", &wl::Tool::Strategie)];
    let volet = |a: &mut App| {
        strategy(a);
        tool_open(a, wl::Tool::Strategie);
    };
    l.add(Form::Phone, "strategy/volet", &shows, volet);
    let compared = move |a: &mut App| {
        volet(a);
        // (their list is made up from the trades of the volet's own result: the report page of the volet has them)
        a.strategie_click(StratClick::Rapport);
        let file = crate::platform::dossier_telechargements().join(THEIR_FILE);
        std::fs::write(&file, tradingview_list(a, Theirs::Apart)).expect("the made-up list, written in the downloads of the test");
        a.ui.bt_page = None;
        a.strategie_click(StratClick::Tv);
        let _ = std::fs::remove_file(&file);
        (a.ui.bt_page, a.selected) = (None, None);
    };
    l.both_followed("strategy/volet/compared", &shows, compared, Some(super::windows::panel));
    l.both("strategy/volet/none-left", &[], move |a| {
        volet(a);
        laid_out(a);
        a.legend_click(argos_render::indleg::LegClick::ScriptRemove(0));
    });
}

/// 🎯 The strategy script on the chart, chosen for the backtest (as the menu 🧪 keeps the choice).
pub(super) fn strategy(a: &mut App) {
    script(a, "Essai stratégie", STRATEGY);
    a.ui.bt_prefs.script = a.scripts.last().map(|script| script.id.clone()).unwrap_or_default();
}

/// 🎯 A second strategy, one trade at a time above its average: the made-up candles end going up, its last trade is
/// still open at the last one.
const TREND: &str = r#"//@version=6
strategy("Essai tendance", overlay = true, initial_capital = 10000)
moyenne = ta.sma(close, 20)
plot(moyenne, "moyenne", color.purple)
if ta.crossover(close, moyenne)
    strategy.entry("Achat", strategy.long)
if ta.crossunder(close, moyenne)
    strategy.close("Achat")
"#;

/// 🎯 Two strategies on the chart: the Stratégie volet shows the first one, chosen as in the menu of its pill (it
/// goes by itself to the one just added, the second).
pub(super) fn strategies(a: &mut App) {
    strategy(a);
    script(a, "Essai tendance", TREND);
    a.strategie_pick(0);
}

/// What a full page needs to know of the app to answer a press or the wheel.
pub(super) struct Around {
    /// The size of the scene's window.
    pub(super) w: f64,
    pub(super) h: f64,
    /// The size the app fits its windows in (on the phone: without the bar at the bottom).
    pub(super) win: (f64, f64),
    /// The rows of the tree of objects.
    pub(super) tree: Vec<argos_render::objtree::TreeRow>,
    /// The indicators on the chart (the Marketplace marks them).
    on: Vec<Ind>,
    /// The lines of the table of the backtest report, in each of its two tabs.
    lines: [usize; 2],
    /// The cards of the home page, with what the app knows now.
    cards: Vec<Carte>,
    /// The round Argos AI button (on the PC, its pill): the app lays it over its pages, its sheets and nearly all of
    /// its windows, and a press there is the button's.
    button: Rect,
}

impl Around {
    pub(super) fn of(a: &App) -> Around {
        let (w, h) = a.ui.taille_test.map_or((0.0, 0.0), |size| (f64::from(size.width), f64::from(size.height)));
        let tz = a.tz_offset_s();
        let table = |tab: usize| {
            let page = BtPage { tab, ..Default::default() };
            a.ui.bt.as_ref().map_or(0, |run| page.lignes(run, tz))
        };
        Around {
            w,
            h,
            win: (a.win_w(), a.area.1 as f64 + a.top_h()),
            tree: a.tree_rows(),
            on: a.ind.on.clone(),
            lines: [table(0), table(1)],
            cards: argos_render::accueil::cartes(&a.accueil_donnees()),
            button: a.ai_bouton_rect(),
        }
    }
}

/// A full page of the app (it covers the window and receives every click) or a window over the chart: a copy of the
/// one that is open, what a press at a point does to it, and the wheel over it.
///
/// The app gives its full pages the size of its window (`App::portfolio_click`, `App::mouse_wheel`…), and a test has
/// no window: a press of their scenes is the page's own (`click`), at the size of the scene ([`Page::held`]). What the
/// app then does with the page's answer (close it, a banner, the money of the demo account) is not in these scenes,
/// except for the home page: the app takes its answers in a function of their own (`App::accueil_result`). A window
/// has its own place and size: its gestures are the app's (a click, a touch, the wheel), found on a copy first.
pub(super) trait Page: Clone + PartialEq + std::fmt::Debug + 'static {
    /// Turns of the wheel tried by [`gestures`], at most.
    const TURNS: usize = 6;
    /// Presses of one kind kept by [`controls`], at most (the cards of a page are many, and all alike): the first
    /// ones, or with `SPREAD` the first, the last and some between them.
    const ALIKE: usize = 8;
    const SPREAD: bool = false;
    /// After a press that brings new controls (a list that opens, a tab), how many of these are pressed in turn.
    const DEEP: usize = 0;
    /// Whether a kind of control that a press brings is pressed once for the whole page, and not once for each press
    /// that brings it.
    const ONCE: bool = false;
    /// Whether two copies of the page that are equal are the same in all a press reads: a copy that a press left as
    /// it was is then pressed again at the next point, instead of a new copy (a big page is long to copy).
    const EXACT: bool = false;
    /// The page itself in the app, when its gestures are done on it (a full page).
    fn held(_a: &mut App) -> Option<&mut Self> {
        None
    }
    /// A copy of the open page, with what its clicks read of the app.
    fn copy(a: &mut App) -> Option<Self> {
        Self::held(a).map(|page| page.clone())
    }
    /// A press on the page. What the page then asks of the app is given in a few words ([`asked`]) when
    /// [`Page::pressed`] hands it over to the app; an empty text otherwise.
    fn press(&mut self, around: &Around, x: f64, y: f64) -> String;
    /// The wheel (or the finger) over the page, `dy` points further down.
    fn wheel(&mut self, around: &Around, dy: f64);
    /// Where the page is in the window: all of it.
    fn area(&self, around: &Around) -> Rect {
        Rect::new(0.0, 0.0, around.w, around.h)
    }
    /// How far apart [`gestures`] presses a page of this width, across and down: closer than its smallest control.
    fn grid(_width: f64) -> (f64, f64) {
        (12.0, 8.0)
    }
    /// The kind of a gesture, from its name and what it changes ([`Page::ALIKE`]): the first thing it changes,
    /// without its value.
    fn kind(name: &str, _changed: &[(String, String)]) -> String {
        name.split('-').next().unwrap_or_default().to_string()
    }
    /// A field that is not a change of the page: how far it is scrolled (the same control is the same one wherever
    /// the page is).
    fn aside(path: &str) -> bool {
        path.ends_with("scroll")
    }
    /// One turn of the wheel on this copy: three quarters of the window.
    fn turn(&mut self, around: &Around) {
        self.wheel(around, around.h * 0.75);
    }
    /// The press done for good: on the open page of the app, or by the app.
    fn pressed(a: &mut App, around: &Around, x: f64, y: f64) {
        match Self::held(a) {
            Some(page) => {
                page.press(around, x, y);
            }
            None => press(a, x, y),
        }
    }
    /// The same turn of the wheel done for good.
    fn turned(a: &mut App, around: &Around) {
        if let Some(page) = Self::held(a) {
            page.turn(around);
        }
    }
    /// Whether the scenes of this page press the control that does `changed`; with `brought`: the control that does
    /// `brought` on what a first press, which did `changed`, brought. A control that several pages show alike is
    /// pressed in the scenes of one of them.
    fn offered(&self, _changed: &[(String, String)], _brought: Option<&[(String, String)]>) -> bool {
        true
    }
    /// Whether what the press that does `changed` brings is looked at ([`Page::DEEP`]).
    fn deepens(&self, _changed: &[(String, String)]) -> bool {
        true
    }
}

/// What a page asks of the app after a press, in a few words: « Section(Academie) », « MenuLayout(0) »; nothing asked:
/// an empty text.
pub(super) fn asked(answer: &impl std::fmt::Debug) -> String {
    let said = format!("{answer:?}");
    let short = said.split([',', '{']).next().unwrap_or_default().trim();
    match short {
        "None" => String::new(),
        _ if short.contains('(') && !short.ends_with(')') => format!("{short})"),
        _ => short.to_string(),
    }
}

impl Page for AccueilPage {
    // (the home page of the phone is ten cards, one under the other; its smallest controls are the star and the « ⋯ »
    // of a tile, 28 points)
    const TURNS: usize = 12;
    fn grid(_width: f64) -> (f64, f64) {
        (16.0, 12.0)
    }
    fn held(a: &mut App) -> Option<&mut Self> {
        a.ui.accueil.as_mut()
    }
    fn press(&mut self, around: &Around, x: f64, y: f64) -> String {
        asked(&self.click(&Text::default(), around.w, around.h, x, y, &around.cards))
    }
    fn wheel(&mut self, around: &Around, dy: f64) {
        AccueilPage::wheel(self, dy, &Text::default(), around.w, around.h, &around.cards);
    }
    fn pressed(a: &mut App, around: &Around, x: f64, y: f64) {
        let Some(page) = a.ui.accueil.as_mut() else { return };
        let asks = page.click(&Text::default(), around.w, around.h, x, y, &around.cards);
        a.accueil_result(asks);
    }
}

impl Page for PfPage {
    fn held(a: &mut App) -> Option<&mut Self> {
        a.ui.pf_page.as_mut()
    }
    fn press(&mut self, around: &Around, x: f64, y: f64) -> String {
        self.click(&Text::default(), around.w, around.h, x, y);
        String::new()
    }
    fn wheel(&mut self, around: &Around, dy: f64) {
        PfPage::wheel(self, dy, &Text::default(), around.w, around.h);
    }
}

impl Page for MkPage {
    fn held(a: &mut App) -> Option<&mut Self> {
        a.ui.mk_page.as_mut()
    }
    fn press(&mut self, around: &Around, x: f64, y: f64) -> String {
        self.click(&Text::default(), around.w, around.h, &around.on, x, y);
        String::new()
    }
    fn wheel(&mut self, around: &Around, dy: f64) {
        MkPage::wheel(self, dy, around.w, around.h, &around.on);
    }
}

impl Page for AcademyPage {
    // (a press of the Academy lays the whole lesson out again: fewer of them. Its controls are rows and wide buttons,
    // the narrowest one the time zone of the phone, 44 points)
    const TURNS: usize = 3;
    fn grid(width: f64) -> (f64, f64) {
        (width / 20.0, 8.0)
    }
    fn held(a: &mut App) -> Option<&mut Self> {
        a.ui.ac_page.as_mut()
    }
    fn press(&mut self, around: &Around, x: f64, y: f64) -> String {
        self.click(&Text::default(), around.w, around.h, x, y);
        String::new()
    }
    fn wheel(&mut self, around: &Around, dy: f64) {
        AcademyPage::wheel(self, dy, &Text::default(), around.w, around.h);
    }
}

impl Page for BtPage {
    fn held(a: &mut App) -> Option<&mut Self> {
        a.ui.bt_page.as_mut()
    }
    fn press(&mut self, around: &Around, x: f64, y: f64) -> String {
        self.click(&Text::default(), around.w, around.h, x, y);
        String::new()
    }
    fn wheel(&mut self, around: &Around, dy: f64) {
        // (the rows of ⚖️ TradingView are the page's own: `BtPage::lignes`)
        let lines = match &self.tv {
            Some(theirs) if self.tab >= around.lines.len() => theirs.c.rangees.len(),
            _ => around.lines[self.tab.min(1)],
        };
        BtPage::wheel(self, dy, &Text::default(), around.w, around.h, lines);
    }
}

/// A text of `{:?}` cut at the commas that are not inside brackets or quotes.
fn parts(said: &str) -> Vec<&str> {
    let (mut all, mut from, mut depth, mut quoted, mut escaped) = (vec![], 0, 0, false, false);
    for (k, c) in said.char_indices() {
        match c {
            _ if escaped => escaped = false,
            '\\' => escaped = quoted,
            '"' => quoted = !quoted,
            _ if quoted => {}
            '{' | '(' | '[' => depth += 1,
            '}' | ')' | ']' => depth -= 1,
            ',' if depth == 0 => {
                all.push(said[from..k].trim());
                from = k + 1;
            }
            _ => {}
        }
    }
    all.push(said[from..].trim());
    all
}

/// The fields of a value as `{:?}` writes them (« PfPage { range: 2, guide: None } » → « range » is « 2 », « guide »
/// is « None »), each under its path: the fields of a field that is itself a set of fields are given one by one
/// (« window.tab »), anything else whole.
fn fields(value: &impl std::fmt::Debug) -> Vec<(String, String)> {
    fn spread(path: &str, said: &str, all: &mut Vec<(String, String)>) {
        let named = |name: &str| !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_');
        let inside = said.strip_suffix(" }").and_then(|said| said.split_once(" { ")).filter(|(name, _)| named(name));
        let Some((_, inside)) = inside else {
            return all.push((path.to_string(), said.to_string()));
        };
        for field in parts(inside) {
            match field.split_once(": ").filter(|(name, _)| named(name)) {
                Some((name, is)) if path.is_empty() => spread(name, is, all),
                Some((name, is)) => spread(&format!("{path}.{name}"), is, all),
                None => all.push((path.to_string(), field.to_string())),
            }
        }
    }
    let mut all = vec![];
    spread("", &format!("{value:?}"), &mut all);
    all
}

/// What a press changed in a page: the fields that are not the same, and their new value (without the fields that
/// are no change of the page, [`Page::aside`]). `was`: its fields before.
fn change<P: Page>(was: &[(String, String)], after: &P) -> Vec<(String, String)> {
    let is = fields(after);
    let aligned = is.len() == was.len() && is.iter().zip(was).all(|(new, old)| new.0 == old.0);
    is.iter()
        .enumerate()
        .filter(|(k, field)| !P::aside(&field.0) && if aligned { was[*k] != **field } else { !was.contains(field) })
        .map(|(_, field)| field.clone())
        .collect()
}

/// A change in a few words, for the name of its scene: « guide-Some(0) » (the name of a field is the end of its
/// path; where a window is and its size are left out when it changed something else); of a long value, what it has
/// more than before (« favs-EmaCross ») or, when that is long too, the name of the field alone; and what the page
/// asks of the app as it is (« Section(Academie) », kept under a field without a name).
fn named(was: &[(String, String)], change: &[(String, String)]) -> String {
    let short = |(path, is): &(String, String)| {
        if path.is_empty() {
            return is.clone();
        }
        let name = path.rsplit('.').next().unwrap_or_default();
        if is.len() <= 14 {
            return format!("{name}-{is}");
        }
        let old = was.iter().find(|(known, _)| known == path).map_or("", |(_, old)| old.as_str());
        let same = is.chars().zip(old.chars()).take_while(|(new, old)| new == old).count();
        let more: String = is.chars().skip(same).filter(|c| c.is_alphanumeric()).collect();
        if more.is_empty() || more.len() > 14 {
            name.to_string()
        } else {
            format!("{name}-{more}")
        }
    };
    // (a window that grows or moves with what it holds: named by what it holds)
    let place = |(path, _): &&(String, String)| path.rsplit('.').nth(1) == Some("rect");
    let told: Vec<&(String, String)> = match change.iter().filter(|field| !place(field)).collect::<Vec<_>>() {
        other if other.is_empty() => change.iter().collect(),
        other => other,
    };
    told.into_iter().map(short).collect::<Vec<_>>().join("-")
}

/// One gesture on a page: the wheel turned this many times, then a press, then a second one on what the first one
/// brought.
#[derive(Clone, Copy)]
pub(super) struct Gesture {
    wheel: usize,
    pub(super) press: Option<(f64, f64)>,
    then: Option<(f64, f64)>,
}

/// What a press at a point does to a page: what it changes ([`change`], with what the page asks of the app), its
/// name, the page after it and what the page asks of the app.
struct Probe<P> {
    changed: Vec<(String, String)>,
    name: String,
    at: (f64, f64),
    after: P,
    asks: String,
}

/// Where a press is tried on a page that is at `area` in the window: across and down, every few points (`grid`) of
/// what the window shows of it (a press is on the screen: the part of a window past the edge of the scene's window
/// is out of reach). With `column`, all at that part of the window's width.
pub(super) fn tried(area: Rect, around: &Around, grid: (f64, f64), column: Option<f64>) -> (Vec<f64>, Vec<f64>) {
    let (right, bottom) = ((area.x + area.w).min(around.w), (area.y + area.h).min(around.h));
    let (left, across) = column.map_or((area.x.max(0.0) + 3.0, grid.0), |part| (around.w * part, around.w));
    let along = |from: f64, to: f64, step: f64| -> Vec<f64> { std::iter::successors(Some(from), |at| Some(at + step)).take_while(|at| *at < to).collect() };
    (along(left, right, across), along(area.y.max(0.0) + 3.0, bottom, grid.1))
}

/// Every press that changes this page or asks something of the app, one for each thing a press can do: asked of a
/// copy of the page at each point of [`tried`]. With `column`, the presses are all at that part of the window's
/// width (a search down a page of rows); with `wanted`, the search stops at the first press whose name begins so
/// (the last one given).
fn probes<P: Page>(page: &P, around: &Around, column: Option<f64>, wanted: Option<&str>) -> Vec<Probe<P>> {
    let (across, down) = tried(page.area(around), around, P::grid(around.w), column);
    let mut found: Vec<Probe<P>> = vec![];
    let was = fields(page);
    // (the points of one control all do the same: what a press did is looked at once)
    let mut seen: Vec<(P, String)> = vec![];
    let mut spare: Option<P> = None;
    // (under the round Argos AI button, the press would open Argos AI and not reach the page)
    let reached = |(x, y): &(f64, f64)| !around.button.contains(*x, *y);
    for (x, y) in down.iter().flat_map(|y| across.iter().map(move |x| (*x, *y))).filter(reached) {
        let mut after = spare.take().unwrap_or_else(|| page.clone());
        let asks = after.press(around, x, y);
        if after == *page && asks.is_empty() {
            spare = P::EXACT.then_some(after);
            continue;
        }
        if seen.iter().rev().any(|(page, asked)| *page == after && *asked == asks) {
            continue;
        }
        let mut changed = change(&was, &after);
        if !asks.is_empty() {
            changed.push((String::new(), asks.clone()));
        }
        seen.push((after.clone(), asks.clone()));
        if changed.is_empty() || found.iter().any(|known| known.changed == changed) {
            continue;
        }
        let name = named(&was, &changed);
        let stop = wanted.is_some_and(|wanted| name.starts_with(wanted));
        found.push(Probe {
            changed,
            name,
            at: (x, y),
            after,
            asks,
        });
        if stop {
            return found;
        }
    }
    found
}

/// Everything one gesture does to the open page, with its name and what it changes: the page further down at each
/// turn of the wheel (`turns` of them at most), and each control that changes the page or asks something of the app
/// ([`probes`]), at each place the wheel brings the page to. With `wanted`, the search stops at the first gesture
/// whose name begins so (the last one given).
fn gestures<P: Page>(a: &mut App, turns: usize, wanted: Option<&str>, column: Option<f64>) -> Vec<(String, Gesture, Vec<(String, String)>)> {
    let around = Around::of(a);
    P::copy(a).map_or(vec![], |page| gestures_on(page, &around, turns, wanted, column))
}

/// [`gestures`], on this copy of the open page.
fn gestures_on<P: Page>(
    mut page: P,
    around: &Around,
    turns: usize,
    wanted: Option<&str>,
    column: Option<f64>,
) -> Vec<(String, Gesture, Vec<(String, String)>)> {
    let mut found: Vec<(String, Gesture, Vec<(String, String)>)> = vec![];
    for wheel in 0..=turns {
        if wheel > 0 {
            let before = page.clone();
            page.turn(around);
            if page == before {
                break;
            }
            let turned = Gesture {
                wheel,
                press: None,
                then: None,
            };
            found.push((format!("wheel-{wheel}"), turned, vec![]));
        }
        for probe in probes(&page, around, column, wanted) {
            if found.iter().any(|(.., changed)| *changed == probe.changed) {
                continue;
            }
            let pressed = Gesture {
                wheel,
                press: Some(probe.at),
                then: None,
            };
            let stop = wanted.is_some_and(|wanted| probe.name.starts_with(wanted));
            found.push((probe.name, pressed, probe.changed));
            if stop {
                return found;
            }
        }
    }
    found
}

/// This gesture, done on the open page (the app draws itself between two presses: the second one is on what the
/// first one brought).
fn done<P: Page>(a: &mut App, gesture: Gesture) {
    let around = Around::of(a);
    for _ in 0..gesture.wheel {
        P::turned(a, &around);
    }
    if let Some((x, y)) = gesture.press {
        P::pressed(a, &around, x, y);
    }
    if let Some((x, y)) = gesture.then {
        laid_out(a);
        P::pressed(a, &Around::of(a), x, y);
    }
}

/// 📄 Each gesture of [`gestures`] on the open page: one scene for each, [`Page::ALIKE`] at most for the presses of
/// one kind.
pub(super) fn controls<P: Page>(a: &mut App) -> Next {
    children::<P>(a, P::TURNS)
}

/// 📄 The same without the wheel: each control of a screen that is not a page to scroll.
fn presses<P: Page>(a: &mut App) -> Next {
    children::<P>(a, 0)
}

/// Of `n` things alike, the ones kept when `limit` at most are: the first ones, or spread from the first to the last.
fn kept(n: usize, limit: usize, spread: bool) -> Vec<usize> {
    if n <= limit || !spread || limit < 2 {
        return (0..n.min(limit)).collect();
    }
    (0..limit).map(|k| k * (n - 1) / (limit - 1)).collect()
}

fn children<P: Page>(a: &mut App, turns: usize) -> Next {
    let around = Around::of(a);
    let first = P::copy(a);
    let all = first.clone().map_or(vec![], |page| gestures_on(page, &around, turns, None, None));
    // (the gestures of each kind, in the order they were found)
    let mut kinds: Vec<(String, Vec<usize>)> = vec![];
    for (k, (name, _, changed)) in all.iter().enumerate() {
        let kind = P::kind(name, changed);
        match kinds.iter_mut().find(|(known, _)| *known == kind) {
            Some((_, of)) => of.push(k),
            None => kinds.push((kind, vec![k])),
        }
    }
    let offered = |changed: &[(String, String)], brought: Option<&[(String, String)]>| first.as_ref().is_none_or(|page| page.offered(changed, brought));
    let mut keep: Vec<usize> = vec![];
    for (_, of) in &kinds {
        let of: Vec<usize> = of.iter().copied().filter(|k| offered(&all[*k].2, None)).collect();
        keep.extend(kept(of.len(), P::ALIKE, P::SPREAD).into_iter().map(|k| of[k]));
    }
    keep.sort_unstable();
    let mut next: Next = vec![];
    // (the kinds of controls that the presses before this one brought, where each is pressed once: `Page::ONCE`)
    let mut brought: Vec<String> = vec![];
    for k in keep {
        let (name, gesture, did) = &all[k];
        let gesture = *gesture;
        next.push((name.clone(), Arc::new(move |a: &mut App| done::<P>(a, gesture))));
        // what this press brought: the controls that were not there before it, one of each kind
        let (Some(mut page), Some((x, y))) = (first.clone().filter(|page| P::DEEP > 0 && page.deepens(did)), gesture.press) else {
            continue;
        };
        (0..gesture.wheel).for_each(|_| page.turn(&around));
        let was = fields(&page);
        let before = page.clone();
        let asked = page.press(&around, x, y);
        if page == before {
            continue;
        }
        if !P::ONCE {
            brought.clear();
        }
        let mut seen: Vec<String> = vec![];
        for probe in probes(&page, &around, None, None) {
            // (not a new control: a press that undoes the first one; one that only closes or leaves what the first
            // one opened, with an answer the page already gave; a control of a kind that was there before; two
            // presses that do together what one press does)
            let mut both = change(&was, &probe.after);
            let undoes = both.is_empty() && (probe.asks.is_empty() || probe.asks == asked);
            let answered = |asks: &str| {
                all.iter()
                    .any(|(.., changed)| changed.last().is_some_and(|(path, is)| path.is_empty() && is == asks))
            };
            let leaves = probe.changed.iter().filter(|(path, _)| !path.is_empty()).all(|(_, is)| is == "None");
            let closes = leaves && (probe.asks.is_empty() || answered(&probe.asks));
            if !probe.asks.is_empty() {
                both.push((String::new(), probe.asks.clone()));
            }
            let known = |done: &Vec<(String, String)>| all.iter().any(|(.., changed)| changed == done);
            // (what the press does besides leaving what the first one opened)
            let does: Vec<(String, String)> = probe.changed.iter().filter(|(_, is)| is != "None").cloned().collect();
            let kind = P::kind(&probe.name, &probe.changed);
            let before = all.iter().any(|(name, _, changed)| P::kind(name, changed) == kind);
            let left_out = brought.contains(&kind) || !offered(did, Some(&probe.changed));
            if seen.len() >= P::DEEP || seen.contains(&kind) || undoes || closes || before || known(&both) || known(&does) || left_out {
                continue;
            }
            brought.push(kind.clone());
            seen.push(kind);
            let both = Gesture {
                then: Some(probe.at),
                ..gesture
            };
            next.push((format!("{name}/{}", probe.name), Arc::new(move |a: &mut App| done::<P>(a, both))));
        }
    }
    next
}

/// The first gesture whose name begins so ([`gestures`]), done on the open page: a second state of the page. With
/// `column`, it is looked for down that part of the page's width only. Gives the gesture that was done.
pub(super) fn reach<P: Page>(a: &mut App, name: &str, column: Option<f64>) -> Gesture {
    let all = gestures::<P>(a, P::TURNS, Some(name), column);
    let Some((_, gesture, _)) = all.last().filter(|(known, ..)| known.starts_with(name)) else {
        panic!(
            "no gesture « {name} » on this page: {:?}",
            all.iter().map(|(known, ..)| known).collect::<Vec<_>>()
        );
    };
    done::<P>(a, *gesture);
    *gesture
}

/// The same with the last gesture whose name begins so, the wheel left alone: the last row of a list.
pub(super) fn reach_last<P: Page>(a: &mut App, name: &str) -> Gesture {
    let all = gestures::<P>(a, 0, None, None);
    let Some((_, gesture, _)) = all.iter().rev().find(|(known, ..)| known.starts_with(name)) else {
        panic!(
            "no gesture « {name} » on this page: {:?}",
            all.iter().map(|(known, ..)| known).collect::<Vec<_>>()
        );
    };
    done::<P>(a, *gesture);
    *gesture
}

/// 🎓 A pupil of the Academy, as the app keeps one (`academy.json`): the first two lessons done (their quiz at 5
/// and at 4 out of 5), the third one read and its quiz missed.
fn pupil() {
    let lesson = |k: usize| lecons()[k].id.clone();
    let progress = Progress {
        read: (0..3).map(lesson).collect(),
        done: (0..2).map(lesson).collect(),
        scores: [(lesson(0), 5), (lesson(1), 4), (lesson(2), 2)].into(),
        ..Default::default()
    };
    App::write_prefs("academy.json", &progress);
}

/// 🎓 The Academy of this pupil, opened by its tool.
fn academy(a: &mut App) {
    pupil();
    tool_open(a, wl::Tool::Academie);
    let done = a.ui.ac_page.as_ref().map(|page| page.done_count().0);
    assert_eq!(done, Some(2), "the two lessons the made-up pupil has done are lessons of the Academy");
}

/// 🎓 A lesson of the Academy of this pupil, opened from the list of its course: the `nth` lesson of this course.
fn lesson(a: &mut App, course: u8, nth: usize) {
    academy(a);
    reach::<AcademyPage>(a, &format!("view-Parcours({course})"), None);
    let first = lecons().iter().position(|lesson| lesson.parcours == course).unwrap_or_default();
    reach::<AcademyPage>(a, &format!("view-Lecon({})", first + nth), None);
}

/// 🎓 The quiz of the open lesson, done without a mistake: the page is pressed row after row, from the top of the
/// lesson to its end (an answer is a row as wide as the page), and a press stays when it gives one more answer, the
/// right one.
fn quiz_done(a: &mut App) {
    let around = Around::of(a);
    let Some(page) = a.ui.ac_page.as_mut() else {
        panic!("the Academy is not open")
    };
    let View::Lecon(open) = page.view else { panic!("no lesson is open") };
    let right: Vec<String> = lecons()[open].quiz.iter().map(|question| format!("Some({})", question.bonne)).collect();
    let given = |page: &AcademyPage| {
        fields(page)
            .into_iter()
            .find(|(name, _)| name == "answers")
            .map_or_else(String::new, |(_, is)| is)
    };
    let left = |answers: &str| answers.matches("None").count();
    loop {
        let mut y = 3.0;
        while y < around.h {
            let mut probe = page.clone();
            probe.press(&around, around.w / 2.0, y);
            let answers = given(&probe);
            let all_right = answers
                .trim_matches(['[', ']'])
                .split(", ")
                .zip(&right)
                .all(|(is, right)| is == "None" || is == right);
            if probe.view == page.view && left(&answers) + 1 == left(&given(page)) && all_right {
                *page = probe;
            }
            y += 16.0;
        }
        let before = page.clone();
        Page::wheel(page, &around, around.h * 0.75);
        if *page == before {
            break;
        }
    }
    assert!(!right.is_empty() && left(&given(page)) == 0, "the quiz of the lesson is done: {}", given(page));
}

/// 🏠 The home page of someone who uses the app: five layouts kept (three tiles show, one is a favourite), a demo
/// account that has traded, a pupil of the Academy.
fn home(a: &mut App) {
    use crate::actions::layouts::{Layout, Layouts, FICHIER};
    use crate::actions::synchro::Derniere;
    traded(a);
    pupil();
    let layout = |(name, base, interval, favourite): (&str, &str, i64, bool)| Layout {
        nom: name.into(),
        marche: Derniere {
            ex: Exchange::Hyperliquid.id().into(),
            base: base.into(),
            interval,
        },
        favori: favourite,
        ..Default::default()
    };
    let kept = [
        ("Argos", "BTC", 1800, false),
        ("Or", "PAXG", 3600, true),
        ("Scalp", "ETH", 60, false),
        ("Semaine", "SOL", 604_800, false),
        ("Essai", "HYPE", 900, false),
    ];
    let layouts = Layouts {
        ouverte: "Argos".into(),
        liste: kept.map(layout).into(),
    };
    App::write_prefs(FICHIER, &layouts);
    a.top.prefs.layout_name = "Argos".into();
    a.open_accueil();
}

/// ⚖️ What TradingView's made-up list holds of the trades of the report.
#[derive(Clone, Copy)]
enum Theirs {
    /// Each trade, as the report has it.
    Alike,
    /// Each kind of difference among the newest trades, a trade still open there, and the oldest trade left out.
    Apart,
    /// The same trades 400 days later: the chart and the list have no time in common.
    Elsewhere,
}

/// The name of TradingView's made-up file (the app only takes a file it did not write itself: no « argos- »).
const THEIR_FILE: &str = "made-up-list-of-trades.csv";

/// ⚖️ TradingView's list of the trades of the open report, as TradingView exports it (« Liste des transactions », the
/// app's own way to write one: `rapport::csv`), in the chart's time zone. It is made up from the report's own trades.
/// [`Theirs::Apart`]: from the newest trade but one (the time compared ends with their newest trade), a result
/// 12,50 $ higher there, an exit a dollar higher, an entry a candle later, an exit a candle earlier, the other side,
/// a trade that is not in their list; then one more trade, still open there, and not the oldest one (it is before
/// the time their list covers).
fn tradingview_list(a: &App, theirs: Theirs) -> String {
    use argos_core::{Money, Side};
    let run = a.ui.bt.as_ref().expect("the backtest of the report");
    let mut list = run.detail.lignes.clone();
    let candle = run
        .detail
        .temps
        .windows(2)
        .map(|pair| pair[1] - pair[0])
        .min()
        .expect("the candles of the backtest");
    match theirs {
        Theirs::Alike => {}
        Theirs::Elsewhere => list.iter_mut().for_each(|trade| {
            trade.t_in += 400 * 86_400;
            trade.t_out += 400 * 86_400;
        }),
        Theirs::Apart => {
            let n = list.len();
            // (a trade moved by a candle lasts two candles or more: it still ends after it starts)
            let lasts = |trade: &argos_script::lang::rapport::Ligne| trade.t_out - trade.t_in >= 2 * candle;
            let moved: Vec<usize> = (1..n.saturating_sub(1)).rev().filter(|k| lasts(&list[*k])).take(2).collect();
            let other: Vec<usize> = (1..n.saturating_sub(1)).rev().filter(|k| !moved.contains(k)).take(4).collect();
            let (&[later, earlier], &[result, price, side, missing]) = (moved.as_slice(), other.as_slice()) else {
                panic!(
                    "the made-up strategy closes {n} trades, {} of two candles or more: too few for each kind of difference",
                    moved.len()
                );
            };
            list[result].net += Money::from_f64(12.5);
            list[price].prix_out += 1.0;
            list[later].t_in += candle;
            list[earlier].t_out -= candle;
            list[side].side = match list[side].side {
                Side::Long => Side::Short,
                Side::Short => Side::Long,
            };
            let mut open = list[n - 1].clone();
            (open.numero, open.sortie) = (n + 1, "Open".into());
            (open.t_in, open.t_out) = (list[n - 1].t_out + candle, list[n - 1].t_out + 2 * candle);
            list.remove(missing);
            list.remove(0);
            list.push(open);
        }
    }
    argos_script::lang::rapport::csv(&list, run.start, "USD", a.tz_offset_s())
}

/// 📄 The full pages: the home page of someone who uses the app, the portfolio of a demo account that has traded,
/// the Marketplace, the Academy of a pupil, the report of a backtest; then each of their controls pressed, and the
/// page further down. Then from a second state: the three screens of a first launch, the guides of the real account
/// on the phone, a search of the Marketplace, a course and three lessons of the Academy.
fn pages(l: &mut List) {
    let portfolio = |a: &mut App| {
        traded(a);
        tool_open(a, wl::Tool::Portfolio);
    };
    // 🛒 two indicators on the chart and two favourites, one of them on the chart: the cards say so, and « Mes
    // indicateurs » and « Favoris » list something
    let marketplace = |a: &mut App| {
        for indicator in [Ind::Tvd, Ind::Cipher] {
            if !a.ind.on.contains(&indicator) {
                a.toggle_indicator(indicator);
            }
        }
        a.ind.favs = vec![Ind::Tvd, Ind::EmaMulti];
        tool_open(a, wl::Tool::Marketplace);
    };
    let report = |a: &mut App| {
        a.run_backtest();
        a.banner = None;
        list_open(a, ListKind::Backtest);
        row(a, "Rapport complet");
    };
    // 🎯 the report of a strategy script
    let script_report = move |a: &mut App| {
        strategy(a);
        report(a);
    };
    // ⚖️ TradingView's list read by that report (a file chosen in the page; the desktop app: `compared` below)
    let read = move |a: &mut App, theirs: Theirs| {
        script_report(a);
        let list = tradingview_list(a, theirs);
        a.rapport_tv_lire(THEIR_FILE, list.as_bytes());
        // (📱 the table is under the blocks of the report: the finger brings it up)
        let (around, phone) = (Around::of(a), a.top.phone);
        if let Some(page) = a.ui.bt_page.as_mut().filter(|_| phone) {
            page.turn(&around);
        }
    };
    // ⚖️ what « ⬆ Comparer avec TradingView » asks of the desktop app: it takes the newest CSV of the downloads, where
    // TradingView's export leaves its list (the downloads of a test are a folder of its own, emptied before each scene)
    let compared = move |a: &mut App| {
        script_report(a);
        let file = crate::platform::dossier_telechargements().join(THEIR_FILE);
        std::fs::write(&file, tradingview_list(a, Theirs::Apart)).expect("the made-up list, written in the downloads of the test");
        a.rapport_tv();
        let _ = std::fs::remove_file(&file);
    };
    // (the app writes what a button of the report did in its banner, and draws the report over the banner: the
    // banner shows once the report is closed, as its ✕ closes it; no drawing is selected: on the phone, the bar of
    // a selected drawing lies over the second line of a banner)
    let closed = |a: &mut App| {
        a.ui.bt_page = None;
        a.selected = None;
    };
    // 👋 the screens of a first launch (« Revoir l'accueil » shows them again): the profile, then the theme, then
    // the sources (« Suivant », pressed in the scenes that follow each screen, only counts the screens)
    let welcome = |a: &mut App, screen: u8| {
        a.revoir_accueil();
        if let Some(page) = a.ui.accueil.as_mut() {
            page.bienvenue = Some(screen);
        }
    };
    for form in Form::ALL {
        l.add(form, "page/Home", &[id("ui", "accueil")], home).follow = Some(controls::<AccueilPage>);
        for (screen, name) in [(0, "profile"), (1, "theme"), (2, "sources")] {
            l.add(form, &format!("page/Home/welcome/{name}"), &[id("ui", "accueil")], move |a| welcome(a, screen))
                .follow = Some(presses::<AccueilPage>);
        }
        l.add(form, "page/Portfolio", &[id("ui", "pf_page")], portfolio).follow = Some(controls::<PfPage>);
        l.add(form, "page/Marketplace", &[id("ui", "mk_page")], marketplace).follow = Some(controls::<MkPage>);
        l.add(form, "page/Academy", &[id("ui", "ac_page")], academy).follow = Some(controls::<AcademyPage>);
        l.add(form, "page/Report", &[id("ui", "bt_page")], report).follow = Some(controls::<BtPage>);
        l.add(form, "page/Report/script", &[id("ui", "bt_page")], script_report).follow = Some(controls::<BtPage>);
        // ⚖️ that report compared with TradingView's list: its third tab, each kind of difference in its table (then
        // each tab and the table further down); every trade the same; a list of another time
        l.add(form, "page/Report/compared", &[id("ui", "bt_page")], compared).follow = Some(controls::<BtPage>);
        l.add(form, "page/Report/compared-alike", &[id("ui", "bt_page")], move |a| read(a, Theirs::Alike));
        l.add(form, "page/Report/compared-elsewhere", &[id("ui", "bt_page")], move |a| {
            read(a, Theirs::Elsewhere)
        });
        // ⚖️ a file that is not the list, and no file at all in the downloads; ⬇️ « Export CSV » (the list of the
        // trades written in the downloads): what the banner says of each
        l.add(form, "page/Report/not-the-list", &[], move |a| {
            script_report(a);
            a.rapport_tv_lire("made-up-candles.csv", b"Date,Open,High,Low,Close\n2026-11-18,1,2,0,1\n");
            closed(a);
        });
        l.add(form, "page/Report/no-file", &[], move |a| {
            script_report(a);
            a.rapport_tv();
            closed(a);
        });
        l.add(form, "page/Report/exported", &[], move |a| {
            script_report(a);
            a.rapport_csv();
            closed(a);
        });
        // 🔎 the search of the Marketplace: three letters typed, one erased (a real keyboard's keys: the page's own)
        l.add(form, "page/Marketplace/search", &[id("ui", "mk_page")], move |a| {
            marketplace(a);
            reach::<MkPage>(a, "typing-true", None);
            if let Some(page) = a.ui.mk_page.as_mut() {
                "rsi".chars().for_each(|c| page.type_char(c));
                page.backspace();
            }
        });
        // 🎓 the first course of the Academy (the pupil's marks) and each of its lessons; then three lessons: an
        // answer of its quiz chosen, its quiz done without a mistake, a block of it unfolded
        l.add(form, "page/Academy/course", &[id("ui", "ac_page")], |a| {
            academy(a);
            reach::<AcademyPage>(a, "view-Parcours(0)", None);
        })
        .follow = Some(controls::<AcademyPage>);
        l.add(form, "page/Academy/lesson", &[id("ui", "ac_page")], |a| {
            lesson(a, 0, 0);
            // (the answers are rows as wide as the page)
            reach::<AcademyPage>(a, "answers", Some(0.5));
        });
        l.add(form, "page/Academy/quiz", &[id("ui", "ac_page")], |a| {
            lesson(a, 0, 2);
            quiz_done(a);
        });
        l.add(form, "page/Academy/unfolded", &[id("ui", "ac_page")], |a| {
            lesson(a, 1, 0);
            let pressed = reach::<AcademyPage>(a, "ouverts", Some(0.5)).press;
            // (the block that opened is brought up to the top quarter of the window: what it holds shows)
            let around = Around::of(a);
            if let (Some(page), Some((_, y))) = (a.ui.ac_page.as_mut(), pressed) {
                Page::wheel(page, &around, y - around.h / 4.0);
            }
        });
    }
    // 📱 the guides of the real account are under « Compte réel › » on the phone: each one opened
    l.add(Form::Phone, "page/Portfolio/real", &[id("ui", "pf_page")], move |a| {
        portfolio(a);
        reach::<PfPage>(a, "reel-true", None);
    })
    .follow = Some(|a| controls::<PfPage>(a).into_iter().filter(|(name, _)| name.starts_with("guide")).collect());
}
