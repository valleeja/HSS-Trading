//! The rules of lot 1: RSI, MACD, EMA cross, multiple EMAs, VWAP. Each one computes its indicator with the Argos
//! Script function the chart uses, with the same settings table, then writes fixed sentences from the last values.

use crate::format::{bars as n_bars, num, percent, price, signed};
use crate::{divergence, Lean, Sense};
use argos_core::Bar;
use argos_lens::explain::{Block, Sentence};
use argos_script::classic_inputs::{EMA_CROSS_PARAMS, MACD_PARAMS, RSI_PARAMS, VWAP_PARAMS};
use argos_script::indicators::classic;
use argos_script::inputs::{defaults, Inputs, Param};
use argos_script::v1_inputs::EMA_MULTI_PARAMS;

mod ai_trend;
mod anchored_vwap;
mod averages;
mod cipher;
mod flow;
mod live;
mod mtf_table;
mod patterns;
mod pivot_vwap;
mod price_action;
mod volume;
#[cfg(test)]
pub(crate) use spring::run as spring_run;
mod smc;
mod spring;
mod tbo;
mod tvd;
mod vector_zone;
#[cfg(test)]
pub(crate) use cipher::shown_points as cipher_points;
mod more;

/// The blocks and the role of each known setting key.
type Read = (Vec<Block>, &'static [(&'static str, &'static str)], Option<Sense>);
/// The blocks of one reading and its sense (`None`: too few bars).
type Blocks = (Vec<Block>, Option<Sense>);

const NOW: &str = "Ce qu'il montre maintenant";
const HOW: &str = "Comment le lire";
const CHANGE: &str = "Ce qui changerait la lecture";

const RSI_ROLES: [(&str, &str); 3] = [
    (
        "len",
        "Nombre de bougies du calcul : plus court, il réagit plus vite et donne plus de faux signaux.",
    ),
    ("haut", "Seuil de surachat : au-dessus, la hausse est jugée tendue."),
    ("bas", "Seuil de survente : en dessous, la baisse est jugée tendue."),
];
const MACD_ROLES: [(&str, &str); 3] = [
    ("fast", "EMA rapide : elle suit le prix de près."),
    ("slow", "EMA lente : la tendance de fond ; le MACD est l'écart entre les deux."),
    ("signal", "Moyenne du MACD : ses croisements avec le MACD marquent les changements d'élan."),
];
const EMA_CROSS_ROLES: [(&str, &str); 2] = [
    ("fast", "EMA rapide : la tendance courte."),
    (
        "slow",
        "EMA lente : la tendance plus longue ; les croisements des deux sont marqués sur la chart.",
    ),
];
const EMA_MULTI_ROLES: [(&str, &str); 6] = [
    ("p1", "Longueur de l'EMA 1 (0 = aucune)."),
    ("p2", "Longueur de l'EMA 2 (0 = aucune)."),
    ("p3", "Longueur de l'EMA 3 (0 = aucune)."),
    ("p4", "Longueur de l'EMA 4 (0 = aucune)."),
    ("p5", "Longueur de l'EMA 5 (0 = aucune)."),
    ("p6", "Longueur de l'EMA 6 (0 = aucune)."),
];
const VWAP_ROLES: [(&str, &str); 1] = [("hl3", "Prix utilisé : le prix typique (haut + bas + clôture) / 3, ou la clôture.")];

/// The settings values in the order of `params`: the defaults, replaced by the settings received.
fn values(params: &[Param], settings: &[(&str, f64)]) -> Vec<f64> {
    let mut v = defaults(params);
    for (k, x) in settings {
        if let Some(i) = params.iter().position(|p| p.key == *k) {
            v[i] = *x;
        }
    }
    v
}

fn s(id: &'static str, text: String) -> Sentence {
    Sentence { id, text }
}

/// 📋 The summary table (« Tableau résumé ») the chart draws, in one sentence: its rows exactly as the shared
/// function writes them (`argos_script::indicators::classic::*_tableau`). `None` without rows.
fn table(rows: &[(&str, String)]) -> Option<Sentence> {
    let cells: Vec<String> = rows.iter().map(|(k, v)| format!("{k} : {v}")).collect();
    (!cells.is_empty()).then(|| s("table", format!("Tableau résumé à l'écran : {}.", cells.join(" ; "))))
}

fn block(title: &'static str, sentences: Vec<Sentence>) -> Block {
    Block { title, sentences }
}

/// The last two values of `v`, when both are computed.
fn last_two(v: &[f64]) -> Option<(f64, f64)> {
    match v {
        [.., a, b] if a.is_finite() && b.is_finite() => Some((*a, *b)),
        _ => None,
    }
}

/// « Pas assez de bougies » for `name`, needing about `needed` closed bars.
fn too_few(name: &str, needed: usize, how: Vec<Sentence>) -> Blocks {
    let blocks = vec![
        block(
            NOW,
            vec![s(
                "too_few",
                format!("Pas assez de bougies clôturées pour calculer {name} (il en faut au moins {needed})."),
            )],
        ),
        block(HOW, how),
    ];
    (blocks, None)
}

/// A sense with no note.
fn sense(lean: Lean) -> Option<Sense> {
    Some(Sense { lean, note: None, value: None })
}

/// The same sense with its key value (« 41,2 », « écart −1,30 % »).
fn valued(sense: Option<Sense>, value: String) -> Option<Sense> {
    sense.map(|x| Sense { value: Some(value), ..x })
}

/// Up when `x` > 0, down when `x` < 0, neutral at 0.
fn lean_of(x: f64) -> Lean {
    if x > 0.0 {
        Lean::Up
    } else if x < 0.0 {
        Lean::Down
    } else {
        Lean::Neutral
    }
}

/// The note of an oscillator in its overbought or oversold zone (`hi` > `lo`).
fn zone_note(v: f64, (hi, lo): (f64, f64)) -> Option<&'static str> {
    if v >= hi {
        Some("en surachat")
    } else if v <= lo {
        Some("en survente")
    } else {
        None
    }
}

