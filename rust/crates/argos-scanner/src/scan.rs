//! Stratégie double structure : le 4H et le Daily doivent pointer dans la même direction.

use crate::candle::Candle;
use crate::ta::{self, Bias, Structure};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Long,
    Short,
    Watch, // le daily est à moins de 2 % de son pivot de cassure dans le sens du 4H
    None,
}

impl Verdict {
    pub fn label(self) -> &'static str {
        match self {
            Verdict::Long => "✅ LONG",
            Verdict::Short => "✅ SHORT",
            Verdict::Watch => "👀 À SURVEILLER",
            Verdict::None => "❌ Pas aligné",
        }
    }
}

/// Plan de trade proposé quand la double structure est alignée.
#[derive(Debug, Clone)]
pub struct Plan {
    pub entry: f64,
    pub stop: f64,
    /// true si le SL structurel était trop loin et a été ramené à 2 ATR.
    pub atr_stop: bool,
    pub target: Option<f64>,
    pub rr: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct Report {
    pub coin: String,
    pub price: f64,
    pub daily: Structure,
    pub h4: Structure,
    pub rsi_4h: Option<f64>,
    pub stoch_4h: Option<f64>,
    pub ema200_4h: Option<f64>,
    pub verdict: Verdict,
    pub plan: Option<Plan>,
    pub note: String,
}

pub fn analyse(coin: &str, daily: &[Candle], h4: &[Candle], pivot: usize) -> Option<Report> {
    let d = ta::structure(daily, pivot)?;
    let s4 = ta::structure(h4, pivot)?;
    let closes: Vec<f64> = h4.iter().map(|c| c.c).collect();
    let price = s4.close;
    let ema20 = ta::ema(&closes, 20);
    let ema200 = ta::ema(&closes, 200);

    let mut note = String::new();
    let verdict = match (d.bias, s4.bias) {
        (Bias::Bull, Bias::Bull) => Verdict::Long,
        (Bias::Bear, Bias::Bear) => Verdict::Short,
        (_, Bias::Bull) if d.last_high.is_some_and(|h| price < h && (h - price) / price < 0.02) => {
            note = format!("Daily casse au-dessus de {:.4} = alignement LONG", d.last_high.unwrap());
            Verdict::Watch
        }
        (_, Bias::Bear) if d.last_low.is_some_and(|l| price > l && (price - l) / price < 0.02) => {
            note = format!("Daily casse sous {:.4} = alignement SHORT", d.last_low.unwrap());
            Verdict::Watch
        }
        (db, hb) => {
            note = format!("Daily {} / 4H {}", name(db), name(hb));
            Verdict::None
        }
    };

    let atr = ta::atr(h4, 14);
    let plan = match verdict {
        Verdict::Long => long_plan(&d, &s4, ema20, atr, price),
        Verdict::Short => short_plan(&d, &s4, ema20, atr, price),
        _ => None,
    };

    Some(Report {
        coin: coin.to_string(),
        price,
        daily: d,
        h4: s4,
        rsi_4h: ta::rsi(&closes, 14),
        stoch_4h: ta::stoch_k(h4, 14),
        ema200_4h: ema200,
        verdict,
        plan,
        note,
    })
}

fn name(b: Bias) -> &'static str {
    match b {
        Bias::Bull => "haussier",
        Bias::Bear => "baissier",
        Bias::Range => "range",
    }
}

/// Choisit la cible : le premier niveau daily qui donne au moins 2R, sinon le plus éloigné.
fn pick_target(levels: &[f64], entry: f64, risk: f64, long: bool) -> Option<f64> {
    let mut beyond: Vec<f64> = levels
        .iter()
        .copied()
        .filter(|&x| if long { x > entry * 1.005 } else { x < entry * 0.995 })
        .collect();
    beyond.sort_by(|a, b| {
        let (da, db) = ((a - entry).abs(), (b - entry).abs());
        da.partial_cmp(&db).unwrap()
    });
    beyond
        .iter()
        .copied()
        .find(|&x| (x - entry).abs() / risk >= 2.0)
        .or_else(|| beyond.last().copied())
}

/// Ramène un SL trop éloigné (plus de 2,5 ATR) à 2 ATR de l'entrée.
fn cap_stop(entry: f64, structural: f64, atr: Option<f64>, long: bool) -> (f64, bool) {
    match atr {
        Some(a) if (entry - structural).abs() > 2.5 * a => (if long { entry - 2.0 * a } else { entry + 2.0 * a }, true),
        _ => (structural, false),
    }
}

/// Entrée sur le retest du dernier sommet 4H cassé (ou l'EMA 20 4H), SL sous le dernier creux 4H,
/// cible = niveau daily au-dessus.
fn long_plan(d: &Structure, s4: &Structure, ema20: Option<f64>, atr: Option<f64>, price: f64) -> Option<Plan> {
    let entry = match (s4.bos_up, s4.last_high, ema20) {
        (true, Some(h), _) => h,
        (_, _, Some(e)) if e < price => e,
        _ => price,
    };
    let (stop, atr_stop) = cap_stop(entry, s4.last_low? * 0.997, atr, true);
    if stop >= entry {
        return None;
    }
    let target = pick_target(&d.highs, entry, entry - stop, true);
    let rr = target.map(|t| (t - entry) / (entry - stop));
    Some(Plan {
        entry,
        stop,
        atr_stop,
        target,
        rr,
    })
}

fn short_plan(d: &Structure, s4: &Structure, ema20: Option<f64>, atr: Option<f64>, price: f64) -> Option<Plan> {
    let entry = match (s4.bos_down, s4.last_low, ema20) {
        (true, Some(l), _) => l,
        (_, _, Some(e)) if e > price => e,
        _ => price,
    };
    let (stop, atr_stop) = cap_stop(entry, s4.last_high? * 1.003, atr, false);
    if stop <= entry {
        return None;
    }
    let target = pick_target(&d.lows, entry, stop - entry, false);
    let rr = target.map(|t| (entry - t) / (stop - entry));
    Some(Plan {
        entry,
        stop,
        atr_stop,
        target,
        rr,
    })
}
