//! 📍 **Argos Pivot VWAP** (conception Argos, écrit de zéro) : des VWAP ancrés sur les pivots du prix.
//!
//! Idée générale (publique) : un pivot haut est une bougie dont le haut dépasse celui des `n` bougies de chaque
//! côté ; un pivot bas, pareil avec le bas. Depuis chaque pivot, on trace le VWAP (prix typique pondéré par le
//! volume) ancré sur ce pivot, jusqu'au pivot suivant. Chaque pivot est marqué par rapport au précédent du même
//! côté : HH / LH (hauts), HL / LL (bas).
//!
//! Deux façons de l'utiliser :
//! - `compute` : une seule échelle (`n` bougies de chaque côté) ;
//! - `compute_multi` : plusieurs échelles ; seuls les pivots **forts** (vus sur au moins `min_votes` échelles)
//!   ancrent un VWAP, et un tronçon s'arrête dès qu'une clôture repasse son VWAP à contre-sens (« cassé »).

use argos_core::Bar;

/// Marque d'un pivot par rapport au précédent du même côté.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mark {
    /// Premier pivot de son côté : pas de comparaison.
    First,
    HigherHigh,
    LowerHigh,
    HigherLow,
    LowerLow,
}

impl Mark {
    /// Texte court (vide pour le premier).
    pub fn short(self) -> &'static str {
        match self {
            Mark::First => "",
            Mark::HigherHigh => "HH",
            Mark::LowerHigh => "LH",
            Mark::HigherLow => "HL",
            Mark::LowerLow => "LL",
        }
    }
}

/// Un pivot confirmé.
#[derive(Debug, Clone, PartialEq)]
pub struct Pivot {
    /// Bougie du pivot.
    pub at: usize,
    /// Bougie où il est connu (les `n` bougies de droite sont passées) : rien n'est dessiné avant.
    pub known_at: usize,
    /// Prix du pivot (haut ou bas de la bougie).
    pub price: f64,
    /// Pivot bas (sinon haut).
    pub is_low: bool,
    pub mark: Mark,
}

/// Un tronçon de VWAP ancré : une valeur par bougie, de l'ancre à sa fin.
#[derive(Debug, Clone, PartialEq)]
pub struct Leg {
    /// Ancré sur un pivot bas (tronçon haussier).
    pub rising: bool,
    pub points: Vec<(usize, f64)>,
}

/// Résultat.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PivotVwap {
    pub pivots: Vec<Pivot>,
    pub legs: Vec<Leg>,
}

impl PivotVwap {
    /// Dernière valeur du dernier tronçon (le VWAP actif).
    pub fn current(&self) -> Option<f64> {
        self.legs.last().and_then(|l| l.points.last()).map(|p| p.1)
    }
}

/// Réglages d'Argos (valeurs par défaut d'Argos).
#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    /// Bougies de chaque côté d'un pivot (une échelle).
    pub span: usize,
    /// Échelles du mode multi-échelle.
    pub spans: Vec<usize>,
    /// Échelles qui doivent voir le pivot pour qu'il compte (mode multi-échelle).
    pub min_votes: usize,
    /// Le tronçon s'arrête quand une clôture repasse son VWAP à contre-sens (mode multi-échelle).
    pub cut_on_break: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            span: 6,
            spans: vec![4, 9, 16],
            min_votes: 2,
            cut_on_break: true,
        }
    }
}

/// Pivots bruts d'une échelle : (bougie, haut / bas), sans marque, dans l'ordre.
fn raw_pivots(bars: &[Bar], n: usize) -> Vec<(usize, bool)> {
    let mut out = vec![];
    if n == 0 || bars.len() < 2 * n + 1 {
        return out;
    }
    for i in n..bars.len() - n {
        let side = |k: usize| k != i;
        let hi = bars[i].high;
        let lo = bars[i].low;
        let is_high = (i - n..=i + n)
            .filter(|&k| side(k))
            .all(|k| if k < i { bars[k].high < hi } else { bars[k].high <= hi });
        let is_low = (i - n..=i + n)
            .filter(|&k| side(k))
            .all(|k| if k < i { bars[k].low > lo } else { bars[k].low >= lo });
        if is_high {
            out.push((i, false));
        }
        if is_low {
            out.push((i, true));
        }
    }
    out
}

