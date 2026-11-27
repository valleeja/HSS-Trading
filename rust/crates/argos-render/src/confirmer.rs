//! ✅ A confirmation box, reusable (Jee, 17 Nov: « Supprimer cette alerte? »): a title, a sentence, a button that
//! does it (red when it destroys something), « Annuler » and ✕ — in glass, centred over a darkened window.

use crate::shapes::{round_fill, round_stroke, Rect};
use crate::text::{Align, Text};
use crate::{fill, jetons as j};
use tiny_skia::Pixmap;

/// The box.
#[derive(Debug, Clone, PartialEq)]
pub struct Confirmer {
    pub titre: String,
    pub texte: String,
    /// The button that does it (« Effacer »).
    pub ok: String,
    /// Red (it destroys something) or blue.
    pub danger: bool,
}

/// What a click did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmerClic {
    Ok,
    Annuler,
}

/// Its places in the window.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Places {
    pub carte: Rect,
    pub fermer: Rect,
    pub ok: Rect,
    pub annuler: Rect,
}

const LARGEUR: f64 = 420.0;
const MARGE: f64 = 22.0;
const BOUTON_H: f64 = 44.0;
const LIGNE: f64 = 21.0;

impl Confirmer {
    /// The sentence cut into lines fitting `w`.
    fn lignes(&self, text: &Text, w: f64) -> Vec<String> {
        let mut out: Vec<String> = vec![];
        let mut cur = String::new();
        for mot in self.texte.split(' ') {
            let essai = if cur.is_empty() { mot.to_string() } else { format!("{cur} {mot}") };
            if text.width(&essai, 14.0) as f64 > w && !cur.is_empty() {
                out.push(std::mem::replace(&mut cur, mot.to_string()));
            } else {
                cur = essai;
            }
        }
        if !cur.is_empty() {
            out.push(cur);
        }
        out
    }

    /// Its places in a window `w` × `h` (centred; on a phone: the whole width less 16 on each side).
    pub fn places(&self, text: &Text, w: f64, h: f64) -> Places {
        let cw = LARGEUR.min(w - 32.0).max(200.0);
        let n = self.lignes(text, cw - 2.0 * MARGE).len() as f64;
        let ch = MARGE + 26.0 + 12.0 + n * LIGNE + 22.0 + BOUTON_H + MARGE;
        let carte = Rect::new(((w - cw) / 2.0).round(), ((h - ch) / 2.0).round().max(8.0), cw, ch);
        let bw = (cw - 2.0 * MARGE - 12.0) / 2.0;
        let by = carte.y + ch - MARGE - BOUTON_H;
        Places {
            carte,
            fermer: Rect::new(carte.x + cw - 14.0 - 32.0, carte.y + 14.0, 32.0, 32.0),
            annuler: Rect::new(carte.x + MARGE, by, bw, BOUTON_H),
            ok: Rect::new(carte.x + MARGE + bw + 12.0, by, bw, BOUTON_H),
        }
    }

    /// A click (or a touch) at (`x`, `y`) of the window: the button, ✕ (= Annuler), outside the box (= Annuler).
    pub fn clic(&self, text: &Text, w: f64, h: f64, x: f64, y: f64) -> Option<ConfirmerClic> {
        let p = self.places(text, w, h);
        if p.ok.contains(x, y) {
            Some(ConfirmerClic::Ok)
        } else if p.annuler.contains(x, y) || p.fermer.contains(x, y) || !p.carte.contains(x, y) {
            Some(ConfirmerClic::Annuler)
        } else {
            None
        }
    }

