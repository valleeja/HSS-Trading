//! 🧱 Argos Murs on the chart, copied from the mockup v5 (maquettes/2026-11-20-maquette-murs-v5.png): a thin line from
//! where each wall appeared to its pill, the pills in one column against the price scale, grey RETIRÉ / TESTÉ / SPOOF
//! words behind them, thin bars of the summed book against the price scale, a summary pill and a colour key at the
//! bottom. Nothing overlaps and nothing is dropped: items too close are moved apart, with a short connector.
//!
//! Perp walls are red (sell) / green (buy); spot walls orange / blue with the word COMPTANT. Perp and spot sizes are
//! never summed: the same price on both gives one pill « … + COMPTANT x M $ ».

use crate::shapes::{round_fill, round_stroke, Rect};
use crate::text::{Align, Rgb, Text};
use crate::{fill, palette};
use argos_core::murs::{Fin, Lointain, Pill, Rempli, Vue, Word, PAR_BOOKS};
use argos_script::inputs::{check, choice, color as couleur, num, Inputs, Param};
use tiny_skia::Pixmap;

thread_local! {
    /// 🧱 What Argos Murs drew in the price area during its last drawing: every wall pill (far walls included), the
    /// « Carnets en direct » pill and the Perp / Comptant key, in the chart's points (as `vp.x_of` / `scale.y_of`).
    static DESSINEES: std::cell::RefCell<Vec<Rect>> = const { std::cell::RefCell::new(Vec::new()) };
    /// What the app lays over the chart (`Chart::garder`: the floating toolbar, the legend's unfolded list): no wall
    /// pill is drawn under it.
    static GARDER: std::cell::RefCell<Vec<Rect>> = const { std::cell::RefCell::new(Vec::new()) };
    /// 📋 (tableaux-1 tour 2, F2) A table is selected by the finger: its hint pill takes the bottom row.
    static SANS_BAS: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// The visible candles' extents (slot width × high-low, wicks included) for the next drawing: a pill that would
    /// touch one is shortened ([`Forme`]).
    static BOUGIES: std::cell::RefCell<Vec<Rect>> = const { std::cell::RefCell::new(Vec::new()) };
    /// How the price scale writes a price (mode, base of Percent / Indexed, decimals): a short pill writes its price
    /// exactly like the scale's labels; `None`: the wall's own step decides the decimals.
    static AXE: std::cell::Cell<Option<(crate::settings::ScaleMode, f64, usize)>> = const { std::cell::Cell::new(None) };
    /// F1i: the walls' price tags for the price scale of the last drawing ([`etiquettes_axe`]).
    static AXE_MURS: std::cell::RefCell<Vec<EtiquetteAxe>> = const { std::cell::RefCell::new(Vec::new()) };
    /// 🫧 The bubble of pressure of the last drawing (F1g): a tap on it opens its tooltip.
    static BULLE_PRESSION: std::cell::Cell<Option<Rect>> = const { std::cell::Cell::new(None) };
    /// The wall pills (and dots) of the last drawing: a tap on one opens its tooltip.
    static PILULES: std::cell::RefCell<Vec<(Rect, String)>> = const { std::cell::RefCell::new(Vec::new()) };
    /// 📋 (tableaux-1) Every wall pill at its full length in the last drawing (before it gave way to a table): a moved
    /// table keeps clear of them.
    static PLEINES: std::cell::RefCell<Vec<Rect>> = const { std::cell::RefCell::new(Vec::new()) };
    /// 👆 Where the finger tapped a wall pill (chart): its tooltip stays open there until the next tap.
    static BULLE: std::cell::Cell<Option<(f64, f64)>> = const { std::cell::Cell::new(None) };
    /// ≫ F1j: when the tapped tooltip opened (ms): it closes by itself after [`BULLE_MS`].
    static BULLE_T: std::cell::Cell<i64> = const { std::cell::Cell::new(0) };
    /// F1j: the depth bars of the last drawing (y, width) and the plot's width: a tap on one opens its tooltip.
    static BARRES: std::cell::RefCell<(Vec<(f64, f64)>, f64)> = const { std::cell::RefCell::new((Vec::new(), 0.0)) };
}

/// ≫ F1j: a tapped tooltip closes by itself after 4 s.
pub const BULLE_MS: i64 = 4_000;

/// 🧱 The visible candles' extents for the next drawing (chart points): no pill is drawn over one at full length.
pub fn set_bougies(v: Vec<Rect>) {
    BOUGIES.with(|b| *b.borrow_mut() = v);
}

/// 🧱 How the price scale writes a price, for the next drawing (see [`prix_axe`]).
pub fn set_axe(mode: crate::settings::ScaleMode, base: f64, exact: usize) {
    AXE.with(|a| a.set(Some((mode, base, exact))));
}

/// 🧱 A price written like the price scale's labels (the last [`set_axe`]): « 65 000 » for a round price, the
/// decimals only when the wall's price has them (F1b, 22 Nov: the pills wrote « 65 000,0 »), never more than the
/// symbol's precision; without a scale, with the decimals of the wall's `step`.
pub fn prix_axe(p: f64, step: f64) -> String {
    match AXE.with(|a| a.get()) {
        Some((mode, base, exact)) => crate::scale_text(mode, base, p, dec_of(p.abs()).min(exact)),
        None => crate::viewport::format_price(p, dec_of(step.max(0.0))),
    }
}

/// 👆 Is there a wall pill (or dot) of the last drawing under `(x, y)` (chart points, a finger's tolerance)?
pub fn pilule_en(x: f64, y: f64) -> bool {
    PILULES.with(|p| p.borrow().iter().any(|(r, _)| touche(r, x, y))) || BULLE_PRESSION.with(|b| b.get()).is_some_and(|r| r.contains(x, y))
}

/// 🫧 F1g: one smoothing step of the book's pressure (the sells' share, 0 to 1): exponential, 30 s time constant, so a
/// single jump of the books moves the bubble only part of the way. `prev`: (smoothed value, its time in ms).
pub fn lisser_pression(prev: Option<(f64, i64)>, x: f64, now_ms: i64) -> (f64, i64) {
    match prev {
        Some((p, t)) if p.is_finite() => {
            let a = 1.0 - (-((now_ms - t).max(0) as f64) / 30_000.0).exp();
            (p + a * (x - p), now_ms)
        }
        _ => (x, now_ms),
    }
}

/// 🫧 The bubble's figure, colour side and words: (share of the dominant side in %, `Some(true)` sells /
/// `Some(false)` buys / `None` balanced, 45–55 %), from the sells' share `s`.
pub fn pression_cote(s: f64) -> (u32, Option<bool>) {
    let dom = s.max(1.0 - s);
    let pct = (dom * 100.0).round() as u32;
    (pct, (pct > 55).then_some(s > 0.5))
}

/// 🫧 The bubble's tooltip: « Pression du carnet ±3 % : vendeurs 56 % · achats 44 % » (or « équilibré »).
pub fn pression_texte(s: f64, range: f64) -> String {
    let v = (s * 100.0).round() as u32;
    let r = crate::viewport::format_number(range.clamp(0.5, 5.0), if range.fract() == 0.0 { 0 } else { 1 });
    let eq = if pression_cote(s).1.is_none() { "équilibré · " } else { "" };
    format!("Pression du carnet ±{r} % : {eq}vendeurs {v} % · achats {} %", 100 - v.min(100))
}

/// 🧱 The wall pills of the last drawing: each box (a dot's box for a dot) and its text (empty: a dot).
pub fn pilules_dessinees() -> Vec<(Rect, String)> {
    PILULES.with(|p| p.borrow().clone())
}

/// 🧱 The candles' extents given to the last drawing ([`set_bougies`]).
pub fn bougies() -> Vec<Rect> {
    BOUGIES.with(|b| b.borrow().clone())
}

/// 👆 Opens (`Some`, chart points) or closes the tooltip of a tapped wall pill (or depth bar).
pub fn set_bulle(at: Option<(f64, f64)>) {
    set_bulle_a(at, crate::journal::now_ms());
}

/// Like [`set_bulle`], opened at `now_ms`.
pub fn set_bulle_a(at: Option<(f64, f64)>, now_ms: i64) {
    BULLE.with(|b| b.set(at));
    BULLE_T.with(|t| t.set(now_ms));
}

/// 👆 Where the tooltip of a tapped wall pill is open, if it is (F1j: for 4 s at most).
pub fn bulle() -> Option<(f64, f64)> {
    bulle_a(crate::journal::now_ms())
}

/// Like [`bulle`] at `now_ms`.
pub fn bulle_a(now_ms: i64) -> Option<(f64, f64)> {
    BULLE.with(|b| b.get()).filter(|_| now_ms - BULLE_T.with(|t| t.get()) < BULLE_MS)
}

/// 👆 F1j: is there a depth bar of the last drawing under `(x, y)` (chart points, a finger's tolerance)?
pub fn barre_en(x: f64, y: f64) -> bool {
    BARRES.with(|b| {
        let (rows, pw) = &*b.borrow();
        x <= *pw && rows.iter().any(|(by, w)| touche(&Rect::new(pw - w, by - BAR_H / 2.0, *w, BAR_H), x, y))
    })
}

/// 👆 F1j: where a tapped tooltip goes (by the finger): under the tapped pill or bar `cible`, right-aligned against
/// the price scale; above it when there is no room under; never on a candle ([`bougies`]) — else the nearest free
/// height, else under the target.
pub fn cadre_toucher(w: f64, h: f64, cible: Rect, pw: f64, ph: f64) -> Rect {
    let x = (pw - 4.0 - w).max(4.0);
    let bougies = bougies();
    let libre = |r: &Rect| r.y >= 2.0 && r.y + r.h <= ph - 2.0 && !bougies.iter().any(|b| croise(b, r));
    let sous = Rect::new(x, cible.y + cible.h + 6.0, w, h);
    let dessus = Rect::new(x, cible.y - 6.0 - h, w, h);
    if libre(&sous) {
        return sous;
    }
    if libre(&dessus) {
        return dessus;
    }
    let mut ys: Vec<f64> = (0..((ph - h) / 6.0).max(0.0) as usize).map(|k| 2.0 + k as f64 * 6.0).collect();
    ys.sort_by(|a, b| (a - cible.cy()).abs().total_cmp(&(b - cible.cy()).abs()));
    ys.into_iter()
        .map(|y| Rect::new(x, y, w, h))
        .find(libre)
        .unwrap_or(Rect::new(x, sous.y.clamp(2.0, (ph - h - 2.0).max(2.0)), w, h))
}

/// A pill (or a dot) under `(x, y)`: its box grown to at least 24 × 24 points around its middle (a finger, a dot).
fn touche(r: &Rect, x: f64, y: f64) -> bool {
    let (w, h) = (r.w.max(24.0), r.h.max(24.0));
    Rect::new(r.cx() - w / 2.0, r.cy() - h / 2.0, w, h).contains(x, y)
}

/// 🧱 How much of a wall pill is written (F1i, Jee 22 Nov 20:36: « enlève le prix des pastilles, on garde juste le
/// montant d'ordre »): the amount alone, then a dot on the edge of the price scale while it would touch a candle or
/// a reserved zone. The side is the colour, the price is on the price scale (or, off screen, in the tooltip with the
/// side, the exchanges and the spot part). A wall is never dropped for that.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Forme {
    /// PC: « 12,4 M $ ».
    Complet,
    /// By the finger: « 12 M », « 8,1 M », « 540 k ».
    Montant,
    /// A small dot of the pill's colour on the edge of the price scale.
    Point,
}

/// The shapes tried in turn: PC the amount (« 12,4 M $ »), by the finger the short amount, then the dot.
pub fn formes(tactile: bool) -> &'static [Forme] {
    if tactile {
        &[Forme::Montant, Forme::Point]
    } else {
        &[Forme::Complet, Forme::Point]
    }
}

/// The text of `q` written as `f` (no tag in a pill any more: BINANCE and HYPERLIQUID are in the tooltip).
pub fn forme_texte(q: &Etiquette, f: Forme) -> (String, Option<Marque>) {
    match f {
        Forme::Complet => (dollars(q.usd), None),
        Forme::Montant | Forme::Point => (montant_court(q.usd), None),
    }
}

/// F1i: a wall's amount, short: « 54 M », « 8,1 M », « 540 k », « 1,2 B » (the user's number format).
pub fn montant_court(usd: f64) -> String {
    let a = usd.abs();
    let (v, u) = if a >= 1e9 {
        (usd / 1e9, "B")
    } else if a >= 1e6 {
        (usd / 1e6, "M")
    } else {
        (usd / 1e3, "k")
    };
    let d = if u != "k" && v.abs() < 10.0 && (v * 10.0).round() % 10.0 != 0.0 {
        1
    } else {
        0
    };
    format!("{} {u}", crate::viewport::format_number(v, d))
}

/// F1i: a wall's price tag on the price scale: its price as the scale writes it, its colours (two for a perp wall
/// and its spot twin at the same price: left half, right half), its money (the bigger stays when two meet).
#[derive(Debug, Clone, PartialEq)]
pub struct EtiquetteAxe {
    pub price: f64,
    pub texte: String,
    pub couleurs: Vec<Rgb>,
    pub usd: f64,
    /// A wall off screen (Jee, 20:37: « prix ici »): the row of its pill at the band's edge (the tag goes there, not
    /// at its price) and its arrow, `true` ▲ (above the screen) / `false` ▼; `None`: a wall on screen, at its price.
    pub bord: Option<(f64, bool)>,
}

/// F1i: the walls' price tags of the last drawing (walls in the band whose pill is drawn), biggest first.
pub fn etiquettes_axe() -> Vec<EtiquetteAxe> {
    AXE_MURS.with(|a| a.borrow().clone())
}

/// F1i: which wall tags go on the scale, at `y_of(price)` (a tag `h` high, centred on its price): never one on
/// another (the bigger wall's stays), never on the last-price label `dernier` (y top, y bottom), inside `0 ..= bas`.
pub fn tags_axe(tags: &[EtiquetteAxe], y_of: &dyn Fn(f64) -> f64, h: f64, bas: f64, dernier: Option<(f64, f64)>) -> Vec<(f64, EtiquetteAxe)> {
    let mut l: Vec<&EtiquetteAxe> = tags.iter().collect();
    l.sort_by(|a, b| b.usd.total_cmp(&a.usd));
    let mut out: Vec<(f64, EtiquetteAxe)> = vec![];
    for t in l {
        let y = t.bord.map_or_else(|| y_of(t.price).round(), |(y, _)| y.round());
        let (a, b) = (y - h / 2.0, y + h / 2.0);
        if a < 0.0 || b > bas {
            continue;
        }
        if dernier.is_some_and(|(da, db)| a < db && da < b) {
            continue;
        }
        // (F1m: a wall at the same price as written, met on the scale — perp and spot — gives its colour to the tag
        // there: one tag in two colours, never one colour for two walls)
        if let Some((_, o)) = out.iter_mut().find(|(oy, _)| (oy - y).abs() < h) {
            if o.texte == t.texte && o.bord.is_none() && t.bord.is_none() {
                for c in &t.couleurs {
                    if !o.couleurs.contains(c) {
                        o.couleurs.push(*c);
                    }
                }
            }
            continue;
        }
        out.push((y, t.clone()));
    }
    out
}

/// F1l: the gap between thousands on a wall's price tag: a regular word space, so the bold tag reads like the
/// scale's label it stands for (« 66 000 »). The narrow space (1.9 points in bold, 2.2 in regular at 12 points) was
/// lost between bold digits: « 61000 », « 61400 » (build 202611230034).
pub fn espace_milliers(text: &Text, size: f32) -> f64 {
    text.width(" ", size) as f64
}

/// F1l: width of a wall's price as [`draw_prix`] writes it.
pub fn largeur_prix(gras: &Text, text: &Text, s: &str, size: f32) -> f64 {
    let parts: Vec<&str> = s.split('\u{202f}').collect();
    parts.iter().map(|p| gras.width(p, size) as f64).sum::<f64>() + espace_milliers(text, size) * parts.len().saturating_sub(1) as f64
}

/// F1l: a wall's price on the scale in bold, from `x` on the baseline `y`, the thousands as far apart as on the
/// scale's labels ([`espace_milliers`]).
#[allow(clippy::too_many_arguments)]
pub fn draw_prix(pm: &mut Pixmap, gras: &Text, text: &Text, s: &str, x: f64, y: f32, size: f32, c: Rgb) {
    let mut x = x;
    for (k, part) in s.split('\u{202f}').enumerate() {
        if k > 0 {
            x += espace_milliers(text, size);
        }
        gras.draw(pm, part, (x as f32, y), size, c, Align::Left);
        x += gras.width(part, size) as f64;
    }
}

/// Side of the dot drawn for [`Forme::Point`] (points).
pub fn point_d(t: &Tailles) -> f64 {
    t.carre
}

/// Does `r` (grown by 2 points) touch one of `obstacles`?
pub fn gene(r: &Rect, obstacles: &[Rect]) -> bool {
    let g = Rect::new(r.x - 2.0, r.y - 2.0, r.w + 4.0, r.h + 4.0);
    obstacles.iter().any(|o| croise(&g, o))
}

/// 🧱 The rects Argos Murs drew in the price area in its last drawing (empty: off, hidden or no book — see
/// [`oublier`]). The high / low price pills keep clear of them; they are drawn before Murs in a frame, so they read
/// the last frame's, which are the same while nothing moves.
pub fn zones_dessinees() -> Vec<Rect> {
    DESSINEES.with(|d| d.borrow().clone())
}

/// 📋 (tableaux-1) The wall pills of the last drawing at their full length (as they would be with nothing in their
/// way): a table moved by the user snaps off them.
pub fn pilules_pleines() -> Vec<Rect> {
    PLEINES.with(|p| p.borrow().clone())
}

/// 🧱 Argos Murs is not drawn this frame: no rect is left behind.
pub fn oublier() {
    DESSINEES.with(|d| d.borrow_mut().clear());
    PLEINES.with(|p| p.borrow_mut().clear());
    AXE_MURS.with(|a| a.borrow_mut().clear());
    PILULES.with(|p| p.borrow_mut().clear());
    BARRES.with(|b| b.borrow_mut().0.clear());
}

/// 📋 (tableaux-1 tour 2, F2) While a table is selected by the finger, the bottom pills (« Carnets en direct… », the
/// pressure bubble and the colour key) are not drawn: the hint pill takes their row (its band stays free of walls).
pub fn set_sans_bas(on: bool) {
    SANS_BAS.with(|s| s.set(on));
}

/// 🧱 The app's rects over the chart for the next drawing: [`draw`] places no wall pill under them.
pub fn set_garder(v: &[Rect]) {
    GARDER.with(|g| *g.borrow_mut() = v.to_vec());
}

fn croise(a: &Rect, b: &Rect) -> bool {
    a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h
}

pub const GROUPS: [&str; 5] = ["🧱 Murs", "🏦 Bourses", "🔔 Alertes", "🎨 Couleurs", "⏪ Relecture"];

/// Books read: perp, spot, or both.
pub const CARNETS: [&str; 3] = ["Perp", "Comptant", "Les deux"];

pub const PARAMS: [Param; 42] = [
    choice("carnets", 0, "Carnets", 2, &CARNETS),
    num("perN", 0, "Murs par côté (perp)", 3.0, 1.0, 6.0, 1.0),
    num("spotN", 0, "Murs par côté (comptant, avec « Les deux »)", 2.0, 0.0, 6.0, 1.0),
    num(
        "mult",
        0,
        "Taille minimum si les murs pas solides sont cachés (× la médiane)",
        1.5,
        1.0,
        50.0,
        0.5,
    ),
    num("minEx", 0, "Mur solide : vu sur au moins (bourses)", 3.0, 1.0, 5.0, 1.0),
    num("holdMin", 0, "Mur solide : tenu depuis au moins (minutes)", 3.0, 0.0, 60.0, 1.0),
    num("range", 0, "Profondeur lue (% du prix)", 3.0, 0.5, 5.0, 0.5),
    Param {
        tip: AIDE_BARRES,
        ..check("bars", 0, "Barres de profondeur contre l'échelle de prix", true)
    },
    check("ghosts", 0, "Murs SPOOF (posés puis retirés plusieurs fois)", true),
    num("ghostH", 0, "Anciens murs effacés après (heures)", 4.0, 0.5, 48.0, 0.5),
    check("sweep", 0, "Pastille SWEEP sur la mèche", false),
    check("summary", 0, "Résumé en bas", true),
    check("whales", 0, "Baleines (grosses transactions)", false),
    // exchanges (one box each, perp then spot)
    check("exBinance", 1, "Binance Futures (perp)", true),
    check("exBybit", 1, "Bybit (perp)", true),
    check("exOkx", 1, "OKX (perp)", true),
    check("exBitunix", 1, "Bitunix (perp)", true),
    check("exHyperliquid", 1, "Hyperliquid (perp)", true),
    check("spBinance", 1, "Binance (comptant)", true),
    check("spBybit", 1, "Bybit (comptant)", true),
    check("spOkx", 1, "OKX (comptant)", true),
    check("spCoinbase", 1, "Coinbase (comptant)", true),
    // alerts: all off by default
    check("alRetire", 2, "Alerte : mur retiré", false),
    check("alTeste", 2, "Alerte : mur testé", false),
    check("alSweep", 2, "Alerte : sweep", false),
    check("alMange", 2, "Alerte : mur mangé", false),
    check("alSpoof", 2, "Alerte : spoof", false),
    couleur("cPerpSell", 3, "Vente perp", 0xF23645),
    couleur("cPerpBuy", 3, "Achat perp", 0x089981),
    couleur("cSpotSell", 3, "Vente comptant", 0xFF9800),
    couleur("cSpotBuy", 3, "Achat comptant", 0x2F8CFF),
    num("keepDays", 4, "Garder les murs enregistrés (jours)", 30.0, 1.0, 90.0, 1.0),
    // (last, so the settings already saved keep their places)
    check("weak", 0, "Montrer aussi les murs pas encore solides (contour fin)", true),
    // an action, not a setting: ticked, the price scale is set on the walls once and the box unticks itself
    check("cadrer", 0, "Cadrer sur les murs (échelle de prix ±3 % autour du prix)", false),
    check("alRempli", 2, "Alerte : mur rempli (vraiment traité)", false),
    num("rempliMin", 2, "Alerte mur rempli : taille minimum (M $)", 5.0, 0.0, 1000.0, 0.5),
    // Jee, 21 Nov 20:23 UTC: « tous les murs + 1 de Binance »
    check("binance", 0, "Toujours montrer un mur de Binance", true),
    // Jee, 21 Nov 20:33 UTC: a book wall lives minutes to hours, useless above 4 h
    check("cache4h", 0, "Cacher au-dessus de 4 h", true),
    // lot loin-1 (Jee, 22 Nov: « Oui, 2 par côté »): Hyperliquid's far groups, beyond the near range
    num("loinN", 0, "Murs lointains par côté (0 = aucun)", 2.0, 0.0, 3.0, 1.0),
    num("loinPct", 0, "Murs lointains : jusqu'à (% du prix)", 15.0, 5.0, 30.0, 5.0),
    // lot loin-1 B: Binance perp's book from its photos and its diff depth stream
    check("binFlux", 0, "Binance : carnet complet en direct", true),
    // F1g (Jee, 22 Nov, option A): the book's pressure in a round bubble beside the summary
    check("pression", 0, "Bulle de pression", true),
];

/// Help line of the depth bars' box: a band read from fewer books is paler.
pub const AIDE_BARRES: &str = "Plus pâle = moins de carnets à ce prix.";

/// Above this timeframe (seconds) nothing is drawn when « Cacher au-dessus de 4 h » is on.
pub const CACHE_AU_DESSUS_S: i64 = 4 * 3_600;

/// Is the drawing hidden at the chart's timeframe `tf_s` (seconds)?
pub fn cache(p: &Inputs, tf_s: i64) -> bool {
    p.on("cache4h") && tf_s > CACHE_AU_DESSUS_S
}

