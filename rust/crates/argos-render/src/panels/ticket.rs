//! 🎫 Module 31 « Trading démo » : le ticket de la feuille Trading, copié de la maquette v1 du 22 nov. (mesures en
//! points depuis le haut de la feuille : le haut de la maquette + 23,5, l'en-tête de la feuille étant plus haut).
//! De haut en bas : le compte démo et le marché ; 🛡 Gardien PropFirm sur une ligne (touchée : sa carte complète) ;
//! Marché / Limite / Stop ; Disponible, Pouvoir d'achat, Position ; Prix (Mid, ⌖), Quantité, Valeur ; le curseur % ;
//! Risque au SL ; TP/SL, Reduce-only et la durée (GTC / IOC / Post-only) ; TP et SL ; VENDRE / SHORT et ACHETER / LONG
//! (📱 collés en bas de la feuille du téléphone) ; Marge, Frais, Max, Glissement ; Compte (replié sur une ligne) ;
//! « Positions et ordres › », « Carnet › » ; « ⚙ Tous les réglages › » (l'ancien ticket complet).
//! Compte démo seulement : rien ici ne parle à une bourse.

use super::cell::{deci, large, prix, telephone, Cell, GARDE_Y0, X};
use super::{Hit, PanelClick, Panels, Trading};
use crate::shapes::{lines, round_fill, round_stroke, Rect};
use crate::text::{Align, Graisse, Rgb, Text};
use argos_core::simu::{self, Kind, OrderTicket, Validity};
use argos_core::Money;
use tiny_skia::Pixmap;

// the mockup's colours (ticket.css)
const TEXTE: Rgb = Rgb(0xe8, 0xec, 0xf7);
const CLAIR: Rgb = Rgb(0xdf, 0xe5, 0xf5);
const DOUX: Rgb = Rgb(0xc9, 0xd3, 0xee);
const GRIS: Rgb = Rgb(0x8a, 0x93, 0xad);
const PALE: Rgb = Rgb(0x93, 0xa0, 0xc2);
const BLEU: Rgb = Rgb(0x4d, 0xa3, 0xff);
const ORANGE: Rgb = Rgb(0xff, 0xb7, 0x4d);
const HAUSSE: Rgb = Rgb(0x3c, 0xcf, 0x8e);
const BAISSE: Rgb = Rgb(0xff, 0x5c, 0x66);
const VENDRE: Rgb = Rgb(234, 57, 67);
const ACHETER: Rgb = Rgb(22, 199, 132);
const MENU: Rgb = Rgb(0x16, 0x1c, 0x31);
const BLANC: Rgb = Rgb(255, 255, 255);

/// The mockup's tops are this much higher (the sheet's header and tabs are taller).
const DY: f64 = 23.5;
/// 📌 The sheet's header (title, ⬓ ⤢ ✕, the tabs) stays at the top: the ticket scrolls under it.
pub const ENTETE: f64 = 116.0;
/// 📱 Room at the bottom of the phone's sheet for VENDRE / ACHETER (12 under them, the line 10 above them).
pub const BAS_COLLE: f64 = 78.0;
const CHAMP_H: f64 = 42.0;
const BOUTONS_H: f64 = 54.0;
const COMPTE_H: f64 = 36.0;
/// The unfolded Compte: its header and its 9 rows in 3 groups.
const COMPTE_OUVERT_H: f64 = 269.0;
/// The Gardien card unfolded: from its top to the end of « Règles » (its message and 🔒 block add to it).
const GARDE_CARTE_H: f64 = 203.3;

/// 🎫 A field of the ticket typed in a box.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Champ {
    Prix,
    Quantite,
    Valeur,
    Pct,
    Tp,
    Sl,
}

impl Champ {
    /// The box's title.
    pub fn titre(self) -> &'static str {
        match self {
            Champ::Prix => "🎫 Prix de l'ordre",
            Champ::Quantite => "🎫 Quantité",
            Champ::Valeur => "🎫 Valeur (en dollars)",
            Champ::Pct => "🎫 % du pouvoir d'achat",
            Champ::Tp => "🎯 Prix du TP",
            Champ::Sl => "🛡️ Prix du SL",
        }
    }

    /// The example under the box.
    pub fn exemple(self) -> &'static str {
        match self {
            Champ::Prix | Champ::Tp | Champ::Sl => "ex. 60 057,9",
            Champ::Quantite => "ex. 0,5",
            Champ::Valeur => "ex. 5 000",
            Champ::Pct => "ex. 25",
        }
    }
}

/// A number typed the French way or not (« 60 057,9 », « 0.5 », « 5 % », « 1 000 $ », « 1,234.5 »): of two marks,
/// the last one is the decimal mark and the other one separates the thousands.
fn nombre(s: &str) -> Option<f64> {
    let t: String = s
        .chars()
        .filter(|c| !c.is_whitespace() && !matches!(c, '%' | '$'))
        .map(|c| if c == '−' { '-' } else { c })
        .collect();
    let t = match (t.rfind(','), t.rfind('.')) {
        (Some(v), Some(p)) if v > p => t.replace('.', "").replace(',', "."),
        (Some(_), Some(_)) => t.replace(',', ""),
        (Some(_), None) if t.matches(',').count() > 1 => t.replace(',', ""),
        (None, Some(_)) if t.matches('.').count() > 1 => t.replace('.', ""),
        _ => t.replace(',', "."),
    };
    t.parse::<f64>().ok().filter(|v| v.is_finite())
}

impl Trading {
    /// Marché / Limite / Stop.
    pub fn kind(&self) -> Kind {
        match self.tab {
            0 => Kind::Market,
            1 => Kind::Limit,
            _ => Kind::Stop,
        }
    }

    /// The order's price: the last price at market, else the typed price (the last price until one is typed).
    pub fn prix_ordre(&self) -> f64 {
        if self.tab == 0 || self.px <= 0.0 {
            self.prix
        } else {
            self.px
        }
    }

    /// 📏 The market's quantity step at the order's price.
    pub fn pas(&self) -> f64 {
        simu::qty_step(self.prix_ordre())
    }

    /// The order of a buy (`buy`) or a sell as the ticket stands; TP and SL (box ticked, not reduce-only) at their
    /// distances from the price it fills at, mirrored for a sell.
    pub fn ordre(&self, buy: bool) -> OrderTicket {
        let kind = self.kind();
        let mut t = OrderTicket {
            buy,
            kind,
            price: self.prix_ordre(),
            qty: self.qty,
            sl: None,
            tp: None,
            validity: if kind == Kind::Limit { self.validity } else { Validity::Gtc },
            reduce_only: self.reduce_only,
        };
        if self.tpsl && !self.reduce_only {
            let e = t.fill_price(self.prix);
            let s = if buy { 1.0 } else { -1.0 };
            // (to the precision the ticket shows them: the level placed is the one read, « 59 457,3 »)
            let pas = 10f64.powi(-(deci(e) as i32));
            let net = |v: f64| (v / pas).round() * pas;
            t.tp = Some(net(e * (1.0 + s * self.tp_pct / 100.0)));
            t.sl = Some(net(e * (1.0 - s * self.sl_pct / 100.0)));
        }
        t
    }

    /// The side the ticket's numbers are shown for (TP, SL, risk, costs): the one where the order waits (a Limit under
    /// the price buys, above it sells; a Stop the other way round); at market, or at the last price, the last side used.
    pub fn sens_affiche(&self) -> bool {
        let (p, last) = (self.prix_ordre(), self.prix);
        match self.kind() {
            Kind::Limit if p < last => true,
            Kind::Limit if p > last => false,
            Kind::Stop if p > last => true,
            Kind::Stop if p < last => false,
            _ => self.buy,
        }
    }

    /// The quantity 100 % of the slider stands for: the buying power at the order's price (reduce-only: the
    /// position on the other side).
    fn base_pct(&self) -> f64 {
        let p = self.prix_ordre();
        if self.reduce_only {
            self.reducible.0.max(self.reducible.1)
        } else if p > 0.0 {
            self.pouvoir / p
        } else {
            0.0
        }
    }

    /// The slider at `pct` %: the quantity follows (rounded down to the step).
    pub fn set_pct(&mut self, pct: f64) {
        let pct = if pct.is_finite() { pct.clamp(0.0, 100.0) } else { 0.0 };
        self.pct = pct;
        self.qty = simu::round_qty(self.base_pct() * pct / 100.0, self.pas());
    }

    /// A quantity (rounded down to the step; reduce-only: at most the position): the slider follows.
    pub fn set_qty(&mut self, q: f64) {
        let b = self.base_pct();
        let q = simu::round_qty(q, self.pas());
        self.qty = if self.reduce_only && b > 0.0 { q.min(b) } else { q };
        self.pct = if b > 0.0 { (self.qty / b * 100.0).clamp(0.0, 100.0) } else { 0.0 };
    }

    /// The order's value: quantity × its price.
    pub fn valeur(&self) -> f64 {
        self.qty * self.prix_ordre()
    }

    /// « Risque au SL » of the side shown: the loss if the SL is hit, and its % of the account (`None`: no SL, no
    /// size).
    pub fn risque_sl(&self) -> Option<(f64, f64)> {
        let t = self.ordre(self.sens_affiche());
        let sl = t.sl?;
        (t.qty > 0.0).then(|| {
            let r = (t.fill_price(self.prix) - sl).abs() * t.qty;
            (r, if self.balance > 0.0 { r / self.balance * 100.0 } else { 0.0 })
        })
    }

