//! 📖 The Carnet panel's arithmetic (mock-up v1, Jee, 22 Nov 2026): a book grouped by a price step, its imbalance,
//! what moved on each price during a window (placed, pulled, filled by trades: the Delta tab) and the coins traded on
//! each price (the DOM tab). Plain numbers in, plain numbers out: the app gives the books and the trades.

use std::collections::{BTreeMap, VecDeque};

/// A grouped level: its price and its size (coins).
pub type Niveau = (f64, f64);

/// The bucket of `px` on a grid of `pas`: a bid goes down to its step, an ask up to its (60 093,4 → 60 090 as a bid,
/// 60 096,2 → 60 100 as an ask, with a step of 5). The tiny margin keeps 60 095 on 60 095 despite the floats.
pub fn case(px: f64, pas: f64, achat: bool) -> i64 {
    let x = px / pas;
    if achat {
        (x + 1e-9).floor() as i64
    } else {
        (x - 1e-9).ceil() as i64
    }
}

/// One side grouped by `pas` (best first, at most `n` levels); `pas` ≤ 0: the levels as they are.
pub fn regrouper(levels: &[(f64, f64)], pas: f64, achat: bool, n: usize) -> Vec<Niveau> {
    if !(pas > 0.0) {
        return levels.iter().copied().filter(|l| l.1 > 0.0).take(n).collect();
    }
    let mut m: BTreeMap<i64, f64> = BTreeMap::new();
    for &(px, q) in levels {
        if px > 0.0 && q > 0.0 {
            *m.entry(case(px, pas, achat)).or_default() += q;
        }
    }
    let lv = |(k, q): (&i64, &f64)| (*k as f64 * pas, *q);
    if achat {
        m.iter().rev().take(n).map(lv).collect()
    } else {
        m.iter().take(n).map(lv).collect()
    }
}

/// The buy side's share of the sizes shown (0 to 1); `None` with nothing on either side.
pub fn part_achat(bids: &[Niveau], asks: &[Niveau]) -> Option<f64> {
    let (b, a) = (bids.iter().map(|l| l.1).sum::<f64>(), asks.iter().map(|l| l.1).sum::<f64>());
    (b + a > 0.0).then(|| b / (b + a))
}

/// The price steps offered for a price: 1, 2, 5 × 10ⁿ around 1/10 000 of the price (BTC at 60 000: 1, 2, 5, 10, 25,
/// 50 $).
pub fn pas_possibles(px: f64) -> Vec<f64> {
    if !(px > 0.0) {
        return vec![];
    }
    let base = px / 10_000.0;
    let e = 10f64.powf(base.log10().floor());
    let mut v: Vec<f64> = [1.0, 2.0, 5.0, 10.0, 25.0, 50.0].iter().map(|m| m * e).collect();
    // (whole numbers stay whole: 2,5 × 10 is 25, never 24,999…)
    for x in v.iter_mut() {
        *x = (*x / e).round() * e;
    }
    v
}

/// The step shown: the one chosen when it fits the price, else the 3rd one offered (BTC: 5 $, the mock-up's).
pub fn pas_du_prix(px: f64, choisi: f64) -> f64 {
    let v = pas_possibles(px);
    if let Some(p) = v.iter().find(|p| (*p - choisi).abs() <= *p * 1e-9) {
        return *p;
    }
    v.get(2).copied().unwrap_or(0.0)
}

/// The smallest gap between two prices of a side (the book's own grouping; 0 with fewer than two levels).
pub fn finesse(levels: &[(f64, f64)]) -> f64 {
    levels
        .windows(2)
        .map(|w| (w[0].0 - w[1].0).abs())
        .filter(|d| *d > 0.0)
        .fold(0.0, |m: f64, d| if m == 0.0 { d } else { m.min(d) })
}

/// The step the panel can really show: the one chosen ([`pas_du_prix`]), or the first one offered at least as coarse
/// as the book read (`fin`: its own grouping, a coarser book would leave empty rows between its levels).
pub fn pas_effectif(px: f64, choisi: f64, fin: f64) -> f64 {
    let p = pas_du_prix(px, choisi);
    if fin <= p * (1.0 + 1e-9) {
        return p;
    }
    pas_possibles(px).into_iter().find(|q| *q >= fin * (1.0 - 1e-9)).unwrap_or(fin)
}

