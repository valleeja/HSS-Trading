//! 🔺 Argos Patterns auto (comme v1) : pivots ZigZag (écart de k × ATR 14), puis les figures sur les derniers pivots :
//! double et triple sommet / creux, épaule-tête-épaule (et inversé), triangles ascendant / descendant / symétrique, biseaux,
//! canaux, et le canal de Fibonacci des trois derniers pivots. Une figure est « confirmée » quand une clôture casse sa ligne (cou, bord) ; l'objectif mesuré des doubles et
//! des ETE est la hauteur de la figure reportée depuis le cou. Les pivots ne changent plus une fois posés : une figure
//! trouvée ne se repeint pas dans le passé. C'est une aide visuelle, pas un signal.

use crate::inputs::{check, color, num, Inputs, Param};
use argos_core::Bar;

pub const GROUPS: [&str; 3] = ["⚙️ Détection", "🔺 Figures", "🎨 Style"];
pub const PARAMS: [Param; 12] = [
    num("k", 0, "Sensibilité du ZigZag (× ATR 14)", 2.5, 0.5, 10.0, 0.25),
    num("keep", 0, "Figures affichées (les plus récentes)", 4.0, 1.0, 12.0, 1.0),
    check("double", 1, "Double sommet / creux", true),
    check("hs", 1, "Épaule-tête-épaule (et inversé)", true),
    check("tri", 1, "Triangles et biseaux", true),
    check("chan", 1, "Canaux", true),
    check("target", 1, "Objectif mesuré (doubles, ETE)", true),
    color("cBull", 2, "Haussier", 0x26a69a),
    color("cBear", 2, "Baissier", 0xf23645),
    color("cNeutral", 2, "Neutre", 0xf0b90b),
    // ajoutés à la fin : les réglages déjà enregistrés gardent leur place
    check("triple", 1, "Triple sommet / creux", true),
    check("fibch", 1, "Canal de Fibonacci auto (3 derniers pivots)", true),
];

/// Niveaux du canal de Fibonacci (0 = ligne de base, 1 = parallèle par le pivot opposé).
pub const FIB_LEVELS: [f64; 7] = [0.0, 0.236, 0.382, 0.5, 0.618, 0.786, 1.0];

/// Un pivot du ZigZag : bougie, prix, sommet (`true`) ou creux.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pivot {
    pub i: usize,
    pub p: f64,
    pub high: bool,
}

/// Une figure trouvée.
#[derive(Debug, Clone, PartialEq)]
pub struct Pattern {
    pub name: &'static str,
    /// Nom court pour l'étiquette (« Double sommet », « ETE », « Triangle sym. »).
    pub short: &'static str,
    /// +1 haussier, −1 baissier, 0 neutre.
    pub bias: i8,
    /// Les pivots de la figure, dans l'ordre.
    pub pts: Vec<Pivot>,
    /// Lignes en tirets (cou, bords) : de `a` à `b`, prolongées jusqu'à la bougie `end`.
    pub lines: Vec<(Pivot, Pivot)>,
    /// Cassure confirmée par une clôture (✓).
    pub conf: bool,
    /// Bougie de la cassure, sinon fin de la figure.
    pub end: usize,
    /// Objectif mesuré (prix).
    pub target: Option<f64>,
    /// 📐 Canal de Fibonacci : ligne de base (deux creux montants ou deux sommets descendants) et écart (prix) jusqu'à
    /// la parallèle qui passe par le pivot opposé ; les niveaux `FIB_LEVELS` sont entre les deux.
    pub fib: Option<(Pivot, Pivot, f64)>,
}

/// ATR (moyenne de Wilder du vrai range) sur `n` bougies, bougie par bougie.
fn atr(bars: &[Bar], n: f64) -> Vec<f64> {
    let mut out = Vec::with_capacity(bars.len());
    let mut a: Option<f64> = None;
    for (i, b) in bars.iter().enumerate() {
        let pc = if i > 0 { bars[i - 1].close } else { b.close };
        let tr = (b.high - b.low).max((b.high - pc).abs()).max((b.low - pc).abs());
        let v = a.map_or(tr, |a| (a * (n - 1.0) + tr) / n);
        a = Some(v);
        out.push(v);
    }
    out
}

