//! 👆 interface-tactile-1 (Jee, 24 Nov 2026, on the iPhone and the tablet: « tout est brisé »): the line between the
//! chart and a pane caught by the finger, a pane's value column (Argos Vague's, a script's), the market search typed
//! with the page's keyboard and the ≫ button back to live.

use super::*;
use argos_render::indicators::Ind;
use winit::event::TouchPhase;

/// 📱 The phone at 402 × 874 by the finger, with Argos Vague and the RSI, laid out as on the screen.
fn phone() -> App {
    let mut a = cell_390();
    a.watch.phone = Some(402);
    a.ui.form.kind = argos_render::formfactor::Kind::Phone;
    a.ui.taille_test = Some(winit::dpi::PhysicalSize::new(402, 874));
    a.ui.frames = 5;
    a.ind.on = vec![Ind::ArgosVague, Ind::Rsi];
    a.refresh_indicators();
    argos_render::actions::set_tactile(true);
    a.draw();
    argos_render::actions::set_tactile(true);
    assert!(a.top.phone, "the phone's layout");
    a
}

/// The top of the band `ind` (chart points).
fn top_of(a: &App, ind: Ind) -> f64 {
    let ph = a.chart.viewport.plot_h();
    a.ind.out.bands(ph).into_iter().find(|(i, _)| *i == ind).expect("the band").1
}

/// 👆 A finger put down at (x, y) of the window, moved by `dy` in steps, lifted.
fn glisser(a: &mut App, x: f64, y: f64, dy: f64) {
    a.touch(7, TouchPhase::Started, (x, y));
    for k in 1..=6 {
        a.touch(7, TouchPhase::Moved, (x, y + dy * k as f64 / 6.0));
    }
    a.touch(7, TouchPhase::Ended, (x, y + dy));
}

/// 👆 A long press at (x, y) of the window (held more than LONG_MS), then moved by `dy`, then lifted.
fn appui_long_puis_glisser(a: &mut App, x: f64, y: f64, dy: f64) {
    a.touch(8, TouchPhase::Started, (x, y));
    std::thread::sleep(std::time::Duration::from_millis(crate::input::touch::LONG_MS as u64 + 100));
    a.touch_tick();
    for k in 1..=6 {
        a.touch(8, TouchPhase::Moved, (x, y + dy * k as f64 / 6.0));
    }
    a.touch(8, TouchPhase::Ended, (x, y + dy));
}

/// 👆 A short tap read late stays a tap; a finger really held makes its long press (bug B-047, asked by the Contrôle on
/// 25 Nov 2026). The finger goes down on the chart; an image then takes 700 ms to draw (a slow device); the finger
/// lifted 100 ms after its down, but its lift waits in the browser behind the image, and winit looks at the long press
/// (`about_to_wait`) right after the image, before the page reads the lift. Each round of work of the event loop begins
/// with `debut_du_tour` (`new_events`), as in the app. The look is put off by a short wait (50 ms), not done at once:
/// the lift waiting in the browser is read in that wait whatever order the browser keeps (asked by the Contrôle).
#[test]
fn a_short_tap_read_late_stays_a_tap() {
    use std::time::Duration;
    dossier_a_soi("tap-lu-tard");
    let image = Duration::from_millis(700);
    let mut a = phone();
    let (x, y) = (a.rw() + 150.0, a.top_h() + 120.0);
    // round 1: the finger's down
    a.debut_du_tour();
    a.touch(9, TouchPhase::Started, (x, y));
    assert!(a.touch_tick().is_some(), "woken for the long press");
    // round 2: an image of 700 ms, then the look at the long press: one more look first, a short wait later (50 ms,
    // whatever order the browser keeps), so the lift waiting behind the image is read before it
    a.debut_du_tour();
    std::thread::sleep(image);
    let attente = Duration::from_secs_f64(crate::input::touch::RELIRE_MS / 1000.0);
    let avant = web_time::Instant::now();
    let regard = a.touch_tick();
    let apres = web_time::Instant::now();
    assert!(
        regard.is_some_and(|t| t >= avant + attente && t <= apres + attente),
        "one more look, 50 ms later: {regard:?}"
    );
    assert!(a.ui.croix.is_none() && !a.ui.context.is_open(), "no long press yet");
    // round 3: the lift, read in that wait, 700 ms after the down: a tap
    a.debut_du_tour();
    a.touch(9, TouchPhase::Ended, (x, y));
    assert!(a.touch_tick().is_none());
    assert!(a.ui.croix.is_none() && !a.ui.context.is_open(), "a tap, never a long press");

    // the other side: the finger still down after the image makes its long press (the crosshair) at the next look
    let mut a = phone();
    a.debut_du_tour();
    a.touch(9, TouchPhase::Started, (x, y));
    a.touch_tick();
    a.debut_du_tour();
    std::thread::sleep(image);
    assert!(a.touch_tick().is_some());
    assert!(a.ui.croix.is_none());
    // the app woken at the end of the wait (nothing read meanwhile): the long press
    std::thread::sleep(attente);
    a.debut_du_tour();
    a.touch_tick();
    assert!(a.ui.croix.is_some(), "held: the crosshair, one look later");
    // lifted after its long press: never read as a tap (the crosshair stays where it was laid)
    let croix = a.ui.croix;
    a.debut_du_tour();
    a.touch(9, TouchPhase::Ended, (x, y));
    assert!(a.ui.croix.is_some() && a.ui.croix == croix, "lifted after a long press: no tap");

    // a finger held while the app waits (woken at the long press's time): the long press at once, as before
    let mut a = phone();
    a.debut_du_tour();
    a.touch(9, TouchPhase::Started, (x, y));
    a.touch_tick();
    std::thread::sleep(Duration::from_millis(crate::input::touch::LONG_MS as u64 + 50));
    a.debut_du_tour();
    a.touch_tick();
    assert!(a.ui.croix.is_some(), "the crosshair at its time");
}

