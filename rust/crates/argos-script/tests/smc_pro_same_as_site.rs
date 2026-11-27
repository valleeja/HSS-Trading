//! Backtest de conformité : l'Argos SMC Rust doit arriver au même état que l'Argos SMC du site, bougie pour bougie :
//! zones vivantes, BOS / CHoCH, EQH / EQL, sweeps, sessions, Premium / Discount, multi-UT, tendance de fond, POI, Fibo / OTE,
//! confluences, confirmation, Fibo 2, PDH / PWH, plan, score /10, ce qui manque, verdict et alertes.
//! Fixture générée en instrumentant le code JavaScript du site : 5 réglages × 6 longueurs d'historique (30 cas, 30m).

use argos_core::Bar;
use argos_script::indicators::smc_pro::{compute, ConfState, Fond, Leg, Missing, SessionTz, Settings, SmcPro, SweepSource, Verdict, Zone, ZoneKind};
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
            fond: Fond::Both,
            wait_conf: false,
            ob_use_body: true,
            ote886: false,
            session_tz: SessionTz::London,
            max_ob: 4,
            max_fvg: 6,
            ..d
        },
        "off" => Settings {
            fond: Fond::Off,
            swing_len: 3,
            session_tz: SessionTz::Utc,
            ..d
        },
        "ema" => Settings {
            fond: Fond::Ema,
            ema_len: 50,
            conf_lb: 1,
            ..d
        },
        "htf" => Settings {
            fond: Fond::Htf,
            fib1_len: 120,
            fib2_len: 300,
            sl_buf_atr: 0.5,
            ..d
        },
        _ => unreachable!(),
    }
}

/// Nombre du JSON (les non-finis sont écrits "nan" / "inf" / "-inf").
fn num(v: &Value) -> f64 {
    match v {
        Value::String(s) if s == "inf" => f64::INFINITY,
        Value::String(s) if s == "-inf" => f64::NEG_INFINITY,
        Value::String(_) | Value::Null => f64::NAN,
        _ => v.as_f64().unwrap(),
    }
}

fn same(case: &str, what: &str, ours: f64, js: &Value) {
    let j = num(js);
    let ok = (ours.is_nan() && j.is_nan()) || ours == j || (ours - j).abs() <= 1e-9 * (1.0 + j.abs());
    assert!(ok, "[{case}] {what} : Rust {ours} / site {j}");
}

fn idx(v: &Value) -> Option<usize> {
    v.as_i64().filter(|&x| x >= 0).map(|x| x as usize)
}

fn kind(nm: &str) -> ZoneKind {
    match nm.trim_end_matches(['+', '−']) {
        "OB" => ZoneKind::Ob,
        "FVG" => ZoneKind::Fvg,
        "BB" => ZoneKind::Bb,
        "IFVG" => ZoneKind::Ifvg,
        x => panic!("zone inconnue {x}"),
    }
}

fn zones(case: &str, what: &str, ours: &[Zone], js: &Value) {
    let js = js.as_array().unwrap();
    assert_eq!(ours.len(), js.len(), "[{case}] {what} : nombre de zones");
    for (z, j) in ours.iter().zip(js) {
        same(case, what, z.top, &j["top"]);
        same(case, what, z.bot, &j["bot"]);
        assert_eq!(Some(z.left), idx(&j["left"]), "[{case}] {what} : bougie de départ");
    }
}

fn leg(case: &str, what: &str, ours: Option<Leg>, js: &Value) {
    match ours {
        None => assert!(js.is_null(), "[{case}] {what} : le site en a une"),
        Some(g) => {
            same(case, what, g.lo, &js["lo"]);
            same(case, what, g.hi, &js["hi"]);
            assert_eq!((Some(g.i0), Some(g.i1)), (idx(&js["i0"]), idx(&js["i1"])), "[{case}] {what} : bougies");
        }
    }
}

