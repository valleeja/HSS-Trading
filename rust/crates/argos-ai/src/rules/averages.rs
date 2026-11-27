//! The readings of TradingView's built-in moving averages, their exact code run by Argos Script (SMA, WMA, HMA,
//! VWMA, Smoothed MA, MA ribbon): the very lines the chart draws, from `argos_script::lang::run` on the built-in
//! script with the settings chosen on the screen (keys: the titles of the script's inputs, « Length@4 » for the SMA's
//! second « Length »). No second computation; a plot the script hides (`display.none`) is never named.

use super::ai_trend::bar_secs;
use super::{block, s, valued, Blocks, CHANGE, HOW, NOW};
use crate::format::{bars as n_bars, num, percent, price};
use crate::{Lean, Sense};
use argos_core::Bar;
use argos_lens::explain::Sentence;
use argos_script::catalogue;
use argos_script::inputs::SOURCES;
use argos_script::lang::{self, InputSpec, Output};
use std::collections::HashMap;

/// Bars over which the slope of an average is measured.
const SLOPE: usize = 5;

pub(super) const SMA_ROLES: [(&str, &str); 6] = [
    ("Length", "Nombre de bougies de la moyenne."),
    ("Source", "Prix moyenné (par défaut la clôture)."),
    ("Offset", "Décale la courbe de n bougies à l'écran."),
    (
        "Type",
        "Lissage de la moyenne : aucun, SMA, SMA + bandes de Bollinger, EMA, SMMA (RMA), WMA ou VWMA.",
    ),
    ("Length@4", "Nombre de bougies du lissage."),
    ("BB StdDev", "Écart des bandes du lissage, en écarts-types."),
];
pub(super) const WMA_ROLES: [(&str, &str); 3] = [
    ("Length", "Nombre de bougies de la moyenne."),
    ("Source", "Prix moyenné (par défaut la clôture)."),
    ("Offset", "Décale la courbe de n bougies à l'écran."),
];
pub(super) const HMA_ROLES: [(&str, &str); 2] = [
    ("Length", "Nombre de bougies de la moyenne."),
    ("Source", "Prix moyenné (par défaut la clôture)."),
];
pub(super) const VWMA_ROLES: [(&str, &str); 3] = WMA_ROLES;
pub(super) const SMMA_ROLES: [(&str, &str); 2] = HMA_ROLES;
pub(super) const RIBBON_ROLES: [(&str, &str); 16] = [
    ("MA #1", "Affiche la moyenne #1."),
    ("MA #1 · 2", "Type de la moyenne #1."),
    ("MA #1 · 3", "Prix moyenné par la moyenne #1."),
    ("MA #1 · 4", "Longueur de la moyenne #1."),
    ("MA #2", "Affiche la moyenne #2."),
    ("MA #2 · 2", "Type de la moyenne #2."),
    ("MA #2 · 3", "Prix moyenné par la moyenne #2."),
    ("MA #2 · 4", "Longueur de la moyenne #2."),
    ("MA #3", "Affiche la moyenne #3."),
    ("MA #3 · 2", "Type de la moyenne #3."),
    ("MA #3 · 3", "Prix moyenné par la moyenne #3."),
    ("MA #3 · 4", "Longueur de la moyenne #3."),
    ("MA #4", "Affiche la moyenne #4."),
    ("MA #4 · 2", "Type de la moyenne #4."),
    ("MA #4 · 3", "Prix moyenné par la moyenne #4."),
    ("MA #4 · 4", "Longueur de la moyenne #4."),
];

/// The ribbon's types, in the order of its list.
const RIBBON_TYPES: [&str; 5] = ["SMA", "EMA", "SMMA (RMA)", "WMA", "VWMA"];
/// The SMA's smoothing types, in the order of its list.
const SMOOTHING: [&str; 7] = ["None", "SMA", "SMA + Bollinger Bands", "EMA", "SMMA (RMA)", "WMA", "VWMA"];

/// A built-in script with the settings received (by key; a missing one keeps its default).
pub(super) struct Script {
    src: &'static str,
    specs: Vec<InputSpec>,
    vals: Vec<f64>,
}

impl Script {
    pub(super) fn new(id: &str, received: &[(&str, f64)]) -> Option<Self> {
        let src = catalogue::source(catalogue::fiche(id)?)?;
        let specs = catalogue::entrees(src);
        let (_, params) = lang::reglages::table(&specs);
        let vals = params
            .iter()
            .map(|p| received.iter().find(|(k, _)| *k == p.key).map_or(p.def, |x| x.1))
            .collect();
        Some(Self { src, specs, vals })
    }

