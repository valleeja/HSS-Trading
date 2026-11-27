//! 🧭 Argos Vector Zone Pro, comme le script Pine de Jee « Argos Vector Zone Pro » (pine/argos-vector-zone-pro.pine) :
//! 4 EMA, bougies vector PVSRA (climax / vector), trendlines auto avec CONFirmation (cassure + vector + rebond sur
//! l'EMA 2), zones d'offre et de demande notées sur 100, patterns auto (triangles, wedges, flags, canaux, range) et
//! canal de Fibonacci sur une UT d'ancrage (ou l'UT supérieure si une figure y domine), patterns chartistes (W, M,
//! triples, ETE), liquidité (BSL / SSL, EQH / EQL) et ses sweeps, tableau Vector Watchlist sur 5 UT, et les 13 alertes.
//! Les couleurs sont de l'affichage : elles restent dans la couche chart. Aide visuelle, pas un conseil.

use crate::inputs::{check, choice, color, int, num, string, Inputs, Param, TF_SECS};
use crate::ta::{atr, ema, pivot, sma};
use argos_core::{aggregate, bucket_end, bucket_start, Bar};

/// Groupes du Pine, dans son ordre.
pub const GROUPS: [&str; 11] = [
    "🧩 Modules",
    "📈 EMA",
    "🔥 Vector Candles (PVSRA)",
    "📐 Trendline auto & Confirmation",
    "🧱 Zones Offre/Demande",
    "🔺 Patterns auto",
    "Ⓜ️ Patterns chartistes",
    "⚓ Ancrage multi-UT (trendlines + Fib)",
    "📏 Canal Fibonacci",
    "💧 Liquidité",
    "📊 Vector Watchlist",
];

/// Unités de temps proposées pour les input.timeframe du Pine (même liste que les autres indicateurs).
pub const TF_NOMS: [&str; 11] = ["5m", "15m", "30m", "1h", "2h", "4h", "5h", "12h", "1D", "2D", "1W"];

/// Les 69 input du Pine, mêmes clés, libellés, défauts et bornes. Les 28 premiers gardent leur place (réglages déjà
/// enregistrés, rangés par position) ; les 41 autres suivent, chacun dans son groupe du Pine.
pub const PARAMS: [Param; 69] = [
    check("showEma", 0, "EMA", true),
    check("showVec", 0, "Vector Candles", true),
    check("showTL", 0, "Trendlines auto + Confirmation", true),
    check("showZones", 0, "Zones Offre/Demande (score)", true),
    check("showLiq", 0, "Liquidité (BSL/SSL, EQH/EQL, sweeps)", true),
    check("showTab", 0, "Tableau Vector Watchlist", true),
    int("len1", 1, "EMA 1", 13.0, 1.0, 500.0),
    int("len2", 1, "EMA 2", 50.0, 1.0, 500.0),
    int("len3", 1, "EMA 3", 100.0, 1.0, 500.0),
    int("len4", 1, "EMA 4", 200.0, 1.0, 500.0),
    int("vLen", 2, "Moyenne volume (barres)", 10.0, 1.0, 500.0),
    num("climaxMult", 2, "Climax (x moyenne)", 2.5, 0.1, 20.0, 0.1),
    num("aboveMult", 2, "Vector (x moyenne)", 1.5, 0.1, 20.0, 0.1),
    color("cBullC", 2, "Climax haussier", 0x00e676),
    color("cBearC", 2, "Climax baissier", 0xf23645),
    color("cBullA", 2, "Vector haussier", 0x2196f3),
    color("cBearA", 2, "Vector baissier", 0xe040fb),
    int("pivLen", 3, "Pivot trendline", 5.0, 2.0, 100.0),
    check("useEmaF", 3, "Exiger un rebond sur l'EMA 2 (50)", true),
    int("emaLook", 3, "Rebond EMA : barres max", 10.0, 1.0, 500.0),
    int("zPivL", 4, "Pivot gauche", 10.0, 2.0, 100.0),
    int("zPivR", 4, "Pivot droite (barres de réaction)", 5.0, 2.0, 100.0),
    num("minReact", 4, "Réaction min. (x ATR)", 1.0, 0.0, 20.0, 0.1),
    int("maxZones", 4, "Zones max par côté", 8.0, 1.0, 50.0),
    check("showWeak", 4, "Afficher les zones faibles", false),
    int("liqPiv", 9, "Pivot swing", 5.0, 2.0, 100.0),
    num("eqTolL", 9, "Equal highs/lows : tolérance (x ATR)", 0.15, 0.0, 5.0, 0.05),
    int("maxLiq", 9, "Niveaux max par côté", 6.0, 1.0, 20.0),
    // ajoutés depuis le Pine (après les 28 premiers : les réglages enregistrés restent lus à la bonne place)
    check("showPat", 0, "Patterns auto (triangle, wedge, flag)", true),
    check("showChart", 0, "Patterns chartistes (W, M, triple, ETE)", true),
    check("showFib", 0, "Canal Fibonacci auto", true),
    int("strongLvl", 4, "Seuil zone forte", 70.0, 0.0, 100.0),
    int("weakLvl", 4, "Seuil zone faible", 40.0, 0.0, 100.0),
    int("pPiv", 5, "Pivot pattern", 5.0, 2.0, 100.0),
    int("patWin", 5, "Fenêtre max (barres)", 450.0, 30.0, 450.0),
    int("patRecent", 5, "Dernière touche max (barres)", 150.0, 5.0, 450.0),
    int("patExc", 5, "Ignorer les N dernières bougies (cassure)", 3.0, 0.0, 20.0),
    num("patSpan", 5, "Priorité structure longue", 0.12, 0.0, 10.0, 0.01),
    num("tolMult", 5, "Tolérance touche (x ATR)", 0.45, 0.0, 10.0, 0.05),
    int("wvMax", 5, "Mèches hors tolérance max", 6.0, 0.0, 20.0),
    color("cRes", 5, "Couleur résistance", 0xffeb3b),
    color("cSup", 5, "Couleur support", 0xff9800),
    int("zzPiv", 6, "Pivot swing", 5.0, 2.0, 100.0),
    num("eqTol", 6, "Sommets égaux : tolérance (x ATR)", 0.6, 0.0, 10.0, 0.1),
    num("minDepth", 6, "Profondeur min. (x ATR)", 1.5, 0.0, 20.0, 0.1),
    color("cBullP", 6, "Couleur pattern haussier", 0x00e676),
    color("cBearP", 6, "Couleur pattern baissier", 0xf23645),
    choice("anchorTF", 7, "UT d'ancrage", 3, &TF_NOMS),
    choice("domTF", 7, "UT supérieure", 5, &TF_NOMS),
    check("useDom", 7, "Prendre la UT sup. si une figure y domine", true),
    int("minT", 7, "Touches min. par ligne (rés./sup./Fib 0%)", 3.0, 2.0, 20.0),
    int("domMinT", 7, "Touches min. pour que la UT sup. domine", 3.0, 2.0, 20.0),
    choice("fibMode", 8, "Direction", 0, &["Auto", "Baissier (sommets)", "Haussier (creux)"]),
    int("fibPiv", 8, "Pivot", 5.0, 2.0, 100.0),
    int("fibGap", 8, "Écart min. entre les 2 pivots (barres)", 10.0, 3.0, 450.0),
    string("fibLevels", 8, "Niveaux", FIB_NIVEAUX),
    int("fibExt", 8, "Prolonger à droite (barres)", 15.0, 0.0, 200.0),
    color("cFib", 8, "Couleur", 0x00e676),
    check("fibGP", 8, "Golden pocket 0.5–0.618 en surbrillance", true),
    num("fibWick", 8, "Niveau des mèches opposées", 0.5, -10.0, 10.0, 0.05),
    int("fibBack", 8, "Reculer le 1er top de N pivots", 0.0, 0.0, 15.0),
    int("reBars", 9, "Réintégration max (barres)", 2.0, 1.0, 10.0),
    color("cBSL", 9, "Couleur BSL (au-dessus)", 0xf23645),
    color("cSSL", 9, "Couleur SSL (en dessous)", 0x00e676),
    choice("tf1", 10, "TF 1", 1, &TF_NOMS),
    choice("tf2", 10, "TF 2", 3, &TF_NOMS),
    choice("tf3", 10, "TF 3", 5, &TF_NOMS),
    choice("tf4", 10, "TF 4", 8, &TF_NOMS),
    choice("tf5", 10, "TF 5", 10, &TF_NOMS),
];

