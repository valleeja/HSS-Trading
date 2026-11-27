//! 📓 Journal de trading, comme Argos v1 : les trades notés à la main (ou venant de la démo), leurs R et P&L,
//! les statistiques (winrate, espérance, profit factor, drawdown, séries, meilleur / pire, groupes) et les règles
//! de prop firm. Calcul pur, gardé par la fenêtre dans journal.json.

use crate::money::Money;
use crate::portfolio::ClosedTrade;
use serde::{Deserialize, Serialize};

/// Stratégies proposées au départ (comme v1).
pub const STRATS0: [&str; 5] = ["Double structure 4H", "SMC 15 min (OB / FVG)", "Retest trendline", "Breakout", "Scalp"];
/// Confluences de v1.
pub const CONF: [&str; 12] = [
    "RSI",
    "Stoch",
    "Divergence",
    "Momentum",
    "OB",
    "FVG",
    "Trendline",
    "Fibo",
    "Volume vector",
    "EMA",
    "Structure BOS/CHoCH",
    "Liquidité / sweep",
];
/// Émotions de v1 : (clé, étiquette).
pub const EMOS: [(&str, &str); 6] = [
    ("calme", "😌 Calme"),
    ("confiant", "😎 Confiant"),
    ("fomo", "😤 FOMO"),
    ("peur", "😰 Peur"),
    ("revenge", "😡 Revenge"),
    ("fatigue", "🥱 Fatigué"),
];

/// Un trade du journal, **au format de v1** (mêmes clés JSON, temps en millisecondes) : une sauvegarde du journal de
/// localhost s'importe telle quelle.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct JTrade {
    pub id: String,
    pub sym: String,
    /// « long » ou « short ».
    pub side: String,
    pub tf: String,
    pub strat: String,
    #[serde(deserialize_with = "num")]
    pub entry: Option<f64>,
    #[serde(deserialize_with = "num")]
    pub sl: Option<f64>,
    #[serde(deserialize_with = "num")]
    pub tp: Option<f64>,
    /// Sortie (vide = ouvert).
    #[serde(deserialize_with = "num")]
    pub exit: Option<f64>,
    #[serde(deserialize_with = "num")]
    pub risk: Option<f64>,
    #[serde(deserialize_with = "num")]
    pub fees: Option<f64>,
    /// P&L réel (sinon calculé avec le R et le risque).
    #[serde(deserialize_with = "num")]
    pub pnl: Option<f64>,
    /// Note de 1 à 5.
    #[serde(deserialize_with = "num")]
    pub rate: Option<f64>,
    /// Entrée et sortie (millisecondes Unix, comme v1).
    pub entry_ts: i64,
    #[serde(deserialize_with = "num_ts")]
    pub exit_ts: Option<i64>,
    pub conf: Vec<String>,
    pub emo: String,
    pub notes: String,
}

/// Nombre de v1 : un nombre, du texte qui en est un, ou vide (« », null) = rien.
fn num<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<f64>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum V {
        N(f64),
        S(String),
        Other(serde::de::IgnoredAny),
    }
    Ok(match Option::<V>::deserialize(d)? {
        Some(V::N(v)) => Some(v),
        Some(V::S(s)) => s.trim().parse().ok(),
        _ => None,
    }
    .filter(|v: &f64| v.is_finite()))
}

fn num_ts<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<i64>, D::Error> {
    Ok(num(d)?.map(|v| v as i64))
}

impl JTrade {
    pub fn short(&self) -> bool {
        self.side == "short"
    }

    pub fn closed(&self) -> bool {
        self.exit.is_some()
    }

    /// Résultat en R : (sortie − entrée) / |entrée − stop|, dans le sens du trade.
    pub fn r(&self) -> Option<f64> {
        let (e, s, x) = (self.entry?, self.sl?, self.exit?);
        if e == s {
            return None;
        }
        Some((x - e) / (e - s).abs() * if self.short() { -1.0 } else { 1.0 })
    }

    /// R prévu (objectif / stop).
    pub fn plan(&self) -> Option<f64> {
        let (e, s, t) = (self.entry?, self.sl?, self.tp?);
        (e != s).then(|| (t - e).abs() / (e - s).abs())
    }

    /// P&L : le réel s'il est noté, sinon R × risque − frais.
    pub fn pnl(&self) -> Option<f64> {
        if let Some(p) = self.pnl {
            return Some(p);
        }
        Some(self.r()? * self.risk.unwrap_or(0.0) - self.fees.unwrap_or(0.0))
    }

    /// Heure qui compte pour les filtres (sortie, sinon entrée ; ms).
    pub fn ts(&self) -> i64 {
        self.exit_ts.unwrap_or(self.entry_ts)
    }
}

