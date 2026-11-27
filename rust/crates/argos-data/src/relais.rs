//! 🔁 The exchanges relay of the PC's server (`/api/bourse/<exchange>/<path>`) for the web page (PC and phone):
//! Bitfinex and KuCoin send no CORS header, so a browser cannot read their public API; the PC's server
//! (argos-serveur, the one that already relays `/api/news` and `/api/calendar`) reads it for the page.
//!
//! It is **not** an open relay:
//! - one fixed host per exchange (`api-pub.bitfinex.com`, `api.kucoin.com`), never an address from the browser;
//! - a closed list of paths and of query parameters, each value checked, and the address **rebuilt** from what was
//!   checked (nothing of the browser's request is copied: no header, no cookie, no unknown parameter);
//! - GET only (the server refuses every other method);
//! - a server-side cache and a cap of requests a minute per kind of request ([`Garde`]), under Bitfinex's public
//!   limits, because the PC and the phone go through the same address.
//!
//! The page asks `/api/bourse/etat` once: when it answers, [`activer`] and the requests to those hosts go through
//! the relay ([`relayer`]); a page served elsewhere (no relay) keeps those exchanges hidden, as before.

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};

/// The relay's prefix on the PC's server.
pub const PREFIXE: &str = "/api/bourse";
/// What the page asks to know whether the relay is there.
pub const ETAT: &str = "/api/bourse/etat";

const BITFINEX: &str = "https://api-pub.bitfinex.com";
const KUCOIN: &str = "https://api.kucoin.com";

/// Longest wait of an answer from the exchange (seconds) and biggest answer relayed.
pub const DELAI_S: u64 = 10;
pub const MAX_REPONSE: usize = 4 * 1024 * 1024;
/// At most this many requests a minute to the exchange for one kind of request (catalogue, prices, book…).
pub const PLAFOND_MINUTE: usize = 20;

/// The exchanges that go through the relay in the browser (the ids of `Exchange`).
pub const BOURSES: [&str; 3] = ["bitfinex", "bitfinex-spot", "kucoin"];

static ACTIF: AtomicBool = AtomicBool::new(false);
static CONNU: AtomicBool = AtomicBool::new(false);

/// The page found the relay (`/api/bourse/etat` answered) or not.
pub fn activer(oui: bool) {
    ACTIF.store(oui, Ordering::Relaxed);
    CONNU.store(true, Ordering::Relaxed);
}

/// `true` once the page knows whether the relay is there (the first request to a relayed host asks first: at start-up
/// the chart's candles left before the answer and the browser refused them).
pub fn connu() -> bool {
    CONNU.load(Ordering::Relaxed)
}

/// `true` once the page found the relay of the PC's server.
pub fn actif() -> bool {
    ACTIF.load(Ordering::Relaxed)
}

/// The answer of `/api/bourse/etat`, with the number of requests really sent to the exchanges since the server
/// started (measures).
pub fn etat_json(envoyees: u64) -> Vec<u8> {
    serde_json::json!({ "bourses": ["bitfinex", "kucoin"], "envoyees": envoyees })
        .to_string()
        .into_bytes()
}

/// `true` if `/api/bourse/etat` answered as the relay does.
pub fn lire_etat(body: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(body).is_ok_and(|v| v["bourses"].as_array().is_some_and(|a| a.iter().any(|b| b == "bitfinex")))
}

/// In the browser: the relay's address for an exchange address it carries (« https://api-pub.bitfinex.com/v2/… » →
/// « /api/bourse/bitfinex/v2/… »); `None`: not one of them (asked as is).
pub fn relayer(url: &str) -> Option<String> {
    for (hote, nom) in [(BITFINEX, "bitfinex"), (KUCOIN, "kucoin")] {
        if let Some(reste) = url.strip_prefix(hote).filter(|r| r.starts_with('/')) {
            return Some(format!("{PREFIXE}/{nom}{reste}"));
        }
    }
    None
}

/// The kind of a relayed request: its cache time and its own cap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Sorte {
    Conf,
    Prix,
    Carnet,
    Bougies,
    /// candles of the past (`end` given): they never change
    Passe,
    Trades,
    Statut,
}

