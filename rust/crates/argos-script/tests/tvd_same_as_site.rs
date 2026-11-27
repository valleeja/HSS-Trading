//! Argos TVD Pro identique à v1 : valeurs attendues générées en lançant le code de v1 (app.js) sur les mêmes bougies.
//! Since B-098, its averages are TradingView's `ta.ema`, as in Jee's Pine script: the values and marks that this
//! changed are said where they are written below.
use argos_core::Bar;
use argos_script::indicators::tvd::{compute, MarkKind::*, Vector::*, PARAMS};
use argos_script::inputs::defaults;

/// Since B-098, two marks of the old site are gone: AI BUY at bar 12 and Trend SELL at bar 39. The old site's averages
/// started on the first value and gave them; with TradingView's `ta.ema`, as in Jee's Pine script, the AI Index and its
/// signal line do not exist yet at bar 12, nor the EMA 50 that the Trend signals read at bar 39. Jee's Pine script
/// draws neither (`ema_same_as_tradingview.rs`).
const MARKS: &[(usize, argos_script::indicators::tvd::MarkKind)] = &[
    (20, RevBull),
    (22, RevBear),
    (32, RevBear),
    (35, AiSell),
    (49, AiBuy),
    (53, RevBull),
    (67, RevBear),
    (73, AiSell),
    (77, RevBull),
    (80, TrendBuy),
    (82, AiSell),
    (78, HBull),
    (84, TrendSell),
    (85, RevBull),
    (91, RevBull),
    (92, AiBuy),
    (94, TrendBuy),
    (102, RevBear),
    (103, AiSell),
    (108, RevBear),
    (117, AiSell),
    (118, RevBear),
    (114, Bear),
    (121, TrendSell),
    (126, RevBull),
    (130, RevBull),
    (137, RevBull),
    (138, AiBuy),
    (142, RevBull),
    (151, TrendSell),
    (155, RevBull),
    (157, RevBull),
    (160, AiBuy),
    (160, RevBull),
    (172, AiSell),
    (169, HBear),
    (175, RevBear),
    (176, TrendSell),
    (177, RevBull),
    (182, RevBull),
    (187, RevBull),
    (188, AiBuy),
    (194, RevBear),
    (197, TrendSell),
    (199, RevBull),
    (201, RevBull),
    (203, AiBuy),
    (203, RevBull),
    (198, Bull),
    (209, GoldSell),
    (210, RevBull),
    (212, TrendBuy),
    (210, Bull),
    (221, RevBear),
    (224, AiSell),
    (224, RevBear),
    (227, TrendSell),
    (235, RevBull),
    (230, HBull),
    (236, TrendBuy),
    (240, TrendSell),
    (244, RevBull),
    (249, AiBuy),
    (259, TrendSell),
    (265, TrendSell),
    (275, AiBuy),
    (280, RevBull),
    (281, RevBull),
    (286, RevBear),
    (293, RevBear),
    (296, RevBear),
    (298, GoldSell),
    (293, HBear),
    (307, AiBuy),
    (309, TrendBuy),
    (312, RevBear),
    (322, AiSell),
    (326, TrendSell),
    (336, AiBuy),
    (337, RevBull),
    (339, TrendBuy),
    (348, TrendSell),
    (353, RevBull),
    (355, AiBuy),
    (356, GoldBuy),
    (357, RevBear),
    (366, AiSell),
    (364, Bear),
    (370, RevBear),
    (371, RevBear),
    (372, RevBear),
    (373, AiSell),
    (377, TrendBuy),
    (377, RevBear),
    (375, HBull),
    (383, TrendBuy),
    (384, AiSell),
    (384, RevBear),
    (386, AiSell),
    (386, RevBear),
    (381, HBull),
    (388, AiSell),
    (396, TrendSell),
    (394, Bear),
    (410, AiBuy),
    (413, RevBull),
    (418, RevBull),
    (422, AiBuy),
    (422, RevBull),
    (421, Bull),
    (429, RevBear),
    (431, RevBear),
    (434, RevBear),
    (436, RevBear),
    (438, AiSell),
    (442, RevBull),
    (449, TrendSell),
    (451, RevBull),
    (459, AiBuy),
    (460, RevBull),
    (462, RevBull),
    (475, AiBuy),
    (487, RevBear),
    (490, AiSell),
    (490, RevBear),
    (486, HBear),
    (493, TrendSell),
    (501, TrendBuy),
];
const VECTOR: &[(usize, argos_script::indicators::tvd::Vector)] = &[
    (36, HighDown),
    (42, MidDown),
    (44, MidUp),
    (62, MidUp),
    (66, HighUp),
    (67, MidDown),
    (68, HighDown),
    (97, HighUp),
    (98, HighDown),
    (99, HighDown),
    (103, MidDown),
    (107, MidUp),
    (113, HighDown),
    (115, HighDown),
    (116, MidDown),
    (120, MidDown),
    (122, MidDown),
    (123, MidUp),
    (124, HighDown),
    (127, MidDown),
    (133, MidDown),
    (134, MidDown),
    (135, HighDown),
    (136, HighDown),
    (137, MidUp),
    (150, HighDown),
    (152, MidUp),
    (154, MidDown),
    (155, MidUp),
    (176, MidDown),
    (180, HighDown),
    (181, HighUp),
    (182, HighUp),
    (183, HighDown),
    (184, HighDown),
    (185, HighUp),
    (196, HighDown),
    (197, MidUp),
    (198, HighDown),
    (209, HighDown),
    (210, HighUp),
    (211, MidUp),
    (239, HighDown),
    (240, MidDown),
    (241, HighDown),
    (243, MidDown),
    (249, MidDown),
    (260, HighUp),
    (261, HighDown),
    (266, HighDown),
    (270, HighDown),
    (272, MidUp),
    (277, MidDown),
    (278, HighUp),
    (280, HighDown),
    (281, HighUp),
    (298, HighDown),
    (299, HighDown),
    (300, MidDown),
    (301, MidUp),
    (304, HighDown),
    (305, MidUp),
    (308, MidUp),
    (311, HighDown),
    (328, HighDown),
    (333, HighDown),
    (353, MidUp),
    (355, HighUp),
    (356, HighUp),
    (357, HighDown),
    (384, HighDown),
    (385, HighUp),
    (386, HighDown),
    (387, HighUp),
    (390, HighDown),
    (392, HighDown),
    (407, MidDown),
    (415, MidDown),
    (417, HighDown),
    (437, HighDown),
    (441, MidDown),
    (442, HighUp),
    (443, MidUp),
    (453, HighDown),
    (455, HighDown),
    (463, MidDown),
    (465, HighDown),
    (472, MidDown),
    (473, HighDown),
    (474, HighUp),
    (502, HighUp),
];

