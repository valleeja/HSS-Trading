//! Example Argos interface v2 module: the simple moving average of the bars.
//! No rights at all: it receives the bars and its settings, and returns one
//! line and one card. Integers only: the average keeps the price scale and
//! is rounded half away from zero.
#![forbid(unsafe_code)]

use argos_module_sdk::{Bar, Card, Input, Line, Output, EMPTY};

/// Default length, as in `argos-module.json`.
pub const DEFAULT_LENGTH: i64 = 20;
/// Largest length, as in `argos-module.json`.
pub const MAX_LENGTH: i64 = 500;

/// The bar value a `source` setting picks (index of its choice).
fn pick(bar: &Bar, source: i64) -> i64 {
    match source {
        1 => bar.o,
        2 => bar.h,
        3 => bar.l,
        _ => bar.c,
    }
}

/// `sum / n` rounded half away from zero (`n > 0`).
fn rounded_div(sum: i128, n: i128) -> i128 {
    let half = n / 2;
    if sum >= 0 {
        (sum + half) / n
    } else {
        (sum - half) / n
    }
}

/// The moving average of `values` over `length`: [`EMPTY`] until the window
/// is full or while it holds an [`EMPTY`] value.
pub fn moving_average(values: &[i64], length: usize) -> Vec<i64> {
    let mut out = Vec::with_capacity(values.len());
    let mut sum: i128 = 0;
    let mut empty = 0_usize;
    let n = i128::try_from(length).unwrap_or(i128::MAX);
    for (i, &value) in values.iter().enumerate() {
        if value == EMPTY {
            empty += 1;
        } else {
            sum += i128::from(value);
        }
        if let Some(old) = i.checked_sub(length).map(|j| values[j]) {
            if old == EMPTY {
                empty -= 1;
            } else {
                sum -= i128::from(old);
            }
        }
        let full = i + 1 >= length && length > 0 && empty == 0;
        out.push(if full {
            // The average of i64 values always fits an i64.
            i64::try_from(rounded_div(sum, n)).unwrap_or(EMPTY)
        } else {
            EMPTY
        });
    }
    out
}

/// Entry logic: settings `length` (1 to 500) and `source` (close, open,
/// high, low). The host checked them; a value out of bounds still gives a
/// line of [`EMPTY`] values, never a panic.
pub fn run(input: Input) -> Output {
    let length = input.settings.first().copied().unwrap_or(DEFAULT_LENGTH);
    let source = input.settings.get(1).copied().unwrap_or(0);
    let length = usize::try_from(length)
        .ok()
        .filter(|l| (1..=500).contains(l))
        .unwrap_or(0);
    let values: Vec<i64> = input.bars.iter().map(|b| pick(b, source)).collect();
    let average = moving_average(&values, length);
    let last = average.last().copied().unwrap_or(EMPTY);
    Output {
        lines: vec![Line {
            name: format!("Moyenne {length}"),
            scale: input.price_scale,
            values: average,
        }],
        labels: Vec::new(),
        cards: vec![Card {
            name: "Dernière moyenne".into(),
            value: last,
            scale: input.price_scale,
        }],
    }
}

argos_module_sdk::entry_data!(run);

#[cfg(test)]
mod tests {
    use super::{moving_average, run, EMPTY};
    use argos_module_sdk::{Bar, Input};

    fn input(closes: &[i64], settings: Vec<i64>) -> Input {
        let bars = closes
            .iter()
            .zip(0..)
            .map(|(&c, t)| Bar {
                t,
                o: c,
                h: c,
                l: c,
                c,
                v: 1,
            })
            .collect();
        Input {
            timeframe: 60,
            price_scale: 2,
            volume_scale: 0,
            symbol: "TEST".into(),
            bars,
            settings,
        }
    }

    #[test]
    fn averages_the_last_bars() {
        assert_eq!(
            moving_average(&[10, 20, 30, 40], 2),
            vec![EMPTY, 15, 25, 35]
        );
        assert_eq!(moving_average(&[1, 2], 2), vec![EMPTY, 2]); // 1.5 rounds to 2
        assert_eq!(moving_average(&[-1, -2], 2), vec![EMPTY, -2]);
        assert_eq!(moving_average(&[5, 6, 7], 1), vec![5, 6, 7]);
    }

    #[test]
    fn empty_values_and_extremes_never_panic() {
        assert_eq!(moving_average(&[], 3), Vec::<i64>::new());
        assert_eq!(moving_average(&[1, 2], 5), vec![EMPTY, EMPTY]);
        assert_eq!(
            moving_average(&[1, EMPTY, 3, 4], 2),
            vec![EMPTY, EMPTY, EMPTY, 4]
        );
        assert_eq!(
            moving_average(&[i64::MAX, i64::MAX], 2),
            vec![EMPTY, i64::MAX]
        );
        assert_eq!(
            moving_average(&[i64::MIN + 1, i64::MIN + 1], 2),
            vec![EMPTY, i64::MIN + 1]
        );
        assert_eq!(moving_average(&[1, 2], 0), vec![EMPTY, EMPTY]);
    }

    #[test]
    fn settings_out_of_bounds_give_empty_values() {
        for settings in [vec![0, 0], vec![501, 0], vec![-3, 0], vec![2, 99]] {
            let out = run(input(&[100, 200, 300], settings.clone()));
            assert_eq!(out.lines[0].values.len(), 3, "{settings:?}");
        }
        assert_eq!(
            run(input(&[100, 200], vec![0, 0])).lines[0].values,
            vec![EMPTY, EMPTY]
        );
        // No settings at all: the default length.
        assert_eq!(run(input(&[1; 25], vec![])).lines[0].name, "Moyenne 20");
    }

    #[test]
    fn the_output_has_one_value_per_bar_and_a_card() {
        let out = run(input(&[100, 200, 300], vec![2, 0]));
        assert_eq!(out.lines[0].values, vec![EMPTY, 150, 250]);
        assert_eq!(out.lines[0].scale, 2);
        assert_eq!(out.cards[0].value, 250);
        assert_eq!(run(input(&[], vec![2, 0])).cards[0].value, EMPTY);
    }
}