/// Default colours (the mockup v5's; the ⚙ can change them).
pub const PERP_SELL: Rgb = Rgb(0xF2, 0x36, 0x45);
pub const PERP_BUY: Rgb = Rgb(0x08, 0x99, 0x81);
pub const SPOT_SELL: Rgb = Rgb(0xFF, 0x98, 0x00);
pub const SPOT_BUY: Rgb = Rgb(0x2F, 0x8C, 0xFF);
/// Grey of the words behind (RETIRÉ, TESTÉ, SPOOF n×).
pub const WORD: Rgb = Rgb(0x8C, 0x90, 0x9C);
/// Inside of the pills (glass on the chart).
pub const PILL_BG: Rgb = Rgb(0x13, 0x16, 0x20);

/// What the app gives the drawing (live or replayed).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Ecran {
    pub vue: Vue,
    /// Bars of the summed book: (price, $, sell).
    pub depth: Vec<(f64, f64, bool)>,
    /// Each bucket of `depth` split by book ($, in the order of `noms`; empty: no split known).
    pub detail: Vec<[f64; PAR_BOOKS]>,
    /// Name of each book of `detail`.
    pub noms: Vec<String>,
    /// Name of each book of a perp pill's and a spot pill's split (`Pill::par`).
    pub noms_perp: Vec<String>,
    pub noms_spot: Vec<String>,
    /// Per book read: (lowest, highest price) its snapshot covers; a depth row's coverage class (lot 3c).
    pub reach: Vec<(f64, f64)>,
    /// Width of one depth bucket (the fine grid of the data layer, BTC: 5 $); 0: points.
    pub depth_step: f64,
    pub step: f64,
    /// Last price (depth bars above it are sell-coloured, below buy-coloured); 0: unknown (each bar keeps its side).
    pub last: f64,
    /// Summed book within `range` % (coins): sell side, buy side; `None`: not live (replay).
    pub totals: Option<(f64, f64)>,
    /// 🫧 The sells' share of the summed book (0 to 1), smoothed over 30 s by the app ([`lisser_pression`]); `None`:
    /// no bubble of pressure.
    pub pression: Option<f64>,
    pub coin: String,
    pub range: f64,
    /// Spot exchanges named in the key (« Binance · Bybit · OKX · Coinbase »); empty: no spot.
    pub spot_names: String,
    pub perp: bool,
    pub spot: bool,
    /// Sweeps: (bar time s, price, sell).
    pub sweeps: Vec<(i64, f64, bool)>,
    /// Clock of the view (ms; live: now, replay: the replay clock): a wall's age for its quality mark.
    pub now_ms: i64,
    /// Books that answered (perp, or spot alone): a wall seen on all of them is solid even when fewer than the minimum.
    pub books: u32,
    /// One candle of the chart (ms): a REMPLI mark stays `REMPLI_BARS` candles.
    pub bar_ms: i64,
    /// Top of the column (pixels): on a phone, under the legend that spans the whole width; 0: the plot's top.
    pub haut: f64,
    /// Free band at the right of the last candle (pixels, the app's margin): a pill with a merged spot part wider
    /// than it is drawn as two pills; 0: no limit.
    pub marge: f64,
    /// 🧱 Far walls (lot loin-1: Hyperliquid's far groups beyond the near range), highest price first; empty in
    /// replay.
    pub loin: Vec<Lointain>,
    /// Nothing to draw yet, said in a small grey pill against the price column (« Lecture des carnets… », « Pas de
    /// carnet public pour ce symbole »); `None`: the walls are drawn.
    pub etat: Option<String>,
}

/// A REMPLI mark stays this many candles of the chart, fading out.
pub const REMPLI_BARS: i64 = 5;

/// The REMPLI marks still shown at `now_ms` (younger than `REMPLI_BARS` candles), with their opacity (fading out).
pub fn remplis_visibles(l: &[Rempli], now_ms: i64, bar_ms: i64) -> Vec<(Rempli, f32)> {
    let vie = (REMPLI_BARS * bar_ms.max(1)) as f64;
    l.iter()
        .filter_map(|r| {
            let age = (now_ms - r.at_ms) as f64;
            (0.0..vie).contains(&age).then(|| (*r, (0.75 * (1.0 - age / vie)).max(0.2) as f32))
        })
        .collect()
}

/// `c` at opacity `a` over the chart's dark background (text has no opacity of its own).
fn attenue(c: Rgb, a: f32) -> Rgb {
    let m = |x: u8, f: u8| (f as f32 + (x as f32 - f as f32) * a).round() as u8;
    Rgb(m(c.0, PILL_BG.0), m(c.1, PILL_BG.1), m(c.2, PILL_BG.2))
}

fn dec_of(step: f64) -> usize {
    let mut d = 0;
    let mut s = step;
    while d < 8 && (s - s.round()).abs() > 1e-9 {
        s *= 10.0;
        d += 1;
    }
    d
}

/// « 18,4 M $ ».
pub fn dollars(usd: f64) -> String {
    format!("{} $", crate::bookheat::compact(usd))
}

/// « 4 346 » (coins, no decimals from 100 up).
pub fn coins(x: f64) -> String {
    let (n, d) = if x >= 100.0 { (x.round(), 0) } else { (x, 1) };
    crate::viewport::format_number(n, d)
}

fn side_word(sell: bool) -> &'static str {
    if sell {
        "VENTE"
    } else {
        "ACHAT"
    }
}

/// One pill of the column: its text, colour, size (to decide who stays) and where its line starts.
#[derive(Debug, Clone, PartialEq)]
pub struct Etiquette {
    pub price: f64,
    pub texte: String,
    pub couleur: Rgb,
    pub usd: f64,
    pub sell: bool,
    pub spot: bool,
    pub since_ms: i64,
    /// Solid wall (seen on enough exchanges, held long enough): full outline; otherwise thinner and dimmer.
    pub fort: bool,
    /// The tag at the right end of the pill: BINANCE (phone: BIN.) on the tag exchange's wall, HYPERLIQUID (phone:
    /// HYP.) on a far wall; `None`: no tag.
    pub tag: Option<Marque>,
    /// A far wall (lot loin-1): never the side's biggest wall, its money all Hyperliquid's.
    pub loin: bool,
    /// The spot part of a perp wall at the same price, drawn as its own pill next to it (tagged or too wide).
    pub jumeau: bool,
    /// Its money per book (`Pill::par`), and the spot twin's amount when merged into it (« + COMPTANT x M $ »).
    pub par: [f64; PAR_BOOKS],
    pub comptant: f64,
}

/// The tag written on the tag exchange's pill (PC, phone).
pub const TAG: &str = "BINANCE";
pub const TAG_COURT: &str = "BIN.";
/// The tag written on a far wall's pill (PC, phone).
pub const TAG_LOIN: &str = "HYPERLIQUID";
pub const TAG_LOIN_COURT: &str = "HYP.";

/// Which tag a pill carries (drawn the same way: white spaced letters at the right end).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Marque {
    /// The tag exchange's wall (Binance), never dropped by the room rule.
    Binance,
    /// A far wall (Hyperliquid's far book).
    Hyperliquid,
}

impl Marque {
    /// The tag's text (PC, `short`: phone).
    pub fn texte(self, short: bool) -> &'static str {
        match (self, short) {
            (Marque::Binance, false) => TAG,
            (Marque::Binance, true) => TAG_COURT,
            (Marque::Hyperliquid, false) => TAG_LOIN,
            (Marque::Hyperliquid, true) => TAG_LOIN_COURT,
        }
    }
}
/// The tag: white, bold, small caps size, spaced letters, about 18 px after the amount (mockup v6).
pub const TAG_COULEUR: Rgb = Rgb(0xE8, 0xEB, 0xF2);
pub const TAG_FONT: f32 = 11.5;
pub const TAG_FONT_COURT: f32 = 9.0;
pub const TAG_ESPACE: f32 = 0.8;
pub const TAG_GAP: f64 = 18.0;
pub const TAG_GAP_COURT: f64 = 4.0;

/// Is a wall solid (setting « Mur solide »)? Seen on at least `minEx` exchanges (or on every book that answered,
/// when fewer answered) and shown for at least `holdMin` minutes.
pub fn solide(q: &Pill, e: &Ecran, p: &Inputs) -> bool {
    let need = (p.get("minEx").max(1.0) as u32).min(e.books.max(1));
    q.venues >= need && e.now_ms - q.since_ms >= (p.get("holdMin") * 60_000.0) as i64
}

/// The pills to draw: perp and spot walls at the same price become one pill (perp colour, « + COMPTANT x M $ »).
/// The four colours of the ⚙: perp sell, perp buy, spot sell, spot buy.
pub fn couleurs(p: &Inputs) -> [Rgb; 4] {
    let c = |k: &str| {
        let (r, g, b) = p.rgb(k);
        Rgb(r, g, b)
    };
    [c("cPerpSell"), c("cPerpBuy"), c("cSpotSell"), c("cSpotBuy")]
}

pub fn etiquettes(e: &Ecran, inp: &Inputs, short: bool, cols: [Rgb; 4]) -> Vec<Etiquette> {
    etiquettes_dans(e, inp, short, cols, &|_| true)
}

/// Like [`etiquettes`]; `fusion(texte merged)`: may a perp pill carry its spot twin's « + COMPTANT x M $ » (it fits in
/// the band)? A tagged pill never does: its spot part is its own pill (mockup v6, lot 3c d).
pub fn etiquettes_dans(e: &Ecran, inp: &Inputs, short: bool, cols: [Rgb; 4], fusion: &dyn Fn(&str) -> bool) -> Vec<Etiquette> {
    let dec = dec_of(e.step);
    // (F1b: « 65 000 » for a round price, as the price scale writes it; the decimals only when the price has them)
    let fp = |p: f64| crate::viewport::format_price(p, dec_of(p.abs()).min(dec));
    let (perp, spot): (Vec<&Pill>, Vec<&Pill>) = e.vue.pills.iter().partition(|p| !p.spot);
    let mut out = vec![];
    let near = |a: f64, b: f64| (a - b).abs() < e.step.max(f64::EPSILON) * 0.5;
    let compt = if short { "COMPT." } else { "COMPTANT" };
    let mut fusionnes: Vec<(bool, f64)> = vec![];
    for p in &perp {
        let twin = spot.iter().find(|s| s.sell == p.sell && near(s.price, p.price));
        let mut texte = format!("{} {} · {}", side_word(p.sell), fp(p.price), dollars(p.usd));
        let mut comptant = 0.0;
        if let Some(s) = twin {
            let avec = format!("{texte} + {compt} {}", dollars(s.usd));
            if !p.tag && fusion(&avec) {
                texte = avec;
                comptant = s.usd;
                fusionnes.push((s.sell, s.price));
            }
        }
        out.push(Etiquette {
            price: p.price,
            texte,
            couleur: if p.sell { cols[0] } else { cols[1] },
            usd: p.usd,
            sell: p.sell,
            spot: false,
            since_ms: p.since_ms,
            fort: solide(p, e, inp),
            tag: p.tag.then_some(Marque::Binance),
            loin: false,
            jumeau: false,
            par: p.par,
            comptant,
        });
    }
    for s in &spot {
        if fusionnes.iter().any(|(sell, px)| *sell == s.sell && near(*px, s.price)) {
            continue;
        }
        out.push(Etiquette {
            price: s.price,
            texte: format!("{} {} · {} {compt}", side_word(s.sell), fp(s.price), dollars(s.usd)),
            couleur: if s.sell { cols[2] } else { cols[3] },
            usd: s.usd,
            sell: s.sell,
            spot: true,
            since_ms: s.since_ms,
            fort: solide(s, e, inp),
            tag: None,
            loin: false,
            jumeau: perp.iter().any(|p| p.sell == s.sell && near(s.price, p.price)),
            par: s.par,
            comptant: 0.0,
        });
    }
    // far walls (lot loin-1): perp colours, thin dim outline, the HYPERLIQUID tag
    for l in &e.loin {
        out.push(Etiquette {
            price: l.price,
            texte: format!("{} {} · {}", side_word(l.sell), fp(l.price), dollars(l.usd)),
            couleur: if l.sell { cols[0] } else { cols[1] },
            usd: l.usd,
            sell: l.sell,
            spot: false,
            since_ms: l.since_ms,
            fort: false,
            tag: Some(Marque::Hyperliquid),
            loin: true,
            jumeau: false,
            par: [0.0; PAR_BOOKS],
            comptant: 0.0,
        });
    }
    out
}

/// Where a wall outside the band `lo ..= hi` is placed: just at the edge, a hair further for a further wall, so that
/// several walls brought to one edge keep their price order (lot 3e).
pub fn au_bord(y: f64, lo: f64, hi: f64) -> f64 {
    if y < lo {
        lo - (lo - y).min(1e6) * 1e-6
    } else if y > hi {
        hi + (y - hi).min(1e6) * 1e-6
    } else {
        y
    }
}

/// One item of the column for the room rule ([`tenir`]): its height, a grey word or REMPLI mark (`petit`), its money,
/// the tagged Binance wall, and for a split spot part the index of its perp wall.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Occupant {
    pub y: f64,
    pub h: f64,
    pub petit: bool,
    pub usd: f64,
    pub tag: bool,
    pub lie: Option<usize>,
}

/// Lot 3e c), the room rule: per band (above the last-price line and below it; the whole free band `lo ..= hi` when
/// the line is not in it), when the items do not fit stacked (heights and `GAP`s), drop the grey words and REMPLI
/// marks first, then the walls from the smallest money up; never the tagged Binance wall; a perp wall takes its split
/// spot part with it. Gives which items stay.
pub fn tenir(o: &[Occupant], lo: f64, hi: f64, ligne: Option<f64>) -> Vec<bool> {
    let mut garde = vec![true; o.len()];
    let bandes: Vec<(f64, Vec<usize>)> = match ligne.filter(|y| *y > lo && *y < hi) {
        Some(ly) => {
            let (h, b): (Vec<usize>, Vec<usize>) = (0..o.len()).partition(|&i| o[i].y < ly);
            vec![(ly - 1.0 - GAP - lo, h), (hi - ly - 1.0 - GAP, b)]
        }
        None => vec![(hi - lo, (0..o.len()).collect())],
    };
    for (place, l) in bandes {
        let besoin = |g: &[bool]| {
            let k: Vec<&usize> = l.iter().filter(|&&i| g[i]).collect();
            k.iter().map(|&&i| o[i].h).sum::<f64>() + GAP * k.len().saturating_sub(1) as f64
        };
        while besoin(&garde) > place {
            let libre = |i: usize| garde[i] && !o[i].tag && o[i].lie.is_none();
            let petit = l.iter().copied().rfind(|&i| libre(i) && o[i].petit);
            let mur = || {
                l.iter()
                    .copied()
                    .filter(|&i| libre(i) && !o[i].petit)
                    .min_by(|&a, &b| o[a].usd.total_cmp(&o[b].usd))
            };
            let Some(i) = petit.or_else(mur) else { break };
            garde[i] = false;
            for (j, x) in o.iter().enumerate() {
                if x.lie == Some(i) {
                    garde[j] = false;
                }
            }
        }
    }
    garde
}

/// Width of a pill: square, text, padding and the tag (pixels).
pub fn largeur_pilule(gras: &Text, q: &Etiquette, t: &Tailles, short: bool) -> f64 {
    largeur_texte(gras, &q.texte, q.tag, t, short)
}

/// Width of a pill writing `texte` with the tag `tag` (pixels).
pub fn largeur_texte(gras: &Text, texte: &str, tag: Option<Marque>, t: &Tailles, short: bool) -> f64 {
    let tw = gras.width(texte, t.font) as f64;
    t.pad + t.carre + 8.0 + tw + t.pad + 2.0 + tag.map_or(0.0, |m| marque_largeur(gras, m, short))
}

/// Room the BINANCE tag takes in a pill (gap and spaced letters).
pub fn tag_largeur(gras: &Text, short: bool) -> f64 {
    marque_largeur(gras, Marque::Binance, short)
}

/// Room a tag takes in a pill (gap and spaced letters), the same rule for every tag.
pub fn marque_largeur(gras: &Text, m: Marque, short: bool) -> f64 {
    let (f, gap) = if short { (TAG_FONT_COURT, TAG_GAP_COURT) } else { (TAG_FONT, TAG_GAP) };
    gap + espace_largeur(gras, m.texte(short), f)
}

fn espace_largeur(gras: &Text, s: &str, f: f32) -> f64 {
    let n = s.chars().count();
    s.chars().map(|c| gras.width(&c.to_string(), f) as f64).sum::<f64>() + TAG_ESPACE as f64 * n.saturating_sub(1) as f64
}

/// Spaced letters (the tag), from `x` on the baseline `y`.
fn espace_draw(pm: &mut Pixmap, gras: &Text, s: &str, x: f64, y: f32, f: f32, c: Rgb) {
    let mut x = x;
    for ch in s.chars() {
        let t = ch.to_string();
        gras.draw(pm, &t, (x as f32, y), f, c, Align::Left);
        x += gras.width(&t, f) as f64 + TAG_ESPACE as f64;
    }
}

/// One item of the right-hand column: centre y, height, priority (bigger stays).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Place {
    pub y: f64,
    pub h: f64,
    pub prio: f64,
}

/// A wall's line is never shorter than this share of the plot width.
pub const MIN_LINE: f64 = 0.25;

/// Minimum free space between two items of the column (pixels).
pub const GAP: f64 = 4.0;

/// Most grey words (SPOOF n×) on screen at once, the most recent first.
pub const MAX_WORDS: usize = 6;

/// Where each item of the column is drawn (centre y): as close to its own price as it can, at least `GAP` from the
/// next one, inside `lo..hi`. Nothing is dropped: items too close are spread apart around where they want to be (each
/// group of touching items is centred on the mean of their places), and a short connector joins the line to the pill.
pub fn ranger(items: &[Place], lo: f64, hi: f64) -> Vec<f64> {
    let mut order: Vec<usize> = (0..items.len()).collect();
    order.sort_by(|a, b| items[*a].y.total_cmp(&items[*b].y));
    // groups of items stacked with GAP between them: (first and last index in `order`, top, height)
    let mut groups: Vec<(usize, usize, f64, f64)> = vec![];
    let want_top = |a: usize, b: usize| -> f64 {
        // mean over the group of (wanted centre − offset of the centre inside the group)
        let mut off = 0.0;
        let mut sum = 0.0;
        for &i in &order[a..=b] {
            sum += items[i].y - (off + items[i].h / 2.0);
            off += items[i].h + GAP;
        }
        sum / (b - a + 1) as f64
    };
    let height = |a: usize, b: usize| order[a..=b].iter().map(|&i| items[i].h).sum::<f64>() + GAP * (b - a) as f64;
    for k in 0..order.len() {
        let mut g = (k, k, want_top(k, k), height(k, k));
        while let Some(&p) = groups.last() {
            if p.2 + p.3 + GAP <= g.2 {
                break;
            }
            groups.pop();
            g = (p.0, g.1, want_top(p.0, g.1), height(p.0, g.1));
        }
        groups.push(g);
    }
    // inside the plot: a group pushed past an edge comes back in (and may then touch the next one: stacked again)
    let mut tops: Vec<f64> = groups.iter().map(|g| g.2.max(lo).min((hi - g.3).max(lo))).collect();
    for k in 1..groups.len() {
        tops[k] = tops[k].max(tops[k - 1] + groups[k - 1].3 + GAP);
    }
    // (lot 3e: the forward pass may push the last group past the bottom: back inside, the backward pass then stacks
    // the others above it)
    if let (Some(t), Some(g)) = (tops.last_mut(), groups.last()) {
        *t = t.min((hi - g.3).max(lo));
    }
    for k in (0..groups.len().saturating_sub(1)).rev() {
        tops[k] = tops[k].min(tops[k + 1] - groups[k].3 - GAP).max(lo);
    }
    let mut out = vec![0.0; items.len()];
    for (g, top) in groups.iter().zip(tops) {
        let mut y = top;
        for &i in &order[g.0..=g.1] {
            out[i] = y + items[i].h / 2.0;
            y += items[i].h + GAP;
        }
    }
    out
}

/// The grey words drawn: SPOOF n× only (Jee, 21 Nov: RETIRÉ and TESTÉ are not drawn any more), the `MAX_WORDS`
/// most recent among those `dedans` keeps.
pub fn mots<'a>(words: &'a [Word], dedans: &dyn Fn(&Word) -> bool) -> Vec<&'a Word> {
    let mut l: Vec<&Word> = words.iter().filter(|w| matches!(w.fin, Fin::Spoof(_)) && dedans(w)).collect();
    l.sort_by_key(|w| std::cmp::Reverse(w.at_ms));
    l.truncate(MAX_WORDS);
    l
}

/// The marker of a wall's pill (lot 3d): its colour square when its level is in the free band, else a filled
/// triangle pointing to the level (▲ above the band, under the header; ▼ below it, behind the bottom pills).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Repere {
    Carre,
    Haut,
    Bas,
}

/// Marker of a level at `y` for the free band `lo ..= hi`.
pub fn repere(y: f64, lo: f64, hi: f64) -> Repere {
    if y < lo {
        Repere::Haut
    } else if y > hi {
        Repere::Bas
    } else {
        Repere::Carre
    }
}

/// Draws the marker in the square's box `r`: the rounded square, or the triangle (base 1.2 × the side, height 1.04 ×
/// the side, centred on the box), as in maquette-murs-lointains-v2.
fn repere_draw(pm: &mut Pixmap, r: Rect, m: Repere, c: Rgb) {
    let (a, h) = ((r.w * 0.6) as f32, (r.w * 0.52) as f32);
    match m {
        Repere::Carre => round_fill(pm, r, 2.0, c, 1.0),
        Repere::Haut => crate::shapes::poly_fill(pm, r.cx(), r.cy(), &[(0.0, -h), (a, h), (-a, h)], c, 1.0),
        Repere::Bas => crate::shapes::poly_fill(pm, r.cx(), r.cy(), &[(0.0, h), (a, -h), (-a, -h)], c, 1.0),
    }
}

/// A thin vertical connector at `x` from the line's height `y` to the item's height `y2` (nothing when they match).
fn connecteur(pm: &mut Pixmap, x: f64, y: f64, y2: f64, c: Rgb, a: f32) {
    if (y2 - y).abs() >= 1.0 {
        fill(pm, x, y.min(y2), 1.0, (y2 - y).abs() + 1.0, c, a);
    }
}

/// Sizes of the column: pill height, text size, marker; smaller by the finger (Jee, 25 Nov 2026: the pills « trop
/// gros » on the tablet and the phone): the phone smaller than before, the tablet as the phone was; the mouse (PC)
/// unchanged (`tactile`: the finger's mode, [`crate::actions::tactile`]). The finger's tolerance around a pill stays 24
/// points ([`touche`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tailles {
    pub pill_h: f64,
    /// Corner radius of a pill.
    pub radius: f64,
    /// Outline of a solid wall's pill.
    pub stroke: f64,
    /// Band of the biggest wall of a side.
    pub band: f64,
    pub font: f32,
    pub word: f32,
    pub carre: f64,
    pub pad: f64,
}

pub fn tailles(pw: f64, tactile: bool) -> Tailles {
    if pw < 560.0 {
        // 📱 phone (an even height: the pills' edges on whole points, sharp)
        Tailles {
            pill_h: 18.0,
            radius: 6.0,
            stroke: 1.2,
            band: 4.0,
            font: 9.5,
            word: 8.5,
            carre: 6.0,
            pad: 6.0,
        }
    } else if tactile {
        // 📱 tablet by the finger: the size of the phone before
        Tailles {
            pill_h: 20.0,
            radius: 7.0,
            stroke: 1.3,
            band: 5.0,
            font: 10.5,
            word: 9.0,
            carre: 7.0,
            pad: 7.0,
        }
    } else {
        Tailles {
            pill_h: 26.0,
            radius: 9.0,
            stroke: 1.6,
            band: 7.0,
            font: 13.0,
            word: 11.0,
            carre: 10.0,
            pad: 12.0,
        }
    }
}

/// Height of a depth bar (pixels): fixed, whatever the zoom (mockup v5).
pub const BAR_H: f64 = 3.0;
/// One bar row every this many pixels: a 3 px bar and a 1 px gap (mockup v5); buckets closer than that are summed.
pub const BAR_ROW: f64 = 4.0;
/// Opacity of a bar next to the last price, and at the far end of the range (mockup: a clear histogram).
pub const BAR_NEAR: f32 = 0.45;
pub const BAR_FAR: f32 = 0.25;
/// Reference of a coverage class: this quantile of its non-empty rows in view reaches the full length.
pub const BAR_QUANTILE: f64 = 0.95;
/// A class with fewer non-empty rows than this uses its own maximum as reference.
pub const BAR_CLASS_MIN_ROWS: usize = 12;
/// Shortest bar drawn for a non-empty row (pixels): a small level still shows.
pub const BAR_MIN: f64 = 6.0;

