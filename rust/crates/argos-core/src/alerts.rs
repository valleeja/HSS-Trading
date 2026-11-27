//! 🔔 Alertes (comme v1) : alerte de prix (franchit vers le haut, vers le bas, ou les deux) et alertes
//! multi-conditions ET / OU (prix, EMA, RSI, volume, nombre ; plus grand, plus petit, croise), vérifiées à la clôture
//! de chaque bougie. Options : une seule fois ou à chaque fois avec un délai, fin de validité, message libre.
//! Calcul pur, gardé par la fenêtre dans alerts.json ; les déclenchements vont dans l'historique.

use crate::Bar;
use serde::{Deserialize, Serialize};

/// Côté d'une condition : une série calculée sur les bougies, ou un nombre.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Src {
    Close,
    Open,
    High,
    Low,
    /// Moyenne mobile exponentielle sur N bougies.
    Ema(u32),
    /// RSI 14 (lissage de Wilder).
    Rsi,
    Volume,
    /// Volume moyen sur 20 bougies.
    VolMa,
    Num(f64),
    /// 📈 Une ligne d'indicateur, de script ou de module, donnée par l'app au moment du calcul (`line_id` de son nom).
    Line(u64),
    /// 🖍 A drawn line through (t0, p0) and (t1, p1) (seconds, price), its price at each bar's time (extended both
    /// ways); t0 == t1: a horizontal line at p0.
    Trend {
        t0: i64,
        p0: f64,
        t1: i64,
        p1: f64,
    },
}

impl Src {
    /// 🖍 The price of a drawn line at time `t`.
    fn trend_at(t0: i64, p0: f64, t1: i64, p1: f64, t: i64) -> f64 {
        if t1 == t0 {
            p0
        } else {
            p0 + (p1 - p0) * (t - t0) as f64 / (t1 - t0) as f64
        }
    }
}

/// Identifiant stable d'une ligne d'indicateur à partir de son nom (« RSI 14 · RSI ») : FNV-1a 64 bits.
pub fn line_id(name: &str) -> u64 {
    name.bytes()
        .fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ b as u64).wrapping_mul(0x0100_0000_01b3))
}

impl Src {
    pub fn label(self) -> String {
        match self {
            Src::Close => "Prix".into(),
            Src::Open => "Ouverture".into(),
            Src::High => "Haut".into(),
            Src::Low => "Bas".into(),
            Src::Ema(n) => format!("EMA {n}"),
            Src::Rsi => "RSI 14".into(),
            Src::Volume => "Volume".into(),
            Src::VolMa => "Volume moyen 20".into(),
            Src::Num(v) => format!("{v}"),
            Src::Line(_) => "Indicateur".into(),
            Src::Trend { .. } => "Dessin".into(),
        }
    }

    /// Valeurs de la série sur toutes les bougies (`None` tant qu'elle n'a pas assez de bougies) ; `ext` : les lignes
    /// d'indicateurs de l'app (une valeur par bougie, `NaN` = vide).
    fn series(self, bars: &[Bar], ext: &dyn Fn(u64) -> Option<Vec<f64>>) -> Vec<Option<f64>> {
        match self {
            Src::Line(id) => match ext(id) {
                Some(v) => (0..bars.len()).map(|i| v.get(i).copied().filter(|x| x.is_finite())).collect(),
                None => vec![None; bars.len()],
            },
            Src::Trend { t0, p0, t1, p1 } => bars.iter().map(|b| Some(Src::trend_at(t0, p0, t1, p1, b.time))).collect(),
            Src::Close => bars.iter().map(|b| Some(b.close)).collect(),
            Src::Open => bars.iter().map(|b| Some(b.open)).collect(),
            Src::High => bars.iter().map(|b| Some(b.high)).collect(),
            Src::Low => bars.iter().map(|b| Some(b.low)).collect(),
            Src::Volume => bars.iter().map(|b| Some(b.volume)).collect(),
            Src::Num(v) => vec![Some(v); bars.len()],
            Src::VolMa => (0..bars.len())
                .map(|i| (i >= 19).then(|| bars[i - 19..=i].iter().map(|b| b.volume).sum::<f64>() / 20.0))
                .collect(),
            Src::Ema(n) => {
                let a = 2.0 / (n as f64 + 1.0);
                let mut e: Option<f64> = None;
                bars.iter()
                    .enumerate()
                    .map(|(i, b)| {
                        let v = e.map_or(b.close, |e| b.close * a + e * (1.0 - a));
                        e = Some(v);
                        (i + 1 >= n as usize).then_some(v)
                    })
                    .collect()
            }
            Src::Rsi => {
                let mut out = vec![None; bars.len()];
                let (mut g, mut l) = (0.0, 0.0);
                for i in 1..bars.len() {
                    let d = bars[i].close - bars[i - 1].close;
                    let (up, dn) = (d.max(0.0), (-d).max(0.0));
                    if i <= 14 {
                        g += up / 14.0;
                        l += dn / 14.0;
                        if i < 14 {
                            continue;
                        }
                    } else {
                        g = (g * 13.0 + up) / 14.0;
                        l = (l * 13.0 + dn) / 14.0;
                    }
                    out[i] = Some(if l > 0.0 { 100.0 - 100.0 / (1.0 + g / l) } else { 100.0 });
                }
                out
            }
        }
    }
}

/// Comparaison.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Op {
    Above,
    Below,
    CrossUp,
    CrossDown,
    /// 🖍 Crosses either way (« Croisement »).
    Cross,
    /// Entre dans la zone entre `r` et `r2` (n'y était pas à la bougie d'avant).
    EntersZone,
    /// Sort de la zone entre `r` et `r2`.
    LeavesZone,
}

