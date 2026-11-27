//! 🧪 Pine strategies (`strategy(...)`, `strategy.entry`, `strategy.exit`, `strategy.close`…) played bar by bar on
//! the demo account's engine (`argos_core::portfolio::Portfolio`: money in cents, its fees), with TradingView's fill
//! rules (its « broker emulator »):
//! - an order given at a bar's close fills at the next bar's open (at that close with `process_orders_on_close`);
//! - limit and stop orders fill inside the bar, at their price (at the open when it is already past them);
//! - inside a bar the price goes open → the extreme nearer to the open → the other extreme → close (Jee chose, 21 Nov:
//!   « Comme TradingView » — the demo and the replay keep the worst case, stop first);
//! - an entry the other way closes the position and turns it around; without `pyramiding`, a second entry the same
//!   way is refused.
//!
//! The script reads the state at the bar's close (`strategy.position_size`, `strategy.equity`…) before its new orders.

use super::memoire;
use argos_core::money::Money;
use argos_core::portfolio::{ClosedTrade, Order, Portfolio};
use argos_core::{Bar, Side};
use std::rc::Rc;

/// How big an order is when the script gives no `qty` (`default_qty_type`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Taille {
    /// A number of contracts (`strategy.fixed`).
    #[default]
    Fixe,
    /// An amount of money (`strategy.cash`).
    Montant,
    /// A percentage of the account's equity (`strategy.percent_of_equity`).
    PctEquite,
}

/// ⚙ The group of the strategy's properties in the settings (after the script's own inputs).
pub const PROPRIETES: &str = "Propriétés de la stratégie";
/// ⚙ The order size types, as the settings show them (in `Taille`'s order).
pub const TAILLES: [&str; 3] = ["Contrats", "Montant", "% du compte"];

impl Taille {
    /// Its name in the settings.
    pub fn nom(self) -> &'static str {
        TAILLES[self as usize]
    }

    /// From its name in the settings.
    pub fn du_nom(n: &str) -> Option<Taille> {
        [Taille::Fixe, Taille::Montant, Taille::PctEquite].into_iter().find(|t| t.nom() == n)
    }

    /// From Pine's constant (« fixed », « cash », « percent_of_equity »).
    pub fn de(t: &str) -> Taille {
        match t {
            "cash" => Taille::Montant,
            "percent_of_equity" => Taille::PctEquite,
            _ => Taille::Fixe,
        }
    }
}

/// The strategy's properties (`strategy(...)` arguments), with TradingView's defaults.
#[derive(Debug, Clone, PartialEq)]
pub struct Reglages {
    /// `initial_capital` (1 000 000 by default).
    pub capital: f64,
    pub taille: Taille,
    /// `default_qty_value` (1 by default).
    pub valeur: f64,
    /// `pyramiding`: entries allowed the same way (0 and 1: one).
    pub pyramiding: u32,
    /// `commission_value` with `strategy.commission.percent`, in % of each fill's value.
    pub commission_pct: f64,
    /// `slippage`, in ticks, against the order (market and stop fills).
    pub glissement: f64,
    /// `process_orders_on_close`: market orders fill at the close of the bar that gives them.
    pub a_la_cloture: bool,
    /// `syminfo.mintick`: the price step of `profit`, `loss`, `trail_points`, `trail_offset`, `slippage`.
    pub tick: f64,
}

impl Default for Reglages {
    fn default() -> Self {
        Self {
            capital: 1_000_000.0,
            taille: Taille::Fixe,
            valeur: 1.0,
            pyramiding: 0,
            commission_pct: 0.0,
            glissement: 0.0,
            a_la_cloture: false,
            tick: 0.01,
        }
    }
}

/// An entry waiting to fill (`strategy.entry`, or `strategy.order` with `brut`).
#[derive(Debug, Clone, PartialEq)]
pub struct Entree {
    pub id: String,
    pub side: Side,
    /// Contracts (`None`: the default size, worked out when it fills).
    pub qty: Option<f64>,
    pub limit: Option<f64>,
    pub stop: Option<f64>,
    /// `strategy.order`: no turning around, no pyramiding limit (it reduces the other way first).
    pub brut: bool,
    pub comment: String,
    /// `oca_name` with `strategy.oca.cancel`: when it fills, the other orders of the group are cancelled.
    pub oca: String,
}

