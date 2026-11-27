//! Argos SMC — structure HH/HL/LH/LL, BOS / CHoCH, OB, Breakers, FVG / iFVG, liquidité EQH / EQL + sweeps,
//! sessions et killzones ICT, Premium / Discount, biais multi-UT, tendance de fond, Fibonacci (OTE), confirmation,
//! plan entrée / SL / TP et score de confiance /10. Même logique que l'« Argos SMC » du site
//! (script Pine personnel de Jee, créé avec l'IA). Les dessins et couleurs restent dans la couche chart.

use argos_core::{aggregate, Bar};
use chrono::{TimeZone, Timelike};

/// Tendance de fond (les setups la suivent toujours).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fond {
    /// Vote : Daily ×3 · 4H ×2 · EMA ×2 · 1H ×1.
    Smart,
    /// 4H + Daily et EMA d'accord (strict).
    Both,
    /// Structure 4H + Daily.
    Htf,
    /// EMA seulement.
    Ema,
    /// Désactivé (biais multi-UT).
    Off,
}

/// Fuseau des horaires de session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionTz {
    NewYork,
    London,
    Utc,
}

impl SessionTz {
    fn tz(self) -> chrono_tz::Tz {
        match self {
            SessionTz::NewYork => chrono_tz::America::New_York,
            SessionTz::London => chrono_tz::Europe::London,
            SessionTz::Utc => chrono_tz::UTC,
        }
    }
}

/// Heures de début / fin d'une session (heure locale du fuseau choisi, pas de 0,5 h).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hours {
    pub start: f64,
    pub end: f64,
}

/// Une unité de temps du biais multi-UT : secondes et poids.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MtfSlot {
    pub seconds: i64,
    pub weight: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    pub swing_len: usize,
    pub ob_use_body: bool,
    pub max_ob: usize,
    pub max_bb: usize,
    pub max_fvg: usize,
    pub fvg_min_atr: f64,
    pub eq_tol: f64,
    /// Sweep « récent » = moins de N bougies.
    pub sweep_memory: usize,
    /// Range de référence Premium / Discount (bougies).
    pub pd_len: usize,
    pub mtf: [MtfSlot; 4],
    pub session_tz: SessionTz,
    pub asia: Hours,
    pub london: Hours,
    pub new_york: Hours,
    pub fond: Fond,
    pub ema_len: usize,
    /// Étendre l'OTE jusqu'à 88,6 % (sinon 78,6 %).
    pub ote886: bool,
    pub fib1_len: usize,
    pub fib2_len: usize,
    /// Attendre la confirmation dans l'OTE avant de proposer un setup.
    pub wait_conf: bool,
    /// Confirmation : cassure du plus bas / haut des N dernières bougies.
    pub conf_lb: usize,
    pub sl_buf_atr: f64,
    pub score_alert_min: u8,
}

/// Groupes et inputs de v1 (générés depuis app.js : `v1_inputs`).
pub use crate::v1_inputs::{SMC_PRO_GROUPS as GROUPS, SMC_PRO_PARAMS as PARAMS};

impl Settings {
    /// Réglages du calcul lus dans les inputs de v1 (valeurs dans l'ordre de `PARAMS`).
    pub fn from_inputs(vals: &[f64]) -> Self {
        let p = crate::inputs::Inputs { params: &PARAMS, vals };
        // UT des listes de v1 : 15m, 1H, 4H, Daily, Weekly
        let tf = |k: &str| [900, 3_600, 14_400, 86_400, 604_800][p.pick(k).min(4)];
        let slot = |n: usize| MtfSlot {
            seconds: tf(&format!("mtf{n}")),
            weight: p.get(&format!("mw{n}")),
        };
        let hours = |a: &str, b: &str| Hours {
            start: p.get(a),
            end: p.get(b),
        };
        Self {
            swing_len: p.len("swingLen"),
            ob_use_body: p.on("obUseBody"),
            max_ob: p.len("maxOB"),
            max_bb: p.len("maxBB"),
            max_fvg: p.len("maxFVG"),
            fvg_min_atr: p.get("fvgMinAtr"),
            eq_tol: p.get("eqTol"),
            sweep_memory: p.len("sweepMemory"),
            pd_len: p.len("pdLen"),
            mtf: [slot(1), slot(2), slot(3), slot(4)],
            session_tz: [SessionTz::NewYork, SessionTz::London, SessionTz::Utc][p.pick("sessTz").min(2)],
            asia: hours("asS", "asE"),
            london: hours("loS", "loE"),
            new_york: hours("nyS", "nyE"),
            fond: [Fond::Smart, Fond::Both, Fond::Htf, Fond::Ema, Fond::Off][p.pick("fond").min(4)],
            ema_len: p.len("emaLen"),
            ote886: p.on("ote886"),
            fib1_len: p.len("fib1Len"),
            fib2_len: p.len("fib2Len"),
            wait_conf: p.on("waitConf"),
            conf_lb: p.len("confLb"),
            sl_buf_atr: p.get("slBufAtr"),
            score_alert_min: p.get("scoreAlertMin").round().clamp(0.0, 255.0) as u8,
        }
    }
}

/// Valeurs par défaut = celles de v1 (une seule source : `PARAMS`).
impl Default for Settings {
    fn default() -> Self {
        Self::from_inputs(&crate::inputs::defaults(&PARAMS))
    }
}

/// Zone vivante (OB, BB, FVG, iFVG). `left` = bougie de départ.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Zone {
    pub top: f64,
    pub bot: f64,
    pub left: usize,
}

/// Zones vivantes, la plus récente en premier.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Zones {
    pub bull_ob: Vec<Zone>,
    pub bear_ob: Vec<Zone>,
    pub bull_fvg: Vec<Zone>,
    pub bear_fvg: Vec<Zone>,
    pub bull_bb: Vec<Zone>,
    pub bear_bb: Vec<Zone>,
    pub bull_ifvg: Vec<Zone>,
    pub bear_ifvg: Vec<Zone>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZoneKind {
    Ob,
    Fvg,
    Bb,
    Ifvg,
}

/// Cassure de structure : du niveau cassé (bougie `from`, prix `level`) à la bougie `at`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StructureEvent {
    pub from: usize,
    pub at: usize,
    pub level: f64,
    pub up: bool,
    pub choch: bool,
}

