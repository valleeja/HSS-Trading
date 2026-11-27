//! 🤖 Argos AI level 1: the "explanation" layer on the chart. It draws the commands of an explanation
//! (`argos_lens::explain::Command`, already kept by `argos_lens::explain::check`), in a discreet style distinct from
//! the user's drawings: thin dashed levels, light zones, a dashed time marker, an arrow, short labels. Everything is
//! clipped to the price zone. No state is kept here: the app owns the list of commands and clears it.

use crate::drawings::Geo;
use crate::jetons::{ACCENT, DIM, TEXTE, T_PETIT};
use crate::palette::{GREEN, RED};
use crate::shapes::{lines, poly_fill, Rect};
use crate::text::{Align, Rgb, Text};
use crate::toolgeom::{dashes, Px};
use argos_lens::explain::{Command, Tone};
use argos_lens::Decimal;
use tiny_skia::Pixmap;

/// Line width of the explanation layer (thinner than the user's drawings, 1.5 by default).
const WIDTH: f32 = 1.0;
/// Dashes: pixels drawn, pixels left empty.
const DASH: (f64, f64) = (6.0, 4.0);
/// Opacity of the zones.
const ZONE_ALPHA: f32 = 0.08;
/// Gap between a label and its line or point.
const GAP: f64 = 4.0;
/// Arrow head length and half width.
const HEAD: (f64, f64) = (8.0, 4.0);

fn tone_color(t: Tone) -> Rgb {
    match t {
        Tone::Entry => ACCENT,
        Tone::Stop => RED,
        Tone::Target => GREEN,
        Tone::Context => DIM,
    }
}

fn price(d: Decimal) -> f64 {
    f64::try_from(d).unwrap_or(f64::NAN)
}

/// The price zone of the chart (left of the price scale, above the oscillators).
fn price_zone(g: &Geo) -> Rect {
    let bottom = (g.scale.top + g.scale.height).min(g.vp.plot_h());
    Rect::new(0.0, 0.0, g.vp.plot_w(), bottom.max(0.0))
}

/// A dashed polyline in points.
fn dashed(pm: &mut Pixmap, pts: &[Px], c: Rgb) {
    let parts: Vec<Vec<(f32, f32)>> = dashes(pts, DASH.0, DASH.1)
        .iter()
        .map(|d| d.iter().map(|&(x, y)| (x as f32, y as f32)).collect())
        .collect();
    let refs: Vec<&[(f32, f32)]> = parts.iter().map(Vec::as_slice).collect();
    lines(pm, 0.0, 0.0, &refs, c, WIDTH);
}

fn one(pm: &mut Pixmap, text: &Text, g: &Geo, zone: Rect, c: &Command) {
    match c {
        Command::Level { price: p, tone, .. } => {
            // (its label is laid out with the others' by `labels`)
            let (y, col) = (g.y_of(price(*p)), tone_color(*tone));
            dashed(pm, &[(zone.x, y), (zone.x + zone.w, y)], col);
        }
        Command::Zone { top, bottom, label, tone, .. } => {
            let (y0, y1) = (g.y_of(price(*top)), g.y_of(price(*bottom)));
            let (ya, yb) = (y0.min(y1), y0.max(y1));
            let col = tone_color(*tone);
            let (x0, x1) = (zone.x as f32, (zone.x + zone.w) as f32);
            poly_fill(
                pm,
                0.0,
                0.0,
                &[(x0, ya as f32), (x1, ya as f32), (x1, yb as f32), (x0, yb as f32)],
                col,
                ZONE_ALPHA,
            );
            let y = ya + f64::from(T_PETIT) + GAP;
            text.draw(pm, label, ((zone.x + GAP) as f32, y as f32), T_PETIT, DIM, Align::Left);
        }
        Command::TimeMark { time, label, .. } => {
            let x = g.x_of(*time);
            dashed(pm, &[(x, zone.y), (x, zone.y + zone.h)], DIM);
            let y = zone.y + f64::from(T_PETIT) + GAP;
            text.draw(pm, label, ((x + GAP) as f32, y as f32), T_PETIT, DIM, Align::Left);
        }
        Command::Arrow {
            time1, price1, time2, price2, ..
        } => {
            let (a, b) = ((g.x_of(*time1), g.y_of(price(*price1))), (g.x_of(*time2), g.y_of(price(*price2))));
            let len = (b.0 - a.0).hypot(b.1 - a.1);
            if len < 1.0 {
                return;
            }
            let (ux, uy) = ((b.0 - a.0) / len, (b.1 - a.1) / len);
            let base = (b.0 - ux * HEAD.0, b.1 - uy * HEAD.0);
            let seg = [(a.0 as f32, a.1 as f32), (base.0 as f32, base.1 as f32)];
            lines(pm, 0.0, 0.0, &[&seg], ACCENT, WIDTH);
            let head = [
                (b.0 as f32, b.1 as f32),
                ((base.0 - uy * HEAD.1) as f32, (base.1 + ux * HEAD.1) as f32),
                ((base.0 + uy * HEAD.1) as f32, (base.1 - ux * HEAD.1) as f32),
            ];
            poly_fill(pm, 0.0, 0.0, &head, ACCENT, 1.0);
        }
        Command::Label { time, price: p, text: s, .. } => {
            let (x, y) = (g.x_of(*time), g.y_of(price(*p)));
            text.draw(pm, s, ((x + GAP) as f32, (y + f64::from(T_PETIT) + GAP) as f32), T_PETIT, TEXTE, Align::Left);
        }
    }
}

