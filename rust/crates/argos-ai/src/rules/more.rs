//! The rules of lot 2: oscillators (Stoch RSI, MFI, Stochastic, CCI, Williams %R, Aroon), volatility and volume (ATR,
//! relative volume, OBV), trend (DMI / ADX, Supertrend, Parabolic SAR) and channels (Bollinger, Keltner, Donchian).
//! Same shape as lot 1: the chart's Argos Script function, the chart's settings table, fixed sentences.

use super::{block, last_two, lean_of, s, sense, side_of, streak, table, too_few, valued, values, with_roles, zone_note, Blocks, Read, CHANGE, HOW, NOW};
use crate::format::{bars as n_bars, num, percent, price, signed};
use crate::{divergence, Lean, Sense};
use argos_core::Bar;
use argos_lens::explain::Sentence;
use argos_script::classic_inputs::{
    AROON_PARAMS, ATR_PARAMS, CCI_PARAMS, DMI_PARAMS, DONCHIAN_PARAMS, ICHIMOKU_PARAMS, KELTNER_PARAMS, MFI_PARAMS, OBV_PARAMS, PSAR_PARAMS, REL_VOLUME_PARAMS,
    STOCH_PARAMS, STOCH_RSI_PARAMS, SUPERTREND_PARAMS, WILLIAMS_PARAMS,
};
use argos_script::indicators::{classic, textbook};
use argos_script::inputs::Inputs;
use argos_script::v1_inputs::{BANDS_OSC_PARAMS, BANDS_PARAMS};

/// Overbought and oversold lines the chart draws for the stochastics and the MFI (fixed, not settings).
const HI_LO_80_20: (f64, f64) = (80.0, 20.0);
/// Bars over which a slope or a width is compared.
const LOOKBACK: usize = 5;

const STOCH_RSI_ROLES: [(&str, &str); 4] = [
    ("rsi", "Longueur du RSI de départ."),
    ("len", "Nombre de bougies où l'on cherche le plus haut et le plus bas du RSI."),
    ("k", "Lissage de %K : plus grand, la ligne est plus calme."),
    ("d", "Lissage de %D, la moyenne de %K."),
];
const MFI_ROLES: [(&str, &str); 1] = [("len", "Nombre de bougies du calcul : plus court, il réagit plus vite.")];
const STOCH_ROLES: [(&str, &str); 3] = [
    ("len", "Nombre de bougies où l'on cherche le plus haut et le plus bas."),
    ("smooth", "Lissage de %K : plus grand, la ligne est plus calme."),
    ("d", "Lissage de %D, la moyenne de %K."),
];
/// TradingView's « Bollinger Bands », run by Argos Script (keys: the titles of its inputs).
const BOLLINGER_ROLES: [(&str, &str); 5] = [
    ("Length", "Longueur de la moyenne du milieu et de l'écart-type."),
    ("Basis MA Type", "Moyenne du milieu : SMA, EMA, SMMA (RMA), WMA ou VWMA."),
    ("Source", "Prix utilisé (par défaut la clôture)."),
    ("StdDev", "Écart des bandes, en écarts-types : plus grand, le prix les touche plus rarement."),
    ("Offset", "Décale les bandes de n bougies à l'écran."),
];
/// The basis types of TradingView's Bollinger Bands, in the order of its list, as the reading names them.
const BOLLINGER_TYPES: [&str; 5] = ["simple", "exponentielle", "lissée (SMMA)", "pondérée (WMA)", "pondérée par le volume (VWMA)"];
const ATR_ROLES: [(&str, &str); 1] = [("len", "Nombre de bougies de la moyenne des amplitudes.")];
const REL_VOLUME_ROLES: [(&str, &str); 2] = [
    ("len", "Nombre de bougies de la moyenne du volume."),
    ("pic", "À partir de combien de fois la moyenne un volume est marqué comme un pic."),
];
const DMI_ROLES: [(&str, &str); 3] = [
    ("len", "Longueur des lignes +DI et −DI."),
    ("adx", "Lissage de l'ADX."),
    ("seuil", "ADX à partir duquel la tendance est jugée forte (ligne de la chart)."),
];
const SUPERTREND_ROLES: [(&str, &str); 2] = [
    ("atr", "Longueur de l'ATR qui règle la distance de la ligne."),
    ("mult", "Distance de la ligne en ATR : plus grand, moins de retournements."),
];
const CCI_ROLES: [(&str, &str); 1] = [("len", "Nombre de bougies du calcul.")];
const WILLIAMS_ROLES: [(&str, &str); 1] = [("len", "Nombre de bougies où l'on cherche le plus haut et le plus bas.")];
const OBV_ROLES: [(&str, &str); 1] = [("sig", "Longueur de la moyenne de l'OBV (0 = aucune).")];
const KELTNER_ROLES: [(&str, &str); 3] = [
    ("len", "Longueur de l'EMA du milieu."),
    ("atr", "Longueur de l'ATR qui règle la largeur."),
    ("mult", "Largeur du canal en ATR."),
];
const DONCHIAN_ROLES: [(&str, &str); 1] = [("len", "Nombre de bougies du plus haut et du plus bas.")];
const PSAR_ROLES: [(&str, &str); 3] = [
    ("start", "Accélération de départ."),
    ("step", "Accélération ajoutée à chaque nouveau plus haut (ou plus bas)."),
    ("max", "Accélération maximale : plus grande, les points rattrapent le prix plus vite."),
];
const AROON_ROLES: [(&str, &str); 1] = [("len", "Nombre de bougies où l'on cherche le dernier plus haut et le dernier plus bas.")];

/// The settings each reading of this lot reads by name, with their role (the one table `read` and `keys_read` use).
pub(super) fn roles_of(id: &str) -> Option<&'static [(&'static str, &'static str)]> {
    Some(match id {
        "StochRsi" => &STOCH_RSI_ROLES,
        "Mfi" => &MFI_ROLES,
        "Stochastic" => &STOCH_ROLES,
        "Bollinger" => &BOLLINGER_ROLES,
        "Atr" => &ATR_ROLES,
        "RelVolume" => &REL_VOLUME_ROLES,
        "Dmi" => &DMI_ROLES,
        "Supertrend" => &SUPERTREND_ROLES,
        "Cci" => &CCI_ROLES,
        "WilliamsR" => &WILLIAMS_ROLES,
        "Obv" => &OBV_ROLES,
        "Keltner" => &KELTNER_ROLES,
        "Donchian" => &DONCHIAN_ROLES,
        "Psar" => &PSAR_ROLES,
        "Aroon" => &AROON_ROLES,
        "Bands" => &BANDS_ROLES,
        "BandsOsc" => &BANDS_OSC_ROLES,
        "Ichimoku" => &ICHIMOKU_ROLES,
        _ => return None,
    })
}

