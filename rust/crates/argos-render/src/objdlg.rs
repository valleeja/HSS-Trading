//! 🎨 Fenêtre de réglages d'un objet dessiné (double-clic, ou clic droit → Configurations…).
//! - Objets : **Style** (couleur, épaisseur, plein / tirets / points, verrouiller), **Texte** (texte, taille),
//!   **Coordonnées** (prix de chaque point), **Visibilité** (unités de temps) — comme Argos v1.
//! - Positions longue / courte : **Paramètres en Entrée** (compte, lot, risque en % ou en devise, entrée, cible,
//!   stop), **Style** (lignes, couleur du stop, de l'objectif, texte et sa taille, étiquettes de prix, Stats à cocher,
//!   mode compact, toujours afficher) et **Visibilité** — comme TradingView.
//! - Bas : **⋯** (Sauvegarder sous…, tes modèles, Appliquer les paramètres par défaut), Annuler, D'accord.
//!
//! Les changements se voient en direct ; Annuler remet l'objet comme avant. Les listes (couleurs, tailles, Stats, ⋯)
//! s'ouvrent par-dessus la fenêtre. Une table de zones (`layout`) sert au dessin et au clic.

use crate::drawings::{Drawing, Style};
use crate::palette as pal;
use crate::palette::{GREY, INK, MUTED, SNOW, WHITE};
use crate::position::{is_position, STATS};
use crate::shapes::{lines, round_fill, round_stroke, Rect};
use crate::text::{Align, Rgb, Text};
use tiny_skia::Pixmap;

const W: f64 = 440.0;
const H: f64 = 600.0;
const PAD: f64 = 18.0;
const ROW: f64 = 46.0;

/// Texte tab: the alignment choices.
const VALIGN: [&str; 3] = ["En haut", "Au milieu", "En bas"];
const HALIGN: [&str; 3] = ["Gauche", "Centre", "Droite"];
/// Tailles de texte (liste comme TradingView).
const SIZES: [f32; 13] = [8.0, 10.0, 11.0, 12.0, 14.0, 16.0, 18.0, 20.0, 22.0, 24.0, 28.0, 32.0, 40.0];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Style,
    Text,
    Coords,
    Vis,
    /// Positions : Paramètres en Entrée.
    Inputs,
    /// Outil Icône : la grille d'émojis et la taille.
    Icon,
}

fn tabs(d: &Drawing) -> &'static [(Tab, &'static str)] {
    if is_position(&d.tool) {
        &[(Tab::Inputs, "Paramètres en Entrée"), (Tab::Style, "Style"), (Tab::Vis, "Visibilité")]
    } else if d.tool == "icon" {
        &[(Tab::Icon, "Icône"), (Tab::Coords, "Coordonnées"), (Tab::Vis, "Visibilité")]
    } else {
        &[
            (Tab::Style, "Style"),
            (Tab::Text, "Texte"),
            (Tab::Coords, "Coordonnées"),
            (Tab::Vis, "Visibilité"),
        ]
    }
}

/// Quelle couleur une liste de couleurs règle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Which {
    Line,
    Stop,
    Target,
    Text,
}

/// Case à cocher des positions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Check {
    Lock,
    PriceLabels,
    Compact,
    Always,
    /// Price Action : ligne médiane, bordure, FVG inversé à la clôture.
    Median,
    Border,
    Invert,
}

/// Outils Price Action (FVG, Order Block, liquidité) : leur onglet Style a les réglages de LuxAlgo.
fn is_pa(tool: &str) -> bool {
    matches!(tool, "fvg" | "ob" | "liq")
}

/// Choix « Prolonger à droite » des outils Price Action.
const EXTEND: [&str; 3] = ["Jusqu'au comblement", "Toujours", "Non"];

/// Champ numérique.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Num {
    /// Prix du point `i` (objets) ; positions : 0 entrée, 1 stop, 2 cible.
    Price(usize),
    Account,
    Lot,
    Risk,
    /// Coordonnées: the bar of the point `i` (0 = the last bar, negative to the left).
    Bar(usize),
    /// Visibilité: the from / to of a unit.
    VisMin(usize),
    VisMax(usize),
}

/// Liste ouverte par-dessus la fenêtre.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pop {
    Size,
    /// Texte: vertical alignment, horizontal alignment.
    VAlign,
    HAlign,
    Stats,
    More,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Hit {
    Close,
    /// ✎ rename the drawing.
    Rename,
    Tab(Tab),
    Swatch(Which),
    Check(Check),
    TextBox,
    SizeBtn,
    /// Texte: B, I, the two alignment lists.
    Bold,
    Italic,
    VAlignBtn,
    HAlignBtn,
    StatsBtn,
    /// Price Action : prolonger à droite (cycle des choix).
    Extend,
    Field(Num),
    /// ↑ / ↓ of the price of point `i` (one step of its last decimal).
    Step(usize, bool),
    RiskUnit(bool),
    /// Visibilité: a unit shown or not; its two-handle slider (the click's x moves the nearer handle).
    VisOn(usize),
    VisSlider(usize),
    /// Émoji de la grille (index dans toutes les sections à la suite).
    Emoji(usize),
    More,
    Cancel,
    Ok,
    // dans une liste ouverte
    PopSize(usize),
    /// An alignment chosen (1 to 3).
    PopV(u8),
    PopH(u8),
    PopStat(usize),
    MoreSave,
    MoreTemplate(usize),
    /// ✕ of a saved template.
    MoreRemove(usize),
    MoreDefault,
}

/// Résultat d'un clic.
#[derive(Debug, Clone, PartialEq)]
pub enum DlgClick {
    /// À côté ou dans une zone vide.
    None,
    /// L'objet a changé (redessiner la chart).
    Changed,
    /// Fermer : `true` = D'accord (garder), `false` = Annuler (remettre comme avant).
    Close(bool),
    /// ⋯ → Sauvegarder sous… : garder ce style sous ce nom (pour cet outil).
    SaveAs(String),
    /// ⋯ → un de tes modèles (index dans `templates`).
    Template(usize),
    /// ✕ of a saved template (index in `templates`).
    RemoveTemplate(usize),
    /// 🎨 A colour button: open the shared palette for it, under this button (window coordinates).
    Palette(Which, Rect),
}

/// Champ où l'on tape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Focus {
    Text,
    Num(Num),
    /// Nom du modèle (Sauvegarder sous…).
    Name,
    /// ✎ The drawing's own name (header).
    Rename,
}

/// La fenêtre ouverte.
#[derive(Debug, Clone, PartialEq)]
pub struct ObjDialog {
    /// Index de l'objet dans les dessins.
    pub index: usize,
    /// L'objet avant les changements (Annuler).
    pub backup: Drawing,
    pub tab: Tab,
    pub rect: Rect,
    focus: Option<Focus>,
    /// Texte du champ en cours de saisie (nombre ou nom du modèle).
    edit: String,
    pop: Option<Pop>,
    /// Unité de temps affichée (bouton « Seulement … »).
    pub current_tf: i64,
    /// Noms de tes modèles pour cet outil (⋯).
    pub templates: Vec<String>,
    /// Coordonnées: the time of the last bar (bar 0), seconds.
    pub bar0: i64,
    /// x of the last click (window coordinates): where a Visibilité slider was touched.
    last_x: f64,
}

impl ObjDialog {
    /// Fenêtre centrée dans la fenêtre (`win_w` × `win_h`).
    pub fn new(index: usize, d: &Drawing, current_tf: i64, win_w: f64, win_h: f64) -> Self {
        let rect = Rect::new(
            ((win_w - W) / 2.0).max(6.0),
            ((win_h - H) / 2.0).max(6.0),
            W.min(win_w - 12.0),
            H.min(win_h - 12.0),
        );
        let tab = tabs(d)[0].0;
        Self {
            index,
            backup: d.clone(),
            tab,
            rect,
            focus: None,
            edit: String::new(),
            pop: None,
            current_tf,
            templates: vec![],
            bar0: 0,
            last_x: 0.0,
        }
    }