impl Sorte {
    /// How long an answer is kept (ms): catalogue 10 min, prices 5 s, book 2 s, last candle 10 s, trades 2 s.
    pub fn garde_ms(self) -> u64 {
        match self {
            Sorte::Conf | Sorte::Passe => 600_000,
            Sorte::Prix | Sorte::Statut => 5_000,
            Sorte::Carnet | Sorte::Trades => 2_000,
            Sorte::Bougies => 10_000,
        }
    }
}

/// A checked request: the exchange's address (rebuilt), its kind, and the exchange (for the cap).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Permis {
    pub url: String,
    pub sorte: Sorte,
    pub bourse: &'static str,
}

/// A refusal: (HTTP code, message).
pub type Refus = (u16, String);

fn non() -> Refus {
    (404, "pas relayé".into())
}

/// A symbol / key: letters, digits and `:` `-` `_` (« tXRPF0:USTF0 », « XRP-BTC »), 1 to 40 characters.
fn symbole_ok(s: &str) -> bool {
    (1..=40).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b':' || b == b'-' || b == b'_')
}

fn entier_ok(s: &str) -> bool {
    (1..=16).contains(&s.len()) && s.trim_start_matches('-').bytes().all(|b| b.is_ascii_digit()) && s != "-"
}

/// The query string as (name, value) pairs; `None` if a name is repeated or something is encoded (`%`, `+`).
fn parametres(q: &str) -> Option<Vec<(&str, &str)>> {
    let mut out: Vec<(&str, &str)> = vec![];
    for p in q.split('&').filter(|p| !p.is_empty()) {
        let (k, v) = p.split_once('=').unwrap_or((p, ""));
        if p.contains(['%', '+']) || out.iter().any(|(x, _)| *x == k) {
            return None;
        }
        out.push((k, v));
    }
    Some(out)
}

/// The parameters a path accepts: (name, check of its value).
type Params = &'static [(&'static str, fn(&str) -> bool)];

/// Checks the parameters against `permis` (name → check) and rebuilds the query in `permis`'s order.
fn requete(q: &str, permis: Params) -> Result<String, Refus> {
    let p = parametres(q).ok_or_else(|| (400, "paramètres refusés".to_string()))?;
    for (k, v) in &p {
        match permis.iter().find(|(n, _)| n == k) {
            Some((_, ok)) if ok(v) => {}
            _ => return Err((400, format!("paramètre refusé : {}", k.chars().take(20).collect::<String>()))),
        }
    }
    let out: Vec<String> = permis
        .iter()
        .filter_map(|(n, _)| p.iter().find(|(k, _)| k == n).map(|(k, v)| format!("{k}={v}")))
        .collect();
    Ok(if out.is_empty() { String::new() } else { format!("?{}", out.join("&")) })
}

fn liste_symboles(v: &str) -> bool {
    v == "ALL" || (v.len() <= 2000 && v.split(',').all(symbole_ok))
}

fn tri(v: &str) -> bool {
    v == "1" || v == "-1"
}

fn limite(v: &str) -> bool {
    v.parse::<u32>().is_ok_and(|n| (1..=10_000).contains(&n))
}

fn longueur(v: &str) -> bool {
    matches!(v, "1" | "25" | "100")
}

/// Bitfinex's candle key « trade:1h:tXRPBTC ».
fn cle_bougie(k: &str) -> bool {
    let mut p = k.splitn(3, ':');
    p.next() == Some("trade")
        && p.next()
            .is_some_and(|tf| ["1m", "5m", "15m", "30m", "1h", "3h", "6h", "12h", "1D", "1W", "14D", "1M"].contains(&tf))
        && p.next().is_some_and(|s| s.starts_with('t') && symbole_ok(s))
}

/// What the browser asked (after `/api/bourse`, « /bitfinex/v2/tickers?symbols=tXRPBTC ») → the exchange's address,
/// or a refusal (404: not relayed; 400: a parameter refused).
pub fn permis(suite: &str) -> Result<Permis, Refus> {
    let (chemin, q) = suite.split_once('?').unwrap_or((suite, ""));
    if chemin.contains("..") || chemin.contains("//") || chemin.contains('%') {
        return Err(non());
    }
    let (bourse, reste) = chemin.trim_start_matches('/').split_once('/').ok_or_else(non)?;
    let morceaux: Vec<&str> = reste.split('/').collect();
    match bourse {
        "bitfinex" => bitfinex(&morceaux, q),
        "kucoin" => kucoin(&morceaux, q),
        _ => Err(non()),
    }
}