/// Bars, counted back from the last one, during which `f` stays true.
fn streak(n: usize, f: impl Fn(usize) -> bool) -> usize {
    (0..n).rev().take_while(|&i| f(i)).count()
}

/// « au-dessus de l'EMA 21 » / « sous l'EMA 21 »; « de le » becomes « du ».
fn side_of(up: bool, what: &str) -> String {
    match (up, what.strip_prefix("le ")) {
        (false, _) => format!("sous {what}"),
        (true, Some(rest)) => format!("au-dessus du {rest}"),
        (true, None) => format!("au-dessus de {what}"),
    }
}

/// A reading with the role of each setting key.
fn with_roles((blocks, sense): Blocks, roles: &'static [(&'static str, &'static str)]) -> Read {
    (blocks, roles, sense)
}

/// The settings that change a number or a mark the reading cites without being named by it (the calculation's
/// lengths, sources, filters), in the order of the catalogue: they come after the named ones in `keys_read`. Found
/// and kept honest by the test `no_setting_outside_keys_read_changes_the_reading`.
pub(crate) fn extra_keys(id: &str) -> &'static [&'static str] {
    match id {
        "EmaMulti" => &["src_hl2"],
        "Cipher" => &[
            "wt_src", "mfi_p", "mfi_m", "mfi_y", "rsi_src", "st_avg", "st_src", "st_len", "st_rsi", "st_k", "st_d", "st_reg",
        ],
        "AiTrend" => &[
            "preset",
            "smooth_type",
            "sig_type",
            "min_move",
            "m1_on",
            "m1_tf",
            "m2_on",
            "m2_tf",
            "m3_on",
            "m3_tf",
            "m4_on",
            "m4_tf",
            "htf_mode",
            "mtf_pl",
            "mtf_pr",
            "mtf_rmin",
            "mtf_rmax",
        ],
        "Tbo" => &["preset", "calc", "gate_on", "div_hid", "minOscAtr"],
        "Tvd" => &[
            "rsiLen",
            "stochLen",
            "momLen",
            "smoothLen",
            "signalLen",
            "trendNeedsEma",
            "pivL",
            "rangeMin",
            "rangeMax",
            "showReg",
            "showHid",
        ],
        "VectorZone" => &[
            "vLen",
            "zPivL",
            "zPivR",
            "minReact",
            "maxZones",
            "liqPiv",
            "eqTolL",
            "maxLiq",
            "strongLvl",
            "weakLvl",
        ],
        "Smc" => &["tf", "len", "ob_body", "ob_wick", "fvg_min", "max_touch", "eq_thr"],
        "SmcPro" => &["swingLen", "maxOB", "obUseBody", "fond", "emaLen", "ote886", "fib1Len"],
        "SpringTension" => &["MFI Length"],
        "PriceAction" => &["portee", "corps", "pwrMode", "vecMult"],
        "Patterns" => &["double", "hs", "tri", "chan", "target", "triple", "fibch"],
        "Tape" => &["showTable", "m1", "m2"],
        "Footprint" => &["poc", "dl"],
        _ => &[],
    }
}

