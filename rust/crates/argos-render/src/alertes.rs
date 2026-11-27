//! 🔔 The Alertes panel (Jee's mock-up « alertes v1 », 22 Nov 2026): one glass panel, on the PC the volet beside the
//! right column, on the phone a bottom sheet with its handle. Header « 🔔 Alertes » with three rounds + (new alert),
//! ⚙ (every alert setting: sound, colour, alert lines…) and ✕; the field « 🔎 Chercher une alerte » (symbol or
//! exchange); the pill tabs Actives | Historique.
//! - Actives: the coin's round, « BTC/USDT » (base in bold), « Binance Perp · 1h · 22 nov. » in grey; on the right
//!   the condition in colour, the price aimed at in bold with ↗ ↘ ↕, the frequency. A paused alert: « ⏸ En pause »,
//!   the whole row pale. A click: the alert line's menu of before (pause, Éditer…, Son…, Effacer). Footer « Actives n
//!   · En pause n » and « Total n ».
//! - Historique (newest first): the price aimed at, the condition, « Déclenchée à X » (the true price of the moment),
//!   the firing's date in the grey line; mouse over a row (PC) or a touch (phone) lights it and shows « Trade ».
//!   « Vider l'historique » beside the field (confirmed by the host). Footer « Aujourd'hui n · Cette semaine n »
//!   (New York, the week from Monday) and « Total n ».
//!
//! The host gives the rows ([`Donnees`]) and does what a click asks ([`AClic`]).

use crate::jetons as j;
use crate::shapes::{round_fill, round_stroke, Rect};
use crate::text::{Align, Graisse, Rgb, Text};
use argos_core::alerts::Sens;
use chrono::{Datelike, TimeZone};
use tiny_skia::Pixmap;

const PAD: f64 = 16.0;
/// The round buttons of the header (diameter) and their step.
const ROND: f64 = 30.0;
const ROND_PAS: f64 = 38.0;
const CHAMP_Y: f64 = 56.0;
const CHAMP_H: f64 = 36.0;
const ONGLETS_Y: f64 = 102.0;
const ONGLETS_H: f64 = 40.0;
/// Top of the list (points from the panel's top).
const LISTE_Y: f64 = 150.0;
const LIGNE_H: f64 = 66.0;
/// A lit history row is taller: its « Trade » button under the prices.
const LIGNE_TRADE: f64 = 30.0;
const PIED: f64 = 42.0;
/// The phone sheet's handle: the header goes down by this much.
const POIGNEE: f64 = 14.0;

/// One row: an alert (Actives) or a firing (Historique).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Ligne {
    /// The alert's id (Actives) or the firing's index in the history (Historique).
    pub cle: u64,
    /// The market (« binance:BTCUSDT »; « Trade » opens it), its base and quote.
    pub coin: String,
    pub base: String,
    pub quote: String,
    /// « Binance Perp » (empty when unknown: an alert saved before the panel).
    pub bourse: String,
    /// « 1h » (empty when unknown).
    pub ut: String,
    /// When it was made (Actives) or when it fired (Historique), seconds UTC.
    pub ts: Option<i64>,
    pub sens: Sens,
    /// The arrow by the price when the sense is « n conditions »: the sense of the condition the price comes from
    /// (`None`: ↕).
    pub fleche: Option<Sens>,
    /// The price aimed at.
    pub prix: Option<f64>,
    /// « Une fois » / « Chaque fois ».
    pub frequence: String,
    pub pause: bool,
    /// Why it is off, said in its row (« En pause », « Déclenchée », « Expirée »; empty: « En pause »).
    pub arret: String,
    /// Historique: the true price when it fired.
    pub declenche: Option<f64>,
}

/// The rows of both tabs (the history newest first).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Donnees {
    pub actives: Vec<Ligne>,
    pub historique: Vec<Ligne>,
}

/// What a click asks the host.
#[derive(Debug, Clone, PartialEq)]
pub enum AClic {
    None,
    Changed,
    Fermer,
    /// + : a new alert (`Rect`: the button, window points, for the menu).
    Nouvelle(Rect),
    /// ⚙ : every alert setting.
    Reglages(Rect),
    /// An active alert: its menu (its id, its row).
    Active(u64, Rect),
    /// « Vider l'historique » (the host asks to confirm).
    Vider,
    /// « Trade » on a history row (its index): that market on the chart and the demo ticket.
    Trade(u64),
}

/// The places of the panel.
#[derive(Debug, Clone, PartialEq)]
pub enum AZone {
    Nouvelle,
    Reglages,
    Fermer,
    Champ,
    Vider,
    Onglet(usize),
    Ligne(u64),
    Trade(u64),
}

/// The panel's state (kept while it is open).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Etat {
    /// 0: Actives, 1: Historique.
    pub onglet: usize,
    /// `Some`: the field takes the keyboard.
    pub recherche: Option<String>,
    pub defile: f64,
    /// The history row lit by a touch or a click (its index).
    pub touche: Option<u64>,
    /// 📱 The phone sheet (a handle on top), not the PC volet.
    pub feuille: bool,
    /// 🖥 The history row the mouse lit (kept while the mouse is in it, « Trade » included).
    pub survol: std::cell::Cell<Option<u64>>,
    /// Where the panel was last drawn (window points): clicks and the wheel use the very same place.
    pub dessine: std::cell::Cell<Option<Rect>>,
}

/// « 61 500,0 », « 1 980,00 », « 31,250 », « 1,2488 »: the decimals by the price's size, as the mock-up writes them.
pub fn prix(p: f64) -> String {
    let a = p.abs();
    let dec = if a >= 10_000.0 {
        1
    } else if a >= 100.0 {
        2
    } else if a >= 10.0 {
        3
    } else if a >= 1.0 {
        4
    } else {
        6
    };
    crate::hlexplorer::nombre(p, dec)
}

