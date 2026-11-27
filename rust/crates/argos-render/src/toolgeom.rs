//! 📐 Phase C — la forme de chaque outil de dessin, **comme Argos v1 / TradingView**, par famille : lignes (info,
//! angle, régression), canaux, fourchettes, Fibonacci, Gann, figures, vagues d'Elliott, cycles, prévisions, volume,
//! formes, annotations.
//!
//! Chaque outil décrit sa forme une seule fois (`geom`) : des traits, des zones remplies et des étiquettes, en pixels.
//! Le **dessin** et le **toucher** (la souris sur l'objet) lisent cette même forme : pas de géométrie écrite deux fois.
//! Un outil dont les points ne sont pas encore tous placés (aperçu pendant le placement) se montre avec ce qu'il a.

use crate::drawings::{Geo, Point};
use crate::text::{Align, Rgb};

pub type Px = (f64, f64);

/// Une étiquette : texte, position (bas du texte ; milieu, ou bord droit avec `Align::Right`), couleur, avec ou
/// sans fond.
#[derive(Debug, Clone, PartialEq)]
pub struct Label {
    pub text: String,
    pub at: Px,
    pub color: Rgb,
    pub boxed: bool,
    pub align: Align,
}

/// La forme d'un objet.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Geom {
    /// Traits pleins (couleur de l'objet sauf indication).
    pub lines: Vec<(Vec<Px>, Option<Rgb>)>,
    /// Traits en tirets.
    pub dashed: Vec<(Vec<Px>, Option<Rgb>)>,
    /// Zones remplies : contour, couleur (`None` : celle de l'objet), opacité.
    pub fills: Vec<(Vec<Px>, Option<Rgb>, f32)>,
    pub labels: Vec<Label>,
    /// Trait épais (surligneur).
    pub thick: bool,
}

impl Geom {
    fn line(&mut self, pts: Vec<Px>) {
        self.lines.push((pts, None));
    }
    fn line_c(&mut self, pts: Vec<Px>, c: Rgb) {
        self.lines.push((pts, Some(c)));
    }
    fn dash(&mut self, pts: Vec<Px>) {
        self.dashed.push((pts, None));
    }
    fn fill(&mut self, pts: Vec<Px>, alpha: f32) {
        self.fills.push((pts, None, alpha));
    }
    fn label(&mut self, text: impl Into<String>, at: Px, color: Rgb, boxed: bool) {
        self.labels.push(Label {
            text: text.into(),
            at,
            color,
            boxed,
            align: Align::Center,
        });
    }
    /// Étiquette qui finit en `at` (à gauche d'une ligne, comme les niveaux de Fibonacci de TradingView).
    fn label_left(&mut self, text: impl Into<String>, at: Px, color: Rgb) {
        self.labels.push(Label {
            text: text.into(),
            at,
            color,
            boxed: false,
            align: Align::Right,
        });
    }

    /// Tous les segments (pour le toucher).
    pub fn segments(&self) -> impl Iterator<Item = (Px, Px)> + '_ {
        self.lines.iter().chain(&self.dashed).flat_map(|(p, _)| p.windows(2).map(|w| (w[0], w[1])))
    }
}

/// Niveaux de Fibonacci les plus utilisés, avec les couleurs d'Argos v1.
pub const FIB_LEVELS: [(f64, Rgb); 7] = [
    (0.0, Rgb(0x95, 0x98, 0xa1)),
    (0.236, Rgb(0xf2, 0x36, 0x45)),
    (0.382, Rgb(0xff, 0x98, 0x00)),
    (0.5, Rgb(0x4c, 0xaf, 0x50)),
    (0.618, Rgb(0x08, 0x99, 0x81)),
    (0.786, Rgb(0x29, 0x62, 0xff)),
    (1.0, Rgb(0x95, 0x98, 0xa1)),
];
/// Extension de Fibonacci : les niveaux et couleurs d'Argos v1 (FIBX / FIBC).
const FIB_EXT: [(f64, Rgb); 10] = [
    (0.0, Rgb(0x95, 0x98, 0xa1)),
    (0.236, Rgb(0xf2, 0x36, 0x45)),
    (0.382, Rgb(0xff, 0x98, 0x00)),
    (0.5, Rgb(0x4c, 0xaf, 0x50)),
    (0.618, Rgb(0x08, 0x99, 0x81)),
    (0.786, Rgb(0x00, 0xbc, 0xd4)),
    (1.0, Rgb(0x95, 0x98, 0xa1)),
    (1.272, Rgb(0x29, 0x62, 0xff)),
    (1.618, Rgb(0x9c, 0x27, 0xb0)),
    (2.618, Rgb(0xe9, 0x1e, 0x63)),
];
/// Canal de Fibonacci : **ton** canal d'Argos v1 (niveau → couleur), des deux côtés de la ligne de base.
const FIB_CHANNEL: [(f64, Rgb); 21] = [
    (0.0, Rgb(0xff, 0xff, 0xff)),
    (0.113, Rgb(0xe6, 0x00, 0x00)),
    (0.236, Rgb(0xe6, 0x00, 0x00)),
    (0.382, Rgb(0xe6, 0x00, 0x00)),
    (0.5, Rgb(0xff, 0xff, 0xff)),
    (0.618, Rgb(0xff, 0xc1, 0x2b)),
    (0.786, Rgb(0xe6, 0x00, 0x00)),
    (0.886, Rgb(0xe6, 0x00, 0x00)),
    (1.0, Rgb(0xff, 0xff, 0xff)),
    (1.272, Rgb(0xe6, 0x00, 0x00)),
    (1.618, Rgb(0xff, 0xc1, 0x2b)),
    (1.786, Rgb(0xe6, 0x00, 0x00)),
    (1.886, Rgb(0x67, 0x3a, 0xb7)),
    (-0.113, Rgb(0xe6, 0x00, 0x00)),
    (-0.236, Rgb(0xe6, 0x00, 0x00)),
    (-0.5, Rgb(0xff, 0xff, 0xff)),
    (-0.618, Rgb(0xff, 0x98, 0x00)),
    (-0.786, Rgb(0xff, 0xc1, 0x2b)),
    (-0.886, Rgb(0x67, 0x3a, 0xb7)),
    (-1.618, Rgb(0xff, 0xc1, 0x2b)),
    (-1.786, Rgb(0xe6, 0x00, 0x00)),
];
const FIB_TIME: [f64; 10] = [0.0, 1.0, 2.0, 3.0, 5.0, 8.0, 13.0, 21.0, 34.0, 55.0];
const FIB_RATIOS: [f64; 5] = [0.236, 0.382, 0.5, 0.618, 1.0];
const GANN_FAN: [(f64, &str); 9] = [
    (8.0, "1×8"),
    (4.0, "1×4"),
    (3.0, "1×3"),
    (2.0, "1×2"),
    (1.0, "1×1"),
    (0.5, "2×1"),
    (1.0 / 3.0, "3×1"),
    (0.25, "4×1"),
    (0.125, "8×1"),
];
const GANN_GRID: [f64; 5] = [0.25, 0.382, 0.5, 0.618, 0.75];

/// Plage de ratios (min, max).
type Ratios = (f64, f64);
/// Figures harmoniques (ratios de Scott Carney) : nom, plages de AB/XA, BC/AB, AD/XA.
const HARMONICS: [(&str, &str, Ratios, Ratios, Ratios); 5] = [
    ("gartley", "Gartley", (0.588, 0.648), (0.382, 0.886), (0.756, 0.816)),
    ("bat", "Bat", (0.382, 0.5), (0.382, 0.886), (0.856, 0.916)),
    ("butterfly", "Butterfly", (0.756, 0.816), (0.382, 0.886), (1.27, 1.618)),
    ("crab", "Crab", (0.382, 0.618), (0.382, 0.886), (1.588, 1.648)),
    ("shark", "Shark", (0.382, 0.886), (1.13, 1.618), (0.886, 1.13)),
];
/// Facteurs des cercles et arcs des outils Sonic (cercles tangents au point A, comme Vela) : cercles, arcs.
const SONIC: [(&str, &[f64], &[f64]); 4] = [
    ("sonic", &[1.0, 2.0, 3.0], &[1.0, 1.5, 2.0, 3.0, 5.0, 8.0]),
    ("supersonic", &[1.0, 2.0, 3.0, 4.0, 5.0], &[1.0, 1.5, 2.0, 3.0, 5.0, 8.0, 13.0, 21.0]),
    ("gsonic", &[0.618, 1.0, 1.618], &[0.618, 1.0, 1.618, 2.618, 4.236, 6.854]),
    (
        "gsupersonic",
        &[0.382, 0.618, 1.0, 1.618, 2.618],
        &[0.382, 0.618, 1.0, 1.618, 2.618, 4.236, 6.854, 11.09],
    ),
];
const GREY: Rgb = Rgb(0x95, 0x98, 0xa1);
const WHITE: Rgb = Rgb(0xe6, 0xe8, 0xee);

