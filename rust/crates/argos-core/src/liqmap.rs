//! 💥 Carte des liquidations : ESTIMATION d'Argos à partir de données publiques (bougies + intérêt ouvert).
//! Quand l'intérêt ouvert monte de ΔOI pendant une bougie, ces nouvelles positions sont supposées ouvertes au prix de
//! clôture, partagées entre longs et shorts, et entre plusieurs leviers. Chacune a un prix de liquidation approché :
//! long = entrée × (1 − 1/levier + marge d'entretien), short = entrée × (1 + 1/levier − marge d'entretien). Les montants
//! s'additionnent par case de prix ; une case est vidée quand une bougie suivante la traverse (positions liquidées).
//! Quand l'intérêt ouvert baisse, toutes les cases encore ouvertes diminuent dans la même proportion (des positions se
//! ferment, on ne sait pas lesquelles). Ce n'est pas une donnée des exchanges : c'est un modèle.

use std::collections::HashMap;

/// Une bougie avec l'intérêt ouvert (en $) à sa clôture.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OiBar {
    pub time: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub oi: f64,
}

/// Réglages du modèle.
#[derive(Debug, Clone, PartialEq)]
pub struct Params {
    /// (levier, poids) ; les poids sont normalisés.
    pub levers: Vec<(f64, f64)>,
    /// Marge d'entretien (fraction, 0,005 = 0,5 %).
    pub mm: f64,
    /// Taille d'une case de prix.
    pub bucket: f64,
    /// Part des longs dans les nouvelles positions ; `None` : selon la bougie (60 % du côté où elle va).
    pub long_share: Option<f64>,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            levers: vec![(10.0, 30.0), (25.0, 30.0), (50.0, 25.0), (100.0, 15.0)],
            mm: 0.005,
            bucket: 50.0,
            long_share: Some(0.5),
        }
    }
}

/// Une bande : des liquidations estimées à `price`, nées à `born`, liquidées à `end` (`None` : encore là).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Band {
    pub price: f64,
    pub long: bool,
    pub born: i64,
    pub end: Option<i64>,
    /// Montant estimé (en $) : au moment de la liquidation, ou maintenant.
    pub usd: f64,
}

/// Prix de liquidation approché d'une position ouverte à `entry` avec ce levier.
pub fn liq_price(entry: f64, lever: f64, mm: f64, long: bool) -> f64 {
    if long {
        entry * (1.0 - 1.0 / lever + mm)
    } else {
        entry * (1.0 + 1.0 / lever - mm)
    }
}

/// Les bandes estimées sur ces bougies (dans l'ordre du temps).
pub fn compute(bars: &[OiBar], p: &Params) -> Vec<Band> {
    let total: f64 = p.levers.iter().map(|l| l.1.max(0.0)).sum();
    if bars.len() < 2 || total <= 0.0 || p.bucket <= 0.0 {
        return vec![];
    }
    let key = |price: f64| (price / p.bucket).round() as i64;
    // case ouverte : (case, long) → (naissance, montant)
    let mut open: HashMap<(i64, bool), (i64, f64)> = HashMap::new();
    let mut done: Vec<Band> = vec![];
    for w in bars.windows(2) {
        let (prev, b) = (w[0], w[1]);
        // 1. la bougie traverse des cases : liquidées
        let (lo, hi) = (key(b.low), key(b.high));
        open.retain(|&(k, long), &mut (born, usd)| {
            if (lo..=hi).contains(&k) {
                done.push(Band {
                    price: k as f64 * p.bucket,
                    long,
                    born,
                    end: Some(b.time),
                    usd,
                });
                false
            } else {
                true
            }
        });
        // 2. l'intérêt ouvert change
        let d = b.oi - prev.oi;
        if d < 0.0 && prev.oi > 0.0 {
            let f = (b.oi / prev.oi).clamp(0.0, 1.0);
            for v in open.values_mut() {
                v.1 *= f;
            }
        } else if d > 0.0 {
            let share = p.long_share.unwrap_or(if b.close >= b.open { 0.6 } else { 0.4 }).clamp(0.0, 1.0);
            for &(lever, wgt) in &p.levers {
                if lever <= 1.0 || wgt <= 0.0 {
                    continue;
                }
                let part = d * wgt / total;
                for (long, s) in [(true, share), (false, 1.0 - share)] {
                    if s <= 0.0 {
                        continue;
                    }
                    let k = key(liq_price(b.close, lever, p.mm, long));
                    let e = open.entry((k, long)).or_insert((b.time, 0.0));
                    e.1 += part * s;
                }
            }
        }
    }
    done.extend(open.into_iter().map(|((k, long), (born, usd))| Band {
        price: k as f64 * p.bucket,
        long,
        born,
        end: None,
        usd,
    }));
    done.sort_by(|a, b| a.born.cmp(&b.born).then(a.price.total_cmp(&b.price)));
    done
}