fn missing_js(m: &str) -> Missing {
    match m {
        "Tendance de fond" => Missing::Fond,
        "Prix dans l'OTE" => Missing::Ote,
        "Confluence POI + OTE" => Missing::Confluence,
        "HTF aligné" => Missing::Htf,
        "Discount" | "Premium" => Missing::PremiumDiscount,
        "Sweep récent" => Missing::Sweep,
        "Prix dans un POI" => Missing::Poi,
        "CHoCH de confirmation" => Missing::Choch,
        "R:R ≥ 2" => Missing::Rr,
        "Killzone" => Missing::Killzone,
        x => panic!("manque inconnu {x}"),
    }
}

fn verdict_js(v: &str) -> Verdict {
    if v.starts_with("Tendance de fond pas claire") {
        Verdict::NoTrend
    } else if v.starts_with("Pas de biais") {
        Verdict::NoBias
    } else if v.contains("invalidé") {
        Verdict::Invalidated
    } else if v.starts_with("Prix dans la zone") {
        Verdict::InZoneWait
    } else if v.starts_with("Attendre que le prix") {
        Verdict::WaitOte
    } else if v.starts_with("Setup A+") {
        Verdict::APlus
    } else if v.starts_with("Setup valide") {
        Verdict::Valid
    } else if v.starts_with("Confirmé mais") {
        Verdict::WeakConfirmed
    } else if v.starts_with("Setup faible") {
        Verdict::Weak
    } else if v.starts_with("Pas de setup") {
        Verdict::NoSetup
    } else {
        panic!("verdict inconnu {v}")
    }
}