/// Niveaux du canal de Fibonacci par défaut (input « Niveaux » du Pine).
pub const FIB_NIVEAUX: &str = "-1.618,-1.236,-0.618,-0.5,0,0.236,0.382,0.5,0.618,0.786,1,1.618,1.786";

/// Niveaux lus dans le texte de l'input (« -1.618,0,0.5… ») ; un morceau illisible est ignoré (comme `str.tonumber`).
pub fn fib_niveaux(txt: &str) -> Vec<f64> {
    txt.split(',').filter_map(|t| t.trim().parse::<f64>().ok()).collect()
}

/// Secondes de l'UT choisie dans un input timeframe (index dans `TF_NOMS`).
pub fn tf_secs(k: usize) -> i64 {
    TF_SECS[k.min(TF_SECS.len() - 1)]
}

/// Nom court d'une UT du tableau (« 15m », « 1H », « 4H », « D », « W » comme le Pine).
pub fn tf_nom(sec: i64) -> String {
    match sec {
        s if s % 604_800 == 0 => {
            if s == 604_800 {
                "W".into()
            } else {
                format!("{}W", s / 604_800)
            }
        }
        s if s % 86_400 == 0 => {
            if s == 86_400 {
                "D".into()
            } else {
                format!("{}D", s / 86_400)
            }
        }
        s if s % 3_600 == 0 => format!("{}H", s / 3_600),
        s => format!("{}m", s / 60),
    }
}

/// Bougie vector : climax (très gros volume) ou vector (gros volume), haussière ou baissière.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VecKind {
    Climax,
    Vector,
}

/// Une zone d'offre (au-dessus) ou de demande (en dessous).
#[derive(Debug, Clone, PartialEq)]
pub struct Zone {
    pub demand: bool,
    pub top: f64,
    pub bot: f64,
    /// Bougie du pivot (début de la zone à gauche).
    pub born: usize,
    /// Note sur 100 : réaction (40), volume vector à la naissance (30), fraîcheur (30 : pas encore touchée).
    pub score: u32,
    pub touches: u32,
    /// Une bougie vector est à l'origine de la zone.
    pub vector: bool,
}

/// Un niveau de liquidité (sommet = BSL au-dessus, creux = SSL en dessous) ; `eq` : sommets / creux égaux.
#[derive(Debug, Clone, PartialEq)]
pub struct Liq {
    pub high: bool,
    pub price: f64,
    pub from: usize,
    /// Bougie où il a été pris (sweep) ou cassé ; `None` : encore là.
    pub end: Option<usize>,
    pub eq: bool,
    /// Fini par un sweep (pris puis réintégré) ; sinon cassé (ligne grise).
    pub swept: bool,
}

/// Un signal sur une bougie (étiquette + alerte).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signal {
    /// Cassure de trendline + vector + rebond sur l'EMA : CONF long / short.
    ConfLong,
    ConfShort,
    /// Liquidité prise puis réintégrée : sweep des creux (SSL) / des sommets (BSL) ; `vec` : avec une vector.
    SweepSsl {
        vec: bool,
    },
    SweepBsl {
        vec: bool,
    },
    /// Zone forte touchée.
    StrongDemand,
    StrongSupply,
    /// Cassure de la résistance / du support du pattern auto (dernière bougie, comme le Pine).
    PatBreakUp,
    PatBreakDn,
    /// Neckline du pattern chartiste cassée à la hausse / à la baisse (dernière bougie).
    NeckUp,
    NeckDn,
}

/// Ligne du tableau Vector Watchlist : direction, statut, volume en % de la moyenne, secondes avant la clôture.
#[derive(Debug, Clone, PartialEq)]
pub struct TfRow {
    pub name: String,
    /// `None` : l'UT ne peut pas être calculée ici (plus petite que la chart, ou pas assez de bougies) → « — ».
    pub cell: Option<(bool, Option<VecKind>, f64, i64)>,
}

/// Ligne en temps (heures en secondes) : du point 1 au point 2, prolongée ; `touches` : pivots qui la touchent.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ligne {
    pub t1: i64,
    pub p1: f64,
    pub t2: i64,
    pub p2: f64,
    pub touches: usize,
}

impl Ligne {
    /// Prix de la ligne à l'heure `t` (`f_at` du Pine).
    pub fn at(&self, t: i64) -> f64 {
        if self.t2 != self.t1 {
            self.p1 + (self.p2 - self.p1) * (t - self.t1) as f64 / (self.t2 - self.t1) as f64
        } else {
            self.p1
        }
    }
}

/// Noms des patterns auto (`patNames` du Pine ; 0 : aucun).
pub const PAT_NOMS: [&str; 11] = [
    "",
    "Triangle symétrique (compression)",
    "Triangle descendant",
    "Triangle ascendant",
    "Falling wedge",
    "Bull flag",
    "Canal descendant",
    "Rising wedge",
    "Bear flag",
    "Canal ascendant",
    "Range",
];

/// Pattern auto (résistance + support) trouvé sur l'UT d'ancrage ou l'UT supérieure.
#[derive(Debug, Clone, PartialEq)]
pub struct PatAuto {
    pub res: Option<Ligne>,
    pub sup: Option<Ligne>,
    /// Index dans `PAT_NOMS` (0 : pas de figure nommée).
    pub kind: usize,
    /// 1 haussier, −1 baissier, 0 neutre.
    pub bias: i8,
    /// UT d'où vient la figure (secondes) et si c'est l'UT supérieure qui domine.
    pub tf: i64,
    pub dom: bool,
}

/// Canal de Fibonacci : ligne de base (sommets en baissier, creux en haussier) et écart `d` (prix au niveau 1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FibChan {
    pub base: Ligne,
    pub d: f64,
}

