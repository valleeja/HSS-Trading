//! 🔠 **TPO** (Time Price Opportunity), comme le type de chart TPO de LuxAlgo (Vela) : un profil par session (jour UTC ;
//! semaine si la bougie fait un jour ou plus), une lettre par période (30 min par défaut : A = la 1re demi-heure, B la
//! 2e… puis a–z, puis 0–9) à chaque rangée de prix touchée. POC (rangée la plus remplie), zone de valeur (70 % autour
//! du POC), initial balance (60 min), single prints (rangées à une seule lettre, hors des bouts du profil).
//! Calculé avec les bougies seulement : ça marche sur tous les marchés.

use crate::indicators::flow::nice;
use argos_core::Bar;

/// Lettres des périodes, dans l'ordre de LuxAlgo.
pub const LETTERS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

/// Réglages (défauts de LuxAlgo).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TpoSet {
    /// Durée d'une période (secondes) : 30 min.
    pub block: i64,
    /// Zone de valeur (%) : 70.
    pub va: f64,
    /// Initial balance (minutes) : 60.
    pub ib: i64,
}

impl Default for TpoSet {
    fn default() -> Self {
        Self { block: 1800, va: 70.0, ib: 60 }
    }
}

/// Le profil d'une session.
#[derive(Debug, Clone, PartialEq)]
pub struct Session {
    /// Première et dernière bougie de la session.
    pub first: usize,
    pub last: usize,
    /// Hauteur d'une rangée (prix) ; la rangée `k` va de `(low_row + k) × step` à `+ step`.
    pub step: f64,
    pub low_row: i64,
    /// Périodes (index dans `LETTERS`) de chaque rangée, de bas en haut.
    pub rows: Vec<Vec<u8>>,
    /// Rangées du POC, du haut et du bas de la zone de valeur.
    pub poc: usize,
    pub vah: usize,
    pub val: usize,
    /// Initial balance : (bas, haut).
    pub ib: (f64, f64),
    pub open: f64,
    pub close: f64,
    /// Rangées « single print ».
    pub singles: Vec<usize>,
    /// Nombre de périodes de la session.
    pub periods: u8,
}

impl Session {
    /// Prix du bas de la rangée `k`.
    pub fn price(&self, k: usize) -> f64 {
        (self.low_row + k as i64) as f64 * self.step
    }
}

/// Profils TPO de toutes les sessions ; `tf` = durée d'une bougie (secondes).
pub fn compute(bars: &[Bar], tf: i64, set: TpoSet) -> Vec<Session> {
    if bars.is_empty() {
        return vec![];
    }
    // session : le jour UTC ; avec des bougies d'un jour ou plus, la semaine (lundi) et une lettre par jour
    let (len, block) = if tf >= 86_400 {
        (7 * 86_400, 86_400.max(tf))
    } else {
        (86_400, set.block.max(tf))
    };
    let start = |t: i64| {
        if len == 86_400 {
            t.div_euclid(len) * len
        } else {
            (t - 4 * 86_400).div_euclid(len) * len + 4 * 86_400
        }
    };
    // bornes des sessions
    let mut spans: Vec<(usize, usize, i64)> = vec![];
    for (i, b) in bars.iter().enumerate() {
        let s = start(b.time);
        match spans.last_mut() {
            Some(sp) if sp.2 == s => sp.1 = i,
            _ => spans.push((i, i, s)),
        }
    }
    // hauteur de rangée : environ 40 rangées pour une session normale (médiane des étendues)
    let mut ranges: Vec<f64> = spans
        .iter()
        .map(|&(a, z, _)| hi_lo(&bars[a..=z]).0 - hi_lo(&bars[a..=z]).1)
        .filter(|r| *r > 0.0)
        .collect();
    ranges.sort_by(f64::total_cmp);
    let step = nice(ranges.get(ranges.len() / 2).copied().unwrap_or(1.0) / 40.0);
    spans.into_iter().map(|(a, z, s0)| session(&bars[a..=z], a, s0, step, block, set)).collect()
}

fn hi_lo(bars: &[Bar]) -> (f64, f64) {
    bars.iter().fold((f64::MIN, f64::MAX), |(h, l), b| (h.max(b.high), l.min(b.low)))
}

