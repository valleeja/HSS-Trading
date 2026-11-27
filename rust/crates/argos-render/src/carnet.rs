//! 📖 The Carnet panel (mock-up v1, approved by Jee on 22 Nov 2026: « k maquettes carnet et options »): one book,
//! three tabs. Carnet = the levels grouped by a price step, a bar per level and the cumulated depth as a staircase, a
//! small band of the heatmap's colour, Argos Murs' walls in bold, the middle price and the imbalance. Delta = what was
//! placed, pulled and filled on each price during 10 s, 1 min or 5 min. DOM = a ladder that stays put (⌖ Centrer
//! brings it back to the price) with the coins traded on each price. Its header picks the exchange and the step; the
//! line at the bottom says where the book comes from (« Binance en direct · même flux qu'Argos Murs »).
//!
//! The app fills a [`CarnetVue`] from its one book feed; this file only draws it and says what a click hit.

use crate::palette::{GREY, WHITE};
use crate::shapes::{poly_fill, round_fill, round_stroke, Rect};
use crate::text::{Align, Graisse, Rgb, Text};
use argos_core::carnet::Mouvement;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tiny_skia::Pixmap;

/// The tabs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Onglet {
    #[default]
    Carnet,
    Delta,
    Dom,
}

impl Onglet {
    pub const ALL: [Onglet; 3] = [Onglet::Carnet, Onglet::Delta, Onglet::Dom];
    pub fn nom(self) -> &'static str {
        match self {
            Onglet::Carnet => "Carnet",
            Onglet::Delta => "Delta",
            Onglet::Dom => "DOM",
        }
    }
}

/// The Delta tab's window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Fenetre {
    S10,
    #[default]
    M1,
    M5,
}

impl Fenetre {
    pub const ALL: [Fenetre; 3] = [Fenetre::S10, Fenetre::M1, Fenetre::M5];
    pub fn ms(self) -> i64 {
        match self {
            Fenetre::S10 => 10_000,
            Fenetre::M1 => 60_000,
            Fenetre::M5 => 300_000,
        }
    }
    pub fn nom(self) -> &'static str {
        match self {
            Fenetre::S10 => "10 s",
            Fenetre::M1 => "1 min",
            Fenetre::M5 => "5 min",
        }
    }
    /// The middle line's words (« Dernière minute : achat +84 ₿ · vente −37 ₿ »).
    pub fn titre(self) -> &'static str {
        match self {
            Fenetre::S10 => "10 dernières secondes",
            Fenetre::M1 => "Dernière minute",
            Fenetre::M5 => "5 dernières minutes",
        }
    }
}

/// The exchanges of the header's ▾ (perp books; Agrégé: all of them summed). The choice is the heatmap's (its ⚙
/// « Carnets »): one book read for both.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bourse {
    Binance,
    Okx,
    Bybit,
    Hyperliquid,
    Bitunix,
    Agrege,
}

impl Bourse {
    pub const ALL: [Bourse; 6] = [
        Bourse::Binance,
        Bourse::Okx,
        Bourse::Bybit,
        Bourse::Hyperliquid,
        Bourse::Bitunix,
        Bourse::Agrege,
    ];
    pub fn nom(self) -> &'static str {
        match self {
            Bourse::Binance => "Binance",
            Bourse::Okx => "OKX",
            Bourse::Bybit => "Bybit",
            Bourse::Hyperliquid => "Hyperliquid",
            Bourse::Bitunix => "Bitunix",
            Bourse::Agrege => "Agrégé",
        }
    }

    /// The heatmap's ticks for it (Binance, Bybit, OKX, Bitunix, Hyperliquid, as in `bookheat::PARAMS`).
    pub fn coches(self) -> [bool; 5] {
        match self {
            Bourse::Binance => [true, false, false, false, false],
            Bourse::Bybit => [false, true, false, false, false],
            Bourse::Okx => [false, false, true, false, false],
            Bourse::Bitunix => [false, false, false, true, false],
            Bourse::Hyperliquid => [false, false, false, false, true],
            Bourse::Agrege => [true; 5],
        }
    }

    /// The exchange these ticks are (`None`: another mix).
    pub fn de_coches(c: [bool; 5]) -> Option<Bourse> {
        Bourse::ALL.into_iter().find(|b| b.coches() == c)
    }
}

/// The band's colours (⚙ « Couleurs »): the heatmap's Argos palette, Feu, Glace.
pub const PALETTES: [&str; 3] = ["Argos", "Feu", "Glace"];

/// The coins traded shown by the DOM (⚙ « Échangé »), in minutes.
pub const ECHANGES_MIN: [i64; 3] = [15, 60, 240];

/// What the panel keeps (saved with the watchlist's settings).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CarnetPrefs {
    /// The price step chosen (0: the price's own, 5 $ on BTC).
    pub pas: f64,
    pub onglet: Onglet,
    pub fenetre: Fenetre,
    /// Sizes and totals in dollars instead of coins.
    pub usd: bool,
    /// Levels per side (0: as many as the panel holds).
    pub niveaux: usize,
    /// The band's palette (index in [`PALETTES`]).
    pub palette: usize,
    /// 🔊 A sound when a big wall appears in the book.
    pub son_murs: bool,
    /// The DOM's traded coins, over this many minutes (one of [`ECHANGES_MIN`]).
    pub echange_min: i64,
}

impl Default for CarnetPrefs {
    fn default() -> Self {
        Self {
            pas: 0.0,
            onglet: Onglet::Carnet,
            fenetre: Fenetre::M1,
            usd: false,
            niveaux: 0,
            palette: 0,
            son_murs: false,
            echange_min: 60,
        }
    }
}

/// What the app gives the panel to draw.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CarnetVue {
    /// The levels grouped by `pas`, best first (sizes in coins).
    pub bids: Vec<(f64, f64)>,
    pub asks: Vec<(f64, f64)>,
    pub pas: f64,
    /// The middle price, and whether it went up since the book before (▲ / ▼; `None`: unchanged or not known).
    pub mid: f64,
    pub monte: Option<bool>,
    /// Best ask − best bid.
    pub ecart: f64,
    /// Decimals of the prices (the chart's).
    pub dec: usize,
    /// The coin's sign or name (« ₿ », « ETH »).
    pub unite: String,
    /// Argos Murs' walls shown on the chart: (bid side, price); their rows are in bold.
    pub murs: Vec<(bool, f64)>,
    /// Delta: what moved on each (bid side, bucket of `pas`) during the window, and the net of each side.
    pub delta: BTreeMap<(bool, i64), Mouvement>,
    pub net: (f64, f64),
    /// DOM: coins traded on each bucket of `pas` (bid grid), the ladder's centre (0: the middle price).
    pub echange: BTreeMap<i64, f64>,
    pub centre: f64,
    /// The exchange pill (« Binance », « Agrégé »), and where the book comes from, said in the footer
    /// (« Binance USDT 1/1 »).
    pub bourse: String,
    pub source: String,
    pub en_direct: bool,
    pub meme_flux: bool,
    /// Instead of the book (none here, or not read yet).
    pub message: Option<String>,
}

/// What a click in the panel hit (`Rect`s in the panel's coordinates: menus open next to them).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CarnetClick {
    Bourse(Rect),
    Pas(Rect),
    Fermer,
    Onglet(Onglet),
    Fenetre(Fenetre),
    Centrer,
    Echange(Rect),
    Reglages(Rect),
    Rien,
}

/// The mock-up's colours (`.a .px`, `.b .px`, the bars, the gold of « rempli »).
const ROUGE_PX: Rgb = Rgb(0xff, 0x7b, 0x84);
const VERT_PX: Rgb = Rgb(0x4f, 0xdc, 0x9b);
const ROUGE: Rgb = Rgb(0xea, 0x39, 0x43);
const VERT: Rgb = Rgb(0x3c, 0xcf, 0x8e);
const OR: Rgb = Rgb(0xf4, 0xc1, 0x52);
const BLEU: Rgb = Rgb(0x4d, 0xa3, 0xff);
const BLEU_TXT: Rgb = Rgb(0x9c, 0xc7, 0xff);

/// The band's colour of a level (`t` 0 to 1: its size against the biggest shown).
pub fn bande(palette: usize, t: f64) -> Rgb {
    let stops: [(f64, f64, f64); 3] = match palette {
        1 => [(70.0, 20.0, 20.0), (220.0, 90.0, 30.0), (255.0, 220.0, 90.0)],
        2 => [(25.0, 40.0, 90.0), (40.0, 140.0, 220.0), (200.0, 240.0, 255.0)],
        _ => [(27.0, 42.0, 85.0), (42.0, 157.0, 143.0), (233.0, 196.0, 106.0)],
    };
    let t = t.clamp(0.0, 1.0) * 2.0;
    let i = (t.floor() as usize).min(1);
    let f = t - i as f64;
    let (a, b) = (stops[i], stops[i + 1]);
    let m = |x: f64, y: f64| (x + (y - x) * f).round() as u8;
    Rgb(m(a.0, b.0), m(a.1, b.1), m(a.2, b.2))
}

/// A size as shown: coins (2 decimals, 0 from 10 000) or dollars (« 1,2 M $ »).
fn taille(q: f64, px: f64, usd: bool) -> String {
    if usd {
        return crate::bookheat::compact(q * px);
    }
    crate::viewport::format_number(q, if q >= 10_000.0 { 0 } else { 2 })
}

/// A Delta cell: its signed figure, or a dot when nothing moved at that level (as the « rempli » column; never
/// « +0,0 » / « −0,0 », a negative zero).
fn signe_case(v: f64, plus: bool) -> String {
    if (v.abs() * 10.0).round() == 0.0 {
        "·".into()
    } else {
        signe(v, plus)
    }
}

/// The texts of a Delta row's three cells, as drawn: posé, retiré, rempli (a dot where nothing moved).
fn textes_delta(m: &Mouvement) -> [String; 3] {
    let rempli = if m.rempli > 0.0 {
        crate::viewport::format_number(m.rempli, 1)
    } else {
        "·".into()
    };
    [signe_case(m.pose, true), signe_case(m.retire, false), rempli]
}

/// A signed size (« +12,9 », « −9,7 »).
fn signe(v: f64, plus: bool) -> String {
    let s = crate::viewport::format_number(v.abs(), 1);
    format!("{}{s}", if plus { "+" } else { "−" })
}

/// Places of the panel at width `w`, height `h`.
struct Geo {
    x0: f64,
    iw: f64,
    /// Header: exchange ▾, step ▾, ✕ (`None` in the column: the column has its own title and ✕); a pill of width 0
    /// is not shown (no room left in a narrow module).
    bourse: Rect,
    pas: Rect,
    /// Their texts (the exchange's name cut with « … » when the module is narrow).
    bourse_txt: String,
    pas_txt: String,
    fermer: Option<Rect>,
    titre_y: f64,
    /// The tabs' bar (width 0: not shown, the column too short: the book itself, its Carnet tab).
    onglets: Rect,
    /// The tab shown.
    onglet: Onglet,
    /// Where the tab's content starts, and where the footer starts.
    y: f64,
    pied: f64,
    /// Is the footer (where the book comes from) shown? A short column keeps its room for the levels.
    pied_vu: bool,
    /// ⚙ Tous les réglages (width 0: not shown, the column too short).
    reglages: Rect,
    rh: f64,
}

