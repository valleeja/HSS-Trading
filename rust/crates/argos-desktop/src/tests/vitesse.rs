//! 🏎 The speed guard (fluidité, lot A — Jee, 24 Nov 2026: « ça fait 4x qu'on fait ça », the app slowed down again
//! by a lot that nobody timed). Two screens of the cell are drawn as on Jee's iPhone: the chart dragged by the finger
//! (×1.2, the scale while a finger moves) and the market list open in glass (×2.4, at rest). For each:
//! - its time per image, divided by the time of a fixed drawing that never changes (the yardstick, tiny-skia alone),
//!   so a busy or slow machine does not matter;
//! - the pixels laid one over another per image (layers, glass): an exact count, the same on every machine.
//!
//! Both are compared with `tests/references/vitesse.txt`: more than 25 % slower, or more than 25 % more pixels laid,
//! and make check is red — the lot is refused. The reference is written again only with Jee's phrase
//! (`ARGOS_VITESSE=maj`, like the safety net's `ARGOS_REFERENCES=maj`), on Jee's PC, where make check runs.

use super::*;
use argos_render::indicators::Ind;
use std::path::{Path, PathBuf};
use winit::event::TouchPhase;

/// Slower than this (or more pixels laid) than the reference: red.
const MARGE: f64 = 1.25;

/// Rounds of each measure: the fastest is kept (another test running beside it only slows some rounds).
const TOURS: usize = 7;

fn reference_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("references").join("vitesse.txt")
}

/// The reference is being written again (`ARGOS_VITESSE=maj`), not compared.
fn mise_a_jour() -> bool {
    std::env::var("ARGOS_VITESSE").is_ok_and(|v| v == "maj")
}

/// 📏 The yardstick: a fixed drawing with tiny-skia alone (shapes with smooth edges, lines, a half-clear image laid
/// over), none of the app's code. Returns its time in ms.
fn etalon_ms() -> f64 {
    use tiny_skia::{Color, FillRule, Paint, PathBuilder, Pixmap, PixmapPaint, Stroke, Transform};
    let t = std::time::Instant::now();
    let mut pm = Pixmap::new(900, 900).unwrap();
    pm.fill(Color::from_rgba8(12, 14, 20, 255));
    let mut paint = Paint {
        anti_alias: true,
        ..Default::default()
    };
    for k in 0..240u32 {
        let (x, y, r) = ((k * 37 % 900) as f32, (k * 53 % 900) as f32, 6.0 + (k % 9) as f32 * 5.0);
        paint.set_color_rgba8((k * 7) as u8, (k * 13) as u8, (k * 29) as u8, 200);
        if let Some(c) = PathBuilder::from_circle(x, y, r) {
            pm.fill_path(&c, &paint, FillRule::Winding, Transform::identity(), None);
        }
    }
    let trait_ = Stroke {
        width: 2.0,
        ..Default::default()
    };
    for k in 0..60u32 {
        let mut pb = PathBuilder::new();
        pb.move_to(0.0, (k * 15) as f32);
        for i in 1..60u32 {
            pb.line_to((i * 15) as f32, (k * 15 + (i * k * 7) % 40) as f32);
        }
        paint.set_color_rgba8(200, (k * 4) as u8, 90, 255);
        if let Some(p) = pb.finish() {
            pm.stroke_path(&p, &paint, &trait_, Transform::identity(), None);
        }
    }
    let mut calque = Pixmap::new(700, 700).unwrap();
    calque.fill(Color::from_rgba8(40, 60, 90, 120));
    for k in 0..4 {
        pm.draw_pixmap(k * 50, k * 40, calque.as_ref(), &PixmapPaint::default(), Transform::identity(), None);
    }
    std::hint::black_box(pm);
    t.elapsed().as_secs_f64() * 1000.0
}

/// 📱 The phone as Jee's iPhone shows it (390 × 844 points), glass on, Argos Vague and the RSI.
fn telephone() -> App {
    let mut a = cell_390();
    a.watch.phone = Some(390);
    a.ui.form.kind = argos_render::formfactor::Kind::Phone;
    a.ui.taille_test = Some(winit::dpi::PhysicalSize::new(390, 844));
    a.ui.frames = 5;
    a.top.prefs.glass = true;
    a.ind.on = vec![Ind::ArgosVague, Ind::Rsi];
    a.refresh_indicators();
    argos_render::actions::set_tactile(true);
    argos_render::px::set_scale(2.4);
    a.draw();
    assert!(a.top.phone, "the phone's layout");
    a
}