/// Draws the explanation `commands` (already checked by `argos_lens::explain::check`) on `pm`, clipped to the price
/// zone of `g`. Nothing is drawn when `commands` is empty. Called once per frame, after the indicators and the
/// user's drawings, before the last-price line and the crosshair.
/// `cloches`: the heights of the alert bells at the chart's right edge; no label is written under one.
pub fn draw(pm: &mut Pixmap, text: &Text, g: &Geo, commands: &[Command], cloches: &[f64]) {
    if commands.is_empty() {
        return;
    }
    let zone = price_zone(g);
    let Some(mut layer) = Pixmap::new(pm.width(), pm.height()) else { return };
    for c in commands {
        one(&mut layer, text, g, zone, c);
    }
    let levels: Vec<(f64, &str, Rgb)> = commands
        .iter()
        .filter_map(|c| match c {
            Command::Level { price: p, label, tone, .. } if !label.is_empty() => Some((g.y_of(price(*p)), label.as_str(), tone_color(*tone))),
            _ => None,
        })
        .collect();
    for (y, line_y, label, col, right) in labels(&levels, zone, cloches) {
        if (y - GAP - line_y).abs() > 0.5 {
            // moved away from its line: a short tick joins them
            let x = (right + 3.0) as f32;
            let mid = (y - GAP - f64::from(T_PETIT) / 2.0) as f32;
            lines(&mut layer, 0.0, 0.0, &[&[(x, mid), (x, line_y as f32)]], col, WIDTH);
        }
        // on a dark backing (the chart's background, almost opaque): no dashed line runs through the text
        let w = f64::from(text.width(label, T_PETIT));
        let fond = Rect::new(right - w - 3.0, y - GAP - f64::from(T_PETIT) - 1.0, w + 6.0, f64::from(T_PETIT) + 4.0);
        crate::shapes::round_fill(&mut layer, fond, 3.0, crate::Theme::current().background, LABEL_BACKING);
        text.draw(&mut layer, label, (right as f32, (y - GAP) as f32), T_PETIT, col, Align::Right);
    }
    let clip = crate::px::int_rect(zone.x, zone.y, zone.w.max(1.0), zone.h.max(1.0)).and_then(|r| layer.clone_rect(r));
    if let Some(clip) = clip {
        crate::px::put(pm, zone.x, zone.y, &clip);
    }
}

/// Height of a level's label line (points): its text and 3 points of air.
const LABEL_H: f64 = T_PETIT as f64 + 3.0;
/// Opacity of the backing under a level's label.
const LABEL_BACKING: f32 = 0.85;
/// Room kept at the right edge for an alert bell (it is drawn 12 points from the price scale, 14 wide).
const BELL_ROOM: f64 = 26.0;

