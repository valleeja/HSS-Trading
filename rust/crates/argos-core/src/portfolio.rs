//! Portfolio : comptes démo, positions, historique, équité. Calcul pur, aucune donnée réseau.

use crate::couts::{Couts, Taux};
use crate::money::Money;
use crate::perp::{self, Perp};
use crate::types::{ArgosError, Bar, Result, Side};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Position {
    pub id: u64,
    pub base: String,
    pub side: Side,
    pub qty: f64,
    pub entry: f64,
    pub stop: Option<f64>,
    pub target: Option<f64>,
    pub opened_at: i64,
    /// 🎛 Gestion automatique (break-even, stop suiveur, objectifs partiels) ; absente des anciennes sauvegardes.
    #[serde(default)]
    pub gestion: Gestion,
    /// Stop et quantité à l'ouverture : le risque de départ (le R) ne bouge pas quand le stop monte ou qu'une partie
    /// sort. Absents des anciennes sauvegardes : le stop et la quantité actuels.
    #[serde(default)]
    pub stop0: Option<f64>,
    #[serde(default)]
    pub qty0: Option<f64>,
    /// Meilleur prix vu depuis l'ouverture (plus haut à l'achat, plus bas à la vente) : le stop suiveur le suit.
    #[serde(default)]
    pub extreme: Option<f64>,
    /// 💸 Coûts de sa bourse (frais maker / taker, écart, glissement), pris à l'ouverture ; absents (anciennes
    /// sauvegardes, backtest, « sans frais ») : les frais du compte (`fee_rate`).
    #[serde(default)]
    pub couts: Option<Couts>,
    /// Entrée par un ordre Limite (frais maker) ; sinon au marché ou par un Stop (taker).
    #[serde(default)]
    pub maker_in: bool,
    /// ⚡ Perpétuel : levier, marge de maintien et intervalle du funding (marge isolée, prix de liquidation).
    /// Absent (marchés au comptant, anciennes sauvegardes) : ni marge, ni liquidation, ni funding.
    #[serde(default)]
    pub perp: Option<Perp>,
    /// ⚡ Funding payé depuis l'ouverture (négatif : reçu), déjà retiré du solde.
    #[serde(default)]
    pub funding: Money,
    /// ⚡ Heure (secondes UTC) jusqu'où le funding est payé : les paiements suivants viennent après.
    #[serde(default)]
    pub funding_vu: i64,
    /// 🤖 Objectif à l'ouverture (le plan de départ) ; absent des anciennes sauvegardes : l'objectif actuel.
    #[serde(default)]
    pub target0: Option<f64>,
    /// 🤖 Ce que la main a changé au stop et à l'objectif depuis l'ouverture.
    #[serde(default)]
    pub retouches: Retouches,
    /// 🤖 Ce que disaient les indicateurs de la chart à l'ouverture (`None` : pas lus).
    #[serde(default)]
    pub lecture: Option<Lecture>,
}

impl Position {
    pub fn pnl(&self, price: f64) -> f64 {
        (price - self.entry) * self.qty * self.side.sign()
    }
    /// 💰 Résultat ouvert au prix `price`, en décimales exactes, arrondi au centime.
    pub fn resultat(&self, price: f64) -> Money {
        Money::resultat(price, self.entry, self.qty, self.side.sign())
    }
    /// Perte si le stop est touché (0 sans stop).
    pub fn risk(&self) -> f64 {
        self.stop.map(|s| ((self.entry - s) * self.side.sign()).max(0.0) * self.qty).unwrap_or(0.0)
    }
    /// Écart de départ entre l'entrée et le stop (1 R par unité ; 0 sans stop).
    pub fn r_unite(&self) -> f64 {
        self.stop0.or(self.stop).map_or(0.0, |s| ((self.entry - s) * self.side.sign()).max(0.0))
    }
    /// Risque de départ de `qty` unités (le R d'une sortie, même après un break-even).
    pub fn risk0(&self, qty: f64) -> f64 {
        self.r_unite() * qty
    }
    /// Quantité à l'ouverture.
    pub fn qty_depart(&self) -> f64 {
        self.qty0.unwrap_or(self.qty)
    }
    /// ⚡ 💰 Marge isolée tenue par la position (0 hors perpétuel).
    pub fn marge(&self) -> Money {
        self.perp.map_or(Money::ZERO, |x| x.marge(self.qty, self.entry))
    }
    /// ⚡ 💀 Prix de liquidation (`None` hors perpétuel).
    pub fn liquidation(&self) -> Option<f64> {
        self.perp.map(|x| x.liquidation(self.side == Side::Long, self.entry))
    }
}

/// 🎯 Un objectif partiel : à `prix`, sortir `part` (0 à 1) de la quantité de départ.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Palier {
    pub prix: f64,
    pub part: f64,
}

/// 🎛 Gestion d'une position, posée avec l'ordre ou plus tard : tout est fait bougie par bougie (en replay) ou au
/// dernier prix (en direct), jamais en devinant l'ordre des prix dans une bougie.
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub struct Gestion {
    /// ⚖ Break-even : quand le prix a fait `be_r` R en faveur, le stop passe au prix d'entrée (une fois).
    #[serde(default)]
    pub be_r: Option<f64>,
    /// 🪜 Stop suiveur : le stop suit le meilleur prix à cette distance (en prix), sans jamais reculer.
    #[serde(default)]
    pub suiveur: Option<f64>,
    /// 🎯 Objectifs partiels, avant l'objectif final (`target`) ; chacun sort une fois.
    #[serde(default)]
    pub paliers: Vec<Palier>,
}

impl Gestion {
    pub fn vide(&self) -> bool {
        self.be_r.is_none() && self.suiveur.is_none() && self.paliers.is_empty()
    }
}

/// 🤖 Ce que la main a changé au plan d'une position pendant le trade (stop ou objectif glissé, ou retapé). La gestion
/// automatique (break-even, stop suiveur) et le bouton « Stop à l'entrée » n'y comptent pas : ils font partie du plan.
#[derive(Debug, Clone, Copy, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub struct Retouches {
    /// Le stop le plus loin de l'entrée posé à la main, quand il a été éloigné (le risque a grandi).
    #[serde(default)]
    pub stop_eloigne: Option<f64>,
    /// Stop rapproché de l'entrée à la main.
    #[serde(default)]
    pub stop_rapproche: bool,
    #[serde(default)]
    pub objectif_eloigne: bool,
    #[serde(default)]
    pub objectif_rapproche: bool,
}

/// 🤖 Ce que disaient les indicateurs de la chart à l'ouverture d'une position, rangés par rapport à son sens : leurs
/// noms tels qu'à l'écran, avec leur note (« RSI 14 (en surachat) »).
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub struct Lecture {
    #[serde(default)]
    pub pour: Vec<String>,
    #[serde(default)]
    pub contre: Vec<String>,
    /// Ceux qui donnent une direction, mais aucune à ce moment.
    #[serde(default)]
    pub neutres: Vec<String>,
    /// Les autres indicateurs affichés : sans direction (agitation, volume), pas lus, ou sans assez de bougies.
    #[serde(default)]
    pub autres: usize,
    /// Pourquoi rien n'est lu (« la chart montrait ETH ») ; vide quand la lecture est faite.
    #[serde(default)]
    pub pas_lue: String,
}

/// 🤖 Le plan d'un trade fermé : son stop et son objectif à l'ouverture et à la sortie, ce que la main y a changé, et
/// la lecture des indicateurs à l'ouverture. Argos AI en tire son commentaire du trade.
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub struct Plan {
    #[serde(default)]
    pub stop0: Option<f64>,
    #[serde(default)]
    pub target0: Option<f64>,
    #[serde(default)]
    pub stop: Option<f64>,
    #[serde(default)]
    pub target: Option<f64>,
    #[serde(default)]
    pub retouches: Retouches,
    #[serde(default)]
    pub lecture: Option<Lecture>,
}

/// Un ordre à ouvrir.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Order {
    pub base: String,
    pub side: Side,
    pub qty: f64,
    pub price: f64,
    pub stop: Option<f64>,
    pub target: Option<f64>,
    /// 💸 Coûts de la bourse où l'ordre part (`None` : les frais du compte).
    #[serde(default)]
    pub couts: Option<Couts>,
    /// ⚡ Perpétuel (levier, marge isolée) ; `None` : au comptant, sans marge.
    #[serde(default)]
    pub perp: Option<Perp>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ClosedTrade {
    pub base: String,
    pub side: Side,
    pub qty: f64,
    pub entry: f64,
    pub exit: f64,
    /// 💰 P&L et frais en centimes (arrondis une fois à la fermeture).
    pub pnl: Money,
    pub fees: Money,
    pub opened_at: i64,
    pub closed_at: i64,
    pub reason: String,
    /// 💰 Ce que coûtait le stop à l'ouverture (0 sans stop ; absent des anciennes sauvegardes).
    #[serde(default)]
    pub risk: Money,
    /// 🎯 Une partie seulement de la position est sortie (objectif partiel, sortie partielle) : le reste est encore
    /// ouvert, le journal attend la dernière sortie.
    #[serde(default)]
    pub partiel: bool,
    /// 💸 Part des frais (`fees`) due à l'écart achat / vente et au glissement (0 sans coûts de bourse).
    #[serde(default)]
    pub glissement: Money,
    /// ⚡ Funding payé pendant le trade (négatif : reçu) ; 0 hors perpétuel.
    #[serde(default)]
    pub funding: Money,
    /// ⚡ Levier du trade (`None` : au comptant).
    #[serde(default)]
    pub levier: Option<f64>,
    /// 🤖 Son plan (absent des anciennes sauvegardes).
    #[serde(default)]
    pub plan: Option<Plan>,
    /// 📅 The real time of a replay exit (UTC seconds), noted by the app: `closed_at` is then the market's time (absent
    /// for live exits, older saves and backtests).
    #[serde(default)]
    pub real_closed_at: Option<i64>,
}

impl ClosedTrade {
    /// 📅 When the exit really happened: the real time noted, otherwise `closed_at`.
    pub fn real_close(&self) -> i64 {
        self.real_closed_at.unwrap_or(self.closed_at)
    }

    /// ⏪ A replay exit: the app noted the real time apart (`closed_at` is the market's time then).
    pub fn replay(&self) -> bool {
        self.real_closed_at.is_some()
    }

    /// Résultat en R (P&L / risque) ; 0 sans stop.
    pub fn r(&self) -> f64 {
        if self.risk > Money::ZERO {
            self.pnl.to_f64() / self.risk.to_f64()
        } else {
            0.0
        }
    }

    /// 💸 Ce que le trade a vraiment rapporté : le P&L moins tous ses coûts (frais, écart, glissement) et le funding.
    pub fn net(&self) -> Money {
        self.pnl - self.fees - self.funding
    }

    /// 💀 Fermé par la liquidation (toute sa marge isolée perdue).
    pub fn liquide(&self) -> bool {
        self.reason == LIQUIDATION
    }
}

/// Statistiques d'un compte.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Stats {
    pub trades: usize,
    /// Taux de réussite (0 à 100).
    pub win_rate: f64,
    /// Ratio moyen : gain moyen / perte moyenne (0 sans gain ou sans perte).
    pub avg_ratio: f64,
    /// Drawdown max de la courbe d'équité (centimes, et % du plus haut).
    pub max_dd: Money,
    pub max_dd_pct: f64,
    /// Résultat depuis le départ (solde − départ).
    pub total: Money,
}

/// Points gardés au plus dans la courbe d'équité.
const CURVE_MAX: usize = 5_000;

/// 💀 Raison d'une sortie par liquidation.
pub const LIQUIDATION: &str = "liquidation";

/// ⚡ « marge insuffisante : il faut $12 000,00, il reste $8 000,00 … » (le message d'un ordre refusé).
fn manque_marge(il_faut: Money, reste: Money) -> String {
    format!(
        "marge insuffisante : il faut {}, il reste {} de disponible (baisse le risque ou monte le levier)",
        crate::nombres::format_money(il_faut.to_f64(), 2),
        crate::nombres::format_money(reste.max(Money::ZERO).to_f64(), 2)
    )
}

/// Type d'un ordre en attente : Limit (acheter sous le prix, vendre au-dessus) ou Stop (acheter au-dessus, vendre en
/// dessous).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum OrderKind {
    Limit,
    Stop,
}

/// Un ordre en attente : `order.price` est le prix de déclenchement (le stop et l'objectif y sont attachés).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Pending {
    pub id: u64,
    pub kind: OrderKind,
    pub order: Order,
    pub placed_at: i64,
    /// Valable jusqu'à cette heure (secondes UTC, « pour la journée ») ; `None` : jusqu'à annulation.
    #[serde(default)]
    pub until: Option<i64>,
    /// 🔀 OCO : l'autre ordre du couple (quand l'un part, l'autre est annulé).
    #[serde(default)]
    pub oco: Option<u64>,
    /// 🎛 Gestion donnée à la position quand l'ordre part.
    #[serde(default)]
    pub gestion: Gestion,
    /// 🔻 Reduce-only (module 31): when it fills, it only takes off the positions on the other side (never opens one).
    #[serde(default)]
    pub reduce_only: bool,
}

