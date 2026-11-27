//! ✏️ Moteur d'objets dessinés — **le même modèle qu'Argos v1** : un objet = l'id de son outil (table `tools`), ses
//! points (temps + prix, pour rester justes quand on zoome, défile ou change d'échelle) et un style commun (couleur,
//! épaisseur, tirets, verrouillé, caché, texte, RR). Toucher, poignées, déplacer et dessiner sont écrits **une fois**
//! ici ; chaque famille d'outils n'a que sa façon de se dessiner et de se toucher. Les outils pas encore dessinés à
//! leur façon (phase C) se montrent en ligne brisée par leurs points, avec leur nom.

use crate::rail::Magnet;
use crate::shapes::{lines, poly_fill, round_fill, round_stroke, Rect};
use crate::text::{Align, Rgb, Text};
use crate::toolgeom::Geom;
use crate::viewport::{PriceScale, Slots, Viewport};
use crate::{fill, LINE_BLUE};
use argos_core::Bar;
use serde::{Deserialize, Serialize};
use tiny_skia::Pixmap;

/// Distance (pixels) à laquelle la souris « touche » un objet ou une poignée.
pub const HIT_PX: f64 = 5.0;
pub const HANDLE_PX: f64 = 7.0;
/// 👆 The same by the finger (B-019, Jee, 25 Nov 2026: « même pas capable de sélectionner le fib »): a finger covers
/// about 40 points, 5 were out of its reach. A handle: 44 points across, Apple's touch target.
pub const HIT_DOIGT: f64 = 14.0;
pub const HANDLE_DOIGT: f64 = 22.0;

/// Visibilité units: name, largest count, seconds per count.
pub const VIS_UNITS: [(&str, u16, i64); 6] = [
    ("Secondes", 59, 1),
    ("Minutes", 59, 60),
    ("Heures", 24, 3600),
    ("Jours", 366, 86_400),
    ("Semaines", 52, 604_800),
    ("Mois", 12, argos_core::types::MONTH_SECONDS),
];

/// The unit and count of a timeframe (`tf` seconds), e.g. 4h → (hours, 4).
pub fn vis_unit(tf: i64) -> Option<(usize, u16)> {
    let m = argos_core::types::MONTH_SECONDS;
    let (u, n) = if tf >= m && tf % m == 0 {
        (5, tf / m)
    } else if tf >= 604_800 && tf % 604_800 == 0 {
        (4, tf / 604_800)
    } else if tf >= 86_400 && tf % 86_400 == 0 {
        (3, tf / 86_400)
    } else if tf >= 3600 && tf % 3600 == 0 {
        (2, tf / 3600)
    } else if tf >= 60 && tf % 60 == 0 {
        (1, tf / 60)
    } else if tf > 0 {
        (0, tf)
    } else {
        return None;
    };
    Some((u, n.clamp(1, u16::MAX as i64) as u16))
}

/// Un point d'un objet : temps (secondes UTC) et vrai prix.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub t: i64,
    pub p: f64,
}

/// Style commun à tous les objets.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Style {
    /// Couleur « #rrggbb » (vide : couleur de l'outil).
    pub color: String,
    pub width: f32,
    /// Trait en tirets, ou en points.
    pub dash: bool,
    pub dotted: bool,
    pub locked: bool,
    pub hidden: bool,
    /// Texte (outil Texte, notes…) et sa taille.
    pub text: String,
    pub tsize: f32,
    /// Unités de temps (secondes) où le dessin s'affiche ; vide = toutes.
    pub intervals: Vec<i64>,
    /// Ratio risque / rendement des positions longue / courte (cible au placement).
    pub rr: f64,
    /// Positions (module `position`) : taille du compte, risque (en % du compte ou en devise), taille du lot.
    pub account: f64,
    pub risk: f64,
    pub risk_pct: bool,
    pub lot: f64,
    /// Positions : couleurs du stop, de l'objectif, du texte (vide = par défaut).
    pub stop_color: String,
    pub target_color: String,
    pub tcolor: String,
    /// Positions : prix sur l'échelle, stats affichées (cases, `position::STATS`), mode compact, toujours afficher.
    pub price_labels: bool,
    pub stats: u16,
    pub compact: bool,
    /// Toujours afficher les stats ; sinon seulement quand la souris est sur la position (comme TradingView).
    pub stats_always: bool,
    /// Price Action (FVG, OB, liquidité ; couleurs : objectif = haussier, stop = baissier) : prolonger jusqu'au
    /// comblement (0), toujours (1), pas du tout (2) ; ligne médiane ; bordure ; FVG inversé à la clôture.
    pub extend: u8,
    pub median: bool,
    pub border: bool,
    pub invert: bool,
    /// 🖍 Bar ⋯ : 0 no sync, 1 sync in the layout, 2 global sync (the choice is kept; copying to other charts: soon).
    pub sync: u8,
    /// ✎ The drawing's own name (settings window header; empty: its tool's name).
    pub name: String,
    /// Texte tab: bold, italic; vertical alignment (0 auto, 1 top, 2 middle, 3 bottom) and horizontal (0 auto,
    /// 1 left, 2 centre, 3 right) — auto keeps each tool's usual place.
    pub bold: bool,
    pub italic: bool,
    pub valign: u8,
    pub halign: u8,
    /// Visibilité: per unit (seconds, minutes, hours, days, weeks, months — `VIS_UNITS`): shown?, from, to. Empty:
    /// `intervals` decides (the old list, kept as it is until the sliders are touched).
    pub vis: Vec<(bool, u16, u16)>,
    /// 🎨 Opacity of the drawing (palette slider), 0–1; the colour itself stays « #rrggbb », so older versions
    /// still read it.
    pub opacity: f32,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            color: String::new(),
            width: 1.5,
            dash: false,
            dotted: false,
            tsize: 14.0,
            intervals: vec![],
            locked: false,
            hidden: false,
            text: String::new(),
            rr: 2.0,
            account: 10_000.0,
            risk: 1.0,
            risk_pct: true,
            lot: 1.0,
            stop_color: String::new(),
            target_color: String::new(),
            tcolor: String::new(),
            price_labels: true,
            stats: crate::position::DEFAULT_STATS,
            compact: false,
            stats_always: false,
            extend: 0,
            median: true,
            border: false,
            invert: false,
            sync: 0,
            name: String::new(),
            bold: false,
            italic: false,
            valign: 0,
            halign: 0,
            vis: vec![],
            opacity: 1.0,
        }
    }
}

/// Un objet dessiné.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(from = "Stored")]
pub struct Drawing {
    pub tool: String,
    pub pts: Vec<Point>,
    pub style: Style,
}

/// Lecture des sauvegardes : le format actuel, ou l'ancien (`{"HLine":{"price":…}}`) qui devient une ligne horizontale.
#[derive(Deserialize)]
#[serde(untagged)]
enum Stored {
    Now {
        tool: String,
        pts: Vec<Point>,
        #[serde(default)]
        style: Box<Style>,
    },
    Old(Legacy),
}

#[derive(Deserialize)]
enum Legacy {
    HLine {
        price: f64,
        #[serde(default)]
        locked: bool,
    },
}

