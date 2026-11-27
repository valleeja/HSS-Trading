//! Backtest de conformité : les bougies de volume Rust doivent donner exactement les mêmes paliers et couleurs
//! que la version JavaScript du site (fixture générée à partir de `src/07-fiche-du-symbole.js`, 400 bougies).
//! Le palier LOW est propre à Argos (le site ne l'a pas) : une bougie LOW doit être « normal » sur le site, avec la même couleur.
//! Les couleurs baissières GOLD / HIGH d'Argos ont changé (mauve foncé / rouge) : on compare avec les couleurs du site pour vérifier la logique.

use argos_core::Bar;
use argos_script::indicators::volume_candles::{compute, Settings, Tier};

#[test]
fn bougies_de_volume_identiques_au_site() {
    let raw: serde_json::Value = serde_json::from_str(include_str!("fixtures/volume_candles_js.json")).unwrap();
    let bars: Vec<Bar> = raw["bars"]
        .as_array()
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
        .collect();
    let site_colors = Settings {
        bear_gold: "#e1b4f5".into(),
        bear_high: "#673ab7".into(),
        ..Settings::default()
    };
    let out = compute(&bars, &site_colors);
    let name = |t: Tier| match t {
        Tier::Gold => "gold",
        Tier::High => "high",
        Tier::Mid => "mid",
        Tier::Normal | Tier::Low => "normal",
    };
    for (i, v) in out.iter().enumerate() {
        assert_eq!(name(v.tier), raw["tiers"][i].as_str().unwrap(), "palier différent à la bougie {i}");
        assert_eq!(v.color, raw["colors"][i].as_str().unwrap(), "couleur différente à la bougie {i}");
    }
    assert!(out.iter().filter(|v| v.tier == Tier::Gold).count() > 5, "la fixture doit contenir des GOLD");
}
