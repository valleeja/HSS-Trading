//! Calcul des séries (indicateurs existants d'argos-script, en f64) puis photo d'une bougie : régime, scores, setup.
//! Toutes les séries sont causales : la valeur d'une bougie ne dépend que des bougies d'avant et d'elle-même.

use crate::num::{fix, to_dec, OSC_DP, PRICE_DP, RATIO_DP};
use crate::Weights;
use argos_core::{aggregate, bucket_start, Bar, Timeframe};
use argos_script::indicators::{ai_trend_volume, classic, smc};
use argos_script::inputs::{defaults, Inputs};
use argos_script::ta;
use argos_script::v1_inputs::AI_TREND_PARAMS;
use rust_decimal::Decimal;

/// Bougies de la fenêtre du percentile de l'ATR.
const ATR_PCT_LEN: usize = 100;
/// Percentile de l'ATR à partir duquel le régime est HIGH_VOLATILITY.
const HIGH_VOL_PCT: f64 = 90.0;
/// ADX à partir duquel le marché est en tendance (régime).
const ADX_TREND: f64 = 25.0;
/// ADX à partir duquel le facteur « force » compte.
const ADX_STRONG: f64 = 20.0;
/// Score minimal d'un setup.
const SETUP_SCORE: u8 = 70;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Long,
    Short,
}

impl Direction {
    pub fn id(self) -> &'static str {
        match self {
            Self::Long => "LONG",
            Self::Short => "SHORT",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Regime {
    TrendUp,
    TrendDown,
    Range,
    HighVolatility,
}

impl Regime {
    pub fn id(self) -> &'static str {
        match self {
            Self::TrendUp => "TREND_UP",
            Self::TrendDown => "TREND_DOWN",
            Self::Range => "RANGE",
            Self::HighVolatility => "HIGH_VOLATILITY",
        }
    }
}

/// Biais EMA 20 / 50 de l'unité de temps supérieure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bias {
    Up,
    Down,
    Neutral,
}

impl Bias {
    pub fn id(self) -> &'static str {
        match self {
            Self::Up => "UP",
            Self::Down => "DOWN",
            Self::Neutral => "NEUTRAL",
        }
    }
}

/// Motifs de bougie confirmés d'Argos AI Trend V Pro (`ai_trend_volume`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pattern {
    Hammer,
    InvertedHammer,
    BullishEngulfing,
    PiercingLine,
    BullishHarami,
    MorningStar,
    TweezerBottom,
    ThreeWhiteSoldiers,
    BullishAbandonedBaby,
    ShootingStar,
    HangingMan,
    BearishEngulfing,
    DarkCloudCover,
    BearishHarami,
    EveningStar,
    TweezerTop,
    ThreeBlackCrows,
    BearishAbandonedBaby,
}

