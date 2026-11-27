//! Indicateurs classiques et natifs simples d'Argos v1, mêmes formules que le site :
//! RSI 14, MACD, Bandes de Bollinger, Croisement EMA 9/21, Stochastique, ATR 14, VWAP, Volume relatif, EMA multiples,
//! Argos Bollinger Bands (squeeze), et trois outils d'Argos écrits de zéro : force des cassures (oscillateur),
//! chaleur du volume (heatmap) et profil de volume. Calcul pur : bougies → séries.

use crate::inputs::Inputs;
use crate::ta::{atr, rsi, sma, stdev};
use crate::v1_inputs::{BANDS_OSC_PARAMS, BANDS_PARAMS, EMA_MULTI_PARAMS, HEATMAP_PARAMS, PROFILE_PARAMS};
use argos_core::nombres::format_number;
use argos_core::Bar;

/// EMA de v1 : départ sur la 1re valeur finie (et non sur la moyenne simple de `ta::ema`), les trous gardent la
/// dernière valeur. Gardée pour les conceptions d'Argos sans modèle TradingView (moyenne EMA d'Argos Bandes, échelle de
/// l'oscillateur des bandes) ; les indicateurs qui ont un modèle TradingView prennent `ta::ema` (B-093, B-096 à B-098).
pub fn ema(src: &[f64], len: usize) -> Vec<f64> {
    let k = 2.0 / (len as f64 + 1.0);
    let mut e = f64::NAN;
    src.iter()
        .map(|&v| {
            if v.is_finite() {
                e = if e.is_finite() { v * k + e * (1.0 - k) } else { v };
            }
            e
        })
        .collect()
}

fn closes(bars: &[Bar]) -> Vec<f64> {
    bars.iter().map(|b| b.close).collect()
}

/// RSI 14 sur la clôture.
pub fn rsi14(bars: &[Bar]) -> Vec<f64> {
    rsi_n(bars, 14)
}

/// RSI de longueur `len` sur la clôture.
pub fn rsi_n(bars: &[Bar], len: usize) -> Vec<f64> {
    rsi(&closes(bars), len.max(1))
}

/// MACD 12 / 26 / 9 : (ligne, signal, histogramme).
pub struct Macd {
    pub line: Vec<f64>,
    pub signal: Vec<f64>,
    pub hist: Vec<f64>,
}

pub fn macd(bars: &[Bar]) -> Macd {
    macd_n(bars, 12, 26, 9)
}

/// MACD avec ses longueurs (rapide, lente, signal), comme celui de TradingView (B-093) : ses moyennes sont celles de
/// `ta.ema`, démarrées par la moyenne simple de leurs premières valeurs ; la ligne est vide avant `lente` bougies, le
/// signal et l'histogramme avant `lente + signal − 1`.
pub fn macd_n(bars: &[Bar], fast: usize, slow: usize, sig: usize) -> Macd {
    let c = closes(bars);
    let (f, s) = (crate::ta::ema(&c, fast.max(1)), crate::ta::ema(&c, slow.max(1)));
    let line: Vec<f64> = f.iter().zip(&s).map(|(a, b)| a - b).collect();
    let signal = crate::ta::ema(&line, sig.max(1));
    let hist = line.iter().zip(&signal).map(|(a, b)| a - b).collect();
    Macd { line, signal, hist }
}

/// Bandes : (moyenne, haute, basse).
pub struct Bands {
    pub basis: Vec<f64>,
    pub upper: Vec<f64>,
    pub lower: Vec<f64>,
}

/// Bandes de Bollinger 20, 2 (SMA ± 2 écarts-types).
pub fn bollinger(bars: &[Bar]) -> Bands {
    bollinger_n(bars, 20, 2.0)
}

/// Bandes de Bollinger de longueur `len`, à `mult` écarts-types.
pub fn bollinger_n(bars: &[Bar], len: usize, mult: f64) -> Bands {
    let c = closes(bars);
    let (m, s) = (sma(&c, len.max(1)), stdev(&c, len.max(1)));
    Bands {
        upper: m.iter().zip(&s).map(|(m, s)| m + mult * s).collect(),
        lower: m.iter().zip(&s).map(|(m, s)| m - mult * s).collect(),
        basis: m,
    }
}