/// An exit (`strategy.exit`): a stop, a limit and / or a trailing stop for the trades of an entry.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Sortie {
    pub id: String,
    /// `from_entry` (empty: every entry).
    pub de: String,
    pub qty: Option<f64>,
    pub qty_pct: Option<f64>,
    /// In ticks from the entry price.
    pub profit: Option<f64>,
    pub loss: Option<f64>,
    pub limit: Option<f64>,
    pub stop: Option<f64>,
    pub trail_price: Option<f64>,
    pub trail_points: Option<f64>,
    pub trail_offset: Option<f64>,
    pub comment: String,
    /// It has had a trade to protect: once none is left, it is gone (TradingView ties it to the position).
    pub(crate) vu: bool,
    /// Its id, shared by every open trade it trails (made when the first one starts, kept when the same id
    /// replaces the order): 450 trades under 450 trailing exits kept 202 500 copies of the ids, 800 MB made in
    /// one step of the broker (B-062).
    pub(crate) cle: Option<Rc<str>>,
}

/// A market close (`strategy.close`, `strategy.close_all`).
#[derive(Debug, Clone, PartialEq)]
pub struct Fermeture {
    /// The entry whose trades close (`None`: all of them).
    pub de: Option<String>,
    pub qty: Option<f64>,
    pub qty_pct: Option<f64>,
    pub comment: String,
}

/// A market order waiting for the next open (or the close, `immediately` / `process_orders_on_close`).
#[derive(Debug, Clone, PartialEq)]
enum Marche {
    Entree(Entree),
    Fermer(Fermeture),
}

/// An open trade: one fill of an entry.
#[derive(Debug, Clone, PartialEq)]
struct Ouvert {
    /// Its position in the portfolio.
    pf: u64,
    entree: String,
    side: Side,
    qty: f64,
    prix: f64,
    bar: usize,
    /// Exits already filled for it (an exit fills once per trade).
    faites: Vec<String>,
    /// Trailing stops: (exit id, shared with its order, best price since it started trailing).
    suivis: Vec<(Rc<str>, f64)>,
}

/// A closed trade (or a part of one), with the ids of its entry and exit.
#[derive(Debug, Clone, PartialEq)]
pub struct Trade {
    pub entree: String,
    pub sortie: String,
    pub bar_in: usize,
    pub bar_out: usize,
    /// Side, quantity, prices, P&L and fees in cents, times.
    pub ferme: ClosedTrade,
}

/// A trade still open at the last bar.
#[derive(Debug, Clone, PartialEq)]
pub struct Ouverture {
    pub entree: String,
    pub side: Side,
    pub qty: f64,
    pub prix: f64,
    pub bar: usize,
    /// Its result at the last close (before exit fees).
    pub resultat: f64,
}

/// A fill drawn on the chart: an arrow with the order's name and its quantity, like TradingView.
#[derive(Debug, Clone, PartialEq)]
pub struct Marque {
    pub bar: usize,
    pub prix: f64,
    pub achat: bool,
    pub texte: String,
    pub qty: f64,
}

/// What a strategy did over the bars.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Bilan {
    pub reglages: Reglages,
    pub trades: Vec<Trade>,
    pub ouverts: Vec<Ouverture>,
    pub marques: Vec<Marque>,
    /// Equity at each bar's close (capital + closed results + open results).
    pub equite: Vec<f64>,
    /// Closed results, fees taken off.
    pub net: Money,
}

/// The broker: the strategy's orders, its trades and its portfolio, bar after bar.
#[derive(Debug, Clone)]
pub struct Courtier {
    pub r: Reglages,
    pf: Portfolio,
    marche: Vec<Marche>,
    /// Market orders to fill at this bar's close (`immediately`, `process_orders_on_close`).
    cloture: Vec<Marche>,
    attente: Vec<Entree>,
    sorties: Vec<Sortie>,
    ouverts: Vec<Ouvert>,
    trades: Vec<Trade>,
    marques: Vec<Marque>,
    equite: Vec<f64>,
    /// The bar being played and its time, its close (for the default size and the open results).
    i: usize,
    t: i64,
    close: f64,
    /// The last entry filled (`strategy.position_entry_name`).
    derniere: String,
}

