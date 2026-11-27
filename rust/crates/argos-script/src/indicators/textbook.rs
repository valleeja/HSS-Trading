//! 📚 Indicateurs classiques du domaine public, écrits à partir de leur définition des manuels (réglages usuels) :
//! RSI stochastique, Ichimoku, canal de Keltner, canal de Donchian, SAR parabolique, DMI / ADX, CCI, Williams %R,
//! OBV, Supertrend, Aroon, MFI. Calcul pur : bougies → séries (`NaN` = pas encore de valeur).

use crate::ta::{atr, ema, highest, lowest, rma, rsi, sma};
use argos_core::Bar;

fn closes(b: &[Bar]) -> Vec<f64> {
    b.iter().map(|x| x.close).collect()
}

fn highs(b: &[Bar]) -> Vec<f64> {
    b.iter().map(|x| x.high).collect()
}

fn lows(b: &[Bar]) -> Vec<f64> {
    b.iter().map(|x| x.low).collect()
}

/// RSI stochastique (RSI 14, stochastique 14, %K lissé 3, %D 3) : (%K, %D) de 0 à 100.
pub fn stoch_rsi(b: &[Bar]) -> (Vec<f64>, Vec<f64>) {
    stoch_rsi_n(b, 14, 14, 3, 3)
}

/// RSI stochastique (longueur RSI, longueur stochastique, lissage %K, lissage %D).
pub fn stoch_rsi_n(b: &[Bar], rl: usize, sl: usize, kl: usize, dl: usize) -> (Vec<f64>, Vec<f64>) {
    let r = rsi(&closes(b), rl.max(1));
    let (hi, lo) = (highest(&r, sl.max(1)), lowest(&r, sl.max(1)));
    let raw: Vec<f64> = (0..r.len())
        .map(|i| {
            let span = hi[i] - lo[i];
            if !(r[i].is_finite() && span.is_finite()) {
                f64::NAN
            } else if span == 0.0 {
                0.0
            } else {
                (r[i] - lo[i]) / span * 100.0
            }
        })
        .collect();
    let k = sma(&raw, kl.max(1));
    let d = sma(&k, dl.max(1));
    (k, d)
}

/// Ichimoku (9, 26, 52, 26) : (tenkan, kijun, senkou A, senkou B, chikou). Comme TradingView (`offset = displacement - 1`),
/// le décalage 26 avance les deux senkou de 25 bougies (valeur calculée 25 bougies plus tôt) et le chikou est la
/// clôture 25 bougies plus tard (B-029 : 26 avant, nuage et Chikou une bougie à côté de TradingView).
pub struct Ichimoku {
    pub tenkan: Vec<f64>,
    pub kijun: Vec<f64>,
    pub span_a: Vec<f64>,
    pub span_b: Vec<f64>,
    pub chikou: Vec<f64>,
    /// The two senkou after the last bar: TradingView draws the cloud `displacement - 1` bars ahead (B-045). The k-th
    /// value is the bar `len + k` (empty with no bar, or a displacement of 1 or less).
    pub ahead_a: Vec<f64>,
    pub ahead_b: Vec<f64>,
}

pub fn ichimoku(b: &[Bar]) -> Ichimoku {
    ichimoku_n(b, 9, 26, 52, 26)
}

/// Les bougies dont TradingView déplace le nuage et le Chikou pour le réglage « décalage » `displacement`.
pub fn ichimoku_offset(displacement: usize) -> usize {
    displacement.saturating_sub(1)
}

/// Ichimoku avec ses longueurs (tenkan, kijun, senkou B) et son réglage de décalage (26 : 25 bougies, comme TradingView).
pub fn ichimoku_n(b: &[Bar], t: usize, k: usize, s: usize, displacement: usize) -> Ichimoku {
    let dec = ichimoku_offset(displacement);
    let mid = |n: usize| -> Vec<f64> {
        let (h, l) = (highest(&highs(b), n.max(1)), lowest(&lows(b), n.max(1)));
        h.iter().zip(&l).map(|(a, c)| (a + c) / 2.0).collect()
    };
    let (tenkan, kijun, b52) = (mid(t), mid(k), mid(s));
    let n = b.len();
    let shift = |v: &[f64]| (0..n).map(|i| if i >= dec { v[i - dec] } else { f64::NAN }).collect::<Vec<f64>>();
    let a: Vec<f64> = tenkan.iter().zip(&kijun).map(|(t, k)| (t + k) / 2.0).collect();
    let c = closes(b);
    let ahead = |v: &[f64]| (n..n + dec).map(|i| if i >= dec { v[i - dec] } else { f64::NAN }).collect::<Vec<f64>>();
    Ichimoku {
        ahead_a: if n == 0 { vec![] } else { ahead(&a) },
        ahead_b: if n == 0 { vec![] } else { ahead(&b52) },
        span_a: shift(&a),
        span_b: shift(&b52),
        chikou: (0..n).map(|i| c.get(i + dec).copied().unwrap_or(f64::NAN)).collect(),
        tenkan,
        kijun,
    }
}

