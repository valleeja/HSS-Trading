//! Argos Cipher Pro — WaveTrend + RSI + Stoch RSI + RSI/MFI, divergences, points d'achat / vente, Golden dot, Purple dot, confluence /4.
//! Même logique que le Cipher actuel du site (calqué sur le Cipher TradingView / VuManChu, vérifié sur ZEC 30m OKX).
//! Crédit : logique du « VuManChu Cipher B + Divergences » (vumanchu, open source sur TradingView), réécrite pour Argos.

use crate::inputs::{defaults, Inputs};
use crate::ta::{ema, rsi, sma, stoch};
use argos_core::Bar;

/// Source d'une série (comme le menu « Source » de TradingView).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Open,
    High,
    Low,
    Close,
    Hl2,
    Hlc3,
    Ohlc4,
}

impl Source {
    /// Dans l'ordre des sources de v1 (`inputs::SOURCES`).
    pub const ALL: [Source; 7] = [Source::Close, Source::Open, Source::High, Source::Low, Source::Hl2, Source::Hlc3, Source::Ohlc4];

    fn of(self, b: &Bar) -> f64 {
        match self {
            Source::Open => b.open,
            Source::High => b.high,
            Source::Low => b.low,
            Source::Close => b.close,
            Source::Hl2 => (b.high + b.low) / 2.0,
            Source::Hlc3 => (b.high + b.low + b.close) / 3.0,
            Source::Ohlc4 => (b.open + b.high + b.low + b.close) / 4.0,
        }
    }
    fn series(self, bars: &[Bar]) -> Vec<f64> {
        bars.iter().map(|b| self.of(b)).collect()
    }
}

/// Toutes les entrées du Cipher, avec les valeurs par défaut du Cipher TradingView.
#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    // WaveTrend
    pub wt_channel: usize,
    pub wt_average: usize,
    pub wt_source: Source,
    pub wt_ma: usize,
    pub ob1: f64,
    pub ob2: f64,
    pub ob3: f64,
    pub os1: f64,
    pub os2: f64,
    pub os3: f64,
    pub wt_div_regular: bool,
    pub wt_div_hidden: bool,
    pub hidden_no_limits: bool,
    pub wt_div_bear_min: f64,
    pub wt_div_bull_min: f64,
    pub wt_div2: bool,
    pub wt_div2_bear: f64,
    pub wt_div2_bull: f64,
    // RSI + MFI
    pub mfi_period: usize,
    pub mfi_multiplier: f64,
    pub mfi_pos_y: f64,
    pub mfi_div_regular: bool,
    pub mfi_div_hidden: bool,
    pub mfi_div_bear_min: f64,
    pub mfi_div_bull_min: f64,
    // RSI
    pub rsi_source: Source,
    pub rsi_len: usize,
    pub rsi_oversold: f64,
    pub rsi_overbought: f64,
    pub rsi_div_regular: bool,
    pub rsi_div_hidden: bool,
    pub rsi_div_bear_min: f64,
    pub rsi_div_bull_min: f64,
    // Stoch RSI
    pub st_log: bool,
    pub st_average_kd: bool,
    pub st_source: Source,
    pub st_len: usize,
    pub st_rsi_len: usize,
    pub st_k: usize,
    pub st_d: usize,
    pub st_div_regular: bool,
    pub st_div_hidden: bool,
    // Confluence
    pub confluence_min: usize,
    pub confluence_window: usize,
    /// Confluence comme TradingView (sinon « Argos large » avec sa fenêtre).
    pub confluence_tv: bool,
}

/// Groupes et inputs de v1 (générés depuis app.js : `v1_inputs`).
pub use crate::v1_inputs::{CIPHER_GROUPS as GROUPS, CIPHER_PARAMS as PARAMS};

