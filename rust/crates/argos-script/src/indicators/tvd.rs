//! Argos TVD Pro — AI Index (RSI + Stoch + momentum), 4 EMA + nuage, bougies vector (volume), divergences avec
//! lignes, signaux Trend / AI / GOLD / retournements. Même code que v1, mêmes inputs (groupes, défauts, min, max),
//! sauf ses six moyennes : `ta.ema` de TradingView, comme son script Pine (`pine/CCQc_TVD_Pro.pine`), depuis B-098.

use crate::inputs::Inputs;
use crate::ta::{ema, highest, lowest, pivot, rsi};
use argos_core::Bar;

/// Groupes et inputs de v1 (générés depuis app.js : `v1_inputs`).
pub use crate::v1_inputs::{TVD_GROUPS as GROUPS, TVD_PARAMS as PARAMS};

/// Repère sur la chart (sous ou au-dessus de la bougie).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkKind {
    TrendBuy,
    TrendSell,
    AiBuy,
    AiSell,
    RevBull,
    RevBear,
    GoldBuy,
    GoldSell,
    Bull,
    HBull,
    Bear,
    HBear,
}

impl MarkKind {
    /// Texte de l'étiquette (vide : triangle, rond ou losange).
    pub fn text(self) -> &'static str {
        match self {
            MarkKind::GoldBuy => "🚀 GOLD BUY",
            MarkKind::GoldSell => "💀 GOLD SELL",
            MarkKind::Bull => "Bull",
            MarkKind::HBull => "H Bull",
            MarkKind::Bear => "Bear",
            MarkKind::HBear => "H Bear",
            _ => "",
        }
    }
    pub fn below(self) -> bool {
        matches!(
            self,
            MarkKind::TrendBuy | MarkKind::AiBuy | MarkKind::RevBull | MarkKind::GoldBuy | MarkKind::Bull | MarkKind::HBull
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mark {
    pub index: usize,
    pub kind: MarkKind,
}

/// Ligne de divergence sur le prix, entre deux pivots ; `kind` : Bull, HBull, Bear ou HBear.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DivLine {
    pub kind: MarkKind,
    pub i0: usize,
    pub p0: f64,
    pub i1: usize,
    pub p1: f64,
}

/// Couleur vector d'une bougie : volume ≥ moyenne × mult (haute) ou ≥ × 1,5 (moyenne).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vector {
    HighUp,
    HighDown,
    MidUp,
    MidDown,
}

/// Résultat.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Tvd {
    pub fast: Vec<f64>,
    pub slow: Vec<f64>,
    pub mid: Vec<f64>,
    pub trend: Vec<f64>,
    pub ai: Vec<f64>,
    pub signal: Vec<f64>,
    /// Nuage entre EMA Fast et Slow : 1 haussier, −1 baissier, 0 neutre (vide si coupé).
    pub cloud: Vec<i8>,
    /// Bougies vector (vide si coupé).
    pub vector: Vec<Option<Vector>>,
    pub marks: Vec<Mark>,
    pub divs: Vec<DivLine>,
    /// Tableau de bord (nom, valeur) ; vide si coupé.
    pub dash: Vec<(&'static str, String)>,
    /// 🔔 Alertes du Pine : (bougie, numéro dans `ALERTS`).
    pub alert_at: Vec<(usize, usize)>,
}

/// 🔔 Les 11 alertes du Pine (noms des `alertcondition`), dans l'ordre de `Tvd::alerts`.
pub const ALERTS: [&str; 11] = [
    "Trend BUY",
    "Trend SELL",
    "GOLD SELL",
    "GOLD BUY",
    "AI BUY",
    "AI SELL",
    "Vector Candle 200%",
    "Bull Div",
    "Bear Div",
    "Hidden Bull Div",
    "Hidden Bear Div",
];

