//! Tests de l'adaptateur : manifeste du kit, refus (réseau, IA, clé), vérification du `.wasm`, conversions en
//! entiers, vecteurs de conformité du kit et un vrai appel du module « Moyenne » dans la bulle.

use super::*;

const MOYENNE_WASM: &[u8] = include_bytes!("../../../../app/modules/moyenne.wasm");
const MOYENNE_JSON: &str = include_str!("../../../../app/modules/moyenne.json");
const VECTORS: &str = include_str!("../../../vendor/kit-module-argos-1.0.2/argos-module-sdk/tests/vectors/abi-v2.json");

fn hex(s: &str) -> Vec<u8> {
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).expect("hex")).collect()
}

#[test]
fn manifeste_du_kit_et_refus() {
    let m = Manifest::parse(MOYENNE_JSON).expect("manifeste d'exemple");
    assert_eq!((m.path.as_str(), m.abi, m.name()), ("trading/moyenne", 2, "moyenne"));
    m.check().expect("exemple valide");
    assert!(m.rights_text().is_empty(), "aucun droit");
    assert_eq!(m.default_settings(), vec![20, 0], "longueur 20, source = clôture");
    let with = |extra: &str| MOYENNE_JSON.replacen('{', &format!("{{ {extra},"), 1);
    let refuse = |extra: &str| Manifest::parse(&with(extra)).and_then(|m| m.check());
    assert!(Manifest::parse(&with("\"cles\": \"sk-1\"")).is_err(), "champ inconnu (clé) : refusé");
    let net = MOYENNE_JSON.replace(
        "\"capabilities\": []",
        "\"capabilities\": [\"Network\"], \"domains\": [\"api.hyperliquid.xyz\"]",
    );
    assert!(Manifest::parse(&net).unwrap().check().unwrap_err().contains("réseau"));
    let ai = MOYENNE_JSON.replace("\"capabilities\": []", "\"capabilities\": [\"Ai\"], \"ai_model\": \"m@1\"");
    assert!(Manifest::parse(&ai).unwrap().check().unwrap_err().contains("IA"));
    assert!(refuse("\"budget_secs\": 0").is_err(), "budget de temps hors limites");
    let v1 = MOYENNE_JSON.replace("\"abi\": 2", "\"abi\": 1");
    assert!(Manifest::parse(&v1).unwrap().check().unwrap_err().contains("v2"));
    let other = MOYENNE_JSON.replace("trading/moyenne", "hss/moyenne");
    assert!(Manifest::parse(&other).unwrap().check().unwrap_err().contains("trading/"));
    let files = MOYENNE_JSON.replace("\"capabilities\": []", "\"capabilities\": [\"ReadFiles\", \"WriteFiles\"]");
    let f = Manifest::parse(&files).unwrap();
    f.check().expect("dossier privé permis");
    assert_eq!(f.rights_text().len(), 2);
}

#[test]
fn verification_du_wasm() {
    assert_eq!(scan(MOYENNE_WASM), Ok(()));
    assert!(scan(b"\0asm\x01\0\0\0").unwrap_err().contains("absent"), "module vide : exports v2 absents");
    assert!(scan(b"pas du wasm").is_err());
}

#[test]
fn bougies_en_entiers_a_l_echelle() {
    let bars: Vec<RawBar> = vec![
        (1, 100.5, 101.25, 99.75, 100.0, 12.5),
        (2, 100.0, 102.0, 99.0, 101.125, 3.0),
        (2, 1.0, 1.0, 1.0, 1.0, 1.0),
    ];
    let i = input_of("BTC/USD", 3600, &bars, vec![20, 0]);
    assert_eq!(
        (i.price_scale, i.volume_scale, i.bars.len()),
        (3, 1, 2),
        "assez de décimales, heure répétée retirée"
    );
    assert_eq!((i.bars[0].h, i.bars[1].c, i.bars[0].v), (101_250, 101_125, 125));
    let back = Input::decode(&i.encode()).expect("format du kit");
    assert_eq!(back, i, "aller-retour au format binaire du kit");
    assert!(real(EMPTY, 2).is_nan() && real(12_345, 2) == 123.45);
    let long: String = "X".repeat(40);
    assert_eq!(input_of(&long, 60, &[], vec![]).symbol.len(), 32, "symbole coupé à 32 octets");
}

#[test]
fn vecteurs_de_conformite_du_kit() {
    let v: serde_json::Value = serde_json::from_str(VECTORS).unwrap();
    for c in v["valid_inputs"].as_array().unwrap() {
        let b = hex(c["hex"].as_str().unwrap());
        let i = Input::decode(&b).unwrap_or_else(|e| panic!("{} : {e:?}", c["name"]));
        assert_eq!(i.encode(), b, "{} : octet pour octet", c["name"]);
    }
    for c in v["invalid_inputs"].as_array().unwrap() {
        assert!(Input::decode(&hex(c["hex"].as_str().unwrap())).is_err(), "{} : doit être refusé", c["name"]);
    }
    for c in v["valid_outputs"].as_array().unwrap() {
        let b = hex(c["hex"].as_str().unwrap());
        let n = c["bars"].as_u64().unwrap() as usize;
        let o = Output::decode(&b, n).unwrap_or_else(|e| panic!("{} : {e:?}", c["name"]));
        assert_eq!(o.encode(), b, "{} : octet pour octet", c["name"]);
    }
    for c in v["invalid_outputs"].as_array().unwrap() {
        let n = c["bars"].as_u64().unwrap() as usize;
        assert!(Output::decode(&hex(c["hex"].as_str().unwrap()), n).is_err(), "{} : doit être refusé", c["name"]);
    }
}

