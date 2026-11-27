//! The reading of Argos AI Trend V Pro (its band): the very values and marks the chart draws, from
//! `argos_script::indicators::ai_trend::compute` with the same settings (no second computation), and only what the
//! band shows with these settings (argos-render's osc.rs `draw_ai`). The multi-timeframe dots need the bars of the
//! smaller units, which `read` does not have: they are computed without them and never mentioned.

use super::{block, s, valued, Blocks, CHANGE, HOW, NOW};
use crate::format::{bars as n_bars, num};
use crate::{Lean, ScreenData, Sense};
use argos_core::Bar;
use argos_script::indicators::ai_trend::{self, AiTrend, Div, HtfZone, LowerBars};
use argos_script::inputs::Inputs;

/// Marks older than this, in bars, are not « recent ».
pub(super) const RECENT: usize = 50;
/// A divergence confirmed more than this many bars ago is not « en cours » (as for the Cipher).
pub(super) const DIV_RECENT: usize = 20;

pub(super) const ROLES: [(&str, &str); 15] = [
    ("ob", "Niveau de surachat de l'AI Trend Index."),
    ("os", "Niveau de survente de l'AI Trend Index."),
    ("xh", "Niveau extrême haut."),
    ("xl", "Niveau extrême bas."),
    ("mid", "Ligne médiane."),
    ("sig_show", "Affiche la ligne signal (la moyenne de l'AI)."),
    ("bg_show", "Affiche le fond des zones extrêmes (Very Red / Very Blue)."),
    ("vr", "Niveau Very Red : au-dessus, fond rouge de zone extrême."),
    ("vb", "Niveau Very Blue : en dessous, fond bleu de zone extrême."),
    ("dots_show", "Affiche les points vert et rouge des croisements de l'AI et de sa ligne signal."),
    ("face_show", "Affiche 🤑 sur les divergences régulières."),
    ("div_show", "Affiche les lignes des divergences régulières."),
    ("div_hid", "Affiche les divergences cachées."),
    ("htf_box", "Affiche la boîte HTF (l'AI d'une unité de temps plus grande)."),
    ("htf_val", "Affiche la valeur de l'AI dans la boîte HTF."),
];

/// The length of a bar, from the bars themselves (the smallest gap between two bars).
pub(super) fn bar_secs(bars: &[Bar]) -> Option<i64> {
    bars.windows(2).map(|w| w[1].time - w[0].time).filter(|d| *d > 0).min()
}

/// The AI Trend of the chart for `bars`, as the chart computes it, with the bars of the smaller units when the screen
/// has them (otherwise without: no multi-timeframe dots).
pub(super) fn compute(bars: &[Bar], vals: &[f64], data: &ScreenData) -> Option<AiTrend> {
    let secs = data.bar_secs.or_else(|| bar_secs(bars))?;
    let none = LowerBars::new();
    Some(ai_trend::compute(
        bars,
        secs,
        &ai_trend::Settings::from_inputs(vals),
        data.lower.unwrap_or(&none),
    ))
}

