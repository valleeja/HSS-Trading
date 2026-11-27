//! 📜 Step 4 closes with this test: Jee's Pine scripts (the `pine/` folder of the repository) run as they are, on test
//! bars, without an error.

use argos_core::Bar;
use std::collections::HashMap;

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

/// Every `.pine` file of `pine/` runs on 50-minute test bars without an error (TBO, Cipher, TVD, AI Trend, Vector
/// Zone and the others there), except the known gaps listed below, which are being filled one by one.
#[test]
fn les_pine_de_jee_passent_sans_erreur() {
    let dossier = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../pine");
    let mut fichiers: Vec<_> = std::fs::read_dir(&dossier)
        .expect("the pine/ folder")
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "pine"))
        .collect();
    fichiers.sort();
    assert!(fichiers.iter().any(|p| p.to_string_lossy().contains("TBO")) && fichiers.iter().any(|p| p.to_string_lossy().contains("Cipher")));
    // Known gaps of the language, each with its exact line and message: when one is filled, its entry goes (in the
    // same commit, with a parity test); a new error, or a changed one, makes the test fail.
    // (editeur-pine-1: `request.security_lower_tf` is there now — ob_fvg_xau.pine and argos-ai-trend-pro.pine
    // pass; its parity tests are in `lang/tests_intrabar.rs`)
    // (tradingview-1, B-037: `request.security` on a lower timeframe is there now — argos-vector-zone-pro.pine passes;
    // its test is `lang/tests_references.rs`)
    let manques: [(&str, u32, &str); 0] = [];
    let b = bars();
    let mut erreurs = vec![];
    for f in &fichiers {
        let nom = f.file_name().unwrap().to_string_lossy().into_owned();
        let src = std::fs::read_to_string(f).unwrap();
        let attendu = manques.iter().find(|m| m.0 == nom);
        match (argos_script::lang::run(&src, &b, 3000, &HashMap::new()), attendu) {
            (Ok(_), None) => {}
            (Err(e), Some((_, ligne, msg))) if e.line == *ligne && e.msg == *msg => {}
            (Ok(_), Some(_)) => erreurs.push(format!("{nom} passes now: remove it from the known gaps")),
            (Err(e), _) => erreurs.push(format!("{nom} line {} : {}", e.line, e.msg)),
        }
    }
    assert!(erreurs.is_empty(), "{}", erreurs.join("\n"));
}
