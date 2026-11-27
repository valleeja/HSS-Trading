//! Backtest de conformité : le TBO Rust doit donner le même signalDelta, les mêmes breaks (filtrés / validés / Vector),
//! les mêmes achats / ventes, les mêmes divergences et le même tableau de statut que le TBO du site (sauf le dernier
//! événement, maintenant choisi comme le Pine TradingView).
//! Fixture générée par le code JavaScript du site, sur les 3000 bougies 30m de l'AI Trend, avec 4 réglages différents.
//! La ligne Signal (moyennes exponentielles) démarre un peu autrement sur le site : comparée 200 bougies après le premier signalDelta.

use argos_core::Bar;
use argos_script::indicators::tbo::{compute, BreakKind, Htf, HtfUsed, LastEvent, Preset, Settings, SlopeCalc, Smoothing, VecMode};
use serde_json::Value;

const WARMUP: usize = 200;

fn bars() -> Vec<Bar> {
    let raw: Value = serde_json::from_str(include_str!("fixtures/ai_trend_js.json")).unwrap();
    let f = |b: &Value, k: &str| b[k].as_f64().unwrap();
    raw["bars"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| Bar {
            time: b["time"].as_i64().unwrap(),
            open: f(b, "open"),
            high: f(b, "high"),
            low: f(b, "low"),
            close: f(b, "close"),
            volume: f(b, "volume"),
        })
        .collect()
}

fn ints(v: &Value) -> Vec<usize> {
    v.as_array().unwrap().iter().map(|x| x.as_u64().unwrap() as usize).collect()
}

fn settings(name: &str) -> Settings {
    // le site d'origine démarrait en Personnalisé ; le défaut Argos est maintenant Day Trade comme le Pine
    let d = Settings {
        preset: Preset::Custom,
        ..Settings::default()
    };
    match name {
        "default" => d,
        "stdev_tema" => Settings {
            calc: SlopeCalc::Stdev,
            smooth: Smoothing::Tema,
            vec_mode: VecMode::V150,
            htf: Htf::Chart,
            ..d
        },
        "linreg_vwma" => Settings {
            calc: SlopeCalc::Linreg,
            smooth: Smoothing::Vwma,
            vec_mode: VecMode::Off,
            htf_on: false,
            div_hidden: false,
            ..d
        },
        "auto" => Settings { preset: Preset::Auto, ..d },
        _ => unreachable!(),
    }
}

fn status(js: &Value, key: &str) -> String {
    js["dash"].as_array().unwrap().iter().find(|r| r[0] == key).unwrap()[1]
        .as_str()
        .unwrap()
        .to_string()
}

