//! 🤖 Idea 2: « Accord entre unités de temps ». Each indicator on the chart is read on the chart's unit and on a higher
//! unit, built here from the chart's closed bars (only whole higher bars: the one under way does not count), and the
//! two senses are put side by side. A plain account: same sense, opposite senses, not enough bars. Never an order.

use crate::together::{list, plural, Item, Overview};
use crate::{read, Block, Coverage, Lean, Sense, Sentence, REMINDER};
use argos_core::types::MONTH_SECONDS;
use argos_core::{bucket_start, Bar, Interval};

/// Higher units offered, from the smallest (5 min … 1 month).
const LADDER: [i64; 10] = [300, 900, 1_800, 3_600, 7_200, 14_400, 43_200, 86_400, 604_800, MONTH_SECONDS];

/// The higher unit advised for a chart of `bar_secs` seconds: the smallest of 5m, 15m, 30m, 1h, 2h, 4h, 12h, 1D, 1W
/// and 1M that is at least 4 times longer and holds a whole number of chart bars (months always fit). 15m → 1h,
/// 1h → 4h, 4h → 1D, 1D → 1W; `None` from 1 month up.
pub fn higher_timeframe(bar_secs: i64) -> Option<i64> {
    (bar_secs > 0).then_some(())?;
    LADDER.into_iter().find(|&t| t >= 4 * bar_secs && (t % bar_secs == 0 || t == MONTH_SECONDS))
}

/// The name of a unit, as the chart writes it (« 15m », « 4h », « 1D », « 1W »).
pub fn timeframe_label(secs: i64) -> String {
    Interval(secs).short()
}

/// The end of the higher bar starting at `t`.
fn bucket_end(t: i64, higher: i64) -> i64 {
    if higher % MONTH_SECONDS == 0 {
        // a month is not a fixed number of seconds: the start of the next one
        bucket_start(t + higher + 4 * 86_400, higher)
    } else {
        t + higher
    }
}

/// The whole higher bars built from `bars` (closed bars of `bar_secs` seconds, oldest first): a higher bar is kept
/// when the chart bars cover it from its start to its end.
pub(crate) fn whole_higher_bars(bars: &[Bar], bar_secs: i64, higher: i64) -> Vec<Bar> {
    let mut out: Vec<(Bar, i64, i64)> = vec![]; // (higher bar, first bar time, last bar end)
    for b in bars {
        let t = bucket_start(b.time, higher);
        match out.last_mut() {
            Some((x, _, end)) if x.time == t => {
                x.high = x.high.max(b.high);
                x.low = x.low.min(b.low);
                x.close = b.close;
                x.volume += b.volume;
                *end = b.time + bar_secs;
            }
            _ => out.push((Bar { time: t, ..*b }, b.time, b.time + bar_secs)),
        }
    }
    out.into_iter()
        .filter(|(x, first, end)| *first == x.time && *end >= bucket_end(x.time, higher))
        .map(|(x, _, _)| x)
        .collect()
}

/// « à la baisse (en survente, 17,3) », « pic de volume (11,19 fois la moyenne) ».
fn describe(x: &Sense) -> String {
    let lean = match x.lean {
        Lean::Up => Some("à la hausse"),
        Lean::Down => Some("à la baisse"),
        Lean::Neutral => Some("sans sens net"),
        Lean::NoDirection => None,
    };
    let extra: Vec<&str> = [lean.and(x.note), x.value.as_deref()].into_iter().flatten().collect();
    let head = lean.or(x.note).unwrap_or("");
    if extra.is_empty() {
        head.to_string()
    } else {
        format!("{head} ({})", extra.join(", "))
    }
}

fn s(id: &'static str, text: String) -> Sentence {
    Sentence { id, text }
}

