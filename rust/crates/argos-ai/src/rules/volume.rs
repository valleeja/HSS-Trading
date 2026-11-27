//! The readings of « Argos Heatmap Volume » and « Argos Volume Profile »: the volume tiers and the profile the chart
//! draws, from `argos_script::indicators::classic::{heatmap_volume, volume_profile}` with the same settings (no second
//! computation). Their summary tables (« Tableau résumé »), when shown, are read from the rows the chart draws
//! (`classic::{heatmap_tableau, profile_tableau}`).

use super::{block, s, table, Blocks, CHANGE, HOW, NOW};
use crate::format::{bars as n_bars, num, percent, price};
use crate::{Lean, Sense};
use argos_core::Bar;
use argos_script::indicators::classic;
use argos_script::inputs::Inputs;
use argos_script::v1_inputs::{HEATMAP_PARAMS, PROFILE_PARAMS};

/// Marks older than this, in bars, are not « recent ».
const RECENT: usize = 50;

pub(super) const HEATMAP_ROLES: [(&str, &str); 6] = [
    ("len", "Bougies de la moyenne et de l'écart-type du volume."),
    ("t1", "Seuil « Très fort », en écarts-types au-dessus de la moyenne."),
    ("t2", "Seuil « Fort »."),
    ("t3", "Seuil « Moyen »."),
    ("t4", "Seuil « Normal » ; en dessous : « Faible »."),
    ("showTable", "Affiche le tableau résumé ; Argos AI en reprend les lignes."),
];
pub(super) const PROFILE_ROLES: [(&str, &str); 4] = [
    ("bars", "Nombre de bougies de la plage du profil."),
    ("rows", "Nombre de rangées de prix."),
    ("va", "Part du volume dans la zone de valeur, en %."),
    ("showTable", "Affiche le tableau résumé ; Argos AI en reprend les lignes."),
];

/// The names of the tiers, as the settings name them.
const TIERS: [&str; 5] = ["Très fort", "Fort", "Moyen", "Normal", "Faible"];

/// « écart-type » or « écarts-types » (2 and more).
fn sd_word(x: f64) -> &'static str {
    if x.abs() >= 2.0 {
        "écarts-types"
    } else {
        "écart-type"
    }
}

pub(super) fn read_heatmap(b: &[Bar], vals: &[f64]) -> Blocks {
    let p = Inputs { params: &HEATMAP_PARAMS, vals };
    let len = p.len("len").max(2);
    let t = ["t1", "t2", "t3", "t4"].map(|k| p.get(k));
    let how = vec![s(
        "how_tiers",
        format!(
            "Chaque volume est comparé à la moyenne des {len} dernières bougies, en écarts-types : « Très fort » à partir de {}, « Fort » de {}, « Moyen » de {}, « Normal » de {}, « Faible » en dessous. Un volume fort montre une participation inhabituelle, pas un sens.",
            num(t[0], 1),
            num(t[1], 1),
            num(t[2], 1),
            num(t[3], 1)
        ),
    )];
    let n = b.len();
    if n < 2 {
        return super::too_few("Argos Heatmap Volume", 2, how);
    }
    let h = classic::heatmap_volume(b, vals);
    let i = n - 1;
    let z = if h.std[i] > 0.0 { (h.volume[i] - h.mean[i]) / h.std[i] } else { 0.0 };
    let tier = TIERS[(h.tier[i] as usize).min(4)];
    let mut now = vec![s(
        "last",
        format!(
            "Volume de la dernière bougie : {}, à {}{} {} de la moyenne ({}) : palier « {tier} ».",
            num(h.volume[i], 2),
            if z > 0.0 { "+" } else { "" },
            num(z, 1),
            sd_word(z),
            num(h.mean[i], 2)
        ),
    )];
    if let Some(k) = (0..n).rev().take(RECENT + 1).find(|&k| h.tier[k] == 0) {
        if k != i {
            now.push(s("strong", format!("Dernière bougie « Très fort » il y a {}.", n_bars(i - k))));
        }
    } else {
        now.push(s("strong", format!("Aucune bougie « Très fort » dans les {} dernières bougies.", RECENT + 1)));
    }
    let count = (0..n).rev().take(20).filter(|&k| h.tier[k] <= 1).count();
    now.push(s(
        "count",
        match count {
            0 => "Aucune bougie « Fort » ou « Très fort » sur les 20 dernières.".to_string(),
            1 => "1 bougie « Fort » ou « Très fort » sur les 20 dernières.".to_string(),
            c => format!("{c} bougies « Fort » ou « Très fort » sur les 20 dernières."),
        },
    ));
    if p.on("showTable") {
        now.extend(table(&classic::heatmap_tableau(&h)));
    }
    let change = vec![s("change", "Une bougie au volume d'un autre palier changerait la lecture.".into())];
    let sense = Some(Sense {
        lean: Lean::NoDirection,
        note: Some(match h.tier[i] {
            0 => "volume très fort",
            1 => "volume fort",
            2 => "volume moyen",
            3 => "volume normal",
            _ => "volume faible",
        }),
        value: None,
    });
    (vec![block(NOW, now), block(HOW, how), block(CHANGE, change)], sense)
}