/// ↕️ By the finger the line between the chart and a pane is caught 12 points away: dragged at once, or after a long
/// press on it (no crosshair, no menu); a long press in the chart still gives the crosshair. The mouse 12 points away
/// does not catch it (±5, as before).
#[test]
fn the_finger_catches_a_panes_top_edge() {
    dossier_a_soi("touch-edge");
    // dragged at once, 12 points under the line
    let mut a = phone();
    let (x, th) = (a.rw() + 150.0, a.top_h());
    let h0 = Ind::ArgosVague.band_h();
    let top = top_of(&a, Ind::ArgosVague);
    glisser(&mut a, x, th + top + 12.0, 60.0);
    let h = a.chart.settings.bands.get(&Ind::ArgosVague).copied().expect("Argos Vague's height changed");
    assert!((h - (h0 - 60.0)).abs() < 1.0, "a pane 60 points shorter: {h0} → {h}");
    assert!(a.pointer.band_drag.is_none(), "let go");
    // a long press on the line, then the finger moves: caught, no crosshair, no menu
    let mut a = phone();
    let top = top_of(&a, Ind::ArgosVague);
    appui_long_puis_glisser(&mut a, x, th + top + 3.0, 50.0);
    let h = a.chart.settings.bands.get(&Ind::ArgosVague).copied().expect("caught after the long press");
    assert!((h - (h0 - 50.0)).abs() < 1.0, "{h0} → {h}");
    assert!(a.ui.croix.is_none() && !a.ui.context.is_open(), "no crosshair, no menu");
    assert!(a.pointer.band_drag.is_none(), "lifted: let go");
    // a long press in the chart: the crosshair, as before
    let mut a = phone();
    appui_long_puis_glisser(&mut a, x, th + 120.0, 0.0);
    assert!(a.ui.croix.is_some(), "the crosshair");
    assert!(a.chart.settings.bands.is_empty());
    // the mouse, 12 points under the line: the edge is not caught
    argos_render::actions::set_tactile(false);
    let mut p = app();
    p.top.prefs.bienvenue_vue = true;
    p.ind.on = vec![Ind::ArgosVague];
    p.ui.frames = 5;
    p.refresh_indicators();
    let _ = argos_render::render(&p.chart_with(false));
    let top = top_of(&p, Ind::ArgosVague);
    let (px, py) = (p.rw() + 300.0, p.top_h() + top + 12.0);
    p.mouse_moved(PhysicalPosition::new(px, py));
    p.left_button(ElementState::Pressed);
    p.mouse_moved(PhysicalPosition::new(px, py + 60.0));
    p.left_button(ElementState::Released);
    assert!(p.chart.settings.bands.is_empty(), "the mouse: ±5 points, as before");
    argos_render::actions::set_tactile(false);
}

/// The pixels of the chart image in the rows `y0..y1`.
fn rangees(a: &App, y0: f64, y1: f64) -> Vec<u8> {
    let pm = argos_render::render(&a.chart_with(false));
    let w = pm.width() as usize * 4;
    let (r0, r1) = (
        (argos_render::px::dev(y0) as usize).min(pm.height() as usize),
        (argos_render::px::dev(y1) as usize).min(pm.height() as usize),
    );
    pm.data()[r0 * w..r1 * w].to_vec()
}

