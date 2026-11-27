//! 🕸 📋 The tables of the chart, moved and resized (tableaux-1).
//!
//! Two tables: the dashboard of Argos TVD Pro (the table of an indicator: its menu can hide it) and the table of a
//! script of the user (`table.new`: its menu cannot). 🖥 By the mouse: over it, its body dragged, a corner pulled
//! (everything scales), a side pulled (wider, taller rows), dropped on the legend of the price (it goes to the
//! nearest free place), a double click (back in place), its right-click menu and each of its lines. 👆 By the finger
//! (phone, tablets): a tap selects it (the hint pill and its « OK »), then a finger moves it, pulls a corner, two
//! fingers spread on it, « OK » or a tap elsewhere ends it. 🏷 With the list of the legend unfolded (every form): the
//! dashboard, a table dropped on the legend, and the table of a script that asks for the top left of the chart, where
//! the lines of the list are.
//!
//! The scenes press where a table answers whether or not something is drawn there (handles, a grip): the mouse one
//! point outside a corner or outside the middle of a side, the finger ten points inside a corner. Each scene then
//! checks what its pull did (moved, scaled, wider, taller): a scene that no longer does what its name says fails, it
//! never keeps the picture of something else.
//!
//! Two clocks no test can freeze:
//! - the double click (`App::est_double`, 350 ms): the scene clicks until the table is back in place (two clicks;
//!   more if the computer stalled between them), so its screen never depends on the clock;
//! - the quick stroke of a finger (`touch::Touches::up`, 700 ms): a selected table swallows it, and [`slid`] refuses
//!   a slide a fast computer would call a stroke anyway.

use super::scenes::{finger, laid_out, List, Phase};
use super::Form;
use crate::tests::{clic, clic_chart};
use crate::*;
use argos_render::indicators::Ind;
use argos_render::tableaux::{self as tb, MenuLigne, Pose, TableLayout};
use winit::dpi::PhysicalPosition;
use winit::event::ElementState;

/// 📋 A script of the user that draws a table of its own, in the bottom right corner of the chart.
const WITH_TABLE: &str = r#"//@version=6
indicator("Essai tableau", overlay = true)
var t = table.new(position.bottom_right, 2, 2, bgcolor = color.new(color.black, 30), frame_color = color.gray, frame_width = 1, border_color = color.gray, border_width = 1)
table.cell(t, 0, 0, "Essai", text_color = color.white)
table.cell(t, 1, 0, "valeur", text_color = color.gray)
table.cell(t, 0, 1, "Clôture", text_color = color.white)
table.cell(t, 1, 1, str.tostring(close), text_color = color.white)
"#;

/// The table `key` where the app draws it now (chart coordinates).
fn placed(a: &mut App, key: &str) -> Pose {
    laid_out(a);
    tb::pose(key).unwrap_or_else(|| panic!("no table « {key} » on the chart"))
}

/// What the user made of the table `key` (its place, its sizes); as the app gives it when nothing was changed.
fn kept(a: &App, key: &str) -> TableLayout {
    a.ind.tables.get(key).copied().unwrap_or_default()
}

/// 📋 Argos TVD Pro on the chart: its dashboard.
fn dashboard(a: &mut App) -> Pose {
    a.toggle_indicator(Ind::Tvd);
    placed(a, &tb::cle(Ind::Tvd, ""))
}

/// 📋 The script of the user on the chart (`src`): its table.
fn of_script(a: &mut App, src: &str) -> Pose {
    super::states::script(a, "Essai tableau", src);
    laid_out(a);
    let table = tb::poses().into_iter().find(|t| t.key.starts_with("script:"));
    table.expect("the table of the script is on the chart")
}

/// 🖱 The mouse over a point of the chart.
fn over(a: &mut App, x: f64, y: f64) {
    a.mouse_moved(PhysicalPosition::new(a.rw() + x, a.top_h() + y));
}

/// 🖱 The left button pressed on a point of the chart and let go further (the mouse stops once on the way); never the
/// second half of a double click.
fn pulled(a: &mut App, (x, y): (f64, f64), (dx, dy): (f64, f64)) {
    a.pointer.tab.dernier = None;
    over(a, x, y);
    a.left_button(ElementState::Pressed);
    over(a, x + dx / 2.0, y + dy / 2.0);
    over(a, x + dx, y + dy);
    a.left_button(ElementState::Released);
}

