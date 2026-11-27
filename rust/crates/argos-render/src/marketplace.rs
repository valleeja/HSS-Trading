//! 🛒 Marketplace Argos, comme v1 : barre du haut (recherche, Mes indicateurs, Favoris, Ma page, Publier), catégories,
//! filtres à gauche (Source, Où ça s'affiche, Publié), grille de cartes avec aperçu calculé sur les bougies affichées,
//! ♡ favori, ⭐ Officiel, ➕ Ajouter / ✅ Ajouté, ⓘ détails. Les indicateurs officiels d'Argos sont là ; ceux de la
//! communauté (signés sur Nostr) arrivent avec le compte Argos.

use crate::indicators::Ind;
use crate::palette as pal;
use crate::palette::{accent, GOLD, GREY, WHITE};
use crate::shapes::{lines, round_fill, round_stroke, Rect};
use crate::text::{Align, Rgb, Text};
use tiny_skia::Pixmap;

/// Catégories de v1 : (clé, nom) — dans le catalogue unique.
pub use argos_script::catalogue::MARCHE_CATEGORIES as CATS;

/// Ce qu'un clic demande à la fenêtre.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MkClick {
    None,
    Changed,
    Close,
    /// Mettre / enlever cet indicateur de la chart.
    Toggle(Ind),
    /// Les favoris ont changé (à garder).
    Favs,
    Soon(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Hit {
    Close,
    Search,
    Mine,
    Fav,
    Me,
    Publish,
    Refresh,
    Cat(usize),
    Src(usize),
    Kind(usize),
    Age(usize),
    Heart(usize),
    Add(usize),
    Info(usize),
    InfoClose,
    InfoAdd,
}

/// 🫧 The Marketplace preview of Argos Bubbles: no real trades there, so no bubble — said plainly.
pub const BULLES_APERCU: &str = "Bulles : vrais trades absents dans l'aperçu";

/// État de la page (les favoris sont gardés par la fenêtre).
#[derive(Debug, Clone, PartialEq)]
pub struct MkPage {
    pub query: String,
    /// Recherche en cours de saisie.
    pub typing: bool,
    cat: usize,
    /// 0 tout, 1 officiel, 2 communauté.
    src: usize,
    /// 0 partout, 1 sur le prix, 2 panneau.
    kind: usize,
    /// 0 tout, 1 7 jours, 2 30 jours.
    age: usize,
    /// 0 tout, 1 mes indicateurs (ajoutés), 2 favoris.
    view: usize,
    info: Option<usize>,
    /// Défilement de la grille (pixels).
    scroll: f64,
    /// Favoris : index dans les officiels du catalogue (gardés dans market_fav.json).
    pub favs: Vec<Ind>,
}

impl MkPage {
    /// La grille est tout en haut (glisser vers le bas ferme alors la page).
    pub fn at_top(&self) -> bool {
        self.scroll <= 0.5
    }

    pub fn new(favs: Vec<Ind>) -> Self {
        Self {
            query: String::new(),
            typing: false,
            cat: 0,
            src: 0,
            kind: 0,
            age: 0,
            view: 0,
            info: None,
            scroll: 0.0,
            favs,
        }
    }

    /// Cartes affichées (index dans les officiels du catalogue) selon les filtres et la recherche ; `on` : indicateurs sur la chart.
    fn shown(&self, on: &[Ind]) -> Vec<usize> {
        let q = self.query.to_lowercase();
        let officiels = crate::catalogue::officiels();
        (0..officiels.len())
            .filter(|&k| {
                let (ind, cat, gold, desc) = officiels[k];
                let ckey = CATS[self.cat].0;
                let cat_ok = match ckey {
                    "all" | "off" => true,
                    "com" => false,
                    "gold" => gold,
                    c => c == cat,
                };
                let view_ok = match self.view {
                    1 => on.contains(&ind),
                    2 => self.favs.contains(&ind),
                    _ => true,
                };
                // les officiels n'ont pas de date de publication : « 7 jours / 30 jours » ne garde que la communauté
                cat_ok
                    && view_ok
                    && self.src != 2
                    && self.age == 0
                    && (self.kind == 0 || (self.kind == 1) != ind.oscillator())
                    && (q.is_empty() || format!("{} {desc} argos", ind.name()).to_lowercase().contains(&q))
            })
            .collect()
    }

    fn layout(&self, text: &Text, w: f64, h: f64, on: &[Ind]) -> Vec<(Rect, Hit)> {
        let inner = (w - 40.0).min(1500.0);
        let x0 = (w - inner) / 2.0;
        let mut z = vec![];
        if let Some(k) = self.info {
            let r = info_rect(w, h);
            z.push((Rect::new(r.x + r.w - 42.0, r.y + 12.0, 30.0, 30.0), Hit::InfoClose));
            z.push((Rect::new(r.x + 24.0, r.y + r.h - 58.0, 160.0, 38.0), Hit::InfoAdd));
            let _ = k;
        }
        let phone = w < PHONE;
        if phone {
            // 📱 cell, comme v1 : ✕ en haut, Mes indicateurs / Favoris / Ma page, recherche, catégories, 2 colonnes
            z.push((Rect::new(w - 46.0, 12.0, 32.0, 32.0), Hit::Close));
            let mut nx = x0;
            for (l, hit) in [
                ("📦 Mes indicateurs".to_string(), Hit::Mine),
                (format!("♡ Favoris {}", self.favs.len()), Hit::Fav),
                ("👤 Ma page".into(), Hit::Me),
            ] {
                let bw = text.width(&l, 13.5) as f64 + 22.0;
                z.push((Rect::new(nx, 56.0, bw, 32.0), hit));
                nx += bw + 6.0;
            }
            z.push((Rect::new(x0, 98.0, inner, 36.0), Hit::Search));
            z.extend(cats_cell(text, w).0);
        } else {
            z.push((Rect::new(x0 + 190.0, 14.0, (inner * 0.32).max(220.0), 36.0), Hit::Search));
        }
        // navigation à droite
        let nav: Vec<(String, Hit)> = vec![
            ("📦 Mes indicateurs".into(), Hit::Mine),
            (format!("♡ Favoris {}", self.favs.len()), Hit::Fav),
            ("👤 Ma page".into(), Hit::Me),
            ("📤 Publier".into(), Hit::Publish),
            ("🔄".into(), Hit::Refresh),
        ];
        let mut nx = x0 + inner - 40.0;
        if !phone {
            z.push((Rect::new(nx, 16.0, 32.0, 32.0), Hit::Close));
        }
        for (l, hit) in nav.iter().rev().filter(|_| !phone) {
            let bw = text.width(l, 13.5) as f64 + 22.0;
            nx -= bw + 8.0;
            z.push((Rect::new(nx, 16.0, bw, 32.0), *hit));
        }
        let mut cx = x0;
        for (k, (_, l)) in CATS.iter().enumerate().filter(|_| !phone) {
            let bw = text.width(l, 13.5) as f64 + 24.0;
            z.push((Rect::new(cx, 64.0, bw, 32.0), Hit::Cat(k)));
            cx += bw + 8.0;
        }
        // filtres à gauche (pas sur cell)
        let side = Rect::new(x0, 112.0, 250.0, h - 132.0);
        let seg = |y: f64, n: usize, f: fn(usize) -> Hit, z: &mut Vec<(Rect, Hit)>| {
            let bw = (side.w - 32.0) / n as f64;
            for k in 0..n {
                z.push((Rect::new(side.x + 16.0 + k as f64 * bw, y, bw - 4.0, 30.0), f(k)));
            }
        };
        if !phone {
            seg(side.y + 44.0, 3, Hit::Src, &mut z);
            seg(side.y + 150.0, 3, Hit::Kind, &mut z);
            seg(side.y + 232.0, 3, Hit::Age, &mut z);
        }
        // grille de cartes
        let grid = grid_rect(w, h);
        let (cw, ch) = (card_w(w), 236.0);
        let cols = (((grid.w + 14.0) / (cw + 14.0)).floor() as usize).max(1);
        for (n, k) in self.shown(on).into_iter().enumerate() {
            let (c, r) = (n % cols, n / cols);
            let card = Rect::new(grid.x + c as f64 * (cw + 14.0), grid.y + r as f64 * (ch + 14.0) - self.scroll, cw, ch);
            // la grille défile : une carte coupée par le haut ou le bas n'est pas montrée
            if card.y < grid.y - 1.0 || card.y + card.h > grid.y + grid.h + 1.0 {
                continue;
            }
            z.push((Rect::new(card.x + card.w - 36.0, card.y + 8.0, 28.0, 28.0), Hit::Heart(k)));
            z.push((Rect::new(card.x + 12.0, card.y + ch - 44.0, card.w - 64.0, 32.0), Hit::Add(k)));
            z.push((Rect::new(card.x + card.w - 46.0, card.y + ch - 44.0, 34.0, 32.0), Hit::Info(k)));
        }
        z
    }

    /// Clic en `(x, y)` (fenêtre `w` × `h`) ; `on` : indicateurs sur la chart.
    pub fn click(&mut self, text: &Text, w: f64, h: f64, on: &[Ind], x: f64, y: f64) -> MkClick {
        let hit = self.layout(text, w, h, on).into_iter().find(|(r, _)| r.contains(x, y)).map(|(_, h)| h);
        self.typing = hit == Some(Hit::Search);
        if let Some(k) = self.info {
            return match hit {
                Some(Hit::InfoAdd) => MkClick::Toggle(crate::catalogue::officiels()[k].0),
                Some(Hit::InfoClose) => {
                    self.info = None;
                    MkClick::Changed
                }
                _ if !info_rect(w, h).contains(x, y) => {
                    self.info = None;
                    MkClick::Changed
                }
                _ => MkClick::None,
            };
        }
        match hit {
            Some(Hit::Close) => MkClick::Close,
            Some(Hit::Search) => MkClick::Changed,
            Some(Hit::Mine) => self.set_view(1),
            Some(Hit::Fav) => self.set_view(2),
            Some(Hit::Me) => MkClick::Soon("👤 Ma page : arrive avec le compte Argos (24 mots), bientôt"),
            Some(Hit::Publish) => MkClick::Soon("📤 Publier : ouvre ton indicateur dans l'éditeur Argos Script → 📤 Publier (bientôt dans la version Rust)"),
            Some(Hit::Refresh) => MkClick::Changed,
            Some(Hit::Cat(k)) => {
                (self.cat, self.view, self.scroll) = (k, 0, 0.0);
                MkClick::Changed
            }
            Some(Hit::Src(k)) => {
                self.src = k;
                MkClick::Changed
            }
            Some(Hit::Kind(k)) => {
                self.kind = k;
                MkClick::Changed
            }
            Some(Hit::Age(k)) => {
                self.age = k;
                MkClick::Changed
            }
            Some(Hit::Heart(k)) => {
                let ind = crate::catalogue::officiels()[k].0;
                match self.favs.iter().position(|i| *i == ind) {
                    Some(p) => _ = self.favs.remove(p),
                    None => self.favs.push(ind),
                }
                MkClick::Favs
            }
            Some(Hit::Add(k)) => MkClick::Toggle(crate::catalogue::officiels()[k].0),
            Some(Hit::Info(k)) => {
                self.info = Some(k);
                MkClick::Changed
            }
            _ => MkClick::None,
        }
    }

    /// Molette : la grille défile (`dy` > 0 : vers le bas).
    pub fn wheel(&mut self, dy: f64, w: f64, h: f64, on: &[Ind]) {
        let grid = grid_rect(w, h);
        let cols = (((grid.w + 14.0) / (card_w(w) + 14.0)).floor() as usize).max(1);
        let rows = self.shown(on).len().div_ceil(cols);
        let max = (rows as f64 * 250.0 - 14.0 - grid.h).max(0.0);
        self.scroll = (self.scroll + dy).clamp(0.0, max);
    }

    fn set_view(&mut self, v: usize) -> MkClick {
        self.view = if self.view == v { 0 } else { v };
        MkClick::Changed
    }

    pub fn type_char(&mut self, c: char) {
        if self.typing && !c.is_control() {
            self.query.push(c);
        }
    }

    pub fn backspace(&mut self) {
        if self.typing {
            self.query.pop();
        }
    }

    /// Échap : ferme les détails, puis la recherche, puis la page (`true` : fermer la page).
    pub fn escape(&mut self) -> bool {
        if self.info.take().is_some() {
            return false;
        }
        if self.typing {
            self.typing = false;
            return false;
        }
        true
    }

    /// Dessine la page ; `spark(ind)` : aperçu (série, couleur) calculé sur les bougies affichées.
    pub fn draw(&self, text: &Text, w: u32, h: u32, on: &[Ind], spark: &dyn Fn(Ind) -> (Vec<f64>, Rgb), mouse: (f64, f64)) -> Option<Pixmap> {
        let mut pm = crate::px::new(w, h)?;
        let (wf, hf) = (w as f64, h as f64);
        crate::fill(&mut pm, 0.0, 0.0, wf, hf, pal::backdrop(), 1.0);
        let z = self.layout(text, wf, hf, on);
        let hover = |r: &Rect| r.contains(mouse.0, mouse.1) && self.info.is_none();
        let inner = (wf - 40.0).min(1500.0);
        let x0 = (wf - inner) / 2.0;
        text.draw(&mut pm, "🛒 Argos", (x0 as f32, 40.0), 20.0, WHITE, Align::Left);
        text.draw(
            &mut pm,
            "Market",
            ((x0 + text.width("🛒 Argos", 20.0) as f64 + 4.0) as f32, 40.0),
            20.0,
            accent(),
            Align::Left,
        );
        let shown = self.shown(on);
        let phone = wf < PHONE;
        let grid = grid_rect(wf, hf);
        // filtres à gauche
        let side = Rect::new(x0, 112.0, 250.0, hf - 132.0);
        let boxes = [
            (side.y, 96.0, "Source"),
            (side.y + 108.0, 76.0, "Où ça s'affiche"),
            (side.y + 190.0, 76.0, "Publié"),
            (side.y + 278.0, 110.0, ""),
        ];
        for (y, bh, title) in boxes.into_iter().filter(|_| !phone) {
            let r = Rect::new(side.x, y, side.w, bh);
            round_fill(&mut pm, r, 10.0, pal::panel(), 1.0);
            round_stroke(&mut pm, r, 10.0, pal::edge(), 1.0);
            text.draw(&mut pm, title, ((side.x + 16.0) as f32, (y + 26.0) as f32), 13.5, WHITE, Align::Left);
        }
        if !phone {
            text.draw(
                &mut pm,
                &format!("{} officiels · 0 de la communauté", argos_script::catalogue::MARCHE.len()),
                ((side.x + 16.0) as f32, (side.y + 88.0) as f32),
                11.5,
                GREY,
                Align::Left,
            );
            let note = [
                "🛡️ Les indicateurs ne peuvent ni",
                "aller sur internet, ni lire tes clés,",
                "ni passer d'ordres. Chaque indicateur",
                "communautaire est signé par son auteur.",
            ];
            for (k, l) in note.iter().enumerate() {
                text.draw(
                    &mut pm,
                    l,
                    ((side.x + 16.0) as f32, (side.y + 304.0 + k as f64 * 20.0) as f32),
                    12.0,
                    GREY,
                    Align::Left,
                );
            }
        }
        let grid_x = grid.x;
        text.draw(
            &mut pm,
            &format!("{} indicateur{}", shown.len(), if shown.len() > 1 { "s" } else { "" }),
            (grid_x as f32, (grid.y - 18.0) as f32),
            13.5,
            GREY,
            Align::Left,
        );
        if shown.is_empty() {
            let msg = if self.src == 2 || CATS[self.cat].0 == "com" || self.age > 0 {
                "La communauté (indicateurs signés sur Nostr) arrive avec le compte Argos."
            } else {
                "Rien ne correspond. Essaie une autre catégorie ou enlève des filtres."
            };
            text.draw(
                &mut pm,
                &text.fit(msg, 14.0, grid.w),
                (grid_x as f32, (grid.y + 30.0) as f32),
                14.0,
                GREY,
                Align::Left,
            );
        }
        // cartes d'abord (leurs boutons par-dessus) ; le ♡ est en haut à droite de sa carte
        for (r, hit) in &z {
            if let Hit::Heart(k) = *hit {
                let cw = card_w(wf);
                let card = Rect::new(r.x + 36.0 - cw, r.y - 8.0, cw, 236.0);
                self.draw_card(&mut pm, text, card, k, spark, hover(&card));
            }
        }
        for (r, hit) in z.iter().rev() {
            let (r, hit) = (*r, *hit);
            match hit {
                Hit::Close => lines(&mut pm, r.cx(), r.cy(), &[&[(-7.0, -7.0), (7.0, 7.0)], &[(7.0, -7.0), (-7.0, 7.0)]], GREY, 1.8),
                Hit::Search => {
                    round_fill(&mut pm, r, 18.0, pal::field(), 1.0);
                    round_stroke(&mut pm, r, 18.0, if self.typing { accent() } else { pal::edge() }, 1.0);
                    let (s, c) = if self.query.is_empty() && !self.typing {
                        ("🔍 Chercher (RSI, or, SMC, scalping…)".to_string(), GREY)
                    } else {
                        (format!("🔍 {}{}", self.query, if self.typing { "|" } else { "" }), WHITE)
                    };
                    text.draw(
                        &mut pm,
                        &text.fit(&s, 13.5, r.w - 24.0),
                        ((r.x + 14.0) as f32, (r.cy() + 5.0) as f32),
                        13.5,
                        c,
                        Align::Left,
                    );
                }
                Hit::Mine | Hit::Fav | Hit::Me | Hit::Publish | Hit::Refresh => {
                    let on_ = matches!((hit, self.view), (Hit::Mine, 1) | (Hit::Fav, 2));
                    round_fill(
                        &mut pm,
                        r,
                        8.0,
                        if on_ {
                            accent()
                        } else if hover(&r) {
                            pal::hover()
                        } else {
                            pal::backdrop()
                        },
                        if on_ { 0.25 } else { 1.0 },
                    );
                    round_stroke(&mut pm, r, 8.0, pal::edge(), 1.0);
                    let label = match hit {
                        Hit::Mine => "📦 Mes indicateurs".to_string(),
                        Hit::Fav => format!("♡ Favoris {}", self.favs.len()),
                        Hit::Me => "👤 Ma page".into(),
                        Hit::Publish => "📤 Publier".into(),
                        _ => "🔄".into(),
                    };
                    text.draw(&mut pm, &label, (r.cx() as f32, (r.cy() + 5.0) as f32), 13.5, WHITE, Align::Center);
                }
                Hit::Cat(k) | Hit::Src(k) | Hit::Kind(k) | Hit::Age(k) => {
                    let (on_, label, rad) = match hit {
                        Hit::Cat(_) => (k == self.cat && self.view == 0, CATS[k].1, 16.0),
                        Hit::Src(_) => (k == self.src, ["Tout", "Officiel", "Communauté"][k], 6.0),
                        Hit::Kind(_) => (k == self.kind, ["Partout", "Sur le prix", "Panneau"][k], 6.0),
                        _ => (k == self.age, ["Tout", "7 jours", "30 jours"][k], 6.0),
                    };
                    round_fill(
                        &mut pm,
                        r,
                        rad,
                        if on_ {
                            accent()
                        } else if hover(&r) {
                            pal::hover()
                        } else {
                            pal::field()
                        },
                        if on_ { 0.3 } else { 1.0 },
                    );
                    round_stroke(&mut pm, r, rad, if on_ { accent() } else { pal::edge() }, 1.0);
                    text.draw(
                        &mut pm,
                        &text.fit(label, 12.5, r.w - 8.0),
                        (r.cx() as f32, (r.cy() + 4.5) as f32),
                        12.5,
                        WHITE,
                        Align::Center,
                    );
                }
                Hit::Heart(k) => {
                    let fav = self.favs.contains(&crate::catalogue::officiels()[k].0);
                    round_fill(&mut pm, r, 14.0, Rgb(0, 0, 0), 0.45);
                    text.draw(
                        &mut pm,
                        if fav { "♥" } else { "♡" },
                        (r.cx() as f32, (r.cy() + 6.0) as f32),
                        16.0,
                        if fav { Rgb(0xff, 0x52, 0x7a) } else { WHITE },
                        Align::Center,
                    );
                }
                Hit::Add(k) => {
                    let have = on.contains(&crate::catalogue::officiels()[k].0);
                    round_fill(
                        &mut pm,
                        r,
                        16.0,
                        if have {
                            crate::palette::up()
                        } else if hover(&r) {
                            pal::hover()
                        } else {
                            accent()
                        },
                        if have { 0.3 } else { 1.0 },
                    );
                    text.draw(
                        &mut pm,
                        if have { "✅ Ajouté" } else { "➕ Ajouter" },
                        (r.cx() as f32, (r.cy() + 5.0) as f32),
                        13.5,
                        WHITE,
                        Align::Center,
                    );
                }
                Hit::Info(_) => {
                    round_fill(&mut pm, r, 16.0, if hover(&r) { pal::hover() } else { pal::field() }, 1.0);
                    // ⓘ dessiné : un « i » dans un rond
                    round_stroke(&mut pm, Rect::new(r.cx() - 9.0, r.cy() - 9.0, 18.0, 18.0), 9.0, WHITE, 1.4);
                    text.draw(&mut pm, "i", (r.cx() as f32, (r.cy() + 4.5) as f32), 13.0, WHITE, Align::Center);
                }
                Hit::InfoClose | Hit::InfoAdd => {}
            }
        }
        if let Some(k) = self.info {
            self.draw_info(&mut pm, text, k, wf, hf, on, mouse);
        }
        Some(pm)
    }

    fn draw_card(&self, pm: &mut Pixmap, text: &Text, r: Rect, k: usize, spark: &dyn Fn(Ind) -> (Vec<f64>, Rgb), hover: bool) {
        let (ind, _, _, _) = crate::catalogue::officiels()[k];
        round_fill(pm, r, 12.0, pal::panel(), 1.0);
        round_stroke(pm, r, 12.0, if hover { accent() } else { pal::edge() }, 1.0);
        // aperçu : l'indicateur calculé sur les bougies affichées
        let img = Rect::new(r.x + 1.0, r.y + 1.0, r.w - 2.0, 110.0);
        crate::fill(pm, img.x, img.y + 10.0, img.w, img.h - 10.0, pal::field(), 1.0);
        round_fill(pm, Rect::new(img.x, img.y, img.w, 20.0), 11.0, pal::field(), 1.0);
        let (v, c) = spark(ind);
        let fin: Vec<f64> = v.iter().copied().filter(|x| x.is_finite()).collect();
        if fin.len() > 4 {
            let (lo, hi) = fin.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &x| (a.min(x), b.max(x)));
            let n = v.len().max(2);
            let pts: Vec<(f32, f32)> = v
                .iter()
                .enumerate()
                .filter(|(_, y)| y.is_finite())
                .map(|(i, y)| {
                    (
                        (img.x + 6.0 + i as f64 / (n - 1) as f64 * (img.w - 12.0)) as f32,
                        (img.y + img.h - 8.0 - (y - lo) / (hi - lo).max(1e-12) * (img.h - 22.0)) as f32,
                    )
                })
                .collect();
            let mut area = pts.clone();
            area.push((pts[pts.len() - 1].0, (img.y + img.h) as f32));
            area.push((pts[0].0, (img.y + img.h) as f32));
            crate::shapes::poly_fill(pm, 0.0, 0.0, &area, c, 0.18);
            lines(pm, 0.0, 0.0, &[&pts], c, 2.0);
        }
        // 🫧 Argos Bubbles draws from the real trades only: the preview has none, it says so (not an empty preview)
        if ind == Ind::Bubbles {
            text.draw(
                pm,
                &text.fit(BULLES_APERCU, 11.5, img.w - 16.0),
                ((img.x + 8.0) as f32, (img.y + 16.0) as f32),
                11.5,
                GREY,
                Align::Left,
            );
        }
        let badge = "⭐ Officiel";
        let bw = text.width(badge, 11.5) as f64 + 14.0;
        round_fill(pm, Rect::new(r.x + 8.0, r.y + 8.0, bw, 22.0), 6.0, GOLD, 0.9);
        text.draw(pm, badge, ((r.x + 15.0) as f32, (r.y + 23.0) as f32), 11.5, Rgb(0x1a, 0x14, 0x00), Align::Left);
        text.draw(
            pm,
            &text.fit(ind.name(), 14.5, r.w - 24.0),
            ((r.x + 12.0) as f32, (r.y + 136.0) as f32),
            14.5,
            WHITE,
            Align::Left,
        );
        let sub = format!("par Argos · {}", if ind.oscillator() { "panneau" } else { "sur le prix" });
        text.draw(pm, &sub, ((r.x + 12.0) as f32, (r.y + 158.0) as f32), 12.0, GREY, Align::Left);
    }

    fn draw_info(&self, pm: &mut Pixmap, text: &Text, k: usize, w: f64, h: f64, on: &[Ind], mouse: (f64, f64)) {
        crate::fill(pm, 0.0, 0.0, w, h, Rgb(0, 0, 0), 0.5);
        let r = info_rect(w, h);
        round_fill(pm, r, 12.0, pal::panel(), 1.0);
        round_stroke(pm, r, 12.0, pal::edge(), 1.0);
        let (ind, _, _, desc) = crate::catalogue::officiels()[k];
        text.draw(pm, ind.name(), ((r.x + 24.0) as f32, (r.y + 40.0) as f32), 19.0, WHITE, Align::Left);
        lines(
            pm,
            r.x + r.w - 27.0,
            r.y + 27.0,
            &[&[(-6.0, -6.0), (6.0, 6.0)], &[(6.0, -6.0), (-6.0, 6.0)]],
            GREY,
            1.6,
        );
        text.draw(
            pm,
            &format!("⭐ Officiel · par Argos · {}", if ind.oscillator() { "panneau" } else { "sur le prix" }),
            ((r.x + 24.0) as f32, (r.y + 66.0) as f32),
            12.5,
            GREY,
            Align::Left,
        );
        let mut line = String::new();
        let mut y = r.y + 104.0;
        for word in desc.split(' ') {
            let next = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
            if text.width(&next, 14.0) as f64 > r.w - 48.0 {
                text.draw(pm, &line, ((r.x + 24.0) as f32, y as f32), 14.0, WHITE, Align::Left);
                y += 22.0;
                line = word.to_string();
            } else {
                line = next;
            }
        }
        text.draw(pm, &line, ((r.x + 24.0) as f32, y as f32), 14.0, WHITE, Align::Left);
        let add = Rect::new(r.x + 24.0, r.y + r.h - 58.0, 160.0, 38.0);
        let have = on.contains(&ind);
        round_fill(
            pm,
            add,
            18.0,
            if have { crate::palette::up() } else { accent() },
            if add.contains(mouse.0, mouse.1) { 0.85 } else { 1.0 },
        );
        text.draw(
            pm,
            if have { "✅ Ajouté" } else { "➕ Ajouter" },
            (add.cx() as f32, (add.cy() + 5.0) as f32),
            14.0,
            WHITE,
            Align::Center,
        );
    }
}

