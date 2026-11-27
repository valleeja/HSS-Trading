//! The readings of « Argos Bubbles · Trades & Liquidité » and of the CVD: the very bubbles and the cumulative delta
//! the chart draws, from `argos_script::indicators::flow::{bubbles, cvd}` with the same settings and, for the CVD,
//! the real orderflow the screen has (no second computation). The bubbles come from the real trades only, as on the
//! chart: a bar without complete real trades has none, and without any the reading says so.

use super::{block, s, valued, Blocks, CHANGE, HOW, NOW};
use crate::format::{bars as n_bars, num, price};
use crate::{Lean, ScreenData, Sense};
use argos_core::Bar;
use argos_script::indicators::flow;
use argos_script::inputs::Inputs;

/// Bubbles counted as « recent », in bars (the chart's table counts the same 50).
const RECENT: usize = 50;
/// Bars over which the CVD's direction is measured.
const LOOKBACK: usize = 10;

pub(super) const BUBBLES_ROLES: [(&str, &str); 6] = [
    ("nb", "Nombre de bougies analysées."),
    ("ratio", "Déséquilibre minimum entre acheteurs et vendeurs d'une rangée pour faire une bulle."),
    ("minv", "Volume minimum d'une rangée (× le volume moyen d'une rangée)."),
    ("mind", "Écart minimum entre acheteurs et vendeurs (× le volume moyen d'une rangée)."),
    ("per", "Rangées de prix par bougie."),
    ("poc", "Affiche le POC (rangée la plus échangée) de chaque bougie."),
];
pub(super) const CVD_ROLES: [(&str, &str); 1] = [("sig", "Longueur de la moyenne du CVD (0 = aucune).")];

/// « sur la dernière bougie » or « il y a N bougies ».
fn ago(n: usize, i: usize) -> String {
    if i + 1 == n {
        "sur la dernière bougie".to_string()
    } else {
        format!("il y a {}", n_bars(n - 1 - i))
    }
}

/// The diagonal imbalances of one footprint bar (buy, sell), by the footprint's own rule
/// (`argos_core::orderflow::desequilibre_diagonal`, the one the drawing uses).
fn imbalances(rows: &[flow::Row], ratio: f64) -> (usize, usize) {
    (0..rows.len())
        .map(|k| argos_core::orderflow::desequilibre_diagonal(rows, k, ratio))
        .fold((0, 0), |(b, s), (x, y)| (b + usize::from(x), s + usize::from(y)))
}

pub(super) fn read_bubbles(b: &[Bar], vals: &[f64], data: &ScreenData) -> Blocks {
    let p = Inputs {
        params: &flow::BUBBLES_PARAMS,
        vals,
    };
    let mut how = vec![s(
        "how_bubbles",
        format!(
            "Chaque bougie aux vrais trades complets est découpée en rangées de prix ; une bulle marque une rangée où un côté domine l'autre d'au moins {} fois, avec assez de volume. Verte : les acheteurs dominent ; rouge : les vendeurs. Une bougie sans vrais trades complets n'a pas de bulle.",
            num(p.get("ratio"), 1)
        ),
    )];
    if p.on("poc") {
        how.push(s(
            "how_poc",
            "Le POC est la rangée la plus échangée de chaque bougie aux vrais trades (point jaune).".into(),
        ));
    }
    let n = b.len();
    if n < 2 {
        return super::too_few("Argos Bubbles", 2, how);
    }
    // (the screen's orderflow also holds the bar in progress: only the closed bars' are kept)
    let last = b[n - 1].time;
    let reel = data.flow.map_or(&[][..], |f| &f[..f.partition_point(|r| r.time <= last)]);
    let x = flow::bubbles(b, vals, reel, &|v| price(v));
    let Some(since) = x.depuis else {
        return (
            vec![
                block(NOW, vec![s("absent", "Bulles : vrais trades absents pour ce marché ou cette unité.".into())]),
                block(HOW, how),
            ],
            None,
        );
    };
    let from = b.partition_point(|q| q.time < since);
    let mut now = vec![s("since", format!("Vrais trades depuis la bougie d'il y a {}.", n_bars(n - 1 - from)))];
    let recent: Vec<&flow::Bubble> = x.items.iter().filter(|q| q.i + RECENT >= n).collect();
    let buy = recent.iter().filter(|q| q.buy).count();
    let sell = recent.len() - buy;
    now.push(s(
        "count",
        match recent.len() {
            0 => format!("Aucune bulle sur les {RECENT} dernières bougies."),
            _ => format!(
                "Sur les {RECENT} dernières bougies : {buy} {} côté acheteurs, {sell} {} côté vendeurs.",
                if buy > 1 { "bulles" } else { "bulle" },
                if sell > 1 { "bulles" } else { "bulle" }
            ),
        },
    ));
    if let Some(q) = x.items.last() {
        now.push(s(
            "last",
            format!(
                "Dernière bulle : côté {} à {}, {}.",
                if q.buy { "acheteurs" } else { "vendeurs" },
                price(q.price),
                ago(n, q.i)
            ),
        ));
    }
    if let Some(&(i, poc)) = x.pocs.last() {
        now.push(s(
            "poc",
            format!("POC de la dernière bougie aux vrais trades ({}) à {}.", ago(n, i), price(poc)),
        ));
    }
    let change = vec![s("change", "De nouvelles bulles de l'autre côté changeraient la lecture.".into())];
    let lean = if buy > sell {
        Lean::Up
    } else if sell > buy {
        Lean::Down
    } else {
        Lean::Neutral
    };
    let sense = valued(
        Some(Sense {
            lean,
            note: Some("vrais trades"),
            value: None,
        }),
        format!("{buy} / {sell} bulles"),
    );
    (vec![block(NOW, now), block(HOW, how), block(CHANGE, change)], sense)
}