/// 🖱 The dashboard dragged by its body to the middle of the chart: where it is then.
fn moved(a: &mut App) -> Pose {
    let t = dashboard(a);
    let (x, y) = (a.chart.viewport.plot_w() * 0.5, a.chart.viewport.plot_h() * 0.5);
    pulled(a, (t.rect.cx(), t.rect.cy()), (x - t.rect.cx(), y - t.rect.cy()));
    assert!(kept(a, &t.key).moved, "the body of a table dragged by the mouse moves it");
    placed(a, &t.key)
}

/// 🖱 The dashboard dragged by its body onto the legend of the price, at the top left of the chart.
fn dropped_on_the_legend(a: &mut App) -> Pose {
    let t = dashboard(a);
    let r = t.rect;
    pulled(a, (r.cx(), r.cy()), (20.0 + r.w / 2.0 - r.cx(), 20.0 + r.h / 2.0 - r.cy()));
    assert!(kept(a, &t.key).moved, "a table dropped on the legend is kept where the app put it");
    placed(a, &t.key)
}

/// 🏷 The list of the legend of the indicators, unfolded.
fn list_unfolded(a: &mut App) {
    if !a.ind.leg.0 {
        a.legend_click(argos_render::indleg::LegClick::Toggle);
    }
    assert!(a.ind.leg.0, "the list of the legend is unfolded");
}

/// 📋 The table of a script at the top left of the chart, where the legend is, and the list of the legend unfolded
/// (a second line in it: a moving average).
fn of_script_with_the_legend(a: &mut App) {
    a.toggle_indicator(Ind::Sma);
    of_script(a, &WITH_TABLE.replace("position.bottom_right", "position.top_left"));
    list_unfolded(a);
}

/// 🖱 The dashboard moved, then a right click on it: its menu.
fn menu(a: &mut App) -> tb::Menu {
    let t = moved(a).rect;
    over(a, t.cx(), t.cy());
    a.right_click();
    a.pointer.tab.menu.clone().expect("the menu of the table is open")
}

/// 🖱 A line of that menu, clicked.
fn menu_line(a: &mut App, line: MenuLigne) {
    let menu = menu(a);
    laid_out(a);
    let lines = tb::menu_lignes(&menu);
    let at = lines
        .into_iter()
        .find(|(_, l)| *l == line)
        .map(|(r, _)| r)
        .expect("the line of the menu of the table");
    clic(a, a.rw() + at.cx(), a.top_h() + at.cy());
    assert!(a.pointer.tab.menu.is_none(), "a line of the menu of a table, clicked, closes the menu");
}

