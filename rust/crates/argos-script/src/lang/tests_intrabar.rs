//! 🔬 `request.security_lower_tf` (editeur-pine-1), the output check of the editor (`has_output_calls`), the drawing
//! limits of the declaration and the chart's symbol in `syminfo`. Pure: synthetic bars, no network.

use super::*;
use argos_core::Bar;
use std::sync::Arc;

/// `n` one-minute bars from `t0` (seconds): a deterministic random walk around `p0`, volumes 1..10.
pub(super) fn minutes(t0: i64, n: usize, p0: f64) -> Vec<Bar> {
    let mut x: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut alea = move || {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        (x >> 11) as f64 / (1u64 << 53) as f64
    };
    let mut p = p0;
    (0..n)
        .map(|k| {
            let o = p;
            let c = o * (1.0 + (alea() - 0.5) * 0.004);
            let (h, l) = (o.max(c) * (1.0 + alea() * 0.001), o.min(c) * (1.0 - alea() * 0.001));
            p = c;
            Bar {
                time: t0 + 60 * k as i64,
                open: o,
                high: h,
                low: l,
                close: c,
                volume: 1.0 + (alea() * 9.0).round(),
            }
        })
        .collect()
}

/// The bars grouped by `tf` seconds (by time, like the chart's own candles).
pub(super) fn grouper(b: &[Bar], tf: i64) -> Vec<Bar> {
    let mut out: Vec<Bar> = vec![];
    for x in b {
        let t = x.time.div_euclid(tf) * tf;
        match out.last_mut() {
            Some(g) if g.time == t => {
                g.high = g.high.max(x.high);
                g.low = g.low.min(x.low);
                g.close = x.close;
                g.volume += x.volume;
            }
            _ => out.push(Bar { time: t, ..*x }),
        }
    }
    out
}

pub(super) fn avec(intrabars: &[(i64, Vec<Bar>)]) -> RunOptions {
    RunOptions {
        intrabars: Arc::new(intrabars.iter().cloned().collect()),
        ..Default::default()
    }
}

fn plot<'o>(o: &'o Output, t: &str) -> &'o [f64] {
    &o.plots.iter().find(|p| p.title == t).unwrap_or_else(|| panic!("plot {t}")).values
}

const LTF: &str = r#"//@version=6
indicator("ltf")
c = request.security_lower_tf(syminfo.tickerid, "1", close)
plot(array.size(c), "n")
plot(array.size(c) > 0 ? array.get(c, 0) : na, "first")
plot(array.size(c) > 0 ? array.get(c, array.size(c) - 1) : na, "last")
[o, cl, v] = request.security_lower_tf(syminfo.tickerid, "1", [open, close, volume])
plot(array.sum(v), "vol")
plot(array.size(o) == array.size(cl) and array.size(cl) == array.size(v) ? 1 : 0, "same")
"#;

#[test]
fn intrabars_d_une_bougie_de_5_minutes() {
    let m = minutes(0, 100, 100.0);
    let bars = grouper(&m, 300);
    let out = run_with(LTF, &bars, 300, &Default::default(), &avec(&[(60, m.clone())])).unwrap();
    assert!(out.lower_tf_missing.is_empty());
    assert_eq!(out.lower_tf_asked, [60], "asked, and given");
    for (i, b) in bars.iter().enumerate() {
        assert_eq!(plot(&out, "n")[i], 5.0, "bar {i}");
        assert_eq!(plot(&out, "first")[i], m[5 * i].close, "bar {i}: in order");
        assert_eq!(plot(&out, "last")[i], b.close, "bar {i}: the last intrabar closes the bar");
        assert!((plot(&out, "vol")[i] - b.volume).abs() < 1e-9, "bar {i}: volumes add up");
        assert_eq!(plot(&out, "same")[i], 1.0);
    }
}

