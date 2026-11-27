//! 📱 The bottom sheets of the phone's single pill (« Argos » ⋯, « Dessins » ✎, « Trading · DÉMO », the small
//! « X | Y » sheets): glass, top corners of 30, a handle, a title, ✕ on the right (📤 on the left for « Argos »), an
//! optional account card, search field and tabs, then tiles in sections (grey icons), and a footer. Dragging it down
//! or touching outside closes it. One table of touch zones ([`Feuille::zones`]) serves the drawing and the touches,
//! so what is drawn is what is touched; every zone is at least 40 points.

use crate::shapes::{round_fill, round_stroke, Rect};
use crate::text::Rgb;
use crate::text::{Align, Graisse, Text};
use tiny_skia::Pixmap;

/// A tile: what it opens (`id`), its grey Phosphor icon, its name (two lines at most), a red badge (0: none), a
/// favourite star (`Some(on)`: shown), and lit (already open).
#[derive(Debug, Clone, PartialEq)]
pub struct Tuile {
    pub id: String,
    pub icone: &'static str,
    pub nom: String,
    pub pastille: usize,
    pub etoile: Option<bool>,
    pub on: bool,
    /// Not available yet (« bientôt »): grey, it does nothing.
    pub grise: bool,
}

impl Tuile {
    pub fn new(id: &str, icone: &'static str, nom: &str) -> Self {
        Self {
            id: id.into(),
            icone,
            nom: nom.into(),
            pastille: 0,
            etoile: None,
            on: false,
            grise: false,
        }
    }
}

/// A section of tiles: its title (empty: none) and its number of columns (2: icon beside the name; 3: icon above).
#[derive(Debug, Clone, PartialEq)]
pub struct Section {
    pub titre: String,
    pub cols: usize,
    pub tuiles: Vec<Tuile>,
}

/// The account card (gold border): a title line and a smaller line.
#[derive(Debug, Clone, PartialEq)]
pub struct Carte {
    pub titre: String,
    pub texte: String,
}

/// A sheet. `champ`: a search field (`Some(text)`), `saisie`: the keyboard is on it; `onglets` and the chosen one;
/// `defil`: scrolled points; `tire`: points dragged down by the finger (closes past a third of its height).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Feuille {
    pub titre: String,
    pub partager: bool,
    pub carte: Option<Carte>,
    pub champ: Option<String>,
    pub saisie: bool,
    pub onglets: Vec<String>,
    pub onglet: usize,
    pub sections: Vec<Section>,
    pub pied: Option<String>,
    pub vide: Option<String>,
    pub defil: f64,
    pub tire: f64,
}

/// What a touch on the sheet does.
#[derive(Debug, Clone, PartialEq)]
pub enum Toucher {
    /// ✕, outside the sheet, or dragged down far enough.
    Fermer,
    Partager,
    Carte,
    Champ,
    Onglet(usize),
    Tuile(String),
    Etoile(String),
    Pied,
    /// On the sheet, on nothing.
    Rien,
}

/// 📱 The smallest touch zone, in layout points: 40 points on the screen at the phone's compact factor (0.8).
pub const TOUCHER: f64 = 40.0 / crate::formfactor::COMPACTE_CELL;
const MARGE: f64 = 12.0;
const ECART: f64 = 8.0;
const TETE: f64 = 56.0;
const RAYON: f64 = 30.0;
const CARTE_H: f64 = 72.0;
const CHAMP_H: f64 = TOUCHER;
const ONGLET_H: f64 = TOUCHER;
const TITRE_H: f64 = 30.0;
const PIED_H: f64 = TOUCHER;
/// Room left at the bottom (the phone's home bar).
const BAS: f64 = 24.0;
/// The sheet never covers more than this share of the screen.
const PART: f64 = 0.88;
/// Grey of the icons (Jee: « tout gris et clean »).
pub const GRIS: Rgb = Rgb(0xb4, 0xb9, 0xc4);

fn tuile_h(cols: usize) -> f64 {
    if cols >= 3 {
        76.0
    } else {
        TOUCHER
    }
}

