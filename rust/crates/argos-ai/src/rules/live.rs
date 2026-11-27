//! The readings of what the screen holds live: « Argos Tape » (the tape's state, kept by the screen from one update to
//! the next), « Argos Heatmap du carnet » (the order-book walls the screen draws) and « Argos Carte des liquidations »
//! (Argos's estimate of liquidation levels). They read only what the screen passes (`ScreenData::tape`, `walls`,
//! `liq`), never a second computation; without it, they say the data is missing. These states include the bar in
//! progress: the texts say « en ce moment ».

use super::{block, s, Blocks, CHANGE, HOW, NOW};
use crate::format::{bars as n_bars, num, percent, price};
use crate::{Lean, ScreenData, Sense};
use argos_core::Bar;
use argos_script::indicators::flow;
use argos_script::inputs::Inputs;

/// Marks older than this, in bars, are not « recent ».
const RECENT: usize = 50;

pub(super) const TAPE_ROLES: [(&str, &str); 3] = [
    ("nl", "Nombre de lignes du ruban affichées."),
    ("minv", "Volume minimum d'une ligne du ruban."),
    ("m2v", "Seuil de delta des marques ⇑ ⇓."),
];

/// « 850 K $ », « 1,2 M $ ».
fn usd(x: f64) -> String {
    let (v, unit) = match x.abs() {
        a if a >= 1e9 => (x / 1e9, " B"),
        a if a >= 1e6 => (x / 1e6, " M"),
        a if a >= 1e3 => (x / 1e3, " K"),
        _ => (x, ""),
    };
    format!("{}{unit} $", num(v, if v.abs() >= 100.0 { 0 } else { 1 }))
}

/// The reading when the screen did not pass its live data.
pub(super) fn missing(what: &str, how: Vec<argos_lens::explain::Sentence>) -> Blocks {
    (
        vec![
            block(
                NOW,
                vec![s("missing", format!("Donnée absente : {what} n'est pas transmis par l'écran, rien à lire."))],
            ),
            block(HOW, how),
        ],
        None,
    )
}

pub(super) fn read_tape(b: &[Bar], vals: &[f64], data: &ScreenData) -> Blocks {
    let p = Inputs {
        params: &flow::TAPE_PARAMS,
        vals,
    };
    let how = vec![
        s(
            "how_tape",
            "Le ruban note, à chaque mise à jour de la bougie en cours, le volume ajouté et le sens du prix ; le delta compte ce volume en plus quand le prix monte, en moins quand il baisse. C'est un état en direct, tenu par l'écran depuis qu'il est ouvert.".into(),
        ),
        s(
            "how_marks",
            format!(
                "« ⬆ ⬇ » : deux bougies de suite dont le delta grandit dans le même sens ; « ⇑ ⇓ » : un delta au-delà de {}.",
                num(p.get("m2v"), 0)
            ),
        ),
    ];
    let Some(t) = data.tape else {
        return missing("le ruban (Argos Tape)", how);
    };
    let n = b.len();
    let mut now = vec![s(
        "delta",
        format!(
            "En ce moment, delta de volume de la bougie en cours : {}{}.",
            if t.d > 0.0 { "+" } else { "" },
            num(t.d, 2)
        ),
    )];
    if p.on("showTable") {
        let shown = t.lines.iter().take(p.get("nl").round().max(0.0) as usize).count();
        match t.lines.first() {
            Some(l) => now.push(s(
                "last_line",
                format!(
                    "Ruban : {shown} {} ; la plus récente à {}, volume {}, prix {}.",
                    if shown > 1 { "lignes affichées" } else { "ligne affichée" },
                    price(l.px),
                    num(l.v, 2),
                    match l.mv {
                        1 => "en hausse",
                        -1 => "en baisse",
                        _ => "inchangé",
                    }
                ),
            )),
            None => now.push(s("no_line", "Ruban : aucune ligne encore (il se remplit en direct).".into())),
        }
    }
    let last_closed = b.last().map_or(i64::MIN, |x| x.time);
    let marks = t.marks(vals);
    if let Some(m) = marks.iter().max_by_key(|m| m.time) {
        let when = if m.time > last_closed {
            Some("sur la bougie en cours".to_string())
        } else {
            b.binary_search_by_key(&m.time, |x| x.time).ok().filter(|&i| n - 1 - i <= RECENT).map(|i| {
                if i + 1 == n {
                    "sur la dernière bougie fermée".to_string()
                } else {
                    format!("il y a {}", n_bars(n - 1 - i))
                }
            })
        };
        if let Some(w) = when {
            now.push(s(
                "mark",
                format!("Dernière marque « {} » (delta {}) {w}.", m.text, if m.up { "acheteur" } else { "vendeur" }),
            ));
        }
    }
    let change = vec![s(
        "change",
        "Le delta change à chaque mise à jour : une poussée dans l'autre sens changerait la lecture.".into(),
    )];
    let lean = if t.d > 0.0 {
        Lean::Up
    } else if t.d < 0.0 {
        Lean::Down
    } else {
        Lean::Neutral
    };
    (
        vec![block(NOW, now), block(HOW, how), block(CHANGE, change)],
        Some(Sense {
            lean,
            note: Some("en direct"),
            value: None,
        }),
    )
}

