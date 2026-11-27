//! 📜 Built-in indicators written in Argos Script: their exact source, run by the interpreter as it is (their settings
//! are their `input()` calls, their drawing the script's output). Licence headers stay in the source.

/// « Spring Tension Ribbon + Volume Exhaustion », Jee's script (MPL 2.0, © cgagnonswtor), from `pine/`.
pub const SPRING_TENSION: &str = include_str!("../../../../pine/Spring_Tension.pine");

/// TradingView's built-in moving averages, their exact code (pasted by Jee, 18 Nov 2026; WMA written from his
/// captures, see its header), from `pine/tradingview/`.
pub const SMA: &str = include_str!("../../../../pine/tradingview/sma.pine");
pub const WMA: &str = include_str!("../../../../pine/tradingview/wma.pine");
pub const HMA: &str = include_str!("../../../../pine/tradingview/hma.pine");
pub const VWMA: &str = include_str!("../../../../pine/tradingview/vwma.pine");
pub const SMMA: &str = include_str!("../../../../pine/tradingview/smma.pine");
pub const MA_RIBBON: &str = include_str!("../../../../pine/tradingview/ma-ribbon.pine");

/// « Argos Bollinger + RSI » (Jee, 19 Nov 2026): Argos's own script from the standard formulas, the idea of ChartArt's
/// « RSI + Bollinger Bands Double Strategy » (none of its code), from `pine/argos/`.
pub const BOLLINGER_RSI: &str = include_str!("../../../../pine/argos/argos-bollinger-rsi.pine");

/// « Argos Vague » (Jee, 20 Nov 2026): Argos's own oscillator (wave, rolling CVD bands, trend cloud, divergences,
/// confluence triangles, volume exhaustion), written from the public description of BigBeluga's « Nautilus PRO+ »
/// (none of its code), from `pine/argos/`.
pub const ARGOS_VAGUE: &str = include_str!("../../../../pine/argos/argos-vague.pine");

/// TradingView's built-in « Bollinger Bands » (written by Argos from TradingView's script, 19 Nov 2026): the first native
/// indicator replaced by its Argos Script source (see `catalogue::CONVERSIONS` for its saved settings).
pub const BOLLINGER: &str = include_str!("../../../../pine/tradingview/bollinger.pine");

