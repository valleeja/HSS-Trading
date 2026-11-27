//! 📍 Inputs prix et heure sur la chart, comme Pine v6 (`input.price`, `input.time`) : un prix est une ligne
//! horizontale, une heure une ligne verticale ; un prix et une heure du même groupe avec le même `inline` font un seul
//! point. On les attrape et on les glisse à la souris : la valeur de l'input suit.

use crate::drawings::Geo;
use crate::indicators::{values, Ind, IndOut};
use crate::shapes::{round_fill, round_stroke, Rect};
use crate::text::{Align, Text};
use argos_script::inputs::{Inputs, Kind};
use tiny_skia::Pixmap;

/// Un repère : l'indicateur, son input prix et / ou heure (index dans sa table), le nom à afficher.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mark {
    pub ind: Ind,
    pub price: Option<usize>,
    pub time: Option<usize>,
    pub label: &'static str,
}

/// Distance (px) à laquelle la souris attrape un repère.
const GRAB: f64 = 6.0;

/// Repères des indicateurs actifs et montrés (seulement les inputs actifs).
pub fn marks(out: &IndOut) -> Vec<Mark> {
    let mut v = vec![];
    for &ind in out.on.iter().filter(|i| !out.hidden.contains(i)) {
        let params = ind.params().1;
        let vals = values(&out.inputs, ind);
        let inp = Inputs { params, vals: &vals };
        let mut used = vec![false; params.len()];
        for (i, p) in params.iter().enumerate() {
            if used[i] || !inp.active(i) || !matches!(p.kind, Kind::Price | Kind::Time) {
                continue;
            }
            // la paire prix + heure (même groupe, même inline) : un point
            let pair = (!p.inline.is_empty())
                .then(|| {
                    params.iter().enumerate().position(|(k, q)| {
                        k != i
                            && !used[k]
                            && q.group == p.group
                            && q.inline == p.inline
                            && matches!((p.kind, q.kind), (Kind::Price, Kind::Time) | (Kind::Time, Kind::Price))
                    })
                })
                .flatten();
            used[i] = true;
            let mut m = Mark {
                ind,
                price: None,
                time: None,
                label: p.label,
            };
            for k in std::iter::once(i).chain(pair) {
                used[k] = true;
                if params[k].kind == Kind::Price {
                    m.price = Some(k)
                } else {
                    m.time = Some(k)
                }
            }
            v.push(m);
        }
    }
    v
}

/// Où est le repère : (x, y) ; `None` pour l'axe qui n'est pas utilisé (ligne).
fn place(m: &Mark, out: &IndOut, g: &Geo) -> (Option<f64>, Option<f64>) {
    let vals = values(&out.inputs, m.ind);
    let inp = Inputs {
        params: m.ind.params().1,
        vals: &vals,
    };
    let get = |k: usize| inp.get(m.ind.params().1[k].key);
    // heure pas encore choisie (0) : 200 bougies en arrière, comme le calcul (ex. l'ancrage du VWAP)
    let auto = || g.bars.get(g.bars.len().saturating_sub(200)).map_or(0, |b| b.time);
    let x = m.time.map(get).map(|t| if t > 0.0 { (t / 1000.0) as i64 } else { auto() }).map(|t| g.x_of(t));
    let y = m.price.map(|k| g.scale.y_of(get(k)));
    (x, y)
}

/// Dessine les repères (lignes pointillées et points, avec leur nom).
pub fn draw(pm: &mut Pixmap, text: &Text, g: &Geo, out: &IndOut, grabbed: Option<Mark>) {
    let (w, h) = (g.vp.plot_w(), g.vp.plot_h());
    let c = crate::palette::accent();
    for m in marks(out) {
        let on = grabbed == Some(m);
        let (x, y) = place(&m, out, g);
        let tag = |pm: &mut Pixmap, x: f64, y: f64| {
            let s = format!("📍 {}", m.label);
            let r = Rect::new(x, y, text.width(&s, 11.5) as f64 + 14.0, 20.0);
            round_fill(pm, r, 5.0, c, if on { 1.0 } else { 0.85 });
            text.draw(pm, &s, ((r.x + 7.0) as f32, (r.y + 14.5) as f32), 11.5, crate::palette::WHITE, Align::Left);
        };
        match (x, y) {
            (Some(x), Some(y)) => {
                let r = Rect::new(x - 6.0, y - 6.0, 12.0, 12.0);
                round_fill(pm, r, 6.0, c, 1.0);
                round_stroke(pm, r, 6.0, crate::palette::WHITE, if on { 2.0 } else { 1.2 });
                tag(pm, x + 10.0, y - 26.0);
            }
            (None, Some(y)) if (0.0..h).contains(&y) => {
                dashed(pm, (0.0, y), (w, y), c, if on { 2.0 } else { 1.2 });
                tag(pm, 8.0, y - 22.0);
            }
            (Some(x), None) if (0.0..w).contains(&x) => {
                dashed(pm, (x, 0.0), (x, h), c, if on { 2.0 } else { 1.2 });
                tag(pm, x + 4.0, h - 28.0);
            }
            _ => {}
        }
    }
}