impl Courtier {
    /// 🧮 The bytes the broker holds (B-062): its orders, its trades, its marks and their texts (an id, a comment
    /// are the script's). Every field is named: a new one has to be placed, counted or not.
    pub(crate) fn poids(&self) -> usize {
        use std::mem::size_of;
        let Courtier {
            r: _,
            pf,
            marche,
            cloture,
            attente,
            sorties,
            ouverts,
            trades,
            marques,
            equite,
            i: _,
            t: _,
            close: _,
            derniere,
        } = self;
        let t = String::capacity;
        // (a shared id: each of its holders counts its part of it)
        let part = |cle: &Rc<str>| cle.len() / Rc::strong_count(cle) + 1;
        fn liste<T>(v: &[T], places: usize, dedans: impl Fn(&T) -> usize) -> usize {
            places.saturating_mul(size_of::<T>()).saturating_add(v.iter().map(dedans).sum())
        }
        let entree = |e: &Entree| t(&e.id) + t(&e.comment) + t(&e.oca);
        let marche_de = |m: &Marche| match m {
            Marche::Entree(e) => entree(e),
            Marche::Fermer(f) => f.de.as_ref().map_or(0, t) + t(&f.comment),
        };
        let ferme = |c: &ClosedTrade| t(&c.base) + t(&c.reason);
        [
            liste(marche, marche.capacity(), marche_de),
            liste(cloture, cloture.capacity(), marche_de),
            liste(attente, attente.capacity(), entree),
            liste(sorties, sorties.capacity(), |s| {
                t(&s.id) + t(&s.de) + t(&s.comment) + s.cle.as_ref().map_or(0, part)
            }),
            liste(ouverts, ouverts.capacity(), |o| {
                t(&o.entree) + liste(&o.faites, o.faites.capacity(), t) + liste(&o.suivis, o.suivis.capacity(), |s| part(&s.0))
            }),
            liste(trades, trades.capacity(), |x| t(&x.entree) + t(&x.sortie) + ferme(&x.ferme)),
            liste(marques, marques.capacity(), |m| t(&m.texte)),
            equite.capacity().saturating_mul(size_of::<f64>()),
            t(derniere),
            // (the portfolio of the broker: its open positions, its closed trades, its curve)
            liste(&pf.positions, pf.positions.capacity(), |p| t(&p.base)),
            liste(&pf.history, pf.history.capacity(), ferme),
            liste(&pf.pending, pf.pending.capacity(), |p| t(&p.order.base)),
            pf.curve.capacity().saturating_mul(size_of::<(i64, Money)>()),
        ]
        .iter()
        .fold(0, |a, b| a.saturating_add(*b))
    }
}

/// 🧪 For the tests: the size of each kind of entry the broker adds to its lists.
#[cfg(test)]
pub(crate) fn tailles() -> [usize; 8] {
    use std::mem::size_of;
    fn place<T>(_: &[T]) -> usize {
        size_of::<T>()
    }
    [
        size_of::<Marche>(),
        size_of::<Entree>(),
        size_of::<Sortie>(),
        size_of::<Ouvert>(),
        size_of::<Trade>(),
        size_of::<Marque>(),
        size_of::<ClosedTrade>(),
        // (an open position of the portfolio, read from its list)
        place(&Portfolio::new("t", 0.0, 0.0).positions),
    ]
}

const BASE: &str = "strategie";
/// Below this, a quantity is nothing (rounding).
const RIEN: f64 = 1e-12;

/// Where an order fills while the price moves from `a` to `b` (continuously): `None` if it does not. `haut`: it fills
/// when the price is at or above `niveau` (buy stop, sell limit); otherwise at or below it. Already past at `a`: at `a`.
fn touche(a: f64, b: f64, haut: bool, niveau: f64) -> Option<f64> {
    if !niveau.is_finite() {
        return None;
    }
    let (deja, atteint) = if haut { (a >= niveau, b >= niveau) } else { (a <= niveau, b <= niveau) };
    if deja {
        Some(a)
    } else if atteint {
        Some(niveau)
    } else {
        None
    }
}

