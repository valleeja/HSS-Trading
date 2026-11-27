//! The readings of « Argos OB · BB · FVG · iFVG » (`smc`) and « Argos SMC » (`smc_pro`): the very zones and structure
//! breaks the chart draws, from `argos_script::indicators::smc::compute` and `smc_pro::compute` with the same settings
//! (no second computation), and only what argos-render's zones.rs (`draw_smc`, `draw_smc_pro`) shows with these
//! settings. At most 3 zones are named, the nearest to the close. The trade plan of « Argos SMC » is not drawn on the
//! chart and is not read.

use super::ai_trend::bar_secs;
use super::{block, s, valued, Blocks, CHANGE, HOW, NOW};
use crate::format::{bars as n_bars, num, price};
use crate::{Lean, Sense};
use argos_core::Bar;
use argos_lens::explain::Sentence;
use argos_script::indicators::{smc, smc_pro};
use argos_script::inputs::Inputs;

/// At most this many zones are named.
pub(super) const MAX_ZONES: usize = 3;

pub(super) const SMC_ROLES: [(&str, &str); 10] = [
    ("show_ob", "Affiche les order blocks (OB+ / OB−)."),
    ("show_bb", "Affiche les breaker blocks (BB+ / BB−), des OB cassés qui ont changé de camp."),
    ("ob_n", "Nombre d'OB actifs affichés par côté."),
    ("show_fvg", "Affiche les FVG (FVG+ / FVG−)."),
    ("show_ifvg", "Affiche les iFVG, des FVG traversés en clôture."),
    ("fvg_n", "Nombre de FVG actifs affichés par côté."),
    ("show_bos", "Affiche les BOS (cassures de structure dans le sens de la tendance)."),
    ("show_choch", "Affiche les CHoCH (cassures contre la tendance)."),
    ("st_n", "Nombre de cassures affichées."),
    ("show_eq", "Affiche les sommets et creux égaux (EQH / EQL)."),
];
pub(super) const PRO_ROLES: [(&str, &str); 8] = [
    ("showOB", "Affiche les order blocks."),
    ("showBB", "Affiche les breaker blocks."),
    ("showFVG", "Affiche les FVG."),
    ("showIFVG", "Affiche les iFVG."),
    ("showBos", "Affiche les BOS et CHoCH."),
    ("showPD", "Affiche les zones Premium / Discount et la ligne d'équilibre."),
    ("pdLen", "Nombre de bougies du range Premium / Discount."),
    ("showOte", "Affiche la zone OTE du Fibonacci (62 – 79 %)."),
];

