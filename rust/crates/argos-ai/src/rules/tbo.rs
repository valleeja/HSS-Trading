//! The reading of Argos TBO (its band, its status table and its Vector candles): the very values and marks the chart
//! draws, from `argos_script::indicators::tbo::compute` with the same settings (no second computation), and only
//! what the chart shows with these settings (argos-render's osc.rs `draw_tbo` and the Vector candles of mod.rs).

use super::ai_trend::bar_secs;
use super::{block, lean_of, s, valued, Blocks, CHANGE, HOW, NOW};
use crate::format::{bars as n_bars, num};
use crate::Sense;
use argos_core::Bar;
use argos_script::indicators::tbo::{self, BreakKind, LastEvent};
use argos_script::inputs::Inputs;

/// Marks older than this, in bars, are not « recent ».
pub(super) const RECENT: usize = 50;
/// A divergence confirmed more than this many bars ago is not « en cours » (as for the Cipher).
pub(super) const DIV_RECENT: usize = 20;

pub(super) const ROLES: [(&str, &str); 9] = [
    ("sig_on", "Affiche la ligne Signal (la moyenne du delta)."),
    ("dotsU", "Affiche les points des breakouts haussiers ▲."),
    ("dotsD", "Affiche les points des breakouts baissiers ▼."),
    ("paint", "Colore les bougies Vector 150+ et 200+."),
    ("vec150", "Seuil Vector 150+ (× la moyenne du volume)."),
    ("vec200", "Seuil Vector 200+ (× la moyenne du volume)."),
    ("div_on", "Affiche les divergences."),
    ("dash", "Affiche le tableau de statut (TBO PRECISION)."),
    ("adx_on", "Filtre le range par l'ADX (ligne FILTRES du tableau)."),
];

/// « sur la dernière bougie » or « il y a N bougies ».
fn ago(n: usize, i: usize) -> String {
    if i + 1 == n {
        "sur la dernière bougie".to_string()
    } else {
        format!("il y a {}", n_bars(n - 1 - i))
    }
}