/// 📖 Hyperliquid's book in 20 levels a side: the significant figures (2 to 4) whose grouping is the largest one not
/// above `pas` (BTC at 90 000 and 10 $: 4, groups of 10 $); `None`: its full precision (5 figures).
pub fn sig_hyperliquid(px: f64, pas: f64) -> Option<u8> {
    if !(px > 0.0 && pas > 0.0) {
        return None;
    }
    let e = px.log10().floor();
    let s = (e + 1.0 - pas.log10() - 1e-9).ceil().clamp(2.0, 5.0) as u8;
    (s < 5).then_some(s)
}

/// What moved on one price during a window: coins placed, pulled (gone without a trade), filled (traded there).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Mouvement {
    pub pose: f64,
    pub retire: f64,
    pub rempli: f64,
}

/// 📖 Delta: each book that arrives is compared with the one before, price by price (buckets of `pas`, on both
/// books' common span only: a level that comes into a snapshot's reach was not « placed »). What grew was placed;
/// what shrank was filled up to the trades counted there, the rest was pulled. Five minutes are kept.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Delta {
    pub pas: f64,
    /// The last book's sizes per (bid side, bucket), and its span (lowest bid, highest ask).
    avant: BTreeMap<(bool, i64), f64>,
    span: Option<(f64, f64)>,
    /// (time ms, bid side, bucket, placed, gone).
    mouvements: VecDeque<(i64, bool, i64, f64, f64)>,
    /// (time ms, resting side hit: bid side, bucket, size).
    trades: VecDeque<(i64, bool, i64, f64)>,
}

/// The longest window kept (5 min).
pub const GARDE_MS: i64 = 5 * 60_000;

impl Delta {
    pub fn new(pas: f64) -> Self {
        Self { pas, ..Default::default() }
    }

    /// A whole book (bids best first, asks best first) at `t_ms`.
    pub fn livre(&mut self, t_ms: i64, bids: &[(f64, f64)], asks: &[(f64, f64)]) {
        if !(self.pas > 0.0) {
            return;
        }
        let mut now: BTreeMap<(bool, i64), f64> = BTreeMap::new();
        for (achat, side) in [(true, bids), (false, asks)] {
            for &(px, q) in side {
                if px > 0.0 && q > 0.0 {
                    *now.entry((achat, case(px, self.pas, achat))).or_default() += q;
                }
            }
        }
        let lo = bids.iter().chain(asks).map(|l| l.0).fold(f64::INFINITY, f64::min);
        let hi = bids.iter().chain(asks).map(|l| l.0).fold(f64::NEG_INFINITY, f64::max);
        let span = (lo.is_finite() && hi.is_finite()).then_some((lo, hi));
        if let (Some((alo, ahi)), Some((lo, hi))) = (self.span, span) {
            // the common span, whole buckets inside it only
            let (clo, chi) = (alo.max(lo), ahi.min(hi));
            let dedans = |achat: bool, k: i64| {
                let px = k as f64 * self.pas;
                px - self.pas >= clo - 1e-9 && px + self.pas <= chi + 1e-9 && (achat || px >= clo) && (!achat || px <= chi)
            };
            let cles: std::collections::BTreeSet<(bool, i64)> = self.avant.keys().chain(now.keys()).copied().collect();
            for (achat, k) in cles {
                if !dedans(achat, k) {
                    continue;
                }
                let (a, n) = (
                    self.avant.get(&(achat, k)).copied().unwrap_or(0.0),
                    now.get(&(achat, k)).copied().unwrap_or(0.0),
                );
                let d = n - a;
                if d.abs() > 1e-12 {
                    self.mouvements.push_back((t_ms, achat, k, d.max(0.0), (-d).max(0.0)));
                }
            }
        }
        (self.avant, self.span) = (now, span);
        self.oublier(t_ms);
    }

    /// A trade: `achat` = the taker bought (it filled the asks).
    pub fn trade(&mut self, t_ms: i64, px: f64, taille: f64, achat: bool) {
        if !(self.pas > 0.0 && px > 0.0 && taille > 0.0) {
            return;
        }
        let cote = !achat;
        self.trades.push_back((t_ms, cote, case(px, self.pas, cote), taille));
        self.oublier(t_ms);
    }