    /// Ouvre directement un onglet (ex. Visibilité depuis le clic droit).
    pub fn with_tab(mut self, t: Tab, d: &Drawing) -> Self {
        if tabs(d).iter().any(|(k, _)| *k == t) {
            self.tab = t;
        }
        self
    }

    fn at(&self, x: f64, y: f64, w: f64, h: f64) -> Rect {
        Rect::new(self.rect.x + x, self.rect.y + y, w, h)
    }

    fn more_rect(&self) -> Rect {
        self.at(PAD, self.rect.h - 54.0, 124.0, 36.0)
    }

    /// The header's title: the drawing's name, or its tool's.
    fn title(d: &Drawing) -> &str {
        if d.style.name.is_empty() {
            crate::tools::def(&d.tool).map_or(d.tool.as_str(), |t| t.label)
        } else {
            &d.style.name
        }
    }

    /// Zones de la fenêtre (coordonnées de la fenêtre).
    fn layout(&self, d: &Drawing) -> Vec<(Rect, Hit)> {
        let mut out = vec![(self.at(self.rect.w - 44.0, 12.0, 30.0, 30.0), Hit::Close)];
        let tl = tabs(d);
        let mut x = PAD;
        let text = Text::default();
        let tw = Text::graisse(crate::text::Graisse::Bold).width(Self::title(d), 19.0) as f64;
        out.push((self.at(PAD + tw + 6.0, 16.0, 28.0, 28.0), Hit::Rename));
        for (t, name) in tl {
            let tw = text.width(name, 15.0) as f64 + 22.0;
            out.push((self.at(x, 54.0, tw, 34.0), Hit::Tab(*t)));
            x += tw + 4.0;
        }
        let top = 106.0;
        let right = self.rect.w - PAD;
        let pos = is_position(&d.tool);
        match self.tab {
            Tab::Style if pos => {
                out.push((self.at(PAD + 170.0, top, 92.0, 34.0), Hit::Swatch(Which::Line)));
                out.push((self.at(PAD + 170.0, top + ROW, 34.0, 34.0), Hit::Swatch(Which::Stop)));
                out.push((self.at(PAD + 170.0, top + 2.0 * ROW, 34.0, 34.0), Hit::Swatch(Which::Target)));
                out.push((self.at(PAD + 170.0, top + 3.0 * ROW, 34.0, 34.0), Hit::Swatch(Which::Text)));
                out.push((self.at(PAD + 214.0, top + 3.0 * ROW, 120.0, 34.0), Hit::SizeBtn));
                out.push((self.at(PAD, top + 4.0 * ROW + 6.0, 260.0, 26.0), Hit::Check(Check::PriceLabels)));
                out.push((self.at(PAD + 170.0, top + 5.0 * ROW + 28.0, right - PAD - 170.0, 34.0), Hit::StatsBtn));
                out.push((self.at(PAD, top + 6.0 * ROW + 34.0, 300.0, 26.0), Hit::Check(Check::Compact)));
                out.push((self.at(PAD, top + 7.0 * ROW + 24.0, 300.0, 26.0), Hit::Check(Check::Always)));
            }
            Tab::Style if is_pa(&d.tool) => {
                out.push((self.at(PAD + 190.0, top, 34.0, 34.0), Hit::Swatch(Which::Target)));
                out.push((self.at(PAD + 190.0, top + ROW, 34.0, 34.0), Hit::Swatch(Which::Stop)));
                out.push((self.at(PAD + 190.0, top + 2.0 * ROW, 200.0, 34.0), Hit::Extend));
                // liquidité : une ligne (pas de médiane ni de bordure) ; FVG : inverser à la clôture en plus
                let checks: &[Check] = match d.tool.as_str() {
                    "fvg" => &[Check::Median, Check::Border, Check::Invert, Check::Lock],
                    "ob" => &[Check::Median, Check::Border, Check::Lock],
                    _ => &[Check::Lock],
                };
                for (k, c) in checks.iter().enumerate() {
                    out.push((self.at(PAD, top + (3 + k) as f64 * ROW + 6.0, 300.0, 26.0), Hit::Check(*c)));
                }
            }
            Tab::Style => {
                // (Jee's mockup: « Droite » and its colour button — square and line —, then « Étiquette de prix » for
                // the lines that show their price on the scale; width and line style are in the palette)
                out.push((self.at(PAD + 76.0, top, 92.0, 34.0), Hit::Swatch(Which::Line)));
                if matches!(d.tool.as_str(), "hline" | "hray" | "cross") {
                    out.push((self.at(PAD, top + ROW + 14.0, 240.0, 26.0), Hit::Check(Check::PriceLabels)));
                }
            }
            Tab::Icon => {
                out.push((self.at(self.rect.w - PAD - 120.0, top - 6.0, 120.0, 32.0), Hit::SizeBtn));
                for (k, (_, i, j)) in emoji_cells().enumerate() {
                    out.push((
                        self.at(
                            PAD + j as f64 * EMO,
                            top + 36.0 + i as f64 * 22.0 + (k / EMO_ROW) as f64 * EMO,
                            EMO - 4.0,
                            EMO - 4.0,
                        ),
                        Hit::Emoji(k),
                    ));
                }
            }
            Tab::Text => {
                // (Jee's mockup: colour, size, B, I on one row; the text; « Alignement du texte » and its two lists)
                out.push((self.at(PAD, top, 34.0, 34.0), Hit::Swatch(Which::Text)));
                out.push((self.at(PAD + 44.0, top, 80.0, 34.0), Hit::SizeBtn));
                out.push((self.at(PAD + 134.0, top, 34.0, 34.0), Hit::Bold));
                out.push((self.at(PAD + 174.0, top, 34.0, 34.0), Hit::Italic));
                out.push((self.at(PAD, top + 46.0, self.rect.w - 2.0 * PAD, 110.0), Hit::TextBox));
                out.push((self.at(PAD, top + 196.0, 150.0, 34.0), Hit::VAlignBtn));
                out.push((self.at(PAD + 160.0, top + 196.0, 150.0, 34.0), Hit::HAlignBtn));
            }
            Tab::Coords => {
                // (Jee's mockup: « #1 (prix, barre) », the price with ↑↓, the bar)
                for i in 0..d.pts.len().min(7) {
                    let y = top + i as f64 * ROW;
                    out.push((self.at(right - 240.0 + 124.0, y, 26.0, 17.0), Hit::Step(i, true)));
                    out.push((self.at(right - 240.0 + 124.0, y + 17.0, 26.0, 17.0), Hit::Step(i, false)));
                    out.push((self.at(right - 240.0, y, 150.0, 34.0), Hit::Field(Num::Price(i))));
                    out.push((self.at(right - 80.0, y, 80.0, 34.0), Hit::Field(Num::Bar(i))));
                }
            }
            Tab::Inputs => {
                let rows = [Num::Account, Num::Lot, Num::Risk, Num::Price(0), Num::Price(2), Num::Price(1)];
                for (k, n) in rows.iter().enumerate() {
                    let w = if *n == Num::Risk { 110.0 } else { 160.0 };
                    let x = if *n == Num::Risk { right - 160.0 - 60.0 + 10.0 } else { right - 160.0 };
                    out.push((self.at(x, top + k as f64 * ROW, w, 34.0), Hit::Field(*n)));
                }
                out.push((self.at(right - 48.0, top + 2.0 * ROW, 24.0, 34.0), Hit::RiskUnit(true)));
                out.push((self.at(right - 24.0, top + 2.0 * ROW, 24.0, 34.0), Hit::RiskUnit(false)));
            }
            Tab::Vis => {
                // (Jee's mockup, without Ticks and Ranges — Argos has no tick or range charts yet): per unit, a box,
                // « from », a two-handle slider, « to »
                for u in 0..crate::drawings::VIS_UNITS.len() {
                    let y = top + u as f64 * ROW;
                    out.push((self.at(PAD, y + 4.0, 120.0, 26.0), Hit::VisOn(u)));
                    out.push((self.at(PAD + 124.0, y, 70.0, 34.0), Hit::Field(Num::VisMin(u))));
                    out.push((self.at(PAD + 204.0, y, right - PAD - 204.0 - 80.0, 34.0), Hit::VisSlider(u)));
                    out.push((self.at(right - 70.0, y, 70.0, 34.0), Hit::Field(Num::VisMax(u))));
                }
            }
        }
        out.push((self.more_rect(), Hit::More));
        out.push((self.at(self.rect.w - PAD - 220.0, self.rect.h - 54.0, 104.0, 36.0), Hit::Cancel));
        out.push((self.at(self.rect.w - PAD - 108.0, self.rect.h - 54.0, 108.0, 36.0), Hit::Ok));
        out
    }

