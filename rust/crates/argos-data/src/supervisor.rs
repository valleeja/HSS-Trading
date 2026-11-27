//! 🛡️ Superviseur de données Argos : réessaie, vérifie les bougies, bascule sur un autre exchange si la source est en panne,
//! et dit toujours d'où viennent les données.
//! À chaque demande, il repart de l'exchange choisi : dès que celui-ci refonctionne, la chart revient dessus toute seule.

use crate::{Exchange, ExchangeFeed, Http};
use argos_core::{ArgosError, Bar, Result, Timeframe};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

/// Réglages du superviseur.
#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    /// Pauses avant chaque nouvel essai sur le même exchange (ms) : 2 essais de plus par défaut.
    pub retry_ms: Vec<u64>,
    /// Dernière bougie trop vieille si elle date de plus de N unités de temps.
    pub stale_bars: i64,
    /// Ordre des exchanges de secours (l'exchange choisi est toujours essayé en premier).
    pub fallback: Vec<Exchange>,
    /// A candle longer than this ((high − low) / close) is shown only once another exchange is asked about it
    /// ([`Problem::Removed`], [`Problem::LoneWick`], bugs B-016 and B-025).
    pub long_wick: f64,
}

impl Default for Config {
    fn default() -> Self {
        use Exchange::*;
        Self {
            retry_ms: vec![500, 1500],
            stale_bars: 3,
            fallback: vec![Binance, Bybit, Okx, Blofin, Bitunix, Weex, Hyperliquid, Bitstamp],
            long_wick: 0.5,
        }
    }
}

/// Ce qui ne va pas dans une réponse.
#[derive(Debug, Clone, PartialEq)]
pub enum Problem {
    /// La requête a échoué (réseau, erreur de l'exchange, symbole inconnu…).
    Request(String),
    /// Aucune bougie reçue.
    Empty,
    /// La dernière bougie date de trop longtemps (secondes).
    Stale { age: i64 },
    /// Prix impossible (≤ 0, haut sous le bas) à cette bougie (temps Unix). A long wick is never one: it is what the
    /// exchange traded (bug B-016).
    BadPrice { time: i64 },
    /// A very long candle at this time (Unix) that `other` saw much shorter: a false price, not a crash (bug B-016),
    /// that makes the chart take the next exchange. The candle in progress (it cannot be taken out: the exchange sends
    /// it again with each trade), or one of too many false prices in one answer (more than one, and more than
    /// `MAX_REFUS_PCT` of the candles: a broken feed); otherwise a false finished candle is [`Problem::Removed`].
    LoneWick { time: i64, other: Exchange },
    /// A finished candle at this time (Unix), very long while `other` saw it much shorter: a false price, taken out of
    /// the chart, which stays on the exchange chosen (bug B-025). Its place is a hole that is never filled again with
    /// that price (`trous::reparer`).
    Removed { time: i64, other: Exchange },
    /// Trou dans le temps : bougies manquantes (avertissement seulement, pas de bascule).
    Gap { after: i64, missing: i64 },
}

impl Problem {
    /// Grave = on ne peut pas se fier à cette source → essayer ailleurs.
    pub fn is_fatal(&self) -> bool {
        !matches!(self, Problem::Gap { .. } | Problem::Removed { .. })
    }
}

impl std::fmt::Display for Problem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Problem::Request(e) => write!(f, "requête en échec : {e}"),
            Problem::Empty => write!(f, "aucune bougie"),
            Problem::Stale { age } => write!(f, "dernière bougie vieille de {} min", age / 60),
            Problem::BadPrice { time } => write!(f, "prix impossible à la bougie {time}"),
            Problem::LoneWick { time, other } => write!(f, "mèche pas vue chez {} à la bougie {time}", other.name()),
            Problem::Removed { time, other } => write!(f, "bougie {time} retirée : mèche pas vue chez {}", other.name()),
            Problem::Gap { after, missing } => write!(f, "{missing} bougie(s) manquante(s) après {after}"),
        }
    }
}

/// How late an exchange gives its newest candle, in seconds, on top of `stale_bars`: Coinbase has no live candles,
/// and gives those of one minute 3 to 5 minutes late (seen on 23 Nov. 2026); its chart stays on Coinbase.
pub fn late_s(ex: Exchange) -> i64 {
    match ex {
        Exchange::Coinbase => 300,
        _ => 0,
    }
}

/// Vérifie une série de bougies. Retourne tous les problèmes trouvés (graves et avertissements). `late_s` : le retard
/// propre à l'exchange ([`late_s`]).
pub fn check(bars: &[Bar], tf: Timeframe, now_s: i64, late_s: i64, cfg: &Config) -> Vec<Problem> {
    let Some(last) = bars.last() else { return vec![Problem::Empty] };
    let step = tf.seconds();
    let mut out = vec![];
    // la bougie en cours a commencé il y a au plus 1 unité de temps : au-delà de `stale_bars` (et du retard propre à
    // l'exchange), la source est figée
    let age = now_s - last.time;
    if age > step * cfg.stale_bars + late_s {
        out.push(Problem::Stale { age });
    }
    // 🔒 reliable data (bug B-016, 25 Nov. 2026): its length alone never makes a candle a bad price. The crash of
    // 10 Oct. 2025 made XRP's daily candle 70 % long on Binance, Hyperliquid, Bybit and BloFin ((high − low) / close);
    // a rule « more than 50 % is absurd » threw them all away for WEEX, whose 300 days did not reach that day. Only
    // what cannot be is a bad price here; a very long candle is asked to another exchange (`Supervisor::lone_wicks`).
    if let Some(b) = bars.iter().find(|b| b.low <= 0.0 || b.high < b.low || b.close <= 0.0) {
        out.push(Problem::BadPrice { time: b.time });
    }
    // les unités de temps mensuelles et plus n'ont pas un pas fixe : pas de contrôle de trous
    if step < 2_419_200 {
        for w in bars.windows(2) {
            let missing = (w[1].time - w[0].time) / step - 1;
            if missing > 0 {
                out.push(Problem::Gap { after: w[0].time, missing });
            }
        }
    }
    out
}

/// Résultat supervisé : les bougies et d'où elles viennent.
#[derive(Debug, Clone, PartialEq)]
pub struct Supervised {
    /// The candles, without the false prices taken out ([`Problem::Removed`]).
    pub bars: Vec<Bar>,
    /// Exchange qui a fourni les bougies.
    pub source: Exchange,
    /// Exchange choisi par l'utilisateur.
    pub primary: Exchange,
    /// Problèmes rencontrés en chemin (par exchange), y compris les avertissements de la source retenue.
    pub problems: Vec<(Exchange, Problem)>,
    /// Heures des bougies refusées par la vérification (incohérentes) : des trous à réparer (étape 3 du plan).
    pub a_reparer: Vec<i64>,
}

impl Supervised {
    /// Données d'un exchange de secours ?
    pub fn is_fallback(&self) -> bool {
        self.source != self.primary
    }

