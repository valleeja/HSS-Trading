//! Types de base partagés par toutes les couches d'Argos.

use std::fmt;

/// Une bougie OHLCV. `time` = début de la bougie, en secondes Unix.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bar {
    pub time: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

impl Bar {
    pub fn is_bull(&self) -> bool {
        self.close >= self.open
    }
    pub fn range(&self) -> f64 {
        self.high - self.low
    }
}

/// Secondes d'un « mois » pour `aggregate` (les bougies sont alors regroupées par mois du calendrier, en UTC).
pub const MONTH_SECONDS: i64 = 2_592_000;

/// Début (UTC) du mois qui contient `t`.
fn month_start(t: i64) -> i64 {
    // jours ↔ date civile (algorithme de Howard Hinnant)
    let z = t.div_euclid(86_400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    // premier jour de ce même mois
    let doy1 = (153 * mp + 2) / 5;
    (era * 146_097 + 365 * yoe + yoe / 4 - yoe / 100 + doy1 - 719_468) * 86_400
}

/// (année, mois 1-12) de la date UTC de `t`.
fn civil(t: i64) -> (i64, i64) {
    let z = t.div_euclid(86_400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (yoe + era * 400 + i64::from(m <= 2), m)
}

/// Secondes UTC du 1er jour du mois `m` (1-12) de l'année `y`.
fn month_epoch(y: i64, m: i64) -> i64 {
    let y = y - i64::from(m <= 2);
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    (era * 146_097 + doe - 719_468) * 86_400
}

/// Une semaine, en secondes.
const WEEK: i64 = 604_800;
/// Lundi 5 janvier 1970, 00:00 UTC : les semaines commencent le lundi (le 1er janvier 1970 était un jeudi).
const MONDAY: i64 = 4 * 86_400;

/// Début de la bougie d'unité de temps `sec` qui contient `time`. Un multiple de `MONTH_SECONDS` = mois du calendrier
/// (3 mois : janv., avr., juil., oct. ; 12 mois : l'année) ; un multiple d'une semaine commence le lundi à 00:00 UTC.
pub fn bucket_start(time: i64, sec: i64) -> i64 {
    if sec == MONTH_SECONDS {
        month_start(time)
    } else if sec > MONTH_SECONDS && sec % MONTH_SECONDS == 0 {
        let n = sec / MONTH_SECONDS;
        let (y, m) = civil(time);
        let k = (y * 12 + m - 1).div_euclid(n) * n;
        month_epoch(k.div_euclid(12), k.rem_euclid(12) + 1)
    } else if sec % WEEK == 0 {
        (time - MONDAY).div_euclid(sec) * sec + MONDAY
    } else {
        time.div_euclid(sec) * sec
    }
}

/// Fin (exclue) de la bougie d'unité de temps `sec` qui commence à `start` : le 1er du mois suivant pour les mois
/// (28 à 31 jours), `start + sec` sinon.
pub fn bucket_end(start: i64, sec: i64) -> i64 {
    if sec >= MONTH_SECONDS && sec % MONTH_SECONDS == 0 {
        let (y, m) = civil(start);
        let k = y * 12 + m - 1 + sec / MONTH_SECONDS;
        month_epoch(k.div_euclid(12), k.rem_euclid(12) + 1)
    } else {
        start + sec
    }
}

/// Regroupe des bougies dans une unité de temps plus haute de `sec` secondes (ex. 30m → 4h ; `MONTH_SECONDS` = par mois). Bougies triées par temps.
pub fn aggregate(bars: &[Bar], sec: i64) -> Vec<Bar> {
    let mut out: Vec<Bar> = Vec::new();
    for b in bars {
        let t = bucket_start(b.time, sec);
        match out.last_mut() {
            Some(x) if x.time == t => {
                x.high = x.high.max(b.high);
                x.low = x.low.min(b.low);
                x.close = b.close;
                x.volume += b.volume;
            }
            _ => out.push(Bar { time: t, ..*b }),
        }
    }
    out
}

/// ⏪ Can bars of `fin` seconds be regrouped into bars of `sec` seconds without one straddling two (`fin` divides `sec`;
/// a week or a month: whole days or less, since they start on a Monday or on the 1st)?
pub fn se_regroupe(fin: i64, sec: i64) -> bool {
    if fin <= 0 || fin > sec {
        return false;
    }
    if fin == sec {
        return true;
    }
    if sec % MONTH_SECONDS == 0 || sec % WEEK == 0 {
        86_400 % fin == 0
    } else {
        sec % fin == 0
    }
}

/// ⏪ Start of the last bar of `sec` seconds finished at `heure` (the hour of a replay).
pub fn derniere_finie(heure: i64, sec: i64) -> i64 {
    let s = bucket_start(heure - 1, sec);
    if bucket_end(s, sec) <= heure {
        s
    } else {
        bucket_start(s - 1, sec)
    }
}

/// ⏪ End of the bar of `sec` seconds in progress at `heure`; `heure` itself when a bar has just closed.
pub fn fin_en_cours(heure: i64, sec: i64) -> i64 {
    let s = bucket_start(heure, sec);
    if s == heure {
        heure
    } else {
        bucket_end(s, sec)
    }
}

/// ⏪ What a chart of `sec`-second bars shows at `heure` during a replay, without seeing the future: its bars finished
/// at `heure`, then the bar in progress (not finished yet), rebuilt from `fines`. `fines`: the finer bars of the same
/// market already played (sorted, see [`se_regroupe`]); empty for another market or a finer chart, which then shows
/// its finished bars only. `own`: the chart's loaded bars (sorted); `fines` only fill in after the last of them.
pub fn au_replay(own: &[Bar], sec: i64, fines: &[Bar], heure: i64) -> Vec<Bar> {
    let mut out: Vec<Bar> = own[..own.partition_point(|b| b.time < heure)]
        .iter()
        .filter(|b| bucket_end(b.time, sec) <= heure)
        .copied()
        .collect();
    let depuis = out.last().map_or(i64::MIN, |b| bucket_end(b.time, sec));
    let fines = &fines[fines.partition_point(|b| b.time < depuis)..fines.partition_point(|b| b.time < heure)];
    let agg = aggregate(fines, sec);
    // a regrouped bar whose beginning is missing (the finer bars start inside it) would be wrong: left out
    let entiere = |a: &Bar| a.time >= depuis && fines.first().is_some_and(|f| f.time <= a.time);
    out.extend(agg.into_iter().filter(entiere));
    out
}

/// Bougies Heikin Ashi (même calcul que TradingView) : clôture = (O + H + L + C) / 4 ; ouverture = moyenne de l'ouverture et de la
/// clôture HA d'avant ((O + C) / 2 pour la première) ; haut / bas = extrêmes du vrai haut / bas et du corps HA. Le volume reste le même.
pub fn heikin_ashi(bars: &[Bar]) -> Vec<Bar> {
    let mut out: Vec<Bar> = Vec::with_capacity(bars.len());
    for b in bars {
        let close = (b.open + b.high + b.low + b.close) / 4.0;
        let open = match out.last() {
            Some(p) => (p.open + p.close) / 2.0,
            None => (b.open + b.close) / 2.0,
        };
        out.push(Bar {
            time: b.time,
            open,
            high: b.high.max(open).max(close),
            low: b.low.min(open).min(close),
            close,
            volume: b.volume,
        });
    }
    out
}

/// Briques Renko (méthode traditionnelle, comme TradingView) sur les clôtures, taille `size`.
/// Une brique de plus dans le même sens quand la clôture dépasse la brique d'une taille ; un retournement demande 2 tailles.
/// Chaque brique garde l'heure de la bougie qui l'a formée ; le volume accumulé depuis la brique d'avant est partagé entre
/// les briques formées par la même bougie.
pub fn renko(bars: &[Bar], size: f64) -> Vec<Bar> {
    let mut out = Vec::new();
    let Some(first) = bars.first() else { return out };
    if !(size.is_finite() && size > 0.0) {
        return out;
    }
    let base = (first.close / size).floor() * size;
    let (mut hi, mut lo) = (base, base);
    let mut vol = 0.0;
    for b in bars {
        vol += b.volume;
        let start = out.len();
        loop {
            let (o, c) = if b.close >= hi + size {
                (hi, hi + size)
            } else if b.close <= lo - size {
                (lo, lo - size)
            } else {
                break;
            };
            (hi, lo) = (o.max(c), o.min(c));
            out.push(Bar {
                time: b.time,
                open: o,
                high: hi,
                low: lo,
                close: c,
                volume: 0.0,
            });
        }
        let k = out.len() - start;
        if k > 0 {
            for x in &mut out[start..] {
                x.volume = vol / k as f64;
            }
            vol = 0.0;
        }
    }
    out
}

/// Unités de temps offertes par Argos.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Timeframe {
    M1,
    M3,
    M5,
    M15,
    M30,
    H1,
    H2,
    H4,
    H12,
    D1,
    W1,
}

impl Timeframe {
    pub const ALL: [Timeframe; 11] = [
        Self::M1,
        Self::M3,
        Self::M5,
        Self::M15,
        Self::M30,
        Self::H1,
        Self::H2,
        Self::H4,
        Self::H12,
        Self::D1,
        Self::W1,
    ];

    pub fn seconds(self) -> i64 {
        match self {
            Self::M1 => 60,
            Self::M3 => 180,
            Self::M5 => 300,
            Self::M15 => 900,
            Self::M30 => 1800,
            Self::H1 => 3600,
            Self::H2 => 7200,
            Self::H4 => 14400,
            Self::H12 => 43200,
            Self::D1 => 86400,
            Self::W1 => 604800,
        }
    }

    /// Identifiant court utilisé partout (« 30m », « 4h », « 1d »…).
    pub fn id(self) -> &'static str {
        match self {
            Self::M1 => "1m",
            Self::M3 => "3m",
            Self::M5 => "5m",
            Self::M15 => "15m",
            Self::M30 => "30m",
            Self::H1 => "1h",
            Self::H2 => "2h",
            Self::H4 => "4h",
            Self::H12 => "12h",
            Self::D1 => "1d",
            Self::W1 => "1w",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|t| t.id() == s)
    }
}