    /// A typed value: the order's price, the quantity, the value, the %, or the TP / SL price (of the side shown).
    /// The message says what is wrong.
    pub fn saisir(&mut self, c: Champ, s: &str) -> Result<(), String> {
        let v = nombre(s).ok_or_else(|| format!("« {} » : écris un nombre", s.trim()))?;
        match c {
            Champ::Prix => {
                if v <= 0.0 {
                    return Err("le prix doit être plus grand que 0".into());
                }
                self.px = v;
                self.set_qty(self.qty);
            }
            Champ::Quantite => self.set_qty(v.max(0.0)),
            Champ::Valeur => {
                let p = self.prix_ordre();
                if p > 0.0 {
                    self.set_qty(v.max(0.0) / p);
                }
            }
            Champ::Pct => self.set_pct(v),
            Champ::Tp | Champ::Sl => {
                let buy = self.sens_affiche();
                let e = self.ordre(buy).fill_price(self.prix);
                if e <= 0.0 {
                    return Err(simu::Refus::PrixAbsent.message(buy));
                }
                // distance in % from the entry, positive in the trade's direction
                let d = (v - e) / e * 100.0 * if buy { 1.0 } else { -1.0 };
                if c == Champ::Tp && d <= 0.0 {
                    return Err(simu::Refus::TpMauvaisCote.message(buy));
                }
                if c == Champ::Sl && d >= 0.0 {
                    return Err(simu::Refus::SlMauvaisCote.message(buy));
                }
                self.poser_niveau(c == Champ::Sl, v);
            }
        }
        Ok(())
    }

    /// The SL (`stop`) or the TP at `price`, for the side shown: its distance in % from the price the order fills at,
    /// kept as it is (a level on the wrong side is the caller's to check: [`Trading::saisir`] refuses it, a line
    /// dragged there on the chart comes back when let go).
    pub fn poser_niveau(&mut self, stop: bool, price: f64) {
        let buy = self.sens_affiche();
        let e = self.ordre(buy).fill_price(self.prix);
        if e <= 0.0 || !price.is_finite() {
            return;
        }
        let d = (price - e) / e * 100.0 * if buy { 1.0 } else { -1.0 };
        if stop {
            self.sl_pct = -d;
        } else {
            self.tp_pct = d;
        }
        self.tpsl = true;
    }

    /// The price a typed field starts with (its box opens with it).
    pub fn champ_txt(&self, c: Champ) -> String {
        let t = self.ordre(self.sens_affiche());
        match c {
            Champ::Prix => prix(self.prix_ordre()),
            Champ::Quantite => qte(self.qty, self.pas()),
            Champ::Valeur => crate::viewport::format_number(self.valeur(), 2),
            Champ::Pct => crate::viewport::format_number(self.pct, 0),
            Champ::Tp => t.tp.map_or(String::new(), prix),
            Champ::Sl => t.sl.map_or(String::new(), prix),
        }
    }
}

/// A quantity with the step's decimals (« 0,8325 »), « 0 » when there is none.
fn qte(q: f64, pas: f64) -> String {
    if q > 0.0 {
        crate::viewport::format_number(q, simu::qty_decimals(pas))
    } else {
        "0".into()
    }
}

/// « 16,65 » : Max with 2 decimals from 1 up (rounded down), else the step's.
fn qte_max(q: f64, pas: f64) -> String {
    if q >= 1.0 {
        crate::viewport::format_number((q * 100.0 + 1e-9).floor() / 100.0, 2)
    } else {
        qte(q, pas)
    }
}

/// « 500,00 $ ».
fn usd(v: f64) -> String {
    crate::simulateur::argent(Money::from_f64(v), false)
}

/// « +83,25 $ » / « −1,25 $ ».
fn usd_signe(v: f64) -> String {
    crate::simulateur::argent(Money::from_f64(v), true)
}

/// « 2R », « 1,5R ».
fn r_txt(r: f64) -> String {
    let t = crate::viewport::format_number(r, 1);
    format!("{}R", t.trim_end_matches('0').trim_end_matches([',', '.']))
}

/// « +2,00 % ».
fn pct_txt(p: f64) -> String {
    format!("{}{} %", if p > 0.0 { "+" } else { "" }, crate::viewport::format_number(p, 2))
}

/// Where each part of the ticket is (points from the top of the sheet).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Places {
    pub acc: f64,
    pub gard: f64,
    pub gard_h: f64,
    pub types: f64,
    pub info: f64,
    /// Prix (Limite, Stop), Quantité, Valeur.
    pub prix: Option<f64>,
    pub qty: f64,
    pub val: f64,
    /// The slider's row (its % box).
    pub curseur: f64,
    pub risque: Option<f64>,
    pub opt: f64,
    pub tpsl: Option<f64>,
    /// VENDRE / ACHETER in the flow (`None`: 📱 kept at the bottom of the phone's sheet).
    pub go: Option<f64>,
    pub cout: f64,
    pub compte: f64,
    pub compte_h: f64,
    pub liens: f64,
    pub tous: f64,
    pub fin: f64,
}

/// Height of the options' row (TP/SL, Reduce-only): 30 with the validity's pill (Limite, Stop), else 16.
fn opt_h(t: &Trading) -> f64 {
    if t.tab != 0 {
        30.0
    } else {
        16.0
    }
}

/// A pill of the mockup (`.pu`): its text 13 points, 11 on each side.
fn pilule_w(text: &Text, s: &str) -> f64 {
    text.width(s, 13.0) as f64 + 22.0
}

/// The validity pill, at the right of the options' row (`opt`); with its ⌄ only where it can change (`menu`: Limite;
/// a Stop waits until it is cancelled).
fn validite_rect(pw: f64, opt: f64, v: Validity, menu: bool) -> Rect {
    let w = Text::graisse(Graisse::Bold).width(v.name(), 13.0) as f64 + if menu { 6.0 + 6.0 } else { 0.0 } + 22.0;
    Rect::new(X + large(pw) - w, opt, w, 30.0)
}

/// The validity menu, under its pill, along the right edge (282 × 116), and its rows.
fn menu_rect(pw: f64, opt: f64) -> Rect {
    Rect::new(X + large(pw) - 282.0, opt + 36.0, 282.0, 116.0)
}

fn menu_ligne(m: Rect, k: usize) -> Rect {
    Rect::new(m.x + 7.0, m.y + 7.0 + 34.0 * k as f64, m.w - 14.0, 34.0)
}

/// The account chip (« 🧪 Démo Argos 100K ▾ ») after « Compte : ».
fn compte_chip(text: &Text, t: &Trading, acc: f64) -> Rect {
    let x = 16.0 + text.width("Compte : ", 11.5) as f64 + 6.0;
    Rect::new(x, acc, text.width(&format!("🧪 {} ▾", nom_compte(t)), 12.0) as f64 + 18.0, 21.0)
}

/// The demo account's name as it is (« Démo Argos 100K » already says « Démo »).
fn nom_compte(t: &Trading) -> String {
    if t.account.is_empty() {
        "Démo Argos 100K".into()
    } else {
        t.account.clone()
    }
}

/// « Carnet » of the links, after the mockup's no-break space and space.
const AVANT_CARNET: &str = "\u{a0} Carnet";

/// The two links (« Positions et ordres › », « Carnet › ») : their touch areas.
fn liens_rects(text: &Text, y: f64) -> [Rect; 2] {
    let w = |s: &str| text.width(s, 12.5) as f64;
    let (a, b, c) = (w("Positions et ordres"), w("›"), w(AVANT_CARNET));
    let x1 = 18.0 + a + 18.0 + b + 18.0;
    [
        Rect::new(X, y - 8.0, x1 - 18.0 - X + 9.0, 31.0),
        Rect::new(x1 - 9.0, y - 8.0, c + 18.0 + b + 18.0, 31.0),
    ]
}

impl Panels {
    /// 🛡 The Gardien card shown whole: touched, or the day locked (its 🔓 button must be reachable).
    fn garde_ouverte(&self) -> bool {
        self.trading.garde_ouverte || self.cell_garde().verrou()
    }

    /// Where each part of the ticket is, for a sheet `pw` wide.
    pub(super) fn ticket_places(&self, pw: f64) -> Places {
        let t = &self.trading;
        let acc = 102.0 + DY;
        let gard = acc + 21.0 + 9.0;
        let gard_h = if self.garde_ouverte() {
            GARDE_CARTE_H + self.cell_garde().decale()
        } else {
            29.0
        };
        let types = gard + gard_h + 10.0;
        let info = types + 26.0 + 10.0;
        let mut y = info + 66.0 + 7.0;
        let mut champ = || {
            let r = y;
            y += CHAMP_H + 7.0;
            r
        };
        let prix = (t.tab != 0).then(&mut champ);
        let qty = champ();
        let val = champ();
        let curseur = y - 7.0 + 11.0;
        y = curseur + 32.0;
        let risque = t.risque_sl().map(|_| {
            let r = y + 7.0;
            y = r + 14.0;
            r
        });
        let opt = y + 11.0;
        y = opt + opt_h(t);
        let tpsl = (t.tpsl && !t.reduce_only).then(|| {
            let r = y + 8.0;
            y = r + CHAMP_H;
            r
        });
        let colle = telephone(pw);
        let go = (!colle).then(|| {
            let r = y + 12.0;
            y = r + BOUTONS_H;
            r
        });
        let cout = y + 9.0;
        let compte = cout + 40.0 + 11.0;
        let compte_h = if t.compte_ouvert { COMPTE_OUVERT_H } else { COMPTE_H };
        let liens = compte + compte_h + 11.0;
        let tous = liens + 15.0 + 10.0;
        let fin = tous + 33.0 + 14.0 + if colle { BAS_COLLE } else { 0.0 };
        Places {
            acc,
            gard,
            gard_h,
            types,
            info,
            prix,
            qty,
            val,
            curseur,
            risque,
            opt,
            tpsl,
            go,
            cout,
            compte,
            compte_h,
            liens,
            tous,
            fin,
        }
    }