impl From<Stored> for Drawing {
    fn from(s: Stored) -> Self {
        match s {
            Stored::Now { tool, pts, style } => Drawing { tool, pts, style: *style },
            Stored::Old(Legacy::HLine { price, locked }) => {
                let mut d = Drawing::hline(price);
                d.style.locked = locked;
                d
            }
        }
    }
}

/// Géométrie de la chart pour placer les objets : temps ↔ x (avec la place des bougies), prix ↔ y.
pub struct Geo<'a> {
    pub vp: &'a Viewport,
    pub scale: &'a PriceScale,
    pub slots: &'a Slots,
    /// Heure de chaque bougie affichée (même ordre que `slots`).
    pub times: &'a [i64],
    /// Bougies affichées (régression, VWAP, profil de volume, motif de bougies).
    pub bars: &'a [Bar],
    /// Durée d'une bougie (pour le futur, à droite de la dernière).
    pub tf: i64,
    /// Souris sur la chart (étiquettes des positions : seulement quand la souris est dessus).
    pub mouse: Option<(f64, f64)>,
}

impl Geo<'_> {
    /// Abscisse d'un moment (entre deux bougies : proportionnel ; avant / après l'historique : au pas de l'intervalle).
    pub fn x_of(&self, t: i64) -> f64 {
        let n = self.times.len().min(self.slots.len());
        if n == 0 {
            return self.vp.plot_w() / 2.0;
        }
        let tf = self.tf.max(1) as f64;
        let times = &self.times[..n];
        let i = times.partition_point(|&x| x <= t);
        if i == 0 {
            return self.vp.x_of(0, self.slots) - (times[0] - t) as f64 / tf * self.vp.bar_spacing;
        }
        let a = i - 1;
        let xa = self.vp.x_of(a, self.slots);
        if i == n {
            return xa + (t - times[a]) as f64 / tf * self.vp.bar_spacing;
        }
        let k = (t - times[a]) as f64 / (times[i] - times[a]).max(1) as f64;
        xa + (self.vp.x_of(i, self.slots) - xa) * k
    }

    /// Moment sous une abscisse (collé à la bougie la plus proche ; hors historique : au pas de l'intervalle).
    pub fn t_of(&self, x: f64) -> i64 {
        let n = self.times.len().min(self.slots.len());
        if n == 0 {
            return 0;
        }
        let (first, last) = (self.vp.x_of(0, self.slots), self.vp.x_of(n - 1, self.slots));
        let steps = |d: f64| (d / self.vp.bar_spacing).round() as i64;
        if x > last {
            return self.times[n - 1] + steps(x - last) * self.tf;
        }
        if x < first {
            return self.times[0] - steps(first - x) * self.tf;
        }
        // les abscisses montent avec l'index : on cherche la bougie la plus proche par dichotomie
        let (mut lo, mut hi) = (0, n - 1);
        while lo < hi {
            let m = (lo + hi) / 2;
            if self.vp.x_of(m, self.slots) < x {
                lo = m + 1;
            } else {
                hi = m;
            }
        }
        let i = lo;
        let best = if i > 0 && (self.vp.x_of(i - 1, self.slots) - x).abs() < (self.vp.x_of(i, self.slots) - x).abs() {
            i - 1
        } else {
            i
        };
        self.times[best]
    }

    pub fn y_of(&self, p: f64) -> f64 {
        self.scale.y_of(p)
    }

    pub fn point_at(&self, x: f64, y: f64) -> Point {
        Point {
            t: self.t_of(x),
            p: self.scale.price_of(y),
        }
    }

    pub fn xy(&self, q: &Point) -> (f64, f64) {
        (self.x_of(q.t), self.y_of(q.p))
    }
}

/// Ce qu'il faut pour placer les objets, calculé une fois par image (la fenêtre le garde pour la souris).
#[derive(Debug, Clone)]
pub struct Frame {
    pub vp: Viewport,
    pub scale: PriceScale,
    pub slots: Slots,
    pub times: Vec<i64>,
    /// Bougies affichées (même ordre que `times`), pour l'aimant.
    pub bars: Vec<Bar>,
    pub tf: i64,
}

/// Distance (pixels) jusqu'à laquelle l'aimant faible colle au prix O / H / L / C.
pub const WEAK_MAGNET_PX: f64 = 30.0;

impl Frame {
    pub fn geo(&self) -> Geo<'_> {
        Geo {
            mouse: None,
            vp: &self.vp,
            scale: &self.scale,
            slots: &self.slots,
            times: &self.times,
            bars: &self.bars,
            tf: self.tf,
        }
    }

    /// Point sous la souris, collé au plus proche O / H / L / C de la bougie par l'aimant (faible : s'il est à moins de
    /// `WEAK_MAGNET_PX` ; fort : toujours).
    pub fn snap(&self, x: f64, y: f64, magnet: Magnet) -> Point {
        let g = self.geo();
        let mut at = g.point_at(x, y);
        if magnet == Magnet::Off {
            return at;
        }
        let Some(b) = self.times.binary_search(&at.t).ok().and_then(|i| self.bars.get(i)) else {
            return at;
        };
        let best = [b.open, b.high, b.low, b.close]
            .into_iter()
            .min_by(|a, c| (g.y_of(*a) - y).abs().total_cmp(&(g.y_of(*c) - y).abs()));
        if let Some(p) = best.filter(|p| magnet == Magnet::Strong || (g.y_of(*p) - y).abs() <= WEAK_MAGNET_PX) {
            at.p = p;
        }
        at
    }
}

/// Distance d'un point à un segment.
fn seg_dist(px: f64, py: f64, a: (f64, f64), b: (f64, f64)) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let l2 = dx * dx + dy * dy;
    let k = if l2 == 0.0 {
        0.0
    } else {
        (((px - a.0) * dx + (py - a.1) * dy) / l2).clamp(0.0, 1.0)
    };
    (px - (a.0 + k * dx)).hypot(py - (a.1 + k * dy))
}

/// Prolonge le segment a → b loin au-delà du bord (vers b ; `both` : des deux côtés).
fn extend(a: (f64, f64), b: (f64, f64), both: bool, w: f64, h: f64) -> ((f64, f64), (f64, f64)) {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    if dx == 0.0 && dy == 0.0 {
        return (a, b);
    }
    let big = (w + h) * 4.0 / dx.hypot(dy);
    let far = (a.0 + dx * big, a.1 + dy * big);
    let back = if both { (a.0 - dx * big, a.1 - dy * big) } else { a };
    (back, far)
}