/// B-062: the fast path (the expression alone on each intrabar) runs no statement, and kept its values in a place
/// no count could see: its texts are now counted like the rest, under the budget of the script.
#[test]
fn les_valeurs_du_chemin_rapide_sont_comptees() {
    let m = minutes(1_700_000_000 - 1_700_000_000 % 300, 3000, 100.0);
    let b = grouper(&m, 300);
    let src = "//@version=6
indicator('t')
c = request.security_lower_tf(syminfo.tickerid, '1', str.repeat('x', 2000) + str.tostring(close))
plot(c.size())";
    let opts = avec(&[(60, m.clone())]);
    // (3 000 intrabars, a text of 2 000 characters each: 6 MB)
    memoire::budget_essai(2 << 20);
    let e = run_with(src, &b, 300, &HashMap::new(), &opts).unwrap_err();
    assert!(e.msg.contains("trop de mémoire") && e.line == 3, "{e}");
    memoire::budget_essai(64 << 20);
    let o = run_with(src, &b, 300, &HashMap::new(), &opts).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(o.plots[0].values[b.len() - 1], 5.0);
}

#[test]
fn intrabars_qui_commencent_en_cours_d_historique() {
    let m = minutes(0, 100, 100.0);
    let bars = grouper(&m, 300);
    // (from minute 52: chart bar 10 starts at minute 50, so it is only partly covered)
    let out = run_with(LTF, &bars, 300, &Default::default(), &avec(&[(60, m[52..].to_vec())])).unwrap();
    let n = plot(&out, "n");
    assert!(n[..=10].iter().all(|x| *x == 0.0), "before and the partly covered bar: empty, {n:?}");
    assert!(n[11..].iter().all(|x| *x == 5.0), "after: full, {n:?}");
}

#[test]
fn sans_intrabars_des_tableaux_vides_et_l_unite_manquante() {
    let m = minutes(0, 100, 100.0);
    let bars = grouper(&m, 300);
    let out = run_with(LTF, &bars, 300, &Default::default(), &RunOptions::default()).unwrap();
    assert!(plot(&out, "n").iter().all(|x| *x == 0.0));
    assert!(plot(&out, "same").iter().all(|x| *x == 1.0), "a tuple of three empty arrays");
    assert_eq!(out.lower_tf_missing, [60]);
    assert_eq!(out.lower_tf_asked, [60]);
    // (run: the same, no intrabar)
    assert_eq!(run(LTF, &bars, 300, &Default::default()).unwrap().lower_tf_missing, [60]);
}

#[test]
fn unite_egale_ou_plus_grande() {
    let m = minutes(0, 100, 100.0);
    let bars = grouper(&m, 300);
    let egale = "//@version=6\nindicator(\"e\")\nc = request.security_lower_tf(syminfo.tickerid, \"5\", close)\nplot(array.size(c), \"n\")\nplot(array.get(c, 0), \"c\")\n";
    let out = run(egale, &bars, 300, &Default::default()).unwrap();
    assert!(plot(&out, "n").iter().all(|x| *x == 1.0));
    assert_eq!(plot(&out, "c"), bars.iter().map(|b| b.close).collect::<Vec<_>>());
    assert!(out.lower_tf_missing.is_empty());
    let haute = "//@version=6\nindicator(\"h\")\n\nc = request.security_lower_tf(syminfo.tickerid, \"15\", close)\nplot(na(c) ? 1 : 0, \"na\")\n";
    let e = run(haute, &bars, 300, &Default::default()).unwrap_err();
    assert_eq!(e.line, 4);
    assert_eq!(e.msg, "request.security_lower_tf : l'unité demandée (15) est plus grande que celle de la chart");
    let ignoree = haute.replace("close)", "close, ignore_invalid_timeframe = true)");
    let out = run(&ignoree, &bars, 300, &Default::default()).unwrap();
    assert!(plot(&out, "na").iter().all(|x| *x == 1.0));
    let inconnue = haute.replace("\"15\"", "\"2X\"");
    assert_eq!(
        run(&inconnue, &bars, 300, &Default::default()).unwrap_err().msg,
        "request.security_lower_tf : unité de temps inconnue « 2X »"
    );
    let autre = haute.replace("syminfo.tickerid", "\"BINANCE:ETHUSDT\"");
    assert!(run(&autre, &bars, 300, &Default::default())
        .unwrap_err()
        .msg
        .starts_with("request.security_lower_tf : un autre symbole (BINANCE:ETHUSDT)"));
}

