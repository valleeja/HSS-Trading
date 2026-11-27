//! 🎨 The colour palette of the drawings (Jee's captures, like TradingView): a glass card with a row of 10 greys
//! (white to black), a row of 10 vivid colours, 6 rows of their shades (light to dark), the chosen colour circled;
//! a line; the custom colours followed by « + »; « Opacité » with a slider (transparent → the colour) and « 100% ».
//! Colours are « #rrggbb », or « #rrggbbaa » when not opaque.

use crate::shapes::{lines, round_fill, round_stroke, Rect};
use crate::text::{Align, Rgb, Text};
use tiny_skia::Pixmap;

const GREYS: [&str; 10] = [
    "#ffffff", "#d1d4dc", "#b2b5be", "#9598a1", "#787b86", "#5d606b", "#434651", "#2a2e39", "#131722", "#000000",
];
const VIVID: [&str; 10] = [
    "#f23645", "#ff9800", "#ffeb3b", "#4caf50", "#089981", "#00bcd4", "#2962ff", "#673ab7", "#9c27b0", "#e91e63",
];
/// Shades of each vivid colour, light to dark (one column each).
const SHADES: [[&str; 6]; 10] = [
    ["#fccbcd", "#faa1a4", "#f77c80", "#f7525f", "#b22833", "#801922"],
    ["#ffe0b2", "#ffcc80", "#ffb74d", "#ffa726", "#f57c00", "#e65100"],
    ["#fff9c4", "#fff59d", "#fff176", "#ffee58", "#fbc02d", "#f57f17"],
    ["#c8e6c9", "#a5d6a7", "#81c784", "#66bb6a", "#388e3c", "#1b5e20"],
    ["#ace5dc", "#70ccbd", "#42bda8", "#22ab94", "#056656", "#00332a"],
    ["#b2ebf2", "#80deea", "#4dd0e1", "#26c6da", "#0097a7", "#006064"],
    ["#bbd9fb", "#90bff9", "#5b9cf6", "#3179f5", "#1848cc", "#0c3299"],
    ["#d1c4e9", "#b39ddb", "#9575cd", "#7e57c2", "#512da8", "#311b92"],
    ["#e1bee7", "#ce93d8", "#ba68c8", "#ab47bc", "#7b1fa2", "#4a148c"],
    ["#f8bbd0", "#f48fb1", "#f06292", "#ec407a", "#c2185b", "#880e4f"],
];

const PAD: f64 = 14.0;
const CELL: f64 = 22.0;
const GAP: f64 = 4.0;
/// Custom colours kept at most.
pub const CUSTOM_MAX: usize = 9;

/// A colour and its opacity (0–1) from « #rrggbb » or « #rrggbbaa ».
pub fn split(s: &str) -> Option<(Rgb, f32)> {
    let h = s.trim_start_matches('#');
    if h.len() != 6 && h.len() != 8 {
        return None;
    }
    let a = h.get(6..8).and_then(|a| u8::from_str_radix(a, 16).ok()).map_or(1.0, |a| a as f32 / 255.0);
    Some((Rgb::hex(&h[..6]), a))
}

/// « #rrggbb » (opaque) or « #rrggbbaa ».
pub fn join(c: Rgb, a: f32) -> String {
    if a >= 0.999 {
        format!("#{:02x}{:02x}{:02x}", c.0, c.1, c.2)
    } else {
        format!("#{:02x}{:02x}{:02x}{:02x}", c.0, c.1, c.2, (a.clamp(0.0, 1.0) * 255.0).round() as u8)
    }
}