/// Tab widths and places (a row wraps to the next one when full); relative to the content's left and top.
fn places_onglets(text: &Text, onglets: &[String], w: f64) -> Vec<Rect> {
    let (mut x, mut y, mut out) = (0.0, 0.0, vec![]);
    for o in onglets {
        let ow = (text.width(o, 13.0) as f64 + 26.0).max(TOUCHER);
        if x > 0.0 && x + ow > w {
            (x, y) = (0.0, y + ONGLET_H + 6.0);
        }
        out.push(Rect::new(x, y, ow, ONGLET_H));
        x += ow + 6.0;
    }
    out
}

impl Feuille {
    fn interieur(w: f64) -> f64 {
        w - 2.0 * MARGE
    }

    /// The parts of the content, top to bottom, relative to the top of the content: (rect, zone).
    fn parties(&self, text: &Text, w: f64) -> Vec<(Rect, Toucher)> {
        let iw = Self::interieur(w);
        let mut y = 0.0;
        let mut out = vec![];
        if self.carte.is_some() {
            out.push((Rect::new(MARGE, y, iw, CARTE_H), Toucher::Carte));
            y += CARTE_H + ECART;
        }
        if self.champ.is_some() {
            out.push((Rect::new(MARGE, y, iw, CHAMP_H), Toucher::Champ));
            y += CHAMP_H + ECART;
        }
        if !self.onglets.is_empty() {
            let p = places_onglets(text, &self.onglets, iw);
            let bas = p.iter().map(|r| r.y + r.h).fold(0.0, f64::max);
            out.extend(
                p.into_iter()
                    .enumerate()
                    .map(|(k, r)| (Rect::new(r.x + MARGE, r.y + y, r.w, r.h), Toucher::Onglet(k))),
            );
            y += bas + ECART;
        }
        for s in &self.sections {
            if !s.titre.is_empty() {
                y += TITRE_H;
            }
            let cols = s.cols.max(1);
            let tw = (iw - ECART * (cols - 1) as f64) / cols as f64;
            let th = tuile_h(cols);
            for (k, t) in s.tuiles.iter().enumerate() {
                let r = Rect::new(MARGE + (k % cols) as f64 * (tw + ECART), y + (k / cols) as f64 * (th + ECART), tw, th);
                if t.etoile.is_some() {
                    out.push((Rect::new(r.x + r.w - TOUCHER, r.y, TOUCHER, TOUCHER), Toucher::Etoile(t.id.clone())));
                }
                out.push((r, Toucher::Tuile(t.id.clone())));
            }
            let rangs = s.tuiles.len().div_ceil(cols) as f64;
            y += rangs * th + (rangs - 1.0).max(0.0) * ECART + ECART;
        }
        if self.vide.is_some() && self.sections.iter().all(|s| s.tuiles.is_empty()) {
            y += 60.0;
        }
        if self.pied.is_some() {
            out.push((Rect::new(MARGE, y, iw, PIED_H), Toucher::Pied));
            y += PIED_H;
        }
        // (the bottom of the content: a zone of nothing, for its height)
        out.push((Rect::new(0.0, y, w, BAS), Toucher::Rien));
        out
    }

    /// Height of everything in the sheet (head and content).
    pub fn hauteur(&self, text: &Text, w: f64) -> f64 {
        TETE + self.parties(text, w).last().map_or(0.0, |(r, _)| r.y + r.h)
    }

    /// The sheet in the window `w` × `h`: stuck to the bottom, moved down by the finger's drag.
    pub fn rect(&self, text: &Text, w: f64, h: f64) -> Rect {
        let ht = self.hauteur(text, w).min(h * PART);
        Rect::new(0.0, h - ht + self.tire.max(0.0), w, ht)
    }

    /// How far the content scrolls at most.
    pub fn defil_max(&self, text: &Text, w: f64, h: f64) -> f64 {
        (self.hauteur(text, w) - self.rect(text, w, h).h).max(0.0)
    }

    /// Scrolls by `dy` points (positive: further down the content).
    pub fn defiler(&mut self, text: &Text, w: f64, h: f64, dy: f64) {
        self.defil = (self.defil + dy).clamp(0.0, self.defil_max(text, w, h));
    }

