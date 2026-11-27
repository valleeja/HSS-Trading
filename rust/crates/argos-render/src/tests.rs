use super::*;
use crate::timefmt::time_label;

mod shown_numbers;

/// 🖥 Time-axis labels never go past the plot edge: the corner cell (below the price scale, right of the time
/// axis, where the Argos AI pill sits) stays plain, wherever the labels fall when scrolling.
#[test]
fn time_labels_stop_at_the_corner_cell() {
    let b: Vec<Bar> = (0..400)
        .map(|i| Bar {
            time: 1_700_000_000 + i * 1800,
            open: 100.0,
            high: 101.0,
            low: 99.0,
            close: 100.5,
            volume: 10.0,
        })
        .collect();
    let mut c = chart(&b, CandleStyle::Classic);
    let (pw, ph) = (c.viewport.plot_w(), c.viewport.plot_h());
    let k = crate::px::scale() as f64;
    for step in 0..60 {
        c.viewport.right_offset = 5.0 - step as f64 * 0.37;
        let pm = render(&c);
        let fond = pm.pixel(((pw + 40.0) * k) as u32, ((ph + 26.0) * k) as u32).unwrap();
        for y in ((ph + 2.0) * k) as u32..((ph + 27.0) * k) as u32 {
            for x in ((pw + 1.0) * k) as u32..((pw + 77.0) * k) as u32 {
                assert_eq!(pm.pixel(x, y).unwrap(), fond, "step {step}: something drawn in the corner cell at ({x}, {y})");
            }
        }
    }
}

#[test]
fn themes() {
    assert_eq!(Theme::named(0), Theme::default(), "Défaut = couleurs du desktop");
    assert_eq!(Theme::named(1).background, Rgb::hex("#ffffff"), "clair");
    assert_eq!(Theme::named(1).chrome, Theme::default().chrome, "barres sombres en clair");
    assert_eq!(Theme::named(2).background, Rgb::hex("#090e18"), "Argos (bleu de v1)");
    assert_eq!(Theme::named(4).text, Rgb::hex("#43c35e"), "Matrix");
    assert_eq!(Theme::named(99), Theme::default());
    assert_eq!(Theme::named(Theme::CLASSIC).background, Rgb::hex("#000000"), "Classique (noir de v1)");
    assert_eq!(theme_index(), Theme::CLASSIC, "Classique par défaut");
    assert_eq!(Theme::NAMES.len(), 10);
    assert!(Theme::is_glass(8) && Theme::is_glass(9) && !Theme::is_glass(1));
    assert!(palette::is_light(9) && !palette::is_light(8), "verre clair : thème clair");
}

fn bars() -> Vec<Bar> {
    (0..300)
        .map(|i| {
            let c = 100.0 + (i as f64 / 15.0).sin() * 10.0;
            let o = if i % 2 == 0 { c - 1.0 } else { c + 1.0 };
            Bar {
                time: 1_700_000_000 + i * 1800,
                open: o,
                high: o.max(c) + 0.5,
                low: o.min(c) - 0.5,
                close: c,
                volume: if i == 290 { 500.0 } else { 10.0 },
            }
        })
        .collect()
}

#[test]
fn mouse_layer_apart() {
    let b = bars();
    let mut c = chart(&b, CandleStyle::Classic);
    c.navigation = false; // the pill follows the mouse too (it stays in the image)
    c.legende = true;
    c.crosshair = Some((120.0, 90.0));
    let (fond, r) = render_fond(&c);
    let r = r.expect("bars shown: a layer");
    // the image without the layer does not follow the mouse
    let mut ailleurs = c.clone();
    ailleurs.crosshair = Some((260.0, 40.0));
    assert_eq!(render_fond(&ailleurs).0.data(), fond.data(), "same image wherever the mouse is");
    // image + finir (the layer, then what goes over it) = the full drawing, exactly, wherever the mouse is
    let mut pm = fond.clone();
    finir(&mut pm, &c, Some(&r));
    assert_eq!(pm.data(), render(&c).data(), "the same image as render");
    assert_ne!(pm.data(), fond.data(), "the layer draws the crosshair");
    let mut pm = fond.clone();
    finir(&mut pm, &ailleurs, Some(&r));
    assert_eq!(pm.data(), render(&ailleurs).data(), "the kept image finished for another mouse");
    // with the navigation pill (it follows the mouse, it is drawn over the crosshair): still the same image
    let mut nav = c.clone();
    nav.navigation = true;
    let (fond, r) = render_fond(&nav);
    for m in [Some((120.0, 90.0)), Some((300.0, nav.viewport.plot_h() - 20.0)), None] {
        let mut at = nav.clone();
        at.crosshair = m;
        let mut pm = fond.clone();
        finir(&mut pm, &at, r.as_ref());
        assert_eq!(pm.data(), render(&at).data(), "pill on: {m:?}");
    }
}

fn chart(b: &[Bar], style: CandleStyle) -> Chart<'_> {
    Chart {
        bars: b,
        tf_seconds: 1800,
        viewport: Viewport {
            width: 400,
            height: 240,
            ..Viewport::default()
        },
        theme: Theme::default(),
        style,
        tz_offset_s: 0,
        title: None,
        crosshair: None,
        banner: None,
        context: Default::default(),
        drawings: vec![],
        selected: None,
        now_s: 0,
        live: None,
        hors_ligne: false,
        legende: false,
        navigation: true,
        goto: None,
        indicators: None,
        mark: None,
        whales: vec![],
        events: vec![],
        alerts: vec![],
        alertes_vue: Default::default(),
        ordres: Default::default(),
        demo: None,
        bid_ask: None,
        book_heat: None,
        book_note: None,
        murs: None,
        liq_map: None,
        pad: None,
        trades: vec![],
        explication: &[],
        garder: vec![],
        garder_pilules: vec![],
        boutons_bas: None,
        settings: ChartSettings::default(),
    }
}