    /// The value of the setting keyed `key`.
    pub(super) fn setting(&self, key: &str) -> f64 {
        let (_, params) = lang::reglages::table(&self.specs);
        params.iter().position(|p| p.key == key).map_or(f64::NAN, |i| self.vals[i])
    }

    pub(super) fn run(&self, b: &[Bar]) -> Option<Output> {
        let chosen: HashMap<String, lang::InVal> = lang::reglages::choisies(&self.specs, &self.vals, &HashMap::new());
        lang::run(self.src, b, bar_secs(b)?, &chosen).ok()
    }
}

/// The values of the visible plot titled `title`.
pub(super) fn plot<'a>(out: &'a Output, title: &str) -> Option<&'a [f64]> {
    out.plots.iter().find(|p| p.title == title && p.visible).map(|p| p.values.as_slice())
}

/// The name of a source index, or `None` for the close.
pub(super) fn source_name(k: f64) -> Option<&'static str> {
    let k = k.max(0.0).round() as usize;
    (k != 0).then(|| SOURCES.get(k).copied().unwrap_or("Clôture"))
}

/// The close against one average line: where it is, since when, and the slope.
fn against(label: &str, b: &[Bar], v: &[f64]) -> Option<(Vec<Sentence>, Lean, f64)> {
    let n = b.len().min(v.len());
    let m = *v.get(n.checked_sub(1)?).filter(|x| x.is_finite())?;
    let c = b[n - 1].close;
    let above = c > m;
    let side = |k: usize| v[k].is_finite() && if above { b[k].close > v[k] } else { b[k].close <= v[k] };
    let streak = (0..n).rev().take_while(|&k| side(k)).count();
    let mut out = vec![s(
        "position",
        format!(
            "{label} à {} : la clôture ({}) est {} ({}), de ce côté depuis {}.",
            price(m),
            price(c),
            if above {
                "au-dessus"
            } else if c < m {
                "en dessous"
            } else {
                "au niveau"
            },
            percent((c - m) / m * 100.0),
            n_bars(streak.max(1))
        ),
    )];
    if let Some(&m0) = n.checked_sub(1 + SLOPE).and_then(|k| v.get(k)).filter(|x| x.is_finite()) {
        out.push(s(
            "slope",
            format!(
                "Pente : elle {} par rapport à il y a {} ({}).",
                if m > m0 {
                    "monte"
                } else if m < m0 {
                    "descend"
                } else {
                    "est plate"
                },
                n_bars(SLOPE),
                percent((m - m0) / m0 * 100.0)
            ),
        ));
    }
    let lean = if above {
        Lean::Up
    } else if c < m {
        Lean::Down
    } else {
        Lean::Neutral
    };
    Some((out, lean, (c - m) / m * 100.0))
}