#[test]
fn chemin_rapide_et_chemin_complet_donnent_les_memes_tableaux() {
    let m = minutes(0, 200, 100.0);
    let bars = grouper(&m, 300);
    let src = |e: &str| {
        format!(
            "//@version=6\nindicator(\"p\")\nk = 1.0\nc = request.security_lower_tf(syminfo.tickerid, \"1\", {e})\nplot(array.size(c), \"n\")\nplot(array.sum(c), \"s\")\nplot(array.size(c) > 0 ? array.get(c, 0) : na, \"f\")\n"
        )
    };
    let o = avec(&[(60, m[7..].to_vec())]);
    let rapide = run_with(&src("close"), &bars, 300, &Default::default(), &o).unwrap();
    let complet = run_with(&src("close * k"), &bars, 300, &Default::default(), &o).unwrap();
    for t in ["n", "s", "f"] {
        let (a, b) = (plot(&rapide, t), plot(&complet, t));
        assert!(a.iter().zip(b).all(|(x, y)| x == y || (x.is_nan() && y.is_nan())), "{t}: {a:?} ≠ {b:?}");
    }
    // (memory inside the expression runs along the intrabars, on both paths)
    let r = run_with(&src("ta.sma(close, 3)"), &bars, 300, &Default::default(), &o).unwrap();
    let c = run_with(&src("ta.sma(close, 3) * k"), &bars, 300, &Default::default(), &o).unwrap();
    assert_eq!(plot(&r, "s")[5], plot(&c, "s")[5]);
    let attendu: f64 = (0..5).map(|j| (m[25 + j].close + m[24 + j].close + m[23 + j].close) / 3.0).sum();
    assert!((plot(&r, "s")[5] - attendu).abs() < 1e-9);
}

#[test]
fn la_bougie_en_cours_n_a_que_ses_intrabars_jusqu_ici() {
    let m = minutes(0, 52, 100.0);
    let bars = grouper(&m, 300);
    let out = run_with(LTF, &bars, 300, &Default::default(), &avec(&[(60, m.clone())])).unwrap();
    let n = plot(&out, "n");
    assert_eq!(n[n.len() - 1], 2.0);
    assert_eq!(plot(&out, "last")[n.len() - 1], m[51].close);
}

const VAGUE_TV: &str = include_str!("../../../../../pine/argos/argos-vague-tradingview.pine");

#[test]
fn argos_vague_tradingview_avec_et_sans_intrabars() {
    let m = minutes(1_700_000_400 - 1_700_000_400 % 3600, 600 * 60, 60_000.0);
    let bars = grouper(&m, 3600);
    assert_eq!(bars.len(), 600);
    let sans = run(VAGUE_TV, &bars, 3600, &Default::default()).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(sans.lower_tf_missing, [60], "the 1 h chart asks for 1-minute intrabars");
    let avec_ib = run_with(VAGUE_TV, &bars, 3600, &Default::default(), &avec(&[(60, m.clone())])).unwrap_or_else(|e| panic!("{e}"));
    assert!(avec_ib.lower_tf_missing.is_empty());
    for o in [&sans, &avec_ib] {
        assert!(plot(o, "Vague")[300..].iter().all(|v| v.is_finite()), "the wave on the last 300 bars");
        assert!(o.has_output_calls);
        // (force_overlay: on the price pane; everything else stays in the oscillator's ±150)
        for s in &o.shapes {
            if !s.force_overlay && s.loc == Loc::Absolute {
                assert!(s.price.abs() < 1_000.0, "shape {} at {}", s.title, s.price);
            }
        }
        for d in o.drawings.iter().filter(|d| !d.force_overlay) {
            for y in [d.y1, d.y2].into_iter().filter(|y| y.is_finite()) {
                assert!(y.abs() < 1_000.0, "{} in the oscillator pane at a price level {y}", d.kind);
            }
        }
        for d in o.drawings.iter().filter(|d| d.force_overlay) {
            assert!(d.y1.abs() > 1_000.0, "{} on the price pane", d.kind);
        }
    }
    // the CVD really uses the intrabars: the ribbons differ somewhere
    let (a, b) = (plot(&sans, "Bosse du haut"), plot(&avec_ib, "Bosse du haut"));
    let (c, d) = (plot(&sans, "Bosse du bas"), plot(&avec_ib, "Bosse du bas"));
    assert!(a
        .iter()
        .zip(b)
        .chain(c.iter().zip(d))
        .any(|(x, y)| x.is_finite() && y.is_finite() && (x - y).abs() > 1e-6));
}

