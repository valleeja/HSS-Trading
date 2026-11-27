use super::*;
use argos_script::indicators::classic;

/// Midnight UTC (14 Nov 2023), 1 h bars.
const T0: i64 = 1_699_920_000;
const H1: i64 = 3_600;

/// Deterministic 1 h bars: `f(i)` gives the close, the open is the previous close.
fn bars(n: usize, f: impl Fn(usize) -> f64) -> Vec<Bar> {
    (0..n)
        .map(|i| {
            let c = f(i);
            let o = if i == 0 { c } else { f(i - 1) };
            Bar {
                time: T0 + i as i64 * H1,
                open: o,
                high: o.max(c) + 0.4,
                low: o.min(c) - 0.4,
                close: c,
                volume: 100.0 + (i % 7) as f64 * 15.0,
            }
        })
        .collect()
}

/// Straight up, no wave.
fn straight(n: usize) -> Vec<Bar> {
    bars(n, |i| 100.0 + i as f64 * 0.5)
}

/// Sideways.
fn flat(n: usize) -> Vec<Bar> {
    bars(n, |i| 100.0 + (i as f64 / 2.0).sin() * 2.0)
}

fn rising(n: usize) -> Vec<Bar> {
    bars(n, |i| 100.0 + i as f64 * 0.5 + (i as f64 / 3.0).sin())
}

fn falling(n: usize) -> Vec<Bar> {
    bars(n, |i| 400.0 - i as f64 * 0.5 + (i as f64 / 3.0).sin())
}

/// Rises, then turns down over the last bars.
fn turn(n: usize) -> Vec<Bar> {
    bars(n, |i| {
        if i + 20 < n {
            100.0 + i as f64 * 0.5
        } else {
            100.0 + (2 * (n - 20) - i) as f64 * 0.5
        }
    })
}

fn text(r: &Reading) -> String {
    r.blocks
        .iter()
        .flat_map(|b| &b.sentences)
        .map(|s| s.text.as_str())
        .collect::<Vec<_>>()
        .join(" ")
}

fn ids(r: &Reading) -> Vec<&'static str> {
    r.blocks.iter().flat_map(|b| &b.sentences).map(|s| s.id).collect()
}

fn titles(r: &Reading) -> Vec<&'static str> {
    r.blocks.iter().map(|b| b.title).collect()
}

const THREE: [&str; 3] = ["Ce qu'il montre maintenant", "Comment le lire", "Ce qui changerait la lecture"];

#[test]
fn rsi_states() {
    let up = read("Rsi", &rising(200), &[]);
    assert_eq!((up.coverage, titles(&up)), (Coverage::Rules, THREE.to_vec()));
    let t = text(&up);
    assert!(t.contains("au-dessus de 50") && t.contains("surachat (au-dessus de 70)"), "{t}");
    let down = text(&read("Rsi", &falling(200), &[]));
    assert!(down.contains("sous 50") && down.contains("survente (sous 30)"), "{down}");
    // the thresholds come from the settings
    let wide = text(&read("Rsi", &flat(200), &[("haut", 99.0), ("bas", 1.0)]));
    assert!(wide.contains("entre 1 et 99 : ni surachat ni survente"), "{wide}");
    // the same number as the chart
    let r = classic::rsi_n(&rising(200), 14);
    assert!(t.contains(&format::num(r[199], 1)), "{t}");
    // just out of the overbought zone after the turn
    let b = turn(200);
    let r = classic::rsi_n(&b, 14);
    let k = (150..200)
        .find(|&k| r[k - 1] >= 70.0 && r[k] < 70.0 && r[k] > 30.0)
        .expect("a bar leaving the zone");
    assert!(text(&read("Rsi", &b[..=k], &[])).contains("vient de sortir du surachat"));
}

#[test]
fn macd_states() {
    let b = rising(200);
    let r = read("Macd", &b, &[]);
    let t = text(&r);
    let m = classic::macd_n(&b, 12, 26, 9);
    assert!(
        t.contains(&format::signed(m.line[199], b[199].close)) && t.contains(&format::signed(m.hist[199], b[199].close)),
        "{t}"
    );
    assert!(t.contains("au-dessus de zéro") && ids(&r).contains(&"cross"), "{t}");
    let after = text(&read("Macd", &turn(200), &[]));
    assert!(after.contains("sous son signal") && after.contains("depuis"), "{after}");
    // the lengths come from the settings
    let m5 = classic::macd_n(&b, 5, 35, 5);
    assert!(text(&read("Macd", &b, &[("fast", 5.0), ("slow", 35.0), ("signal", 5.0)])).contains(&format::signed(m5.line[199], b[199].close)));
}

#[test]
fn ema_states() {
    let up = text(&read("EmaCross", &straight(200), &[]));
    assert!(
        up.contains("au-dessus de l'EMA 9") && up.contains("EMA 9 au-dessus de l'EMA 21") && up.contains("EMA 21 en hausse"),
        "{up}"
    );
    let down = text(&read("EmaCross", &turn(200), &[]));
    assert!(down.contains("dernier croisement à la baisse il y a"), "{down}");
    let e = classic::ema_cross_n(&straight(200), 9, 21);
    assert!(up.contains(&format::price(e.slow[199])));
    // multiple EMAs: aligned up, aligned down, not computed yet for the 200
    let m = text(&read("EmaMulti", &rising(300), &[]));
    assert!(m.contains("alignement haussier") && m.contains("au-dessus de l'EMA 200"), "{m}");
    assert!(text(&read("EmaMulti", &falling(300), &[])).contains("alignement baissier"));
    let short = text(&read("EmaMulti", &rising(120), &[]));
    assert!(short.contains("1 EMA pas encore calculée :"), "{short}");
}

#[test]
fn vwap_states() {
    let b = rising(30);
    let t = text(&read("Vwap", &b, &[]));
    let v = classic::vwap_src(&b, true);
    assert!(
        t.contains("au-dessus du VWAP du jour") && t.contains(&format::price(v[29])) && t.contains("+"),
        "{t}"
    );
    assert!(!t.contains("de le"), "{t}");
    let t = text(&read("Vwap", &falling(30), &[("hl3", 0.0)]));
    assert!(t.contains("sous le VWAP") && t.contains("−"), "{t}");
}

#[test]
fn too_few_bars_are_said() {
    for id in READ_BY_RULES {
        let r = read(id, &rising(1), &[]);
        assert_eq!(r.coverage, Coverage::Rules);
        // (a live state without the screen's data says that first; the bubbles need 2 bars first)
        let want = if matches!(id, "Tape" | "BookHeat" | "LiqMap" | "Footprint") {
            "missing"
        } else {
            "too_few"
        };
        assert!(ids(&r).contains(&want) || id == "Vwap", "{id}: {}", text(&r));
        assert!(
            read(id, &[], &[]).blocks.iter().flat_map(|b| &b.sentences).any(|s| s.id == want),
            "{id} with no bar"
        );
    }
}

#[test]
fn unknown_id_reads_its_settings_only() {
    let r = read("Lens", &rising(200), &[("len", 20.0), ("mult", 1.5)]);
    assert_eq!(r.coverage, Coverage::SettingsOnly);
    assert_eq!(titles(&r), ["Lecture des réglages seulement"]);
    assert_eq!(
        r.settings,
        vec![
            Setting {
                key: "len".into(),
                value: 20.0,
                role: None
            },
            Setting {
                key: "mult".into(),
                value: 1.5,
                role: None
            }
        ]
    );
    assert_eq!(r.reminder, REMINDER);
    // a known indicator gives the role of its settings, and ignores an unknown key
    let rsi = read("Rsi", &rising(200), &[("len", 21.0), ("nope", 3.0)]);
    assert!(rsi.settings[0].role.is_some() && rsi.settings[1].role.is_none());
    assert!(text(&rsi).contains("RSI 21"));
}

/// Never a word of an order, in any reading.
#[test]
fn never_an_order() {
    const WORDS: [&str; 9] = ["achète", "acheter", "achat", "vends", "vendre", "vente", "entrer", "prends position", "ordre"];
    for id in READ_BY_RULES.iter().chain(&["Lens"]) {
        for b in [rising(300), falling(300), turn(300), rising(3)] {
            let t = text(&read(id, &b, &[])).to_lowercase();
            let words: Vec<&str> = t.split(|c: char| !c.is_alphabetic()).collect();
            for w in WORDS {
                let hit = if w.contains(' ') { t.contains(w) } else { words.contains(&w) };
                assert!(!hit, "{id}: « {w} » in {t}");
            }
        }
    }
}

/// 🧪 Real examples on candles from the cache (not run unless `ARGOS_EXEMPLES_AI` names a candle file).
#[test]
fn real_examples_from_the_cache() {
    let Some(path) = std::env::var_os("ARGOS_EXEMPLES_AI") else { return };
    let b = argos_data::cache::read_raw(std::path::Path::new(&path)).expect("cache file");
    for cut in [0usize, 7, 19] {
        let closed = &b[..b.len() - 1 - cut];
        let t = closed.last().map_or(0, |x| x.time);
        for id in READ_BY_RULES {
            // (the summary tables shown: Bands Osc's too, the only one unticked by default)
            let r = read(id, closed, &[("showTable", 1.0)]);
            println!("===== {id} · bougie {t} =====");
            for bl in &r.blocks {
                println!("## {}", bl.title);
                for x in &bl.sentences {
                    println!("- {}", x.text);
                }
            }
        }
    }
}

// ---- lot 2 ----

use argos_script::indicators::textbook;

/// The reading of `id` with its default settings.
fn t(id: &str, b: &[Bar]) -> String {
    text(&read(id, b, &[]))
}

#[test]
fn stoch_rsi_and_stochastic_states() {
    for id in ["StochRsi", "Stochastic"] {
        let up = t(id, &rising(200));
        assert!(up.contains("%K à") && up.contains("depuis"), "{id}: {up}");
        let after = t(id, &turn(200));
        assert!(after.contains("sous %D") || after.contains("survente"), "{id}: {after}");
        assert_eq!(read(id, &rising(200), &[]).coverage, Coverage::Rules);
    }
    let b = rising(200);
    let (k, _) = textbook::stoch_rsi_n(&b, 14, 14, 3, 3);
    assert!(t("StochRsi", &b).contains(&format::num(k[199], 1)));
    // the chart's Stochastic as it comes: 14, 1, 3, as TradingView's (Jee's choice, 26 Nov 2026)
    let (k, _) = classic::stochastic_n(&b, 14, 1, 3);
    assert!(t("Stochastic", &b).contains(&format::num(k[199], 1)));
    // settings change the numbers
    let (k5, _) = textbook::stoch_rsi_n(&b, 5, 5, 1, 1);
    assert!(text(&read("StochRsi", &b, &[("rsi", 5.0), ("len", 5.0), ("k", 1.0), ("d", 1.0)])).contains(&format::num(k5[199], 1)));
}

#[test]
fn mfi_states() {
    let up = t("Mfi", &rising(200));
    let m = textbook::mfi_n(&rising(200), 14);
    assert!(
        up.contains(&format::num(m[199], 1)) && up.contains("MFI au-dessus de 50 : l'argent qui entre domine"),
        "{up}"
    );
    let down = t("Mfi", &falling(200));
    assert!(down.contains("MFI sous 50 : l'argent qui sort domine"), "{down}");
}

#[test]
fn channel_states() {
    let b = straight(200);
    let up = t("Bollinger", &b);
    let bb = classic::bollinger_n(&b, 20, 2.0);
    assert!(
        up.contains(&format::price(bb.basis[199])) && up.contains("du canal de Bollinger") && up.contains("Largeur"),
        "{up}"
    );
    assert!(t("Bollinger", &flat(200)).contains("entre"));
    let k = t("Keltner", &b);
    let (_, km, _) = textbook::keltner_n(&b, 20, 10, 2.0);
    assert!(k.contains(&format::price(km[199])) && k.contains("canal de Keltner"), "{k}");
    let d = t("Donchian", &b);
    assert!(d.contains("touche le plus haut des 20 dernières bougies"), "{d}");
    assert!(t("Donchian", &falling(200)).contains("touche le plus bas"));
    assert!(!up.contains("de le") && !d.contains("de du"), "{up} {d}");
}

/// 📜 Bollinger is TradingView's script run by Argos Script: its keys are the five input titles, and the reading
/// with Jee-like settings quotes the script's bands (equal to the native calculation, `classic::bollinger_n`).
#[test]
fn bollinger_is_the_scripts() {
    assert_eq!(keys_read("Bollinger"), ["Length", "Basis MA Type", "Source", "StdDev", "Offset"]);
    let b = walk(400);
    let r = text(&read("Bollinger", &b, &[("Length", 30.0), ("StdDev", 2.5)]));
    let bb = classic::bollinger_n(&b, 30, 2.5);
    let n = b.len() - 1;
    // (the reading quotes the two bands around the close and the width, which takes the three of them)
    let largeur = |u: f64, l: f64| format!("Largeur {} % du prix", format!("{:.2}", (u - l) / bb.basis[n] * 100.0).replace('.', ","));
    assert!(
        r.contains(&format::price(bb.basis[n]))
            && (r.contains(&format::price(bb.upper[n])) || r.contains(&format::price(bb.lower[n])))
            && r.contains(&largeur(bb.upper[n], bb.lower[n])),
        "the three bands: {r}"
    );
    assert!(r.contains("Moyenne simple 30 au milieu, bandes à 2,5 écarts-types"), "{r}");
    assert!(!r.contains("décalées"), "{r}");
}

/// 📜 Bollinger « StdDev » alone: the bands widen, the middle line stays the same.
#[test]
fn bollinger_stddev_alone_widens() {
    let b = walk(400);
    let n = b.len() - 1;
    let (b2, b3) = (classic::bollinger_n(&b, 30, 2.0), classic::bollinger_n(&b, 30, 3.0));
    let r2 = text(&read("Bollinger", &b, &[("Length", 30.0), ("StdDev", 2.0)]));
    let r3 = text(&read("Bollinger", &b, &[("Length", 30.0), ("StdDev", 3.0)]));
    let basis = format::price(b2.basis[n]);
    assert_eq!(basis, format::price(b3.basis[n]), "same middle line");
    assert!(r2.contains(&basis) && r3.contains(&basis), "the middle line in both: {r2} / {r3}");
    let largeur = |bb: &classic::Bands| format!("{:.2}", (bb.upper[n] - bb.lower[n]) / bb.basis[n] * 100.0).replace('.', ",");
    assert!(
        r3.contains(&format!("Largeur {} %", largeur(&b3))) && r2.contains(&format!("Largeur {} %", largeur(&b2))) && largeur(&b2) != largeur(&b3),
        "wider bands: {r2} / {r3}"
    );
}

/// 📜 Bollinger « Basis MA Type »: the middle line is the chosen average (EMA here), named in the reading.
#[test]
fn bollinger_basis_type_is_read() {
    let b = walk(400);
    let sma = text(&read("Bollinger", &b, &[]));
    let ema = text(&read("Bollinger", &b, &[("Basis MA Type", 1.0)]));
    assert!(ema.contains("Moyenne exponentielle 20 au milieu"), "{ema}");
    assert_ne!(sma.replace("simple", "exponentielle"), ema, "the bands changed too");
    let e = argos_script::ta::ema(&b.iter().map(|x| x.close).collect::<Vec<_>>(), 20);
    let mid = |t: &str| t.contains(&format::price(e[b.len() - 1]));
    assert!(mid(&ema) && !mid(&sma), "{ema}");
}

/// 📜 Bollinger « Source »: another price changes the bands and is named.
#[test]
fn bollinger_source_is_read() {
    let b = walk(400);
    let close = text(&read("Bollinger", &b, &[]));
    let high = text(&read("Bollinger", &b, &[("Source", 2.0)]));
    assert!(high.contains("Moyenne simple 20 (Haut) au milieu"), "{high}");
    assert_ne!(close.replace(" (Haut)", ""), high.replace(" (Haut)", ""), "other bands");
}

/// 📜 Bollinger « Offset »: shifted right by k bars, the bands quoted are the ones drawn on the last closed bar (their
/// values k bars earlier); shifted left, the last computed ones; both say so.
#[test]
fn bollinger_offset_is_read() {
    let b = walk(400);
    let n = b.len() - 1;
    let bb = classic::bollinger_n(&b, 20, 2.0);
    let right = text(&read("Bollinger", &b, &[("Offset", 3.0)]));
    assert!(right.contains("décalées de 3 bougies vers la droite"), "{right}");
    assert!(right.contains(&format!("milieu ({})", format::price(bb.basis[n - 3]))), "{right}");
    let left = text(&read("Bollinger", &b, &[("Offset", -3.0)]));
    assert!(left.contains("vers la gauche"), "{left}");
    assert!(
        left.contains("dessinées 3 bougies plus tôt (rien n'est dessiné sur la dernière bougie)"),
        "{left}"
    );
    assert!(left.contains(&format!("milieu ({})", format::price(bb.basis[n]))), "{left}");
}

#[test]
fn volatility_and_volume_states() {
    let b = rising(200);
    let a = t("Atr", &b);
    let atr = classic::atr_n(&b, 14);
    assert!(a.contains(&format::signed(atr[199], b[199].close)) && a.contains("% du prix"), "{a}");
    let mut v = rising(60);
    v[59].volume = 10_000.0;
    let r = t("RelVolume", &v);
    assert!(r.contains("un pic (au moins 3,0 fois la moyenne)"), "{r}");
    let r = classic::relative_volume_n(&b, 20);
    assert!(t("RelVolume", &b).contains(&format::num(r[199], 2)));
    assert!(t("Obv", &rising(60)).contains("OBV en hausse, prix en hausse (le volume confirme le mouvement)"));
    assert!(text(&read("Obv", &rising(60), &[("sig", 5.0)])).contains("sa moyenne de 5 bougies"));
}