/// EQH (BSL) / EQL (SSL) : deux swings à peu près égaux.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EqualLevel {
    pub i0: usize,
    pub i1: usize,
    pub p0: f64,
    pub p1: f64,
    pub up: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SweepSource {
    Swing,
    Asia,
    London,
    NewYork,
}

/// Liquidité prise : `dir` = 1 (SSL / bas pris → haussier) ou −1 (BSL / haut pris → baissier).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sweep {
    pub at: usize,
    pub dir: i8,
    pub source: SweepSource,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SessionBox {
    pub i0: usize,
    pub i1: usize,
    pub hi: f64,
    pub lo: f64,
}

/// High / Low d'une session terminée, prolongé jusqu'à ce qu'il soit pris.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SessionLevel {
    pub i0: usize,
    pub price: f64,
    pub end: Option<usize>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SessionState {
    pub boxes: Vec<SessionBox>,
    pub current: Option<SessionBox>,
    pub levels: Vec<SessionLevel>,
    /// Range de la dernière session terminée (NaN si aucune).
    pub last_hi: f64,
    pub last_lo: f64,
}

/// Une ligne du biais multi-UT.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MtfRow {
    pub seconds: i64,
    pub weight: f64,
    /// 1 haussière, −1 baissière, 0 aucune.
    pub trend: i8,
    /// Dernier événement : 0 aucun, 1 BOS, 2 CHoCH.
    pub event: u8,
    /// Niveau d'invalidation (niveau protégé).
    pub invalidation: f64,
    /// Pas assez de bougies dans cette UT (ou UT plus petite) : on a pris la structure de la chart.
    pub same: bool,
}

/// Jambe de Fibonacci.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Leg {
    pub lo: f64,
    pub hi: f64,
    pub i0: usize,
    pub i1: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Poi {
    pub zone: Zone,
    pub kind: ZoneKind,
}

/// Zone de confluence POI ∩ OTE.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Confluence {
    pub top: f64,
    pub bot: f64,
    pub kind: ZoneKind,
    pub left: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfState {
    /// Attendre l'entrée dans la zone.
    None,
    /// Dans la zone : attendre la confirmation.
    Zone,
    /// Confirmé (bougie de rejet).
    Ok,
    /// Invalidé (au-delà de 100 %).
    Invalid,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Confirmation {
    pub state: ConfState,
    pub touch: Option<usize>,
    pub bar: Option<usize>,
    /// Extrême atteint depuis l'entrée dans la zone (sert au stop).
    pub ext: f64,
}

/// Plus haut / plus bas de la période précédente.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PrevRange {
    pub hi: f64,
    pub lo: f64,
    pub time: i64,
}

