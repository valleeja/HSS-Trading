//! 📓 Journal de trading, **comme Argos v1** : panneau de droite avec 📋 Trades · 📊 Stats · 🏦 PropFirm et « + Trade »,
//! périodes (7 j · 30 j · 90 j · Tout), filtres stratégie et symbole ; tableau des trades (clic = modifier), export
//! CSV, sauvegarde et import JSON (au format de v1) ; 12 statistiques, courbe de capital et tableaux par stratégie,
//! session, symbole, émotion et jour ; règles de prop firm (jauges, avertissements, réglages du compte).
//! La fiche d'un trade (`JForm`) est une fenêtre par-dessus l'app, avec les mêmes champs que v1.
//!
//! Comme les autres panneaux : une table de zones (`items`) lue à la fois par le dessin et par le clic.

use crate::palette as pal;
use crate::palette::{GREY, WHITE};
use crate::sessions::SESSIONS;
use crate::settings::tz_offset;
use crate::shapes::{lines, poly_fill, round_fill, round_stroke, Rect};
use crate::text::{Align, Rgb, Text};
use argos_core::journal::{group_by, propfirm, stats, Deverrouillage, Group, JCfg, JTrade, PropState, Stats, CONF, EMOS};
use argos_core::Money;
use serde::{Deserialize, Serialize};
use tiny_skia::Pixmap;

const PAD: f64 = 16.0;
const TOP: f64 = 44.0;
/// Fuseau des dates du journal (comme `nyTime` de v1).
const TZ: &str = "America/New_York";
const DAY_MS: i64 = 86_400_000;
const TABS: [&str; 3] = ["📋 Trades", "📊 Stats", "🏦 PropFirm"];
const PERIODS: [(&str, i64); 4] = [("7 j", 7), ("30 j", 30), ("90 j", 90), ("Tout", 0)];
const FOOT: [&str; 4] = ["⬇️ Export CSV", "💾 Sauvegarde", "📂 Importer", "🆘 Code de secours du journal"];
/// 🆘 The rescue code button exists only in the desktop app, which reaches the keyring directly; never in the browser
/// page (the key never goes through the browser).
pub const SECOURS_ICI: bool = cfg!(not(target_arch = "wasm32"));
const WD: [&str; 7] = ["lun.", "mar.", "mer.", "jeu.", "ven.", "sam.", "dim."];
const MONTHS: [&str; 12] = [
    "janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.", "oct.", "nov.", "déc.",
];
/// 🔓 Weeks shown in the discipline review (the most recent).
const SEMAINES: usize = 8;

/// 🔓 Lines of the discipline review (text, colour, indent): per week, how many unlocks, then each reason.
fn discipline(liste: &[Deverrouillage]) -> Vec<(String, Rgb, f64)> {
    let semaines = argos_core::journal::bilan_discipline(liste);
    if semaines.is_empty() {
        return vec![("Aucun déverrouillage : les règles ont tenu.".into(), crate::palette::up(), 0.0)];
    }
    let mut out = vec![];
    for s in semaines.iter().take(SEMAINES) {
        let date = {
            let mut p = s.lundi.splitn(3, '-');
            let (a, m, j) = (p.next().unwrap_or(""), p.next().and_then(|m| m.parse::<usize>().ok()), p.next().unwrap_or(""));
            let mois = m.and_then(|m| MONTHS.get(m.wrapping_sub(1))).copied().unwrap_or("");
            format!("{} {mois} {a}", j.trim_start_matches('0'))
        };
        let n = s.raisons.len();
        // one unlock in a week: noted; two or more: orange
        let c = if n > 1 { crate::palette::ORANGE } else { WHITE };
        out.push((format!("Semaine du {date} : {n} déverrouillage{}", if n > 1 { "s" } else { "" }), c, 0.0));
        out.extend(s.raisons.iter().map(|r| (format!("• « {r} »"), GREY, 12.0)));
    }
    out
}

/// Tailles de compte proposées (− / +).
const ACCOUNTS: [f64; 6] = [10_000.0, 25_000.0, 50_000.0, 100_000.0, 200_000.0, 400_000.0];
/// Réglages du compte de prop firm : nom affiché (dans l'ordre de v1).
const CFG: [&str; 8] = [
    "Taille du compte $",
    "Phase",
    "Perte max du jour %",
    "Perte max totale %",
    "Jours minimum",
    "Mes trades max / jour",
    "Risque par trade $",
    "Début du challenge",
];
const NOTE: &str = "Règles prop firm standard pré-remplies (5 % / jour, 10 % au total, objectif 10 % puis 5 %). Vérifie-les sur ton compte de prop firm : elles changent selon le type de compte.";

/// Ce qu'un clic dans le journal demande à la fenêtre.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JClick {
    None,
    Changed,
    /// « + Trade » : nouvelle fiche.
    New,
    /// Modifier ce trade (index dans `trades`).
    Edit(usize),
    /// Export : `true` = CSV (trades filtrés), `false` = sauvegarde JSON (tout).
    Export(bool),
    Import,
    /// 🆘 The rescue code of the journal key (desktop app only, `SECOURS_ICI`).
    Secours,
}

/// Zone (cliquable ou dessinée seulement).
#[derive(Debug, Clone, PartialEq)]
enum It {
    Tab(usize),
    New,
    Period(usize),
    /// Filtre : 0 stratégie, 1 symbole ; puis une option de sa liste ouverte (0 = toutes).
    Pick(u8),
    Opt(u8, usize),
    Text(String, Rgb),
    Head,
    Row(usize),
    Foot(usize),
    Kpi(usize),
    Title(String),
    Curve,
    GHead,
    GRow(usize, usize),
    Bar(usize),
    Cfg(usize),
    Step(usize, i8),
}

/// État du journal (gardé dans journal.json par la fenêtre).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Journal {
    /// 0 Trades, 1 Stats, 2 PropFirm.
    pub tab: usize,
    /// Index dans `PERIODS` (3 = Tout, comme v1).
    pub period: usize,
    pub strat: Option<String>,
    pub sym: Option<String>,
    pub trades: Vec<JTrade>,
    pub cfg: JCfg,
    #[serde(skip)]
    menu: Option<u8>,
    /// 🛡️ Jour et plus haut palier déjà annoncés par le gardien (un avis par palier et par jour).
    #[serde(skip)]
    warned: (String, GuardLevel),
}

impl Default for Journal {
    fn default() -> Self {
        Self {
            tab: 0,
            period: 3,
            strat: None,
            sym: None,
            trades: vec![],
            cfg: JCfg::default(),
            menu: None,
            warned: Default::default(),
        }
    }
}

/// Maintenant, en millisecondes.
pub fn now_ms() -> i64 {
    web_time::SystemTime::now()
        .duration_since(web_time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as i64)
}