/// 🌊 Argos Vague's pane (Jee's oscillator with SURACHAT / SURVENTE), by the finger: dragged in its value column its
/// scale changes, a double tap there puts it back; dragged inside it, it moves up / down.
#[test]
fn argos_vague_value_column_zooms_and_resets() {
    dossier_a_soi("touch-vague-column");
    let mut a = phone();
    let (th, pw) = (a.top_h(), a.chart.viewport.plot_w());
    let top = top_of(&a, Ind::ArgosVague);
    let (y0, y1) = (top + 30.0, top + Ind::ArgosVague.band_h() - 10.0);
    let avant = rangees(&a, y0, y1);
    let col = a.rw() + pw + 20.0;
    glisser(&mut a, col, th + (y0 + y1) / 2.0, 80.0);
    let (_, z) = a.chart.settings.band_view.get(&Ind::ArgosVague).copied().expect("its view");
    assert!(z > 1.2, "dragged down the column: a larger scale ({z})");
    assert_ne!(rangees(&a, y0, y1), avant, "the pane is drawn with its new scale");
    // a double tap in the column: its view put back
    a.pointer.last_axis_click = None;
    toucher(&mut a, col, th + (y0 + y1) / 2.0);
    toucher(&mut a, col, th + (y0 + y1) / 2.0);
    assert!(a.chart.settings.band_view.get(&Ind::ArgosVague).is_none_or(|v| v.1 == 1.0), "reset");
    assert_eq!(rangees(&a, y0, y1), avant, "drawn as before");
    // dragged inside the pane: it moves
    let x = a.rw() + 120.0;
    glisser(&mut a, x, th + (y0 + y1) / 2.0, 50.0);
    let (off, _) = a.chart.settings.band_view.get(&Ind::ArgosVague).copied().expect("moved");
    assert!(off.abs() > 0.01, "{off}");
    argos_render::actions::set_tactile(false);
}

/// 🌊 Argos Vague's pane dragged far up, then far down (Jee, 24 Nov 2026, with a screenshot: « gros bug », « les
/// indicateurs doivent rester dans leur boîte »): its curves never spill over the price or the pane below it; only
/// its own pane changes.
#[test]
fn argos_vague_dragged_stays_in_its_pane() {
    dossier_a_soi("touch-vague-box");
    let mut a = phone();
    let th = a.top_h();
    let (top, bh) = (top_of(&a, Ind::ArgosVague), Ind::ArgosVague.band_h());
    let (dessus, dessous) = (rangees(&a, 0.0, top - 1.0), rangees(&a, top + bh + 1.0, 10_000.0));
    let pane = rangees(&a, top, top + bh);
    let x = a.rw() + 120.0;
    // (each drag from the pane at rest, the finger on the screen: 250 points from its middle reach the chart's bottom.
    // Before B-019 the second drag, at the same place right after the first, was taken as a double tap: the pane went
    // full screen, and this test passed on it)
    for dy in [-250.0, 250.0] {
        a.chart.settings.band_view.remove(&Ind::ArgosVague);
        glisser(&mut a, x, th + top + bh / 2.0, dy);
        assert!(a.ui.plein_ecran.is_none(), "dragged {dy}: a drag, not a double tap: never full screen");
        let (off, _) = a.chart.settings.band_view.get(&Ind::ArgosVague).copied().expect("moved");
        assert!(off.abs() > 0.5, "dragged {dy}: {off}");
        assert_ne!(rangees(&a, top, top + bh), pane, "dragged {dy}: its pane moved");
        assert!(rangees(&a, 0.0, top - 1.0) == dessus, "dragged {dy}: nothing drawn over the price");
        assert!(rangees(&a, top + bh + 1.0, 10_000.0) == dessous, "dragged {dy}: nothing drawn below its pane");
    }
    argos_render::actions::set_tactile(false);
}