pub(super) fn read_profile(b: &[Bar], vals: &[f64]) -> Blocks {
    let p = Inputs { params: &PROFILE_PARAMS, vals };
    let (bars, va) = (p.len("bars").max(1), p.get("va").clamp(1.0, 100.0));
    let how = vec![s(
        "how_profile",
        format!(
            "Le profil range le volume des {bars} dernières bougies par niveau de prix. Le POC est le niveau le plus échangé ; la zone de valeur, les niveaux autour du POC qui font {} % du volume. Le prix revient souvent vers ces niveaux, sans que ce soit sûr.",
            num(va, 0)
        ),
    )];
    let n = b.len();
    let Some(pr) = classic::volume_profile(b, vals).filter(|_| n >= 2) else {
        return super::too_few("Argos Volume Profile", 2, how);
    };
    let c = b[n - 1].close;
    let poc = pr.bot + pr.step * (pr.poc as f64 + 0.5);
    let (lo, hi) = (pr.bot + pr.step * pr.va_lo as f64, pr.bot + pr.step * (pr.va_hi + 1) as f64);
    let place = if c > hi {
        "au-dessus de la zone de valeur"
    } else if c < lo {
        "sous la zone de valeur"
    } else {
        "dans la zone de valeur"
    };
    let up: f64 = pr.up.iter().sum();
    let all: f64 = up + pr.down.iter().sum::<f64>();
    let mut now = vec![
        s(
            "poc",
            format!(
                "POC à {} sur les {} : la clôture ({}) est {} ({}).",
                price(poc),
                n_bars(n - pr.from),
                price(c),
                if c > poc {
                    "au-dessus"
                } else if c < poc {
                    "en dessous"
                } else {
                    "au niveau"
                },
                percent((c - poc) / poc * 100.0)
            ),
        ),
        s(
            "value_area",
            format!("Zone de valeur de {} à {} : la clôture est {place}.", price(lo), price(hi)),
        ),
    ];
    if all > 0.0 {
        now.push(s(
            "sides",
            format!("Sur la plage, {} % du volume est sur des bougies en hausse.", num(up / all * 100.0, 1)),
        ));
    }
    if p.on("showTable") {
        now.extend(table(&classic::profile_tableau(&pr, b, &price)));
    }
    let change = vec![s(
        "change",
        format!(
            "Une clôture qui {} la zone de valeur changerait la lecture.",
            if place == "dans la zone de valeur" { "sort de" } else { "revient dans" }
        ),
    )];
    let sense = Some(Sense {
        lean: Lean::NoDirection,
        note: Some(place),
        value: Some(format!("POC à {}", price(poc))),
    });
    (vec![block(NOW, now), block(HOW, how), block(CHANGE, change)], sense)
}