    /// 🎚 The slider's track (sheet `pw` wide): 0 % at its left, 100 % at its right.
    pub fn ticket_curseur(&self, pw: f64) -> Rect {
        let p = self.ticket_places(pw);
        Rect::new(X, p.curseur + 7.0, large(pw) - 12.0 - 64.0, 18.0)
    }

    /// 📌 Height of the Trading sheet's header that stays at the top while the rest scrolls (0: no such sheet).
    pub fn entete_fixe(&self) -> f64 {
        if self.feuille_trading() {
            ENTETE
        } else {
            0.0
        }
    }

    /// 🎚 The % slider follows the finger at `x` (sheet `pw` wide): a whole %, caught by a notch (0, 25, 50, 75,
    /// 100) within 2 %.
    pub fn ticket_glisse(&mut self, x: f64, pw: f64) {
        let tr = self.ticket_curseur(pw);
        let v = ((x - tr.x) / tr.w * 100.0).clamp(0.0, 100.0);
        let cran = (v / 25.0).round() * 25.0;
        self.trading.set_pct(if (v - cran).abs() <= 2.0 { cran } else { v.round() });
    }

    /// 📌📱 What stays in place in the Trading sheet (`pw` × `h`, `fond` its background), over the scrolled content:
    /// the header at the top, and on the phone VENDRE / ACHETER at the bottom.
    pub fn trading_fixe_draw(&self, pm: &mut Pixmap, text: &Text, (pw, h): (f64, f64), mouse: Option<(f64, f64)>, fond: Rgb) {
        if !self.feuille_trading() {
            return;
        }
        crate::fill(pm, 0.0, 0.0, pw, ENTETE, fond, 1.0);
        self.cell_entete_draw(pm, text, pw, mouse);
        self.ticket_boutons_draw(pm, text, pw, h, mouse, fond);
    }

    /// 📱 A touch at the bottom of the phone's Trading sheet (`y` in its view, `h` high), where VENDRE / ACHETER stay:
    /// one of them (nothing while the day is locked), the validity menu closed, or nothing; `None` above them.
    pub fn click_colle(&mut self, x: f64, y: f64, pw: f64, h: f64) -> Option<PanelClick> {
        let boutons = self.ticket_boutons_colles(pw, h)?;
        if y < h - BAS_COLLE {
            return None;
        }
        if self.trading.menu_validite {
            self.trading.menu_validite = false;
            return Some(PanelClick::Changed);
        }
        let achat = boutons.iter().position(|r| r.contains(x, y)).map(|k| k == 1);
        Some(match achat {
            Some(b) if !self.cell_garde().verrou() => self.cell_click(Cell::Passer(b)),
            _ => PanelClick::None,
        })
    }

    /// 📱 VENDRE / ACHETER kept at the bottom of the phone's sheet (`h` high): their places, `None` elsewhere.
    pub fn ticket_boutons_colles(&self, pw: f64, h: f64) -> Option<[Rect; 2]> {
        (self.feuille_trading() && !self.trading.positions_vue && !self.trading.pas_btc && telephone(pw))
            .then(|| boutons(pw, h - 12.0 - BOUTONS_H, self.trading.place_ai))
    }

    /// The ticket's touch areas (the validity menu, when open, covers the rest: a touch beside closes it).
    pub(super) fn ticket_zones(&self, pw: f64) -> Vec<(Rect, Hit)> {
        let t = &self.trading;
        let text = Text::default();
        let p = self.ticket_places(pw);
        let l = large(pw);
        let mut z = vec![];
        // ₿ a pair against BTC: the message only, nothing to touch
        if t.pas_btc {
            return z;
        }
        if t.menu_validite {
            let m = menu_rect(pw, p.opt);
            z.extend((0..3).map(|k| (menu_ligne(m, k), Hit::Cell(Cell::Duree(k)))));
            z.push((Rect::new(0.0, 0.0, pw, p.fin.max(4_000.0)), Hit::Cell(Cell::Validite)));
            return z;
        }
        z.push((compte_chip(&text, t, p.acc), Hit::Account));
        if self.garde_ouverte() {
            z.extend(self.garde_zones(pw, p.gard - GARDE_Y0));
        } else {
            z.push((Rect::new(X, p.gard, l, 29.0), Hit::Cell(Cell::Gardien)));
        }
        let mut x = X;
        for (k, s) in ["Marché", "Limite", "Stop"].into_iter().enumerate() {
            let w = pilule_w(&text, s);
            z.push((Rect::new(x, p.types, w, 26.0), Hit::Tab(k)));
            x += w + 6.0;
        }
        if let Some(y) = p.prix {
            let f = Rect::new(X, y, l, CHAMP_H);
            let cible = Rect::new(f.x + f.w - 13.0 - 18.0 - 8.0, y + 4.0, 34.0, 34.0);
            let mid = Rect::new(f.x + f.w - 13.0 - 18.0 - 8.0 - 39.7, y + 9.5, 39.7, 23.0);
            z.push((cible, Hit::Cell(Cell::Cible)));
            z.push((mid, Hit::Cell(Cell::Mid)));
            z.push((f, Hit::Cell(Cell::Champ(Champ::Prix))));
        }
        z.push((Rect::new(X, p.qty, l, CHAMP_H), Hit::Cell(Cell::Champ(Champ::Quantite))));
        z.push((Rect::new(X, p.val, l, CHAMP_H), Hit::Cell(Cell::Champ(Champ::Valeur))));
        let tr = self.ticket_curseur(pw);
        // (the track is 18 high: touched 11 above and under it, like a finger)
        z.push((Rect::new(tr.x, tr.y - 11.0, tr.w + 6.0, tr.h + 22.0), Hit::Cell(Cell::Curseur)));
        z.push((Rect::new(X + l - 64.0, p.curseur, 64.0, 32.0), Hit::Cell(Cell::Champ(Champ::Pct))));
        // (30 high, on the row's middle)
        let cy = p.opt + opt_h(t) / 2.0;
        let cb = |s: &str, x: f64| Rect::new(x - 4.0, cy - 15.0, 15.0 + 7.0 + text.width(s, 13.0) as f64 + 8.0, 30.0);
        let tpsl = cb("TP/SL", X);
        z.push((tpsl, Hit::Cell(Cell::TpSl)));
        z.push((cb("Reduce-only", tpsl.x + tpsl.w + 10.0), Hit::Cell(Cell::Reduire)));
        if t.tab == 1 {
            z.push((validite_rect(pw, p.opt, t.validity, true), Hit::Cell(Cell::Validite)));
        }
        if let Some(y) = p.tpsl {
            let w = (l - 7.0) / 2.0;
            z.push((Rect::new(X, y, w, CHAMP_H), Hit::Cell(Cell::Champ(Champ::Tp))));
            z.push((Rect::new(X + w + 7.0, y, w, CHAMP_H), Hit::Cell(Cell::Champ(Champ::Sl))));
        }
        if let Some(y) = p.go {
            let [v, a] = boutons(pw, y, 0.0);
            z.push((v, Hit::Cell(Cell::Passer(false))));
            z.push((a, Hit::Cell(Cell::Passer(true))));
        }
        z.push((Rect::new(X, p.compte, l, COMPTE_H), Hit::Cell(Cell::Compte)));
        let [pos, carnet] = liens_rects(&text, p.liens);
        z.push((pos, Hit::Cell(Cell::Positions)));
        z.push((carnet, Hit::Cell(Cell::Carnet)));
        z.push((Rect::new(X, p.tous, l, 33.0), Hit::Cell(Cell::Reglages)));
        // 🔒 day locked: the ticket does not answer (it is greyed; 🔓 Déverrouiller first)
        if self.cell_garde().verrou() {
            z.retain(|(_, h)| {
                !matches!(
                    h,
                    Hit::Tab(_)
                        | Hit::Cell(Cell::Passer(_) | Cell::Champ(_) | Cell::Mid | Cell::Cible | Cell::Curseur | Cell::TpSl | Cell::Reduire | Cell::Validite)
                )
            });
        }
        z
    }