impl Settings {
    /// Réglages du calcul lus dans les inputs (valeurs dans l'ordre de `PARAMS`).
    pub fn from_inputs(vals: &[f64]) -> Self {
        let p = Inputs { params: &PARAMS, vals };
        let src = |k: &str| Source::ALL[p.pick(k).min(Source::ALL.len() - 1)];
        Self {
            wt_channel: p.len("ch"),
            wt_average: p.len("avg"),
            wt_source: src("wt_src"),
            wt_ma: p.len("ma"),
            ob1: p.get("ob1"),
            ob2: p.get("ob2"),
            ob3: p.get("ob3"),
            os1: p.get("os1"),
            os2: p.get("os2"),
            os3: p.get("os3"),
            wt_div_regular: p.on("wtd_reg"),
            wt_div_hidden: p.on("wtd_hid"),
            hidden_no_limits: p.on("wtd_nolim"),
            wt_div_bear_min: p.get("wtd_bear"),
            wt_div_bull_min: p.get("wtd_bull"),
            wt_div2: p.on("wtd2"),
            wt_div2_bear: p.get("wtd2_bear"),
            wt_div2_bull: p.get("wtd2_bull"),
            mfi_period: p.len("mfi_p"),
            mfi_multiplier: p.get("mfi_m"),
            mfi_pos_y: p.get("mfi_y"),
            mfi_div_regular: p.on("mfi_reg"),
            mfi_div_hidden: p.on("mfi_hid"),
            mfi_div_bear_min: p.get("mfi_bear"),
            mfi_div_bull_min: p.get("mfi_bull"),
            rsi_source: src("rsi_src"),
            rsi_len: p.len("rsi_len"),
            rsi_oversold: p.get("rsi_os"),
            rsi_overbought: p.get("rsi_ob"),
            rsi_div_regular: p.on("rsi_reg"),
            rsi_div_hidden: p.on("rsi_hid"),
            rsi_div_bear_min: p.get("rsi_bear"),
            rsi_div_bull_min: p.get("rsi_bull"),
            st_log: p.on("st_log"),
            st_average_kd: p.on("st_avg"),
            st_source: src("st_src"),
            st_len: p.len("st_len"),
            st_rsi_len: p.len("st_rsi"),
            st_k: p.len("st_k"),
            st_d: p.len("st_d"),
            st_div_regular: p.on("st_reg"),
            st_div_hidden: p.on("st_hid"),
            confluence_min: p.len("conf_min"),
            confluence_window: p.len("conf_win"),
            confluence_tv: p.pick("conf_mode") == 0,
        }
    }
}

/// Valeurs par défaut = celles de v1 (une seule source : `PARAMS`).
impl Default for Settings {
    fn default() -> Self {
        Self::from_inputs(&defaults(&PARAMS))
    }
}

/// Une divergence : pivot d'avant, pivot actuel, bougie où elle est confirmée (pivot + 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Div {
    pub prev: usize,
    pub pivot: usize,
    pub confirmed: usize,
}

/// Divergences d'une série.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Divs {
    pub bull: Vec<Div>,
    pub bear: Vec<Div>,
    pub hidden_bull: Vec<Div>,
    pub hidden_bear: Vec<Div>,
    /// Creux et sommets fractals retenus (servent au Golden dot).
    pub bottoms: Vec<usize>,
    pub tops: Vec<usize>,
}

