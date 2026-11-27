//! 🧪 Ticket démo commun (volet Trading et Simulateur) : un seul moteur, le compte démo [`Portfolio`] en centimes.
//! Ici : le ticket (Marché / Limite / Stop, SL et TP **obligatoires**, taille tirée du risque) vérifié puis changé en
//! ordre du portfolio, et la vue du ⏯ replay du Simulateur. L'exécution sur les bougies est `Portfolio::on_bar`.
//! 🎫 Module 31 (the ticket of the Trading panel): [`OrderTicket`], a size instead of a risk, SL and TP optional, a
//! validity (GTC / IOC / Post-only) and reduce-only; [`estimate`] gives the margin, fees, slippage and Max before
//! sending, [`mid`] the book's middle.
//!
//! Trading démo seulement : rien ici ne parle à un exchange.

use crate::couts::Couts;
use crate::perp::Perp;
use crate::portfolio::{ClosedTrade, Order, OrderKind, Pending, Portfolio, Position};
use crate::types::Side;
use crate::Money;

/// Genre d'ordre.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Market,
    Limit,
    Stop,
}

impl Kind {
    pub const ALL: [Kind; 3] = [Kind::Market, Kind::Limit, Kind::Stop];

    pub fn name(self) -> &'static str {
        match self {
            Kind::Market => "Marché",
            Kind::Limit => "Limite",
            Kind::Stop => "Stop",
        }
    }

    /// L'ordre en attente du portfolio (`None` : au marché).
    pub fn pending(self) -> Option<OrderKind> {
        match self {
            Kind::Market => None,
            Kind::Limit => Some(OrderKind::Limit),
            Kind::Stop => Some(OrderKind::Stop),
        }
    }
}

/// Un ticket : `price` sert pour Limite et Stop (au marché : le dernier prix).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ticket {
    pub buy: bool,
    pub kind: Kind,
    pub price: f64,
    pub sl: f64,
    pub tp: f64,
    /// 💰 Perte si le SL est touché.
    pub risk: Money,
}

/// 🚦 Raison d'un refus avant envoi (première règle cassée, dans l'ordre de [`controler`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refus {
    /// Prix d'entrée (Limite, Stop) ou dernier prix absent, nul ou pas un nombre.
    PrixAbsent,
    /// SL ou TP absent.
    SlTpAbsents,
    /// SL du mauvais côté de l'entrée.
    SlMauvaisCote,
    /// TP du mauvais côté de l'entrée.
    TpMauvaisCote,
    /// Limite ou Stop du mauvais côté du dernier prix pour ce sens.
    AttenteMalPlacee(OrderKind),
    /// Taille nulle, négative ou pas un nombre.
    TailleNulle,
}

impl Refus {
    /// Le message montré dans le ticket (`buy` : sens de l'ordre).
    pub fn message(self, buy: bool) -> String {
        let (sous, dessus) = ("sous", "au-dessus de");
        match self {
            Refus::PrixAbsent => "prix d'entrée manquant".into(),
            Refus::SlTpAbsents => "SL et TP sont obligatoires".into(),
            Refus::SlMauvaisCote => format!("le SL doit être {} l'entrée", if buy { sous } else { dessus }),
            Refus::TpMauvaisCote => format!("le TP doit être {} l'entrée", if buy { dessus } else { sous }),
            Refus::AttenteMalPlacee(k) => {
                let (quoi, place) = match (k, buy) {
                    (OrderKind::Limit, true) => ("un Buy Limit", "sous le"),
                    (OrderKind::Stop, false) => ("un Sell Stop", "sous le"),
                    (OrderKind::Limit, false) => ("un Sell Limit", "au-dessus du"),
                    (OrderKind::Stop, true) => ("un Buy Stop", "au-dessus du"),
                };
                format!("{quoi} se place {place} prix actuel")
            }
            Refus::TailleNulle => "la taille doit être plus grande que 0".into(),
        }
    }
}

/// 🚦 Un ordre tel qu'il partirait : `entry` est le prix d'entrée (au marché : `last`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Avant {
    pub buy: bool,
    pub kind: Kind,
    /// Dernier prix du marché.
    pub last: f64,
    pub entry: f64,
    pub sl: f64,
    pub tp: f64,
    pub qty: f64,
}

fn prix(v: f64) -> bool {
    v.is_finite() && v > 0.0
}

/// SL du bon côté de l'entrée : sous à l'achat, au-dessus à la vente.
pub fn sl_ok(buy: bool, entry: f64, sl: f64) -> bool {
    if buy {
        sl < entry
    } else {
        sl > entry
    }
}

/// TP du bon côté de l'entrée : au-dessus à l'achat, sous à la vente.
pub fn tp_ok(buy: bool, entry: f64, tp: f64) -> bool {
    if buy {
        tp > entry
    } else {
        tp < entry
    }
}

/// Ordre en attente du bon côté du dernier prix : Limite sous le prix à l'achat (au-dessus à la vente), Stop
/// au-dessus à l'achat (sous à la vente).
pub fn attente_ok(kind: OrderKind, buy: bool, price: f64, last: f64) -> bool {
    match (kind, buy) {
        (OrderKind::Limit, true) | (OrderKind::Stop, false) => price < last,
        (OrderKind::Limit, false) | (OrderKind::Stop, true) => price > last,
    }
}

/// 🚦 Contrôles avant envoi, fonction pure : prix présents, SL et TP posés et du bon côté, Limite / Stop du bon côté
/// du dernier prix, taille plus grande que 0. Rend la première règle cassée.
pub fn controler(a: &Avant) -> Result<(), Refus> {
    if !prix(a.last) || !prix(a.entry) {
        return Err(Refus::PrixAbsent);
    }
    if !prix(a.sl) || !prix(a.tp) {
        return Err(Refus::SlTpAbsents);
    }
    if !sl_ok(a.buy, a.entry, a.sl) {
        return Err(Refus::SlMauvaisCote);
    }
    if !tp_ok(a.buy, a.entry, a.tp) {
        return Err(Refus::TpMauvaisCote);
    }
    if let Some(k) = a.kind.pending() {
        if !attente_ok(k, a.buy, a.entry, a.last) {
            return Err(Refus::AttenteMalPlacee(k));
        }
    }
    if !(a.qty.is_finite() && a.qty > 0.0) {
        return Err(Refus::TailleNulle);
    }
    Ok(())
}

/// Taille pour perdre `risk` si le SL est touché (0 si entrée = SL).
pub fn size_for_risk(risk: Money, entry: f64, sl: f64) -> f64 {
    let d = (entry - sl).abs();
    if d <= 0.0 || !d.is_finite() {
        return 0.0;
    }
    risk.to_f64() / d
}

impl Ticket {
    /// Prix d'entrée au dernier prix `last` (au marché : `last`).
    pub fn entry(&self, last: f64) -> f64 {
        if self.kind == Kind::Market {
            last
        } else {
            self.price
        }
    }

