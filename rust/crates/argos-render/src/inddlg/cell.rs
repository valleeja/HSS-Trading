//! ⚙️ Réglages d'un indicateur sur cell et tablette (modèle de Jee, 9 nov. 2026 : captures TradingView mobile ;
//! écran modèle dans la maquette, `#rgind`, mesuré à 390 × 844) : feuille plein écran en verre ; titre 22 pt ;
//! onglets « Paramètres en Entrée » · « Style » · « Visibilité » ; sections en MAJUSCULES grises ; lignes de 54 pt
//! (nom à gauche, champ arrondi à droite, case à cocher devant le nom, carré de couleur, ⓘ de l'aide) ; liste qui
//! défile ; pied ⋯ · Annuler · D'accord, posé juste au-dessus d'Argos AI. Même moteur que la fenêtre du PC : saisie,
//! palette, défauts, Annuler qui remet les valeurs d'avant, changements en direct sur la chart.

use super::{Hit, PALETTE, TRANSP};
use super::{IndDialog, Kind};
use crate::shapes::{round_fill, round_stroke, Rect};
use crate::text::{Align, Graisse, Rgb, Text};
use tiny_skia::Pixmap;

const TEXTE: Rgb = crate::jetons::TEXTE;
const DIM: Rgb = crate::jetons::DIM;
pub(super) const ONGLETS: [&str; 3] = ["Paramètres en Entrée", "Style", "Visibilité"];
const HAUT_LISTE: f64 = 170.0;
const LIGNE: f64 = 54.0;
const SECTION: f64 = 40.0;
const VISIBILITE: [(&str, &str, &str); 8] = [
    ("Ticks", "", ""),
    ("Secondes", "1", "59"),
    ("Minutes", "1", "59"),
    ("Heures", "1", "24"),
    ("Jours", "1", "366"),
    ("Semaines", "1", "52"),
    ("Mois", "1", "12"),
    ("Ranges", "", ""),
];

/// Une ligne de la liste.
#[derive(Debug, Clone, PartialEq)]
enum Ligne {
    Section(String),
    /// Les inputs de la ligne (un, ou plusieurs avec le même `inline`).
    Inputs(Vec<usize>),
    Vis(usize),
    Note(&'static str),
}

/// Nom d'un groupe en section : sans son émoji, en majuscules.
fn section(g: &str) -> String {
    g.trim_start_matches(|c: char| !c.is_alphanumeric()).trim().to_uppercase()
}

impl IndDialog {
    /// 📱 La feuille plein écran de `w` × `h` (cell, tablette).
    pub fn en_cell(mut self, w: f64, h: f64) -> Self {
        (self.cell, self.rect, self.tab, self.scroll) = (true, Rect::new(0.0, 0.0, w, h), 0, 0.0);
        self
    }

    fn bas_liste(&self) -> f64 {
        self.rect.h - 130.0
    }

    fn lignes_cell(&self) -> Vec<(Ligne, f64)> {
        let mut out = vec![];
        if self.tab == 2 {
            out.push((Ligne::Note("Visibilité par unité de temps : bientôt"), 40.0));
            out.extend((0..VISIBILITE.len()).map(|k| (Ligne::Vis(k), LIGNE)));
            return out;
        }
        let mut groupe = usize::MAX;
        for l in self.lines() {
            let g = self.params()[l[0]].group;
            if g != groupe {
                groupe = g;
                let nom = self.groups.get(g).map_or_else(String::new, |s| section(s));
                out.push((Ligne::Section(if self.tab == 1 && nom.is_empty() { "STYLE".into() } else { nom }), SECTION));
            }
            let h = self.line_h(&l) / super::ROW * LIGNE;
            out.push((Ligne::Inputs(l), h));
        }
        if out.is_empty() {
            out.push((
                Ligne::Note(if self.tab == 1 {
                    "Pas de couleur réglable pour cet indicateur"
                } else {
                    "Pas de réglage dans cet onglet"
                }),
                40.0,
            ));
        }
        out
    }

    fn places_cell(&self) -> Vec<(Ligne, Rect)> {
        // (maquette : la 1re ligne à y 214, son champ à 223)
        let mut y = HAUT_LISTE + 4.0 - self.scroll;
        let mut out = vec![];
        for (l, h) in self.lignes_cell() {
            out.push((l, Rect::new(0.0, y, self.rect.w, h)));
            y += h;
        }
        out
    }

    pub(super) fn max_cell(&self) -> f64 {
        let total: f64 = self.lignes_cell().iter().map(|(_, h)| h).sum::<f64>() + 16.0;
        (total - (self.bas_liste() - HAUT_LISTE)).max(0.0)
    }