/// Pivots du ZigZag : un sommet est posé quand le prix redescend de plus de `k × ATR` depuis le plus haut (et
/// inversement pour un creux).
pub fn pivots(bars: &[Bar], atr: &[f64], k: f64) -> Vec<Pivot> {
    let mut out = vec![];
    if bars.len() < 20 {
        return out;
    }
    let mut dir = 0i8;
    let mut ext = Pivot {
        i: 0,
        p: bars[0].high,
        high: true,
    };
    let mut lo = Pivot {
        i: 0,
        p: bars[0].low,
        high: false,
    };
    for i in 1..bars.len() {
        let (b, th) = (&bars[i], k * atr[i]);
        if dir >= 0 {
            if ext.high && b.high > ext.p {
                ext = Pivot { i, p: b.high, high: true };
            }
            if dir == 0 && b.low < lo.p {
                lo = Pivot { i, p: b.low, high: false };
            }
            if ext.high && ext.p - b.low > th {
                out.push(ext);
                dir = -1;
                ext = Pivot { i, p: b.low, high: false };
                continue;
            }
            if dir == 0 && b.high - lo.p > th {
                out.push(lo);
                dir = 1;
                ext = Pivot { i, p: b.high, high: true };
                continue;
            }
        }
        if dir < 0 {
            if b.low < ext.p {
                ext = Pivot { i, p: b.low, high: false };
            }
            if b.high - ext.p > th {
                out.push(ext);
                dir = 1;
                ext = Pivot { i, p: b.high, high: true };
            }
        }
    }
    out
}

/// Prix de la droite `a`→`b` à la bougie `i`.
fn at(a: Pivot, b: Pivot, i: usize) -> f64 {
    let d = b.i as f64 - a.i as f64;
    a.p + (b.p - a.p) * (i as f64 - a.i as f64) / if d == 0.0 { 1.0 } else { d }
}