/// The path of the price inside a bar (TradingView): open, the extreme nearer to the open, the other one, close.
pub fn chemin(b: &Bar) -> [f64; 4] {
    if b.high - b.open < b.open - b.low {
        [b.open, b.high, b.low, b.close]
    } else {
        [b.open, b.low, b.high, b.close]
    }
}

/// What fills next on the path.
#[derive(Debug, Clone, PartialEq)]
enum Coup {
    /// A waiting entry (index in `attente`).
    Entree(usize),
    /// An exit of an open trade: (trade index, exit index, a stop or a trailing stop: they pay the slippage).
    Sortie(usize, usize, bool),
}

impl Courtier {
    pub fn new(r: Reglages) -> Self {
        let pf = Portfolio::new("Stratégie", r.capital, r.commission_pct.max(0.0) / 100.0);
        Courtier {
            r,
            pf,
            marche: vec![],
            cloture: vec![],
            attente: vec![],
            sorties: vec![],
            ouverts: vec![],
            trades: vec![],
            marques: vec![],
            equite: vec![],
            i: 0,
            t: 0,
            close: f64::NAN,
            derniere: String::new(),
        }
    }

    // ── orders given by the script ──────────────────────────────────────────────────────────────────────────────

    /// `strategy.entry` / `strategy.order`: a market order without `limit` nor `stop`, else a waiting one. The same id
    /// replaces the order still waiting.
    pub fn entree(&mut self, e: Entree, immediatement: bool) {
        self.annuler_entree(&e.id);
        if e.limit.is_some() || e.stop.is_some() {
            self.attente.push(e);
        } else if immediatement || self.r.a_la_cloture {
            self.cloture.push(Marche::Entree(e));
        } else {
            self.marche.push(Marche::Entree(e));
        }
    }

    /// `strategy.exit`: the same id replaces it (its new levels apply from the next price).
    pub fn sortie(&mut self, s: Sortie) {
        match self.sorties.iter_mut().find(|x| x.id == s.id && x.de == s.de) {
            Some(x) => {
                *x = Sortie {
                    vu: x.vu,
                    cle: x.cle.take(),
                    ..s
                }
            }
            None => self.sorties.push(s),
        }
    }

    /// `strategy.close` / `strategy.close_all`: at the next open, or at this close (`immediately`).
    pub fn fermer(&mut self, f: Fermeture, immediatement: bool) {
        if immediatement || self.r.a_la_cloture {
            self.cloture.push(Marche::Fermer(f));
        } else {
            self.marche.push(Marche::Fermer(f));
        }
    }

    /// `strategy.cancel(id)`: the entries and exits still waiting with this id. `None`: all of them
    /// (`strategy.cancel_all`).
    pub fn annuler(&mut self, id: Option<&str>) {
        match id {
            Some(id) => {
                self.annuler_entree(id);
                self.sorties.retain(|s| s.id != id);
            }
            None => {
                self.attente.clear();
                self.marche.retain(|m| matches!(m, Marche::Fermer(_)));
                self.sorties.clear();
            }
        }
    }

    fn annuler_entree(&mut self, id: &str) {
        self.attente.retain(|e| e.id != id);
        self.marche.retain(|m| !matches!(m, Marche::Entree(e) if e.id == id));
        self.cloture.retain(|m| !matches!(m, Marche::Entree(e) if e.id == id));
    }

    // ── what the script reads ───────────────────────────────────────────────────────────────────────────────────

    /// `strategy.position_size`: contracts, negative when short.
    pub fn position(&self) -> f64 {
        self.ouverts.iter().map(|o| o.qty * o.side.sign()).sum()
    }

    /// `strategy.position_avg_price` (NaN when flat).
    pub fn prix_moyen(&self) -> f64 {
        let q: f64 = self.ouverts.iter().map(|o| o.qty).sum();
        if q > RIEN {
            self.ouverts.iter().map(|o| o.qty * o.prix).sum::<f64>() / q
        } else {
            f64::NAN
        }
    }

    /// The open trades' result at `prix`, before any fee.
    pub fn latent(&self, prix: f64) -> f64 {
        self.ouverts.iter().map(|o| (prix - o.prix) * o.qty * o.side.sign()).sum()
    }

