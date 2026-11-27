//! 🟠 The DÉMO band over the chart of the upright tablet (bug B-018, found on 25 Nov 2026 in B-017's captures): a wall
//! pill of Argos Murs went half under it, and a high / low pill could too.

use super::bande_noire::{dessiner, ecran, ouverte};
use super::*;

/// 📟 B-018: the iPad upright by the finger, the DÉMO band over the chart under the top bar, Argos Murs on with its
/// walls at the last candles' price (the band's row): no wall pill and no high / low pill is drawn under the band.
/// The walls make room, as they do for the drawing bar: a pill moves under the band, or the smallest wall is left out
/// when there is no room between the band and the last price.
#[test]
fn the_demo_band_keeps_the_wall_pills_clear() {
    dossier_a_soi("demo_murs");
    let mut a = ouverte(&ecran(1024.0, 1260.0, 2.0), true);
    let _ = dessiner(&mut a);
    assert!(!a.demo_en_bas(), "the tablet upright keeps the DÉMO band at the top");
    let d = a.bandeau_rect();
    let (rw, th) = (a.rw(), a.top_h());
    assert!(d.w > 0.0 && d.y + d.h > th, "the band over the chart: {d:?}");
    let band = Rect::new(d.x - rw, d.y - th, d.w, d.h);
    let pills = argos_render::murs::pilules_dessinees();
    assert!(!pills.is_empty(), "walls drawn");
    // (a wall of the test stands in the band's row: the guard has something to see)
    assert!(
        pills.iter().any(|(r, _)| r.y < band.y + band.h + 40.0),
        "a wall pill near the band: {pills:?} {band:?}"
    );
    for (r, s) in &pills {
        assert!(!croisent(r, &band), "the wall pill {s:?} {r:?} under the DÉMO band {band:?}");
    }
    for r in argos_render::hilo_dessines() {
        assert!(!croisent(&r, &band), "a high / low pill {r:?} under the DÉMO band {band:?}");
    }
    argos_render::actions::set_tactile(false);
}