#[test]
fn ce_script_trace_t_il_quelque_chose() {
    let bars = grouper(&minutes(0, 600, 100.0), 300);
    let debut: String = VAGUE_TV.lines().take(79).map(|l| format!("{l}\n")).collect();
    assert!(
        !run(&debut, &bars, 300, &Default::default()).unwrap().has_output_calls,
        "the first 79 lines draw nothing"
    );
    let src = "//@version=6\nindicator(\"l\", overlay = true)\nf(x) => x + 1\nif barstate.islast\n    label.new(bar_index, high, \"x\")\n";
    assert!(run(src, &bars, 300, &Default::default()).unwrap().has_output_calls);
    let rien = "//@version=6\nindicator(\"r\")\nx = ta.sma(close, 3)\n";
    assert!(!run(rien, &bars, 300, &Default::default()).unwrap().has_output_calls);
    let strat = "//@version=6\nstrategy(\"s\")\nx = close\n";
    assert!(run(strat, &bars, 300, &Default::default()).unwrap().has_output_calls);
}

#[test]
fn max_boxes_count_et_parametres_de_box_new() {
    let bars = grouper(&minutes(0, 60, 100.0), 300);
    let src = "//@version=6\nindicator(\"b\", max_boxes_count = 2)\nif bar_index < 3\n    box.new(bar_index, 2, bar_index + 1, 1)\n";
    let out = run(src, &bars, 300, &Default::default()).unwrap();
    assert_eq!(out.drawings.iter().map(|d| d.deleted).collect::<Vec<_>>(), [true, false, false]);
    let sans = src.replace(", max_boxes_count = 2", "");
    assert!(
        run(&sans, &bars, 300, &Default::default()).unwrap().drawings.iter().all(|d| !d.deleted),
        "500 when not given"
    );
    // (positional: xloc is box.new's 9th parameter, text_color its 13th)
    let pos = "//@version=6\nindicator(\"p\")\nif barstate.islast\n    box.new(time, 2, time, 1, color.red, 1, line.style_solid, extend.none, xloc.bar_time, color.blue, \"t\", size.small, color.green)\n";
    let d = &run(pos, &bars, 300, &Default::default()).unwrap().drawings[0];
    assert!(d.xtime);
    assert_eq!(d.text_color & 0xFF_FFFF, 0x4CAF50);
}

#[test]
fn le_symbole_de_la_chart_dans_syminfo() {
    let bars = grouper(&minutes(0, 60, 100.0), 300);
    let src = "//@version=6\nindicator(\"s\")\nalert(syminfo.ticker + \" \" + syminfo.tickerid)\nx = request.security(syminfo.tickerid, \"15\", close)\ny = request.security(\"BTC-USDC\", \"15\", close)\nplot(x + y)\n";
    let o = RunOptions {
        ticker: "BTC-USDC".into(),
        exchange: "hyperliquid".into(),
        ..Default::default()
    };
    let out = run_with(src, &bars, 300, &Default::default(), &o).unwrap();
    assert_eq!(out.logs[0].1, "BTC-USDC HYPERLIQUID:BTC-USDC");
    let out = run(src.replace("BTC-USDC", "ARGOS").as_str(), &bars, 300, &Default::default()).unwrap();
    assert_eq!(out.logs[0].1, "ARGOS ARGOS", "nothing said: as before");
}

/// ⏱ The time of both paths for 30 000 one-minute intrabars (500 bars of 1 h), Jee's file:
/// `cargo test -p argos-script --release -- --ignored temps_des_deux_chemins --nocapture`.
#[test]
#[ignore]
fn temps_des_deux_chemins() {
    let m = minutes(1_699_999_200, 30_000, 60_000.0);
    let bars = grouper(&m, 3600);
    let o = avec(&[(60, m.clone())]);
    let mesure = |src: &str, opts: &RunOptions| {
        let t = std::time::Instant::now();
        run_with(src, &bars, 3600, &Default::default(), opts).unwrap_or_else(|e| panic!("{e}"));
        t.elapsed().as_secs_f64()
    };
    let base = mesure(VAGUE_TV, &RunOptions::default());
    let rapide = mesure(VAGUE_TV, &o);
    let complet_src = VAGUE_TV.replace("[open, close, volume])", "[open, close, volume * (cvdLen > 0 ? 1 : 1)])");
    assert_ne!(complet_src, VAGUE_TV);
    let complet = mesure(&complet_src, &o);
    println!(
        "sans intrabars {base:.3} s · chemin rapide {rapide:.3} s (+{:.3}) · chemin complet {complet:.3} s (+{:.3})",
        rapide - base,
        complet - base
    );
}