impl Pattern {
    const BULL: [(&'static str, Pattern); 9] = [
        ("Hammer", Self::Hammer),
        ("Inv. Hammer", Self::InvertedHammer),
        ("Bullish Engulfing", Self::BullishEngulfing),
        ("Piercing Line", Self::PiercingLine),
        ("Bullish Harami", Self::BullishHarami),
        ("Morning Star", Self::MorningStar),
        ("Tweezer Bottom", Self::TweezerBottom),
        ("3 White Soldiers", Self::ThreeWhiteSoldiers),
        ("Abandoned Baby", Self::BullishAbandonedBaby),
    ];
    const BEAR: [(&'static str, Pattern); 9] = [
        ("Shooting Star", Self::ShootingStar),
        ("Hanging Man", Self::HangingMan),
        ("Bearish Engulfing", Self::BearishEngulfing),
        ("Dark Cloud Cover", Self::DarkCloudCover),
        ("Bearish Harami", Self::BearishHarami),
        ("Evening Star", Self::EveningStar),
        ("Tweezer Top", Self::TweezerTop),
        ("3 Black Crows", Self::ThreeBlackCrows),
        ("Abandoned Baby", Self::BearishAbandonedBaby),
    ];

    fn from_name(name: &str, bull: bool) -> Option<Self> {
        let table = if bull { &Self::BULL } else { &Self::BEAR };
        table.iter().find(|(n, _)| *n == name).map(|&(_, p)| p)
    }

    pub fn direction(self) -> Direction {
        if Self::BULL.iter().any(|&(_, p)| p == self) {
            Direction::Long
        } else {
            Direction::Short
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            Self::Hammer => "HAMMER",
            Self::InvertedHammer => "INVERTED_HAMMER",
            Self::BullishEngulfing => "BULLISH_ENGULFING",
            Self::PiercingLine => "PIERCING_LINE",
            Self::BullishHarami => "BULLISH_HARAMI",
            Self::MorningStar => "MORNING_STAR",
            Self::TweezerBottom => "TWEEZER_BOTTOM",
            Self::ThreeWhiteSoldiers => "THREE_WHITE_SOLDIERS",
            Self::BullishAbandonedBaby => "BULLISH_ABANDONED_BABY",
            Self::ShootingStar => "SHOOTING_STAR",
            Self::HangingMan => "HANGING_MAN",
            Self::BearishEngulfing => "BEARISH_ENGULFING",
            Self::DarkCloudCover => "DARK_CLOUD_COVER",
            Self::BearishHarami => "BEARISH_HARAMI",
            Self::EveningStar => "EVENING_STAR",
            Self::TweezerTop => "TWEEZER_TOP",
            Self::ThreeBlackCrows => "THREE_BLACK_CROWS",
            Self::BearishAbandonedBaby => "BEARISH_ABANDONED_BABY",
        }
    }
}

/// Setup sur bougie clôturée : entrée = clôture, stop à 1,5 × ATR, cibles à 1,5 R et 3 R. Aucune taille, aucun montant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Setup {
    pub side: Direction,
    pub entry: Decimal,
    pub stop: Decimal,
    pub tp1: Decimal,
    pub tp2: Decimal,
}

impl Setup {
    /// Entrée, stop, cible 1 et cible 2 en `f64`, pour le moteur de backtest et la position à l'écran (le paquet et le
    /// texte affiché gardent les décimaux).
    pub fn niveaux_f64(&self) -> [f64; 4] {
        [self.entry, self.stop, self.tp1, self.tp2].map(crate::num::to_f64)
    }
}

/// Photo d'une bougie clôturée. Champs dans l'ordre du paquet ; `None` = donnée absente (`null`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    pub close: Option<Decimal>,
    pub ema20: Option<Decimal>,
    pub ema50: Option<Decimal>,
    pub ema200: Option<Decimal>,
    pub rsi14: Option<Decimal>,
    pub macd: Option<Decimal>,
    pub macd_signal: Option<Decimal>,
    pub macd_hist: Option<Decimal>,
    pub plus_di: Option<Decimal>,
    pub minus_di: Option<Decimal>,
    pub adx14: Option<Decimal>,
    pub atr14: Option<Decimal>,
    pub atr_pct: Option<Decimal>,
    pub rel_volume: Option<Decimal>,
    pub vwap: Option<Decimal>,
    pub prev_day_high: Option<Decimal>,
    pub prev_day_low: Option<Decimal>,
    pub htf_bias: Option<Bias>,
    /// Sens de la dernière cassure de structure (BOS ou CHoCH) d'Argos OB · BB · FVG.
    pub structure: Option<Direction>,
    /// BOS sur cette bougie.
    pub bos: Option<Direction>,
    /// Motif confirmé sur cette bougie (aucun si un motif haussier et un baissier sortent ensemble).
    pub pattern: Option<Pattern>,
    pub regime: Option<Regime>,
    pub score_long: u8,
    pub score_short: u8,
    pub setup: Option<Setup>,
    /// What makes the Long score, argument by argument (not in the packet).
    pub for_long: Components,
    /// What makes the Short score, argument by argument (not in the packet).
    pub for_short: Components,
}

