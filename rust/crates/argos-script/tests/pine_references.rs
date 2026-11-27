//! 📜 Point 3 (« indicateurs = TradingView », tradingview-1): the TradingView community scripts Jee pasted, kept in
//! `pine/references/` with their licence, run as they are and draw what they draw on TradingView. The v6 versions
//! Argos made of LuxAlgo's v5 scripts give the very same drawings as the v5 code they come from.

use argos_core::Bar;
use argos_script::lang::{run, Output};
use std::collections::HashMap;
use std::path::PathBuf;

fn bars() -> Vec<Bar> {
    let v: serde_json::Value = serde_json::from_str(include_str!("fixtures/hype_hl_50m.json")).unwrap();
    v.as_array()
        .unwrap()
        .iter()
        .map(|b| Bar {
            time: b["time"].as_i64().unwrap(),
            open: b["open"].as_f64().unwrap(),
            high: b["high"].as_f64().unwrap(),
            low: b["low"].as_f64().unwrap(),
            close: b["close"].as_f64().unwrap(),
            volume: b["volume"].as_f64().unwrap(),
        })
        .collect()
}

fn dossier() -> PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../pine/references")
}

fn source(nom: &str) -> String {
    std::fs::read_to_string(dossier().join(nom)).unwrap_or_else(|e| panic!("{nom}: {e}"))
}

fn go(nom: &str) -> Output {
    run(&source(nom), &bars(), 3000, &HashMap::new()).unwrap_or_else(|e| panic!("{nom} line {} : {}", e.line, e.msg))
}

fn vivants<'a>(o: &'a Output, kind: &str) -> Vec<&'a argos_script::lang::Drawing> {
    o.drawings.iter().filter(|d| d.kind == kind && !d.deleted).collect()
}

fn textes(o: &Output) -> Vec<&str> {
    vivants(o, "label").iter().map(|d| d.text.as_str()).collect()
}

/// The same drawings, field for field (a label has no second point: NaN there on both sides).
fn same_drawings(v5: &Output, v6: &Output) {
    assert_eq!(v5.drawings.len(), v6.drawings.len(), "v6 = v5");
    for (k, (a, b)) in v5.drawings.iter().zip(&v6.drawings).enumerate() {
        assert_eq!(format!("{a:?}"), format!("{b:?}"), "drawing {k}: v6 = v5");
    }
}

const FICHIERS: [&str; 7] = [
    "luxalgo-liquidity-swings-v5.pine",
    "luxalgo-liquidity-swings-v6.pine",
    "luxalgo-smart-money-concepts-v5.pine",
    "luxalgo-smart-money-concepts-v6.pine",
    "viprasol-fib-ote-fvg.pine",
    "zeiierman-smart-swing-vwap-mtf.pine",
    "zeiierman-smart-swing-vwap.pine",
];

/// Every reference has its check below and opens with its licence: Creative Commons BY-NC-SA 4.0 (LuxAlgo,
/// Zeiierman) or the Mozilla Public License 2.0 (Viprasol). GPL code never comes in (the repository's licences).
#[test]
fn chaque_reference_a_sa_licence() {
    let mut vus: Vec<String> = std::fs::read_dir(dossier())
        .expect("pine/references")
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".pine"))
        .collect();
    vus.sort();
    assert_eq!(vus, FICHIERS, "a new reference gets its check in this file");
    for nom in FICHIERS {
        let src = source(nom);
        let tete: String = src.lines().take(2).collect::<Vec<_>>().join(" ");
        assert!(
            tete.contains("(CC BY-NC-SA 4.0) https://creativecommons.org/licenses/by-nc-sa/4.0/") || tete.contains("Mozilla Public License 2.0"),
            "{nom}: its licence on the first lines"
        );
        let bas = src.to_lowercase();
        assert!(!bas.contains("gpl") && !bas.contains("gnu general public"), "{nom}: no GPL code");
    }
}

/// LuxAlgo « Smart Money Concepts », Argos Script's SMC reference: its structure labels (BOS, CHoCH, equal highs /
/// lows, strong / weak high and low) and its order blocks; the v6 version draws exactly what the v5 code draws.
#[test]
fn luxalgo_smart_money_concepts() {
    let (v5, v6) = (go("luxalgo-smart-money-concepts-v5.pine"), go("luxalgo-smart-money-concepts-v6.pine"));
    assert_eq!(v5.title, "Smart Money Concepts [LuxAlgo]");
    let t = textes(&v5);
    for mot in ["BOS", "CHoCH", "Strong High", "Weak Low"] {
        assert!(t.contains(&mot), "« {mot} » among {t:?}");
    }
    assert!(!vivants(&v5, "box").is_empty(), "order blocks");
    assert!(vivants(&v5, "line").len() >= 10, "structure lines");
    // (a BOS / CHoCH label has no background: its colour is `na`, B-039)
    assert!(vivants(&v5, "label").iter().filter(|d| d.text == "BOS").all(|d| (d.color >> 24) & 0x7f == 100));
    same_drawings(&v5, &v6);
}