    /// Vérifie le ticket au dernier prix `last` : risque entre 0 et 10 % du compte `pf`, puis les contrôles avant
    /// envoi ([`controler`]) ; rend le prix d'entrée.
    pub fn check(&self, pf: &Portfolio, last: f64) -> Result<f64, String> {
        let max = pf.balance.max(Money::ZERO).pct(10.0);
        if self.risk <= Money::ZERO || self.risk > max {
            return Err("risque entre 0 et 10 % du compte".into());
        }
        let entry = self.entry(last);
        controler(&Avant {
            buy: self.buy,
            kind: self.kind,
            last,
            entry,
            sl: self.sl,
            tp: self.tp,
            qty: size_for_risk(self.risk, entry, self.sl),
        })
        .map_err(|r| r.message(self.buy))?;
        Ok(entry)
    }

    /// L'ordre du portfolio pour `base` (taille tirée du risque).
    pub fn order(&self, base: &str, entry: f64) -> Order {
        Order {
            base: base.into(),
            side: if self.buy { Side::Long } else { Side::Short },
            qty: size_for_risk(self.risk, entry, self.sl),
            price: entry,
            stop: Some(self.sl),
            target: Some(self.tp),
            couts: None,
            perp: None,
        }
    }

    /// Vérifie puis place le ticket dans `pf` (au marché : ouvert tout de suite au prix `last`), avec les coûts de
    /// sa bourse (`None` : les frais du compte). Rend l'identifiant et `true` si une position est ouverte (`false` :
    /// ordre en attente).
    /// ⚡ `perp` : un perpétuel (levier, marge isolée), `None` au comptant.
    pub fn place(&self, pf: &mut Portfolio, base: &str, last: f64, now: i64, couts: Option<Couts>, perp: Option<Perp>) -> Result<(u64, bool), String> {
        let entry = self.check(pf, last)?;
        let o = Order {
            couts,
            perp,
            ..self.order(base, entry)
        };
        match self.kind.pending() {
            None => pf.open(o, now).map(|id| (id, true)),
            Some(k) => pf.place(k, o, last, None, now).map(|id| (id, false)),
        }
        .map_err(|e| e.to_string())
    }
}

/// 🔀 Place un OCO : les deux tickets (ordres en attente, souvent Buy Stop au-dessus et Sell Stop dessous) vérifiés
/// chacun, puis posés ensemble et liés ; quand l'un part, l'autre est annulé. Rend les deux identifiants.
pub fn place_oco(
    a: &Ticket,
    b: &Ticket,
    pf: &mut Portfolio,
    base: &str,
    last: f64,
    now: i64,
    couts: Option<Couts>,
    perp: Option<Perp>,
) -> Result<(u64, u64), String> {
    let mut couple = vec![];
    for t in [a, b] {
        let entry = t.check(pf, last)?;
        let kind = t.kind.pending().ok_or("un OCO se fait avec deux ordres en attente")?;
        couple.push((
            kind,
            Order {
                couts,
                perp,
                ..t.order(base, entry)
            },
        ));
    }
    let b = couple.pop().ok_or("OCO incomplet")?;
    let a = couple.pop().ok_or("OCO incomplet")?;
    pf.place_oco(a, b, last, None, now).map_err(|e| e.to_string())
}

/// ⏳ How long an order lives (module 31 ticket): until it is cancelled, now or never, or only as a maker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Validity {
    /// Good till cancelled: the order waits until the price reaches it, or until it is cancelled.
    #[default]
    Gtc,
    /// Immediate or cancel: it fills now if the price is at its limit or better, else it is cancelled.
    Ioc,
    /// Post-only: it waits like GTC, but it is refused if it would fill now (it always pays the maker fee).
    PostOnly,
}

impl Validity {
    pub const ALL: [Validity; 3] = [Validity::Gtc, Validity::Ioc, Validity::PostOnly];

    /// The name in the ticket (« GTC », « IOC », « Post-only »).
    pub fn name(self) -> &'static str {
        match self {
            Validity::Gtc => "GTC",
            Validity::Ioc => "IOC",
            Validity::PostOnly => "Post-only",
        }
    }

    /// What it means, in a few words (the menu of the ticket).
    pub fn detail(self) -> &'static str {
        match self {
            Validity::Gtc => "Jusqu'à annulation",
            Validity::Ioc => "Tout de suite, ou annulé",
            Validity::PostOnly => "Seulement en maker",
        }
    }
}

/// 🎫 The order of the module 31 ticket: a size (quantity) instead of a risk, SL and TP optional, a validity, and
/// reduce-only. Demo account only.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrderTicket {
    pub buy: bool,
    pub kind: Kind,
    /// Limit price, or the Stop's trigger (at market: not used).
    pub price: f64,
    pub qty: f64,
    pub sl: Option<f64>,
    pub tp: Option<f64>,
    /// Limit only: a Stop waits until it is cancelled (GTC), a market order fills now.
    pub validity: Validity,
    /// Only takes off the positions on the other side, never opens nor grows one (SL and TP not used).
    pub reduce_only: bool,
}

/// 🎫 What the ticket did.
#[derive(Debug, Clone, PartialEq)]
pub enum Placed {
    /// A position opened now (its id): at market, or a Limit at the last price or better (taker).
    Opened(u64),
    /// An order waits for its price (its id).
    Resting(u64),
    /// Reduce-only that filled now: the trades it closed.
    Reduced(Vec<ClosedTrade>),
}

/// The message of a refused IOC (its price is not reached now).
pub const IOC_REFUS: &str = "IOC : le prix du moment n'est pas à ta limite, l'ordre est annulé (rien n'attend)";
/// The message of a refused Post-only (it would fill now, as a taker).
pub const POST_ONLY_REFUS: &str = "Post-only : à ce prix l'ordre passerait tout de suite, il est refusé (il n'entre qu'en maker)";

impl OrderTicket {
    /// Would the order fill now at the last price `last`? At market always; a Limit at the last price or better
    /// (buy at or above it, sell at or below it); a Stop never (it waits for its trigger).
    pub fn marketable(&self, last: f64) -> bool {
        match self.kind {
            Kind::Market => true,
            Kind::Limit => !attente_ok(OrderKind::Limit, self.buy, self.price, last),
            Kind::Stop => false,
        }
    }

    /// The price the order fills at, or waits at: the last price when it fills now, else its own price.
    pub fn fill_price(&self, last: f64) -> f64 {
        if self.marketable(last) {
            last
        } else {
            self.price
        }
    }

