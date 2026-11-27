//! The reading of Argos TVD Pro (on the price): the very marks, cloud, Vector candles and table the chart draws, from
//! `argos_script::indicators::tvd::compute` with the same settings (no second computation). The script itself leaves
//! out what its settings turn off (marks, cloud, Vector candles, table), and argos-render's flow_draw.rs `draw_tvd`
//! draws what it returns: the reading follows the script's outputs.

use super::{block, lean_of, s, valued, Blocks, CHANGE, HOW, NOW};
use crate::format::{bars as n_bars, num, price};
use crate::{Lean, Sense};
use argos_core::Bar;
use argos_script::indicators::tvd::{self, MarkKind, Vector};
use argos_script::inputs::Inputs;

/// Marks older than this, in bars, are not « recent ».
pub(super) const RECENT: usize = 50;
/// A divergence confirmed more than this many bars ago is not « en cours » (as for the Cipher).
pub(super) const DIV_RECENT: usize = 20;

pub(super) const ROLES: [(&str, &str); 14] = [
    ("emaFastLen", "Longueur de l'EMA rapide."),
    ("emaSlowLen", "Longueur de l'EMA lente."),
    ("emaMidLen", "Longueur de l'EMA moyenne."),
    ("emaTrendLen", "Longueur de l'EMA tendance."),
    ("showCloud", "Affiche le nuage entre l'EMA rapide et l'EMA lente."),
    ("bullLvl", "Niveau haussier de l'AI Index (nuage vert, triangles Trend)."),
    ("bearLvl", "Niveau baissier de l'AI Index (nuage rouge, triangles Trend)."),
    ("showVector", "Colore les bougies Vector (gros volume)."),
    ("vecMult", "Seuil des bougies Vector hautes (× la moyenne du volume)."),
    ("showSignals", "Affiche les repères Trend et GOLD."),
    ("showAiSignals", "Affiche les repères AI."),
    ("showPatterns", "Affiche les losanges de retournement."),
    ("pivR", "Bougies à droite d'un pivot : une divergence est confirmée après elles."),
    ("showTable", "Affiche le tableau Argos TVD Pro."),
];

/// The name of a mark as the chart draws it.
fn mark_name(k: MarkKind) -> &'static str {
    match k {
        MarkKind::TrendBuy => "triangle vert (Trend)",
        MarkKind::TrendSell => "triangle rouge (Trend)",
        MarkKind::AiBuy => "rond vert (AI)",
        MarkKind::AiSell => "rond rouge (AI)",
        MarkKind::RevBull => "losange vert (retournement)",
        MarkKind::RevBear => "losange rouge (retournement)",
        MarkKind::GoldBuy => "étiquette « 🚀 GOLD BUY »",
        MarkKind::GoldSell => "étiquette « 💀 GOLD SELL »",
        MarkKind::Bull => "étiquette « Bull » (divergence haussière)",
        MarkKind::HBull => "étiquette « H Bull » (divergence haussière cachée)",
        MarkKind::Bear => "étiquette « Bear » (divergence baissière)",
        MarkKind::HBear => "étiquette « H Bear » (divergence baissière cachée)",
    }
}

fn is_div(k: MarkKind) -> bool {
    matches!(k, MarkKind::Bull | MarkKind::HBull | MarkKind::Bear | MarkKind::HBear)
}

/// « sur la dernière bougie » or « il y a N bougies ».
fn ago(n: usize, i: usize) -> String {
    if i + 1 == n {
        "sur la dernière bougie".to_string()
    } else {
        format!("il y a {}", n_bars(n - 1 - i))
    }
}

