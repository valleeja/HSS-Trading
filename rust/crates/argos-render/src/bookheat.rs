//! 🔥 Heatmap du carnet d'ordres (comme v1) : à chaque photo du carnet, pour chaque niveau de prix regroupé par tick,
//! la plus grosse taille posée (en $) pendant la bougie. Dessinée derrière les bougies : plus le mur est gros, plus la
//! bande est vive (dégradé viridis, lisible en thème clair et pour les daltoniens). Historique borné en mémoire ; rien
//! n'est envoyé ni gardé sur disque.

use argos_script::inputs::{check, choice, color as couleur, num, Param};
use std::collections::{BTreeMap, HashMap};

pub const GROUPS: [&str; 3] = ["🔥 Heatmap du carnet", "🧱 Murs du carnet", "📖 Carnets lus (heatmap et carnet d'ordres)"];

/// Where the books come from (heatmap and order-book panel).
pub const SOURCES: [&str; 2] = ["Bourse de la chart", "Bourses cochées ci-dessous"];
/// Quote currency of the books read.
pub const MONNAIES: [&str; 4] = ["Celle de la chart", "USDT", "USDC", "USDT + USDC (additionnés)"];

pub const PARAMS: [Param; 18] = [
    num("thr", 0, "Seuil (% du plafond, en dessous : rien)", 70.0, 0.0, 95.0, 5.0),
    num("ceil", 0, "Plafond des couleurs (centile)", 99.0, 80.0, 100.0, 1.0),
    num("levels", 0, "Niveaux gardés par bougie (les plus gros)", 40.0, 5.0, 200.0, 5.0),
    choice("res", 0, "Regroupement des prix", 0, &["HD (1 ‰ du prix)", "4K (0,4 ‰)"]),
    num("op", 0, "Opacité (%)", 85.0, 10.0, 100.0, 5.0),
    // order-book walls on the price scale (added at the end: saved settings keep their place)
    check("walls", 1, "Murs du carnet sur l'échelle de prix (SELL / BUY)", false),
    num("wallsN", 1, "Murs par côté", 3.0, 1.0, 10.0, 1.0),
    num("wallsMult", 1, "Taille minimum (× la moyenne du carnet)", 3.0, 1.0, 50.0, 0.5),
    num("wallsDepth", 1, "Profondeur max (% du prix)", 2.0, 0.1, 12.0, 0.1),
    couleur("wallsSell", 1, "Couleur SELL", 0xf23645),
    couleur("wallsBuy", 1, "Couleur BUY", 0x089981),
    // 📖 the books read, shared with the order-book panel (Jee, 21 Nov)
    choice("source", 2, "Carnets", 0, &SOURCES),
    check("cBinance", 2, "Binance (perp)", true),
    check("cBybit", 2, "Bybit (perp)", true),
    check("cOkx", 2, "OKX (perp)", true),
    check("cBitunix", 2, "Bitunix (perp)", true),
    check("cHyperliquid", 2, "Hyperliquid (perp)", true),
    choice("monnaie", 2, "Monnaie", 0, &MONNAIES),
];

/// Écart gardé autour du prix (±12 %, comme v1).
const AROUND: f64 = 0.12;

/// Historique du carnet : une colonne par bougie (heure d'ouverture), niveau (prix / tick) → taille max en $.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BookHeat {
    /// Regroupement des prix.
    pub tick: f64,
    pub cols: BTreeMap<i64, HashMap<i64, f64>>,
    /// Colonnes gardées au plus (les plus anciennes partent).
    pub max_cols: usize,
    /// Symbole et unité de temps de l'historique (un autre : on repart de zéro).
    pub key: String,
}

/// Pas « rond » au-dessus de `x` (1, 2, 2,5, 5 × 10ⁿ).
pub fn nice(x: f64) -> f64 {
    if !(x > 0.0) {
        return 1.0;
    }
    let e = 10f64.powf(x.log10().floor());
    [1.0, 2.0, 2.5, 5.0, 10.0].iter().map(|m| m * e).find(|v| *v >= x).unwrap_or(10.0 * e)
}

/// Tick de regroupement pour un prix : un millième du prix (« HD ») ou 0,4 millième (« 4K »), arrondi.
pub fn tick_for(px: f64, fine: bool) -> f64 {
    nice(px * if fine { 0.0004 } else { 0.001 })
}

/// Meilleur achat et meilleure vente : (bid, ask). `None` si un côté est vide.
pub fn best(bids: &[(f64, f64)], asks: &[(f64, f64)]) -> Option<(f64, f64)> {
    let b = bids.iter().map(|l| l.0).fold(f64::NEG_INFINITY, f64::max);
    let a = asks.iter().map(|l| l.0).fold(f64::INFINITY, f64::min);
    (b.is_finite() && a.is_finite()).then_some((b, a))
}