    /// Draws the ticket under the sheet's header.
    pub(super) fn ticket_draw(&self, pm: &mut Pixmap, text: &Text, pw: f64, mouse: Option<(f64, f64)>) {
        let gras = Text::graisse(Graisse::Bold);
        let demi = Text::graisse(Graisse::Medium);
        let t = &self.trading;
        let p = self.ticket_places(pw);
        let l = large(pw);
        let droite = X + l;
        let survol = |r: Rect| mouse.is_some_and(|(x, y)| r.contains(x, y));
        let fond = |pm: &mut Pixmap, r: Rect, radius: f64, a: f32| round_fill(pm, r, radius, BLANC, a);
        // ₿ a pair against BTC: the demo account counts in dollars, no ticket yet: the message instead
        if t.pas_btc {
            let r = Rect::new(X, p.acc, l, 64.0);
            fond(pm, r, 12.0, 0.06);
            let s = text.fit(super::TICKET_PAS_BTC, 13.0, l - 24.0);
            text.draw(pm, &s, ((X + 12.0) as f32, (r.cy() + 4.5) as f32), 13.0, GRIS, Align::Left);
            return;
        }
        // the account and the market
        text.draw(pm, "Compte :", (16.0, (p.acc + 14.5) as f32), 11.5, GRIS, Align::Left);
        let chip = compte_chip(text, t, p.acc);
        round_fill(pm, chip, chip.h / 2.0, BLANC, if survol(chip) { 0.11 } else { 0.07 });
        text.draw(
            pm,
            &format!("🧪 {} ▾", nom_compte(t)),
            ((chip.x + 9.0) as f32, (p.acc + 15.0) as f32),
            12.0,
            CLAIR,
            Align::Left,
        );
        let (sym, bourse) = &t.marche;
        let bw = text.width(bourse, 11.5) as f64;
        text.draw(pm, bourse, ((droite - 2.0) as f32, (p.acc + 14.5) as f32), 11.5, GRIS, Align::Right);
        let sx = droite - 2.0 - bw - if bourse.is_empty() { 0.0 } else { 4.0 };
        let sym = demi.fit(sym, 11.5, (sx - chip.x - chip.w - 8.0).max(0.0));
        demi.draw(pm, &sym, (sx as f32, (p.acc + 14.5) as f32), 11.5, DOUX, Align::Right);
        // 🛡 the Gardien: one line, or its whole card
        if self.garde_ouverte() {
            self.garde_carte(pm, text, pw, p.gard - GARDE_Y0, mouse);
        } else {
            self.garde_ligne(pm, text, pw, p.gard, survol(Rect::new(X, p.gard, l, 29.0)));
        }
        // Marché / Limite / Stop
        let mut x = X;
        for (k, s) in ["Marché", "Limite", "Stop"].into_iter().enumerate() {
            let r = Rect::new(x, p.types, pilule_w(text, s), 26.0);
            let on = t.tab == k;
            let (c, a) = if on { (BLEU, 0.35) } else { (BLANC, if survol(r) { 0.11 } else { 0.07 }) };
            round_fill(pm, r, 12.0, c, a);
            text.draw(pm, s, (r.cx() as f32, (r.y + 17.5) as f32), 13.0, if on { BLANC } else { CLAIR }, Align::Center);
            x += r.w + 6.0;
        }
        // Disponible, Pouvoir d'achat, Position
        let levier = if t.levier_max > 0.0 {
            format!("Pouvoir d'achat · {}", argos_core::perp::levier_txt(t.levier))
        } else {
            "Pouvoir d'achat".into()
        };
        let base = t.marche.0.split(['-', '/']).next().unwrap_or("").to_string();
        let base = unite_base(&base);
        let (position, pc) = match t.position {
            Some((q, pnl)) if q != 0.0 => (
                format!(
                    "{} {} {base} · {}",
                    if q > 0.0 { "Long" } else { "Short" },
                    qte(q.abs(), t.pas()),
                    usd_signe(pnl)
                ),
                if pnl < 0.0 { BAISSE } else { HAUSSE },
            ),
            _ => ("aucune".to_string(), TEXTE),
        };
        let infos = [
            ("Disponible".to_string(), usd(t.dispo), TEXTE),
            (levier, usd(t.pouvoir), TEXTE),
            ("Position".to_string(), position, pc),
        ];
        for (k, (a, b, c)) in infos.into_iter().enumerate() {
            let y = (p.info + 22.0 * k as f64 + 15.5) as f32;
            text.draw(pm, &a, (16.0, y), 12.5, GRIS, Align::Left);
            demi.draw(pm, &demi.fit(&b, 12.5, l - 130.0), ((droite - 2.0) as f32, y), 12.5, c, Align::Right);
        }
        // Prix (Mid, ⌖), Quantité, Valeur
        let champ = |pm: &mut Pixmap, y: f64, nom: &str, v: &str, unite: &str, fin: f64| {
            let r = Rect::new(X, y, l, CHAMP_H);
            round_fill(pm, r, 14.0, BLANC, if survol(r) { 0.1 } else { 0.07 });
            round_stroke(pm, r, 14.0, crate::jetons::sur_fond((BLANC, 0.06)), 1.0);
            text.draw(pm, nom, ((X + 15.0) as f32, (y + 26.0) as f32), 13.0, GRIS, Align::Left);
            text.draw(pm, unite, ((fin - 34.0) as f32, (y + 26.0) as f32), 12.0, GRIS, Align::Left);
            demi.draw(pm, v, ((fin - 34.0 - 8.0) as f32, (y + 26.5) as f32), 15.0, TEXTE, Align::Right);
        };
        let quote = unite_quote(&t.marche.0);
        if let Some(y) = p.prix {
            let fin = droite - 13.0 - 18.0 - 8.0 - 39.7 - 8.0;
            champ(pm, y, if t.tab == 2 { "Déclenchement" } else { "Prix" }, &prix(t.prix_ordre()), quote, fin);
            let mid = Rect::new(droite - 13.0 - 18.0 - 8.0 - 39.7, y + 9.5, 39.7, 23.0);
            round_fill(pm, mid, 8.0, BLANC, if survol(mid) { 0.12 } else { 0.0 });
            round_stroke(pm, mid, 8.0, crate::jetons::sur_fond((BLANC, 0.28)), 1.0);
            demi.draw(pm, "Mid", (mid.cx() as f32, (mid.y + 16.0) as f32), 12.0, CLAIR, Align::Center);
            // ⌖ (drawn: a circle and its four ticks); blue while it waits for the chart
            let (cx, cy) = (droite - 13.0 - 9.0, y + 21.0);
            let c = if t.cible { BLEU } else { DOUX };
            lines(pm, cx, cy, &[&crate::shapes::arc(6.2, 0.0, 360.0, 28)], c, 1.6);
            lines(
                pm,
                cx,
                cy,
                &[
                    &[(0.0, -8.5), (0.0, -4.5)],
                    &[(0.0, 4.5), (0.0, 8.5)],
                    &[(-8.5, 0.0), (-4.5, 0.0)],
                    &[(4.5, 0.0), (8.5, 0.0)],
                ],
                c,
                1.6,
            );
        }
        let fin = droite - 13.0;
        champ(pm, p.qty, "Quantité", &qte(t.qty, t.pas()), &base, fin);
        let val = if t.qty > 0.0 {
            crate::viewport::format_number(t.valeur(), 2)
        } else {
            "0".into()
        };
        champ(pm, p.val, "Valeur", &val, quote, fin);
        // the % slider, its notches (0, 25, 50, 75, 100) and its box
        let tr = self.ticket_curseur(pw);
        let part = (t.pct / 100.0).clamp(0.0, 1.0);
        round_fill(pm, Rect::new(tr.x, tr.y + 8.0, tr.w, 3.0), 1.5, BLANC, 0.14);
        if part > 0.0 {
            round_fill(pm, Rect::new(tr.x, tr.y + 8.0, tr.w * part, 3.0), 1.5, BLEU, 1.0);
        }
        for k in 0..5 {
            let x = tr.x + tr.w * k as f64 / 4.0 - 1.0;
            round_fill(pm, Rect::new(x, tr.y + 6.0, 2.0, 7.0), 1.0, BLANC, 0.3);
        }
        let kx = tr.x + tr.w * part;
        round_fill(pm, Rect::new(kx - 7.5, tr.y + 3.0, 15.0, 15.0), 7.5, Rgb(0, 0, 0), 0.35);
        round_fill(pm, Rect::new(kx - 7.5, tr.y + 2.0, 15.0, 15.0), 7.5, BLANC, 1.0);
        let pc = Rect::new(droite - 64.0, p.curseur, 64.0, 32.0);
        round_fill(pm, pc, 10.0, BLANC, if survol(pc) { 0.11 } else { 0.07 });
        let unit_w = text.width("%", 14.0) as f64;
        text.draw(pm, "%", ((pc.x + pc.w - 10.0) as f32, (pc.y + 21.0) as f32), 14.0, GRIS, Align::Right);
        demi.draw(
            pm,
            &if (t.pct - t.pct.round()).abs() < 0.05 {
                crate::viewport::format_number(t.pct.round(), 0)
            } else {
                crate::viewport::format_number(t.pct, 1)
            },
            ((pc.x + pc.w - 10.0 - unit_w - 5.0) as f32, (pc.y + 21.0) as f32),
            14.0,
            TEXTE,
            Align::Right,
        );
        // Risque au SL
        if let (Some(y), Some((r, pct))) = (p.risque, t.risque_sl()) {
            let a = "Risque au SL : ";
            let b = usd(r);
            let c = format!(" · {} % du compte", crate::viewport::format_number(pct, 2));
            let yb = (y + 11.0) as f32;
            text.draw(pm, a, (16.0, yb), 11.5, GRIS, Align::Left);
            let x1 = 16.0 + text.width(a, 11.5) as f64;
            demi.draw(pm, &b, (x1 as f32, yb), 11.5, ORANGE, Align::Left);
            let x2 = x1 + demi.width(&b, 11.5) as f64;
            text.draw(pm, &text.fit(&c, 11.5, droite - x2), (x2 as f32, yb), 11.5, GRIS, Align::Left);
        }
        // TP/SL, Reduce-only, the validity
        let case = |pm: &mut Pixmap, x: f64, s: &str, on: bool| -> f64 {
            let b = Rect::new(x, p.opt + opt_h(t) / 2.0 - 7.5, 15.0, 15.0);
            if on {
                round_fill(pm, b, 4.0, BLEU, 1.0);
                lines(pm, b.cx(), b.cy(), &[&[(-3.5, 0.2), (-1.0, 2.8), (3.8, -2.6)]], BLANC, 1.8);
            } else {
                round_stroke(
                    pm,
                    Rect::new(b.x + 0.75, b.y + 0.75, 13.5, 13.5),
                    3.5,
                    crate::jetons::sur_fond((BLANC, 0.4)),
                    1.5,
                );
            }
            text.draw(pm, s, ((x + 22.0) as f32, (p.opt + opt_h(t) / 2.0 + 4.5) as f32), 13.0, CLAIR, Align::Left);
            x + 22.0 + text.width(s, 13.0) as f64 + 14.0
        };
        let x2 = case(pm, X, "TP/SL", t.tpsl && !t.reduce_only);
        case(pm, x2, "Reduce-only", t.reduce_only);
        if t.tab != 0 {
            let limite = t.tab == 1;
            let v = if limite { t.validity } else { Validity::Gtc };
            let r = validite_rect(pw, p.opt, v, limite);
            if limite && survol(r) {
                round_fill(pm, r, 10.0, BLANC, 0.08);
            }
            round_stroke(pm, r, 10.0, crate::jetons::sur_fond((BLANC, 0.3)), 1.0);
            gras.draw(pm, v.name(), ((r.x + 10.0) as f32, (r.y + 19.5) as f32), 13.0, TEXTE, Align::Left);
            if limite {
                chevron(pm, r.x + r.w - 11.0 - 3.0, r.y + 15.5, 3.0, t.menu_validite, PALE);
            }
        }
        // TP and SL (of the side shown), their distance in % and in R
        if let Some(y) = p.tpsl {
            let o = t.ordre(t.sens_affiche());
            let e = o.fill_price(t.prix);
            let w = (l - 7.0) / 2.0;
            for (k, (nom, niveau)) in [("TP", o.tp), ("SL", o.sl)].into_iter().enumerate() {
                let r = Rect::new(X + k as f64 * (w + 7.0), y, w, CHAMP_H);
                round_fill(pm, r, 14.0, BLANC, if survol(r) { 0.1 } else { 0.07 });
                round_stroke(pm, r, 14.0, crate::jetons::sur_fond((BLANC, 0.06)), 1.0);
                text.draw(pm, nom, ((r.x + 15.0) as f32, (y + 26.0) as f32), 13.0, GRIS, Align::Left);
                let Some(n) = niveau.filter(|_| e > 0.0) else { continue };
                let d = (n - e) / e * 100.0;
                let rr = if k == 0 && t.sl_pct > 0.0 { t.tp_pct / t.sl_pct } else { 1.0 };
                demi.draw(pm, &prix(n), ((r.x + r.w - 13.0) as f32, (y + 20.0) as f32), 14.0, TEXTE, Align::Right);
                text.draw(
                    pm,
                    &format!("{} · {}", pct_txt(d), r_txt(rr)),
                    ((r.x + r.w - 13.0) as f32, (y + 34.0) as f32),
                    10.5,
                    GRIS,
                    Align::Right,
                );
            }
        }
        // VENDRE / ACHETER in the flow (📱 the phone keeps them at the bottom: `ticket_boutons_draw`)
        if let Some(y) = p.go {
            self.boutons_draw(pm, text, boutons(pw, y, 0.0), mouse);
        }
        // Marge, Frais | Max, Glissement
        let e = &t.estimate;
        let rien = t.qty <= 0.0;
        let m = |v: Money| if rien { "—".to_string() } else { crate::simulateur::argent(v, false) };
        let cw = (l - 22.0 - 12.0) / 2.0;
        let cols = [
            (X + 3.0, [("Marge", m(e.margin)), ("Frais", m(e.fees))]),
            (
                X + 3.0 + cw + 22.0 + 6.0,
                [("Max", format!("{} {base}", qte_max(e.max_qty, t.pas()))), ("Glissement", m(e.slippage))],
            ),
        ];
        for (x, rangs) in cols {
            for (k, (a, b)) in rangs.into_iter().enumerate() {
                let y = (p.cout + 20.0 * k as f64 + 14.0) as f32;
                text.draw(pm, a, (x as f32, y), 12.0, GRIS, Align::Left);
                demi.draw(pm, &b, ((x + cw) as f32, y), 12.0, CLAIR, Align::Right);
            }
        }
        // Compte, folded on one line or unfolded
        self.compte_draw(pm, text, pw, p.compte, survol(Rect::new(X, p.compte, l, COMPTE_H)));
        // the links, then ⚙ Tous les réglages
        let [pos, carnet] = liens_rects(text, p.liens);
        let yl = (p.liens + 12.0) as f32;
        let mut x = 18.0;
        for (s, c) in [("Positions et ordres", DOUX), ("›", PALE), (AVANT_CARNET, DOUX), ("›", PALE)] {
            text.draw(pm, s, (x as f32, yl), 12.5, c, Align::Left);
            x += text.width(s, 12.5) as f64 + 18.0;
        }
        for r in [pos, carnet] {
            if survol(r) {
                round_fill(pm, r, 9.0, BLANC, 0.05);
            }
        }
        let tous = Rect::new(X, p.tous, l, 33.0);
        fond(pm, tous, 12.0, if survol(tous) { 0.09 } else { 0.05 });
        text.draw(pm, "⚙\u{fe0f}", ((X + 12.0) as f32, (p.tous + 21.5) as f32), 11.7, DOUX, Align::Left);
        text.draw(
            pm,
            "Tous les réglages",
            ((X + 12.0 + 14.0 + 7.0) as f32, (p.tous + 21.0) as f32),
            12.5,
            DOUX,
            Align::Left,
        );
        text.draw(pm, "›", ((droite - 12.0) as f32, (p.tous + 21.0) as f32), 12.5, PALE, Align::Right);
        // 🔒 the ticket in pause while the day is locked (the orders are refused by the Gardien anyway)
        if self.cell_garde().verrou() {
            let y0 = p.types - 4.0;
            round_fill(pm, Rect::new(X - 4.0, y0, l + 8.0, p.cout + 44.0 - y0), 14.0, Rgb(0, 0, 0), 0.55);
        }
        // the validity menu, over the rest
        if t.menu_validite {
            let m = menu_rect(pw, p.opt);
            round_fill(pm, Rect::new(m.x, m.y + 8.0, m.w, m.h), 14.0, Rgb(0, 0, 0), 0.35);
            round_fill(pm, m, 14.0, MENU, 1.0);
            round_stroke(pm, m, 14.0, crate::jetons::sur_fond((BLANC, 0.16)), 1.0);
            for (k, v) in Validity::ALL.into_iter().enumerate() {
                let r = menu_ligne(m, k);
                if v == t.validity {
                    round_fill(pm, r, 9.0, BLEU, 0.18);
                } else if survol(r) {
                    round_fill(pm, r, 9.0, BLANC, 0.06);
                }
                gras.draw(pm, v.name(), ((r.x + 10.0) as f32, (r.y + 22.0) as f32), 13.0, TEXTE, Align::Left);
                text.draw(pm, v.detail(), ((r.x + r.w - 10.0) as f32, (r.y + 22.0) as f32), 12.0, PALE, Align::Right);
            }
        }
    }

