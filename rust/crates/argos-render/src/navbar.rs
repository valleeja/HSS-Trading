//! 🔍 The navigation pill (Jee, 17 Nov): − + ‹ › ↺ ↶ ↷ (Zen ⏪ on the main chart), centred at the
//! bottom of the chart, above the economic-event row (never over its flags), in the zoom pill's style (zoompad: one
//! source). Over a button, a bubble shows its name and shortcut, read from the single action table (`actions`).

use crate::actions::{self, ChartAction};
use crate::palette as pal;
use crate::palette::WHITE;
use crate::shapes::Rect;
use crate::text::{Align, Text};
use crate::viewport::Viewport;
use crate::{fill, Theme};
use std::cell::Cell;
use tiny_skia::Pixmap;

/// 🖥 Margin around the pill where the mouse shows it (Jee, 17 Nov: « like the bottom bar, only when the mouse
/// comes into its zone »).
pub const MARGE_SURVOL: f64 = 24.0;

thread_local! {
    static AU_SURVOL: Cell<bool> = const { Cell::new(false) };
}

/// 🖥 The pill shows only while the mouse is in its zone (`true`, the PC) or always (`false`, the default).
pub fn set_au_survol(on: bool) {
    AU_SURVOL.with(|c| c.set(on));
}

/// Whether the pill is shown: always, or (on the PC) the mouse in its place grown by `MARGE_SURVOL` on every side.
pub fn visible(vp: &Viewport, bottom: f64, mouse: Option<(f64, f64)>, full: bool) -> bool {
    if !AU_SURVOL.with(|c| c.get()) {
        return true;
    }
    let r = rect(vp, bottom, full);
    mouse.is_some_and(|(x, y)| x >= r.x - MARGE_SURVOL && x <= r.x + r.w + MARGE_SURVOL && y >= r.y - MARGE_SURVOL && y <= r.y + r.h + MARGE_SURVOL)
}

/// Distance of the buttons' centre above the bottom of the chart: the pill (40 high) ends 34 above it, over the
/// economic-event row (28 high, 6 above the bottom) and the floating footer's bar.
const LIFT: f64 = 54.0;
/// Demi-côté d'un bouton (carré arrondi de 28 px, comme TradingView).
const R: f64 = 14.0;
const GAP: f64 = 36.0;

/// Les boutons, de gauche à droite.
pub const BUTTONS: [(ChartAction, &str); 7] = [
    (ChartAction::ZoomOut, "−"),
    (ChartAction::ZoomIn, "+"),
    (ChartAction::ScrollLeft, "‹"),
    (ChartAction::ScrollRight, "›"),
    (ChartAction::ResetView, "↺"),
    (ChartAction::Undo, "↶"),
    (ChartAction::Redo, "↷"),
];

/// Un bouton de la barre : une action de la chart, 🧘 Zen ou ⏪ Replay (chart principale : `full`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NavBtn {
    Action(ChartAction),
    Zen,
    Replay,
}

/// Boutons de la barre : − + ‹ › ↺, puis Zen et Replay sur la chart principale (`full`).
pub fn buttons(full: bool) -> Vec<NavBtn> {
    let mut v: Vec<NavBtn> = BUTTONS.iter().map(|(a, _)| NavBtn::Action(*a)).collect();
    if full {
        v.extend([NavBtn::Zen, NavBtn::Replay]);
    }
    v
}

/// Demi-largeur d'un bouton (Zen : plus large, il porte son nom).
fn half_w(b: NavBtn) -> f64 {
    if b == NavBtn::Zen {
        R * 1.6
    } else {
        R
    }
}