/// ⚙️ Jee (9 nov. 2026) : chacun des 43 indicateurs a des réglages ; ajouté, il se dessine sur la chart ; changer
/// un réglage (le premier nombre) change le calcul ; rien ne plante.
#[test]
fn les_43_indicateurs_se_dessinent_et_se_reglent() {
    use crate::indicators::{compute, values, Ind, IndOut, Settings};
    use argos_script::inputs::Kind;
    let b: Vec<Bar> = (0..600)
        .map(|i| {
            let t = i as f64;
            let c = 100.0 + 8.0 * (t / 17.0).sin() + 4.0 * (t / 5.0).sin() + t * 0.02;
            Bar {
                time: 1_700_000_000 + i * 1800,
                open: c - 0.4,
                high: c + 1.2,
                low: c - 1.3,
                close: c + 0.3,
                volume: 1000.0 + 300.0 * (t / 3.0).cos().abs() + if i % 97 == 0 { 6000.0 } else { 0.0 },
            }
        })
        .collect();
    // comme l'app : les indicateurs actifs et leurs réglages vont avec le calcul
    let calcul = |on: &[Ind], set: &Settings| -> IndOut {
        let mut out = compute(&b, 1800, on, set, None, 0, &Default::default(), &[]);
        (out.on, out.inputs) = (on.to_vec(), set.clone());
        out
    };
    let dessin = |out: &IndOut| -> Vec<u8> {
        let mut c = chart(&b, CandleStyle::Classic);
        c.viewport = Viewport {
            width: 600,
            height: 400,
            ..Viewport::default()
        };
        c.indicators = Some(out);
        render(&c).data().to_vec()
    };
    // (carnet d'ordres, liquidations, transactions : dessinés avec les données en direct, absentes ici ; leurs
    // réglages agissent sur ce dessin-là)
    let en_direct = [Ind::Footprint, Ind::BookHeat, Ind::LiqMap, Ind::ArgosMurs];
    let vide = dessin(&calcul(&[], &Settings::new()));
    let (mut sans_dessin, mut sans_effet) = (vec![], vec![]);
    for ind in Ind::ALL {
        let params = ind.params().1;
        assert!(!params.is_empty(), "{ind:?} : aucun réglage");
        let base = calcul(&[ind], &Settings::new());
        let image = dessin(&base);
        // (Price Action : ces vagues trop régulières ne forment pas d'order block ; le calcul, lui, est fait)
        let calcule = ind == Ind::PriceAction && base.pa.is_some();
        if image == vide && !en_direct.contains(&ind) && !calcule {
            sans_dessin.push(ind);
        }
        // au moins un réglage change le calcul ou le dessin
        let change = params.iter().enumerate().any(|(k, p)| {
            let mut v = values(&Settings::new(), ind);
            v[k] = match p.kind {
                Kind::Num(min, max, step) => {
                    let d = p.def;
                    if d + 5.0 * step <= max {
                        d + 5.0 * step
                    } else {
                        (d - 5.0 * step).max(min)
                    }
                }
                Kind::Bool => 1.0 - p.def,
                Kind::Color => (p.def as u32 ^ 0x00ff_00ff) as f64,
                _ => return false,
            };
            let mut set = Settings::new();
            set.insert(ind, v);
            let o = calcul(&[ind], &set);
            format!("{:?}", (&o.plots, &o.cipher, &o.ai, &o.tbo, &o.smc, &o.smc_pro, &o.swing, &o.smart, &o.tvd))
                != format!(
                    "{:?}",
                    (
                        &base.plots,
                        &base.cipher,
                        &base.ai,
                        &base.tbo,
                        &base.smc,
                        &base.smc_pro,
                        &base.swing,
                        &base.smart,
                        &base.tvd
                    )
                )
                || format!("{:?}", (&o.profile, &o.footprint, &o.bubbles, &o.tape, &o.pa, &o.pat, &o.vz, &o.mtf, &o.avwap))
                    != format!(
                        "{:?}",
                        (
                            &base.profile,
                            &base.footprint,
                            &base.bubbles,
                            &base.tape,
                            &base.pa,
                            &base.pat,
                            &base.vz,
                            &base.mtf,
                            &base.avwap
                        )
                    )
                || dessin(&o) != image
        });
        if !change && !matches!(ind, Ind::BookHeat | Ind::LiqMap | Ind::ArgosMurs) {
            sans_effet.push(ind);
        }
    }
    assert!(
        sans_dessin.is_empty() && sans_effet.is_empty(),
        "rien dessiné : {sans_dessin:?} ; aucun réglage n'a d'effet : {sans_effet:?}"
    );
}

fn rgb(pm: &Pixmap, x: u32, y: u32) -> (u8, u8, u8) {
    let p = pm.pixel(x, y).unwrap();
    (p.red(), p.green(), p.blue())
}

/// 🔭 Échelle de temps très dézoomée : mois du calendrier, puis années.
#[test]
fn etiquettes_mois_et_annees() {
    let t = |y: i32, m: u32, d: u32| {
        chrono::NaiveDate::from_ymd_opt(y, m, d)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp()
    };
    assert_eq!(time_bucket(t(2025, 3, 15), viewport::MONTH).1, "mars");
    assert_eq!(time_bucket(t(2025, 1, 2), viewport::MONTH).1, "2025", "janvier : l'année");
    assert_eq!(
        time_bucket(t(2025, 3, 31), viewport::MONTH).0,
        time_bucket(t(2025, 3, 1), viewport::MONTH).0,
        "même mois"
    );
    assert_ne!(time_bucket(t(2025, 3, 31), viewport::MONTH).0, time_bucket(t(2025, 4, 1), viewport::MONTH).0);
    assert_eq!(time_bucket(t(2024, 7, 9), 12 * viewport::MONTH).1, "2024");
    assert_eq!(time_bucket(t(2025, 5, 1), 3 * viewport::MONTH).1, "avr", "trimestre avril-juin");
    // 1 h par bougie, 0,003 px par bougie : des années
    assert!(viewport::time_interval(3600, 0.003, 90.0) >= 12 * viewport::MONTH);
}

/// 🔭 Ligne très dense : un passage par colonne de pixel (premier, haut, bas, dernier), aucun pic perdu ; une ligne
/// peu dense reste exacte.
#[test]
fn ligne_dense_regroupee_par_pixel() {
    let pts: Vec<(f64, f64)> = (0..50_000)
        .map(|i| (i as f64 * 400.0 / 50_000.0, if i == 25_003 { -500.0 } else { (i as f64 / 300.0).sin() * 100.0 }))
        .collect();
    let d = dense_line(pts.iter().copied());
    assert!(d.len() <= 4 * 401, "{} points pour 400 colonnes", d.len());
    assert!(d.iter().any(|p| p.1 == -500.0), "le pic isolé reste");
    let few = [(0.0, 1.0), (10.0, 2.0), (20.0, 3.0)];
    assert_eq!(dense_line(few.iter().copied()), vec![(0.0, 1.0), (10.0, 2.0), (20.0, 3.0)]);
}

/// 🔭 50 000 bougies tout dézoomé : une bougie par colonne de pixel (haut et bas des bougies qui y tombent),
/// pas de trou, et l'image se fait vite.
#[test]
fn cinquante_mille_bougies_dezoomees() {
    let n = 50_000;
    let b: Vec<Bar> = (0..n)
        .map(|i| {
            let c = 1000.0 + (i as f64 / 700.0).sin() * 300.0;
            Bar {
                time: 1_600_000_000 + i as i64 * 60,
                open: c - 1.0,
                high: c + 2.0,
                low: c - 3.0,
                close: c,
                volume: 10.0,
            }
        })
        .collect();
    let style = CandleStyle::all()[0].clone(); // bougies de volume Argos (le type par défaut)
    let mut ch = chart(&b, style.clone());
    ch.navigation = false; // (the always-visible navigation pill would cover the lowest candles)
    ch.settings.labels.high_low_chart = false; // (and the high / low pills, the columns at the top and the bottom)
    let s = slots(&b, &style);
    ch.viewport.show_range(0, n - 1, &s);
    assert!(ch.viewport.bar_spacing < 0.01, "plus de cent bougies par pixel : {}", ch.viewport.bar_spacing);
    let t = std::time::Instant::now();
    let pm = render(&ch);
    let ms = t.elapsed().as_millis();
    let th = Theme::default();
    let plot = ch.viewport.plot_w() as u32;
    // chaque colonne de la zone des bougies a une bougie (verte ou rouge)
    let mut empty = 0;
    for x in 2..plot - 6 {
        let any = (0..ch.viewport.plot_h() as u32).any(|y| {
            let c = rgb(&pm, x, y);
            c == (th.bull.0, th.bull.1, th.bull.2) || c == (th.bear.0, th.bear.1, th.bear.2)
        });
        if !any {
            empty += 1;
        }
    }
    assert!(empty <= 2, "{empty} colonnes vides");
    // (mode débogage, sans optimisation : large marge ; en version publiée c'est bien plus rapide)
    assert!(ms < 3000, "image en {ms} ms");
}

#[test]
fn dessine_des_bougies_vertes_et_rouges() {
    let b = bars();
    let pm = render(&chart(&b, CandleStyle::Classic));
    assert_eq!((pm.width(), pm.height()), (400, 240));
    let th = Theme::default();
    let (mut green, mut red) = (0, 0);
    for y in 0..240 {
        for x in 0..300 {
            let c = rgb(&pm, x, y);
            if c == (th.bull.0, th.bull.1, th.bull.2) {
                green += 1;
            }
            if c == (th.bear.0, th.bear.1, th.bear.2) {
                red += 1;
            }
        }
    }
    assert!(green > 100 && red > 100, "vert {green}, rouge {red}");
    // fond dans le coin haut gauche (hors titre)
    assert_eq!(rgb(&pm, 2, 2), (th.background.0, th.background.1, th.background.2));
}

