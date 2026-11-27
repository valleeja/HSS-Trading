//! Le **direct** (WebSocket) : la dernière bougie bouge avec le prix.
//! Partie pure (testée) : l'abonnement et la lecture des messages, par exchange. Partie réseau (`native`) : connexion,
//! ping, et reconnexion automatique si le lien tombe. Règle (comme TradingView) : une mise à jour **remplace** toute la
//! dernière bougie, ou en commence une nouvelle ; l'historique n'est jamais modifié.
//! Aussi les **trades** (chaque transaction) pour construire les bougies en secondes.
//! Bougies en direct : Hyperliquid, Binance, Bybit, OKX, BloFin, WEEX, Bitunix (chaque exchange écrit son abonnement et
//! sa lecture dans son fichier) ; Bitstamp, Coinbase et KuCoin (flux à jeton) par leurs trades. Trades (bougies en
//! secondes) : Hyperliquid, Bitstamp, Coinbase, KuCoin. La chart ouvre son direct avec `direct_chart`.

use crate::Exchange;
use argos_core::{Bar, Timeframe};

/// Où se connecter et quoi demander.
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct LiveSub {
    pub url: String,
    /// Message(s) d'abonnement (JSON) à envoyer une fois connecté, un par ligne.
    pub subscribe: String,
    /// Message pour garder la connexion ouverte (envoyé régulièrement) ; vide = rien à envoyer (l'exchange pingue).
    pub ping: String,
    /// 💤 The exchange answers no `ping` of the app with a message (Bitunix: nothing came back to its ping in the
    /// answers recorded on 23 Nov. 2026). The ping still leaves, to keep the link open, but a web page cannot tell the
    /// silence of this link from a quiet market: it does not judge it ([`LiveSub::judged`]). Only a page reads it: it
    /// is left out of the description written for the shell of the app, which gets the same text as before.
    #[serde(skip)]
    pub ping_unanswered: bool,
    /// Messages binaires compressés en gzip (HTX) : décompressés avant d'être lus.
    pub gzip: bool,
    /// Réponse aux messages de contrôle de l'exchange (battement de cœur de Crypto.com, ping de HTX…).
    pub reply: Reply,
}

impl LiveSub {
    /// 💤 Can a web page judge the silence of this connection ([`Silence`])? Only when the app pings it itself and the
    /// exchange answers those pings with a message: then a long silence proves the link is dead.
    pub fn judged(&self) -> bool {
        !self.ping.is_empty() && !self.ping_unanswered
    }
}

/// Messages de contrôle auxquels il faut répondre pour que l'exchange garde la connexion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum Reply {
    #[default]
    None,
    /// Crypto.com : `{"id":N,"method":"public/heartbeat"}` → `{"id":N,"method":"public/respond-heartbeat"}`.
    CryptoCom,
    /// HTX : `{"ping":T}` → `{"pong":T}`.
    Htx,
    /// KuCoin : rien à répondre (on pingue nous-mêmes), mais le message de bienvenue et les pong sont ignorés.
    KuCoin,
}

