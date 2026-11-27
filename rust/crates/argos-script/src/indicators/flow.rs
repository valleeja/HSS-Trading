//! 👣 Flux d'ordres, comme Argos v1, à partir des bougies ou 🧮 des vrais trades quand il y en a (`argos_core::orderflow`) :
//! - **Argos Footprint** : achats × ventes par rangée de prix dans chaque bougie · delta · POC · déséquilibres ;
//! - **Argos Bubbles · Trades & Liquidité** : bulles des achats / ventes agressifs (déséquilibre ≥ ratio), taille ou
//!   transparence selon le volume · POC (port de Footprint Imbalance Bubbles) ; 🧮 vrais trades seulement ;
//! - **Argos Tape** : ruban des mises à jour en direct (heure · prix · volume), delta de volume de la bougie et
//!   marqueurs de poussées de delta (port de Tape LucF ; se remplit en direct seulement).
//!
//! La répartition achat / vente d'une bougie (`est_rows`) est celle de v1 : le volume est partagé entre le corps et les
//! mèches, puis entre les rangées de prix touchées.

use crate::inputs::Inputs;
pub use crate::v1_inputs::{BUBBLES_GROUPS, BUBBLES_PARAMS, FOOTPRINT_BOURSES, FOOTPRINT_CASES, FOOTPRINT_GROUPS, FOOTPRINT_PARAMS, TAPE_GROUPS, TAPE_PARAMS};
use argos_core::orderflow::FlowBar;
use argos_core::Bar;

/// Pas « rond » (1, 2, 5 × 10ⁿ), comme `nice` de v1 : le même que l'orderflow réel.
pub use argos_core::orderflow::nice;

/// Hauteur d'une rangée : la plage médiane des 120 dernières bougies / `per`, arrondie (`autoStep` de v1).
pub fn auto_step(bars: &[Bar], per: f64) -> f64 {
    let mut rg: Vec<f64> = bars[bars.len().saturating_sub(120)..].iter().map(|b| b.high - b.low).collect();
    rg.sort_by(f64::total_cmp);
    let med = rg.get(rg.len() / 2).copied().filter(|v| *v != 0.0).unwrap_or(1.0);
    nice(med / per.max(2.0))
}

/// Une rangée de prix : (index de rangée, achats, ventes) ; prix bas = rangée × pas.
pub type Row = (i64, f64, f64);

/// Achats / ventes estimés par rangée dans la bougie `b` (`estRows` de v1), dans l'ordre où v1 les remplit.
pub fn est_rows(b: &Bar, step: f64) -> Vec<Row> {
    let v = b.volume;
    let mut m: Vec<Row> = vec![];
    if v == 0.0 || !(step > 0.0) {
        return m;
    }
    let mut add = |r: i64, x: f64, y: f64| match m.iter_mut().find(|c| c.0 == r) {
        Some(c) => {
            c.1 += x;
            c.2 += y;
        }
        None => m.push((r, x, y)),
    };
    let (h, l, o, c) = (b.high, b.low, b.open, b.close);
    let (bt, bb) = (o.max(c), o.min(c));
    let (body, tw, bw) = (bt - bb, h - bt, bb - l);
    let den = 2.0 * tw + 2.0 * bw + body;
    if !(den > 0.0) {
        add((c / step).floor() as i64, v / 2.0, v / 2.0);
        return m;
    }
    let rng = if h - l != 0.0 { h - l } else { 1.0 };
    let bf = if c >= o {
        0.5 + 0.4 * (body / rng).min(1.0)
    } else {
        0.5 - 0.4 * (body / rng).min(1.0)
    };
    for (a, z, vol, f) in [
        (bb, bt, body / den * v, bf),
        (bt, h, 2.0 * tw / den * v, 0.35),
        (l, bb, 2.0 * bw / den * v, 0.65),
    ] {
        if !(vol > 0.0) {
            continue;
        }
        let (r0, r1) = ((a / step).floor() as i64, ((z - 1e-12) / step).floor() as i64);
        for r in r0..=r1 {
            let ov = z.min((r + 1) as f64 * step) - a.max(r as f64 * step);
            if ov > 0.0 {
                let x = vol * ov / (z - a);
                add(r, x * f, x * (1.0 - f));
            }
        }
    }
    m
}

