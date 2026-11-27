//! 📏 Échelle de prix de la chart (automatique, manuelle, log…) et le cadre des dessins.

use crate::*;

/// Échelle de prix des bougies `first..=last` selon les réglages (auto, bornes à la main, rapport verrouillé, log, inversée).
pub(crate) fn build_scale(c: &Chart, bars: &[Bar], first: usize, last: usize) -> PriceScale {
    let vp = c.viewport;
    let ph = vp.plot_h();
    let o = c.settings.scale;
    let fit = if close_scaled(&c.style) {
        let closes: Vec<Bar> = bars[first..=last]
            .iter()
            .map(|b| Bar {
                high: b.close,
                low: b.close,
                ..*b
            })
            .collect();
        PriceScale::fit(&closes, 0, closes.len() - 1, ph)
    } else {
        PriceScale::fit(bars, first, last, ph)
    };
    let mut s = fit.with_log(o.mode == ScaleMode::Log);
    if let Some((lo, hi)) = o.manual {
        (s.min, s.max) = (lo, hi);
    }
    if let Some(r) = o.lock_ratio {
        let (mid, half) = ((s.min + s.max) / 2.0, r * vp.bar_spacing * ph / 2.0);
        (s.min, s.max) = (mid - half, mid + half);
    }
    s.invert = o.invert;
    // oscillateurs (bandes en bas) : les bougies prennent la place au-dessus, comme un panneau de TradingView
    let osc = c.indicators.map_or(0.0, |i| i.osc_h());
    if osc > 0.0 {
        s.height = (s.height - osc).max(80.0);
    }
    s
}

/// L'échelle de prix affichée en ce moment (la fenêtre desktop s'en sert pour glisser / zoomer l'échelle sans saut).
pub fn current_scale(c: &Chart) -> Option<PriceScale> {
    let shown = series(c.bars, &c.style);
    let (first, last) = c.viewport.visible(&slots(c.bars, &c.style))?;
    Some(build_scale(c, &shown, first, last))
}

/// Géométrie pour placer les objets dessinés (temps ↔ x, prix ↔ y) telle qu'affichée en ce moment.
pub fn frame(c: &Chart) -> Option<drawings::Frame> {
    let shown = series(c.bars, &c.style);
    let slots = slots(c.bars, &c.style);
    let (first, last) = c.viewport.visible(&slots)?;
    let scale = build_scale(c, &shown, first, last);
    let times = shown.iter().map(|b| b.time).collect();
    Some(drawings::Frame {
        vp: c.viewport,
        scale,
        slots,
        times,
        bars: shown.into_owned(),
        tf: c.tf_seconds,
    })
}