/// Statistiques d'une liste de trades (comme `jStats` de v1).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Stats {
    pub n: usize,
    pub open: usize,
    pub win: Option<f64>,
    pub avg_r: Option<f64>,
    pub total: f64,
    /// Profit factor (infini s'il n'y a que des gains).
    pub pf: Option<f64>,
    pub avg_win: Option<f64>,
    pub avg_loss: Option<f64>,
    pub dd: f64,
    /// Courbe de capital (P&L cumulé, commence à 0).
    pub curve: Vec<f64>,
    pub best_streak: usize,
    pub worst_streak: usize,
    /// Index (dans la liste donnée) du meilleur et du pire trade.
    pub best: Option<usize>,
    pub worst: Option<usize>,
}

pub fn stats(list: &[JTrade]) -> Stats {
    let mut idx: Vec<usize> = (0..list.len()).filter(|&i| list[i].closed()).collect();
    idx.sort_by_key(|&i| list[i].ts());
    let val = |i: usize| list[i].pnl().or(list[i].r()).unwrap_or(0.0);
    let rs: Vec<f64> = idx.iter().filter_map(|&i| list[i].r()).collect();
    let wins: Vec<usize> = idx.iter().copied().filter(|&i| val(i) > 0.0).collect();
    let losses: Vec<usize> = idx.iter().copied().filter(|&i| val(i) < 0.0).collect();
    let pnl = |i: usize| list[i].pnl().unwrap_or(0.0);
    let gw: f64 = wins.iter().map(|&i| pnl(i)).sum();
    let gl: f64 = -losses.iter().map(|&i| pnl(i)).sum::<f64>();
    let (mut eq, mut peak, mut dd, mut cur, mut best_s, mut worst_s) = (0.0f64, 0.0f64, 0.0f64, 0i64, 0i64, 0i64);
    let mut curve = vec![0.0];
    for &i in &idx {
        let v = pnl(i);
        eq += v;
        curve.push(eq);
        peak = peak.max(eq);
        dd = dd.max(peak - eq);
        if v > 0.0 {
            cur = if cur > 0 { cur + 1 } else { 1 };
            best_s = best_s.max(cur);
        } else if v < 0.0 {
            cur = if cur < 0 { cur - 1 } else { -1 };
            worst_s = worst_s.min(cur);
        }
    }
    let n = idx.len();
    let by = |better: fn(f64, f64) -> bool| {
        idx.iter()
            .copied()
            .filter(|&i| list[i].pnl().is_some())
            .fold(None, |m: Option<usize>, i| match m {
                Some(j) if !better(pnl(i), pnl(j)) => Some(j),
                _ => Some(i),
            })
    };
    Stats {
        n,
        open: list.len() - n,
        win: (n > 0).then(|| wins.len() as f64 / n as f64 * 100.0),
        avg_r: (!rs.is_empty()).then(|| rs.iter().sum::<f64>() / rs.len() as f64),
        total: idx.iter().filter_map(|&i| list[i].pnl()).sum(),
        pf: if gl > 0.0 {
            Some(gw / gl)
        } else if gw > 0.0 {
            Some(f64::INFINITY)
        } else {
            None
        },
        avg_win: (!wins.is_empty()).then(|| gw / wins.len() as f64),
        avg_loss: (!losses.is_empty()).then(|| -gl / losses.len() as f64),
        dd,
        curve,
        best_streak: best_s as usize,
        worst_streak: (-worst_s) as usize,
        best: by(|a, b| a > b),
        worst: by(|a, b| a < b),
    }
}

/// Une ligne de tableau par groupe (stratégie, session, symbole, émotion, jour), triée par P&L.
#[derive(Debug, Clone, PartialEq)]
pub struct Group {
    pub key: String,
    pub n: usize,
    pub win: Option<f64>,
    pub avg_r: Option<f64>,
    pub total: f64,
}

pub fn group_by(list: &[JTrade], key: impl Fn(&JTrade) -> String) -> Vec<Group> {
    let mut keys: Vec<String> = vec![];
    for t in list.iter().filter(|t| t.closed()) {
        let k = key(t);
        let k = if k.is_empty() { "—".to_string() } else { k };
        if !keys.contains(&k) {
            keys.push(k);
        }
    }
    let mut out: Vec<Group> = keys
        .into_iter()
        .map(|k| {
            let sub: Vec<JTrade> = list
                .iter()
                .filter(|t| {
                    t.closed() && {
                        let kk = key(t);
                        (if kk.is_empty() { "—".to_string() } else { kk }) == k
                    }
                })
                .cloned()
                .collect();
            let s = stats(&sub);
            Group {
                key: k,
                n: s.n,
                win: s.win,
                avg_r: s.avg_r,
                total: s.total,
            }
        })
        .collect();
    out.sort_by(|a, b| b.total.total_cmp(&a.total));
    out
}