/// 📜 A user script's lower pane, with the mouse as by the finger: its value column zooms it (drag, wheel), a double
/// click there puts it back, a drag inside it moves it; the view is kept by script id.
#[test]
fn a_script_pane_value_column_zooms_and_resets() {
    dossier_a_soi("touch-script-column");
    argos_render::actions::set_tactile(false);
    let mut a = app();
    tester_dans_l_editeur(&mut a, "//@version=6\nindicator(\"p\")\nplot(close)\nhline(0)\n");
    // (the editor closed: the chart's value column under the mouse)
    a.ui.editor = None;
    a.refresh_indicators();
    let _ = argos_render::render(&a.chart_with(false));
    let ph = a.chart.viewport.plot_h();
    let (k, top) = a.ind.out.script_bands(ph)[0];
    let key = a.ind.out.scripts[k].view_key();
    assert_eq!(key, a.scripts[0].id, "kept by script id");
    let bas = top + a.ind.out.scripts[k].band_h();
    let avant = rangees(&a, top + 25.0, bas - 5.0);
    let (col, mid) = (a.rw() + a.chart.viewport.plot_w() + 20.0, a.top_h() + (top + bas) / 2.0);
    a.mouse_moved(PhysicalPosition::new(col, mid));
    a.left_button(ElementState::Pressed);
    a.mouse_moved(PhysicalPosition::new(col, mid + 80.0));
    a.left_button(ElementState::Released);
    let (_, z) = a.chart.settings.script_view.get(&key).copied().expect("its view");
    assert!(z > 1.2, "{z}");
    assert_ne!(rangees(&a, top + 25.0, bas - 5.0), avant, "drawn with its new scale");
    // double click on the column: put back
    a.pointer.last_axis_click = None;
    clic(&mut a, col, mid);
    clic(&mut a, col, mid);
    assert!(!a.chart.settings.script_view.contains_key(&key), "reset");
    // the wheel on the column (two fingers pinching there do the same)
    a.mouse_moved(PhysicalPosition::new(col, mid));
    a.mouse_wheel(winit::event::MouseScrollDelta::LineDelta(0.0, 2.0));
    assert!(a.chart.settings.script_view.get(&key).is_some_and(|v| v.1 < 1.0), "wheel: zoomed");
    // a drag inside the pane moves it
    a.chart.settings.script_view.clear();
    let x = a.rw() + 200.0;
    a.pointer.last_chart_click = None;
    a.mouse_moved(PhysicalPosition::new(x, mid));
    a.left_button(ElementState::Pressed);
    a.mouse_moved(PhysicalPosition::new(x, mid + 30.0));
    a.left_button(ElementState::Released);
    assert!(a.chart.settings.script_view.get(&key).is_some_and(|v| v.0.abs() > 0.01), "moved");
}

/// 🔎 The market sheet of the phone: its field « Chercher un marché » is a zone where one types (the page's hidden
/// field takes the focus there, the iPhone's keyboard opens); the text of the page's keyboard goes into it and
/// filters the list, its Backspace erases, its Escape closes the sheet.
#[test]
fn the_market_search_takes_the_page_keyboard() {
    dossier_a_soi("touch-market-search");
    let mut a = phone();
    assert!(a.marches_zone_saisie().is_none(), "closed: no zone");
    a.marches_ouvrir(Rect::default(), true);
    let m = a.ui.marches.as_ref().expect("the sheet");
    let zone = a.marches_zone_saisie().expect("its field is a zone where one types");
    assert_eq!(zone, m.champ_rect());
    assert!(m.rect.contains(zone.cx(), zone.cy()) && zone.h >= 30.0, "{zone:?} in {:?}", m.rect);
    assert!(a.touche_page(None, Some("eth")), "the page's text goes to the sheet");
    assert_eq!(a.ui.marches.as_ref().unwrap().texte, "eth");
    assert_eq!(a.ui.marches.as_ref().unwrap().requete(), "eth", "the list is searched with it");
    assert!(a.touche_page(Some("Backspace"), None));
    assert_eq!(a.ui.marches.as_ref().unwrap().texte, "et");
    assert!(a.touche_page(Some("Escape"), None));
    assert!(a.ui.marches.is_none(), "Escape closes it");
    argos_render::actions::set_tactile(false);
}

/// ≫ By the finger (phone, Argos Murs on): a touch on the button's centre, or 4 points beside the drawn circle, goes
/// back to live; also while offline; then the button is gone and does not come back by itself. It stays where it is
/// from one drawing to the next.
#[test]
fn the_back_to_live_button_answers_the_finger() {
    dossier_a_soi("touch-back-to-live");
    let live = argos_render::viewport::Viewport::default().right_offset;
    for (cas, hors_ligne) in [("centre", false), ("beside", false), ("offline", true)] {
        let mut a = murs_aux_dernieres_bougies(phone());
        a.chart.hors_ligne = hors_ligne;
        argos_render::actions::set_tactile(true);
        let s = a.chart.slots.clone();
        a.chart.viewport.scroll_px(400.0, &s);
        let _ = argos_render::render(&a.chart_with(false));
        let r = argos_render::bouton_direct().expect("≫ off live");
        assert!(r.w <= 36.0 + 1e-9, "{cas}: drawn at 36 on the phone");
        // drawn again: at the same place
        let _ = argos_render::render(&a.chart_with(false));
        assert_eq!(argos_render::bouton_direct(), Some(r), "{cas}: it stays put");
        let (x, y) = if cas == "beside" { (r.x - 4.0, r.cy()) } else { (r.cx(), r.cy()) };
        let (wx, wy) = (x + a.rw(), y + a.top_h());
        toucher(&mut a, wx, wy);
        assert_eq!(a.chart.viewport.right_offset, live, "{cas}: back to live");
        a.murs_refresh();
        let _ = argos_render::render(&a.chart_with(false));
        assert!(argos_render::bouton_direct().is_none(), "{cas}: gone, and not back by itself");
        assert_eq!(a.chart.viewport.right_offset, live);
    }
    argos_render::actions::set_tactile(false);
}

