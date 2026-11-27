//! 🚩 Menus de la watchlist, comme Argos v1 :
//! - **ajout** (＋) : champ de recherche, filtres (Tout, un exchange, un groupe), résultats par groupe avec « déjà là »
//!   ou ＋ (clic = ajouter à la liste) ;
//! - **nom** : petite boîte pour nommer une nouvelle liste, une section, ou renommer la liste (Entrée = OK).
//!
//! Partie pure (état, clics, clavier : testée) + dessin dans leur propre image, collée par-dessus la fenêtre. Les
//! résultats viennent de la base (`argos-store`) : la fenêtre refait la recherche quand le texte ou le filtre change.

use crate::palette as pal;
use crate::palette::{GREY, MUTED, WHITE};
use crate::shapes::{lines, round_fill, round_stroke, Rect};
use crate::text::{Align, Rgb, Text};
use crate::watchlist::coin_badge;
use tiny_skia::Pixmap;

const PAD: f64 = 10.0;
const FIELD_H: f64 = 36.0;
const CHIP_H: f64 = 28.0;
const ROW_H: f64 = 38.0;
const HEAD_H: f64 = 30.0;

/// Un filtre (puce) : sa clé (« », « ex:hl », « grp:Crypto ») et ce qui est écrit dessus.
#[derive(Debug, Clone, PartialEq)]
pub struct Chip {
    pub key: String,
    pub label: String,
    /// Pastille de couleur avant le nom (exchanges, comme v1).
    pub dot: Option<Rgb>,
}

/// Un symbole trouvé.
#[derive(Debug, Clone, PartialEq)]
pub struct Found {
    /// Identifiant `exchange:symbole`.
    pub id: String,
    /// Symbole tel que l'exchange l'écrit.
    pub name: String,
    /// En gris à droite du nom (exchange, inactif…).
    pub detail: String,
    /// Groupe (en-tête de la liste des résultats).
    pub group: String,
    /// Déjà dans la liste (ou plus listé) : pas de ＋.
    pub have: bool,
}

/// Résultat d'un clic.
#[derive(Debug, Clone, PartialEq)]
pub enum AddClick {
    /// À côté : fermer.
    Outside,
    /// Dans le panneau, rien de spécial.
    Inside,
    /// Le filtre a changé : refaire la recherche.
    Filter,
    /// Ajouter ce symbole.
    Add(String),
}

/// Champ de texte : le texte et une barre de saisie au bout.
fn field(pm: &mut Pixmap, text: &Text, r: Rect, value: &str, placeholder: &str) {
    round_fill(pm, r, 6.0, pal::field(), 1.0);
    round_stroke(pm, r, 6.0, crate::palette::accent(), 1.2);
    let ty = (r.cy() + 5.0) as f32;
    if value.is_empty() {
        text.draw(pm, placeholder, ((r.x + 34.0) as f32, ty), 14.0, MUTED, Align::Left);
    } else {
        text.draw(pm, value, ((r.x + 34.0) as f32, ty), 14.0, WHITE, Align::Left);
    }
    // loupe, puis la barre de saisie
    let ring = crate::shapes::arc(5.5, 0.0, 360.0, 20);
    lines(pm, r.x + 15.0, r.cy() - 1.0, &[&ring, &[(4.0, 4.0), (8.0, 8.0)]], GREY, 1.4);
    let cx = r.x + 34.0 + text.width(value, 14.0) as f64 + 1.0;
    crate::fill(pm, cx, r.cy() - 9.0, 1.5, 18.0, WHITE, 1.0);
}

/// Panneau d'ajout de symboles.
#[derive(Debug, Clone, PartialEq)]
pub struct AddPanel {
    /// Où il est (coordonnées de la fenêtre).
    pub rect: Rect,
    pub query: String,
    pub chips: Vec<Chip>,
    /// Filtre choisi (index dans `chips`).
    pub chip: usize,
    pub results: Vec<Found>,
    scroll: f64,
}

/// Ligne affichée dans les résultats.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Line {
    Head(usize),
    Row(usize),
}