/// Un compte (démo ou suivi de challenge).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Portfolio {
    pub name: String,
    /// 💰 Capital de départ et solde, en centimes (additions exactes, aucune dérive).
    pub start: Money,
    pub balance: Money,
    /// Frais en fraction de la valeur (0.0005 = 0,05 %), payés à l'ouverture et à la fermeture.
    pub fee_rate: f64,
    pub positions: Vec<Position>,
    pub history: Vec<ClosedTrade>,
    next_id: u64,
    /// Ordres en attente (Limit / Stop) ; absents des anciennes sauvegardes.
    #[serde(default)]
    pub pending: Vec<Pending>,
    /// Courbe d'équité : (heure, équité) à chaque bougie jouée et à chaque trade fermé ; absente des anciennes
    /// sauvegardes.
    #[serde(default)]
    pub curve: Vec<(i64, Money)>,
    /// 💸 « Sans frais » choisi : les nouveaux ordres ne paient pas les coûts de leur bourse (seulement `fee_rate`).
    #[serde(default)]
    pub sans_frais: bool,
    /// ⚡ Funding payments (time in seconds UTC, amount paid; negative: received), the last [`FUNDING_LOG_MAX`]:
    /// « Funding du jour » of the ticket's account block.
    #[serde(default)]
    pub funding_log: Vec<(i64, Money)>,
}

/// ⚡ Funding payments kept in [`Portfolio::funding_log`] (a day of hourly payments on many positions fits).
pub const FUNDING_LOG_MAX: usize = 2_000;

/// 🔻 Reason of a trade closed by a reduce-only order.
pub const REDUCE_ONLY: &str = "reduce-only";

/// 🔻 Why a reduce-only order is refused or cancelled.
pub const NOTHING_TO_REDUCE: &str = "Reduce-only : rien à réduire (aucune position dans l'autre sens)";

/// Compte démo d'Argos par défaut : 100 000 $, sans frais (comme v1).
impl Default for Portfolio {
    fn default() -> Self {
        Self::new("Démo Argos 100K", 100_000.0, 0.0)
    }
}

impl Portfolio {
    pub fn new(name: &str, start: f64, fee_rate: f64) -> Self {
        Self {
            name: name.to_string(),
            start: Money::from_f64(start),
            balance: Money::from_f64(start),
            fee_rate,
            positions: vec![],
            history: vec![],
            next_id: 1,
            pending: vec![],
            curve: vec![],
            sans_frais: false,
            funding_log: vec![],
        }
    }

    /// ↺ Remise à zéro : même nom, même capital de départ, mêmes frais, plus aucune position, ordre ni historique.
    pub fn reset(&mut self) {
        let sans_frais = self.sans_frais;
        *self = Portfolio::new(&self.name, self.start.to_f64(), self.fee_rate);
        self.sans_frais = sans_frais;
    }

    /// 💸 Ce que paie une exécution d'une position aux coûts `couts` : taker (`true`) ou maker ; sans coûts de
    /// bourse, les frais du compte.
    fn taux(&self, couts: Option<Couts>, taker: bool) -> Taux {
        match couts {
            Some(c) => c.taux(taker),
            None => Taux {
                frais: self.fee_rate,
                glissement: 0.0,
            },
        }
    }

    /// 💸 Ce que l'ouverture de `p` a déjà payé (frais, demi-écart et glissement d'entrée), déjà retiré du solde.
    pub fn couts_entree(&self, p: &Position) -> Money {
        self.taux(p.couts, !p.maker_in).total(p.qty, p.entry)
    }

    /// 💸 ⚡ Ce qu'une position ouverte a déjà payé et que le solde a déjà perdu : ses coûts d'entrée et son funding.
    pub fn deja_paye(&self, p: &Position) -> Money {
        self.couts_entree(p) + p.funding
    }

    /// ⚡ 💰 Marge tenue : celle des positions perpétuelles ouvertes et celle des ordres qui attendent (les deux
    /// ordres d'un OCO comptent une fois, la plus grande : un seul peut partir).
    pub fn marge_utilisee(&self) -> Money {
        let m = |p: &Pending| p.order.perp.map_or(Money::ZERO, |x| x.marge(p.order.qty, p.order.price));
        let mut total: Money = self.positions.iter().map(Position::marge).sum();
        for p in &self.pending {
            match p.oco.and_then(|o| self.pending.iter().find(|q| q.id == o)) {
                Some(q) if q.id < p.id => {}
                Some(q) => total += m(p).max(m(q)),
                None => total += m(p),
            }
        }
        total
    }

    /// ⚡ 💰 Disponible pour de nouveaux ordres : le solde moins les marges tenues.
    pub fn disponible(&self) -> Money {
        self.balance - self.marge_utilisee()
    }

    fn point(&mut self, t: i64, e: Money) {
        self.curve.push((t, e));
        if self.curve.len() > CURVE_MAX {
            self.curve.drain(..CURVE_MAX / 5);
        }
    }

    /// Ouvre une position au marché (frais taker de sa bourse).
    pub fn open(&mut self, o: Order, now: i64) -> Result<u64> {
        self.ouvrir(o, now, true)
    }

    /// Ouvre une position : `taker` au marché ou par un ordre Stop, maker par un ordre Limite. Les coûts d'entrée
    /// sont payés tout de suite.
    fn ouvrir(&mut self, o: Order, now: i64, taker: bool) -> Result<u64> {
        let Order {
            base,
            side,
            qty,
            price,
            stop,
            target,
            couts,
            perp,
        } = o;
        if !(qty > 0.0 && price > 0.0) {
            return Err(ArgosError::Invalid("quantité et prix doivent être positifs".into()));
        }
        let buy = side == Side::Long;
        if let Some(s) = stop {
            if !crate::simu::sl_ok(buy, price, s) {
                return Err(ArgosError::Invalid("le stop doit être du bon côté de l'entrée".into()));
            }
        }
        if let Some(t) = target {
            if !crate::simu::tp_ok(buy, price, t) {
                return Err(ArgosError::Invalid("l'objectif doit être du bon côté de l'entrée".into()));
            }
        }
        let couts = couts.filter(|_| !self.sans_frais);
        let entree = self.taux(couts, taker).total(qty, price);
        // ⚡ marge isolée : elle et les coûts d'entrée doivent tenir dans le disponible
        if let Some(x) = perp {
            let (il_faut, reste) = (x.marge(qty, price) + entree, self.disponible());
            if il_faut > reste {
                return Err(ArgosError::Invalid(manque_marge(il_faut, reste)));
            }
        }
        self.balance -= entree;
        let id = self.next_id;
        self.next_id += 1;
        self.positions.push(Position {
            id,
            base,
            side,
            qty,
            entry: price,
            stop,
            target,
            opened_at: now,
            gestion: Gestion::default(),
            stop0: stop,
            qty0: Some(qty),
            extreme: Some(price),
            couts,
            maker_in: !taker,
            perp,
            funding: Money::ZERO,
            funding_vu: now,
            target0: target,
            retouches: Retouches::default(),
            lecture: None,
        });
        Ok(id)
    }

    /// 🎛 Pose la gestion d'une position ou d'un ordre en attente. Refusée si un objectif partiel est du mauvais côté
    /// de l'entrée, si les parts dépassent la position, ou si le break-even / le stop suiveur n'ont pas de sens.
    pub fn gerer(&mut self, id: u64, g: Gestion) -> std::result::Result<(), String> {
        let (side, entry, stop, target) = self.niveaux(id).ok_or("ordre introuvable")?;
        gestion_error(side, entry, stop, target, &g).map_or(Ok(()), |e| Err(e.to_string()))?;
        let mut g = g;
        let sens = side.sign();
        g.paliers.sort_by(|a, b| ((a.prix - b.prix) * sens).total_cmp(&0.0));
        if let Some(p) = self.positions.iter_mut().find(|p| p.id == id) {
            p.gestion = g;
        } else if let Some(p) = self.pending.iter_mut().find(|p| p.id == id) {
            p.gestion = g;
        }
        Ok(())
    }

    /// ✂ Sort `qty` d'une position au prix `price`, au marché (tout si `qty` couvre le reste) : un trade fermé pour
    /// cette partie (coûts d'ouverture au prorata), le reste reste ouvert avec son stop, son objectif et sa gestion.
    pub fn close_part(&mut self, id: u64, qty: f64, price: f64, now: i64, reason: &str) -> Result<ClosedTrade> {
        self.sortir_part(id, qty, price, now, reason, true)
    }

    /// ✂ Sortie partielle : `taker` au marché, maker pour un objectif partiel (ordre posé dans le carnet).
    fn sortir_part(&mut self, id: u64, qty: f64, price: f64, now: i64, reason: &str, taker: bool) -> Result<ClosedTrade> {
        let p = self
            .positions
            .iter_mut()
            .find(|p| p.id == id)
            .ok_or_else(|| ArgosError::Invalid("position introuvable".into()))?;
        if !(qty > 0.0) {
            return Err(ArgosError::Invalid("quantité à sortir positive".into()));
        }
        if qty >= p.qty * (1.0 - 1e-9) {
            return self.fermer(id, price, now, reason, taker);
        }
        // ⚡ le funding payé suit la quantité : la partie sortie emporte sa part
        let f = Money::produit(&[p.funding.to_f64(), qty / p.qty]);
        p.qty -= qty;
        p.funding -= f;
        let part = Position { qty, funding: f, ..p.clone() };
        Ok(self.sortie(&part, price, now, reason, taker, true))
    }

    /// Le trade fermé d'une position (ou d'une partie) sortie à `price` (`taker` : au marché ; sinon un ordre posé,
    /// maker), inscrit au solde, à l'historique et à la courbe.
    fn sortie(&mut self, p: &Position, price: f64, now: i64, reason: &str, taker: bool, partiel: bool) -> ClosedTrade {
        // chaque montant arrondi une fois au centime : coûts d'ouverture (déjà payés), de fermeture, P&L
        let (frais_in, gl_in) = self.taux(p.couts, !p.maker_in).montants(p.qty, p.entry);
        let mut pnl = p.resultat(price);
        // ⚡ 💀 marge isolée : une position liquidée (ou sortie au-delà de sa liquidation, trou de cotation) perd toute
        // sa marge et rien de plus ; la bourse la ferme elle-même, sans frais de sortie de plus
        let marge = p.marge();
        let liquidee = p.perp.is_some() && (reason == LIQUIDATION || pnl <= Money::ZERO - marge);
        let (frais_out, gl_out) = if liquidee {
            (Money::ZERO, Money::ZERO)
        } else {
            self.taux(p.couts, taker).montants(p.qty, price)
        };
        if liquidee {
            pnl = Money::ZERO - marge;
        }
        let reason = if liquidee { LIQUIDATION } else { reason };
        self.balance += pnl - frais_out - gl_out;
        let t = ClosedTrade {
            base: p.base.clone(),
            side: p.side,
            qty: p.qty,
            entry: p.entry,
            exit: price,
            pnl,
            fees: frais_in + gl_in + frais_out + gl_out,
            opened_at: p.opened_at,
            closed_at: now,
            reason: reason.to_string(),
            risk: Money::from_f64(p.risk0(p.qty)),
            partiel,
            glissement: gl_in + gl_out,
            funding: p.funding,
            levier: p.perp.map(|x| x.levier),
            plan: Some(Plan {
                stop0: p.stop0.or(p.stop),
                target0: p.target0.or(p.target),
                stop: p.stop,
                target: p.target,
                retouches: p.retouches,
                lecture: p.lecture.clone(),
            }),
            real_closed_at: None,
        };
        self.history.push(t.clone());
        let b = self.balance;
        self.point(now, b);
        t
    }

    /// Ferme une position au marché (frais taker) au prix `price`.
    pub fn close(&mut self, id: u64, price: f64, now: i64, reason: &str) -> Result<ClosedTrade> {
        self.fermer(id, price, now, reason, true)
    }

    /// Ferme une position : `taker` au marché ou par un stop touché, maker par un objectif (ordre posé).
    fn fermer(&mut self, id: u64, price: f64, now: i64, reason: &str, taker: bool) -> Result<ClosedTrade> {
        let i = self
            .positions
            .iter()
            .position(|p| p.id == id)
            .ok_or_else(|| ArgosError::Invalid("position introuvable".into()))?;
        let p = self.positions.remove(i);
        // une position déjà sortie en partie : sa dernière sortie l'est aussi (le journal la ferme à celle-ci)
        Ok(self.sortie(&p, price, now, reason, taker, false))
    }

    /// Ferme ce qui touche son stop ou son objectif pendant une bougie (stop prioritaire si les deux sont touchés),
    /// sort les objectifs partiels atteints, puis monte le stop (break-even, stop suiveur) pour la suite. En direct :
    /// `high` = `low` = dernier prix.
    pub fn check_exits(&mut self, base: &str, high: f64, low: f64, now: i64) -> Vec<ClosedTrade> {
        self.sorties(base, None, high, low, now)
    }