    /// Bouton qui a ouvert la liste (elle s'ouvre sous lui, ou au-dessus pour ⋯).
    fn pop_anchor(&self, d: &Drawing, p: Pop) -> Option<Rect> {
        let want = match p {
            Pop::Size => Hit::SizeBtn,
            Pop::VAlign => Hit::VAlignBtn,
            Pop::HAlign => Hit::HAlignBtn,
            Pop::Stats => Hit::StatsBtn,
            Pop::More => Hit::More,
        };
        self.layout(d).into_iter().find(|(_, h)| *h == want).map(|(r, _)| r)
    }

    /// Zones de la liste ouverte (coordonnées de la fenêtre).
    fn pop_layout(&self, d: &Drawing) -> Vec<(Rect, Hit)> {
        let Some(p) = self.pop else { return vec![] };
        let Some(a) = self.pop_anchor(d, p) else { return vec![] };
        let mut out = vec![];
        match p {
            Pop::Size => {
                for i in 0..SIZES.len() {
                    out.push((Rect::new(a.x + 4.0, a.y + a.h + 8.0 + i as f64 * 30.0, a.w - 8.0, 28.0), Hit::PopSize(i)));
                }
            }
            Pop::VAlign | Pop::HAlign => {
                for k in 1..=3u8 {
                    let r = Rect::new(a.x + 4.0, a.y + a.h + 8.0 + (k - 1) as f64 * 30.0, a.w - 8.0, 28.0);
                    out.push((r, if p == Pop::VAlign { Hit::PopV(k) } else { Hit::PopH(k) }));
                }
            }
            Pop::Stats => {
                for i in 0..STATS.len() {
                    out.push((
                        Rect::new(a.x - 60.0, a.y - 8.0 - (STATS.len() - i) as f64 * 30.0, a.w + 60.0, 28.0),
                        Hit::PopStat(i),
                    ));
                }
            }
            Pop::More => {
                // (Jee's mockup: « Sauvegarder sous… », « Appliquer les paramètres par défaut », a line, the saved
                // templates — a click applies, ✕ removes)
                let mut y = a.y + a.h + 8.0;
                out.push((Rect::new(a.x, y, 300.0, 30.0), Hit::MoreSave));
                y += 32.0;
                out.push((Rect::new(a.x, y, 300.0, 30.0), Hit::MoreDefault));
                y += 40.0;
                for i in 0..self.templates.len() {
                    out.push((Rect::new(a.x + 268.0, y, 32.0, 30.0), Hit::MoreRemove(i)));
                    out.push((Rect::new(a.x, y, 300.0, 30.0), Hit::MoreTemplate(i)));
                    y += 32.0;
                }
            }
        }
        out
    }

    /// Cadre de la liste ouverte.
    fn pop_frame(&self, d: &Drawing) -> Option<Rect> {
        let z = self.pop_layout(d);
        let (x0, y0) = z.iter().fold((f64::MAX, f64::MAX), |(x, y), (r, _)| (x.min(r.x), y.min(r.y)));
        let (x1, y1) = z.iter().fold((f64::MIN, f64::MIN), |(x, y), (r, _)| (x.max(r.x + r.w), y.max(r.y + r.h)));
        (!z.is_empty()).then(|| Rect::new(x0 - 8.0, y0 - 8.0, x1 - x0 + 16.0, y1 - y0 + 16.0))
    }

    /// Clic (coordonnées de la fenêtre) ; change l'objet `d` en direct.
    /// Barre du titre (sans le ✕) : on l'attrape pour déplacer la fenêtre.
    pub fn title_bar(&self) -> Rect {
        Rect::new(self.rect.x, self.rect.y, self.rect.w - 52.0, 46.0)
    }

    pub fn click(&mut self, x: f64, y: f64, d: &mut Drawing) -> DlgClick {
        self.last_x = x;
        self.apply_edit(d);
        // liste ouverte : elle reçoit le clic ; à côté, elle se ferme
        if self.pop.is_some() {
            let hit = self.pop_layout(d).into_iter().find(|(r, _)| r.contains(x, y)).map(|(_, h)| h);
            let inside = self.pop_frame(d).is_some_and(|f| f.contains(x, y));
            if let Some(h) = hit {
                return self.apply(h, d);
            }
            if inside {
                return DlgClick::None;
            }
            self.pop = None;
            if !self.rect.contains(x, y) {
                return DlgClick::None;
            }
        }
        // à côté : fermer en gardant les changements, comme la fenêtre de réglages d'un indicateur
        if !self.rect.contains(x, y) {
            return DlgClick::Close(true);
        }
        let Some((_, hit)) = self.layout(d).into_iter().find(|(r, _)| r.contains(x, y)) else {
            self.focus = None;
            return DlgClick::None;
        };
        self.apply(hit, d)
    }