impl AddPanel {
    /// Panneau à cet endroit (coordonnées de la fenêtre) : comme Argos v1, il prend la place de la liste, sous
    /// l'en-tête de la watchlist.
    pub fn new(rect: Rect, chips: Vec<Chip>) -> Self {
        Self {
            rect,
            query: String::new(),
            chips,
            chip: 0,
            results: vec![],
            scroll: 0.0,
        }
    }

    /// Clé du filtre choisi.
    pub fn filter(&self) -> &str {
        self.chips.get(self.chip).map_or("", |c| c.key.as_str())
    }

    pub fn type_char(&mut self, c: char) {
        if !c.is_control() && self.query.chars().count() < 40 {
            self.query.push(c);
            self.scroll = 0.0;
        }
    }

    pub fn backspace(&mut self) {
        self.query.pop();
        self.scroll = 0.0;
    }

    /// Nouveaux résultats (après une recherche).
    pub fn set_results(&mut self, r: Vec<Found>) {
        self.results = r;
        self.scroll = self.scroll.min(self.max_scroll());
    }

    /// Premier symbole qu'on peut ajouter (Entrée).
    pub fn first(&self) -> Option<String> {
        self.results.iter().find(|f| !f.have).map(|f| f.id.clone())
    }

    /// Le champ de recherche (fenêtre) : 📱 y toucher ouvre le clavier du cell.
    pub fn field_rect(&self) -> Rect {
        Rect::new(self.rect.x + PAD, self.rect.y + PAD, self.rect.w - 2.0 * PAD, FIELD_H)
    }

    /// Puces (coordonnées de la fenêtre), sur autant de rangées que nécessaire.
    fn chip_rects(&self, text: &Text) -> Vec<Rect> {
        let (left, right) = (self.rect.x + PAD, self.rect.x + self.rect.w - PAD);
        let (mut x, mut y) = (left, self.rect.y + PAD + FIELD_H + 8.0);
        self.chips
            .iter()
            .map(|c| {
                let w = text.width(&c.label, 12.5) as f64 + 20.0 + if c.dot.is_some() { 14.0 } else { 0.0 };
                if x + w > right && x > left {
                    (x, y) = (left, y + CHIP_H + 6.0);
                }
                let r = Rect::new(x, y, w, CHIP_H);
                x += w + 6.0;
                r
            })
            .collect()
    }

    /// Haut de la liste des résultats.
    fn list_top(&self, text: &Text) -> f64 {
        self.chip_rects(text).last().map_or(self.rect.y + PAD + FIELD_H, |r| r.y + r.h) + 10.0
    }

    /// Bottom of the results' room (window): the card's padding under it stays empty (B-017, Jee 25 Nov 2026 on the
    /// phone: the last result was cut by the card's edge and ran over the list behind it).
    fn list_bottom(&self) -> f64 {
        self.rect.y + self.rect.h - PAD
    }

    fn lines(&self) -> Vec<Line> {
        let mut out = vec![];
        for (i, f) in self.results.iter().enumerate() {
            if i == 0 || self.results[i - 1].group != f.group {
                out.push(Line::Head(i));
            }
            out.push(Line::Row(i));
        }
        out
    }

    fn line_h(l: Line) -> f64 {
        if matches!(l, Line::Head(_)) {
            HEAD_H
        } else {
            ROW_H
        }
    }

    fn content_h(&self) -> f64 {
        self.lines().into_iter().map(Self::line_h).sum()
    }

    fn max_scroll(&self) -> f64 {
        // the results' room: under the chips, as many rows of them as the width needs (7 on a phone: B-017, 3 were
        // assumed and the last results could not be reached), and above the card's bottom padding (the app draws
        // with the default text: the same chip rows as on the screen)
        let room = self.list_bottom() - self.list_top(&Text::default());
        (self.content_h() - room.max(0.0)).max(0.0)
    }

    pub fn wheel(&mut self, steps: f64) {
        self.scroll = (self.scroll - steps * ROW_H).clamp(0.0, self.max_scroll());
    }