/// Niveaux du retracement de Fibonacci : **ta** liste d'Argos v1 (les niveaux cochés par défaut, avec leurs couleurs).
const FIB: [(f64, Rgb); 14] = [
    (0.0, Rgb(0x95, 0x98, 0xa1)),
    (0.214, Rgb(0xff, 0xc1, 0x07)),
    (0.382, Rgb(0xff, 0xc1, 0x07)),
    (0.5, Rgb(0x8a, 0x8f, 0x99)),
    (0.618, Rgb(0xff, 0xc1, 0x07)),
    (0.652, Rgb(0xff, 0xc1, 0x07)),
    (0.718, Rgb(0x9e, 0x9e, 0x9e)),
    (0.882, Rgb(0x9c, 0x27, 0xb0)),
    (1.0, Rgb(0x95, 0x98, 0xa1)),
    (1.414, Rgb(0xf2, 0x36, 0x45)),
    (1.618, Rgb(0xd4, 0xa0, 0x17)),
    (1.657, Rgb(0xff, 0xc1, 0x07)),
    (1.786, Rgb(0xff, 0xc1, 0x07)),
    (1.882, Rgb(0xff, 0x98, 0x00)),
];

/// Familles d'outils qui partagent une forme (une ligne par famille : le reste passe par la ligne brisée).
const BOXES: &[&str] = &["rect", "measure", "pricerange", "daterange", "dprange", "fib"];
const LABELS: &[&str] = &[
    "text",
    "note",
    "pricenote",
    "pin",
    "callout",
    "comment",
    "pricelabel",
    "signpost",
    "flag",
    "icon",
    "table",
];
const MARKS: &[&str] = &["arrowup", "arrowdn", "amup", "amdn", "amleft", "amright"];

/// Flèches pleines (pointe en (0, 0)), vers le haut ; les autres sens sont des rotations.
const ARROW_UP: [(f32, f32); 7] = [(0.0, 0.0), (-9.0, 12.0), (-3.5, 12.0), (-3.5, 24.0), (3.5, 24.0), (3.5, 12.0), (9.0, 12.0)];

impl Drawing {
    pub fn new(tool: &str, pts: Vec<Point>) -> Self {
        Self {
            tool: tool.to_string(),
            pts,
            style: Style::default(),
        }
    }

    /// Style de départ d'un outil, comme Argos v1 : lignes de prix et croix à 1,5, boîtes et Fibonacci à 1, le reste à 2.
    pub fn default_style(tool: &str) -> Style {
        let width = match tool {
            "hline" | "hray" | "vline" | "cross" => 1.5,
            "rect" | "fib" | "fibch" => 1.0,
            _ => 2.0,
        };
        Style { width, ..Style::default() }
    }

    /// Ligne horizontale à un prix (le temps ne compte pas).
    /// The text's alignment as shown (vertical 1 top / 2 middle / 3 bottom, horizontal 1 left / 2 centre / 3 right):
    /// « auto » is each tool's usual place — labels middle-left, lines above and centred.
    pub fn text_align(&self) -> (u8, u8) {
        let label = LABELS.contains(&self.tool.as_str());
        let v = if self.style.valign == 0 {
            if label {
                2
            } else {
                1
            }
        } else {
            self.style.valign
        };
        let h = if self.style.halign == 0 {
            if label {
                1
            } else {
                2
            }
        } else {
            self.style.halign
        };
        (v, h)
    }

    /// 🎨 The drawing's opacity: its `opacity`, times the « aa » of a « #rrggbbaa » colour (saved that way before
    /// the opacity had its own field — still read).
    pub fn alpha(&self) -> f32 {
        let aa = crate::colorpick::split(&self.style.color).map_or(1.0, |p| p.1);
        (self.style.opacity * aa).clamp(0.0, 1.0)
    }

    /// The font of the text when bold or italic is chosen (`None`: the chart's own font).
    fn text_font(&self) -> Option<Text> {
        if !self.style.bold && !self.style.italic {
            return None;
        }
        let mut t = if self.style.bold {
            Text::graisse(crate::text::Graisse::Bold)
        } else {
            Text::default()
        };
        t.italic = self.style.italic;
        Some(t)
    }

    pub fn hline(price: f64) -> Self {
        Self::new("hline", vec![Point { t: 0, p: price }])
    }

    /// Émoji de l'outil Icône (son texte ; vide = celui par défaut).
    pub fn emoji(&self) -> &str {
        if self.style.text.is_empty() {
            crate::emoji::DEFAULT
        } else {
            &self.style.text
        }
    }

    /// Taille de l'icône en pixels : 2,5 × la taille du texte (14 → 35 px).
    pub fn icon_px(&self) -> u32 {
        (self.style.tsize * 2.5).clamp(16.0, 120.0) as u32
    }

    pub fn locked(&self) -> bool {
        self.style.locked
    }

    pub fn set_locked(&mut self, on: bool) {
        self.style.locked = on;
    }

    /// Affiché sur cette unité de temps (onglet Visibilité) ?
    pub fn visible_at(&self, tf: i64) -> bool {
        if self.style.hidden {
            return false;
        }
        if self.style.vis.len() == VIS_UNITS.len() {
            return vis_unit(tf).is_none_or(|(u, n)| self.style.vis[u].0 && (self.style.vis[u].1..=self.style.vis[u].2).contains(&n));
        }
        self.style.intervals.is_empty() || self.style.intervals.contains(&tf)
    }

    /// Visibilité as shown: the ranges, or (not yet touched) the old interval list read as ranges — every unit whole
    /// when it is empty, else each unit from its smallest to its largest listed interval.
    pub fn vis_shown(&self) -> Vec<(bool, u16, u16)> {
        if self.style.vis.len() == VIS_UNITS.len() {
            return self.style.vis.clone();
        }
        VIS_UNITS
            .iter()
            .enumerate()
            .map(|(u, (_, max, _))| {
                if self.style.intervals.is_empty() {
                    return (true, 1, *max);
                }
                let ns: Vec<u16> = self
                    .style
                    .intervals
                    .iter()
                    .filter_map(|t| vis_unit(*t).filter(|x| x.0 == u).map(|x| x.1))
                    .collect();
                match (ns.iter().min(), ns.iter().max()) {
                    (Some(a), Some(b)) => (true, *a, *b),
                    _ => (false, 1, *max),
                }
            })
            .collect()
    }

    /// Motif du trait : (pixels tracés, pixels vides) ; `None` = plein.
    fn pattern(&self) -> Option<(f64, f64)> {
        match (self.style.dash, self.style.dotted) {
            (_, true) => Some((2.0, 4.0)),
            (true, _) => Some((8.0, 5.0)),
            _ => None,
        }
    }

    fn color(&self) -> Rgb {
        if let Some((c, _)) = crate::colorpick::split(&self.style.color) {
            return c;
        }
        match self.tool.as_str() {
            "arrowup" | "amup" | "long" => crate::palette::up(),
            "arrowdn" | "amdn" | "short" => crate::palette::down(),
            _ => LINE_BLUE,
        }
    }

    fn px(&self, g: &Geo) -> Vec<(f64, f64)> {
        self.pts.iter().map(|q| g.xy(q)).collect()
    }

    /// Poignées (pixels) : une par point (lignes horizontales / verticales : au milieu de la chart).
    pub fn handles(&self, g: &Geo) -> Vec<(f64, f64)> {
        if crate::position::is_position(&self.tool) {
            return crate::position::handles(self, g);
        }
        let mut h = self.px(g);
        match self.tool.as_str() {
            "hline" => h.iter_mut().for_each(|q| q.0 = g.vp.plot_w() / 2.0),
            "vline" => h.iter_mut().for_each(|q| q.1 = g.vp.plot_h() / 2.0),
            _ => {}
        }
        h
    }