/// Date et heure de New York : (année, mois 1-12, jour, heure, minute, jour de semaine 0 = lundi).
fn ny(ms: i64) -> (i32, u32, u32, u32, u32, usize) {
    use chrono::{Datelike, Timelike};
    let s = ms.div_euclid(1000);
    let d = chrono::DateTime::from_timestamp(s + tz_offset(TZ, s).unwrap_or(0), 0).unwrap_or_default();
    (d.year(), d.month(), d.day(), d.hour(), d.minute(), d.weekday().num_days_from_monday() as usize)
}

/// Jour « AAAA-MM-JJ » dans le fuseau `tz` (le gardien : minuit dans le fuseau du reset).
fn day_key(ms: i64, tz: &str) -> String {
    let s = ms.div_euclid(1000);
    use chrono::Datelike;
    chrono::DateTime::from_timestamp(s + tz_offset(tz, s).unwrap_or(0), 0)
        .map(|d| format!("{}-{:02}-{:02}", d.year(), d.month(), d.day()))
        .unwrap_or_default()
}

/// « 2026-10-26 14:30 » (New York) ; c'est aussi ce qu'on tape dans la fiche.
pub fn date_txt(ms: i64) -> String {
    let (y, m, d, h, mi, _) = ny(ms);
    format!("{y}-{m:02}-{d:02} {h:02}:{mi:02}")
}

/// Lit « AAAA-MM-JJ[ HH:MM] » (heure de New York) → millisecondes.
pub fn parse_date(s: &str) -> Option<i64> {
    let s = s.trim();
    let dt = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M")
        .or_else(|_| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").map(|d| d.and_hms_opt(0, 0, 0).unwrap_or_default()))
        .ok()?;
    let local = dt.and_utc().timestamp();
    // le décalage de l'heure visée (essai, puis correction au changement d'heure)
    let guess = local - tz_offset(TZ, local).unwrap_or(0);
    Some((local - tz_offset(TZ, guess).unwrap_or(0)) * 1000)
}

/// « −1 234 $ » (sans décimales, comme v1), « — » si rien.
pub fn money(v: Option<f64>) -> String {
    let Some(v) = v.filter(|v| v.is_finite()) else { return "—".into() };
    let digits = format!("{:.0}", v.abs());
    let mut out = String::new();
    for (k, c) in digits.chars().enumerate() {
        if k > 0 && (digits.len() - k) % 3 == 0 {
            out.push(' ');
        }
        out.push(c);
    }
    format!("{}{out} $", if v < 0.0 && digits != "0" { "−" } else { "" })
}

/// « +1.23 R ».
pub fn r_txt(r: Option<f64>) -> String {
    match r.filter(|v| v.is_finite()) {
        Some(r) => format!("{}{:.2} R", if r >= 0.0 { "+" } else { "−" }, r.abs()),
        None => "—".into(),
    }
}

/// Session de l'heure `ms` (comme `sessOf` de v1).
pub fn sess_of(ms: i64) -> &'static str {
    let on = |name: &str| SESSIONS.iter().any(|s| s.name == name && s.is_open(ms.div_euclid(1000)));
    match (on("Londres"), on("New York")) {
        (true, true) => "Londres + NY",
        (_, true) => "New York",
        (true, _) => "Londres",
        _ if on("Tokyo") => "Tokyo",
        _ if on("Sydney") => "Sydney",
        _ => "Hors session",
    }
}

fn emo_label(k: &str) -> &'static str {
    EMOS.iter().find(|e| e.0 == k).map_or("—", |e| e.1)
}

fn tone(v: Option<f64>) -> Rgb {
    match v {
        Some(v) if v > 0.0 => crate::palette::up(),
        Some(v) if v < 0.0 => crate::palette::down(),
        _ => WHITE,
    }
}

/// Coupe `s` en lignes de largeur `max` (mesure avec `text`).
fn wrap(text: &Text, s: &str, size: f32, max: f64) -> Vec<String> {
    let mut out = vec![String::new()];
    for word in s.split(' ') {
        let last = out.last_mut().unwrap();
        let next = if last.is_empty() { word.to_string() } else { format!("{last} {word}") };
        if text.width(&next, size) as f64 > max && !last.is_empty() {
            out.push(word.to_string());
        } else {
            *last = next;
        }
    }
    out
}

/// Ce que montre le journal (calculé une fois par dessin ou par clic).
struct View {
    /// Trades filtrés (index dans `trades`), les plus récents en premier.
    rows: Vec<usize>,
    stats: Stats,
    sub: Vec<JTrade>,
    groups: Vec<(&'static str, Vec<Group>)>,
    prop: PropState,
    syms: Vec<String>,
}

impl Journal {
    /// 🛡️ Jour de trading de l'heure `ms` : il finit à minuit dans le fuseau du reset (`cfg.reset_tz`).
    pub fn propfirm_day(&self, ms: i64) -> String {
        day_key(ms, self.reset_tz())
    }

    fn reset_tz(&self) -> &str {
        if self.cfg.reset_tz.is_empty() {
            "Europe/Prague"
        } else {
            &self.cfg.reset_tz
        }
    }

    /// 🛡️ La journée est-elle verrouillée par le gardien (limite touchée aujourd'hui, heure du reset) ?
    pub fn locked(&self, now: i64) -> bool {
        !self.cfg.lock.is_empty() && self.cfg.lock == self.propfirm_day(now)
    }

    pub(crate) fn propfirm_now(&self, now: i64) -> PropState {
        let since = if self.cfg.start.is_empty() {
            0
        } else {
            parse_date(&self.cfg.start).unwrap_or(0)
        };
        propfirm(&self.trades, &self.cfg, now, since, &|ms| self.propfirm_day(ms), &|v| money(Some(v)))
    }