    /// Message à afficher sur la chart (`None` si tout est normal).
    pub fn banner(&self) -> Option<String> {
        if self.is_fallback() {
            let why = self.problems.iter().find(|(e, p)| *e == self.primary && p.is_fatal()).map(|(_, p)| p);
            let (primary, source) = (self.primary.name(), self.source.name());
            // une ligne simple, sans code technique, qui dit la vraie raison : un exchange qui répond avec un prix
            // douteux répond (bug B-016 : « Binance ne répond pas » alors que Binance répondait)
            Some(match why {
                Some(Problem::LoneWick { other, .. }) => {
                    format!("⚠️ {primary} : prix douteux (pas vu chez {}), données de {source} pour l'instant", other.name())
                }
                Some(Problem::BadPrice { .. }) => format!("⚠️ {primary} : prix impossible reçu, données de {source} pour l'instant"),
                _ => format!("⚠️ {primary} ne répond pas : données de {source} pour l'instant"),
            })
        } else {
            let mine = || self.problems.iter().filter(|(e, _)| *e == self.source).map(|(_, p)| p);
            let gaps: i64 = mine().map(|p| if let Problem::Gap { missing, .. } = p { *missing } else { 0 }).sum();
            // a false price taken out (bug B-025): the chart stays here, and says it
            let mut others: Vec<&str> = vec![];
            let mut removed = 0;
            for p in mine() {
                if let Problem::Removed { other, .. } = p {
                    removed += 1;
                    if !others.contains(&other.name()) {
                        others.push(other.name());
                    }
                }
            }
            let mut parts = vec![];
            match removed {
                0 => {}
                1 => parts.push(format!("1 bougie au prix douteux retirée (pas vue chez {})", others.join(", "))),
                n => parts.push(format!("{n} bougies au prix douteux retirées (pas vues chez {})", others.join(", "))),
            }
            if gaps > 0 {
                parts.push(format!("{gaps} bougie(s) manquante(s) dans l'historique"));
            }
            (!parts.is_empty()).then(|| format!("⚠️ {} : {}", self.source.name(), parts.join(" · ")))
        }
    }
}

/// Le superviseur : un client HTTP partagé par tous les exchanges.
pub struct Supervisor<H: Http + Clone> {
    http: H,
    pub config: Config,
    now_ms: fn() -> i64,
    /// What other exchanges said of the very long candles already asked ([`Supervisor::lone_wicks`]).
    seen: Seen,
}

/// Attentes de plus en plus longues (ms) quand l'exchange répond « trop de requêtes » (HTTP 429).
const RATE_WAITS: [u64; 3] = [3_000, 8_000, 15_000];

/// A very long candle of (exchange, symbol, interval, time) → the exchange that saw it much shorter (a false price),
/// or `None`: seen long there too, or no other exchange could show it (kept as it came).
type Seen = Arc<Mutex<HashMap<(Exchange, String, Timeframe, i64), Option<Exchange>>>>;

/// Answers kept for the whole session, shared by every supervisor of the app: a candle is asked about once.
static SEEN: OnceLock<Seen> = OnceLock::new();

/// Beyond this many answers kept, they are all forgotten (each one asked again when needed).
const SEEN_MAX: usize = 10_000;

/// How many other exchanges are asked about a very long candle before it is kept as it came.
const SECOND_OPINIONS: usize = 2;

/// At most this many candles are asked to another exchange, in one page (the exchange may give fewer): the newest very
/// long candle not judged yet and those before it.
const SECOND_SPAN: i64 = 500;

/// Length of a candle measured against its close: comparable between two exchanges even when one counts in contracts
/// of 1000 coins.
fn stretch(b: &Bar) -> f64 {
    (b.high - b.low) / b.close
}

/// 🔒 Bug B-025: takes out of `bars` (candles of `ex` that came later: older history, a hole repaired) those already
/// found false prices this session ([`Problem::Removed`]), so that the next page never puts back a candle taken out of
/// the chart. A lookup only, never a question; the answers of the supervisors of the app ([`Supervisor::new`]).
pub(crate) fn drop_known_false(ex: Exchange, base: &str, tf: Timeframe, bars: &mut Vec<Bar>) {
    let Some(seen) = SEEN.get() else { return };
    let seen = seen.lock().unwrap_or_else(|e| e.into_inner());
    if seen.is_empty() {
        return;
    }
    let mut key = (ex, base.to_uppercase(), tf, 0);
    bars.retain(|b| {
        key.3 = b.time;
        !matches!(seen.get(&key), Some(Some(_)))
    });
}

/// What the next exchanges said of the very long candles of a series ([`Supervisor::lone_wicks`]).
#[derive(Debug, Default, PartialEq)]
pub(crate) struct Wicks {
    /// False prices: the time of each candle, and the exchange that saw it much shorter.
    pub(crate) lone: Vec<(i64, Exchange)>,
    /// Candles no exchange could answer for yet (a failed question, or older than the page given): never remembered.
    pub(crate) unsure: Vec<i64>,
}

impl Wicks {
    /// A false price, or a candle not judged yet.
    pub(crate) fn holds(&self, time: i64) -> bool {
        self.unsure.contains(&time) || self.lone.iter().any(|(t, _)| *t == time)
    }
}

impl<H: Http + Clone> Supervisor<H> {
    pub fn new(http: H) -> Self {
        Self::shared(http, crate::system_now_ms)
    }

    /// Like [`Supervisor::new`], with this clock: the answers about long candles are those of the whole app.
    pub(crate) fn shared(http: H, now_ms: fn() -> i64) -> Self {
        Self {
            http,
            config: Config::default(),
            now_ms,
            seen: SEEN.get_or_init(Seen::default).clone(),
        }
    }

    /// A supervisor of its own (tests): with this clock, and answers about long candles kept for itself only.
    pub fn with_clock(http: H, config: Config, now_ms: fn() -> i64) -> Self {
        Self {
            http,
            config,
            now_ms,
            seen: Seen::default(),
        }
    }

    /// Ordre d'essai : l'exchange choisi, puis les secours (sans doublon).
    pub fn order(&self, primary: Exchange) -> Vec<Exchange> {
        let mut v = vec![primary];
        v.extend(self.config.fallback.iter().copied().filter(|e| *e != primary));
        v
    }

