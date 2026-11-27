//! 📈 Séries en ligne et aires HLC (types de chart sans bougies), et le petit trait des barres OHLC.

use crate::*;

/// Ligne / aire / ligne de base à travers les points (x, y des clôtures). `base_y` = niveau de la ligne de base (pixels).
pub(crate) fn draw_line_series(pm: &mut Pixmap, pts: &[(f32, f32)], style: &CandleStyle, base_y: f32, ph: f32) {
    let Some(&(x0, y0)) = pts.first() else { return };
    let mut pb = PathBuilder::new();
    pb.move_to(x0, y0);
    for k in 1..pts.len() {
        if *style == CandleStyle::StepLine {
            pb.line_to(pts[k].0, pts[k - 1].1);
        }
        pb.line_to(pts[k].0, pts[k].1);
    }
    let Some(line) = pb.finish() else { return };
    let (bull, bear) = (Rgb::hex("#089981"), Rgb::hex("#f23645"));
    let b = (base_y / ph).clamp(0.0, 1.0);
    let filled_to = |y_end: f32| {
        let mut pb = PathBuilder::new();
        pb.move_to(x0, y_end);
        for &(x, y) in pts {
            pb.line_to(x, y);
        }
        pb.line_to(pts[pts.len() - 1].0, y_end);
        pb.close();
        pb.finish()
    };
    let stroke = Stroke {
        width: 2.0,
        ..Stroke::default()
    };
    match style {
        CandleStyle::Area => {
            if let Some(a) = filled_to(ph) {
                let top = pts.iter().map(|p| p.1).fold(f32::INFINITY, f32::min);
                let g = gradient(top, ph, &[(0.0, LINE_BLUE, 0.28), (1.0, LINE_BLUE, 0.05)]);
                pm.fill_path(&a, &g, tiny_skia::FillRule::Winding, crate::px::t(), None);
            }
            pm.stroke_path(
                &line,
                &gradient(0.0, ph, &[(0.0, LINE_BLUE, 1.0), (1.0, LINE_BLUE, 1.0)]),
                &stroke,
                crate::px::t(),
                None,
            );
        }
        CandleStyle::Baseline => {
            // dégradés coupés net au niveau de base : vert au-dessus, rouge en dessous
            let e = (b + 1e-4).min(1.0);
            if let Some(a) = filled_to(base_y) {
                let g = gradient(0.0, ph, &[(0.0, bull, 0.28), (b, bull, 0.05), (e, bear, 0.05), (1.0, bear, 0.28)]);
                pm.fill_path(&a, &g, tiny_skia::FillRule::Winding, crate::px::t(), None);
            }
            let g = gradient(0.0, ph, &[(0.0, bull, 1.0), (b, bull, 1.0), (e, bear, 1.0), (1.0, bear, 1.0)]);
            pm.stroke_path(&line, &g, &stroke, crate::px::t(), None);
        }
        _ => {
            pm.stroke_path(
                &line,
                &gradient(0.0, ph, &[(0.0, LINE_BLUE, 1.0), (1.0, LINE_BLUE, 1.0)]),
                &stroke,
                crate::px::t(),
                None,
            );
        }
    }
    // marqueurs : seulement si les points sont assez espacés pour être lisibles
    let room = pts.len() > 1 && pts[1].0 - pts[0].0 >= 6.0;
    if *style == CandleStyle::LineMarkers && room {
        let paint = gradient(0.0, ph, &[(0.0, LINE_BLUE, 1.0), (1.0, LINE_BLUE, 1.0)]);
        for &(x, y) in pts {
            if let Some(dot) = PathBuilder::from_circle(x, y, 3.0) {
                pm.fill_path(&dot, &paint, tiny_skia::FillRule::Winding, crate::px::t(), None);
            }
        }
    }
}

/// Point d'aire HLC : (x, y haut, y clôture, y bas).
pub(crate) type Hlc = (f32, f32, f32, f32);

/// Choisit une des lignes d'un point HLC.
pub(crate) type Pick = fn(&Hlc) -> f32;

/// Aire HLC (couleurs TradingView) : `pts` = (x, y haut, y clôture, y bas).
pub(crate) fn draw_hlc_area(pm: &mut Pixmap, pts: &[Hlc], ph: f32) {
    if pts.is_empty() {
        return;
    }
    let (green, red, gray) = (Rgb::hex("#089981"), Rgb::hex("#f23645"), Rgb::hex("#868993"));
    let solid = |c: Rgb, a: f32| gradient(0.0, ph, &[(0.0, c, a), (1.0, c, a)]);
    // zone entre deux lignes : aller sur la première, retour sur la seconde
    let band = |a: Pick, b: Pick| {
        let mut pb = PathBuilder::new();
        pb.move_to(pts[0].0, a(&pts[0]));
        for p in &pts[1..] {
            pb.line_to(p.0, a(p));
        }
        for p in pts.iter().rev() {
            pb.line_to(p.0, b(p));
        }
        pb.close();
        pb.finish()
    };
    let line = |f: Pick| {
        let mut pb = PathBuilder::new();
        pb.move_to(pts[0].0, f(&pts[0]));
        for p in &pts[1..] {
            pb.line_to(p.0, f(p));
        }
        pb.finish()
    };
    let (hi, cl, lo): (Pick, Pick, Pick) = (|p| p.1, |p| p.2, |p| p.3);
    if let Some(a) = band(hi, cl) {
        pm.fill_path(&a, &solid(green, 0.2), tiny_skia::FillRule::Winding, crate::px::t(), None);
    }
    if let Some(a) = band(cl, lo) {
        pm.fill_path(&a, &solid(red, 0.2), tiny_skia::FillRule::Winding, crate::px::t(), None);
    }
    let stroke = Stroke {
        width: 2.0,
        ..Stroke::default()
    };
    for (f, c) in [(hi, green), (lo, red), (cl, gray)] {
        if let Some(l) = line(f) {
            pm.stroke_path(&l, &solid(c, 1.0), &stroke, crate::px::t(), None);
        }
    }
}

/// Longueur (px) des petits traits d'ouverture / clôture d'une barre OHLC : 30 % de l'espace, impaire, 1 minimum (règle TradingView).
pub(crate) fn bar_tick(sp: f64) -> f64 {
    let w = (0.3 * sp).floor().max(1.0);
    if w >= 2.0 && w % 2.0 == 0.0 {
        w - 1.0
    } else {
        w
    }
}