/// Canal de Keltner (EMA 20 de la clôture ± 2 × ATR 10) : (haut, milieu, bas).
pub fn keltner(b: &[Bar]) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    keltner_n(b, 20, 10, 2.0)
}

/// Canal de Keltner (EMA `len` ± `mult` × ATR `al`).
pub fn keltner_n(b: &[Bar], len: usize, al: usize, mult: f64) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let mid = ema(&closes(b), len.max(1));
    let a = atr(b, al.max(1));
    let up = mid.iter().zip(&a).map(|(m, x)| m + mult * x).collect();
    let dn = mid.iter().zip(&a).map(|(m, x)| m - mult * x).collect();
    (up, mid, dn)
}

/// Canal de Donchian (20) : (plus haut, milieu, plus bas) des 20 dernières bougies.
pub fn donchian(b: &[Bar]) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    donchian_n(b, 20)
}

/// Canal de Donchian de `len` bougies.
pub fn donchian_n(b: &[Bar], len: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let (h, l) = (highest(&highs(b), len.max(1)), lowest(&lows(b), len.max(1)));
    let m = h.iter().zip(&l).map(|(a, c)| (a + c) / 2.0).collect();
    (h, m, l)
}

/// SAR parabolique (départ 0,02, pas 0,02, maximum 0,2), comme `ta.sar` de TradingView.
pub fn psar(b: &[Bar]) -> Vec<f64> {
    psar_n(b, 0.02, 0.02, 0.2)
}

/// SAR parabolique (départ, pas, maximum), pas à pas comme `ta.sar` de TradingView (B-091). À la 2e bougie, la
/// tendance suit la clôture (haussière si elle monte) et le SAR part de l'autre côté de la 1re bougie. À chaque
/// bougie le SAR avance vers l'extrême ; le retournement se lit sur ce SAR AVANT qu'il soit borné par les deux bougies
/// d'avant (Wilder et TA-Lib bornent d'abord, d'où l'écart) ; au retournement il part de l'extrême, ou du haut / bas
/// de la bougie s'il le dépasse, puis il est borné lui aussi. NaN à la 1re bougie.
pub fn psar_n(b: &[Bar], start: f64, step: f64, max: f64) -> Vec<f64> {
    let n = b.len();
    let mut out = vec![f64::NAN; n];
    let (mut below, mut ext, mut sar, mut af) = (true, f64::NAN, f64::NAN, start);
    for i in 1..n {
        let mut first = i == 1;
        if first {
            below = b[1].close > b[0].close;
            (ext, sar) = if below { (b[1].high, b[0].low) } else { (b[1].low, b[0].high) };
            af = start;
        }
        sar += af * (ext - sar);
        if below && sar > b[i].low {
            (first, below, sar, ext, af) = (true, false, b[i].high.max(ext), b[i].low, start);
        } else if !below && sar < b[i].high {
            (first, below, sar, ext, af) = (true, true, b[i].low.min(ext), b[i].high, start);
        }
        if !first {
            if below && b[i].high > ext {
                (ext, af) = (b[i].high, (af + step).min(max));
            } else if !below && b[i].low < ext {
                (ext, af) = (b[i].low, (af + step).min(max));
            }
        }
        sar = if below { sar.min(b[i - 1].low) } else { sar.max(b[i - 1].high) };
        if i > 1 {
            sar = if below { sar.min(b[i - 2].low) } else { sar.max(b[i - 2].high) };
        }
        out[i] = sar;
    }
    out
}

/// DMI / ADX de Wilder (14) : (+DI, −DI, ADX).
pub fn dmi(b: &[Bar]) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    dmi_n(b, 14, 14)
}