fn lerp(a: Px, b: Px, k: f64) -> Px {
    (a.0 + (b.0 - a.0) * k, a.1 + (b.1 - a.1) * k)
}
fn mid(a: Px, b: Px) -> Px {
    lerp(a, b, 0.5)
}
fn dist(a: Px, b: Px) -> f64 {
    (b.0 - a.0).hypot(b.1 - a.1)
}
fn add(a: Px, d: Px) -> Px {
    (a.0 + d.0, a.1 + d.1)
}
fn sub(a: Px, b: Px) -> Px {
    (a.0 - b.0, a.1 - b.1)
}

/// Outils Price Action (comme LuxAlgo) : un clic sur la bougie ; la zone suit les réglages de l'objet (prolonger jusqu'au
/// comblement / toujours / pas du tout, couleurs haussier et baissier, ligne médiane, bordure, FVG inversé à la clôture).
pub fn price_action(tool: &str, pts: &[Point], g: &Geo, st: &crate::drawings::Style) -> Option<Geom> {
    if !matches!(tool, "fvg" | "ob" | "liq") || pts.is_empty() || g.bars.is_empty() {
        return None;
    }
    let mut o = Geom::default();
    let n = g.bars.len();
    let w = g.vp.plot_w();
    let at = g.bars.partition_point(|b| b.time < pts[0].t).min(n - 1);
    let hex = |s: &str, def: Rgb| if s.trim_start_matches('#').len() == 6 { Rgb::hex(s) } else { def };
    let (bull, bear) = (hex(&st.target_color, crate::palette::up()), hex(&st.stop_color, crate::palette::down()));
    // fin de la zone : là où elle est comblée (0), toujours au bord (1), ou 3 bougies plus loin (2)
    let end = |hit: Option<usize>, from: usize| match st.extend {
        1 => w,
        2 => g.x_of(g.bars[(from + 3).min(n - 1)].time),
        _ => hit.map_or(w, |k| g.x_of(g.bars[k].time)),
    };
    let zone = |o: &mut Geom, x0: f64, x1: f64, top: f64, bot: f64, c: Rgb, txt: &str| {
        let (yt, yb) = (g.y_of(top), g.y_of(bot));
        let rect = vec![(x0, yt), (x1, yt), (x1, yb), (x0, yb)];
        o.fills.push((rect.clone(), Some(c), 0.22));
        if st.border {
            o.lines.push(([rect.as_slice(), &[rect[0]]].concat(), Some(c)));
        }
        if st.median {
            o.dashed.push((vec![(x0, (yt + yb) / 2.0), (x1, (yt + yb) / 2.0)], Some(c)));
        }
        o.labels.push(Label {
            text: txt.into(),
            at: (x1 - 4.0, yt.min(yb) + 12.0),
            color: c,
            boxed: false,
            align: Align::Right,
        });
    };
    match tool {
        // FVG : écart entre la bougie d'avant et celle d'après (la bougie cliquée, ou sa voisine qui en a un)
        "fvg" => {
            let gap = |i: usize| -> Option<(f64, f64, bool)> {
                let (a, c) = (g.bars.get(i.checked_sub(1)?)?, g.bars.get(i + 1)?);
                if c.low > a.high {
                    Some((c.low, a.high, true))
                } else if c.high < a.low {
                    Some((a.low, c.high, false))
                } else {
                    None
                }
            };
            match [at, at + 1, at.saturating_sub(1)].into_iter().find_map(|i| gap(i).map(|z| (i, z))) {
                Some((i, (top, bot, up))) => {
                    // comblé quand le prix traverse toute la zone
                    let filled = (i + 2..n).find(|&k| if up { g.bars[k].low <= bot } else { g.bars[k].high >= top });
                    let x0 = g.x_of(g.bars[i - 1].time);
                    // inversé à la clôture : une clôture de l'autre côté fait un iFVG, de la couleur inverse, jusqu'au bord
                    let flip = st
                        .invert
                        .then(|| (i + 2..n).find(|&k| if up { g.bars[k].close < bot } else { g.bars[k].close > top }))
                        .flatten();
                    match flip {
                        Some(k) => {
                            let xk = g.x_of(g.bars[k].time);
                            zone(&mut o, x0, xk, top, bot, if up { bull } else { bear }, if up { "FVG ▲" } else { "FVG ▼" });
                            zone(&mut o, xk, w, top, bot, if up { bear } else { bull }, if up { "iFVG ▼" } else { "iFVG ▲" });
                        }
                        None => zone(
                            &mut o,
                            x0,
                            end(filled, i),
                            top,
                            bot,
                            if up { bull } else { bear },
                            if up { "FVG ▲" } else { "FVG ▼" },
                        ),
                    }
                }
                None => o.label("Pas de FVG ici", add(g.xy(&pts[0]), (0.0, -10.0)), GREY, true),
            }
        }
        // Order Block : la bougie cliquée ; baissière = OB haussier, haussière = OB baissier ; mitigé quand une clôture
        // repasse de l'autre côté
        "ob" => {
            let b = g.bars[at];
            let up = b.close < b.open;
            let gone = (at + 1..n).find(|&k| if up { g.bars[k].close < b.low } else { g.bars[k].close > b.high });
            zone(&mut o, g.x_of(b.time), end(gone, at), b.high, b.low, if up { bull } else { bear }, "OB");
        }
        // Liquidité : le haut (clic au-dessus du milieu) ou le bas de la bougie ; balayée quand une mèche le dépasse
        _ => {
            let b = g.bars[at];
            let high = pts[0].p >= (b.high + b.low) / 2.0;
            let lvl = if high { b.high } else { b.low };
            let swept = (at + 1..n).find(|&k| if high { g.bars[k].high > lvl } else { g.bars[k].low < lvl });
            let (x0, x1, y) = (g.x_of(b.time), end(swept, at), g.y_of(lvl));
            let c = if high { bear } else { bull };
            o.line_c(vec![(x0, y), (x1, y)], c);
            o.labels.push(Label {
                text: "$$$".into(),
                at: (x1 - 4.0, y + if high { -4.0 } else { 13.0 }),
                color: c,
                boxed: false,
                align: Align::Right,
            });
            if swept.is_some() && st.extend == 0 {
                o.label("✕", (x1, y + 4.0), c, false);
            }
        }
    }
    Some(o)
}

fn gcd(a: i64, b: i64) -> i64 {
    if b == 0 {
        a.abs()
    } else {
        gcd(b, a % b)
    }
}

/// Demi-droite de `a` dans la direction de `b`, prolongée loin hors de la chart.
fn ray(a: Px, b: Px, g: &Geo) -> Vec<Px> {
    let (dx, dy) = sub(b, a);
    let len = dx.hypot(dy);
    if len == 0.0 {
        return vec![a, b];
    }
    let k = (g.vp.plot_w() + g.vp.plot_h()) * 3.0 / len;
    vec![a, add(a, (dx * k, dy * k))]
}

/// Cercle (ou arc, degrés : 0 = droite, sens trigonométrique à l'écran) de centre `c` et rayons `rx`, `ry`.
fn ellipse(c: Px, rx: f64, ry: f64, from: f64, to: f64) -> Vec<Px> {
    let n = 64;
    (0..=n)
        .map(|k| {
            let t = (from + (to - from) * k as f64 / n as f64).to_radians();
            (c.0 + rx * t.cos(), c.1 - ry * t.sin())
        })
        .collect()
}