    /// 🛡️ Gardien PropFirm avant un ordre démo de risque `risk` ($), avec `open` positions démo déjà ouvertes,
    /// `has_sl` (l'ordre a un stop loss) et `news` (la news à fort impact en cours sur ce symbole, s'il y en a une) :
    /// `Some(raison)` si la journée est verrouillée, si une pause anti-tilt est en cours, si une news bloque, si le stop manque alors qu'il est obligatoire,
    /// si le trade risque plus que ton % max du compte, si tu as déjà ton nombre max de positions, s'il ferait dépasser
    /// la perte max du jour ou totale (perte réalisée + risque déjà ouvert + ce risque), ou ta limite de trades par jour.
    pub fn guard(&self, risk: f64, now: i64, open: usize, has_sl: bool, news: Option<&str>) -> Option<String> {
        if self.locked(now) {
            return Some("journée verrouillée : une limite a été touchée aujourd'hui, reprise à minuit (heure du reset)".into());
        }
        if let Some(min) = self.cfg.pause_left(now) {
            return Some(format!("pause anti-tilt : encore {min} min"));
        }
        if let Some(n) = news.filter(|_| self.cfg.news) {
            return Some(format!("news à fort impact en cours ({n}) : aucun ordre 2 min avant → 2 min après"));
        }
        if self.cfg.need_sl && !has_sl {
            return Some("stop loss obligatoire".into());
        }
        let acc = self.account();
        let r_max = acc * self.cfg.risk_pct / 100.0;
        if self.cfg.risk_pct > 0.0 && risk > r_max * 1.001 {
            return Some(format!(
                "risque de {} > {} permis ({} % du compte)",
                money(Some(risk)),
                money(Some(r_max)),
                self.cfg.risk_pct
            ));
        }
        if self.cfg.max_pos > 0 && open as u32 >= self.cfg.max_pos {
            return Some(format!("{open} positions ouvertes : ta limite est {}", self.cfg.max_pos));
        }
        let f = self.propfirm_now(now);
        // 💰 comparaisons aux limites en centimes exacts (le risque du trade arrondi une fois)
        let r = Money::from_f64(risk);
        if -f.today_pnl + f.today_open_risk + r > f.daily_limit {
            return Some(format!(
                "perte max du jour : {} perdu + {} ouvert + {} de ce trade > limite de {}",
                money(Some((-f.today_pnl).to_f64())),
                money(Some(f.today_open_risk.to_f64())),
                money(Some(r.to_f64())),
                money(Some(f.daily_limit.to_f64()))
            ));
        }
        if -f.pnl + f.today_open_risk + r > f.max_limit {
            return Some(format!("perte max totale : la limite de {} serait dépassée", money(Some(f.max_limit.to_f64()))));
        }
        if f.trades_today as u32 >= self.cfg.max_trades {
            return Some(format!("{} trades aujourd'hui : ta limite est {}", f.trades_today, self.cfg.max_trades));
        }
        None
    }

    fn account(&self) -> f64 {
        if self.cfg.account > 0.0 {
            self.cfg.account
        } else {
            100_000.0
        }
    }

    /// 🛡️ Surveillance du gardien : avec `floating` ($, gain ou perte des positions encore ouvertes), une limite
    /// est-elle touchée ? `Some("perte du jour" | "perte maximale")`.
    pub fn breach(&self, floating: f64, now: i64) -> Option<&'static str> {
        let f = self.propfirm_now(now);
        let floating = Money::from_f64(floating);
        if -(f.today_pnl + floating) >= f.daily_limit {
            Some("perte du jour")
        } else if -(f.pnl + floating) >= f.max_limit {
            Some("perte maximale")
        } else {
            None
        }
    }

    /// 🛡️ État du gardien pour la carte au-dessus du ticket (`floating` : P&L des positions ouvertes, `open` : leur
    /// nombre).
    pub fn guard_card(&self, floating: f64, open: usize, now: i64) -> GuardCard {
        let f = self.propfirm_now(now);
        let floating = Money::from_f64(floating);
        GuardCard {
            day_loss: (-(f.today_pnl + floating)).max(Money::ZERO).to_f64(),
            day_lim: f.daily_limit.to_f64(),
            tot_loss: (-(f.pnl + floating)).max(Money::ZERO).to_f64(),
            max_lim: f.max_limit.to_f64(),
            open,
            max_pos: self.cfg.max_pos,
            risk_pct: self.cfg.risk_pct,
            risk_max: self.account() * self.cfg.risk_pct / 100.0,
            warn: self.cfg.warn,
            locked: self.locked(now),
        }
    }

    /// 🛡️ Pilule du Gardien (barre du haut, maquette verre) : « perte du jour / limite du jour » en % du compte
    /// (« 0,8 % / 5 % »), positions ouvertes comprises, et le niveau (couleur).
    pub fn pilule(&self, floating: f64, open: usize, now: i64) -> (String, GuardLevel) {
        let c = self.guard_card(floating, open, now);
        let pct = |v: f64| {
            let s = crate::viewport::format_number(v / self.account() * 100.0, 1);
            s.strip_suffix(",0").or_else(|| s.strip_suffix(".0")).map_or(s.clone(), str::to_string)
        };
        (format!("{} % / {} %", pct(c.day_loss), pct(c.day_lim)), c.level())
    }

    /// 🛡️ Avertissement du jour : `Some(%)` la première fois aujourd'hui que la perte atteint un palier
    /// (`GUARD_HALF` %, puis `cfg.warn` %) d'une limite ; un avis par palier et par jour (noté dans `warned`).
    pub fn warn_once(&mut self, floating: f64, open: usize, now: i64) -> Option<f64> {
        let card = self.guard_card(floating, open, now);
        let level = card.level();
        let day = self.propfirm_day(now);
        if self.warned.0 != day {
            self.warned = (day, GuardLevel::Ok);
        }
        if matches!(level, GuardLevel::Half | GuardLevel::Warn) && level > self.warned.1 {
            self.warned.1 = level;
            Some(card.pct())
        } else {
            None
        }
    }

    /// 🔓 Déverrouille la journée (le bouton de la carte) : il faut une raison (non vide, `RAISON_MAX` caractères au
    /// plus), notée avec le jour et l'heure (`now`, ms) ; sans elle, rien ne change et la journée reste verrouillée.
    pub fn unlock(&mut self, raison: &str, now: i64) -> Result<(), String> {
        let raison = raison.trim();
        if raison.is_empty() {
            return Err("Écris une raison pour déverrouiller (obligatoire)".into());
        }
        if raison.chars().count() > RAISON_MAX {
            return Err(format!("🔒 Raison trop longue ({RAISON_MAX} caractères au plus)"));
        }
        self.cfg.lock.clear();
        let d = Deverrouillage {
            jour: self.propfirm_day(now),
            ms: now,
            raison: raison.into(),
        };
        // 🔓 kept for the discipline review too (the oldest go when the list is full)
        self.cfg.deverrouillages.push(d.clone());
        let trop = self.cfg.deverrouillages.len().saturating_sub(argos_core::journal::DEVERROUILLAGES_MAX);
        self.cfg.deverrouillages.drain(..trop);
        self.cfg.deverrouillage = Some(Box::new(d));
        Ok(())
    }

    /// 🔓 La raison si la journée a été déverrouillée aujourd'hui (jour du reset) et ne s'est pas reverrouillée
    /// depuis ; sinon `None` (fiche : « 🔓 Déverrouillée aujourd'hui · raison notée »).
    pub fn deverrouillee(&self, now: i64) -> Option<&str> {
        let d = self.cfg.deverrouillage.as_ref()?;
        (d.jour == self.propfirm_day(now) && !self.locked(now)).then_some(d.raison.as_str())
    }
}

/// 🔓 Longueur maximale de la raison d'un déverrouillage (caractères, sans les espaces autour).
pub const RAISON_MAX: usize = 200;

/// 🛡️ Premier palier du gardien : la moitié d'une limite de perte (jaune).
pub const GUARD_HALF: f64 = 50.0;