fn dashed(pm: &mut Pixmap, a: (f64, f64), b: (f64, f64), c: crate::text::Rgb, w: f32) {
    for d in crate::toolgeom::dashes(&[a, b], 6.0, 4.0) {
        let pts: Vec<(f32, f32)> = d.iter().map(|q| (q.0 as f32, q.1 as f32)).collect();
        crate::shapes::lines(pm, 0.0, 0.0, &[&pts], c, w);
    }
}

/// Repère sous la souris (coordonnées de la chart).
pub fn hit(out: &IndOut, g: &Geo, mx: f64, my: f64) -> Option<Mark> {
    marks(out).into_iter().find(|m| match place(m, out, g) {
        (Some(x), Some(y)) => (mx - x).hypot(my - y) <= GRAB + 4.0,
        (None, Some(y)) => (my - y).abs() <= GRAB,
        (Some(x), None) => (mx - x).abs() <= GRAB,
        _ => false,
    })
}

/// Nouvelles valeurs quand on glisse le repère en `(mx, my)` : (index de l'input, valeur), déjà bornées.
pub fn drag(m: &Mark, g: &Geo, mx: f64, my: f64) -> Vec<(usize, f64)> {
    let params = m.ind.params().1;
    let mut v = vec![];
    if let Some(k) = m.price {
        v.push((k, argos_script::inputs::clamp(&params[k], g.scale.price_of(my))));
    }
    if let Some(k) = m.time {
        v.push((k, g.t_of(mx) as f64 * 1000.0));
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indicators::{compute, Settings};
    use crate::viewport::{PriceScale, Slots, Viewport};
    use argos_core::Bar;

    #[test]
    fn repere_ancrage_glisse() {
        let bars: Vec<Bar> = (0..120)
            .map(|i| Bar {
                time: i * 60,
                open: 100.0,
                high: 101.0 + (i % 5) as f64,
                low: 99.0,
                close: 100.5,
                volume: 10.0,
            })
            .collect();
        let mut set = Settings::new();
        let mut v = argos_script::inputs::defaults(Ind::AnchoredVwap.params().1);
        v[0] = 3_000_000.0; // bougie de 3000 s (index 50)
        set.insert(Ind::AnchoredVwap, v);
        let out = compute(&bars, 60, &[Ind::AnchoredVwap], &set, None, 0, &Default::default(), &[]);
        let m = marks(&out);
        assert_eq!(m.len(), 1);
        assert_eq!((m[0].time, m[0].price, m[0].label), (Some(0), None, "Ancrage"), "heure seule : ligne verticale");
        let vp = Viewport {
            width: 1000,
            height: 600,
            bar_spacing: 7.0,
            right_offset: 0.0,
            ..Default::default()
        };
        let slots = Slots::uniform(bars.len());
        let scale = PriceScale::fit(&bars, 0, bars.len() - 1, 600.0);
        let times: Vec<i64> = bars.iter().map(|b| b.time).collect();
        let g = Geo {
            vp: &vp,
            scale: &scale,
            slots: &slots,
            times: &times,
            bars: &bars,
            tf: 60,
            mouse: None,
        };
        let x = g.x_of(3000);
        assert_eq!(hit(&out, &g, x + 3.0, 300.0), Some(m[0]), "attrapé près de la ligne");
        assert_eq!(hit(&out, &g, x + 40.0, 300.0), None);
        // glisser 10 bougies plus loin : l'ancrage suit
        let x10 = g.x_of(3600);
        assert_eq!(drag(&m[0], &g, x10, 300.0), vec![(0, 3_600_000.0)]);
        let mut pm = crate::px::new(1000, 600).unwrap();
        draw(&mut pm, &Text::default(), &g, &out, Some(m[0]));
    }
}