impl fmt::Display for Timeframe {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.id())
    }
}

/// Un marché : un exchange + le symbole de base (ex. « BTC » sur « okx »).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Symbol {
    pub exchange: String,
    pub base: String,
}

impl Symbol {
    pub fn new(exchange: &str, base: &str) -> Self {
        Self {
            exchange: exchange.to_lowercase(),
            base: base.to_uppercase(),
        }
    }
}

/// Achat (long) ou vente (short).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Side {
    Long,
    Short,
}

impl Side {
    /// +1 pour un long, -1 pour un short : multiplie un écart de prix pour avoir le P&L.
    pub fn sign(self) -> f64 {
        match self {
            Side::Long => 1.0,
            Side::Short => -1.0,
        }
    }
}

/// Erreurs communes à toutes les couches.
#[derive(Debug, Clone, PartialEq)]
pub enum ArgosError {
    /// La source de données n'a pas répondu ou a renvoyé une erreur.
    Feed(String),
    /// Une entrée invalide (symbole, quantité, prix…).
    Invalid(String),
    /// Action refusée par la sécurité (compte verrouillé, wallet non confirmé…).
    Denied(String),
    /// Message déjà écrit pour l'utilisateur, montré tel quel (erreurs des sources : « OANDA : jeton refusé »…).
    Other(String),
}

