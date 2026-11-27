//! Argos TBO Pro Div — Trend Break Oscillator (script Pine personnel de Jee, créé avec l'IA) : force des cassures de trendlines,
//! filtres ATR / ADX / HTF / volume, Vector 150+ / 200+, divergences régulières et cachées.
//! Même logique que le script Pine « TBO Pr Divergence » de TradingView (filtre HTF sans regarder dans le futur, dernier
//! événement, signaux gardés quand les points sont masqués, alertes). Les couleurs sont de l'affichage : elles restent dans la couche chart.

use crate::inputs::{Inputs, TF_SECS};
use crate::ta::{adx, atr, ema, highest, pivot, rma, sma, stdev, wma};
use argos_core::{aggregate, bucket_start, Bar, MONTH_SECONDS};

/// Calcul de la pente des trendlines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlopeCalc {
    Atr,
    Stdev,
    Linreg,
}

/// Lissage de la ligne Signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Smoothing {
    None,
    Rma,
    Sma,
    Tma,
    Ema,
    Dema,
    Tema,
    Hma,
    Wma,
    Swma,
    Vwma,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Preset {
    Custom,
    Auto,
    Scalping,
    DayTrade,
    Swing,
}

/// Filtre multi-timeframe : même UT que la chart, Auto (≤5m 1H · ≤30m 4H · ≤4H Daily · ≤2D Weekly · sinon Monthly), ou fixe (secondes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Htf {
    Chart,
    Auto,
    Fixed(i64),
}

/// Volume de chandelle requis pour un breakout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VecMode {
    Off,
    V150,
    V200,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    pub preset: Preset,
    // Trendlines
    pub length: usize,
    pub mult: f64,
    pub calc: SlopeCalc,
    // Oscillateur
    pub short_len: usize,
    pub long_len: usize,
    pub smooth: Smoothing,
    pub smooth_len: usize,
    // Filtres
    pub gate_on: bool,
    pub gate: f64,
    pub adx_on: bool,
    pub adx_len: usize,
    pub adx_min: f64,
    pub htf_on: bool,
    pub htf: Htf,
    pub vol_on: bool,
    pub vol_len: usize,
    pub vol_mult: f64,
    // Vector Candle
    pub vec_mode: VecMode,
    pub vec_lookback: usize,
    pub vec_avg: usize,
    pub vec200: f64,
    pub vec150: f64,
    pub climax: bool,
    // Divergences
    pub div_on: bool,
    pub div_hidden: bool,
    pub div_len: usize,
    pub min_div: usize,
    pub max_div: usize,
    pub min_osc_atr: f64,
    // Points de break affichés (affichage seulement : un côté masqué garde ses signaux, comme le Pine)
    pub dots_up: bool,
    pub dots_down: bool,
    pub signal_on: bool,
}

/// Groupes et inputs de v1 (générés depuis app.js : `v1_inputs`).
pub use crate::v1_inputs::{TBO_GROUPS as GROUPS, TBO_PARAMS as PARAMS};