pub(super) fn read(id: &str, b: &[Bar], st: &[(&str, f64)]) -> Option<Read> {
    Some(match id {
        "StochRsi" => with_roles(stoch_rsi(b, &values(&STOCH_RSI_PARAMS, st)), &STOCH_RSI_ROLES),
        "Mfi" => with_roles(mfi(b, &values(&MFI_PARAMS, st)), &MFI_ROLES),
        "Stochastic" => with_roles(stochastic(b, &values(&STOCH_PARAMS, st)), &STOCH_ROLES),
        "Bollinger" => with_roles(bollinger(b, st), &BOLLINGER_ROLES),
        "Atr" => with_roles(atr(b, &values(&ATR_PARAMS, st)), &ATR_ROLES),
        "RelVolume" => with_roles(rel_volume(b, &values(&REL_VOLUME_PARAMS, st)), &REL_VOLUME_ROLES),
        "Dmi" => with_roles(dmi(b, &values(&DMI_PARAMS, st)), &DMI_ROLES),
        "Supertrend" => with_roles(supertrend(b, &values(&SUPERTREND_PARAMS, st)), &SUPERTREND_ROLES),
        "Cci" => with_roles(cci(b, &values(&CCI_PARAMS, st)), &CCI_ROLES),
        "WilliamsR" => with_roles(williams(b, &values(&WILLIAMS_PARAMS, st)), &WILLIAMS_ROLES),
        "Obv" => with_roles(obv(b, &values(&OBV_PARAMS, st)), &OBV_ROLES),
        "Keltner" => with_roles(keltner(b, &values(&KELTNER_PARAMS, st)), &KELTNER_ROLES),
        "Donchian" => with_roles(donchian(b, &values(&DONCHIAN_PARAMS, st)), &DONCHIAN_ROLES),
        "Psar" => with_roles(psar(b, &values(&PSAR_PARAMS, st)), &PSAR_ROLES),
        "Aroon" => with_roles(aroon(b, &values(&AROON_PARAMS, st)), &AROON_ROLES),
        "Bands" => with_roles(bands(b, &values(&BANDS_PARAMS, st)), &BANDS_ROLES),
        "BandsOsc" => with_roles(bands_osc(b, &values(&BANDS_OSC_PARAMS, st)), &BANDS_OSC_ROLES),
        "Ichimoku" => with_roles(ichimoku(b, &values(&ICHIMOKU_PARAMS, st)), &ICHIMOKU_ROLES),
        _ => return None,
    })
}

// — shared readings —

/// « {name} à 64,3 : il monte depuis la bougie d'avant (58,1). »
fn moves(name: &str, v: f64, prev: f64, dp: u32) -> Sentence {
    s(
        "value",
        format!(
            "{name} à {} : il {} depuis la bougie d'avant ({}).",
            num(v, dp),
            if v >= prev { "monte" } else { "baisse" },
            num(prev, dp)
        ),
    )
}

/// The zone of an oscillator between `lo` and `hi` (`hi` > `lo`), and what would change it.
fn zone(v: f64, prev: f64, (hi, lo): (f64, f64)) -> (Sentence, Option<Sentence>) {
    let (h, l) = (num(hi, 0), num(lo, 0));
    if v >= hi {
        (
            s("zone", format!("Il est en surachat (au-dessus de {h}).")),
            Some(s("change_zone", format!("Un retour sous {h} montrerait que l'élan haussier faiblit."))),
        )
    } else if v <= lo {
        (
            s("zone", format!("Il est en survente (sous {l}).")),
            Some(s("change_zone", format!("Un retour au-dessus de {l} montrerait que l'élan baissier faiblit."))),
        )
    } else if prev >= hi {
        (s("zone", format!("Il vient de sortir du surachat (repassé sous {h}).")), None)
    } else if prev <= lo {
        (s("zone", format!("Il vient de sortir de la survente (repassé au-dessus de {l}).")), None)
    } else {
        (s("zone", format!("Il est entre {l} et {h} : ni surachat ni survente.")), None)
    }
}

/// %K against %D: the side and since when, and what would change it.
fn k_over_d(k: &[f64], d: &[f64]) -> Option<(Sentence, Sentence)> {
    let ((_, kv), (_, dv)) = (last_two(k)?, last_two(d)?);
    let up = kv > dv;
    let n = streak(k.len().min(d.len()), |i| k[i].is_finite() && d[i].is_finite() && (k[i] > d[i]) == up);
    Some((
        s(
            "cross",
            format!("%K à {} : {} ({}) depuis {}.", num(kv, 1), side_of(up, "%D"), num(dv, 1), n_bars(n)),
        ),
        s(
            "change_cross",
            format!("Un croisement de %K {} marquerait un changement d'élan à court terme.", side_of(!up, "%D")),
        ),
    ))
}

/// A stochastic-like oscillator: %K, %D, zone on %K.
fn kd(name: &str, needed: usize, (k, d): (Vec<f64>, Vec<f64>), how: Vec<Sentence>) -> Blocks {
    let (Some((kp, kv)), Some((cross, change_cross))) = (last_two(&k), k_over_d(&k, &d)) else {
        return too_few(name, needed, how);
    };
    let (z, change_zone) = zone(kv, kp, HI_LO_80_20);
    let mut change = vec![change_cross];
    change.extend(change_zone);
    let dv = last_two(&d).map_or(kv, |x| x.1);
    let sense = Some(Sense {
        lean: lean_of(kv - dv),
        note: zone_note(kv, HI_LO_80_20),
        value: Some(format!("%K {}", num(kv, 1))),
    });
    (vec![block(NOW, vec![cross, z]), block(HOW, how), block(CHANGE, change)], sense)
}

/// Where the close stands in a channel (upper, middle, lower), and the channel's width now and `LOOKBACK` bars ago.
fn channel(name: &str, c: f64, (u, m, l): (&[f64], &[f64], &[f64])) -> Option<(Vec<Sentence>, Sentence, Sense)> {
    let n = u.len().min(m.len()).min(l.len());
    let at = |v: &[f64], i: usize| v.get(i).copied().filter(|x| x.is_finite());
    let (uv, mv, lv) = (at(u, n.checked_sub(1)?)?, at(m, n - 1)?, at(l, n - 1)?);
    let where_ = if c > uv {
        format!("au-dessus de la bande haute ({})", price(uv))
    } else if c < lv {
        format!("sous la bande basse ({})", price(lv))
    } else if c >= mv {
        format!("entre le milieu ({}) et la bande haute ({})", price(mv), price(uv))
    } else {
        format!("entre la bande basse ({}) et le milieu ({})", price(lv), price(mv))
    };
    let mut now = vec![s("position", format!("Clôture à {} : {where_} du {name}.", price(c)))];
    let width = |i: usize| Some((at(u, i)? - at(l, i)?) / at(m, i)? * 100.0);
    if let (Some(w), Some(w0)) = (width(n - 1), n.checked_sub(1 + LOOKBACK).and_then(width)) {
        now.push(s(
            "width",
            format!(
                "Largeur {} du prix : elle {} depuis {} ({}).",
                num(w, 2) + " %",
                if w >= w0 { "s'élargit" } else { "se resserre" },
                n_bars(LOOKBACK),
                num(w0, 2) + " %"
            ),
        ));
    }
    let change = s(
        "change_middle",
        format!("Une clôture {} changerait la lecture.", side_of(c < mv, &format!("le milieu ({})", price(mv)))),
    );
    let note = if c > uv {
        Some("au-dessus de la bande haute")
    } else if c < lv {
        Some("sous la bande basse")
    } else {
        None
    };
    Some((
        now,
        change,
        Sense {
            lean: lean_of(c - mv),
            note,
            value: None,
        },
    ))
}