    /// 🛡 The Gardien on one line (`y`): its state, and › to open its card.
    fn garde_ligne(&self, pm: &mut Pixmap, text: &Text, pw: f64, y: f64, survol: bool) {
        let etat = self.cell_garde().ligne();
        let r = Rect::new(X, y, large(pw), 29.0);
        round_fill(pm, r, 12.0, BLANC, if survol { 0.09 } else { 0.05 });
        text.draw(pm, "🛡", ((X + 11.0) as f32, (y + 19.5) as f32), 11.7, DOUX, Align::Left);
        let demi = Text::graisse(Graisse::Medium);
        let x1 = X + 11.0 + 14.0 + 7.0;
        demi.draw(pm, "Gardien PropFirm", (x1 as f32, (y + 19.0) as f32), 12.5, CLAIR, Align::Left);
        let x2 = x1 + demi.width("Gardien PropFirm", 12.5) as f64 + 7.0;
        text.draw(
            pm,
            &text.fit(&format!("· {etat}"), 12.5, r.x + r.w - 30.0 - x2),
            (x2 as f32, (y + 19.0) as f32),
            12.5,
            DOUX,
            Align::Left,
        );
        text.draw(pm, "›", ((r.x + r.w - 11.0) as f32, (y + 19.0) as f32), 12.5, PALE, Align::Right);
    }

