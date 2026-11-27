//! Conformité : les indicateurs classiques Rust donnent les mêmes valeurs que les formules du site (Argos v1,
//! `app.js` : sma, ema, rma, rsi, stdev, highest / lowest, vwap du jour), sur les 504 bougies HYPE 50 min.
//! Valeurs du site calculées avec Node à partir des fonctions copiées telles quelles du site.

use argos_core::Bar;
use argos_script::indicators::classic::*;

/// (bougie, [RSI 14, MACD, signal, BB haute, BB basse, ATR 14, %K, %D, VWAP, volume relatif, EMA 9, EMA 200]).
/// The EMA 200 (of EMA multiples) is TradingView's `ta.ema` since B-097, started on the simple mean of the first 200
/// closes; the old site started it on the first close (80.98649129519774, 79.33390948930554, 78.12129508239525 at these
/// bars). Values redone apart, in Python, from TradingView's definition (lot tv-classiques-2). The EMA 9 (Croisement
/// EMA, B-096) has long forgotten its start at these bars: the same value either way.
const SITE: [(usize, [f64; 12]); 3] = [
    (
        300,
        [
            31.26484800842742,
            -0.3588493466432112,
            -0.19650221958431613,
            79.7192078120254,
            77.28609218797463,
            0.5671914432005292,
            8.16013992889103,
            13.096256220963804,
            78.18579566451632,
            1.8754153333401489,
            77.9506084650826,
            81.12501511224723,
        ],
    ),
    (
        454,
        [
            41.19121171534347,
            -0.23699103885060424,
            -0.13317010064394944,
            78.81257755635214,
            76.87842244364789,
            0.6294058978854685,
            28.129213398438186,
            30.928424371175876,
            77.73813459822011,
            1.4799381487840098,
            77.43410976936254,
            79.36360599666511,
        ],
    ),
    (
        503,
        [
            57.27917150498716,
            0.10734500521776624,
            -0.012659711256092675,
            77.2189099800621,
            75.40829001993795,
            0.5078600530105186,
            84.82760768552485,
            82.20976631950442,
            76.39205523957544,
            1.0623630845911127,
            76.49846002439102,
            78.13948787233032,
        ],
    ),
];

#[test]
fn classiques_identiques_au_site() {
    let raw: serde_json::Value = serde_json::from_str(include_str!("fixtures/hype_hl_50m.json")).unwrap();
    let bars: Vec<Bar> = raw
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
    let (r, m, bb, at, (k, d), vw, rv, x) = (
        rsi14(&bars),
        macd(&bars),
        bollinger(&bars),
        atr14(&bars),
        stochastic(&bars),
        vwap(&bars),
        relative_volume(&bars),
        ema_cross(&bars),
    );
    let e200 = &ema_multiples(&bars, &argos_script::inputs::defaults(&argos_script::v1_inputs::EMA_MULTI_PARAMS))[3];
    let names = [
        "RSI 14",
        "MACD",
        "signal",
        "BB haute",
        "BB basse",
        "ATR 14",
        "%K",
        "%D",
        "VWAP",
        "volume relatif",
        "EMA 9",
        "EMA 200",
    ];
    for (i, site) in SITE {
        let rust = [
            r[i],
            m.line[i],
            m.signal[i],
            bb.upper[i],
            bb.lower[i],
            at[i],
            k[i],
            d[i],
            vw[i],
            rv[i],
            x.fast[i],
            e200[i],
        ];
        for (j, (a, b)) in rust.iter().zip(site).enumerate() {
            let tol = 1e-6 * b.abs().max(1.0);
            assert!((a - b).abs() <= tol, "{} bougie {i} : Rust {a} / site {b}", names[j]);
        }
    }
    // Argos Profil de volume (écrit de zéro) : tout le volume des 200 dernières bougies est réparti, POC dans la zone
    // de valeur, rangées de la plage
    let p = volume_profile(&bars, &argos_script::inputs::defaults(&argos_script::v1_inputs::PROFILE_PARAMS)).unwrap();
    let vol: f64 = bars[bars.len() - 200..].iter().map(|b| b.volume).sum();
    let tot: f64 = p.up.iter().zip(&p.down).map(|(u, d)| u + d).sum();
    assert!((tot - vol).abs() < 1e-6 * vol);
    assert!(p.va_lo <= p.poc && p.poc <= p.va_hi && p.up.len() == 30);
    let lo = bars[bars.len() - 200..].iter().map(|b| b.low).fold(f64::INFINITY, f64::min);
    assert!((p.bot - lo).abs() < 1e-12);
}
