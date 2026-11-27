//! 🚩 Watchlist Hyperliquid : les marchés les plus populaires (volume 24 h) — **Crypto** (les perps d'Hyperliquid, + PAXG),
//! **HIP-3** et **HIP-3 Growth mode** (frais réduits de 90 %). HIP-3 = perps déployés par d'autres sur Hyperliquid, nommés
//! `dex:SYMBOLE` (ex. `xyz:NVDA`).
//! Requêtes : `perpDexs` (la liste des dex HIP-3) et `metaAndAssetCtxs` (prix, prix d'il y a 24 h, volume) par dex ;
//! le flux `activeAssetCtx` (les mêmes chiffres, en direct) des seuls marchés à l'écran.
//! Partie pure (testée) + rafraîchissement en arrière-plan (`native`).

#[cfg(any(feature = "native", feature = "web"))]
use crate::exchanges::hyperliquid::{dexes_request, parse_dexes};
use crate::exchanges::Request;
use argos_core::{ArgosError, Result};

const INFO: &str = "https://api.hyperliquid.xyz/info";

/// Un marché de la watchlist.
#[derive(Debug, Clone, PartialEq)]
pub struct Quote {
    /// Nom exact pour l'API (« BTC », « xyz:NVDA »).
    pub coin: String,
    /// Prix tel que l'exchange l'écrit (garde ses décimales).
    pub mark: String,
    /// Prix d'il y a 24 h.
    pub prev: f64,
    /// Volume 24 h en dollars (pour classer).
    pub volume: f64,
    /// HIP-3 en « growth mode » (frais réduits de 90 % par le déployeur).
    pub growth: bool,
    /// Prix oracle, intérêt ouvert (en unités du marché) et funding horaire (fraction : 0.0000125 = 0,00125 %).
    pub oracle: f64,
    pub oi: f64,
    pub funding: f64,
}

/// Une section (« Crypto », « HIP-3 »).
#[derive(Debug, Clone, PartialEq)]
pub struct Section {
    pub name: String,
    pub quotes: Vec<Quote>,
}