/// One screen measured: ms per image and the yardstick's ms (the fastest rounds of each), pixels laid per image.
struct Mesure {
    ms: f64,
    etalon: f64,
    pixels: u64,
}

/// `images` images drawn by `image` (each draws once), [`TOURS`] rounds after one to warm up; the yardstick is drawn
/// before each round, so both see the machine as busy as it is at that moment.
fn mesurer(a: &mut App, images: usize, image: &mut dyn FnMut(&mut App, usize)) -> Mesure {
    for k in 0..images {
        image(a, k);
    }
    let (mut ms, mut etalon, mut pixels) = (f64::INFINITY, f64::INFINITY, 0);
    for _ in 0..TOURS {
        etalon = etalon.min(etalon_ms());
        let p0 = argos_render::px::pixels_poses();
        let t = std::time::Instant::now();
        for k in 0..images {
            image(a, k);
        }
        ms = ms.min(t.elapsed().as_secs_f64() * 1000.0 / images as f64);
        pixels = (argos_render::px::pixels_poses() - p0) / images as u64;
    }
    Mesure { ms, etalon, pixels }
}

/// 👆 The chart dragged by the finger: put down, then 6 points to the left each image (×1.2, as on the iPhone while a
/// finger moves); the finger lifted after each round, nothing left over.
fn glisser() -> Mesure {
    let mut a = telephone();
    argos_render::px::set_scale(1.2);
    let (x, y) = (220.0, a.top_h() + 160.0);
    let m = mesurer(&mut a, 16, &mut |a, k| {
        if k == 0 {
            // (put down — never the 2nd half of a double tap — then past the finger's slack: the drag has begun)
            a.pointer.last_chart_click = None;
            a.touch(5, TouchPhase::Started, (x, y));
            a.touch(5, TouchPhase::Moved, (x - 20.0, y));
        }
        a.touch(5, TouchPhase::Moved, (x - 20.0 - 6.0 * (k + 1) as f64, y));
        assert!(a.geste_en_cours(), "the finger drags the chart (image {k})");
        a.draw();
        if k == 15 {
            a.touch(5, TouchPhase::Ended, (x - 116.0, y));
        }
    });
    argos_render::px::set_scale(1.0);
    argos_render::actions::set_tactile(false);
    m
}

/// 🔎 The market list open in glass over the chart, at rest (×2.4), drawn again and again.
fn liste() -> Mesure {
    let mut a = telephone();
    a.marches_ouvrir(Rect::default(), true);
    assert!(a.ui.marches.is_some(), "the market list is open");
    let m = mesurer(&mut a, 8, &mut |a, _| a.draw());
    argos_render::px::set_scale(1.0);
    argos_render::actions::set_tactile(false);
    m
}

/// 🏎 The speed guard: dragging the chart and the market list on the cell stay within 25 % of their reference, in
/// time (divided by the yardstick) and in pixels laid. See the top of this file.
#[test]
fn the_cell_stays_as_fast_as_its_reference() {
    let ecrans = [("glisser", glisser()), ("liste", liste())];
    let lignes: Vec<String> = ecrans.iter().map(|(nom, m)| format!("{nom} {:.3} {}", m.ms / m.etalon, m.pixels)).collect();
    let resume: String = ecrans
        .iter()
        .map(|(nom, m)| {
            format!(
                "{nom}: {:.1} ms par image, étalon {:.1} ms, {:.3} étalons, {} pixels posés\n",
                m.ms,
                m.etalon,
                m.ms / m.etalon,
                m.pixels
            )
        })
        .collect();
    eprintln!("🏎 {resume}");
    let chemin = reference_path();
    let texte = std::fs::read_to_string(&chemin).ok();
    if mise_a_jour() || texte.is_none() {
        let mut t = String::from(
            "# Written by the test `vitesse::the_cell_stays_as_fast_as_its_reference` with ARGOS_VITESSE=maj (Jee's \
             phrase), never by hand.\n# screen, time per image in yardsticks, pixels laid per image\n",
        );
        for l in &lignes {
            t.push_str(l);
            t.push('\n');
        }
        std::fs::write(&chemin, t).expect("the speed reference written");
        return;
    }
    let reference: std::collections::HashMap<String, (f64, u64)> = texte
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.starts_with('#'))
        .filter_map(|l| {
            let mut c = l.split_whitespace();
            Some((c.next()?.to_string(), (c.next()?.parse().ok()?, c.next()?.parse().ok()?)))
        })
        .collect();
    let mut rouge = vec![];
    for (nom, m) in &ecrans {
        let Some(&(r_ms, r_px)) = reference.get(*nom) else {
            rouge.push(format!("{nom}: pas de référence (ARGOS_VITESSE=maj, avec la phrase de Jee)"));
            continue;
        };
        let ratio = m.ms / m.etalon;
        if ratio > r_ms * MARGE {
            rouge.push(format!(
                "{nom}: {:.0} % plus lent que la référence ({ratio:.3} étalons au lieu de {r_ms:.3})",
                (ratio / r_ms - 1.0) * 100.0
            ));
        } else if ratio < r_ms / MARGE {
            eprintln!(
                "🏎 {nom}: {:.0} % plus rapide que la référence — à écrire avec la phrase de Jee",
                (1.0 - ratio / r_ms) * 100.0
            );
        }
        if m.pixels as f64 > r_px as f64 * MARGE {
            rouge.push(format!("{nom}: {} pixels posés par image au lieu de {r_px} (un calque de trop ?)", m.pixels));
        }
    }
    assert!(
        rouge.is_empty(),
        "🏎 L'app ralentit sur cell :\n{}\nUn ralentissement voulu : ARGOS_VITESSE=maj, seulement avec la phrase de Jee.",
        rouge.join("\n")
    );
}

