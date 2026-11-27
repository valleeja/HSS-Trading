//! 🔍 Échelle de rendu : l'interface est mise en page en points (pixels logiques) mais dessinée en pixels de l'écran,
//! `scale()` fois plus fins (DPR de l'iPhone : 3, écran Retina : 2 ; desktop : 1, rien ne change). Les images
//! (`new`) sont `scale()` fois plus grandes, les formes et le texte sont dessinés avec la même échelle ; les positions
//! et les clics restent en points.

use std::cell::Cell;
use tiny_skia::{Pixmap, Transform};

thread_local! {
    /// Échelle du fil qui dessine (le dessin se fait sur un seul fil ; chaque test a la sienne).
    static SCALE: Cell<f32> = const { Cell::new(1.0) };
    /// 🖼 What the kept background of the chart shows beyond the chart itself, set from outside it (the Explorer's
    /// lines of an address): a number that changes with it.
    static FOND_REVISION: Cell<usize> = const { Cell::new(0) };
    /// 🏎 Pixels laid by [`pose`] and the glass since the start (the speed guard's exact counter).
    static POSES: Cell<u64> = const { Cell::new(0) };
    /// ✏️ Draft drawing while a finger moves the chart (B): smooth edges off for shapes.
    static BROUILLON: Cell<bool> = const { Cell::new(false) };
}

/// ✏️ (B) Shapes drawn without smooth edges (a finger moves the chart: speed first; sharp again at rest).
pub fn brouillon() -> bool {
    BROUILLON.with(|b| b.get())
}

/// ✏️ See [`brouillon`].
pub fn set_brouillon(b: bool) {
    BROUILLON.with(|c| c.set(b));
}

/// 🏎 (lot A) Pixels laid one over another so far on this thread ([`pose`], the glass): the speed guard of make check
/// counts them per image — a layer of the whole screen added by mistake doubles them, whatever the machine.
pub fn pixels_poses() -> u64 {
    POSES.with(|p| p.get())
}

/// 🏎 `n` more pixels laid (see [`pixels_poses`]).
pub fn compter_pixels(n: u64) {
    POSES.with(|p| p.set(p.get().wrapping_add(n)));
}

/// 🖼 A layer of the chart set from outside it changed: a background kept with the number before is drawn again.
pub fn fond_change() {
    FOND_REVISION.with(|r| r.set(r.get().wrapping_add(1)));
}

/// 🖼 The number of [`fond_change`]: part of the key of the kept background.
pub fn fond_revision() -> usize {
    FOND_REVISION.with(|r| r.get())
}

/// Pixels de l'écran par point (0.5 à 4 ; under 1: the compact interface of the desktop app, `formfactor::compacte`).
pub fn scale() -> f32 {
    SCALE.with(|s| s.get())
}

/// Choisit l'échelle de rendu (bornée de 0.5 à 4).
pub fn set_scale(s: f32) {
    let s = if s.is_finite() { s.clamp(0.5, 4.0) } else { 1.0 };
    SCALE.with(|c| c.set(s));
}

/// Transformation des dessins (points → pixels).
pub fn t() -> Transform {
    let s = scale();
    Transform::from_scale(s, s)
}

/// Taille en pixels d'une longueur en points.
pub fn dev(v: f64) -> f64 {
    v * scale() as f64
}

/// 🔍 Sharpness: a rectangle (points) snapped to whole screen pixels. Its size comes from `w` and `h` alone (rounded,
/// at least 1 pixel), so equal widths stay equal wherever they sit (candles, grid, histogram bars never vanish).
pub fn snap_rect(x: f64, y: f64, w: f64, h: f64) -> (f64, f64, f64, f64) {
    let s = scale() as f64;
    let (x0, y0) = ((x * s).round(), (y * s).round());
    let (pw, ph) = ((w * s).round().max(1.0), (h * s).round().max(1.0));
    (x0 / s, y0 / s, pw / s, ph / s)
}

/// 🔍 Sharpness: where a straight line of `width` points crosses the pixel grid sharply — on a pixel's centre for an
/// odd number of screen pixels, on a pixel edge for an even one (moves it by half a screen pixel at most).
pub fn snap_line(v: f64, width: f32) -> f64 {
    let s = scale() as f64;
    let wd = (width as f64 * s).round().max(1.0);
    if wd as i64 % 2 == 1 {
        ((v * s).floor() + 0.5) / s
    } else {
        (v * s).round() / s
    }
}

