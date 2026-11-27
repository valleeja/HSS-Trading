//! Argos OB · BB · FVG · iFVG — structure BOS / CHoCH, order blocks, breaker blocks (OB cassé qui change de camp),
//! fair value gaps, iFVG (FVG traversé = polarité inversée, invalidé après N touches), liquidité EQH / EQL + sweeps, option UT supérieure.
//! Même logique que l'indicateur du site. Le choix des zones affichées et les couleurs restent dans la couche chart.

use argos_core::{aggregate, Bar};

#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    /// Unité de temps des zones en secondes (`None` = celle de la chart).
    pub tf: Option<i64>,
    /// Longueur des pivots (swing).
    pub len: usize,
    /// Zone de l'OB = corps de la bougie (sinon mèche complète).
    pub ob_body: bool,
    /// OB invalidé par la mèche (sinon par la clôture).
    pub ob_wick: bool,
    pub breakers: bool,
    pub ifvg: bool,
    /// Taille min d'un FVG (× ATR 14).
    pub fvg_min: f64,
    /// FVG invalidé après N touches (0 = jamais).
    pub max_touch: u32,
    pub eq: bool,
    /// Tolérance égalité EQH / EQL (× ATR).
    pub eq_thr: f64,
}

/// Groupes et inputs de v1 (générés depuis app.js : `v1_inputs`).
pub use crate::v1_inputs::{SMC_GROUPS as GROUPS, SMC_PARAMS as PARAMS};

impl Settings {
    /// Réglages du calcul lus dans les inputs de v1 (valeurs dans l'ordre de `PARAMS`).
    pub fn from_inputs(vals: &[f64]) -> Self {
        let p = crate::inputs::Inputs { params: &PARAMS, vals };
        Self {
            // « Graphique », 15m, 1h, 4H, 1D
            tf: [None, Some(900), Some(3_600), Some(14_400), Some(86_400)][p.pick("tf").min(4)],
            len: p.len("len"),
            ob_body: p.on("ob_body"),
            ob_wick: p.on("ob_wick"),
            breakers: p.on("show_bb"),
            ifvg: p.on("show_ifvg"),
            fvg_min: p.get("fvg_min"),
            max_touch: p.get("max_touch").round().max(0.0) as u32,
            eq: p.on("show_eq"),
            eq_thr: p.get("eq_thr"),
        }
    }
}

/// Valeurs par défaut = celles de v1 (une seule source : `PARAMS`).
impl Default for Settings {
    fn default() -> Self {
        Self::from_inputs(&crate::inputs::defaults(&PARAMS))
    }
}

/// Order block (`breaker` = OB cassé qui a changé de camp).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrderBlock {
    pub up: bool,
    pub breaker: bool,
    pub i0: usize,
    pub top: f64,
    pub bot: f64,
    /// Bougie d'invalidation (`None` = encore actif).
    pub end: Option<usize>,
    /// Bougie de la cassure qui l'a créé.
    pub from: usize,
}

/// FVG (`inverse` = iFVG). `up` garde le sens du FVG d'origine.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fvg {
    pub up: bool,
    pub inverse: bool,
    pub i0: usize,
    pub top: f64,
    pub bot: f64,
    pub end: Option<usize>,
    pub from: usize,
    pub touches: u32,
    /// Traversé en clôture.
    pub filled: bool,
    /// Usé (trop de touches).
    pub worn: bool,
    in_zone: bool,
}

