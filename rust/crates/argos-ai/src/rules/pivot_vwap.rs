//! The reading of « Argos Pivot VWAP » (one scale) and « Argos Pivot VWAP Multi » (several scales, strong pivots
//! only): the very VWAP legs anchored on pivots and the HH / LH / HL / LL marks the chart draws, from
//! `argos_script::indicators::pivot_vwap::{compute, compute_multi}` with the same settings. The settings become the
//! script's `Config` exactly as argos-render's `pivot_config` does (a translation of settings, no computation).

use super::{block, s, valued, Blocks, CHANGE, HOW, NOW};
use crate::format::{bars as n_bars, percent, price};
use crate::{Lean, Sense};
use argos_core::Bar;
use argos_lens::explain::Sentence;
use argos_script::classic_inputs::{PIVOT_VWAP_MULTI_PARAMS, PIVOT_VWAP_PARAMS};
use argos_script::indicators::pivot_vwap::{self, Config, Mark, PivotVwap};
use argos_script::inputs::Inputs;

pub(super) const ONE_ROLES: [(&str, &str); 1] = [(
    "span",
    "Bougies de chaque côté d'un pivot : plus grand, seuls les grands sommets et creux comptent.",
)];
pub(super) const MULTI_ROLES: [(&str, &str); 5] = [
    ("s1", "Première échelle : bougies de chaque côté d'un pivot."),
    ("s2", "Deuxième échelle."),
    ("s3", "Troisième échelle."),
    ("votes", "Nombre d'échelles qui doivent voir un pivot pour qu'il ancre un VWAP."),
    ("cut", "Arrête un tronçon quand une clôture le casse."),
];

/// « sur la dernière bougie » or « il y a N bougies ».
fn ago(n: usize, i: usize) -> String {
    if i + 1 == n {
        "sur la dernière bougie".to_string()
    } else {
        format!("il y a {}", n_bars(n - 1 - i))
    }
}

fn mark_name(m: Mark) -> &'static str {
    match m {
        Mark::HigherHigh => "« HH » (sommet plus haut que le précédent)",
        Mark::LowerHigh => "« LH » (sommet plus bas que le précédent)",
        Mark::HigherLow => "« HL » (creux plus haut que le précédent)",
        Mark::LowerLow => "« LL » (creux plus bas que le précédent)",
        Mark::First => "",
    }
}