/// The settings each reading reads by name, with their role: the one table both `read` (the roles of
/// `Reading::settings`) and `keys_read` use. `None`: no rules for this id.
pub(crate) fn roles_of(id: &str) -> Option<&'static [(&'static str, &'static str)]> {
    Some(match id {
        "Rsi" => &RSI_ROLES,
        "Macd" => &MACD_ROLES,
        "EmaCross" => &EMA_CROSS_ROLES,
        "EmaMulti" => &EMA_MULTI_ROLES,
        "Vwap" => &VWAP_ROLES,
        "Cipher" => &cipher::ROLES,
        "AiTrend" => &ai_trend::ROLES,
        "Tbo" => &tbo::ROLES,
        "Tvd" => &tvd::ROLES,
        "VectorZone" => &vector_zone::ROLES,
        "Smc" => &smc::SMC_ROLES,
        "SpringTension" => &spring::ROLES,
        "SmcPro" => &smc::PRO_ROLES,
        "PriceAction" => &price_action::ROLES,
        "Patterns" => &patterns::ROLES,
        "SwingVwap" => &pivot_vwap::ONE_ROLES,
        "SmartSwingVwap" => &pivot_vwap::MULTI_ROLES,
        "AnchoredVwap" => &anchored_vwap::ROLES,
        "Heatmap" => &volume::HEATMAP_ROLES,
        "Profile" => &volume::PROFILE_ROLES,
        "Sma" => &averages::SMA_ROLES,
        "Wma" => &averages::WMA_ROLES,
        "Hma" => &averages::HMA_ROLES,
        "Vwma" => &averages::VWMA_ROLES,
        "Smma" => &averages::SMMA_ROLES,
        "MaRibbon" => &averages::RIBBON_ROLES,
        "Bubbles" => &flow::BUBBLES_ROLES,
        "Cvd" => &flow::CVD_ROLES,
        "MtfTable" => &mtf_table::ROLES,
        "Tape" => &live::TAPE_ROLES,
        "BookHeat" | "LiqMap" => &[],
        "Footprint" => &flow::FOOTPRINT_ROLES,
        _ => return more::roles_of(id),
    })
}

