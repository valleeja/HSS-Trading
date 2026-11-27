//! 📈📉 Position longue / courte, **comme TradingView** :
//! - points : entrée (temps de gauche), stop et cible (temps de droite = largeur) ;
//! - calcul : risque (montant, ou % du compte), quantité = risque ÷ distance du stop ÷ taille du lot, montants de la
//!   cible et du stop, ratio risque / récompense, P&L ouvert au dernier prix ;
//! - dessin : zones cible / stop, étiquettes (Cible…, Ouverture P&L…, Ratio…, Stop…) selon les **Stats** cochées,
//!   prix sur l'échelle ;
//! - poignées : entrée (gauche), cible (haut-gauche), stop (bas-gauche), largeur (droite).

use crate::drawings::{Drawing, Geo, Point};
use crate::shapes::{round_fill, Rect};
use crate::text::{Align, Rgb, Text};
use tiny_skia::Pixmap;

/// Les stats qu'on peut afficher (cases de la liste « Stats »), dans l'ordre de TradingView.
pub const STATS: [&str; 13] = [
    "Décalage du prix TP",
    "Décalage du TP en pourcents",
    "Décalage du tick TP",
    "Montant du TP",
    "TP PL",
    "PL ouvert/fermé",
    "Qté",
    "Ratio Risque/Récompense",
    "Décalage du prix SL",
    "Décalage du SL en pourcents",
    "Décalage du tick SL",
    "Montant SL",
    "SL PL",
];

/// Stats cochées au départ (toutes sauf « TP PL » et « SL PL », comme TradingView).
pub const DEFAULT_STATS: u16 = 0b0_1111_1110_1111;

pub const DEFAULT_TARGET: &str = "#089981";
pub const DEFAULT_STOP: &str = "#f23645";
const LINE: Rgb = Rgb(0x78, 0x7b, 0x86);
const WHITE: Rgb = Rgb(0xff, 0xff, 0xff);

pub fn is_position(tool: &str) -> bool {
    matches!(tool, "long" | "short")
}

/// Les chiffres d'une position.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Calc {
    pub long: bool,
    pub entry: f64,
    pub stop: f64,
    pub target: f64,
    /// Risque en devise (montant du stop).
    pub risk: f64,
    pub qty: f64,
    /// Montant gagné à la cible.
    pub reward: f64,
    pub rr: f64,
    /// Taille d'un tick (pour compter les ticks).
    pub tick: f64,
    /// P&L au dernier prix (`None` sans bougies).
    pub open_pl: Option<f64>,
}

/// Cible par défaut : à RR fois la distance du stop, du bon côté.
fn default_target(d: &Drawing) -> f64 {
    let e = d.pts[0].p;
    e + (e - stop_of(d)) * d.style.rr
}

/// Prix du stop, toujours du bon côté de l'entrée (aussi pendant le placement, avant `normalize`).
fn stop_of(d: &Drawing) -> f64 {
    let e = d.pts[0].p;
    e - dir(d) * d.pts.get(1).map_or(0.0, |q| (e - q.p).abs())
}

/// Calcule la position ; `last` = dernier prix, `tick` = plus petit écart de prix.
pub fn calc(d: &Drawing, last: Option<f64>, tick: f64) -> Calc {
    let long = d.tool == "long";
    let entry = d.pts[0].p;
    let stop = stop_of(d);
    let target = d.pts.get(2).map_or_else(|| default_target(d), |q| q.p);
    let st = &d.style;
    let risk = if st.risk_pct { st.account * st.risk / 100.0 } else { st.risk };
    let dist = (entry - stop).abs();
    let qty = if dist > 0.0 { risk / dist / st.lot.max(1e-12) } else { 0.0 };
    let reward = qty * st.lot * (target - entry).abs();
    let rr = if dist > 0.0 { (target - entry).abs() / dist } else { 0.0 };
    let dir = if long { 1.0 } else { -1.0 };
    let open_pl = last.map(|p| (p - entry) * dir * qty * st.lot);
    Calc {
        long,
        entry,
        stop,
        target,
        risk,
        qty,
        reward,
        rr,
        tick: tick.max(1e-12),
        open_pl,
    }
}

/// Taille d'un tick : les décimales des prix, mais au plus 5 chiffres significatifs (80 000 → 1, 3 000 → 0,1,
/// 150 → 0,01) — les prix lus en nombres à virgule ont parfois de fausses décimales (0,000001 → des millions de ticks).
pub fn tick_of(bars: &[argos_core::Bar]) -> f64 {
    let dec = crate::viewport::data_decimals(bars) as i32;
    let p = bars.last().map_or(1.0, |b| b.close.abs()).max(1e-12);
    let sig = (5 - (p.log10().floor() as i32 + 1)).max(0);
    10f64.powi(-dec.min(sig))
}