/// ✏️ (B) While a finger drags the chart, the chart's shapes are a draft (no smooth edges: twice as fast), the bars
/// around it stay sharp; the finger lifted,
/// the next image is sharp again — the very image of a chart never touched (nothing of the draft is kept). The mouse
/// never draws a draft.
#[test]
fn draft_while_the_finger_moves_sharp_at_rest() {
    let mut a = telephone();
    a.draw();
    let net = std::mem::take(&mut a.ui.image_test);
    assert!(!argos_render::px::brouillon(), "at rest: sharp");
    let (x, y) = (220.0, a.top_h() + 160.0);
    a.pointer.last_chart_click = None;
    a.touch(7, TouchPhase::Started, (x, y));
    a.touch(7, TouchPhase::Moved, (x - 20.0, y));
    a.touch(7, TouchPhase::Moved, (x - 30.0, y));
    assert!(a.geste_en_cours(), "the finger drags the chart");
    a.draw();
    assert!(a.brouillon_du_doigt(), "a finger moves the chart: a draft");
    assert!(!argos_render::px::brouillon(), "the draft ends with the chart: the bars around it are sharp");
    let brouillon = std::mem::take(&mut a.ui.image_test);
    // (back where it went down, then lifted: the same view as before)
    a.touch(7, TouchPhase::Moved, (x, y));
    a.touch(7, TouchPhase::Ended, (x, y));
    a.draw();
    assert!(!a.brouillon_du_doigt(), "the finger lifted: sharp again");
    let apres = std::mem::take(&mut a.ui.image_test);
    assert!(brouillon != net, "the draft is another image");
    assert!(apres == net, "at rest again: the very image drawn before the finger, nothing of the draft kept");
    argos_render::actions::set_tactile(false);
    argos_render::px::set_scale(1.0);

    // 🖱 the mouse dragging the chart (PC): never a draft
    let mut a = app();
    let (wx, wy) = (300.0 + a.rw(), 200.0 + a.top_h());
    a.pointer.last_chart_click = None;
    a.mouse_moved(winit::dpi::PhysicalPosition::new(wx, wy));
    a.left_button(winit::event::ElementState::Pressed);
    a.mouse_moved(winit::dpi::PhysicalPosition::new(wx + 40.0, wy));
    assert!(a.geste_en_cours());
    a.draw();
    assert!(!a.brouillon_du_doigt(), "the mouse: no draft");
    a.left_button(winit::event::ElementState::Released);

    // 👆 a finger scrolling the market list (the chart does not move): no draft
    let mut a = telephone();
    a.marches_ouvrir(Rect::default(), true);
    let r = a.ui.marches.as_ref().map(|m| m.rect).expect("the market list is open");
    let (x, y) = (r.x + r.w / 2.0, r.y + r.h * 0.6);
    a.touch(8, TouchPhase::Started, (x, y));
    a.touch(8, TouchPhase::Moved, (x, y - 30.0));
    a.touch(8, TouchPhase::Moved, (x, y - 60.0));
    a.draw();
    assert!(!a.brouillon_du_doigt(), "a finger scrolling a list: no draft");
    a.touch(8, TouchPhase::Ended, (x, y - 60.0));
    argos_render::actions::set_tactile(false);
    argos_render::px::set_scale(1.0);
}

