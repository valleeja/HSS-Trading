//! 📸 Image d'une idée de trade : la chart déjà dessinée (`render`), avec les lignes entrée / SL / TP et une légende
//! (marché, unité, sens, R:R, « Argos HSS Trading »), réduite à `max_w` pixels de large au plus, puis en PNG.

use crate::text::{Align, Rgb, Text};
use tiny_skia::{Color, FilterQuality, Paint, PathBuilder, Pixmap, PixmapPaint, Rect, Stroke, StrokeDash, Transform};

/// Une ligne de prix à tracer : hauteur (points), couleur, étiquette.
pub struct Mark {
    pub y: f64,
    pub color: Rgb,
    pub label: String,
}

fn paint(c: Rgb, a: u8) -> Paint<'static> {
    let mut p = Paint::default();
    p.set_color(Color::from_rgba8(c.0, c.1, c.2, a));
    p.anti_alias = true;
    p
}

/// Ajoute les lignes et la légende sur `pm` (zone des prix : `plot_w` × `plot_h` points), réduit et encode en PNG.
pub fn idea_png(mut pm: Pixmap, plot_w: f64, plot_h: f64, marks: &[Mark], caption: &str, note: &str, max_w: u32) -> Option<Vec<u8>> {
    let text = Text::default();
    let t = crate::px::t();
    for m in marks.iter().filter(|m| m.y.is_finite() && m.y >= 0.0 && m.y <= plot_h) {
        let mut pb = PathBuilder::new();
        pb.move_to(0.0, m.y as f32);
        pb.line_to(plot_w as f32, m.y as f32);
        if let Some(path) = pb.finish() {
            let stroke = Stroke {
                width: 1.5,
                dash: StrokeDash::new(vec![6.0, 4.0], 0.0),
                ..Default::default()
            };
            pm.stroke_path(&path, &paint(m.color, 230), &stroke, t, None);
        }
        let w = text.width(&m.label, 12.0) as f64 + 10.0;
        if let Some(r) = Rect::from_xywh(6.0, (m.y - 9.0) as f32, w as f32, 18.0) {
            pm.fill_rect(r, &paint(m.color, 235), t, None);
        }
        text.draw(&mut pm, &m.label, (11.0, (m.y + 4.0) as f32), 12.0, Rgb(255, 255, 255), Align::Left);
    }
    // légende en bas à gauche (le haut garde la légende de la chart), sur un fond sombre lisible quel que soit le thème
    let note = text.fit(note, 12.0, (plot_w - 40.0).max(60.0));
    let w = (text.width(caption, 14.0).max(text.width(&note, 12.0)) as f64 + 20.0).min(plot_w - 8.0);
    let h = if note.is_empty() { 28.0 } else { 46.0 };
    let top = (plot_h - h - 8.0).max(8.0) as f32;
    if let Some(r) = Rect::from_xywh(8.0, top, w.max(40.0) as f32, h as f32) {
        pm.fill_rect(r, &paint(Rgb(10, 12, 18), 215), t, None);
    }
    text.draw(&mut pm, caption, (18.0, top + 19.0), 14.0, Rgb(255, 255, 255), Align::Left);
    if !note.is_empty() {
        text.draw(&mut pm, &note, (18.0, top + 37.0), 12.0, Rgb(200, 205, 215), Align::Left);
    }
    shrink(&pm, max_w)?.encode_png().ok()
}

/// Même image, au plus `max_w` pixels de large (proportions gardées).
fn shrink(pm: &Pixmap, max_w: u32) -> Option<Pixmap> {
    if pm.width() <= max_w {
        return Some(pm.clone());
    }
    let k = max_w as f32 / pm.width() as f32;
    let mut out = Pixmap::new(max_w, ((pm.height() as f32 * k).round() as u32).max(1))?;
    let pp = PixmapPaint {
        quality: FilterQuality::Bicubic,
        ..Default::default()
    };
    out.draw_pixmap(0, 0, pm.as_ref(), &pp, Transform::from_scale(k, k), None);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_reduite_et_lignes_posees() {
        let pm = Pixmap::new(2000, 1000).unwrap();
        let marks = [Mark {
            y: 100.0,
            color: Rgb(0, 200, 0),
            label: "TP 90".into(),
        }];
        let png = idea_png(pm, 1900.0, 980.0, &marks, "▼ BTC 30m · SHORT · R:R 3.0 · Argos HSS Trading", "rejet", 1280).unwrap();
        let back = Pixmap::decode_png(&png).unwrap();
        assert_eq!((back.width(), back.height()), (1280, 640), "réduite à 1280 px, proportions gardées");
        assert!(back.pixels().iter().any(|p| p.green() > 150 && p.red() < 60), "la ligne TP est dessinée");
    }
}