/// What a click on the palette did.
#[derive(Debug, Clone, PartialEq)]
pub enum CpClick {
    /// Nothing (inside the card).
    None,
    /// The new colour (with its opacity).
    Pick(String),
    /// « + »: keep the current colour among the custom ones.
    Add,
    /// Outside the card: close it.
    Close,
    /// « Épaisseur » (settings window): 1 to 4.
    Width(f32),
    /// « Style de ligne »: 0 solid, 1 dashed, 2 dotted.
    Dash(u8),
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Zone {
    Swatch(usize),
    Custom(usize),
    Add,
    Slider,
    Width(usize),
    Dash(u8),
}

/// The palette open under a button (window coordinates).
#[derive(Debug, Clone, PartialEq)]
pub struct ColorPick {
    pub rect: Rect,
    /// The colour shown as chosen (« #rrggbb[aa] »).
    pub current: String,
    pub custom: Vec<String>,
    /// From the settings window: « Épaisseur » and « Style de ligne » under the colours (width, 0 solid / 1 dashed /
    /// 2 dotted).
    pub line: Option<(f32, u8)>,
}

fn all() -> Vec<&'static str> {
    let mut v: Vec<&str> = GREYS.to_vec();
    v.extend(VIVID);
    for row in 0..6 {
        v.extend(SHADES.iter().map(|col| col[row]));
    }
    v
}

impl ColorPick {
    pub fn size() -> (f64, f64) {
        let w = 2.0 * PAD + 10.0 * CELL + 9.0 * GAP;
        // greys, vivid, a space, 6 shades, a line, the custom row, « Opacité », the slider
        let h = PAD + 2.0 * (CELL + GAP) + 8.0 + 6.0 * (CELL + GAP) + 14.0 + (CELL + GAP) + 24.0 + 32.0 + PAD;
        (w, h)
    }

    /// Height of « Épaisseur » and « Style de ligne » (settings window).
    const LINE_H: f64 = 2.0 * (24.0 + 36.0 + 8.0);

    /// Under `anchor` (above it when there is no room), always inside the window.
    pub fn new(anchor: Rect, current: &str, custom: Vec<String>, win_w: f64, win_h: f64) -> Self {
        Self::with_line(anchor, current, custom, None, win_w, win_h)
    }

    /// The same, with « Épaisseur » and « Style de ligne » (`line`: width, dash) under the colours.
    pub fn with_line(anchor: Rect, current: &str, custom: Vec<String>, line: Option<(f32, u8)>, win_w: f64, win_h: f64) -> Self {
        let (w, h) = Self::size();
        let h = h + if line.is_some() { Self::LINE_H } else { 0.0 };
        let mut y = anchor.y + anchor.h + 6.0;
        if y + h > win_h - 4.0 {
            y = anchor.y - h - 6.0;
        }
        let x = anchor.x.min(win_w - w - 4.0).max(4.0);
        Self {
            rect: Rect::new(x, y.min(win_h - h - 4.0).max(4.0), w, h),
            current: current.into(),
            custom,
            line,
        }
    }

    fn zones(&self) -> Vec<(Rect, Zone)> {
        let r = self.rect;
        let mut out = vec![];
        let cell = |col: usize, y: f64| Rect::new(r.x + PAD + col as f64 * (CELL + GAP), y, CELL, CELL);
        let mut y = r.y + PAD;
        for (k, _) in all().iter().enumerate() {
            let row = k / 10;
            if row == 2 && k % 10 == 0 {
                y += 8.0;
            }
            out.push((cell(k % 10, y + row as f64 * (CELL + GAP)), Zone::Swatch(k)));
        }
        y += 8.0 * (CELL + GAP) + 14.0;
        for k in 0..self.custom.len().min(CUSTOM_MAX) {
            out.push((cell(k, y), Zone::Custom(k)));
        }
        out.push((cell(self.custom.len().min(CUSTOM_MAX), y), Zone::Add));
        let sy = y + CELL + GAP + 24.0;
        out.push((Rect::new(r.x + PAD, sy, r.w - 2.0 * PAD - 70.0, 24.0), Zone::Slider));
        if self.line.is_some() {
            // « Épaisseur »: 4 boxes side by side; « Style de ligne »: 3
            let inner = r.w - 2.0 * PAD;
            let wy = sy + 32.0 + 8.0 + 24.0;
            for k in 0..4 {
                out.push((Rect::new(r.x + PAD + k as f64 * inner / 4.0, wy, inner / 4.0, 36.0), Zone::Width(k)));
            }
            let dy = wy + 36.0 + 8.0 + 24.0;
            for k in 0..3u8 {
                out.push((Rect::new(r.x + PAD + k as f64 * inner / 3.0, dy, inner / 3.0, 36.0), Zone::Dash(k)));
            }
        }
        out
    }