/// D'où viennent les rangées d'une bougie du footprint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origine {
    /// 🧮 Vrais trades, bougie complète.
    Reel,
    /// Estimées à partir de la bougie (`est_rows`) : pas de vrais trades pour elle.
    Estime,
    /// Estimées : des vrais trades existent mais ont pu manquer (démarrage, coupure, minutes absentes).
    Incomplet,
}

/// Une bougie du footprint.
#[derive(Debug, Clone, PartialEq)]
pub struct FpBar {
    pub i: usize,
    /// Rangées de bas en haut.
    pub rows: Vec<Row>,
    /// Rangée du POC (plus gros volume).
    pub poc: i64,
    /// Delta (achats − ventes).
    pub d: f64,
    /// Plus gros achat ou vente d'une rangée.
    pub mx: f64,
    pub origine: Origine,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Footprint {
    pub step: f64,
    pub bars: Vec<FpBar>,
}

/// La bougie d'orderflow réel de `b` (même heure de début) dans `reel` (trié par heure).
fn reel_de<'a>(reel: &'a [FlowBar], b: &Bar) -> Option<&'a FlowBar> {
    reel.binary_search_by_key(&b.time, |r| r.time).ok().map(|k| &reel[k])
}

/// Vraies rangées (pas de l'orderflow) remises au pas `step` de l'écran : chaque rangée va dans celle qui contient son
/// milieu.
fn au_pas(r: &FlowBar, step: f64) -> Vec<Row> {
    let mut m: Vec<Row> = vec![];
    for &(k, a, v) in &r.rangees {
        let j = (((k as f64 + 0.5) * r.pas) / step).floor() as i64;
        match m.iter_mut().find(|x| x.0 == j) {
            Some(x) => (x.1, x.2) = (x.1 + a, x.2 + v),
            None => m.push((j, a, v)),
        }
    }
    m
}

/// Argos Footprint (inputs de v1 : `per`, `nb`…). Une bougie avec des vrais trades complets dans `reel`
/// (`argos_core::orderflow`, à l'unité de la chart, trié par heure) prend ses vraies rangées ; sinon l'estimation.
pub fn footprint(bars: &[Bar], vals: &[f64], reel: &[FlowBar]) -> Footprint {
    footprint_de(bars, vals, reel, false)
}

/// 🏦 Argos Footprint multi-bourses : `reel` est l'orderflow de plusieurs bourses additionné
/// (`argos_core::orderflow::fusionner`, tailles en pièces ou en $ selon `unite`). Une bougie dont une bourse a manqué
/// un bout garde ce qui a été reçu (point orange) au lieu de l'estimé : l'estimé ne lit que la bourse de la chart. Une
/// bougie sans vrais trades : l'estimé (en $ : × le prix de chaque rangée).
pub fn footprint_multi(bars: &[Bar], vals: &[f64], reel: &[FlowBar]) -> Footprint {
    footprint_de(bars, vals, reel, true)
}

