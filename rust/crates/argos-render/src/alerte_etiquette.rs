//! 🔔 The label of a chosen alert line (Jee, 17 Nov, « ok alerte »): on the line, left of its bell — the alert's name
//! cut with « … » in a rounded glass box, a separator, 🗑 (delete); and a small round blue handle on the line to drag
//! the alert to another price (its price label on the scale follows, as the line).

use crate::shapes::{round_fill, round_stroke, Rect};
use crate::text::{Align, Rgb, Text};
use crate::{fill, jetons as j};
use tiny_skia::Pixmap;

/// Its places on the chart (chart coordinates).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Places {
    /// The whole box (name, separator, 🗑).
    pub boite: Rect,
    pub poubelle: Rect,
    /// The round handle on the line, left of the box.
    pub poignee: Rect,
}

const H: f64 = 26.0;
const NOM_MAX: f64 = 170.0;
const TAILLE: f32 = 12.5;
/// The bell at the end of the line (drawn by the chart, 12 from the price scale): the box stops before it.
const CLOCHE: f64 = 26.0;

/// The name as shown: cut with « … » to fit the box.
pub fn nom_coupe(text: &Text, nom: &str) -> String {
    text.fit(nom, TAILLE, NOM_MAX)
}

/// Its places for the line at height `y` on a chart `pw` wide.
pub fn places(text: &Text, nom: &str, y: f64, pw: f64) -> Places {
    let nw = text.width(&nom_coupe(text, nom), TAILLE) as f64;
    let w = 10.0 + nw + 10.0 + 1.0 + H;
    let boite = Rect::new((pw - CLOCHE - 6.0 - w).max(30.0).round(), (y - H / 2.0).round(), w, H);
    let poubelle = Rect::new(boite.x + boite.w - H, boite.y, H, H);
    Places {
        boite,
        poubelle,
        poignee: Rect::new(boite.x - 10.0 - 14.0, y - 7.0, 14.0, 14.0),
    }
}

/// 🔔 How the chart draws the alert lines (Jee, 19 Nov, his TradingView captures).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Vue {
    /// Prices of the paused alerts: a dimmed grey line (their label says « (en pause) »).
    pub pause: Vec<f64>,
    /// The one colour chosen for all alert lines (line, short form, handle); `None`: as before (gold line, blue
    /// handle).
    pub couleur: Option<Rgb>,
    /// « Lignes d'alerte » unchecked: no full-width line; the short form by the price scale (drawn by the app).
    pub courtes: bool,
}

impl Vue {
    /// The colour of an alert line (grey when paused).
    pub fn ligne(&self, pause: bool) -> Rgb {
        if pause {
            crate::palette::GREY
        } else {
            self.couleur.unwrap_or(crate::palette::GOLD)
        }
    }

    /// The colour of its round handle (grey when paused).
    pub fn poignee(&self, pause: bool) -> Rgb {
        if pause {
            crate::palette::GREY
        } else {
            self.couleur.unwrap_or(j::ACCENT)
        }
    }
}

/// The short form's dashed segment, from the handle to the price scale.
pub const TIRET: f64 = 24.0;

/// The places of the short form (« Lignes d'alerte » unchecked), glued to the price scale: the box, then the handle,
/// then the dashed segment to the scale.
pub fn places_courte(text: &Text, nom: &str, y: f64, pw: f64) -> Places {
    let nw = text.width(&nom_coupe(text, nom), TAILLE) as f64;
    let w = 10.0 + nw + 10.0 + 1.0 + H;
    let poignee = Rect::new(pw - TIRET - 14.0, y - 7.0, 14.0, 14.0);
    let boite = Rect::new((poignee.x - 6.0 - w).max(30.0).round(), (y - H / 2.0).round(), w, H);
    Places {
        boite,
        poubelle: Rect::new(boite.x + boite.w - H, boite.y, H, H),
        poignee,
    }
}

/// Draws it (`mouse` in chart coordinates: the 🗑 lights up under it); `poignee`: the handle's colour.
pub fn draw(pm: &mut Pixmap, text: &Text, nom: &str, y: f64, pw: f64, mouse: Option<(f64, f64)>, poignee: Rgb) {
    let p = places(text, nom, y, pw);
    boite(pm, text, nom, &p, mouse);
    rond(pm, p.poignee, poignee);
}

/// Draws the short form: the label, the handle (`poignee`), the dashed segment (`ligne`) to the price scale.
pub fn draw_courte(pm: &mut Pixmap, text: &Text, nom: &str, y: f64, pw: f64, mouse: Option<(f64, f64)>, ligne: Rgb, poignee: Rgb) {
    let p = places_courte(text, nom, y, pw);
    let mut x = p.poignee.x + p.poignee.w + 2.0;
    while x < pw {
        fill(pm, x, y.round(), 4.0_f64.min(pw - x), 1.0, ligne, 0.9);
        x += 7.0;
    }
    boite(pm, text, nom, &p, mouse);
    rond(pm, p.poignee, poignee);
}

