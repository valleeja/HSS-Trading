//! 🧭 Explorer Hyperliquid (Jee's mock-ups « explorer hyperliquid v1 », PC and phone, 22 Nov 2026): header
//! « 🧭 Explorer Hyperliquid » ⚙, the field « Adresse 0x… (portefeuille ou vault) », then for the address read: its
//! pill and « Compte vu le … · n positions · n ordres », 4 tiles (Valeur du compte, PnL 24 h, PnL 30 j, Levier moyen),
//! « ⭐ Suivre cette adresse » and « Voir sur la chart › », the tabs Positions / Ordres / Trades / Funding and their
//! table, and at the bottom the source and the freshness. Under ⚙: the addresses followed (remove) and the reading
//! interval. Wide (PC, 520 points and more): 4 tiles in a row and 6 columns; narrow (phone sheet, narrow panel): tiles
//! 2 × 2 and 4 columns (Marché with Long / Short under it, Taille with « entrée … » under it, PnL, Liq.).
//!
//! Also the French number helpers shared with 📶 Stats Hyperliquid (`crate::hlstats`).

use crate::jetons as j;
use crate::shapes::{round_fill, round_stroke, Rect};
use crate::text::{Align, Graisse, Rgb, Text};
use argos_data::explorer::{court, Compte, Fill, Funding, Order};
use std::collections::HashMap;
use tiny_skia::Pixmap;

/// The tabs.
pub const ONGLETS: [&str; 4] = ["Positions", "Ordres", "Trades", "Funding"];
/// The reading intervals offered under ⚙ (seconds, the account's).
pub const INTERVALLES: [u64; 3] = [10, 30, 60];

const PAD: f64 = 14.0;
const HEAD: f64 = 48.0;
const CHAMP_H: f64 = 36.0;
const TUILE_H: f64 = 64.0;
const BOUTON_H: f64 = 38.0;
const ONGLET_H: f64 = 36.0;
const PIED: f64 = 40.0;
/// ⚙'s place from the right edge: left of the panel's own ⬓ ⤢ ✕ (as in the Actualités).
const REGLAGES_X: f64 = 140.0;

// ── French numbers ────────────────────────────────────────────────────────────────────────────────────────────────

/// « 59 820,5 »: `dec` decimals, the French comma, thousands apart, the true minus « − ».
pub fn nombre(v: f64, dec: usize) -> String {
    let s = format!("{:.*}", dec, v.abs());
    let (ent, frac) = s.split_once('.').map_or((s.as_str(), ""), |(a, b)| (a, b));
    let mut g = String::new();
    for (i, c) in ent.chars().enumerate() {
        if i > 0 && (ent.len() - i) % 3 == 0 {
            g.push(' ');
        }
        g.push(c);
    }
    let zero = !s.chars().any(|c| c.is_ascii_digit() && c != '0');
    let signe = if v < 0.0 && !zero { "−" } else { "" };
    if frac.is_empty() {
        format!("{signe}{g}")
    } else {
        format!("{signe}{g},{frac}")
    }
}

/// « +3 120 $ », « −8 940 $ » (signed, no decimals).
pub fn dollars_signe(v: f64) -> String {
    let n = nombre(v, 0);
    if n.starts_with('−') || n == "0" {
        format!("{n} $")
    } else {
        format!("+{n} $")
    }
}

/// « 184 350 $ ».
pub fn dollars(v: f64) -> String {
    format!("{} $", nombre(v, 0))
}

/// Short amounts: « 1,42 Md$ », « 612 M$ », « 7,9 M$ », « 41 k$ », « 800 $ » (`dec_md`: decimals of the billions).
pub fn court_dollars(v: f64, dec_md: usize) -> String {
    let a = v.abs();
    let signe = if v < 0.0 { "−" } else { "" };
    let s = if a >= 1e9 {
        format!("{} Md$", nombre(a / 1e9, dec_md))
    } else if a >= 1e7 {
        format!("{} M$", nombre(a / 1e6, 0))
    } else if a >= 1e6 {
        format!("{} M$", nombre(a / 1e6, 1))
    } else if a >= 1e4 {
        format!("{} k$", nombre(a / 1e3, 0))
    } else {
        format!("{} $", nombre(a, 0))
    };
    format!("{signe}{s}")
}

/// « +0,0100 % » (`v` a fraction, `dec` decimals of the percentage), signed with the true minus.
pub fn pourcent(v: f64, dec: usize) -> String {
    let n = nombre(v * 100.0, dec);
    if n.starts_with('−') || !n.chars().any(|c| c.is_ascii_digit() && c != '0') {
        format!("{n} %")
    } else {
        format!("+{n} %")
    }
}

/// The decimals of a price as the exchange writes it (« 59820.5 » → 1).
pub fn decimales_de(mark: &str) -> usize {
    mark.split_once('.').map_or(0, |(_, d)| d.trim_end_matches('0').len())
}

/// The colour of a value as it is WRITTEN (`affiche`, from [`pourcent`] or [`dollars_signe`]): rounded to zero
/// (« 0,0 % », « 0 $ ») white like its missing sign, else the colour of a gain / a loss.
pub fn couleur_affichee(affiche: &str, v: f64) -> Rgb {
    if affiche.chars().any(|c| c.is_ascii_digit() && c != '0') {
        couleur(v)
    } else {
        j::TEXTE
    }
}

/// The colour of a gain / a loss.
fn couleur(v: f64) -> Rgb {
    if v >= 0.0 {
        crate::palette::up()
    } else {
        crate::palette::down()
    }
}

// ── The panel ─────────────────────────────────────────────────────────────────────────────────────────────────────

/// The module's state; the app fills the readings.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Explorer {
    /// The field takes the keyboard (its text).
    pub saisie: Option<String>,
    /// The address shown (lower case).
    pub adresse: Option<String>,
    pub compte: Option<Compte>,
    /// Time of the last account reading (ms).
    pub compte_ms: Option<i64>,
    pub ordres: Option<Vec<Order>>,
    pub fills: Option<Vec<Fill>>,
    pub funding: Option<Vec<Funding>>,
    /// PnL 24 h and 30 days (`portfolio`, or the fills when it cannot be read: `pnl_fills`).
    pub pnl: (Option<f64>, Option<f64>),
    pub pnl_fills: bool,
    pub onglet: usize,
    /// ⚙ open.
    pub reglages: bool,
    /// ⭐ The addresses followed (an alert when one opens or closes a position).
    pub suivies: Vec<String>,
    /// Index in [`INTERVALLES`].
    pub intervalle: usize,
    /// A message (invalid address, error).
    pub note: String,
    /// Now (ms) and each market's price decimals (from the app's price list).
    pub now_ms: i64,
    pub decimales: HashMap<String, usize>,
    /// Offset of the time shown (s), the app's.
    pub tz_s: i64,
}

/// Where a click lands.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Zone {
    Reglages,
    Champ,
    Suivre,
    Voir,
    Onglet(usize),
    /// A position's row (its coin): the chart of that market.
    Marche(String),
    /// ⚙ an address followed: show it / remove it; an interval.
    Suivie(String),
    Retirer(String),
    Intervalle(usize),
}

/// What a click asks of the app.
#[derive(Debug, Clone, PartialEq)]
pub enum ExClick {
    None,
    Changed,
    /// Read this address (valid, lower case).
    Lire(String),
    /// ⭐ Follow / stop following the address shown.
    Suivre(String),
    /// « Voir sur la chart › »: the address's entry and liquidation lines on the chart of this market.
    Voir(String, String),
    /// The chart of this market.
    Chart(String),
    /// A tab opened: its data read if stale.
    Onglet(usize),
}