    /// `strategy.netprofit`: the closed trades' results, fees taken off (like TradingView, an open trade is not in it,
    /// not even its entry fee: that one is in `strategy.openprofit`).
    pub fn net(&self) -> f64 {
        self.net_ferme().to_f64()
    }

    fn net_ferme(&self) -> Money {
        self.trades.iter().map(|t| t.ferme.net()).sum()
    }

    /// `strategy.equity` at `prix`: the account (closed results and the fees already paid) and the open trades.
    pub fn equite_a(&self, prix: f64) -> f64 {
        (self.pf.balance - self.pf.start).to_f64() + self.r.capital + self.latent(prix)
    }

    /// A `strategy.*` variable at the bar's close (`None`: not one of them).
    pub fn lire(&self, nom: &str) -> Option<f64> {
        let nets = || self.trades.iter().map(|t| t.ferme.net().to_f64());
        Some(match nom {
            "position_size" => self.position(),
            "position_avg_price" => self.prix_moyen(),
            // (equity = initial capital + net profit + open profit, as TradingView has it)
            "openprofit" => self.equite_a(self.close) - self.r.capital - self.net(),
            "netprofit" => self.net(),
            "equity" => self.equite_a(self.close),
            "initial_capital" => self.r.capital,
            "opentrades" => self.ouverts.len() as f64,
            "closedtrades" => self.trades.len() as f64,
            "wintrades" => nets().filter(|n| *n > 0.0).count() as f64,
            "losstrades" => nets().filter(|n| *n < 0.0).count() as f64,
            "eventrades" => nets().filter(|n| *n == 0.0).count() as f64,
            "grossprofit" => nets().filter(|n| *n > 0.0).sum(),
            "grossloss" => -nets().filter(|n| *n < 0.0).sum::<f64>(),
            "max_drawdown" => {
                let (mut haut, mut pire) = (f64::MIN, 0.0f64);
                for e in &self.equite {
                    haut = haut.max(*e);
                    pire = pire.max(haut - e);
                }
                pire
            }
            _ => return None,
        })
    }

    /// `strategy.position_entry_name`.
    pub fn nom_entree(&self) -> &str {
        if self.ouverts.is_empty() {
            ""
        } else {
            &self.derniere
        }
    }

    // ── the bars ────────────────────────────────────────────────────────────────────────────────────────────────

    /// Before the script runs on bar `i`: the market orders fill at its open, then the waiting entries and the exits
    /// along its path.
    pub fn avant(&mut self, i: usize, b: &Bar) {
        (self.i, self.t, self.close) = (i, b.time, b.close);
        for m in std::mem::take(&mut self.marche) {
            self.au_marche(m, b.open);
        }
        let pts = chemin(b);
        for s in 0..3 {
            let (mut a, fin) = (pts[s], pts[s + 1]);
            // (a level reached at a segment's start counts there: a trailing stop starts as soon as it is reached)
            self.suivre(a);
            // (a bounded number of fills per segment: each one moves the price along, and orders are finite)
            for _ in 0..1000 {
                let Some((coup, prix)) = self.prochain(a, fin) else { break };
                self.remplir(coup, prix);
                a = prix;
            }
            self.suivre(fin);
        }
    }

    /// After the script ran on bar `i`: the orders for this close (`immediately`, `process_orders_on_close`), the
    /// exits left without a trade, the equity at the close.
    pub fn apres(&mut self, b: &Bar) {
        for m in std::mem::take(&mut self.cloture) {
            self.au_marche(m, b.close);
        }
        self.oublier_sorties();
        self.equite.push(self.equite_a(b.close));
    }

    /// What the strategy did, its open trades valued at the last close.
    pub fn bilan(self) -> Bilan {
        let ouverts = self
            .ouverts
            .iter()
            .map(|o| Ouverture {
                entree: o.entree.clone(),
                side: o.side,
                qty: o.qty,
                prix: o.prix,
                bar: o.bar,
                resultat: (self.close - o.prix) * o.qty * o.side.sign(),
            })
            .collect();
        Bilan {
            net: self.net_ferme(),
            reglages: self.r,
            trades: self.trades,
            ouverts,
            marques: self.marques,
            equite: self.equite,
        }
    }