impl Op {
    pub fn label(self) -> &'static str {
        match self {
            Op::Above => "plus grand que",
            Op::Below => "plus petit que",
            Op::CrossUp => "croise au-dessus de",
            Op::CrossDown => "croise en dessous de",
            Op::Cross => "croise",
            Op::EntersZone => "entre dans la zone",
            Op::LeavesZone => "sort de la zone",
        }
    }
}

/// Une condition : `l op r`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Cond {
    pub l: Src,
    pub op: Op,
    pub r: Src,
    /// Seconde borne d'une zone (« entre / sort de la zone » entre `r` et `r2`).
    #[serde(default)]
    pub r2: Option<Src>,
}

/// Sens d'une alerte de prix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Dir {
    Up,
    Down,
    Both,
}

/// Ce que surveille l'alerte.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Kind {
    /// Le prix en direct franchit `price`.
    Price { price: f64, dir: Dir },
    /// Conditions sur la dernière bougie fermée de l'unité de temps `tf` (secondes) ; `all` : ET, sinon OU.
    Multi { tf: i64, all: bool, conds: Vec<Cond> },
}

/// Une alerte.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Alert {
    pub id: u64,
    /// Symbole (comme le titre de la chart : « BTC », « XAUUSD »…).
    pub sym: String,
    pub kind: Kind,
    /// `None` : une seule fois (elle se coupe) ; `Some(s)` : à chaque fois, au moins `s` secondes entre deux.
    pub every: Option<i64>,
    /// Fin de validité (secondes UTC) ; `None` : sans fin.
    pub until: Option<i64>,
    /// Message libre (vide : le texte de la condition).
    pub msg: String,
    pub on: bool,
    /// Dernier déclenchement (secondes UTC).
    pub fired: Option<i64>,
    /// Dernier prix vu (alerte de prix) ou dernière bougie déjà vérifiée (multi).
    #[serde(default)]
    pub seen: Option<f64>,
    /// 📈 Noms des lignes d'indicateurs utilisées (pour le texte de l'alerte).
    #[serde(default)]
    pub lines: Vec<(u64, String)>,
    /// 🔊 This alert's sound (a name of `SONS`, or `SON_COUPE`); `None`: the default sound of the settings (alerts
    /// saved before sounds existed read as `None`).
    #[serde(default)]
    pub son: Option<String>,
    /// 🔔 The market the alert was made on, as the app writes it (« binance:BTCUSDT », « hl:BTC »): the panel shows
    /// its exchange and quote, « Trade » opens it. Empty for alerts saved before the panel (only `sym` is known).
    #[serde(default)]
    pub coin: String,
    /// 🔔 The chart's time frame when the alert was made (seconds); `None` for alerts saved before the panel.
    #[serde(default)]
    pub tf: Option<i64>,
    /// 🔔 When the alert was made (seconds UTC); `None` for alerts saved before the panel.
    #[serde(default)]
    pub created: Option<i64>,
}

/// 🔔 How an alert reads in the panel: « Au-dessus », « En dessous », « Croisement », « n conditions ».
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Sens {
    Up,
    Down,
    #[default]
    Cross,
    /// A multi-condition alert (its number of conditions).
    Conds(usize),
}

/// 💰 A price kept exactly: `Decimal` written as a string in alerts.json (« 58897.5 »).
mod dec_str {
    use rust_decimal::Decimal;
    use serde::{Deserialize, Deserializer, Serializer};
    use std::str::FromStr;

    pub fn serialize<S: Serializer>(d: &Option<Decimal>, s: S) -> Result<S::Ok, S::Error> {
        match d {
            Some(d) => s.serialize_some(&d.to_string()),
            None => s.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Decimal>, D::Error> {
        let s: Option<String> = Option::deserialize(d)?;
        Ok(s.and_then(|s| Decimal::from_str(&s).ok()))
    }
}

/// 💰 A price read as f64 (the feeds) kept as an exact decimal, as it is written (« 58897.5 », not 58897.4999…).
pub fn decimal(x: f64) -> Option<rust_decimal::Decimal> {
    use std::str::FromStr;
    x.is_finite().then(|| rust_decimal::Decimal::from_str(&format!("{x}")).ok()).flatten()
}

/// 🔊 The alert sounds, by name (the page /app/ plays them by the same names, synthesised with WebAudio); the first
/// one is the default.
pub const SONS: [&str; 8] = ["Bip", "Carillon", "Cloche", "Triple bip", "Montée", "Descente", "Gong doux", "Clochette"];
/// 🔊 The default alert sound (the beep of before).
pub const SON_DEFAUT: &str = SONS[0];
/// 🔇 « Son coupé »: the alert shows its notification, silently.
pub const SON_COUPE: &str = "aucun";
/// 🔊 The default alert volume (0 – 100).
pub const VOLUME_DEFAUT: u8 = 60;

/// 🔊 The sound choices of one alert, in the menus' order: (label, the alert's `son`): « Par défaut » (the settings'
/// sound), the 8 sounds, « Son coupé ».
pub fn choix_sons() -> Vec<(&'static str, Option<&'static str>)> {
    std::iter::once(("Par défaut", None))
        .chain(SONS.iter().map(|s| (*s, Some(*s))))
        .chain(std::iter::once(("Son coupé", Some(SON_COUPE))))
        .collect()
}

/// 🔊 A sound name as it is played: a known sound or `SON_COUPE` as is, anything else the default sound.
pub fn son_connu(son: &str) -> &'static str {
    if son == SON_COUPE {
        return SON_COUPE;
    }
    SONS.iter().copied().find(|s| *s == son).unwrap_or(SON_DEFAUT)
}

/// 🔊 The sound of alerts fired together (`sons`: each fired alert's own sound, in order; `defaut`: the settings'
/// sound): the first alert with its own sound plays it; without one, the default; silent only when every fired
/// alert is « Son coupé ».
pub fn son_a_jouer<'a>(sons: impl IntoIterator<Item = Option<&'a str>>, defaut: &str) -> &'static str {
    let mut tous_coupes = true;
    let mut vide = true;
    for s in sons {
        vide = false;
        match s {
            Some(SON_COUPE) => {}
            Some(s) => return son_connu(s),
            None => tous_coupes = false,
        }
    }
    if tous_coupes && !vide {
        SON_COUPE
    } else {
        son_connu(defaut)
    }
}