    /// The touch zones in the window: the head (📤, ✕), then the content scrolled, kept to what shows.
    pub fn zones(&self, text: &Text, w: f64, h: f64) -> Vec<(Rect, Toucher)> {
        let r = self.rect(text, w, h);
        let mut out = vec![(Rect::new(r.x + r.w - TOUCHER - 6.0, r.y + 4.0, TOUCHER, TOUCHER), Toucher::Fermer)];
        if self.partager {
            out.push((Rect::new(r.x + 6.0, r.y + 4.0, TOUCHER, TOUCHER), Toucher::Partager));
        }
        let top = r.y + TETE - self.defil;
        let (vis0, vis1) = (r.y + TETE, r.y + r.h);
        for (z, t) in self.parties(text, w) {
            let z = Rect::new(z.x + r.x, z.y + top, z.w, z.h);
            let (a, b) = (z.y.max(vis0), (z.y + z.h).min(vis1));
            if b - a >= 1.0 && t != Toucher::Rien {
                out.push((Rect::new(z.x, a, z.w, b - a), t));
            }
        }
        out
    }

    /// What is under the finger: outside the sheet closes it.
    pub fn toucher(&self, text: &Text, w: f64, h: f64, x: f64, y: f64) -> Toucher {
        if !self.rect(text, w, h).contains(x, y) {
            return Toucher::Fermer;
        }
        self.zones(text, w, h)
            .into_iter()
            .find(|(r, _)| r.contains(x, y))
            .map_or(Toucher::Rien, |(_, t)| t)
    }

    /// The finger dragged down by `dy` points from the head (`fin`: lifted): past a third of the sheet it closes
    /// (`true`), else it comes back.
    pub fn tirer(&mut self, text: &Text, w: f64, h: f64, dy: f64, fin: bool) -> bool {
        if !fin {
            self.tire = dy.max(0.0);
            return false;
        }
        let ferme = self.tire > self.rect(text, w, h).h / 3.0;
        self.tire = 0.0;
        ferme
    }