    /// Poignée sous la souris.
    pub fn handle_at(&self, x: f64, y: f64, g: &Geo) -> Option<usize> {
        self.handle_at_r(x, y, g, HANDLE_PX)
    }

    /// 👆 The handle within `r` points of (`x`, `y`), the closest (by the finger two handles can both be in reach).
    pub fn handle_at_r(&self, x: f64, y: f64, g: &Geo, r: f64) -> Option<usize> {
        self.handles(g)
            .iter()
            .enumerate()
            .map(|(i, (hx, hy))| (i, (hx - x).hypot(hy - y)))
            .filter(|(_, d)| *d <= r)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(i, _)| i)
    }

    /// La souris en (`x`, `y`) touche-t-elle l'objet ?
    pub fn hit(&self, x: f64, y: f64, g: &Geo) -> bool {
        self.hit_r(x, y, g, HIT_PX)
    }

    /// 👆 Does a pointer `tol` points wide at (`x`, `y`) touch the object? The mouse: [`HIT_PX`]; the finger:
    /// [`HIT_DOIGT`] (lines, zones, marks and texts all grow by the same margin).
    pub fn hit_r(&self, x: f64, y: f64, g: &Geo, tol: f64) -> bool {
        let (w, h) = (g.vp.plot_w(), g.vp.plot_h());
        if x < 0.0 || x > w || y < 0.0 || y > h || !self.visible_at(g.tf) {
            return false;
        }
        let p = self.px(g);
        let Some(&a) = p.first() else { return false };
        // (what the mouse reached beyond a line: the finger reaches that much further)
        let plus = (tol - HIT_PX).max(0.0);
        let near = |s: (f64, f64), e: (f64, f64)| seg_dist(x, y, s, e) <= tol;
        let inside =
            |s: (f64, f64), e: (f64, f64)| x >= s.0.min(e.0) - plus && x <= s.0.max(e.0) + plus && y >= s.1.min(e.1) - plus && y <= s.1.max(e.1) + plus;
        let tool = self.tool.as_str();
        match (tool, p.get(1).copied()) {
            ("hline", _) => (a.1 - y).abs() <= tol,
            ("hray", _) => (a.1 - y).abs() <= tol && x >= a.0 - tol,
            ("vline", _) => (a.0 - x).abs() <= tol,
            ("cross", _) => (a.1 - y).abs() <= tol || (a.0 - x).abs() <= tol,
            ("ray" | "ext", Some(b)) => {
                let (s, e) = extend(a, b, tool == "ext", w, h);
                near(s, e)
            }
            ("long" | "short", _) => crate::position::hit(self, x, y, g),
            ("icon", _) => {
                let half = self.icon_px() as f64 / 2.0 + plus;
                (x - a.0).abs() <= half && (y - a.1).abs() <= half
            }
            // ✏️ (B-019) the retracement: its box, every level line (past 1 too: they were out of reach) and every label
            // drawn (« 1.618 (63 751) », left of the lines)
            ("fib", Some(b)) => {
                let (x1, x2, niveaux) = fib_niveaux(&self.pts, a, b, g);
                let etiquettes = fib_etiquettes(&niveaux);
                let sur_ligne = x >= x1 - plus && x <= x2 + plus && niveaux.iter().any(|(_, y0, _)| (y - y0).abs() <= tol);
                // (an 11-point label on its baseline 4 points under its line)
                let sur_etiquette = x >= x1 - FIB_ETIQUETTE_W - plus
                    && x < x1
                    && niveaux
                        .iter()
                        .filter(|(lv, _, _)| etiquettes.contains(&lv.to_bits()))
                        .any(|(_, y0, _)| (y - (y0 - 1.5)).abs() <= FIB_ETIQUETTE_H / 2.0 + plus);
                inside(a, b) || sur_ligne || sur_etiquette
            }
            (t, Some(b)) if BOXES.contains(&t) => inside(a, b),
            (t, _) if LABELS.contains(&t) => {
                let at = Self::text_at(t, &p);
                (x >= at.0 - 4.0 - plus && x <= at.0 + 80.0 + plus && (y - at.1).abs() <= 12.0 + plus) || self.geom(g).is_some_and(|o| geom_hit(&o, x, y, tol))
            }
            (t, _) if MARKS.contains(&t) => (x - a.0).hypot(y - a.1) <= 18.0 + plus,
            _ => match self.geom(g) {
                Some(o) => geom_hit(&o, x, y, tol),
                None => p.windows(2).any(|s| near(s[0], s[1])) || (p.len() == 1 && (x - a.0).hypot(y - a.1) <= tol * 2.0),
            },
        }
    }

    /// Forme de la phase C (module `toolgeom`) ; `None` pour les outils dessinés ici.
    fn geom(&self, g: &Geo) -> Option<Geom> {
        self.shape(g, &|p| format!("{p}"))
    }

    /// La forme de l'objet : Price Action selon ses réglages, sinon la phase C.
    fn shape(&self, g: &Geo, label: &dyn Fn(f64) -> String) -> Option<Geom> {
        crate::toolgeom::price_action(&self.tool, &self.pts, g, &self.style).or_else(|| crate::toolgeom::geom(&self.tool, &self.pts, g, label))
    }

    /// Où s'écrit le texte d'une annotation (bulle et note de prix : au 2e point ; panneau : sur la plaque).
    fn text_at(tool: &str, p: &[(f64, f64)]) -> (f64, f64) {
        let a = p[0];
        match (tool, p.get(1)) {
            ("callout" | "pricenote", Some(b)) => (b.0 + 6.0, b.1 + 22.0),
            ("signpost", _) => (a.0 + 6.0, a.1 - 26.0),
            ("pin" | "flag" | "icon", _) => (a.0 + 14.0, a.1 + 5.0),
            ("table", _) => (a.0, a.1 - 6.0),
            // texte, note, commentaire : le texte commence après la poignée
            _ => (a.0 + 9.0, a.1 + 5.0),
        }
    }

    /// Dessine une forme de la phase C : zones, traits, tirets, étiquettes.
    fn draw_geom(pm: &mut Pixmap, text: &Text, o: &Geom, c: Rgb, lw: f32, pat: Option<(f64, f64)>) {
        let f32s = |v: &[(f64, f64)]| v.iter().map(|q| (q.0 as f32, q.1 as f32)).collect::<Vec<_>>();
        for (poly, col, alpha) in &o.fills {
            poly_fill(pm, 0.0, 0.0, &f32s(poly), col.unwrap_or(c), *alpha);
        }
        let width = if o.thick { 10.0 } else { lw };
        for (poly, col) in &o.lines {
            stroke(pm, poly, col.unwrap_or(c), width, if o.thick { None } else { pat });
        }
        for (poly, col) in &o.dashed {
            for d in crate::toolgeom::dashes(poly, 6.0, 4.0) {
                lines(pm, 0.0, 0.0, &[&f32s(&d)], col.unwrap_or(c), 1.0);
            }
        }
        for l in &o.labels {
            if l.boxed {
                let tw = text.width(&l.text, 11.5) as f64 + 12.0;
                round_fill(pm, Rect::new(l.at.0 - tw / 2.0, l.at.1 - 14.0, tw, 19.0), 4.0, Rgb(0x1e, 0x21, 0x2a), 0.92);
            }
            text.draw(pm, &l.text, (l.at.0 as f32, l.at.1 as f32), 11.5, l.color, l.align);
        }
    }

    /// Déplace tout l'objet de `dx`, `dy` pixels depuis `from` (ses points au moment où on l'a attrapé) ; rien s'il est
    /// verrouillé. Partir de `from` évite que les arrondis au temps des bougies s'accumulent pendant le glisser.
    pub fn move_from(&mut self, from: &[Point], dx: f64, dy: f64, g: &Geo) {
        if self.locked() {
            return;
        }
        let keep_t = self.tool == "hline";
        for (q, f) in self.pts.iter_mut().zip(from) {
            let (x, y) = g.xy(f);
            let to = g.point_at(x + dx, y + dy);
            *q = Point {
                t: if keep_t { f.t } else { to.t },
                p: to.p,
            };
        }
    }

    /// Déplace un point (poignée).
    pub fn move_point(&mut self, i: usize, at: Point) {
        if self.locked() {
            return;
        }
        if crate::position::is_position(&self.tool) {
            return crate::position::move_handle(self, i, at);
        }
        if let Some(q) = self.pts.get_mut(i) {
            *q = at;
        }
    }

    /// Dessine l'objet ; `label` écrit un prix (format du mode : normal, pourcent…) ; `selected` : avec ses poignées.
    /// Prix que l'objet montre sur l'échelle de prix (lignes horizontales, croix, positions) : prix, couleur.
    pub fn axis_labels(&self, g: &Geo) -> Vec<(f64, Rgb)> {
        if !self.visible_at(g.tf) || self.pts.is_empty() {
            return vec![];
        }
        match self.tool.as_str() {
            "hline" | "hray" | "cross" => vec![(self.pts[0].p, self.color())],
            "long" | "short" if self.pts.len() > 1 => crate::position::axis_labels(self, g),
            _ => vec![],
        }
    }

    pub fn draw(&self, pm: &mut Pixmap, text: &Text, g: &Geo, label: &dyn Fn(f64) -> String, selected: bool) {
        // 🎨 the opacity of the palette (« #rrggbbaa »): the drawing on its own layer, laid with that opacity
        let alpha = self.alpha();
        if alpha < 0.999 {
            if let Some(mut layer) = Pixmap::new(pm.width(), pm.height()) {
                self.draw_opaque(&mut layer, text, g, label, selected);
                crate::px::pose(pm, 0, 0, layer.as_ref(), alpha);
                return;
            }
        }
        self.draw_opaque(pm, text, g, label, selected);
    }

    fn draw_opaque(&self, pm: &mut Pixmap, text: &Text, g: &Geo, label: &dyn Fn(f64) -> String, selected: bool) {
        if !self.visible_at(g.tf) || self.pts.is_empty() {
            return;
        }
        let (w, h) = (g.vp.plot_w(), g.vp.plot_h());
        let c = self.color();
        let lw = self.style.width.max(1.0);
        let p = self.px(g);
        let a = p[0];
        // droites horizontales / verticales nettes (au pixel, comme TradingView) ; les autres lissées
        let pat = self.pattern();
        let seg = |pm: &mut Pixmap, s: (f64, f64), e: (f64, f64), col: Rgb, width: f32| {
            let t = (width as f64).round().max(1.0);
            if let Some(pt) = pat.filter(|_| width >= lw) {
                stroke(pm, &[s, e], col, width, Some(pt));
            } else if s.1 == e.1 {
                fill(pm, s.0.min(e.0), s.1.round(), (e.0 - s.0).abs(), t, col, 1.0);
            } else if s.0 == e.0 {
                fill(pm, s.0.round(), s.1.min(e.1), t, (e.1 - s.1).abs(), col, 1.0);
            } else {
                lines(pm, 0.0, 0.0, &[&[(s.0 as f32, s.1 as f32), (e.0 as f32, e.1 as f32)]], col, width);
            }
        };
        let tag = |pm: &mut Pixmap, s: &str, x: f64, y: f64, bg: Rgb| {
            let tw = text.width(s, 11.5) as f64 + 12.0;
            round_fill(pm, Rect::new(x - tw / 2.0, y - 10.0, tw, 20.0), 4.0, bg, 0.92);
            text.draw(pm, s, (x as f32, (y + 4.0) as f32), 11.5, Rgb(255, 255, 255), Align::Center);
        };
        let tool = self.tool.as_str();
        match (tool, p.get(1).copied()) {
            // icône : l'émoji en couleur (Twemoji), centré sur le point
            ("icon", _) => {
                let s = self.icon_px();
                if let Some(img) = crate::emoji::image(self.emoji(), crate::px::dev(s as f64).round() as u32) {
                    let half = s as f64 / 2.0;
                    crate::px::put(pm, a.0 - half, a.1 - half, &img);
                }
            }
            ("hline", _) => seg(pm, (0.0, a.1), (w, a.1), c, lw),
            ("hray", _) => seg(pm, a, (w, a.1), c, lw),
            ("vline", _) => seg(pm, (a.0, 0.0), (a.0, h), c, lw),
            ("cross", _) => {
                seg(pm, (0.0, a.1), (w, a.1), c, lw);
                seg(pm, (a.0, 0.0), (a.0, h), c, lw);
            }
            ("ray" | "ext", Some(b)) => {
                let (s, e) = extend(a, b, tool == "ext", w, h);
                seg(pm, s, e, c, lw);
            }
            ("rect", Some(b)) => {
                let r = Rect::new(a.0.min(b.0), a.1.min(b.1), (b.0 - a.0).abs(), (b.1 - a.1).abs());
                fill(pm, r.x, r.y, r.w, r.h, c, 0.14);
                let edge = [(r.x, r.y), (r.x + r.w, r.y), (r.x + r.w, r.y + r.h), (r.x, r.y + r.h), (r.x, r.y)].map(|q| (q.0 as f32, q.1 as f32));
                let edge: Vec<(f64, f64)> = edge.iter().map(|q| (q.0 as f64, q.1 as f64)).collect();
                stroke(pm, &edge, c, lw, pat);
            }
            ("fib", Some(b)) => {
                let (p0, p1) = (self.pts[0].p, self.pts[1].p);
                let (x1, x2, niveaux) = fib_niveaux(&self.pts, a, b, g);
                seg(pm, a, b, Rgb(0x78, 0x7b, 0x86), 1.0);
                for &(_, y, col) in &niveaux {
                    seg(pm, (x1, y), (x2, y), col, 1.0);
                }
                // (B-019: a label never over another — « 0.618 » and « 0.652 » were drawn one on the other; the
                // levels that matter most keep theirs, the others' lines stay)
                let etiquettes = fib_etiquettes(&niveaux);
                for &(lv, y, col) in &niveaux {
                    if etiquettes.contains(&lv.to_bits()) {
                        let price = p1 + (p0 - p1) * lv;
                        text.draw(
                            pm,
                            &format!("{lv} ({})", label(price)),
                            ((x1 - 4.0) as f32, (y + 4.0) as f32),
                            11.0,
                            col,
                            Align::Right,
                        );
                    }
                }
            }
            ("long" | "short", Some(_)) => crate::position::draw(self, pm, text, g, label),
            (t, Some(b)) if BOXES.contains(&t) => {
                // mesure (prix, date, prix + date) : boîte verte en hausse, rouge en baisse, avec l'écart
                let (p0, p1) = (self.pts[0].p, self.pts[1].p);
                let col = if p1 >= p0 { crate::palette::up() } else { crate::palette::down() };
                fill(pm, a.0.min(b.0), a.1.min(b.1), (b.0 - a.0).abs(), (b.1 - a.1).abs(), col, 0.16);
                seg(pm, ((a.0 + b.0) / 2.0, a.1), ((a.0 + b.0) / 2.0, b.1), col, 1.4);
                seg(pm, (a.0, (a.1 + b.1) / 2.0), (b.0, (a.1 + b.1) / 2.0), col, 1.4);
                let (t0, t1) = (self.pts[0].t, self.pts[1].t);
                let bars = g.times.partition_point(|&t| t <= t1) as i64 - g.times.partition_point(|&t| t <= t0) as i64;
                let secs = (t1 - t0).abs();
                let dur = match secs {
                    86_400.. => format!("{:.1} j", secs as f64 / 86_400.0),
                    3600.. => format!("{:.1} h", secs as f64 / 3600.0),
                    _ => format!("{} min", secs / 60),
                };
                let sign = if p1 >= p0 { "+" } else { "" };
                let s = format!("{sign}{} ({:+.2} %) · {bars} bougies, {dur}", label(p1 - p0), (p1 / p0 - 1.0) * 100.0);
                tag(pm, &s, (a.0 + b.0) / 2.0, b.1 + if p1 >= p0 { -16.0 } else { 16.0 }, col);
            }
            (t, _) if LABELS.contains(&t) => {
                // la forme (épingle, drapeau, bulle…) puis le texte
                if let Some(o) = crate::toolgeom::geom(t, &self.pts, g, label) {
                    Self::draw_geom(pm, text, &o, c, lw, pat);
                }
                let s = if self.style.text.is_empty() {
                    crate::tools::def(t).map_or("Texte", |d| if t == "text" { "Texte" } else { d.label })
                } else {
                    &self.style.text
                };
                // étiquette de prix : le prix suffit (déjà dans sa forme)
                if t != "pricelabel" || !self.style.text.is_empty() {
                    let at = Self::text_at(t, &p);
                    let tc = crate::colorpick::split(&self.style.tcolor).map_or(if t == "signpost" { Rgb(255, 255, 255) } else { c }, |p| p.0);
                    // (Texte tab: alignment from the usual place, bold, italic)
                    let size = self.style.tsize.clamp(8.0, 48.0);
                    let (v, h) = self.text_align();
                    let dy = match v {
                        1 => -(size as f64) * 0.9,
                        3 => size as f64 * 0.9,
                        _ => 0.0,
                    };
                    let align = [Align::Left, Align::Center, Align::Right][(h.clamp(1, 3) - 1) as usize];
                    self.text_font()
                        .as_ref()
                        .unwrap_or(text)
                        .draw(pm, s, (at.0 as f32, (at.1 + dy) as f32), size, tc, align);
                }
            }
            (t, _) if MARKS.contains(&t) => {
                // tourne la flèche « haut » : bas = 180°, gauche = pointe à gauche, droite = pointe à droite
                let rot = |f: fn((f32, f32)) -> (f32, f32)| ARROW_UP.map(f);
                let shape = match t {
                    "arrowdn" | "amdn" => rot(|(x, y)| (x, -y)),
                    "amleft" => rot(|(x, y)| (y, x)),
                    "amright" => rot(|(x, y)| (-y, x)),
                    _ => ARROW_UP,
                };
                poly_fill(pm, a.0, a.1, &shape, c, 1.0);
            }
            _ => match self.shape(g, label) {
                // phase C : la forme de l'outil (canaux, fourchettes, Gann, figures, vagues, volume, formes…)
                Some(o) => Self::draw_geom(pm, text, &o, c, lw, pat),
                // ligne de tendance (ou un seul point en cours de placement) : ligne brisée par ses points
                None => {
                    if p.len() > 1 {
                        stroke(pm, &p, c, lw, pat);
                    }
                }
            },
        }
        // 🖍 the text of a line (bar: T, settings: Texte), above its first segment, in the text colour
        if !self.style.text.is_empty() && !LABELS.contains(&self.tool.as_str()) && !matches!(self.tool.as_str(), "long" | "short") {
            let (x, y) = match p.get(1) {
                Some(b) => ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0),
                None => (a.0.clamp(8.0, w - 8.0), a.1),
            };
            let tc = crate::colorpick::split(&self.style.tcolor).map_or(c, |p| p.0);
            let size = self.style.tsize.clamp(8.0, 48.0);
            let (v, h) = self.text_align();
            let by = match v {
                2 => y + size as f64 * 0.35,
                3 => y + size as f64 + 4.0,
                _ => y - 6.0,
            };
            let align = [Align::Left, Align::Center, Align::Right][(h.clamp(1, 3) - 1) as usize];
            self.text_font()
                .as_ref()
                .unwrap_or(text)
                .draw(pm, &self.style.text, (x as f32, by as f32), size, tc, align);
        }
        if selected {
            for (hx, hy) in self.handles(g) {
                let r = Rect::new(hx - 4.5, hy - 4.5, 9.0, 9.0);
                round_fill(pm, r, 4.5, Rgb(0x13, 0x17, 0x22), 1.0);
                round_stroke(pm, r, 4.5, c, 1.5);
            }
        }
    }
}