    /// Draws it over the window image `pm` (`w` × `h` points): the window darkened, the box, its buttons.
    pub fn draw(&self, pm: &mut Pixmap, text: &Text, w: f64, h: f64, mouse: Option<(f64, f64)>) {
        let p = self.places(text, w, h);
        fill(pm, 0.0, 0.0, w, h, j::FOND, 0.55);
        j::fond_fenetre(pm, p.carte, j::R_FENETRE);
        let sur = |r: &Rect| mouse.is_some_and(|(x, y)| r.contains(x, y));
        let gras = Text::graisse(crate::text::Graisse::Bold);
        let x0 = p.carte.x + MARGE;
        gras.draw(
            pm,
            &text.fit(&self.titre, 18.0, p.fermer.x - x0 - 8.0),
            (x0 as f32, (p.carte.y + MARGE + 18.0) as f32),
            18.0,
            j::TEXTE,
            Align::Left,
        );
        for (k, l) in self.lignes(text, p.carte.w - 2.0 * MARGE).iter().enumerate() {
            let y = p.carte.y + MARGE + 26.0 + 12.0 + (k as f64 + 0.75) * LIGNE;
            text.draw(pm, l, (x0 as f32, y as f32), 14.0, j::DIM, Align::Left);
        }
        // ✕
        round_fill(pm, p.fermer, p.fermer.h / 2.0, j::VERRE2.0, if sur(&p.fermer) { 0.16 } else { j::VERRE2.1 });
        text.draw(pm, "✕", (p.fermer.cx() as f32, (p.fermer.cy() + 5.0) as f32), 14.0, j::TEXTE, Align::Center);
        // Annuler
        round_fill(pm, p.annuler, p.annuler.h / 2.0, j::VERRE2.0, if sur(&p.annuler) { 0.16 } else { j::VERRE2.1 });
        round_stroke(pm, p.annuler, p.annuler.h / 2.0, j::sur_fond(j::LISERE), 1.0);
        gras.draw(
            pm,
            &crate::i18n::tr("Annuler"),
            (p.annuler.cx() as f32, (p.annuler.cy() + 5.0) as f32),
            14.5,
            j::TEXTE,
            Align::Center,
        );
        // the button that does it
        let c = if self.danger { j::BAS } else { j::ACCENT };
        round_fill(pm, p.ok, p.ok.h / 2.0, c, if sur(&p.ok) { 0.85 } else { 1.0 });
        gras.draw(
            pm,
            &self.ok,
            (p.ok.cx() as f32, (p.ok.cy() + 5.0) as f32),
            14.5,
            crate::text::Rgb(0xff, 0xff, 0xff),
            Align::Center,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn boite() -> Confirmer {
        Confirmer {
            titre: "Supprimer cette alerte?".into(),
            texte: "Cette action supprimera définitivement votre alerte « BTC monte au-dessus de 70 000 ».".into(),
            ok: "Effacer".into(),
            danger: true,
        }
    }

    /// The box: centred, its buttons inside, ✕ and outside = Annuler, the red button = Ok; drawn (red under « Effacer »).
    #[test]
    fn boite_de_confirmation() {
        let t = Text::default();
        let b = boite();
        for (w, h) in [(1440.0, 900.0), (402.0, 874.0)] {
            let p = b.places(&t, w, h);
            assert!(p.carte.x >= 16.0 - 0.5 && p.carte.x + p.carte.w <= w - 16.0 + 0.5, "inside the window at {w}");
            for r in [p.ok, p.annuler, p.fermer] {
                assert!(
                    r.x >= p.carte.x && r.x + r.w <= p.carte.x + p.carte.w && r.y + r.h <= p.carte.y + p.carte.h,
                    "{r:?}"
                );
            }
            assert!(p.ok.h >= 40.0 && p.annuler.h >= 40.0, "touch zones of 40");
            assert_eq!(b.clic(&t, w, h, p.ok.cx(), p.ok.cy()), Some(ConfirmerClic::Ok));
            assert_eq!(b.clic(&t, w, h, p.annuler.cx(), p.annuler.cy()), Some(ConfirmerClic::Annuler));
            assert_eq!(b.clic(&t, w, h, p.fermer.cx(), p.fermer.cy()), Some(ConfirmerClic::Annuler));
            assert_eq!(b.clic(&t, w, h, 2.0, 2.0), Some(ConfirmerClic::Annuler), "outside: cancelled");
            assert_eq!(b.clic(&t, w, h, p.carte.x + 30.0, p.carte.y + 60.0), None, "in the box: nothing");
            let mut pm = crate::px::new(w as u32, h as u32).unwrap();
            b.draw(&mut pm, &t, w, h, None);
            let q = pm.pixel((p.ok.x + 8.0) as u32, p.ok.cy() as u32).unwrap();
            assert!(q.red() > 180 && q.green() < 100, "« Effacer » in red: {q:?}");
        }
    }
}