    // ── fills ───────────────────────────────────────────────────────────────────────────────────────────────────

    /// The price after slippage, against the order (`achat`: it pays more).
    fn glisse(&self, prix: f64, achat: bool) -> f64 {
        let g = self.r.glissement.max(0.0) * self.r.tick;
        if achat {
            prix + g
        } else {
            (prix - g).max(self.r.tick)
        }
    }

    fn au_marche(&mut self, m: Marche, prix: f64) {
        match m {
            Marche::Entree(e) => {
                let p = self.glisse(prix, e.side == Side::Long);
                self.entrer(&e, p);
            }
            Marche::Fermer(f) => self.fermer_au(&f, prix),
        }
    }

    /// `strategy.default_entry_qty(fill_price)`.
    pub fn taille_par_defaut(&self, prix: f64) -> f64 {
        self.taille(prix)
    }

    /// The default size of an order at `prix` (`default_qty_type` / `default_qty_value`).
    fn taille(&self, prix: f64) -> f64 {
        match self.r.taille {
            Taille::Fixe => self.r.valeur,
            Taille::Montant => self.r.valeur / prix,
            Taille::PctEquite => self.equite_a(prix) * self.r.valeur / 100.0 / prix,
        }
    }

    /// An entry fills at `prix` (slippage already in): it turns the position around, or adds to it (`pyramiding`), or
    /// for `strategy.order` reduces the other way first.
    fn entrer(&mut self, e: &Entree, prix: f64) {
        if !(prix > 0.0) {
            return;
        }
        let qty = e.qty.filter(|q| *q > 0.0).unwrap_or_else(|| self.taille(prix));
        if !(qty > RIEN) {
            return;
        }
        let autre = |o: &Ouvert| o.side != e.side;
        if e.brut {
            // strategy.order: the other way's trades first (oldest first), then the rest opens
            let mut reste = qty;
            while reste > RIEN {
                let Some(k) = self.ouverts.iter().position(autre) else { break };
                let q = reste.min(self.ouverts[k].qty);
                self.sortir(k, q, prix, &e.id, e.comment.clone());
                reste -= q;
            }
            if reste > RIEN {
                self.ouvrir(e, reste, prix);
            }
        } else {
            let retourne = self.ouverts.iter().any(autre);
            let meme = self.ouverts.iter().filter(|o| o.side == e.side).count();
            if !retourne && meme >= self.r.pyramiding.max(1) as usize {
                return; // pyramiding: refused (it is not kept waiting)
            }
            while let Some(k) = self.ouverts.iter().position(autre) {
                let q = self.ouverts[k].qty;
                self.sortir(k, q, prix, &e.id, e.comment.clone());
            }
            self.ouvrir(e, qty, prix);
        }
        if !e.oca.is_empty() {
            let (oca, id) = (e.oca.clone(), e.id.clone());
            self.attente.retain(|x| x.oca != oca || x.id == id);
        }
    }

    fn ouvrir(&mut self, e: &Entree, qty: f64, prix: f64) {
        let o = Order {
            base: BASE.into(),
            side: e.side,
            qty,
            price: prix,
            stop: None,
            target: None,
            couts: None,
            perp: None,
        };
        let Ok(pf) = self.pf.open(o, self.t) else { return };
        // (B-062: a fill adds its entries between two statements of the script: counted here, with their texts)
        memoire::creer(3 * memoire::APPEL_SORTIE + 3 * (e.id.len() + e.comment.len()));
        self.ouverts.push(Ouvert {
            pf,
            entree: e.id.clone(),
            side: e.side,
            qty,
            prix,
            bar: self.i,
            faites: vec![],
            suivis: vec![],
        });
        self.derniere = e.id.clone();
        let texte = if e.comment.is_empty() { e.id.clone() } else { e.comment.clone() };
        self.marques.push(Marque {
            bar: self.i,
            prix,
            achat: e.side == Side::Long,
            texte,
            qty,
        });
    }