/// One average alone (SMA, WMA, HMA, VWMA, Smoothed MA).
pub(super) fn read_one(id: &str, b: &[Bar], received: &[(&str, f64)]) -> Blocks {
    let (short, line, what) = match id {
        "Sma" => (
            "SMA",
            "MA",
            "La SMA est la moyenne simple des clôtures (ou de la source) des dernières bougies : chaque bougie compte autant.",
        ),
        "Wma" => ("WMA", "WMA", "La WMA donne plus de poids aux bougies récentes qu'aux anciennes."),
        "Hma" => (
            "HMA",
            "HMA",
            "La HMA (Hull) combine plusieurs WMA pour réagir plus vite qu'une moyenne simple de même longueur.",
        ),
        "Vwma" => (
            "VWMA",
            "VWMA",
            "La VWMA pèse chaque bougie par son volume : les bougies très échangées comptent plus.",
        ),
        _ => (
            "SMMA",
            "SMMA",
            "La SMMA (moyenne lissée) change plus lentement qu'une moyenne simple de même longueur.",
        ),
    };
    let Some(sc) = Script::new(id, received) else {
        return (
            vec![block(NOW, vec![s("missing", "Le script de cette moyenne n'est pas dans ce catalogue.".into())])],
            None,
        );
    };
    let len = sc.setting("Length").max(1.0).round() as usize;
    let name = match source_name(sc.setting("Source")) {
        Some(src) => format!("{short} {len} ({src})"),
        None => format!("{short} {len}"),
    };
    let mut how = vec![
        s("how_kind", what.into()),
        s(
            "how_side",
            "Au-dessus de sa moyenne, le prix est plus haut que son niveau moyen récent ; une moyenne qui monte suit une hausse. Ce n'est pas une promesse pour la suite.".into(),
        ),
    ];
    let out = sc.run(b);
    let Some((mut now, lean, gap)) = out.as_ref().and_then(|o| plot(o, line)).and_then(|v| against(&name, b, v)) else {
        return super::too_few(&name, len + 1, how);
    };
    let out = out.expect("ran");
    let offset = out.plots.iter().find(|p| p.title == line).map_or(0, |p| p.offset);
    if offset != 0 {
        now.push(s(
            "offset",
            format!(
                "La courbe est décalée de {} vers la {} à l'écran (réglage Offset).",
                n_bars(offset.unsigned_abs() as usize),
                if offset > 0 { "droite" } else { "gauche" }
            ),
        ));
    }
    // the SMA's smoothing, only when the script draws it
    if id == "Sma" {
        let kind = SMOOTHING.get(sc.setting("Type").max(0.0) as usize).copied().unwrap_or("None");
        if let Some(sm) = plot(&out, "SMA-based MA").and_then(|v| v.last().copied()).filter(|x| x.is_finite()) {
            let k = sc.setting("Length@4").round();
            how.push(s(
                "how_smoothing",
                format!(
                    "La « SMA-based MA » lisse la SMA par une {} {} : elle est plus lente qu'elle.",
                    if kind.starts_with("SMA") { "SMA" } else { kind },
                    num(k, 0)
                ),
            ));
            now.push(s("smoothing", format!("SMA-based MA ({kind}, {}) à {}.", num(k, 0), price(sm))));
            let band = |t: &str| plot(&out, t).and_then(|v| v.last().copied()).filter(|x| x.is_finite());
            if let (Some(u), Some(l)) = (band("Upper Bollinger Band"), band("Lower Bollinger Band")) {
                let c = b[b.len() - 1].close;
                let place = if c > u {
                    "au-dessus de la bande haute"
                } else if c < l {
                    "sous la bande basse"
                } else {
                    "entre les bandes"
                };
                now.push(s(
                    "bands",
                    format!(
                        "Bandes de Bollinger du lissage ({} écarts-types) de {} à {} : la clôture est {place}.",
                        num(sc.setting("BB StdDev"), 1),
                        price(l),
                        price(u)
                    ),
                ));
            }
        }
    }
    let change = vec![s("change", format!("Une clôture de l'autre côté de la {name} changerait la lecture."))];
    let sense = valued(Some(Sense { lean, note: None, value: None }), format!("{} de la {name}", percent(gap)));
    (vec![block(NOW, now), block(HOW, how), block(CHANGE, change)], sense)
}