    /// Le cœur des sorties d'une bougie. `open` : prix d'ouverture (une position déjà ouverte avant la bougie dont un
    /// niveau est dépassé à l'ouverture sort à l'ouverture, trou de cotation) ; `None` en direct.
    fn sorties(&mut self, base: &str, open: Option<f64>, high: f64, low: f64, now: i64) -> Vec<ClosedTrade> {
        let ids: Vec<u64> = self.positions.iter().filter(|p| p.base == base).map(|p| p.id).collect();
        let mut closed = vec![];
        for id in ids {
            let Some(p) = self.positions.iter().find(|p| p.id == id).cloned() else {
                continue;
            };
            let s = p.side.sign();
            // prix d'ouverture pour une position d'avant la bougie
            let o = open.filter(|_| p.opened_at < now);
            // le pire prix et le meilleur de la bougie pour cette position
            let (pire, meilleur) = if s > 0.0 { (low, high) } else { (high, low) };
            let atteint = |niveau: f64, favorable: bool| {
                if favorable {
                    (meilleur - niveau) * s >= 0.0
                } else {
                    (pire - niveau) * s <= 0.0
                }
            };
            // prix de sortie : l'ouverture si elle a déjà dépassé le niveau, sinon le niveau
            let prix = |niveau: f64, favorable: bool| match o {
                Some(o) if favorable && (o - niveau) * s >= 0.0 => o,
                Some(o) if !favorable && (o - niveau) * s <= 0.0 => o,
                _ => niveau,
            };
            // 0. 💀 liquidation (marge isolée) : avant le stop quand il n'y a pas de stop ou qu'il est au-delà, ou quand
            // le prix où le stop partirait est déjà au-delà de la liquidation (trou de cotation)
            if let Some(liq) = p.liquidation() {
                let par_le_stop = p.stop.filter(|&st| (st - liq) * s > 0.0 && atteint(st, false)).map(|st| prix(st, false));
                let px = match par_le_stop {
                    Some(f) if (f - liq) * s <= 0.0 => Some(f),
                    Some(_) => None,
                    None if atteint(liq, false) && p.stop.is_none_or(|st| (st - liq) * s <= 0.0) => Some(prix(liq, false)),
                    None => None,
                };
                if let Some(px) = px {
                    closed.extend(self.fermer(id, px, now, LIQUIDATION, true).ok());
                    continue;
                }
            }
            // 1. stop (prioritaire : le pire cas)
            if let Some(st) = p.stop.filter(|&st| atteint(st, false)) {
                let bouge = p.stop0.is_some_and(|s0| s0 != st);
                let why = if bouge && st == p.entry {
                    "break-even"
                } else if bouge && p.gestion.suiveur.is_some() {
                    "stop suiveur"
                } else {
                    "stop"
                };
                // un stop touché part au marché (taker)
                closed.extend(self.fermer(id, prix(st, false), now, why, true).ok());
                continue;
            }
            // 2. objectifs partiels atteints, dans l'ordre
            let mut reste = p.gestion.paliers.clone();
            while let Some(pal) = reste.first().copied().filter(|pal| atteint(pal.prix, true)) {
                reste.remove(0);
                let q = (pal.part * p.qty_depart()).min(self.positions.iter().find(|x| x.id == id).map_or(0.0, |x| x.qty));
                if q > 0.0 {
                    closed.extend(self.sortir_part(id, q, prix(pal.prix, true), now, "objectif partiel", false).ok());
                }
                if let Some(x) = self.positions.iter_mut().find(|x| x.id == id) {
                    x.gestion.paliers = reste.clone();
                } else {
                    break;
                }
            }
            if !self.positions.iter().any(|x| x.id == id) {
                continue;
            }
            // 3. objectif final
            if let Some(t) = p.target.filter(|&t| atteint(t, true)) {
                // un objectif attend dans le carnet (maker)
                closed.extend(self.fermer(id, prix(t, true), now, "objectif", false).ok());
                continue;
            }
            // 4. pour la suite : break-even, stop suiveur (jamais en reculant). Une position ouverte pendant cette
            // bougie attend la suivante : son plus haut a pu venir avant l'entrée.
            if open.is_some() && p.opened_at >= now {
                continue;
            }
            if let Some(x) = self.positions.iter_mut().find(|x| x.id == id) {
                suivre(x, meilleur);
            }
        }
        closed
    }

    /// Place un ordre Limit ou Stop au prix `o.price`, le prix actuel étant `last`. Refusé s'il est du mauvais côté :
    /// Buy Limit et Sell Stop sous le prix, Sell Limit et Buy Stop au-dessus.
    pub fn place(&mut self, kind: OrderKind, o: Order, last: f64, until: Option<i64>, now: i64) -> Result<u64> {
        self.placer(kind, o, last, until, now, true)
    }

    /// Place un ordre en attente ; `marge` : vérifier que sa marge (perpétuel) tient dans le disponible (un OCO la
    /// vérifie lui-même, une fois pour les deux).
    fn placer(&mut self, kind: OrderKind, o: Order, last: f64, until: Option<i64>, now: i64, marge: bool) -> Result<u64> {
        let buy = o.side == Side::Long;
        if !crate::simu::attente_ok(kind, buy, o.price, last) {
            return Err(ArgosError::Invalid(crate::simu::Refus::AttenteMalPlacee(kind).message(buy)));
        }
        // mêmes vérifications qu'à l'ouverture (quantité, côté du stop et de l'objectif), au prix de l'ordre
        let mut probe = Portfolio::new("", 0.0, 0.0);
        probe.open(Order { perp: None, ..o.clone() }, now)?;
        // ⚡ la marge d'un ordre perpétuel est tenue pendant qu'il attend
        if let Some(x) = o.perp.filter(|_| marge) {
            let (il_faut, reste) = (x.marge(o.qty, o.price), self.disponible());
            if il_faut > reste {
                return Err(ArgosError::Invalid(manque_marge(il_faut, reste)));
            }
        }
        let id = self.next_id;
        self.next_id += 1;
        self.pending.push(Pending {
            id,
            kind,
            order: o,
            placed_at: now,
            until,
            oco: None,
            gestion: Gestion::default(),
            reduce_only: false,
        });
        Ok(id)
    }

    /// 🔀 OCO : deux ordres en attente liés (souvent un Buy Stop au-dessus et un Sell Stop dessous, pour une cassure) ;
    /// quand l'un part, l'autre est annulé. Les deux sont vérifiés avant d'être posés (aucun posé si l'un est refusé).
    pub fn place_oco(&mut self, a: (OrderKind, Order), b: (OrderKind, Order), last: f64, until: Option<i64>, now: i64) -> Result<(u64, u64)> {
        let mut probe = Portfolio::new("", 0.0, 0.0);
        probe.placer(a.0, a.1.clone(), last, until, now, false)?;
        probe.placer(b.0, b.1.clone(), last, until, now, false)?;
        // ⚡ un seul des deux peut partir : la plus grande des deux marges doit tenir dans le disponible
        let m = |o: &Order| o.perp.map_or(Money::ZERO, |x| x.marge(o.qty, o.price));
        let (il_faut, reste) = (m(&a.1).max(m(&b.1)), self.disponible());
        if il_faut > reste {
            return Err(ArgosError::Invalid(manque_marge(il_faut, reste)));
        }
        let ia = self.placer(a.0, a.1, last, until, now, false)?;
        let ib = self.placer(b.0, b.1, last, until, now, false)?;
        for (x, y) in [(ia, ib), (ib, ia)] {
            if let Some(p) = self.pending.iter_mut().find(|p| p.id == x) {
                p.oco = Some(y);
            }
        }
        Ok((ia, ib))
    }

    /// 🔻 Quantity a reduce-only order on `base` can still take off: the open positions on the other side of `buy`.
    pub fn reducible(&self, base: &str, buy: bool) -> f64 {
        let side = if buy { Side::Short } else { Side::Long };
        self.positions.iter().filter(|p| p.base == base && p.side == side).map(|p| p.qty).sum()
    }

    /// 🔻 Reduce-only at market: takes `qty` off the positions of `base` on the other side of `buy` (oldest first) at
    /// `price`, as a taker. Returns the closed parts; refused when there is nothing to reduce.
    pub fn reduce_now(&mut self, base: &str, buy: bool, qty: f64, price: f64, now: i64) -> Result<Vec<ClosedTrade>> {
        if self.reducible(base, buy) <= 0.0 {
            return Err(ArgosError::Invalid(NOTHING_TO_REDUCE.into()));
        }
        if !(qty > 0.0 && price > 0.0) {
            return Err(ArgosError::Invalid("quantité et prix doivent être positifs".into()));
        }
        Ok(self.reduce(base, buy, qty, price, now, true))
    }

    /// 🔻 Places a reduce-only Limit or Stop on `base`: when it fills it only reduces (like [`Portfolio::reduce_now`],
    /// maker for a Limit); it holds no margin and is cancelled once there is nothing left to reduce.
    #[allow(clippy::too_many_arguments)]
    pub fn place_reduce(&mut self, kind: OrderKind, base: &str, buy: bool, qty: f64, price: f64, last: f64, now: i64) -> Result<u64> {
        if self.reducible(base, buy) <= 0.0 {
            return Err(ArgosError::Invalid(NOTHING_TO_REDUCE.into()));
        }
        let o = Order {
            base: base.into(),
            side: if buy { Side::Long } else { Side::Short },
            qty,
            price,
            stop: None,
            target: None,
            couts: None,
            perp: None,
        };
        let id = self.placer(kind, o, last, None, now, false)?;
        if let Some(p) = self.pending.iter_mut().find(|p| p.id == id) {
            p.reduce_only = true;
        }
        Ok(id)
    }

    /// 🔻 Takes up to `qty` off the positions of `base` on the other side of `buy`, oldest first, at `price` (`taker`
    /// at market or by a Stop, maker by a resting Limit).
    fn reduce(&mut self, base: &str, buy: bool, qty: f64, price: f64, now: i64, taker: bool) -> Vec<ClosedTrade> {
        let side = if buy { Side::Short } else { Side::Long };
        let targets: Vec<(u64, f64)> = self
            .positions
            .iter()
            .filter(|p| p.base == base && p.side == side)
            .map(|p| (p.id, p.qty))
            .collect();
        let mut left = qty;
        let mut out = vec![];
        for (id, q) in targets {
            if left <= qty * 1e-9 {
                break;
            }
            let take = left.min(q);
            if let Ok(c) = self.sortir_part(id, take, price, now, REDUCE_ONLY, taker) {
                left -= take;
                out.push(c);
            }
        }
        out
    }

    /// « Tout fermer » : ferme chaque position au prix que donne `price` (une position sans prix reste ouverte : on
    /// ne ferme pas sur un prix inconnu) et annule tous les ordres en attente. Rend les trades fermés et le nombre
    /// d'ordres annulés.
    pub fn close_all(&mut self, price: impl Fn(&str) -> Option<f64>, now: i64, reason: &str) -> (Vec<ClosedTrade>, usize) {
        let priced: Vec<(u64, f64)> = self.positions.iter().filter_map(|p| price(&p.base).map(|px| (p.id, px))).collect();
        let closed = priced.into_iter().filter_map(|(id, px)| self.close(id, px, now, reason).ok()).collect();
        let cancelled = std::mem::take(&mut self.pending).len();
        (closed, cancelled)
    }

    /// 💾 Met de côté les positions et les ordres en attente (séance de replay gardée) : ils sortent du compte sans
    /// être fermés ni annulés (solde, historique et courbe ne bougent pas).
    pub fn mettre_de_cote(&mut self) -> (Vec<Position>, Vec<Pending>) {
        (std::mem::take(&mut self.positions), std::mem::take(&mut self.pending))
    }

    /// ▶ Remet des positions et des ordres mis de côté, avec de nouveaux identifiants (jamais deux fois le même, même
    /// si le compte a servi entre-temps) ; l'heure d'ouverture reste celle d'origine.
    pub fn remettre(&mut self, positions: Vec<Position>, pending: Vec<Pending>) {
        for mut p in positions {
            p.id = self.next_id;
            self.next_id += 1;
            self.positions.push(p);
        }
        for mut p in pending {
            p.id = self.next_id;
            self.next_id += 1;
            self.pending.push(p);
        }
    }