/// Image de `w` × `h` points (donc `scale()` fois plus de pixels).
pub fn new(w: u32, h: u32) -> Option<Pixmap> {
    let s = scale();
    Pixmap::new(((w as f32 * s).ceil() as u32).max(1), ((h as f32 * s).ceil() as u32).max(1))
}

/// Largeur et hauteur d'une image, en points.
pub fn lw(pm: &Pixmap) -> f64 {
    pm.width() as f64 / scale() as f64
}
pub fn lh(pm: &Pixmap) -> f64 {
    pm.height() as f64 / scale() as f64
}

/// Pose `src` (déjà en pixels de l'écran) sur `dst` au point (`x`, `y`).
pub fn put(dst: &mut Pixmap, x: f64, y: f64, src: &Pixmap) {
    pose(dst, dev(x).round() as i32, dev(y).round() as i32, src.as_ref(), 1.0);
}

/// Comme `put`, `src` posé avec l'opacité `alpha` (0 à 1) : fond et lettres ensemble, comme `opacity` en CSS.
pub fn put_alpha(dst: &mut Pixmap, x: f64, y: f64, src: &Pixmap, alpha: f32) {
    pose(dst, dev(x).round() as i32, dev(y).round() as i32, src.as_ref(), alpha.clamp(0.0, 1.0));
}

/// 🚀 (fluidité, lot A) `src` laid over `dst` with its top left corner on the pixel (`x`, `y`), with the opacity
/// `alpha` (source over): the very pixels of tiny-skia's `draw_pixmap` (identity transform, no mask), without its
/// pipeline (a quarter of a frame while dragging the chart on the cell). Empty pixels are skipped, full ones copied,
/// the others get tiny-skia's arithmetic in f32. `pose_same_pixels_as_tiny_skia` checks every value.
pub fn pose(dst: &mut Pixmap, x: i32, y: i32, src: tiny_skia::PixmapRef, alpha: f32) {
    if let Some(tout) = tiny_skia::IntRect::from_xywh(0, 0, src.width(), src.height()) {
        pose_zone(dst, x, y, src, tout, alpha);
    }
}

/// 🚀 Like [`pose`], only the part `zone` of `src` (in its pixels, cut to its size, like `clone_rect`), its top left
/// corner on the pixel (`x`, `y`) of `dst`: the pixels of `clone_rect` then `draw_pixmap`, with no copy.
pub fn pose_zone(dst: &mut Pixmap, x: i32, y: i32, src: tiny_skia::PixmapRef, zone: tiny_skia::IntRect, alpha: f32) {
    let Some(zone) = tiny_skia::IntRect::from_xywh(0, 0, src.width(), src.height()).and_then(|t| t.intersect(&zone)) else {
        return;
    };
    let (dw, dh) = (dst.width() as i64, dst.height() as i64);
    if dw > TUILES || dh > TUILES {
        // (tiny-skia cuts so large an image in tiles: its own way then)
        compter_pixels(zone.width() as u64 * zone.height() as u64);
        if let Some(part) = src.clone_rect(zone) {
            let paint = tiny_skia::PixmapPaint {
                opacity: alpha,
                ..Default::default()
            };
            dst.draw_pixmap(x, y, part.as_ref(), &paint, Transform::identity(), None);
        }
        return;
    }
    // (tiny-skia's opacity: 0 to 1, nothing for NaN or the infinites)
    let c = if alpha.is_finite() { alpha.clamp(0.0, 1.0) } else { 0.0 };
    let (x, y, zw, zh) = (x as i64, y as i64, zone.width() as i64, zone.height() as i64);
    // (tiny-skia rounds a negative corner one pixel right or down — `saturate_round` truncates −27.5 to −27 — and fills
    // that extra column or row with the image's last one: the same here)
    let (rx, ry) = (x + (x < 0) as i64, y + (y < 0) as i64);
    let (x0, y0) = (rx.max(0), ry.max(0));
    let (x1, y1) = ((rx + zw).min(dw), (ry + zh).min(dh));
    if x0 >= x1 || y0 >= y1 {
        return;
    }
    compter_pixels(((x1 - x0) * (y1 - y0)) as u64);
    // (the columns read straight from the image, then at most one extra column repeating its last one)
    let x_fin = x1.min(x + zw);
    let n = (x_fin - x0).max(0) as usize * 4;
    let (sw, dwu) = (src.width() as usize, dw as usize);
    let sx = (x0 - x).min(zw - 1) as usize + zone.x() as usize;
    let s_der = (zone.x() as i64 + zw - 1) as usize;
    let sd = src.data();
    let dd = dst.data_mut();
    for yy in y0..y1 {
        let sy = (yy - y).min(zh - 1) as usize + zone.y() as usize;
        let ligne = (yy as usize * dwu) * 4;
        if n > 0 {
            let s = &sd[(sy * sw + sx) * 4..][..n];
            let d = &mut dd[ligne + x0 as usize * 4..][..n];
            over_row(d, s, c);
        }
        if x1 > x_fin {
            let s = &sd[(sy * sw + s_der) * 4..][..4];
            let d = &mut dd[ligne + x_fin.max(0) as usize * 4..][..4];
            over_row(d, s, c);
        }
    }
}