#[test]
fn trend_states() {
    let up = t("Dmi", &straight(200));
    let (pl, _, adx) = textbook::dmi_n(&straight(200), 14, 14);
    assert!(
        up.contains(&format::num(pl[199], 1)) && up.contains("les hausses dominent") && up.contains("tendance forte"),
        "{up}"
    );
    let calm = t("Dmi", &flat(200));
    let (_, _, a) = textbook::dmi_n(&flat(200), 14, 14);
    assert!(a[199] < 25.0 && calm.contains("pas de tendance forte (sous 25)"), "{calm}");
    assert!(adx[199] >= 25.0);
    let st = t("Supertrend", &straight(200));
    assert!(st.contains("Supertrend haussier") && st.contains("sous la clôture"), "{st}");
    assert!(t("Supertrend", &turn(200)).contains("Supertrend baissier"));
    let ps = t("Psar", &straight(200));
    assert!(ps.contains("SAR parabolique haussier"), "{ps}");
    assert!(t("Psar", &turn(200)).contains("SAR parabolique baissier"));
}

#[test]
fn other_oscillator_states() {
    let c = t("Cci", &straight(200));
    assert!(c.contains("CCI 20") && c.contains("Au-dessus de +100"), "{c}");
    assert!(t("Cci", &falling(200)).contains("Sous −100"));
    let w = t("WilliamsR", &straight(200));
    assert!(w.contains("surachat (au-dessus de −20)"), "{w}");
    assert!(t("WilliamsR", &falling(200)).contains("survente (sous −80)"));
    let a = t("Aroon", &straight(200));
    assert!(a.contains("Aroon haut à 100") && a.contains("tendance haussière nette"), "{a}");
}

/// Every id read by rules gives the three blocks on enough bars.
#[test]
fn every_rule_gives_three_blocks() {
    assert_eq!(READ_BY_RULES.len(), 51);
    for id in READ_BY_RULES {
        // (the TBO needs a confirmed high and low: on a wave)
        let b = if id == "Tbo" { waves(300) } else { rising(300) };
        // (the live states without the screen's data: what it shows, how to read it, and nothing to change)
        let want = if matches!(id, "Tape" | "BookHeat" | "LiqMap" | "Footprint" | "Bubbles") {
            THREE[..2].to_vec()
        } else {
            THREE.to_vec()
        };
        assert_eq!(titles(&read(id, &b, &[])), want, "{id}");
    }
}

// ---- idea 1: the sense of each reading, and the indicators together ----