/// Bandes encore là, cumulées par prix : (prix, longs $, shorts $), du plus bas au plus haut.
pub fn profile(bands: &[Band]) -> Vec<(f64, f64, f64)> {
    let mut m: std::collections::BTreeMap<i64, (f64, f64, f64)> = std::collections::BTreeMap::new();
    for b in bands.iter().filter(|b| b.end.is_none()) {
        let e = m.entry((b.price * 1e6).round() as i64).or_insert((b.price, 0.0, 0.0));
        if b.long {
            e.1 += b.usd;
        } else {
            e.2 += b.usd;
        }
    }
    m.into_values().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar(time: i64, low: f64, high: f64, close: f64, oi: f64) -> OiBar {
        OiBar {
            time,
            open: close,
            high,
            low,
            close,
            oi,
        }
    }

    #[test]
    fn prix_de_liquidation_par_levier() {
        for (lev, long, want) in [
            (10.0, true, 72_400.0),
            (25.0, true, 77_200.0),
            (50.0, true, 78_800.0),
            (100.0, true, 79_600.0),
            (10.0, false, 87_600.0),
            (25.0, false, 82_800.0),
            (50.0, false, 81_200.0),
            (100.0, false, 80_400.0),
        ] {
            let got = liq_price(80_000.0, lev, 0.005, long);
            assert!((got - want).abs() < 1e-6, "{lev}x {} : {got}", if long { "long" } else { "short" });
        }
    }

    #[test]
    fn accumulation_dans_les_cases() {
        // une seule bougie de hausse d'OI (+1 000 000 $), 50/50, un seul levier 10x
        let p = Params {
            levers: vec![(10.0, 1.0)],
            mm: 0.0,
            bucket: 100.0,
            long_share: Some(0.5),
        };
        let bars = [bar(0, 999.0, 1_001.0, 1_000.0, 0.0), bar(300, 999.0, 1_001.0, 1_000.0, 1_000_000.0)];
        let b = compute(&bars, &p);
        assert_eq!(b.len(), 2);
        let long = b.iter().find(|x| x.long).unwrap();
        let short = b.iter().find(|x| !x.long).unwrap();
        assert_eq!((long.price, long.usd, long.end), (900.0, 500_000.0, None));
        assert_eq!((short.price, short.usd), (1_100.0, 500_000.0));
        assert_eq!(profile(&b), vec![(900.0, 500_000.0, 0.0), (1_100.0, 0.0, 500_000.0)]);
        // deux hausses au même prix : la même case grossit, née à la première
        let bars = [
            bar(0, 999.0, 1_001.0, 1_000.0, 0.0),
            bar(300, 999.0, 1_001.0, 1_000.0, 1_000.0),
            bar(600, 999.0, 1_001.0, 1_000.0, 3_000.0),
        ];
        let long = compute(&bars, &p).into_iter().find(|x| x.long).unwrap();
        assert_eq!((long.born, long.usd), (300, 1_500.0));
    }

    #[test]
    fn case_videe_quand_le_prix_la_traverse_et_oi_qui_baisse() {
        let p = Params {
            levers: vec![(10.0, 1.0)],
            mm: 0.0,
            bucket: 100.0,
            long_share: Some(0.5),
        };
        let bars = [
            bar(0, 999.0, 1_001.0, 1_000.0, 0.0),
            bar(300, 999.0, 1_001.0, 1_000.0, 1_000.0),
            // l'OI baisse de moitié : les cases restantes aussi
            bar(600, 999.0, 1_001.0, 1_000.0, 500.0),
            // le prix descend jusqu'à 880 : la case des longs (900) est liquidée
            bar(900, 880.0, 1_000.0, 890.0, 500.0),
        ];
        let b = compute(&bars, &p);
        let long = b.iter().find(|x| x.long).unwrap();
        assert_eq!((long.end, long.usd), (Some(900), 250.0), "liquidée à la 4e bougie, après la baisse de l'OI");
        let short = b.iter().find(|x| !x.long).unwrap();
        assert_eq!((short.end, short.usd), (None, 250.0), "les shorts à 1 100 sont encore là");
        assert_eq!(profile(&b), vec![(1_100.0, 0.0, 250.0)]);
    }

    #[test]
    fn serie_connue_et_plusieurs_bourses() {
        // poids par défaut, ΔOI de 10 M$ à 80 000 : longs 50x à 78 800 = 10 M × 25 % × 50 % = 1,25 M$
        let p = Params {
            bucket: 100.0,
            ..Default::default()
        };
        let a = [bar(0, 79_990.0, 80_010.0, 80_000.0, 100e6), bar(300, 79_990.0, 80_010.0, 80_000.0, 110e6)];
        let b = compute(&a, &p);
        assert_eq!(b.len(), 8, "4 leviers × longs et shorts");
        let l50 = b.iter().find(|x| x.long && x.price == 78_800.0).unwrap();
        assert!((l50.usd - 1.25e6).abs() < 1e-3);
        // deux bourses : leurs bandes s'additionnent dans le profil
        let mut both = b.clone();
        both.extend(compute(&a, &p));
        let row = profile(&both).into_iter().find(|r| r.0 == 78_800.0).unwrap();
        assert!((row.1 - 2.5e6).abs() < 1e-3);
        // sens de la bougie : 60 % du côté où elle va
        let up = [OiBar { open: 79_000.0, ..a[0] }, OiBar { open: 79_000.0, ..a[1] }];
        let b = compute(&up, &Params { long_share: None, ..p.clone() });
        let longs: f64 = b.iter().filter(|x| x.long).map(|x| x.usd).sum();
        assert!((longs - 6e6).abs() < 1e-3);
        assert!(compute(&a[..1], &p).is_empty(), "une seule bougie : rien");
    }
}