pub(crate) fn read(id: &str, b: &[Bar], settings: &[(&str, f64)], data: &crate::ScreenData) -> Option<Read> {
    Some(match id {
        "Rsi" => with_roles(rsi(b, &values(&RSI_PARAMS, settings)), &RSI_ROLES),
        "Macd" => with_roles(macd(b, &values(&MACD_PARAMS, settings)), &MACD_ROLES),
        "EmaCross" => with_roles(ema_cross(b, &values(&EMA_CROSS_PARAMS, settings)), &EMA_CROSS_ROLES),
        "EmaMulti" => with_roles(ema_multi(b, &values(&EMA_MULTI_PARAMS, settings)), &EMA_MULTI_ROLES),
        "Vwap" => with_roles(vwap(b, &values(&VWAP_PARAMS, settings)), &VWAP_ROLES),
        "Cipher" => with_roles(cipher::read(b, &values(&argos_script::indicators::cipher::PARAMS, settings)), &cipher::ROLES),
        "AiTrend" => with_roles(
            ai_trend::read(b, &values(&argos_script::indicators::ai_trend::PARAMS, settings), data),
            &ai_trend::ROLES,
        ),
        "Tbo" => with_roles(tbo::read(b, &values(&argos_script::indicators::tbo::PARAMS, settings)), &tbo::ROLES),
        "Tvd" => with_roles(tvd::read(b, &values(&argos_script::indicators::tvd::PARAMS, settings)), &tvd::ROLES),
        "VectorZone" => with_roles(
            vector_zone::read(b, &values(&argos_script::indicators::vector_zone::PARAMS, settings)),
            &vector_zone::ROLES,
        ),
        "SpringTension" => with_roles(spring::read(b, settings), &spring::ROLES),
        "Smc" => with_roles(smc::read_smc(b, &values(&argos_script::indicators::smc::PARAMS, settings)), &smc::SMC_ROLES),
        "SmcPro" => with_roles(smc::read_pro(b, &values(&argos_script::indicators::smc_pro::PARAMS, settings)), &smc::PRO_ROLES),
        "PriceAction" => with_roles(
            price_action::read(b, &values(&argos_script::indicators::order_blocks::PARAMS, settings)),
            &price_action::ROLES,
        ),
        "Patterns" => with_roles(
            patterns::read(b, &values(&argos_script::indicators::patterns::PARAMS, settings)),
            &patterns::ROLES,
        ),
        "SwingVwap" => with_roles(
            pivot_vwap::read_one(b, &values(&argos_script::classic_inputs::PIVOT_VWAP_PARAMS, settings)),
            &pivot_vwap::ONE_ROLES,
        ),
        "SmartSwingVwap" => with_roles(
            pivot_vwap::read_multi(b, &values(&argos_script::classic_inputs::PIVOT_VWAP_MULTI_PARAMS, settings)),
            &pivot_vwap::MULTI_ROLES,
        ),
        "AnchoredVwap" => with_roles(
            anchored_vwap::read(b, &values(&argos_script::indicators::anchored_vwap::PARAMS, settings), data),
            &anchored_vwap::ROLES,
        ),
        "Heatmap" => with_roles(
            volume::read_heatmap(b, &values(&argos_script::v1_inputs::HEATMAP_PARAMS, settings)),
            &volume::HEATMAP_ROLES,
        ),
        "Profile" => with_roles(
            volume::read_profile(b, &values(&argos_script::v1_inputs::PROFILE_PARAMS, settings)),
            &volume::PROFILE_ROLES,
        ),
        "Sma" => with_roles(averages::read_one("Sma", b, settings), &averages::SMA_ROLES),
        "Wma" => with_roles(averages::read_one("Wma", b, settings), &averages::WMA_ROLES),
        "Hma" => with_roles(averages::read_one("Hma", b, settings), &averages::HMA_ROLES),
        "Vwma" => with_roles(averages::read_one("Vwma", b, settings), &averages::VWMA_ROLES),
        "Smma" => with_roles(averages::read_one("Smma", b, settings), &averages::SMMA_ROLES),
        "MaRibbon" => with_roles(averages::read_ribbon(b, settings), &averages::RIBBON_ROLES),
        "Bubbles" => with_roles(
            flow::read_bubbles(b, &values(&argos_script::indicators::flow::BUBBLES_PARAMS, settings), data),
            &flow::BUBBLES_ROLES,
        ),
        "Cvd" => with_roles(
            flow::read_cvd(b, &values(&argos_script::classic_inputs::OBV_PARAMS, settings), data),
            &flow::CVD_ROLES,
        ),
        "MtfTable" => with_roles(
            mtf_table::read(b, &values(&argos_script::indicators::mtf_table::PARAMS, settings), data),
            &mtf_table::ROLES,
        ),
        "Tape" => with_roles(
            live::read_tape(b, &values(&argos_script::indicators::flow::TAPE_PARAMS, settings), data),
            &live::TAPE_ROLES,
        ),
        "BookHeat" => with_roles(live::read_book(b, data), &[]),
        "LiqMap" => with_roles(live::read_liq(b, data), &[]),
        "Footprint" => with_roles(
            flow::read_footprint(b, &values(&argos_script::indicators::flow::FOOTPRINT_PARAMS, settings), data),
            &flow::FOOTPRINT_ROLES,
        ),
        _ => return more::read(id, b, settings),
    })
}