/// Height of a row (cell: a little less, like the mock-up's 390).
fn rangee_h(cell: bool) -> f64 {
    if cell {
        19.0
    } else {
        20.0
    }
}

/// Room of the ⌖ drawn before « Centrer ».
const CIBLE_W: f64 = 16.0;

/// Height of the middle block (price, écart, imbalance).
const MILIEU_H: f64 = 52.0;
/// Height of the column titles.
const COLS_H: f64 = 18.0;

impl Geo {
    fn new(text: &Text, w: f64, h: f64, prefs: &CarnetPrefs, vue: &CarnetVue, cell: bool, entete: bool) -> Geo {
        let pad = if cell { 12.0 } else { 14.0 };
        let x0 = pad;
        let iw = (w - 2.0 * pad).max(40.0);
        // header pills, from the right
        let mut x = x0 + iw;
        let fermer = entete.then(|| {
            x -= 24.0;
            Rect::new(x, pad + 1.0, 24.0, 24.0)
        });
        if fermer.is_some() {
            x -= 6.0;
        }
        // (no exchange: a market without a public book, nothing to choose)
        let pw = |s: &str| if vue.bourse.is_empty() { 0.0 } else { text.width(s, 12.5) as f64 + 20.0 };
        // 📖 the pills stay in the panel, right of the volet's title « 📖 Carnet »: in a narrow module the exchange's
        // pill ran past the module's left edge (B-044); its name is cut with « … », then the pill goes, then the
        // step's
        let gauche = if entete {
            x0 + 23.0 + Text::graisse(Graisse::Bold).width("Carnet", 16.0) as f64 + 10.0
        } else {
            x0
        };
        let pas_txt = pas_texte(vue, prefs);
        let wp = pw(&pas_txt);
        let pas = if x - wp >= gauche {
            Rect::new(x - wp, pad + 1.0, wp, 24.0)
        } else {
            Rect::new(x, pad + 1.0, 0.0, 24.0)
        };
        if pas.w > 0.0 {
            x -= wp + 6.0;
        }
        let place = x - gauche;
        let mut bourse_txt = bourse_texte(vue);
        if pw(&bourse_txt) > place {
            let nom = text.fit(bourse_nom(vue), 12.5, place - 20.0 - text.width(" ▾", 12.5) as f64);
            bourse_txt = if nom.chars().count() >= 3 { format!("{nom} ▾") } else { String::new() };
        }
        let wb = if bourse_txt.is_empty() || pas.w == 0.0 { 0.0 } else { pw(&bourse_txt) };
        let bourse = Rect::new(x - wb, pad + 1.0, wb, 24.0);
        // (a short column, a module among others: the ⚙ bar goes first, then the footer, so that two levels a side
        // still fit under the tab's own buttons; shorter still, the tabs' bar goes and the book itself stays, one
        // level a side at least; nothing drawn over another)
        let rh = rangee_h(cell);
        let boutons = |o: Onglet| match o {
            Onglet::Delta => 58.0,
            Onglet::Dom => 30.0,
            Onglet::Carnet => 0.0,
        };
        let barre = Rect::new(x0, pad + 36.0, iw, 34.0);
        // (too narrow for the Delta's or the DOM's figures and buttons: the tabs' bar goes too, the book itself stays,
        // its figures smaller if need be; nothing written over another, B-044)
        let etroit = !onglets_tiennent(text, iw, prefs, vue);
        let (reglages_h, pied_h) = (32.0 + 8.0, 18.0);
        // 📖 a volet or the phone's sheet (`entete`) is never scrolled: its footer and ⚙ always in view at its bottom,
        // whatever its height (the phone lying down: a sheet of about 140 points), its only way to the settings; the
        // levels, then the tabs' bar, make room
        let fixe = if entete { reglages_h + pied_h } else { 0.0 };
        let court = h - pad - fixe < barre.y + barre.h + 8.0 + boutons(prefs.onglet) + COLS_H + MILIEU_H + 2.0 * rh + 4.0;
        let (onglet, onglets, y) = if court || etroit {
            (Onglet::Carnet, Rect::new(x0, barre.y, 0.0, 0.0), barre.y)
        } else {
            (prefs.onglet, barre, barre.y + barre.h + 8.0)
        };
        let libre = h - pad - (y + boutons(onglet) + COLS_H + MILIEU_H + 4.0 * rh + 4.0);
        let (avec_reglages, pied_vu) = if entete {
            (true, true)
        } else {
            (libre >= reglages_h + pied_h, libre >= pied_h)
        };
        let bas = if avec_reglages { h - pad - reglages_h } else { h - pad };
        Geo {
            x0,
            iw,
            bourse,
            pas,
            bourse_txt,
            pas_txt,
            fermer,
            titre_y: pad + 19.0,
            onglets,
            onglet,
            y,
            pied: if pied_vu { bas - pied_h } else { bas },
            pied_vu,
            reglages: if avec_reglages {
                Rect::new(x0, h - pad - 32.0, iw, 32.0)
            } else {
                Rect::new(x0, h, 0.0, 0.0)
            },
            rh,
        }
    }

    /// The tabs, each one third of the segmented bar.
    fn onglet(&self, k: usize) -> Rect {
        let w = (self.onglets.w - 6.0) / 3.0;
        Rect::new(self.onglets.x + 3.0 + k as f64 * w, self.onglets.y + 3.0, w, self.onglets.h - 6.0)
    }

    /// The Delta tab's window buttons (70 % of the width, centred).
    fn fenetre(&self, k: usize) -> Rect {
        let bw = self.iw * 0.7;
        let x = self.x0 + (self.iw - bw) / 2.0;
        let w = (bw - 6.0) / 3.0;
        Rect::new(x + 3.0 + k as f64 * w, self.y + 3.0, w, 26.0)
    }

    /// The DOM's two pills (⌖ Centrer, Échangé : 1 h ▾), centred.
    fn dom_pilules(&self, text: &Text, prefs: &CarnetPrefs) -> (Rect, Rect) {
        let (a, b) = pilules_dom(text, prefs);
        let x = self.x0 + (self.iw - a - b - 6.0) / 2.0;
        (Rect::new(x, self.y, a, 24.0), Rect::new(x + a + 6.0, self.y, b, 24.0))
    }
}

/// The widths of the DOM's two pills (⌖ Centrer, Échangé : 1 h ▾).
fn pilules_dom(text: &Text, prefs: &CarnetPrefs) -> (f64, f64) {
    (
        text.width("Centrer", 12.5) as f64 + 20.0 + CIBLE_W,
        text.width(&echange_texte(prefs), 12.5) as f64 + 20.0,
    )
}

/// « 5 $ ▾ ».
fn pas_texte(vue: &CarnetVue, prefs: &CarnetPrefs) -> String {
    let p = if vue.pas > 0.0 { vue.pas } else { prefs.pas };
    if p <= 0.0 {
        return "Pas ▾".into();
    }
    format!("{} ▾", pas_nom(p))
}

/// « 5 $ », « 0,0005 $ »: a step with as few decimals as it needs (the step menu's lines).
pub fn pas_nom(p: f64) -> String {
    let dec = (0..=8).find(|d| {
        let x = p * 10f64.powi(*d);
        (x - x.round()).abs() <= x.abs() * 1e-9
    });
    format!("{} $", crate::viewport::format_number(p, dec.unwrap_or(8) as usize))
}

/// « Binance ▾ » (the exchange read: the chart's, or the one chosen).
fn bourse_texte(vue: &CarnetVue) -> String {
    format!("{} ▾", bourse_nom(vue))
}

/// « Binance » (« Bourse » before one is read).
fn bourse_nom(vue: &CarnetVue) -> &str {
    if vue.bourse.is_empty() {
        "Bourse"
    } else {
        vue.bourse.as_str()
    }
}

/// « Échangé : 1 h ▾ ».
fn echange_texte(prefs: &CarnetPrefs) -> String {
    format!("Échangé : {} ▾", duree_nom(prefs.echange_min))
}

/// « 15 min », « 1 h », « 4 h ».
pub fn duree_nom(m: i64) -> String {
    if m >= 60 && m % 60 == 0 {
        format!("{} h", m / 60)
    } else {
        format!("{m} min")
    }
}

/// Levels per side that fit between `y0` and `y1` (rows of `rh`, the middle block between), at most the chosen number.
fn niveaux(prefs: &CarnetPrefs, y0: f64, y1: f64, rh: f64) -> usize {
    let n = ((y1 - y0 - MILIEU_H) / 2.0 / rh).floor().max(1.0) as usize;
    if prefs.niveaux > 0 {
        n.min(prefs.niveaux)
    } else {
        n.min(30)
    }
}

/// Levels a side of the DOM's ladder whose titles start at `y0`: its rows (one in the middle) and the imbalance line
/// under them above the footer, at most the chosen number.
fn dom_niveaux(g: &Geo, y0: f64, prefs: &CarnetPrefs) -> usize {
    let rh = g.rh * 0.96;
    let n = (((g.pied - 4.0 - y0 - COLS_H - DESEQ_H) / rh - 1.0) / 2.0).floor().max(1.0) as usize;
    n.min(if prefs.niveaux > 0 { prefs.niveaux } else { 30 })
}

/// Height of the DOM's imbalance line, under its ladder.
const DESEQ_H: f64 = 24.0;

/// A row's bucket on the grid of `pas`.
fn cle(px: f64, pas: f64, achat: bool) -> i64 {
    argos_core::carnet::case(px, pas, achat)
}

/// Is this level one of Argos Murs' walls (same side, same step)?
fn est_mur(vue: &CarnetVue, achat: bool, px: f64) -> bool {
    vue.pas > 0.0 && vue.murs.iter().any(|(a, p)| *a == achat && cle(*p, vue.pas, achat) == cle(px, vue.pas, achat))
}

/// What is under `(x, y)`.
#[allow(clippy::too_many_arguments)]
pub fn click(text: &Text, (w, h): (f64, f64), prefs: &CarnetPrefs, vue: &CarnetVue, cell: bool, entete: bool, x: f64, y: f64) -> CarnetClick {
    let g = Geo::new(text, w, h, prefs, vue, cell, entete);
    if g.fermer.is_some_and(|r| r.contains(x, y)) {
        return CarnetClick::Fermer;
    }
    if g.bourse.w > 0.0 && g.bourse.contains(x, y) {
        return CarnetClick::Bourse(g.bourse);
    }
    if g.pas.w > 0.0 && g.pas.contains(x, y) {
        return CarnetClick::Pas(g.pas);
    }
    for (k, o) in Onglet::ALL.iter().enumerate() {
        if g.onglets.w > 0.0 && g.onglet(k).contains(x, y) {
            return CarnetClick::Onglet(*o);
        }
    }
    if g.reglages.w > 0.0 && g.reglages.contains(x, y) {
        return CarnetClick::Reglages(g.reglages);
    }
    match g.onglet {
        Onglet::Delta => {
            for (k, f) in Fenetre::ALL.iter().enumerate() {
                if g.fenetre(k).contains(x, y) {
                    return CarnetClick::Fenetre(*f);
                }
            }
        }
        Onglet::Dom => {
            let (c, e) = g.dom_pilules(text, prefs);
            if c.contains(x, y) {
                return CarnetClick::Centrer;
            }
            if e.contains(x, y) {
                return CarnetClick::Echange(e);
            }
        }
        Onglet::Carnet => {}
    }
    CarnetClick::Rien
}