/// Trace une ligne brisée, pleine ou selon un motif (tirets / points).
fn stroke(pm: &mut Pixmap, pts: &[(f64, f64)], c: Rgb, width: f32, pat: Option<(f64, f64)>) {
    // 🔍 a horizontal or vertical line sits on the pixel grid (sharp, not two grey pixels)
    let flat = |k: fn(&(f64, f64)) -> f64| pts.len() > 1 && pts.iter().all(|q| (k(q) - k(&pts[0])).abs() < 0.01);
    let snapped: Vec<(f64, f64)> = if flat(|q| q.1) {
        let y = crate::px::snap_line(pts[0].1, width);
        pts.iter().map(|q| (q.0, y)).collect()
    } else if flat(|q| q.0) {
        let x = crate::px::snap_line(pts[0].0, width);
        pts.iter().map(|q| (x, q.1)).collect()
    } else {
        pts.to_vec()
    };
    let pts = &snapped[..];
    let f = |v: &[(f64, f64)]| v.iter().map(|q| (q.0 as f32, q.1 as f32)).collect::<Vec<_>>();
    match pat {
        Some((on, off)) => {
            for d in crate::toolgeom::dashes(pts, on, off) {
                lines(pm, 0.0, 0.0, &[&f(&d)], c, width);
            }
        }
        None => lines(pm, 0.0, 0.0, &[&f(pts)], c, width),
    }
}