/// Prix + volume d'un dex (`""` = Hyperliquid lui-même).
pub fn quotes_request(dex: &str) -> Request {
    let body = if dex.is_empty() {
        r#"{"type":"metaAndAssetCtxs"}"#.to_string()
    } else {
        format!(r#"{{"type":"metaAndAssetCtxs","dex":"{dex}"}}"#)
    };
    Request::PostJson { url: INFO.into(), body }
}

/// Marchés d'un dex (les retirés sont ignorés). Les noms HIP-3 ont déjà leur préfixe `dex:` ; on l'ajoute s'il manque.
pub fn parse_quotes(body: &str, dex: &str) -> Result<Vec<Quote>> {
    let v: serde_json::Value = serde_json::from_str(body).map_err(|e| ArgosError::Feed(format!("Hyperliquid : {e}")))?;
    let (meta, ctx) = (&v[0]["universe"], &v[1]);
    let list = meta
        .as_array()
        .ok_or_else(|| ArgosError::Feed("Hyperliquid : liste des marchés absente".into()))?;
    let num = |x: &serde_json::Value| x.as_str().and_then(|s| s.parse::<f64>().ok()).or_else(|| x.as_f64());
    Ok(list
        .iter()
        .enumerate()
        .filter(|(_, u)| !u["isDelisted"].as_bool().unwrap_or(false))
        .filter_map(|(i, u)| {
            let name = u["name"].as_str()?;
            let coin = if dex.is_empty() || name.contains(':') {
                name.to_string()
            } else {
                format!("{dex}:{name}")
            };
            let c = &ctx[i];
            let mark = c["markPx"]
                .as_str()
                .map(str::to_string)
                .or_else(|| c["markPx"].as_f64().map(|f| f.to_string()))?;
            // champ `growthMode` de l'univers : "enabled" (ou vrai)
            let g = &u["growthMode"];
            Some(Quote {
                coin,
                mark,
                prev: num(&c["prevDayPx"]).unwrap_or(0.0),
                volume: num(&c["dayNtlVlm"]).unwrap_or(0.0),
                growth: g.as_str() == Some("enabled") || g.as_bool() == Some(true),
                oracle: num(&c["oraclePx"]).unwrap_or(0.0),
                oi: num(&c["openInterest"]).unwrap_or(0.0),
                funding: num(&c["funding"]).unwrap_or(0.0),
            })
        })
        .collect())
}

/// Les `n` plus populaires (volume 24 h, du plus gros au plus petit).
pub fn top(mut quotes: Vec<Quote>, n: usize) -> Vec<Quote> {
    quotes.sort_by(|a, b| b.volume.total_cmp(&a.volume));
    quotes.truncate(n);
    quotes
}

/// Combien de marchés par section.
pub const TOP: usize = 25;

/// Toujours dans la section Crypto, même hors du top (si Hyperliquid les a).
pub const ALWAYS: &[&str] = &["PAXG"];

/// Section « HIP-3 Growth mode » : tous les marchés HIP-3 en growth mode, du plus gros volume au plus petit.
pub fn growth_mode(hip3: &[Quote]) -> Vec<Quote> {
    top(hip3.iter().filter(|q| q.growth).cloned().collect(), usize::MAX)
}

/// Section Crypto : les `TOP` plus populaires, puis ceux de `ALWAYS` qui n'y sont pas déjà.
pub fn crypto(quotes: Vec<Quote>) -> Vec<Quote> {
    let extra: Vec<Quote> = quotes.iter().filter(|q| ALWAYS.contains(&q.coin.as_str())).cloned().collect();
    let mut out = top(quotes, TOP);
    for q in extra {
        if !out.iter().any(|o| o.coin == q.coin) {
            out.push(q);
        }
    }
    out
}

/// 📡 One market's live figures (Hyperliquid's `activeAssetCtx`, about once a second): the same fields as
/// `metaAndAssetCtxs`. A field that is missing leaves the price list's value as it was.
#[derive(Debug, Clone, PartialEq)]
pub struct Ctx {
    pub coin: String,
    pub mark: String,
    pub prev: Option<f64>,
    pub volume: Option<f64>,
    pub oracle: Option<f64>,
    pub oi: Option<f64>,
    pub funding: Option<f64>,
}

impl Ctx {
    /// Puts these figures into `q` (the same market only).
    pub fn apply(&self, q: &mut Quote) {
        if q.coin != self.coin {
            return;
        }
        q.mark.clone_from(&self.mark);
        let set = |v: Option<f64>, to: &mut f64| {
            if let Some(v) = v {
                *to = v;
            }
        };
        set(self.prev, &mut q.prev);
        set(self.volume, &mut q.volume);
        set(self.oracle, &mut q.oracle);
        set(self.oi, &mut q.oi);
        set(self.funding, &mut q.funding);
    }
}

/// At most this many markets in the live feed (≈ 315 bytes a second each).
pub const MAX_LIVE: usize = 60;

/// The live feed of `coins`, on one connection (one subscription a line).
pub fn ctx_sub(coins: &[String]) -> crate::live::LiveSub {
    crate::live::LiveSub {
        url: "wss://api.hyperliquid.xyz/ws".into(),
        subscribe: coins
            .iter()
            .map(|c| format!(r#"{{"method":"subscribe","subscription":{{"type":"activeAssetCtx","coin":"{c}"}}}}"#))
            .collect::<Vec<_>>()
            .join("\n"),
        ping: r#"{"method":"ping"}"#.into(),
        ..Default::default()
    }
}

/// The figures of an `activeAssetCtx` message (`None`: something else, or no price).
pub fn parse_ctx(msg: &str) -> Option<Ctx> {
    if !msg.contains("\"activeAssetCtx\"") {
        return None;
    }
    let v: serde_json::Value = serde_json::from_str(msg).ok()?;
    if v["channel"] != "activeAssetCtx" {
        return None;
    }
    let (coin, c) = (v["data"]["coin"].as_str()?, &v["data"]["ctx"]);
    let num = |k: &str| c[k].as_str().and_then(|s| s.parse::<f64>().ok()).or_else(|| c[k].as_f64());
    Some(Ctx {
        coin: coin.to_string(),
        mark: c["markPx"].as_str()?.to_string(),
        prev: num("prevDayPx"),
        volume: num("dayNtlVlm"),
        oracle: num("oraclePx"),
        oi: num("openInterest"),
        funding: num("funding"),
    })
}

/// The markets of the live feed: those the app asks for (on screen, the chart, the demo positions, the alerts),
/// only markets Hyperliquid has (`known`), at most [`MAX_LIVE`], sorted. The `bool`: all of them fit.
pub fn live_coins(wanted: &[String], known: impl Fn(&str) -> bool) -> (Vec<String>, bool) {
    let mut out: Vec<String> = vec![];
    for c in wanted.iter().filter(|c| known(c)) {
        if !out.contains(c) {
            out.push(c.clone());
        }
    }
    let fit = out.len() <= MAX_LIVE;
    out.truncate(MAX_LIVE);
    out.sort();
    (out, fit)
}

/// 🚦 A HIP-3 dex with a line on screen: asked every 30 s (as before), 60 s on a phone.
pub const HIP3_AFFICHE_MS: i64 = 30_000;
/// 🚦 The other dexes (no line on screen): each one asked once, one call every 20 s (so the HIP-3 sections fill
/// up without a burst), then no more.
pub const HIP3_DECOUVERTE_MS: i64 = 20_000;
/// 🚦 At least this long between two dex calls: spread over time, never a burst.
pub const HIP3_ECART_MS: i64 = 3_000;
/// 🚦 After a « 429 Too Many Requests »: no Hyperliquid call of the watchlist for 60 s.
pub const PAUSE_429_MS: i64 = 60_000;

/// Is this error a « 429 Too Many Requests » (desktop « HTTP 429 Too Many Requests … », browser « HTTP 429 : … »)?
pub fn est_429(err: &str) -> bool {
    err.contains("HTTP 429")
}

/// The HIP-3 dexes with a line on screen (`wanted`: « xyz:NVDA » → « xyz »), among the known `dexes`.
pub fn dexes_affiches(wanted: &[String], dexes: &[String]) -> Vec<String> {
    let mut out: Vec<String> = vec![];
    for c in wanted {
        if let Some((d, _)) = c.split_once(':') {
            if dexes.iter().any(|x| x == d) && !out.iter().any(|x| x == d) {
                out.push(d.to_string());
            }
        }
    }
    out
}

/// 🚦 Hyperliquid's budget is 1,200 points a minute per IP for every screen of the house (≈ 20 a
/// `metaAndAssetCtxs`): the watchlist used to ask its 10 HIP-3 dexes every 30 s (≈ 20 calls a minute a tab, « 429 »
/// on everything with three screens). Now (pure, tested): a dex with a line on screen keeps its 30 s; the others are
/// asked once each, spread; one call at a time, [`HIP3_ECART_MS`] apart; nothing for [`PAUSE_429_MS`] after a 429.
#[derive(Debug, Default)]
pub struct Plan {
    /// dex → time of its last call (ms).
    derniers: std::collections::HashMap<String, i64>,
    /// Time of the last dex call, and of the last discovery call.
    dernier_ms: Option<i64>,
    decouverte_ms: Option<i64>,
    /// No call before this time (after a 429).
    pause_ms: i64,
}

impl Plan {
    /// May the watchlist call Hyperliquid now (no 429 pause)?
    pub fn libre(&self, now_ms: i64) -> bool {
        now_ms >= self.pause_ms
    }

    /// The dex to ask now, if any: a dex on screen that is due (the most overdue first), else a dex never asked
    /// (discovery, one every [`HIP3_DECOUVERTE_MS`]). `cadence_ms`: [`HIP3_AFFICHE_MS`] (or a phone's minute).
    pub fn suivant(&self, now_ms: i64, dexes: &[String], affiches: &[String], cadence_ms: i64) -> Option<String> {
        if !self.libre(now_ms) || self.dernier_ms.is_some_and(|t| now_ms - t < HIP3_ECART_MS) {
            return None;
        }
        let du = |d: &String| self.derniers.get(d).map_or(i64::MAX, |t| now_ms - t);
        if let Some(d) = affiches.iter().filter(|d| du(d) >= cadence_ms).max_by_key(|d| du(d)) {
            return Some(d.clone());
        }
        if self.decouverte_ms.is_some_and(|t| now_ms - t < HIP3_DECOUVERTE_MS) {
            return None;
        }
        dexes.iter().find(|d| !self.derniers.contains_key(*d)).cloned()
    }

    /// `dex` is being asked now (`decouverte`: not on screen).
    pub fn appele(&mut self, dex: &str, now_ms: i64, decouverte: bool) {
        self.derniers.insert(dex.to_string(), now_ms);
        self.dernier_ms = Some(now_ms);
        if decouverte {
            self.decouverte_ms = Some(now_ms);
        }
    }

    /// A call ended with `err`: after a 429, every Hyperliquid call of the watchlist waits [`PAUSE_429_MS`].
    pub fn erreur(&mut self, err: &str, now_ms: i64) {
        if est_429(err) {
            self.pause_ms = self.pause_ms.max(now_ms + PAUSE_429_MS);
        }
    }
}

#[cfg(any(feature = "native", feature = "web"))]
pub use native::spawn;

#[cfg(any(feature = "native", feature = "web"))]
mod native {
    use super::*;
    use crate::http::Http;
    use crate::live::LiveEvent;
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    /// Without a live message for this many seconds, the 15-second poll takes over again.
    const SILENCE_S: u32 = 5;
    /// Seconds between two full polls (ranking, volumes, every market of the lists): without the live feed, while it
    /// runs, and while it runs without all the markets asked for (more than [`MAX_LIVE`]). 🚦 15 s at most without
    /// the feed (it was 3 s: ≈ 20 calls a minute, Hyperliquid's budget is shared by every screen).
    const FULL_S: u64 = 15;
    const FULL_LIVE_S: u64 = 60;
    const FULL_PART_S: u64 = 15;
    /// 📱 Phone: the full poll once a minute (≈ 73 KB each, mobile data), live feed or not.
    const PHONE_S: u64 = 60;

    async fn send<H: Http>(http: &H, r: Request) -> Result<String> {
        match r {
            Request::PostJson { url, body } => http.post_json(&url, &body).await,
            Request::Get(url) => http.get(&url).await,
        }
    }

    /// What the live feed received since the last look, and whether it stopped (hidden page).
    #[derive(Default)]
    struct Direct {
        ctx: HashMap<String, Ctx>,
        stopped: bool,
    }

    /// 📡 Opens the live feed of `coins` (number `g`): it stops as soon as `gen` changes, and when the page is hidden
    /// (then `stopped`: opened again on its return).
    fn start_feed(coins: &[String], g: u64, gen: Arc<AtomicU64>, direct: Arc<Mutex<Direct>>) {
        let parse = |m: &str| parse_ctx(m).map(|c| LiveEvent::Ctx(Box::new(c)));
        crate::live::spawn(ctx_sub(coins), parse, move |ev| {
            if gen.load(Ordering::SeqCst) != g {
                return false;
            }
            if crate::rt::hidden() {
                if let Ok(mut d) = direct.lock() {
                    d.stopped = true;
                }
                return false;
            }
            if let (LiveEvent::Ctx(c), Ok(mut d)) = (ev, direct.lock()) {
                d.ctx.insert(c.coin.clone(), *c);
            }
            true
        });
    }

    /// Rafraîchit la watchlist en arrière-plan : les prix en direct (flux `activeAssetCtx`, ≈ chaque seconde) des
    /// seuls marchés que l'app demande (`wanted` : ceux à l'écran, la chart, les positions démo, les alertes), un
    /// relevé complet à la minute. Si le flux se tait 5 s : le relevé d'avant reprend (PC aux 3 s, téléphone à la
    /// minute). HIP-3 : PC aux 30 s, téléphone à la minute. `on_update` reçoit les sections ; tout s'arrête quand il
    /// renvoie `false` (fenêtre fermée).
    pub fn spawn(wanted: Arc<Mutex<Vec<String>>>, on_update: impl Fn(Vec<Section>, Vec<Quote>) -> bool + Send + 'static) {
        crate::rt::spawn(move || async move {
            let http = crate::rt::Client::default();
            let mut dexes: Vec<String> = vec![];
            let mut hip3: Vec<Quote> = vec![];
            let mut growth: Vec<Quote> = vec![];
            let mut all_hip3: Vec<Quote> = vec![];
            let mut main: Vec<Quote> = vec![];
            let direct = Arc::new(Mutex::new(Direct::default()));
            let gen = Arc::new(AtomicU64::new(0));
            // the feed running (its markets, all fit) and what it was chosen from (the app's ask)
            let mut feed: Option<(Vec<String>, bool)> = None;
            let mut asked: Vec<String> = vec![];
            let mut quiet = SILENCE_S + 1;
            let mut since_full = u64::MAX;
            // 🚦 HIP-3: each dex's last markets, and when to ask which (see `Plan`)
            let mut plan = Plan::default();
            let mut par_dex: HashMap<String, Vec<Quote>> = HashMap::new();
            for tick in 0u64.. {
                let phone = crate::rt::phone();
                // 🚦 hidden tab: no call at all, back on its return
                let hidden = crate::rt::hidden();
                let now = crate::system_now_ms();
                // 📱 téléphone : HIP-3 à la minute (données mobiles)
                let hip3_s: u64 = if phone { 60 } else { 30 };
                let mut hip3_new = false;
                if !hidden && plan.libre(now) {
                    if dexes.is_empty() && tick % hip3_s == 0 {
                        match send(&http, dexes_request()).await {
                            Ok(b) => dexes = parse_dexes(&b).unwrap_or_default(),
                            Err(e) => plan.erreur(&e.to_string(), now),
                        }
                    }
                    let ask = wanted.lock().map(|w| w.clone()).unwrap_or_default();
                    let shown = dexes_affiches(&ask, &dexes);
                    if let Some(d) = plan.suivant(now, &dexes, &shown, hip3_s as i64 * 1000) {
                        plan.appele(&d, now, !shown.contains(&d));
                        match send(&http, quotes_request(&d)).await.and_then(|b| parse_quotes(&b, &d)) {
                            Ok(q) => {
                                par_dex.insert(d, q);
                                hip3_new = true;
                            }
                            Err(e) => plan.erreur(&e.to_string(), now),
                        }
                    }
                }
                if hip3_new {
                    // the dexes in their order (as before: the same lists, only asked less often)
                    let all: Vec<Quote> = dexes.iter().filter_map(|d| par_dex.get(d)).flatten().cloned().collect();
                    if !all.is_empty() {
                        growth = growth_mode(&all);
                        all_hip3 = all.clone();
                        hip3 = top(all, TOP);
                    }
                }
                // the live figures received since the last second (`stopped`: the page was hidden, the feed closed)
                let fresh: Vec<Ctx> = match direct.lock() {
                    Ok(mut d) => {
                        if std::mem::take(&mut d.stopped) {
                            feed = None;
                        }
                        d.ctx.drain().map(|(_, c)| c).collect()
                    }
                    Err(_) => vec![],
                };
                quiet = if fresh.is_empty() { quiet.saturating_add(1) } else { 0 };
                let every_s = match &feed {
                    _ if phone => PHONE_S,
                    // nothing on screen to follow live: the poll of the minute is enough
                    Some((coins, _)) if coins.is_empty() => FULL_LIVE_S,
                    Some((_, fit)) if quiet <= SILENCE_S => {
                        if *fit {
                            FULL_LIVE_S
                        } else {
                            FULL_PART_S
                        }
                    }
                    _ => FULL_S,
                };
                let mut changed = hip3_new && !all_hip3.is_empty();
                let mut polled = false;
                if since_full >= every_s && !hidden && plan.libre(now) {
                    since_full = 0; // an error waits for the next turn too (as before)
                    match send(&http, quotes_request("")).await.and_then(|b| parse_quotes(&b, "")) {
                        Ok(q) => (main, changed, polled) = (q, true, true),
                        Err(e) => plan.erreur(&e.to_string(), now),
                    }
                }
                since_full = since_full.saturating_add(1);
                // the live figures go on top (newer than the poll)
                for c in &fresh {
                    for q in main.iter_mut().chain(all_hip3.iter_mut()).chain(hip3.iter_mut()).chain(growth.iter_mut()) {
                        c.apply(q);
                    }
                }
                changed |= !fresh.is_empty();
                if changed && !main.is_empty() {
                    // tous les prix (pour les listes de l'utilisateur), puis les sections de la liste Hyperliquid
                    let every: Vec<Quote> = main.iter().chain(&all_hip3).cloned().collect();
                    let sections = vec![
                        Section {
                            name: "Crypto".into(),
                            quotes: crypto(main.clone()),
                        },
                        Section {
                            name: "HIP-3".into(),
                            quotes: hip3.clone(),
                        },
                        Section {
                            name: "HIP-3 Growth mode".into(),
                            quotes: growth.clone(),
                        },
                    ];
                    if !on_update(sections, every) {
                        gen.fetch_add(1, Ordering::SeqCst); // the feed stops too
                        return;
                    }
                }
                // 📡 the live feed: chosen again after a full poll or when the app asks for other markets
                let ask = wanted.lock().map(|w| w.clone()).unwrap_or_default();
                if !main.is_empty() && (polled || ask != asked || feed.is_none()) {
                    let known = |c: &str| main.iter().chain(&all_hip3).any(|q| q.coin == c);
                    let (coins, fit) = live_coins(&ask, known);
                    if feed.as_ref().map(|f| &f.0) != Some(&coins) {
                        let g = gen.fetch_add(1, Ordering::SeqCst) + 1; // the feed before stops
                        if !coins.is_empty() {
                            start_feed(&coins, g, gen.clone(), direct.clone());
                        }
                    }
                    feed = Some((coins, fit));
                    asked = ask;
                }
                crate::rt::sleep(Duration::from_secs(1)).await;
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requetes_et_lecture() {
        let Request::PostJson { body, .. } = quotes_request("xyz") else { panic!() };
        assert_eq!(body, r#"{"type":"metaAndAssetCtxs","dex":"xyz"}"#);
        let Request::PostJson { body, .. } = quotes_request("") else { panic!() };
        assert_eq!(body, r#"{"type":"metaAndAssetCtxs"}"#);
        let dexes = parse_dexes(r#"[null,{"name":"xyz","fullName":"XYZ"},{"name":"flx","fullName":"Felix"}]"#).unwrap();
        assert_eq!(dexes, vec!["xyz", "flx"]);
        let body = r#"[{"universe":[{"name":"BTC","szDecimals":5},{"name":"OLD","isDelisted":true},{"name":"ETH"}]},
            [{"markPx":"80261.3","prevDayPx":"80848.6","dayNtlVlm":"2000000000.0","oraclePx":"80250.0","openInterest":"25000.5","funding":"0.0000125"},{},{"markPx":"2300.5","prevDayPx":"2350","dayNtlVlm":"900000000"}]]"#;
        let q = parse_quotes(body, "").unwrap();
        assert_eq!(q.len(), 2);
        assert_eq!(
            q[0],
            Quote {
                coin: "BTC".into(),
                mark: "80261.3".into(),
                prev: 80848.6,
                volume: 2e9,
                growth: false,
                oracle: 80250.0,
                oi: 25000.5,
                funding: 0.0000125
            }
        );
        // HIP-3 : préfixe déjà là, ou ajouté s'il manque
        let hip = parse_quotes(r#"[{"universe":[{"name":"xyz:NVDA","growthMode":"enabled"},{"name":"TSLA"}]},[{"markPx":"180.1","prevDayPx":"178","dayNtlVlm":"5"},{"markPx":"250","prevDayPx":"260","dayNtlVlm":"9"}]]"#, "xyz").unwrap();
        assert_eq!((hip[0].coin.as_str(), hip[1].coin.as_str()), ("xyz:NVDA", "xyz:TSLA"));
        assert!(hip[0].growth && !hip[1].growth);
        assert_eq!(growth_mode(&hip).len(), 1, "growth mode seulement");
        let t = top(hip, 1);
        assert_eq!(t[0].coin, "xyz:TSLA", "le plus gros volume en premier");
        assert!(parse_quotes("pas du json", "").is_err());
        // PAXG toujours dans Crypto, même avec un petit volume
        let mut many: Vec<Quote> = (0..30)
            .map(|i| Quote {
                coin: format!("C{i}"),
                mark: "1".into(),
                prev: 1.0,
                volume: 1000.0 + i as f64,
                growth: false,
                oracle: 1.0,
                oi: 0.0,
                funding: 0.0,
            })
            .collect();
        many.push(Quote {
            coin: "PAXG".into(),
            mark: "4100".into(),
            prev: 4090.0,
            volume: 1.0,
            growth: false,
            oracle: 4100.0,
            oi: 0.0,
            funding: 0.0,
        });
        let c = crypto(many);
        assert_eq!(c.len(), TOP + 1);
        assert_eq!(c.last().map(|q| q.coin.as_str()), Some("PAXG"));
    }

    /// 📡 The live feed: one subscription a market, its figures read and put on the right market only (a missing
    /// field keeps the old value), other messages ignored; the markets chosen (the app's first, known ones only, at
    /// most MAX_LIVE, sorted).
    #[test]
    fn prix_en_direct() {
        let sub = ctx_sub(&["BTC".into(), "xyz:NVDA".into()]);
        assert_eq!(sub.url, "wss://api.hyperliquid.xyz/ws");
        assert_eq!(
            sub.subscribe.lines().collect::<Vec<_>>(),
            [
                r#"{"method":"subscribe","subscription":{"type":"activeAssetCtx","coin":"BTC"}}"#,
                r#"{"method":"subscribe","subscription":{"type":"activeAssetCtx","coin":"xyz:NVDA"}}"#
            ]
        );
        assert!(crate::net::autorise(&sub.url));
        let msg = r#"{"channel":"activeAssetCtx","data":{"coin":"BTC","ctx":{"funding":"0.0000083802","openInterest":"38569.82574","prevDayPx":"64166.0","dayNtlVlm":"3630855189.72","oraclePx":"64252.0","markPx":"64224.0","midPx":"64227.5"}}}"#;
        let c = parse_ctx(msg).expect("read");
        assert_eq!(
            (c.coin.as_str(), c.mark.as_str(), c.prev, c.oi),
            ("BTC", "64224.0", Some(64166.0), Some(38569.82574))
        );
        let q0 = Quote {
            coin: "BTC".into(),
            mark: "1".into(),
            prev: 2.0,
            volume: 3.0,
            growth: true,
            oracle: 4.0,
            oi: 5.0,
            funding: 6.0,
        };
        let mut q = q0.clone();
        c.apply(&mut q);
        assert_eq!(
            (q.mark.as_str(), q.prev, q.volume, q.oracle, q.funding),
            ("64224.0", 64166.0, 3630855189.72, 64252.0, 0.0000083802)
        );
        assert!(q.growth, "growth stays");
        let mut other = Quote {
            coin: "ETH".into(),
            ..q0.clone()
        };
        c.apply(&mut other);
        assert_eq!(other.mark, "1", "another market: untouched");
        let partial = parse_ctx(r#"{"channel":"activeAssetCtx","data":{"coin":"BTC","ctx":{"markPx":"7"}}}"#).unwrap();
        let mut q = q0.clone();
        partial.apply(&mut q);
        assert_eq!((q.mark.as_str(), q.prev, q.oi), ("7", 2.0, 5.0), "a missing field keeps its value");
        for other in [
            r#"{"channel":"subscriptionResponse","data":{"method":"subscribe","subscription":{"type":"activeAssetCtx","coin":"BTC"}}}"#,
            r#"{"channel":"pong"}"#,
            r#"{"channel":"activeAssetCtx","data":{"coin":"BTC","ctx":{}}}"#,
            "not json activeAssetCtx",
        ] {
            assert!(parse_ctx(other).is_none(), "{other}");
        }
        // the markets: only those asked for (on screen…), unknown ones out, no double, at most MAX_LIVE, sorted
        let ask: Vec<String> = ["xyz:GOLD", "NOPE", "ETH", "BTC", "ETH"].map(String::from).to_vec();
        let (coins, fit) = live_coins(&ask, |c| c != "NOPE");
        assert_eq!(coins, ["BTC", "ETH", "xyz:GOLD"]);
        assert!(fit);
        assert_eq!(live_coins(&[], |_| true), (vec![], true), "nothing on screen: no feed");
        let many: Vec<String> = (0..MAX_LIVE + 5).map(|i| format!("C{i:03}")).collect();
        let (coins, fit) = live_coins(&many, |_| true);
        assert_eq!(coins.len(), MAX_LIVE);
        assert!(!fit && coins.contains(&"C000".to_string()), "the first asked");
    }

    /// Runs the plan second by second for `secs` like the watchlist's loop: the calls of each dex.
    fn derouler(plan: &mut Plan, dexes: &[String], shown: &[String], from_s: i64, secs: i64) -> Vec<(i64, String)> {
        let mut out = vec![];
        for s in from_s..from_s + secs {
            let now = s * 1000;
            if let Some(d) = plan.suivant(now, dexes, shown, HIP3_AFFICHE_MS) {
                plan.appele(&d, now, !shown.contains(&d));
                out.push((s, d));
            }
        }
        out
    }

    fn dix() -> Vec<String> {
        ["abcd", "cash", "flx", "hyna", "io", "km", "mkts", "para", "vntl", "xyz"]
            .map(String::from)
            .to_vec()
    }

    #[test]
    fn dex_affiches_seulement() {
        let wanted: Vec<String> = ["BTC", "xyz:NVDA", "xyz:TSLA", "km:GOLD", "nope:X"].map(String::from).to_vec();
        assert_eq!(dexes_affiches(&wanted, &dix()), vec!["xyz", "km"]);
        assert!(dexes_affiches(&["BTC".into(), "ETH".into()], &dix()).is_empty(), "default list: no HIP-3 line");
    }

    #[test]
    fn hip3_sans_ligne_affichee_une_seule_fois_etalee() {
        // the default watchlist: no HIP-3 line on screen → each dex once, one every 20 s, then nothing
        let mut p = Plan::default();
        let calls = derouler(&mut p, &dix(), &[], 0, 600);
        assert_eq!(calls.len(), 10, "each dex once");
        assert!(calls.windows(2).all(|w| w[1].0 - w[0].0 >= 20), "spread: {calls:?}");
        // at most 3 dex calls in any minute (+ the main one = 4 metaAndAssetCtxs a minute)
        for start in 0..600 {
            assert!(calls.iter().filter(|(t, _)| (start..start + 60).contains(t)).count() <= 3);
        }
        assert!(derouler(&mut p, &dix(), &[], 600, 600).is_empty(), "then no more");
    }

    #[test]
    fn hip3_affiche_garde_ses_30_s() {
        let mut p = Plan::default();
        let shown = vec!["xyz".to_string(), "km".to_string()];
        let calls = derouler(&mut p, &dix(), &shown, 0, 300);
        for d in ["xyz", "km"] {
            let t: Vec<i64> = calls.iter().filter(|c| c.1 == d).map(|c| c.0).collect();
            assert!(t.len() >= 9 && t.windows(2).all(|w| w[1] - w[0] == 30), "{d}: every 30 s, {t:?}");
        }
        assert_eq!(calls.iter().filter(|c| c.1 == "xyz").count(), 10);
        // on screen first, never two calls less than 3 s apart (no burst)
        assert!(shown.contains(&calls[0].1) && shown.contains(&calls[1].1));
        assert!(calls.windows(2).all(|w| w[1].0 - w[0].0 >= HIP3_ECART_MS / 1000), "{calls:?}");
        // the others: still each once
        assert_eq!(calls.iter().filter(|c| c.1 == "flx").count(), 1);
    }

    #[test]
    fn apres_un_429_pause_de_60_s() {
        let mut p = Plan::default();
        assert!(p.libre(0));
        p.erreur("HTTP 500 : x", 0);
        assert!(p.libre(1), "another error: no pause");
        p.erreur("HTTP 429 : null", 1_000);
        assert!(!p.libre(60_999) && p.suivant(30_000, &dix(), &["xyz".into()], HIP3_AFFICHE_MS).is_none());
        assert!(p.libre(61_000) && p.suivant(61_000, &dix(), &["xyz".into()], HIP3_AFFICHE_MS).is_some());
        assert!(est_429("HTTP 429 Too Many Requests (Retry-After 5 s) : x") && !est_429("HTTP 500 : x"));
    }
}