/// 🖥️ Un vrai appel dans la bulle du kit : la moyenne des 4 bougies des vecteurs, identique à la sortie attendue.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn moyenne_dans_la_bulle_du_kit() {
    let v: serde_json::Value = serde_json::from_str(VECTORS).unwrap();
    let m = Manifest::parse(MOYENNE_JSON).unwrap();
    let input = Input::decode(&hex(v["moyenne"]["input_hex"].as_str().unwrap())).unwrap();
    let want = Output::decode(&hex(v["moyenne"]["output_hex"].as_str().unwrap()), input.bars.len()).unwrap();
    let e = engine::Engine::new().unwrap();
    let dir = std::env::temp_dir().join(format!("argos-modules-tests-{}", std::process::id()));
    let ok = sha384(MOYENNE_WASM);
    let got = e.run(&m, MOYENNE_WASM, &ok, &[], &dir, &[], &input).expect("appel");
    assert_eq!(got, want);
    let lines = lines_of(&got);
    assert_eq!(lines[0].1.len(), 4);
    assert!(lines[0].1[0].is_nan(), "EMPTY → vide");
    assert!(!cards_of(&got).is_empty());
    // un autre fichier que celui accepté : refusé ; un réglage hors limites : refusé par la bulle
    assert!(e.run(&m, MOYENNE_WASM, &[0; 48], &[], &dir, &[], &input).unwrap_err().contains("changé"));
    let bad = Input {
        settings: vec![9999, 0],
        ..input
    };
    assert!(e.run(&m, MOYENNE_WASM, &ok, &[], &dir, &[], &bad).is_err());
}

/// 🪟 Étape 7 : 200 000 bougies → les 50 000 plus récentes, sous 4 Mio une fois encodées, lisibles par le kit.
#[test]
fn fenetre_de_200_000_bougies() {
    let bars: Vec<RawBar> = (0..200_000i64)
        .map(|k| {
            let p = 60_000.0 + (k % 977) as f64 * 1.25;
            (1_700_000_000 + k * 60, p, p + 5.5, p - 4.25, p + 1.5, 12.5)
        })
        .collect();
    let f = fenetre("BTC/USD", 60, &bars, vec![20, 0]);
    assert_eq!(f.input.bars.len(), MAX_BARS, "les 50 000 plus récentes");
    assert_eq!(f.laissees, 150_000);
    assert_eq!(f.input.bars.last().unwrap().t, bars.last().unwrap().0, "jusqu'à la dernière bougie");
    assert_eq!(f.octets, f.input.encode().len(), "taille réelle mesurée");
    assert!(f.octets <= MAX_BYTES, "{} octets ≤ 4 Mio", f.octets);
    println!(
        "fenêtre de 50 000 bougies : {} octets ({:.2} Mio sur 4)",
        f.octets,
        f.octets as f64 / 1_048_576.0
    );
    assert_eq!(Input::decode(&f.input.encode()).expect("le kit l'accepte"), f.input);
    // peu de bougies : tout part, rien n'est laissé
    let f = fenetre("BTC/USD", 60, &bars[..1000], vec![20, 0]);
    assert_eq!((f.input.bars.len(), f.laissees), (1000, 0));
}

/// 🪟 Si un jour l'entrée dépassait la taille permise (plus de champs, autre format…), les plus anciennes bougies
/// sont retirées juste assez, sans planter : essayé avec une limite de 10 000 octets.
#[test]
fn fenetre_coupee_a_la_taille() {
    let bars: Vec<RawBar> = (0..5000i64).map(|k| (1_700_000_000 + k * 60, 10.0, 11.0, 9.0, 10.5, 1.0)).collect();
    let f = fenetre_avec("ETH", 60, &bars, vec![], MAX_BARS, 10_000);
    assert!(f.octets <= 10_000 && f.octets > 10_000 - 48, "au plus près de la limite : {}", f.octets);
    assert_eq!(f.laissees + f.input.bars.len(), 5000);
    assert_eq!(f.input.bars.last().unwrap().t, bars.last().unwrap().0, "les plus récentes gardées");
    assert!(Input::decode(&f.input.encode()).is_ok());
    // limite plus petite que l'en-tête : aucune bougie, pas de plantage
    let f = fenetre_avec("ETH", 60, &bars, vec![], MAX_BARS, 10);
    assert!(f.input.bars.is_empty() && f.laissees == 5000);
}