    /// 🚦 Checks before sending, the first broken rule: prices present, a size, reduce-only with something to reduce,
    /// SL and TP (when set) on the right side of the fill price, a Stop on the right side of the last price, and the
    /// validity (IOC that cannot fill now, Post-only that would).
    pub fn check(&self, pf: &Portfolio, base: &str, last: f64) -> Result<(), String> {
        if !prix(last) || (self.kind != Kind::Market && !prix(self.price)) {
            return Err(Refus::PrixAbsent.message(self.buy));
        }
        if !(self.qty.is_finite() && self.qty > 0.0) {
            return Err(Refus::TailleNulle.message(self.buy));
        }
        if self.reduce_only && pf.reducible(base, self.buy) <= 0.0 {
            return Err(crate::portfolio::NOTHING_TO_REDUCE.into());
        }
        let entry = self.fill_price(last);
        if !self.reduce_only {
            if self.sl.is_some_and(|sl| !prix(sl) || !sl_ok(self.buy, entry, sl)) {
                return Err(Refus::SlMauvaisCote.message(self.buy));
            }
            if self.tp.is_some_and(|tp| !prix(tp) || !tp_ok(self.buy, entry, tp)) {
                return Err(Refus::TpMauvaisCote.message(self.buy));
            }
        }
        if self.kind == Kind::Stop && !attente_ok(OrderKind::Stop, self.buy, self.price, last) {
            return Err(Refus::AttenteMalPlacee(OrderKind::Stop).message(self.buy));
        }
        if self.kind == Kind::Limit {
            match self.validity {
                Validity::Ioc if !self.marketable(last) => return Err(IOC_REFUS.into()),
                Validity::PostOnly if self.marketable(last) => return Err(POST_ONLY_REFUS.into()),
                _ => {}
            }
        }
        Ok(())
    }

    /// Checks then sends the order to the demo account `pf` for `base`: it fills now (taker, at the last price) when
    /// it can, else it waits (a Limit fills as a maker). `couts`: the exchange's costs (`None`: the account's fee);
    /// ⚡ `perp`: a perpetual (leverage, isolated margin), `None` on spot.
    pub fn place(&self, pf: &mut Portfolio, base: &str, last: f64, now: i64, couts: Option<Couts>, perp: Option<Perp>) -> Result<Placed, String> {
        self.check(pf, base, last)?;
        let now_fill = self.marketable(last);
        if self.reduce_only {
            return if now_fill {
                pf.reduce_now(base, self.buy, self.qty, last, now).map(Placed::Reduced)
            } else {
                let kind = self.kind.pending().unwrap_or(OrderKind::Limit);
                pf.place_reduce(kind, base, self.buy, self.qty, self.price, last, now).map(Placed::Resting)
            }
            .map_err(|e| e.to_string());
        }
        let o = Order {
            base: base.into(),
            side: if self.buy { Side::Long } else { Side::Short },
            qty: self.qty,
            price: self.fill_price(last),
            stop: self.sl,
            target: self.tp,
            couts,
            perp,
        };
        match self.kind.pending().filter(|_| !now_fill) {
            None => pf.open(o, now).map(Placed::Opened),
            Some(k) => pf.place(k, o, last, None, now).map(Placed::Resting),
        }
        .map_err(|e| e.to_string())
    }
}

/// 🎫 What the ticket shows before sending (the row Marge / Frais / Max / Glissement).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Estimate {
    /// Margin held: the value over the leverage (perpetual), the whole value on spot, nothing for reduce-only.
    pub margin: Money,
    /// The exchange's fee: taker when it fills now (market, a Limit at the last price or better, a Stop when it
    /// triggers), maker for a Limit that waits.
    pub fees: Money,
    /// Half the spread and the slippage (only an order that takes the book pays them).
    pub slippage: Money,
    /// The largest quantity: the buying power over the price (reduce-only: the position on the other side), rounded
    /// down to the market's step.
    pub max_qty: f64,
}

/// ⚡ Buying power: what is available times the leverage (spot: what is available).
pub fn buying_power(pf: &Portfolio, perp: Option<Perp>) -> Money {
    let dispo = pf.disponible().max(Money::ZERO);
    match perp {
        Some(x) => Money::produit(&[dispo.to_f64(), x.levier.max(1.0)]),
        None => dispo,
    }
}

/// 🎫 The estimate of `t` on `base` at the last price `last`, with the exchange's costs `couts` (`None` or « sans
/// frais »: the account's fee) and ⚡ `perp` (`None`: spot).
pub fn estimate(pf: &Portfolio, t: &OrderTicket, base: &str, last: f64, couts: Option<Couts>, perp: Option<Perp>) -> Estimate {
    let px = t.fill_price(last);
    let taker = t.marketable(last) || t.kind == Kind::Stop;
    let taux = match couts.filter(|_| !pf.sans_frais) {
        Some(c) => c.taux(taker),
        None => crate::couts::Taux {
            frais: pf.fee_rate,
            glissement: 0.0,
        },
    };
    let qty = if t.qty.is_finite() && t.qty > 0.0 && prix(px) { t.qty } else { 0.0 };
    let (fees, slippage) = taux.montants(qty, px);
    let margin = match (t.reduce_only, perp) {
        (true, _) => Money::ZERO,
        (false, Some(x)) => x.marge(qty, px),
        (false, None) => Money::produit(&[qty, px]),
    };
    let max_qty = if t.reduce_only {
        pf.reducible(base, t.buy)
    } else if prix(px) {
        buying_power(pf, perp).to_f64() / px
    } else {
        0.0
    };
    Estimate {
        margin,
        fees,
        slippage,
        max_qty: round_qty(max_qty, qty_step(px)),
    }
}

/// 📏 The market's quantity step at `price`: about 10 $ of value, a power of ten, at most 1 (BTC at 60 000: 0.0001;
/// ETH at 3 000: 0.001; SOL at 150: 0.01; DOGE: 1).
pub fn qty_step(price: f64) -> f64 {
    if !prix(price) {
        return 1.0;
    }
    10f64.powi((10.0 / price).log10().floor() as i32).min(1.0)
}

/// 📏 `qty` rounded down to the step `step` (never more than asked, nor than possible).
pub fn round_qty(qty: f64, step: f64) -> f64 {
    if !(qty.is_finite() && qty > 0.0 && step > 0.0) {
        return 0.0;
    }
    let inv = (1.0 / step).round().max(1.0);
    (qty * inv + 1e-9).floor() / inv
}

/// 📏 Decimals of a quantity at the step `step` (0.0001: 4).
pub fn qty_decimals(step: f64) -> usize {
    if step >= 1.0 || step <= 0.0 {
        0
    } else {
        (-step.log10()).round() as usize
    }
}

/// ⚖ « Mid »: halfway between the best bid and the best ask of the book, when both are there, not crossed, and
/// within 0.5 % of the last price (a stale book is not used); else the last price.
pub fn mid(best_bid: Option<f64>, best_ask: Option<f64>, last: f64) -> f64 {
    match (best_bid, best_ask) {
        (Some(b), Some(a)) if prix(b) && prix(a) && b <= a => {
            let m = (a + b) / 2.0;
            if prix(last) && (m - last).abs() > last * 0.005 {
                last
            } else {
                m
            }
        }
        _ => last,
    }
}