/// The levels the DOM shows from its centre: `n` asks above, `n` bids below (prices on the grid of `pas`).
pub fn echelle(vue: &CarnetVue, n: usize) -> Vec<f64> {
    let pas = vue.pas;
    if !(pas > 0.0) {
        return vec![];
    }
    let c = if vue.centre > 0.0 { vue.centre } else { vue.mid };
    let k0 = cle(c, pas, true);
    (0..2 * n as i64 + 1).map(|i| (k0 + n as i64 - i) as f64 * pas).collect()
}

/// Draws the panel in `w` × `h` (`entete`: its own title and ✕, in a volet; the column has its own).
#[allow(clippy::too_many_arguments)]
pub fn draw(pm: &mut Pixmap, text: &Text, (w, h): (f64, f64), prefs: &CarnetPrefs, vue: &CarnetVue, mouse: Option<(f64, f64)>, cell: bool, entete: bool) {
    let g = Geo::new(text, w, h, prefs, vue, cell, entete);
    let survol = |r: Rect| mouse.is_some_and(|(x, y)| r.contains(x, y));
    let gras = Text::graisse(Graisse::Bold);
    let moyen = Text::graisse(Graisse::Medium);
    // header: 📖 Carnet, the exchange, the step, ✕
    if entete {
        text.draw(pm, "📖", (g.x0 as f32, g.titre_y as f32), 16.0, WHITE, Align::Left);
        gras.draw(pm, "Carnet", ((g.x0 + 23.0) as f32, g.titre_y as f32), 16.0, WHITE, Align::Left);
    }
    for (r, s) in [(g.bourse, &g.bourse_txt), (g.pas, &g.pas_txt)].into_iter().filter(|(r, _)| r.w > 0.0) {
        round_fill(pm, r, r.h / 2.0, crate::jetons::BLANC, if survol(r) { 0.14 } else { 0.08 });
        text.draw(pm, s, (r.cx() as f32, (r.cy() + 4.5) as f32), 12.5, crate::jetons::TEXTE, Align::Center);
    }
    if let Some(r) = g.fermer {
        round_fill(pm, r, r.h / 2.0, crate::jetons::BLANC, if survol(r) { 0.14 } else { 0.06 });
        text.draw(pm, "✕", (r.cx() as f32, (r.cy() + 4.5) as f32), 12.0, GREY, Align::Center);
    }
    // the tabs
    let o = g.onglets;
    if o.w > 0.0 {
        round_fill(pm, o, o.h / 2.0, crate::jetons::BLANC, 0.06);
    }
    for (k, t) in Onglet::ALL.iter().enumerate().filter(|_| o.w > 0.0) {
        let r = g.onglet(k);
        let on = *t == g.onglet;
        if on {
            round_fill(pm, r, r.h / 2.0, BLEU, 0.25);
        } else if survol(r) {
            round_fill(pm, r, r.h / 2.0, crate::jetons::BLANC, 0.05);
        }
        let (f, c) = if on { (&moyen, crate::jetons::BLANC) } else { (text, Rgb(0xc9, 0xd3, 0xee)) };
        f.draw(pm, t.nom(), (r.cx() as f32, (r.cy() + 4.5) as f32), 13.0, c, Align::Center);
    }
    // the footer: where the book comes from, ⚙ Tous les réglages ›
    let st_y = g.pied + 14.0;
    if g.pied_vu && !vue.source.is_empty() {
        let mut s = vue.source.clone();
        if vue.en_direct {
            s.push_str(" en direct");
        }
        if vue.meme_flux {
            s.push_str(" · même flux qu'Argos Murs");
        }
        let s = text.fit(&s, 11.5, g.iw - 16.0);
        let tw = text.width(&s, 11.5) as f64;
        let x = g.x0 + (g.iw - tw - 12.0) / 2.0;
        let point = if vue.en_direct { VERT } else { GREY };
        round_fill(pm, Rect::new(x, st_y - 7.5, 7.0, 7.0), 3.5, point, 1.0);
        text.draw(pm, &s, ((x + 12.0) as f32, st_y as f32), 11.5, Rgb(0x93, 0xa0, 0xc2), Align::Left);
    }
    let r = g.reglages;
    if r.w > 0.0 {
        round_fill(pm, r, r.h / 2.0, crate::jetons::BLANC, if survol(r) { 0.10 } else { 0.06 });
        text.draw(
            pm,
            &text.fit("⚙ Tous les réglages", 13.0, r.w - 14.0 - 32.0),
            ((r.x + 14.0) as f32, (r.cy() + 4.5) as f32),
            13.0,
            Rgb(0xc9, 0xd3, 0xee),
            Align::Left,
        );
        text.draw(
            pm,
            "›",
            ((r.x + r.w - 14.0) as f32, (r.cy() + 5.0) as f32),
            15.0,
            Rgb(0xc9, 0xd3, 0xee),
            Align::Right,
        );
    }
    // the tab
    let mut y = g.y;
    if g.onglet == Onglet::Delta {
        // 10 s · 1 min · 5 min, and the legend
        let bw = g.iw * 0.7;
        let seg = Rect::new(g.x0 + (g.iw - bw) / 2.0, y, bw, 32.0);
        round_fill(pm, seg, seg.h / 2.0, crate::jetons::BLANC, 0.06);
        for (k, f) in Fenetre::ALL.iter().enumerate() {
            let r = g.fenetre(k);
            let on = *f == prefs.fenetre;
            if on {
                round_fill(pm, r, r.h / 2.0, BLEU, 0.25);
            }
            let (t, c) = if on { (&moyen, crate::jetons::BLANC) } else { (text, Rgb(0xc9, 0xd3, 0xee)) };
            t.draw(pm, f.nom(), (r.cx() as f32, (r.cy() + 4.5) as f32), 13.0, c, Align::Center);
        }
        y += 38.0;
        let leg = LEGENDE_DELTA;
        let lw: f64 = leg.iter().map(|(s, _)| text.width(s, 11.0) as f64 + 13.0).sum::<f64>() + 20.0;
        let mut x = g.x0 + (g.iw - lw) / 2.0;
        for (s, c) in leg {
            round_fill(pm, Rect::new(x, y + 3.0, 9.0, 9.0), 2.0, c, 1.0);
            text.draw(pm, s, ((x + 13.0) as f32, (y + 11.5) as f32), 11.0, Rgb(0xc9, 0xd3, 0xee), Align::Left);
            x += text.width(s, 11.0) as f64 + 23.0;
        }
        y += 20.0;
    } else if g.onglet == Onglet::Dom {
        let (c, e) = g.dom_pilules(text, prefs);
        for (r, s) in [(c, "Centrer".to_string()), (e, echange_texte(prefs))] {
            round_fill(pm, r, r.h / 2.0, crate::jetons::BLANC, if survol(r) { 0.14 } else { 0.08 });
            let dx = if r == c { CIBLE_W / 2.0 } else { 0.0 };
            text.draw(pm, &s, ((r.cx() + dx) as f32, (r.cy() + 4.5) as f32), 12.5, crate::jetons::TEXTE, Align::Center);
        }
        // ⌖ (not in the fonts): a small ring and its cross
        let (ix, iy) = (c.x + 10.0 + 5.0, c.cy());
        round_stroke(pm, Rect::new(ix - 4.5, iy - 4.5, 9.0, 9.0), 4.5, crate::jetons::TEXTE, 1.2);
        crate::shapes::lines(
            pm,
            ix,
            iy,
            &[
                &[(-7.0, 0.0), (-2.5, 0.0)],
                &[(2.5, 0.0), (7.0, 0.0)],
                &[(0.0, -7.0), (0.0, -2.5)],
                &[(0.0, 2.5), (0.0, 7.0)],
            ],
            crate::jetons::TEXTE,
            1.2,
        );
        y += 30.0;
    }
    if let Some(m) = vue.message.as_ref().filter(|_| vue.bids.is_empty() && vue.asks.is_empty()) {
        let m = text.fit(m, 12.5, g.iw);
        text.draw(pm, &m, ((g.x0 + g.iw / 2.0) as f32, (y + 40.0) as f32), 12.5, GREY, Align::Center);
        return;
    }
    // (not even one level a side under the tab's buttons, or a book too narrow for its figures and its imbalance:
    // said, not drawn over the rest)
    let n = niveaux(prefs, y + COLS_H, g.pied - 4.0, g.rh);
    let trop_etroit = g.onglet == Onglet::Carnet && (taille_carnet(g.iw, prefs, vue, n) < TAILLE_MIN_CARNET || g.iw < desequilibre_min(text, vue));
    if y + COLS_H + MILIEU_H + 2.0 * g.rh > g.pied - 4.0 || trop_etroit {
        let m = match (entete, cell) {
            (false, _) => "Agrandis le module pour voir le carnet",
            (true, true) => "Tourne le téléphone pour voir le carnet",
            (true, false) => "Agrandis la fenêtre pour voir le carnet",
        };
        let m = text.fit(m, 12.0, g.iw);
        text.draw(
            pm,
            &m,
            ((g.x0 + g.iw / 2.0) as f32, (y + 18.0).min(g.pied - 4.0) as f32),
            12.0,
            GREY,
            Align::Center,
        );
        return;
    }
    match g.onglet {
        Onglet::Carnet => draw_carnet(pm, text, &g, y, n, prefs, vue),
        Onglet::Delta => draw_delta(pm, text, &g, y, n, prefs, vue),
        Onglet::Dom => draw_dom(pm, text, &g, y, prefs, vue),
    }
}

/// The column titles (small capitals, spaced), each at its anchor (`Align::Left`: its left edge). Each title comes
/// with its shorter forms (« Quantité ₿ », « Qté ₿ », « Qté »): two titles closer than 4 points, the longer one takes
/// its next form, and goes when it has none; a title past the panel's edges too. In a narrow module « PRIX » and
/// « QUANTITÉ ₿ » were written over each other (« PRQUANTITÉ ₿ », B-044); wide, nothing changes.
fn titres(pm: &mut Pixmap, text: &Text, g: &Geo, y: f64, cols: &[(&[&str], f64, Align)]) {
    let large = |s: &str| {
        let s = s.to_uppercase();
        text.width(&s, 10.5) as f64 + 0.8 * s.chars().count() as f64
    };
    let boite = |k: usize, v: usize| {
        let (formes, x, a) = &cols[k];
        let w = large(formes[v]);
        match a {
            Align::Right => (x - w, *x),
            Align::Center => (x - w / 2.0, x + w / 2.0),
            Align::Left => (*x, x + w),
        }
    };
    let mut forme: Vec<Option<usize>> = vec![Some(0); cols.len()];
    loop {
        let vus: Vec<(usize, usize)> = forme.iter().enumerate().filter_map(|(k, v)| v.map(|v| (k, v))).collect();
        let hors = vus.iter().find(|(k, v)| {
            let (a, b) = boite(*k, *v);
            a < g.x0 - 0.5 || b > g.x0 + g.iw + 0.5
        });
        let serres = || {
            vus.windows(2).find(|p| boite(p[1].0, p[1].1).0 - boite(p[0].0, p[0].1).1 < 4.0).map(|p| {
                let w = |(k, v): (usize, usize)| large(cols[k].0[v]);
                if w(p[0]) >= w(p[1]) {
                    p[0]
                } else {
                    p[1]
                }
            })
        };
        let Some((k, v)) = hors.copied().or_else(serres) else { break };
        forme[k] = (v + 1 < cols[k].0.len()).then_some(v + 1);
    }
    for (k, v) in forme.iter().enumerate().filter_map(|(k, v)| v.map(|v| (k, v))) {
        let s = cols[k].0[v].to_uppercase();
        let x0 = boite(k, v).0;
        text.draw_espace(pm, &s, (x0 as f32, (y + 12.0) as f32), 10.5, Rgb(0x93, 0xa0, 0xc2), 0.8);
    }
}