    fn apply(&mut self, hit: Hit, d: &mut Drawing) -> DlgClick {
        let st = &mut d.style;
        if !matches!(hit, Hit::PopStat(_) | Hit::MoreSave) {
            self.focus = None;
        }
        match hit {
            Hit::Close | Hit::Cancel => return DlgClick::Close(false),
            Hit::Ok => return DlgClick::Close(true),
            Hit::Tab(t) => {
                self.tab = t;
                return DlgClick::None;
            }
            Hit::Swatch(w) => {
                self.pop = None;
                let r = self.layout(d).into_iter().find(|(_, h)| *h == Hit::Swatch(w)).map_or(self.rect, |(r, _)| r);
                return DlgClick::Palette(w, r);
            }
            Hit::Rename => {
                self.focus = Some(Focus::Rename);
                return DlgClick::None;
            }
            Hit::SizeBtn => {
                self.pop = if self.pop == Some(Pop::Size) { None } else { Some(Pop::Size) };
                return DlgClick::None;
            }
            Hit::StatsBtn => {
                self.pop = if self.pop == Some(Pop::Stats) { None } else { Some(Pop::Stats) };
                return DlgClick::None;
            }
            Hit::More => {
                self.pop = if self.pop == Some(Pop::More) { None } else { Some(Pop::More) };
                return DlgClick::None;
            }
            Hit::PopSize(i) => {
                st.tsize = SIZES[i];
                self.pop = None;
            }
            Hit::Bold => st.bold = !st.bold,
            Hit::Italic => st.italic = !st.italic,
            Hit::VAlignBtn | Hit::HAlignBtn => {
                let p = if hit == Hit::VAlignBtn { Pop::VAlign } else { Pop::HAlign };
                self.pop = if self.pop == Some(p) { None } else { Some(p) };
                return DlgClick::None;
            }
            Hit::PopV(k) => {
                st.valign = k;
                self.pop = None;
            }
            Hit::PopH(k) => {
                st.halign = k;
                self.pop = None;
            }
            Hit::PopStat(i) => st.stats ^= 1 << i, // la liste reste ouverte pour en cocher plusieurs
            Hit::MoreSave => {
                self.focus = Some(Focus::Name);
                self.edit.clear();
                return DlgClick::None;
            }
            Hit::MoreTemplate(i) => {
                self.pop = None;
                return DlgClick::Template(i);
            }
            Hit::MoreRemove(i) => {
                if i < self.templates.len() {
                    self.templates.remove(i);
                }
                return DlgClick::RemoveTemplate(i);
            }
            Hit::MoreDefault => {
                // style par défaut de l'outil ; le texte, la visibilité et le verrou restent
                let keep = (st.text.clone(), st.intervals.clone(), st.locked);
                *st = Style::default();
                (st.text, st.intervals, st.locked) = keep;
                self.pop = None;
            }
            Hit::Emoji(k) => {
                if let Some((e, _, _)) = emoji_cells().nth(k) {
                    st.text = e.to_string();
                }
            }
            Hit::Check(c) => match c {
                Check::Lock => st.locked = !st.locked,
                Check::PriceLabels => st.price_labels = !st.price_labels,
                Check::Compact => st.compact = !st.compact,
                Check::Always => st.stats_always = !st.stats_always,
                Check::Median => st.median = !st.median,
                Check::Border => st.border = !st.border,
                Check::Invert => st.invert = !st.invert,
            },
            Hit::Extend => st.extend = (st.extend + 1) % EXTEND.len() as u8,
            Hit::TextBox => {
                self.focus = Some(Focus::Text);
                return DlgClick::None;
            }
            Hit::Field(n) => {
                self.focus = Some(Focus::Num(n));
                self.edit = match n {
                    Num::Price(i) => d.pts.get(i).map_or(String::new(), |q| num(q.p)),
                    Num::Bar(i) => d.pts.get(i).map_or(String::new(), |q| self.bar_of(q.t).to_string()),
                    Num::VisMin(u) => d.vis_shown()[u].1.to_string(),
                    Num::VisMax(u) => d.vis_shown()[u].2.to_string(),
                    Num::Account => num(st.account),
                    Num::Lot => num(st.lot),
                    Num::Risk => num(st.risk),
                };
                return DlgClick::None;
            }
            Hit::Step(i, up) => {
                if let Some(q) = d.pts.get_mut(i) {
                    let s = num(q.p);
                    let dec = s.split('.').nth(1).map_or(0, |f| f.len()) as i32;
                    let step = 10f64.powi(-dec);
                    q.p = ((q.p + if up { step } else { -step }) * 10f64.powi(dec)).round() / 10f64.powi(dec);
                }
            }
            Hit::RiskUnit(pct) => {
                // même risque en devise, exprimé dans l'autre unité
                if pct != st.risk_pct && st.account > 0.0 {
                    st.risk = if pct { st.risk / st.account * 100.0 } else { st.account * st.risk / 100.0 };
                }
                st.risk_pct = pct;
            }
            Hit::VisOn(u) => {
                let mut v = d.vis_shown();
                v[u].0 = !v[u].0;
                d.style.vis = v;
            }
            Hit::VisSlider(u) => {
                if let Some((r, _)) = self.layout(d).into_iter().find(|(_, h)| *h == Hit::VisSlider(u)) {
                    let max = crate::drawings::VIS_UNITS[u].1 as f64;
                    let n = (1.0 + ((self.last_x - r.x - 8.0) / (r.w - 16.0)).clamp(0.0, 1.0) * (max - 1.0)).round() as u16;
                    let mut v = d.vis_shown();
                    let (_, a, b) = v[u];
                    if (n as i32 - a as i32).abs() <= (n as i32 - b as i32).abs() {
                        v[u].1 = n.min(b);
                    } else {
                        v[u].2 = n.max(a);
                    }
                    d.style.vis = v;
                }
            }
        }
        DlgClick::Changed
    }

    /// Le nombre tapé passe dans l'objet (s'il se lit).
    fn apply_edit(&mut self, d: &mut Drawing) {
        let Some(Focus::Num(n)) = self.focus else { return };
        let Ok(v) = self.edit.trim().replace(',', ".").replace(' ', "").parse::<f64>() else {
            return;
        };
        if let Num::VisMin(u) | Num::VisMax(u) = n {
            let max = crate::drawings::VIS_UNITS[u].1;
            let k = (v.round().clamp(1.0, max as f64)) as u16;
            let mut vis = d.vis_shown();
            if matches!(n, Num::VisMin(_)) {
                vis[u].1 = k.min(vis[u].2);
            } else {
                vis[u].2 = k.max(vis[u].1);
            }
            d.style.vis = vis;
            return;
        }
        if let Num::Bar(i) = n {
            // a bar number: the point goes to that bar's time
            if let (Some(q), true) = (d.pts.get_mut(i), v.is_finite()) {
                q.t = self.bar0 + v.round() as i64 * self.current_tf.max(1);
            }
            return;
        }
        if !v.is_finite() || v < 0.0 {
            return;
        }
        if is_position(&d.tool) {
            crate::position::normalize(d);
        }
        match n {
            Num::Price(i) if v > 0.0 => {
                if let Some(q) = d.pts.get_mut(i) {
                    q.p = v;
                }
            }
            Num::Account => d.style.account = v,
            Num::Lot if v > 0.0 => d.style.lot = v,
            Num::Risk => d.style.risk = v,
            _ => {}
        }
    }

    /// The bar of time `t` from the last bar (0), in units of the chart's timeframe.
    fn bar_of(&self, t: i64) -> i64 {
        ((t - self.bar0) as f64 / self.current_tf.max(1) as f64).round() as i64
    }

    /// Un champ attend le clavier ?
    pub fn typing(&self) -> bool {
        self.focus.is_some()
    }

    pub fn type_char(&mut self, c: char, d: &mut Drawing) {
        if c.is_control() {
            return;
        }
        match self.focus {
            Some(Focus::Text) if d.style.text.chars().count() < 200 => d.style.text.push(c),
            Some(Focus::Num(Num::Bar(_))) if c.is_ascii_digit() || (c == '-' && self.edit.is_empty()) => self.edit.push(c),
            Some(Focus::Num(_)) if c.is_ascii_digit() || c == '.' || c == ',' => self.edit.push(c),
            Some(Focus::Name) if self.edit.chars().count() < 40 => self.edit.push(c),
            Some(Focus::Rename) if d.style.name.chars().count() < 60 => d.style.name.push(c),
            _ => {}
        }
    }

    pub fn backspace(&mut self, d: &mut Drawing) {
        match self.focus {
            Some(Focus::Text) => {
                d.style.text.pop();
            }
            Some(Focus::Rename) => {
                d.style.name.pop();
            }
            Some(Focus::Num(_) | Focus::Name) => {
                self.edit.pop();
            }
            None => {}
        }
    }

    /// Entrée : un nombre tapé s'applique ; le nom d'un modèle → Sauvegarder sous ce nom.
    pub fn enter(&mut self, d: &mut Drawing) -> DlgClick {
        if self.focus == Some(Focus::Name) {
            let name = self.edit.trim().to_string();
            (self.focus, self.pop) = (None, None);
            self.edit.clear();
            return if name.is_empty() { DlgClick::None } else { DlgClick::SaveAs(name) };
        }
        self.apply_edit(d);
        self.focus = None;
        DlgClick::Changed
    }

    /// Dessin : la fenêtre, puis la liste ouverte par-dessus (coordonnées de la fenêtre).
    pub fn draw(&self, text: &Text, d: &Drawing, mouse: (f64, f64)) -> Vec<(Pixmap, Rect)> {
        let mut out = vec![];
        if let Some(main) = self.draw_main(text, d, mouse) {
            out.push(main);
        }
        if let Some(p) = self.draw_pop(text, d, mouse) {
            out.push(p);
        }
        out
    }

