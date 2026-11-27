//! 🧱 Quality gate, bug B-001: a black band right of the candles, in
//! the price pane and in every indicator pane (Jee, 24 Nov 2026, five times: « le noir à droite », « un bloc noir »).
//! H7 took Argos Murs' band away; this guard keeps any band from coming back, whatever brings it: on a phone and on a
//! tablet, upright and lying down, at the first image and after the screen is turned.

use super::*;
use argos_render::formfactor::{choose, Kind, Screen};
use argos_render::indicators::Ind;

/// Jee's screens, by the finger, in their browser: the iPhone 402 × 778 upright (as the safety net reads it) and
/// 778 × 370 lying down; the iPad Pro 13 1024 × 1260 upright and 1366 × 918 lying down.
pub(super) fn ecran(w: f64, h: f64, scale: f64) -> Screen {
    Screen {
        w,
        h,
        scale,
        coarse: true,
        ..Default::default()
    }
}

/// The app told its screen turned, as `App::update_form` does with a window: its form and its size in points (the
/// page's px over the zoom of the form); a tablet turned upright folds its panel, turned back it opens it again.
fn tourner(a: &mut App, s: &Screen) {
    let (before, form) = (a.ui.form, choose(s));
    a.ui.form = form;
    if form.drawer() && !before.drawer() && before.kind != Kind::Phone {
        a.ui.drawer_was_open = a.watch.prefs.open;
        a.watch.prefs.open = false;
    } else if before.drawer() && !form.drawer() && a.ui.drawer_was_open {
        a.watch.prefs.open = true;
    }
    a.ui.taille_test = Some(winit::dpi::PhysicalSize::new((s.w / form.zoom) as u32, (s.h / form.zoom) as u32));
}

/// The app at rest on that screen (the tests' candles, the welcome pages seen), RSI in a pane under the price, and
/// Argos Murs on with two walls at the last candles' price when `murs`.
pub(super) fn ouverte(s: &Screen, murs: bool) -> App {
    let mut a = app();
    a.top.prefs.bienvenue_vue = true;
    a.ui.form = choose(s);
    if a.ui.form.drawer() {
        a.ui.drawer_was_open = a.watch.prefs.open;
        a.watch.prefs.open = false;
    }
    a.ind.on.push(Ind::Rsi);
    if murs {
        a = murs_aux_dernieres_bougies(a);
    }
    tourner(&mut a, s);
    a.ui.frames = 5;
    a
}

/// The whole screen drawn (two images: the first lays it out), at the density of the test.
pub(super) fn dessiner(a: &mut App) -> (Vec<u32>, u32, u32, f64) {
    let size = a.ui.taille_test.unwrap();
    a.ui.fond = None;
    a.draw_frame();
    a.draw_frame();
    let sc = argos_render::px::scale() as f64;
    let (w, h) = ((size.width as f64 * sc).round() as u32, (size.height as f64 * sc).round() as u32);
    (std::mem::take(&mut a.ui.image_test), w, h, sc)
}

/// The right-most column (points, from the left of the plot) of `rows` (points, from the top of the screen) where
/// the two images differ, between `x0` and `x0 + pw`; a row where they differ over more than a quarter of the width
/// is a line drawn across the pane, dashed or not (the last price's, a level's): left out. `None`: no difference there.
fn dernier_change(av: &[u32], ap: &[u32], w: u32, sc: f64, x0: f64, pw: f64, rows: (f64, f64)) -> Option<f64> {
    let (c0, c1) = ((x0 * sc).round() as usize, ((x0 + pw) * sc).round() as usize);
    let (r0, r1) = ((rows.0 * sc).round() as usize, (rows.1 * sc).round() as usize);
    let mut droite: Option<usize> = None;
    for y in r0..r1 {
        let ligne = |x: usize| av[y * w as usize + x] != ap[y * w as usize + x];
        let n = (c0..c1).filter(|&x| ligne(x)).count();
        if n == 0 || n * 4 > c1 - c0 {
            continue;
        }
        if let Some(x) = (c0..c1).rev().find(|&x| ligne(x)) {
            droite = droite.max(Some(x));
        }
    }
    droite.map(|x| x as f64 / sc - x0)
}