    /// Bougies de l'exchange choisi ; s'il est en panne (après les nouveaux essais), du premier secours qui répond correctement.
    pub async fn candles(&self, primary: Exchange, base: &str, tf: Timeframe, limit: usize) -> Result<Supervised> {
        let mut problems = vec![];
        for ex in self.order(primary) {
            let feed = ExchangeFeed::with_clock(ex, self.http.clone(), self.now_ms);
            // sur un secours, un seul essai : on ne fait pas attendre la chart pour un exchange qui n'a peut-être pas ce symbole
            let mut pauses: Vec<u64> = if ex == primary { self.config.retry_ms.clone() } else { vec![] };
            let mut attempt = 0;
            loop {
                if attempt > 0 {
                    // petit délai au hasard en plus : des requêtes en attente ne repartent pas toutes ensemble
                    self.http.wait(crate::http::jitter(pauses[attempt - 1])).await;
                }
                let (got, fatal, warn) = match feed.candles_checked(base, tf, limit).await {
                    Ok(mut c) => {
                        let (mut fatal, mut warn): (Vec<Problem>, Vec<Problem>) = check(&c.bars, tf, (self.now_ms)() / 1000, late_s(ex), &self.config)
                            .into_iter()
                            // a quiet time of an exchange that sends no candle without a trade: nothing is missing
                            .filter(|p| !(ex.skips_quiet_candles() && matches!(p, Problem::Gap { .. })))
                            .partition(|p| p.is_fatal());
                        if fatal.is_empty() {
                            // 🔒 bug B-025: a false price is taken out, and the chart stays on this exchange; only the
                            // candle in progress, that the exchange sends again with each trade, makes it take the next
                            let live = c.bars.last().map(|b| b.time);
                            let lone = self.lone_wicks(ex, base, tf, &c.bars, live).await.lone;
                            // a feed that sends many false prices is not to be trusted (the review of 25 Nov.): more
                            // than one, and more than MAX_REFUS_PCT of the candles received, like refused candles,
                            // make the chart take the next exchange, as before
                            let finished = lone.iter().filter(|(t, _)| Some(*t) != live).count();
                            let broken = finished > 1 && finished * 100 > c.bars.len() * crate::MAX_REFUS_PCT;
                            for (time, other) in lone {
                                if broken || Some(time) == live {
                                    fatal.push(Problem::LoneWick { time, other });
                                } else {
                                    warn.push(Problem::Removed { time, other });
                                }
                            }
                            c.bars
                                .retain(|b| !warn.iter().any(|p| matches!(p, Problem::Removed { time, .. } if *time == b.time)));
                        }
                        (c, fatal, warn)
                    }
                    Err(e) => (Default::default(), vec![Problem::Request(e.to_string())], vec![]),
                };
                if fatal.is_empty() {
                    problems.extend(warn.into_iter().map(|p| (ex, p)));
                    return Ok(Supervised {
                        bars: got.bars,
                        source: ex,
                        primary,
                        problems,
                        a_reparer: got.a_reparer,
                    });
                }
                // « trop de requêtes » (HTTP 429) sur l'exchange choisi : il ralentit, il n'est pas en panne ; on attend
                // plus longtemps et on réessaie au lieu de passer à un secours
                let busy = fatal.iter().find_map(|p| match p {
                    Problem::Request(e) if e.contains("429") => Some(e.clone()),
                    _ => None,
                });
                if let Some(e) = busy.filter(|_| ex == primary && attempt == pauses.len()) {
                    if pauses.len() < self.config.retry_ms.len() + RATE_WAITS.len() {
                        // le délai demandé par l'exchange (Retry-After), sinon 3 s, 8 s, 15 s
                        let base = RATE_WAITS[pauses.len() - self.config.retry_ms.len()];
                        pauses.push(crate::http::retry_after_ms(&e).unwrap_or(base));
                    }
                }
                // on garde seulement le dernier essai de chaque exchange dans le rapport
                if attempt == pauses.len() {
                    problems.extend(fatal.into_iter().map(|p| (ex, p)));
                    break;
                }
                attempt += 1;
            }
        }
        let detail = problems.iter().map(|(e, p)| format!("{} : {p}", e.name())).collect::<Vec<_>>().join(" · ");
        Err(ArgosError::Feed(format!("aucune source de bougies disponible pour {base} {tf} — {detail}")))
    }

    /// 🔍 Bug B-016: the very long candles of `ex` (more than `long_wick` of their close) are asked to the next two
    /// exchanges in order before the chart shows them, in one question (one page) each. One that another exchange saw
    /// less than a quarter as long is a false price ([`Wicks::lone`]: taken out of the chart, or the next exchange for
    /// the candle in progress, bug B-025); one seen long there too (a crash, like XRP's of 10 Oct. 2025, even deeper on
    /// one exchange), or that the exchanges asked answered without, is kept as it came. Those answers are remembered
    /// for the session (a finished candle only: never `live`, the candle in progress). A candle no exchange could
    /// answer for (a failed question, or older than the page it gave, [`Wicks::unsure`]) is asked again next time,
    /// never remembered: a false price met during a short outage is not kept for the whole session.
    async fn lone_wicks(&self, ex: Exchange, base: &str, tf: Timeframe, bars: &[Bar], live: Option<i64>) -> Wicks {
        let key = |t: i64| (ex, base.to_uppercase(), tf, t);
        let mut out = Wicks::default();
        // newest first
        let mut open: Vec<&Bar> = vec![];
        {
            let seen = self.seen.lock().unwrap_or_else(|e| e.into_inner());
            for b in bars.iter().rev().filter(|b| stretch(b) > self.config.long_wick) {
                match seen.get(&key(b.time)) {
                    Some(Some(other)) => out.lone.push((b.time, *other)),
                    Some(None) => {}
                    None => open.push(b),
                }
            }
        }
        if open.is_empty() {
            return out;
        }
        let step = tf.seconds();
        let mut judged: Vec<(i64, Option<Exchange>)> = vec![];
        // the candles an exchange asked could not answer for: never remembered
        let mut unsure: Vec<i64> = vec![];
        for other in self.order(ex).into_iter().skip(1).take(SECOND_OPINIONS) {
            let (Some(newest), Some(oldest)) = (open.first(), open.last()) else { break };
            let n = ((newest.time - oldest.time) / step + 1).clamp(1, SECOND_SPAN) as usize;
            // one page, one question, never minutes of waiting for an exchange that is not the one shown
            let feed = ExchangeFeed::with_clock(other, self.http.clone(), self.now_ms).impatient();
            let Ok(there) = feed.older_one_page(base, tf, newest.time + step, n).await else {
                unsure.extend(open.iter().map(|b| b.time));
                continue;
            };
            let first = there.first().map(|b| b.time);
            open.retain(|b| match there.iter().find(|o| o.time == b.time) {
                Some(o) => {
                    judged.push((b.time, (stretch(o) < stretch(b) / 4.0).then_some(other)));
                    false
                }
                None => {
                    // older than the page given: that exchange was not asked about this one
                    if first.is_some_and(|f| b.time < f) {
                        unsure.push(b.time);
                    }
                    true
                }
            });
        }
        // the exchanges asked answered without these (that day or that symbol missing there): kept as they came
        judged.extend(open.iter().filter(|b| !unsure.contains(&b.time)).map(|b| (b.time, None)));
        // a candle the second exchange judged after the first one failed is judged, not unsure
        unsure.retain(|t| !judged.iter().any(|(j, _)| j == t));
        unsure.sort_unstable();
        unsure.dedup();
        out.unsure = unsure;
        let mut seen = self.seen.lock().unwrap_or_else(|e| e.into_inner());
        if seen.len() > SEEN_MAX {
            seen.clear();
        }
        for (time, other) in judged {
            if let Some(other) = other {
                out.lone.push((time, other));
            }
            // the candle in progress can still change: asked again next time
            if Some(time) != live {
                seen.insert(key(time), other);
            }
        }
        out
    }