fn rsi(b: &[Bar], vals: &[f64]) -> Blocks {
    let p = Inputs { params: &RSI_PARAMS, vals };
    let (len, hi, lo) = (p.len("len"), p.get("haut"), p.get("bas"));
    let name = format!("le RSI {len}");
    let mut how = vec![
        s("how_50", "Au-dessus de 50, les hausses récentes l'emportent sur les baisses ; sous 50, c'est l'inverse.".into()),
        s(
            "how_zones",
            format!(
                "Au-dessus de {} (surachat), la hausse est tendue ; sous {} (survente), la baisse est tendue. Dans une forte tendance, le RSI peut rester longtemps dans ces zones.",
                num(hi, 0),
                num(lo, 0)
            ),
        ),
    ];
    let r = classic::rsi_n(b, len);
    let Some((prev, v)) = last_two(&r) else {
        return too_few(&name, len + 2, how);
    };
    let mut now = vec![s(
        "value",
        format!(
            "RSI {len} à {} : {}, il {} depuis la bougie d'avant ({}).",
            num(v, 1),
            side_of(v > 50.0, "50"),
            if v >= prev { "monte" } else { "baisse" },
            num(prev, 1)
        ),
    )];
    now.push(if v >= hi {
        s("zone", format!("Il est en surachat (au-dessus de {}).", num(hi, 0)))
    } else if v <= lo {
        s("zone", format!("Il est en survente (sous {}).", num(lo, 0)))
    } else if prev >= hi {
        s("zone", format!("Il vient de sortir du surachat (repassé sous {}).", num(hi, 0)))
    } else if prev <= lo {
        s("zone", format!("Il vient de sortir de la survente (repassé au-dessus de {}).", num(lo, 0)))
    } else {
        s("zone", format!("Il est entre {} et {} : ni surachat ni survente.", num(lo, 0), num(hi, 0)))
    });
    let mut change = vec![s(
        "change_50",
        if v > 50.0 {
            "Un passage sous 50 montrerait que les baisses reprennent le dessus.".to_string()
        } else {
            "Un passage au-dessus de 50 montrerait que les hausses reprennent le dessus.".to_string()
        },
    )];
    if v >= hi {
        change.push(s(
            "change_zone",
            format!("Un retour sous {} montrerait que l'élan haussier faiblit.", num(hi, 0)),
        ));
    } else if v <= lo {
        change.push(s(
            "change_zone",
            format!("Un retour au-dessus de {} montrerait que l'élan baissier faiblit.", num(lo, 0)),
        ));
    }
    let div = divergence::find(b, &r);
    if let Some(d) = &div {
        let (now_d, how_d) = divergence::sentences(d, &format!("le RSI {len}"), price, |x| num(x, 1));
        now.push(now_d);
        how.push(how_d);
    }
    let sense = Some(Sense {
        lean: lean_of(v - 50.0),
        note: divergence::note(zone_note(v, (hi, lo)), div.as_ref()),
        value: Some(num(v, 1)),
    });
    (vec![block(NOW, now), block(HOW, how), block(CHANGE, change)], sense)
}

fn macd(b: &[Bar], vals: &[f64]) -> Blocks {
    let p = Inputs { params: &MACD_PARAMS, vals };
    let (fast, slow, sig) = (p.len("fast"), p.len("slow"), p.len("signal"));
    let mut how = vec![
        s(
            "how_signal",
            "MACD au-dessus de son signal : l'élan est haussier ; en dessous : baissier.".into(),
        ),
        s(
            "how_zero",
            format!("Au-dessus de zéro, l'EMA {fast} est au-dessus de l'EMA {slow} ; sous zéro, en dessous."),
        ),
        s(
            "how_hist",
            "L'histogramme est l'écart entre le MACD et son signal : quand il diminue, l'élan ralentit.".into(),
        ),
    ];
    let m = classic::macd_n(b, fast, slow, sig);
    let (Some((_, line)), Some((_, signal)), Some((h0, h))) = (last_two(&m.line), last_two(&m.signal), last_two(&m.hist)) else {
        return too_few(&format!("le MACD {fast}/{slow}/{sig}"), slow.max(fast) + sig + 1, how);
    };
    let c = b.last().map_or(line, |x| x.close);
    let sg = |x: f64| signed(x, c);
    let up = h > 0.0;
    let k = streak(m.hist.len(), |i| m.hist[i].is_finite() && (m.hist[i] > 0.0) == up);
    let mut now = vec![
        s(
            "cross",
            format!("MACD à {} : {} ({}) depuis {}.", sg(line), side_of(up, "son signal"), sg(signal), n_bars(k)),
        ),
        s("zero", format!("Le MACD est {} de zéro.", if line > 0.0 { "au-dessus" } else { "en dessous" })),
        s(
            "hist",
            format!(
                "Histogramme à {} : l'écart {} (bougie d'avant : {}).",
                sg(h),
                if h.abs() >= h0.abs() { "s'élargit" } else { "se resserre" },
                sg(h0)
            ),
        ),
    ];
    let div = divergence::find(b, &m.line);
    if let Some(d) = &div {
        let (now_d, how_d) = divergence::sentences(d, "le MACD", price, sg);
        now.push(now_d);
        how.push(how_d);
    }
    let change = vec![
        s(
            "change_cross",
            format!("Un croisement du MACD {} changerait la lecture de l'élan.", side_of(!up, "son signal")),
        ),
        s(
            "change_zero",
            format!("Un passage {} changerait la lecture de la tendance.", side_of(line <= 0.0, "zéro")),
        ),
    ];
    (
        vec![block(NOW, now), block(HOW, how), block(CHANGE, change)],
        valued(
            Some(Sense {
                lean: lean_of(h),
                note: divergence::note(None, div.as_ref()),
                value: None,
            }),
            format!("histogramme {}", sg(h)),
        ),
    )
}

