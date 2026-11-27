//! Formes de base partagées par les boutons, menus et fenêtres : rectangle (avec test de clic), rectangle arrondi
//! plein ou en contour, et traits fins pour les icônes. Écrit une seule fois ici.

use crate::palette as pal;
use crate::text::Rgb;
use tiny_skia::{FillRule, LineCap, LineJoin, Paint, Path, PathBuilder, Pixmap, Stroke};

/// Rectangle en pixels.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Rect {
    pub const fn new(x: f64, y: f64, w: f64, h: f64) -> Self {
        Self { x, y, w, h }
    }
    pub fn contains(&self, px: f64, py: f64) -> bool {
        px >= self.x && px <= self.x + self.w && py >= self.y && py <= self.y + self.h
    }
    pub fn cx(&self) -> f64 {
        self.x + self.w / 2.0
    }
    pub fn cy(&self) -> f64 {
        self.y + self.h / 2.0
    }
}

fn paint(c: Rgb, alpha: f32) -> Paint<'static> {
    let mut p = Paint::default();
    p.set_color_rgba8(c.0, c.1, c.2, (alpha.clamp(0.0, 1.0) * 255.0) as u8);
    p.anti_alias = !crate::px::brouillon();
    p
}

pub(crate) fn round_path(r: Rect, radius: f64) -> Option<Path> {
    // 🫧 pills with round ends everywhere (jetons::rayon: bars, buttons, chips, fields, bubbles; cards keep theirs)
    let radius = crate::jetons::rayon(r, radius);
    let (x, y, w, h) = (r.x as f32, r.y as f32, r.w as f32, r.h as f32);
    let k = (radius as f32).min(w / 2.0).min(h / 2.0);
    // 🫧 true quarter circles (cubic, 0.5523 × the radius): the quadratic corners before were squarer than a circle
    // (0.25 instead of 0.29 of the radius from the corner at their middle) — pills looked square-ended (Jee, 17 Nov)
    let c = 0.552_284_8 * k;
    let mut pb = PathBuilder::new();
    pb.move_to(x + k, y);
    pb.line_to(x + w - k, y);
    pb.cubic_to(x + w - k + c, y, x + w, y + k - c, x + w, y + k);
    pb.line_to(x + w, y + h - k);
    pb.cubic_to(x + w, y + h - k + c, x + w - k + c, y + h, x + w - k, y + h);
    pb.line_to(x + k, y + h);
    pb.cubic_to(x + k - c, y + h, x, y + h - k + c, x, y + h - k);
    pb.line_to(x, y + k);
    pb.cubic_to(x, y + k - c, x + k - c, y, x + k, y);
    pb.close();
    pb.finish()
}

/// Rectangle arrondi plein (edges on whole screen pixels: no blurred border).
pub fn round_fill(pm: &mut Pixmap, r: Rect, radius: f64, c: Rgb, alpha: f32) {
    let (x, y, w, h) = crate::px::snap_rect(r.x, r.y, r.w, r.h);
    if let Some(path) = round_path(Rect::new(x, y, w, h), radius) {
        pm.fill_path(&path, &paint(c, alpha), FillRule::Winding, crate::px::t(), None);
    }
}

/// 🫧 Peint `c` dans `outer` autour du rectangle arrondi `inner` (les coins et la marge d'une carte en verre posée
/// sur une barre : la carte flotte, comme dans la maquette).
pub fn autour(pm: &mut Pixmap, outer: Rect, inner: Rect, radius: f64, c: Rgb) {
    let mut pb = PathBuilder::new();
    pb.push_rect(
        match tiny_skia::Rect::from_xywh(outer.x as f32, outer.y as f32, outer.w as f32, outer.h as f32) {
            Some(r) => r,
            None => return,
        },
    );
    let Some(carte) = round_path(inner, radius) else { return };
    pb.push_path(&carte);
    if let Some(path) = pb.finish() {
        pm.fill_path(&path, &paint(c, 1.0), FillRule::EvenOdd, crate::px::t(), None);
    }
}

/// Contour d'un rectangle arrondi (straight sides on the pixel grid: a thin outline stays one sharp pixel wide).
pub fn round_stroke(pm: &mut Pixmap, r: Rect, radius: f64, c: Rgb, width: f32) {
    round_stroke_alpha(pm, r, radius, c, width, 1.0);
}

/// `round_stroke` in transparency (`alpha` 0–1).
pub fn round_stroke_alpha(pm: &mut Pixmap, r: Rect, radius: f64, c: Rgb, width: f32, alpha: f32) {
    let (x0, y0) = (crate::px::snap_line(r.x, width), crate::px::snap_line(r.y, width));
    let (x1, y1) = (crate::px::snap_line(r.x + r.w, width), crate::px::snap_line(r.y + r.h, width));
    if let Some(path) = round_path(Rect::new(x0, y0, x1 - x0, y1 - y0), radius) {
        let stroke = Stroke { width, ..Stroke::default() };
        pm.stroke_path(&path, &paint(c, alpha), &stroke, crate::px::t(), None);
    }
}

