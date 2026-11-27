//! Fonctions d'analyse technique de base, avec la même logique que Pine Script (TradingView).
//! Une série = `Vec<f64>`, alignée sur les bougies. `NaN` = pas encore de valeur (comme `na` en Pine).

use argos_core::Bar;

/// Moyenne simple sur `len` valeurs. NaN si une valeur de la fenêtre manque (comme `ta.sma` en Pine).
pub fn sma(src: &[f64], len: usize) -> Vec<f64> {
    let mut out = vec![f64::NAN; src.len()];
    if len == 0 {
        return out;
    }
    let (mut sum, mut finite) = (0.0, 0usize);
    for i in 0..src.len() {
        if src[i].is_finite() {
            sum += src[i];
            finite += 1;
        }
        if i >= len && src[i - len].is_finite() {
            sum -= src[i - len];
            finite -= 1;
        }
        if i + 1 >= len && finite == len {
            out[i] = sum / len as f64;
        }
    }
    out
}

/// Moyenne exponentielle, démarrée par la moyenne simple des `len` premières valeurs. Équivaut à `ta.ema`.
pub fn ema(src: &[f64], len: usize) -> Vec<f64> {
    smoothed(src, len, 2.0 / (len as f64 + 1.0))
}

/// Moyenne de Wilder (alpha = 1/len), utilisée par le RSI et l'ATR. Équivaut à `ta.rma`.
pub fn rma(src: &[f64], len: usize) -> Vec<f64> {
    smoothed(src, len, 1.0 / len as f64)
}

/// Lissage démarré par la moyenne simple des `len` premières valeurs présentes : les NaN sont sautés, au début comme
/// parmi ces premières valeurs, puis gardent la dernière valeur, comme `ta.ema` / `ta.rma` d'Argos Script (`Smooth`).
/// Avant B-101, un NaN parmi les `len` premières valeurs laissait la moyenne vide sur toute la série.
fn smoothed(src: &[f64], len: usize, alpha: f64) -> Vec<f64> {
    let mut out = vec![f64::NAN; src.len()];
    if len == 0 {
        return out;
    }
    let (mut seed, mut seen) = (Vec::with_capacity(len), 0usize);
    let mut prev = f64::NAN;
    for (i, &x) in src.iter().enumerate() {
        if seen < len {
            if x.is_finite() {
                seed.push(x);
                seen += 1;
                if seen == len {
                    prev = seed.iter().sum::<f64>() / len as f64;
                    out[i] = prev;
                }
            }
            continue;
        }
        if x.is_finite() {
            prev = alpha * x + (1.0 - alpha) * prev;
        }
        out[i] = prev;
    }
    out
}

/// Moyenne pondérée (poids len, len-1, …, 1). NaN si une valeur de la fenêtre manque. Équivaut à `ta.wma`.
pub fn wma(src: &[f64], len: usize) -> Vec<f64> {
    let w = (len * (len + 1) / 2) as f64;
    (0..src.len())
        .map(|i| {
            if len == 0 || i + 1 < len {
                return f64::NAN;
            }
            (0..len).map(|j| src[i - j] * (len - j) as f64).sum::<f64>() / w
        })
        .collect()
}

/// RSI de Wilder. Équivaut à `ta.rsi`.
pub fn rsi(src: &[f64], len: usize) -> Vec<f64> {
    let n = src.len();
    let mut up = vec![0.0; n];
    let mut down = vec![0.0; n];
    for i in 1..n {
        let d = src[i] - src[i - 1];
        up[i] = d.max(0.0);
        down[i] = (-d).max(0.0);
    }
    // la première variation est à l'index 1 : on lisse à partir de là
    let (u, d) = (rma(&up[1.min(n)..], len), rma(&down[1.min(n)..], len));
    let mut out = vec![f64::NAN; n];
    for i in 0..u.len() {
        let (a, b) = (u[i], d[i]);
        out[i + 1] = if a.is_nan() || b.is_nan() {
            f64::NAN
        } else if b == 0.0 {
            100.0
        } else if a == 0.0 {
            0.0
        } else {
            100.0 - 100.0 / (1.0 + a / b)
        };
    }
    out
}

/// Stochastique : position de `src` dans son range des `len` dernières valeurs, de 0 à 100 (50 si le range est plat). Équivaut à `ta.stoch(src, src, src, len)`.
pub fn stoch(src: &[f64], len: usize) -> Vec<f64> {
    let (h, l) = (highest(src, len), lowest(src, len));
    (0..src.len())
        .map(|i| {
            let r = h[i] - l[i];
            if r.is_nan() {
                f64::NAN
            } else if r == 0.0 {
                50.0
            } else {
                (src[i] - l[i]) / r * 100.0
            }
        })
        .collect()
}