impl Explorer {
    /// The account's reading interval (s).
    pub fn intervalle_s(&self) -> u64 {
        INTERVALLES[self.intervalle.min(INTERVALLES.len() - 1)]
    }

    /// Wide layout (PC): 4 tiles in a row, 6 columns.
    fn large(pw: f64) -> bool {
        pw >= 520.0 && !crate::actions::tactile()
    }

    fn dec(&self, coin: &str, prix: f64) -> usize {
        self.decimales.get(coin).copied().unwrap_or(if prix >= 1000.0 {
            1
        } else if prix >= 10.0 {
            2
        } else {
            4
        })
    }

    /// The address's followed state.
    pub fn suivie(&self) -> bool {
        self.adresse.as_ref().is_some_and(|a| self.suivies.contains(a))
    }

    /// Text typed (or pasted) in the field: a whole valid address is read at once.
    pub fn taper(&mut self, c: Option<char>) -> ExClick {
        let Some(s) = self.saisie.as_mut() else { return ExClick::None };
        match c {
            Some(c) if !c.is_control() && !c.is_whitespace() => s.push(c),
            None => {
                s.pop();
            }
            _ => {}
        }
        let s = s.trim().to_lowercase();
        if argos_data::whales::is_address(&s) {
            return self.valider();
        }
        ExClick::Changed
    }

    /// Enter in the field: a valid address is read, otherwise a message.
    pub fn valider(&mut self) -> ExClick {
        let s = self.saisie.clone().unwrap_or_default().trim().to_lowercase();
        if argos_data::whales::is_address(&s) {
            self.saisie = None;
            self.note.clear();
            if self.adresse.as_deref() != Some(s.as_str()) {
                let (suivies, intervalle, decimales, onglet) = (
                    std::mem::take(&mut self.suivies),
                    self.intervalle,
                    std::mem::take(&mut self.decimales),
                    self.onglet,
                );
                *self = Explorer {
                    suivies,
                    intervalle,
                    decimales,
                    onglet,
                    now_ms: self.now_ms,
                    tz_s: self.tz_s,
                    adresse: Some(s.clone()),
                    ..Default::default()
                };
            }
            return ExClick::Lire(s);
        }
        self.note = if s.is_empty() {
            String::new()
        } else {
            "Adresse invalide : 0x suivi de 40 caractères (adresse publique, jamais une clé privée)".into()
        };
        ExClick::Changed
    }

    /// The top of the account read; a note under the field (« ⚠️ Compte illisible : … ») moves it down (n° 13: the note
    /// was written over the address and its badge).
    fn haut_compte(&self) -> f64 {
        HEAD + CHAMP_H + 14.0 + if self.note.is_empty() { 0.0 } else { 20.0 }
    }

    /// Tiles (2 × 2 when narrow).
    fn tuiles(pw: f64, y: f64) -> Vec<Rect> {
        let large = Self::large(pw);
        let (n, g) = (if large { 4 } else { 2 }, 8.0);
        let w = (pw - 2.0 * PAD - g * (n as f64 - 1.0)) / n as f64;
        (0..4)
            .map(|k| {
                let (col, row) = (k % n, k / n);
                Rect::new(PAD + col as f64 * (w + g), y + row as f64 * (TUILE_H + g), w, TUILE_H)
            })
            .collect()
    }

    fn haut_tableau(&self, pw: f64) -> f64 {
        let y = self.haut_compte() + 58.0;
        let t = Self::tuiles(pw, y);
        let bas = t.last().map_or(y, |r| r.y + r.h);
        bas + 12.0 + BOUTON_H + 12.0 + ONGLET_H + 8.0
    }

    fn ligne_h(&self, pw: f64) -> f64 {
        if Self::large(pw) && self.onglet == 0 {
            44.0
        } else {
            52.0
        }
    }

    fn lignes(&self) -> usize {
        match self.onglet {
            0 => self.compte.as_ref().map_or(0, |c| c.positions.len()),
            1 => self.ordres.as_ref().map_or(0, Vec::len),
            2 => self.fills.as_ref().map_or(0, Vec::len),
            _ => self.funding.as_ref().map_or(0, Vec::len),
        }
    }

    /// The zones (content points).
    pub(crate) fn zones(&self, text: &Text, pw: f64) -> Vec<(Rect, Zone)> {
        let _ = text;
        // (n° 14: in the column of modules there is no ⬓ ⤢ ✕ of the panel: ⚙ went over the title's end; never before
        // the title's end, never out of the width)
        let titre = PAD + Text::graisse(Graisse::Bold).width("🧭 Explorer Hyperliquid", 17.0) as f64 + 10.0;
        let gx = (pw - REGLAGES_X).max(titre).min(pw - 38.0);
        let mut z = vec![
            (Rect::new(gx, 9.0, 30.0, 30.0), Zone::Reglages),
            (Rect::new(PAD, HEAD + 4.0, pw - 2.0 * PAD, CHAMP_H), Zone::Champ),
        ];
        if self.reglages {
            let mut y = HEAD + CHAMP_H + 48.0;
            for a in &self.suivies {
                z.push((Rect::new(pw - PAD - 84.0, y, 84.0, 30.0), Zone::Retirer(a.clone())));
                z.push((Rect::new(PAD, y, pw - 2.0 * PAD - 90.0, 30.0), Zone::Suivie(a.clone())));
                y += 36.0;
            }
            y += if self.suivies.is_empty() { 30.0 } else { 0.0 } + 34.0;
            let mut x = PAD;
            for (k, _) in INTERVALLES.iter().enumerate() {
                z.push((Rect::new(x, y, 72.0, 32.0), Zone::Intervalle(k)));
                x += 78.0;
            }
            return z;
        }
        if self.adresse.is_none() {
            return z;
        }
        let y = self.haut_compte() + 58.0;
        let t = Self::tuiles(pw, y);
        let yb = t.last().map_or(y, |r| r.y + r.h) + 12.0;
        let w = (pw - 2.0 * PAD - 10.0) / 2.0;
        z.push((Rect::new(PAD, yb, w, BOUTON_H), Zone::Suivre));
        z.push((Rect::new(PAD + w + 10.0, yb, w, BOUTON_H), Zone::Voir));
        let yo = yb + BOUTON_H + 12.0;
        let wo = (pw - 2.0 * PAD - 8.0) / 4.0;
        for k in 0..4 {
            z.push((Rect::new(PAD + 4.0 + k as f64 * wo, yo + 4.0, wo, ONGLET_H - 8.0), Zone::Onglet(k)));
        }
        if self.onglet == 0 {
            let (y0, h) = (self.haut_tableau(pw) + 24.0, self.ligne_h(pw));
            for (i, p) in self.compte.iter().flat_map(|c| &c.positions).enumerate() {
                z.push((Rect::new(6.0, y0 + i as f64 * h, pw - 12.0, h), Zone::Marche(p.coin.clone())));
            }
        }
        z
    }

    /// The content's height (it scrolls).
    pub fn content_h(&self, text: &Text, pw: f64) -> f64 {
        if self.reglages {
            return self.zones(text, pw).iter().map(|(r, _)| r.y + r.h).fold(0.0, f64::max) + 30.0 + PIED;
        }
        if self.adresse.is_none() {
            return HEAD + CHAMP_H + 140.0 + PIED;
        }
        self.haut_tableau(pw) + 24.0 + self.lignes().max(1) as f64 * self.ligne_h(pw) + 16.0 + PIED + crate::eco::marge_ai()
    }