/// Centres des boutons (petit espace en plus entre le zoom et le défilement, puis avant ↺, puis avant Zen).
pub fn centers(vp: &Viewport, bottom: f64, full: bool) -> Vec<(f64, f64)> {
    let y = bottom - LIFT;
    // − + ‹ › ↺ ↶ ↷ (puis Zen ⏪ sur la chart principale), comme la maquette
    // (centred: the pill's ends are symmetric around the middle of the chart)
    let offs: &[f64] = if full {
        &[-4.4, -3.4, -2.3, -1.3, -0.2, 0.8, 1.8, 3.1, 4.4]
    } else {
        &[-3.0, -2.0, -1.0, 0.0, 1.0, 2.0, 3.0]
    };
    let cx = vp.plot_w() / 2.0;
    // a narrow chart (wide side panel): the buttons come closer so the pill stays inside the chart
    let span = offs[offs.len() - 1] - offs[0];
    let gap = ((vp.plot_w() - 2.0 * R - 24.0) / span).clamp(2.0 * R + 2.0, GAP);
    offs.iter().map(|o| (cx + o * gap, y)).collect()
}

/// The glass pill around the buttons (chart coordinates).
pub fn rect(vp: &Viewport, bottom: f64, full: bool) -> Rect {
    let c = centers(vp, bottom, full);
    let (a, b) = (c[0], c[c.len() - 1]);
    Rect::new(a.0 - R - 8.0, a.1 - R - 6.0, b.0 - a.0 + 2.0 * R + 16.0, 2.0 * R + 12.0)
}

/// Bouton sous la souris (barre visible seulement).
pub fn hit(vp: &Viewport, bottom: f64, mouse: Option<(f64, f64)>, full: bool) -> Option<NavBtn> {
    if !visible(vp, bottom, mouse, full) {
        return None;
    }
    let (x, y) = mouse?;
    centers(vp, bottom, full)
        .into_iter()
        .zip(buttons(full))
        .find(|((cx, cy), b)| (x - cx).abs() <= half_w(*b) + 2.0 && (y - cy).abs() <= R + 2.0)
        .map(|(_, b)| b)
}

/// Dessine la barre (si visible) et la bulle du bouton survolé. `zen` : `Some(allumé)` sur la chart principale, qui a
/// aussi les boutons Zen et Replay.
pub fn draw(pm: &mut Pixmap, text: &Text, vp: &Viewport, bottom: f64, mouse: Option<(f64, f64)>, th: &Theme, zen: Option<bool>) {
    let full = zen.is_some();
    if !visible(vp, bottom, mouse, full) {
        return;
    }
    let hovered = hit(vp, bottom, mouse, full);
    // 🫧 the style of the zoom pill (zoompad: one source): its glass behind the buttons, round hover patches,
    // no square boxes, its glyphs and ink; the buttons, their order, place and click zones are unchanged
    crate::zoompad::fond(pm, rect(vp, bottom, full));
    let white = WHITE;
    for ((cx, cy), b) in centers(vp, bottom, full).iter().zip(buttons(full)) {
        let on = b == NavBtn::Zen && zen == Some(true);
        let hw = half_w(b);
        let case = Rect::new(cx - hw, cy - R, 2.0 * hw, 2.0 * R);
        if hovered == Some(b) || on {
            crate::zoompad::survol(pm, case);
        }
        match b {
            NavBtn::Action(a) => crate::zoompad::glyphe(pm, text, case, BUTTONS.iter().find(|x| x.0 == a).map_or("", |x| x.1)),
            NavBtn::Zen => text.draw(pm, "Zen", (*cx as f32, (*cy + 4.5) as f32), 12.5, crate::zoompad::ENCRE, Align::Center),
            NavBtn::Replay => {
                let _ = crate::phosphor::draw(pm, "rewind", *cx, *cy, 16, crate::zoompad::ENCRE);
            }
        }
    }
    // bulle : « Grossissement | Ctrl + ↑ »
    if let Some(b) = hovered {
        let i = buttons(full).iter().position(|x| *x == b).unwrap_or(0);
        let (cx, cy) = centers(vp, bottom, full)[i];
        let (name, sc) = bulle(b);
        let (wn, ws) = (text.width(name, 13.0) as f64, text.width(&sc, 12.0) as f64);
        let w = wn + if sc.is_empty() { 0.0 } else { ws + 26.0 } + 20.0;
        let (bx, by) = (bulle_x(cx, w, vp.plot_w()), cy - R - 38.0);
        crate::shapes::round_fill(pm, crate::shapes::Rect::new(bx, by, w, 28.0), 14.0, pal::hover(), 1.0);
        text.draw(pm, name, ((bx + 10.0) as f32, (by + 19.0) as f32), 13.0, white, Align::Left);
        if !sc.is_empty() {
            let sx = bx + 10.0 + wn + 8.0;
            fill(pm, sx, by + 6.0, 1.0, 16.0, pal::knob(), 1.0);
            crate::shapes::round_fill(pm, crate::shapes::Rect::new(sx + 8.0, by + 5.0, ws + 10.0, 18.0), 9.0, pal::knob(), 1.0);
            text.draw(pm, &sc, ((sx + 13.0) as f32, (by + 19.0) as f32), 12.0, white, Align::Left);
        }
        let _ = th;
    }
}

