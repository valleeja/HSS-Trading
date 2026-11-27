//! 🖼 Images de référence (demande de Jee, 16 nov. : « à chaque fois que tu fais de quoi tu changes des affaires
//! ailleurs »). Des zones fixes de l'interface sont dessinées avec des données fixes et comparées au pixel près à leur
//! image gardée dans `tests/references/`. Une zone qui change sans qu'on l'ait demandé fait échouer `make vert`.
//!
//! Changement voulu d'une zone : relancer avec `ARGOS_REFERENCES=maj cargo test -p argos-render --test references`,
//! regarder les nouvelles images (git diff les montre), puis les commiter avec le changement. Une image qui diffère est
//! aussi écrite à côté (`<nom>.nouveau.png`) pour la comparer à l'œil.
//!
//! (Binaire de test à part : son propre processus, les réglages globaux — thème, verre — ne bougent pas pendant le
//! dessin, contrairement aux tests du crate qui tournent en parallèle.)

use argos_core::Bar;
use argos_render::econ::{Mark, NEWS};
use argos_render::shapes::Rect;
use argos_render::text::Text;
use argos_render::viewport::Viewport;
use argos_render::{CandleStyle, Chart, Theme};
use std::path::PathBuf;
use tiny_skia::Pixmap;

fn dossier() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("references")
}

/// Compare `pm` à la référence `nom` ; `Some(raison)` si elle diffère.
fn comparer(nom: &str, pm: &Pixmap) -> Option<String> {
    let d = dossier();
    std::fs::create_dir_all(&d).ok()?;
    let chemin = d.join(format!("{nom}.png"));
    let maj = std::env::var("ARGOS_REFERENCES").is_ok_and(|v| v == "maj");
    if maj || !chemin.exists() {
        pm.save_png(&chemin).expect("écrire la référence");
        eprintln!("🖼 référence écrite : {}", chemin.display());
        return None;
    }
    let reference = Pixmap::load_png(&chemin).expect("lire la référence");
    let raison = if (reference.width(), reference.height()) != (pm.width(), pm.height()) {
        Some(format!(
            "{nom} : taille {}×{} au lieu de {}×{}",
            pm.width(),
            pm.height(),
            reference.width(),
            reference.height()
        ))
    } else {
        let differents = reference
            .pixels()
            .iter()
            .zip(pm.pixels())
            .filter(|(a, b)| {
                let e = |x: u8, y: u8| x.abs_diff(y) > 12;
                e(a.red(), b.red()) || e(a.green(), b.green()) || e(a.blue(), b.blue()) || e(a.alpha(), b.alpha())
            })
            .count();
        (differents > 0).then(|| format!("{nom} : {differents} pixels ont changé"))
    };
    if raison.is_some() {
        let _ = pm.save_png(d.join(format!("{nom}.nouveau.png")));
    }
    raison
}

fn barres() -> Vec<Bar> {
    (0..300)
        .map(|i| {
            let t = i as f64;
            let o = 100.0 + (t / 9.0).sin() * 8.0 + t * 0.03;
            let c = o + (t / 3.0).cos() * 1.6;
            Bar {
                time: 1_700_000_000 + i * 1800,
                open: o,
                high: o.max(c) + 0.8,
                low: o.min(c) - 0.8,
                close: c,
                volume: 10.0 + (t / 5.0).sin().abs() * 20.0,
            }
        })
        .collect()
}

fn marque(ts: i64, c: &str, impact: u8) -> Mark {
    Mark {
        ts,
        currency: c.into(),
        impact,
        title: "Annonce".into(),
        ..Default::default()
    }
}

fn chart<'a>(b: &'a [Bar], events: Vec<Mark>) -> Chart<'a> {
    Chart {
        bars: b,
        tf_seconds: 1800,
        viewport: Viewport {
            width: 900,
            height: 480,
            ..Viewport::default()
        },
        theme: Theme::default(),
        style: CandleStyle::Classic,
        tz_offset_s: 0,
        title: Some("BTC-USDC Perpétuel · 30m · Hyperliquid".into()),
        crosshair: None,
        banner: None,
        context: Default::default(),
        drawings: vec![],
        selected: None,
        now_s: 0,
        live: None,
        hors_ligne: false,
        legende: true,
        navigation: true,
        goto: None,
        indicators: None,
        mark: None,
        whales: vec![],
        events,
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
        settings: Default::default(),
    }
}