/// DMI / ADX (longueur DI, lissage ADX).
pub fn dmi_n(b: &[Bar], len: usize, al: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let (len, al) = (len.max(1), al.max(1));
    let n = b.len();
    let (mut pdm, mut mdm, mut tr) = (vec![f64::NAN; n], vec![f64::NAN; n], vec![f64::NAN; n]);
    for i in 1..n {
        let (up, down) = (b[i].high - b[i - 1].high, b[i - 1].low - b[i].low);
        pdm[i] = if up > down && up > 0.0 { up } else { 0.0 };
        mdm[i] = if down > up && down > 0.0 { down } else { 0.0 };
        let pc = b[i - 1].close;
        tr[i] = (b[i].high - b[i].low).max((b[i].high - pc).abs()).max((b[i].low - pc).abs());
    }
    let (sp, sm, st) = (rma(&pdm, len), rma(&mdm, len), rma(&tr, len));
    let di = |s: &[f64]| {
        s.iter()
            .zip(&st)
            .map(|(a, t)| if *t > 0.0 { 100.0 * a / t } else { f64::NAN })
            .collect::<Vec<f64>>()
    };
    let (plus, minus) = (di(&sp), di(&sm));
    let dx: Vec<f64> = plus
        .iter()
        .zip(&minus)
        .map(|(p, m)| if p + m > 0.0 { 100.0 * (p - m).abs() / (p + m) } else { f64::NAN })
        .collect();
    let adx = rma(&dx, al);
    (plus, minus, adx)
}

/// CCI (20) : (prix typique − sa moyenne) ÷ (0,015 × écart moyen absolu).
pub fn cci(b: &[Bar]) -> Vec<f64> {
    cci_n(b, 20)
}

/// CCI de longueur `len`.
pub fn cci_n(b: &[Bar], len: usize) -> Vec<f64> {
    let len = len.max(1);
    let tp: Vec<f64> = b.iter().map(|x| (x.high + x.low + x.close) / 3.0).collect();
    let m = sma(&tp, len);
    (0..tp.len())
        .map(|i| {
            if !m[i].is_finite() {
                return f64::NAN;
            }
            let md = tp[i + 1 - len..=i].iter().map(|v| (v - m[i]).abs()).sum::<f64>() / len as f64;
            if md > 0.0 {
                (tp[i] - m[i]) / (0.015 * md)
            } else {
                0.0
            }
        })
        .collect()
}

/// Williams %R (14) : de −100 (au plus bas) à 0 (au plus haut).
pub fn williams_r(b: &[Bar]) -> Vec<f64> {
    williams_r_n(b, 14)
}

/// Williams %R de longueur `len`.
pub fn williams_r_n(b: &[Bar], len: usize) -> Vec<f64> {
    let (h, l) = (highest(&highs(b), len.max(1)), lowest(&lows(b), len.max(1)));
    (0..b.len())
        .map(|i| {
            let span = h[i] - l[i];
            if !span.is_finite() {
                f64::NAN
            } else if span == 0.0 {
                -50.0
            } else {
                (h[i] - b[i].close) / span * -100.0
            }
        })
        .collect()
}

/// OBV : volume ajouté quand la clôture monte, retiré quand elle baisse (départ à 0).
pub fn obv(b: &[Bar]) -> Vec<f64> {
    let mut acc = 0.0;
    (0..b.len())
        .map(|i| {
            if i > 0 {
                if b[i].close > b[i - 1].close {
                    acc += b[i].volume;
                } else if b[i].close < b[i - 1].close {
                    acc -= b[i].volume;
                }
            }
            acc
        })
        .collect()
}

/// Supertrend (ATR 10, × 3) : (ligne, tendance haussière ?). La bande suit le prix et ne recule pas tant que la
/// tendance tient ; elle bascule quand la clôture la traverse.
pub fn supertrend(b: &[Bar]) -> (Vec<f64>, Vec<bool>) {
    supertrend_n(b, 10, 3.0)
}

/// Supertrend (ATR `al`, × `mult`).
pub fn supertrend_n(b: &[Bar], al: usize, mult: f64) -> (Vec<f64>, Vec<bool>) {
    let a = atr(b, al.max(1));
    let n = b.len();
    let (mut line, mut up) = (vec![f64::NAN; n], vec![true; n]);
    let (mut fu, mut fl) = (f64::NAN, f64::NAN);
    let mut trend_up = true;
    for i in 0..n {
        if !a[i].is_finite() {
            continue;
        }
        let mid = (b[i].high + b[i].low) / 2.0;
        let (bu, bl) = (mid + mult * a[i], mid - mult * a[i]);
        let prev = if i > 0 { b[i - 1].close } else { b[i].close };
        fu = if fu.is_nan() || bu < fu || prev > fu { bu } else { fu };
        fl = if fl.is_nan() || bl > fl || prev < fl { bl } else { fl };
        if trend_up && b[i].close < fl {
            trend_up = false;
        } else if !trend_up && b[i].close > fu {
            trend_up = true;
        }
        line[i] = if trend_up { fl } else { fu };
        up[i] = trend_up;
    }
    (line, up)
}