impl Alert {
    /// Texte de la condition : « BTC franchit 80 000 », « BTC 4h : RSI 14 plus petit que 30 ET … ».
    pub fn text(&self) -> String {
        match &self.kind {
            Kind::Price { price, dir } => format!(
                "{} {} {price}",
                self.sym,
                match dir {
                    Dir::Up => "monte au-dessus de",
                    Dir::Down => "descend sous",
                    Dir::Both => "franchit",
                }
            ),
            Kind::Multi { tf, all, conds } => format!(
                "{} {} : {}",
                self.sym,
                crate::Interval(*tf).short(),
                conds
                    .iter()
                    .map(|c| {
                        let name = |s: Src| match s {
                            Src::Line(id) => self.lines.iter().find(|l| l.0 == id).map_or(s.label(), |l| l.1.clone()),
                            _ => s.label(),
                        };
                        match c.r2 {
                            Some(r2) if matches!(c.op, Op::EntersZone | Op::LeavesZone) => {
                                format!("{} {} {} – {}", name(c.l), c.op.label(), name(c.r), name(r2))
                            }
                            _ => format!("{} {} {}", name(c.l), c.op.label(), name(c.r)),
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(if *all { " ET " } else { " OU " })
            ),
        }
    }

    /// Peut-elle se déclencher à `now` (allumée, pas expirée, délai écoulé) ?
    fn ready(&self, now: i64) -> bool {
        self.on && self.until.is_none_or(|u| now <= u) && self.fired.is_none_or(|f| self.every.is_some_and(|s| now - f >= s))
    }

    /// 🔔 How the panel reads it: its sense (« Au-dessus »…) and its price (a price alert's, else the first number
    /// or drawn line of its conditions, if any).
    pub fn sens(&self) -> (Sens, Option<f64>) {
        match &self.kind {
            Kind::Price { price, dir } => (
                match dir {
                    Dir::Up => Sens::Up,
                    Dir::Down => Sens::Down,
                    Dir::Both => Sens::Cross,
                },
                Some(*price),
            ),
            Kind::Multi { conds, .. } => {
                let px = conds.iter().find_map(|c| match c.r {
                    Src::Num(n) => Some(n),
                    Src::Trend { p1, .. } => Some(p1),
                    _ => None,
                });
                let sens = match conds.as_slice() {
                    [c] => match c.op {
                        Op::Above | Op::CrossUp => Sens::Up,
                        Op::Below | Op::CrossDown => Sens::Down,
                        Op::Cross | Op::EntersZone | Op::LeavesZone => Sens::Cross,
                    },
                    _ => Sens::Conds(conds.len()),
                };
                (sens, px)
            }
        }
    }

    /// 🔔 The arrow by the price the panel shows: the sense of the condition that price comes from (↗ « au-dessus »,
    /// ↘ « en dessous », ↕ a crossing or no price); a price alert's own sense.
    pub fn sens_du_prix(&self) -> Sens {
        match &self.kind {
            Kind::Price { .. } => self.sens().0,
            Kind::Multi { conds, .. } => conds
                .iter()
                .find(|c| matches!(c.r, Src::Num(_) | Src::Trend { .. }))
                .map_or(Sens::Cross, |c| match c.op {
                    Op::Above | Op::CrossUp => Sens::Up,
                    Op::Below | Op::CrossDown => Sens::Down,
                    Op::Cross | Op::EntersZone | Op::LeavesZone => Sens::Cross,
                }),
        }
    }

    fn fire(&mut self, now: i64, px: f64) -> (Fired, Declenchement) {
        self.fired = Some(now);
        if self.every.is_none() {
            self.on = false;
        }
        let (sens, target) = self.sens();
        let tf = match self.kind {
            Kind::Multi { tf, .. } => Some(tf),
            Kind::Price { .. } => self.tf,
        };
        let f = Fired {
            id: self.id,
            ts: now,
            text: if self.msg.is_empty() {
                self.text()
            } else {
                format!("{} · {}", self.msg, self.text())
            },
        };
        let d = Declenchement {
            id: self.id,
            ts: now,
            sym: self.sym.clone(),
            coin: self.coin.clone(),
            tf,
            sens,
            target: target.and_then(decimal),
            price: decimal(px),
            every: self.every.is_some(),
        };
        (f, d)
    }
}

/// Un déclenchement (historique : the 🔔 notifications feed; Argos Murs adds its own here too).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Fired {
    pub id: u64,
    pub ts: i64,
    pub text: String,
}

/// 🔔 A firing as the alerts panel's « Historique » shows it (22 Nov 2026): the alert's symbol, market, time frame
/// and sense, the price aimed at and the TRUE price when it fired (the live price; a candle alert: the close of the
/// closed candle), exact decimals written as text in alerts.json.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Declenchement {
    pub id: u64,
    pub ts: i64,
    pub sym: String,
    pub coin: String,
    pub tf: Option<i64>,
    pub sens: Sens,
    #[serde(with = "dec_str")]
    pub target: Option<rust_decimal::Decimal>,
    #[serde(with = "dec_str")]
    pub price: Option<rust_decimal::Decimal>,
    /// « Chaque fois » (else « Une fois »).
    pub every: bool,
}

/// Toutes les alertes et l'historique (alerts.json). `declenchements` came with the alerts panel: a file saved before
/// reads it empty.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Alerts {
    pub list: Vec<Alert>,
    pub history: Vec<Fired>,
    pub next: u64,
    /// 🔔 The alerts panel's history, newest first, [`HISTORY`] at most.
    pub declenchements: Vec<Declenchement>,
}