/// The last bar where `up` changed, as « il y a N bougies ».
fn last_flip(up: &[bool]) -> Option<usize> {
    (1..up.len()).rev().find(|&i| up[i] != up[i - 1]).map(|i| up.len() - 1 - i)
}

// — oscillators —

fn stoch_rsi(b: &[Bar], vals: &[f64]) -> Blocks {
    let p = Inputs {
        params: &STOCH_RSI_PARAMS,
        vals,
    };
    let (r, len) = (p.len("rsi"), p.len("len"));
    let how = vec![
        s(
            "how_scale",
            format!("Il place le RSI {r} entre son plus bas (0) et son plus haut (100) des {len} dernières bougies : il réagit très vite."),
        ),
        s(
            "how_zones",
            "Au-dessus de 80, le RSI est au plus haut de sa plage récente (surachat) ; sous 20, au plus bas (survente).".into(),
        ),
        s("how_cross", "%K qui croise %D marque un changement d'élan à court terme.".into()),
    ];
    let kdv = textbook::stoch_rsi_n(b, r, len, p.len("k"), p.len("d"));
    kd("le RSI stochastique", r + len + p.len("k") + p.len("d"), kdv, how)
}

fn stochastic(b: &[Bar], vals: &[f64]) -> Blocks {
    let p = Inputs { params: &STOCH_PARAMS, vals };
    let len = p.len("len");
    let how = vec![
        s(
            "how_scale",
            format!("Il place la clôture entre le plus bas (0) et le plus haut (100) des {len} dernières bougies."),
        ),
        s(
            "how_zones",
            "Au-dessus de 80, le prix clôture près de ses plus hauts récents (surachat) ; sous 20, près de ses plus bas (survente).".into(),
        ),
        s("how_cross", "%K qui croise %D marque un changement d'élan à court terme.".into()),
    ];
    let kdv = classic::stochastic_n(b, len, p.len("smooth"), p.len("d"));
    kd("le stochastique", len + p.len("smooth") + p.len("d"), kdv, how)
}

fn mfi(b: &[Bar], vals: &[f64]) -> Blocks {
    let p = Inputs { params: &MFI_PARAMS, vals };
    let len = p.len("len");
    let mut how = vec![
        s("how_flow", "Le MFI est un RSI qui tient compte du volume : il compare l'argent qui entre sur les bougies en hausse à l'argent qui sort sur les bougies en baisse.".into()),
        s("how_zones", "Au-dessus de 80, le flux entrant domine fortement (surachat) ; sous 20, le flux sortant (survente).".into()),
    ];
    let m = textbook::mfi_n(b, len);
    let Some((prev, v)) = last_two(&m) else {
        return too_few(&format!("le MFI {len}"), len + 2, how);
    };
    let (z, change_zone) = zone(v, prev, HI_LO_80_20);
    let flow = s(
        "flow",
        format!(
            "MFI {} : l'argent {} domine sur les {len} dernières bougies.",
            side_of(v > 50.0, "50"),
            if v > 50.0 { "qui entre" } else { "qui sort" }
        ),
    );
    let mut change = vec![s(
        "change_50",
        format!("Un passage {} montrerait que le flux s'inverse.", side_of(v <= 50.0, "50")),
    )];
    change.extend(change_zone);
    let mut now = vec![moves(&format!("MFI {len}"), v, prev, 1), flow, z];
    let div = divergence::find(b, &m);
    if let Some(d) = &div {
        let (now_d, how_d) = divergence::sentences(d, &format!("le MFI {len}"), price, |x| num(x, 1));
        now.push(now_d);
        how.push(how_d);
    }
    let sense = Some(Sense {
        lean: lean_of(v - 50.0),
        note: divergence::note(zone_note(v, HI_LO_80_20), div.as_ref()),
        value: Some(num(v, 1)),
    });
    let blocks = vec![block(NOW, now), block(HOW, how), block(CHANGE, change)];
    (blocks, sense)
}

fn cci(b: &[Bar], vals: &[f64]) -> Blocks {
    let p = Inputs { params: &CCI_PARAMS, vals };
    let len = p.len("len");
    let how = vec![
        s(
            "how_scale",
            format!("Le CCI mesure l'écart du prix typique à sa moyenne des {len} dernières bougies, rapporté à sa dispersion habituelle."),
        ),
        s(
            "how_zones",
            "Au-dessus de +100, le prix est loin au-dessus de sa moyenne (élan fort) ; sous −100, loin en dessous.".into(),
        ),
    ];
    let c = textbook::cci_n(b, len);
    let Some((prev, v)) = last_two(&c) else {
        return too_few(&format!("le CCI {len}"), len + 1, how);
    };
    let z = if v >= 100.0 {
        "Au-dessus de +100 : le prix est nettement au-dessus de sa moyenne."
    } else if v <= -100.0 {
        "Sous −100 : le prix est nettement sous sa moyenne."
    } else if v >= 0.0 {
        "Entre 0 et +100 : le prix est un peu au-dessus de sa moyenne."
    } else {
        "Entre −100 et 0 : le prix est un peu sous sa moyenne."
    };
    let change = vec![s("change_0", format!("Un passage {} changerait la lecture.", side_of(v < 0.0, "zéro")))];
    let note = (v.abs() >= 100.0).then_some("loin de sa moyenne");
    let blocks = vec![
        block(NOW, vec![moves(&format!("CCI {len}"), v, prev, 0), s("zone", z.into())]),
        block(HOW, how),
        block(CHANGE, change),
    ];
    (
        blocks,
        Some(Sense {
            lean: lean_of(v),
            note,
            value: Some(num(v, 0)),
        }),
    )
}

