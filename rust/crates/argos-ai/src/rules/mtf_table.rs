//! The reading of « Argos Tableau multi-UT »: the very rows the chart's table shows, from
//! `argos_script::indicators::mtf_table::compute` on the chart's bars, with the same settings (no second computation).
//! The « time left » needs the time of the screen's computation: without it, it is not cited.

use super::ai_trend::bar_secs;
use super::{block, s, Blocks, CHANGE, HOW, NOW};
use crate::format::{num, percent};
use crate::{Lean, ScreenData, Sense};
use argos_core::Bar;
use argos_script::indicators::mtf_table::{self, PARAMS};
use argos_script::inputs::Inputs;

pub(super) const ROLES: [(&str, &str); 8] = [
    ("fast", "Longueur de l'EMA rapide de la tendance."),
    ("slow", "Longueur de l'EMA lente de la tendance."),
    ("rsi", "Longueur du RSI."),
    ("m15", "Affiche la ligne 15m."),
    ("h1", "Affiche la ligne 1H."),
    ("h4", "Affiche la ligne 4H."),
    ("d1", "Affiche la ligne D."),
    ("w1", "Affiche la ligne W."),
];

/// « 1 h 05 min », « 3 j 4 h ».
fn duration(secs: i64) -> String {
    let (d, h, m) = (secs / 86_400, secs % 86_400 / 3600, secs % 3600 / 60);
    match (d, h) {
        (0, 0) => format!("{m} min"),
        (0, _) => format!("{h} h {m:02} min"),
        _ => format!("{d} j {h} h"),
    }
}

pub(super) fn read(b: &[Bar], vals: &[f64], data: &ScreenData) -> Blocks {
    let p = Inputs { params: &PARAMS, vals };
    let (fast, slow, rl) = (p.get("fast").round(), p.get("slow").round(), p.get("rsi").round());
    let how = vec![
        s(
            "how_rows",
            format!(
                "Chaque ligne regroupe les bougies de la chart dans une unité de temps plus grande : tendance haussière si l'EMA {} est au-dessus de l'EMA {} et le prix au-dessus de l'EMA {} (baissière à l'inverse, neutre sinon), RSI {}, et la variation de la bougie en cours de cette unité.",
                num(fast, 0),
                num(slow, 0),
                num(fast, 0),
                num(rl, 0)
            ),
        ),
        s(
            "how_smaller",
            "Une unité plus petite que celle de la chart ne peut pas être calculée : « — ». Des unités alignées dans le même sens montrent une tendance plus solide, sans le garantir.".into(),
        ),
    ];
    let Some(secs) = data.bar_secs.or_else(|| bar_secs(b)) else {
        return super::too_few("le tableau multi-UT", 2, how);
    };
    let now = data.now.unwrap_or_else(|| b.last().map_or(0, |x| x.time + secs));
    let rows = mtf_table::compute(b, secs, now, vals);
    if rows.is_empty() {
        return (
            vec![
                block(NOW, vec![s("none", "Aucune unité de temps cochée dans le tableau.".into())]),
                block(HOW, how),
            ],
            Some(Sense {
                lean: Lean::NoDirection,
                note: Some("aucune unité cochée"),
                value: None,
            }),
        );
    }
    let mut now_s: Vec<_> = rows
        .iter()
        .map(|r| match r.cell {
            Some(c) => s(
                "row",
                format!(
                    "{} : tendance {}, RSI {}, bougie en cours {}{}.",
                    r.name,
                    match c.trend {
                        1 => "haussière",
                        -1 => "baissière",
                        _ => "neutre",
                    },
                    num(c.rsi, 1),
                    percent(c.chg),
                    if data.now.is_some() {
                        format!(", clôture dans {}", duration(c.left))
                    } else {
                        String::new()
                    }
                ),
            ),
            None => s("row", format!("{} : « — » (unité plus petite que la chart, ou pas assez de bougies).", r.name)),
        })
        .collect();
    let (up, down) = mtf_table::score(&rows);
    let filled = rows.iter().filter(|r| r.cell.is_some()).count();
    now_s.push(s(
        "score",
        format!(
            "{up} {} haussière{} et {down} baissière{} sur {filled} {}.",
            if up > 1 { "unités" } else { "unité" },
            if up > 1 { "s" } else { "" },
            if down > 1 { "s" } else { "" },
            if filled > 1 { "calculées" } else { "calculée" }
        ),
    ));
    let change = vec![s("change", "Une unité qui change de tendance changerait la lecture.".into())];
    let lean = if filled == 0 {
        Lean::NoDirection
    } else if up == filled {
        Lean::Up
    } else if down == filled {
        Lean::Down
    } else {
        Lean::Neutral
    };
    let sense = Some(Sense {
        lean,
        note: (up == filled && filled > 1)
            .then_some("toutes les unités alignées")
            .or((down == filled && filled > 1).then_some("toutes les unités alignées")),
        value: Some(format!("{up} / {down}")),
    });
    (vec![block(NOW, now_s), block(HOW, how), block(CHANGE, change)], sense)
}
