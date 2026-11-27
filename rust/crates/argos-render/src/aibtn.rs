//! Bouton de l'assistant Argos (AI) : l'icône Argos AI (disque sombre, anneau blanc, « AI »), en bas à droite de la fenêtre, gros et bien visible.
//! Il est dessiné en dernier, par-dessus tout (pages, fenêtres, menus), et c'est lui qui reçoit le clic en premier.

use crate::palette as pal;
use crate::palette::WHITE;
use crate::shapes::{round_fill, round_stroke, Rect};
use std::sync::OnceLock;
use tiny_skia::Pixmap;

/// Taille du bouton (carré) et marge avec les bords de la fenêtre.
pub const SIZE: f64 = 68.0;
/// 🖥 Marge de fin d'une liste qui descend jusqu'en bas à droite (fenêtres 🔔, ⚙…) : la hauteur du rond, sa marge du
/// bas (14) et 8 ; la dernière ligne peut toujours défiler au-dessus du rond.
pub const FIN_DE_LISTE: f64 = SIZE + 14.0 + 8.0;
const MARGIN: f64 = 14.0;

/// Logo « AI » d'Argos en blanc (PNG 256 px recoloré, une seule fois).
fn ai_white() -> Option<&'static Pixmap> {
    static AI: OnceLock<Option<Pixmap>> = OnceLock::new();
    AI.get_or_init(|| {
        let mut p = Pixmap::decode_png(include_bytes!("../assets/argos-ai-256.png")).ok()?;
        for px in p.pixels_mut() {
            let a = px.alpha();
            *px = tiny_skia::PremultipliedColorU8::from_rgba(a, a, a, a)?;
        }
        Some(p)
    })
    .as_ref()
}

/// 🤖 Icône Argos AI : disque sombre, anneau blanc, « AI » blanc dedans. C'est l'icône de **tous** les boutons d'assistance.
pub fn ai_icon(pm: &mut Pixmap, cx: f64, cy: f64, size: f64) {
    let r = Rect::new(cx - size / 2.0, cy - size / 2.0, size, size);
    round_fill(pm, r, size / 2.0, pal::night(), 1.0);
    round_stroke(pm, r, size / 2.0, WHITE, (size / 18.0).max(1.2) as f32);
    if let Some(l) = ai_white() {
        let s = size * 0.62;
        let k = (s / l.width().max(1) as f64) as f32;
        let at = tiny_skia::Transform::from_row(k, 0.0, 0.0, k, (cx - s / 2.0) as f32, (cy - s / 2.0) as f32);
        let paint = tiny_skia::PixmapPaint {
            quality: tiny_skia::FilterQuality::Bicubic,
            ..Default::default()
        };
        pm.draw_pixmap(0, 0, l.as_ref(), &paint, crate::px::t().pre_concat(at), None);
    }
}

/// The colour Argos AI logo (PNG; also in the printed month report).
pub(crate) const LOGO_COULEUR: &[u8] = include_bytes!("../assets/argos-ai-couleur.png");

/// Logo Argos AI en couleur (celui de la maquette et de l'icône de l'app).
fn logo_couleur() -> Option<&'static Pixmap> {
    static L: OnceLock<Option<Pixmap>> = OnceLock::new();
    L.get_or_init(|| Pixmap::decode_png(LOGO_COULEUR).ok()).as_ref()
}

/// ✨ LE bouton Argos AI (maquette `#aipill`, Jee 2026-11-09) : une pilule ronde de verre transparent (fond sombre
/// translucide, fin liseré clair) et le logo Argos AI en couleur dedans. Le même partout (cell, PC, pages).
pub fn bouton_verre(pm: &mut Pixmap, r: Rect, survol: bool) {
    let rayon = r.h.min(r.w) / 2.0;
    round_fill(pm, r, rayon, crate::text::Rgb(20, 26, 40), 0.45);
    round_stroke(pm, r, rayon, crate::jetons::sur_fond(crate::jetons::LISERE), 1.0);
    if survol {
        round_fill(pm, r, rayon, WHITE, 0.12);
    }
    let s = r.w.min(r.h) * 0.64;
    logo(pm, r.cx() - s / 2.0, r.cy() - s / 2.0, s);
}

/// The colour Argos AI logo (the round Argos AI button's), `s` points wide, top-left at (`x`, `y`).
pub fn logo(pm: &mut Pixmap, x: f64, y: f64, s: f64) {
    if let Some(l) = logo_couleur() {
        let k = (s / l.width().max(1) as f64) as f32;
        let at = tiny_skia::Transform::from_row(k, 0.0, 0.0, k, x as f32, y as f32);
        let paint = tiny_skia::PixmapPaint {
            quality: tiny_skia::FilterQuality::Bicubic,
            ..Default::default()
        };
        pm.draw_pixmap(0, 0, l.as_ref(), &paint, crate::px::t().pre_concat(at), None);
    }
}