impl BookHeat {
    pub fn new(key: &str, tick: f64, max_cols: usize) -> Self {
        Self {
            tick,
            cols: BTreeMap::new(),
            max_cols,
            key: key.into(),
        }
    }

    /// Niveau (index) du prix `px`.
    pub fn bucket(&self, px: f64) -> i64 {
        (px / self.tick).round() as i64
    }

    /// Une photo du carnet (niveaux `(prix, taille)`) pendant la bougie ouverte à `bar_t`, autour du prix `mid`.
    pub fn add(&mut self, bar_t: i64, levels: impl Iterator<Item = (f64, f64)>, mid: f64) {
        if !(self.tick > 0.0) {
            return;
        }
        let tick = self.tick;
        let col = self.cols.entry(bar_t).or_default();
        for (px, sz) in levels {
            if !(px > 0.0 && sz > 0.0) || (mid > 0.0 && (px / mid - 1.0).abs() > AROUND) {
                continue;
            }
            let k = (px / tick).round() as i64;
            let usd = px * sz;
            let v = col.entry(k).or_insert(0.0);
            if usd > *v {
                *v = usd;
            }
        }
        while self.cols.len() > self.max_cols.max(1) {
            let first = *self.cols.keys().next().unwrap_or(&0);
            self.cols.remove(&first);
        }
    }

    /// Plafond de l'échelle des couleurs : le centile `ceil` (ex. 99) de toutes les tailles gardées.
    pub fn ceiling(&self, ceil: f64) -> f64 {
        let mut v: Vec<f64> = self.cols.values().flat_map(|c| c.values().copied()).collect();
        if v.is_empty() {
            return 0.0;
        }
        v.sort_by(f64::total_cmp);
        let i = ((ceil.clamp(0.0, 100.0) / 100.0) * (v.len() - 1) as f64).round() as usize;
        v[i.min(v.len() - 1)]
    }

    /// Intensité 0..1 d'une taille : 0 sous le seuil (`thr` % du plafond), 1 au plafond et au-dessus.
    pub fn intensity(usd: f64, top: f64, thr: f64) -> f64 {
        if top <= 0.0 {
            return 0.0;
        }
        let lo = top * thr.clamp(0.0, 99.0) / 100.0;
        if usd <= lo {
            0.0
        } else {
            ((usd - lo) / (top - lo).max(f64::EPSILON)).min(1.0)
        }
    }
}

/// An order-book wall: price level, size in $, and side (above the price = SELL, below = BUY).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Wall {
    pub price: f64,
    pub usd: f64,
    pub sell: bool,
}

impl Wall {
    /// Label on the price scale, e.g. « SELL 98 250 · 1,2 M ».
    pub fn label(&self, decimals: usize) -> String {
        format!(
            "{} {} · {}",
            if self.sell { "SELL" } else { "BUY" },
            crate::viewport::format_price(self.price, decimals),
            compact(self.usd)
        )
    }
}

/// Compact dollar size: 850 K, 1,2 M, 3,4 B.
pub fn compact(usd: f64) -> String {
    let (v, unit) = match usd.abs() {
        a if a >= 1e9 => (usd / 1e9, "B"),
        a if a >= 1e6 => (usd / 1e6, "M"),
        a if a >= 1e3 => (usd / 1e3, "K"),
        _ => (usd, ""),
    };
    let s = if v.abs() >= 100.0 { format!("{v:.0}") } else { format!("{v:.1}") };
    format!("{} {unit}", s.replace('.', ",")).trim_end().to_string()
}

impl BookHeat {
    /// Walls of the latest book snapshot (current candle): levels at least `mult` × the column's average size, within
    /// `depth` % of `mid`; the `n` biggest above the price (SELL) and below (BUY), biggest first.
    pub fn walls(&self, mid: f64, n: usize, mult: f64, depth: f64) -> Vec<Wall> {
        let Some(col) = self.cols.values().next_back().filter(|c| !c.is_empty()) else {
            return vec![];
        };
        if !(mid > 0.0 && self.tick > 0.0) {
            return vec![];
        }
        let avg = col.values().sum::<f64>() / col.len() as f64;
        let mut all: Vec<Wall> = col
            .iter()
            .map(|(&k, &usd)| (k as f64 * self.tick, usd))
            .filter(|&(px, usd)| usd >= avg * mult && (px / mid - 1.0).abs() * 100.0 <= depth && px != mid)
            .map(|(price, usd)| Wall { price, usd, sell: price > mid })
            .collect();
        all.sort_by(|a, b| b.usd.total_cmp(&a.usd));
        let side = |sell: bool| all.iter().filter(move |w| w.sell == sell).take(n).copied();
        side(true).chain(side(false)).collect()
    }
}