    /// `qty` of open trade `k` leaves at `prix` (the exit `sortie`, its label `texte`).
    fn sortir(&mut self, k: usize, qty: f64, prix: f64, sortie: &str, texte: String) {
        let o = &self.ouverts[k];
        let (pf, entree, side, bar_in) = (o.pf, o.entree.clone(), o.side, o.bar);
        let Ok(ferme) = self.pf.close_part(pf, qty, prix, self.t, sortie) else {
            return;
        };
        // (B-062: as in `ouvrir`: the mark, the trade and its copy in the portfolio, the exit done or followed)
        memoire::creer(4 * memoire::APPEL_SORTIE + 2 * entree.len() + 5 * sortie.len() + texte.len());
        let texte = if texte.is_empty() { sortie.to_string() } else { texte };
        self.marques.push(Marque {
            bar: self.i,
            prix,
            achat: side == Side::Short,
            texte,
            qty,
        });
        self.trades.push(Trade {
            entree,
            sortie: sortie.to_string(),
            bar_in,
            bar_out: self.i,
            ferme,
        });
        self.ouverts[k].qty -= qty;
        // (all of it out: the portfolio closed the position)
        if self.ouverts[k].qty <= RIEN || !self.pf.positions.iter().any(|p| p.id == pf) {
            self.ouverts.remove(k);
        }
    }

    /// `strategy.close` / `close_all` at `prix`: the trades of the entry (all of them without one), oldest first.
    fn fermer_au(&mut self, f: &Fermeture, prix: f64) {
        let du = |o: &Ouvert| f.de.as_deref().is_none_or(|d| o.entree == d);
        let total: f64 = self.ouverts.iter().filter(|o| du(o)).map(|o| o.qty).sum();
        let mut reste = match (f.qty, f.qty_pct) {
            (Some(q), _) if q > 0.0 => q.min(total),
            (_, Some(p)) => total * p.clamp(0.0, 100.0) / 100.0,
            _ => total,
        };
        let nom = match &f.de {
            Some(d) => format!("Close entry(s) order {d}"),
            None => "Close position order".to_string(),
        };
        while reste > RIEN {
            let Some(k) = self.ouverts.iter().position(du) else { break };
            let q = reste.min(self.ouverts[k].qty);
            let p = self.glisse(prix, self.ouverts[k].side == Side::Short);
            self.sortir(k, q, p, &nom, f.comment.clone());
            reste -= q;
        }
    }

    // ── the path inside a bar ───────────────────────────────────────────────────────────────────────────────────

    /// The levels of exit `s` for open trade `o`: (stop, limit, trailing stop now).
    fn niveaux(&self, s: &Sortie, o: &Ouvert) -> (Option<f64>, Option<f64>, Option<f64>) {
        let (sens, tick) = (o.side.sign(), self.r.tick);
        // the stop the price reaches first (the closer one), the limit too
        let plus_proche = |a: Option<f64>, b: Option<f64>, vers_le_haut: bool| match (a, b) {
            (Some(x), Some(y)) => Some(if vers_le_haut { x.min(y) } else { x.max(y) }),
            (x, y) => x.or(y),
        };
        let stop = plus_proche(s.stop, s.loss.map(|l| o.prix - sens * l * tick), o.side == Side::Short);
        let limit = plus_proche(s.limit, s.profit.map(|p| o.prix + sens * p * tick), o.side == Side::Long);
        let suivi = s
            .trail_offset
            .and_then(|off| o.suivis.iter().find(|x| *x.0 == *s.id).map(|x| x.1 - sens * off * tick));
        (stop, limit, suivi)
    }

    /// The exit `s` applies to open trade `o`.
    fn pour(s: &Sortie, o: &Ouvert) -> bool {
        (s.de.is_empty() || s.de == o.entree) && !o.faites.contains(&s.id)
    }