    fn onglets(&self, text: &Text) -> Vec<Rect> {
        let gras = Text::graisse(Graisse::Bold);
        let _ = text;
        let mut x = 21.0;
        ONGLETS
            .iter()
            .map(|o| {
                let w = gras.width(o, 17.0) as f64;
                let r = Rect::new(x, 140.0, w, 25.0);
                x += w + 22.0;
                r
            })
            .collect()
    }

    fn pied(&self) -> [(Rect, Hit); 3] {
        let (w, y) = (self.rect.w, self.rect.h - 120.0);
        [
            (Rect::new(21.0, y, 60.0, 36.0), Hit::Defaults),
            (Rect::new(w - 212.0, y, 86.0, 36.0), Hit::Cancel),
            (Rect::new(w - 116.0, y, 96.0, 36.0), Hit::Ok),
        ]
    }

    /// Champ (nombre, liste, texte) de la ligne qui commence à `y`.
    fn champ(&self, y: f64) -> Rect {
        Rect::new(self.rect.w - 152.0, y + 9.0, 102.0, 36.0)
    }

    /// Les options de la liste ouverte (rectangles et valeurs).
    fn options(&self) -> Vec<(Rect, usize, String)> {
        let Some(i) = self.liste else { return vec![] };
        let Some((_, r)) = self.places_cell().into_iter().find(|(l, _)| matches!(l, Ligne::Inputs(v) if v.contains(&i))) else {
            return vec![];
        };
        let noms: Vec<String> = match self.params()[i].kind {
            Kind::Choice(o) => o.iter().map(|s| s.to_string()).collect(),
            Kind::Source => self.source_opts().into_iter().map(|(n, _)| n).collect(),
            _ => vec![],
        };
        let f = self.champ(r.y);
        let (lw, lh) = (220.0_f64.min(self.rect.w - 32.0), 40.0);
        let total = (noms.len() as f64 * lh).min(self.bas_liste() - HAUT_LISTE - 16.0);
        let x = (f.x + f.w - lw).max(16.0);
        let y = if f.y + f.h + 4.0 + total <= self.bas_liste() {
            f.y + f.h + 4.0
        } else {
            (f.y - 4.0 - total).max(HAUT_LISTE)
        };
        noms.into_iter()
            .enumerate()
            .map(|(k, n)| (Rect::new(x, y + k as f64 * lh, lw, lh), k, n))
            .filter(|(q, _, _)| q.y + q.h <= y + total + 0.5)
            .collect()
    }

    /// Zones touchables de la feuille.
    pub(super) fn zones_cell(&self, text: &Text) -> Vec<(Rect, Hit)> {
        let w = self.rect.w;
        // menu « ⋯ » (défauts) ouvert : par-dessus tout
        let mut out: Vec<(Rect, Hit)> = self.menu_defauts_zones().into_iter().map(|(q, h, _)| (q, h)).collect();
        // liste ouverte et palette : par-dessus le reste
        for (q, k, _) in self.options() {
            if let Some(i) = self.liste {
                out.push((q, Hit::Opt(i, k)));
            }
        }
        if let Some((px, py)) = self.palette_xy() {
            for (k, c) in PALETTE.iter().enumerate() {
                out.push((
                    Rect::new(px + 8.0 + (k % 10) as f64 * 30.0, py + 8.0 + (k / 10) as f64 * 30.0, 27.0, 27.0),
                    Hit::Pick(*c),
                ));
            }
            let tw = (300.0 - 16.0) / TRANSP.len() as f64;
            for (k, t) in TRANSP.iter().enumerate() {
                out.push((
                    Rect::new(px + 8.0 + k as f64 * tw, py + 8.0 + 4.0 * 30.0 + 6.0, tw - 4.0, 28.0),
                    Hit::Transp(*t),
                ));
            }
        }
        out.push((Rect::new(w - 45.0, 85.0, 30.0, 30.0), Hit::Cancel));
        out.extend(
            self.onglets(text)
                .into_iter()
                .enumerate()
                .map(|(k, r)| (Rect::new(r.x - 4.0, 132.0, r.w + 8.0, 36.0), Hit::Tab(k))),
        );
        out.extend(self.pied());
        let (haut, bas) = (HAUT_LISTE, self.bas_liste());
        for (l, r) in self.places_cell() {
            let Ligne::Inputs(v) = l else { continue };
            if r.y + 20.0 < haut || r.y + r.h - 20.0 > bas {
                continue;
            }
            // (plusieurs inputs sur une ligne : le dernier a la place à droite, les autres se touchent par leur nom)
            for (n, &i) in v.iter().enumerate() {
                let p = &self.params()[i];
                let dernier = n + 1 == v.len();
                match p.kind {
                    Kind::Bool => out.push((Rect::new(0.0, r.y, if dernier { w - 60.0 } else { w / 2.0 }, LIGNE), Hit::Check(i))),
                    Kind::Color => out.push((
                        Rect::new(w - 84.0 - if dernier { 0.0 } else { 44.0 * (v.len() - 1 - n) as f64 }, r.y + 10.0, 34.0, 34.0),
                        Hit::Swatch(i),
                    )),
                    Kind::Price | Kind::Time => {
                        out.push((Rect::new(w - 46.0, r.y + 9.0, 30.0, 36.0), Hit::Here(i)));
                        out.push((self.champ(r.y), Hit::Field(i)));
                    }
                    _ if dernier => out.push((self.champ(r.y), Hit::Field(i))),
                    _ => {}
                }
            }
            if let Some(&i) = v.iter().rev().find(|&&i| !self.params()[i].tip.is_empty()) {
                out.push((Rect::new(w - 38.0, r.y + 19.0, 16.0, 16.0), Hit::Tip(i)));
            }
        }
        out
    }