impl Settings {
    /// Réglages du calcul lus dans les inputs de v1 (valeurs dans l'ordre de `PARAMS`).
    pub fn from_inputs(vals: &[f64]) -> Self {
        let p = Inputs { params: &PARAMS, vals };
        use Smoothing::*;
        Self {
            preset: [Preset::Custom, Preset::Auto, Preset::Scalping, Preset::DayTrade, Preset::Swing][p.pick("preset").min(4)],
            length: p.len("length"),
            mult: p.get("mult"),
            calc: [SlopeCalc::Atr, SlopeCalc::Stdev, SlopeCalc::Linreg][p.pick("calc").min(2)],
            short_len: p.len("shortLen"),
            long_len: p.len("longLen"),
            smooth: [None, Rma, Sma, Tma, Ema, Dema, Tema, Hma, Wma, Swma, Vwma][p.pick("smooth").min(10)],
            smooth_len: p.len("smoothLen"),
            gate_on: p.on("gate_on"),
            gate: p.get("gate"),
            adx_on: p.on("adx_on"),
            adx_len: p.len("adxLen"),
            adx_min: p.get("adxMin"),
            htf_on: p.on("htf_on"),
            // « Graphique », « Auto », puis les unités de temps de v1
            htf: match p.pick("htf") {
                0 => Htf::Chart,
                1 => Htf::Auto,
                k => Htf::Fixed(TF_SECS[(k - 2).min(TF_SECS.len() - 1)]),
            },
            vol_on: p.on("vol_on"),
            vol_len: p.len("volLen"),
            vol_mult: p.get("volMult"),
            vec_mode: [VecMode::Off, VecMode::V150, VecMode::V200][p.pick("vecMode").min(2)],
            vec_lookback: p.len("vecLb"),
            vec_avg: p.len("vecAvg"),
            vec200: p.get("vec200"),
            vec150: p.get("vec150"),
            climax: p.on("climax"),
            div_on: p.on("div_on"),
            div_hidden: p.on("div_hid"),
            div_len: p.len("divLen"),
            min_div: p.len("minDiv"),
            max_div: p.len("maxDiv"),
            min_osc_atr: p.get("minOscAtr"),
            dots_up: p.on("dotsU"),
            dots_down: p.on("dotsD"),
            signal_on: p.on("sig_on"),
        }
    }
}

/// Valeurs par défaut = celles de v1 (une seule source : `PARAMS`).
impl Default for Settings {
    fn default() -> Self {
        Self::from_inputs(&crate::inputs::defaults(&PARAMS))
    }
}

/// Valeurs effectives après le preset.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Active {
    pub preset: Preset,
    pub length: usize,
    pub mult: f64,
    pub calc: SlopeCalc,
    pub short_len: usize,
    pub long_len: usize,
    pub smooth: Smoothing,
    pub smooth_len: usize,
    pub gate: f64,
    pub adx_min: f64,
    pub div_len: usize,
    pub min_div: usize,
    pub max_div: usize,
}

impl Settings {
    pub fn active(&self, chart_seconds: i64) -> Active {
        let p = match self.preset {
            Preset::Auto if chart_seconds <= 900 => Preset::Scalping,
            Preset::Auto if chart_seconds <= 14_400 => Preset::DayTrade,
            Preset::Auto => Preset::Swing,
            other => other,
        };
        // (length, mult, short, long, smooth, gate, adxMin, divLen, minDiv, maxDiv)
        let v = match p {
            Preset::Scalping => (21, 1.25, 8, 28, 14, 0.35, 18.0, 6, 8, 80),
            Preset::DayTrade => (12, 1.05, 9, 21, 9, 0.25, 16.0, 5, 6, 60),
            Preset::Swing => (8, 1.0, 8, 18, 8, 0.22, 15.0, 4, 5, 40),
            _ => {
                return Active {
                    preset: p,
                    length: self.length,
                    mult: self.mult,
                    calc: self.calc,
                    short_len: self.short_len,
                    long_len: self.long_len,
                    smooth: self.smooth,
                    smooth_len: self.smooth_len,
                    gate: self.gate,
                    adx_min: self.adx_min,
                    div_len: self.div_len,
                    min_div: self.min_div,
                    max_div: self.max_div,
                }
            }
        };
        Active {
            preset: p,
            length: v.0,
            mult: v.1,
            calc: self.calc,
            short_len: v.2,
            long_len: v.3,
            smooth: Smoothing::Ema,
            smooth_len: v.4,
            gate: v.5,
            adx_min: v.6,
            div_len: v.7,
            min_div: v.8,
            max_div: v.9,
        }
    }
}

/// État d'une trendline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineState {
    /// Remplacée par un nouveau pivot.
    Replaced,
    /// Cassée par le prix.
    Broken,
    /// Encore active à la dernière bougie (pointillé).
    Open,
}

