//! Analyse technique : EMA, RSI, Stoch, pivots et structure de marché.

use crate::candle::Candle;

/// EMA classique, initialisée sur la première valeur (comme les scans faits dans le chat).
pub fn ema(values: &[f64], len: usize) -> Option<f64> {
    if values.is_empty() || len == 0 {
        return None;
    }
    let k = 2.0 / (len as f64 + 1.0);
    let mut e = values[0];
    for &v in &values[1..] {
        e = v * k + e * (1.0 - k);
    }
    Some(e)
}

/// RSI de Wilder.
pub fn rsi(values: &[f64], len: usize) -> Option<f64> {
    if values.len() <= len {
        return None;
    }
    let (mut gain, mut loss) = (0.0, 0.0);
    for i in 1..=len {
        let d = values[i] - values[i - 1];
        if d > 0.0 {
            gain += d
        } else {
            loss -= d
        }
    }
    gain /= len as f64;
    loss /= len as f64;
    for i in (len + 1)..values.len() {
        let d = values[i] - values[i - 1];
        gain = (gain * (len as f64 - 1.0) + d.max(0.0)) / len as f64;
        loss = (loss * (len as f64 - 1.0) + (-d).max(0.0)) / len as f64;
    }
    if loss == 0.0 {
        return Some(100.0);
    }
    Some(100.0 - 100.0 / (1.0 + gain / loss))
}

/// Stochastique %K brut sur `len` chandelles.
pub fn stoch_k(c: &[Candle], len: usize) -> Option<f64> {
    if c.len() < len {
        return None;
    }
    let w = &c[c.len() - len..];
    let hh = w.iter().map(|x| x.h).fold(f64::MIN, f64::max);
    let ll = w.iter().map(|x| x.l).fold(f64::MAX, f64::min);
    if hh == ll {
        return Some(50.0);
    }
    Some((c[c.len() - 1].c - ll) / (hh - ll) * 100.0)
}

/// ATR de Wilder (Average True Range).
pub fn atr(c: &[Candle], len: usize) -> Option<f64> {
    if c.len() <= len {
        return None;
    }
    let tr = |i: usize| {
        let p = c[i - 1].c;
        (c[i].h - c[i].l).max((c[i].h - p).abs()).max((c[i].l - p).abs())
    };
    let mut a = (1..=len).map(tr).sum::<f64>() / len as f64;
    for i in (len + 1)..c.len() {
        a = (a * (len as f64 - 1.0) + tr(i)) / len as f64;
    }
    Some(a)
}

/// Un pivot confirmé (sommet ou creux).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pivot {
    pub idx: usize,
    pub price: f64,
}

/// Pivots hauts et bas confirmés avec `l` chandelles de chaque côté.
pub fn pivots(c: &[Candle], l: usize) -> (Vec<Pivot>, Vec<Pivot>) {
    let (mut highs, mut lows) = (Vec::new(), Vec::new());
    if c.len() < 2 * l + 1 {
        return (highs, lows);
    }
    for i in l..c.len() - l {
        let mut is_h = true;
        let mut is_l = true;
        for j in 1..=l {
            if c[i - j].h >= c[i].h || c[i + j].h > c[i].h {
                is_h = false;
            }
            if c[i - j].l <= c[i].l || c[i + j].l < c[i].l {
                is_l = false;
            }
        }
        if is_h {
            highs.push(Pivot { idx: i, price: c[i].h });
        }
        if is_l {
            lows.push(Pivot { idx: i, price: c[i].l });
        }
    }
    (highs, lows)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bias {
    Bull,
    Bear,
    Range,
}

impl Bias {
    pub fn emoji(self) -> &'static str {
        match self {
            Bias::Bull => "🟢",
            Bias::Bear => "🔴",
            Bias::Range => "⚪",
        }
    }
}

/// Lecture de structure d'un timeframe.
#[derive(Debug, Clone)]
pub struct Structure {
    #[allow(dead_code)]
    /// Structure des 2 derniers pivots (HH+HL, LH+LL, ou range).
    pub swings: Bias,
    /// Cassure du dernier pivot par la clôture actuelle.
    pub bos_up: bool,
    pub bos_down: bool,
    /// Biais final : une cassure l'emporte sur la structure des pivots.
    pub bias: Bias,
    /// La cassure va contre la structure précédente (CHoCH).
    pub choch: bool,
    pub last_high: Option<f64>,
    pub last_low: Option<f64>,
    pub highs: Vec<f64>,
    pub lows: Vec<f64>,
    pub close: f64,
}

