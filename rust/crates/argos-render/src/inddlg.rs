//! ⚙️ Fenêtre de réglages d'un indicateur, comme Argos v1 et les inputs de Pine Script v6 : un onglet par groupe
//! (`group`), une ligne par input — ou plusieurs inputs sur la même ligne (`inline`) —, un « ? » qui montre l'aide
//! (`tooltip`), les inputs grisés quand leur case est décochée (`active`). Sortes d'inputs : nombre (− / + et saisie),
//! case à cocher, liste (‹ ›), couleur (palette et transparence), prix, heure, session, texte (une ou plusieurs
//! lignes), symbole, source (un prix ou la sortie d'un autre indicateur de la chart).
//! Les onglets passent à la ligne s'ils sont nombreux ; la liste défile à la molette. Les changements se voient en
//! direct sur la chart ; Annuler remet les valeurs d'avant. « Défauts » ouvre un petit menu, comme TradingView :
//! « Réinitialiser les réglages » (les défauts enregistrés, sinon ceux de l'indicateur), « Enregistrer comme défaut »
//! (repris chaque fois que l'indicateur est ajouté) et, s'il y a des défauts enregistrés, « Revenir aux réglages
//! d'origine ».
//! La table des inputs vient de l'indicateur (`Ind::params`) : rien n'est écrit en double ici.

use crate::indicators::Ind;
use crate::palette as pal;
use crate::palette::{accent, GREY, WHITE};
use crate::shapes::{lines, round_fill, round_stroke, Rect};
use crate::text::{Align, Rgb, Text};
use argos_script::inputs::{clamp, defaults, session_txt, session_val, Inputs, Kind, Param, Src, SOURCES};
use std::collections::HashMap;
use tiny_skia::Pixmap;

mod cell;

const W: f64 = 640.0;
const H: f64 = 640.0;
const PAD: f64 = 24.0;
const ROW: f64 = 42.0;
const TAB_H: f64 = 32.0;
const TAB_FONT: f32 = 13.0;
/// Haut des onglets (sous le titre).
const TABS_Y: f64 = 60.0;
/// Bas de la liste (au-dessus des boutons Défauts / Annuler / OK).
const FOOT: f64 = 72.0;
const GAP: f64 = 8.0;
/// Pas des transparences proposées sous la palette (Pine : 0 à 100).
const TRANSP: [u32; 6] = [0, 20, 40, 60, 80, 90];

/// Palette des couleurs (comme celle de TradingView) : 10 colonnes.
const PALETTE: [u32; 40] = [
    0xffffff, 0xd1d4dc, 0xb2b5be, 0x9598a1, 0x787b86, 0x5d606b, 0x434651, 0x2a2e39, 0x131722, 0x000000, //
    0xf23645, 0xff9800, 0xffeb3b, 0x4caf50, 0x089981, 0x00bcd4, 0x2962ff, 0x673ab7, 0x9c27b0, 0xe91e63, //
    0xff5252, 0xffb74d, 0xfff176, 0x81c784, 0x42bda8, 0x4dd0e1, 0x5b9cf6, 0x9575cd, 0xba68c8, 0xf06292, //
    0xe60000, 0x3ee145, 0xc33ee1, 0x4994ec, 0x1f1559, 0x00e676, 0x21baf3, 0xffe500, 0x3fff00, 0xc000ff,
];

/// Ce qu'un clic (ou Entrée) demande.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IndClick {
    None,
    /// Une valeur a changé : recalculer l'indicateur.
    Changed,
    /// Fermer : `true` = OK (garder), `false` = Annuler (remettre les valeurs d'avant).
    Close(bool),
    /// 📍 Choisir ce prix / cette heure sur la chart (la fenêtre se cache le temps du clic).
    Here(usize),
    /// Défauts ▸ « Enregistrer comme défaut » : garder `vals` et `texts` comme réglages de départ de l'indicateur.
    SaveDefaults,
}

/// Lignes du menu « Défauts » : (zone, texte).
const MENU_DEFAUTS: [(Hit, &str); 3] = [
    (Hit::Reset, "Réinitialiser les réglages"),
    (Hit::SaveDef, "Enregistrer comme défaut"),
    (Hit::Origine, "Revenir aux réglages d'origine"),
];

/// Zone cliquable (coordonnées de la fenêtre de l'app).
#[derive(Debug, Clone, Copy, PartialEq)]
enum Hit {
    Close,
    Tab(usize),
    Minus(usize),
    Plus(usize),
    Field(usize),
    Check(usize),
    Swatch(usize),
    /// « ? » : aide de l'input (montrée au survol).
    Tip(usize),
    /// 📍 au bout d'un champ prix / heure : choisir sur la chart.
    Here(usize),
    /// Couleur de la palette ouverte, puis une transparence.
    Pick(u32),
    Transp(u32),
    /// Bouton « Défauts » (📱 « ⋯ ») : ouvre / ferme son menu.
    Defaults,
    /// Lignes du menu « Défauts ».
    Reset,
    SaveDef,
    Origine,
    Cancel,
    Ok,
    /// 📱 Une option de la liste ouverte : (input, rang de l'option).
    Opt(usize, usize),
}

/// La fenêtre ouverte.
#[derive(Debug, Clone, PartialEq)]
pub struct IndDialog {
    pub ind: Ind,
    /// ⚙ A user script's window (step 3): its id and its inputs (then `ind` is not used).
    pub script: Option<(String, Vec<argos_script::lang::InputSpec>)>,
    /// Nom, onglets et table des inputs (ceux de l'indicateur).
    title: String,
    groups: &'static [&'static str],
    params: &'static [Param],
    /// Valeurs en cours (dans l'ordre de la table).
    pub vals: Vec<f64>,
    /// Valeurs à l'ouverture (Annuler).
    pub backup: Vec<f64>,
    /// Textes des inputs texte et symbole (par clé), et ceux de l'ouverture.
    pub texts: HashMap<String, String>,
    pub texts_backup: HashMap<String, String>,
    /// Sorties des autres indicateurs de la chart qu'une source peut prendre : (nom, valeur de l'input).
    sources: Vec<(String, f64)>,
    pub tab: usize,
    pub rect: Rect,
    /// Input en cours de saisie et son texte.
    edit: Option<(usize, String)>,
    /// Palette ouverte pour cet input (couleur).
    palette: Option<usize>,
    /// Défilement de la liste (pixels).
    scroll: f64,
    /// « Confirmer les inputs » (Pine `confirm = true`) : seulement ces inputs, sans onglets.
    pub confirm: bool,
    /// 📱 Feuille plein écran du cell et de la tablette (modèle de Jee) ; liste (choix, source) ouverte.
    pub cell: bool,
    liste: Option<usize>,
    /// Menu « Défauts » ouvert.
    menu_defauts: bool,
    /// Défauts enregistrés par l'utilisateur pour cet indicateur (valeurs, textes), s'il y en a.
    pub mes_defauts: Option<(Vec<f64>, HashMap<String, String>)>,
    /// « Enregistrer comme défaut » proposé (indicateurs de l'app ; pas encore les scripts).
    pub enregistrable: bool,
}

