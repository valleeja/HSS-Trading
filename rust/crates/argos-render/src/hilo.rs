//! 📍 Highest and lowest price of the visible part, written on the chart next to their wick (Jee, 21 Nov, like
//! TradingView): a dot at the wick's tip, a short horizontal line, and the price in its own glass pill, at the left of
//! the candle (at the right when the left has no room). Recomputed on every frame, so it follows every move, zoom,
//! time frame, split cell and replay.

use crate::shapes::{self, Rect};
use crate::text::{Align, Text};
use crate::{fill, Rgb};
use argos_core::Bar;
use tiny_skia::Pixmap;

/// Length of the line from the wick's tip to the pill.
pub const LINE: f64 = 22.0;
/// Height of the pill.
pub const PILL_H: f64 = 18.0;
const PAD: f64 = 6.0;
const FONT: f32 = 11.0;

/// One marker: the wick's tip, its price and where the pill goes.
#[derive(Debug, Clone, PartialEq)]
pub struct Mark {
    /// Index of the candle (in the bars shown).
    pub index: usize,
    pub price: f64,
    /// The highest (`true`) or the lowest.
    pub high: bool,
    /// Tip of the wick (points).
    pub tip: (f64, f64),
    /// The pill sits at the right of the candle (no room at the left).
    pub right: bool,
    pub pill: Rect,
}

impl Mark {
    /// The pill was moved off the tip's height (both sides touched a place to keep clear): a short vertical line
    /// joins the wick's tip to the pill's height.
    /// The pill found no free place (pushed up to the top of the plot, or still on a place to keep clear): it
    /// would lie over something else (the title, the legend, the row of ⚡): not drawn.
    pub fn sans_place(&self, avoid: &[Rect]) -> bool {
        self.pill.y <= 0.0 || avoid.iter().any(|a| touches(a, &self.pill))
    }

    pub fn moved(&self) -> bool {
        (self.pill.y + PILL_H / 2.0 - self.tip.1).abs() > 0.5
    }

    /// x of the vertical line and the dot: the wick's, kept at least 2 points inside the plot of width `pw` (the
    /// candle cut at the edge: the line stays visible at the edge).
    pub fn trait_x(&self, pw: f64) -> f64 {
        self.tip.0.max(EDGE).min((pw - EDGE - 1.0).max(EDGE))
    }
}

/// Smallest distance from the plot's edge to the vertical line.
const EDGE: f64 = 2.0;

fn touches(a: &Rect, b: &Rect) -> bool {
    a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h
}

/// The two markers of `bars[first..=last]`: highest high, then lowest low (the first candle reaching it).
/// `x_of`: index → x; `y_of`: price → y; `label_w`: width of a price's pill; `pw`: width of the plot; `avoid`: places
/// the pills keep clear of (the legend at the top left, the bottom pills of Argos Murs, the indicators' tables).
///
/// The pill stays at the tip's height, at the left of the candle, else at its right. When both sides touch a place
/// to keep clear, the highest's pill goes just under it and the lowest's just above it (on the first side).
#[allow(clippy::too_many_arguments)]
pub fn marks(
    bars: &[Bar],
    first: usize,
    last: usize,
    x_of: &dyn Fn(usize) -> f64,
    y_of: &dyn Fn(f64) -> f64,
    label_w: &dyn Fn(f64) -> f64,
    pw: f64,
    avoid: &[Rect],
) -> Vec<Mark> {
    if first > last || last >= bars.len() {
        return vec![];
    }
    let mut hi = first;
    let mut lo = first;
    for i in first..=last {
        if bars[i].high > bars[hi].high {
            hi = i;
        }
        if bars[i].low < bars[lo].low {
            lo = i;
        }
    }
    let clear = |r: &Rect| !avoid.iter().any(|a| touches(a, r));
    [(hi, bars[hi].high, true), (lo, bars[lo].low, false)]
        .into_iter()
        .map(|(index, price, high)| {
            let tip = (x_of(index), y_of(price));
            let w = label_w(price);
            let y = tip.1 - PILL_H / 2.0;
            // (a wick cut at the plot's edge: the pill starts from the edge, see `Mark::trait_x`)
            let tx = tip.0.max(EDGE).min((pw - EDGE - 1.0).max(EDGE));
            let left = Rect::new(tx - LINE - w, y, w, PILL_H);
            let right_r = Rect::new(tx + LINE, y, w, PILL_H);
            let left_ok = left.x >= 0.0;
            let right_ok = right_r.x + right_r.w <= pw;
            // the usual side: left, else right when the left has no room
            let first_right = !left_ok && right_ok;
            let (right, mut pill) = if left_ok && clear(&left) {
                (false, left)
            } else if right_ok && clear(&right_r) {
                (true, right_r)
            } else {
                // under (highest) / above (lowest) whatever it touches, until clear (a few steps at most)
                let mut r = if first_right { right_r } else { left };
                for _ in 0..avoid.len() {
                    let Some(a) = avoid.iter().find(|a| touches(a, &r)) else { break };
                    r.y = if high { a.y + a.h + 4.0 } else { a.y - PILL_H - 4.0 };
                }
                (first_right, r)
            };
            pill.y = pill.y.max(0.0);
            Mark {
                index,
                price,
                high,
                tip,
                right,
                pill,
            }
        })
        .collect()
}