/// The candle `n` turned into a falling one from its high to its low (or back to `garde`), its indicators computed
/// again at once: its colour and its indicators' last values change, the price range does not (same high, same low).
fn retourner(a: &mut App, n: usize, garde: Option<argos_core::Bar>) -> argos_core::Bar {
    let b = &mut a.chart.bars[n];
    let avant = *b;
    match garde {
        Some(g) => *b = g,
        None => (b.open, b.close) = (b.high, b.low),
    }
    a.ind.key.0 = usize::MAX;
    avant
}

/// The checks on the screen as it is now (`quoi`: the screen and the step, for the messages).
fn sans_bande(a: &mut App, quoi: &str, murs: bool, phone: bool) {
    let (avant, w, h, sc) = dessiner(a);
    // (`ARGOS_CAPTURES=<folder>`: each screen checked, in a PNG, to look at)
    if let Ok(d) = std::env::var("ARGOS_CAPTURES") {
        let mut pm = tiny_skia::Pixmap::new(w, h).unwrap();
        for (p, v) in pm.pixels_mut().iter_mut().zip(&avant) {
            *p = tiny_skia::ColorU8::from_rgba((v >> 16) as u8, (v >> 8) as u8, *v as u8, 255).premultiply();
        }
        let nom: String = quoi.chars().map(|c| if c.is_alphanumeric() { c } else { '-' }).collect();
        pm.save_png(std::path::Path::new(&d).join(format!("bande-noire-{nom}.png"))).unwrap();
    }
    if phone {
        assert!(a.top.phone, "{quoi}: the phone's layout");
    }
    let vp = a.chart.viewport;
    let (pw, ph) = (vp.plot_w(), vp.plot_h());
    if murs {
        assert!(a.murs_marge() > 0.0, "{quoi}: Argos Murs drawn");
    }
    // 1. no band kept in the viewport: the candles' width is the plot's, up to the price scale
    assert_eq!(vp.marge, 0.0, "{quoi}: a band kept right of the candles");
    assert_eq!(vp.bars_w(), pw, "{quoi}: the candles stop before the price scale");
    // 2. the phone's chart takes the whole width of the screen (after a turn too)
    let size = a.ui.taille_test.unwrap();
    if a.top.phone {
        assert_eq!(vp.width, size.width, "{quoi}: the chart is narrower than the screen");
    } else {
        assert!(a.rw() + vp.width as f64 <= size.width as f64 + 0.5, "{quoi}: the chart wider than the screen");
    }
    // 3. live, the last candle `right_offset` candles from the price scale, never further
    let live = argos_render::viewport::Viewport::default().right_offset;
    assert_eq!(vp.right_offset, live, "{quoi}: not live");
    let last = vp.x_of(a.chart.bars.len() - 1, &a.chart.slots);
    let vide = pw - last;
    assert!(
        vide <= (live + 1.0) * vp.bar_spacing,
        "{quoi}: {vide:.0} points empty right of the last candle ({} candles of {:.1})",
        live,
        vp.bar_spacing
    );
    // 4. what is seen: the last candle turned into a falling one changes the picture of
    // every pane right where the last candle is, close to the price scale — never at a band's left edge
    let (x0, y0) = (a.rw(), a.top_h());
    let n = a.chart.bars.len() - 1;
    let garde = retourner(a, n, None);
    let (apres, ..) = dessiner(a);
    retourner(a, n, Some(garde));
    let osc = a.ind.out.osc_h();
    let mut panes: Vec<(String, (f64, f64))> = a
        .ind
        .out
        .bands(ph)
        .into_iter()
        .map(|(i, top)| (format!("pane {i:?}"), (y0 + top, y0 + top + i.band_h())))
        .collect();
    assert!(!panes.is_empty(), "{quoi}: the RSI pane drawn");
    // (the price pane: with Argos Murs on, its pills follow the last price, at the scale: they would say nothing)
    if !murs {
        panes.push(("price pane".into(), (y0, y0 + ph - osc)));
    }
    let au_moins = pw - (live + 1.5) * vp.bar_spacing;
    for (pane, rows) in panes {
        let x = dernier_change(&avant, &apres, w, sc, x0, pw, rows);
        assert!(
            x.is_some_and(|x| x >= au_moins),
            "{quoi}: in the {pane}, the last candle is drawn at {x:?}, not near the price scale (≥ {au_moins:.0} of {pw:.0}): a band right of it"
        );
    }
}