    /// Coin haut gauche de la palette ouverte (sous le carré de couleur, ou au-dessus s'il n'y a pas la place).
    fn palette_xy(&self) -> Option<(f64, f64)> {
        let i = self.palette?;
        let (_, r) = self.places_cell().into_iter().find(|(l, _)| matches!(l, Ligne::Inputs(v) if v.contains(&i)))?;
        let (pw, ph) = (316.0, 8.0 + 4.0 * 30.0 + 6.0 + 28.0 + 8.0);
        let x = (self.rect.w - 16.0 - pw).max(8.0);
        let y = if r.y + r.h + ph <= self.bas_liste() {
            r.y + r.h
        } else {
            (r.y - ph).max(HAUT_LISTE)
        };
        Some((x, y))
    }

    /// 📱 Champs où l'on tape (pour le clavier du cell : la page met le focus sur son champ caché quand le doigt y
    /// touche).
    pub fn champs_saisie(&self, text: &Text) -> Vec<Rect> {
        if !self.cell {
            return vec![self.rect];
        }
        self.zones_cell(text)
            .into_iter()
            .filter(|(_, h)| matches!(h, Hit::Field(i) if !matches!(self.params()[*i].kind, Kind::Choice(_) | Kind::Source)))
            .map(|(r, _)| r)
            .collect()
    }