/// 📱 Catégories de la page cell, passées à la ligne quand elles ne tiennent pas (aucune n'est enlevée faute de
/// place) ; et le bas de leur dernière rangée.
fn cats_cell(text: &Text, w: f64) -> (Vec<(Rect, Hit)>, f64) {
    let inner = (w - 40.0).min(1500.0);
    let x0 = (w - inner) / 2.0;
    let (mut cx, mut y) = (x0, 144.0);
    let mut z = vec![];
    for (k, (_, l)) in CATS.iter().enumerate() {
        let bw = text.width(l, 13.5) as f64 + 24.0;
        if cx + bw > x0 + inner && cx > x0 {
            (cx, y) = (x0, y + 40.0);
        }
        z.push((Rect::new(cx, y, bw, 32.0), Hit::Cat(k)));
        cx += bw + 8.0;
    }
    (z, y + 32.0)
}

/// Zone de la grille de cartes.
/// 📱 Sous cette largeur : la page en version cell (pas de filtres à gauche, 2 colonnes de cartes).
const PHONE: f64 = 700.0;

fn grid_rect(w: f64, h: f64) -> Rect {
    let inner = (w - 40.0).min(1500.0);
    let x0 = (w - inner) / 2.0;
    if w < PHONE {
        // (sous les catégories, passées à la ligne s'il le faut)
        let top = cats_cell(&Text::default(), w).1 + 38.0;
        return Rect::new(x0, top, inner, h - top - 10.0);
    }
    Rect::new(x0 + 266.0, 150.0, inner - 266.0, h - 170.0)
}