    fn slider_y(&self) -> f64 {
        self.zones().iter().find(|(_, z)| *z == Zone::Slider).map_or(0.0, |(r, _)| r.y)
    }

    /// A click at (`x`, `y`) (window coordinates).
    pub fn click(&mut self, x: f64, y: f64) -> CpClick {
        if !self.rect.contains(x, y) {
            return CpClick::Close;
        }
        let (c, a) = split(&self.current).unwrap_or((Rgb(0x29, 0x62, 0xff), 1.0));
        match self.zones().into_iter().find(|(r, _)| r.contains(x, y)) {
            Some((_, Zone::Swatch(k))) => {
                self.current = join(Rgb::hex(all()[k]), a);
                CpClick::Pick(self.current.clone())
            }
            Some((_, Zone::Custom(k))) => {
                self.current = join(split(&self.custom[k]).map_or(c, |p| p.0), a);
                CpClick::Pick(self.current.clone())
            }
            Some((_, Zone::Add)) => CpClick::Add,
            Some((_, Zone::Width(k))) => {
                let w = (k + 1) as f32;
                self.line = self.line.map(|(_, d)| (w, d));
                CpClick::Width(w)
            }
            Some((_, Zone::Dash(k))) => {
                self.line = self.line.map(|(w, _)| (w, k));
                CpClick::Dash(k)
            }
            Some((r, Zone::Slider)) => {
                self.current = join(c, ((x - r.x) / r.w).clamp(0.0, 1.0) as f32);
                CpClick::Pick(self.current.clone())
            }
            None => CpClick::None,
        }
    }

    /// The opacity slider dragged to `x` (window coordinates): the new colour.
    pub fn drag(&mut self, x: f64) -> Option<String> {
        let (r, _) = self.zones().into_iter().find(|(_, z)| *z == Zone::Slider)?;
        let (c, _) = split(&self.current)?;
        self.current = join(c, ((x - r.x) / r.w).clamp(0.0, 1.0) as f32);
        Some(self.current.clone())
    }

    /// `true` when (`x`, `y`) is on the slider (a drag starts there).
    pub fn on_slider(&self, x: f64, y: f64) -> bool {
        self.zones().iter().any(|(r, z)| *z == Zone::Slider && r.contains(x, y))
    }