/// 🔓 Un déverrouillage de la journée par le Gardien : il faut une raison, gardée ici (dans le journal chiffré).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Deverrouillage {
    /// Le jour déverrouillé (« AAAA-MM-JJ » dans le fuseau du reset).
    pub jour: String,
    /// L'heure du déverrouillage (ms UTC).
    pub ms: i64,
    /// La raison donnée.
    pub raison: String,
}

/// 🔓 Unlocks kept for the discipline review (the oldest go first).
pub const DEVERROUILLAGES_MAX: usize = 500;

/// 🔓 One week of the discipline review: its Monday (« AAAA-MM-JJ ») and the reasons of its unlocks, in order.
#[derive(Debug, Clone, PartialEq)]
pub struct SemaineDiscipline {
    pub lundi: String,
    pub raisons: Vec<String>,
}

/// Days since 1 January 1970 of a « AAAA-MM-JJ » day (Howard Hinnant's algorithm); `None` if it is not a date.
fn jours_de(jour: &str) -> Option<i64> {
    let mut it = jour.splitn(3, '-').map(|p| p.parse::<i64>().ok());
    let (y, m, d) = (it.next()??, it.next()??, it.next()??);
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    let y = y - i64::from(m <= 2);
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * ((m + 9) % 12) + 2) / 5 + d - 1;
    Some(era * 146_097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719_468)
}

/// « AAAA-MM-JJ » of a day count since 1 January 1970 (Howard Hinnant's algorithm).
fn jour_txt(jours: i64) -> String {
    let z = jours + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    format!("{:04}-{m:02}-{d:02}", yoe + era * 400 + i64::from(m <= 2))
}

/// 🔓 Discipline review: the unlocks of the guardian grouped by week (Monday to Sunday, by the day unlocked), the
/// most recent week first; an unlock without a readable day is left out.
pub fn bilan_discipline(liste: &[Deverrouillage]) -> Vec<SemaineDiscipline> {
    let mut out: Vec<SemaineDiscipline> = vec![];
    let mut tries: Vec<(i64, &Deverrouillage)> = liste.iter().filter_map(|d| jours_de(&d.jour).map(|j| (j, d))).collect();
    tries.sort_by_key(|(j, d)| (*j, d.ms));
    for (j, d) in tries {
        // 1 January 1970 was a Thursday: Monday is 3 days before it
        let lundi = jour_txt(j - (j + 3).rem_euclid(7));
        match out.iter_mut().find(|s| s.lundi == lundi) {
            Some(s) => s.raisons.push(d.raison.clone()),
            None => out.push(SemaineDiscipline {
                lundi,
                raisons: vec![d.raison.clone()],
            }),
        }
    }
    out.reverse();
    out
}

/// Réglages du compte de prop firm (règles prop firm standard pré-remplies, comme v1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct JCfg {
    pub risk: f64,
    pub account: f64,
    /// 0 challenge, 1 vérification, 2 compte financé.
    pub phase: u8,
    /// Objectif du challenge en %.
    pub target: f64,
    pub daily: f64,
    pub maxloss: f64,
    pub min_days: u32,
    pub max_trades: u32,
    /// Début du challenge (« AAAA-MM-JJ », vide = depuis toujours).
    pub start: String,
    pub strats: Vec<String>,
    /// 🛡️ Gardien PropFirm : risque max d'un trade (% du compte) et positions ouvertes max.
    pub risk_pct: f64,
    pub max_pos: u32,
    /// 🛡️ Jour verrouillé par le gardien (« AAAA-MM-JJ » dans le fuseau du reset) : une limite a été touchée, plus
    /// d'ordre jusqu'à minuit (heure du reset). Vide : pas de verrou.
    pub lock: String,
    /// 🔓 Dernier déverrouillage de la journée (avec sa raison) ; absent : jamais (ou ancien journal).
    pub deverrouillage: Option<Box<Deverrouillage>>,
    /// 🔓 Every unlock of the guardian, oldest first (`DEVERROUILLAGES_MAX` at most), for the discipline review of
    /// the Journal; empty in an old journal.
    pub deverrouillages: Vec<Deverrouillage>,
    /// 🛡️ Alerte (une fois par jour) quand la perte atteint ce % d'une limite.
    pub warn: f64,
    /// 🛡️ Stop loss obligatoire sur chaque ordre.
    pub need_sl: bool,
    /// 🛡️ Pas d'ordre de 2 min avant à 2 min après une news à fort impact sur une devise du symbole.
    pub news: bool,
    /// 🛡️ Fuseau du reset quotidien (la journée de trading finit à minuit dans ce fuseau).
    pub reset_tz: String,
    /// 🛡️ Préréglage d'une firme (vide : règles par défaut ; les préréglages viendront plus tard).
    pub firm: String,
    /// 🧊 Pause anti-tilt : nombre de pertes de suite qui la déclenche (0 : éteinte).
    pub tilt_n: u32,
    /// 🧊 Durée de la pause, en minutes.
    pub tilt_min: u32,
    /// 🧊 Un trade fermé noté émotion « revenge » déclenche aussi la pause.
    pub tilt_revenge: bool,
    /// 🧊 Fin de la pause en cours (millisecondes UTC ; 0 : pas de pause).
    pub pause_until: i64,
}