/// 🧱 Prix déplacés à la main (bougies qui sortent par le bas) : l'axe du temps reste celui d'une chart dont rien
/// ne sort par le bas (aucun pixel de bougie, d'orderflow ou de courbe dedans), en bougies classiques et de volume.
#[test]
fn rien_ne_deborde_sur_l_axe_du_temps() {
    let b = bars();
    let th = Theme::default();
    let bg = (th.background.0, th.background.1, th.background.2);
    for style in [CandleStyle::Classic, CandleStyle::Volume(Box::default())] {
        let mut c = chart(&b, style);
        let (pw, ph) = (c.viewport.plot_w() as u32, c.viewport.plot_h() as u32);
        let axe = |pm: &Pixmap| {
            (ph + 1..240)
                .flat_map(|y| (0..pw).map(move |x| (x, y)))
                .map(|(x, y)| rgb(pm, x, y))
                .collect::<Vec<_>>()
        };
        // échelle 100 → 130 : les bougies sous 100 (le creux) tombent sous la zone des prix
        c.settings.scale.manual = Some((100.0, 130.0));
        let bas = render(&c);
        let dedans = (0..ph)
            .flat_map(|y| (0..pw).map(move |x| (x, y)))
            .filter(|&(x, y)| rgb(&bas, x, y) != bg)
            .count();
        assert!(dedans > 500, "des bougies restent dans la zone des prix ({dedans})");
        // échelle 0 → 10 : toutes les bougies sont au-dessus, rien ne peut descendre dans l'axe
        c.settings.scale.manual = Some((0.0, 10.0));
        let haut = render(&c);
        assert!(axe(&bas) == axe(&haut), "pixels de bougie dans l'axe du temps ({:?})", c.style);
    }
}

#[test]
fn bougie_de_volume_gold_plus_large() {
    let b = bars();
    let s = volume_candles::Settings::default();
    let pm = render(&chart(&b, CandleStyle::Volume(Box::new(s.clone()))));
    // la bougie 290 a 50 × le volume moyen → palier GOLD : sa couleur doit apparaître
    let vc = volume_candles::compute(&b, &s);
    let gold = Rgb::hex(&vc[290].color);
    let found = (0..240).any(|y| (0..322).any(|x| rgb(&pm, x, y) == (gold.0, gold.1, gold.2)));
    assert!(found, "couleur GOLD {:?} absente", gold);
}

#[test]
fn bougie_low_vide_au_centre() {
    // 20 bougies à fort volume puis une à très faible volume, grande et bien zoomée
    let mut b: Vec<Bar> = (0..21)
        .map(|i| Bar {
            time: 1_700_000_000 + i * 1800,
            open: 100.0,
            high: 101.0,
            low: 99.0,
            close: 100.5,
            volume: 100.0,
        })
        .collect();
    b[20] = Bar {
        time: b[20].time,
        open: 99.2,
        high: 101.0,
        low: 99.0,
        close: 100.8,
        volume: 1.0,
    };
    let mut c = chart(&b, CandleStyle::Volume(Box::default()));
    c.viewport = Viewport {
        width: 400,
        height: 240,
        bar_spacing: 30.0,
        right_offset: 1.0,
        ..Viewport::default()
    };
    let pm = render(&c);
    let sl = slots(&b, &c.style);
    let scale = PriceScale::fit(&b, c.viewport.visible(&sl).unwrap().0, 20, c.viewport.plot_h());
    let (x, y) = (c.viewport.x_of(20, &sl).round() as u32 + 2, scale.y_of(100.37).round() as u32); // prix choisi entre deux lignes de grille
    let th = Theme::default();
    assert_eq!(
        rgb(&pm, x, y),
        (th.background.0, th.background.1, th.background.2),
        "le centre d'une bougie LOW doit être vide"
    );
}

#[test]
fn grosse_bougie_pousse_ses_voisines() {
    let b = bars();
    let style = CandleStyle::Volume(Box::default());
    let sl = slots(&b, &style);
    // bougie 290 : 50 × la moyenne → 1 + log2(50) ≈ 6,6 places ; ses voisines restent à 1
    assert!((sl.size(290) - (1.0 + 50f64.log2())).abs() < 1e-9);
    assert_eq!(sl.size(289), 1.0);
    let vp = chart(&b, style).viewport;
    let gap = vp.x_of(291, &sl) - vp.x_of(289, &sl);
    let expected = sl.size(289) / 2.0 + sl.size(290) + sl.size(291) / 2.0;
    assert!((gap - expected * vp.bar_spacing).abs() < 1e-9);
    // en classique, tout reste régulier
    assert_eq!(slots(&b, &CandleStyle::Classic), Slots::uniform(300));
}

#[test]
fn bougies_creuses() {
    // 3 bougies bien zoomées : 0 = départ ; 1 = monte (vide) au-dessus de la clôture d'avant → vert ;
    // 2 = baisse (pleine) mais ferme plus haut que la 1 → vert plein
    let mk = |i: i64, o: f64, c: f64| Bar {
        time: 1_700_000_000 + i * 1800,
        open: o,
        high: o.max(c) + 1.0,
        low: o.min(c) - 1.0,
        close: c,
        volume: 1.0,
    };
    let b = vec![mk(0, 100.0, 100.0), mk(1, 100.0, 104.0), mk(2, 110.0, 106.0)];
    let mut c = chart(&b, CandleStyle::Hollow);
    c.viewport.bar_spacing = 30.0;
    c.viewport.right_offset = 1.0;
    let pm = render(&c);
    let sl = slots(&b, &c.style);
    let scale = PriceScale::fit(&b, 0, 2, c.viewport.plot_h());
    let th = Theme::default();
    let bull = (th.bull.0, th.bull.1, th.bull.2);
    let at = |i: usize, dx: f64, p: f64| rgb(&pm, (c.viewport.x_of(i, &sl).round() + dx) as u32, scale.y_of(p).round() as u32);
    assert_ne!(at(1, 4.0, 102.3), bull, "bougie montante : vide au centre");
    assert_eq!(at(2, 4.0, 108.3), bull, "baissière mais au-dessus de la clôture d'avant : verte et pleine");
}

#[test]
fn heikin_ashi_echelle_en_valeurs_ha() {
    let b = bars();
    let pm_ha = render(&chart(&b, CandleStyle::HeikinAshi { real_last_price: false }));
    let pm_cl = render(&chart(&b, CandleStyle::Classic));
    assert_ne!(pm_ha.data(), pm_cl.data(), "les bougies HA doivent différer des classiques");
    let th = Theme::default();
    let has = |c: Rgb| (0..240).any(|y| (0..300).any(|x| rgb(&pm_ha, x, y) == (c.0, c.1, c.2)));
    assert!(has(th.bull) && has(th.bear));
    // vrai dernier prix : l'étiquette change de place (clôture réelle ≠ clôture HA)
    let pm_real = render(&chart(&b, CandleStyle::HeikinAshi { real_last_price: true }));
    assert_ne!(pm_real.data(), pm_ha.data());
}

#[test]
fn ligne_aire_ligne_de_base() {
    let b = bars();
    let th = Theme::default();
    let count = |pm: &Pixmap, c: (u8, u8, u8)| {
        (0..240)
            .flat_map(|y| (0..300).map(move |x| (x, y)))
            .filter(|&(x, y)| rgb(pm, x, y) == c)
            .count()
    };
    let blue = (LINE_BLUE.0, LINE_BLUE.1, LINE_BLUE.2);
    let line = render(&chart(&b, CandleStyle::Line));
    assert!(count(&line, blue) > 50, "ligne bleue");
    assert_eq!(count(&line, (th.bull.0, th.bull.1, th.bull.2)), 0, "pas de bougies");
    // l'aire remplit sous la ligne : plus de pixels hors du fond que la ligne seule
    let bg = (th.background.0, th.background.1, th.background.2);
    let area = render(&chart(&b, CandleStyle::Area));
    assert!(240 * 300 - count(&area, bg) > 240 * 300 - count(&line, bg) + 2000);
    // ligne de base : du vert au-dessus du milieu et du rouge en dessous
    let base = render(&chart(&b, CandleStyle::Baseline));
    let (g, r) = ((th.bull.0, th.bull.1, th.bull.2), (th.bear.0, th.bear.1, th.bear.2));
    let mid = (240 - 28) / 2;
    assert!((0..mid).any(|y| (0..300).any(|x| rgb(&base, x, y) == g)));
    assert!((mid + 2..212).any(|y| (0..300).any(|x| rgb(&base, x, y) == r)));
}