/// Confluence qui manque au score.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Missing {
    Fond,
    Ote,
    Confluence,
    Htf,
    /// Discount pour un long, Premium pour un short.
    PremiumDiscount,
    Sweep,
    Poi,
    Choch,
    Rr,
    Killzone,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Tendance de fond pas claire → pas de setup.
    NoTrend,
    /// Pas de biais clair (tendance de fond désactivée).
    NoBias,
    Invalidated,
    InZoneWait,
    WaitOte,
    APlus,
    Valid,
    WeakConfirmed,
    Weak,
    NoSetup,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SmcPro {
    pub zones: Zones,
    pub events: Vec<StructureEvent>,
    pub equal: Vec<EqualLevel>,
    pub sweeps: Vec<Sweep>,
    pub sessions: [SessionState; 3],
    pub in_asia: bool,
    pub in_london: bool,
    pub in_new_york: bool,
    pub trend: i8,
    pub swing_high: f64,
    pub swing_low: f64,
    pub protected_high: f64,
    pub protected_low: f64,
    pub range_top: f64,
    pub range_bot: f64,
    pub equilibrium: f64,
    pub pd_pct: f64,
    pub mtf: [MtfRow; 4],
    pub htf_score: f64,
    pub htf_total: f64,
    pub ema: f64,
    pub ema_dir: i8,
    pub htf_dir: i8,
    pub vote: i32,
    pub fond_dir: i8,
    pub dir: i8,
    pub poi: Option<Poi>,
    pub leg: Option<Leg>,
    pub ote_top: f64,
    pub ote_bot: f64,
    pub in_ote: bool,
    pub confluences: Vec<Confluence>,
    pub in_conf: bool,
    pub confirmation: Confirmation,
    pub leg2: Option<Leg>,
    pub ote2_top: f64,
    pub ote2_bot: f64,
    pub prev_day: Option<PrevRange>,
    pub prev_week: Option<PrevRange>,
    pub plan_ok: bool,
    pub entry: f64,
    pub sl: f64,
    pub tp1: f64,
    pub tp2: f64,
    pub rr: f64,
    pub raw: u8,
    pub score: u8,
    pub missing: Vec<Missing>,
    pub verdict: Verdict,
    /// Alerte sur la dernière bougie.
    pub buy: bool,
    pub sell: bool,
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

/// Pivots stricts (sommet, creux) à `j`, avec `l` bougies de chaque côté jusqu'à `to`.
fn strict_pivot(b: &[Bar], j: usize, l: usize, to: usize) -> (bool, bool) {
    let (mut ih, mut il) = (true, true);
    for k in (j - l..=to).filter(|&k| k != j) {
        if b[k].high >= b[j].high {
            ih = false;
        }
        if b[k].low <= b[j].low {
            il = false;
        }
    }
    (ih, il)
}

/// Structure seule (tendance, dernier événement, niveau d'invalidation) à chaque bougie — pour le multi-UT.
pub fn structure_only(b: &[Bar], l: usize) -> (Vec<i8>, Vec<u8>, Vec<f64>) {
    let n = b.len();
    let (mut tv, mut ev_v, mut inv) = (vec![0i8; n], vec![0u8; n], vec![f64::NAN; n]);
    let (mut t, mut ev) = (0i8, 0u8);
    let (mut sh, mut sl, mut hb_, mut lb_) = (f64::NAN, f64::NAN, true, true);
    let (mut ph, mut pl) = (f64::NAN, f64::NAN);
    let (mut phb, mut plb, mut shb, mut slb) = (0usize, 0usize, 0usize, 0usize);
    for i in 0..n {
        if i >= 2 * l {
            let j = i - l;
            let (ih, il) = strict_pivot(b, j, l, i);
            if ih {
                sh = b[j].high;
                shb = j;
                hb_ = false;
            }
            if il {
                sl = b[j].low;
                slb = j;
                lb_ = false;
            }
        }
        let c = b[i].close;
        let (mut up, mut dn) = (0u8, 0u8);
        if t == -1 && c > ph {
            up = 2;
        } else if t != -1 && !hb_ && c > sh {
            up = 1;
        }
        if up == 0 {
            if t == 1 && c < pl {
                dn = 2;
            } else if t != 1 && !lb_ && c < sl {
                dn = 1;
            }
        }
        if up > 0 {
            let from = if up == 2 { phb } else { shb };
            let mut lb = i;
            for k in from..i {
                if b[k].low < b[lb].low {
                    lb = k;
                }
            }
            pl = b[lb].low;
            plb = lb;
            ph = f64::NAN;
            ev = up;
            t = 1;
            hb_ = true;
            lb_ = true;
        }
        if dn > 0 {
            let from = if dn == 2 { plb } else { slb };
            let mut hb = i;
            for k in from..i {
                if b[k].high > b[hb].high {
                    hb = k;
                }
            }
            ph = b[hb].high;
            phb = hb;
            pl = f64::NAN;
            ev = dn;
            t = -1;
            lb_ = true;
            hb_ = true;
        }
        tv[i] = t;
        ev_v[i] = ev;
        inv[i] = match t {
            1 => pl,
            -1 => ph,
            _ => f64::NAN,
        };
    }
    (tv, ev_v, inv)
}

/// Ajoute une zone en tête ; garde les `max` plus récentes.
fn add(arr: &mut Vec<Zone>, z: Zone, max: usize) {
    arr.insert(0, z);
    if arr.len() > max {
        arr.pop();
    }
}

/// Zone cassée en clôture → passe dans `dst` (rôle inversé) ; touchée par la mèche (`wick`) → retirée.
fn flip(arr: &mut Vec<Zone>, bull: bool, wick: bool, dst: &mut Vec<Zone>, max: usize, bar: &Bar) {
    for k in (0..arr.len()).rev() {
        let z = arr[k];
        let broke = if bull { bar.close < z.bot } else { bar.close > z.top };
        let touched = wick && if bull { bar.low < z.bot } else { bar.high > z.top };
        if broke || touched {
            arr.remove(k);
            if broke {
                add(dst, z, max);
            }
        }
    }
}

fn clean(arr: &mut Vec<Zone>, bull: bool, bar: &Bar) {
    arr.retain(|z| !(if bull { bar.close < z.bot } else { bar.close > z.top }));
}

fn sign(v: f64) -> i8 {
    if v > 0.0 {
        1
    } else if v < 0.0 {
        -1
    } else {
        0
    }
}

/// Minute du jour (0–1439) dans le fuseau, comme le site : décalage de l'heure pleine + minutes écoulées.
fn minute_of_day(t: i64, tz: chrono_tz::Tz) -> i64 {
    let hk = t.div_euclid(3600);
    let local = tz.timestamp_opt(hk * 3600, 0).single().expect("heure valide");
    (local.hour() as i64 * 60 + local.minute() as i64 + (t - hk * 3600).div_euclid(60)) % 1440
}

fn in_session(m: i64, h: Hours) -> bool {
    let a = ((h.start * 60.0).round() as i64).rem_euclid(1440);
    let b = ((h.end * 60.0).round() as i64).rem_euclid(1440);
    if a == b {
        false
    } else if a < b {
        m >= a && m < b
    } else {
        m >= a || m < b
    }
}

#[derive(Default)]
struct SessRun {
    was: bool,
    hi: f64,
    lo: f64,
    i0: usize,
    hi_taken: bool,
    lo_taken: bool,
    hi_level: Option<usize>,
    lo_level: Option<usize>,
    out: SessionState,
}

pub fn compute(bars: &[Bar], chart_seconds: i64, s: &Settings) -> SmcPro {
    let n = bars.len();
    assert!(n > 0, "Argos SMC : aucune bougie");
    let l = s.swing_len.max(2);
    let (h, lo, c, o): (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) = (
        bars.iter().map(|b| b.high).collect(),
        bars.iter().map(|b| b.low).collect(),
        bars.iter().map(|b| b.close).collect(),
        bars.iter().map(|b| b.open).collect(),
    );
    let atr = atr14(bars);
    let mut z = Zones::default();
    let (mut events, mut equal, mut sweeps) = (vec![], vec![], vec![]);
    let (mut sw_h, mut sw_hb, mut sw_l, mut sw_lb) = (f64::NAN, None::<usize>, f64::NAN, None::<usize>);
    let (mut h_broken, mut l_broken, mut h_swept, mut l_swept) = (true, true, true, true);
    let mut trend: i8 = 0;
    let (mut last_sweep_dir, mut last_sweep_bar): (i8, Option<usize>) = (0, None);
    let (mut prot_h, mut prot_hb, mut prot_l, mut prot_lb) = (f64::NAN, 0usize, f64::NAN, 0usize);

    // sessions (heure du fuseau choisi, à la minute)
    let intra = chart_seconds <= 3600;
    let tz = s.session_tz.tz();
    let hours = [s.asia, s.london, s.new_york];
    let sources = [SweepSource::Asia, SweepSource::London, SweepSource::NewYork];
    let mut ss: [SessRun; 3] = Default::default();
    for x in ss.iter_mut() {
        x.hi_taken = true;
        x.lo_taken = true;
        x.out.last_hi = f64::NAN;
        x.out.last_lo = f64::NAN;
    }
    let mut in_ses = [false; 3];

    for i in 0..n {
        if i >= 2 * l {
            let j = i - l;
            let (ih, il) = strict_pivot(bars, j, l, i);
            if ih {
                if let Some(pb) = sw_hb.filter(|_| (h[j] - sw_h).abs() <= atr[i] * s.eq_tol) {
                    equal.push(EqualLevel {
                        i0: pb,
                        i1: j,
                        p0: sw_h,
                        p1: h[j],
                        up: true,
                    });
                }
                sw_h = h[j];
                sw_hb = Some(j);
                h_broken = false;
                h_swept = false;
            }
            if il {
                if let Some(pb) = sw_lb.filter(|_| (lo[j] - sw_l).abs() <= atr[i] * s.eq_tol) {
                    equal.push(EqualLevel {
                        i0: pb,
                        i1: j,
                        p0: sw_l,
                        p1: lo[j],
                        up: false,
                    });
                }
                sw_l = lo[j];
                sw_lb = Some(j);
                l_broken = false;
                l_swept = false;
            }
        }

        // 📐 structure : BOS = dernier swing dans le sens de la tendance ; CHoCH = niveau protégé
        let mut ev: Option<StructureEvent> = None;
        if trend == -1 && c[i] > prot_h {
            ev = Some(StructureEvent {
                from: prot_hb,
                at: i,
                level: prot_h,
                up: true,
                choch: true,
            });
        } else if let Some(fb) = sw_hb.filter(|_| trend != -1 && !h_broken && c[i] > sw_h) {
            ev = Some(StructureEvent {
                from: fb,
                at: i,
                level: sw_h,
                up: true,
                choch: false,
            });
        }
        if ev.is_none() {
            if trend == 1 && c[i] < prot_l {
                ev = Some(StructureEvent {
                    from: prot_lb,
                    at: i,
                    level: prot_l,
                    up: false,
                    choch: true,
                });
            } else if let Some(fb) = sw_lb.filter(|_| trend != 1 && !l_broken && c[i] < sw_l) {
                ev = Some(StructureEvent {
                    from: fb,
                    at: i,
                    level: sw_l,
                    up: false,
                    choch: false,
                });
            }
        }
        if let Some(e) = ev {
            events.push(e);
            let span = (i - e.from).min(300);
            let mut x = i;
            for k in 1..=span {
                if (e.up && lo[i - k] < lo[x]) || (!e.up && h[i - k] > h[x]) {
                    x = i - k;
                }
            }
            if span >= 1 {
                let (top, bot) = if s.ob_use_body { (o[x].max(c[x]), o[x].min(c[x])) } else { (h[x], lo[x]) };
                add(if e.up { &mut z.bull_ob } else { &mut z.bear_ob }, Zone { top, bot, left: x }, s.max_ob);
            }
            if e.up {
                prot_l = lo[x];
                prot_lb = x;
                prot_h = f64::NAN;
                trend = 1;
            } else {
                prot_h = h[x];
                prot_hb = x;
                prot_l = f64::NAN;
                trend = -1;
            }
            h_broken = true;
            l_broken = true;
        }
        if !h_broken && !h_swept && h[i] > sw_h && c[i] < sw_h {
            h_swept = true;
            last_sweep_dir = -1;
            last_sweep_bar = Some(i);
            sweeps.push(Sweep {
                at: i,
                dir: -1,
                source: SweepSource::Swing,
            });
        }
        if !l_broken && !l_swept && lo[i] < sw_l && c[i] > sw_l {
            l_swept = true;
            last_sweep_dir = 1;
            last_sweep_bar = Some(i);
            sweeps.push(Sweep {
                at: i,
                dir: 1,
                source: SweepSource::Swing,
            });
        }

        // sessions
        let m = if intra { minute_of_day(bars[i].time, tz) } else { -1 };
        let (mut sw_hi_src, mut sw_lo_src) = (None, None);
        for (si, x) in ss.iter_mut().enumerate() {
            let on = intra && in_session(m, hours[si]);
            let (started, ended) = (on && !x.was, !on && x.was);
            if started {
                x.hi = h[i];
                x.lo = lo[i];
                x.i0 = i;
            } else if on {
                x.hi = x.hi.max(h[i]);
                x.lo = x.lo.min(lo[i]);
            }
            if on {
                x.out.current = Some(SessionBox {
                    i0: x.i0,
                    i1: i,
                    hi: x.hi,
                    lo: x.lo,
                });
            }
            if ended {
                if let Some(b) = x.out.current.take() {
                    x.out.boxes.push(b);
                }
                x.out.last_hi = x.hi;
                x.out.last_lo = x.lo;
                x.hi_taken = false;
                x.lo_taken = false;
                x.out.levels.push(SessionLevel {
                    i0: i - 1,
                    price: x.hi,
                    end: None,
                });
                x.hi_level = Some(x.out.levels.len() - 1);
                x.out.levels.push(SessionLevel {
                    i0: i - 1,
                    price: x.lo,
                    end: None,
                });
                x.lo_level = Some(x.out.levels.len() - 1);
            }
            if !ended && !x.hi_taken && h[i] > x.out.last_hi {
                x.hi_taken = true;
                if c[i] < x.out.last_hi {
                    sw_hi_src = sw_hi_src.or(Some(sources[si]));
                }
                if let Some(k) = x.hi_level {
                    x.out.levels[k].end = Some(i);
                }
            }
            if !ended && !x.lo_taken && lo[i] < x.out.last_lo {
                x.lo_taken = true;
                if c[i] > x.out.last_lo {
                    sw_lo_src = sw_lo_src.or(Some(sources[si]));
                }
                if let Some(k) = x.lo_level {
                    x.out.levels[k].end = Some(i);
                }
            }
            x.was = on;
            in_ses[si] = on;
        }
        if let Some(src) = sw_hi_src {
            last_sweep_dir = -1;
            last_sweep_bar = Some(i);
            sweeps.push(Sweep { at: i, dir: -1, source: src });
        }
        if let Some(src) = sw_lo_src {
            last_sweep_dir = 1;
            last_sweep_bar = Some(i);
            sweeps.push(Sweep { at: i, dir: 1, source: src });
        }

        // FVG (la bougie du milieu doit aller dans le sens du gap)
        if i >= 2 {
            if lo[i] > h[i - 2] && lo[i] - h[i - 2] >= atr[i] * s.fvg_min_atr && c[i - 1] > o[i - 1] {
                add(
                    &mut z.bull_fvg,
                    Zone {
                        top: lo[i],
                        bot: h[i - 2],
                        left: i - 2,
                    },
                    s.max_fvg,
                );
            }
            if h[i] < lo[i - 2] && lo[i - 2] - h[i] >= atr[i] * s.fvg_min_atr && c[i - 1] < o[i - 1] {
                add(
                    &mut z.bear_fvg,
                    Zone {
                        top: lo[i - 2],
                        bot: h[i],
                        left: i - 2,
                    },
                    s.max_fvg,
                );
            }
        }
        let b = &bars[i];
        flip(&mut z.bull_ob, true, false, &mut z.bear_bb, s.max_bb, b);
        flip(&mut z.bear_ob, false, false, &mut z.bull_bb, s.max_bb, b);
        flip(&mut z.bull_fvg, true, true, &mut z.bear_ifvg, s.max_fvg, b);
        flip(&mut z.bear_fvg, false, true, &mut z.bull_ifvg, s.max_fvg, b);
        clean(&mut z.bull_bb, true, b);
        clean(&mut z.bear_bb, false, b);
        clean(&mut z.bull_ifvg, true, b);
        clean(&mut z.bear_ifvg, false, b);
    }
    let last = n - 1;
    let close = c[last];

    // Premium / Discount
    let pd_n = n.min(s.pd_len);
    let range_top = h[n - pd_n..].iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let range_bot = lo[n - pd_n..].iter().copied().fold(f64::INFINITY, f64::min);
    let equilibrium = (range_top + range_bot) / 2.0;
    let pd_pct = if range_top != range_bot {
        (close - range_bot) / (range_top - range_bot) * 100.0
    } else {
        50.0
    };

    // multi-UT
    let chart_s = structure_only(bars, l);
    let mtf = s.mtf.map(|slot| {
        if slot.seconds > chart_seconds {
            let hb = aggregate(bars, slot.seconds);
            if hb.len() > l * 2 + 2 {
                let (t, e, inv) = structure_only(&hb, l);
                let ix = hb.len() - 1;
                return MtfRow {
                    seconds: slot.seconds,
                    weight: slot.weight,
                    trend: t[ix],
                    event: e[ix],
                    invalidation: inv[ix],
                    same: false,
                };
            }
        }
        MtfRow {
            seconds: slot.seconds,
            weight: slot.weight,
            trend: chart_s.0[last],
            event: chart_s.1[last],
            invalidation: chart_s.2[last],
            same: slot.seconds != chart_seconds,
        }
    });
    let htf_score: f64 = mtf.iter().map(|x| x.trend as f64 * x.weight).sum();
    let htf_total: f64 = mtf.iter().map(|x| x.weight).sum();
    let dir_raw = if htf_score != 0.0 { sign(htf_score) } else { trend };

    // 🧭 tendance de fond
    let k = 2.0 / (s.ema_len as f64 + 1.0);
    let ema = c[1..].iter().fold(c[0], |e, v| v * k + e * (1.0 - k));
    let ema_dir = if n > s.ema_len { sign(close - ema) } else { 0 };
    let hs4 = mtf[2].trend as f64 * mtf[2].weight.max(1.0) + mtf[3].trend as f64 * mtf[3].weight.max(1.0);
    let htf_dir = sign(hs4);
    let vote = 3 * mtf[3].trend as i32 + 2 * mtf[2].trend as i32 + 2 * ema_dir as i32 + mtf[1].trend as i32;
    let fond_dir = match s.fond {
        Fond::Smart => vote.signum() as i8,
        Fond::Ema => ema_dir,
        Fond::Htf => htf_dir,
        Fond::Both => {
            if htf_dir != 0 && htf_dir == ema_dir {
                htf_dir
            } else {
                0
            }
        }
        Fond::Off => dir_raw,
    };
    let dir = if s.fond == Fond::Off { dir_raw } else { fond_dir };

    // POI le plus proche (sous le prix pour un long, au-dessus pour un short)
    let near = |arr: &[Zone], below: bool| {
        let mut best: Option<Zone> = None;
        for z in arr {
            if below && z.bot < close && best.is_none_or(|b| z.top > b.top) {
                best = Some(*z);
            }
            if !below && z.top > close && best.is_none_or(|b| z.bot < b.bot) {
                best = Some(*z);
            }
        }
        best
    };
    let pick = |list: [(Option<Zone>, ZoneKind); 4], long: bool| {
        list.into_iter().fold(None, |a: Option<Poi>, (zz, kind)| match (zz, a) {
            (None, _) => a,
            (Some(zone), None) => Some(Poi { zone, kind }),
            (Some(zone), Some(p)) => {
                if (long && zone.top > p.zone.top) || (!long && zone.bot < p.zone.bot) {
                    Some(Poi { zone, kind })
                } else {
                    a
                }
            }
        })
    };
    let poi = match dir {
        1 => pick(
            [
                (near(&z.bull_ob, true), ZoneKind::Ob),
                (near(&z.bull_fvg, true), ZoneKind::Fvg),
                (near(&z.bull_bb, true), ZoneKind::Bb),
                (near(&z.bull_ifvg, true), ZoneKind::Ifvg),
            ],
            true,
        ),
        -1 => pick(
            [
                (near(&z.bear_ob, false), ZoneKind::Ob),
                (near(&z.bear_fvg, false), ZoneKind::Fvg),
                (near(&z.bear_bb, false), ZoneKind::Bb),
                (near(&z.bear_ifvg, false), ZoneKind::Ifvg),
            ],
            false,
        ),
        _ => None,
    };
    let (poi_top, poi_bot) = poi.map_or((f64::NAN, f64::NAN), |p| (p.zone.top, p.zone.bot));
    let a = atr[last];

    // 📐 Fibonacci : impulsion dans une fenêtre de N bougies (le départ peut remonter jusqu'à 2 × N avant)
    let impulse = |nn: usize| -> Option<Leg> {
        let nn = nn.min(n).max(10);
        let s0 = n.saturating_sub(nn);
        match dir {
            -1 => {
                let mut lb = s0;
                for i in s0..=last {
                    if lo[i] < lo[lb] {
                        lb = i;
                    }
                }
                let b0 = lb.saturating_sub(2 * nn);
                let mut hb = b0;
                for i in b0..=lb {
                    if h[i] > h[hb] {
                        hb = i;
                    }
                }
                (lb > hb).then(|| Leg {
                    hi: h[hb],
                    lo: lo[lb],
                    i0: hb,
                    i1: lb,
                })
            }
            1 => {
                let mut hb = s0;
                for i in s0..=last {
                    if h[i] > h[hb] {
                        hb = i;
                    }
                }
                let b0 = hb.saturating_sub(2 * nn);
                let mut lb = b0;
                for i in b0..=hb {
                    if lo[i] < lo[lb] {
                        lb = i;
                    }
                }
                (hb > lb).then(|| Leg {
                    lo: lo[lb],
                    hi: h[hb],
                    i0: lb,
                    i1: hb,
                })
            }
            _ => None,
        }
    };
    // 🔴 Fibo 1 = dernier high → dernier low (pivots de structure)
    let (mut piv_h, mut piv_l) = (vec![], vec![]);
    if last >= 2 * l {
        for j in l..=last - l {
            let (ph, pl) = strict_pivot(bars, j, l, j + l);
            if ph {
                piv_h.push(j);
            }
            if pl {
                piv_l.push(j);
            }
        }
    }
    let last_leg = || -> Option<Leg> {
        let w = s.fib1_len.min(n).max(10);
        let s0 = n.saturating_sub(w);
        match dir {
            -1 => {
                let mut lb = s0;
                for i in s0..=last {
                    if lo[i] < lo[lb] {
                        lb = i;
                    }
                }
                let hb = match piv_h.iter().rev().find(|&&j| j < lb) {
                    Some(&j) => j,
                    None => {
                        let start = lb.saturating_sub(2 * w);
                        let mut hb = start;
                        for i in start..lb {
                            if h[i] > h[hb] {
                                hb = i;
                            }
                        }
                        hb
                    }
                };
                (hb < lb).then(|| Leg {
                    hi: h[hb],
                    lo: lo[lb],
                    i0: hb,
                    i1: lb,
                })
            }
            1 => {
                let mut hb = s0;
                for i in s0..=last {
                    if h[i] > h[hb] {
                        hb = i;
                    }
                }
                let lb = match piv_l.iter().rev().find(|&&j| j < hb) {
                    Some(&j) => j,
                    None => {
                        let start = hb.saturating_sub(2 * w);
                        let mut lb = start;
                        for i in start..hb {
                            if lo[i] < lo[lb] {
                                lb = i;
                            }
                        }
                        lb
                    }
                };
                (lb < hb).then(|| Leg {
                    lo: lo[lb],
                    hi: h[hb],
                    i0: lb,
                    i1: hb,
                })
            }
            _ => None,
        }
    };
    let mut leg = last_leg().or_else(|| impulse(s.fib1_len));
    if leg.is_none_or(|g| g.hi - g.lo < a * 0.5) {
        if let Some(w) = impulse(s.fib1_len * 2).filter(|w| w.hi - w.lo >= a * 2.0) {
            leg = Some(w);
        }
    }
    if leg.is_none() {
        if let (1, Some(sb)) = (dir, sw_lb) {
            let (mut hi, mut hb) = (f64::NEG_INFINITY, sb);
            for (i, &v) in h.iter().enumerate().skip(sb) {
                if v > hi {
                    hi = v;
                    hb = i;
                }
            }
            if hi > sw_l {
                leg = Some(Leg { lo: sw_l, hi, i0: sb, i1: hb });
            }
        } else if let (-1, Some(sb)) = (dir, sw_hb) {
            let (mut lw, mut lb) = (f64::INFINITY, sb);
            for (i, &v) in lo.iter().enumerate().skip(sb) {
                if v < lw {
                    lw = v;
                    lb = i;
                }
            }
            if lw < sw_h {
                leg = Some(Leg {
                    lo: lw,
                    hi: sw_h,
                    i0: sb,
                    i1: lb,
                });
            }
        }
    }
    let fib = |g: Option<Leg>, r: f64| g.map_or(f64::NAN, |g| if dir == 1 { g.hi - (g.hi - g.lo) * r } else { g.lo + (g.hi - g.lo) * r });
    let (ote_a, ote_b) = (fib(leg, 0.618), fib(leg, if s.ote886 { 0.886 } else { 0.786 }));
    let (ote_top, ote_bot) = if leg.is_some() {
        (ote_a.max(ote_b), ote_a.min(ote_b))
    } else {
        (f64::NAN, f64::NAN)
    };
    let in_ote = leg.is_some() && close >= ote_bot - a * 0.2 && close <= ote_top + a * 0.2;

    // ⭐ confluences POI ∩ OTE
    let zones_dir: Vec<(&Vec<Zone>, ZoneKind)> = match dir {
        1 => vec![
            (&z.bull_ob, ZoneKind::Ob),
            (&z.bull_fvg, ZoneKind::Fvg),
            (&z.bull_bb, ZoneKind::Bb),
            (&z.bull_ifvg, ZoneKind::Ifvg),
        ],
        -1 => vec![
            (&z.bear_ob, ZoneKind::Ob),
            (&z.bear_fvg, ZoneKind::Fvg),
            (&z.bear_bb, ZoneKind::Bb),
            (&z.bear_ifvg, ZoneKind::Ifvg),
        ],
        _ => vec![],
    };
    let mut confluences = vec![];
    if leg.is_some() {
        for (arr, kind) in zones_dir {
            for zz in arr {
                let (t, b) = (zz.top.min(ote_top), zz.bot.max(ote_bot));
                if t > b {
                    confluences.push(Confluence {
                        top: t,
                        bot: b,
                        kind,
                        left: zz.left,
                    });
                }
            }
        }
    }
    if dir == 1 {
        confluences.sort_by(|x, y| y.top.total_cmp(&x.top));
    } else {
        confluences.sort_by(|x, y| x.bot.total_cmp(&y.bot));
    }
    let best_c = confluences.first().copied();
    let in_conf = best_c.is_some_and(|b| close >= b.bot - a * 0.3 && close <= b.top + a * 0.3);

    // ✋ confirmation : entrer dans l'OTE, puis bougie de rejet qui casse le mini-sommet (long) / mini-creux (short) ; invalidé au-delà de 100 %
    let mut cf = Confirmation {
        state: ConfState::None,
        touch: None,
        bar: None,
        ext: f64::NAN,
    };
    if let Some(g) = leg.filter(|_| dir != 0) {
        let lim = if dir == 1 { g.lo } else { g.hi };
        let nn = s.conf_lb.max(1);
        for i in g.i1 + 1..=last {
            let in_z = lo[i] <= ote_top && h[i] >= ote_bot;
            let touch = match cf.touch {
                Some(t) => t,
                None if in_z => {
                    cf.touch = Some(i);
                    cf.ext = if dir == 1 { lo[i] } else { h[i] };
                    i
                }
                None => continue,
            };
            cf.ext = if dir == 1 { cf.ext.min(lo[i]) } else { cf.ext.max(h[i]) };
            if (dir == 1 && c[i] < lim) || (dir == -1 && c[i] > lim) {
                cf.state = ConfState::Invalid;
                cf.bar = Some(i);
                break;
            }
            if i > touch || nn == 1 {
                let mut r = if dir == 1 { f64::NEG_INFINITY } else { f64::INFINITY };
                for kk in i.saturating_sub(nn).max(touch)..i {
                    r = if dir == 1 { r.max(h[kk]) } else { r.min(lo[kk]) };
                }
                let rej = if dir == 1 { c[i] > o[i] && c[i] > r } else { c[i] < o[i] && c[i] < r };
                if r.is_finite() && rej {
                    cf.state = ConfState::Ok;
                    cf.bar = Some(i);
                    break;
                }
            }
        }
        if cf.state == ConfState::None && cf.touch.is_some() {
            cf.state = ConfState::Zone;
        }
    }
    let plan_ok = !s.wait_conf || cf.state == ConfState::Ok;

    // 🔵 Fibo 2 = l'avant-dernière impulsion (finit avant le départ du Fibo 1)
    let impulse_before = |end: usize, nn: usize| -> Option<Leg> {
        if end < 5 {
            return None;
        }
        let s0 = end.saturating_sub(nn.max(10));
        match dir {
            -1 => {
                let mut lb = s0;
                for i in s0..end {
                    if lo[i] < lo[lb] {
                        lb = i;
                    }
                }
                let mut hb = s0;
                for i in s0..=lb {
                    if h[i] > h[hb] {
                        hb = i;
                    }
                }
                (lb > hb).then(|| Leg {
                    hi: h[hb],
                    lo: lo[lb],
                    i0: hb,
                    i1: lb,
                })
            }
            1 => {
                let mut hb = s0;
                for i in s0..end {
                    if h[i] > h[hb] {
                        hb = i;
                    }
                }
                let mut lb = s0;
                for i in s0..=hb {
                    if lo[i] < lo[lb] {
                        lb = i;
                    }
                }
                (hb > lb).then(|| Leg {
                    lo: lo[lb],
                    hi: h[hb],
                    i0: lb,
                    i1: hb,
                })
            }
            _ => None,
        }
    };
    let leg2 = leg
        .filter(|_| dir != 0)
        .and_then(|g| impulse_before(g.i0, s.fib2_len))
        .filter(|g2| g2.hi - g2.lo >= a * 1.5)
        .filter(|g2| !leg.is_some_and(|g| (g2.hi - g.hi).abs() < a * 0.3 && (g2.lo - g.lo).abs() < a * 0.3));
    let (o2a, o2b) = (fib(leg2, 0.618), fib(leg2, 0.886));
    let (ote2_top, ote2_bot) = if leg2.is_some() { (o2a.max(o2b), o2a.min(o2b)) } else { (f64::NAN, f64::NAN) };

    // PDH / PDL et PWH / PWL
    let prev = |sec: i64| -> Option<PrevRange> {
        if sec <= chart_seconds {
            return None;
        }
        let hb = aggregate(bars, sec);
        (hb.len() >= 2).then(|| {
            let p = hb[hb.len() - 2];
            PrevRange {
                hi: p.high,
                lo: p.low,
                time: p.time,
            }
        })
    };
    let (prev_day, prev_week) = (prev(86_400), prev(604_800));

    // 🎯 plan
    let (mut entry, mut sl, mut tp1, mut tp2) = (f64::NAN, f64::NAN, f64::NAN, f64::NAN);
    if dir == 1 {
        entry = match best_c {
            Some(b) => close.min(b.top),
            None if poi_top.is_finite() => close.min(poi_top),
            None => close,
        };
        sl = match best_c {
            Some(b) => b.bot.min(if poi_bot.is_finite() { poi_bot } else { b.bot }),
            None if poi_bot.is_finite() => poi_bot,
            None => sw_l,
        } - a * s.sl_buf_atr;
        tp1 = if sw_h.is_finite() && sw_h > entry { sw_h } else { range_top };
        tp2 = if range_top > tp1 { range_top } else { tp1 + (tp1 - entry) };
    } else if dir == -1 {
        entry = match best_c {
            Some(b) => close.max(b.bot),
            None if poi_bot.is_finite() => close.max(poi_bot),
            None => close,
        };
        sl = match best_c {
            Some(b) => b.top.max(if poi_top.is_finite() { poi_top } else { b.top }),
            None if poi_top.is_finite() => poi_top,
            None => sw_h,
        } + a * s.sl_buf_atr;
        tp1 = if sw_l.is_finite() && sw_l < entry { sw_l } else { range_bot };
        tp2 = if range_bot < tp1 { range_bot } else { tp1 - (entry - tp1) };
    }
    if let (true, Some(bar)) = (s.wait_conf && dir != 0 && cf.state == ConfState::Ok, cf.bar) {
        entry = c[bar];
        sl = if dir == 1 { cf.ext - a * s.sl_buf_atr } else { cf.ext + a * s.sl_buf_atr };
        if let Some(g) = leg {
            tp1 = if dir == 1 {
                g.hi.max(if tp1.is_finite() { tp1 } else { g.hi })
            } else {
                g.lo.min(if tp1.is_finite() { tp1 } else { g.lo })
            };
            tp2 = if dir == 1 {
                g.hi + (g.hi - g.lo) * 0.272
            } else {
                g.lo - (g.hi - g.lo) * 0.272
            };
        }
    }
    let risk = (entry - sl).abs();
    let rr = if risk > 0.0 { (tp1 - entry).abs() / risk } else { f64::NAN };

    // score /10
    let (in_asia, in_london, in_new_york) = (in_ses[0], in_ses[1], in_ses[2]);
    let kz = in_london || in_new_york;
    let htf_ok = dir != 0 && sign(htf_score) == dir;
    let pd_ok = match dir {
        1 => close < equilibrium,
        -1 => close > equilibrium,
        _ => false,
    };
    let sweep_ok = dir != 0 && last_sweep_dir == dir && last_sweep_bar.is_some_and(|b| last - b <= s.sweep_memory);
    let zone_ok = poi.is_some() && close >= poi_bot - a * 0.5 && close <= poi_top + a * 0.5;
    let ltf_ok = dir != 0 && trend == dir;
    let rr_ok = rr.is_finite() && rr >= 2.0;
    let kz_ok = intra && kz;
    let fond_ok = dir != 0 && (s.fond == Fond::Off || fond_dir == dir);
    let raw = [
        (fond_ok, 2),
        (htf_ok, 1),
        (pd_ok, 1),
        (sweep_ok, 2),
        (zone_ok, 2),
        (in_ote, 1),
        (in_conf, 1),
        (ltf_ok, 1),
        (rr_ok, 1),
        (kz_ok, 1),
    ]
    .iter()
    .filter(|x| x.0)
    .map(|x| x.1)
    .sum::<u8>();
    let max_pts = if intra { 13.0 } else { 12.0 };
    let score = if dir == 0 {
        0
    } else {
        (raw as f64 / max_pts * 10.0).round().clamp(0.0, 10.0) as u8
    };
    let missing: Vec<Missing> = [
        (fond_ok, Missing::Fond),
        (in_ote, Missing::Ote),
        (in_conf, Missing::Confluence),
        (htf_ok, Missing::Htf),
        (pd_ok, Missing::PremiumDiscount),
        (sweep_ok, Missing::Sweep),
        (zone_ok, Missing::Poi),
        (ltf_ok, Missing::Choch),
        (rr_ok, Missing::Rr),
        (kz_ok || !intra, Missing::Killzone),
    ]
    .into_iter()
    .filter(|x| !x.0)
    .map(|x| x.1)
    .collect();
    let verdict = if dir == 0 {
        if s.fond != Fond::Off {
            Verdict::NoTrend
        } else {
            Verdict::NoBias
        }
    } else if s.wait_conf && cf.state != ConfState::Ok {
        match cf.state {
            ConfState::Invalid => Verdict::Invalidated,
            ConfState::Zone => Verdict::InZoneWait,
            _ => Verdict::WaitOte,
        }
    } else if score >= 8 {
        Verdict::APlus
    } else if score >= 6 {
        Verdict::Valid
    } else if score >= 4 {
        if s.wait_conf {
            Verdict::WeakConfirmed
        } else {
            Verdict::Weak
        }
    } else {
        Verdict::NoSetup
    };

    // alertes sur la dernière bougie
    let alert = plan_ok && score >= s.score_alert_min;
    let conf_now = s.wait_conf && cf.state == ConfState::Ok && cf.bar == Some(last);
    let buy = (alert && dir == 1) || (conf_now && dir == 1);
    let sell = (alert && dir == -1) || (conf_now && dir != 1);

    let sessions = ss.map(|x| x.out);
    SmcPro {
        zones: z,
        events,
        equal,
        sweeps,
        sessions,
        in_asia,
        in_london,
        in_new_york,
        trend,
        swing_high: sw_h,
        swing_low: sw_l,
        protected_high: prot_h,
        protected_low: prot_l,
        range_top,
        range_bot,
        equilibrium,
        pd_pct,
        mtf,
        htf_score,
        htf_total,
        ema,
        ema_dir,
        htf_dir,
        vote,
        fond_dir,
        dir,
        poi,
        leg,
        ote_top,
        ote_bot,
        in_ote,
        confluences,
        in_conf,
        confirmation: cf,
        leg2,
        ote2_top,
        ote2_bot,
        prev_day,
        prev_week,
        plan_ok,
        entry,
        sl,
        tp1,
        tp2,
        rr,
        raw,
        score,
        missing,
        verdict,
        buy,
        sell,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sessions_et_fuseaux() {
        // 2026-07-15 12:00 UTC = 08:00 à New York (heure d'été), 13:00 à Londres
        let t = 1_784_116_800;
        assert_eq!(minute_of_day(t, chrono_tz::America::New_York), 8 * 60);
        assert_eq!(minute_of_day(t, chrono_tz::Europe::London), 13 * 60);
        // 2026-01-15 12:30 UTC = 07:30 à New York (heure normale)
        assert_eq!(minute_of_day(1_768_480_200, chrono_tz::America::New_York), 7 * 60 + 30);
        let asia = Hours { start: 20.0, end: 0.0 };
        assert!(in_session(21 * 60, asia) && !in_session(0, asia) && !in_session(19 * 60, asia));
        assert!(in_session(7 * 60, Hours { start: 7.0, end: 10.0 }));
    }

    #[test]
    fn zones_ajout_et_bascule() {
        let mut ob = vec![];
        add(&mut ob, Zone { top: 10.0, bot: 9.0, left: 1 }, 2);
        add(&mut ob, Zone { top: 12.0, bot: 11.0, left: 2 }, 2);
        add(&mut ob, Zone { top: 14.0, bot: 13.0, left: 3 }, 2);
        assert_eq!(ob.iter().map(|z| z.left).collect::<Vec<_>>(), vec![3, 2]);
        let mut bb = vec![];
        // clôture sous l'OB 13–14 seulement → il devient un breaker, l'OB 11–12 reste
        let bar = Bar {
            time: 0,
            open: 12.0,
            high: 12.5,
            low: 11.2,
            close: 11.5,
            volume: 1.0,
        };
        flip(&mut ob, true, false, &mut bb, 1, &bar);
        assert_eq!(ob.iter().map(|z| z.left).collect::<Vec<_>>(), vec![2]);
        assert_eq!(bb[0].left, 3);
    }
}
