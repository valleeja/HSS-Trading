//! 🤖 Idea 4: « Volatilité du moment ». Three measures of how much the market moves, with fixed lengths so that the
//! reading is the same everywhere: the ATR in % of the price and in percentile (the very number of Argos Lens), the
//! width of the Bollinger bands and its rank, and the relative volume. A summary among « calme inhabituel »,
//! « agitation normale » and « agitation forte », or the plain fact that the measures disagree. Never a direction
//! of the price, never an order.

use crate::format::num;
use crate::together::{list, plural};
use crate::{Block, Overview, Sentence, REMINDER};
use argos_core::Bar;
use argos_script::indicators::classic;

/// ATR length (as Argos Lens).
pub(crate) const ATR_LEN: usize = 14;
/// Bollinger bands: length and number of standard deviations.
pub(crate) const BB_LEN: usize = 20;
pub(crate) const BB_MULT: f64 = 2.0;
/// Bars over which the width of the bands is ranked.
pub(crate) const BB_WINDOW: usize = 120;
/// Relative volume: length of the average, and the recent bars looked at.
pub(crate) const VOL_LEN: usize = 20;
pub(crate) const VOL_RECENT: usize = 5;
/// Calm: ATR percentile at most this, bands width rank at most `CALM_RANK`, recent volume under `CALM_VOLUME`.
pub(crate) const CALM_PCT: f64 = 20.0;
pub(crate) const CALM_RANK: f64 = 10.0;
pub(crate) const CALM_VOLUME: f64 = 0.8;
/// Strong: ATR percentile at least this, bands width rank at least `STRONG_RANK`, last volume at least
/// `STRONG_VOLUME` (or the recent average at least `STRONG_RECENT`).
pub(crate) const STRONG_PCT: f64 = 80.0;
pub(crate) const STRONG_RANK: f64 = 90.0;
pub(crate) const STRONG_VOLUME: f64 = 2.0;
pub(crate) const STRONG_RECENT: f64 = 1.5;

/// What one measure says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Says {
    Calm,
    Normal,
    Strong,
}

fn s(id: &'static str, text: String) -> Sentence {
    Sentence { id, text }
}

/// The length of a bar, from the bars themselves (the smallest gap between two bars), for Argos Lens.
fn bar_secs(bars: &[Bar]) -> Option<i64> {
    bars.windows(2).map(|w| w[1].time - w[0].time).filter(|d| *d > 0).min()
}

/// The ATR percentile of Argos Lens on the last bar (its window of 100 bars), the same number as the Lens card.
fn lens_atr_percentile(bars: &[Bar]) -> Option<f64> {
    let config = argos_lens::Config {
        htf: None,
        ..argos_lens::Config::default()
    };
    let photos = argos_lens::photos(bars, bar_secs(bars)?, &config, None).ok()?;
    f64::try_from(photos.last()?.atr_pct?).ok()
}

/// The share (0 to 100) of the last `window` values of `v` that are at most the last one; `None` without a full window.
fn rank(v: &[f64], window: usize) -> Option<f64> {
    let w = &v[v.len().checked_sub(window)?..];
    let last = *w.last()?;
    w.iter()
        .all(|x| x.is_finite())
        .then(|| 100.0 * w.iter().filter(|&&x| x <= last).count() as f64 / window as f64)
}

/// The summary from what each measure says: « calme inhabituel » when at least two say calm and none says agitated,
/// « agitation forte » when at least two say agitated and none says calm, the disagreement when some say calm and
/// others agitated, « agitation normale » otherwise (none out of the ordinary, or only one, which is named).
pub(crate) fn summarize(says: &[(&str, Says)]) -> String {
    let names = |w: Says| -> Vec<String> { says.iter().filter(|x| x.1 == w).map(|x| x.0.to_string()).collect() };
    let (calm, strong) = (names(Says::Calm), names(Says::Strong));
    if says.is_empty() {
        "Pas assez de bougies pour lire la volatilité du moment.".to_string()
    } else if !calm.is_empty() && !strong.is_empty() {
        format!(
            "Les mesures ne sont pas d'accord : {} {} un marché calme, {} un marché agité.",
            list(&calm),
            plural(calm.len(), "montre", "montrent"),
            list(&strong)
        )
    } else if calm.len() >= 2 {
        format!(
            "Calme inhabituel : {} {} un marché plus calme que d'habitude.",
            list(&calm),
            plural(calm.len(), "montre", "montrent")
        )
    } else if strong.len() >= 2 {
        format!(
            "Agitation forte : {} {} un marché plus agité que d'habitude.",
            list(&strong),
            plural(strong.len(), "montre", "montrent")
        )
    } else {
        match (calm.first(), strong.first()) {
            (Some(n), _) => format!("Agitation normale : seule la mesure {n} sort de l'habitude et montre un marché calme."),
            (_, Some(n)) => format!("Agitation normale : seule la mesure {n} sort de l'habitude et montre un marché agité."),
            _ => "Agitation normale : aucune mesure ne sort nettement de l'habitude.".to_string(),
        }
    }
}