    fn draw_main(&self, text: &Text, d: &Drawing, mouse: (f64, f64)) -> Option<(Pixmap, Rect)> {
        let r = self.rect;
        let mut pm = crate::px::new(r.w as u32, r.h as u32)?;
        let loc = |q: Rect| Rect::new(q.x - r.x, q.y - r.y, q.w, q.h);
        crate::jetons::fond_fenetre(&mut pm, loc(r), 12.0);
        round_stroke(&mut pm, loc(r), 12.0, pal::edge_soft(), 1.0);
        // (Jee's mockup: the drawing's name in bold, then ✎ to rename it)
        let name = Self::title(d);
        Text::graisse(crate::text::Graisse::Bold).draw(&mut pm, name, (PAD as f32, 36.0), 19.0, WHITE, Align::Left);
        if self.focus == Some(Focus::Rename) {
            let tw = Text::graisse(crate::text::Graisse::Bold).width(name, 19.0) as f64;
            crate::fill(&mut pm, PAD + tw + 1.0, 18.0, 1.5, 22.0, WHITE, 1.0);
        }
        crate::fill(&mut pm, PAD, 89.0, r.w - 2.0 * PAD, 2.0, pal::edge_soft(), 1.0);
        let hover = |q: &Rect| q.contains(mouse.0, mouse.1) && self.pop.is_none();
        let st = &d.style;
        let top = 106.0f32;
        let lab = |pm: &mut Pixmap, s: &str, y: f32| text.draw(pm, s, (PAD as f32, y), 14.0, WHITE, Align::Left);
        let small = |pm: &mut Pixmap, s: &str, y: f32| text.draw(pm, s, (PAD as f32, y), 11.5, GREY, Align::Left);
        let pos = is_position(&d.tool);
        let row = ROW as f32;
        match self.tab {
            Tab::Style if pos => {
                lab(&mut pm, "Lignes", top + 22.0);
                lab(&mut pm, "Couleur du stop", top + row + 22.0);
                lab(&mut pm, "Couleur de l'objectif", top + 2.0 * row + 22.0);
                lab(&mut pm, "Texte", top + 3.0 * row + 22.0);
                small(&mut pm, "INFO", top + 5.0 * row + 10.0);
                lab(&mut pm, "Stats", top + 5.0 * row + 50.0);
            }
            Tab::Style if is_pa(&d.tool) => {
                lab(&mut pm, "Haussier", top + 22.0);
                lab(&mut pm, "Baissier", top + row + 22.0);
                lab(&mut pm, "Prolonger à droite", top + 2.0 * row + 22.0);
            }
            Tab::Style => {
                lab(&mut pm, "Droite", top + 22.0);
            }
            Tab::Text => lab(&mut pm, "Alignement du texte", top + 186.0),
            Tab::Icon => {
                lab(&mut pm, "Taille", top + 16.0);
                for (i, (name, _)) in crate::emoji::SECTIONS.iter().enumerate() {
                    let y = top + 30.0 + i as f32 * (22.0 + 2.0 * EMO as f32);
                    text.draw(&mut pm, &name.to_uppercase(), (PAD as f32, y + 2.0), 11.0, GREY, Align::Left);
                }
            }
            Tab::Coords => {
                for i in 0..d.pts.len().min(7) {
                    lab(&mut pm, &format!("#{} (prix, barre)", i + 1), top + 22.0 + i as f32 * row);
                }
            }
            Tab::Inputs => {
                let names = [
                    "Taille du compte",
                    "Taille du lot",
                    "Risque",
                    "Prix d'entrée",
                    "Niveau de profit (prix)",
                    "Niveau de stop (prix)",
                ];
                for (k, n) in names.iter().enumerate() {
                    lab(&mut pm, n, top + 22.0 + k as f32 * row);
                }
                // résumé en direct
                let mut dd = d.clone();
                crate::position::normalize(&mut dd);
                let c = crate::position::calc(&dd, None, 1.0);
                let s = format!(
                    "Qté {:.3} · risque {} · gain {} · RR {:.2}",
                    c.qty,
                    crate::viewport::format_price(c.risk, 2),
                    crate::viewport::format_price(c.reward, 2),
                    c.rr
                );
                small(&mut pm, &s, top + 6.0 * row + 16.0);
            }
            Tab::Vis => {}
        }
        for (q, hit) in self.layout(d) {
            let lq = loc(q);
            let h = hover(&q);
            let (cx, cy) = (lq.cx() as f32, (lq.cy() + 5.0) as f32);
            let button = |pm: &mut Pixmap, s: &str, on: bool| {
                round_fill(
                    pm,
                    lq,
                    6.0,
                    if on {
                        pal::hover()
                    } else if h {
                        pal::tile()
                    } else {
                        pal::field()
                    },
                    1.0,
                );
                round_stroke(pm, lq, 6.0, if on { GREY } else { pal::edge_soft() }, 1.0);
                text.draw(pm, s, (cx, cy), 13.5, WHITE, Align::Center);
            };
            match hit {
                Hit::Close => {
                    if h {
                        round_fill(&mut pm, lq, 6.0, pal::hover(), 1.0);
                    }
                    lines(
                        &mut pm,
                        lq.cx(),
                        lq.cy(),
                        &[&[(-6.0, -6.0), (6.0, 6.0)], &[(6.0, -6.0), (-6.0, 6.0)]],
                        GREY,
                        1.6,
                    );
                }
                Hit::Tab(t) => {
                    let on = self.tab == t;
                    let label = tabs(d).iter().find(|(k, _)| *k == t).map_or("", |(_, l)| *l);
                    text.draw(&mut pm, label, (cx, cy), 15.0, if on || h { WHITE } else { GREY }, Align::Center);
                    if on {
                        crate::fill(&mut pm, lq.x + 8.0, 88.0, lq.w - 16.0, 3.0, WHITE, 1.0);
                    }
                }
                Hit::Swatch(w) => {
                    let (s, def) = match w {
                        Which::Line => (&st.color, if pos { "#787b86" } else { "#2962ff" }),
                        Which::Stop => (&st.stop_color, crate::position::DEFAULT_STOP),
                        Which::Target => (&st.target_color, crate::position::DEFAULT_TARGET),
                        Which::Text => (&st.tcolor, "#ffffff"),
                    };
                    let c = crate::colorpick::split(s).map_or(Rgb::hex(def), |p| p.0);
                    round_fill(&mut pm, lq, 6.0, pal::field(), 1.0);
                    round_stroke(&mut pm, lq, 6.0, if h { GREY } else { pal::edge_soft() }, 1.0);
                    round_fill(&mut pm, Rect::new(lq.x + 7.0, lq.y + 7.0, 20.0, 20.0), 4.0, c, 1.0);
                    if w == Which::Line {
                        crate::fill(&mut pm, lq.x + 38.0, lq.cy(), 44.0, (st.width as f64).clamp(1.0, 4.0), c, 1.0);
                    }
                }
                Hit::Check(c) => {
                    let (on, label) = match c {
                        Check::Lock => (st.locked, "Verrouiller le dessin"),
                        Check::PriceLabels => (st.price_labels, "Étiquettes de prix"),
                        Check::Compact => (st.compact, "Mode statistiques compact"),
                        Check::Always => (st.stats_always, "Toujours afficher les statistiques"),
                        Check::Median => (st.median, "Ligne médiane"),
                        Check::Border => (st.border, "Bordure"),
                        Check::Invert => (st.invert, "Inverser à la clôture (iFVG)"),
                    };
                    check(&mut pm, text, lq, on, label);
                }
                Hit::TextBox => {
                    let on = self.focus == Some(Focus::Text);
                    round_fill(&mut pm, lq, 6.0, pal::field(), 1.0);
                    round_stroke(&mut pm, lq, 6.0, if on { crate::palette::accent() } else { pal::edge_soft() }, 1.2);
                    let (s, col) = if st.text.is_empty() && !on {
                        ("Ajouter un texte", MUTED)
                    } else {
                        (st.text.as_str(), WHITE)
                    };
                    text.draw(&mut pm, s, ((lq.x + 10.0) as f32, (lq.y + 24.0) as f32), 14.0, col, Align::Left);
                    if on {
                        crate::fill(&mut pm, lq.x + 11.0 + text.width(&st.text, 14.0) as f64, lq.y + 10.0, 1.5, 18.0, WHITE, 1.0);
                    }
                }
                Hit::SizeBtn => dropdown(&mut pm, text, lq, &format!("{}", st.tsize), h || self.pop == Some(Pop::Size)),
                Hit::Bold | Hit::Italic => {
                    let bold = hit == Hit::Bold;
                    button(&mut pm, "", if bold { st.bold } else { st.italic });
                    let mut t = if bold { Text::graisse(crate::text::Graisse::Bold) } else { Text::default() };
                    t.italic = !bold;
                    t.draw(&mut pm, if bold { "B" } else { "I" }, (cx, cy), 15.0, WHITE, Align::Center);
                }
                Hit::VAlignBtn | Hit::HAlignBtn => {
                    let (v, hz) = d.text_align();
                    let label = if hit == Hit::VAlignBtn {
                        VALIGN[(v.clamp(1, 3) - 1) as usize]
                    } else {
                        HALIGN[(hz.clamp(1, 3) - 1) as usize]
                    };
                    let open = self.pop == Some(if hit == Hit::VAlignBtn { Pop::VAlign } else { Pop::HAlign });
                    dropdown(&mut pm, text, lq, label, h || open);
                }
                Hit::Extend => dropdown(&mut pm, text, lq, EXTEND[(st.extend as usize).min(EXTEND.len() - 1)], h),
                Hit::StatsBtn => {
                    let on: Vec<&str> = STATS.iter().enumerate().filter(|(i, _)| st.stats & (1 << i) != 0).map(|(_, s)| *s).collect();
                    let s = if on.is_empty() { "Aucune".to_string() } else { on.join(", ") };
                    let s = text.fit(&s, 13.5, lq.w - 40.0);
                    dropdown(&mut pm, text, lq, &s, h || self.pop == Some(Pop::Stats));
                }
                Hit::Field(n) => {
                    let on = self.focus == Some(Focus::Num(n));
                    round_fill(&mut pm, lq, 6.0, pal::field(), 1.0);
                    round_stroke(
                        &mut pm,
                        lq,
                        6.0,
                        if on {
                            crate::palette::accent()
                        } else if h {
                            GREY
                        } else {
                            pal::edge_soft()
                        },
                        1.2,
                    );
                    let s = if on {
                        self.edit.clone()
                    } else {
                        match n {
                            Num::Price(i) => d.pts.get(i).map_or_else(
                                || {
                                    if i == 2 && is_position(&d.tool) {
                                        num(crate::position::calc(d, None, 1.0).target)
                                    } else {
                                        String::new()
                                    }
                                },
                                |q| num(q.p),
                            ),
                            Num::Bar(i) => d.pts.get(i).map_or(String::new(), |q| self.bar_of(q.t).to_string()),
                            Num::VisMin(u) => d.vis_shown()[u].1.to_string(),
                            Num::VisMax(u) => d.vis_shown()[u].2.to_string(),
                            Num::Account => crate::viewport::format_price(st.account, 2),
                            Num::Lot => num(st.lot),
                            Num::Risk => num(st.risk),
                        }
                    };
                    text.draw(&mut pm, &s, ((lq.x + lq.w - 10.0) as f32, cy), 14.0, WHITE, Align::Right);
                }
                Hit::Step(_, up) => {
                    let ch: [(f32, f32); 3] = if up {
                        [(-3.5, 2.0), (0.0, -1.5), (3.5, 2.0)]
                    } else {
                        [(-3.5, -2.0), (0.0, 1.5), (3.5, -2.0)]
                    };
                    lines(&mut pm, lq.cx(), lq.cy(), &[&ch], if h { WHITE } else { GREY }, 1.3);
                }
                Hit::RiskUnit(pct) => {
                    let on = st.risk_pct == pct;
                    round_fill(&mut pm, lq, 4.0, if on { pal::hover() } else { pal::field() }, 1.0);
                    text.draw(
                        &mut pm,
                        if pct { "%" } else { "$" },
                        (cx, cy),
                        14.0,
                        if on { WHITE } else { GREY },
                        Align::Center,
                    );
                }
                Hit::VisOn(u) => check(&mut pm, text, lq, d.vis_shown()[u].0, crate::drawings::VIS_UNITS[u].0),
                Hit::VisSlider(u) => {
                    // a light track between the two handles, the rest dim
                    let (on, a, b) = d.vis_shown()[u];
                    let max = crate::drawings::VIS_UNITS[u].1 as f64;
                    let tx = |n: u16| lq.x + 8.0 + (n as f64 - 1.0) / (max - 1.0).max(1.0) * (lq.w - 16.0);
                    crate::fill(&mut pm, lq.x + 8.0, lq.cy() - 2.0, lq.w - 16.0, 4.0, pal::edge_soft(), 1.0);
                    crate::fill(&mut pm, tx(a), lq.cy() - 2.0, tx(b) - tx(a), 4.0, if on { WHITE } else { GREY }, 0.85);
                    for n in [a, b] {
                        let k = Rect::new(tx(n) - 7.0, lq.cy() - 7.0, 14.0, 14.0);
                        round_fill(&mut pm, k, 7.0, WHITE, 1.0);
                        round_stroke(&mut pm, k, 7.0, Rgb(0x20, 0x22, 0x28), 1.5);
                    }
                }
                Hit::More => {
                    // « Modèle ⌄ » (Jee's mockup; its frame lit while its list is open)
                    let on = self.pop == Some(Pop::More);
                    round_fill(&mut pm, lq, 8.0, pal::field(), 1.0);
                    round_stroke(&mut pm, lq, 8.0, if on { crate::palette::accent() } else { pal::edge_soft() }, 1.4);
                    text.draw(&mut pm, "Modèle", ((lq.x + 12.0) as f32, (lq.cy() + 5.0) as f32), 14.0, WHITE, Align::Left);
                    let (vx, vy) = (lq.x + lq.w - 18.0, lq.cy());
                    let chev: [(f32, f32); 3] = if on {
                        [(-4.0, 2.0), (0.0, -2.0), (4.0, 2.0)]
                    } else {
                        [(-4.0, -2.0), (0.0, 2.0), (4.0, -2.0)]
                    };
                    lines(&mut pm, vx, vy, &[&chev], WHITE, 1.4);
                }
                Hit::Rename => {
                    if h || self.focus == Some(Focus::Rename) {
                        round_fill(&mut pm, lq, 6.0, pal::hover(), 1.0);
                    }
                    crate::phosphor::draw(&mut pm, "pencil-simple", lq.cx(), lq.cy(), 17, GREY);
                }
                Hit::Emoji(k) => {
                    if let Some((e, _, _)) = emoji_cells().nth(k) {
                        let on = d.emoji() == e;
                        if on || h {
                            round_fill(
                                &mut pm,
                                lq,
                                6.0,
                                if on { crate::palette::accent() } else { pal::hover() },
                                if on { 0.35 } else { 1.0 },
                            );
                        }
                        if let Some(img) = crate::emoji::image(e, crate::px::dev(24.0) as u32) {
                            crate::px::put(&mut pm, lq.cx() - 12.0, lq.cy() - 12.0, &img);
                        }
                    }
                }
                Hit::Cancel => button(&mut pm, "Annuler", false),
                Hit::Ok => {
                    round_fill(&mut pm, lq, 8.0, if h { Rgb(0xd6, 0xd8, 0xde) } else { SNOW }, 1.0);
                    text.draw(&mut pm, "D'accord", (cx, cy), 14.5, INK, Align::Center);
                }
                _ => {}
            }
        }
        // Sauvegarder sous… : le nom se tape dans le bas de la fenêtre
        if self.focus == Some(Focus::Name) {
            let q = loc(self.at(PAD + 64.0, self.rect.h - 54.0, self.rect.w - 2.0 * PAD - 64.0 - 228.0, 36.0));
            round_fill(&mut pm, q, 6.0, pal::field(), 1.0);
            round_stroke(&mut pm, q, 6.0, crate::palette::accent(), 1.2);
            let s = if self.edit.is_empty() { "Nom du modèle…" } else { self.edit.as_str() };
            text.draw(
                &mut pm,
                s,
                ((q.x + 8.0) as f32, (q.cy() + 5.0) as f32),
                13.0,
                if self.edit.is_empty() { GREY } else { WHITE },
                Align::Left,
            );
        }
        Some((pm, r))
    }