impl Tvd {
    /// 🔔 Signaux des alertes (`ALERTS`) : 1 sur les bougies où le signal arrive, 0 ailleurs.
    pub fn alerts(&self) -> Vec<(&'static str, Vec<f64>)> {
        let n = self.ai.len();
        let mut out: Vec<(&'static str, Vec<f64>)> = ALERTS.iter().map(|a| (*a, vec![0.0; n])).collect();
        for &(i, k) in self.alert_at.iter().filter(|(i, k)| *i < n && *k < ALERTS.len()) {
            out[k].1[i] = 1.0;
        }
        out
    }
}

/// Moyenne simple qui vaut NaN si une valeur de la fenêtre manque (comme v1).
fn sma_n(x: &[f64], len: usize) -> Vec<f64> {
    (0..x.len())
        .map(|i| {
            if i + 1 < len {
                f64::NAN
            } else {
                x[i + 1 - len..=i].iter().sum::<f64>() / len as f64
            }
        })
        .collect()
}

/// Écart-type (population) sur `len`.
fn sd_n(x: &[f64], len: usize) -> Vec<f64> {
    (0..x.len())
        .map(|i| {
            if i + 1 < len {
                return f64::NAN;
            }
            let w = &x[i + 1 - len..=i];
            let m = w.iter().sum::<f64>() / len as f64;
            (w.iter().map(|v| (v - m).powi(2)).sum::<f64>() / len as f64).sqrt()
        })
        .collect()
}

pub fn compute(bars: &[Bar], vals: &[f64]) -> Tvd {
    let p = Inputs { params: &PARAMS, vals };
    let n = bars.len();
    let c: Vec<f64> = bars.iter().map(|b| b.close).collect();
    let (o, h, lw): (Vec<f64>, Vec<f64>, Vec<f64>) = (
        bars.iter().map(|b| b.open).collect(),
        bars.iter().map(|b| b.high).collect(),
        bars.iter().map(|b| b.low).collect(),
    );
    let v: Vec<f64> = bars.iter().map(|b| b.volume).collect();
    // AI Index = (RSI + Stoch %K lissé + momentum normalisé par tanh) / 3, lissé en EMA
    let rs = rsi(&c, p.len("rsiLen"));
    let sl = p.len("stochLen");
    let (hh, ll) = (highest(&h, sl), lowest(&lw, sl));
    let st_raw: Vec<f64> = (0..n)
        .map(|i| {
            if hh[i].is_finite() && hh[i] - ll[i] != 0.0 {
                (c[i] - ll[i]) / (hh[i] - ll[i]) * 100.0
            } else {
                f64::NAN
            }
        })
        .collect();
    let st_k = sma_n(&st_raw, 3);
    let ml = p.len("momLen");
    let sd = sd_n(&c, ml);
    let mom: Vec<f64> = (0..n)
        .map(|i| {
            if i < ml || !sd[i].is_finite() {
                f64::NAN
            } else if sd[i] > 0.0 {
                50.0 + 50.0 * ((c[i] - c[i - ml]) / (sd[i] * 2.0)).clamp(-20.0, 20.0).tanh()
            } else {
                50.0
            }
        })
        .collect();
    let raw: Vec<f64> = (0..n)
        .map(|i| {
            if rs[i].is_finite() && st_k[i].is_finite() && mom[i].is_finite() {
                (rs[i] + st_k[i] + mom[i]) / 3.0
            } else {
                f64::NAN
            }
        })
        .collect();
    let ai = ema(&raw, p.len("smoothLen"));
    let sig = ema(&ai, p.len("signalLen"));
    let (ef, es, em, et) = (
        ema(&c, p.len("emaFastLen")),
        ema(&c, p.len("emaSlowLen")),
        ema(&c, p.len("emaMidLen")),
        ema(&c, p.len("emaTrendLen")),
    );
    let va = sma_n(&v, 20);
    let (bull, bear, vm) = (p.get("bullLvl"), p.get("bearLvl"), p.get("vecMult"));
    let (cloud_on, vec_on, sig_on, ai_on, pat_on, need_ema) = (
        p.on("showCloud"),
        p.on("showVector"),
        p.on("showSignals"),
        p.on("showAiSignals"),
        p.on("showPatterns"),
        p.on("trendNeedsEma"),
    );
    let (l, r) = (p.len("pivL"), p.len("pivR"));
    let (rmin, rmax) = (p.get("rangeMin"), p.get("rangeMax"));
    let (reg, hid, zone) = (p.on("showReg"), p.on("showHid"), p.on("zoneFilter"));
    let cross = |i: usize| i > 0 && ai[i] > sig[i] && ai[i - 1] <= sig[i - 1];
    let under = |i: usize| i > 0 && ai[i] < sig[i] && ai[i - 1] >= sig[i - 1];
    let vec_of = |i: usize| {
        let hi = va[i].is_finite() && v[i] >= va[i] * vm;
        (hi, !hi && va[i].is_finite() && v[i] >= va[i] * 1.5)
    };

    let mut out = Tvd {
        cloud: if cloud_on { vec![0; n] } else { vec![] },
        vector: if vec_on { vec![None; n] } else { vec![] },
        ..Default::default()
    };
    let (mut ph_prev, mut pl_prev): (Option<(f64, f64, usize)>, Option<(f64, f64, usize)>) = (None, None);
    let (mut last_div, mut last_sig): (Option<(MarkKind, usize)>, Option<(MarkKind, usize)>) = (None, None);
    for i in 0..n {
        let up = c[i] >= o[i];
        let (vh, vmid) = vec_of(i);
        if cloud_on {
            out.cloud[i] = if ai[i] > bull && c[i] > es[i] {
                1
            } else if ai[i] < bear && c[i] < es[i] {
                -1
            } else {
                0
            };
        }
        if vec_on {
            out.vector[i] = match (vh, vmid, up) {
                (true, _, true) => Some(Vector::HighUp),
                (true, _, false) => Some(Vector::HighDown),
                (_, true, true) => Some(Vector::MidUp),
                (_, true, false) => Some(Vector::MidDown),
                _ => None,
            };
        }
        let tu = i > 0 && ai[i] > bull && ai[i - 1] <= bull && (!need_ema || c[i] > es[i]);
        let td = i > 0 && ai[i] < bear && ai[i - 1] >= bear && (!need_ema || c[i] < es[i]);
        let (gs, gb) = (td && vh && c[i] < et[i], tu && vh && c[i] > et[i]);
        let (ab, as_) = (cross(i) && ai[i] < bear, under(i) && ai[i] > bull);
        // 🔔 alertes du Pine (sans les boutons d'affichage) : Trend BUY / SELL, GOLD SELL / BUY, AI BUY / SELL, Vector 200 %
        for (on, k) in [(tu, 0), (td, 1), (gs, 2), (gb, 3), (ab, 4), (as_, 5), (vh, 6)] {
            if on {
                out.alert_at.push((i, k));
            }
        }
        let body = (c[i] - o[i]).abs();
        let (uw, dw) = (h[i] - o[i].max(c[i]), o[i].min(c[i]) - lw[i]);
        let be = i > 0 && c[i] > o[i] && c[i - 1] < o[i - 1] && c[i] >= o[i - 1] && o[i] <= c[i - 1];
        let se = i > 0 && c[i] < o[i] && c[i - 1] > o[i - 1] && c[i] <= o[i - 1] && o[i] >= c[i - 1];
        let brev = (be || (body > 0.0 && dw >= body * 2.0 && uw <= body)) && ai[i] < 50.0;
        let srev = (se || (body > 0.0 && uw >= body * 2.0 && dw <= body)) && ai[i] > 50.0;
        for (on, kind) in [
            (sig_on && tu && !gb, MarkKind::TrendBuy),
            (sig_on && td && !gs, MarkKind::TrendSell),
            (ai_on && ab, MarkKind::AiBuy),
            (ai_on && as_, MarkKind::AiSell),
            (pat_on && brev, MarkKind::RevBull),
            (pat_on && srev, MarkKind::RevBear),
            (sig_on && gs, MarkKind::GoldSell),
            (sig_on && gb, MarkKind::GoldBuy),
        ] {
            if on {
                out.marks.push(Mark { index: i, kind });
            }
        }
        if gb || gs || tu || td || ab || as_ {
            let k = if gb {
                MarkKind::GoldBuy
            } else if gs {
                MarkKind::GoldSell
            } else if tu {
                MarkKind::TrendBuy
            } else if td {
                MarkKind::TrendSell
            } else if ab {
                MarkKind::AiBuy
            } else {
                MarkKind::AiSell
            };
            last_sig = Some((k, i));
        }
        // divergences sur les pivots de l'AI Index (confirmés R bougies plus tard, comme ta.pivothigh)
        let Some(pb) = i.checked_sub(r).filter(|&pb| ai[pb].is_finite()) else {
            continue;
        };
        for top in [true, false] {
            // (comme `ta.pivothigh` / `ta.pivotlow` du Pine : plus haut (bas) que les L bougies d'avant, au moins aussi haut
            // (bas) que les R d'après ; avant B-102, une égalité à droite refusait le pivot, comme `topAt` / `botAt` de v1)
            if pivot(&ai, i, l, r, top).is_none() {
                continue;
            }
            let (osc, pr) = (ai[pb], if top { h[pb] } else { lw[pb] });
            let prev = if top { &mut ph_prev } else { &mut pl_prev };
            if let Some((po, pp, pbi)) = *prev {
                let d = (pb - pbi) as f64;
                let ok = d >= rmin && d <= rmax && (!zone || if top { osc > 50.0 } else { osc < 50.0 });
                let (rb, hb) = if top {
                    (reg && ok && pr > pp && osc < po, hid && ok && pr < pp && osc > po)
                } else {
                    (reg && ok && pr < pp && osc > po, hid && ok && pr > pp && osc < po)
                };
                for (on, kind) in [
                    (rb, if top { MarkKind::Bear } else { MarkKind::Bull }),
                    (hb, if top { MarkKind::HBear } else { MarkKind::HBull }),
                ] {
                    if on {
                        out.divs.push(DivLine {
                            kind,
                            i0: pbi,
                            p0: pp,
                            i1: pb,
                            p1: pr,
                        });
                        out.marks.push(Mark { index: pb, kind });
                        last_div = Some((kind, pb));
                        // 🔔 alerte au moment où le Pine la voit (pivot confirmé, à la bougie i)
                        let k = match kind {
                            MarkKind::Bull => 7,
                            MarkKind::Bear => 8,
                            MarkKind::HBull => 9,
                            _ => 10,
                        };
                        out.alert_at.push((i, k));
                    }
                }
            }
            *prev = Some((osc, pr, pb));
        }
    }
    if p.on("showTable") && n > 0 {
        let z = n - 1;
        let f1 = |v: f64| if v.is_finite() { format!("{v:.1}") } else { "—".into() };
        let r0 = |v: f64| if v.is_finite() { format!("{}", v.round()) } else { "—".into() };
        let (vh, vmid) = vec_of(z);
        let ago = |x: Option<(MarkKind, usize)>, name: fn(MarkKind) -> &'static str| x.map_or("—".into(), |(k, b)| format!("{} ({} bars)", name(k), z - b));
        out.dash = vec![
            ("AI Index / Signal", format!("{} / {}", f1(ai[z]), f1(sig[z]))),
            (
                "Trend",
                if ai[z] > bull && c[z] > es[z] {
                    "BULL 🟢"
                } else if ai[z] < bear && c[z] < es[z] {
                    "BEAR 🔴"
                } else {
                    "NEUTRAL ⚪"
                }
                .into(),
            ),
            ("RSI / Stoch", format!("{} / {}", r0(rs[z]), r0(st_k[z]))),
            ("EMA Fast / Slow", if ef[z] > es[z] { "Bull ✅" } else { "Bear ❌" }.into()),
            ("EMA Trend", if c[z] > et[z] { "Above ✅" } else { "Below ❌" }.into()),
            (
                "Volume",
                if vh {
                    "Vector 🔥"
                } else if vmid {
                    "Vector 150%"
                } else {
                    "Normal"
                }
                .into(),
            ),
            (
                "Last Div",
                ago(last_div, |k| match k {
                    MarkKind::Bull => "Bull 🟢",
                    MarkKind::HBull => "H Bull 🟢",
                    MarkKind::Bear => "Bear 🔴",
                    _ => "H Bear 🔴",
                }),
            ),
            (
                "Last Signal",
                ago(last_sig, |k| match k {
                    MarkKind::GoldBuy => "🚀 GOLD BUY",
                    MarkKind::GoldSell => "💀 GOLD SELL",
                    MarkKind::TrendBuy => "Trend BUY ⬆️",
                    MarkKind::TrendSell => "Trend SELL ⬇️",
                    MarkKind::AiBuy => "AI BUY 🟢",
                    _ => "AI SELL 🔴",
                }),
            ),
        ];
    }
    Tvd {
        fast: ef,
        slow: es,
        mid: em,
        trend: et,
        ai,
        signal: sig,
        ..out
    }
}