fn footprint_de(bars: &[Bar], vals: &[f64], reel: &[FlowBar], multi: bool) -> Footprint {
    let p = Inputs {
        params: &FOOTPRINT_PARAMS,
        vals,
    };
    let n = bars.len();
    let step = auto_step(bars, p.get("per"));
    let dollars = multi && p.pick("unite") == 1;
    let estime = |b: &Bar| {
        let mut rows = est_rows(b, step);
        if dollars {
            for r in &mut rows {
                let prix = (r.0 as f64 + 0.5) * step;
                (r.1, r.2) = (r.1 * prix, r.2 * prix);
            }
        }
        rows
    };
    let out = (n.saturating_sub(p.get("nb").round() as usize)..n)
        .filter_map(|i| {
            let (mut rows, origine) = match reel_de(reel, &bars[i]) {
                Some(r) if r.complete => (au_pas(r, step), Origine::Reel),
                Some(r) if multi && !r.rangees.is_empty() => (au_pas(r, step), Origine::Incomplet),
                Some(_) => (estime(&bars[i]), Origine::Incomplet),
                None => (estime(&bars[i]), Origine::Estime),
            };
            rows.sort_by_key(|r| r.0);
            let first = *rows.first()?;
            let (mut pk, mut d, mut mx) = (first, 0.0, 0.0f64);
            for x in &rows {
                if x.1 + x.2 > pk.1 + pk.2 {
                    pk = *x;
                }
                d += x.1 - x.2;
                mx = mx.max(x.1).max(x.2);
            }
            Some(FpBar {
                i,
                rows,
                poc: pk.0,
                d,
                mx,
                origine,
            })
        })
        .collect();
    Footprint { step, bars: out }
}

/// Delta estimé d'une bougie (achats − ventes) : la part acheteuse est la position de la clôture dans la bougie,
/// `volume × (clôture − bas) / (haut − bas)` (moitié-moitié sans plage).
pub fn delta_estime(b: &Bar) -> f64 {
    let r = b.high - b.low;
    let buy = if r != 0.0 { b.volume * (b.close - b.low) / r } else { b.volume / 2.0 };
    buy - (b.volume - buy)
}

/// 🧮 The real delta (buys − sells) of each bar from the bars with complete trades in `reel`, NaN elsewhere: Argos
/// Script's `argos.real_delta` (`lang::RunOptions`).
pub fn real_deltas(bars: &[Bar], reel: &[FlowBar]) -> Vec<f64> {
    bars.iter()
        .map(|b| match reel_de(reel, b) {
            Some(r) if r.complete => r.delta(),
            _ => f64::NAN,
        })
        .collect()
}

/// 🧮 CVD (delta cumulé) : le vrai delta des bougies aux trades complets dans `reel`, sinon `delta_estime`.
pub fn cvd(bars: &[Bar], reel: &[FlowBar]) -> Vec<f64> {
    let mut s = 0.0;
    bars.iter()
        .map(|b| {
            s += match reel_de(reel, b) {
                Some(r) if r.complete => r.delta(),
                _ => delta_estime(b),
            };
            s
        })
        .collect()
}

