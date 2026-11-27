//! The reading of « Spring Tension » (Jee's « Spring Tension Ribbon + Volume Exhaustion », run as it is by Argos
//! Script): the very marks, ribbon and warning the chart draws, from `argos_script::lang::run_avec_variables` on the
//! built-in source with the same settings (keys = the input titles), as the screen runs it. No second computation:
//! the marks come from the shapes and alert conditions of the run, the RSI and the money flow from the script's own
//! variables `rsiVal` and `mfiVal`. The money flow is the script's own function, named « flux d'argent du script
//! (MFI) », not the standard MFI.

use super::ai_trend::bar_secs;
use super::{block, s, valued, Blocks, CHANGE, HOW, NOW};
use crate::format::{bars as n_bars, num};
use crate::{Lean, Sense};
use argos_core::Bar;
use argos_script::catalogue;
use argos_script::lang::{self, InputSpec, Output};
use std::collections::HashMap;

/// Marks older than this, in bars, are not « recent ».
pub(super) const RECENT: usize = 50;

pub(super) const ROLES: [(&str, &str); 12] = [
    ("RSI Length", "Longueur du RSI."),
    ("Lookback prix", "Fenêtre du plus bas / plus haut récent pour le Spring et l'Upthrust."),
    ("RSI max Spring", "Le RSI doit être sous ce niveau pour un Spring."),
    ("RSI min Upthrust", "Le RSI doit être au-dessus de ce niveau pour un Upthrust."),
    (
        "Si pas de volume",
        "Sans volume : le flux d'argent devient un RSI sur hlc3, ou il est désactivé.",
    ),
    ("Volume MA Length", "Longueur de la moyenne du volume."),
    ("Volume % sous la moyenne", "Volume « sec » : sous la moyenne de ce pourcentage."),
    (
        "Variation RSI depuis le pic/creux",
        "De combien le RSI doit être revenu depuis son pic ou son creux des 10 dernières bougies.",
    ),
    ("Fenêtre du cluster", "Bougies où l'on compte les points pour un NUKE."),
    ("Signaux min. pour NUKE", "Nombre de points dans la fenêtre pour un NUKE."),
    ("Short", "Affiche les points « Short » et « SHORT NUKE » (essoufflement des acheteurs)."),
    ("Long", "Affiche les points « Long » et « LONG NUKE » (essoufflement des vendeurs)."),
];

/// The source and the inputs of the built-in script.
fn script() -> Option<(&'static str, Vec<InputSpec>)> {
    let src = catalogue::source(catalogue::fiche("SpringTension")?)?;
    Some((src, catalogue::entrees(src)))
}

/// The settings received, as the script's chosen values (a missing one keeps its default).
struct Settings {
    specs: Vec<InputSpec>,
    vals: Vec<f64>,
}

impl Settings {
    fn new(specs: Vec<InputSpec>, received: &[(&str, f64)]) -> Self {
        let (_, params) = lang::reglages::table(&specs);
        let vals = params
            .iter()
            .map(|p| received.iter().find(|(k, _)| *k == p.key).map_or(p.def, |x| x.1))
            .collect();
        Self { specs, vals }
    }

    /// The value of the setting titled `title`.
    fn setting(&self, title: &str) -> f64 {
        let (_, params) = lang::reglages::table(&self.specs);
        params.iter().position(|p| p.key == title).map_or(f64::NAN, |i| self.vals[i])
    }

    fn chosen(&self) -> HashMap<String, lang::InVal> {
        lang::reglages::choisies(&self.specs, &self.vals, &HashMap::new())
    }
}

/// The bars of the shapes titled `title`.
fn marks(out: &Output, title: &str) -> Vec<usize> {
    out.shapes.iter().filter(|x| x.title == title).map(|x| x.bar).collect()
}

/// The bars where the alert condition titled `title` is true.
fn alert(out: &Output, title: &str) -> Vec<usize> {
    out.alertconditions
        .iter()
        .find(|a| a.0 == title)
        .map(|a| a.2.iter().enumerate().filter(|(_, v)| **v == 1.0).map(|(i, _)| i).collect())
        .unwrap_or_default()
}

/// « sur la dernière bougie » or « il y a N bougies ».
fn ago(n: usize, i: usize) -> String {
    if i + 1 == n {
        "sur la dernière bougie".to_string()
    } else {
        format!("il y a {}", n_bars(n - 1 - i))
    }
}