    fn oublier(&mut self, t_ms: i64) {
        while self.mouvements.front().is_some_and(|m| m.0 < t_ms - GARDE_MS) {
            self.mouvements.pop_front();
        }
        while self.trades.front().is_some_and(|m| m.0 < t_ms - GARDE_MS) {
            self.trades.pop_front();
        }
    }

    /// What moved on each (bid side, bucket) during the last `fenetre_ms` before `now_ms`.
    pub fn fenetre(&self, now_ms: i64, fenetre_ms: i64) -> BTreeMap<(bool, i64), Mouvement> {
        let depuis = now_ms - fenetre_ms;
        let mut ote: BTreeMap<(bool, i64), f64> = BTreeMap::new();
        let mut out: BTreeMap<(bool, i64), Mouvement> = BTreeMap::new();
        for &(t, achat, k, pose, parti) in &self.mouvements {
            if t > depuis && t <= now_ms {
                out.entry((achat, k)).or_default().pose += pose;
                *ote.entry((achat, k)).or_default() += parti;
            }
        }
        for &(t, achat, k, q) in &self.trades {
            if t > depuis && t <= now_ms {
                out.entry((achat, k)).or_default().rempli += q;
            }
        }
        for (cle, parti) in ote {
            let m = out.entry(cle).or_default();
            m.retire = (parti - m.rempli).max(0.0);
        }
        out
    }

    /// The net change of the resting sizes during the window, (bid side, ask side): placed − pulled − filled.
    pub fn net(&self, now_ms: i64, fenetre_ms: i64) -> (f64, f64) {
        let mut n = (0.0, 0.0);
        for ((achat, _), m) in self.fenetre(now_ms, fenetre_ms) {
            let v = m.pose - m.retire - m.rempli;
            if achat {
                n.0 += v;
            } else {
                n.1 += v;
            }
        }
        n
    }
}