/// 📱 / 🖥 Argos Vague and the RSI at `w` × `h` (402: the phone, 819 × 1008: the tablet upright, 1092 × 734: lying
/// down, 1440: the PC, the only one with the mouse), Argos Murs on or off, laid out and drawn.
fn vague_at(w: u32, h: u32, murs: bool) -> App {
    argos_render::actions::set_tactile(false);
    let tactile = w != 1440;
    let mut a = if w < 560 {
        phone()
    } else {
        let mut a = app();
        a.top.prefs.bienvenue_vue = true;
        a.ind.on = vec![Ind::ArgosVague, Ind::Rsi];
        a.refresh_indicators();
        a
    };
    if murs {
        a = murs_aux_dernieres_bougies(a);
    }
    if w >= 560 && tactile {
        a.ui.form.kind = if w < h {
            argos_render::formfactor::Kind::TabletPortrait
        } else {
            argos_render::formfactor::Kind::TabletLandscape
        };
    }
    let _ = image_ecran(&mut a, w, h);
    argos_render::actions::set_tactile(tactile);
    let _ = argos_render::render(&a.chart_with(false));
    a
}

/// 🌊 Argos Vague's labels (SURACHAT, TENDANCE HAUSSIÈRE / BAISSIÈRE, its state, SURVENTE; Jee, 24 Nov: « Pilules verre
/// par-dessus, à droite », then « je ne veux pas qu'elle embarque sur l'oscillateur »): live, at 402, 819 and 1440,
/// Argos Murs on and off, they are drawn as pills in its pane, right of its last bar (never on its curves), over its
/// value column when the chart has no room, apart from one another.
#[test]
fn argos_vague_labels_are_glass_pills_in_its_pane() {
    dossier_a_soi("vague-pills");
    for (w, h) in [(402u32, 874u32), (819, 1008), (1092, 734), (1440, 900)] {
        for murs in [false, true] {
            let quoi = format!("{w} Murs {murs}");
            let a = vague_at(w, h, murs);
            assert!(a.ind.out.shown(Ind::ArgosVague), "{quoi}");
            let pills = argos_render::scriptview::vague_pills();
            let (top, bh, pw) = (top_of(&a, Ind::ArgosVague), Ind::ArgosVague.band_h(), a.chart.viewport.plot_w());
            assert!(pills.len() >= 3, "{quoi}: {pills:?}");
            assert!(pills.iter().any(|(_, t)| t.starts_with('●')), "{quoi}: the state {pills:?}");
            let m = argos_render::scriptview::VAGUE_PILL_MARGIN;
            let (x_last, axe) = (
                a.chart.viewport.x_of(a.chart.bars.len() - 1, &a.chart.slots),
                a.chart.viewport.price_axis as f64,
            );
            for (r, t) in &pills {
                assert!(
                    r.x >= x_last + m && r.x + r.w <= pw + axe - m + 0.01,
                    "{quoi}: {t} {r:?} right of the last bar {x_last}"
                );
                assert!(
                    r.x >= 0.0 && r.y >= top && r.y + r.h <= top + bh + 0.01,
                    "{quoi}: {t} {r:?} in the pane {top} + {bh}"
                );
            }
            for (i, (p, _)) in pills.iter().enumerate() {
                for (q, _) in &pills[i + 1..] {
                    assert!(p.y + p.h <= q.y + 0.01 || q.y + q.h <= p.y + 0.01, "{quoi}: {p:?} on {q:?}");
                }
            }
        }
    }
    argos_render::actions::set_tactile(false);
}

/// 🏷️ By the finger, a slide that starts on a legend button away from any pane's edge (the price legend's « ⌄ N ») is
/// played as before past the legend: the chart scrolls, the button does nothing; a real tap there still acts.
#[test]
fn a_slide_from_a_legend_button_scrolls_the_chart() {
    dossier_a_soi("touch-legend-slide");
    for (w, h) in [(819u32, 1008u32), (1092, 734), (402, 874)] {
        let mut a = vague_at(w, h, false);
        let (pw, ph) = (a.chart.viewport.plot_w(), a.chart.viewport.plot_h());
        let (x, y) = (0..(ph as i32))
            .flat_map(|y| (0..(pw as i32)).step_by(2).map(move |x| (x as f64 + 2.0, y as f64 + 2.0)))
            .find(|&(x, y)| matches!(a.legende_bouton(x, y), Some(argos_render::indleg::LegClick::Toggle)))
            .expect("the legend's « ⌄ N »");
        let (wx, wy) = (a.rw() + x, a.top_h() + y);
        let (open, ro) = (a.ind.leg.0, a.chart.viewport.right_offset);
        a.touch(6, TouchPhase::Started, (wx, wy));
        for k in 1..=8 {
            a.touch(6, TouchPhase::Moved, (wx + 25.0 * k as f64, wy));
        }
        a.touch(6, TouchPhase::Ended, (wx + 200.0, wy));
        assert_eq!(a.ind.leg.0, open, "{w}: the slide does not open the list");
        assert_ne!(a.chart.viewport.right_offset, ro, "{w}: the chart scrolled");
        // a real tap: the list opens or folds, as before
        a.pointer.last_chart_click = None;
        toucher(&mut a, wx, wy);
        assert_ne!(a.ind.leg.0, open, "{w}: a real tap acts");
    }
    argos_render::actions::set_tactile(false);
}

