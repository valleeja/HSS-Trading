//! 📉 Indicateurs classiques en `Plot` générique (RSI, MACD, croisement EMA, Stochastique, ATR, VWAP,
//! volume relatif, EMA multiples…) : lignes, histogramme, niveaux et signaux, et la mini-courbe des listes (`spark`).

use super::*;

/// Signaux d'achat (sous la bougie, vert) / de vente (au-dessus, rouge) avec leur texte.
pub(super) fn signals(list: impl IntoIterator<Item = (usize, bool)>, buy: &'static str, sell: &'static str) -> Vec<(usize, bool, Rgb, &'static str)> {
    list.into_iter()
        .map(|(i, up)| {
            if up {
                (i, true, crate::palette::up(), buy)
            } else {
                (i, false, crate::palette::down(), sell)
            }
        })
        .collect()
}

/// Croisements de `a` au-dessus (true) / en dessous (false) de `b`.
pub(super) fn crosses(a: &[f64], b: &[f64]) -> Vec<(usize, bool)> {
    (1..a.len().min(b.len()))
        .filter_map(|i| {
            let (p, q) = (a[i - 1] - b[i - 1], a[i] - b[i]);
            (p.is_finite() && q.is_finite() && (p <= 0.0) != (q <= 0.0)).then_some((i, q > 0.0))
        })
        .collect()
}