/// 📖 DOM: the coins traded on each bucket of `pas` (buys + sells), from 1-minute orderflow bars since `depuis_s`.
pub fn echange(minutes: &[crate::orderflow::FlowBar], depuis_s: i64, pas: f64) -> BTreeMap<i64, f64> {
    let mut m: BTreeMap<i64, f64> = BTreeMap::new();
    if !(pas > 0.0) {
        return m;
    }
    for b in minutes.iter().filter(|b| b.time >= depuis_s) {
        for &(num, achats, ventes) in &b.rangees {
            // the row's middle price, on the bid grid (a bucket holds [k × pas, (k + 1) × pas))
            let px = (num as f64 + 0.5) * b.pas;
            *m.entry(case(px, pas, true)).or_default() += achats + ventes;
        }
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pas_suit_le_carnet_lu() {
        let bids = [(60_100.0, 1.0), (60_000.0, 1.0), (59_900.0, 1.0)];
        assert_eq!(finesse(&bids), 100.0);
        assert_eq!(finesse(&bids[..1]), 0.0);
        // BTC at 60 000: 5 $ chosen on a book in 1 $ steps; the same on a book grouped by 100 $ (50 $ at most offered)
        assert_eq!(pas_effectif(60_000.0, 5.0, 1.0), 5.0);
        assert_eq!(pas_effectif(60_000.0, 5.0, 10.0), 10.0);
        assert_eq!(pas_effectif(60_000.0, 5.0, 100.0), 100.0);
        assert_eq!(pas_effectif(60_000.0, 0.0, 0.0), 5.0, "nothing read yet: the price's own");
    }

    #[test]
    fn chiffres_de_hyperliquid() {
        assert_eq!(sig_hyperliquid(90_000.0, 5.0), None, "1 $ groups: its full precision");
        assert_eq!(sig_hyperliquid(90_000.0, 10.0), Some(4));
        assert_eq!(sig_hyperliquid(90_000.0, 50.0), Some(4));
        assert_eq!(sig_hyperliquid(90_000.0, 100.0), Some(3));
        assert_eq!(sig_hyperliquid(3_000.0, 0.5), None);
        assert_eq!(sig_hyperliquid(3_000.0, 1.0), Some(4));
        assert_eq!(sig_hyperliquid(0.0, 1.0), None);
    }

    #[test]
    fn regroupe_par_pas() {
        let bids = [(60_093.4, 1.0), (60_091.0, 2.0), (60_089.9, 0.5), (60_085.0, 4.0)];
        let asks = [(60_096.2, 3.0), (60_099.0, 1.0), (60_100.0, 2.0), (60_101.0, 0.25)];
        assert_eq!(regrouper(&bids, 5.0, true, 10), vec![(60_090.0, 3.0), (60_085.0, 4.5)]);
        assert_eq!(regrouper(&asks, 5.0, false, 10), vec![(60_100.0, 6.0), (60_105.0, 0.25)]);
        assert_eq!(regrouper(&asks, 5.0, false, 1).len(), 1, "n levels at most");
        assert_eq!(regrouper(&bids, 0.0, true, 2), vec![(60_093.4, 1.0), (60_091.0, 2.0)], "no step: as they are");
        let p = part_achat(&[(1.0, 48.0)], &[(2.0, 52.0)]).unwrap();
        assert!((p - 0.48).abs() < 1e-12);
        assert_eq!(part_achat(&[], &[]), None);
    }

    #[test]
    fn pas_du_btc_et_d_une_petite_piece() {
        assert_eq!(pas_possibles(60_096.0), vec![1.0, 2.0, 5.0, 10.0, 25.0, 50.0]);
        assert_eq!(pas_du_prix(60_096.0, 5.0), 5.0);
        assert_eq!(pas_du_prix(60_096.0, 0.0), 5.0, "nothing chosen: the 3rd step");
        assert!((pas_du_prix(2.0, 5.0) - 0.0005).abs() < 1e-15, "a step of another price: the 3rd of this one");
        assert!(pas_possibles(0.0).is_empty());
    }

    #[test]
    fn delta_pose_retire_rempli() {
        let mut d = Delta::new(5.0);
        let bids = |q: f64| vec![(60_095.0, 1.0), (60_090.0, q), (60_085.0, 1.0), (60_000.0, 1.0)];
        let asks = |q: f64| vec![(60_100.0, q), (60_105.0, 1.0), (60_200.0, 1.0)];
        d.livre(0, &bids(10.0), &asks(5.0));
        // 10 s later: 6 more on the 60 090 bid; 4 gone from the 60 100 ask, 3 of them bought there
        d.trade(5_000, 60_098.0, 3.0, true);
        d.livre(10_000, &bids(16.0), &asks(1.0));
        let f = d.fenetre(10_000, 60_000);
        let b = f[&(true, 12_018)];
        assert_eq!((b.pose, b.retire, b.rempli), (6.0, 0.0, 0.0));
        let a = f[&(false, 12_020)];
        assert_eq!((a.pose, a.retire, a.rempli), (0.0, 1.0, 3.0), "4 gone: 3 filled, 1 pulled");
        assert_eq!(d.net(10_000, 60_000), (6.0, -4.0));
        // a 10 s window from 20 s sees nothing
        assert!(d.fenetre(20_001, 10_000).is_empty());
        // kept 5 minutes only
        d.livre(400_000, &bids(16.0), &asks(1.0));
        assert!(d.fenetre(400_000, GARDE_MS * 2).is_empty());
    }

    #[test]
    fn delta_ne_compte_pas_ce_qui_entre_dans_la_portee() {
        let mut d = Delta::new(5.0);
        d.livre(0, &[(60_095.0, 1.0)], &[(60_100.0, 1.0)]);
        // the next snapshot reaches further: 60 050 was not « placed », it came into view
        d.livre(5_000, &[(60_095.0, 1.0), (60_050.0, 40.0)], &[(60_100.0, 1.0), (60_150.0, 30.0)]);
        assert!(d.fenetre(5_000, 10_000).is_empty());
    }

    #[test]
    fn echange_par_case() {
        use crate::orderflow::FlowBar;
        let m = |time, rangees| FlowBar {
            time,
            pas: 1.0,
            rangees,
            complete: true,
        };
        let l = [m(0, vec![(60_091, 1.0, 2.0), (60_096, 0.5, 0.0)]), m(60, vec![(60_093, 4.0, 0.0)])];
        let e = echange(&l, 0, 5.0);
        assert_eq!(e[&12_018], 7.0, "60 091 and 60 093 in 60 090");
        assert_eq!(e[&12_019], 0.5);
        assert_eq!(echange(&l, 60, 5.0)[&12_018], 4.0, "since 60 s only");
    }
}