#[test]
fn colonnes_haut_bas_hlc() {
    let b = bars();
    let th = Theme::default();
    let bg = (th.background.0, th.background.1, th.background.2);
    // colonnes : remplies jusqu'en bas de la zone
    let col = render(&chart(&b, CandleStyle::Columns));
    let ph = 240 - 28;
    assert!((0..300).any(|x| rgb(&col, x, ph - 2) != bg), "les colonnes touchent le bas");
    // haut-bas : barres bleues
    let hl = render(&chart(&b, CandleStyle::HiLo));
    assert!((0..ph).any(|y| (0..300).any(|x| rgb(&hl, x, y) == (LINE_BLUE.0, LINE_BLUE.1, LINE_BLUE.2))));
    // HLC : pas de trait d'ouverture → moins de pixels colorés que les barres OHLC
    let count = |pm: &Pixmap| {
        (0..ph)
            .flat_map(|y| (0..300).map(move |x| (x, y)))
            .filter(|&(x, y)| rgb(pm, x, y) != bg)
            .count()
    };
    let mut c = chart(&b, CandleStyle::HlcBars);
    c.viewport.bar_spacing = 20.0;
    let hlc = render(&c);
    c.style = CandleStyle::Bars;
    let ohlc = render(&c);
    assert!(count(&hlc) < count(&ohlc));
}

#[test]
fn variantes_de_ligne() {
    let b = bars();
    let th = Theme::default();
    let bg = (th.background.0, th.background.1, th.background.2);
    let count = |pm: &Pixmap, f: &dyn Fn((u8, u8, u8)) -> bool| (0..212).flat_map(|y| (0..300).map(move |x| (x, y))).filter(|&(x, y)| f(rgb(pm, x, y))).count();
    let blue = (LINE_BLUE.0, LINE_BLUE.1, LINE_BLUE.2);
    let mut c = chart(&b, CandleStyle::Line);
    c.viewport.bar_spacing = 12.0;
    let line = render(&c);
    c.style = CandleStyle::LineMarkers;
    let marks = render(&c);
    assert!(count(&marks, &|p| p == blue) > count(&line, &|p| p == blue), "les marqueurs ajoutent du bleu");
    c.style = CandleStyle::StepLine;
    let step = render(&c);
    assert_ne!(step.data(), line.data(), "l'escalier diffère de la ligne");
    c.style = CandleStyle::HlcArea;
    let hlc = render(&c);
    let gray = Rgb::hex("#868993");
    assert!(count(&hlc, &|p| p == (gray.0, gray.1, gray.2)) > 20, "ligne de clôture grise");
    assert!(count(&hlc, &|p| p != bg) > count(&line, &|p| p != bg) + 1000, "zones colorées");
}

#[test]
fn renko_dessine_des_briques() {
    let b = bars();
    let style = CandleStyle::Renko(RenkoBox::Fixed(2.0));
    let s = series(&b, &style);
    assert!(!s.is_empty() && s.len() != b.len(), "briques ≠ bougies");
    assert!(s.iter().all(|x| ((x.close - x.open).abs() - 2.0).abs() < 1e-9), "toutes de taille 2");
    assert_eq!(slots(&b, &style).len(), s.len());
    let pm = render(&chart(&b, style));
    let th = Theme::default();
    assert!((0..212).any(|y| (0..300).any(|x| rgb(&pm, x, y) == (th.bull.0, th.bull.1, th.bull.2))));
    // ATR : une taille > 0 arrondie à la précision des prix
    assert!(renko_size(&b, RenkoBox::Atr(14)) > 0.0);
}

#[test]
fn reglages_echelle_visibles() {
    let b = bars();
    let base = chart(&b, CandleStyle::Classic);
    let normal = render(&base);
    let with = |f: &dyn Fn(&mut ChartSettings)| {
        let mut c = base.clone();
        f(&mut c.settings);
        render(&c)
    };
    // chaque réglage change l'image
    for (name, pm) in [
        ("inverser", with(&|v| v.scale.invert = true)),
        ("pourcent", with(&|v| v.scale.mode = ScaleMode::Percent)),
        ("indexé", with(&|v| v.scale.mode = ScaleMode::Indexed)),
        ("log", with(&|v| v.scale.mode = ScaleMode::Log)),
        ("grille", with(&|v| v.scale.grid = false)),
        ("étiquette", with(&|v| v.scale.last_price_label = false)),
        ("ligne", with(&|v| v.scale.last_price_line = false)),
        ("manuel", with(&|v| v.scale.manual = Some((50.0, 150.0)))),
    ] {
        assert_ne!(pm.data(), normal.data(), "{name} ne change rien");
    }
    // inversée : les bougies gardent leur hauteur (pas écrasées en traits)
    let th = Theme::default();
    let colored = |pm: &Pixmap| {
        (0..212)
            .flat_map(|y| (0..300).map(move |x| (x, y)))
            .filter(|&(x, y)| {
                let p = rgb(pm, x, y);
                p == (th.bull.0, th.bull.1, th.bull.2) || p == (th.bear.0, th.bear.1, th.bear.2)
            })
            .count() as f64
    };
    // bougies à longues mèches (ouverture = clôture) : la mèche doit garder sa hauteur une fois inversée
    let wicks: Vec<Bar> = (0..30)
        .map(|i| Bar {
            time: 1_700_000_000 + i * 1800,
            open: 100.0,
            high: 110.0,
            low: 90.0,
            close: 100.0,
            volume: 1.0,
        })
        .collect();
    let mut z = chart(&wicks, CandleStyle::Classic);
    let up = colored(&render(&z));
    z.settings.scale.invert = true;
    let down = colored(&render(&z));
    assert!((down - up).abs() < up * 0.1, "bougies inversées écrasées : {down} px colorés au lieu de ~{up}");
    // inverser : la bougie la plus haute passe en bas
    let mut c = base.clone();
    let s = current_scale(&c).unwrap();
    c.settings.scale.invert = true;
    let si = current_scale(&c).unwrap();
    assert!((s.y_of(105.0) - (s.height - si.y_of(105.0))).abs() < 1e-9);
    // rapport prix / barre verrouillé : 2 × plus d'espace entre bougies = 2 × plus d'écart de prix à l'écran
    let mut c = base.clone();
    let cur = current_scale(&c);
    actions::apply_settings(actions::ChartAction::LockRatio, &mut c.settings, c.viewport.bar_spacing, cur);
    let r1 = current_scale(&c).map(|s| s.max - s.min).unwrap();
    c.viewport.bar_spacing *= 2.0;
    let r2 = current_scale(&c).map(|s| s.max - s.min).unwrap();
    assert!((r2 / r1 - 2.0).abs() < 1e-9, "verrou prix / barre : {r1} → {r2}");
    // menu ouvert : le panneau est dessiné
    let mut c = base.clone();
    c.context.show(context_menu::Source::PriceAxis, context_menu::Anchor::Gear, 100.0, 1);
    assert_ne!(render(&c).data(), normal.data());
    // 🫧 on its own layer (glass): the chart image is the plain one, the menu comes apart, same place
    assert!(menus_calque(&base).is_none(), "nothing open: no layer");
    set_menus_a_part(true);
    let sans = render(&c);
    set_menus_a_part(false);
    assert!(sans.data() != render(&c).data(), "the menu is no longer in the chart image");
    let (m, r) = menus_calque(&c).expect("the menu on its layer");
    assert!(m.width() > 50 && r.w > 50.0 && r.x >= 0.0 && r.y >= 0.0, "{r:?}");
}