#[test]
fn tbo_identique_au_site() {
    let bars = bars();
    let all: Value = serde_json::from_str(include_str!("fixtures/tbo_js.json")).unwrap();
    for name in ["default", "stdev_tema", "linreg_vwma", "auto"] {
        let js = &all[name];
        let r = compute(&bars, 1800, &settings(name));

        for (i, v) in r.core.delta.iter().enumerate() {
            match js["delta"][i].as_f64() {
                Some(j) => assert!((v - j).abs() < 1e-6, "[{name}] signalDelta différent à la bougie {i} : Rust {v} / site {j}"),
                None => assert!(v.is_nan(), "[{name}] le site n'a pas de signalDelta à la bougie {i}"),
            }
        }
        // la ligne Signal se stabilise 200 bougies après le premier signalDelta (plus tard en preset, la trendline attend ses pivots)
        let start = r.core.delta.iter().position(|v| v.is_finite()).unwrap() + WARMUP;
        for (i, v) in r.signal.iter().enumerate().skip(start) {
            let j = js["sig"][i].as_f64().unwrap_or(f64::NAN);
            assert!(
                (v - j).abs() < 1e-6 || (v.is_nan() && j.is_nan()),
                "[{name}] Signal différent à la bougie {i} : Rust {v} / site {j}"
            );
        }

        let ours: Vec<(usize, bool, u8)> = r
            .breaks
            .iter()
            .map(|b| {
                let k = match b.kind {
                    BreakKind::Filtered => 0,
                    BreakKind::Valid => 1,
                    BreakKind::Vector => 2,
                };
                (b.index, b.up, k)
            })
            .collect();
        let site: Vec<(usize, bool, u8)> = js["dots"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| (x[0].as_u64().unwrap() as usize, x[1] == 1, x[2].as_u64().unwrap() as u8))
            .collect();
        assert_eq!(ours, site, "[{name}] points de break");
        assert_eq!(r.buy, ints(&js["buy"]), "[{name}] achats");
        assert_eq!(r.sell, ints(&js["sell"]), "[{name}] ventes");

        let ours: Vec<(usize, usize, bool, bool)> = r.divs.iter().map(|d| (d.prev, d.pivot, d.bull, d.hidden)).collect();
        let site: Vec<(usize, usize, bool, bool)> = js["divs"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| (x[0].as_u64().unwrap() as usize, x[1].as_u64().unwrap() as usize, x[2] == 1, x[3] == 1))
            .collect();
        assert_eq!(ours, site, "[{name}] divergences");

        let vec_site: Vec<u8> = js["vector"].as_array().unwrap().iter().map(|x| x.as_u64().unwrap() as u8).collect();
        assert_eq!(r.vector, vec_site, "[{name}] bougies Vector");

        // tableau de statut
        assert_eq!(format!("{:.1}", r.adx.last().unwrap()), status(js, "ADX"), "[{name}] ADX");
        assert_eq!(r.active.length.to_string(), status(js, "Swing"), "[{name}] Swing");
        let htf = match r.htf {
            HtfUsed::Off => "off".to_string(),
            HtfUsed::Chart => "Graphique".to_string(),
            HtfUsed::Seconds(14_400) => "4h".to_string(),
            HtfUsed::Seconds(s) => format!("{s}s"),
        };
        assert_eq!(htf, status(js, "HTF"), "[{name}] HTF");
        let arrow = |up: bool| if up { "▲" } else { "▼" };
        let last = match r.last {
            Some(LastEvent::Break { up, vector }) => format!("{}{}", if vector { "BO + VEC200 " } else { "BREAK " }, arrow(up)),
            Some(LastEvent::Divergence { bull, hidden }) => format!("{} {}", if hidden { "HID" } else { "REG" }, arrow(bull)),
            None => "—".to_string(),
        };
        // dernier événement comme le Pine (`lastEvt`) : la bougie la plus récente parmi les breaks validés du site et ses
        // divergences (confirmées à pivot + longueur) ; dans une même bougie : breaks ▲ ▼, REG ▲ ▼, HID ▲ ▼. Le site
        // donnait toujours la dernière divergence, même avec un break plus récent : c'était faux.
        let mut events: Vec<(usize, u8, String)> = vec![];
        for x in js["dots"].as_array().unwrap() {
            let (i, up, k) = (x[0].as_u64().unwrap() as usize, x[1] == 1, x[2].as_u64().unwrap());
            if k > 0 {
                let txt = format!("{}{}", if k == 2 { "BO + VEC200 " } else { "BREAK " }, arrow(up));
                events.push((i, u8::from(!up), txt));
            }
        }
        for x in js["divs"].as_array().unwrap() {
            let (pivot, bull, hidden) = (x[1].as_u64().unwrap() as usize, x[2] == 1, x[3] == 1);
            let rank = 2 + u8::from(!bull) + 2 * u8::from(hidden);
            let txt = format!("{} {}", if hidden { "HID" } else { "REG" }, arrow(bull));
            events.push((pivot + r.active.div_len, rank, txt));
        }
        let want = events.into_iter().max_by_key(|e| (e.0, e.1)).map_or("—".to_string(), |e| e.2);
        assert_eq!(last, want, "[{name}] dernier événement");
        // 🔔 alertes du Pine : break validé = achat / vente, une divergence par signal, Vector 200+ dans le sens de la bougie
        let al = r.alerts(&bars);
        let ones = |k: usize| al[k].1.iter().filter(|v| **v == 1.0).count();
        assert_eq!((al.len(), ones(0), ones(1)), (10, r.buy.len(), r.sell.len()), "[{name}] alertes de break");
        assert_eq!((6..10).map(ones).sum::<usize>(), r.divs.len(), "[{name}] alertes de divergence");
        assert_eq!(ones(4) + ones(5), r.vector.iter().filter(|v| **v == 2).count(), "[{name}] alertes Vector 200+");
    }
}