/// ⏯ Replay du Simulateur (vu par le volet) : bougies montrées, lecture, vitesse. Le pas d'une bougie est celui du
/// replay de la chart (`argos_render::replay`), qui coupe les bougies ; les deux ont les mêmes vitesses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Replay {
    /// Bougies montrées.
    pub shown: usize,
    pub play: bool,
    /// Bougies par seconde (une de `SPEEDS`).
    pub speed: u32,
}

impl Replay {
    /// ⏯ Vitesses du replay (bougies par seconde), les mêmes pour la chart et le Simulateur.
    pub const SPEEDS: [u32; 4] = [1, 2, 5, 10];

    /// Vitesse suivante : ×1 → ×2 → ×5 → ×10 → ×1 (une vitesse inconnue repart comme ×1).
    pub fn next_speed(speed: u32) -> u32 {
        let i = Self::SPEEDS.iter().position(|&s| s == speed).unwrap_or(0);
        Self::SPEEDS[(i + 1) % Self::SPEEDS.len()]
    }
}

/// 💾 Séance de replay gardée pour plus tard : le marché, l'heure de la dernière bougie jouée, la vitesse, et ce qui
/// était ouvert. Les positions et ordres en attente sont mis de côté hors du compte démo (ni fermés ni annulés), pour
/// que leurs prix du passé ne touchent jamais le direct ; « Reprendre » les remet dans le compte.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Seance {
    /// Exchange (son identifiant, `Exchange::id`), symbole et unité de temps (secondes).
    pub exchange: String,
    pub base: String,
    pub interval: i64,
    /// Heure de la dernière bougie jouée (secondes UTC) : le replay reprend là.
    pub t: i64,
    pub speed: u32,
    /// Dernier prix joué (clôture de la bougie `t`) : « Oublier » ferme les positions gardées à ce prix.
    #[serde(default)]
    pub prix: f64,
    #[serde(default)]
    pub positions: Vec<Position>,
    #[serde(default)]
    pub pending: Vec<Pending>,
    /// Heure de la sauvegarde (secondes UTC).
    pub saved_at: i64,
}