/// Pivot confirmé à la bougie `i` : sommet (`high`) ou creux à `i - right`, strictement plus haut (bas) que les `left` bougies d'avant
/// et au moins aussi haut (bas) que les `right` bougies d'après. Comme `ta.pivothigh` / `ta.pivotlow`, mais retourne l'index du pivot.
pub fn pivot(s: &[f64], i: usize, left: usize, right: usize, high: bool) -> Option<usize> {
    let j = i.checked_sub(right)?.checked_sub(left)? + left;
    if !s[j].is_finite() {
        return None;
    }
    let beats_left = |k: usize| if high { s[k] < s[j] } else { s[k] > s[j] };
    let beats_right = |k: usize| if high { s[k] <= s[j] } else { s[k] >= s[j] };
    ((j - left..j).all(beats_left) && (j + 1..=i).all(beats_right)).then_some(j)
}

/// Écart-type de population sur `len` valeurs. Équivaut à `ta.stdev`.
pub fn stdev(src: &[f64], len: usize) -> Vec<f64> {
    (0..src.len())
        .map(|i| {
            if len == 0 || i + 1 < len {
                return f64::NAN;
            }
            let w = &src[i + 1 - len..=i];
            let m = w.iter().sum::<f64>() / len as f64;
            (w.iter().map(|x| (x - m).powi(2)).sum::<f64>() / len as f64).sqrt()
        })
        .collect()
}

/// Vrai range (la 1re bougie : haut − bas).
pub fn true_range(bars: &[Bar]) -> Vec<f64> {
    (0..bars.len())
        .map(|i| {
            let b = &bars[i];
            if i == 0 {
                b.range()
            } else {
                let pc = bars[i - 1].close;
                b.range().max((b.high - pc).abs()).max((b.low - pc).abs())
            }
        })
        .collect()
}

/// ATR (moyenne de Wilder du vrai range). Équivaut à `ta.atr`.
pub fn atr(bars: &[Bar], len: usize) -> Vec<f64> {
    rma(&true_range(bars), len)
}

/// +DI, −DI et ADX de Wilder sur `len` bougies (séries alignées sur les bougies).
pub struct Dmi {
    pub plus: Vec<f64>,
    pub minus: Vec<f64>,
    pub adx: Vec<f64>,
}

/// +DI / −DI (100 × mouvement directionnel lissé / ATR) et ADX comme le site : le DX vaut 0 tant que les DI
/// n'existent pas encore (différence avec Pine limitée aux premières bougies).
pub fn dmi(bars: &[Bar], len: usize) -> Dmi {
    let n = bars.len();
    let (mut pdm, mut mdm) = (vec![f64::NAN; n], vec![f64::NAN; n]);
    for i in 1..n {
        let up = bars[i].high - bars[i - 1].high;
        let dn = bars[i - 1].low - bars[i].low;
        pdm[i] = if up > dn && up > 0.0 { up } else { 0.0 };
        mdm[i] = if dn > up && dn > 0.0 { dn } else { 0.0 };
    }
    let (t, sp, sm) = (atr(bars, len), rma(&pdm, len), rma(&mdm, len));
    let plus: Vec<f64> = (0..n).map(|i| 100.0 * sp[i] / t[i]).collect();
    let minus: Vec<f64> = (0..n).map(|i| 100.0 * sm[i] / t[i]).collect();
    let dx: Vec<f64> = (0..n)
        .map(|i| {
            let s = plus[i] + minus[i];
            if s != 0.0 && !s.is_nan() {
                100.0 * (plus[i] - minus[i]).abs() / s
            } else {
                0.0
            }
        })
        .collect();
    Dmi {
        plus,
        minus,
        adx: rma(&dx, len),
    }
}

/// ADX seul (voir [`dmi`]).
pub fn adx(bars: &[Bar], len: usize) -> Vec<f64> {
    dmi(bars, len).adx
}

/// Plus haute valeur des `len` dernières. Équivaut à `ta.highest`.
pub fn highest(src: &[f64], len: usize) -> Vec<f64> {
    window(src, len, f64::max)
}

/// Plus basse valeur des `len` dernières. Équivaut à `ta.lowest`.
pub fn lowest(src: &[f64], len: usize) -> Vec<f64> {
    window(src, len, f64::min)
}