/// Croisement EMA 9 / 21 : (EMA 9, EMA 21, croisements haussiers, baissiers).
pub struct EmaCross {
    pub fast: Vec<f64>,
    pub slow: Vec<f64>,
    pub up: Vec<usize>,
    pub down: Vec<usize>,
}

pub fn ema_cross(bars: &[Bar]) -> EmaCross {
    ema_cross_n(bars, 9, 21)
}

/// Croisement de deux EMA (rapide, lente), comme deux `ta.ema` de TradingView (B-096) : chaque moyenne démarre sur la
/// moyenne simple de ses premières clôtures, vide avant ; un croisement demande les deux moyennes à la bougie d'avant.
pub fn ema_cross_n(bars: &[Bar], f: usize, s: usize) -> EmaCross {
    let c = closes(bars);
    let (fast, slow) = (crate::ta::ema(&c, f.max(1)), crate::ta::ema(&c, s.max(1)));
    let (mut up, mut down) = (vec![], vec![]);
    for i in 1..c.len() {
        if fast[i] > slow[i] && fast[i - 1] <= slow[i - 1] {
            up.push(i);
        }
        if fast[i] < slow[i] && fast[i - 1] >= slow[i - 1] {
            down.push(i);
        }
    }
    EmaCross { fast, slow, up, down }
}

/// Stochastique 14, 3, 3 de v1 (le vieux site) : (%K lissé, %D), (clôture − plus bas) / (plus haut − plus bas) des 14
/// bougies. L'indicateur de la chart part de 14, 1, 3, comme TradingView (`STOCH_PARAMS`).
pub fn stochastic(bars: &[Bar]) -> (Vec<f64>, Vec<f64>) {
    stochastic_n(bars, 14, 3, 3)
}

/// Stochastique (longueur %K, lissage %K, lissage %D), comme `ta.stoch` de TradingView : vide tant que la chart n'a
/// pas `len` bougies (B-094 ; avant, les premières bougies prenaient une fenêtre plus courte).
pub fn stochastic_n(bars: &[Bar], len: usize, smooth: usize, dl: usize) -> (Vec<f64>, Vec<f64>) {
    let n = bars.len();
    let raw: Vec<f64> = (0..n)
        .map(|i| {
            if i + 1 < len.max(1) {
                return f64::NAN;
            }
            let a = i + 1 - len.max(1);
            let h = bars[a..=i].iter().map(|b| b.high).fold(f64::NEG_INFINITY, f64::max);
            let l = bars[a..=i].iter().map(|b| b.low).fold(f64::INFINITY, f64::min);
            let r = if h - l == 0.0 { 1.0 } else { h - l };
            (bars[i].close - l) / r * 100.0
        })
        .collect();
    let k = sma(&raw, smooth.max(1));
    let d = sma(&k, dl.max(1));
    (k, d)
}

/// ATR 14.
pub fn atr14(bars: &[Bar]) -> Vec<f64> {
    atr(bars, 14)
}

/// ATR de longueur `len`.
pub fn atr_n(bars: &[Bar], len: usize) -> Vec<f64> {
    atr(bars, len.max(1))
}

/// VWAP du jour (remis à zéro à minuit UTC), prix typique (haut + bas + clôture) / 3.
pub fn vwap(bars: &[Bar]) -> Vec<f64> {
    vwap_src(bars, true)
}

/// VWAP du jour sur le prix typique (`hl3`) ou sur la clôture.
pub fn vwap_src(bars: &[Bar], hl3: bool) -> Vec<f64> {
    let (mut pv, mut vv, mut day) = (0.0, 0.0, i64::MIN);
    bars.iter()
        .map(|b| {
            let d = b.time.div_euclid(86_400);
            if d != day {
                (day, pv, vv) = (d, 0.0, 0.0);
            }
            pv += if hl3 { (b.high + b.low + b.close) / 3.0 } else { b.close } * b.volume;
            vv += b.volume;
            if vv > 0.0 {
                pv / vv
            } else {
                f64::NAN
            }
        })
        .collect()
}

/// Volume relatif : volume / moyenne 20.
pub fn relative_volume(bars: &[Bar]) -> Vec<f64> {
    relative_volume_n(bars, 20)
}