/// Historique gardé (les plus récents).
const FEED: usize = 50;
/// 🔔 The alerts panel's history: the 500 last firings (Jee's mock-up « alertes v1 », 22 Nov 2026).
pub const HISTORY: usize = 500;

/// Conditions vraies sur la dernière bougie fermée de `bars` (l'avant-dernière : la dernière est en cours) ?
pub fn eval(all: bool, conds: &[Cond], bars: &[Bar]) -> bool {
    eval_with(all, conds, bars, &|_| None)
}

/// Comme `eval`, avec les lignes d'indicateurs de l'app (`ext`).
pub fn eval_with(all: bool, conds: &[Cond], bars: &[Bar], ext: &dyn Fn(u64) -> Option<Vec<f64>>) -> bool {
    if bars.len() < 3 || conds.is_empty() {
        return false;
    }
    let i = bars.len() - 2;
    let one = |c: &Cond| {
        let (l, r) = (c.l.series(bars, ext), c.r.series(bars, ext));
        let (Some(a), Some(b)) = (l[i], r[i]) else { return false };
        match c.op {
            Op::Above => a > b,
            Op::Below => a < b,
            Op::EntersZone | Op::LeavesZone => {
                let Some(r2) = c.r2.map(|s| s.series(bars, ext)) else { return false };
                let (Some(b2), Some(a0), Some(b0), Some(b20)) = (r2[i], l[i - 1], r[i - 1], r2[i - 1]) else {
                    return false;
                };
                let inside = |x: f64, p: f64, q: f64| x >= p.min(q) && x <= p.max(q);
                let (now, before) = (inside(a, b, b2), inside(a0, b0, b20));
                if c.op == Op::EntersZone {
                    now && !before
                } else {
                    !now && before
                }
            }
            Op::CrossUp | Op::CrossDown | Op::Cross => {
                let (Some(a0), Some(b0)) = (l[i - 1], r[i - 1]) else { return false };
                let (up, down) = (a0 <= b0 && a > b, a0 >= b0 && a < b);
                match c.op {
                    Op::CrossUp => up,
                    Op::CrossDown => down,
                    _ => up || down,
                }
            }
        }
    };
    if all {
        conds.iter().all(one)
    } else {
        conds.iter().any(one)
    }
}

impl Alerts {
    /// Ajoute une alerte (allumée) ; son identifiant.
    pub fn add(&mut self, sym: &str, kind: Kind, every: Option<i64>, until: Option<i64>, msg: &str) -> u64 {
        self.next += 1;
        self.list.push(Alert {
            id: self.next,
            sym: sym.into(),
            kind,
            every,
            until,
            msg: msg.into(),
            on: true,
            fired: None,
            seen: None,
            lines: vec![],
            son: None,
            coin: String::new(),
            tf: None,
            created: None,
        });
        self.next
    }

    fn log(&mut self, f: Vec<(Fired, Declenchement)>) -> Vec<Fired> {
        let (f, d): (Vec<Fired>, Vec<Declenchement>) = f.into_iter().unzip();
        self.history.splice(0..0, f.iter().rev().cloned());
        self.history.truncate(FEED);
        self.declenchements.splice(0..0, d.into_iter().rev());
        self.declenchements.truncate(HISTORY);
        f
    }

    /// 🔔 « Vider l'historique » (after its confirmation): the panel's history and the alerts of the notifications
    /// feed; the alerts themselves stay.
    pub fn clear_history(&mut self) {
        self.declenchements.clear();
        self.history.clear();
    }

    /// Symboles qui ont une alerte allumée (à surveiller en direct, même s'ils ne sont pas sur la chart).
    pub fn symbols(&self) -> Vec<String> {
        let mut v: Vec<String> = self.list.iter().filter(|a| a.on).map(|a| a.sym.clone()).collect();
        v.sort();
        v.dedup();
        v
    }

    /// Prix en direct `px` de `sym` à `now` : les alertes de prix franchies se déclenchent.
    pub fn check_price(&mut self, sym: &str, px: f64, now: i64) -> Vec<Fired> {
        let mut out = vec![];
        for a in self.list.iter_mut().filter(|a| a.sym == sym) {
            let Kind::Price { price, dir } = a.kind else { continue };
            let prev = a.seen.replace(px);
            let Some(p0) = prev else { continue };
            let up = p0 < price && px >= price;
            let down = p0 > price && px <= price;
            let hit = match dir {
                Dir::Up => up,
                Dir::Down => down,
                Dir::Both => up || down,
            };
            if hit && a.ready(now) {
                out.push(a.fire(now, px));
            }
        }
        self.log(out)
    }

    /// Bougies `bars` de `sym` en unité de temps `tf` : les alertes multi-conditions se vérifient une fois par
    /// bougie fermée.
    pub fn check_bars(&mut self, sym: &str, tf: i64, bars: &[Bar], now: i64) -> Vec<Fired> {
        self.check_bars_with(sym, tf, bars, now, &|_| None)
    }