/// 🛡️ Niveau de la carte du gardien, du plus calme au plus grave.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum GuardLevel {
    /// Dans les règles.
    #[default]
    Ok,
    /// `GUARD_HALF` % d'une limite atteint : jaune.
    Half,
    /// `warn` % d'une limite atteint : bordure orange.
    Warn,
    /// Limite atteinte ou journée verrouillée : bordure rouge.
    Bad,
}

/// 🛡️ Ce que montre la carte du gardien.
#[derive(Debug, Clone, PartialEq)]
pub struct GuardCard {
    pub day_loss: f64,
    pub day_lim: f64,
    pub tot_loss: f64,
    pub max_lim: f64,
    pub open: usize,
    pub max_pos: u32,
    pub risk_pct: f64,
    pub risk_max: f64,
    pub warn: f64,
    pub locked: bool,
}

impl GuardCard {
    /// Plus haut % atteint des deux limites de perte.
    pub fn pct(&self) -> f64 {
        let p = |v: f64, l: f64| if l > 0.0 { v / l * 100.0 } else { 0.0 };
        p(self.day_loss, self.day_lim).max(p(self.tot_loss, self.max_lim))
    }

    /// Paliers 50 / `warn` / 100 % (un `warn` ≤ 50 l'emporte : pas de jaune).
    pub fn level(&self) -> GuardLevel {
        let p = self.pct();
        if self.locked || p >= 100.0 {
            GuardLevel::Bad
        } else if self.warn > 0.0 && p >= self.warn {
            GuardLevel::Warn
        } else if p >= GUARD_HALF {
            GuardLevel::Half
        } else {
            GuardLevel::Ok
        }
    }

    /// Les 4 lignes de la carte : (libellé, valeur).
    pub fn rows(&self) -> [(&'static str, String); 4] {
        let m = |v: f64| money(Some(v));
        [
            ("Perte du jour", format!("{} / {}", m(self.day_loss), m(self.day_lim))),
            ("Perte max", format!("{} / {}", m(self.tot_loss), m(self.max_lim))),
            ("Positions", format!("{} / {}", self.open, self.max_pos)),
            ("Risque max", format!("{} % ({})", self.risk_pct, m(self.risk_max))),
        ]
    }
}

impl Journal {
    fn view(&self, now: i64) -> View {
        let lim = PERIODS[self.period.min(3)].1;
        let mut rows: Vec<usize> = (0..self.trades.len())
            .filter(|&i| {
                let t = &self.trades[i];
                (lim == 0 || t.ts() >= now - lim * DAY_MS) && self.sym.as_ref().is_none_or(|s| *s == t.sym) && self.strat.as_ref().is_none_or(|s| *s == t.strat)
            })
            .collect();
        rows.sort_by_key(|&i| std::cmp::Reverse(self.trades[i].entry_ts));
        let sub: Vec<JTrade> = rows.iter().map(|&i| self.trades[i].clone()).collect();
        let groups = vec![
            ("🧠 Par stratégie", group_by(&sub, |t| t.strat.clone())),
            ("🕐 Par session", group_by(&sub, |t| sess_of(t.entry_ts).to_string())),
            ("🪙 Par symbole", group_by(&sub, |t| t.sym.clone())),
            ("😶 Par émotion", group_by(&sub, |t| emo_label(&t.emo).to_string())),
            ("📅 Par jour", group_by(&sub, |t| WD[ny(t.entry_ts).5].to_string())),
        ];
        let since = if self.cfg.start.is_empty() {
            0
        } else {
            parse_date(&self.cfg.start).unwrap_or(0)
        };
        let prop = propfirm(&self.trades, &self.cfg, now, since, &|ms| self.propfirm_day(ms), &|v| money(Some(v)));
        let mut syms: Vec<String> = vec![];
        for t in &self.trades {
            if !syms.contains(&t.sym) {
                syms.push(t.sym.clone());
            }
        }
        View {
            rows,
            stats: stats(&sub),
            sub,
            groups,
            prop,
            syms,
        }
    }

    /// Options d'un filtre (sans « toutes »).
    fn options(&self, v: &View, which: u8) -> Vec<String> {
        if which == 0 {
            self.cfg.strats.clone()
        } else {
            v.syms.clone()
        }
    }

    /// Jauges PropFirm : (nom, valeur, maximum, bonne jauge ?, texte) ; `None` = ligne « P&L compte financé ».
    fn bars(&self, f: &PropState) -> Vec<Option<(&'static str, f64, f64, bool, String)>> {
        let c = &self.cfg;
        let d = |m: Money| m.to_f64();
        let mut out = vec![if f.target > Money::ZERO {
            Some((
                "🎯 Objectif de profit",
                d(f.pnl.max(Money::ZERO)),
                d(f.target),
                true,
                format!("{} / {}", money(Some(d(f.pnl))), money(Some(d(f.target)))),
            ))
        } else {
            None
        }];
        out.push(Some((
            "📆 Perte du jour (minuit heure de Prague)",
            d((-f.today_pnl).max(Money::ZERO)),
            d(f.daily_limit),
            false,
            format!("{} / −{}", money(Some(d(f.today_pnl.min(Money::ZERO)))), money(Some(d(f.daily_limit)))),
        )));
        out.push(Some((
            "🧱 Perte maximale",
            d(-f.low),
            d(f.max_limit),
            false,
            format!("{} / −{}", money(Some(d(f.low))), money(Some(d(f.max_limit)))),
        )));
        if c.phase != 2 {
            out.push(Some((
                "🗓️ Jours de trading",
                f.days as f64,
                c.min_days as f64,
                true,
                format!("{} / {}", f.days, c.min_days),
            )));
        }
        out.push(Some((
            "✋ Trades aujourd'hui",
            f.trades_today as f64,
            c.max_trades as f64,
            false,
            format!("{} / {}", f.trades_today, c.max_trades),
        )));
        out
    }

    fn cfg_value(&self, k: usize) -> String {
        let c = &self.cfg;
        match k {
            0 => money(Some(c.account)),
            1 => [
                format!("Challenge (objectif {} %)", c.target),
                "Verification (objectif 5 %)".into(),
                "Compte financé".into(),
            ][c.phase.min(2) as usize]
                .clone(),
            2 => format!("{} %", c.daily),
            3 => format!("{} %", c.maxloss),
            4 => c.min_days.to_string(),
            5 => c.max_trades.to_string(),
            6 => money(Some(c.risk)),
            _ if c.start.is_empty() => "depuis toujours".into(),
            _ => c.start.clone(),
        }
    }

