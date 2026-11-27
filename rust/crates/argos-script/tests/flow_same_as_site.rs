//! Argos Footprint et Bubbles identiques à v1 (valeurs générées en lançant le code de v1 sur les mêmes bougies ; Bubbles
//! reçoit l'estimation de v1 comme vrais trades) ;
//! Argos Price Action (order blocks d'Argos, écrits de zéro) vérifié sur les mêmes bougies réelles.
use argos_core::Bar;
use argos_script::indicators::{flow, order_blocks};
use argos_script::inputs::defaults;

fn bars() -> Vec<Bar> {
    let raw: serde_json::Value = serde_json::from_str(include_str!("fixtures/hype_hl_50m.json")).unwrap();
    raw.as_array()
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

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-9 * b.abs().max(1.0)
}

#[test]
fn order_blocks_sur_bougies_reelles() {
    let b = bars();
    let r = order_blocks::compute(&b, &defaults(&order_blocks::PARAMS));
    assert!(!r.rising.is_empty() && !r.falling.is_empty(), "des zones dans les deux sens sur 500 bougies");
    for (zones, bearish_origin) in [(&r.rising, true), (&r.falling, false)] {
        for z in zones {
            let o = &b[z.origin];
            assert!(z.high >= z.low, "zone dans le bon ordre");
            assert_eq!(o.close < o.open, bearish_origin, "la bougie de sens opposé à l'impulsion");
            assert!(z.broken_at.is_none_or(|k| k > z.origin), "cassée après sa naissance");
        }
    }
    assert!(r.rising.windows(2).all(|w| w[0].origin > w[1].origin), "les plus récentes d'abord");
    assert!(r.flips.iter().all(|f| f.at < b.len() && f.price.is_finite()));
}

#[test]
fn footprint_identique_au_site() {
    let b = bars();
    let fp = flow::footprint(&b, &defaults(&flow::FOOTPRINT_PARAMS), &[]);
    assert_eq!((fp.step, fp.bars.len()), (0.05, 150));
    let last = fp.bars.last().unwrap();
    let site: &[(i64, f64, f64)] = &[
        (1534, 63.1405819296325, 33.998774885186734),
        (1535, 789.2572741196211, 424.98468606441133),
        (1536, 789.2572741193968, 424.98468606429054),
        (1537, 309.6264885077881, 443.20352680593936),
        (1538, 158.16413515697087, 448.9568449348728),
        (1539, 158.16413515697087, 448.9568449348728),
        (1540, 158.16413515701583, 448.9568449350004),
        (1541, 158.16413515697087, 448.9568449348728),
        (1542, 206.19183432029297, 510.2109221880929),
        (1543, 424.98468606429054, 789.2572741193968),
        (1544, 101.9963246554394, 189.42174578867318),
    ];
    assert_eq!(last.rows.len(), site.len());
    for (r, s) in last.rows.iter().zip(site) {
        assert!(r.0 == s.0 && near(r.1, s.1) && near(r.2, s.2), "rangée {r:?} / site {s:?}");
    }
    assert_eq!(last.poc, 1535);
    assert!(near(last.d, -1294.7779913112197) && near(last.mx, 789.2572741196211));
}

#[test]
fn bubbles_identiques_au_site() {
    let b = bars();
    // 🧮 Bubbles now reads only real trades: v1's candle estimate is given to it as if it were the real trades (one
    // FlowBar per bar, at the screen step, so its rows are taken unchanged)
    let vals = defaults(&flow::BUBBLES_PARAMS);
    let per = argos_script::inputs::Inputs {
        params: &flow::BUBBLES_PARAMS,
        vals: &vals,
    }
    .get("per");
    let step = flow::auto_step(&b, per);
    let reel: Vec<argos_core::orderflow::FlowBar> = b
        .iter()
        .map(|x| argos_core::orderflow::FlowBar {
            time: x.time,
            pas: step,
            rangees: flow::est_rows(x, step),
            complete: true,
        })
        .collect();
    let bu = flow::bubbles(&b, &vals, &reel, &|v| v.to_string());
    assert_eq!(bu.items.len() + bu.pocs.len(), 338);
    let f = &bu.items[0];
    assert!(near(f.price, 77.85000000000001) && near(f.r, 15.24) && !f.buy);
    assert_eq!(b[f.i].time, 1792067400);
    let dash: Vec<(&str, &str)> = bu.dash.iter().map(|(k, v)| (*k, v.as_str())).collect();
    assert_eq!(
        dash[..3],
        [("Bulles achat (50 b.)", "0"), ("Bulles vente (50 b.)", "8"), ("Pression", "Vendeurs 🔴")]
    );
}