/// Trendline : du pivot (`i0`, `p0`) à (`i1`, `p1`). `upper` = trendline de sommets (résistance).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Trendline {
    pub upper: bool,
    pub i0: usize,
    pub p0: f64,
    pub i1: usize,
    pub p1: f64,
    pub state: LineState,
}

/// Cœur du TBO : cassures de trendlines et force du break.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Core {
    /// signalDelta = force haussière − force baissière.
    pub delta: Vec<f64>,
    pub break_up: Vec<bool>,
    pub break_down: Vec<bool>,
    /// Distance du close au-dessus de la trendline haute / sous la trendline basse.
    pub size_up: Vec<f64>,
    pub size_down: Vec<f64>,
    pub trendlines: Vec<Trendline>,
}

pub fn core(bars: &[Bar], a: &Active) -> Core {
    let n = bars.len();
    let len = a.length;
    let close: Vec<f64> = bars.iter().map(|b| b.close).collect();
    let hi: Vec<f64> = bars.iter().map(|b| b.high).collect();
    let lo: Vec<f64> = bars.iter().map(|b| b.low).collect();
    let slope: Vec<f64> = match a.calc {
        SlopeCalc::Atr => atr(bars, len).iter().map(|v| v / len as f64 * a.mult).collect(),
        SlopeCalc::Stdev => stdev(&close, len).iter().map(|v| v / len as f64 * a.mult).collect(),
        SlopeCalc::Linreg => {
            let idx: Vec<f64> = (0..n).map(|i| i as f64).collect();
            let xa = sma(&close.iter().enumerate().map(|(i, v)| v * i as f64).collect::<Vec<_>>(), len);
            let (xb, xc, sd) = (sma(&close, len), sma(&idx, len), stdev(&idx, len));
            (0..n)
                .map(|i| {
                    if sd[i] != 0.0 && !sd[i].is_nan() {
                        (xa[i] - xb[i] * xc[i]).abs() / (sd[i] * sd[i]) / 2.0 * a.mult
                    } else {
                        f64::NAN
                    }
                })
                .collect()
        }
    };
    let (sa, la) = (2.0 / (a.short_len as f64 + 1.0), 2.0 / (a.long_len as f64 + 1.0));
    let (mut upper, mut lower, mut sph, mut spl) = (0.0, 0.0, 0.0, 0.0);
    let (mut upos, mut dnos, mut bull, mut bear) = (0u8, 0u8, 0.0, 0.0);
    let (mut ph_i, mut pl_i): (Option<usize>, Option<usize>) = (None, None);
    let (mut seen_h, mut seen_l) = (false, false);
    let mut c = Core {
        delta: vec![f64::NAN; n],
        break_up: vec![false; n],
        break_down: vec![false; n],
        size_up: vec![f64::NAN; n],
        size_down: vec![f64::NAN; n],
        trendlines: vec![],
    };
    for i in 0..n {
        let ph = pivot(&hi, i, len, len, true);
        let pl = pivot(&lo, i, len, len, false);
        let s = if slope[i].is_finite() { slope[i] } else { 0.0 };
        let (pu, pd) = (upos, dnos);
        if let Some(p) = ph {
            if let Some(j) = ph_i {
                c.trendlines.push(Trendline {
                    upper: true,
                    i0: j,
                    p0: hi[j],
                    i1: i,
                    p1: upper - sph,
                    state: LineState::Replaced,
                });
            }
            sph = s;
            upper = hi[p] - sph * len as f64;
            ph_i = Some(p);
            seen_h = true;
        } else {
            upper -= sph;
        }
        if let Some(p) = pl {
            if let Some(j) = pl_i {
                c.trendlines.push(Trendline {
                    upper: false,
                    i0: j,
                    p0: lo[j],
                    i1: i,
                    p1: lower + spl,
                    state: LineState::Replaced,
                });
            }
            spl = s;
            lower = lo[p] + spl * len as f64;
            pl_i = Some(p);
            seen_l = true;
        } else {
            lower += spl;
        }
        if !seen_h || !seen_l {
            upos = 0;
            dnos = 0;
            continue;
        }
        upos = if ph.is_some() {
            0
        } else if close[i] > upper {
            1
        } else {
            upos
        };
        dnos = if pl.is_some() {
            0
        } else if close[i] < lower {
            1
        } else {
            dnos
        };
        if upos > pu {
            c.break_up[i] = true;
            bull += (close[i] - upper - bull) * sa;
            if let Some(j) = ph_i.take() {
                c.trendlines.push(Trendline {
                    upper: true,
                    i0: j,
                    p0: hi[j],
                    i1: i,
                    p1: upper,
                    state: LineState::Broken,
                });
            }
        } else if upos == 1 {
            bull += ((close[i] - upper).max(0.0) - bull) * la;
        } else {
            bull -= bull * la;
        }
        if dnos > pd {
            c.break_down[i] = true;
            bear += (lower - close[i] - bear) * sa;
            if let Some(j) = pl_i.take() {
                c.trendlines.push(Trendline {
                    upper: false,
                    i0: j,
                    p0: lo[j],
                    i1: i,
                    p1: lower,
                    state: LineState::Broken,
                });
            }
        } else if dnos == 1 {
            bear += ((lower - close[i]).max(0.0) - bear) * la;
        } else {
            bear -= bear * la;
        }
        c.delta[i] = bull - bear;
        c.size_up[i] = close[i] - upper;
        c.size_down[i] = lower - close[i];
    }
    if n > 0 {
        if let Some(j) = ph_i {
            c.trendlines.push(Trendline {
                upper: true,
                i0: j,
                p0: hi[j],
                i1: n - 1,
                p1: upper,
                state: LineState::Open,
            });
        }
        if let Some(j) = pl_i {
            c.trendlines.push(Trendline {
                upper: false,
                i0: j,
                p0: lo[j],
                i1: n - 1,
                p1: lower,
                state: LineState::Open,
            });
        }
    }
    c
}