pub(super) fn read(b: &[Bar], vals: &[f64]) -> Blocks {
    let p = Inputs { params: &tvd::PARAMS, vals };
    let (bull, bear) = (p.get("bullLvl"), p.get("bearLvl"));
    let t = tvd::compute(b, vals);
    let n = b.len();
    let mut how = vec![s(
        "how_emas",
        format!(
            "Quatre EMA sur le prix : rapide ({}), lente ({}), moyenne ({}) et tendance ({}).",
            p.len("emaFastLen"),
            p.len("emaSlowLen"),
            p.len("emaMidLen"),
            p.len("emaTrendLen")
        ),
    )];
    if !t.cloud.is_empty() {
        how.push(s(
            "how_cloud",
            format!(
                "Nuage vert : l'AI Index est au-dessus de {} et le prix au-dessus de l'EMA lente ; rouge : l'AI sous {} et le prix sous l'EMA lente ; gris sinon.",
                num(bull, 0),
                num(bear, 0)
            ),
        ));
    }
    if p.on("showSignals") || p.on("showAiSignals") || p.on("showPatterns") {
        how.push(s(
            "how_marks",
            format!(
                "Triangles Trend : l'AI Index passe {} (vert) ou {} (rouge) ; « GOLD » : le même passage avec une bougie Vector haute et le prix du bon côté de l'EMA tendance ; ronds AI : l'AI croise sa ligne signal ; losanges : retournements de bougies. Ce sont des noms de repères, pas des consignes.",
                side(true, bull),
                side(false, bear)
            ),
        ));
    }
    if !t.vector.is_empty() {
        how.push(s(
            "how_vector",
            format!(
                "Bougie Vector : volume d'au moins {} fois la moyenne de 20 bougies (haute) ou 1,5 fois (150 %).",
                num(p.get("vecMult"), 1)
            ),
        ));
    }
    let (Some(&f), Some(&sl)) = (t.fast.last(), t.slow.last()) else {
        return super::too_few("le TVD", 2, how);
    };
    // (the EMAs start at their length, like the Pine's ta.ema, B-098; the reading waits for the AI Index and slow EMA)
    let ai_ready = t.ai.last().is_some_and(|x| x.is_finite());
    if !(f.is_finite() && sl.is_finite() && ai_ready && n >= p.len("emaSlowLen")) {
        return super::too_few("le TVD", p.len("emaSlowLen"), how);
    }
    let mut now = vec![s(
        "emas",
        format!(
            "EMA rapide à {} : {} l'EMA lente ({}).",
            price(f),
            if f > sl { "au-dessus de" } else { "sous" },
            price(sl)
        ),
    )];
    let cloud = t.cloud.last().copied();
    if let Some(c) = cloud {
        now.push(s(
            "cloud",
            match c {
                1 => "Nuage vert : l'AI Index et le prix vont dans le sens de la hausse.".to_string(),
                -1 => "Nuage rouge : l'AI Index et le prix vont dans le sens de la baisse.".to_string(),
                _ => "Nuage gris : pas de tendance nette entre l'AI Index et le prix.".to_string(),
            },
        ));
    }
    if let Some(Some(v)) = t.vector.last() {
        now.push(s(
            "vector",
            match v {
                Vector::HighUp => "La dernière bougie est une bougie Vector haute haussière.",
                Vector::HighDown => "La dernière bougie est une bougie Vector haute baissière.",
                Vector::MidUp => "La dernière bougie est une bougie Vector 150 % haussière.",
                Vector::MidDown => "La dernière bougie est une bougie Vector 150 % baissière.",
            }
            .to_string(),
        ));
    }
    let last = t
        .marks
        .iter()
        .filter(|m| m.index < n && !is_div(m.kind))
        .max_by_key(|m| m.index)
        .filter(|m| n - 1 - m.index <= RECENT);
    if let Some(m) = last {
        now.push(s("last_mark", format!("Dernier repère : {} {}.", mark_name(m.kind), ago(n, m.index))));
    }
    let r = p.len("pivR");
    let div = t
        .marks
        .iter()
        .filter(|m| is_div(m.kind) && m.index + r < n && n - 1 - (m.index + r) <= DIV_RECENT)
        .max_by_key(|m| m.index);
    if let Some(m) = div {
        now.push(s(
            "divergence",
            format!(
                "Divergence : {} posée il y a {}, confirmée {}.",
                mark_name(m.kind),
                n_bars(n - 1 - m.index),
                ago(n, m.index + r)
            ),
        ));
    }
    if !t.dash.is_empty() {
        let rows: Vec<String> = t.dash.iter().map(|(k, v)| format!("{k} : {v}")).collect();
        now.push(s("table", format!("Tableau Argos TVD Pro : {}.", rows.join(" ; "))));
    }
    let change = vec![s(
        "change_emas",
        format!(
            "Un croisement de l'EMA rapide {} l'EMA lente changerait la lecture.",
            if f > sl { "sous" } else { "au-dessus de" }
        ),
    )];
    let lean = match cloud {
        Some(1) => Lean::Up,
        Some(-1) => Lean::Down,
        Some(_) => Lean::Neutral,
        None => lean_of(f - sl),
    };
    let note = div.map(|m| {
        if matches!(m.kind, MarkKind::Bull | MarkKind::HBull) {
            "divergence haussière"
        } else {
            "divergence baissière"
        }
    });
    let sense = valued(Some(Sense { lean, note, value: None }), format!("EMA rapide {}", price(f)));
    (vec![block(NOW, now), block(HOW, how), block(CHANGE, change)], sense)
}

/// « au-dessus de 60 » / « sous 40 ».
fn side(up: bool, level: f64) -> String {
    if up {
        format!("au-dessus de {}", num(level, 0))
    } else {
        format!("sous {}", num(level, 0))
    }
}