/// Width of the pill of `label`.
pub fn pill_w(text: &Text, label: &str) -> f64 {
    text.width(label, FONT) as f64 + 2.0 * PAD
}

/// Draws `m` with its `label` (the scale's price text) in a plot of width `pw`; `glass`: the pill's inside, `ink`:
/// line, dot and pill's edge, `fg`: the price. A moved pill: a vertical line along the wick from its tip to the
/// pill's height, then the short horizontal line (the wick cut at the plot's edge: both at the edge, see
/// [`Mark::trait_x`]).
#[allow(clippy::too_many_arguments)]
pub fn draw(pm: &mut Pixmap, text: &Text, m: &Mark, label: &str, pw: f64, glass: Rgb, ink: Rgb, fg: Rgb) {
    let (x, y) = (m.trait_x(pw), m.tip.1.round());
    let py = (m.pill.y + PILL_H / 2.0).round();
    if m.moved() {
        fill(pm, x, y.min(py), 1.0, (py - y).abs(), ink, 0.9);
    }
    let (a, b) = if m.right { (x, m.pill.x) } else { (m.pill.x + m.pill.w, x) };
    fill(pm, a, py, b - a, 1.0, ink, 0.9);
    shapes::round_fill(pm, Rect::new(x - 2.5, y - 2.0, 5.0, 5.0), 2.5, ink, 1.0);
    shapes::round_fill(pm, m.pill, PILL_H / 2.0, glass, 0.82);
    shapes::round_stroke(pm, m.pill, PILL_H / 2.0, ink, 1.0);
    text.draw(pm, label, ((m.pill.x + PAD) as f32, (py + 4.0) as f32), FONT, fg, Align::Left);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar(time: i64, high: f64, low: f64) -> Bar {
        Bar {
            time,
            open: (high + low) / 2.0,
            high,
            low,
            close: (high + low) / 2.0,
            volume: 1.0,
        }
    }

    fn sample(tf: i64) -> Vec<Bar> {
        [(10.0, 8.0), (12.0, 9.0), (60_980.0, 7.0), (11.0, 55_154.0 / 10_000.0), (13.0, 9.5)]
            .iter()
            .enumerate()
            .map(|(i, &(h, l))| bar(i as i64 * tf, h, l))
            .collect()
    }

    fn run(bars: &[Bar], first: usize, last: usize, spacing: f64, pw: f64) -> Vec<Mark> {
        marks(bars, first, last, &|i| 10.0 + i as f64 * spacing, &|p| 500.0 - p / 200.0, &|_| 60.0, pw, &[])
    }

    #[test]
    fn marqueurs_au_plus_haut_et_au_plus_bas_visibles() {
        let bars = sample(900);
        let m = run(&bars, 0, 4, 100.0, 800.0);
        assert_eq!(m.len(), 2);
        assert!(m[0].high && m[0].index == 2 && m[0].price == 60_980.0);
        assert!(!m[1].high && m[1].index == 3 && m[1].price == 5.5154);
        // only the visible part counts
        let m = run(&bars, 3, 4, 100.0, 800.0);
        assert_eq!((m[0].index, m[1].index), (4, 3));
        // tip at the wick, pill at the left, centred on the tip
        let m = run(&bars, 0, 4, 100.0, 800.0);
        assert_eq!(m[0].tip, (210.0, 500.0 - 60_980.0 / 200.0));
        assert!(!m[0].right && m[0].pill.x + m[0].pill.w == 210.0 - LINE);
        assert_eq!(m[0].pill.y + PILL_H / 2.0, m[0].tip.1);
    }

    #[test]
    fn a_droite_pres_du_bord_gauche() {
        let bars = sample(900);
        // the highest candle at x = 30: no room at the left
        let m = run(&bars, 0, 4, 10.0, 800.0);
        assert!(m[0].right, "{:?}", m[0]);
        assert_eq!(m[0].pill.x, m[0].tip.0 + LINE);
    }

    #[test]
    fn meme_resultat_sur_deux_unites_de_temps() {
        let a = run(&sample(900), 0, 4, 100.0, 800.0);
        let b = run(&sample(14_400), 0, 4, 100.0, 800.0);
        assert_eq!(a, b, "15 min and 4 h");
    }

    #[test]
    fn rien_sans_bougie_visible() {
        assert!(run(&[], 0, 0, 10.0, 800.0).is_empty());
        assert!(run(&sample(60), 3, 2, 10.0, 800.0).is_empty());
    }

    fn run_avoid(bars: &[Bar], spacing: f64, pw: f64, avoid: &[Rect]) -> Vec<Mark> {
        marks(
            bars,
            0,
            bars.len() - 1,
            &|i| 10.0 + i as f64 * spacing,
            &|p| 500.0 - p / 200.0,
            &|_| 60.0,
            pw,
            avoid,
        )
    }

    /// The legend (title, O H L C, Vol line, « ⌄ N ») at the top left: the highest's pill never on it, on 15 min and
    /// 4 h alike; when both sides touch it, just under it, joined to the tip by a vertical line.
    #[test]
    fn plus_haut_sous_la_legende() {
        for tf in [900, 14_400] {
            let bars = sample(tf);
            let tip = run(&bars, 0, 4, 100.0, 800.0)[0].tip;
            // a wide legend over the tip's height, on both sides of the candle
            let legende = Rect::new(0.0, tip.1 - 40.0, 600.0, 80.0);
            let m = run_avoid(&bars, 100.0, 800.0, &[legende]);
            assert!(!touches(&m[0].pill, &legende), "{:?}", m[0]);
            assert!(m[0].moved(), "moved under the legend");
            assert_eq!(m[0].pill.y, legende.y + legende.h + 4.0);
            assert!(!m[0].right, "first side kept");
            assert_eq!(m[0].tip, tip, "the tip stays at the wick");
            // a narrow legend on the left only: the pill goes to the right, at the tip's height
            let etroite = Rect::new(0.0, tip.1 - 40.0, 190.0, 80.0);
            let m = run_avoid(&bars, 100.0, 800.0, &[etroite]);
            assert!(m[0].right && !m[0].moved() && !touches(&m[0].pill, &etroite), "{:?}", m[0]);
            let mut pm = Pixmap::new(800, 600).unwrap();
            draw(
                &mut pm,
                &Text::default(),
                &run_avoid(&bars, 100.0, 800.0, &[legende])[0],
                "60 980",
                800.0,
                Rgb(0, 0, 0),
                Rgb(0, 200, 0),
                Rgb(255, 255, 255),
            );
        }
    }

    /// The bottom pills of Argos Murs: the lowest's pill goes just above them.
    #[test]
    fn plus_bas_au_dessus_des_pilules_du_bas() {
        let bars = sample(900);
        let tip = run(&bars, 0, 4, 100.0, 800.0)[1].tip;
        let bas = Rect::new(0.0, tip.1 - 10.0, 800.0, 46.0);
        let m = run_avoid(&bars, 100.0, 800.0, &[bas]);
        assert!(!touches(&m[1].pill, &bas) && m[1].moved(), "{:?}", m[1]);
        assert_eq!(m[1].pill.y + PILL_H + 4.0, bas.y);
        assert!(!m[1].sans_place(&[bas]), "just above: drawn");
        // places to keep clear up to the top of the plot: no free place, not drawn
        let haut = Rect::new(0.0, 0.0, 800.0, bas.y);
        let m = run_avoid(&bars, 100.0, 800.0, &[bas, haut]);
        assert!(m[1].sans_place(&[bas, haut]), "{:?}", m[1]);
    }

    /// Nothing to keep clear near the tips: unchanged.
    #[test]
    fn cas_normal_inchange() {
        let bars = sample(900);
        let loin = Rect::new(700.0, 0.0, 10.0, 10.0);
        assert_eq!(run_avoid(&bars, 100.0, 800.0, &[loin]), run(&bars, 0, 4, 100.0, 800.0));
        assert!(run(&bars, 0, 4, 100.0, 800.0).iter().all(|m| !m.moved()));
    }

    /// The floating toolbar laid over the highest's height (both sides of the candle): the pill goes under it, never
    /// under the bar; the legend cut in lines: the pill stays at the tip's height right of a short line.
    #[test]
    fn plus_haut_hors_de_la_barre_d_outils() {
        for tf in [900, 14_400] {
            let bars = sample(tf);
            let tip = run(&bars, 0, 4, 100.0, 800.0)[0].tip;
            let barre = Rect::new(20.0, tip.1 - 20.0, 700.0, 40.0);
            let legende = Rect::new(0.0, barre.y + barre.h, 300.0, 22.0);
            let m = run_avoid(&bars, 100.0, 800.0, &[legende, barre]);
            assert!(!touches(&m[0].pill, &barre) && !touches(&m[0].pill, &legende), "{:?}", m[0]);
            assert!(m[0].pill.y >= legende.y + legende.h, "under both");
            // a short legend line at the tip's height, nothing else: the pill stays at that height
            let courte = Rect::new(0.0, tip.1 - 10.0, 60.0, 20.0);
            let m = run_avoid(&bars, 100.0, 800.0, &[courte]);
            assert!(!m[0].moved() && !touches(&m[0].pill, &courte), "{:?}", m[0]);
        }
    }

    /// The highest candle cut at the left edge (its x < 0): the vertical line is drawn inside the plot, at the edge
    /// + 2 points; a wick inside the plot keeps its x.
    #[test]
    fn meche_coupee_trait_au_bord() {
        let bars = sample(900);
        // the highest (index 2) at x = 10 + 2·100 − 260 = −50
        let m = marks(
            &bars,
            0,
            4,
            &|i| -250.0 + i as f64 * 100.0,
            &|p| 500.0 - p / 200.0,
            &|_| 60.0,
            800.0,
            &[Rect::new(0.0, 0.0, 800.0, 300.0)],
        );
        let h = &m[0];
        assert!(h.tip.0 < 0.0 && h.moved(), "{h:?}");
        assert_eq!(h.trait_x(800.0), 2.0);
        assert!(h.right && h.pill.x == 2.0 + LINE, "the pill from the edge: {h:?}");
        let mut pm = Pixmap::new(800, 600).unwrap();
        draw(&mut pm, &Text::default(), h, "60 980", 800.0, Rgb(0, 0, 0), Rgb(0, 200, 0), Rgb(255, 255, 255));
        // a pixel of the line between the tip and the pill
        let y = ((h.tip.1 + h.pill.y + PILL_H / 2.0) / 2.0).round() as u32;
        let px = pm.pixel(2, y).unwrap();
        assert!(px.green() > 100 && px.alpha() > 200, "{px:?}");
        // normal case: the wick's x
        let n = run(&bars, 0, 4, 100.0, 800.0);
        assert_eq!(n[0].trait_x(800.0), n[0].tip.0);
    }
}