/// Divergences comme le Cipher (`f_findDivs`) : fractale 2/2, la fractale d'avant doit aussi passer le seuil, pas de distance maximale.
///
/// Comme Pine : `f_fractalize` = `f_top_fractal(src) ? 1 : f_bot_fractal(src) ? -1 : 0`, donc le motif du creux ne tourne
/// que sur les bougies sans motif de sommet. Sur une bougie à sommet, son `src` à lui garde la valeur d'avant ; il regarde
/// donc `src[1..4]` dans cette histoire-là (mesuré sur l'export TradingView BTCUSDT 1D : 32 points sur 32, 4 faux avant).
/// Le pivot, son seuil, `low[2]` et la fractale d'avant restent ceux de la bougie `i - 2`, comme `src[2]` et `valuewhen` dans Pine.
pub fn find_divs(src: &[f64], bars: &[Bar], top_limit: f64, bot_limit: f64, use_limits: bool) -> Divs {
    let mut d = Divs::default();
    let (mut pt, mut pb): (Option<usize>, Option<usize>) = (None, None);
    let mut bot_src: Vec<f64> = Vec::with_capacity(src.len());
    for i in 0..src.len() {
        let top = i >= 4 && {
            let v = src[i - 2];
            src[i - 4] < v && src[i - 3] < v && v > src[i - 1] && v > src[i]
        };
        let bottom = if top {
            bot_src.push(bot_src.last().copied().unwrap_or(f64::NAN));
            false
        } else {
            bot_src.push(src[i]);
            let w = &bot_src[bot_src.len().saturating_sub(5)..];
            w.len() == 5 && w[0] > w[2] && w[1] > w[2] && w[2] < w[3] && w[2] < w[4]
        };
        if i < 4 {
            continue;
        }
        let j = i - 2;
        let v = src[j];
        if top && (!use_limits || v >= top_limit) {
            if let Some(p) = pt {
                let div = Div {
                    prev: p,
                    pivot: j,
                    confirmed: i,
                };
                if bars[j].high > bars[p].high && v < src[p] {
                    d.bear.push(div);
                }
                if bars[j].high < bars[p].high && v > src[p] {
                    d.hidden_bear.push(div);
                }
            }
            d.tops.push(j);
            pt = Some(j);
        }
        if bottom && (!use_limits || v <= bot_limit) {
            if let Some(p) = pb {
                let div = Div {
                    prev: p,
                    pivot: j,
                    confirmed: i,
                };
                if bars[j].low < bars[p].low && v > src[p] {
                    d.bull.push(div);
                }
                if bars[j].low > bars[p].low && v < src[p] {
                    d.hidden_bull.push(div);
                }
            }
            d.bottoms.push(j);
            pb = Some(j);
        }
    }
    d
}

/// Résultat du Cipher.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Cipher {
    pub wt1: Vec<f64>,
    pub wt2: Vec<f64>,
    /// WT1 − WT2 (« Fast WT » / VWAP).
    pub fast: Vec<f64>,
    pub rsi: Vec<f64>,
    pub mfi: Vec<f64>,
    pub stoch_k: Vec<f64>,
    pub stoch_d: Vec<f64>,
    /// Chaque croisement des vagues (point vert / rouge sur WT2).
    pub cross_up: Vec<usize>,
    pub cross_down: Vec<usize>,
    /// Croisement en zone (sous l'oversold 1 / au-dessus de l'overbought 1).
    pub buy: Vec<usize>,
    pub sell: Vec<usize>,
    /// Points de divergence (au pivot) : WT, WT 2nd, Stoch, RSI.
    pub div_bull_dots: Vec<usize>,
    pub div_bear_dots: Vec<usize>,
    /// Bougies du signal Golden dot (`wtGoldBuy`) : alertes, tableau, « il y a n barres ».
    pub gold: Vec<usize>,
    /// Où le chart dessine les Golden dots : au creux, 2 bougies avant le signal (`offset = -2` du Pine).
    pub gold_dots: Vec<usize>,
    pub purple: Vec<usize>,
    /// Signaux de confluence (n/4 atteint).
    pub confluence_buy: Vec<(usize, usize)>,
    pub confluence_sell: Vec<(usize, usize)>,
    /// Bougies où le Pine voit buySignalDiv / sellSignalDiv (pivot + 2).
    pub div_bull_at: Vec<usize>,
    pub div_bear_at: Vec<usize>,
    /// Divergences régulières WT + RSI + Stoch + MFI confirmées sur chaque bougie (bullDivCount / bearDivCount du Pine).
    pub bull_count: Vec<u8>,
    pub bear_count: Vec<u8>,
    /// Tableau Confluence : (bull, bear) à la dernière bougie pour WT, RSI, Stoch RSI, MFI.
    pub table_last: [(bool, bool); 4],
    pub wt_divs: Divs,
    pub wt2_divs: Divs,
    pub rsi_divs: Divs,
    pub stoch_divs: Divs,
    pub mfi_divs: Divs,
}

/// 🔔 Les 9 alertes du Pine (noms des `alertcondition`), dans l'ordre de `Cipher::alerts`.
pub const ALERTS: [&str; 9] = [
    "Buy (Big green circle)",
    "Buy (Big green circle + Div)",
    "GOLD Buy (Big GOLDEN circle)",
    "Buy (Small green dot)",
    "Sell (Big red circle)",
    "Sell (Big red circle + Div)",
    "Sell (Small red dot)",
    "Confluence Bullish (2+/4)",
    "Confluence Bearish (2+/4)",
];