pub fn structure(c: &[Candle], l: usize) -> Option<Structure> {
    let close = c.last()?.c;
    let (hs, ls) = pivots(c, l);
    let swings = if hs.len() >= 2 && ls.len() >= 2 {
        let (h0, h1) = (hs[hs.len() - 2].price, hs[hs.len() - 1].price);
        let (l0, l1) = (ls[ls.len() - 2].price, ls[ls.len() - 1].price);
        if h1 > h0 && l1 > l0 {
            Bias::Bull
        } else if h1 < h0 && l1 < l0 {
            Bias::Bear
        } else {
            Bias::Range
        }
    } else {
        Bias::Range
    };
    let last_high = hs.last().map(|p| p.price);
    let last_low = ls.last().map(|p| p.price);
    let bos_up = last_high.is_some_and(|h| close > h);
    let bos_down = last_low.is_some_and(|lo| close < lo);
    let bias = if bos_up {
        Bias::Bull
    } else if bos_down {
        Bias::Bear
    } else {
        swings
    };
    let choch = (bos_up && swings == Bias::Bear) || (bos_down && swings == Bias::Bull);
    Some(Structure {
        swings,
        bos_up,
        bos_down,
        bias,
        choch,
        last_high,
        last_low,
        highs: hs.iter().map(|p| p.price).collect(),
        lows: ls.iter().map(|p| p.price).collect(),
        close,
    })
}

impl Structure {
    pub fn label(&self) -> String {
        let mut s = match self.bias {
            Bias::Bull => "Bull".to_string(),
            Bias::Bear => "Bear".to_string(),
            Bias::Range => "Range".to_string(),
        };
        if self.choch {
            s.push_str(if self.bos_up { " CHoCH↑" } else { " CHoCH↓" });
        } else if self.bos_up {
            s.push_str(" BOS↑");
        } else if self.bos_down {
            s.push_str(" BOS↓");
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(h: f64, l: f64, close: f64) -> Candle {
        Candle {
            t: 0,
            o: close,
            h,
            l,
            c: close,
        }
    }

    #[test]
    fn ema_constant_series() {
        assert!((ema(&[5.0; 30], 20).unwrap() - 5.0).abs() < 1e-9);
    }

    #[test]
    fn rsi_only_up_is_100() {
        let v: Vec<f64> = (0..30).map(|x| x as f64).collect();
        assert_eq!(rsi(&v, 14).unwrap(), 100.0);
    }

    #[test]
    fn detects_bull_structure_and_bos() {
        // Deux creux montants, deux sommets montants, puis clôture au-dessus du dernier sommet.
        let highs = [10., 11., 12., 11., 10., 12., 13., 14., 13., 12., 13., 15., 16.];
        let lows = [8., 9., 10., 9., 7., 9., 11., 12., 10., 9., 10., 12., 15.];
        let closes = [9., 10., 11., 10., 8., 11., 12., 13., 11., 10., 12., 14., 15.5];
        let candles: Vec<Candle> = (0..highs.len()).map(|i| c(highs[i], lows[i], closes[i])).collect();
        let s = structure(&candles, 2).unwrap();
        assert_eq!(s.swings, Bias::Bull);
        assert!(s.bos_up);
        assert_eq!(s.bias, Bias::Bull);
    }

    #[test]
    fn detects_choch_down() {
        // Structure haussière puis clôture sous le dernier creux.
        let highs = [10., 11., 12., 11., 10., 12., 13., 14., 13., 12., 13., 15., 13.];
        let lows = [8., 9., 10., 9., 7., 9., 11., 12., 10., 9., 10., 12., 8.5];
        let closes = [9., 10., 11., 10., 8., 11., 12., 13., 11., 10., 12., 14., 8.6];
        let candles: Vec<Candle> = (0..highs.len()).map(|i| c(highs[i], lows[i], closes[i])).collect();
        let s = structure(&candles, 2).unwrap();
        assert_eq!(s.swings, Bias::Bull);
        assert!(s.bos_down);
        assert!(s.choch);
        assert_eq!(s.bias, Bias::Bear);
    }
}