/// The MA ribbon: only the averages ticked.
pub(super) fn read_ribbon(b: &[Bar], received: &[(&str, f64)]) -> Blocks {
    let how = vec![
        s(
            "how_ribbon",
            "Le ruban montre jusqu'à 4 moyennes de longueurs différentes. Rangées dans le sens haussier (la plus courte en haut, la plus longue en bas), elles suivent une hausse ; dans le sens inverse, une baisse ; mêlées, le marché hésite.".into(),
        ),
        s("how_cross", "Une moyenne courte qui croise une plus longue marque un changement de rythme, pas une promesse.".into()),
    ];
    let Some(sc) = Script::new("MaRibbon", received) else {
        return (
            vec![block(NOW, vec![s("missing", "Le script du ruban n'est pas dans ce catalogue.".into())])],
            None,
        );
    };
    let n = b.len();
    let Some(out) = sc.run(b) else {
        return super::too_few("le ruban de moyennes", 2, how);
    };
    // (number, name, length, values) of the ticked averages
    let mut shown: Vec<(usize, String, f64, &[f64])> = vec![];
    for k in 1..=4 {
        if sc.setting(&format!("MA #{k}")) == 0.0 {
            continue;
        }
        let Some(v) = plot(&out, &format!("MA #{k}")) else { continue };
        let ty = RIBBON_TYPES
            .get(sc.setting(&format!("MA #{k} · 2")).max(0.0) as usize)
            .copied()
            .unwrap_or("SMA");
        let len = sc.setting(&format!("MA #{k} · 4"));
        let src = source_name(sc.setting(&format!("MA #{k} · 3"))).map_or(String::new(), |x| format!(" ({x})"));
        shown.push((k, format!("MA #{k} ({ty} {}{src})", num(len, 0)), len, v));
    }
    if shown.is_empty() {
        return (
            vec![block(NOW, vec![s("none", "Aucune moyenne cochée dans le ruban.".into())]), block(HOW, how)],
            Some(Sense {
                lean: Lean::NoDirection,
                note: Some("aucune moyenne affichée"),
                value: None,
            }),
        );
    }
    let last = |v: &[f64]| v.get(n.wrapping_sub(1)).copied().filter(|x| x.is_finite());
    if n < 2 || shown.iter().any(|x| last(x.3).is_none()) {
        let longest = shown.iter().map(|x| x.2).fold(1.0, f64::max) as usize;
        return super::too_few("le ruban de moyennes", longest + 1, how);
    }
    let c = b[n - 1].close;
    let mut now: Vec<Sentence> = shown
        .iter()
        .map(|(_, name, _, v)| {
            let m = last(v).unwrap_or(f64::NAN);
            s(
                "average",
                format!("{name} à {} (la clôture est à {} d'elle).", price(m), percent((c - m) / m * 100.0)),
            )
        })
        .collect();
    // order: shortest first
    let mut by_len: Vec<&(usize, String, f64, &[f64])> = shown.iter().collect();
    by_len.sort_by(|x, y| x.2.total_cmp(&y.2));
    let vals: Vec<f64> = by_len.iter().map(|x| last(x.3).unwrap_or(f64::NAN)).collect();
    let order = if vals.len() < 2 {
        None
    } else if vals.windows(2).all(|w| w[0] > w[1]) {
        Some(true)
    } else if vals.windows(2).all(|w| w[0] < w[1]) {
        Some(false)
    } else {
        None
    };
    if vals.len() >= 2 {
        now.push(s(
            "order",
            match order {
                Some(true) => "Rangement haussier : la plus courte en haut, la plus longue en bas.".to_string(),
                Some(false) => "Rangement baissier : la plus courte en bas, la plus longue en haut.".to_string(),
                None => "Moyennes mêlées : ni rangement haussier ni rangement baissier.".to_string(),
            },
        ));
    }
    let (hi, lo) = (vals.iter().cloned().fold(f64::MIN, f64::max), vals.iter().cloned().fold(f64::MAX, f64::min));
    let place = if c > hi {
        "au-dessus de toutes les moyennes affichées"
    } else if c < lo {
        "en dessous de toutes les moyennes affichées"
    } else {
        "dans le ruban"
    };
    now.push(s("place", format!("La clôture ({}) est {place}.", price(c))));
    // the last cross between two shown averages
    let mut cross: Option<(usize, usize, usize)> = None;
    for (a, x) in shown.iter().enumerate() {
        for y in &shown[a + 1..] {
            let d = |k: usize| x.3[k] - y.3[k];
            let mut side: Option<bool> = None;
            for k in 0..n {
                let v = d(k);
                if !v.is_finite() || v == 0.0 {
                    continue;
                }
                if side.is_some_and(|s0| s0 != (v > 0.0)) && cross.is_none_or(|c0| k > c0.0) {
                    cross = Some((k, x.0, y.0));
                }
                side = Some(v > 0.0);
            }
        }
    }
    if let Some((k, p, q)) = cross {
        now.push(s(
            "cross",
            format!(
                "Dernier croisement : MA #{p} et MA #{q}, {}.",
                if k + 1 == n {
                    "sur la dernière bougie".to_string()
                } else {
                    format!("il y a {}", n_bars(n - 1 - k))
                }
            ),
        ));
    }
    let lean = match (order, place) {
        (Some(true), "au-dessus de toutes les moyennes affichées") => Lean::Up,
        (Some(false), "en dessous de toutes les moyennes affichées") => Lean::Down,
        _ => Lean::Neutral,
    };
    let change = vec![s(
        "change",
        "Une clôture qui traverse le ruban, ou deux moyennes qui se croisent, changerait la lecture.".into(),
    )];
    let sense = Some(Sense {
        lean,
        note: Some(place),
        value: None,
    });
    (vec![block(NOW, now), block(HOW, how), block(CHANGE, change)], sense)
}