/// The middle block: the price with ▲ / ▼ (and the spread), and its second line.
fn milieu(pm: &mut Pixmap, text: &Text, g: &Geo, y: f64, vue: &CarnetVue, ligne: &dyn Fn(&mut Pixmap, f64)) {
    let ligne_c = crate::jetons::BLANC;
    crate::fill(pm, g.x0, y + 1.0, g.iw, 1.0, ligne_c, 0.08);
    crate::fill(pm, g.x0, y + MILIEU_H - 2.0, g.iw, 1.0, ligne_c, 0.08);
    let gras = Text::graisse(Graisse::Bold);
    let c = match vue.monte {
        Some(true) => VERT_PX,
        Some(false) => ROUGE_PX,
        None => WHITE,
    };
    let fleche = match vue.monte {
        Some(true) => " ▲",
        Some(false) => " ▼",
        None => "",
    };
    let prix = format!("{}{fleche}", crate::viewport::format_number(vue.mid, vue.dec.max(1)));
    let ecart = format!("écart {}", crate::viewport::format_number(vue.ecart, vue.dec.max(1)));
    // (a narrow module: the spread goes, then the price gets smaller; it ran past the module's edges, B-044)
    let pw = gras.width(&prix, 21.0) as f64;
    let ew = text.width(&ecart, 11.0) as f64;
    let avec_ecart = pw + 6.0 + ew <= g.iw;
    let taille = if pw <= g.iw { 21.0 } else { (21.0 * g.iw / pw) as f32 * 0.98 };
    let pw = gras.width(&prix, taille) as f64;
    let x = g.x0 + (g.iw - pw - if avec_ecart { 6.0 + ew } else { 0.0 }) / 2.0;
    gras.draw(pm, &prix, (x as f32, (y + 25.0) as f32), taille, c, Align::Left);
    if avec_ecart {
        text.draw(pm, &ecart, ((x + pw + 6.0) as f32, (y + 25.0) as f32), 11.0, Rgb(0x93, 0xa0, 0xc2), Align::Left);
    }
    ligne(pm, y + 43.0);
}

/// Levels a side the imbalance is read on: always the same depth, whatever the rows shown (a short module, the phone
/// and the volet gave three figures for one book: 20 %, 49 %, 50 %).
const DESEQ_NIVEAUX: usize = 10;

/// The buyers' part of the book's first levels: the same depth both sides, `DESEQ_NIVEAUX` at most (a side thinner
/// than that sets the depth of both: 10 bids against 4 asks read « 70 % achat » for a book even level by level).
fn part_achat(vue: &CarnetVue) -> Option<f64> {
    let k = DESEQ_NIVEAUX.min(vue.bids.len()).min(vue.asks.len());
    argos_core::carnet::part_achat(&vue.bids[..k], &vue.asks[..k])
}

/// The imbalance line's figures, the one place they are written: (buyers' part, « N % achat », « M % vente »).
fn textes_desequilibre(vue: &CarnetVue) -> Option<(f64, String, String)> {
    let p = part_achat(vue)?;
    let pa = (p * 100.0).round() as i64;
    Some((p, format!("{pa} % achat"), format!("{} % vente", 100 - pa)))
}

/// How the imbalance line fits a width `iw`, 12 px kept on each side: (the word « Déséquilibre » shown, the figures
/// without « achat » / « vente », the bar's width, the line's whole width). A narrow module drops the word first, then
/// shortens the bar (24 px at least): at 280 wide the line ran past both edges (« éséquilibre … 47 % vent », B-026);
/// narrower still, « 48 % ▬ 52 % » (their colours say which side, B-044).
fn place_desequilibre(text: &Text, iw: f64, achat: &str, vente: &str) -> (bool, bool, f64, f64) {
    let tw = |s: &str| text.width(s, 11.5) as f64;
    let place = iw - 24.0;
    let chiffres = tw(achat) + 6.0 + 6.0 + tw(vente);
    if tw(MOT_DESEQUILIBRE) + 6.0 + chiffres + 90.0 <= place {
        return (true, false, 90.0, tw(MOT_DESEQUILIBRE) + 6.0 + chiffres + 90.0);
    }
    if chiffres + 24.0 <= place {
        let barre = (place - chiffres).min(90.0);
        return (false, false, barre, chiffres + barre);
    }
    let courts = tw(&court_pct(achat)) + 6.0 + 6.0 + tw(&court_pct(vente));
    let barre = (place - courts).clamp(24.0, 90.0);
    (false, true, barre, courts + barre)
}

/// Smallest size of the Delta's and the DOM's figures: narrower, the tabs' bar goes and the book itself stays (B-044).
const TAILLE_MIN_ONGLET: f32 = 10.0;
/// Smallest size of the book's figures: narrower, « Agrandis le module pour voir le carnet ».
const TAILLE_MIN_CARNET: f32 = 8.0;

/// The figures' size: 12.5, smaller when two of a row's figures would come closer than 4 points (a narrow module:
/// they were written over each other, B-044). Each `(room, width at 12.5)`: the widest figures a room holds.
fn taille_figures(places: &[(f64, f64)]) -> f32 {
    let k = places.iter().filter(|(_, w)| *w > 0.0).map(|(p, w)| p / w).fold(1.0, f64::min);
    if k >= 1.0 {
        12.5
    } else {
        (12.5 * k * 0.98) as f32
    }
}

/// The widest of these texts at 12.5 (in bold: a wall's row is).
fn plus_large(textes: impl Iterator<Item = String>) -> f64 {
    let gras = Text::graisse(Graisse::Bold);
    textes.map(|s| gras.width(&s, 12.5) as f64).fold(0.0, f64::max)
}

/// The Carnet tab's columns from the inner left edge: price (left), size and total (right edges).
fn cols_carnet(iw: f64) -> (f64, f64, f64) {
    let cw = (iw - 18.0) / 3.1;
    (12.0, 12.0 + 2.1 * cw, 12.0 + 3.1 * cw)
}

/// The size of the Carnet tab's figures, its `n` levels a side.
fn taille_carnet(iw: f64, prefs: &CarnetPrefs, vue: &CarnetVue, n: usize) -> f32 {
    let (ix, x_q, x_t) = cols_carnet(iw);
    let niveaux = || vue.asks.iter().take(n).chain(vue.bids.iter().take(n));
    let cumul = |l: &[(f64, f64)]| -> Vec<(f64, f64)> {
        l.iter()
            .take(n)
            .scan(0.0, |a, x| {
                *a += x.1;
                Some((x.0, *a))
            })
            .collect()
    };
    let pw = plus_large(niveaux().map(|l| prix_niveau(vue, l.0)));
    let qw = plus_large(niveaux().map(|l| taille(l.1, l.0, prefs.usd)));
    let tw = plus_large(cumul(&vue.asks).into_iter().chain(cumul(&vue.bids)).map(|(p, t)| taille(t, p, prefs.usd)));
    taille_figures(&[(x_q - ix - 4.0, pw + qw), (x_t - x_q - 4.0, tw)])
}

/// The Delta tab's columns from the inner left edge: price (left), posé, retiré, rempli (right edges).
fn cols_delta(iw: f64) -> (f64, f64, f64, f64) {
    let cw = (iw - 18.0) / 3.9;
    (12.0, 12.0 + 2.1 * cw, 12.0 + 3.1 * cw, 12.0 + 3.9 * cw)
}

/// The size of the Delta tab's figures, its `n` levels a side.
fn taille_delta(iw: f64, vue: &CarnetVue, n: usize) -> f32 {
    let (ix, x_p, x_r, x_f) = cols_delta(iw);
    let niveaux = || {
        vue.asks
            .iter()
            .take(n)
            .map(|l| (false, l.0))
            .chain(vue.bids.iter().take(n).map(|l| (true, l.0)))
    };
    let mv = |(achat, px): (bool, f64)| vue.delta.get(&(achat, cle(px, vue.pas, achat))).copied().unwrap_or_default();
    let col = |k: usize| plus_large(niveaux().map(|l| textes_delta(&mv(l))[k].clone()));
    let pw = plus_large(niveaux().map(|l| prix_niveau(vue, l.1)));
    // (a cell's figure ends 4 points left of its column's edge)
    taille_figures(&[(x_p - 4.0 - ix - 4.0, pw + col(0)), (x_r - x_p - 4.0, col(1)), (x_f - x_r - 4.0, col(2))])
}

/// A level's price, as the Carnet's and the Delta's rows write it.
fn prix_niveau(vue: &CarnetVue, px: f64) -> String {
    crate::viewport::format_number(px, vue.dec)
}

/// The DOM's price, as its ladder writes it (no decimals for a step of 1 or more).
fn prix_dom(vue: &CarnetVue, px: f64) -> String {
    crate::viewport::format_number(px, vue.dec.min(if vue.pas >= 1.0 { 0 } else { vue.dec }))
}

/// The size of the DOM's figures (all the book's levels: its ladder moves with the price).
fn taille_dom(iw: f64, prefs: &CarnetPrefs, vue: &CarnetVue) -> f32 {
    let cw = iw / 4.0;
    let somme = |l: &[(f64, f64)], achat: bool| -> BTreeMap<i64, (f64, f64)> {
        let mut m = BTreeMap::new();
        for (px, q) in l {
            let e = m.entry(cle(*px, vue.pas, achat)).or_insert((*px, 0.0));
            e.1 += q;
        }
        m
    };
    let cote = |l: &[(f64, f64)], achat: bool| plus_large(somme(l, achat).into_values().map(|(p, q)| taille(q, p, prefs.usd)));
    let pw = plus_large(vue.asks.iter().chain(&vue.bids).map(|l| prix_dom(vue, l.0)).chain([prix_dom(vue, vue.mid)]));
    let ew = plus_large(vue.echange.values().map(|e| crate::viewport::format_number(*e, 1)));
    taille_figures(&[
        (cw - 4.0, cote(&vue.bids, true)),
        (cw - 4.0, pw),
        (cw - 4.0, cote(&vue.asks, false)),
        (cw - 4.0, ew),
    ])
}

/// The Delta's middle line: « 1 min : achat +12,9 ₿ · vente −9,7 ₿ », its pieces and their colours.
fn delta_milieu(prefs: &CarnetPrefs, vue: &CarnetVue) -> [(String, Rgb); 4] {
    let (na, nv) = vue.net;
    let u = &vue.unite;
    [
        (format!("{} : achat ", prefs.fenetre.titre()), Rgb(0xc9, 0xd3, 0xee)),
        (format!("{} {u}", signe(na, na >= 0.0)), VERT_PX),
        (" · vente ".to_string(), Rgb(0xc9, 0xd3, 0xee)),
        (format!("{} {u}", signe(nv, nv >= 0.0)), ROUGE_PX),
    ]
}