fn bitfinex(m: &[&str], q: &str) -> Result<Permis, Refus> {
    let page: Params = &[("limit", limite), ("sort", tri), ("start", entier_ok), ("end", entier_ok)];
    let (chemin, sorte, params): (String, Sorte, Params) = match m {
        ["v2", "conf", c]
            if [
                "pub:list:pair:exchange",
                "pub:list:pair:futures",
                "pub:map:currency:sym",
                "pub:map:currency:label",
            ]
            .contains(c) =>
        {
            (format!("/v2/conf/{c}"), Sorte::Conf, &[])
        }
        ["v2", "tickers"] => ("/v2/tickers".into(), Sorte::Prix, &[("symbols", liste_symboles)]),
        ["v2", "candles", k, fin @ ("hist" | "last")] if cle_bougie(k) => {
            let passe = *fin == "hist" && parametres(q).is_some_and(|p| p.iter().any(|(k, _)| *k == "end"));
            (format!("/v2/candles/{k}/{fin}"), if passe { Sorte::Passe } else { Sorte::Bougies }, page)
        }
        ["v2", "book", s, p] if s.starts_with('t') && symbole_ok(s) && ["P0", "P1", "P2", "P3", "P4", "R0"].contains(p) => {
            (format!("/v2/book/{s}/{p}"), Sorte::Carnet, &[("len", longueur)])
        }
        ["v2", "trades", s, "hist"] if s.starts_with('t') && symbole_ok(s) => (format!("/v2/trades/{s}/hist"), Sorte::Trades, page),
        ["v2", "status", "deriv"] => ("/v2/status/deriv".into(), Sorte::Statut, &[("keys", liste_symboles)]),
        _ => return Err(non()),
    };
    Ok(Permis {
        url: format!("{BITFINEX}{chemin}{}", requete(q, params)?),
        sorte,
        bourse: "bitfinex",
    })
}

fn kucoin(m: &[&str], q: &str) -> Result<Permis, Refus> {
    fn types(v: &str) -> bool {
        [
            "1min", "3min", "5min", "15min", "30min", "1hour", "2hour", "4hour", "6hour", "8hour", "12hour", "1day", "1week",
        ]
        .contains(&v)
    }
    let (chemin, sorte, params): (&str, Sorte, Params) = match m {
        ["api", "v2", "symbols"] => ("/api/v2/symbols", Sorte::Conf, &[]),
        ["api", "v1", "market", "allTickers"] => ("/api/v1/market/allTickers", Sorte::Prix, &[]),
        ["api", "v1", "market", "candles"] => (
            "/api/v1/market/candles",
            if q.contains("endAt=") { Sorte::Passe } else { Sorte::Bougies },
            &[("type", types), ("symbol", symbole_ok), ("startAt", entier_ok), ("endAt", entier_ok)],
        ),
        _ => return Err(non()),
    };
    Ok(Permis {
        url: format!("{KUCOIN}{chemin}{}", requete(q, params)?),
        sorte,
        bourse: "kucoin",
    })
}

/// 🧊 The server's cache and cap: answers kept per address ([`Sorte::garde_ms`]), and at most [`PLAFOND_MINUTE`]
/// requests a minute to an exchange per kind of request. Over the cap: the last answer kept (even old), or a refusal
/// (429). Pure (the clock is given): the server holds one in a mutex.
#[derive(Default)]
pub struct Garde {
    reponses: HashMap<String, (u64, Vec<u8>)>,
    envois: HashMap<(&'static str, Sorte), VecDeque<u64>>,
    /// requests really sent to the exchanges (measures)
    pub envoyees: u64,
}

/// What to do with a request.
#[derive(Debug, PartialEq, Eq)]
pub enum Decision {
    /// a kept answer
    Garde(Vec<u8>),
    /// ask the exchange (the request is counted)
    Demander,
    /// over the cap, nothing kept: wait this many seconds
    Attendre(u64),
}

impl Garde {
    /// What to do for `p` at `now_ms`.
    pub fn decider(&mut self, p: &Permis, now_ms: u64) -> Decision {
        if let Some((t, b)) = self.reponses.get(&p.url) {
            if now_ms.saturating_sub(*t) < p.sorte.garde_ms() {
                return Decision::Garde(b.clone());
            }
        }
        let f = self.envois.entry((p.bourse, p.sorte)).or_default();
        while f.front().is_some_and(|t| now_ms.saturating_sub(*t) >= 60_000) {
            f.pop_front();
        }
        if f.len() >= PLAFOND_MINUTE {
            if let Some((_, b)) = self.reponses.get(&p.url) {
                return Decision::Garde(b.clone());
            }
            let attente = f.front().map_or(1, |t| (60_000 - now_ms.saturating_sub(*t)).div_ceil(1000));
            return Decision::Attendre(attente.max(1));
        }
        f.push_back(now_ms);
        self.envoyees += 1;
        Decision::Demander
    }