/// ✨ 🖥 The Argos AI pill (Jee, 17 Nov): the whole corner cell in glass, round ends, teal outline, the colour logo on
/// the left (≈ 70 % of the height) and « Argos AI » in light bold text, as large as fits; too narrow for the text:
/// the logo alone, centred.
pub fn pastille(pm: &mut Pixmap, r: Rect, survol: bool) {
    let rayon = r.h / 2.0;
    round_fill(pm, r, rayon, crate::text::Rgb(20, 26, 40), 0.55);
    round_stroke(pm, r, rayon, crate::jetons::CYAN, 1.2);
    if survol {
        round_fill(pm, r, rayon, WHITE, 0.12);
    }
    let s = (r.h * 0.7).round();
    let (texte, taille) = pastille_texte(r);
    let x_logo = if texte { r.x + 4.0 } else { r.cx() - s / 2.0 };
    if let Some(l) = logo_couleur() {
        let k = (s / l.width().max(1) as f64) as f32;
        let at = tiny_skia::Transform::from_row(k, 0.0, 0.0, k, x_logo as f32, (r.cy() - s / 2.0) as f32);
        let paint = tiny_skia::PixmapPaint {
            quality: tiny_skia::FilterQuality::Bicubic,
            ..Default::default()
        };
        pm.draw_pixmap(0, 0, l.as_ref(), &paint, crate::px::t().pre_concat(at), None);
    }
    if texte {
        let t = crate::text::Text::graisse(crate::text::Graisse::Bold);
        let x = x_logo + s + 3.0;
        let fin = r.x + r.w - 4.0;
        t.draw(
            pm,
            PASTILLE,
            (((x + fin) / 2.0) as f32, (r.cy() + taille as f64 * 0.36) as f32),
            taille,
            crate::jetons::TEXTE,
            crate::text::Align::Center,
        );
    }
}

/// The pill text.
const PASTILLE: &str = "Argos AI";

/// Does the text fit in the pill `r`, and at which size (the largest, from 13 down to 9 points)?
pub fn pastille_texte(r: Rect) -> (bool, f32) {
    let t = crate::text::Text::graisse(crate::text::Graisse::Bold);
    let libre = r.w - 8.0 - (r.h * 0.7).round() - 3.0;
    [13.0f32, 12.5, 12.0, 11.5, 11.0, 10.5, 10.0, 9.5, 9.0]
        .into_iter()
        .find(|&f| t.width(PASTILLE, f) as f64 <= libre && f as f64 <= r.h * 0.62)
        .map_or((false, 0.0), |f| (true, f))
}

/// Image of the pill, `w` × `h` points.
pub fn pastille_image(w: f64, h: f64, survol: bool) -> Option<Pixmap> {
    let mut pm = crate::px::new(w.ceil() as u32 + 1, h.ceil() as u32 + 1)?;
    pastille(&mut pm, Rect::new(0.5, 0.5, w, h), survol);
    Some(pm)
}

/// Image du bouton de `taille` points (pilule de verre et logo couleur).
pub fn image(taille: f64, survol: bool) -> Option<Pixmap> {
    let mut pm = crate::px::new(taille.ceil() as u32 + 1, taille.ceil() as u32 + 1)?;
    bouton_verre(&mut pm, Rect::new(0.5, 0.5, taille, taille), survol);
    Some(pm)
}

/// Place du bouton dans la fenêtre `win_w` × `win_h`.
pub fn rect(win_w: f64, win_h: f64) -> Rect {
    Rect::new(win_w - SIZE - MARGIN, win_h - SIZE - MARGIN, SIZE, SIZE)
}

/// Image du bouton (taille `SIZE`) ; `hover` : la souris est dessus.
pub fn draw(hover: bool) -> Option<Pixmap> {
    image(SIZE - 1.0, hover)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ✨ The Argos AI pill of the corner cell (78 × 28 minus 2 on each side): « Argos AI » fits, logo on the left;
    /// too narrow, the logo alone.
    #[test]
    fn pastille_de_la_case_du_coin() {
        let (tient, taille) = pastille_texte(Rect::new(0.0, 0.0, 74.0, 24.0));
        assert!(tient && taille >= 9.0);
        assert!(!pastille_texte(Rect::new(0.0, 0.0, 30.0, 24.0)).0, "too narrow: the logo alone");
        let pm = pastille_image(74.0, 24.0, false).unwrap();
        assert_eq!(pm.pixel(1, 1).unwrap().alpha(), 0, "round ends");
    }

    #[test]
    fn bouton_ai() {
        let r = rect(1400.0, 800.0);
        assert!(r.contains(1400.0 - MARGIN - 1.0, 800.0 - MARGIN - 1.0), "coin bas droit");
        assert!(draw(true).is_some());
        assert!(ai_white().is_some(), "logo AI recoloré");
        assert!(logo_couleur().is_some(), "logo Argos AI en couleur");
    }
}
