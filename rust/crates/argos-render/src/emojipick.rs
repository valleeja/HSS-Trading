//! 😀 Sélecteur de l'outil « Icônes et émojis » : tout le jeu Twemoji 15.0.0 (demande de Jee, 9 nov. 2026), rangé
//! par catégories comme le clavier d'un téléphone, avec une recherche (noms anglais d'Unicode) et une grille qui
//! défile. Style verre de la maquette (carte arrondie, puces, champ). Les images arrivent à la demande
//! (`emoji::a_charger`) : seules celles qu'on voit sont chargées.

use crate::shapes::{round_fill, round_stroke, Rect};
use crate::text::{Align, Graisse, Text};
use tiny_skia::Pixmap;

/// Puce de chaque catégorie (dans l'ordre de `emoji_tout::GROUPES`).
const PUCES: [&str; 10] = ["📈", "😀", "👋", "🐶", "🍔", "✈️", "⚽", "💡", "🔣", "🏁"];
const CASE: f64 = 40.0;
const HAUT_GRILLE: f64 = 146.0;
const PAD: f64 = 12.0;

/// Ce que fait un toucher.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PickClick {
    None,
    Close,
    /// L'état a changé (catégorie, recherche) : redessiner.
    Changed,
    Emoji(&'static str),
}

/// État du sélecteur.
#[derive(Debug, Clone, PartialEq)]
pub struct EmojiPick {
    pub rect: Rect,
    pub groupe: usize,
    /// `Some` : la recherche reçoit le clavier.
    pub recherche: Option<String>,
    pub defile: f64,
}

impl EmojiPick {
    /// Sous le bouton `anchor` (ou au-dessus s'il n'y a pas la place), dans la chart `w` × `h`.
    pub fn new(anchor: Rect, w: f64, h: f64) -> Self {
        let pw = (w - 16.0).clamp(200.0, 360.0);
        let ph = (h - 16.0).clamp(220.0, 440.0);
        let mut y = anchor.y + anchor.h + 6.0;
        if y + ph > h - 4.0 {
            y = (anchor.y - ph - 6.0).max(4.0);
        }
        let x = (anchor.cx() - pw / 2.0).min(w - pw - 4.0).max(4.0);
        Self {
            rect: Rect::new(x, y.max(4.0), pw, ph),
            groupe: 0,
            recherche: None,
            defile: 0.0,
        }
    }