/// tiny-skia draws an image wider or taller than this in tiles (`DrawTiler`).
const TUILES: i64 = 8191;

/// One row of [`pose_zone`]: tiny-skia's highp `source_over_rgba` (load `u8 × (1/255)`, opacity `× c`,
/// `d × (1 − sa) + s`, store clamped `× 255` rounded half to even), pixel by pixel.
#[inline]
fn over_row(d: &mut [u8], s: &[u8], c: f32) {
    const F: f32 = 1.0 / 255.0;
    let unnorm = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round_ties_even() as u8;
    let vide = |p: &[u8; 4]| u32::from_ne_bytes(*p) == 0;
    let (d, s) = (d.as_chunks_mut::<4>().0, s.as_chunks::<4>().0);
    if c == 1.0 {
        for (dp, sp) in d.iter_mut().zip(s) {
            // (full source, or nothing under it: the source itself; empty source: nothing changes)
            if sp[3] == 255 || vide(dp) {
                dp.copy_from_slice(sp);
            } else if !vide(sp) {
                let inv = 1.0 - sp[3] as f32 * F;
                for k in 0..4 {
                    dp[k] = unnorm(dp[k] as f32 * F * inv + sp[k] as f32 * F);
                }
            }
        }
    } else {
        for (dp, sp) in d.iter_mut().zip(s) {
            if !vide(sp) {
                let inv = 1.0 - sp[3] as f32 * F * c;
                for k in 0..4 {
                    dp[k] = unnorm(dp[k] as f32 * F * inv + sp[k] as f32 * F * c);
                }
            }
        }
    }
}