fn williams(b: &[Bar], vals: &[f64]) -> Blocks {
    let p = Inputs {
        params: &WILLIAMS_PARAMS,
        vals,
    };
    let len = p.len("len");
    let how = vec![
        s(
            "how_scale",
            format!("Il place la clôture entre le plus haut (0) et le plus bas (−100) des {len} dernières bougies."),
        ),
        s(
            "how_zones",
            "Au-dessus de −20, le prix clôture près de ses plus hauts (surachat) ; sous −80, près de ses plus bas (survente).".into(),
        ),
    ];
    let w = textbook::williams_r_n(b, len);
    let Some((prev, v)) = last_two(&w) else {
        return too_few(&format!("le Williams %R {len}"), len, how);
    };
    let (z, change_zone) = zone(v, prev, (-20.0, -80.0));
    let change: Vec<Sentence> = change_zone
        .into_iter()
        .chain([s("change_50", format!("Un passage {} changerait la lecture.", side_of(v <= -50.0, "−50")))])
        .collect();
    let now = vec![moves(&format!("Williams %R {len}"), v, prev, 1), z];
    let sense = Some(Sense {
        lean: lean_of(v + 50.0),
        note: zone_note(v, (-20.0, -80.0)),
        value: Some(num(v, 1)),
    });
    (vec![block(NOW, now), block(HOW, how), block(CHANGE, change)], sense)
}

fn aroon(b: &[Bar], vals: &[f64]) -> Blocks {
    let p = Inputs { params: &AROON_PARAMS, vals };
    let len = p.len("len");
    let how = vec![
        s("how_scale", format!("Aroon haut vaut 100 quand le plus haut des {len} dernières bougies vient d'être fait, et baisse avec le temps ; Aroon bas, pareil pour le plus bas.")),
        s("how_zones", "Aroon haut au-dessus de 70 et Aroon bas sous 30 : tendance haussière nette ; l'inverse : baissière.".into()),
    ];
    let (u, d) = textbook::aroon_n(b, len);
    let (Some((_, uv)), Some((_, dv))) = (last_two(&u), last_two(&d)) else {
        return too_few(&format!("l'Aroon {len}"), len + 1, how);
    };
    let reading = if uv >= 70.0 && dv <= 30.0 {
        "tendance haussière nette"
    } else if dv >= 70.0 && uv <= 30.0 {
        "tendance baissière nette"
    } else {
        "pas de tendance nette"
    };
    let now = vec![s("value", format!("Aroon haut à {}, Aroon bas à {} : {reading}.", num(uv, 0), num(dv, 0)))];
    let change = vec![s(
        "change_cross",
        format!(
            "Un croisement d'Aroon {} par Aroon {} changerait la lecture.",
            if uv >= dv { "haut" } else { "bas" },
            if uv >= dv { "bas" } else { "haut" }
        ),
    )];
    let lean = if uv >= 70.0 && dv <= 30.0 {
        Lean::Up
    } else if dv >= 70.0 && uv <= 30.0 {
        Lean::Down
    } else {
        Lean::Neutral
    };
    let value = format!("haut {}, bas {}", num(uv, 0), num(dv, 0));
    (vec![block(NOW, now), block(HOW, how), block(CHANGE, change)], valued(sense(lean), value))
}

// — volatility and volume —

fn atr(b: &[Bar], vals: &[f64]) -> Blocks {
    let p = Inputs { params: &ATR_PARAMS, vals };
    let len = p.len("len");
    let how = vec![
        s(
            "how_size",
            format!("L'ATR {len} est l'amplitude moyenne d'une bougie, en prix : il mesure l'agitation, pas le sens."),
        ),
        s(
            "how_use",
            "Un ATR qui monte : les bougies s'allongent ; qui baisse : le marché se calme.".into(),
        ),
    ];
    let a = classic::atr_n(b, len);
    let (Some((_, v)), Some(last)) = (last_two(&a), b.last()) else {
        return too_few(&format!("l'ATR {len}"), len + 1, how);
    };
    let mut now = vec![s(
        "value",
        format!("ATR {len} à {}, soit {} du prix.", signed(v, last.close), num(v / last.close * 100.0, 2) + " %"),
    )];
    let before = a.len().checked_sub(1 + LOOKBACK).map(|i| a[i]).filter(|x| x.is_finite());
    if let Some(v0) = before {
        now.push(s(
            "trend",
            format!(
                "Il {} depuis {} (il était à {}).",
                if v >= v0 { "monte" } else { "baisse" },
                n_bars(LOOKBACK),
                signed(v0, last.close)
            ),
        ));
    }
    let change = vec![s("change_trend", "Un ATR qui change de sens changerait la lecture de l'agitation.".into())];
    let note = before.map(|v0| if v >= v0 { "agitation en hausse" } else { "agitation en baisse" });
    (
        vec![block(NOW, now), block(HOW, how), block(CHANGE, change)],
        Some(Sense {
            lean: Lean::NoDirection,
            note,
            value: Some(num(v / last.close * 100.0, 2) + " % du prix"),
        }),
    )
}

fn rel_volume(b: &[Bar], vals: &[f64]) -> Blocks {
    let p = Inputs {
        params: &REL_VOLUME_PARAMS,
        vals,
    };
    let (len, pic) = (p.len("len"), p.get("pic"));
    let how = vec![
        s(
            "how_ratio",
            format!("Le volume de chaque bougie divisé par la moyenne des {len} dernières : 1 = un volume habituel."),
        ),
        s(
            "how_peak",
            format!(
                "À partir de {} fois la moyenne, la bougie est marquée comme un pic : un mouvement suivi par beaucoup de monde.",
                num(pic, 1)
            ),
        ),
    ];
    let r = classic::relative_volume_n(b, len);
    let Some((_, v)) = last_two(&r) else {
        return too_few(&format!("le volume relatif {len}"), len, how);
    };
    let level = if v >= pic {
        format!("un pic (au moins {} fois la moyenne)", num(pic, 1))
    } else if v >= 1.0 {
        "au-dessus de la moyenne".to_string()
    } else {
        "sous la moyenne".to_string()
    };
    let above = r.iter().rev().take(LOOKBACK).filter(|x| **x >= 1.0).count();
    let now = vec![
        s("value", format!("Volume de la dernière bougie : {} fois la moyenne, {level}.", num(v, 2))),
        s(
            "recent",
            format!("{above} des {} dernières bougies ont un volume au-dessus de la moyenne.", LOOKBACK),
        ),
    ];
    let change = vec![s(
        "change_volume",
        "Un mouvement de prix sans volume est moins solide ; un retour du volume au-dessus de 1 le confirmerait.".into(),
    )];
    let note = if v >= pic {
        "pic de volume"
    } else if v >= 1.0 {
        "volume au-dessus de la moyenne"
    } else {
        "volume sous la moyenne"
    };
    (
        vec![block(NOW, now), block(HOW, how), block(CHANGE, change)],
        Some(Sense {
            lean: Lean::NoDirection,
            note: Some(note),
            value: Some(format!("{} fois la moyenne", num(v, 2))),
        }),
    )
}