/// The sense of each id agrees with its own text, on rising, falling, turning and flat bars.
#[test]
fn sense_matches_text() {
    for b in [rising(300), falling(300), turn(300), flat(300)] {
        for id in READ_BY_RULES {
            let r = read(id, &b, &[]);
            if id == "Tbo" && r.sense.is_none() {
                // a straight move has no high nor low: no TBO value, and the text says why
                assert!(text(&r).contains("Pas encore de valeur TBO"), "{}", text(&r));
                continue;
            }
            if matches!(id, "Tape" | "BookHeat" | "LiqMap" | "Footprint" | "Bubbles") {
                // (a live state without the screen's data: said, no sense)
                let said = if id == "Bubbles" { "vrais trades absents" } else { "Donnée absente" };
                assert!(r.sense.is_none() && text(&r).contains(said), "{id}");
                continue;
            }
            let x = r.sense.unwrap_or_else(|| panic!("{id}: no sense"));
            // what it shows now and what would change it (« Comment le lire » names both sides)
            let tx = r
                .blocks
                .iter()
                .filter(|bl| bl.title != "Comment le lire")
                .flat_map(|bl| &bl.sentences)
                .map(|x| x.text.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            let up = x.lean == Lean::Up;
            let has = |w: &str| tx.contains(w);
            let ok = match id {
                "Rsi" => up == has("au-dessus de 50,"),
                "Macd" | "StochRsi" | "Stochastic" => up == (has(": au-dessus de son signal") || has(": au-dessus de %D")) || x.lean == Lean::Neutral,
                "EmaCross" => up == has("EMA 9 au-dessus de l'EMA 21 :"),
                "EmaMulti" => (x.lean == Lean::Up) == has("alignement haussier") && (x.lean == Lean::Down) == has("alignement baissier"),
                "Vwap" => up == has("au-dessus du VWAP du jour"),
                "Mfi" => up == has("MFI au-dessus de 50"),
                "Cci" => up == (has("Au-dessus de +100") || has("Entre 0 et +100")),
                "WilliamsR" => up == has("Un passage sous −50"),
                "Aroon" => (x.lean == Lean::Up) == has("haussière nette") && (x.lean == Lean::Down) == has("baissière nette"),
                "Atr" | "RelVolume" => x.lean == Lean::NoDirection && x.note.is_some(),
                "Obv" => up == has("OBV en hausse"),
                "Dmi" => (x.lean == Lean::Neutral) == has("pas de tendance forte") && (!up || has("les hausses dominent")),
                "Supertrend" | "Psar" => up == has("haussier :"),
                "Patterns" => match x.lean {
                    Lean::Up => has("Figure la plus récente « ▲"),
                    Lean::Down => has("Figure la plus récente « ▼"),
                    Lean::Neutral => has("Figure la plus récente « ◆"),
                    _ => has("Aucune figure affichée."),
                },
                "SwingVwap" | "SmartSwingVwap" => match x.lean {
                    Lean::Up => has(") est au-dessus ("),
                    Lean::Down => has(") est en dessous ("),
                    Lean::NoDirection => has("Aucun VWAP actif") || has("Aucun pivot"),
                    _ => has(") est au niveau ("),
                },
                "AnchoredVwap" => up == has(") est au-dessus ("),
                "Heatmap" | "Profile" => x.lean == Lean::NoDirection,
                "Sma" | "Wma" | "Hma" | "Vwma" | "Smma" => up == has(") est au-dessus ("),
                "MaRibbon" => match x.lean {
                    Lean::Up => has("Rangement haussier") && has("au-dessus de toutes les moyennes affichées"),
                    Lean::Down => has("Rangement baissier") && has("en dessous de toutes les moyennes affichées"),
                    Lean::Neutral => !(has("Rangement haussier") && has("au-dessus de toutes")) && !(has("Rangement baissier") && has("en dessous de toutes")),
                    _ => has("Aucune moyenne cochée"),
                },
                "Bubbles" => {
                    let v = x.value.clone().unwrap_or_default();
                    let k: Vec<usize> = v.split(' ').filter_map(|w| w.parse().ok()).collect();
                    k.len() == 2 && (x.lean == Lean::Up) == (k[0] > k[1]) && (x.lean == Lean::Down) == (k[1] > k[0])
                }
                "Cvd" => up == has("CVD en hausse"),
                "MtfTable" => match x.lean {
                    Lean::Up => !has("tendance baissière") && !has("tendance neutre"),
                    Lean::Down => !has("tendance haussière") && !has("tendance neutre"),
                    Lean::Neutral => true,
                    _ => !has("tendance "),
                },
                "Ichimoku" => (x.lean == Lean::Up) == has("au-dessus du nuage (haut") && (x.lean == Lean::Down) == has("sous le nuage (bas"),
                "BandsOsc" => up == has("la force haussière domine"),
                "Bollinger" | "Bands" | "Keltner" | "Donchian" => up == has("Une clôture sous le milieu"),
                "Cipher" => up == has("WT1 est au-dessus de WT2"),
                "AiTrend" => up == has("L'AI est au-dessus de sa ligne signal"),
                "Tbo" => up == has("la force haussière domine"),
                "VectorZone" | "SpringTension" | "PriceAction" => x.lean == Lean::NoDirection,
                "Smc" | "SmcPro" => match x.lean {
                    Lean::Up => has("Dernière cassure de structure : BOS haussier") || has("Dernière cassure de structure : CHoCH haussier"),
                    Lean::Down => has("Dernière cassure de structure : BOS baissier") || has("Dernière cassure de structure : CHoCH baissier"),
                    _ => !has("Dernière cassure de structure"),
                },
                "Tvd" => up == (has("Nuage vert") || (!has("Nuage ") && has("EMA rapide à") && has(": au-dessus de l'EMA lente"))),
                _ => false,
            };
            assert!(ok, "{id}: {x:?} / {tx}");
        }
    }
    // a reading without rules, or with too few bars, has no sense
    assert_eq!(read("Lens", &rising(300), &[]).sense, None);
    assert_eq!(read("Rsi", &rising(2), &[]).sense, None);
    // the note of a zone
    assert_eq!(read("Rsi", &straight(200), &[]).sense.and_then(|x| x.note), Some("en surachat"));
}

fn item<'a>(id: &'a str, name: &'a str) -> Item<'a> {
    Item { id, name, settings: &[] }
}

fn overview_text(o: &Overview) -> String {
    o.blocks
        .iter()
        .flat_map(|b| &b.sentences)
        .map(|s| s.text.as_str())
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn together_counts_and_agreement() {
    let items = [
        item("Rsi", "RSI 14"),
        item("Supertrend", "Supertrend"),
        item("Vwap", "VWAP"),
        item("Atr", "ATR 14"),
        item("Lens", "Argos Lens"),
    ];
    let o = read_together(&items, &falling(300));
    let t = overview_text(&o);
    assert_eq!(o.reminder, REMINDER);
    assert!(
        t.contains("Sur 3 indicateurs qui donnent un sens : 3 à la baisse (RSI 14 (en survente), Supertrend et VWAP)"),
        "{t}"
    );
    assert!(t.contains("Ils vont tous dans le même sens : à la baisse."), "{t}");
    assert!(t.contains("Sans direction, ils mesurent l'agitation ou le volume : ATR 14 (agitation"), "{t}");
    assert!(t.contains("Pas encore lus par Argos AI : Argos Lens."), "{t}");
    // a disagreement, found where the RSI and the Supertrend lean opposite ways
    let (b, k) = [turn(300), flat(300), rising(300)]
        .into_iter()
        .find_map(|b| {
            let k = (60..=b.len()).find(|&k| {
                let s = |id| read(id, &b[..k], &[]).sense.map(|x| x.lean);
                matches!(
                    (s("Rsi"), s("Supertrend")),
                    (Some(Lean::Up), Some(Lean::Down)) | (Some(Lean::Down), Some(Lean::Up))
                )
            })?;
            Some((b, k))
        })
        .expect("bars where they disagree");
    let t = overview_text(&read_together(&[item("Rsi", "RSI 14"), item("Supertrend", "Supertrend")], &b[..k]));
    assert!(
        t.contains("Ils ne sont pas d'accord :") && t.contains("RSI 14") && t.contains("Supertrend"),
        "{t}"
    );
}

#[test]
fn together_simple_cases() {
    assert!(overview_text(&read_together(&[], &rising(300))).contains("Aucun indicateur sur la chart"));
    let t = overview_text(&read_together(&[item("Rsi", "RSI 14")], &rising(300)));
    assert!(t.contains("Un seul indicateur donne un sens"), "{t}");
    let t = overview_text(&read_together(&[item("Atr", "ATR 14"), item("RelVolume", "Volume relatif")], &rising(300)));
    assert!(t.contains("Aucun des indicateurs affichés ne donne de sens"), "{t}");
    let t = overview_text(&read_together(&[item("Rsi", "RSI 14")], &rising(3)));
    assert!(t.contains("Pas assez de bougies clôturées pour lire : RSI 14."), "{t}");
}

/// The overview never says an order, never pushes to act (« donc »), never gives a global score.
#[test]
fn together_never_an_order() {
    let all: Vec<Item> = READ_BY_RULES.iter().map(|id| item(id, id)).collect();
    for b in [rising(300), falling(300), turn(300), flat(300)] {
        let t = overview_text(&read_together(&all, &b)).to_lowercase();
        let words: Vec<&str> = t.split(|c: char| !c.is_alphabetic()).collect();
        for w in [
            "achète", "acheter", "achat", "vends", "vendre", "vente", "entrer", "ordre", "donc", "score", "signal",
        ] {
            assert!(!words.contains(&w), "« {w} » in {t}");
        }
    }
}

/// 🧪 The overview on cached candles (not run unless `ARGOS_EXEMPLES_AI` names a candle file).
#[test]
fn real_overview_from_the_cache() {
    let Some(path) = std::env::var_os("ARGOS_EXEMPLES_AI") else { return };
    let b = argos_data::cache::read_raw(std::path::Path::new(&path)).expect("cache file");
    let sets: [&[(&str, &str)]; 3] = [
        &[("Rsi", "RSI 14"), ("Macd", "MACD"), ("EmaCross", "Croisement EMA 9/21"), ("Vwap", "VWAP")],
        &[
            ("Supertrend", "Supertrend"),
            ("Dmi", "DMI"),
            ("Atr", "ATR 14"),
            ("RelVolume", "Volume relatif"),
            ("Lens", "Argos Lens"),
        ],
        &[
            ("StochRsi", "RSI stochastique"),
            ("Mfi", "MFI"),
            ("Bollinger", "Bollinger"),
            ("Obv", "OBV"),
            ("Aroon", "Aroon"),
        ],
    ];
    for (cut, set) in [0usize, 7, 19].into_iter().zip(sets) {
        let closed = &b[..b.len() - 1 - cut];
        let items: Vec<Item> = set.iter().map(|(id, name)| item(id, name)).collect();
        println!("===== Ensemble · bougie {} =====", closed.last().map_or(0, |x| x.time));
        for bl in &read_together(&items, closed).blocks {
            println!("## {}", bl.title);
            for x in &bl.sentences {
                println!("- {}", x.text);
            }
        }
    }
}

/// No « (s) » left anywhere: every count agrees with its noun.
#[test]
fn plurals_are_written() {
    let all: Vec<Item> = READ_BY_RULES.iter().map(|id| item(id, id)).collect();
    for b in [rising(300), falling(300), turn(300), flat(300), rising(120), rising(3)] {
        let mut texts = vec![overview_text(&read_together(&all, &b)), overview_text(&read_timeframes(&all, &b, H1, 4 * H1))];
        texts.extend(READ_BY_RULES.iter().map(|id| text(&read(id, &b, &[]))));
        texts.push(overview_text(&read_together(&all[..1], &b)));
        for t in texts {
            assert!(!t.contains("(s)") && !one_plural(&t, "indicateurs") && !one_plural(&t, "bougies"), "{t}");
        }
    }
}

// ---- idea 2: two time units ----

#[test]
fn higher_timeframe_for_each_chart_unit() {
    use argos_core::types::MONTH_SECONDS as MO;
    let want = [
        (1, 300),
        (60, 300),
        (180, 900),
        (300, 1_800),
        (900, 3_600),
        (1_800, 7_200),
        (3_600, 14_400),
        (7_200, 43_200),
        (14_400, 86_400),
        (43_200, 604_800),
        (86_400, 604_800),
        (604_800, MO),
    ];
    for (c, h) in want {
        assert_eq!(higher_timeframe(c), Some(h), "{c}");
    }
    assert_eq!(higher_timeframe(MO), None);
    assert_eq!(higher_timeframe(0), None);
    // odd units: a whole number of chart bars, at least 4
    for c in [120, 2_700, 10_800, 18_000] {
        let h = higher_timeframe(c).unwrap();
        assert!(h >= 4 * c && h % c == 0, "{c} → {h}");
    }
    assert_eq!(
        (timeframe_label(3_600), timeframe_label(14_400), timeframe_label(86_400)),
        ("1h".into(), "4h".into(), "1D".into())
    );
}

#[test]
fn only_whole_higher_bars() {
    let b = rising(10); // 10 bars of 1 h from midnight
    let h = timeframes::whole_higher_bars(&b, H1, 4 * H1);
    assert_eq!(h.len(), 2, "08:00-10:00 is not finished");
    assert_eq!((h[0].time, h[0].open, h[0].close), (T0, b[0].open, b[3].close));
    assert_eq!(h[0].high, b[..4].iter().map(|x| x.high).fold(f64::MIN, f64::max));
    assert_eq!(h[0].volume, b[..4].iter().map(|x| x.volume).sum::<f64>());
    // a history starting in the middle of a 4 h bar: that one goes too
    assert_eq!(timeframes::whole_higher_bars(&b[1..], H1, 4 * H1).len(), 1);
}

#[test]
fn timeframes_agreement() {
    let items = [
        item("Rsi", "RSI 14"),
        item("EmaCross", "Croisement EMA 9/21"),
        item("Atr", "ATR 14"),
        item("Vwap", "VWAP"),
        item("Lens", "Argos Lens"),
    ];
    let o = read_timeframes(&items, &straight(400), H1, 4 * H1);
    let t = overview_text(&o);
    assert_eq!(o.reminder, REMINDER);
    assert!(t.contains("100 bougies 4h complètes construites"), "{t}");
    assert!(
        t.contains("RSI 14 : 1h à la hausse") && t.contains("4h à la hausse") && t.contains("même sens, à la hausse"),
        "{t}"
    );
    assert!(
        t.contains("Sur 2 indicateurs comparés : 2 dans le même sens (RSI 14 et Croisement EMA 9/21)"),
        "{t}"
    );
    assert!(t.contains("ATR 14 : 1h agitation en") && t.contains("% du prix)"), "{t}");
    assert!(
        t.contains("VWAP : il repart chaque jour") && t.contains("Pas encore lus par Argos AI : Argos Lens."),
        "{t}"
    );
    // too few higher bars for one indicator, without blocking the others
    let t = overview_text(&read_timeframes(&[item("Rsi", "RSI 14"), item("Obv", "OBV")], &rising(60), H1, 4 * H1));
    assert!(t.contains("RSI 14 : pas assez de bougies en 4h."), "{t}");
    assert!(t.contains("OBV : 1h"), "{t}");
    // a disagreement between the units
    let b = turn(400);
    let k = (200..=400)
        .find(|&k| {
            let l1 = read("Rsi", &b[..k], &[]).sense.map(|x| x.lean);
            let l2 = read("Rsi", &timeframes::whole_higher_bars(&b[..k], H1, 4 * H1), &[]).sense.map(|x| x.lean);
            matches!((l1, l2), (Some(Lean::Down), Some(Lean::Up)) | (Some(Lean::Up), Some(Lean::Down)))
        })
        .expect("a bar where 1h and 4h disagree");
    let t = overview_text(&read_timeframes(&[item("Rsi", "RSI 14")], &b[..k], H1, 4 * H1));
    assert!(t.contains("pas d'accord") && t.contains("1 pas d'accord (RSI 14)"), "{t}");
}

#[test]
fn timeframes_never_an_order() {
    let all: Vec<Item> = READ_BY_RULES.iter().map(|id| item(id, id)).collect();
    for b in [rising(400), falling(400), turn(400), flat(400)] {
        let t = overview_text(&read_timeframes(&all, &b, H1, 4 * H1)).to_lowercase();
        let words: Vec<&str> = t.split(|c: char| !c.is_alphabetic()).collect();
        for w in [
            "achète", "acheter", "achat", "vends", "vendre", "vente", "entrer", "ordre", "donc", "score", "signal",
        ] {
            assert!(!words.contains(&w), "« {w} » in {t}");
        }
    }
}

/// 🧪 Two units on cached candles (not run unless `ARGOS_EXEMPLES_AI` names a candle file and `ARGOS_EXEMPLES_SECS`
/// its unit).
#[test]
fn real_timeframes_from_the_cache() {
    let Some(path) = std::env::var_os("ARGOS_EXEMPLES_AI") else { return };
    let secs: i64 = std::env::var("ARGOS_EXEMPLES_SECS").ok().and_then(|s| s.parse().ok()).unwrap_or(H1);
    let b = argos_data::cache::read_raw(std::path::Path::new(&path)).expect("cache file");
    let set = [
        ("Rsi", "RSI 14"),
        ("Macd", "MACD"),
        ("EmaCross", "Croisement EMA 9/21"),
        ("Supertrend", "Supertrend"),
        ("Dmi", "DMI"),
        ("Atr", "ATR 14"),
        ("Vwap", "VWAP"),
    ];
    let items: Vec<Item> = set.iter().map(|(id, name)| item(id, name)).collect();
    let higher = higher_timeframe(secs).expect("a higher unit");
    for cut in [0usize, 7, 19] {
        let closed = &b[..b.len() - 1 - cut];
        println!("===== Unités · bougie {} · {secs} → {higher} =====", closed.last().map_or(0, |x| x.time));
        for bl in &read_timeframes(&items, closed, secs, higher).blocks {
            println!("## {}", bl.title);
            for x in &bl.sentences {
                println!("- {}", x.text);
            }
        }
    }
}

// ---- idea 3: divergences ----

/// A slow rise, a steep rally to a first high (bar 45), a drop, a slower rally to a higher second high (bar 61), then
/// a drop that confirms it: a higher high with less momentum. `extra` more closes are added at the end.
fn bearish_divergence(extra: &[f64]) -> Vec<Bar> {
    let mut c: Vec<f64> = (0..40).map(|i| 100.0 + i as f64 * 0.05).collect();
    for step in [4.0; 6].into_iter().chain([-2.0; 5]).chain([1.0; 11]).chain([-1.0; 5]) {
        c.push(c[c.len() - 1] + step);
    }
    c.extend_from_slice(extra);
    bars(c.len(), |i| c[i])
}

/// The same, upside down: a lower low with less downward momentum.
fn bullish_divergence(extra: &[f64]) -> Vec<Bar> {
    let b = bearish_divergence(&[]);
    let c: Vec<f64> = b.iter().map(|x| 300.0 - x.close).chain(extra.iter().copied()).collect();
    bars(c.len(), |i| c[i])
}

fn has_divergence(id: &str, b: &[Bar]) -> (bool, String) {
    let r = read(id, b, &[]);
    (ids(&r).contains(&"divergence"), text(&r))
}

#[test]
fn divergences_found_and_written() {
    let b = bearish_divergence(&[]);
    let (yes, t) = has_divergence("Rsi", &b);
    assert!(yes && t.contains("Divergence baissière : le prix a fait un plus haut plus haut"), "{t}");
    assert!(
        t.contains("mais le RSI 14 un plus haut plus bas") && t.contains("il y a 21 bougies, puis") && t.contains("confirmée il y a 2 bougies"),
        "{t}"
    );
    assert!(t.contains("pas un retournement certain"), "{t}");
    let r = read("Rsi", &b, &[]);
    assert!(
        r.sense.as_ref().and_then(|x| x.note).is_some_and(|n| n.contains("divergence baissière")),
        "{:?}",
        r.sense
    );
    let (yes, t) = has_divergence("Rsi", &bullish_divergence(&[]));
    assert!(
        yes && t.contains("Divergence haussière : le prix a fait un plus bas plus bas") && t.contains("un plus bas plus haut"),
        "{t}"
    );
}

#[test]
fn no_divergence_when_not_current() {
    // none on a plain rise, and nothing said about it
    let (yes, t) = has_divergence("Rsi", &rising(200));
    assert!(!yes && !t.contains("ivergence"), "{t}");
    // the second high not confirmed yet (only 1 bar after it)
    let b = bearish_divergence(&[]);
    assert!(!has_divergence("Rsi", &b[..63]).0);
    // too old: 20 more bars of slow fall
    let last = b[b.len() - 1].close;
    let later: Vec<f64> = (1..=20).map(|k| last - k as f64 * 0.05).collect();
    assert!(!has_divergence("Rsi", &bearish_divergence(&later)).0);
    // gone past: the price went above the second high since
    assert!(!has_divergence("Rsi", &bearish_divergence(&[last + 3.0, last + 6.0, last + 9.0])).0);
    // too few bars
    assert!(!has_divergence("Rsi", &rising(3)).0);
}

/// The note of RSI, MACD and MFI says « divergence » exactly when the text has one, on every series.
#[test]
fn divergence_note_matches_text() {
    let series = [
        bearish_divergence(&[]),
        bullish_divergence(&[]),
        rising(300),
        falling(300),
        turn(300),
        flat(300),
    ];
    for b in &series {
        for id in ["Rsi", "Macd", "Mfi"] {
            let r = read(id, b, &[]);
            let in_note = r.sense.as_ref().and_then(|x| x.note).is_some_and(|n| n.contains("divergence"));
            assert_eq!(in_note, ids(&r).contains(&"divergence"), "{id}: {}", text(&r));
        }
    }
    // the overview shows it through the note
    let t = overview_text(&read_together(&[item("Rsi", "RSI 14"), item("Vwap", "VWAP")], &bearish_divergence(&[])));
    assert!(t.contains("RSI 14 (") && t.contains("divergence baissière"), "{t}");
}

/// 🧪 Real divergences on cached candles (not run unless `ARGOS_EXEMPLES_AI` names a candle file): every RSI
/// divergence on the bar it is confirmed, with what the price did over the next 12 bars, for Jee's document.
#[test]
fn real_divergences_from_the_cache() {
    let Some(path) = std::env::var_os("ARGOS_EXEMPLES_AI") else { return };
    let b = argos_data::cache::read_raw(std::path::Path::new(&path)).expect("cache file");
    const AFTER: usize = 12;
    for k in 100..b.len().saturating_sub(AFTER) {
        let closed = &b[..k];
        let r = classic::rsi_n(closed, 14);
        let Some(d) = divergence::find(closed, &r).filter(|d| d.confirmed == 0) else {
            continue;
        };
        let (c0, c1) = (closed[k - 1].close, b[k - 1 + AFTER].close);
        let later = &b[k..k + AFTER];
        let failed = if d.bearish {
            later.iter().any(|x| x.high > d.prices.1)
        } else {
            later.iter().any(|x| x.low < d.prices.1)
        };
        println!(
            "===== RSI · bougie {} · {} · ensuite {} {:+.2} % en {AFTER} bougies, {} =====",
            closed[k - 1].time,
            if d.bearish { "baissière" } else { "haussière" },
            if c1 >= c0 { "hausse" } else { "baisse" },
            (c1 - c0) / c0 * 100.0,
            if failed {
                "dernier sommet ou creux dépassé"
            } else {
                "dernier sommet ou creux tenu"
            }
        );
        for x in &read("Rsi", closed, &[]).blocks[0].sentences {
            println!("- {}", x.text);
        }
    }
}

// ---- idea 4: volatility of the moment ----

use volatility::{summarize, Says};

/// Bars of 1 h whose close swings by `amp(i)` around 100, with `vol(i)` of volume.
fn swings(n: usize, amp: impl Fn(usize) -> f64, vol: impl Fn(usize) -> f64) -> Vec<Bar> {
    (0..n)
        .map(|i| {
            let a = amp(i);
            let c = 100.0 + if i % 2 == 0 { a } else { -a };
            Bar {
                time: T0 + i as i64 * H1,
                open: 100.0,
                high: 100.0 + a + 0.01,
                low: 100.0 - a - 0.01,
                close: c,
                volume: vol(i),
            }
        })
        .collect()
}

fn vol_text(b: &[Bar]) -> String {
    overview_text(&read_volatility(b))
}

#[test]
fn volatility_summary_rules() {
    use Says::{Calm, Normal, Strong};
    let t = |x: &[(&str, Says)]| summarize(x);
    assert!(t(&[("ATR", Calm), ("Bollinger", Calm), ("volume", Normal)]).starts_with("Calme inhabituel : ATR et Bollinger montrent"));
    assert!(t(&[("ATR", Strong), ("Bollinger", Normal), ("volume", Strong)]).starts_with("Agitation forte : ATR et volume montrent"));
    assert_eq!(
        t(&[("ATR", Calm), ("Bollinger", Normal), ("volume", Normal)]),
        "Agitation normale : seule la mesure ATR sort de l'habitude et montre un marché calme."
    );
    assert_eq!(
        t(&[("ATR", Normal), ("Bollinger", Normal), ("volume", Strong)]),
        "Agitation normale : seule la mesure volume sort de l'habitude et montre un marché agité."
    );
    assert_eq!(
        t(&[("ATR", Normal), ("Bollinger", Normal), ("volume", Normal)]),
        "Agitation normale : aucune mesure ne sort nettement de l'habitude."
    );
    assert_eq!(
        t(&[("ATR", Calm), ("Bollinger", Calm), ("volume", Strong)]),
        "Les mesures ne sont pas d'accord : ATR et Bollinger montrent un marché calme, volume un marché agité."
    );
    assert!(t(&[]).starts_with("Pas assez de bougies"));
}

#[test]
fn volatility_calm_and_strong() {
    // big swings, then very small ones and little volume: calm
    let calm = vol_text(&swings(300, |i| if i < 280 { 3.0 } else { 0.05 }, |i| if i < 290 { 100.0 } else { 40.0 }));
    assert!(calm.contains("Calme inhabituel : ATR, Bollinger et volume montrent"), "{calm}");
    assert!(calm.contains("au plus étroit des 120 dernières bougies"), "{calm}");
    // small swings, then big ones with a lot of volume: strong
    let strong = vol_text(&swings(300, |i| if i < 290 { 0.2 } else { 4.0 }, |i| if i < 297 { 100.0 } else { 600.0 }));
    assert!(strong.contains("Agitation forte : ATR, Bollinger et volume montrent"), "{strong}");
    assert!(strong.contains("dans les 10 % les plus larges"), "{strong}");
    assert!(calm.contains("pas dans quel sens") || calm.contains("ni dans quel sens"), "{calm}");
}

/// The ATR percentile is the very number of the Lens card, the width the one of the chart's bands.
#[test]
fn volatility_same_numbers() {
    let b = turn(300);
    let t = vol_text(&b);
    let config = argos_lens::Config {
        htf: None,
        ..argos_lens::Config::default()
    };
    let p = argos_lens::photos(&b, H1, &config, None).unwrap().last().unwrap().atr_pct.unwrap();
    assert!(t.contains(&format!("au centile {} des 100", format::num(f64::try_from(p).unwrap(), 0))), "{t}");
    let bb = classic::bollinger_n(&b, 20, 2.0);
    let w = (bb.upper[299] - bb.lower[299]) / bb.basis[299] * 100.0;
    assert!(t.contains(&(format::num(w, 2) + " % du prix")), "{t}");
    let a = classic::atr_n(&b, 14);
    assert!(t.contains(&(format::num(a[299] / b[299].close * 100.0, 2) + " % du prix")), "{t}");
}

#[test]
fn volatility_window_bounds_and_too_few() {
    // the bands need 120 widths, so 139 bars
    assert!(vol_text(&flat(138)).contains("le rang de la largeur des bandes"));
    assert!(!vol_text(&flat(139)).contains("le rang de la largeur des bandes"));
    let t = vol_text(&flat(30));
    assert!(
        t.contains("Pas assez de bougies clôturées pour l'ATR 14 en centile") && t.contains(" et le rang de la largeur des bandes"),
        "{t}"
    );
    assert!(t.contains("Volume de la dernière bougie"), "{t}");
    assert!(vol_text(&[]).contains("Pas assez de bougies pour lire la volatilité du moment."));
}

#[test]
fn volatility_never_an_order_nor_a_direction() {
    for b in [rising(300), falling(300), turn(300), flat(300), flat(30)] {
        let t = vol_text(&b).to_lowercase();
        let words: Vec<&str> = t.split(|c: char| !c.is_alphabetic()).collect();
        for w in [
            "achète", "acheter", "achat", "vends", "vendre", "vente", "entrer", "ordre", "donc", "hausse", "baisse", "haussier", "baissier",
        ] {
            assert!(!words.contains(&w), "« {w} » in {t}");
        }
        assert!(!t.contains("(s)") && !one_plural(&t, "bougies"), "{t}");
    }
}

/// 🧪 The volatility on cached candles (not run unless `ARGOS_EXEMPLES_AI` names a candle file): the summary on every
/// bar, to pick a calm, a normal and a strong moment for Jee's document.
#[test]
fn real_volatility_from_the_cache() {
    let Some(path) = std::env::var_os("ARGOS_EXEMPLES_AI") else { return };
    let b = argos_data::cache::read_raw(std::path::Path::new(&path)).expect("cache file");
    let mut seen = vec![];
    for k in (b.len() - 400..b.len()).rev() {
        let o = read_volatility(&b[..k]);
        let first = o.blocks[0].sentences[0].text.clone();
        let kind = first.split(" :").next().unwrap_or("").to_string();
        if seen.contains(&kind) {
            continue;
        }
        seen.push(kind);
        println!("===== Volatilité · bougie {} =====", b[k - 1].time);
        for bl in &o.blocks {
            println!("## {}", bl.title);
            for x in &bl.sentences {
                println!("- {}", x.text);
            }
        }
    }
}

// ---- Argos Cipher Pro ----

use argos_script::indicators::cipher;

/// Big swings of the price: the WaveTrend goes deep both ways and the waves cross often.
fn waves(n: usize) -> Vec<Bar> {
    bars(n, |i| 100.0 + (i as f64 / 6.0).sin() * 8.0 + (i as f64 / 17.0).sin() * 5.0 + i as f64 * 0.01)
}

fn cipher_defaults() -> Vec<f64> {
    argos_script::inputs::defaults(&cipher::PARAMS)
}

/// The last shown point of the chart, computed here from the script's own lists (most telling kind on a tie).
fn expected_last(b: &[Bar], vals: &[f64]) -> Option<(&'static str, usize)> {
    let c = cipher::compute(b, &cipher::Settings::from_inputs(vals));
    let mut best: Option<(&'static str, usize)> = None;
    for (name, l) in rules::cipher_points(&c, vals) {
        if let Some(&i) = l.last() {
            if best.is_none_or(|(_, j)| i > j) {
                best = Some((name, i));
            }
        }
    }
    best
}

/// The last point named in the text is the one the script draws last, with its age; on every bar of the series.
#[test]
fn cipher_last_point_is_the_scripts() {
    let vals = cipher_defaults();
    let mut kinds_seen = vec![];
    let mixed = bars(500, |i| {
        100.0 + i as f64 * 0.03 + (i as f64 / 9.0).sin() * 3.0 + (i as f64 / 2.3).sin() * 1.5 + (i as f64 / 41.0).sin() * 6.0
    });
    let mirror = bars(500, |i| {
        300.0 - (100.0 + i as f64 * 0.03 + (i as f64 / 9.0).sin() * 3.0 + (i as f64 / 2.3).sin() * 1.5 + (i as f64 / 41.0).sin() * 6.0)
    });
    for b in [waves(400), mixed, mirror, flat(300), turn(300)] {
        for k in 120..=b.len() {
            let t = text(&read("Cipher", &b[..k], &[]));
            match expected_last(&b[..k], &vals) {
                Some((name, i)) if k - 1 - i <= 50 => {
                    let when = if k - 1 == i {
                        "sur la dernière bougie".to_string()
                    } else {
                        format!("il y a {}", format::bars(k - 1 - i))
                    };
                    assert!(t.contains(&format!("Dernier point : {name} {when}")), "{k}: {name} {when} / {t}");
                    if !kinds_seen.contains(&name) {
                        kinds_seen.push(name);
                    }
                }
                _ => assert!(t.contains("Aucun point affiché dans les 50 dernières bougies"), "{t}"),
            }
        }
    }
    // (the green divergence dot sits on the low and a cross follows before it is confirmed: it is seldom the last
    // point; its sentence « Divergence haussière » is checked below)
    for name in [
        "petit point vert",
        "petit point rouge",
        "gros point vert",
        "gros point rouge",
        "point de divergence rouge",
        "point doré (Golden dot)",
        "point violet (Purple dot)",
    ] {
        assert!(kinds_seen.contains(&name), "{name} never seen: {kinds_seen:?}");
    }
    let mirror = bars(500, |i| {
        300.0 - (100.0 + i as f64 * 0.03 + (i as f64 / 9.0).sin() * 3.0 + (i as f64 / 2.3).sin() * 1.5 + (i as f64 / 41.0).sin() * 6.0)
    });
    let seen = |w: &str| (120..=500).any(|k| text(&read("Cipher", &mirror[..k], &[])).contains(w));
    assert!(seen("Divergence haussière du") && seen("Divergence baissière du"));
}

/// A point hidden by its « Show … » setting is never mentioned.
#[test]
fn cipher_hidden_points_are_not_mentioned() {
    let b = waves(400);
    let off: Vec<(&str, f64)> = ["wt_show", "buy_show", "sell_show", "gold_show", "purple_show", "div_dots", "mfi_show"]
        .iter()
        .map(|k| (*k, 0.0))
        .collect();
    let t = text(&read("Cipher", &b, &off));
    assert!(
        t.contains("Aucun point affiché") && !t.contains("WaveTrend : WT2") && !t.contains("Flux d'argent") && !t.contains("Divergence"),
        "{t}"
    );
    // only the big points: no small point is named
    let mut only_big = off.clone();
    only_big.retain(|(k, _)| *k != "buy_show" && *k != "sell_show");
    let t = text(&read("Cipher", &b, &only_big));
    assert!(!t.contains("petit point") && t.contains("Gros point vert :"), "{t}");
}

#[test]
fn cipher_reading_shape() {
    let r = read("Cipher", &waves(400), &[]);
    assert_eq!((r.coverage, titles(&r)), (Coverage::Rules, THREE.to_vec()));
    let t = text(&r);
    assert!(t.contains("WaveTrend : WT2 à") && t.contains("Flux d'argent du Cipher"), "{t}");
    assert!(
        t.contains("Point doré : une divergence haussière") && t.contains("sous le niveau de survente 1 (−53)"),
        "{t}"
    );
    // the thresholds come from the settings
    assert!(text(&read("Cipher", &waves(400), &[("os1", -40.0)])).contains("survente 1 (−40)"));
    // too few bars
    assert!(ids(&read("Cipher", &rising(5), &[])).contains(&"too_few"));
}

/// 🧪 Real Cipher moments on cached candles (not run unless `ARGOS_EXEMPLES_AI` names a candle file): the bars where a
/// gros point vert, a point doré or a divergence just appeared, with what the price did over the next 12 bars.
#[test]
fn real_cipher_from_the_cache() {
    let Some(path) = std::env::var_os("ARGOS_EXEMPLES_AI") else { return };
    let b = argos_data::cache::read_raw(std::path::Path::new(&path)).expect("cache file");
    const AFTER: usize = 12;
    let start = b.len().saturating_sub(600).max(200);
    for k in start..b.len() - AFTER {
        let r = read("Cipher", &b[..k], &[]);
        let now: Vec<&str> = r.blocks[0].sentences.iter().map(|x| x.text.as_str()).collect();
        let fresh = now
            .iter()
            .any(|t| t.contains("sur la dernière bougie") && (t.contains("gros point vert") || t.contains("point doré") || t.starts_with("Divergence")));
        if !fresh {
            continue;
        }
        let (c0, c1) = (b[k - 1].close, b[k - 1 + AFTER].close);
        println!(
            "===== Cipher · bougie {} · ensuite {:+.2} % en {AFTER} bougies =====",
            b[k - 1].time,
            (c1 - c0) / c0 * 100.0
        );
        for t in now {
            println!("- {t}");
        }
    }
}

// ---- Argos AI Trend V Pro ----

use argos_script::indicators::ai_trend;

/// The settings values of AI Trend with some keys changed.
fn ai_vals(changes: &[(&str, f64)]) -> Vec<f64> {
    let mut v = argos_script::inputs::defaults(&ai_trend::PARAMS);
    for (k, x) in changes {
        let i = ai_trend::PARAMS.iter().position(|p| p.key == *k).unwrap();
        v[i] = *x;
    }
    v
}

/// Without the bars of the smaller units, every output but the multi-timeframe dots is the same.
#[test]
fn ai_trend_same_without_lower_bars() {
    let h1 = waves(24 * 60);
    let d1 = argos_core::aggregate(&h1, 86_400);
    let s = ai_trend::Settings::from_inputs(&ai_vals(&[("m1_on", 1.0), ("m1_tf", 3.0)]));
    let lower: ai_trend::LowerBars = [(3_600, h1.clone())].into_iter().collect();
    let mut with = ai_trend::compute(&d1, 86_400, &s, &lower);
    let mut without = ai_trend::compute(&d1, 86_400, &s, &ai_trend::LowerBars::new());
    assert!(!with.mtf.is_empty(), "the slot gives dots with its bars");
    with.mtf.clear();
    without.mtf.clear();
    // (NaN is never equal to itself: the outputs are compared as written out)
    assert_eq!(format!("{with:?}"), format!("{without:?}"));
}

/// The last point named is the script's own (with the dots shown), on every bar; nothing about the dots otherwise.
#[test]
fn ai_trend_last_point_is_the_scripts() {
    let b = waves(400);
    let vals = ai_vals(&[("dots_show", 1.0)]);
    let shown: Vec<(&str, f64)> = vec![("dots_show", 1.0)];
    let mut seen = (false, false);
    for k in 60..=b.len() {
        let a = ai_trend::compute(&b[..k], H1, &ai_trend::Settings::from_inputs(&vals), &ai_trend::LowerBars::new());
        let last = [(&a.buy, "point vert"), (&a.sell, "point rouge")]
            .into_iter()
            .filter_map(|(l, nm)| l.last().map(|&i| (nm, i)))
            .max_by_key(|x| x.1);
        let t = text(&read("AiTrend", &b[..k], &shown));
        match last {
            Some((name, i)) if k - 1 - i <= 50 => {
                let when = if k - 1 == i {
                    "sur la dernière bougie".to_string()
                } else {
                    format!("il y a {}", format::bars(k - 1 - i))
                };
                assert!(t.contains(&format!("Dernier point : {name} {when}")), "{t}");
                if name == "point vert" {
                    seen.0 = true
                } else {
                    seen.1 = true
                }
            }
            _ => assert!(t.contains("Aucun point vert ou rouge"), "{t}"),
        }
        // dots hidden by default: never named
        assert!(!text(&read("AiTrend", &b[..k], &[])).contains("Dernier point"));
    }
    assert!(seen.0 && seen.1);
}

#[test]
fn ai_trend_reading_shape() {
    let r = read("AiTrend", &waves(400), &[]);
    assert_eq!(titles(&r), THREE.to_vec());
    let t = text(&r);
    assert!(
        t.contains("AI Trend Index à") && t.contains("sa ligne signal") && t.contains("Boîte HTF (signal 1h)"),
        "{t}"
    );
    // hidden by settings
    let t = text(&read(
        "AiTrend",
        &waves(400),
        &[("sig_show", 0.0), ("htf_box", 0.0), ("face_show", 0.0), ("div_show", 0.0), ("div_hid", 0.0)],
    ));
    assert!(!t.contains("ligne signal (") && !t.contains("Boîte HTF") && !t.contains("Divergence"), "{t}");
    // too few bars: one bar, or not enough for the index
    assert!(ids(&read("AiTrend", &rising(1), &[])).contains(&"too_few"));
    assert!(ids(&read("AiTrend", &rising(5), &[])).contains(&"too_few"));
    // divergences appear (regular and hidden), named as the band draws them
    let mixed = bars(500, |i| {
        100.0 + i as f64 * 0.03 + (i as f64 / 9.0).sin() * 3.0 + (i as f64 / 2.3).sin() * 1.5 + (i as f64 / 41.0).sin() * 6.0
    });
    let all: String = [waves(400), mixed]
        .iter()
        .flat_map(|b| (60..=b.len()).map(|k| text(&read("AiTrend", &b[..k], &[]))))
        .collect();
    assert!(all.contains("régulière (🤑)") && all.contains("cachée : pivot"), "no divergence found");
}

/// 🧪 Real moments of any reading on cached candles (not run unless `ARGOS_EXEMPLES_AI` names a candle file,
/// `ARGOS_EXEMPLES_ID` the indicator and `ARGOS_EXEMPLES_MOT` a phrase of « Ce qu'il montre maintenant »): every bar of
/// the last 600 where the phrase is there and was not on the bar before, with what the price did over the next 12 bars.
#[test]
fn real_moments_from_the_cache() {
    let (Some(path), Ok(id), Ok(word)) = (
        std::env::var_os("ARGOS_EXEMPLES_AI"),
        std::env::var("ARGOS_EXEMPLES_ID"),
        std::env::var("ARGOS_EXEMPLES_MOT"),
    ) else {
        return;
    };
    let b = argos_data::cache::read_raw(std::path::Path::new(&path)).expect("cache file");
    const AFTER: usize = 12;
    let now = |k: usize| -> Vec<String> { read(&id, &b[..k], &[]).blocks[0].sentences.iter().map(|x| x.text.clone()).collect() };
    let start = b.len().saturating_sub(600).max(200);
    let mut before = now(start - 1).join(" ").contains(&word);
    for k in start..b.len() - AFTER {
        let lines = now(k);
        let has = lines.join(" ").contains(&word);
        if has && !before {
            let (c0, c1) = (b[k - 1].close, b[k - 1 + AFTER].close);
            println!(
                "===== {id} · bougie {} · ensuite {:+.2} % en {AFTER} bougies =====",
                b[k - 1].time,
                (c1 - c0) / c0 * 100.0
            );
            for t in &lines {
                println!("- {t}");
            }
        }
        before = has;
    }
}

// ---- Argos TBO ----

use argos_script::indicators::tbo;

/// The last break named is the script's own (with its kind), on every bar; nothing about breaks when they are hidden.
#[test]
fn tbo_last_break_is_the_scripts() {
    let mixed = bars(500, |i| {
        100.0 + i as f64 * 0.03 + (i as f64 / 9.0).sin() * 3.0 + (i as f64 / 2.3).sin() * 1.5 + (i as f64 / 41.0).sin() * 6.0
    });
    let vals = argos_script::inputs::defaults(&tbo::PARAMS);
    let mut kinds = vec![];
    for b in [waves(400), mixed] {
        for k in 80..=b.len() {
            let t = tbo::compute(&b[..k], H1, &tbo::Settings::from_inputs(&vals));
            let text_k = text(&read("Tbo", &b[..k], &[]));
            match t.breaks.iter().max_by_key(|x| x.index).filter(|x| k - 1 - x.index <= 50) {
                Some(x) => {
                    let side = if x.up { "haussier ▲" } else { "baissier ▼" };
                    let when = if k - 1 == x.index {
                        "sur la dernière bougie".to_string()
                    } else {
                        format!("il y a {}", format::bars(k - 1 - x.index))
                    };
                    assert!(text_k.contains(&format!("Dernier break {side}")) && text_k.contains(&when), "{k}: {text_k}");
                    if !kinds.contains(&x.kind) {
                        kinds.push(x.kind);
                    }
                }
                None => assert!(text_k.contains("Aucun point de break"), "{text_k}"),
            }
        }
    }
    assert!(kinds.len() >= 2, "{kinds:?}");
    let hidden = text(&read(
        "Tbo",
        &waves(400),
        &[("dotsU", 0.0), ("dotsD", 0.0), ("sig_on", 0.0), ("div_on", 0.0), ("paint", 0.0)],
    ));
    assert!(
        !hidden.contains("break") && !hidden.contains("ligne Signal (") && !hidden.contains("Divergence") && !hidden.contains("Vector"),
        "{hidden}"
    );
}

#[test]
fn tbo_vector_status_and_too_few() {
    // a big volume on the last bar: a Vector 200+ candle, named as the app does
    let mut b = waves(300);
    let last = b.len() - 1;
    b[last].volume = 5_000.0;
    let t = text(&read("Tbo", &b, &[]));
    assert!(t.contains("bougie Vector 200+"), "{t}");
    assert!(!text(&read("Tbo", &b, &[("paint", 0.0)])).contains("La dernière bougie est une bougie Vector"));
    // the status table only when shown
    let st = text(&read("Tbo", &b, &[("dash", 1.0)]));
    assert!(st.contains("Tableau de statut : filtres « ") && st.contains("dernier événement"), "{st}");
    assert!(!t.contains("Tableau de statut"));
    assert!(ids(&read("Tbo", &rising(1), &[])).contains(&"too_few"));
}

/// « 1 bougies » (but not « 21 bougies »): the number 1 alone before a plural noun.
fn one_plural(t: &str, noun: &str) -> bool {
    let pat = format!("1 {noun}");
    t.match_indices(&pat).any(|(i, _)| !t[..i].ends_with(|c: char| c.is_ascii_digit()))
}

/// No visible text has ✨ (on screen the button is « le rond Argos AI »): every public entry, on several series.
#[test]
fn no_sparkle_in_any_text() {
    let all: Vec<Item> = READ_BY_RULES.iter().map(|id| item(id, id)).collect();
    for b in [rising(300), turn(300), waves(400), rising(3)] {
        let mut texts: Vec<String> = READ_BY_RULES
            .iter()
            .chain(&["Lens"])
            .map(|id| {
                let r = read(id, &b, &[]);
                format!("{} {}", text(&r), r.reminder)
            })
            .collect();
        texts.push(overview_text(&read_together(&all, &b)));
        texts.push(overview_text(&read_timeframes(&all, &b, H1, 4 * H1)));
        texts.push(overview_text(&read_volatility(&b)));
        for t in texts {
            assert!(!t.contains('✨'), "{t}");
        }
    }
}

// ---- Argos TVD Pro ----

use argos_script::indicators::tvd;

/// The last mark named is the script's own, on every bar; the divergence marks are read apart.
#[test]
fn tvd_last_mark_is_the_scripts() {
    let mixed = bars(500, |i| {
        100.0 + i as f64 * 0.03 + (i as f64 / 9.0).sin() * 3.0 + (i as f64 / 2.3).sin() * 1.5 + (i as f64 / 41.0).sin() * 6.0
    });
    let vals = argos_script::inputs::defaults(&tvd::PARAMS);
    let mut seen = 0;
    for b in [waves(400), mixed] {
        for k in 220..=b.len() {
            let t = tvd::compute(&b[..k], &vals);
            let tx = text(&read("Tvd", &b[..k], &[]));
            let div = |m: &&tvd::Mark| matches!(m.kind, tvd::MarkKind::Bull | tvd::MarkKind::HBull | tvd::MarkKind::Bear | tvd::MarkKind::HBear);
            match t.marks.iter().filter(|m| !div(m)).max_by_key(|m| m.index).filter(|m| k - 1 - m.index <= 50) {
                Some(m) => {
                    let when = if k - 1 == m.index {
                        "sur la dernière bougie".to_string()
                    } else {
                        format!("il y a {}", format::bars(k - 1 - m.index))
                    };
                    assert!(tx.contains("Dernier repère : ") && tx.contains(&when), "{k}: {:?} / {tx}", m.kind);
                    seen += 1;
                }
                None => assert!(!tx.contains("Dernier repère"), "{tx}"),
            }
        }
    }
    assert!(seen > 0);
}

#[test]
fn tvd_shown_parts_only() {
    let b = waves(400);
    let t = text(&read("Tvd", &b, &[]));
    assert!(
        t.contains("EMA rapide à") && t.contains("Nuage ") && t.contains("Tableau Argos TVD Pro : AI Index / Signal : "),
        "{t}"
    );
    let off = text(&read(
        "Tvd",
        &b,
        &[
            ("showCloud", 0.0),
            ("showTable", 0.0),
            ("showSignals", 0.0),
            ("showAiSignals", 0.0),
            ("showPatterns", 0.0),
            ("showVector", 0.0),
        ],
    ));
    assert!(
        !off.contains("Nuage") && !off.contains("Tableau") && !off.contains("Dernier repère") && !off.contains("bougie Vector"),
        "{off}"
    );
    assert!(ids(&read("Tvd", &rising(1), &[])).contains(&"too_few"));
}

// ---- Argos Vector Zone ----

use argos_script::indicators::vector_zone;

fn vz(b: &[Bar]) -> vector_zone::VectorZone {
    vector_zone::compute(b, H1, b[b.len() - 1].time + H1, &argos_script::inputs::defaults(&vector_zone::PARAMS))
}

/// The zones named are the script's own nearest ones (at most 3), and the last label is the script's own, on every
/// bar of two series.
#[test]
fn vector_zone_is_the_scripts() {
    let mixed = bars(500, |i| {
        100.0 + i as f64 * 0.03 + (i as f64 / 9.0).sin() * 3.0 + (i as f64 / 2.3).sin() * 1.5 + (i as f64 / 41.0).sin() * 6.0
    });
    let (mut zones_seen, mut labels_seen) = (0, 0);
    for b in [waves(400), mixed] {
        for k in (60..=b.len()).step_by(3) {
            let v = vz(&b[..k]);
            let t = text(&read("VectorZone", &b[..k], &[]));
            let c = b[k - 1].close;
            let dist = |z: &vector_zone::Zone| {
                if c > z.top {
                    c - z.top
                } else if c < z.bot {
                    z.bot - c
                } else {
                    0.0
                }
            };
            let mut zs: Vec<_> = v.zones.iter().collect();
            zs.sort_by(|a, b| dist(a).total_cmp(&dist(b)));
            for z in zs.iter().take(3) {
                assert!(t.contains(&format!("de {} à {}", format::price(z.bot), format::price(z.top))), "{k}: {t}");
                zones_seen += 1;
            }
            assert_eq!(t.matches("Zone « ").count(), zs.len().min(3), "{t}");
            if let Some(&(i, _)) = v.signals.iter().max_by_key(|x| x.0).filter(|x| k - 1 - x.0 <= 50) {
                let when = if k - 1 == i {
                    "sur la dernière bougie".to_string()
                } else {
                    format!("il y a {}", format::bars(k - 1 - i))
                };
                assert!(t.contains("Dernier événement : ") && t.contains(&when), "{t}");
                labels_seen += 1;
            } else {
                assert!(!t.contains("Dernier événement"), "{t}");
            }
        }
    }
    assert!(zones_seen > 0 && labels_seen > 0);
}

#[test]
fn vector_zone_hidden_and_too_few() {
    let off = text(&read(
        "VectorZone",
        &waves(400),
        &[
            ("showZones", 0.0),
            ("showLiq", 0.0),
            ("showVec", 0.0),
            ("showTL", 0.0),
            ("showPat", 0.0),
            ("showChart", 0.0),
            ("showFib", 0.0),
        ],
    ));
    assert!(
        !off.contains("Zone « ") && !off.contains("Liquidité") && !off.contains("bougie VECTOR") && !off.contains("bougie CLIMAX"),
        "{off}"
    );
    assert!(ids(&read("VectorZone", &rising(1), &[])).contains(&"too_few"));
}

// ---- Argos OB · BB · FVG · iFVG (Smc) and Argos SMC (SmcPro) ----

use argos_script::indicators::{smc, smc_pro};

/// The last break named is the script's own, and every zone named is an active zone of the script with that label.
#[test]
fn smc_is_the_scripts() {
    let mixed = bars(500, |i| {
        100.0 + i as f64 * 0.03 + (i as f64 / 9.0).sin() * 3.0 + (i as f64 / 2.3).sin() * 1.5 + (i as f64 / 41.0).sin() * 6.0
    });
    let vals = argos_script::inputs::defaults(&smc::PARAMS);
    let (mut zones, mut breaks) = (0, 0);
    for b in [waves(400), mixed, walk(600)] {
        for k in (60..=b.len()).step_by(4) {
            let r = smc::compute(&b[..k], H1, &smc::Settings::from_inputs(&vals));
            let t = text(&read("Smc", &b[..k], &[]));
            let shown: Vec<_> = r.breaks.iter().collect();
            if let Some(x) = shown.last() {
                let name = format!(
                    "{} {}",
                    if x.kind == smc::BreakType::Choch { "CHoCH" } else { "BOS" },
                    if x.up { "haussier" } else { "baissier" }
                );
                assert!(
                    t.contains(&format!("Dernière cassure de structure : {name} au niveau {}", format::price(x.price))),
                    "{k}: {t}"
                );
                breaks += 1;
            }
            let act: Vec<(f64, f64)> = r
                .order_blocks
                .iter()
                .filter(|o| o.end.is_none())
                .map(|o| (o.bot, o.top))
                .chain(r.fvgs.iter().filter(|f| f.end.is_none()).map(|f| (f.bot, f.top)))
                .collect();
            let named = t.matches("Zone « ").count();
            assert!(named <= 3 && (named > 0) == !act.is_empty(), "{k}: {named} / {} / {t}", act.len());
            for (bot, top) in &act {
                if t.contains(&format!("de {} à {}", format::price(*bot), format::price(*top))) {
                    zones += 1;
                }
            }
        }
    }
    assert!(zones > 0 && breaks > 0, "{zones} {breaks}");
}

#[test]
fn smc_pro_is_the_scripts() {
    let b = walk(600);
    let vals = argos_script::inputs::defaults(&smc_pro::PARAMS);
    let mut breaks = 0;
    for k in (80..=b.len()).step_by(4) {
        let r = smc_pro::compute(&b[..k], H1, &smc_pro::Settings::from_inputs(&vals));
        let t = text(&read("SmcPro", &b[..k], &[]));
        if let Some(e) = r.events.iter().max_by_key(|e| e.at) {
            let name = format!("{} {}", if e.choch { "CHoCH" } else { "BOS" }, if e.up { "haussier" } else { "baissier" });
            assert!(
                t.contains(&format!("Dernière cassure de structure : {name} au niveau {}", format::price(e.level))),
                "{k}: {t}"
            );
            breaks += 1;
        }
        // the plan of the script is not drawn: never read
        assert!(!t.contains("entrée") && !t.contains("stop") && !t.contains("cible"), "{t}");
    }
    assert!(breaks > 0);
    let pd = text(&read("SmcPro", &b, &[("showPD", 1.0)]));
    assert!(pd.contains("% du range") && pd.contains("de l'équilibre"), "{pd}");
}

#[test]
fn smc_hidden_and_too_few() {
    let off: Vec<(&str, f64)> = ["show_ob", "show_bb", "show_fvg", "show_ifvg", "show_bos", "show_choch", "show_eq"]
        .iter()
        .map(|k| (*k, 0.0))
        .collect();
    let t = text(&read("Smc", &waves(400), &off));
    assert!(
        t.contains("Rien d'affiché") && !t.contains("Zone « ") && !t.contains("cassure de structure :"),
        "{t}"
    );
    let off: Vec<(&str, f64)> = ["showOB", "showBB", "showFVG", "showIFVG", "showBos", "showPD", "showOte"]
        .iter()
        .map(|k| (*k, 0.0))
        .collect();
    let t = text(&read("SmcPro", &waves(400), &off));
    assert!(t.contains("Rien d'affiché") && !t.contains("OTE de") && !t.contains("du range"), "{t}");
    for id in ["Smc", "SmcPro"] {
        assert!(ids(&read(id, &rising(1), &[])).contains(&"too_few"));
    }
}

/// A fixed pseudo-random walk (a simple linear congruential generator) with wicks of varying length: highs and
/// lows like a real market (two bars side by side seldom share their high).
fn walk(n: usize) -> Vec<Bar> {
    let mut x: u64 = 12_345;
    let mut rnd = move || {
        x = x.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        (x >> 33) as f64 / (1u64 << 31) as f64
    };
    let mut c = 100.0;
    (0..n)
        .map(|i| {
            let o = c;
            c += (rnd() - 0.5) * 2.0;
            let (hw, lw) = (0.05 + rnd() * 0.6, 0.05 + rnd() * 0.6);
            Bar {
                time: T0 + i as i64 * H1,
                open: o,
                high: o.max(c) + hw,
                low: o.min(c) - lw,
                close: c,
                volume: 100.0 + rnd() * 80.0,
            }
        })
        .collect()
}

// ---- keys_read: the settings each reading reads ----

use argos_script::classic_inputs as ci;
use argos_script::inputs::{Kind, Param};

/// The settings table of the catalogue for an id with rules.
fn catalogue(id: &str) -> &'static [Param] {
    match id {
        "Rsi" => &ci::RSI_PARAMS,
        "Macd" => &ci::MACD_PARAMS,
        "EmaCross" => &ci::EMA_CROSS_PARAMS,
        "EmaMulti" => &argos_script::v1_inputs::EMA_MULTI_PARAMS,
        "Vwap" => &ci::VWAP_PARAMS,
        "StochRsi" => &ci::STOCH_RSI_PARAMS,
        "Mfi" => &ci::MFI_PARAMS,
        "Stochastic" => &ci::STOCH_PARAMS,
        "Atr" => &ci::ATR_PARAMS,
        "RelVolume" => &ci::REL_VOLUME_PARAMS,
        "Dmi" => &ci::DMI_PARAMS,
        "Supertrend" => &ci::SUPERTREND_PARAMS,
        "Cci" => &ci::CCI_PARAMS,
        "WilliamsR" => &ci::WILLIAMS_PARAMS,
        "Obv" => &ci::OBV_PARAMS,
        "Keltner" => &ci::KELTNER_PARAMS,
        "Donchian" => &ci::DONCHIAN_PARAMS,
        "Psar" => &ci::PSAR_PARAMS,
        "Aroon" => &ci::AROON_PARAMS,
        "Cipher" => &cipher::PARAMS,
        "AiTrend" => &ai_trend::PARAMS,
        "Tbo" => &tbo::PARAMS,
        "Tvd" => &tvd::PARAMS,
        "VectorZone" => &vector_zone::PARAMS,
        "Smc" => &smc::PARAMS,
        "SmcPro" => &smc_pro::PARAMS,
        "PriceAction" => &argos_script::indicators::order_blocks::PARAMS,
        "Bands" => &argos_script::v1_inputs::BANDS_PARAMS,
        "Ichimoku" => &ci::ICHIMOKU_PARAMS,
        "Patterns" => &argos_script::indicators::patterns::PARAMS,
        "SwingVwap" => &ci::PIVOT_VWAP_PARAMS,
        "SmartSwingVwap" => &ci::PIVOT_VWAP_MULTI_PARAMS,
        "AnchoredVwap" => &argos_script::indicators::anchored_vwap::PARAMS,
        "Heatmap" => &argos_script::v1_inputs::HEATMAP_PARAMS,
        "Profile" => &argos_script::v1_inputs::PROFILE_PARAMS,
        "Bubbles" => &argos_script::v1_inputs::BUBBLES_PARAMS,
        "Cvd" => &ci::OBV_PARAMS,
        "MtfTable" => &argos_script::indicators::mtf_table::PARAMS,
        "Tape" => &argos_script::v1_inputs::TAPE_PARAMS,
        "Footprint" => &argos_script::v1_inputs::FOOTPRINT_PARAMS,
        // (their settings are not in argos-script's catalogue: argos-render draws them with its own, and the reading
        // uses what the screen computed)
        "BookHeat" | "LiqMap" => &[],
        "Sma" | "Wma" | "Hma" | "Vwma" | "Smma" | "MaRibbon" | "Bollinger" => argos_script::catalogue::reglages(argos_script::catalogue::fiche(id).unwrap()).1,
        "BandsOsc" => &argos_script::v1_inputs::BANDS_OSC_PARAMS,
        "SpringTension" => argos_script::catalogue::reglages(argos_script::catalogue::fiche("SpringTension").unwrap()).1,
        _ => panic!("{id}: no catalogue table in this test"),
    }
}

#[test]
fn keys_read_are_in_the_catalogue() {
    for id in READ_BY_RULES {
        let keys = keys_read(id);
        // (BookHeat and LiqMap: their settings are not in argos-script's catalogue, argos-render draws them with its
        // own; the reading uses what the screen computed with them, so it reads no setting)
        assert_eq!(keys.is_empty(), matches!(id, "BookHeat" | "LiqMap"), "{id}");
        let table = catalogue(id);
        for k in keys {
            assert!(table.iter().any(|p| p.key == *k), "{id}: « {k} » is not a key of its catalogue table");
        }
        let mut sorted = keys.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), keys.len(), "{id}: a key twice");
        // the same table gives the roles of Reading::settings
        let all: Vec<(&str, f64)> = table.iter().map(|p| (p.key, p.def)).collect();
        // (Reading::settings follows the order of the catalogue, keys_read the order of the reading)
        let mut with_role: Vec<String> = read(id, &waves(300), &all)
            .settings
            .into_iter()
            .filter(|x| x.role.is_some())
            .map(|x| x.key)
            .collect();
        with_role.sort_unstable();
        let mut named: Vec<String> = rules::roles_of(id).unwrap().iter().map(|(k, _)| k.to_string()).collect();
        named.sort_unstable();
        assert_eq!(with_role, named, "{id}");
        // keys_read: the named keys first, then the extra ones
        let n = rules::roles_of(id).unwrap().len();
        assert!(keys.iter().take(n).zip(rules::roles_of(id).unwrap()).all(|(k, (r, _))| k == r), "{id}");
        assert_eq!(&keys[n..], rules::extra_keys(id), "{id}");
    }
    for id in ["Lens", "", "Nope"] {
        assert!(keys_read(id).is_empty(), "{id}");
    }
}