/// Pattern chartiste (W, M, triples, ETE) sur l'UT de la chart.
#[derive(Debug, Clone, PartialEq)]
pub struct ChartPat {
    pub name: &'static str,
    /// 1 haussier, −1 baissier.
    pub bias: i8,
    /// Les sommets / creux de la figure (bougie, prix).
    pub pts: Vec<(usize, f64)>,
    /// Neckline : (bougie 1, prix 1) et sa valeur à la dernière bougie.
    pub neck: (usize, f64),
    pub neck_now: f64,
    /// Valeur de la neckline à l'avant-dernière bougie (cassure).
    pub neck_prev: f64,
    pub target: f64,
    pub confirmed: bool,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct VectorZone {
    pub emas: [Vec<f64>; 4],
    /// Statut vector et sens de chaque bougie.
    pub vec: Vec<Option<VecKind>>,
    pub bull: Vec<bool>,
    /// Trendlines en cours : (bougie 1, prix 1, bougie 2, prix 2), descendante (sommets) et montante (creux).
    pub tl_down: Option<(usize, f64, usize, f64)>,
    pub tl_up: Option<(usize, f64, usize, f64)>,
    pub zones: Vec<Zone>,
    pub liqs: Vec<Liq>,
    pub signals: Vec<(usize, Signal)>,
    pub table: Vec<TfRow>,
    pub pat: Option<PatAuto>,
    pub chart: Option<ChartPat>,
    pub fib: Option<FibChan>,
    /// Seuils de note (inputs « Seuil zone forte / faible »).
    pub strong: u32,
    pub weak: u32,
}

/// Statut vector de chaque bougie (`f_vec` du Pine : moyenne des `len` bougies d'avant ; climax aussi si volume ×
/// range est le plus grand des `len` dernières) et volume en % de la moyenne.
pub fn vectors(bars: &[Bar], len: usize, cm: f64, am: f64) -> (Vec<Option<VecKind>>, Vec<f64>) {
    let len = len.max(1);
    let vol: Vec<f64> = bars.iter().map(|b| b.volume).collect();
    let avg = sma(&vol, len);
    let sv: Vec<f64> = bars.iter().map(|b| b.volume * (b.high - b.low)).collect();
    let mut out = vec![None; bars.len()];
    let mut pct = vec![0.0; bars.len()];
    for i in 0..bars.len() {
        let a = if i > 0 { avg[i - 1] } else { f64::NAN };
        // ta.highest : rien tant qu'il n'y a pas `len` bougies
        let hs = if i + 1 >= len {
            sv[i + 1 - len..=i].iter().copied().fold(f64::MIN, f64::max)
        } else {
            f64::NAN
        };
        let v = vol[i];
        if a > 0.0 {
            pct[i] = v / a * 100.0;
        }
        let climax = (a.is_finite() && v >= a * cm) || (hs.is_finite() && sv[i] >= hs);
        out[i] = if climax {
            Some(VecKind::Climax)
        } else if a.is_finite() && v >= a * am {
            Some(VecKind::Vector)
        } else {
            None
        };
    }
    (out, pct)
}

/// Structure d'une UT (`f_struct` du Pine, à sa dernière bougie) : meilleure résistance et meilleur support, pattern,
/// biais et canal de Fibonacci.
#[derive(Debug, Clone, Default)]
struct Structure {
    res: Option<Ligne>,
    sup: Option<Ligne>,
    pat: usize,
    bias: i8,
    fib: Option<FibChan>,
}

/// Réglages des patterns auto / Fibonacci lus une fois.
struct PatSet {
    piv: usize,
    win: usize,
    recent: usize,
    exc: usize,
    span: f64,
    tol_mult: f64,
    wv_max: usize,
    min_t: usize,
    fib_mode: usize,
    fib_piv: usize,
    fib_gap: usize,
    fib_wick: f64,
    fib_back: usize,
}

/// Pivots (bougie, prix, heure), les 20 derniers (`f_push`).
fn pivots20(src: &[f64], b: &[Bar], piv: usize, high: bool) -> Vec<(usize, f64, i64)> {
    let mut v: Vec<(usize, f64, i64)> = vec![];
    for i in 0..src.len() {
        if let Some(j) = pivot(src, i, piv, piv, high) {
            v.push((j, src[j], b[j].time));
            if v.len() > 20 {
                v.remove(0);
            }
        }
    }
    v
}

/// `f_bestArr` : la meilleure ligne entre deux pivots (corps sous / au-dessus, mèches hors tolérance limitées),
/// notée par ses touches, sa longueur et ses mèches ; (a, b, touches).
fn best_line(b: &[Bar], pv: &[(usize, f64, i64)], res: bool, tol: f64, s: &PatSet) -> Option<(usize, usize, usize)> {
    let last = b.len() - 1;
    let n = pv.len();
    let mut best: Option<(f64, usize, usize, usize)> = None;
    for a in 0..n.saturating_sub(1) {
        for bb in a + 1..n {
            let (i1, i2) = (pv[a].0, pv[bb].0);
            if i2 - i1 < 4 || last - i1 > s.win || i1 + s.exc > last {
                continue;
            }
            let sl = (pv[bb].1 - pv[a].1) / (i2 - i1) as f64;
            let (mut ok, mut wv) = (true, 0usize);
            for (k, c) in b.iter().enumerate().take(last - s.exc + 1).skip(i1) {
                let lv = pv[a].1 + sl * (k - i1) as f64;
                if res {
                    if c.open.max(c.close) > lv + tol * 0.5 {
                        ok = false;
                        break;
                    }
                    if c.high > lv + tol {
                        wv += 1;
                    }
                } else {
                    if c.open.min(c.close) < lv - tol * 0.5 {
                        ok = false;
                        break;
                    }
                    if c.low < lv - tol {
                        wv += 1;
                    }
                }
            }
            if !ok || wv > s.wv_max {
                continue;
            }
            let (mut t, mut last_t) = (0usize, None);
            for q in &pv[a..] {
                let lm = pv[a].1 + sl * (q.0 as f64 - i1 as f64);
                if (q.1 - lm).abs() <= tol {
                    t += 1;
                    last_t = Some(q.0);
                }
            }
            if last_t.is_some_and(|lt| last - lt <= s.recent) && t >= s.min_t {
                let sc = t as f64 * 10.0 + (last - i1) as f64 * s.span - wv as f64 * 3.0;
                if best.is_none_or(|x| sc > x.0) {
                    best = Some((sc, a, bb, t));
                }
            }
        }
    }
    best.map(|x| (x.1, x.2, x.3))
}

fn structure(b: &[Bar], s: &PatSet) -> Structure {
    let mut out = Structure::default();
    let n = b.len();
    if n < 2 {
        return out;
    }
    let last = n - 1;
    let a14 = *atr(b, 14).last().unwrap_or(&f64::NAN);
    if !(a14 > 0.0) {
        return out;
    }
    let close: Vec<f64> = b.iter().map(|x| x.close).collect();
    let (hi, lo): (Vec<f64>, Vec<f64>) = b.iter().map(|x| (x.high, x.low)).unzip();
    let tol = s.tol_mult * a14;
    let (h, l) = (pivots20(&hi, b, s.piv, true), pivots20(&lo, b, s.piv, false));
    let ligne = |pv: &[(usize, f64, i64)], a: usize, bb: usize, t: usize| Ligne {
        t1: pv[a].2,
        p1: pv[a].1,
        t2: pv[bb].2,
        p2: pv[bb].1,
        touches: t,
    };
    let r = best_line(b, &h, true, tol, s);
    let sp = best_line(b, &l, false, tol, s);
    out.res = r.map(|(a, bb, t)| ligne(&h, a, bb, t));
    out.sup = sp.map(|(a, bb, t)| ligne(&l, a, bb, t));
    if let (Some((ra, rb, _)), Some((sa, sb, _))) = (r, sp) {
        let rsl = (h[rb].1 - h[ra].1) / (h[rb].0 - h[ra].0) as f64;
        let ssl = (l[sb].1 - l[sa].1) / (l[sb].0 - l[sa].0) as f64;
        let (ri, si) = (h[ra].0, l[sa].0);
        let flat = 0.04 * a14;
        let st = ri.max(si);
        let w0 = (h[ra].1 + rsl * (st as f64 - ri as f64)) - (l[sa].1 + ssl * (st as f64 - si as f64));
        let w1 = (h[ra].1 + rsl * (last as f64 - ri as f64)) - (l[sa].1 + ssl * (last as f64 - si as f64));
        let comp = if w0 > 0.0 { w1 / w0 } else { 1.0 };
        let off = last - st;
        let back = (off + 25).min(499);
        let imp = if back <= last { (close[st] - close[last - back]) / a14 } else { f64::NAN };
        (out.pat, out.bias) = if rsl < -flat && ssl > flat {
            (1, 0)
        } else if rsl < -flat && ssl.abs() <= flat {
            (2, -1)
        } else if rsl.abs() <= flat && ssl > flat {
            (3, 1)
        } else if rsl < -flat && ssl < -flat {
            let k = if comp < 0.75 {
                4
            } else if imp > 4.0 {
                5
            } else {
                6
            };
            (k, if comp < 0.75 || imp > 4.0 { 1 } else { 0 })
        } else if rsl > flat && ssl > flat {
            let k = if comp < 0.75 {
                7
            } else if imp < -4.0 {
                8
            } else {
                9
            };
            (k, if comp < 0.75 || imp < -4.0 { -1 } else { 0 })
        } else {
            (10, 0)
        };
    }
    // 📏 canal de Fibonacci : la ligne de base la plus récente qui touche au moins `min_t` pivots sans être percée
    let em = *ema(&close, 50).last().unwrap_or(&f64::NAN);
    let bear = s.fib_mode == 1 || (s.fib_mode == 0 && close[last] < em);
    let pv = if bear {
        pivots20(&hi, b, s.fib_piv, true)
    } else {
        pivots20(&lo, b, s.fib_piv, false)
    };
    let np = pv.len();
    let (mut ia, mut ib, mut la, mut lb, mut skip): (Option<usize>, usize, Option<usize>, usize, usize) = (None, 0, None, 0, 0);
    if np >= 3 {
        for j in (2..np).rev() {
            if ia.is_some() {
                break;
            }
            for i in (0..=j - 2).rev() {
                if ia.is_some() || pv[j].0 - pv[i].0 < s.fib_gap || last - pv[i].0 > 450 {
                    continue;
                }
                let st = (pv[j].1 - pv[i].1) / (pv[j].0 - pv[i].0) as f64;
                if !((bear && st < 0.0) || (!bear && st > 0.0)) {
                    continue;
                }
                let (mut tc, mut bad) = (0usize, false);
                for (m, q) in pv.iter().enumerate().skip(i) {
                    let dv = q.1 - (pv[i].1 + st * (q.0 as f64 - pv[i].0 as f64));
                    if dv.abs() <= tol {
                        tc += 1;
                    } else if m <= j && ((bear && dv > tol) || (!bear && dv < -tol)) {
                        bad = true;
                    }
                }
                if tc >= s.min_t && !bad {
                    (la, lb) = (Some(i), j);
                    if skip < s.fib_back {
                        skip += 1;
                    } else {
                        (ia, ib) = (Some(i), j);
                    }
                }
            }
        }
    }
    if ia.is_none() {
        (ia, ib) = (la, lb);
    }
    if let Some(ia) = ia {
        let (i1, i2, p1) = (pv[ia].0, pv[ib].0, pv[ia].1);
        let sl = (pv[ib].1 - p1) / (i2 - i1) as f64;
        let mut d = 0.0f64;
        for (k, c) in b.iter().enumerate().take(i2 + 1).skip(i1) {
            let base = p1 + sl * (k - i1) as f64;
            d = if bear { d.min(c.low - base) } else { d.max(c.high - base) };
        }
        if d != 0.0 && s.fib_wick != 0.0 {
            out.fib = Some(FibChan {
                base: Ligne {
                    t1: pv[ia].2,
                    p1,
                    t2: pv[ib].2,
                    p2: pv[ib].1,
                    touches: 0,
                },
                d: d / s.fib_wick,
            });
        }
    }
    out
}

/// Pattern chartiste trouvé : nom, biais, points de la figure, neckline (bougie 1, prix 1, bougie 2, prix 2).
type Trouve = (&'static str, i8, Vec<(usize, f64)>, (usize, f64, usize, f64));

/// Patterns chartistes (W, M, triples, ETE) sur le zigzag des swings (`f_zz`, 12 derniers), à la dernière bougie.
fn chart_pattern(bars: &[Bar], zz_piv: usize, eq_tol: f64, min_depth: f64, atr_now: f64) -> Option<ChartPat> {
    let n = bars.len();
    if n < 2 || !(atr_now > 0.0) {
        return None;
    }
    let (hi, lo): (Vec<f64>, Vec<f64>) = bars.iter().map(|b| (b.high, b.low)).unzip();
    let mut zz: Vec<(usize, f64, i8)> = vec![];
    let push = |zz: &mut Vec<(usize, f64, i8)>, b: usize, p: f64, t: i8| match zz.last_mut() {
        Some(l) if l.2 == t => {
            if (t == 1 && p > l.1) || (t == -1 && p < l.1) {
                (l.0, l.1) = (b, p);
            }
        }
        _ => {
            zz.push((b, p, t));
            if zz.len() > 12 {
                zz.remove(0);
            }
        }
    };
    for i in 0..n {
        if let Some(j) = pivot(&hi, i, zz_piv, zz_piv, true) {
            push(&mut zz, j, hi[j], 1);
        }
        if let Some(j) = pivot(&lo, i, zz_piv, zz_piv, false) {
            push(&mut zz, j, lo[j], -1);
        }
    }
    let (eqt, dep) = (eq_tol * atr_now, min_depth * atr_now);
    let k = zz.len();
    let mut found: Option<Trouve> = None;
    for back in 0..=1usize {
        if found.is_some() || k < 1 + back {
            continue;
        }
        let e = k - 1 - back;
        if e >= 4 {
            let p: Vec<f64> = (0..5).map(|q| zz[e - 4 + q].1).collect();
            let (mx, mn) = (p[0].max(p[2]).max(p[4]), p[0].min(p[2]).min(p[4]));
            let name = if zz[e - 4].2 == 1 {
                if p[2] > p[0] + 0.5 * atr_now && p[2] > p[4] + 0.5 * atr_now && (p[0] - p[4]).abs() <= 2.0 * eqt && p[2] - p[1].max(p[3]) >= dep {
                    Some(("Épaule-Tête-Épaule", -1))
                } else if mx - mn <= eqt && mn - p[1].max(p[3]) >= dep {
                    Some(("Triple top", -1))
                } else {
                    None
                }
            } else if p[2] < p[0] - 0.5 * atr_now && p[2] < p[4] - 0.5 * atr_now && (p[0] - p[4]).abs() <= 2.0 * eqt && p[1].min(p[3]) - p[2] >= dep {
                Some(("ETE inversé", 1))
            } else if mx - mn <= eqt && p[1].min(p[3]) - mx >= dep {
                Some(("Triple bottom", 1))
            } else {
                None
            };
            if let Some((name, bias)) = name {
                let pts = zz[e - 4..=e].iter().map(|z| (z.0, z.1)).collect();
                found = Some((name, bias, pts, (zz[e - 3].0, p[1], zz[e - 1].0, p[3])));
            }
        }
        if found.is_none() && e >= 2 {
            let (p0, p1, p2) = (zz[e - 2].1, zz[e - 1].1, zz[e].1);
            let name = if zz[e - 2].2 == 1 && (p0 - p2).abs() <= eqt && p0.min(p2) - p1 >= dep {
                Some(("Double top (M)", -1))
            } else if zz[e - 2].2 == -1 && (p0 - p2).abs() <= eqt && p1 - p0.max(p2) >= dep {
                Some(("Double bottom (W)", 1))
            } else {
                None
            };
            if let Some((name, bias)) = name {
                let pts = zz[e - 2..=e].iter().map(|z| (z.0, z.1)).collect();
                found = Some((name, bias, pts, (zz[e - 1].0, p1, zz[e].0 + 1, p1)));
            }
        }
    }
    let (name, bias, pts, (nb1, np1, nb2, np2)) = found?;
    let last = n - 1;
    let nsl = (np2 - np1) / (nb2 as f64 - nb1 as f64).max(1.0);
    let neck_now = np1 + nsl * (last as f64 - nb1 as f64);
    let ext = if bias < 0 {
        pts.iter().map(|q: &(usize, f64)| q.1).fold(f64::MIN, f64::max)
    } else {
        pts.iter().map(|q: &(usize, f64)| q.1).fold(f64::MAX, f64::min)
    };
    let mid = pts[pts.len() / 2].0;
    let height = (ext - (np1 + nsl * (mid as f64 - nb1 as f64))).abs();
    let close = bars[last].close;
    Some(ChartPat {
        name,
        bias,
        pts,
        neck: (nb1, np1),
        neck_now,
        neck_prev: np1 + nsl * (last as f64 - 1.0 - nb1 as f64),
        target: if bias < 0 { neck_now - height } else { neck_now + height },
        confirmed: if bias < 0 { close < neck_now } else { close > neck_now },
    })
}

/// Tout l'indicateur. `now` : l'heure (secondes) pour le compte à rebours du tableau ; `tf` : l'UT de la chart.
pub fn compute(bars: &[Bar], tf: i64, now: i64, vals: &[f64]) -> VectorZone {
    let p = Inputs { params: &PARAMS, vals };
    let n = bars.len();
    let close: Vec<f64> = bars.iter().map(|b| b.close).collect();
    let high: Vec<f64> = bars.iter().map(|b| b.high).collect();
    let low: Vec<f64> = bars.iter().map(|b| b.low).collect();
    let emas = [
        ema(&close, p.len("len1")),
        ema(&close, p.len("len2")),
        ema(&close, p.len("len3")),
        ema(&close, p.len("len4")),
    ];
    let (cm, am) = (p.get("climaxMult"), p.get("aboveMult"));
    let (vec, _) = vectors(bars, p.len("vLen"), cm, am);
    let bull: Vec<bool> = bars.iter().map(|b| b.close >= b.open).collect();
    let a14 = atr(bars, 14);
    let (strong, weak) = (p.get("strongLvl").round().max(0.0) as u32, p.get("weakLvl").round().max(0.0) as u32);
    let mut out = VectorZone {
        emas,
        vec: vec.clone(),
        bull: bull.clone(),
        strong,
        weak,
        ..Default::default()
    };
    if n < 3 {
        return out;
    }

    // 📐 trendlines : les deux derniers sommets (descendante si plus bas) et creux (montante si plus haut) ; CONF quand
    // une clôture casse la ligne avec une vector du bon sens, après un rebond sur l'EMA 2
    let pl = p.len("pivLen");
    let (mut ph, mut pll): (Vec<(usize, f64)>, Vec<(usize, f64)>) = (vec![], vec![]);
    let e2 = &out.emas[1];
    let (mut since_low_e2, mut since_high_e2) = (usize::MAX, usize::MAX);
    let look = p.len("emaLook");
    for i in 0..n {
        since_low_e2 = if e2[i].is_finite() && low[i] <= e2[i] {
            0
        } else {
            since_low_e2.saturating_add(1)
        };
        since_high_e2 = if e2[i].is_finite() && high[i] >= e2[i] {
            0
        } else {
            since_high_e2.saturating_add(1)
        };
        if let Some(j) = pivot(&high, i, pl, pl, true) {
            ph.push((j, high[j]));
        }
        if let Some(j) = pivot(&low, i, pl, pl, false) {
            pll.push((j, low[j]));
        }
        let line = |a: (usize, f64), b: (usize, f64), k: usize| a.1 + (b.1 - a.1) / (b.0 as f64 - a.0 as f64) * (k as f64 - a.0 as f64);
        if i > 0 && p.on("showTL") {
            let isv = vec[i].is_some();
            if let [.., a, b] = ph[..] {
                if b.1 < a.1 {
                    let (l0, l1) = (line(a, b, i - 1), line(a, b, i));
                    let bounce = !p.on("useEmaF") || (since_low_e2 <= look && close[i] > e2[i]);
                    if close[i - 1] <= l0 && close[i] > l1 && isv && bull[i] && bounce {
                        out.signals.push((i, Signal::ConfLong));
                    }
                }
            }
            if let [.., a, b] = pll[..] {
                if b.1 > a.1 {
                    let (l0, l1) = (line(a, b, i - 1), line(a, b, i));
                    let bounce = !p.on("useEmaF") || (since_high_e2 <= look && close[i] < e2[i]);
                    if close[i - 1] >= l0 && close[i] < l1 && isv && !bull[i] && bounce {
                        out.signals.push((i, Signal::ConfShort));
                    }
                }
            }
        }
    }
    if let [.., a, b] = ph[..] {
        out.tl_down = (b.1 < a.1).then_some((a.0, a.1, b.0, b.1));
    }
    if let [.., a, b] = pll[..] {
        out.tl_up = (b.1 > a.1).then_some((a.0, a.1, b.0, b.1));
    }

    // 🧱 zones : un creux (demande) / sommet (offre) pivot suivi d'une réaction d'au moins `minReact` ATR ; note sur 100
    if p.on("showZones") {
        let (zl, zr) = (p.len("zPivL"), p.len("zPivR"));
        let vlen = p.len("vLen");
        let vol: Vec<f64> = bars.iter().map(|b| b.volume).collect();
        let avg = sma(&vol, vlen);
        let mut zones: Vec<(Zone, f64, u32, bool)> = vec![]; // (zone, réaction en ATR, points de volume, dedans)
        for i in 0..n {
            let a = a14[i];
            // points de volume : la plus forte bougie des `zr` dernières (climax 30, vector 20)
            let vol_pts = || {
                let best = (i.saturating_sub(zr - 1)..=i)
                    .filter(|&j| j > 0 && avg[j - 1] > 0.0)
                    .map(|j| vol[j] / avg[j - 1])
                    .fold(0.0, f64::max);
                if best >= cm {
                    30
                } else if best >= am {
                    20
                } else {
                    0
                }
            };
            if a > 0.0 {
                if let Some(j) = pivot(&low, i, zl, zr, false) {
                    let bot = low[j];
                    let top = bars[j].open.max(bars[j].close).max(bot + 0.25 * a).min(bot + a);
                    let react = ((j + 1..=i).map(|k| high[k]).fold(f64::MIN, f64::max) - bot) / a;
                    if react >= p.get("minReact") {
                        let vp = vol_pts();
                        let z = Zone {
                            demand: true,
                            top,
                            bot,
                            born: j,
                            score: 0,
                            touches: 0,
                            vector: vp > 0,
                        };
                        zones.push((z, react, vp, false));
                    }
                }
                if let Some(j) = pivot(&high, i, zl, zr, true) {
                    let top = high[j];
                    let bot = bars[j].open.min(bars[j].close).min(top - 0.25 * a).max(top - a);
                    let react = (top - (j + 1..=i).map(|k| low[k]).fold(f64::MAX, f64::min)) / a;
                    if react >= p.get("minReact") {
                        let vp = vol_pts();
                        let z = Zone {
                            demand: false,
                            top,
                            bot,
                            born: j,
                            score: 0,
                            touches: 0,
                            vector: vp > 0,
                        };
                        zones.push((z, react, vp, false));
                    }
                }
            }
            // au plus `maxZones` par côté : la plus ancienne part
            for side in [true, false] {
                while zones.iter().filter(|z| z.0.demand == side).count() > p.len("maxZones") {
                    if let Some(k) = zones.iter().position(|z| z.0.demand == side) {
                        zones.remove(k);
                    }
                }
            }
            // cassée (clôture au-delà) : elle disparaît ; touchée : une touche de plus (une seule par passage)
            zones.retain(|(z, ..)| if z.demand { close[i] >= z.bot } else { close[i] <= z.top });
            for (z, react, vp, inside) in zones.iter_mut() {
                let now_in = if z.demand { low[i] <= z.top } else { high[i] >= z.bot };
                if now_in && !*inside && i > z.born + zr {
                    let sc = score(*react, *vp, z.touches);
                    if sc >= strong {
                        out.signals.push((i, if z.demand { Signal::StrongDemand } else { Signal::StrongSupply }));
                    }
                    z.touches += 1;
                }
                *inside = now_in;
            }
        }
        out.zones = zones
            .into_iter()
            .map(|(mut z, react, vp, _)| {
                z.score = score(react, vp, z.touches);
                z
            })
            .filter(|z| p.on("showWeak") || z.score >= weak)
            .collect();
    }

    // 🔺 patterns auto et 📏 canal de Fibonacci : sur l'UT d'ancrage, ou l'UT supérieure si une figure y domine
    // (`request.security` du Pine : la bougie HTF en cours compte, comme à la dernière bougie sur TradingView). Une
    // UT plus petite que la chart se calcule sur les bougies de la chart (seules disponibles ici).
    if p.on("showPat") || p.on("showFib") {
        let s = PatSet {
            piv: p.len("pPiv"),
            win: p.len("patWin"),
            recent: p.len("patRecent"),
            exc: p.get("patExc").round().max(0.0) as usize,
            span: p.get("patSpan"),
            tol_mult: p.get("tolMult"),
            wv_max: p.get("wvMax").round().max(0.0) as usize,
            min_t: p.len("minT"),
            fib_mode: p.pick("fibMode"),
            fib_piv: p.len("fibPiv"),
            fib_gap: p.len("fibGap"),
            fib_wick: p.get("fibWick"),
            fib_back: p.get("fibBack").round().max(0.0) as usize,
        };
        let serie = |sec: i64| if sec > tf && tf > 0 { aggregate(bars, sec) } else { bars.to_vec() };
        let (anchor, dom_tf) = (tf_secs(p.pick("anchorTF")), tf_secs(p.pick("domTF")));
        let a = structure(&serie(anchor), &s);
        let d = structure(&serie(dom_tf), &s);
        let dmin = p.len("domMinT");
        let dom = p.on("useDom") && (1..=9).contains(&d.pat) && d.res.is_some_and(|l| l.touches >= dmin) && d.sup.is_some_and(|l| l.touches >= dmin);
        let x = if dom { d } else { a };
        if p.on("showPat") {
            // cassure de la résistance / du support à la dernière bougie
            let (t0, t1) = (bars[n - 2].time, bars[n - 1].time);
            if x.res.is_some_and(|r| close[n - 1] > r.at(t1) && close[n - 2] <= r.at(t0)) {
                out.signals.push((n - 1, Signal::PatBreakUp));
            }
            if x.sup.is_some_and(|su| close[n - 1] < su.at(t1) && close[n - 2] >= su.at(t0)) {
                out.signals.push((n - 1, Signal::PatBreakDn));
            }
            out.pat = Some(PatAuto {
                res: x.res,
                sup: x.sup,
                kind: x.pat,
                bias: x.bias,
                tf: if dom { dom_tf } else { anchor },
                dom,
            });
        }
        if p.on("showFib") {
            out.fib = x.fib;
        }
    }

    // Ⓜ️ patterns chartistes sur l'UT de la chart
    if p.on("showChart") {
        out.chart = chart_pattern(bars, p.len("zzPiv"), p.get("eqTol"), p.get("minDepth"), a14[n - 1]);
        if let Some(c) = &out.chart {
            if c.bias < 0 && close[n - 1] < c.neck_now && close[n - 2] >= c.neck_prev {
                out.signals.push((n - 1, Signal::NeckDn));
            }
            if c.bias > 0 && close[n - 1] > c.neck_now && close[n - 2] <= c.neck_prev {
                out.signals.push((n - 1, Signal::NeckUp));
            }
        }
    }

    // 💧 liquidité : sommets (BSL) et creux (SSL) pivots ; égaux (EQH / EQL) à `eqTolL` ATR près ; pris puis
    // réintégrés (sweep) dans les `reBars` bougies, sinon cassés
    if p.on("showLiq") {
        let lp = p.len("liqPiv");
        let re = p.len("reBars");
        let mut live: Vec<(Liq, u8, usize)> = vec![]; // (niveau, état 0 / 1 = dépassé, bougie du dépassement)
        let mut done: Vec<Liq> = vec![];
        for i in 0..n {
            let a = a14[i];
            for (hi, piv) in [(true, pivot(&high, i, lp, lp, true)), (false, pivot(&low, i, lp, lp, false))] {
                let (Some(j), true) = (piv, a > 0.0) else { continue };
                let price = if hi { high[j] } else { low[j] };
                if let Some(z) = live
                    .iter_mut()
                    .find(|z| z.0.high == hi && z.1 == 0 && (z.0.price - price).abs() <= p.get("eqTolL") * a)
                {
                    z.0.eq = true;
                } else {
                    live.push((
                        Liq {
                            high: hi,
                            price,
                            from: j,
                            end: None,
                            eq: false,
                            swept: false,
                        },
                        0,
                        0,
                    ));
                    while live.iter().filter(|z| z.0.high == hi).count() > p.len("maxLiq") {
                        if let Some(k) = live.iter().position(|z| z.0.high == hi) {
                            live.remove(k);
                        }
                    }
                }
            }
            let isv = vec[i].is_some();
            let mut k = 0;
            while k < live.len() {
                let (z, state, at) = &mut live[k];
                let (swept, broken) = if z.high {
                    if *state == 0 {
                        if high[i] > z.price && close[i] <= z.price {
                            (true, false)
                        } else {
                            if close[i] > z.price {
                                (*state, *at) = (1, i);
                            }
                            (false, false)
                        }
                    } else if close[i] <= z.price {
                        (true, false)
                    } else {
                        (false, i - *at >= re)
                    }
                } else if *state == 0 {
                    if low[i] < z.price && close[i] >= z.price {
                        (true, false)
                    } else {
                        if close[i] < z.price {
                            (*state, *at) = (1, i);
                        }
                        (false, false)
                    }
                } else if close[i] >= z.price {
                    (true, false)
                } else {
                    (false, i - *at >= re)
                };
                if swept || broken {
                    if swept {
                        out.signals.push((
                            i,
                            if z.high {
                                Signal::SweepBsl { vec: isv && !bull[i] }
                            } else {
                                Signal::SweepSsl { vec: isv && bull[i] }
                            },
                        ));
                    }
                    let mut z = live.remove(k).0;
                    (z.end, z.swept) = (Some(i), swept);
                    done.push(z);
                } else {
                    k += 1;
                }
            }
        }
        done.extend(live.into_iter().map(|z| z.0));
        out.liqs = done;
    }

    // 📊 Vector Watchlist : les 5 UT des inputs, chacune au moins égale à celle de la chart, regroupée depuis ses bougies
    if p.on("showTab") {
        out.table = ["tf1", "tf2", "tf3", "tf4", "tf5"]
            .iter()
            .map(|k| {
                let sec = tf_secs(p.pick(k));
                TfRow {
                    name: tf_nom(sec),
                    cell: (sec >= tf && tf > 0)
                        .then(|| if sec == tf { bars.to_vec() } else { aggregate(bars, sec) })
                        .filter(|b| b.len() > p.len("vLen") + 1)
                        .map(|b| {
                            let (v, pct) = vectors(&b, p.len("vLen"), cm, am);
                            let last = b.len() - 1;
                            let left = bucket_end(bucket_start(b[last].time, sec), sec) - now;
                            (b[last].close >= b[last].open, v[last], pct[last], left.max(0))
                        }),
                }
            })
            .collect();
    }
    out.signals.sort_by_key(|s| s.0);
    out
}

/// 🔔 Les 13 alertes du Pine (noms des `alertcondition`), dans l'ordre de `VectorZone::alerts`.
pub const ALERTS: [&str; 13] = [
    "Confirmation Long",
    "Confirmation Short",
    "Vector Candle",
    "Zone de demande forte touchée",
    "Zone d'offre forte touchée",
    "Cassure pattern + Vector haussière",
    "Cassure pattern + Vector baissière",
    "Neckline cassée - W / Triple bottom / ETE inversé",
    "Neckline cassée - M / Triple top / ETE",
    "Sweep SSL (liquidité basse prise)",
    "Sweep BSL (liquidité haute prise)",
    "Sweep SSL + Vector haussière",
    "Sweep BSL + Vector baissière",
];

impl VectorZone {
    /// 🔔 Signaux des alertes (`ALERTS`) : 1 sur les bougies où le signal arrive, 0 ailleurs.
    pub fn alerts(&self) -> Vec<(&'static str, Vec<f64>)> {
        let n = self.vec.len();
        let mut out: Vec<(&'static str, Vec<f64>)> = ALERTS.iter().map(|a| (*a, vec![0.0; n])).collect();
        let isv = |i: usize| self.vec.get(i).is_some_and(|v| v.is_some());
        let up = |i: usize| self.bull.get(i).copied().unwrap_or(false);
        for i in (0..n).filter(|&i| isv(i)) {
            out[2].1[i] = 1.0;
        }
        for &(i, s) in self.signals.iter().filter(|s| s.0 < n) {
            let mut on = |k: usize| out[k].1[i] = 1.0;
            match s {
                Signal::ConfLong => on(0),
                Signal::ConfShort => on(1),
                Signal::StrongDemand => on(3),
                Signal::StrongSupply => on(4),
                Signal::PatBreakUp if isv(i) && up(i) => on(5),
                Signal::PatBreakDn if isv(i) && !up(i) => on(6),
                Signal::NeckUp => on(7),
                Signal::NeckDn => on(8),
                Signal::SweepSsl { vec } => {
                    on(9);
                    if vec {
                        on(11);
                    }
                }
                Signal::SweepBsl { vec } => {
                    on(10);
                    if vec {
                        on(12);
                    }
                }
                _ => {}
            }
        }
        out
    }
}

/// Note d'une zone : réaction (jusqu'à 3 ATR : 40 points), volume vector (30 / 20), fraîcheur (30 / 20 / 10 / 0).
pub fn score(react: f64, vol_pts: u32, touches: u32) -> u32 {
    let rp = react.min(3.0) / 3.0 * 40.0;
    let fp = match touches {
        0 => 30.0,
        1 => 20.0,
        2 => 10.0,
        _ => 0.0,
    };
    (rp + vol_pts as f64 + fp).round() as u32
}

/// Total du tableau : UT où la dernière bougie est vector ou climax (« x / 5 »).
pub fn table_total(t: &[TfRow]) -> usize {
    t.iter().filter(|r| r.cell.as_ref().is_some_and(|c| c.1.is_some())).count()
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
    fn bougies_vector_pvsra() {
        // volume 100 partout, puis 160 (vector, ≥ 1,5 ×) et 300 (climax, ≥ 2,5 ×)
        // (la bougie 15 a un peu moins de volume : elle n'est pas la plus forte de sa fenêtre)
        let mut b: Vec<Bar> = (0..20)
            .map(|i| bar(i * 60, 10.0, 11.0, 9.0, 10.5, if i == 15 { 90.0 } else { 100.0 }))
            .collect();
        b.push(bar(1200, 10.0, 10.6, 9.9, 10.5, 160.0));
        b.push(bar(1260, 10.5, 10.6, 10.4, 10.45, 300.0));
        let (v, pct) = vectors(&b, 10, 2.5, 1.5);
        assert_eq!(v[20], Some(VecKind::Vector));
        assert_eq!(v[21], Some(VecKind::Climax));
        assert!((pct[20] - 16_000.0 / 99.0).abs() < 1e-9, "160 / moyenne 99 : {}", pct[20]);
        assert_eq!(v[15], None, "volume normal");
    }

    #[test]
    fn note_des_zones() {
        assert_eq!(score(3.0, 30, 0), 100, "réaction forte, climax, jamais touchée");
        assert_eq!(score(1.5, 0, 1), 40);
        assert_eq!(score(10.0, 20, 5), 60, "réaction plafonnée à 3 ATR");
    }

    #[test]
    fn zones_sweeps_et_tableau() {
        // descente, creux à 90 avec gros volume, remontée franche : zone de demande ; puis mèche sous 90 qui
        // réintègre : sweep SSL
        let mut b = vec![];
        let mut t = 1_700_000_000;
        let mut px = 110.0;
        for k in 0..60 {
            let (o, c) = if k < 20 {
                (px, px - 1.0)
            } else if k < 40 {
                (px, px + 1.0)
            } else {
                (px, px - 0.5)
            };
            let v = if k == 20 { 400.0 } else { 100.0 };
            b.push(bar(t, o, o.max(c) + 0.2, o.min(c) - 0.2, c, v));
            px = c;
            t += 3600;
        }
        // mèche sous le creux (≈ 89,8) et clôture au-dessus
        let low_min = b.iter().map(|x| x.low).fold(f64::MAX, f64::min);
        b.push(bar(t, 95.0, 95.5, low_min - 1.0, 95.2, 100.0));
        let out = compute(&b, 3600, t + 1800, &[]);
        assert_eq!(out.emas.len(), 4);
        assert!(
            out.zones.iter().any(|z| z.demand && z.score >= out.weak),
            "zone de demande notée : {:?}",
            out.zones
        );
        assert!(
            out.signals.iter().any(|s| matches!(s.1, Signal::SweepSsl { .. })),
            "sweep SSL : {:?}",
            out.signals
        );
        // tableau : 15m plus petit que la chart (1H) → « — » ; 1H, 4H, D calculés ; W : trop peu de bougies → « — »
        assert_eq!(out.table.len(), 5);
        assert!(out.table[0].cell.is_none(), "15m sous l'UT de la chart");
        assert!(out.table[1].cell.is_some() && out.table[2].cell.is_some());
        assert!(out.table[4].cell.is_none(), "W : pas assez de semaines");
        let (_, _, _, left) = out.table[1].cell.unwrap();
        assert!((0..=3600).contains(&left));
        assert!(table_total(&out.table) <= 5);
    }

    /// Bougies d'une suite de prix (ouverture = clôture d'avant), mèches de 0,1.
    fn suite(px: &[f64], t0: i64, pas: i64) -> Vec<Bar> {
        px.iter()
            .enumerate()
            .map(|(k, &c)| {
                let o = if k > 0 { px[k - 1] } else { c };
                bar(t0 + k as i64 * pas, o, o.max(c) + 0.1, o.min(c) - 0.1, c, 100.0)
            })
            .collect()
    }

    #[test]
    fn reglages_du_pine() {
        assert_eq!(PARAMS.len(), 69, "les 69 input du Pine");
        assert_eq!(GROUPS.len(), 11);
        // les 28 premiers gardent leur place (réglages enregistrés par position)
        let cles: Vec<&str> = PARAMS.iter().map(|p| p.key).collect();
        assert_eq!(&cles[..6], &["showEma", "showVec", "showTL", "showZones", "showLiq", "showTab"]);
        assert_eq!(cles[27], "maxLiq");
        let defauts = crate::inputs::defaults(&PARAMS);
        let p = Inputs {
            params: &PARAMS,
            vals: &defauts,
        };
        assert_eq!((p.get("strongLvl"), p.get("weakLvl"), p.get("reBars")), (70.0, 40.0, 2.0));
        assert_eq!(
            ["tf1", "tf2", "tf3", "tf4", "tf5"].map(|k| tf_nom(tf_secs(p.pick(k)))),
            ["15m", "1H", "4H", "D", "W"]
        );
        assert_eq!(fib_niveaux(FIB_NIVEAUX).len(), 13);
        assert_eq!(fib_niveaux("0, 0.5 ,x,1"), vec![0.0, 0.5, 1.0]);
    }

    #[test]
    fn triangle_descendant_et_alertes() {
        // vagues : sommets de plus en plus bas (130, 126, 122…), creux à 100 : résistance descendante, support plat
        let mut px = vec![];
        for k in 0..8 {
            let top = 130.0 - 4.0 * k as f64;
            px.extend((0..10).map(|j| 100.0 + (top - 100.0) * j as f64 / 10.0));
            px.extend((0..10).map(|j| top - (top - 100.0) * j as f64 / 10.0));
        }
        let b = suite(&px, 1_700_000_000, 3600);
        let out = compute(&b, 3600, 1_700_000_000 + 160 * 3600, &[]);
        let pat = out.pat.clone().expect("pattern auto");
        assert_eq!((PAT_NOMS[pat.kind], pat.bias), ("Triangle descendant", -1), "{pat:?}");
        assert!(pat.res.is_some_and(|r| r.touches >= 3) && pat.sup.is_some_and(|s| s.touches >= 3));
        let al = out.alerts();
        assert_eq!(al.len(), 13);
        assert!(al.iter().all(|(_, v)| v.len() == b.len()));
    }

    #[test]
    fn double_bottom_w() {
        // descente à 100, remontée à 110, nouveau creux à 100,2, remontée à 106 : W en formation, neckline à 110
        let mut px: Vec<f64> = (0..10).map(|j| 110.0 - j as f64).collect();
        px.push(100.0);
        px.extend((1..=10).map(|j| 100.0 + j as f64));
        px.extend((1..=10).map(|j| 110.0 - 0.98 * j as f64));
        px.extend((1..=6).map(|j| 100.2 + j as f64));
        let b = suite(&px, 1_700_000_000, 3600);
        let c = chart_pattern(&b, 5, 0.6, 1.5, *atr(&b, 14).last().unwrap()).expect("W");
        assert_eq!((c.name, c.bias, c.confirmed), ("Double bottom (W)", 1, false), "{c:?}");
        assert!((c.neck_now - 110.1).abs() < 1e-9, "neckline au sommet entre les creux : {}", c.neck_now);
        assert!(c.target > c.neck_now);
    }
}