/// 🖼 Les zones gardées : la chart (bougies, légende, échelles, rangée d'annonces avec drapeaux) ; la rangée
/// d'annonces avec 60 nouvelles (les drapeaux restent) ; le menu des alertes ; la fenêtre 🔔 du PC.
#[test]
fn images_de_reference() {
    let mut echecs = vec![];
    let b = barres();
    let t0 = b[0].time;
    let agenda = vec![
        marque(t0 + 220 * 1800, "USD", 2),
        marque(t0 + 240 * 1800, "CAD", 2),
        marque(t0 + 260 * 1800, "EUR", 1),
        marque(t0 + 280 * 1800, "GBP", 2),
    ];
    // 1) la chart avec l'agenda
    let pm = argos_render::render(&chart(&b, agenda.clone()));
    echecs.extend(comparer("chart-agenda", &pm));
    // 2) la même avec 60 nouvelles : les drapeaux restent à leur place
    let mut avec_news = agenda.clone();
    avec_news.extend((0..60).map(|k| Mark {
        ts: t0 + (200 + k) * 1800,
        currency: NEWS.into(),
        title: "Nouvelle".into(),
        ..Default::default()
    }));
    let pm = argos_render::render(&chart(&b, avec_news));
    echecs.extend(comparer("chart-agenda-et-news", &pm));
    // 2b) the navigation pill (always visible), the mouse over it: its hover patch and bubble
    let mut c = chart(&b, vec![]);
    c.crosshair = Some(argos_render::navbar::centers(&c.viewport, c.viewport.plot_h(), false)[1]);
    let pm = argos_render::render(&c);
    echecs.extend(comparer("chart-barre-du-bas", &pm));
    // 3) le menu des alertes (choix en pilules, sans traits)
    {
        use argos_render::listmenu::{ListMenu, MenuGroup, MenuItem};
        let it = |id: i64, l: &str, on: bool| MenuItem {
            id,
            label: l.into(),
            enabled: true,
            checked: on,
            ..Default::default()
        };
        let groups = vec![
            MenuGroup {
                title: "NOUVELLE ALERTE".into(),
                items: vec![
                    it(1, "Alerte de prix à 65 000,00 (BTC)", false),
                    it(2, "RSI 14 sous 30 (survente) · BTC 30m", false),
                ],
                strip: false,
            },
            MenuGroup {
                title: "DÉCLENCHEMENT".into(),
                items: ["Une fois", "À chaque fois · 15 min", "À chaque fois · 1 h"]
                    .iter()
                    .enumerate()
                    .map(|(k, l)| it(30 + k as i64, l, k == 0))
                    .collect(),
                strip: true,
            },
            MenuGroup {
                title: "VALIDITÉ".into(),
                items: ["Sans fin", "24 h", "7 jours"]
                    .iter()
                    .enumerate()
                    .map(|(k, l)| it(40 + k as i64, l, k == 0))
                    .collect(),
                strip: true,
            },
        ];
        let m = ListMenu::new(groups, vec![], Rect::new(10.0, 10.0, 30.0, 30.0), 1440.0, 900.0);
        echecs.extend(comparer("menu-alertes", &m.draw(&Text::default(), None).expect("menu")));
    }
    // 4) la fenêtre 🔔 du PC
    {
        use argos_render::notifs::{exemples, Notifs};
        let n = Notifs {
            pop: Some(Notifs::sous(Rect::new(1300.0, 12.0, 40.0, 36.0), 1440.0, 900.0)),
            ..Default::default()
        };
        let (pm, _) = n.draw(&Text::default(), 1440.0, 900.0, &exemples(10_000), 10_000, None, false).expect("🔔");
        echecs.extend(comparer("fenetre-notifications", &pm));
    }
    assert!(
        echecs.is_empty(),
        "zones changées sans demande (si c'est voulu : ARGOS_REFERENCES=maj, puis regarder et commiter les images) :\n{}",
        echecs.join("\n")
    );
}