/// Reads the volatility of the moment on `closed_bars` (closed bars only, oldest first).
pub fn read_volatility(closed_bars: &[Bar]) -> Overview {
    let b = closed_bars;
    let mut now = vec![];
    let mut says: Vec<(&str, Says)> = vec![];
    let mut missing: Vec<String> = vec![];
    let close = b.last().map(|x| x.close);

    // ATR: in % of the price, and its percentile (Argos Lens)
    let atr = classic::atr_n(b, ATR_LEN);
    match (atr.last().copied().filter(|x| x.is_finite()), close, lens_atr_percentile(b)) {
        (Some(a), Some(c), Some(p)) => {
            now.push(s(
                "atr",
                format!(
                    "ATR {ATR_LEN} : {} du prix, au centile {} des 100 dernières bougies (comme la carte Argos Lens).",
                    num(a / c * 100.0, 2) + " %",
                    num(p, 0)
                ),
            ));
            says.push((
                "ATR",
                if p <= CALM_PCT {
                    Says::Calm
                } else if p >= STRONG_PCT {
                    Says::Strong
                } else {
                    Says::Normal
                },
            ));
        }
        _ => missing.push(format!("l'ATR {ATR_LEN} en centile (il faut au moins {} bougies)", 100 + ATR_LEN)),
    }

    // Bollinger: width in % of the price, and its rank over BB_WINDOW bars
    let bb = classic::bollinger_n(b, BB_LEN, BB_MULT);
    let width: Vec<f64> = bb.upper.iter().zip(&bb.lower).zip(&bb.basis).map(|((u, l), m)| (u - l) / m * 100.0).collect();
    match (width.last().copied().filter(|x| x.is_finite()), rank(&width, BB_WINDOW)) {
        (Some(w), Some(r)) => {
            let narrowest = width[width.len() - BB_WINDOW..].iter().all(|&x| x >= w);
            let place = if narrowest {
                format!("au plus étroit des {BB_WINDOW} dernières bougies")
            } else if r <= CALM_RANK {
                format!("dans les {} % les plus étroites des {BB_WINDOW} dernières bougies", num(CALM_RANK, 0))
            } else if r >= STRONG_RANK {
                format!("dans les {} % les plus larges des {BB_WINDOW} dernières bougies", num(100.0 - STRONG_RANK, 0))
            } else {
                format!("plus large que {} % des {BB_WINDOW} dernières bougies", num(r, 0))
            };
            now.push(s(
                "bands",
                format!(
                    "Bandes de Bollinger {BB_LEN}/{} : largeur {} du prix, {place}.",
                    num(BB_MULT, 0),
                    num(w, 2) + " %"
                ),
            ));
            says.push((
                "Bollinger",
                if r <= CALM_RANK {
                    Says::Calm
                } else if r >= STRONG_RANK {
                    Says::Strong
                } else {
                    Says::Normal
                },
            ));
        }
        _ => missing.push(format!("le rang de la largeur des bandes (il faut au moins {} bougies)", BB_WINDOW + BB_LEN)),
    }

    // relative volume: the last bar and the average of the last VOL_RECENT
    let rv = classic::relative_volume_n(b, VOL_LEN);
    let recent: Vec<f64> = rv.iter().rev().take(VOL_RECENT).copied().collect();
    match rv.last().copied().filter(|x| x.is_finite()) {
        Some(v) if recent.len() == VOL_RECENT && recent.iter().all(|x| x.is_finite()) => {
            let avg = recent.iter().sum::<f64>() / VOL_RECENT as f64;
            now.push(s(
                "volume",
                format!(
                    "Volume de la dernière bougie : {} fois la moyenne des {VOL_LEN} dernières ; {} en moyenne sur les {VOL_RECENT} dernières.",
                    num(v, 2),
                    num(avg, 2)
                ),
            ));
            says.push((
                "volume",
                if v >= STRONG_VOLUME || avg >= STRONG_RECENT {
                    Says::Strong
                } else if avg < CALM_VOLUME {
                    Says::Calm
                } else {
                    Says::Normal
                },
            ));
        }
        _ => missing.push(format!("le volume relatif (il faut au moins {} bougies)", VOL_LEN + VOL_RECENT)),
    }

    if !missing.is_empty() {
        now.push(s("too_few", format!("Pas assez de bougies clôturées pour {}.", list(&missing))));
    }
    let summary = summarize(&says);
    let mut blocks = vec![Block {
        title: "Volatilité du moment",
        sentences: [vec![s("summary", summary)], now].concat(),
    }];
    blocks.push(Block {
        title: "Comment le lire",
        sentences: vec![
            s("how_calm", "Un calme inhabituel ne dit ni dans quel sens le prix partira, ni quand.".into()),
            s(
                "how_strong",
                "Une agitation forte élargit les écarts : les bougies sont plus longues et le prix va plus loin d'une bougie à l'autre.".into(),
            ),
            s(
                "how_measures",
                format!(
                    "Longueurs fixes, les mêmes partout : ATR {ATR_LEN}, bandes {BB_LEN}/{}, volume sur {VOL_LEN} bougies.",
                    num(BB_MULT, 0)
                ),
            ),
        ],
    });
    Overview { blocks, reminder: REMINDER }
}