    pub fn draw(&self, text: &Text, mouse: Option<(f64, f64)>) -> Option<Pixmap> {
        let r = self.rect;
        let mut pm = crate::px::new(r.w.ceil() as u32, r.h.ceil() as u32)?;
        let loc = |q: Rect| Rect::new(q.x - r.x, q.y - r.y, q.w, q.h);
        crate::shapes::menu_card(&mut pm, loc(r));
        let (cur, alpha) = split(&self.current).unwrap_or((Rgb(0x29, 0x62, 0xff), 1.0));
        let hover = |q: &Rect| mouse.is_some_and(|(x, y)| q.contains(x, y));
        let colours = all();
        for (q, z) in self.zones() {
            let lq = loc(q);
            match z {
                Zone::Swatch(k) | Zone::Custom(k) => {
                    let c = match z {
                        Zone::Swatch(_) => Rgb::hex(colours[k]),
                        _ => split(&self.custom[k]).map_or(cur, |p| p.0),
                    };
                    round_fill(&mut pm, lq, 4.0, c, 1.0);
                    if c == cur {
                        round_stroke(
                            &mut pm,
                            Rect::new(lq.x - 2.5, lq.y - 2.5, lq.w + 5.0, lq.h + 5.0),
                            6.0,
                            crate::jetons::TEXTE,
                            1.6,
                        );
                    } else if hover(&q) {
                        round_stroke(&mut pm, lq, 4.0, crate::jetons::TEXTE, 1.0);
                    }
                }
                Zone::Add => {
                    let ink = crate::zoompad::ENCRE;
                    lines(&mut pm, lq.cx(), lq.cy(), &[&[(-6.0, 0.0), (6.0, 0.0)], &[(0.0, -6.0), (0.0, 6.0)]], ink, 1.5);
                }
                Zone::Width(_) | Zone::Dash(_) => {
                    let k = if let Zone::Width(k) = z { k } else { 0 };
                    let on = match (z, self.line) {
                        (Zone::Width(_), Some((w, _))) => (w.round() as usize).clamp(1, 4) == k + 1,
                        (Zone::Dash(j), Some((_, d))) => d == j,
                        _ => false,
                    };
                    if on {
                        crate::fill(&mut pm, lq.x, lq.y, lq.w, lq.h, crate::jetons::TEXTE, 0.92);
                    } else if hover(&q) {
                        crate::fill(&mut pm, lq.x, lq.y, lq.w, lq.h, crate::jetons::BLANC, 0.08);
                    }
                    crate::shapes::round_stroke(&mut pm, lq, 2.0, crate::jetons::sur_fond(crate::jetons::LISERE), 1.0);
                    let ink = if on { crate::jetons::FOND } else { crate::jetons::TEXTE };
                    let (x0, x1) = (lq.x + 14.0, lq.x + lq.w - 14.0);
                    match z {
                        Zone::Width(_) => crate::fill(&mut pm, x0, lq.cy() - (k + 1) as f64 / 2.0, x1 - x0, (k + 1) as f64, ink, 1.0),
                        _ => {
                            let (seg, gap) = match z {
                                Zone::Dash(1) => (6.0, 4.0),
                                Zone::Dash(2) => (2.0, 3.0),
                                _ => (x1 - x0, 0.0),
                            };
                            let mut x = x0;
                            while x < x1 {
                                crate::fill(&mut pm, x, lq.cy() - 1.0, f64::min(seg, x1 - x), 2.0, ink, 1.0);
                                x += seg + f64::max(gap, 0.01);
                            }
                        }
                    }
                }
                Zone::Slider => {
                    // checkerboard (transparent) under a gradient to the colour, the knob at the opacity
                    let tr = Rect::new(lq.x, lq.cy() - 5.0, lq.w, 10.0);
                    let mut x = tr.x;
                    let mut k = 0;
                    while x < tr.x + tr.w {
                        let w = 5.0f64.min(tr.x + tr.w - x);
                        for (j, yy) in [tr.y, tr.y + 5.0].iter().enumerate() {
                            let dark = (k + j) % 2 == 0;
                            crate::fill(&mut pm, x, *yy, w, 5.0, if dark { Rgb(0x55, 0x58, 0x60) } else { Rgb(0x2a, 0x2d, 0x34) }, 1.0);
                        }
                        x += 5.0;
                        k += 1;
                    }
                    let steps = 40;
                    for s in 0..steps {
                        let f = s as f64 / steps as f64;
                        crate::fill(&mut pm, tr.x + f * tr.w, tr.y, tr.w / steps as f64 + 0.5, 10.0, cur, f as f32);
                    }
                    let kx = tr.x + alpha as f64 * tr.w;
                    round_fill(&mut pm, Rect::new(kx - 7.0, tr.cy() - 7.0, 14.0, 14.0), 7.0, Rgb(0xff, 0xff, 0xff), 1.0);
                    round_stroke(&mut pm, Rect::new(kx - 7.0, tr.cy() - 7.0, 14.0, 14.0), 7.0, Rgb(0x20, 0x22, 0x28), 1.5);
                    let bx = Rect::new(lq.x + lq.w + 12.0, lq.y - 4.0, 58.0, 32.0);
                    round_stroke(&mut pm, bx, 6.0, crate::jetons::sur_fond(crate::jetons::LISERE), 1.0);
                    let pct = format!("{}%", (alpha * 100.0).round() as i32);
                    text.draw(
                        &mut pm,
                        &pct,
                        (bx.cx() as f32, (bx.cy() + 5.0) as f32),
                        14.0,
                        crate::jetons::TEXTE,
                        Align::Center,
                    );
                }
            }
        }
        // the line between the shades and the custom colours, « Opacité »
        let sep = loc(Rect::new(r.x + PAD, self.slider_y() - 24.0 - CELL - GAP - 9.0, r.w - 2.0 * PAD, 1.0));
        crate::fill(&mut pm, sep.x, sep.y, sep.w, 1.0, crate::jetons::sur_fond(crate::jetons::LISERE), 1.0);
        let oy = self.slider_y() - r.y - 8.0;
        text.draw(&mut pm, "Opacité", (PAD as f32, oy as f32), 13.0, crate::palette::GREY, Align::Left);
        if self.line.is_some() {
            let first = |want: fn(&Zone) -> bool| self.zones().into_iter().find(|(_, z)| want(z)).map_or(0.0, |(r, _)| r.y);
            let wy = first(|z| matches!(z, Zone::Width(_))) - r.y - 8.0;
            let dy = first(|z| matches!(z, Zone::Dash(_))) - r.y - 8.0;
            text.draw(&mut pm, "Épaisseur", (PAD as f32, wy as f32), 13.0, crate::palette::GREY, Align::Left);
            text.draw(&mut pm, "Style de ligne", (PAD as f32, dy as f32), 13.0, crate::palette::GREY, Align::Left);
        }
        Some(pm)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn couleurs_et_opacite() {
        assert_eq!(all().len(), 80, "10 greys, 10 vivid, 6 × 10 shades");
        assert_eq!(join(Rgb(0xff, 0, 0), 1.0), "#ff0000");
        assert_eq!(split("#ff000080"), Some((Rgb(0xff, 0, 0), 128.0 / 255.0)));
        let mut p = ColorPick::new(Rect::new(100.0, 100.0, 30.0, 30.0), "#2962ff", vec!["#123456".into()], 1440.0, 900.0);
        // a swatch: the colour, opacity kept
        let (q, _) = p.zones()[10];
        assert_eq!(p.click(q.cx(), q.cy()), CpClick::Pick("#f23645".into()));
        // the slider at its middle: half opacity
        let (s, _) = *p.zones().iter().find(|(_, z)| *z == Zone::Slider).unwrap();
        let CpClick::Pick(c) = p.click(s.x + s.w / 2.0, s.cy()) else { panic!() };
        assert!((split(&c).unwrap().1 - 0.5).abs() < 0.01, "{c}");
        // the custom colour, then « + », then outside: close
        let (q, _) = *p.zones().iter().find(|(_, z)| *z == Zone::Custom(0)).unwrap();
        assert!(matches!(p.click(q.cx(), q.cy()), CpClick::Pick(c) if c.starts_with("#123456")));
        let (q, _) = *p.zones().iter().find(|(_, z)| *z == Zone::Add).unwrap();
        assert_eq!(p.click(q.cx(), q.cy()), CpClick::Add);
        assert_eq!(p.click(5.0, 5.0), CpClick::Close);
        assert!(p.draw(&Text::default(), None).is_some());
        // no room below: above the button
        let up = ColorPick::new(Rect::new(100.0, 850.0, 30.0, 30.0), "#2962ff", vec![], 1440.0, 900.0);
        assert!(up.rect.y + up.rect.h <= 850.0);
        // from the settings window: width and line style under the colours
        let mut l = ColorPick::with_line(Rect::new(100.0, 100.0, 30.0, 30.0), "#2962ff", vec![], Some((1.0, 0)), 1440.0, 900.0);
        let (q, _) = *l.zones().iter().find(|(_, z)| *z == Zone::Width(2)).unwrap();
        assert_eq!(l.click(q.cx(), q.cy()), CpClick::Width(3.0));
        let (q, _) = *l.zones().iter().find(|(_, z)| *z == Zone::Dash(1)).unwrap();
        assert_eq!(l.click(q.cx(), q.cy()), CpClick::Dash(1));
        assert_eq!(l.line, Some((3.0, 1)));
        assert!(l.draw(&Text::default(), None).is_some());
    }
}