fn ny(ts: i64) -> chrono::DateTime<chrono_tz::Tz> {
    chrono_tz::America::New_York
        .timestamp_opt(ts, 0)
        .single()
        .unwrap_or_else(|| chrono_tz::America::New_York.timestamp_opt(0, 0).unwrap())
}

const MOIS: [&str; 12] = [
    "janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.", "oct.", "nov.", "déc.",
];

/// « aujourd'hui » (the same New York day as `now`), else « 21 nov. ».
pub fn quand(ts: i64, now: i64) -> String {
    let (d, n) = (ny(ts), ny(now));
    if d.date_naive() == n.date_naive() {
        "aujourd'hui".into()
    } else {
        format!("{} {}", d.day(), MOIS[d.month0() as usize])
    }
}

/// Historique's footer: (today, this week from Monday 00:00, total), New York time.
pub fn compte_historique(h: &[Ligne], now: i64) -> (usize, usize, usize) {
    let n = ny(now).date_naive();
    let lundi = n - chrono::Days::new(n.weekday().num_days_from_monday() as u64);
    let jour = |l: &Ligne| l.ts.map(|t| ny(t).date_naive());
    let auj = h.iter().filter(|l| jour(l) == Some(n)).count();
    let sem = h.iter().filter(|l| jour(l).is_some_and(|d| d >= lundi && d <= n)).count();
    (auj, sem, h.len())
}

/// Actives' footer: (on, paused, total).
pub fn compte_actives(a: &[Ligne]) -> (usize, usize, usize) {
    let p = a.iter().filter(|l| l.pause).count();
    (a.len() - p, p, a.len())
}

/// The condition's words and colour.
fn sens_texte(s: Sens) -> (String, Rgb) {
    match s {
        Sens::Up => ("Au-dessus".into(), crate::palette::up()),
        Sens::Down => ("En dessous".into(), crate::palette::down()),
        Sens::Cross => ("Croisement".into(), j::ACCENT),
        Sens::Conds(n) => (format!("{n} conditions"), j::ACCENT),
    }
}

/// The arrow of a row's price: its sense, or for « n conditions » the sense of the price's own condition (↕ when
/// unknown; ↕ stays for « Croisement »).
fn fleche_de(l: &Ligne) -> Sens {
    match l.sens {
        Sens::Conds(_) => l.fleche.unwrap_or(Sens::Cross),
        s => s,
    }
}

/// ↗ ↘ ↕ drawn with lines (the font has no arrows), its centre at (`cx`, `cy`).
fn fleche(pm: &mut Pixmap, s: Sens, cx: f64, cy: f64, c: Rgb) {
    match s {
        Sens::Up => crate::shapes::lines(pm, cx, cy, &[&[(-4.0, 4.0), (4.0, -4.0)], &[(-1.0, -4.0), (4.0, -4.0), (4.0, 1.0)]], c, 1.6),
        Sens::Down => crate::shapes::lines(pm, cx, cy, &[&[(-4.0, -4.0), (4.0, 4.0)], &[(4.0, -1.0), (4.0, 4.0), (-1.0, 4.0)]], c, 1.6),
        _ => crate::shapes::lines(
            pm,
            cx,
            cy,
            &[
                &[(0.0, -6.0), (0.0, 6.0)],
                &[(-3.5, -2.5), (0.0, -6.0), (3.5, -2.5)],
                &[(-3.5, 2.5), (0.0, 6.0), (3.5, 2.5)],
            ],
            c,
            1.6,
        ),
    }
}

/// 🔎 A name as the search compares it: no case, no accent, no « / - _ . : » nor spaces, and a raw symbol's « .P » /
/// « PERP » ending dropped (« BTCUSDC.P » → « btcusdc », « Sol Usdt » → « solusdt »).
fn net(s: &str) -> String {
    let p = crate::i18n::plain(s.trim());
    let p = p.strip_suffix(".p").unwrap_or(&p);
    let p = p.strip_suffix("perp").unwrap_or(p);
    p.chars().filter(|c| !matches!(c, '/' | '-' | '_' | '.' | ':') && !c.is_whitespace()).collect()
}