/// Traits fins (icônes) : chaque liste de points est une ligne brisée, autour du centre `(cx, cy)`.
pub fn lines(pm: &mut Pixmap, cx: f64, cy: f64, parts: &[&[(f32, f32)]], c: Rgb, width: f32) {
    lines_alpha(pm, cx, cy, parts, c, width, 1.0);
}

/// `lines` en transparence (`alpha` 0–1 : couleurs des scripts).
pub fn lines_alpha(pm: &mut Pixmap, cx: f64, cy: f64, parts: &[&[(f32, f32)]], c: Rgb, width: f32, alpha: f32) {
    let (cx, cy) = (cx as f32, cy as f32);
    let mut pb = PathBuilder::new();
    for pts in parts.iter().filter(|p| !p.is_empty()) {
        pb.move_to(cx + pts[0].0, cy + pts[0].1);
        for (x, y) in &pts[1..] {
            pb.line_to(cx + x, cy + y);
        }
    }
    let Some(path) = pb.finish() else { return };
    let stroke = Stroke {
        width,
        line_cap: LineCap::Round,
        line_join: LineJoin::Round,
        ..Stroke::default()
    };
    pm.stroke_path(&path, &paint(c, alpha), &stroke, crate::px::t(), None);
}

/// Polygone plein (points autour du centre `(cx, cy)`).
pub fn poly_fill(pm: &mut Pixmap, cx: f64, cy: f64, pts: &[(f32, f32)], c: Rgb, alpha: f32) {
    let Some(first) = pts.first() else { return };
    let (cx, cy) = (cx as f32, cy as f32);
    let mut pb = PathBuilder::new();
    pb.move_to(cx + first.0, cy + first.1);
    for (x, y) in &pts[1..] {
        pb.line_to(cx + x, cy + y);
    }
    pb.close();
    if let Some(path) = pb.finish() {
        pm.fill_path(&path, &paint(c, alpha), FillRule::Winding, crate::px::t(), None);
    }
}

/// Points d'un arc de cercle (degrés, 0 = en haut, sens horaire), pour `lines`.
pub fn arc(r: f32, from_deg: f32, to_deg: f32, steps: usize) -> Vec<(f32, f32)> {
    (0..=steps)
        .map(|k| {
            let t = (from_deg + (to_deg - from_deg) * k as f32 / steps as f32).to_radians();
            (r * t.sin(), -r * t.cos())
        })
        .collect()
}

/// 🛒 Carte d'un menu, au style de la page Marketplace (tous les menus d'Argos) : coins arrondis, bordure.
pub fn menu_card(pm: &mut Pixmap, r: Rect) {
    // 🫧 style verre : coins plus arrondis (le verre et le reflet sont posés à l'affichage, voir `glass::blit`)
    // (maquette `.pop{border-radius:16px}` : le même rayon avec ou sans verre)
    let k = crate::jetons::R_MENU;
    round_fill(pm, r, k, pal::panel(), 1.0);
    round_stroke(pm, r, k, pal::edge(), 1.0);
}

/// Fond d'une ligne de menu (maquette `.pop .ligne`) : pilule à bouts ronds, sans liseré ; choisie = bleu Argos
/// teinté, survol = gris.
pub fn menu_row(pm: &mut Pixmap, r: Rect, on: bool, hover: bool) {
    let k = r.h / 2.0;
    if on {
        round_fill(pm, r, k, crate::palette::accent(), 0.25);
    } else if hover {
        round_fill(pm, r, k, pal::hover(), 1.0);
    }
}

/// Pastille de choix (maquette `.puce`) : pilule à bouts ronds ; choisie = `.puce.on` (bleu), sinon le verre 2 (plus
/// clair au survol).
pub fn pilule(pm: &mut Pixmap, r: Rect, on: bool, hover: bool) {
    let (c, a) = if on { crate::jetons::PUCE_ON } else { crate::jetons::VERRE2 };
    round_fill(pm, r, r.h / 2.0, c, a + if hover && !on { 0.06 } else { 0.0 });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rectangle_et_dessin() {
        let r = Rect::new(10.0, 10.0, 20.0, 10.0);
        assert!(r.contains(15.0, 15.0) && !r.contains(31.0, 15.0));
        assert_eq!((r.cx(), r.cy()), (20.0, 15.0));
        let mut pm = crate::px::new(40, 40).unwrap();
        round_fill(&mut pm, r, 4.0, Rgb(255, 255, 255), 1.0);
        assert_eq!(pm.pixel(20, 15).unwrap().red(), 255);
        assert_eq!(pm.pixel(10, 10).unwrap().alpha(), 0, "coin arrondi");
        lines(&mut pm, 5.0, 30.0, &[&[(0.0, 0.0), (30.0, 0.0)]], Rgb(255, 0, 0), 2.0);
        assert!(pm.pixel(20, 30).unwrap().red() > 0);
        assert_eq!(arc(5.0, 0.0, 180.0, 2)[0], (0.0, -5.0));
    }
}
