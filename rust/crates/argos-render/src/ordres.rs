//! 💼 Ordres et positions démo sur la chart, comme TradingView : pour chaque position ou ordre en attente du symbole,
//! une ligne d'entrée avec son étiquette (`qté | P&L` ou `qté | Limite d'achat` et ✕), une ligne TP (verte) et une
//! ligne SL (orange) avec ce qu'elles rapportent ou coûtent si elles sont touchées.
//! SL et TP se déplacent à la souris ; du mauvais côté de l'entrée, une pastille rouge « ! » et le message s'affichent.
//! Un brouillon (ordre préparé sur la chart) a son bouton « Buy » / « Sell » : un clic envoie l'ordre au compte démo.
//!
//! Une seule table de zones ([`zones`]) sert au dessin et aux clics. Les montants sont en centimes.

use crate::palette as pal;
use crate::shapes::{round_fill, round_stroke, Rect};
use crate::text::{Align, Rgb, Text};
use argos_core::Money;
use tiny_skia::Pixmap;

/// Une position ou un ordre en attente du symbole de la chart.
#[derive(Debug, Clone, PartialEq)]
pub struct OrdreChart {
    pub id: u64,
    /// Ordre en attente : « Limite » ou « Stop » ; position : `None`.
    pub attente: Option<&'static str>,
    pub buy: bool,
    pub qty: f64,
    pub entree: f64,
    pub sl: Option<f64>,
    pub tp: Option<f64>,
    /// P&L latent (positions).
    pub pnl: Option<Money>,
    /// Ce que le SL coûte et ce que le TP rapporte s'ils sont touchés.
    pub perte_sl: Money,
    pub gain_tp: Money,
    /// ⚡ 💀 Perpétuel : le prix de liquidation et le levier (ligne « ×10 | Liquidation » en tirets rouges).
    pub liq: Option<(f64, f64)>,
}

/// Un ordre préparé sur la chart, pas encore envoyé.
#[derive(Debug, Clone, PartialEq)]
pub struct Brouillon {
    pub buy: bool,
    pub entree: f64,
    pub sl: f64,
    pub tp: f64,
    pub qty: f64,
    /// « Limite » ou « Stop » (selon le prix d'entrée par rapport au dernier prix), ou « Marché ».
    pub genre: &'static str,
    pub perte_sl: Money,
    pub gain_tp: Money,
}

/// 🎫 The order the Trading ticket prepares (module 31), shown on the chart before it leaves: its Limite / Stop line
/// (none at market), its TP and SL with what they bring or cost, all dragged with the mouse / the finger.
#[derive(Debug, Clone, PartialEq)]
pub struct Apercu {
    pub buy: bool,
    /// « Limite » or « Stop » and the order's price; `None` at market.
    pub entree: Option<(&'static str, f64)>,
    /// The price the order fills at (TP and SL count from it).
    pub depart: f64,
    pub sl: Option<f64>,
    pub tp: Option<f64>,
    /// 0: no size yet (the TP and SL lines say their distance in % instead of an amount).
    pub qty: f64,
}

impl Apercu {
    /// 📏 The prices the zoom auto keeps in view: the Limite / Stop line, the TP, the SL.
    pub fn prix_en_vue(&self) -> Vec<f64> {
        self.entree
            .map(|(_, p)| p)
            .into_iter()
            .chain(self.tp)
            .chain(self.sl)
            .filter(|p| p.is_finite() && *p > 0.0)
            .collect()
    }
}

/// Ce qu'on attrape ou touche sur la chart.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Prise {
    Entree(u64),
    Sl(u64),
    Tp(u64),
    /// ✕ : annuler l'ordre en attente ou fermer la position.
    Fermer(u64),
    BrEntree,
    BrSl,
    BrTp,
    /// Bouton « Buy » / « Sell » du brouillon : envoyer l'ordre.
    BrEnvoyer,
    /// ✕ du brouillon.
    BrAnnuler,
    /// ⚡ 💀 La ligne de liquidation d'une position : montrée seulement (elle ne s'attrape pas).
    Liq(u64),
    /// 🎫 The lines of the ticket's order ([`Apercu`]): its price, its SL, its TP.
    TkEntree,
    TkSl,
    TkTp,
}

