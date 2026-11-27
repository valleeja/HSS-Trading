//! 📖 The four scripts of the part « Recettes » of the Argos Script reference (documents/2026-11-22-argos-script-
//! reference-v1.md), kept as fixtures in `pine/argos/recettes/`: each compiles and runs on 5 000 bars and draws
//! something. And a timing test (`#[ignore]`): compile and run times of four scripts. Pure: synthetic bars.

use super::*;
use argos_core::Bar;

const RECETTES: [(&str, &str); 4] = [
    ("Supertrend", include_str!("../../../../../pine/argos/recettes/01-supertrend.pine")),
    ("Veille", include_str!("../../../../../pine/argos/recettes/02-veille.pine")),
    ("RSI multi-UT", include_str!("../../../../../pine/argos/recettes/03-rsi-multi-ut.pine")),
    ("Gros volume", include_str!("../../../../../pine/argos/recettes/04-gros-volume.pine")),
];

/// `n` bars of `tf` seconds: a deterministic random walk from 60 000 (seeded), volumes 1..100.
fn barres(n: usize, tf: i64) -> Vec<Bar> {
    let mut x: u64 = 0x2545_F491_4F6C_DD1D;
    let mut alea = move || {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        (x >> 11) as f64 / (1u64 << 53) as f64
    };
    let t0 = 1_700_000_000 - 1_700_000_000 % tf;
    let mut p = 60_000.0;
    (0..n)
        .map(|k| {
            let o = p;
            let c = o * (1.0 + (alea() - 0.5) * 0.01);
            let (h, l) = (o.max(c) * (1.0 + alea() * 0.003), o.min(c) * (1.0 - alea() * 0.003));
            p = c;
            Bar {
                time: t0 + tf * k as i64,
                open: o,
                high: h,
                low: l,
                close: c,
                volume: 1.0 + (alea() * 99.0).round(),
            }
        })
        .collect()
}

/// Something drawn: a plot, a shape, a drawing, a table or a coloured bar.
fn dessine(o: &Output) -> bool {
    !o.plots.is_empty()
        || !o.shapes.is_empty()
        || o.drawings.iter().any(|d| !d.deleted)
        || o.tables.iter().any(|t| !t.deleted)
        || o.barcolor.iter().any(Option::is_some)
}

/// Each recipe of the reference, pasted as it is, compiles and runs on 5 000 one-hour bars without error and draws.
#[test]
fn recettes_de_la_reference() {
    let bars = barres(5_000, 3_600);
    for (nom, src) in RECETTES {
        let o = run(src, &bars, 3_600, &Default::default()).unwrap_or_else(|e| panic!("{nom}: {e}"));
        assert!(dessine(&o), "{nom}: nothing drawn");
    }
}

/// F2 (22 Nov): Argos Vector Zone Pro, imported as it is, compiles and draws on 5 000 bars of 5 and 15 minutes (its
/// `[17 names] = request.security(…, f_struct())` gives na before the first hour closes, as in Pine). Above 15 min
/// its « TF 1 » (15 min) asks a lower timeframe of `request.security`: the known gap of `tests/pine_de_jee.rs`.
#[test]
fn vector_zone_pro_compile_et_dessine() {
    let src = include_str!("../../../../../pine/argos-vector-zone-pro.pine");
    for tf in [300, 900] {
        let bars = barres(5_000, tf);
        let o = run(src, &bars, tf, &Default::default()).unwrap_or_else(|e| panic!("{tf} s: {e}"));
        assert!(dessine(&o), "{tf} s: nothing drawn");
    }
}

/// ⏱ Compile time (translation, lexer, parser) and run time of four scripts on 5 000 bars, median of 7 runs, printed
/// (`cargo test --release -p argos-script -- --ignored --nocapture chrono`).
#[test]
#[ignore]
fn chrono_des_scripts() {
    use std::time::{Duration, Instant};
    let scripts: [(&str, &str); 4] = [
        (
            "Argos Vague TradingView v5 (Pine)",
            include_str!("../../../../../pine/argos/argos-vague-tradingview-v5.pine"),
        ),
        ("Argos Bollinger + RSI", crate::integres::BOLLINGER_RSI),
        (
            "Argos Vector Zone Pro (766 lines)",
            include_str!("../../../../../pine/argos-vector-zone-pro.pine"),
        ),
        ("Argos AI Trend Pro (723 lines)", include_str!("../../../../../pine/argos-ai-trend-pro.pine")),
    ];
    let bars = barres(5_000, 3_600);
    let mediane = |mut v: Vec<Duration>| {
        v.sort();
        v[v.len() / 2]
    };
    for (nom, src) in scripts {
        let (mut c, mut r) = (vec![], vec![]);
        let mut erreur = None;
        for _ in 0..7 {
            let t = Instant::now();
            let conv = dialect::to_pine(src);
            let script = lexer::lex(&conv.pine).and_then(parser::parse);
            c.push(t.elapsed());
            let Ok(script) = script else {
                erreur = script.err().map(|e| e.msg);
                break;
            };
            let t = Instant::now();
            let reglages = HashMap::new();
            let mut it = interp::Interp::new(&script, &bars, 3_600, &reglages);
            regle_et_ou(&mut it, &conv.dialect);
            let res = it.run();
            r.push(t.elapsed());
            if let Err(e) = res {
                erreur = Some(e.msg);
                break;
            }
        }
        match erreur {
            Some(e) => println!("chrono {nom}: error {e}"),
            None => println!("chrono {nom}: compile {:?}, run {:?} (5 000 bars, median of 7)", mediane(c), mediane(r)),
        }
    }
}