/// Lissage de la ligne Signal.
pub fn smooth(x: &[f64], t: Smoothing, len: usize, volume: &[f64]) -> Vec<f64> {
    let e1 = || ema(x, len);
    match t {
        Smoothing::None => x.to_vec(),
        Smoothing::Ema => e1(),
        Smoothing::Rma => rma(x, len),
        Smoothing::Sma => sma(x, len),
        Smoothing::Tma => sma(&sma(x, len), len),
        Smoothing::Wma => wma(x, len),
        Smoothing::Dema => {
            let e = e1();
            let e2 = ema(&e, len);
            e.iter().zip(&e2).map(|(a, b)| 2.0 * a - b).collect()
        }
        Smoothing::Tema => {
            let e = e1();
            let e2 = ema(&e, len);
            let e3 = ema(&e2, len);
            (0..x.len()).map(|i| 3.0 * e[i] - 3.0 * e2[i] + e3[i]).collect()
        }
        Smoothing::Hma => {
            let half = (len / 2).max(1);
            let sq = ((len as f64).sqrt().round() as usize).max(1);
            let (a, b) = (wma(x, half), wma(x, len));
            wma(&a.iter().zip(&b).map(|(a, b)| 2.0 * a - b).collect::<Vec<_>>(), sq)
        }
        Smoothing::Swma => (0..x.len())
            .map(|i| {
                if i < 3 {
                    f64::NAN
                } else {
                    (x[i - 3] + 2.0 * x[i - 2] + 2.0 * x[i - 1] + x[i]) / 6.0
                }
            })
            .collect(),
        Smoothing::Vwma => {
            let pv = sma(&x.iter().zip(volume).map(|(v, w)| v * w).collect::<Vec<_>>(), len);
            let vv = sma(volume, len);
            pv.iter()
                .zip(&vv)
                .map(|(p, v)| if *v != 0.0 && !v.is_nan() { p / v } else { f64::NAN })
                .collect()
        }
    }
}

