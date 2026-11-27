//! 🎯 Valeurs de référence des indicateurs classiques sur des bougies fixes (40 bougies d'une heure, une sinusoïde qui
//! monte). Les valeurs attendues ont été recalculées à part (script Python indépendant, mêmes définitions que Pine :
//! SMA, EMA démarrée par la SMA, moyenne de Wilder, RSI et ATR de Wilder, MACD avec l'EMA de Pine (B-093), VWAP du jour) :
//! un changement de formule, même petit, fait échouer ce test. Les 12 indicateurs des manuels (`textbook`) aussi.

use argos_core::Bar;
use argos_script::indicators::classic;
use argos_script::ta;

fn bars() -> Vec<Bar> {
    let c: Vec<f64> = (0..40).map(|i| 100.0 + 10.0 * (i as f64 * 0.5).sin() + i as f64 * 0.3).collect();
    (0..40)
        .map(|i| {
            let open = if i == 0 { c[0] - 0.5 } else { c[i - 1] };
            Bar {
                time: 1_700_000_000 + i as i64 * 3600,
                open,
                high: open.max(c[i]) + 1.0 + 0.1 * (i % 3) as f64,
                low: open.min(c[i]) - 1.0 - 0.1 * (i % 2) as f64,
                close: c[i],
                volume: 1000.0 + (i % 7) as f64 * 100.0,
            }
        })
        .collect()
}

fn near(got: f64, want: f64, what: &str) {
    assert!((got - want).abs() < 1e-9, "{what} : {got} au lieu de {want}");
}

#[test]
fn moyennes_rsi_atr() {
    let b = bars();
    let c: Vec<f64> = b.iter().map(|x| x.close).collect();
    let sma = ta::sma(&c, 20);
    assert!(sma[18].is_nan(), "pas de SMA 20 avant 20 bougies");
    near(sma[39], 107.264696093500, "SMA 20");
    let ema = ta::ema(&c, 10);
    near(ema[9], 103.232165248343, "EMA 10 (départ = SMA)");
    near(ema[39], 109.256494386752, "EMA 10");
    let rsi = classic::rsi14(&b);
    assert!(rsi[13].is_nan());
    near(rsi[14], 61.538226964957, "RSI 14 (1re valeur)");
    near(rsi[39], 62.188789452052, "RSI 14");
    near(classic::atr14(&b)[39], 5.440455868122, "ATR 14");
}

#[test]
fn macd_bollinger_vwap() {
    let b = bars();
    let m = classic::macd(&b);
    // comme TradingView (B-093) : ligne vide avant la 26e bougie, signal avant la 34e
    assert!(m.line[24].is_nan() && m.line[25].is_finite());
    assert!(m.signal[32].is_nan() && m.signal[33].is_finite() && m.hist[32].is_nan());
    near(m.line[39], 1.2922213197856252, "MACD ligne");
    near(m.signal[39], 0.8290231173630838, "MACD signal");
    near(m.hist[39], 1.2922213197856252 - 0.8290231173630838, "MACD histogramme");
    near(classic::bollinger(&b).upper[39], 121.798471675539, "Bollinger haute");
    let vw = classic::vwap(&b);
    near(vw[1], 101.699581245919, "VWAP (avant minuit UTC)");
    near(vw[39], 109.933996276514, "VWAP (remis à zéro chaque jour)");
}

/// Même série, 80 bougies (Ichimoku a besoin de 52 + 26 bougies).
fn bars80() -> Vec<Bar> {
    let c: Vec<f64> = (0..80).map(|i| 100.0 + 10.0 * (i as f64 * 0.5).sin() + i as f64 * 0.3).collect();
    (0..80)
        .map(|i| {
            let open = if i == 0 { c[0] - 0.5 } else { c[i - 1] };
            Bar {
                time: 1_700_000_000 + i as i64 * 3600,
                open,
                high: open.max(c[i]) + 1.0 + 0.1 * (i % 3) as f64,
                low: open.min(c[i]) - 1.0 - 0.1 * (i % 2) as f64,
                close: c[i],
                volume: 1000.0 + (i % 7) as f64 * 100.0,
            }
        })
        .collect()
}

#[test]
fn indicateurs_des_manuels() {
    use argos_script::indicators::textbook as t;
    let b = bars80();
    let (k, d) = t::stoch_rsi(&b);
    near(k[79], 95.03176876554755, "RSI stochastique %K");
    near(d[79], 83.33443180393552, "RSI stochastique %D");
    let i = t::ichimoku(&b);
    near(i.tenkan[79], 122.5593330112593, "Ichimoku tenkan");
    near(i.kijun[79], 120.77806915801057, "Ichimoku kijun");
    // décalage 26 = 25 bougies, comme TradingView (B-029) ; la bougie 78 sépare 25 de 26 (112,789 / 108,075 avec 26)
    near(i.span_a[79], 114.0730894158722, "Ichimoku senkou A (décalé de 25)");
    near(i.span_b[79], 109.3591376271941, "Ichimoku senkou B (décalé de 25)");
    near(i.span_a[78], 114.0730894158722, "Ichimoku senkou A à la bougie 78 (décalé de 25)");
    near(i.span_b[78], 109.3591376271941, "Ichimoku senkou B à la bougie 78 (décalé de 25)");
    near(i.chikou[50], 120.52201200363538, "Ichimoku chikou (clôture 25 bougies plus tard)");
    let (ku, km, kd) = t::keltner(&b);
    near(ku[79], 132.54616950834503, "Keltner haut");
    near(km[79], 122.08751470047949, "Keltner milieu");
    near(kd[79], 111.62885989261395, "Keltner bas");
    let (dh, _, dl) = t::donchian(&b);
    near(dh[79], 134.53645455694976, "Donchian haut");
    near(dl[79], 107.01968375907138, "Donchian bas");
    near(t::psar(&b)[79], 113.0691513537368, "SAR parabolique");
    let (p, m, adx) = t::dmi(&b);
    near(p[79], 37.91283797475684, "+DI");
    near(m[79], 19.542959136672852, "−DI");
    near(adx[79], 17.83444996300386, "ADX");
    near(t::cci(&b)[79], 121.8285313288401, "CCI");
    near(t::williams_r(&b)[79], -4.592088323574667, "Williams %R");
    near(t::obv(&b)[79], 9000.0, "OBV");
    let (st, up) = t::supertrend(&b);
    near(st[79], 117.549221998097, "Supertrend");
    assert!(up[79], "Supertrend haussier");
    let (au, ad) = t::aroon(&b);
    near(au[79], 100.0, "Aroon haut");
    near(ad[79], 28.0, "Aroon bas");
    near(t::mfi(&b)[79], 57.608280262761006, "MFI");
}