pub(super) fn read_cvd(b: &[Bar], vals: &[f64], data: &ScreenData) -> Blocks {
    let p = Inputs {
        params: &argos_script::classic_inputs::OBV_PARAMS,
        vals,
    };
    let sig = p.get("sig").round().max(0.0) as usize;
    let mut how = vec![s(
        "how_cvd",
        "Le CVD additionne, bougie après bougie, le delta : le volume acheté moins le volume vendu. Il monte quand les acheteurs dominent, il descend quand ce sont les vendeurs ; seul son sens compte, pas son niveau.".into(),
    )];
    // (the screen's orderflow also holds the bar in progress: only the closed bars' are kept)
    let n = b.len();
    let last = b.last().map_or(i64::MIN, |x| x.time);
    let reel: &[argos_core::orderflow::FlowBar] = data.flow.map_or(&[], |f| &f[..f.partition_point(|r| r.time <= last)]);
    let real = b
        .iter()
        .filter(|x| reel.binary_search_by_key(&x.time, |r| r.time).ok().is_some_and(|k| reel[k].complete))
        .count();
    how.push(s(
        "how_source",
        match (data.flow, real) {
            (None, _) => "Delta estimé d'après les bougies (la place de la clôture dans la bougie) : l'orderflow réel n'est pas transmis.".to_string(),
            (Some(_), 0) => {
                "Delta estimé d'après les bougies (la place de la clôture dans la bougie), comme la chart : pas d'orderflow réel pour ces bougies.".to_string()
            }
            (Some(_), k) => format!(
                "Vrai delta (trades complets) sur {} des {} chargées, estimé d'après les bougies sur les autres.",
                n_bars(k),
                n
            ),
        },
    ));
    let c = flow::cvd(b, reel);
    let (Some(&v), Some(i0)) = (c.last(), c.len().checked_sub(1 + LOOKBACK)) else {
        return super::too_few("le CVD", LOOKBACK + 1, how);
    };
    let (c_up, p_up) = (v >= c[i0], b[n - 1].close >= b[i0].close);
    let mut now = vec![s(
        "trend",
        format!(
            "Sur {} : CVD en {}, prix en {} ({}).",
            n_bars(LOOKBACK),
            if c_up { "hausse" } else { "baisse" },
            if p_up { "hausse" } else { "baisse" },
            if c_up == p_up {
                "le delta accompagne le mouvement"
            } else {
                "le delta ne suit pas le mouvement"
            }
        ),
    )];
    if sig >= 1 {
        if let Some(m) = argos_script::ta::sma(&c, sig).last().copied().filter(|x| x.is_finite()) {
            now.push(s(
                "average",
                format!("CVD {} de sa moyenne de {sig} bougies.", if v > m { "au-dessus" } else { "en dessous" }),
            ));
        }
    }
    let change = vec![s("change", "Un CVD qui part dans l'autre sens que le prix changerait la lecture.".into())];
    let note = match (data.flow, real) {
        (Some(_), k) if k > 0 => "vrai delta en partie",
        _ => "delta estimé",
    };
    let sense = Some(Sense {
        lean: if c_up { Lean::Up } else { Lean::Down },
        note: Some(note),
        value: None,
    });
    (vec![block(NOW, now), block(HOW, how), block(CHANGE, change)], sense)
}