/// La souris touche-t-elle une forme : près d'un trait (à `tol` points) ou dans une zone remplie ? 👆 The finger (a
/// `tol` wider than the mouse's) also reaches its texts (B-019: « Pas de FVG ici » could not be selected, nor removed).
fn geom_hit(o: &Geom, x: f64, y: f64, tol: f64) -> bool {
    let plus = tol - HIT_PX;
    let sur_texte = |l: &crate::toolgeom::Label| {
        // (11.5-point text: about 6.6 points a letter, its box 19 high from 14 above its baseline)
        let w = l.text.chars().count() as f64 * 6.6 + 12.0;
        let x0 = match l.align {
            Align::Left => l.at.0,
            Align::Right => l.at.0 - w,
            Align::Center => l.at.0 - w / 2.0,
        };
        x >= x0 - plus && x <= x0 + w + plus && y >= l.at.1 - 14.0 - plus && y <= l.at.1 + 5.0 + plus
    };
    o.segments().any(|(a, b)| seg_dist(x, y, a, b) <= tol)
        || o.fills.iter().any(|(poly, _, _)| crate::toolgeom::inside(poly, x, y))
        || (plus > 0.0 && o.labels.iter().any(sur_texte))
}

/// Objet sous la souris : le plus récent d'abord (celui dessiné par-dessus).
pub fn hit_index(drawings: &[Drawing], x: f64, y: f64, g: &Geo) -> Option<usize> {
    hit_index_r(drawings, x, y, g, HIT_PX)
}