fn obv(b: &[Bar], vals: &[f64]) -> Blocks {
    let p = Inputs { params: &OBV_PARAMS, vals };
    let sig = p.get("sig").round().max(0.0) as usize;
    let how = vec![
        s(
            "how_sum",
            "L'OBV ajoute le volume des bougies qui clôturent en hausse et retire celui des bougies en baisse : il suit le volume qui accompagne le mouvement."
                .into(),
        ),
        s(
            "how_compare",
            "Seul son sens compte : un OBV qui monte avec le prix confirme la hausse ; qui baisse pendant que le prix monte, il la contredit.".into(),
        ),
    ];
    let o = textbook::obv(b);
    let (Some(&v), Some(i0)) = (o.last(), o.len().checked_sub(1 + 2 * LOOKBACK)) else {
        return too_few("l'OBV", 2 * LOOKBACK + 1, how);
    };
    let (o_up, p_up) = (v >= o[i0], b[b.len() - 1].close >= b[i0].close);
    let mut now = vec![s(
        "trend",
        format!(
            "Sur {} : OBV en {}, prix en {} ({}).",
            n_bars(2 * LOOKBACK),
            if o_up { "hausse" } else { "baisse" },
            if p_up { "hausse" } else { "baisse" },
            if o_up == p_up {
                "le volume confirme le mouvement"
            } else {
                "le volume ne confirme pas le mouvement"
            }
        ),
    )];
    if sig >= 1 {
        if let Some(m) = argos_script::ta::sma(&o, sig).last().copied().filter(|x| x.is_finite()) {
            now.push(s("average", format!("OBV {} de {sig} bougies.", side_of(v > m, "sa moyenne"))));
        }
    }
    let change = vec![s("change_trend", "Un OBV qui part dans l'autre sens que le prix changerait la lecture.".into())];
    (
        vec![block(NOW, now), block(HOW, how), block(CHANGE, change)],
        sense(if o_up { Lean::Up } else { Lean::Down }),
    )
}

// — trend —

fn dmi(b: &[Bar], vals: &[f64]) -> Blocks {
    let p = Inputs { params: &DMI_PARAMS, vals };
    let (len, seuil) = (p.len("len"), p.get("seuil"));
    let how = vec![
        s(
            "how_di",
            "+DI mesure la force des hausses, −DI celle des baisses : la plus haute des deux donne le sens.".into(),
        ),
        s(
            "how_adx",
            format!(
                "L'ADX mesure la force de la tendance, sans le sens : au-dessus de {}, la tendance est forte.",
                num(seuil, 0)
            ),
        ),
    ];
    let (pl, mi, adx) = textbook::dmi_n(b, len, p.len("adx"));
    let (Some((_, pv)), Some((_, mv)), Some((a0, av))) = (last_two(&pl), last_two(&mi), last_two(&adx)) else {
        return too_few("le DMI", len + p.len("adx") + 1, how);
    };
    let now = vec![
        s(
            "di",
            format!(
                "+DI à {} {} −DI ({}) : les {} dominent.",
                num(pv, 1),
                if pv > mv { "au-dessus de" } else { "sous" },
                num(mv, 1),
                if pv > mv { "hausses" } else { "baisses" }
            ),
        ),
        s(
            "adx",
            format!(
                "ADX à {} ({} depuis la bougie d'avant) : {}.",
                num(av, 1),
                if av >= a0 { "en hausse" } else { "en baisse" },
                if av >= seuil {
                    format!("tendance forte (au-dessus de {})", num(seuil, 0))
                } else {
                    format!("pas de tendance forte (sous {})", num(seuil, 0))
                }
            ),
        ),
    ];
    let change = vec![
        s("change_di", "Un croisement de +DI et −DI changerait le sens de la lecture.".into()),
        s(
            "change_adx",
            format!("Un ADX qui passe {} changerait la lecture de la force.", side_of(av < seuil, &num(seuil, 0))),
        ),
    ];
    let sense = if av >= seuil {
        sense(lean_of(pv - mv))
    } else {
        Some(Sense {
            lean: Lean::Neutral,
            note: Some("sans tendance forte"),
            value: None,
        })
    };
    (
        vec![block(NOW, now), block(HOW, how), block(CHANGE, change)],
        valued(sense, format!("ADX {}", num(av, 1))),
    )
}

/// A trend line that sits under the price in an up trend and over it in a down trend (Supertrend, Parabolic SAR).
fn trend_line(name: &str, b: &[Bar], line: &[f64], up: &[bool], how: Vec<Sentence>, needed: usize) -> Blocks {
    let (Some((_, v)), Some(last)) = (last_two(line), b.last()) else {
        return too_few(name, needed, how);
    };
    let c = last.close;
    let u = up.last().copied().unwrap_or(c > v);
    let flip = last_flip(up).map_or_else(
        || "aucun retournement dans l'historique chargé".to_string(),
        |k| format!("retourné il y a {}", n_bars(k)),
    );
    let now = vec![
        s(
            "side",
            format!(
                "{} {} : ligne à {}, {} la clôture ({}), écart {}.",
                name,
                if u { "haussier" } else { "baissier" },
                price(v),
                if u { "sous" } else { "au-dessus de" },
                price(c),
                percent((c - v) / c * 100.0)
            ),
        ),
        s("flip", format!("Dernier retournement : {flip}.")),
    ];
    let change = vec![s(
        "change_side",
        format!(
            "Une clôture {} ({}) le ferait passer {}.",
            if u { "sous la ligne" } else { "au-dessus de la ligne" },
            price(v),
            if u { "baissier" } else { "haussier" }
        ),
    )];
    (
        vec![block(NOW, now), block(HOW, how), block(CHANGE, change)],
        valued(sense(if u { Lean::Up } else { Lean::Down }), format!("écart {}", percent((c - v) / c * 100.0))),
    )
}

fn supertrend(b: &[Bar], vals: &[f64]) -> Blocks {
    let p = Inputs {
        params: &SUPERTREND_PARAMS,
        vals,
    };
    let (a, mult) = (p.len("atr"), p.get("mult"));
    let how = vec![
        s(
            "how_line",
            format!(
                "La ligne suit le prix à {} ATR {a} de distance et ne recule pas tant que la tendance tient.",
                num(mult, 1)
            ),
        ),
        s(
            "how_flip",
            "Sous le prix, elle est haussière ; au-dessus, baissière. Elle se retourne quand une clôture la traverse.".into(),
        ),
    ];
    let (line, up) = textbook::supertrend_n(b, a, mult);
    trend_line("Supertrend", b, &line, &up, how, a + 1)
}

fn psar(b: &[Bar], vals: &[f64]) -> Blocks {
    let p = Inputs { params: &PSAR_PARAMS, vals };
    let how = vec![
        s(
            "how_dots",
            "Les points du SAR parabolique se placent sous le prix en hausse et au-dessus en baisse, et s'en rapprochent un peu plus à chaque bougie.".into(),
        ),
        s(
            "how_flip",
            "Quand le prix touche les points, ils passent de l'autre côté : le sens se retourne.".into(),
        ),
    ];
    let line = textbook::psar_n(b, p.get("start"), p.get("step"), p.get("max"));
    let up: Vec<bool> = line.iter().zip(b).filter(|(v, _)| v.is_finite()).map(|(v, x)| *v < x.close).collect();
    trend_line("SAR parabolique", b, &line, &up, how, 3)
}