/// Type de point de break.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BreakKind {
    /// Bloqué par un filtre (petit point pâle).
    Filtered,
    /// Break validé (point or / cyan) → signal.
    Valid,
    /// Break validé + Vector 200+ (point jaune / mauve avec glow) → signal.
    Vector,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Break {
    pub index: usize,
    pub up: bool,
    pub kind: BreakKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Div {
    pub prev: usize,
    pub pivot: usize,
    pub bull: bool,
    pub hidden: bool,
}

/// Unité de temps du filtre HTF (pour le tableau de statut).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HtfUsed {
    Off,
    Chart,
    Seconds(i64),
}

/// Dernier événement (ligne « Dernier » du tableau de statut).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LastEvent {
    Break { up: bool, vector: bool },
    Divergence { bull: bool, hidden: bool },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Tbo {
    pub active: Active,
    pub core: Core,
    pub signal: Vec<f64>,
    pub adx: Vec<f64>,
    /// Niveau Vector de chaque bougie : 0, 1 (150+) ou 2 (200+).
    pub vector: Vec<u8>,
    pub breaks: Vec<Break>,
    pub buy: Vec<usize>,
    pub sell: Vec<usize>,
    pub divs: Vec<Div>,
    pub htf: HtfUsed,
    pub last: Option<LastEvent>,
}

fn htf_auto(sec: i64) -> i64 {
    match sec {
        s if s <= 300 => 3_600,
        s if s <= 1_800 => 14_400,
        s if s <= 14_400 => 86_400,
        s if s <= 172_800 => 604_800,
        _ => MONTH_SECONDS,
    }
}

/// 🔔 Les 10 alertes du Pine (noms des `alertcondition`), dans l'ordre de `Tbo::alerts`.
pub const ALERTS: [&str; 10] = [
    "Break haussier validé",
    "Break baissier validé",
    "Breakout + Vector 200+ haussier",
    "Breakout + Vector 200+ baissier",
    "Vector Candle 200+ haussière",
    "Vector Candle 200+ baissière",
    "Divergence régulière haussière",
    "Divergence régulière baissière",
    "Divergence cachée haussière",
    "Divergence cachée baissière",
];