/// 👆 [`hit_index`] for a pointer `tol` points wide (the finger: [`HIT_DOIGT`]).
pub fn hit_index_r(drawings: &[Drawing], x: f64, y: f64, g: &Geo, tol: f64) -> Option<usize> {
    drawings.iter().rposition(|d| d.hit_r(x, y, g, tol))
}

/// Width (points) kept left of the retracement's lines for a level's label (« 0.618 (63 659) »).
const FIB_ETIQUETTE_W: f64 = 120.0;
/// Height (points) of a level's label: two labels closer than this would be drawn one on the other.
const FIB_ETIQUETTE_H: f64 = 13.0;

/// The retracement's lines: from `x1` to `x2` (60 points at least), and each level (its value, its y, its colour).
fn fib_niveaux(pts: &[Point], a: (f64, f64), b: (f64, f64), g: &Geo) -> (f64, f64, Vec<(f64, f64, Rgb)>) {
    let (p0, p1) = (pts[0].p, pts[1].p);
    let (x1, x2) = (a.0.min(b.0), a.0.max(b.0).max(a.0.min(b.0) + 60.0));
    let niveaux = FIB.iter().map(|&(lv, col)| (lv, g.y_of(p1 + (p0 - p1) * lv), col)).collect();
    (x1, x2, niveaux)
}

/// 🏷️ The levels whose label is drawn (as bits of their value): none closer than [`FIB_ETIQUETTE_H`] to another, the
/// levels that matter most first (0 and 1, then 0.618, 0.5, 0.382, 1.618, then the list's order).
fn fib_etiquettes(niveaux: &[(f64, f64, Rgb)]) -> Vec<u64> {
    const D_ABORD: [f64; 6] = [0.0, 1.0, 0.618, 0.5, 0.382, 1.618];
    let mut ordre: Vec<&(f64, f64, Rgb)> = niveaux.iter().collect();
    ordre.sort_by_key(|(lv, _, _)| D_ABORD.iter().position(|d| d == lv).unwrap_or(D_ABORD.len()));
    let mut gardes: Vec<(u64, f64)> = vec![];
    for &&(lv, y, _) in &ordre {
        if gardes.iter().all(|(_, y0)| (y - y0).abs() >= FIB_ETIQUETTE_H) {
            gardes.push((lv.to_bits(), y));
        }
    }
    gardes.into_iter().map(|(k, _)| k).collect()
}

#[cfg(test)]
mod tests {

    /// 🎨 Opacity: its own field; a colour saved as « #rrggbbaa » is still read (colour and opacity).
    #[test]
    fn opacite_lue_des_deux_facons() {
        let mut d = Drawing::hline(1.0);
        d.style.opacity = 0.5;
        assert!((d.alpha() - 0.5).abs() < 1e-6);
        let mut old = Drawing::hline(1.0);
        old.style.color = "#ff000080".into();
        assert!((old.alpha() - 128.0 / 255.0).abs() < 1e-3 && old.color() == Rgb(0xff, 0, 0));
    }
    use super::*;

    fn setup() -> (Viewport, PriceScale, Slots, Vec<i64>) {
        let vp = Viewport {
            width: 1078,
            height: 528,
            right_offset: 5.0,
            bar_spacing: 10.0,
            ..Viewport::default()
        };
        let scale = PriceScale {
            min: 100.0,
            max: 200.0,
            top: 0.0,
            height: vp.plot_h(),
            log: false,
            invert: false,
        };
        let times: Vec<i64> = (0..200).map(|i| 1_700_000_000 + i * 60).collect();
        (vp, scale, Slots::uniform(times.len()), times)
    }

    #[test]
    fn temps_et_pixels() {
        let (vp, scale, slots, times) = setup();
        let g = Geo {
            mouse: None,
            vp: &vp,
            scale: &scale,
            slots: &slots,
            times: &times,
            bars: &[],
            tf: 60,
        };
        let x = vp.x_of(150, &slots);
        assert!((g.x_of(times[150]) - x).abs() < 1e-9);
        assert_eq!(g.t_of(x + 2.0), times[150], "collé à la bougie la plus proche");
        assert_eq!(g.t_of(x + 7.0), times[151]);
        // futur : 3 bougies après la dernière
        let last = vp.x_of(199, &slots);
        assert_eq!(g.t_of(last + 30.0), times[199] + 180);
        assert!((g.x_of(times[199] + 180) - (last + 30.0)).abs() < 1e-9);
    }

    #[test]
    fn toucher_deplacer_et_poignees() {
        let (vp, scale, slots, times) = setup();
        let g = Geo {
            mouse: None,
            vp: &vp,
            scale: &scale,
            slots: &slots,
            times: &times,
            bars: &[],
            tf: 60,
        };
        let mut d = Drawing::new("trend", vec![Point { t: times[100], p: 150.0 }, Point { t: times[150], p: 170.0 }]);
        let (a, b) = (g.xy(&d.pts[0]), g.xy(&d.pts[1]));
        let mid = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
        assert!(d.hit(mid.0, mid.1 + 2.0, &g));
        assert!(!d.hit(mid.0, mid.1 + 30.0, &g));
        assert_eq!(d.handle_at(b.0 + 2.0, b.1, &g), Some(1));
        // descendre de 50 px = −10 au prix (100 de prix sur 500 px), et 3 bougies à droite
        let from = d.pts.clone();
        d.move_from(&from, 30.0, 50.0, &g);
        assert!((d.pts[0].p - 140.0).abs() < 1e-6);
        assert_eq!(d.pts[0].t, times[103]);
        // verrouillé : ne bouge plus
        d.set_locked(true);
        let from = d.pts.clone();
        d.move_from(&from, 0.0, 50.0, &g);
        assert!((d.pts[0].p - 140.0).abs() < 1e-6);
        // le plus récent dessus
        let ds = vec![d, Drawing::hline(150.0)];
        assert_eq!(hit_index(&ds, 300.0, g.y_of(150.0), &g), Some(1));
        assert_eq!(hit_index(&ds, 300.0, g.y_of(120.0), &g), None);
    }