    /// Dessin de la feuille (coordonnées de la fenêtre : elle la couvre toute).
    pub(super) fn draw_cell(&self, text: &Text, mouse: (f64, f64)) -> Option<(Pixmap, Rect)> {
        let (w, h) = (self.rect.w, self.rect.h);
        let mut pm = crate::px::new(w.ceil() as u32, h.ceil() as u32)?;
        round_fill(&mut pm, Rect::new(0.0, 0.0, w, h), 0.0, crate::jetons::sur_fond(crate::jetons::VERRE), 1.0);
        let gras = Text::graisse(Graisse::Bold);
        let inp = self.inputs();
        let survol = |q: &Rect| q.contains(mouse.0, mouse.1);
        // la liste (coupée entre le trait des onglets et le pied)
        let mut liste = crate::px::new(w.ceil() as u32, h.ceil() as u32)?;
        for (l, r) in self.places_cell() {
            if r.y + r.h < HAUT_LISTE || r.y > self.bas_liste() {
                continue;
            }
            match l {
                Ligne::Section(s) => text.draw(&mut liste, &s, (21.0, (r.y + 30.0) as f32), 13.0, DIM, Align::Left),
                Ligne::Note(t) => text.draw(&mut liste, t, (21.0, (r.y + 26.0) as f32), 14.0, DIM, Align::Left),
                Ligne::Vis(k) => {
                    let (n, a, b) = VISIBILITE[k];
                    let c = Rect::new(21.0, r.y + 18.0, 18.0, 18.0);
                    round_fill(&mut liste, c, 5.0, crate::jetons::BLANC, 0.35);
                    text.draw(&mut liste, n, (51.0, (r.y + 33.0) as f32), 17.0, DIM, Align::Left);
                    if !a.is_empty() {
                        for (x, v) in [(w - 190.0, a), (w - 108.0, b)] {
                            let q = Rect::new(x, r.y + 9.0, 62.0, 36.0);
                            round_stroke(&mut liste, q, 8.0, crate::jetons::sur_fond((crate::jetons::BLANC, 0.22)), 1.0);
                            text.draw(&mut liste, v, ((q.x + 10.0) as f32, (q.y + 24.0) as f32), 17.0, DIM, Align::Left);
                        }
                        text.draw(&mut liste, "–", ((w - 122.0) as f32, (r.y + 33.0) as f32), 17.0, DIM, Align::Center);
                    }
                }
                Ligne::Inputs(v) => {
                    let i0 = v[0];
                    let p0 = &self.params()[i0];
                    let actif = inp.active(i0);
                    let c = if actif { TEXTE } else { DIM };
                    let x_nom = if matches!(p0.kind, Kind::Bool) { 51.0 } else { 21.0 };
                    if matches!(p0.kind, Kind::Bool) {
                        let q = Rect::new(21.0, r.y + 18.0, 18.0, 18.0);
                        let on = self.val(i0) != 0.0;
                        if on {
                            round_fill(&mut liste, q, 5.0, crate::jetons::BLANC, if actif { 1.0 } else { 0.5 });
                            crate::shapes::lines(&mut liste, q.cx(), q.cy(), &[&[(-4.0, 0.0), (-1.0, 3.5), (4.5, -3.5)]], Rgb(0, 0, 0), 1.8);
                        } else {
                            round_stroke(&mut liste, q, 5.0, crate::jetons::sur_fond((crate::jetons::BLANC, 0.5)), 1.5);
                        }
                    }
                    // nom (sur deux lignes s'il est long)
                    let place = w - 160.0 - x_nom;
                    let nom = p0.label;
                    if text.width(nom, 17.0) as f64 <= place {
                        text.draw(&mut liste, nom, (x_nom as f32, (r.y + 33.0) as f32), 17.0, c, Align::Left);
                    } else {
                        let (a, b) = coupe(text, nom, place);
                        text.draw(&mut liste, &a, (x_nom as f32, (r.y + 23.0) as f32), 17.0, c, Align::Left);
                        text.draw(
                            &mut liste,
                            &text.fit(&b, 17.0, place),
                            (x_nom as f32, (r.y + 44.0) as f32),
                            17.0,
                            c,
                            Align::Left,
                        );
                    }
                    for &i in &v {
                        let p = &self.params()[i];
                        match p.kind {
                            Kind::Bool => {}
                            Kind::Color => {
                                if let Some((q, _)) = self.zones_cell(text).into_iter().find(|(_, h)| *h == Hit::Swatch(i)) {
                                    let col = self.val(i) as u32;
                                    let a = 1.0 - (((col >> 24) & 0x7f).min(100) as f32) / 100.0;
                                    round_fill(&mut liste, q, 8.0, Rgb((col >> 16) as u8, (col >> 8) as u8, col as u8), a.max(0.15));
                                    round_stroke(&mut liste, q, 8.0, crate::jetons::sur_fond((crate::jetons::BLANC, 0.22)), 1.0);
                                }
                            }
                            _ => {
                                if let Some((q, _)) = self.zones_cell(text).into_iter().find(|(_, h)| *h == Hit::Field(i)) {
                                    let en_saisie = self.edit.as_ref().filter(|(k, _)| *k == i).map(|(_, s)| format!("{s}▏"));
                                    let bord = if en_saisie.is_some() || self.liste == Some(i) {
                                        crate::jetons::ACCENT
                                    } else {
                                        crate::jetons::sur_fond((crate::jetons::BLANC, 0.22))
                                    };
                                    round_stroke(&mut liste, q, 8.0, bord, 1.0);
                                    let liste_ = matches!(p.kind, Kind::Choice(_) | Kind::Source);
                                    let t = en_saisie.unwrap_or_else(|| self.field_txt(i));
                                    let place = q.w - if liste_ { 30.0 } else { 18.0 };
                                    text.draw(
                                        &mut liste,
                                        &text.fit(&t, 17.0, place),
                                        ((q.x + 10.0) as f32, (q.y + 24.0) as f32),
                                        17.0,
                                        c,
                                        Align::Left,
                                    );
                                    if liste_ {
                                        text.draw(&mut liste, "⌄", ((q.x + q.w - 12.0) as f32, (q.y + 21.0) as f32), 14.0, DIM, Align::Center);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        // ⓘ des aides
        for (q, hit) in self.zones_cell(text) {
            if let Hit::Tip(_) = hit {
                if q.y >= HAUT_LISTE && q.y + q.h <= self.bas_liste() {
                    round_stroke(&mut liste, q, 8.0, DIM, 1.3);
                    gras.draw(&mut liste, "i", (q.cx() as f32, (q.cy() + 3.8) as f32), 10.0, DIM, Align::Center);
                }
            }
        }
        // (seulement entre le trait des onglets et le pied)
        {
            let k = crate::px::dev(1.0);
            let (haut, bas) = ((HAUT_LISTE * k).round() as usize, (self.bas_liste() * k).round() as usize);
            let lw = liste.width() as usize;
            let d = liste.data_mut();
            let n = d.len();
            d[..(haut * lw * 4).min(n)].fill(0);
            if bas * lw * 4 < n {
                d[bas * lw * 4..].fill(0);
            }
        }
        crate::px::put(&mut pm, 0.0, 0.0, &liste);
        // barre de défilement
        let max = self.max_cell();
        if max > 0.0 {
            let vue = self.bas_liste() - HAUT_LISTE;
            let th = (vue * vue / (vue + max)).max(30.0);
            let ty = HAUT_LISTE + self.scroll / max * (vue - th);
            crate::jetons::barre_defilement(&mut pm, w - 3.0, ty, th, false);
        }
        // en-tête : titre, ✕, onglets et leur trait
        gras.draw(&mut pm, &text.fit(&self.title, 22.0, w - 90.0), (21.0, 108.0), 22.0, TEXTE, Align::Left);
        let x = Rect::new(w - 45.0, 85.0, 30.0, 30.0);
        crate::shapes::lines(&mut pm, x.cx(), x.cy(), &[&[(-7.0, -7.0), (7.0, 7.0)], &[(7.0, -7.0), (-7.0, 7.0)]], TEXTE, 1.8);
        for (k, r) in self.onglets(text).into_iter().enumerate() {
            let on = k == self.tab;
            gras.draw(
                &mut pm,
                ONGLETS[k],
                (r.x as f32, (r.y + 18.0) as f32),
                17.0,
                if on { TEXTE } else { DIM },
                Align::Left,
            );
            if on {
                round_fill(&mut pm, Rect::new(r.x, 162.0, r.w, 3.0), 1.5, TEXTE, 1.0);
            }
        }
        crate::fill(&mut pm, 0.0, 165.0, w, 1.0, crate::jetons::sur_fond((crate::jetons::BLANC, 0.14)), 1.0);
        // pied : ⋯ (valeurs par défaut) · Annuler · D'accord
        for (q, hit) in self.pied() {
            match hit {
                Hit::Defaults => {
                    let bord = if self.menu_defauts_ouvert() {
                        TEXTE
                    } else {
                        crate::jetons::sur_fond((crate::jetons::BLANC, 0.22))
                    };
                    round_stroke(&mut pm, q, 8.0, bord, 1.0);
                    gras.draw(&mut pm, "⋯", (q.cx() as f32, (q.cy() + 6.0) as f32), 16.0, TEXTE, Align::Center);
                }
                Hit::Cancel => {
                    round_stroke(&mut pm, q, 8.0, TEXTE, 1.0);
                    gras.draw(&mut pm, "Annuler", (q.cx() as f32, (q.cy() + 6.0) as f32), 16.0, TEXTE, Align::Center);
                }
                _ => {
                    round_fill(&mut pm, q, 8.0, crate::jetons::BLANC, if survol(&q) { 0.9 } else { 1.0 });
                    gras.draw(&mut pm, "D'accord", (q.cx() as f32, (q.cy() + 6.0) as f32), 16.0, Rgb(0, 0, 0), Align::Center);
                }
            }
        }
        // liste ouverte (pilule de verre) et palette : par-dessus
        let opts = self.options();
        if let (Some(i), Some(first)) = (self.liste, opts.first()) {
            let last = opts.last().map_or(first.0, |o| o.0);
            let carte = Rect::new(first.0.x, first.0.y, first.0.w, last.y + last.h - first.0.y);
            round_fill(&mut pm, carte, 14.0, crate::jetons::sur_fond(crate::jetons::VERRE), 1.0);
            round_stroke(&mut pm, carte, 14.0, crate::jetons::sur_fond(crate::jetons::LISERE), 1.0);
            let cur = self.field_txt(i);
            for (q, _, n) in &opts {
                if *n == cur {
                    let (pc, pa) = crate::jetons::PUCE_ON;
                    round_fill(&mut pm, Rect::new(q.x + 4.0, q.y + 3.0, q.w - 8.0, q.h - 6.0), 10.0, pc, pa);
                }
                text.draw(
                    &mut pm,
                    &text.fit(n, 15.0, q.w - 24.0),
                    ((q.x + 14.0) as f32, (q.y + 25.0) as f32),
                    15.0,
                    TEXTE,
                    Align::Left,
                );
            }
        }
        // menu « ⋯ » : Réinitialiser les réglages · Enregistrer comme défaut (· Revenir aux réglages d'origine)
        if let Some(carte) = self.menu_defauts_carte() {
            round_fill(&mut pm, carte, 14.0, crate::jetons::sur_fond(crate::jetons::VERRE), 1.0);
            round_stroke(&mut pm, carte, 14.0, crate::jetons::sur_fond(crate::jetons::LISERE), 1.0);
            for (q, _, t) in self.menu_defauts_zones() {
                if survol(&q) {
                    let (pc, pa) = crate::jetons::PUCE_ON;
                    round_fill(&mut pm, Rect::new(q.x + 4.0, q.y + 3.0, q.w - 8.0, q.h - 6.0), 10.0, pc, pa);
                }
                text.draw(&mut pm, t, ((q.x + 14.0) as f32, (q.y + 27.0) as f32), 16.0, TEXTE, Align::Left);
            }
        }
        if let Some((px, py)) = self.palette_xy() {
            let carte = Rect::new(px, py, 316.0, 8.0 + 4.0 * 30.0 + 6.0 + 28.0 + 8.0);
            round_fill(&mut pm, carte, 14.0, crate::jetons::sur_fond(crate::jetons::VERRE), 1.0);
            round_stroke(&mut pm, carte, 14.0, crate::jetons::sur_fond(crate::jetons::LISERE), 1.0);
            for (q, hit) in self.zones_cell(text) {
                match hit {
                    Hit::Pick(c) => round_fill(&mut pm, q, 6.0, Rgb((c >> 16) as u8, (c >> 8) as u8, c as u8), 1.0),
                    Hit::Transp(t) => {
                        round_stroke(&mut pm, q, 8.0, crate::jetons::sur_fond(crate::jetons::LISERE), 1.0);
                        text.draw(&mut pm, &format!("{t} %"), (q.cx() as f32, (q.cy() + 4.5) as f32), 12.0, TEXTE, Align::Center);
                    }
                    _ => {}
                }
            }
        }
        Some((pm, self.rect))
    }
}

/// Coupe `s` en deux lignes au dernier espace qui tient dans `place`.
fn coupe(text: &Text, s: &str, place: f64) -> (String, String) {
    let mut a = String::new();
    for (k, mot) in s.split(' ').enumerate() {
        let essai = if k == 0 { mot.to_string() } else { format!("{a} {mot}") };
        if text.width(&essai, 17.0) as f64 > place && !a.is_empty() {
            let reste = s[a.len()..].trim().to_string();
            return (a, reste);
        }
        a = essai;
    }
    (a, String::new())
}

#[cfg(test)]
mod tests {
    use super::super::{IndClick, IndDialog};
    use super::*;
    use crate::indicators::{values, Ind, Settings};

    /// ⚙️ Jee (9 nov. 2026) : chacun des 43 indicateurs a sa feuille de réglages sur cell, avec au moins un réglage à
    /// toucher ; un toucher le change (saisie au clavier, case, ou option de la liste), Annuler remet la valeur
    /// d'avant ; les trois onglets s'ouvrent et la liste défile.
    #[test]
    fn reglages_des_43_au_doigt() {
        let t = Text::default();
        for ind in Ind::ALL {
            let mut d = IndDialog::new(ind, values(&Settings::new(), ind), 390.0, 844.0).en_cell(390.0, 844.0);
            let z = d.zones_cell(&t);
            let avant = d.vals.clone();
            // le premier réglage : un champ, une case ou une couleur
            // (un nombre d'abord ; sinon une case, une liste, une couleur)
            let rang = |h: &Hit| match h {
                Hit::Field(i) if matches!(d.params()[*i].kind, Kind::Num(..)) => 0,
                Hit::Check(_) => 1,
                Hit::Field(i) if matches!(d.params()[*i].kind, Kind::Choice(_) | Kind::Source) => 2,
                Hit::Swatch(_) => 3,
                _ => 9,
            };
            let (q, h) = *z
                .iter()
                .filter(|(_, h)| rang(h) < 9)
                .min_by_key(|(_, h)| rang(h))
                .unwrap_or_else(|| panic!("{ind:?} : aucun réglage dans la feuille"));
            let changed = match h {
                Hit::Check(_) => d.click(q.cx(), q.cy(), &t),
                Hit::Swatch(_) => {
                    d.click(q.cx(), q.cy(), &t);
                    let pick = d
                        .zones_cell(&t)
                        .into_iter()
                        .find(|(_, h)| matches!(h, Hit::Pick(c) if *c == 0xf23645))
                        .unwrap()
                        .0;
                    d.click(pick.cx(), pick.cy(), &t)
                }
                Hit::Field(i) => match d.params()[i].kind {
                    Kind::Choice(_) | Kind::Source => {
                        d.click(q.cx(), q.cy(), &t);
                        let opts = d.options();
                        assert!(opts.len() > 1, "{ind:?} : la liste montre ses options");
                        let autre = opts.iter().find(|(_, _, n)| *n != d.field_txt(i)).unwrap().0;
                        d.click(autre.cx(), autre.cy(), &t)
                    }
                    _ => {
                        d.click(q.cx(), q.cy(), &t);
                        assert!(d.typing(), "{ind:?} : le champ prend le clavier");
                        d.backspace();
                        d.backspace();
                        d.backspace();
                        let Kind::Num(min, max, _) = d.params()[i].kind else { panic!("{ind:?}") };
                        let v = if (min..=max).contains(&7.0) && d.val(i) != 7.0 { "7" } else { "1" };
                        v.chars().for_each(|c| d.type_char(c));
                        d.enter()
                    }
                },
                _ => unreachable!(),
            };
            assert_eq!(changed, IndClick::Changed, "{ind:?} : le réglage change");
            assert_ne!(d.vals, avant, "{ind:?} : la valeur a changé");
            // Annuler : la feuille se ferme sans garder (l'app remet les valeurs d'avant)
            let (q, _) = *d.zones_cell(&t).iter().find(|(_, h)| *h == Hit::Cancel).unwrap();
            assert_eq!(d.click(q.cx(), q.cy(), &t), IndClick::Close(false), "{ind:?} : Annuler");
            assert_eq!(d.backup, avant);
            // les trois onglets
            for k in 0..3 {
                let q = d.zones_cell(&t).into_iter().find(|(_, h)| *h == Hit::Tab(k)).unwrap().0;
                d.click(q.cx(), q.cy(), &t);
                assert_eq!(d.tab, k);
                assert!(!d.lignes_cell().is_empty());
            }
        }
        // places de la maquette (`#rgind`, 390 × 844) : 1er champ à y 223, pied à 724
        let d = IndDialog::new(Ind::Ichimoku, values(&Settings::new(), Ind::Ichimoku), 390.0, 844.0).en_cell(390.0, 844.0);
        let champ = d.champs_saisie(&t)[0];
        assert_eq!((champ.x, champ.y, champ.w, champ.h), (238.0, 223.0, 102.0, 36.0));
        let ok = d.zones_cell(&t).into_iter().find(|(_, h)| *h == Hit::Ok).unwrap().0;
        assert_eq!((ok.x, ok.y, ok.w, ok.h), (274.0, 724.0, 96.0, 36.0));
        // une longue liste (Cipher) défile jusqu'au bout
        let mut d = IndDialog::new(Ind::Cipher, values(&Settings::new(), Ind::Cipher), 390.0, 844.0).en_cell(390.0, 844.0);
        assert!(d.max_cell() > 0.0, "Cipher : la liste dépasse");
        d.wheel(100_000.0, &t);
        assert!((d.scroll - d.max_cell()).abs() < 0.5, "jusqu'au bout");
        let (q, _) = *d.zones_cell(&t).iter().find(|(_, h)| *h == Hit::Ok).unwrap();
        assert_eq!(d.click(q.cx(), q.cy(), &t), IndClick::Close(true), "D'accord");
    }
}

#[cfg(test)]
mod tests_style {
    use super::super::{IndClick, IndDialog};
    use super::*;
    use crate::indicators::{values, Ind, Settings};

    /// 🎨 Onglet Style (Jee, « tout fonctionnel ») : chaque sortie d'Ichimoku a sa couleur, son épaisseur (1 à 4) et son
    /// type de ligne (plein, tirets, points) ; choisir « Tirets » et une épaisseur de 3 change le dessin de la chart.
    #[test]
    fn style_epaisseur_et_type_de_ligne() {
        let t = Text::default();
        let mut d = IndDialog::new(Ind::Ichimoku, values(&Settings::new(), Ind::Ichimoku), 390.0, 844.0).en_cell(390.0, 844.0);
        let q = d.zones_cell(&t).into_iter().find(|(_, h)| *h == Hit::Tab(1)).unwrap().0;
        d.click(q.cx(), q.cy(), &t);
        let noms: Vec<&str> = d.lines().iter().map(|l| d.params()[l[0]].label).collect();
        for s in ["Tenkan", "Kijun", "Senkou A", "Senkou B", "Chikou"] {
            assert!(
                noms.contains(&format!("Épaisseur · {s}").as_str()) && noms.contains(&format!("Type de ligne · {s}").as_str()),
                "{s} : {noms:?}"
            );
        }
        // type de ligne de Tenkan : Tirets (2e option de la liste)
        let i = d.params().iter().position(|p| p.key == "ty1").unwrap();
        let champ = d.zones_cell(&t).into_iter().find(|(_, h)| *h == Hit::Field(i)).unwrap().0;
        d.click(champ.cx(), champ.cy(), &t);
        let tirets = d.options().into_iter().find(|(_, _, n)| n == "Tirets").unwrap().0;
        assert_eq!(d.click(tirets.cx(), tirets.cy(), &t), IndClick::Changed);
        let e = d.params().iter().position(|p| p.key == "ep1").unwrap();
        let champ = d.zones_cell(&t).into_iter().find(|(_, h)| *h == Hit::Field(e)).unwrap().0;
        d.click(champ.cx(), champ.cy(), &t);
        d.backspace();
        d.backspace();
        d.backspace();
        d.type_char('3');
        assert_eq!(d.enter(), IndClick::Changed);
        // appliqué au calcul : Tenkan en tirets, épaisseur 3
        let mut set = Settings::new();
        set.insert(Ind::Ichimoku, d.vals.clone());
        let bars: Vec<argos_core::Bar> = (0..200)
            .map(|i| {
                let c = 100.0 + (i as f64 / 9.0).sin() * 5.0;
                argos_core::Bar {
                    time: 1_700_000_000 + i * 60,
                    open: c,
                    high: c + 1.0,
                    low: c - 1.0,
                    close: c + 0.2,
                    volume: 10.0,
                }
            })
            .collect();
        let pl = crate::indicators::compute(&bars, 60, &[Ind::Ichimoku], &set, None, 0, &Default::default(), &[])
            .plots
            .remove(0);
        assert_eq!((pl.dash[0], pl.lines[0].2), (1, 3.0), "Tenkan : tirets, épaisseur 3");
        assert_eq!((pl.dash[1], pl.lines[1].2), (0, 1.0), "Kijun : inchangée");
    }

    /// 🎨 A built-in written in Argos Script (TradingView's SMA): its ⚙ « Style » lists the colour, width and line type
    /// of each plot — on the PC in the tab « 🎨 Style », on the phone in the Style tab of the sheet (not in Entrée) —
    /// and both windows draw.
    #[test]
    fn style_d_un_integre_en_script() {
        let t = Text::default();
        let attendus = [
            "Couleur · MA",
            "Épaisseur · MA",
            "Type de ligne · MA",
            "Couleur · SMA-based MA",
            "Type de ligne · Upper Bollinger Band",
        ];
        // PC: the tab of the group « 🎨 Style », the last one
        let mut d = IndDialog::new(Ind::Sma, values(&Settings::new(), Ind::Sma), 1400.0, 860.0);
        let k = d.groups.iter().position(|g| *g == argos_script::style_integres::GROUPE).expect("a Style tab");
        assert_eq!(k, d.groups.len() - 1);
        d.tab = k;
        let noms: Vec<&str> = d.lines().iter().map(|l| d.params()[l[0]].label).collect();
        assert!(attendus.iter().all(|a| noms.contains(a)), "{noms:?}");
        assert!(!noms.contains(&"Length"), "the inputs stay in their tab");
        assert!(d.draw(&t, (0.0, 0.0)).is_some());
        // phone: the sheet's Style tab
        let mut d = IndDialog::new(Ind::Sma, values(&Settings::new(), Ind::Sma), 390.0, 844.0).en_cell(390.0, 844.0);
        let entree: Vec<&str> = d.lines().iter().map(|l| d.params()[l[0]].label).collect();
        assert!(!entree.iter().any(|n| attendus.contains(n)), "not in Entrée: {entree:?}");
        let q = d.zones_cell(&t).into_iter().find(|(_, h)| *h == Hit::Tab(1)).unwrap().0;
        d.click(q.cx(), q.cy(), &t);
        let noms: Vec<&str> = d.lines().iter().map(|l| d.params()[l[0]].label).collect();
        assert!(attendus.iter().all(|a| noms.contains(a)), "{noms:?}");
        assert!(d.draw(&t, (0.0, 0.0)).is_some());
    }
}