fn window(src: &[f64], len: usize, f: fn(f64, f64) -> f64) -> Vec<f64> {
    (0..src.len())
        .map(|i| {
            if len == 0 || i + 1 < len {
                f64::NAN
            } else {
                src[i + 1 - len..=i].iter().copied().fold(src[i], f)
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9 || (a.is_nan() && b.is_nan())
    }

    #[test]
    fn moyennes() {
        let s = [1.0, 2.0, 3.0, 4.0, 5.0];
        assert!(sma(&s, 3).iter().zip([f64::NAN, f64::NAN, 2.0, 3.0, 4.0]).all(|(a, b)| close(*a, b)));
        // EMA 3 : départ à la moyenne simple 2, puis alpha 0.5
        assert!(ema(&s, 3).iter().zip([f64::NAN, f64::NAN, 2.0, 3.0, 4.0]).all(|(a, b)| close(*a, b)));
        assert!(close(rma(&s, 2)[4], 4.0625)); // 1,5 → 2,25 → 3,125 → 4,0625
    }

    #[test]
    fn rsi_hausse_continue_100_et_cas_connu() {
        let up: Vec<f64> = (0..20).map(|i| i as f64).collect();
        assert_eq!(rsi(&up, 14)[19], 100.0);
        let flat_down = [5.0, 4.0, 3.0, 2.0];
        assert_eq!(rsi(&flat_down, 2)[3], 0.0);
        // alternance +1 / -1 : RSI autour de 50
        let alt: Vec<f64> = (0..40).map(|i| if i % 2 == 0 { 10.0 } else { 11.0 }).collect();
        assert!((rsi(&alt, 14)[39] - 50.0).abs() < 5.0);
    }

    #[test]
    fn nan_au_debut_ne_bloque_pas() {
        let s = [f64::NAN, f64::NAN, 1.0, 2.0, 3.0, 4.0];
        assert!(close(sma(&s, 2)[3], 1.5));
        assert!(sma(&s, 2)[2].is_nan());
        assert!(close(ema(&s, 2)[3], 1.5));
        assert_eq!(stoch(&[1.0, 2.0, 3.0], 3)[2], 100.0);
        assert_eq!(stoch(&[2.0, 2.0, 2.0], 3)[2], 50.0);
    }

    #[test]
    fn haut_bas() {
        let s = [3.0, 1.0, 4.0, 1.0, 5.0];
        assert_eq!(highest(&s, 3)[4], 5.0);
        assert_eq!(lowest(&s, 3)[3], 1.0);
        assert!(highest(&s, 3)[1].is_nan());
    }

    /// L'ADX tel qu'il était écrit avant `dmi` (copie de référence) : les résultats doivent rester identiques au bit près.
    fn adx_avant(bars: &[Bar], len: usize) -> Vec<f64> {
        let n = bars.len();
        let (mut pdm, mut mdm) = (vec![f64::NAN; n], vec![f64::NAN; n]);
        for i in 1..n {
            let up = bars[i].high - bars[i - 1].high;
            let dn = bars[i - 1].low - bars[i].low;
            pdm[i] = if up > dn && up > 0.0 { up } else { 0.0 };
            mdm[i] = if dn > up && dn > 0.0 { dn } else { 0.0 };
        }
        let (t, sp, sm) = (atr(bars, len), rma(&pdm, len), rma(&mdm, len));
        let dx: Vec<f64> = (0..n)
            .map(|i| {
                let (a, c) = (100.0 * sp[i] / t[i], 100.0 * sm[i] / t[i]);
                let s = a + c;
                if s != 0.0 && !s.is_nan() {
                    100.0 * (a - c).abs() / s
                } else {
                    0.0
                }
            })
            .collect();
        rma(&dx, len)
    }

    #[test]
    fn dmi_expose_les_di_sans_changer_l_adx() {
        let bars: Vec<Bar> = (0..60)
            .map(|i| {
                let c = 100.0 + i as f64 + (i as f64).sin() * 3.0;
                Bar {
                    time: i,
                    open: c - 0.5,
                    high: c + 1.0,
                    low: c - 1.5,
                    close: c,
                    volume: 1.0,
                }
            })
            .collect();
        let d = dmi(&bars, 14);
        assert_eq!(
            adx_avant(&bars, 14).iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
            d.adx.iter().map(|v| v.to_bits()).collect::<Vec<_>>()
        );
        // hausse : +DI au-dessus de −DI, valeurs de 0 à 100
        assert!(d.plus[59] > d.minus[59]);
        assert!(d.plus[14..].iter().chain(&d.minus[14..]).all(|v| (0.0..=100.0).contains(v)));
        assert!(d.plus[0].is_nan());
    }
}