fn n2(v: f64) -> String {
    crate::viewport::format_price(v, 2)
}

/// Les 3 étiquettes (haut, milieu, bas) selon les stats cochées ; `price` écrit un écart de prix.
pub fn labels(c: &Calc, stats: u16, compact: bool, price: &dyn Fn(f64) -> String) -> (String, Vec<String>, String) {
    let on = |k: usize| stats & (1 << k) != 0;
    let side = |name: &str, lvl: f64, amount: f64, signed: f64, base: usize| {
        let off = (lvl - c.entry).abs();
        let mut head = vec![];
        if on(base) {
            head.push(price(off));
        }
        if on(base + 1) {
            head.push(format!("({:.3} %)", off / c.entry * 100.0));
        }
        if on(base + 2) {
            head.push(format!("{}", (off / c.tick).round()));
        }
        let mut parts = vec![];
        if !head.is_empty() {
            parts.push(head.join(" "));
        }
        if on(base + 3) {
            parts.push(if compact { n2(amount) } else { format!("Montant: {}", n2(amount)) });
        }
        if on(base + 4) {
            parts.push(format!("PL: {}", n2(signed)));
        }
        if parts.is_empty() {
            return String::new();
        }
        if compact {
            parts.join(" · ")
        } else {
            format!("{name}: {}", parts.join(", "))
        }
    };
    let top = side("Cible", c.target, c.reward, c.reward, 0);
    let bottom = side("Stop", c.stop, c.risk, -c.risk, 8);
    let mut mid = vec![];
    let mut l1 = vec![];
    if on(5) {
        let pl = c.open_pl.map_or("—".to_string(), n2);
        l1.push(if compact { pl } else { format!("Ouverture P&L: {pl}") });
    }
    if on(6) {
        l1.push(if compact { format!("{:.3}", c.qty) } else { format!("Qté: {:.3}", c.qty) });
    }
    if !l1.is_empty() {
        mid.push(l1.join(", "));
    }
    if on(7) {
        mid.push(if compact {
            format!("RR {:.2}", c.rr)
        } else {
            format!("Ratio Risque/Récompense: {:.2}", c.rr)
        });
    }
    (top, mid, bottom)
}

/// Prix ↔ pixels : x de gauche, x de droite (au moins 60 px de large), y de l'entrée, du stop, de la cible.
fn geom(d: &Drawing, g: &Geo) -> (f64, f64, f64, f64, f64) {
    let x0 = g.x_of(d.pts[0].t);
    let x1 = d.pts.get(1).map_or(x0 + 120.0, |q| g.x_of(q.t)).max(x0 + 60.0);
    let ty = g.y_of(d.pts.get(2).map_or_else(|| default_target(d), |q| q.p));
    (x0, x1, g.y_of(d.pts[0].p), g.y_of(stop_of(d)), ty)
}

/// Poignées : entrée (gauche), cible (haut-gauche), stop (bas-gauche), largeur (droite, à l'entrée).
pub fn handles(d: &Drawing, g: &Geo) -> Vec<(f64, f64)> {
    let (x0, x1, ye, ys, yt) = geom(d, g);
    vec![(x0, ye), (x0, yt), (x0, ys), (x1, ye)]
}

/// Déplace une poignée.
pub fn move_handle(d: &mut Drawing, h: usize, at: Point) {
    normalize(d);
    match h {
        0 => d.pts[0] = at,
        // comme TradingView : la cible et le stop ne passent pas de l'autre côté de l'entrée
        1 => d.pts[2].p = d.pts[0].p + dir(d) * ((at.p - d.pts[0].p) * dir(d)).max(0.0),
        2 => d.pts[1].p = d.pts[0].p - dir(d) * ((d.pts[0].p - at.p) * dir(d)).max(0.0),
        _ => {
            let t = at.t.max(d.pts[0].t + 1);
            (d.pts[1].t, d.pts[2].t) = (t, t);
        }
    }
}

/// +1 pour une longue (cible au-dessus), −1 pour une courte.
fn dir(d: &Drawing) -> f64 {
    if d.tool == "long" {
        1.0
    } else {
        -1.0
    }
}

