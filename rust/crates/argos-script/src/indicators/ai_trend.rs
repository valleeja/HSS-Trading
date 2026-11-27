//! Argos AI Trend V Pro — AI Trend Index (RSI + Stoch + Momentum), ligne signal, divergences (+ 🤑), divergences multi-unités de temps,
//! zones extrêmes, boîte HTF, points d'achat / vente filtrés par la tendance EMA.
//! Même logique que le script Pine personnel de Jee (créé par lui avec l'IA) : presets, momentum normalisé, divergences,
//! filtre des signaux, zones Very Red / Very Blue, boîte HTF). Les couleurs (dégradé, étiquettes) sont de l'affichage :
//! elles restent dans la couche chart.

use crate::inputs::{Inputs, TF_SECS};
use crate::ta::{ema, highest, lowest, pivot, rma, rsi, sma, wma};
use argos_core::{aggregate, Bar};

/// Type de moyenne (menu « Smoothing Type » / « Type de moyenne »).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaType {
    Sma,
    Ema,
    Wma,
    Rma,
}

fn ma(src: &[f64], len: usize, t: MaType) -> Vec<f64> {
    match t {
        MaType::Sma => sma(src, len),
        MaType::Ema => ema(src, len),
        MaType::Wma => wma(src, len),
        MaType::Rma => rma(src, len),
    }
}

/// Preset (Auto : ≤ 15m Scalping · ≤ 4h Day Trade · sinon Swing).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Preset {
    Custom,
    Auto,
    Scalping,
    DayTrade,
    Swing,
}

/// Longueurs de l'oscillateur et des divergences (remplacées par le preset s'il n'est pas « Personnalisé »).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lengths {
    pub rsi: usize,
    pub stoch: usize,
    pub smooth_k: usize,
    pub momentum: usize,
    pub smoothing: usize,
    pub signal: usize,
    pub pivot_left: usize,
    pub pivot_right: usize,
    pub max_distance: usize,
}

impl Preset {
    fn lengths(self, chart_seconds: i64, custom: Lengths) -> Lengths {
        let p = match self {
            Preset::Custom => return custom,
            Preset::Auto if chart_seconds <= 900 => Preset::Scalping,
            Preset::Auto if chart_seconds <= 14_400 => Preset::DayTrade,
            Preset::Auto => Preset::Swing,
            other => other,
        };
        // valeurs du Pine (TradingView)
        let l = |rsi, stoch, smooth_k, momentum, smoothing, pivot_left, pivot_right, max_distance, signal| Lengths {
            rsi,
            stoch,
            smooth_k,
            momentum,
            smoothing,
            signal,
            pivot_left,
            pivot_right,
            max_distance,
        };
        match p {
            Preset::Scalping => l(7, 7, 2, 8, 2, 2, 1, 60, 5),
            Preset::DayTrade => l(10, 10, 3, 12, 3, 2, 2, 100, 9),
            _ => l(14, 14, 3, 14, 4, 3, 3, 150, 12),
        }
    }
}

/// Un emplacement de divergences multi-unités de temps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MtfSlot {
    pub on: bool,
    /// Unité de temps en secondes (ex. 18 000 = 5h).
    pub seconds: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    pub preset: Preset,
    pub lengths: Lengths,
    pub smooth_type: MaType,
    pub overbought: f64,
    pub oversold: f64,
    pub extreme_high: f64,
    pub extreme_low: f64,
    pub midline: f64,
    /// Zones « Very Red » / « Very Blue » (fond des bougies extrêmes).
    pub very_red: f64,
    pub very_blue: f64,
    pub signal_show: bool,
    pub signal_type: MaType,
    /// Filtre des signaux : 0 Aucun, 1 Midline, 2 Zones.
    pub signal_filter: usize,
    pub div_hidden: bool,
    pub min_move: f64,
    pub mtf: [MtfSlot; 4],
    /// Divergences MTF : pivots gauche / droite, distance min / max entre pivots (bougies de l'UT du slot).
    pub mtf_pivots: (usize, usize),
    pub mtf_range: (usize, usize),
    pub htf_box: bool,
    /// `None` = Auto (l'UT de la chart, comme le Pine) ; `Some(sec)` = unité de temps fixe.
    pub htf_fixed: Option<i64>,
    pub ema_filter: bool,
    pub ema_fast: usize,
    pub ema_slow: usize,
    /// Sections 2 et 3 du Pine : Volume Vector, EMAs, patterns, confirmation, Ignition.
    pub volume: super::ai_trend_volume::Settings,
}