    /// The image of the sheet (`rect` size), its touch zones drawn from the same table.
    pub fn draw(&self, text: &Text, w: f64, h: f64, survol: Option<(f64, f64)>) -> Option<Pixmap> {
        let r = self.rect(text, w, h);
        let mut pm = crate::px::new(r.w.ceil() as u32, r.h.ceil() as u32)?;
        let gras = Text::graisse(Graisse::Bold);
        // glass, top corners round (the bottom ones go below the image)
        let fond = Rect::new(0.0, 0.0, r.w, r.h + RAYON + 2.0);
        round_fill(&mut pm, fond, RAYON, crate::jetons::sur_fond(crate::jetons::VERRE), 1.0);
        round_stroke(
            &mut pm,
            Rect::new(0.5, 0.5, r.w - 1.0, r.h + RAYON),
            RAYON,
            crate::jetons::sur_fond(crate::jetons::LISERE),
            1.0,
        );
        // handle, title, ✕, 📤
        round_fill(
            &mut pm,
            Rect::new(r.w / 2.0 - 18.0, 8.0, 36.0, 4.0),
            2.0,
            crate::jetons::sur_fond(crate::jetons::LISERE),
            1.0,
        );
        gras.draw(&mut pm, &self.titre, ((r.w / 2.0) as f32, 38.0), 16.0, crate::jetons::TEXTE, Align::Center);
        let local = |z: Rect| Rect::new(z.x - r.x, z.y - r.y, z.w, z.h);
        let sur = |z: Rect| survol.is_some_and(|(x, y)| z.contains(x, y));
        let zones = self.zones(text, w, h);
        for (z, t) in &zones {
            let q = local(*z);
            match t {
                Toucher::Fermer => {
                    if sur(*z) {
                        round_fill(&mut pm, q, q.h / 2.0, crate::jetons::VERRE2.0, crate::jetons::VERRE2.1);
                    }
                    text.draw(&mut pm, "✕", (q.cx() as f32, (q.cy() + 6.0) as f32), 17.0, GRIS, Align::Center);
                }
                Toucher::Partager => {
                    icone(&mut pm, "export", q.cx(), q.cy(), 20.0);
                }
                _ => {}
            }
        }
        // the content, scrolled, cut under the head
        let Some(mut c) = crate::px::new(r.w.ceil() as u32, (r.h - TETE).max(1.0).ceil() as u32) else {
            return Some(pm);
        };
        let top = -self.defil;
        let place = |z: &Rect| Rect::new(z.x, z.y + top, z.w, z.h);
        let parties = self.parties(text, w);
        let mut sections = self.sections.iter();
        let mut section_tuiles: Vec<&Tuile> = vec![];
        for (z, t) in parties {
            let q = place(&z);
            let survole = survol.is_some_and(|(x, y)| q.contains(x - r.x, y - r.y - TETE));
            match &t {
                Toucher::Carte => {
                    let k = self.carte.as_ref();
                    round_fill(&mut c, q, 18.0, crate::jetons::CARTE.0, crate::jetons::CARTE.1);
                    round_stroke(&mut c, q, 18.0, crate::jetons::TIER2, 1.5);
                    round_fill(&mut c, Rect::new(q.x + 16.0, q.y + 22.0, 8.0, 8.0), 4.0, crate::jetons::TIER2, 1.0);
                    if let Some(k) = k {
                        gras.draw(
                            &mut c,
                            &k.titre,
                            ((q.x + 32.0) as f32, (q.y + 30.0) as f32),
                            14.0,
                            crate::jetons::TEXTE,
                            Align::Left,
                        );
                        let l = text.fit(&k.texte, 12.0, q.w - 32.0);
                        text.draw(&mut c, &l, ((q.x + 16.0) as f32, (q.y + 54.0) as f32), 12.0, crate::jetons::DIM, Align::Left);
                    }
                    text.draw(&mut c, "›", ((q.x + q.w - 16.0) as f32, (q.y + 40.0) as f32), 18.0, GRIS, Align::Center);
                }
                Toucher::Champ => {
                    round_fill(&mut c, q, 14.0, crate::jetons::CHAMP.0, crate::jetons::CHAMP.1);
                    if self.saisie {
                        round_stroke(&mut c, q, 14.0, crate::jetons::ACCENT, 1.5);
                    }
                    icone(&mut c, "magnifying-glass", q.x + 22.0, q.cy(), 16.0);
                    let s = self.champ.clone().unwrap_or_default();
                    let (t2, col) = if s.is_empty() {
                        ("Chercher".to_string(), crate::jetons::DIM)
                    } else {
                        (s, crate::jetons::TEXTE)
                    };
                    let t2 = if self.saisie { format!("{t2}│") } else { t2 };
                    text.draw(&mut c, &t2, ((q.x + 40.0) as f32, (q.cy() + 5.0) as f32), 14.0, col, Align::Left);
                }
                Toucher::Onglet(k) => {
                    let on = *k == self.onglet;
                    let (f, a) = if on { crate::jetons::PUCE_ON } else { crate::jetons::VERRE2 };
                    round_fill(&mut c, q, q.h / 2.0, f, a);
                    let col = if on { crate::jetons::TEXTE } else { GRIS };
                    let o = self.onglets.get(*k).cloned().unwrap_or_default();
                    text.draw(&mut c, &o, (q.cx() as f32, (q.cy() + 4.5) as f32), 13.0, col, Align::Center);
                }
                Toucher::Etoile(_) => {}
                Toucher::Tuile(id) => {
                    if section_tuiles.is_empty() {
                        // the next section that has tiles: its title above its first tile
                        if let Some(s) = sections.by_ref().find(|s| !s.tuiles.is_empty()) {
                            if !s.titre.is_empty() {
                                text.draw(
                                    &mut c,
                                    &s.titre,
                                    ((q.x + 4.0) as f32, (q.y - 10.0) as f32),
                                    11.0,
                                    crate::jetons::DIM,
                                    Align::Left,
                                );
                            }
                            section_tuiles = s.tuiles.iter().rev().collect();
                        }
                    }
                    let Some(tu) = section_tuiles.pop() else { continue };
                    debug_assert_eq!(&tu.id, id);
                    let cols3 = q.h > 60.0;
                    let (f, a) = if tu.on {
                        crate::jetons::PUCE_ON
                    } else if survole {
                        crate::jetons::CARTE_SURVOL
                    } else {
                        crate::jetons::VERRE2
                    };
                    round_fill(&mut c, q, crate::jetons::R_TUILE, f, a);
                    if cols3 {
                        icone(&mut c, tu.icone, q.cx(), q.y + 24.0, 22.0);
                        let l = super::couper(text, &tu.nom, 11.5, q.w - 8.0);
                        for (k, ligne) in l.iter().take(2).enumerate() {
                            text.draw(
                                &mut c,
                                ligne,
                                (q.cx() as f32, (q.y + 52.0 + k as f64 * 14.0) as f32),
                                11.5,
                                crate::jetons::TEXTE,
                                Align::Center,
                            );
                        }
                    } else {
                        let encre = if tu.grise { crate::jetons::DIM } else { crate::jetons::TEXTE };
                        icone(&mut c, tu.icone, q.x + 22.0, q.cy(), 20.0);
                        let coche = if tu.on && q.w > 200.0 { 28.0 } else { 0.0 };
                        let l = text.fit(&tu.nom, 13.0, q.w - 48.0 - if tu.pastille > 0 { 26.0 } else { 0.0 } - coche);
                        text.draw(&mut c, &l, ((q.x + 42.0) as f32, (q.cy() + 4.5) as f32), 13.0, encre, Align::Left);
                        // (a wide row that is on: ✓ at its end, like the chart's menu)
                        if coche > 0.0 {
                            crate::phosphor::check(&mut c, q.x + q.w - 22.0, q.cy(), 16.0);
                        }
                    }
                    if tu.pastille > 0 {
                        let n = if tu.pastille > 99 { "99+".to_string() } else { tu.pastille.to_string() };
                        let pw = (text.width(&n, 10.0) as f64 + 10.0).max(18.0);
                        let p = Rect::new(q.x + q.w - pw - 8.0, q.y + 8.0, pw, 18.0);
                        round_fill(&mut c, p, 9.0, crate::jetons::BAS, 1.0);
                        gras.draw(&mut c, &n, (p.cx() as f32, (p.cy() + 3.5) as f32), 10.0, crate::jetons::BLANC, Align::Center);
                    }
                    if let Some(on) = tu.etoile {
                        let (s, col) = if on { ("★", crate::jetons::TIER2) } else { ("☆", GRIS) };
                        text.draw(&mut c, s, ((q.x + q.w - 14.0) as f32, (q.y + 18.0) as f32), 13.0, col, Align::Center);
                    }
                }
                Toucher::Pied => {
                    let s = self.pied.clone().unwrap_or_default();
                    round_fill(&mut c, q, 14.0, crate::jetons::VERRE2.0, crate::jetons::VERRE2.1);
                    icone(&mut c, "clock", q.x + 22.0, q.cy(), 18.0);
                    text.draw(&mut c, &s, ((q.x + 42.0) as f32, (q.cy() + 4.5) as f32), 13.0, crate::jetons::DIM, Align::Left);
                }
                Toucher::Rien | Toucher::Fermer | Toucher::Partager => {}
            }
        }
        if let Some(v) = &self.vide {
            if self.sections.iter().all(|s| s.tuiles.is_empty()) {
                let y = self
                    .parties(text, w)
                    .iter()
                    .find(|(_, t)| *t == Toucher::Rien || *t == Toucher::Pied)
                    .map_or(80.0, |(z, _)| z.y)
                    - 30.0
                    + top;
                text.draw(&mut c, v, ((r.w / 2.0) as f32, y as f32), 13.0, crate::jetons::DIM, Align::Center);
            }
        }
        crate::px::put(&mut pm, 0.0, TETE, &c);
        Some(pm)
    }
}

