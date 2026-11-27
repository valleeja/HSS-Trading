//! 🔍 The pill style (glass, round ends, round hover patches, glyph ink), the single source for the navigation pill
//! (navbar.rs); also `PadBtn`, the buttons of the phone favourites bar (− + RAZ Zen ⏪), and `Pad` (Zen on).

use crate::actions::ChartAction;
use crate::shapes::{round_fill, round_stroke, Rect};
use crate::text::{Align, Text};
use tiny_skia::Pixmap;

/// A button of the phone favourites bar.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PadBtn {
    Action(ChartAction),
    Zen,
    /// ⏪ Replay (phone favourites bar).
    Replay,
}

/// What the chart needs to know for the navigation pill: Zen mode on (its Zen button lit).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pad {
    pub zen: bool,
}

// — the pill style, the single source for the zoom pill and the navigation bar (navbar.rs) —

/// The glass of a pill: round ends (radius = half the height), glass fill, light outline.
pub fn fond(pm: &mut Pixmap, s: Rect) {
    use crate::jetons as j;
    round_fill(pm, s, s.h / 2.0, j::sur_fond(j::VERRE), 0.92);
    round_stroke(pm, s, s.h / 2.0, j::sur_fond(j::LISERE), 1.0);
}

/// A hovered button of a pill: a light round patch, no square box.
pub fn survol(pm: &mut Pixmap, r: Rect) {
    round_fill(pm, r, r.h / 2.0, crate::jetons::BLANC, 0.12);
}

/// The ink of a pill's glyphs and icons: also every icon of the drawing bar (left), the favourites bar and the tool
/// menus (Jee, 17 Nov: « tout gris et clean »), never the category colours.
pub const ENCRE: crate::text::Rgb = crate::jetons::TEXTE;

/// The light translucent patch of a chosen item (active tool, current timeframe, chosen row): grey, never blue.
pub const CHOISI: (crate::text::Rgb, f32) = (crate::jetons::BLANC, 0.16);

/// A button's glyph, centred (↺ a little smaller, like the zoom pill).
pub fn glyphe(pm: &mut Pixmap, text: &Text, r: Rect, label: &str) {
    let size = if label == "↺" { 15.0 } else { 18.0 };
    text.draw(pm, label, (r.cx() as f32, (r.cy() + size as f64 * 0.36) as f32), size, ENCRE, Align::Center);
}
