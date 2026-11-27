//! 📈 Module Argos HSS Trading « EMA » (kit module Argos, interface v2) : la moyenne mobile exponentielle des bougies,
//! démarrée par la moyenne simple des `length` premières valeurs (même calcul que l'EMA intégrée d'Argos,
//! `argos_script::ta::ema`). Aucun droit : il reçoit les bougies et ses réglages, rend une courbe et une carte.
//! Les prix arrivent en entiers à l'échelle `price_scale` ; le calcul se fait en nombres à virgule, puis la courbe
//! repart en entiers à l'échelle `price_scale + 4` (arrondi une fois, moitié loin de zéro), `EMPTY` = pas de valeur.
#![forbid(unsafe_code)]

use argos_module_sdk::{Bar, Card, Input, Line, Output, EMPTY};

/// Longueur par défaut et la plus grande, comme dans `argos-module.json`.
pub const DEFAULT_LENGTH: i64 = 20;
pub const MAX_LENGTH: i64 = 500;
/// Décimales ajoutées à l'échelle des prix pour la courbe.
pub const EXTRA_DECIMALS: u8 = 4;

/// La valeur de bougie choisie par le réglage `source` (index du choix).
fn pick(bar: &Bar, source: i64) -> i64 {
    match source {
        1 => bar.o,
        2 => bar.h,
        3 => bar.l,
        _ => bar.c,
    }
}

/// EMA démarrée par la moyenne simple des `len` premières valeurs présentes (les vides du début sont sautés ; un vide
/// plus loin garde la dernière valeur) ; `NaN` avant.
pub fn ema(src: &[f64], len: usize) -> Vec<f64> {
    let mut out = vec![f64::NAN; src.len()];
    let Some(first) = src.iter().position(|v| v.is_finite()) else { return out };
    if len == 0 || src.len() < first + len {
        return out;
    }
    let alpha = 2.0 / (len as f64 + 1.0);
    let mut prev = src[first..first + len].iter().sum::<f64>() / len as f64;
    out[first + len - 1] = prev;
    for i in first + len..src.len() {
        if src[i].is_finite() {
            prev = alpha * src[i] + (1.0 - alpha) * prev;
        }
        out[i] = prev;
    }
    out
}

/// Un nombre à virgule en entier à `scale` décimales (moitié loin de zéro) ; `NaN` ou trop grand : `EMPTY`.
fn to_int(x: f64, scale: u8) -> i64 {
    let v = (x * 10f64.powi(i32::from(scale))).round();
    if v.is_finite() && v.abs() < 9.0e18 {
        v as i64
    } else {
        EMPTY
    }
}

/// L'entrée du module.
pub fn run(input: Input) -> Output {
    let length = input.settings.first().copied().unwrap_or(DEFAULT_LENGTH).clamp(1, MAX_LENGTH);
    let source = input.settings.get(1).copied().unwrap_or(0);
    let k = 10f64.powi(i32::from(input.price_scale));
    let values: Vec<f64> = input
        .bars
        .iter()
        .map(|b| {
            let v = pick(b, source);
            if v == EMPTY {
                f64::NAN
            } else {
                v as f64 / k
            }
        })
        .collect();
    let scale = (input.price_scale + EXTRA_DECIMALS).min(12);
    let line: Vec<i64> = ema(&values, length as usize).into_iter().map(|x| to_int(x, scale)).collect();
    let mut cards = Vec::new();
    if let Some(&last) = line.iter().rev().find(|v| **v != EMPTY) {
        cards.push(Card {
            name: "Dernière EMA".into(),
            value: last,
            scale,
        });
    }
    Output {
        lines: vec![Line {
            name: format!("EMA {length}"),
            scale,
            values: line,
        }],
        labels: Vec::new(),
        cards,
    }
}

argos_module_sdk::entry_data!(run);

#[cfg(test)]
mod tests {
    use super::*;

    fn input(closes: &[i64], settings: Vec<i64>) -> Input {
        Input {
            timeframe: 60,
            price_scale: 2,
            volume_scale: 0,
            symbol: "X".into(),
            bars: closes
                .iter()
                .enumerate()
                .map(|(i, &c)| Bar {
                    t: i as i64 * 60,
                    o: c,
                    h: c,
                    l: c,
                    c,
                    v: 1,
                })
                .collect(),
            settings,
        }
    }

    #[test]
    fn ema_demarree_par_la_moyenne_simple() {
        // 1,00 2,00 3,00 4,00 ; EMA 2 : 1,50 à la 2e, puis 0,667 × 3 + 0,333 × 1,5 = 2,5, puis 3,5
        let out = run(input(&[100, 200, 300, 400], vec![2, 0]));
        let l = &out.lines[0];
        assert_eq!((l.name.as_str(), l.scale), ("EMA 2", 6));
        assert_eq!(l.values, vec![EMPTY, 1_500_000, 2_500_000, 3_500_000]);
        assert_eq!(out.cards[0].value, 3_500_000);
        assert!(run(input(&[], vec![20, 0])).lines[0].values.is_empty());
    }
}