/// The divergence confirmed last among `lists` (bull, kind name), if it is recent.
fn last_div(n: usize, lists: &[(&[Div], bool, &'static str)]) -> Option<(bool, &'static str, Div)> {
    lists
        .iter()
        .filter_map(|(l, bull, name)| l.last().map(|d| (*bull, *name, *d)))
        .filter(|(_, _, d)| d.confirmed < n && n - 1 - d.confirmed <= DIV_RECENT)
        .max_by_key(|(_, _, d)| d.confirmed)
}

/// « sur la dernière bougie » or « il y a N bougies ».
fn ago(n: usize, i: usize) -> String {
    if i + 1 == n {
        "sur la dernière bougie".to_string()
    } else {
        format!("il y a {}", n_bars(n - 1 - i))
    }
}

pub(super) fn read(b: &[Bar], vals: &[f64], data: &ScreenData) -> Blocks {
    let p = Inputs {
        params: &ai_trend::PARAMS,
        vals,
    };
    let (ob, os, xh, xl, mid) = (p.get("ob"), p.get("os"), p.get("xh"), p.get("xl"), p.get("mid"));
    let mut how = vec![s(
        "how_index",
        format!(
            "L'AI Trend Index mélange RSI, stochastique et momentum, de 0 à 100 : au-dessus de {} surachat, sous {} survente, au-delà de {} et {} zones extrêmes.",
            num(ob, 0),
            num(os, 0),
            num(xh, 0),
            num(xl, 0)
        ),
    )];
    if p.on("sig_show") {
        how.push(s(
            "how_signal",
            "La ligne signal est la moyenne de l'AI : l'AI au-dessus d'elle montre un élan qui monte, en dessous un élan qui baisse.".into(),
        ));
    }
    if p.on("dots_show") {
        how.push(s(
            "how_dots",
            "Point vert ou rouge : l'AI croise sa ligne signal vers le haut (vert) ou vers le bas (rouge), selon le filtre choisi dans les réglages.".into(),
        ));
    }
    if p.on("face_show") || p.on("div_show") || p.on("div_hid") {
        how.push(s(
            "how_divergence",
            "Divergence régulière (🤑) : le prix fait un plus bas plus bas mais l'AI un plus bas plus haut (haussière), ou l'inverse en haut ; cachée : le contraire. Elle est confirmée quelques bougies après son pivot.".into(),
        ));
    }
    if p.on("htf_box") {
        how.push(s(
            "how_htf",
            format!(
                "Boîte HTF : l'AI d'une unité de temps plus grande ; « BUY 🟢 » quand il est sous {}, « SELL 🔴 » au-dessus de {}, sinon « NEUTRAL ⚪ ». Ce sont les noms de la boîte, pas des consignes.",
                num(os, 0),
                num(ob, 0)
            ),
        ));
    }
    let a = compute(b, vals, data);
    let n = b.len();
    let (Some(a), true) = (a, n >= 2) else {
        return super::too_few("l'AI Trend", 2, how);
    };
    let Some(v) = a.ai.last().copied().filter(|x| x.is_finite()) else {
        return super::too_few("l'AI Trend", 30, how);
    };
    let zone = if v >= xh {
        format!("en zone extrême haute (au-dessus de {})", num(xh, 0))
    } else if v >= ob {
        format!("en surachat (au-dessus de {})", num(ob, 0))
    } else if v <= xl {
        format!("en zone extrême basse (sous {})", num(xl, 0))
    } else if v <= os {
        format!("en survente (sous {})", num(os, 0))
    } else if v >= mid {
        format!("au-dessus de la ligne médiane ({})", num(mid, 0))
    } else {
        format!("sous la ligne médiane ({})", num(mid, 0))
    };
    let mut now = vec![s("value", format!("AI Trend Index à {} : {zone}.", num(v, 1)))];
    let sig = a.signal.last().copied().filter(|x| x.is_finite());
    if let (true, Some(sg)) = (p.on("sig_show"), sig) {
        now.push(s(
            "signal",
            format!("L'AI est {} sa ligne signal ({}).", if v > sg { "au-dessus de" } else { "sous" }, num(sg, 1)),
        ));
    }
    if p.on("bg_show") {
        if a.extreme_high.last() == Some(&(n - 1)) {
            now.push(s(
                "extreme",
                format!("Fond rouge de zone extrême : l'AI est au-dessus du niveau Very Red ({}).", num(p.get("vr"), 0)),
            ));
        } else if a.extreme_low.last() == Some(&(n - 1)) {
            now.push(s(
                "extreme",
                format!("Fond bleu de zone extrême : l'AI est sous le niveau Very Blue ({}).", num(p.get("vb"), 0)),
            ));
        }
    }
    if p.on("dots_show") {
        let last = [(&a.buy, "point vert"), (&a.sell, "point rouge")]
            .into_iter()
            .filter_map(|(l, name)| l.last().map(|&i| (name, i)))
            .max_by_key(|x| x.1)
            .filter(|(_, i)| n - 1 - i <= RECENT);
        now.push(match last {
            Some((name, i)) => s(
                "last_point",
                format!("Dernier point : {name} {}, AI à {} sur cette bougie.", ago(n, i), num(a.ai[i], 1)),
            ),
            None => s("no_point", format!("Aucun point vert ou rouge dans les {RECENT} dernières bougies.")),
        });
    }
    let regular = p.on("face_show") || p.on("div_show");
    let mut lists: Vec<(&[Div], bool, &'static str)> = vec![];
    if regular {
        lists.extend([(&a.divs.bull[..], true, "régulière (🤑)"), (&a.divs.bear[..], false, "régulière (🤑)")]);
    }
    if p.on("div_hid") {
        lists.extend([(&a.divs.hidden_bull[..], true, "cachée"), (&a.divs.hidden_bear[..], false, "cachée")]);
    }
    if let Some((bull, kind, d)) = last_div(n, &lists) {
        now.push(s(
            "divergence",
            format!(
                "Divergence {} {kind} : pivot il y a {}, confirmée {}.",
                if bull { "haussière" } else { "baissière" },
                n_bars(n - 1 - d.pivot),
                ago(n, d.confirmed)
            ),
        ));
    }
    if let (true, Some(h)) = (p.on("htf_box"), a.htf.as_ref()) {
        let zone = match h.zone {
            HtfZone::Buy => "« BUY 🟢 » (l'AI de cette unité est en survente)",
            HtfZone::Sell => "« SELL 🔴 » (l'AI de cette unité est en surachat)",
            HtfZone::Neutral => "« NEUTRAL ⚪ »",
        };
        let value = if p.on("htf_val") {
            format!(", AI à {}", num(h.value, 1))
        } else {
            String::new()
        };
        now.push(s(
            "htf",
            format!("Boîte HTF (signal {}) : zone {zone}{value}.", crate::timeframes::timeframe_label(h.seconds)),
        ));
    }
    // the multi-timeframe dots, only when the screen passes the bars of the smaller units
    if let (Some(lower), Some(secs)) = (data.lower, data.bar_secs.or_else(|| bar_secs(b))) {
        let st = ai_trend::Settings::from_inputs(vals);
        let slots: Vec<(usize, i64)> = st
            .mtf
            .iter()
            .enumerate()
            .filter(|(_, m)| m.on && m.seconds < secs)
            .map(|(k, m)| (k + 1, m.seconds))
            .collect();
        if slots.is_empty() {
            now.push(s(
                "mtf_none",
                "Aucun slot multi-UT coché plus petit que l'unité de la chart : pas de points multi-UT.".into(),
            ));
        } else {
            how.push(s(
                "how_mtf",
                "Points multi-UT : une divergence de l'AI trouvée dans une unité plus petite (un slot), posée sous la ligne si elle est haussière, au-dessus si elle est baissière.".into(),
            ));
            for &(k, sec) in &slots {
                if lower.get(&sec).is_none_or(|x| x.is_empty()) {
                    now.push(s(
                        "mtf_missing",
                        format!(
                            "Slot {k} ({}) : bougies de cette unité absentes, pas de points.",
                            crate::timeframes::timeframe_label(sec)
                        ),
                    ));
                }
            }
            match a.mtf.iter().filter(|m| m.index < n && n - 1 - m.index <= RECENT).max_by_key(|m| m.index) {
                Some(m) => now.push(s(
                    "mtf_last",
                    format!(
                        "Dernier point multi-UT : slot {} ({}), divergence {}, {}.",
                        m.slot,
                        slots
                            .iter()
                            .find(|x| x.0 == m.slot)
                            .map_or(String::new(), |x| crate::timeframes::timeframe_label(x.1)),
                        if m.bull { "haussière" } else { "baissière" },
                        ago(n, m.index)
                    ),
                )),
                None => now.push(s("mtf_last", format!("Aucun point multi-UT sur les {} dernières bougies.", RECENT + 1))),
            }
        }
    }
    let above = sig.is_some_and(|sg| v > sg);
    let change = vec![if sig.is_some() && p.on("sig_show") {
        s(
            "change_signal",
            format!(
                "Un passage de l'AI {} sa ligne signal changerait la lecture de l'élan.",
                if above { "sous" } else { "au-dessus de" }
            ),
        )
    } else {
        s(
            "change_mid",
            format!(
                "Un passage de l'AI {} la ligne médiane ({}) changerait la lecture.",
                if v >= mid { "sous" } else { "au-dessus de" },
                num(mid, 0)
            ),
        )
    }];
    let lean = match sig {
        Some(sg) if v > sg => Lean::Up,
        Some(sg) if v < sg => Lean::Down,
        Some(_) => Lean::Neutral,
        None => super::lean_of(v - mid),
    };
    let note = if v >= xh || v <= xl {
        Some("en zone extrême")
    } else if v >= ob {
        Some("en surachat")
    } else if v <= os {
        Some("en survente")
    } else {
        None
    };
    let sense = valued(Some(Sense { lean, note, value: None }), format!("AI {}", num(v, 1)));
    (vec![block(NOW, now), block(HOW, how), block(CHANGE, change)], sense)
}