/// Groupes et inputs de v1 (générés depuis app.js : `v1_inputs`).
pub use crate::v1_inputs::{AI_TREND_GROUPS as GROUPS, AI_TREND_PARAMS as PARAMS};

impl Settings {
    /// Réglages du calcul lus dans les inputs de v1 (valeurs dans l'ordre de `PARAMS`).
    pub fn from_inputs(vals: &[f64]) -> Self {
        let p = Inputs { params: &PARAMS, vals };
        let tf = |k: &str| TF_SECS[p.pick(k).min(TF_SECS.len() - 1)];
        let ma = |i: usize| [MaType::Sma, MaType::Ema, MaType::Wma, MaType::Rma][i.min(3)];
        let slot = |n: usize| MtfSlot {
            on: p.on(&format!("m{n}_on")),
            seconds: tf(&format!("m{n}_tf")),
        };
        Self {
            preset: [Preset::Custom, Preset::Auto, Preset::Scalping, Preset::DayTrade, Preset::Swing][p.pick("preset").min(4)],
            lengths: Lengths {
                rsi: p.len("rsi_len"),
                stoch: p.len("stoch_len"),
                smooth_k: p.len("smooth_k"),
                momentum: p.len("mom_len"),
                smoothing: p.len("smooth_len"),
                signal: p.len("sig_len"),
                pivot_left: p.len("piv_l"),
                pivot_right: p.len("piv_r"),
                max_distance: p.len("max_dist"),
            },
            smooth_type: ma(p.pick("smooth_type")),
            overbought: p.get("ob"),
            oversold: p.get("os"),
            extreme_high: p.get("xh"),
            extreme_low: p.get("xl"),
            midline: p.get("mid"),
            very_red: p.get("vr"),
            very_blue: p.get("vb"),
            signal_show: p.on("sig_show"),
            // « MME » (exponentielle) ou « MMS » (simple), comme v1
            signal_type: if p.pick("sig_type") == 0 { MaType::Ema } else { MaType::Sma },
            signal_filter: p.pick("sig_filter"),
            div_hidden: p.on("div_hid"),
            min_move: p.get("min_move"),
            mtf: [slot(1), slot(2), slot(3), slot(4)],
            mtf_pivots: (p.len("mtf_pl"), p.len("mtf_pr")),
            mtf_range: (p.len("mtf_rmin"), p.len("mtf_rmax")),
            htf_box: p.on("htf_box"),
            htf_fixed: (p.pick("htf_mode") == 1).then(|| tf("htf_tf")),
            ema_filter: p.on("ema_filter"),
            ema_fast: p.len("ema_fast"),
            ema_slow: p.len("ema_slow"),
            volume: super::ai_trend_volume::Settings::from_inputs(&p),
        }
    }
}

/// Valeurs par défaut = celles de v1 (une seule source : `PARAMS`).
impl Default for Settings {
    fn default() -> Self {
        Self::from_inputs(&crate::inputs::defaults(&PARAMS))
    }
}

/// AI Trend Index seul (RSI + Stoch + Momentum, lissé).
pub fn index(bars: &[Bar], l: Lengths, smooth_type: MaType) -> Vec<f64> {
    let n = bars.len();
    let close: Vec<f64> = bars.iter().map(|b| b.close).collect();
    let r = rsi(&close, l.rsi);
    let h = highest(&bars.iter().map(|b| b.high).collect::<Vec<_>>(), l.stoch);
    let lo = lowest(&bars.iter().map(|b| b.low).collect::<Vec<_>>(), l.stoch);
    let raw_st: Vec<f64> = (0..n)
        .map(|i| {
            let range = h[i] - lo[i];
            if range != 0.0 && !range.is_nan() {
                (close[i] - lo[i]) / range * 100.0
            } else {
                50.0
            }
        })
        .collect();
    let st = sma(&raw_st, l.smooth_k.max(1));
    let mom: Vec<f64> = (0..n)
        .map(|i| if i >= l.momentum { close[i] - close[i - l.momentum] } else { f64::NAN })
        .collect();
    // momentum ramené à 0..100 entre son plus bas et son plus haut des 2 × L dernières bougies (comme le Pine)
    let (mh, ml) = (highest(&mom, l.momentum * 2), lowest(&mom, l.momentum * 2));
    let mom_n: Vec<f64> = (0..n)
        .map(|i| {
            if !mom[i].is_finite() || !mh[i].is_finite() || !ml[i].is_finite() {
                f64::NAN
            } else if mh[i] != ml[i] {
                (mom[i] - ml[i]) / (mh[i] - ml[i]) * 100.0
            } else {
                50.0
            }
        })
        .collect();
    let raw: Vec<f64> = (0..n).map(|i| (r[i] + st[i] + mom_n[i]) / 3.0).collect();
    if l.smoothing > 1 {
        ma(&raw, l.smoothing, smooth_type)
    } else {
        raw
    }
}

