//! The reading of « Argos Patterns auto »: the very figures the chart draws (double and triple tops and bottoms,
//! head and shoulders, triangles, wedges, channels, the auto Fibonacci channel), from
//! `argos_script::indicators::patterns::compute` with the same settings (no second computation). The script keeps the
//! « keep » most recent figures and leaves out the kinds turned off; at most 3 figures are named, the most recent.

use super::{block, s, Blocks, CHANGE, HOW, NOW};
use crate::format::{bars as n_bars, num, percent, price};
use crate::{Lean, Sense};
use argos_core::Bar;
use argos_script::indicators::patterns::{self, Pattern};
use argos_script::inputs::Inputs;

/// At most this many figures are named.
pub(super) const MAX_FIGURES: usize = 3;
/// Bars the script needs before it looks for figures.
const NEEDED: usize = 40;

pub(super) const ROLES: [(&str, &str); 2] = [
    (
        "k",
        "Sensibilité du ZigZag : plus grand, seuls les grands mouvements font des pivots, donc moins de figures.",
    ),
    ("keep", "Nombre de figures affichées, les plus récentes."),
];

/// The label of a figure as the chart draws it: « ▲ Double creux ✓ ».
fn label(o: &Pattern) -> String {
    format!(
        "{} {}{}",
        if o.bias > 0 {
            "▲"
        } else if o.bias < 0 {
            "▼"
        } else {
            "◆"
        },
        o.short,
        if o.conf { " ✓" } else { "" }
    )
}

/// One figure: its label, its name, its break and its measured target.
fn figure(o: &Pattern, n: usize, c: f64) -> String {
    let orient = match o.bias {
        1 => "figure haussière",
        -1 => "figure baissière",
        _ => "figure sans orientation",
    };
    let state = if o.conf {
        let when = if o.end + 1 == n {
            "sur la dernière bougie".to_string()
        } else {
            format!("il y a {}", n_bars(n - 1 - o.end))
        };
        format!("cassure confirmée par une clôture {when}")
    } else {
        "pas encore de cassure confirmée".to_string()
    };
    let target = o
        .target
        .map(|t| format!(", objectif mesuré à {} ({} de la clôture)", price(t), percent((t - c) / c * 100.0)))
        .unwrap_or_default();
    let first = o.pts.first().map_or(0, |q| q.i);
    let name = if o.name == o.short { String::new() } else { format!("{}, ", o.name) };
    format!(
        "« {} » : {name}{orient}, premier pivot il y a {}, {state}{target}.",
        label(o),
        n_bars(n.saturating_sub(1 + first))
    )
}

pub(super) fn read(b: &[Bar], vals: &[f64]) -> Blocks {
    let p = Inputs {
        params: &patterns::PARAMS,
        vals,
    };
    let mut how = vec![
        s(
            "how_zigzag",
            format!(
                "Les figures sont tracées entre les pivots d'un ZigZag : un sommet ou un creux est posé quand le prix repart de plus de {} ATR 14.",
                num(p.get("k"), 2)
            ),
        ),
        s(
            "how_labels",
            "▲ : figure orientée à la hausse, ▼ : à la baisse, ◆ : sans orientation ; « ✓ » : la cassure est confirmée par une clôture. L'objectif mesuré reporte la hauteur de la figure depuis sa ligne de cou ; c'est une mesure, pas une promesse.".into(),
        ),
    ];
    let n = b.len();
    if n < NEEDED {
        return super::too_few("Argos Patterns auto", NEEDED, how);
    }
    let list = patterns::compute(b, vals);
    let c = b[n - 1].close;
    let figures: Vec<&Pattern> = list.iter().filter(|o| o.fib.is_none()).collect();
    let mut now: Vec<_> = figures
        .iter()
        .take(MAX_FIGURES)
        .enumerate()
        .map(|(k, o)| {
            let t = figure(o, n, c);
            if k == 0 {
                s("figure", format!("Figure la plus récente {t}"))
            } else {
                s("figure", format!("Figure {t}"))
            }
        })
        .collect();
    if figures.is_empty() {
        now.push(s("no_figure", "Aucune figure affichée.".into()));
    } else if figures.len() > MAX_FIGURES {
        now.push(s(
            "more",
            match figures.len() - MAX_FIGURES {
                1 => "1 autre figure, plus ancienne, est affichée.".to_string(),
                k => format!("{k} autres figures, plus anciennes, sont affichées."),
            },
        ));
    }
    if let Some((a, z, d)) = list.iter().find_map(|o| o.fib) {
        how.push(s(
            "how_fib",
            "Canal de Fibonacci : sa ligne de base passe par les deux derniers creux (ou sommets), sa parallèle par le pivot entre eux ; les niveaux 0,236 à 0,786 sont entre les deux.".into(),
        ));
        let slope = (z.p - a.p) / ((z.i as f64 - a.i as f64).max(1.0));
        let base = a.p + slope * ((n - 1) as f64 - a.i as f64);
        let level = (c - base) / d;
        let place = if level < 0.0 {
            "hors du canal, du côté de la ligne de base (niveau 0)".to_string()
        } else if level > 1.0 {
            "hors du canal, au-delà de la parallèle (niveau 1)".to_string()
        } else {
            let lv = patterns::FIB_LEVELS;
            let k = lv.iter().rposition(|&x| x <= level).unwrap_or(0).min(lv.len() - 2);
            let fr = |x: f64| format!("{x}").replace('.', ",");
            format!("entre les niveaux {} et {}", fr(lv[k]), fr(lv[k + 1]))
        };
        now.push(s(
            "fib",
            format!(
                "« {} » ({}) : la clôture est au niveau {}, {place}.",
                label(list.iter().find(|o| o.fib.is_some()).expect("the channel")),
                if z.p > a.p { "ligne de base montante" } else { "ligne de base descendante" },
                num(level, 2)
            ),
        ));
    }
    let change = vec![s(
        "change",
        "Une clôture qui casse la ligne de cou ou un bord de la figure la plus récente, ou un nouveau pivot du ZigZag, changerait la lecture.".into(),
    )];
    let sense = Some(match figures.first() {
        Some(o) => Sense {
            lean: match o.bias {
                1 => Lean::Up,
                -1 => Lean::Down,
                _ => Lean::Neutral,
            },
            note: Some(if o.conf { "figure confirmée" } else { "figure non confirmée" }),
            value: None,
        },
        None => Sense {
            lean: Lean::NoDirection,
            note: None,
            value: None,
        },
    });
    (vec![block(NOW, now), block(HOW, how), block(CHANGE, change)], sense)
}