impl Tbo {
    /// 🔔 Signaux des alertes (`ALERTS`) : 1 sur les bougies où le signal arrive, 0 ailleurs. Une divergence arrive à
    /// la bougie qui confirme son pivot (pivot + longueur), comme dans le Pine.
    pub fn alerts(&self, bars: &[Bar]) -> Vec<(&'static str, Vec<f64>)> {
        let n = self.vector.len();
        let mut out: Vec<(&'static str, Vec<f64>)> = ALERTS.iter().map(|a| (*a, vec![0.0; n])).collect();
        for b in self.breaks.iter().filter(|b| b.index < n) {
            if b.kind != BreakKind::Filtered {
                out[usize::from(!b.up)].1[b.index] = 1.0;
            }
            if b.kind == BreakKind::Vector {
                out[2 + usize::from(!b.up)].1[b.index] = 1.0;
            }
        }
        for (i, b) in bars.iter().enumerate().take(n).filter(|(i, _)| self.vector[*i] == 2) {
            out[4 + usize::from(b.close < b.open)].1[i] = 1.0;
        }
        for dv in &self.divs {
            let at = dv.pivot + self.active.div_len;
            if at < n {
                out[6 + usize::from(!dv.bull) + 2 * usize::from(dv.hidden)].1[at] = 1.0;
            }
        }
        out
    }
}

/// Valeur HTF vue par chaque bougie de la chart, comme `request.security(..., lookahead_off)` de TradingView sur
/// l'historique : la bougie HTF n'est connue qu'à sa clôture (dernière bougie de la chart dans sa période) ; avant, c'est
/// la valeur de la bougie HTF d'avant. La toute dernière bougie (en cours) voit la valeur en cours.
fn htf_values(bars: &[Bar], hb: &[Bar], hs: i64, hdelta: &[f64]) -> Vec<f64> {
    let n = bars.len();
    let bucket: Vec<i64> = bars.iter().map(|b| bucket_start(b.time, hs)).collect();
    let mut j = 0;
    (0..n)
        .map(|i| {
            while j + 1 < hb.len() && hb[j + 1].time <= bucket[i] {
                j += 1;
            }
            let closes = i + 1 == n || bucket[i + 1] != bucket[i];
            match (closes, j) {
                (true, _) => hdelta[j],
                (false, 0) => f64::NAN,
                (false, _) => hdelta[j - 1],
            }
        })
        .collect()
}

/// Rang d'un événement dans une même bougie, comme l'ordre des `lastEvt :=` du Pine (le dernier gagne).
fn rank(e: LastEvent) -> u8 {
    match e {
        LastEvent::Break { up: true, .. } => 0,
        LastEvent::Break { up: false, .. } => 1,
        LastEvent::Divergence { bull: true, hidden: false } => 2,
        LastEvent::Divergence { bull: false, hidden: false } => 3,
        LastEvent::Divergence { bull: true, hidden: true } => 4,
        LastEvent::Divergence { bull: false, hidden: true } => 5,
    }
}

pub fn compute(bars: &[Bar], chart_seconds: i64, s: &Settings) -> Tbo {
    let n = bars.len();
    let a = s.active(chart_seconds);
    let vol: Vec<f64> = bars.iter().map(|b| b.volume).collect();
    let base = core(bars, &a);
    let d = &base.delta;
    let signal = if s.signal_on {
        smooth(d, a.smooth, a.smooth_len, &vol)
    } else {
        vec![f64::NAN; n]
    };
    let atr14 = atr(bars, 14);
    let adx = adx(bars, s.adx_len);
    let vavg = sma(&vol, s.vol_len);

    // Vector 150+ / 200+ (climax = spread × volume record)
    let va = sma(&vol, s.vec_avg);
    let spread: Vec<f64> = bars.iter().map(|b| b.range() * b.volume).collect();
    let smax = highest(&spread, s.vec_avg);
    let mut vector = vec![0u8; n];
    for i in 1..n {
        let (avg, mx) = (va[i - 1], smax[i - 1]);
        if avg.is_nan() || avg <= 0.0 {
            continue;
        }
        let v200 = vol[i] >= avg * s.vec200 || (s.climax && mx > 0.0 && spread[i] >= mx);
        vector[i] = if v200 {
            2
        } else if vol[i] >= avg * s.vec150 {
            1
        } else {
            0
        };
    }

    // filtre HTF : pour chaque bougie, le signalDelta de la bougie HTF qui la contient (bougie en cours)
    let (htf, htf_delta): (HtfUsed, Option<Vec<f64>>) = match (s.htf_on, s.htf) {
        (false, _) => (HtfUsed::Off, None),
        (true, Htf::Chart) => (HtfUsed::Chart, Some(d.clone())),
        (true, h) => {
            let hs = if let Htf::Fixed(x) = h { x } else { htf_auto(chart_seconds) };
            let mut hd = None;
            if hs == chart_seconds {
                hd = Some(d.clone());
            } else if hs > chart_seconds {
                let hb = aggregate(bars, hs);
                // comme request.security du Pine : en preset Auto, le HTF prend le preset de sa propre unité de temps
                let ha = s.active(hs);
                if hb.len() > ha.length * 2 + 5 {
                    hd = Some(htf_values(bars, &hb, hs, &core(&hb, &ha).delta));
                }
            }
            (HtfUsed::Seconds(hs), hd)
        }
    };

    let need = match s.vec_mode {
        VecMode::Off => 0,
        VecMode::V150 => 1,
        VecMode::V200 => 2,
    };
    let (mut breaks, mut buy, mut sell) = (vec![], vec![], vec![]);
    // dernier événement : la bougie la plus récente, et dans une même bougie l'ordre du Pine (breaks, puis REG ▲ ▼, HID ▲ ▼)
    let mut last: Option<(usize, u8, LastEvent)> = None;
    let mut note = |i: usize, e: LastEvent| {
        if last.is_none_or(|(j, r, _)| (i, rank(e)) >= (j, r)) {
            last = Some((i, rank(e), e));
        }
    };
    for i in 0..n {
        if !d[i].is_finite() || (!base.break_up[i] && !base.break_down[i]) {
            continue;
        }
        let trend_ok = !s.adx_on || adx[i] >= a.adx_min;
        let vol_ok = !s.vol_on || vavg[i].is_nan() || vavg[i] == 0.0 || vol[i] >= vavg[i] * s.vol_mult;
        let (mut h_bull, mut h_bear) = (true, true);
        if let Some(hd) = &htf_delta {
            let v = hd[i];
            h_bull = !v.is_finite() || v > 0.0;
            h_bear = !v.is_finite() || v < 0.0;
        }
        let (mut v_up, mut v_dn) = (false, false);
        for k in 0..s.vec_lookback.min(i + 1) {
            let b = &bars[i - k];
            let ok = if need == 1 { vector[i - k] >= 1 } else { vector[i - k] == 2 };
            if ok && b.close >= b.open {
                v_up = true;
            }
            if ok && b.close < b.open {
                v_dn = true;
            }
        }
        for up in [true, false] {
            // masquer les points ▲ ▼ ne change que l'affichage (comme le Pine) : les signaux restent
            if (up && !base.break_up[i]) || (!up && !base.break_down[i]) {
                continue;
            }
            let size = if up { base.size_up[i] } else { base.size_down[i] };
            let size_ok = !s.gate_on || size >= atr14[i] * a.gate;
            let brk = trend_ok && vol_ok && size_ok;
            let h_ok = !s.htf_on || if up { h_bull } else { h_bear };
            let v_rec = if up { v_up } else { v_dn };
            let fin = brk && h_ok && (need == 0 || v_rec);
            let conf = brk && h_ok && v_rec;
            let kind = if conf {
                BreakKind::Vector
            } else if fin {
                BreakKind::Valid
            } else {
                BreakKind::Filtered
            };
            breaks.push(Break { index: i, up, kind });
            if fin {
                if up { &mut buy } else { &mut sell }.push(i);
                note(i, LastEvent::Break { up, vector: conf });
            }
        }
    }

    // divergences sur signalDelta (pivots div_len, amplitude normalisée par l'ATR)
    let mut divs = vec![];
    // calculées même si elles sont masquées (comme le Pine : le dernier événement et les alertes les voient)
    {
        let (mut h0, mut l0): (Option<usize>, Option<usize>) = (None, None);
        for (i, &a14) in atr14.iter().enumerate() {
            let an = if a14.is_finite() { a14 } else { 0.0 };
            let ok = |p: usize, c: usize| {
                let mv = if an > 0.0 { (d[c] - d[p]).abs() / an } else { 0.0 };
                let bb = c - p;
                bb >= a.min_div && bb <= a.max_div && mv >= s.min_osc_atr
            };
            if let Some(ph) = pivot(d, i, a.div_len, a.div_len, true) {
                if let Some(p) = h0.replace(ph).filter(|&p| ok(p, ph)) {
                    let (hc, hp) = (bars[ph].high, bars[p].high);
                    let hidden = if hc > hp && d[ph] < d[p] {
                        Some(false)
                    } else if s.div_hidden && hc < hp && d[ph] > d[p] {
                        Some(true)
                    } else {
                        None
                    };
                    if let Some(hidden) = hidden {
                        divs.push(Div {
                            prev: p,
                            pivot: ph,
                            bull: false,
                            hidden,
                        });
                        note(i, LastEvent::Divergence { bull: false, hidden });
                    }
                }
            }
            if let Some(pl) = pivot(d, i, a.div_len, a.div_len, false) {
                if let Some(p) = l0.replace(pl).filter(|&p| ok(p, pl)) {
                    let (lc, lp) = (bars[pl].low, bars[p].low);
                    let hidden = if lc < lp && d[pl] > d[p] {
                        Some(false)
                    } else if s.div_hidden && lc > lp && d[pl] < d[p] {
                        Some(true)
                    } else {
                        None
                    };
                    if let Some(hidden) = hidden {
                        divs.push(Div {
                            prev: p,
                            pivot: pl,
                            bull: true,
                            hidden,
                        });
                        note(i, LastEvent::Divergence { bull: true, hidden });
                    }
                }
            }
        }
    }

    Tbo {
        active: a,
        core: base,
        signal,
        adx,
        vector,
        breaks,
        buy,
        sell,
        divs,
        htf,
        last: last.map(|x| x.2),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets() {
        let s = Settings {
            preset: Preset::Auto,
            ..Settings::default()
        };
        assert_eq!(s.active(900).length, 21);
        assert_eq!(s.active(1800).length, 12);
        assert_eq!(s.active(86_400).length, 8);
        assert_eq!(s.active(86_400).smooth, Smoothing::Ema);
        assert_eq!(Settings::default().preset, Preset::DayTrade);
        let custom = Settings {
            preset: Preset::Custom,
            ..Settings::default()
        };
        assert_eq!(custom.active(60).length, 2);
        assert_eq!(htf_auto(1800), 14_400);
        assert_eq!(htf_auto(259_200), MONTH_SECONDS);
    }

    #[test]
    fn htf_sans_regarder_dans_le_futur() {
        // bougies 1h, HTF 4h : avant la clôture d'une 4h on voit la 4h d'avant ; à sa clôture, la 4h elle-même ;
        // la dernière bougie (en cours) voit la 4h en cours
        let bars: Vec<Bar> = (0..10)
            .map(|i| Bar {
                time: i * 3_600,
                open: 1.0,
                high: 1.0,
                low: 1.0,
                close: 1.0,
                volume: 1.0,
            })
            .collect();
        let hb = aggregate(&bars, 14_400);
        let v = htf_values(&bars, &hb, 14_400, &[10.0, 20.0, 30.0]);
        assert!(v[..3].iter().all(|x| x.is_nan()));
        assert_eq!(&v[3..], &[10.0, 10.0, 10.0, 10.0, 20.0, 20.0, 30.0]);
    }

    #[test]
    fn dernier_evenement_dans_l_ordre_du_pine() {
        let brk = LastEvent::Break { up: false, vector: false };
        let reg = LastEvent::Divergence { bull: true, hidden: false };
        let hid = LastEvent::Divergence { bull: false, hidden: true };
        assert!(rank(brk) < rank(reg) && rank(reg) < rank(hid));
    }

    #[test]
    fn cassure_de_resistance() {
        // sommet à 10 puis creux : le prix repasse au-dessus de la trendline descendante → break haussier
        let closes = [5.0, 7.0, 10.0, 7.0, 5.0, 4.0, 3.0, 2.0, 3.0, 4.0, 12.0, 13.0];
        let bars: Vec<Bar> = closes
            .iter()
            .enumerate()
            .map(|(i, &c)| Bar {
                time: i as i64,
                open: c,
                high: c + 0.1,
                low: c - 0.1,
                close: c,
                volume: 1.0,
            })
            .collect();
        let custom = Settings {
            preset: Preset::Custom,
            ..Settings::default()
        };
        let c = core(&bars, &custom.active(60));
        let i = c.break_up.iter().position(|&b| b).expect("un break haussier");
        assert!(c.delta[i] > 0.0);
        assert!(c.trendlines.iter().any(|t| t.upper && t.state == LineState::Broken));
    }
}