    /// Les émojis montrés : la recherche (dans tout le jeu), sinon la catégorie choisie.
    pub fn items(&self) -> Vec<&'static str> {
        let q = self.recherche.as_deref().map(|q| q.trim().to_lowercase()).unwrap_or_default();
        if !q.is_empty() {
            let g = |k: u8| crate::emoji_tout::GROUPES.get(k as usize).map_or(String::new(), |s| s.to_lowercase());
            return crate::emoji_tout::TOUT
                .iter()
                .filter(|(gr, _, _, nom)| nom.contains(&q) || g(*gr).contains(&q))
                .map(|(_, _, e, _)| *e)
                .collect();
        }
        if self.groupe == 0 {
            return crate::emoji::SECTIONS.iter().flat_map(|(_, l)| l.iter().copied()).collect();
        }
        crate::emoji_tout::TOUT
            .iter()
            .filter(|(g, ..)| *g as usize == self.groupe)
            .map(|(_, _, e, _)| *e)
            .collect()
    }

    fn cols(&self) -> usize {
        (((self.rect.w - 2.0 * PAD) / CASE).floor() as usize).max(1)
    }

    /// Hauteur de la grille visible et défilement maximum.
    fn vue(&self) -> f64 {
        self.rect.h - HAUT_GRILLE - 8.0
    }

    fn max_defile(&self) -> f64 {
        let lignes = self.items().len().div_ceil(self.cols()) as f64;
        (lignes * CASE - self.vue()).max(0.0)
    }

    /// Molette / doigt (`dy` > 0 : vers le bas de la liste).
    pub fn wheel(&mut self, dy: f64) {
        self.defile = (self.defile + dy).clamp(0.0, self.max_defile());
    }

    /// Clavier de la recherche : un caractère, ou `None` = effacer.
    pub fn key(&mut self, c: Option<char>) {
        if let Some(q) = self.recherche.as_mut() {
            match c {
                Some(c) if !c.is_control() => q.push(c),
                Some(_) => {}
                None => _ = q.pop(),
            }
            self.defile = 0.0;
        }
    }

    fn fermer(&self) -> Rect {
        Rect::new(self.rect.x + self.rect.w - 42.0, self.rect.y + 9.0, 30.0, 30.0)
    }

    pub fn champ(&self) -> Rect {
        Rect::new(self.rect.x + PAD, self.rect.y + 48.0, self.rect.w - 2.0 * PAD, 36.0)
    }

    fn puce(&self, k: usize) -> Rect {
        let l = (self.rect.w - 2.0 * PAD) / PUCES.len() as f64;
        Rect::new(self.rect.x + PAD + l * k as f64 + 1.0, self.rect.y + 92.0, l - 2.0, 34.0)
    }

    fn case(&self, i: usize) -> Rect {
        let c = self.cols();
        let gx = self.rect.x + (self.rect.w - c as f64 * CASE) / 2.0;
        Rect::new(
            gx + (i % c) as f64 * CASE,
            self.rect.y + HAUT_GRILLE + (i / c) as f64 * CASE - self.defile,
            CASE,
            CASE,
        )
    }

    /// Toucher (coordonnées de la chart).
    pub fn click(&mut self, x: f64, y: f64) -> PickClick {
        if !self.rect.contains(x, y) {
            return PickClick::Close;
        }
        if self.fermer().contains(x, y) {
            return PickClick::Close;
        }
        if self.champ().contains(x, y) {
            self.recherche.get_or_insert_with(String::new);
            return PickClick::Changed;
        }
        for k in 0..PUCES.len() {
            if self.puce(k).contains(x, y) {
                (self.groupe, self.recherche, self.defile) = (k, None, 0.0);
                return PickClick::Changed;
            }
        }
        if y >= self.rect.y + HAUT_GRILLE {
            for (i, e) in self.items().into_iter().enumerate() {
                if self.case(i).contains(x, y) {
                    return PickClick::Emoji(e);
                }
            }
        }
        PickClick::None
    }

    /// Dessin (image de la taille de `rect`) ; `choisi` : l'émoji de l'outil ; `mouse` en coordonnées de la chart.
    pub fn draw(&self, text: &Text, choisi: &str, mouse: Option<(f64, f64)>) -> Option<Pixmap> {
        let r = self.rect;
        let mut pm = crate::px::new(r.w.ceil() as u32, r.h.ceil() as u32)?;
        let loc = |q: Rect| Rect::new(q.x - r.x, q.y - r.y, q.w, q.h);
        let survol = |q: &Rect| mouse.is_some_and(|(x, y)| q.contains(x, y));
        let carte = loc(r);
        round_fill(&mut pm, carte, 20.0, crate::jetons::sur_fond(crate::jetons::VERRE), 1.0);
        round_stroke(&mut pm, carte, 20.0, crate::jetons::sur_fond(crate::jetons::LISERE), 1.0);
        let gras = Text::graisse(Graisse::Bold);
        gras.draw(&mut pm, "Icônes et émojis", (14.0, 30.0), 15.0, crate::jetons::TEXTE, Align::Left);
        let f = loc(self.fermer());
        round_fill(&mut pm, f, 15.0, crate::jetons::BLANC, if survol(&self.fermer()) { 0.12 } else { 0.07 });
        text.draw(&mut pm, "✕", (f.cx() as f32, (f.cy() + 4.7) as f32), 13.0, crate::jetons::TEXTE, Align::Center);
        // 🔎 recherche
        let c = loc(self.champ());
        let (cc, ca) = crate::jetons::CHAMP;
        round_fill(&mut pm, c, 18.0, cc, ca + if self.recherche.is_some() { 0.04 } else { 0.0 });
        if self.recherche.is_some() {
            round_stroke(&mut pm, c, 18.0, crate::jetons::ACCENT, 1.0);
        }
        let (t, col) = match self.recherche.as_deref() {
            Some(q) if !q.is_empty() => (format!("🔎 {q}▏"), crate::jetons::TEXTE),
            Some(_) => ("🔎 ▏".to_string(), crate::jetons::TEXTE),
            None => ("🔎 Chercher (nom anglais : heart, rocket, fire…)".to_string(), crate::jetons::DIM),
        };
        text.draw(
            &mut pm,
            &text.fit(&t, 13.0, c.w - 24.0),
            ((c.x + 12.0) as f32, (c.cy() + 4.5) as f32),
            13.0,
            col,
            Align::Left,
        );
        // catégories
        for (k, e) in PUCES.iter().enumerate() {
            let q = self.puce(k);
            let lq = loc(q);
            let on = self.recherche.as_deref().is_none_or(str::is_empty) && k == self.groupe;
            let (pc, pa) = if on { crate::jetons::PUCE_ON } else { crate::jetons::VERRE2 };
            round_fill(&mut pm, lq, 12.0, pc, pa + if survol(&q) && !on { 0.05 } else { 0.0 });
            if let Some(img) = crate::emoji::image(e, crate::px::dev(20.0).round() as u32) {
                crate::px::put(&mut pm, lq.cx() - 10.0, lq.cy() - 10.0, &img);
            }
        }
        let items = self.items();
        let titre = match self.recherche.as_deref() {
            Some(q) if !q.trim().is_empty() => format!("RECHERCHE · {}", items.len()),
            _ => format!(
                "{} · {}",
                crate::emoji_tout::GROUPES.get(self.groupe).copied().unwrap_or("").to_uppercase(),
                items.len()
            ),
        };
        text.draw(
            &mut pm,
            &titre,
            (PAD as f32 + 2.0, (HAUT_GRILLE - 4.0) as f32),
            11.0,
            crate::jetons::DIM,
            Align::Left,
        );
        // la grille : seules les lignes visibles sont dessinées (et donc chargées)
        let haut = r.y + HAUT_GRILLE;
        let bas = r.y + r.h - 8.0;
        let taille = crate::px::dev(28.0).round() as u32;
        for (i, e) in items.iter().enumerate() {
            let q = self.case(i);
            if q.y + q.h < haut || q.y > bas {
                continue;
            }
            let lq = loc(q);
            if *e == choisi || survol(&q) {
                let (pc, pa) = if *e == choisi { crate::jetons::PUCE_ON } else { crate::jetons::VERRE2 };
                round_fill(&mut pm, Rect::new(lq.x + 2.0, lq.y + 2.0, lq.w - 4.0, lq.h - 4.0), 10.0, pc, pa);
            }
            if let Some(img) = crate::emoji::image(e, taille) {
                // (coupée au bord de la grille quand elle défile)
                let y0 = lq.cy() - 14.0;
                if y0 >= HAUT_GRILLE - 2.0 && y0 + 28.0 <= r.h - 4.0 {
                    crate::px::put(&mut pm, lq.cx() - 14.0, y0, &img);
                }
            }
        }
        if items.is_empty() {
            text.draw(
                &mut pm,
                "Aucun émoji trouvé",
                ((r.w / 2.0) as f32, (HAUT_GRILLE + 30.0) as f32),
                13.0,
                crate::jetons::DIM,
                Align::Center,
            );
        }
        // petite barre de défilement
        let max = self.max_defile();
        if max > 0.0 {
            let vue = self.vue();
            let total = vue + max;
            let th = (vue * vue / total).max(24.0);
            let ty = HAUT_GRILLE + self.defile / max * (vue - th);
            crate::jetons::barre_defilement(&mut pm, r.w - 4.0, ty, th, false);
        }
        Some(pm)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 😀 Tout le jeu est offert : chaque catégorie a ses émojis, la recherche trouve dans tout le jeu, un toucher sur
    /// une case choisit son émoji, la grille défile.
    #[test]
    fn tout_twemoji_dans_le_selecteur() {
        assert!(crate::emoji_tout::TOUT.len() > 1800, "tout le jeu Twemoji");
        let mut p = EmojiPick::new(Rect::new(20.0, 20.0, 30.0, 30.0), 390.0, 700.0);
        for g in 0..PUCES.len() {
            p.groupe = g;
            assert!(!p.items().is_empty(), "catégorie {g} vide");
        }
        p.groupe = 1;
        let premier = p.items()[0];
        let c = p.case(0);
        assert_eq!(p.click(c.cx(), c.cy()), PickClick::Emoji(premier));
        let ch = p.champ();
        assert_eq!(p.click(ch.cx(), ch.cy()), PickClick::Changed);
        for c in "pizza".chars() {
            p.key(Some(c));
        }
        assert!(p.items().contains(&"🍕"), "la recherche trouve 🍕");
        p.recherche = None;
        p.groupe = 9;
        p.wheel(10_000.0);
        assert!(p.defile > 0.0 && p.defile <= p.max_defile(), "la grille défile jusqu'au bout");
        assert!(
            crate::emoji::connu("🍕") && crate::emoji::connu("👍🏽"),
            "le jeu entier est connu (peaux comprises)"
        );
    }

    /// 📸 Le sélecteur (Visages, puis une recherche « heart »), images lues dans `emoji/` du dépôt : seulement si
    /// `ARGOS_CAPTURES`.
    #[test]
    fn capture_selecteur() {
        let Ok(dir) = std::env::var("ARGOS_CAPTURES") else { return };
        let t = Text::default();
        let racine = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../emoji");
        for (nom, groupe, q) in [("emoji-visages", 1, None), ("emoji-recherche", 1, Some("heart"))] {
            let mut p = EmojiPick::new(Rect::new(20.0, 60.0, 40.0, 40.0), 390.0, 700.0);
            p.groupe = groupe;
            p.recherche = q.map(String::from);
            // 1er dessin : il demande les images visibles ; on les lit dans le dossier, puis on redessine
            let _ = p.draw(&t, "🚀", None);
            for k in crate::emoji::a_charger() {
                if let Ok(svg) = std::fs::read(racine.join(format!("{k}.svg"))) {
                    crate::emoji::charger(&k, svg);
                }
            }
            let img = p.draw(&t, "🚀", None).unwrap();
            let mut pm = crate::px::new(p.rect.w as u32 + 40, p.rect.h as u32 + 40).unwrap();
            crate::fill(&mut pm, 0.0, 0.0, p.rect.w + 40.0, p.rect.h + 40.0, crate::text::Rgb(0x0b, 0x0e, 0x16), 1.0);
            crate::px::put(&mut pm, 20.0, 20.0, &img);
            pm.save_png(format!("{dir}/{nom}.png")).unwrap();
        }
    }
}