/// Les figures (les plus récentes d'abord), selon les réglages `vals`.
pub fn compute(bars: &[Bar], vals: &[f64]) -> Vec<Pattern> {
    let p = Inputs { params: &PARAMS, vals };
    if bars.len() < 40 {
        return vec![];
    }
    let a = atr(bars, 14.0);
    let pv = pivots(bars, &a, p.get("k").max(0.1));
    let last = bars.len() - 1;
    let tol = |i: usize| a[i] * 0.9;
    let brk = |from: usize, f: &dyn Fn(usize) -> bool| (from + 1..=last).find(|&i| f(i));
    let mut out: Vec<Pattern> = vec![];
    let mut used = std::collections::HashSet::new();
    let mut push = |o: Pattern, out: &mut Vec<Pattern>| {
        if o.pts.iter().any(|q| used.contains(&q.i)) {
            return false;
        }
        used.extend(o.pts.iter().map(|q| q.i));
        out.push(o);
        true
    };
    let target_on = p.on("target");
    for j in (2..pv.len()).rev() {
        if out.len() >= 12 {
            break;
        }
        if j >= 4 {
            let (q0, q1, q2, q3, q4) = (pv[j - 4], pv[j - 3], pv[j - 2], pv[j - 1], pv[j]);
            // triple sommet / creux : trois pivots à la même hauteur, deux creux (sommets) bien marqués entre eux
            if p.on("triple") {
                let t = tol(q4.i);
                let ext = [q0.p, q2.p, q4.p];
                let (hi, lo) = (ext.iter().cloned().fold(f64::MIN, f64::max), ext.iter().cloned().fold(f64::MAX, f64::min));
                let top = q0.high && hi - lo < t * 1.2 && lo - q1.p.max(q3.p) > t * 2.2;
                let bot = !q0.high && hi - lo < t * 1.2 && q1.p.min(q3.p) - hi > t * 2.2;
                if (top || bot) && q4.i - q0.i >= 10 {
                    let x = brk(q4.i, &|i| if top { bars[i].close < at(q1, q3, i) } else { bars[i].close > at(q1, q3, i) });
                    let neck = at(q1, q3, q4.i);
                    let h = if top { hi - neck } else { neck - lo };
                    let o = Pattern {
                        name: if top { "Triple sommet" } else { "Triple creux" },
                        short: if top { "Triple sommet" } else { "Triple creux" },
                        bias: if top { -1 } else { 1 },
                        pts: vec![q0, q1, q2, q3, q4],
                        lines: vec![(q1, q3)],
                        conf: x.is_some(),
                        end: x.unwrap_or(q4.i),
                        target: target_on.then_some(if top { neck - h } else { neck + h }),
                        fib: None,
                    };
                    if push(o, &mut out) {
                        continue;
                    }
                }
            }
            // épaule-tête-épaule : épaules à peu près égales, tête plus haute (ou plus basse pour l'inversé)
            if p.on("hs") {
                let t = tol(q2.i);
                let hs = q0.high && q2.p > q0.p + t * 0.5 && q2.p > q4.p + t * 0.5 && (q0.p - q4.p).abs() < t * 1.8;
                let inv = !q0.high && q2.p < q0.p - t * 0.5 && q2.p < q4.p - t * 0.5 && (q0.p - q4.p).abs() < t * 1.8;
                if hs || inv {
                    let x = brk(q4.i, &|i| if hs { bars[i].close < at(q1, q3, i) } else { bars[i].close > at(q1, q3, i) });
                    let neck = at(q1, q3, q2.i);
                    let o = Pattern {
                        name: if hs { "Épaule-tête-épaule" } else { "ETE inversé" },
                        short: if hs { "ETE" } else { "ETE inversé" },
                        bias: if hs { -1 } else { 1 },
                        pts: vec![q0, q1, q2, q3, q4],
                        lines: vec![(q1, q3)],
                        conf: x.is_some(),
                        end: x.unwrap_or(q4.i),
                        target: target_on.then_some(neck - (q2.p - neck)),
                        fib: None,
                    };
                    if push(o, &mut out) {
                        continue;
                    }
                }
            }
            // triangles, biseaux, canaux : 5 pivots, 3 d'un côté et 2 de l'autre, alignés sur deux droites
            if p.on("tri") || p.on("chan") {
                let s = [q0, q1, q2, q3, q4];
                let h: Vec<Pivot> = s.iter().copied().filter(|q| q.high).collect();
                let l: Vec<Pivot> = s.iter().copied().filter(|q| !q.high).collect();
                let (h0, h1, l0, l1) = (h[0], h[h.len() - 1], l[0], l[l.len() - 1]);
                let (i0, i1) = (q0.i, q4.i);
                let t = tol(i1);
                let ok = h.iter().all(|q| (at(h0, h1, q.i) - q.p).abs() < t) && l.iter().all(|q| (at(l0, l1, q.i) - q.p).abs() < t);
                let (w0, w1) = (at(h0, h1, i0) - at(l0, l1, i0), at(h0, h1, i1) - at(l0, l1, i1));
                let (su, sd) = (at(h0, h1, i1) - at(h0, h1, i0), at(l0, l1, i1) - at(l0, l1, i0));
                let flat = |v: f64| v.abs() < t * 1.2;
                if ok && w0 > 0.0 && w1 > 0.0 && i1 - i0 >= 10 {
                    let shape = if w1 < w0 * 0.72 && p.on("tri") {
                        if flat(su) && sd > 0.0 {
                            Some(("Triangle ascendant", "Triangle asc.", 1))
                        } else if flat(sd) && su < 0.0 {
                            Some(("Triangle descendant", "Triangle desc.", -1))
                        } else if su < 0.0 && sd > 0.0 {
                            Some(("Triangle symétrique", "Triangle sym.", 0))
                        } else if su > 0.0 && sd > 0.0 {
                            Some(("Biseau montant", "Biseau montant", -1))
                        } else if su < 0.0 && sd < 0.0 {
                            Some(("Biseau descendant", "Biseau desc.", 1))
                        } else {
                            None
                        }
                    } else if w1 > w0 * 0.8 && w1 < w0 * 1.25 && !flat(su) && su.signum() == sd.signum() && p.on("chan") {
                        Some(if su > 0.0 {
                            ("Canal haussier", "Canal haussier", 1)
                        } else {
                            ("Canal baissier", "Canal baissier", -1)
                        })
                    } else {
                        None
                    };
                    if let Some((name, short, bias)) = shape {
                        let x = brk(i1, &|i| bars[i].close > at(h0, h1, i) + t * 0.2 || bars[i].close < at(l0, l1, i) - t * 0.2);
                        let up = x.map_or(0, |x| if bars[x].close > at(h0, h1, x) { 1 } else { -1 });
                        let o = Pattern {
                            name,
                            short,
                            bias: if up != 0 { up } else { bias },
                            pts: s.to_vec(),
                            lines: vec![(h0, h1), (l0, l1)],
                            conf: x.is_some(),
                            end: x.unwrap_or_else(|| last.min(i1 + (i1 - i0) / 2)),
                            target: None,
                            fib: None,
                        };
                        if push(o, &mut out) {
                            continue;
                        }
                    }
                }
            }
        }
        // double sommet / double creux : deux pivots à la même hauteur, un creux (ou sommet) bien marqué entre les deux
        if p.on("double") {
            let (q0, q1, q2) = (pv[j - 2], pv[j - 1], pv[j]);
            let t = tol(q2.i);
            let top = q0.high && (q0.p - q2.p).abs() < t && q0.p.min(q2.p) - q1.p > t * 2.2 && q2.i - q0.i >= 6;
            let bot = !q0.high && (q0.p - q2.p).abs() < t && q1.p - q0.p.max(q2.p) > t * 2.2 && q2.i - q0.i >= 6;
            if top || bot {
                let neck = q1.p;
                let x = brk(q2.i, &|i| if top { bars[i].close < neck } else { bars[i].close > neck });
                let h = if top { q0.p.max(q2.p) - neck } else { neck - q0.p.min(q2.p) };
                let o = Pattern {
                    name: if top { "Double sommet" } else { "Double creux" },
                    short: if top { "Double sommet" } else { "Double creux" },
                    bias: if top { -1 } else { 1 },
                    pts: vec![q0, q1, q2],
                    lines: vec![(Pivot { i: q0.i, p: neck, high: false }, Pivot { i: q2.i, p: neck, high: false })],
                    conf: x.is_some(),
                    end: x.unwrap_or(q2.i),
                    target: target_on.then_some(if top { neck - h } else { neck + h }),
                    fib: None,
                };
                push(o, &mut out);
            }
        }
    }
    out.sort_by_key(|o| std::cmp::Reverse(o.end));
    out.truncate(p.get("keep").max(1.0) as usize);
    if p.on("fibch") {
        if let Some(f) = fib_channel(&pv, last) {
            out.insert(0, f);
        }
    }
    out
}