#[test]
fn lignes_et_bouton_plus() {
    let b = bars();
    let base = chart(&b, CandleStyle::Classic);
    let normal = render(&base);
    let s = current_scale(&base).unwrap();
    let blue = (LINE_BLUE.0, LINE_BLUE.1, LINE_BLUE.2);
    // ligne horizontale : une rangée bleue sur toute la largeur à son prix
    let mut c = base.clone();
    c.drawings = vec![drawings::Drawing::hline(105.0)];
    let pm = render(&c);
    let y = s.y_of(105.0).round() as u32;
    assert!((5..300).all(|x| rgb(&pm, x, y) == blue), "ligne bleue à 105");
    // plus de bouton ➕ au bout du réticule (le clic droit / l'appui long font tout) ; menu ouvert dessiné
    c.crosshair = Some((100.0, 80.0));
    let with_cross = render(&c);
    let (bx, by) = context_menu::plus_center(&c.viewport, 80.0);
    let mut no_line = c.clone();
    no_line.crosshair = Some((100.0, 120.0));
    assert_eq!(
        rgb(&with_cross, bx as u32, (by - 6.0) as u32),
        rgb(&render(&no_line), bx as u32, (by - 6.0) as u32),
        "pas de ➕"
    );
    c.context.show(context_menu::Source::Plus, context_menu::Anchor::Below(80.0), 105.0, 1);
    assert_ne!(render(&c).data(), with_cross.data());
    let _ = (&base, &normal);
}

#[test]
fn horloge_et_voyant_du_direct() {
    let b = bars();
    let mut base = chart(&b, CandleStyle::Classic);
    base.title = Some("BTC · 30m · Hyperliquid".into());
    let mut c = base.clone();
    c.now_s = 1_792_965_600; // 25 oct. 2026 22:00 UTC
    c.tz_offset_s = -4 * 3600;
    let clock = render(&c);
    assert_ne!(clock.data(), render(&base).data(), "horloge dans la barre de temps");
    c.live = Some(true);
    let on = render(&c);
    let green = (0x08, 0x99, 0x81);
    assert!((10..28).any(|y| (100..300).any(|x| rgb(&on, x, y) == green)), "voyant vert à droite du titre");
    c.live = Some(false);
    assert_ne!(render(&c).data(), on.data(), "voyant de reconnexion différent");
    // 📴 hors ligne : une étiquette orange sous le titre
    let sans = render(&c);
    c.hors_ligne = true;
    let avec = render(&c);
    // orange (fond de l'étiquette, à peine mélangé au fond de la chart)
    let orange = |p: (u8, u8, u8)| p.0 > 150 && (60..130).contains(&p.1) && p.2 < 60;
    assert!((36..56).any(|y| (14..60).any(|x| orange(rgb(&avec, x, y)))), "étiquette hors ligne");
    assert!(!(36..56).any(|y| (14..60).any(|x| orange(rgb(&sans, x, y)))));
}

#[test]
fn heures_dans_le_futur() {
    let b = bars();
    let mut c = chart(&b, CandleStyle::Classic);
    c.viewport.width = 900;
    c.viewport.bar_spacing = 6.0;
    c.viewport.right_offset = 80.0; // 480 px de futur à droite de la dernière bougie
    let pm = render(&c);
    let ph = c.viewport.plot_h() as u32;
    let x_last = c.viewport.x_of(b.len() - 1, &slots(&b, &c.style)) as u32;
    let bg = rgb(&pm, 5, ph + 2);
    let future_text = (ph + 6..ph + 24).any(|y| (x_last + 40..c.viewport.plot_w() as u32).any(|x| rgb(&pm, x, y) != bg));
    assert!(future_text, "étiquettes de temps à droite de la dernière bougie");
    // réticule dans le futur : l'heure à venir s'affiche sur l'échelle de temps
    let mut h = c.clone();
    h.crosshair = Some((x_last as f64 + 200.0, 100.0));
    let label = Rgb(0x36, 0x3a, 0x45);
    let with = render(&h);
    assert_eq!(
        rgb(&with, x_last + 200, ph + 4),
        (label.0, label.1, label.2),
        "étiquette du réticule dans le futur"
    );
    // Renko : pas de futur
    let mut r = c.clone();
    r.style = CandleStyle::Renko(RenkoBox::default());
    r.crosshair = h.crosshair;
    assert_ne!(rgb(&render(&r), x_last + 200, ph + 4), (label.0, label.1, label.2));
}

#[test]
fn jour_de_la_semaine_au_reticule() {
    // 24 oct. 2026 06:00 (déjà dans le fuseau affiché) : un samedi
    let t = 1_792_800_000 + 6 * 3600;
    assert_eq!(crosshair_label(t, 1800), "sam. 24 Oct '26  06:00");
    assert_eq!(crosshair_label(t, 86_400), "sam. 24 Oct '26", "en 1 jour : la date seulement");
}

#[test]
fn menu_des_types() {
    let all = CandleStyle::all();
    assert_eq!(all.len(), 17, "15 + Footprint et TPO de LuxAlgo");
    assert!(matches!(all[0], CandleStyle::Volume(_)), "bougies de volume par défaut");
    let mut names: Vec<_> = all.iter().map(|s| s.name()).collect();
    names.dedup();
    assert_eq!(names.len(), 17, "un nom différent par type");
    // footprint et TPO se dessinent
    let b: Vec<Bar> = (0..200)
        .map(|i| {
            let c = 100.0 + (i as f64 / 7.0).sin() * 3.0;
            Bar {
                time: i * 1800,
                open: c - 0.4,
                high: c + 1.0,
                low: c - 1.0,
                close: c,
                volume: 50.0,
            }
        })
        .collect();
    for st in [CandleStyle::Footprint, CandleStyle::Tpo] {
        let mut c = chart(&b, st);
        c.tf_seconds = 1800;
        assert!(render(&c).width() > 0);
    }
}

#[test]
fn barres_ohlc() {
    assert_eq!((bar_tick(2.0), bar_tick(7.0), bar_tick(10.0), bar_tick(20.0)), (1.0, 1.0, 3.0, 5.0));
    // une barre montante seule, bien zoomée : trait vertical, ouverture à gauche, clôture à droite, rien au centre du corps
    let b: Vec<Bar> = (0..3)
        .map(|i| Bar {
            time: 1_700_000_000 + i * 1800,
            open: 100.0,
            high: 104.0,
            low: 96.0,
            close: 102.0,
            volume: 1.0,
        })
        .collect();
    let mut c = chart(&b, CandleStyle::Bars);
    c.viewport.bar_spacing = 20.0;
    c.viewport.right_offset = 1.0;
    let pm = render(&c);
    let sl = slots(&b, &c.style);
    let scale = PriceScale::fit(&b, 0, 2, c.viewport.plot_h());
    let cx = c.viewport.x_of(2, &sl).round();
    let th = Theme::default();
    let bull = (th.bull.0, th.bull.1, th.bull.2);
    let at = |x: f64, p: f64| rgb(&pm, x as u32, scale.y_of(p).round() as u32);
    assert_eq!(at(cx, 103.0), bull, "trait vertical");
    assert_eq!(at(cx - 3.0, 100.0), bull, "ouverture à gauche");
    assert_eq!(at(cx + 3.0, 102.0), bull, "clôture à droite");
    assert_ne!(at(cx + 3.0, 100.0), bull, "pas d'ouverture à droite");
    assert_ne!(at(cx - 3.0, 101.3), bull, "pas de corps plein");
}

#[test]
fn largeur_du_corps_comme_tradingview() {
    assert_eq!(candle_width(1.0), 1.0);
    assert_eq!(candle_width(3.0), 3.0);
    assert_eq!(candle_width(7.0), 5.0); // 6 px → pair, ramené à 5
    assert_eq!(candle_width(10.0), 7.0);
    assert_eq!(candle_width(40.0), 31.0); // ~ 80 % de la place quand on zoome
}

#[test]
fn reticule_dessine_sans_planter() {
    let b = bars();
    let mut c = chart(&b, CandleStyle::Classic);
    c.crosshair = Some((150.0, 100.0));
    c.banner = Some("⚠️ Source temporaire : Bybit".into());
    let with = render(&c);
    c.crosshair = None;
    let without = render(&c);
    assert_ne!(with.data(), without.data());
    // souris hors de la chart : rien de plus
    c.crosshair = Some((-5.0, 100.0));
    assert_eq!(render(&c).data(), without.data());
}