/// 🖥 The tables under the mouse.
fn by_the_mouse(l: &mut List) {
    let pc = Form::Pc;
    // the mouse over it
    l.add(pc, "table/hovered", &[], |a| {
        let t = dashboard(a);
        over(a, t.rect.cx(), t.rect.cy());
        assert_eq!(a.pointer.tab.survol.as_ref().map(|(key, _)| key), Some(&t.key), "the mouse is over the table");
    });
    // dragged by its body to the middle of the chart
    l.add(pc, "table/moved", &[], |a| {
        moved(a);
    });
    // its bottom right corner pulled: everything scales, the opposite corner held
    l.add(pc, "table/moved/scaled", &[], |a| {
        let t = moved(a);
        pulled(a, (t.rect.x + t.rect.w + 1.0, t.rect.y + t.rect.h + 1.0), (60.0, 40.0));
        assert!(kept(a, &t.key).scale > 1.0, "a corner of a table pulled away by the mouse scales it up");
    });
    // its right side pulled: wider
    l.add(pc, "table/moved/wider", &[], |a| {
        let t = moved(a);
        pulled(a, (t.rect.x + t.rect.w + 1.0, t.rect.cy()), (60.0, 0.0));
        assert!(kept(a, &t.key).wx > 1.0, "the right side of a table pulled away by the mouse makes it wider");
    });
    // its bottom side pulled: taller rows
    l.add(pc, "table/moved/taller", &[], |a| {
        let t = moved(a);
        pulled(a, (t.rect.cx(), t.rect.y + t.rect.h + 1.0), (0.0, 40.0));
        assert!(
            kept(a, &t.key).hy > 1.0,
            "the bottom side of a table pulled away by the mouse makes its rows taller"
        );
    });
    // dropped on the legend of the price: it goes to the nearest free place
    l.add(pc, "table/dropped-on-the-legend", &[], |a| {
        dropped_on_the_legend(a);
    });
    // then the list of the legend unfolded (a second line in it): the table is kept under its lines
    l.add(pc, "table/dropped-on-the-legend/list-unfolded", &[], |a| {
        a.toggle_indicator(Ind::Sma);
        dropped_on_the_legend(a);
        list_unfolded(a);
    });
    // a double click: back in place (the clicks go on until it is: see the head of this file)
    l.add(pc, "table/moved/double-click", &[], |a| {
        let t = moved(a);
        let (x, y) = (a.rw() + t.rect.cx(), a.top_h() + t.rect.cy());
        a.pointer.tab.dernier = None;
        for _ in 0..8 {
            if !a.ind.tables.contains_key(&t.key) {
                break;
            }
            clic(a, x, y);
        }
        assert!(!a.ind.tables.contains_key(&t.key), "a double click on a table puts it back in place");
    });
    // its right-click menu (its title, « ↺ Remettre à sa place », its size, « Masquer le tableau »), then each line
    // that answers, and a click beside the menu: it closes, the click goes on to the chart
    l.add(pc, "table/menu", &[], |a| {
        menu(a);
    });
    l.add(pc, "table/menu/back-in-place", &[], |a| {
        menu_line(a, MenuLigne::Remettre);
        assert!(a.ind.tables.is_empty(), "« Remettre à sa place » forgets where the table was put");
    });
    l.add(pc, "table/menu/hidden", &[], |a| {
        menu_line(a, MenuLigne::Masquer);
        laid_out(a);
        assert!(
            tb::pose(&tb::cle(Ind::Tvd, "")).is_none(),
            "« Masquer le tableau » takes the table off the chart"
        );
    });
    l.add(pc, "table/menu/closed-beside", &[], |a| {
        menu(a);
        clic_chart(a, 60.0, a.chart.viewport.plot_h() * 0.8);
        assert!(a.pointer.tab.menu.is_none(), "a click beside the menu of a table closes it");
    });
    // the table of a script: over it, and its menu (no « Masquer le tableau »: a script has no setting for it)
    l.add(pc, "table/of-script/hovered", &[], |a| {
        let t = of_script(a, WITH_TABLE);
        over(a, t.rect.cx(), t.rect.cy());
        assert_eq!(a.pointer.tab.survol.as_ref().map(|(key, _)| key), Some(&t.key), "the mouse is over the table");
    });
    l.add(pc, "table/of-script/menu", &[], |a| {
        let t = of_script(a, WITH_TABLE).rect;
        over(a, t.cx(), t.cy());
        a.right_click();
        let menu = a.pointer.tab.menu.clone().expect("the menu of the table of the script is open");
        laid_out(a);
        let hides = tb::menu_lignes(&menu).into_iter().any(|(_, line)| line == MenuLigne::Masquer);
        assert!(!hides, "the menu of the table of a script has no « Masquer le tableau »");
    });
    // the list of the legend unfolded: the dashboard where the app puts it, and the table of a script that asks for
    // the top left of the chart, kept under the lines of the list
    l.add(pc, "table/with-the-legend", &[], |a| {
        a.toggle_indicator(Ind::Sma);
        dashboard(a);
        list_unfolded(a);
    });
    l.add(pc, "table/of-script/with-the-legend", &[], of_script_with_the_legend);
}

/// 👆 A tap of the finger on a point of the chart.
fn tap(a: &mut App, x: f64, y: f64) {
    let at = (a.rw() + x, a.top_h() + y);
    finger(a, 1, Phase::Down, at);
    finger(a, 1, Phase::Lifted, at);
}

/// 👆 A finger put down on a point of the chart and lifted further (it stops once on the way). Never a quick stroke
/// to the right or down: the clock of the gestures would tell a fast computer from a slow one.
fn slid(a: &mut App, (x, y): (f64, f64), (dx, dy): (f64, f64)) {
    let stroke = (dx > 80.0 && dy.abs() < 60.0) || (dy > 110.0 && dx.abs() < 60.0);
    assert!(
        !stroke,
        "a finger slid by ({dx}, {dy}) is a quick stroke on a fast computer: slide it another way"
    );
    let (x, y) = (a.rw() + x, a.top_h() + y);
    finger(a, 1, Phase::Down, (x, y));
    finger(a, 1, Phase::Moved, (x + dx / 2.0, y + dy / 2.0));
    finger(a, 1, Phase::Moved, (x + dx, y + dy));
    finger(a, 1, Phase::Lifted, (x + dx, y + dy));
}

/// 👆 Two fingers put down on the table, one of them moved away: bigger.
fn spread(a: &mut App, t: &Pose) {
    let before = kept(a, &t.key).scale;
    let (x, y) = (a.rw() + t.rect.cx(), a.top_h() + t.rect.cy());
    finger(a, 1, Phase::Down, (x - 30.0, y));
    finger(a, 2, Phase::Down, (x + 30.0, y));
    finger(a, 2, Phase::Moved, (x + 60.0, y));
    finger(a, 2, Phase::Lifted, (x + 60.0, y));
    finger(a, 1, Phase::Lifted, (x - 30.0, y));
    assert!(kept(a, &t.key).scale > before, "two fingers spread on a selected table scale it up");
}