impl Etat {
    /// The rows of the open tab kept by the search, case and separators aside (« solusdt », « SOL/USDT »,
    /// « sol-usdt », « Sol Usdt »): in base + quote, in the raw symbol, or in the exchange's name.
    pub fn montrees<'a>(&self, d: &'a Donnees) -> Vec<&'a Ligne> {
        let q = self.recherche.as_deref().map(net).unwrap_or_default();
        let l = if self.onglet == 1 { &d.historique } else { &d.actives };
        l.iter()
            .filter(|l| {
                let brut = l.coin.rsplit_once(':').map_or(l.coin.as_str(), |(_, s)| s);
                q.is_empty()
                    || [format!("{}{}", l.base, l.quote), brut.to_string(), l.bourse.clone()]
                        .iter()
                        .any(|s| net(s).contains(&q))
            })
            .collect()
    }

    fn haut(&self) -> f64 {
        if self.feuille {
            POIGNEE
        } else {
            0.0
        }
    }

    /// The bottom band kept free (📱 the round Argos AI button over the sheet's bottom right).
    fn marge_bas(&self) -> f64 {
        if self.feuille {
            crate::eco::marge_ai()
        } else {
            0.0
        }
    }

    /// The list's visible band (window points): its top and bottom.
    fn bande(&self, r: Rect) -> (f64, f64) {
        (r.y + self.haut() + LISTE_Y, r.y + r.h - PIED - self.marge_bas())
    }

    /// The history row lit: the touched one, else (PC) the one under the mouse.
    fn allumee(&self, r: Rect, d: &Donnees, mouse: Option<(f64, f64)>) -> Option<u64> {
        if self.onglet != 1 {
            return None;
        }
        if self.touche.is_some() {
            return self.touche;
        }
        let Some((x, y)) = mouse.filter(|_| !crate::actions::tactile()) else {
            self.survol.set(None);
            return None;
        };
        let (haut, bas) = self.bande(r);
        if !r.contains(x, y) || y < haut || y > bas {
            self.survol.set(None);
            return None;
        }
        let montrees = self.montrees(d);
        // (the row already lit keeps the mouse while it is in it, its « Trade » band included: the mouse going down
        // to « Trade » must not light the next row)
        if let Some(p) = self.survol.get() {
            if let Some(k) = montrees.iter().position(|l| l.cle == p) {
                let y0 = haut - self.defile + k as f64 * LIGNE_H;
                if y >= y0 && y < y0 + LIGNE_H + LIGNE_TRADE {
                    return Some(p);
                }
            }
        }
        let k = ((y - haut + self.defile) / LIGNE_H).floor();
        let lit = (k >= 0.0).then(|| montrees.get(k as usize).map(|l| l.cle)).flatten();
        self.survol.set(lit);
        lit
    }

    /// The rows' places (window points, scrolled) and the lit one.
    fn lignes(&self, r: Rect, d: &Donnees, mouse: Option<(f64, f64)>) -> (Vec<(Rect, u64)>, Option<u64>) {
        let lit = self.allumee(r, d, mouse);
        let mut y = self.bande(r).0 - self.defile;
        let mut out = vec![];
        for l in self.montrees(d) {
            let h = LIGNE_H + if lit == Some(l.cle) { LIGNE_TRADE } else { 0.0 };
            out.push((Rect::new(r.x + 6.0, y, r.w - 12.0, h), l.cle));
            y += h;
        }
        (out, lit)
    }

    /// « Trade » in the lit row.
    fn trade_rect(q: Rect) -> Rect {
        Rect::new(q.x + q.w - 10.0 - 62.0, q.y + q.h - 34.0, 62.0, 26.0)
    }

    /// Every place of the panel `r` (window points).
    pub fn zones(&self, text: &Text, r: Rect, d: &Donnees, mouse: Option<(f64, f64)>) -> Vec<(Rect, AZone)> {
        let y0 = r.y + self.haut();
        let bx = r.x + r.w - PAD - ROND;
        let by = y0 + 12.0;
        let mut z = vec![
            (Rect::new(bx - 2.0 * ROND_PAS, by, ROND, ROND), AZone::Nouvelle),
            (Rect::new(bx - ROND_PAS, by, ROND, ROND), AZone::Reglages),
            (Rect::new(bx, by, ROND, ROND), AZone::Fermer),
        ];
        let vider_w = if self.onglet == 1 {
            text.width("Vider l'historique", 14.0) as f64 + 14.0
        } else {
            0.0
        };
        z.push((Rect::new(r.x + PAD, y0 + CHAMP_Y, r.w - 2.0 * PAD - vider_w, CHAMP_H), AZone::Champ));
        if vider_w > 0.0 {
            z.push((Rect::new(r.x + r.w - PAD - vider_w + 8.0, y0 + CHAMP_Y, vider_w - 8.0, CHAMP_H), AZone::Vider));
        }
        let ow = (r.w - 2.0 * PAD - 8.0) / 2.0;
        for k in 0..2 {
            z.push((
                Rect::new(r.x + PAD + 4.0 + k as f64 * ow, y0 + ONGLETS_Y + 4.0, ow, ONGLETS_H - 8.0),
                AZone::Onglet(k),
            ));
        }
        let (haut, bas) = self.bande(r);
        let (lignes, lit) = self.lignes(r, d, mouse);
        for (q, cle) in lignes {
            // (only the part in view answers)
            if q.y + q.h <= haut || q.y >= bas {
                continue;
            }
            if lit == Some(cle) {
                let t = Self::trade_rect(q);
                if t.y >= haut && t.y + t.h <= bas {
                    z.push((t, AZone::Trade(cle)));
                }
            }
            let y = q.y.max(haut);
            z.push((Rect::new(q.x, y, q.w, q.y + q.h - y - (q.y + q.h - bas).max(0.0)), AZone::Ligne(cle)));
        }
        z
    }

    /// The list's full height (it scrolls).
    fn contenu_h(&self, d: &Donnees) -> f64 {
        let n = self.montrees(d).len() as f64;
        n * LIGNE_H + if self.onglet == 1 { LIGNE_TRADE } else { 0.0 } + 12.0
    }

    /// Wheel or finger (`dy` > 0: down).
    pub fn wheel(&mut self, dy: f64, r: Rect, d: &Donnees) {
        let (haut, bas) = self.bande(r);
        let max = (self.contenu_h(d) - (bas - haut)).max(0.0);
        self.defile = (self.defile + dy).clamp(0.0, max);
    }

    /// A click at (`x`, `y`) in the panel `r` (window points).
    pub fn click(&mut self, text: &Text, r: Rect, d: &Donnees, x: f64, y: f64) -> AClic {
        if !r.contains(x, y) {
            return AClic::Fermer;
        }
        let zones = self.zones(text, r, d, Some((x, y)));
        // (« Trade » lies in its row: it wins)
        let z = zones
            .iter()
            .find(|(q, z)| matches!(z, AZone::Trade(_)) && q.contains(x, y))
            .or_else(|| zones.iter().find(|(q, _)| q.contains(x, y)))
            .cloned();
        if !matches!(z, Some((_, AZone::Champ))) {
            self.recherche = self.recherche.take().filter(|q| !q.is_empty());
        }
        let Some((q, z)) = z else { return AClic::None };
        match z {
            AZone::Nouvelle => AClic::Nouvelle(q),
            AZone::Reglages => AClic::Reglages(q),
            AZone::Fermer => AClic::Fermer,
            AZone::Champ => {
                self.recherche.get_or_insert_with(String::new);
                AClic::Changed
            }
            AZone::Vider => AClic::Vider,
            AZone::Onglet(k) => {
                if self.onglet != k {
                    (self.onglet, self.defile, self.touche) = (k, 0.0, None);
                }
                AClic::Changed
            }
            AZone::Ligne(cle) if self.onglet == 0 => AClic::Active(cle, q),
            AZone::Ligne(cle) => {
                self.touche = Some(cle);
                AClic::Changed
            }
            AZone::Trade(cle) => AClic::Trade(cle),
        }
    }

    /// Keyboard in the field (`None`: backspace).
    pub fn taper(&mut self, c: Option<char>) {
        if let Some(q) = self.recherche.as_mut() {
            match c {
                Some(c) if !c.is_control() => q.push(c),
                None => {
                    q.pop();
                }
                _ => {}
            }
            (self.defile, self.touche) = (0.0, None);
        }
    }

    /// One row in `pm` at `q` (points of `pm`); `lit`: « Trade » shown.
    fn ligne_draw(&self, pm: &mut Pixmap, text: &Text, logo: &dyn Fn(&str) -> Option<Pixmap>, q: Rect, l: &Ligne, now: i64, lit: bool) {
        let gras = Text::graisse(Graisse::Bold);
        let histo = self.onglet == 1;
        let cy = q.y + LIGNE_H / 2.0;
        if lit {
            round_fill(pm, q, j::R_LIGNE + 4.0, j::BLANC, 0.06);
        }
        let lg = logo(&l.coin).or_else(|| logo(&l.base));
        crate::watchlist::coin_badge(pm, text, &l.base, lg.as_ref(), q.x + PAD - 6.0 + 16.0, cy, 16.0);
        let tx = q.x + PAD - 6.0 + 42.0;
        // the right column first: the name is cut to what is left
        let droite = q.x + q.w - PAD + 6.0;
        let (mot, couleur) = sens_texte(l.sens);
        let arret = if l.arret.is_empty() { "En pause".to_string() } else { l.arret.clone() };
        let (mot, couleur) = if l.pause { (arret, j::DIM) } else { (mot, couleur) };
        let p = l.prix.map(prix).unwrap_or_else(|| "—".into());
        let pw = gras.width(&p, 15.0) as f64;
        let mut col_w = pw + 16.0;
        if histo {
            let y_px = cy - 12.0;
            gras.draw(pm, &p, (droite as f32, (y_px + 5.0) as f32), 15.0, j::TEXTE, Align::Right);
            fleche(pm, fleche_de(l), droite - pw - 9.0, y_px, j::TEXTE);
            text.draw(pm, &mot, (droite as f32, (cy + 8.0) as f32), 12.5, couleur, Align::Right);
            let dec = l.declenche.map(|d| format!("Déclenchée à {}", prix(d))).unwrap_or_default();
            text.draw(pm, &dec, (droite as f32, (cy + 24.0) as f32), 12.0, j::DIM, Align::Right);
            col_w = col_w.max(text.width(&dec, 12.0) as f64);
        } else {
            if l.pause {
                // ⏸ two short bars, then « En pause »
                let mw = text.width(&mot, 12.5) as f64;
                crate::fill(pm, droite - mw - 9.0, cy - 25.0, 1.8, 8.0, j::DIM, 1.0);
                crate::fill(pm, droite - mw - 5.5, cy - 25.0, 1.8, 8.0, j::DIM, 1.0);
            }
            text.draw(pm, &mot, (droite as f32, (cy - 16.0) as f32), 12.5, couleur, Align::Right);
            gras.draw(pm, &p, (droite as f32, (cy + 5.0) as f32), 15.0, j::TEXTE, Align::Right);
            fleche(pm, fleche_de(l), droite - pw - 9.0, cy, j::TEXTE);
            text.draw(pm, &l.frequence, (droite as f32, (cy + 22.0) as f32), 12.0, j::DIM, Align::Right);
            col_w = col_w.max(text.width(&l.frequence, 12.0) as f64).max(text.width(&mot, 12.5) as f64 + 10.0);
        }
        let place = (droite - col_w - 10.0 - tx).max(30.0);
        let base = gras.fit(&l.base, 15.0, place);
        let bw = gras.width(&base, 15.0) as f64;
        gras.draw(pm, &base, (tx as f32, (cy - 2.0) as f32), 15.0, j::TEXTE, Align::Left);
        if !l.quote.is_empty() && bw < place {
            let qt = text.fit(&format!("/{}", l.quote), 15.0, place - bw);
            text.draw(pm, &qt, ((tx + bw) as f32, (cy - 2.0) as f32), 15.0, j::DIM, Align::Left);
        }
        let mut gris: Vec<String> = vec![];
        if !l.bourse.is_empty() {
            gris.push(l.bourse.clone());
        }
        if !l.ut.is_empty() {
            gris.push(l.ut.clone());
        }
        if let Some(t) = l.ts {
            gris.push(quand(t, now));
        }
        let g = text.fit(&gris.join(" · "), 12.5, place);
        text.draw(pm, &g, (tx as f32, (cy + 16.0) as f32), 12.5, j::DIM, Align::Left);
        if lit {
            let t = Self::trade_rect(q);
            round_fill(pm, t, t.h / 2.0, j::BLANC, 0.1);
            round_stroke(pm, t, t.h / 2.0, j::sur_fond(j::LISERE), 1.0);
            text.draw(pm, "Trade", (t.cx() as f32, (t.cy() + 5.0) as f32), 13.5, j::TEXTE, Align::Center);
        }
    }

    /// Draws the panel `r` (window points) in its own image; `verre`: the blurred chart is under it (glass tint),
    /// else opaque.
    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &self,
        text: &Text,
        r: Rect,
        d: &Donnees,
        now: i64,
        mouse: Option<(f64, f64)>,
        verre: bool,
        logo: &dyn Fn(&str) -> Option<Pixmap>,
    ) -> Option<Pixmap> {
        let gras = Text::graisse(Graisse::Bold);
        let mut pm = crate::px::new(r.w.ceil() as u32, r.h.ceil() as u32)?;
        let loc = |q: Rect| Rect::new(q.x - r.x, q.y - r.y, q.w, q.h);
        let survol = |q: &Rect| mouse.is_some_and(|(x, y)| q.contains(x, y));
        let (carte, rayon) = if self.feuille {
            (Rect::new(0.0, 0.0, r.w, r.h + 30.0), 26.0)
        } else {
            (Rect::new(0.5, 0.5, r.w - 1.0, r.h - 1.0), j::R_VOLET)
        };
        if verre {
            round_fill(&mut pm, carte, rayon, j::VERRE.0, j::VERRE.1);
        } else {
            round_fill(&mut pm, carte, rayon, j::sur_fond(j::VERRE), 1.0);
        }
        round_stroke(&mut pm, carte, rayon, j::sur_fond(j::LISERE), 1.0);
        if self.feuille {
            round_fill(&mut pm, Rect::new(r.w / 2.0 - 20.0, 8.0, 40.0, 5.0), 2.5, j::DIM, 0.7);
        }
        let y0 = self.haut();
        gras.draw(&mut pm, "🔔 Alertes", (PAD as f32, (y0 + 34.0) as f32), 19.0, j::TEXTE, Align::Left);
        let zones = self.zones(text, r, d, mouse);
        for (q, z) in &zones {
            let lq = loc(*q);
            match z {
                AZone::Nouvelle | AZone::Reglages | AZone::Fermer => {
                    let plus = *z == AZone::Nouvelle;
                    let (c, a) = if plus { (j::ACCENT, 0.35) } else { (j::BLANC, 0.08) };
                    round_fill(&mut pm, lq, lq.h / 2.0, c, a + if survol(q) { 0.1 } else { 0.0 });
                    round_stroke(&mut pm, lq, lq.h / 2.0, if plus { j::ACCENT } else { j::sur_fond(j::LISERE) }, 1.0);
                    match z {
                        AZone::Nouvelle => {
                            crate::phosphor::draw(&mut pm, "plus", lq.cx(), lq.cy(), 16, j::TEXTE);
                        }
                        AZone::Reglages => {
                            crate::phosphor::draw(&mut pm, "gear", lq.cx(), lq.cy(), 16, j::TEXTE);
                        }
                        _ => text.draw(&mut pm, "✕", (lq.cx() as f32, (lq.cy() + 5.0) as f32), 14.0, j::TEXTE, Align::Center),
                    }
                }
                AZone::Champ => {
                    round_fill(&mut pm, lq, lq.h / 2.0, j::CHAMP.0, j::CHAMP.1);
                    round_stroke(
                        &mut pm,
                        lq,
                        lq.h / 2.0,
                        if self.recherche.is_some() { j::ACCENT } else { j::sur_fond(j::LISERE) },
                        1.0,
                    );
                    let (s, c) = match self.recherche.as_deref() {
                        Some(q) => (format!("🔎  {q}|"), j::TEXTE),
                        None => ("🔎  Chercher une alerte".to_string(), j::DIM),
                    };
                    text.draw(
                        &mut pm,
                        &text.fit(&s, 14.0, lq.w - 28.0),
                        ((lq.x + 14.0) as f32, (lq.cy() + 5.0) as f32),
                        14.0,
                        c,
                        Align::Left,
                    );
                }
                AZone::Vider => {
                    let f = if survol(q) { &gras } else { text };
                    f.draw(
                        &mut pm,
                        "Vider l'historique",
                        ((lq.x + lq.w) as f32, (lq.cy() + 5.0) as f32),
                        14.0,
                        crate::palette::down(),
                        Align::Right,
                    );
                }
                AZone::Onglet(k) => {
                    if *k == 0 {
                        let fond = Rect::new(PAD, y0 + ONGLETS_Y, r.w - 2.0 * PAD, ONGLETS_H);
                        round_fill(&mut pm, fond, fond.h / 2.0, j::BLANC, 0.05);
                    }
                    let on = *k == self.onglet;
                    if on || survol(q) {
                        round_fill(&mut pm, lq, lq.h / 2.0, j::ACCENT, if on { 0.35 } else { 0.08 });
                    }
                    let f = if on { &gras } else { text };
                    let nom = if *k == 0 { "Actives" } else { "Historique" };
                    f.draw(
                        &mut pm,
                        nom,
                        (lq.cx() as f32, (lq.cy() + 5.0) as f32),
                        14.5,
                        if on { j::TEXTE } else { j::DIM },
                        Align::Center,
                    );
                }
                AZone::Ligne(_) | AZone::Trade(_) => {}
            }
        }
        // the list, in its own image, clipped between the tabs and the footer
        let (haut, bas) = self.bande(r);
        if let Some(mut liste) = crate::px::new(r.w.ceil() as u32, r.h.ceil() as u32) {
            let (lignes, lit) = self.lignes(r, d, mouse);
            let montrees = self.montrees(d);
            for (q, cle) in &lignes {
                if q.y + q.h <= haut || q.y >= bas {
                    continue;
                }
                let Some(l) = montrees.iter().find(|l| l.cle == *cle) else { continue };
                let lq = loc(*q);
                let allume = lit == Some(*cle);
                if l.pause {
                    // the whole row pale (paused)
                    if let Some(mut p) = crate::px::new(r.w.ceil() as u32, (lq.y + lq.h + 2.0).max(1.0).ceil() as u32) {
                        self.ligne_draw(&mut p, text, logo, lq, l, now, false);
                        crate::px::put_alpha(&mut liste, 0.0, 0.0, &p, 0.45);
                    }
                } else {
                    if self.onglet == 0 && survol(q) {
                        round_fill(&mut liste, lq, j::R_LIGNE + 4.0, j::BLANC, 0.05);
                    }
                    self.ligne_draw(&mut liste, text, logo, lq, l, now, allume);
                }
            }
            if montrees.is_empty() {
                let msg = match (self.onglet, self.recherche.as_deref().is_some_and(|q| !q.trim().is_empty())) {
                    (_, true) => "Aucune alerte avec ce filtre",
                    (0, false) => "Aucune alerte : touche + pour en créer une",
                    _ => "Aucune alerte déclenchée depuis cette version",
                };
                text.draw(
                    &mut liste,
                    &text.fit(msg, 13.5, r.w - 2.0 * PAD),
                    (PAD as f32, (haut - r.y + 28.0) as f32),
                    13.5,
                    j::DIM,
                    Align::Left,
                );
            }
            if let Some(part) = crate::px::int_rect(0.0, haut - r.y, r.w, (bas - haut).max(1.0)).and_then(|q| liste.clone_rect(q)) {
                crate::px::put(&mut pm, 0.0, haut - r.y, &part);
            }
        }
        // the footer
        let fy = bas - r.y;
        crate::fill(&mut pm, PAD, fy, r.w - 2.0 * PAD, 1.0, j::BLANC, 0.08);
        let (gauche, total) = if self.onglet == 1 {
            let (a, s, t) = compte_historique(&d.historique, now);
            ([("Aujourd'hui", a), ("Cette semaine", s)], t)
        } else {
            let (a, p, t) = compte_actives(&d.actives);
            ([("Actives", a), ("En pause", p)], t)
        };
        let ty = (fy + PIED / 2.0 + 5.0) as f32;
        let mut x = PAD;
        for (k, (mot, n)) in gauche.iter().enumerate() {
            if k > 0 {
                text.draw(&mut pm, "·", ((x + 8.0) as f32, ty), 13.0, j::DIM, Align::Center);
                x += 16.0;
            }
            let mot = crate::i18n::tr(mot);
            text.draw(&mut pm, &mot, (x as f32, ty), 13.5, j::DIM, Align::Left);
            x += text.width(&mot, 13.5) as f64 + 5.0;
            let s = n.to_string();
            gras.draw(&mut pm, &s, (x as f32, ty), 13.5, j::TEXTE, Align::Left);
            x += gras.width(&s, 13.5) as f64 + 6.0;
        }
        let s = total.to_string();
        let sw = gras.width(&s, 13.5) as f64;
        gras.draw(&mut pm, &s, ((r.w - PAD) as f32, ty), 13.5, j::TEXTE, Align::Right);
        text.draw(&mut pm, "Total", ((r.w - PAD - sw - 5.0) as f32, ty), 13.5, j::DIM, Align::Right);
        Some(pm)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Sun 22 Nov 2026, 15:00 New York (20:00 UTC)
    const NOW: i64 = 1_795_377_600;

    fn l(cle: u64, base: &str, bourse: &str, ts: i64, sens: Sens, px: f64, pause: bool) -> Ligne {
        Ligne {
            cle,
            coin: format!("x:{base}"),
            base: base.into(),
            quote: "USDT".into(),
            bourse: bourse.into(),
            ut: "1h".into(),
            ts: Some(ts),
            sens,
            fleche: None,
            prix: Some(px),
            frequence: "Une fois".into(),
            pause,
            arret: String::new(),
            declenche: Some(px - 2.5),
        }
    }

    /// 🔎 The search ignores case and separators: « solusdt », « SOL/USDT », « sol-usdt », « Sol Usdt » find
    /// SOL/USDT; « btcusd » finds BTC/USD, BTC/USDC and BTC/USDT; « hyperliquid » the Hyperliquid alerts; « eth »
    /// not BTC/USDT; the same in Historique.
    #[test]
    fn recherche_sans_separateurs() {
        let p = |cle: u64, coin: &str, base: &str, quote: &str, bourse: &str| Ligne {
            coin: coin.into(),
            quote: quote.into(),
            ..l(cle, base, bourse, NOW, Sens::Up, 1.0, false)
        };
        let lignes = vec![
            p(1, "bybit:SOLUSDT", "SOL", "USDT", "Bybit Perp"),
            p(2, "bitstamp:BTC", "BTC", "USD", "Bitstamp Spot"),
            p(3, "hl:BTC", "BTC", "USDC", "Hyperliquid Perp"),
            p(4, "binance:BTCUSDT", "BTC", "USDT", "Binance Perp"),
            p(5, "hl:HYPE", "HYPE", "USDC", "Hyperliquid Perp"),
            p(6, "", "BTCUSDC.P", "", ""),
        ];
        let d = Donnees {
            actives: lignes.clone(),
            historique: lignes,
        };
        for onglet in [0, 1] {
            let mut e = Etat { onglet, ..Default::default() };
            let mut cles = |q: &str| {
                e.recherche = Some(q.into());
                e.montrees(&d).iter().map(|l| l.cle).collect::<Vec<_>>()
            };
            for q in ["solusdt", "SOL/USDT", "sol-usdt", "Sol Usdt", "sol_usdt", "SOLUSDT.P"] {
                assert_eq!(cles(q), [1], "{q}");
            }
            assert_eq!(cles("btcusd"), [2, 3, 4, 6]);
            assert_eq!(cles("btcusdc"), [3, 6]);
            assert_eq!(cles("hyperliquid"), [3, 5]);
            assert!(!cles("eth").contains(&4), "eth: not BTC/USDT");
            assert_eq!(cles("/"), [1, 2, 3, 4, 5, 6], "nothing left to search: all");
        }
    }

    /// 🔔 A 2-condition alert's arrow is its price's condition (↘ « en dessous », ↗ « au-dessus »), not ↕; ↕ stays
    /// for « Croisement » and for « n conditions » with no known sense.
    #[test]
    fn fleche_des_alertes_a_conditions() {
        let mut x = l(4, "ZEC", "Hyperliquid Perp", NOW, Sens::Conds(2), 420.0, false);
        assert_eq!(fleche_de(&x), Sens::Cross);
        x.fleche = Some(Sens::Down);
        assert_eq!(fleche_de(&x), Sens::Down);
        x.fleche = Some(Sens::Up);
        assert_eq!(fleche_de(&x), Sens::Up);
        let c = l(3, "HYPE", "Hyperliquid Perp", NOW, Sens::Cross, 31.25, false);
        assert_eq!(fleche_de(&c), Sens::Cross);
        assert_eq!(fleche_de(&l(2, "ETH", "Binance Perp", NOW, Sens::Down, 1_980.0, false)), Sens::Down);
    }

    fn donnees() -> Donnees {
        Donnees {
            actives: vec![
                l(1, "BTC", "Binance Perp", NOW - 3_600, Sens::Up, 61_500.0, false),
                l(2, "ETH", "Binance Perp", NOW - 90_000, Sens::Down, 1_980.0, false),
                l(3, "HYPE", "Hyperliquid Perp", NOW - 90_000, Sens::Cross, 31.25, false),
                l(4, "ZEC", "Hyperliquid Perp", NOW - 180_000, Sens::Conds(2), 420.0, false),
                l(5, "SOL", "OKX Perp", NOW - 270_000, Sens::Down, 118.4, true),
            ],
            historique: vec![
                l(0, "BTC", "Binance Perp", NOW - 600, Sens::Down, 58_900.0, false),
                l(1, "ZEC", "Hyperliquid Perp", NOW - 86_400, Sens::Up, 452.0, false),
                l(2, "XRP", "OKX Perp", NOW - 2 * 86_400, Sens::Down, 1.2488, false),
                l(3, "ETH", "Binance Perp", NOW - 8 * 86_400, Sens::Up, 2_150.0, false),
            ],
        }
    }

    /// 🔔 Prices as the mock-up writes them; « aujourd'hui » / « 21 nov. » in New York time.
    #[test]
    fn prix_et_dates() {
        assert_eq!(prix(61_500.0), "61 500,0");
        assert_eq!(prix(1_980.0), "1 980,00");
        assert_eq!(prix(31.25), "31,250");
        assert_eq!(prix(1.2488), "1,2488");
        assert_eq!(prix(58_897.5), "58 897,5");
        assert_eq!(quand(NOW - 600, NOW), "aujourd'hui");
        assert_eq!(quand(NOW - 86_400, NOW), "21 nov.");
        // 23:30 New York on the 21st is 04:30 UTC on the 22nd: still the 21st in New York
        assert_eq!(quand(1_795_321_800, NOW), "21 nov.");
    }

    /// 🔔 The footers: Actives n · En pause n · Total; Aujourd'hui · Cette semaine (from Monday, New York) · Total;
    /// the day changes at midnight New York, not UTC.
    #[test]
    fn compteurs_du_pied() {
        let d = donnees();
        assert_eq!(compte_actives(&d.actives), (4, 1, 5));
        // Sunday 22 Nov: the week started Monday 16 Nov → 10 min, yesterday, 2 days; not 8 days ago
        assert_eq!(compte_historique(&d.historique, NOW), (1, 3, 4));
        // Monday 23 Nov 00:30 New York: a new day and a new week
        let lundi = 1_795_411_800;
        assert_eq!(compte_historique(&d.historique, lundi), (0, 0, 4));
        // a firing at 23:50 New York Sunday counts for Sunday, though it is Monday in UTC
        let h = vec![Ligne {
            ts: Some(1_795_409_400),
            ..Default::default()
        }];
        assert_eq!(compte_historique(&h, 1_795_409_700), (1, 1, 1));
        assert_eq!(compte_historique(&h, lundi), (0, 0, 1));
    }

    /// 🔎 The search keeps the rows whose symbol or exchange it names (no case, no accent), in the open tab.
    #[test]
    fn filtre_de_recherche() {
        let d = donnees();
        let mut e = Etat {
            recherche: Some("hyper".into()),
            ..Default::default()
        };
        let cles = |e: &Etat| e.montrees(&d).iter().map(|l| l.cle).collect::<Vec<_>>();
        assert_eq!(cles(&e), [3, 4]);
        e.recherche = Some("BTC".into());
        assert_eq!(cles(&e), [1]);
        e.recherche = Some("btc/usdt".into());
        assert_eq!(cles(&e), [1]);
        e.onglet = 1;
        e.recherche = Some("okx".into());
        assert_eq!(cles(&e), [2]);
        e.recherche = None;
        assert_eq!(cles(&e), [0, 1, 2, 3]);
    }

    /// 🔔 Clicks: + ⚙ ✕, the tabs, a row of Actives (its menu), a history row lights and shows Trade, Trade, Vider,
    /// the field takes the keyboard; outside the sheet: close.
    #[test]
    fn clics_du_panneau() {
        let t = Text::default();
        let d = donnees();
        let r = Rect::new(1000.0, 60.0, 420.0, 760.0);
        let mut e = Etat::default();
        let zone = |e: &Etat, z: &AZone| e.zones(&t, r, &d, None).into_iter().find(|(_, x)| x == z).map(|(q, _)| q);
        let q = zone(&e, &AZone::Nouvelle).unwrap();
        assert!(matches!(e.click(&t, r, &d, q.cx(), q.cy()), AClic::Nouvelle(_)));
        let q = zone(&e, &AZone::Reglages).unwrap();
        assert!(matches!(e.click(&t, r, &d, q.cx(), q.cy()), AClic::Reglages(_)));
        let q = zone(&e, &AZone::Fermer).unwrap();
        assert_eq!(e.click(&t, r, &d, q.cx(), q.cy()), AClic::Fermer);
        assert!(zone(&e, &AZone::Vider).is_none(), "« Vider » only in Historique");
        let q = zone(&e, &AZone::Ligne(2)).unwrap();
        assert!(matches!(e.click(&t, r, &d, q.cx(), q.cy()), AClic::Active(2, _)));
        let q = zone(&e, &AZone::Onglet(1)).unwrap();
        assert_eq!(e.click(&t, r, &d, q.cx(), q.cy()), AClic::Changed);
        assert_eq!(e.onglet, 1);
        assert!(zone(&e, &AZone::Trade(1)).is_none(), "no Trade before a row is lit");
        let q = zone(&e, &AZone::Ligne(1)).unwrap();
        assert_eq!(e.click(&t, r, &d, q.cx(), q.cy()), AClic::Changed);
        assert_eq!(e.touche, Some(1));
        let q = zone(&e, &AZone::Trade(1)).unwrap();
        assert_eq!(e.click(&t, r, &d, q.cx(), q.cy()), AClic::Trade(1));
        let q = zone(&e, &AZone::Vider).unwrap();
        assert_eq!(e.click(&t, r, &d, q.cx(), q.cy()), AClic::Vider);
        let q = zone(&e, &AZone::Champ).unwrap();
        e.click(&t, r, &d, q.cx(), q.cy());
        for c in "zec".chars() {
            e.taper(Some(c));
        }
        assert_eq!(e.recherche.as_deref(), Some("zec"));
        assert_eq!(e.montrees(&d).len(), 1);
        assert_eq!(e.click(&t, r, &d, 10.0, 10.0), AClic::Fermer);
        // PC: the mouse over a history row lights it (Trade under the mouse's row)
        let mut e = Etat {
            onglet: 1,
            ..Default::default()
        };
        let q = zone(&e, &AZone::Ligne(0)).unwrap();
        let z = e.zones(&t, r, &d, Some((q.cx(), q.cy())));
        let tr = z.iter().find(|(_, x)| *x == AZone::Trade(0)).expect("Trade on the row under the mouse").0;
        assert_eq!(e.click(&t, r, &d, tr.cx(), tr.cy()), AClic::Trade(0));
        e.wheel(500.0, r, &d);
        assert_eq!(e.defile, 0.0, "a short list does not scroll");
    }

    /// 📱 The phone sheet 390 × 844: nothing overlaps, everything inside, the footer above the round Argos AI.
    #[test]
    fn rien_ne_chevauche_sur_cell() {
        let t = Text::default();
        crate::actions::set_tactile(true);
        let d = donnees();
        let r = Rect::new(0.0, 844.0 - 584.3, 390.0, 584.3);
        for onglet in [0, 1] {
            let e = Etat {
                onglet,
                feuille: true,
                touche: (onglet == 1).then_some(0),
                ..Default::default()
            };
            let z = e.zones(&t, r, &d, None);
            for (i, (a, za)) in z.iter().enumerate() {
                assert!(a.x >= r.x && a.x + a.w <= r.x + r.w + 0.01 && a.y >= r.y, "{za:?} {a:?} out of the sheet");
                for (b, zb) in &z[i + 1..] {
                    let dedans = matches!((za, zb), (AZone::Trade(_), AZone::Ligne(_)));
                    let croise = a.x < b.x + b.w - 0.01 && b.x < a.x + a.w - 0.01 && a.y < b.y + b.h - 0.01 && b.y < a.y + a.h - 0.01;
                    assert!(dedans || !croise, "{za:?} {a:?} overlaps {zb:?} {b:?}");
                }
            }
            let (_, bas) = e.bande(r);
            assert!(bas + PIED <= r.y + r.h - crate::eco::marge_ai() + 0.01, "the footer above the round Argos AI");
            assert!(z.iter().filter(|(_, x)| matches!(x, AZone::Ligne(_))).all(|(q, _)| q.y + q.h <= bas + 0.01));
            assert!(e.draw(&t, r, &d, NOW, None, true, &|_| None).is_some());
        }
        crate::actions::set_tactile(false);
    }
}