#[test]
fn etiquettes_de_temps() {
    assert_eq!(time_label(1_792_929_600 - 43_200, 43_200), "25");
    assert_eq!(time_label(1_792_929_600, 3600), "12:00");
    assert_eq!(time_label(1_790_812_800, 86_400), "oct");
}

#[test]
fn compte_a_rebours_des_bougies() {
    // mercredi 4 nov. 2026, 19:23 UTC
    let now = 1_793_820_180;
    let (week, month) = (604_800, argos_core::MONTH_SECONDS);
    let w = argos_core::bucket_start(now, week);
    assert_eq!(countdown_text(close_of(w, week) - now), "4j 04:37", "weekly : clôture lundi 00:00 UTC");
    let m = argos_core::bucket_start(now, month);
    assert_eq!(countdown_text(close_of(m, month) - now), "26j 04:37", "monthly : clôture le 1er déc. 00:00 UTC");
    let d = argos_core::bucket_start(now, 86_400);
    assert_eq!(countdown_text(close_of(d, 86_400) - now), "04:37:00", "daily : minuit UTC");
    assert_eq!(
        countdown_text(close_of(argos_core::bucket_start(now, 14_400), 14_400) - now),
        "37:00",
        "4h : 20:00 UTC"
    );
    assert_eq!(countdown_text(3_599), "59:59");
}

#[test]
fn format_de_l_etiquette_du_prix() {
    // mercredi 4 nov. 2026, 20:10 UTC
    let now = 1_793_823_000;
    let at = |tf: i64| countdown_text(close_of(argos_core::bucket_start(now, tf), tf) - now);
    assert_eq!(at(14_400), "03:50:00", "4h : HH:MM:SS sur 2 chiffres");
    assert_eq!(at(900), "05:00", "15 min : MM:SS");
    assert_eq!(at(604_800), "4j 03:50", "W : Xj HH:MM");
    assert_eq!(
        viewport::format_price(80_131.0, 1),
        "80\u{202f}131,0",
        "espace fine des milliers, virgule décimale"
    );
}