/// Une bulle : bougie, prix (milieu de la rangée), rayon (px), achat ?, opacité.
#[derive(Debug, Clone, PartialEq)]
pub struct Bubble {
    pub i: usize,
    pub price: f64,
    pub r: f64,
    pub buy: bool,
    pub alpha: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Bubbles {
    pub items: Vec<Bubble>,
    /// POC de chaque bougie : (bougie, prix).
    pub pocs: Vec<(usize, f64)>,
    /// Tableau résumé (comme v1).
    pub dash: Vec<(&'static str, String)>,
    /// 🧮 Opening time of the first bar that has real trades (`None`: no real trades for this market or unit).
    pub depuis: Option<i64>,
}

/// Argos Bubbles · Trades & Liquidité, from the 🧮 real trades only: `reel` is the market's real orderflow at the
/// chart's unit (sorted by time, as for [`footprint`]). A bar with complete real trades in `reel` (like the footprint
/// and the CVD) gets its real rows; a bar without them gets no bubble and no POC (no estimate from the candle any more).
/// `fmt` writes a price (« Dernière : Achat @ … »).
pub fn bubbles(bars: &[Bar], vals: &[f64], reel: &[FlowBar], fmt: &dyn Fn(f64) -> String) -> Bubbles {
    let p = Inputs { params: &BUBBLES_PARAMS, vals };
    let n = bars.len();
    let step = auto_step(bars, p.get("per"));
    let i0 = n.saturating_sub(p.get("nb").round() as usize);
    // (complete bars of real trades only, like the footprint and the CVD: a bar still forming or cut short has none)
    let depuis = bars.iter().find(|b| reel_de(reel, b).is_some_and(|r| r.complete)).map(|b| b.time);
    let (mut all, mut pocs, mut sum, mut cnt) = (vec![], vec![], 0.0, 0usize);
    for (i, bar) in bars.iter().enumerate().skip(i0) {
        let Some(r) = reel_de(reel, bar).filter(|r| r.complete) else { continue };
        let mut pk: Option<Row> = None;
        for x in au_pas(r, step) {
            sum += x.1 + x.2;
            cnt += 1;
            all.push((i, x));
            if pk.is_none_or(|k| x.1 + x.2 > k.1 + k.2) {
                pk = Some(x);
            }
        }
        if let Some(k) = pk {
            pocs.push((i, (k.0 as f64 + 0.5) * step));
        }
    }
    let avg = if cnt > 0 { sum / cnt as f64 } else { 1.0 };
    let (ratio, minv, mind) = (p.get("ratio"), p.get("minv"), p.get("mind"));
    let bub: Vec<(usize, Row)> = all
        .into_iter()
        .filter(|(_, (_, b, s))| {
            b + s >= minv * avg && (b - s).abs() >= mind * avg && ((if *s > 0.0 { b / s } else { *b }) >= ratio || (if *b > 0.0 { s / b } else { *s }) >= ratio)
        })
        .collect();
    let mut vs: Vec<f64> = bub.iter().map(|(_, x)| x.1 + x.2).collect();
    vs.sort_by(f64::total_cmp);
    let mn = minv * avg;
    let mx = if p.on("auto") {
        (mn * 1.01).max(vs.get((vs.len() as f64 * 0.97) as usize).copied().unwrap_or(0.0))
    } else {
        p.get("maxv") * avg
    };
    let rg = (mx - mn).max(1e-9);
    let size = p.pick("mode") == 0;
    let items = bub
        .iter()
        .map(|(i, (r, b, s))| {
            let k = ((b + s - mn) / rg).clamp(0.0, 1.0);
            let tier = if k >= 1.0 {
                1.0
            } else if k >= 0.7 {
                0.72
            } else if k >= 0.4 {
                0.45
            } else if k >= 0.15 {
                0.25
            } else {
                0.1
            };
            Bubble {
                i: *i,
                price: (*r as f64 + 0.5) * step,
                r: if size { 3.0 + tier * (p.get("maxr") - 3.0) } else { 6.0 },
                buy: b > s,
                alpha: if size { p.get("fop") / 100.0 } else { 0.2 + 0.7 * k },
            }
        })
        .collect();
    let rec: Vec<&(usize, Row)> = bub.iter().filter(|(i, _)| *i + 50 >= n).collect();
    let nb = rec.iter().filter(|(_, x)| x.1 > x.2).count();
    let ns = rec.len() - nb;
    let dash = vec![
        ("Bulles achat (50 b.)", nb.to_string()),
        ("Bulles vente (50 b.)", ns.to_string()),
        (
            "Pression",
            if nb > ns {
                "Acheteurs 🟢"
            } else if nb < ns {
                "Vendeurs 🔴"
            } else {
                "Neutre"
            }
            .to_string(),
        ),
        (
            "Dernière",
            bub.last().map_or("—".into(), |(_, x)| {
                format!("{} @ {}", if x.1 > x.2 { "Achat" } else { "Vente" }, fmt((x.0 as f64 + 0.5) * step))
            }),
        ),
    ];
    Bubbles {
        items,
        pocs: if p.on("poc") { pocs } else { vec![] },
        dash,
        depuis,
    }
}

/// Une ligne du ruban.
#[derive(Debug, Clone, PartialEq)]
pub struct TapeLine {
    /// Heure de la mise à jour (secondes Unix).
    pub ts: i64,
    pub px: f64,
    /// Le prix a bougé : +1 / −1 ; 0 = même prix.
    pub mv: i8,
    pub v: f64,
    pub up: bool,
    /// Première mise à jour d'une nouvelle bougie.
    pub nb: bool,
}

/// Un marqueur de poussée de delta : bougie (heure d'ouverture), haussier ?, texte (⬆ ⬇ ⇑ ⇓).
#[derive(Debug, Clone, PartialEq)]
pub struct TapeMark {
    pub time: i64,
    pub up: bool,
    pub text: &'static str,
}

/// Argos Tape : l'état reste d'une mise à jour à l'autre (comme `TAPE` de v1), remis à zéro au changement de
/// symbole ou d'unité de temps (`key`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Tape {
    key: (i64, i64),
    t: Option<i64>,
    v: f64,
    c: f64,
    up: bool,
    pub lines: Vec<TapeLine>,
    /// Delta de volume de la bougie en cours.
    pub d: f64,
    /// Delta des bougies finies : (heure d'ouverture, delta, bougie montante ?), dans l'ordre.
    hist: Vec<(i64, f64, bool)>,
}

impl Tape {
    /// Nouvelle mise à jour des bougies (`now` : heure de la mise à jour, secondes Unix).
    pub fn update(&mut self, bars: &[Bar], tf_seconds: i64, vals: &[f64], now: i64) {
        let p = Inputs { params: &TAPE_PARAMS, vals };
        let key = (tf_seconds, bars.first().map_or(0, |b| b.time));
        if self.key != key {
            *self = Tape {
                key,
                up: true,
                ..Default::default()
            };
        }
        let n = bars.len();
        let Some(b) = bars.last() else { return };
        let Some(t) = self.t else {
            (self.t, self.v, self.c) = (Some(b.time), b.volume, b.close);
            return;
        };
        let nb = b.time != t;
        if nb {
            let up = n < 2 || bars[n - 2].close >= bars[n - 2].open;
            match self.hist.iter_mut().find(|h| h.0 == t) {
                Some(h) => *h = (t, self.d, up),
                None => self.hist.push((t, self.d, up)),
            }
            (self.d, self.v, self.t) = (0.0, 0.0, Some(b.time));
        }
        let nv = b.volume - self.v;
        let moved = b.close != self.c;
        if moved {
            self.up = b.close > self.c;
        }
        if nv > 0.0 || moved {
            self.d += nv.max(0.0) * if self.up { 1.0 } else { -1.0 };
            if nv >= p.get("minv") {
                self.lines.insert(
                    0,
                    TapeLine {
                        ts: now,
                        px: b.close,
                        mv: if !moved {
                            0
                        } else if self.up {
                            1
                        } else {
                            -1
                        },
                        v: nv.max(0.0),
                        up: self.up,
                        nb,
                    },
                );
                self.lines.truncate(50);
            }
        }
        (self.v, self.c) = (b.volume, b.close);
    }