// — channels —

fn channel_blocks(name: &str, short: &str, b: &[Bar], uml: (&[f64], &[f64], &[f64]), how: Vec<Sentence>, needed: usize) -> Blocks {
    let Some((now, change, sense)) = b.last().and_then(|x| channel(short, x.close, uml)) else {
        return too_few(name, needed, how);
    };
    (vec![block(NOW, now), block(HOW, how), block(CHANGE, vec![change])], Some(sense))
}

/// TradingView's Bollinger Bands: the bands its script draws (`argos_script::lang::run` on the built-in with the
/// screen's settings, no second computation), read where they are drawn on the last closed bar (Offset).
fn bollinger(b: &[Bar], received: &[(&str, f64)]) -> Blocks {
    let name = "les bandes de Bollinger";
    let Some(sc) = super::averages::Script::new("Bollinger", received) else {
        return (
            vec![block(
                NOW,
                vec![s("missing", "Le script des bandes de Bollinger n'est pas dans ce catalogue.".into())],
            )],
            None,
        );
    };
    let len = sc.setting("Length").max(1.0).round() as usize;
    let mult = sc.setting("StdDev");
    let kind = BOLLINGER_TYPES.get(sc.setting("Basis MA Type").max(0.0) as usize).copied().unwrap_or("simple");
    let src = super::averages::source_name(sc.setting("Source")).map_or(String::new(), |x| format!(" ({x})"));
    let how = vec![
        s(
            "how_bands",
            format!(
                "Moyenne {kind} {len}{src} au milieu, bandes à {} écarts-types : le prix reste dedans la plupart du temps.",
                num(mult, 1)
            ),
        ),
        s(
            "how_width",
            "Des bandes qui se resserrent montrent un marché calme, souvent avant un mouvement ; un prix collé à une bande montre une tendance forte.".into(),
        ),
    ];
    let Some(out) = sc.run(b) else {
        return too_few(name, len, how);
    };
    let line = |t: &str| super::averages::plot(&out, t);
    let (Some(u), Some(m), Some(l)) = (line("Upper"), line("Basis"), line("Lower")) else {
        return too_few(name, len, how);
    };
    // (shifted right, a band drawn on the last bar is its value `offset` bars earlier; shifted left, the bands end
    // before the last bar: their last values are read)
    let offset = out.plots.iter().find(|p| p.title == "Basis").map_or(0, |p| p.offset);
    let k = offset.max(0) as usize;
    let drawn = |v: &[f64]| -> Vec<f64> { (0..v.len()).map(|i| i.checked_sub(k).map_or(f64::NAN, |j| v[j])).collect() };
    let (u, m, l) = (drawn(u), drawn(m), drawn(l));
    let mut r = channel_blocks(name, "canal de Bollinger", b, (&u, &m, &l), how, len + k);
    if offset != 0 && r.1.is_some() {
        if let Some(first) = r.0.first_mut() {
            first.sentences.push(s(
                "offset",
                format!(
                    "Les bandes sont décalées de {} vers la {} à l'écran (réglage Offset) : {}.",
                    n_bars(offset.unsigned_abs() as usize),
                    if offset > 0 { "droite" } else { "gauche" },
                    if offset > 0 {
                        "valeurs lues là où elles sont dessinées sur la dernière bougie".to_string()
                    } else {
                        format!(
                            "valeurs de la dernière bougie calculée, dessinées {} plus tôt (rien n'est dessiné sur la dernière bougie)",
                            n_bars(offset.unsigned_abs() as usize)
                        )
                    }
                ),
            ));
        }
    }
    r
}

fn keltner(b: &[Bar], vals: &[f64]) -> Blocks {
    let p = Inputs { params: &KELTNER_PARAMS, vals };
    let (len, a, mult) = (p.len("len"), p.len("atr"), p.get("mult"));
    let how = vec![
        s(
            "how_bands",
            format!(
                "EMA {len} au milieu, bandes à {} ATR {a} : un canal qui suit l'agitation du marché.",
                num(mult, 1)
            ),
        ),
        s("how_out", "Une clôture hors du canal montre un mouvement plus fort que d'habitude.".into()),
    ];
    let (u, m, l) = textbook::keltner_n(b, len, a, mult);
    channel_blocks("le canal de Keltner", "canal de Keltner", b, (&u, &m, &l), how, len.max(a))
}

fn donchian(b: &[Bar], vals: &[f64]) -> Blocks {
    let p = Inputs {
        params: &DONCHIAN_PARAMS,
        vals,
    };
    let len = p.len("len");
    let how = vec![
        s("how_bands", format!("Le plus haut et le plus bas des {len} dernières bougies, et leur milieu.")),
        s(
            "how_break",
            "Une bougie qui touche le haut fait un nouveau plus haut sur la période ; le bas, un nouveau plus bas.".into(),
        ),
    ];
    let (u, m, l) = textbook::donchian_n(b, len);
    let mut out = channel_blocks("le canal de Donchian", "canal de Donchian", b, (&u, &m, &l), how, len);
    if let (Some(x), Some(&uv), Some(&lv)) = (b.last(), u.last(), l.last()) {
        let touch = if x.high >= uv {
            Some(format!("La dernière bougie touche le plus haut des {len} dernières bougies."))
        } else if x.low <= lv {
            Some(format!("La dernière bougie touche le plus bas des {len} dernières bougies."))
        } else {
            None
        };
        if let (Some(t), Some(first)) = (touch, out.0.first_mut().filter(|bl| bl.title == NOW && uv.is_finite())) {
            first.sentences.push(s("touch", t));
        }
    }
    out
}

/// Marks older than this, in bars, are not « recent ».
pub(super) const RECENT: usize = 50;
/// Under this, both strengths of the Bollinger Oscillator are weak and none « dominates ».
const WEAK: f64 = 10.0;

const BANDS_ROLES: [(&str, &str); 7] = [
    ("len", "Longueur de la moyenne du milieu et de l'écart-type."),
    ("mult", "Écart des bandes, en écarts-types : plus grand, le prix les touche plus rarement."),
    ("ma", "Moyenne du milieu : simple (SMA) ou exponentielle (EMA)."),
    ("sq", "Repère le squeeze (bandes au plus serré)."),
    ("sqlen", "Nombre de bougies où l'on cherche la largeur la plus faible."),
    ("touch", "Marque les clôtures qui sortent des bandes (▲BB / ▼BB)."),
    ("showTable", "Affiche le tableau résumé ; Argos AI en reprend les lignes."),
];
const BANDS_OSC_ROLES: [(&str, &str); 4] = [
    ("len", "Nombre de bougies de la bande et du lissage."),
    ("mult", "Demi-largeur de la bande, en écarts-types."),
    ("sig", "Marque les passages au-dessus de 50."),
    ("showTable", "Affiche le tableau résumé ; Argos AI en reprend les lignes."),
];