/// Une divergence : pivot d'avant, pivot actuel, bougie où elle est confirmée.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Div {
    pub prev: usize,
    pub pivot: usize,
    pub confirmed: usize,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Divs {
    /// 🤑 sous la ligne
    pub bull: Vec<Div>,
    /// 🤑 au-dessus de la ligne
    pub bear: Vec<Div>,
    pub hidden_bull: Vec<Div>,
    pub hidden_bear: Vec<Div>,
}

/// Divergences de l'AI Trend : pivots avec `left` bougies à gauche et `right` à droite, distance max entre pivots, mouvement minimum.
pub fn find_divs(osc: &[f64], bars: &[Bar], left: usize, right: usize, max_distance: usize, min_move: f64, hidden: bool) -> Divs {
    let mut d = Divs::default();
    let (mut pt, mut pb): (Option<usize>, Option<usize>) = (None, None);
    let ok = |a: usize, b: usize| b - a >= 1 && b - a <= max_distance && (osc[b] - osc[a]).abs() >= min_move;
    for i in 0..osc.len() {
        if let Some(t) = pivot(osc, i, left, right, true) {
            if let Some(p) = pt.filter(|&p| ok(p, t)) {
                let div = Div {
                    prev: p,
                    pivot: t,
                    confirmed: i,
                };
                if osc[t] < osc[p] && bars[t].high > bars[p].high {
                    d.bear.push(div);
                } else if hidden && osc[t] > osc[p] && bars[t].high < bars[p].high {
                    d.hidden_bear.push(div);
                }
            }
            pt = Some(t);
        }
        if let Some(b) = pivot(osc, i, left, right, false) {
            if let Some(p) = pb.filter(|&p| ok(p, b)) {
                let div = Div {
                    prev: p,
                    pivot: b,
                    confirmed: i,
                };
                if osc[b] > osc[p] && bars[b].low < bars[p].low {
                    d.bull.push(div);
                } else if hidden && osc[b] < osc[p] && bars[b].low > bars[p].low {
                    d.hidden_bull.push(div);
                }
            }
            pb = Some(b);
        }
    }
    d
}

/// Point de divergence d'une unité de temps plus petite (slot MTF), sur la bougie de la chart qui la contient.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MtfDot {
    /// Emplacement 1 à 4.
    pub slot: usize,
    pub bull: bool,
    pub index: usize,
}