    fn step_cfg(&mut self, k: usize, d: i8, now: i64) {
        let c = &mut self.cfg;
        let df = d as f64;
        match k {
            0 => {
                let i = ACCOUNTS.iter().position(|a| *a >= c.account).unwrap_or(ACCOUNTS.len() - 1) as i64 + d as i64;
                c.account = ACCOUNTS[i.clamp(0, ACCOUNTS.len() as i64 - 1) as usize];
            }
            1 => c.phase = (c.phase as i8 + d).rem_euclid(3) as u8,
            2 => c.daily = (c.daily + 0.5 * df).clamp(0.5, 50.0),
            3 => c.maxloss = (c.maxloss + 0.5 * df).clamp(0.5, 100.0),
            4 => c.min_days = (c.min_days as i64 + d as i64).clamp(0, 60) as u32,
            5 => c.max_trades = (c.max_trades as i64 + d as i64).clamp(1, 50) as u32,
            6 => c.risk = (c.risk + 50.0 * df).max(0.0),
            _ => {
                // date du début : ± 1 jour ; vide → aujourd'hui ; après aujourd'hui → vide (depuis toujours)
                let today = day_key(now, TZ);
                c.start = match parse_date(&c.start) {
                    None => today,
                    Some(ms) => {
                        let next = day_key(ms + d as i64 * DAY_MS + DAY_MS / 2, TZ);
                        if next > today {
                            String::new()
                        } else {
                            next
                        }
                    }
                };
            }
        }
    }

    /// Zones du journal dans un panneau de largeur `pw` (sous l'en-tête du panneau).
    fn items(&self, v: &View, text: &Text, pw: f64) -> Vec<(Rect, It)> {
        let inner = pw - 2.0 * PAD;
        let mut out = vec![];
        let add_w = 84.0;
        // onglets à la largeur de leur nom, réduits ensemble s'il manque de place
        let ws: Vec<f64> = TABS.iter().map(|t| text.width(t, 13.0) as f64 + 20.0).collect();
        let k = ((inner - add_w - 18.0) / ws.iter().sum::<f64>()).min(1.0);
        let mut x = PAD;
        for (i, w) in ws.iter().enumerate() {
            out.push((Rect::new(x, TOP + 6.0, w * k, 34.0), It::Tab(i)));
            x += w * k + 6.0;
        }
        out.push((Rect::new(pw - PAD - add_w, TOP + 6.0, add_w, 34.0), It::New));
        let mut y = TOP + 50.0;
        if self.tab != 2 {
            let mut x = PAD;
            for (k, w) in [44.0, 50.0, 50.0, 54.0].iter().enumerate() {
                out.push((Rect::new(x, y, *w, 26.0), It::Period(k)));
                x += w + 6.0;
            }
            y += 34.0;
            let half = (inner - 8.0) / 2.0;
            for k in 0..2u8 {
                out.push((Rect::new(PAD + k as f64 * (half + 8.0), y, half, 30.0), It::Pick(k)));
            }
            y += 42.0;
        }
        let empty = |out: &mut Vec<(Rect, It)>, y: &mut f64, msg: &str| {
            for l in wrap(text, msg, 13.0, inner) {
                out.push((Rect::new(PAD, *y, inner, 20.0), It::Text(l, GREY)));
                *y += 20.0;
            }
            *y += 12.0;
        };
        match self.tab {
            0 => {
                if v.rows.is_empty() {
                    let msg = format!(
                        "Aucun trade{}. Clique « + Trade » pour noter un trade.",
                        if self.trades.is_empty() { "" } else { " pour ces filtres" }
                    );
                    // 📱 au doigt : « Touche « + Trade »… »
                    empty(&mut out, &mut y, &crate::actions::sans_touches(&msg));
                } else {
                    out.push((Rect::new(PAD, y, inner, 28.0), It::Head));
                    y += 28.0;
                    for &i in &v.rows {
                        out.push((Rect::new(6.0, y, pw - 12.0, 34.0), It::Row(i)));
                        y += 34.0;
                    }
                    y += 12.0;
                }
                // export / sauvegarde / importer toujours là : une sauvegarde de v1 s'importe même journal vide
                pied(&mut out, y, inner, SECOURS_ICI);
            }
            1 if v.stats.n == 0 => empty(&mut out, &mut y, "Pas encore de trades fermés pour calculer les stats."),
            1 => {
                let w = (inner - 8.0) / 2.0;
                for k in 0..12 {
                    out.push((Rect::new(PAD + (k % 2) as f64 * (w + 8.0), y + (k / 2) as f64 * 58.0, w, 50.0), It::Kpi(k)));
                }
                y += 6.0 * 58.0 + 6.0;
                out.push((Rect::new(PAD, y, inner, 28.0), It::Title("📈 Courbe de capital".into())));
                out.push((Rect::new(PAD, y + 30.0, inner, 130.0), It::Curve));
                y += 172.0;
                for (g, (title, rows)) in v.groups.iter().enumerate() {
                    out.push((Rect::new(PAD, y, inner, 28.0), It::Title(title.to_string())));
                    out.push((Rect::new(PAD, y + 28.0, inner, 24.0), It::GHead));
                    y += 52.0;
                    for r in 0..rows.len() {
                        out.push((Rect::new(PAD, y, inner, 26.0), It::GRow(g, r)));
                        y += 26.0;
                    }
                    y += 10.0;
                }
            }
            _ => {
                let f = &v.prop;
                let (msgs, c) = if f.warnings.is_empty() {
                    (vec!["✅ Dans les règles aujourd'hui".to_string()], crate::palette::up())
                } else {
                    (f.warnings.clone(), crate::palette::down())
                };
                y += 4.0;
                for m in msgs {
                    for l in wrap(text, &m, 13.0, inner - 24.0) {
                        out.push((Rect::new(PAD + 12.0, y, inner - 24.0, 20.0), It::Text(l, c)));
                        y += 20.0;
                    }
                }
                y += 22.0;
                for k in 0..self.bars(f).len() {
                    out.push((Rect::new(PAD, y, inner, 46.0), It::Bar(k)));
                    y += 50.0;
                }
                y += 8.0;
                // 🔓 discipline review: the unlocks of the guardian, week by week, with their reasons
                out.push((Rect::new(PAD, y, inner, 30.0), It::Title("🔓 Discipline : déverrouillages du Gardien".into())));
                y += 34.0;
                for (l, c, retrait) in discipline(&self.cfg.deverrouillages) {
                    for l in wrap(text, &l, 13.0, inner - retrait) {
                        out.push((Rect::new(PAD + retrait, y, inner - retrait, 20.0), It::Text(l, c)));
                        y += 20.0;
                    }
                }
                y += 14.0;
                out.push((Rect::new(PAD, y, inner, 30.0), It::Title("⚙️ Mon compte prop firm".into())));
                y += 34.0;
                for k in 0..CFG.len() {
                    // − / + avant leur case : dessinés par-dessus (la table se dessine à l'envers)
                    out.push((Rect::new(pw - PAD - 64.0, y + 4.0, 28.0, 28.0), It::Step(k, -1)));
                    out.push((Rect::new(pw - PAD - 32.0, y + 4.0, 28.0, 28.0), It::Step(k, 1)));
                    out.push((Rect::new(PAD, y, inner, 36.0), It::Cfg(k)));
                    y += 40.0;
                }
                y += 6.0;
                for l in wrap(text, NOTE, 13.0, inner) {
                    out.push((Rect::new(PAD, y, inner, 20.0), It::Text(l, GREY)));
                    y += 20.0;
                }
            }
        }
        // liste ouverte d'un filtre : par-dessus le reste (en tête : prioritaire au clic)
        if let Some(which) = self.menu {
            if let Some((r, _)) = out.iter().find(|(_, it)| *it == It::Pick(which)).cloned() {
                let n = self.options(v, which).len() + 1;
                let menu: Vec<(Rect, It)> = (0..n)
                    .map(|k| {
                        (
                            Rect::new(r.x, r.y + r.h + 2.0 + k as f64 * 30.0, r.w.max(180.0).min(pw - r.x - 6.0), 30.0),
                            It::Opt(which, k),
                        )
                    })
                    .collect();
                out.splice(0..0, menu);
            }
        }
        out
    }