fn session(bars: &[Bar], first: usize, s0: i64, step: f64, block: i64, set: TpoSet) -> Session {
    let (hi, lo) = hi_lo(bars);
    let low_row = (lo / step).floor() as i64;
    let n_rows = (((hi / step).floor() as i64 - low_row + 1).clamp(1, 2000)) as usize;
    let mut rows: Vec<Vec<u8>> = vec![vec![]; n_rows];
    let mut periods = 0u8;
    let (mut ib_h, mut ib_l) = (f64::MIN, f64::MAX);
    let t0 = bars[0].time;
    for b in bars {
        let p = ((b.time - s0) / block).clamp(0, LETTERS.len() as i64 - 1) as u8;
        periods = periods.max(p + 1);
        let (r0, r1) = (
            ((b.low / step).floor() as i64 - low_row).max(0) as usize,
            (((b.high / step).floor() as i64 - low_row).max(0) as usize).min(n_rows - 1),
        );
        for row in rows.iter_mut().take(r1 + 1).skip(r0) {
            if row.last() != Some(&p) {
                row.push(p);
            }
        }
        if b.time < t0 + set.ib * 60 {
            (ib_h, ib_l) = (ib_h.max(b.high), ib_l.min(b.low));
        }
    }
    // POC : la rangée la plus remplie (à égalité, la plus proche du milieu)
    let mid = n_rows as f64 / 2.0;
    let poc = (0..n_rows)
        .max_by(|&a, &b| {
            rows[a]
                .len()
                .cmp(&rows[b].len())
                .then(((b as f64 - mid).abs()).total_cmp(&(a as f64 - mid).abs()))
        })
        .unwrap_or(0);
    // zone de valeur : on ajoute le côté le plus rempli jusqu'à `va` % des lettres
    let total: usize = rows.iter().map(Vec::len).sum();
    let (mut vah, mut val, mut got) = (poc, poc, rows[poc].len());
    while (got as f64) < total as f64 * set.va / 100.0 && (vah + 1 < n_rows || val > 0) {
        let up = rows.get(vah + 1).map_or(0, Vec::len);
        let dn = if val > 0 { rows[val - 1].len() } else { 0 };
        if (up >= dn && vah + 1 < n_rows) || val == 0 {
            vah += 1;
            got += up;
        } else {
            val -= 1;
            got += dn;
        }
    }
    // single prints : une seule lettre, pas collée à un bout du profil (ce sont des queues)
    let singles = (0..n_rows)
        .filter(|&k| rows[k].len() == 1)
        .filter(|&k| (k..n_rows).any(|j| rows[j].len() > 1) && (0..=k).any(|j| rows[j].len() > 1))
        .collect();
    Session {
        first,
        last: first + bars.len() - 1,
        step,
        low_row,
        rows,
        poc,
        vah,
        val,
        ib: (ib_l, ib_h),
        open: bars[0].open,
        close: bars[bars.len() - 1].close,
        singles,
        periods,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar(time: i64, low: f64, high: f64) -> Bar {
        Bar {
            time,
            open: low,
            high,
            low,
            close: high,
            volume: 1.0,
        }
    }

    #[test]
    fn lettres_poc_zone_de_valeur() {
        // une journée de bougies de 30 min : A et B en bas, C, D, E au milieu, F en haut
        let mut bars = vec![bar(0, 100.0, 101.9), bar(1800, 100.0, 101.9)];
        bars.extend((2..5).map(|k| bar(k * 1800, 102.0, 103.9)));
        bars.push(bar(5 * 1800, 104.0, 104.9));
        bars.push(bar(86_400, 100.0, 105.0)); // le lendemain : une autre session
        let s = compute(&bars, 1800, TpoSet::default());
        assert_eq!(s.len(), 2, "une session par jour");
        let d = &s[0];
        assert_eq!((d.first, d.last, d.periods), (0, 5, 6));
        let row_of = |p: f64| ((p / d.step).floor() as i64 - d.low_row) as usize;
        assert_eq!(d.rows[row_of(100.5)], vec![0, 1], "A B en bas");
        assert_eq!(d.rows[row_of(103.0)], vec![2, 3, 4], "C D E au milieu");
        assert_eq!(d.price(d.poc).floor(), 102.0, "POC au milieu (C D E)");
        assert!(d.price(d.val) <= 102.0 && d.price(d.vah) >= 103.0);
        assert_eq!(d.ib, (100.0, 101.9), "initial balance = la 1re heure (A, B)");
        assert!(d.singles.is_empty(), "le F du haut est une queue, pas un single print");
        assert_eq!(LETTERS[d.rows[row_of(104.5)][0] as usize], b'F');
    }
}
