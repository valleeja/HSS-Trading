//! The reading of « Argos Price Action » (Argos Order Blocks): the very zones, breakers, labels and polarity marks the
//! chart draws, from `argos_script::indicators::order_blocks::compute` with the same settings (no second
//! computation), and only what argos-render's zones.rs `draw_pa` shows with these settings: the last « nHaut » rising
//! and « nBas » falling zones. At most 3 zones are named, the nearest to the close.

use super::smc::{born, where_};
use super::{block, s, Blocks, CHANGE, HOW, NOW};
use crate::format::{bars as n_bars, num, price};
use crate::{Lean, Sense};
use argos_core::Bar;
use argos_script::indicators::order_blocks::{self, Zone};
use argos_script::inputs::Inputs;

/// Labels older than this, in bars, are not « recent ».
pub(super) const RECENT: usize = 50;
/// At most this many zones are named.
pub(super) const MAX_ZONES: usize = 3;

pub(super) const ROLES: [(&str, &str); 9] = [
    ("nHaut", "Nombre de zones haussières affichées."),
    ("nBas", "Nombre de zones baissières affichées."),
    ("buySell", "Affiche les étiquettes « BUY » / « SELL » des zones actives."),
    ("pwr", "Affiche la force Pwr % de chaque zone."),
    ("polarite", "Marque les changements de polarité ▲ ▼."),
    ("n4", "Affiche les étiquettes BULL / BEAR n/4."),
    ("n4Min", "Nombre de critères minimum pour l'étiquette n/4."),
    ("pwrMin", "Force minimum pour qu'une zone compte dans le n/4."),
    ("tableau", "Affiche le tableau : zones actives et cassées de chaque côté, dernière polarité."),
];

fn distance(c: f64, z: &Zone) -> f64 {
    if c > z.high {
        c - z.high
    } else if c < z.low {
        z.low - c
    } else {
        0.0
    }
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
    let p = Inputs {
        params: &order_blocks::PARAMS,
        vals,
    };
    let mut how = vec![s(
        "how_zones",
        "Zone : la dernière bougie de sens opposé avant une impulsion qui laisse un FVG et casse le dernier sommet (ou creux) de swing. Haussière sous le prix, baissière au-dessus ; quand une clôture la traverse de l'autre côté, elle devient un breaker (sa polarité s'inverse).".into(),
    )];
    if p.on("pwr") {
        how.push(s(
            "how_pwr",
            if p.pick("pwrMode") == 0 {
                "Pwr % : la force de la zone, la part du volume de son impulsion parmi les zones affichées du même côté (100 % en tout).".into()
            } else {
                "Pwr % : la force de la zone, le volume de son impulsion divisé par le volume moyen (100 % = la moyenne).".into()
            },
        ));
    }
    if p.on("buySell") {
        how.push(s(
            "how_names",
            "« BUY » et « SELL » sont les noms des étiquettes des zones, pas des consignes.".into(),
        ));
    }
    if p.on("n4") {
        how.push(s(
            "how_n4",
            format!(
                "BULL / BEAR n/4 : quand le prix entre dans une zone, 4 critères sont comptés (prix dans la zone, sens de la dernière impulsion, force d'au moins {} %, bougie vector dans le même sens) ; l'étiquette paraît à partir de {} sur 4.",
                num(p.get("pwrMin"), 0),
                num(p.get("n4Min"), 0)
            ),
        ));
    }
    let n = b.len();
    if n < 2 {
        return super::too_few("Argos Price Action", 2, how);
    }
    let ob = order_blocks::compute(b, vals);
    let c = b[n - 1].close;
    let (nu, nd) = (p.get("nHaut").max(0.0) as usize, p.get("nBas").max(0.0) as usize);
    let mut zones: Vec<(&Zone, bool)> = ob
        .rising
        .iter()
        .take(nu)
        .map(|z| (z, true))
        .chain(ob.falling.iter().take(nd).map(|z| (z, false)))
        .collect();
    zones.sort_by(|x, y| distance(c, x.0).total_cmp(&distance(c, y.0)));
    let mut now: Vec<_> = zones
        .iter()
        .take(MAX_ZONES)
        .map(|(z, up)| {
            let state = match z.broken_at {
                Some(k) => format!(", cassée (breaker) {}", ago(n, k)),
                None => {
                    let mut tags = vec![];
                    if p.on("buySell") {
                        tags.push(if *up { "BUY" } else { "SELL" }.to_string());
                    }
                    if p.on("pwr") {
                        tags.push(format!("Pwr {} %", num(z.pwr, 0)));
                    }
                    if tags.is_empty() {
                        String::new()
                    } else {
                        format!(", étiquette « {} »", tags.join(" · "))
                    }
                }
            };
            s(
                "zone",
                format!(
                    "Zone {} de {} à {}{state}, {} : {}.",
                    if *up { "haussière" } else { "baissière" },
                    price(z.low),
                    price(z.high),
                    born(n.saturating_sub(1 + z.origin)),
                    where_(c, z.low, z.high)
                ),
            )
        })
        .collect();
    if now.is_empty() {
        now.push(s("no_zone", "Aucune zone affichée.".into()));
    }
    if p.on("polarite") {
        if let Some(f) = ob.flips.iter().filter(|f| f.at < n && n - 1 - f.at <= RECENT).max_by_key(|f| f.at) {
            now.push(s(
                "flip",
                format!(
                    "Dernier changement de polarité : {} à {}, {}.",
                    if f.support {
                        "« ▲ » (un ancien plafond devient support)"
                    } else {
                        "« ▼ » (un ancien plancher devient résistance)"
                    },
                    price(f.price),
                    ago(n, f.at)
                ),
            ));
        }
    }
    if p.on("n4") {
        if let Some(t) = ob.notes.iter().filter(|t| t.at < n && n - 1 - t.at <= RECENT).max_by_key(|t| t.at) {
            now.push(s(
                "n4",
                format!("Dernière étiquette « {} {}/4 » {}.", if t.bull { "BULL" } else { "BEAR" }, t.n, ago(n, t.at)),
            ));
        }
    }
    if p.on("tableau") {
        let count = |v: &[Zone]| {
            (
                v.iter().filter(|z| z.broken_at.is_none()).count(),
                v.iter().filter(|z| z.broken_at.is_some()).count(),
            )
        };
        let ((ua, ub), (da, db)) = (count(&ob.rising), count(&ob.falling));
        let pol = ob
            .flips
            .last()
            .map_or("aucune", |f| if f.support { "▲ support confirmé" } else { "▼ résistance confirmée" });
        now.push(s(
            "table",
            format!("Tableau : zones haussières {ua} actives (+ {ub} cassées), zones baissières {da} actives (+ {db} cassées), polarité « {pol} »."),
        ));
    }
    let change = vec![s(
        "change",
        "Une clôture qui traverse la zone la plus proche (elle deviendrait un breaker), ou une nouvelle zone, changerait la lecture.".into(),
    )];
    let inside = zones.iter().find(|(z, _)| distance(c, z) == 0.0);
    let note = inside.map(|(z, up)| match (z.broken_at.is_some(), up) {
        (true, _) => "dans un breaker",
        (false, true) => "dans une zone haussière",
        (false, false) => "dans une zone baissière",
    });
    (
        vec![block(NOW, now), block(HOW, how), block(CHANGE, change)],
        Some(Sense {
            lean: Lean::NoDirection,
            note,
            value: None,
        }),
    )
}