#[test]
fn tvd_identique_au_site() {
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
    let t = compute(&bars, &defaults(&PARAMS));
    let n = bars.len();
    for (k, i) in [300, 454, n - 1].into_iter().enumerate() {
        for (name, rust, site) in [
            ("fast", t.fast[i], [78.14001331609774, 77.5576695784863, 76.4021050062443][k]),
            // TradingView's `ta.ema` since B-098, as Jee's Pine script of TVD Pro (started on the simple mean of the first
            // 50 / 200 closes; the old site started on the first close: slow 78.88678628391168, 78.12681939609948,
            // 76.49256034775057, trend 80.98649129519774, 79.33390948930554, 78.12129508239525). Redone apart in Python.
            ("slow", t.slow[i], [78.88678086943669, 78.1268193846721, 76.49256034614136][k]),
            ("trend", t.trend[i], [81.12501511224723, 79.36360599666511, 78.13948787233032][k]),
        ] {
            assert!((rust - site).abs() < 1e-9 * site.abs(), "EMA {name} bougie {i} : Rust {rust} / site {site}");
        }
        assert_eq!(t.cloud[i], [-1, -1, 1][k], "nuage bougie {i}");
    }
    let got: Vec<_> = t.marks.iter().map(|m| (m.index, m.kind)).collect();
    assert_eq!(got, MARKS, "repères (signaux, retournements, divergences)");
    let vec: Vec<_> = t.vector.iter().enumerate().filter_map(|(i, v)| v.map(|v| (i, v))).collect();
    assert_eq!(vec, VECTOR, "bougies vector");
    assert_eq!(t.dash.len(), 8);
}

/// 🔔 Les 11 alertes du Pine : mêmes bougies que les repères Trend / GOLD / AI, les bougies Vector 200 % et les
/// divergences (à la confirmation du pivot, Pivot Right bougies après).
#[test]
fn tvd_alertes_comme_pine() {
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
    let t = compute(&bars, &defaults(&PARAMS));
    let al = t.alerts();
    assert_eq!(al.len(), 11);
    let at = |k: usize| al[k].1.iter().enumerate().filter(|(_, v)| **v == 1.0).map(|(i, _)| i).collect::<Vec<_>>();
    let marks = |kind| t.marks.iter().filter(|m| m.kind == kind).map(|m| m.index).collect::<Vec<_>>();
    assert_eq!(at(3), marks(GoldBuy));
    assert_eq!(at(2), marks(GoldSell));
    assert_eq!(at(4), marks(AiBuy));
    assert_eq!(at(5), marks(AiSell));
    let vec_high: Vec<usize> = t
        .vector
        .iter()
        .enumerate()
        .filter(|(_, v)| matches!(v, Some(HighUp | HighDown)))
        .map(|(i, _)| i)
        .collect();
    assert_eq!(at(6), vec_high);
    let piv_r = 5;
    let divs = |kind| marks(kind).into_iter().map(|i| i + piv_r).collect::<Vec<_>>();
    assert_eq!((at(7), at(8), at(9), at(10)), (divs(Bull), divs(Bear), divs(HBull), divs(HBear)));
    // Trend BUY : le repère est caché quand c'est un GOLD BUY, l'alerte part quand même
    let mut tb = marks(TrendBuy);
    tb.extend(marks(GoldBuy));
    tb.sort_unstable();
    assert_eq!(at(0), tb);
}