    /// Marqueurs (poussées de delta, delta au-delà du seuil), comme v1.
    pub fn marks(&self, vals: &[f64]) -> Vec<TapeMark> {
        let p = Inputs { params: &TAPE_PARAMS, vals };
        let mut h = self.hist.clone();
        h.sort_by_key(|x| x.0);
        let mut out = vec![];
        for k in 0..h.len() {
            let (t, d, up) = h[k];
            let div = (d > 0.0 && !up) || (d < 0.0 && up);
            if div {
                continue;
            }
            if let Some(&(_, pd, _)) = k.checked_sub(1).and_then(|j| h.get(j)) {
                if p.on("m1") && ((d > 0.0 && pd > 0.0 && d > pd) || (d < 0.0 && pd < 0.0 && d < pd)) {
                    out.push(TapeMark {
                        time: t,
                        up: d > 0.0,
                        text: if d > 0.0 { "⬆" } else { "⬇" },
                    });
                }
            }
            if p.on("m2") && d.abs() > p.get("m2v") {
                out.push(TapeMark {
                    time: t,
                    up: d > 0.0,
                    text: if d > 0.0 { "⇑" } else { "⇓" },
                });
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar(t: i64, o: f64, h: f64, l: f64, c: f64, v: f64) -> Bar {
        Bar {
            time: t,
            open: o,
            high: h,
            low: l,
            close: c,
            volume: v,
        }
    }

    #[test]
    fn rangees_et_pas() {
        assert_eq!((nice(0.13), nice(2.7), nice(6.0), nice(80.0)), (0.1, 2.0, 5.0, 100.0));
        let b = bar(0, 100.0, 104.0, 99.0, 103.0, 1000.0);
        let rows = est_rows(&b, 1.0);
        let tot: f64 = rows.iter().map(|r| r.1 + r.2).sum();
        assert!((tot - 1000.0).abs() < 1e-6, "tout le volume est réparti");
        assert!(rows.iter().map(|r| r.1).sum::<f64>() > 500.0, "bougie haussière : plus d'achats");
        let fp = footprint(&[b; 40], &crate::inputs::defaults(&FOOTPRINT_PARAMS), &[]);
        assert_eq!(fp.bars.len(), 40);
        assert_eq!(fp.step, 0.5, "plage médiane 5 / 8 rangées = 0,625 → arrondi à 0,5");
    }

    /// 🧮 Vraies rangées quand la bougie a des trades complets, estimation sinon (marquée incomplète s'il en manque).
    #[test]
    fn footprint_reel_estime_ou_incomplet() {
        let bars: Vec<Bar> = (0..40).map(|k| bar(k * 60, 100.0, 104.0, 99.0, 103.0, 1000.0)).collect();
        let vals = crate::inputs::defaults(&FOOTPRINT_PARAMS);
        let vrai = |time: i64, complete: bool| FlowBar {
            time,
            pas: 0.1,
            // 100,0-100,1 (2 achats), 100,1-100,2 (3 ventes), 101,7-101,8 (5 achats)
            rangees: vec![(1000, 2.0, 0.0), (1001, 0.0, 3.0), (1017, 5.0, 0.0)],
            complete,
        };
        let reel = vec![vrai(38 * 60, false), vrai(39 * 60, true)];
        let fp = footprint(&bars, &vals, &reel);
        assert_eq!(fp.step, 0.5);
        let (avant, inc, ok) = (&fp.bars[37], &fp.bars[38], &fp.bars[39]);
        assert_eq!((avant.origine, inc.origine, ok.origine), (Origine::Estime, Origine::Incomplet, Origine::Reel));
        assert_eq!(inc.rows, avant.rows, "incomplète : l'estimation, inchangée");
        // au pas 0,5 de l'écran : 100,0-100,5 (2 achats, 3 ventes) et 101,5-102,0 (5 achats)
        assert_eq!(ok.rows, vec![(200, 2.0, 3.0), (203, 5.0, 0.0)]);
        assert_eq!((ok.d, ok.poc, ok.mx), (4.0, 200, 5.0));
    }

    /// 🧮 CVD : le vrai delta quand il existe et est complet, sinon la formule d'estimation (la même que l'AI Trend).
    #[test]
    fn cvd_reel_sinon_estime() {
        let b = [bar(0, 100.0, 110.0, 100.0, 107.5, 100.0), bar(60, 100.0, 110.0, 100.0, 100.0, 40.0)];
        assert_eq!(delta_estime(&b[0]), 75.0 - 25.0, "clôture aux 3/4 : 75 % d'achats");
        assert_eq!(delta_estime(&bar(0, 5.0, 5.0, 5.0, 5.0, 8.0)), 0.0, "sans plage : moitié-moitié");
        assert_eq!(cvd(&b, &[]), vec![50.0, 50.0 - 40.0]);
        let reel = |complete| FlowBar {
            time: 60,
            pas: 1.0,
            rangees: vec![(100, 30.0, 10.0)],
            complete,
        };
        assert_eq!(cvd(&b, &[reel(true)]), vec![50.0, 70.0], "vrai delta +20");
        assert_eq!(cvd(&b, &[reel(false)]), vec![50.0, 10.0], "incomplet : estimation");
        // argos.real_delta: the real delta where the trades are complete, NaN elsewhere (no estimate)
        let d = real_deltas(&b, &[reel(true)]);
        assert!(d[0].is_nan() && d[1] == 20.0);
        assert!(real_deltas(&b, &[reel(false)]).iter().all(|x| x.is_nan()));
    }

    /// 🧮 Bubbles from the real trades only: bubbles and POC on the bars that have trades (complete or not), none on
    /// the others (no estimate any more); `depuis` is the first bar with real trades.
    #[test]
    fn bulles_des_vrais_trades_seulement() {
        let bars: Vec<Bar> = (0..40).map(|k| bar(k * 60, 100.0, 104.0, 99.0, 103.0, 1000.0)).collect();
        let vals = crate::inputs::defaults(&BUBBLES_PARAMS);
        let step = auto_step(
            &bars,
            Inputs {
                params: &BUBBLES_PARAMS,
                vals: &vals,
            }
            .get("per"),
        );
        let vrai = |time: i64, complete: bool| FlowBar {
            time,
            pas: step,
            // three balanced rows and one row of aggressive buyers (100 against 1)
            rangees: vec![(1000, 5.0, 5.0), (1001, 5.0, 5.0), (1002, 5.0, 5.0), (1003, 100.0, 1.0)],
            complete,
        };
        let reel = vec![vrai(38 * 60, false), vrai(39 * 60, true)];
        let b = bubbles(&bars, &vals, &reel, &|v| v.to_string());
        // (bar 38: real trades but not complete — like the footprint and the CVD, none)
        assert_eq!(b.items.iter().map(|x| x.i).collect::<Vec<_>>(), vec![39], "only the bars with complete trades");
        assert!(b.items.iter().all(|x| x.buy && (x.price - 1003.5 * step).abs() < 1e-9));
        assert_eq!(b.pocs.iter().map(|x| x.0).collect::<Vec<_>>(), vec![39], "a POC only where there are trades");
        assert_eq!(b.depuis, Some(39 * 60));
        assert_eq!(b.dash[0], ("Bulles achat (50 b.)", "1".to_string()));
        let rien = bubbles(&bars, &vals, &[], &|v| v.to_string());
        assert!(rien.items.is_empty() && rien.pocs.is_empty(), "no real trades: no bubble, no estimate");
        assert_eq!(rien.depuis, None);
    }

    #[test]
    fn ruban_en_direct() {
        let vals = crate::inputs::defaults(&TAPE_PARAMS);
        let mut t = Tape::default();
        let mut bars = vec![bar(0, 10.0, 11.0, 9.0, 10.5, 100.0), bar(60, 10.5, 10.6, 10.4, 10.5, 10.0)];
        t.update(&bars, 60, &vals, 1);
        bars[1].close = 10.6;
        bars[1].volume = 15.0;
        t.update(&bars, 60, &vals, 2);
        assert_eq!(t.lines.len(), 1);
        assert_eq!((t.lines[0].v, t.lines[0].mv, t.d), (5.0, 1, 5.0));
        bars.push(bar(120, 10.6, 10.6, 10.6, 10.6, 3.0));
        t.update(&bars, 60, &vals, 3);
        assert_eq!(t.d, 3.0, "nouvelle bougie : delta repart");
        assert!(t.lines[0].nb);
    }
}