/// 🧱 B-001: no black band right of the candles — the iPhone and the iPad, upright and lying down, at the first image
/// and after being turned (upright → lying → upright, lying → upright), with Argos Murs off and on.
#[test]
fn no_black_band_right_of_the_candles_phone_tablet_any_way_up() {
    dossier_a_soi("bande_noire");
    let appareils = [
        ("iPhone", ecran(402.0, 778.0, 3.0), ecran(778.0, 370.0, 3.0), true),
        ("iPad", ecran(1024.0, 1260.0, 2.0), ecran(1366.0, 918.0, 2.0), false),
    ];
    for murs in [false, true] {
        let m = if murs { "Argos Murs on" } else { "Argos Murs off" };
        for (nom, debout, couche, phone) in &appareils {
            // upright, then turned lying down, then upright again
            let mut a = ouverte(debout, murs);
            sans_bande(&mut a, &format!("{nom} upright, {m}"), murs, *phone);
            tourner(&mut a, couche);
            sans_bande(&mut a, &format!("{nom} turned lying down, {m}"), murs, *phone);
            tourner(&mut a, debout);
            sans_bande(&mut a, &format!("{nom} turned upright again, {m}"), murs, *phone);
            // lying down from the start, then turned upright
            let mut a = ouverte(couche, murs);
            sans_bande(&mut a, &format!("{nom} lying down, {m}"), murs, *phone);
            tourner(&mut a, debout);
            sans_bande(&mut a, &format!("{nom} lying, turned upright, {m}"), murs, *phone);
        }
    }
    argos_render::actions::set_tactile(false);
}

/// The guard sees a band: 20 candles of empty room kept right of the last one (what Argos Murs' band of before
/// looked like) is seen in the price pane and in the RSI pane.
#[test]
fn the_black_band_guard_sees_a_band() {
    dossier_a_soi("bande_noire_vue");
    let s = ecran(402.0, 778.0, 3.0);
    let mut a = ouverte(&s, false);
    let live = argos_render::viewport::Viewport::default().right_offset;
    a.chart.viewport.right_offset = live + 20.0;
    let (avant, w, _, sc) = dessiner(&mut a);
    let vp = a.chart.viewport;
    assert_eq!(vp.right_offset, live + 20.0, "the room kept");
    let (pw, ph) = (vp.plot_w(), vp.plot_h());
    let n = a.chart.bars.len() - 1;
    retourner(&mut a, n, None);
    let (apres, ..) = dessiner(&mut a);
    let (y0, osc) = (a.top_h(), a.ind.out.osc_h());
    let (top, h) = a.ind.out.bands(ph).into_iter().map(|(i, top)| (top, i.band_h())).next().unwrap();
    for (pane, rows) in [("RSI pane", (y0 + top, y0 + top + h)), ("price pane", (y0, y0 + ph - osc))] {
        let x = dernier_change(&avant, &apres, w, sc, a.rw(), pw, rows);
        assert!(
            x.is_some_and(|x| x < pw - (live + 1.5) * vp.bar_spacing),
            "{pane}: the band is seen: last change at {x:?} of {pw:.0}"
        );
    }
    argos_render::actions::set_tactile(false);
}