    /// ⚡ Paie le funding de chaque position perpétuelle jusqu'à `now` : chaque échéance depuis la dernière payée, au
    /// taux `taux_8h(symbole)` (donné pour 8 heures, ramené à l'intervalle de la position) et au prix `prix(symbole)`.
    /// Une position sans prix ou sans taux attend. Rend ce qui a été payé (négatif : reçu), symbole par symbole.
    pub fn payer_funding(&mut self, now: i64, taux_8h: impl Fn(&str) -> Option<f64>, prix: impl Fn(&str) -> Option<f64>) -> Vec<(String, Money)> {
        let mut payes: Vec<(String, Money)> = vec![];
        let mut total = Money::ZERO;
        let mut log = vec![];
        for p in self.positions.iter_mut() {
            let Some(x) = p.perp else { continue };
            let (Some(r), Some(px)) = (taux_8h(&p.base), prix(&p.base)) else { continue };
            let ech = perp::echeances(p.funding_vu.max(p.opened_at), now, x.funding_h);
            let n = ech.len();
            p.funding_vu = p.funding_vu.max(now);
            if n == 0 {
                continue;
            }
            let un = perp::funding(p.side == Side::Long, p.qty, px, x.taux_par_paiement(r));
            // ⚡ each payment at its own time (a catch-up after the app was closed keeps its days)
            log.extend(ech.into_iter().map(|t| (t, un)));
            let m = Money::from_cents(un.cents() * n as i64);
            p.funding += m;
            total += m;
            match payes.iter_mut().find(|(b, _)| *b == p.base) {
                Some((_, t)) => *t += m,
                None => payes.push((p.base.clone(), m)),
            }
        }
        if !payes.is_empty() {
            self.balance -= total;
            let b = self.balance;
            self.point(now, b);
            self.funding_log.extend(log);
            if self.funding_log.len() > FUNDING_LOG_MAX {
                self.funding_log.drain(..FUNDING_LOG_MAX / 5);
            }
        }
        payes
    }

    /// ⚡ Funding paid at the times `keep` accepts (« du jour »: the trading day), negative when received.
    pub fn funding_paid(&self, keep: impl Fn(i64) -> bool) -> Money {
        self.funding_log.iter().filter(|(t, _)| keep(*t)).map(|(_, m)| *m).sum()
    }

    /// Annule un ordre en attente ; `false` s'il n'existe plus.
    pub fn cancel(&mut self, id: u64) -> bool {
        let n = self.pending.len();
        self.pending.retain(|p| p.id != id);
        // 🔀 l'autre ordre d'un OCO reste seul : il n'est plus lié
        for p in self.pending.iter_mut().filter(|p| p.oco == Some(id)) {
            p.oco = None;
        }
        self.pending.len() != n
    }

    /// Déplace un ordre en attente (ligne glissée) : le stop et l'objectif suivent du même écart.
    pub fn move_pending(&mut self, id: u64, price: f64) {
        if let Some(p) = self.pending.iter_mut().find(|p| p.id == id) {
            let d = price - p.order.price;
            p.order.price = price;
            p.order.stop = p.order.stop.map(|s| s + d);
            p.order.target = p.order.target.map(|t| t + d);
        }
    }

    /// Ordres en attente de `base` atteints par la bougie (`high`, `low`) : ils deviennent des positions au prix de
    /// l'ordre (les expirés sont annulés). Les ordres que `allow` refuse (gardien) sont annulés, avec leur raison.
    /// 🔻 A reduce-only order reduces instead (its trades in `closed`), or is cancelled when nothing is left to reduce.
    pub fn check_pending(&mut self, base: &str, high: f64, low: f64, now: i64, mut allow: impl FnMut(&Order) -> Option<String>) -> PendingOutcome {
        let mut out = PendingOutcome::default();
        // 🔻 a reduce-only order whose position is gone has nothing left to do: cancelled
        let vides: Vec<u64> = self
            .pending
            .iter()
            .filter(|p| p.reduce_only && p.order.base == base && self.reducible(base, p.order.side == Side::Long) <= 0.0)
            .map(|p| p.id)
            .collect();
        self.pending.retain(|p| !vides.contains(&p.id));
        out.nothing_to_reduce = vides;
        let (hit, keep): (Vec<Pending>, Vec<Pending>) = std::mem::take(&mut self.pending).into_iter().partition(|p| {
            p.order.base == base
                && match (p.kind, p.order.side) {
                    (OrderKind::Limit, Side::Long) | (OrderKind::Stop, Side::Short) => low <= p.order.price,
                    (OrderKind::Limit, Side::Short) | (OrderKind::Stop, Side::Long) => high >= p.order.price,
                }
        });
        self.pending = keep;
        let expired: Vec<u64> = self.pending.iter().filter(|p| p.until.is_some_and(|u| now > u)).map(|p| p.id).collect();
        self.pending.retain(|p| !expired.contains(&p.id));
        out.expired = expired;
        let mut annules: Vec<u64> = vec![];
        for p in hit {
            // 🔀 l'autre ordre de son OCO est déjà parti dans cette bougie : celui-ci est annulé
            if annules.contains(&p.id) {
                out.oco.push(p.id);
                continue;
            }
            // 🔻 reduce-only: it takes off the other side (a Limit waited in the book: maker), never opens; the Gardien
            // does not stop an order that lowers the risk
            if p.reduce_only {
                let buy = p.order.side == Side::Long;
                let closed = self.reduce(base, buy, p.order.qty, p.order.price, now, p.kind == OrderKind::Stop);
                if closed.is_empty() {
                    out.nothing_to_reduce.push(p.id);
                } else {
                    out.reduced.push(p.id);
                    out.closed.extend(closed);
                }
                continue;
            }
            if let Some(why) = allow(&p.order) {
                out.refused.push((p.id, why));
                continue;
            }
            // 🔀 ⚡ l'autre ordre de son OCO est mis de côté pendant l'ouverture : un seul des deux part, sa marge ne
            // compte pas en plus (remis en place si l'ouverture est refusée)
            let autre = p
                .oco
                .and_then(|o| self.pending.iter().position(|q| q.id == o))
                .map(|i| (i, self.pending.remove(i)));
            // 💸 un ordre Limite attendait dans le carnet (maker) ; un Stop part au marché (taker)
            match self.ouvrir(p.order.clone(), now, p.kind == OrderKind::Stop) {
                Ok(id) => {
                    out.filled.push((p.id, id));
                    if let Some(x) = self.positions.iter_mut().find(|x| x.id == id) {
                        x.gestion = p.gestion.clone();
                    }
                    if let Some(o) = p.oco {
                        annules.push(o);
                        if let Some((_, a)) = autre {
                            out.oco.push(a.id);
                        }
                    }
                }
                Err(e) => {
                    if let Some((i, a)) = autre {
                        self.pending.insert(i.min(self.pending.len()), a);
                    }
                    out.refused.push((p.id, e.to_string()));
                }
            }
        }
        out
    }

    /// Côté, entrée, stop et objectif d'une position ou d'un ordre en attente (même suite d'identifiants).
    pub fn niveaux(&self, id: u64) -> Option<(Side, f64, Option<f64>, Option<f64>)> {
        if let Some(p) = self.positions.iter().find(|p| p.id == id) {
            return Some((p.side, p.entry, p.stop, p.target));
        }
        self.pending
            .iter()
            .find(|p| p.id == id)
            .map(|p| (p.order.side, p.order.price, p.order.stop, p.order.target))
    }

    /// 🛡️ Déplace le stop (`stop = true`) ou l'objectif d'une position ou d'un ordre en attente. Refusé s'il passe du
    /// mauvais côté de l'entrée (achat : stop dessous, objectif dessus ; l'inverse en vente) ; le message dit pourquoi.
    pub fn set_level(&mut self, id: u64, stop: bool, price: f64) -> std::result::Result<(), String> {
        let (side, entry, _, _) = self.niveaux(id).ok_or("ordre introuvable")?;
        if let Some(why) = level_error(side, entry, stop, price) {
            return Err(why.into());
        }
        let set = |s: &mut Option<f64>, t: &mut Option<f64>| *(if stop { s } else { t }) = Some(price);
        if let Some(p) = self.positions.iter_mut().find(|p| p.id == id) {
            retoucher(p, stop, price);
            set(&mut p.stop, &mut p.target);
        } else if let Some(p) = self.pending.iter_mut().find(|p| p.id == id) {
            set(&mut p.order.stop, &mut p.order.target);
        }
        Ok(())
    }

    /// ⚖ Monte le stop d'une position à son prix d'entrée (break-even à la main). Refusé si le stop y est déjà ou au-delà.
    pub fn set_stop_entree(&mut self, id: u64) -> std::result::Result<(), String> {
        let p = self.positions.iter_mut().find(|p| p.id == id).ok_or("position introuvable")?;
        if p.stop.is_some_and(|st| (st - p.entry) * p.side.sign() >= 0.0) {
            return Err("le stop est déjà à l'entrée ou au-delà".into());
        }
        p.stop = Some(p.entry);
        p.gestion.be_r = None;
        Ok(())
    }

    /// 🕯️ Joue une bougie de `base` (heure `b.time`), sans deviner l'ordre des prix à l'intérieur :
    /// 1. un ordre en attente déjà dépassé à l'ouverture se remplit à l'ouverture (trou de cotation) ; les autres au
    ///    prix de l'ordre si la bougie l'atteint ; `allow` (gardien) peut les refuser ;
    /// 2. une position dont le stop ou l'objectif est dépassé à l'ouverture sort à l'ouverture ;
    /// 3. puis stop ou objectif touché pendant la bougie (stop prioritaire : le pire cas) ;
    /// 4. un point d'équité au prix de clôture.
    pub fn on_bar(&mut self, base: &str, b: &Bar, allow: impl FnMut(&Order) -> Option<String>) -> (PendingOutcome, Vec<ClosedTrade>) {
        for p in self.pending.iter_mut().filter(|p| p.order.base == base) {
            let jumped = match (p.kind, p.order.side) {
                (OrderKind::Limit, Side::Long) | (OrderKind::Stop, Side::Short) => b.open < p.order.price,
                (OrderKind::Limit, Side::Short) | (OrderKind::Stop, Side::Long) => b.open > p.order.price,
            };
            if jumped {
                p.order.price = b.open;
            }
        }
        let mut out = self.check_pending(base, b.high, b.low, b.time, allow);
        // 🔻 the trades closed by reduce-only orders come first, with the exits of the bar
        let mut closed = std::mem::take(&mut out.closed);
        closed.extend(self.sorties(base, Some(b.open), b.high, b.low, b.time));
        let e = self.balance + self.positions.iter().filter(|p| p.base == base).map(|p| p.resultat(b.close)).sum::<Money>();
        self.point(b.time, e);
        (out, closed)
    }

    /// 🎯 Résultat net de chaque trade (coûts déduits), ses sorties partielles comprises (une position = un trade),
    /// dans l'ordre de la première sortie.
    pub fn trades_complets(&self) -> Vec<Money> {
        let mut cles: Vec<(&str, i64)> = vec![];
        let mut pnl: Vec<Money> = vec![];
        for c in &self.history {
            let k = (c.base.as_str(), c.opened_at);
            match cles.iter().position(|x| *x == k) {
                Some(i) => pnl[i] += c.net(),
                None => {
                    cles.push(k);
                    pnl.push(c.net());
                }
            }
        }
        pnl
    }

    /// Statistiques : taux de réussite, ratio moyen (gain moyen / perte moyenne), drawdown max de la courbe.
    pub fn stats(&self) -> Stats {
        let trades = self.trades_complets();
        let n = trades.len();
        let wins: Vec<Money> = trades.iter().copied().filter(|p| *p > Money::ZERO).collect();
        let losses: Vec<Money> = trades.iter().copied().filter(|p| *p < Money::ZERO).collect();
        let avg = |v: &[Money]| {
            if v.is_empty() {
                0.0
            } else {
                v.iter().copied().sum::<Money>().to_f64() / v.len() as f64
            }
        };
        let (aw, al) = (avg(&wins), avg(&losses).abs());
        let mut peak = self.start;
        let (mut dd, mut dd_pct) = (Money::ZERO, 0.0);
        for &(_, e) in &self.curve {
            peak = peak.max(e);
            if peak - e > dd {
                dd = peak - e;
                dd_pct = if peak > Money::ZERO { dd.to_f64() / peak.to_f64() * 100.0 } else { 0.0 };
            }
        }
        Stats {
            trades: n,
            win_rate: if n == 0 { 0.0 } else { wins.len() as f64 * 100.0 / n as f64 },
            avg_ratio: if aw > 0.0 && al > 0.0 { aw / al } else { 0.0 },
            max_dd: dd,
            max_dd_pct: dd_pct,
            total: self.balance - self.start,
        }
    }

    /// Équité = solde + P&L ouvert (chaque position arrondie au centime), aux prix donnés.
    pub fn equity(&self, price_of: impl Fn(&str) -> Option<f64>) -> Money {
        self.balance
            + self
                .positions
                .iter()
                .map(|p| price_of(&p.base).map_or(Money::ZERO, |px| p.resultat(px)))
                .sum::<Money>()
    }

    /// 📒 The ticket's account block at the prices `price_of` gives (a position without a price counts for nothing).
    pub fn summary(&self, price_of: impl Fn(&str) -> Option<f64>) -> AccountSummary {
        let mut s = AccountSummary {
            margin_used: self.marge_utilisee(),
            available: self.disponible(),
            ..AccountSummary::default()
        };
        for p in &self.positions {
            let Some(px) = price_of(&p.base) else { continue };
            s.unrealized += p.resultat(px);
            let value = Money::produit(&[p.qty, px]);
            match p.side {
                Side::Long => s.long += value,
                Side::Short => s.short += value,
            }
            if let Some(x) = p.perp {
                s.maintenance += Money::produit(&[p.qty, px, x.maintenance]);
            }
            s.positions += 1;
        }
        s.equity = self.balance + s.unrealized;
        s
    }
}