    /// Hauteur du contenu (le panneau défile en dessous).
    pub fn content_h(&self, pw: f64) -> f64 {
        let v = self.view(now_ms());
        self.items(&v, &Text::default(), pw).iter().map(|(r, _)| r.y + r.h).fold(0.0, f64::max) + 20.0
    }

    /// Clic dans le panneau (coordonnées du panneau).
    pub fn click(&mut self, x: f64, y: f64, pw: f64, now: i64) -> JClick {
        let v = self.view(now);
        let hit = self
            .items(&v, &Text::default(), pw)
            .into_iter()
            .find(|(r, it)| {
                r.contains(x, y)
                    && !matches!(
                        it,
                        It::Text(..) | It::Title(_) | It::Head | It::GHead | It::Curve | It::Kpi(_) | It::GRow(..) | It::Bar(_) | It::Cfg(_)
                    )
            })
            .map(|(_, it)| it);
        let menu = self.menu.take();
        match hit {
            Some(It::Opt(which, k)) => {
                let pick = (k > 0).then(|| self.options(&v, which)[k - 1].clone());
                if which == 0 {
                    self.strat = pick
                } else {
                    self.sym = pick
                }
                JClick::Changed
            }
            Some(It::Tab(k)) => {
                self.tab = k;
                JClick::Changed
            }
            Some(It::New) => JClick::New,
            Some(It::Period(k)) => {
                self.period = k;
                JClick::Changed
            }
            Some(It::Pick(k)) => {
                self.menu = (menu != Some(k)).then_some(k);
                JClick::Changed
            }
            Some(It::Row(i)) => JClick::Edit(i),
            Some(It::Foot(k)) => [JClick::Export(true), JClick::Export(false), JClick::Import, JClick::Secours][k],
            Some(It::Step(k, d)) => {
                self.step_cfg(k, d, now);
                JClick::Changed
            }
            _ if menu.is_some() => JClick::Changed,
            _ => JClick::None,
        }
    }

