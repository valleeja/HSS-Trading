//! The reading of « Argos Anchored VWAP »: the VWAP from the anchor and the bands the chart draws, from
//! `argos_script::indicators::anchored_vwap::compute` with the same settings (no second computation). A source taken
//! from another indicator's output needs that output from the screen: it is not read here (it waits for lot C).

use super::{block, s, valued, Blocks, CHANGE, HOW, NOW};
use crate::format::{bars as n_bars, num, percent, price};
use crate::{Lean, ScreenData, Sense};
use argos_core::Bar;
use argos_script::indicators::anchored_vwap::{self, PARAMS};
use argos_script::inputs::{Inputs, Src, SOURCES};

pub(super) const ROLES: [(&str, &str); 8] = [
    ("anchor", "Bougie où le VWAP démarre (sans choix : 200 bougies en arrière)."),
    ("src", "Prix moyenné (par défaut (H + L + C)/3)."),
    ("b1", "Affiche les bandes #1."),
    ("m1", "Écart des bandes #1, en écarts-types."),
    ("b2", "Affiche les bandes #2."),
    ("m2", "Écart des bandes #2, en écarts-types."),
    ("b3", "Affiche les bandes #3."),
    ("m3", "Écart des bandes #3, en écarts-types."),
];

/// « écart-type » or « écarts-types » (2 and more).
fn sd_word(x: f64) -> &'static str {
    if x.abs() >= 2.0 {
        "écarts-types"
    } else {
        "écart-type"
    }
}

/// The source the chart uses when none is chosen, (H + L + C)/3.
const HLC3: usize = 5;

pub(super) fn read(b: &[Bar], vals: &[f64], data: &ScreenData) -> Blocks {
    let p = Inputs { params: &PARAMS, vals };
    let mut how = vec![
        s(
            "how_vwap",
            "Le VWAP ancré est le prix moyen pondéré par le volume depuis la bougie d'ancrage : au-dessus, ceux qui ont acheté depuis l'ancrage sont en gain en moyenne ; en dessous, en perte.".into(),
        ),
        s(
            "how_bands",
            "Les bandes s'écartent du VWAP d'un nombre d'écarts-types (pondérés par le volume) : plus le prix s'en éloigne, plus il est loin de son prix moyen depuis l'ancrage.".into(),
        ),
    ];
    // (the source: a price, or another indicator's output the screen passes, used only when it has one value per bar)
    let n = b.len();
    let (k, ext) = match (p.source("src"), data.ext_source) {
        (Src::Price(k), _) => (k, None),
        (Src::Ext(..), Some(x)) => (HLC3, Some(x.len() == n)),
        (Src::Ext(..), None) => {
            return (
                vec![block(
                    NOW,
                    vec![s(
                        "ext",
                        "La source de ce VWAP est la sortie d'un autre indicateur : Argos AI ne la lit pas encore (elle vient de l'écran).".into(),
                    )],
                )],
                None,
            )
        }
    };
    if n < 2 {
        return super::too_few("Argos Anchored VWAP", 2, how);
    }
    let a = anchored_vwap::compute(b, vals, &|_, _| data.ext_source.map(<[f64]>::to_vec));
    let (Some(&v), Some(&dev)) = (a.vwap.last(), a.dev.last()) else {
        return super::too_few("Argos Anchored VWAP", 2, how);
    };
    if !v.is_finite() {
        return super::too_few("Argos Anchored VWAP", 2, how);
    }
    let c = b[n - 1].close;
    let default_anchor = p.get("anchor") <= 0.0;
    match ext {
        Some(true) => how.push(s("how_src", "Valeur moyennée : la sortie d'un autre indicateur, choisie comme source.".into())),
        Some(false) => how.push(s(
            "how_src",
            "La source choisie (la sortie d'un autre indicateur) n'a pas pu être alignée sur les bougies : le calcul est fait sur (H + L + C)/3, comme le script.".into(),
        )),
        None if k != HLC3 => how.push(s("how_src", format!("Prix moyenné : {}.", SOURCES.get(k).copied().unwrap_or("Clôture")))),
        None => {}
    }
    let mut now = vec![s(
        "vwap",
        format!(
            "VWAP ancré {}{} à {} : la clôture ({}) est {} ({}).",
            if a.from + 1 == n {
                "sur la dernière bougie".to_string()
            } else {
                format!("il y a {}", n_bars(n - 1 - a.from))
            },
            if default_anchor {
                " (ancrage par défaut, l'ancre n'est pas encore choisie)"
            } else {
                ""
            },
            price(v),
            price(c),
            if c > v {
                "au-dessus"
            } else if c < v {
                "en dessous"
            } else {
                "au niveau"
            },
            percent((c - v) / v * 100.0)
        ),
    )];
    let bands: Vec<(usize, f64)> = [("b1", "m1"), ("b2", "m2"), ("b3", "m3")]
        .iter()
        .enumerate()
        .filter(|(_, (on, _))| p.on(on))
        .map(|(i, (_, m))| (i + 1, p.get(m)))
        .collect();
    if dev > 0.0 && !bands.is_empty() {
        let z = (c - v) / dev;
        let parts: Vec<String> = bands
            .iter()
            .map(|&(i, m)| {
                let (lo, hi) = (v - m * dev, v + m * dev);
                let place = if c > hi {
                    "clôture au-dessus"
                } else if c < lo {
                    "clôture en dessous"
                } else {
                    "clôture dedans"
                };
                format!("bandes #{i} (± {} {}) de {} à {}, {place}", num(m, 1), sd_word(m), price(lo), price(hi))
            })
            .collect();
        now.push(s(
            "bands",
            format!(
                "La clôture est à {}{} {} du VWAP ; {}.",
                if z > 0.0 { "+" } else { "" },
                num(z, 1),
                sd_word(z),
                parts.join(" ; ")
            ),
        ));
    }
    let change = vec![s(
        "change",
        format!("Une clôture de l'autre côté du VWAP ({}) changerait la lecture.", price(v)),
    )];
    let sense = valued(
        Some(Sense {
            lean: if c > v {
                Lean::Up
            } else if c < v {
                Lean::Down
            } else {
                Lean::Neutral
            },
            note: None,
            value: None,
        }),
        format!("{} du VWAP ancré", percent((c - v) / v * 100.0)),
    );
    (vec![block(NOW, now), block(HOW, how), block(CHANGE, change)], sense)
}