/// Courbe de Bézier (2 ou 3 points de contrôle) en ligne brisée.
fn bezier(p: &[Px]) -> Vec<Px> {
    (0..=40)
        .map(|k| {
            let t = k as f64 / 40.0;
            let mut q = p.to_vec();
            while q.len() > 1 {
                q = q.windows(2).map(|w| lerp(w[0], w[1], t)).collect();
            }
            q[0]
        })
        .collect()
}

/// Étiquettes des figures et des vagues (une par point).
fn names(tool: &str) -> &'static [&'static str] {
    match tool {
        "xabcd" | "cypher" | "gartley" | "bat" | "butterfly" | "crab" | "shark" => &["X", "A", "B", "C", "D"],
        "hs" => &["", "Épaule", "", "Tête", "", "Épaule", ""],
        "abcd" | "tripattern" => &["A", "B", "C", "D"],
        "threedrives" => &["", "1", "", "2", "", "3", ""],
        "ewimp" => &["0", "1", "2", "3", "4", "5"],
        "ewcorr" => &["0", "A", "B", "C"],
        "ewtri" => &["0", "A", "B", "C", "D", "E"],
        "ewdouble" => &["0", "W", "X", "Y"],
        "ewtriple" => &["0", "W", "X", "Y", "X", "Z"],
        _ => &[],
    }
}

/// Étiquette au-dessus d'un sommet, au-dessous d'un creux (selon ses voisins).
fn peak_label(p: &[Px], i: usize) -> Px {
    let y = p[i].1;
    let others: Vec<f64> = [i.checked_sub(1), (i + 1 < p.len()).then_some(i + 1)]
        .into_iter()
        .flatten()
        .map(|j| p[j].1)
        .collect();
    let top = others.iter().all(|o| y <= *o);
    (p[i].0, if top { y - 8.0 } else { y + 18.0 })
}