/// 📒 The account block of the ticket (module 31): equity, margins and exposure at the current prices.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct AccountSummary {
    /// Fonds propres: the balance and the open P&L.
    pub equity: Money,
    /// Disponible pour trader: the balance less the margins held (isolated margin: the open P&L is not lent).
    pub available: Money,
    pub margin_used: Money,
    /// Marge de maintien: what the perpetual positions must keep (their value × the maintenance rate).
    pub maintenance: Money,
    /// P&L latent.
    pub unrealized: Money,
    /// Value of the long and of the short positions at the current prices.
    pub long: Money,
    pub short: Money,
    /// Open positions counted (those with a price).
    pub positions: usize,
}

impl AccountSummary {
    /// Ratio de marge: the maintenance margin over the equity, in % (`None` without a perpetual position, or when
    /// the equity is gone).
    pub fn margin_ratio(&self) -> Option<f64> {
        (self.maintenance > Money::ZERO && self.equity > Money::ZERO).then(|| self.maintenance.to_f64() / self.equity.to_f64() * 100.0)
    }

    /// Levier: the positions' value over the equity (0 without a position).
    pub fn leverage(&self) -> f64 {
        if self.equity > Money::ZERO {
            (self.long + self.short).to_f64() / self.equity.to_f64()
        } else {
            0.0
        }
    }
}

/// 🤖 Le stop (`stop = true`) ou l'objectif de `p` passe à la main à `price` : la retouche est notée (plus loin ou
/// plus près de l'entrée qu'avant ; un stop posé là où il n'y en avait pas le rapproche).
fn retoucher(p: &mut Position, stop: bool, price: f64) {
    let s = p.side.sign();
    // écart à l'entrée, côté de la perte pour le stop, côté du gain pour l'objectif
    let ecart = |x: f64| if stop { (p.entry - x) * s } else { (x - p.entry) * s };
    let avant = if stop { p.stop } else { p.target }.map(ecart);
    let apres = ecart(price);
    let pas = 1e-9 * p.entry.abs().max(1.0);
    let r = &mut p.retouches;
    match (stop, avant) {
        (true, Some(a)) if apres > a + pas => {
            if r.stop_eloigne.is_none_or(|x| apres > ecart(x)) {
                r.stop_eloigne = Some(price);
            }
        }
        (true, Some(a)) if apres < a - pas => r.stop_rapproche = true,
        (true, None) => r.stop_rapproche = true,
        (false, Some(a)) if apres > a + pas => r.objectif_eloigne = true,
        (false, Some(a)) if apres < a - pas => r.objectif_rapproche = true,
        _ => {}
    }
}

/// 🎛 Après une bougie (ou un prix du direct) : le meilleur prix vu, puis le stop monte au break-even ou suit le
/// meilleur prix ; il ne recule jamais.
fn suivre(p: &mut Position, meilleur: f64) {
    let s = p.side.sign();
    let ext = p.extreme.map_or(meilleur, |e| if (meilleur - e) * s > 0.0 { meilleur } else { e });
    p.extreme = Some(ext);
    let mieux = |nouveau: f64, stop: Option<f64>| stop.is_none_or(|st| (nouveau - st) * s > 0.0);
    if let Some(r) = p.gestion.be_r {
        let un_r = p.r_unite();
        if un_r > 0.0 && (ext - p.entry) * s >= r * un_r {
            if mieux(p.entry, p.stop) {
                p.stop = Some(p.entry);
            }
            p.gestion.be_r = None;
        }
    }
    if let Some(d) = p.gestion.suiveur.filter(|d| *d > 0.0) {
        let nouveau = ext - d * s;
        if mieux(nouveau, p.stop) {
            p.stop = Some(nouveau);
        }
    }
}

/// 🎛 Une gestion qui n'a pas de sens pour cette position : le message dit pourquoi.
pub fn gestion_error(side: Side, entry: f64, stop: Option<f64>, target: Option<f64>, g: &Gestion) -> Option<&'static str> {
    let s = side.sign();
    if g.be_r.is_some_and(|r| !(r.is_finite() && r > 0.0)) {
        return Some("Le break-even se règle en R positifs.");
    }
    if g.be_r.is_some() && stop.is_none() {
        return Some("Le break-even demande un Stop Loss.");
    }
    if g.suiveur.is_some_and(|d| !(d.is_finite() && d > 0.0)) {
        return Some("La distance du stop suiveur doit être positive.");
    }
    let mut total = 0.0;
    for p in &g.paliers {
        if !(p.prix.is_finite() && p.prix > 0.0 && p.part > 0.0 && p.part <= 1.0) {
            return Some("Objectif partiel invalide.");
        }
        if (p.prix - entry) * s <= 0.0 {
            return Some("Un objectif partiel doit être du côté du gain.");
        }
        if target.is_some_and(|t| (t - p.prix) * s <= 0.0) {
            return Some("Un objectif partiel doit être avant l'objectif final.");
        }
        total += p.part;
    }
    if total > 1.0 + 1e-9 {
        return Some("Les objectifs partiels dépassent la position.");
    }
    None
}

/// Le stop (`stop = true`) ou l'objectif à `price` est-il du mauvais côté de l'entrée ? Le message, comme TradingView.
pub fn level_error(side: Side, entry: f64, stop: bool, price: f64) -> Option<&'static str> {
    if !(price.is_finite() && price > 0.0) {
        return Some("Prix invalide.");
    }
    let au_dessus = price > entry;
    match (side, stop) {
        (Side::Long, true) if price >= entry => Some("Le Stop Loss doit être inférieur au prix d'entrée."),
        (Side::Short, true) if price <= entry => Some("Le Stop Loss doit être supérieur au prix d'entrée."),
        (Side::Long, false) if !au_dessus => Some("Le Take Profit doit être supérieur au prix d'entrée."),
        (Side::Short, false) if au_dessus || price == entry => Some("Le Take Profit doit être inférieur au prix d'entrée."),
        _ => None,
    }
}