/// Largeur d'une carte (📱 cell : deux par rangée).
fn card_w(w: f64) -> f64 {
    if w < PHONE {
        (grid_rect(w, 800.0).w - 14.0) / 2.0
    } else {
        230.0
    }
}

fn info_rect(w: f64, h: f64) -> Rect {
    let (iw, ih) = (560.0f64.min(w - 24.0), 340.0f64.min(h - 24.0));
    Rect::new((w - iw) / 2.0, (h - ih) / 2.0, iw, ih)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marketplace() {
        let t = Text::default();
        let mut m = MkPage::new(vec![]);
        assert_eq!(m.shown(&[]).len(), argos_script::catalogue::MARCHE.len());
        // catégorie Oscillateurs
        let z = m.layout(&t, 1600.0, 900.0, &[]);
        let at = |z: &Vec<(Rect, Hit)>, h: Hit| z.iter().find(|(_, x)| *x == h).map(|(r, _)| (r.cx(), r.cy())).unwrap();
        let (x, y) = at(&z, Hit::Cat(3));
        m.click(&t, 1600.0, 900.0, &[], x, y);
        assert!(m.shown(&[]).iter().all(|&k| crate::catalogue::officiels()[k].1 == "osc"));
        // recherche « smc »
        m.cat = 0;
        "smc".chars().for_each(|c| {
            m.typing = true;
            m.type_char(c)
        });
        assert!(m
            .shown(&[])
            .iter()
            .all(|&k| matches!(crate::catalogue::officiels()[k].0, Ind::Smc | Ind::SmcPro)));
        m.query.clear();
        // ➕ Ajouter et ♡
        let z = m.layout(&t, 1600.0, 900.0, &[]);
        let (x, y) = at(&z, Hit::Add(0));
        assert_eq!(m.click(&t, 1600.0, 900.0, &[], x, y), MkClick::Toggle(Ind::Tvd));
        let (x, y) = at(&z, Hit::Heart(2));
        assert_eq!(m.click(&t, 1600.0, 900.0, &[], x, y), MkClick::Favs);
        assert_eq!(m.favs, vec![Ind::Cipher]);
        // communauté : vide pour l'instant
        m.src = 2;
        assert!(m.shown(&[]).is_empty());
        assert!(m
            .draw(&t, 1600, 900, &[], &|_| (vec![1.0, 2.0, 1.5, 3.0, 2.5, 4.0], accent()), (0.0, 0.0))
            .is_some());
    }

    /// 📱 Marketplace sur cell (390 et 600 de large) : les 10 catégories sont là (passées à la ligne), dans la page, sans
    /// se croiser, et la grille de cartes commence sous elles.
    #[test]
    fn categories_toujours_la_sur_cell() {
        let t = Text::default();
        let m = MkPage::new(vec![]);
        for w in [390.0, 600.0] {
            let z = m.layout(&t, w, 844.0, &[]);
            let cats: Vec<Rect> = z.iter().filter(|(_, h)| matches!(h, Hit::Cat(_))).map(|(r, _)| *r).collect();
            assert_eq!(cats.len(), CATS.len(), "{w} : toutes les catégories");
            for (i, a) in cats.iter().enumerate() {
                assert!(a.x >= 0.0 && a.x + a.w <= w, "{w} : dans la page");
                for b in cats.iter().skip(i + 1) {
                    assert!(
                        !(a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h),
                        "{w} : sans se croiser"
                    );
                }
            }
            let bas = cats.iter().map(|r| r.y + r.h).fold(0.0, f64::max);
            assert!(grid_rect(w, 844.0).y >= bas, "{w} : la grille sous les catégories");
        }
    }
}