/// One depth bar: y centre, length, above the last price, and its opacity factor (paler where fewer books cover it).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Barre {
    pub y: f64,
    pub w: f64,
    pub above: bool,
    pub pale: f32,
    /// Money of the row, its split by book (`Ecran::noms`) and the prices of its lowest and highest bucket: the
    /// tooltip under the mouse.
    pub usd: f64,
    pub par: [f64; PAR_BOOKS],
    pub prix: (f64, f64),
}

/// The books whose snapshot covers `price` (bit i = book i of `reach`): its coverage class.
fn classe(reach: &[(f64, f64)], price: f64) -> u32 {
    reach
        .iter()
        .enumerate()
        .take(32)
        .filter(|(_, (lo, hi))| *lo <= price && price <= *hi)
        .fold(0, |m, (i, _)| m | 1 << i)
}

/// The depth bars to draw, on a fixed screen grid whatever the zoom: one row every `BAR_ROW` px (a `BAR_H` bar and a
/// 1 px gap). Each row holds the money of the depth buckets inside its price span; a bucket (`depth_step` wide,
/// centred on its price) straddling two rows gives each the share of its own span that falls in it, so zoomed out
/// the buckets are summed and zoomed in a bucket fills the rows of its own span (no hole, no slab). A row no bucket
/// covers stays empty: nothing is drawn where no exchange gives depth. `depth_step` 0: each bucket is a point.
///
/// Length (lot 3c, the v5 images were flat at the cap and fell off a cliff where Binance's snapshot ends): each row
/// belongs to a coverage class (the books whose snapshot covers its price, nested bands around the price); its
/// length is `cap × √(row / reference of its class)`, the reference being the 95th percentile of the class's rows in
/// view (its maximum under 12 rows), clipped to [`BAR_MIN`, cap] (70 px PC, 40 px phone). A class read from fewer
/// books is paler: × (0.55 + 0.45 × books / most books of any class).
pub fn barres_rows(e: &Ecran, pw: f64, ph: f64, y_of: &dyn Fn(f64) -> f64) -> Vec<Barre> {
    use std::collections::{BTreeMap, HashMap};
    let last = if e.last > 0.0 { Some(e.last) } else { None };
    let n_rows = (ph / BAR_ROW).ceil() as i64;
    let half = e.depth_step.max(0.0) / 2.0;
    // (row, above) → (sum $, $ per class, $ per book, lowest and highest bucket price)
    type Rangee = (f64, HashMap<u32, f64>, [f64; PAR_BOOKS], (f64, f64));
    let mut rows: BTreeMap<(i64, bool), Rangee> = BTreeMap::new();
    let mut add = |r: (i64, bool), usd: f64, c: u32, par: Option<&[f64; PAR_BOOKS]>, part: f64, price: f64| {
        let e = rows.entry(r).or_insert_with(|| (0.0, HashMap::new(), [0.0; PAR_BOOKS], (price, price)));
        e.0 += usd;
        *e.1.entry(c).or_insert(0.0) += usd;
        if let Some(par) = par {
            for (a, b) in e.2.iter_mut().zip(par) {
                *a += b * part;
            }
        }
        e.3 = (e.3 .0.min(price), e.3 .1.max(price));
    };
    for (j, &(price, usd, sell)) in e.depth.iter().enumerate() {
        let par = e.detail.get(j);
        if !(usd > 0.0) {
            continue;
        }
        let above = match last {
            Some(l) => price > l,
            None => sell,
        };
        let c = classe(&e.reach, price);
        let (ya, yb) = (y_of(price + half), y_of(price - half));
        let (top, bot) = (ya.min(yb), ya.max(yb));
        if !(bot >= 0.0 && top < ph) {
            continue;
        }
        if bot - top < 1e-9 {
            add(((top / BAR_ROW).floor() as i64, above), usd, c, par, 1.0, price);
            continue;
        }
        let r0 = ((top / BAR_ROW).floor() as i64).max(0);
        let r1 = ((bot / BAR_ROW).ceil() as i64 - 1).min(n_rows - 1);
        for r in r0..=r1 {
            let part = (bot.min((r + 1) as f64 * BAR_ROW) - top.max(r as f64 * BAR_ROW)) / (bot - top);
            if part > 1e-6 {
                add((r, above), usd * part, c, par, part, price);
            }
        }
    }
    // each row's class: the one that brings most of its money
    let rows: Vec<((i64, bool), f64, u32, [f64; PAR_BOOKS], (f64, f64))> = rows
        .into_iter()
        .filter(|(_, (usd, _, _, _))| *usd > 0.0)
        .map(|(k, (usd, cl, par, prix))| {
            let c = cl.into_iter().max_by(|a, b| a.1.total_cmp(&b.1).then(b.0.cmp(&a.0))).map_or(0, |x| x.0);
            (k, usd, c, par, prix)
        })
        .collect();
    if rows.is_empty() {
        return vec![];
    }
    let mut par_classe: HashMap<u32, Vec<f64>> = HashMap::new();
    for (_, usd, c, _, _) in &rows {
        par_classe.entry(*c).or_default().push(*usd);
    }
    let reference: HashMap<u32, f64> = par_classe
        .into_iter()
        .map(|(c, mut v)| {
            v.sort_by(f64::total_cmp);
            let r = if v.len() < BAR_CLASS_MIN_ROWS {
                v[v.len() - 1]
            } else {
                v[((v.len() - 1) as f64 * BAR_QUANTILE).round() as usize]
            };
            (c, r)
        })
        .collect();
    // (no coverage known: every row at full opacity)
    let most = rows.iter().map(|r| r.2.count_ones()).max().unwrap_or(0);
    let cap = if pw < 560.0 { 40.0 } else { 70.0 };
    rows.into_iter()
        // each row on its own 4 px slot (3 px bar, 1 px gap): two rows never touch
        .map(|((row, above), usd, c, par, prix)| {
            let r = reference.get(&c).copied().unwrap_or(usd).max(f64::MIN_POSITIVE);
            Barre {
                y: row as f64 * BAR_ROW + BAR_H / 2.0,
                w: (cap * (usd / r).sqrt()).clamp(BAR_MIN, cap),
                above,
                pale: if most == 0 { 1.0 } else { 0.55 + 0.45 * c.count_ones() as f32 / most as f32 },
                usd,
                par,
                prix,
            }
        })
        .collect()
}

/// Thin bars of the summed book against the price scale (setting « bars »), drawn behind the candles.
/// Gives the rows drawn (the tooltip under the mouse); none when the setting is off.
pub fn barres(pm: &mut Pixmap, e: &Ecran, p: &Inputs, pw: f64, ph: f64, y_of: &dyn Fn(f64) -> f64) -> Vec<Barre> {
    BARRES.with(|b| *b.borrow_mut() = (vec![], pw));
    if !p.on("bars") || e.depth.is_empty() {
        return vec![];
    }
    let cols = couleurs(p);
    let rows = barres_rows(e, pw, ph, y_of);
    BARRES.with(|b| *b.borrow_mut() = (rows.iter().map(|r| (r.y, r.w)).collect(), pw));
    // opacity: strong next to the last price, fading towards the far end of the range
    let ly = if e.last > 0.0 { y_of(e.last) } else { ph / 2.0 };
    let far = rows.iter().map(|r| (r.y - ly).abs()).fold(1.0, f64::max);
    for b in &rows {
        let a = (BAR_NEAR - (BAR_NEAR - BAR_FAR) * ((b.y - ly).abs() / far) as f32) * b.pale;
        fill(
            pm,
            pw - b.w,
            (b.y - BAR_H / 2.0).round(),
            b.w,
            BAR_H,
            palette::on_chart(couleur_barre(&cols, e.perp, b.above)),
            a,
        );
    }
    rows
}

/// Colour of a depth bar: perp or spot, above (sell) or below (buy) the last price.
fn couleur_barre(cols: &[Rgb; 4], perp: bool, above: bool) -> Rgb {
    match (perp, above) {
        (true, true) => cols[0],
        (true, false) => cols[1],
        (false, true) => cols[2],
        (false, false) => cols[3],
    }
}

/// A wall pill as drawn (its box), for the tooltip under the mouse.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ZonePilule {
    pub r: Rect,
    pub price: f64,
    pub usd: f64,
    pub sell: bool,
    pub spot: bool,
    /// Its money per book (adds up to `usd`), and a merged spot part's amount.
    pub par: [f64; PAR_BOOKS],
    pub comptant: f64,
    /// A far wall (lot loin-1): its money is Hyperliquid's.
    pub loin: bool,
    /// The tag of its full pill (BINANCE, HYPERLIQUID), said in the tooltip when the pill is shortened.
    pub tag: Option<Marque>,
}

/// 🫧 What the mouse layer keeps to answer the mouse over Argos Murs (lot 3e R2, Jee 22 Nov: « mettre la souris dessus
/// et qu'on voie les ordres »): the depth bars drawn, the pills drawn, the fine buckets split by exchange.
#[derive(Debug, Clone, PartialEq)]
pub struct Survol {
    pub rows: Vec<Barre>,
    pub pilules: Vec<ZonePilule>,
    pub depth: Vec<(f64, f64, bool)>,
    pub detail: Vec<[f64; PAR_BOOKS]>,
    pub noms: Vec<String>,
    pub noms_perp: Vec<String>,
    pub noms_spot: Vec<String>,
    /// The fine buckets are perp books (else spot).
    pub perp: bool,
    pub coin: String,
    pub step: f64,
    pub cols: [Rgb; 4],
    pub cap: f64,
    /// Bottom of the price pane (an indicator pane below it: nothing of Argos Murs there, tooltip included).
    pub bas: f64,
    /// 🫧 The bubble of pressure (its place and its tooltip), F1g.
    pub pression: Option<(Rect, String)>,
}

impl Survol {
    pub fn new(e: &Ecran, p: &Inputs, rows: Vec<Barre>, pilules: Vec<ZonePilule>, pw: f64) -> Self {
        Self {
            rows,
            pilules,
            depth: e.depth.clone(),
            detail: e.detail.clone(),
            noms: e.noms.clone(),
            noms_perp: e.noms_perp.clone(),
            noms_spot: e.noms_spot.clone(),
            perp: e.perp,
            coin: e.coin.clone(),
            step: e.step,
            cols: couleurs(p),
            cap: if pw < 560.0 { 40.0 } else { 70.0 },
            bas: f64::INFINITY,
            pression: BULLE_PRESSION
                .with(|b| b.get())
                .zip(e.pression)
                .filter(|_| p.on("pression"))
                .map(|(r, s)| (r, pression_texte(s, e.range))),
        }
    }
}

/// What the mouse points at: a wall pill (first), or a depth bar.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Vise<'a> {
    Pilule(&'a ZonePilule),
    Barre(&'a Barre),
}

/// The pill under the mouse, else (over the bars column, `x >= pw - cap - 4`) the bar of the 4 px row under it, or the
/// nearest row with a bar within 6 px.
pub fn vise(s: &Survol, mx: f64, my: f64, pw: f64) -> Option<Vise<'_>> {
    if let Some(z) = s.pilules.iter().find(|z| touche(&z.r, mx, my)) {
        return Some(Vise::Pilule(z));
    }
    if mx < pw - s.cap - 4.0 || mx > pw {
        return None;
    }
    let ligne = (my / BAR_ROW).floor();
    s.rows
        .iter()
        .filter(|b| (b.y / BAR_ROW).floor() == ligne || (b.y - my).abs() <= 6.0)
        .min_by(|a, b| (a.y - my).abs().total_cmp(&(b.y - my).abs()))
        .map(Vise::Barre)
}

/// The tooltip's lines: side and price (a span when the row holds several buckets), money and coins, then each
/// exchange holding money there, biggest first, at most six.
pub fn infobulle(s: &Survol, v: Vise) -> Vec<String> {
    let dec = dec_of(s.step.max(0.0));
    let fp = |x: f64| crate::viewport::format_price(x, dec);
    let (sell, (lo, hi), usd, par): (bool, (f64, f64), f64, Option<[f64; PAR_BOOKS]>) = match v {
        Vise::Barre(b) => (b.above, b.prix, b.usd, Some(b.par)),
        Vise::Pilule(z) => (z.sell, (z.price, z.price), z.usd, Some(z.par)),
    };
    let prix = if (hi - lo).abs() < 1e-9 {
        fp(lo)
    } else {
        format!("{} – {}", fp(lo), fp(hi))
    };
    let mid = (lo + hi) / 2.0;
    // a pill: what its full text says, perp or spot and its tag (a short pill writes only its price)
    let quoi = match v {
        Vise::Pilule(z) => {
            let genre = if z.spot { " · COMPTANT" } else { " · PERP" };
            format!("{genre}{}", z.tag.map_or(String::new(), |m| format!(" · {}", m.texte(false))))
        }
        Vise::Barre(_) => String::new(),
    };
    let mut l = vec![
        format!("{} {prix}{quoi}", side_word(sell)),
        format!("{} · {} {}", dollars(usd), coins(if mid > 0.0 { usd / mid } else { 0.0 }), s.coin),
    ];
    // a pill's split: its tracker's books (perp or spot), recorded with its amount; a bar row's: the fine profile's
    let noms = match v {
        Vise::Pilule(z) if z.spot => &s.noms_spot,
        Vise::Pilule(_) => &s.noms_perp,
        Vise::Barre(_) => &s.noms,
    };
    if let Some(par) = par {
        let mut liste: Vec<(&String, f64)> = noms.iter().zip(par).filter(|(_, u)| *u >= 1.0).collect();
        liste.sort_by(|a, b| b.1.total_cmp(&a.1));
        l.extend(liste.into_iter().take(6).map(|(n, u)| format!("{n} {}", dollars(u))));
    }
    // a far wall: all Hyperliquid's
    if let Vise::Pilule(z) = v {
        if z.loin {
            l.push(format!("Hyperliquid {}", dollars(z.usd)));
        }
    }
    // a merged pill: its spot part last
    if let Vise::Pilule(z) = v {
        if z.comptant > 0.0 {
            l.push(format!("Comptant {}", dollars(z.comptant)));
        }
    }
    l
}

/// The pill under the mouse (its box drawn), if any.
pub fn pilule_sous(s: &Survol, mx: f64, my: f64) -> Option<&ZonePilule> {
    s.pilules.iter().find(|z| touche(&z.r, mx, my))
}

/// The pixel box of `r` (points) in the pixmap (whole screen pixels at the drawing scale, inside it).
fn zone_px(pm: &Pixmap, r: Rect) -> Option<tiny_skia::IntRect> {
    let d = crate::px::dev;
    let (x0, y0) = (d(r.x).floor().max(0.0), d(r.y).floor().max(0.0));
    let x1 = d(r.x + r.w).ceil().min(pm.width() as f64);
    let y1 = d(r.y + r.h).ceil().min(pm.height() as f64);
    tiny_skia::IntRect::from_ltrb(x0 as i32, y0 as i32, x1 as i32, y1 as i32)
}

/// A copy of the pixels of `r` (the hovered pill as drawn, before the crosshair lines).
pub fn copie_zone(pm: &Pixmap, r: Rect) -> Option<Pixmap> {
    pm.clone_rect(zone_px(pm, r)?)
}

/// Puts back the pixels of `r` taken by [`copie_zone`], exactly (the crosshair lines no longer cross the pill).
pub fn remet_zone(pm: &mut Pixmap, r: Rect, c: &Pixmap) {
    let Some(z) = zone_px(pm, r) else { return };
    let paint = tiny_skia::PixmapPaint {
        blend_mode: tiny_skia::BlendMode::Source,
        ..Default::default()
    };
    pm.draw_pixmap(z.x(), z.y(), c.as_ref(), &paint, tiny_skia::Transform::identity(), None);
}

/// 👆 The round « ⋯ » button of the finger's crosshair on a phone (chart coordinates, plot `pw` × `ph`, crosshair at
/// `y`): the app draws it there, above the chart; the tooltip keeps clear of it.
pub fn bouton_croix(pw: f64, ph: f64, y: f64) -> Rect {
    let cy = y.clamp(20.0, (ph - 20.0).max(20.0));
    Rect::new(pw - 44.0, cy - 20.0, 40.0, 40.0)
}

fn se_touchent(a: Rect, b: Rect) -> bool {
    a.x < b.x + b.w && a.x + a.w > b.x && a.y < b.y + b.h && a.y + a.h > b.y
}

/// The tooltip's box never on a wall pill (Jee, 22 Nov): when it meets pills, it goes at the left of the leftmost of
/// them (same 8 px gap, same height), again if it then meets others; when that would leave the plot, it stays where
/// it was.
pub fn loin_des_pilules(r: Rect, pilules: &[Rect], pw: f64, ph: f64) -> Rect {
    let mut c = r;
    for _ in 0..pilules.len() {
        let gauche = pilules.iter().filter(|p| se_touchent(c, **p)).map(|p| p.x).fold(f64::INFINITY, f64::min);
        if !gauche.is_finite() {
            return c;
        }
        if gauche - 8.0 - c.w < 4.0 {
            return r;
        }
        c = cadre_infobulle(c.w, c.h, gauche, c.cy(), pw, ph);
    }
    r
}

/// The tooltip's box moved above (else below) the « ⋯ » button `b` when it meets it, still inside the plot.
pub fn loin_du_bouton(r: Rect, b: Rect, ph: f64) -> Rect {
    let touche = r.x < b.x + b.w && r.x + r.w > b.x && r.y < b.y + b.h && r.y + r.h > b.y;
    if !touche {
        return r;
    }
    let dessus = b.y - 6.0 - r.h;
    let y = if dessus >= 4.0 {
        dessus
    } else {
        (b.y + b.h + 6.0).min((ph - r.h - 4.0).max(4.0))
    };
    Rect::new(r.x, y, r.w, r.h)
}

/// Box of the tooltip: `w` × `h` at the left of `gauche` (8 px gap), centred on `cy`, kept inside the plot.
pub fn cadre_infobulle(w: f64, h: f64, gauche: f64, cy: f64, pw: f64, ph: f64) -> Rect {
    let x = (gauche - 8.0 - w).clamp(4.0, (pw - w - 4.0).max(4.0));
    let y = (cy - h / 2.0).clamp(4.0, (ph - h - 4.0).max(4.0));
    Rect::new(x, y, w, h)
}

/// 🫧 The tooltip of what the mouse points at (mouse layer): the bar under the mouse at full opacity, then a glass box
/// at its left (or the pill's): side and price in the side colour, money and coins in white, one line per exchange.
pub fn infobulle_draw(pm: &mut Pixmap, text: &Text, s: &Survol, mx: f64, my: f64, pw: f64, ph: f64) {
    // only in the price pane (Argos Vague's pane below it: nothing there)
    let ph = ph.min(s.bas);
    if my > ph {
        return;
    }
    // 🫧 the bubble of pressure: one line above it
    if let Some((r, t)) = s.pression.as_ref().filter(|(r, _)| r.contains(mx, my)) {
        let f = if pw < 560.0 { 10.0 } else { 12.0 };
        let w = (text.width(t, f) as f64 + 20.0).min(pw - 16.0);
        let b = Rect::new((r.cx() - w / 2.0).clamp(8.0, (pw - 8.0 - w).max(8.0)), r.y - 8.0 - 28.0, w, 28.0);
        round_fill(pm, b, 9.0, PILL_BG, 0.94);
        round_stroke(pm, b, 9.0, palette::edge(), 1.0);
        let t = text.fit(t, f, w - 20.0);
        text.draw(
            pm,
            &t,
            ((b.x + 10.0) as f32, (b.y + 14.0 + f as f64 * 0.36) as f32),
            f,
            palette::WHITE,
            Align::Left,
        );
        return;
    }
    let Some(v) = vise(s, mx, my, pw) else { return };
    let lignes = infobulle(s, v);
    let sell = match v {
        Vise::Barre(b) => b.above,
        Vise::Pilule(z) => z.sell,
    };
    let (gauche, cy) = match v {
        Vise::Barre(b) => {
            let c = couleur_barre(&s.cols, s.perp, b.above);
            fill(pm, pw - b.w, (b.y - BAR_H / 2.0).round(), b.w, BAR_H, palette::on_chart(c), 1.0);
            (pw - b.w, b.y)
        }
        Vise::Pilule(z) => (z.r.x, z.r.cy()),
    };
    let small = pw < 560.0;
    let (f, lh): (f32, f64) = if small { (10.0, 14.0) } else { (12.0, 17.0) };
    let (px, py) = (10.0, 8.0);
    let gras = Text::graisse(crate::text::Graisse::Bold);
    let largeur = |k: usize, l: &str| if k < 2 { gras.width(l, f) } else { text.width(l, f) } as f64;
    let w = lignes.iter().enumerate().map(|(k, l)| largeur(k, l)).fold(0.0, f64::max) + 2.0 * px;
    let h = lignes.len() as f64 * lh + 2.0 * py;
    let r = if crate::actions::tactile() {
        // F1j (by the finger): under the tapped pill / bar, against the price scale, never on a candle
        let cible = match v {
            Vise::Barre(b) => Rect::new(pw - b.w, b.y - BAR_H / 2.0, b.w, BAR_H),
            Vise::Pilule(z) => z.r,
        };
        cadre_toucher(w, h, cible, pw, ph)
    } else {
        let r = cadre_infobulle(w, h, gauche, cy, pw, ph);
        let cadres: Vec<Rect> = s.pilules.iter().map(|z| z.r).collect();
        loin_des_pilules(r, &cadres, pw, ph)
    };
    let cote = palette::on_chart(if sell { s.cols[0] } else { s.cols[1] });
    round_fill(pm, r, 9.0, PILL_BG, 0.94);
    crate::shapes::round_stroke_alpha(pm, r, 9.0, cote, 1.0, 0.55);
    for (k, l) in lignes.iter().enumerate() {
        let base = (r.y + py + lh * k as f64 + lh * 0.72) as f32;
        let pos = ((r.x + px) as f32, base);
        match k {
            0 => gras.draw(pm, l, pos, f, cote, Align::Left),
            1 => gras.draw(pm, l, pos, f, Rgb(0xF2, 0xF3, 0xF5), Align::Left),
            _ => text.draw(pm, l, pos, f, Rgb(0xC8, 0xCC, 0xD6), Align::Left),
        }
    }
}