/// Left of a tooltip `w` wide centred on `cx`, kept inside a chart `plot_w` wide (2 from each edge). In a pane
/// narrower than the tooltip (phone, split screen of 2 columns, ≈ 173 points) the right bound went under the left
/// one and `clamp` stopped the app (filet n° 39): the tooltip then starts at 2 and may pass the right edge.
fn bulle_x(cx: f64, w: f64, plot_w: f64) -> f64 {
    let (bas, haut) = (2.0, plot_w - w - 2.0);
    (cx - w / 2.0).min(haut).max(bas)
}

/// Tooltip of a button: its name and its shortcut (📱 phone view: the name only, there is no keyboard).
fn bulle(b: NavBtn) -> (&'static str, String) {
    match b {
        NavBtn::Action(a) => (actions::label(a), actions::shortcut_bulle(a).unwrap_or_default()),
        NavBtn::Zen if actions::tactile() => ("Mode Zen (la chart seule)", String::new()),
        NavBtn::Zen => ("Mode Zen (la chart seule)", "Échap pour sortir".to_string()),
        NavBtn::Replay => ("Replay : rejouer la chart bougie par bougie", String::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 📱 Jee (20 Nov): on the phone and the touch tablet the tooltips keep the name, without the keyboard shortcut;
    /// the PC keeps « Grossissement | Ctrl + ↑ ».
    #[test]
    fn bulles_sans_raccourci_sur_cell() {
        let zoom = NavBtn::Action(ChartAction::ZoomIn);
        actions::set_tactile(false);
        assert_eq!(bulle(zoom), ("Grossissement", "Ctrl + ↑".to_string()));
        assert_eq!(bulle(NavBtn::Zen).1, "Échap pour sortir");
        actions::set_tactile(true);
        assert_eq!(bulle(zoom), ("Grossissement", String::new()));
        assert!(bulle(NavBtn::Zen).1.is_empty());
        actions::set_tactile(false);
    }

    #[test]
    fn boutons_et_place() {
        let vp = Viewport {
            width: 1400,
            height: 800,
            ..Viewport::default()
        };
        let ph = vp.plot_h();
        assert_eq!(hit(&vp, ph, None, true), None);
        // above the economic-event row (ph − 28 .. ph − 6): the pill ends 34 above the bottom
        let r = rect(&vp, ph, true);
        assert!(r.y + r.h <= ph - 34.0 + 0.01 && (r.cx() - vp.plot_w() / 2.0).abs() < 0.5, "{r:?}");
        let c = centers(&vp, ph, false);
        assert_eq!(hit(&vp, ph, Some(c[1]), false), Some(NavBtn::Action(ChartAction::ZoomIn)));
        assert_eq!(hit(&vp, ph, Some(c[4]), false), Some(NavBtn::Action(ChartAction::ResetView)));
        assert_eq!(hit(&vp, ph, Some((c[2].0, c[2].1 - 40.0)), false), None);
        // ↶ ↷ (maquette) après ↺
        assert_eq!(hit(&vp, ph, Some(c[5]), false), Some(NavBtn::Action(ChartAction::Undo)));
        assert_eq!(hit(&vp, ph, Some(c[6]), false), Some(NavBtn::Action(ChartAction::Redo)));
        // chart principale : un seul groupe − + ‹ › ↺ ↶ ↷ Zen Replay, sans chevauchement
        let f = centers(&vp, ph, true);
        assert_eq!(f.len(), 9);
        assert_eq!(hit(&vp, ph, Some(f[7]), true), Some(NavBtn::Zen));
        assert_eq!(hit(&vp, ph, Some(f[8]), true), Some(NavBtn::Replay));
        for (i, p) in f.iter().enumerate() {
            assert_eq!(hit(&vp, ph, Some(*p), true), Some(buttons(true)[i]), "bouton {i} sous son centre");
        }
        // les bulles viennent de la table d'actions
        assert_eq!(actions::shortcut(BUTTONS[0].0).as_deref(), Some("Ctrl + ↓"));
        assert_eq!(actions::label(BUTTONS[3].0), "Faire défiler vers la droite");
    }

    /// 🪟 Filet n° 39: in a narrow pane (split screen of 2 columns on the phone, ≈ 173 points; 120) touching a button of
    /// the pill drew its tooltip and the app stopped (clamp with its right bound under its left one). Every button,
    /// PC and phone, in panes of 120 and 173 points: drawn, the tooltip starts inside the pane.
    #[test]
    fn bulle_dans_une_case_etroite() {
        assert_eq!(bulle_x(60.0, 200.0, 120.0), 2.0);
        assert_eq!(bulle_x(86.0, 200.0, 173.0), 2.0, "wider than the pane: from its left edge");
        assert_eq!(bulle_x(86.0, 150.0, 173.0), 11.0, "it fits: centred");
        assert_eq!(bulle_x(700.0, 100.0, 1400.0), 650.0, "wide: centred as before");
        assert_eq!(bulle_x(1390.0, 100.0, 1400.0), 1298.0, "kept 2 from the right edge");
        let t = Text::default();
        for tactile in [false, true] {
            actions::set_tactile(tactile);
            for largeur in [120u32, 173] {
                let mut vp = Viewport {
                    height: 400,
                    ..Viewport::default()
                };
                vp.width = largeur + vp.price_axis;
                let ph = vp.plot_h();
                for full in [false, true] {
                    for c in centers(&vp, ph, full) {
                        let mut pm = Pixmap::new(vp.width, vp.height).unwrap();
                        draw(&mut pm, &t, &vp, ph, Some(c), &Theme::default(), full.then_some(false));
                    }
                }
            }
        }
        actions::set_tactile(false);
    }

    /// 🖥 Jee (17 Nov): on the PC the pill shows only when the mouse comes into its zone (its place + 24 around);
    /// elsewhere nothing is drawn and no button answers; off (cell, tablet, the default): always there.
    #[test]
    fn pilule_au_survol() {
        let vp = Viewport {
            width: 1400,
            height: 800,
            ..Viewport::default()
        };
        let ph = vp.plot_h();
        let r = rect(&vp, ph, true);
        let c = centers(&vp, ph, true);
        let t = Text::default();
        let dessin = |m: Option<(f64, f64)>| {
            let mut pm = Pixmap::new(vp.width, vp.height).unwrap();
            draw(&mut pm, &t, &vp, ph, m, &Theme::default(), Some(false));
            pm.data().iter().any(|&b| b != 0)
        };
        set_au_survol(true);
        assert!(!visible(&vp, ph, None, true) && !dessin(None), "no mouse: hidden");
        assert!(!dessin(Some((r.cx(), r.y - MARGE_SURVOL - 5.0))), "just outside its zone: hidden");
        assert!(
            visible(&vp, ph, Some((r.cx(), r.y - MARGE_SURVOL + 2.0)), true),
            "at the edge of its zone: shown"
        );
        assert!(dessin(Some((r.x - 10.0, r.cy()))), "beside it, inside the margin: drawn");
        assert_eq!(
            hit(&vp, ph, Some(c[0]), true),
            Some(NavBtn::Action(ChartAction::ZoomOut)),
            "each button answers"
        );
        assert_eq!(hit(&vp, ph, Some(c[7]), true), Some(NavBtn::Zen));
        set_au_survol(false);
        assert!(visible(&vp, ph, None, true) && dessin(None), "off: always there");
    }
}