    /// Comme `check_bars`, avec les lignes d'indicateurs de l'app (`ext`, alignées sur `bars`).
    pub fn check_bars_with(&mut self, sym: &str, tf: i64, bars: &[Bar], now: i64, ext: &dyn Fn(u64) -> Option<Vec<f64>>) -> Vec<Fired> {
        let mut out = vec![];
        let Some((closed, close)) = bars.len().checked_sub(2).map(|i| (bars[i].time as f64, bars[i].close)) else {
            return vec![];
        };
        for a in self.list.iter_mut().filter(|a| a.sym == sym) {
            let Kind::Multi { tf: t, all, conds } = &a.kind else { continue };
            if *t != tf || a.seen == Some(closed) {
                continue;
            }
            a.seen = Some(closed);
            if a.ready(now) && eval_with(*all, conds, bars, ext) {
                out.push(a.fire(now, close));
            }
        }
        self.log(out)
    }

    /// Les alertes expirées s'éteignent (elles restent dans la liste).
    pub fn expire(&mut self, now: i64) {
        for a in self.list.iter_mut().filter(|a| a.until.is_some_and(|u| now > u)) {
            a.on = false;
        }
    }

    /// Prix des alertes de prix allumées de `sym` (le compte de la barre du haut) : (identifiant, prix).
    pub fn lines(&self, sym: &str) -> Vec<(u64, f64)> {
        self.list
            .iter()
            .filter(|a| a.on && a.sym == sym)
            .filter_map(|a| match a.kind {
                Kind::Price { price, .. } => Some((a.id, price)),
                Kind::Multi { .. } => None,
            })
            .collect()
    }

    /// 🔔 The price alert lines of `sym` drawn on the chart, paused ones too (Jee, 19 Nov: a paused alert stays,
    /// grey): (id, price, on). Triggering never reads it (`ready` checks `on`).
    pub fn lignes(&self, sym: &str) -> Vec<(u64, f64, bool)> {
        self.list
            .iter()
            .filter(|a| a.sym == sym)
            .filter_map(|a| match a.kind {
                Kind::Price { price, .. } => Some((a.id, price, a.on)),
                Kind::Multi { .. } => None,
            })
            .collect()
    }