/// Pills, grey words, sweeps and the bottom pills, over the candles (pills and words in Inter Bold, like the mockup).
/// `y_of`: price → y; `x_of`: time (ms) → x (`None`: before the first bar shown, the line starts at the left edge).
#[allow(clippy::too_many_arguments)]
pub fn draw(
    pm: &mut Pixmap,
    text: &Text,
    e: &Ecran,
    p: &Inputs,
    pw: f64,
    ph: f64,
    y_of: &dyn Fn(f64) -> f64,
    x_of: &dyn Fn(i64) -> Option<f64>,
) -> Vec<ZonePilule> {
    let tactile = crate::actions::tactile();
    let t = tailles(pw, tactile);
    let cols = couleurs(p);
    let right = pw - 8.0;
    let gras = Text::graisse(crate::text::Graisse::Bold);
    let short = pw < 560.0;
    // the bottom band belongs to the summary and the colour key: no pill nor word is drawn over them
    let bas_y = if p.on("summary") { ph - bas_hauteur(pw) - t.pill_h / 2.0 - GAP } else { ph };
    // a pill with its spot part merged must fit in the free band (lot 3c d); otherwise two pills
    let tient = |texte: &str| {
        let q = Etiquette {
            price: 0.0,
            texte: texte.to_string(),
            couleur: PILL_BG,
            usd: 0.0,
            sell: false,
            spot: false,
            since_ms: 0,
            fort: true,
            tag: None,
            loin: false,
            jumeau: false,
            par: [0.0; PAR_BOOKS],
            comptant: 0.0,
        };
        // (PC: a merged pill may reach left of the band, over no candle, as in mockup v6; phone: never)
        !short || e.marge <= 0.0 || largeur_pilule(&gras, &q, &t, short) + 8.0 <= e.marge
    };
    // every wall keeps its pill at any zoom (lot 3e: no spacing rule); a wall outside the plot too, at the edge of the
    // band with its arrow and no line (lot 3d)
    let toutes: Vec<Etiquette> = etiquettes_dans(e, p, short, cols, &tient);
    let words: Vec<&Word> = if p.on("ghosts") {
        mots(&e.vue.words, &|w| (0.0..ph).contains(&y_of(w.price)))
    } else {
        vec![]
    };
    // REMPLI marks (walls really filled by trades), small, in the column too: never over a pill
    let remplis: Vec<(Rempli, f32)> = remplis_visibles(&e.vue.remplis, e.now_ms, e.bar_ms)
        .into_iter()
        .filter(|(r, _)| (0.0..ph).contains(&y_of(r.price)))
        .collect();
    let (lo, hi) = (haut_colonne(e), bas_y);
    let ligne = (e.last > 0.0).then(|| y_of(e.last));
    // the room rule: what does not fit in its band leaves (grey words and REMPLI first, then the smallest walls)
    let near = |a: f64, b: f64| (a - b).abs() < e.step.max(f64::EPSILON) * 0.5;
    let petit = t.word as f64 + 3.0;
    // (the app's rects over the pill column count as occupied: never dropped, never moved)
    let garder: Vec<Rect> = GARDER.with(|g| g.borrow().clone());
    let colonne = toutes.iter().map(|q| largeur_pilule(&gras, q, &t, short)).fold(0.0, f64::max);
    let occupe: Vec<Occupant> = garder
        .iter()
        .filter(|g| g.x < right && g.x + g.w > right - colonne)
        .map(|g| Occupant {
            y: g.y + g.h / 2.0,
            h: g.h,
            petit: false,
            usd: f64::INFINITY,
            tag: true,
            lie: None,
        })
        .collect();
    let occ: Vec<Occupant> = toutes
        .iter()
        .map(|q| Occupant {
            y: y_of(q.price),
            h: t.pill_h,
            petit: false,
            usd: q.usd,
            // (never dropped: the BINANCE-tagged wall only; a far wall is treated like the near ones)
            tag: q.tag == Some(Marque::Binance),
            lie: q
                .jumeau
                .then(|| toutes.iter().position(|k| !k.spot && k.sell == q.sell && near(k.price, q.price)))
                .flatten(),
        })
        .chain(
            words
                .iter()
                .map(|w| (w.price, 0.0))
                .chain(remplis.iter().map(|(r, _)| (r.price, 0.0)))
                .map(|(px, usd)| Occupant {
                    y: y_of(px),
                    h: petit,
                    petit: true,
                    usd,
                    tag: false,
                    lie: None,
                }),
        )
        .chain(occupe)
        .collect();
    let garde = tenir(&occ, lo, hi, ligne);
    let (gp, rest) = garde.split_at(toutes.len());
    let (gw, gr) = rest.split_at(words.len());
    let pills: Vec<Etiquette> = toutes.iter().zip(gp).filter(|(_, k)| **k).map(|(q, _)| q.clone()).collect();
    let words: Vec<&Word> = words.into_iter().zip(gw).filter(|(_, k)| **k).map(|(w, _)| w).collect();
    let remplis: Vec<(Rempli, f32)> = remplis.into_iter().zip(gr).filter(|(_, k)| **k).map(|(r, _)| r).collect();
    // nothing to draw yet: say so in a small grey pill against the price column
    if pills.is_empty() {
        if let Some(m) = &e.etat {
            etat(pm, text, m, e, pw, ph, y_of);
        }
    }
    // (a wall outside the band is placed as if at its edge: it does not drag its neighbours out with it)
    let mut items: Vec<Place> = pills
        .iter()
        .map(|q| Place {
            y: au_bord(y_of(q.price), lo, hi),
            h: t.pill_h,
            prio: 1e15 + q.usd,
        })
        .collect();
    items.extend(words.iter().map(|w| Place {
        y: y_of(w.price),
        h: petit,
        prio: w.at_ms as f64 / 1e3,
    }));
    items.extend(remplis.iter().map(|(r, _)| Place {
        y: y_of(r.price),
        h: petit,
        prio: 0.0,
    }));
    let placed = ranger_autour(&items, lo, hi, ligne);
    // a level outside the free band (under the header, behind the bottom pills): no line, no connector there (lot 3d)
    let dedans = |y: f64| repere(y, lo, hi) == Repere::Carre;
    let (at_pills, rest) = placed.split_at(pills.len());
    let (at_words, at_remplis) = rest.split_at(words.len());
    for ((r, a), &yr) in remplis.iter().zip(at_remplis) {
        let (y, yr) = (y_of(r.price).round(), yr.round());
        let mot = format!("REMPLI {}", dollars(r.usd));
        let c = palette::on_chart(match (r.spot, r.sell) {
            (false, true) => cols[0],
            (false, false) => cols[1],
            (true, true) => cols[2],
            (true, false) => cols[3],
        });
        let tw = gras.width(&mot, t.word) as f64;
        let end = right - tw - 8.0;
        // a short line at the wall's price, then the word
        if dedans(y) {
            fill(pm, (end - 40.0).max(0.0), y - 0.5, 40.0, 1.0, c, *a);
            connecteur(pm, end, y, yr, c, *a);
        }
        gras.draw(
            pm,
            &mot,
            (right as f32, (yr + t.word as f64 * 0.36) as f32),
            t.word,
            attenue(c, *a + 0.15),
            Align::Right,
        );
    }
    // grey words first (behind): a dashed grey line at the price and the word at the right (moved up or down when a
    // pill is there, with a short grey connector)
    for (w, &yw) in words.iter().zip(at_words) {
        let y = y_of(w.price).round();
        let yw = yw.round();
        let mot = w.fin.mot();
        let tw = gras.width(&mot, t.word) as f64;
        let end = right - tw - 10.0;
        if dedans(y) {
            let x0 = x_of(w.since_ms).unwrap_or(0.0).min(end - pw * MIN_LINE).clamp(0.0, end);
            let mut x = x0;
            while x < end {
                let l = 5.0f64.min(end - x);
                fill(pm, x, y, l, 1.0, WORD, 0.45);
                x += 9.0;
            }
            connecteur(pm, end, y, yw, WORD, 0.45);
        }
        gras.draw(pm, &mot, (right as f32, (yw + t.word as f64 * 0.36) as f32), t.word, WORD, Align::Right);
    }
    // the walls: line (thicker for a bigger wall) from where it appeared, then the pill against the price scale;
    // every line first, then every pill on an opaque backdrop: nothing shows through a pill's text (lot 3c b)
    // (a far wall is never « the biggest wall of the side »: the thick band stays a near-wall mark)
    let biggest = |sell: bool| pills.iter().filter(|q| q.sell == sell && !q.loin).map(|q| q.usd).fold(0.0, f64::max);
    let (big_sell, big_buy) = (biggest(true), biggest(false));
    // (lot pilules-1) each pill as long as it can be without touching a candle (wicks included), an indicator's or a
    // script's table (`indicators::tableaux`) or the app's rects (`Chart::garder`): the full text, then shorter, down
    // to a dot on the edge of the price scale; by the finger, the price alone (as the scale writes it) or the dot
    let mut obstacles: Vec<Rect> = BOUGIES.with(|b| b.borrow().clone());
    obstacles.extend(crate::indicators::tableaux());
    obstacles.extend(garder.iter().copied());
    let d = point_d(&t);
    let mut pleines: Vec<Rect> = vec![];
    let choix: Vec<(Forme, String, Option<Marque>, Rect)> = pills
        .iter()
        .zip(at_pills)
        .map(|(q, &yp)| {
            let yp = yp.round();
            let essai = |f: Forme| {
                let (s, m) = forme_texte(q, f);
                let r = if f == Forme::Point {
                    Rect::new(pw - d - 1.0, yp - d / 2.0, d, d)
                } else {
                    let w = largeur_texte(&gras, &s, m, &t, short);
                    Rect::new(right - w, yp - t.pill_h / 2.0, w, t.pill_h)
                };
                (f, s, m, r)
            };
            pleines.push(essai(formes(tactile)[0]).3);
            formes(tactile)
                .iter()
                .map(|&f| essai(f))
                .find(|c| c.0 == Forme::Point || !gene(&c.3, &obstacles))
                .unwrap_or_else(|| essai(Forme::Point))
        })
        .collect();
    PLEINES.with(|p| *p.borrow_mut() = pleines);
    // (Z3: even a dot under the app's rects — the floating toolbar, an open list — is hidden by them: that wall waits
    // there, the BINANCE-tagged one excepted, as before)
    let garde: Vec<usize> = (0..pills.len())
        .filter(|&i| !(choix[i].0 == Forme::Point && gene(&choix[i].3, &garder) && pills[i].tag != Some(Marque::Binance)))
        .collect();
    let (pills, at_pills, choix): (Vec<Etiquette>, Vec<f64>, Vec<_>) = (
        garde.iter().map(|&i| pills[i].clone()).collect(),
        garde.iter().map(|&i| at_pills[i]).collect(),
        garde.iter().map(|&i| choix[i].clone()).collect(),
    );
    let rects: Vec<Rect> = choix.iter().map(|c| c.3).collect();
    let at_pills = &at_pills[..];
    for ((q, &yp), r) in pills.iter().zip(at_pills).zip(&rects) {
        let y = y_of(q.price).round();
        let yp = yp.round();
        let c = palette::on_chart(q.couleur);
        if !dedans(y_of(q.price)) {
            continue;
        }
        // the line starts where the wall was first seen, and is never shorter than a quarter of the plot (a new
        // wall is visible at once, as in the mockup); a split spot part has no line of its own (its perp wall's)
        let end = r.x - 4.0;
        if !q.jumeau {
            let x0 = x_of(q.since_ms).unwrap_or(0.0).min(end - pw * MIN_LINE).clamp(0.0, end);
            let big = !q.loin && q.usd >= if q.sell { big_sell } else { big_buy };
            let band = if big { t.band } else { 3.0 };
            fill(pm, x0, y - band / 2.0, end - x0, band, c, if big { 0.3 } else { 0.2 });
            fill(pm, x0, y - 0.75, end - x0, 1.5, c, if q.fort { 0.95 } else { 0.7 });
        }
        // pill moved off its price (another one was there): a short thin connector from the line's end to the pill
        connecteur(pm, end, y, yp, c, 0.8);
        if (yp - y).abs() >= 1.0 {
            fill(pm, end, yp - 0.5, 4.0, 1.0, c, 0.8);
        }
    }
    for ((q, &yp), (forme, texte, tag, r)) in pills.iter().zip(at_pills).zip(&choix) {
        let (r, yp) = (*r, yp.round());
        let c = palette::on_chart(q.couleur);
        if *forme == Forme::Point {
            // (a dark ring first: the dot reads on a candle and on the scale's edge)
            round_fill(pm, Rect::new(r.x - 1.5, r.y - 1.5, r.w + 3.0, r.h + 3.0), r.w / 2.0 + 1.5, PILL_BG, 1.0);
            round_fill(pm, r, r.w / 2.0, c, 1.0);
            continue;
        }
        round_fill(pm, r, t.radius, PILL_BG, 1.0);
        // a wall not solid yet (few exchanges, or just born): thinner, slightly dimmer outline
        let (sw, sa) = if q.fort { (t.stroke as f32, 1.0) } else { (1.0, 0.6) };
        crate::shapes::round_stroke_alpha(pm, r, t.radius, c, sw, sa);
        let boite = Rect::new(r.x + t.pad, yp - t.carre / 2.0, t.carre, t.carre);
        repere_draw(pm, boite, repere(y_of(q.price), lo, hi), c);
        let base = (yp + t.font as f64 * 0.36) as f32;
        gras.draw(pm, texte, ((r.x + t.pad + t.carre + 8.0) as f32, base), t.font, c, Align::Left);
        if let Some(m) = *tag {
            let (s, f) = (m.texte(short), if short { TAG_FONT_COURT } else { TAG_FONT });
            let x = r.x + r.w - t.pad - 2.0 - espace_largeur(&gras, s, f);
            espace_draw(pm, &gras, s, x, (yp + f as f64 * 0.36) as f32, f, TAG_COULEUR);
        }
    }
    // ⚡ SWEEP on the wick (option, off by default)
    if p.on("sweep") {
        for &(bar_t, price, sell) in &e.sweeps {
            let (Some(x), y) = (x_of(bar_t * 1000), y_of(price)) else { continue };
            let s = "⚡ SWEEP";
            let tw = text.width(s, 10.0) as f64 + 12.0;
            let yy = if sell { y - 16.0 } else { y + 16.0 };
            let r = Rect::new(x - tw / 2.0, yy - 9.0, tw, 18.0);
            round_fill(pm, r, 9.0, PILL_BG, 0.92);
            round_stroke(pm, r, 9.0, Rgb(0xFF, 0xD6, 0x00), 1.2);
            text.draw(pm, s, (x as f32, (yy + 3.5) as f32), 10.0, Rgb(0xFF, 0xD6, 0x00), Align::Center);
        }
    }
    // 3. bottom: summary pill (left) and colour key (right)
    let mut dessinees = rects.clone();
    if p.on("summary") && !SANS_BAS.with(|s| s.get()) {
        dessinees.extend(bas(pm, text, e, pw, ph, cols, p.on("pression")));
    } else {
        BULLE_PRESSION.with(|b| b.set(None));
    }
    DESSINEES.with(|d| *d.borrow_mut() = dessinees);
    // F1i: the price of each wall in the band on the price scale, in its colour (a perp wall and its spot part at the
    // same price, merged or split: one tag in two colours)
    let mut axe: Vec<EtiquetteAxe> = vec![];
    for (q, &yp) in pills.iter().zip(at_pills) {
        let c = palette::on_chart(q.couleur);
        // (a wall off screen: its price on the scale too, « ▲ 65 000 », on the row of its pill — Jee, 20:37)
        if !dedans(y_of(q.price)) {
            // (F1m, cell v1: « ▲ 61400 » red and « ▲ 61400 » orange: a perp wall and a spot wall at the same price
            // off screen give one tag in two colours, on the row of the first one's pill)
            let (ecrit, haut) = (prix_axe(q.price, e.step), y_of(q.price) < lo);
            if let Some(a) = axe.iter_mut().find(|a| a.bord.is_some_and(|(_, h)| h == haut) && a.texte == ecrit) {
                if !a.couleurs.contains(&c) {
                    if q.spot {
                        a.couleurs.push(c);
                    } else {
                        a.couleurs.insert(0, c);
                    }
                }
                a.usd = a.usd.max(q.usd);
                continue;
            }
            let mut couleurs = vec![c];
            if q.comptant > 0.0 {
                let s = palette::on_chart(if q.sell { cols[2] } else { cols[3] });
                if s != c {
                    couleurs.push(s);
                }
            }
            axe.push(EtiquetteAxe {
                price: q.price,
                texte: ecrit,
                couleurs,
                usd: q.usd,
                bord: Some((yp.round(), haut)),
            });
            continue;
        }
        // (F1m: the same price, or the same price as written on the scale)
        let ecrit = prix_axe(q.price, e.step);
        let jumeau = |a: &EtiquetteAxe| a.bord.is_none() && (near(a.price, q.price) || a.texte == ecrit);
        match axe.iter_mut().find(|a| jumeau(a)) {
            Some(a) => {
                if !a.couleurs.contains(&c) {
                    if q.spot {
                        a.couleurs.push(c);
                    } else {
                        a.couleurs.insert(0, c);
                    }
                }
                a.usd = a.usd.max(q.usd);
            }
            None => {
                let mut couleurs = vec![c];
                if q.comptant > 0.0 {
                    couleurs.push(palette::on_chart(if q.sell { cols[2] } else { cols[3] }));
                }
                axe.push(EtiquetteAxe {
                    price: q.price,
                    texte: prix_axe(q.price, e.step),
                    couleurs,
                    usd: q.usd,
                    bord: None,
                });
            }
        }
    }
    AXE_MURS.with(|a| *a.borrow_mut() = axe);
    PILULES.with(|p| {
        *p.borrow_mut() = choix
            .iter()
            .map(|(f, s, _, r)| (*r, if *f == Forme::Point { String::new() } else { s.clone() }))
            .collect()
    });
    pills
        .iter()
        .zip(&rects)
        .map(|(q, &r)| ZonePilule {
            r,
            price: q.price,
            usd: q.usd,
            sell: q.sell,
            spot: q.spot,
            par: q.par,
            comptant: q.comptant,
            loin: q.loin,
            tag: q.tag,
        })
        .collect()
}

/// Lot 3c b): like [`ranger`], the last-price line (`ligne`, y) being an obstacle: the items above it stay above it,
/// the ones below stay below, never on it.
pub fn ranger_autour(items: &[Place], lo: f64, hi: f64, ligne: Option<f64>) -> Vec<f64> {
    let Some(ly) = ligne.filter(|y| *y > lo && *y < hi) else {
        return ranger(items, lo, hi);
    };
    let (haut, bas): (Vec<usize>, Vec<usize>) = (0..items.len()).partition(|&i| items[i].y < ly);
    let mut out = vec![0.0; items.len()];
    for (l, (a, b)) in [(haut, (lo, ly - 1.0 - GAP)), (bas, (ly + 1.0 + GAP, hi))] {
        let sous: Vec<Place> = l.iter().map(|&i| items[i]).collect();
        for (k, y) in l.iter().zip(ranger(&sous, a, b)) {
            out[*k] = y;
        }
    }
    out
}

/// The small grey glass pill said when nothing is drawn yet, against the price column, just above the last price.
fn etat(pm: &mut Pixmap, text: &Text, m: &str, e: &Ecran, pw: f64, ph: f64, y_of: &dyn Fn(f64) -> f64) {
    let f = if pw < 560.0 { 10.0 } else { 11.5 };
    let h = if pw < 560.0 { 20.0 } else { 24.0 };
    let w = text.width(m, f) as f64 + 22.0;
    let y = if e.last > 0.0 { y_of(e.last) - h - 6.0 } else { ph / 2.0 };
    let y = y.clamp(haut_colonne(e), (ph - h - 2.0).max(2.0));
    let r = Rect::new(pw - 8.0 - w, y, w, h);
    round_fill(pm, r, h / 2.0, PILL_BG, 0.92);
    round_stroke(pm, r, h / 2.0, palette::edge(), 1.0);
    text.draw(pm, m, ((r.x + 11.0) as f32, (y + h / 2.0 + f as f64 * 0.36) as f32), f, WORD, Align::Left);
}

/// Top of the pill column (chart points): under the phone's legend (`Ecran::haut`) and (F1d, 22 Nov: on /app2/ PC
/// the top pill « VENTE 65,000 · … HYPERLIQUID » was cut in half) under the band of the top rows laid over the
/// plot's top ([`crate::haut_legende`]); 2 points from the plot's top at least.
fn haut_colonne(e: &Ecran) -> f64 {
    e.haut.max(crate::haut_legende()).max(2.0)
}

/// F1h / F1i (22 Nov, Jee's iPad: about 20 % of the plot stayed empty right of the last candle): the free band
/// right of the last candle, now that a wall in the band writes no price (it is on the price scale) — the widest
/// first form of the near walls' pills (PC « VENTE 12,4 M $ » and its tag, by the finger « 12,4 M »), plus the depth
/// bars' room, plus 8 points, rounded up to 10 points (no jitter) and never more than `cap` (the band of before).
pub fn marge_juste(e: &Ecran, p: &Inputs, pw: f64, cap: f64, tactile: bool) -> f64 {
    let t = tailles(pw, tactile);
    let short = pw < 560.0;
    let gras = Text::graisse(crate::text::Graisse::Bold);
    let premiere = formes(tactile)[0];
    let pilule = etiquettes(e, p, short, couleurs(p))
        .iter()
        .filter(|q| !q.loin)
        .map(|q| {
            let (s, m) = forme_texte(q, premiere);
            largeur_texte(&gras, &s, m, &t, short)
        })
        .fold(0.0, f64::max);
    let barres = if short { 40.0 } else { 70.0 };
    (((pilule + barres + 8.0) / 10.0).ceil() * 10.0).min(cap)
}

/// Height taken by the bottom pills (pill and margin under it).
pub fn bas_hauteur(pw: f64) -> f64 {
    if pw < 560.0 {
        22.0 + 6.0
    } else {
        32.0 + 14.0
    }
}

/// « 4 346 » on a PC; on a phone, long numbers are shortened (« 1,2 M », « 845 K »): the summary always fits.
pub fn coins_court(x: f64, small: bool) -> String {
    if small && x >= 10_000.0 {
        crate::bookheat::compact(x)
    } else {
        coins(x)
    }
}

/// F1f (Jee, 22 Nov, option A): money of the walls really filled during the last `REMPLI_BARS` candles — the REMPLI
/// marks still shown — (sells, buys), perp and spot together.
pub fn remplis_totaux(e: &Ecran) -> (f64, f64) {
    remplis_visibles(&e.vue.remplis, e.now_ms, e.bar_ms)
        .iter()
        .fold((0.0, 0.0), |(v, a), (r, _)| if r.sell { (v + r.usd, a) } else { (v, a + r.usd) })
}

/// The summary pill's part about the filled walls, after a thin separator (an empty text): PC « remplis (5 bougies) :
/// ventes 24 M $ · achats 7 M $ », `court` without « (5 bougies) », `small` « remplis 24 M · 7 M »; nothing when no
/// wall was filled.
fn remplis_parts(e: &Ecran, small: bool, court: bool, cols: [Rgb; 4]) -> Vec<(String, Rgb)> {
    let (v, a) = remplis_totaux(e);
    if !(v > 0.0 || a > 0.0) {
        return vec![];
    }
    let m = |x: f64| match (small, x > 0.0) {
        (true, true) => crate::bookheat::compact(x),
        (true, false) => "0 M".into(),
        (false, true) => dollars(x),
        (false, false) => "0 M $".into(),
    };
    let titre = match (small, court) {
        (true, _) => "remplis".to_string(),
        (false, true) => "remplis :".into(),
        (false, false) => format!("remplis ({REMPLI_BARS} bougies) :"),
    };
    let (vt, at) = if small {
        (m(v), m(a))
    } else {
        (format!("ventes {}", m(v)), format!("achats {}", m(a)))
    };
    vec![
        (String::new(), palette::GREY),
        (titre, palette::GREY),
        (vt, palette::on_chart(cols[0])),
        ("·".into(), palette::GREY),
        (at, palette::on_chart(cols[1])),
    ]
}

/// Width of the summary's parts (an empty part: the 1-point separator) with their gaps and paddings.
fn largeur_resume(text: &Text, parts: &[(String, Rgb)], f: f32, gap: f64, pad: f64) -> f64 {
    parts.iter().map(|(s, _)| if s.is_empty() { 1.0 } else { text.width(s, f) as f64 }).sum::<f64>() + gap * (parts.len() as f64 - 1.0) + 2.0 * pad
}

/// The summary pill's parts (text, colour).
fn resume(e: &Ecran, small: bool, cols: [Rgb; 4]) -> Vec<(String, Rgb)> {
    match e.totals {
        Some((sell, buy)) => vec![
            (
                if small {
                    format!("±{} %", crate::viewport::format_number(e.range, 0))
                } else {
                    format!("Carnets en direct, ±{} % :", crate::viewport::format_number(e.range, 0))
                },
                palette::GREY,
            ),
            (
                if small {
                    format!("ventes {}", coins_court(sell, true))
                } else {
                    format!("ventes {} {}", coins(sell), e.coin)
                },
                palette::on_chart(cols[0]),
            ),
            ("·".into(), palette::GREY),
            (
                if small {
                    format!("achats {}", coins_court(buy, true))
                } else {
                    format!("achats {} {}", coins(buy), e.coin)
                },
                palette::on_chart(cols[1]),
            ),
        ],
        None => vec![("Murs enregistrés (relecture)".into(), palette::GREY)],
    }
}

fn bas_cle_tient(sum_w: f64, kr: Rect) -> bool {
    8.0 + sum_w + 8.0 <= kr.x
}