/// A zone as the chart labels it: (label, bottom, top, start time).
type Shown = (&'static str, f64, f64, i64);

/// « Le prix est dedans », « … au-dessus, à 0,55 % », « … en dessous, à 1,41 % ».
pub(super) fn where_(c: f64, bot: f64, top: f64) -> String {
    let d = |x: f64| x / c * 100.0;
    if c > top && d(c - top) < 0.005 {
        "le prix est juste au-dessus".to_string()
    } else if c > top {
        format!("le prix est au-dessus, à {} %", num(d(c - top), 2))
    } else if c < bot && d(bot - c) < 0.005 {
        "le prix est juste en dessous".to_string()
    } else if c < bot {
        format!("le prix est en dessous, à {} %", num(d(bot - c), 2))
    } else {
        "le prix est dedans".to_string()
    }
}

/// « née sur la dernière bougie » or « née il y a N bougies ».
pub(super) fn born(a: usize) -> String {
    if a == 0 {
        "née sur la dernière bougie".to_string()
    } else {
        format!("née il y a {}", n_bars(a))
    }
}

fn distance(c: f64, bot: f64, top: f64) -> f64 {
    if c > top {
        c - top
    } else if c < bot {
        bot - c
    } else {
        0.0
    }
}

/// Age in chart bars of something that starts at `t`.
fn age(b: &[Bar], t: i64) -> usize {
    b.len() - b.partition_point(|x| x.time <= t).max(1)
}

/// The 3 nearest shown zones, as sentences.
fn zone_sentences(b: &[Bar], zones: &mut [Shown]) -> Vec<Sentence> {
    let c = b[b.len() - 1].close;
    zones.sort_by(|x, y| distance(c, x.1, x.2).total_cmp(&distance(c, y.1, y.2)));
    zones
        .iter()
        .take(MAX_ZONES)
        .map(|(label, bot, top, t)| {
            s(
                "zone",
                format!(
                    "Zone « {label} » de {} à {}, {} : {}.",
                    price(*bot),
                    price(*top),
                    born(age(b, *t)),
                    where_(c, *bot, *top)
                ),
            )
        })
        .collect()
}

/// The last structure break shown: « BOS haussier au niveau X, il y a N bougies ».
fn break_sentence(b: &[Bar], choch: bool, up: bool, level: f64, t: i64) -> Sentence {
    let a = age(b, t);
    s(
        "structure",
        format!(
            "Dernière cassure de structure : {} {} au niveau {}, {}.",
            if choch { "CHoCH" } else { "BOS" },
            if up { "haussier" } else { "baissier" },
            price(level),
            if a == 0 {
                "sur la dernière bougie".to_string()
            } else {
                format!("il y a {}", n_bars(a))
            }
        ),
    )
}

fn how_zones() -> Sentence {
    s(
        "how_zones",
        "OB : la dernière bougie opposée avant l'impulsion qui a cassé la structure ; BB : un OB cassé qui a changé de camp ; FVG : un trou laissé par trois bougies ; iFVG : un FVG traversé en clôture, au rôle inversé. « + » : sous le prix (soutien), « − » : au-dessus (résistance).".into(),
    )
}

fn how_structure() -> Sentence {
    s(
        "how_structure",
        "BOS : le prix casse un sommet (ou un creux) dans le sens de la tendance ; CHoCH : il casse contre la tendance, premier signe de changement de caractère.".into(),
    )
}

pub(super) fn read_smc(b: &[Bar], vals: &[f64]) -> Blocks {
    let p = Inputs { params: &smc::PARAMS, vals };
    let mut how = vec![];
    if ["show_ob", "show_bb", "show_fvg"].iter().any(|k| p.on(k)) {
        how.push(how_zones());
    }
    if p.on("show_bos") || p.on("show_choch") {
        how.push(how_structure());
    }
    let n = b.len();
    let Some(secs) = bar_secs(b) else {
        return super::too_few("Argos OB · BB · FVG · iFVG", 2, how);
    };
    let r = smc::compute(b, secs, &smc::Settings::from_inputs(vals));
    // zones and breaks are indexed in the bars they were found in (the chart's, or the higher unit's)
    let src: &[Bar] = r.htf_bars.as_deref().unwrap_or(b);
    let t_of = |i: usize| src.get(i).map_or(i64::MIN, |x| x.time);
    let mut zones: Vec<Shown> = vec![];
    // as draw_smc: per side and kind, the last « ob_n » / « fvg_n » active zones
    let (ob_n, fvg_n) = (p.len("ob_n"), p.len("fvg_n"));
    for up in [true, false] {
        for (on, brk, label) in [
            (p.on("show_ob"), false, if up { "OB+" } else { "OB−" }),
            (p.on("show_bb"), true, if up { "BB+" } else { "BB−" }),
        ] {
            if on {
                let act: Vec<_> = r.order_blocks.iter().filter(|o| o.up == up && o.breaker == brk && o.end.is_none()).collect();
                zones.extend(act[act.len().saturating_sub(ob_n)..].iter().map(|o| (label, o.bot, o.top, t_of(o.i0))));
            }
        }
        if p.on("show_fvg") {
            for (inv, label) in [(false, if up { "FVG+" } else { "FVG−" }), (true, if up { "iFVG−" } else { "iFVG+" })] {
                if inv && !p.on("show_ifvg") {
                    continue;
                }
                let act: Vec<_> = r.fvgs.iter().filter(|f| f.up == up && f.inverse == inv && f.end.is_none()).collect();
                zones.extend(act[act.len().saturating_sub(fvg_n)..].iter().map(|f| (label, f.bot, f.top, t_of(f.i0))));
            }
        }
    }
    let mut now = zone_sentences(b, &mut zones);
    if now.is_empty() && ["show_ob", "show_bb", "show_fvg"].iter().any(|k| p.on(k)) {
        now.push(s("no_zone", "Aucune zone active affichée.".into()));
    }
    let shown: Vec<&smc::StructureBreak> = r
        .breaks
        .iter()
        .filter(|x| {
            if x.kind == smc::BreakType::Choch {
                p.on("show_choch")
            } else {
                p.on("show_bos")
            }
        })
        .collect();
    let last = shown[shown.len().saturating_sub(p.len("st_n"))..].last().copied();
    if let Some(x) = last {
        now.push(break_sentence(b, x.kind == smc::BreakType::Choch, x.up, x.price, t_of(x.i1)));
    }
    if p.on("show_eq") {
        let c = b[n - 1].close;
        let open: Vec<_> = r.equal.iter().filter(|e| e.swept.is_none()).collect();
        let eqh = open.iter().filter(|e| e.up && e.price > c).min_by(|x, y| x.price.total_cmp(&y.price));
        let eql = open.iter().filter(|e| !e.up && e.price < c).max_by(|x, y| x.price.total_cmp(&y.price));
        let parts: Vec<String> = [
            eqh.map(|e| format!("EQH à {}", price(e.price))),
            eql.map(|e| format!("EQL à {}", price(e.price))),
        ]
        .into_iter()
        .flatten()
        .collect();
        if !parts.is_empty() {
            now.push(s("equal", format!("Sommets ou creux égaux encore là : {}.", parts.join(" ; "))));
        }
    }
    if now.is_empty() {
        now.push(s("nothing", "Rien d'affiché à lire avec ces réglages.".into()));
    }
    let change = vec![s(
        "change",
        "Une nouvelle cassure de structure, ou une clôture qui traverse la zone la plus proche, changerait la lecture.".into(),
    )];
    let lean = match last {
        Some(x) if x.up => Lean::Up,
        Some(_) => Lean::Down,
        None => Lean::NoDirection,
    };
    let note = last.map(|x| {
        if x.kind == smc::BreakType::Choch {
            "dernière cassure : CHoCH"
        } else {
            "dernière cassure : BOS"
        }
    });
    (
        vec![block(NOW, now), block(HOW, how), block(CHANGE, change)],
        Some(Sense { lean, note, value: None }),
    )
}

pub(super) fn read_pro(b: &[Bar], vals: &[f64]) -> Blocks {
    let p = Inputs {
        params: &smc_pro::PARAMS,
        vals,
    };
    let mut how = vec![];
    if ["showOB", "showBB", "showFVG", "showIFVG"].iter().any(|k| p.on(k)) {
        how.push(how_zones());
    }
    if p.on("showBos") {
        how.push(how_structure());
    }
    if p.on("showPD") {
        how.push(s(
            "how_pd",
            format!(
                "Premium / Discount : sur les {} dernières bougies, au-dessus de l'équilibre (le milieu du range) le prix est cher (premium), en dessous il est bon marché (discount).",
                p.len("pdLen")
            ),
        ));
    }
    if p.on("showOte") {
        how.push(s(
            "how_ote",
            "OTE : la zone de 62 à 79 % du retracement de la dernière jambe (Fibonacci).".into(),
        ));
    }
    let n = b.len();
    let Some(secs) = bar_secs(b) else {
        return super::too_few("Argos SMC", 2, how);
    };
    let r = smc_pro::compute(b, secs, &smc_pro::Settings::from_inputs(vals));
    let t_of = |i: usize| b.get(i).map_or(i64::MIN, |x| x.time);
    let z = &r.zones;
    let mut zones: Vec<Shown> = vec![];
    for (on, list, label) in [
        (p.on("showOB"), &z.bull_ob, "OB haussier"),
        (p.on("showOB"), &z.bear_ob, "OB baissier"),
        (p.on("showBB"), &z.bull_bb, "BB haussier"),
        (p.on("showBB"), &z.bear_bb, "BB baissier"),
        (p.on("showFVG"), &z.bull_fvg, "FVG haussier"),
        (p.on("showFVG"), &z.bear_fvg, "FVG baissier"),
        (p.on("showIFVG"), &z.bull_ifvg, "iFVG haussier"),
        (p.on("showIFVG"), &z.bear_ifvg, "iFVG baissier"),
    ] {
        if on {
            zones.extend(list.iter().map(|x| (label, x.bot, x.top, t_of(x.left))));
        }
    }
    let mut now = zone_sentences(b, &mut zones);
    let last = r.events.iter().filter(|_| p.on("showBos")).max_by_key(|e| e.at);
    if let Some(e) = last {
        now.push(break_sentence(b, e.choch, e.up, e.level, t_of(e.at)));
    }
    let c = b[n - 1].close;
    if p.on("showPD") && r.range_top.is_finite() && r.range_bot.is_finite() && r.range_top > r.range_bot {
        now.push(s(
            "pd",
            format!(
                "Le prix est à {} % du range ({} à {}) : {} de l'équilibre ({}).",
                num(r.pd_pct, 0),
                price(r.range_bot),
                price(r.range_top),
                if c > r.equilibrium {
                    "zone premium, au-dessus"
                } else {
                    "zone discount, en dessous"
                },
                price(r.equilibrium)
            ),
        ));
    }
    if p.on("showOte") && r.ote_top.is_finite() && r.ote_bot.is_finite() && r.ote_top > r.ote_bot {
        now.push(s(
            "ote",
            format!("Zone OTE de {} à {} : {}.", price(r.ote_bot), price(r.ote_top), where_(c, r.ote_bot, r.ote_top)),
        ));
    }
    if now.is_empty() {
        now.push(s("nothing", "Rien d'affiché à lire avec ces réglages.".into()));
    }
    let change = vec![s(
        "change",
        "Une nouvelle cassure de structure, ou une clôture qui traverse la zone la plus proche, changerait la lecture.".into(),
    )];
    let lean = match last {
        Some(e) if e.up => Lean::Up,
        Some(_) => Lean::Down,
        None => Lean::NoDirection,
    };
    let note = last.map(|e| if e.choch { "dernière cassure : CHoCH" } else { "dernière cassure : BOS" });
    let sense = Some(Sense { lean, note, value: None });
    let sense = if p.on("showPD") {
        valued(sense, format!("{} % du range", num(r.pd_pct, 0)))
    } else {
        sense
    };
    (vec![block(NOW, now), block(HOW, how), block(CHANGE, change)], sense)
}