/// A tuple given by a script function (`f()` ending with `[a, b]`, as in AI Trend Pro): a tuple of arrays, also
/// without intrabars, and a tuple of `na` for an ignored higher timeframe.
#[test]
fn tuple_d_une_fonction_du_script() {
    let m = minutes(0, 100, 100.0);
    let bars = grouper(&m, 300);
    let src = "//@version=6\nindicator(\"f\")\nf() =>\n    [close > open, close < open]\n[a, b] = request.security_lower_tf(syminfo.tickerid, \"1\", f())\n[c, d] = request.security_lower_tf(syminfo.tickerid, \"60\", f(), ignore_invalid_timeframe = true)\nplot(array.size(a) + array.size(b), \"n\")\nplot(na(c) and na(d) ? 1 : 0, \"na\")\n";
    let sans = run(src, &bars, 300, &Default::default()).unwrap_or_else(|e| panic!("{e}"));
    assert!(plot(&sans, "n").iter().all(|x| *x == 0.0));
    assert!(plot(&sans, "na").iter().all(|x| *x == 1.0));
    let avec_ib = run_with(src, &bars, 300, &Default::default(), &avec(&[(60, m)])).unwrap();
    assert!(plot(&avec_ib, "n").iter().all(|x| *x == 10.0));
}

/// The full path reads the newest 10 000 intrabars at most: older chart bars get empty arrays.
#[test]
fn chemin_complet_limite_aux_10000_dernieres() {
    let m = minutes(0, 10_500, 100.0);
    let bars = grouper(&m, 300);
    let src = "//@version=6\nindicator(\"c\")\nk = 1.0\nc = request.security_lower_tf(syminfo.tickerid, \"1\", close * k)\nplot(array.size(c), \"n\")\n";
    let out = run_with(src, &bars, 300, &Default::default(), &avec(&[(60, m)])).unwrap();
    let n = plot(&out, "n");
    assert!(
        n[..100].iter().all(|x| *x == 0.0) && n[100..].iter().all(|x| *x == 5.0),
        "500 older intrabars left out"
    );
}

const VAGUE_TV5: &str = include_str!("../../../../../pine/argos/argos-vague-tradingview-v5.pine");

/// R3 (editeur-pine-1): Jee's TradingView edition v5, with the gauge of the Argos edition, compiles, has as many plots
/// as v4, and a word of the gauge within 16 of the pill's level is not drawn (the others are).
#[test]
fn argos_vague_tradingview_v5_jauge() {
    let m = minutes(1_700_000_400 - 1_700_000_400 % 3600, 600 * 60, 60_000.0);
    let bars = grouper(&m, 3600);
    let v4 = run(VAGUE_TV, &bars, 3600, &Default::default()).unwrap_or_else(|e| panic!("{e}"));
    let v5 = run(VAGUE_TV5, &bars, 3600, &Default::default()).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(v5.plots.len(), v4.plots.len());
    let vivants: Vec<&Drawing> = v5.drawings.iter().filter(|d| d.kind == "label" && !d.deleted).collect();
    let pastille = vivants.iter().find(|d| d.text.contains(" · CVD")).expect("the pill");
    let lvl = pastille.y1;
    let mots = [
        (70.0, "SURACHAT"),
        (30.0, "TENDANCE HAUSSIÈRE"),
        (-30.0, "TENDANCE BAISSIÈRE"),
        (-70.0, "SURVENTE"),
    ];
    for (y, mot) in mots {
        let dessine = vivants.iter().any(|d| d.text == mot);
        assert_eq!(dessine, (y - lvl).abs() >= 16.0, "{mot} at {y}, pill at {lvl}");
    }
    assert!(vivants
        .iter()
        .filter(|d| mots.iter().any(|(_, m)| d.text == *m))
        .all(|d| d.style == "label_left"));
}