/// 🏷️ Where the finger finds Argos Vague's 👁 in its legend (window coordinates), within reach of the pane's top edge.
fn oeil_de_vague(a: &App) -> (f64, f64) {
    let top = top_of(a, Ind::ArgosVague);
    let (pw, ph) = (a.chart.viewport.plot_w(), a.chart.viewport.plot_h());
    let (x, y) = (0..(ph as i32))
        .flat_map(|y| (0..(pw as i32)).step_by(2).map(move |x| (x as f64, y as f64)))
        .find(|&(x, y)| matches!(a.legende_bouton(x, y), Some(argos_render::indleg::LegClick::Eye(Ind::ArgosVague))))
        .expect("Argos Vague's 👁");
    // (its top row, 2 points in)
    let y = y + 2.0;
    assert!((y - top).abs() <= 12.0, "the 👁 at {x}, {y}: within reach of the pane's top edge {top}");
    (a.rw() + x + 2.0, a.top_h() + y)
}

/// 👆 The tablet upright (and the phone) by the finger: a legend button of a pane acts only on a real tap. A finger
/// that slides from Argos Vague's 👁 (within reach of the pane's edge) drags the edge and Argos Vague stays shown; a
/// long press there catches the edge too; a finger that went 9 points before lifting does nothing; a real tap hides
/// it, as before.
#[test]
fn legend_buttons_only_on_a_real_tap() {
    dossier_a_soi("touch-legend-tap");
    // (the legend's list open: the panes' buttons shown, as when the finger is on the pane)
    let ouverte = |w: u32, h: u32| {
        let mut a = vague_at(w, h, false);
        a.ind.leg.0 = true;
        let _ = argos_render::render(&a.chart_with(false));
        a
    };
    for (w, h) in [(819u32, 1008u32), (1092, 734)] {
        // a slide from the 👁: the pane taller, Argos Vague still there
        let mut a = ouverte(w, h);
        let h0 = Ind::ArgosVague.band_h();
        let (x, y) = oeil_de_vague(&a);
        glisser(&mut a, x, y, -60.0);
        assert!(a.ind.out.shown(Ind::ArgosVague), "{w}: a slide does not hide it");
        let h1 = a.chart.settings.bands.get(&Ind::ArgosVague).copied().expect("the edge dragged");
        assert!(h1 > h0 + 30.0, "{w}: taller {h0} → {h1}");
        assert!(a.pointer.band_drag.is_none(), "{w}: let go");
        // a long press on the 👁: the edge caught
        let mut a = ouverte(w, h);
        let (x, y) = oeil_de_vague(&a);
        appui_long_puis_glisser(&mut a, x, y, -40.0);
        assert!(a.ind.out.shown(Ind::ArgosVague), "{w}: a long press does not hide it");
        let h2 = a.chart.settings.bands.get(&Ind::ArgosVague).copied().expect("caught after the long press");
        assert!(h2 > h0 + 20.0, "{w}: {h0} → {h2}");
        // the finger went 9 points and lifted: nothing
        let mut a = ouverte(w, h);
        let (x, y) = oeil_de_vague(&a);
        a.touch(5, TouchPhase::Started, (x, y));
        a.touch(5, TouchPhase::Moved, (x + 9.0, y));
        a.touch(5, TouchPhase::Ended, (x + 9.0, y));
        assert!(a.ind.out.shown(Ind::ArgosVague), "{w}: no real tap");
        // a real tap: hidden, as before
        toucher(&mut a, x, y);
        assert!(!a.ind.out.shown(Ind::ArgosVague), "{w}: a real tap hides it");
    }
    argos_render::actions::set_tactile(false);
}

/// 👆 (B-048) A tap at (x, y) of the window: the finger lifted at `lift` (its own time, `touch::now_ms`'s clock) and
/// went down 30 ms before; both read now.
fn toucher_a(a: &mut App, (x, y): (f64, f64), lift: f64) {
    a.touch_at(3, TouchPhase::Started, (x, y), lift - 30.0);
    a.touch_at(3, TouchPhase::Ended, (x, y), lift);
}