/// Aroon (25) : (Aroon haut, Aroon bas) de 0 à 100 — 100 × (25 − bougies depuis le plus haut / bas) ÷ 25.
pub fn aroon(b: &[Bar]) -> (Vec<f64>, Vec<f64>) {
    aroon_n(b, 25)
}

/// Aroon de longueur `n`.
pub fn aroon_n(b: &[Bar], n: usize) -> (Vec<f64>, Vec<f64>) {
    #[allow(non_snake_case)]
    let N = n.max(1);
    let mut up = vec![f64::NAN; b.len()];
    let mut dn = vec![f64::NAN; b.len()];
    for i in N..b.len() {
        let w = &b[i - N..=i];
        let hi = w.iter().enumerate().fold(0, |k, (j, x)| if x.high >= w[k].high { j } else { k });
        let lo = w.iter().enumerate().fold(0, |k, (j, x)| if x.low <= w[k].low { j } else { k });
        up[i] = 100.0 * hi as f64 / N as f64;
        dn[i] = 100.0 * lo as f64 / N as f64;
    }
    (up, dn)
}

/// MFI (14) : RSI pondéré par le volume, sur le prix typique.
pub fn mfi(b: &[Bar]) -> Vec<f64> {
    mfi_n(b, 14)
}

/// MFI de longueur `len`.
pub fn mfi_n(b: &[Bar], len: usize) -> Vec<f64> {
    let len = len.max(1);
    let tp: Vec<f64> = b.iter().map(|x| (x.high + x.low + x.close) / 3.0).collect();
    (0..b.len())
        .map(|i| {
            if i < len {
                return f64::NAN;
            }
            let (mut pos, mut neg) = (0.0, 0.0);
            for k in i + 1 - len..=i {
                let flow = tp[k] * b[k].volume;
                if tp[k] > tp[k - 1] {
                    pos += flow;
                } else if tp[k] < tp[k - 1] {
                    neg += flow;
                }
            }
            if neg == 0.0 {
                100.0
            } else {
                100.0 - 100.0 / (1.0 + pos / neg)
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar(o: f64, h: f64, l: f64, c: f64, v: f64) -> Bar {
        Bar {
            time: 0,
            open: o,
            high: h,
            low: l,
            close: c,
            volume: v,
        }
    }

    #[test]
    fn cas_simples() {
        // montée régulière : OBV monte, Williams %R au plus haut, Aroon haut = 100, SAR sous le prix, Supertrend haussier
        let b: Vec<Bar> = (0..60).map(|i| bar(i as f64, i as f64 + 1.0, i as f64 - 1.0, i as f64 + 0.5, 10.0)).collect();
        assert_eq!(obv(&b)[59], 590.0);
        assert!((williams_r(&b)[59] - (-100.0 * 0.5 / 15.0)).abs() < 1e-9);
        assert_eq!(aroon(&b).0[59], 100.0);
        assert!(psar(&b)[59] < b[59].low);
        let (st, up) = supertrend(&b);
        assert!(up[59] && st[59] < b[59].close);
        let (p, m, adx) = dmi(&b);
        assert!(p[59] > m[59] && adx[59] > 50.0, "tendance forte : ADX élevé");
        assert_eq!(mfi(&b)[59], 100.0, "aucun flux négatif");
        let (k, _) = stoch_rsi(&b);
        assert_eq!(k[59], 0.0, "RSI plat à 100 : stochastique à 0 (étendue nulle)");
        let i = ichimoku(&b);
        assert!(i.span_a[25].is_nan() && i.span_a[59].is_finite() && i.chikou[59].is_nan());
        let (dh, dm, dl) = donchian(&b);
        assert_eq!((dh[59], dl[59]), (60.0, 39.0));
        assert_eq!(dm[59], 49.5);
        assert!(keltner(&b).0[59] > keltner(&b).2[59]);
        assert!(cci(&b)[59] > 100.0);
    }
}