/// Volume relatif sur une moyenne de `len` bougies.
pub fn relative_volume_n(bars: &[Bar], len: usize) -> Vec<f64> {
    let v: Vec<f64> = bars.iter().map(|b| b.volume).collect();
    let m = sma(&v, len.max(1));
    v.iter().zip(&m).map(|(v, m)| if *m > 0.0 { v / m } else { f64::NAN }).collect()
}

/// EMA multiples (par défaut 13, 50, 100, 200) : les 6 EMA de v1 (`p1` … `p6`, 0 = aucune), sur la clôture ou hl2,
/// comme `ta.ema` de TradingView (B-097) : démarrées sur la moyenne simple des `len` premières valeurs, NaN avant.
pub fn ema_multiples(bars: &[Bar], vals: &[f64]) -> Vec<Vec<f64>> {
    let p = Inputs {
        params: &EMA_MULTI_PARAMS,
        vals,
    };
    let src: Vec<f64> = bars.iter().map(|b| if p.on("src_hl2") { (b.high + b.low) / 2.0 } else { b.close }).collect();
    ["p1", "p2", "p3", "p4", "p5", "p6"]
        .iter()
        .map(|k| {
            let len = p.get(k).round().max(0.0) as usize;
            if len == 0 {
                return vec![f64::NAN; src.len()];
            }
            crate::ta::ema(&src, len)
        })
        .collect()
}

/// Moyenne et écart-type glissants comme le `rollMS` du site (les premières bougies : sur ce qu'il y a).
fn roll_ms(x: &[f64], len: usize) -> (Vec<f64>, Vec<f64>) {
    let (mut a, mut b) = (0.0, 0.0);
    let mut m = Vec::with_capacity(x.len());
    let mut s = Vec::with_capacity(x.len());
    for i in 0..x.len() {
        a += x[i];
        b += x[i] * x[i];
        if i >= len {
            a -= x[i - len];
            b -= x[i - len] * x[i - len];
        }
        let k = (i + 1).min(len) as f64;
        let mu = a / k;
        m.push(mu);
        s.push((b / k - mu * mu).max(0.0).sqrt());
    }
    (m, s)
}

/// Argos Bollinger Bands (20, 2, SMA) avec le squeeze : largeur au plus bas des 120 dernières bougies (× 1,05).
pub struct ArgosBands {
    pub bands: Bands,
    pub squeeze: Vec<bool>,
}

pub fn argos_bands(bars: &[Bar], vals: &[f64]) -> ArgosBands {
    let p = Inputs { params: &BANDS_PARAMS, vals };
    let (len, mult, sq) = (p.len("len"), p.get("mult"), p.len("sqlen"));
    let c = closes(bars);
    let (m, s) = roll_ms(&c, len);
    let ok = |i: usize| i + 1 >= len;
    // moyenne SMA (masquée avant `len`) ou EMA de v1
    let basis: Vec<f64> = if p.pick("ma") == 1 {
        ema(&c, len)
    } else {
        (0..c.len()).map(|i| if ok(i) { m[i] } else { f64::NAN }).collect()
    };
    let upper: Vec<f64> = (0..c.len()).map(|i| if ok(i) { basis[i] + mult * s[i] } else { f64::NAN }).collect();
    let lower: Vec<f64> = (0..c.len()).map(|i| if ok(i) { basis[i] - mult * s[i] } else { f64::NAN }).collect();
    let w: Vec<f64> = (0..c.len()).map(|i| (upper[i] - lower[i]) / basis[i]).collect();
    let squeeze = (0..c.len())
        .map(|i| {
            if !p.on("sq") || !w[i].is_finite() || i < sq {
                return false;
            }
            let mn = w[i + 1 - sq..=i].iter().filter(|v| v.is_finite()).fold(f64::INFINITY, |a, &b| a.min(b));
            w[i] <= mn * 1.05
        })
        .collect();
    ArgosBands {
        bands: Bands { basis, upper, lower },
        squeeze,
    }
}