/// Zone de la boîte HTF.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HtfZone {
    Buy,
    Sell,
    Neutral,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HtfBox {
    pub seconds: i64,
    pub value: f64,
    pub zone: HtfZone,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct AiTrend {
    pub ai: Vec<f64>,
    pub signal: Vec<f64>,
    pub divs: Divs,
    pub mtf: Vec<MtfDot>,
    pub buy: Vec<usize>,
    pub sell: Vec<usize>,
    /// Bougies en zone extrême (fond rouge au-dessus du Very Red, bleu sous le Very Blue).
    pub extreme_high: Vec<usize>,
    pub extreme_low: Vec<usize>,
    /// Preset utilisé (« Day Trade »…), pour la boîte HTF.
    pub preset: &'static str,
    pub htf: Option<HtfBox>,
    /// Tendance EMA rapide ≥ lente à la dernière bougie.
    pub trend_bull: bool,
    /// Flèches du Pine (croise la survente vers le haut / le surachat vers le bas, filtre EMA) : pour les alertes.
    pub arrow_buy: Vec<usize>,
    pub arrow_sell: Vec<usize>,
    /// Sections 2 et 3 du Pine (Volume Vector, EMAs, patterns, Ignition 🚀).
    pub volume: super::ai_trend_volume::Volume,
}

/// Bougies des UT plus petites que la chart, par durée en secondes (une par slot MTF actif), chargées par la fenêtre.
/// 🔔 Les 17 alertes du Pine (noms des `alertcondition`), dans l'ordre de `AiTrend::alerts`.
pub const ALERTS: [&str; 17] = [
    "Bullish Pattern Detected",
    "Bearish Pattern Detected",
    "Bullish Abandoned Baby",
    "Bearish Abandoned Baby",
    "Bull Gold Vector",
    "Bear Gold Vector",
    "Divergence Haussière Régulière",
    "Divergence Baissière Régulière",
    "Divergence Haussière Cachée",
    "Divergence Baissière Cachée",
    "Argos Signal BUY (croisement)",
    "Argos Signal SELL (croisement)",
    "Argos Div Bull MTF",
    "Argos Div Bear MTF",
    "Argos Flèche Verte (Buy)",
    "Argos Fusée (Ignition)",
    "Argos Ignition - Flèche Verte + Fusée",
];

impl AiTrend {
    /// 🔔 Signaux des alertes (`ALERTS`) : 1 sur les bougies où le signal arrive, 0 ailleurs. Une divergence arrive à la
    /// bougie qui confirme son pivot, comme dans le Pine.
    pub fn alerts(&self, bars: &[Bar]) -> Vec<(&'static str, Vec<f64>)> {
        let n = self.ai.len();
        let at = |idx: &mut dyn Iterator<Item = usize>| {
            let mut v = vec![0.0; n];
            for i in idx.filter(|&i| i < n) {
                v[i] = 1.0;
            }
            v
        };
        let v = &self.volume;
        let gold = |up: bool| {
            (0..n.min(v.tier.len()).min(bars.len())).filter(move |&i| v.tier[i] == super::ai_trend_volume::Tier::Gold && (bars[i].close >= bars[i].open) == up)
        };
        let conf = |l: &[Div]| l.iter().map(|d| d.confirmed).collect::<Vec<_>>();
        let mtf = |bull: bool| self.mtf.iter().filter(move |m| m.bull == bull).map(|m| m.index);
        let series = [
            at(&mut v.bull_patterns.iter().map(|p| p.0)),
            at(&mut v.bear_patterns.iter().map(|p| p.0)),
            at(&mut v.baby_bull.iter().copied()),
            at(&mut v.baby_bear.iter().copied()),
            at(&mut gold(true)),
            at(&mut gold(false)),
            at(&mut conf(&self.divs.bull).into_iter()),
            at(&mut conf(&self.divs.bear).into_iter()),
            at(&mut conf(&self.divs.hidden_bull).into_iter()),
            at(&mut conf(&self.divs.hidden_bear).into_iter()),
            at(&mut self.buy.iter().copied()),
            at(&mut self.sell.iter().copied()),
            at(&mut mtf(true)),
            at(&mut mtf(false)),
            at(&mut self.arrow_buy.iter().copied()),
            at(&mut v.ignition.iter().copied()),
            at(&mut self.arrow_buy.iter().chain(&v.ignition).copied()),
        ];
        ALERTS.iter().copied().zip(series).collect()
    }
}

pub type LowerBars = std::collections::HashMap<i64, Vec<Bar>>;

/// Durées (secondes) des slots MTF actifs plus petites que la chart : celles que la fenêtre doit charger.
/// Comme le Pine (`ignore_invalid_timeframe`), un slot plus grand ou égal à la chart ne donne rien.
pub fn lower_needed(s: &Settings, chart_seconds: i64) -> Vec<i64> {
    let mut v: Vec<i64> = s.mtf.iter().filter(|m| m.on && m.seconds < chart_seconds).map(|m| m.seconds).collect();
    v.sort_unstable();
    v.dedup();
    v
}

/// `f_mtfDivs` du Pine sur les bougies d'un slot : (haussière, baissière) à chaque bougie, à la confirmation du pivot.
fn lower_divs(lower: &[Bar], s: &Settings, l: Lengths) -> Vec<(bool, bool)> {
    let o = index(lower, l, s.smooth_type);
    let (pl, pr) = s.mtf_pivots;
    let (rmin, rmax) = s.mtf_range;
    let mut out = vec![(false, false); lower.len()];
    // dernier pivot trouvé (bougie de confirmation, valeur de l'oscillateur, prix) pour les creux et les sommets
    let (mut last_lo, mut last_hi): (Option<(usize, f64, f64)>, Option<(usize, f64, f64)>) = (None, None);
    for (i, slot) in out.iter_mut().enumerate() {
        let in_range = |last: Option<(usize, f64, f64)>| last.is_some_and(|(j, _, _)| (rmin..=rmax).contains(&(i - 1 - j)));
        if let Some(k) = pivot(&o, i, pl, pr, false) {
            slot.0 = in_range(last_lo) && last_lo.is_some_and(|(_, po, pp)| o[k] > po && lower[k].low < pp);
            last_lo = Some((i, o[k], lower[k].low));
        }
        if let Some(k) = pivot(&o, i, pl, pr, true) {
            slot.1 = in_range(last_hi) && last_hi.is_some_and(|(_, po, pp)| o[k] < po && lower[k].high > pp);
            last_hi = Some((i, o[k], lower[k].high));
        }
    }
    out
}

/// Points MTF : la bougie de la chart a un point si une des bougies du slot qu'elle contient a une divergence (`array.includes`).
fn mtf_dots(bars: &[Bar], chart_seconds: i64, s: &Settings, lower: &LowerBars) -> Vec<MtfDot> {
    let mut dots = vec![];
    for (k, slot) in s.mtf.iter().enumerate().filter(|(_, m)| m.on && m.seconds < chart_seconds) {
        let Some(lb) = lower.get(&slot.seconds).filter(|lb| !lb.is_empty()) else {
            continue;
        };
        // comme request.security_lower_tf : en preset Auto, la petite UT prend le preset de sa propre unité de temps
        let divs = lower_divs(lb, s, s.preset.lengths(slot.seconds, s.lengths));
        for (j, &(bull, bear)) in divs.iter().enumerate().filter(|(_, d)| d.0 || d.1) {
            // bougie de la chart qui contient cette bougie du slot
            let i = bars.partition_point(|b| b.time <= lb[j].time);
            let Some(i) = i.checked_sub(1).filter(|&i| lb[j].time < bars[i].time + chart_seconds) else {
                continue;
            };
            for (on, is_bull) in [(bull, true), (bear, false)] {
                let dot = MtfDot {
                    slot: k + 1,
                    bull: is_bull,
                    index: i,
                };
                if on && !dots.contains(&dot) {
                    dots.push(dot);
                }
            }
        }
    }
    dots
}

pub fn compute(bars: &[Bar], chart_seconds: i64, s: &Settings, lower: &LowerBars) -> AiTrend {
    let n = bars.len();
    let l = s.preset.lengths(chart_seconds, s.lengths);
    let preset = match s.preset {
        Preset::Custom => "Personnalisé",
        Preset::Auto if chart_seconds <= 900 => "Scalping",
        Preset::Auto if chart_seconds <= 14_400 => "Day Trade",
        Preset::Auto => "Swing",
        Preset::Scalping => "Scalping",
        Preset::DayTrade => "Day Trade",
        Preset::Swing => "Swing",
    };
    let ai = index(bars, l, s.smooth_type);
    // toujours calculée (comme le Pine : les croisements et leurs alertes marchent même si la ligne est masquée)
    let signal = ma(&ai, l.signal, s.signal_type);
    let extreme_high = (0..n).filter(|&i| ai[i] >= s.very_red).collect();
    let extreme_low = (0..n).filter(|&i| ai[i] <= s.very_blue).collect();
    // cachées toujours calculées (leurs alertes marchent même si les lignes sont masquées, comme le Pine)
    let divs = find_divs(&ai, bars, l.pivot_left, l.pivot_right, l.max_distance, s.min_move, true);

    // divergences multi-unités de temps : comme `request.security_lower_tf` du Pine, seulement les UT plus petites que la chart
    let mtf = mtf_dots(bars, chart_seconds, s, lower);

    // signaux achat / vente (comme le Pine : croisement AI / ligne signal, filtré Aucun / Midline / Zones)
    let close: Vec<f64> = bars.iter().map(|b| b.close).collect();
    let (ef, es) = (ema(&close, s.ema_fast), ema(&close, s.ema_slow));
    let (mut buy, mut sell) = (vec![], vec![]);
    for i in 1..n {
        if !signal[i].is_finite() || !signal[i - 1].is_finite() {
            continue;
        }
        let up = ai[i] > signal[i] && ai[i - 1] <= signal[i - 1];
        let dn = ai[i] < signal[i] && ai[i - 1] >= signal[i - 1];
        let (buy_ok, sell_ok) = match s.signal_filter {
            0 => (true, true),
            1 => (signal[i] < s.midline, signal[i] > s.midline),
            _ => (signal[i] <= s.oversold, signal[i] >= s.overbought),
        };
        if up && buy_ok {
            buy.push(i);
        }
        if dn && sell_ok {
            sell.push(i);
        }
    }

    // boîte HTF : Auto = l'UT de la chart (comme le Pine), sinon l'UT fixe
    let htf = s
        .htf_box
        .then(|| s.htf_fixed.unwrap_or(chart_seconds))
        .filter(|&hs| hs >= chart_seconds)
        .and_then(|hs| {
            let h = if hs == chart_seconds { bars.to_vec() } else { aggregate(bars, hs) };
            (h.len() > 20).then(|| {
                let value = if hs == chart_seconds {
                    *ai.last().unwrap_or(&f64::NAN)
                } else {
                    // request.security : en preset Auto, le HTF prend le preset de sa propre unité de temps
                    *index(&h, s.preset.lengths(hs, s.lengths), s.smooth_type).last().unwrap_or(&f64::NAN)
                };
                let zone = if value <= s.oversold {
                    HtfZone::Buy
                } else if value >= s.overbought {
                    HtfZone::Sell
                } else {
                    HtfZone::Neutral
                };
                HtfBox { seconds: hs, value, zone }
            })
        });
    let trend_bull = n > 0 && ef[n - 1] >= es[n - 1];
    // flèches du Pine (buyCondition / sellCondition) : l'AI sort de la survente / du surachat, filtrées par la tendance EMA
    let (mut arrow_buy, mut arrow_sell) = (vec![], vec![]);
    for i in 1..n {
        if !ai[i].is_finite() || !ai[i - 1].is_finite() {
            continue;
        }
        if ai[i] > s.oversold && ai[i - 1] <= s.oversold && (!s.ema_filter || ef[i] > es[i]) {
            arrow_buy.push(i);
        }
        if ai[i] < s.overbought && ai[i - 1] >= s.overbought && (!s.ema_filter || ef[i] < es[i]) {
            arrow_sell.push(i);
        }
    }

    AiTrend {
        ai,
        signal,
        divs,
        mtf,
        buy,
        sell,
        extreme_high,
        extreme_low,
        preset,
        htf,
        trend_bull,
        arrow_buy,
        arrow_sell,
        volume: super::ai_trend_volume::compute(bars, chart_seconds, &s.volume),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets() {
        let c = Settings::default().lengths;
        assert_eq!(Preset::Custom.lengths(60, c), c);
        assert_eq!(Preset::Auto.lengths(900, c).rsi, 7);
        assert_eq!(Preset::Auto.lengths(14_400, c).rsi, 10);
        assert_eq!(Preset::Auto.lengths(86_400, c).rsi, 14);
    }

    #[test]
    fn index_borne_0_100() {
        let bars: Vec<Bar> = (0..400)
            .map(|i| {
                let c = 100.0 + (i as f64 / 9.0).sin() * 4.0;
                Bar {
                    time: i * 1800,
                    open: c,
                    high: c + 1.0,
                    low: c - 1.0,
                    close: c,
                    volume: 1.0,
                }
            })
            .collect();
        let r = compute(&bars, 1800, &Settings::default(), &LowerBars::new());
        assert!(r.ai.iter().filter(|v| v.is_finite()).all(|v| (0.0..=100.0).contains(v)));
        assert!(r.htf.is_some());
    }

    #[test]
    fn mtf_comme_security_lower_tf() {
        // bougies 1h qui ondulent ; la chart est en 1D (24 bougies 1h chacune)
        let h1: Vec<Bar> = (0..2400)
            .map(|i| {
                let c = 100.0 + (i as f64 / 7.0).sin() * (3.0 + (i as f64 / 50.0).cos() * 2.0) + i as f64 * 0.002;
                Bar {
                    time: i * 3600,
                    open: c,
                    high: c + 0.5,
                    low: c - 0.5,
                    close: c,
                    volume: 1.0,
                }
            })
            .collect();
        let d1 = aggregate(&h1, 86_400);
        let mut s = Settings::default();
        s.mtf[0] = MtfSlot { on: true, seconds: 3600 };
        s.mtf[1].on = false;
        s.mtf[2].on = false;
        s.mtf[3].on = false;
        assert_eq!(lower_needed(&s, 86_400), vec![3600]);
        assert!(lower_needed(&s, 1800).is_empty(), "slot plus grand que la chart : ignoré, comme le Pine");
        let lower: LowerBars = [(3600, h1)].into_iter().collect();
        let r = compute(&d1, 86_400, &s, &lower);
        assert!(r.mtf.iter().any(|m| m.bull) && r.mtf.iter().any(|m| !m.bull), "{:?}", r.mtf.len());
        assert!(r.mtf.iter().all(|m| m.slot == 1 && m.index < d1.len()));
        // chart plus petite que le slot : rien
        assert!(compute(&d1, 1800, &s, &lower).mtf.is_empty());
    }
}