/// La forme de l'outil `tool` (`None` : outil dessiné ailleurs — lignes simples, rectangle, Fibonacci, positions,
/// règle, textes, flèches de marquage — ou pas encore assez de points).
pub fn geom(tool: &str, pts: &[Point], g: &Geo, label: &dyn Fn(f64) -> String) -> Option<Geom> {
    let p: Vec<Px> = pts.iter().map(|q| g.xy(q)).collect();
    let need = crate::tools::def(tool).map_or(1, |d| (d.points as usize).min(7));
    let mut o = Geom::default();
    let (w, h) = (g.vp.plot_w(), g.vp.plot_h());
    match tool {
        // ── lignes ──────────────────────────────────────────────────────────────────────────────────────────────
        "infoline" if p.len() >= 2 => {
            o.line(vec![p[0], p[1]]);
            let (a, b) = (pts[0], pts[1]);
            let bars = (g.times.partition_point(|&t| t <= b.t) as i64 - g.times.partition_point(|&t| t <= a.t) as i64).abs();
            let ang = (-(p[1].1 - p[0].1)).atan2(p[1].0 - p[0].0).to_degrees();
            let s = format!("{} ({:+.2} %) · {bars} bougies · {ang:.0}°", label(b.p - a.p), (b.p / a.p - 1.0) * 100.0);
            o.label(s, add(p[1], (0.0, -10.0)), WHITE, true);
        }
        "trendangle" if p.len() >= 2 => {
            o.line(vec![p[0], p[1]]);
            let r = dist(p[0], p[1]).clamp(20.0, 60.0);
            o.dash(vec![p[0], add(p[0], (r + 20.0, 0.0))]);
            let ang = (-(p[1].1 - p[0].1)).atan2(p[1].0 - p[0].0).to_degrees();
            o.line(ellipse(p[0], r, r, 0.0, ang));
            o.label(format!("{ang:.1}°"), add(p[0], (r + 26.0, 4.0)), WHITE, false);
        }
        "regression" if p.len() >= 2 => {
            // droite des moindres carrés sur les clôtures entre les 2 points, ± 2 écarts-types
            let (t0, t1) = (pts[0].t.min(pts[1].t), pts[0].t.max(pts[1].t));
            let sel: Vec<(f64, f64)> = g
                .bars
                .iter()
                .filter(|b| b.time >= t0 && b.time <= t1)
                .map(|b| (g.x_of(b.time), b.close))
                .collect();
            if sel.len() < 2 {
                o.line(vec![p[0], p[1]]);
            } else {
                let n = sel.len() as f64;
                let (mx, my) = (sel.iter().map(|s| s.0).sum::<f64>() / n, sel.iter().map(|s| s.1).sum::<f64>() / n);
                let sxx: f64 = sel.iter().map(|s| (s.0 - mx).powi(2)).sum();
                let slope = if sxx > 0.0 {
                    sel.iter().map(|s| (s.0 - mx) * (s.1 - my)).sum::<f64>() / sxx
                } else {
                    0.0
                };
                let at = |x: f64| my + slope * (x - mx);
                let sd = (sel.iter().map(|s| (s.1 - at(s.0)).powi(2)).sum::<f64>() / n).sqrt();
                let (xa, xb) = (sel[0].0, sel[sel.len() - 1].0);
                for (k, c) in [(0.0, None), (2.0, Some(GREY)), (-2.0, Some(GREY))] {
                    let seg = vec![(xa, g.y_of(at(xa) + k * sd)), (xb, g.y_of(at(xb) + k * sd))];
                    match c {
                        None => o.line(seg),
                        Some(_) => o.dash(seg),
                    }
                }
                let band = vec![
                    (xa, g.y_of(at(xa) + 2.0 * sd)),
                    (xb, g.y_of(at(xb) + 2.0 * sd)),
                    (xb, g.y_of(at(xb) - 2.0 * sd)),
                    (xa, g.y_of(at(xa) - 2.0 * sd)),
                ];
                o.fill(band, 0.08);
            }
        }
        // ── canaux ──────────────────────────────────────────────────────────────────────────────────────────────
        "channel" | "fibch" if p.len() >= 2 => {
            let off = p.get(2).map_or((0.0, 40.0), |c| {
                (0.0, c.1 - (p[0].1 + (p[1].1 - p[0].1) * ((c.0 - p[0].0) / (p[1].0 - p[0].0).max(1e-9))))
            });
            let (a2, b2) = (add(p[0], off), add(p[1], off));
            if tool == "channel" {
                o.line(vec![p[0], p[1]]);
                o.line(vec![a2, b2]);
                o.dash(vec![mid(p[0], a2), mid(p[1], b2)]);
                o.fill(vec![p[0], p[1], b2, a2], 0.12);
            } else {
                for (k, c) in FIB_CHANNEL {
                    let (a, b) = (lerp(p[0], a2, k), lerp(p[1], b2, k));
                    o.line_c(vec![a, b], c);
                    o.label(format!("{k}"), add(a, (-14.0, 4.0)), c, false);
                }
            }
        }
        "flattop" if p.len() >= 2 => {
            o.line(vec![p[0], p[1]]);
            let y = p.get(2).map_or(p[0].1 - 40.0, |c| c.1);
            o.line(vec![(p[0].0, y), (p[1].0, y)]);
            o.fill(vec![p[0], p[1], (p[1].0, y), (p[0].0, y)], 0.12);
        }
        "disjoint" if p.len() >= 2 => {
            o.line(vec![p[0], p[1]]);
            let c = p.get(2).copied().unwrap_or(add(p[0], (0.0, 40.0)));
            // 2e ligne : pente inverse, de même longueur
            let d = (p[1].0 - p[0].0, -(p[1].1 - p[0].1));
            let c2 = add(c, d);
            o.line(vec![c, c2]);
            o.fill(vec![p[0], p[1], c2, c], 0.12);
        }
        // ── fourchettes ─────────────────────────────────────────────────────────────────────────────────────────
        "pitchfork" | "schiff" | "mschiff" | "insidepf" if p.len() >= 3 => {
            let m = mid(p[1], p[2]);
            let origin = match tool {
                "schiff" => (p[0].0, (p[0].1 + p[1].1) / 2.0),
                "mschiff" => mid(p[0], p[1]),
                "insidepf" => mid(p[0], m),
                _ => p[0],
            };
            let dir = sub(m, origin);
            o.line(vec![p[0], origin]);
            o.line(ray(origin, m, g));
            o.line(vec![p[1], p[2]]);
            for q in [p[1], p[2]] {
                o.line(ray(q, add(q, dir), g));
            }
            o.fill(
                vec![p[1], add(p[1], (dir.0 * 4.0, dir.1 * 4.0)), add(p[2], (dir.0 * 4.0, dir.1 * 4.0)), p[2]],
                0.08,
            );
        }
        "pitchfan" if p.len() >= 3 => {
            for (k, c) in FIB_LEVELS {
                o.line_c(ray(p[0], lerp(p[1], p[2], k), g), c);
            }
            o.line(vec![p[1], p[2]]);
        }
        // ── Fibonacci ───────────────────────────────────────────────────────────────────────────────────────────
        "fibext" if p.len() >= 3 => {
            o.dash(vec![p[0], p[1], p[2]]);
            let d = pts[1].p - pts[0].p;
            let (x1, x2) = (p[2].0, p[2].0.max(p[1].0) + 80.0);
            for (lv, c) in FIB_EXT {
                let price = pts[2].p + d * lv;
                let y = g.y_of(price);
                o.line_c(vec![(x1, y), (x2, y)], c);
                o.label_left(format!("{lv} ({})", label(price)), (x1 - 6.0, y + 4.0), c);
            }
        }
        // extension simple (comme LuxAlgo) : niveaux mesurés de A vers B, prolongés au-delà de B
        "fibext2" if p.len() >= 2 => {
            o.dash(vec![p[0], p[1]]);
            let d = pts[1].p - pts[0].p;
            let (x1, x2) = (p[0].0.min(p[1].0), p[0].0.max(p[1].0) + 80.0);
            for (lv, c) in FIB_EXT.iter().copied().chain([(3.618, Rgb(0x9c, 0x27, 0xb0)), (4.236, Rgb(0xff, 0x98, 0x00))]) {
                let price = pts[0].p + d * lv;
                let y = g.y_of(price);
                o.line_c(vec![(x1, y), (x2, y)], c);
                o.label_left(format!("{lv} ({})", label(price)), (x1 - 6.0, y + 4.0), c);
            }
        }
        // Sonic (comme Vela) : cercles numérotés tangents en A (dans la direction de B), tangente en A, arcs de l'autre côté
        t if SONIC.iter().any(|s| s.0 == t) && p.len() >= 2 => {
            let (_, circles, arcs) = SONIC.iter().find(|s| s.0 == t).copied().unwrap_or(SONIC[0]);
            let ab = sub(p[1], p[0]);
            let len = dist(p[0], p[1]).max(1.0);
            let u = (ab.0 / len, ab.1 / len);
            let big = (w + h) * 3.0;
            o.line(vec![add(p[0], (u.1 * big, -u.0 * big)), add(p[0], (-u.1 * big, u.0 * big))]);
            for (k, f) in circles.iter().enumerate() {
                let r = len / 2.0 * f;
                let c = FIB_LEVELS[k % FIB_LEVELS.len()].1;
                let ctr = add(p[0], (u.0 * r, u.1 * r));
                o.line_c(ellipse(ctr, r, r, 0.0, 360.0), c);
                o.label(
                    (if f.fract() == 0.0 { format!("{f}") } else { format!("{f:.3}") }).to_string(),
                    add(ctr, (0.0, -r - 6.0)),
                    c,
                    false,
                );
            }
            for (k, f) in arcs.iter().enumerate() {
                let r = len * f;
                let c = FIB_LEVELS[(k + 2) % FIB_LEVELS.len()].1;
                o.line_c(ellipse(add(p[0], (-u.0 * r, -u.1 * r)), r, r, 0.0, 360.0), c);
            }
        }
        // Pavage de Dedekind (comme Vela) : la tessellation modulaire sur la base A → B (demi-cercles de Farey)
        "dedekind" if p.len() >= 2 => {
            let (a, b) = (p[0], p[1]);
            let span = b.0 - a.0;
            let up = if b.1 <= a.1 { -1.0 } else { 1.0 }; // B plus haut que A : les arcs montent
            o.line(vec![a, (b.0, a.1)]);
            // fractions de Farey d'ordre 6 entre 0 et 1, et un demi-cercle entre chaque paire voisine (b·c − a·d = 1)
            let n = 6i64;
            let mut fr: Vec<(i64, i64)> = (1..=n).flat_map(|q| (0..=q).map(move |k| (k, q))).filter(|&(k, q)| gcd(k, q) == 1).collect();
            fr.sort_by(|x, y| (x.0 * y.1).cmp(&(y.0 * x.1)));
            fr.dedup();
            for (i, x) in fr.iter().enumerate() {
                for y in &fr[i + 1..] {
                    if y.0 * x.1 - x.0 * y.1 == 1 {
                        let (x0, x1) = (a.0 + span * x.0 as f64 / x.1 as f64, a.0 + span * y.0 as f64 / y.1 as f64);
                        let r = (x1 - x0).abs() / 2.0;
                        let arc = ellipse(((x0 + x1) / 2.0, a.1), r, r, 0.0, 180.0);
                        o.line(arc.into_iter().map(|q| (q.0, a.1 + (q.1 - a.1) * -up)).collect());
                    }
                }
                let xv = a.0 + span * x.0 as f64 / x.1 as f64;
                if x.1 == 1 {
                    o.dash(vec![(xv, a.1), (xv, a.1 + up * span.abs())]);
                }
            }
        }
        // loupe : le rectangle de la plage à agrandir (la chart s'y cadre au 2e clic)
        "loupe" | "aicapture" if p.len() >= 2 => {
            let (a, b) = (p[0], p[1]);
            o.dash(vec![a, (b.0, a.1), b, (a.0, b.1), a]);
            o.fill(vec![a, (b.0, a.1), b, (a.0, b.1)], 0.08);
        }
        "fibtime" | "fibtrendtime" if p.len() >= 2 => {
            let (start, step) = if tool == "fibtime" {
                (pts[0].t, pts[1].t - pts[0].t)
            } else {
                (pts.get(2).map_or(pts[1].t, |q| q.t), pts[1].t - pts[0].t)
            };
            if tool == "fibtrendtime" {
                o.dash(p.clone());
            }
            for (k, n) in FIB_TIME.iter().enumerate() {
                let x = g.x_of(start + (step as f64 * n) as i64);
                if x > w * 4.0 {
                    break;
                }
                let c = FIB_LEVELS[k % FIB_LEVELS.len()].1;
                o.line_c(vec![(x, 0.0), (x, h)], c);
                o.label(format!("{n}"), (x + 4.0, h - 6.0), c, false);
            }
        }
        "fibfan" if p.len() >= 2 => {
            let (dx, dy) = sub(p[1], p[0]);
            for (k, c) in FIB_LEVELS {
                o.line_c(ray(p[0], add(p[0], (dx, dy * k)), g), c);
                o.line_c(ray(p[0], add(p[0], (dx * k, dy)), g), c);
            }
        }
        "fibcircles" | "fibarcs" if p.len() >= 2 => {
            let (c, r0) = if tool == "fibcircles" {
                (mid(p[0], p[1]), dist(p[0], p[1]) / 2.0)
            } else {
                (p[1], dist(p[0], p[1]))
            };
            let up = p[0].1 > p[1].1;
            // cercles complets, ou demi-cercles tournés vers le 1er point (arcs)
            let (from, to) = match (tool, up) {
                ("fibcircles", _) => (0.0, 360.0),
                (_, true) => (180.0, 360.0),
                _ => (0.0, 180.0),
            };
            for ((_, col), lv) in FIB_LEVELS.iter().skip(1).zip(FIB_RATIOS) {
                let r = r0 * lv;
                o.line_c(ellipse(c, r, r, from, to), *col);
            }
            o.dash(vec![p[0], p[1]]);
        }
        "fibspiral" if p.len() >= 2 => {
            // spirale d'or : le rayon grandit de φ à chaque quart de tour
            let r0 = dist(p[0], p[1]).max(1.0);
            let a0 = (p[1].1 - p[0].1).atan2(p[1].0 - p[0].0);
            let phi = (1.0 + 5f64.sqrt()) / 2.0;
            let pts_s: Vec<Px> = (0..=240)
                .map(|k| {
                    let t = k as f64 / 240.0 * 4.0 * std::f64::consts::TAU;
                    let r = r0 * phi.powf(-t / std::f64::consts::FRAC_PI_2 * 0.5);
                    (p[0].0 + r * (a0 + t).cos(), p[0].1 + r * (a0 + t).sin())
                })
                .collect();
            o.line(pts_s);
        }
        "fibwedge" if p.len() >= 2 => {
            let c = p.get(2).copied().unwrap_or(add(p[1], (0.0, 40.0)));
            o.line(ray(p[0], p[1], g));
            o.line(ray(p[0], c, g));
            let (a1, a2) = (
                (-(p[1].1 - p[0].1)).atan2(p[1].0 - p[0].0).to_degrees(),
                (-(c.1 - p[0].1)).atan2(c.0 - p[0].0).to_degrees(),
            );
            let r0 = dist(p[0], p[1]);
            for ((_, col), lv) in FIB_LEVELS.iter().skip(1).zip(FIB_RATIOS) {
                o.line_c(ellipse(p[0], r0 * lv, r0 * lv, a1, a2), *col);
            }
        }
        // ── Gann ────────────────────────────────────────────────────────────────────────────────────────────────
        "gannbox" | "gannsq" | "gannsqfixed" if p.len() >= 2 => {
            let (a, mut b) = (p[0], p[1]);
            if tool != "gannbox" {
                // carré : même largeur que hauteur à l'écran
                let s = (b.0 - a.0).abs().max((b.1 - a.1).abs());
                b = (a.0 + s * (b.0 - a.0).signum(), a.1 + s * (b.1 - a.1).signum());
            }
            o.line(vec![a, (b.0, a.1), b, (a.0, b.1), a]);
            for k in GANN_GRID {
                let (x, y) = (a.0 + (b.0 - a.0) * k, a.1 + (b.1 - a.1) * k);
                o.line_c(vec![(x, a.1), (x, b.1)], GREY);
                o.line_c(vec![(a.0, y), (b.0, y)], GREY);
            }
            if tool != "gannbox" {
                o.line(vec![a, b]);
                o.line(vec![(a.0, b.1), (b.0, a.1)]);
            }
            o.fill(vec![a, (b.0, a.1), b, (a.0, b.1)], 0.06);
        }
        "gannfan" if p.len() >= 2 => {
            let (dx, dy) = sub(p[1], p[0]);
            for (k, (ratio, name)) in GANN_FAN.iter().enumerate() {
                let c = FIB_LEVELS[k % FIB_LEVELS.len()].1;
                let q = add(p[0], (dx, dy * ratio));
                o.line_c(ray(p[0], q, g), c);
                o.label(*name, add(q, (6.0, 0.0)), c, false);
            }
        }
        // ── figures et vagues d'Elliott ─────────────────────────────────────────────────────────────────────────
        t if !names(t).is_empty() && p.len() >= 2 => {
            o.line(p.clone());
            if matches!(t, "xabcd" | "cypher" | "gartley" | "bat" | "butterfly" | "crab" | "shark") {
                if p.len() >= 3 {
                    o.fill(vec![p[0], p[1], p[2]], 0.15);
                }
                if p.len() >= 5 {
                    o.fill(vec![p[2], p[3], p[4]], 0.15);
                    o.dash(vec![p[0], p[2], p[4]]);
                    o.dash(vec![p[1], p[3]]);
                }
            }
            if t == "tripattern" && p.len() >= 4 {
                o.fill(vec![p[0], p[1], p[2], p[3]], 0.12);
            }
            if t == "hs" && p.len() >= 5 {
                // ligne de cou : entre les deux creux autour de la tête
                o.line_c(ray(p[2], p[4], g), crate::palette::down());
            }
            // harmoniques : ratios sur les traits pointillés, nom ✓ si tous dans les plages de la figure
            if let Some(&(_, name, ab, bc, ad)) = HARMONICS.iter().find(|h| h.0 == t).filter(|_| p.len() >= 5) {
                let r = |a: usize, b: usize, c: usize, d: usize| ((pts[d].p - pts[c].p) / (pts[b].p - pts[a].p)).abs();
                let (rab, rbc, rad) = (r(0, 1, 1, 2), r(1, 2, 2, 3), r(0, 1, 1, 4));
                for (v, a, b) in [(rab, p[0], p[2]), (rbc, p[1], p[3]), (rad, p[0], p[4])] {
                    o.label(format!("{v:.3}"), mid(a, b), GREY, true);
                }
                let ok = [(rab, ab), (rbc, bc), (rad, ad)].iter().all(|(v, (lo, hi))| v >= lo && v <= hi);
                o.label(
                    format!("{name} {}", if ok { "✓" } else { "✗" }),
                    add(mid(p[1], p[3]), (0.0, -24.0)),
                    if ok { crate::palette::up() } else { WHITE },
                    true,
                );
            }
            let is_wave = t.starts_with("ew");
            for (i, n) in names(t).iter().enumerate().take(p.len()) {
                if !n.is_empty() {
                    let at = peak_label(&p, i);
                    o.label(if is_wave { format!("({n})") } else { n.to_string() }, at, WHITE, !is_wave);
                }
            }
        }
        // ── cycles ──────────────────────────────────────────────────────────────────────────────────────────────
        "cyclic" if p.len() >= 2 => {
            let step = (p[1].0 - p[0].0).abs().max(4.0);
            let mut x = p[0].0.min(p[1].0);
            while x < w * 2.0 {
                o.line(vec![(x, 0.0), (x, h)]);
                x += step;
            }
        }
        "timecycles" if p.len() >= 2 => {
            let step = (p[1].0 - p[0].0).abs().max(4.0);
            let mut x = p[0].0.min(p[1].0);
            while x < w * 2.0 {
                o.line(ellipse((x + step / 2.0, p[0].1), step / 2.0, step / 2.0, 0.0, 180.0));
                x += step;
            }
        }
        "sine" if p.len() >= 2 => {
            let (half, amp) = ((p[1].0 - p[0].0).abs().max(4.0), (p[1].1 - p[0].1) / 2.0);
            let base = (p[0].1 + p[1].1) / 2.0;
            let start = p[0].0.min(p[1].0);
            let wave: Vec<Px> = (0..=((w * 2.0 - start).max(0.0) / 3.0) as usize)
                .map(|k| {
                    let x = start + k as f64 * 3.0;
                    (x, base - amp * ((x - start) / half * std::f64::consts::PI).cos())
                })
                .collect();
            o.line(wave);
        }
        // ── prévisions ──────────────────────────────────────────────────────────────────────────────────────────
        "posforecast" if p.len() >= 2 => {
            let up = pts[1].p >= pts[0].p;
            let c = if up { crate::palette::up() } else { crate::palette::down() };
            o.line_c(vec![p[0], p[1]], c);
            o.dash(vec![p[0], (p[1].0, p[0].1)]);
            o.label(
                format!("{} ({:+.2} %)", label(pts[1].p), (pts[1].p / pts[0].p - 1.0) * 100.0),
                add(p[1], (0.0, if up { -10.0 } else { 18.0 })),
                c,
                true,
            );
        }
        "barspattern" if p.len() >= 2 => {
            // copie des bougies entre les 2 premiers points, collée à partir du 3e
            let (t0, t1) = (pts[0].t.min(pts[1].t), pts[0].t.max(pts[1].t));
            let dest = pts.get(2).copied().unwrap_or(pts[1]);
            let src: Vec<&argos_core::Bar> = g.bars.iter().filter(|b| b.time >= t0 && b.time <= t1).collect();
            if let Some(first) = src.first() {
                let shift = dest.p - first.open;
                for b in &src {
                    let x = g.x_of(dest.t + (b.time - t0));
                    o.line_c(vec![(x, g.y_of(b.high + shift)), (x, g.y_of(b.low + shift))], GREY);
                    let c = if b.close >= b.open { crate::palette::up() } else { crate::palette::down() };
                    o.line_c(vec![(x, g.y_of(b.open + shift)), (x, g.y_of(b.close + shift))], c);
                }
            }
            o.dash(vec![p[0], p[1]]);
        }
        "ghostfeed" if p.len() >= 2 => {
            // bougies fantômes le long du trajet : petites bougies en tirets du 1er au 2e point
            let n = ((p[1].0 - p[0].0).abs() / 10.0).clamp(2.0, 60.0) as usize;
            for k in 0..n {
                let (a, b) = (lerp(p[0], p[1], k as f64 / n as f64), lerp(p[0], p[1], (k + 1) as f64 / n as f64));
                let x = (a.0 + b.0) / 2.0;
                let wick: f64 = if k % 2 == 0 { 10.0 } else { 6.0 };
                o.dash(vec![(x, a.1.min(b.1) - wick), (x, a.1.max(b.1) + 6.0)]);
                o.fills.push((
                    vec![(x - 3.0, a.1), (x + 3.0, a.1), (x + 3.0, b.1), (x - 3.0, b.1)],
                    Some(if b.1 < a.1 { crate::palette::up() } else { crate::palette::down() }),
                    0.35,
                ));
            }
        }
        "sector" if p.len() >= 2 => {
            let c = p.get(2).copied().unwrap_or(add(p[1], (0.0, 40.0)));
            let r = dist(p[0], p[1]);
            let (a1, a2) = (
                (-(p[1].1 - p[0].1)).atan2(p[1].0 - p[0].0).to_degrees(),
                (-(c.1 - p[0].1)).atan2(c.0 - p[0].0).to_degrees(),
            );
            let mut shape = vec![p[0]];
            shape.extend(ellipse(p[0], r, r, a1, a2));
            shape.push(p[0]);
            o.line(shape.clone());
            o.fill(shape, 0.15);
        }
        // ── volume ──────────────────────────────────────────────────────────────────────────────────────────────
        "avwap" if !p.is_empty() => {
            let (mut pv, mut v) = (0.0, 0.0);
            let line: Vec<Px> = g
                .bars
                .iter()
                .filter(|b| b.time >= pts[0].t)
                .map(|b| {
                    let typ = (b.high + b.low + b.close) / 3.0;
                    pv += typ * b.volume;
                    v += b.volume;
                    (g.x_of(b.time), g.y_of(if v > 0.0 { pv / v } else { typ }))
                })
                .collect();
            o.line(if line.len() >= 2 { line } else { vec![p[0], add(p[0], (40.0, 0.0))] });
            o.label("VWAP", add(p[0], (0.0, -8.0)), WHITE, true);
        }
        "fvp" | "avp" if !p.is_empty() => {
            // profil de volume : volume par tranche de prix, entre les 2 points (ancré : jusqu'à la dernière bougie)
            let t0 = pts[0].t;
            let t1 = if tool == "fvp" { pts.get(1).map_or(t0, |q| q.t) } else { i64::MAX };
            let (lo_t, hi_t) = (t0.min(t1), t0.max(t1));
            let sel: Vec<&argos_core::Bar> = g.bars.iter().filter(|b| b.time >= lo_t && b.time <= hi_t).collect();
            let (lo, hi) = sel.iter().fold((f64::MAX, f64::MIN), |(l, h2), b| (l.min(b.low), h2.max(b.high)));
            if sel.is_empty() || hi <= lo {
                o.line(p.clone());
            } else {
                const ROWS: usize = 24;
                let mut vol = [0.0f64; ROWS];
                for b in &sel {
                    let k = (((b.close - lo) / (hi - lo)) * (ROWS as f64 - 1.0)).round() as usize;
                    vol[k.min(ROWS - 1)] += b.volume;
                }
                let maxv = vol.iter().cloned().fold(0.0, f64::max).max(1e-9);
                let poc = vol.iter().enumerate().max_by(|a, b| a.1.total_cmp(b.1)).map_or(0, |(i, _)| i);
                let x0 = g.x_of(lo_t);
                let width = (g.x_of(hi_t.min(g.times.last().copied().unwrap_or(hi_t))) - x0).abs().clamp(140.0, 260.0) * 0.6;
                for (k, v) in vol.iter().enumerate() {
                    let (pa, pb) = (lo + (hi - lo) * k as f64 / ROWS as f64, lo + (hi - lo) * (k + 1) as f64 / ROWS as f64);
                    let (ya, yb) = (g.y_of(pa), g.y_of(pb));
                    let len = width * v / maxv;
                    let c = if k == poc { crate::palette::down() } else { Rgb(0x29, 0x62, 0xff) };
                    o.fills
                        .push((vec![(x0, ya - 0.5), (x0 + len, ya - 0.5), (x0 + len, yb + 0.5), (x0, yb + 0.5)], Some(c), 0.45));
                }
                let py = g.y_of(lo + (hi - lo) * (poc as f64 + 0.5) / ROWS as f64);
                o.line_c(vec![(x0, py), (x0 + width * 1.4, py)], crate::palette::down());
                o.label("POC", (x0 + width * 1.4 + 16.0, py + 4.0), crate::palette::down(), false);
                o.dash(vec![(x0, g.y_of(hi)), (x0, g.y_of(lo))]);
            }
        }
        // ── formes ──────────────────────────────────────────────────────────────────────────────────────────────
        "brush" | "path" | "polyline" if p.len() >= 2 => o.line(p.clone()),
        "highlighter" if p.len() >= 2 => {
            o.line(p.clone());
            o.thick = true;
        }
        "arrow" | "arrowmarker" if p.len() >= 2 => {
            let (dx, dy) = sub(p[1], p[0]);
            let len = dx.hypot(dy).max(1e-9);
            let (ux, uy) = (dx / len, dy / len);
            let head = if tool == "arrowmarker" { 16.0 } else { 11.0 };
            let side = (-uy, ux);
            let base = add(p[1], (-ux * head, -uy * head));
            let l = add(base, (side.0 * head * 0.55, side.1 * head * 0.55));
            let r = add(base, (-side.0 * head * 0.55, -side.1 * head * 0.55));
            if tool == "arrowmarker" {
                // flèche pleine : corps + pointe
                let t = 4.0;
                let body = vec![
                    add(p[0], (side.0 * t, side.1 * t)),
                    add(base, (side.0 * t, side.1 * t)),
                    l,
                    p[1],
                    r,
                    add(base, (-side.0 * t, -side.1 * t)),
                    add(p[0], (-side.0 * t, -side.1 * t)),
                ];
                o.fill(body.clone(), 0.9);
                o.line(body);
            } else {
                o.line(vec![p[0], p[1]]);
                o.line(vec![l, p[1], r]);
            }
        }
        "rotrect" if p.len() >= 2 => {
            let c = p.get(2).copied().unwrap_or(add(p[1], (0.0, 30.0)));
            let (dx, dy) = sub(p[1], p[0]);
            let len = dx.hypot(dy).max(1e-9);
            let n = (-dy / len, dx / len);
            let off = (c.0 - p[1].0) * n.0 + (c.1 - p[1].1) * n.1;
            let d = (n.0 * off, n.1 * off);
            let shape = vec![p[0], p[1], add(p[1], d), add(p[0], d), p[0]];
            o.fill(shape.clone(), 0.14);
            o.line(shape);
        }
        "circle" if p.len() >= 2 => {
            let r = dist(p[0], p[1]);
            let c = ellipse(p[0], r, r, 0.0, 360.0);
            o.fill(c.clone(), 0.12);
            o.line(c);
        }
        "ellipse" if p.len() >= 2 => {
            let c = ellipse(mid(p[0], p[1]), (p[1].0 - p[0].0).abs() / 2.0, (p[1].1 - p[0].1).abs() / 2.0, 0.0, 360.0);
            o.fill(c.clone(), 0.12);
            o.line(c);
        }
        "triangle" if p.len() >= 2 => {
            let mut t = p.clone();
            t.push(p[0]);
            o.fill(t.clone(), 0.14);
            o.line(t);
        }
        "arc" | "curve" if p.len() >= 2 => {
            // arc : passe par le 2e point ; courbe : le 2e point tire la courbe
            let ctrl = match (tool, p.get(2)) {
                ("arc", Some(_)) => add(p[1], sub(p[1], mid(p[0], p[2]))),
                _ => p[1],
            };
            let end = p.get(2).copied().unwrap_or(p[1]);
            o.line(bezier(&[p[0], ctrl, end]));
        }
        "dcurve" if p.len() >= 2 => o.line(bezier(&p)),
        // ── annotations ─────────────────────────────────────────────────────────────────────────────────────────
        "pin" | "flag" | "signpost" | "comment" | "note" | "pricelabel" | "icon" | "table" if !p.is_empty() => {
            let a = p[0];
            match tool {
                "pin" => {
                    o.fill(ellipse(add(a, (0.0, -16.0)), 7.0, 7.0, 0.0, 360.0), 1.0);
                    o.line(vec![a, add(a, (0.0, -9.0))]);
                }
                "flag" => {
                    o.line(vec![a, add(a, (0.0, -26.0))]);
                    o.fill(vec![add(a, (0.0, -26.0)), add(a, (16.0, -21.0)), add(a, (0.0, -16.0))], 1.0);
                }
                "signpost" => {
                    o.line(vec![a, add(a, (0.0, -22.0))]);
                    o.fill(
                        vec![
                            add(a, (-2.0, -40.0)),
                            add(a, (92.0, -40.0)),
                            add(a, (102.0, -31.0)),
                            add(a, (92.0, -22.0)),
                            add(a, (-2.0, -22.0)),
                        ],
                        0.9,
                    );
                }
                "table" => {
                    for k in 0..=3 {
                        o.line_c(vec![add(a, (0.0, k as f64 * 14.0)), add(a, (90.0, k as f64 * 14.0))], GREY);
                    }
                    for k in [0.0, 45.0, 90.0] {
                        o.line_c(vec![add(a, (k, 0.0)), add(a, (k, 42.0))], GREY);
                    }
                }
                "icon" => o.fill(ellipse(a, 9.0, 9.0, 0.0, 360.0), 0.9),
                // note : un papillon derrière le texte ; commentaire : une bulle avec sa pointe
                "note" => o.fill(
                    vec![add(a, (-6.0, -14.0)), add(a, (106.0, -14.0)), add(a, (106.0, 12.0)), add(a, (-6.0, 12.0))],
                    0.18,
                ),
                "comment" => {
                    let b = vec![
                        add(a, (-6.0, -14.0)),
                        add(a, (106.0, -14.0)),
                        add(a, (106.0, 12.0)),
                        add(a, (14.0, 12.0)),
                        add(a, (4.0, 22.0)),
                        add(a, (6.0, 12.0)),
                        add(a, (-6.0, 12.0)),
                        add(a, (-6.0, -14.0)),
                    ];
                    o.fill(b.clone(), 0.15);
                    o.line(b);
                }
                "pricelabel" => {
                    o.fill(
                        vec![a, add(a, (8.0, -10.0)), add(a, (70.0, -10.0)), add(a, (70.0, 10.0)), add(a, (8.0, 10.0))],
                        0.9,
                    );
                    o.label(label(pts[0].p), add(a, (39.0, 4.5)), WHITE, false);
                }
                _ => {}
            }
        }
        "callout" | "pricenote" if p.len() >= 2 => {
            o.line(vec![p[0], p[1]]);
            if tool == "pricenote" {
                o.label(label(pts[0].p), add(p[1], (0.0, -10.0)), WHITE, true); // au-dessus de la poignée
            }
        }
        "image" if p.len() >= 2 => {
            let (a, b) = (p[0], p[1]);
            o.line(vec![a, (b.0, a.1), b, (a.0, b.1), a]);
            o.line(vec![a, b]);
            o.line(vec![(a.0, b.1), (b.0, a.1)]);
            o.label("Image", mid(a, b), GREY, true);
        }
        _ => return None,
    }
    // pas encore tous les points (aperçu) : on trace aussi le chemin des points placés
    if p.len() < need && p.len() >= 2 && !matches!(tool, "brush" | "path" | "polyline" | "highlighter") {
        o.dash(p.clone());
    }
    Some(o)
}