/// 💪 **Argos Force des cassures** (conception Argos, écrit de zéro ; affiché « Argos Bollinger Oscillator »).
/// Pour chaque bougie : de combien la clôture sort de sa bande (moyenne ± `mult` écarts-types sur `len`), en largeurs
/// de demi-bande, bornée de 0 à 2 ; puis lissée (moyenne exponentielle sur `len`) et ramenée de 0 à 100.
/// Deux séries : la force au-dessus de la bande haute (haussière) et sous la bande basse (baissière) ; 50 = une
/// cassure moyenne d'une demi-bande.
pub fn bollinger_oscillator(bars: &[Bar], vals: &[f64]) -> (Vec<f64>, Vec<f64>) {
    let p = Inputs {
        params: &BANDS_OSC_PARAMS,
        vals,
    };
    let (len, mult) = (p.len("len").max(2), p.get("mult").max(0.01));
    let c = closes(bars);
    let (mid, sd) = (sma(&c, len), stdev(&c, len));
    let (mut up, mut dn) = (Vec::with_capacity(c.len()), Vec::with_capacity(c.len()));
    for i in 0..c.len() {
        let half = mult * sd[i];
        if !(mid[i].is_finite() && half > 0.0) {
            up.push(f64::NAN);
            dn.push(f64::NAN);
            continue;
        }
        up.push(((c[i] - (mid[i] + half)) / half).clamp(0.0, 2.0));
        dn.push((((mid[i] - half) - c[i]) / half).clamp(0.0, 2.0));
    }
    let scale = |v: Vec<f64>| ema(&v, len).into_iter().map(|x| x * 50.0).collect();
    (scale(up), scale(dn))
}

/// 🌡️ **Argos Chaleur du volume** (conception Argos, écrit de zéro ; affiché « Argos Heatmap Volume ») : l'écart de
/// chaque volume à sa moyenne, en écarts-types (`len`). Palier 0 = très fort (≥ `t1` σ) … 4 = faible (sous `t4` σ).
#[derive(Debug, Clone, PartialEq)]
pub struct Heatmap {
    pub volume: Vec<f64>,
    pub tier: Vec<u8>,
    /// Moyenne et écart-type glissants du volume (pour les lignes de seuil).
    pub mean: Vec<f64>,
    pub std: Vec<f64>,
}

pub fn heatmap_volume(bars: &[Bar], vals: &[f64]) -> Heatmap {
    let p = Inputs { params: &HEATMAP_PARAMS, vals };
    let len = p.len("len").max(2);
    let volume: Vec<f64> = bars.iter().map(|b| b.volume).collect();
    // au début : moyenne et écart-type de ce qu'il y a déjà (pas de trou dans la heatmap)
    let mut mean = Vec::with_capacity(volume.len());
    let mut std = Vec::with_capacity(volume.len());
    for i in 0..volume.len() {
        let w = &volume[i.saturating_sub(len - 1)..=i];
        let m = w.iter().sum::<f64>() / w.len() as f64;
        let var = w.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / w.len() as f64;
        mean.push(m);
        std.push(var.sqrt());
    }
    let cuts = ["t1", "t2", "t3", "t4"].map(|k| p.get(k));
    let tier = (0..volume.len())
        .map(|i| {
            let z = if std[i] > 0.0 { (volume[i] - mean[i]) / std[i] } else { 0.0 };
            cuts.iter().position(|c| z >= *c).unwrap_or(4) as u8
        })
        .collect();
    Heatmap { volume, tier, mean, std }
}

/// 📊 **Argos Profil de volume** (écrit de zéro) : sur les `bars` dernières bougies, `rows` rangées de prix ; le volume de
/// chaque bougie est réparti à parts égales sur les rangées que touche sa mèche, côté hausse ou baisse selon la
/// bougie. POC = rangée la plus chargée ; zone de valeur = rangées ajoutées autour du POC (la plus chargée des deux
/// voisines à chaque pas) jusqu'à `va` % du volume.
#[derive(Debug, Clone, PartialEq)]
pub struct Profile {
    /// Première bougie de la plage.
    pub from: usize,
    pub bot: f64,
    pub step: f64,
    /// Volume haussier et baissier de chaque rangée (du bas vers le haut).
    pub up: Vec<f64>,
    pub down: Vec<f64>,
    pub poc: usize,
    /// Zone de valeur : rangées `va_lo..=va_hi`.
    pub va_lo: usize,
    pub va_hi: usize,
}

