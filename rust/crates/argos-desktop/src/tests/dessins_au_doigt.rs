//! ✏️ Every drawing tool by the finger (bug B-019, Jee, 25 Nov 2026: « les outils ça fait dur, c'est même pas
//! utilisable », « même pas capable de sélectionner le fib », « même pas capable de les modifier une fois mis sur le
//! graphique »). On the phone and on the tablet, each tool is laid by the finger as it is drawn (a drag, then a tap
//! for each further point), selected by a tap, moved by a drag, stretched by a handle touched a finger's width off,
//! and its settings opened by a double tap and by the ⚙ of its bar — and the chart never moves nor goes full screen.

use super::bande_noire::{ecran, ouverte};
use super::*;
use winit::event::TouchPhase;

/// Time passes between two gestures (the tests run them at once: two taps would make a double tap).
fn pause(a: &mut App) {
    (
        a.pointer.last_chart_click,
        a.pointer.last_axis_click,
        a.pointer.last_time_click,
        a.pointer.triple,
    ) = (None, None, None, None);
}

/// A finger goes down at the first point (chart coordinates), slides through the others, lifts at the last.
fn doigt(a: &mut App, pts: &[(f64, f64)]) {
    let (rw, th) = (a.rw(), a.top_h());
    let w = |p: (f64, f64)| (p.0 + rw, p.1 + th);
    a.touch(4, TouchPhase::Started, w(pts[0]));
    for p in pts.windows(2) {
        for k in 1..=4 {
            let s = k as f64 / 4.0;
            a.touch(4, TouchPhase::Moved, w((p[0].0 + (p[1].0 - p[0].0) * s, p[0].1 + (p[1].1 - p[0].1) * s)));
        }
    }
    a.touch(4, TouchPhase::Ended, w(*pts.last().unwrap()));
}

/// A tap at (x, y) of the chart.
fn tape(a: &mut App, (x, y): (f64, f64)) {
    let (rw, th) = (a.rw(), a.top_h());
    toucher(a, x + rw, y + th);
}

/// What must not happen while a drawing is laid, touched or moved: the chart moved, zoomed, full screen.
fn vue(a: &App) -> String {
    let vp = &a.chart.viewport;
    format!(
        "{:?} {:?} {:?} {:?} {:?}",
        (vp.right_offset, vp.bar_spacing),
        a.chart.settings.scale.manual,
        a.ui.plein_ecran,
        a.ui.croix,
        a.chart.settings.scale.auto()
    )
}

/// Where a tap touches the drawing: on its shape at a mouse's reach (else, a shape that is only a text — « Pas de FVG
/// ici » —, at a finger's), the place closest to its first point (where a person touches it).
fn sur_le_dessin(a: &App) -> Option<(f64, f64)> {
    let f = a.frame_now()?;
    let g = f.geo();
    let d = a.chart.drawings.first()?;
    let h = d.handles(&g);
    let (x0, y0) = *h.first()?;
    let (pw, ph) = (a.chart.viewport.plot_w(), a.chart.viewport.plot_h());
    let mut essais: Vec<(f64, f64)> = h.clone();
    for p in h.windows(2) {
        for k in 1..8 {
            let s = k as f64 / 8.0;
            essais.push((p[0].0 + (p[1].0 - p[0].0) * s, p[0].1 + (p[1].1 - p[0].1) * s));
        }
    }
    // (the whole chart, closest to the first point first: a regression's line is not where its points are)
    let mut grille: Vec<(f64, f64)> = (4..pw as i32 - 4)
        .step_by(3)
        .flat_map(|x| (4..ph as i32 - 4).step_by(3).map(move |y| (x as f64, y as f64)))
        .collect();
    grille.sort_by(|p, q| (p.0 - x0).hypot(p.1 - y0).total_cmp(&(q.0 - x0).hypot(q.1 - y0)));
    essais.extend(grille);
    let dedans = |&(x, y): &(f64, f64)| x > 2.0 && y > 2.0 && x < pw - 2.0 && y < ph - 2.0;
    let touche = |tol: f64| essais.iter().copied().filter(dedans).find(|&(x, y)| d.hit_r(x, y, &g, tol));
    touche(argos_render::drawings::HIT_PX).or_else(|| touche(argos_render::drawings::HIT_DOIGT))
}