/// The 12 arguments of one side's score: `true` = the argument counts for that side (price "above" for Long, "below"
/// for Short). A missing value never counts. The score is computed from here, and only from here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Components {
    /// Trend (1 quarter each): price on the right side of EMA 50.
    pub ema50: bool,
    /// EMA 20 on the right side of EMA 50.
    pub ema20_ema50: bool,
    /// Price on the right side of EMA 200.
    pub ema200: bool,
    /// Higher-timeframe bias in this direction.
    pub bias: bool,
    /// Momentum (2 quarters each): RSI 14 on the right side of 50.
    pub rsi: bool,
    /// MACD histogram on the right side of 0.
    pub macd: bool,
    /// Strength (2 quarters each): ADX 14 at least 20 (the same for both sides).
    pub adx: bool,
    /// +DI above −DI (Long) or below (Short).
    pub di: bool,
    /// Volume (2 quarters each): relative volume at least 1 (the same for both sides).
    pub volume: bool,
    /// Price on the right side of VWAP.
    pub vwap: bool,
    /// Structure (2 quarters each): last structure break in this direction.
    pub structure: bool,
    /// BOS in this direction on this bar, or a confirmed candle pattern in this direction.
    pub trigger: bool,
}

impl Components {
    /// The 5 factors in quarters (0 to 4): trend, momentum, strength, volume, structure.
    pub fn quarters(&self) -> [u16; 5] {
        let q = |a: bool, b: bool, unit: u16| (u16::from(a) + u16::from(b)) * unit;
        [
            q(self.ema50, self.ema20_ema50, 1) + q(self.ema200, self.bias, 1),
            q(self.rsi, self.macd, 2),
            q(self.adx, self.di, 2),
            q(self.volume, self.vwap, 2),
            q(self.structure, self.trigger, 2),
        ]
    }

    /// The score (0 to 100) with these weights (sum 100).
    pub fn score(&self, w: Weights) -> u8 {
        let weights = [w.trend, w.momentum, w.strength, w.volume, w.structure];
        let total: u16 = self.quarters().iter().zip(weights).map(|(q, p)| q * u16::from(p)).sum();
        // weights sum to 100, so total ≤ 400 and the score ≤ 100
        u8::try_from((total + 2) / 4).unwrap_or(100)
    }
}

/// Séries calculées une fois sur tout l'historique clôturé.
pub(crate) struct Series {
    close: Vec<f64>,
    ema20: Vec<f64>,
    ema50: Vec<f64>,
    ema200: Vec<f64>,
    rsi: Vec<f64>,
    macd: classic::Macd,
    dmi: ta::Dmi,
    atr: Vec<f64>,
    atr_pct: Vec<f64>,
    rvol: Vec<f64>,
    vwap: Vec<f64>,
    pdh: Vec<f64>,
    pdl: Vec<f64>,
    bias: Vec<Option<Bias>>,
    structure: Vec<Option<Direction>>,
    bos: Vec<Option<Direction>>,
    pattern: Vec<Option<Pattern>>,
}

fn dir(up: bool) -> Direction {
    if up {
        Direction::Long
    } else {
        Direction::Short
    }
}