impl fmt::Display for ArgosError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Feed(m) => write!(f, "source de données : {m}"),
            Self::Invalid(m) => write!(f, "entrée invalide : {m}"),
            Self::Denied(m) => write!(f, "refusé : {m}"),
            Self::Other(m) => f.write_str(m),
        }
    }
}

impl std::error::Error for ArgosError {}

/// Un message d'erreur (texte) devient une `ArgosError` montrée telle quelle : `?` marche sur un `Result<_, String>`.
impl From<String> for ArgosError {
    fn from(m: String) -> Self {
        Self::Other(m)
    }
}

impl From<&str> for ArgosError {
    fn from(m: &str) -> Self {
        Self::Other(m.to_string())
    }
}

/// Et l'inverse : le texte montré à l'utilisateur, pour le code qui garde des erreurs en texte.
impl From<ArgosError> for String {
    fn from(e: ArgosError) -> Self {
        e.to_string()
    }
}

pub type Result<T> = std::result::Result<T, ArgosError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn erreur_texte_gardee_telle_quelle() {
        let e: ArgosError = String::from("OANDA : jeton refusé").into();
        assert_eq!(e.to_string(), "OANDA : jeton refusé", "le message de la source n'est pas changé");
        assert_eq!(String::from(ArgosError::from("x")), "x");
        assert_eq!(ArgosError::Feed("y".into()).to_string(), "source de données : y");
        // `?` d'une erreur texte vers une ArgosError, et l'inverse
        fn texte() -> std::result::Result<(), String> {
            Err("réseau coupé".into())
        }
        fn argos() -> Result<()> {
            texte()?;
            Ok(())
        }
        fn retour() -> std::result::Result<(), String> {
            argos()?;
            Ok(())
        }
        assert_eq!(retour(), Err("réseau coupé".to_string()));
    }

    #[test]
    fn timeframes_aller_retour() {
        for t in Timeframe::ALL {
            assert_eq!(Timeframe::parse(t.id()), Some(t));
        }
        assert_eq!(Timeframe::H4.seconds(), 14400);
        assert_eq!(Timeframe::parse("2d"), None);
    }

    #[test]
    fn symbole_normalise() {
        assert_eq!(
            Symbol::new("OKX", "zec"),
            Symbol {
                exchange: "okx".into(),
                base: "ZEC".into()
            }
        );
    }

    #[test]
    fn heikin_ashi_comme_tradingview() {
        let mk = |t: i64, o: f64, h: f64, l: f64, c: f64| Bar {
            time: t,
            open: o,
            high: h,
            low: l,
            close: c,
            volume: 5.0,
        };
        let ha = heikin_ashi(&[mk(0, 10.0, 12.0, 9.0, 11.0), mk(60, 11.0, 15.0, 10.0, 14.0)]);
        assert_eq!((ha[0].open, ha[0].close, ha[0].high, ha[0].low), (10.5, 10.5, 12.0, 9.0));
        // ouverture = (10,5 + 10,5) / 2 ; clôture = (11 + 15 + 10 + 14) / 4
        assert_eq!((ha[1].open, ha[1].close, ha[1].high, ha[1].low, ha[1].volume), (10.5, 12.5, 15.0, 10.0, 5.0));
    }

    #[test]
    fn renko_traditionnel() {
        let mk = |t: i64, c: f64| Bar {
            time: t,
            open: c,
            high: c,
            low: c,
            close: c,
            volume: 6.0,
        };
        // taille 10, départ 100 : 125 → 2 briques montantes (100-110, 110-120) ;
        // 105 → pas de retournement (il faut ≤ 100) ; 95 → 1 brique baissière 110 → 100 ; 79 → 100 → 90 → 80
        let r = renko(&[mk(0, 100.0), mk(1, 125.0), mk(2, 105.0), mk(3, 95.0), mk(4, 79.0)], 10.0);
        let oc: Vec<(f64, f64)> = r.iter().map(|b| (b.open, b.close)).collect();
        assert_eq!(oc, vec![(100.0, 110.0), (110.0, 120.0), (110.0, 100.0), (100.0, 90.0), (90.0, 80.0)]);
        assert_eq!(r.iter().map(|b| b.time).collect::<Vec<_>>(), vec![1, 1, 3, 4, 4]);
        // volume : 2 bougies (12) partagé entre les 2 premières briques ; puis 2 bougies (12) sur la brique de 3
        assert_eq!((r[0].volume, r[1].volume, r[2].volume), (6.0, 6.0, 12.0));
        assert!(renko(&[mk(0, 1.0)], 0.0).is_empty());
    }

    #[test]
    fn regroupe_30m_en_1h() {
        let b = |t: i64, o: f64, h: f64, l: f64, c: f64| Bar {
            time: t,
            open: o,
            high: h,
            low: l,
            close: c,
            volume: 1.0,
        };
        let h = aggregate(&[b(3600, 1.0, 3.0, 0.5, 2.0), b(5400, 2.0, 4.0, 1.0, 3.5), b(7200, 3.5, 3.6, 3.0, 3.1)], 3600);
        assert_eq!(h.len(), 2);
        assert_eq!(
            h[0],
            Bar {
                time: 3600,
                open: 1.0,
                high: 4.0,
                low: 0.5,
                close: 3.5,
                volume: 2.0
            }
        );
        assert_eq!(h[1].time, 7200);
        // 2026-10-25 12:00 UTC → 2026-10-01 ; 2024-02-29 → 2024-02-01
        assert_eq!(month_start(1_792_929_600), 1_790_812_800);
        assert_eq!(month_start(1_709_164_800), 1_706_745_600);
    }

    /// ⏪ A 4h chart at 13:00 during a replay: its finished bars, then the 12:00 bar in progress made of the 30m bars
    /// already played (12:00 to 12:30), never the rest of it.
    #[test]
    fn au_replay_bougie_en_cours() {
        let t0 = 14_400 * 118_052; // a 4h boundary
        let bar = |time: i64, p: f64| Bar {
            time,
            open: p,
            high: p + 2.0,
            low: p - 2.0,
            close: p + 1.0,
            volume: 1.0,
        };
        let fines: Vec<Bar> = (0..40).map(|i| bar(t0 + i * 1800, 100.0 + i as f64)).collect();
        let own = aggregate(&fines, 14_400);
        assert_eq!(own.len(), 5);
        let heure = t0 + 4 * 3600 + 3600; // 13:00 when t0 is 08:00
        let v = au_replay(&own, 14_400, &fines, heure);
        assert_eq!(v.len(), 2, "08:00 finished, 12:00 in progress");
        assert_eq!(v[0], own[0]);
        let c = v[1];
        assert_eq!(c.time, t0 + 14_400);
        assert_eq!((c.open, c.close), (108.0, 110.0), "the 12:00 and 12:30 bars only");
        assert_eq!((c.high, c.low, c.volume), (111.0, 106.0, 2.0));
        assert!(v.iter().all(|b| b.time < heure));
        // at a 4h close: no bar in progress
        let v = au_replay(&own, 14_400, &fines, t0 + 2 * 14_400);
        assert_eq!(v, own[..2].to_vec());
        // another market (no finer bars): the finished bars only
        assert_eq!(au_replay(&own, 14_400, &[], heure), own[..1].to_vec());
        // the finer bars start inside a 4h bar: that one comes from the loaded bars, whole
        let v = au_replay(&own, 14_400, &fines[3..], t0 + 2 * 14_400 + 1800);
        assert_eq!(v[..2].to_vec(), own[..2].to_vec());
        assert_eq!(v.len(), 3);
        assert_eq!(v[2].time, t0 + 2 * 14_400);
        assert_eq!(v[2].close, fines[16].close);
        // a finer chart (1h at 13:00 from its own bars): finished bars only, up to 12:00
        let h1 = aggregate(&fines, 3600);
        let v = au_replay(&h1, 3600, &[], heure);
        assert_eq!(v.last().unwrap().time, heure - 3600);
    }

    #[test]
    fn heures_du_replay() {
        let t0 = 14_400 * 118_052;
        assert_eq!(
            derniere_finie(t0 + 3600, 14_400),
            t0 - 14_400,
            "13:00 → the 4h bar of 08:00 is the last finished"
        );
        assert_eq!(derniere_finie(t0 + 14_400, 14_400), t0);
        assert_eq!(derniere_finie(t0 + 3600, 900), t0 + 2700);
        assert_eq!(fin_en_cours(t0 + 3600, 14_400), t0 + 14_400);
        assert_eq!(fin_en_cours(t0, 14_400), t0);
        // a month: 25 Oct. 2026 12:00 → the October bar ends on 1 Nov.
        assert_eq!(fin_en_cours(1_792_929_600, MONTH_SECONDS), 1_793_491_200);
        assert_eq!(derniere_finie(1_792_929_600, MONTH_SECONDS), month_start(1_790_812_800 - 1));
        assert!(se_regroupe(1800, 14_400) && se_regroupe(3600, 3600) && se_regroupe(3600, WEEK) && se_regroupe(86_400, MONTH_SECONDS));
        assert!(!se_regroupe(14_400, 3600), "a coarser chart");
        assert!(!se_regroupe(2700, 3600), "45m straddles the hours");
        assert!(!se_regroupe(3 * 86_400, MONTH_SECONDS), "3 days straddle the months");
        assert!(!se_regroupe(3 * 86_400, WEEK));
    }
}