/// The sentences and the sense, the same for both.
fn reading(b: &[Bar], r: &PivotVwap, how: Vec<Sentence>) -> Blocks {
    let n = b.len();
    let c = b[n - 1].close;
    let mut now = vec![];
    // the leg that reaches the last bar is the active VWAP
    let active = r.legs.iter().rev().find(|l| l.points.last().is_some_and(|p| p.0 + 1 == n));
    let anchor = |l: &pivot_vwap::Leg| l.points.first().map_or(0, |p| p.0);
    if let Some(l) = active {
        let v = l.points.last().map_or(c, |p| p.1);
        now.push(s(
            "active",
            format!(
                "VWAP actif à {}, ancré sur le {} {} : la clôture ({}) est {} ({}).",
                price(v),
                if l.rising { "creux" } else { "sommet" },
                ago(n, anchor(l)),
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
        ));
    } else if let Some(l) = r.legs.last() {
        let (end, v) = l.points.last().copied().unwrap_or((0, c));
        now.push(s(
            "cut",
            format!(
                "Aucun VWAP actif : le dernier tronçon, ancré sur le {} {}, a été cassé {} (à {}).",
                if l.rising { "creux" } else { "sommet" },
                ago(n, anchor(l)),
                ago(n, end),
                price(v)
            ),
        ));
    }
    let last = |low: bool| r.pivots.iter().rev().find(|p| p.is_low == low && p.mark != Mark::First);
    let (hi, lo) = (last(false), last(true));
    for p in [hi, lo].into_iter().flatten() {
        now.push(s(
            if p.is_low { "low" } else { "high" },
            format!(
                "Dernier {} : {} à {}, {}.",
                if p.is_low { "creux" } else { "sommet" },
                mark_name(p.mark),
                price(p.price),
                ago(n, p.at)
            ),
        ));
    }
    if let (Some(h), Some(l)) = (hi, lo) {
        let t = match (h.mark, l.mark) {
            (Mark::HigherHigh, Mark::HigherLow) => Some("sommets et creux montent : structure haussière"),
            (Mark::LowerHigh, Mark::LowerLow) => Some("sommets et creux descendent : structure baissière"),
            _ => None,
        };
        if let Some(t) = t {
            now.push(s("structure", format!("Les derniers {t}.")));
        }
    }
    if now.is_empty() {
        now.push(s("no_pivot", "Aucun pivot sur ces bougies : pas de VWAP ancré à lire.".into()));
    }
    let change = vec![s(
        "change",
        match active {
            Some(l) => format!(
                "Une clôture de l'autre côté du VWAP actif ({}), ou un nouveau pivot, changerait la lecture.",
                price(l.points.last().map_or(c, |p| p.1))
            ),
            None => "Un nouveau pivot, qui ancrerait un nouveau VWAP, changerait la lecture.".into(),
        },
    )];
    let sense = match active.and_then(|l| l.points.last()) {
        Some(&(_, v)) => valued(
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
            format!("{} du VWAP actif", percent((c - v) / v * 100.0)),
        ),
        None => Some(Sense {
            lean: Lean::NoDirection,
            note: Some("aucun VWAP actif"),
            value: None,
        }),
    };
    (vec![block(NOW, now), block(HOW, how), block(CHANGE, change)], sense)
}

/// The common way to read both.
fn how_vwap() -> Sentence {
    s(
        "how_vwap",
        "Depuis chaque pivot, un VWAP (prix moyen pondéré par le volume) est ancré jusqu'au pivot suivant : vert depuis un creux, rouge depuis un sommet. Le prix au-dessus du VWAP actif : les acheteurs depuis ce pivot sont en gain en moyenne ; en dessous, en perte.".into(),
    )
}

pub(super) fn read_one(b: &[Bar], vals: &[f64]) -> Blocks {
    let p = Inputs {
        params: &PIVOT_VWAP_PARAMS,
        vals,
    };
    let span = p.len("span");
    let how = vec![
        s(
            "how_pivot",
            format!(
                "Un pivot est un sommet (ou un creux) plus haut (ou plus bas) que les {span} bougies de chaque côté : il n'est connu que {} plus tard.",
                n_bars(span)
            ),
        ),
        how_vwap(),
    ];
    let cfg = Config { span, ..Config::default() };
    let r = pivot_vwap::compute(b, &cfg);
    if b.len() < 2 * span + 2 {
        return super::too_few("Argos Pivot VWAP", 2 * span + 2, how);
    }
    reading(b, &r, how)
}

pub(super) fn read_multi(b: &[Bar], vals: &[f64]) -> Blocks {
    let p = Inputs {
        params: &PIVOT_VWAP_MULTI_PARAMS,
        vals,
    };
    let spans = vec![p.len("s1"), p.len("s2"), p.len("s3")];
    let votes = p.len("votes").min(3);
    let cut = p.on("cut");
    let mut how = vec![
        s(
            "how_pivot",
            format!(
                "Seuls les pivots forts ancrent un VWAP : vus sur au moins {votes} des 3 échelles ({}, {} et {} bougies de chaque côté).",
                spans[0], spans[1], spans[2]
            ),
        ),
        how_vwap(),
    ];
    if cut {
        how.push(s(
            "how_cut",
            "Un tronçon s'arrête dès qu'une clôture repasse son VWAP à contre-sens (« cassé »).".into(),
        ));
    }
    let needed = 2 * spans.iter().copied().max().unwrap_or(1) + 2;
    let cfg = Config {
        spans,
        min_votes: votes,
        cut_on_break: cut,
        ..Config::default()
    };
    let r = pivot_vwap::compute_multi(b, &cfg);
    if b.len() < needed {
        return super::too_few("Argos Pivot VWAP Multi", needed, how);
    }
    reading(b, &r, how)
}
