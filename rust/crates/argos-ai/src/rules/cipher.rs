//! The reading of Argos Cipher Pro: the very points the chart draws, from `argos_script::indicators::cipher::compute`
//! with the same settings (no second computation). A point hidden by its « Show … » setting is never mentioned.
//! Names as the app and the Pine call them: petit point vert / rouge (the waves cross), gros point vert / rouge (they
//! cross in the oversold / overbought zone), point doré (Golden dot), point violet (Purple dot), points de divergence.

use super::{block, s, valued, Blocks, CHANGE, HOW, NOW};
use crate::format::{bars as n_bars, num};
use crate::{Lean, Sense};
use argos_core::Bar;
use argos_script::indicators::cipher::{self, Cipher};
use argos_script::inputs::Inputs;

/// Points older than this, in bars, are not « recent ».
pub(super) const RECENT: usize = 50;
/// A divergence confirmed more than this many bars ago is not « en cours ».
pub(super) const DIV_RECENT: usize = 20;

pub(super) const ROLES: [(&str, &str); 15] = [
    ("wt_show", "Affiche les vagues WaveTrend et leurs petits points de croisement."),
    ("os1", "Niveau de survente 1 : sous lui, un croisement vers le haut donne un gros point vert."),
    ("ob1", "Niveau de surachat 1 : au-dessus, un croisement vers le bas donne un gros point rouge."),
    ("buy_show", "Affiche les gros points verts (croisement vers le haut en zone de survente)."),
    ("sell_show", "Affiche les gros points rouges (croisement vers le bas en zone de surachat)."),
    ("gold_show", "Affiche les points dorés (Golden dot)."),
    ("os3", "Niveau de survente 3 : le creux d'avant d'un point doré doit être sous lui."),
    ("rsi_os", "RSI de survente : le RSI au creux d'avant d'un point doré doit être sous lui."),
    ("purple_show", "Affiche les points violets (Purple dot, le miroir du point doré)."),
    ("rsi_ob", "RSI de surachat, utilisé par le point violet."),
    ("div_dots", "Affiche les points de divergence."),
    ("mfi_show", "Affiche la zone RSI + MFI (le flux d'argent du Cipher)."),
    ("ch", "Longueur du canal du WaveTrend."),
    ("avg", "Longueur de la moyenne du WaveTrend."),
    ("ma", "Longueur de la moyenne qui donne WT2."),
];

/// One kind of point the chart draws: its name, its bars, and whether it is shown.
struct Kind<'a> {
    id: &'static str,
    name: &'static str,
    bars: &'a [usize],
    shown: bool,
}

/// The kinds of points of `c`, with the settings that show them (`p`), the most telling first.
fn kinds<'a>(c: &'a Cipher, p: &Inputs) -> [Kind<'a>; 8] {
    let wt = p.on("wt_show");
    [
        Kind {
            id: "gold",
            name: "point doré (Golden dot)",
            bars: &c.gold_dots,
            shown: p.on("gold_show"),
        },
        Kind {
            id: "purple",
            name: "point violet (Purple dot)",
            bars: &c.purple,
            shown: p.on("purple_show"),
        },
        Kind {
            id: "buy",
            name: "gros point vert",
            bars: &c.buy,
            shown: p.on("buy_show") || wt,
        },
        Kind {
            id: "sell",
            name: "gros point rouge",
            bars: &c.sell,
            shown: p.on("sell_show") || wt,
        },
        Kind {
            id: "div_bull",
            name: "point de divergence vert",
            bars: &c.div_bull_dots,
            shown: p.on("div_dots"),
        },
        Kind {
            id: "div_bear",
            name: "point de divergence rouge",
            bars: &c.div_bear_dots,
            shown: p.on("div_dots"),
        },
        Kind {
            id: "small_up",
            name: "petit point vert",
            bars: &c.cross_up,
            shown: wt,
        },
        Kind {
            id: "small_down",
            name: "petit point rouge",
            bars: &c.cross_down,
            shown: wt,
        },
    ]
}