/// Zone (x, y, w, h) en points → rectangle entier en pixels.
pub fn int_rect(x: f64, y: f64, w: f64, h: f64) -> Option<tiny_skia::IntRect> {
    tiny_skia::IntRect::from_xywh(
        dev(x).round() as i32,
        dev(y).round() as i32,
        dev(w).round().max(1.0) as u32,
        dev(h).round().max(1.0) as u32,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn echelle_points_pixels() {
        set_scale(3.0);
        let pm = new(100, 40).unwrap();
        assert_eq!((pm.width(), pm.height()), (300, 120));
        assert_eq!((lw(&pm), lh(&pm)), (100.0, 40.0));
        assert_eq!(dev(10.0), 30.0);
        assert_eq!(int_rect(1.0, 2.0, 3.0, 4.0).map(|r| (r.x(), r.y(), r.width(), r.height())), Some((3, 6, 9, 12)));
        set_scale(0.2);
        assert_eq!(scale(), 0.5, "never under 0.5");
        set_scale(f32::NAN);
        assert_eq!(scale(), 1.0);
        set_scale(1.0);
    }

    #[test]
    fn texte_et_formes_a_l_echelle() {
        let ink = |pm: &Pixmap, x0: u32, x1: u32, y0: u32, y1: u32| (y0..y1).any(|y| (x0..x1).any(|x| pm.pixel(x, y).is_some_and(|p| p.red() > 128)));
        for s in [1.0f32, 2.0] {
            set_scale(s);
            let mut pm = new(60, 30).unwrap();
            assert_eq!(pm.width(), (60.0 * s) as u32);
            crate::text::Text::default().draw(&mut pm, "H", (10.0, 20.0), 14.0, crate::text::Rgb(255, 255, 255), crate::text::Align::Left);
            crate::fill(&mut pm, 40.0, 5.0, 10.0, 10.0, crate::text::Rgb(255, 255, 255), 1.0);
            let k = |v: f32| (v * s) as u32;
            assert!(ink(&pm, k(10.0), k(22.0), k(8.0), k(21.0)), "lettre à sa place (échelle {s})");
            assert!(!ink(&pm, 0, k(9.0), 0, k(30.0)), "rien avant la lettre");
            assert!(
                ink(&pm, k(41.0), k(49.0), k(6.0), k(14.0)) && !ink(&pm, k(51.0), k(60.0), 0, k(30.0)),
                "carré à sa place"
            );
        }
        set_scale(1.0);
    }

    /// 🚀 (fluidité, lot A) `pose` gives the very pixels of tiny-skia's `draw_pixmap`: every (destination, source,
    /// source alpha) value of a channel, with several opacities.
    #[test]
    fn pose_same_pixels_as_tiny_skia() {
        use tiny_skia::{PixmapPaint, PremultipliedColorU8};
        let px = |r, g, b, a| PremultipliedColorU8::from_rgba(r, g, b, a).unwrap();
        for c in [1.0f32, 0.45, 0.5, 0.999, 0.003, 0.0, 2.0, -1.0, f32::NAN, f32::INFINITY] {
            for sa in 0..=255u8 {
                // (a row per source value s ≤ sa, a column per destination value d)
                let (w, h) = (256u32, sa as u32 + 1);
                let mut src = Pixmap::new(w, h).unwrap();
                for (i, p) in src.pixels_mut().iter_mut().enumerate() {
                    let s = (i / 256) as u8;
                    *p = px(s, s / 2, sa - s, sa);
                }
                for variante in 0..2 {
                    let mut a = Pixmap::new(w, h).unwrap();
                    for (i, p) in a.pixels_mut().iter_mut().enumerate() {
                        let d = (i % 256) as u8;
                        *p = if variante == 0 { px(d, 255 - d, d / 3, 255) } else { px(d, d, d / 2, d) };
                    }
                    let mut b = a.clone();
                    let paint = PixmapPaint {
                        opacity: c,
                        ..Default::default()
                    };
                    a.draw_pixmap(0, 0, src.as_ref(), &paint, Transform::identity(), None);
                    pose(&mut b, 0, 0, src.as_ref(), c);
                    assert!(a.data() == b.data(), "opacity {c}, source alpha {sa}, destination {variante}");
                }
            }
        }
    }

    /// 🚀 `pose` and `pose_zone` cut by every edge: the pixels of `draw_pixmap` (and of `clone_rect` then `draw_pixmap`).
    #[test]
    fn pose_cut_by_edges_same_pixels_as_tiny_skia() {
        use tiny_skia::{IntRect, PixmapPaint};
        let mut graine = 12345u32;
        let mut hasard = move || {
            graine = graine.wrapping_mul(1_103_515_245).wrapping_add(12345);
            (graine >> 16) as u8
        };
        let mut remplir = |pm: &mut Pixmap| {
            for p in pm.pixels_mut() {
                let a = match hasard() % 4 {
                    0 => 0,
                    1 => 255,
                    _ => hasard(),
                };
                let k = |v: u8| ((v as u32 * a as u32) / 255) as u8;
                *p = tiny_skia::PremultipliedColorU8::from_rgba(k(hasard()), k(hasard()), k(hasard()), a).unwrap();
            }
        };
        let mut src = Pixmap::new(30, 20).unwrap();
        remplir(&mut src);
        let mut fond = Pixmap::new(50, 40).unwrap();
        remplir(&mut fond);
        for c in [1.0f32, 0.45] {
            let paint = PixmapPaint {
                opacity: c,
                ..Default::default()
            };
            // (every edge, and the negative corners tiny-skia rounds one pixel over)
            for x in [-36, -31, -30, -29, -28, -21, -1, 0, 1, 13, 20, 21, 49, 50] {
                for y in [-26, -21, -20, -19, -7, -1, 0, 1, 20, 21, 39, 40] {
                    let (mut a, mut b) = (fond.clone(), fond.clone());
                    a.draw_pixmap(x, y, src.as_ref(), &paint, Transform::identity(), None);
                    pose(&mut b, x, y, src.as_ref(), c);
                    assert!(a.data() == b.data(), "pose at {x}, {y}, opacity {c}");
                    for z in [
                        IntRect::from_xywh(3, 2, 10, 9),
                        IntRect::from_xywh(-4, -3, 12, 8),
                        IntRect::from_xywh(25, 15, 9, 9),
                    ] {
                        let z = z.unwrap();
                        let (mut a, mut b) = (fond.clone(), fond.clone());
                        if let Some(part) = src.clone_rect(z) {
                            a.draw_pixmap(x, y, part.as_ref(), &paint, Transform::identity(), None);
                        }
                        pose_zone(&mut b, x, y, src.as_ref(), z, c);
                        assert!(a.data() == b.data(), "zone {z:?} at {x}, {y}, opacity {c}");
                    }
                }
            }
        }
    }
}