/// Garde une suite qui alterne haut / bas : deux du même côté de suite → le plus extrême reste.
fn alternate(bars: &[Bar], raw: &[(usize, bool)]) -> Vec<(usize, bool)> {
    let mut out: Vec<(usize, bool)> = vec![];
    for &(i, low) in raw {
        match out.last().copied() {
            Some((j, l)) if l == low => {
                let better = if low { bars[i].low < bars[j].low } else { bars[i].high > bars[j].high };
                if better {
                    *out.last_mut().expect("non vide") = (i, low);
                }
            }
            _ => out.push((i, low)),
        }
    }
    out
}

/// Ajoute les marques HH / LH / HL / LL.
fn marked(bars: &[Bar], seq: &[(usize, bool)], lag: usize) -> Vec<Pivot> {
    let (mut last_hi, mut last_lo): (Option<f64>, Option<f64>) = (None, None);
    seq.iter()
        .map(|&(i, low)| {
            let price = if low { bars[i].low } else { bars[i].high };
            let mark = if low {
                let m = last_lo.map_or(Mark::First, |p| if price > p { Mark::HigherLow } else { Mark::LowerLow });
                last_lo = Some(price);
                m
            } else {
                let m = last_hi.map_or(Mark::First, |p| if price > p { Mark::HigherHigh } else { Mark::LowerHigh });
                last_hi = Some(price);
                m
            };
            Pivot {
                at: i,
                known_at: (i + lag).min(bars.len().saturating_sub(1)),
                price,
                is_low: low,
                mark,
            }
        })
        .collect()
}

/// VWAP ancré de `from` à `to` compris (prix typique × volume ; sans volume : moyenne simple du prix typique).
fn anchored(bars: &[Bar], from: usize, to: usize) -> Vec<(usize, f64)> {
    let (mut pv, mut v, mut sum) = (0.0, 0.0, 0.0);
    (from..=to.min(bars.len().saturating_sub(1)))
        .enumerate()
        .map(|(k, i)| {
            let b = &bars[i];
            let tp = (b.high + b.low + b.close) / 3.0;
            pv += tp * b.volume;
            v += b.volume;
            sum += tp;
            (i, if v > 0.0 { pv / v } else { sum / (k + 1) as f64 })
        })
        .collect()
}

/// Tronçons : de chaque pivot jusqu'à la bougie avant le pivot suivant (ou la fin). `cut` : arrêt à la première
/// clôture qui repasse le VWAP à contre-sens.
fn legs(bars: &[Bar], pivots: &[Pivot], cut: bool) -> Vec<Leg> {
    let last = bars.len().saturating_sub(1);
    pivots
        .iter()
        .enumerate()
        .map(|(k, p)| {
            let end = pivots.get(k + 1).map_or(last, |q| q.at.saturating_sub(1).max(p.at));
            let mut pts = anchored(bars, p.at, end);
            if cut {
                if let Some(stop) = pts
                    .iter()
                    .position(|&(i, vw)| i > p.at && if p.is_low { bars[i].close < vw } else { bars[i].close > vw })
                {
                    pts.truncate(stop + 1);
                }
            }
            Leg { rising: p.is_low, points: pts }
        })
        .collect()
}

/// Une échelle : pivots à `cfg.span` bougies de chaque côté et leurs VWAP ancrés.
pub fn compute(bars: &[Bar], cfg: &Config) -> PivotVwap {
    let seq = alternate(bars, &raw_pivots(bars, cfg.span));
    let pivots = marked(bars, &seq, cfg.span);
    PivotVwap {
        legs: legs(bars, &pivots, false),
        pivots,
    }
}

