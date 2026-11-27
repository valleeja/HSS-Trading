//! The reading of Argos Vector Zone: the very zones, liquidity levels, labels and Vector candles the chart draws, from
//! `argos_script::indicators::vector_zone::compute` with the same settings (no second computation). The script
//! leaves out what its settings turn off (zones, liquidity, labels); the Vector candles follow « showVec », as in
//! argos-render's mod.rs. At most 3 zones are named, the nearest to the price.

use super::ai_trend::bar_secs;
use super::{block, s, valued, Blocks, CHANGE, HOW, NOW};
use crate::format::{bars as n_bars, num, percent, price};
use crate::{Lean, Sense};
use argos_core::Bar;
use argos_lens::explain::Sentence;
use argos_script::indicators::vector_zone::{self, Signal, VecKind, VectorZone, Zone};
use argos_script::inputs::Inputs;

/// Labels older than this, in bars, are not « recent ».
pub(super) const RECENT: usize = 50;
/// At most this many zones are named.
pub(super) const MAX_ZONES: usize = 3;

pub(super) const ROLES: [(&str, &str); 5] = [
    ("showZones", "Affiche les zones d'offre et de demande avec leur note."),
    ("showLiq", "Affiche la liquidité : BSL (sommets) et SSL (creux), et les sweeps."),
    ("showVec", "Colore les bougies Vector et Climax (gros volume)."),
    ("aboveMult", "Seuil des bougies VECTOR (× la moyenne du volume)."),
    ("climaxMult", "Seuil des bougies CLIMAX (× la moyenne du volume)."),
];

/// The name of a label as the chart draws it.
fn signal_name(sg: Signal) -> &'static str {
    match sg {
        Signal::ConfLong => "étiquette « CONF » sous la bougie (cassure de trendline confirmée, vers le haut)",
        Signal::ConfShort => "étiquette « CONF » au-dessus de la bougie (cassure de trendline confirmée, vers le bas)",
        Signal::SweepSsl { vec: true } => "étiquette « 🔥💧 Sweep SSL » (creux pris puis réintégrés, avec une bougie Vector)",
        Signal::SweepSsl { vec: false } => "étiquette « 💧 Sweep SSL » (creux pris puis réintégrés)",
        Signal::SweepBsl { vec: true } => "étiquette « 🔥💧 Sweep BSL » (sommets pris puis réintégrés, avec une bougie Vector)",
        Signal::SweepBsl { vec: false } => "étiquette « 💧 Sweep BSL » (sommets pris puis réintégrés)",
        Signal::StrongDemand => "« ▲ » (zone de demande forte touchée)",
        Signal::StrongSupply => "« ▼ » (zone d'offre forte touchée)",
        Signal::PatBreakUp => "flèche vers le haut (cassure de la résistance du pattern)",
        Signal::PatBreakDn => "flèche vers le bas (cassure du support du pattern)",
        Signal::NeckUp => "cassure de la neckline vers le haut",
        Signal::NeckDn => "cassure de la neckline vers le bas",
    }
}

/// The distance from the close to a zone, in % of the close (0 inside).
fn distance(z: &Zone, c: f64) -> f64 {
    if c > z.top {
        (c - z.top) / c * 100.0
    } else if c < z.bot {
        (z.bot - c) / c * 100.0
    } else {
        0.0
    }
}

/// One zone, with where the close stands.
fn zone_sentence(v: &VectorZone, z: &Zone, c: f64, n: usize) -> Sentence {
    let strength = if z.score >= v.strong {
        "forte"
    } else if z.score >= v.weak {
        "faible"
    } else {
        "sous le seuil faible"
    };
    let where_ = super::smc::where_(c, z.bot, z.top);
    s(
        "zone",
        format!(
            "Zone « {} {}/100 · {}T{} » ({strength}) de {} à {}, {} : {where_}.",
            if z.demand { "DEMANDE" } else { "OFFRE" },
            z.score,
            z.touches,
            if z.vector { " · VECTOR" } else { "" },
            price(z.bot),
            price(z.top),
            super::smc::born(n.saturating_sub(1 + z.born))
        ),
    )
}