/// The Delta's legend: its three pieces and their colours.
const LEGENDE_DELTA: [(&str, Rgb); 3] = [("posé", VERT), ("retiré", ROUGE), ("rempli", OR)];

/// Do the three tabs hold in the inner width `iw`: their names in the bar; the Delta's window buttons, legend, middle
/// line and figures; the DOM's two pills, figures and imbalance (B-044: in a narrow module they ran into each other
/// and past its edges)? Not: the tabs' bar goes, the book itself stays.
fn onglets_tiennent(text: &Text, iw: f64, prefs: &CarnetPrefs, vue: &CarnetVue) -> bool {
    let large = |noms: &mut dyn Iterator<Item = &str>| noms.map(|s| text.width(s, 13.0) as f64).fold(0.0, f64::max);
    let barre = 3.0 * (large(&mut Onglet::ALL.iter().map(|o| o.nom())) + 16.0) + 6.0;
    let fenetres = (3.0 * (large(&mut Fenetre::ALL.iter().map(|f| f.nom())) + 12.0) + 6.0) / 0.7;
    let legende: f64 = LEGENDE_DELTA.iter().map(|(s, _)| text.width(s, 11.0) as f64 + 23.0).sum::<f64>() - 10.0;
    let milieu: f64 = delta_milieu(prefs, vue).iter().map(|(s, _)| text.width(s, 11.5) as f64).sum();
    let (a, b) = pilules_dom(text, prefs);
    [barre, fenetres, legende, milieu, a + b + 6.0, desequilibre_min(text, vue)]
        .iter()
        .all(|w| *w <= iw)
        && taille_delta(iw, vue, usize::MAX) >= TAILLE_MIN_ONGLET
        && taille_dom(iw, prefs, vue) >= TAILLE_MIN_ONGLET
}

/// « 48 % achat » → « 48 % ».
fn court_pct(s: &str) -> String {
    s.split(" % ").next().map_or_else(|| s.to_string(), |n| format!("{n} %"))
}

/// The narrowest inner width the imbalance line holds in, its short form with its 12 px margins.
fn desequilibre_min(text: &Text, vue: &CarnetVue) -> f64 {
    textes_desequilibre(vue).map_or(0.0, |(_, a, v)| {
        text.width(&court_pct(&a), 11.5) as f64 + 12.0 + text.width(&court_pct(&v), 11.5) as f64 + 24.0 + 24.0
    })
}

const MOT_DESEQUILIBRE: &str = "Déséquilibre";

/// « Déséquilibre 48 % achat ▬▬ 52 % vente », centred on baseline `y` (its figures from `textes_desequilibre`, its
/// room from `place_desequilibre`).
fn desequilibre(pm: &mut Pixmap, text: &Text, g: &Geo, y: f64, vue: &CarnetVue) {
    let Some((p, s2, s3)) = textes_desequilibre(vue) else { return };
    let tw = |s: &str| text.width(s, 11.5) as f64;
    let (mot, court, barre, total) = place_desequilibre(text, g.iw, &s2, &s3);
    let (s2, s3) = if court { (court_pct(&s2), court_pct(&s3)) } else { (s2, s3) };
    let mut x = g.x0 + (g.iw - total) / 2.0;
    if mot {
        text.draw(pm, MOT_DESEQUILIBRE, (x as f32, y as f32), 11.5, Rgb(0xc9, 0xd3, 0xee), Align::Left);
        x += tw(MOT_DESEQUILIBRE) + 6.0;
    }
    text.draw(pm, &s2, (x as f32, y as f32), 11.5, VERT_PX, Align::Left);
    x += tw(&s2) + 6.0;
    let bar = Rect::new(x, y - 7.0, barre, 6.0);
    round_fill(pm, bar, 3.0, ROUGE, 1.0);
    round_fill(pm, Rect::new(bar.x, bar.y, (bar.w * p).max(3.0), bar.h), 3.0, VERT, 1.0);
    x += barre + 6.0;
    text.draw(pm, &s3, (x as f32, y as f32), 11.5, ROUGE_PX, Align::Left);
}

/// 📖 The Carnet tab.
fn draw_carnet(pm: &mut Pixmap, text: &Text, g: &Geo, y0: f64, n: usize, prefs: &CarnetPrefs, vue: &CarnetVue) {
    let gras = Text::graisse(Graisse::Bold);
    // columns: 1.1fr 1fr 1fr, 12 in from the left, 6 from the right; the figures smaller in a narrow module
    let (ix, x_q, x_t) = cols_carnet(g.iw);
    let (ix, x_q, x_t) = (g.x0 + ix, g.x0 + x_q, g.x0 + x_t);
    let tf = taille_carnet(g.iw, prefs, vue, n);
    let unite = if prefs.usd { "$".to_string() } else { vue.unite.clone() };
    let (q1, q2) = (format!("Quantité {unite}"), format!("Qté {unite}"));
    titres(
        pm,
        text,
        g,
        y0,
        &[
            (&["Prix"], ix, Align::Left),
            (&[&q1, &q2, "Qté"], x_q, Align::Right),
            (&["Total", "Tot."], x_t, Align::Right),
        ],
    );
    let rh = g.rh;
    let asks: Vec<(f64, f64)> = vue.asks.iter().take(n).copied().collect();
    let bids: Vec<(f64, f64)> = vue.bids.iter().take(n).copied().collect();
    let cum = |l: &[(f64, f64)]| -> Vec<f64> {
        l.iter()
            .scan(0.0, |a, x| {
                *a += x.1;
                Some(*a)
            })
            .collect()
    };
    let (ca, cb) = (cum(&asks), cum(&bids));
    let max_c = ca.last().copied().unwrap_or(0.0).max(cb.last().copied().unwrap_or(0.0)).max(1e-12);
    let max_q = asks.iter().chain(&bids).map(|l| l.1).fold(0.0, f64::max).max(1e-12);
    let ya = y0 + COLS_H;
    let ym = ya + n as f64 * rh;
    let yb = ym + MILIEU_H;
    // the staircase of the cumulated depth, behind the rows (asks: the furthest at the top)
    let escalier = |pm: &mut Pixmap, top: f64, rows: &[f64], c: Rgb, fond: f32| {
        if rows.is_empty() {
            return;
        }
        let right = (g.x0 + g.iw) as f32;
        let mut pts: Vec<(f32, f32)> = vec![(right, top as f32)];
        for (i, cv) in rows.iter().enumerate() {
            let x = right - (g.iw * cv / max_c) as f32;
            pts.push((x, (top + i as f64 * rh) as f32));
            pts.push((x, (top + (i + 1) as f64 * rh) as f32));
        }
        pts.push((right, (top + rows.len() as f64 * rh) as f32));
        poly_fill(pm, 0.0, 0.0, &pts, c, fond);
        let contour: Vec<(f32, f32)> = pts[1..pts.len() - 1].to_vec();
        crate::shapes::lines_alpha(pm, 0.0, 0.0, &[&contour], c, 1.0, 0.45);
    };
    let ca_haut: Vec<f64> = ca.iter().rev().copied().collect();
    escalier(pm, ya, &ca_haut, ROUGE, 0.10);
    escalier(pm, yb, &cb, VERT, 0.09);
    let ligne = |pm: &mut Pixmap, y: f64, (px, q): (f64, f64), total: f64, achat: bool| {
        let mur = est_mur(vue, achat, px);
        // the band (heatmap's colours) and the level's bar
        round_fill(pm, Rect::new(g.x0, y + 2.0, 4.0, rh - 4.0), 2.0, bande(prefs.palette, q / max_q), 1.0);
        let bw = g.iw * 0.62 * (q / max_q);
        let bar = Rect::new(g.x0 + g.iw - bw, y + 1.0, bw, rh - 2.0);
        round_fill(pm, bar, 3.0, if achat { VERT } else { ROUGE }, if achat { 0.27 } else { 0.30 });
        if mur {
            round_stroke(pm, bar, 3.0, crate::jetons::BLANC, 1.0);
        }
        let base = (y + rh / 2.0 + 4.5) as f32;
        let f = if mur { &gras } else { text };
        let cp = if mur {
            crate::jetons::BLANC
        } else if achat {
            VERT_PX
        } else {
            ROUGE_PX
        };
        f.draw(pm, &prix_niveau(vue, px), (ix as f32, base), tf, cp, Align::Left);
        let cq = if mur { crate::jetons::BLANC } else { crate::jetons::TEXTE };
        f.draw(pm, &taille(q, px, prefs.usd), (x_q as f32, base), tf, cq, Align::Right);
        f.draw(pm, &taille(total, px, prefs.usd), (x_t as f32, base), tf, cq, Align::Right);
    };
    // asks: the best just above the middle
    for (k, (l, t)) in asks.iter().zip(&ca).enumerate() {
        ligne(pm, ya + (n - 1 - k) as f64 * rh, *l, *t, false);
    }
    milieu(pm, text, g, ym, vue, &|pm, y| desequilibre(pm, text, g, y, vue));
    for (k, (l, t)) in bids.iter().zip(&cb).enumerate() {
        ligne(pm, yb + k as f64 * rh, *l, *t, true);
    }
}

/// 📖 The Delta tab.
fn draw_delta(pm: &mut Pixmap, text: &Text, g: &Geo, y0: f64, n: usize, prefs: &CarnetPrefs, vue: &CarnetVue) {
    let gras = Text::graisse(Graisse::Bold);
    // columns: 1.1fr 1fr 1fr .8fr; the figures smaller in a narrow module
    let cw = (g.iw - 18.0) / 3.9;
    let (ix, x_p, x_r, x_f) = cols_delta(g.iw);
    let (ix, x_p, x_r, x_f) = (g.x0 + ix, g.x0 + x_p, g.x0 + x_r, g.x0 + x_f);
    let tf = taille_delta(g.iw, vue, n);
    titres(
        pm,
        text,
        g,
        y0,
        &[
            (&["Prix"], ix, Align::Left),
            (&["Posé"], x_p, Align::Right),
            (&["Retiré", "Ret."], x_r, Align::Right),
            (&["Rempli", "Remp."], x_f, Align::Right),
        ],
    );
    let rh = g.rh;
    let ya = y0 + COLS_H;
    let ym = ya + n as f64 * rh;
    let yb = ym + MILIEU_H;
    let mv = |achat: bool, px: f64| vue.delta.get(&(achat, cle(px, vue.pas, achat))).copied().unwrap_or_default();
    let rows: Vec<(bool, f64)> = vue
        .asks
        .iter()
        .take(n)
        .map(|l| (false, l.0))
        .chain(vue.bids.iter().take(n).map(|l| (true, l.0)))
        .collect();
    let max = rows
        .iter()
        .map(|(a, p)| {
            let m = mv(*a, *p);
            m.pose.max(m.retire).max(m.rempli)
        })
        .fold(0.0, f64::max)
        .max(1e-12);
    let cellule = |pm: &mut Pixmap, x_droite: f64, y: f64, v: f64, s: &str, c: Rgb, fond: Rgb, f: &Text| {
        let w = (cw - 4.0) * (v / max).min(1.0);
        if w > 0.5 {
            round_fill(pm, Rect::new(x_droite - w, y + 2.0, w, rh - 4.0), 3.0, fond, 0.30);
        }
        f.draw(pm, s, ((x_droite - 4.0) as f32, (y + rh / 2.0 + 4.5) as f32), tf, c, Align::Right);
    };
    let ligne = |pm: &mut Pixmap, y: f64, achat: bool, px: f64| {
        let m = mv(achat, px);
        let mur = est_mur(vue, achat, px);
        let f = if mur { &gras } else { text };
        let cp = if mur {
            crate::jetons::BLANC
        } else if achat {
            VERT_PX
        } else {
            ROUGE_PX
        };
        f.draw(pm, &prix_niveau(vue, px), (ix as f32, (y + rh / 2.0 + 4.5) as f32), tf, cp, Align::Left);
        let [tp, tr, trp] = textes_delta(&m);
        cellule(pm, x_p, y, m.pose, &tp, VERT_PX, VERT, f);
        cellule(pm, x_r, y, m.retire, &tr, ROUGE_PX, ROUGE, f);
        cellule(pm, x_f, y, m.rempli, &trp, OR, OR, f);
    };
    for (k, l) in vue.asks.iter().take(n).enumerate() {
        ligne(pm, ya + (n - 1 - k) as f64 * rh, false, l.0);
    }
    milieu(pm, text, g, ym, vue, &|pm, y| {
        let parts = delta_milieu(prefs, vue);
        let tw: f64 = parts.iter().map(|(s, _)| text.width(s, 11.5) as f64).sum();
        let mut x = g.x0 + (g.iw - tw) / 2.0;
        for (s, c) in parts {
            text.draw(pm, &s, (x as f32, y as f32), 11.5, c, Align::Left);
            x += text.width(&s, 11.5) as f64;
        }
    });
    for (k, l) in vue.bids.iter().take(n).enumerate() {
        ligne(pm, yb + k as f64 * rh, true, l.0);
    }
}