/// Luminance relative (WCAG) d'une couleur.
fn lum(c: Rgb) -> f64 {
    let f = |v: u8| {
        let v = v as f64 / 255.0;
        if v <= 0.03928 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * f(c.0) + 0.7152 * f(c.1) + 0.0722 * f(c.2)
}

/// Vision simulée (matrices de Machado, Oliveira et Fernandes 2009, sévérité 100 %) en RGB linéaire, rendue en sRGB 0-255.
fn cvd(c: Rgb, m: [[f64; 3]; 3]) -> [f64; 3] {
    let lin = |v: u8| {
        let v = v as f64 / 255.0;
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    let x = [lin(c.0), lin(c.1), lin(c.2)];
    let back = |v: f64| {
        let v = v.clamp(0.0, 1.0);
        255.0 * if v <= 0.0031308 { v * 12.92 } else { 1.055 * v.powf(1.0 / 2.4) - 0.055 }
    };
    [0, 1, 2].map(|r| back(m[r][0] * x[0] + m[r][1] * x[1] + m[r][2] * x[2]))
}

#[test]
fn themes_lisibles_et_hausse_baisse_distinctes() {
    for k in 0..Theme::NAMES.len() {
        let t = Theme::named(k);
        assert_ne!(t.bull, t.bear, "{} : hausse et baisse différentes", Theme::NAMES[k]);
        let (a, b) = (lum(t.text) + 0.05, lum(t.background) + 0.05);
        let ratio = a.max(b) / a.min(b);
        assert!(ratio >= 4.5, "{} : contraste du texte {ratio:.2}:1 (au moins 4,5:1)", Theme::NAMES[k]);
    }
    assert_eq!(Theme::NAMES[1], "Clair");
    assert!(lum(Theme::named(1).background) > 0.9, "Clair : fond blanc");
}

#[test]
fn themes_daltoniens_restent_distincts() {
    const PROTAN: [[f64; 3]; 3] = [
        [0.152286, 1.052583, -0.204868],
        [0.114503, 0.786281, 0.099216],
        [-0.003882, -0.048116, 1.051998],
    ];
    const DEUTAN: [[f64; 3]; 3] = [[0.367322, 0.860646, -0.227968], [0.280085, 0.672501, 0.047413], [-0.011820, 0.042940, 0.968881]];
    const TRITAN: [[f64; 3]; 3] = [
        [1.255528, -0.076749, -0.178779],
        [-0.078411, 0.930809, 0.147602],
        [0.004733, 0.691367, 0.303900],
    ];
    let dist = |a: [f64; 3], b: [f64; 3]| ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt();
    let gap = |k: usize, m| {
        let t = Theme::named(k);
        dist(cvd(t.bull, m), cvd(t.bear, m))
    };
    // rouge-vert (sombre) et blanc / orange / bleu : hausse / baisse distinctes pour les trois daltonismes
    for k in [6, 7] {
        for (m, who) in [(PROTAN, "protanopie"), (DEUTAN, "deutéranopie"), (TRITAN, "tritanopie")] {
            assert!(
                gap(k, m) > 100.0,
                "{} / {who} : hausse et baisse trop proches ({:.0})",
                Theme::NAMES[k],
                gap(k, m)
            );
        }
    }
    // le vert / rouge par défaut se confond bien plus pour un deutéranope : c'est pour ça qu'il y a ces thèmes
    assert!(gap(0, DEUTAN) < gap(6, DEUTAN));
    // les deux thèmes daltoniens ne se ressemblent pas : fond sombre contre fond blanc
    let (a, b) = (Theme::named(6), Theme::named(7));
    assert!(palette::contrast(a.background, b.background) > 10.0);
    // et pas les mêmes bougies : rouge-vert (vert bleuté / vermillon) contre bleu / orange
    assert!(a.bull.0 < 0x80 && a.bull.1 > 0xb0, "hausse verte en rouge-vert");
    assert!(b.bull.2 > b.bull.0 && b.bull.2 > b.bull.1, "hausse bleue en blanc / orange / bleu");
    assert_ne!(a.bull, b.bull);
    // bougies du daltonien clair lisibles sur le blanc (graphique : 3:1)
    assert!(palette::contrast(b.bull, b.background) >= 3.0 && palette::contrast(b.bear, b.background) >= 3.0);
    // icônes par catégorie des thèmes daltoniens : deux à deux distinctes, même en simulation
    for (k, m) in [(6, DEUTAN), (6, PROTAN), (7, DEUTAN)] {
        set_theme(k);
        let c: Vec<[f64; 3]> = [
            palette::IconCat::Draw,
            palette::IconCat::Analyse,
            palette::IconCat::Trading,
            palette::IconCat::Market,
            palette::IconCat::Info,
            palette::IconCat::Assistant,
            palette::IconCat::Settings,
        ]
        .map(|cat| cvd(palette::icon_color(cat), m))
        .to_vec();
        for i in 0..c.len() {
            for j in i + 1..c.len() {
                assert!(dist(c[i], c[j]) > 20.0, "thème {k} : icônes {i} et {j} trop proches");
            }
        }
    }
    set_theme(Theme::CLASSIC);
}

#[test]
fn contrastes_de_tous_les_themes() {
    for k in 0..Theme::NAMES.len() {
        for (what, fg, bg, min) in palette::pairs(k) {
            let c = palette::contrast(fg, bg);
            assert!(c >= min, "{} : {what} {c:.2}:1 (au moins {min}:1) {fg:?} sur {bg:?}", Theme::NAMES[k]);
        }
    }
}

/// 🏷 Jee, 19 Nov: the alert's price was hidden under the last price's label. On the scale no label covers
/// another nor the last price's label (which keeps its place); a label touching nothing does not move.
#[test]
fn scale_labels_never_cover_each_other() {
    let tag = |y: f64, h: f64| Tag {
        y,
        h,
        rayon: 4.0,
        col: Rgb(0, 0, 0),
        alpha: 1.0,
        texte: String::new(),
        taille: 11.5,
        encre: Rgb(255, 255, 255),
    };
    // the last price at 300 with its countdown (289 → 328), an alert at 302, the previous close at 296, bid & ask
    // around it, a high far above
    let tags = [tag(302.0, 18.0), tag(296.0, 20.0), tag(299.0, 16.0), tag(312.0, 16.0), tag(80.0, 20.0)];
    let band = (289.0, 328.0);
    let ys = ecarter(&tags, Some(band));
    let spans: Vec<(f64, f64)> = ys.iter().zip(&tags).map(|(y, t)| (y - t.h / 2.0, y + t.h / 2.0)).collect();
    for (i, a) in spans.iter().enumerate() {
        assert!(a.1 <= band.0 || a.0 >= band.1, "label {a:?} under the last price's label");
        for b in &spans[i + 1..] {
            assert!(a.1 <= b.0 || b.1 <= a.0, "labels {a:?} and {b:?} overlap");
        }
    }
    assert_eq!(ys[4], 80.0, "far from the others: exact place");
    // no last-price label: labels still never overlap
    let ys = ecarter(&tags, None);
    let mut spans: Vec<(f64, f64)> = ys.iter().zip(&tags).map(|(y, t)| (y - t.h / 2.0, y + t.h / 2.0)).collect();
    spans.sort_by(|a, b| a.0.total_cmp(&b.0));
    assert!(spans.windows(2).all(|w| w[0].1 <= w[1].0));
    // Jee's capture, zoomed out (scale 90 000 at 89 to 50 000 at 380): the last price 63 174 with its countdown,
    // the alert at 60 773 was under its label; now just below it, whole
    let y_of = |p: f64| (89.0 + (90_000.0 - p) * 291.0 / 40_000.0).round();
    let last = y_of(63_174.0);
    let band = (last - 11.0, last - 9.0 + 37.0);
    let alerte = tag(y_of(60_773.0), 18.0);
    assert!(alerte.y > band.0 && alerte.y < band.1, "the case: the alert inside the last price's label");
    let y = ecarter(&[alerte], Some(band))[0];
    assert!(
        y - 9.0 >= band.1 && y - 9.0 <= band.1 + 2.0,
        "the alert glued just below the last price: {y} {band:?}"
    );
}

/// 📱 Jee (21 Nov, list 2): by the finger, a small chart (the watchlist open on the phone) shows no O H L C · Vol ·
/// Acheteurs / Vendeurs lines over its candles; a tall one does, and the PC always does.
#[test]
fn legende_cachee_sur_petite_chart_au_doigt() {
    let b: Vec<Bar> = (0..50)
        .map(|i| Bar {
            time: 1_700_000_000 + i * 1800,
            open: 100.0,
            high: 101.0,
            low: 99.0,
            close: 100.5,
            volume: 10.0,
        })
        .collect();
    let mut c = chart(&b, CandleStyle::Classic);
    c.legende = true;
    let ecrit = |c: &Chart, tactile: bool| {
        crate::actions::set_tactile(tactile);
        let _ = crate::text::ecrits();
        let _ = render(c);
        crate::text::ecrits().iter().any(|s| s.starts_with("Vol ") || s.starts_with("O "))
    };
    assert!(c.viewport.plot_h() < LEGENDE_MIN_H);
    assert!(ecrit(&c, false), "PC: the legend");
    assert!(!ecrit(&c, true), "finger, small chart: none");
    c.viewport.height = 700;
    assert!(ecrit(&c, true), "finger, tall chart: the legend");
    crate::actions::set_tactile(false);
}

/// 📱 Jee (21 Nov, list 2): by the finger the clock is not written on the time axis (under ƒx and the round Argos AI
/// button); the PC keeps it.
#[test]
fn horloge_pas_sous_les_boutons_au_doigt() {
    let b: Vec<Bar> = (0..50)
        .map(|i| Bar {
            time: 1_700_000_000 + i * 1800,
            open: 100.0,
            high: 101.0,
            low: 99.0,
            close: 100.5,
            volume: 10.0,
        })
        .collect();
    let mut c = chart(&b, CandleStyle::Classic);
    c.now_s = 1_700_090_000;
    let horloge = |c: &Chart, tactile: bool| {
        crate::actions::set_tactile(tactile);
        let _ = crate::text::ecrits();
        let _ = render(c);
        crate::text::ecrits().iter().any(|s| s.contains("UTC"))
    };
    assert!(horloge(&c, false), "PC: the clock");
    assert!(!horloge(&c, true), "finger: none");
    crate::actions::set_tactile(false);
}

/// 📋 Suivi 10 (Jee, 21 Nov): the backtest's summary — the chart's banner — lay over the Argos TVD Pro table (top
/// right; the Backtest panel makes the chart narrower). The banner never covers an indicator's table: it goes under
/// it, at the same place across.
#[test]
fn bandeau_jamais_sur_un_tableau() {
    let vp = Viewport {
        width: 700,
        height: 500,
        ..Viewport::default()
    };
    let text = Text::default();
    let long = "🧪 Backtest Double Structure : 48 trades, 52 % gagnants, +12,4 R, drawdown max 6,1 %";
    let libre = banner_rect(&vp, &text, long);
    // two tables at the top right, one under the other, the first under the banner's place
    let (t1, t2) = (
        shapes::Rect::new(libre.x + libre.w - 120.0, 30.0, 200.0, 120.0),
        shapes::Rect::new(libre.x + libre.w - 120.0, 154.0, 200.0, 60.0),
    );
    assert!(indicators::poser_tableau(t2) && indicators::poser_tableau(t1));
    let r = banner_rect(&vp, &text, long);
    let croise = |t: &shapes::Rect| t.x < r.x + r.w && r.x < t.x + t.w && t.y < r.y + r.h && r.y < t.y + t.h;
    assert!(!croise(&t1) && !croise(&t2), "{r:?} over a table");
    assert_eq!((r.x, r.w, r.y), (libre.x, libre.w, t2.y + t2.h + 6.0), "under both, same place across");
}

/// 🧱 Argos Murs lot 3e (Jee 22 Nov): the mouse on a wall pill: the crosshair lines never cross the pill (its pixels
/// are the same with the mouse on it as with no mouse), and the tooltip is drawn beside it.
#[test]
fn pilule_survolee_sans_ligne_du_reticule() {
    // at the PC's scale and at a phone's (the pixmap has 2 pixels per point there)
    for echelle in [1.0, 2.0] {
        crate::px::set_scale(echelle);
        pilule_survolee_a_l_echelle(echelle as f64);
    }
    crate::px::set_scale(1.0);
}

fn pilule_survolee_a_l_echelle(s: f64) {
    use crate::indicators::{compute, Ind, IndOut, Settings};
    let b: Vec<Bar> = (0..300)
        .map(|i| {
            let t = i as f64;
            let c = 100.0 + 2.0 * (t / 17.0).sin();
            Bar {
                time: 1_700_000_000 + i * 1800,
                open: c - 0.2,
                high: c + 0.5,
                low: c - 0.5,
                close: c + 0.1,
                volume: 1000.0,
            }
        })
        .collect();
    let mut out: IndOut = compute(&b, 1800, &[Ind::ArgosMurs], &Settings::new(), None, 0, &Default::default(), &[]);
    (out.on, out.inputs) = (vec![Ind::ArgosMurs], Settings::new());
    let last = b.last().unwrap().close;
    let mut e = murs::Ecran {
        step: 0.1,
        last,
        coin: "BTC".into(),
        range: 3.0,
        perp: true,
        spot: false,
        ..Default::default()
    };
    e.vue.pills = vec![argos_core::murs::Pill {
        price: last + 1.0,
        usd: 9e6,
        sell: true,
        spot: false,
        since_ms: 0,
        venues: 3,
        tag: false,
        par: [0.0; argos_core::murs::PAR_BOOKS],
    }];
    let mut c = chart(&b, CandleStyle::Classic);
    c.viewport = Viewport {
        width: 900,
        height: 600,
        ..Viewport::default()
    };
    c.indicators = Some(&out);
    c.murs = Some(e);
    let (fond, layer) = render_fond(&c);
    let layer = layer.expect("mouse layer");
    let r = layer.murs.as_ref().expect("Argos Murs kept for the mouse").pilules[0].r;
    let sans = {
        let mut pm = fond.clone();
        c.crosshair = None;
        finir(&mut pm, &c, Some(&layer));
        pm
    };
    c.crosshair = Some((r.cx(), r.cy()));
    let mut avec = fond.clone();
    finir(&mut avec, &c, Some(&layer));
    let dedans = |pm: &Pixmap| -> Vec<u8> {
        ((r.y * s).ceil() as u32..((r.y + r.h) * s).floor() as u32)
            .flat_map(|y| ((r.x * s).ceil() as u32..((r.x + r.w) * s).floor() as u32).map(move |x| (x, y)))
            .flat_map(|(x, y)| {
                let p = pm.pixel(x, y).unwrap();
                [p.red(), p.green(), p.blue(), p.alpha()]
            })
            .collect()
    };
    assert_eq!(dedans(&avec), dedans(&sans), "no crosshair line through the hovered pill");
    // the crosshair is drawn elsewhere, and the tooltip at the left of the pill
    let gauche = |pm: &Pixmap| (0..((r.x * s) as u32).saturating_sub(10)).any(|x| pm.pixel(x, (r.cy() * s) as u32) != sans.pixel(x, (r.cy() * s) as u32));
    assert!(gauche(&avec), "crosshair and tooltip left of the pill");
}

/// 🧱 Lot loin-1: with a pane open under the chart (Argos Vague), the far buys sit at the bottom of the PRICE pane's
/// free band: nothing of them is drawn inside the pane (PC and phone).
#[test]
fn murs_lointains_dans_le_panneau_des_prix() {
    use crate::indicators::{compute, Ind, Settings};
    let b: Vec<Bar> = (0..300)
        .map(|i| {
            let t = i as f64;
            // (the last price in the middle of the pane: room above and below it)
            let c = 100.0 + 8.0 * ((t - 299.0) / 17.0).sin();
            Bar {
                time: 1_700_000_000 + i * 1800,
                open: c - 0.4,
                high: c + 1.2,
                low: c - 1.3,
                close: c + 0.3,
                volume: 1000.0,
            }
        })
        .collect();
    let on = [Ind::ArgosMurs, Ind::ArgosVague];
    let mut out = compute(&b, 1800, &on, &Settings::new(), None, 0, &Default::default(), &[]);
    (out.on, out.inputs) = (on.to_vec(), Settings::new());
    let lointain = |price: f64, sell: bool| argos_core::murs::Lointain {
        price,
        usd: 20e6,
        coins: 20e6 / price,
        sell,
        since_ms: 0,
    };
    let e = murs::Ecran {
        step: 0.1,
        coin: "BTC".into(),
        range: 3.0,
        perp: true,
        spot: false,
        last: b.last().unwrap().close,
        ..Default::default()
    };
    for (w, h) in [(1100u32, 760u32), (390, 700)] {
        let dessin = |loin: bool| -> Pixmap {
            let mut c = chart(&b, CandleStyle::Classic);
            c.viewport = Viewport {
                width: w,
                height: h,
                ..Viewport::default()
            };
            c.indicators = Some(&out);
            let mut e = e.clone();
            if loin {
                e.loin = vec![lointain(160.0, true), lointain(150.0, true), lointain(40.0, false), lointain(30.0, false)];
            }
            c.murs = Some(e);
            render(&c)
        };
        let vp = Viewport {
            width: w,
            height: h,
            ..Viewport::default()
        };
        let lignes = |pm: &Pixmap, y0: f64, y1: f64| -> Vec<u8> {
            let row = pm.width() as usize * 4;
            let (a, z) = (y0.ceil() as usize, (y1.floor() as usize).min(pm.height() as usize));
            pm.data()[a * row..z * row].to_vec()
        };
        let (avec, sans) = (dessin(true), dessin(false));
        let bas_prix = vp.plot_h() - out.osc_h();
        assert!(
            lignes(&avec, bas_prix + 1.0, vp.plot_h()) == lignes(&sans, bas_prix + 1.0, vp.plot_h()),
            "{w}: over the pane"
        );
        let pill = murs::tailles(vp.plot_w(), crate::actions::tactile()).pill_h;
        let bas = bas_prix - murs::bas_hauteur(vp.plot_w()) - pill / 2.0 - murs::GAP;
        assert!(
            lignes(&avec, bas - 2.0 * pill - murs::GAP - 1.0, bas + 1.0) != lignes(&sans, bas - 2.0 * pill - murs::GAP - 1.0, bas + 1.0),
            "{w}: the far buys just above the price pane's bottom pills"
        );
    }
}

/// 🧱 Argos Murs (Argos Vague, 21 Nov): the bottom pills (live books summary, Perp / Comptant key) sit at the bottom
/// of the PRICE pane: nothing of Argos Murs is drawn inside a lower panel; without a panel they are still there.
#[test]
fn pilules_argos_murs_au_bas_du_prix_jamais_sur_un_panneau() {
    use crate::indicators::{compute, Ind, IndOut, Settings};
    let b: Vec<Bar> = (0..300)
        .map(|i| {
            let t = i as f64;
            let c = 100.0 + 8.0 * (t / 17.0).sin() + 4.0 * (t / 5.0).sin();
            Bar {
                time: 1_700_000_000 + i * 1800,
                open: c - 0.4,
                high: c + 1.2,
                low: c - 1.3,
                close: c + 0.3,
                volume: 1000.0,
            }
        })
        .collect();
    let calcul = |on: &[Ind]| -> IndOut {
        let mut out = compute(&b, 1800, on, &Settings::new(), None, 0, &Default::default(), &[]);
        (out.on, out.inputs) = (on.to_vec(), Settings::new());
        out
    };
    let e = murs::Ecran {
        step: 0.1,
        totals: Some((4_346.0, 4_272.0)),
        coin: "BTC".into(),
        range: 3.0,
        spot_names: "Binance · Bybit".into(),
        perp: true,
        spot: true,
        ..Default::default()
    };
    for (w, h) in [(1100u32, 760u32), (390, 700)] {
        let dessin = |out: &IndOut, murs: bool| -> Pixmap {
            let mut c = chart(&b, CandleStyle::Classic);
            c.viewport = Viewport {
                width: w,
                height: h,
                ..Viewport::default()
            };
            c.indicators = Some(out);
            c.murs = murs.then(|| e.clone());
            render(&c)
        };
        let vp = Viewport {
            width: w,
            height: h,
            ..Viewport::default()
        };
        let lignes = |pm: &Pixmap, y0: f64, y1: f64| -> Vec<u8> {
            let row = pm.width() as usize * 4;
            let (a, z) = (y0.ceil() as usize, (y1.floor() as usize).min(pm.height() as usize));
            pm.data()[a * row..z * row].to_vec()
        };
        // with the Argos Vague panel: its rows are the same with or without Argos Murs
        let out = calcul(&[Ind::ArgosMurs, Ind::ArgosVague]);
        let (avec, sans) = (dessin(&out, true), dessin(&out, false));
        let ph = vp.plot_h();
        let bas_prix = ph - out.osc_h();
        assert!(bas_prix < ph - 100.0, "{w}: a real panel under the price ({bas_prix} / {ph})");
        assert!(
            lignes(&avec, bas_prix + 1.0, ph) == lignes(&sans, bas_prix + 1.0, ph),
            "{w}: Argos Murs drew over the panel"
        );
        assert!(
            lignes(&avec, bas_prix - murs::bas_hauteur(vp.plot_w()) - 2.0, bas_prix)
                != lignes(&sans, bas_prix - murs::bas_hauteur(vp.plot_w()) - 2.0, bas_prix),
            "{w}: the pills are just above the panel"
        );
        // no lower panel: the pills at the bottom of the plot, as before
        let seul = calcul(&[Ind::ArgosMurs]);
        assert_eq!(seul.osc_h(), 0.0);
        let (avec, sans) = (dessin(&seul, true), dessin(&seul, false));
        let bas = ph - murs::bas_hauteur(vp.plot_w()) - 2.0;
        assert!(
            lignes(&avec, bas, ph) != lignes(&sans, bas, ph),
            "{w}: no panel: the pills at the bottom of the plot"
        );
    }
}