pub(super) fn read(b: &[Bar], vals: &[f64]) -> Blocks {
    let p = Inputs { params: &tbo::PARAMS, vals };
    let (dots_up, dots_down, dash) = (p.on("dotsU"), p.on("dotsD"), p.on("dash"));
    let vector_shown = p.on("paint") || dash;
    let mut how = vec![s(
        "how_delta",
        "Le delta TBO est la force haussière moins la force baissière : au-dessus de zéro, la hausse domine ; en dessous, la baisse.".into(),
    )];
    if p.on("sig_on") {
        how.push(s(
            "how_signal",
            "La ligne Signal est la moyenne du delta : le delta au-dessus d'elle montre une force qui monte.".into(),
        ));
    }
    if dots_up || dots_down {
        how.push(s(
            "how_breaks",
            "Points de break : la clôture sort d'une ligne de tendance. Point plein : break validé ; point avec halo : break validé avec une bougie Vector 200+ ; petit point pâle : break bloqué par un filtre (pas de signal).".into(),
        ));
    }
    if vector_shown {
        how.push(s(
            "how_vector",
            format!(
                "Bougie Vector : un volume d'au moins {} fois la moyenne (150+) ou {} fois (200+).",
                num(p.get("vec150"), 1),
                num(p.get("vec200"), 1)
            ),
        ));
    }
    if p.on("div_on") {
        how.push(s(
            "how_divergence",
            "Divergence entre le prix et le delta : régulière (le prix va plus loin, le delta non) ou cachée (l'inverse).".into(),
        ));
    }
    let n = b.len();
    let Some(secs) = bar_secs(b) else {
        return super::too_few("le TBO", 2, how);
    };
    let t = tbo::compute(b, secs, &tbo::Settings::from_inputs(vals));
    let Some(d) = t.core.delta.last().copied().filter(|x| x.is_finite()) else {
        // (the delta starts with the first confirmed high and low: a straight move has none)
        let why = format!(
            "Pas encore de valeur TBO : il faut un sommet et un creux confirmés ({} bougies de chaque côté) pour tracer les lignes de tendance.",
            t.active.length
        );
        return (vec![block(NOW, vec![s("too_few", why)]), block(HOW, how)], None);
    };
    let mut now = vec![s(
        "delta",
        format!(
            "Delta TBO à {} : {}.",
            num(d, 4),
            if d > 0.0 {
                "la force haussière domine"
            } else {
                "la force baissière domine"
            }
        ),
    )];
    let sig = t.signal.last().copied().filter(|x| x.is_finite());
    if let (true, Some(sg)) = (p.on("sig_on"), sig) {
        now.push(s(
            "signal",
            format!(
                "Le delta est {} sa ligne Signal ({}).",
                if d > sg { "au-dessus de" } else { "sous" },
                num(sg, 4)
            ),
        ));
    }
    let shown = |up: bool| if up { dots_up } else { dots_down };
    let last_break = t
        .breaks
        .iter()
        .filter(|x| shown(x.up) && x.index < n)
        .max_by_key(|x| x.index)
        .filter(|x| n - 1 - x.index <= RECENT);
    if dots_up || dots_down {
        now.push(match last_break {
            Some(x) => {
                let kind = match x.kind {
                    BreakKind::Vector => "break validé avec Vector 200+ (point avec halo)",
                    BreakKind::Valid => "break validé (point plein)",
                    BreakKind::Filtered => "break bloqué par un filtre (petit point pâle)",
                };
                s(
                    "last_break",
                    format!(
                        "Dernier break {} : {kind}, {}.",
                        if x.up { "haussier ▲" } else { "baissier ▼" },
                        ago(n, x.index)
                    ),
                )
            }
            None => s("no_break", format!("Aucun point de break dans les {RECENT} dernières bougies.")),
        });
    }
    if vector_shown {
        let up = b[n - 1].close >= b[n - 1].open;
        match t.vector.get(n - 1) {
            Some(2) => now.push(s(
                "vector",
                format!("La dernière bougie est une bougie Vector 200+ {}.", if up { "haussière" } else { "baissière" }),
            )),
            Some(1) => now.push(s(
                "vector",
                format!("La dernière bougie est une bougie Vector 150+ {}.", if up { "haussière" } else { "baissière" }),
            )),
            _ => {}
        }
    }
    if p.on("div_on") {
        let len = t.active.div_len;
        if let Some(dv) = t
            .divs
            .iter()
            .filter(|dv| dv.pivot + len < n && n - 1 - (dv.pivot + len) <= DIV_RECENT)
            .max_by_key(|dv| dv.pivot)
        {
            now.push(s(
                "divergence",
                format!(
                    "Divergence {} {} : pivot il y a {}, confirmée {}.",
                    if dv.hidden { "cachée" } else { "régulière" },
                    if dv.bull { "haussière" } else { "baissière" },
                    n_bars(n - 1 - dv.pivot),
                    ago(n, dv.pivot + len)
                ),
            ));
        }
    }
    if dash {
        let adx = t.adx.last().copied().filter(|x| x.is_finite());
        let trend_ok = !p.on("adx_on") || adx.is_some_and(|v| v >= t.active.adx_min);
        let last = match t.last {
            Some(LastEvent::Break { up, vector }) => format!(
                "{} {}",
                if vector { "breakout avec Vector 200+" } else { "break" },
                if up { "haussier ▲" } else { "baissier ▼" }
            ),
            Some(LastEvent::Divergence { bull, hidden }) => {
                format!(
                    "divergence {} {}",
                    if hidden { "cachée" } else { "régulière" },
                    if bull { "haussière ▲" } else { "baissière ▼" }
                )
            }
            None => "aucun".to_string(),
        };
        now.push(s(
            "status",
            format!(
                "Tableau de statut : filtres « {} » (ADX {}), dernier événement : {last}.",
                if trend_ok { "TREND OK" } else { "RANGE" },
                adx.map_or("—".to_string(), |v| num(v, 1))
            ),
        ));
    }
    let change = vec![if let (true, Some(_)) = (p.on("sig_on"), sig) {
        s(
            "change_signal",
            format!(
                "Un passage du delta {} sa ligne Signal changerait la lecture de la force.",
                if sig.is_some_and(|sg| d > sg) { "sous" } else { "au-dessus de" }
            ),
        )
    } else {
        s(
            "change_zero",
            format!(
                "Un passage du delta {} zéro changerait la lecture.",
                if d > 0.0 { "sous" } else { "au-dessus de" }
            ),
        )
    }];
    let note = match (vector_shown, t.vector.get(n - 1)) {
        (true, Some(2)) => Some("bougie Vector 200+"),
        (true, Some(1)) => Some("bougie Vector 150+"),
        _ => None,
    };
    let sense = valued(
        Some(Sense {
            lean: lean_of(d),
            note,
            value: None,
        }),
        format!("delta {}", num(d, 4)),
    );
    (vec![block(NOW, now), block(HOW, how), block(CHANGE, change)], sense)
}