fn ema_cross(b: &[Bar], vals: &[f64]) -> Blocks {
    let p = Inputs {
        params: &EMA_CROSS_PARAMS,
        vals,
    };
    let (f, sl) = (p.len("fast"), p.len("slow"));
    let (ef, es) = (format!("l'EMA {f}"), format!("l'EMA {sl}"));
    let how = vec![
        s(
            "how_order",
            format!("EMA {f} au-dessus de {es} : la tendance courte est haussière ; en dessous : baissière."),
        ),
        s(
            "how_price",
            "Le prix au-dessus des deux moyennes confirme la hausse ; en dessous des deux, la baisse.".into(),
        ),
    ];
    let e = classic::ema_cross_n(b, f, sl);
    let (Some((_, vf)), Some((_, vs)), Some(last)) = (last_two(&e.fast), last_two(&e.slow), b.last()) else {
        return too_few(&format!("les EMA {f} et {sl}"), 2, how);
    };
    let c = last.close;
    let n = b.len();
    let up = vf > vs;
    // the last cross, and how many bars ago
    let last_cross = match (e.up.last(), e.down.last()) {
        (Some(&u), Some(&d)) => Some((u.max(d), u > d)),
        (Some(&u), None) => Some((u, true)),
        (None, Some(&d)) => Some((d, false)),
        (None, None) => None,
    };
    let slope = e.slow.len().checked_sub(4).map(|i| e.slow[i]).filter(|x| x.is_finite());
    let mut now = vec![
        s(
            "price",
            format!(
                "Clôture à {} : {} ({}) et {} ({}).",
                price(c),
                side_of(c > vf, &ef),
                price(vf),
                side_of(c > vs, &es),
                price(vs)
            ),
        ),
        s(
            "order",
            format!(
                "EMA {f} {} : {}.",
                side_of(up, &es),
                match last_cross {
                    Some((i, u)) => format!(
                        "dernier croisement {} il y a {}",
                        if u { "à la hausse" } else { "à la baisse" },
                        n_bars(n - 1 - i)
                    ),
                    None => "aucun croisement dans l'historique chargé".to_string(),
                }
            ),
        ),
    ];
    if let Some(s3) = slope {
        now.push(s(
            "slope",
            format!(
                "EMA {sl} en {} sur 3 bougies (de {} à {}).",
                if vs >= s3 { "hausse" } else { "baisse" },
                price(s3),
                price(vs)
            ),
        ));
    }
    let change = vec![s(
        "change_cross",
        format!("Un croisement de l'EMA {f} {} changerait la lecture de la tendance courte.", side_of(!up, &es)),
    )];
    (
        vec![block(NOW, now), block(HOW, how), block(CHANGE, change)],
        valued(sense(lean_of(vf - vs)), format!("EMA {f} à {}, EMA {sl} à {}", price(vf), price(vs))),
    )
}