/// Point dans un polygone (règle pair / impair).
pub fn inside(poly: &[Px], x: f64, y: f64) -> bool {
    let mut on = false;
    let n = poly.len();
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + n - 1) % n]);
        if (a.1 > y) != (b.1 > y) && x < (b.0 - a.0) * (y - a.1) / (b.1 - a.1) + a.0 {
            on = !on;
        }
    }
    on
}

/// Découpe une ligne brisée en tirets (`on` pixels tracés, `off` vides).
pub fn dashes(pts: &[Px], on: f64, off: f64) -> Vec<Vec<Px>> {
    let mut out = vec![];
    let mut cur: Vec<Px> = vec![];
    let (mut drawing, mut left) = (true, on);
    for w in pts.windows(2) {
        let (mut a, b) = (w[0], w[1]);
        let mut seg = dist(a, b);
        if drawing && cur.is_empty() {
            cur.push(a);
        }
        while seg > left {
            let q = lerp(a, b, left / seg);
            if drawing {
                cur.push(q);
                out.push(std::mem::take(&mut cur));
            } else {
                cur = vec![q];
            }
            seg -= left;
            a = q;
            drawing = !drawing;
            left = if drawing { on } else { off };
        }
        left -= seg;
        if drawing {
            cur.push(b);
        }
    }
    if cur.len() >= 2 {
        out.push(cur);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::viewport::{PriceScale, Slots, Viewport};
    use argos_core::Bar;

    #[test]
    fn tirets_et_polygone() {
        let d = dashes(&[(0.0, 0.0), (100.0, 0.0)], 6.0, 4.0);
        assert_eq!(d.len(), 10, "10 tirets de 6 px sur 100 px");
        assert!((d[0][1].0 - 6.0).abs() < 1e-9);
        let sq = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)];
        assert!(inside(&sq, 5.0, 5.0) && !inside(&sq, 15.0, 5.0));
    }

    #[test]
    fn harmoniques_et_sonic() {
        let vp = Viewport {
            width: 1078,
            height: 528,
            right_offset: 5.0,
            bar_spacing: 10.0,
            ..Viewport::default()
        };
        let scale = PriceScale {
            min: 90.0,
            max: 210.0,
            top: 0.0,
            height: vp.plot_h(),
            log: false,
            invert: false,
        };
        let times: Vec<i64> = (0..50).map(|i| i * 60).collect();
        let bars: Vec<Bar> = times
            .iter()
            .map(|&t| Bar {
                time: t,
                open: 150.0,
                high: 151.0,
                low: 149.0,
                close: 150.0,
                volume: 1.0,
            })
            .collect();
        let slots = Slots::uniform(times.len());
        let g = Geo {
            mouse: None,
            vp: &vp,
            scale: &scale,
            slots: &slots,
            times: &times,
            bars: &bars,
            tf: 60,
        };
        let lbl = |p: f64| format!("{p:.1}");
        // Gartley parfait : AB = 0,618 XA, BC = 0,618 AB, AD = 0,786 XA
        let xabcd: Vec<Point> = [(0, 100.0), (600, 200.0), (1200, 138.2), (1800, 176.4), (2400, 121.4)]
            .iter()
            .map(|&(t, p)| Point { t, p })
            .collect();
        let name = |t: &str| {
            geom(t, &xabcd, &g, &lbl)
                .unwrap()
                .labels
                .into_iter()
                .find(|l| l.text.contains('✓') || l.text.contains('✗'))
                .unwrap()
                .text
        };
        assert_eq!(name("gartley"), "Gartley ✓");
        assert_eq!(name("bat"), "Bat ✗", "AB trop grand pour un Bat");
        // Sonic : 3 cercles numérotés passent par A, tangente en A
        let s = geom("sonic", &[Point { t: 600, p: 120.0 }, Point { t: 1200, p: 150.0 }], &g, &lbl).unwrap();
        assert_eq!(s.lines.len(), 1 + 3 + 6);
        let a = g.xy(&Point { t: 600, p: 120.0 });
        assert!(s.lines[1].0.iter().any(|q| (q.0 - a.0).hypot(q.1 - a.1) < 2.5), "le cercle 1 touche A");
        assert!(
            geom("dedekind", &[Point { t: 600, p: 120.0 }, Point { t: 1800, p: 120.0 }], &g, &lbl)
                .unwrap()
                .lines
                .len()
                > 10
        );
    }

    #[test]
    fn price_action_comme_luxalgo() {
        let vp = Viewport {
            width: 1078,
            height: 528,
            right_offset: 5.0,
            bar_spacing: 10.0,
            ..Viewport::default()
        };
        let scale = PriceScale {
            min: 90.0,
            max: 130.0,
            top: 0.0,
            height: vp.plot_h(),
            log: false,
            invert: false,
        };
        // bougie 0 haut 101, bougie 2 bas 104 : FVG haussier 101 → 104 ; comblé par la bougie 5 (bas 100)
        let hl = [
            (99.0, 101.0, 100.0, 99.5),
            (101.0, 106.0, 101.5, 105.5),
            (104.0, 108.0, 105.0, 107.0),
            (105.0, 109.0, 107.0, 108.0),
            (103.0, 108.0, 108.0, 104.0),
            (100.0, 104.0, 104.0, 100.5),
        ];
        let bars: Vec<Bar> = hl
            .iter()
            .enumerate()
            .map(|(i, &(low, high, open, close))| Bar {
                time: i as i64 * 60,
                open,
                high,
                low,
                close,
                volume: 1.0,
            })
            .collect();
        let times: Vec<i64> = bars.iter().map(|b| b.time).collect();
        let slots = Slots::uniform(times.len());
        let g = Geo {
            mouse: None,
            vp: &vp,
            scale: &scale,
            slots: &slots,
            times: &times,
            bars: &bars,
            tf: 60,
        };
        let f = price_action("fvg", &[Point { t: 60, p: 103.0 }], &g, &Default::default()).unwrap();
        let z = &f.fills[0].0;
        assert_eq!((z[0].1, z[2].1), (g.y_of(104.0), g.y_of(101.0)), "zone entre le haut d'avant et le bas d'après");
        assert_eq!(z[1].0, g.x_of(300), "s'arrête à la bougie qui la comble");
        assert_eq!(f.fills[0].1, Some(crate::palette::up()));
        // OB : bougie baissière 4 (103 → 108) → OB haussier, mitigé par la clôture à 101 de la bougie 5
        let o = price_action("ob", &[Point { t: 240, p: 105.0 }], &g, &Default::default()).unwrap();
        assert_eq!((o.fills[0].1, o.fills[0].0[1].0), (Some(crate::palette::up()), g.x_of(300)));
        // OB baissier sur la bougie haussière 3 : rien ne clôture au-dessus de 109, jusqu'au bord de la chart
        let o = price_action("ob", &[Point { t: 180, p: 107.0 }], &g, &Default::default()).unwrap();
        assert_eq!((o.fills[0].1, o.fills[0].0[1].0), (Some(crate::palette::down()), vp.plot_w()));
        // liquidité au haut de la bougie 1 (106) : balayée par la bougie 2 (108)
        let l = price_action("liq", &[Point { t: 60, p: 105.9 }], &g, &Default::default()).unwrap();
        assert_eq!(l.lines[0].0, vec![(g.x_of(60), g.y_of(106.0)), (g.x_of(120), g.y_of(106.0))]);
        assert!(l.labels.iter().any(|x| x.text == "✕"), "marquée balayée");
        // réglages : toujours prolongé, sans médiane, avec bordure, couleur haussière à soi
        let st = crate::drawings::Style {
            extend: 1,
            median: false,
            border: true,
            target_color: "#2962ff".into(),
            ..Default::default()
        };
        let f = price_action("fvg", &[Point { t: 60, p: 103.0 }], &g, &st).unwrap();
        assert_eq!(
            (f.fills[0].0[1].0, f.dashed.len(), f.lines.len(), f.fills[0].1),
            (vp.plot_w(), 0, 1, Some(Rgb::hex("#2962ff")))
        );
        // inversé à la clôture : la bougie 5 clôture sous 101 → iFVG rouge jusqu'au bord
        let st = crate::drawings::Style {
            invert: true,
            ..Default::default()
        };
        let f = price_action("fvg", &[Point { t: 60, p: 103.0 }], &g, &st).unwrap();
        assert_eq!(f.fills.len(), 2);
        assert_eq!((f.fills[1].1, f.fills[1].0[1].0), (Some(crate::palette::down()), vp.plot_w()));
        assert!(f.labels.iter().any(|l| l.text == "iFVG ▼"));
    }

    #[test]
    fn toutes_les_familles() {
        let vp = Viewport {
            width: 1078,
            height: 528,
            right_offset: 5.0,
            bar_spacing: 10.0,
            ..Viewport::default()
        };
        let scale = PriceScale {
            min: 100.0,
            max: 200.0,
            top: 0.0,
            height: vp.plot_h(),
            log: false,
            invert: false,
        };
        let times: Vec<i64> = (0..200).map(|i| 1_700_000_000 + i * 60).collect();
        let bars: Vec<Bar> = times
            .iter()
            .enumerate()
            .map(|(i, &t)| {
                let c = 150.0 + (i as f64 / 10.0).sin() * 10.0;
                Bar {
                    time: t,
                    open: c - 1.0,
                    high: c + 2.0,
                    low: c - 3.0,
                    close: c,
                    volume: 1.0 + i as f64,
                }
            })
            .collect();
        let slots = Slots::uniform(times.len());
        let g = Geo {
            mouse: None,
            vp: &vp,
            scale: &scale,
            slots: &slots,
            times: &times,
            bars: &bars,
            tf: 60,
        };
        let pts: Vec<Point> = [(150, 150.0), (165, 170.0), (175, 140.0), (182, 165.0), (188, 145.0), (192, 160.0), (196, 150.0)]
            .iter()
            .map(|&(i, p)| Point { t: times[i], p })
            .collect();
        let lbl = |p: f64| format!("{p:.1}");
        let mut drawn = 0;
        for t in crate::tools::TOOLS.iter().filter(|t| t.points > 0) {
            let n = (t.points as usize).clamp(1, 7);
            if let Some(o) = geom(t.id, &pts[..n], &g, &lbl) {
                assert!(!o.lines.is_empty() || !o.fills.is_empty() || !o.labels.is_empty(), "{} : forme vide", t.id);
                drawn += 1;
            }
            // aperçu (moins de points) : jamais de plantage
            let _ = geom(t.id, &pts[..1], &g, &lbl);
        }
        // 107 outils à placer − 22 dessinés à part (lignes simples, rectangle, Fibonacci, positions, règle, textes, flèches)
        assert_eq!(
            drawn, 82,
            "chaque outil de la phase C a sa forme (+ les 16 de LuxAlgo, sauf FVG/OB/liquidité : `price_action`)"
        );
        // régression : la droite suit les clôtures
        let r = geom("regression", &pts[..2], &g, &lbl).unwrap();
        assert_eq!(r.lines.len(), 1);
        assert_eq!(r.dashed.len(), 2, "± 2 écarts-types");
        // profil de volume : un POC
        let v = geom("fvp", &pts[..2], &g, &lbl).unwrap();
        assert!(v.labels.iter().any(|l| l.text == "POC"));
        // vagues : étiquettes (1) (2) …
        let e = geom("ewimp", &pts[..6], &g, &lbl).unwrap();
        assert!(e.labels.iter().any(|l| l.text == "(5)"));
    }
}