impl Reply {
    /// La réponse à envoyer à ce message, s'il en faut une.
    pub fn answer(self, msg: &str) -> Option<String> {
        match self {
            Reply::None | Reply::KuCoin => None,
            Reply::CryptoCom => {
                if !msg.contains("public/heartbeat") {
                    return None;
                }
                let v: serde_json::Value = serde_json::from_str(msg).ok()?;
                Some(format!(r#"{{"id":{},"method":"public/respond-heartbeat"}}"#, v["id"].as_i64()?))
            }
            Reply::Htx => {
                if !msg.starts_with("{\"ping\"") {
                    return None;
                }
                let v: serde_json::Value = serde_json::from_str(msg).ok()?;
                Some(format!(r#"{{"pong":{}}}"#, v["ping"].as_i64()?))
            }
        }
    }
}

/// 🛡️ Taille maximale d'un message du direct (4 Mio) : un message plus gros est ignoré (jamais lu ni décompressé).
pub const MAX_MSG: usize = 4 * 1024 * 1024;

/// Décompresse un message gzip (HTX) ; `None` s'il est illisible ou s'il dépasse `MAX_MSG` une fois décompressé (un
/// petit message ne peut pas se gonfler en gigaoctets).
pub fn gunzip(data: &[u8]) -> Option<String> {
    use std::io::Read;
    let mut s = String::new();
    flate2::read::GzDecoder::new(data).take(MAX_MSG as u64 + 1).read_to_string(&mut s).ok()?;
    (s.len() <= MAX_MSG).then_some(s)
}

/// Abonnement au direct des bougies ; `None` si cet exchange n'a pas encore de direct dans Argos.
pub fn subscription(ex: Exchange, base: &str, tf: Timeframe) -> Option<LiveSub> {
    match ex {
        Exchange::Hyperliquid => Some(LiveSub {
            url: "wss://api.hyperliquid.xyz/ws".into(),
            subscribe: format!(
                r#"{{"method":"subscribe","subscription":{{"type":"candle","coin":"{}","interval":"{}"}}}}"#,
                crate::exchanges::hyperliquid::coin(base),
                tf.id()
            ),
            ping: r#"{"method":"ping"}"#.into(),
            ..Default::default()
        }),
        Exchange::Binance => Some(crate::exchanges::binance::live_sub(base, tf)),
        Exchange::Bybit => Some(crate::exchanges::bybit::live_sub(base, tf)),
        Exchange::Okx => Some(crate::exchanges::okx::live_sub(base, tf)),
        Exchange::Blofin => Some(crate::exchanges::blofin::live_sub(base, tf)),
        Exchange::Weex => Some(crate::exchanges::weex::live_sub(base, tf)),
        Exchange::Bitunix => Some(crate::exchanges::bitunix::live_sub(base, tf)),
        // pas de canal de bougies : les trades, qu'Argos range dans les bougies (`fold_trade`)
        Exchange::Bitstamp => Some(crate::exchanges::bitstamp::trades_sub(&[base.to_string()])),
        Exchange::Coinbase => Some(crate::exchanges::coinbase::trades_sub(&[base.to_string()])),
        Exchange::Bitget => crate::exchanges::bitget::live_sub(base, tf),
        Exchange::Gate => crate::exchanges::gate::live_sub(base, tf),
        Exchange::Mexc => crate::exchanges::mexc::live_sub(base, tf),
        Exchange::Kraken => crate::exchanges::kraken::live_sub(base, tf),
        Exchange::Aster => Some(crate::exchanges::aster::live_sub(base, tf)),
        // 🔑 l'adresse de KuCoin demande d'abord un jeton : voir `direct_chart`
        Exchange::Kucoin => None,
        // Bitfinex : son canal de bougies ; pas de 4 h chez lui : ses trades, rangés dans la bougie de 4 h
        Exchange::Bitfinex | Exchange::BitfinexSpot => crate::exchanges::bitfinex::live_sub(base, ex == Exchange::Bitfinex, tf),
        e => crate::exchanges::spot::live_sub(e.spot_venue()?, base, tf),
    }
}

/// Le direct d'une chart : un abonnement prêt à ouvrir, ou 🔑 (KuCoin) les pièces d'un flux de trades dont l'adresse
/// demande d'abord un jeton public (`spawn_jeton` avec `whales::kucoin_preparer`, redemandé à chaque connexion).
#[derive(Debug, Clone, PartialEq)]
pub enum DirectChart {
    Pret(LiveSub),
    Jeton(Vec<String>),
}

/// Le direct de la chart de `base` sur `ex` : ses bougies (`tf`, l'unité téléchargée), ou ses trades (bougies en
/// secondes : `tf` = `None` ; KuCoin) ; `None` : pas de direct ici.
pub fn direct_chart(ex: Exchange, base: &str, tf: Option<Timeframe>) -> Option<DirectChart> {
    direct_chart_si(ex, base, tf, crate::direct())
}

/// [`direct_chart`] selon que les requêtes partent sans les règles d'un navigateur (`direct` : app de bureau, coque
/// Tauri) ou d'une page web.
fn direct_chart_si(ex: Exchange, base: &str, tf: Option<Timeframe>, direct: bool) -> Option<DirectChart> {
    if ex == Exchange::Kucoin {
        // 🔑 son jeton est refusé à une page web (CORS, comme ses bougies) : pas de jeton redemandé sans fin ; la chart
        // y prend le direct de l'exchange de secours dont elle montre les bougies
        return direct.then(|| DirectChart::Jeton(vec![base.to_string()]));
    }
    match tf {
        Some(tf) => subscription(ex, base, tf),
        None => trades_subscription(ex, base),
    }
    .map(DirectChart::Pret)
}

/// Abonnement aux **trades** en direct (chaque transaction) : sert à construire les bougies en secondes, comme
/// TradingView. `None` si cet exchange n'a pas encore de trades en direct dans Argos.
pub fn trades_subscription(ex: Exchange, base: &str) -> Option<LiveSub> {
    match ex {
        Exchange::Hyperliquid => Some(LiveSub {
            url: "wss://api.hyperliquid.xyz/ws".into(),
            subscribe: format!(r#"{{"method":"subscribe","subscription":{{"type":"trades","coin":"{base}"}}}}"#),
            ping: r#"{"method":"ping"}"#.into(),
            ..Default::default()
        }),
        Exchange::Bitstamp => Some(crate::exchanges::bitstamp::trades_sub(&[base.to_string()])),
        Exchange::Coinbase => Some(crate::exchanges::coinbase::trades_sub(&[base.to_string()])),
        Exchange::Bitfinex | Exchange::BitfinexSpot => Some(crate::exchanges::bitfinex::trades_sub(base, ex == Exchange::Bitfinex)),
        _ => None,
    }
}

/// Une transaction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Trade {
    /// Moment, en millisecondes UTC.
    pub time_ms: i64,
    pub price: f64,
    pub size: f64,
}

/// The trades of a live message (empty: something else). A trade that cannot be (a price or a size that is not a
/// number above zero, `feed::price_size_ok`) is left out, whatever the exchange.
pub fn parse_trades(ex: Exchange, msg: &str) -> Vec<Trade> {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(msg) else {
        return vec![];
    };
    let mut trades: Vec<Trade> = match ex {
        Exchange::Hyperliquid if v["channel"] == "trades" => v["data"]
            .as_array()
            .map(|l| {
                l.iter()
                    .filter_map(|t| {
                        let n = |k: &str| t[k].as_str().and_then(|s| s.parse::<f64>().ok());
                        Some(Trade {
                            time_ms: t["time"].as_i64()?,
                            price: n("px")?,
                            size: n("sz")?,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default(),
        Exchange::Bitstamp => crate::exchanges::bitstamp::parse_trades(msg),
        Exchange::Coinbase => crate::exchanges::coinbase::parse_trades(msg),
        Exchange::Kucoin => crate::exchanges::kucoin::parse_trades(msg),
        // Bitfinex: its 4 h / 2 h / 3 min (made from shorter candles) and its seconds follow the live trades
        Exchange::Bitfinex | Exchange::BitfinexSpot => crate::exchanges::bitfinex::parse_live_trades(msg),
        _ => vec![],
    };
    trades.retain(|t| argos_core::feed::price_size_ok(t.price, t.size));
    trades
}

/// Adds a trade to the candles of `secs` seconds: it completes the candle of its period, or begins a new one. A trade
/// older than the last candle is ignored. So is a trade that cannot be (`feed::price_size_ok`), as an incoherent
/// candle is in [`apply`]: the candles of the chart never take a price or a volume that is not one. `true` when a
/// new candle began.
pub fn fold_trade(bars: &mut Vec<Bar>, t: Trade, secs: i64) -> bool {
    if !argos_core::feed::price_size_ok(t.price, t.size) {
        return false;
    }
    let start = (t.time_ms / 1000).div_euclid(secs) * secs;
    match bars.last_mut() {
        Some(b) if b.time == start => {
            b.high = b.high.max(t.price);
            b.low = b.low.min(t.price);
            b.close = t.price;
            b.volume += t.size;
            false
        }
        Some(b) if start < b.time => false,
        _ => {
            bars.push(Bar {
                time: start,
                open: t.price,
                high: t.price,
                low: t.price,
                close: t.price,
                volume: t.size,
            });
            true
        }
    }
}

/// Bougie d'un message du direct (`None` : autre chose, ex. confirmation d'abonnement ou pong).
pub fn parse_update(ex: Exchange, msg: &str) -> Option<Bar> {
    let v: serde_json::Value = serde_json::from_str(msg).ok()?;
    match ex {
        Exchange::Hyperliquid => {
            if v["channel"] != "candle" {
                return None;
            }
            let k = &v["data"];
            let num = |key: &str| k[key].as_str().and_then(|s| s.parse::<f64>().ok()).or_else(|| k[key].as_f64());
            Some(Bar {
                time: k["t"].as_i64()? / 1000,
                open: num("o")?,
                high: num("h")?,
                low: num("l")?,
                close: num("c")?,
                volume: num("v").unwrap_or(0.0),
            })
        }
        Exchange::Binance => crate::exchanges::binance::parse_live(msg),
        Exchange::Bybit => crate::exchanges::bybit::parse_live(msg),
        Exchange::Okx => crate::exchanges::okx::parse_live(msg),
        Exchange::Blofin => crate::exchanges::blofin::parse_live(msg),
        Exchange::Weex => crate::exchanges::weex::parse_live(msg),
        Exchange::Bitunix => crate::exchanges::bitunix::parse_live(msg),
        Exchange::Bitstamp | Exchange::Coinbase => None,
        Exchange::Bitget => crate::exchanges::bitget::parse_live(msg),
        Exchange::Gate => crate::exchanges::gate::parse_live(msg),
        Exchange::Mexc => crate::exchanges::mexc::parse_live(msg),
        Exchange::Kraken => crate::exchanges::kraken::parse_live(msg),
        Exchange::Aster => crate::exchanges::aster::parse_live(msg),
        Exchange::Kucoin => None,
        Exchange::Bitfinex | Exchange::BitfinexSpot => crate::exchanges::bitfinex::parse_live(msg),
        e => crate::exchanges::spot::parse_live(e.spot_venue()?, msg),
    }
}

/// Applique une bougie du direct : remplace la dernière (même heure) ou en ajoute une nouvelle. Une bougie plus vieille
/// que la dernière, ou incohérente (`bar_ok` : haut sous la clôture, prix négatif, NaN…), est ignorée (l'historique ne
/// change jamais). `true` si une **nouvelle** bougie a commencé.
pub fn apply(bars: &mut Vec<Bar>, b: Bar) -> bool {
    if !argos_core::feed::bar_ok(&b) {
        return false;
    }
    match bars.last_mut() {
        Some(last) if last.time == b.time => {
            *last = b;
            false
        }
        Some(last) if b.time < last.time => false,
        _ => {
            bars.push(b);
            true
        }
    }
}

/// Ce qui arrive du direct.
#[derive(Debug, Clone, PartialEq)]
pub enum LiveEvent {
    Connected,
    Bar(Bar),
    /// Trades (abonnement aux trades).
    Trades(Vec<Trade>),
    /// 🐋 Grosses transactions (flux des baleines).
    Whales(Vec<crate::whales::Whale>),
    /// 📖 Carnet d'ordres et ⚡ trades (modules de la colonne).
    Book(crate::book::Book),
    Tape(Vec<crate::whales::Whale>),
    /// 💥 Liquidations (module de la colonne).
    Liqs(Vec<crate::liq::Liq>),
    /// 📖 Argos Murs: one update of Binance perp's diff depth stream.
    Profondeur(Box<crate::murs::MajProfondeur>),
    /// 📡 One market's live figures (the watchlist's prices).
    Ctx(Box<crate::watchlist::Ctx>),
    /// Lien perdu (raison) : Argos se reconnecte tout seul.
    Disconnected(String),
    /// 🔑 Le flux ne peut pas s'ouvrir (jeton refusé, réseau…) : nouvel essai plus tard (`spawn_jeton`).
    Indisponible(String),
}

/// Événement d'un message du direct des bougies / trades de `ex` (`None` : autre chose).
pub fn parse_event(ex: Exchange, msg: &str) -> Option<LiveEvent> {
    match parse_update(ex, msg) {
        Some(b) => Some(LiveEvent::Bar(b)),
        None => Some(parse_trades(ex, msg)).filter(|l| !l.is_empty()).map(LiveEvent::Trades),
    }
}

/// 💤 Sans aucun message pendant ce temps, la connexion du direct est refaite (sortie de veille, réseau coupé).
pub const SILENCE: std::time::Duration = std::time::Duration::from_secs(60);

/// 💤 The time an exchange has to answer a ping before its silence counts (see [`Silence`]).
pub const ANSWER: std::time::Duration = std::time::Duration::from_secs(30);

/// 💤 The silence of a live connection in a web page: is the link dead though nobody said so (a computer back from
/// sleep, a network cut with no close)? The desktop reads the time since the last message (`native::run_once`). A
/// page does the same, but only where silence proves something. It cannot see the pings an exchange sends by itself
/// (the browser answers them alone), so there a market where nothing trades and a dead link look the same: a
/// connection is judged only when the app pings it itself and the exchange answers those pings with a message
/// ([`LiveSub::judged`]).
///
/// Dead: nothing at all came for more than [`SILENCE`], though a ping left more than [`ANSWER`] ago. The second
/// condition is for a page whose timers were held back (a tab in the background, a computer asleep): the time went
/// by with no ping sent, so the next ping gets its chance to be answered before anything is judged. With pings every
/// 20 s both conditions come true together, at the moment the desktop cuts: 60 to 80 s after the last message.
///
/// The moments are times since any fixed origin (the opening of the connection, in the page).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Silence {
    /// When the last message came (or when the connection opened).
    heard: std::time::Duration,
    /// When the first ping since that message left (`None`: no ping since).
    asked: Option<std::time::Duration>,
}

impl Silence {
    /// A connection that just opened, or that just received a message (any message: data, an answer to a ping).
    pub fn heard(now: std::time::Duration) -> Self {
        Self { heard: now, asked: None }
    }

    /// A ping leaves.
    pub fn asked(&mut self, now: std::time::Duration) {
        self.asked.get_or_insert(now);
    }

    /// Is the link dead? (Read just before a ping leaves.)
    pub fn dead(&self, now: std::time::Duration) -> bool {
        now.saturating_sub(self.heard) > SILENCE && self.asked.is_some_and(|asked| now.saturating_sub(asked) > ANSWER)
    }
}

#[cfg(feature = "native")]
pub use native::spawn;
#[cfg(all(feature = "web", not(feature = "native")))]
pub use web::spawn;

/// 🔑 Attente la plus longue entre deux essais d'un direct à jeton (`spawn_jeton`).
pub const ATTENTE_JETON_MAX: std::time::Duration = std::time::Duration::from_secs(60);

/// 🔑 Un direct dont l'adresse demande un jeton public (KuCoin) : `preparer` le demande à **chaque** (re)connexion (un
/// jeton expire), puis le direct s'ouvre comme les autres (`spawn`). À une coupure, `on_event` la reçoit (la minute en
/// cours devient incomplète) et un jeton neuf est demandé. Si la demande échoue : `LiveEvent::Indisponible(raison)`,
/// puis un nouvel essai, avec une attente qui double (2 s, 4 s… jusqu'à `ATTENTE_JETON_MAX`). Tout s'arrête quand
/// `on_event` renvoie `false`.
#[cfg(any(feature = "native", feature = "web"))]
pub fn spawn_jeton<F>(
    preparer: impl Fn() -> F + Send + 'static,
    parse: impl Fn(&str) -> Option<LiveEvent> + Send + 'static,
    on_event: impl Fn(LiveEvent) -> bool + Send + 'static,
) where
    F: std::future::Future<Output = Result<LiveSub, String>> + 'static,
{
    use std::sync::atomic::{AtomicU8, Ordering};
    use std::sync::{Arc, Mutex};
    // état d'une connexion : en route, connectée, coupée (jeton à refaire), arrêt demandé
    const EN_ROUTE: u8 = 0;
    const CONNECTEE: u8 = 1;
    const COUPEE: u8 = 2;
    const FINI: u8 = 3;
    let parse = Arc::new(Mutex::new(parse));
    let on_event = Arc::new(Mutex::new(on_event));
    crate::rt::spawn(move || async move {
        let mut attente = std::time::Duration::from_secs(2);
        loop {
            match preparer().await {
                Err(e) => {
                    if !on_event.lock().is_ok_and(|f| f(LiveEvent::Indisponible(e))) {
                        return;
                    }
                }
                Ok(sub) => {
                    let etat = Arc::new(AtomicU8::new(EN_ROUTE));
                    let vue = Arc::new(std::sync::atomic::AtomicBool::new(false));
                    let (p, o, e2, v2) = (parse.clone(), on_event.clone(), etat.clone(), vue.clone());
                    spawn(
                        sub,
                        move |t| p.lock().ok().and_then(|f| f(t)),
                        move |ev| {
                            let suite = match ev {
                                LiveEvent::Connected => CONNECTEE,
                                LiveEvent::Disconnected(_) => COUPEE,
                                _ => EN_ROUTE,
                            };
                            if !o.lock().is_ok_and(|f| f(ev)) {
                                e2.store(FINI, Ordering::SeqCst);
                                return false;
                            }
                            if suite == CONNECTEE {
                                v2.store(true, Ordering::SeqCst);
                            }
                            if suite != EN_ROUTE {
                                e2.store(suite, Ordering::SeqCst);
                            }
                            // coupée : ce direct s'arrête, un jeton neuf rouvrira le suivant
                            suite != COUPEE
                        },
                    );
                    loop {
                        crate::rt::sleep(std::time::Duration::from_millis(250)).await;
                        match etat.load(Ordering::SeqCst) {
                            COUPEE => break,
                            FINI => return,
                            _ => {}
                        }
                    }
                    // elle a marché : le prochain jeton est demandé vite
                    if vue.load(Ordering::SeqCst) {
                        attente = std::time::Duration::from_secs(2);
                    }
                }
            }
            crate::rt::sleep(std::time::Duration::from_millis(crate::http::jitter(attente.as_millis() as u64))).await;
            attente = (attente * 2).min(ATTENTE_JETON_MAX);
        }
    });
}

/// Direct dans le navigateur : le WebSocket de la page, mêmes règles que le desktop (abonnement à la connexion,
/// ping régulier, reconnexion 2 s puis jusqu'à 30 s, arrêt quand `on_event` renvoie `false`). 💤 And its watch on
/// silence, where the exchange answers the app's pings: a link that answers nothing any more is made again
/// ([`Silence`], [`LiveSub::judged`]).
#[cfg(all(feature = "web", not(feature = "native")))]
mod web {
    use super::{LiveEvent, LiveSub, Silence};
    use std::cell::Cell;
    use std::rc::Rc;
    use wasm_bindgen::prelude::*;
    use wasm_bindgen::JsCast;
    use web_sys::{MessageEvent, WebSocket};

    pub fn spawn(sub: LiveSub, parse: impl Fn(&str) -> Option<LiveEvent> + Send + 'static, on_event: impl Fn(LiveEvent) -> bool + Send + 'static) {
        // 📱 dans la coque Tauri : le cœur Rust ouvre le WebSocket (la page n'a pas de réseau)
        if crate::coque::active() {
            return par_la_coque(sub, Rc::new(parse), Rc::new(on_event));
        }
        connect(Rc::new(sub), Rc::new(parse), Rc::new(on_event), Rc::new(Cell::new(false)), 2);
    }

    /// Le direct par le cœur de la coque : il se connecte, s'abonne, pingue, répond aux contrôles et se reconnecte
    /// lui-même ; la page reçoit chaque message par un canal (`{k: "c"}` connecté, `{k: "t", t}` texte, `{k: "d", r}`
    /// coupé) et le lit comme avant. `on_event` qui rend `false` : la page demande au cœur de fermer.
    fn par_la_coque(sub: LiveSub, parse: Rc<dyn Fn(&str) -> Option<LiveEvent>>, on_event: Rc<dyn Fn(LiveEvent) -> bool>) {
        let Ok(json) = serde_json::to_string(&sub) else { return };
        let id: Rc<Cell<Option<f64>>> = Rc::new(Cell::new(None));
        let fini = Rc::new(Cell::new(false));
        let (id2, fini2) = (id.clone(), fini.clone());
        let fermer = move |id: &Rc<Cell<Option<f64>>>| {
            if let Some(n) = id.get() {
                let args = crate::coque::objet(&[("id", JsValue::from_f64(n))]);
                wasm_bindgen_futures::spawn_local(async move {
                    let _ = crate::coque::invoke("direct_fermer", &args).await;
                });
            }
        };
        let Some(canal) = crate::coque::canal(move |m: JsValue| {
            if fini2.get() {
                return;
            }
            let champ = |k: &str| js_sys::Reflect::get(&m, &JsValue::from_str(k)).ok().and_then(|v| v.as_string());
            let ev = match champ("k").as_deref() {
                Some("c") => Some(LiveEvent::Connected),
                Some("d") => Some(LiveEvent::Disconnected(champ("r").unwrap_or_default())),
                Some("t") => champ("t").and_then(|t| parse(&t)),
                _ => None,
            };
            if let Some(ev) = ev {
                if !on_event(ev) {
                    fini2.set(true);
                    fermer(&id2);
                }
            }
        }) else {
            return;
        };
        wasm_bindgen_futures::spawn_local(async move {
            let args = crate::coque::objet(&[("abonnement", JsValue::from_str(&json)), ("canal", canal)]);
            if let Ok(v) = crate::coque::invoke("direct_ouvrir", &args).await {
                id.set(v.as_f64());
                // déjà fini avant même la réponse : on ferme tout de suite
                if fini.get() {
                    if let Some(n) = id.get() {
                        let args = crate::coque::objet(&[("id", JsValue::from_f64(n))]);
                        let _ = crate::coque::invoke("direct_fermer", &args).await;
                    }
                }
            }
        });
    }

    fn connect(sub: Rc<LiveSub>, parse: Rc<dyn Fn(&str) -> Option<LiveEvent>>, on_event: Rc<dyn Fn(LiveEvent) -> bool>, stop: Rc<Cell<bool>>, wait: u32) {
        if !crate::net::autorise(&sub.url) {
            return;
        }
        let Ok(ws) = WebSocket::new(&sub.url) else { return };
        // 💤 the silence of this connection, read at each ping (`Silence`)
        let born = web_time::Instant::now();
        let silence = Rc::new(Cell::new(Silence::heard(born.elapsed())));
        let (ws2, s2, ev2, st2, si2) = (ws.clone(), sub.clone(), on_event.clone(), stop.clone(), silence.clone());
        let open = Closure::<dyn FnMut()>::new(move || {
            si2.set(Silence::heard(born.elapsed()));
            for line in s2.subscribe.lines().filter(|l| !l.is_empty()) {
                let _ = ws2.send_with_str(line);
            }
            if !ev2(LiveEvent::Connected) {
                st2.set(true);
                let _ = ws2.close();
            }
        });
        ws.set_onopen(Some(open.as_ref().unchecked_ref()));
        open.forget();
        let (ws3, ev3, st3, p3) = (ws.clone(), on_event.clone(), stop.clone(), parse.clone());
        if sub.gzip {
            ws.set_binary_type(web_sys::BinaryType::Arraybuffer);
        }
        let (s3, ws5, si3) = (sub.clone(), ws.clone(), silence.clone());
        let msg = Closure::<dyn FnMut(MessageEvent)>::new(move |e: MessageEvent| {
            // (any message, read or not: the link lives)
            si3.set(Silence::heard(born.elapsed()));
            let data = e.data();
            let t = match data.as_string() {
                Some(t) if t.len() > super::MAX_MSG => return,
                Some(t) => t,
                None if s3.gzip => match data
                    .dyn_into::<js_sys::ArrayBuffer>()
                    .ok()
                    .filter(|b| (b.byte_length() as usize) <= super::MAX_MSG)
                    .and_then(|b| super::gunzip(&js_sys::Uint8Array::new(&b).to_vec()))
                {
                    Some(t) => t,
                    None => return,
                },
                None => return,
            };
            if let Some(r) = s3.reply.answer(&t) {
                let _ = ws5.send_with_str(&r);
                return;
            }
            if let Some(ev) = p3(&t) {
                if !ev3(ev) {
                    st3.set(true);
                    let _ = ws3.close();
                }
            }
        });
        ws.set_onmessage(Some(msg.as_ref().unchecked_ref()));
        msg.forget();
        // the end of this connection, said once: by the browser (`onclose`), or 💤 by the silence of the exchange
        let ping: Rc<Cell<Option<i32>>> = Rc::new(Cell::new(None));
        let over: Rc<dyn Fn(&str)> = {
            let (ping, ended, sub) = (ping.clone(), Cell::new(false), sub.clone());
            Rc::new(move |why: &str| {
                if ended.replace(true) {
                    return;
                }
                if let (Some(id), Some(w)) = (ping.take(), web_sys::window()) {
                    w.clear_interval_with_handle(id);
                }
                if stop.get() || !on_event(LiveEvent::Disconnected(why.into())) {
                    return;
                }
                let (sub, parse, on_event, stop) = (sub.clone(), parse.clone(), on_event.clone(), stop.clone());
                let again = Closure::once_into_js(move || connect(sub, parse, on_event, stop, (wait * 2).min(30)));
                if let Some(w) = web_sys::window() {
                    // attente croissante avec un petit délai au hasard (toutes les connexions ne repartent pas ensemble)
                    let ms = crate::http::jitter(wait as u64 * 1000) as i32;
                    let _ = w.set_timeout_with_callback_and_timeout_and_arguments_0(again.unchecked_ref(), ms);
                }
            })
        };
        // ping toutes les 20 s tant que la connexion vit
        if !sub.ping.is_empty() {
            let (ws4, p, over, judged) = (ws.clone(), sub.ping.clone(), over.clone(), sub.judged());
            let tick = Closure::<dyn FnMut()>::new(move || {
                if ws4.ready_state() != WebSocket::OPEN {
                    return;
                }
                let (now, mut heard) = (born.elapsed(), silence.get());
                // 💤 the exchange answers nothing any more: the link is dead though the browser has not said so (its
                // own close can come minutes later, or never). The dead socket is let go, it tells this page nothing
                // more, and the connection is made again now. (Only where the exchange answers pings: `judged`.)
                if judged && heard.dead(now) {
                    ws4.set_onclose(None);
                    ws4.set_onmessage(None);
                    let _ = ws4.close();
                    return over(&format!("plus rien reçu depuis {} s (veille ou réseau coupé ?)", super::SILENCE.as_secs()));
                }
                if ws4.send_with_str(&p).is_ok() {
                    heard.asked(now);
                    silence.set(heard);
                }
            });
            ping.set(web_sys::window().and_then(|w| {
                w.set_interval_with_callback_and_timeout_and_arguments_0(tick.as_ref().unchecked_ref(), 20_000)
                    .ok()
            }));
            tick.forget();
        }
        let close = Closure::<dyn FnMut()>::new(move || over("connexion fermée"));
        ws.set_onclose(Some(close.as_ref().unchecked_ref()));
        close.forget();
    }
}

#[cfg(feature = "native")]
mod native {
    use super::{LiveEvent, LiveSub, SILENCE};
    use futures_util::{SinkExt, StreamExt};
    use std::time::Duration;
    use tokio_tungstenite::tungstenite::Message;

    /// Lance le direct dans son propre fil : `on_event` reçoit chaque événement. Reconnexion automatique (2 s, puis
    /// jusqu'à 30 s d'attente si ça échoue encore). Le fil s'arrête quand `on_event` renvoie `false`.
    pub fn spawn(
        sub: LiveSub,
        parse: impl Fn(&str) -> Option<LiveEvent> + Send + 'static,
        on_event: impl Fn(LiveEvent) -> bool + Send + 'static,
    ) -> std::thread::JoinHandle<()> {
        std::thread::spawn(move || {
            let Ok(rt) = tokio::runtime::Builder::new_current_thread().enable_all().build() else {
                return;
            };
            rt.block_on(async move {
                let mut wait = 2u64;
                loop {
                    let reason = match run_once(&sub, &parse, &on_event).await {
                        Ok(true) => return, // la fenêtre est fermée
                        Ok(false) => "connexion fermée par l'exchange".to_string(),
                        Err(e) => e,
                    };
                    if !on_event(LiveEvent::Disconnected(reason)) {
                        return;
                    }
                    // attente croissante (2 s → 30 s) avec un petit délai au hasard : après une coupure, toutes les
                    // connexions ne repartent pas au même moment
                    tokio::time::sleep(Duration::from_millis(crate::http::jitter(wait * 1000))).await;
                    wait = (wait * 2).min(30);
                }
            });
        })
    }

    /// Une connexion : `Ok(true)` = arrêt demandé, `Ok(false)` = fermée par l'exchange, `Err` = erreur.
    async fn run_once(sub: &LiveSub, parse: &impl Fn(&str) -> Option<LiveEvent>, on_event: &impl Fn(LiveEvent) -> bool) -> Result<bool, String> {
        crate::net::verifier(&sub.url).map_err(|e| e.to_string())?;
        let (ws, _) = tokio_tungstenite::connect_async(sub.url.as_str()).await.map_err(|e| e.to_string())?;
        let (mut tx, mut rx) = ws.split();
        for line in sub.subscribe.lines().filter(|l| !l.is_empty()) {
            tx.send(Message::Text(line.into())).await.map_err(|e| e.to_string())?;
        }
        if !on_event(LiveEvent::Connected) {
            return Ok(true);
        }
        // 20 s : sous la limite des exchanges les plus pressés (OKX coupe après 30 s sans message)
        let mut ping = tokio::time::interval(Duration::from_secs(20));
        let mut dernier = std::time::Instant::now();
        loop {
            tokio::select! {
                msg = rx.next() => match msg {
                    Some(Ok(m @ (Message::Text(_) | Message::Binary(_)))) => {
                        dernier = std::time::Instant::now();
                        if m.len() > super::MAX_MSG {
                            continue;
                        }
                        let t = match m {
                            Message::Text(t) => t.to_string(),
                            Message::Binary(b) if sub.gzip => match super::gunzip(&b) {
                                Some(t) => t,
                                None => continue,
                            },
                            _ => continue,
                        };
                        if let Some(r) = sub.reply.answer(&t) {
                            tx.send(Message::Text(r)).await.map_err(|e| e.to_string())?;
                            continue;
                        }
                        if let Some(ev) = parse(&t) {
                            if !on_event(ev) {
                                return Ok(true);
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => return Ok(false),
                    Some(Ok(_)) => {}
                    Some(Err(e)) => return Err(e.to_string()),
                },
                _ = ping.tick() => {
                    // 💤 plus rien reçu depuis longtemps (ordinateur sorti de veille, réseau coupé sans fermeture) :
                    // la connexion est morte même si personne ne l'a dit ; on la refait
                    if dernier.elapsed() > SILENCE {
                        return Err(format!("plus rien reçu depuis {} s (veille ou réseau coupé ?)", SILENCE.as_secs()));
                    }
                    if !sub.ping.is_empty() {
                        tx.send(Message::Text(sub.ping.clone())).await.map_err(|e| e.to_string())?;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar(t: i64, c: f64) -> Bar {
        Bar {
            time: t,
            open: 1.0,
            high: c.max(1.0),
            low: 1.0,
            close: c,
            volume: 1.0,
        }
    }

    #[test]
    fn abonnement_et_lecture_hyperliquid() {
        let s = subscription(Exchange::Hyperliquid, "BTC", Timeframe::M30).unwrap();
        assert_eq!(s.url, "wss://api.hyperliquid.xyz/ws");
        assert!(s.subscribe.contains(r#""type":"candle""#) && s.subscribe.contains(r#""coin":"BTC""#) && s.subscribe.contains(r#""interval":"30m""#));
        assert!(
            Exchange::ALL
                .iter()
                .filter(|e| **e != Exchange::Kucoin)
                .all(|e| subscription(*e, "BTC", Timeframe::M30).is_some()),
            "tous les exchanges ont leur direct (sauf KuCoin, qui demande un jeton)"
        );
        let msg = r#"{"channel":"candle","data":{"t":1792962000000,"T":1792963799999,"s":"BTC","i":"30m","o":"80400.0","c":"80512.5","h":"80600.0","l":"80350.0","v":"123.45","n":987}}"#;
        assert_eq!(
            parse_update(Exchange::Hyperliquid, msg),
            Some(Bar {
                time: 1_792_962_000,
                open: 80400.0,
                high: 80600.0,
                low: 80350.0,
                close: 80512.5,
                volume: 123.45
            })
        );
        assert_eq!(parse_update(Exchange::Hyperliquid, r#"{"channel":"subscriptionResponse","data":{}}"#), None);
        assert_eq!(parse_update(Exchange::Hyperliquid, r#"{"channel":"pong"}"#), None);
        assert_eq!(parse_update(Exchange::Hyperliquid, "pas du json"), None);
    }

    #[test]
    fn direct_des_autres_exchanges() {
        let b = |t: i64| Bar {
            time: t,
            open: 80400.0,
            high: 80600.0,
            low: 80350.0,
            close: 80512.5,
            volume: 12.5,
        };
        // Binance
        let s = subscription(Exchange::Binance, "BTC", Timeframe::M30).unwrap();
        assert_eq!(s.subscribe, r#"{"method":"SUBSCRIBE","params":["btcusdt@kline_30m"],"id":1}"#);
        assert_eq!(
            s.url, "wss://fstream.binance.com/market/ws",
            "route /market : l'ancienne /ws n'envoie plus rien"
        );
        assert!(s.ping.is_empty(), "Binance pingue lui-même");
        let msg = r#"{"e":"kline","E":1792962001000,"s":"BTCUSDT","k":{"t":1792962000000,"T":1792963799999,"s":"BTCUSDT","i":"30m","o":"80400.0","c":"80512.5","h":"80600.0","l":"80350.0","v":"12.5","x":false}}"#;
        assert_eq!(parse_update(Exchange::Binance, msg), Some(b(1_792_962_000)));
        assert_eq!(parse_update(Exchange::Binance, r#"{"result":null,"id":1}"#), None);
        // Bybit
        let s = subscription(Exchange::Bybit, "BTC", Timeframe::M30).unwrap();
        assert_eq!(s.subscribe, r#"{"op":"subscribe","args":["kline.30.BTCUSDT"]}"#);
        let msg = r#"{"topic":"kline.30.BTCUSDT","data":[{"start":1792962000000,"end":1792963799999,"interval":"30","open":"80400.0","close":"80512.5","high":"80600.0","low":"80350.0","volume":"12.5","turnover":"1","confirm":false,"timestamp":1792962001000}],"ts":1792962001000,"type":"snapshot"}"#;
        assert_eq!(parse_update(Exchange::Bybit, msg), Some(b(1_792_962_000)));
        assert_eq!(parse_update(Exchange::Bybit, r#"{"success":true,"ret_msg":"pong","op":"ping"}"#), None);
        // OKX et BloFin (même format)
        let s = subscription(Exchange::Okx, "BTC", Timeframe::M30).unwrap();
        assert_eq!(s.subscribe, r#"{"op":"subscribe","args":[{"channel":"candle30m","instId":"BTC-USDT-SWAP"}]}"#);
        let s = subscription(Exchange::Blofin, "BTC", Timeframe::M30).unwrap();
        assert_eq!(s.subscribe, r#"{"op":"subscribe","args":[{"channel":"candle30m","instId":"BTC-USDT"}]}"#);
        // (the volume of their perps in coins at index 6; 5 counts contracts)
        let msg = r#"{"arg":{"channel":"candle30m","instId":"BTC-USDT-SWAP"},"data":[["1792962000000","80400.0","80600.0","80350.0","80512.5","1250","12.5","1006406.25","0"]]}"#;
        assert_eq!(parse_update(Exchange::Okx, msg), Some(b(1_792_962_000)));
        assert_eq!(parse_update(Exchange::Blofin, msg), Some(b(1_792_962_000)));
        assert_eq!(parse_update(Exchange::Okx, "pong"), None);
        assert_eq!(parse_update(Exchange::Okx, r#"{"event":"subscribe","arg":{"channel":"candle30m"}}"#), None);
    }

    /// Vraie connexion WebSocket vers un faux serveur local au format Hyperliquid : connexion, abonnement reçu par le
    /// serveur, bougies lues, puis reconnexion quand le serveur coupe.
    #[cfg(feature = "native")]
    #[test]
    fn connexion_reelle_a_un_faux_serveur() {
        use futures_util::{SinkExt, StreamExt};
        use std::sync::mpsc;
        use tokio_tungstenite::tungstenite::Message;

        let (addr_tx, addr_rx) = mpsc::channel();
        let (sub_tx, sub_rx) = mpsc::channel();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
            rt.block_on(async move {
                let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
                addr_tx.send(listener.local_addr().unwrap()).unwrap();
                for round in 0..2 {
                    let (tcp, _) = listener.accept().await.unwrap();
                    let mut ws = tokio_tungstenite::accept_async(tcp).await.unwrap();
                    if let Some(Ok(Message::Text(t))) = ws.next().await {
                        sub_tx.send(t.to_string()).unwrap();
                    }
                    for (k, c) in [(0, "80500.5"), (1, "80510.0")] {
                        let msg = format!(
                            r#"{{"channel":"candle","data":{{"t":{},"T":0,"s":"BTC","i":"30m","o":"80400","c":"{c}","h":"80600","l":"80300","v":"1.5","n":1}}}}"#,
                            (1_792_962_000i64 + round as i64 * 1800 + k as i64) * 1000
                        );
                        ws.send(Message::Text(msg)).await.unwrap();
                    }
                    ws.close(None).await.ok(); // coupe : le client doit se reconnecter
                }
            });
        });
        let addr = addr_rx.recv().unwrap();
        let sub = LiveSub {
            url: format!("ws://{addr}"),
            ..subscription(Exchange::Hyperliquid, "BTC", Timeframe::M30).unwrap()
        };
        let (ev_tx, ev_rx) = mpsc::channel();
        spawn(sub, |t| parse_event(Exchange::Hyperliquid, t), move |ev| ev_tx.send(ev).is_ok());
        let wait = std::time::Duration::from_secs(10);
        let mut events = vec![];
        while let Ok(ev) = ev_rx.recv_timeout(wait) {
            events.push(ev);
            if events.iter().filter(|e| matches!(e, LiveEvent::Bar(_))).count() == 4 {
                break;
            }
        }
        assert!(sub_rx.recv_timeout(wait).unwrap().contains(r#""coin":"BTC""#), "le serveur a reçu l'abonnement");
        assert_eq!(events.first(), Some(&LiveEvent::Connected));
        assert!(matches!(&events[1], LiveEvent::Bar(b) if b.close == 80500.5 && b.time == 1_792_962_000));
        assert!(events.iter().any(|e| matches!(e, LiveEvent::Disconnected(_))), "coupure vue");
        assert_eq!(events.iter().filter(|e| **e == LiveEvent::Connected).count(), 2, "reconnecté tout seul");
    }

    /// 🔑 Direct à jeton (KuCoin) contre un faux serveur local : le 1er jeton est refusé (« indisponible », nouvel
    /// essai après une attente), puis chaque connexion, et chaque reconnexion après une coupure, demande un jeton neuf.
    #[cfg(feature = "native")]
    #[test]
    fn direct_a_jeton_redemande_a_chaque_connexion() {
        use futures_util::StreamExt;
        use std::sync::atomic::{AtomicU32, Ordering};
        use std::sync::{mpsc, Arc};
        use tokio_tungstenite::tungstenite::Message;

        let (addr_tx, addr_rx) = mpsc::channel();
        let (sub_tx, sub_rx) = mpsc::channel();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
            rt.block_on(async move {
                let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
                addr_tx.send(listener.local_addr().unwrap()).unwrap();
                for _ in 0..2 {
                    let (tcp, _) = listener.accept().await.unwrap();
                    let mut ws = tokio_tungstenite::accept_async(tcp).await.unwrap();
                    if let Some(Ok(Message::Text(t))) = ws.next().await {
                        sub_tx.send(t.to_string()).unwrap();
                    }
                    ws.close(None).await.ok(); // coupe : un jeton neuf doit être demandé
                }
            });
        });
        let addr = addr_rx.recv().unwrap();
        let appels = Arc::new(AtomicU32::new(0));
        let a2 = appels.clone();
        let preparer = move || {
            let n = a2.fetch_add(1, Ordering::SeqCst) + 1;
            async move {
                if n == 1 {
                    return Err("jeton refusé".to_string());
                }
                Ok(LiveSub {
                    url: format!("ws://{addr}"),
                    subscribe: format!(r#"{{"jeton":{n}}}"#),
                    ..Default::default()
                })
            }
        };
        let (ev_tx, ev_rx) = mpsc::channel();
        spawn_jeton(preparer, |_| None, move |ev| ev_tx.send(ev).is_ok());
        let attente = std::time::Duration::from_secs(20);
        assert_eq!(ev_rx.recv_timeout(attente).unwrap(), LiveEvent::Indisponible("jeton refusé".into()));
        assert_eq!(sub_rx.recv_timeout(attente).unwrap(), r#"{"jeton":2}"#, "nouvel essai, jeton neuf");
        assert_eq!(
            sub_rx.recv_timeout(attente).unwrap(),
            r#"{"jeton":3}"#,
            "après la coupure : encore un jeton neuf"
        );
        let vus: Vec<LiveEvent> = ev_rx.try_iter().collect();
        assert!(
            vus.contains(&LiveEvent::Connected) && vus.iter().any(|e| matches!(e, LiveEvent::Disconnected(_))),
            "{vus:?}"
        );
    }

    #[test]
    fn trades_en_bougies_de_secondes() {
        let s = trades_subscription(Exchange::Hyperliquid, "BTC").unwrap();
        assert!(s.subscribe.contains(r#""type":"trades""#) && s.subscribe.contains(r#""coin":"BTC""#));
        assert!(trades_subscription(Exchange::Okx, "BTC").is_none());
        // vrai message d'Hyperliquid (raccourci)
        let msg = r#"{"channel":"trades","data":[{"coin":"BTC","side":"B","px":"80144.0","sz":"0.0022","time":1792973722031,"tid":1},{"coin":"BTC","side":"A","px":"80149.0","sz":"0.07404","time":1792973722931,"tid":2}]}"#;
        let t = parse_trades(Exchange::Hyperliquid, msg);
        assert_eq!(t.len(), 2);
        assert_eq!(
            t[0],
            Trade {
                time_ms: 1_792_973_722_031,
                price: 80144.0,
                size: 0.0022
            }
        );
        assert!(parse_trades(Exchange::Hyperliquid, r#"{"channel":"pong"}"#).is_empty());
        // bougies de 5 s : les deux trades dans la même, un 3e 5 s plus tard en commence une autre
        let mut bars = vec![];
        assert!(fold_trade(&mut bars, t[0], 5));
        assert!(!fold_trade(&mut bars, t[1], 5));
        assert_eq!(
            (bars[0].time, bars[0].open, bars[0].high, bars[0].close),
            (1_792_973_720, 80144.0, 80149.0, 80149.0)
        );
        assert!((bars[0].volume - 0.07624).abs() < 1e-9);
        assert!(fold_trade(
            &mut bars,
            Trade {
                time_ms: 1_792_973_726_000,
                price: 80100.0,
                size: 1.0
            },
            5
        ));
        assert!(
            !fold_trade(
                &mut bars,
                Trade {
                    time_ms: 1_792_973_700_000,
                    price: 1.0,
                    size: 1.0
                },
                5
            ),
            "trop vieux : ignoré"
        );
        assert_eq!(bars.len(), 2);
    }

    #[test]
    fn remplace_ou_ajoute_jamais_l_historique() {
        let mut bars = vec![bar(0, 10.0), bar(60, 11.0)];
        assert!(!apply(&mut bars, bar(60, 12.5)), "même heure : remplace");
        assert_eq!((bars.len(), bars[1].close), (2, 12.5));
        assert!(apply(&mut bars, bar(120, 13.0)), "nouvelle bougie");
        assert_eq!(bars.len(), 3);
        assert!(!apply(&mut bars, bar(0, 99.0)), "vieille bougie ignorée");
        assert_eq!(bars[0].close, 10.0);
    }

    /// Vrais messages du direct en 1 minute (BTC, 26 oct. 2026), un par exchange : chacun donne la bougie de 1 min.
    #[test]
    fn direct_1_minute_vrais_messages() {
        let msgs = [
            (
                Exchange::Hyperliquid,
                r#"{"channel":"candle","data":{"t":1793032920000,"T":1793032979999,"s":"BTC","i":"1m","o":"79910.0","c":"79910.0","h":"79927.0","l":"79910.0","v":"32.85883","n":122}}"#,
                79910.0,
            ),
            (
                Exchange::Binance,
                r#"{"e":"kline","E":1793032928190,"s":"BTCUSDT","k":{"t":1793032920000, "T":1793032979999, "s":"BTCUSDT", "i":"1m", "f":8206657821, "L":8206658465, "o":"79905.70", "c":"79892.10", "h":"79911.90", "l":"79892.10", "v":"33.260", "n":643, "x":false, "q":"2657659.41370", "V":"17.016", "Q":"1359694.66480", "B":"0"}}"#,
                79892.1,
            ),
            (
                Exchange::Bybit,
                r#"{"topic":"kline.1.BTCUSDT","data":[{"start":1793032920000,"end":1793032979999,"interval":"1","open":"79907.8","close":"79902.2","high":"79919.9","low":"79896","volume":"6.328","turnover":"505639.8965","confirm":false,"timestamp":1793032927670}],"ts":1793032927670,"type":"snapshot"}"#,
                79902.2,
            ),
            (
                Exchange::Okx,
                r#"{"arg":{"channel":"candle1m","instId":"BTC-USDT-SWAP"},"data":[["1793032920000","79914.6","79920.3","79906.6","79906.6","537.41","5.3741","429459.12875000","0"]]}"#,
                79906.6,
            ),
            (
                Exchange::Blofin,
                r#"{"arg":{"channel":"candle1m","instId":"BTC-USDT"},"data":[["1793032920000","79905.9","79905.9","79892.3","79892.3","893.5","0.8935","71393.83631","0"]]}"#,
                79892.3,
            ),
            (
                Exchange::Weex,
                r#"{"e":"kline","E":1793032928898,"s":"BTCUSDT","p":"LAST_PRICE","d":[{"t":1793032920000,"T":1793032980000,"s":"BTCUSDT","i":"1m","o":"80304.2","c":"80301.0","h":"80312.3","l":"80300.8","v":"7.9984","n":49,"q":"642337.08166","V":"3.0311","Q":"243412.60476"}]}"#,
                80301.0,
            ),
            // Bitunix : pas d'heure de bougie, celle du message (ts) ramenée à la minute
            (
                Exchange::Bitunix,
                r#"{"ch":"market_kline_1min","symbol":"BTCUSDT","ts":1793032946133,"data":{"o":"80200","c":"80254.9","h":"80272","l":"80190","b":"27.2241","q":"2184450.27421"}}"#,
                80254.9,
            ),
        ];
        for (ex, m, close) in msgs {
            let b = parse_update(ex, m).unwrap_or_else(|| panic!("{} : message 1m pas lu", ex.name()));
            assert_eq!(b.time, 1_793_032_920, "{}", ex.name());
            assert_eq!(b.close, close, "{}", ex.name());
            assert!(b.high >= b.low && b.volume > 0.0, "{}", ex.name());
        }
    }

    /// Bitstamp : pas de bougies en direct, les trades (vrai message) remplissent la bougie de 1 minute.
    #[test]
    fn bitstamp_par_les_trades() {
        let m = r#"{"data":{"id":661442941,"timestamp":"1793034045","amount":0.00006298,"amount_str":"0.00006298","price":80151.55,"price_str":"80151.55","type":0,"microtimestamp":"1793034045622000","buy_order_id":2065199394672640,"sell_order_id":2065199391125504},"channel":"live_trades_btcusd","event":"trade"}"#;
        assert!(parse_update(Exchange::Bitstamp, m).is_none());
        let t = parse_trades(Exchange::Bitstamp, m);
        assert_eq!(
            t,
            vec![Trade {
                time_ms: 1_793_034_045_622,
                price: 80151.55,
                size: 0.00006298
            }]
        );
        let mut bars = vec![];
        assert!(fold_trade(&mut bars, t[0], 60));
        assert_eq!((bars[0].time, bars[0].close), (1_793_034_000, 80151.55));
        assert!(subscription(Exchange::Bitstamp, "BTC", Timeframe::M1).is_some_and(|s| s.subscribe.contains("live_trades_btcusd")));
        // Bitunix en semaine : le lundi 00:00 UTC
        let w = r#"{"ch":"market_kline_1week","symbol":"BTCUSDT","ts":1793034046133,"data":{"o":"1","c":"2","h":"3","l":"0.5","b":"9"}}"#;
        let b = parse_update(Exchange::Bitunix, w).unwrap();
        assert_eq!(chrono_weekday(b.time), 0, "lundi");
    }

    /// 🏦 Coinbase (Suivi 1, Jee 21 Nov : sa chart de 1 minute restait figée, faute de direct) : pas de canal de
    /// bougies, ses trades (vrai message du canal public `matches`, relevé le 2026-11-15) remplissent la bougie ;
    /// `last_match` (d'avant l'abonnement) est déjà dans les bougies lues.
    #[test]
    fn coinbase_par_les_trades() {
        let m = r#"{"type":"match","trade_id":1130234654,"maker_order_id":"d266fecd-2c24-4fb1-8506-74d29f0d8cc2","taker_order_id":"02aeb562-b9a3-4e49-a4e2-41fa7fdcdd8a","side":"buy","size":"0.00062299","price":"65072.26","product_id":"BTC-USD","sequence":139512478702,"time":"2026-11-15T20:59:18.152420Z"}"#;
        assert!(parse_update(Exchange::Coinbase, m).is_none());
        let t = parse_trades(Exchange::Coinbase, m);
        assert_eq!(
            t,
            vec![Trade {
                time_ms: 1_794_776_358_152,
                price: 65072.26,
                size: 0.00062299
            }]
        );
        assert_eq!(parse_event(Exchange::Coinbase, m), Some(LiveEvent::Trades(t.clone())));
        let mut bars = vec![];
        assert!(fold_trade(&mut bars, t[0], 60));
        assert_eq!((bars[0].time, bars[0].close), (1_794_776_340, 65072.26));
        let dernier = m.replace(r#""type":"match""#, r#""type":"last_match""#);
        assert!(parse_trades(Exchange::Coinbase, &dernier).is_empty(), "d'avant l'abonnement");
        assert!(parse_trades(
            Exchange::Coinbase,
            r#"{"type":"subscriptions","channels":[{"name":"matches","product_ids":["BTC-USD"]}]}"#
        )
        .is_empty());
        for s in [
            subscription(Exchange::Coinbase, "BTC", Timeframe::M1),
            trades_subscription(Exchange::Coinbase, "BTC"),
        ] {
            let s = s.expect("le direct de la chart (minutes et secondes)");
            assert!(crate::net::autorise(&s.url), "ws-feed.exchange.coinbase.com déjà dans la liste blanche");
            assert!(s.subscribe.contains(r#""product_ids":["BTC-USD"]"#) && s.subscribe.contains(r#""channels":["matches"]"#));
        }
    }

    /// 🔑 KuCoin (Suivi, 22 Nov : sa chart restait figée, faute de direct) : ses trades (vrai message du canal
    /// `/market/match`, relevé le 2026-11-15) remplissent la bougie, sur un flux dont l'adresse demande un jeton.
    #[test]
    fn kucoin_par_ses_trades() {
        let m = r#"{"topic":"/market/match:BTC-USDT","type":"message","subject":"trade.l3match","data":{"makerOrderId":"511549248764932098","price":"65127.6","sequence":"25299163879325696","side":"sell","size":"0.00030434","symbol":"BTC-USDT","takerOrderId":"511549356621537281","time":"1794776072748000000","tradeId":"25299163879325696","type":"match"}}"#;
        let t = parse_trades(Exchange::Kucoin, m);
        assert_eq!(
            t,
            vec![Trade {
                time_ms: 1_794_776_072_748,
                price: 65127.6,
                size: 0.00030434
            }]
        );
        assert_eq!(parse_event(Exchange::Kucoin, m), Some(LiveEvent::Trades(t.clone())));
        let mut bars = vec![];
        assert!(fold_trade(&mut bars, t[0], 60));
        assert_eq!((bars[0].time, bars[0].close), (1_794_776_040, 65127.6));
        for msg in [
            r#"{"id":"x","type":"welcome"}"#,
            r#"{"id":"1","type":"ack"}"#,
            r#"{"id":"argos","type":"pong"}"#,
        ] {
            assert_eq!(parse_event(Exchange::Kucoin, msg), None, "{msg}");
        }
    }

    /// Chaque exchange a le direct de sa chart (bougies, ou trades pour les bougies en secondes) ; 🔑 KuCoin par son
    /// flux à jeton (Suivi, 22 Nov : sa chart, comme celle de Coinbase avant, n'avait pas de direct et restait figée).
    #[test]
    fn chaque_chart_a_son_direct() {
        for e in Exchange::ALL {
            assert!(direct_chart(e, "BTC", Some(Timeframe::M1)).is_some(), "{e:?}");
        }
        assert_eq!(
            direct_chart(Exchange::Kucoin, "BTC", Some(Timeframe::M1)),
            Some(DirectChart::Jeton(vec!["BTC".into()]))
        );
        assert_eq!(direct_chart(Exchange::Kucoin, "BTC", None), Some(DirectChart::Jeton(vec!["BTC".into()])));
        for e in [Exchange::Hyperliquid, Exchange::Bitstamp, Exchange::Coinbase] {
            assert!(matches!(direct_chart(e, "BTC", None), Some(DirectChart::Pret(_))), "{e:?} en secondes");
        }
        let jeton = crate::whales::kucoin_sub_at("wss://ws-api-spot.kucoin.com/?token=x&connectId=argos", &["BTC".into()]);
        assert!(jeton.subscribe.contains("/market/match:BTC-USDT"), "le canal de ses trades");
        // page web : KuCoin y refuse son jeton (CORS) ; pas de demande sans fin, les autres gardent leur direct
        assert_eq!(direct_chart_si(Exchange::Kucoin, "BTC", Some(Timeframe::M1), false), None);
        assert!(matches!(
            direct_chart_si(Exchange::Coinbase, "BTC", Some(Timeframe::M1), false),
            Some(DirectChart::Pret(_))
        ));
    }

    /// Jour de la semaine (0 = lundi) d'une heure Unix.
    fn chrono_weekday(t: i64) -> i64 {
        (t.div_euclid(86_400) + 3).rem_euclid(7)
    }

    #[test]
    fn direct_et_messages_fous_ignores() {
        let mut bars = vec![bar(3600, 2.0)];
        let mut folle = bar(7200, 3.0);
        folle.high = 1.0;
        assert!(!apply(&mut bars, folle), "bougie incohérente du direct : ignorée");
        assert_eq!(bars.len(), 1);
        assert!(apply(&mut bars, bar(7200, 3.0)));
        // gzip qui se gonfle au-delà de MAX_MSG : refusé
        use std::io::Write;
        let mut z = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
        z.write_all(&vec![b' '; MAX_MSG + 10]).unwrap();
        let petit = z.finish().unwrap();
        assert!(petit.len() < 100_000 && gunzip(&petit).is_none());
    }

    /// 🔌 Coupure réelle (petit serveur WebSocket local) : le serveur ferme la connexion ; le direct le voit, attend
    /// (2 s + hasard) et se reconnecte tout seul, puis reçoit de nouveau les messages.
    #[cfg(feature = "native")]
    #[test]
    fn coupure_puis_reconnexion_toute_seule() {
        use futures_util::SinkExt;
        use std::sync::{mpsc, Arc, Mutex};
        let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build().unwrap();
        let l = rt.block_on(tokio::net::TcpListener::bind("127.0.0.1:0")).unwrap();
        let port = l.local_addr().unwrap().port();
        rt.spawn(async move {
            for (k, msg) in ["avant", "apres"].into_iter().enumerate() {
                let (tcp, _) = l.accept().await.unwrap();
                let mut ws = tokio_tungstenite::accept_async(tcp).await.unwrap();
                ws.send(tokio_tungstenite::tungstenite::Message::Text(msg.into())).await.unwrap();
                if k == 0 {
                    ws.close(None).await.ok(); // 🔌 la coupure
                } else {
                    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                }
            }
        });
        let journal = Arc::new(Mutex::new(Vec::<String>::new()));
        let (tx, rx) = mpsc::channel();
        let j = journal.clone();
        let sub = LiveSub {
            url: format!("ws://127.0.0.1:{port}"),
            ..Default::default()
        };
        let t0 = std::time::Instant::now();
        spawn(
            sub,
            |t| Some(LiveEvent::Disconnected(format!("msg:{t}"))),
            move |ev| {
                let e = match ev {
                    LiveEvent::Connected => "connecté".to_string(),
                    LiveEvent::Disconnected(r) if r.starts_with("msg:") => r,
                    LiveEvent::Disconnected(_) => "coupé".to_string(),
                    _ => "?".into(),
                };
                let fini = e == "msg:apres";
                j.lock().unwrap().push(e);
                if fini {
                    let _ = tx.send(t0.elapsed());
                }
                !fini
            },
        );
        let duree = rx.recv_timeout(std::time::Duration::from_secs(15)).expect("reconnecté dans les 15 s");
        let j = journal.lock().unwrap().clone();
        assert_eq!(j, ["connecté", "msg:avant", "coupé", "connecté", "msg:apres"], "{j:?}");
        assert!(
            duree >= std::time::Duration::from_secs(2),
            "attend au moins 2 s avant de se reconnecter : {duree:?}"
        );
        drop(rt);
    }

    /// 💤 A page's connection, played. Its timer fires at each of `ticks` (ms since the opening); the exchange answers
    /// each ping `late` ms after it left and sends `data` besides, as long as the link lives (it dies at `dies`). The
    /// tick at which the page says the link is dead, with the moment of the last message it had (`None`: never said
    /// dead), read as `web::connect` reads it: the messages arrived are noted, then `dead` is read, then the ping leaves.
    fn said_dead(ticks: &[u64], late: u64, data: &[u64], dies: Option<u64>) -> Option<(u64, u64)> {
        let ms = std::time::Duration::from_millis;
        let lives = |t: u64| dies.is_none_or(|end| t < end);
        let mut coming: std::collections::BTreeSet<u64> = data.iter().copied().filter(|t| lives(*t)).collect();
        let (mut silence, mut last) = (Silence::heard(ms(0)), 0);
        for &tick in ticks {
            while let Some(t) = coming.first().copied().filter(|t| *t <= tick) {
                coming.remove(&t);
                (silence, last) = (Silence::heard(ms(t)), t);
            }
            if silence.dead(ms(tick)) {
                return Some((tick, last));
            }
            silence.asked(ms(tick));
            if lives(tick + late) {
                coming.insert(tick + late);
            }
        }
        None
    }

    /// 💤 The watch on silence of a page ([`Silence`]): a link that answers nothing any more is said dead when the
    /// desktop would cut it, 60 to 80 s after its last message. A market where nothing trades, an exchange slow to
    /// answer, a tab whose timers are held back and a computer back from sleep with its link intact are not.
    #[test]
    fn silence_d_une_connexion_dans_une_page() {
        use std::time::Duration;
        let (s, minute) = (1_000, 60_000);
        let every = |step: u64, from: u64, until: u64| (from / step + 1..=until / step).map(|k| k * step).collect::<Vec<u64>>();
        let pings = every(20 * s, 0, 120 * minute);
        // a market where nothing trades: the answers to the pings are its only messages
        assert_eq!(said_dead(&pings, 150, &[], None), None);
        // an exchange slow to answer (25 s, each time)
        assert_eq!(said_dead(&pings, 25 * s, &[], None), None);
        // the link dies, at any moment, on a quiet market and on a busy one
        let busy = every(700, 0, 30 * minute);
        for dies in (0..20 * minute).step_by(1_777) {
            for data in [&[][..], &busy[..]] {
                let (at, last) = said_dead(&pings, 150, data, Some(dies)).expect("a dead link is said dead");
                assert!(
                    last <= dies && at - last > 60 * s && at - last <= 80 * s,
                    "link dead at {dies} ms: its last message came at {last}, it was said dead at {at}"
                );
            }
        }
        // a tab in the background: its timer fires once a minute, some seconds late each time. More than the limit
        // goes by between two answers (the time since the last message, alone, would cut a link in good health here)
        let held: Vec<u64> = (1..=120).map(|k| k * minute + (k * 7_919) % 5_000).collect();
        assert!(held.windows(2).any(|pair| pair[1] - (pair[0] + 150) > SILENCE.as_millis() as u64));
        assert_eq!(said_dead(&held, 150, &[], None), None);
        // and a link that dies there is said dead at the second ping left without an answer
        let dies = 10 * minute;
        let second = held.iter().copied().filter(|tick| *tick >= dies).nth(1);
        assert_eq!(said_dead(&held, 150, &[], Some(dies)).map(|(at, _)| at), second);
        // a computer asleep for three hours (no timer, no message), then back
        let back = 190 * minute;
        let asleep: Vec<u64> = every(20 * s, 0, 10 * minute)
            .into_iter()
            .chain(every(20 * s, back, back + 10 * minute))
            .collect();
        // its link held: the first ping back is answered
        assert_eq!(said_dead(&asleep, 150, &[], None), None);
        // its link died meanwhile: the first ping back gets its time to be answered, then the link is said dead
        assert_eq!(said_dead(&asleep, 150, &[], Some(11 * minute)).map(|(at, _)| at), Some(back + 60 * s));
        // a connection the app does not ping is never judged: in a page its silence proves nothing
        assert!(!Silence::heard(Duration::ZERO).dead(Duration::from_secs(36_000)));
        // a ping has its time to be answered, whatever the silence before it; the first ping left without an answer
        // is the one that counts
        let mut silence = Silence::heard(Duration::ZERO);
        silence.asked(Duration::from_secs(600));
        assert!(!silence.dead(Duration::from_secs(600) + ANSWER) && silence.dead(Duration::from_secs(601) + ANSWER));
        silence.asked(Duration::from_secs(620));
        assert!(silence.dead(Duration::from_secs(601) + ANSWER));
        // a clock that goes back says nothing dead
        assert!(!silence.dead(Duration::from_secs(10)));
        // (the pings leave every 20 s: an answer is given more time than that, and less than the limit of silence)
        assert!(Duration::from_secs(20) < ANSWER && ANSWER < SILENCE);
    }

    /// 💤 A page judges the silence of a connection only where the exchange answers the app's pings with a message
    /// (`LiveSub::judged`): seen in the answers recorded on 23 Nov. 2026, where every pinged exchange answered but
    /// Bitunix. The live candles here; the trades in the tests of the trades file, which this file does not read.
    #[test]
    fn une_page_ne_juge_que_les_liens_qui_repondent_au_ping() {
        // pinged by the app, and the exchange answers: judged
        for ex in [
            Exchange::Hyperliquid,
            Exchange::Okx,
            Exchange::Bybit,
            Exchange::Blofin,
            Exchange::Weex,
            Exchange::Kraken,
            Exchange::Bitfinex,
        ] {
            assert!(subscription(ex, "BTC", Timeframe::M1).expect("live candles").judged(), "{ex:?}");
        }
        // the exchange pings by itself, or asks the app to answer its own pings: not judged
        for ex in [Exchange::Binance, Exchange::Aster, Exchange::Coinbase] {
            assert!(!subscription(ex, "BTC", Timeframe::M1).expect("live candles").judged(), "{ex:?}");
        }
        // pinged by the app, but nothing comes back (Bitunix): still pinged, to keep the link open, and not judged
        let sub = subscription(Exchange::Bitunix, "BTC", Timeframe::M1).expect("live candles");
        assert!(!sub.ping.is_empty() && sub.ping_unanswered && !sub.judged(), "{sub:?}");
        // the shell of the app gets the description in JSON: the same fields as before, without the new one…
        let json = serde_json::to_string(&sub).expect("write");
        let value: serde_json::Value = serde_json::from_str(&json).expect("json");
        let mut fields: Vec<String> = value.as_object().expect("an object").keys().cloned().collect();
        fields.sort();
        assert_eq!(fields, ["gzip", "ping", "reply", "subscribe", "url"], "{json}");
        // …and it reads it as before
        assert_eq!(serde_json::from_str::<LiveSub>(&json).expect("read"), LiveSub { ping_unanswered: false, ..sub });
    }
}