/// Ce que la bougie a fait des ordres en attente.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PendingOutcome {
    /// (ordre, position ouverte).
    pub filled: Vec<(u64, u64)>,
    /// (ordre, raison) : refusés à l'exécution (gardien, ordre invalide).
    pub refused: Vec<(u64, String)>,
    /// Expirés (fin de journée).
    pub expired: Vec<u64>,
    /// 🔀 Annulés parce que l'autre ordre de leur OCO est parti.
    pub oco: Vec<u64>,
    /// 🔻 Reduce-only orders that filled, and the trades they closed ([`Portfolio::on_bar`] hands these trades back
    /// with the bar's exits, and leaves `closed` empty).
    pub reduced: Vec<u64>,
    pub closed: Vec<ClosedTrade>,
    /// 🔻 Reduce-only orders cancelled: nothing left to reduce.
    pub nothing_to_reduce: Vec<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn o(base: &str, side: Side, qty: f64, price: f64, stop: Option<f64>, target: Option<f64>) -> Order {
        Order {
            base: base.into(),
            side,
            qty,
            price,
            stop,
            target,
            couts: None,
            perp: None,
        }
    }

    #[test]
    fn long_puis_short_avec_frais() {
        let mut pf = Portfolio::new("Démo", 100_000.0, 0.0);
        let id = pf.open(o("BTC", Side::Long, 1.0, 80_000.0, Some(79_000.0), Some(82_000.0)), 0).unwrap();
        assert_eq!(pf.positions[0].risk(), 1_000.0);
        assert_eq!(pf.equity(|_| Some(81_000.0)), Money::from_f64(101_000.0));
        let t = pf.close(id, 81_000.0, 10, "manuel").unwrap();
        assert_eq!(t.pnl, Money::from_f64(1_000.0));
        assert_eq!(pf.balance, Money::from_f64(101_000.0));
        let mut pf2 = Portfolio::new("Démo", 10_000.0, 0.001);
        let id = pf2.open(o("ETH", Side::Short, 2.0, 2_000.0, None, None), 0).unwrap();
        pf2.close(id, 1_900.0, 1, "manuel").unwrap();
        assert_eq!(pf2.balance, Money::from_f64(10_000.0 + 200.0 - 4.0 - 3.8), "au centime près, exactement");
    }

    /// « Tout fermer » : chaque position avec un prix ferme, tous les ordres en attente sont annulés, le solde est la
    /// somme exacte des P&L ; une position sans prix reste ouverte.
    #[test]
    fn tout_fermer_positions_et_ordres() {
        let mut pf = Portfolio::new("Démo", 10_000.0, 0.0);
        pf.open(o("BTC", Side::Long, 1.0, 100.0, Some(95.0), None), 0).unwrap();
        pf.open(o("ETH", Side::Short, 2.0, 50.0, Some(55.0), None), 0).unwrap();
        pf.open(o("BTC", Side::Short, 0.5, 104.0, None, None), 0).unwrap();
        pf.place(OrderKind::Limit, o("BTC", Side::Long, 1.0, 90.0, Some(85.0), None), 100.0, None, 0)
            .unwrap();
        pf.place(OrderKind::Stop, o("ETH", Side::Short, 1.0, 45.0, Some(48.0), None), 50.0, None, 0)
            .unwrap();
        let px = |b: &str| match b {
            "BTC" => Some(110.0),
            "ETH" => Some(45.5),
            _ => None,
        };
        let (closed, cancelled) = pf.close_all(px, 9, "tout fermer");
        assert_eq!((closed.len(), cancelled), (3, 2));
        assert!(pf.positions.is_empty() && pf.pending.is_empty());
        let total: Money = closed.iter().map(|c| c.pnl).sum();
        assert_eq!(total, Money::from_f64(10.0 + 9.0 - 3.0));
        assert_eq!(pf.balance, Money::from_f64(10_000.0) + total, "solde = somme exacte");
        assert!(closed.iter().all(|c| c.reason == "tout fermer" && c.closed_at == 9));
        // sans prix, la position reste ; il n'y a rien à annuler
        pf.open(o("XAU", Side::Long, 1.0, 2_000.0, None, None), 10).unwrap();
        assert_eq!(pf.close_all(px, 11, "tout fermer").0.len(), 0);
        assert_eq!(pf.positions.len(), 1);
    }

    #[test]
    fn stop_et_objectif() {
        let mut pf = Portfolio::new("Démo", 1_000.0, 0.0);
        pf.open(o("BTC", Side::Long, 1.0, 100.0, Some(95.0), Some(110.0)), 0).unwrap();
        pf.open(o("BTC", Side::Short, 1.0, 100.0, Some(105.0), Some(90.0)), 0).unwrap();
        let out = pf.check_exits("BTC", 106.0, 99.0, 5);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].reason, "stop");
        assert_eq!(out[0].pnl, Money::from_f64(-5.0));
        assert!(pf.open(o("BTC", Side::Long, 1.0, 100.0, Some(101.0), None), 0).is_err());
    }

    #[test]
    fn limit_et_stop_executes_au_bon_prix() {
        let mut pf = Portfolio::new("Démo", 10_000.0, 0.0);
        let lim = pf
            .place(OrderKind::Limit, o("BTC", Side::Long, 1.0, 95.0, Some(90.0), Some(105.0)), 100.0, None, 0)
            .unwrap();
        let stp = pf
            .place(OrderKind::Stop, o("BTC", Side::Short, 1.0, 92.0, Some(97.0), None), 100.0, None, 0)
            .unwrap();
        assert!(pf.check_pending("BTC", 101.0, 96.0, 1, |_| None).filled.is_empty(), "pas encore atteint");
        assert_eq!(pf.pending.len(), 2);
        let r = pf.check_pending("BTC", 97.0, 94.5, 2, |_| None);
        assert_eq!(r.filled.len(), 1, "le Buy Limit à 95 seulement");
        assert_eq!(r.filled[0].0, lim);
        let p = &pf.positions[0];
        assert_eq!(
            (p.entry, p.stop, p.target),
            (95.0, Some(90.0), Some(105.0)),
            "SL et TP attachés, au prix de l'ordre"
        );
        let r = pf.check_pending("BTC", 94.0, 91.0, 3, |_| None);
        assert_eq!(r.filled[0].0, stp, "le Sell Stop à 92 se déclenche quand le prix descend");
        assert!(pf.pending.is_empty());
    }

    #[test]
    fn ordre_du_mauvais_cote_annulation_et_validite() {
        let mut pf = Portfolio::new("Démo", 10_000.0, 0.0);
        let e = pf
            .place(OrderKind::Limit, o("BTC", Side::Long, 1.0, 105.0, None, None), 100.0, None, 0)
            .unwrap_err();
        assert!(e.to_string().contains("Buy Limit se place sous"), "{e}");
        assert!(
            pf.place(OrderKind::Stop, o("BTC", Side::Long, 1.0, 95.0, None, None), 100.0, None, 0).is_err(),
            "Buy Stop sous le prix"
        );
        assert!(
            pf.place(OrderKind::Limit, o("BTC", Side::Long, 1.0, 95.0, Some(96.0), None), 100.0, None, 0)
                .is_err(),
            "stop du mauvais côté"
        );
        let id = pf
            .place(OrderKind::Limit, o("BTC", Side::Short, 1.0, 110.0, None, None), 100.0, None, 0)
            .unwrap();
        assert!(pf.cancel(id));
        assert!(!pf.cancel(id), "déjà annulé");
        // pour la journée : expiré après `until`
        pf.place(OrderKind::Limit, o("BTC", Side::Short, 1.0, 110.0, None, None), 100.0, Some(50), 0)
            .unwrap();
        let r = pf.check_pending("BTC", 101.0, 99.0, 60, |_| None);
        assert_eq!(r.expired.len(), 1);
        assert!(pf.pending.is_empty());
        // glisser : le stop et l'objectif suivent
        let id = pf
            .place(OrderKind::Limit, o("BTC", Side::Long, 1.0, 95.0, Some(90.0), Some(105.0)), 100.0, None, 0)
            .unwrap();
        pf.move_pending(id, 93.0);
        assert_eq!(
            (pf.pending[0].order.price, pf.pending[0].order.stop, pf.pending[0].order.target),
            (93.0, Some(88.0), Some(103.0))
        );
    }

    #[test]
    fn gardien_refuse_a_l_execution_et_vieille_sauvegarde() {
        let mut pf = Portfolio::new("Démo", 10_000.0, 0.0);
        pf.place(OrderKind::Limit, o("BTC", Side::Long, 1.0, 95.0, None, None), 100.0, None, 0).unwrap();
        let r = pf.check_pending("BTC", 100.0, 94.0, 1, |_| Some("stop loss obligatoire".into()));
        assert_eq!(r.refused.len(), 1);
        assert!(pf.positions.is_empty() && pf.pending.is_empty(), "refusé : annulé, pas de position");
        // une sauvegarde d'avant les ordres en attente se charge encore
        let old = r#"{"name":"Démo Argos 100K","start":100000.0,"balance":100000.0,"fee_rate":0.0,"positions":[],"history":[],"next_id":3}"#;
        let pf: Portfolio = serde_json::from_str(old).unwrap();
        assert!(pf.pending.is_empty());
        assert_eq!(pf.balance, Money::from_f64(100_000.0));
    }

    #[test]
    fn mille_trades_sans_derive_du_solde() {
        // 1000 allers-retours de 0,1 unité à +1 $ puis −1 $ : solde exact au centime à la fin
        let mut pf = Portfolio::new("Démo", 100_000.0, 0.0);
        for k in 0..1000 {
            let id = pf.open(o("X", Side::Long, 0.1, 100.0, None, None), k).unwrap();
            let exit = if k % 2 == 0 { 110.0 } else { 90.0 };
            pf.close(id, exit, k, "manuel").unwrap();
        }
        assert_eq!(pf.balance, Money::from_f64(100_000.0), "+1 $ et −1 $ 500 fois chacun : exactement le départ");
        assert_eq!(pf.history.iter().map(|t| t.pnl).sum::<Money>(), Money::ZERO);
    }

    /// 🛡️ SL / TP déplacés sur la chart : du bon côté seulement (message comme TradingView), position ou ordre.
    #[test]
    fn deplacer_sl_et_tp() {
        let mut pf = Portfolio::default();
        let id = pf.open(o("BTC", Side::Long, 1.0, 100.0, Some(90.0), Some(120.0)), 0).unwrap();
        assert!(pf.set_level(id, true, 95.0).is_ok());
        assert_eq!(pf.positions[0].stop, Some(95.0));
        assert_eq!(pf.set_level(id, true, 101.0).unwrap_err(), "Le Stop Loss doit être inférieur au prix d'entrée.");
        assert_eq!(
            pf.set_level(id, false, 99.0).unwrap_err(),
            "Le Take Profit doit être supérieur au prix d'entrée."
        );
        assert_eq!(pf.positions[0].stop, Some(95.0), "refusé : rien ne bouge");
        let v = pf
            .place(OrderKind::Limit, o("BTC", Side::Short, 1.0, 110.0, Some(115.0), Some(100.0)), 100.0, None, 0)
            .unwrap();
        assert!(pf.set_level(v, false, 95.0).is_ok());
        assert_eq!(pf.pending[0].order.target, Some(95.0));
        assert!(pf.set_level(v, true, 105.0).unwrap_err().contains("supérieur"));
        assert_eq!(pf.niveaux(v).map(|n| n.1), Some(110.0));
        assert!(pf.set_level(999, true, 1.0).is_err());
    }

    /// Une bougie de 30 min : ouverture, plus haut, plus bas, clôture.
    fn bougie(t: i64, o: f64, h: f64, l: f64, c: f64) -> Bar {
        Bar {
            time: t,
            open: o,
            high: h,
            low: l,
            close: c,
            volume: 0.0,
        }
    }

    fn joue(pf: &mut Portfolio, b: Bar) -> Vec<ClosedTrade> {
        pf.on_bar("BTC", &b, |_| None).1
    }

    #[test]
    fn break_even_puis_sortie_a_l_entree() {
        let mut pf = Portfolio::new("Démo", 10_000.0, 0.0);
        let id = pf.open(o("BTC", Side::Long, 10.0, 100.0, Some(90.0), Some(150.0)), 0).unwrap();
        pf.gerer(
            id,
            Gestion {
                be_r: Some(1.0),
                ..Default::default()
            },
        )
        .unwrap();
        // pas encore 1 R : le stop ne bouge pas
        assert!(joue(&mut pf, bougie(1800, 100.0, 109.0, 99.0, 105.0)).is_empty());
        assert_eq!(pf.positions[0].stop, Some(90.0));
        // 1 R atteint : le stop passe à l'entrée pour la bougie suivante
        assert!(joue(&mut pf, bougie(3600, 105.0, 110.0, 104.0, 108.0)).is_empty());
        assert_eq!((pf.positions[0].stop, pf.positions[0].gestion.be_r), (Some(100.0), None));
        let c = joue(&mut pf, bougie(5400, 108.0, 108.0, 99.0, 99.0));
        assert_eq!((c[0].reason.as_str(), c[0].exit, c[0].pnl), ("break-even", 100.0, Money::ZERO));
        assert_eq!((c[0].risk, c[0].r()), (Money::from_cents(10_000), 0.0), "le R reste celui du départ");
    }

    #[test]
    fn stop_suiveur_ne_recule_jamais() {
        let mut pf = Portfolio::new("Démo", 10_000.0, 0.0);
        let id = pf.open(o("BTC", Side::Long, 1.0, 100.0, Some(90.0), None), 0).unwrap();
        pf.gerer(
            id,
            Gestion {
                suiveur: Some(5.0),
                ..Default::default()
            },
        )
        .unwrap();
        joue(&mut pf, bougie(1800, 100.0, 108.0, 100.0, 107.0));
        assert_eq!(pf.positions[0].stop, Some(103.0));
        joue(&mut pf, bougie(3600, 107.0, 112.0, 106.0, 110.0));
        assert_eq!(pf.positions[0].stop, Some(107.0));
        joue(&mut pf, bougie(5400, 110.0, 111.0, 108.0, 109.0));
        assert_eq!(pf.positions[0].stop, Some(107.0), "plus haut plus bas : le stop reste");
        let c = joue(&mut pf, bougie(7200, 109.0, 109.0, 104.0, 105.0));
        assert_eq!((c[0].reason.as_str(), c[0].exit), ("stop suiveur", 107.0));
        assert_eq!(c[0].r(), 0.7);
        // à la vente : le stop descend avec le plus bas
        let id = pf.open(o("BTC", Side::Short, 1.0, 100.0, Some(110.0), None), 8000).unwrap();
        pf.gerer(
            id,
            Gestion {
                suiveur: Some(4.0),
                ..Default::default()
            },
        )
        .unwrap();
        joue(&mut pf, bougie(9000, 100.0, 101.0, 92.0, 93.0));
        assert_eq!(pf.positions[0].stop, Some(96.0));
    }

    #[test]
    fn objectifs_partiels_puis_final() {
        let mut pf = Portfolio::new("Démo", 10_000.0, 0.0);
        let id = pf.open(o("BTC", Side::Long, 10.0, 100.0, Some(90.0), Some(130.0)), 0).unwrap();
        let paliers = vec![Palier { prix: 120.0, part: 0.25 }, Palier { prix: 110.0, part: 0.5 }];
        pf.gerer(id, Gestion { paliers, ..Default::default() }).unwrap();
        assert_eq!(pf.positions[0].gestion.paliers[0].prix, 110.0, "rangés du plus proche au plus loin");
        let c = joue(&mut pf, bougie(1800, 100.0, 115.0, 99.0, 112.0));
        assert_eq!((c.len(), c[0].qty, c[0].exit, c[0].partiel), (1, 5.0, 110.0, true));
        assert_eq!((c[0].reason.as_str(), c[0].r()), ("objectif partiel", 1.0));
        assert_eq!(pf.positions[0].qty, 5.0);
        let c = joue(&mut pf, bougie(3600, 112.0, 125.0, 111.0, 124.0));
        assert_eq!((c[0].qty, c[0].exit), (2.5, 120.0));
        let c = joue(&mut pf, bougie(5400, 124.0, 131.0, 123.0, 130.0));
        assert_eq!((c[0].qty, c[0].reason.as_str(), c[0].partiel), (2.5, "objectif", false));
        assert!(pf.positions.is_empty());
        // 5 × 10 + 2,5 × 20 + 2,5 × 30, au centime ; un seul trade dans les statistiques
        assert_eq!(pf.balance, Money::from_f64(10_175.0));
        assert_eq!((pf.stats().trades, pf.stats().win_rate), (1, 100.0));
    }

    #[test]
    fn stop_avant_objectif_partiel_et_trou_a_l_ouverture() {
        let mut pf = Portfolio::new("Démo", 10_000.0, 0.0);
        let id = pf.open(o("BTC", Side::Long, 10.0, 100.0, Some(90.0), Some(130.0)), 0).unwrap();
        pf.gerer(
            id,
            Gestion {
                paliers: vec![Palier { prix: 110.0, part: 0.5 }],
                ..Default::default()
            },
        )
        .unwrap();
        // la bougie touche le stop et l'objectif partiel : le pire cas, tout sort au stop
        let c = joue(&mut pf, bougie(1800, 100.0, 112.0, 89.0, 100.0));
        assert_eq!((c.len(), c[0].reason.as_str(), c[0].qty), (1, "stop", 10.0));
        // ouverture au-dessus de l'objectif partiel : la moitié sort à l'ouverture (meilleur prix)
        let id = pf.open(o("BTC", Side::Long, 10.0, 100.0, Some(90.0), Some(130.0)), 2000).unwrap();
        pf.gerer(
            id,
            Gestion {
                paliers: vec![Palier { prix: 110.0, part: 0.5 }],
                ..Default::default()
            },
        )
        .unwrap();
        let c = joue(&mut pf, bougie(3600, 113.0, 114.0, 111.0, 112.0));
        assert_eq!((c[0].qty, c[0].exit), (5.0, 113.0));
    }

    #[test]
    fn sortie_partielle_a_la_main() {
        let mut pf = Portfolio::new("Démo", 10_000.0, 0.001);
        let id = pf.open(o("BTC", Side::Long, 4.0, 100.0, Some(90.0), None), 0).unwrap();
        let c = pf.close_part(id, 1.0, 120.0, 10, "fermé à la main").unwrap();
        assert_eq!((c.qty, c.pnl, c.partiel), (1.0, Money::from_cents(2_000), true));
        // frais : ouverture au prorata (1 × 100 × 0,1 %) + fermeture (1 × 120 × 0,1 %)
        assert_eq!(c.fees, Money::from_cents(22));
        assert_eq!(pf.positions[0].qty, 3.0);
        let c = pf.close_part(id, 9.0, 110.0, 20, "fermé à la main").unwrap();
        assert_eq!((c.qty, c.partiel), (3.0, false), "plus que le reste : tout sort");
        assert!(pf.close_part(id, 1.0, 110.0, 30, "x").is_err());
    }

    #[test]
    fn oco_l_un_part_l_autre_est_annule() {
        let mut pf = Portfolio::new("Démo", 10_000.0, 0.0);
        let haut = o("BTC", Side::Long, 1.0, 110.0, Some(100.0), Some(130.0));
        let bas = o("BTC", Side::Short, 1.0, 90.0, Some(100.0), Some(70.0));
        let (a, b) = pf.place_oco((OrderKind::Stop, haut), (OrderKind::Stop, bas), 100.0, None, 0).unwrap();
        assert_eq!(pf.pending.iter().map(|p| p.oco).collect::<Vec<_>>(), vec![Some(b), Some(a)]);
        let (out, _) = pf.on_bar("BTC", &bougie(1800, 101.0, 111.0, 101.0, 110.0), |_| None);
        assert_eq!((out.filled.len(), out.oco.clone()), (1, vec![b]));
        assert!(pf.pending.is_empty() && pf.positions[0].side == Side::Long);
        // un couple mal placé : rien n'est posé
        let haut = o("BTC", Side::Long, 1.0, 95.0, Some(90.0), Some(130.0));
        let bas = o("BTC", Side::Short, 1.0, 90.0, Some(100.0), Some(70.0));
        assert!(pf.place_oco((OrderKind::Stop, haut), (OrderKind::Stop, bas), 100.0, None, 0).is_err());
        assert!(pf.pending.is_empty());
        // annuler l'un délie l'autre
        let haut = o("BTC", Side::Long, 1.0, 110.0, Some(100.0), Some(130.0));
        let bas = o("BTC", Side::Short, 1.0, 90.0, Some(100.0), Some(70.0));
        let (a, b) = pf.place_oco((OrderKind::Stop, haut), (OrderKind::Stop, bas), 100.0, None, 0).unwrap();
        assert!(pf.cancel(a));
        assert_eq!(pf.pending.iter().find(|p| p.id == b).unwrap().oco, None);
    }

    #[test]
    fn gestion_posee_sur_un_ordre_en_attente_et_refus() {
        let mut pf = Portfolio::new("Démo", 10_000.0, 0.0);
        let id = pf
            .place(OrderKind::Limit, o("BTC", Side::Long, 1.0, 95.0, Some(90.0), Some(120.0)), 100.0, None, 0)
            .unwrap();
        let g = Gestion {
            be_r: Some(1.0),
            suiveur: Some(3.0),
            paliers: vec![Palier { prix: 105.0, part: 0.5 }],
        };
        pf.gerer(id, g.clone()).unwrap();
        joue(&mut pf, bougie(1800, 100.0, 100.0, 94.0, 96.0));
        assert_eq!(pf.positions[0].gestion, g, "la position garde la gestion de l'ordre");
        let pid = pf.positions[0].id;
        let refus = |g: Gestion| gestion_error(Side::Long, 100.0, Some(90.0), Some(120.0), &g);
        assert!(refus(Gestion {
            be_r: Some(0.0),
            ..Default::default()
        })
        .is_some());
        assert!(refus(Gestion {
            suiveur: Some(-1.0),
            ..Default::default()
        })
        .is_some());
        assert!(
            refus(Gestion {
                paliers: vec![Palier { prix: 95.0, part: 0.5 }],
                ..Default::default()
            })
            .is_some(),
            "côté perte"
        );
        assert!(
            refus(Gestion {
                paliers: vec![Palier { prix: 125.0, part: 0.5 }],
                ..Default::default()
            })
            .is_some(),
            "après le final"
        );
        let trop = vec![Palier { prix: 105.0, part: 0.7 }, Palier { prix: 110.0, part: 0.5 }];
        assert!(refus(Gestion {
            paliers: trop,
            ..Default::default()
        })
        .is_some());
        assert!(gestion_error(
            Side::Long,
            100.0,
            None,
            None,
            &Gestion {
                be_r: Some(1.0),
                ..Default::default()
            }
        )
        .is_some());
        assert!(pf
            .gerer(
                pid,
                Gestion {
                    be_r: Some(-2.0),
                    ..Default::default()
                }
            )
            .is_err());
        // vieille sauvegarde : pas de gestion, le R vient du stop actuel
        let vieux = r#"{"id":1,"base":"BTC","side":"Long","qty":2.0,"entry":100.0,"stop":90.0,"target":null,"opened_at":0}"#;
        let p: Position = serde_json::from_str(vieux).unwrap();
        assert!(p.gestion.vide() && p.r_unite() == 10.0 && p.qty_depart() == 2.0);
    }

    /// 💸 Coûts d'une bourse : maker 0,02 %, taker 0,05 %, écart 0,02 % (moitié payée au marché), glissement 0,01 %.
    fn bourse() -> Couts {
        Couts {
            maker: 0.0002,
            taker: 0.0005,
            ecart: 0.0002,
            glissement: 0.0001,
        }
    }

    fn avec_couts(o: Order) -> Order {
        Order { couts: Some(bourse()), ..o }
    }

    #[test]
    fn couts_reels_marche_stop_et_objectif() {
        // achat au marché de 1 à 10 000 : taker 5 $, écart + glissement 0,02 % = 2 $ payés tout de suite
        let mut pf = Portfolio::new("Démo", 100_000.0, 0.0);
        let id = pf
            .open(avec_couts(o("BTC", Side::Long, 1.0, 10_000.0, Some(9_900.0), Some(10_200.0))), 0)
            .unwrap();
        assert_eq!(pf.balance, Money::from_f64(100_000.0 - 7.0));
        // le stop touché part au marché : taker 4,95 $ + 1,98 $ à 9 900
        let b = Bar {
            time: 60,
            open: 10_000.0,
            high: 10_010.0,
            low: 9_890.0,
            close: 9_895.0,
            volume: 0.0,
        };
        let (_, c) = pf.on_bar("BTC", &b, |_| None);
        assert_eq!(c[0].reason, "stop");
        assert_eq!(c[0].pnl, Money::from_f64(-100.0), "le prix de sortie reste le niveau du stop");
        assert_eq!(c[0].fees, Money::from_f64(7.0 + 6.93));
        assert_eq!(c[0].glissement, Money::from_f64(2.0 + 1.98));
        assert_eq!(c[0].net(), Money::from_f64(-113.93));
        assert_eq!(pf.balance, Money::from_f64(100_000.0 - 113.93), "le solde a payé chaque centime");
        assert!(pf.positions.iter().all(|p| p.id != id));
        // l'objectif attend dans le carnet : maker 2,04 $ à 10 200, sans écart ni glissement
        pf.open(avec_couts(o("BTC", Side::Long, 1.0, 10_000.0, Some(9_900.0), Some(10_200.0))), 100)
            .unwrap();
        let b = Bar {
            time: 160,
            open: 10_000.0,
            high: 10_210.0,
            low: 9_990.0,
            close: 10_205.0,
            volume: 0.0,
        };
        let (_, c) = pf.on_bar("BTC", &b, |_| None);
        assert_eq!(
            (c[0].reason.as_str(), c[0].fees, c[0].glissement),
            ("objectif", Money::from_f64(9.04), Money::from_f64(2.0))
        );
        assert_eq!(pf.stats().trades, 2);
        assert_eq!(pf.stats().total, Money::from_f64(-113.93 + 200.0 - 9.04));
    }

    #[test]
    fn couts_limite_maker_et_sans_frais() {
        // un Buy Limit atteint : maker seulement (0,02 % de 10 000 = 2 $)
        let mut pf = Portfolio::new("Démo", 100_000.0, 0.0);
        pf.place(
            OrderKind::Limit,
            avec_couts(o("BTC", Side::Long, 1.0, 10_000.0, Some(9_900.0), Some(10_300.0))),
            10_050.0,
            None,
            0,
        )
        .unwrap();
        let r = pf.check_pending("BTC", 10_000.0, 10_000.0, 10, |_| None);
        assert_eq!(r.filled.len(), 1);
        assert!(pf.positions[0].maker_in);
        assert_eq!(pf.balance, Money::from_f64(100_000.0 - 2.0));
        // ✂ la moitié sortie à la main (au marché) : entrée au prorata (1 $) + taker 2,525 $ + 1,01 $ à 10 100
        let t = pf.close_part(pf.positions[0].id, 0.5, 10_100.0, 20, "à la main").unwrap();
        assert_eq!((t.fees, t.glissement), (Money::from_f64(1.0 + 2.53 + 1.01), Money::from_f64(1.01)));
        // « Sans frais » : les nouveaux ordres ne paient rien, la position déjà ouverte garde ses coûts
        pf.sans_frais = true;
        let avant = pf.balance;
        let id = pf.open(avec_couts(o("ETH", Side::Short, 2.0, 1_000.0, Some(1_010.0), None)), 30).unwrap();
        assert_eq!(pf.balance, avant);
        let t = pf.close(id, 990.0, 40, "à la main").unwrap();
        assert_eq!((t.pnl, t.fees), (Money::from_f64(20.0), Money::ZERO));
        assert!(pf.positions[0].couts.is_some());
        pf.reset();
        assert!(pf.sans_frais, "la remise à zéro garde le choix « sans frais »");
    }

    #[test]
    fn anciennes_sauvegardes_sans_couts() {
        let vieux = r#"{"name":"Démo","start":100000.0,"balance":100000.0,"fee_rate":0.001,"positions":[{"id":1,"base":"BTC","side":"Long","qty":1.0,"entry":100.0,"stop":90.0,"target":null,"opened_at":0}],"history":[],"next_id":2}"#;
        let mut pf: Portfolio = serde_json::from_str(vieux).unwrap();
        assert!(!pf.sans_frais && pf.positions[0].couts.is_none());
        // les frais du compte, comme avant : 0,1 % à l'entrée (déjà payés) et à la sortie
        let t = pf.close(1, 110.0, 5, "à la main").unwrap();
        assert_eq!((t.fees, t.glissement), (Money::from_f64(0.1 + 0.11), Money::ZERO));
        let o: Order = serde_json::from_str(r#"{"base":"BTC","side":"Long","qty":1.0,"price":1.0,"stop":null,"target":null}"#).unwrap();
        assert!(o.couts.is_none());
    }

    #[test]
    fn couts_entree_deja_payes() {
        // achat au marché de 2 à 10 000 : ce que le solde a perdu à l'ouverture
        let mut pf = Portfolio::new("Démo", 100_000.0, 0.0);
        let avant = pf.balance;
        let id = pf.open(avec_couts(o("BTC", Side::Long, 2.0, 10_000.0, Some(9_900.0), None)), 0).unwrap();
        let entree = pf.couts_entree(&pf.positions[0]);
        assert!(entree > Money::ZERO);
        assert_eq!(entree, avant - pf.balance);
        // ✂ la moitié sortie : il reste la moitié des coûts d'entrée (à 1 centime près, arrondis)
        pf.close_part(id, 1.0, 10_000.0, 10, "à la main").unwrap();
        let reste = pf.couts_entree(&pf.positions[0]);
        assert!((reste.to_f64() - entree.to_f64() / 2.0).abs() <= 0.01, "{reste:?} / {entree:?}");
    }

    /// ⚡ Un ordre perpétuel ×`l` (marge de maintien 0,5 %, funding toutes les 8 h).
    fn perp(o: Order, l: f64) -> Order {
        Order {
            perp: Some(Perp::new(l, 0.005, 8.0)),
            ..o
        }
    }

    #[test]
    fn marge_isolee_tenue_et_refusee() {
        // 10 000 $ : 1 BTC à 100 000 $ en ×10 tient juste (10 000 $ de marge)
        let mut pf = Portfolio::new("Démo", 10_000.0, 0.0);
        pf.open(perp(o("BTC", Side::Long, 1.0, 100_000.0, None, None), 10.0), 0).unwrap();
        assert_eq!((pf.marge_utilisee(), pf.disponible()), (Money::from_f64(10_000.0), Money::ZERO));
        assert_eq!(pf.positions[0].liquidation(), Some(90_500.0));
        // plus rien de disponible : un autre perpétuel est refusé, au marché comme en attente, avec la raison
        let e = pf.open(perp(o("ETH", Side::Short, 1.0, 1_000.0, None, None), 10.0), 1).unwrap_err().to_string();
        assert!(e.contains("marge insuffisante"), "{e}");
        assert!(pf
            .place(OrderKind::Limit, perp(o("ETH", Side::Long, 1.0, 900.0, None, None), 10.0), 1_000.0, None, 1)
            .is_err());
        // au comptant : pas de marge, comme avant
        assert!(pf.open(o("ETH", Side::Long, 1.0, 1_000.0, None, None), 2).is_ok());
        assert!(pf.positions[1].liquidation().is_none());
    }

    #[test]
    fn liquidation_perd_toute_la_marge() {
        // achat ×10 à 100 000 : liquidation à 90 500, la bougie descend à 90 000 sans stop → −10 000 $ et rien de plus
        let mut pf = Portfolio::new("Démo", 50_000.0, 0.0);
        pf.open(perp(o("BTC", Side::Long, 1.0, 100_000.0, None, None), 10.0), 0).unwrap();
        let (_, c) = pf.on_bar("BTC", &bougie(60, 99_000.0, 99_500.0, 90_000.0, 91_000.0), |_| None);
        assert_eq!(c.len(), 1);
        assert!(c[0].liquide());
        assert_eq!((c[0].pnl, c[0].fees, c[0].levier), (Money::from_f64(-10_000.0), Money::ZERO, Some(10.0)));
        assert_eq!(pf.balance, Money::from_f64(40_000.0));
        // un stop avant la liquidation sort au stop, normalement
        pf.open(perp(o("BTC", Side::Long, 1.0, 100_000.0, Some(95_000.0), None), 10.0), 100).unwrap();
        let (_, c) = pf.on_bar("BTC", &bougie(160, 99_000.0, 99_500.0, 90_000.0, 91_000.0), |_| None);
        assert_eq!((c[0].reason.as_str(), c[0].pnl), ("stop", Money::from_f64(-5_000.0)));
        // un trou de cotation sous la liquidation : le stop ne sauve pas, la marge est perdue
        pf.open(perp(o("BTC", Side::Long, 1.0, 100_000.0, Some(95_000.0), None), 10.0), 200).unwrap();
        let (_, c) = pf.on_bar("BTC", &bougie(260, 89_000.0, 89_500.0, 88_000.0, 88_500.0), |_| None);
        assert!(c[0].liquide());
        assert_eq!((c[0].exit, c[0].pnl), (89_000.0, Money::from_f64(-10_000.0)));
        // une vente ×20 : liquidation au-dessus (100 000 × (1 + 0,05 − 0,005) = 104 500)
        pf.open(perp(o("BTC", Side::Short, 0.5, 100_000.0, Some(110_000.0), None), 20.0), 300).unwrap();
        assert!((pf.positions[0].liquidation().unwrap() - 104_500.0).abs() < 1e-6);
        let (_, c) = pf.on_bar("BTC", &bougie(360, 101_000.0, 105_000.0, 100_500.0, 104_000.0), |_| None);
        assert!(c[0].liquide());
        assert!((c[0].exit - 104_500.0).abs() < 1e-6);
        assert_eq!(c[0].pnl, Money::from_f64(-2_500.0));
    }

    #[test]
    fn funding_paye_a_chaque_echeance() {
        let jour = 1_795_132_800; // 2026-11-20 00:00 UTC
        let mut pf = Portfolio::new("Démo", 50_000.0, 0.0);
        let id = pf.open(perp(o("BTC", Side::Long, 2.0, 100_000.0, None, None), 10.0), jour + 7 * 3600).unwrap();
        // 08:00 et 16:00 à 0,01 % de 200 000 $ : 2 × 20 $
        let r = pf.payer_funding(jour + 17 * 3600, |_| Some(0.0001), |_| Some(100_000.0));
        assert_eq!(r, vec![("BTC".to_string(), Money::from_f64(40.0))]);
        assert_eq!(pf.balance, Money::from_f64(50_000.0 - 40.0));
        assert_eq!(pf.deja_paye(&pf.positions[0]), Money::from_f64(40.0));
        // déjà payé jusque-là : rien de plus ; sans taux connu : la position attend
        assert!(pf.payer_funding(jour + 17 * 3600, |_| Some(0.0001), |_| Some(100_000.0)).is_empty());
        assert!(pf.payer_funding(jour + 30 * 3600, |_| None, |_| Some(100_000.0)).is_empty());
        // ✂ la moitié sortie emporte la moitié du funding ; le net le compte
        let t = pf.close_part(id, 1.0, 100_000.0, jour + 17 * 3600, "à la main").unwrap();
        assert_eq!((t.funding, pf.positions[0].funding), (Money::from_f64(20.0), Money::from_f64(20.0)));
        assert_eq!(t.net(), Money::from_f64(-20.0));
        // une vente reçoit le funding positif ; au comptant : pas de funding
        let mut pf = Portfolio::new("Démo", 50_000.0, 0.0);
        pf.open(perp(o("BTC", Side::Short, 1.0, 100_000.0, None, None), 5.0), jour).unwrap();
        pf.open(o("ETH", Side::Long, 1.0, 1_000.0, None, None), jour).unwrap();
        let r = pf.payer_funding(jour + 8 * 3600, |_| Some(0.0001), |_| Some(100_000.0));
        assert_eq!(r, vec![("BTC".to_string(), Money::from_f64(-10.0))]);
        assert_eq!(pf.balance, Money::from_f64(50_010.0));
        // une ancienne sauvegarde : au comptant, sans marge ni funding
        let vieux = r#"{"id":1,"base":"BTC","side":"Long","qty":1.0,"entry":100.0,"stop":90.0,"target":null,"opened_at":0}"#;
        let p: Position = serde_json::from_str(vieux).unwrap();
        assert!(p.perp.is_none() && p.funding == Money::ZERO && p.marge() == Money::ZERO);
    }

    #[test]
    fn oco_perpetuel_une_seule_marge() {
        // deux ordres Stop ×10 (8 080 $ et 7 920 $ de marge) : un seul peut partir, le plus grand est tenu
        let mut pf = Portfolio::new("Démo", 10_000.0, 0.0);
        let haut = perp(o("BTC", Side::Long, 0.8, 101_000.0, None, None), 10.0);
        let bas = perp(o("BTC", Side::Short, 0.8, 99_000.0, None, None), 10.0);
        pf.place_oco((OrderKind::Stop, haut), (OrderKind::Stop, bas), 100_000.0, None, 0).unwrap();
        assert_eq!(pf.marge_utilisee(), Money::from_f64(8_080.0));
        // il reste 1 920 $ : un ordre de 2 450 $ de marge est refusé
        let e = pf.place(
            OrderKind::Limit,
            perp(o("BTC", Side::Long, 0.25, 98_000.0, None, None), 10.0),
            100_000.0,
            None,
            1,
        );
        assert!(e.is_err());
        // la cassure par le haut : l'achat part (sa marge ne compte pas deux fois), la vente est annulée
        let r = pf.check_pending("BTC", 101_500.0, 100_500.0, 10, |_| None);
        assert_eq!((r.filled.len(), r.oco.len(), pf.pending.len()), (1, 1, 0));
        assert_eq!(pf.marge_utilisee(), Money::from_f64(8_080.0));
        // trop grand pour le compte : refusé dès la pose, rien n'est posé
        let mut pf = Portfolio::new("Démo", 5_000.0, 0.0);
        let haut = perp(o("BTC", Side::Long, 0.8, 101_000.0, None, None), 10.0);
        let bas = perp(o("BTC", Side::Short, 0.8, 99_000.0, None, None), 10.0);
        assert!(pf.place_oco((OrderKind::Stop, haut), (OrderKind::Stop, bas), 100_000.0, None, 0).is_err());
        assert!(pf.pending.is_empty());
    }

    /// 🤖 Lot 7 : le stop et l'objectif glissés à la main sont notés (plus loin, plus près de l'entrée), et le trade
    /// fermé garde son plan de départ, ce qu'il est devenu et ces retouches.
    #[test]
    fn retouches_a_la_main_gardees_dans_le_plan() {
        let mut pf = Portfolio::new("Démo", 10_000.0, 0.0);
        let id = pf.open(o("BTC", Side::Long, 1.0, 100.0, Some(90.0), Some(120.0)), 0).unwrap();
        assert_eq!(pf.positions[0].retouches, Retouches::default());
        // le stop éloigné deux fois : le plus loin est gardé, même quand il revient ensuite plus près
        pf.set_level(id, true, 85.0).unwrap();
        pf.set_level(id, true, 80.0).unwrap();
        pf.set_level(id, true, 88.0).unwrap();
        // l'objectif éloigné puis rapproché
        pf.set_level(id, false, 130.0).unwrap();
        pf.set_level(id, false, 110.0).unwrap();
        let r = pf.positions[0].retouches;
        assert_eq!(r.stop_eloigne, Some(80.0));
        assert!(r.stop_rapproche && r.objectif_eloigne && r.objectif_rapproche);
        let t = pf.close(id, 95.0, 10, "fermé à la main").unwrap();
        let plan = t.plan.unwrap();
        assert_eq!(
            (plan.stop0, plan.target0, plan.stop, plan.target),
            (Some(90.0), Some(120.0), Some(88.0), Some(110.0))
        );
        assert_eq!(plan.retouches, r);
        assert_eq!(plan.lecture, None);
        // à la vente, « plus loin » est au-dessus pour le stop et en dessous pour l'objectif
        let id = pf.open(o("ETH", Side::Short, 1.0, 100.0, Some(110.0), Some(80.0)), 20).unwrap();
        pf.set_level(id, true, 115.0).unwrap();
        pf.set_level(id, false, 70.0).unwrap();
        let r = pf.positions[0].retouches;
        assert_eq!(
            (r.stop_eloigne, r.stop_rapproche, r.objectif_eloigne, r.objectif_rapproche),
            (Some(115.0), false, true, false)
        );
        // le même prix n'est pas une retouche
        let id = pf.open(o("SOL", Side::Long, 1.0, 100.0, Some(90.0), Some(120.0)), 30).unwrap();
        pf.set_level(id, true, 90.0).unwrap();
        assert_eq!(pf.positions.iter().find(|p| p.id == id).unwrap().retouches, Retouches::default());
    }

    /// 🤖 Lot 7 : la gestion (break-even automatique, stop suiveur) et « Stop à l'entrée » font partie du plan : ce
    /// ne sont pas des retouches ; le stop de départ reste celui de l'ouverture.
    #[test]
    fn gestion_et_stop_a_l_entree_ne_sont_pas_des_retouches() {
        let mut pf = Portfolio::new("Démo", 10_000.0, 0.0);
        let id = pf.open(o("BTC", Side::Long, 1.0, 100.0, Some(90.0), Some(130.0)), 0).unwrap();
        pf.gerer(
            id,
            Gestion {
                be_r: Some(1.0),
                ..Gestion::default()
            },
        )
        .unwrap();
        // +1 R : le stop passe à l'entrée, puis le prix y revient
        pf.check_exits("BTC", 110.0, 104.0, 10);
        let c = pf.check_exits("BTC", 101.0, 99.0, 20);
        assert_eq!(c[0].reason, "break-even");
        let plan = c[0].plan.clone().unwrap();
        assert_eq!((plan.stop0, plan.stop), (Some(90.0), Some(100.0)));
        assert_eq!(plan.retouches, Retouches::default());
        let id = pf.open(o("ETH", Side::Long, 1.0, 100.0, Some(90.0), Some(130.0)), 30).unwrap();
        pf.set_stop_entree(id).unwrap();
        assert_eq!(pf.positions[0].retouches, Retouches::default());
    }

    /// 🤖 Lot 7 : une position et un trade fermé des sauvegardes d'avant se lisent sans plan ni lecture.
    #[test]
    fn anciennes_sauvegardes_sans_plan() {
        let vieux = r#"{"id":1,"base":"BTC","side":"Long","qty":2.0,"entry":100.0,"stop":90.0,"target":120.0,"opened_at":0}"#;
        let p: Position = serde_json::from_str(vieux).unwrap();
        assert_eq!((p.target0, p.retouches, &p.lecture), (None, Retouches::default(), &None));
        let vieux = r#"{"base":"BTC","side":"Long","qty":1.0,"entry":100.0,"exit":110.0,"pnl":1000,"fees":0,"opened_at":0,"closed_at":1,"reason":"objectif"}"#;
        let t: ClosedTrade = serde_json::from_str(vieux).unwrap();
        assert_eq!(t.plan, None);
        // une position d'avant fermée : son objectif actuel tient lieu d'objectif de départ
        let mut pf = Portfolio::new("Démo", 10_000.0, 0.0);
        pf.positions.push(Position { id: 7, ..p });
        pf.next_id = 8;
        let t = pf.close(7, 110.0, 5, "fermé à la main").unwrap();
        let plan = t.plan.unwrap();
        assert_eq!((plan.stop0, plan.target0), (Some(90.0), Some(120.0)));
    }

    /// 📒 Module 31: the account block (equity, available, margins, maintenance, exposure) at the current prices, and
    /// ⚡ « Funding du jour » from the payments of the day (a payment of another day does not count).
    #[test]
    fn bloc_compte_et_funding_du_jour() {
        let mut pf = Portfolio::default();
        let vide = pf.summary(|_| Some(60_000.0));
        assert_eq!((vide.equity, vide.margin_ratio(), vide.leverage()), (Money::from_cents(10_000_000), None, 0.0));
        let perp = Perp::new(10.0, 0.005, 8.0);
        pf.open(
            Order {
                perp: Some(perp),
                ..o("BTC", Side::Long, 0.8325, 60_057.9, None, None)
            },
            0,
        )
        .unwrap();
        let s = pf.summary(|b| (b == "BTC").then_some(60_157.9));
        assert_eq!(s.unrealized, Money::from_cents(8_325));
        assert_eq!(s.equity, Money::from_cents(10_008_325));
        assert_eq!(s.margin_used, Money::from_cents(499_982));
        assert_eq!(s.available, Money::from_cents(9_500_018), "isolated margin: the open P&L is not lent");
        assert_eq!((s.long, s.short), (Money::from_cents(5_008_145), Money::ZERO));
        assert_eq!(s.maintenance, Money::from_cents(25_041));
        assert_eq!(format!("{:.2} {:.2}", s.margin_ratio().unwrap(), s.leverage()), "0.25 0.50");
        // a position without a price counts for nothing
        assert_eq!(pf.summary(|_| None).positions, 0);
        // ⚡ funding: one payment at 08:00 UTC (0,8325 × 60 000 × 0,01 %), then one the next day
        pf.payer_funding(8 * 3600, |_| Some(0.0001), |_| Some(60_000.0));
        pf.payer_funding(86_400 + 3_600, |_| Some(0.0001), |_| Some(60_000.0));
        assert_eq!(pf.funding_log.len(), 3, "08:00, then 16:00 and 00:00 caught up at 01:00");
        let jour2 = |t: i64| t >= 86_400;
        assert_eq!(pf.funding_paid(jour2), Money::from_cents(500), "only the payment of day 2 (00:00)");
        assert_eq!(pf.funding_paid(|_| true), Money::from_cents(1_500));
    }
}