/// Every setting the code of the rules asks for by name (p.get / on / len / pick / rgb("…"), and the keys of the
/// multiple EMAs) is in keys_read: the rules can only read settings through that table.
#[test]
fn rules_read_settings_only_through_keys_read() {
    let files: [(&str, &str, &[(&str, &str)]); 18] = [
        (
            "rules.rs",
            include_str!("rules.rs"),
            &[
                ("rsi", "Rsi"),
                ("macd", "Macd"),
                ("ema_cross", "EmaCross"),
                ("ema_multi", "EmaMulti"),
                ("vwap", "Vwap"),
            ],
        ),
        (
            "rules/more.rs",
            include_str!("rules/more.rs"),
            &[
                ("stoch_rsi", "StochRsi"),
                ("mfi", "Mfi"),
                ("stochastic", "Stochastic"),
                ("cci", "Cci"),
                ("williams", "WilliamsR"),
                ("aroon", "Aroon"),
                ("bands", "Bands"),
                ("bands_osc", "BandsOsc"),
                ("ichimoku", "Ichimoku"),
                ("atr", "Atr"),
                ("rel_volume", "RelVolume"),
                ("obv", "Obv"),
                ("dmi", "Dmi"),
                ("supertrend", "Supertrend"),
                ("psar", "Psar"),
                ("bollinger", "Bollinger"),
                ("keltner", "Keltner"),
                ("donchian", "Donchian"),
            ],
        ),
        ("rules/cipher.rs", include_str!("rules/cipher.rs"), &[("*", "Cipher")]),
        ("rules/ai_trend.rs", include_str!("rules/ai_trend.rs"), &[("*", "AiTrend")]),
        ("rules/tbo.rs", include_str!("rules/tbo.rs"), &[("*", "Tbo")]),
        ("rules/tvd.rs", include_str!("rules/tvd.rs"), &[("*", "Tvd")]),
        ("rules/vector_zone.rs", include_str!("rules/vector_zone.rs"), &[("*", "VectorZone")]),
        ("rules/smc.rs", include_str!("rules/smc.rs"), &[("read_smc", "Smc"), ("read_pro", "SmcPro")]),
        ("rules/spring.rs", include_str!("rules/spring.rs"), &[("read", "SpringTension")]),
        ("rules/price_action.rs", include_str!("rules/price_action.rs"), &[("*", "PriceAction")]),
        ("rules/patterns.rs", include_str!("rules/patterns.rs"), &[("*", "Patterns")]),
        (
            "rules/pivot_vwap.rs",
            include_str!("rules/pivot_vwap.rs"),
            &[("read_one", "SwingVwap"), ("read_multi", "SmartSwingVwap")],
        ),
        ("rules/anchored_vwap.rs", include_str!("rules/anchored_vwap.rs"), &[("*", "AnchoredVwap")]),
        (
            "rules/volume.rs",
            include_str!("rules/volume.rs"),
            &[("read_heatmap", "Heatmap"), ("read_profile", "Profile")],
        ),
        ("rules/averages.rs", include_str!("rules/averages.rs"), &[("read_one", "Sma")]),
        (
            "rules/flow.rs",
            include_str!("rules/flow.rs"),
            &[("read_bubbles", "Bubbles"), ("read_cvd", "Cvd"), ("read_footprint", "Footprint")],
        ),
        ("rules/mtf_table.rs", include_str!("rules/mtf_table.rs"), &[("*", "MtfTable")]),
        ("rules/live.rs", include_str!("rules/live.rs"), &[("read_tape", "Tape")]),
    ];
    let mut checked = 0;
    for (file, src, fns) in files {
        for (f, id) in fns {
            // the body of `fn f(` up to the next item at the start of a line (or the whole file for « * »)
            let body = if *f == "*" {
                src
            } else {
                let at = src.find(&format!("fn {f}(")).unwrap_or_else(|| panic!("{file}: no fn {f}"));
                let rest = &src[at..];
                &rest[..rest[1..]
                    .find("\nfn ")
                    .or_else(|| rest[1..].find("\npub(super) fn "))
                    .map_or(rest.len(), |e| e + 1)]
            };
            let mut keys: Vec<&str> = vec![];
            for call in [".get(\"", ".on(\"", ".len(\"", ".pick(\"", ".rgb(\"", ".setting(\""] {
                for (i, _) in body.match_indices(call) {
                    let k = &body[i + call.len()..];
                    keys.push(&k[..k.find('"').unwrap()]);
                }
            }
            if *id == "EmaMulti" {
                keys.extend(["p1", "p2", "p3", "p4", "p5", "p6"]);
            }
            assert!(!keys.is_empty(), "{file} {f}: no setting read?");
            for k in keys {
                assert!(keys_read(id).contains(&k), "{file} {f} ({id}) reads « {k} », which is not in keys_read");
                checked += 1;
            }
        }
    }
    assert!(checked > 60, "{checked}");
}