pub fn volume_profile(bars: &[Bar], vals: &[f64]) -> Option<Profile> {
    let p = Inputs { params: &PROFILE_PARAMS, vals };
    let (n, rows, va) = (p.len("bars").max(1), p.len("rows").max(1), p.get("va").clamp(1.0, 100.0) / 100.0);
    let from = bars.len().checked_sub(n.min(bars.len()))?;
    let range = &bars[from..];
    let top = range.iter().map(|b| b.high).fold(f64::NEG_INFINITY, f64::max);
    let bot = range.iter().map(|b| b.low).fold(f64::INFINITY, f64::min);
    if range.is_empty() || !(top > bot) {
        return None;
    }
    let step = (top - bot) / rows as f64;
    let row_of = |x: f64| (((x - bot) / step).floor().max(0.0) as usize).min(rows - 1);
    let (mut up, mut down) = (vec![0.0; rows], vec![0.0; rows]);
    for b in range {
        let (a, z) = (row_of(b.low), row_of(b.high));
        let share = b.volume / (z - a + 1) as f64;
        let side = if b.close >= b.open { &mut up } else { &mut down };
        side[a..=z].iter_mut().for_each(|v| *v += share);
    }
    let tot: Vec<f64> = up.iter().zip(&down).map(|(u, d)| u + d).collect();
    let all: f64 = tot.iter().sum();
    let poc = (0..rows).fold(0, |best, k| if tot[k] > tot[best] { k } else { best });
    let (mut lo, mut hi, mut got) = (poc, poc, tot[poc]);
    while got < all * va && (lo > 0 || hi + 1 < rows) {
        let below = if lo > 0 { tot[lo - 1] } else { f64::NEG_INFINITY };
        let above = if hi + 1 < rows { tot[hi + 1] } else { f64::NEG_INFINITY };
        if above >= below {
            hi += 1;
            got += above;
        } else {
            lo -= 1;
            got += below;
        }
    }
    Some(Profile {
        from,
        bot,
        step,
        up,
        down,
        poc,
        va_lo: lo,
        va_hi: hi,
    })
}

// 📋 Summary tables (« Tableau résumé ») of Heatmap, Bands, Bands Osc and Profile: the rows the chart draws, shared
// with Argos AI (Lens) so both say exactly the same. They always compute the rows; the caller checks `showTable`.
// Their numbers follow the chosen number format (🌐, `argos_core::nombres`), as on the chart.

/// Short volume as v1's `kf`, with the decimal mark of the chosen number format (🌐).
pub use argos_core::nombres::kf;

/// 📋 Argos Heatmap Volume's summary table, as v1's `hmv` dash (src/26-gabarit-unique-de-l-assistant.js): the last
/// bar's volume, its level, its deviation from the mean (σ) and the mean.
pub fn heatmap_tableau(h: &Heatmap) -> Vec<(&'static str, String)> {
    const NIVEAUX: [&str; 5] = ["Très fort", "Fort", "Moyen", "Normal", "Faible"];
    let Some(z) = h.volume.len().checked_sub(1) else {
        return vec![];
    };
    let e = if h.std[z] > 0.0 { (h.volume[z] - h.mean[z]) / h.std[z] } else { 0.0 };
    vec![
        ("Volume", kf(h.volume[z])),
        ("Niveau", NIVEAUX[(h.tier[z] as usize).min(4)].to_string()),
        ("Écart", format!("{}{} σ", if e >= 0.0 { "+" } else { "−" }, format_number(e.abs(), 2))),
        ("Moyenne", kf(h.mean[z])),
    ]
}

/// 📋 Argos Bollinger Bands' summary table, as v1's `bb` dash (src/26-gabarit-unique-de-l-assistant.js): %B, width,
/// squeeze and where the last close is.
pub fn bands_tableau(a: &ArgosBands, bars: &[Bar]) -> Vec<(&'static str, String)> {
    let Some(z) = bars.len().checked_sub(1).filter(|&z| z < a.bands.upper.len()) else {
        return vec![];
    };
    let (c, up, lo, mid) = (bars[z].close, a.bands.upper[z], a.bands.lower[z], a.bands.basis[z]);
    let pb = (c - lo) / if up - lo != 0.0 { up - lo } else { 1.0 };
    let pct = |v: f64, d: usize| {
        if v.is_finite() {
            format!("{} %", format_number(v * 100.0, d))
        } else {
            "—".to_string()
        }
    };
    let position = if c > up {
        "Au-dessus de la bande haute"
    } else if c < lo {
        "Sous la bande basse"
    } else if c >= mid {
        "Moitié haute"
    } else {
        "Moitié basse"
    };
    vec![
        ("%B", pct(pb, 0)),
        ("Largeur", pct((up - lo) / mid, 2)),
        (
            "Squeeze",
            if a.squeeze.get(z) == Some(&true) {
                "Oui 🟡 (explosion à venir)"
            } else {
                "Non"
            }
            .to_string(),
        ),
        ("Position", position.to_string()),
    ]
}

