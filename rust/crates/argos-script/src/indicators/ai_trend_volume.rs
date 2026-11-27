//! Argos AI Trend V Pro, sections 2 et 3 du Pine « Ultimate Ai Trend All Div Pro » (TradingView) :
//! Volume Vector (bougies colorées selon le volume), EMA 13 / 50 / 200, patterns de retournement avec filtres de confirmation
//! (RSI, MACD, support / résistance), divergence Volume / Chaos RSI et signal 🚀 Ignition. Calcul pur ; le dessin est dans la chart.

use crate::inputs::Inputs;
use crate::ta::{ema, highest, lowest, pivot, rsi, sma};
use argos_core::Bar;

#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    /// Preset Volume Vector : 0 Auto, 1 Scalping, 2 Intraday, 3 Swing, 4 Position, 5 Daily/Weekly, 6 Custom.
    pub vec_preset: usize,
    /// MID / HIGH / GOLD en % du volume moyen, longueur de la moyenne (preset Custom).
    pub vec_custom: (f64, f64, f64, usize),
    pub pat_trend: bool,
    pub pat_trend_len: usize,
    pub conf_on: bool,
    pub conf_all: bool,
    pub conf_rsi: bool,
    pub conf_rsi_len: usize,
    pub conf_os: f64,
    pub conf_ob: f64,
    pub conf_macd: bool,
    pub conf_macd_len: (usize, usize, usize),
    pub conf_sr: bool,
    pub conf_sr_lookback: usize,
    pub conf_tolerance: f64,
    pub conf_next: bool,
    /// Preset Ignition : 0 Auto, 1 30m, 2 2H, 3 Custom.
    pub ign_preset: usize,
    pub ign_custom: Ignition,
    /// Source « Gold » du Cipher : `false` = clôture > 0 (défaut du Pine, toujours armé), `true` = Golden Dot du Cipher Argos.
    pub ign_cipher_gold: bool,
}

/// Longueurs de l'Ignition (Chaos RSI, CVD, divergences, fenêtre de confluence).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ignition {
    pub rsi: usize,
    pub fractal: usize,
    pub smooth_min: usize,
    pub smooth_max: usize,
    pub cvd: usize,
    pub left: usize,
    pub right: usize,
    pub range_low: usize,
    pub range_up: usize,
    pub lookback: usize,
}

impl Settings {
    pub fn from_inputs(p: &Inputs) -> Self {
        Self {
            vec_preset: p.pick("vv_preset"),
            vec_custom: (p.get("vv_mid"), p.get("vv_high"), p.get("vv_gold"), p.len("vv_len")),
            pat_trend: p.on("pat_trend"),
            pat_trend_len: p.len("pat_tlen"),
            conf_on: p.on("conf_on"),
            conf_all: p.pick("conf_mode") == 1,
            conf_rsi: p.on("conf_rsi"),
            conf_rsi_len: p.len("conf_rlen"),
            conf_os: p.get("conf_os"),
            conf_ob: p.get("conf_ob"),
            conf_macd: p.on("conf_macd"),
            conf_macd_len: (p.len("conf_mf"), p.len("conf_ms"), p.len("conf_msig")),
            conf_sr: p.on("conf_sr"),
            conf_sr_lookback: p.len("conf_srlb"),
            conf_tolerance: p.get("conf_tol"),
            conf_next: p.on("conf_next"),
            ign_preset: p.pick("ign_preset"),
            ign_custom: Ignition {
                rsi: p.len("ign_rsi"),
                fractal: p.len("ign_fd"),
                smooth_min: p.len("ign_smin"),
                smooth_max: p.len("ign_smax"),
                cvd: p.len("ign_cvd"),
                left: p.len("ign_dl"),
                right: p.len("ign_dr"),
                range_low: p.len("ign_rlo"),
                range_up: p.len("ign_rup"),
                lookback: p.len("ign_lb"),
            },
            ign_cipher_gold: p.pick("ign_gold") == 1,
        }
    }

    /// MID / HIGH / GOLD (%) et longueur de la moyenne de volume, selon le preset (valeurs du Pine).
    pub fn vector_levels(&self, chart_seconds: i64) -> (f64, f64, f64, usize) {
        let name = match self.vec_preset {
            6 => return self.vec_custom,
            0 => match chart_seconds / 60 {
                m if m <= 5 => 1,
                m if m <= 30 => 2,
                m if m <= 240 => 3,
                m if m <= 720 => 4,
                _ => 5,
            },
            p => p,
        };
        match name {
            1 => (130.0, 180.0, 250.0, 20),
            2 => (140.0, 190.0, 270.0, 20),
            4 => (160.0, 220.0, 320.0, 14),
            5 => (170.0, 240.0, 350.0, 10),
            _ => (150.0, 200.0, 300.0, 20),
        }
    }