/// 👆 A double tap is counted with the finger's own times, not with when the app read them (bug B-048): two taps 600 ms
/// apart, the first read late (behind a slow image), are no double tap; two taps 150 ms apart stay one, read together or
/// read 400 ms apart.
#[test]
fn a_double_tap_counts_the_finger_s_own_time() {
    use crate::actions::plein_ecran::PleinEcran;
    dossier_a_soi("double-heure-du-doigt");
    let ici = |a: &App| (100.0 + a.rw(), 60.0 + a.top_h());
    // 600 ms apart by the finger, read together: two taps
    let mut a = cell_pilule();
    let t = crate::input::touch::now_ms();
    let p = ici(&a);
    toucher_a(&mut a, p, t - 600.0);
    toucher_a(&mut a, p, t);
    assert_eq!(a.ui.plein_ecran, None, "600 ms apart by the finger: no double tap");
    // 150 ms apart by the finger, read together: a double tap (full screen)
    let mut a = cell_pilule();
    let t = crate::input::touch::now_ms();
    let p = ici(&a);
    toucher_a(&mut a, p, t - 150.0);
    toucher_a(&mut a, p, t);
    assert_eq!(a.ui.plein_ecran, Some(PleinEcran::Prix), "150 ms apart: a double tap");
    // 150 ms apart by the finger, the second read 400 ms after the first: still a double tap
    let mut a = cell_pilule();
    let t = crate::input::touch::now_ms();
    let p = ici(&a);
    toucher_a(&mut a, p, t);
    std::thread::sleep(std::time::Duration::from_millis(400));
    toucher_a(&mut a, p, t + 150.0);
    assert_eq!(a.ui.plein_ecran, Some(PleinEcran::Prix), "read late, 150 ms apart by the finger: a double tap");
}

/// 👆 A swipe's speed is counted with the finger's own times (bug B-048): a slow drag (900 ms) whose down was read
/// 400 ms late does not close the page; a quick swipe (300 ms) whose lift was read late closes it.
#[test]
fn a_swipe_counts_the_finger_s_own_time() {
    use std::time::Duration;
    dossier_a_soi("coup-heure-du-doigt");
    let (x, y) = (60.0, 400.0);
    let glisser_a = |a: &mut App, down: f64, lift: f64| {
        a.touch_at(4, TouchPhase::Started, (x, y), down);
        for k in 1..=5 {
            a.touch_at(4, TouchPhase::Moved, (x + 30.0 * k as f64, y), down + (lift - down) * k as f64 / 6.0);
        }
        a.touch_at(4, TouchPhase::Ended, (x + 150.0, y), lift);
    };
    // slow by the finger (900 ms), read in 500 ms: the page stays
    let mut a = cell_pilule();
    a.ui.pf_page = Some(argos_render::portfolio::PfPage::new());
    let t = crate::input::touch::now_ms();
    let lift = t + 500.0;
    a.touch_at(4, TouchPhase::Started, (x, y), t - 400.0);
    std::thread::sleep(Duration::from_millis(520));
    for k in 1..=5 {
        a.touch_at(4, TouchPhase::Moved, (x + 30.0 * k as f64, y), lift - 20.0 * (6 - k) as f64);
    }
    a.touch_at(4, TouchPhase::Ended, (x + 150.0, y), lift);
    assert!(a.ui.pf_page.is_some(), "a slow drag by the finger, its down read late: no swipe");
    // quick by the finger (300 ms), its lift read 800 ms after its down: the page closes
    let mut a = cell_pilule();
    a.ui.pf_page = Some(argos_render::portfolio::PfPage::new());
    let t = crate::input::touch::now_ms();
    a.touch_at(4, TouchPhase::Started, (x, y), t);
    std::thread::sleep(Duration::from_millis(820));
    for k in 1..=5 {
        a.touch_at(4, TouchPhase::Moved, (x + 30.0 * k as f64, y), t + 50.0 * k as f64);
    }
    a.touch_at(4, TouchPhase::Ended, (x + 150.0, y), t + 300.0);
    assert!(a.ui.pf_page.is_none(), "a quick swipe by the finger, read late: the page closes");
    // read as it goes, quick: closes, as before
    let mut a = cell_pilule();
    a.ui.pf_page = Some(argos_render::portfolio::PfPage::new());
    let t = crate::input::touch::now_ms();
    glisser_a(&mut a, t - 300.0, t);
    assert!(a.ui.pf_page.is_none(), "a quick swipe: the page closes");
}