impl Series {
    pub(crate) fn new(bars: &[Bar], secs: i64, htf: Option<Timeframe>, htf_bars: Option<&[Bar]>) -> Self {
        let n = bars.len();
        let close: Vec<f64> = bars.iter().map(|b| b.close).collect();
        let atr = ta::atr(bars, 14);
        let atr_pct = (0..n)
            .map(|i| {
                let Some(w) = (i + 1).checked_sub(ATR_PCT_LEN).map(|s| &atr[s..=i]) else {
                    return f64::NAN;
                };
                if w.iter().all(|v| v.is_finite()) {
                    100.0 * w.iter().filter(|&&v| v <= atr[i]).count() as f64 / ATR_PCT_LEN as f64
                } else {
                    f64::NAN
                }
            })
            .collect();

        // structure : sens de la dernière cassure et BOS de la bougie
        let (mut structure, mut bos) = (vec![None; n], vec![None; n]);
        let mut breaks = smc::core(bars, &smc::Settings::default()).breaks.into_iter().peekable();
        let mut last = None;
        for i in 0..n {
            while let Some(b) = breaks.next_if(|b| b.i1 <= i) {
                last = Some(dir(b.up));
                if b.i1 == i && b.kind == smc::BreakType::Bos {
                    bos[i] = Some(dir(b.up));
                }
            }
            structure[i] = last;
        }

        // motifs confirmés (réglages par défaut d'Argos AI Trend V Pro)
        let settings = ai_trend_volume::Settings::from_inputs(&Inputs {
            params: &AI_TREND_PARAMS,
            vals: &defaults(&AI_TREND_PARAMS),
        });
        let vol = ai_trend_volume::compute(bars, secs, &settings);
        let (mut bull, mut bear) = (vec![None; n], vec![None; n]);
        for &(i, name) in &vol.bull_patterns {
            bull[i] = Pattern::from_name(name, true);
        }
        for &(i, name) in &vol.bear_patterns {
            bear[i] = Pattern::from_name(name, false);
        }
        let pattern = bull
            .iter()
            .zip(&bear)
            .map(|(a, b)| if a.is_some() == b.is_some() { None } else { a.or(*b) })
            .collect();

        let (pdh, pdl) = prev_day(bars, secs);
        Self {
            ema20: ta::ema(&close, 20),
            ema50: ta::ema(&close, 50),
            ema200: ta::ema(&close, 200),
            rsi: ta::rsi(&close, 14),
            macd: classic::macd(bars),
            dmi: ta::dmi(bars, 14),
            atr,
            atr_pct,
            rvol: classic::relative_volume(bars),
            vwap: classic::vwap(bars),
            pdh,
            pdl,
            bias: htf_bias(bars, secs, htf, htf_bars),
            structure,
            bos,
            pattern,
            close,
        }
    }

    pub(crate) fn snapshot(&self, i: usize, w: Weights) -> Snapshot {
        let c = self.close[i];
        let (e20, e50, e200) = (self.ema20[i], self.ema50[i], self.ema200[i]);
        let (rsi, hist) = (self.rsi[i], self.macd.hist[i]);
        let (plus, minus, adx) = (self.dmi.plus[i], self.dmi.minus[i], self.dmi.adx[i]);
        let (rvol, vwap, pct) = (self.rvol[i], self.vwap[i], self.atr_pct[i]);
        let (bias, structure, bos, pattern) = (self.bias[i], self.structure[i], self.bos[i], self.pattern[i]);

        let regime = [c, e20, e50, adx, pct].iter().all(|v| v.is_finite()).then_some({
            if pct >= HIGH_VOL_PCT {
                Regime::HighVolatility
            } else if adx >= ADX_TREND && e20 > e50 && c > e50 {
                Regime::TrendUp
            } else if adx >= ADX_TREND && e20 < e50 && c < e50 {
                Regime::TrendDown
            } else {
                Regime::Range
            }
        });

        // a missing value (NaN) never counts: any comparison with NaN is false
        let components = |long: bool| -> Components {
            let side = dir(long);
            let above = |a: f64, b: f64| if long { a > b } else { a < b };
            Components {
                ema50: above(c, e50),
                ema20_ema50: above(e20, e50),
                ema200: above(c, e200),
                bias: bias == Some(if long { Bias::Up } else { Bias::Down }),
                rsi: above(rsi, 50.0),
                macd: above(hist, 0.0),
                adx: adx >= ADX_STRONG,
                di: above(plus, minus),
                volume: rvol >= 1.0,
                vwap: above(c, vwap),
                structure: structure == Some(side),
                trigger: bos == Some(side) || pattern.is_some_and(|p| p.direction() == side),
            }
        };
        let (for_long, for_short) = (components(true), components(false));
        let (score_long, score_short) = (for_long.score(w), for_short.score(w));

        let close = to_dec(c, PRICE_DP);
        let atr14 = to_dec(self.atr[i], PRICE_DP);
        let side = if score_long >= SETUP_SCORE && score_long > score_short {
            Some(Direction::Long)
        } else if score_short >= SETUP_SCORE && score_short > score_long {
            Some(Direction::Short)
        } else {
            None
        };
        let setup = match (side, close, atr14) {
            (Some(side), Some(entry), Some(atr)) if atr > Decimal::ZERO => {
                let risk = fix(atr * Decimal::new(15, 1), PRICE_DP);
                let sign = if side == Direction::Long { Decimal::ONE } else { Decimal::NEGATIVE_ONE };
                Some(Setup {
                    side,
                    entry,
                    stop: fix(entry - sign * risk, PRICE_DP),
                    tp1: fix(entry + sign * risk * Decimal::new(15, 1), PRICE_DP),
                    tp2: fix(entry + sign * risk * Decimal::from(3), PRICE_DP),
                })
            }
            _ => None,
        };

        Snapshot {
            close,
            ema20: to_dec(e20, PRICE_DP),
            ema50: to_dec(e50, PRICE_DP),
            ema200: to_dec(e200, PRICE_DP),
            rsi14: to_dec(rsi, OSC_DP),
            macd: to_dec(self.macd.line[i], PRICE_DP),
            macd_signal: to_dec(self.macd.signal[i], PRICE_DP),
            macd_hist: to_dec(hist, PRICE_DP),
            plus_di: to_dec(plus, OSC_DP),
            minus_di: to_dec(minus, OSC_DP),
            adx14: to_dec(adx, OSC_DP),
            atr14,
            atr_pct: to_dec(pct, OSC_DP),
            rel_volume: to_dec(rvol, RATIO_DP),
            vwap: to_dec(vwap, PRICE_DP),
            prev_day_high: to_dec(self.pdh[i], PRICE_DP),
            prev_day_low: to_dec(self.pdl[i], PRICE_DP),
            htf_bias: bias,
            structure,
            bos,
            pattern,
            regime,
            score_long,
            score_short,
            setup,
            for_long,
            for_short,
        }
    }
}