    /// A click (content points).
    pub fn click(&mut self, text: &Text, pw: f64, x: f64, y: f64) -> ExClick {
        let Some(z) = self.zones(text, pw).into_iter().find(|(r, _)| r.contains(x, y)).map(|(_, z)| z) else {
            if self.saisie.as_deref().is_some_and(str::is_empty) {
                self.saisie = None;
                return ExClick::Changed;
            }
            return ExClick::None;
        };
        if z != Zone::Champ && self.saisie.as_deref().is_some_and(str::is_empty) {
            self.saisie = None;
        }
        match z {
            Zone::Reglages => self.reglages = !self.reglages,
            Zone::Champ => {
                self.saisie.get_or_insert_with(String::new);
            }
            Zone::Suivre => return self.adresse.clone().map_or(ExClick::None, ExClick::Suivre),
            Zone::Voir => {
                let (Some(a), Some(p)) = (self.adresse.clone(), self.compte.as_ref().and_then(|c| c.positions.first())) else {
                    self.note = "Aucune position ouverte à montrer sur la chart".into();
                    return ExClick::Changed;
                };
                return ExClick::Voir(a, p.coin.clone());
            }
            Zone::Onglet(k) => {
                self.onglet = k;
                return ExClick::Onglet(k);
            }
            Zone::Marche(c) => return ExClick::Chart(c),
            Zone::Suivie(a) => {
                self.reglages = false;
                self.saisie = Some(a);
                return self.valider();
            }
            Zone::Retirer(a) => self.suivies.retain(|s| *s != a),
            Zone::Intervalle(k) => self.intervalle = k,
        }
        ExClick::Changed
    }

    /// Draws the module (`pw` wide, `pm` as tall as [`Explorer::content_h`]).
    pub fn draw(&self, pm: &mut Pixmap, text: &Text, pw: f64, mouse: Option<(f64, f64)>, fond: Rgb) {
        let gras = Text::graisse(Graisse::Bold);
        let survol = |r: &Rect| mouse.is_some_and(|(x, y)| r.contains(x, y));
        crate::fill(pm, 0.0, 0.0, pw, crate::px::lh(pm), fond, 1.0);
        gras.draw(pm, "🧭 Explorer Hyperliquid", (PAD as f32, 31.0), 17.0, j::TEXTE, Align::Left);
        let zones = self.zones(text, pw);
        for (r, z) in &zones {
            match z {
                Zone::Reglages => {
                    round_fill(
                        pm,
                        *r,
                        r.h / 2.0,
                        if self.reglages { j::ACCENT } else { j::BLANC },
                        if self.reglages || survol(r) { 0.2 } else { 0.07 },
                    );
                    crate::phosphor::draw(pm, "gear", r.cx(), r.cy(), 16, j::TEXTE);
                }
                Zone::Champ => {
                    round_fill(pm, *r, r.h / 2.0, j::CHAMP.0, j::CHAMP.1);
                    round_stroke(pm, *r, r.h / 2.0, if self.saisie.is_some() { j::ACCENT } else { j::sur_fond(j::LISERE) }, 1.0);
                    let (s, c) = match self.saisie.as_deref() {
                        Some(q) => (format!("{q}|"), j::TEXTE),
                        None => ("🔎  Adresse 0x… (portefeuille ou vault)".to_string(), j::DIM),
                    };
                    text.draw(
                        pm,
                        &text.fit(&s, 13.5, r.w - 28.0),
                        ((r.x + 14.0) as f32, (r.cy() + 5.0) as f32),
                        13.5,
                        c,
                        Align::Left,
                    );
                }
                _ => {}
            }
        }
        if !self.note.is_empty() {
            let y = HEAD + CHAMP_H + 22.0;
            text.draw(
                pm,
                &text.fit(&self.note, 12.5, pw - 2.0 * PAD),
                (PAD as f32, y as f32),
                12.5,
                Rgb(0xff, 0xab, 0x00),
                Align::Left,
            );
        }
        if self.reglages {
            self.reglages_draw(pm, text, pw, &zones, &survol);
        } else if let Some(a) = &self.adresse {
            self.compte_draw(pm, text, pw, a, &zones, &survol);
        } else {
            let y = HEAD + CHAMP_H + 34.0 + if self.note.is_empty() { 0.0 } else { 18.0 };
            for (k, l) in [
                "Colle une adresse publique Hyperliquid (0x… 40 caractères) :",
                "valeur du compte, PnL, levier, positions, ordres, trades, funding.",
                "Lecture seule : aucune clé, aucun ordre.",
            ]
            .iter()
            .enumerate()
            {
                text.draw(
                    pm,
                    &text.fit(l, 12.5, pw - 2.0 * PAD),
                    (PAD as f32, (y + k as f64 * 19.0) as f32),
                    12.5,
                    j::DIM,
                    Align::Left,
                );
            }
        }
        // bottom: the source and the freshness
        let h = crate::px::lh(pm)
            - if self.adresse.is_some() && !self.reglages {
                crate::eco::marge_ai()
            } else {
                0.0
            };
        let y = h - PIED / 2.0 + 5.0;
        crate::fill(pm, PAD, h - PIED, pw - 2.0 * PAD, 1.0, j::BLANC, 0.08);
        text.draw(pm, "Source api.hyperliquid.xyz", (PAD as f32, y as f32), 12.5, j::DIM, Align::Left);
        if let Some(t) = self.compte_ms {
            let s = ((self.now_ms - t).max(0) / 1000).to_string();
            text.draw(pm, &format!("à jour il y a {s} s"), ((pw - PAD) as f32, y as f32), 12.5, j::DIM, Align::Right);
        }
    }

