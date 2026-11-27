//! 🧪 Stratégies du backtest : un signal par bougie (+1 achat, −1 vente, 0 rien) **à la clôture** de la bougie,
//! calculé seulement avec les bougies déjà fermées (aucun regard vers le futur), pour le moteur
//! `argos_core::backtest` (entrée à l'ouverture de la bougie suivante, stop et objectif vérifiés à chaque bougie).

use crate::indicators::vector_zone;
use crate::ta::{ema, rsi};
use argos_core::Bar;

/// Les stratégies offertes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Strategy {
    /// Croisement EMA 9 / 21.
    #[default]
    EmaCross,
    /// RSI 14 qui remonte au-dessus de 30 (achat) ou redescend sous 70 (vente).
    Rsi,
    /// MACD (12, 26, 9) qui croise sa ligne de signal.
    Macd,
    /// Argos Vector Zone Pro : signaux CONF (cassure de trendline + vector + rebond sur l'EMA).
    VectorZone,
}

impl Strategy {
    pub const ALL: [Strategy; 4] = [Strategy::EmaCross, Strategy::Rsi, Strategy::Macd, Strategy::VectorZone];

    pub fn name(self) -> &'static str {
        match self {
            Strategy::EmaCross => "Croisement EMA 9 / 21",
            Strategy::Rsi => "RSI 14 (sortie de 30 / 70)",
            Strategy::Macd => "MACD 12 / 26 / 9",
            Strategy::VectorZone => "Vector Zone Pro (CONF)",
        }
    }
}

fn cross(a: &[f64], b: &[f64], i: usize) -> i8 {
    if i == 0 || ![a[i - 1], b[i - 1], a[i], b[i]].iter().all(|v| v.is_finite()) {
        return 0;
    }
    if a[i - 1] <= b[i - 1] && a[i] > b[i] {
        1
    } else if a[i - 1] >= b[i - 1] && a[i] < b[i] {
        -1
    } else {
        0
    }
}

/// Signal de chaque bougie (même longueur que `bars`). Chaque valeur ne dépend que des bougies jusqu'à elle.
pub fn signals(s: Strategy, bars: &[Bar], tf_s: i64) -> Vec<i8> {
    let n = bars.len();
    let close: Vec<f64> = bars.iter().map(|b| b.close).collect();
    match s {
        Strategy::EmaCross => {
            let (f, sl) = (ema(&close, 9), ema(&close, 21));
            (0..n).map(|i| cross(&f, &sl, i)).collect()
        }
        Strategy::Rsi => {
            let r = rsi(&close, 14);
            let (lo, hi) = (vec![30.0; n], vec![70.0; n]);
            (0..n)
                .map(|i| match (cross(&r, &lo, i), cross(&r, &hi, i)) {
                    (1, _) => 1,
                    (_, -1) => -1,
                    _ => 0,
                })
                .collect()
        }
        Strategy::Macd => {
            let (f, sl) = (ema(&close, 12), ema(&close, 26));
            let macd: Vec<f64> = f.iter().zip(&sl).map(|(a, b)| a - b).collect();
            let sig = ema(&macd, 9);
            (0..n).map(|i| cross(&macd, &sig, i)).collect()
        }
        Strategy::VectorZone => {
            let mut out = vec![0i8; n];
            if n > 60 {
                let now = bars[n - 1].time + tf_s;
                let z = vector_zone::compute(bars, tf_s, now, &crate::inputs::defaults(&vector_zone::PARAMS));
                for &(i, sig) in &z.signals {
                    if i < n {
                        match sig {
                            vector_zone::Signal::ConfLong => out[i] = 1,
                            vector_zone::Signal::ConfShort => out[i] = -1,
                            _ => {}
                        }
                    }
                }
            }
            out
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bars(closes: impl Iterator<Item = f64>) -> Vec<Bar> {
        closes
            .enumerate()
            .map(|(i, c)| Bar {
                time: i as i64 * 3600,
                open: c,
                high: c + 1.0,
                low: c - 1.0,
                close: c,
                volume: 1.0,
            })
            .collect()
    }

    #[test]
    fn croisement_ema_achat_puis_vente() {
        // baisse, hausse, baisse : un achat puis une vente
        let b = bars(
            (0..60)
                .map(|i| 100.0 - i as f64)
                .chain((0..60).map(|i| 40.0 + 2.0 * i as f64))
                .chain((0..60).map(|i| 160.0 - 2.0 * i as f64)),
        );
        let s = signals(Strategy::EmaCross, &b, 3600);
        assert_eq!(s.len(), b.len());
        let buys: Vec<usize> = (0..s.len()).filter(|&i| s[i] == 1).collect();
        let sells: Vec<usize> = (0..s.len()).filter(|&i| s[i] == -1).collect();
        assert_eq!((buys.len(), sells.len()), (1, 1), "{buys:?} {sells:?}");
        assert!(buys[0] > 60 && sells[0] > 120 && buys[0] < sells[0]);
    }

    #[test]
    fn pas_de_regard_vers_le_futur() {
        let b = bars((0..200).map(|i| 100.0 + (i as f64 * 0.3).sin() * 10.0));
        for st in Strategy::ALL {
            let all = signals(st, &b, 3600);
            let part = signals(st, &b[..150], 3600);
            assert_eq!(
                &all[..140],
                &part[..140],
                "{st:?} : les signaux passés ne changent pas avec de nouvelles bougies"
            );
        }
    }
}