impl Default for JCfg {
    fn default() -> Self {
        Self {
            risk: 500.0,
            account: 100_000.0,
            phase: 0,
            target: 10.0,
            daily: 5.0,
            maxloss: 10.0,
            min_days: 4,
            max_trades: 4,
            start: String::new(),
            strats: STRATS0.iter().map(|s| s.to_string()).collect(),
            risk_pct: 1.0,
            max_pos: 2,
            lock: String::new(),
            deverrouillage: None,
            deverrouillages: vec![],
            warn: 80.0,
            need_sl: true,
            news: true,
            reset_tz: "Europe/Prague".into(),
            firm: String::new(),
            tilt_n: 0,
            tilt_min: 30,
            tilt_revenge: false,
            pause_until: 0,
        }
    }
}

/// 🧊 Pertes de suite à la fin de l'historique (P&L < 0 ; un gain ou un trade à 0 remet le compte à zéro).
pub fn losing_streak(history: &[ClosedTrade]) -> u32 {
    history.iter().rev().take_while(|t| t.pnl < Money::ZERO).count() as u32
}

impl JCfg {
    /// 🧊 Juste après la fermeture de `closed` (déjà dans `history`) : la pause anti-tilt démarre-t-elle ?
    /// Oui s'il y a une nouvelle perte et au moins `tilt_n` pertes de suite, ou (option `tilt_revenge`) si un de ces
    /// trades est noté « revenge » (`revenge`). Rend la fin de la pause (ms) et la raison. Une série déjà punie ne
    /// relance rien toute seule : il faut un nouveau trade fermé.
    pub fn tilt_start(&self, history: &[ClosedTrade], closed: &[ClosedTrade], revenge: bool, now: i64) -> Option<(i64, String)> {
        let streak = losing_streak(history);
        let new_loss = closed.iter().any(|t| t.pnl < Money::ZERO);
        let why = if self.tilt_n > 0 && new_loss && streak >= self.tilt_n {
            format!("{streak} perte{} de suite", if streak > 1 { "s" } else { "" })
        } else if self.tilt_revenge && revenge {
            "trade noté revenge".to_string()
        } else {
            return None;
        };
        Some((now + i64::from(self.tilt_min) * 60_000, why))
    }

    /// 🧊 Minutes de pause qui restent à `now` (ms), arrondies vers le haut ; `None` hors pause.
    pub fn pause_left(&self, now: i64) -> Option<i64> {
        let left = self.pause_until - now;
        (left > 0).then(|| (left + 59_999) / 60_000)
    }
}

/// État prop firm du jour (comme v1) ; `day_of(ts)` : jour du trade dans le fuseau du reset (minuit).
/// 💰 Montants en centimes : chaque trade est arrondi une fois, les sommes et les comparaisons aux limites sont exactes.
#[derive(Debug, Clone, PartialEq)]
pub struct PropState {
    pub pnl: Money,
    pub target: Money,
    pub today_pnl: Money,
    pub today_open_risk: Money,
    pub daily_limit: Money,
    pub max_limit: Money,
    pub low: Money,
    pub days: usize,
    pub trades_today: usize,
    pub warnings: Vec<String>,
}