    fn reglages_draw(&self, pm: &mut Pixmap, text: &Text, pw: f64, zones: &[(Rect, Zone)], survol: &dyn Fn(&Rect) -> bool) {
        let gras = Text::graisse(Graisse::Bold);
        let y0 = HEAD + CHAMP_H + 34.0;
        gras.draw(pm, "ADRESSES SUIVIES", (PAD as f32, y0 as f32), 11.5, j::DIM, Align::Left);
        if self.suivies.is_empty() {
            text.draw(
                pm,
                "Aucune : « ⭐ Suivre cette adresse » l'ajoute ici.",
                (PAD as f32, (y0 + 24.0) as f32),
                12.5,
                j::DIM,
                Align::Left,
            );
        }
        let mut yi = 0.0;
        for (r, z) in zones {
            match z {
                Zone::Suivie(a) => {
                    if survol(r) {
                        round_fill(pm, *r, j::R_LIGNE, j::BLANC, 0.05);
                    }
                    text.draw(
                        pm,
                        &format!("⭐ {}", court(a)),
                        ((r.x + 6.0) as f32, (r.cy() + 5.0) as f32),
                        13.5,
                        j::TEXTE,
                        Align::Left,
                    );
                }
                Zone::Retirer(_) => {
                    round_fill(pm, *r, r.h / 2.0, j::BLANC, if survol(r) { 0.14 } else { 0.07 });
                    text.draw(pm, "Retirer", (r.cx() as f32, (r.cy() + 4.5) as f32), 12.5, j::TEXTE, Align::Center);
                }
                Zone::Intervalle(k) => {
                    yi = r.y;
                    let on = *k == self.intervalle.min(INTERVALLES.len() - 1);
                    round_fill(pm, *r, r.h / 2.0, if on { j::ACCENT } else { j::BLANC }, if on { 0.3 } else { 0.06 });
                    text.draw(
                        pm,
                        &format!("{} s", INTERVALLES[*k]),
                        (r.cx() as f32, (r.cy() + 4.5) as f32),
                        13.0,
                        j::TEXTE,
                        Align::Center,
                    );
                }
                _ => {}
            }
        }
        gras.draw(pm, "RELECTURE DU COMPTE", (PAD as f32, (yi - 12.0) as f32), 11.5, j::DIM, Align::Left);
        text.draw(
            pm,
            &text.fit(
                "Ordres, trades et funding : à l'ouverture de l'onglet, puis chaque minute.",
                12.0,
                pw - 2.0 * PAD,
            ),
            (PAD as f32, (yi + 52.0) as f32),
            12.0,
            j::DIM,
            Align::Left,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn compte_draw(&self, pm: &mut Pixmap, text: &Text, pw: f64, a: &str, zones: &[(Rect, Zone)], survol: &dyn Fn(&Rect) -> bool) {
        let gras = Text::graisse(Graisse::Bold);
        // identity: a square coloured from the address, its short form, « Compte vu le … »
        let y = self.haut_compte();
        let teinte = a.bytes().skip(2).take(6).fold(0u32, |h, b| h.wrapping_mul(31).wrapping_add(b as u32));
        let c1 = Rgb(
            0x20 + (teinte % 0x60) as u8,
            0xa0 + (teinte / 7 % 0x40) as u8,
            0xc0 + (teinte / 13 % 0x30) as u8,
        );
        round_fill(pm, Rect::new(PAD, y, 44.0, 44.0), 12.0, c1, 1.0);
        round_fill(pm, Rect::new(PAD + 14.0, y + 14.0, 30.0, 30.0), 10.0, j::ACCENT, 0.55);
        gras.draw(pm, &court(a), ((PAD + 56.0) as f32, (y + 18.0) as f32), 16.0, j::TEXTE, Align::Left);
        let vu = self.compte_ms.map_or("Lecture du compte…".to_string(), |t| {
            let d = chrono::DateTime::from_timestamp_millis(t + self.tz_s * 1000)
                .map(|d| d.naive_utc())
                .unwrap_or_default();
            let mois = [
                "janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.", "oct.", "nov.", "déc.",
            ];
            use chrono::{Datelike, Timelike};
            let n = self.compte.as_ref().map_or(0, |c| c.positions.len());
            let mut s = format!(
                "Compte vu le {} {}, {:02}:{:02} · {n} position{}",
                d.day(),
                mois[d.month0() as usize],
                d.hour(),
                d.minute(),
                if n > 1 { "s" } else { "" }
            );
            if let Some(o) = &self.ordres {
                s.push_str(&format!(" · {} ordre{}", o.len(), if o.len() > 1 { "s" } else { "" }));
            }
            s
        });
        text.draw(
            pm,
            &text.fit(&vu, 12.5, pw - PAD - 56.0 - PAD),
            ((PAD + 56.0) as f32, (y + 37.0) as f32),
            12.5,
            j::DIM,
            Align::Left,
        );
        // tiles
        let c = self.compte.as_ref();
        let pnl_n = if self.pnl_fills { " ≈" } else { "" };
        let tuiles: [(String, String, Rgb); 4] = [
            ("Valeur du compte".into(), c.map_or("—".into(), |c| dollars(c.value)), j::TEXTE),
            (
                format!("PnL 24 h{pnl_n}"),
                self.pnl.0.map_or("—".into(), dollars_signe),
                self.pnl.0.map_or(j::TEXTE, couleur),
            ),
            (
                format!("PnL 30 j{pnl_n}"),
                self.pnl.1.map_or("—".into(), dollars_signe),
                self.pnl.1.map_or(j::TEXTE, couleur),
            ),
            (
                "Levier moyen".into(),
                c.and_then(Compte::levier_moyen).map_or("—".into(), |l| format!("{}×", nombre(l, 1))),
                j::TEXTE,
            ),
        ];
        for (r, (l, v, col)) in Self::tuiles(pw, y + 58.0).iter().zip(tuiles) {
            round_fill(pm, *r, j::R_TUILE * 0.75, j::BLANC, 0.04);
            round_stroke(pm, *r, j::R_TUILE * 0.75, j::sur_fond(j::LISERE), 1.0);
            text.draw(
                pm,
                &text.fit(&l, 12.0, r.w - 20.0),
                ((r.x + 12.0) as f32, (r.y + 22.0) as f32),
                12.0,
                j::DIM,
                Align::Left,
            );
            let mut taille = 19.0;
            while taille > 12.0 && gras.width(&v, taille) as f64 > r.w - 20.0 {
                taille -= 1.0;
            }
            gras.draw(pm, &v, ((r.x + 12.0) as f32, (r.y + 49.0) as f32), taille, col, Align::Left);
        }
        for (r, z) in zones {
            match z {
                Zone::Suivre => {
                    let on = self.suivie();
                    round_fill(pm, *r, r.h / 2.0, j::ACCENT, if survol(r) { 0.34 } else { 0.22 });
                    round_stroke(pm, *r, r.h / 2.0, j::ACCENT, 0.9);
                    let l = if on { "⭐ Suivie · ne plus suivre" } else { "⭐ Suivre cette adresse" };
                    gras.draw(
                        pm,
                        &gras.fit(l, 13.5, r.w - 16.0),
                        (r.cx() as f32, (r.cy() + 5.0) as f32),
                        13.5,
                        j::TEXTE,
                        Align::Center,
                    );
                }
                Zone::Voir => {
                    round_fill(pm, *r, r.h / 2.0, j::BLANC, if survol(r) { 0.1 } else { 0.05 });
                    round_stroke(pm, *r, r.h / 2.0, j::sur_fond(j::LISERE), 1.0);
                    gras.draw(pm, "Voir sur la chart ›", (r.cx() as f32, (r.cy() + 5.0) as f32), 13.5, j::TEXTE, Align::Center);
                }
                Zone::Onglet(k) => {
                    if *k == 0 {
                        let seg = Rect::new(PAD, r.y - 4.0, pw - 2.0 * PAD, ONGLET_H);
                        round_fill(pm, seg, seg.h / 2.0, j::BLANC, 0.05);
                    }
                    let on = *k == self.onglet;
                    if on || survol(r) {
                        round_fill(pm, *r, r.h / 2.0, if on { j::ACCENT } else { j::BLANC }, if on { 0.35 } else { 0.06 });
                    }
                    (if on { &gras } else { text }).draw(
                        pm,
                        ONGLETS[*k],
                        (r.cx() as f32, (r.cy() + 4.5) as f32),
                        13.5,
                        if on { j::TEXTE } else { j::DIM },
                        Align::Center,
                    );
                }
                _ => {}
            }
        }
        self.tableau_draw(pm, text, pw, zones, survol);
    }

    /// The table's columns: (right edge, title). 📱 Narrow, positions: the PnL column ends where the widest
    /// liquidation price starts (14 points between them).
    fn colonnes(&self, text: &Text, pw: f64) -> Vec<(f64, &'static str)> {
        let gras = Text::graisse(Graisse::Bold);
        let large = Self::large(pw) && self.onglet == 0;
        let r = pw - PAD;
        match (self.onglet, large) {
            (0, true) => vec![
                (r - 310.0, "Taille"),
                (r - 236.0, "Entrée"),
                (r - 166.0, "Prix"),
                (r - 92.0, "PnL"),
                (r, "Liq."),
            ],
            (0, false) => {
                // (the PnL column ends where the widest liquidation price starts, 14 points between them: a 7-digit
                // price never touches it)
                let (mut liq_w, mut pnl_w) = (text.width("Liq.", 12.0) as f64, text.width("PnL", 12.0) as f64);
                for p in self.compte.iter().flat_map(|c| c.positions.iter()) {
                    if let Some(l) = p.liq {
                        liq_w = liq_w.max(text.width(&nombre(l, self.dec(&p.coin, p.entry)), 13.5) as f64);
                    }
                    pnl_w = pnl_w.max(gras.width(&dollars_signe(p.pnl), 13.5) as f64);
                }
                let pnl = (r - 64.0).min(r - liq_w - 14.0);
                vec![(pnl.min(r - 150.0).min(pnl - pnl_w - 14.0), "Taille"), (pnl, "PnL"), (r, "Liq.")]
            }
            (1, _) => vec![(r - 170.0, "Côté"), (r - 80.0, "Prix"), (r, "Taille")],
            (2, _) => vec![(r - 170.0, "Sens"), (r - 80.0, "Prix"), (r, "PnL")],
            _ => vec![(r - 170.0, "Taux 1 h"), (r - 80.0, "Taille"), (r, "Montant")],
        }
    }

    fn tableau_draw(&self, pm: &mut Pixmap, text: &Text, pw: f64, zones: &[(Rect, Zone)], survol: &dyn Fn(&Rect) -> bool) {
        let gras = Text::graisse(Graisse::Bold);
        let y0 = self.haut_tableau(pw);
        let large = Self::large(pw) && self.onglet == 0;
        let h = self.ligne_h(pw);
        let cols = self.colonnes(text, pw);
        text.draw(pm, "Marché", (PAD as f32, (y0 + 14.0) as f32), 12.0, j::DIM, Align::Left);
        for (x, t) in &cols {
            text.draw(pm, t, (*x as f32, (y0 + 14.0) as f32), 12.0, j::DIM, Align::Right);
        }
        let y0 = y0 + 24.0;
        let rien = |pm: &mut Pixmap, m: &str| text.draw(pm, m, (PAD as f32, (y0 + 26.0) as f32), 12.5, j::DIM, Align::Left);
        let logo = |pm: &mut Pixmap, coin: &str, cy: f64| crate::watchlist::coin_badge(pm, text, coin, None, PAD + 11.0, cy, 11.0);
        let nom = |c: &str| c.split_once(':').map_or(c, |(_, n)| n).to_string();
        let quand = |t: i64| -> String {
            let d = chrono::DateTime::from_timestamp_millis(t + self.tz_s * 1000)
                .map(|d| d.naive_utc())
                .unwrap_or_default();
            d.format("%d/%m %H:%M").to_string()
        };
        match self.onglet {
            0 => {
                let Some(c) = &self.compte else { return rien(pm, "Lecture du compte…") };
                if c.positions.is_empty() {
                    return rien(pm, "Aucune position ouverte");
                }
                for (i, p) in c.positions.iter().enumerate() {
                    let y = y0 + i as f64 * h;
                    let zr = zones.iter().find(|(_, z)| *z == Zone::Marche(p.coin.clone())).map(|(r, _)| *r);
                    if zr.is_some_and(|r| survol(&r)) {
                        round_fill(pm, Rect::new(6.0, y + 2.0, pw - 12.0, h - 4.0), j::R_LIGNE, j::BLANC, 0.05);
                    }
                    let dec = self.dec(&p.coin, p.entry);
                    let (sens, sc) = if p.long() {
                        ("Long", crate::palette::up())
                    } else {
                        ("Short", crate::palette::down())
                    };
                    let pill = format!("{sens} {}×", nombre(p.leverage, 0));
                    let pill_w = text.width(&pill, 11.0) as f64 + 12.0;
                    let pastille = |pm: &mut Pixmap, x: f64, cy: f64| {
                        let b = Rect::new(x, cy - 9.0, pill_w, 18.0);
                        round_fill(pm, b, 6.0, sc, 0.16);
                        text.draw(pm, &pill, ((b.x + 6.0) as f32, (b.cy() + 4.0) as f32), 11.0, sc, Align::Left);
                    };
                    let liq = p.liq.map_or("—".to_string(), |l| nombre(l, dec));
                    let orange = Rgb(0xf5, 0x9e, 0x0b);
                    if large {
                        let cy = y + h / 2.0;
                        logo(pm, &p.coin, cy);
                        let n = nom(&p.coin);
                        gras.draw(pm, &n, ((PAD + 28.0) as f32, (cy + 5.0) as f32), 14.0, j::TEXTE, Align::Left);
                        pastille(pm, PAD + 34.0 + gras.width(&n, 14.0) as f64, cy);
                        let vals = [
                            (nombre(p.size.abs(), if p.size.abs() >= 100.0 { 0 } else { 2 }), j::TEXTE),
                            (nombre(p.entry, dec), j::TEXTE),
                            (nombre(p.price, dec), j::TEXTE),
                            (dollars_signe(p.pnl), couleur_affichee(&dollars_signe(p.pnl), p.pnl)),
                            (liq, orange),
                        ];
                        for ((x, _), (v, col)) in cols.iter().zip(vals) {
                            let f = if col == j::TEXTE { text } else { &gras };
                            f.draw(pm, &v, (*x as f32, (cy + 5.0) as f32), 13.5, col, Align::Right);
                        }
                    } else {
                        let (l1, l2) = (y + 20.0, y + 40.0);
                        logo(pm, &p.coin, y + h / 2.0);
                        gras.draw(pm, &nom(&p.coin), ((PAD + 28.0) as f32, l1 as f32), 14.0, j::TEXTE, Align::Left);
                        pastille(pm, PAD + 28.0, l2 - 4.0);
                        text.draw(
                            pm,
                            &nombre(p.size.abs(), if p.size.abs() >= 100.0 { 0 } else { 2 }),
                            (cols[0].0 as f32, l1 as f32),
                            13.5,
                            j::TEXTE,
                            Align::Right,
                        );
                        // (n° 14, narrow column of modules: « entrée … » went over « Long 5× »: the number alone then)
                        let entree = format!("entrée {}", nombre(p.entry, dec));
                        let libre = cols[0].0 - (PAD + 28.0 + pill_w + 6.0);
                        let entree = if text.width(&entree, 11.5) as f64 <= libre {
                            entree
                        } else {
                            text.fit(&nombre(p.entry, dec), 11.5, libre.max(20.0))
                        };
                        text.draw(pm, &entree, (cols[0].0 as f32, l2 as f32), 11.5, j::DIM, Align::Right);
                        let cy = y + h / 2.0 + 5.0;
                        let pnl = dollars_signe(p.pnl);
                        gras.draw(pm, &pnl, (cols[1].0 as f32, cy as f32), 13.5, couleur_affichee(&pnl, p.pnl), Align::Right);
                        text.draw(pm, &liq, (cols[2].0 as f32, cy as f32), 13.5, orange, Align::Right);
                    }
                }
            }
            1 => {
                let Some(o) = &self.ordres else { return rien(pm, "Lecture des ordres…") };
                if o.is_empty() {
                    return rien(pm, "Aucun ordre ouvert");
                }
                for (i, o) in o.iter().enumerate() {
                    let y = y0 + i as f64 * h;
                    logo(pm, &o.coin, y + h / 2.0);
                    gras.draw(pm, &nom(&o.coin), ((PAD + 28.0) as f32, (y + 22.0) as f32), 14.0, j::TEXTE, Align::Left);
                    text.draw(pm, &o.kind, ((PAD + 28.0) as f32, (y + 40.0) as f32), 11.5, j::DIM, Align::Left);
                    let cy = (y + h / 2.0 + 5.0) as f32;
                    let (s, c) = if o.buy {
                        ("Achat", crate::palette::up())
                    } else {
                        ("Vente", crate::palette::down())
                    };
                    gras.draw(pm, s, (cols[0].0 as f32, cy), 13.5, c, Align::Right);
                    text.draw(pm, &nombre(o.px, self.dec(&o.coin, o.px)), (cols[1].0 as f32, cy), 13.5, j::TEXTE, Align::Right);
                    text.draw(
                        pm,
                        &nombre(o.size, if o.size >= 100.0 { 0 } else { 4 }),
                        (cols[2].0 as f32, cy),
                        13.5,
                        j::TEXTE,
                        Align::Right,
                    );
                }
            }
            2 => {
                let Some(f) = &self.fills else { return rien(pm, "Lecture des trades…") };
                if f.is_empty() {
                    return rien(pm, "Aucun trade récent");
                }
                for (i, x) in f.iter().enumerate() {
                    let y = y0 + i as f64 * h;
                    logo(pm, &x.coin, y + h / 2.0);
                    gras.draw(pm, &nom(&x.coin), ((PAD + 28.0) as f32, (y + 22.0) as f32), 14.0, j::TEXTE, Align::Left);
                    text.draw(pm, &quand(x.time_ms), ((PAD + 28.0) as f32, (y + 40.0) as f32), 11.5, j::DIM, Align::Left);
                    let cy = (y + h / 2.0 + 5.0) as f32;
                    let sens = match x.dir.as_str() {
                        "Open Long" => "Ouvre long",
                        "Open Short" => "Ouvre short",
                        "Close Long" => "Ferme long",
                        "Close Short" => "Ferme short",
                        _ if x.buy => "Achat",
                        _ => "Vente",
                    };
                    let c = if x.buy { crate::palette::up() } else { crate::palette::down() };
                    text.draw(pm, &text.fit(sens, 12.5, 90.0), (cols[0].0 as f32, cy - 7.0), 12.5, c, Align::Right);
                    text.draw(
                        pm,
                        &format!("× {}", nombre(x.size, if x.size >= 100.0 { 0 } else { 4 })),
                        (cols[0].0 as f32, cy + 10.0),
                        11.0,
                        j::DIM,
                        Align::Right,
                    );
                    text.draw(pm, &nombre(x.px, self.dec(&x.coin, x.px)), (cols[1].0 as f32, cy), 13.5, j::TEXTE, Align::Right);
                    let pnl = if x.closed_pnl == 0.0 {
                        "—".to_string()
                    } else {
                        dollars_signe(x.closed_pnl)
                    };
                    gras.draw(
                        pm,
                        &pnl,
                        (cols[2].0 as f32, cy),
                        13.5,
                        if x.closed_pnl == 0.0 { j::DIM } else { couleur(x.closed_pnl) },
                        Align::Right,
                    );
                }
            }
            _ => {
                let Some(f) = &self.funding else { return rien(pm, "Lecture du funding…") };
                if f.is_empty() {
                    return rien(pm, "Aucun funding sur 30 jours");
                }
                for (i, x) in f.iter().enumerate() {
                    let y = y0 + i as f64 * h;
                    logo(pm, &x.coin, y + h / 2.0);
                    gras.draw(pm, &nom(&x.coin), ((PAD + 28.0) as f32, (y + 22.0) as f32), 14.0, j::TEXTE, Align::Left);
                    text.draw(pm, &quand(x.time_ms), ((PAD + 28.0) as f32, (y + 40.0) as f32), 11.5, j::DIM, Align::Left);
                    let cy = (y + h / 2.0 + 5.0) as f32;
                    let taux = pourcent(x.rate, 4);
                    text.draw(pm, &taux, (cols[0].0 as f32, cy), 13.0, couleur_affichee(&taux, x.rate), Align::Right);
                    text.draw(
                        pm,
                        &nombre(x.szi.abs(), if x.szi.abs() >= 100.0 { 0 } else { 2 }),
                        (cols[1].0 as f32, cy),
                        13.5,
                        j::TEXTE,
                        Align::Right,
                    );
                    let m = if x.usdc.abs() < 1.0 {
                        format!("{} $", nombre(x.usdc, 2))
                    } else {
                        dollars(x.usdc)
                    };
                    let m = if x.usdc > 0.0 { format!("+{m}") } else { m };
                    gras.draw(pm, &m, (cols[2].0 as f32, cy), 13.5, couleur(x.usdc), Align::Right);
                }
            }
        }
    }
}

// ── « Voir sur la chart » ─────────────────────────────────────────────────────────────────────────────────────────

/// The lines of an address on the chart of one market: its entry and its liquidation, each with its glass pill.
#[derive(Debug, Clone, PartialEq)]
pub struct LignesAdresse {
    pub adresse: String,
    pub coin: String,
    pub long: bool,
    pub taille: f64,
    pub entree: f64,
    pub liq: Option<f64>,
    pub dec: usize,
}

/// A pill laid by [`lignes_draw`] (chart points), for the ✕ (it removes the lines).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PiluleLigne {
    pub rect: Rect,
    pub fermer: Rect,
}

impl LignesAdresse {
    /// The two labels: (price, text, colour).
    pub fn etiquettes(&self) -> Vec<(f64, String, Rgb)> {
        let a = court(&self.adresse);
        let sens = if self.long { "Long" } else { "Short" };
        let base = self.coin.split_once(':').map_or(self.coin.as_str(), |(_, n)| n);
        let mut v = vec![(
            self.entree,
            format!(
                "{a} · {sens} {} {base} · entrée {}",
                nombre(self.taille.abs(), if self.taille.abs() >= 100.0 { 0 } else { 2 }),
                nombre(self.entree, self.dec)
            ),
            j::ACCENT,
        )];
        if let Some(l) = self.liq {
            v.push((l, format!("{a} · liquidation {}", nombre(l, self.dec)), Rgb(0xf5, 0x9e, 0x0b)));
        }
        v
    }
}

/// Draws the lines on the chart (`pw` × `ph` points, `y_of` price → y): a dashed line at each price and its pill; a
/// price out of the screen: its pill at the edge with ↑ / ↓, no line. The pills stay under `haut` (the bottom of the
/// title and the legend: never over them) and clear of `eviter`: at their line's height first (left, middle, right of
/// the plot), else the nearest free height (down first). Two pills never overlap; a pill with no free place left is
/// not drawn. Returns the pills (their ✕ removes the lines).
#[allow(clippy::too_many_arguments)]
pub fn lignes_draw(
    pm: &mut Pixmap,
    text: &Text,
    l: &LignesAdresse,
    pw: f64,
    ph: f64,
    haut: f64,
    y_of: &dyn Fn(f64) -> f64,
    eviter: &[Rect],
) -> Vec<PiluleLigne> {
    let gras = Text::graisse(Graisse::Bold);
    let mut out: Vec<PiluleLigne> = vec![];
    // (the pills' band: under the title and the legend, above the bottom)
    let haut = haut.max(0.0).min((ph - 24.0).max(0.0));
    for (prix, label, c) in l.etiquettes() {
        let y = y_of(prix);
        let (dedans, label) = if y < 14.0 {
            (false, format!("↑ {label}"))
        } else if y > ph - 14.0 {
            (false, format!("↓ {label}"))
        } else {
            (true, label)
        };
        if dedans {
            let mut x = 0.0;
            while x < pw {
                crate::fill(pm, x, y.round(), 6.0_f64.min(pw - x), 1.0, c, 0.85);
                x += 10.0;
            }
        }
        let w = gras.width(&label, 12.0) as f64 + 44.0;
        let ww = w.min(pw - 20.0);
        // (nothing overlaps: the other pill, the title and the legend, the walls' pills, the row of ⚡ and flags…)
        let touche = |r: &Rect| {
            r.y < haut
                || r.y + r.h > ph
                || out.iter().any(|o| r.y < o.rect.y + o.rect.h + 3.0 && o.rect.y < r.y + r.h + 3.0)
                || eviter.iter().any(|e| r.x < e.x + e.w && e.x < r.x + r.w && r.y < e.y + e.h && e.y < r.y + r.h)
        };
        let y0 = (y.clamp(haut + 12.0, (ph - 12.0).max(haut + 12.0)) - 12.0).round();
        let xs = [10.0, ((pw - ww) / 2.0).round(), (pw - ww - 10.0).max(10.0)];
        // at its line's height first, then 6 points further each time: down, then up
        let mut place = None;
        let mut d = 0.0;
        while place.is_none() && (y0 + d + 24.0 <= ph || y0 - d >= haut) {
            let essais: &[f64] = if d == 0.0 { &[y0] } else { &[y0 + d, y0 - d] };
            place = essais
                .iter()
                .flat_map(|&yy| xs.iter().map(move |&x| Rect::new(x, yy, ww, 24.0)))
                .find(|r| !touche(r));
            d += 6.0;
        }
        let Some(r) = place else {
            continue;
        };
        round_fill(pm, r, 12.0, Rgb(0x12, 0x15, 0x22), 0.92);
        round_stroke(pm, r, 12.0, c, 0.9);
        gras.draw(
            pm,
            &gras.fit(&label, 12.0, r.w - 40.0),
            ((r.x + 10.0) as f32, (r.cy() + 4.5) as f32),
            12.0,
            c,
            Align::Left,
        );
        let f = Rect::new(r.x + r.w - 24.0, r.y + 3.0, 18.0, 18.0);
        text.draw(pm, "✕", (f.cx() as f32, (f.cy() + 4.5) as f32), 11.5, j::DIM, Align::Center);
        out.push(PiluleLigne { rect: r, fermer: f });
    }
    out
}

thread_local! {
    /// 📈 The lines to draw and the chart they belong to (the address of its candles), set by the app.
    static VUE: std::cell::RefCell<Option<(LignesAdresse, usize)>> = const { std::cell::RefCell::new(None) };
    /// 📈 The pills of the last drawing (chart points).
    static PILULES: std::cell::RefCell<Vec<PiluleLigne>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// 📈 The app gives the lines of « Voir sur la chart » for the chart whose candles start at `cle` (`None`: none).
pub fn set_lignes(l: Option<LignesAdresse>, cle: usize) {
    let l = l.map(|l| (l, cle));
    // (n° 16: the kept background of the chart did not know them: they came, and left at the ✕, with the next
    // second's image, up to 1 s late)
    if VUE.with(|v| *v.borrow() != l) {
        crate::px::fond_change();
    }
    VUE.with(|v| *v.borrow_mut() = l);
    if VUE.with(|v| v.borrow().is_none()) {
        PILULES.with(|p| p.borrow_mut().clear());
    }
}

/// The lines for the chart whose candles start at `cle`.
pub(crate) fn lignes_pour(cle: usize) -> Option<LignesAdresse> {
    VUE.with(|v| v.borrow().as_ref().filter(|(_, k)| *k == cle).map(|(l, _)| l.clone()))
}

pub(crate) fn set_pilules(p: Vec<PiluleLigne>) {
    PILULES.with(|v| *v.borrow_mut() = p);
}

/// 📈 (`x`, `y`, chart points) is on the ✕ of a line's pill (drawn last time).
pub fn fermer_sous(x: f64, y: f64) -> bool {
    PILULES.with(|v| {
        v.borrow()
            .iter()
            .any(|p| Rect::new(p.fermer.x - 4.0, p.fermer.y - 4.0, p.fermer.w + 8.0, p.fermer.h + 8.0).contains(x, y))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use argos_data::explorer::Position;

    fn compte() -> Compte {
        let p = |c: &str, s: f64, e: f64, v: f64| Position {
            coin: c.into(),
            size: s,
            entry: e,
            price: e,
            value: v,
            pnl: -12.5,
            leverage: 5.0,
            liq: Some(e * 0.9),
        };
        Compte {
            value: 1000.0,
            ntl: 4600.0,
            margin_used: 0.0,
            positions: vec![p("BTC", 2.4, 59_820.0, 3000.0), p("xyz:NVDA", -3.0, 180.25, 600.0)],
        }
    }

    #[test]
    fn nombres_en_francais() {
        assert_eq!(nombre(59_820.0, 0), "59 820");
        assert_eq!(nombre(-2088.5, 1), "−2 088,5");
        assert_eq!(nombre(-0.004, 2), "0,00", "no « −0 »");
        assert_eq!(dollars_signe(3120.0), "+3 120 $");
        assert_eq!(dollars_signe(-8940.0), "−8 940 $");
        assert_eq!(court_dollars(1.42e9, 2), "1,42 Md$");
        assert_eq!(court_dollars(612e6, 2), "612 M$");
        assert_eq!(court_dollars(7.9e9, 1), "7,9 Md$");
        assert_eq!(court_dollars(41_000.0, 2), "41 k$");
        assert_eq!(pourcent(0.0001, 4), "+0,0100 %");
        assert_eq!(pourcent(-0.000042, 4), "−0,0042 %");
        // (SOL « 0,0 % » in red, capture 202611222137-15): sign and colour from the value written
        assert_eq!(pourcent(-0.0004, 1), "0,0 %");
        assert_eq!(couleur_affichee(&pourcent(-0.0004, 1), -0.0004), j::TEXTE);
        assert_eq!(couleur_affichee(&pourcent(0.0004, 1), 0.0004), j::TEXTE);
        assert_eq!(couleur_affichee(&pourcent(-0.0006, 1), -0.0006), crate::palette::down());
        assert_eq!(couleur_affichee(&dollars_signe(-0.4), -0.4), j::TEXTE);
        assert_eq!(couleur_affichee(&dollars_signe(2.0), 2.0), crate::palette::up());
        assert_eq!(decimales_de("59820.5"), 1);
        assert_eq!(decimales_de("33.400"), 1);
    }

    /// 🧭 An invalid address gives a message, a valid one (typed or pasted whole) is read; a click on a position
    /// opens its chart; ⭐ and « Voir sur la chart » ask the app.
    #[test]
    fn adresse_et_clics() {
        let t = Text::default();
        let pw = 402.0;
        let mut e = Explorer::default();
        let zone = |e: &Explorer, z: &Zone| e.zones(&t, pw).into_iter().find(|(_, k)| k == z).unwrap().0;
        let c = zone(&e, &Zone::Champ);
        assert_eq!(e.click(&t, pw, c.cx(), c.cy()), ExClick::Changed);
        for ch in "0x12zz".chars() {
            e.taper(Some(ch));
        }
        assert_eq!(e.valider(), ExClick::Changed);
        assert!(e.note.contains("invalide"));
        e.saisie = Some(String::new());
        let a = "0x7A3F00000000000000000000000000000000C91E";
        let mut dernier = ExClick::None;
        for ch in a.chars() {
            dernier = e.taper(Some(ch));
        }
        let a = a.to_lowercase();
        assert_eq!(dernier, ExClick::Lire(a.clone()), "read as soon as it is whole");
        assert_eq!((e.adresse.as_deref(), e.saisie.is_none(), e.note.is_empty()), (Some(a.as_str()), true, true));
        e.compte = Some(compte());
        e.compte_ms = Some(1_000);
        let s = zone(&e, &Zone::Suivre);
        assert_eq!(e.click(&t, pw, s.cx(), s.cy()), ExClick::Suivre(a.clone()));
        let v = zone(&e, &Zone::Voir);
        assert_eq!(e.click(&t, pw, v.cx(), v.cy()), ExClick::Voir(a.clone(), "BTC".into()));
        let m = zone(&e, &Zone::Marche("xyz:NVDA".into()));
        assert_eq!(e.click(&t, pw, m.cx(), m.cy()), ExClick::Chart("xyz:NVDA".into()));
        let o = zone(&e, &Zone::Onglet(2));
        assert_eq!(e.click(&t, pw, o.cx(), o.cy()), ExClick::Onglet(2));
        // 📱 a 7-digit liquidation price (capture 202611222137-14): the columns never touch
        let mut grande = compte();
        grande.positions[0].liq = Some(2_570_530.0);
        grande.positions[0].pnl = -12_345.0;
        e.compte = Some(grande.clone());
        e.onglet = 0;
        for pw in [320.0, 402.0] {
            let c = e.colonnes(&t, pw);
            let liq = grande
                .positions
                .iter()
                .filter_map(|p| p.liq.map(|l| t.width(&nombre(l, e.dec(&p.coin, p.entry)), 13.5) as f64))
                .fold(0.0, f64::max);
            assert!(c[1].0 + 14.0 <= c[2].0 - liq, "{pw}: {c:?}");
            let pnl = Text::graisse(Graisse::Bold).width(&dollars_signe(-12_345.0), 13.5) as f64;
            assert!(c[0].0 + 14.0 <= c[1].0 - pnl, "{pw}: {c:?}");
        }
        e.compte = Some(compte());
        // drawn at each width and tab, ⚙ too
        for pw in [320.0, 402.0, 640.0] {
            for k in 0..4 {
                e.onglet = k;
                let mut pm = crate::px::new(pw as u32, e.content_h(&t, pw) as u32).unwrap();
                e.draw(&mut pm, &t, pw, None, Rgb(0, 0, 0));
            }
        }
        e.suivies = vec![a.clone()];
        let g = zone(&e, &Zone::Reglages);
        e.click(&t, pw, g.cx(), g.cy());
        assert!(e.reglages);
        let r = zone(&e, &Zone::Retirer(a.clone()));
        e.click(&t, pw, r.cx(), r.cy());
        assert!(e.suivies.is_empty());
        let i = zone(&e, &Zone::Intervalle(2));
        e.click(&t, pw, i.cx(), i.cy());
        assert_eq!(e.intervalle_s(), 60);
    }

    /// 🧭 N° 13: « ⚠️ Compte illisible : … » under the field moves the account read down (it was written over the
    /// address); n° 14: in the narrow column of modules (no ⬓ ⤢ ✕), ⚙ stays after the title.
    #[test]
    fn note_et_reglages_sans_chevauchement() {
        let t = Text::default();
        let mut e = Explorer::default();
        let haut = e.haut_compte();
        e.note = "⚠️ Compte illisible : HTTP 500".into();
        assert!(e.haut_compte() >= haut + 18.0, "the account starts under the note");
        let fin = PAD + Text::graisse(Graisse::Bold).width("🧭 Explorer Hyperliquid", 17.0) as f64;
        for pw in [300.0, 360.0, 420.0] {
            let g = e.zones(&t, pw).into_iter().find(|(_, z)| *z == Zone::Reglages).unwrap().0;
            assert!(g.x >= fin && g.x + g.w <= pw, "{pw}: ⚙ {g:?} after the title ({fin})");
        }
    }

    /// 📈 The lines on the chart: entry and liquidation with their pills; a price off the screen: a pill at the edge
    /// with ↓, no line; the pills never overlap.
    #[test]
    fn lignes_sur_la_chart() {
        let l = LignesAdresse {
            adresse: "0x7a3f00000000000000000000000000000000c91e".into(),
            coin: "BTC".into(),
            long: true,
            taille: 2.4,
            entree: 59_820.0,
            liq: Some(54_310.0),
            dec: 0,
        };
        // n° 16: the kept background of the chart knows when they come and when they go (they waited up to 1 s)
        let r0 = crate::px::fond_revision();
        set_lignes(Some(l.clone()), 7);
        let r1 = crate::px::fond_revision();
        set_lignes(Some(l.clone()), 7);
        assert!(r1 != r0 && crate::px::fond_revision() == r1, "they come: once");
        set_lignes(None, 7);
        assert!(crate::px::fond_revision() != r1, "they go at the ✕");
        let e = l.etiquettes();
        assert_eq!(e[0].1, "0x7a3f…c91e · Long 2,40 BTC · entrée 59 820");
        assert_eq!(e[1].1, "0x7a3f…c91e · liquidation 54 310");
        let t = Text::default();
        let mut pm = crate::px::new(800, 400).unwrap();
        // 60 000 at y 100, 1 point per 10 $: the liquidation (y 667) is under the screen
        let y_of = |p: f64| 100.0 + (60_000.0 - p) / 10.0;
        let p = lignes_draw(&mut pm, &t, &l, 800.0, 400.0, 0.0, &y_of, &[]);
        assert_eq!(p.len(), 2);
        assert!((p[0].rect.cy() - y_of(59_820.0)).abs() < 1.0);
        assert!(p[1].rect.y + p[1].rect.h <= 400.0, "at the bottom edge");
        // both near the same price: the second moves down
        let y_of = |p: f64| 200.0 + (60_000.0 - p) / 1e6;
        let p = lignes_draw(&mut pm, &t, &l, 800.0, 400.0, 0.0, &y_of, &[]);
        assert!(p[1].rect.y >= p[0].rect.y + p[0].rect.h);
        // a place to keep clear (the legend): the pill leaves it
        let legende = Rect::new(0.0, 150.0, 300.0, 100.0);
        let p = lignes_draw(&mut pm, &t, &l, 800.0, 400.0, 0.0, &y_of, &[legende]);
        let libre = |r: &Rect| !(r.x < legende.x + legende.w && legende.x < r.x + r.w && r.y < legende.y + legende.h && legende.y < r.y + r.h);
        assert!(p.iter().all(|p| libre(&p.rect)), "{p:?}");
        assert!((p[0].rect.cy() - 200.0).abs() < 1.0, "at its line, moved sideways: {p:?}");
        // the whole width taken at that height: down
        let bande = Rect::new(0.0, 150.0, 800.0, 100.0);
        let p = lignes_draw(&mut pm, &t, &l, 800.0, 400.0, 0.0, &y_of, &[bande]);
        assert!(p.iter().all(|p| p.rect.y >= 250.0 || p.rect.y + p.rect.h <= 150.0), "{p:?}");
        set_pilules(p.clone());
        assert!(fermer_sous(p[0].fermer.cx(), p[0].fermer.cy()));
        assert!(!fermer_sous(p[0].rect.x + 4.0, p[0].rect.cy()));
        // a loaded chart (capture 202611222137-12): the liquidation above the screen, the title and the legend on
        // top (haut 110), a table on the left and the walls' pills on the right at every height: the pills stay under
        // the legend, between them, and touch nothing
        let l = LignesAdresse { liq: Some(226_745.0), ..l };
        let y_of = |p: f64| 120.0 + (60_969.0 - p) / 10.0;
        let tableau = Rect::new(0.0, 110.0, 300.0, 290.0);
        let murs = Rect::new(1000.0, 0.0, 400.0, 400.0);
        let mut pm = crate::px::new(1400, 400).unwrap();
        let p = lignes_draw(&mut pm, &t, &l, 1400.0, 400.0, 110.0, &y_of, &[tableau, murs]);
        assert_eq!(p.len(), 2, "{p:?}");
        let libre = |r: &Rect| {
            [tableau, murs]
                .iter()
                .all(|e| !(r.x < e.x + e.w && e.x < r.x + r.w && r.y < e.y + e.h && e.y < r.y + r.h))
        };
        assert!(p.iter().all(|p| p.rect.y >= 110.0 && libre(&p.rect)), "{p:?}");
        assert!(p[1].rect.y >= p[0].rect.y + p[0].rect.h || p[0].rect.y >= p[1].rect.y + p[1].rect.h, "{p:?}");
        // no free place at all: no pill over the rest
        let plein = Rect::new(0.0, 0.0, 1400.0, 400.0);
        assert!(lignes_draw(&mut pm, &t, &l, 1400.0, 400.0, 110.0, &y_of, &[plein]).is_empty());
    }
}