fn ema_multi(b: &[Bar], vals: &[f64]) -> Blocks {
    let p = Inputs {
        params: &EMA_MULTI_PARAMS,
        vals,
    };
    let lens: Vec<usize> = ["p1", "p2", "p3", "p4", "p5", "p6"]
        .iter()
        .map(|k| p.get(k).round().max(0.0) as usize)
        .collect();
    let how = vec![
        s(
            "how_align",
            "Les EMA courtes au-dessus des longues, bien rangées : tendance haussière nette ; l'inverse : baissière.".into(),
        ),
        s("how_mixed", "Des EMA mélangées montrent un marché sans tendance claire.".into()),
    ];
    let series = classic::ema_multiples(b, vals);
    // (length, last value) of each EMA in use and computed, from the shortest
    let mut on: Vec<(usize, f64)> = lens
        .iter()
        .zip(&series)
        .filter(|(l, _)| **l > 0)
        .filter_map(|(l, v)| v.last().copied().filter(|x| x.is_finite()).map(|x| (*l, x)))
        .collect();
    on.sort_by_key(|x| x.0);
    let Some(last) = b.last().filter(|_| !on.is_empty()) else {
        let longest = lens.iter().copied().max().unwrap_or(0);
        return too_few("les EMA", longest.max(1), how);
    };
    let c = last.close;
    let missing = lens.iter().filter(|l| **l > 0).count() - on.len();
    let pos: Vec<String> = on
        .iter()
        .map(|(l, v)| format!("{} ({})", side_of(c > *v, &format!("l'EMA {l}")), price(*v)))
        .collect();
    let mut now = vec![s("price", format!("Clôture à {} : {}.", price(c), pos.join(", ")))];
    if missing > 0 {
        now.push(s(
            "missing",
            if missing == 1 {
                "1 EMA pas encore calculée : pas assez de bougies pour sa longueur.".to_string()
            } else {
                format!("{missing} EMA pas encore calculées : pas assez de bougies pour leur longueur.")
            },
        ));
    }
    let up = on.windows(2).all(|w| w[0].1 > w[1].1);
    let down = on.windows(2).all(|w| w[0].1 < w[1].1);
    now.push(s(
        "align",
        if on.len() < 2 {
            "Une seule EMA calculée : pas d'alignement à lire.".to_string()
        } else if up {
            "Les EMA sont rangées de la plus courte (en haut) à la plus longue : alignement haussier.".to_string()
        } else if down {
            "Les EMA sont rangées de la plus longue (en haut) à la plus courte : alignement baissier.".to_string()
        } else {
            "Les EMA ne sont pas bien rangées : pas d'alignement net.".to_string()
        },
    ));
    let (l_long, v_long) = on[on.len() - 1];
    let change = vec![s(
        "change_long",
        format!(
            "Une clôture {} (la plus longue, à {}) changerait la lecture de fond.",
            side_of(c <= v_long, &format!("l'EMA {l_long}")),
            price(v_long)
        ),
    )];
    let lean = if on.len() < 2 {
        lean_of(c - v_long)
    } else if up {
        Lean::Up
    } else if down {
        Lean::Down
    } else {
        Lean::Neutral
    };
    (vec![block(NOW, now), block(HOW, how), block(CHANGE, change)], sense(lean))
}

fn vwap(b: &[Bar], vals: &[f64]) -> Blocks {
    let p = Inputs { params: &VWAP_PARAMS, vals };
    let how = vec![
        s(
            "how_day",
            "Le VWAP est le prix moyen du jour pondéré par le volume ; il repart de zéro chaque jour à 00:00 UTC.".into(),
        ),
        s(
            "how_side",
            "Au-dessus du VWAP, les acheteurs du jour sont en moyenne gagnants ; en dessous, ce sont les vendeurs.".into(),
        ),
    ];
    let v = classic::vwap_src(b, p.on("hl3"));
    let (Some((_, w)), Some(last)) = (last_two(&v), b.last()) else {
        return too_few("le VWAP", 2, how);
    };
    let c = last.close;
    let up = c > w;
    let k = streak(b.len().min(v.len()), |i| v[i].is_finite() && (b[i].close > v[i]) == up);
    let now = vec![
        s(
            "price",
            format!(
                "Clôture à {} : {} du jour ({}), écart {}.",
                price(c),
                side_of(up, "le VWAP"),
                price(w),
                percent((c - w) / w * 100.0)
            ),
        ),
        s("streak", format!("Le prix clôture de ce côté depuis {}.", n_bars(k))),
    ];
    let change = vec![s(
        "change_side",
        format!("Une clôture {} changerait la lecture du jour.", side_of(!up, "le VWAP")),
    )];
    (
        vec![block(NOW, now), block(HOW, how), block(CHANGE, change)],
        valued(sense(lean_of(c - w)), format!("écart {}", percent((c - w) / w * 100.0))),
    )
}
