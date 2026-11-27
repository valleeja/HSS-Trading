//! Backtest de conformité : le SMC Rust (OB · BB · FVG · iFVG) doit donner exactement les mêmes zones, cassures BOS / CHoCH,
//! EQH / EQL et sweeps que l'indicateur du site. Fixture générée par le code JavaScript du site, sur les 3000 bougies 30m
//! de l'AI Trend, avec 3 réglages : défaut, réglage alternatif (corps, mèche, sans BB / iFVG…) et UT supérieure 4h.

use argos_core::Bar;
use argos_script::indicators::smc::{compute, BreakType, Settings};
use serde_json::Value;

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

fn settings(name: &str) -> Settings {
    let d = Settings::default();
    match name {
        "default" => d,
        "alt" => Settings {
            len: 3,
            ob_body: true,
            ob_wick: true,
            breakers: false,
            ifvg: false,
            max_touch: 0,
            fvg_min: 0.0,
            eq_thr: 0.3,
            ..d
        },
        "htf4h" => Settings { tf: Some(14_400), ..d },
        _ => unreachable!(),
    }
}

fn u(v: &Value) -> usize {
    v.as_u64().unwrap() as usize
}
fn end(v: &Value) -> Option<usize> {
    let x = v.as_i64().unwrap();
    (x >= 0).then_some(x as usize)
}

#[test]
fn smc_identique_au_site() {
    let bars = bars();
    let all: Value = serde_json::from_str(include_str!("fixtures/smc_js.json")).unwrap();
    for name in ["default", "alt", "htf4h"] {
        let js = &all[name];
        let r = compute(&bars, 1800, &settings(name));
        let n = r.htf_bars.as_ref().map_or(bars.len(), |h| h.len());
        assert_eq!(n, u(&js["n"]), "[{name}] nombre de bougies");
        assert_eq!(r.trend as i64, js["trend"].as_i64().unwrap(), "[{name}] structure");

        let ob: Vec<_> = r.order_blocks.iter().map(|o| (o.up, o.breaker, o.i0, o.top, o.bot, o.end, o.from)).collect();
        let ob_js: Vec<_> = js["ob"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| {
                (
                    x[0] == 1,
                    x[1] == 1,
                    u(&x[2]),
                    x[3].as_f64().unwrap(),
                    x[4].as_f64().unwrap(),
                    end(&x[5]),
                    u(&x[6]),
                )
            })
            .collect();
        assert_eq!(ob, ob_js, "[{name}] order blocks / breakers");

        let fvg: Vec<_> = r
            .fvgs
            .iter()
            .map(|f| (f.up, f.inverse, f.i0, f.top, f.bot, f.end, f.from, f.touches as usize, f.filled, f.worn))
            .collect();
        let fvg_js: Vec<_> = js["fvg"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| {
                (
                    x[0] == 1,
                    x[1] == 1,
                    u(&x[2]),
                    x[3].as_f64().unwrap(),
                    x[4].as_f64().unwrap(),
                    end(&x[5]),
                    u(&x[6]),
                    u(&x[7]),
                    x[8] == 1,
                    x[9] == 1,
                )
            })
            .collect();
        assert_eq!(fvg, fvg_js, "[{name}] FVG / iFVG");

        let lines: Vec<_> = r.breaks.iter().map(|l| (l.i0, l.i1, l.price, l.up, l.kind == BreakType::Choch)).collect();
        let lines_js: Vec<_> = js["lines"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| (u(&x[0]), u(&x[1]), x[2].as_f64().unwrap(), x[3] == 1, x[4] == 1))
            .collect();
        assert_eq!(lines, lines_js, "[{name}] BOS / CHoCH");

        let eq: Vec<_> = r.equal.iter().map(|e| (e.i0, e.i1, e.price, e.up, e.swept)).collect();
        let eq_js: Vec<_> = js["eq"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| (u(&x[0]), u(&x[1]), x[2].as_f64().unwrap(), x[3] == 1, end(&x[4])))
            .collect();
        assert_eq!(eq, eq_js, "[{name}] EQH / EQL + sweeps");
    }
}