/// Couleur d'une intensité 0..1 : dégradé viridis (bleu nuit → turquoise → vert → jaune).
pub fn color(t: f64) -> crate::text::Rgb {
    const STOPS: [(f64, f64, f64); 4] = [(59.0, 28.0, 140.0), (33.0, 145.0, 140.0), (94.0, 201.0, 98.0), (253.0, 231.0, 37.0)];
    let t = t.clamp(0.0, 1.0) * 3.0;
    let i = (t.floor() as usize).min(2);
    let f = t - i as f64;
    let (a, b) = (STOPS[i], STOPS[i + 1]);
    let m = |x: f64, y: f64| (x + (y - x) * f).round() as u8;
    crate::text::Rgb(m(a.0, b.0), m(a.1, b.1), m(a.2, b.2))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regroupement_par_tick() {
        assert_eq!(tick_for(80_000.0, false), 100.0);
        assert_eq!(tick_for(80_000.0, true), 50.0);
        assert_eq!(tick_for(4_000.0, false), 5.0);
        let mut h = BookHeat::new("BTC|1h", 10.0, 10);
        h.add(0, [(80_004.0, 1.0), (80_006.0, 2.0), (79_996.0, 0.5)].into_iter(), 80_000.0);
        let col = &h.cols[&0];
        assert_eq!(col.len(), 2, "80 004 et 79 996 → 8 000 ; 80 006 → 8 001");
        assert_eq!(col[&8001], 80_006.0 * 2.0);
        assert_eq!(col[&8000], 80_004.0, "la plus grosse taille du niveau");
        // loin du prix : ignoré
        h.add(0, [(95_000.0, 10.0)].into_iter(), 80_000.0);
        assert_eq!(h.cols[&0].len(), 2);
    }

    #[test]
    fn intensite_normalisee() {
        assert_eq!(BookHeat::intensity(50.0, 100.0, 70.0), 0.0, "sous le seuil");
        assert!((BookHeat::intensity(85.0, 100.0, 70.0) - 0.5).abs() < 1e-9);
        assert_eq!(BookHeat::intensity(500.0, 100.0, 70.0), 1.0, "plafonné");
        assert_eq!(BookHeat::intensity(5.0, 0.0, 70.0), 0.0, "pas de données");
        let mut h = BookHeat::new("BTC|1h", 1.0, 10);
        h.add(0, (1..=100).map(|i| (1_000.0 + i as f64, 1.0)), 1_050.0);
        assert!((h.ceiling(99.0) - 1_099.0).abs() < 1.5, "centile 99");
        assert_ne!(color(0.0), color(1.0));
    }

    #[test]
    fn historique_borne() {
        let mut h = BookHeat::new("BTC|1h", 1.0, 3);
        for t in 0..10 {
            h.add(t * 3600, [(100.0, 1.0)].into_iter(), 100.0);
        }
        assert_eq!(h.cols.len(), 3);
        assert_eq!(*h.cols.keys().next().unwrap(), 7 * 3600, "les plus récentes restent");
    }

    #[test]
    fn order_book_walls() {
        let mut h = BookHeat::new("BTC|1h", 10.0, 10);
        // old candle: ignored (only the latest snapshot counts)
        h.add(0, [(100_500.0, 50.0)].into_iter(), 100_000.0);
        // latest candle: small levels around the price, a big ask at 100 500 and a big bid at 99 400
        let small = (1..=20).flat_map(|i| [(100_000.0 + i as f64 * 10.0, 0.1), (100_000.0 - i as f64 * 10.0, 0.1)]);
        h.add(3600, small.chain([(100_500.0, 12.0), (99_400.0, 8.0), (130_000.0, 99.0)]), 100_000.0);
        let w = h.walls(100_000.0, 3, 3.0, 2.0);
        assert_eq!(w.len(), 2, "{w:?}");
        assert_eq!((w[0].price, w[0].sell), (100_500.0, true));
        assert_eq!((w[1].price, w[1].sell), (99_400.0, false));
        let l = w[0].label(0);
        assert!(l.starts_with("SELL ") && l.ends_with(" · 1,2 M"), "{l}");
        assert_eq!(compact(850_000.0), "850 K");
        assert_eq!(compact(3_400_000_000.0), "3,4 B");
        assert!(BookHeat::new("x", 1.0, 3).walls(100.0, 3, 3.0, 2.0).is_empty());
    }

    #[test]
    fn meilleur_bid_et_ask() {
        assert_eq!(best(&[(99.0, 1.0), (98.5, 3.0)], &[(100.5, 1.0), (100.0, 2.0)]), Some((99.0, 100.0)));
        assert_eq!(best(&[], &[(100.0, 1.0)]), None, "un côté vide");
    }
}