/// The run of the script as the screen runs it, with its RSI and money flow, for the tests.
#[cfg(test)]
pub(crate) fn run(b: &[Bar], received: &[(&str, f64)]) -> Option<(Output, HashMap<String, Vec<f64>>)> {
    let (src, specs) = script()?;
    let st = Settings::new(specs, received);
    lang::run_avec_variables(src, b, bar_secs(b)?, &st.chosen(), &["rsiVal", "mfiVal"]).ok()
}

pub(super) fn read(b: &[Bar], received: &[(&str, f64)]) -> Blocks {
    let Some((src, specs)) = script() else {
        return (
            vec![block(NOW, vec![s("missing", "Le script Spring Tension n'est pas dans ce catalogue.".into())])],
            None,
        );
    };
    let st = Settings::new(specs, received);
    let (show_short, show_long) = (st.setting("Short") != 0.0, st.setting("Long") != 0.0);
    let (window, nuke_min) = (st.setting("Fenêtre du cluster"), st.setting("Signaux min. pour NUKE"));
    let mut how = vec![
        s(
            "how_spring",
            format!(
                "Spring : le plus bas touche le plus bas des {} dernières bougies, le flux d'argent monte et le RSI est sous {}. Upthrust : le plus haut touche le plus haut des {} dernières bougies, le flux d'argent baisse et le RSI est au-dessus de {}.",
                num(st.setting("Lookback prix"), 0),
                num(st.setting("RSI max Spring"), 0),
                num(st.setting("Lookback prix"), 0),
                num(st.setting("RSI min Upthrust"), 0)
            ),
        ),
        s(
            "how_ribbon",
            "Ruban : or pour un Spring, magenta pour un Upthrust, vert (Haussier) quand le flux d'argent est au-dessus de 50 et que la clôture monte, rouge (Survendu) quand le RSI est sous 30, gris (Neutre) sinon.".into(),
        ),
    ];
    let dry = format!(
        "le volume est sous sa moyenne de {} bougies d'au moins {} %",
        num(st.setting("Volume MA Length"), 0),
        num(st.setting("Volume % sous la moyenne"), 0)
    );
    let back = num(st.setting("Variation RSI depuis le pic/creux"), 1);
    let nuke = format!("au moins {} de ces points dans les {} dernières bougies", num(nuke_min, 0), num(window, 0));
    if show_long {
        how.push(s(
            "how_long",
            format!("Point « Long » : essoufflement des vendeurs ; {dry}, le RSI est sous 40 et il est remonté d'au moins {back} depuis son creux des 10 dernières bougies. « LONG NUKE » (une croix à la place du point) : {nuke}."),
        ));
    }
    if show_short {
        how.push(s(
            "how_short",
            format!("Point « Short » : essoufflement des acheteurs ; {dry}, le RSI est au-dessus de 60 et il est redescendu d'au moins {back} depuis son pic des 10 dernières bougies. « SHORT NUKE » (une croix à la place du point) : {nuke}."),
        ));
    }
    if show_long || show_short {
        how.push(s(
            "how_names",
            "« Long », « Short », « LONG NUKE » et « SHORT NUKE » sont les noms des marques du script, pas des consignes.".into(),
        ));
    }
    let n = b.len();
    let Some(secs) = bar_secs(b) else {
        return super::too_few("Spring Tension", 2, how);
    };
    let Ok((out, vars)) = lang::run_avec_variables(src, b, secs, &st.chosen(), &["rsiVal", "mfiVal"]) else {
        return super::too_few("Spring Tension", 2, how);
    };
    let last = |name: &str| vars.get(name).and_then(|v| v.last().copied()).filter(|x| x.is_finite());
    let Some(r) = last("rsiVal") else {
        return super::too_few("Spring Tension", st.setting("RSI Length") as usize + 1, how);
    };
    let has_volume = b.iter().any(|x| x.volume > 0.0);
    let (springs, upthrusts) = (alert(&out, "🌱 Bullish Spring"), alert(&out, "🥀 Bearish Upthrust"));
    let ribbon = out
        .plots
        .iter()
        .find(|p| p.title == "Ruban")
        .and_then(|p| p.colors.get(n - 1).copied().flatten());
    // the state of the ribbon: Spring and Upthrust from the alerts, the others by the transparency the script gives
    // them (20 Haussier, 40 Survendu, 80 Neutre)
    let state = if springs.last() == Some(&(n - 1)) {
        "or (Spring)"
    } else if upthrusts.last() == Some(&(n - 1)) {
        "magenta (Upthrust)"
    } else {
        match ribbon.map(|c| (c >> 24) & 0x7f) {
            Some(20) => "vert (Haussier)",
            Some(40) => "rouge (Survendu)",
            Some(80) => "gris (Neutre)",
            _ => "sans couleur",
        }
    };
    let flow = match last("mfiVal") {
        Some(m) => format!("flux d'argent du script (MFI) à {}", num(m, 1)),
        None => "flux d'argent du script (MFI) sans valeur".to_string(),
    };
    let mut now = vec![s(
        "ribbon",
        format!(
            "Ruban {state} sur la dernière bougie ; RSI {} à {}, {flow}.",
            num(st.setting("RSI Length"), 0),
            num(r, 1)
        ),
    )];
    if let Some((name, i)) = [("Spring", springs.last()), ("Upthrust", upthrusts.last())]
        .into_iter()
        .filter_map(|(nm, i)| i.map(|&i| (nm, i)))
        .max_by_key(|x| x.1)
        .filter(|x| n - 1 - x.1 <= RECENT)
    {
        now.push(s("last_spring", format!("Dernier {name} : {}.", ago(n, i))));
    }
    if !has_volume {
        let mode = if st.setting("Si pas de volume") == 0.0 {
            "le flux d'argent est remplacé par un RSI sur hlc3"
        } else {
            "le flux d'argent est désactivé"
        };
        now.push(s(
            "no_volume",
            format!("Pas de volume sur cette chart : {mode}, et aucun essoufflement n'est calculé."),
        ));
    } else {
        let w = window.round().max(1.0) as usize;
        for (on, dot, cross, what) in [
            (show_long, "Long", "LONG NUKE", "essoufflement des vendeurs"),
            (show_short, "Short", "SHORT NUKE", "essoufflement des acheteurs"),
        ] {
            if !on {
                continue;
            }
            let (dots, crosses) = (marks(&out, dot), marks(&out, cross));
            if crosses.last() == Some(&(n - 1)) {
                let run = (0..n).rev().take_while(|i| crosses.contains(i)).count();
                now.push(s("nuke", format!("« {cross} » en cours ({what} en grappe), depuis {}.", n_bars(run))));
            }
            let last = dots.iter().chain(&crosses).max().copied().filter(|i| n - 1 - i <= RECENT);
            match last {
                Some(i) => {
                    let in_window = dots.iter().chain(&crosses).filter(|&&k| k + w >= n).count();
                    now.push(s(
                        "last_mark",
                        format!(
                            "Dernière marque « {} » ({what}) {} ; {in_window} {} « {dot} » ou « {cross} » dans les {w} dernières bougies.",
                            if crosses.contains(&i) { cross } else { dot },
                            ago(n, i),
                            if in_window >= 2 { "marques" } else { "marque" }
                        ),
                    ));
                }
                None => now.push(s("no_mark", format!("Aucune marque « {dot} » ({what}) dans les {RECENT} dernières bougies."))),
            }
        }
    }
    let change = vec![s(
        "change",
        "Un retour du volume au-dessus de sa moyenne, ou un RSI qui sort de sa zone (au-dessus de 40 pour les vendeurs, sous 60 pour les acheteurs), ferait disparaître l'essoufflement.".into(),
    )];
    let long_nuke = show_long && marks(&out, "LONG NUKE").last() == Some(&(n - 1));
    let short_nuke = show_short && marks(&out, "SHORT NUKE").last() == Some(&(n - 1));
    let recent = |l: &[usize]| l.last().is_some_and(|&i| n - 1 - i <= 5);
    let note = if long_nuke {
        Some("LONG NUKE en cours")
    } else if short_nuke {
        Some("SHORT NUKE en cours")
    } else if show_long && recent(&marks(&out, "Long")) {
        Some("essoufflement des vendeurs récent")
    } else if show_short && recent(&marks(&out, "Short")) {
        Some("essoufflement des acheteurs récent")
    } else if recent(&springs) {
        Some("Spring récent")
    } else if recent(&upthrusts) {
        Some("Upthrust récent")
    } else {
        None
    };
    let sense = valued(
        Some(Sense {
            lean: Lean::NoDirection,
            note,
            value: None,
        }),
        format!("RSI {}", num(r, 1)),
    );
    (vec![block(NOW, now), block(HOW, how), block(CHANGE, change)], sense)
}
