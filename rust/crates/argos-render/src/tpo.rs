//! 🔠 Dessin du type de chart **TPO** (comme LuxAlgo / Vela) : chaque session est un profil de lettres collées à gauche
//! (vue « packed »), une case par période et par rangée de prix ; couleur selon l'heure dans la session, cases hors de
//! la zone de valeur atténuées (45 %), POC surligné, lignes VAH / VAL, crochet de l'initial balance à gauche, single
//! prints ombrés, flèches d'ouverture et de clôture. Le calcul est dans `argos_script::indicators::tpo`.

use crate::indicators::Geo;
use crate::palette::{GOLD, GREY, WHITE};
use crate::text::{Align, Rgb, Text};
use argos_script::indicators::tpo::{Session, LETTERS};
use tiny_skia::Pixmap;

/// Rampe de couleur « Simple » selon l'heure : début de session (cyan) → fin (violet).
fn ramp(t: f64) -> Rgb {
    let (a, b) = ((0x22, 0xd3, 0xee), (0x8b, 0x5c, 0xf6));
    let m = |x: u8, y: u8| (x as f64 + (y as f64 - x as f64) * t.clamp(0.0, 1.0)).round() as u8;
    Rgb(m(a.0, b.0), m(a.1, b.1), m(a.2, b.2))
}

/// Opacité des cases hors de la zone de valeur.
const OUTSIDE_VA: f32 = 0.45;

/// `letters` / `blocks` : ce qu'on montre dans chaque case (les deux, ou un seul ; sans lettres = profil de chaleur).
pub fn draw(pm: &mut Pixmap, text: &Text, g: &Geo, sessions: &[Session], letters: bool, blocks: bool) {
    let cw = g.vp.bar_spacing.max(1.0);
    let right = g.vp.plot_w();
    for s in sessions.iter().filter(|s| s.last >= g.first && s.first <= g.last) {
        let x0 = g.vp.x_of(s.first, g.slots) - cw / 2.0;
        let y = |p: f64| g.scale.y_of(p);
        let row_h = (y(0.0) - y(s.step)).abs();
        // lettres seulement si elles ont la place ; sinon des blocs (pour que le profil reste visible)
        let room = cw >= 7.0 && row_h >= 8.0;
        let (show_l, show_b) = (letters && room, blocks || !(letters && room));
        let fs = (row_h.min(cw) as f32 - 1.0).clamp(7.0, 12.0);
        let width = s.rows.iter().map(Vec::len).max().unwrap_or(0) as f64 * cw;
        for (k, row) in s.rows.iter().enumerate() {
            let (top, bot) = (y(s.price(k) + s.step), y(s.price(k)));
            let (ry, rh) = (top.min(bot), ((bot - top).abs() - if row_h > 4.0 { 1.0 } else { 0.0 }).max(1.0));
            if ry + rh < 0.0 || ry > g.price_h {
                continue;
            }
            let in_va = (s.val..=s.vah).contains(&k);
            // single print : la rangée est ombrée sur toute la largeur du profil
            if s.singles.contains(&k) {
                crate::fill(pm, x0, ry, width, rh, GOLD, 0.10);
            }
            for (j, &p) in row.iter().enumerate() {
                let x = x0 + j as f64 * cw;
                if x > right {
                    break;
                }
                let c = ramp(p as f64 / (s.periods.max(2) - 1) as f64);
                let a = if in_va { 1.0 } else { OUTSIDE_VA };
                if show_b {
                    crate::fill(pm, x, ry, (cw - 1.0).max(1.0), rh, c, if show_l { 0.22 } else { 0.8 } * a);
                }
                if show_l {
                    let ch = (LETTERS[p as usize] as char).to_string();
                    text.draw(
                        pm,
                        &ch,
                        ((x + cw / 2.0) as f32, (ry + rh / 2.0 + fs as f64 * 0.36) as f32),
                        fs,
                        if k == s.poc { GOLD } else { c },
                        Align::Center,
                    );
                }
            }
            // POC : contour doré de la rangée
            if k == s.poc {
                let w = row.len() as f64 * cw;
                crate::fill(pm, x0, ry, w, 1.0, GOLD, 0.9);
                crate::fill(pm, x0, ry + rh - 1.0, w, 1.0, GOLD, 0.9);
            }
        }
        // VAH / VAL : traits fins sur la largeur du profil
        for k in [s.vah + 1, s.val] {
            let yy = y(s.price(k)).round();
            if (0.0..g.price_h).contains(&yy) {
                crate::fill(pm, x0, yy, width, 1.0, GREY, 0.7);
            }
        }
        // initial balance : crochet à gauche du profil
        if s.ib.1 > s.ib.0 {
            let (a, b) = (y(s.ib.1), y(s.ib.0));
            crate::fill(pm, x0 - 3.0, a.min(b), 2.0, (b - a).abs(), WHITE, 0.8);
        }
        // ouverture ◂ et clôture ▸
        for (p, xx, s_) in [(s.open, x0 - 7.0, "▸"), (s.close, x0 + width + 2.0, "◂")] {
            text.draw(pm, s_, (xx as f32, (y(p) + 4.0) as f32), 10.0, WHITE, Align::Left);
        }
    }
}