/// Haut / bas du jour UTC précédent (jours construits avec `aggregate`, comme les PDH / PDL d'Argos SMC Pro).
/// Rien à partir de l'unité de temps journalière.
fn prev_day(bars: &[Bar], secs: i64) -> (Vec<f64>, Vec<f64>) {
    const DAY: i64 = 86_400;
    let n = bars.len();
    let (mut hi, mut lo) = (vec![f64::NAN; n], vec![f64::NAN; n]);
    if secs >= DAY {
        return (hi, lo);
    }
    let days = aggregate(bars, DAY);
    let mut k = 0;
    for (i, b) in bars.iter().enumerate() {
        let d = bucket_start(b.time, DAY);
        while days[k].time < d {
            k += 1;
        }
        if let Some(p) = k.checked_sub(1).map(|p| days[p]) {
            (hi[i], lo[i]) = (p.high, p.low);
        }
    }
    (hi, lo)
}

/// Biais de l'unité supérieure à la clôture de chaque bougie : seules les bougies supérieures déjà finies comptent.
/// Haussier si clôture > EMA 50 et EMA 20 > EMA 50, baissier à l'inverse, neutre sinon.
fn htf_bias(bars: &[Bar], secs: i64, htf: Option<Timeframe>, given: Option<&[Bar]>) -> Vec<Option<Bias>> {
    let n = bars.len();
    let Some(hs) = htf.map(Timeframe::seconds).filter(|&hs| hs > secs) else {
        return vec![None; n];
    };
    let built;
    let hb = match given {
        Some(h) => h,
        None => {
            built = aggregate(bars, hs);
            &built[..]
        }
    };
    let c: Vec<f64> = hb.iter().map(|b| b.close).collect();
    let (e20, e50) = (ta::ema(&c, 20), ta::ema(&c, 50));
    let mut k = 0;
    bars.iter()
        .map(|b| {
            let end = b.time.saturating_add(secs);
            while k < hb.len() && hb[k].time.saturating_add(hs) <= end {
                k += 1;
            }
            let j = k.checked_sub(1)?;
            let (c, a, s) = (c[j], e20[j], e50[j]);
            (a.is_finite() && s.is_finite()).then_some({
                if c > s && a > s {
                    Bias::Up
                } else if c < s && a < s {
                    Bias::Down
                } else {
                    Bias::Neutral
                }
            })
        })
        .collect()
}