/// LuxAlgo « Liquidity Swings »: its swing zones (boxes), their levels (lines) and the volume of each zone written
/// the TradingView way (`format.volume`: « 119.073K »); the v6 version draws exactly what the v5 code draws.
#[test]
fn luxalgo_liquidity_swings() {
    let (v5, v6) = (go("luxalgo-liquidity-swings-v5.pine"), go("luxalgo-liquidity-swings-v6.pine"));
    assert_eq!(v5.title, "Liquidity Swings [LuxAlgo]");
    assert!(vivants(&v5, "box").len() >= 5 && vivants(&v5, "line").len() >= 5);
    let t = textes(&v5);
    assert!(t.len() >= 5);
    for s in &t {
        let chiffres = s.trim_end_matches(['K', 'M', 'B', 'T']);
        assert!(chiffres.parse::<f64>().is_ok(), "volume « {s} »");
    }
    assert!(t.iter().any(|s| s.ends_with('K')));
    same_drawings(&v5, &v6);
}

/// Viprasol « Fib OTE + FVG Confluence »: its OTE zone named with its levels, its Fibonacci lines and its gaps.
#[test]
fn viprasol_fib_ote_fvg() {
    let o = go("viprasol-fib-ote-fvg.pine");
    let t = textes(&o);
    assert!(t.iter().any(|s| s.contains("OTE 0.618-0.786")), "{t:?}");
    assert!(vivants(&o, "line").len() >= 3 && !vivants(&o, "box").is_empty());
}

/// `n` one-hour bars: a deterministic random walk from 100 (the series of the Contrôle's measurement, 25 Nov 2026).
fn serie(n: usize) -> Vec<Bar> {
    let mut x: u64 = 12345;
    let mut p = 100.0f64;
    (0..n)
        .map(|i| {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            let r = (x % 1000) as f64 / 1000.0 - 0.5;
            let o = p;
            p *= 1.0 + r * 0.02;
            Bar {
                time: 1_600_000_000 + i as i64 * 3600,
                open: o,
                high: o.max(p) * 1.003,
                low: o.min(p) * 0.997,
                close: p,
                volume: 1000.0 + (x % 500) as f64,
            }
        })
        .collect()
}

/// B-032 (found by the Contrôle before the merge, 25 Nov): Smart Swing VWAP deletes its curve and draws it again
/// with all its points at every bar (« Recolor Entire Active Line », its default). A deleted polyline kept its
/// points: 935 000 points kept for 3 000 bars (about 14 Mo), growing with the square of the curve's length. It keeps
/// none, as TradingView frees a deleted polyline; the live curve keeps its own. (Since B-064 the deleted ones leave
/// the list too, once they are more than 1 024: the few still in it at the end are the ones checked here.)
#[test]
fn smart_swing_vwap_ses_polylines_effacees_ne_gardent_aucun_point() {
    let nom = "zeiierman-smart-swing-vwap.pine";
    let o = run(&source(nom), &serie(3000), 3600, &HashMap::new()).unwrap_or_else(|e| panic!("{nom} line {} : {}", e.line, e.msg));
    let (vivantes, effacees): (Vec<_>, Vec<_>) = o.polylines.iter().partition(|p| !p.deleted);
    assert!(
        (1..=1024).contains(&effacees.len()),
        "the curve drawn again at each bar: {} deleted polylines still in the list",
        effacees.len()
    );
    assert_eq!(
        effacees.iter().map(|p| p.points.len()).sum::<usize>(),
        0,
        "points kept by the deleted polylines"
    );
    assert!(
        !vivantes.is_empty() && vivantes.iter().all(|p| p.points.len() >= 2),
        "the live curves keep their points"
    );
}

/// Zeiierman « Smart Swing VWAP »: its anchored VWAPs drawn as polylines from the swings it ranks (HH, HL, LH, LL);
/// the version Jee pasted with an MTF module adds the 4H / D / W VWAPs, written with their price.
#[test]
fn zeiierman_smart_swing_vwap() {
    for nom in ["zeiierman-smart-swing-vwap.pine", "zeiierman-smart-swing-vwap-mtf.pine"] {
        let o = go(nom);
        let courbes: Vec<_> = o.polylines.iter().filter(|p| !p.deleted).collect();
        assert!(!courbes.is_empty() && courbes.iter().all(|p| p.points.len() >= 2), "{nom}: VWAP curves");
        let t = textes(&o);
        for s in ["HH", "HL", "LH", "LL"] {
            assert!(t.iter().any(|x| x.starts_with(s)), "{nom}: « {s} » among {t:?}");
        }
        if nom.ends_with("-mtf.pine") {
            for s in ["4H VWAP", "D VWAP", "W VWAP"] {
                assert!(t.iter().any(|x| x.starts_with(s)), "{nom}: « {s} » among {t:?}");
            }
            assert!(
                o.plots.iter().filter(|p| p.values.iter().any(|v| v.is_finite())).count() >= 3,
                "{nom}: MTF VWAP lines"
            );
        }
    }
}
