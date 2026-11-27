//! 🔐 Boîte des **24 mots** d'un nouveau compte Argos : les mots dans l'ordre (3 colonnes), l'avertissement, 📋 Copier,
//! « Je les ai notés en lieu sûr » et Continuer (seulement une fois coché), comme le site. Les mots ne vivent qu'en
//! mémoire le temps de la création : la fenêtre les efface (`wipe`) dès que la boîte se ferme.

use crate::shapes::{round_fill, round_stroke, Rect};
use crate::text::{Align, Text};
use tiny_skia::Pixmap;

use crate::palette as pal;
use crate::palette::{accent, DIM, GOLD, GREY, WHITE};

/// Ce qu'un clic demande à la fenêtre.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WordsClick {
    None,
    Cancel,
    Copy,
    Continue,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WordsBox {
    pub words: Vec<String>,
    /// « Je les ai notés en lieu sûr » coché.
    pub noted: bool,
    pub rect: Rect,
}

const W: f64 = 520.0;
const H: f64 = 470.0;

impl WordsBox {
    pub fn new(words: Vec<String>, win_w: f64, win_h: f64) -> Self {
        let w = W.min(win_w - 12.0);
        Self {
            words,
            noted: false,
            rect: Rect::new((win_w - w) / 2.0, ((win_h - H) / 2.0).max(6.0), w, H),
        }
    }

    fn at(&self, x: f64, y: f64, w: f64, h: f64) -> Rect {
        Rect::new(self.rect.x + x, self.rect.y + y, w, h)
    }

    fn copy_rect(&self) -> Rect {
        self.at(18.0, H - 104.0, 120.0, 34.0)
    }

    fn check_rect(&self) -> Rect {
        self.at(152.0, H - 104.0, self.rect.w - 170.0, 34.0)
    }

    fn cancel_rect(&self) -> Rect {
        self.at(self.rect.w - 230.0, H - 52.0, 100.0, 36.0)
    }

    fn go_rect(&self) -> Rect {
        self.at(self.rect.w - 120.0, H - 52.0, 102.0, 36.0)
    }

    pub fn click(&mut self, x: f64, y: f64) -> WordsClick {
        if self.go_rect().contains(x, y) {
            return if self.noted { WordsClick::Continue } else { WordsClick::None };
        }
        if self.copy_rect().contains(x, y) {
            return WordsClick::Copy;
        }
        if self.check_rect().contains(x, y) {
            self.noted = !self.noted;
            return WordsClick::None;
        }
        if self.cancel_rect().contains(x, y) || !self.rect.contains(x, y) {
            return WordsClick::Cancel;
        }
        WordsClick::None
    }

    /// Efface les mots de la mémoire (fermeture de la boîte).
    pub fn wipe(&mut self) {
        for w in &mut self.words {
            let n = w.len();
            w.clear();
            w.extend(std::iter::repeat_n('0', n));
            w.clear();
        }
        self.words.clear();
    }