    /// Lignes visibles avec leur haut (coordonnées de la fenêtre).
    fn placed(&self, text: &Text) -> Vec<(f64, Line)> {
        let mut y = self.list_top(text) - self.scroll;
        self.lines()
            .into_iter()
            .map(|l| {
                let top = y;
                y += Self::line_h(l);
                (top, l)
            })
            .collect()
    }

    /// Clic (coordonnées de la fenêtre).
    pub fn click(&mut self, x: f64, y: f64, text: &Text) -> AddClick {
        if !self.rect.contains(x, y) {
            return AddClick::Outside;
        }
        if let Some(i) = self.chip_rects(text).iter().position(|r| r.contains(x, y)) {
            if i != self.chip {
                self.chip = i;
                self.scroll = 0.0;
                return AddClick::Filter;
            }
            return AddClick::Inside;
        }
        let top = self.list_top(text);
        if y < top || y >= self.list_bottom() {
            // (under the results' room: the card's padding, no result there)
            return AddClick::Inside;
        }
        match self.placed(text).into_iter().find(|(t, l)| y >= *t && y < t + Self::line_h(*l)) {
            Some((_, Line::Row(i))) if !self.results[i].have => AddClick::Add(self.results[i].id.clone()),
            _ => AddClick::Inside,
        }
    }

    /// Dessin : l'image et où la coller (coordonnées de la fenêtre).
    pub fn draw(&self, text: &Text, mouse: (f64, f64), logos: &std::collections::HashMap<String, Pixmap>) -> Option<(Pixmap, Rect)> {
        let r = self.rect;
        let mut pm = crate::px::new(r.w as u32, r.h as u32)?;
        let at = |q: Rect| Rect::new(q.x - r.x, q.y - r.y, q.w, q.h);
        crate::shapes::menu_card(&mut pm, at(r));
        // (the bare card, laid again under the results' room: B-017)
        let card = pm.clone();
        let local = (mouse.0 - r.x, mouse.1 - r.y);
        let top = self.list_top(text) - r.y;
        let bottom = self.list_bottom() - r.y;
        // résultats (d'abord ; le champ et les puces passent par-dessus)
        for (t, l) in self.placed(text) {
            let t = t - r.y;
            if t + Self::line_h(l) < top || t >= bottom {
                continue;
            }
            match l {
                Line::Head(i) => {
                    let g = &self.results[i].group;
                    let n = self.results.iter().filter(|f| f.group == *g).count();
                    text.draw(
                        &mut pm,
                        &format!("{}  {n}", g.to_uppercase()),
                        (14.0, (t + 20.0) as f32),
                        11.5,
                        GREY,
                        Align::Left,
                    );
                }
                Line::Row(i) => {
                    let f = &self.results[i];
                    let row = Rect::new(4.0, t + 1.0, r.w - 8.0, ROW_H - 2.0);
                    crate::shapes::menu_row(&mut pm, row, false, !f.have && row.contains(local.0, local.1));
                    let base = f.id.split_once(':').map_or(f.id.as_str(), |(_, s)| s);
                    coin_badge(&mut pm, text, base, logos.get(&f.id), 26.0, t + ROW_H / 2.0, 10.0);
                    let col = if f.have { GREY } else { WHITE };
                    text.draw(&mut pm, &f.name, (44.0, (t + 24.0) as f32), 14.0, col, Align::Left);
                    let dx = 44.0 + text.width(&f.name, 14.0) as f64 + 10.0;
                    text.draw(&mut pm, &f.detail, (dx as f32, (t + 24.0) as f32), 12.0, GREY, Align::Left);
                    let (mark, mc) = if f.have { ("déjà là", GREY) } else { ("+", WHITE) };
                    text.draw(
                        &mut pm,
                        mark,
                        ((r.w - 16.0) as f32, (t + 24.0) as f32),
                        if f.have { 12.0 } else { 16.0 },
                        mc,
                        Align::Right,
                    );
                }
            }
        }
        if self.results.is_empty() {
            let msg = if self.query.is_empty() {
                "Aucun symbole : catalogue en chargement…".to_string()
            } else {
                format!("Aucun marché ne correspond à « {} ».", self.query)
            };
            text.draw(&mut pm, &msg, ((r.w / 2.0) as f32, (top + 30.0) as f32), 13.0, GREY, Align::Center);
        }
        // B-017: nothing under the results' room: the card's bottom (its padding, its round corners) as it was, a
        // result half way out cut at the room's edge
        let s = f64::from(crate::px::scale());
        let w4 = pm.width() as usize * 4;
        let y0 = ((bottom * s).floor().max(0.0) as usize).min(pm.height() as usize);
        pm.data_mut()[y0 * w4..].copy_from_slice(&card.data()[y0 * w4..]);
        // haut : fond, champ, puces
        crate::fill(&mut pm, 1.0, 1.0, r.w - 2.0, top - 4.0, pal::panel(), 1.0);
        field(&mut pm, text, at(self.field_rect()), &self.query, "Symbole, nom… (ex. BTC, NVDA)");
        for (i, (c, cr)) in self.chips.iter().zip(self.chip_rects(text)).enumerate() {
            let cr = at(cr);
            let on = i == self.chip;
            let hover = cr.contains(local.0, local.1);
            // puces comme les catégories du Marketplace : choisie = bleu Argos teinté, contour bleu
            round_fill(
                &mut pm,
                cr,
                CHIP_H / 2.0,
                if on {
                    crate::palette::accent()
                } else if hover {
                    pal::hover()
                } else {
                    pal::field()
                },
                if on { 0.3 } else { 1.0 },
            );
            round_stroke(&mut pm, cr, CHIP_H / 2.0, if on { crate::palette::accent() } else { pal::edge_soft() }, 1.0);
            let tc = WHITE;
            let shift = if let Some(d) = c.dot {
                round_fill(&mut pm, Rect::new(cr.x + 10.0, cr.cy() - 5.0, 10.0, 10.0), 5.0, d, 1.0);
                7.0
            } else {
                0.0
            };
            text.draw(&mut pm, &c.label, ((cr.cx() + shift) as f32, (cr.cy() + 4.5) as f32), 12.5, tc, Align::Center);
        }
        crate::fill(&mut pm, 8.0, top - 4.0, r.w - 16.0, 1.0, pal::edge_soft(), 1.0);
        Some((pm, r))
    }
}