/// Trois points (entrée, stop, cible) : la cible se calcule au RR si elle manque.
pub fn normalize(d: &mut Drawing) {
    if d.pts.len() == 1 {
        let p = d.pts[0];
        d.pts.push(Point {
            t: p.t + 1,
            p: p.p * if d.tool == "long" { 0.99 } else { 1.01 },
        });
    }
    if d.pts.len() == 2 {
        // stop cliqué du mauvais côté : il passe de l'autre côté de l'entrée (une longue reste une longue)
        d.pts[1].p = stop_of(d);
        let tp = default_target(d);
        d.pts.push(Point { t: d.pts[1].t, p: tp });
    }
}

/// Inverse longue ↔ courte : stop et cible changent de côté autour de l'entrée.
pub fn invert(d: &mut Drawing) {
    normalize(d);
    d.tool = if d.tool == "long" { "short".into() } else { "long".into() };
    let e = d.pts[0].p;
    for q in &mut d.pts[1..] {
        q.p = 2.0 * e - q.p;
    }
}

/// La souris touche-t-elle la position (ses deux zones) ?
pub fn hit(d: &Drawing, x: f64, y: f64, g: &Geo) -> bool {
    let (x0, x1, ye, ys, yt) = geom(d, g);
    x >= x0 && x <= x1 && y >= ye.min(ys).min(yt) && y <= ye.max(ys).max(yt)
}

/// Étiquette centrée en `(cx, y)`, au-dessus ou en dessous de `y`.
fn tag(pm: &mut Pixmap, text: &Text, lines: &[String], (cx, y): (f64, f64), above: bool, bg: Rgb, size: f32) {
    let lines: Vec<&String> = lines.iter().filter(|l| !l.is_empty()).collect();
    if lines.is_empty() {
        return;
    }
    let lh = size as f64 + 5.0;
    let w = lines.iter().map(|l| text.width(l, size) as f64).fold(0.0, f64::max) + 14.0;
    let h = lh * lines.len() as f64 + 6.0;
    let top = if above { y - h - 4.0 } else { y + 4.0 };
    round_fill(pm, Rect::new(cx - w / 2.0, top, w, h), 4.0, bg, 0.95);
    for (i, l) in lines.iter().enumerate() {
        text.draw(pm, l, (cx as f32, (top + 3.0 + lh * (i as f64 + 1.0) - 4.0) as f32), size, WHITE, Align::Center);
    }
}

/// Dessine la position ; `label` écrit un prix. Les étiquettes : quand la souris est dessus (`g.mouse`).
pub fn draw(d: &Drawing, pm: &mut Pixmap, text: &Text, g: &Geo, label: &dyn Fn(f64) -> String) {
    let st = &d.style;
    let (x0, x1, ye, ys, yt) = geom(d, g);
    let col = |s: &str, def: &str| Rgb::hex(if s.trim_start_matches('#').len() == 6 { s } else { def });
    let (cs, ct) = (col(&st.stop_color, DEFAULT_STOP), col(&st.target_color, DEFAULT_TARGET));
    let line = if st.color.trim_start_matches('#').len() == 6 {
        Rgb::hex(&st.color)
    } else {
        LINE
    };
    crate::fill(pm, x0, ye.min(yt), x1 - x0, (yt - ye).abs(), ct, 0.22);
    crate::fill(pm, x0, ye.min(ys), x1 - x0, (ys - ye).abs(), cs, 0.22);
    crate::fill(pm, x0, ye.round(), x1 - x0, 1.0, line, 1.0);
    let tick = tick_of(g.bars);
    let c = calc(d, g.bars.last().map(|b| b.close), tick);
    // étiquettes : souris sur la position (ou « Toujours afficher »)
    let over = g.mouse.is_some_and(|(mx, my)| hit(d, mx, my, g));
    if st.stats_always || over {
        let (top, mid, bottom) = labels(&c, st.stats, st.compact, label);
        let size = st.tsize.clamp(8.0, 40.0) * 0.86;
        let cx = (x0 + x1) / 2.0;
        let tcol = if st.tcolor.trim_start_matches('#').len() == 6 {
            Rgb::hex(&st.tcolor)
        } else {
            WHITE
        };
        let _ = tcol;
        // la cible au-dessus de sa zone, le stop en dessous (inversé pour une courte)
        tag(pm, text, &[top], (cx, yt), yt < ye, ct, size);
        tag(pm, text, &[bottom], (cx, ys), ys < ye, cs, size);
        let pl_bg = match c.open_pl {
            Some(p) if p >= 0.0 => ct,
            Some(_) => cs,
            None => Rgb(0x2a, 0x2e, 0x39),
        };
        tag(
            pm,
            text,
            &mid,
            (cx, ye - 4.0 - (size as f64 + 5.0) * mid.len() as f64 / 2.0),
            false,
            pl_bg,
            size,
        );
    }
}