    /// ✏️ B-019 (Jee, 25 Nov 2026, IMG_3901 and IMG_3902): a Fibonacci retracement's labels are never drawn one on the
    /// other (« 0.618 » and « 0.652 », « 1.618 » and « 1.657 » were); 0 always keeps its own, 1 too when there is
    /// room, a tall one keeps all of them. Its whole drawing is touched: every level line (past 1 too) and its label,
    /// its box — the finger a finger's width off, the mouse at its own reach.
    #[test]
    fn fib_labels_apart_and_the_whole_fib_touched() {
        let (vp, scale, slots, times) = setup();
        let g = Geo {
            mouse: None,
            vp: &vp,
            scale: &scale,
            slots: &slots,
            times: &times,
            bars: &[],
            tf: 60,
        };
        for (haut, bas) in [(150.0, 149.0), (150.0, 140.0), (190.0, 110.0), (180.0, 100.0), (120.0, 115.0)] {
            let pts = [Point { t: times[150], p: haut }, Point { t: times[180], p: bas }];
            let (a, b) = (g.xy(&pts[0]), g.xy(&pts[1]));
            let (_, _, niveaux) = fib_niveaux(&pts, a, b, &g);
            let gardes = fib_etiquettes(&niveaux);
            let ys: Vec<f64> = niveaux.iter().filter(|(lv, _, _)| gardes.contains(&lv.to_bits())).map(|n| n.1).collect();
            for (i, y) in ys.iter().enumerate() {
                for y2 in &ys[i + 1..] {
                    assert!(
                        (y - y2).abs() >= FIB_ETIQUETTE_H,
                        "{haut}→{bas}: two labels {y:.1} and {y2:.1} one on the other"
                    );
                }
            }
            let span = (a.1 - b.1).abs();
            assert!(gardes.contains(&0.0f64.to_bits()), "{haut}→{bas}: 0 kept");
            assert_eq!(
                gardes.contains(&1.0f64.to_bits()),
                span >= FIB_ETIQUETTE_H,
                "{haut}→{bas}: 1 kept when it has room"
            );
            if span >= 14.0 * FIB_ETIQUETTE_H / 0.03 {
                assert_eq!(gardes.len(), FIB.len(), "{haut}→{bas}: tall, every label");
            }
        }
        // the whole Fibonacci touched by the finger, its level past 1 and its labels; the mouse: its box only, as before
        let d = Drawing::new("fib", vec![Point { t: times[150], p: 160.0 }, Point { t: times[180], p: 140.0 }]);
        let (a, b) = (g.xy(&d.pts[0]), g.xy(&d.pts[1]));
        let (x1, _, niveaux) = fib_niveaux(&d.pts, a, b, &g);
        let y1618 = niveaux.iter().find(|n| n.0 == 1.618).unwrap().1;
        assert!(d.hit_r(x1 + 10.0, y1618 + 12.0, &g, HIT_DOIGT), "the 1.618 line by the finger, 12 points off");
        assert!(
            d.hit(x1 + 10.0, y1618 + 4.0, &g) && !d.hit(x1 + 10.0, y1618 + 12.0, &g),
            "by the mouse, at its reach"
        );
        assert!(d.hit_r(x1 - 60.0, y1618, &g, HIT_DOIGT) && d.hit(x1 - 60.0, y1618, &g), "the 1.618 label");
        assert!(d.hit_r((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0, &g, HIT_DOIGT), "its box");
        assert!(!d.hit_r(x1 + 10.0, y1618 - 60.0, &g, HIT_DOIGT), "nothing far from it");
        // a handle a finger's width off (11 points) is caught by the finger, the closest one
        assert_eq!(d.handle_at_r(b.0 + 9.0, b.1 + 6.0, &g, HANDLE_DOIGT), Some(1));
        assert_eq!(d.handle_at(b.0 + 9.0, b.1 + 6.0, &g), None, "the mouse: as before");
    }

    #[test]
    fn aimant() {
        let (vp, scale, slots, times) = setup();
        let bars: Vec<Bar> = times
            .iter()
            .map(|&t| Bar {
                time: t,
                open: 150.0,
                high: 160.0,
                low: 140.0,
                close: 155.0,
                volume: 1.0,
            })
            .collect();
        let f = Frame {
            vp,
            scale,
            slots,
            times,
            bars,
            tf: 60,
        };
        let x = vp.x_of(150, &f.slots);
        let y = f.geo().y_of(158.0);
        assert!((f.snap(x, y, Magnet::Off).p - 158.0).abs() < 1e-6);
        assert_eq!(f.snap(x, y, Magnet::Weak).p, 160.0, "le haut est à 10 px");
        let far = f.geo().y_of(190.0);
        assert!((f.snap(x, far, Magnet::Weak).p - 190.0).abs() < 1e-6, "trop loin pour l'aimant faible");
        assert_eq!(f.snap(x, far, Magnet::Strong).p, 160.0);
    }

    #[test]
    fn anciennes_sauvegardes() {
        let old: Vec<Drawing> = serde_json::from_str(r#"[{"HLine":{"price":80123.4}}, {"HLine":{"price":1.0,"locked":true}}]"#).unwrap();
        assert_eq!(old[0].tool, "hline");
        assert_eq!(old[0].pts[0].p, 80123.4);
        assert!(old[1].locked());
        let now = Drawing::new("rect", vec![Point { t: 1, p: 2.0 }, Point { t: 3, p: 4.0 }]);
        let back: Drawing = serde_json::from_str(&serde_json::to_string(&now).unwrap()).unwrap();
        assert_eq!(back, now);
    }

    #[test]
    fn tous_les_outils_se_dessinent() {
        let (vp, scale, slots, times) = setup();
        let g = Geo {
            mouse: None,
            vp: &vp,
            scale: &scale,
            slots: &slots,
            times: &times,
            bars: &[],
            tf: 60,
        };
        let t = Text::default();
        for tool in crate::tools::TOOLS.iter().filter(|t| t.points > 0) {
            let mut pm = crate::px::new(vp.width, vp.height).unwrap();
            let pts = [(150, 150.0), (170, 130.0), (190, 160.0)].map(|(i, p)| Point { t: times[i], p });
            let d = Drawing::new(tool.id, pts[..(tool.points as usize).clamp(1, 3)].to_vec());
            d.draw(&mut pm, &t, &g, &|p| format!("{p:.1}"), true);
            assert!(pm.pixels().iter().any(|p| p.alpha() > 0), "{} ne dessine rien", tool.id);
        }
    }
}