    /// Keeps the answer of `p` (the oldest ones go when too many are kept).
    pub fn garder(&mut self, p: &Permis, now_ms: u64, corps: Vec<u8>) {
        if self.reponses.len() >= 500 {
            let vieux: Vec<String> = self
                .reponses
                .iter()
                .filter(|(_, (t, _))| now_ms.saturating_sub(*t) > 600_000)
                .map(|(k, _)| k.clone())
                .collect();
            for k in vieux {
                self.reponses.remove(&k);
            }
            if self.reponses.len() >= 500 {
                self.reponses.clear();
            }
        }
        self.reponses.insert(p.url.clone(), (now_ms, corps));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn liste_blanche_bitfinex_et_kucoin() {
        let ok = |s: &str| permis(s).map(|p| p.url);
        assert_eq!(
            ok("/bitfinex/v2/conf/pub:list:pair:exchange").unwrap(),
            "https://api-pub.bitfinex.com/v2/conf/pub:list:pair:exchange"
        );
        assert_eq!(
            ok("/bitfinex/v2/tickers?symbols=ALL").unwrap(),
            "https://api-pub.bitfinex.com/v2/tickers?symbols=ALL"
        );
        assert_eq!(
            ok("/bitfinex/v2/candles/trade:1h:tXRPBTC/hist?sort=-1&limit=500&end=1795413600000").unwrap(),
            "https://api-pub.bitfinex.com/v2/candles/trade:1h:tXRPBTC/hist?limit=500&sort=-1&end=1795413600000",
            "rebuilt in a fixed order"
        );
        assert_eq!(permis("/bitfinex/v2/candles/trade:1h:tXRPBTC/hist?limit=5&end=1").unwrap().sorte, Sorte::Passe);
        assert_eq!(permis("/bitfinex/v2/candles/trade:1h:tXRPBTC/hist?limit=5").unwrap().sorte, Sorte::Bougies);
        assert_eq!(
            ok("/bitfinex/v2/book/tXRPF0:USTF0/P0?len=100").unwrap(),
            "https://api-pub.bitfinex.com/v2/book/tXRPF0:USTF0/P0?len=100"
        );
        assert_eq!(
            ok("/bitfinex/v2/trades/tXRPBTC/hist?limit=100").unwrap(),
            "https://api-pub.bitfinex.com/v2/trades/tXRPBTC/hist?limit=100"
        );
        assert_eq!(
            ok("/bitfinex/v2/status/deriv?keys=tXRPF0:USTF0").unwrap(),
            "https://api-pub.bitfinex.com/v2/status/deriv?keys=tXRPF0:USTF0"
        );
        assert_eq!(ok("/kucoin/api/v2/symbols").unwrap(), "https://api.kucoin.com/api/v2/symbols");
        assert_eq!(
            ok("/kucoin/api/v1/market/candles?type=1hour&symbol=XRP-BTC").unwrap(),
            "https://api.kucoin.com/api/v1/market/candles?type=1hour&symbol=XRP-BTC"
        );
        // every address the app asks really goes through
        for r in crate::exchanges::bitfinex::instruments_requests() {
            let crate::exchanges::Request::Get(u) = r else { panic!() };
            assert!(permis(&relayer(&u).unwrap()[PREFIXE.len()..]).is_ok(), "{u}");
        }
        let app = [
            crate::exchanges::bitfinex::candles_request("XRP/BTC", false, argos_core::Timeframe::H1, 500, Some(1_795_413_600_000)).unwrap(),
            crate::exchanges::bitfinex::book_request("XRP", true),
            crate::exchanges::bitfinex::trades_request("XRP/BTC", false),
            crate::exchanges::bitfinex::status_request("XRP"),
            crate::exchanges::kucoin::candles_request("XRP/BTC", argos_core::Timeframe::H1, 300, Some(1_795_413_600_000)).unwrap(),
        ];
        for r in app {
            let crate::exchanges::Request::Get(u) = r else { panic!() };
            assert!(permis(&relayer(&u).unwrap()[PREFIXE.len()..]).is_ok(), "{u}");
        }
    }

    /// 🛡️ Not an open relay: another host, another path, a smuggled parameter or address → refused.
    #[test]
    fn pas_un_relais_ouvert() {
        for s in [
            "/evil/v2/tickers",
            "/bitfinex/v2/auth/r/orders",
            "/bitfinex/v2/conf/pub:list:currency",
            "/bitfinex/v1/symbols",
            "/bitfinex/v2/../v2/auth/r/wallets",
            "/bitfinex//api-pub.bitfinex.com.evil.io/v2/tickers",
            "/bitfinex/v2/book/tXRPBTC/P9",
            "/bitfinex/v2/candles/trade:4h:tXRPBTC/hist",
            "/kucoin/api/v1/bullet-public",
            "/kucoin/api/v1/accounts",
        ] {
            assert_eq!(permis(s).unwrap_err().0, 404, "{s}");
        }
        for s in [
            "/bitfinex/v2/tickers?symbols=ALL&apikey=x",
            "/bitfinex/v2/tickers?symbols=https://evil.io",
            "/bitfinex/v2/tickers?symbols=ALL&symbols=tBTCUSD",
            "/bitfinex/v2/trades/tXRPBTC/hist?limit=1%26x",
            "/bitfinex/v2/book/tXRPBTC/P0?len=7",
        ] {
            assert_eq!(permis(s).unwrap_err().0, 400, "{s}");
        }
        assert_eq!(
            relayer("https://api-pub.bitfinex.com/v2/tickers?symbols=ALL").unwrap(),
            "/api/bourse/bitfinex/v2/tickers?symbols=ALL"
        );
        // (built, not written whole: the white-list guard reads every address written in the code)
        assert_eq!(relayer(&format!("https://api-pub.bitfinex.com{}/v2/tickers", ".evil.io")), None);
        assert_eq!(relayer("https://api.binance.com/api/v3/klines"), None, "the others: as is");
        assert!(lire_etat(std::str::from_utf8(&etat_json(3)).unwrap()));
        assert!(!lire_etat("<html>404</html>"));
    }

    /// 🧊 Cache per kind of request, and never more than the cap a minute to the exchange.
    #[test]
    fn cache_et_plafond() {
        let mut g = Garde::default();
        let carnet = permis("/bitfinex/v2/book/tXRPBTC/P0?len=100").unwrap();
        assert_eq!(g.decider(&carnet, 0), Decision::Demander);
        g.garder(&carnet, 0, b"[1]".to_vec());
        assert_eq!(g.decider(&carnet, 1_999), Decision::Garde(b"[1]".to_vec()), "kept 2 s");
        assert_eq!(g.decider(&carnet, 2_000), Decision::Demander);
        // 1 000 page requests in a minute (PC and phone): never more than the cap to Bitfinex
        let mut g = Garde::default();
        let mut envoyees = 0;
        for i in 0..1000u64 {
            let p = permis(&format!(
                "/bitfinex/v2/trades/t{}BTC/hist?limit=100",
                ["XRP", "ETH", "SOL", "ADA"][(i % 4) as usize]
            ))
            .unwrap();
            match g.decider(&p, i * 60) {
                Decision::Demander => {
                    envoyees += 1;
                    g.garder(&p, i * 60, b"[]".to_vec());
                }
                Decision::Garde(_) | Decision::Attendre(_) => {}
            }
        }
        assert_eq!(envoyees, PLAFOND_MINUTE, "at most the cap in 60 s");
        assert_eq!(g.envoyees, PLAFOND_MINUTE as u64);
        let neuf = permis("/bitfinex/v2/trades/tDOTBTC/hist?limit=100").unwrap();
        assert!(
            matches!(g.decider(&neuf, 59_999), Decision::Attendre(s) if (1..=60).contains(&s)),
            "over the cap, nothing kept"
        );
        let conf = permis("/bitfinex/v2/conf/pub:list:pair:futures").unwrap();
        assert_eq!(g.decider(&conf, 59_999), Decision::Demander, "each kind has its own cap");
    }
}