/// 🧱 Where [`draw`] puts its bottom pills (« Carnets en direct… » and, when both fit, the « Perp · Comptant » key),
/// without drawing them (an empty image is drawn on): the tables are placed above them (`indicators::reserver`).
pub fn bas_rects(text: &Text, e: &Ecran, p: &Inputs, pw: f64, ph: f64) -> Vec<Rect> {
    if !p.on("summary") {
        return vec![];
    }
    match Pixmap::new(1, 1) {
        Some(mut pm) => bas(&mut pm, text, e, pw, ph, couleurs(p), p.on("pression")),
        None => vec![],
    }
}

/// The two pills at the bottom of the plot: the summary (left, always shown) and the colour key (right, dropped when
/// both do not fit).
fn bas(pm: &mut Pixmap, text: &Text, e: &Ecran, pw: f64, ph: f64, cols: [Rgb; 4], bulle: bool) -> Vec<Rect> {
    let small = pw < 560.0;
    // 🫧 F1g: the bubble of pressure right of the summary (8 points between), when the books are read
    let pression = e.pression.filter(|s| bulle && e.totals.is_some() && s.is_finite());
    let d = if small { 28.0 } else { 36.0 };
    let bulle_w = if pression.is_some() { 8.0 + d } else { 0.0 };
    let (h, f) = if small { (22.0, 10.0) } else { (32.0, 12.5) };
    let y = ph - h - if small { 6.0 } else { 14.0 };
    let base = (y + h / 2.0 + f as f64 * 0.36) as f32;
    let gap = if small { 8.0 } else { 18.0 };
    let pad = if small { 10.0 } else { 14.0 };
    // (F1f: the filled walls after it; within the screen less 16 points: shorter, then the phone's form, then left out)
    let base_parts = resume(e, small, cols);
    let parts = [(small, false), (small, true), (true, true)]
        .into_iter()
        .map(|(sm, court)| {
            let mut v = base_parts.clone();
            v.extend(remplis_parts(e, sm, court, cols));
            v
        })
        .chain(std::iter::once(base_parts.clone()))
        .find(|v| 8.0 + largeur_resume(text, v, f, gap, pad) + bulle_w <= pw - 8.0)
        .unwrap_or(base_parts);
    let sum_w = largeur_resume(text, &parts, f, gap, pad);
    // colour key on the right
    let mut key: Vec<(Vec<Rgb>, String)> = vec![];
    if e.perp {
        key.push((vec![cols[0], cols[1]], "Perp".into()));
    }
    if e.spot {
        let names = if small || e.spot_names.is_empty() {
            "Compt.".to_string()
        } else {
            format!("Comptant ({})", e.spot_names)
        };
        key.push((vec![cols[2], cols[3]], names));
    }
    let sq = if small { 8.0 } else { 11.0 };
    let key_w: f64 = key
        .iter()
        .map(|(c, s)| c.len() as f64 * (sq + 3.0) + 6.0 + text.width(s, f) as f64 + 18.0)
        .sum::<f64>()
        + 10.0;
    let kr = Rect::new(pw - 8.0 - key_w, y, key_w, h);
    let mut faits = vec![];
    if bas_cle_tient(sum_w + bulle_w, kr) {
        faits.push(kr);
        round_fill(pm, kr, h / 2.0, PILL_BG, 0.92);
        round_stroke(pm, kr, h / 2.0, palette::edge(), 1.0);
        let mut x = kr.x + 14.0;
        for (cs, s) in &key {
            for c in cs {
                round_fill(pm, Rect::new(x, y + h / 2.0 - sq / 2.0, sq, sq), 2.0, palette::on_chart(*c), 1.0);
                x += sq + 3.0;
            }
            x += 6.0;
            text.draw(pm, s, (x as f32, base), f, palette::GREY, Align::Left);
            x += text.width(s, f) as f64 + 18.0;
        }
    }
    // summary on the left
    let r = Rect::new(8.0, y, sum_w, h);
    round_fill(pm, r, h / 2.0, PILL_BG, 0.92);
    round_stroke(pm, r, h / 2.0, palette::edge(), 1.0);
    let mut x = r.x + pad;
    for (s, c) in &parts {
        if s.is_empty() {
            fill(pm, x, y + h * 0.25, 1.0, h * 0.5, palette::GREY, 0.6);
            x += 1.0 + gap;
            continue;
        }
        text.draw(pm, s, (x as f32, base), f, *c, Align::Left);
        x += text.width(s, f) as f64 + gap;
    }
    faits.push(r);
    BULLE_PRESSION.with(|b| b.set(None));
    if let Some(s) = pression {
        let br = Rect::new(r.x + r.w + 8.0, y + h / 2.0 - d / 2.0, d, d);
        let (pct, cote) = pression_cote(s);
        let c = match cote {
            Some(true) => palette::on_chart(cols[0]),
            Some(false) => palette::on_chart(cols[1]),
            None => palette::GREY,
        };
        round_fill(pm, br, d / 2.0, PILL_BG, 0.92);
        round_fill(pm, br, d / 2.0, c, 0.22);
        round_stroke(pm, br, d / 2.0, c, 1.6);
        // (a lighter tint of the colour)
        let clair = Rgb(c.0 / 2 + 128, c.1 / 2 + 128, c.2 / 2 + 128);
        let fs = if small { 9.0 } else { 11.0 };
        let gras = Text::graisse(crate::text::Graisse::Bold);
        gras.draw(
            pm,
            &format!("{pct} %"),
            (br.cx() as f32, (br.cy() + fs as f64 * 0.36) as f32),
            fs,
            clair,
            Align::Center,
        );
        BULLE_PRESSION.with(|b| b.set(Some(br)));
        faits.push(br);
    }
    faits
}

/// 🧱 « Cadrer sur les murs »: the price range holding the walls drawn (pills and SPOOF words) and the last price,
/// with a little room, never wider than the depth read (`range` %, ±3 % by default) around the last price; `None`:
/// nothing to frame.
pub fn cadre(e: &Ecran, last: f64) -> Option<(f64, f64)> {
    if !(last > 0.0) {
        return None;
    }
    let r = e.range.clamp(0.5, 5.0) / 100.0;
    let (min, max) = (last * (1.0 - r), last * (1.0 + r));
    let prices = e
        .vue
        .pills
        .iter()
        .map(|p| p.price)
        .chain(e.vue.words.iter().filter(|w| matches!(w.fin, Fin::Spoof(_))).map(|w| w.price))
        .filter(|p| (min..=max).contains(p));
    let (lo, hi) = prices.fold((last, last), |(lo, hi), p| (lo.min(p), hi.max(p)));
    // room above and below (the top and the bottom pills of the column), at least ±0.5 % around the price
    let pad = (hi - lo) * 0.08 + last * 0.002;
    let (lo, hi) = ((lo - pad).min(last * 0.995).max(min), (hi + pad).max(last * 1.005).min(max));
    (hi > lo).then_some((lo, hi))
}

/// 🏷️ Argos Murs' legend (mockup v5): the first pill « Argos Murs  Binance · Bybit · OKX · Bitunix · Hyperliquid 5/5 »,
/// the second « Carnets  Perp + Comptant »; on a phone « Argos Murs 4/5 » and « Perp + Compt. ».
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Legende {
    /// After the name (PC): the exchanges read and how many answered, or the replay's note.
    pub long: String,
    /// After the name (phone): the count, or a short replay note.
    pub court: String,
    /// The books: « Perp + Comptant » (phone: « Perp + Compt. »).
    pub carnets: String,
    pub carnets_court: String,
    /// Hidden above 4 h: the legend is written in grey.
    pub cache: bool,
}

/// The legend: exchanges read (`base` first, then « + » and the `autres`), how many answered (`ok` of `total`), the
/// books (`carnets`: 0 perp, 1 spot, 2 both), or the replay's `note`.
pub fn titre(base: &[&str], autres: &[&str], ok: usize, total: usize, carnets: usize, note: Option<&str>) -> Legende {
    let (c, cc) = match carnets {
        0 => ("Perp", "Perp"),
        1 => ("Comptant", "Compt."),
        _ => ("Perp + Comptant", "Perp + Compt."),
    };
    let (long, court) = match note {
        Some(n) => (n.to_string(), "pas de carnet".to_string()),
        None => {
            let noms = match (base.is_empty(), autres.is_empty()) {
                (false, false) => format!("{} + {}", base.join(" · "), autres.join(" · ")),
                (false, true) => base.join(" · "),
                (true, _) => autres.join(" · "),
            };
            (format!("{noms} {ok}/{total}"), format!("{ok}/{total}"))
        }
    };
    Legende {
        long,
        court,
        carnets: c.into(),
        carnets_court: cc.into(),
        cache: false,
    }
}

/// The legend while the drawing is hidden above 4 h (the books are still read): grey, « · caché au-dessus de 4 h ».
pub fn titre_cache(l: Legende) -> Legende {
    Legende {
        long: "· caché au-dessus de 4 h".into(),
        court: "· caché > 4 h".into(),
        cache: true,
        ..l
    }
}