    /// Déplace la ligne d'une alerte de prix (glissée à la souris ou au doigt).
    pub fn move_line(&mut self, id: u64, price: f64) {
        if let Some(a) = self.list.iter_mut().find(|a| a.id == id) {
            if let Kind::Price { price: p, .. } = &mut a.kind {
                *p = price;
                a.seen = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {

    /// 🖍 An alert on a drawn line: the price crossing it (either way), the line followed through time.
    #[test]
    fn alerte_sur_un_dessin() {
        let b = |t: i64, c: f64| Bar {
            time: t,
            open: c,
            high: c,
            low: c,
            close: c,
            volume: 1.0,
        };
        // a rising line 100 → 110 over 10 bars; the price stays at 104: under the line late, over it early
        let line = Src::Trend {
            t0: 0,
            p0: 100.0,
            t1: 10,
            p1: 110.0,
        };
        let c = |op| {
            vec![Cond {
                l: Src::Close,
                op,
                r: line,
                r2: None,
            }]
        };
        let bars: Vec<Bar> = (0..8).map(|t| b(t, 104.0)).collect();
        // at bar 4 the line is at 104, then above: the close (104) crosses it downwards between bars 4 and 5
        assert!(eval(true, &c(Op::Cross), &bars[..7]), "crosses");
        assert!(eval(true, &c(Op::CrossDown), &bars[..7]));
        assert!(!eval(true, &c(Op::CrossUp), &bars[..7]));
        // a horizontal line
        let h = Src::Trend {
            t0: 3,
            p0: 105.0,
            t1: 3,
            p1: 0.0,
        };
        assert_eq!(h.series(&bars, &|_| None)[7], Some(105.0));
        assert_eq!(h.label(), "Dessin");
    }
    use super::*;

    fn bars(closes: &[f64]) -> Vec<Bar> {
        closes
            .iter()
            .enumerate()
            .map(|(i, &c)| Bar {
                time: i as i64 * 3600,
                open: c,
                high: c + 1.0,
                low: c - 1.0,
                close: c,
                volume: 10.0,
            })
            .collect()
    }

    #[test]
    fn prix_franchi_vers_le_haut_et_vers_le_bas() {
        let mut al = Alerts::default();
        let up = al.add("BTC", Kind::Price { price: 100.0, dir: Dir::Up }, None, None, "");
        let down = al.add("BTC", Kind::Price { price: 90.0, dir: Dir::Down }, None, None, "");
        assert!(al.check_price("BTC", 95.0, 0).is_empty(), "premier prix : rien à comparer");
        assert!(al.check_price("ETH", 200.0, 1).is_empty(), "autre symbole");
        let f = al.check_price("BTC", 101.0, 2);
        assert_eq!(f.iter().map(|f| f.id).collect::<Vec<_>>(), vec![up]);
        assert!(f[0].text.contains("monte au-dessus de 100"));
        assert!(!al.list[0].on, "une seule fois : elle se coupe");
        let f = al.check_price("BTC", 89.0, 3);
        assert_eq!(f.iter().map(|f| f.id).collect::<Vec<_>>(), vec![down]);
        assert_eq!(al.history.len(), 2);
        assert_eq!(al.history[0].id, down, "le plus récent en premier");
        // dans les deux sens
        let both = al.add("BTC", Kind::Price { price: 95.0, dir: Dir::Both }, Some(0), None, "Zone");
        al.check_price("BTC", 94.0, 4);
        assert_eq!(al.check_price("BTC", 96.0, 5)[0].id, both);
        let f = al.check_price("BTC", 94.5, 6);
        assert_eq!(f[0].id, both, "et vers le bas");
        assert!(f[0].text.starts_with("Zone · "), "message libre devant");
    }

    #[test]
    fn a_chaque_fois_avec_delai_et_expiration() {
        let mut al = Alerts::default();
        al.add("BTC", Kind::Price { price: 100.0, dir: Dir::Both }, Some(600), Some(10_000), "");
        al.check_price("BTC", 99.0, 0);
        assert_eq!(al.check_price("BTC", 101.0, 100).len(), 1);
        assert!(al.check_price("BTC", 99.0, 200).is_empty(), "délai de 10 min pas écoulé");
        assert!(al.list[0].on, "à chaque fois : reste allumée");
        assert_eq!(al.check_price("BTC", 101.0, 800).len(), 1, "après le délai");
        al.check_price("BTC", 99.0, 20_000);
        assert!(al.check_price("BTC", 101.0, 20_700).is_empty(), "expirée");
        al.expire(20_700);
        assert!(!al.list[0].on);
        assert!(al.symbols().is_empty());
    }

    #[test]
    fn conditions_et_ou() {
        // le prix monte : la clôture croise au-dessus de 105 sur la dernière bougie fermée
        let b = bars(&[100.0, 102.0, 104.0, 106.0, 107.0]);
        let x = Cond {
            l: Src::Close,
            op: Op::CrossUp,
            r: Src::Num(105.0),
            r2: None,
        };
        let low = Cond {
            l: Src::Close,
            op: Op::Below,
            r: Src::Num(50.0),
            r2: None,
        };
        assert!(eval(true, &[x], &b));
        assert!(!eval(true, &[x, low], &b), "ET : les deux doivent être vraies");
        assert!(eval(false, &[x, low], &b), "OU : une suffit");
        assert!(!eval(
            true,
            &[Cond {
                l: Src::Close,
                op: Op::CrossDown,
                r: Src::Num(105.0),
                r2: None
            }],
            &b
        ));
        // RSI d'une montée continue : 100 ; EMA 3 sous le prix
        let up = bars(&(0..20).map(|i| 100.0 + i as f64).collect::<Vec<_>>());
        assert!(eval(
            true,
            &[Cond {
                l: Src::Rsi,
                op: Op::Above,
                r: Src::Num(70.0),
                r2: None
            }],
            &up
        ));
        assert!(eval(
            true,
            &[Cond {
                l: Src::Close,
                op: Op::Above,
                r: Src::Ema(3),
                r2: None
            }],
            &up
        ));
        assert!(
            !eval(
                true,
                &[Cond {
                    l: Src::VolMa,
                    op: Op::Above,
                    r: Src::Num(0.0),
                    r2: None
                }],
                &up[..10]
            ),
            "pas encore 20 bougies"
        );
        // une fois par bougie fermée
        let mut al = Alerts::default();
        al.add(
            "BTC",
            Kind::Multi {
                tf: 3600,
                all: true,
                conds: vec![x],
            },
            Some(0),
            None,
            "",
        );
        assert_eq!(al.check_bars("BTC", 3600, &b, 0).len(), 1);
        assert!(al.check_bars("BTC", 3600, &b, 10).is_empty(), "même bougie : pas deux fois");
        assert!(al.check_bars("BTC", 900, &b, 10).is_empty(), "autre unité de temps");
        assert!(al.list[0].text().contains("Prix croise au-dessus de 105"));
    }

    #[test]
    fn sauvegarde_et_rechargement() {
        let mut al = Alerts::default();
        al.add("XAUUSD", Kind::Price { price: 4000.0, dir: Dir::Down }, None, Some(99), "Or");
        al.add(
            "BTC",
            Kind::Multi {
                tf: 14_400,
                all: false,
                conds: vec![Cond {
                    l: Src::Rsi,
                    op: Op::Below,
                    r: Src::Num(30.0),
                    r2: None,
                }],
            },
            Some(3600),
            None,
            "",
        );
        al.check_price("XAUUSD", 4010.0, 1);
        let json = serde_json::to_string(&al).unwrap();
        let back: Alerts = serde_json::from_str(&json).unwrap();
        assert_eq!(back, al);
        assert_eq!(back.lines("XAUUSD"), vec![(1, 4000.0)]);
        let empty: Alerts = serde_json::from_str("{}").unwrap();
        assert!(empty.list.is_empty(), "fichier vide : rien");
        let mut m = back;
        m.move_line(1, 3990.0);
        assert_eq!(m.lines("XAUUSD"), vec![(1, 3990.0)]);
    }

    /// 🔔 A 2-condition alert reads « 2 conditions » with the arrow of its price's condition (↘ below, ↗ above);
    /// a crossing keeps ↕; a price alert keeps its own sense.
    #[test]
    fn fleche_du_prix_des_alertes_a_conditions() {
        let cond = |l: Src, op: Op, r: Src| Cond { l, op, r, r2: None };
        let multi = |conds: Vec<Cond>| Kind::Multi { tf: 3600, all: true, conds };
        let mut al = Alerts::default();
        let below = cond(Src::Close, Op::Below, Src::Num(420.0));
        let above = cond(Src::Close, Op::Above, Src::Num(452.0));
        let cross = cond(Src::Close, Op::Cross, Src::Num(430.0));
        al.add("ZEC", multi(vec![cond(Src::Rsi, Op::Above, Src::Close), below]), None, None, "");
        al.add("ZEC", multi(vec![above, cond(Src::Volume, Op::Above, Src::Num(9.0))]), None, None, "");
        al.add("ZEC", multi(vec![cross, cond(Src::Rsi, Op::Below, Src::Num(30.0))]), None, None, "");
        al.add("BTC", Kind::Price { price: 61_500.0, dir: Dir::Up }, None, None, "");
        assert_eq!(al.list[0].sens(), (Sens::Conds(2), Some(420.0)));
        assert_eq!(al.list[0].sens_du_prix(), Sens::Down, "price below 420: ↘");
        assert_eq!(al.list[1].sens_du_prix(), Sens::Up, "price above 452: ↗");
        assert_eq!(al.list[2].sens_du_prix(), Sens::Cross, "crossing: ↕");
        assert_eq!(al.list[3].sens_du_prix(), Sens::Up);
    }

    #[test]
    fn alerte_sur_une_ligne_d_indicateur_et_zones() {
        let b = bars(&[1.0, 1.0, 1.0, 1.0, 1.0, 1.0]);
        // la ligne « Osc · K » : 20, 25, 35, 50, 75, (bougie en cours) 90 ; la dernière fermée = 75
        let id = line_id("Osc · K");
        assert_ne!(id, line_id("Osc · D"), "noms différents, identifiants différents");
        let ext = |q: u64| (q == id).then(|| vec![10.0, 20.0, 25.0, 50.0, 75.0, 90.0]);
        let c = |op: Op, r: f64, r2: Option<f64>| Cond {
            l: Src::Line(id),
            op,
            r: Src::Num(r),
            r2: r2.map(Src::Num),
        };
        assert!(eval_with(true, &[c(Op::CrossUp, 70.0, None)], &b, &ext), "75 croise 70 (50 → 75)");
        assert!(!eval_with(true, &[c(Op::CrossDown, 70.0, None)], &b, &ext));
        assert!(eval_with(true, &[c(Op::LeavesZone, 30.0, Some(70.0))], &b, &ext), "sort de 30–70 (50 → 75)");
        assert!(eval_with(true, &[c(Op::EntersZone, 60.0, Some(80.0))], &b, &ext), "entre dans 60–80");
        assert!(!eval_with(true, &[c(Op::EntersZone, 0.0, Some(100.0))], &b, &ext), "déjà dedans avant");
        assert!(!eval(true, &[c(Op::CrossUp, 70.0, None)], &b), "sans les lignes de l'app : jamais vrai");
        let mut al = Alerts::default();
        al.add(
            "BTC",
            Kind::Multi {
                tf: 3600,
                all: true,
                conds: vec![c(Op::LeavesZone, 30.0, Some(70.0))],
            },
            None,
            None,
            "",
        );
        al.list[0].lines = vec![(id, "Osc · K".into())];
        assert_eq!(al.list[0].text(), "BTC 1h : Osc · K sort de la zone 30 – 70");
        assert_eq!(al.check_bars_with("BTC", 3600, &b, 10, &ext).len(), 1);
    }

    /// 🔊 An alerts.json from before sounds (no `son` field) still reads: each alert takes the default sound.
    #[test]
    fn ancien_fichier_sans_son() {
        let json = r#"{"list":[{"id":1,"sym":"BTC","kind":{"Price":{"price":80000.0,"dir":"Both"}},
            "every":null,"until":null,"msg":"","on":true,"fired":null}],"history":[],"next":1}"#;
        let al: Alerts = serde_json::from_str(json).unwrap();
        assert_eq!(al.list.len(), 1);
        assert_eq!(al.list[0].son, None);
    }

    /// 🔊 An alert's sound (and « Son coupé ») is kept after saving and reading back.
    #[test]
    fn son_garde_apres_relecture() {
        let mut al = Alerts::default();
        let a = al.add("BTC", Kind::Price { price: 1.0, dir: Dir::Up }, None, None, "");
        let b = al.add("BTC", Kind::Price { price: 2.0, dir: Dir::Up }, None, None, "");
        al.list[0].son = Some("Cloche".into());
        al.list[1].son = Some(SON_COUPE.into());
        let back: Alerts = serde_json::from_str(&serde_json::to_string(&al).unwrap()).unwrap();
        assert_eq!(back.list.iter().find(|x| x.id == a).unwrap().son.as_deref(), Some("Cloche"));
        assert_eq!(back.list.iter().find(|x| x.id == b).unwrap().son.as_deref(), Some(SON_COUPE));
    }

    /// 🔊 Which sound alerts fired together play: the first one with its own sound, else the default; silent
    /// only when all are « Son coupé »; an unknown name plays the default.
    #[test]
    fn son_des_alertes_declenchees() {
        assert_eq!(son_a_jouer([None, Some("Gong doux"), Some("Cloche")], "Bip"), "Gong doux");
        assert_eq!(son_a_jouer([Some(SON_COUPE), Some("Montée")], "Bip"), "Montée");
        assert_eq!(son_a_jouer([None, None], "Carillon"), "Carillon");
        assert_eq!(son_a_jouer([Some(SON_COUPE), None], "Carillon"), "Carillon");
        assert_eq!(son_a_jouer([Some(SON_COUPE), Some(SON_COUPE)], "Bip"), SON_COUPE);
        assert_eq!(son_a_jouer([Some("inconnu")], "Cloche"), SON_DEFAUT);
        assert_eq!(son_a_jouer([None], "inconnu"), SON_DEFAUT);
        assert_eq!(SONS.len(), 8);
        assert_eq!(SON_DEFAUT, "Bip");
        let c = choix_sons();
        assert_eq!(c.len(), 10);
        assert_eq!((c[0], c[1], c[9]), (("Par défaut", None), ("Bip", Some("Bip")), ("Son coupé", Some(SON_COUPE))));
    }

    /// 🔔 The alerts panel's history: a firing keeps the symbol, market, time frame, sense, the price aimed at and the
    /// TRUE price of the moment (exact decimals); a candle alert keeps the closed candle's close.
    #[test]
    fn historique_garde_le_vrai_prix() {
        use std::str::FromStr;
        let d = |s: &str| rust_decimal::Decimal::from_str(s).unwrap();
        let mut al = Alerts::default();
        let id = al.add(
            "BTC",
            Kind::Price {
                price: 58900.0,
                dir: Dir::Down,
            },
            None,
            None,
            "",
        );
        al.list[0].coin = "binance:BTCUSDT".into();
        al.list[0].tf = Some(3600);
        assert!(al.check_price("BTC", 59010.0, 100).is_empty());
        let f = al.check_price("BTC", 58897.5, 200);
        assert_eq!(f.len(), 1);
        let h = &al.declenchements[0];
        assert_eq!(
            (h.id, h.ts, h.sym.as_str(), h.coin.as_str(), h.tf, h.every),
            (id, 200, "BTC", "binance:BTCUSDT", Some(3600), false)
        );
        assert_eq!(al.history[0].id, id, "the notifications feed too");
        assert_eq!(h.sens, Sens::Down);
        assert_eq!((h.target, h.price), (Some(d("58900")), Some(d("58897.5"))));
        // a candle alert: the close of the closed candle
        let b = |t: i64, c: f64| Bar {
            time: t,
            open: c,
            high: c,
            low: c,
            close: c,
            volume: 1.0,
        };
        let conds = vec![Cond {
            l: Src::Close,
            op: Op::Above,
            r: Src::Num(452.0),
            r2: None,
        }];
        al.add("ZEC", Kind::Multi { tf: 300, all: true, conds }, Some(60), None, "");
        al.check_bars("ZEC", 300, &[b(0, 450.0), b(300, 452.35), b(600, 453.0)], 900);
        let h = &al.declenchements[0];
        assert_eq!((h.sym.as_str(), h.tf, h.sens), ("ZEC", Some(300), Sens::Up));
        assert_eq!((h.target, h.price), (Some(d("452")), Some(d("452.35"))));
        // kept as text in alerts.json, read back exactly
        let back: Alerts = serde_json::from_str(&serde_json::to_string(&al).unwrap()).unwrap();
        assert_eq!(back, al);
        assert!(serde_json::to_string(&al).unwrap().contains("\"58897.5\""));
    }

    /// 🔔 The history keeps the 500 last firings (the newest first); « Vider l'historique » empties it, the alerts stay.
    #[test]
    fn historique_limite_a_500_et_vider() {
        let mut al = Alerts::default();
        al.add("BTC", Kind::Price { price: 100.0, dir: Dir::Both }, Some(0), None, "");
        let mut px = 99.0;
        al.check_price("BTC", px, 0);
        for t in 1..=520 {
            px = if px < 100.0 { 101.0 } else { 99.0 };
            assert_eq!(al.check_price("BTC", px, t).len(), 1);
        }
        assert_eq!(al.declenchements.len(), HISTORY);
        assert_eq!(
            (al.declenchements[0].ts, al.declenchements[HISTORY - 1].ts),
            (520, 21),
            "the newest first, the oldest dropped"
        );
        al.clear_history();
        assert!(al.declenchements.is_empty() && al.history.is_empty());
        assert_eq!(al.list.len(), 1, "the alerts stay");
    }

    /// 🔔 An alerts.json saved before the panel (no market, time frame, creation date; a history with only id, ts and
    /// text) reads back without loss: the new fields take their defaults.
    #[test]
    fn ancien_fichier_relu_sans_perte() {
        let ancien = r#"{"list":[{"id":3,"sym":"BTC","kind":{"Price":{"price":61500.0,"dir":"Up"}},"every":null,
            "until":null,"msg":"cassure","on":false,"fired":null,"seen":61000.0,"lines":[],"son":"Cloche"},
            {"id":4,"sym":"ETH","kind":{"Multi":{"tf":14400,"all":true,"conds":[{"l":"Close","op":"Below","r":{"Num":1980.0}}]}},
            "every":60,"until":null,"msg":"","on":true,"fired":1700000000,"seen":null}],
            "history":[{"id":4,"ts":1700000000,"text":"ETH 4h : Clôture plus petit que 1980"}],"next":4}"#;
        let al: Alerts = serde_json::from_str(ancien).unwrap();
        assert_eq!(al.list.len(), 2);
        let a = &al.list[0];
        assert_eq!(
            (a.id, a.sym.as_str(), a.msg.as_str(), a.on, a.son.as_deref()),
            (3, "BTC", "cassure", false, Some("Cloche"))
        );
        assert_eq!((a.coin.as_str(), a.tf, a.created), ("", None, None));
        assert_eq!(a.sens(), (Sens::Up, Some(61500.0)));
        assert_eq!(al.list[1].sens(), (Sens::Down, Some(1980.0)));
        let h = &al.history[0];
        assert_eq!((h.id, h.ts, h.text.as_str()), (4, 1700000000, "ETH 4h : Clôture plus petit que 1980"));
        assert!(al.declenchements.is_empty(), "no panel history yet: nothing lost, nothing made up");
        assert_eq!(al.next, 4);
    }
}