/// Ce que la boîte de nom va nommer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameFor {
    NewList,
    Section,
    /// 📱 Renommer une section (son nom d'avant : `UiState.section`).
    RenameSection,
    Rename,
    /// Mises en page ▸ Renommer.
    Layout,
    /// 🌐 Chercher une langue.
    Lang,
    /// 💼 Renommer le compte démo.
    Account,
    /// 🧠 Ajouter une note à la mémoire de l'assistant.
    AiNote,
    /// 🧠 Méthode / règles perso envoyées à l'assistant.
    AiMethod,
    /// 👛 Adresse publique Hyperliquid (wallet en lecture seule).
    Wallet,
    /// 💡 Idée de trade (Communauté).
    Idea,
    /// ⚙ Valeur d'un réglage « nombre » d'un module.
    ModNumber,
    /// 🔔 Valeur(s) d'une alerte sur un indicateur (« 70 », « 30 70 »).
    AlertValue,
    /// ⏱ Intervalle personnalisé (« 90m », « 2h », « 10D »).
    Interval,
    /// 🔓 Raison pour déverrouiller la journée (Gardien).
    Raison,
    /// 🖍 Name of a drawing template (« Sauvegarder le modèle de dessin sous… »).
    DrawingTemplate,
    /// 🎫 A field of the Trading ticket (price, quantity, value, %, TP, SL).
    Ticket(crate::panels::ticket::Champ),
}

/// Boîte pour un nom (nouvelle liste, section, renommer).
#[derive(Debug, Clone, PartialEq)]
pub struct NameBox {
    pub rect: Rect,
    pub kind: NameFor,
    pub value: String,
}