// 🧮 Counted work (Jee, 25 Nov 2026: « je veux 1 % », « sinon on peut les accumuler vite »). A time changes from one
// run to the next (up to 9 % in a browser, more on a busy computer); the number of instructions the processor runs to
// draw the same images does not. `make porte` runs each `work_*` test twice under valgrind (tools/porte/travail.sh:
// cachegrind counting the instructions), with ARGOS_TRAVAIL_IMAGES=N then 2N: the difference is the exact work of N
// images of the gesture. More than 1 % above its reference (tools/porte/travail.txt) is red, and the reference only
// ever goes down (a ratchet): small slowdowns never add up. Run alone (make check), each draws a few images and checks
// that the gesture really ran.

/// Images drawn by a `work_*` test (ARGOS_TRAVAIL_IMAGES, 3 when it is not given).
fn images_comptees() -> usize {
    std::env::var("ARGOS_TRAVAIL_IMAGES").ok().and_then(|v| v.parse().ok()).unwrap_or(3)
}

/// 👆 The chart of the cell dragged by a finger (×1.2, as on the iPhone while a finger moves).
#[test]
fn work_cell_drag() {
    let mut a = telephone();
    argos_render::px::set_scale(1.2);
    let (x, y) = (220.0, a.top_h() + 160.0);
    a.pointer.last_chart_click = None;
    a.touch(5, TouchPhase::Started, (x, y));
    a.touch(5, TouchPhase::Moved, (x - 20.0, y));
    let n = images_comptees();
    for k in 0..n {
        a.touch(5, TouchPhase::Moved, (x - 20.0 - 4.0 * (k + 1) as f64, y));
        assert!(a.geste_en_cours(), "the finger drags the chart (image {k})");
        a.draw();
    }
    a.touch(5, TouchPhase::Ended, (x - 20.0 - 4.0 * n as f64, y));
    argos_render::px::set_scale(1.0);
    argos_render::actions::set_tactile(false);
}

/// 🤏 The chart of the cell pinched by two fingers (×1.2).
#[test]
fn work_cell_pinch() {
    let mut a = telephone();
    argos_render::px::set_scale(1.2);
    let (cx, y) = (200.0, a.top_h() + 180.0);
    a.pointer.last_chart_click = None;
    a.touch(5, TouchPhase::Started, (cx - 40.0, y));
    a.touch(6, TouchPhase::Started, (cx + 40.0, y));
    let n = images_comptees();
    for k in 0..n {
        let d = 40.0 + 3.0 * (k + 1) as f64;
        a.touch(5, TouchPhase::Moved, (cx - d, y));
        a.touch(6, TouchPhase::Moved, (cx + d, y));
        assert!(a.pointer.touch.pince(), "two fingers pinch the chart (image {k})");
        a.draw();
    }
    let d = 40.0 + 3.0 * n as f64;
    a.touch(6, TouchPhase::Ended, (cx + d, y));
    a.touch(5, TouchPhase::Ended, (cx - d, y));
    argos_render::px::set_scale(1.0);
    argos_render::actions::set_tactile(false);
}

/// 🔎 The market list of the cell open in glass, at rest (×2.4), drawn again and again.
#[test]
fn work_cell_market_list() {
    let mut a = telephone();
    a.marches_ouvrir(Rect::default(), true);
    assert!(a.ui.marches.is_some(), "the market list is open");
    for _ in 0..images_comptees() {
        a.draw();
    }
    argos_render::px::set_scale(1.0);
    argos_render::actions::set_tactile(false);
}

/// 🖱 The chart of the PC dragged by the mouse.
#[test]
fn work_pc_drag() {
    let mut a = app();
    let (wx, wy) = (300.0 + a.rw(), 200.0 + a.top_h());
    a.pointer.last_chart_click = None;
    a.mouse_moved(winit::dpi::PhysicalPosition::new(wx, wy));
    a.left_button(winit::event::ElementState::Pressed);
    a.mouse_moved(winit::dpi::PhysicalPosition::new(wx + 20.0, wy));
    let n = images_comptees();
    for k in 0..n {
        a.mouse_moved(winit::dpi::PhysicalPosition::new(wx + 20.0 + 4.0 * (k + 1) as f64, wy));
        assert!(a.geste_en_cours(), "the mouse drags the chart (image {k})");
        a.draw();
    }
    a.left_button(winit::event::ElementState::Released);
}