impl Fvg {
    /// Zone de support (FVG haussier ou iFVG issu d'un FVG baissier).
    pub fn support(&self) -> bool {
        self.up != self.inverse
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BreakType {
    Bos,
    Choch,
}

/// Cassure de structure : du swing cassé (`i0`, `price`) à la bougie de cassure `i1`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StructureBreak {
    pub i0: usize,
    pub i1: usize,
    pub price: f64,
    pub up: bool,
    pub kind: BreakType,
}

/// Equal highs / lows (`up` = EQH). `swept` = bougie qui a pris la liquidité.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EqualLevel {
    pub i0: usize,
    pub i1: usize,
    pub price: f64,
    pub up: bool,
    pub swept: Option<usize>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Smc {
    /// UT des zones (secondes) quand l'option UT supérieure est active ; les index pointent alors dans `htf_bars`.
    pub tf: Option<i64>,
    pub htf_bars: Option<Vec<Bar>>,
    /// 1 haussière, −1 baissière, 0 pas encore de cassure.
    pub trend: i8,
    pub order_blocks: Vec<OrderBlock>,
    pub fvgs: Vec<Fvg>,
    pub breaks: Vec<StructureBreak>,
    pub equal: Vec<EqualLevel>,
}

#[derive(Debug, Clone, Copy)]
struct Swing {
    p: f64,
    i: usize,
    taken: bool,
}

/// ATR 14 du site : moyenne simple pendant les 14 premières bougies, puis Wilder.
fn atr14(bars: &[Bar]) -> Vec<f64> {
    let mut a = 0.0;
    (0..bars.len())
        .map(|i| {
            let b = &bars[i];
            let tr = if i == 0 {
                b.range()
            } else {
                b.range().max((b.high - bars[i - 1].close).abs()).max((b.low - bars[i - 1].close).abs())
            };
            a = if i < 14 {
                (a * i as f64 + tr) / (i as f64 + 1.0)
            } else {
                (a * 13.0 + tr) / 14.0
            };
            a
        })
        .collect()
}

/// Calcul sur une série de bougies (celle de la chart ou de l'UT supérieure).
pub fn core(b: &[Bar], s: &Settings) -> Smc {
    let n = b.len();
    let (h, l, c, o): (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) = (
        b.iter().map(|x| x.high).collect(),
        b.iter().map(|x| x.low).collect(),
        b.iter().map(|x| x.close).collect(),
        b.iter().map(|x| x.open).collect(),
    );
    let atr = atr14(b);
    let len = s.len.max(2);
    let mut r = Smc::default();
    let (mut hi, mut lo): (Option<Swing>, Option<Swing>) = (None, None);
    let (mut pr_h, mut pr_l): (Option<Swing>, Option<Swing>) = (None, None);
    let mut trend: i8 = 0;

    for i in 0..n {
        // pivots confirmés à i (sommet / creux à j = i − len), stricts des deux côtés
        if i >= 2 * len {
            let j = i - len;
            let others = (j - len..=i).filter(|&k| k != j);
            let is_h = others.clone().all(|k| h[k] < h[j]);
            let is_l = others.clone().all(|k| l[k] > l[j]);
            if is_h {
                if let Some(a) = hi.filter(|_| s.eq) {
                    if (a.p - h[j]).abs() <= s.eq_thr * atr[j] && j - a.i > len {
                        r.equal.push(EqualLevel {
                            i0: a.i,
                            i1: j,
                            price: a.p.max(h[j]),
                            up: true,
                            swept: None,
                        });
                    }
                }
                hi = Some(Swing { p: h[j], i: j, taken: false });
            }
            if is_l {
                if let Some(a) = lo.filter(|_| s.eq) {
                    if (a.p - l[j]).abs() <= s.eq_thr * atr[j] && j - a.i > len {
                        r.equal.push(EqualLevel {
                            i0: a.i,
                            i1: j,
                            price: a.p.min(l[j]),
                            up: false,
                            swept: None,
                        });
                    }
                }
                lo = Some(Swing { p: l[j], i: j, taken: false });
            }
        }

        // cassures : BOS = dernier swing dans le sens de la tendance ; CHoCH = niveau protégé (d'où est partie la dernière cassure)
        let mut ev: Option<(bool, Swing, BreakType)> = None;
        if let Some(p) = pr_h.filter(|p| trend == -1 && c[i] > p.p) {
            ev = Some((true, p, BreakType::Choch));
        } else if let Some(p) = hi.filter(|p| trend != -1 && !p.taken && c[i] > p.p) {
            ev = Some((true, p, BreakType::Bos));
        }
        if ev.is_none() {
            if let Some(p) = pr_l.filter(|p| trend == 1 && c[i] < p.p) {
                ev = Some((false, p, BreakType::Choch));
            } else if let Some(p) = lo.filter(|p| trend != 1 && !p.taken && c[i] < p.p) {
                ev = Some((false, p, BreakType::Bos));
            }
        }
        if let Some((up, lv, kind)) = ev {
            if let Some(x) = hi.as_mut() {
                x.taken = true;
            }
            if let Some(x) = lo.as_mut() {
                x.taken = true;
            }
            trend = if up { 1 } else { -1 };
            r.breaks.push(StructureBreak {
                i0: lv.i,
                i1: i,
                price: lv.p,
                up,
                kind,
            });
            // extrême entre le swing cassé et la cassure → nouveau niveau protégé
            let mut ext = i;
            for k in lv.i..i {
                if (up && l[k] < l[ext]) || (!up && h[k] > h[ext]) {
                    ext = k;
                }
            }
            // OB = dernière bougie opposée la plus extrême avant la cassure
            let mut k0: Option<usize> = None;
            for k in (lv.i..i).rev() {
                let opposite = if up { c[k] < o[k] } else { c[k] > o[k] };
                if opposite && k0.is_none_or(|z| if up { l[k] < l[z] } else { h[k] > h[z] }) {
                    k0 = Some(k);
                }
            }
            let k0 = k0.unwrap_or(ext);
            if up {
                pr_l = Some(Swing {
                    p: l[ext],
                    i: ext,
                    taken: false,
                });
                pr_h = None;
                let top = if s.ob_body { o[k0].max(c[k0]) } else { h[k0] };
                r.order_blocks.push(OrderBlock {
                    up: true,
                    breaker: false,
                    i0: k0,
                    top,
                    bot: l[k0],
                    end: None,
                    from: i,
                });
            } else {
                pr_h = Some(Swing {
                    p: h[ext],
                    i: ext,
                    taken: false,
                });
                pr_l = None;
                let bot = if s.ob_body { o[k0].min(c[k0]) } else { l[k0] };
                r.order_blocks.push(OrderBlock {
                    up: false,
                    breaker: false,
                    i0: k0,
                    top: h[k0],
                    bot,
                    end: None,
                    from: i,
                });
            }
        }

        // invalidation des OB (clôture ou mèche au-delà) → breaker block
        let count = r.order_blocks.len();
        for k in 0..count {
            let ob = r.order_blocks[k];
            if ob.end.is_some() || i <= ob.from {
                continue;
            }
            if ob.breaker {
                if (ob.up && c[i] < ob.bot) || (!ob.up && c[i] > ob.top) {
                    r.order_blocks[k].end = Some(i);
                }
                continue;
            }
            let px = if s.ob_wick {
                if ob.up {
                    l[i]
                } else {
                    h[i]
                }
            } else {
                c[i]
            };
            if (ob.up && px < ob.bot) || (!ob.up && px > ob.top) {
                r.order_blocks[k].end = Some(i);
                if s.breakers {
                    r.order_blocks.push(OrderBlock {
                        up: !ob.up,
                        breaker: true,
                        i0: i,
                        top: ob.top,
                        bot: ob.bot,
                        end: None,
                        from: i,
                    });
                }
            }
        }

        // nouveaux FVG
        if i >= 2 {
            let m = s.fvg_min * atr[i];
            let fvg = |up: bool, top: f64, bot: f64| Fvg {
                up,
                inverse: false,
                i0: i - 2,
                top,
                bot,
                end: None,
                from: i,
                touches: 0,
                filled: false,
                worn: false,
                in_zone: false,
            };
            if l[i] > h[i - 2] && l[i] - h[i - 2] >= m {
                r.fvgs.push(fvg(true, l[i], h[i - 2]));
            }
            if h[i] < l[i - 2] && l[i - 2] - h[i] >= m {
                r.fvgs.push(fvg(false, l[i - 2], h[i]));
            }
        }
        // touches, traversée en clôture → iFVG, usure après N touches
        let count = r.fvgs.len();
        for k in 0..count {
            let f = &mut r.fvgs[k];
            if f.end.is_some() || i <= f.from {
                continue;
            }
            let sup = f.support();
            let in_zone = if sup { l[i] <= f.top } else { h[i] >= f.bot };
            if in_zone && !f.in_zone {
                f.touches += 1;
            }
            f.in_zone = in_zone;
            if (sup && c[i] < f.bot) || (!sup && c[i] > f.top) {
                f.end = Some(i);
                f.filled = true;
                let f = *f;
                if !f.inverse && s.ifvg {
                    r.fvgs.push(Fvg {
                        inverse: true,
                        i0: i,
                        end: None,
                        from: i,
                        touches: 0,
                        filled: false,
                        worn: false,
                        in_zone: true,
                        ..f
                    });
                }
                continue;
            }
            if s.max_touch > 0 && f.touches >= s.max_touch {
                f.end = Some(i);
                f.worn = true;
            }
        }
    }

    // liquidité balayée
    for e in r.equal.iter_mut() {
        e.swept = (e.i1 + 1..n).find(|&i| if e.up { h[i] > e.price } else { l[i] < e.price });
    }
    r.trend = trend;
    r
}

/// Calcul complet : sur la chart, ou sur l'UT supérieure choisie (si elle a plus de 30 bougies).
pub fn compute(bars: &[Bar], chart_seconds: i64, s: &Settings) -> Smc {
    if let Some(hs) = s.tf.filter(|&hs| hs > chart_seconds) {
        let hb = aggregate(bars, hs);
        if hb.len() > 30 {
            let mut r = core(&hb, s);
            r.tf = Some(hs);
            r.htf_bars = Some(hb);
            return r;
        }
    }
    core(bars, s)
}

/// Résumé (tableau du site).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Summary {
    pub ob_up: usize,
    pub ob_down: usize,
    pub bb_up: usize,
    pub bb_down: usize,
    pub fvg: usize,
    pub ifvg: usize,
    /// Zone active la plus proche du dernier prix : (genre, bas, haut).
    pub nearest: Option<(ZoneKind, f64, f64)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZoneKind {
    Ob,
    Bb,
    Fvg,
    Ifvg,
}

impl Smc {
    pub fn summary(&self, last_close: f64) -> Summary {
        let obs = self.order_blocks.iter().filter(|z| z.end.is_none());
        let fvgs = self.fvgs.iter().filter(|z| z.end.is_none());
        let count = |breaker: bool, up: bool| obs.clone().filter(|z| z.breaker == breaker && z.up == up).count();
        let dist = |top: f64, bot: f64| (last_close - top).abs().min((last_close - bot).abs());
        // même ordre que le site : OB, puis BB, puis FVG / iFVG ; le premier gagne en cas d'égalité
        let zones = obs
            .clone()
            .filter(|z| !z.breaker)
            .map(|z| (ZoneKind::Ob, z.bot, z.top))
            .chain(obs.clone().filter(|z| z.breaker).map(|z| (ZoneKind::Bb, z.bot, z.top)))
            .chain(fvgs.clone().map(|z| (if z.inverse { ZoneKind::Ifvg } else { ZoneKind::Fvg }, z.bot, z.top)));
        let nearest = zones.fold(None, |best: Option<(ZoneKind, f64, f64)>, z| match best {
            Some(b) if dist(b.2, b.1) <= dist(z.2, z.1) => Some(b),
            _ => Some(z),
        });
        Summary {
            ob_up: count(false, true),
            ob_down: count(false, false),
            bb_up: count(true, true),
            bb_down: count(true, false),
            fvg: fvgs.clone().filter(|z| !z.inverse).count(),
            ifvg: fvgs.filter(|z| z.inverse).count(),
            nearest,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar(i: i64, o: f64, h: f64, l: f64, c: f64) -> Bar {
        Bar {
            time: i * 60,
            open: o,
            high: h,
            low: l,
            close: c,
            volume: 1.0,
        }
    }

    #[test]
    fn fvg_haussier_et_ifvg() {
        // gap entre le haut de la bougie 0 (10) et le bas de la bougie 2 (11), puis clôture sous 10 → iFVG
        let b = [
            bar(0, 9.0, 10.0, 8.5, 9.8),
            bar(1, 9.8, 12.0, 9.7, 11.8),
            bar(2, 11.8, 12.5, 11.0, 12.2),
            bar(3, 12.0, 12.1, 9.0, 9.5),
        ];
        let r = core(
            &b,
            &Settings {
                fvg_min: 0.0,
                ..Settings::default()
            },
        );
        let f = r.fvgs[0];
        assert!(f.up && !f.inverse && f.bot == 10.0 && f.top == 11.0);
        assert_eq!(f.end, Some(3));
        assert!(f.filled);
        assert!(r.fvgs[1].inverse && !r.fvgs[1].support());
    }

    #[test]
    fn resume_zone_la_plus_proche() {
        let mut r = Smc::default();
        r.order_blocks.push(OrderBlock {
            up: true,
            breaker: false,
            i0: 0,
            top: 10.0,
            bot: 9.0,
            end: None,
            from: 1,
        });
        r.fvgs.push(Fvg {
            up: false,
            inverse: false,
            i0: 0,
            top: 13.0,
            bot: 12.5,
            end: None,
            from: 1,
            touches: 0,
            filled: false,
            worn: false,
            in_zone: false,
        });
        let s = r.summary(12.0);
        assert_eq!((s.ob_up, s.fvg), (1, 1));
        assert_eq!(s.nearest, Some((ZoneKind::Fvg, 12.5, 13.0)));
    }
}