pub(super) const FOOTPRINT_ROLES: [(&str, &str); 3] = [
    ("per", "Rangées de prix par bougie."),
    ("nb", "Nombre de bougies affichées."),
    ("imb", "Seuil d'un déséquilibre en diagonale, en % (300 = trois fois plus)."),
];

pub(super) fn read_footprint(b: &[Bar], vals: &[f64], data: &ScreenData) -> Blocks {
    let p = Inputs {
        params: &flow::FOOTPRINT_PARAMS,
        vals,
    };
    let imb = p.get("imb");
    let how = vec![
        s(
            "how_rows",
            "Chaque bougie est découpée en rangées de prix : à gauche le volume vendu, à droite le volume acheté. Le delta est leur différence ; le POC, la rangée la plus échangée.".into(),
        ),
        s(
            "how_imbalance",
            format!(
                "Déséquilibre en diagonale : les achats d'une rangée font au moins {} % des ventes de la rangée du dessous (ou les ventes, des achats du dessus). Il montre un côté plus agressif à ce prix, sans dire la suite.",
                num(imb, 0)
            ),
        ),
    ];
    let Some(f) = data.flow else {
        return super::live::missing("l'orderflow (même vide)", how);
    };
    // (the screen's orderflow also holds the bar in progress: only the closed bars' are kept)
    let n = b.len();
    let last = b.last().map_or(i64::MIN, |x| x.time);
    let reel = &f[..f.partition_point(|r| r.time <= last)];
    let fp = flow::footprint(b, vals, reel);
    let Some(x) = fp.bars.last().filter(|x| x.i + 1 == n) else {
        return super::too_few("le footprint", 1, how);
    };
    let origin = match x.origine {
        flow::Origine::Reel => "vrais trades complets",
        flow::Origine::Incomplet => "trades incomplets, rangées estimées d'après la bougie",
        flow::Origine::Estime => "estimée d'après la bougie (pas d'orderflow réel)",
    };
    let mut now = vec![s("origin", format!("Dernière bougie fermée : {origin}."))];
    if p.on("dl") {
        now.push(s(
            "delta",
            format!("Delta de la dernière bougie : {}{}.", if x.d > 0.0 { "+" } else { "" }, num(x.d, 2)),
        ));
    }
    if p.on("poc") {
        now.push(s("poc", format!("POC de la dernière bougie à {}.", price((x.poc as f64 + 0.5) * fp.step))));
    }
    let (buy, sell) = imbalances(&x.rows, imb / 100.0);
    now.push(s(
        "imbalances",
        match (buy, sell) {
            (0, 0) => "Aucun déséquilibre en diagonale sur la dernière bougie.".to_string(),
            _ => format!("Déséquilibres en diagonale sur la dernière bougie : {buy} côté acheteurs, {sell} côté vendeurs."),
        },
    ));
    let recent: Vec<&flow::FpBar> = fp.bars.iter().rev().take(LOOKBACK).collect();
    let pos = recent.iter().filter(|q| q.d > 0.0).count();
    now.push(s(
        "recent",
        format!(
            "Sur les {} dernières bougies : delta positif {pos} fois, négatif ou nul {} fois.",
            recent.len(),
            recent.len() - pos
        ),
    ));
    let change = vec![s(
        "change",
        "Un delta de l'autre signe sur les prochaines bougies changerait la lecture.".into(),
    )];
    let lean = if x.d > 0.0 {
        Lean::Up
    } else if x.d < 0.0 {
        Lean::Down
    } else {
        Lean::Neutral
    };
    let note = match x.origine {
        flow::Origine::Reel => "vrais trades",
        _ => "estimé",
    };
    (
        vec![block(NOW, now), block(HOW, how), block(CHANGE, change)],
        Some(Sense {
            lean,
            note: Some(note),
            value: None,
        }),
    )
}