impl Cipher {
    /// 🔔 Signaux des alertes (`ALERTS`) : 1 sur les bougies où le signal arrive, 0 ailleurs.
    pub fn alerts(&self) -> Vec<(&'static str, Vec<f64>)> {
        let n = self.wt2.len();
        let at = |idx: &[usize]| {
            let mut v = vec![0.0; n];
            for &i in idx.iter().filter(|&&i| i < n) {
                v[i] = 1.0;
            }
            v
        };
        let two = |c: &[u8]| c.iter().map(|&x| if x >= 2 { 1.0 } else { 0.0 }).collect::<Vec<f64>>();
        let series = [
            at(&self.buy),
            at(&self.div_bull_at),
            at(&self.gold),
            at(&self.cross_up),
            at(&self.sell),
            at(&self.div_bear_at),
            at(&self.cross_down),
            two(&self.bull_count),
            two(&self.bear_count),
        ];
        ALERTS.iter().copied().zip(series).collect()
    }
}

fn crosses(a: &[f64], b: &[f64]) -> (Vec<usize>, Vec<usize>) {
    let (mut up, mut dn) = (vec![], vec![]);
    for i in 1..a.len() {
        if a[i] > b[i] && a[i - 1] <= b[i - 1] {
            up.push(i);
        }
        if a[i] < b[i] && a[i - 1] >= b[i - 1] {
            dn.push(i);
        }
    }
    (up, dn)
}

/// Garde les divergences régulières seulement si `regular`, cachées seulement si `hidden` (prises dans `hidden_src`).
fn select(regular_src: Divs, hidden_src: Option<&Divs>, regular: bool, hidden: bool) -> Divs {
    let h = hidden_src.unwrap_or(&regular_src);
    Divs {
        hidden_bull: if hidden { h.hidden_bull.clone() } else { vec![] },
        hidden_bear: if hidden { h.hidden_bear.clone() } else { vec![] },
        bull: if regular { regular_src.bull.clone() } else { vec![] },
        bear: if regular { regular_src.bear.clone() } else { vec![] },
        bottoms: regular_src.bottoms.clone(),
        tops: regular_src.tops.clone(),
    }
}