/// 📖 The DOM tab: a ladder of `2n + 1` prices around its centre, the bids left, the asks right of the price, the coins
/// traded on the right; the middle price's row lit.
fn draw_dom(pm: &mut Pixmap, text: &Text, g: &Geo, y0: f64, prefs: &CarnetPrefs, vue: &CarnetVue) {
    let gras = Text::graisse(Graisse::Bold);
    let cw = g.iw / 4.0;
    let cx = |k: f64| g.x0 + cw * (k + 0.5);
    titres(
        pm,
        text,
        g,
        y0,
        &[
            (&["Achat"], cx(0.0), Align::Center),
            (&["Prix"], cx(1.0), Align::Center),
            (&["Vente"], cx(2.0), Align::Center),
            (&["Échangé", "Éch."], cx(3.0), Align::Center),
        ],
    );
    // the ladder holds the middle row and the imbalance below it: as many levels a side as fit above the footer (the
    // book tabs' count left an empty band under the imbalance)
    let rh = g.rh * 0.96;
    let n = dom_niveaux(g, y0, prefs);
    let tf = taille_dom(g.iw, prefs, vue);
    let prix = echelle(vue, n);
    let pas = vue.pas;
    let taille_a = |achat: bool, px: f64| -> f64 {
        let l = if achat { &vue.bids } else { &vue.asks };
        l.iter().filter(|x| cle(x.0, pas, achat) == cle(px, pas, achat)).map(|x| x.1).sum()
    };
    let max_q = vue.asks.iter().chain(&vue.bids).map(|l| l.1).fold(0.0, f64::max).max(1e-12);
    let max_e = prix
        .iter()
        .map(|p| vue.echange.get(&cle(*p, pas, true)).copied().unwrap_or(0.0))
        .fold(0.0, f64::max)
        .max(1e-12);
    let k_mid = cle(vue.mid, pas, true);
    let best_ask = vue.asks.first().map_or(f64::INFINITY, |l| l.0);
    let best_bid = vue.bids.first().map_or(0.0, |l| l.0);
    let mut y = y0 + COLS_H;
    for px in &prix {
        let cur = cle(*px, pas, true) == k_mid;
        if cur {
            round_fill(pm, Rect::new(g.x0, y, g.iw, rh), 6.0, crate::jetons::BLANC, 0.10);
        }
        let base = (y + rh / 2.0 + 4.5) as f32;
        let (qa, qb) = (
            if *px >= best_ask - 1e-9 { taille_a(false, *px) } else { 0.0 },
            if *px <= best_bid + 1e-9 { taille_a(true, *px) } else { 0.0 },
        );
        let mur = (qb > 0.0 && est_mur(vue, true, *px)) || (qa > 0.0 && est_mur(vue, false, *px));
        let f = if cur || mur { &gras } else { text };
        if qb > 0.0 {
            let w = g.iw * 0.25 * (qb / max_q);
            let r = Rect::new(g.x0, y + 1.0, w, rh - 2.0);
            round_fill(pm, r, 3.0, VERT, 0.30);
            if mur {
                round_stroke(pm, r, 3.0, crate::jetons::BLANC, 1.0);
            }
            f.draw(pm, &taille(qb, *px, prefs.usd), (cx(0.0) as f32, base), tf, VERT_PX, Align::Center);
        }
        if qa > 0.0 {
            let w = g.iw * 0.25 * (qa / max_q);
            let r = Rect::new(g.x0 + g.iw * 0.5, y + 1.0, w, rh - 2.0);
            round_fill(pm, r, 3.0, ROUGE, 0.32);
            if mur {
                round_stroke(pm, r, 3.0, crate::jetons::BLANC, 1.0);
            }
            f.draw(pm, &taille(qa, *px, prefs.usd), (cx(2.0) as f32, base), tf, ROUGE_PX, Align::Center);
        }
        let cp = if cur {
            crate::jetons::BLANC
        } else if *px >= vue.mid {
            ROUGE_PX
        } else {
            VERT_PX
        };
        f.draw(pm, &prix_dom(vue, *px), (cx(1.0) as f32, base), tf, cp, Align::Center);
        let e = vue.echange.get(&cle(*px, pas, true)).copied().unwrap_or(0.0);
        let w = g.iw * 0.25 * (e / max_e);
        if w > 0.5 {
            round_fill(pm, Rect::new(g.x0 + g.iw - w, y + 1.0, w, rh - 2.0), 3.0, BLEU, 0.30);
        }
        text.draw(pm, &crate::viewport::format_number(e, 1), (cx(3.0) as f32, base), tf, BLEU_TXT, Align::Center);
        y += rh;
    }
    desequilibre(pm, text, g, y + 16.0, vue);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The mock-up's book (Binance BTC, step 5 $): 12 levels a side, two walls a side.
    pub(crate) fn vue_maquette() -> CarnetVue {
        let asks: Vec<(f64, f64)> = (0..12)
            .map(|i| {
                let q = [3.0, 15.15, 33.63, 16.83, 34.71, 31.75, 74.43, 37.30, 18.72, 32.32, 5.26, 58.03][i];
                (60_100.0 + 5.0 * i as f64, q)
            })
            .collect();
        let bids: Vec<(f64, f64)> = (0..12)
            .map(|i| {
                let q = [40.83, 12.42, 25.31, 34.08, 25.71, 65.75, 29.44, 13.68, 32.32, 59.77, 31.36, 22.72][i];
                (60_095.0 - 5.0 * i as f64, q)
            })
            .collect();
        CarnetVue {
            bids,
            asks,
            pas: 5.0,
            mid: 60_096.0,
            monte: Some(false),
            ecart: 1.0,
            dec: 1,
            unite: "₿".into(),
            murs: vec![(false, 60_130.0), (false, 60_155.0), (true, 60_070.0), (true, 60_050.0)],
            bourse: "Binance".into(),
            source: "Binance".into(),
            en_direct: true,
            meme_flux: true,
            ..Default::default()
        }
    }

    #[test]
    fn clics_du_panneau() {
        let t = Text::default();
        let (w, h) = (350.0, 720.0);
        let p = CarnetPrefs::default();
        let v = vue_maquette();
        let g = Geo::new(&t, w, h, &p, &v, false, true);
        let c = |x: f64, y: f64, p: &CarnetPrefs| click(&t, (w, h), p, &v, false, true, x, y);
        assert_eq!(c(g.onglet(1).cx(), g.onglet(1).cy(), &p), CarnetClick::Onglet(Onglet::Delta));
        assert!(matches!(c(g.bourse.cx(), g.bourse.cy(), &p), CarnetClick::Bourse(_)));
        assert!(matches!(c(g.pas.cx(), g.pas.cy(), &p), CarnetClick::Pas(_)));
        assert_eq!(c(g.fermer.unwrap().cx(), g.fermer.unwrap().cy(), &p), CarnetClick::Fermer);
        assert!(matches!(c(g.reglages.cx(), g.reglages.cy(), &p), CarnetClick::Reglages(_)));
        let d = CarnetPrefs {
            onglet: Onglet::Delta,
            ..Default::default()
        };
        assert_eq!(c(g.fenetre(0).cx(), g.fenetre(0).cy(), &d), CarnetClick::Fenetre(Fenetre::S10));
        let dom = CarnetPrefs {
            onglet: Onglet::Dom,
            ..Default::default()
        };
        let (ce, ec) = g.dom_pilules(&t, &dom);
        assert_eq!(c(ce.cx(), ce.cy(), &dom), CarnetClick::Centrer);
        assert!(matches!(c(ec.cx(), ec.cy(), &dom), CarnetClick::Echange(_)));
        // the column: no ✕ of its own
        let gc = Geo::new(&t, w, h, &p, &v, false, false);
        assert!(gc.fermer.is_none());
        assert!(gc.pas.x + gc.pas.w > g.pas.x + g.pas.w, "its pills go to the right edge");
        assert!(matches!(click(&t, (w, h), &p, &v, false, false, w - 20.0, gc.pas.cy()), CarnetClick::Pas(_)));
    }

    /// 📖 A short column (a module among others): the ⚙ bar goes, then the footer, then the tabs' bar (the book
    /// itself stays); with no room for one level a side, a word instead of rows drawn over one another.
    #[test]
    fn une_colonne_courte_ne_superpose_rien() {
        let t = Text::default();
        let (p, v) = (CarnetPrefs::default(), vue_maquette());
        let g = Geo::new(&t, 320.0, 720.0, &p, &v, false, false);
        assert!(g.reglages.w > 0.0 && g.pied_vu);
        let g = Geo::new(&t, 320.0, 300.0, &p, &v, false, false);
        assert!(g.reglages.w == 0.0 && g.pied_vu, "the ⚙ bar goes first");
        assert_eq!(click(&t, (320.0, 300.0), &p, &v, false, false, 160.0, 290.0), CarnetClick::Rien);
        let g = Geo::new(&t, 320.0, 270.0, &p, &v, false, false);
        assert!(g.reglages.w == 0.0 && !g.pied_vu, "then the footer");
        // shorter still: no tabs' bar, the book itself (Delta chosen: its buttons need more room)
        let d = CarnetPrefs {
            onglet: Onglet::Delta,
            ..Default::default()
        };
        let g = Geo::new(&t, 320.0, 250.0, &d, &v, false, false);
        assert!(g.onglets.w == 0.0 && g.onglet == Onglet::Carnet);
        assert_eq!(click(&t, (320.0, 250.0), &d, &v, false, false, 60.0, 67.0), CarnetClick::Rien);
        assert_eq!(Geo::new(&t, 320.0, 400.0, &d, &v, false, false).onglet, Onglet::Delta);
        for h in [140.0, 220.0, 270.0, 300.0, 400.0] {
            let g = Geo::new(&t, 320.0, h, &p, &v, false, false);
            let n = niveaux(&p, g.y + COLS_H, g.pied - 4.0, g.rh);
            let bas = g.y + COLS_H + MILIEU_H + 2.0 * n as f64 * g.rh;
            assert!(
                bas <= g.pied - 4.0 || g.y + COLS_H + MILIEU_H + 2.0 * g.rh > g.pied - 4.0,
                "{h}: {bas} / {}",
                g.pied
            );
            let mut pm = Pixmap::new(320, h as u32).unwrap();
            draw(&mut pm, &t, (320.0, h), &p, &v, None, false, false);
        }
    }

    /// 📖 One book, one imbalance: read on the same 10 levels a side in a short module, on the phone, in the volet
    /// and on the DOM (it was read on the rows shown: three figures for the same book). What is DRAWN is checked (the
    /// control's review, 25 Nov: a `take(n)` put back in the drawing kept the figure's own test green).
    #[test]
    fn un_seul_desequilibre_pour_un_carnet() {
        let v = vue_maquette();
        let p = part_achat(&v).unwrap();
        let (b, a): (f64, f64) = (v.bids.iter().take(10).map(|l| l.1).sum(), v.asks.iter().take(10).map(|l| l.1).sum());
        assert!((p - b / (b + a)).abs() < 1e-12);
        let (_, achat, vente) = textes_desequilibre(&v).unwrap();
        assert_eq!((achat.as_str(), vente.as_str()), ("53 % achat", "47 % vente"));
        // what the rows shown would have read (1 a side, 12 a side): other figures, once rounded too
        let lu = |k: usize| (argos_core::carnet::part_achat(&v.bids[..k], &v.asks[..k]).unwrap() * 100.0).round();
        assert!(lu(1) != 53.0 && lu(12) != 53.0, "{} {}", lu(1), lu(12));
        let t = Text::default();
        let dom = CarnetPrefs {
            onglet: Onglet::Dom,
            ..Default::default()
        };
        let carnet = CarnetPrefs::default();
        for (cell, w, h, entete, p) in [
            (false, 320.0, 250.0, false, &carnet),
            (true, 374.0, 552.0, true, &carnet),
            (false, 400.0, 820.0, true, &carnet),
            (true, 374.0, 552.0, true, &dom),
            (false, 400.0, 820.0, true, &dom),
        ] {
            let _ = crate::text::ecrits();
            let mut pm = Pixmap::new(w as u32, h as u32).unwrap();
            draw(&mut pm, &t, (w, h), p, &v, None, cell, entete);
            let e = crate::text::ecrits();
            let lus: Vec<&str> = e.iter().filter(|s| s.ends_with("% achat")).map(|s| s.as_str()).collect();
            assert_eq!(lus, [achat.as_str()], "{w}×{h} {:?}: the imbalance drawn", p.onglet);
            assert!(e.contains(&vente), "{w}×{h} {:?}", p.onglet);
        }
    }

    /// 📖 A side thinner than 10 levels sets the depth of both (the control's review, 25 Nov: 10 bids were compared
    /// with 4 asks, « 70 % achat » for a book even level by level).
    #[test]
    fn un_cote_mince_regle_la_profondeur_des_deux() {
        let mut v = vue_maquette();
        v.bids = (0..12).map(|i| (60_095.0 - 5.0 * i as f64, 10.0)).collect();
        v.asks = (0..4).map(|i| (60_100.0 + 5.0 * i as f64, 10.0)).collect();
        let textes = |v: &CarnetVue| textes_desequilibre(v).map(|(_, a, b)| (a, b));
        assert_eq!(textes(&v), Some(("50 % achat".into(), "50 % vente".into())));
        let t = Text::default();
        let _ = crate::text::ecrits();
        let mut pm = Pixmap::new(400, 820).unwrap();
        draw(&mut pm, &t, (400.0, 820.0), &CarnetPrefs::default(), &v, None, false, true);
        assert!(crate::text::ecrits().iter().any(|s| s == "50 % achat"), "drawn so");
        // the thin side's own sizes count: 4 asks twice as big, read on 4 a side
        v.asks.iter_mut().for_each(|l| l.1 = 20.0);
        assert_eq!(textes(&v), Some(("33 % achat".into(), "67 % vente".into())));
        // one side empty: nothing to compare, no line (the book's own message says why)
        v.asks.clear();
        assert_eq!(textes(&v), None);
    }

    /// 📖 In a volet or the phone's sheet the panel fits the height it is given: its footer and ⚙ in view (never below
    /// a scroll), its levels between them, the DOM's ladder down to the footer (capture of the safety net, 24 Nov: the
    /// phone's footer cut by the screen's bottom, the tablet's ⚙ below a scroll, an empty band under the DOM). The
    /// phone's sheet is given its height less the round Argos AI's room (watchlist's `bande_ai`).
    #[test]
    fn le_volet_et_la_feuille_tiennent_dans_leur_hauteur() {
        let t = Text::default();
        let v = vue_maquette();
        for (cell, w, h) in [(true, 374.0, 552.0), (true, 374.0, 690.0), (false, 400.0, 660.0), (false, 400.0, 820.0)] {
            for onglet in Onglet::ALL {
                let p = CarnetPrefs { onglet, ..Default::default() };
                let g = Geo::new(&t, w, h, &p, &v, cell, true);
                assert_eq!(g.onglet, onglet, "{cell} {h}: its tabs are there");
                assert!(g.pied_vu && g.reglages.w > 0.0, "{cell} {h} {onglet:?}: footer and ⚙ in view");
                assert!(g.reglages.y + g.reglages.h <= h + 0.01, "{cell} {h}: ⚙ inside the panel");
                // (where the rows start: under the Delta's window buttons and legend, the DOM's pills)
                let y = g.y + [0.0, 58.0, 30.0][onglet as usize];
                let n = niveaux(&p, y + COLS_H, g.pied - 4.0, g.rh);
                assert!(n >= 6, "{cell} {h} {onglet:?}: {n} levels a side");
                assert!(
                    y + COLS_H + MILIEU_H + 2.0 * n as f64 * g.rh <= g.pied - 4.0,
                    "{cell} {h}: rows above the footer"
                );
                if onglet == Onglet::Dom {
                    // the ladder: its rows, the middle one, the imbalance, down to the footer without a gap
                    let k = dom_niveaux(&g, y, &p);
                    let bas = y + COLS_H + (2 * k + 1) as f64 * g.rh * 0.96 + DESEQ_H;
                    assert!(bas <= g.pied - 4.0 && bas > g.pied - 4.0 - 2.0 * g.rh, "{cell} {h}: {bas} / {}", g.pied);
                }
                let mut pm = Pixmap::new(w as u32, h as u32).unwrap();
                draw(&mut pm, &t, (w, h), &p, &v, None, cell, true);
            }
        }
    }

    /// 📖 Lower than the panel's six levels (the phone lying down: its sheet less the round Argos AI's room, about 140
    /// to 170 points; a low window's volet): the footer and ⚙ stay at its bottom, in view, the levels make room, and
    /// with not even one a side a word says what to do, above the footer (the control's review, 25 Nov: lying down,
    /// the sheet scrolled and its footer and ⚙ went below the scroll, B-020 again).
    #[test]
    fn plus_bas_le_pied_et_le_reglage_restent_en_bas() {
        let t = Text::default();
        let v = vue_maquette();
        for (cell, w) in [(true, 833.0), (true, 1055.0), (false, 400.0)] {
            for h in [130.0, 138.0, 150.0, 172.0, 200.0, 230.0, 260.0, 300.0, 360.0, 420.0, 520.0] {
                for onglet in Onglet::ALL {
                    let p = CarnetPrefs { onglet, ..Default::default() };
                    let g = Geo::new(&t, w, h, &p, &v, cell, true);
                    let pad = if cell { 12.0 } else { 14.0 };
                    assert!(g.pied_vu && g.reglages.w > 0.0, "{cell} {h} {onglet:?}: footer and ⚙ in view");
                    assert!((g.reglages.y + g.reglages.h - (h - pad)).abs() < 0.01, "{cell} {h}: ⚙ at the bottom");
                    assert!(g.pied >= pad + 36.0, "{cell} {h}: the footer under the header, {}", g.pied);
                    let _ = crate::text::ecrits();
                    let mut pm = Pixmap::new(w as u32, h as u32).unwrap();
                    draw(&mut pm, &t, (w, h), &p, &v, None, cell, true);
                    let e = crate::text::ecrits();
                    let dit = if cell {
                        "Tourne le téléphone pour voir le carnet"
                    } else {
                        "Agrandis la fenêtre pour voir le carnet"
                    };
                    let y = g.y + [0.0, 58.0, 30.0][g.onglet as usize];
                    if y + COLS_H + MILIEU_H + 2.0 * g.rh > g.pied - 4.0 {
                        assert!(e.iter().any(|s| s == dit), "{cell} {h} {onglet:?}: {e:?}");
                    } else {
                        // at least one level a side, above the footer, and no word
                        let n = niveaux(&p, y + COLS_H, g.pied - 4.0, g.rh);
                        assert!(n >= 1 && y + COLS_H + MILIEU_H + 2.0 * n as f64 * g.rh <= g.pied - 4.0, "{cell} {h}");
                        assert!(!e.iter().any(|s| s == dit), "{cell} {h} {onglet:?}");
                    }
                    if h >= 420.0 {
                        assert_eq!(g.onglet, onglet, "{cell} {h}: its tabs");
                    }
                }
            }
        }
        // a short module (no header of its own): its ⚙ and footer go first by design, its settings are the volet's
        let g = Geo::new(&t, 320.0, 250.0, &CarnetPrefs::default(), &v, false, false);
        assert!(g.reglages.w == 0.0 && !g.pied_vu && g.reglages.y >= 250.0);
        let _ = crate::text::ecrits();
        let mut pm = Pixmap::new(320, 160).unwrap();
        draw(&mut pm, &t, (320.0, 160.0), &CarnetPrefs::default(), &v, None, false, false);
        assert!(crate::text::ecrits().iter().any(|s| s == "Agrandis le module pour voir le carnet"));
    }

    #[test]
    fn echelle_du_dom_reste_en_place() {
        let mut v = vue_maquette();
        assert_eq!(echelle(&v, 2), vec![60_105.0, 60_100.0, 60_095.0, 60_090.0, 60_085.0]);
        v.centre = 60_200.0;
        assert_eq!(echelle(&v, 1), vec![60_205.0, 60_200.0, 60_195.0], "its own centre until ⌖ Centrer");
    }

    /// A look at the panel (`ARGOS_CARNET_APERCU=<folder>`): the three tabs side by side like the mock-up, on the
    /// volet's glass, and the phone at 374 wide. Writes nothing otherwise.
    #[test]
    fn apercu_png() {
        let Ok(dir) = std::env::var("ARGOS_CARNET_APERCU") else { return };
        let t = Text::default();
        let th = crate::Theme::default();
        let mut v = vue_maquette();
        let pr = |o| CarnetPrefs {
            onglet: o,
            ..Default::default()
        };
        // Delta and DOM data like the mock-up's
        for (i, l) in v.asks.clone().iter().enumerate() {
            let m = Mouvement {
                pose: 4.0 + (i as f64 * 3.7) % 10.0,
                retire: 2.0 + (i as f64 * 5.3) % 9.0,
                rempli: if i % 3 == 0 { 2.6 } else { 0.0 },
            };
            v.delta.insert((false, argos_core::carnet::case(l.0, 5.0, false)), m);
        }
        for (i, l) in v.bids.clone().iter().enumerate() {
            let m = Mouvement {
                pose: 3.0 + (i as f64 * 4.1) % 11.0,
                retire: 1.5 + (i as f64 * 2.9) % 8.0,
                rempli: if i % 4 == 1 { 1.6 } else { 0.0 },
            };
            v.delta.insert((true, argos_core::carnet::case(l.0, 5.0, true)), m);
        }
        v.net = (84.0, -37.0);
        for k in -14..=14i64 {
            v.echange
                .insert(12_019 + k, (30.0 - (k.abs() as f64) * 1.7).max(0.0) * (1.0 + (k as f64 * 1.3).sin() * 0.4));
        }
        let mut pm = crate::px::new(1200, 760).unwrap();
        crate::fill(&mut pm, 0.0, 0.0, 1200.0, 760.0, th.chrome, 1.0);
        for (k, o) in Onglet::ALL.iter().enumerate() {
            let mut sub = crate::px::new(372, 740).unwrap();
            draw(&mut sub, &t, (372.0, 740.0), &pr(*o), &v, None, false, true);
            crate::px::put(&mut pm, 18.0 + k as f64 * 396.0, 10.0, &sub);
        }
        pm.save_png(format!("{dir}/carnet-onglets.png")).unwrap();
        let mut cell = crate::px::new(374, 520).unwrap();
        crate::fill(&mut cell, 0.0, 0.0, 374.0, 520.0, th.chrome, 1.0);
        draw(&mut cell, &t, (374.0, 520.0), &pr(Onglet::Carnet), &v, None, true, true);
        cell.save_png(format!("{dir}/carnet-390.png")).unwrap();
    }

    /// 📖 A book with every tab's figures: Delta's moves and the DOM's trades on each level (`prix`: the best ask,
    /// `pas`: its step).
    fn vue_pleine(prix: f64, pas: f64, dec: usize, q: f64) -> CarnetVue {
        let asks: Vec<(f64, f64)> = (0..12).map(|i| (prix + pas * i as f64, q * (1 + i % 4) as f64)).collect();
        let bids: Vec<(f64, f64)> = (0..12).map(|i| (prix - pas * (i + 1) as f64, q * (1 + i % 3) as f64)).collect();
        let mut v = CarnetVue {
            pas,
            mid: prix - pas / 2.0,
            monte: Some(true),
            ecart: pas,
            dec,
            unite: "₿".into(),
            murs: vec![(false, asks[3].0), (true, bids[2].0)],
            bourse: "Hyperliquid".into(),
            source: "Hyperliquid BTC-USDC".into(),
            en_direct: true,
            net: (128.9, -97.6),
            ..Default::default()
        };
        for (achat, l) in [(false, &asks), (true, &bids)] {
            for (px, _) in l.iter() {
                let m = Mouvement {
                    pose: 12.9,
                    retire: 9.7,
                    rempli: 3.4,
                };
                v.delta.insert((achat, cle(*px, pas, achat)), m);
                v.echange.insert(cle(*px, pas, true), 1_234.5);
            }
        }
        v.asks = asks;
        v.bids = bids;
        v
    }

    /// 📖 B-044: in a narrow module nothing is written over another text or past the panel's edges, in any tab, for a
    /// Bitcoin book (« 104 520,0 ») or a 222 $ one, in coins or in dollars: the header's pills (the exchange's name cut
    /// with « … », then gone), the column titles (« QTÉ ₿ »), the figures (smaller), the price without its spread, the
    /// imbalance (« 48 % ▬ 52 % »); too narrow for the Delta's and the DOM's figures and buttons, the tabs' bar goes and
    /// the book itself stays. At 174 wide, under the chart, « PRIX » and « QUANTITÉ ₿ » were written over each other
    /// (« PRQUANTITÉ ₿ ») and the « Hyperliquid ▾ » pill ran past the module's left edge. What is DRAWN is checked.
    #[test]
    fn rien_ne_se_chevauche_dans_un_module_etroit() {
        let t = Text::default();
        let livres = [vue_pleine(104_520.0, 5.0, 1, 0.25), vue_pleine(222.2, 0.25, 2, 2.0)];
        let mut tailles: Vec<(bool, bool, f64, f64)> = vec![];
        for w in (120..=420).step_by(4) {
            for h in [250.0, 420.0, 640.0] {
                tailles.push((false, false, w as f64, h));
            }
        }
        for w in (200..=420).step_by(4) {
            tailles.push((true, false, w as f64, 820.0));
            tailles.push((true, true, w as f64, 552.0));
        }
        let mut sans_onglets_a_174 = false;
        for v in &livres {
            for &(entete, cell, w, h) in &tailles {
                for onglet in Onglet::ALL {
                    for usd in [false, true] {
                        let p = CarnetPrefs {
                            onglet,
                            usd,
                            ..Default::default()
                        };
                        let _ = crate::text::boites();
                        let mut pm = Pixmap::new(w as u32, h as u32).unwrap();
                        draw(&mut pm, &t, (w, h), &p, v, None, cell, entete);
                        let b = crate::text::boites();
                        let ou = format!("{w}×{h} {onglet:?} usd {usd} volet {entete} cell {cell} ({})", v.asks[0].0);
                        for x in &b {
                            assert!(x.x0 >= -0.5 && x.x1 as f64 <= w + 0.5, "{ou}: « {} » past an edge: {x:?}", x.texte);
                        }
                        let haut = |x: &crate::text::Boite| (x.y - 0.75 * x.size, x.y + 0.15 * x.size);
                        for (i, a) in b.iter().enumerate() {
                            for c in &b[i + 1..] {
                                let ((a0, a1), (c0, c1)) = (haut(a), haut(c));
                                let dessus = a.x0.max(c.x0) < a.x1.min(c.x1) - 0.5 && a0.max(c0) < a1.min(c1) - 1.0;
                                assert!(!dessus, "{ou}: « {} » over « {} »: {a:?} {c:?}", a.texte, c.texte);
                            }
                        }
                        if w == 172.0 && !entete && h == 420.0 {
                            let g = Geo::new(&t, w, h, &p, v, cell, entete);
                            sans_onglets_a_174 |= g.onglets.w == 0.0 && g.onglet == Onglet::Carnet && g.bourse.x >= g.x0;
                        }
                    }
                }
            }
        }
        assert!(sans_onglets_a_174, "a module of 172: the book itself, its pills inside");
        // wide, nothing changes: the three tabs, the whole titles, the figures at 12.5, the whole imbalance line
        let v = &livres[0];
        let g = Geo::new(&t, 320.0, 640.0, &CarnetPrefs::default(), v, false, false);
        assert!(g.onglets.w > 0.0 && g.bourse_txt == "Hyperliquid ▾");
        assert_eq!(taille_carnet(g.iw, &CarnetPrefs::default(), v, 12), 12.5);
        let _ = crate::text::ecrits();
        let mut pm = Pixmap::new(320, 640).unwrap();
        draw(&mut pm, &t, (320.0, 640.0), &CarnetPrefs::default(), v, None, false, false);
        let e = crate::text::ecrits();
        assert!(e.iter().any(|s| s == "QUANTITÉ ₿") && e.iter().any(|s| s.ends_with("% achat")), "{e:?}");
    }

    /// 📖 B-026: the imbalance line stays inside its 12 px margins at every module width (at 280 wide it ran past both
    /// edges: « éséquilibre … 47 % vent »); the word goes first, then the bar shortens; wide, nothing changes. What is
    /// DRAWN is checked: no figure's pixel left of 12 px or right of the width less 12.
    #[test]
    fn le_desequilibre_tient_dans_un_module_etroit() {
        let t = Text::default();
        let v = vue_maquette();
        let (_, achat, vente) = textes_desequilibre(&v).unwrap();
        for w in [240.0, 260.0, 280.0, 300.0, 312.0, 320.0, 340.0, 372.0, 420.0] {
            let g = Geo::new(&t, w, 260.0, &CarnetPrefs::default(), &v, false, false);
            let (mot, court, barre, total) = place_desequilibre(&t, g.iw, &achat, &vente);
            assert!(total <= g.iw - 24.0 && !court, "{w}: {total} in {}", g.iw);
            assert!((24.0..=90.0).contains(&barre));
            assert!(mot || w < 372.0, "{w}: the volet's width keeps the whole line");
            assert!(!mot || w > 300.0, "{w}: no room for the word");
            // the drawing: the imbalance's own row, between the asks and the bids
            let mut pm = Pixmap::new(w as u32, 260).unwrap();
            draw(&mut pm, &t, (w, 260.0), &CarnetPrefs::default(), &v, None, false, false);
            let n = niveaux(&CarnetPrefs::default(), g.y + COLS_H, g.pied - 4.0, g.rh);
            let ym = g.y + COLS_H + n as f64 * g.rh;
            // (the line's baseline: 43 under the middle's top; its glyphs and bar from 12 above to 3 below)
            let (y0, y1) = ((ym + 31.0) as u32, (ym + 46.0) as u32);
            let encre = |x: u32| (y0..y1).any(|y| pm.pixel(x, y).is_some_and(|p| p.red().max(p.green()) > 90));
            let bords = (0..(g.x0 + 12.0) as u32).chain(((g.x0 + g.iw - 12.0) as u32)..w as u32);
            assert!(!bords.into_iter().any(encre), "{w}: ink in the margins on rows {y0}..{y1}");
        }
        assert!(place_desequilibre(&t, 600.0, &achat, &vente).0, "wide: the whole line");
    }

    #[test]
    fn textes_des_pilules() {
        let p = CarnetPrefs::default();
        let mut v = vue_maquette();
        assert_eq!(pas_texte(&v, &p), "5 $ ▾");
        v.pas = 0.0005;
        assert_eq!(pas_texte(&v, &p), "0,0005 $ ▾");
        assert_eq!(bourse_texte(&v), "Binance ▾");
        assert_eq!(echange_texte(&p), "Échangé : 1 h ▾");
        assert_eq!(
            echange_texte(&CarnetPrefs {
                echange_min: 15,
                ..Default::default()
            }),
            "Échangé : 15 min ▾"
        );
        assert_eq!(taille(74.43, 60_000.0, false), "74,43");
        assert_eq!(signe(9.7, false), "−9,7");
        // a Delta row where nothing moved: three dots, never « +0,0 » / « −0,0 » (the texts its cells draw)
        let rien = Mouvement {
            pose: 0.0,
            retire: -0.0,
            rempli: 0.0,
        };
        assert_eq!(textes_delta(&rien), ["·", "·", "·"]);
        let peu = Mouvement {
            pose: 0.04,
            retire: 0.049,
            rempli: 0.0,
        };
        assert_eq!(textes_delta(&peu), ["·", "·", "·"]);
        let m = Mouvement {
            pose: 0.05,
            retire: 9.7,
            rempli: 2.6,
        };
        assert_eq!(textes_delta(&m), ["+0,1", "−9,7", "2,6"]);
    }
}