    fn draw_pop(&self, text: &Text, d: &Drawing, mouse: (f64, f64)) -> Option<(Pixmap, Rect)> {
        let f = self.pop_frame(d)?;
        let mut pm = crate::px::new(f.w as u32, f.h as u32)?;
        let loc = |q: Rect| Rect::new(q.x - f.x, q.y - f.y, q.w, q.h);
        round_fill(&mut pm, loc(f), 8.0, pal::popup(), 1.0);
        round_stroke(&mut pm, loc(f), 8.0, pal::edge_soft(), 1.0);
        let st = &d.style;
        for (q, hit) in self.pop_layout(d) {
            let lq = loc(q);
            let h = q.contains(mouse.0, mouse.1);
            let row = |pm: &mut Pixmap, s: &str, on: bool| {
                if on {
                    round_fill(pm, lq, 6.0, SNOW, 1.0);
                } else if h {
                    round_fill(pm, lq, 6.0, pal::hover(), 1.0);
                }
                text.draw(
                    pm,
                    s,
                    ((lq.x + 10.0) as f32, (lq.cy() + 5.0) as f32),
                    14.0,
                    if on { INK } else { WHITE },
                    Align::Left,
                );
            };
            match hit {
                Hit::PopSize(i) => row(&mut pm, &format!("{}", SIZES[i]), (st.tsize - SIZES[i]).abs() < 0.01),
                Hit::PopV(k) => row(&mut pm, VALIGN[(k - 1) as usize], d.text_align().0 == k),
                Hit::PopH(k) => row(&mut pm, HALIGN[(k - 1) as usize], d.text_align().1 == k),
                Hit::PopStat(i) => {
                    if h {
                        round_fill(&mut pm, lq, 6.0, pal::hover(), 1.0);
                    }
                    check(&mut pm, text, Rect::new(lq.x + 8.0, lq.y, lq.w - 8.0, lq.h), st.stats & (1 << i) != 0, STATS[i]);
                }
                Hit::MoreSave => row(&mut pm, "Sauvegarder sous…", false),
                Hit::MoreTemplate(i) => row(&mut pm, &self.templates[i], false),
                Hit::MoreRemove(_) => lines(
                    &mut pm,
                    lq.cx(),
                    lq.cy(),
                    &[&[(-4.5, -4.5), (4.5, 4.5)], &[(4.5, -4.5), (-4.5, 4.5)]],
                    GREY,
                    1.5,
                ),
                Hit::MoreDefault => row(&mut pm, "Appliquer les paramètres par défaut", false),
                _ => {}
            }
        }
        Some((pm, f))
    }
}