/// 🏷 Where the levels' labels go (Jee, 19 Nov: « tout reste lisible »): each written just above its line, at the
/// right edge; labels that would touch are moved down just enough to be one under the other, and a label never sits
/// under an alert bell (it moves left of it). Gives, per label: the height it is written at (its line's height when it
/// did not move), its line's height, the text, its colour and the right edge of the text.
fn labels<'a>(levels: &[(f64, &'a str, Rgb)], zone: Rect, cloches: &[f64]) -> Vec<(f64, f64, &'a str, Rgb, f64)> {
    let mut v: Vec<(f64, &str, Rgb)> = levels.to_vec();
    v.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut out = Vec::with_capacity(v.len());
    let mut next = f64::NEG_INFINITY;
    for (line_y, label, col) in v {
        let y = line_y.max(next);
        next = y + LABEL_H;
        let mid = y - GAP - f64::from(T_PETIT) / 2.0;
        let bell = cloches.iter().any(|&b| (b - 9.0 - mid).abs() < LABEL_H);
        let right = zone.x + zone.w - if bell { BELL_ROOM } else { GAP };
        out.push((y, line_y, label, col, right));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::viewport::{PriceScale, Slots, Viewport};

    const T0: i64 = 1_700_000_000;

    struct Chart {
        vp: Viewport,
        scale: PriceScale,
        slots: Slots,
        times: Vec<i64>,
    }

    impl Chart {
        /// 600 × 400 points; prices 100 to 200 over the top `price_h` points (the rest: oscillators).
        fn new(price_h: f64) -> Self {
            let vp = Viewport {
                width: 600,
                height: 400,
                right_offset: 5.0,
                bar_spacing: 10.0,
                ..Viewport::default()
            };
            let scale = PriceScale {
                min: 100.0,
                max: 200.0,
                top: 0.0,
                height: price_h,
                log: false,
                invert: false,
            };
            let times: Vec<i64> = (0..60).map(|i| T0 + i * 60).collect();
            Self {
                vp,
                scale,
                slots: Slots::uniform(times.len()),
                times,
            }
        }

        fn geo(&self) -> Geo<'_> {
            Geo {
                mouse: None,
                vp: &self.vp,
                scale: &self.scale,
                slots: &self.slots,
                times: &self.times,
                bars: &[],
                tf: 60,
            }
        }

        /// A dark background the size of the window, and the image after drawing `cmds` on it.
        fn render(&self, cmds: &[Command]) -> (Pixmap, Pixmap) {
            let mut bg = crate::px::new(self.vp.width, self.vp.height).unwrap();
            bg.fill(tiny_skia::Color::from_rgba8(10, 13, 20, 255));
            let mut pm = bg.clone();
            draw(&mut pm, &Text::default(), &self.geo(), cmds, &[]);
            (bg, pm)
        }
    }

    fn d(v: i64) -> Decimal {
        Decimal::from(v)
    }

    /// Device pixels that differ between the two images, as (x, y).
    fn changed(a: &Pixmap, b: &Pixmap) -> Vec<(u32, u32)> {
        let w = a.width();
        a.pixels()
            .iter()
            .zip(b.pixels())
            .enumerate()
            .filter(|(_, (p, q))| p != q)
            .map(|(i, _)| (i as u32 % w, i as u32 / w))
            .collect()
    }

    fn dev(v: f64) -> u32 {
        crate::px::dev(v).round() as u32
    }

    #[test]
    fn nothing_to_draw_changes_nothing() {
        let c = Chart::new(400.0);
        let (bg, pm) = c.render(&[]);
        assert!(changed(&bg, &pm).is_empty());
    }

    #[test]
    fn a_level_is_a_line_at_its_price() {
        let c = Chart::new(400.0);
        let level = Command::Level {
            price: d(150),
            label: String::new(),
            tone: Tone::Entry,
            sentence: "setup",
        };
        let (bg, pm) = c.render(&[level]);
        let y = dev(c.geo().y_of(150.0));
        let px = changed(&bg, &pm);
        assert!(!px.is_empty());
        assert!(px.iter().all(|&(_, py)| py.abs_diff(y) <= 2), "only around y = {y}");
        // dashed, across the price zone
        let xs: Vec<u32> = px.iter().map(|p| p.0).collect();
        assert!(xs.iter().min().unwrap() < &dev(20.0) && xs.iter().max().unwrap() > &dev(c.vp.plot_w() - 20.0));
    }

    #[test]
    fn nothing_leaves_the_price_zone() {
        // oscillators under 250 points: a zone and a time marker over the whole height stay above them, and nothing
        // goes on the price scale
        let c = Chart::new(250.0);
        let cmds = [
            Command::Zone {
                top: d(300),
                bottom: d(0),
                label: "Risque".into(),
                tone: Tone::Stop,
                sentence: "stop",
            },
            Command::TimeMark {
                time: T0 + 30 * 60,
                label: "Setup Long".into(),
                sentence: "setup",
            },
            Command::Level {
                price: d(120),
                label: "Cible 1 120,00 très longue étiquette".into(),
                tone: Tone::Target,
                sentence: "setup",
            },
        ];
        let (bg, pm) = c.render(&cmds);
        let px = changed(&bg, &pm);
        assert!(!px.is_empty());
        assert!(px.iter().all(|&(x, y)| y < dev(250.0) && x < dev(c.vp.plot_w())), "inside the price zone");
    }

    #[test]
    fn a_zone_fills_between_its_prices() {
        let c = Chart::new(400.0);
        let zone = Command::Zone {
            top: d(160),
            bottom: d(140),
            label: String::new(),
            tone: Tone::Target,
            sentence: "setup",
        };
        let (bg, pm) = c.render(&[zone]);
        let (y0, y1) = (dev(c.geo().y_of(160.0)), dev(c.geo().y_of(140.0)));
        let px = changed(&bg, &pm);
        assert!(px.iter().any(|&(_, y)| y == (y0 + y1) / 2));
        assert!(px.iter().all(|&(_, y)| y + 1 >= y0 && y <= y1 + 1));
    }

    #[test]
    fn a_time_mark_is_vertical_at_its_time() {
        let c = Chart::new(400.0);
        let t = T0 + 20 * 60;
        let (bg, pm) = c.render(&[Command::TimeMark {
            time: t,
            label: String::new(),
            sentence: "setup",
        }]);
        let x = dev(c.geo().x_of(t));
        let px = changed(&bg, &pm);
        assert!(!px.is_empty() && px.iter().all(|&(px, _)| px.abs_diff(x) <= 2));
    }

    #[test]
    fn an_arrow_goes_from_its_start_to_its_end() {
        let c = Chart::new(400.0);
        let (t1, t2) = (T0 + 35 * 60, T0 + 50 * 60);
        let arrow = Command::Arrow {
            time1: t1,
            price1: d(130),
            time2: t2,
            price2: d(170),
            sentence: "setup",
        };
        let (bg, pm) = c.render(&[arrow]);
        let g = c.geo();
        let px = changed(&bg, &pm);
        let near = |x: f64, y: f64| px.iter().any(|&(a, b)| a.abs_diff(dev(x)) <= 2 && b.abs_diff(dev(y)) <= 2);
        let (a, b) = ((g.x_of(t1), g.y_of(130.0)), (g.x_of(t2), g.y_of(170.0)));
        assert!(a.0 > 0.0 && b.0 < c.vp.plot_w(), "both ends visible");
        assert!(near(a.0, a.1) && near(b.0, b.1));
    }

    #[test]
    fn a_label_writes_next_to_its_point() {
        let c = Chart::new(400.0);
        let t = T0 + 15 * 60;
        let (bg, pm) = c.render(&[Command::Label {
            time: t,
            price: d(150),
            text: "Long 70/100".into(),
            sentence: "scores",
        }]);
        let g = c.geo();
        let px = changed(&bg, &pm);
        assert!(!px.is_empty());
        assert!(
            px.iter().all(|&(x, y)| x >= dev(g.x_of(t)) && y >= dev(g.y_of(150.0))),
            "right of and below the point"
        );
    }

    /// 🏷 Jee, 19 Nov: « Haut veille », « VWAP » and « Bas veille » a few points apart never cover each other, each
    /// keeps a tick to its line, and none is written under an alert bell.
    #[test]
    fn close_levels_labels_stay_readable() {
        let zone = Rect::new(0.0, 0.0, 600.0, 400.0);
        let levels = [
            (200.0, "Haut veille 64 970,00", RED),
            (204.0, "VWAP 63 365,77", ACCENT),
            (207.0, "Bas veille 62 035,00", GREEN),
        ];
        let out = labels(&levels, zone, &[]);
        let boxes: Vec<(f64, f64)> = out.iter().map(|o| (o.0 - GAP - f64::from(T_PETIT), o.0 - GAP)).collect();
        for (i, a) in boxes.iter().enumerate() {
            for b in &boxes[i + 1..] {
                assert!(a.1 <= b.0 || b.1 <= a.0, "labels {a:?} and {b:?} overlap");
            }
        }
        assert_eq!(out[0].0, 200.0, "the first label keeps its place");
        // an alert bell at the height of a label: that label ends left of the bell (drawn 12 from the right edge,
        // 7 each side)
        let bell_y = out[1].0 - GAP - f64::from(T_PETIT) / 2.0 + 9.0;
        let with_bell = labels(&levels, zone, &[bell_y]);
        assert!(with_bell[1].4 <= zone.w - 12.0 - 7.0 - 2.0, "the label stays clear of the bell");
        assert_eq!(with_bell[0].0, out[0].0, "the heights do not change");
        // Jee's capture, zoomed out (scale 90 000 at 89 to 50 000 at 380, the alert at 60 773 with its bell)
        let y_of = |p: f64| 89.0 + (90_000.0 - p) * 291.0 / 40_000.0;
        let levels = [
            (y_of(64_970.0), "Haut veille 64 970,00", DIM),
            (y_of(63_365.77), "VWAP 63 365,77", DIM),
            (y_of(62_035.0), "Bas veille 62 035,00", DIM),
        ];
        let bell = y_of(60_773.0).round();
        let out = labels(&levels, zone, &[bell]);
        let boxes: Vec<(f64, f64)> = out.iter().map(|o| (o.0 - GAP - f64::from(T_PETIT), o.0 - GAP)).collect();
        assert!(boxes.windows(2).all(|w| w[0].1 <= w[1].0), "zoomed out: one under the other {boxes:?}");
        for (o, b) in out.iter().zip(&boxes) {
            let bell_box = (bell - 16.0, bell - 2.0);
            let under_bell = b.1 > bell_box.0 && b.0 < bell_box.1;
            assert!(!under_bell || o.4 <= zone.w - 12.0 - 7.0 - 2.0, "« {} » clear of the bell", o.2);
        }
    }
}