    /// 📒 The Compte block at `y`: folded (equity · P&L latent) or its 9 rows.
    fn compte_draw(&self, pm: &mut Pixmap, text: &Text, pw: f64, y: f64, survol: bool) {
        let t = &self.trading;
        let c = &t.compte;
        let l = large(pw);
        let h = if t.compte_ouvert { COMPTE_OUVERT_H } else { COMPTE_H };
        round_fill(pm, Rect::new(X, y, l, h), 14.0, BLANC, if survol { 0.08 } else { 0.05 });
        let demi = Text::graisse(Graisse::Medium);
        demi.draw(pm, "Compte", ((X + 12.0) as f32, (y + 23.0) as f32), 13.5, TEXTE, Align::Left);
        chevron(pm, X + l - 16.0, y + 18.0, 4.0, t.compte_ouvert, PALE);
        let ouvert = c.positions > 0;
        let latent = if ouvert { usd_signe(c.unrealized.to_f64()) } else { "—".into() };
        if !t.compte_ouvert {
            let x = X + 12.0 + demi.width("Compte", 13.5) as f64 + 8.0;
            let s = format!("{} · P&L latent {latent}", usd(c.equity.to_f64()));
            text.draw(
                pm,
                &text.fit(&s, 12.0, X + l - 32.0 - x),
                (x as f32, (y + 23.0) as f32),
                12.0,
                GRIS,
                Align::Left,
            );
            return;
        }
        let tiret = || "—".to_string();
        let rangs: [Option<(&str, String, Rgb)>; 11] = [
            Some(("Fonds propres", usd(c.equity.to_f64()), TEXTE)),
            Some(("Disponible pour trader", usd(c.available.to_f64()), TEXTE)),
            Some(("Marge utilisée", usd(c.margin_used.to_f64()), TEXTE)),
            Some((
                "Ratio de marge",
                c.margin_ratio().map_or_else(tiret, |r| format!("{} %", crate::viewport::format_number(r, 2))),
                TEXTE,
            )),
            None,
            Some(("P&L latent", latent, TEXTE)),
            Some((
                "Funding du jour",
                if t.funding_jour != 0.0 {
                    // paid: negative for the account
                    usd_signe(-t.funding_jour)
                } else {
                    tiret()
                },
                TEXTE,
            )),
            None,
            Some(("Levier", format!("{}×", crate::viewport::format_number(c.leverage(), 2)), TEXTE)),
            Some(("Long / short", format!("{} / {}", usd(c.long.to_f64()), usd(c.short.to_f64())), TEXTE)),
            Some((
                "Marge de maintien",
                if c.maintenance > Money::ZERO { usd(c.maintenance.to_f64()) } else { tiret() },
                TEXTE,
            )),
        ];
        let mut yy = y + 36.0;
        for r in rangs {
            match r {
                None => {
                    round_fill(pm, Rect::new(X + 12.0, yy + 4.0, l - 24.0, 1.0), 0.5, BLANC, 0.07);
                    yy += 9.0;
                }
                Some((a, b, col)) => {
                    text.draw(pm, a, ((X + 12.0) as f32, (yy + 16.0) as f32), 12.5, GRIS, Align::Left);
                    demi.draw(pm, &b, ((X + l - 12.0) as f32, (yy + 16.0) as f32), 12.5, col, Align::Right);
                    yy += 23.0;
                }
            }
        }
    }

    /// VENDRE / SHORT and ACHETER / LONG at their places: with the size and the price; pale without a size (« Entre
    /// une taille »), or reduce-only without a position on the other side (« Reduce-only : rien à réduire »).
    pub(super) fn boutons_draw(&self, pm: &mut Pixmap, text: &Text, places: [Rect; 2], mouse: Option<(f64, f64)>) {
        let t = &self.trading;
        let base = unite_base(t.marche.0.split(['-', '/']).next().unwrap_or(""));
        let sous = if t.tab == 0 {
            format!("{} {base} au marché", qte(t.qty, t.pas()))
        } else {
            format!("{} {base} à {}", qte(t.qty, t.pas()), prix(t.prix_ordre()))
        };
        for (buy, r) in [false, true].into_iter().zip(places) {
            let (titre, c) = if buy { ("ACHETER / LONG", ACHETER) } else { ("VENDRE / SHORT", VENDRE) };
            // 🔻 reduce-only: the side with a position on the other side closes or reduces it, the other has nothing
            let reste = if buy { t.reducible.0 } else { t.reducible.1 };
            let (ligne, pale) = if t.reduce_only && reste <= 0.0 {
                ("Reduce-only : rien à réduire".to_string(), true)
            } else if t.qty <= 0.0 {
                ("Entre une taille".to_string(), true)
            } else if t.reduce_only {
                let de = if buy { "short" } else { "long" };
                (format!("{} ta position {de}", if t.qty >= reste { "ferme" } else { "réduit" }), false)
            } else {
                (sous.clone(), false)
            };
            let sur = !pale && mouse.is_some_and(|(x, y)| r.contains(x, y));
            bouton(pm, text, r, (titre, &ligne), c, sur, pale);
        }
    }

    /// 📱 Draws VENDRE / ACHETER kept at the bottom of the phone's sheet (`h` high), over the ticket, and the line above.
    pub fn ticket_boutons_draw(&self, pm: &mut Pixmap, text: &Text, pw: f64, h: f64, mouse: Option<(f64, f64)>, fond: Rgb) {
        let Some([v, a]) = self.ticket_boutons_colles(pw, h) else { return };
        crate::fill(pm, 0.0, h - BAS_COLLE, pw, BAS_COLLE, fond, 1.0);
        crate::fill(pm, 0.0, h - BAS_COLLE, pw, 1.0, BLANC, 0.08);
        self.boutons_draw(pm, text, [v, a], mouse);
        // 🔒 the day locked: in pause, like the rest of the ticket
        if self.cell_garde().verrou() {
            round_fill(
                pm,
                Rect::new(X - 4.0, v.y - 4.0, a.x + a.w - X + 8.0, BOUTONS_H + 8.0),
                16.0,
                Rgb(0, 0, 0),
                0.55,
            );
        }
    }
}

/// VENDRE (left) and ACHETER (right) at `y`, 8 apart; `reserve`: room kept at their right (📱 the Argos AI round).
fn boutons(pw: f64, y: f64, reserve: f64) -> [Rect; 2] {
    let w = ((large(pw) - reserve.max(0.0)).max(16.0) - 8.0) / 2.0;
    [Rect::new(X, y, w, BOUTONS_H), Rect::new(X + w + 8.0, y, w, BOUTONS_H)]
}

/// A VENDRE / ACHETER button: its colour, the bold title and the line under it (white at .9); `pale`: the whole
/// button at .45, letters and colour together (the mockup's `opacity`).
fn bouton(pm: &mut Pixmap, text: &Text, r: Rect, (titre, ligne): (&str, &str), c: Rgb, sur: bool, pale: bool) {
    if pale {
        if let Some(mut sub) = crate::px::new(r.w.ceil() as u32, r.h.ceil() as u32) {
            bouton(&mut sub, text, Rect::new(0.0, 0.0, r.w, r.h), (titre, ligne), c, false, false);
            crate::px::put_alpha(pm, r.x, r.y, &sub, 0.45);
        }
        return;
    }
    let gras = Text::graisse(Graisse::Bold);
    round_fill(pm, r, 16.0, c, if sur { 0.9 } else { 1.0 });
    // (letter-spacing .02em of the mockup)
    let ecart = 0.26;
    let w = gras.width(titre, 13.0) as f64 + ecart * (titre.chars().count() as f64 - 1.0);
    gras.draw_espace(pm, titre, ((r.cx() - w / 2.0) as f32, (r.y + 24.0) as f32), 13.0, BLANC, ecart as f32);
    let l = text.fit(ligne, 10.8, r.w - 12.0);
    text.draw(pm, &l, (r.cx() as f32, (r.y + 40.0) as f32), 10.8, melange(BLANC, c, 0.9), Align::Center);
}

/// `a` laid over `b` at `k` (0 to 1).
fn melange(a: Rgb, b: Rgb, k: f64) -> Rgb {
    let m = |x: u8, y: u8| (x as f64 * k + y as f64 * (1.0 - k)).round() as u8;
    Rgb(m(a.0, b.0), m(a.1, b.1), m(a.2, b.2))
}

/// A small chevron `2 × demi` wide: ⌄ (closed) or ⌃ (open), centred on (`cx`, `cy`).
fn chevron(pm: &mut Pixmap, cx: f64, cy: f64, demi: f32, haut: bool, c: Rgb) {
    let s: f32 = if haut { -1.0 } else { 1.0 };
    let v = demi / 2.0 * s;
    lines(pm, cx, cy, &[&[(-demi, -v), (0.0, v), (demi, -v)]], c, 1.4);
}