impl Seance {
    /// Ce qui était ouvert au moment de la sauvegarde : positions et ordres en attente.
    pub fn ouverts(&self) -> usize {
        self.positions.len() + self.pending.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Bar;

    fn bar(t: i64, o: f64, h: f64, l: f64, c: f64) -> Bar {
        Bar {
            time: t,
            open: o,
            high: h,
            low: l,
            close: c,
            volume: 0.0,
        }
    }

    /// Achat avec 1 000 $ de risque (1 % du compte de 100 000 $).
    fn achat(kind: Kind, price: f64, sl: f64, tp: f64) -> Ticket {
        Ticket {
            buy: true,
            kind,
            price,
            sl,
            tp,
            risk: Money::from_cents(100_000),
        }
    }

    fn rien(_: &Order) -> Option<String> {
        None
    }

    #[test]
    fn taille_a_partir_du_risque_en_cents() {
        let mut pf = Portfolio::default();
        // SL à 10 $ de l'entrée → 100 unités
        achat(Kind::Market, 0.0, 90.0, 120.0).place(&mut pf, "XAU", 100.0, 0, None, None).unwrap();
        assert_eq!(pf.positions[0].qty, 100.0);
        // TP touché : +20 $ × 100 = 2 000 $ exactement, soit +2 R
        let (_, c) = pf.on_bar("XAU", &bar(60, 101.0, 121.0, 99.0, 115.0), rien);
        assert_eq!((c[0].pnl, c[0].r(), c[0].reason.as_str()), (Money::from_cents(200_000), 2.0, "objectif"));
        assert_eq!(pf.balance, Money::from_cents(10_200_000));
        // mille petits trades : aucune dérive (centimes entiers)
        let mut b = Portfolio::default();
        for i in 0..1000 {
            let t = Ticket {
                risk: Money::from_cents(3_001),
                ..achat(Kind::Market, 0.0, 99.97, 100.03)
            };
            t.place(&mut b, "XAU", 100.0, i * 2, None, None).unwrap();
            b.on_bar("XAU", &bar(i * 2 + 1, 100.0, 100.05, 99.99, 100.0), rien);
        }
        let sum: Money = b.history.iter().map(|c| c.pnl).sum();
        assert_eq!(b.balance - b.start, sum, "solde = somme exacte des trades");
    }

    #[test]
    fn sl_et_tp_obligatoires_et_du_bon_cote() {
        let pf = Portfolio::default();
        assert!(achat(Kind::Market, 0.0, 0.0, 110.0).check(&pf, 100.0).unwrap_err().contains("obligatoires"));
        assert!(achat(Kind::Market, 0.0, 101.0, 110.0).check(&pf, 100.0).unwrap_err().contains("SL"));
        assert!(achat(Kind::Market, 0.0, 95.0, 99.0).check(&pf, 100.0).unwrap_err().contains("TP"));
        let mut pf2 = Portfolio::default();
        assert!(
            achat(Kind::Limit, 101.0, 95.0, 110.0).place(&mut pf2, "XAU", 100.0, 0, None, None).is_err(),
            "limite achat au-dessus du prix"
        );
        assert!(
            achat(Kind::Stop, 99.0, 95.0, 110.0).place(&mut pf2, "XAU", 100.0, 0, None, None).is_err(),
            "stop achat sous le prix"
        );
        let gros = Ticket {
            risk: Money::from_cents(1_100_000),
            ..achat(Kind::Market, 0.0, 95.0, 110.0)
        };
        assert!(gros.check(&pf, 100.0).is_err(), "risque > 10 % du compte");
    }

    #[test]
    fn limite_stop_trous_et_pire_cas() {
        let mut pf = Portfolio::default();
        achat(Kind::Limit, 98.0, 95.0, 104.0).place(&mut pf, "XAU", 100.0, 0, None, None).unwrap();
        achat(Kind::Stop, 102.0, 99.0, 108.0).place(&mut pf, "XAU", 100.0, 0, None, None).unwrap();
        pf.on_bar("XAU", &bar(1, 100.0, 101.0, 97.5, 99.0), rien);
        assert_eq!((pf.pending.len(), pf.positions.len(), pf.positions[0].entry), (1, 1, 98.0), "la limite seule");
        // SL et TP dans la même bougie : sortie au SL (le pire cas) ; le stop déclenché dans cette bougie est aussi
        // touché à son SL : sorti tout de suite
        let (_, c) = pf.on_bar("XAU", &bar(2, 99.0, 104.5, 94.0, 100.0), rien);
        assert_eq!(
            c.iter().map(|c| (c.reason.as_str(), c.entry)).collect::<Vec<_>>(),
            [("stop", 98.0), ("stop", 102.0)]
        );
        assert_eq!(c[0].r(), -1.0);
        assert!(pf.positions.is_empty() && pf.pending.is_empty());
        // trou : ouverture sous le SL → sortie à l'ouverture, plus que 1 R de perte
        achat(Kind::Market, 0.0, 98.0, 110.0).place(&mut pf, "XAU", 100.0, 3, None, None).unwrap();
        let (_, c) = pf.on_bar("XAU", &bar(4, 97.0, 97.5, 96.0, 97.0), rien);
        assert_eq!((c[0].reason.as_str(), c[0].exit), ("stop", 97.0));
        assert!(c[0].r() < -1.0);
        // limite sautée par l'ouverture : remplie à l'ouverture (meilleur prix)
        achat(Kind::Limit, 95.0, 90.0, 105.0).place(&mut pf, "XAU", 97.0, 5, None, None).unwrap();
        pf.on_bar("XAU", &bar(6, 94.0, 96.0, 93.5, 95.5), rien);
        assert_eq!(pf.positions[0].entry, 94.0);
        // un autre symbole ne touche pas ce compte
        let (_, c) = pf.on_bar("BTC", &bar(7, 1.0, 1.0, 1.0, 1.0), rien);
        assert!(c.is_empty() && pf.positions.len() == 1);
    }

    #[test]
    fn stats_courbe_et_remise_a_zero() {
        let mut pf = Portfolio::default();
        achat(Kind::Market, 0.0, 90.0, 120.0).place(&mut pf, "XAU", 100.0, 0, None, None).unwrap();
        pf.on_bar("XAU", &bar(1, 100.0, 125.0, 99.0, 120.0), rien); // +2 R
        let t = Ticket {
            risk: Money::from_cents(102_000),
            ..achat(Kind::Market, 0.0, 110.0, 140.0)
        };
        t.place(&mut pf, "XAU", 120.0, 2, None, None).unwrap();
        pf.on_bar("XAU", &bar(3, 120.0, 121.0, 105.0, 108.0), rien); // −1 R
        let s = pf.stats();
        assert_eq!((s.trades, s.win_rate), (2, 50.0));
        assert!((s.avg_ratio - 2.0 / 1.02).abs() < 0.01, "{}", s.avg_ratio);
        assert_eq!(s.max_dd, Money::from_cents(102_000));
        assert!((s.max_dd_pct - 1.0).abs() < 1e-9, "1 020 $ sous le plus haut de 102 000 $ : 1 %");
        assert!(!pf.curve.is_empty());
        let nom = pf.name.clone();
        pf.reset();
        assert_eq!((pf.balance, pf.history.len(), pf.curve.len(), pf.name.as_str()), (pf.start, 0, 0, nom.as_str()));
    }

    /// Un ordre valable au dernier prix 100 : achat SL 95 / TP 110, vente SL 105 / TP 90, taille 1.
    fn avant(buy: bool, kind: Kind, entry: f64) -> Avant {
        let (sl, tp) = if buy { (entry - 5.0, entry + 10.0) } else { (entry + 5.0, entry - 10.0) };
        Avant {
            buy,
            kind,
            last: 100.0,
            entry,
            sl,
            tp,
            qty: 1.0,
        }
    }

    #[test]
    fn controle_ordres_valables() {
        for buy in [true, false] {
            assert_eq!(controler(&avant(buy, Kind::Market, 100.0)), Ok(()), "marché, achat = {buy}");
        }
        // Buy Limit et Sell Stop sous le prix, Sell Limit et Buy Stop au-dessus
        assert_eq!(controler(&avant(true, Kind::Limit, 98.0)), Ok(()));
        assert_eq!(controler(&avant(false, Kind::Stop, 98.0)), Ok(()));
        assert_eq!(controler(&avant(false, Kind::Limit, 102.0)), Ok(()));
        assert_eq!(controler(&avant(true, Kind::Stop, 102.0)), Ok(()));
    }

    #[test]
    fn controle_prix_absent() {
        for buy in [true, false] {
            for v in [0.0, -1.0, f64::NAN, f64::INFINITY] {
                let a = Avant {
                    entry: v,
                    ..avant(buy, Kind::Limit, 98.0)
                };
                assert_eq!(controler(&a), Err(Refus::PrixAbsent), "entrée {v}");
                let a = Avant {
                    last: v,
                    ..avant(buy, Kind::Market, 100.0)
                };
                assert_eq!(controler(&a), Err(Refus::PrixAbsent), "dernier prix {v}");
            }
        }
        assert_eq!(Refus::PrixAbsent.message(true), "prix d'entrée manquant");
    }

    #[test]
    fn controle_sl_tp_absents() {
        for buy in [true, false] {
            for v in [0.0, f64::NAN] {
                let a = Avant {
                    sl: v,
                    ..avant(buy, Kind::Market, 100.0)
                };
                assert_eq!(controler(&a), Err(Refus::SlTpAbsents));
                let a = Avant {
                    tp: v,
                    ..avant(buy, Kind::Market, 100.0)
                };
                assert_eq!(controler(&a), Err(Refus::SlTpAbsents));
            }
        }
    }

    #[test]
    fn controle_sl_mauvais_cote() {
        // achat : SL au-dessus de l'entrée ou égal ; vente : SL sous l'entrée ou égal
        for (buy, sl) in [(true, 101.0), (true, 100.0), (false, 99.0), (false, 100.0)] {
            let a = Avant {
                sl,
                ..avant(buy, Kind::Market, 100.0)
            };
            assert_eq!(controler(&a), Err(Refus::SlMauvaisCote), "achat = {buy}, SL {sl}");
        }
        assert_eq!(Refus::SlMauvaisCote.message(true), "le SL doit être sous l'entrée");
        assert_eq!(Refus::SlMauvaisCote.message(false), "le SL doit être au-dessus de l'entrée");
    }

    #[test]
    fn controle_tp_mauvais_cote() {
        for (buy, tp) in [(true, 99.0), (true, 100.0), (false, 101.0), (false, 100.0)] {
            let a = Avant {
                tp,
                ..avant(buy, Kind::Market, 100.0)
            };
            assert_eq!(controler(&a), Err(Refus::TpMauvaisCote), "achat = {buy}, TP {tp}");
        }
        assert_eq!(Refus::TpMauvaisCote.message(true), "le TP doit être au-dessus de l'entrée");
        assert_eq!(Refus::TpMauvaisCote.message(false), "le TP doit être sous l'entrée");
    }

    #[test]
    fn controle_limite_incoherente_avec_le_sens() {
        let a = controler(&avant(true, Kind::Limit, 102.0));
        assert_eq!(a, Err(Refus::AttenteMalPlacee(OrderKind::Limit)));
        assert_eq!(a.unwrap_err().message(true), "un Buy Limit se place sous le prix actuel");
        let v = controler(&avant(false, Kind::Limit, 98.0));
        assert_eq!(v, Err(Refus::AttenteMalPlacee(OrderKind::Limit)));
        assert_eq!(v.unwrap_err().message(false), "un Sell Limit se place au-dessus du prix actuel");
        // au prix exact : refusé (ce serait un ordre au marché)
        assert!(controler(&avant(true, Kind::Limit, 100.0)).is_err());
    }

    #[test]
    fn controle_stop_incoherent_avec_le_sens() {
        let a = controler(&avant(true, Kind::Stop, 98.0));
        assert_eq!(a, Err(Refus::AttenteMalPlacee(OrderKind::Stop)));
        assert_eq!(a.unwrap_err().message(true), "un Buy Stop se place au-dessus du prix actuel");
        let v = controler(&avant(false, Kind::Stop, 102.0));
        assert_eq!(v, Err(Refus::AttenteMalPlacee(OrderKind::Stop)));
        assert_eq!(v.unwrap_err().message(false), "un Sell Stop se place sous le prix actuel");
        assert!(controler(&avant(false, Kind::Stop, 100.0)).is_err());
    }

    #[test]
    fn controle_taille_nulle_ou_negative() {
        for buy in [true, false] {
            for qty in [0.0, -1.0, f64::NAN, f64::INFINITY] {
                let a = Avant {
                    qty,
                    ..avant(buy, Kind::Market, 100.0)
                };
                assert_eq!(controler(&a), Err(Refus::TailleNulle), "achat = {buy}, taille {qty}");
            }
        }
        assert_eq!(Refus::TailleNulle.message(true), "la taille doit être plus grande que 0");
    }

    #[test]
    fn ticket_vente_et_limite_refusee_avant_envoi() {
        let pf = Portfolio::default();
        let vente = Ticket {
            buy: false,
            ..achat(Kind::Market, 0.0, 105.0, 90.0)
        };
        assert_eq!(vente.check(&pf, 100.0), Ok(100.0));
        // la Limite mal placée est refusée dès `check` (l'écran peut le dire avant l'envoi)
        let e = achat(Kind::Limit, 101.0, 95.0, 110.0).check(&pf, 100.0).unwrap_err();
        assert_eq!(e, "un Buy Limit se place sous le prix actuel");
        // le compte démo garde la même règle et le même message
        let mut pf2 = Portfolio::default();
        let o = achat(Kind::Limit, 101.0, 95.0, 110.0).order("XAU", 101.0);
        let e = pf2.place(OrderKind::Limit, o, 100.0, None, 0).unwrap_err();
        assert!(e.to_string().contains("un Buy Limit se place sous le prix actuel"), "{e}");
        let zero = Ticket {
            risk: Money::ZERO,
            ..achat(Kind::Market, 0.0, 95.0, 110.0)
        };
        assert!(zero.check(&pf, 100.0).unwrap_err().contains("risque"));
    }

    #[test]
    fn replay_vitesses() {
        assert_eq!(Replay::next_speed(1), 2);
        assert_eq!(Replay::next_speed(2), 5);
        assert_eq!(Replay::next_speed(5), 10);
        assert_eq!(Replay::next_speed(10), 1, "on revient à ×1");
        assert_eq!(Replay::next_speed(3), 2, "vitesse inconnue (ancienne sauvegarde) : comme ×1");
    }

    #[test]
    fn oco_depuis_deux_tickets() {
        let mut pf = Portfolio::default();
        let haut = achat(Kind::Stop, 105.0, 100.0, 115.0);
        let bas = Ticket {
            buy: false,
            sl: 100.0,
            tp: 85.0,
            price: 95.0,
            ..haut
        };
        let (a, b) = place_oco(&haut, &bas, &mut pf, "BTC", 100.0, 0, None, None).unwrap();
        assert_eq!(pf.pending.len(), 2);
        assert_eq!(pf.pending.iter().find(|p| p.id == a).unwrap().oco, Some(b));
        let marche = achat(Kind::Market, 0.0, 90.0, 110.0);
        assert!(place_oco(&marche, &bas, &mut pf, "BTC", 100.0, 0, None, None).is_err(), "au marché : pas d'OCO");
        assert_eq!(pf.pending.len(), 2, "rien de plus posé");
    }

    #[test]
    fn seance_mise_de_cote_puis_reprise() {
        let mut pf = Portfolio::default();
        let a = achat(Kind::Market, 100.0, 90.0, 120.0);
        let (pos, _) = a.place(&mut pf, "BTC", 100.0, 1_000, None, None).unwrap();
        let l = achat(Kind::Limit, 95.0, 90.0, 110.0);
        let (ord, _) = l.place(&mut pf, "BTC", 100.0, 1_000, None, None).unwrap();
        let solde = pf.balance;
        let (positions, pending) = pf.mettre_de_cote();
        assert!(pf.positions.is_empty() && pf.pending.is_empty(), "rien ne reste dans le compte du direct");
        assert_eq!((pf.balance, pf.history.len()), (solde, 0), "ni fermé ni payé");
        let s = Seance {
            exchange: "binance".into(),
            base: "BTC".into(),
            interval: 900,
            t: 5_000,
            speed: 5,
            prix: 101.0,
            positions,
            pending,
            saved_at: 6_000,
        };
        assert_eq!(s.ouverts(), 2);
        // la sauvegarde se relit (fichier JSON)
        let lu: Seance = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(lu, s);
        // entre-temps, un trade du direct prend un identifiant
        let (autre, _) = a.place(&mut pf, "ETH", 100.0, 7_000, None, None).unwrap();
        pf.remettre(lu.positions, lu.pending);
        assert_eq!((pf.positions.len(), pf.pending.len()), (2, 1));
        let ids: Vec<u64> = pf.positions.iter().map(|p| p.id).chain(pf.pending.iter().map(|p| p.id)).collect();
        assert_eq!(ids.len(), 3);
        assert!(
            ids.iter().all(|&i| ids.iter().filter(|&&j| j == i).count() == 1),
            "jamais deux fois le même : {ids:?}"
        );
        assert!(
            ids.contains(&autre) && !ids.contains(&ord) && !ids.contains(&pos),
            "les remis ont de nouveaux numéros"
        );
        // la position remise garde son heure d'ouverture (le journal la retrouve)
        assert!(pf.positions.iter().any(|p| p.base == "BTC" && p.opened_at == 1_000));
    }

    /// Module 31: a buy (`buy`) or sell of `qty` at `price` with no SL / TP, GTC, not reduce-only.
    fn ordre(buy: bool, kind: Kind, price: f64, qty: f64) -> OrderTicket {
        OrderTicket {
            buy,
            kind,
            price,
            qty,
            sl: None,
            tp: None,
            validity: Validity::Gtc,
            reduce_only: false,
        }
    }

    /// The exchange of the examples: maker 0.02 %, taker 0.045 %, slippage 0.01 %.
    fn bourse() -> Couts {
        Couts {
            maker: 0.0002,
            taker: 0.00045,
            ecart: 0.0,
            glissement: 0.0001,
        }
    }

    /// 🎫 Module 31: GTC waits under the price and fills at once (taker, at the last price) at or past it; IOC fills now
    /// or is refused; Post-only waits or is refused when it would fill now; a Stop on the wrong side is refused.
    #[test]
    fn ticket_31_gtc_ioc_post_only() {
        let last = 60_000.0;
        let mut pf = Portfolio::default();
        // GTC under the price: it waits, then fills as a maker at its own price
        let gtc = ordre(true, Kind::Limit, 59_000.0, 0.5);
        assert!(!gtc.marketable(last));
        let Ok(Placed::Resting(id)) = gtc.place(&mut pf, "BTC", last, 0, Some(bourse()), None) else {
            panic!()
        };
        assert_eq!(pf.pending[0].id, id);
        let (out, _) = pf.on_bar("BTC", &bar(60, 59_500.0, 59_600.0, 58_900.0, 59_200.0), rien);
        assert_eq!(out.filled.len(), 1);
        assert!(pf.positions[0].maker_in && pf.positions[0].entry == 59_000.0, "maker, at the order's price");
        // GTC at the last price or past it: it fills now, at the last price, as a taker
        for px in [last, 61_000.0] {
            let mut pf = Portfolio::default();
            assert_eq!(ordre(true, Kind::Limit, px, 0.5).fill_price(last), last);
            let Ok(Placed::Opened(_)) = ordre(true, Kind::Limit, px, 0.5).place(&mut pf, "BTC", last, 0, Some(bourse()), None) else {
                panic!()
            };
            assert!(!pf.positions[0].maker_in && pf.positions[0].entry == last, "{px}: taker at {last}");
        }
        // IOC: now or never (nothing waits)
        let mut pf = Portfolio::default();
        let ioc = |price| OrderTicket {
            validity: Validity::Ioc,
            ..ordre(true, Kind::Limit, price, 0.5)
        };
        assert_eq!(ioc(59_000.0).place(&mut pf, "BTC", last, 0, None, None), Err(IOC_REFUS.into()));
        assert!(pf.pending.is_empty() && pf.positions.is_empty());
        assert!(matches!(ioc(60_100.0).place(&mut pf, "BTC", last, 0, None, None), Ok(Placed::Opened(_))));
        // Post-only: it waits, or it is refused when it would fill now (buy at or above the price, sell at or below)
        let mut pf = Portfolio::default();
        let post = |buy, price| OrderTicket {
            validity: Validity::PostOnly,
            ..ordre(buy, Kind::Limit, price, 0.5)
        };
        for (buy, price) in [(true, 60_000.0), (true, 60_500.0), (false, 60_000.0), (false, 59_500.0)] {
            assert_eq!(
                post(buy, price).place(&mut pf, "BTC", last, 0, None, None),
                Err(POST_ONLY_REFUS.into()),
                "{buy} {price}"
            );
        }
        assert!(matches!(
            post(true, 59_500.0).place(&mut pf, "BTC", last, 0, None, None),
            Ok(Placed::Resting(_))
        ));
        assert!(matches!(
            post(false, 60_500.0).place(&mut pf, "BTC", last, 0, None, None),
            Ok(Placed::Resting(_))
        ));
        assert!(pf.positions.is_empty() && pf.pending.len() == 2);
        // a Stop waits for its trigger; on the wrong side it is refused
        let e = ordre(true, Kind::Stop, 59_000.0, 0.5).place(&mut pf, "BTC", last, 0, None, None).unwrap_err();
        assert_eq!(e, "un Buy Stop se place au-dessus du prix actuel");
        assert!(matches!(
            ordre(true, Kind::Stop, 61_000.0, 0.5).place(&mut pf, "BTC", last, 0, None, None),
            Ok(Placed::Resting(_))
        ));
        // the names of the ticket's menu
        let noms: Vec<(&str, &str)> = Validity::ALL.iter().map(|v| (v.name(), v.detail())).collect();
        assert_eq!(
            noms,
            [
                ("GTC", "Jusqu'à annulation"),
                ("IOC", "Tout de suite, ou annulé"),
                ("Post-only", "Seulement en maker")
            ]
        );
    }

    /// 🎫 Module 31: SL and TP are optional; set, they must be on the right side of the price the order fills at (a
    /// Limit past the price fills at the last price); a size is needed.
    #[test]
    fn ticket_31_tp_sl_facultatifs() {
        let last = 60_000.0;
        let mut pf = Portfolio::default();
        let Ok(Placed::Opened(id)) = ordre(true, Kind::Market, 0.0, 0.1).place(&mut pf, "BTC", last, 0, None, None) else {
            panic!()
        };
        let p = pf.positions.iter().find(|p| p.id == id).unwrap();
        assert_eq!(
            (p.stop, p.target, p.risk()),
            (None, None, 0.0),
            "no SL: no R (Argos AI says « Entré sans stop »)"
        );
        let avec = |sl, tp| OrderTicket {
            sl: Some(sl),
            tp: Some(tp),
            ..ordre(true, Kind::Limit, 61_000.0, 0.1)
        };
        // the Limit at 61 000 fills at 60 000: an SL at 60 500 would be above the entry
        assert_eq!(avec(60_500.0, 62_000.0).check(&pf, "BTC", last), Err("le SL doit être sous l'entrée".into()));
        assert_eq!(
            avec(59_500.0, 59_800.0).check(&pf, "BTC", last),
            Err("le TP doit être au-dessus de l'entrée".into())
        );
        assert_eq!(avec(59_500.0, 61_200.0).check(&pf, "BTC", last), Ok(()));
        for qty in [0.0, -1.0, f64::NAN] {
            assert_eq!(
                ordre(false, Kind::Market, 0.0, qty).check(&pf, "BTC", last),
                Err("la taille doit être plus grande que 0".into())
            );
        }
        assert_eq!(ordre(true, Kind::Limit, 0.0, 1.0).check(&pf, "BTC", last), Err("prix d'entrée manquant".into()));
    }

    /// 🔻 Module 31: reduce-only only takes off the other side (oldest first), never opens nor flips; at market now,
    /// a Limit waits (no margin held) and reduces as a maker; once the position is gone it is cancelled.
    #[test]
    fn reduce_only_ne_fait_que_reduire() {
        let last = 100.0;
        let perp = Perp::new(10.0, 0.005, 8.0);
        let mut pf = Portfolio::default();
        let reduit = |buy, kind, price, qty| OrderTicket {
            reduce_only: true,
            sl: Some(1.0),
            tp: Some(1_000.0),
            ..ordre(buy, kind, price, qty)
        };
        // nothing open: refused
        assert_eq!(
            reduit(false, Kind::Market, 0.0, 1.0).place(&mut pf, "XAU", last, 0, None, Some(perp)),
            Err(crate::portfolio::NOTHING_TO_REDUCE.into())
        );
        // two longs (3 then 2), and a short elsewhere
        for q in [3.0, 2.0] {
            ordre(true, Kind::Market, 0.0, q).place(&mut pf, "XAU", last, 0, None, Some(perp)).unwrap();
        }
        ordre(false, Kind::Market, 0.0, 1.0).place(&mut pf, "BTC", last, 0, None, Some(perp)).unwrap();
        assert_eq!(pf.reducible("XAU", false), 5.0);
        assert_eq!(pf.reducible("XAU", true), 0.0, "a buy has nothing to reduce on XAU");
        assert!(reduit(true, Kind::Market, 0.0, 1.0).place(&mut pf, "XAU", last, 1, None, Some(perp)).is_err());
        // at market: 4 off, the oldest first (3, then 1 of the 2)
        let Ok(Placed::Reduced(c)) = reduit(false, Kind::Market, 0.0, 4.0).place(&mut pf, "XAU", 110.0, 2, None, Some(perp)) else {
            panic!()
        };
        assert_eq!(
            c.iter().map(|c| (c.qty, c.partiel, c.reason.as_str())).collect::<Vec<_>>(),
            [(3.0, false, "reduce-only"), (1.0, true, "reduce-only")]
        );
        assert_eq!(pf.reducible("XAU", false), 1.0);
        // more than the position: only the position (never a short)
        let Ok(Placed::Reduced(c)) = reduit(false, Kind::Market, 0.0, 9.0).place(&mut pf, "XAU", 110.0, 3, None, Some(perp)) else {
            panic!()
        };
        assert_eq!(c[0].qty, 1.0);
        assert!(!pf.positions.iter().any(|p| p.base == "XAU"), "closed, nothing opened");
        // a reduce-only Limit waits without margin, and reduces as a maker when the price comes
        ordre(true, Kind::Market, 0.0, 2.0).place(&mut pf, "XAU", last, 4, None, Some(perp)).unwrap();
        let marge = pf.marge_utilisee();
        let Ok(Placed::Resting(id)) = reduit(false, Kind::Limit, 105.0, 1.5).place(&mut pf, "XAU", last, 5, None, Some(perp)) else {
            panic!()
        };
        assert!(pf.pending.iter().any(|p| p.id == id && p.reduce_only && p.order.stop.is_none()));
        assert_eq!(pf.marge_utilisee(), marge, "no margin held");
        let (out, closed) = pf.on_bar("XAU", &bar(6, 101.0, 106.0, 100.0, 104.0), rien);
        assert_eq!((out.reduced.as_slice(), out.closed.len()), ([id].as_slice(), 0), "on_bar hands the trades back");
        assert_eq!((closed.len(), closed[0].qty, closed[0].exit), (1, 1.5, 105.0));
        assert_eq!(pf.reducible("XAU", false), 0.5);
        assert!(pf.positions.iter().all(|p| p.side == Side::Long || p.base != "XAU"));
        // a reduce-only order whose position is gone is cancelled
        let Ok(Placed::Resting(id)) = reduit(false, Kind::Limit, 108.0, 0.5).place(&mut pf, "XAU", last, 7, None, Some(perp)) else {
            panic!()
        };
        let pos = pf.positions.iter().find(|p| p.base == "XAU").unwrap().id;
        pf.close(pos, 104.0, 8, "fermé à la main").unwrap();
        let out = pf.check_pending("XAU", 104.0, 104.0, 9, rien);
        assert_eq!(out.nothing_to_reduce, [id]);
        assert!(pf.pending.is_empty());
    }

    /// 🎫 Module 31: margin (value / leverage), fees (maker for a Limit that waits, taker plus slippage at market),
    /// Max (buying power / price, rounded down to the step; reduce-only: the position), « sans frais ».
    #[test]
    fn estimation_marge_frais_max() {
        let perp = Perp::new(10.0, 0.005, 8.0);
        let mut pf = Portfolio::default();
        let last = 60_100.0;
        let lim = ordre(true, Kind::Limit, 60_057.9, 0.8325);
        let e = estimate(&pf, &lim, "BTC", last, Some(bourse()), Some(perp));
        assert_eq!(e.margin, Money::from_cents(499_982), "49 998,20 / 10");
        assert_eq!((e.fees, e.slippage), (Money::from_cents(1_000), Money::ZERO), "maker: 0,02 %");
        assert_eq!(e.max_qty, 16.6505, "1 000 000 / 60 057,9, step 0,0001");
        assert_eq!(buying_power(&pf, Some(perp)), Money::from_cents(100_000_000));
        let mkt = ordre(true, Kind::Market, 0.0, 0.8325);
        let e = estimate(&pf, &mkt, "BTC", 60_057.9, Some(bourse()), Some(perp));
        assert_eq!(
            (e.fees, e.slippage),
            (Money::from_cents(2_250), Money::from_cents(500)),
            "taker 0,045 % + 0,01 %"
        );
        // spot: the whole value, ×1
        let e = estimate(&pf, &mkt, "BTC", 60_057.9, None, None);
        assert_eq!((e.margin, e.fees, e.max_qty), (Money::from_cents(4_999_820), Money::ZERO, 1.665));
        // « sans frais »: the exchange's costs are not paid
        pf.sans_frais = true;
        assert_eq!(estimate(&pf, &mkt, "BTC", 60_057.9, Some(bourse()), Some(perp)).fees, Money::ZERO);
        // reduce-only: no margin, Max = the position
        ordre(true, Kind::Market, 0.0, 0.8325)
            .place(&mut pf, "BTC", 60_057.9, 0, None, Some(perp))
            .unwrap();
        let red = OrderTicket {
            reduce_only: true,
            ..ordre(false, Kind::Market, 0.0, 0.5)
        };
        let e = estimate(&pf, &red, "BTC", 60_057.9, None, Some(perp));
        assert_eq!((e.margin, e.max_qty), (Money::ZERO, 0.8325));
        assert_eq!(estimate(&pf, &ordre(true, Kind::Market, 0.0, 0.0), "BTC", 0.0, None, None), Estimate::default());
    }

    /// 📏 The quantity step and its decimals, the rounding down; ⚖ Mid: the book's middle, or the last price.
    #[test]
    fn pas_de_quantite_et_mid() {
        let pas: Vec<f64> = [60_000.0, 3_000.0, 150.0, 0.15, 0.0].iter().map(|p| qty_step(*p)).collect();
        assert_eq!(pas, [0.0001, 0.001, 0.01, 1.0, 1.0]);
        assert_eq!(qty_decimals(0.0001), 4);
        assert_eq!(qty_decimals(1.0), 0);
        assert_eq!(round_qty(50_000.0 / 60_057.9, 0.0001), 0.8325);
        assert_eq!(round_qty(0.83, 0.0001), 0.83, "never below what was typed");
        assert_eq!(round_qty(2.9, 1.0), 2.0);
        assert_eq!(round_qty(-1.0, 0.01), 0.0);
        assert_eq!(mid(Some(60_057.0), Some(60_058.8), 60_058.0), 60_057.9);
        assert_eq!(mid(None, Some(60_058.8), 60_058.0), 60_058.0, "no book: the last price");
        assert_eq!(mid(Some(60_059.0), Some(60_057.0), 60_058.0), 60_058.0, "crossed book");
        assert_eq!(mid(Some(50_000.0), Some(50_010.0), 60_058.0), 60_058.0, "stale book (another market)");
    }
}