/// The grey word of an ended wall (for the alerts).
pub fn alerte(fin: Fin) -> &'static str {
    match fin {
        Fin::Retire => "alRetire",
        Fin::Teste => "alTeste",
        Fin::Spoof(_) => "alSpoof",
        Fin::Mange => "alMange",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pill(price: f64, usd: f64, sell: bool, spot: bool) -> Pill {
        Pill {
            price,
            usd,
            sell,
            spot,
            since_ms: 0,
            venues: 3,
            tag: false,
            par: [0.0; PAR_BOOKS],
        }
    }

    fn ecran() -> Ecran {
        Ecran {
            vue: Vue {
                pills: vec![
                    pill(59_480.0, 18.4e6, true, false),
                    pill(59_120.0, 22.1e6, true, false),
                    pill(58_940.0, 31.6e6, true, false),
                    pill(59_880.0, 1.9e6, true, true),
                    pill(58_480.0, 26.3e6, false, false),
                    pill(58_480.0, 1.6e6, false, true),
                    pill(58_620.0, 1.8e6, false, true),
                    // too close to 58 940: the smaller one must go
                    pill(58_950.0, 3.0e6, true, false),
                ],
                words: vec![Word {
                    price: 59_800.0,
                    sell: true,
                    spot: false,
                    fin: Fin::Spoof(3),
                    at_ms: 0,
                    since_ms: 0,
                }],
                ..Default::default()
            },
            step: 20.0,
            totals: Some((4_346.0, 4_272.0)),
            coin: "BTC".into(),
            range: 3.0,
            spot_names: "Binance · Bybit · OKX · Coinbase".into(),
            perp: true,
            spot: true,
            ..Default::default()
        }
    }

    const COLS: [Rgb; 4] = [PERP_SELL, PERP_BUY, SPOT_SELL, SPOT_BUY];

    fn defs() -> Vec<f64> {
        PARAMS.iter().map(|p| p.def).collect()
    }

    #[test]
    fn meme_prix_une_seule_pilule_comptant_a_part() {
        let l = etiquettes(
            &ecran(),
            &Inputs {
                params: &PARAMS,
                vals: &defs(),
            },
            false,
            COLS,
        );
        let at = |p: f64| l.iter().find(|e| e.price == p).unwrap();
        assert_eq!(at(58_480.0).texte, "ACHAT 58\u{202f}480 · 26,3 M $ + COMPTANT 1,6 M $");
        assert_eq!(at(58_480.0).couleur, PERP_BUY);
        assert_eq!(at(59_880.0).texte, "VENTE 59\u{202f}880 · 1,9 M $ COMPTANT");
        assert_eq!(at(59_880.0).couleur, SPOT_SELL);
        assert_eq!(at(58_620.0).couleur, SPOT_BUY);
        assert_eq!(l.len(), 7, "the spot twin is merged, not drawn twice");
        assert!(etiquettes(
            &ecran(),
            &Inputs {
                params: &PARAMS,
                vals: &defs()
            },
            true,
            COLS
        )
        .iter()
        .any(|e| e.texte.ends_with("+ COMPT. 1,6 M $")));
    }

    #[test]
    fn rien_ne_se_chevauche_et_aucune_pilule_ne_part() {
        let items = [
            Place { y: 100.0, h: 28.0, prio: 3.0 },
            Place { y: 110.0, h: 28.0, prio: 9.0 },
            Place { y: 150.0, h: 28.0, prio: 1.0 },
            Place { y: 300.0, h: 14.0, prio: 0.5 },
        ];
        let at = ranger(&items, 0.0, 1000.0);
        assert_eq!(at.len(), 4, "nothing dropped");
        assert_eq!(at[3], 300.0, "an item alone stays at its price");
        // the three close ones are spread around where they wanted to be, in price order
        assert!(at[0] < at[1] && at[1] < at[2]);
        assert!((at[0] + at[1] + at[2]) / 3.0 - (100.0 + 110.0 + 150.0) / 3.0 < 1e-9);
        // an edge pushes a group back in
        let at = ranger(&items[..2], 0.0, 1000.0);
        assert!(at[0] - 14.0 >= 0.0);
        let at = ranger(&[Place { y: 995.0, h: 28.0, prio: 1.0 }], 0.0, 1000.0);
        assert_eq!(at[0], 986.0);
        // 📱 a top bound (under the phone's legend): items above it come down under it, still apart
        let at = ranger(&items, 200.0, 1000.0);
        assert!(at.iter().zip(&items).all(|(y, p)| y - p.h / 2.0 >= 200.0 - 1e-9), "{at:?}");
        // a phone and a PC, the mockup's walls with one more right next to 58 940: every pill is drawn, at least GAP
        // from the next, and each stays in price order
        for (pw, ph) in [(390.0 - 60.0, 620.0), (1020.0, 760.0)] {
            let e = ecran();
            let t = tailles(pw, false);
            let y_of = |p: f64| (60_300.0 - p) / 2_000.0 * ph;
            let l = etiquettes(
                &e,
                &Inputs {
                    params: &PARAMS,
                    vals: &defs(),
                },
                pw < 560.0,
                COLS,
            );
            let mut items: Vec<Place> = l
                .iter()
                .map(|q| Place {
                    y: y_of(q.price),
                    h: t.pill_h,
                    prio: q.usd,
                })
                .collect();
            items.extend(e.vue.words.iter().map(|w| Place {
                y: y_of(w.price),
                h: t.word as f64 + 3.0,
                prio: 0.0,
            }));
            let at = ranger(&items, 0.0, ph);
            assert_eq!(at.len(), items.len(), "{pw}: no pill dropped");
            let mut order: Vec<usize> = (0..items.len()).collect();
            order.sort_by(|a, b| items[*a].y.total_cmp(&items[*b].y));
            for w in order.windows(2) {
                let (a, b) = (w[0], w[1]);
                assert!(at[b] - items[b].h / 2.0 >= at[a] + items[a].h / 2.0 + GAP - 1e-9, "{pw}: {a} {b} overlap");
            }
            assert!(l.iter().any(|q| q.price == 58_950.0) && l.iter().any(|q| q.price == 58_940.0));
        }
    }

    /// Jee, 21 Nov: RETIRÉ and TESTÉ are never drawn (nor MANGÉ); SPOOF is, at most 6, the most recent first.
    #[test]
    fn retire_et_teste_jamais_dessines() {
        let text = Text::default();
        let vals = defs();
        let p = Inputs { params: &PARAMS, vals: &vals };
        let w = |price: f64, fin: Fin, at_ms: i64| Word {
            price,
            sell: true,
            spot: false,
            fin,
            at_ms,
            since_ms: 0,
        };
        let (pw, ph) = (1020u32, 760u32);
        let y_of = |px: f64| (60_300.0 - px) / 2_000.0 * ph as f64;
        let x_of = |_t: i64| Some(10.0);
        let image = |words: Vec<Word>| {
            let mut e = ecran();
            e.vue.words = words;
            let mut pm = Pixmap::new(pw, ph).unwrap();
            draw(&mut pm, &text, &e, &p, pw as f64, ph as f64, &y_of, &x_of);
            pm
        };
        let none = image(vec![]);
        let grey = image(vec![w(59_700.0, Fin::Retire, 5), w(59_300.0, Fin::Teste, 6), w(58_800.0, Fin::Mange, 7)]);
        assert!(none.data() == grey.data(), "RETIRÉ / TESTÉ / MANGÉ leave no pixel");
        let spoof = image(vec![w(59_700.0, Fin::Spoof(2), 5)]);
        assert!(none.data() != spoof.data(), "SPOOF is drawn");
        let many: Vec<Word> = (0..9).map(|k| w(59_000.0 + k as f64 * 100.0, Fin::Spoof(2), k)).collect();
        let l = mots(&many, &|_| true);
        assert_eq!(l.len(), MAX_WORDS);
        assert_eq!(l[0].at_ms, 8, "most recent first");
    }

    /// Mockup v5: a SPOOF at a price where no wall is (any more) gets its grey dashed line across the plot up to the
    /// word (5 px dash, 4 px gap), not only the word.
    #[test]
    fn spoof_sans_mur_a_sa_ligne_pointillee() {
        let text = Text::default();
        let vals = defs();
        let p = Inputs { params: &PARAMS, vals: &vals };
        let (pw, ph) = (1020u32, 760u32);
        let y_of = |px: f64| (60_300.0 - px) / 2_000.0 * ph as f64;
        let x_of = |_t: i64| Some(10.0);
        let image = |spoof: bool| {
            let mut e = ecran();
            e.vue.words = if spoof {
                vec![Word {
                    price: 59_700.0,
                    sell: true,
                    spot: false,
                    fin: Fin::Spoof(2),
                    at_ms: 1,
                    since_ms: 0,
                }]
            } else {
                vec![]
            };
            assert!(e.vue.pills.iter().all(|q| (q.price - 59_700.0).abs() > 100.0), "no wall at the SPOOF price");
            let mut pm = Pixmap::new(pw, ph).unwrap();
            draw(&mut pm, &text, &e, &p, pw as f64, ph as f64, &y_of, &x_of);
            pm
        };
        let (a, b) = (image(false), image(true));
        let y = y_of(59_700.0).round() as u32;
        let changed: Vec<bool> = (10..600).map(|x| a.pixel(x, y) != b.pixel(x, y)).collect();
        let n = changed.iter().filter(|c| **c).count();
        assert!(n > 250 && n < 450, "a dashed line ({n} px of 590 painted)");
        // dashes: gaps along the line
        assert!(changed.windows(2).filter(|w| w[0] && !w[1]).count() > 50, "dashes, not a solid line");
    }

    /// Never drop, shift: a wall whose price is under the bottom pills (summary, colour key) keeps its pill, moved
    /// up above them (lot 3d: a ▼ instead of its square, no line nor connector); on a phone as on a PC.
    #[test]
    fn mur_sous_le_resume_decale_pas_supprime() {
        let text = Text::default();
        let vals = defs();
        let p = Inputs { params: &PARAMS, vals: &vals };
        let x_of = |_t: i64| Some(10.0);
        for (pw, ph) in [(1020u32, 760u32), (390, 620)] {
            let y_of = |px: f64| (60_300.0 - px) / 2_000.0 * ph as f64;
            let price = 60_300.0 - (ph as f64 - 12.0) / ph as f64 * 2_000.0;
            let image = |with: bool| {
                let mut e = ecran();
                e.vue.words = vec![];
                e.vue.pills = if with { vec![pill(price, 20e6, false, false)] } else { vec![] };
                let mut pm = Pixmap::new(pw, ph).unwrap();
                draw(&mut pm, &text, &e, &p, pw as f64, ph as f64, &y_of, &x_of);
                pm
            };
            let (a, b) = (image(false), image(true));
            let bas_y = ph as f64 - bas_hauteur(pw as f64) - tailles(pw as f64, false).pill_h / 2.0 - GAP;
            assert!(y_of(price) > bas_y);
            let pilule = (pw - 150..pw - 12).any(|x| (0..bas_y as u32).any(|y| a.pixel(x, y) != b.pixel(x, y)));
            assert!(pilule, "{pw}: the pill is drawn above the bottom pills");
        }
    }

    #[test]
    fn dessin_sans_panique_et_textes() {
        let text = Text::default();
        let vals: Vec<f64> = PARAMS.iter().map(|p| p.def).collect();
        let p = Inputs { params: &PARAMS, vals: &vals };
        for (w, h) in [(330u32, 600u32), (1020, 760)] {
            let mut pm = Pixmap::new(w, h).unwrap();
            let y_of = |px: f64| (60_300.0 - px) / 2_000.0 * h as f64;
            let x_of = |_t: i64| Some(10.0);
            let mut e = ecran();
            e.depth = vec![(59_000.0, 5e6, true), (58_400.0, 7e6, false)];
            barres(&mut pm, &e, &p, w as f64, h as f64, &y_of);
            draw(&mut pm, &text, &e, &p, w as f64, h as f64, &y_of, &x_of);
            assert!(pm.data().iter().any(|b| *b != 0));
        }
        let l = titre(&["Binance"], &["Bybit"], 1, 2, 2, None);
        assert_eq!((l.long.as_str(), l.court.as_str()), ("Binance + Bybit 1/2", "1/2"));
        // all five, base first (mockup v6); a base book unticked is not listed; the others alone (fallback)
        let l = titre(&["Binance", "OKX", "Hyperliquid"], &["Bybit", "Bitunix"], 5, 5, 2, None);
        assert_eq!(l.long, "Binance · OKX · Hyperliquid + Bybit · Bitunix 5/5");
        assert_eq!(
            titre(&["Binance", "Hyperliquid"], &["Bybit", "Bitunix"], 4, 4, 0, None).long,
            "Binance · Hyperliquid + Bybit · Bitunix 4/4"
        );
        assert_eq!(titre(&[], &["Bybit"], 1, 1, 0, None).long, "Bybit 1/1");
        assert_eq!(titre_cache(l.clone()).long, "· caché au-dessus de 4 h");
        assert_eq!((l.carnets.as_str(), l.carnets_court.as_str()), ("Perp + Comptant", "Perp + Compt."));
        assert_eq!(
            titre(&[], &[], 0, 0, 0, Some("pas de carnet enregistré pour cette période")).long,
            "pas de carnet enregistré pour cette période"
        );
        assert_eq!(coins(4_346.4), "4\u{202f}346");
        assert_eq!(dollars(18.4e6), "18,4 M $");
    }

    /// The bug Jee saw (21 Nov): bars as tall as a price bucket made one opaque band along the price scale. At any zoom,
    /// with buckets as points or spread over their span, no bar is thicker than 3 px, nothing wider than the cap.
    #[test]
    fn barres_fines_a_tous_les_zooms() {
        let vals: Vec<f64> = PARAMS.iter().map(|p| p.def).collect();
        let p = Inputs { params: &PARAMS, vals: &vals };
        let mid = 60_000.0;
        let step = 50.0;
        let mut e = ecran();
        e.vue = Vue::default();
        e.last = mid;
        e.depth = (1..=36)
            .flat_map(|i| {
                let d = i as f64 * step;
                [(mid + d, 1e6 + (i % 7) as f64 * 3e5, true), (mid - d, 1e6 + (i % 5) as f64 * 4e5, false)]
            })
            .collect();
        for depth_step in [0.0, step] {
            e.depth_step = depth_step;
            for (w, h) in [(330u32, 600u32), (1020, 760)] {
                for pct in [0.05, 0.5, 2.0, 10.0, 40.0] {
                    let span = mid * pct / 100.0;
                    let y_of = |px: f64| (mid + span / 2.0 - px) / span * h as f64;
                    let mut pm = Pixmap::new(w, h).unwrap();
                    barres(&mut pm, &e, &p, w as f64, h as f64, &y_of);
                    let painted = |x: u32, y: u32| pm.pixel(x, y).is_some_and(|c| c.alpha() > 0);
                    // every column: no run of painted pixels taller than 3
                    for x in 0..w {
                        let mut run = 0;
                        for y in 0..h {
                            run = if painted(x, y) { run + 1 } else { 0 };
                            assert!(run <= 3, "{w}×{h} at {pct} % (step {depth_step}): a bar {run} px tall at x {x}");
                        }
                    }
                    let area = (0..w).flat_map(|x| (0..h).map(move |y| (x, y))).filter(|(x, y)| painted(*x, *y)).count();
                    // 0.05 %: 30 $ in view, the nearest bucket 50 $ away: nothing to draw there
                    assert!(area > 0 || pct < 0.5, "{pct} %: bars drawn");
                    if depth_step == 0.0 {
                        assert!((area as f64) < 0.06 * (w * h) as f64, "{w}×{h} at {pct} %: {area} px painted");
                    }
                    // nothing wider than the cap, nothing over the candles
                    let cap = if w < 560 { 40 } else { 70 };
                    assert!((0..w - cap - 1).all(|x| (0..h).all(|y| !painted(x, y))), "{pct} %: wider than {cap} px");
                }
            }
        }
        // above the last price: sell colour; below: buy colour (whatever side the bucket came from)
        e.depth_step = 0.0;
        let y_of = |px: f64| (61_000.0 - px) / 2_000.0 * 600.0;
        e.depth = vec![(60_500.0, 1e6, false), (59_500.0, 1e6, true)];
        let rows = barres_rows(&e, 1020.0, 600.0, &y_of);
        assert_eq!(rows.iter().map(|r| r.above).collect::<Vec<_>>(), vec![true, false]);
    }

    /// Mockup v5: a continuous histogram, one 3 px bar every 4 px whatever the zoom; zoomed in (a 5 $ bucket over
    /// 10 px), a bucket fills the rows of its own span (no 8 px hole); zoomed out, buckets are summed into a row.
    /// A price no bucket covers stays empty.
    #[test]
    fn barres_pas_de_4_px() {
        let mut e = ecran();
        e.vue = Vue::default();
        e.last = 60_000.0;
        e.depth_step = 5.0;
        // a dense book every 5 $ from 60 005 to 60 300, then nothing (no exchange goes further), then one bucket at 60 500
        e.depth = (1..=60).map(|i| (60_000.0 + 5.0 * i as f64, 1e5 * (1 + i % 4) as f64, true)).collect();
        e.depth.push((60_500.0, 2e5, true));
        for px_per_usd in [2.0, 0.8, 0.1] {
            let y_of = |px: f64| 400.0 - (px - 60_000.0) * px_per_usd;
            let rows = barres_rows(&e, 1020.0, 800.0, &y_of);
            let ys: Vec<f64> = rows.iter().map(|r| r.y).collect();
            // every bar on the 4 px grid
            assert!(ys.iter().all(|y| ((y - BAR_H / 2.0) / BAR_ROW).fract().abs() < 1e-9), "{px_per_usd}: {ys:?}");
            // over the dense part: one bar every 4 px, no hole
            let (top, bot) = (y_of(60_300.0 + 2.5), y_of(60_002.5));
            let inside: Vec<f64> = ys.iter().copied().filter(|y| *y > top + BAR_ROW && *y < bot - BAR_ROW).collect();
            assert!(
                inside.windows(2).all(|w| (w[1] - w[0] - BAR_ROW).abs() < 1e-9),
                "{px_per_usd}: hole in {inside:?}"
            );
            assert!(!inside.is_empty());
            // between 60 305 and 60 495 nothing (when that is more than a row wide)
            let (gt, gb) = (y_of(60_497.5), y_of(60_302.5));
            if gb - gt > 3.0 * BAR_ROW {
                // (one row of tolerance at each edge: the rows that hold the edge buckets)
                assert!(
                    ys.iter().all(|y| *y < gt + BAR_ROW || *y > gb - BAR_ROW),
                    "{px_per_usd}: a bar where no exchange gives depth"
                );
            }
        }
    }

    /// Length (lot 3c): `cap × √(row / reference)`, the reference being the 95th percentile of the class's rows (here
    /// one class of 12 rows), clipped at 70 px on a PC and 40 on a phone, at least 6 px; opacity 0.45 by the price.
    #[test]
    fn barres_normalisees_et_visibles() {
        let vals: Vec<f64> = PARAMS.iter().map(|p| p.def).collect();
        let p = Inputs { params: &PARAMS, vals: &vals };
        let mut e = ecran();
        e.vue = Vue::default();
        e.last = 60_000.0;
        // ten rows of 1 to 10 M $, one huge wall of 85 M $ and a tiny one, each on its own row
        e.depth = (1..=10).map(|i| (60_000.0 + 20.0 * i as f64, 1e6 * i as f64, true)).collect();
        e.depth.push((60_400.0, 85e6, true));
        e.depth.push((59_800.0, 0.01e6, false));
        let y_of = |px: f64| (61_000.0 - px) / 2_000.0 * 600.0;
        for (pw, cap) in [(1020.0, 70.0), (390.0, 40.0)] {
            let rows = barres_rows(&e, pw, 600.0, &y_of);
            assert_eq!(rows.len(), 12);
            let w_at = |px: f64| rows.iter().find(|r| (r.y - y_of(px)).abs() < BAR_ROW).unwrap().w;
            // 95th percentile of 12 rows = the 10 M $ one: it and the huge one reach the cap, 2.5 M $ is half of it
            assert_eq!(w_at(60_400.0), cap, "{pw}: the huge wall clipped");
            assert_eq!(w_at(60_200.0), cap);
            assert!((w_at(60_100.0) - cap * 0.5f64.sqrt()).abs() < 1e-9, "{pw}: square root");
            assert!(w_at(60_160.0) < cap, "{pw}: 8 M $ under the cap");
            assert_eq!(w_at(59_800.0), BAR_MIN, "{pw}: the tiny one still shows");
        }
        let mut pm = Pixmap::new(1020, 600).unwrap();
        barres(&mut pm, &e, &p, 1020.0, 600.0, &y_of);
        let y = y_of(60_020.0).floor() as u32;
        let a = (y.saturating_sub(3)..y + 4).filter_map(|y| pm.pixel(1015, y)).map(|c| c.alpha()).max().unwrap();
        assert!((100..=116).contains(&a), "alpha {a} (0.45 by the price)");
    }

    /// Lot 3c a): a realistic book is no flat block at the cap (at most 8 % of a class's rows reach it), and where a
    /// short snapshot ends (Binance ±0.2 %) the bars do not fall off a cliff when every book is as deep as the others.
    #[test]
    fn barres_par_classe_sans_bloc_ni_falaise() {
        let mut e = ecran();
        e.vue = Vue::default();
        e.last = 60_000.0;
        e.depth_step = 5.0;
        // book A (short snapshot) covers ±150 $, book B ±1 500 $; each holds the same irregular depth per 5 $
        e.reach = vec![(59_850.0, 60_150.0), (58_500.0, 61_500.0)];
        let par_carnet = |i: i64| 1e5 * (1.0 + ((i * 37) % 23) as f64 / 4.0);
        e.depth = (1..=300)
            .flat_map(|i| {
                let d = 5.0 * i as f64;
                let n = |px: f64| e.reach.iter().filter(|(lo, hi)| *lo <= px && px <= *hi).count() as f64;
                [
                    (60_000.0 + d, par_carnet(i) * n(60_000.0 + d), true),
                    (60_000.0 - d, par_carnet(i + 7) * n(60_000.0 - d), false),
                ]
            })
            .collect();
        // 1 px per 5 $: each row holds a bucket or so
        let y_of = |px: f64| 1_600.0 - (px - 58_400.0) / 2.0;
        for (pw, cap) in [(1020.0, 70.0), (390.0, 40.0)] {
            let rows = barres_rows(&e, pw, 1_600.0, &y_of);
            assert!(rows.len() > 200);
            let au_cap = rows.iter().filter(|r| r.w >= cap - 1e-9).count();
            assert!((au_cap as f64) <= 0.08 * rows.len() as f64, "{pw}: {au_cap} of {} at the cap", rows.len());
            // mean length just inside and just outside A's edge (60 150): about the same
            let moyenne = |lo: f64, hi: f64| {
                let l: Vec<f64> = rows.iter().filter(|r| r.y >= y_of(hi) && r.y <= y_of(lo)).map(|r| r.w).collect();
                l.iter().sum::<f64>() / l.len().max(1) as f64
            };
            let (dedans, dehors) = (moyenne(60_050.0, 60_145.0), moyenne(60_155.0, 60_250.0));
            assert!((dedans / dehors - 1.0).abs() < 0.25, "{pw}: cliff {dedans} → {dehors}");
            // the band read from one book only is paler
            let pale = |px: f64| rows.iter().find(|r| (r.y - y_of(px)).abs() < BAR_ROW).unwrap().pale;
            assert!(pale(60_400.0) < pale(60_100.0));
            assert_eq!(pale(60_100.0), 1.0);
        }
        assert!(PARAMS.iter().any(|p| p.key == "bars" && p.tip == "Plus pâle = moins de carnets à ce prix."));
    }

    /// « Cadrer sur les murs »: the walls and the last price inside, ±3 % at most, nothing set without walls far out.
    #[test]
    fn cadrer_sur_les_murs() {
        let mut e = ecran();
        let (lo, hi) = cadre(&e, 58_725.0).unwrap();
        assert!(lo < 58_480.0 && hi > 59_880.0, "{lo} {hi}");
        assert!(lo >= 58_725.0 * 0.97 && hi <= 58_725.0 * 1.03);
        // a wall far out (10 %) is not followed
        e.vue.pills.push(pill(65_000.0, 50e6, true, false));
        let (_, hi2) = cadre(&e, 58_725.0).unwrap();
        assert!(hi2 <= 58_725.0 * 1.03);
        assert_eq!(cadre(&e, 0.0), None);
    }

    /// REMPLI: drawn while younger than 5 candles, fading; gone after; never drawn for a wall without the mark.
    #[test]
    fn rempli_cinq_bougies_puis_parti() {
        let r = Rempli {
            price: 59_200.0,
            sell: true,
            spot: false,
            usd: 21.2e6,
            at_ms: 1_000_000,
        };
        let bar = 1_800_000;
        let v = remplis_visibles(&[r], r.at_ms + bar, bar);
        assert_eq!(v.len(), 1);
        let plus_tard = remplis_visibles(&[r], r.at_ms + 4 * bar, bar);
        assert!(plus_tard[0].1 < v[0].1, "it fades");
        assert!(remplis_visibles(&[r], r.at_ms + 5 * bar, bar).is_empty(), "gone after 5 candles");
        let text = Text::default();
        let vals = defs();
        let p = Inputs { params: &PARAMS, vals: &vals };
        let (w, h) = (1020u32, 760u32);
        let y_of = |px: f64| (60_300.0 - px) / 2_000.0 * h as f64;
        let x_of = |_t: i64| Some(10.0);
        let image = |l: Vec<Rempli>, now: i64| {
            let mut e = ecran();
            (e.vue.remplis, e.now_ms, e.bar_ms) = (l, now, bar);
            let mut pm = Pixmap::new(w, h).unwrap();
            draw(&mut pm, &text, &e, &p, w as f64, h as f64, &y_of, &x_of);
            pm
        };
        let sans = image(vec![], r.at_ms + bar);
        assert!(image(vec![r], r.at_ms + bar).data() != sans.data(), "REMPLI drawn");
        assert!(image(vec![r], r.at_ms + 5 * bar).data() == sans.data(), "and gone after 5 candles");
    }

    /// Phone, long numbers (XRP): the summary shortens them and is always drawn, the colour key gives way.
    #[test]
    fn resume_telephone_nombres_courts() {
        let mut e = ecran();
        e.coin = "XRP".into();
        e.totals = Some((48_312_904.0, 51_007_220.0));
        let parts = resume(&e, true, COLS);
        assert_eq!(parts[1].0, "ventes 48,3 M");
        assert_eq!(parts[3].0, "achats 51,0 M");
        assert_eq!(coins_court(4_346.0, true), "4\u{202f}346");
        let text = Text::default();
        let (w, h) = (330u32, 600u32);
        let mut pm = Pixmap::new(w, h).unwrap();
        bas(&mut pm, &text, &e, w as f64, h as f64, COLS, true);
        // the summary pill starts at x 8: its left edge is painted on the bottom band
        let y = h - 6 - 11;
        assert!((8..12).any(|x| pm.pixel(x, y).is_some_and(|c| c.alpha() > 0)));
    }

    /// F1f (Jee, 22 Nov, option A): the summary pill adds the money of the walls really filled during the last 5
    /// candles, sells and buys; nothing filled: the pill is exactly as before; it fits a 390 screen and never meets
    /// the colour key.
    #[test]
    fn resume_avec_les_remplis() {
        let mut e = ecran();
        e.totals = Some((5.142, 5.118));
        e.coin = "BTC".into();
        let bar = 60_000;
        e.bar_ms = bar;
        e.now_ms = 10 * bar;
        let r = |sell: bool, usd: f64, at: i64| Rempli {
            price: 60_000.0,
            sell,
            spot: false,
            usd,
            at_ms: at,
        };
        let text = Text::default();
        let dessin = |e: &Ecran, w: u32| {
            let mut pm = Pixmap::new(w, 600).unwrap();
            let z = bas(&mut pm, &text, e, w as f64, 600.0, COLS, true);
            (pm, z)
        };
        let sans = dessin(&e, 1400);
        assert_eq!(remplis_parts(&e, false, false, COLS), vec![]);
        // an old one (more than 5 candles): still nothing, same image
        e.vue.remplis = vec![r(true, 30e6, 2 * bar)];
        assert_eq!(remplis_totaux(&e), (0.0, 0.0));
        assert!(dessin(&e, 1400).0.data() == sans.0.data(), "nothing filled: the pill as before");
        // sells only: « 0 M $ » for the buys
        e.vue.remplis = vec![r(true, 24e6, 9 * bar), r(true, 1e6, 8 * bar)];
        let p = remplis_parts(&e, false, false, COLS);
        assert_eq!(p[1].0, "remplis (5 bougies) :");
        assert_eq!(p[2].0, format!("ventes {}", dollars(25e6)));
        assert_eq!(p[4].0, "achats 0 M $");
        // both sides, phone form
        e.vue.remplis.push(r(false, 7e6, 9 * bar));
        let p = remplis_parts(&e, true, true, COLS);
        assert_eq!(
            (p[1].0.as_str(), p[2].0.clone(), p[4].0.clone()),
            ("remplis", crate::bookheat::compact(25e6), crate::bookheat::compact(7e6))
        );
        // PC: wider than before; 390: inside the screen less 16, never on the key
        assert!(dessin(&e, 1400).1[1].w > sans.1[1].w);
        for w in [1400u32, 390, 330] {
            let (_, z) = dessin(&e, w);
            let resume = *z.last().unwrap();
            assert!(resume.x + resume.w <= w as f64 - 8.0 + 0.5, "{w}: {resume:?}");
            if z.len() == 2 {
                assert!(resume.x + resume.w + 8.0 <= z[0].x, "{w}: on the key");
            }
        }
    }

    /// F1g (Jee, 22 Nov, option A): the bubble of pressure — the dominant side's share, in its colour, grey between
    /// 45 and 55 %; smoothed (one jump moves it part of the way); off with its setting; in `bas_rects` (the tables
    /// stay above it); at 390 the summary (with its filled walls) and the bubble fit the screen less 16.
    #[test]
    fn bulle_de_pression() {
        assert_eq!(pression_cote(0.56), (56, Some(true)));
        assert_eq!(pression_cote(0.30), (70, Some(false)));
        assert_eq!(pression_cote(0.52).1, None);
        assert_eq!(pression_cote(0.46).1, None);
        assert!(
            pression_texte(0.56, 3.0).ends_with("vendeurs 56 % · achats 44 %"),
            "{}",
            pression_texte(0.56, 3.0)
        );
        assert!(pression_texte(0.5, 3.0).contains("équilibré"));
        // smoothing: a jump from 50 % to 80 % one second later moves it only a little; long after, all the way
        let (p0, t0) = lisser_pression(None, 0.5, 0);
        let (p1, _) = lisser_pression(Some((p0, t0)), 0.8, 1_000);
        assert!(p1 > 0.5 && p1 < 0.52, "{p1}");
        let (p2, _) = lisser_pression(Some((p1, 1_000)), 0.8, 600_000);
        assert!((p2 - 0.8).abs() < 1e-6);
        // drawn and in bas_rects; setting off: not there
        let vals = defs();
        let mut off = vals.to_vec();
        let k = PARAMS.iter().position(|p| p.key == "pression").unwrap();
        assert_eq!(k, PARAMS.len() - 1, "appended last: saved settings keep their places");
        off[k] = 0.0;
        let text = Text::default();
        let mut e = ecran();
        e.totals = Some((5.142, 5.118));
        e.coin = "BTC".into();
        e.pression = Some(0.56);
        let bar = 60_000;
        (e.bar_ms, e.now_ms) = (bar, 10 * bar);
        e.vue.remplis = vec![
            Rempli {
                price: 1.0,
                sell: true,
                spot: false,
                usd: 24e6,
                at_ms: 9 * bar,
            },
            Rempli {
                price: 1.0,
                sell: false,
                spot: false,
                usd: 7e6,
                at_ms: 9 * bar,
            },
        ];
        for w in [1400.0, 390.0] {
            let on = Inputs { params: &PARAMS, vals: &vals };
            let z = bas_rects(&text, &e, &on, w, 600.0);
            let b = *z.last().unwrap();
            assert!(
                (b.w - if w < 560.0 { 28.0 } else { 36.0 }).abs() < 1e-9 && (b.w - b.h).abs() < 1e-9,
                "{w}: the bubble {z:?}"
            );
            let resume = z[z.len() - 2];
            assert!((b.x - (resume.x + resume.w + 8.0)).abs() < 1e-9, "{w}: 8 points right of the summary");
            assert!(b.x + b.w <= w - 8.0 + 0.5, "{w}: inside the screen less 16");
            if z.len() == 3 {
                assert!(b.x + b.w + 8.0 <= z[0].x, "{w}: never on the key");
            }
            let sans = Inputs { params: &PARAMS, vals: &off };
            assert_eq!(bas_rects(&text, &e, &sans, w, 600.0).len(), z.len() - 1, "{w}: setting off, no bubble");
        }
        // no totals (replay): no bubble
        e.totals = None;
        let on = Inputs { params: &PARAMS, vals: &vals };
        assert!(bas_rects(&text, &e, &on, 1400.0, 600.0).iter().all(|r| (r.w - r.h).abs() > 1e-9));
    }

    /// F1i (Jee, 22 Nov 20:36: « enlève le prix des pastilles, on garde juste le montant d'ordre », PC, tablet and
    /// cell): every pill writes its amount only (PC « 26,3 M $ », finger « 26,3 M »), then the dot — off-screen walls
    /// too; no price, no side word, no exchange, no « + COMPTANT » in a pill. The prices of the walls on screen go on
    /// the scale in the wall's colour, a perp wall and its spot twin in two colours; tags never on each other (the
    /// bigger stays) nor on the last-price label.
    /// F1m / F1l: two walls whose prices the scale writes the same, met on the scale: one tag, both colours (perp
    /// first); off-screen tags keep their own; the price drawn with the scale's gap between thousands.
    #[test]
    fn deux_murs_meme_prix_une_etiquette_deux_couleurs() {
        let tag = |price: f64, texte: &str, c: Rgb, usd: f64, bord: Option<(f64, bool)>| EtiquetteAxe {
            price,
            texte: texte.into(),
            couleurs: vec![c],
            usd,
            bord,
        };
        let y_of = |p: f64| 1_000.0 - p / 100.0;
        let l = vec![
            tag(61_000.0, "61\u{202f}000", PERP_SELL, 77e6, None),
            tag(61_000.4, "61\u{202f}000", SPOT_SELL, 12e6, None),
        ];
        let mis = tags_axe(&l, &y_of, 20.0, 1_000.0, None);
        assert_eq!(mis.len(), 1, "{mis:?}");
        assert_eq!(mis[0].1.couleurs, vec![PERP_SELL, SPOT_SELL]);
        // a different price close by: the bigger stays, alone in its colour
        let l = vec![
            tag(61_000.0, "61\u{202f}000", PERP_SELL, 77e6, None),
            tag(61_050.0, "61\u{202f}050", SPOT_SELL, 12e6, None),
        ];
        let mis = tags_axe(&l, &|p| 1_000.0 - p / 1_000.0, 20.0, 1_000.0, None);
        assert_eq!(mis.len(), 1);
        assert_eq!(mis[0].1.couleurs, vec![PERP_SELL]);
        // the price drawn: the thousands as far apart as on the scale's labels
        let (gras, text) = (Text::graisse(crate::text::Graisse::Bold), Text::default());
        let s = crate::viewport::format_price(66_000.0, 0);
        assert!(largeur_prix(&gras, &text, &s, 12.0) >= gras.width("66", 12.0) as f64 + gras.width("000", 12.0) as f64 + espace_milliers(&text, 12.0));
        assert!(espace_milliers(&text, 12.0) >= 3.0, "a gap you see between bold digits");
        assert!(
            espace_milliers(&text, 12.0) > gras.width("\u{202f}", 12.0) as f64,
            "wider than the bold narrow space"
        );
        let mut pm = Pixmap::new(120, 30).unwrap();
        draw_prix(&mut pm, &gras, &text, &s, 4.0, 20.0, 12.0, Rgb(255, 255, 255));
        assert!(pm.pixels().iter().any(|p| p.red() > 200), "drawn");
        // off screen too (cell v1: « ▲ 61400 » red and « ▲ 61400 » orange): the perp wall and its spot twin above the
        // screen, PC and finger: one « ▲ » tag, in both colours
        let vals = defs();
        let p = Inputs { params: &PARAMS, vals: &vals };
        let x_of = |_t: i64| Some(10.0);
        for (tactile, pw) in [(false, 1520u32), (true, 390)] {
            crate::actions::set_tactile(tactile);
            let ph = 754u32;
            let y_of = |px: f64| (58_400.0 - px) / 1_000.0 * ph as f64;
            let mut pm = Pixmap::new(pw, ph).unwrap();
            draw(&mut pm, &text, &ecran(), &p, pw as f64, ph as f64, &y_of, &x_of);
            let ecrit = prix_axe(58_480.0, 20.0);
            let l: Vec<EtiquetteAxe> = etiquettes_axe().into_iter().filter(|t| t.texte == ecrit).collect();
            assert_eq!(l.len(), 1, "{tactile}: one tag for the price: {l:?}");
            assert_eq!(l[0].couleurs.len(), 2, "{tactile}: perp and spot: {l:?}");
            assert_eq!(l[0].bord.map(|b| b.1), Some(true), "{tactile}: ▲");
        }
        crate::actions::set_tactile(false);
    }

    #[test]
    fn montant_dans_la_pilule_prix_sur_l_echelle() {
        assert_eq!(montant_court(54e6), crate::viewport::format_number(54.0, 0) + " M");
        assert_eq!(montant_court(8.1e6), crate::viewport::format_number(8.1, 1) + " M");
        assert_eq!(montant_court(540e3), "540 k");
        assert_eq!(montant_court(8e6), "8 M");
        assert_eq!(formes(true), &[Forme::Montant, Forme::Point]);
        assert_eq!(formes(false), &[Forme::Complet, Forme::Point]);
        let vals = defs();
        let p = Inputs { params: &PARAMS, vals: &vals };
        let l = etiquettes(&ecran(), &p, false, COLS);
        let q = l.iter().find(|q| q.price == 58_480.0 && !q.spot).unwrap();
        assert_eq!(forme_texte(q, Forme::Complet), (dollars(26.3e6), None), "no side, no COMPTANT, no tag");
        assert_eq!(forme_texte(q, Forme::Montant).0, montant_court(26.3e6));
        // drawn: no price in a pill in the band (PC and finger), the prices on the scale
        let text = Text::default();
        let x_of = |_t: i64| Some(10.0);
        for tactile in [false, true] {
            crate::actions::set_tactile(tactile);
            let (pw, ph) = (1520u32, 754u32);
            let y_of = |px: f64| (60_000.0 - px) / 2_000.0 * ph as f64;
            let mut pm = Pixmap::new(pw, ph).unwrap();
            let e = ecran();
            draw(&mut pm, &text, &e, &p, pw as f64, ph as f64, &y_of, &x_of);
            let pilules = pilules_dessinees();
            assert!(!pilules.is_empty());
            for (_, s) in pilules.iter().filter(|(_, s)| !s.is_empty()) {
                let montant = |w: &Pill| if tactile { montant_court(w.usd) } else { dollars(w.usd) };
                assert!(e.vue.pills.iter().any(|w| montant(w) == *s), "{tactile}: the amount only, not {s:?}");
            }
            let tags = etiquettes_axe();
            let jumeaux = tags.iter().find(|t| t.price == 58_480.0).expect("the twin's tag");
            assert_eq!(
                jumeaux.couleurs,
                vec![palette::on_chart(couleurs(&p)[1]), palette::on_chart(couleurs(&p)[3])],
                "two colours"
            );
            assert_eq!(jumeaux.texte, prix_axe(58_480.0, 20.0));
            let vente = tags.iter().find(|t| t.price == 58_940.0).unwrap();
            assert_eq!(vente.couleurs, vec![palette::on_chart(couleurs(&p)[0])]);
            // on the scale: none on another, the bigger one stays; none on the last-price label
            let mis = tags_axe(&tags, &y_of, 20.0, ph as f64, None);
            for (i, (a, _)) in mis.iter().enumerate() {
                for (b, _) in &mis[i + 1..] {
                    assert!((a - b).abs() >= 20.0);
                }
            }
            assert!(mis.iter().any(|(_, t)| t.price == 58_940.0) && !mis.iter().any(|(_, t)| t.price == 58_950.0 && t.usd < 31.6e6));
            let ly = y_of(59_120.0);
            let sans = tags_axe(&tags, &y_of, 20.0, ph as f64, Some((ly - 10.0, ly + 10.0)));
            assert!(!sans.iter().any(|(y, _)| (y - ly).abs() < 20.0), "no tag on the last-price label");
        }
        crate::actions::set_tactile(false);
        // a wall outside the band by the finger: the amount alone too, no tag on the scale; its tooltip says the price
        crate::actions::set_tactile(true);
        let (pw, ph) = (390u32, 620u32);
        let y_of = |px: f64| (60_900.0 - px) / 1_200.0 * ph as f64;
        let mut e = ecran();
        e.vue.pills = vec![];
        e.vue.words = vec![];
        e.loin = vec![loin(66_000.0, 12e6, true)];
        let mut pm = Pixmap::new(pw, ph).unwrap();
        draw(&mut pm, &text, &e, &p, pw as f64, ph as f64, &y_of, &x_of);
        let s = pilules_dessinees();
        assert_eq!(s.len(), 1);
        assert!(s[0].1.is_empty() || s[0].1 == montant_court(12e6), "{s:?}");
        // its price on the scale all the same, « ▲ 66 000 », on the row of its pill (Jee, 20:37)
        let tags = etiquettes_axe();
        assert_eq!(tags.len(), 1);
        assert_eq!((tags[0].texte.clone(), tags[0].bord.map(|b| b.1)), (prix_axe(66_000.0, e.step), Some(true)));
        assert!((tags[0].bord.unwrap().0 - s[0].0.cy()).abs() <= 1.0, "the row of its pill");
        // two far walls above: two tags, in the pills' order, not on each other
        e.loin.push(loin(65_000.0, 8e6, true));
        draw(&mut pm, &text, &e, &p, pw as f64, ph as f64, &y_of, &x_of);
        let mis = tags_axe(&etiquettes_axe(), &y_of, 20.0, ph as f64, None);
        assert_eq!(mis.len(), 2, "{mis:?}");
        let (h66, h65) = (
            mis.iter().find(|(_, t)| t.price == 66_000.0).unwrap().0,
            mis.iter().find(|(_, t)| t.price == 65_000.0).unwrap().0,
        );
        assert!(h66 + 20.0 <= h65, "66 000 over 65 000");
        let z = draw(&mut pm, &text, &e, &p, pw as f64, ph as f64, &y_of, &x_of);
        let mut sv = survol_test();
        sv.pilules = z;
        let k = sv.pilules.iter().position(|z| z.price == 66_000.0).expect("the far pill");
        let bulle = infobulle(&sv, Vise::Pilule(&sv.pilules[k])).join(" ");
        let dec = dec_of(sv.step.max(0.0));
        assert!(
            bulle.contains(&crate::viewport::format_price(66_000.0, dec)),
            "the price in the tooltip: {bulle}"
        );
        crate::actions::set_tactile(false);
    }

    fn lot3c_ecran() -> Ecran {
        let mut e = ecran();
        e.vue.pills = vec![
            Pill {
                tag: true,
                ..pill(60_450.0, 81.7e6, true, false)
            },
            pill(61_200.0, 40.0e6, true, false),
            Pill {
                tag: true,
                ..pill(60_300.0, 79.4e6, false, false)
            },
            pill(60_300.0, 10.5e6, false, true),
            pill(59_100.0, 30.0e6, false, false),
        ];
        e.vue.words.clear();
        e.step = 50.0;
        e.last = 60_380.0;
        e
    }

    /// F1b (22 Nov): a wall pill writes its price as the price scale does: « 65 000 », not « 65 000,0 ».
    #[test]
    fn prix_comme_l_axe_sans_virgule_zero() {
        set_axe(crate::settings::ScaleMode::Normal, 0.0, 1);
        assert_eq!(prix_axe(65_000.0, 10.0), crate::viewport::format_price(65_000.0, 0), "a round price: no « ,0 »");
        assert!(!prix_axe(65_000.0, 10.0).contains(','));
        assert_eq!(
            prix_axe(65_000.5, 0.5),
            crate::viewport::format_price(65_000.5, 1),
            "its decimals when it has them"
        );
        set_axe(crate::settings::ScaleMode::Normal, 0.0, 2);
        assert_eq!(prix_axe(1.25, 0.05), crate::viewport::format_price(1.25, 2));
        assert_eq!(prix_axe(1.2, 0.1), crate::viewport::format_price(1.2, 1));
        AXE.with(|a| a.set(None));
        // the PC long form too: a 0.1 step, a round wall price → no « ,0 »
        let mut e = ecran();
        e.step = 0.1;
        let vals = defs();
        let l = etiquettes(&e, &Inputs { params: &PARAMS, vals: &vals }, false, COLS);
        let at = l.iter().find(|q| q.price == 58_480.0).unwrap();
        assert!(
            at.texte.starts_with(&format!("ACHAT {} ·", crate::viewport::format_price(58_480.0, 0))),
            "{}",
            at.texte
        );
    }

    /// Lot 3c, mockup v6: the tag BINANCE (phone BIN.) in the pill; a tagged wall never carries the merged spot part
    /// (its own spot pill); a merged pill too wide for the band is split on the phone; the PC band holds the widest
    /// « side price · amount  BINANCE » pill and the phone band the « BIN. » one.
    #[test]
    fn etiquette_binance_et_bande() {
        let vals = defs();
        let p = Inputs { params: &PARAMS, vals: &vals };
        let e = lot3c_ecran();
        let l = etiquettes(&e, &p, false, COLS);
        let achat = l.iter().find(|q| !q.spot && !q.sell && q.price == 60_300.0).unwrap();
        assert!(achat.tag.is_some() && !achat.texte.contains("COMPT"), "{}", achat.texte);
        assert!(l.iter().any(|q| q.spot && q.jumeau && q.price == 60_300.0 && q.texte.ends_with("COMPTANT")));
        let gras = Text::graisse(crate::text::Graisse::Bold);
        // PC: even a 6-digit price and a 3-digit amount; phone: BTC's usual « VENTE 60 450 · 81,7 M $ »
        for (pw, marge, short, texte) in [
            (1500.0, 330.0, false, "VENTE 104\u{202f}450 · 181,7 M $"),
            (330.0, 190.0, true, "VENTE 60\u{202f}450 · 81,7 M $"),
        ] {
            let t = tailles(pw, false);
            let tag = Etiquette {
                texte: texte.into(),
                ..achat.clone()
            };
            let w = largeur_pilule(&gras, &tag, &t, short);
            assert!(w + 8.0 <= marge, "{pw}: tagged pill {w} px in a band of {marge}");
            // untagged merged pill on the phone: split when it does not fit
            let mut e2 = e.clone();
            e2.vue.pills[2].tag = false;
            e2.marge = marge;
            let tient = |texte: &str| {
                let q = Etiquette {
                    texte: texte.into(),
                    tag: None,
                    ..achat.clone()
                };
                !short || largeur_pilule(&gras, &q, &t, short) + 8.0 <= marge
            };
            let l2 = etiquettes_dans(&e2, &p, short, COLS, &tient);
            // every perp pill fits; a merged one that would not is split (its spot part on its own)
            let larg: Vec<f64> = l2
                .iter()
                .filter(|q| !q.spot && (short || q.tag.is_some()))
                .map(|q| largeur_pilule(&gras, q, &t, short))
                .collect();
            assert!(larg.iter().all(|w| w + 8.0 <= marge), "{pw}: {larg:?}");
            assert_eq!(l2.iter().any(|q| q.jumeau), short, "{pw}: split only where it does not fit");
        }
        assert!(tag_largeur(&gras, true) < tag_largeur(&gras, false));
    }

    /// Lot 3e a): no spacing rule any more: zoomed out (1 px per 100 $, walls a few px apart), every wall of a tall
    /// plot is drawn, each pill on its own place, none overlapping.
    #[test]
    fn murs_serres_tous_dessines() {
        let text = Text::default();
        let vals = defs();
        let p = Inputs { params: &PARAMS, vals: &vals };
        let mut e = lot3c_ecran();
        e.vue.pills = (0..6).map(|k| pill(60_500.0 + 500.0 * k as f64, (10 + k) as f64 * 1e6, true, false)).collect();
        // 5 px between two walls
        let (pw, ph) = (1020u32, 760u32);
        let y_of = |px: f64| 400.0 - (px - 60_380.0) * 0.01;
        let x_of = |_t: i64| Some(10.0);
        let image = |pills: Vec<Pill>| {
            let mut e = e.clone();
            e.vue.pills = pills;
            let mut pm = Pixmap::new(pw, ph).unwrap();
            draw(&mut pm, &text, &e, &p, pw as f64, ph as f64, &y_of, &x_of);
            pm
        };
        let tout = image(e.vue.pills.clone());
        // each wall changes the image: none is left out
        for k in 0..e.vue.pills.len() {
            let mut sans = e.vue.pills.clone();
            sans.remove(k);
            assert_ne!(image(sans).data(), tout.data(), "wall {k} drawn");
        }
    }

    /// Lot 3e c), the 15 min case seen on Jee's view: four sell pills (one tagged Binance) and a short band above the
    /// last price: none goes below the line, none overlaps, the tagged one stays; the smallest leave first.
    #[test]
    fn quinze_minutes_prix_haut() {
        let (lo, ly, hi) = (40.0, 160.0, 700.0);
        let ventes = [
            (60_000.0, 13.4e6, false),
            (61_000.0, 20.8e6, false),
            (62_000.0, 43.0e6, true),
            (63_000.0, 5.0e6, false),
        ];
        let mut occ: Vec<Occupant> = ventes
            .iter()
            .enumerate()
            .map(|(k, &(_, usd, tag))| Occupant {
                y: ly - 10.0 - 8.0 * k as f64,
                h: 26.0,
                petit: false,
                usd,
                tag,
                lie: None,
            })
            .collect();
        // two buys below and a grey word above
        occ.push(Occupant {
            y: ly + 20.0,
            h: 26.0,
            petit: false,
            usd: 35e6,
            tag: true,
            lie: None,
        });
        occ.push(Occupant {
            y: ly + 30.0,
            h: 26.0,
            petit: false,
            usd: 4.5e6,
            tag: false,
            lie: Some(4),
        });
        occ.push(Occupant {
            y: ly - 5.0,
            h: 14.0,
            petit: true,
            usd: 0.0,
            tag: false,
            lie: None,
        });
        let garde = tenir(&occ, lo, hi, Some(ly));
        // room above: 160 - 1 - 4 - 40 = 115 px = 3 pills (3 × 26 + 2 × 4 = 86; 4 would take 116)
        assert!(!garde[6], "the grey word leaves first");
        assert_eq!(garde[..4], [true, true, true, false], "the smallest sell leaves, the tagged one stays");
        assert!(garde[4] && garde[5], "the buys have room");
        let items: Vec<Place> = occ
            .iter()
            .zip(&garde)
            .filter(|(_, k)| **k)
            .map(|(o, _)| Place { y: o.y, h: o.h, prio: o.usd })
            .collect();
        let at = ranger_autour(&items, lo, hi, Some(ly));
        for (i, (y, it)) in at.iter().zip(&items).enumerate() {
            assert!(y + it.h / 2.0 < ly || y - it.h / 2.0 > ly, "{i}: on the line");
            assert_eq!(*y < ly, it.y < ly, "{i}: its side");
            assert!(y - it.h / 2.0 >= lo - 1e-9 && y + it.h / 2.0 <= hi + 1e-9);
            for (j, (y2, it2)) in at.iter().zip(&items).enumerate().skip(i + 1) {
                assert!((y - y2).abs() >= (it.h + it2.h) / 2.0 - 1e-9, "{i} and {j} overlap");
            }
        }
        // a perp wall leaving takes its split spot part with it
        let lie = [
            Occupant {
                y: 10.0,
                h: 26.0,
                petit: false,
                usd: 1.0,
                tag: false,
                lie: None,
            },
            Occupant {
                y: 12.0,
                h: 26.0,
                petit: false,
                usd: 9.0,
                tag: false,
                lie: Some(0),
            },
            Occupant {
                y: 14.0,
                h: 26.0,
                petit: false,
                usd: 5.0,
                tag: false,
                lie: None,
            },
        ];
        assert_eq!(tenir(&lie, 0.0, 30.0, None), [false, false, true]);
    }

    /// Lot 3e: walls brought to the same edge keep their price order (59 400 under 59 600 at the bottom).
    #[test]
    fn murs_au_bord_dans_l_ordre() {
        let (lo, hi) = (40.0, 700.0);
        let items: Vec<Place> = [760.0, 820.0, 1200.0, -50.0, -10.0]
            .iter()
            .map(|&y| Place {
                y: au_bord(y, lo, hi),
                h: 26.0,
                prio: 1.0,
            })
            .collect();
        let at = ranger(&items, lo, hi);
        assert!(at[0] < at[1] && at[1] < at[2], "bottom: in price order {at:?}");
        assert!(at[3] < at[4], "top: in price order {at:?}");
        assert!(at.iter().all(|y| *y - 13.0 >= lo - 1e-9 && *y + 13.0 <= hi + 1e-9));
    }

    /// Lot 3e c): two groups clamped at the bottom edge never overflow it (the forward pass pushed the last one out).
    #[test]
    fn ranger_ne_deborde_pas_en_bas() {
        let items = [
            Place { y: 280.0, h: 26.0, prio: 1.0 },
            Place { y: 330.0, h: 26.0, prio: 1.0 },
            Place { y: 900.0, h: 26.0, prio: 1.0 },
        ];
        let at = ranger(&items, 0.0, 320.0);
        assert!(at.iter().all(|y| y + 13.0 <= 320.0 + 1e-9), "{at:?}");
        let mut s = at.clone();
        s.sort_by(f64::total_cmp);
        assert!(s.windows(2).all(|w| w[1] - w[0] >= 26.0 - 1e-9), "{at:?}");
    }

    /// Lot 3e b): a wall whose price is above the plot (zoomed in) keeps its pill at the top of the band, with ▲ and no
    /// line pixel; a grey word up there stays undrawn.
    #[test]
    fn mur_au_dessus_du_graphique() {
        let text = Text::default();
        let vals = defs();
        let p = Inputs { params: &PARAMS, vals: &vals };
        let x_of = |_t: i64| Some(10.0);
        let (pw, ph) = (1020u32, 760u32);
        let y_of = |px: f64| (60_300.0 - px) / 2_000.0 * ph as f64;
        let haut = 48.0;
        let image = |pills: Vec<Pill>| {
            let mut e = ecran();
            e.haut = haut;
            e.last = 59_500.0;
            e.vue.words = vec![];
            e.vue.pills = pills;
            let mut pm = Pixmap::new(pw, ph).unwrap();
            draw(&mut pm, &text, &e, &p, pw as f64, ph as f64, &y_of, &x_of);
            pm
        };
        let prix = 61_000.0;
        assert!(y_of(prix) < 0.0);
        let (a, b) = (image(vec![]), image(vec![pill(prix, 30e6, true, false)]));
        let diff = |x: u32, y: u32| a.pixel(x, y) != b.pixel(x, y);
        assert!(
            (haut as u32..haut as u32 + 30).any(|y| (pw - 200..pw - 12).any(|x| diff(x, y))),
            "the pill at the top of the band"
        );
        assert!((0..ph).all(|y| (0..pw / 2).all(|x| !diff(x, y))), "no line");
        assert_eq!(repere(y_of(prix), haut, ph as f64), Repere::Haut);
    }

    fn survol_test() -> Survol {
        let par = |v: &[f64]| {
            let mut p = [0.0; PAR_BOOKS];
            p[..v.len()].copy_from_slice(v);
            p
        };
        let barre = |y: f64, prix: (f64, f64), p: [f64; PAR_BOOKS]| Barre {
            y,
            w: 30.0,
            above: true,
            pale: 1.0,
            usd: p.iter().sum(),
            par: p,
            prix,
        };
        Survol {
            rows: vec![
                barre(101.5, (60_400.0, 60_480.0), par(&[1e6, 12.1e6, 2e6, 0.0, 5e5, 3e5, 2e5, 1e5])),
                barre(109.5, (60_380.0, 60_380.0), par(&[2e6])),
            ],
            pilules: vec![
                ZonePilule {
                    r: Rect::new(800.0, 300.0, 150.0, 26.0),
                    price: 60_500.0,
                    usd: 9e6,
                    sell: true,
                    spot: false,
                    par: par(&[1e6, 5e6, 3e6]),
                    comptant: 0.0,
                    loin: false,
                    tag: None,
                },
                ZonePilule {
                    r: Rect::new(800.0, 340.0, 150.0, 26.0),
                    price: 60_300.0,
                    usd: 4e6,
                    sell: true,
                    spot: true,
                    par: par(&[2.5e6, 1.5e6]),
                    comptant: 0.0,
                    loin: false,
                    tag: None,
                },
                ZonePilule {
                    r: Rect::new(800.0, 380.0, 150.0, 26.0),
                    price: 60_100.0,
                    usd: 6e6,
                    sell: false,
                    spot: false,
                    par: par(&[0.0, 6e6]),
                    comptant: 9.1e6,
                    loin: false,
                    tag: None,
                },
            ],
            depth: vec![(60_480.0, 4e6, true), (60_520.0, 5e6, true), (60_700.0, 7e6, true), (60_500.0, 1e6, false)],
            detail: vec![par(&[1e6, 3e6]), par(&[0.0, 5e6]), par(&[7e6]), par(&[1e6])],
            noms: ["Binance", "Hyperliquid", "OKX", "Bybit", "Bitunix", "Coinbase", "Kraken", "Autre"]
                .map(String::from)
                .to_vec(),
            noms_perp: ["Binance", "Hyperliquid", "OKX"].map(String::from).to_vec(),
            noms_spot: ["Coinbase", "Bybit"].map(String::from).to_vec(),
            perp: true,
            coin: "BTC".into(),
            step: 100.0,
            cols: COLS,
            cap: 70.0,
            bas: f64::INFINITY,
            pression: None,
        }
    }

    /// Lot 3e R2: what the mouse points at: a pill first; over the bars column, the row under the mouse or the
    /// nearest bar within 6 px; nothing left of the column or far from every bar.
    #[test]
    fn infobulle_zone_et_rangee() {
        let s = survol_test();
        let pw = 1020.0;
        let y = |v: Option<Vise>| match v {
            Some(Vise::Barre(b)) => Some(b.y),
            _ => None,
        };
        assert_eq!(y(vise(&s, 1000.0, 101.0, pw)), Some(101.5), "the row under the mouse");
        assert_eq!(y(vise(&s, 1000.0, 106.5, pw)), Some(109.5), "the nearest bar within 6 px");
        assert_eq!(vise(&s, 1000.0, 130.0, pw), None, "no bar near");
        assert_eq!(y(vise(&s, pw - 70.0 - 3.0, 101.0, pw)), Some(101.5), "the column starts at cap + 4 px");
        assert_eq!(vise(&s, pw - 70.0 - 5.0, 101.0, pw), None, "left of the column");
        assert!(matches!(vise(&s, 850.0, 310.0, pw), Some(Vise::Pilule(_))), "a pill");
        let no_bars = Survol { rows: vec![], ..survol_test() };
        assert!(
            matches!(vise(&no_bars, 850.0, 310.0, pw), Some(Vise::Pilule(_))),
            "bars off: the pills still answer"
        );
    }

    /// Lot 3e R2: the lines: side and price span, money and coins, then the exchanges, biggest first, at most six; a
    /// pill's split is summed from the fine buckets of its own bucket and side.
    #[test]
    fn infobulle_lignes() {
        let s = survol_test();
        let l = infobulle(&s, Vise::Barre(&s.rows[0]));
        assert!(l[0].starts_with("VENTE ") && l[0].contains(" – "), "{l:?}");
        assert!(l[1].contains("M $") && l[1].ends_with(" BTC"), "{l:?}");
        assert_eq!(l.len(), 2 + 6, "at most six exchanges: {l:?}");
        assert!(
            l[2].starts_with("Hyperliquid ") && l[3].starts_with("OKX ") && l[4].starts_with("Binance "),
            "{l:?}"
        );
        assert!(l.iter().all(|x| !x.starts_with("Bybit")), "no money: not listed");
        let un = infobulle(&s, Vise::Barre(&s.rows[1]));
        assert!(!un[0].contains(" – "), "one bucket: one price");
        // a bar row: its exchange lines add up to the row's total
        let b = &s.rows[0];
        assert!((b.par.iter().sum::<f64>() - b.usd).abs() < 1e-6);
        // a perp pill: its own split, recorded with its amount: the lines add up to the pill
        let z = s.pilules[0];
        let p = infobulle(&s, Vise::Pilule(&z));
        assert_eq!(p[2..], ["Hyperliquid 5,0 M $", "OKX 3,0 M $", "Binance 1,0 M $"].map(String::from), "{p:?}");
        assert!((z.par.iter().sum::<f64>() - z.usd).abs() < 1e-6);
        // a spot pill: the spot books' names
        let c = infobulle(&s, Vise::Pilule(&s.pilules[1]));
        assert!(c[2].starts_with("Coinbase ") && c[3].starts_with("Bybit ") && c.len() == 4, "{c:?}");
        // a merged pill: the perp amount and exchanges, then « Comptant »
        let f = infobulle(&s, Vise::Pilule(&s.pilules[2]));
        assert!(f[0].starts_with("ACHAT ") && f[1].starts_with("6,0 M $"), "{f:?}");
        assert_eq!(f[2..], ["Hyperliquid 6,0 M $", "Comptant 9,1 M $"].map(String::from), "{f:?}");
    }

    /// Lot 3e R2: the tooltip stays inside the plot, at the left of what it describes.
    #[test]
    fn infobulle_dans_le_graphique() {
        let (pw, ph) = (1020.0, 760.0);
        for (gauche, cy) in [(990.0, 2.0), (990.0, 758.0), (60.0, 300.0), (990.0, 380.0)] {
            let r = cadre_infobulle(180.0, 120.0, gauche, cy, pw, ph);
            assert!(r.x >= 0.0 && r.y >= 0.0 && r.x + r.w <= pw && r.y + r.h <= ph, "{gauche} {cy}: {r:?}");
        }
        let r = cadre_infobulle(180.0, 120.0, 990.0, 380.0, pw, ph);
        assert!(r.x + r.w <= 990.0 - 8.0 + 1e-9 && (r.cy() - 380.0).abs() < 1e-9);
        // drawn over the bar, nothing when the mouse is far from Argos Murs
        let text = Text::default();
        let s = survol_test();
        let mut a = Pixmap::new(pw as u32, ph as u32).unwrap();
        infobulle_draw(&mut a, &text, &s, 1000.0, 101.0, pw, ph);
        assert!((850..980).any(|x| (60..150).any(|y| a.pixel(x, y).unwrap().alpha() > 0)), "the tooltip");
        let mut b = Pixmap::new(pw as u32, ph as u32).unwrap();
        infobulle_draw(&mut b, &text, &s, 300.0, 400.0, pw, ph);
        assert!(b.data().iter().all(|x| *x == 0), "nothing");
    }

    /// Argos Vague below the price pane: the tooltip, its hit zone and the bar's highlight stay in the price pane.
    #[test]
    fn infobulle_dans_le_panneau_des_prix() {
        let text = Text::default();
        let s = Survol { bas: 140.0, ..survol_test() };
        let (pw, ph) = (1020.0, 760.0);
        let mut a = Pixmap::new(pw as u32, ph as u32).unwrap();
        infobulle_draw(&mut a, &text, &s, 1000.0, 109.5, pw, ph);
        assert!(a.data().iter().any(|x| *x != 0), "drawn");
        assert!(
            (140..ph as u32).all(|y| (0..pw as u32).all(|x| a.pixel(x, y).unwrap().alpha() == 0)),
            "nothing on the pane below"
        );
        let mut b = Pixmap::new(pw as u32, ph as u32).unwrap();
        infobulle_draw(&mut b, &text, &Survol { bas: 100.0, ..survol_test() }, 1000.0, 109.5, pw, ph);
        assert!(b.data().iter().all(|x| *x == 0), "mouse on the pane below: nothing");
    }

    /// Lot 3e: the tooltip never covers a wall pill: with a wider pill above the hovered one it goes at the left of
    /// it; with no room at the left, it stays where it was.
    #[test]
    fn infobulle_jamais_sur_une_pilule() {
        let (pw, ph) = (1020.0, 760.0);
        let survolee = Rect::new(800.0, 300.0, 200.0, 26.0);
        let large = Rect::new(640.0, 270.0, 360.0, 26.0);
        let pilules = [survolee, large, Rect::new(820.0, 340.0, 180.0, 26.0)];
        let r0 = cadre_infobulle(180.0, 110.0, survolee.x, survolee.cy(), pw, ph);
        assert!(pilules.iter().any(|p| se_touchent(r0, *p)), "today it meets the wide pill");
        let r = loin_des_pilules(r0, &pilules, pw, ph);
        assert!(pilules.iter().all(|p| !se_touchent(r, *p)), "{r:?}");
        assert!(r.x + r.w <= large.x - 8.0 + 1e-9 && (r.cy() - r0.cy()).abs() < 1e-9 && r.x >= 0.0);
        // no room at the left: unchanged
        let tout = [survolee, Rect::new(100.0, 270.0, 900.0, 26.0)];
        let r1 = cadre_infobulle(180.0, 110.0, survolee.x, survolee.cy(), pw, ph);
        assert_eq!(loin_des_pilules(r1, &tout, pw, ph), r1);
        // nothing in the way: unchanged
        assert_eq!(loin_des_pilules(r1, &[survolee], pw, ph), r1);
    }

    /// Lot 3e (phone): the tooltip never meets the round « ⋯ » button of the finger's crosshair: moved above it, or
    /// below it near the top, always inside the plot; away from it, unchanged.
    #[test]
    fn infobulle_loin_du_bouton() {
        let (pw, ph) = (330.0, 700.0);
        for y in [25.0, 300.0, 680.0] {
            let b = bouton_croix(pw, ph, y);
            assert!(b.x >= 0.0 && b.y >= 0.0 && b.x + b.w <= pw && b.y + b.h <= ph);
            // a pill at the right end, the tooltip at its left, on the crosshair's level
            let r0 = cadre_infobulle(150.0, 90.0, 200.0, y, pw, ph);
            let r = loin_du_bouton(r0, b, ph);
            let touche = r.x < b.x + b.w && r.x + r.w > b.x && r.y < b.y + b.h && r.y + r.h > b.y;
            assert!(!touche, "{y}: {r:?} meets {b:?}");
            assert!(r.y >= 0.0 && r.y + r.h <= ph, "{y}: inside the plot");
        }
        let loin = Rect::new(10.0, 10.0, 50.0, 50.0);
        assert_eq!(loin_du_bouton(loin, bouton_croix(pw, ph, 400.0), ph), loin);
    }

    /// Lot 3c b): a pill never sits on the last-price line; items keep their side of it.
    #[test]
    fn pilules_jamais_sur_la_ligne_du_prix() {
        let items = [
            Place { y: 300.0, h: 26.0, prio: 1.0 },
            Place { y: 306.0, h: 26.0, prio: 1.0 },
            Place { y: 318.0, h: 26.0, prio: 1.0 },
        ];
        let ly = 310.0;
        let at = ranger_autour(&items, 0.0, 800.0, Some(ly));
        for (y, it) in at.iter().zip(&items) {
            assert!(y + it.h / 2.0 < ly || y - it.h / 2.0 > ly, "{at:?}");
            assert_eq!(*y < ly, it.y < ly);
        }
    }

    /// Lot 3d: the marker of a pill is its square inside the free band, ▲ above it, ▼ below it.
    #[test]
    fn repere_carre_ou_fleche() {
        assert_eq!(repere(5.0, 40.0, 500.0), Repere::Haut);
        assert_eq!(repere(40.0, 40.0, 500.0), Repere::Carre);
        assert_eq!(repere(300.0, 40.0, 500.0), Repere::Carre);
        assert_eq!(repere(500.0, 40.0, 500.0), Repere::Carre);
        assert_eq!(repere(520.0, 40.0, 500.0), Repere::Bas);
    }

    /// Lot 3d: a wall under the header and one behind the bottom pills: nothing of them above the top bound nor below
    /// the bottom bound (no line, no connector), both pills inside the free band, each with its triangle.
    #[test]
    fn mur_hors_bande_sans_ligne_avec_fleche() {
        let text = Text::default();
        let vals = defs();
        let p = Inputs { params: &PARAMS, vals: &vals };
        let x_of = |_t: i64| Some(10.0);
        for (pw, ph) in [(1020u32, 760u32), (390, 620)] {
            let y_of = |px: f64| (60_300.0 - px) / 2_000.0 * ph as f64;
            let price_of = |y: f64| 60_300.0 - y / ph as f64 * 2_000.0;
            let haut = 48.0;
            let (p_haut, p_bas) = (price_of(12.0), price_of(ph as f64 - 12.0));
            let lo = haut;
            let hi = ph as f64 - bas_hauteur(pw as f64) - tailles(pw as f64, false).pill_h / 2.0 - GAP;
            assert_eq!(repere(y_of(p_haut), lo, hi), Repere::Haut);
            assert_eq!(repere(y_of(p_bas), lo, hi), Repere::Bas);
            let image = |pills: Vec<Pill>| {
                let mut e = ecran();
                e.haut = haut;
                e.last = 0.0;
                e.vue.words = vec![];
                e.vue.pills = pills;
                let mut pm = Pixmap::new(pw, ph).unwrap();
                draw(&mut pm, &text, &e, &p, pw as f64, ph as f64, &y_of, &x_of);
                pm
            };
            let a = image(vec![]);
            let b = image(vec![pill(p_haut, 26.5e6, true, false), pill(p_bas, 20e6, false, false)]);
            let diff = |x: u32, y: u32| a.pixel(x, y) != b.pixel(x, y);
            for y in (0..lo as u32).chain(hi.ceil() as u32 + 1..ph) {
                assert!((0..pw).all(|x| !diff(x, y)), "{pw}: something of a wall drawn at row {y}, outside the band");
            }
            // both pills are drawn, in the band
            assert!(
                (lo as u32..(lo + 40.0) as u32).any(|y| (pw - 150..pw - 12).any(|x| diff(x, y))),
                "{pw}: top pill"
            );
            assert!(
                ((hi - 40.0) as u32..hi as u32).any(|y| (pw - 150..pw - 12).any(|x| diff(x, y))),
                "{pw}: bottom pill"
            );
            // no line across the plot (left of the pills)
            assert!((0..ph).all(|y| (0..pw / 3).all(|x| !diff(x, y))), "{pw}: no line of an outside wall");
        }
    }

    /// F1d (22 Nov): with walls far above and far below the visible range, and the top rows' band laid over the
    /// plot's top, every pill (and dot) stays whole inside the free band: under that band, above the bottom pills; at
    /// PC width and at 390.
    #[test]
    fn pilules_entieres_dans_la_bande() {
        let text = Text::default();
        let vals = defs();
        let p = Inputs { params: &PARAMS, vals: &vals };
        let x_of = |_t: i64| Some(10.0);
        for (pw, ph, bande) in [(1700u32, 900u32, 0.0), (1700, 900, 26.0), (390, 620, 0.0)] {
            crate::set_haut_legende(bande);
            let y_of = |px: f64| (60_300.0 - px) / 2_000.0 * ph as f64;
            let mut e = ecran();
            e.haut = 0.0;
            e.vue.words = vec![];
            e.vue.pills = (0..8)
                .flat_map(|k| {
                    let d = 900.0 + 400.0 * k as f64;
                    [
                        pill(60_300.0 + d, 20e6 + k as f64, true, false),
                        pill(60_300.0 - 2_000.0 - d, 20e6 + k as f64, false, false),
                    ]
                })
                .chain([pill(60_290.0, 30e6, true, false)])
                .collect();
            let mut pm = Pixmap::new(pw, ph).unwrap();
            let z = draw(&mut pm, &text, &e, &p, pw as f64, ph as f64, &y_of, &x_of);
            assert!(!z.is_empty());
            for q in &z {
                assert!(
                    q.r.y >= bande.max(2.0) - 0.5 && q.r.y + q.r.h <= ph as f64 + 0.5,
                    "{pw}, {bande}: {:?} outside",
                    q.r
                );
            }
        }
        crate::set_haut_legende(0.0);
    }

    /// Lot 3c section 3: with nothing to draw yet, a small grey pill says why; with walls, it is not drawn.
    #[test]
    fn etat_quand_rien_a_dessiner() {
        let text = Text::default();
        let vals = defs();
        let p = Inputs { params: &PARAMS, vals: &vals };
        let (w, h) = (1020u32, 760u32);
        let y_of = |px: f64| (60_300.0 - px) / 2_000.0 * h as f64;
        let x_of = |_t: i64| Some(10.0);
        let img = |e: &Ecran| {
            let mut pm = Pixmap::new(w, h).unwrap();
            draw(&mut pm, &text, e, &p, w as f64, h as f64, &y_of, &x_of);
            pm
        };
        let mut e = ecran();
        e.vue = Vue::default();
        e.last = 59_500.0;
        let vide = img(&e);
        e.etat = Some("Lecture des carnets…".into());
        assert!(img(&e).data() != vide.data(), "said");
        let mut e2 = ecran();
        let avec = img(&e2);
        e2.etat = Some("Lecture des carnets…".into());
        assert!(img(&e2).data() == avec.data(), "not over the walls");
    }

    fn loin(price: f64, usd: f64, sell: bool) -> Lointain {
        Lointain {
            price,
            usd,
            coins: usd / price,
            sell,
            since_ms: 0,
        }
    }

    /// Lot loin-1 (mockup v3): two far sells above and two far buys below the screen are four pills in the free band,
    /// in price order, at its edges (PC: the first sell at the band's top, the next 30 px under it), ▲ / ▼, the
    /// HYPERLIQUID / HYP. tag, no pixel of a line; nothing overlaps, nothing on the last-price line.
    #[test]
    fn murs_lointains_au_bord_de_la_bande() {
        let text = Text::default();
        let vals = defs();
        let p = Inputs { params: &PARAMS, vals: &vals };
        let x_of = |_t: i64| Some(10.0);
        for (pw, ph) in [(1520u32, 754u32), (390, 620)] {
            let y_of = |px: f64| (60_900.0 - px) / 1_200.0 * ph as f64;
            let gras = Text::graisse(crate::text::Graisse::Bold);
            let t = tailles(pw as f64, false);
            let short = pw < 560;
            let base = || {
                let mut e = ecran();
                e.last = 60_300.0;
                e.vue.words = vec![];
                e.vue.pills = vec![];
                e
            };
            let mut e = base();
            e.loin = vec![
                loin(66_000.0, 31.8e6, true),
                loin(65_000.0, 20.5e6, true),
                loin(55_000.0, 14.5e6, false),
                loin(53_000.0, 24.6e6, false),
            ];
            let mut pm = Pixmap::new(pw, ph).unwrap();
            let z = draw(&mut pm, &text, &e, &p, pw as f64, ph as f64, &y_of, &x_of);
            let mut sans = Pixmap::new(pw, ph).unwrap();
            draw(&mut sans, &text, &base(), &p, pw as f64, ph as f64, &y_of, &x_of);
            assert_eq!(z.len(), 4, "{pw}");
            assert!(z.iter().all(|q| q.loin));
            let (lo, hi) = (2.0, ph as f64 - bas_hauteur(pw as f64) - t.pill_h / 2.0 - GAP);
            let at = |px: f64| z.iter().find(|q| q.price == px).unwrap().r;
            // in price order: 66 000 over 65 000 at the top, 55 000 over 53 000 at the bottom
            assert!(at(66_000.0).y < at(65_000.0).y && at(55_000.0).y < at(53_000.0).y, "{pw}");
            assert_eq!(at(66_000.0).y, lo, "{pw}: the top of the band");
            assert_eq!(at(65_000.0).y, lo + t.pill_h + GAP, "{pw}");
            assert!((at(53_000.0).y + t.pill_h - hi).abs() <= 1.0, "{pw}: the bottom of the band");
            assert_eq!(at(53_000.0).y - at(55_000.0).y, t.pill_h + GAP, "{pw}");
            let ly = y_of(60_300.0);
            for (i, a) in z.iter().enumerate() {
                assert!(a.r.y >= lo - 1e-9 && a.r.y + a.r.h <= hi + 1.0, "{pw}: {i} in the band");
                assert!(a.r.y + a.r.h < ly || a.r.y > ly, "{pw}: {i} on the last-price line");
                for b in z.iter().skip(i + 1) {
                    assert!(a.r.y + a.r.h <= b.r.y || b.r.y + b.r.h <= a.r.y, "{pw}: overlap");
                }
                // (F1i, Jee 22 Nov 20:36: the amount only in the pill, its HYPERLIQUID tag in the tooltip)
                let q = etiquettes(&e, &p, short, COLS).into_iter().find(|q| q.price == a.price).unwrap();
                assert_eq!(q.tag, Some(Marque::Hyperliquid));
                assert!(
                    (a.r.w - largeur_texte(&gras, &dollars(q.usd), None, &t, short)).abs() < 1e-9,
                    "{pw}: « 31,8 M $ »"
                );
            }
            let gauche = z.iter().map(|q| q.r.x).fold(f64::INFINITY, f64::min) as u32 - 6;
            assert!((0..ph).all(|y| (0..gauche).all(|x| pm.pixel(x, y) == sans.pixel(x, y))), "{pw}: no line");
            assert_eq!(repere(y_of(66_000.0), lo, hi), Repere::Haut);
            assert_eq!(repere(y_of(53_000.0), lo, hi), Repere::Bas);
            assert!(Marque::Hyperliquid.texte(short) == if short { "HYP." } else { "HYPERLIQUID" });
        }
    }

    /// Lot loin-1: a far level on screen (zoomed out) is a normal pill with its square and its line from `since_ms`;
    /// it is never the side's biggest wall (no thick band), and replay or « 0 » gives no far pill.
    #[test]
    fn mur_lointain_a_l_ecran_et_sans_bande_epaisse() {
        let text = Text::default();
        let vals = defs();
        let p = Inputs { params: &PARAMS, vals: &vals };
        let x_of = |_t: i64| Some(10.0);
        let (pw, ph) = (1520u32, 754u32);
        // zoomed out: 50 000 .. 70 000 on screen
        let y_of = |px: f64| (70_000.0 - px) / 20_000.0 * ph as f64;
        let mut e = ecran();
        e.last = 60_300.0;
        e.vue.words = vec![];
        e.vue.pills = vec![pill(61_000.0, 5e6, true, false)];
        let sans = {
            let mut pm = Pixmap::new(pw, ph).unwrap();
            draw(&mut pm, &text, &e, &p, pw as f64, ph as f64, &y_of, &x_of);
            pm
        };
        e.loin = vec![loin(65_000.0, 40e6, true)];
        let mut pm = Pixmap::new(pw, ph).unwrap();
        let z = draw(&mut pm, &text, &e, &p, pw as f64, ph as f64, &y_of, &x_of);
        let r = z.iter().find(|q| q.loin).unwrap().r;
        let y = y_of(65_000.0).round();
        assert!((r.cy() - y).abs() <= 1.0, "at its price");
        assert_ne!(pm.pixel(200, y as u32), sans.pixel(200, y as u32), "its line");
        // thin line only (3 px band, not the biggest wall's 7): 4 px off its price nothing changes
        assert_eq!(pm.pixel(200, y as u32 + 4), sans.pixel(200, y as u32 + 4), "no thick band");
        // the near wall keeps its thick band (the far one is bigger, but not counted)
        let yn = y_of(61_000.0).round() as u32;
        assert_ne!(sans.pixel(200, yn + 3), Pixmap::new(pw, ph).unwrap().pixel(200, yn + 3));
        assert_eq!(pm.pixel(200, yn + 3), sans.pixel(200, yn + 3));
    }

    /// Lot loin-1, the mockup's crowded case (PC, 5 min): 185 px above the last price, seven pills need 206 px, so
    /// the smallest (61 000 · 3,1 M $ COMPTANT) waits; far and near walls are treated alike, the BINANCE one stays.
    #[test]
    fn haut_charge_le_plus_petit_attend() {
        let text = Text::default();
        let vals = defs();
        let p = Inputs { params: &PARAMS, vals: &vals };
        let x_of = |_t: i64| Some(10.0);
        let (pw, ph) = (1520u32, 754u32);
        // the last price at y = 192: 192 − 1 − 4 − 2 = 185 px above it
        let y_of = |px: f64| 192.0 - (px - 60_300.0) * 0.12;
        let mut e = ecran();
        e.last = 60_300.0;
        e.step = 100.0;
        e.vue.words = vec![];
        e.vue.pills = vec![
            Pill {
                tag: true,
                ..pill(60_500.0, 2.0e6, true, false)
            },
            pill(60_700.0, 12e6, true, false),
            pill(60_900.0, 8e6, true, false),
            pill(61_000.0, 3.1e6, true, true),
            pill(61_200.0, 6e6, true, false),
        ];
        e.loin = vec![loin(65_000.0, 20.5e6, true), loin(66_000.0, 31.8e6, true)];
        let mut pm = Pixmap::new(pw, ph).unwrap();
        let z = draw(&mut pm, &text, &e, &p, pw as f64, ph as f64, &y_of, &x_of);
        let haut: Vec<&ZonePilule> = z.iter().filter(|q| q.sell).collect();
        assert_eq!(haut.len(), 6, "one waits");
        assert!(!z.iter().any(|q| q.spot && q.price == 61_000.0), "the smallest waits");
        assert!(z.iter().any(|q| q.price == 60_500.0), "the BINANCE wall stays (smaller, but tagged)");
        assert_eq!(z.iter().filter(|q| q.loin).count(), 2);
    }

    /// Lot loin-1: the tooltip of a far pill: side and price, money and coins, then Hyperliquid's money.
    #[test]
    fn infobulle_mur_lointain() {
        let mut s = survol_test();
        let z = ZonePilule {
            r: Rect::new(800.0, 100.0, 200.0, 26.0),
            price: 65_000.0,
            usd: 20.5e6,
            sell: true,
            spot: false,
            par: [0.0; PAR_BOOKS],
            comptant: 0.0,
            loin: true,
            tag: None,
        };
        s.pilules = vec![z];
        let l = infobulle(&s, Vise::Pilule(&s.pilules[0]));
        assert_eq!(l[0], "VENTE 65\u{202f}000 · PERP");
        assert_eq!(l[1], "20,5 M $ · 315 BTC");
        assert_eq!(l[2..], ["Hyperliquid 20,5 M $".to_string()]);
    }

    /// Z (editeur-pine-1, with the Interface thread): the real rects Murs drew — every wall pill, « Carnets en
    /// direct » and the colour key — are given to the high / low pills, which keep clear of them; none after
    /// `oublier` (Murs off / hidden / no book).
    #[test]
    fn zones_dessinees_et_pilule_du_plus_bas() {
        let text = Text::default();
        let vals = defs();
        let p = Inputs { params: &PARAMS, vals: &vals };
        let (w, h) = (1020.0, 760.0);
        let y_of = |px: f64| (60_300.0 - px) / 2_000.0 * h;
        let x_of = |_t: i64| Some(10.0);
        let mut pm = Pixmap::new(w as u32, h as u32).unwrap();
        set_garder(&[]);
        let z = draw(&mut pm, &text, &ecran(), &p, w, h, &y_of, &x_of);
        let r = zones_dessinees();
        assert!(z.iter().all(|q| r.contains(&q.r)), "every wall pill");
        let carnets = *r.iter().find(|x| x.x == 8.0).expect("« Carnets en direct »");
        assert!(carnets.y + carnets.h > h - 20.0, "at the bottom left");
        // the lowest price at the bottom left, where « Carnets en direct » is: its pill keeps clear of every rect
        let bars: Vec<argos_core::Bar> = (0..40)
            .map(|i| {
                let low = if i == 2 { 58_340.0 } else { 59_000.0 };
                argos_core::Bar {
                    time: i,
                    open: 59_100.0,
                    high: 59_200.0,
                    low,
                    close: 59_100.0,
                    volume: 1.0,
                }
            })
            .collect();
        assert!(
            (carnets.y..carnets.y + carnets.h).contains(&y_of(58_340.0)),
            "the wick tip is level with the pill"
        );
        let marks = crate::hilo::marks(&bars, 0, 39, &|i| 12.0 + i as f64 * 10.0, &y_of, &|_| 70.0, w, &r);
        let bas = marks.iter().find(|m| !m.high).expect("the lowest");
        assert!(r.iter().all(|x| !croise(x, &bas.pill)), "{:?} free of {r:?}", bas.pill);
        oublier();
        assert!(zones_dessinees().is_empty());
    }

    /// Z3: a rect of the app over a pill's place (the floating toolbar over the column): that pill is not drawn
    /// under it — it moves or waits.
    #[test]
    fn aucune_pilule_sous_un_rect_garde() {
        let text = Text::default();
        let vals = defs();
        let p = Inputs { params: &PARAMS, vals: &vals };
        let (w, h) = (1020.0, 760.0);
        let y_of = |px: f64| (60_300.0 - px) / 2_000.0 * h;
        let x_of = |_t: i64| Some(10.0);
        let mut pm = Pixmap::new(w as u32, h as u32).unwrap();
        set_garder(&[]);
        let avant = draw(&mut pm, &text, &ecran(), &p, w, h, &y_of, &x_of);
        let cible = avant[0].r;
        let g = Rect::new(cible.x - 20.0, cible.y - 4.0, cible.w + 40.0, cible.h + 8.0);
        set_garder(&[g]);
        let apres = draw(&mut pm, &text, &ecran(), &p, w, h, &y_of, &x_of);
        set_garder(&[]);
        assert!(apres.iter().all(|q| !croise(&q.r, &g)), "no pill under the kept rect");
        assert!(zones_dessinees().iter().filter(|x| x.x > 8.0).all(|x| !croise(x, &g) || x.y > h - 60.0));
    }

    /// 📍 Jee (25 Nov 2026, IMG_0075: « trop gros » on the tablet and the phone): the order pills left of the price
    /// column smaller by the finger: the phone's smaller than before, the tablet's as the phone's were; the mouse (PC)
    /// unchanged; the text still readable; the finger's tolerance around a pill still 24 points.
    #[test]
    fn the_wall_pills_are_smaller_by_the_finger() {
        let pc = tailles(1200.0, false);
        let (tablette, cell) = (tailles(1000.0, true), tailles(390.0, true));
        assert_eq!(tailles(390.0, false), cell, "a phone-wide chart: the same with the mouse");
        assert_eq!((pc.pill_h, pc.font, pc.word), (26.0, 13.0, 11.0), "the PC unchanged");
        assert!(tablette.pill_h <= 20.0 && tablette.font <= 10.5, "the tablet: {tablette:?}");
        assert!(cell.pill_h < 20.0 && cell.font < 10.5, "the phone smaller than before: {cell:?}");
        assert!(cell.pill_h >= 16.0 && cell.font >= 9.0 && cell.word >= 8.0, "still readable: {cell:?}");
        let r = Rect::new(100.0, 100.0, 30.0, cell.pill_h);
        assert!(
            touche(&r, 115.0, 100.0 + cell.pill_h / 2.0 + 11.5),
            "the finger's 24 points around a small pill"
        );
    }
}