/// Le `Plot` d'un indicateur simple ; `None` : il a son propre dessin.
pub(super) fn plot(ind: Ind, bars: &[Bar], set: &Settings, reel: &[argos_core::orderflow::FlowBar]) -> Option<Plot> {
    let vals = values(set, ind);
    let inp = argos_script::inputs::Inputs {
        params: ind.params().1,
        vals: &vals,
    };
    let col = |k: &str| crate::palette::on_chart(inp.rgb(k));
    let p = |sub, lines, levels: &[f64], range| Plot {
        ind,
        name: ind.name(),
        sub,
        lines,
        hist: vec![],
        zones: vec![],
        bars: vec![],
        levels: levels.to_vec(),
        range,
        marks: vec![],
        fill: None,
        fill_down: None,
        ahead: vec![],
        dash: vec![],
        tableau: vec![],
    };
    let mut pl = match ind {
        Ind::Rsi => {
            let r = classic::rsi_n(bars, inp.len("len"));
            let (haut, bas) = (inp.get("haut"), inp.get("bas"));
            let lv = [haut, bas].map(|l| vec![l; r.len()]);
            Plot {
                marks: signals(
                    crosses(&r, &lv[1])
                        .into_iter()
                        .filter(|m| m.1)
                        .chain(crosses(&r, &lv[0]).into_iter().filter(|m| !m.1)),
                    "RSI",
                    "RSI",
                ),
                levels: vec![haut, bas],
                ..p(true, vec![(r, col("col"), 2.0)], &[], Some((0.0, 100.0)))
            }
        }
        Ind::Macd => {
            let m = classic::macd_n(bars, inp.len("fast"), inp.len("slow"), inp.len("signal"));
            Plot {
                hist: m.hist.iter().map(|&v| (v, crate::palette::on_chart(hex(0x26a69a)), 0.85)).collect(),
                marks: signals(crosses(&m.line, &m.signal), "MACD", "MACD"),
                ..p(true, vec![(m.line, col("col"), 2.0), (m.signal, col("colSig"), 2.0)], &[0.0], None)
            }
        }
        Ind::EmaCross => {
            let e = classic::ema_cross_n(bars, inp.len("fast"), inp.len("slow"));
            let marks = signals(e.up.iter().map(|&i| (i, true)).chain(e.down.iter().map(|&i| (i, false))), "Achat", "Vente");
            Plot {
                marks,
                ..p(false, vec![(e.fast, col("col"), 2.0), (e.slow, col("colSlow"), 2.0)], &[], None)
            }
        }
        Ind::Stochastic => {
            let (k, d) = classic::stochastic_n(bars, inp.len("len"), inp.len("smooth"), inp.len("d"));
            p(true, vec![(k, col("col"), 2.0), (d, ORANGE, 1.0)], &[80.0, 20.0], Some((0.0, 100.0)))
        }
        Ind::Atr => p(true, vec![(classic::atr_n(bars, inp.len("len")), col("col"), 2.0)], &[], None),
        Ind::Vwap => p(false, vec![(classic::vwap_src(bars, inp.on("hl3")), col("col"), 2.0)], &[], None),
        // 📚 classiques des manuels
        Ind::StochRsi => {
            let (k, d) = textbook::stoch_rsi_n(bars, inp.len("rsi"), inp.len("len"), inp.len("k"), inp.len("d"));
            p(true, vec![(k, col("col"), 2.0), (d, ORANGE, 1.0)], &[80.0, 20.0], Some((0.0, 100.0)))
        }
        Ind::Ichimoku => {
            let i = textbook::ichimoku_n(
                bars,
                inp.len("tenkan"),
                inp.len("kijun"),
                inp.len("senkou"),
                inp.get("shift").round().max(0.0) as usize,
            );
            // TradingView's cloud: 25 bars ahead of the last bar, green (67, 160, 71) where senkou A is above
            // senkou B, red (244, 67, 54) where it is below, both at 90 % transparency (B-045)
            Plot {
                fill: Some((2, 3, hex(0x43a047), 0.10)),
                fill_down: Some(hex(0xf44336)),
                ahead: vec![vec![], vec![], i.ahead_a, i.ahead_b, vec![]],
                ..p(
                    false,
                    vec![
                        (i.tenkan, col("col"), 1.0),
                        (i.kijun, hex(0xb71c1c), 1.0),
                        (i.span_a, hex(0x43a047), 1.0),
                        (i.span_b, hex(0xef5350), 1.0),
                        (i.chikou, hex(0x9e9e9e), 1.0),
                    ],
                    &[],
                    None,
                )
            }
        }
        Ind::Keltner | Ind::Donchian => {
            let (u, m, l) = if ind == Ind::Keltner {
                textbook::keltner_n(bars, inp.len("len"), inp.len("atr"), inp.get("mult"))
            } else {
                textbook::donchian_n(bars, inp.len("len"))
            };
            let c = col("col");
            Plot {
                fill: Some((0, 2, c, 0.06)),
                ..p(false, vec![(u, c, 1.0), (m, hex(0xff6d00), 1.0), (l, c, 1.0)], &[], None)
            }
        }
        Ind::Psar => p(
            false,
            vec![(textbook::psar_n(bars, inp.get("start"), inp.get("step"), inp.get("max")), col("col"), 1.0)],
            &[],
            None,
        ),
        Ind::Dmi => {
            let (pl, mi, adx) = textbook::dmi_n(bars, inp.len("len"), inp.len("adx"));
            Plot {
                levels: vec![inp.get("seuil")],
                ..p(
                    true,
                    vec![(pl, crate::palette::up(), 1.5), (mi, crate::palette::down(), 1.5), (adx, col("col"), 2.0)],
                    &[],
                    None,
                )
            }
        }
        Ind::Cci => p(
            true,
            vec![(textbook::cci_n(bars, inp.len("len")), col("col"), 2.0)],
            &[100.0, 0.0, -100.0],
            None,
        ),
        Ind::WilliamsR => p(
            true,
            vec![(textbook::williams_r_n(bars, inp.len("len")), col("col"), 2.0)],
            &[-20.0, -80.0],
            Some((-100.0, 0.0)),
        ),
        // OBV, et 🧮 CVD (delta cumulé : vrais trades des bougies complètes, sinon estimé) avec les mêmes réglages
        Ind::Obv | Ind::Cvd => {
            let (o, niveaux): (Vec<f64>, &[f64]) = if ind == Ind::Obv {
                (textbook::obv(bars), &[])
            } else {
                (argos_script::indicators::flow::cvd(bars, reel), &[0.0])
            };
            let mut lines = vec![(o.clone(), col("col"), 2.0)];
            let n = inp.get("sig").round();
            if n >= 1.0 {
                lines.push((argos_script::ta::sma(&o, n as usize), ORANGE, 1.0));
            }
            p(true, lines, niveaux, None)
        }
        Ind::Supertrend => {
            let (line, up) = textbook::supertrend_n(bars, inp.len("atr"), inp.get("mult"));
            let side = |want: bool| -> Vec<f64> { line.iter().zip(&up).map(|(v, u)| if *u == want { *v } else { f64::NAN }).collect() };
            p(
                false,
                vec![(side(true), crate::palette::up(), 2.0), (side(false), crate::palette::down(), 2.0)],
                &[],
                None,
            )
        }
        Ind::Aroon => {
            let (u, d) = textbook::aroon_n(bars, inp.len("len"));
            p(
                true,
                vec![(u, crate::palette::up(), 2.0), (d, crate::palette::down(), 2.0)],
                &[70.0, 30.0],
                Some((0.0, 100.0)),
            )
        }
        Ind::Mfi => p(
            true,
            vec![(textbook::mfi_n(bars, inp.len("len")), col("col"), 2.0)],
            &[80.0, 20.0],
            Some((0.0, 100.0)),
        ),
        Ind::RelVolume => {
            let r = classic::relative_volume_n(bars, inp.len("len"));
            let (c, pic) = (col("col"), inp.get("pic"));
            Plot {
                hist: r.iter().map(|&v| (v, c, 0.85)).collect(),
                marks: r.iter().enumerate().filter(|(_, v)| **v >= pic).map(|(i, _)| (i, true, GOLD, "")).collect(),
                ..p(true, vec![], &[1.5, 2.0, 3.0], None)
            }
        }
        Ind::EmaMulti => p(
            false,
            classic::ema_multiples(bars, &vals)
                .into_iter()
                .zip(EMA_COLORS)
                .enumerate()
                .map(|(k, (v, c))| (v, c, if k == 3 { 2.0 } else { 1.0 }))
                .collect(),
            &[],
            None,
        ),
        Ind::Heatmap => {
            let h = classic::heatmap_volume(bars, &vals);
            let heat = ["h0", "h1", "h2", "h3", "h4"].map(col);
            // couleur de chaque bougie : la heatmap, ou hausse / baisse plus pâle quand le palier baisse (comme v1)
            let shade = [1.0, 0.82, 0.64, 0.46, 0.3];
            let ud = inp.pick("cmode") == 1;
            let colors: Vec<(Rgb, f32)> = (0..bars.len())
                .map(|i| {
                    let t = h.tier[i] as usize;
                    if ud {
                        (if bars[i].close > bars[i].open { col("cu") } else { col("cd") }, shade[t])
                    } else {
                        (heat[t], 1.0)
                    }
                })
                .collect();
            let level = |m: f64| h.mean.iter().zip(&h.std).map(|(a, s)| a + m * s).collect::<Vec<f64>>();
            let t = ["t1", "t2", "t3", "t4"].map(|k| inp.get(k));
            let (lines_on, bg_on) = match inp.pick("zmode") {
                1 => (true, false),
                2 => (false, true),
                3 => (true, true),
                _ => (false, false),
            };
            let mut lines = vec![];
            if inp.on("ma") {
                lines.push((h.mean.clone(), hex(0x9e9e9e), 2.0));
            }
            if lines_on {
                lines.extend(t.iter().zip(heat).map(|(m, c)| (level(*m), c, 1.0)));
            }
            let mut zones = vec![];
            if let (true, Some(z)) = (bg_on, bars.len().checked_sub(1)) {
                let l: Vec<f64> = t.iter().map(|m| level(*m)[z]).collect();
                let edges = [l[0] + (l[0] - l[1]), l[0], l[1], l[2], l[3], 0.0];
                zones = (0..5).map(|k| (edges[k], edges[k + 1], heat[k], 0.13)).collect();
            }
            Plot {
                tableau: if inp.on("showTable") { classic::heatmap_tableau(&h) } else { vec![] },
                hist: h.volume.iter().zip(&colors).map(|(&v, &(c, a))| (v, c, a)).collect(),
                zones,
                bars: if inp.on("bar") {
                    colors.iter().map(|(c, _)| Some(*c)).collect()
                } else {
                    vec![]
                },
                ..p(true, lines, &[], None)
            }
        }
        Ind::Bands => {
            let a = classic::argos_bands(bars, &vals);
            // squeeze : un repère jaune au début, comme v1 ; cassures hors bandes (option) ; nuage entre les bandes
            let mut marks: Vec<(usize, bool, Rgb, &'static str)> = (1..a.squeeze.len())
                .filter(|&i| a.squeeze[i] && !a.squeeze[i - 1])
                .map(|i| (i, true, hex(0xffd600), "Squeeze"))
                .collect();
            if inp.on("touch") {
                let (u, l) = (&a.bands.upper, &a.bands.lower);
                for i in 1..bars.len() {
                    let (c, c0) = (bars[i].close, bars[i - 1].close);
                    if c > u[i] && c0 <= u[i - 1] {
                        marks.push((i, false, BLUE, "▲BB"));
                    }
                    if c < l[i] && c0 >= l[i - 1] {
                        marks.push((i, true, BLUE, "▼BB"));
                    }
                }
            }
            let fill = inp.on("fill").then(|| (0, 2, col("cf"), 0.08));
            let tableau = if inp.on("showTable") { classic::bands_tableau(&a, bars) } else { vec![] };
            Plot {
                marks,
                fill,
                tableau,
                ..p(
                    false,
                    vec![(a.bands.upper, BLUE, 1.0), (a.bands.basis, hex(0xff6d00), 1.0), (a.bands.lower, BLUE, 1.0)],
                    &[],
                    None,
                )
            }
        }
        Ind::BandsOsc => {
            let (bull, bear) = classic::bollinger_oscillator(bars, &vals);
            // signaux au croisement de 50 (option de v1)
            let marks = if inp.on("sig") {
                let mid = vec![50.0; bull.len()];
                signals(
                    crosses(&bull, &mid)
                        .into_iter()
                        .filter(|m| m.1)
                        .chain(crosses(&bear, &mid).into_iter().filter(|m| m.1).map(|(i, _)| (i, false))),
                    "",
                    "",
                )
            } else {
                vec![]
            };
            let tableau = if inp.on("showTable") {
                classic::bands_osc_tableau(&bull, &bear)
            } else {
                vec![]
            };
            Plot {
                marks,
                tableau,
                ..p(true, vec![(bull, col("cbu"), 1.5), (bear, col("cbe"), 1.5)], &[50.0], Some((0.0, 100.0)))
            }
        }
        _ => return None,
    };
    // 🎨 onglet Style : épaisseur et type de ligne de chaque sortie (« ep1 », « ty1 »… s'il y en a)
    let a = |k: &str| ind.params().1.iter().any(|p| p.key == k);
    pl.dash = (1..=pl.lines.len())
        .map(|k| if a(&format!("ty{k}")) { inp.pick(&format!("ty{k}")) } else { 0 })
        .collect();
    for (k, l) in pl.lines.iter_mut().enumerate() {
        let key = format!("ep{}", k + 1);
        if a(&key) {
            l.2 = inp.get(&key) as f32;
        }
    }
    Some(pl)
}

/// Aperçu d'un indicateur (Marketplace, comme v1) : sa première série calculée sur `bars`, et sa couleur.
pub fn spark(ind: Ind, bars: &[Bar], tf_seconds: i64) -> (Vec<f64>, Rgb) {
    let o = compute(bars, tf_seconds, &[ind], &Settings::new(), None, 0, &Default::default(), &[]);
    let closes = || bars.iter().map(|b| b.close).collect::<Vec<f64>>();
    let first_line = |o: &IndOut| {
        o.plots.first().and_then(|p| {
            p.lines
                .first()
                .map(|l| (l.0.clone(), l.1))
                .or_else(|| Some((p.hist.iter().map(|h| h.0).collect(), p.hist.first().map_or(GREY, |h| h.1))))
        })
    };
    match ind {
        Ind::Tvd => (o.tvd.map(|t| t.fast).unwrap_or_default(), hex(0x22d3ee)),
        Ind::Cipher => (o.cipher.map(|c| c.wt1).unwrap_or_default(), hex(0x4994ec)),
        Ind::AiTrend => (o.ai.map(|a| a.ai).unwrap_or_default(), hex(0x00e676)),
        Ind::Tbo => (o.tbo.map(|t| t.core.delta).unwrap_or_default(), hex(0xff0000)),
        Ind::Bands => (o.plots.first().map(|p| p.lines[1].0.clone()).unwrap_or_default(), hex(0xff6d00)),
        _ => first_line(&o).unwrap_or_else(|| (closes(), hex(0x7c5cff))),
    }
}