pub fn propfirm(list: &[JTrade], c: &JCfg, now: i64, since: i64, day_of: &dyn Fn(i64) -> String, money: &dyn Fn(f64) -> String) -> PropState {
    let mut closed: Vec<&JTrade> = list.iter().filter(|t| t.closed() && t.ts() >= since).collect();
    closed.sort_by_key(|t| t.ts());
    let m = |v: Option<f64>| Money::from_f64(v.unwrap_or(0.0));
    let pnl: Money = closed.iter().map(|t| m(t.pnl())).sum();
    let today = day_of(now);
    let today_list: Vec<&JTrade> = list.iter().filter(|t| day_of(t.ts()) == today).collect();
    let today_pnl: Money = today_list.iter().filter(|t| t.closed()).map(|t| m(t.pnl())).sum();
    let today_open_risk: Money = today_list.iter().filter(|t| !t.closed()).map(|t| m(t.risk)).sum();
    let (mut eq, mut low) = (Money::ZERO, Money::ZERO);
    for t in &closed {
        eq += m(t.pnl());
        low = low.min(eq);
    }
    let acc = Money::from_f64(if c.account > 0.0 { c.account } else { 100_000.0 });
    let target = match c.phase {
        2 => Money::ZERO,
        1 => acc.pct(5.0),
        _ => acc.pct(c.target),
    };
    let (daily_limit, max_limit) = (acc.pct(c.daily), acc.pct(c.maxloss));
    let mut days: Vec<String> = closed.iter().map(|t| day_of(t.entry_ts)).collect();
    days.sort();
    days.dedup();
    let n_today = today_list.len();
    let mut warnings = vec![];
    if -today_pnl + today_open_risk >= daily_limit.pct(80.0) {
        warnings.push(format!(
            "⛔ Tu es à {} de perte aujourd'hui (+ {} de risque ouvert) sur une limite de {} : arrête pour la journée.",
            money((-today_pnl).to_f64()),
            money(today_open_risk.to_f64()),
            money(daily_limit.to_f64())
        ));
    }
    if n_today as u32 >= c.max_trades {
        warnings.push(format!("✋ {n_today} trades aujourd'hui : ta limite perso est {}.", c.max_trades));
    }
    if -pnl >= max_limit.pct(70.0) {
        warnings.push(format!(
            "⚠️ Perte totale de {} sur {} permis.",
            money((-pnl).to_f64()),
            money(max_limit.to_f64())
        ));
    }
    PropState {
        pnl,
        target,
        today_pnl,
        today_open_risk,
        daily_limit,
        max_limit,
        low,
        days: days.len(),
        trades_today: n_today,
        warnings,
    }
}

/// Export CSV, mêmes colonnes que v1 (avec le BOM pour Excel) ; `date` : temps ms → texte, `sess` : session de l'entrée.
pub fn csv(list: &[JTrade], date: &dyn Fn(i64) -> String, sess: &dyn Fn(i64) -> String) -> String {
    let q = |s: &str| format!("\"{}\"", s.replace('"', "\"\""));
    let n = |v: Option<f64>| v.map(|x| x.to_string()).unwrap_or_default();
    let cols = [
        "date entrée",
        "date sortie",
        "symbole",
        "sens",
        "ut",
        "stratégie",
        "entrée",
        "sl",
        "tp",
        "sortie",
        "risque $",
        "frais $",
        "R",
        "P&L $",
        "session",
        "confluences",
        "émotion",
        "note",
        "notes",
    ];
    let mut out = format!("\u{feff}{}", cols.map(q).join(","));
    for t in list {
        let row = [
            date(t.entry_ts),
            t.exit_ts.map(date).unwrap_or_default(),
            t.sym.clone(),
            t.side.clone(),
            t.tf.clone(),
            t.strat.clone(),
            n(t.entry),
            n(t.sl),
            n(t.tp),
            n(t.exit),
            n(t.risk),
            n(t.fees),
            t.r().map(|v| format!("{v:.3}")).unwrap_or_default(),
            t.pnl().map(|v| format!("{v:.2}")).unwrap_or_default(),
            sess(t.entry_ts),
            t.conf.join(" | "),
            t.emo.clone(),
            n(t.rate),
            t.notes.clone(),
        ];
        out.push('\n');
        out.push_str(&row.iter().map(|c| q(c)).collect::<Vec<_>>().join(","));
    }
    out
}

/// 🔐 Le journal tel qu'il est gardé (chiffré sur le PC) et partagé avec le cell : réglages, trades, et le numéro de
/// version de la base (il monte à chaque écriture, d'où qu'elle vienne).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Donnees {
    pub cfg: JCfg,
    pub trades: Vec<JTrade>,
    pub version: u64,
}

/// 🔐 Ce qui change dans le journal, **entrée par entrée** (jamais le journal entier) : les réglages s'ils ont changé,
/// les trades ajoutés ou modifiés, les identifiants des trades retirés. Règle des conflits (PC + cell) : pour chaque
/// entrée, la dernière écriture gagne ; une entrée que l'appareil n'a pas touchée n'est jamais renvoyée, donc jamais
/// écrasée par un état plus vieux.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Changements {
    pub cfg: Option<JCfg>,
    pub mettre: Vec<JTrade>,
    pub retirer: Vec<String>,
}

impl Changements {
    /// Ce qui a changé entre `avant` (le dernier état connu de la base) et `apres` (l'état de l'appareil).
    pub fn entre(avant: &Donnees, apres_cfg: &JCfg, apres_trades: &[JTrade]) -> Self {
        let mettre = apres_trades
            .iter()
            .filter(|t| !t.id.is_empty() && avant.trades.iter().find(|a| a.id == t.id) != Some(t))
            .cloned()
            .collect();
        let retirer = avant
            .trades
            .iter()
            .filter(|a| !apres_trades.iter().any(|t| t.id == a.id))
            .map(|a| a.id.clone())
            .collect();
        Self {
            cfg: (*apres_cfg != avant.cfg).then(|| apres_cfg.clone()),
            mettre,
            retirer,
        }
    }