pub fn compute(bars: &[Bar], s: &Settings) -> Cipher {
    let n = bars.len();
    // WaveTrend
    let src = s.wt_source.series(bars);
    let esa = ema(&src, s.wt_channel);
    let dev: Vec<f64> = src.iter().zip(&esa).map(|(v, e)| (v - e).abs()).collect();
    let de = ema(&dev, s.wt_channel);
    // (like Pine: na / na and x / 0 are na — while `de` warms up (n1 − 1 bars after `esa`), dividing by 1e-9 gave
    // millions, and the first bars of the history pushed Cipher's scale to −25 000 000; Jee 19 Nov)
    let ci: Vec<f64> = (0..n)
        .map(|i| {
            let den = 0.015 * de[i];
            if den != 0.0 && den.is_finite() {
                (src[i] - esa[i]) / den
            } else {
                f64::NAN
            }
        })
        .collect();
    let wt1 = ema(&ci, s.wt_average);
    let wt2 = sma(&wt1, s.wt_ma);
    let fast: Vec<f64> = wt1.iter().zip(&wt2).map(|(a, b)| a - b).collect();
    // RSI + MFI (« money flow » du Cipher)
    let raw_mfi: Vec<f64> = bars
        .iter()
        .map(|b| {
            if b.range() != 0.0 {
                (b.close - b.open) / b.range() * s.mfi_multiplier
            } else {
                0.0
            }
        })
        .collect();
    let mfi: Vec<f64> = sma(&raw_mfi, s.mfi_period).iter().map(|v| v - s.mfi_pos_y).collect();
    // RSI
    let r = rsi(&s.rsi_source.series(bars), s.rsi_len);
    // Stoch RSI
    let st_src: Vec<f64> = s.st_source.series(bars).iter().map(|v| if s.st_log { v.ln() } else { *v }).collect();
    let kk = sma(&stoch(&rsi(&st_src, s.st_rsi_len), s.st_len), s.st_k);
    let d = sma(&kk, s.st_d);
    let k: Vec<f64> = if s.st_average_kd {
        kk.iter().zip(&d).map(|(a, b)| (a + b) / 2.0).collect()
    } else {
        kk.clone()
    };

    // Divergences
    let wl = find_divs(&wt2, bars, s.wt_div_bear_min, s.wt_div_bull_min, true);
    let wt_nolim = s.hidden_no_limits.then(|| find_divs(&wt2, bars, 0.0, 0.0, false));
    let wt_divs = select(wl.clone(), wt_nolim.as_ref(), s.wt_div_regular, s.wt_div_hidden);
    // 2e WT : lignes régulières et cachées seulement si « Show 2nd » ; ses points de divergence suivent « Show WT Regular »
    // (comme le Pine : buySignalDiv prend wtBullDiv_add sans regarder « Show 2nd »)
    let wa = find_divs(&wt2, bars, s.wt_div2_bear, s.wt_div2_bull, true);
    let wt2_divs = if s.wt_div2 {
        select(wa.clone(), None, s.wt_div_regular, s.wt_div_hidden)
    } else {
        Divs::default()
    };
    let wt_add = select(wa, None, s.wt_div_regular, false);
    let rl = find_divs(&r, bars, s.rsi_div_bear_min, s.rsi_div_bull_min, true);
    let rsi_nolim = s.hidden_no_limits.then(|| find_divs(&r, bars, 0.0, 0.0, false));
    let rsi_divs = select(rl.clone(), rsi_nolim.as_ref(), s.rsi_div_regular, s.rsi_div_hidden);
    let sl = find_divs(&k, bars, 0.0, 0.0, false);
    let stoch_divs = select(sl.clone(), None, s.st_div_regular, s.st_div_hidden);
    let mfi_hidden = s.hidden_no_limits.then(|| find_divs(&mfi, bars, 0.0, 0.0, false));
    let ml = find_divs(&mfi, bars, s.mfi_div_bear_min, s.mfi_div_bull_min, true);
    let mfi_divs = select(ml.clone(), mfi_hidden.as_ref(), s.mfi_div_regular, s.mfi_div_hidden);
    // tableau Confluence du Pine : les divergences régulières de la dernière bougie, boutons « Show » ou non
    let at_last = |d: &Divs| {
        let now = |l: &[Div]| l.iter().any(|x| x.confirmed + 1 == n);
        (now(&d.bull), now(&d.bear))
    };
    let table_last = [at_last(&wl), at_last(&rl), at_last(&sl), at_last(&ml)];

    // Points
    let (cross_up, cross_down) = crosses(&wt1, &wt2);
    let buy: Vec<usize> = cross_up.iter().copied().filter(|&i| wt2[i] <= s.os1).collect();
    let sell: Vec<usize> = cross_down.iter().copied().filter(|&i| wt2[i] >= s.ob1).collect();
    let pick = |lists: [&Vec<Div>; 4], at: fn(&Div) -> usize| -> Vec<usize> {
        let mut v: Vec<usize> = lists.iter().flat_map(|l| l.iter().map(at)).collect();
        v.sort_unstable();
        v.dedup();
        v
    };
    let div_bull_dots = pick([&wt_divs.bull, &wt_add.bull, &stoch_divs.bull, &rsi_divs.bull], |x| x.pivot);
    let div_bear_dots = pick([&wt_divs.bear, &wt_add.bear, &stoch_divs.bear, &rsi_divs.bear], |x| x.pivot);
    // bougies où le Pine voit buySignalDiv / sellSignalDiv (confirmation du pivot) : pour les alertes
    let div_bull_at = pick([&wt_divs.bull, &wt_add.bull, &stoch_divs.bull, &rsi_divs.bull], |x| x.confirmed);
    let div_bear_at = pick([&wt_divs.bear, &wt_add.bear, &stoch_divs.bear, &rsi_divs.bear], |x| x.confirmed);

    // 🟡 Golden dot : divergence haussière (WT ou RSI) + creux WT d'avant ≤ oversold 3 + WT2 remonté au-dessus (≥ 5 de plus) + RSI < 30 à ce creux
    let mut prev_bottom = vec![None; n];
    {
        let (mut last, mut k2) = (None, 0);
        for (i, slot) in prev_bottom.iter_mut().enumerate() {
            while k2 < wl.bottoms.len() && wl.bottoms[k2] + 2 <= i {
                last = Some(wl.bottoms[k2]);
                k2 += 1;
            }
            *slot = last;
        }
    }
    let mut bull_at = vec![false; n];
    for x in wt_divs.bull.iter().chain(&rsi_divs.bull) {
        bull_at[x.confirmed] = true;
    }
    let gold: Vec<usize> = (3..n)
        .filter(|&i| {
            bull_at[i]
                && prev_bottom[i - 1].is_some_and(|pj| {
                    let low_prev = wt2[pj];
                    low_prev <= s.os3 && wt2[i] > s.os3 && low_prev - wt2[i] <= -5.0 && r[pj] < s.rsi_oversold
                })
        })
        .collect();

    // 🟣 Purple dot : le miroir baissier
    let bear_all: Vec<Div> = [&wt_divs.bear, &wt2_divs.bear, &rsi_divs.bear, &stoch_divs.bear, &mfi_divs.bear]
        .iter()
        .flat_map(|l| l.iter().copied())
        .collect();
    let near_bear = |i: usize| bear_all.iter().any(|x| x.confirmed.abs_diff(i) <= 3 || x.pivot.abs_diff(i) <= 3);
    let mut purple: Vec<usize> = wt_divs
        .bear
        .iter()
        .filter(|x| wt2[x.pivot] >= -s.os3 && r[x.pivot] >= s.rsi_overbought)
        .map(|x| x.pivot)
        .collect();
    purple.extend(
        sell.iter()
            .copied()
            .filter(|&i| wt2[i] >= -s.os3 && (r[i] >= s.rsi_overbought - 5.0 || near_bear(i))),
    );
    purple.sort_unstable();
    purple.dedup();

    // Confluence /4 : WT, RSI, MFI, Stoch
    let flag = |idx: Vec<usize>| {
        let mut f = vec![false; n];
        for i in idx {
            if i < n {
                f[i] = true;
            }
        }
        f
    };
    let conf = |l: &[Div]| l.iter().map(|x| x.confirmed).collect::<Vec<_>>();
    let (st_up, st_dn) = crosses(&kk, &d);
    let mut mfi_up = vec![];
    let mut mfi_dn = vec![];
    for i in 1..n {
        if mfi[i] > 0.0 && mfi[i - 1] <= 0.0 {
            mfi_up.push(i);
        }
        if mfi[i] < 0.0 && mfi[i - 1] >= 0.0 {
            mfi_dn.push(i);
        }
    }
    let r_low: Vec<usize> = (0..n).filter(|&i| r[i] <= s.rsi_oversold).collect();
    let r_high: Vec<usize> = (0..n).filter(|&i| r[i] >= s.rsi_overbought).collect();
    let cat = |a: &[usize], b: Vec<usize>| [a.to_vec(), b].concat();
    let bull_div_conf = [conf(&wt_divs.bull), conf(&wt2_divs.bull), conf(&stoch_divs.bull), conf(&rsi_divs.bull)].concat();
    let bear_div_conf = [conf(&wt_divs.bear), conf(&wt2_divs.bear), conf(&stoch_divs.bear), conf(&rsi_divs.bear)].concat();
    let b_flags = [
        flag(cat(&buy, bull_div_conf)),
        flag(cat(&r_low, [conf(&rsi_divs.bull), conf(&rsi_divs.hidden_bull)].concat())),
        flag(cat(&mfi_up, conf(&mfi_divs.bull))),
        flag(cat(
            &st_up.iter().copied().filter(|&i| kk[i] < 20.0).collect::<Vec<_>>(),
            conf(&stoch_divs.bull),
        )),
    ];
    let s_flags = [
        flag(cat(&sell, bear_div_conf)),
        flag(cat(&r_high, [conf(&rsi_divs.bear), conf(&rsi_divs.hidden_bear)].concat())),
        flag(cat(&mfi_dn, conf(&mfi_divs.bear))),
        flag(cat(
            &st_dn.iter().copied().filter(|&i| kk[i] > 80.0).collect::<Vec<_>>(),
            conf(&stoch_divs.bear),
        )),
    ];
    // Confluence /4 comme le Pine : divergences régulières WT, RSI, Stoch et MFI confirmées sur la même bougie
    // (bullDivCount / bearDivCount) ; l'alerte part dès 2/4
    let mut bull_count = vec![0u8; n];
    let mut bear_count = vec![0u8; n];
    for d in [&wt_divs, &rsi_divs, &stoch_divs, &mfi_divs] {
        for x in d.bull.iter().filter(|x| x.confirmed < n) {
            bull_count[x.confirmed] += 1;
        }
        for x in d.bear.iter().filter(|x| x.confirmed < n) {
            bear_count[x.confirmed] += 1;
        }
    }
    let (mut confluence_buy, mut confluence_sell) = (vec![], vec![]);
    if s.confluence_tv {
        // étiquette au pivot (bar_index[2]) quand un creux / sommet MFI est confirmé avec assez de divergences
        for &j in &mfi_divs.bottoms {
            let c = bull_count.get(j + 2).copied().unwrap_or(0) as usize;
            if c >= s.confluence_min {
                confluence_buy.push((j, c));
            }
        }
        for &j in &mfi_divs.tops {
            let c = bear_count.get(j + 2).copied().unwrap_or(0) as usize;
            if c >= s.confluence_min {
                confluence_sell.push((j, c));
            }
        }
    } else {
        // Argos large : zones, croisements et divergences de chaque oscillateur dans une fenêtre de bougies
        let w = s.confluence_window.max(1);
        let any_in = |f: &Vec<bool>, i: usize| (i.saturating_sub(w - 1)..=i).any(|k| f[k]);
        let (mut lb, mut ls) = (0, 0);
        for i in 0..n {
            let cb = b_flags.iter().filter(|f| any_in(f, i)).count();
            let cs = s_flags.iter().filter(|f| any_in(f, i)).count();
            if cb >= s.confluence_min && lb < s.confluence_min {
                confluence_buy.push((i, cb));
            }
            if cs >= s.confluence_min && ls < s.confluence_min {
                confluence_sell.push((i, cs));
            }
            lb = cb;
            ls = cs;
        }
    }

    Cipher {
        div_bull_at,
        div_bear_at,
        bull_count,
        bear_count,
        table_last,
        wt1,
        wt2,
        fast,
        rsi: r,
        mfi,
        stoch_k: k,
        stoch_d: d,
        cross_up,
        cross_down,
        buy,
        sell,
        div_bull_dots,
        div_bear_dots,
        gold_dots: gold.iter().filter_map(|&i| i.checked_sub(2)).collect(),
        gold,
        purple,
        confluence_buy,
        confluence_sell,
        wt_divs,
        wt2_divs,
        rsi_divs,
        stoch_divs,
        mfi_divs,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bars_from(closes: &[f64]) -> Vec<Bar> {
        closes
            .iter()
            .enumerate()
            .map(|(i, &c)| Bar {
                time: i as i64 * 60,
                open: c,
                high: c + 0.5,
                low: c - 0.5,
                close: c,
                volume: 1.0,
            })
            .collect()
    }

    fn bars_from_lows(lows: &[f64]) -> Vec<Bar> {
        lows.iter()
            .enumerate()
            .map(|(i, &l)| Bar {
                time: i as i64,
                open: l + 1.0,
                high: l + 2.0,
                low: l,
                close: l + 1.0,
                volume: 1.0,
            })
            .collect()
    }

    #[test]
    fn divergence_haussiere_simple() {
        // prix : creux 10 puis creux plus bas 9 ; oscillateur : creux -80 puis creux plus haut -70
        let osc = [0.0, -50.0, -80.0, -50.0, 0.0, -10.0, -40.0, -70.0, -40.0, 0.0];
        let bars = bars_from_lows(&[12.0, 11.0, 10.0, 11.0, 12.0, 12.0, 11.0, 9.0, 11.0, 12.0]);
        let d = find_divs(&osc, &bars, 0.0, -65.0, true);
        assert_eq!(
            d.bull,
            vec![Div {
                prev: 2,
                pivot: 7,
                confirmed: 9
            }]
        );
        // seuil plus strict : le 2e creux (-70) ne passe plus sous -75 → pas de divergence
        assert!(find_divs(&osc, &bars, 0.0, -75.0, true).bull.is_empty());
    }

    /// B-027 : comme Pine (`f_top_fractal(src) ? 1 : f_bot_fractal(src) ? -1 : 0`), le creux ne voit pas la bougie
    /// où un sommet se confirme. Ici le sommet 0 (bougie 4) se confirme sur la bougie 6, celle du creux -70 :
    /// TradingView ne voit pas ce creux, donc pas de divergence (le même motif avec une bougie de plus en voit une, test du dessus).
    #[test]
    fn creux_sur_la_confirmation_d_un_sommet_comme_pine() {
        let osc = [0.0, -50.0, -80.0, -50.0, 0.0, -40.0, -70.0, -40.0, 0.0];
        let bars = bars_from_lows(&[12.0, 11.0, 10.0, 11.0, 12.0, 11.0, 9.0, 11.0, 12.0]);
        let d = find_divs(&osc, &bars, 0.0, -65.0, true);
        assert_eq!(d.tops, vec![4]);
        assert_eq!(d.bottoms, vec![2]);
        assert!(d.bull.is_empty());
    }

    /// 📏 Jee (19 Nov): scrolled to the very first bars of a BTC history, Cipher's scale went to −25 000 000 — while
    /// `de` warmed up, WaveTrend divided by 1e-9. Like Pine (na / na = na), those bars give nothing: every WaveTrend
    /// value is NaN or a normal one, from the very first bar.
    #[test]
    fn wavetrend_sans_valeur_geante_au_demarrage() {
        let closes: Vec<f64> = (0..400).map(|i| 63_000.0 + (i as f64 / 9.0).sin() * 900.0 + i as f64 * 3.0).collect();
        let c = compute(&bars_from(&closes), &Settings::default());
        for (nom, v) in [("wt1", &c.wt1), ("wt2", &c.wt2)] {
            let geants: Vec<(usize, f64)> = v.iter().copied().enumerate().filter(|(_, x)| x.is_finite() && x.abs() > 200.0).collect();
            assert!(geants.is_empty(), "{nom}: {geants:?}");
            assert!(v[..8].iter().all(|x| x.is_nan()), "{nom}: nothing while it warms up, like Pine");
            assert!(v[100..].iter().all(|x| x.is_finite()), "{nom}: then a value on every bar");
        }
        // a flat start (no move at all): na like Pine, never millions
        let mut plat = vec![63_000.0; 40];
        plat.extend((0..200).map(|i| 63_000.0 + (i as f64 / 5.0).sin() * 500.0));
        let c = compute(&bars_from(&plat), &Settings::default());
        assert!(c.wt1.iter().all(|x| !x.is_finite() || x.abs() < 200.0), "flat start: {:?}", &c.wt1[..60]);
    }

    #[test]
    fn series_bornees_et_signaux() {
        let closes: Vec<f64> = (0..300).map(|i| 100.0 + (i as f64 / 7.0).sin() * 5.0 + (i as f64 / 31.0).sin() * 8.0).collect();
        let c = compute(&bars_from(&closes), &Settings::default());
        assert_eq!(c.wt1.len(), 300);
        assert!(c.rsi.iter().filter(|v| v.is_finite()).all(|v| (0.0..=100.0).contains(v)));
        let bad: Vec<f64> = c
            .stoch_k
            .iter()
            .copied()
            .filter(|v| v.is_finite() && !(-1e-9..=100.0 + 1e-9).contains(v))
            .collect();
        assert!(bad.is_empty(), "{bad:?}");
        assert!(!c.cross_up.is_empty() && !c.cross_down.is_empty());
        assert!(c.buy.iter().all(|i| c.cross_up.contains(i)));
    }
}