/// A grey icon, centred: a Phosphor icon, else a drawing tool's own icon (no category tint: Jee wants the tools
/// grey).
pub fn icone(pm: &mut Pixmap, nom: &str, cx: f64, cy: f64, taille: f64) {
    if nom.is_empty() {
        return;
    }
    let px = crate::px::dev(taille).round() as u32;
    let img = if crate::phosphor::has(nom) {
        crate::phosphor::image(nom, px, GRIS)
    } else {
        crate::icons::icon(nom, px, GRIS)
    };
    match img {
        Some(img) => crate::px::put(pm, cx - taille / 2.0, cy - taille / 2.0, &img),
        // (a sign of the chart's menu: ↺ ◷ — ▲ ▼ ✕ ⚙)
        None => Text::default().draw(pm, nom, (cx as f32, (cy + taille * 0.32) as f32), (taille * 0.85) as f32, GRIS, Align::Center),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feuille() -> Feuille {
        let tuiles = |n: usize| (0..n).map(|k| Tuile::new(&format!("t{k}"), "gear", &format!("Tuile {k}"))).collect();
        Feuille {
            titre: "Argos".into(),
            partager: true,
            carte: Some(Carte {
                titre: "DÉMO · Argos 100K".into(),
                texte: "Aucun ordre réel".into(),
            }),
            sections: vec![
                Section {
                    titre: String::new(),
                    cols: 3,
                    tuiles: tuiles(6),
                },
                Section {
                    titre: "OUTILS".into(),
                    cols: 2,
                    tuiles: (0..8).map(|k| Tuile::new(&format!("o{k}"), "bell", "Outil")).collect(),
                },
            ],
            pied: Some("Bientôt (17)".into()),
            ..Default::default()
        }
    }

    /// Each zone is at least 40 points, inside the sheet, and touching it gives its action; ✕ and outside close;
    /// scrolling shows the rest; dragging down past a third closes.
    #[test]
    fn feuille_zones_et_toucher() {
        let t = Text::default();
        let (w, h) = (402.0, 874.0);
        let mut f = feuille();
        let r = f.rect(&t, w, h);
        assert!(r.h <= h * PART + 0.5 && (r.y + r.h - h).abs() < 0.5, "stuck to the bottom");
        let z = f.zones(&t, w, h);
        for (q, k) in &z {
            assert!(
                q.w >= TOUCHER && (q.h >= TOUCHER || q.y + q.h >= r.y + r.h - 0.5 || q.y <= r.y + TETE + 0.5),
                "{k:?} {q:?}"
            );
            assert!(q.x >= r.x && q.x + q.w <= r.x + r.w + 0.5);
            assert_eq!(f.toucher(&t, w, h, q.cx(), q.cy()), *k);
        }
        assert!(z.iter().any(|(_, k)| *k == Toucher::Carte) && z.iter().any(|(_, k)| *k == Toucher::Partager));
        assert_eq!(f.toucher(&t, w, h, 200.0, r.y - 10.0), Toucher::Fermer, "outside");
        // all the tiles can be reached by scrolling
        let mut vus = std::collections::HashSet::new();
        for _ in 0..20 {
            for (_, k) in f.zones(&t, w, h) {
                vus.insert(format!("{k:?}"));
            }
            f.defiler(&t, w, h, 120.0);
        }
        assert!(vus.contains("Pied") && vus.contains("Tuile(\"o7\")") && vus.contains("Tuile(\"t0\")"));
        // the drawing has the size of the sheet
        let pm = f.draw(&t, w, h, None).expect("image");
        assert!((crate::px::lw(&pm) - r.w).abs() < 1.5);
        // dragged down
        assert!(!f.tirer(&t, w, h, 40.0, false) && !f.tirer(&t, w, h, 40.0, true), "a little: comes back");
        f.tirer(&t, w, h, r.h, false);
        assert!(f.tirer(&t, w, h, r.h, true), "far: closes");
        assert_eq!(f.tire, 0.0);
    }

    /// Tabs wrap to the next row; a star has its own 40 × 40 zone, before its tile.
    #[test]
    fn onglets_et_etoiles() {
        let t = Text::default();
        let mut tu = Tuile::new("trend", "pencil-simple", "Ligne de tendance");
        tu.etoile = Some(false);
        let f = Feuille {
            titre: "Dessins".into(),
            champ: Some(String::new()),
            onglets: [
                "★ Favoris",
                "Lignes 20",
                "Fibonacci 12",
                "Figures 18",
                "Positions 4",
                "Formes 15",
                "Annotations 22",
                "Autres 14",
            ]
            .map(String::from)
            .to_vec(),
            sections: vec![Section {
                titre: String::new(),
                cols: 3,
                tuiles: vec![tu],
            }],
            ..Default::default()
        };
        let z = f.zones(&t, 402.0, 874.0);
        let ys: std::collections::BTreeSet<i64> = z.iter().filter(|(_, k)| matches!(k, Toucher::Onglet(_))).map(|(r, _)| r.y as i64).collect();
        assert!(ys.len() >= 2, "tabs on more than one row");
        let (e, _) = z.iter().find(|(_, k)| *k == Toucher::Etoile("trend".into())).unwrap();
        assert_eq!(f.toucher(&t, 402.0, 874.0, e.cx(), e.cy()), Toucher::Etoile("trend".into()));
        assert!(e.w >= TOUCHER && e.h >= TOUCHER);
    }
}