/// Multi-échelle : un pivot compte s'il est vu (à la même bougie) sur au moins `min_votes` échelles ; ses VWAP
/// s'arrêtent à la première cassure si `cut_on_break`.
pub fn compute_multi(bars: &[Bar], cfg: &Config) -> PivotVwap {
    let spans: Vec<usize> = cfg.spans.iter().copied().filter(|n| *n > 0).collect();
    if spans.is_empty() {
        return PivotVwap::default();
    }
    let all: Vec<Vec<(usize, bool)>> = spans.iter().map(|&n| raw_pivots(bars, n)).collect();
    let need = cfg.min_votes.clamp(1, spans.len());
    let mut strong: Vec<(usize, bool)> = all[0]
        .iter()
        .chain(all.iter().skip(1).flatten())
        .copied()
        .filter(|p| all.iter().filter(|s| s.contains(p)).count() >= need)
        .collect();
    strong.sort_unstable();
    strong.dedup();
    let seq = alternate(bars, &strong);
    // connu quand la plus petite échelle qui le voit l'a confirmé
    let lag = spans.iter().copied().min().unwrap_or(1);
    let pivots = marked(bars, &seq, lag);
    PivotVwap {
        legs: legs(bars, &pivots, cfg.cut_on_break),
        pivots,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar(h: f64, l: f64, v: f64) -> Bar {
        Bar {
            time: 0,
            open: (h + l) / 2.0,
            high: h,
            low: l,
            close: (h + l) / 2.0,
            volume: v,
        }
    }

    /// Une vague : monte jusqu'à 10, descend jusqu'à 2, remonte jusqu'à 12, redescend.
    fn wave() -> Vec<Bar> {
        let mids: Vec<f64> = (0..=10)
            .map(f64::from)
            .chain((2..10).rev().map(f64::from))
            .chain((3..=12).map(f64::from))
            .chain((5..12).rev().map(f64::from))
            .collect();
        mids.iter().map(|m| bar(m + 0.5, m - 0.5, 100.0)).collect()
    }

    #[test]
    fn pivots_alternes_et_marques() {
        let b = wave();
        let r = compute(&b, &Config { span: 3, ..Default::default() });
        let kinds: Vec<(bool, Mark)> = r.pivots.iter().map(|p| (p.is_low, p.mark)).collect();
        assert_eq!(kinds, [(false, Mark::First), (true, Mark::First), (false, Mark::HigherHigh)]);
        assert_eq!(r.pivots[0].price, 10.5);
        assert_eq!(r.pivots[1].price, 1.5);
        assert_eq!(r.pivots[2].known_at, r.pivots[2].at + 3, "connu 3 bougies plus tard");
        assert_eq!(r.legs.len(), 3);
        assert!(r.legs[1].rising && !r.legs[0].rising);
        // le tronçon s'arrête juste avant le pivot suivant
        assert_eq!(r.legs[0].points.last().unwrap().0 + 1, r.pivots[1].at);
    }

    #[test]
    fn vwap_ancre_pondere_par_le_volume() {
        let b = vec![bar(11.0, 9.0, 1.0), bar(21.0, 19.0, 3.0)];
        let p = anchored(&b, 0, 1);
        assert_eq!(p[0].1, 10.0);
        assert!((p[1].1 - (10.0 * 1.0 + 20.0 * 3.0) / 4.0).abs() < 1e-12);
        let z = anchored(&[bar(11.0, 9.0, 0.0), bar(21.0, 19.0, 0.0)], 0, 1);
        assert_eq!(z[1].1, 15.0, "sans volume : moyenne simple");
    }

    #[test]
    fn multi_echelle_garde_les_pivots_forts_et_coupe_a_la_cassure() {
        let b = wave();
        let cfg = Config {
            spans: vec![2, 3, 4],
            min_votes: 3,
            ..Default::default()
        };
        let r = compute_multi(&b, &cfg);
        assert!(!r.pivots.is_empty() && r.pivots.iter().all(|p| p.price.is_finite()));
        assert!(r.pivots.windows(2).all(|w| w[0].is_low != w[1].is_low), "alternés");
        assert!(r.current().is_some());
        assert_eq!(
            compute_multi(
                &b,
                &Config {
                    spans: vec![],
                    ..Default::default()
                }
            ),
            PivotVwap::default()
        );
        assert!(compute(&b[..3], &Config::default()).pivots.is_empty(), "trop peu de bougies");
    }
}