impl Prise {
    /// Une ligne qu'on fait glisser (pas un bouton).
    pub fn glisse(self) -> bool {
        matches!(
            self,
            Prise::Entree(_) | Prise::Sl(_) | Prise::Tp(_) | Prise::BrEntree | Prise::BrSl | Prise::BrTp | Prise::TkEntree | Prise::TkSl | Prise::TkTp
        )
    }

    /// Le stop (`Some(true)`), l'objectif (`Some(false)`) ou l'entrée (`None`).
    pub fn niveau(self) -> Option<bool> {
        match self {
            Prise::Sl(_) | Prise::BrSl | Prise::TkSl => Some(true),
            Prise::Tp(_) | Prise::BrTp | Prise::TkTp => Some(false),
            _ => None,
        }
    }
}

/// Ce que la chart montre.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct VueOrdres {
    pub ordres: Vec<OrdreChart>,
    pub brouillon: Option<Brouillon>,
    /// 🎫 The order of the open Trading ticket.
    pub ticket: Option<Apercu>,
    /// Ligne en cours de glisser du mauvais côté, et son message (pastille rouge « ! »).
    pub erreur: Option<(Prise, &'static str)>,
}

/// Une ligne à dessiner.
#[derive(Debug, Clone, PartialEq)]
struct Ligne {
    prix: f64,
    couleur: Rgb,
    /// Ligne pleine (position), tirets (ordre), pointillés (SL / TP, brouillon).
    trait_: Trait,
    qty: String,
    texte: String,
    /// Couleur du texte (P&L vert / rouge).
    texte_c: Rgb,
    prise: Prise,
    /// ✕ à droite de l'étiquette.
    fermer: Option<Prise>,
    /// Bouton « Buy » / « Sell » à gauche (brouillon).
    bouton: Option<(&'static str, Prise)>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Trait {
    Plein,
    Tirets,
    Points,
}

const H: f64 = 20.0;
/// Bord droit des étiquettes : à cette distance de l'échelle de prix.
const MARGE: f64 = 70.0;

fn usd(m: Money) -> String {
    let c = m.cents();
    let s = if c > 0 {
        "+"
    } else if c < 0 {
        "−"
    } else {
        ""
    };
    let a = c.unsigned_abs();
    let ent = (a / 100).to_string();
    let mut g = String::new();
    for (i, ch) in ent.chars().enumerate() {
        if i > 0 && (ent.len() - i).is_multiple_of(3) {
            g.push(' ');
        }
        g.push(ch);
    }
    format!("{s}{g},{:02} USD", a % 100)
}

fn qte(q: f64) -> String {
    let t = if q >= 100.0 {
        format!("{q:.0}")
    } else if q >= 1.0 {
        format!("{q:.2}")
    } else {
        format!("{q:.4}")
    };
    t.replace('.', ",")
}

fn sl_c() -> Rgb {
    pal::ORANGE
}

/// Les lignes de la vue.
fn lignes(v: &VueOrdres) -> Vec<Ligne> {
    let mut out = vec![];
    for o in &v.ordres {
        let couleur = if o.buy { pal::ACCENT } else { pal::down() };
        let (texte, texte_c, trait_) = match (o.attente, o.pnl) {
            (Some(k), _) => {
                let sens = if o.buy { "d'achat" } else { "de vente" };
                (format!("{k} {sens}"), couleur, Trait::Tirets)
            }
            (None, p) => {
                let p = p.unwrap_or(Money::ZERO);
                (usd(p), pal::pnl(p.to_f64()), Trait::Plein)
            }
        };
        out.push(Ligne {
            prix: o.entree,
            couleur,
            trait_,
            qty: qte(o.qty),
            texte,
            texte_c,
            prise: Prise::Entree(o.id),
            fermer: Some(Prise::Fermer(o.id)),
            bouton: None,
        });
        // ⚡ 💀 où la position serait liquidée (perpétuel, marge isolée)
        if let Some((liq, levier)) = o.liq {
            out.push(Ligne {
                prix: liq,
                couleur: pal::RED,
                trait_: Trait::Tirets,
                qty: argos_core::perp::levier_txt(levier),
                texte: "Liquidation".into(),
                texte_c: pal::RED,
                prise: Prise::Liq(o.id),
                fermer: None,
                bouton: None,
            });
        }
        for (prix, stop) in [(o.tp, false), (o.sl, true)] {
            let Some(p) = prix else { continue };
            let (c, m) = if stop { (sl_c(), -o.perte_sl.abs()) } else { (pal::up(), o.gain_tp.abs()) };
            out.push(Ligne {
                prix: p,
                couleur: c,
                trait_: Trait::Points,
                qty: qte(o.qty),
                texte: usd(m),
                texte_c: c,
                prise: if stop { Prise::Sl(o.id) } else { Prise::Tp(o.id) },
                fermer: None,
                bouton: None,
            });
        }
    }
    if let Some(b) = &v.brouillon {
        let couleur = if b.buy { pal::ACCENT } else { pal::down() };
        let sens = if b.buy { "d'achat" } else { "de vente" };
        out.push(Ligne {
            prix: b.entree,
            couleur,
            trait_: Trait::Points,
            qty: qte(b.qty),
            texte: if b.genre == "Marché" {
                "Au marché".into()
            } else {
                format!("{} {sens}", b.genre)
            },
            texte_c: couleur,
            prise: Prise::BrEntree,
            fermer: Some(Prise::BrAnnuler),
            bouton: Some((if b.buy { "Buy" } else { "Sell" }, Prise::BrEnvoyer)),
        });
        out.push(Ligne {
            prix: b.tp,
            couleur: pal::up(),
            trait_: Trait::Points,
            qty: qte(b.qty),
            texte: usd(b.gain_tp.abs()),
            texte_c: pal::up(),
            prise: Prise::BrTp,
            fermer: None,
            bouton: None,
        });
        out.push(Ligne {
            prix: b.sl,
            couleur: sl_c(),
            trait_: Trait::Points,
            qty: qte(b.qty),
            texte: usd(-b.perte_sl.abs()),
            texte_c: sl_c(),
            prise: Prise::BrSl,
            fermer: None,
            bouton: None,
        });
    }
    if let Some(a) = &v.ticket {
        out.extend(lignes_ticket(a));
    }
    out
}

/// 🎫 The lines of the ticket's order: dotted, like an order not sent yet; TP and SL with what they bring or cost (in
/// % of the price until there is a size).
fn lignes_ticket(a: &Apercu) -> Vec<Ligne> {
    let mut out = vec![];
    let couleur = if a.buy { pal::ACCENT } else { pal::down() };
    let qty = if a.qty > 0.0 { qte(a.qty) } else { "—".to_string() };
    if let Some((genre, prix)) = a.entree {
        out.push(Ligne {
            prix,
            couleur,
            trait_: Trait::Points,
            qty: qty.clone(),
            texte: format!("{genre} {}", if a.buy { "d'achat" } else { "de vente" }),
            texte_c: couleur,
            prise: Prise::TkEntree,
            fermer: None,
            bouton: None,
        });
    }
    for (niveau, stop) in [(a.tp, false), (a.sl, true)] {
        let Some(prix) = niveau else { continue };
        let c = if stop { sl_c() } else { pal::up() };
        let texte = if a.qty > 0.0 {
            let m = Money::from_f64((prix - a.depart).abs() * a.qty);
            usd(if stop { -m } else { m })
        } else if a.depart > 0.0 {
            let d = (prix - a.depart) / a.depart * 100.0 * if a.buy { 1.0 } else { -1.0 };
            format!("{d:+.2} %").replace('.', ",").replace('-', "−")
        } else {
            String::new()
        };
        out.push(Ligne {
            prix,
            couleur: c,
            trait_: Trait::Points,
            qty: qty.clone(),
            texte,
            texte_c: c,
            prise: if stop { Prise::TkSl } else { Prise::TkTp },
            fermer: None,
            bouton: None,
        });
    }
    out
}

/// Les cases d'une étiquette : (zone entière, case quantité, case texte, ✕, bouton) pour la ligne à `y`.
struct Cases {
    tout: Rect,
    qty: Rect,
    texte: Rect,
    fermer: Option<Rect>,
    bouton: Option<Rect>,
}

fn cases(text: &Text, l: &Ligne, y: f64, droite: f64) -> Cases {
    let qw = text.width(&l.qty, 12.0) as f64 + 14.0;
    let tw = text.width(&l.texte, 12.0) as f64 + 18.0;
    let xw = if l.fermer.is_some() { 24.0 } else { 0.0 };
    let x0 = droite - xw - tw - qw;
    let top = y - H / 2.0;
    let bouton = l.bouton.map(|(t, _)| {
        let bw = text.width(t, 12.5) as f64 + 22.0;
        Rect::new(x0 - bw - 10.0, top, bw, H)
    });
    Cases {
        tout: Rect::new(x0, top, qw + tw + xw, H),
        qty: Rect::new(x0, top, qw, H),
        texte: Rect::new(x0 + qw, top, tw, H),
        fermer: l.fermer.map(|_| Rect::new(x0 + qw + tw, top, xw, H)),
        bouton,
    }
}

/// The room a label takes, with its « Buy » / « Sell » button at its left (a draft).
fn occupe(c: &Cases) -> Rect {
    match c.bouton {
        Some(b) => Rect::new(b.x, c.tout.y, c.tout.x + c.tout.w - b.x, c.tout.h),
        None => c.tout,
    }
}

fn croise(a: &Rect, b: &Rect) -> bool {
    a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h
}

/// 🏷 Where each label goes: on its line, its right edge `MARGE` from the price scale; one that would cover a label
/// already placed (an entry is placed before its TP and SL) moves left, next to it (Suivi 4: with the SL 0,5 % from
/// the entry, the entry's blue label was almost hidden under the SL's).
fn places(text: &Text, y_of: &dyn Fn(f64) -> f64, pw: f64, ls: &[Ligne]) -> Vec<Cases> {
    let mut out: Vec<Cases> = vec![];
    for l in ls {
        let y = y_of(l.prix).round();
        let mut c = cases(text, l, y, pw - MARGE);
        while let Some(x) = out.iter().map(occupe).find(|o| croise(o, &occupe(&c))).map(|o| o.x) {
            c = cases(text, l, y, x - 6.0);
        }
        out.push(c);
    }
    out
}

/// Ce que touche la souris en (`mx`, `my`) : un bouton (✕, Buy), sinon une ligne à 6 px près.
pub fn prise(text: &Text, y_of: &dyn Fn(f64) -> f64, pw: f64, v: &VueOrdres, mx: f64, my: f64) -> Option<Prise> {
    let ls = lignes(v);
    for (l, c) in ls.iter().zip(places(text, y_of, pw, &ls)) {
        if let (Some(r), Some(p)) = (c.fermer, l.fermer) {
            if r.contains(mx, my) {
                return Some(p);
            }
        }
        if let (Some(r), Some((_, p))) = (c.bouton, l.bouton) {
            if r.contains(mx, my) {
                return Some(p);
            }
        }
    }
    // les lignes : la plus proche, SL / TP avant l'entrée quand elles se touchent
    ls.iter()
        .filter(|l| !matches!(l.prise, Prise::Liq(_)) && mx <= pw && (y_of(l.prix) - my).abs() <= 6.0)
        .min_by(|a, b| (y_of(a.prix) - my).abs().total_cmp(&(y_of(b.prix) - my).abs()))
        .map(|l| l.prise)
}

/// Dessine les lignes, étiquettes et prix sur l'échelle (`pw` × `ph` : zone des bougies ; `axe` : largeur de
/// l'échelle de prix).
#[allow(clippy::too_many_arguments)]
pub fn draw(pm: &mut Pixmap, text: &Text, y_of: &dyn Fn(f64) -> f64, prix_txt: &dyn Fn(f64) -> String, pw: f64, ph: f64, axe: f64, v: &VueOrdres) {
    let ls = lignes(v);
    let couleur = |l: &Ligne| {
        if v.erreur.is_some_and(|(p, _)| p == l.prise) {
            pal::down()
        } else {
            l.couleur
        }
    };
    for (l, c) in ls.iter().zip(places(text, y_of, pw, &ls)) {
        let y = y_of(l.prix).round();
        if !(0.0..ph).contains(&y) {
            continue;
        }
        let err = v.erreur.filter(|(p, _)| *p == l.prise).map(|(_, m)| m);
        let c_ligne = couleur(l);
        // la ligne
        let (seg, pas) = match l.trait_ {
            Trait::Plein => (pw, pw),
            Trait::Tirets => (8.0, 13.0),
            Trait::Points => (2.0, 5.0),
        };
        let mut x = 0.0;
        while x < pw {
            // ligne discrète (comme TradingView) : pâle, sauf du mauvais côté
            crate::fill(pm, x, y, seg.min(pw - x), 1.0, c_ligne, if err.is_some() { 0.9 } else { 0.45 });
            x += pas;
        }
        if err.is_some() {
            crate::fill(pm, 0.0, y - 7.0, pw, 14.0, pal::down(), 0.12);
        }
        // l'étiquette : [qté | texte | ✕]
        round_fill(pm, c.tout, 3.0, pal::BACKDROP, 0.95);
        round_stroke(pm, c.tout, 3.0, c_ligne, 1.0);
        crate::fill(pm, c.qty.x + c.qty.w, c.tout.y + 3.0, 1.0, H - 6.0, c_ligne, 0.6);
        text.draw(pm, &l.qty, (c.qty.cx() as f32, (y + 4.0) as f32), 12.0, pal::WHITE, Align::Center);
        text.draw(pm, &l.texte, (c.texte.cx() as f32, (y + 4.0) as f32), 12.0, l.texte_c, Align::Center);
        if let Some(f) = c.fermer {
            crate::fill(pm, f.x, c.tout.y + 3.0, 1.0, H - 6.0, c_ligne, 0.6);
            crate::shapes::lines(pm, f.cx(), y, &[&[(-4.0, -4.0), (4.0, 4.0)], &[(4.0, -4.0), (-4.0, 4.0)]], c_ligne, 1.4);
        }
        if let (Some(b), Some((t, _))) = (c.bouton, l.bouton) {
            let ok = v.erreur.is_none();
            round_fill(pm, b, 4.0, if ok { l.couleur } else { pal::GREY }, 1.0);
            text.draw(pm, t, (b.cx() as f32, (y + 4.5) as f32), 12.5, pal::ink_on(l.couleur), Align::Center);
        }
        // pastille rouge « ! » et le message au-dessus
        if let Some(m) = err {
            let cx = c.tout.x - 14.0;
            round_fill(pm, Rect::new(cx - 8.0, y - 8.0, 16.0, 16.0), 8.0, pal::down(), 1.0);
            text.draw(pm, "!", (cx as f32, (y + 4.5) as f32), 12.5, Rgb(255, 255, 255), Align::Center);
            let mw = (text.width(m, 12.5) as f64 + 20.0).min(pw - 8.0);
            let b = Rect::new((cx - mw / 2.0).clamp(4.0, pw - mw - 4.0), y - 46.0, mw, 30.0);
            round_fill(pm, b, 6.0, Rgb(0x3a, 0x3c, 0x42), 0.97);
            text.draw(
                pm,
                &text.fit(m, 12.5, mw - 16.0),
                ((b.x + 10.0) as f32, (b.cy() + 4.5) as f32),
                12.5,
                pal::WHITE,
                Align::Left,
            );
        }
    }
    // the prices on the scale: an entry's last, on top of its TP and SL when they touch
    for l in ls.iter().rev() {
        let y = y_of(l.prix).round();
        if !(0.0..ph).contains(&y) {
            continue;
        }
        let tag = Rect::new(pw + 2.0, y - 9.0, axe - 4.0, 18.0);
        round_fill(pm, tag, 4.0, pal::BACKDROP, 1.0);
        round_stroke(pm, tag, 4.0, couleur(l), 1.0);
        text.draw(pm, &prix_txt(l.prix), ((pw + 8.0) as f32, (y + 4.0) as f32), 11.5, couleur(l), Align::Left);
    }
}

/// 🧱 F1k: the price labels [`draw`] puts on the scale (chart coordinates): the scale's own labels keep clear of them.
pub fn etiquettes_echelle(y_of: &dyn Fn(f64) -> f64, pw: f64, ph: f64, axe: f64, v: &VueOrdres) -> Vec<Rect> {
    lignes(v)
        .iter()
        .map(|l| y_of(l.prix).round())
        .filter(|y| (0.0..ph).contains(y))
        .map(|y| Rect::new(pw + 2.0, y - 9.0, axe - 4.0, 18.0))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vue() -> VueOrdres {
        VueOrdres {
            ordres: vec![
                OrdreChart {
                    id: 1,
                    attente: None,
                    buy: true,
                    qty: 2.6862,
                    entree: 100.0,
                    sl: Some(90.0),
                    tp: Some(120.0),
                    pnl: Some(Money::from_cents(9_670)),
                    perte_sl: Money::from_cents(100_000),
                    gain_tp: Money::from_cents(200_000),
                    liq: None,
                },
                OrdreChart {
                    id: 2,
                    attente: Some("Limite"),
                    buy: true,
                    qty: 1.0,
                    entree: 80.0,
                    sl: Some(75.0),
                    tp: Some(95.0),
                    pnl: None,
                    perte_sl: Money::from_cents(500),
                    gain_tp: Money::from_cents(1_500),
                    liq: None,
                },
            ],
            brouillon: None,
            ticket: None,
            erreur: None,
        }
    }

    /// Prix → y : 1 px par cent de prix, 100 en y = 200.
    fn y_of(p: f64) -> f64 {
        300.0 - p
    }

    #[test]
    fn etiquettes_comme_tradingview() {
        let ls = lignes(&vue());
        assert_eq!(ls.len(), 6, "entrée, TP, SL × 2");
        assert_eq!((ls[0].qty.as_str(), ls[0].texte.as_str()), ("2,69", "+96,70 USD"));
        assert_eq!(ls[0].texte_c, pal::up(), "P&L positif en vert");
        assert_eq!((ls[1].texte.as_str(), ls[2].texte.as_str()), ("+2 000,00 USD", "−1 000,00 USD"));
        assert_eq!(ls[3].texte, "Limite d'achat");
        assert_eq!(ls[3].trait_, Trait::Tirets);
        assert_eq!(usd(Money::ZERO), "0,00 USD");
    }

    #[test]
    fn prises_lignes_et_boutons() {
        let t = Text::default();
        let v = vue();
        let pw = 800.0;
        // une ligne à 6 px près
        assert_eq!(prise(&t, &y_of, pw, &v, 100.0, y_of(120.0) + 3.0), Some(Prise::Tp(1)));
        assert_eq!(prise(&t, &y_of, pw, &v, 100.0, y_of(90.0)), Some(Prise::Sl(1)));
        assert_eq!(prise(&t, &y_of, pw, &v, 100.0, y_of(80.0)), Some(Prise::Entree(2)));
        assert_eq!(prise(&t, &y_of, pw, &v, 100.0, y_of(110.0)), None);
        // le ✕ de la position
        let l = &lignes(&v)[0];
        let c = cases(&t, l, y_of(100.0), pw - MARGE);
        let f = c.fermer.unwrap();
        assert_eq!(prise(&t, &y_of, pw, &v, f.cx(), f.cy()), Some(Prise::Fermer(1)));
        // brouillon : bouton Buy et ✕
        let mut v = v;
        v.brouillon = Some(Brouillon {
            buy: true,
            entree: 150.0,
            sl: 140.0,
            tp: 170.0,
            qty: 1.0,
            genre: "Stop",
            perte_sl: Money::from_cents(1_000),
            gain_tp: Money::from_cents(2_000),
        });
        let ls = lignes(&v);
        let b = ls.iter().find(|l| l.prise == Prise::BrEntree).unwrap();
        assert_eq!(b.texte, "Stop d'achat");
        let c = cases(&t, b, y_of(150.0), pw - MARGE);
        let (bb, ff) = (c.bouton.unwrap(), c.fermer.unwrap());
        assert_eq!(prise(&t, &y_of, pw, &v, bb.cx(), bb.cy()), Some(Prise::BrEnvoyer));
        assert_eq!(prise(&t, &y_of, pw, &v, ff.cx(), ff.cy()), Some(Prise::BrAnnuler));
        assert_eq!(prise(&t, &y_of, pw, &v, 50.0, y_of(140.0)), Some(Prise::BrSl));
        assert!(Prise::BrSl.glisse() && !Prise::BrEnvoyer.glisse());
        assert_eq!(Prise::Tp(1).niveau(), Some(false));
        // le dessin ne casse pas, avec une erreur affichée
        v.erreur = Some((Prise::BrSl, "Le Stop Loss doit être inférieur au prix d'entrée."));
        let mut pm = crate::px::new(900, 300).unwrap();
        draw(&mut pm, &t, &y_of, &|p| format!("{p:.2}"), pw, 300.0, 90.0, &v);
        assert!(pm.pixels().iter().any(|p| p.alpha() > 0));
    }

    /// 🏷 Suivi 4 (Jee, 21 Nov): with the SL 0,5 % from the entry, the entry's blue label was almost hidden under the
    /// SL's. A label that would cover another moves left, next to it, on its own line: the entry's stays at its
    /// place, whole, and the pointer finds each one where it is drawn.
    #[test]
    fn etiquette_de_l_entree_jamais_cachee() {
        let t = Text::default();
        let pw = 800.0;
        let mut v = vue();
        v.ordres.truncate(1);
        v.ordres[0].sl = Some(99.5);
        let ls = lignes(&v);
        assert_eq!(ls.iter().map(|l| l.prise).collect::<Vec<_>>(), [Prise::Entree(1), Prise::Tp(1), Prise::Sl(1)]);
        let cs = places(&t, &y_of, pw, &ls);
        let (entree, sl) = (&cs[0], &cs[2]);
        assert_eq!(entree.tout.x + entree.tout.w, pw - MARGE, "the entry's label at its place");
        assert_eq!(sl.tout.x + sl.tout.w, entree.tout.x - 6.0, "the SL's next to it, at its left");
        assert_eq!(sl.tout.y, y_of(99.5).round() - H / 2.0, "on its own line");
        for (i, a) in cs.iter().enumerate() {
            assert!(cs[i + 1..].iter().all(|b| !croise(&occupe(a), &occupe(b))), "no label over another");
        }
        // the TP, far: at its place
        assert_eq!(cs[1].tout.x + cs[1].tout.w, pw - MARGE);
        let f = entree.fermer.unwrap();
        assert_eq!(prise(&t, &y_of, pw, &v, f.cx(), f.cy()), Some(Prise::Fermer(1)));
        // drawn: the entry's label keeps its blue frame where the SL's used to lie
        let mut pm = crate::px::new(900, 300).unwrap();
        draw(&mut pm, &t, &y_of, &|p| format!("{p:.2}"), pw, 300.0, 90.0, &v);
        let px = pm.pixel(entree.tout.x as u32, entree.tout.cy() as u32).unwrap();
        assert!(px.blue() > px.red(), "the entry's frame (blue), not the SL's (orange): {px:?}");
    }

    /// 🎫 Module 31: the ticket's order on the chart, before it leaves: its Limite line, its TP and SL with their
    /// amounts (in % without a size), each one caught to be dragged; at market, only TP and SL.
    #[test]
    fn lignes_du_ticket() {
        let t = Text::default();
        let mut v = VueOrdres {
            ticket: Some(Apercu {
                buy: true,
                entree: Some(("Limite", 100.0)),
                depart: 100.0,
                sl: Some(90.0),
                tp: Some(120.0),
                qty: 2.0,
            }),
            ..VueOrdres::default()
        };
        let ls = lignes(&v);
        let txt: Vec<(&str, &str)> = ls.iter().map(|l| (l.qty.as_str(), l.texte.as_str())).collect();
        assert_eq!(
            txt,
            vec![("2,00", "Limite d'achat"), ("2,00", "+40,00 USD"), ("2,00", "−20,00 USD")],
            "the TP brings (120 − 100) × 2, the SL costs (100 − 90) × 2"
        );
        assert!(ls.iter().all(|l| l.trait_ == Trait::Points && l.fermer.is_none() && l.bouton.is_none()));
        assert_eq!(prise(&t, &y_of, 800.0, &v, 100.0, y_of(100.0)), Some(Prise::TkEntree));
        assert_eq!(prise(&t, &y_of, 800.0, &v, 100.0, y_of(120.0)), Some(Prise::TkTp));
        assert_eq!(prise(&t, &y_of, 800.0, &v, 100.0, y_of(90.0)), Some(Prise::TkSl));
        assert!(Prise::TkSl.glisse() && Prise::TkEntree.glisse());
        assert_eq!(
            (Prise::TkSl.niveau(), Prise::TkTp.niveau(), Prise::TkEntree.niveau()),
            (Some(true), Some(false), None)
        );
        // a sell at market without a size: no entry line, the distances in %
        v.ticket = Some(Apercu {
            buy: false,
            entree: None,
            depart: 100.0,
            sl: Some(101.0),
            tp: Some(98.0),
            qty: 0.0,
        });
        let ls = lignes(&v);
        let txt: Vec<(&str, &str)> = ls.iter().map(|l| (l.qty.as_str(), l.texte.as_str())).collect();
        assert_eq!(txt, vec![("—", "+2,00 %"), ("—", "−1,00 %")]);
        let mut pm = crate::px::new(900, 300).unwrap();
        draw(&mut pm, &t, &y_of, &|p| format!("{p:.2}"), 800.0, 300.0, 90.0, &v);
        assert!(pm.pixels().iter().any(|p| p.alpha() > 0));
    }

    /// ⚡ 💀 Lot 4 : la ligne de liquidation d'une position perpétuelle, en tirets rouges ; elle ne s'attrape pas.
    #[test]
    fn ligne_de_liquidation() {
        let t = Text::default();
        let mut v = vue();
        v.ordres[0].liq = Some((60.0, 10.0));
        let ls = lignes(&v);
        let l = ls.iter().find(|l| l.prise == Prise::Liq(1)).unwrap();
        assert_eq!((l.qty.as_str(), l.texte.as_str(), l.trait_), ("×10", "Liquidation", Trait::Tirets));
        assert_eq!(prise(&t, &y_of, 800.0, &v, 100.0, y_of(60.0)), None, "montrée seulement");
        assert!(!Prise::Liq(1).glisse());
        let mut pm = crate::px::new(900, 300).unwrap();
        draw(&mut pm, &t, &y_of, &|p| format!("{p:.2}"), 800.0, 300.0, 90.0, &v);
        assert!(pm.pixels().iter().any(|p| p.alpha() > 0));
    }

    /// 📸 Capture des lignes d'ordre pour Jee (seulement si `ARGOS_CAPTURES` donne un dossier).
    #[test]
    fn captures_pour_jee() {
        let Ok(dir) = std::env::var("ARGOS_CAPTURES") else { return };
        let t = Text::default();
        let mut v = vue();
        v.brouillon = Some(Brouillon {
            buy: true,
            entree: 150.0,
            sl: 158.0,
            tp: 170.0,
            qty: 1.0,
            genre: "Stop",
            perte_sl: Money::from_cents(800),
            gain_tp: Money::from_cents(2_000),
        });
        v.erreur = Some((Prise::BrSl, "Le Stop Loss doit être inférieur au prix d'entrée."));
        let mut pm = crate::px::new(900, 300).unwrap();
        pm.fill(tiny_skia::Color::from_rgba8(0x13, 0x17, 0x22, 255));
        draw(
            &mut pm,
            &t,
            &|p| 290.0 - (p - 70.0) * 2.7,
            &|p| format!("{p:.2}").replace('.', ","),
            800.0,
            300.0,
            100.0,
            &v,
        );
        pm.save_png(format!("{dir}/ordres-sur-la-chart.png")).unwrap();
    }
}