/// « sur la dernière bougie » or « il y a N bougies ».
fn ago(n: usize, i: usize) -> String {
    if i + 1 == n {
        "sur la dernière bougie".to_string()
    } else {
        format!("il y a {}", n_bars(n - 1 - i))
    }
}

/// Argos Bollinger Bands: the bands, the squeeze and the ▲BB / ▼BB marks the chart draws (`classic::argos_bands`).
fn bands(b: &[Bar], vals: &[f64]) -> Blocks {
    let p = Inputs { params: &BANDS_PARAMS, vals };
    let (len, mult) = (p.len("len"), p.get("mult"));
    let mut how = vec![s(
        "how_bands",
        format!(
            "Moyenne {} {len} au milieu, bandes à {} écarts-types : le prix reste dedans la plupart du temps.",
            if p.pick("ma") == 1 { "exponentielle (EMA)" } else { "simple" },
            num(mult, 1)
        ),
    )];
    if p.on("sq") {
        how.push(s(
            "how_squeeze",
            format!(
                "« Squeeze » : la largeur des bandes est au plus bas (à 5 % près) des {} dernières bougies, un marché calme, souvent avant un mouvement.",
                p.len("sqlen")
            ),
        ));
    }
    if p.on("touch") {
        how.push(s(
            "how_touch",
            "« ▲BB » : une clôture qui sort au-dessus de la bande haute ; « ▼BB » : sous la bande basse.".into(),
        ));
    }
    let a = classic::argos_bands(b, vals);
    let (mut blocks, sense) = channel_blocks(
        "les bandes de Bollinger",
        "canal de Bollinger",
        b,
        (&a.bands.upper, &a.bands.basis, &a.bands.lower),
        how,
        len,
    );
    let n = b.len();
    let mut more = vec![];
    if p.on("sq") && n > 0 {
        let start = (1..n).rev().find(|&i| a.squeeze[i] && !a.squeeze[i - 1]);
        if a.squeeze[n - 1] {
            if let Some(i) = start {
                more.push(s("squeeze", format!("Squeeze en cours : repère « Squeeze » {}.", ago(n, i))));
            }
        } else if let Some(i) = start.filter(|&i| n - 1 - i <= RECENT) {
            more.push(s("squeeze", format!("Dernier repère « Squeeze » {} ; il est fini.", ago(n, i))));
        }
    }
    if p.on("touch") {
        let (u, l) = (&a.bands.upper, &a.bands.lower);
        let last = (1..n).rev().take(RECENT + 1).find_map(|i| {
            let (c, c0) = (b[i].close, b[i - 1].close);
            if c > u[i] && c0 <= u[i - 1] {
                Some((i, "« ▲BB » (clôture sortie au-dessus de la bande haute)"))
            } else if c < l[i] && c0 >= l[i - 1] {
                Some((i, "« ▼BB » (clôture sortie sous la bande basse)"))
            } else {
                None
            }
        });
        if let Some((i, what)) = last {
            more.push(s("touch", format!("Dernière marque {what} {}.", ago(n, i))));
        }
    }
    if p.on("showTable") && n > 0 {
        // the drawn « Squeeze » cell adds a forecast; the reading keeps only the fact, Oui or Non
        let squeeze = if a.squeeze[n - 1] { "Oui" } else { "Non" };
        let rows: Vec<(&str, String)> = classic::bands_tableau(&a, b)
            .into_iter()
            .map(|(k, v)| (k, if k == "Squeeze" { squeeze.to_string() } else { v }))
            .collect();
        more.extend(table(&rows));
    }
    if let Some(first) = blocks.first_mut().filter(|bl| bl.title == NOW && sense.is_some()) {
        first.sentences.extend(more);
    }
    (blocks, sense)
}

/// Argos Bollinger Oscillator: the bullish and bearish strengths, 0 to 100, and the marks at 50
/// (`classic::bollinger_oscillator`).
fn bands_osc(b: &[Bar], vals: &[f64]) -> Blocks {
    let p = Inputs {
        params: &BANDS_OSC_PARAMS,
        vals,
    };
    let (len, mult) = (p.len("len").max(2), p.get("mult").max(0.01));
    let mut how = vec![
        s(
            "how_strength",
            format!(
                "Force haussière : de combien la clôture sort au-dessus de sa bande (moyenne {len} + {} écarts-types), lissée sur {len} bougies et mise de 0 à 100 ; 0 tant que la clôture reste dans la bande. La force baissière fait de même sous la bande.",
                num(mult, 1)
            ),
        ),
        s(
            "how_50",
            "À 50, la clôture dépasse en moyenne la bande d'une demi-largeur de plus : une cassure nette. Tant que les deux forces restent sous 10, aucune ne domine.".into(),
        ),
    ];
    if p.on("sig") {
        how.push(s(
            "how_sig",
            "Les flèches marquent une force qui passe au-dessus de 50 : vers le haut pour la force haussière, vers le bas pour la force baissière.".into(),
        ));
    }
    let (bull, bear) = classic::bollinger_oscillator(b, vals);
    let (Some((_, u)), Some((_, d))) = (last_two(&bull), last_two(&bear)) else {
        return too_few("l'oscillateur de Bollinger", len + 1, how);
    };
    let side = if u.max(d) < WEAK {
        "Les deux forces sont faibles (sous 10), la clôture sort à peine de sa bande"
    } else if u > d {
        "La force haussière domine"
    } else if d > u {
        "La force baissière domine"
    } else {
        "Aucune des deux forces ne domine"
    };
    let mut now = vec![s(
        "strengths",
        format!(
            "Force haussière à {}, force baissière à {} (sur 100) : {}.",
            num(u, 1),
            num(d, 1),
            side.to_lowercase()
        ),
    )];
    let above: Vec<&str> = [(u, "haussière"), (d, "baissière")].iter().filter(|x| x.0 > 50.0).map(|x| x.1).collect();
    if let [w] = above.as_slice() {
        now.push(s("above_50", format!("La force {w} est au-dessus de 50 : cassure nette de la bande.")));
    }
    let n = b.len();
    if p.on("sig") {
        let cross = |v: &[f64], i: usize| v[i - 1] <= 50.0 && v[i] > 50.0;
        let last = (1..n).rev().take(RECENT + 1).find_map(|i| {
            if cross(&bull, i) {
                Some((i, "flèche vers le haut (la force haussière passe au-dessus de 50)"))
            } else if cross(&bear, i) {
                Some((i, "flèche vers le bas (la force baissière passe au-dessus de 50)"))
            } else {
                None
            }
        });
        if let Some((i, what)) = last {
            now.push(s("sig", format!("Dernière marque : {what} {}.", ago(n, i))));
        }
    }
    if p.on("showTable") {
        now.extend(table(&classic::bands_osc_tableau(&bull, &bear)));
    }
    let change = vec![s(
        "change",
        "Une force qui passe 50, ou l'autre force qui reprend le dessus, changerait la lecture.".into(),
    )];
    let sense = Some(Sense {
        lean: if u.max(d) < WEAK { Lean::Neutral } else { lean_of(u - d) },
        note: (u.max(d) > 50.0).then_some(if u > d { "cassure nette vers le haut" } else { "cassure nette vers le bas" }),
        value: Some(format!("{} / {}", num(u, 1), num(d, 1))),
    });
    (vec![block(NOW, now), block(HOW, how), block(CHANGE, change)], sense)
}