    pub fn draw(&self, text: &Text, mouse: (f64, f64)) -> Option<(Pixmap, Rect)> {
        let r = self.rect;
        let mut pm = crate::px::new(r.w.ceil() as u32, r.h.ceil() as u32)?;
        let loc = |q: Rect| Rect::new(q.x - r.x, q.y - r.y, q.w, q.h);
        let hov = |q: &Rect| q.contains(mouse.0, mouse.1);
        crate::jetons::fond_fenetre(&mut pm, loc(r), 12.0);
        round_stroke(&mut pm, loc(r), 12.0, pal::edge_soft(), 1.0);
        text.draw(&mut pm, "🔐 Ta clé de 24 mots", (18.0, 32.0), 16.0, WHITE, Align::Left);
        let warn = [
            "Note ces 24 mots sur papier, dans l'ordre : seule façon de récupérer ton compte.",
            "Argos ne les garde pas en clair et ne peut pas te les renvoyer.",
            "⚠️ Pas la phrase de ton wallet crypto : ne la mets jamais dans Argos.",
        ];
        for (k, l) in warn.iter().enumerate() {
            text.draw(
                &mut pm,
                &text.fit(l, 12.0, r.w - 36.0),
                (18.0, (58.0 + k as f64 * 17.0) as f32),
                12.0,
                GOLD,
                Align::Left,
            );
        }
        // les mots : 3 colonnes de 8, numérotés
        let (top, cw, rh) = (118.0, (r.w - 36.0) / 3.0, 28.0);
        for (i, w) in self.words.iter().enumerate() {
            let (col, row) = (i / 8, i % 8);
            let q = Rect::new(18.0 + col as f64 * cw, top + row as f64 * rh, cw - 8.0, rh - 4.0);
            round_fill(&mut pm, q, 5.0, pal::field(), 1.0);
            text.draw(
                &mut pm,
                &format!("{}.", i + 1),
                ((q.x + 24.0) as f32, (q.cy() + 4.5) as f32),
                12.0,
                DIM,
                Align::Right,
            );
            text.draw(&mut pm, w, ((q.x + 30.0) as f32, (q.cy() + 5.0) as f32), 14.0, WHITE, Align::Left);
        }
        let (cp, ck, ca, go) = (self.copy_rect(), self.check_rect(), self.cancel_rect(), self.go_rect());
        round_fill(&mut pm, loc(cp), 6.0, if hov(&cp) { pal::hover() } else { pal::field() }, 1.0);
        round_stroke(&mut pm, loc(cp), 6.0, pal::edge_soft(), 1.0);
        text.draw(
            &mut pm,
            "📋 Copier",
            (loc(cp).cx() as f32, (loc(cp).cy() + 5.0) as f32),
            13.5,
            WHITE,
            Align::Center,
        );
        // case à cocher
        let b = Rect::new(loc(ck).x + 4.0, loc(ck).cy() - 9.0, 18.0, 18.0);
        round_fill(&mut pm, b, 4.0, if self.noted { accent() } else { pal::field() }, 1.0);
        round_stroke(&mut pm, b, 4.0, if self.noted { accent() } else { GREY }, 1.2);
        if self.noted {
            crate::shapes::lines(&mut pm, b.x, b.y, &[&[(4.0, 9.5), (7.5, 13.0), (14.0, 5.5)]], WHITE, 2.0);
        }
        text.draw(
            &mut pm,
            "Je les ai notés en lieu sûr",
            ((b.x + 28.0) as f32, (b.cy() + 5.0) as f32),
            13.5,
            WHITE,
            Align::Left,
        );
        round_fill(&mut pm, loc(ca), 6.0, if hov(&ca) { pal::hover() } else { pal::field() }, 1.0);
        round_stroke(&mut pm, loc(ca), 6.0, pal::edge_soft(), 1.0);
        text.draw(
            &mut pm,
            "Annuler",
            (loc(ca).cx() as f32, (loc(ca).cy() + 5.0) as f32),
            14.0,
            WHITE,
            Align::Center,
        );
        round_fill(&mut pm, loc(go), 6.0, if self.noted { accent() } else { pal::hover() }, 1.0);
        text.draw(
            &mut pm,
            "Continuer",
            (loc(go).cx() as f32, (loc(go).cy() + 5.0) as f32),
            14.0,
            if self.noted { crate::palette::ink_on(accent()) } else { GREY },
            Align::Center,
        );
        Some((pm, r))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn continuer_seulement_une_fois_note() {
        let words: Vec<String> = (0..24).map(|i| format!("mot{i}")).collect();
        let mut b = WordsBox::new(words, 1200.0, 800.0);
        let go = b.go_rect();
        assert_eq!(b.click(go.cx(), go.cy()), WordsClick::None, "pas encore coché");
        let ck = b.check_rect();
        b.click(ck.x + 10.0, ck.cy());
        assert!(b.noted);
        assert_eq!(b.click(go.cx(), go.cy()), WordsClick::Continue);
        let cp = b.copy_rect();
        assert_eq!(b.click(cp.cx(), cp.cy()), WordsClick::Copy);
        assert!(b.draw(&Text::default(), (0.0, 0.0)).is_some());
        b.wipe();
        assert!(b.words.is_empty());
        assert_eq!(b.click(0.0, 0.0), WordsClick::Cancel, "à côté = annuler");
    }
}