/// Every box of keys_read changes what the reading says, on some series, with the other boxes on or off.
#[test]
fn every_box_of_keys_read_changes_the_reading() {
    let mut big = waves(300);
    let last = big.len() - 1;
    big[last].volume = 5_000.0;
    // (a zigzag of equal moves: no trend at all, a very low ADX)
    let zigzag = bars(300, |i| 100.0 + (i % 2) as f64 * 0.5);
    // (a saw with equal highs and lows every 20 bars: open EQH / EQL)
    let saw: Vec<Bar> = (0..400)
        .map(|i| {
            let tri = |k: usize| 100.0 + 10.0 - ((k % 20) as f64 - 10.0).abs();
            let (o, c) = (if i == 0 { tri(0) } else { tri(i - 1) }, tri(i));
            let wick = if i % 20 == 10 || i % 20 == 0 { 0.6 } else { 0.1 };
            Bar {
                time: T0 + i as i64 * H1,
                open: o,
                high: o.max(c) + wick,
                low: o.min(c) - wick,
                close: c,
                volume: 100.0,
            }
        })
        .collect();
    let series = [walk(600), waves(400), flat(300), big, zigzag, saw];
    for id in READ_BY_RULES {
        let table = catalogue(id);
        // (the boxes the reading names; the extra keys are tested by no_setting_outside_keys_read_changes_the_reading)
        let boxes: Vec<&str> = rules::roles_of(id)
            .unwrap()
            .iter()
            .map(|(k, _)| *k)
            .filter(|k| table.iter().any(|p| p.key == *k && matches!(p.kind, Kind::Bool)))
            .collect();
        for k in &boxes {
            let changes = [1.0, 0.0].iter().any(|others| {
                let mut base: Vec<(&str, f64)> = boxes.iter().filter(|x| *x != k).map(|x| (*x, *others)).collect();
                // (the ADX filter of the TBO only matters under its threshold: the Custom preset with a high one)
                if id == "Tbo" {
                    base.extend([("preset", 0.0), ("adxMin", 100.0)]);
                }
                // (on several moments of each series: some boxes only matter on some bars)
                series.iter().any(|b| {
                    (100..=b.len()).rev().step_by(37).any(|n| {
                        let with = |v: f64| {
                            let mut st = base.clone();
                            st.push((k, v));
                            text(&read(id, &b[..n], &st))
                        };
                        with(1.0) != with(0.0)
                    })
                })
            });
            assert!(changes, "{id}: « {k} » changes nothing in the reading");
        }
    }
}

// ---- Spring Tension ----

/// The marks named are the run's own on every bar of several series: the last « Long » / « LONG NUKE », « Short » /
/// « SHORT NUKE », Spring / Upthrust, and the NUKE in progress.
#[test]
fn spring_tension_is_the_scripts() {
    let mut seen: Vec<&str> = vec![];
    for b in [walk(600), waves(400), turn(300)] {
        for k in (60..=b.len()).step_by(3) {
            let (out, _) = rules::spring_run(&b[..k], &[]).expect("the script runs");
            let t = text(&read("SpringTension", &b[..k], &[]));
            let n = k;
            for (dot, cross) in [("Long", "LONG NUKE"), ("Short", "SHORT NUKE")] {
                let bars_of = |title: &str| out.shapes.iter().filter(|x| x.title == title).map(|x| x.bar).collect::<Vec<_>>();
                let (dots, crosses) = (bars_of(dot), bars_of(cross));
                // a NUKE replaces the dot on its bar
                assert!(dots.iter().all(|i| !crosses.contains(i)), "{dot} and {cross} on the same bar");
                match dots.iter().chain(&crosses).max().copied().filter(|i| n - 1 - i <= 50) {
                    Some(i) => {
                        let name = if crosses.contains(&i) { cross } else { dot };
                        let when = if n - 1 == i {
                            "sur la dernière bougie".to_string()
                        } else {
                            format!("il y a {}", format::bars(n - 1 - i))
                        };
                        assert!(t.contains(&format!("Dernière marque « {name} »")) && t.contains(&when), "{k}: {t}");
                        if !seen.contains(&name) {
                            seen.push(name);
                        }
                    }
                    None => assert!(t.contains(&format!("Aucune marque « {dot} »")), "{t}"),
                }
                assert_eq!(crosses.last() == Some(&(n - 1)), t.contains(&format!("« {cross} » en cours")), "{t}");
            }
            for (title, name) in [("🌱 Bullish Spring", "Spring"), ("🥀 Bearish Upthrust", "Upthrust")] {
                let on = out.alertconditions.iter().find(|a| a.0 == title).unwrap();
                if on.2.last() == Some(&1.0) {
                    assert!(t.contains(&format!("Dernier {name} : sur la dernière bougie")), "{t}");
                    if !seen.contains(&name) {
                        seen.push(name);
                    }
                }
            }
        }
    }
    for name in ["Long", "Short", "Spring", "Upthrust"] {
        assert!(seen.contains(&name), "{name} never seen: {seen:?}");
    }
}