/// 📐 Canal de Fibonacci des trois derniers pivots : les deux pivots du même côté (creux ou sommets) donnent la ligne
/// de base, montante (canal haussier) ou descendante (baissier) ; le pivot du milieu donne la parallèle. Base plate :
/// rien.
fn fib_channel(pv: &[Pivot], last: usize) -> Option<Pattern> {
    let n = pv.len();
    if n < 3 {
        return None;
    }
    let (a, c, b) = (pv[n - 3], pv[n - 2], pv[n - 1]);
    let up = b.p > a.p;
    if a.high != b.high || b.i <= a.i || (b.p - a.p).abs() < 1e-12 {
        return None;
    }
    // le pivot du milieu est de l'autre côté de la ligne de base (au-dessus des creux, en dessous des sommets)
    let d = c.p - at(a, b, c.i);
    if (a.high && d >= 0.0) || (!a.high && d <= 0.0) {
        return None;
    }
    Some(Pattern {
        name: "Canal de Fibonacci",
        short: "Canal Fibo",
        bias: if up { 1 } else { -1 },
        pts: vec![a, c, b],
        lines: vec![],
        conf: false,
        end: last,
        target: None,
        fib: Some((a, b, d)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Bougies qui passent par ces points (prix, nombre de bougies pour y aller), avec une petite mèche.
    fn path(points: &[(f64, usize)]) -> Vec<Bar> {
        let mut bars = vec![];
        let mut prev = points[0].0;
        for &(target, n) in &points[1..] {
            for k in 1..=n {
                let c = prev + (target - prev) * k as f64 / n as f64;
                let o = bars.last().map_or(c, |b: &Bar| b.close);
                bars.push(Bar {
                    time: bars.len() as i64 * 3600,
                    open: o,
                    high: o.max(c) + 0.2,
                    low: o.min(c) - 0.2,
                    close: c,
                    volume: 1.0,
                });
            }
            prev = target;
        }
        bars
    }

    fn names(bars: &[Bar]) -> Vec<&'static str> {
        compute(bars, &crate::inputs::defaults(&PARAMS)).iter().map(|p| p.name).collect()
    }

    #[test]
    fn double_sommet_net_et_confirme() {
        let b = path(&[(100.0, 1), (100.0, 10), (120.0, 10), (105.0, 8), (120.0, 8), (98.0, 12)]);
        let f = compute(&b, &crate::inputs::defaults(&PARAMS));
        let d = f.iter().find(|p| p.name == "Double sommet").expect("double sommet trouvé");
        assert!(d.conf, "le cou (105) est cassé par une clôture");
        assert_eq!(d.bias, -1);
        let t = d.target.unwrap();
        assert!((t - 90.0).abs() < 1.5, "objectif mesuré ≈ 105 − 15 : {t}");
    }

    #[test]
    fn triple_sommet_et_triple_creux() {
        let b = path(&[(100.0, 1), (100.0, 10), (120.0, 10), (105.0, 8), (120.0, 8), (105.0, 8), (120.0, 8), (95.0, 12)]);
        let f = compute(&b, &crate::inputs::defaults(&PARAMS));
        let t = f.iter().find(|p| p.name == "Triple sommet").unwrap_or_else(|| panic!("{:?}", names(&b)));
        assert!(t.conf && t.bias == -1);
        assert!((t.target.unwrap() - 90.0).abs() < 1.5, "objectif ≈ 105 − 15");
        let b = path(&[
            (120.0, 1),
            (120.0, 10),
            (100.0, 10),
            (115.0, 8),
            (100.0, 8),
            (115.0, 8),
            (100.0, 8),
            (125.0, 12),
        ]);
        assert!(names(&b).contains(&"Triple creux"), "{:?}", names(&b));
    }

    #[test]
    fn canal_de_fibonacci_sur_tendance_haussiere() {
        let b = path(&[(100.0, 1), (100.0, 6), (90.0, 8), (115.0, 10), (100.0, 8), (125.0, 10), (112.0, 8)]);
        let f = compute(&b, &crate::inputs::defaults(&PARAMS));
        let c = f.iter().find(|p| p.name == "Canal de Fibonacci").unwrap_or_else(|| panic!("{:?}", names(&b)));
        let (a, bb, d) = c.fib.unwrap();
        assert!(bb.p > a.p && c.bias == 1 && (d > 0.0) != a.high, "{c:?}");
        // une montée sans retour : pas de canal
        assert!(compute(&path(&[(100.0, 1), (200.0, 120)]), &crate::inputs::defaults(&PARAMS)).is_empty());
    }

    #[test]
    fn epaule_tete_epaule_net() {
        let b = path(&[(100.0, 1), (100.0, 8), (115.0, 8), (105.0, 6), (125.0, 8), (105.0, 8), (115.0, 6), (95.0, 10)]);
        assert!(names(&b).contains(&"Épaule-tête-épaule"), "{:?}", names(&b));
    }

    #[test]
    fn triangle_symetrique() {
        let b = path(&[
            (100.0, 1),
            (100.0, 6),
            (130.0, 8),
            (104.0, 8),
            (126.0, 7),
            (108.0, 7),
            (122.0, 6),
            (112.0, 5),
            (114.0, 3),
        ]);
        assert!(names(&b).contains(&"Triangle symétrique"), "{:?}", names(&b));
    }

    #[test]
    fn serie_sans_figure_ne_detecte_rien() {
        // une montée régulière sans retour
        let b = path(&[(100.0, 1), (200.0, 120)]);
        assert!(names(&b).is_empty(), "{:?}", names(&b));
        assert!(compute(&b[..30], &crate::inputs::defaults(&PARAMS)).is_empty(), "trop peu de bougies");
    }

    #[test]
    fn pas_de_repeinte_dans_le_passe() {
        let b = path(&[(100.0, 1), (100.0, 10), (120.0, 10), (105.0, 8), (120.0, 8), (98.0, 12)]);
        let v = crate::inputs::defaults(&PARAMS);
        let before = compute(&b, &v);
        let mut more = b.clone();
        more.extend(path(&[(98.0, 1), (96.0, 6)]).into_iter().enumerate().map(|(k, mut x)| {
            x.time = (b.len() + k) as i64 * 3600;
            x
        }));
        let after = compute(&more, &v);
        let d0 = before.iter().find(|p| p.name == "Double sommet").unwrap();
        let d1 = after.iter().find(|p| p.name == "Double sommet").unwrap();
        assert_eq!(d0.pts, d1.pts, "mêmes pivots après de nouvelles bougies");
    }
}