    /// Longueurs de l'Ignition selon le preset (Auto : UT ≤ 60m → 30m, sinon 2H).
    pub fn ignition(&self, chart_seconds: i64) -> Ignition {
        let p = match self.ign_preset {
            3 => return self.ign_custom,
            0 if chart_seconds <= 3600 => 1,
            0 => 2,
            p => p,
        };
        let l = |rsi, fractal, smooth_min, smooth_max, cvd, left, right, range_low, range_up, lookback| Ignition {
            rsi,
            fractal,
            smooth_min,
            smooth_max,
            cvd,
            left,
            right,
            range_low,
            range_up,
            lookback,
        };
        if p == 1 {
            l(12, 20, 2, 14, 14, 2, 2, 4, 40, 15)
        } else {
            l(14, 24, 3, 21, 20, 3, 3, 5, 80, 24)
        }
    }
}

/// Niveau de volume d'une bougie (Volume Vector).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tier {
    #[default]
    Low,
    Mid,
    High,
    Gold,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Volume {
    pub tier: Vec<Tier>,
    pub ema13: Vec<f64>,
    pub ema50: Vec<f64>,
    pub ema200: Vec<f64>,
    /// Patterns confirmés : (bougie du signal, nom du pattern).
    pub bull_patterns: Vec<(usize, &'static str)>,
    pub bear_patterns: Vec<(usize, &'static str)>,
    /// Divergences haussières Volume (CVD) / Chaos RSI : bougie où elle est confirmée.
    pub vol_chaos_bull: Vec<usize>,
    /// 🚀 Ignition : confluence Vol/Chaos + Chaos RSI + Gold Cipher qui s'allume.
    pub ignition: Vec<usize>,
    /// Abandoned Baby haussier / baissier bruts (alertes du Pine).
    pub baby_bull: Vec<usize>,
    pub baby_bear: Vec<usize>,
    /// Preset Volume Vector utilisé (pour la légende).
    pub vec_preset: &'static str,
}

const BULL_NAMES: [&str; 9] = [
    "Hammer",
    "Inv. Hammer",
    "Bullish Engulfing",
    "Piercing Line",
    "Bullish Harami",
    "Morning Star",
    "Tweezer Bottom",
    "3 White Soldiers",
    "Abandoned Baby",
];
const BEAR_NAMES: [&str; 9] = [
    "Shooting Star",
    "Hanging Man",
    "Bearish Engulfing",
    "Dark Cloud Cover",
    "Bearish Harami",
    "Evening Star",
    "Tweezer Top",
    "3 Black Crows",
    "Abandoned Baby",
];

/// Nom du premier pattern haussier / baissier de la bougie `i` (ordre du Pine), sans le filtre de confirmation.
fn patterns(b: &[Bar], i: usize, down_ctx: bool, up_ctx: bool) -> (Option<&'static str>, Option<&'static str>, bool, bool) {
    if i < 2 {
        return (None, None, false, false);
    }
    let (c, p, q) = (&b[i], &b[i - 1], &b[i - 2]);
    let body0 = (c.close - c.open).abs();
    let range0 = c.high - c.low;
    let up_wick0 = c.high - c.close.max(c.open);
    let lo_wick0 = c.close.min(c.open) - c.low;
    let (bull0, bear0) = (c.close > c.open, c.close < c.open);
    let small0 = range0 > 0.0 && body0 <= range0 * 0.3;
    let body1 = (p.close - p.open).abs();
    let range1 = p.high - p.low;
    let (bull1, bear1) = (p.close > p.open, p.close < p.open);
    let mid1 = (p.open + p.close) / 2.0;
    let small1 = range1 > 0.0 && body1 <= range1 * 0.3;
    let (bull2, bear2) = (q.close > q.open, q.close < q.open);
    let bull = [
        down_ctx && small0 && lo_wick0 >= body0 * 2.0 && up_wick0 <= body0 * 0.5,
        down_ctx && small0 && up_wick0 >= body0 * 2.0 && lo_wick0 <= body0 * 0.5,
        down_ctx && bear1 && bull0 && c.close >= p.open && c.open <= p.close && body0 > body1,
        down_ctx && bear1 && bull0 && c.open < p.close && c.close > mid1 && c.close < p.open,
        down_ctx && bear1 && bull0 && c.open > p.close && c.close < p.open && body0 < body1,
        down_ctx && bear2 && body1 <= range1 * 0.4 && bull0 && c.close > (q.open + q.close) / 2.0 && p.close < q.close,
        down_ctx && bear1 && bull0 && (c.low - p.low).abs() <= range1 * 0.1,
        bull2 && bull1 && bull0 && p.close > q.close && c.close > p.close && p.open < q.close && c.open < p.close,
        down_ctx && bear2 && small1 && p.high < q.low && bull0 && c.low > p.high,
    ];
    let bear = [
        up_ctx && small0 && up_wick0 >= body0 * 2.0 && lo_wick0 <= body0 * 0.5,
        up_ctx && small0 && lo_wick0 >= body0 * 2.0 && up_wick0 <= body0 * 0.5,
        up_ctx && bull1 && bear0 && c.close <= p.open && c.open >= p.close && body0 > body1,
        up_ctx && bull1 && bear0 && c.open > p.close && c.close < mid1 && c.close > p.open,
        up_ctx && bull1 && bear0 && c.open < p.close && c.close > p.open && body0 < body1,
        up_ctx && bull2 && body1 <= range1 * 0.4 && bear0 && c.close < (q.open + q.close) / 2.0 && p.close > q.close,
        up_ctx && bull1 && bear0 && (c.high - p.high).abs() <= range1 * 0.1,
        bear2 && bear1 && bear0 && p.close < q.close && c.close < p.close && p.open > q.close && c.open > p.close,
        up_ctx && bull2 && small1 && p.low > q.high && bear0 && c.high < p.low,
    ];
    (
        bull.iter().position(|&x| x).map(|k| BULL_NAMES[k]),
        bear.iter().position(|&x| x).map(|k| BEAR_NAMES[k]),
        bull[8],
        bear[8],
    )
}

/// Comme `ta.barssince(cond)` : bougies depuis la dernière fois que `cond` était vrai (à `i` compris), `None` si jamais.
fn bars_since(last: Option<usize>, i: usize) -> Option<usize> {
    last.map(|j| i - j)
}

/// Fractal dimension (`f_fractalDim` du Pine), bornée entre 1 et 2.
fn fractal_dim(b: &[Bar], hh: &[f64], ll: &[f64], i: usize, len: usize) -> f64 {
    let range = hh[i] - ll[i];
    if len < 2 || i + 1 < len || !range.is_finite() || range == 0.0 {
        return 1.0;
    }
    let sum: f64 = (0..len - 1)
        .map(|k| {
            let p1 = (b[i - k].close - ll[i]) / range;
            let p2 = (b[i - k - 1].close - ll[i]) / range;
            ((p1 - p2).powi(2) + (1.0 / len as f64).powi(2)).sqrt()
        })
        .sum();
    let fd = if sum > 0.0 {
        1.0 + (sum.ln() + 2f64.ln()) / (2.0 * len as f64).ln()
    } else {
        1.0
    };
    fd.clamp(1.0, 2.0)
}

pub fn compute(bars: &[Bar], chart_seconds: i64, s: &Settings) -> Volume {
    let n = bars.len();
    let close: Vec<f64> = bars.iter().map(|b| b.close).collect();
    let vol: Vec<f64> = bars.iter().map(|b| b.volume).collect();

    // ---- Volume Vector
    let (mid, high, gold, avg_len) = s.vector_levels(chart_seconds);
    let avg = sma(&vol, avg_len);
    let tier = (0..n)
        .map(|i| {
            let pct = if avg[i] > 0.0 { vol[i] / avg[i] * 100.0 } else { 0.0 };
            match pct {
                p if p >= gold => Tier::Gold,
                p if p >= high => Tier::High,
                p if p >= mid => Tier::Mid,
                _ => Tier::Low,
            }
        })
        .collect();
    let vec_preset = ["Auto", "Scalping", "Intraday", "Swing", "Position", "Daily/Weekly", "Custom"][s.vec_preset.min(6)];

    // ---- Patterns + confirmation
    let trend = sma(&close, s.pat_trend_len);
    let rsi_c = rsi(&close, s.conf_rsi_len);
    let (mf, ms, msig) = s.conf_macd_len;
    let (ef, es) = (ema(&close, mf), ema(&close, ms));
    let macd: Vec<f64> = (0..n).map(|i| ef[i] - es[i]).collect();
    let sig = ema(&macd, msig);
    let hist: Vec<f64> = (0..n).map(|i| macd[i] - sig[i]).collect();
    let (hi, lo): (Vec<f64>, Vec<f64>) = bars.iter().map(|b| (b.high, b.low)).unzip();
    let lb = s.conf_sr_lookback;
    let (mut last_ph, mut last_pl) = (f64::NAN, f64::NAN);
    let total = [s.conf_rsi, s.conf_macd, s.conf_sr].iter().filter(|&&x| x).count();
    let ok = |count: usize| !s.conf_on || total == 0 || if s.conf_all { count == total } else { count >= 1 };
    let mut confirmed: Vec<(Option<&'static str>, Option<&'static str>)> = vec![(None, None); n];
    let (mut baby_bull, mut baby_bear) = (vec![], vec![]);
    for i in 0..n {
        if let Some(j) = pivot(&hi, i, lb, lb, true) {
            last_ph = hi[j];
        }
        if let Some(j) = pivot(&lo, i, lb, lb, false) {
            last_pl = lo[j];
        }
        if i < 2 {
            continue;
        }
        let (down, up) = if s.pat_trend {
            (close[i - 1] < trend[i - 1], close[i - 1] > trend[i - 1])
        } else {
            (true, true)
        };
        let (bull, bear, baby_up, baby_dn) = patterns(bars, i, down, up);
        // Abandoned Baby brut (alertes du Pine, sans le filtre de confirmation)
        if baby_up {
            baby_bull.push(i);
        }
        if baby_dn {
            baby_bear.push(i);
        }
        let c = close[i];
        let near = |lvl: f64| lvl.is_finite() && (c - lvl).abs() / c * 100.0 <= s.conf_tolerance;
        let n_bull = [
            s.conf_rsi && rsi_c[i] <= s.conf_os,
            s.conf_macd && hist[i] > hist[i - 1],
            s.conf_sr && near(last_pl),
        ]
        .iter()
        .filter(|&&x| x)
        .count();
        let n_bear = [
            s.conf_rsi && rsi_c[i] >= s.conf_ob,
            s.conf_macd && hist[i] < hist[i - 1],
            s.conf_sr && near(last_ph),
        ]
        .iter()
        .filter(|&&x| x)
        .count();
        confirmed[i] = (bull.filter(|_| ok(n_bull)), bear.filter(|_| ok(n_bear)));
    }
    let (mut bull_patterns, mut bear_patterns) = (vec![], vec![]);
    for i in 1..n {
        // « Confirm with next candle close » : le signal sort à la bougie suivante si elle clôture au-delà de la mèche
        let (b, r) = if s.conf_next {
            (
                confirmed[i - 1].0.filter(|_| close[i] > hi[i - 1]),
                confirmed[i - 1].1.filter(|_| close[i] < lo[i - 1]),
            )
        } else {
            confirmed[i]
        };
        if let Some(name) = b {
            bull_patterns.push((i, name));
        }
        if let Some(name) = r {
            bear_patterns.push((i, name));
        }
    }

    // ---- Ignition : Chaos RSI (lissage selon la dimension fractale), CVD, divergences
    let g = s.ignition(chart_seconds);
    let cvd_delta: Vec<f64> = bars.iter().map(crate::indicators::flow::delta_estime).collect();
    let cvd: Vec<f64> = (0..n)
        .map(|i| {
            if i + 1 >= g.cvd {
                cvd_delta[i + 1 - g.cvd..=i].iter().sum()
            } else {
                f64::NAN
            }
        })
        .collect();
    let (hh, ll) = (highest(&hi, g.fractal), lowest(&lo, g.fractal));
    let raw = rsi(&close, g.rsi);
    let mut chaos = vec![f64::NAN; n];
    for i in 0..n {
        let factor = fractal_dim(bars, &hh, &ll, i, g.fractal) - 1.0;
        let len = g.smooth_min as f64 + factor * (g.smooth_max as f64 - g.smooth_min as f64);
        let a = 2.0 / (len + 1.0);
        let prev = if i > 0 { chaos[i - 1] } else { f64::NAN };
        chaos[i] = if prev.is_nan() { raw[i] } else { a * raw[i] + (1.0 - a) * prev };
    }
    // divergences haussières (f_regDivOsc / f_volChaosDiv du Pine) sur les creux du Chaos RSI
    let (l, r) = (g.left, g.right);
    let (mut prev_pl, mut last_found): (Option<usize>, Option<usize>) = (None, None);
    let (mut chaos_bull, mut vol_chaos_bull) = (vec![false; n], vec![false; n]);
    for i in 0..n {
        let found = pivot(&chaos, i, l, r, false).is_some();
        if found {
            // barssince(plFound[1]) : depuis le creux trouvé avant cette bougie
            let since = last_found.map(|j| i - 1 - j);
            let in_range = since.is_some_and(|d| g.range_low <= d && d <= g.range_up);
            if let Some(p) = prev_pl {
                let (k, pk) = (i - r, p - r);
                chaos_bull[i] = lo[k] < lo[pk] && chaos[k] > chaos[pk] && in_range;
                vol_chaos_bull[i] = chaos[k] < chaos[pk] && cvd[k] > cvd[pk] && in_range;
            }
            prev_pl = Some(i);
            last_found = Some(i);
        }
    }
    let gold_src: Option<Vec<bool>> = s.ign_cipher_gold.then(|| {
        let c = super::cipher::compute(bars, &super::cipher::Settings::default());
        let mut g = vec![false; n];
        for &i in c.gold.iter().filter(|&&i| i < n) {
            g[i] = true;
        }
        g
    });
    let (mut last_vol, mut last_chaos, mut last_gold) = (None, None, None);
    let mut prev_cond = false;
    let mut ignition = vec![];
    for i in 0..n {
        if vol_chaos_bull[i] {
            last_vol = Some(i);
        }
        if chaos_bull[i] {
            last_chaos = Some(i);
        }
        // défaut du Pine : source = clôture, « > 0 » toujours vrai
        if gold_src.as_ref().map_or(close[i] > 0.0, |gs| gs[i]) {
            last_gold = Some(i);
        }
        let armed = |last: Option<usize>| bars_since(last, i).is_some_and(|d| d <= g.lookback);
        let cond = armed(last_vol) && armed(last_chaos) && armed(last_gold);
        if cond && !prev_cond {
            ignition.push(i);
        }
        prev_cond = cond;
    }

    Volume {
        tier,
        ema13: ema(&close, 13),
        ema50: ema(&close, 50),
        ema200: ema(&close, 200),
        bull_patterns,
        bear_patterns,
        vol_chaos_bull: (0..n).filter(|&i| vol_chaos_bull[i]).collect(),
        ignition,
        baby_bull,
        baby_bear,
        vec_preset,
    }
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
    fn presets_du_pine() {
        let s = Settings::from_inputs(&Inputs {
            params: &crate::v1_inputs::AI_TREND_PARAMS,
            vals: &crate::inputs::defaults(&crate::v1_inputs::AI_TREND_PARAMS),
        });
        assert_eq!(s.vector_levels(300), (130.0, 180.0, 250.0, 20));
        assert_eq!(s.vector_levels(1800), (140.0, 190.0, 270.0, 20));
        assert_eq!(s.vector_levels(14_400), (150.0, 200.0, 300.0, 20));
        assert_eq!(s.vector_levels(86_400), (170.0, 240.0, 350.0, 10));
        assert_eq!(s.ignition(1800).rsi, 12);
        assert_eq!(s.ignition(7200).range_up, 80);
    }

    #[test]
    fn volume_vector_et_engulfing() {
        // 25 bougies calmes en baisse, puis un engulfing haussier sur gros volume
        let mut b: Vec<Bar> = (0..25)
            .map(|i| {
                let c = 100.0 - i as f64 * 0.5;
                bar(c + 0.3, c + 0.5, c - 0.5, c, 10.0)
            })
            .collect();
        let c = b[24].close;
        b.push(bar(c + 0.2, c + 0.3, c - 0.4, c - 0.3, 10.0));
        b.push(bar(c - 0.5, c + 1.5, c - 0.6, c + 1.2, 40.0));
        let mut s = Settings::from_inputs(&Inputs {
            params: &crate::v1_inputs::AI_TREND_PARAMS,
            vals: &crate::inputs::defaults(&crate::v1_inputs::AI_TREND_PARAMS),
        });
        s.conf_on = false;
        let v = compute(&b, 1800, &s);
        assert_eq!(v.tier[26], Tier::Gold, "40 / ~11 de moyenne ≥ 270 %");
        assert_eq!(v.tier[10], Tier::Low);
        assert!(v.bull_patterns.contains(&(26, "Bullish Engulfing")), "{:?}", v.bull_patterns);
    }
}