    /// 🔒 Bug B-025: the candles of `ex` found later in a hole of the chart (`trous::reparer`) that must not enter it:
    /// the very long ones another exchange saw much shorter (a false price taken out of the chart left that hole), and
    /// those no exchange could answer for yet (asked again at the next repair). Asked like on the first screen, all
    /// finished candles; no question at all when none is very long.
    pub(crate) async fn held_back(&self, ex: Exchange, base: &str, tf: Timeframe, bars: &[Bar]) -> Wicks {
        self.lone_wicks(ex, base, tf, bars, None).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    const NOW_MS: i64 = 1_700_036_000_000; // 10 h après le début des bougies de test

    /// Réponse simulée : le corps, ou le texte de la panne.
    type Answer = std::result::Result<String, String>;

    /// Faux Internet : une réponse (ou une panne) par domaine, avec le compte des appels et des pauses.
    #[derive(Clone, Default)]
    struct Net {
        routes: Arc<Mutex<HashMap<&'static str, Vec<Answer>>>>,
        calls: Arc<Mutex<Vec<String>>>,
        waits: Arc<Mutex<Vec<u64>>>,
    }
    impl Net {
        /// Réponses successives pour un domaine (la dernière se répète).
        fn route(self, host: &'static str, answers: Vec<Answer>) -> Self {
            self.routes.lock().unwrap().insert(host, answers);
            self
        }
        fn answer(&self, url: &str) -> Result<String> {
            self.calls.lock().unwrap().push(url.to_string());
            let mut r = self.routes.lock().unwrap();
            let (_, list) = r.iter_mut().find(|(h, _)| url.contains(*h)).expect("domaine non simulé");
            let a = if list.len() > 1 { list.remove(0) } else { list[0].clone() };
            a.map_err(ArgosError::Feed)
        }
    }
    impl Http for Net {
        async fn get(&self, url: &str) -> Result<String> {
            self.answer(url)
        }
        async fn post_json(&self, url: &str, _: &str) -> Result<String> {
            self.answer(url)
        }
        async fn wait(&self, ms: u64) {
            self.waits.lock().unwrap().push(ms);
        }
    }

    /// 20 bougies 30m au format Binance / Bybit, finissant juste avant `NOW_MS` ; `bad` met un prix à 0.
    fn binance(n: i64, end_s: i64, bad: bool) -> String {
        let rows: Vec<String> = (0..n)
            .map(|i| {
                let t = (end_s - (n - 1 - i) * 1800) * 1000;
                let c = if bad && i == n - 1 { 0.0 } else { 100.0 + i as f64 };
                format!(r#"[{t},"{c}","{}","{}","{c}","10"]"#, c + 1.0, (c - 1.0).max(0.0))
            })
            .collect();
        format!("[{}]", rows.join(","))
    }
    fn bybit(n: i64, end_s: i64) -> String {
        // Bybit renvoie la plus récente en premier
        let rows: Vec<String> = (0..n)
            .rev()
            .map(|i| format!(r#"["{}","100","101","99","100","10","1000"]"#, (end_s - (n - 1 - i) * 1800) * 1000))
            .collect();
        format!(r#"{{"retCode":0,"result":{{"list":[{}]}}}}"#, rows.join(","))
    }
    /// `n` candles of 30 min written as Bitstamp writes them, without the one at `quiet` (a time without a trade:
    /// Bitstamp sends no candle for it).
    fn bitstamp(n: i64, end_s: i64, quiet: i64) -> String {
        let rows: Vec<String> = (0..n)
            .filter(|i| *i != quiet)
            .map(|i| {
                let t = end_s - (n - 1 - i) * 1800;
                format!(r#"{{"timestamp":"{t}","open":"100","high":"101","low":"99","close":"100","volume":"10"}}"#)
            })
            .collect();
        format!(r#"{{"data":{{"pair":"BTC/USD","ohlc":[{}]}}}}"#, rows.join(","))
    }
    fn fresh_end() -> i64 {
        NOW_MS / 1000 - 600
    }
    fn sup(net: &Net) -> Supervisor<Net> {
        Supervisor::with_clock(net.clone(), Config::default(), || NOW_MS)
    }

    #[test]
    fn verification_des_bougies() {
        let cfg = Config::default();
        let bar = |t: i64, l: f64, h: f64, c: f64| Bar {
            time: t,
            open: c,
            high: h,
            low: l,
            close: c,
            volume: 1.0,
        };
        let now = 10_000;
        let ok = [bar(8200, 99.0, 101.0, 100.0), bar(10_000 - 1800, 99.0, 101.0, 100.0)];
        assert!(check(&ok, Timeframe::M30, now, 0, &cfg).is_empty());
        assert_eq!(check(&[], Timeframe::M30, now, 0, &cfg), vec![Problem::Empty]);
        // figée depuis 3 h en 30m
        assert!(matches!(
            check(&[bar(10_000 - 10_800 - 1, 99.0, 101.0, 100.0)], Timeframe::M30, now, 0, &cfg)[0],
            Problem::Stale { .. }
        ));
        // a price of zero → impossible price
        assert!(check(&[bar(8200, 0.0, 100.0, 50.0)], Timeframe::M30, now, 0, &cfg).contains(&Problem::BadPrice { time: 8200 }));
        // 🔒 bug B-016: a wick of 160 % of the close is long, not impossible: no problem at all
        assert!(check(&[bar(8200, 20.0, 100.0, 50.0)], Timeframe::M30, now, 0, &cfg).is_empty());
        // trou de 2 bougies → avertissement seulement
        let gap = [bar(1000, 99.0, 101.0, 100.0), bar(1000 + 3 * 1800, 99.0, 101.0, 100.0)];
        let p = check(&gap, Timeframe::M30, 1000 + 3 * 1800, 0, &cfg);
        assert_eq!(p, vec![Problem::Gap { after: 1000, missing: 2 }]);
        assert!(!p[0].is_fatal());
        // 🔒 reliable data: Coinbase gives its candles of one minute 3 to 5 minutes late (23 Nov. 2026), and has no live
        // candles: its own lateness keeps its chart on Coinbase; the same age on another exchange is a stale source
        let late = [bar(10_000 - 300, 99.0, 101.0, 100.0)];
        assert!(check(&late, Timeframe::M1, now, late_s(Exchange::Coinbase), &cfg).is_empty());
        assert!(matches!(
            check(&late, Timeframe::M1, now, late_s(Exchange::Binance), &cfg)[0],
            Problem::Stale { age: 300 }
        ));
        let frozen = [bar(10_000 - 481, 99.0, 101.0, 100.0)];
        assert!(matches!(
            check(&frozen, Timeframe::M1, now, late_s(Exchange::Coinbase), &cfg)[0],
            Problem::Stale { .. }
        ));
    }

    #[tokio::test]
    async fn source_normale_aucun_message() {
        let net = Net::default().route("binance", vec![Ok(binance(20, fresh_end(), false))]);
        let r = sup(&net).candles(Exchange::Binance, "BTC", Timeframe::M30, 20).await.unwrap();
        assert_eq!((r.source, r.is_fallback(), r.bars.len()), (Exchange::Binance, false, 20));
        assert!(r.banner().is_none());
        assert!(net.waits.lock().unwrap().is_empty());
    }

    /// 🔒 Reliable data: Bitstamp sends no candle for a time without a trade (its minute of 05:42 on 24 Nov. 2026):
    /// that quiet time is not a missing candle, the chart warns of nothing; the same gap elsewhere is one.
    #[tokio::test]
    async fn a_quiet_time_of_bitstamp_is_not_a_missing_candle() {
        let net = Net::default().route("bitstamp", vec![Ok(bitstamp(20, fresh_end(), 10))]);
        let r = sup(&net).candles(Exchange::Bitstamp, "BTC", Timeframe::M30, 20).await.unwrap();
        assert_eq!((r.source, r.is_fallback(), r.bars.len()), (Exchange::Bitstamp, false, 19));
        assert!(r.problems.is_empty() && r.banner().is_none(), "{:?}", r.problems);
        let gaps = check(&r.bars, Timeframe::M30, NOW_MS / 1000, 0, &Config::default());
        assert!(matches!(gaps[..], [Problem::Gap { missing: 1, .. }]), "the gap is there: {gaps:?}");
        assert!(!Exchange::Binance.skips_quiet_candles() && Exchange::Bitstamp.skips_quiet_candles());
    }

    #[tokio::test]
    async fn panne_passagere_reessai_puis_ok() {
        let net = Net::default().route("binance", vec![Err("timeout".into()), Ok(binance(20, fresh_end(), false))]);
        let r = sup(&net).candles(Exchange::Binance, "BTC", Timeframe::M30, 20).await.unwrap();
        assert_eq!(r.source, Exchange::Binance);
        let w = net.waits.lock().unwrap().clone();
        assert!(w.len() == 1 && (500..=625).contains(&w[0]), "{w:?}");
    }

    /// « Trop de requêtes » (HTTP 429) : l'exchange ralentit ; on attend plus longtemps et on réessaie, sans secours.
    #[tokio::test]
    async fn trop_de_requetes_on_attend_sans_secours() {
        let net = Net::default()
            .route(
                "binance",
                vec![
                    Err("HTTP 429".into()),
                    Err("HTTP 429".into()),
                    Err("HTTP 429".into()),
                    Ok(binance(20, fresh_end(), false)),
                ],
            )
            .route("bybit", vec![Ok(bybit(20, fresh_end()))]);
        let r = sup(&net).candles(Exchange::Binance, "BTC", Timeframe::M30, 20).await.unwrap();
        assert_eq!(r.source, Exchange::Binance, "pas de bascule pour un 429");
        let w = net.waits.lock().unwrap().clone();
        assert!(w.len() == 3 && w[2] >= 3_000, "attente plus longue après les essais normaux : {w:?}");
    }

    #[tokio::test]
    async fn panne_qui_dure_bascule_sur_le_secours_et_l_annonce() {
        let net = Net::default()
            .route("binance", vec![Err("HTTP 503".into())])
            .route("bybit", vec![Ok(bybit(20, fresh_end()))]);
        let r = sup(&net).candles(Exchange::Binance, "BTC", Timeframe::M30, 20).await.unwrap();
        assert_eq!((r.source, r.primary), (Exchange::Bybit, Exchange::Binance));
        assert!(r.is_fallback());
        let banner = r.banner().unwrap();
        // une ligne simple, sans code technique
        assert_eq!(banner, "⚠️ Binance ne répond pas : données de Bybit pour l'instant");
        assert!(!banner.contains("HTTP") && !banner.contains("null"));
        // 3 essais sur Binance (2 pauses de 500 ms et 1,5 s, plus 0 à 25 % au hasard), puis Bybit
        let w = net.waits.lock().unwrap().clone();
        assert!(w.len() == 2 && (500..=625).contains(&w[0]) && (1500..=1875).contains(&w[1]), "{w:?}");
        assert_eq!(net.calls.lock().unwrap().iter().filter(|u| u.contains("binance")).count(), 3);
    }

    #[tokio::test]
    async fn donnees_figees_ou_absurdes_aussi_basculent() {
        // Binance répond, mais sa dernière bougie date de 5 h : figée → Bybit
        let net = Net::default()
            .route("binance", vec![Ok(binance(20, NOW_MS / 1000 - 18_000, false))])
            .route("bybit", vec![Ok(bybit(20, fresh_end()))]);
        let r = sup(&net).candles(Exchange::Binance, "BTC", Timeframe::M30, 20).await.unwrap();
        assert_eq!(r.source, Exchange::Bybit);
        assert!(matches!(r.problems[0], (Exchange::Binance, Problem::Stale { .. })));
        // une bougie à prix 0 : refusée dès la lecture (bougie incohérente), les 19 autres restent sur Binance
        let net = Net::default()
            .route("binance", vec![Ok(binance(20, fresh_end(), true))])
            .route("bybit", vec![Ok(bybit(20, fresh_end()))]);
        let r = sup(&net).candles(Exchange::Binance, "BTC", Timeframe::M30, 20).await.unwrap();
        assert_eq!((r.source, r.bars.len()), (Exchange::Binance, 19));
        assert!(r.bars.iter().all(|b| b.close > 0.0 && b.low > 0.0), "aucune bougie à 0");
    }

    #[tokio::test]
    async fn revient_a_la_source_choisie_des_qu_elle_refonctionne() {
        let net = Net::default()
            .route(
                "binance",
                vec![Err("down".into()), Err("down".into()), Err("down".into()), Ok(binance(20, fresh_end(), false))],
            )
            .route("bybit", vec![Ok(bybit(20, fresh_end()))]);
        let s = sup(&net);
        assert_eq!(s.candles(Exchange::Binance, "BTC", Timeframe::M30, 20).await.unwrap().source, Exchange::Bybit);
        assert_eq!(s.candles(Exchange::Binance, "BTC", Timeframe::M30, 20).await.unwrap().source, Exchange::Binance);
    }

    #[tokio::test]
    async fn tout_en_panne_erreur_claire() {
        let s = Supervisor::with_clock(
            Net::default()
                .route("binance", vec![Err("down".into())])
                .route("bybit", vec![Err("symbole inconnu".into())]),
            Config {
                fallback: vec![Exchange::Bybit],
                ..Config::default()
            },
            || NOW_MS,
        );
        let e = s.candles(Exchange::Binance, "HYPE", Timeframe::M30, 20).await.unwrap_err().to_string();
        assert!(e.contains("aucune source") && e.contains("Binance") && e.contains("Bybit"), "{e}");
    }

    #[test]
    fn ordre_d_essai() {
        let s = sup(&Net::default());
        let o = s.order(Exchange::Okx);
        assert_eq!(o[0], Exchange::Okx);
        assert_eq!(o.len(), 8);
        assert_eq!(o.iter().filter(|e| **e == Exchange::Okx).count(), 1);
    }

    /// Plus de 5 % de bougies refusées : la réponse entière est rejetée et le superviseur passe au secours ; 4 % : gardée,
    /// les refusées deviennent des trous à réparer.
    #[tokio::test]
    async fn trop_de_bougies_refusees_bascule_sur_le_secours() {
        let body = |bad: i64| {
            let end = fresh_end();
            let rows: Vec<String> = (0..100)
                .map(|i| {
                    let t = (end - (99 - i) * 1800) * 1000;
                    let (h, l) = if i % 16 == 5 && i / 16 < bad { ("99", "101") } else { ("101", "99") };
                    format!(r#"[{t},"100","{h}","{l}","100","10"]"#)
                })
                .collect();
            format!("[{}]", rows.join(","))
        };
        for (bad, source, trous) in [(4, Exchange::Binance, 4), (6, Exchange::Bybit, 0)] {
            let net = Net::default()
                .route("binance", vec![Ok(body(bad))])
                .route("bybit", vec![Ok(bybit(100, fresh_end()))]);
            let r = sup(&net).candles(Exchange::Binance, "BTC", Timeframe::M30, 100).await.unwrap();
            assert_eq!((r.source, r.a_reparer.len()), (source, trous), "{bad} %");
        }
    }

    /// 429 avec Retry-After : le superviseur attend le délai demandé par l'exchange (60 s au plus), pas 3 s.
    #[tokio::test]
    async fn trop_de_requetes_avec_retry_after() {
        let net = Net::default()
            .route(
                "binance",
                vec![
                    Err("HTTP 429 (Retry-After 20 s)".into()),
                    Err("HTTP 429 (Retry-After 20 s)".into()),
                    Err("HTTP 429 (Retry-After 20 s)".into()),
                    Ok(binance(20, fresh_end(), false)),
                ],
            )
            .route("bybit", vec![Ok(bybit(20, fresh_end()))]);
        let r = sup(&net).candles(Exchange::Binance, "BTC", Timeframe::M30, 20).await.unwrap();
        assert_eq!(r.source, Exchange::Binance);
        let w = net.waits.lock().unwrap().clone();
        assert!(w.len() == 3 && (20_000..=25_000).contains(&w[2]), "{w:?}");
    }

    /// 10 Oct. 2025 00:00 UTC: the day of the crash (bug B-016).
    const CRASH: i64 = 1_760_054_400;

    /// Two weeks and an hour after the crash day.
    fn after_crash_ms() -> i64 {
        (CRASH + 14 * 86_400 + 3_600) * 1000
    }

    /// 20 daily candles around the crash day, the other days quiet: (time, [open, high, low, close]).
    fn days(crash_day: [f64; 4]) -> Vec<(i64, [f64; 4])> {
        (-5..15)
            .map(|d| (CRASH + d * 86_400, if d == 0 { crash_day } else { [2.5, 2.55, 2.45, 2.5] }))
            .collect()
    }

    /// Daily candles as Binance gives them (oldest first).
    fn binance_days(days: &[(i64, [f64; 4])]) -> String {
        let rows: Vec<String> = days
            .iter()
            .map(|(t, [o, h, l, c])| format!(r#"[{},"{o}","{h}","{l}","{c}","2326268635.4"]"#, t * 1000))
            .collect();
        format!("[{}]", rows.join(","))
    }

    /// Daily candles as Bybit gives them (newest first).
    fn bybit_days(days: &[(i64, [f64; 4])]) -> String {
        let rows: Vec<String> = days
            .iter()
            .rev()
            .map(|(t, [o, h, l, c])| format!(r#"["{}","{o}","{h}","{l}","{c}","10","1000"]"#, t * 1000))
            .collect();
        format!(r#"{{"retCode":0,"result":{{"list":[{}]}}}}"#, rows.join(","))
    }

    /// The exchanges asked, in order, one name per question.
    fn asked(net: &Net) -> Vec<&'static str> {
        let hosts = ["binance", "bybit", "okx", "blofin", "bitunix", "weex", "hyperliquid", "bitstamp"];
        let calls = net.calls.lock().unwrap();
        calls.iter().map(|u| hosts.into_iter().find(|h| u.contains(*h)).unwrap_or("?")).collect()
    }

    /// 🔒 Bug B-016 (25 Nov. 2026): « Binance ne répond pas : données de WEEX pour l'instant » on XRP in one day. The
    /// crash of 10 Oct. 2025 is a real daily candle 70 % long ((high − low) / close), the same on Binance, Hyperliquid
    /// and Bybit (their answers of 25 Nov. 2026, as they came): asked to the next exchange, it is seen there too, and
    /// the exchange chosen keeps the chart, without a banner, without any fallback asked.
    #[tokio::test]
    async fn a_real_crash_wick_keeps_the_exchange_chosen() {
        let hl_body = {
            let rows: Vec<String> = days([2.8029, 2.8369, 1.175, 2.3637])
                .into_iter()
                .map(|(t, [o, h, l, c])| {
                    format!(
                        r#"{{"t":{},"T":{},"s":"XRP","i":"1d","o":"{o}","c":"{c}","h":"{h}","l":"{l}","v":"250975459.0","n":127577}}"#,
                        t * 1000,
                        (t + 86_400) * 1000 - 1
                    )
                })
                .collect();
            format!("[{}]", rows.join(","))
        };
        for (ex, host, second) in [(Exchange::Binance, "binance", "bybit"), (Exchange::Hyperliquid, "hyperliquid", "binance")] {
            let net = Net::default()
                .route("binance", vec![Ok(binance_days(&days([2.8011, 2.8358, 1.1792, 2.3666])))])
                .route("hyperliquid", vec![Ok(hl_body.clone())])
                .route("bybit", vec![Ok(bybit_days(&days([2.8012, 2.8362, 1.1652, 2.3717])))]);
            let sup = Supervisor::with_clock(net.clone(), Config::default(), after_crash_ms);
            let r = sup.candles(ex, "XRP", Timeframe::D1, 20).await.unwrap();
            assert_eq!((r.source, r.bars.len()), (ex, 20), "{ex:?}");
            assert_eq!(r.banner(), None, "{ex:?}");
            let crash = r.bars.iter().find(|b| b.time == CRASH).expect("the crash day is kept");
            assert!((crash.high - crash.low) / crash.close > 0.69, "{ex:?}: the wick is shown as it came");
            // the exchange chosen once, the next one once about the crash day, and nothing else
            assert_eq!(asked(&net), vec![host, second], "{ex:?}");
        }
    }

    /// 🔒 Bug B-016: what the old rule of length still did right. A false price (a high 1000 times the close) on a quiet
    /// day that the next exchange saw quiet is not shown. A finished candle is taken out, and the chart stays on the
    /// exchange chosen, with a banner that says it (bug B-025); the candle in progress, that the exchange sends again
    /// with each trade, makes the chart take that exchange, and the banner says why (never « ne répond pas »). A crash
    /// only deeper on the exchange chosen (110 % there, 40 % on the next one) is kept.
    #[tokio::test]
    async fn a_false_wick_no_other_exchange_saw_is_not_shown() {
        let quiet = days([2.5, 2.55, 2.45, 2.5]);
        let net = Net::default()
            .route("binance", vec![Ok(binance_days(&days([2.5, 2500.0, 2.45, 2.5])))])
            .route("bybit", vec![Ok(bybit_days(&quiet))]);
        let sup = Supervisor::with_clock(net.clone(), Config::default(), after_crash_ms);
        let r = sup.candles(Exchange::Binance, "XRP", Timeframe::D1, 20).await.unwrap();
        assert_eq!((r.source, r.bars.len()), (Exchange::Binance, 19), "the chart stays on Binance");
        assert!(r.bars.iter().all(|b| b.high < 3.0 && b.time != CRASH), "the false price is not on the chart");
        assert!(r.problems.contains(&(
            Exchange::Binance,
            Problem::Removed {
                time: CRASH,
                other: Exchange::Bybit
            }
        )));
        assert!(r.problems.iter().all(|(_, p)| !p.is_fatal()), "a candle taken out never moves the chart");
        assert_eq!(
            r.banner().as_deref(),
            Some("⚠️ Binance : 1 bougie au prix douteux retirée (pas vue chez Bybit)")
        );
        assert!(r.a_reparer.is_empty(), "never a candle to ask again to the exchange that gave the false price");
        // Binance once, Bybit once about that day; asked again, the answer kept is used
        assert_eq!(asked(&net), vec!["binance", "bybit"]);
        let again = sup.candles(Exchange::Binance, "XRP", Timeframe::D1, 20).await.unwrap();
        assert_eq!((again.source, again.bars.len(), again.banner()), (r.source, 19, r.banner()));
        assert_eq!(asked(&net), vec!["binance", "bybit", "binance"]);

        // the candle in progress: the next exchange, and the banner says why
        let today = CRASH + 14 * 86_400;
        let mut false_today = quiet.clone();
        false_today.last_mut().expect("20 days").1 = [2.5, 2500.0, 2.45, 2.5];
        let net = Net::default()
            .route("binance", vec![Ok(binance_days(&false_today))])
            .route("bybit", vec![Ok(bybit_days(&quiet))]);
        let sup = Supervisor::with_clock(net.clone(), Config::default(), after_crash_ms);
        let r = sup.candles(Exchange::Binance, "XRP", Timeframe::D1, 20).await.unwrap();
        assert_eq!((r.source, r.bars.len()), (Exchange::Bybit, 20));
        assert!(r.bars.iter().all(|b| b.high < 3.0), "the false price is not on the chart");
        assert!(r.problems.contains(&(
            Exchange::Binance,
            Problem::LoneWick {
                time: today,
                other: Exchange::Bybit
            }
        )));
        assert_eq!(
            r.banner().as_deref(),
            Some("⚠️ Binance : prix douteux (pas vu chez Bybit), données de Bybit pour l'instant")
        );
        // Binance and its 2 new tries, each with its question to Bybit (the candle in progress is never kept), then
        // Bybit for the chart
        assert_eq!(asked(&net), vec!["binance", "bybit", "binance", "bybit", "binance", "bybit", "bybit"]);

        let deeper = Net::default()
            .route("binance", vec![Ok(binance_days(&days([2.5, 2.55, 0.1, 2.2])))])
            .route("bybit", vec![Ok(bybit_days(&days([2.5, 2.55, 1.6, 2.3])))]);
        let sup = Supervisor::with_clock(deeper.clone(), Config::default(), after_crash_ms);
        let r = sup.candles(Exchange::Binance, "XRP", Timeframe::D1, 20).await.unwrap();
        assert_eq!((r.source, r.banner()), (Exchange::Binance, None), "a crash deeper here is still a crash");
        assert_eq!(asked(&deeper), vec!["binance", "bybit"]);
    }

    /// Daily candles as OKX gives them (newest first).
    fn okx_days(days: &[(i64, [f64; 4])]) -> String {
        let rows: Vec<String> = days
            .iter()
            .rev()
            .map(|(t, [o, h, l, c])| format!(r#"["{}","{o}","{h}","{l}","{c}","10","10","25","1"]"#, t * 1000))
            .collect();
        format!(r#"{{"code":"0","data":[{}]}}"#, rows.join(","))
    }

    /// 🔒 Bug B-016: a very long candle that the next two exchanges answered without (that day missing there) is kept
    /// as it came, on the exchange chosen, after two questions only; asked again, the answer kept is used, with no
    /// question. The candle in progress can still change: it is asked about each time.
    #[tokio::test]
    async fn a_long_wick_nobody_else_can_show_is_kept() {
        let without = |day: i64| -> Vec<(i64, [f64; 4])> { days([2.5, 2.55, 2.45, 2.5]).into_iter().filter(|(t, _)| *t != day).collect() };
        let net = Net::default()
            .route("binance", vec![Ok(binance_days(&days([2.5, 2500.0, 2.45, 2.5])))])
            .route("bybit", vec![Ok(bybit_days(&without(CRASH)))])
            .route("okx", vec![Ok(okx_days(&without(CRASH)))]);
        let sup = Supervisor::with_clock(net.clone(), Config::default(), after_crash_ms);
        for _ in 0..2 {
            let r = sup.candles(Exchange::Binance, "XRP", Timeframe::D1, 20).await.unwrap();
            assert_eq!((r.source, r.banner()), (Exchange::Binance, None));
            assert!(r.bars.iter().any(|b| b.high == 2500.0), "kept as it came");
        }
        assert_eq!(asked(&net), vec!["binance", "bybit", "okx", "binance"]);

        let today = CRASH + 14 * 86_400;
        let mut long_today = days([2.5, 2.55, 2.45, 2.5]);
        long_today.last_mut().expect("20 days").1 = [2.5, 2500.0, 2.45, 2.5];
        let net = Net::default()
            .route("binance", vec![Ok(binance_days(&long_today))])
            .route("bybit", vec![Ok(bybit_days(&without(today)))])
            .route("okx", vec![Ok(okx_days(&without(today)))]);
        let sup = Supervisor::with_clock(net.clone(), Config::default(), after_crash_ms);
        for _ in 0..2 {
            let r = sup.candles(Exchange::Binance, "XRP", Timeframe::D1, 20).await.unwrap();
            assert_eq!((r.source, r.banner()), (Exchange::Binance, None));
        }
        assert_eq!(asked(&net), vec!["binance", "bybit", "okx", "binance", "bybit", "okx"]);
    }

    /// 🔒 Bug B-016, the review of 25 Nov.: a second opinion is one question (one page) per exchange, with no wait, so
    /// the first screen never waits for pages; and a candle no exchange could answer for (a failed question, or older
    /// than the page given) is kept for now and asked again next time, never remembered (a false price met during a
    /// short outage would else stay for the whole session).
    #[tokio::test]
    async fn a_second_opinion_is_one_question_and_a_failure_is_asked_again() {
        // the next two fail (too many requests, unknown): kept, and asked again
        let net = Net::default()
            .route("binance", vec![Ok(binance_days(&days([2.5, 2500.0, 2.45, 2.5])))])
            .route("bybit", vec![Err("HTTP 429 too many requests".into())])
            .route("okx", vec![Err("symbole inconnu".into())]);
        let sup = Supervisor::with_clock(net.clone(), Config::default(), after_crash_ms);
        for _ in 0..2 {
            let r = sup.candles(Exchange::Binance, "XRP", Timeframe::D1, 20).await.unwrap();
            assert_eq!((r.source, r.banner()), (Exchange::Binance, None));
        }
        assert_eq!(asked(&net), vec!["binance", "bybit", "okx", "binance", "bybit", "okx"]);
        assert!(net.waits.lock().unwrap().is_empty(), "no wait for a busy exchange that is not the one shown");

        // two long candles 15 days apart; Bybit's page holds only the newer one (a real crash there too): one question
        // to Bybit, not a second page for the older one. OKX answers without that day, but Bybit was not asked about
        // it: kept for now, asked again
        let mut two = days([2.5, 2.55, 2.45, 2.5]);
        for (t, p) in two.iter_mut() {
            if *t == CRASH + 12 * 86_400 {
                *p = [2.8, 2.83, 1.17, 2.36];
            }
            if *t == CRASH - 3 * 86_400 {
                *p = [2.5, 2500.0, 2.45, 2.5];
            }
        }
        let page: Vec<(i64, [f64; 4])> = two
            .iter()
            .copied()
            .filter(|(t, _)| *t >= CRASH + 8 * 86_400)
            .map(|(t, p)| if t == CRASH + 12 * 86_400 { (t, [2.8, 2.84, 1.16, 2.37]) } else { (t, p) })
            .collect();
        let net = Net::default()
            .route("binance", vec![Ok(binance_days(&two))])
            .route("bybit", vec![Ok(bybit_days(&page))])
            .route(
                "okx",
                vec![Ok(okx_days(&two.iter().copied().filter(|(t, _)| *t != CRASH - 3 * 86_400).collect::<Vec<_>>()))],
            );
        let sup = Supervisor::with_clock(net.clone(), Config::default(), after_crash_ms);
        let r = sup.candles(Exchange::Binance, "XRP", Timeframe::D1, 20).await.unwrap();
        assert_eq!((r.source, r.banner()), (Exchange::Binance, None));
        assert_eq!(asked(&net), vec!["binance", "bybit", "okx"]);
        // the crash judged once is remembered; the older candle, never answered for, is asked again
        sup.candles(Exchange::Binance, "XRP", Timeframe::D1, 20).await.unwrap();
        assert_eq!(asked(&net), vec!["binance", "bybit", "okx", "binance", "bybit", "okx"]);
        assert!(net.waits.lock().unwrap().is_empty());
    }

    /// Daily candles as a chart (or its cache) holds them.
    fn as_bars(days: &[(i64, [f64; 4])]) -> Vec<Bar> {
        days.iter()
            .map(|(t, [o, h, l, c])| Bar {
                time: *t,
                open: *o,
                high: *h,
                low: *l,
                close: *c,
                volume: 1.0,
            })
            .collect()
    }

    /// The holes of daily candles repaired on Binance, as the chart does it.
    async fn repair(net: &Net, base: &str, holes: &[crate::trous::Trou]) -> crate::trous::Reparation {
        let feed = ExchangeFeed::with_clock(Exchange::Binance, net.clone(), after_crash_ms);
        crate::trous::reparer(&feed, base, Timeframe::D1, holes, 10).await
    }

    /// 🔒 Bug B-025 (25 Nov. 2026, Jee's choice): a false price on a finished candle is taken out and the chart stays on
    /// the exchange chosen; nothing puts it back the same session. The hole it leaves is repaired without it (noted like
    /// a hole the exchange does not have, with no question to another exchange), and the history filled before it does
    /// not bring it back either.
    #[tokio::test]
    async fn a_false_candle_taken_out_is_never_put_back() {
        // the answers of the app are shared by its supervisors: a symbol of this test only
        let base = "MECHEA";
        let net = Net::default()
            .route("binance", vec![Ok(binance_days(&days([2.5, 2500.0, 2.45, 2.5])))])
            .route("bybit", vec![Ok(bybit_days(&days([2.5, 2.55, 2.45, 2.5])))]);
        let r = Supervisor::shared(net.clone(), after_crash_ms)
            .candles(Exchange::Binance, base, Timeframe::D1, 20)
            .await
            .unwrap();
        assert_eq!((r.source, r.bars.len()), (Exchange::Binance, 19));
        let holes = crate::trous::trouver(&r.bars, Timeframe::D1, &r.a_reparer, crate::trous::Calendrier::Continu, &[]);
        let hole = crate::trous::Trou {
            debut: CRASH,
            fin: CRASH + 86_400,
        };
        assert_eq!(holes, vec![hole]);
        let fixed = repair(&net, base, &holes).await;
        assert_eq!((fixed.bars.clone(), fixed.irreparables), (vec![], vec![hole]));
        assert_eq!(crate::trous::combler(&r.bars, &fixed.bars), r.bars, "the false price is not put back");
        // the history filled from the next day does not bring it back
        let feed = ExchangeFeed::with_clock(Exchange::Binance, net.clone(), after_crash_ms);
        let older = feed.older(base, Timeframe::D1, CRASH + 86_400, 5).await.unwrap();
        assert!(!older.is_empty() && older.iter().all(|b| b.time < CRASH), "{older:?}");
        // Binance for the chart and Bybit once about that day; then Binance only, for the hole and for the history
        assert_eq!(asked(&net), vec!["binance", "bybit", "binance", "binance"]);
    }

    /// 🔒 Bug B-025: the hole a false price left, met again another day (the chart kept in its cache, the answers of the
    /// session gone). The candle found in it is asked to the next exchange before it enters the chart, and stays out;
    /// asked again the same session, no question to another exchange. A real crash found in a hole enters the chart.
    /// When no other exchange can answer, the candle stays out and its hole is asked again next time, never noted as one
    /// the exchange does not have.
    #[tokio::test]
    async fn a_hole_left_by_a_false_candle_is_repaired_without_it() {
        let quiet = days([2.5, 2.55, 2.45, 2.5]);
        let cached: Vec<Bar> = as_bars(&quiet).into_iter().filter(|b| b.time != CRASH).collect();
        let hole = vec![crate::trous::Trou {
            debut: CRASH,
            fin: CRASH + 86_400,
        }];
        assert_eq!(crate::trous::trouver(&cached, Timeframe::D1, &[], crate::trous::Calendrier::Continu, &[]), hole);

        // a false price: out, its hole noted; the same session, no question to Bybit any more
        let net = Net::default()
            .route("binance", vec![Ok(binance_days(&days([2.5, 2500.0, 2.45, 2.5])))])
            .route("bybit", vec![Ok(bybit_days(&quiet))]);
        for _ in 0..2 {
            let r = repair(&net, "MECHEB", &hole).await;
            assert_eq!((r.bars, r.irreparables), (vec![], hole.clone()));
        }
        assert_eq!(asked(&net), vec!["binance", "bybit", "binance"]);

        // a real crash, seen long on Bybit too: it enters the chart
        let net = Net::default()
            .route("binance", vec![Ok(binance_days(&days([2.8011, 2.8358, 1.1792, 2.3666])))])
            .route("bybit", vec![Ok(bybit_days(&days([2.8012, 2.8362, 1.1652, 2.3717])))]);
        let r = repair(&net, "MECHEC", &hole).await;
        assert_eq!((r.bars.iter().map(|b| b.time).collect::<Vec<_>>(), r.irreparables), (vec![CRASH], vec![]));
        assert_eq!(asked(&net), vec!["binance", "bybit"]);
        // Bybit busy, OKX saw the crash too: it enters as well
        let net = Net::default()
            .route("binance", vec![Ok(binance_days(&days([2.8011, 2.8358, 1.1792, 2.3666])))])
            .route("bybit", vec![Err("HTTP 429 too many requests".into())])
            .route("okx", vec![Ok(okx_days(&days([2.8, 2.84, 1.16, 2.37])))]);
        let r = repair(&net, "MECHEE", &hole).await;
        assert_eq!((r.bars.iter().map(|b| b.time).collect::<Vec<_>>(), r.irreparables), (vec![CRASH], vec![]));
        assert_eq!(asked(&net), vec!["binance", "bybit", "okx"]);

        // nobody else can answer (too many requests, unknown symbol): out for now, and asked again next time
        let net = Net::default()
            .route("binance", vec![Ok(binance_days(&days([2.5, 2500.0, 2.45, 2.5])))])
            .route("bybit", vec![Err("HTTP 429 too many requests".into())])
            .route("okx", vec![Err("symbole inconnu".into())]);
        for _ in 0..2 {
            let r = repair(&net, "MECHED", &hole).await;
            assert_eq!((r.bars, r.irreparables), (vec![], vec![]));
        }
        assert_eq!(asked(&net), vec!["binance", "bybit", "okx", "binance", "bybit", "okx"]);
        assert!(net.waits.lock().unwrap().is_empty(), "no wait for a busy exchange that is not the one shown");
    }

    /// 🔒 Bug B-025, the review of 25 Nov.: a feed that sends many false prices is not to be trusted. Up to
    /// `MAX_REFUS_PCT` of the candles received (5 of 100), and always one, they are taken out and the chart stays;
    /// beyond (6 of 100, 2 of 20), the chart takes the next exchange, as before, and the banner says why.
    #[tokio::test]
    async fn too_many_false_prices_still_move_the_chart() {
        let quiet = [2.5, 2.55, 2.45, 2.5];
        let spike = [2.5, 2500.0, 2.45, 2.5];
        let today = CRASH + 14 * 86_400;
        // `n` days up to today, a false price on the days of `false_days` (counted back from yesterday)
        let series = |n: i64, false_days: &[i64]| -> Vec<(i64, [f64; 4])> {
            (0..n)
                .map(|k| {
                    let t = today - (n - 1 - k) * 86_400;
                    let back = (today - t) / 86_400;
                    (t, if false_days.contains(&back) { spike } else { quiet })
                })
                .collect()
        };
        let all_quiet = series(100, &[]);
        for (n, false_days, limit, stays) in [
            (100, vec![1, 9, 23, 47, 88], 100, true),
            (100, vec![1, 9, 23, 47, 88, 95], 100, false),
            (20, vec![3, 11], 20, false),
            (20, vec![1], 3, true),
        ] {
            let net = Net::default()
                .route("binance", vec![Ok(binance_days(&series(n, &false_days)))])
                .route("bybit", vec![Ok(bybit_days(&all_quiet))]);
            let sup = Supervisor::with_clock(net.clone(), Config::default(), after_crash_ms);
            let r = sup.candles(Exchange::Binance, "XRP", Timeframe::D1, limit).await.unwrap();
            let k = false_days.len();
            assert!(r.bars.iter().all(|b| b.high < 3.0), "{k} of {limit}: no false price on the chart");
            if stays {
                assert_eq!((r.source, r.bars.len()), (Exchange::Binance, limit - k), "{k} of {limit}");
                let banner = if k == 1 {
                    "⚠️ Binance : 1 bougie au prix douteux retirée (pas vue chez Bybit)".to_string()
                } else {
                    format!("⚠️ Binance : {k} bougies au prix douteux retirées (pas vues chez Bybit)")
                };
                assert_eq!(r.banner(), Some(banner), "{k} of {limit}");
            } else {
                assert_eq!((r.source, r.bars.len()), (Exchange::Bybit, limit), "{k} of {limit}");
                assert_eq!(
                    r.banner().as_deref(),
                    Some("⚠️ Binance : prix douteux (pas vu chez Bybit), données de Bybit pour l'instant"),
                    "{k} of {limit}"
                );
                // Binance and its 2 new tries (the answers about finished candles kept: no more question), then Bybit
                assert_eq!(asked(&net), vec!["binance", "bybit", "binance", "binance", "bybit"], "{k} of {limit}");
            }
        }
    }
}