/// Reads the `items` (the indicators on the chart) on the chart's unit (`bar_secs`) and on `higher_secs`, from the
/// chart's `closed_bars` (closed bars only, oldest first).
pub fn read_timeframes(items: &[Item], closed_bars: &[Bar], bar_secs: i64, higher_secs: i64) -> Overview {
    let (l1, l2) = (timeframe_label(bar_secs), timeframe_label(higher_secs));
    let higher = whole_higher_bars(closed_bars, bar_secs, higher_secs);
    let (mut same, mut opposite, mut unclear, mut lines, mut not_read) = (vec![], vec![], vec![], vec![], vec![]);
    for it in items {
        if it.id == "Vwap" {
            lines.push(s(
                "vwap",
                format!("{} : il repart chaque jour, c'est le même sur les deux unités ; il n'est pas comparé.", it.name),
            ));
            continue;
        }
        let (r1, r2) = (read(it.id, closed_bars, it.settings), read(it.id, &higher, it.settings));
        if r1.coverage == Coverage::SettingsOnly {
            not_read.push(it.name.to_string());
            continue;
        }
        let (a, b) = match (r1.sense, r2.sense) {
            (None, _) => {
                lines.push(s("too_few", format!("{} : pas assez de bougies en {l1}.", it.name)));
                continue;
            }
            (Some(_), None) => {
                lines.push(s("too_few", format!("{} : pas assez de bougies en {l2}.", it.name)));
                continue;
            }
            (Some(a), Some(b)) => (a, b),
        };
        let both = format!("{} : {l1} {}, {l2} {}", it.name, describe(&a), describe(&b));
        let verdict = match (a.lean, b.lean) {
            (Lean::NoDirection, _) | (_, Lean::NoDirection) => None,
            (x, y) if x == y && x != Lean::Neutral => {
                same.push(it.name.to_string());
                Some(format!("même sens, {}", if x == Lean::Up { "à la hausse" } else { "à la baisse" }))
            }
            (Lean::Up, Lean::Down) | (Lean::Down, Lean::Up) => {
                opposite.push(it.name.to_string());
                Some("pas d'accord".to_string())
            }
            _ => {
                unclear.push(it.name.to_string());
                Some("pas de sens net sur une des deux".to_string())
            }
        };
        lines.push(s("pair", verdict.map_or_else(|| format!("{both}."), |v| format!("{both} : {v}."))));
    }
    if !not_read.is_empty() {
        lines.push(s("not_read", format!("Pas encore lus par Argos AI : {}.", list(&not_read))));
    }
    let compared = same.len() + opposite.len() + unclear.len();
    let mut summary = vec![s(
        "bars",
        format!(
            "{} {} {l2} {} à partir des bougies {l1} clôturées.",
            higher.len(),
            plural(higher.len(), "bougie", "bougies"),
            plural(higher.len(), "complète construite", "complètes construites")
        ),
    )];
    summary.push(if compared == 0 {
        s("none", "Aucun indicateur avec un sens à comparer sur les deux unités.".into())
    } else {
        let mut parts = vec![];
        if !same.is_empty() {
            parts.push(format!("{} dans le même sens ({})", same.len(), list(&same)));
        }
        if !opposite.is_empty() {
            parts.push(format!("{} pas d'accord ({})", opposite.len(), list(&opposite)));
        }
        if !unclear.is_empty() {
            parts.push(format!("{} sans sens net sur une des deux ({})", unclear.len(), list(&unclear)));
        }
        s(
            "count",
            format!(
                "Sur {compared} {} : {}.",
                plural(compared, "indicateur comparé", "indicateurs comparés"),
                parts.join(", ")
            ),
        )
    });
    let how = vec![
        s(
            "how_units",
            format!("L'unité supérieure ({l2}) montre le fond ; l'unité de la chart ({l1}), le moment."),
        ),
        s(
            "how_agree",
            "Quand les deux vont dans le même sens, la lecture est plus nette ; quand elles s'opposent, le mouvement de la chart va contre le fond.".into(),
        ),
        s(
            "how_bars",
            format!("Les bougies {l2} sont construites avec les bougies {l1} clôturées ; la bougie {l2} en cours ne compte pas."),
        ),
    ];
    Overview {
        blocks: vec![
            Block {
                title: "Accord entre unités de temps",
                sentences: summary,
            },
            Block {
                title: "Indicateur par indicateur",
                sentences: lines,
            },
            Block {
                title: "Comment le lire",
                sentences: how,
            },
        ],
        reminder: REMINDER,
    }
}