/// LuxAlgo's « Smart Money Concepts » (CC BY-NC-SA 4.0, © LuxAlgo), in the Pine v6 version Argos made of the v5 code
/// Jee pasted (its changes listed in its header), from `pine/references/`: Argos Script's SMC reference, first of
/// the SMC list and drawn as TradingView draws it (Jee, 25 Nov 2026).
pub const LUXALGO_SMC: &str = include_str!("../../../../pine/references/luxalgo-smart-money-concepts-v6.pine");

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalogue::{self, entrees, source};

    /// 📜 Argos Bollinger + RSI (Jee, 19 Nov): its defaults are those of Jee's TradingView capture (RSI 6, Bollinger
    /// 200, ×2, both colour boxes on); its bands are exactly SMA(close, 200) ± 2 × the population standard deviation
    /// (Pine's ta.stdev); a buy arrow follows a buy signal on the next bar with « RSI_BB_L » and +1 (+2 when it turns a
    /// short position around), like a TradingView strategy — shown only, no order.
    #[test]
    fn bollinger_rsi_defauts_bandes_et_fleches() {
        let e = entrees(BOLLINGER_RSI);
        let def = |t: &str| e.iter().find(|x| x.title == t).unwrap_or_else(|| panic!("« {t} »")).def.clone();
        assert_eq!(
            e.iter().map(|x| x.title.as_str()).collect::<Vec<_>>(),
            [
                "Période RSI",
                "Période Bollinger",
                "Multiplicateur",
                "Activer la couleur des bougies",
                "Activer la couleur du fond"
            ]
        );
        assert_eq!(
            (def("Période RSI"), def("Période Bollinger"), def("Multiplicateur")),
            (crate::lang::InVal::Num(6.0), crate::lang::InVal::Num(200.0), crate::lang::InVal::Num(2.0))
        );
        assert_eq!(def("Activer la couleur des bougies"), crate::lang::InVal::Bool(true));
        assert_eq!(def("Activer la couleur du fond"), crate::lang::InVal::Bool(true));
        // bands against the reference (SMA 200 + 2 × population stdev)
        let bars: Vec<argos_core::Bar> = (0..700)
            .map(|i| {
                // (a quiet wave, a short plunge under the lower band then back up, a short jump over the upper band then
                // back down: one buy and one sell signal)
                let mut c = 63_000.0 + (i as f64 / 23.0).sin() * 300.0;
                if (300..304).contains(&i) {
                    c -= 4_000.0;
                }
                if (500..504).contains(&i) {
                    c += 4_000.0;
                }
                argos_core::Bar {
                    time: i * 3600,
                    open: c - 50.0,
                    high: c + 300.0,
                    low: c - 300.0,
                    close: c,
                    volume: 10.0,
                }
            })
            .collect();
        let out = crate::lang::run(BOLLINGER_RSI, &bars, 3600, &Default::default()).unwrap();
        assert!(out.overlay, "on the price");
        // colours measured on Jee's TradingView captures (19 Nov): bands #850707 / #08A3A5 / #09A00C, fill #37AFE1 at 90 %
        let couleur = |t: &str| out.plots.iter().find(|x| x.title == t).and_then(|p| p.colors.iter().flatten().next().copied());
        assert_eq!(couleur("Bande haute").map(|c| c & 0xffffff), Some(0x850707));
        assert_eq!(couleur("Base").map(|c| c & 0xffffff), Some(0x08a3a5));
        assert_eq!(couleur("Bande basse").map(|c| c & 0xffffff), Some(0x09a00c));
        let fond = out.fills.first().and_then(|f| f.2.iter().flatten().next().copied());
        assert_eq!(fond.map(|c| c & 0xffffff), Some(0x37afe1), "the fill between the bands");
        let closes: Vec<f64> = bars.iter().map(|b| b.close).collect();
        let (m, sd) = (crate::ta::sma(&closes, 200), crate::ta::stdev(&closes, 200));
        let p = |t: &str| out.plots.iter().find(|x| x.title == t).unwrap_or_else(|| panic!("plot « {t} »")).values.clone();
        let (haut, base, bas) = (p("Bande haute"), p("Base"), p("Bande basse"));
        for i in 0..bars.len() {
            for (v, r) in [(haut[i], m[i] + 2.0 * sd[i]), (base[i], m[i]), (bas[i], m[i] - 2.0 * sd[i])] {
                assert!((v.is_nan() && r.is_nan()) || (v - r).abs() < 1e-6, "bar {i}: {v} vs {r}");
            }
        }
        // arrows: texts as TradingView's strategy, never two on one bar
        let fleches: Vec<&crate::lang::Shape> = out.shapes.iter().filter(|s| s.text.contains("RSI_BB")).collect();
        assert!(!fleches.is_empty(), "some signals on this series");
        for f in &fleches {
            assert!(
                ["RSI_BB_L\n+1", "RSI_BB_L\n+2", "-1\nRSI_BB_S", "-2\nRSI_BB_S"].contains(&f.text.as_str()),
                "« {} »",
                f.text
            );
        }
        let mut vues = std::collections::HashSet::new();
        assert!(fleches.iter().all(|f| vues.insert(f.bar)), "one arrow per bar at most");
        // the first entry is +1 / -1 (no position before), a reversal +2 / -2
        assert!(fleches[0].text.contains("+1") || fleches[0].text.contains("-1"), "first: {}", fleches[0].text);
    }

    /// 📜 Spring Tension in the catalogue: Jee's source as it is (licence header kept), its ⚙ from its `input()` calls
    /// (titles, defaults, groups, the options of « Si pas de volume » in the script's order).
    #[test]
    fn spring_tension_au_catalogue() {
        assert!(SPRING_TENSION.starts_with("// This Pine Script® code is subject to the terms of the Mozilla Public License 2.0"));
        assert!(SPRING_TENSION.contains("// © cgagnonswtor"));
        let f = catalogue::fiche("SpringTension").expect("in the catalogue");
        assert_eq!(source(f), Some(SPRING_TENSION));
        assert_eq!(catalogue::FICHES.iter().position(|g| g.id == "SpringTension"), Some(catalogue::AVANT_SCRIPTS));
        let titres: Vec<String> = entrees(SPRING_TENSION).into_iter().map(|s| s.title).collect();
        assert_eq!(
            titres,
            [
                "RSI Length",
                "MFI Length",
                "Lookback prix",
                "RSI max Spring",
                "RSI min Upthrust",
                "Si pas de volume",
                "Volume MA Length",
                "Variation RSI depuis le pic/creux",
                "Volume % sous la moyenne",
                "Fenêtre du cluster",
                "Signaux min. pour NUKE",
                "Short",
                "Long",
                "Fond",
                "Spring",
                "Upthrust",
                "Haussier",
                "Survendu",
                "Neutre",
            ]
        );
        let (groupes, params) = catalogue::reglages(f);
        assert_eq!(groupes, ["🌱 Spring Tension", "📉 Volume Exhaustion", "🎨 Visuels"]);
        let defs: Vec<f64> = params.iter().take(11).map(|p| p.def).collect();
        assert_eq!(defs, [14.0, 14.0, 5.0, 35.0, 65.0, 0.0, 20.0, 5.0, 20.0, 10.0, 4.0]);
        let choix = params.iter().find(|p| p.key == "Si pas de volume").unwrap();
        assert!(matches!(choix.kind, crate::inputs::Kind::Choice(o) if o == ["RSI sur hlc3", "Désactiver"]));
        assert_eq!(params.iter().find(|p| p.key == "RSI max Spring").unwrap().inline, "lvl");
        assert!(!params.iter().find(|p| p.key == "Lookback prix").unwrap().tip.is_empty(), "ⓘ on Lookback prix");
    }

    /// 📜 TradingView's moving averages run as they are, without an error, on real bars, and draw their line(s).
    #[test]
    fn moyennes_tradingview_tournent_telles_quelles() {
        let v: serde_json::Value = serde_json::from_str(include_str!("../tests/fixtures/hype_hl_50m.json")).unwrap();
        let bars: Vec<argos_core::Bar> = v
            .as_array()
            .unwrap()
            .iter()
            .map(|b| argos_core::Bar {
                time: b["time"].as_i64().unwrap(),
                open: b["open"].as_f64().unwrap(),
                high: b["high"].as_f64().unwrap(),
                low: b["low"].as_f64().unwrap(),
                close: b["close"].as_f64().unwrap(),
                volume: b["volume"].as_f64().unwrap(),
            })
            .collect();
        for (nom, src) in [
            ("SMA", SMA),
            ("WMA", WMA),
            ("HMA", HMA),
            ("VWMA", VWMA),
            ("SMMA", SMMA),
            ("MA ribbon", MA_RIBBON),
        ] {
            let out = crate::lang::run(src, &bars, 3000, &Default::default()).unwrap_or_else(|e| panic!("{nom}: {e}"));
            assert!(out.overlay, "{nom}: on the price");
            assert!(out.plots.iter().any(|p| p.values.iter().any(|x| x.is_finite())), "{nom}: a line");
        }
    }

    /// 📜 TradingView's Bollinger Bands run by Argos Script draw the same bands as the native calculation
    /// (`classic::bollinger_n`) on real bars, with the defaults and with Jee-like settings; its inputs are TradingView's
    /// (keys = titles), its style keys those of its three plots and fill.
    #[test]
    fn bollinger_comme_le_natif() {
        use crate::indicators::classic;
        use crate::lang::InVal;
        let v: serde_json::Value = serde_json::from_str(include_str!("../tests/fixtures/hype_hl_50m.json")).unwrap();
        let bars: Vec<argos_core::Bar> = v
            .as_array()
            .unwrap()
            .iter()
            .map(|b| argos_core::Bar {
                time: b["time"].as_i64().unwrap(),
                open: b["open"].as_f64().unwrap(),
                high: b["high"].as_f64().unwrap(),
                low: b["low"].as_f64().unwrap(),
                close: b["close"].as_f64().unwrap(),
                volume: b["volume"].as_f64().unwrap(),
            })
            .collect();
        let titres: Vec<String> = entrees(BOLLINGER).into_iter().map(|s| s.title).collect();
        assert_eq!(titres, ["Length", "Basis MA Type", "Source", "StdDev", "Offset"]);
        let (_, p) = catalogue::reglages(catalogue::fiche("Bollinger").unwrap());
        let cles: Vec<&str> = p.iter().map(|x| x.key).collect();
        assert_eq!(
            cles,
            [
                "Length",
                "Basis MA Type",
                "Source",
                "StdDev",
                "Offset",
                "Basis · couleur",
                "Basis · épaisseur",
                "Basis · type de ligne",
                "Upper · couleur",
                "Upper · épaisseur",
                "Upper · type de ligne",
                "Lower · couleur",
                "Lower · épaisseur",
                "Lower · type de ligne",
                "Remplissage Upper – Lower · couleur",
            ]
        );
        let defs: Vec<f64> = p[..5].iter().map(|x| x.def).collect();
        assert_eq!(defs, [20.0, 0.0, 0.0, 2.0, 0.0], "TradingView's defaults: 20, SMA, close, 2, 0");
        for (len, mult) in [(20usize, 2.0), (34, 2.5), (5, 1.0)] {
            let m = [("Length".to_string(), InVal::Num(len as f64)), ("StdDev".to_string(), InVal::Num(mult))].into();
            let out = crate::lang::run(BOLLINGER, &bars, 3000, &m).unwrap();
            let natif = classic::bollinger_n(&bars, len, mult);
            for (titre, attendu) in [("Basis", &natif.basis), ("Upper", &natif.upper), ("Lower", &natif.lower)] {
                let v = &out.plots.iter().find(|x| x.title == titre).unwrap().values;
                assert_eq!(v.len(), attendu.len());
                for (i, (a, b)) in v.iter().zip(attendu.iter()).enumerate() {
                    assert!(
                        (a.is_nan() && b.is_nan()) || (a - b).abs() <= 1e-9 * b.abs().max(1.0),
                        "{titre} {len}/{mult} bar {i}: {a} ≠ {b}"
                    );
                }
            }
        }
    }

    /// Two inputs with the same title (TradingView's SMA: « Length » of the average and « Length » of the smoothing)
    /// keep their own value: the second gets the key « Length@4 » (its rank), shown « Length ».
    #[test]
    fn deux_reglages_du_meme_nom_restent_separes() {
        use crate::lang::InVal;
        let titres: Vec<String> = entrees(SMA).into_iter().map(|s| s.title).collect();
        assert_eq!(titres, ["Length", "Source", "Offset", "Type", "Length@4", "BB StdDev"]);
        let (_, p) = catalogue::reglages(catalogue::fiche("Sma").unwrap());
        let lissage = p.iter().find(|x| x.key == "Length@4").unwrap();
        assert_eq!((lissage.label, lissage.def), ("Length", 14.0));
        let bars: Vec<argos_core::Bar> = (0..120)
            .map(|i| {
                let c = 100.0 + (i as f64 * 0.3).sin() * 5.0 + i as f64 * 0.1;
                argos_core::Bar {
                    time: i * 60_000,
                    open: c,
                    high: c + 1.0,
                    low: c - 1.0,
                    close: c,
                    volume: 10.0,
                }
            })
            .collect();
        let tracer = |reglages: &[(&str, InVal)]| {
            let m = reglages.iter().map(|(k, v)| (k.to_string(), v.clone())).collect();
            let out = crate::lang::run(SMA, &bars, 60, &m).unwrap();
            (out.plots[0].values.clone(), out.plots[1].values.clone())
        };
        let sma = || ("Type", InVal::Text("SMA".into()));
        let base = tracer(&[sma()]);
        let lissage_50 = tracer(&[sma(), ("Length@4", InVal::Num(50.0))]);
        let moyenne_30 = tracer(&[sma(), ("Length", InVal::Num(30.0))]);
        let egal = |a: &[f64], b: &[f64]| a.iter().zip(b).all(|(x, y)| x == y || (x.is_nan() && y.is_nan()));
        assert!(egal(&base.0, &lissage_50.0), "the smoothing's length leaves the average alone");
        assert!(!egal(&base.1, &lissage_50.1), "… and changes the smoothing");
        assert!(!egal(&base.0, &moyenne_30.0), "the average's length changes the average");
    }

    /// Source (Jee 18 Nov): Volume and (H + L + C + C)/4 added at the end, so the saved indices 0 to 6 keep their
    /// source; the menu in TradingView's order; a script's « close » … « hlcc4 » map to the same indices.
    #[test]
    fn sources_comme_tradingview_sans_decaler() {
        use crate::inputs::{SOURCES, SOURCES_ORDRE_TV};
        assert_eq!(
            SOURCES[..7],
            ["Clôture", "Ouverture", "Haut", "Bas", "(H + L)/2", "(H + L + C)/3", "(O + H + L + C)/4"],
            "saved indices unchanged"
        );
        let menu: Vec<&str> = SOURCES_ORDRE_TV.iter().map(|&k| SOURCES[k]).collect();
        assert_eq!(
            menu,
            [
                "Ouverture",
                "Haut",
                "Bas",
                "Clôture",
                "Volume",
                "(H + L)/2",
                "(H + L + C)/3",
                "(O + H + L + C)/4",
                "(H + L + C + C)/4"
            ]
        );
        let b = argos_core::Bar {
            time: 0,
            open: 1.0,
            high: 4.0,
            low: 2.0,
            close: 3.0,
            volume: 9.0,
        };
        assert_eq!(crate::inputs::price_series(&[b], 7), [9.0], "Volume");
        assert_eq!(crate::inputs::price_series(&[b], 8), [3.0], "(H + L + C + C)/4");
        assert_eq!(crate::inputs::price_series(&[b], 5), [3.0], "(H + L + C)/3 still at 5");
    }

    /// TradingView's ribbon: its untitled settings on each line get stable keys (« MA #1 · 2 » …), shown without a
    /// label; the SMA's Source is a Source (not a number); the defaults are TradingView's.
    #[test]
    fn reglages_des_moyennes() {
        let titres: Vec<String> = entrees(MA_RIBBON).into_iter().map(|s| s.title).collect();
        assert_eq!(titres[..4], ["MA #1", "MA #1 · 2", "MA #1 · 3", "MA #1 · 4"]);
        assert_eq!(titres.len(), 16);
        let f = catalogue::fiche("MaRibbon").unwrap();
        let (_, p) = catalogue::reglages(f);
        assert_eq!(p[1].label, "", "no label, like TradingView");
        assert_eq!(p[3].def, 20.0);
        assert_eq!(p[15].def, 200.0);
        let (_, sma) = catalogue::reglages(catalogue::fiche("Sma").unwrap());
        let src = sma.iter().find(|x| x.key == "Source").unwrap();
        assert!(matches!(src.kind, crate::inputs::Kind::Source), "Source: the price list");
        let defs = |id: &str| {
            catalogue::reglages(catalogue::fiche(id).unwrap())
                .1
                .iter()
                .find(|x| x.key == "Length")
                .map(|x| x.def)
        };
        assert_eq!(
            [defs("Sma"), defs("Wma"), defs("Hma"), defs("Vwma"), defs("Smma")],
            [Some(9.0), Some(9.0), Some(9.0), Some(20.0), Some(7.0)]
        );
    }

    /// Bars of a test fixture (`tests/fixtures/<nom>`).
    fn bougies(json: &str) -> Vec<argos_core::Bar> {
        let v: serde_json::Value = serde_json::from_str(json).unwrap();
        v.as_array()
            .unwrap()
            .iter()
            .map(|b| argos_core::Bar {
                time: b["time"].as_i64().unwrap(),
                open: b["open"].as_f64().unwrap(),
                high: b["high"].as_f64().unwrap(),
                low: b["low"].as_f64().unwrap(),
                close: b["close"].as_f64().unwrap(),
                volume: b["volume"].as_f64().unwrap(),
            })
            .collect()
    }

    /// 📜 Argos Vague (Jee, 20 Nov): in the catalogue (ARGOS, ƒx Oscillateurs), its header kept, its ⚙ (titles,
    /// groups, defaults) as in Jee's v8.
    #[test]
    fn argos_vague_au_catalogue_et_ses_reglages() {
        assert!(ARGOS_VAGUE.contains("BigBeluga") && ARGOS_VAGUE.contains("none of its code"), "header kept");
        let f = catalogue::fiche("ArgosVague").expect("in the catalogue");
        assert_eq!((f.nom, f.section), ("Argos Vague", "ARGOS"));
        assert_eq!(source(f), Some(ARGOS_VAGUE));
        assert!(catalogue::CATEGORIES_FX
            .iter()
            .any(|(_, n, ids)| *n == "Oscillateurs" && ids.contains(&"ArgosVague")));
        let e = entrees(ARGOS_VAGUE);
        let def = |t: &str| e.iter().find(|x| x.title == t).unwrap_or_else(|| panic!("« {t} »")).def.clone();
        let groupe = |t: &str| e.iter().find(|x| x.title == t).unwrap().group.clone();
        use crate::lang::InVal::{Bool, Color, Num, Text};
        assert_eq!(e.len(), 44, "{:?}", e.iter().map(|x| &x.title).collect::<Vec<_>>());
        assert_eq!(e[0].title, "Moteur");
        assert_eq!(def("Moteur"), Text("Classique".into()));
        assert_eq!(e[0].options, ["Classique", "Tendance adaptative", "Impulsion volatilité"]);
        assert_eq!(def("Style"), Text("Auto".into()));
        assert_eq!(e[1].options, ["Auto", "Scalpeur", "Trader du jour", "Swing", "Personnalisé"]);
        assert_eq!(
            (def("Période (Perso)"), def("Lissage (Perso)"), def("Ligne signal du nuage (Perso)")),
            (Num(10.0), Num(3.0), Num(9.0))
        );
        assert_eq!((def("Seuils adaptatifs"), def("Seuil fixe")), (Bool(true), Num(50.0)));
        assert_eq!((def("Bougies pour les seuils"), def("Écarts-types")), (Num(200.0), Num(1.5)));
        assert_eq!(def("Couleur des triangles"), Color(0xFFD600));
        assert_eq!(def("Filtre EMA"), Bool(false));
        assert_eq!(def("Couleur des bougies selon la tendance"), Bool(false));
        assert_eq!((def("Haut"), def("Bas")), (Color(0xF23645), Color(0x089981)));
        for (t, g) in [
            ("Moteur", "🌊 Vague"),
            ("Seuil fixe", "📏 Seuils"),
            ("Bandes CVD", "📊 CVD"),
            ("Nuage de tendance", "☁ Tendance"),
            ("Divergences cachées", "◆ Divergences"),
            ("Triangles de confluence forte", "▲ Signaux"),
            ("Zones d'épuisement", "◯ Épuisement"),
            ("Couleur des bougies selon la tendance", "🕯 Bougies"),
            ("Jauge de confluence", "🎨 Couleurs"),
        ] {
            assert_eq!(groupe(t), g, "{t}");
        }
    }

    /// 📜 Argos Vague's styles: Auto picks Scalpeur up to 15 min, Trader du jour up to 4H, Swing above, with the
    /// values of Jee's « Ultimate AI Trend Pro »; a chosen style wins over the unit; Personnalisé takes the (Perso)
    /// values.
    #[test]
    fn argos_vague_styles() {
        use crate::lang::InVal;
        let bars = bougies(include_str!("../tests/fixtures/btc_hl_30m.json"));
        let noms = ["len", "smooth", "signalLen", "cvdLen", "pivL", "pivR", "divMax", "bestWin"];
        let lire = |tf: i64, style: &str| {
            let m = [("Style".to_string(), InVal::Text(style.into()))].into();
            let (_, v) = crate::lang::run_avec_variables(ARGOS_VAGUE, &bars, tf, &m, &noms).unwrap();
            noms.map(|n| *v[n].last().unwrap())
        };
        let scalp = [7.0, 2.0, 5.0, 14.0, 2.0, 1.0, 60.0, 15.0];
        let jour = [10.0, 3.0, 9.0, 20.0, 2.0, 2.0, 100.0, 20.0];
        let swing = [14.0, 4.0, 12.0, 20.0, 3.0, 3.0, 150.0, 24.0];
        assert_eq!(lire(300, "Auto"), scalp, "5 min");
        assert_eq!(lire(900, "Auto"), scalp, "15 min");
        assert_eq!(lire(1800, "Auto"), jour, "30 min");
        assert_eq!(lire(14_400, "Auto"), jour, "4H");
        assert_eq!(lire(86_400, "Auto"), swing, "1D");
        assert_eq!(lire(300, "Swing"), swing, "chosen style");
        assert_eq!(lire(86_400, "Scalpeur"), scalp);
        assert_eq!(lire(300, "Trader du jour"), jour);
        assert_eq!(lire(300, "Personnalisé"), [10.0, 3.0, 9.0, 20.0, 2.0, 2.0, 100.0, 20.0], "the (Perso) defaults");
    }

    /// 📜 Argos Vague runs without an error on real bars (HYPE 50 min, BTC 30 min, XRP 5H): the wave stays in −100…100, the
    /// extreme zones are red at the top and green at the bottom, the triangles yellow #FFD600 (also on the price, by
    /// `force_overlay`), a big dot only with a triangle on the same bar; no triangle on the last bar while it is still
    /// forming.
    #[test]
    fn argos_vague_sur_vraies_bougies() {
        let mut triangles_vus = 0;
        for (nom, json, tf) in [
            ("HYPE 50m", include_str!("../tests/fixtures/hype_hl_50m.json"), 3000),
            ("BTC 30m", include_str!("../tests/fixtures/btc_hl_30m.json"), 1800),
            // (the triangles are rare: XRP 5H has two, and one best signal; BTC 30m's fixture has open = close on every
            // bar, so its estimated delta is 0 and its CVD never turns)
            ("XRP 5H", include_str!("../tests/fixtures/xrp_hl_5h.json"), 18_000),
        ] {
            let bars = bougies(json);
            let out = crate::lang::run(ARGOS_VAGUE, &bars, tf, &Default::default()).unwrap_or_else(|e| panic!("{nom}: {e}"));
            assert!(!out.overlay, "{nom}: its own band");
            let p = |t: &str| out.plots.iter().find(|x| x.title == t).unwrap_or_else(|| panic!("{nom}: plot « {t} »"));
            let vague = &p("Vague").values;
            assert!(vague.iter().filter(|v| v.is_finite()).count() > bars.len() / 2, "{nom}: a wave");
            assert!(vague.iter().filter(|v| v.is_finite()).all(|v| (-100.0..=100.0).contains(v)), "{nom}: −100…100");
            // extreme zones: 100 → overbought in red, −100 → oversold in green
            let (haut, bas) = (&out.fills[0], &out.fills[1]);
            assert_eq!(out.plots[haut.0 .1].values[0], 100.0);
            assert_eq!(out.plots[bas.0 .1].values[0], -100.0);
            let teinte = |f: &(crate::lang::Tracer, crate::lang::Tracer, Vec<Option<u32>>)| f.2.iter().flatten().next().map(|c| c & 0xffffff);
            assert_eq!((teinte(haut), teinte(bas)), (Some(0xF23645), Some(0x089981)), "{nom}: red top, green bottom");
            for (k, v) in p("Surachat").values.iter().enumerate() {
                let o = p("Survente").values[k];
                assert!(!v.is_finite() || (*v >= 30.0 && *v <= 95.0 && (-95.0..=-30.0).contains(&o)), "{nom} bar {k}");
            }
            // triangles: yellow, in the band and on the price
            let tri: Vec<&crate::lang::Shape> = out.shapes.iter().filter(|s| s.shape.starts_with("triangle")).collect();
            assert!(tri.iter().all(|s| s.color & 0xffffff == 0xFFD600), "{nom}: #FFD600");
            for s in &tri {
                let prix = s.title.ends_with("(prix)");
                assert_eq!(s.force_overlay, prix, "{nom}: « {} »", s.title);
                if prix {
                    assert!(matches!(s.loc, crate::lang::Loc::AboveBar | crate::lang::Loc::BelowBar));
                }
            }
            triangles_vus += tri.len();
            // big dot ⇒ a triangle of the same direction on its bar
            for s in out.shapes.iter().filter(|s| s.title.starts_with("Meilleur signal")) {
                let sens = if s.title.contains("vente") { "Signal baissier" } else { "Signal haussier" };
                assert!(
                    out.shapes.iter().any(|t| t.bar == s.bar && t.title == sens),
                    "{nom}: « {} » alone at {}",
                    s.title,
                    s.bar
                );
                assert_eq!(s.color & 0xffffff, if s.title.contains("vente") { 0xF23645 } else { 0x089981 });
            }
            // the last bar still forming: no triangle there, until it closes
            if let Some(k) = tri.iter().map(|s| s.bar).max() {
                let coupe = &bars[..=k];
                let ouverte = crate::lang::RunOptions {
                    real_delta: vec![],
                    last_open: true,
                    ..Default::default()
                };
                let o = crate::lang::run_with(ARGOS_VAGUE, coupe, tf, &Default::default(), &ouverte).unwrap();
                assert!(!o.shapes.iter().any(|s| s.bar == k && s.shape.starts_with("triangle")), "{nom}: forming bar");
                let f = crate::lang::run(ARGOS_VAGUE, coupe, tf, &Default::default()).unwrap();
                assert!(f.shapes.iter().any(|s| s.bar == k && s.shape.starts_with("triangle")), "{nom}: closed bar");
            }
        }
        assert!(triangles_vus > 0, "some triangles on these bars");
    }

    /// 📜 LuxAlgo's Smart Money Concepts (Jee, 25 Nov: « en tête, identique à TradingView »): first of the ƒx SMC
    /// list, its v6 source as it is with LuxAlgo's licence and name, its ⚙ from its own inputs (LuxAlgo's defaults),
    /// and what it draws on real bars: its structure labels (BOS, CHoCH, strong / weak highs and lows) and its order
    /// blocks.
    #[test]
    fn luxalgo_smc_en_tete_de_la_liste_smc() {
        assert!(LUXALGO_SMC.starts_with("// This work is licensed under a Attribution-NonCommercial-ShareAlike 4.0 International (CC BY-NC-SA 4.0)"));
        assert!(LUXALGO_SMC.contains("// © LuxAlgo"));
        let f = catalogue::fiche("LuxAlgoSmc").expect("in the catalogue");
        assert_eq!(source(f), Some(LUXALGO_SMC));
        assert_eq!(f.famille, catalogue::Famille::Communaute);
        assert!(f.nom.ends_with("[LuxAlgo]"), "LuxAlgo named: {}", f.nom);
        let smc = catalogue::CATEGORIES_FX.iter().find(|c| c.1 == "SMC / Price action").expect("the SMC list");
        assert_eq!(smc.2.first(), Some(&"LuxAlgoSmc"), "first of the SMC list");
        assert!(catalogue::texte_recherche("LuxAlgoSmc", f.nom).contains("choch"));
        let e = entrees(LUXALGO_SMC);
        let def = |t: &str| e.iter().find(|x| x.title == t).unwrap_or_else(|| panic!("« {t} »")).def.clone();
        assert_eq!(def("Mode"), crate::lang::InVal::Text("Historical".into()));
        assert_eq!(def("Style"), crate::lang::InVal::Text("Colored".into()));
        let bars = bougies(include_str!("../tests/fixtures/hype_hl_50m.json"));
        let out = crate::lang::run(LUXALGO_SMC, &bars, 3000, &Default::default()).unwrap();
        let textes: Vec<&str> = out
            .drawings
            .iter()
            .filter(|d| d.kind == "label" && !d.deleted)
            .map(|d| d.text.as_str())
            .collect();
        for mot in ["BOS", "CHoCH", "Strong High", "Weak Low"] {
            assert!(textes.contains(&mot), "« {mot} » among {textes:?}");
        }
        assert!(out.drawings.iter().any(|d| d.kind == "box" && !d.deleted), "order blocks");
    }
}