const ICHIMOKU_ROLES: [(&str, &str); 4] = [
    (
        "tenkan",
        "Tenkan-sen : le milieu du plus haut et du plus bas sur peu de bougies (ligne rapide).",
    ),
    ("kijun", "Kijun-sen : le même milieu sur plus de bougies (ligne lente)."),
    ("senkou", "Nombre de bougies du Senkou B, le bord lent du nuage."),
    (
        "shift",
        "Décalage du nuage vers l'avant et du Chikou vers l'arrière, comme le « Lagging Span » de TradingView : 26 les déplace de 25 bougies.",
    ),
];

/// Ichimoku: the close against the cloud the chart draws on the last bar, Tenkan against Kijun, and the Chikou (the
/// close moved back) against the price of then (`textbook::ichimoku_n`).
fn ichimoku(b: &[Bar], vals: &[f64]) -> Blocks {
    let p = Inputs {
        params: &ICHIMOKU_PARAMS,
        vals,
    };
    let (t, k, sb) = (p.len("tenkan"), p.len("kijun"), p.len("senkou"));
    // (the setting is TradingView's « Lagging Span » 26: it moves the cloud and the Chikou by 25 bars)
    let dec = textbook::ichimoku_offset(p.get("shift").round().max(0.0) as usize);
    let how = vec![
        s(
            "how_cloud",
            format!(
                "Le nuage va de Senkou A (milieu de Tenkan et Kijun) à Senkou B (milieu du plus haut et du plus bas des {sb} dernières bougies), tous deux calculés {} plus tôt. Au-dessus du nuage, le marché est en tendance haussière ; dessous, baissière ; dedans, sans direction claire.",
                n_bars(dec)
            ),
        ),
        s(
            "how_lines",
            format!("Tenkan ({t}) et Kijun ({k}) : le milieu du plus haut et du plus bas sur {t} et sur {k} bougies. Tenkan au-dessus de Kijun : l'élan récent est haussier."),
        ),
        s(
            "how_chikou",
            format!("Chikou : la clôture reportée {} en arrière, comparée au prix d'alors.", n_bars(dec)),
        ),
    ];
    let i = textbook::ichimoku_n(b, t, k, sb, p.get("shift").round().max(0.0) as usize);
    let n = b.len();
    let at = |v: &[f64], j: usize| v.get(j).copied().filter(|x| x.is_finite());
    let (Some(tv), Some(kv), Some(av), Some(bv)) = (
        n.checked_sub(1).and_then(|j| at(&i.tenkan, j)),
        n.checked_sub(1).and_then(|j| at(&i.kijun, j)),
        n.checked_sub(1).and_then(|j| at(&i.span_a, j)),
        n.checked_sub(1).and_then(|j| at(&i.span_b, j)),
    ) else {
        return too_few("l'Ichimoku", sb.max(k).max(t) + dec, how);
    };
    let c = b[n - 1].close;
    let (top, bot) = (av.max(bv), av.min(bv));
    let (place, lean, note) = if c > top {
        (format!("au-dessus du nuage (haut à {})", price(top)), Lean::Up, "au-dessus du nuage")
    } else if c < bot {
        (format!("sous le nuage (bas à {})", price(bot)), Lean::Down, "sous le nuage")
    } else {
        (format!("dans le nuage ({} à {})", price(bot), price(top)), Lean::Neutral, "dans le nuage")
    };
    let mut now = vec![
        s("cloud", format!("Clôture à {} : {place}.", price(c))),
        s(
            "spans",
            format!(
                "Senkou A à {}, {} Senkou B à {}.",
                price(av),
                if av >= bv { "au-dessus de" } else { "sous" },
                price(bv)
            ),
        ),
    ];
    // the last Tenkan / Kijun cross
    // (a cross: Tenkan passes to the other side of Kijun; touching it is not a cross)
    let mut side: Option<bool> = None;
    let mut cross = None;
    for j in 0..n {
        let d = i.tenkan[j] - i.kijun[j];
        if d.is_finite() && d != 0.0 {
            if side.is_some_and(|s| s != (d > 0.0)) {
                cross = Some(j);
            }
            side = Some(d > 0.0);
        }
    }
    let cross = cross.filter(|&j| n - 1 - j <= RECENT);
    let tk = format!(
        "Tenkan à {}, {} la Kijun à {}",
        price(tv),
        if tv > kv {
            "au-dessus de"
        } else if tv < kv {
            "sous"
        } else {
            "au niveau de"
        },
        price(kv)
    );
    now.push(s(
        "tenkan_kijun",
        match cross {
            Some(j) => format!(
                "{tk} ; elles se sont croisées {}.",
                if j + 1 == n {
                    "sur la dernière bougie".to_string()
                } else {
                    format!("il y a {}", n_bars(n - 1 - j))
                }
            ),
            None => format!("{tk}."),
        },
    ));
    if let Some(then) = n.checked_sub(1 + dec).filter(|_| dec > 0).map(|j| b[j].close) {
        now.push(s(
            "chikou",
            format!(
                "Chikou : la clôture actuelle est {} de la clôture d'il y a {} ({}).",
                if c > then {
                    "au-dessus"
                } else if c < then {
                    "en dessous"
                } else {
                    "au niveau"
                },
                n_bars(dec),
                price(then)
            ),
        ));
    }
    let change = vec![s(
        "change",
        match lean {
            Lean::Up => format!("Une clôture qui entre dans le nuage (sous {}) changerait la lecture.", price(top)),
            Lean::Down => format!("Une clôture qui entre dans le nuage (au-dessus de {}) changerait la lecture.", price(bot)),
            _ => format!(
                "Une clôture qui sort du nuage (au-dessus de {} ou sous {}) changerait la lecture.",
                price(top),
                price(bot)
            ),
        },
    )];
    let sense = Some(Sense {
        lean,
        note: Some(note),
        value: None,
    });
    (vec![block(NOW, now), block(HOW, how), block(CHANGE, change)], sense)
}