/// Nombre lisible et retapable : 2 décimales au-dessus de 1 000, 4 au-dessus de 1, sinon 8 ; sans zéros inutiles.
fn num(v: f64) -> String {
    let dec = if v.abs() >= 1000.0 {
        2
    } else if v.abs() >= 1.0 {
        4
    } else {
        8
    };
    let s = format!("{v:.dec$}");
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s
    }
}

/// Taille d'une case de la grille d'émojis, et émojis par rangée.
const EMO: f64 = 40.0;
const EMO_ROW: usize = 8;

/// Les émojis de la grille : (émoji, section, colonne), 8 par rangée, section après section.
fn emoji_cells() -> impl Iterator<Item = (&'static str, usize, usize)> {
    crate::emoji::SECTIONS
        .iter()
        .enumerate()
        .flat_map(|(i, (_, l))| l.iter().enumerate().map(move |(j, e)| (*e, i, j % EMO_ROW)))
}

/// Case à cocher + son texte.
fn check(pm: &mut Pixmap, text: &Text, r: Rect, on: bool, label: &str) {
    let b = Rect::new(r.x, r.cy() - 9.0, 18.0, 18.0);
    round_fill(pm, b, 3.0, if on { WHITE } else { pal::field() }, 1.0);
    round_stroke(pm, b, 3.0, if on { WHITE } else { GREY }, 1.2);
    if on {
        lines(pm, b.cx(), b.cy(), &[&[(-4.5, 0.0), (-1.0, 3.5), (5.0, -4.0)]], INK, 2.0);
    }
    text.draw(pm, label, ((r.x + 28.0) as f32, (r.cy() + 5.0) as f32), 14.0, WHITE, Align::Left);
}