impl NameBox {
    pub fn new(kind: NameFor, value: &str, anchor: Rect, win_w: f64) -> Self {
        let w = 320.0f64.min(win_w - 12.0);
        let x = anchor.x.clamp(6.0, (win_w - w - 6.0).max(6.0));
        Self {
            rect: Rect::new(x, anchor.y + anchor.h + 6.0, w, 2.0 * FIELD_H + 3.0 * PAD + 20.0),
            kind,
            value: value.into(),
        }
    }

    fn title(&self) -> &'static str {
        match self.kind {
            NameFor::NewList => "Nouvelle liste",
            NameFor::Section => "Ajouter une section",
            NameFor::Rename => "Renommer la liste",
            NameFor::RenameSection => "Renommer la section",
            NameFor::Layout => "Renommer la mise en page",
            NameFor::Lang => "Chercher une langue",
            NameFor::Account => "Renommer le compte démo",
            NameFor::AiNote => "🧠 Nouvelle note pour l'assistant",
            NameFor::AiMethod => "🧠 Ta méthode / tes règles",
            NameFor::Wallet => "👛 Ton adresse publique Hyperliquid",
            NameFor::Idea => "💡 Ton idée : sens, entrée, sl, tp, texte",
            NameFor::ModNumber => "⚙ Nouvelle valeur",
            NameFor::AlertValue => "🔔 Valeur de l'alerte (une zone : bas haut)",
            NameFor::Interval => "⏱️ Intervalle personnalisé (nombre + unité)",
            NameFor::Raison => "Pourquoi déverrouiller ? (obligatoire, noté au Journal)",
            NameFor::DrawingTemplate => "Sauvegarder le modèle de dessin sous…",
            NameFor::Ticket(c) => c.titre(),
        }
    }

    fn placeholder(&self) -> &'static str {
        match self.kind {
            NameFor::Section => "Nom de la section (ex. ISO 20022)",
            NameFor::Lang => "ex. Deutsch, polonais, Greek",
            NameFor::AiNote => "ex. Mes setups partent du 4H",
            NameFor::AiMethod => "ex. SMC du H4 au M15, risque 0,5 % par trade",
            NameFor::Wallet => "0x… (publique, jamais ta clé privée)",
            NameFor::Idea => "ex. long 76000 sl 75000 tp 80000 cassure",
            NameFor::ModNumber => "ex. 20",
            NameFor::AlertValue => "ex. 70   ou   30 70",
            NameFor::Interval => "ex. 90m, 2h, 10D, 3W, 2M, 45s",
            NameFor::Raison => "ex. erreur de saisie, je reprends demain",
            NameFor::DrawingTemplate => "ex. Mon rayon or",
            NameFor::Ticket(c) => c.exemple(),
            _ => "Nom de la liste",
        }
    }

    fn field_rect(&self) -> Rect {
        Rect::new(self.rect.x + PAD, self.rect.y + PAD + 22.0, self.rect.w - 2.0 * PAD, FIELD_H)
    }

    fn ok_rect(&self) -> Rect {
        Rect::new(self.rect.x + self.rect.w - PAD - 70.0, self.rect.y + self.rect.h - PAD - 30.0, 70.0, 30.0)
    }

    pub fn type_char(&mut self, c: char) {
        let max = match self.kind {
            NameFor::AiNote | NameFor::AiMethod | NameFor::Idea => 300,
            NameFor::Wallet => 42,
            NameFor::Raison => crate::journal::RAISON_MAX,
            _ => 40,
        };
        if !c.is_control() && self.value.chars().count() < max {
            self.value.push(c);
        }
    }

    pub fn backspace(&mut self) {
        self.value.pop();
    }

    /// Le nom (sans espaces autour) ; `None` s'il est vide.
    pub fn name(&self) -> Option<String> {
        Some(self.value.trim().to_string()).filter(|v| !v.is_empty())
    }

    /// Clic : `Some(true)` = OK, `Some(false)` = à côté (fermer), `None` = dans la boîte.
    pub fn click(&self, x: f64, y: f64) -> Option<bool> {
        if self.ok_rect().contains(x, y) {
            return Some(true);
        }
        (!self.rect.contains(x, y)).then_some(false)
    }

    pub fn draw(&self, text: &Text, _mouse: (f64, f64)) -> Option<(Pixmap, Rect)> {
        let r = self.rect;
        let mut pm = crate::px::new(r.w as u32, r.h as u32)?;
        let at = |q: Rect| Rect::new(q.x - r.x, q.y - r.y, q.w, q.h);
        crate::shapes::menu_card(&mut pm, at(r));
        text.draw(&mut pm, self.title(), (PAD as f32, (PAD + 12.0) as f32), 12.5, GREY, Align::Left);
        field(&mut pm, text, at(self.field_rect()), &self.value, self.placeholder());
        let ok = at(self.ok_rect());
        round_fill(&mut pm, ok, 6.0, crate::palette::accent(), 1.0);
        text.draw(&mut pm, "OK", (ok.cx() as f32, (ok.cy() + 5.0) as f32), 14.0, WHITE, Align::Center);
        text.draw(
            &mut pm,
            // (📱 phone view: keyboard only, left out)
            &crate::actions::sans_touches("Entrée = OK · Échap = annuler"),
            (PAD as f32, (ok.cy() + 4.0) as f32),
            11.5,
            MUTED,
            Align::Left,
        );
        Some((pm, r))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chips() -> Vec<Chip> {
        [("", "Tout"), ("ex:hl", "Hyperliquid"), ("ex:binance", "Binance"), ("grp:Crypto", "Crypto")]
            .map(|(k, l)| Chip {
                key: k.into(),
                label: l.into(),
                dot: None,
            })
            .to_vec()
    }

    fn found(id: &str, group: &str, have: bool) -> Found {
        Found {
            id: id.into(),
            name: id.split_once(':').unwrap().1.into(),
            detail: String::new(),
            group: group.into(),
            have,
        }
    }

    #[test]
    fn recherche_filtres_et_ajout() {
        let t = Text::default();
        let mut p = AddPanel::new(Rect::new(1000.0, 44.0, 340.0, 700.0), chips());
        for c in "btc".chars() {
            p.type_char(c);
        }
        p.backspace();
        assert_eq!(p.query, "bt");
        p.set_results(vec![
            found("hl:BTC", "Crypto", true),
            found("binance:BTCUSDT", "Crypto", false),
            found("hl:xyz:BTCX", "HIP-3 xyz", false),
        ]);
        assert_eq!(p.first().as_deref(), Some("binance:BTCUSDT"), "Entrée : le premier qu'on peut ajouter");
        // puce Binance
        let cr = p.chip_rects(&t)[2];
        assert_eq!(p.click(cr.cx(), cr.cy(), &t), AddClick::Filter);
        assert_eq!(p.filter(), "ex:binance");
        assert_eq!(p.click(cr.cx(), cr.cy(), &t), AddClick::Inside, "déjà choisi");
        // lignes : en-tête Crypto, BTC (déjà là), BTCUSDT
        let top = p.list_top(&t);
        assert_eq!(p.click(p.rect.x + 100.0, top + HEAD_H + 5.0, &t), AddClick::Inside, "déjà là : pas d'ajout");
        assert_eq!(
            p.click(p.rect.x + 100.0, top + HEAD_H + ROW_H + 5.0, &t),
            AddClick::Add("binance:BTCUSDT".into())
        );
        assert_eq!(p.click(1.0, 1.0, &t), AddClick::Outside);
        assert!(p.draw(&t, (0.0, 0.0), &Default::default()).is_some());
    }

    #[test]
    fn boite_de_nom() {
        let mut b = NameBox::new(NameFor::Rename, "Ma liste", Rect::new(1000.0, 8.0, 230.0, 30.0), 1400.0);
        b.backspace();
        b.type_char('X');
        assert_eq!(b.name().as_deref(), Some("Ma listX"));
        let ok = b.ok_rect();
        assert_eq!(b.click(ok.cx(), ok.cy()), Some(true));
        assert_eq!(b.click(b.rect.cx(), b.rect.y + 5.0), None);
        assert_eq!(b.click(0.0, 0.0), Some(false));
        b.value = "   ".into();
        assert_eq!(b.name(), None, "vide : rien");
        assert!(b.draw(&Text::default(), (0.0, 0.0)).is_some());
    }

    /// 🔎 B-017 (Jee, 25 Nov 2026, IMG_3900): on the phone the search panel's chips take 7 rows; scrolled to the end,
    /// the last result lies whole inside the card, nothing is drawn under the results' room, and a tap there adds
    /// nothing.
    #[test]
    fn the_phone_search_keeps_its_last_result_inside_the_card() {
        let t = Text::default();
        // the phone's chips (every platform, then the groups), on a 402 points wide sheet
        let names = [
            "Tout",
            "Hyperliquid",
            "OKX",
            "Binance",
            "Bybit",
            "Bitstamp",
            "WEEX",
            "Bitunix",
            "BloFin",
            "Bitget",
            "Gate.io",
            "MEXC",
            "KuCoin",
            "Coinbase",
            "Kraken",
            "Binance Spot",
            "OKX Spot",
            "Bybit Spot",
            "Bitget Spot",
            "Aster",
            "Bitfinex",
            "Bitfinex Spot",
            "Crypto",
            "Spot",
            "TradFi",
            "HIP-3 xyz",
            "Stocks",
            "HIP-3 para",
            "HIP-3 io",
            "Indices",
            "Commodities",
            "HIP-3 mkts",
        ];
        let chips = names
            .iter()
            .enumerate()
            .map(|(i, l)| Chip {
                key: format!("k{i}"),
                label: (*l).into(),
                dot: (i > 0 && i < 22).then_some(Rgb(200, 160, 40)),
            })
            .collect();
        let mut p = AddPanel::new(Rect::new(0.0, 97.0, 402.0, 371.0), chips);
        let rows = {
            let mut y: Vec<u64> = p.chip_rects(&t).iter().map(|r| r.y as u64).collect();
            y.dedup();
            y.len()
        };
        assert!(rows > 3, "the phone's chips take more than 3 rows: {rows}");
        p.set_results((0..40).map(|i| found(&format!("binance:C{i}USDT"), "Crypto", false)).collect());
        p.wheel(-1000.0);
        let (top, bottom) = (p.list_top(&t), p.list_bottom());
        let last = p.placed(&t).last().copied().expect("results");
        assert!(
            last.0 + ROW_H <= bottom + 1e-9 && last.0 + ROW_H > bottom - ROW_H,
            "scrolled to the end, the last result is whole, against the room's bottom: {} in [{top}, {bottom}]",
            last.0
        );
        assert_eq!(p.click(200.0, bottom + 2.0, &t), AddClick::Inside, "under the room: no result there");
        assert_eq!(p.click(200.0, last.0 + ROW_H / 2.0, &t), AddClick::Add("binance:C39USDT".into()));
        // nothing drawn under the room: the bare card there, pixel for pixel (scrolled half a row: a result is cut)
        p.wheel(0.5);
        let (pm, r) = p.draw(&t, (200.0, bottom - 5.0), &Default::default()).unwrap();
        let mut card = crate::px::new(r.w as u32, r.h as u32).unwrap();
        crate::shapes::menu_card(&mut card, Rect::new(0.0, 0.0, r.w, r.h));
        let s = f64::from(crate::px::scale());
        let w4 = pm.width() as usize * 4;
        let y0 = ((bottom - r.y) * s).ceil() as usize;
        assert!(pm.data()[y0 * w4..] == card.data()[y0 * w4..], "something drawn under the results' room");
        // and the results are drawn above it
        let y1 = ((bottom - r.y - ROW_H / 2.0) * s) as usize;
        assert!(pm.data()[y1 * w4..y0 * w4] != card.data()[y1 * w4..y0 * w4], "the results are drawn");
    }
}