/// The last shown point: its kind and its bar (on the same bar, the most telling kind wins).
fn last_point<'a>(ks: &'a [Kind<'a>]) -> Option<(&'a Kind<'a>, usize)> {
    ks.iter()
        .filter(|k| k.shown)
        .filter_map(|k| k.bars.last().map(|&i| (k, i)))
        .fold(None, |best: Option<(&Kind, usize)>, (k, i)| match best {
            Some((_, j)) if j >= i => best,
            _ => Some((k, i)),
        })
}

/// The series where the divergence at `pivot` was found (RSI, Stoch RSI, MFI, else WaveTrend).
fn div_source(c: &Cipher, pivot: usize, bull: bool) -> &'static str {
    let has = |d: &cipher::Divs| {
        if bull {
            d.bull.iter().any(|x| x.pivot == pivot)
        } else {
            d.bear.iter().any(|x| x.pivot == pivot)
        }
    };
    if has(&c.rsi_divs) {
        "RSI"
    } else if has(&c.stoch_divs) {
        "Stoch RSI"
    } else {
        "WaveTrend"
    }
}

pub(super) fn read(b: &[Bar], vals: &[f64]) -> Blocks {
    let p = Inputs { params: &cipher::PARAMS, vals };
    let c = cipher::compute(b, &cipher::Settings::from_inputs(vals));
    let n = b.len();
    let (os1, ob1, os3) = (p.get("os1"), p.get("ob1"), p.get("os3"));
    let ks = kinds(&c, &p);
    let mut how = vec![];
    for k in ks.iter().filter(|k| k.shown) {
        // a pair of kinds (green and red) shares its sentence
        let id = match k.id {
            "sell" => "buy",
            "div_bear" => "div_bull",
            "small_down" => "small_up",
            x => x,
        };
        if how.iter().any(|x: &argos_lens::explain::Sentence| x.id == id) {
            continue;
        }
        let text = match id {
            "small_up" => "Petit point vert ou rouge : la vague rapide WT1 croise la vague lente WT2, vers le haut (vert) ou vers le bas (rouge). Plus le point est gros sur la vague, plus le WaveTrend est loin de zéro.".to_string(),
            "buy" => format!(
                "Gros point vert : ce croisement vers le haut se fait sous le niveau de survente 1 ({}) ; gros point rouge : vers le bas au-dessus du surachat 1 ({}).",
                num(os1, 0),
                num(ob1, 0)
            ),
            "gold" => format!(
                "Point doré : une divergence haussière du WaveTrend ou du RSI est confirmée, alors que le creux d'avant du WaveTrend était sous {} avec un RSI sous {}, et que le WaveTrend est remonté d'au moins 5 points. Il est posé sur le creux, 2 bougies avant cette confirmation.",
                num(os3, 0),
                num(p.get("rsi_os"), 0)
            ),
            "purple" => format!(
                "Point violet : le miroir à la baisse ; une divergence baissière du WaveTrend avec un RSI au-dessus de {}, ou un gros point rouge avec un RSI haut ou une divergence baissière à 3 bougies près.",
                num(p.get("rsi_ob"), 0)
            ),
            "div_bull" => "Points de divergence (vert en bas, rouge en haut) : posés sur le creux ou le sommet de la divergence, qui n'est confirmée que 2 bougies plus tard.".to_string(),
            _ => continue,
        };
        how.push(s(id, text));
    }
    let wt2 = c.wt2.last().copied().filter(|x| x.is_finite());
    let wt1 = c.wt1.last().copied().filter(|x| x.is_finite());
    let (Some(w2), Some(w1)) = (wt2, wt1) else {
        let needed = p.len("ch") + p.len("avg") + p.len("ma");
        return super::too_few("le Cipher", needed, how);
    };
    let mut now = vec![];
    if p.on("wt_show") {
        let zone = if w2 <= os1 {
            format!("sous le niveau de survente 1 ({})", num(os1, 0))
        } else if w2 >= ob1 {
            format!("au-dessus du niveau de surachat 1 ({})", num(ob1, 0))
        } else {
            format!("entre la survente ({}) et le surachat ({})", num(os1, 0), num(ob1, 0))
        };
        now.push(s(
            "wave",
            format!(
                "WaveTrend : WT2 à {}, {zone} ; la vague rapide WT1 est {} de WT2.",
                num(w2, 1),
                if w1 > w2 { "au-dessus" } else { "en dessous" }
            ),
        ));
    }
    let last = last_point(&ks).filter(|(_, i)| n - 1 - i <= RECENT);
    match last {
        Some((k, i)) => now.push(s(
            "last_point",
            format!(
                "Dernier point : {} {}, WT2 à {} sur cette bougie.",
                k.name,
                if n - 1 == i {
                    "sur la dernière bougie".to_string()
                } else {
                    format!("il y a {}", n_bars(n - 1 - i))
                },
                num(c.wt2[i], 1)
            ),
        )),
        None => now.push(s("no_point", format!("Aucun point affiché dans les {RECENT} dernières bougies."))),
    }
    if p.on("div_dots") {
        let latest = [(true, &c.div_bull_dots), (false, &c.div_bear_dots)]
            .into_iter()
            .filter_map(|(bull, l)| l.last().map(|&pv| (bull, pv)))
            .filter(|(_, pv)| pv + 2 < n && n - 1 - (pv + 2) <= DIV_RECENT)
            .max_by_key(|(_, pv)| *pv);
        if let Some((bull, pv)) = latest {
            let ago = n - 1 - (pv + 2);
            now.push(s(
                "divergence",
                format!(
                    "Divergence {} du {} : point posé il y a {}, confirmée {}.",
                    if bull { "haussière" } else { "baissière" },
                    div_source(&c, pv, bull),
                    n_bars(n - 1 - pv),
                    if ago == 0 {
                        "sur la dernière bougie".to_string()
                    } else {
                        format!("il y a {}", n_bars(ago))
                    }
                ),
            ));
        }
    }
    if p.on("mfi_show") {
        if let Some(m) = c.mfi.last().copied().filter(|x| x.is_finite()) {
            now.push(s(
                "mfi",
                format!(
                    "Flux d'argent du Cipher (zone RSI + MFI) à {} : {}.",
                    num(m, 1),
                    if m > 0.0 {
                        "positif, les bougies récentes ferment plutôt haut"
                    } else {
                        "négatif, les bougies récentes ferment plutôt bas"
                    }
                ),
            ));
        }
    }
    let up = w1 > w2;
    let named = if !p.on("wt_show") {
        String::new()
    } else if up {
        " (petit point rouge)".to_string()
    } else {
        " (petit point vert)".to_string()
    };
    let change = vec![s(
        "change_cross",
        format!(
            "Un croisement de WT1 {} WT2{named} changerait la lecture de la vague.",
            if up { "sous" } else { "au-dessus de" }
        ),
    )];
    let note: Option<&'static str> = last.filter(|(_, i)| n - 1 - i <= 5).map(|(k, _)| match k.id {
        "gold" => "point doré récent",
        "purple" => "point violet récent",
        "buy" => "gros point vert récent",
        "sell" => "gros point rouge récent",
        "div_bull" => "divergence haussière récente",
        "div_bear" => "divergence baissière récente",
        "small_up" => "petit point vert récent",
        _ => "petit point rouge récent",
    });
    let lean = if up { Lean::Up } else { Lean::Down };
    let sense = valued(Some(Sense { lean, note, value: None }), format!("WT2 {}", num(w2, 1)));
    (vec![block(NOW, now), block(HOW, how), block(CHANGE, change)], sense)
}

/// The shown kinds of points (name, bars), most telling first, for the tests.
#[cfg(test)]
pub(crate) fn shown_points(c: &Cipher, vals: &[f64]) -> Vec<(&'static str, Vec<usize>)> {
    let p = Inputs { params: &cipher::PARAMS, vals };
    kinds(c, &p).iter().filter(|k| k.shown).map(|k| (k.name, k.bars.to_vec())).collect()
}