    /// Dessine le journal dans le panneau (`pm` déjà peint, en-tête compris).
    pub fn draw(&self, pm: &mut Pixmap, text: &Text, pw: f64, mouse: Option<(f64, f64)>, grid: Rgb, now: i64) {
        let v = self.view(now);
        let items = self.items(&v, text, pw);
        let hover = |r: &Rect| self.menu.is_none() && mouse.is_some_and(|(x, y)| r.contains(x, y));
        let inner = pw - 2.0 * PAD;
        let chip = |pm: &mut Pixmap, r: Rect, on: bool, hov: bool, label: &str, size: f32| {
            round_fill(
                pm,
                r,
                6.0,
                if on {
                    pal::on()
                } else if hov {
                    pal::hover()
                } else {
                    pal::panel()
                },
                1.0,
            );
            round_stroke(pm, r, 6.0, if on { GREY } else { pal::edge() }, 1.0);
            text.draw(
                pm,
                &text.fit(label, size, r.w - 10.0),
                (r.cx() as f32, (r.cy() + size as f64 * 0.36) as f32),
                size,
                WHITE,
                Align::Center,
            );
        };
        // colonnes du tableau des trades (stratégie quand il y a la place, comme `hide-s` de v1)
        let wide = pw >= 440.0;
        let cols: &[(f64, &str)] = if wide {
            &[
                (0.0, "Date"),
                (0.14, "Symbole"),
                (0.30, "Sens"),
                (0.44, "Stratégie"),
                (0.68, "R"),
                (0.83, "P&L"),
            ]
        } else {
            &[(0.0, "Date"), (0.19, "Symbole"), (0.40, "Sens"), (0.60, "R"), (0.79, "P&L")]
        };
        let s = &v.stats;
        let best_worst = |k: Option<usize>| k.map_or("—".to_string(), |k| format!("{} {}", v.sub[k].sym, money(v.sub[k].pnl())));
        let kpis: [(&str, String, Rgb); 12] = [
            ("💰 P&L total", money(Some(s.total)), tone(Some(s.total))),
            ("🎯 Winrate", s.win.map_or("—".into(), |w| format!("{w:.0} %")), WHITE),
            ("📐 Espérance", r_txt(s.avg_r), tone(s.avg_r)),
            (
                "⚖️ Profit factor",
                s.pf.map_or("—".into(), |p| if p.is_infinite() { "∞".into() } else { format!("{p:.2}") }),
                WHITE,
            ),
            ("📉 Drawdown max", money(Some(-s.dd)), crate::palette::down()),
            (
                "🔢 Trades",
                if s.open > 0 {
                    format!("{} (+{} ouverts)", s.n, s.open)
                } else {
                    s.n.to_string()
                },
                WHITE,
            ),
            ("✅ Gain moyen", money(s.avg_win), crate::palette::up()),
            ("❌ Perte moyenne", money(s.avg_loss), crate::palette::down()),
            ("🔥 Série gagnante", s.best_streak.to_string(), WHITE),
            ("🧊 Série perdante", s.worst_streak.to_string(), WHITE),
            ("🏆 Meilleur", best_worst(s.best), crate::palette::up()),
            ("💀 Pire", best_worst(s.worst), crate::palette::down()),
        ];
        let bars = self.bars(&v.prop);
        for (r, it) in items.iter().rev() {
            let r = *r;
            match it {
                It::Tab(k) => chip(pm, r, self.tab == *k, hover(&r), TABS[*k], 13.0),
                It::New => {
                    round_fill(pm, r, 6.0, crate::palette::accent(), if hover(&r) { 0.85 } else { 1.0 });
                    text.draw(pm, "+ Trade", (r.cx() as f32, (r.cy() + 5.0) as f32), 14.0, WHITE, Align::Center);
                }
                It::Period(k) => chip(pm, r, self.period.min(3) == *k, hover(&r), PERIODS[*k].0, 12.5),
                It::Pick(k) => {
                    let label = match k {
                        0 => self.strat.clone().unwrap_or("Toutes les stratégies".into()),
                        _ => self.sym.clone().unwrap_or("Tous les symboles".into()),
                    };
                    round_fill(pm, r, 6.0, if hover(&r) { pal::hover() } else { pal::field() }, 1.0);
                    round_stroke(pm, r, 6.0, if self.menu == Some(*k) { GREY } else { pal::edge() }, 1.0);
                    text.draw(
                        pm,
                        &text.fit(&label, 13.0, r.w - 30.0),
                        ((r.x + 10.0) as f32, (r.cy() + 5.0) as f32),
                        13.0,
                        WHITE,
                        Align::Left,
                    );
                    lines(pm, r.x + r.w - 14.0, r.cy(), &[&[(-4.0, -2.0), (0.0, 2.0), (4.0, -2.0)]], GREY, 1.4);
                }
                It::Opt(which, k) => {
                    let label = if *k == 0 {
                        if *which == 0 {
                            "Toutes les stratégies".to_string()
                        } else {
                            "Tous les symboles".to_string()
                        }
                    } else {
                        self.options(&v, *which)[k - 1].clone()
                    };
                    let on = if *which == 0 { &self.strat } else { &self.sym };
                    let sel = on.as_deref() == (*k > 0).then_some(label.as_str());
                    crate::fill(
                        pm,
                        r.x,
                        r.y,
                        r.w,
                        r.h,
                        if mouse.is_some_and(|(x, y)| r.contains(x, y)) {
                            pal::hover()
                        } else {
                            pal::field()
                        },
                        1.0,
                    );
                    if *k == 0 {
                        crate::fill(pm, r.x, r.y - 1.0, r.w, 1.0, pal::edge(), 1.0);
                    }
                    text.draw(
                        pm,
                        &text.fit(&label, 13.0, r.w - 20.0),
                        ((r.x + 10.0) as f32, (r.cy() + 5.0) as f32),
                        13.0,
                        if sel { WHITE } else { GREY },
                        Align::Left,
                    );
                }
                It::Text(s, c) => text.draw(pm, s, (r.x as f32, (r.y + 15.0) as f32), 13.0, *c, Align::Left),
                It::Head => {
                    for (f, name) in cols {
                        text.draw(pm, name, ((r.x + f * inner) as f32, (r.y + 18.0) as f32), 12.5, GREY, Align::Left);
                    }
                    crate::fill(pm, 0.0, r.y + r.h - 1.0, pw, 1.0, grid, 1.0);
                }
                It::Row(i) => {
                    let t = &self.trades[*i];
                    if hover(&r) {
                        round_fill(pm, r, 6.0, pal::hover(), 1.0);
                    }
                    let (_, m, d, _, _, _) = ny(t.entry_ts);
                    let p = t.pnl();
                    let c = if t.closed() { tone(p) } else { WHITE };
                    let col = |k: usize| PAD + cols[k].0 * inner;
                    let y = (r.cy() + 5.0) as f32;
                    text.draw(pm, &format!("{d} {}", MONTHS[(m - 1) as usize]), (col(0) as f32, y), 13.0, WHITE, Align::Left);
                    text.draw(
                        pm,
                        &text.fit(&t.sym, 13.0, (cols[2].0 - cols[1].0) * inner - 4.0),
                        (col(1) as f32, y),
                        13.0,
                        WHITE,
                        Align::Left,
                    );
                    text.draw(
                        pm,
                        if t.short() { "▼ Short" } else { "▲ Long" },
                        (col(2) as f32, y),
                        13.0,
                        if t.short() { crate::palette::down() } else { crate::palette::up() },
                        Align::Left,
                    );
                    let off = if wide {
                        text.draw(
                            pm,
                            &text.fit(&t.strat, 13.0, (cols[4].0 - cols[3].0) * inner - 6.0),
                            (col(3) as f32, y),
                            13.0,
                            GREY,
                            Align::Left,
                        );
                        1
                    } else {
                        0
                    };
                    if t.closed() {
                        text.draw(pm, &r_txt(t.r()), (col(3 + off) as f32, y), 13.0, c, Align::Left);
                    } else {
                        let tag = Rect::new(col(3 + off), r.cy() - 10.0, 50.0, 20.0);
                        round_fill(pm, tag, 4.0, pal::on(), 1.0);
                        text.draw(pm, "ouvert", (tag.cx() as f32, (tag.cy() + 4.0) as f32), 11.5, GREY, Align::Center);
                    }
                    text.draw(pm, &money(p), (col(4 + off) as f32, y), 13.0, c, Align::Left);
                    let emo = EMOS.iter().find(|e| e.0 == t.emo).and_then(|e| e.1.split(' ').next()).unwrap_or("");
                    text.draw(pm, emo, ((r.x + r.w - 8.0) as f32, y), 13.0, WHITE, Align::Right);
                }
                It::Foot(k) => {
                    round_fill(pm, r, 6.0, if hover(&r) { pal::hover() } else { pal::panel() }, 1.0);
                    round_stroke(pm, r, 6.0, pal::edge(), 1.0);
                    text.draw(
                        pm,
                        &text.fit(FOOT[*k], 12.5, r.w - 8.0),
                        (r.cx() as f32, (r.cy() + 4.5) as f32),
                        12.5,
                        WHITE,
                        Align::Center,
                    );
                }
                It::Kpi(k) => {
                    let (name, val, c) = &kpis[*k];
                    round_fill(pm, r, 8.0, pal::panel(), 1.0);
                    round_stroke(pm, r, 8.0, pal::edge(), 1.0);
                    text.draw(
                        pm,
                        &text.fit(name, 12.0, r.w - 16.0),
                        ((r.x + 10.0) as f32, (r.y + 19.0) as f32),
                        12.0,
                        GREY,
                        Align::Left,
                    );
                    text.draw(
                        pm,
                        &text.fit(val, 15.0, r.w - 16.0),
                        ((r.x + 10.0) as f32, (r.y + 40.0) as f32),
                        15.0,
                        *c,
                        Align::Left,
                    );
                }
                It::Title(s) => text.draw(pm, s, (r.x as f32, (r.y + 20.0) as f32), 14.0, WHITE, Align::Left),
                It::Curve => {
                    round_fill(pm, r, 8.0, pal::field(), 1.0);
                    let c = &s.curve;
                    if c.len() >= 2 {
                        let (mn, mx) = c.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), v| (a.min(*v), b.max(*v)));
                        let span = if mx > mn { mx - mn } else { 1.0 };
                        let xy = |i: usize, v: f64| {
                            (
                                (r.x + 4.0 + i as f64 / (c.len() - 1) as f64 * (r.w - 8.0)) as f32,
                                (r.y + r.h - 6.0 - (v - mn) / span * (r.h - 12.0)) as f32,
                            )
                        };
                        // zéro en pointillés
                        let zy = xy(0, 0.0).1 as f64;
                        let mut x = r.x;
                        while x < r.x + r.w {
                            crate::fill(pm, x, zy, 3.0, 1.0, grid, 1.0);
                            x += 6.0;
                        }
                        let line: Vec<(f32, f32)> = c.iter().enumerate().map(|(i, v)| xy(i, *v)).collect();
                        let col = if c[c.len() - 1] >= 0.0 {
                            crate::palette::up()
                        } else {
                            crate::palette::down()
                        };
                        let mut area = line.clone();
                        area.push((line[line.len() - 1].0, (r.y + r.h) as f32));
                        area.push((line[0].0, (r.y + r.h) as f32));
                        poly_fill(pm, 0.0, 0.0, &area, col, 0.12);
                        lines(pm, 0.0, 0.0, &[&line], col, 2.0);
                    }
                }
                It::GHead | It::GRow(..) => {
                    // nom à gauche, chiffres alignés à droite de leur colonne
                    let gcols = [0.0, 0.48, 0.63, 0.81, 1.0];
                    let cells: Vec<(String, Rgb)> = match it {
                        It::GRow(g, k) => {
                            let row = &v.groups[*g].1[*k];
                            vec![
                                (text.fit(&row.key, 12.5, 0.36 * inner), WHITE),
                                (row.n.to_string(), WHITE),
                                (row.win.map_or("—".into(), |w| format!("{w:.0} %")), WHITE),
                                (
                                    r_txt(row.avg_r),
                                    if row.avg_r.unwrap_or(0.0) >= 0.0 {
                                        crate::palette::up()
                                    } else {
                                        crate::palette::down()
                                    },
                                ),
                                (
                                    money(Some(row.total)),
                                    if row.total >= 0.0 { crate::palette::up() } else { crate::palette::down() },
                                ),
                            ]
                        }
                        _ => ["", "Trades", "Win %", "R moy.", "P&L"].iter().map(|s| (s.to_string(), GREY)).collect(),
                    };
                    for (k, (cell, c)) in cells.iter().enumerate() {
                        text.draw(
                            pm,
                            cell,
                            ((r.x + gcols[k] * inner) as f32, (r.y + 17.0) as f32),
                            12.5,
                            *c,
                            if k == 0 { Align::Left } else { Align::Right },
                        );
                    }
                    crate::fill(pm, r.x, r.y + r.h - 1.0, r.w, 1.0, grid, 1.0);
                }
                It::Bar(k) => match &bars[*k] {
                    None => {
                        text.draw(pm, "💰 P&L compte financé", (r.x as f32, (r.y + 16.0) as f32), 13.0, GREY, Align::Left);
                        text.draw(
                            pm,
                            &money(Some(v.prop.pnl.to_f64())),
                            ((r.x + r.w) as f32, (r.y + 16.0) as f32),
                            13.0,
                            tone(Some(v.prop.pnl.to_f64())),
                            Align::Right,
                        );
                    }
                    Some((name, val, max, good, txt)) => {
                        let p = if *max > 0.0 { (val / max * 100.0).clamp(0.0, 100.0) } else { 0.0 };
                        let tw = text.width(txt, 13.0) as f64;
                        text.draw(
                            pm,
                            &text.fit(name, 12.5, r.w - tw - 10.0),
                            (r.x as f32, (r.y + 16.0) as f32),
                            12.5,
                            GREY,
                            Align::Left,
                        );
                        text.draw(pm, txt, ((r.x + r.w) as f32, (r.y + 16.0) as f32), 13.0, WHITE, Align::Right);
                        let track = Rect::new(r.x, r.y + 28.0, r.w, 8.0);
                        round_fill(pm, track, 4.0, pal::field(), 1.0);
                        let c = if *good {
                            crate::palette::up()
                        } else if p >= 80.0 {
                            crate::palette::down()
                        } else {
                            crate::palette::GOLD
                        };
                        if p > 0.0 {
                            round_fill(pm, Rect::new(track.x, track.y, track.w * p / 100.0, track.h), 4.0, c, 1.0);
                        }
                    }
                },
                It::Cfg(k) => {
                    round_fill(pm, r, 6.0, pal::field(), 1.0);
                    text.draw(pm, CFG[*k], ((r.x + 10.0) as f32, (r.cy() + 4.5) as f32), 12.5, GREY, Align::Left);
                    let lw = text.width(CFG[*k], 12.5) as f64 + 20.0;
                    text.draw(
                        pm,
                        &text.fit(&self.cfg_value(*k), 13.0, r.w - lw - 72.0),
                        ((r.x + r.w - 72.0) as f32, (r.cy() + 5.0) as f32),
                        13.0,
                        WHITE,
                        Align::Right,
                    );
                }
                It::Step(_, d) => {
                    round_fill(pm, r, 5.0, if hover(&r) { pal::hover() } else { pal::panel() }, 1.0);
                    let c = if hover(&r) { WHITE } else { GREY };
                    if *d > 0 {
                        lines(pm, r.cx(), r.cy(), &[&[(-5.0, 0.0), (5.0, 0.0)], &[(0.0, -5.0), (0.0, 5.0)]], c, 1.6);
                    } else {
                        lines(pm, r.cx(), r.cy(), &[&[(-5.0, 0.0), (5.0, 0.0)]], c, 1.6);
                    }
                }
            }
        }
        // cadre de l'état PropFirm (derrière son texte : redessiné en transparence par-dessus)
        if self.tab == 2 {
            // (only the state lines, above the first title)
            let txt: Vec<&Rect> = items
                .iter()
                .take_while(|(_, it)| !matches!(it, It::Title(_)))
                .filter(|(_, it)| matches!(it, It::Text(_, c) if *c != GREY))
                .map(|(r, _)| r)
                .collect();
            if let (Some(a), Some(b)) = (txt.first(), txt.last()) {
                let c = if v.prop.warnings.is_empty() {
                    crate::palette::up()
                } else {
                    crate::palette::down()
                };
                let frame = Rect::new(PAD, a.y - 8.0, inner, b.y + b.h - a.y + 14.0);
                round_fill(pm, frame, 8.0, c, 0.10);
                round_stroke(pm, frame, 8.0, c, 1.0);
            }
        }
    }
}

// ---------- fiche d'un trade (journal/formulaire.rs) ----------
mod formulaire;
pub use formulaire::*;

/// The buttons at the foot of the trades: export, backup, import, always there (a v1 backup imports even into an
/// empty journal); 🆘 the rescue code of the journal key on a second line, only when `secours` (`SECOURS_ICI`).
fn pied(out: &mut Vec<(Rect, It)>, y: f64, inner: f64, secours: bool) {
    let w = (inner - 12.0) / 3.0;
    for k in 0..3 {
        out.push((Rect::new(PAD + k as f64 * (w + 6.0), y, w, 32.0), It::Foot(k)));
    }
    if secours {
        out.push((Rect::new(PAD, y + 38.0, inner, 32.0), It::Foot(3)));
    }
}

#[cfg(test)]
mod tests;
