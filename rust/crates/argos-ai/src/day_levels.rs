//! Idea 5: the day's levels, drawn. The previous UTC day's high and low and the day's VWAP are taken from the very
//! snapshot of the Argos Lens card (`argos_lens::photos`, the same functions as the card and as the Vwap indicator:
//! no second computation), cited in sentences, and drawn as three `Command::Level` that `explain::check` accepts
//! (their prices are the snapshot's context levels).

use crate::format::{num, price};
use crate::together::Overview;
use argos_core::Bar;
use argos_lens::explain::{Block, Command, Explanation, Sentence, Tone, MAX_DRAWINGS, REMINDER};
use argos_lens::{Config, Decimal, Snapshot};

/// One UTC day, in seconds.
const DAY: i64 = 86_400;

/// The day's levels: the text (the same blocks and reminder as `read_together`), the explanation the screen draws
/// with `argos_render::explication::draw` after `explain::check`, and the Lens snapshot `check` needs (`None` when
/// Argos Lens refuses the bars: nothing is drawn then).
#[derive(Debug, Clone, PartialEq)]
pub struct DayLevels {
    pub overview: Overview,
    pub explanation: Explanation,
    pub snapshot: Option<Snapshot>,
}

fn s(id: &'static str, text: String) -> Sentence {
    Sentence { id, text }
}

fn f(d: Decimal) -> f64 {
    d.try_into().unwrap_or(f64::NAN)
}

/// Where the close stands against a level: « au-dessus (+0,42 %) ».
fn side(c: f64, l: f64) -> String {
    let pct = (c - l) / l * 100.0;
    let p = if pct > 0.0 {
        format!("+{} %", num(pct, 2))
    } else {
        format!("{} %", num(pct, 2))
    };
    if c > l {
        format!("au-dessus ({p})")
    } else if c < l {
        format!("en dessous ({p})")
    } else {
        "au niveau".to_string()
    }
}

/// The smallest positive gap between two bars, in seconds (the bar length).
fn bar_secs(b: &[Bar]) -> Option<i64> {
    b.windows(2).map(|w| w[1].time - w[0].time).filter(|d| *d > 0).min()
}

pub fn read_day_levels(closed_bars: &[Bar]) -> DayLevels {
    let b = closed_bars;
    let how = Block {
        title: "Comment le lire",
        sentences: vec![
            s(
                "how_day",
                "Le jour est compté en UTC (de minuit à minuit UTC), comme dans la carte Argos Lens et le VWAP : le haut et le bas de la veille sont ceux du jour UTC précédent ; le VWAP repart de zéro à minuit UTC.".into(),
            ),
            s(
                "how_levels",
                "Ces niveaux sont souvent regardés par les traders : le prix y réagit parfois, sans que ce soit sûr. Au-dessus du VWAP du jour, ceux qui ont acheté aujourd'hui sont en gain en moyenne.".into(),
            ),
        ],
    };
    let none = |text: String| DayLevels {
        overview: Overview {
            blocks: vec![
                Block {
                    title: "Ce que montrent les niveaux du jour",
                    sentences: vec![s("none", text)],
                },
                how.clone(),
            ],
            reminder: REMINDER,
        },
        explanation: Explanation {
            blocks: vec![],
            reminder: REMINDER,
            drawings: vec![],
        },
        snapshot: None,
    };
    let Some(secs) = bar_secs(b) else {
        return none("Pas assez de bougies pour lire les niveaux du jour.".into());
    };
    if secs >= DAY {
        return none("Bougies d'un jour ou plus : le haut et le bas de la veille n'ont pas de sens dans cette unité de temps, rien n'est dessiné.".into());
    }
    let Some(snap) = argos_lens::photos(b, secs, &Config::default(), None).ok().and_then(|v| v.last().cloned()) else {
        return none("Argos Lens ne peut pas lire ces bougies : pas de niveaux du jour.".into());
    };
    let n = b.len();
    let c = b[n - 1].close;
    let today = b[n - 1].time.div_euclid(DAY);
    let day: Vec<&Bar> = b.iter().filter(|x| x.time.div_euclid(DAY) == today).collect();
    let (hi, lo) = (
        day.iter().map(|x| x.high).fold(f64::NEG_INFINITY, f64::max),
        day.iter().map(|x| x.low).fold(f64::INFINITY, f64::min),
    );
    let mut now = vec![];
    let mut drawings = vec![];
    let mut dist: Vec<(f64, &str)> = vec![];
    match (snap.prev_day_high, snap.prev_day_low) {
        (Some(h), Some(l)) => {
            let (hf, lf) = (f(h), f(l));
            now.push(s(
                "pdh",
                format!(
                    "Haut de la veille à {} : la clôture est {} ; aujourd'hui, le prix {}.",
                    price(hf),
                    side(c, hf),
                    if hi > hf {
                        "l'a déjà dépassé"
                    } else if hi == hf {
                        "l'a touché"
                    } else {
                        "ne l'a pas encore atteint"
                    }
                ),
            ));
            now.push(s(
                "pdl",
                format!(
                    "Bas de la veille à {} : la clôture est {} ; aujourd'hui, le prix {}.",
                    price(lf),
                    side(c, lf),
                    if lo < lf {
                        "est déjà passé dessous"
                    } else if lo == lf {
                        "l'a touché"
                    } else {
                        "ne l'a pas encore atteint"
                    }
                ),
            ));
            dist.extend([((c - hf).abs() / hf, "le haut de la veille"), ((c - lf).abs() / lf, "le bas de la veille")]);
            drawings.push(Command::Level {
                price: h,
                label: format!("Haut veille {}", price(hf)),
                tone: Tone::Context,
                sentence: "pdh",
            });
            drawings.push(Command::Level {
                price: l,
                label: format!("Bas veille {}", price(lf)),
                tone: Tone::Context,
                sentence: "pdl",
            });
        }
        _ => now.push(s(
            "no_prev_day",
            "La veille n'est pas complète dans les bougies chargées : pas de haut ni de bas de la veille.".into(),
        )),
    }
    match snap.vwap {
        Some(v) => {
            let vf = f(v);
            now.push(s("vwap", format!("VWAP du jour à {} : la clôture est {}.", price(vf), side(c, vf))));
            dist.push(((c - vf).abs() / vf, "le VWAP du jour"));
            drawings.push(Command::Level {
                price: v,
                label: format!("VWAP {}", price(vf)),
                tone: Tone::Context,
                sentence: "vwap",
            });
        }
        None => now.push(s("no_vwap", "Pas de volume aujourd'hui dans ces bougies : pas de VWAP du jour.".into())),
    }
    if let Some((_, name)) = dist.iter().filter(|x| x.0.is_finite()).min_by(|x, y| x.0.total_cmp(&y.0)) {
        now.push(s("nearest", format!("Le niveau le plus proche de la clôture est {name}.")));
    }
    let change = Block {
        title: "Ce qui changerait la lecture",
        sentences: vec![s(
            "change",
            "Une clôture de l'autre côté du niveau le plus proche, ou le passage à minuit UTC (de nouveaux niveaux de veille, un VWAP qui repart), changerait la lecture.".into(),
        )],
    };
    let blocks = vec![
        Block {
            title: "Ce que montrent les niveaux du jour",
            sentences: now,
        },
        how,
        change,
    ];
    drawings.truncate(MAX_DRAWINGS);
    DayLevels {
        overview: Overview {
            blocks: blocks.clone(),
            reminder: REMINDER,
        },
        explanation: Explanation {
            blocks,
            reminder: REMINDER,
            drawings,
        },
        snapshot: Some(snap),
    }
}