/// The RSI and the money flow cited are the script's own variables on the last bar; the marks agree with the RSI:
/// a « Long » point only under 40, a « Short » point only above 60.
#[test]
fn spring_tension_rsi_and_flow_are_the_scripts() {
    let b = walk(600);
    let (out, vars) = rules::spring_run(&b, &[]).unwrap();
    let (rsi, mfi) = (&vars["rsiVal"], &vars["mfiVal"]);
    let t = text(&read("SpringTension", &b, &[]));
    assert!(t.contains(&format!("RSI 14 à {}", format::num(rsi[599], 1))), "{t}");
    assert!(t.contains(&format!("flux d'argent du script (MFI) à {}", format::num(mfi[599], 1))), "{t}");
    for sh in &out.shapes {
        match sh.title.as_str() {
            "Long" => assert!(rsi[sh.bar] < 40.0, "{}", rsi[sh.bar]),
            "Short" => assert!(rsi[sh.bar] > 60.0, "{}", rsi[sh.bar]),
            _ => {}
        }
    }
    // without volume and with « Désactiver »: no number for the money flow
    let mut dry = b.clone();
    for x in &mut dry {
        x.volume = 0.0;
    }
    assert!(text(&read("SpringTension", &dry, &[("Si pas de volume", 1.0)])).contains("flux d'argent du script (MFI) sans valeur"));
}

#[test]
fn spring_tension_hidden_no_volume_and_too_few() {
    let b = walk(600);
    let off = text(&read("SpringTension", &b, &[("Short", 0.0), ("Long", 0.0)]));
    assert!(!off.contains("« Long") && !off.contains("« Short") && !off.contains("NUKE"), "{off}");
    // « Fond » only changes the background
    assert_eq!(text(&read("SpringTension", &b, &[("Fond", 0.0)])), text(&read("SpringTension", &b, &[])));
    // no volume, in both modes
    let mut dry = b.clone();
    for x in &mut dry {
        x.volume = 0.0;
    }
    let t = text(&read("SpringTension", &dry, &[]));
    assert!(
        t.contains("Pas de volume") && t.contains("RSI sur hlc3") && !t.contains("Dernière marque"),
        "{t}"
    );
    let t = text(&read("SpringTension", &dry, &[("Si pas de volume", 1.0)]));
    assert!(t.contains("Pas de volume") && t.contains("désactivé"), "{t}");
    assert!(ids(&read("SpringTension", &rising(1), &[])).contains(&"too_few"));
    assert!(ids(&read("SpringTension", &rising(5), &[])).contains(&"too_few"));
}

/// « Long », « Short », « LONG NUKE », « SHORT NUKE » appear only as the names of the marks, between « »; no other
/// English word of an order.
#[test]
fn spring_tension_names_only() {
    for b in [walk(600), waves(400)] {
        let t = text(&read("SpringTension", &b, &[]));
        let mut rest = t.clone();
        for name in ["« LONG NUKE »", "« SHORT NUKE »", "« Long »", "« Short »"] {
            rest = rest.replace(name, "");
        }
        let words: Vec<String> = rest.split(|c: char| !c.is_alphabetic()).map(|w| w.to_lowercase()).collect();
        for w in ["long", "short", "nuke", "buy", "sell", "achat", "vente", "acheter", "vendre", "entrer", "ordre"] {
            assert!(!words.iter().any(|x| x == w), "« {w} » outside the names of marks: {t}");
        }
    }
}

/// Other valid values of a setting: the box flipped, a number moved inside its bounds, another choice.
fn other_values(p: &Param) -> Vec<f64> {
    match p.kind {
        Kind::Bool => vec![if p.def != 0.0 { 0.0 } else { 1.0 }],
        Kind::Num(min, max, _) => {
            let clamp = |x: f64| x.clamp(min, max);
            let mut v: Vec<f64> = [clamp(p.def * 2.0), clamp(p.def / 2.0), min, max]
                .into_iter()
                .filter(|x| x.is_finite() && *x != p.def)
                .collect();
            v.dedup();
            v
        }
        Kind::Choice(o) => (0..o.len()).map(|i| i as f64).filter(|x| *x != p.def).collect(),
        _ => vec![],
    }
}