/// Nombre écrit sans zéros inutiles (« 2 », « 2.5 »).
fn show(v: f64) -> String {
    let s = format!("{v:.6}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

fn rgb(c: f64) -> Rgb {
    let c = c as u32;
    Rgb((c >> 16) as u8, (c >> 8) as u8, c as u8)
}

/// Heure (ms, UTC) ↔ « AAAA-MM-JJ HH:MM ».
fn time_txt(ms: f64) -> String {
    use chrono::{Datelike, Timelike};
    chrono::DateTime::from_timestamp((ms / 1000.0) as i64, 0).map_or(String::new(), |d| {
        format!("{}-{:02}-{:02} {:02}:{:02}", d.year(), d.month(), d.day(), d.hour(), d.minute())
    })
}

fn parse_time(s: &str) -> Option<f64> {
    let s = s.trim();
    let dt = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M")
        .or_else(|_| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").map(|d| d.and_hms_opt(0, 0, 0).unwrap_or_default()))
        .ok()?;
    Some(dt.and_utc().timestamp() as f64 * 1000.0)
}

/// Largeur du champ d'un input (sans ses boutons).
fn field_w(k: Kind) -> f64 {
    match k {
        Kind::Num(..) => 90.0,
        Kind::Choice(_) | Kind::Source => 170.0,
        Kind::Price => 130.0,
        Kind::Time => 160.0,
        Kind::Session => 110.0,
        Kind::Symbol => 130.0,
        Kind::Text { .. } => 260.0,
        Kind::Bool => 24.0,
        Kind::Color => 48.0,
    }
}

/// Largeur de tout le contrôle (champ et − / + ou ‹ ›).
fn widget_w(k: Kind) -> f64 {
    match k {
        Kind::Num(..) | Kind::Choice(_) | Kind::Source => field_w(k) + 64.0,
        _ => field_w(k),
    }
}

impl IndDialog {
    /// Fenêtre centrée dans la fenêtre de l'app (`win_w` × `win_h`).
    pub fn new(ind: Ind, vals: Vec<f64>, win_w: f64, win_h: f64) -> Self {
        let (groups, params) = ind.params();
        Self::table(ind, ind.name(), groups, params, vals, win_w, win_h)
    }

    /// ⚙ The settings window of a user script (step 3), from the table made of its inputs.
    #[allow(clippy::too_many_arguments)]
    pub fn script(
        id: String,
        specs: Vec<argos_script::lang::InputSpec>,
        title: &str,
        groups: &'static [&'static str],
        params: &'static [Param],
        vals: Vec<f64>,
        win_w: f64,
        win_h: f64,
    ) -> Self {
        Self {
            script: Some((id, specs)),
            ..Self::table(Ind::ALL[0], title, groups, params, vals, win_w, win_h)
        }
    }

    /// « Confirmer les inputs » à l'ajout de l'indicateur (comme TradingView et LuxAlgo) : seulement les inputs
    /// `confirm`, dans une seule liste ; `None` s'il n'en a pas.
    pub fn confirm(ind: Ind, vals: Vec<f64>, win_w: f64, win_h: f64) -> Option<Self> {
        let (groups, params) = ind.params();
        if !params.iter().any(|p| p.confirm) {
            return None;
        }
        let mut d = Self {
            confirm: true,
            ..Self::table(ind, &format!("Confirmer les inputs : {}", ind.name()), groups, params, vals, win_w, win_h)
        };
        // juste assez haute pour ses inputs
        d.rect.h = (TABS_Y + 16.0 + d.list_h() + FOOT).min(d.rect.h);
        d.rect.y = ((win_h - d.rect.h) / 2.0).max(6.0);
        Some(d)
    }

    /// Fenêtre pour une table d'inputs donnée.
    fn table(ind: Ind, title: &str, groups: &'static [&'static str], params: &'static [Param], vals: Vec<f64>, win_w: f64, win_h: f64) -> Self {
        let rect = Rect::new(
            ((win_w - W) / 2.0).max(6.0),
            ((win_h - H) / 2.0).max(6.0),
            W.min(win_w - 12.0),
            H.min(win_h - 12.0),
        );
        Self {
            ind,
            script: None,
            title: title.to_string(),
            groups,
            params,
            backup: vals.clone(),
            vals,
            texts: HashMap::new(),
            texts_backup: HashMap::new(),
            sources: vec![],
            tab: 0,
            rect,
            edit: None,
            palette: None,
            scroll: 0.0,
            confirm: false,
            cell: false,
            liste: None,
            menu_defauts: false,
            mes_defauts: None,
            enregistrable: false,
        }
    }

    /// Les défauts enregistrés de l'indicateur (`None` : aucun) ; « Enregistrer comme défaut » devient possible.
    pub fn with_mes_defauts(mut self, d: Option<(Vec<f64>, HashMap<String, String>)>) -> Self {
        self.mes_defauts = d;
        self.enregistrable = true;
        self
    }

    /// Le menu « Défauts » est-il ouvert ?
    pub fn menu_defauts_ouvert(&self) -> bool {
        self.menu_defauts
    }

    /// Bouton « Défauts » (📱 « ⋯ »), coordonnées de la fenêtre de l'app.
    fn bouton_defauts(&self) -> Rect {
        let r = self.rect;
        if self.cell {
            Rect::new(r.x + 21.0, r.y + r.h - 120.0, 60.0, 36.0)
        } else {
            Rect::new(r.x + PAD, r.y + r.h - 58.0, 100.0, 38.0)
        }
    }

    /// Lignes du menu « Défauts » ouvert, au-dessus de son bouton : (zone, ligne, texte).
    fn menu_defauts_zones(&self) -> Vec<(Rect, Hit, &'static str)> {
        if !self.menu_defauts {
            return vec![];
        }
        let lignes: Vec<(Hit, &'static str)> = MENU_DEFAUTS
            .iter()
            .copied()
            .filter(|(h, _)| match h {
                Hit::SaveDef => self.enregistrable,
                Hit::Origine => self.mes_defauts.is_some(),
                _ => true,
            })
            .collect();
        let b = self.bouton_defauts();
        let (lw, lh) = if self.cell {
            (290.0_f64.min(self.rect.w - 32.0), 44.0)
        } else {
            (270.0, 36.0)
        };
        let y0 = b.y - 6.0 - 6.0 - lignes.len() as f64 * lh;
        lignes
            .into_iter()
            .enumerate()
            .map(|(k, (h, t))| (Rect::new(b.x + 6.0, y0 + k as f64 * lh, lw - 12.0, lh), h, t))
            .collect()
    }

    /// Carte (fond) du menu « Défauts » ouvert.
    fn menu_defauts_carte(&self) -> Option<Rect> {
        let z = self.menu_defauts_zones();
        let (first, last) = (z.first()?.0, z.last()?.0);
        Some(Rect::new(first.x - 6.0, first.y - 6.0, first.w + 12.0, last.y + last.h - first.y + 12.0))
    }

    /// Les textes gardés (inputs texte et symbole).
    pub fn with_texts(mut self, texts: HashMap<String, String>) -> Self {
        self.texts_backup = texts.clone();
        self.texts = texts;
        self
    }

    /// Les sorties des autres indicateurs qu'une source peut prendre : (nom affiché, valeur de l'input).
    pub fn with_sources(mut self, sources: Vec<(String, f64)>) -> Self {
        self.sources = sources;
        self
    }

    fn params(&self) -> &'static [Param] {
        self.params
    }

    fn val(&self, i: usize) -> f64 {
        self.vals.get(i).copied().unwrap_or(self.params()[i].def)
    }

    fn inputs(&self) -> Inputs<'_> {
        Inputs {
            params: self.params,
            vals: &self.vals,
        }
    }

    fn text(&self, i: usize) -> String {
        let p = &self.params()[i];
        self.texts.get(p.key).cloned().unwrap_or_else(|| p.text.to_string())
    }

    /// Choix d'une source : les prix, puis les sorties des autres indicateurs.
    fn source_opts(&self) -> Vec<(String, f64)> {
        // (in TradingView's order; the value saved stays the index in SOURCES)
        argos_script::inputs::SOURCES_ORDRE_TV
            .iter()
            .map(|&k| (SOURCES[k].to_string(), k as f64))
            .chain(self.sources.iter().cloned())
            .collect()
    }

    /// Ce qu'affiche le champ de l'input `i`.
    fn field_txt(&self, i: usize) -> String {
        let v = self.val(i);
        match self.params()[i].kind {
            Kind::Choice(o) => o.get(v as usize).copied().unwrap_or("").to_string(),
            Kind::Source => self.source_opts().into_iter().find(|(_, x)| *x == v).map_or_else(
                || match Src::of(v) {
                    Src::Ext(..) => "Sortie d'indicateur".into(),
                    Src::Price(k) => SOURCES[k].into(),
                },
                |(n, _)| n,
            ),
            Kind::Time => time_txt(v),
            Kind::Session => session_txt(v),
            Kind::Text { .. } | Kind::Symbol => self.text(i),
            _ => show(v),
        }
    }

    /// Onglets (passent à la ligne quand ils ne tiennent pas) ; renvoie aussi le bas des onglets.
    fn tabs(&self, text: &Text) -> (Vec<(Rect, usize)>, f64) {
        let r = self.rect;
        let (mut x, mut y) = (r.x + PAD, r.y + TABS_Y);
        let mut out = vec![];
        for (k, g) in self.groups.iter().enumerate().filter(|_| !self.confirm) {
            let w = text.width(g, TAB_FONT) as f64 + 20.0;
            if x + w > r.x + r.w - PAD && x > r.x + PAD {
                x = r.x + PAD;
                y += TAB_H + 4.0;
            }
            out.push((Rect::new(x, y, w, TAB_H), k));
            x += w + 4.0;
        }
        let bottom = if out.is_empty() { r.y + TABS_Y } else { y + TAB_H };
        (out, bottom + 10.0)
    }

    /// Zone de la liste des inputs (sous les onglets, au-dessus des boutons).
    fn list_area(&self, text: &Text) -> Rect {
        let top = self.tabs(text).1 + 6.0;
        Rect::new(self.rect.x, top, self.rect.w, self.rect.y + self.rect.h - FOOT - top)
    }

    /// Lignes de l'onglet ouvert : les inputs d'une ligne (un seul, ou tous ceux du même `inline`, comme Pine).
    fn lines(&self) -> Vec<Vec<usize>> {
        let mut out: Vec<Vec<usize>> = vec![];
        for (i, p) in self.params().iter().enumerate().filter(|(_, p)| {
            if self.confirm {
                p.confirm
            } else if self.cell {
                // 📱 Paramètres en Entrée : tout sauf le style ; Style : les couleurs et le groupe « Style »
                // (épaisseur, type de ligne…) ; Visibilité : à part
                let style = matches!(p.kind, Kind::Color) || self.groups.get(p.group).is_some_and(|g| g.contains("Style"));
                match self.tab {
                    0 => !style,
                    1 => style,
                    _ => false,
                }
            } else {
                p.group == self.tab
            }
        }) {
            match out.iter_mut().find(|l| !p.inline.is_empty() && self.params()[l[0]].inline == p.inline) {
                Some(l) => l.push(i),
                None => out.push(vec![i]),
            }
        }
        out
    }

    fn line_h(&self, l: &[usize]) -> f64 {
        if l.iter().any(|&i| matches!(self.params()[i].kind, Kind::Text { multi: true })) {
            2.0 * ROW
        } else {
            ROW
        }
    }

    /// Hauteur de toute la liste de l'onglet.
    fn list_h(&self) -> f64 {
        self.lines().iter().map(|l| self.line_h(l)).sum()
    }

    /// Lignes visibles : (haut, inputs de la ligne, hauteur).
    fn placed(&self, text: &Text) -> Vec<(f64, Vec<usize>, f64)> {
        let area = self.list_area(text);
        let mut y = area.y - self.scroll;
        let mut out = vec![];
        for l in self.lines() {
            let h = self.line_h(&l);
            if y >= area.y - 1.0 && y + h <= area.y + area.h + 1.0 {
                out.push((y, l, h));
            }
            y += h;
        }
        out
    }

    fn zones(&self, text: &Text) -> Vec<(Rect, Hit)> {
        if self.cell {
            return self.zones_cell(text);
        }
        let r = self.rect;
        let mut out: Vec<(Rect, Hit)> = self.menu_defauts_zones().into_iter().map(|(q, h, _)| (q, h)).collect();
        // palette ouverte (couleurs et transparences) : par-dessus tout le reste
        if let Some(i) = self.palette {
            if let Some(sw) = self.row_zones(text).into_iter().find(|(_, h)| *h == Hit::Swatch(i)).map(|(q, _)| q) {
                let (cols, cell) = (10, 24.0);
                let pw = cols as f64 * cell + 16.0;
                let ph = (PALETTE.len() / cols) as f64 * cell + 16.0 + 30.0;
                let px = (sw.x + sw.w - pw).max(r.x + 8.0);
                let py = if sw.y + sw.h + ph + 4.0 < r.y + r.h {
                    sw.y + sw.h + 4.0
                } else {
                    sw.y - ph - 4.0
                };
                for (k, c) in PALETTE.iter().enumerate() {
                    let (cx, cy) = ((k % cols) as f64, (k / cols) as f64);
                    out.push((Rect::new(px + 8.0 + cx * cell, py + 8.0 + cy * cell, cell - 3.0, cell - 3.0), Hit::Pick(*c)));
                }
                let tw = (pw - 16.0) / TRANSP.len() as f64;
                for (k, t) in TRANSP.iter().enumerate() {
                    out.push((Rect::new(px + 8.0 + k as f64 * tw, py + ph - 32.0, tw - 4.0, 24.0), Hit::Transp(*t)));
                }
            }
        }
        out.push((Rect::new(r.x + r.w - 44.0, r.y + 16.0, 28.0, 28.0), Hit::Close));
        out.extend(self.tabs(text).0.into_iter().map(|(q, k)| (q, Hit::Tab(k))));
        out.extend(self.row_zones(text));
        let by = r.y + r.h - 58.0;
        out.push((Rect::new(r.x + PAD, by, 100.0, 38.0), Hit::Defaults));
        out.push((Rect::new(r.x + r.w - PAD - 90.0 - 10.0 - 100.0, by, 100.0, 38.0), Hit::Cancel));
        out.push((Rect::new(r.x + r.w - PAD - 90.0, by, 90.0, 38.0), Hit::Ok));
        out
    }

    /// Zones des lignes visibles : les contrôles de chaque ligne, alignés à droite, le dernier input au bout (comme
    /// Pine) ; le « ? » de l'aide juste avant les contrôles.
    fn row_zones(&self, text: &Text) -> Vec<(Rect, Hit)> {
        let right = self.rect.x + self.rect.w - PAD;
        let mut out = vec![];
        for (y, l, h) in self.placed(text) {
            let cy = y + 21.0;
            let mut x = right;
            for (n, &i) in l.iter().enumerate().rev() {
                let p = &self.params()[i];
                let fw = field_w(p.kind);
                match p.kind {
                    Kind::Bool => out.push((Rect::new(x - 24.0, cy - 12.0, 24.0, 24.0), Hit::Check(i))),
                    Kind::Color => out.push((Rect::new(x - 48.0, cy - 14.0, 48.0, 28.0), Hit::Swatch(i))),
                    Kind::Num(..) | Kind::Choice(_) | Kind::Source => {
                        out.push((Rect::new(x - 32.0, cy - 16.0, 32.0, 32.0), Hit::Plus(i)));
                        out.push((Rect::new(x - 32.0 - fw, cy - 16.0, fw, 32.0), Hit::Field(i)));
                        out.push((Rect::new(x - 64.0 - fw, cy - 16.0, 32.0, 32.0), Hit::Minus(i)));
                    }
                    Kind::Text { multi: true } => out.push((Rect::new(x - fw, y + 5.0, fw, h - 10.0), Hit::Field(i))),
                    // prix / heure : le 📍 au bout du champ choisit sur la chart
                    Kind::Price | Kind::Time => {
                        out.push((Rect::new(x - 26.0, cy - 16.0, 26.0, 32.0), Hit::Here(i)));
                        out.push((Rect::new(x - fw, cy - 16.0, fw - 26.0, 32.0), Hit::Field(i)));
                    }
                    _ => out.push((Rect::new(x - fw, cy - 16.0, fw, 32.0), Hit::Field(i))),
                }
                x -= widget_w(p.kind) + GAP;
                // le nom d'un input suivant sur la même ligne se met devant son contrôle
                if n > 0 {
                    x -= text.width(p.label, 12.5) as f64 + GAP;
                }
            }
            // aide : la dernière de la ligne (comme Pine)
            if let Some(&i) = l.iter().rev().find(|&&i| !self.params()[i].tip.is_empty()) {
                out.push((Rect::new(x - 22.0, cy - 9.0, 18.0, 18.0), Hit::Tip(i)));
            }
        }
        out
    }

    /// Valeur `v` bornée gardée pour l'input `i`.
    /// 📍 Point choisi sur la chart pour l'input `i` : son prix ou son heure, et l'autre moitié du point (prix + heure
    /// du même groupe sur la même ligne `inline`, comme Pine).
    pub fn here(&mut self, i: usize, price: f64, time_ms: f64) {
        let p = &self.params()[i];
        let pair: Vec<usize> = (0..self.params().len())
            .filter(|&k| k == i || (!p.inline.is_empty() && self.params()[k].group == p.group && self.params()[k].inline == p.inline))
            .collect();
        for k in pair {
            match self.params()[k].kind {
                Kind::Price => _ = self.set(k, price),
                Kind::Time => _ = self.set(k, time_ms),
                _ => {}
            }
        }
    }

    fn set(&mut self, i: usize, v: f64) -> IndClick {
        let v = clamp(&self.params()[i], v);
        if self.vals.len() < self.params().len() {
            self.vals = (0..self.params().len()).map(|k| self.val(k)).collect();
        }
        if self.vals[i] == v {
            return IndClick::None;
        }
        self.vals[i] = v;
        IndClick::Changed
    }

    /// Valide la saisie en cours (s'il y en a une).
    fn commit(&mut self) -> IndClick {
        let Some((i, s)) = self.edit.take() else { return IndClick::None };
        let p = self.params()[i];
        match p.kind {
            Kind::Text { .. } | Kind::Symbol => {
                let s = if matches!(p.kind, Kind::Symbol) { s.trim().to_uppercase() } else { s };
                if self.text(i) == s {
                    return IndClick::None;
                }
                self.texts.insert(p.key.to_string(), s);
                IndClick::Changed
            }
            Kind::Time => parse_time(&s).map_or(IndClick::None, |v| self.set(i, v)),
            Kind::Session => {
                let v = session_val(s.trim());
                if v == 0.0 && s.trim() != "0000-0000" {
                    IndClick::None
                } else {
                    self.set(i, v)
                }
            }
            _ => s.replace(',', ".").parse::<f64>().map_or(IndClick::None, |v| self.set(i, v)),
        }
    }

    /// Molette : fait défiler la liste (`dy` > 0 : vers le bas).
    /// Barre du titre (sans le ✕) : on l'attrape pour déplacer la fenêtre.
    pub fn title_bar(&self) -> Rect {
        Rect::new(self.rect.x, self.rect.y, self.rect.w - 52.0, TABS_Y - 6.0)
    }

    pub fn wheel(&mut self, dy: f64, text: &Text) {
        let max = if self.cell {
            self.max_cell()
        } else {
            (self.list_h() - self.list_area(text).h).max(0.0)
        };
        self.liste = None;
        self.scroll = (self.scroll + dy).clamp(0.0, max);
        self.palette = None;
    }

    /// Clic en `(x, y)` (coordonnées de la fenêtre de l'app). `text` sert à placer les onglets.
    pub fn click(&mut self, x: f64, y: f64, text: &Text) -> IndClick {
        let hit = self.zones(text).into_iter().find(|(q, _)| q.contains(x, y)).map(|(_, h)| h);
        // un input grisé (sa case `active` décochée) ne se change pas
        let idx = match hit {
            Some(Hit::Minus(i) | Hit::Plus(i) | Hit::Field(i) | Hit::Check(i) | Hit::Swatch(i) | Hit::Here(i)) => Some(i),
            _ => None,
        };
        if idx.is_some_and(|i| !self.inputs().active(i)) {
            return IndClick::None;
        }
        let typed = if matches!(hit, Some(Hit::Field(_))) { IndClick::None } else { self.commit() };
        let had_palette = self.palette.take();
        let had_liste = self.liste.take();
        let had_menu = std::mem::take(&mut self.menu_defauts);
        // menu « Défauts » ouvert : un clic à côté le ferme seulement
        if had_menu && !matches!(hit, Some(Hit::Defaults | Hit::Reset | Hit::SaveDef | Hit::Origine)) {
            return typed;
        }
        let out = match hit {
            None if !self.rect.contains(x, y) => IndClick::Close(true), // à côté : on garde, comme v1
            None | Some(Hit::Tip(_)) => IndClick::None,
            Some(Hit::Here(i)) => IndClick::Here(i),
            Some(Hit::Close) | Some(Hit::Ok) => IndClick::Close(true),
            Some(Hit::Cancel) => IndClick::Close(false),
            Some(Hit::Tab(k)) => {
                self.tab = k;
                self.scroll = 0.0;
                IndClick::None
            }
            Some(Hit::Defaults) => {
                self.menu_defauts = !had_menu;
                IndClick::None
            }
            // les défauts enregistrés s'il y en a, sinon ceux de l'indicateur
            Some(Hit::Reset) => match self.mes_defauts.clone() {
                Some((v, t)) => {
                    let n = self.params().len();
                    self.vals = (0..n).map(|k| v.get(k).copied().unwrap_or(self.params()[k].def)).collect();
                    self.texts = t;
                    IndClick::Changed
                }
                None => self.origine(),
            },
            Some(Hit::Origine) => self.origine(),
            Some(Hit::SaveDef) => {
                self.mes_defauts = Some((self.vals.clone(), self.texts.clone()));
                IndClick::SaveDefaults
            }
            Some(Hit::Check(i)) => self.set(i, 1.0 - self.val(i)),
            Some(Hit::Swatch(i)) => {
                self.palette = (had_palette != Some(i)).then_some(i);
                IndClick::None
            }
            Some(Hit::Pick(c)) => match had_palette {
                // nouvelle couleur, même transparence ; la palette reste ouverte pour choisir la transparence
                Some(i) => {
                    let t = (self.val(i) as u32) & 0x7f00_0000;
                    self.palette = Some(i);
                    self.set(i, (c | t) as f64)
                }
                None => IndClick::None,
            },
            Some(Hit::Transp(t)) => match had_palette {
                Some(i) => self.set(i, (((self.val(i) as u32) & 0xff_ffff) | (t << 24)) as f64),
                None => IndClick::None,
            },
            Some(Hit::Minus(i)) | Some(Hit::Plus(i)) => {
                let plus = matches!(hit, Some(Hit::Plus(_)));
                match self.params()[i].kind {
                    Kind::Source => self.step_source(i, plus),
                    Kind::Num(_, _, s) => self.set(i, self.val(i) + if plus { s } else { -s }),
                    _ => self.set(i, self.val(i) + if plus { 1.0 } else { -1.0 }),
                }
            }
            // 📱 une liste : ses options dans une pilule de verre (un 2e toucher la ferme)
            Some(Hit::Field(i)) if self.cell && matches!(self.params()[i].kind, Kind::Choice(_) | Kind::Source) => {
                self.liste = (had_liste != Some(i)).then_some(i);
                IndClick::None
            }
            Some(Hit::Opt(i, k)) => match self.params()[i].kind {
                Kind::Source => {
                    let v = self.source_opts().get(k).map(|o| o.1);
                    v.map_or(IndClick::None, |v| self.set(i, v))
                }
                _ => self.set(i, k as f64),
            },
            Some(Hit::Field(i)) if matches!(self.params()[i].kind, Kind::Choice(_)) => self.set(i, self.val(i) + 1.0),
            Some(Hit::Field(i)) if matches!(self.params()[i].kind, Kind::Source) => self.step_source(i, true),
            Some(Hit::Field(i)) => {
                let typed = self.commit();
                let s = match self.params()[i].kind {
                    Kind::Num(..) | Kind::Price => show(self.val(i)),
                    _ => self.field_txt(i),
                };
                self.edit = Some((i, s));
                typed
            }
        };
        if typed == IndClick::Changed && out == IndClick::None {
            IndClick::Changed
        } else {
            out
        }
    }

    /// Les réglages d'origine de l'indicateur (ceux de son code).
    fn origine(&mut self) -> IndClick {
        self.vals = defaults(self.params());
        self.texts.clear();
        IndClick::Changed
    }

    /// Source suivante (ou précédente) : les prix, puis les sorties des autres indicateurs.
    fn step_source(&mut self, i: usize, next: bool) -> IndClick {
        let opts = self.source_opts();
        let k = opts.iter().position(|(_, v)| *v == self.val(i)).unwrap_or(0) as i64 + if next { 1 } else { -1 };
        let v = opts[k.rem_euclid(opts.len() as i64) as usize].1;
        self.set(i, v)
    }

    /// Un champ est-il en saisie ?
    pub fn typing(&self) -> bool {
        self.edit.is_some()
    }

    /// Caractère tapé dans le champ en saisie (filtré selon la sorte d'input).
    pub fn type_char(&mut self, c: char) {
        let Some((i, s)) = &mut self.edit else { return };
        let ok = match self.params[*i].kind {
            Kind::Num(..) | Kind::Price => c.is_ascii_digit() || matches!(c, '.' | ',' | '-'),
            Kind::Time => c.is_ascii_digit() || matches!(c, '-' | ':' | ' '),
            Kind::Session => (c.is_ascii_digit() || c == '-') && s.len() < 9,
            Kind::Symbol => !c.is_whitespace() && !c.is_control(),
            Kind::Text { multi } => !c.is_control() || (multi && c == '\n'),
            _ => false,
        };
        if ok {
            s.push(c);
        }
    }

    pub fn backspace(&mut self) {
        if let Some((_, s)) = &mut self.edit {
            s.pop();
        }
    }

    /// Entrée : valide la saisie (un texte sur plusieurs lignes : nouvelle ligne) ; sinon : OK.
    pub fn enter(&mut self) -> IndClick {
        match &self.edit {
            Some((i, _)) if matches!(self.params[*i].kind, Kind::Text { multi: true }) => {
                self.type_char('\n');
                IndClick::None
            }
            Some(_) => self.commit(),
            None => IndClick::Close(true),
        }
    }

    pub fn draw(&self, text: &Text, mouse: (f64, f64)) -> Option<(Pixmap, Rect)> {
        if self.cell {
            return self.draw_cell(text, mouse);
        }
        let r = self.rect;
        let mut pm = crate::px::new(r.w as u32, r.h as u32)?;
        let loc = |q: Rect| Rect::new(q.x - r.x, q.y - r.y, q.w, q.h);
        crate::shapes::menu_card(&mut pm, loc(r));
        text.draw(
            &mut pm,
            &text.fit(&format!("⚙️ {}", self.title), 19.0, r.w - 2.0 * PAD - 40.0),
            (PAD as f32, 40.0),
            19.0,
            WHITE,
            Align::Left,
        );
        let area = self.list_area(text);
        crate::fill(&mut pm, 0.0, area.y - 8.0 - r.y, r.w, 1.0, pal::edge(), 1.0);
        let hover = |q: &Rect| q.contains(mouse.0, mouse.1);
        if self.params().is_empty() {
            text.draw(
                &mut pm,
                "Pas encore de réglages pour cet indicateur (valeurs de v1).",
                (PAD as f32, (area.y - r.y + 24.0) as f32),
                14.0,
                GREY,
                Align::Left,
            );
        }
        let inp = self.inputs();
        let zones = self.zones(text);
        let first_ctrl = |i: usize| {
            zones
                .iter()
                .filter(|(_, h)| matches!(h, Hit::Minus(k) | Hit::Field(k) | Hit::Check(k) | Hit::Swatch(k) if *k == i))
                .map(|(q, _)| q.x)
                .fold(f64::INFINITY, f64::min)
        };
        // noms : le 1er input de la ligne à gauche ; les suivants devant leur contrôle ; grisés si inactifs
        for (y, l, _) in self.placed(text) {
            for (n, &i) in l.iter().enumerate() {
                let p = &self.params()[i];
                let c = if inp.active(i) { WHITE } else { GREY };
                if n == 0 {
                    let left = l.iter().map(|&k| first_ctrl(k)).fold(f64::INFINITY, f64::min);
                    let room = left - r.x - PAD - 30.0;
                    text.draw(
                        &mut pm,
                        &text.fit(p.label, 14.0, room.max(40.0)),
                        (PAD as f32, (y - r.y + 26.0) as f32),
                        14.0,
                        c,
                        Align::Left,
                    );
                } else {
                    let x = first_ctrl(i) - GAP;
                    text.draw(&mut pm, p.label, ((x - r.x) as f32, (y - r.y + 26.0) as f32), 12.5, GREY, Align::Right);
                }
            }
        }
        // barre de défilement
        let total = self.list_h();
        if total > area.h {
            let h = (area.h * area.h / total).max(24.0);
            let y = area.y + (area.h - h) * self.scroll / (total - area.h);
            crate::jetons::barre_defilement(&mut pm, r.w - 2.0, y - r.y, h, false);
        }
        let palette_open = self.palette.is_some();
        let mut tip: Option<(Rect, &str)> = None;
        // la palette (en tête de la table) se dessine en dernier, par-dessus
        for (q, h) in zones.iter().rev() {
            let (q, h) = (*q, *h);
            let l = loc(q);
            let dim = match h {
                Hit::Minus(i) | Hit::Plus(i) | Hit::Field(i) | Hit::Check(i) | Hit::Swatch(i) | Hit::Here(i) => !inp.active(i),
                _ => false,
            };
            let a = if dim { 0.4 } else { 1.0 };
            match h {
                Hit::Opt(..) | Hit::Reset | Hit::SaveDef | Hit::Origine => {}
                Hit::Close => {
                    if hover(&q) {
                        round_fill(&mut pm, l, 6.0, pal::hover(), 1.0);
                    }
                    lines(&mut pm, l.cx(), l.cy(), &[&[(-6.0, -6.0), (6.0, 6.0)], &[(6.0, -6.0), (-6.0, 6.0)]], WHITE, 1.6);
                }
                Hit::Tab(k) => {
                    let on = k == self.tab;
                    crate::shapes::menu_row(&mut pm, l, on, hover(&q));
                    text.draw(
                        &mut pm,
                        self.groups[k],
                        (l.cx() as f32, (l.cy() + 5.0) as f32),
                        TAB_FONT,
                        if on { WHITE } else { GREY },
                        Align::Center,
                    );
                }
                Hit::Here(_) => {
                    round_fill(&mut pm, l, 6.0, if hover(&q) && !dim { pal::hover() } else { pal::field() }, a);
                    round_stroke(&mut pm, l, 6.0, pal::edge(), 1.0);
                    crate::icons::draw(&mut pm, "📍", l.cx(), l.cy(), 16, WHITE);
                }
                Hit::Tip(i) => {
                    round_fill(&mut pm, l, l.w / 2.0, pal::on(), 1.0);
                    text.draw(&mut pm, "?", (l.cx() as f32, (l.cy() + 4.5) as f32), 12.0, WHITE, Align::Center);
                    if hover(&q) {
                        tip = Some((q, self.params()[i].tip));
                    }
                }
                Hit::Minus(i) | Hit::Plus(i) => {
                    round_fill(&mut pm, l, 6.0, if hover(&q) && !dim { pal::hover() } else { pal::field() }, a);
                    round_stroke(&mut pm, l, 6.0, pal::edge(), 1.0);
                    let plus = matches!(h, Hit::Plus(_));
                    let shape: &[&[(f32, f32)]] = match (self.params()[i].kind, plus) {
                        (Kind::Choice(_) | Kind::Source, true) => &[&[(-2.5, -5.0), (2.5, 0.0), (-2.5, 5.0)]],
                        (Kind::Choice(_) | Kind::Source, false) => &[&[(2.5, -5.0), (-2.5, 0.0), (2.5, 5.0)]],
                        (_, true) => &[&[(-5.0, 0.0), (5.0, 0.0)], &[(0.0, -5.0), (0.0, 5.0)]],
                        (_, false) => &[&[(-5.0, 0.0), (5.0, 0.0)]],
                    };
                    lines(&mut pm, l.cx(), l.cy(), shape, if dim { GREY } else { WHITE }, 1.6);
                }
                Hit::Field(i) => {
                    let on = matches!(&self.edit, Some((k, _)) if *k == i);
                    let p = &self.params()[i];
                    round_fill(&mut pm, l, 6.0, pal::field(), a);
                    round_stroke(&mut pm, l, 6.0, if on { accent() } else { pal::edge() }, 1.0);
                    let s = match &self.edit {
                        Some((k, s)) if *k == i => format!("{s}|"),
                        _ => self.field_txt(i),
                    };
                    let c = if dim { GREY } else { WHITE };
                    match p.kind {
                        Kind::Text { multi: true } => {
                            for (k, line) in s.split('\n').take(((l.h - 6.0) / 17.0) as usize).enumerate() {
                                text.draw(
                                    &mut pm,
                                    &text.fit(line, 13.0, l.w - 14.0),
                                    ((l.x + 8.0) as f32, (l.y + 17.0 + k as f64 * 17.0) as f32),
                                    13.0,
                                    c,
                                    Align::Left,
                                );
                            }
                        }
                        Kind::Price | Kind::Time => text.draw(
                            &mut pm,
                            &text.fit(&s, 13.5, l.w - 12.0),
                            ((l.x + 8.0) as f32, (l.cy() + 5.0) as f32),
                            13.5,
                            c,
                            Align::Left,
                        ),
                        _ => text.draw(
                            &mut pm,
                            &text.fit(&s, 13.5, l.w - 10.0),
                            (l.cx() as f32, (l.cy() + 5.0) as f32),
                            13.5,
                            c,
                            Align::Center,
                        ),
                    }
                }
                Hit::Check(i) => {
                    let on = self.val(i) != 0.0;
                    round_fill(&mut pm, l, 5.0, if on { accent() } else { pal::field() }, a);
                    round_stroke(&mut pm, l, 5.0, if on { accent() } else { pal::edge() }, 1.0);
                    if on {
                        lines(&mut pm, l.cx(), l.cy(), &[&[(-5.5, 0.0), (-1.5, 4.0), (5.5, -4.0)]], WHITE, 2.0);
                    }
                }
                Hit::Swatch(i) => {
                    round_fill(&mut pm, l, 6.0, pal::field(), 1.0);
                    round_stroke(&mut pm, l, 6.0, if self.palette == Some(i) { accent() } else { pal::edge() }, 1.0);
                    let op = inp.opacity(self.params()[i].key) * a;
                    round_fill(&mut pm, Rect::new(l.x + 5.0, l.y + 5.0, l.w - 10.0, l.h - 10.0), 4.0, rgb(self.val(i)), op);
                }
                Hit::Pick(c) => {
                    round_fill(&mut pm, l, 3.0, rgb(c as f64), 1.0);
                    if hover(&q) || self.palette.is_some_and(|i| (self.val(i) as u32) & 0xff_ffff == c) {
                        round_stroke(&mut pm, Rect::new(l.x - 2.0, l.y - 2.0, l.w + 4.0, l.h + 4.0), 4.0, WHITE, 1.5);
                    }
                }
                Hit::Transp(t) => {
                    let on = self.palette.is_some_and(|i| ((self.val(i) as u32) >> 24) & 0x7f == t);
                    crate::shapes::menu_row(&mut pm, l, on, hover(&q));
                    if !on && !hover(&q) {
                        round_stroke(&mut pm, l, 8.0, pal::edge(), 1.0);
                    }
                    text.draw(&mut pm, &format!("{t} %"), (l.cx() as f32, (l.cy() + 4.5) as f32), 11.5, WHITE, Align::Center);
                }
                Hit::Defaults | Hit::Cancel | Hit::Ok => {
                    let ok = h == Hit::Ok;
                    round_fill(
                        &mut pm,
                        l,
                        8.0,
                        if ok {
                            accent()
                        } else if hover(&q) {
                            pal::hover()
                        } else {
                            pal::field()
                        },
                        1.0,
                    );
                    if !ok {
                        let ouvert = h == Hit::Defaults && self.menu_defauts;
                        round_stroke(&mut pm, l, 8.0, if ouvert { accent() } else { pal::edge() }, 1.0);
                    }
                    let label = match h {
                        Hit::Defaults => "Défauts",
                        Hit::Cancel => "Annuler",
                        _ => "OK",
                    };
                    text.draw(&mut pm, label, (l.cx() as f32, (l.cy() + 5.0) as f32), 14.0, WHITE, Align::Center);
                }
            }
            // fond de la palette juste avant ses cases (la dernière case rencontrée ici est la 1re)
            if palette_open && matches!(h, Hit::Close) {
                let picks: Vec<Rect> = zones
                    .iter()
                    .filter(|(_, h)| matches!(h, Hit::Pick(_) | Hit::Transp(_)))
                    .map(|(q, _)| *q)
                    .collect();
                let (x0, y0) = picks.iter().fold((f64::INFINITY, f64::INFINITY), |(a, b), q| (a.min(q.x), b.min(q.y)));
                let (x1, y1) = picks.iter().fold((0.0f64, 0.0f64), |(a, b), q| (a.max(q.x + q.w), b.max(q.y + q.h)));
                if !picks.is_empty() {
                    let bg = Rect::new(x0 - 8.0, y0 - 8.0, x1 - x0 + 16.0, y1 - y0 + 16.0);
                    round_fill(&mut pm, loc(bg), 8.0, pal::panel(), 1.0);
                    round_stroke(&mut pm, loc(bg), 8.0, pal::edge(), 1.0);
                }
            }
        }
        // menu « Défauts » ouvert : par-dessus la liste
        if let Some(c) = self.menu_defauts_carte() {
            crate::shapes::menu_card(&mut pm, loc(c));
            for (q, _, t) in self.menu_defauts_zones() {
                let l = loc(q);
                crate::shapes::menu_row(&mut pm, l, false, hover(&q));
                text.draw(&mut pm, t, ((l.x + 12.0) as f32, (l.cy() + 5.0) as f32), 14.0, WHITE, Align::Left);
            }
        }
        // bulle d'aide par-dessus tout (`\n` = nouvelle ligne)
        if let Some((q, t)) = tip {
            let rows: Vec<&str> = t.split('\n').collect();
            let w = rows.iter().map(|s| text.width(s, 12.5) as f64).fold(0.0, f64::max) + 20.0;
            let h = rows.len() as f64 * 18.0 + 12.0;
            let b = Rect::new((q.x + q.w - w).max(r.x + 8.0), q.y - h - 6.0, w, h);
            let lb = loc(b);
            round_fill(&mut pm, lb, 8.0, pal::on(), 1.0);
            round_stroke(&mut pm, lb, 8.0, pal::edge(), 1.0);
            for (k, s) in rows.iter().enumerate() {
                text.draw(
                    &mut pm,
                    s,
                    ((lb.x + 10.0) as f32, (lb.y + 19.0 + k as f64 * 18.0) as f32),
                    12.5,
                    WHITE,
                    Align::Left,
                );
            }
        }
        Some((pm, r))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use argos_script::inputs::{check, color_t, int, price, session, source, source_ext, string, symbol, text_area, time};

    fn at(d: &IndDialog, t: &Text, h: Hit) -> (f64, f64) {
        d.zones(t)
            .iter()
            .find(|(_, x)| *x == h)
            .map(|(q, _)| (q.cx(), q.cy()))
            .unwrap_or_else(|| panic!("{h:?} introuvable"))
    }

    #[test]
    fn reglages_tvd() {
        let t = Text::default();
        let mut d = IndDialog::new(Ind::Tvd, defaults(Ind::Tvd.params().1), 1600.0, 900.0);
        // RSI Length (input 0) : + → 11
        let (px, py) = at(&d, &t, Hit::Plus(0));
        assert_eq!(d.click(px, py, &t), IndClick::Changed);
        assert_eq!(d.vals[0], 11.0);
        // saisie au clavier : 500 → borné à 100 (max de v1)
        let (fx, fy) = at(&d, &t, Hit::Field(0));
        d.click(fx, fy, &t);
        d.backspace();
        d.backspace();
        "500".chars().for_each(|c| d.type_char(c));
        assert_eq!(d.enter(), IndClick::Changed);
        assert_eq!(d.vals[0], 100.0);
        // onglet 🕯️ Volume Vector : la case « Vector Candles » (input 14) se coupe
        let (tx, ty) = at(&d, &t, Hit::Tab(2));
        d.click(tx, ty, &t);
        let (cx, cy) = at(&d, &t, Hit::Check(14));
        assert_eq!(d.click(cx, cy, &t), IndClick::Changed);
        assert_eq!(d.vals[14], 0.0);
        // Défauts ▸ Réinitialiser les réglages, puis Annuler
        let (dx, dy) = at(&d, &t, Hit::Defaults);
        d.click(dx, dy, &t);
        let (rx, ry) = at(&d, &t, Hit::Reset);
        assert_eq!(d.click(rx, ry, &t), IndClick::Changed);
        assert_eq!(d.vals, defaults(Ind::Tvd.params().1));
        let (ax, ay) = at(&d, &t, Hit::Cancel);
        assert_eq!(d.click(ax, ay, &t), IndClick::Close(false));
        assert!(d.draw(&t, (0.0, 0.0)).is_some());
    }

    /// Menu « Défauts » (comme TradingView) : Réinitialiser · Enregistrer comme défaut · Revenir aux réglages
    /// d'origine (seulement s'il y a des défauts enregistrés) ; un clic à côté le ferme sans rien changer.
    #[test]
    fn menu_defauts() {
        let t = Text::default();
        let origine = defaults(Ind::Tvd.params().1);
        let mut d = IndDialog::new(Ind::Tvd, origine.clone(), 1600.0, 900.0).with_mes_defauts(None);
        let has = |d: &IndDialog, h: Hit| d.zones(&t).iter().any(|(_, x)| *x == h);
        assert!(!has(&d, Hit::Reset), "menu fermé au départ");
        let (dx, dy) = at(&d, &t, Hit::Defaults);
        d.click(dx, dy, &t);
        assert!(has(&d, Hit::Reset) && has(&d, Hit::SaveDef) && !has(&d, Hit::Origine));
        assert!(d.draw(&t, (dx, dy - 40.0)).is_some());
        // le menu tient dans la fenêtre, au-dessus du bouton
        let (carte, b) = (d.menu_defauts_carte().unwrap(), d.bouton_defauts());
        assert!(carte.y >= d.rect.y && carte.y + carte.h <= b.y && carte.x + carte.w <= d.rect.x + d.rect.w);
        // clic à côté : le menu se ferme, la valeur ne bouge pas
        let (px, py) = at(&d, &t, Hit::Plus(0));
        assert_eq!(d.click(px, py, &t), IndClick::None);
        assert!(!d.menu_defauts_ouvert());
        assert_eq!(d.vals, origine);
        // RSI 11, puis Enregistrer comme défaut
        d.click(px, py, &t);
        assert_eq!(d.vals[0], 11.0);
        d.click(dx, dy, &t);
        let (sx, sy) = at(&d, &t, Hit::SaveDef);
        assert_eq!(d.click(sx, sy, &t), IndClick::SaveDefaults);
        assert!(!d.menu_defauts_ouvert());
        assert_eq!(d.mes_defauts.as_ref().map(|m| m.0[0]), Some(11.0));
        // RSI 12 ; Réinitialiser = mes défauts (11) ; Revenir aux réglages d'origine = 10
        d.click(px, py, &t);
        assert_eq!(d.vals[0], 12.0);
        d.click(dx, dy, &t);
        assert!(has(&d, Hit::Origine));
        let (rx, ry) = at(&d, &t, Hit::Reset);
        assert_eq!(d.click(rx, ry, &t), IndClick::Changed);
        assert_eq!(d.vals[0], 11.0);
        d.click(dx, dy, &t);
        let (ox, oy) = at(&d, &t, Hit::Origine);
        assert_eq!(d.click(ox, oy, &t), IndClick::Changed);
        assert_eq!(d.vals, origine);
        // un script (pas de with_mes_defauts) : seulement Réinitialiser
        let mut s = IndDialog::new(Ind::Tvd, origine.clone(), 1600.0, 900.0);
        s.click(dx, dy, &t);
        assert!(has(&s, Hit::Reset) && !has(&s, Hit::SaveDef) && !has(&s, Hit::Origine));
        // 📱 cell : le « ⋯ » ouvre le même menu, dans la feuille
        let mut c = IndDialog::new(Ind::Tvd, origine.clone(), 390.0, 844.0)
            .with_mes_defauts(None)
            .en_cell(390.0, 844.0);
        let (cx, cy) = at(&c, &t, Hit::Defaults);
        c.click(cx, cy, &t);
        let carte = c.menu_defauts_carte().unwrap();
        assert!(carte.x >= 0.0 && carte.x + carte.w <= 390.0 && carte.y >= 170.0);
        let (sx, sy) = at(&c, &t, Hit::SaveDef);
        assert_eq!(c.click(sx, sy, &t), IndClick::SaveDefaults);
        assert!(c.draw(&t, (0.0, 0.0)).is_some());
    }

    #[test]
    fn reglages_cipher_liste_couleur_defilement() {
        let t = Text::default();
        let p = Ind::Cipher.params().1;
        let mut d = IndDialog::new(Ind::Cipher, defaults(p), 1600.0, 900.0);
        // 9 onglets (avec 🟡 Golden Dot Table) : sur 2 lignes, tous dans la fenêtre
        let (tabs, _) = d.tabs(&t);
        assert_eq!(tabs.len(), 9);
        assert!(tabs.iter().all(|(q, _)| q.x + q.w <= d.rect.x + d.rect.w));
        // WaveTrend : 25 inputs, la liste défile jusqu'en bas
        d.wheel(10_000.0, &t);
        assert!(d.scroll > 0.0);
        let last = p.iter().rposition(|x| x.group == 0).unwrap();
        assert!(
            d.row_zones(&t).iter().any(|(_, h)| matches!(h, Hit::Check(i) | Hit::Plus(i) if *i == last)),
            "dernier input visible"
        );
        // « WT MA Source » (liste) : clic sur le champ → option suivante
        d.wheel(-10_000.0, &t);
        let src = p.iter().position(|x| x.key == "wt_src").unwrap();
        d.wheel(((src as f64) - 3.0).max(0.0) * ROW, &t);
        let (fx, fy) = at(&d, &t, Hit::Field(src));
        assert_eq!(d.click(fx, fy, &t), IndClick::Changed);
        assert_eq!(d.vals[src], 6.0, "hlc3 → ohlc4");
        // couleurs : la palette s'ouvre, une case choisit la couleur, puis une transparence
        let col = p.iter().position(|x| x.key == "pc_wt1").unwrap();
        let (tx, ty) = at(&d, &t, Hit::Tab(7));
        d.click(tx, ty, &t);
        d.wheel(((col - p.iter().position(|x| x.group == 7).unwrap()) as f64) * ROW, &t);
        let (sx, sy) = at(&d, &t, Hit::Swatch(col));
        d.click(sx, sy, &t);
        let (kx, ky) = at(&d, &t, Hit::Pick(0xff9800));
        assert_eq!(d.click(kx, ky, &t), IndClick::Changed);
        assert_eq!(d.vals[col], 0xff9800 as f64);
        let (kx, ky) = at(&d, &t, Hit::Transp(40));
        assert_eq!(d.click(kx, ky, &t), IndClick::Changed);
        assert_eq!(d.vals[col], (0xff9800u32 | (40 << 24)) as f64, "orange à 40 % de transparence");
        assert!(d.draw(&t, (0.0, 0.0)).is_some());
    }

    /// Table d'essai avec toutes les sortes d'inputs de Pine v6.
    const GROUPS: [&str; 2] = ["⚙️ Réglages", "📝 Textes"];
    const ALL: [Param; 11] = [
        check("sig", 0, "Afficher le signal", true),
        int("slen", 0, "Longueur", 9.0, 1.0, 50.0)
            .active_if("sig")
            .inline("s")
            .tip("Lissage de la ligne\nsignal"),
        color_t("scol", 0, "Couleur", 0xffffff, 0).inline("s"),
        source("src", 0, "Source", 0),
        price("lvl", 0, "Niveau", 80_000.0).confirm(),
        time("t0", 0, "Départ", 1_793_039_400_000.0),
        session("sess", 0, "Session", "0930-1600"),
        string("note", 1, "Note", "OK"),
        symbol("cmp", 1, "Comparer avec", "ETH"),
        text_area("syms", 1, "Symboles", "BTC,ETH"),
        int("last", 1, "Dernier", 3.0, 1.0, 10.0),
    ];

    #[test]
    fn confirmer_et_choisir_sur_la_chart() {
        let mut d = IndDialog::confirm(Ind::AnchoredVwap, defaults(Ind::AnchoredVwap.params().1), 1600.0, 900.0).expect("ancrage à confirmer");
        let t = Text::default();
        assert_eq!(d.lines(), vec![vec![0]], "seulement l'ancrage, sans onglets");
        assert!(d.tabs(&t).0.is_empty());
        let here = d.zones(&t).into_iter().find(|(_, h)| *h == Hit::Here(0)).expect("📍 de l'ancrage").0;
        assert_eq!(d.click(here.cx(), here.cy(), &t), IndClick::Here(0));
        d.here(0, 1.0, 1_700_000_000_000.0);
        assert_eq!(d.vals[0], 1_700_000_000_000.0);
        assert!(IndDialog::confirm(Ind::Rsi, vec![], 1600.0, 900.0).is_none(), "rien à confirmer");
        d.draw(&t, (0.0, 0.0)).expect("dessin");
    }

    #[test]
    fn toutes_les_sortes_comme_pine() {
        let t = Text::default();
        let mut d = IndDialog::table(Ind::Rsi, "Essai", &GROUPS, &ALL, defaults(&ALL), 1600.0, 900.0)
            .with_sources(vec![("Argos TVD Pro : AI".into(), source_ext(0, 1))]);
        // inline : « Longueur » et « Couleur » sur une seule ligne ; la case seule sur la sienne
        assert_eq!(d.lines()[..2], [vec![0], vec![1, 2]]);
        let (_, y1) = at(&d, &t, Hit::Field(1));
        let (_, y2) = at(&d, &t, Hit::Swatch(2));
        assert_eq!(y1, y2, "même ligne");
        // tooltip : un « ? » sur la ligne
        assert!(d.zones(&t).iter().any(|(_, h)| *h == Hit::Tip(1)));
        assert!(d.draw(&t, at(&d, &t, Hit::Tip(1))).is_some(), "bulle d'aide au survol");
        // active : case décochée → la longueur ne se change plus
        let (x, y) = at(&d, &t, Hit::Check(0));
        d.click(x, y, &t);
        let (x, y) = at(&d, &t, Hit::Plus(1));
        assert_eq!(d.click(x, y, &t), IndClick::None);
        assert_eq!(d.vals[1], 9.0);
        let (x, y) = at(&d, &t, Hit::Check(0));
        d.click(x, y, &t);
        let (x, y) = at(&d, &t, Hit::Plus(1));
        assert_eq!(d.click(x, y, &t), IndClick::Changed);
        assert_eq!(d.vals[1], 10.0);
        // source : les 9 prix dans l'ordre de TradingView (Ouverture, Haut, Bas, Clôture, Volume…), puis la sortie
        // d'un autre indicateur ; ‹ depuis Clôture : Bas, et depuis Ouverture (la première) : la dernière option
        let (x, y) = at(&d, &t, Hit::Minus(3));
        assert_eq!(d.click(x, y, &t), IndClick::Changed);
        assert_eq!(d.vals[3], 3.0, "‹ depuis Clôture : Bas");
        for _ in 0..3 {
            d.click(x, y, &t);
        }
        assert_eq!(d.vals[3], source_ext(0, 1), "‹ depuis Ouverture : la dernière option");
        assert_eq!(d.field_txt(3), "Argos TVD Pro : AI");
        // prix tapé
        let (x, y) = at(&d, &t, Hit::Field(4));
        d.click(x, y, &t);
        (0..10).for_each(|_| d.backspace());
        "81250.5".chars().for_each(|c| d.type_char(c));
        assert_eq!(d.enter(), IndClick::Changed);
        assert_eq!(d.vals[4], 81_250.5);
        // heure tapée (UTC)
        let (x, y) = at(&d, &t, Hit::Field(5));
        d.click(x, y, &t);
        (0..20).for_each(|_| d.backspace());
        "2026-10-27 09:30".chars().for_each(|c| d.type_char(c));
        assert_eq!(d.enter(), IndClick::Changed);
        assert_eq!(time_txt(d.vals[5]), "2026-10-27 09:30");
        // session : mal écrite = gardée telle quelle ; bien écrite = changée
        let (x, y) = at(&d, &t, Hit::Field(6));
        d.click(x, y, &t);
        (0..12).for_each(|_| d.backspace());
        "2500-9999".chars().for_each(|c| d.type_char(c));
        assert_eq!(d.enter(), IndClick::None);
        let (x, y) = at(&d, &t, Hit::Field(6));
        d.click(x, y, &t);
        (0..12).for_each(|_| d.backspace());
        "0800-1700".chars().for_each(|c| d.type_char(c));
        assert_eq!(d.enter(), IndClick::Changed);
        assert_eq!(session_txt(d.vals[6]), "0800-1700");
        // onglet Textes : texte, symbole (mis en majuscules), texte sur plusieurs lignes (Entrée = nouvelle ligne)
        let (x, y) = at(&d, &t, Hit::Tab(1));
        d.click(x, y, &t);
        let (x, y) = at(&d, &t, Hit::Field(7));
        d.click(x, y, &t);
        " go".chars().for_each(|c| d.type_char(c));
        assert_eq!(d.enter(), IndClick::Changed);
        assert_eq!(d.texts["note"], "OK go");
        let (x, y) = at(&d, &t, Hit::Field(8));
        d.click(x, y, &t);
        (0..5).for_each(|_| d.backspace());
        "sol".chars().for_each(|c| d.type_char(c));
        d.enter();
        assert_eq!(d.texts["cmp"], "SOL");
        let (x, y) = at(&d, &t, Hit::Field(9));
        d.click(x, y, &t);
        assert_eq!(d.enter(), IndClick::None, "Entrée : nouvelle ligne");
        "XRP".chars().for_each(|c| d.type_char(c));
        let (x, y) = at(&d, &t, Hit::Field(10));
        assert_eq!(d.click(x, y, &t), IndClick::Changed, "quitter le champ valide le texte");
        assert_eq!(d.texts["syms"], "BTC,ETH\nXRP");
        assert!(d.draw(&t, (0.0, 0.0)).is_some());
        // Défauts ▸ Réinitialiser : nombres et textes reviennent
        let (x, y) = at(&d, &t, Hit::Defaults);
        d.click(x, y, &t);
        let (x, y) = at(&d, &t, Hit::Reset);
        d.click(x, y, &t);
        assert_eq!((d.vals.clone(), d.texts.len()), (defaults(&ALL), 0));
    }
}
#[cfg(test)]
mod tests_ai_trend {
    use super::*;
    use crate::indicators::Ind;
    #[test]
    /// 15 onglets (sections 2 et 3 du Pine comprises) : sur 3 lignes, tous dans la fenêtre.
    fn ai_15_onglets() {
        let t = Text::default();
        let p = Ind::AiTrend.params().1;
        let d = IndDialog::new(Ind::AiTrend, argos_script::inputs::defaults(p), 1400.0, 860.0);
        let (tabs, _) = d.tabs(&t);
        assert_eq!(tabs.len(), 15);
        assert!(tabs.iter().all(|(q, _)| q.x + q.w <= d.rect.x + d.rect.w));
        assert!(d.draw(&t, (0.0, 0.0)).is_some());
    }

    /// 📱 Sur cell (390 × 844), la fenêtre des réglages d'un oscillateur tient dans l'écran et se dessine.
    #[test]
    fn reglages_sur_cell() {
        let ind = *Ind::ALL.iter().find(|i| i.oscillator() && !i.params().1.is_empty()).unwrap();
        let d = IndDialog::new(ind, crate::indicators::values(&crate::indicators::Settings::new(), ind), 390.0, 844.0);
        assert!(d.rect.x >= 0.0 && d.rect.x + d.rect.w <= 390.0 && d.rect.y + d.rect.h <= 844.0, "{:?}", d.rect);
        assert!(d.draw(&Text::default(), (0.0, 0.0)).is_some());
    }
}