/// 👆 The fling after a finger scroll counts the finger's own times (bug B-048): a finger lifted while moving keeps its
/// fling even when its lift is read 300 ms late; a finger held still 200 ms before lifting has none, even when all is
/// read at once.
#[test]
fn the_fling_counts_the_finger_s_own_time() {
    dossier_a_soi("elan-heure-du-doigt");
    let (x, y) = (200.0, 600.0);
    let defiler = |a: &mut App, t0: f64| {
        a.touch_at(6, TouchPhase::Started, (x, y), t0);
        for k in 1..=6 {
            a.touch_at(6, TouchPhase::Moved, (x, y - 30.0 * k as f64), t0 + 16.0 * k as f64);
        }
    };
    // lifted while moving (16 ms after its last move), the lift read 300 ms late: the fling goes on
    let mut a = cell_pilule();
    a.ui.pf_page = Some(argos_render::portfolio::PfPage::new());
    let t = crate::input::touch::now_ms();
    defiler(&mut a, t - 96.0);
    assert!(a.pointer.finger_scroll.is_some(), "the page scrolls under the finger");
    std::thread::sleep(std::time::Duration::from_millis(300));
    a.touch_at(6, TouchPhase::Ended, (x, y - 180.0), t + 16.0);
    assert!(a.pointer.fling.0 != 0.0, "lifted while moving: a fling, even read late");
    // held still 200 ms before lifting, all read at once: no fling
    let mut a = cell_pilule();
    a.ui.pf_page = Some(argos_render::portfolio::PfPage::new());
    let t = crate::input::touch::now_ms();
    defiler(&mut a, t - 296.0);
    a.touch_at(6, TouchPhase::Ended, (x, y - 180.0), t);
    assert_eq!(a.pointer.fling.0, 0.0, "held still before lifting: no fling, even read at once");
}

/// 👆 The floating bar's held press (380 ms: the bar follows) counts the finger's own times (bug B-048): a ⭐ slid along
/// the bar 40 ms after its press, read 400 ms late, sorts the favourites (the bar stays); held 400 ms by the finger, it
/// lifts the bar.
#[test]
fn the_floating_bar_s_held_press_counts_the_finger_s_own_time() {
    use argos_render::toolbar::{self, Btn};
    dossier_a_soi("barre-heure-du-doigt");
    let text = argos_render::text::Text::default();
    let place = |a: &App| {
        let (_, btns) = toolbar::layout(&a.tools, &text);
        let r = btns.iter().find(|(_, b)| matches!(b, Btn::Fav(_))).expect("a ⭐ on the bar").0;
        (r.cx() + a.rw(), r.cy() + a.top_h())
    };
    let pas = |a: &App, d: f64| if a.tools.vertical { (0.0, d) } else { (d, 0.0) };
    let barre_aux_favoris = || {
        let mut a = app();
        a.top.prefs.bienvenue_vue = true;
        a.rail.favs = vec!["trend".into(), "hline".into(), "rect".into()];
        a.sync_toolbar();
        a
    };
    // slid 40 ms after its press, read 400 ms late: sorting, the bar stays
    let mut a = barre_aux_favoris();
    let (x, y) = place(&a);
    let barre = (a.tools.x, a.tools.y);
    let t = crate::input::touch::now_ms();
    let (dx, dy) = pas(&a, 11.0);
    a.touch_at(2, TouchPhase::Started, (x, y), t - 20.0);
    a.touch_at(2, TouchPhase::Moved, (x + dx, y + dy), t);
    assert!(a.pointer.bar_press.is_some_and(|b| b.mode == PressMode::Wait), "pressed on the ⭐");
    std::thread::sleep(std::time::Duration::from_millis(400));
    let (dx, dy) = pas(&a, 20.0);
    a.touch_at(2, TouchPhase::Moved, (x + dx, y + dy), t + 40.0);
    assert!(a.pointer.bar_press.is_some_and(|b| b.mode == PressMode::Sort), "40 ms by the finger: sorting");
    assert_eq!((a.tools.x, a.tools.y), barre, "the bar stays");
    a.touch_at(2, TouchPhase::Ended, (x + dx, y + dy), t + 60.0);
    // held 400 ms by the finger (read at once): the bar follows
    let mut a = barre_aux_favoris();
    let (x, y) = place(&a);
    let barre = (a.tools.x, a.tools.y);
    let t = crate::input::touch::now_ms();
    let (dx, dy) = pas(&a, 11.0);
    a.touch_at(2, TouchPhase::Started, (x, y), t - 440.0);
    a.touch_at(2, TouchPhase::Moved, (x + dx, y + dy), t - 420.0);
    let (dx, dy) = pas(&a, 20.0);
    a.touch_at(2, TouchPhase::Moved, (x + dx, y + dy + 30.0), t);
    assert!(
        a.pointer.bar_press.is_some_and(|b| matches!(b.mode, PressMode::Lift(..))),
        "held 400 ms: the bar follows"
    );
    assert_ne!((a.tools.x, a.tools.y), barre, "the bar moved");
}