    /// Rien à écrire ?
    pub fn vide(&self) -> bool {
        self.cfg.is_none() && self.mettre.is_empty() && self.retirer.is_empty()
    }

    /// Applique ces changements à un journal (même règle que la base : entrée par entrée).
    pub fn appliquer(&self, d: &mut Donnees) {
        if let Some(c) = &self.cfg {
            d.cfg = c.clone();
        }
        d.trades.retain(|t| !self.retirer.contains(&t.id));
        for t in &self.mettre {
            match d.trades.iter_mut().find(|x| x.id == t.id) {
                Some(x) => *x = t.clone(),
                None => d.trades.push(t.clone()),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 🔓 The discipline review groups the unlocks by week (Monday to Sunday), most recent week first, reasons in order.
    #[test]
    fn bilan_discipline_par_semaine() {
        let d = |jour: &str, ms: i64, raison: &str| Deverrouillage {
            jour: jour.into(),
            ms,
            raison: raison.into(),
        };
        assert_eq!(jours_de("1970-01-01"), Some(0));
        assert_eq!(jour_txt(jours_de("2026-11-16").unwrap()), "2026-11-16");
        assert_eq!(jours_de("2026-13-01"), None);
        let liste = [
            d("2026-11-16", 2, "Erreur de saisie"), // Monday
            d("2026-11-22", 5, "News manquée"),     // Sunday, same week
            d("2026-11-23", 7, "Revenge"),          // next Monday
            d("2026-11-16", 1, "Premier de la semaine"),
            d("pas une date", 9, "ignoré"),
        ];
        let b = bilan_discipline(&liste);
        assert_eq!(
            b,
            vec![
                SemaineDiscipline {
                    lundi: "2026-11-23".into(),
                    raisons: vec!["Revenge".into()],
                },
                SemaineDiscipline {
                    lundi: "2026-11-16".into(),
                    raisons: vec!["Premier de la semaine".into(), "Erreur de saisie".into(), "News manquée".into()],
                },
            ]
        );
        assert!(bilan_discipline(&[]).is_empty());
        let ancien: JCfg = serde_json::from_str(r#"{"tilt_n":2}"#).unwrap();
        assert!(ancien.deverrouillages.is_empty(), "an old journal has no unlock history");
    }

    fn t(short: bool, entry: f64, sl: f64, exit: Option<f64>, ts: i64) -> JTrade {
        JTrade {
            sym: "BTC".into(),
            side: if short { "short" } else { "long" }.into(),
            entry: Some(entry),
            sl: Some(sl),
            exit,
            risk: Some(100.0),
            entry_ts: ts,
            exit_ts: exit.map(|_| ts + 60),
            strat: "Scalp".into(),
            ..Default::default()
        }
    }

    #[test]
    fn r_pnl_stats() {
        let a = t(false, 100.0, 90.0, Some(120.0), 0); // +2 R
        let b = t(true, 100.0, 110.0, Some(105.0), 100); // −0,5 R
        let c = t(false, 100.0, 95.0, None, 200); // ouvert
        assert_eq!((a.r(), b.r(), c.r()), (Some(2.0), Some(-0.5), None));
        assert_eq!((a.pnl(), b.pnl()), (Some(200.0), Some(-50.0)));
        let s = stats(&[a.clone(), b.clone(), c]);
        assert_eq!((s.n, s.open, s.total), (2, 1, 150.0));
        assert_eq!(s.win, Some(50.0));
        assert_eq!(s.pf, Some(4.0));
        assert_eq!(s.curve, vec![0.0, 200.0, 150.0]);
        assert_eq!(s.dd, 50.0);
        assert_eq!((s.best, s.worst), (Some(0), Some(1)));
        let g = group_by(&[a, b], |t| t.strat.clone());
        assert_eq!((g.len(), g[0].n, g[0].total), (1, 2, 150.0));
    }

    #[test]
    fn regles_propfirm() {
        let c = JCfg::default();
        let loss = t(false, 100.0, 90.0, Some(50.0), 1_000); // −5 R = −500 $ ; puis −4 000 $ de pnl réel
        let big = JTrade {
            pnl: Some(-4_000.0),
            ..loss.clone()
        };
        let day = |_: i64| "2026-10-26".to_string();
        let f = propfirm(&[big], &c, 2_000, 0, &day, &|v| format!("{v:.0} $"));
        let m = Money::from_f64;
        assert_eq!((f.target, f.daily_limit, f.max_limit), (m(10_000.0), m(5_000.0), m(10_000.0)));
        assert_eq!((f.pnl, f.low), (m(-4_000.0), m(-4_000.0)));
        assert_eq!(f.days, 1);
        // une sauvegarde de v1 (champs vides en « », temps en ms) s'importe telle quelle
        let v1: Vec<JTrade> = serde_json::from_str(r#"[{"id":"a1","sym":"BTC","side":"short","entry":100,"sl":110,"tp":"","exit":"90","risk":250,"fees":"","pnl":"","rate":"","entryTs":1700000000000,"exitTs":null,"conf":["RSI"],"emo":"calme","shot":"data:x"}]"#).unwrap();
        assert_eq!((v1[0].tp, v1[0].exit, v1[0].r(), v1[0].pnl()), (None, Some(90.0), Some(1.0), Some(250.0)));
        assert!(f.warnings.iter().any(|w| w.starts_with("⛔")), "80 % de la perte du jour : arrêter");
        assert_eq!(csv(&[loss], &|_| "26 oct.".into(), &|_| "Londres".into()).lines().count(), 2);
    }

    #[test]
    fn limites_propfirm_au_centime_pres() {
        // perte du jour exactement à la limite (5 000 $) en 3 trades à virgule : touchée, sans erreur d'arrondi
        let c = JCfg::default();
        let tr = |pnl: f64, ts: i64| JTrade {
            pnl: Some(pnl),
            exit: Some(1.0),
            entry_ts: ts,
            exit_ts: Some(ts + 1),
            ..Default::default()
        };
        let day = |_: i64| "2026-11-06".to_string();
        let list = [tr(-1666.67, 1), tr(-1666.67, 2), tr(-1666.66, 3)];
        let f = propfirm(&list, &c, 10, 0, &day, &|v| format!("{v:.2}"));
        assert_eq!(f.today_pnl, Money::from_f64(-5_000.0), "somme exacte : −5 000,00");
        assert!(-f.today_pnl >= f.daily_limit, "limite du jour touchée exactement");
        // 1000 petits trades de −0,01 $ : −10,00 $ exactement
        let many: Vec<JTrade> = (0..1000).map(|k| tr(-0.01, k)).collect();
        assert_eq!(propfirm(&many, &c, 10, 0, &day, &|v| format!("{v:.2}")).pnl, Money::from_f64(-10.0));
    }

    #[test]
    fn ancien_reglage_du_gardien_se_charge() {
        // un journal.json d'avant le gardien PropFirm : les nouveaux champs prennent leurs valeurs par défaut
        let c: JCfg = serde_json::from_str(r#"{"risk":300,"account":50000,"risk_pct":0.5,"lock":""}"#).unwrap();
        assert_eq!((c.risk, c.account, c.risk_pct), (300.0, 50_000.0, 0.5));
        assert_eq!((c.warn, c.need_sl, c.news), (80.0, true, true));
        assert_eq!(c.reset_tz, "Europe/Prague");
        assert!(c.firm.is_empty());
        // et la pause anti-tilt, arrivée après : éteinte, 30 min, sans revenge, pas de pause en cours
        assert_eq!((c.tilt_n, c.tilt_min, c.tilt_revenge, c.pause_until), (0, 30, false, 0));
    }

    /// 🧊 Un trade fermé de P&L `pnl` $.
    fn ferme(pnl: f64) -> ClosedTrade {
        ClosedTrade {
            base: "XAU".into(),
            side: crate::types::Side::Long,
            qty: 1.0,
            entry: 100.0,
            exit: 100.0 + pnl,
            pnl: Money::from_f64(pnl),
            fees: Money::ZERO,
            opened_at: 0,
            closed_at: 0,
            reason: "stop".into(),
            risk: Money::from_f64(10.0),
            partiel: false,
            glissement: Money::ZERO,
            funding: Money::ZERO,
            levier: None,
            plan: None,
            real_closed_at: None,
        }
    }

    #[test]
    fn pertes_de_suite_comptees_depuis_la_fin() {
        let h: Vec<ClosedTrade> = [-5.0, 8.0, -1.0, -2.0, -3.0].map(ferme).to_vec();
        assert_eq!(losing_streak(&h), 3);
        assert_eq!(losing_streak(&[ferme(-1.0), ferme(0.0)]), 0, "un trade à 0 remet à zéro");
        assert_eq!(losing_streak(&[]), 0);
    }

    #[test]
    fn pause_anti_tilt_eteinte_par_defaut() {
        let c = JCfg::default();
        let h: Vec<ClosedTrade> = [-1.0; 6].map(ferme).to_vec();
        assert_eq!(c.tilt_start(&h, &h[5..], true, 0), None, "ni série ni revenge : éteinte");
        assert_eq!(c.pause_left(0), None);
    }

    #[test]
    fn pause_anti_tilt_apres_n_pertes() {
        let c = JCfg { tilt_n: 3, ..JCfg::default() };
        let mut h: Vec<ClosedTrade> = [-1.0, -1.0].map(ferme).to_vec();
        assert_eq!(c.tilt_start(&h, &h[1..], false, 0), None, "2 pertes (N − 1) : rien");
        h.push(ferme(-1.0));
        assert_eq!(c.tilt_start(&h, &h[2..], false, 1_000), Some((1_000 + 30 * 60_000, "3 pertes de suite".into())));
        // un gain au milieu remet à zéro
        let h: Vec<ClosedTrade> = [-1.0, -1.0, 4.0, -1.0].map(ferme).to_vec();
        assert_eq!(c.tilt_start(&h, &h[3..], false, 0), None);
        // pas de redémarrage sans nouvelle perte : un gain fermé ensuite ne relance rien, une 4e perte oui
        let mut h: Vec<ClosedTrade> = [-1.0; 3].map(ferme).to_vec();
        assert_eq!(c.tilt_start(&h, &[], false, 0), None, "rien de neuf fermé");
        h.push(ferme(-2.0));
        assert!(c.tilt_start(&h, &h[3..], false, 0).is_some_and(|p| p.1 == "4 pertes de suite"));
    }

    #[test]
    fn pause_anti_tilt_sur_un_trade_revenge() {
        let h = vec![ferme(5.0)];
        let sans = JCfg::default();
        assert_eq!(sans.tilt_start(&h, &h, true, 0), None, "option éteinte : revenge ne compte pas");
        let avec = JCfg {
            tilt_revenge: true,
            tilt_min: 15,
            ..JCfg::default()
        };
        assert_eq!(avec.tilt_start(&h, &h, true, 0), Some((15 * 60_000, "trade noté revenge".into())));
        assert_eq!(avec.tilt_start(&h, &h, false, 0), None, "pas noté revenge");
    }

    #[test]
    fn minutes_de_pause_arrondies_vers_le_haut() {
        let c = JCfg {
            pause_until: 30 * 60_000,
            ..JCfg::default()
        };
        assert_eq!(c.pause_left(0), Some(30));
        assert_eq!(c.pause_left(60_000 + 1), Some(29), "28 min 59,999 s → 29");
        assert_eq!(c.pause_left(30 * 60_000 - 1), Some(1));
        assert_eq!(c.pause_left(30 * 60_000), None, "fin de la pause");
    }

    fn trade(id: &str, pnl: f64) -> JTrade {
        JTrade {
            id: id.into(),
            sym: "BTC".into(),
            pnl: Some(pnl),
            ..Default::default()
        }
    }

    #[test]
    fn changements_entree_par_entree() {
        let avant = Donnees {
            cfg: JCfg::default(),
            trades: vec![trade("a", 1.0), trade("b", 2.0), trade("c", 3.0)],
            version: 7,
        };
        let cfg2 = JCfg { tilt_n: 3, ..JCfg::default() };
        let apres = vec![trade("a", 1.0), trade("b", 20.0), trade("d", 4.0)];
        let ch = Changements::entre(&avant, &cfg2, &apres);
        assert_eq!(ch.cfg.as_ref().map(|c| c.tilt_n), Some(3));
        assert_eq!(
            ch.mettre.iter().map(|t| t.id.as_str()).collect::<Vec<_>>(),
            ["b", "d"],
            "seulement le modifié et le nouveau"
        );
        assert_eq!(ch.retirer, ["c"]);
        let mut d = avant.clone();
        ch.appliquer(&mut d);
        assert_eq!((d.cfg, d.trades), (cfg2, apres));
        assert!(
            Changements::entre(&avant, &JCfg::default(), &avant.trades).vide(),
            "rien de changé : rien à écrire"
        );
    }

    /// PC et cell écrivent chacun de leur côté : le cell (resté sur un état plus vieux) n'envoie que ce qu'il a touché,
    /// donc il n'écrase pas les trades ajoutés ou modifiés par le PC entre-temps.
    #[test]
    fn deux_appareils_sans_ecrasement() {
        let base0 = Donnees {
            trades: vec![trade("a", 1.0)],
            ..Default::default()
        };
        let mut base = base0.clone();
        // le PC modifie « a » et ajoute « p »
        Changements::entre(&base0, &JCfg::default(), &[trade("a", 10.0), trade("p", 5.0)]).appliquer(&mut base);
        // le cell, qui n'a vu que base0, ajoute « m » sans toucher « a »
        let du_cell = Changements::entre(&base0, &JCfg::default(), &[trade("a", 1.0), trade("m", 7.0)]);
        assert!(du_cell.cfg.is_none() && du_cell.retirer.is_empty());
        du_cell.appliquer(&mut base);
        assert_eq!(base.trades, vec![trade("a", 10.0), trade("p", 5.0), trade("m", 7.0)], "rien de perdu");
    }
}