/// The drawing changes the chart's picture (a Fair Value Gap, an order block… is drawn only where the candles have
/// one; the test's candles may have none where it is laid).
fn se_voit(a: &App) -> bool {
    let avec = argos_render::render(&a.chart_with(true));
    let mut c = a.chart_with(true);
    c.drawings.clear();
    let sans = argos_render::render(&c);
    avec.data() != sans.data()
}

/// Every tool laid, touched, moved, stretched and opened by the finger on `a` (laid out, by the finger); the faults
/// found, one line each.
fn chaque_outil(a: &mut App, ecran_nom: &str) -> Vec<String> {
    let mut fautes = vec![];
    let (pw, ph) = (a.chart.viewport.plot_w(), a.chart.viewport.plot_h());
    let at = |fx: f64, fy: f64| (pw * fx, ph * fy);
    let places = [
        at(0.25, 0.3),
        at(0.55, 0.55),
        at(0.7, 0.35),
        at(0.4, 0.7),
        at(0.8, 0.6),
        at(0.3, 0.5),
        at(0.6, 0.25),
    ];
    for t in argos_render::tools::TOOLS {
        // (0: an action; 99: as many points as wanted, a double tap ends it; the loupe and the capture: actions)
        if !(1..=7).contains(&t.points) || matches!(t.id, "loupe" | "aicapture") {
            continue;
        }
        let quoi = format!("{ecran_nom} « {} » ({})", t.label, t.id);
        a.chart.drawings.clear();
        (a.selected, a.draft, a.ui.obj_dlg, a.ui.plein_ecran, a.ui.croix) = (None, None, None, None, None);
        a.chart.settings.scale.manual = None;
        pause(a);
        a.set_tool(t.id);
        // (🙂 the icon: its emoji chosen in the grid first)
        a.ui.pop = None;
        let avant = vue(a);
        // 1. laid as it is drawn: a drag for its first two points, a tap for each further one
        let n = t.points as usize;
        if n == 1 {
            tape(a, places[0]);
        } else {
            doigt(a, &[places[0], places[1]]);
            for &p in &places[2..n] {
                pause(a);
                tape(a, p);
            }
        }
        if a.draft.is_some() || a.chart.drawings.len() != 1 {
            fautes.push(format!(
                "{quoi}: not laid by the finger ({} drawings, unfinished: {})",
                a.chart.drawings.len(),
                a.draft.is_some()
            ));
            continue;
        }
        if a.tool != "cursor" {
            fautes.push(format!("{quoi}: the tool stays chosen after it is laid"));
        }
        let f = a.frame_now().unwrap();
        let d = &a.chart.drawings[0];
        if n >= 2 && !argos_render::position::is_position(t.id) {
            let (pa, pb) = (f.snap(places[0].0, places[0].1, a.rail.magnet), f.snap(places[1].0, places[1].1, a.rail.magnet));
            let pres = |q: &argos_render::drawings::Point, p: &argos_render::drawings::Point| q.t == p.t && (q.p - p.p).abs() < 1e-6;
            if !(pres(&d.pts[0], &pa) && pres(&d.pts[1], &pb)) {
                fautes.push(format!("{quoi}: its first two points are not where the finger went down and lifted"));
            }
        }
        if vue(a) != avant {
            fautes.push(format!("{quoi}: laying it moved the chart: {avant} → {}", vue(a)));
        }
        // 2. a tap on it selects it
        let Some(sur) = sur_le_dessin(a) else {
            if se_voit(a) {
                fautes.push(format!("{quoi}: drawn, but no place where a tap touches it"));
            } else {
                // (made from the candles: none here — nothing on the screen to touch)
                eprintln!("{quoi}: nothing drawn with the test's candles where it is laid");
            }
            continue;
        };
        a.selected = None;
        pause(a);
        tape(a, sur);
        if a.selected != Some(0) {
            fautes.push(format!("{quoi}: a tap on it at {sur:?} does not select it"));
            continue;
        }
        // (`ARGOS_CAPTURES=<folder>`: each tool laid by the finger and selected, the chart in a PNG, to look at)
        if let Ok(d) = std::env::var("ARGOS_CAPTURES") {
            let png = std::path::Path::new(&d).join(format!("dessin-{ecran_nom}-{}.png", t.id));
            argos_render::render(&a.chart_with(true)).save_png(png).unwrap();
        }
        // 3. a drag on it (selected) moves it, not the chart; right after the tap too (no double tap)
        let avant_pts = a.chart.drawings[0].pts.clone();
        let avant = vue(a);
        doigt(a, &[sur, (sur.0 + 24.0, sur.1 + 36.0)]);
        if a.chart.drawings[0].pts == avant_pts {
            fautes.push(format!("{quoi}: a drag right after the tap does not move it"));
        }
        if vue(a) != avant || a.ui.obj_dlg.is_some() {
            fautes.push(format!("{quoi}: the drag moved the chart or opened its settings: {avant} → {}", vue(a)));
            a.ui.obj_dlg = None;
        }
        if a.selected != Some(0) {
            fautes.push(format!("{quoi}: moved, it is no longer selected"));
            a.selected = Some(0);
        }
        // 4. a handle touched a finger's width off (11 points) stretches it
        pause(a);
        let f = a.frame_now().unwrap();
        let h = *a.chart.drawings[0].handles(&f.geo()).last().unwrap();
        let depart = (h.0 + 9.0, h.1 + 6.0);
        let avant_pts = a.chart.drawings[0].pts.clone();
        let avant = vue(a);
        doigt(a, &[depart, (depart.0 - 30.0, depart.1 + 26.0)]);
        if a.chart.drawings[0].pts == avant_pts {
            fautes.push(format!("{quoi}: its handle touched 11 points off does not stretch it"));
        }
        if vue(a) != avant {
            fautes.push(format!("{quoi}: stretching it moved the chart: {avant} → {}", vue(a)));
        }
        // 5. its settings: a double tap on it, and the ⚙ of its bar
        pause(a);
        let f = a.frame_now().unwrap();
        let h0 = a.chart.drawings[0].handles(&f.geo())[0];
        let avant = vue(a);
        tape(a, h0);
        tape(a, h0);
        if a.ui.obj_dlg.is_none() {
            fautes.push(format!("{quoi}: a double tap on it does not open its settings"));
        }
        if vue(a) != avant {
            fautes.push(format!("{quoi}: the double tap on it went to the chart: {avant} → {}", vue(a)));
        }
        a.ui.obj_dlg = None;
        a.ui.plein_ecran = None;
        a.selected = Some(0);
        pause(a);
        match a.drawbar().and_then(|(_, b)| b.at(argos_render::drawbar::DbBtn::Settings)) {
            Some(r) => {
                tape(a, (r.cx(), r.cy()));
                if a.ui.obj_dlg.is_none() {
                    fautes.push(format!("{quoi}: the ⚙ of its bar does not open its settings"));
                }
            }
            None => fautes.push(format!("{quoi}: no ⚙ in its bar")),
        }
        a.ui.obj_dlg = None;
    }
    fautes
}

/// ✏️ B-019 on the phone (iPhone 402 × 874, the single pill): every drawing tool by the finger.
#[test]
fn every_drawing_tool_by_the_finger_on_the_phone() {
    dossier_a_soi("dessins_au_doigt_cell");
    let mut a = cell_pilule();
    a.draw();
    argos_render::actions::set_tactile(true);
    let fautes = chaque_outil(&mut a, "phone");
    argos_render::actions::set_tactile(false);
    assert!(fautes.is_empty(), "{} faults:\n{}", fautes.len(), fautes.join("\n"));
}

/// ✏️ B-019 on the tablet (iPad 1024 × 1260 upright, by the finger): every drawing tool by the finger.
#[test]
fn every_drawing_tool_by_the_finger_on_the_tablet() {
    dossier_a_soi("dessins_au_doigt_tablette");
    let mut a = ouverte(&ecran(1024.0, 1260.0, 2.0), false);
    a.draw();
    argos_render::actions::set_tactile(true);
    let fautes = chaque_outil(&mut a, "tablet");
    argos_render::actions::set_tactile(false);
    assert!(fautes.is_empty(), "{} faults:\n{}", fautes.len(), fautes.join("\n"));
}