/// 📋 Argos Bollinger Oscillator's summary table, as v1's `bbo` dash (src/26-gabarit-unique-de-l-assistant.js): the
/// last bullish and bearish strength, in %.
pub fn bands_osc_tableau(bull: &[f64], bear: &[f64]) -> Vec<(&'static str, String)> {
    let last = |v: &[f64]| v.last().copied().filter(|x| x.is_finite()).unwrap_or(0.0);
    vec![
        ("Haussier", format!("{} %", format_number(last(bull), 0))),
        ("Baissier", format!("{} %", format_number(last(bear), 0))),
    ]
}

/// 📋 Argos Volume Profile's summary table, as v1's `vprof` dash (src/26-gabarit-unique-de-l-assistant.js): POC, VAH,
/// VAL and the last close against the POC. `prix` writes a price (the chart: its number format and the bars'
/// decimals); empty without bars.
pub fn profile_tableau(p: &Profile, bars: &[Bar], prix: &dyn Fn(f64) -> String) -> Vec<(&'static str, String)> {
    let Some(last) = bars.last() else {
        return vec![];
    };
    let poc = p.bot + p.step * (p.poc as f64 + 0.5);
    vec![
        ("POC", prix(poc)),
        ("VAH", prix(p.bot + p.step * (p.va_hi + 1) as f64)),
        ("VAL", prix(p.bot + p.step * p.va_lo as f64)),
        ("Prix vs POC", if last.close >= poc { "Au-dessus ⬆️" } else { "En dessous ⬇️" }.to_string()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bars(n: usize) -> Vec<Bar> {
        (0..n)
            .map(|i| {
                let t = i as f64;
                let c = 100.0 + 5.0 * (t / 9.0).sin() + t * 0.01;
                Bar {
                    time: 1_700_000_000 + i as i64 * 3600,
                    open: c - 0.3,
                    high: c + 1.0,
                    low: c - 1.0,
                    close: c + 0.2,
                    volume: 100.0 + 40.0 * (t / 4.0).cos().abs(),
                }
            })
            .collect()
    }

    #[test]
    fn classiques() {
        let b = bars(400);
        let r = rsi14(&b);
        assert!(r[399] > 0.0 && r[399] < 100.0);
        let m = macd(&b);
        assert!((m.hist[399] - (m.line[399] - m.signal[399])).abs() < 1e-12);
        let bb = bollinger(&b);
        assert!(bb.upper[399] > bb.basis[399] && bb.basis[399] > bb.lower[399]);
        let x = ema_cross(&b);
        assert!(!x.up.is_empty() && !x.down.is_empty(), "les vagues font des croisements");
        let (k, d) = stochastic(&b);
        assert!((0.0..=100.0).contains(&k[399]) && (0.0..=100.0).contains(&d[399]));
        assert!(atr14(&b)[399] > 0.0);
        let v = vwap(&b);
        assert!(v[399] > 90.0 && v[399] < 115.0);
        assert!(relative_volume(&b)[399] > 0.0);
        let d = |t: &[crate::inputs::Param]| crate::inputs::defaults(t);
        let e = ema_multiples(&b, &d(&EMA_MULTI_PARAMS));
        assert_eq!(e.len(), 6);
        assert!(e[4].iter().all(|v| v.is_nan()), "EMA 5 à 0 : aucune");
        assert!(e[3][198].is_nan() && e[3][199].is_finite(), "EMA 200 : rien avant 200 bougies");
    }

    #[test]
    fn natifs_simples() {
        let b = bars(800);
        let d = |t: &[crate::inputs::Param]| crate::inputs::defaults(t);
        let ab = argos_bands(&b, &d(&BANDS_PARAMS));
        assert!(ab.bands.upper[799] > ab.bands.lower[799]);
        let (bu, be) = bollinger_oscillator(&b, &d(&BANDS_OSC_PARAMS));
        assert!((0.0..=100.0).contains(&bu[799]) && (0.0..=100.0).contains(&be[799]));
        let h = heatmap_volume(&b, &d(&HEATMAP_PARAMS));
        assert!(h.tier.iter().all(|t| *t <= 4));
        let p = volume_profile(&b, &d(&PROFILE_PARAMS)).unwrap();
        assert_eq!((p.from, p.up.len()), (600, 30), "défauts d'Argos : 200 bougies, 30 rangées");
        // inputs : 300 bougies, 40 rangées
        let mut v = d(&PROFILE_PARAMS);
        (v[0], v[1]) = (300.0, 40.0);
        let q = volume_profile(&b, &v).unwrap();
        assert_eq!((q.from, q.up.len()), (500, 40));
        assert!(p.va_lo <= p.poc && p.poc <= p.va_hi);
        // la zone de valeur contient au moins 68 % du volume, et tout le volume de la plage est réparti
        let tot: f64 = p.up.iter().zip(&p.down).map(|(u, d)| u + d).sum();
        let va: f64 = (p.va_lo..=p.va_hi).map(|x| p.up[x] + p.down[x]).sum();
        assert!(va >= tot * 0.68 - 1e-9);
        let vol: f64 = b[600..].iter().map(|x| x.volume).sum();
        assert!((tot - vol).abs() < 1e-6 * vol.max(1.0));
        // force des cassures : nulle tant que la clôture reste dans sa bande
        let flat: Vec<Bar> = (0..60)
            .map(|i| Bar {
                close: 100.0 + (i % 2) as f64 * 0.1,
                ..b[i]
            })
            .collect();
        let (fu, fd) = bollinger_oscillator(&flat, &d(&BANDS_OSC_PARAMS));
        assert!(fu[59] < 1.0 && fd[59] < 1.0, "pas de cassure : force presque nulle");
    }

    /// 📋 The summary tables' rows, to the character (the chart draws them, Argos AI reads them), in the default number
    /// format (🌐 « 1 234,56 »: this test never changes the chosen one).
    #[test]
    fn lignes_des_tableaux_au_caractere_pres() {
        assert_eq!(
            (kf(950.0), kf(1234.0), kf(3_456_789.0), kf(123_456.0), kf(2.1e9)),
            ("950".into(), "1,2K".into(), "3,5M".into(), "123K".into(), "2,1B".into())
        );
        let h = Heatmap {
            volume: vec![10.0, 1500.0],
            tier: vec![4, 1],
            mean: vec![10.0, 1000.0],
            std: vec![0.0, 200.0],
        };
        assert_eq!(
            heatmap_tableau(&h),
            vec![
                ("Volume", "1,5K".into()),
                ("Niveau", "Fort".into()),
                ("Écart", "+2,50 σ".into()),
                ("Moyenne", "1,0K".into())
            ]
        );
        let bar = |c: f64| Bar {
            time: 0,
            open: c,
            high: c,
            low: c,
            close: c,
            volume: 1.0,
        };
        let a = ArgosBands {
            bands: Bands {
                basis: vec![100.0],
                upper: vec![110.0],
                lower: vec![90.0],
            },
            squeeze: vec![true],
        };
        assert_eq!(
            bands_tableau(&a, &[bar(105.0)]),
            vec![
                ("%B", "75 %".into()),
                ("Largeur", "20,00 %".into()),
                ("Squeeze", "Oui 🟡 (explosion à venir)".into()),
                ("Position", "Moitié haute".into())
            ]
        );
        assert_eq!(
            bands_osc_tableau(&[12.4, 61.6], &[f64::NAN]),
            vec![("Haussier", "62 %".into()), ("Baissier", "0 %".into())]
        );
        let p = Profile {
            from: 0,
            bot: 100.0,
            step: 2.0,
            up: vec![1.0; 5],
            down: vec![1.0; 5],
            poc: 2,
            va_lo: 1,
            va_hi: 3,
        };
        assert_eq!(
            profile_tableau(&p, &[bar(104.0)], &|v| format!("{v:.1}")),
            vec![
                ("POC", "105.0".into()),
                ("VAH", "108.0".into()),
                ("VAL", "102.0".into()),
                ("Prix vs POC", "En dessous ⬇️".into())
            ]
        );
        assert!(profile_tableau(&p, &[], &|v| format!("{v}")).is_empty());
    }
}