/// The glass box: the name, a separator, 🗑.
fn boite(pm: &mut Pixmap, text: &Text, nom: &str, p: &Places, mouse: Option<(f64, f64)>) {
    let r = p.boite;
    round_fill(pm, r, r.h / 2.0, j::FOND, 0.88);
    round_fill(pm, r, r.h / 2.0, j::VERRE.0, j::VERRE.1);
    round_stroke(pm, r, r.h / 2.0, j::sur_fond(j::LISERE), 1.0);
    text.draw(
        pm,
        &nom_coupe(text, nom),
        ((r.x + 10.0) as f32, (r.cy() + 4.5) as f32),
        TAILLE,
        j::TEXTE,
        Align::Left,
    );
    // separator, then 🗑
    fill(pm, p.poubelle.x, r.y + 6.0, 1.0, r.h - 12.0, j::sur_fond(j::LISERE), 1.0);
    if mouse.is_some_and(|(x, y)| p.poubelle.contains(x, y)) {
        let c = p.poubelle;
        round_fill(pm, Rect::new(c.x + 3.0, c.y + 3.0, c.w - 6.0, c.h - 6.0), (c.h - 6.0) / 2.0, j::BAS, 0.25);
    }
    let _ = crate::phosphor::draw(pm, "trash", p.poubelle.cx(), p.poubelle.cy(), 15, j::TEXTE);
}

/// The handle: a coloured round with a white ring.
fn rond(pm: &mut Pixmap, g: Rect, c: Rgb) {
    round_fill(pm, g, g.h / 2.0, Rgb(0xff, 0xff, 0xff), 1.0);
    round_fill(pm, Rect::new(g.x + 2.0, g.y + 2.0, g.w - 4.0, g.h - 4.0), (g.h - 4.0) / 2.0, c, 1.0);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The box sits on the line, left of the bell, inside the chart; a long name is cut with « … »; 🗑 at its right end;
    /// the handle on the line, left of it.
    #[test]
    fn etiquette_sur_la_ligne() {
        let t = Text::default();
        let long = "BTC monte au-dessus de 70 000 — alerte avec un très long message qui ne tient pas";
        let p = places(&t, long, 300.0, 1200.0);
        assert!((p.boite.cy() - 300.0).abs() <= 1.0 && (p.poignee.cy() - 300.0).abs() <= 1.0, "on the line");
        assert!(p.boite.x + p.boite.w <= 1200.0 - CLOCHE, "left of the bell");
        assert!(p.poignee.x + p.poignee.w < p.boite.x, "the handle left of the box");
        assert!(nom_coupe(&t, long).ends_with('…'), "a long name cut");
        assert_eq!(p.poubelle.x + p.poubelle.w, p.boite.x + p.boite.w, "🗑 at its right end");
        let mut pm = crate::px::new(1300, 600).unwrap();
        draw(&mut pm, &t, long, 300.0, 1200.0, None, j::ACCENT);
        let g = pm.pixel(p.poignee.cx() as u32, p.poignee.cy() as u32).unwrap();
        assert!(g.blue() > 200 && g.red() < 120, "the handle in blue: {g:?}");
    }

    /// The short form (« Lignes d'alerte » unchecked): the box, then the handle, then the dashed segment, glued to the
    /// price scale; in the chosen colour.
    #[test]
    fn forme_courte_collee_a_l_echelle() {
        let t = Text::default();
        let nom = "BTCUSDC.P Croisement 63 584,0";
        let p = places_courte(&t, nom, 300.0, 1200.0);
        assert!(p.boite.x + p.boite.w < p.poignee.x, "the box left of the handle");
        assert_eq!(p.poignee.x + p.poignee.w, 1200.0 - TIRET, "the handle, then the dashes to the scale");
        let rouge = Rgb(0xff, 0x20, 0x20);
        let mut pm = crate::px::new(1300, 600).unwrap();
        draw_courte(&mut pm, &t, nom, 300.0, 1200.0, None, rouge, rouge);
        let g = pm.pixel(p.poignee.cx() as u32, p.poignee.cy() as u32).unwrap();
        assert!(g.red() > 200 && g.blue() < 80, "the handle in the chosen colour: {g:?}");
        let d = pm.pixel(1200 - 6, 300).unwrap();
        assert!(d.red() > 150 && d.alpha() > 100, "a dash by the scale: {d:?}");
    }
}