pub(super) fn read(b: &[Bar], vals: &[f64]) -> Blocks {
    let p = Inputs {
        params: &vector_zone::PARAMS,
        vals,
    };
    let mut how = vec![];
    if p.on("showZones") {
        how.push(s(
            "how_zones",
            "Zone de DEMANDE (sous un creux) ou d'OFFRE (sous un sommet), notée sur 100 : la réaction qui l'a créée (40), une bougie Vector à sa naissance (30), sa fraîcheur (30, tant qu'elle n'est pas touchée) ; « T » compte les touches.".into(),
        ));
    }
    if p.on("showLiq") {
        how.push(s(
            "how_liquidity",
            "BSL : des sommets où des ordres attendent au-dessus ; SSL : des creux, en dessous. Un « Sweep » : le prix les prend puis revient de l'autre côté."
                .into(),
        ));
    }
    if p.on("showVec") {
        how.push(s(
            "how_vector",
            format!(
                "Bougie VECTOR : volume d'au moins {} fois la moyenne ; CLIMAX : {} fois, ou le plus grand volume × amplitude récent.",
                num(p.get("aboveMult"), 1),
                num(p.get("climaxMult"), 1)
            ),
        ));
    }
    let n = b.len();
    let Some(secs) = bar_secs(b) else {
        return super::too_few("Vector Zone", 2, how);
    };
    let v = vector_zone::compute(b, secs, b[n - 1].time + secs, vals);
    let c = b[n - 1].close;
    let mut now = vec![];
    if p.on("showZones") {
        let mut zs: Vec<&Zone> = v.zones.iter().collect();
        zs.sort_by(|a, b| distance(a, c).total_cmp(&distance(b, c)));
        if zs.is_empty() {
            now.push(s("no_zone", "Aucune zone d'offre ou de demande affichée.".into()));
        }
        now.extend(zs.iter().take(MAX_ZONES).map(|z| zone_sentence(&v, z, c, n)));
    }
    if p.on("showLiq") {
        let open: Vec<_> = v.liqs.iter().filter(|l| l.end.is_none()).collect();
        let above = open.iter().filter(|l| l.high && l.price > c).min_by(|a, b| a.price.total_cmp(&b.price));
        let below = open.iter().filter(|l| !l.high && l.price < c).max_by(|a, b| a.price.total_cmp(&b.price));
        let mut parts = vec![];
        if let Some(l) = above {
            parts.push(format!(
                "BSL{} à {} ({})",
                if l.eq { " (sommets égaux)" } else { "" },
                price(l.price),
                percent((l.price - c) / c * 100.0)
            ));
        }
        if let Some(l) = below {
            parts.push(format!(
                "SSL{} à {} ({})",
                if l.eq { " (creux égaux)" } else { "" },
                price(l.price),
                percent((l.price - c) / c * 100.0)
            ));
        }
        if !parts.is_empty() {
            now.push(s("liquidity", format!("Liquidité la plus proche : {}.", parts.join(" ; "))));
        }
    }
    if let Some(&(i, sg)) = v.signals.iter().filter(|x| x.0 < n).max_by_key(|x| x.0).filter(|x| n - 1 - x.0 <= RECENT) {
        let when = if i + 1 == n {
            "sur la dernière bougie".to_string()
        } else {
            format!("il y a {}", n_bars(n - 1 - i))
        };
        now.push(s("last_signal", format!("Dernier événement : {} {when}.", signal_name(sg))));
    }
    if p.on("showVec") {
        if let Some(Some(k)) = v.vec.get(n - 1) {
            let up = v.bull.get(n - 1).copied().unwrap_or(true);
            now.push(s(
                "vector",
                format!(
                    "La dernière bougie est une bougie {} {}.",
                    if *k == VecKind::Climax { "CLIMAX" } else { "VECTOR" },
                    if up { "haussière" } else { "baissière" }
                ),
            ));
        }
    }
    if now.is_empty() {
        now.push(s("nothing", "Rien d'affiché à lire avec ces réglages (zones et liquidité masquées).".into()));
    }
    let change = vec![s(
        "change_zone",
        "Une clôture qui sort de la zone la plus proche, ou un nouveau sweep de liquidité, changerait la lecture.".into(),
    )];
    let inside = v.zones.iter().filter(|_| p.on("showZones")).find(|z| distance(z, c) == 0.0);
    let note = inside.map(|z| if z.demand { "dans une zone de demande" } else { "dans une zone d'offre" });
    let nearest = v
        .zones
        .iter()
        .filter(|_| p.on("showZones"))
        .map(|z| distance(z, c))
        .fold(f64::INFINITY, f64::min);
    let sense = Some(Sense {
        lean: Lean::NoDirection,
        note,
        value: None,
    });
    let sense = if nearest.is_finite() {
        valued(sense, format!("zone la plus proche à {} %", num(nearest, 2)))
    } else {
        sense
    };
    (vec![block(NOW, now), block(HOW, how), block(CHANGE, change)], sense)
}