    /// The next fill while the price goes from `a` to `b`: the order reached first.
    fn prochain(&self, a: f64, b: f64) -> Option<(Coup, f64)> {
        let mut mieux: Option<(Coup, f64)> = None;
        let mut garder = |c: Coup, p: f64| {
            if mieux.as_ref().is_none_or(|m| (p - a).abs() < (m.1 - a).abs()) {
                mieux = Some((c, p));
            }
        };
        for (k, e) in self.attente.iter().enumerate() {
            let achat = e.side == Side::Long;
            // stop-limit: the stop first, then it waits as a limit (see `remplir`)
            if let Some(s) = e.stop {
                if let Some(p) = touche(a, b, achat, s) {
                    garder(Coup::Entree(k), p);
                }
            } else if let Some(l) = e.limit {
                if let Some(p) = touche(a, b, !achat, l) {
                    garder(Coup::Entree(k), p);
                }
            }
        }
        for (k, o) in self.ouverts.iter().enumerate() {
            let vend = o.side == Side::Long;
            for (j, s) in self.sorties.iter().enumerate() {
                if !Self::pour(s, o) {
                    continue;
                }
                let (stop, limit, suivi) = self.niveaux(s, o);
                // (a long's stop and trailing stop fill on the way down, its limit on the way up)
                for niveau in [stop, suivi] {
                    if let Some(p) = niveau.and_then(|n| touche(a, b, !vend, n)) {
                        garder(Coup::Sortie(k, j, true), p);
                    }
                }
                if let Some(p) = limit.and_then(|n| touche(a, b, vend, n)) {
                    garder(Coup::Sortie(k, j, false), p);
                }
            }
        }
        mieux
    }

    /// Fills what `prochain` found at `prix`.
    fn remplir(&mut self, c: Coup, prix: f64) {
        match c {
            Coup::Entree(k) => {
                let e = self.attente.remove(k);
                if let (Some(_), Some(l)) = (e.stop, e.limit) {
                    // stop-limit: its stop is reached, it now waits as a limit order
                    self.attente.push(Entree {
                        stop: None,
                        limit: Some(l),
                        ..e
                    });
                    return;
                }
                let achat = e.side == Side::Long;
                // (a stop pays the slippage, a limit fills at its price)
                let p = if e.stop.is_some() { self.glisse(prix, achat) } else { prix };
                self.entrer(&e, p);
            }
            Coup::Sortie(k, j, au_stop) => {
                let (s, o) = (self.sorties[j].clone(), &self.ouverts[k]);
                let qty = match (s.qty, s.qty_pct) {
                    (Some(q), _) if q > 0.0 => q.min(o.qty),
                    (_, Some(p)) => o.qty * p.clamp(0.0, 100.0) / 100.0,
                    _ => o.qty,
                };
                let p = if au_stop { self.glisse(prix, o.side == Side::Short) } else { prix };
                self.ouverts[k].faites.push(s.id.clone());
                self.sortir(k, qty.max(RIEN), p, &s.id, s.comment.clone());
            }
        }
    }

    /// The trailing stops follow the price: they start once it reaches their level (`trail_price`, or `trail_points`
    /// from the entry), then keep the best price seen.
    fn suivre(&mut self, prix: f64) {
        let tick = self.r.tick;
        let sorties = &mut self.sorties;
        for o in &mut self.ouverts {
            let sens = o.side.sign();
            for s in sorties.iter_mut().filter(|s| (s.de.is_empty() || s.de == o.entree) && s.trail_offset.is_some()) {
                let depart = s.trail_price.or(s.trail_points.map(|p| o.prix + sens * p * tick));
                match o.suivis.iter_mut().find(|x| *x.0 == *s.id) {
                    Some(x) => {
                        if sens * (prix - x.1) > 0.0 {
                            x.1 = prix;
                        }
                    }
                    None if depart.is_some_and(|d| sens * (prix - d) >= 0.0) => {
                        // (B-062: this runs between two bars, where no call of the script is: the place of the
                        // trade is counted here, and the id is shared, never copied for each trade)
                        memoire::creer(memoire::APPEL_SORTIE);
                        let cle = s.cle.get_or_insert_with(|| memoire::cle(&s.id));
                        o.suivis.push((cle.clone(), prix));
                    }
                    None => {}
                }
            }
        }
    }

    /// The exits that protected trades and have none left are gone.
    fn oublier_sorties(&mut self) {
        let ouverts = &self.ouverts;
        for s in &mut self.sorties {
            s.vu |= ouverts.iter().any(|o| s.de.is_empty() || s.de == o.entree);
        }
        self.sorties
            .retain(|s| !s.vu || ouverts.iter().any(|o| (s.de.is_empty() || s.de == o.entree) && !o.faites.contains(&s.id)));
    }
}