pub(super) fn read_book(b: &[Bar], data: &ScreenData) -> Blocks {
    let how = vec![s(
        "how_walls",
        "Un mur est un niveau du carnet d'ordres où la taille posée est bien plus grande que la moyenne : au-dessus du prix, côté vendeurs (étiquette « SELL ») ; en dessous, côté acheteurs (« BUY »). Ce sont des noms d'étiquettes, pas des consignes ; un mur peut être retiré à tout moment.".into(),
    )];
    let Some(walls) = data.walls else {
        return missing("le carnet d'ordres (ses murs)", how);
    };
    let Some(c) = b.last().map(|x| x.close) else {
        return super::too_few("la heatmap du carnet", 1, how);
    };
    let biggest = |sell: bool| walls.iter().filter(|w| w.sell == sell).max_by(|x, y| x.usd.total_cmp(&y.usd));
    let mut now = vec![];
    for (sell, side) in [(true, "au-dessus, étiquette « SELL »"), (false, "en dessous, étiquette « BUY »")] {
        match biggest(sell) {
            Some(w) => now.push(s(
                if sell { "wall_sell" } else { "wall_buy" },
                format!(
                    "En ce moment, plus gros mur {side} : {} à {} ({} de la clôture).",
                    usd(w.usd),
                    price(w.price),
                    percent((w.price - c) / c * 100.0)
                ),
            )),
            None => now.push(s(
                if sell { "wall_sell" } else { "wall_buy" },
                format!("En ce moment, aucun mur {}.", if sell { "au-dessus du prix" } else { "en dessous du prix" }),
            )),
        }
    }
    let change = vec![s(
        "change",
        "Un mur retiré, ou un nouveau mur plus proche du prix, changerait la lecture.".into(),
    )];
    (
        vec![block(NOW, now), block(HOW, how), block(CHANGE, change)],
        Some(Sense {
            lean: Lean::NoDirection,
            note: Some("carnet en direct"),
            value: None,
        }),
    )
}

pub(super) fn read_liq(b: &[Bar], data: &ScreenData) -> Blocks {
    let how = vec![s(
        "how_liq",
        "C'est une estimation d'Argos, pas des données de liquidation réelles : à partir de l'intérêt ouvert public, Argos place les prix où des positions à effet de levier seraient liquidées. Sous le prix : des longs ; au-dessus : des shorts. Le prix est souvent attiré par ces amas, sans que ce soit sûr.".into(),
    )];
    let Some(l) = data.liq else {
        return missing("la carte des liquidations", how);
    };
    let Some(c) = b.last().map(|x| x.close) else {
        return super::too_few("la carte des liquidations", 1, how);
    };
    let above = l.profile.iter().filter(|x| x.0 > c).max_by(|x, y| x.2.total_cmp(&y.2));
    let below = l.profile.iter().filter(|x| x.0 < c).max_by(|x, y| x.1.total_cmp(&y.1));
    let mut now = vec![];
    match above {
        Some(x) if x.2 > 0.0 => now.push(s(
            "above",
            format!(
                "Estimation, en ce moment : plus gros amas de shorts au-dessus à {} ({}), environ {}.",
                price(x.0),
                percent((x.0 - c) / c * 100.0),
                usd(x.2)
            ),
        )),
        _ => now.push(s("above", "Estimation, en ce moment : aucun amas de shorts au-dessus du prix.".into())),
    }
    match below {
        Some(x) if x.1 > 0.0 => now.push(s(
            "below",
            format!(
                "Estimation, en ce moment : plus gros amas de longs en dessous à {} ({}), environ {}.",
                price(x.0),
                percent((x.0 - c) / c * 100.0),
                usd(x.1)
            ),
        )),
        _ => now.push(s("below", "Estimation, en ce moment : aucun amas de longs en dessous du prix.".into())),
    }
    let (longs, shorts): (f64, f64) = (
        l.profile.iter().filter(|x| x.0 < c).map(|x| x.1).sum(),
        l.profile.iter().filter(|x| x.0 > c).map(|x| x.2).sum(),
    );
    now.push(s(
        "totals",
        format!("En tout : environ {} de longs en dessous et {} de shorts au-dessus.", usd(longs), usd(shorts)),
    ));
    for note in l.notes {
        now.push(s("note", format!("Message de la carte : {note}")));
    }
    let change = vec![s(
        "change",
        "Un amas atteint par le prix (des positions liquidées), ou de nouvelles positions, changerait la lecture.".into(),
    )];
    (
        vec![block(NOW, now), block(HOW, how), block(CHANGE, change)],
        Some(Sense {
            lean: Lean::NoDirection,
            note: Some("estimation d'Argos"),
            value: None,
        }),
    )
}