/// Liste déroulante fermée : sa valeur et ⌄.
fn dropdown(pm: &mut Pixmap, text: &Text, r: Rect, value: &str, active: bool) {
    round_fill(pm, r, 6.0, pal::field(), 1.0);
    round_stroke(pm, r, 6.0, if active { GREY } else { pal::edge_soft() }, 1.0);
    text.draw(pm, value, ((r.x + 10.0) as f32, (r.cy() + 5.0) as f32), 13.5, WHITE, Align::Left);
    lines(pm, r.x + r.w - 16.0, r.cy(), &[&[(-4.0, -2.0), (0.0, 2.0), (4.0, -2.0)]], GREY, 1.4);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drawings::Point;

    fn rect_of(dl: &ObjDialog, d: &Drawing, h: Hit) -> Rect {
        dl.layout(d).into_iter().chain(dl.pop_layout(d)).find(|(_, x)| *x == h).unwrap().0
    }

    fn press(dl: &mut ObjDialog, d: &mut Drawing, h: Hit) -> DlgClick {
        let r = rect_of(dl, d, h);
        dl.click(r.cx(), r.cy(), d)
    }

    #[test]
    fn icone_grille_d_emojis() {
        let mut d = Drawing::new("icon", vec![Point { t: 0, p: 1.0 }]);
        let mut dl = ObjDialog::new(0, &d, 1800, 1400.0, 900.0);
        assert_eq!(dl.tab, Tab::Icon, "icône : la grille d'abord");
        assert_eq!(d.emoji(), crate::emoji::DEFAULT);
        assert_eq!(press(&mut dl, &mut d, Hit::Emoji(1)), DlgClick::Changed);
        assert_eq!(d.emoji(), "📈");
        press(&mut dl, &mut d, Hit::Emoji(16)); // 1re de la 2e section
        assert_eq!(d.emoji(), "✅");
        // toutes les cases dans la fenêtre, au-dessus des boutons du bas
        let bottom = dl.rect.y + dl.rect.h - 54.0;
        assert!(dl
            .layout(&d)
            .iter()
            .all(|(r, h)| !matches!(h, Hit::Emoji(_)) || (r.y + r.h <= bottom && dl.rect.contains(r.x, r.y))));
        assert!(!dl.draw(&Text::default(), &d, (0.0, 0.0)).is_empty());
    }

    #[test]
    fn reglages_price_action_comme_luxalgo() {
        let mut d = Drawing::new("fvg", vec![Point { t: 0, p: 1.0 }]);
        let mut dl = ObjDialog::new(0, &d, 1800, 1400.0, 900.0);
        assert!(
            dl.layout(&d).iter().any(|(_, h)| *h == Hit::Check(Check::Invert)),
            "FVG : inverser à la clôture"
        );
        press(&mut dl, &mut d, Hit::Extend);
        assert_eq!(d.style.extend, 1, "jusqu'au comblement → toujours");
        press(&mut dl, &mut d, Hit::Check(Check::Median));
        press(&mut dl, &mut d, Hit::Check(Check::Invert));
        assert!(!d.style.median && d.style.invert);
        assert!(
            matches!(press(&mut dl, &mut d, Hit::Swatch(Which::Target)), DlgClick::Palette(Which::Target, _)),
            "the shared palette"
        );
        let ob = Drawing::new("ob", vec![Point { t: 0, p: 1.0 }]);
        assert!(
            !ObjDialog::new(0, &ob, 1800, 1400.0, 900.0)
                .layout(&ob)
                .iter()
                .any(|(_, h)| *h == Hit::Check(Check::Invert)),
            "pas d'iFVG pour un OB"
        );
        assert!(!dl.draw(&Text::default(), &d, (0.0, 0.0)).is_empty());
    }

    #[test]
    fn nombres_lisibles() {
        assert_eq!(num(81807.67105084745), "81807.67");
        assert_eq!(num(1.0), "1");
        assert_eq!(num(0.000123456789), "0.00012346");
    }

    #[test]
    fn objet_style_texte_coordonnees_visibilite() {
        let mut d = Drawing::new("trend", vec![Point { t: 1_700_000_000, p: 100.0 }, Point { t: 1_700_003_600, p: 120.0 }]);
        let mut dl = ObjDialog::new(0, &d, 1800, 1400.0, 900.0);
        assert_eq!(dl.tab, Tab::Style);
        // the colour button opens the shared palette (with width and line style) under itself
        let DlgClick::Palette(w, r) = press(&mut dl, &mut d, Hit::Swatch(Which::Line)) else {
            panic!("palette")
        };
        assert!(w == Which::Line && r.w > 50.0);
        // ✎ renames the drawing (the header shows its name)
        press(&mut dl, &mut d, Hit::Rename);
        "Ma ligne".chars().for_each(|c| dl.type_char(c, &mut d));
        assert_eq!((d.style.name.as_str(), ObjDialog::title(&d)), ("Ma ligne", "Ma ligne"));
        // texte
        press(&mut dl, &mut d, Hit::Tab(Tab::Text));
        press(&mut dl, &mut d, Hit::TextBox);
        "Achat!".chars().for_each(|c| dl.type_char(c, &mut d));
        dl.backspace(&mut d);
        assert_eq!(d.style.text, "Achat");
        press(&mut dl, &mut d, Hit::SizeBtn);
        press(&mut dl, &mut d, Hit::PopSize(0));
        assert_eq!(d.style.tsize, 8.0);
        // B, I, the alignments (a line: auto = above and centred)
        press(&mut dl, &mut d, Hit::Bold);
        press(&mut dl, &mut d, Hit::Italic);
        assert!(d.style.bold && d.style.italic);
        assert_eq!(d.text_align(), (1, 2));
        press(&mut dl, &mut d, Hit::VAlignBtn);
        press(&mut dl, &mut d, Hit::PopV(3));
        press(&mut dl, &mut d, Hit::HAlignBtn);
        press(&mut dl, &mut d, Hit::PopH(3));
        assert_eq!(d.text_align(), (3, 3));
        assert!(matches!(press(&mut dl, &mut d, Hit::Swatch(Which::Text)), DlgClick::Palette(Which::Text, _)));
        // coordonnées
        press(&mut dl, &mut d, Hit::Tab(Tab::Coords));
        press(&mut dl, &mut d, Hit::Field(Num::Price(1)));
        (0..10).for_each(|_| dl.backspace(&mut d));
        "123,5".chars().for_each(|c| dl.type_char(c, &mut d));
        dl.enter(&mut d);
        assert_eq!(d.pts[1].p, 123.5);
        // ↑ one step of the last decimal; the bar field moves the point in time
        press(&mut dl, &mut d, Hit::Step(1, true));
        assert_eq!(d.pts[1].p, 123.6);
        dl.bar0 = 1_700_000_000;
        press(&mut dl, &mut d, Hit::Field(Num::Bar(0)));
        (0..10).for_each(|_| dl.backspace(&mut d));
        "-4".chars().for_each(|c| dl.type_char(c, &mut d));
        dl.enter(&mut d);
        assert_eq!(d.pts[0].t, 1_700_000_000 - 4 * 1800);
        // visibilité
        press(&mut dl, &mut d, Hit::Tab(Tab::Vis));
        assert_eq!(d.vis_shown()[1], (true, 1, 59), "not touched: every minute");
        // Minutes off: hidden on 30m, still on 4h
        press(&mut dl, &mut d, Hit::VisOn(1));
        assert!(!d.visible_at(1800) && d.visible_at(14_400));
        // Hours 2 to 8: shown on 4h, not on 1h or 12h
        press(&mut dl, &mut d, Hit::Field(Num::VisMin(2)));
        (0..5).for_each(|_| dl.backspace(&mut d));
        "2".chars().for_each(|c| dl.type_char(c, &mut d));
        press(&mut dl, &mut d, Hit::Field(Num::VisMax(2)));
        (0..5).for_each(|_| dl.backspace(&mut d));
        "8".chars().for_each(|c| dl.type_char(c, &mut d));
        dl.enter(&mut d);
        assert!(d.visible_at(14_400) && !d.visible_at(3600) && !d.visible_at(43_200));
        // the old list read as ranges, untouched until the sliders move
        let mut old = Drawing::new("trend", vec![]);
        old.style.intervals = vec![14_400, 3600];
        assert_eq!(old.vis_shown()[2], (true, 1, 4));
        assert!(
            !old.vis_shown()[1].0 && old.visible_at(3600) && !old.visible_at(7200),
            "the old list still decides"
        );
        assert!(!dl.draw(&Text::default(), &d, (0.0, 0.0)).is_empty());
        assert_eq!(press(&mut dl, &mut d, Hit::Cancel), DlgClick::Close(false));
        assert_eq!(press(&mut dl, &mut d, Hit::Ok), DlgClick::Close(true));
    }

    #[test]
    fn position_parametres_style_et_points() {
        let mut d = Drawing::new("long", vec![Point { t: 0, p: 100.0 }, Point { t: 3600, p: 95.0 }]);
        crate::position::normalize(&mut d);
        let mut dl = ObjDialog::new(0, &d, 1800, 1400.0, 900.0);
        assert_eq!(dl.tab, Tab::Inputs, "positions : Paramètres en Entrée d'abord");
        // risque 1 % → 100 $ (compte 10 000)
        press(&mut dl, &mut d, Hit::RiskUnit(false));
        assert!(!d.style.risk_pct && (d.style.risk - 100.0).abs() < 1e-9);
        press(&mut dl, &mut d, Hit::Field(Num::Price(2)));
        (0..10).for_each(|_| dl.backspace(&mut d));
        "120".chars().for_each(|c| dl.type_char(c, &mut d));
        dl.enter(&mut d);
        assert_eq!(d.pts[2].p, 120.0, "niveau de profit");
        // style : couleur du stop, Stats (décocher Qté), compact, toujours afficher
        press(&mut dl, &mut d, Hit::Tab(Tab::Style));
        assert!(matches!(press(&mut dl, &mut d, Hit::Swatch(Which::Stop)), DlgClick::Palette(Which::Stop, _)));
        press(&mut dl, &mut d, Hit::StatsBtn);
        press(&mut dl, &mut d, Hit::PopStat(6));
        assert_eq!(d.style.stats & (1 << 6), 0, "Qté décochée, la liste reste ouverte");
        assert!(dl.pop.is_some());
        press(&mut dl, &mut d, Hit::Check(Check::Compact));
        assert!(d.style.compact);
        // ⋯ : sauvegarder sous un nom, puis paramètres par défaut
        press(&mut dl, &mut d, Hit::More);
        press(&mut dl, &mut d, Hit::MoreSave);
        "Scalp".chars().for_each(|c| dl.type_char(c, &mut d));
        assert_eq!(dl.enter(&mut d), DlgClick::SaveAs("Scalp".into()));
        dl.templates = vec!["Scalp".into()];
        press(&mut dl, &mut d, Hit::More);
        assert_eq!(press(&mut dl, &mut d, Hit::MoreTemplate(0)), DlgClick::Template(0));
        press(&mut dl, &mut d, Hit::More);
        assert_eq!(press(&mut dl, &mut d, Hit::MoreRemove(0)), DlgClick::RemoveTemplate(0));
        assert!(dl.templates.is_empty(), "✕ removes it");
        press(&mut dl, &mut d, Hit::More);
        press(&mut dl, &mut d, Hit::MoreDefault);
        assert!(!d.style.compact && d.style.stop_color.is_empty());
        assert_eq!(dl.draw(&Text::default(), &d, (0.0, 0.0)).len(), 1);
    }
}