/// The base unit of a market's name (« BTCUSDT.P » → « BTC », « ETH » → « ETH »).
fn unite_base(sym: &str) -> String {
    let s = sym.trim_end_matches(".P");
    for q in ["USDT", "USDC", "USD", "PERP"] {
        if let Some(b) = s.strip_suffix(q).filter(|b| !b.is_empty()) {
            return b.to_string();
        }
    }
    s.to_string()
}

/// The quote unit of a market's name (« BTCUSDT.P » → « USDT »; else « USD »).
fn unite_quote(sym: &str) -> &'static str {
    let s = sym.trim_end_matches(".P");
    if s.ends_with("USDT") {
        "USDT"
    } else if s.ends_with("USDC") {
        "USDC"
    } else {
        "USD"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A ticket like the mockup's: BTC at 60 057,9, 100 000 $ available, ×10.
    fn maquette() -> Panels {
        let mut p = Panels::default();
        let t = &mut p.trading;
        t.cell = true;
        (t.prix, t.px, t.tab, t.balance) = (60_057.9, 60_057.9, 1, 100_000.0);
        (t.dispo, t.pouvoir, t.levier_max, t.levier) = (100_000.0, 1_000_000.0, 100.0, 10.0);
        t.marche = ("BTCUSDT.P".into(), "Binance".into());
        t.set_pct(5.0);
        p
    }

    /// 🎫 The slider gives the quantity (5 % of 1 000 000 $ at 60 057,9: 0,8325 BTC, step 0,0001) and the value;
    /// a typed quantity moves the slider; TP / SL at 2 % / 1 % of the side shown, typed prices become distances;
    /// a price on the wrong side is refused with its message.
    #[test]
    fn curseur_quantite_et_tp_sl() {
        let mut p = maquette();
        let t = &mut p.trading;
        assert_eq!(t.qty, 0.8325);
        assert_eq!(format!("{:.2}", t.valeur()), "49998.20");
        t.saisir(Champ::Quantite, "1,5").unwrap();
        assert_eq!((t.qty, format!("{:.2}", t.pct)), (1.5, "9.01".into()));
        t.saisir(Champ::Valeur, "6 005,79").unwrap();
        assert_eq!(t.qty, 0.1);
        t.saisir(Champ::Pct, "5 %").unwrap();
        assert_eq!(t.qty, 0.8325);
        // a Limit at the last price: the last side used (buy by default)
        assert!(t.sens_affiche());
        let o = t.ordre(true);
        assert_eq!((prix(o.tp.unwrap()), prix(o.sl.unwrap())), ("61\u{202f}259,1".into(), "59\u{202f}457,3".into()));
        let (r, pct) = t.risque_sl().unwrap();
        // (from the levels shown: 0,8325 × 600,6 = 500,00 $, as the mockup)
        assert_eq!((format!("{r:.2}"), format!("{pct:.2}")), ("500.00".into(), "0.50".into()));
        // typed TP / SL prices become distances; the wrong side is refused
        t.saisir(Champ::Sl, "59 457,3").unwrap();
        t.saisir(Champ::Tp, "62 000").unwrap();
        assert_eq!(format!("{:.3} {:.3}", t.tp_pct, t.sl_pct), "3.234 1.000");
        assert_eq!(t.saisir(Champ::Sl, "61 000"), Err("le SL doit être sous l'entrée".into()));
        assert_eq!(t.saisir(Champ::Tp, "abc"), Err("« abc » : écris un nombre".into()));
        // a sell shows TP under the price and SL above it
        t.px = 61_000.0;
        assert!(!t.sens_affiche(), "a Limit above the price sells");
        let o = t.ordre(false);
        assert!(o.tp.unwrap() < 61_000.0 && o.sl.unwrap() > 61_000.0);
        // reduce-only: no TP / SL, the slider stands for the position
        t.reduce_only = true;
        t.reducible = (0.0, 0.8325);
        t.set_pct(100.0);
        assert_eq!((t.qty, t.ordre(false).sl), (0.8325, None));
        t.set_qty(5.0);
        assert_eq!(t.qty, 0.8325, "never more than the position");
    }

    /// 🎫 The places of the mockup (PC, 420 wide, Limite, TP/SL, Compte folded): every part where the mockup has it.
    #[test]
    fn places_de_la_maquette() {
        crate::actions::set_tactile(false);
        let p = maquette();
        let pl = p.ticket_places(420.0);
        assert_eq!((pl.acc, pl.gard, pl.types, pl.info), (125.5, 155.5, 194.5, 230.5));
        assert_eq!((pl.prix, pl.qty, pl.val, pl.curseur), (Some(303.5), 352.5, 401.5, 454.5));
        assert_eq!((pl.risque, pl.opt, pl.tpsl, pl.go), (Some(493.5), 518.5, Some(556.5), Some(610.5)));
        assert_eq!((pl.cout, pl.compte, pl.liens, pl.tous, pl.fin), (673.5, 724.5, 771.5, 796.5, 843.5));
        // Marché without TP/SL nor size (mockup's state 3): the fields go up, no risk line
        let mut p = maquette();
        (p.trading.tab, p.trading.tpsl) = (0, false);
        p.trading.set_pct(0.0);
        let pl = p.ticket_places(420.0);
        assert_eq!(
            (pl.prix, pl.qty, pl.curseur, pl.risque, pl.opt, pl.go),
            (None, 303.5, 405.5, None, 448.5, Some(476.5))
        );
    }

    /// 🎫 Every touch of the ticket answers: Mid, ⌖, the fields, the slider, TP/SL, Reduce-only, the validity menu
    /// (its rows, and a touch beside closes it), VENDRE / ACHETER, Compte, the links, ⚙ Tous les réglages.
    #[test]
    fn chaque_touche_du_ticket() {
        use super::super::{Kind as PK, PanelClick};
        crate::actions::set_tactile(false);
        let pw = 420.0;
        let mut p = maquette();
        let h = 900.0;
        let at = |p: &mut Panels, hit: Hit| {
            let r = p.cell_zones(pw).into_iter().find(|(_, x)| *x == hit).unwrap_or_else(|| panic!("{hit:?}")).0;
            p.click(PK::Trading, r.cx(), r.cy(), pw, h)
        };
        assert_eq!(at(&mut p, Hit::Cell(Cell::Champ(Champ::Prix))), PanelClick::Saisir(Champ::Prix));
        p.trading.mid = 60_050.0;
        assert_eq!(at(&mut p, Hit::Cell(Cell::Mid)), PanelClick::Changed);
        assert_eq!(p.trading.px, 60_050.0, "Mid");
        assert_eq!(at(&mut p, Hit::Cell(Cell::Cible)), PanelClick::Cible);
        assert!(p.trading.cible);
        for c in [Champ::Quantite, Champ::Valeur, Champ::Pct, Champ::Tp, Champ::Sl] {
            assert_eq!(at(&mut p, Hit::Cell(Cell::Champ(c))), PanelClick::Saisir(c));
        }
        // the slider: a touch at 3/4 of the track (then it follows the finger: the window holds it)
        let tr = p.ticket_curseur(pw);
        assert!(p.sur_curseur(tr.x + tr.w * 0.75, tr.cy(), pw, h) && !p.sur_curseur(tr.x, tr.y - 30.0, pw, h));
        assert_eq!(p.click(PK::Trading, tr.x + tr.w * 0.75, tr.cy(), pw, h), PanelClick::Curseur);
        assert_eq!(p.trading.pct, 75.0);
        // TP/SL off, then Reduce-only on (TP/SL hidden)
        at(&mut p, Hit::Cell(Cell::TpSl));
        assert!(!p.trading.tpsl);
        at(&mut p, Hit::Cell(Cell::Reduire));
        assert!(p.trading.reduce_only && p.ticket_places(pw).tpsl.is_none());
        at(&mut p, Hit::Cell(Cell::Reduire));
        // the validity menu: open, pick IOC; open, a touch beside closes it
        at(&mut p, Hit::Cell(Cell::Validite));
        assert!(p.trading.menu_validite);
        assert_eq!(at(&mut p, Hit::Cell(Cell::Duree(1))), PanelClick::Changed);
        assert_eq!((p.trading.validity, p.trading.menu_validite), (Validity::Ioc, false));
        at(&mut p, Hit::Cell(Cell::Validite));
        assert_eq!(p.click(PK::Trading, 30.0, 340.0, pw, h), PanelClick::Changed);
        assert!(!p.trading.menu_validite && p.trading.validity == Validity::Ioc);
        assert_eq!(at(&mut p, Hit::Cell(Cell::Passer(false))), PanelClick::Passer(false));
        assert_eq!(at(&mut p, Hit::Cell(Cell::Passer(true))), PanelClick::Passer(true));
        at(&mut p, Hit::Cell(Cell::Compte));
        assert!(p.trading.compte_ouvert);
        assert_eq!(p.ticket_places(pw).compte_h, COMPTE_OUVERT_H);
        assert_eq!(at(&mut p, Hit::Cell(Cell::Carnet)), PanelClick::Carnet);
        at(&mut p, Hit::Cell(Cell::Gardien));
        assert!(p.trading.garde_ouverte, "the Gardien's card");
        at(&mut p, Hit::Cell(Cell::Positions));
        assert!(p.trading.positions_vue);
        p.trading.positions_vue = false;
        at(&mut p, Hit::Cell(Cell::Reglages));
        assert!(p.trading.options, "⚙ Tous les réglages: the old full ticket");
    }

    /// 🛡 The Gardien on: unticking TP/SL is refused (the SL stays compulsory, it checks the max risk).
    #[test]
    fn gardien_garde_le_sl() {
        use super::super::{Kind as PK, PanelClick};
        crate::actions::set_tactile(false);
        let mut p = maquette();
        p.trading.guardian = true;
        let r = p.cell_zones(420.0).into_iter().find(|(_, x)| *x == Hit::Cell(Cell::TpSl)).unwrap().0;
        assert!(matches!(p.click(PK::Trading, r.cx(), r.cy(), 420.0, 900.0), PanelClick::Soon(m) if m.contains("SL reste obligatoire")));
        assert!(p.trading.tpsl);
    }

    /// 📸 ARGOS_TICKET_PNG=dossier : the mockup's states drawn by the app, to lay beside it (PC 420 × 830: Limite
    /// with TP/SL; the validity menu open; a long, reduce-only, Compte unfolded; at market without a size; phone 390
    /// × 590 with VENDRE / ACHETER at the bottom).
    #[test]
    fn images_des_etats_de_la_maquette() {
        let Ok(dir) = std::env::var("ARGOS_TICKET_PNG") else { return };
        crate::px::set_scale(2.0);
        let (text, th) = (Text::default(), crate::Theme::default());
        let image = |p: &Panels, (w, h): (f64, f64), off: f64, nom: &str| {
            let ch = p.content_h(super::super::Kind::Trading, w).max(h);
            let mut sub = crate::px::new(w as u32, ch as u32).unwrap();
            crate::fill(&mut sub, 0.0, 0.0, w, ch, th.chrome, 1.0);
            p.draw(super::super::Kind::Trading, &mut sub, &text, (w, ch), None, &th);
            let mut pm = crate::px::new(w as u32, h as u32).unwrap();
            crate::px::put(&mut pm, 0.0, -off, &sub);
            p.trading_fixe_draw(&mut pm, &text, (w, h), None, th.chrome);
            pm.save_png(format!("{dir}/ticket-{nom}.png")).unwrap();
        };
        let estimer = |t: &mut Trading, taker: bool| {
            let v = t.valeur();
            t.estimate = simu::Estimate {
                margin: Money::from_f64(if t.reduce_only { 0.0 } else { v / t.levier }),
                fees: Money::from_f64(v * if taker { 0.00045 } else { 0.0002 }),
                slippage: Money::from_f64(if taker { v * 0.0001 } else { 0.0 }),
                max_qty: if t.reduce_only {
                    t.reducible.1
                } else {
                    simu::round_qty(t.pouvoir / t.prix, t.pas())
                },
            };
        };
        crate::actions::set_tactile(false);
        let mut p = maquette();
        estimer(&mut p.trading, false);
        image(&p, (420.0, 830.0), 0.0, "pc");
        p.trading.tpsl = false;
        p.trading.menu_validite = true;
        image(&p, (420.0, 830.0), 0.0, "etat-1-duree");
        let mut p = maquette();
        let t = &mut p.trading;
        (t.tab, t.tpsl, t.reduce_only, t.compte_ouvert) = (0, false, true, true);
        (t.dispo, t.pouvoir, t.reducible, t.position) = (95_072.18, 950_721.8, (0.0, 0.8325), Some((0.8325, 83.25)));
        t.set_pct(5.0);
        t.compte = argos_core::portfolio::AccountSummary {
            equity: Money::from_f64(100_072.0),
            available: Money::from_f64(95_072.18),
            margin_used: Money::from_f64(4_999.82),
            maintenance: Money::from_f64(250.41),
            unrealized: Money::from_f64(83.25),
            long: Money::from_f64(50_081.45),
            short: Money::ZERO,
            positions: 1,
        };
        t.funding_jour = 1.25;
        estimer(t, true);
        image(&p, (420.0, 830.0), 330.0, "etat-2-compte");
        let mut p = maquette();
        (p.trading.tab, p.trading.tpsl) = (0, false);
        p.trading.set_pct(0.0);
        estimer(&mut p.trading, true);
        image(&p, (420.0, 830.0), 0.0, "etat-3-sans-taille");
        crate::actions::set_tactile(true);
        let mut p = maquette();
        estimer(&mut p.trading, false);
        image(&p, (390.0, 590.0), 0.0, "cell-haut");
        p.trading.compte_ouvert = true;
        image(&p, (390.0, 590.0), 476.0, "cell-bas");
        crate::actions::set_tactile(false);
        crate::px::set_scale(1.0);
    }

    /// ⚡ « ⚙ Tous les réglages » (the old full ticket): its leverage cell is the real leverage; touched, the market's
    /// next one (after the largest, the smallest again); on spot, there is no leverage.
    #[test]
    fn levier_sous_les_reglages() {
        use super::super::{Kind as PK, PanelClick};
        let (pw, h) = (360.0, 900.0);
        let mut p = Panels::default();
        (p.trading.options, p.trading.levier_max) = (true, 20.0);
        let levier = |p: &mut Panels| {
            let r = p.layout(PK::Trading, pw, h).into_iter().find(|(_, x)| *x == Hit::Margin(1)).unwrap().0;
            p.click(PK::Trading, r.cx(), r.cy(), pw, h)
        };
        let mut vus = vec![];
        for _ in 0..4 {
            assert_eq!(levier(&mut p), PanelClick::Changed);
            vus.push(p.trading.levier);
        }
        assert_eq!(vus, vec![20.0, 1.0, 5.0, 10.0], "×10 → ×20 (the market's largest) → ×1 → ×5 → ×10");
        p.trading.levier_max = 0.0;
        assert!(matches!(levier(&mut p), PanelClick::Soon(s) if s.contains("comptant")));
        assert_eq!(p.trading.levier, 10.0, "spot: unchanged");
    }

    /// 🔻 VENDRE / ACHETER in reduce-only with a long: VENDRE « ferme ta position long », ACHETER « rien à réduire »;
    /// no size: « Entre une taille » (both pale). The base unit and the quote of the market's name.
    #[test]
    fn boutons_et_unites() {
        let _ = crate::text::ecrits();
        let mut p = maquette();
        p.trading.reduce_only = true;
        p.trading.reducible = (0.0, 0.8325);
        p.trading.set_pct(100.0);
        let mut pm = crate::px::new(420, 700).unwrap();
        p.boutons_draw(&mut pm, &Text::default(), boutons(420.0, 100.0, 0.0), None);
        let e = crate::text::ecrits();
        assert!(
            e.iter().any(|s| s == "ferme ta position long") && e.iter().any(|s| s == "Reduce-only : rien à réduire"),
            "{e:?}"
        );
        p.trading.reduce_only = false;
        p.trading.set_pct(0.0);
        p.boutons_draw(&mut pm, &Text::default(), boutons(420.0, 100.0, 0.0), None);
        assert_eq!(crate::text::ecrits().iter().filter(|s| *s == "Entre une taille").count(), 2);
        assert_eq!(
            (unite_base("BTCUSDT.P"), unite_base("ETH"), unite_base("XAUUSD")),
            ("BTC".into(), "ETH".into(), "XAU".into())
        );
        assert_eq!((unite_quote("BTCUSDT.P"), unite_quote("BTC")), ("USDT", "USD"));
        assert_eq!((qte_max(16.650_58, 0.0001), qte_max(0.8325, 0.0001)), ("16,65".into(), "0,8325".into()));
        assert_eq!((r_txt(2.0), r_txt(1.5)), ("2R".into(), "1,5R".into()));
    }

    /// 📱 VENDRE / ACHETER kept at the bottom of the phone's sheet stop before the Argos AI round (the whole screen's
    /// capture of 22 Nov showed it over ACHETER): its touch is its own, the buttons stay whole and side by side.
    #[test]
    fn boutons_colles_laissent_le_rond_ai() {
        crate::actions::set_tactile(true);
        let (pw, h) = (402.0, 700.0);
        let mut p = maquette();
        let [_, a] = p.ticket_boutons_colles(pw, h).unwrap();
        assert_eq!(a.x + a.w, X + large(pw), "nothing there: the whole width");
        p.trading.place_ai = 64.0;
        let [v, a] = p.ticket_boutons_colles(pw, h).unwrap();
        assert!(a.x + a.w <= pw - 64.0 && (v.w - a.w).abs() < 1e-9 && a.x - v.x - v.w == 8.0, "{v:?} {a:?}");
        // a touch on ACHETER, now narrower, buys
        assert_eq!(p.click_colle(a.cx(), a.cy(), pw, h), Some(super::super::PanelClick::Passer(true)));
        crate::actions::set_tactile(false);
    }

    /// 🎫 The account's chip: its name as it is, the default « Démo Argos 100K » once (the whole screen's capture of
    /// 22 Nov showed « Démo Démo Argos 100K »).
    #[test]
    fn nom_du_compte_une_seule_fois() {
        let mut p = maquette();
        p.trading.account = argos_core::portfolio::Portfolio::default().name;
        assert_eq!(nom_compte(&p.trading), "Démo Argos 100K");
        p.trading.account = "Défi 50K".into();
        assert_eq!(nom_compte(&p.trading), "Défi 50K");
        p.trading.account.clear();
        assert_eq!(nom_compte(&p.trading), "Démo Argos 100K");
    }
}