/// A setting outside keys_read never changes what the reading says (numbers, marks, words) on the fixed series:
/// any setting that does is in keys_read. Colours and other style settings are not tried.
#[test]
fn no_setting_outside_keys_read_changes_the_reading() {
    let series = [walk(600), waves(400), turn(300)];
    let mut offenders: Vec<String> = vec![];
    for id in READ_BY_RULES {
        let base: Vec<String> = series.iter().map(|b| text(&read(id, b, &[]))).collect();
        for p in catalogue(id).iter().filter(|p| !keys_read(id).contains(&p.key)) {
            let changes = other_values(p)
                .into_iter()
                .any(|v| series.iter().zip(&base).any(|(b, t)| text(&read(id, b, &[(p.key, v)])) != *t));
            if changes {
                offenders.push(format!("{id}: {}", p.key));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "settings outside keys_read that change the reading:\n{}",
        offenders.join("\n")
    );
}

// ---- Argos Price Action (Order Blocks) ----

/// The zones named are the script's own, nearest first, among the ones the chart shows; hidden sides and labels
/// turned off are never named; the last flip and n/4 label are the script's.
#[test]
fn price_action_names_the_zones_the_chart_draws() {
    use argos_script::indicators::order_blocks;
    let mut seen = 0;
    for b in [walk(600), waves(400)] {
        for n in (60..=b.len()).step_by(23) {
            let vals = argos_script::inputs::defaults(&order_blocks::PARAMS);
            let ob = order_blocks::compute(&b[..n], &vals);
            let t = text(&read("PriceAction", &b[..n], &[]));
            let shown = ob.rising.iter().take(3).count() + ob.falling.iter().take(3).count();
            if shown == 0 {
                assert!(t.contains("Aucune zone affichée."), "{t}");
                continue;
            }
            seen += 1;
            let c = b[n - 1].close;
            let d = |z: &order_blocks::Zone| {
                if c > z.high {
                    c - z.high
                } else if c < z.low {
                    z.low - c
                } else {
                    0.0
                }
            };
            let nearest = ob
                .rising
                .iter()
                .take(3)
                .chain(ob.falling.iter().take(3))
                .min_by(|x, y| d(x).total_cmp(&d(y)))
                .unwrap();
            assert!(
                t.contains(&format!("de {} à {}", format::price(nearest.low), format::price(nearest.high))),
                "{t}"
            );
            assert!(t.matches("Zone haussière").count() + t.matches("Zone baissière").count() <= 3, "{t}");
            let flips = text(&read("PriceAction", &b[..n], &[("polarite", 1.0)]));
            assert!(!t.contains("polarité :"), "{t}");
            if let Some(f) = ob.flips.iter().filter(|f| n - 1 - f.at <= 50).max_by_key(|f| f.at) {
                assert!(
                    flips.contains(&format!("Dernier changement de polarité : « {} »", if f.support { "▲" } else { "▼" })),
                    "{flips}"
                );
            }
            // hidden sides and labels
            let up_off = text(&read("PriceAction", &b[..n], &[("nHaut", 0.0)]));
            assert!(!up_off.contains("Zone haussière"), "{up_off}");
            let dn_off = text(&read("PriceAction", &b[..n], &[("nBas", 0.0)]));
            assert!(!dn_off.contains("Zone baissière"), "{dn_off}");
            let plain = text(&read("PriceAction", &b[..n], &[("buySell", 0.0), ("pwr", 0.0), ("n4", 0.0), ("polarite", 0.0)]));
            for w in ["« BUY", "« SELL", "Pwr", "BULL", "BEAR", "polarité :"] {
                assert!(!plain.contains(w), "{w}: {plain}");
            }
        }
    }
    assert!(seen > 5, "too few moments with zones: {seen}");
}

// ---- Argos Bollinger Bands and Argos Bollinger Oscillator ----

/// The ▲BB / ▼BB marks and the squeeze named are the chart's own (the same conditions as classic_plots.rs), on
/// every moment; nothing about them when their box is off.
#[test]
fn bands_marks_are_the_charts() {
    let vals = |touch: f64| {
        let mut v = argos_script::inputs::defaults(&argos_script::v1_inputs::BANDS_PARAMS);
        v[5] = touch;
        v
    };
    let (mut touches, mut squeezes) = (0, 0);
    for b in [walk(600), waves(400)] {
        for n in (150..=b.len()).step_by(7) {
            let a = classic::argos_bands(&b[..n], &vals(1.0));
            let (u, l) = (&a.bands.upper, &a.bands.lower);
            let last = (1..n).rev().take(51).find_map(|i| {
                let (c, c0) = (b[i].close, b[i - 1].close);
                if c > u[i] && c0 <= u[i - 1] {
                    Some("« ▲BB »")
                } else if c < l[i] && c0 >= l[i - 1] {
                    Some("« ▼BB »")
                } else {
                    None
                }
            });
            let t = text(&read("Bands", &b[..n], &[("touch", 1.0)]));
            match last {
                Some(m) => {
                    touches += 1;
                    assert!(t.contains(&format!("Dernière marque {m}")), "{m}: {t}");
                }
                None => assert!(!t.contains("Dernière marque"), "{t}"),
            }
            if a.squeeze[n - 1] {
                squeezes += 1;
                assert!(t.contains("Squeeze en cours"), "{t}");
            }
            let off = text(&read("Bands", &b[..n], &[("touch", 0.0), ("sq", 0.0), ("showTable", 0.0)]));
            assert!(!off.contains("BB »") && !off.contains("Squeeze"), "{off}");
        }
    }
    assert!(touches > 5 && squeezes > 0, "{touches} / {squeezes}");
}

/// The strengths named are the script's last values; the last mark at 50 is the chart's.
#[test]
fn bands_osc_reads_the_scripts_strengths() {
    let mut marks = 0;
    // (a staircase: sudden breaks out of the band, up then down)
    let stairs = bars(400, |i| {
        100.0
            + if (i / 100) % 2 == 0 {
                (i % 100 / 12) as f64 * 8.0
            } else {
                64.0 - (i % 100 / 12) as f64 * 8.0
            }
    });
    // (a narrow band, mult 0.3: the strengths often pass 50)
    for (b, len, mult) in [
        (walk(600), 20.0, 1.5),
        (waves(400), 20.0, 1.5),
        (stairs, 5.0, 1.5),
        (walk(600), 20.0, 0.3),
        (waves(400), 10.0, 0.3),
    ] {
        let mut vals = argos_script::inputs::defaults(&argos_script::v1_inputs::BANDS_OSC_PARAMS);
        vals[0] = len;
        vals[1] = mult;
        for n in (60..=b.len()).step_by(11) {
            let (u, d) = classic::bollinger_oscillator(&b[..n], &vals);
            let t = text(&read("BandsOsc", &b[..n], &[("sig", 1.0), ("len", len), ("mult", mult)]));
            assert!(
                t.contains(&format!(
                    "Force haussière à {}, force baissière à {}",
                    format::num(u[n - 1], 1),
                    format::num(d[n - 1], 1)
                )),
                "{t}"
            );
            let cross = |v: &[f64], i: usize| v[i - 1] <= 50.0 && v[i] > 50.0;
            let last = (1..n).rev().take(51).find_map(|i| {
                if cross(&u, i) {
                    Some("vers le haut")
                } else if cross(&d, i) {
                    Some("vers le bas")
                } else {
                    None
                }
            });
            match last {
                Some(m) => {
                    marks += 1;
                    assert!(t.contains(&format!("Dernière marque : flèche {m}")), "{t}");
                }
                None => assert!(!t.contains("Dernière marque"), "{t}"),
            }
            assert!(!text(&read("BandsOsc", &b[..n], &[("len", len), ("mult", mult)])).contains("Dernière marque"));
        }
    }
    assert!(marks > 3, "{marks}");
}

// ---- Ichimoku ----

/// The cloud, Tenkan and Kijun named are the script's last values on every moment; the Chikou compares the close
/// with the close `shift` bars before; too few bars says so.
#[test]
fn ichimoku_reads_the_scripts_lines() {
    use argos_script::indicators::textbook;
    let mut seen = [0; 3];
    for (b, shift) in [(walk(600), 26.0), (waves(400), 26.0), (walk(600), 0.0)] {
        for n in (90..=b.len()).step_by(13) {
            let i = textbook::ichimoku_n(&b[..n], 9, 26, 52, shift as usize);
            let r = read("Ichimoku", &b[..n], &[("shift", shift)]);
            let t = text(&r);
            let (a, s) = (i.span_a[n - 1], i.span_b[n - 1]);
            let c = b[n - 1].close;
            let k = if c > a.max(s) {
                0
            } else if c < a.min(s) {
                1
            } else {
                2
            };
            seen[k] += 1;
            assert!(t.contains(["au-dessus du nuage", "sous le nuage", "dans le nuage"][k]), "{t}");
            assert!(
                t.contains(&format!("Senkou A à {}", format::price(a))) && t.contains(&format!("Senkou B à {}", format::price(s))),
                "{t}"
            );
            assert!(
                t.contains(&format!("Tenkan à {}", format::price(i.tenkan[n - 1]))) && t.contains(&format!("la Kijun à {}", format::price(i.kijun[n - 1]))),
                "{t}"
            );
            assert_eq!(t.contains("Chikou : la clôture actuelle"), shift > 0.0, "{t}");
            if shift > 0.0 {
                // (TradingView's 26 moves the Chikou back by 25 bars: the close of then is bar n − 26)
                assert!(t.contains(&format!("({})", format::price(b[n - 26].close))), "{t}");
            }
        }
    }
    assert!(seen.iter().all(|&x| x > 0), "{seen:?}");
    let few = text(&read("Ichimoku", &walk(60), &[]));
    assert!(read("Ichimoku", &walk(60), &[]).sense.is_none(), "{few}");
}

// ---- Argos Patterns auto ----

/// The figures named are the script's own, most recent first, with the label the chart draws; kinds turned off are
/// never named.
#[test]
fn patterns_names_the_charts_figures() {
    use argos_script::indicators::patterns;
    let mut seen = 0;
    for b in [walk(600), waves(400)] {
        for n in (60..=b.len()).step_by(17) {
            let vals = argos_script::inputs::defaults(&patterns::PARAMS);
            let list = patterns::compute(&b[..n], &vals);
            let t = text(&read("Patterns", &b[..n], &[]));
            let figs: Vec<_> = list.iter().filter(|o| o.fib.is_none()).collect();
            match figs.first() {
                Some(o) => {
                    seen += 1;
                    assert!(t.contains("Figure la plus récente « ") && t.contains(o.name), "{}: {t}", o.name);
                    assert!(t.contains(&format!("{}{}", o.short, if o.conf { " ✓ »" } else { " »" })), "{t}");
                }
                None => assert!(t.contains("Aucune figure affichée."), "{t}"),
            }
            assert_eq!(t.contains("Canal Fibo"), list.iter().any(|o| o.fib.is_some()), "{t}");
            let off = text(&read(
                "Patterns",
                &b[..n],
                &[("double", 0.0), ("hs", 0.0), ("tri", 0.0), ("chan", 0.0), ("triple", 0.0), ("fibch", 0.0)],
            ));
            assert!(off.contains("Aucune figure affichée.") && !off.contains("Canal Fibo"), "{off}");
        }
    }
    assert!(seen > 5, "{seen}");
    assert!(read("Patterns", &walk(30), &[]).sense.is_none());
}

// ---- Argos Pivot VWAP and Argos Pivot VWAP Multi ----

/// The active VWAP and the last marks named are the script's own (one scale and several), on every moment.
#[test]
fn pivot_vwap_reads_the_scripts_legs() {
    use argos_script::indicators::pivot_vwap;
    let (mut active, mut cut) = (0, 0);
    for b in [walk(600), waves(400)] {
        for n in (60..=b.len()).step_by(11) {
            for (id, r) in [
                ("SwingVwap", pivot_vwap::compute(&b[..n], &pivot_vwap::Config::default())),
                ("SmartSwingVwap", pivot_vwap::compute_multi(&b[..n], &pivot_vwap::Config::default())),
            ] {
                let t = text(&read(id, &b[..n], &[]));
                match r.legs.iter().rev().find(|l| l.points.last().is_some_and(|p| p.0 + 1 == n)) {
                    Some(l) => {
                        active += 1;
                        let v = l.points.last().unwrap().1;
                        assert!(t.contains(&format!("VWAP actif à {}", format::price(v))), "{id}: {t}");
                    }
                    None if !r.legs.is_empty() => {
                        cut += 1;
                        assert!(t.contains("Aucun VWAP actif"), "{id}: {t}");
                    }
                    None => {}
                }
                for low in [false, true] {
                    if let Some(p) = r.pivots.iter().rev().find(|p| p.is_low == low && p.mark != pivot_vwap::Mark::First) {
                        assert!(
                            t.contains(&format!("« {} »", p.mark.short())) && t.contains(&format::price(p.price)),
                            "{id}: {t}"
                        );
                    }
                }
            }
        }
    }
    assert!(active > 10 && cut > 0, "{active} / {cut}");
}

// ---- Argos Anchored VWAP ----

/// The VWAP named is the script's own from the anchor; bands turned off are never named; a source from another
/// indicator is not read.
#[test]
fn anchored_vwap_reads_the_scripts_vwap() {
    use argos_script::indicators::anchored_vwap;
    for b in [walk(600), waves(400)] {
        for n in (30..=b.len()).step_by(19) {
            for anchor in [0.0, (b[n / 2].time * 1000) as f64] {
                let mut vals = argos_script::inputs::defaults(&anchored_vwap::PARAMS);
                vals[0] = anchor;
                let a = anchored_vwap::compute(&b[..n], &vals, &|_, _| None);
                let t = text(&read("AnchoredVwap", &b[..n], &[("anchor", anchor)]));
                assert!(t.contains(&format!("à {} : la clôture", format::price(a.vwap[n - 1]))), "{t}");
                assert!(t.contains("bandes #1") && !t.contains("bandes #2"), "{t}");
                assert_eq!(t.contains("ancrage par défaut"), anchor == 0.0, "{t}");
                let off = text(&read("AnchoredVwap", &b[..n], &[("anchor", anchor), ("b1", 0.0)]));
                assert!(!off.contains("bandes #"), "{off}");
            }
        }
    }
    let ext = read("AnchoredVwap", &walk(300), &[("src", argos_script::inputs::source_ext(3, 0))]);
    assert!(text(&ext).contains("ne la lit pas encore") && ext.sense.is_none());
}

// ---- Argos Heatmap Volume and Argos Volume Profile ----

/// The tier and the profile named are the script's own on every moment.
#[test]
fn heatmap_and_profile_read_the_scripts_values() {
    const TIERS: [&str; 5] = ["Très fort", "Fort", "Moyen", "Normal", "Faible"];
    let hv = argos_script::inputs::defaults(&argos_script::v1_inputs::HEATMAP_PARAMS);
    let pv = argos_script::inputs::defaults(&argos_script::v1_inputs::PROFILE_PARAMS);
    let mut big = waves(400);
    for k in (50..400).step_by(41) {
        big[k].volume = 3_000.0;
    }
    let mut tiers = [0; 5];
    for b in [walk(600), big] {
        for n in (30..=b.len()).step_by(13) {
            for bump in [1.0, 40.0] {
                let mut bb = b[..n].to_vec();
                bb[n - 1].volume *= bump;
                let h = classic::heatmap_volume(&bb, &hv);
                let t = text(&read("Heatmap", &bb, &[]));
                let k = h.tier[n - 1] as usize;
                tiers[k] += 1;
                assert!(t.contains(&format!("palier « {} »", TIERS[k])), "{t}");
            }
            let pr = classic::volume_profile(&b[..n], &pv).unwrap();
            let poc = pr.bot + pr.step * (pr.poc as f64 + 0.5);
            let t = text(&read("Profile", &b[..n], &[]));
            assert!(t.contains(&format!("POC à {}", format::price(poc))), "{t}");
            assert!(
                t.contains(&format!("Zone de valeur de {}", format::price(pr.bot + pr.step * pr.va_lo as f64))),
                "{t}"
            );
        }
    }
    assert!(tiers[0] > 0 && tiers[4] > 0, "{tiers:?}");
}

// ---- TradingView's moving averages (SMA, WMA, HMA, VWMA, Smoothed MA, MA ribbon) ----

/// The run of a built-in script with settings by key, as the screen runs it.
fn run_builtin(id: &str, b: &[Bar], st: &[(&str, f64)]) -> argos_script::lang::Output {
    use argos_script::{catalogue, lang};
    let src = catalogue::source(catalogue::fiche(id).unwrap()).unwrap();
    let specs = catalogue::entrees(src);
    let (_, params) = lang::reglages::table(&specs);
    let vals: Vec<f64> = params.iter().map(|p| st.iter().find(|(k, _)| *k == p.key).map_or(p.def, |x| x.1)).collect();
    let chosen = lang::reglages::choisies(&specs, &vals, &Default::default());
    lang::run(src, b, H1, &chosen).unwrap()
}

/// The value named is the script's last value of its line, on every moment, for every average and every smoothing.
#[test]
fn averages_cite_the_scripts_values() {
    let last = |o: &argos_script::lang::Output, t: &str| o.plots.iter().find(|p| p.title == t).unwrap().values.last().copied().unwrap();
    for b in [walk(400), waves(300)] {
        for n in (40..=b.len()).step_by(23) {
            for (id, line) in [("Sma", "MA"), ("Wma", "WMA"), ("Hma", "HMA"), ("Vwma", "VWMA"), ("Smma", "SMMA")] {
                let o = run_builtin(id, &b[..n], &[]);
                let t = text(&read(id, &b[..n], &[]));
                assert!(t.contains(&format!(" à {} : la clôture", format::price(last(&o, line)))), "{id}: {t}");
                assert!(!t.contains("SMA-based MA") && !t.contains("décalée"), "{id}: {t}");
            }
            // each smoothing of the SMA: named only when drawn, with its value
            for ty in 1..=6 {
                let st = [("Type", ty as f64)];
                let o = run_builtin("Sma", &b[..n], &st);
                let t = text(&read("Sma", &b[..n], &st));
                let sm = last(&o, "SMA-based MA");
                if sm.is_finite() {
                    assert!(t.contains("SMA-based MA (") && t.contains(&format::price(sm)), "{ty}: {t}");
                }
                assert_eq!(t.contains("Bandes de Bollinger du lissage"), ty == 2 && sm.is_finite(), "{ty}: {t}");
            }
            // the ribbon: only the ticked averages
            let t = text(&read("MaRibbon", &b[..n], &[("MA #3", 0.0), ("MA #4", 0.0)]));
            let o = run_builtin("MaRibbon", &b[..n], &[("MA #3", 0.0), ("MA #4", 0.0)]);
            if last(&o, "MA #2").is_finite() {
                assert!(t.contains(&format!("MA #2 (SMA 50) à {}", format::price(last(&o, "MA #2")))), "{t}");
            }
            assert!(!t.contains("MA #3") && !t.contains("MA #4"), "{t}");
        }
    }
    // the offset, the source, too few bars
    let b = walk(300);
    let t = text(&read("Wma", &b, &[("Offset", 3.0), ("Source", 1.0)]));
    assert!(t.contains("décalée de 3 bougies vers la droite") && t.contains("WMA 9 (Ouverture)"), "{t}");
    assert!(read("Sma", &walk(5), &[]).sense.is_none());
    assert!(read("MaRibbon", &walk(30), &[]).sense.is_none(), "the 200 needs more bars");
    let none = read("MaRibbon", &b, &[("MA #1", 0.0), ("MA #2", 0.0), ("MA #3", 0.0), ("MA #4", 0.0)]);
    assert!(text(&none).contains("Aucune moyenne cochée"));
}

/// The SMA's two « Length » stay apart: « Length@4 » changes the smoothing only.
#[test]
fn sma_smoothing_length_is_its_own_key() {
    let b = walk(300);
    let a = text(&read("Sma", &b, &[("Type", 1.0)]));
    let c = text(&read("Sma", &b, &[("Type", 1.0), ("Length@4", 50.0)]));
    let first = |t: &str| t.split(" : la clôture").next().unwrap_or_default().to_string();
    assert_eq!(first(&a), first(&c), "the average is the same");
    assert_ne!(a, c, "the smoothing changed");
}

// ---- Idea 5: the day's levels, drawn ----

/// The three levels are the Lens card's own (and the VWAP is the Vwap indicator's), cited and drawn; check() keeps the
/// three drawings; the limits say what is missing; no order word.
#[test]
fn day_levels_are_the_lens_cards() {
    use argos_lens::explain::{check, Command, Window};
    let all = Window {
        time_from: i64::MIN,
        time_to: i64::MAX,
        price_low: argos_lens::Decimal::MIN,
        price_high: argos_lens::Decimal::MAX,
    };
    let b = walk(600);
    for n in (60..=b.len()).step_by(29) {
        let d = crate::read_day_levels(&b[..n]);
        let snap = argos_lens::photos(&b[..n], H1, &argos_lens::Config::default(), None).unwrap().pop().unwrap();
        assert_eq!(d.snapshot.as_ref(), Some(&snap), "the Lens card's snapshot");
        let t: String = d
            .overview
            .blocks
            .iter()
            .flat_map(|x| &x.sentences)
            .map(|x| x.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        let kept = check(&d.explanation, &snap, &all);
        let levels: Vec<argos_lens::Decimal> = kept
            .iter()
            .filter_map(|c| if let Command::Level { price, .. } = c { Some(*price) } else { None })
            .collect();
        for (p, word) in [
            (snap.prev_day_high, "Haut de la veille à "),
            (snap.prev_day_low, "Bas de la veille à "),
            (snap.vwap, "VWAP du jour à "),
        ] {
            match p {
                Some(p) => {
                    assert!(levels.contains(&p), "{word}: drawn and accepted");
                    assert!(t.contains(&format!("{word}{}", format::price(p.try_into().unwrap()))), "{t}");
                }
                None => assert!(!t.contains(word), "{t}"),
            }
        }
        assert_eq!(kept.len(), d.explanation.drawings.len(), "check() keeps every drawing");
        assert!(kept.iter().all(|c| matches!(c, Command::Level { .. })), "levels only");
        // the VWAP is the Vwap indicator's
        if let Some(v) = snap.vwap {
            let vt = text(&read("Vwap", &b[..n], &[]));
            assert!(vt.contains(&format::price(v.try_into().unwrap())), "{vt} / {t}");
        }
        assert!(t.contains("UTC"), "{t}");
        for w in ["achète", "vends", "acheter", "vendre", "✨"] {
            assert!(!t.contains(w), "{w}: {t}");
        }
    }
    // no previous day yet (less than a day of 1 h bars): said, not drawn
    let d = crate::read_day_levels(&walk(20));
    let t: String = d
        .overview
        .blocks
        .iter()
        .flat_map(|x| &x.sentences)
        .map(|x| x.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        t.contains("La veille n'est pas complète") && d.explanation.drawings.iter().all(|c| c.sentence() == "vwap"),
        "{t}"
    );
    // daily bars: nothing drawn
    let days: Vec<Bar> = walk(100)
        .into_iter()
        .enumerate()
        .map(|(i, x)| Bar {
            time: T0 + i as i64 * 86_400,
            ..x
        })
        .collect();
    let d = crate::read_day_levels(&days);
    assert!(d.explanation.drawings.is_empty() && d.snapshot.is_none());
    // no volume: no VWAP, said
    let dry: Vec<Bar> = walk(100).into_iter().map(|x| Bar { volume: 0.0, ..x }).collect();
    let d = crate::read_day_levels(&dry);
    let t: String = d
        .overview
        .blocks
        .iter()
        .flat_map(|x| &x.sentences)
        .map(|x| x.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        t.contains("pas de VWAP du jour") && !d.explanation.drawings.iter().any(|c| c.sentence() == "vwap"),
        "{t}"
    );
}

/// 🧪 The day's levels on cached candles (not run unless `ARGOS_EXEMPLES_JOUR` names a candle file): every 7th bar of
/// the last 200, with the move 12 bars later.
#[test]
fn real_day_levels_from_the_cache() {
    let Some(path) = std::env::var_os("ARGOS_EXEMPLES_JOUR") else { return };
    let b = argos_data::cache::read_raw(std::path::Path::new(&path)).expect("cache file");
    for k in (b.len().saturating_sub(200)..b.len() - 12).step_by(7) {
        let d = crate::read_day_levels(&b[..k]);
        let (c0, c1) = (b[k - 1].close, b[k - 1 + 12].close);
        println!(
            "===== bougie {} · ensuite {:+.2} % en 12 bougies · {} dessins =====",
            b[k - 1].time,
            (c1 - c0) / c0 * 100.0,
            d.explanation.drawings.len()
        );
        for x in &d.overview.blocks[0].sentences {
            println!("- {}", x.text);
        }
    }
}

// ---- Lot C: what the screen passes besides the bars ----

/// With no screen data, `read_with` is `read`, for every indicator read by rules.
#[test]
fn read_with_no_data_is_read() {
    let b = walk(400);
    for id in READ_BY_RULES {
        assert_eq!(read_with(id, &b, &[], &ScreenData::default()), read(id, &b, &[]), "{id}");
    }
}

/// The bubbles named are the script's own, from the real trades of the closed bars only: the counts of the last 50
/// bars, the last bubble, the last POC, since when; without the screen's orderflow or without complete trades, said.
#[test]
fn bubbles_are_the_scripts() {
    use argos_core::orderflow::FlowBar;
    use argos_script::indicators::flow;
    let vals = argos_script::inputs::defaults(&flow::BUBBLES_PARAMS);
    let b = walk(300);
    let absent = "Bulles : vrais trades absents pour ce marché ou cette unité.";
    assert!(text(&read("Bubbles", &b, &[])).contains(absent));
    assert!(text(&read_with(
        "Bubbles",
        &b,
        &[],
        &ScreenData {
            flow: Some(&[]),
            ..Default::default()
        }
    ))
    .contains(absent));
    // real trades on the last 30 closed bars (rows of aggressive buyers or sellers), one incomplete, and the bar in
    // progress (left out)
    let step = 10.0;
    let row = |x: &Bar, k: usize| {
        let r = (x.close / step).floor() as i64;
        let (buy, sell) = if k.is_multiple_of(3) { (90.0, 2.0) } else { (2.0, 90.0) };
        vec![(r - 1, 5.0, 5.0), (r, buy, sell), (r + 1, 5.0, 5.0)]
    };
    let mut reel: Vec<FlowBar> = b[270..]
        .iter()
        .enumerate()
        .map(|(k, x)| FlowBar {
            time: x.time,
            pas: step,
            rangees: row(x, k),
            complete: k != 5,
        })
        .collect();
    reel.push(FlowBar {
        time: b[299].time + H1,
        pas: step,
        rangees: row(&b[299], 0),
        complete: true,
    });
    let incomplete: Vec<FlowBar> = reel[..30].iter().cloned().map(|r| FlowBar { complete: false, ..r }).collect();
    assert!(text(&read_with(
        "Bubbles",
        &b,
        &[],
        &ScreenData {
            flow: Some(&incomplete),
            ..Default::default()
        }
    ))
    .contains(absent));
    let x = flow::bubbles(&b, &vals, &reel[..30], &|v| format::price(v));
    let r = read_with(
        "Bubbles",
        &b,
        &[],
        &ScreenData {
            flow: Some(&reel),
            ..Default::default()
        },
    );
    let t = text(&r);
    assert!(t.contains("Vrais trades depuis la bougie d'il y a 29 bougies"), "{t}");
    let rec: Vec<_> = x.items.iter().filter(|q| q.i + 50 >= 300).collect();
    let buy = rec.iter().filter(|q| q.buy).count();
    assert!(!rec.is_empty() && t.contains(&format!(": {buy} bulle")), "{t}");
    let q = x.items.last().unwrap();
    assert!(
        t.contains(&format!(
            "Dernière bulle : côté {} à {}",
            if q.buy { "acheteurs" } else { "vendeurs" },
            format::price(q.price)
        )),
        "the bar in progress is left out: {t}"
    );
    let (_, poc) = *x.pocs.last().unwrap();
    assert!(t.contains(&format!("à {}.", format::price(poc))), "{t}");
    assert!(!t.contains("estim"), "no estimate any more: {t}");
    let off = read_with(
        "Bubbles",
        &b,
        &[("poc", 0.0)],
        &ScreenData {
            flow: Some(&reel),
            ..Default::default()
        },
    );
    assert!(!text(&off).contains("POC de la dernière"));
}

/// The rows named are the script's own (trend, RSI, change); the time left only with the screen's time.
#[test]
fn mtf_table_rows_are_the_scripts() {
    use argos_script::indicators::mtf_table;
    let vals = argos_script::inputs::defaults(&mtf_table::PARAMS);
    let b = walk(600);
    for n in (100..=b.len()).step_by(41) {
        let end = b[n - 1].time + H1;
        let rows = mtf_table::compute(&b[..n], H1, end, &vals);
        let t = text(&read("MtfTable", &b[..n], &[]));
        for r in &rows {
            match r.cell {
                Some(c) => {
                    let trend = match c.trend {
                        1 => "haussière",
                        -1 => "baissière",
                        _ => "neutre",
                    };
                    assert!(t.contains(&format!("{} : tendance {trend}, RSI {}", r.name, format::num(c.rsi, 1))), "{t}");
                }
                None => assert!(t.contains(&format!("{} : « — »", r.name)), "{t}"),
            }
        }
        assert!(!t.contains("clôture dans"), "no time left without the screen's time: {t}");
        let data = ScreenData {
            now: Some(end - 600),
            ..Default::default()
        };
        assert!(text(&read_with("MtfTable", &b[..n], &[], &data)).contains("clôture dans"));
    }
    let none = read("MtfTable", &b, &[("m15", 0.0), ("h1", 0.0), ("h4", 0.0), ("d1", 0.0), ("w1", 0.0)]);
    assert!(text(&none).contains("Aucune unité de temps cochée"));
}

/// « Lire tous les indicateurs » with the screen's data: `data[k]` goes with `items[k]`; a shorter slice reads the
/// rest with no data, a longer one is cut; never a panic.
#[test]
fn read_together_with_any_length_of_data() {
    let b = walk(300);
    let items = [item("Rsi", "RSI 14"), item("Footprint", "Footprint")];
    let flow = ScreenData {
        flow: Some(&[]),
        ..Default::default()
    };
    // Footprint without data: « Donnée absente », counted as too few to read; with an empty orderflow: read
    let none = overview_text(&read_together_with(&items, &b, &[]));
    assert_eq!(none, overview_text(&read_together(&items, &b)));
    let shorter = overview_text(&read_together_with(&items, &b, &[ScreenData::default()]));
    assert_eq!(shorter, none, "the item without an entry has no data");
    let same = overview_text(&read_together_with(&items, &b, &[ScreenData::default(), flow]));
    let longer = overview_text(&read_together_with(&items, &b, &[ScreenData::default(), flow, flow, flow]));
    assert_eq!(longer, same, "the extra entries are ignored");
    assert_ne!(same, none, "the entry of Footprint is used");
}

/// The CVD: estimated without the screen's orderflow (said), the same with an empty one, and the real delta of the
/// closed bars only (the bar in progress the screen also holds is left out).
#[test]
fn cvd_uses_the_screens_orderflow_of_closed_bars() {
    use argos_core::orderflow::FlowBar;
    use argos_script::indicators::flow;
    let b = walk(300);
    let none = text(&read("Cvd", &b, &[]));
    assert!(none.contains("l'orderflow réel n'est pas transmis"), "{none}");
    let empty = read_with(
        "Cvd",
        &b,
        &[],
        &ScreenData {
            flow: Some(&[]),
            ..Default::default()
        },
    );
    assert!(text(&empty).contains("pas d'orderflow réel pour ces bougies"));
    // the last 20 closed bars with real (strongly selling) trades, plus the bar in progress
    let mut reel: Vec<FlowBar> = b[280..]
        .iter()
        .map(|x| FlowBar {
            time: x.time,
            pas: 1.0,
            rangees: vec![(100, 1.0, x.volume * 3.0)],
            complete: true,
        })
        .collect();
    reel.push(FlowBar {
        time: b[299].time + H1,
        pas: 1.0,
        rangees: vec![(100, 1e9, 0.0)],
        complete: true,
    });
    let r = read_with(
        "Cvd",
        &b,
        &[],
        &ScreenData {
            flow: Some(&reel),
            ..Default::default()
        },
    );
    let t = text(&r);
    assert!(t.contains("Vrai delta (trades complets) sur 20 bougies des 300 chargées"), "{t}");
    let c = flow::cvd(&b, &reel[..20]);
    let down = c[299] < c[289];
    assert!(t.contains(if down { "CVD en baisse" } else { "CVD en hausse" }), "{t}");
    assert_eq!(
        r.sense.map(|x| x.lean),
        Some(if down { Lean::Down } else { Lean::Up }),
        "the bar in progress is left out"
    );
}

/// The AI Trend's multi-timeframe dots: the same reading without the smaller units' bars; with them, the last dot is
/// the script's own; a slot without its bars says so.
#[test]
fn ai_trend_reads_the_dots_with_the_smaller_units() {
    let h1 = waves(24 * 60);
    let d1 = argos_core::aggregate(&h1, 86_400);
    let st = [("m1_on", 1.0), ("m1_tf", 3.0)];
    let lower: ai_trend::LowerBars = [(3_600, h1.clone())].into_iter().collect();
    let with = ScreenData {
        lower: Some(&lower),
        bar_secs: Some(86_400),
        ..Default::default()
    };
    let a = ai_trend::compute(&d1, 86_400, &ai_trend::Settings::from_inputs(&ai_vals(&st)), &lower);
    let t = text(&read_with("AiTrend", &d1, &st, &with));
    let n = d1.len();
    match a.mtf.iter().filter(|m| n - 1 - m.index <= 50).max_by_key(|m| m.index) {
        Some(m) => assert!(
            t.contains(&format!(
                "Dernier point multi-UT : slot 1 (1h), divergence {}",
                if m.bull { "haussière" } else { "baissière" }
            )),
            "{t}"
        ),
        None => assert!(t.contains("Aucun point multi-UT"), "{t}"),
    }
    assert_eq!(
        read_with("AiTrend", &d1, &st, &ScreenData::default()),
        read("AiTrend", &d1, &st),
        "without the bars: unchanged"
    );
    let empty = ai_trend::LowerBars::new();
    let missing = text(&read_with(
        "AiTrend",
        &d1,
        &st,
        &ScreenData {
            lower: Some(&empty),
            ..Default::default()
        },
    ));
    assert!(missing.contains("bougies de cette unité absentes"), "{missing}");
}

/// The Anchored VWAP on another indicator's output: aligned, the VWAP of that series; not aligned, the price, said; not
/// passed, the sentence as before.
#[test]
fn anchored_vwap_on_an_external_source() {
    use argos_script::indicators::anchored_vwap;
    let b = walk(300);
    let src = argos_script::inputs::source_ext(3, 0);
    let ext: Vec<f64> = b.iter().map(|x| x.close * 1.01).collect();
    let mut vals = argos_script::inputs::defaults(&anchored_vwap::PARAMS);
    vals[1] = src;
    let a = anchored_vwap::compute(&b, &vals, &|_, _| Some(ext.clone()));
    let t = text(&read_with(
        "AnchoredVwap",
        &b,
        &[("src", src)],
        &ScreenData {
            ext_source: Some(&ext),
            ..Default::default()
        },
    ));
    assert!(
        t.contains(&format!("à {} : la clôture", format::price(a.vwap[299]))) && t.contains("la sortie d'un autre indicateur"),
        "{t}"
    );
    let long: Vec<f64> = ext.iter().copied().chain([1.0]).collect();
    let t = text(&read_with(
        "AnchoredVwap",
        &b,
        &[("src", src)],
        &ScreenData {
            ext_source: Some(&long),
            ..Default::default()
        },
    ));
    assert!(t.contains("n'a pas pu être alignée"), "{t}");
    assert!(text(&read("AnchoredVwap", &b, &[("src", src)])).contains("ne la lit pas encore"));
}

// ---- Lot C part 2: the live states (Tape, order-book walls, liquidation map) ----

/// The tape: its delta « en ce moment », its last line and its last mark, from the screen's own state; the bar in
/// progress is never called closed.
#[test]
fn tape_reads_the_screens_state() {
    use argos_script::indicators::flow::Tape;
    let vals = argos_script::inputs::defaults(&argos_script::v1_inputs::TAPE_PARAMS);
    let all = waves(200);
    let mut t = Tape::default();
    // the screen updates its tape on every bar (and the bar in progress), as it runs
    for k in 2..=all.len() {
        let mut part = all[..k].to_vec();
        let last = part.len() - 1;
        part[last].volume *= 0.5;
        t.update(&part, H1, &vals, all[k - 1].time + 60);
        t.update(&all[..k], H1, &vals, all[k - 1].time + 120);
    }
    let closed = &all[..all.len() - 1];
    let r = read_with(
        "Tape",
        closed,
        &[],
        &ScreenData {
            tape: Some(&t),
            ..Default::default()
        },
    );
    let x = text(&r);
    assert!(
        x.contains(&format!(
            "En ce moment, delta de volume de la bougie en cours : {}{}",
            if t.d > 0.0 { "+" } else { "" },
            format::num(t.d, 2)
        )),
        "{x}"
    );
    if let Some(l) = t.lines.first() {
        assert!(x.contains(&format!("la plus récente à {}", format::price(l.px))), "{x}");
    }
    if let Some(m) = t.marks(&vals).iter().max_by_key(|m| m.time) {
        if m.time > closed.last().unwrap().time {
            assert!(x.contains("sur la bougie en cours"), "{x}");
        }
    }
    assert!(!text(&read_with(
        "Tape",
        closed,
        &[("showTable", 0.0)],
        &ScreenData {
            tape: Some(&t),
            ..Default::default()
        }
    ))
    .contains("Ruban :"));
    assert!(text(&read("Tape", closed, &[])).contains("Donnée absente"));
}

/// The walls and the liquidation map: the biggest on each side, as passed; « SELL » / « BUY » named as labels; the
/// map said to be Argos's estimate.
#[test]
fn walls_and_liquidation_map_read_what_the_screen_passes() {
    let b = walk(100);
    let c = b[99].close;
    let walls = [
        Wall {
            price: c * 1.01,
            usd: 2_500_000.0,
            sell: true,
        },
        Wall {
            price: c * 1.02,
            usd: 900_000.0,
            sell: true,
        },
        Wall {
            price: c * 0.99,
            usd: 1_200_000.0,
            sell: false,
        },
    ];
    let t = text(&read_with(
        "BookHeat",
        &b,
        &[],
        &ScreenData {
            walls: Some(&walls),
            ..Default::default()
        },
    ));
    assert!(t.contains(&format!("étiquette « SELL » : 2,5 M $ à {}", format::price(c * 1.01))), "{t}");
    assert!(t.contains(&format!("étiquette « BUY » : 1,2 M $ à {}", format::price(c * 0.99))), "{t}");
    let none = text(&read_with(
        "BookHeat",
        &b,
        &[],
        &ScreenData {
            walls: Some(&[]),
            ..Default::default()
        },
    ));
    assert!(
        none.contains("aucun mur au-dessus du prix") && none.contains("aucun mur en dessous du prix"),
        "{none}"
    );
    let profile = [(c * 0.97, 5e6, 0.0), (c * 0.98, 2e6, 1e5), (c * 1.03, 0.0, 7e6), (c * 1.05, 0.0, 1e6)];
    let notes = ["OKX : historique en construction".to_string()];
    let t = text(&read_with(
        "LiqMap",
        &b,
        &[],
        &ScreenData {
            liq: Some(LiqData {
                profile: &profile,
                notes: &notes,
            }),
            ..Default::default()
        },
    ));
    assert!(
        t.contains(&format!("amas de shorts au-dessus à {}", format::price(c * 1.03))) && t.contains("7,0 M $"),
        "{t}"
    );
    assert!(
        t.contains(&format!("amas de longs en dessous à {}", format::price(c * 0.97))) && t.contains("5,0 M $"),
        "{t}"
    );
    assert!(t.contains("estimation d'Argos") && t.contains("Message de la carte : OKX"), "{t}");
    for w in ["achète", "acheter", "achat", "vends", "vendre", "vente", "✨"] {
        assert!(!t.contains(w), "{w}: {t}");
    }
}

/// The footprint: « Donnée absente » without the screen's orderflow; with an empty one, the chart's estimate of the
/// last closed bar; with real trades, their delta and POC; the bar in progress the screen also holds is left out.
#[test]
fn footprint_reads_the_last_closed_bar() {
    use argos_core::orderflow::FlowBar;
    use argos_script::indicators::flow;
    let b = walk(200);
    let vals = argos_script::inputs::defaults(&flow::FOOTPRINT_PARAMS);
    assert!(text(&read("Footprint", &b, &[])).contains("Donnée absente"));
    let est = flow::footprint(&b, &vals, &[]);
    let x = est.bars.last().unwrap();
    let t = text(&read_with(
        "Footprint",
        &b,
        &[],
        &ScreenData {
            flow: Some(&[]),
            ..Default::default()
        },
    ));
    assert!(t.contains("estimée d'après la bougie"), "{t}");
    assert!(
        t.contains(&format!(
            "Delta de la dernière bougie : {}{}",
            if x.d > 0.0 { "+" } else { "" },
            format::num(x.d, 2)
        )),
        "{t}"
    );
    assert!(
        t.contains(&format!("POC de la dernière bougie à {}", format::price((x.poc as f64 + 0.5) * est.step))),
        "{t}"
    );
    // real trades on the last closed bar (sellers), and a huge buying bar in progress
    let last = b[199];
    let reel = [
        FlowBar {
            time: last.time,
            pas: est.step,
            rangees: vec![(((last.low / est.step).floor()) as i64, 1.0, 9.0)],
            complete: true,
        },
        FlowBar {
            time: last.time + H1,
            pas: est.step,
            rangees: vec![(0, 1e9, 0.0)],
            complete: true,
        },
    ];
    let r = read_with(
        "Footprint",
        &b,
        &[],
        &ScreenData {
            flow: Some(&reel),
            ..Default::default()
        },
    );
    let t = text(&r);
    let real = flow::footprint(&b, &vals, &reel[..1]);
    let y = real.bars.last().unwrap();
    assert!(
        t.contains("vrais trades complets") && t.contains(&format!("Delta de la dernière bougie : {}", format::num(y.d, 2))),
        "{t}"
    );
    assert_eq!(r.sense.map(|s| s.lean), Some(Lean::Down), "the bar in progress is left out");
    let off = text(&read_with(
        "Footprint",
        &b,
        &[("poc", 0.0), ("dl", 0.0)],
        &ScreenData {
            flow: Some(&[]),
            ..Default::default()
        },
    ));
    assert!(!off.contains("POC de la dernière") && !off.contains("Delta de la dernière"), "{off}");
}

// ---- 📋 Summary tables (« Tableau résumé ») of Heatmap, Profile, Bands and Bands Osc ----

/// The sentence of the summary table, when the reading has one.
fn table_text(r: &Reading) -> Option<String> {
    r.blocks.iter().flat_map(|bl| &bl.sentences).find(|x| x.id == "table").map(|x| x.text.clone())
}

/// The rows the chart draws for `id` on `b` with the default settings, from the shared functions (the Bands'
/// « Squeeze » cell as the reading writes it: Oui or Non).
fn drawn_rows(id: &str, b: &[Bar]) -> Vec<(&'static str, String)> {
    use argos_script::inputs::defaults;
    use argos_script::v1_inputs::{BANDS_OSC_PARAMS, BANDS_PARAMS, HEATMAP_PARAMS, PROFILE_PARAMS};
    match id {
        "Heatmap" => classic::heatmap_tableau(&classic::heatmap_volume(b, &defaults(&HEATMAP_PARAMS))),
        "Profile" => classic::profile_tableau(&classic::volume_profile(b, &defaults(&PROFILE_PARAMS)).unwrap(), b, &format::price),
        "Bands" => {
            let a = classic::argos_bands(b, &defaults(&BANDS_PARAMS));
            let sq = if a.squeeze[b.len() - 1] { "Oui" } else { "Non" };
            classic::bands_tableau(&a, b)
                .into_iter()
                .map(|(k, v)| (k, if k == "Squeeze" { sq.to_string() } else { v }))
                .collect()
        }
        "BandsOsc" => {
            let (u, d) = classic::bollinger_oscillator(b, &defaults(&BANDS_OSC_PARAMS));
            classic::bands_osc_tableau(&u, &d)
        }
        _ => unreachable!(),
    }
}

/// Box ticked: one sentence with every label and every value of the drawn table, in order.
#[test]
fn table_ticked_says_the_drawn_rows() {
    for id in ["Heatmap", "Profile", "Bands", "BandsOsc"] {
        for b in [walk(600), waves(400)] {
            for n in (150..=b.len()).step_by(37) {
                let t = table_text(&read(id, &b[..n], &[("showTable", 1.0)])).unwrap_or_else(|| panic!("{id}: no table"));
                let rows = drawn_rows(id, &b[..n]);
                assert!(!rows.is_empty(), "{id}");
                let cells: Vec<String> = rows.iter().map(|(k, v)| format!("{k} : {v}")).collect();
                assert_eq!(t, format!("Tableau résumé à l'écran : {}.", cells.join(" ; ")), "{id}");
                for (k, v) in &rows {
                    assert!(t.contains(k) && t.contains(v.as_str()), "{id}: {k} {v} in {t}");
                }
            }
        }
    }
}

/// Box unticked: no sentence about the table.
#[test]
fn table_unticked_says_nothing() {
    for id in ["Heatmap", "Profile", "Bands", "BandsOsc"] {
        let r = read(id, &walk(600), &[("showTable", 0.0)]);
        assert!(table_text(&r).is_none() && !text(&r).contains("Tableau résumé"), "{id}");
    }
}

/// The catalogue's default: the table of Heatmap, Profile and Bands is shown, Bands Osc's is not.
#[test]
fn table_by_default() {
    for (id, shown) in [("Heatmap", true), ("Profile", true), ("Bands", true), ("BandsOsc", false)] {
        assert_eq!(table_text(&read(id, &walk(600), &[])).is_some(), shown, "{id}");
    }
}

/// The Bands' « Squeeze » cell says only Oui or Non, from the last bar's squeeze; never a forecast.
#[test]
fn table_bands_squeeze_yes_and_no() {
    let vals = argos_script::inputs::defaults(&argos_script::v1_inputs::BANDS_PARAMS);
    let (mut yes, mut no) = (0, 0);
    for b in [walk(600), waves(400)] {
        for n in (150..=b.len()).step_by(7) {
            let a = classic::argos_bands(&b[..n], &vals);
            let t = table_text(&read("Bands", &b[..n], &[])).unwrap();
            assert!(!t.to_lowercase().contains("explosion") && !t.contains("venir"), "{t}");
            if a.squeeze[n - 1] {
                yes += 1;
                assert!(t.contains("Squeeze : Oui ;"), "{t}");
            } else {
                no += 1;
                assert!(t.contains("Squeeze : Non ;"), "{t}");
            }
        }
    }
    assert!(yes > 0 && no > 0, "{yes} / {no}");
}

/// The table's box is read: it is in keys_read, with its role.
#[test]
fn table_box_in_keys_read() {
    for id in ["Heatmap", "Profile", "Bands", "BandsOsc"] {
        assert!(keys_read(id).contains(&"showTable"), "{id}");
        let r = read(id, &walk(600), &[("showTable", 1.0)]);
        assert!(r.settings.iter().any(|x| x.key == "showTable" && x.role.is_some()), "{id}");
    }
}

/// 🌐 Argos AI writes its numbers in each of the screen's formats exactly as the chart does (`argos_core::nombres`),
/// with a true minus sign and never « −0,00 ».
#[test]
fn numbers_follow_each_screen_format() {
    use argos_core::nombres::{format_number_in, format_price_in, NUMBER_FORMATS};
    let expected = ["64\u{202f}812,50", "64,812.50", "64.812,50", "64812,50", "64,812.500"];
    assert_eq!(expected.len(), NUMBER_FORMATS.len(), "every format is tested");
    for (fmt, want) in expected.iter().enumerate() {
        assert_eq!(format::price_in(64812.5, fmt), *want, "format {fmt}");
        assert_eq!(format::price_in(64812.5, fmt), format_price_in(64812.5, 2, fmt), "format {fmt}");
        assert_eq!(format::num_in(1234.56, 1, fmt), format_number_in(1234.56, 1, fmt), "format {fmt}");
    }
    assert_eq!(format::num_in(-80.04, 1, 1), "−80.0");
    assert_eq!(format::num_in(-0.001, 2, 0), "0,00");
    assert_eq!(format::price_in(f64::NAN, 0), "?");
}

/// 🌐 Without a choice in 🌐, Argos AI writes like the chart's default format (« 64 812,50 »).
#[test]
fn default_format_is_the_charts() {
    assert_eq!(format::price(64812.5), "64\u{202f}812,50");
    assert_eq!(format::percent(-1.1), "−1,10 %");
}