/// 👆 The dashboard selected by a tap: where it is.
fn selected(a: &mut App) -> Pose {
    let t = dashboard(a);
    tap(a, t.rect.cx(), t.rect.cy());
    assert_eq!(a.pointer.tab.choisi.as_deref(), Some(t.key.as_str()), "a tap selects the table");
    placed(a, &t.key)
}

/// Which half of the chart the table is in: (the right one, the lower one).
fn half(a: &App, t: &Pose) -> (bool, bool) {
    (t.rect.cx() > a.chart.viewport.plot_w() * 0.5, t.rect.cy() > a.chart.viewport.plot_h() * 0.5)
}

/// 👆 The tables under the finger: the phone and the two tablets.
fn by_the_finger(l: &mut List) {
    for form in [Form::Phone, Form::TabletWide, Form::TabletTall] {
        // a tap selects it: the hint pill and its « OK »
        l.add(form, "table/selected", &[], |a| {
            selected(a);
        });
        // then a finger moves it, towards the middle of the chart
        l.add(form, "table/selected/moved", &[], |a| {
            let t = selected(a);
            let (right, low) = half(a, &t);
            slid(
                a,
                (t.rect.cx(), t.rect.cy()),
                (if right { -90.0 } else { 90.0 }, if low { -90.0 } else { 90.0 }),
            );
            assert!(kept(a, &t.key).moved, "a finger slid on a selected table moves it");
            assert_eq!(a.pointer.tab.choisi.as_deref(), Some(t.key.as_str()), "the table moved stays selected");
        });
        // a finger pulls the corner nearest to the middle of the chart towards it: everything scales
        l.add(form, "table/selected/corner", &[], |a| {
            let t = selected(a);
            let (right, low) = half(a, &t);
            let r = t.rect;
            let at = (
                if right { r.x + 10.0 } else { r.x + r.w - 10.0 },
                if low { r.y + 10.0 } else { r.y + r.h - 10.0 },
            );
            slid(a, at, (if right { -30.0 } else { 30.0 }, if low { -30.0 } else { 30.0 }));
            assert!(kept(a, &t.key).scale > 1.0, "a corner of a selected table pulled away by a finger scales it up");
        });
        // two fingers spread on it: bigger
        l.add(form, "table/selected/spread", &[], |a| {
            let t = selected(a);
            spread(a, &t);
        });
        // « OK »: done, the table kept at the size the fingers gave it
        l.add(form, "table/selected/spread/ok", &[], |a| {
            let t = selected(a);
            spread(a, &t);
            laid_out(a);
            let ok = tb::ok_cible();
            tap(a, ok.cx(), ok.cy());
            assert!(a.pointer.tab.choisi.is_none(), "« OK » ends the selection of the table");
            assert!(kept(a, &t.key).scale > 1.0, "« OK » keeps the table at the size the fingers gave it");
        });
        // a tap elsewhere on the chart: done
        l.add(form, "table/selected/elsewhere", &[], |a| {
            let t = selected(a).rect;
            let (w, h) = (a.chart.viewport.plot_w(), a.chart.viewport.plot_h());
            let around = Rect::new(t.x - 48.0, t.y - 48.0, t.w + 96.0, t.h + 96.0);
            let ok = tb::ok_cible();
            let free = [(0.5, 0.6), (0.5, 0.3), (0.25, 0.5), (0.75, 0.5)]
                .into_iter()
                .map(|(x, y)| (w * x, h * y))
                .find(|(x, y)| !around.contains(*x, *y) && !ok.contains(*x, *y));
            let (x, y) = free.expect("a point of the chart off the table");
            tap(a, x, y);
            assert!(a.pointer.tab.choisi.is_none(), "a tap elsewhere ends the selection of the table");
        });
        // the list of the legend unfolded: the dashboard, and the table of a script that asks for the top left of the
        // chart, kept under the lines of the list
        l.add(form, "table/with-the-legend", &[], |a| {
            a.toggle_indicator(Ind::Sma);
            dashboard(a);
            list_unfolded(a);
        });
        l.add(form, "table/of-script/with-the-legend", &[], of_script_with_the_legend);
    }
}

/// 📋 The scenes of the tables.
pub(super) fn scenes(l: &mut List) {
    by_the_mouse(l);
    by_the_finger(l);
}