/// Prix de la position sur l'échelle (entrée, cible, stop), si l'option est cochée.
pub fn axis_labels(d: &Drawing, g: &Geo) -> Vec<(f64, Rgb)> {
    let st = &d.style;
    if !st.price_labels {
        return vec![];
    }
    let col = |s: &str, def: &str| Rgb::hex(if s.trim_start_matches('#').len() == 6 { s } else { def });
    let c = calc(d, g.bars.last().map(|b| b.close), tick_of(g.bars));
    vec![
        (c.entry, LINE),
        (c.target, col(&st.target_color, DEFAULT_TARGET)),
        (c.stop, col(&st.stop_color, DEFAULT_STOP)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn long() -> Drawing {
        let mut d = Drawing::new("long", vec![Point { t: 0, p: 100.0 }, Point { t: 3600, p: 95.0 }]);
        d.style.account = 10_000.0;
        d.style.risk = 1.0;
        d.style.risk_pct = true;
        normalize(&mut d);
        d
    }

    #[test]
    fn taille_du_tick() {
        let bar = |c: f64| argos_core::Bar {
            time: 0,
            open: c,
            high: c,
            low: c,
            close: c,
            volume: 0.0,
        };
        assert_eq!(tick_of(&[bar(80623.400001)]), 1.0, "BTC : fausses décimales ignorées");
        assert!((tick_of(&[bar(3012.25)]) - 0.1).abs() < 1e-12);
        assert!((tick_of(&[bar(0.21345)]) - 0.00001).abs() < 1e-12);
    }

    #[test]
    fn calcul() {
        let d = long();
        assert_eq!(d.pts[2].p, 110.0, "cible au RR 2");
        let c = calc(&d, Some(102.0), 0.01);
        assert_eq!(c.risk, 100.0, "1 % de 10 000");
        assert!((c.qty - 20.0).abs() < 1e-9, "100 $ ÷ 5 $ de stop");
        assert!((c.reward - 200.0).abs() < 1e-9);
        assert!((c.rr - 2.0).abs() < 1e-9);
        assert!((c.open_pl.unwrap() - 40.0).abs() < 1e-9, "+2 $ × 20");
        let (top, mid, bottom) = labels(&c, DEFAULT_STATS, false, &|p| format!("{p:.2}"));
        assert_eq!(top, "Cible: 10.00 (10.000 %) 1000, Montant: 200,00");
        assert_eq!(
            mid,
            vec!["Ouverture P&L: 40,00, Qté: 20.000".to_string(), "Ratio Risque/Récompense: 2.00".into()]
        );
        assert!(bottom.starts_with("Stop: 5.00 (5.000 %) 500, Montant: 100,00"));
        // rien de coché : pas d'étiquettes
        let (t, m, b) = labels(&c, 0, false, &|p| format!("{p}"));
        assert!(t.is_empty() && m.is_empty() && b.is_empty());
    }

    #[test]
    fn stop_toujours_du_bon_cote() {
        // longue : stop cliqué au-dessus de l'entrée → il passe en dessous, la cible au-dessus
        let mut d = Drawing::new("long", vec![Point { t: 0, p: 100.0 }, Point { t: 10, p: 105.0 }]);
        normalize(&mut d);
        assert_eq!(d.pts[1].p, 95.0);
        assert!(d.pts[2].p > 100.0);
        // tirer le stop au-dessus de l'entrée : il s'arrête à l'entrée ; la cible, pareil en dessous
        move_handle(&mut d, 2, Point { t: 0, p: 120.0 });
        assert_eq!(d.pts[1].p, 100.0);
        move_handle(&mut d, 1, Point { t: 0, p: 80.0 });
        assert_eq!(d.pts[2].p, 100.0);
        // courte : stop cliqué en dessous → au-dessus
        let mut s = Drawing::new("short", vec![Point { t: 0, p: 100.0 }, Point { t: 10, p: 96.0 }]);
        normalize(&mut s);
        assert_eq!(s.pts[1].p, 104.0);
        assert!(s.pts[2].p < 100.0);
    }

    #[test]
    fn poignees_et_inverse() {
        let mut d = long();
        move_handle(&mut d, 1, Point { t: 0, p: 120.0 });
        assert_eq!(d.pts[2].p, 120.0, "cible déplacée");
        move_handle(&mut d, 3, Point { t: 7200, p: 0.0 });
        assert_eq!((d.pts[1].t, d.pts[2].t), (7200, 7200), "largeur");
        invert(&mut d);
        assert_eq!(d.tool, "short");
        assert_eq!((d.pts[1].p, d.pts[2].p), (105.0, 80.0), "stop et cible de l'autre côté");
    }
}