fn check(case: &str, r: &SmcPro, js: &Value, bars: &[Bar]) {
    let st = &js["st"];
    // historique : cassures, EQH / EQL, sweeps
    let ev: Vec<_> = r.events.iter().map(|e| (e.from, e.at, e.level, e.up, e.choch)).collect();
    let ev_js: Vec<_> = js["ev"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| (idx(&x[0]).unwrap(), idx(&x[1]).unwrap(), num(&x[2]), x[3] == 1, x[4] == 1))
        .collect();
    assert_eq!(ev, ev_js, "[{case}] BOS / CHoCH");
    let eq: Vec<_> = r.equal.iter().map(|e| (e.i0, e.i1, e.p0, e.p1, e.up)).collect();
    let eq_js: Vec<_> = js["eq"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| (idx(&x[0]).unwrap(), idx(&x[1]).unwrap(), num(&x[2]), num(&x[3]), x[4] == 1))
        .collect();
    assert_eq!(eq, eq_js, "[{case}] EQH / EQL");
    let src = |s: SweepSource| match s {
        SweepSource::Swing => 0,
        SweepSource::Asia => 1,
        SweepSource::London => 2,
        SweepSource::NewYork => 3,
    };
    let sw: Vec<_> = r.sweeps.iter().map(|s| (s.at, s.dir as i64, src(s.source))).collect();
    let sw_js: Vec<_> = js["sw"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| (idx(&x[0]).unwrap(), x[1].as_i64().unwrap(), x[2].as_i64().unwrap()))
        .collect();
    assert_eq!(sw, sw_js, "[{case}] sweeps");

    // zones vivantes
    let z = &st["Z"];
    zones(case, "OB+", &r.zones.bull_ob, &z["bullOb"]);
    zones(case, "OB−", &r.zones.bear_ob, &z["bearOb"]);
    zones(case, "FVG+", &r.zones.bull_fvg, &z["bullFvg"]);
    zones(case, "FVG−", &r.zones.bear_fvg, &z["bearFvg"]);
    zones(case, "BB+", &r.zones.bull_bb, &z["bullBb"]);
    zones(case, "BB−", &r.zones.bear_bb, &z["bearBb"]);
    zones(case, "IFVG+", &r.zones.bull_ifvg, &z["bullIf"]);
    zones(case, "IFVG−", &r.zones.bear_ifvg, &z["bearIf"]);

    // structure, sessions
    assert_eq!(r.trend as i64, st["trend"].as_i64().unwrap(), "[{case}] tendance");
    same(case, "swing haut", r.swing_high, &st["swH"]);
    same(case, "swing bas", r.swing_low, &st["swL"]);
    same(case, "niveau protégé haut", r.protected_high, &st["protH"]);
    same(case, "niveau protégé bas", r.protected_low, &st["protL"]);
    assert_eq!(
        (r.in_asia, r.in_london, r.in_new_york),
        (st["inAsia"] == true, st["inLon"] == true, st["inNy"] == true),
        "[{case}] session en cours"
    );
    for (k, s) in r.sessions.iter().enumerate() {
        let j = &st["SS"][k];
        let bx: Vec<_> = s.boxes.iter().map(|b| (b.i0, b.i1, b.hi, b.lo)).collect();
        let bx_js: Vec<_> = j["boxes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|b| (idx(&b["i0"]).unwrap(), idx(&b["i1"]).unwrap(), num(&b["hi"]), num(&b["lo"])))
            .collect();
        assert_eq!(bx, bx_js, "[{case}] session {k} : boîtes");
        assert_eq!(
            s.current.map(|b| (b.i0, b.i1, b.hi, b.lo)),
            (!j["cur"].is_null()).then(|| (
                idx(&j["cur"]["i0"]).unwrap(),
                idx(&j["cur"]["i1"]).unwrap(),
                num(&j["cur"]["hi"]),
                num(&j["cur"]["lo"])
            )),
            "[{case}] session {k} : en cours"
        );
        let lv: Vec<_> = s.levels.iter().map(|l| (l.i0, l.price, l.end)).collect();
        let lv_js: Vec<_> = j["liq"]
            .as_array()
            .unwrap()
            .iter()
            .map(|l| (idx(&l["i0"]).unwrap(), num(&l["p"]), idx(&l["end"])))
            .collect();
        assert_eq!(lv, lv_js, "[{case}] session {k} : liquidité");
        same(case, "range session haut", s.last_hi, &j["lastHi"]);
        same(case, "range session bas", s.last_lo, &j["lastLo"]);
    }

    // Premium / Discount, multi-UT, tendance de fond
    same(case, "range haut", r.range_top, &st["rTop"]);
    same(case, "range bas", r.range_bot, &st["rBot"]);
    same(case, "équilibre", r.equilibrium, &st["eqLvl"]);
    same(case, "Premium / Discount %", r.pd_pct, &st["pdPct"]);
    for (k, m) in r.mtf.iter().enumerate() {
        let j = &st["mt"][k];
        assert_eq!(
            (m.trend as i64, m.event as i64, m.same),
            (j["t"].as_i64().unwrap(), j["e"].as_i64().unwrap(), j["same"] == true),
            "[{case}] multi-UT {k}"
        );
        same(case, "multi-UT invalidation", m.invalidation, &j["inv"]);
    }
    same(case, "biais HTF", r.htf_score, &st["htfScore"]);
    same(case, "biais HTF total", r.htf_total, &st["htfTotal"]);
    same(case, "EMA", r.ema, &st["emaF"]);
    assert_eq!(
        (r.ema_dir as i64, r.htf_dir as i64, r.vote as i64, r.fond_dir as i64, r.dir as i64),
        (
            st["emaDir"].as_i64().unwrap(),
            st["htfDir"].as_i64().unwrap(),
            st["vote"].as_i64().unwrap(),
            st["fondDir"].as_i64().unwrap(),
            st["dir"].as_i64().unwrap()
        ),
        "[{case}] directions"
    );

    // POI, Fibo, confluences, confirmation
    match r.poi {
        None => assert!(st["poi"].is_null(), "[{case}] POI : le site en a un"),
        Some(p) => {
            assert_eq!(p.kind, kind(st["poi"]["nm"].as_str().unwrap()), "[{case}] POI");
            zones(case, "POI", &[p.zone], &Value::Array(vec![st["poi"]["z"].clone()]));
        }
    }
    leg(case, "Fibo 1", r.leg, &st["leg"]);
    same(case, "OTE haut", r.ote_top, &st["oteTop"]);
    same(case, "OTE bas", r.ote_bot, &st["oteBot"]);
    assert_eq!(r.in_ote, st["inOte"] == true, "[{case}] prix dans l'OTE");
    let cf = st["confl"].as_array().unwrap();
    assert_eq!(r.confluences.len(), cf.len(), "[{case}] confluences");
    for (c, j) in r.confluences.iter().zip(cf) {
        assert_eq!(
            (c.kind, Some(c.left)),
            (kind(j["nm"].as_str().unwrap()), idx(&j["left"])),
            "[{case}] confluence"
        );
        same(case, "confluence haut", c.top, &j["top"]);
        same(case, "confluence bas", c.bot, &j["bot"]);
    }
    assert_eq!(r.in_conf, st["inConf"] == true, "[{case}] prix dans la confluence");
    let c = &st["cf"];
    let state = match c["st"].as_str().unwrap() {
        "none" => ConfState::None,
        "zone" => ConfState::Zone,
        "ok" => ConfState::Ok,
        _ => ConfState::Invalid,
    };
    assert_eq!(
        (r.confirmation.state, r.confirmation.touch, r.confirmation.bar),
        (state, idx(&c["touch"]), idx(&c["bar"])),
        "[{case}] confirmation"
    );
    same(case, "extrême de confirmation", r.confirmation.ext, &c["ext"]);
    leg(case, "Fibo 2", r.leg2, &st["leg2"]);
    same(case, "OTE 2 haut", r.ote2_top, &st["ote2Top"]);
    same(case, "OTE 2 bas", r.ote2_bot, &st["ote2Bot"]);
    // PWH / PWL : semaines du lundi 00:00 UTC (le site les faisait partir du jeudi, l'époque Unix) : la semaine d'avant
    // est recalculée ici à partir des bougies
    let weeks = argos_core::aggregate(bars, 604_800);
    let pw = weeks[weeks.len() - 2];
    for row in st["lvl"].as_array().unwrap() {
        let (p, want) = match row[0].as_str().unwrap() {
            "PDH" => (r.prev_day.unwrap().hi, row[1].clone()),
            "PDL" => (r.prev_day.unwrap().lo, row[1].clone()),
            "PWH" => (r.prev_week.unwrap().hi, Value::from(pw.high)),
            _ => (r.prev_week.unwrap().lo, Value::from(pw.low)),
        };
        same(case, row[0].as_str().unwrap(), p, &want);
    }

    // plan, score, verdict, alertes
    assert_eq!(r.plan_ok, st["planOk"] == true, "[{case}] plan prêt");
    same(case, "entrée", r.entry, &st["entry"]);
    same(case, "SL", r.sl, &st["sl"]);
    same(case, "TP1", r.tp1, &st["tp1"]);
    same(case, "TP2", r.tp2, &st["tp2"]);
    same(case, "R:R", r.rr, &st["rr"]);
    assert_eq!(
        (r.raw as i64, r.score as i64),
        (st["raw"].as_i64().unwrap(), st["score"].as_i64().unwrap()),
        "[{case}] score"
    );
    let miss: Vec<Missing> = st["miss"].as_array().unwrap().iter().map(|m| missing_js(m.as_str().unwrap())).collect();
    assert_eq!(r.missing, miss, "[{case}] ce qui manque");
    assert_eq!(r.verdict, verdict_js(st["verdict"].as_str().unwrap()), "[{case}] verdict");
    assert_eq!((r.buy, r.sell), (st["buy"] == 1, st["sell"] == 1), "[{case}] alertes");
}

#[test]
fn argos_smc_identique_au_site() {
    let bars = bars();
    let all: Value = serde_json::from_str(include_str!("fixtures/smc_pro_js.json")).unwrap();
    let mut n = 0;
    for (case, js) in all.as_object().unwrap() {
        let name = case.split('_').next().unwrap();
        let cut = js["cut"].as_u64().unwrap() as usize;
        let r = compute(&bars[..cut], 1800, &settings(name));
        check(case, &r, js, &bars[..cut]);
        n += 1;
    }
    assert_eq!(n, 30);
}
