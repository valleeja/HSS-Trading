//! # Argos Data — Layer 2A · API chart
//! Bougies et marchés de 8 exchanges, en lecture seule (aucune clé, aucun ordre).
//! Utilise seulement la couche 1 (`argos-core`). Le client HTTP est injecté (natif, navigateur ou faux client de test).

#![forbid(unsafe_code)]

pub mod actu;
pub mod book;
pub mod cache;
pub mod claude;
pub mod cmcap;
#[cfg(feature = "native")]
pub mod collecte;
pub mod contracts;
#[cfg(all(feature = "web", not(feature = "native")))]
pub mod coque;
pub mod couts;
pub mod dex;
pub mod eco;
pub mod exchanges;
pub mod explorer;
pub mod fiches;
#[cfg(any(feature = "native", feature = "web"))]
pub mod file;
pub mod http;
pub mod langpack;
pub mod liq;
pub mod liqdata;
pub mod live;
pub mod logos;
pub mod market;
pub mod murs;
pub mod net;
pub mod oanda;
pub mod orderflow;
pub mod paire;
pub mod parse;
pub mod perp;
pub mod prices;
pub mod reel;
pub mod relais;
#[cfg(any(feature = "native", feature = "web"))]
pub mod rt;
pub mod supervisor;
pub mod synth;
pub mod tickers;
pub mod tous;
pub mod trous;
pub mod tv;
pub mod wallet;
pub mod watchlist;
pub mod whales;
pub mod whaletop;
pub mod world;

use argos_core::feed::{clean_bars_detail, Feed, Market};
use argos_core::ArgosError;
use argos_core::{Bar, Instrument, Result, Timeframe};
use exchanges::*;
pub use http::Http;

/// Nombre de bougies demandé par défaut pour une chart (assez pour la mise en route de tous les indicateurs).
pub const DEFAULT_CANDLES: usize = 5000;

/// Les sources « directes » (Yahoo, flux RSS, calendrier) se lisent sans le relais du site : sur le desktop, et dans
/// la coque Tauri (son cœur Rust fait la requête, sans les règles CORS d'un navigateur). Dans un navigateur normal :
/// par le relais du site (`/api/…`).
pub fn direct() -> bool {
    #[cfg(all(feature = "web", not(feature = "native")))]
    {
        coque::active()
    }
    #[cfg(not(all(feature = "web", not(feature = "native"))))]
    {
        cfg!(feature = "native")
    }
}

/// Le message quand une fonction qui demande une clé est utilisée dans la coque Tauri (avant l'étape T2).
pub fn coque_sans_cle() -> String {
    "🔑 Dans la coque, les clés arrivent à l'étape T2 (coffre du système) : pas encore de clé ici".into()
}

/// ⚡ Premier écran : au plus autant de bougies récentes au premier chargement (une seule page presque partout) ; le
/// passé se remplit ensuite en arrière-plan jusqu'à `DEFAULT_CANDLES`.
pub const FIRST_SCREEN: usize = 500;
/// Garde-fou : nombre maximum de pages par demande.
const MAX_PAGES: usize = 80;

/// Au-delà de ce pourcentage de bougies refusées, toute la réponse est rejetée.
pub const MAX_REFUS_PCT: usize = 5;

/// « Trop de requêtes » (HTTP 429) pendant le remplissage par pages : attentes qui doublent, de 2 s à 60 s, puis
/// abandon (on garde ce qu'on a déjà, la chart n'est jamais bloquée).
pub const BUSY_WAITS: [u64; 6] = [2_000, 4_000, 8_000, 16_000, 32_000, 60_000];

/// La ligne du journal pour une bougie refusée : exchange, symbole, unité, heure UTC, raison. Rien d'autre (ni adresse,
/// ni paramètre de requête).
pub fn texte_refus(ex: Exchange, base: &str, tf: Timeframe, t: i64, raison: &str) -> String {
    let heure = chrono::DateTime::from_timestamp(t, 0).map_or(t.to_string(), |d| d.format("%Y-%m-%d %H:%M UTC").to_string());
    format!("⚠️ Bougie refusée · {} {base} {tf} · {heure} · {raison}", ex.name())
}

/// 📇 Fiche d'un exchange : bougies au plus par requête, pause entre deux pages (ms).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fiche {
    pub page: usize,
    pub pause_ms: u64,
}

/// Bougies vérifiées : les bonnes, et les heures des bougies refusées (des trous à réparer, étape 3 du plan).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Checked {
    pub bars: Vec<Bar>,
    pub a_reparer: Vec<i64>,
}

/// Les exchanges offerts par Argos.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Exchange {
    Hyperliquid,
    Okx,
    Binance,
    Bybit,
    Bitstamp,
    Weex,
    Bitunix,
    Blofin,
    Bitget,
    Gate,
    Mexc,
    Kucoin,
    Coinbase,
    Kraken,
    /// 🪙 Spot de Binance, OKX, Bybit et Bitget (même exchange, autre marché que le perp).
    BinanceSpot,
    OkxSpot,
    BybitSpot,
    BitgetSpot,
    /// Aster (perps, API façon Binance Futures).
    Aster,
    /// Bitfinex : perps (contre USDT et contre BTC) et son spot (contre USD et contre BTC), en lecture seule.
    Bitfinex,
    BitfinexSpot,
}

impl Exchange {
    pub const ALL: [Exchange; 21] = [
        Self::Hyperliquid,
        Self::Okx,
        Self::Binance,
        Self::Bybit,
        Self::Bitstamp,
        Self::Weex,
        Self::Bitunix,
        Self::Blofin,
        Self::Bitget,
        Self::Gate,
        Self::Mexc,
        Self::Kucoin,
        Self::Coinbase,
        Self::Kraken,
        Self::BinanceSpot,
        Self::OkxSpot,
        Self::BybitSpot,
        Self::BitgetSpot,
        Self::Aster,
        Self::Bitfinex,
        Self::BitfinexSpot,
    ];

    /// 🪙 Le marché spot de cet exchange, s'il en est un.
    pub fn spot_venue(self) -> Option<exchanges::spot::Venue> {
        use exchanges::spot::Venue;
        Some(match self {
            Self::BinanceSpot => Venue::Binance,
            Self::OkxSpot => Venue::Okx,
            Self::BybitSpot => Venue::Bybit,
            Self::BitgetSpot => Venue::Bitget,
            _ => return None,
        })
    }

    /// 🪙 Le spot d'un exchange qui a aussi des perps (Binance → Binance Spot…).
    pub fn spot_of(self) -> Option<Self> {
        Some(match self {
            Self::Binance => Self::BinanceSpot,
            Self::Okx => Self::OkxSpot,
            Self::Bybit => Self::BybitSpot,
            Self::Bitget => Self::BitgetSpot,
            Self::Bitfinex => Self::BitfinexSpot,
            _ => return None,
        })
    }

    /// Bitfinex (its perps or its spot): `Some(perp)`, else `None`.
    pub fn bitfinex(self) -> Option<bool> {
        match self {
            Self::Bitfinex => Some(true),
            Self::BitfinexSpot => Some(false),
            _ => None,
        }
    }

    /// The candles asked to the exchange for `tf`, and how many of them make one `tf` candle (Bitfinex has no 4 h:
    /// 4 × 1 h). Everywhere else: `tf` itself.
    pub fn source_tf(self, tf: Timeframe) -> (Timeframe, usize) {
        match self.bitfinex() {
            Some(_) => exchanges::bitfinex::source_tf(tf),
            None => (tf, 1),
        }
    }

    /// Sends no candle for an interval without a trade (seen on 24 Nov. 2026: Bitstamp's minute of 05:42 for bitcoin,
    /// Bitfinex's quiet minutes of DOGE): a gap between two of its candles is a quiet time, not a missing candle.
    pub fn skips_quiet_candles(self) -> bool {
        matches!(self, Self::Bitstamp | Self::Bitfinex | Self::BitfinexSpot)
    }

    /// Exchange de base d'un marché spot (Binance Spot → Binance), sinon lui-même.
    pub fn parent(self) -> Self {
        match self {
            Self::BinanceSpot => Self::Binance,
            Self::OkxSpot => Self::Okx,
            Self::BybitSpot => Self::Bybit,
            Self::BitgetSpot => Self::Bitget,
            Self::BitfinexSpot => Self::Bitfinex,
            e => e,
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            Self::Hyperliquid => "hl",
            Self::Okx => "okx",
            Self::Binance => "binance",
            Self::Bybit => "bybit",
            Self::Bitstamp => "bitstamp",
            Self::Weex => "weex",
            Self::Bitunix => "bitunix",
            Self::Blofin => "blofin",
            Self::Bitget => "bitget",
            Self::Gate => "gate",
            Self::Mexc => "mexc",
            Self::Kucoin => "kucoin",
            Self::Coinbase => "coinbase",
            Self::Kraken => "kraken",
            Self::Aster => "aster",
            Self::Bitfinex => "bitfinex",
            Self::BitfinexSpot => "bitfinex-spot",
            Self::BinanceSpot => "binance-spot",
            Self::OkxSpot => "okx-spot",
            Self::BybitSpot => "bybit-spot",
            Self::BitgetSpot => "bitget-spot",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Hyperliquid => "Hyperliquid",
            Self::Okx => "OKX",
            Self::Binance => "Binance",
            Self::Bybit => "Bybit",
            Self::Bitstamp => "Bitstamp",
            Self::Weex => "WEEX",
            Self::Bitunix => "Bitunix",
            Self::Blofin => "BloFin",
            Self::Bitget => "Bitget",
            Self::Gate => "Gate.io",
            Self::Mexc => "MEXC",
            Self::Kucoin => "KuCoin",
            Self::Coinbase => "Coinbase",
            Self::Kraken => "Kraken",
            Self::Aster => "Aster",
            Self::Bitfinex => "Bitfinex",
            Self::BitfinexSpot => "Bitfinex Spot",
            Self::BinanceSpot => "Binance Spot",
            Self::OkxSpot => "OKX Spot",
            Self::BybitSpot => "Bybit Spot",
            Self::BitgetSpot => "Bitget Spot",
        }
    }

    /// Le ticker complet, écrit comme l'exchange l'affiche (« BTCUSDT », « BTC-USDT-SWAP », « BTC-USDC »…).
    pub fn ticker(self, base: &str) -> String {
        // HIP-3 sur Hyperliquid (« xyz:NVDA ») : le nom tel que l'exchange l'écrit
        if self == Self::Hyperliquid && base.contains(':') {
            return base.to_string();
        }
        // ₿ a pair against BTC (« XRP/BTC »), written as the exchange writes it
        if let (a, Some(q)) = paire::parts(base) {
            let (a, q) = (a.to_uppercase(), q.to_uppercase());
            return match self {
                Self::OkxSpot | Self::Kucoin | Self::Coinbase => format!("{a}-{q}"),
                Self::Bitstamp | Self::Kraken => format!("{a}/{q}"),
                Self::Bitfinex | Self::BitfinexSpot => exchanges::bitfinex::ticker(base, self == Self::Bitfinex),
                _ => format!("{a}{q}"),
            };
        }
        let b = base.to_uppercase();
        match self {
            Self::Hyperliquid => format!("{b}-USDC"),
            Self::Okx => format!("{b}-USDT-SWAP"),
            Self::Blofin => format!("{b}-USDT"),
            Self::Binance | Self::Bybit | Self::Weex | Self::Bitunix => format!("{b}USDT"),
            Self::Bitstamp => format!("{b}/USD"),
            Self::Bitget | Self::Aster => format!("{b}USDT"),
            Self::Gate | Self::Mexc => format!("{b}_USDT"),
            Self::Kucoin => format!("{b}-USDT"),
            Self::Coinbase => format!("{b}-USD"),
            Self::Kraken => format!("{b}/USD"),
            Self::BinanceSpot | Self::BybitSpot | Self::BitgetSpot => format!("{b}USDT"),
            Self::OkxSpot => format!("{b}-USDT"),
            Self::Bitfinex => format!("{b}USDT"),
            Self::BitfinexSpot => format!("{b}USD"),
        }
    }

    /// Type de marché : perpétuel ou spot.
    pub fn market_kind(self) -> &'static str {
        match self {
            Self::Bitstamp | Self::Kucoin | Self::Coinbase | Self::Kraken | Self::BitfinexSpot => "Spot",
            e if e.spot_venue().is_some() => "Spot",
            _ => "Perpétuel",
        }
    }

    /// Titre de chart : « BTCUSDT Perpétuel · 30m · Binance ».
    pub fn chart_title(self, base: &str, interval: &str) -> String {
        format!("{} {} · {interval} · {}", self.ticker(base), self.market_kind(), self.name())
    }

    pub fn parse(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|e| e.id() == id)
    }

    /// Bloque les navigateurs (CORS) : sur le web, passer par le relais Argos.
    #[cfg(test)]
    pub fn needs_relay(self) -> bool {
        matches!(self, Self::Weex | Self::Bitunix | Self::Mexc | Self::Kucoin)
    }

    /// Une page de bougies. `end_ms` : seulement celles d'avant ce moment (pagination vers le passé) ; `None` = les plus récentes.
    pub fn candles_request(self, base: &str, tf: Timeframe, limit: usize, now_ms: i64, end_ms: Option<i64>) -> Result<Request> {
        match self {
            Self::Hyperliquid => hyperliquid::candles_request(base, tf, limit, now_ms, end_ms),
            Self::Okx => okx::candles_request(base, tf, limit, end_ms),
            Self::Binance => binance::candles_request(base, tf, limit, end_ms),
            Self::Bybit => bybit::candles_request(base, tf, limit, end_ms),
            Self::Bitstamp => bitstamp::candles_request(base, tf, limit, end_ms),
            Self::Weex => weex::candles_request(base, tf, limit, end_ms),
            Self::Bitunix => bitunix::candles_request(base, tf, limit, end_ms),
            Self::Blofin => blofin::candles_request(base, tf, limit, end_ms),
            Self::Bitget => bitget::candles_request(base, tf, limit, end_ms),
            Self::Gate => gate::candles_request(base, tf, limit, end_ms),
            Self::Mexc => mexc::candles_request(base, tf, limit, end_ms),
            Self::Kucoin => kucoin::candles_request(base, tf, limit, end_ms),
            Self::Coinbase => coinbase::candles_request(base, tf, limit, end_ms),
            Self::Kraken => kraken::candles_request(base, tf, limit, end_ms),
            Self::Aster => exchanges::aster::candles_request(base, tf, limit, end_ms),
            Self::Bitfinex | Self::BitfinexSpot => exchanges::bitfinex::candles_request(base, self == Self::Bitfinex, tf, limit, end_ms),
            e => exchanges::spot::candles_request(e.spot_venue().expect("spot"), base, tf, limit, end_ms),
        }
    }

    /// 📇 Fiche de l'exchange : bougies au plus par requête et pause entre deux pages (limites publiques de chaque
    /// API). Les connecteurs ne demandent jamais plus que `page` (test `fiches_des_exchanges`).
    pub fn fiche(self) -> Fiche {
        let page = match self {
            Self::Hyperliquid => 5000,
            Self::Gate => 1999,
            Self::Mexc => 2000,
            Self::Binance | Self::Kucoin | Self::Aster => 1500,
            Self::Blofin => 1440,
            Self::Bybit | Self::Bitget | Self::Bitstamp | Self::BinanceSpot | Self::BybitSpot | Self::BitgetSpot => 1000,
            Self::Kraken => 720,
            Self::Okx | Self::OkxSpot | Self::Weex | Self::Coinbase => 300,
            Self::Bitunix => 200,
            Self::Bitfinex | Self::BitfinexSpot => exchanges::bitfinex::PAGE,
        };
        let pause_ms = match self {
            // Bitfinex : about 30 candle requests a minute for everyone
            Self::Bitfinex | Self::BitfinexSpot => 2_100,
            Self::Okx | Self::OkxSpot | Self::Bitunix | Self::Weex | Self::Kucoin | Self::Coinbase | Self::Kraken => 150,
            _ => 60,
        };
        Fiche { page, pause_ms }
    }

    /// Pause entre deux pages (ms), pour rester sous les limites de requêtes de l'exchange.
    pub fn page_pause_ms(self) -> u64 {
        self.fiche().pause_ms
    }

    pub fn parse_candles(self, body: &str) -> Result<Vec<Bar>> {
        self.parse_candles_checked(body).map(|(bars, _)| bars)
    }

    /// Les bougies d'une réponse, vérifiées : les incohérentes sont retirées et rendues à part (heure, raison). Si
    /// plus de 5 % des bougies sont refusées, **toute** la réponse est rejetée (le superviseur passe alors à
    /// l'exchange de secours).
    pub fn parse_candles_checked(self, body: &str) -> Result<(Vec<Bar>, Vec<(i64, &'static str)>)> {
        let bars = match self {
            Self::Hyperliquid => hyperliquid::parse_candles(body),
            Self::Okx => okx::parse_candles(body),
            Self::Binance => binance::parse_candles(body),
            Self::Bybit => bybit::parse_candles(body),
            Self::Bitstamp => bitstamp::parse_candles(body),
            Self::Weex => weex::parse_candles(body),
            Self::Bitunix => bitunix::parse_candles(body),
            Self::Blofin => blofin::parse_candles(body),
            Self::Bitget => bitget::parse_candles(body),
            Self::Gate => gate::parse_candles(body),
            Self::Mexc => mexc::parse_candles(body),
            Self::Kucoin => kucoin::parse_candles(body),
            Self::Coinbase => coinbase::parse_candles(body),
            Self::Kraken => kraken::parse_candles(body),
            Self::Aster => exchanges::aster::parse_candles(body),
            Self::Bitfinex | Self::BitfinexSpot => exchanges::bitfinex::parse_candles(body),
            e => exchanges::spot::parse_candles(e.spot_venue().expect("spot"), body),
        }?;
        let n = bars.len();
        let (bars, refus) = clean_bars_detail(bars);
        if refus.len() * 100 > n * MAX_REFUS_PCT {
            return Err(ArgosError::Feed(format!(
                "réponse rejetée : {} bougies incohérentes sur {n} (plus de {MAX_REFUS_PCT} %)",
                refus.len()
            )));
        }
        Ok((bars, refus))
    }

    pub fn markets_request(self) -> Request {
        match self {
            Self::Hyperliquid => hyperliquid::markets_request(),
            Self::Okx => okx::markets_request(),
            Self::Binance => binance::markets_request(),
            Self::Bybit => bybit::markets_request(),
            Self::Bitstamp => bitstamp::markets_request(),
            Self::Weex => weex::markets_request(),
            Self::Bitunix => bitunix::markets_request(),
            Self::Blofin => blofin::markets_request(),
            Self::Bitget => bitget::markets_request(),
            Self::Gate => gate::markets_request(),
            Self::Mexc => mexc::markets_request(),
            Self::Kucoin => kucoin::markets_request(),
            Self::Coinbase => coinbase::markets_request(),
            Self::Kraken => kraken::markets_request(),
            Self::Aster => exchanges::aster::markets_request(),
            Self::Bitfinex | Self::BitfinexSpot => exchanges::bitfinex::markets_request(),
            e => exchanges::spot::markets_request(e.spot_venue().expect("spot")),
        }
    }

    pub fn parse_markets(self, body: &str) -> Result<Vec<Market>> {
        let mut m = match self {
            Self::Hyperliquid => hyperliquid::parse_markets(body),
            Self::Okx => okx::parse_markets(body),
            Self::Binance => binance::parse_markets(body),
            Self::Bybit => bybit::parse_markets(body),
            Self::Bitstamp => bitstamp::parse_markets(body),
            Self::Weex => weex::parse_markets(body),
            Self::Bitunix => bitunix::parse_markets(body),
            Self::Blofin => blofin::parse_markets(body),
            Self::Bitget => bitget::parse_markets(body),
            Self::Gate => gate::parse_markets(body),
            Self::Mexc => mexc::parse_markets(body),
            Self::Kucoin => kucoin::parse_markets(body),
            Self::Coinbase => coinbase::parse_markets(body),
            Self::Kraken => kraken::parse_markets(body),
            Self::Aster => exchanges::aster::parse_markets(body),
            Self::Bitfinex | Self::BitfinexSpot => exchanges::bitfinex::parse_markets(body, self == Self::Bitfinex),
            e => exchanges::spot::parse_markets(e.spot_venue().expect("spot"), body),
        }?;
        m.retain(|x| !x.base.is_empty() && x.price.is_finite());
        m.sort_by(|a, b| a.base.cmp(&b.base));
        Ok(m)
    }
}

/// Candles of a shorter interval made into `tf` candles (`k` of them in one: Bitfinex's 4 h from 1 h); `k` = 1:
/// unchanged. A candle to repair is then the `tf` candle that holds it.
fn regrouper(c: Checked, tf: Timeframe, k: usize) -> Checked {
    if k <= 1 {
        return c;
    }
    let sec = tf.seconds();
    let mut a_reparer: Vec<i64> = c.a_reparer.iter().map(|t| t - t.rem_euclid(sec)).collect();
    a_reparer.dedup();
    Checked {
        bars: argos_core::aggregate(&c.bars, sec),
        a_reparer,
    }
}

/// Volumes counted in contracts, given in coins (`size`: the coins in one contract; `None`: already in coins).
fn in_coins(mut bars: Vec<Bar>, size: Option<f64>) -> Vec<Bar> {
    if let Some(k) = size {
        for b in &mut bars {
            b.volume *= k;
        }
    }
    bars
}

/// Un exchange branché sur un client HTTP : c'est ce que le reste d'Argos utilise (contrat `Feed` du Core).
pub struct ExchangeFeed<H: Http> {
    pub exchange: Exchange,
    http: H,
    /// Horloge injectée (millisecondes Unix) : la vraie en production, une fixe en test.
    now_ms: fn() -> i64,
    /// Waits and asks again on « too many requests » (HTTP 429); `false` for a question the chart must not wait
    /// minutes for (a second opinion on a long wick, bug B-016).
    patient: bool,
}

pub(crate) fn system_now_ms() -> i64 {
    web_time::SystemTime::now()
        .duration_since(web_time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

impl<H: Http> ExchangeFeed<H> {
    pub fn new(exchange: Exchange, http: H) -> Self {
        Self {
            exchange,
            http,
            now_ms: system_now_ms,
            patient: true,
        }
    }

    pub fn with_clock(exchange: Exchange, http: H, now_ms: fn() -> i64) -> Self {
        Self {
            exchange,
            http,
            now_ms,
            patient: true,
        }
    }

    /// The same exchange, that gives up at once on « too many requests » (HTTP 429) instead of waiting up to a minute.
    pub(crate) fn impatient(mut self) -> Self {
        self.patient = false;
        self
    }

    /// 📜 Bougies plus anciennes que `before_s` (secondes Unix), page par page vers le passé, jusqu'à `limit` : pour
    /// dézoomer plus loin que ce qui est chargé. Vide : l'exchange n'a rien de plus ancien (début de l'historique).
    pub async fn older(&self, base: &str, tf: Timeframe, before_s: i64, limit: usize) -> Result<Vec<Bar>> {
        self.older_checked(base, tf, before_s, limit).await.map(|c| c.bars)
    }

    /// Comme `older`, avec les heures des bougies refusées (trous à réparer).
    pub async fn older_checked(&self, base: &str, tf: Timeframe, before_s: i64, limit: usize) -> Result<Checked> {
        self.older_pages(base, tf, before_s, limit, MAX_PAGES).await
    }

    /// [`Self::older`] in one question: one page, at most what the exchange gives at once (MEXC and Gate.io also ask
    /// the size of their contract). For a second opinion the chart waits for (bug B-016).
    pub(crate) async fn older_one_page(&self, base: &str, tf: Timeframe, before_s: i64, limit: usize) -> Result<Vec<Bar>> {
        self.older_pages(base, tf, before_s, limit, 1).await.map(|c| c.bars)
    }

    /// [`Self::older_checked`] in at most `pages` questions, without the false prices already taken out of the chart
    /// this session (bug B-025: the history filled or a hole repaired never puts one back).
    async fn older_pages(&self, base: &str, tf: Timeframe, before_s: i64, limit: usize, pages: usize) -> Result<Checked> {
        let (src, k) = self.exchange.source_tf(tf);
        let c = self
            .older_natif(base, src, before_s - before_s.rem_euclid(tf.seconds()), limit * k, pages)
            .await?;
        let mut c = regrouper(c, tf, k);
        supervisor::drop_known_false(self.exchange, base, tf, &mut c.bars);
        Ok(c)
    }

    /// [`Self::older_checked`] in an interval the exchange has, in at most `pages` questions.
    async fn older_natif(&self, base: &str, tf: Timeframe, before_s: i64, limit: usize, pages: usize) -> Result<Checked> {
        let base = base.to_uppercase();
        let size = self.coins_per_contract(&base).await?;
        let now = (self.now_ms)();
        let mut all: Vec<Bar> = vec![];
        let mut a_reparer: Vec<i64> = vec![];
        let mut end = before_s;
        for k in 0..pages {
            if all.len() >= limit {
                break;
            }
            if k > 0 {
                self.http.wait(self.exchange.page_pause_ms()).await;
            }
            let req = self.exchange.candles_request(&base, tf, limit - all.len(), now, Some(end * 1000 - 1))?;
            let body = match self.send_patient(req).await {
                Ok(b) => b,
                Err(e) if all.is_empty() => return Err(e),
                Err(_) => break,
            };
            let (page, refus) = match self.exchange.parse_candles_checked(&body) {
                Ok((page, refus)) => (in_coins(page, size), refus),
                Err(e) if all.is_empty() => return Err(e),
                Err(_) => break,
            };
            self.noter_refus(&base, tf, &refus);
            a_reparer.extend(refus.iter().map(|r| r.0).filter(|t| *t < end));
            let older: Vec<Bar> = page.into_iter().filter(|b| b.time < end).collect();
            let Some(first) = older.first().map(|b| b.time) else { break };
            all = argos_core::feed::clean_bars([older, all].concat());
            end = first;
        }
        let skip = all.len().saturating_sub(limit);
        Ok(Checked {
            bars: all.split_off(skip),
            a_reparer,
        })
    }

    /// Bougies récentes vérifiées (comme `Feed::candles`), avec les heures des bougies refusées (trous à réparer).
    pub async fn candles_checked(&self, base: &str, tf: Timeframe, limit: usize) -> Result<Checked> {
        let (src, k) = self.exchange.source_tf(tf);
        let c = self.candles_natif(base, src, limit * k).await?;
        Ok(regrouper(c, tf, k))
    }

    /// [`Self::candles_checked`] in an interval the exchange has.
    async fn candles_natif(&self, base: &str, tf: Timeframe, limit: usize) -> Result<Checked> {
        let base = base.to_uppercase();
        let size = self.coins_per_contract(&base).await?;
        let now = (self.now_ms)();
        let first = self.send(self.exchange.candles_request(&base, tf, limit, now, None)?).await?;
        let (all, refus) = self.exchange.parse_candles_checked(&first)?;
        let mut all = in_coins(all, size);
        self.noter_refus(&base, tf, &refus);
        let mut a_reparer: Vec<i64> = refus.iter().map(|r| r.0).collect();
        for _ in 1..MAX_PAGES {
            let Some(oldest) = all.first().map(|b| b.time) else { break };
            if all.len() >= limit {
                break;
            }
            self.http.wait(self.exchange.page_pause_ms()).await;
            let req = self.exchange.candles_request(&base, tf, limit - all.len(), now, Some(oldest * 1000 - 1))?;
            let Ok(body) = self.send_patient(req).await else { break };
            let Ok((page, refus)) = self.exchange.parse_candles_checked(&body) else {
                break;
            };
            let page = in_coins(page, size);
            self.noter_refus(&base, tf, &refus);
            a_reparer.extend(refus.iter().map(|r| r.0).filter(|t| *t < oldest));
            let older: Vec<Bar> = page.into_iter().filter(|b| b.time < oldest).collect();
            if older.is_empty() {
                break;
            }
            all = argos_core::feed::clean_bars([older, all].concat());
        }
        let skip = all.len().saturating_sub(limit);
        let bars = all.split_off(skip);
        let debut = bars.first().map_or(i64::MAX, |b| b.time);
        a_reparer.retain(|t| *t >= debut);
        a_reparer.sort_unstable();
        a_reparer.dedup();
        Ok(Checked { bars, a_reparer })
    }

    /// 📏 MEXC and Gate.io count the volume of their candles in contracts: the coins in one contract of `base`, asked
    /// to the exchange with each request of candles (and kept for the live candles and the trades,
    /// `contracts::set_contracts`). If it does not answer: the size read before, or noted by hand (BTC, ETH, SOL, XRP);
    /// if none, no candle (never a volume in contracts shown as coins). `None`: an exchange that counts in coins.
    async fn coins_per_contract(&self, base: &str) -> Result<Option<f64>> {
        let (name, req): (&str, Request) = match self.exchange {
            Exchange::Mexc => ("MEXC", mexc::contract_request(base)),
            Exchange::Gate => ("Gate.io", gate::contract_request(base)),
            _ => return Ok(None),
        };
        let read = match self.send(req).await {
            Ok(body) if self.exchange == Exchange::Mexc => mexc::parse_contract(&body),
            Ok(body) => gate::parse_contract(&body),
            Err(e) => Err(e),
        };
        match read {
            Ok(k) => {
                contracts::set_contracts(name, vec![(base.to_string(), k)]);
                Ok(Some(k))
            }
            Err(e) => contracts::contract(name, base)
                .map(Some)
                .ok_or_else(|| ArgosError::Feed(format!("{name} : taille du contrat de {base} inconnue ({e})"))),
        }
    }

    /// 📝 Note les bougies refusées dans le journal, sans rien de sensible : exchange, symbole, unité, heure UTC et
    /// raison (jamais l'adresse de la requête ni ses paramètres). Au plus 5 lignes par réponse, puis un total.
    fn noter_refus(&self, base: &str, tf: Timeframe, refus: &[(i64, &'static str)]) {
        for (t, raison) in refus.iter().take(5) {
            eprintln!("{}", texte_refus(self.exchange, base, tf, *t, raison));
        }
        if refus.len() > 5 {
            eprintln!(
                "⚠️ … et {} autre(s) bougie(s) refusée(s) · {} {base} {tf}",
                refus.len() - 5,
                self.exchange.name()
            );
        }
    }

    /// Une requête de remplissage : sur « trop de requêtes » (HTTP 429), attend et réessaie : le délai demandé par
    /// l'exchange (`Retry-After`, 60 s au plus), sinon 2 s, 4 s… 60 s, avec un petit délai au hasard en plus. An
    /// [impatient](Self::impatient) feed asks once.
    async fn send_patient(&self, req: Request) -> Result<String> {
        let mut k = 0;
        loop {
            match self.send(req.clone()).await {
                Err(e) if self.patient && e.to_string().contains("429") && k < BUSY_WAITS.len() => {
                    let ms = http::retry_after_ms(&e.to_string()).unwrap_or(BUSY_WAITS[k]);
                    self.http.wait(http::jitter(ms)).await;
                    k += 1;
                }
                r => return r,
            }
        }
    }

    /// 📇 Catalogue : tous les symboles de l'exchange (infos fixes, sans prix). Hyperliquid : ses perps + tous les
    /// dex HIP-3 ; Bybit : page par page.
    pub async fn instruments(&self) -> Result<Vec<Instrument>> {
        let one = |r: Request| self.send(r);
        Ok(match self.exchange {
            Exchange::Hyperliquid => {
                let mut all = hyperliquid::parse_instruments(&one(hyperliquid::instruments_request("")).await?, "")?;
                // HIP-3 : un dex qui ne répond pas n'empêche pas les autres ; ⏩ tous en même temps
                let dexes = hyperliquid::parse_dexes(&one(hyperliquid::dexes_request()).await?)?;
                let asks = dexes.iter().map(|dex| async move {
                    let body = one(hyperliquid::instruments_request(dex)).await.ok()?;
                    hyperliquid::parse_instruments(&body, dex).ok()
                });
                all.extend(http::ensemble(asks.collect()).await.into_iter().flatten().flatten());
                all
            }
            Exchange::Bybit => {
                let (mut all, mut next) = bybit::parse_instruments(&one(bybit::instruments_request(None)).await?)?;
                for _ in 1..MAX_PAGES {
                    let Some(cursor) = next.take() else { break };
                    let (page, n) = bybit::parse_instruments(&one(bybit::instruments_request(Some(&cursor))).await?)?;
                    all.extend(page);
                    next = n;
                }
                all
            }
            Exchange::Okx => okx::parse_instruments(&one(okx::instruments_request()).await?)?,
            Exchange::Binance => binance::parse_instruments(&one(binance::instruments_request()).await?)?,
            Exchange::Bitstamp => bitstamp::parse_instruments(&one(bitstamp::instruments_request()).await?)?,
            Exchange::Weex => weex::parse_instruments(&one(weex::instruments_request()).await?)?,
            Exchange::Bitunix => bitunix::parse_instruments(&one(bitunix::instruments_request()).await?)?,
            Exchange::Blofin => blofin::parse_instruments(&one(blofin::instruments_request()).await?)?,
            Exchange::Bitget => bitget::parse_instruments(&one(bitget::instruments_request()).await?)?,
            Exchange::Gate => gate::parse_instruments(&one(gate::instruments_request()).await?)?,
            Exchange::Mexc => mexc::parse_instruments(&one(mexc::instruments_request()).await?)?,
            Exchange::Kucoin => kucoin::parse_instruments(&one(kucoin::instruments_request()).await?)?,
            Exchange::Coinbase => coinbase::parse_instruments(&one(coinbase::instruments_request()).await?)?,
            Exchange::Kraken => kraken::parse_instruments(&one(kraken::instruments_request()).await?)?,
            Exchange::Aster => exchanges::aster::parse_instruments(&one(exchanges::aster::instruments_request()).await?)?,
            // Bitfinex : spot list, perpetuals list and prices (decimals), the same three for both catalogues; each
            // keeps its own markets
            Exchange::Bitfinex | Exchange::BitfinexSpot => {
                let [s, f, t] = exchanges::bitfinex::instruments_requests();
                let (s, f, t) = (one(s).await?, one(f).await?, one(t).await?);
                let mut all = exchanges::bitfinex::parse_instruments(&s, &f, &t)?;
                all.retain(|i| i.exchange == self.exchange.id());
                all
            }
            e => {
                let v = e.spot_venue().expect("spot");
                exchanges::spot::parse_instruments(v, &one(exchanges::spot::instruments_request(v)).await?)?
            }
        })
    }

    async fn send(&self, req: Request) -> Result<String> {
        match req {
            Request::Get(url) => self.http.get(&url).await,
            Request::PostJson { url, body } => self.http.post_json(&url, &body).await,
        }
    }
}

impl<H: Http> Feed for ExchangeFeed<H> {
    fn id(&self) -> &'static str {
        self.exchange.id()
    }
    fn name(&self) -> &'static str {
        self.exchange.name()
    }
    /// Autant de bougies que demandé, page par page vers le passé (chaque exchange a sa limite par requête).
    /// S'arrête quand l'exchange n'a plus rien de plus ancien (ou ignore la pagination), ou si une page échoue après la 1re
    /// (on garde alors ce qu'on a déjà).
    async fn candles(&self, base: &str, tf: Timeframe, limit: usize) -> Result<Vec<Bar>> {
        self.candles_checked(base, tf, limit).await.map(|c| c.bars)
    }
    async fn markets(&self) -> Result<Vec<Market>> {
        let body = self.send(self.exchange.markets_request()).await?;
        self.exchange.parse_markets(&body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ticker_complet_comme_l_exchange() {
        assert_eq!(Exchange::Binance.ticker("btc"), "BTCUSDT");
        assert_eq!(Exchange::Okx.ticker("BTC"), "BTC-USDT-SWAP");
        assert_eq!(Exchange::Blofin.ticker("BTC"), "BTC-USDT");
        assert_eq!(Exchange::Hyperliquid.ticker("BTC"), "BTC-USDC");
        assert_eq!(Exchange::Bitstamp.ticker("BTC"), "BTC/USD");
        assert_eq!(Exchange::Hyperliquid.ticker("xyz:NVDA"), "xyz:NVDA", "HIP-3 tel quel");
        assert_eq!(Exchange::Binance.chart_title("BTC", "30m"), "BTCUSDT Perpétuel · 30m · Binance");
        assert_eq!(Exchange::Bitstamp.chart_title("ETH", "4h"), "ETH/USD Spot · 4h · Bitstamp");
    }

    /// 📜 Pages plus anciennes : chaque requête demande avant la plus vieille bougie reçue ; rien de plus ancien = fin.
    #[tokio::test]
    async fn historique_plus_ancien_par_pages() {
        struct Pages(std::sync::Mutex<Vec<String>>);
        impl Http for Pages {
            async fn get(&self, url: &str) -> Result<String> {
                self.0.lock().unwrap().push(url.to_string());
                Ok("[]".into())
            }
            async fn post_json(&self, _url: &str, body: &str) -> Result<String> {
                self.0.lock().unwrap().push(body.to_string());
                // Hyperliquid : 3 bougies d'une heure juste avant la fin demandée, rien avant 1 699 980 000 s
                let v: serde_json::Value = serde_json::from_str(body).unwrap();
                let end = v["req"]["endTime"].as_i64().unwrap() / 1000;
                if end < 1_699_980_000 {
                    return Ok("[]".into());
                }
                let bars: Vec<String> = (1..=3)
                    .rev()
                    .map(|k| {
                        let t = (end / 3600 - k + 1) * 3600 * 1000;
                        format!(r#"{{"t":{t},"o":"1","h":"2","l":"0.5","c":"1.5","v":"10"}}"#)
                    })
                    .collect();
                Ok(format!("[{}]", bars.join(",")))
            }
        }
        let feed = ExchangeFeed::with_clock(Exchange::Hyperliquid, Pages(Default::default()), || 1_800_000_000_000);
        let before = 1_700_007_200; // 2 h au-dessus de la limite : 3 pages
        let got = feed.older("BTC", Timeframe::H1, before, 7).await.unwrap();
        assert_eq!(got.len(), 7, "autant que demandé, en plusieurs pages");
        assert!(got.windows(2).all(|w| w[0].time < w[1].time), "dans l'ordre, sans doublon");
        assert!(got.iter().all(|b| b.time < before), "tout est plus ancien");
        let start = feed.older("BTC", Timeframe::H1, 1_600_000_000, 100).await.unwrap();
        assert!(start.is_empty(), "rien de plus ancien : début de l'historique");
    }

    /// Faux client : renvoie toujours la même réponse et garde la dernière requête.
    struct Fake(&'static str, std::sync::Mutex<String>);
    impl Http for Fake {
        async fn get(&self, url: &str) -> Result<String> {
            *self.1.lock().unwrap() = url.to_string();
            Ok(self.0.to_string())
        }
        async fn post_json(&self, url: &str, body: &str) -> Result<String> {
            *self.1.lock().unwrap() = format!("{url} {body}");
            Ok(self.0.to_string())
        }
    }

    #[tokio::test]
    async fn chemin_complet_okx_nettoie_et_trie() {
        let fake = Fake(
            // (the volume in coins at index 6, in contracts at 5)
            r#"{"code":"0","data":[["1700001800000","2","3","1","2.5","100","1","2.5","1"],["1700000000000","1","2","0.5","2","50","0.5","1","1"],["1700001800000","2","3","1","2.6","101","1.01","2.6","1"]]}"#,
            Default::default(),
        );
        let feed = ExchangeFeed::new(Exchange::Okx, fake);
        let bars = feed.candles("zec", Timeframe::M30, 300).await.unwrap();
        assert_eq!(bars.iter().map(|b| b.time).collect::<Vec<_>>(), vec![1_700_000_000, 1_700_001_800]);
        assert_eq!(bars[1].close, 2.6);
        assert!(feed.http.1.lock().unwrap().contains("instId=ZEC-USDT-SWAP"));
    }

    #[tokio::test]
    async fn horloge_injectee_hyperliquid() {
        let feed = ExchangeFeed::with_clock(Exchange::Hyperliquid, Fake("[]", Default::default()), || 3_600_000);
        assert!(feed.candles("BTC", Timeframe::M1, 10).await.unwrap().is_empty());
        assert!(feed.http.1.lock().unwrap().contains(r#""startTime":3000000,"endTime":3600000"#));
    }

    /// Faux Hyperliquid : une réponse selon la demande (meta, perpDexs, meta d'un dex).
    struct HlCatalog;
    impl Http for HlCatalog {
        async fn get(&self, _: &str) -> Result<String> {
            Err(ArgosError::Feed("pas de GET".into()))
        }
        async fn post_json(&self, _: &str, body: &str) -> Result<String> {
            Ok(if body.contains("perpDexs") {
                r#"[null,{"name":"xyz"},{"name":"mort"}]"#
            } else if body.contains(r#""dex":"xyz""#) {
                r#"{"universe":[{"name":"xyz:NVDA","szDecimals":3,"growthMode":"enabled"}]}"#
            } else if body.contains(r#""dex":"mort""#) {
                return Err(ArgosError::Feed("dex en panne".into()));
            } else {
                r#"{"universe":[{"name":"BTC","szDecimals":5},{"name":"ETH","szDecimals":4}]}"#
            }
            .into())
        }
    }

    #[tokio::test]
    async fn catalogue_hyperliquid_avec_hip3() {
        let all = ExchangeFeed::new(Exchange::Hyperliquid, HlCatalog).instruments().await.unwrap();
        let ids: Vec<String> = all.iter().map(|i| i.id()).collect();
        assert_eq!(ids, vec!["hl:BTC", "hl:ETH", "hl:xyz:NVDA"], "un dex en panne n'empêche pas les autres");
        assert!(all[2].growth);
    }

    #[tokio::test]
    async fn erreur_exchange_remontee() {
        let feed = ExchangeFeed::new(Exchange::Bybit, Fake(r#"{"retCode":10001,"retMsg":"params error"}"#, Default::default()));
        assert!(matches!(feed.candles("BTC", Timeframe::H1, 10).await, Err(ArgosError::Feed(_))));
    }

    /// Faux Binance : 3000 bougies 1h en mémoire, renvoyées par pages selon `endTime` et `limit` (1500 max), comme le vrai.
    struct PagedBinance(std::sync::Mutex<usize>);
    impl Http for PagedBinance {
        async fn get(&self, url: &str) -> Result<String> {
            *self.0.lock().unwrap() += 1;
            let q = |k: &str| url.split(&format!("{k}=")).nth(1).map(|v| v.split('&').next().unwrap().parse::<i64>().unwrap());
            let limit = q("limit").unwrap().min(1500) as usize;
            let end = q("endTime").unwrap_or(i64::MAX);
            let all: Vec<i64> = (0..3000).map(|i| 1_700_000_000_000 + i * 3_600_000).filter(|&t| t <= end).collect();
            let page = &all[all.len().saturating_sub(limit)..];
            let rows: Vec<String> = page.iter().map(|t| format!(r#"[{t},"1","2","0.5","1.5","10"]"#)).collect();
            Ok(format!("[{}]", rows.join(",")))
        }
        async fn post_json(&self, _: &str, _: &str) -> Result<String> {
            unreachable!()
        }
    }

    #[tokio::test]
    async fn pagination_jusqu_a_la_limite_puis_arret() {
        let feed = ExchangeFeed::new(Exchange::Binance, PagedBinance(Default::default()));
        // 2500 bougies : 2 pages (1500 + 1000), sans trou ni doublon
        let bars = feed.candles("BTC", Timeframe::H1, 2500).await.unwrap();
        assert_eq!(bars.len(), 2500);
        assert!(bars.windows(2).all(|w| w[1].time - w[0].time == 3600));
        assert_eq!(bars.last().unwrap().time, (1_700_000_000_000 + 2999 * 3_600_000) / 1000);
        assert_eq!(*feed.http.0.lock().unwrap(), 2);
        // 5000 demandées mais l'exchange n'en a que 3000 : on s'arrête proprement avec tout ce qu'il a
        let feed = ExchangeFeed::new(Exchange::Binance, PagedBinance(Default::default()));
        assert_eq!(feed.candles("BTC", Timeframe::H1, DEFAULT_CANDLES).await.unwrap().len(), 3000);
    }

    #[tokio::test]
    async fn exchange_qui_ignore_la_pagination_ne_boucle_pas() {
        // renvoie toujours la même page : 1 seule page supplémentaire tentée, puis arrêt
        let fake = Fake(
            r#"{"code":"0","data":[["1700001800000","2","3","1","2.5","100","0.1","0.25","1"],["1700000000000","1","2","0.5","2","50","0.05","0.1","1"]]}"#,
            Default::default(),
        );
        let feed = ExchangeFeed::new(Exchange::Blofin, fake);
        assert_eq!(feed.candles("BTC", Timeframe::M30, 5000).await.unwrap().len(), 2);
        assert!(feed.http.1.lock().unwrap().contains("after=1699999999999"));
    }

    /// 🔒 Reliable data: MEXC and Gate.io count the volume of their candles in contracts. The feed asks the size of
    /// the coin's contract with each request of candles, and gives the volume in coins; when the exchange does not
    /// answer, the size known before (or noted by hand) is used, and with none there is no candle at all (never a
    /// volume in contracts shown as coins).
    #[tokio::test]
    async fn mexc_and_gate_volumes_in_coins() {
        /// MEXC and Gate.io: their contract sizes (or a failure), and one candle of 500 contracts.
        struct Contracts(std::sync::Mutex<Vec<String>>, bool);
        impl Http for Contracts {
            async fn get(&self, url: &str) -> Result<String> {
                self.0.lock().unwrap().push(url.to_string());
                let size = |body: &str| {
                    if self.1 {
                        Ok(body.to_string())
                    } else {
                        Err(ArgosError::Feed("HTTP 503".into()))
                    }
                };
                if url.contains("/contract/detail?symbol=") {
                    return size(r#"{"success":true,"code":0,"data":{"symbol":"ZZFEEDM_USDT","contractSize":0.01}}"#);
                }
                if url.contains("/futures/usdt/contracts/") {
                    return size(r#"{"name":"ZZFEEDG_USDT","quanto_multiplier":"0.001"}"#);
                }
                if url.contains("/contract/kline/") {
                    return Ok(
                        r#"{"success":true,"code":0,"data":{"time":[1700000000],"open":[1],"close":[2],"high":[3],"low":[0.5],"vol":[500],"amount":[1000]}}"#
                            .into(),
                    );
                }
                if url.contains("/futures/usdt/candlesticks?") {
                    return Ok(r#"[{"o":"1","v":500,"t":1700000000,"c":"2","l":"0.5","h":"3","sum":"1000"}]"#.into());
                }
                Err(ArgosError::Feed(format!("not asked here: {url}")))
            }
            async fn post_json(&self, url: &str, _body: &str) -> Result<String> {
                Err(ArgosError::Feed(format!("not asked here: {url}")))
            }
        }
        let feed = |ex, answers| ExchangeFeed::with_clock(ex, Contracts(Default::default(), answers), || 1_700_000_100_000);

        // the size read with the candles: the volume in coins, and the size kept for the live candles and trades
        for (ex, coin, k) in [(Exchange::Mexc, "ZZFEEDM", 0.01), (Exchange::Gate, "ZZFEEDG", 0.001)] {
            let f = feed(ex, true);
            let c = f.candles_checked(coin, Timeframe::M1, 1).await.unwrap();
            assert_eq!(c.bars.len(), 1, "{ex:?}");
            assert!((c.bars[0].volume - 500.0 * k).abs() < 1e-12, "{ex:?}: {}", c.bars[0].volume);
            let asked = f.http.0.lock().unwrap().clone();
            assert_eq!(asked.len(), 2, "the size, then the candles: {asked:?}");
            assert_eq!(contracts::contract(ex.name(), coin), Some(k), "{ex:?}");
            // older candles too
            let older = f.older_checked(coin, Timeframe::M1, 1_700_000_060, 1).await.unwrap();
            assert!((older.bars[0].volume - 500.0 * k).abs() < 1e-12, "{ex:?}");
        }
        // no answer about the size: the one noted by hand for BTC; none for an unknown coin, and no candle asked
        for ex in [Exchange::Mexc, Exchange::Gate] {
            let f = feed(ex, false);
            let c = f.candles_checked("BTC", Timeframe::M1, 1).await.unwrap();
            assert!((c.bars[0].volume - 500.0 * 0.0001).abs() < 1e-12, "{ex:?}: {}", c.bars[0].volume);
            let f = feed(ex, false);
            let e = f.candles_checked("ZZFEEDX", Timeframe::M1, 1).await.unwrap_err().to_string();
            assert!(e.contains("taille du contrat de ZZFEEDX inconnue"), "{ex:?}: {e}");
            assert_eq!(f.http.0.lock().unwrap().len(), 1, "{ex:?}: only the size was asked");
        }
        // an exchange that counts in coins asks nothing more
        let f = ExchangeFeed::new(Exchange::Blofin, Fake(r#"{"code":"0","data":[]}"#, Default::default()));
        assert!(f.candles_checked("BTC", Timeframe::M1, 1).await.unwrap().bars.is_empty());
    }

    #[test]
    fn huit_exchanges_identifiants_uniques() {
        for e in Exchange::ALL {
            assert_eq!(Exchange::parse(e.id()), Some(e));
            assert!(e.candles_request("BTC", Timeframe::H1, 100, 0, None).is_ok());
            assert!(e.candles_request("BTC", Timeframe::H1, 100, 0, Some(1_700_000_000_000)).is_ok());
        }
        assert_eq!(Exchange::ALL.iter().filter(|e| e.needs_relay()).count(), 4, "WEEX, Bitunix, MEXC, KuCoin");
    }

    /// Corps Binance de `n` bougies d'une heure (la plus récente à `end_ms`) ; celles des positions `bad` sont
    /// incohérentes (haut sous le bas).
    fn binance_body(n: i64, bad: &[i64], end_ms: i64) -> String {
        let rows: Vec<String> = (0..n)
            .map(|i| {
                let t = end_ms - (n - 1 - i) * 3_600_000;
                let (h, l) = if bad.contains(&i) { ("0.5", "2") } else { ("2", "0.5") };
                format!(r#"[{t},"1","{h}","{l}","1.5","10"]"#)
            })
            .collect();
        format!("[{}]", rows.join(","))
    }

    #[test]
    fn refus_4_5_et_6_pour_cent() {
        let end = 1_700_000_000_000;
        // 4 % et 5 % : la réponse est gardée, les refusées sont rendues à part (heures)
        for bad in [4, 5] {
            let pos: Vec<i64> = (0..bad).map(|k| k * 17).collect();
            let (bars, refus) = Exchange::Binance.parse_candles_checked(&binance_body(100, &pos, end)).unwrap();
            assert_eq!((bars.len(), refus.len()), (100 - bad as usize, bad as usize), "{bad} %");
            assert!(refus.iter().all(|r| r.1 == "haut sous l'ouverture ou la clôture"));
        }
        // 6 % : toute la réponse est rejetée
        let pos: Vec<i64> = (0..6).map(|k| k * 17).collect();
        let e = Exchange::Binance.parse_candles_checked(&binance_body(100, &pos, end)).unwrap_err().to_string();
        assert!(e.contains("rejetée") && e.contains("6 bougies incohérentes sur 100"), "{e}");
    }

    #[tokio::test]
    async fn bougies_refusees_deviennent_des_trous_a_reparer() {
        // 100 bougies, celles des positions 40 et 41 refusées : 98 gardées, 2 trous à réparer à leurs heures
        let end = 1_700_000_000_000;
        let fake = Fake(Box::leak(binance_body(100, &[40, 41], end).into_boxed_str()), Default::default());
        let c = ExchangeFeed::new(Exchange::Binance, fake)
            .candles_checked("BTC", Timeframe::H1, 98)
            .await
            .unwrap();
        assert_eq!(c.bars.len(), 98);
        let t = |i: i64| (end - (99 - i) * 3_600_000) / 1000;
        assert_eq!(c.a_reparer, vec![t(40), t(41)]);
        assert!(c.bars.iter().all(|b| b.time != t(40) && b.time != t(41)));
    }

    #[test]
    fn ligne_du_journal_sans_rien_de_sensible() {
        let l = texte_refus(Exchange::Okx, "BTC", Timeframe::H1, 1_700_000_000, "volume négatif ou illisible");
        assert_eq!(l, "⚠️ Bougie refusée · OKX BTC 1h · 2023-11-14 22:13 UTC · volume négatif ou illisible");
        assert!(!l.contains("http") && !l.contains('?') && !l.contains('='), "ni adresse ni paramètre");
    }

    #[test]
    fn fiches_des_exchanges() {
        for e in Exchange::ALL {
            let f = e.fiche();
            assert!(f.page >= 100 && f.pause_ms >= 60, "{e:?}");
            // un connecteur ne demande jamais plus que sa fiche
            let Ok(Request::Get(url)) = e.candles_request("BTC", Timeframe::H1, 1_000_000, 1_800_000_000_000, None) else {
                continue;
            };
            if let Some(n) = url
                .split("limit=")
                .nth(1)
                .and_then(|v| v.split('&').next())
                .and_then(|v| v.parse::<usize>().ok())
            {
                assert_eq!(n, f.page, "{e:?} : {url}");
            }
        }
        assert_eq!(Exchange::Okx.page_pause_ms(), 150);
    }

    /// « Trop de requêtes » (429) sur une page du remplissage : attentes 2 s puis 4 s, puis la page arrive.
    #[tokio::test]
    async fn trop_de_requetes_attend_de_plus_en_plus() {
        struct Occupe(std::sync::Mutex<(usize, Vec<u64>)>);
        impl Http for Occupe {
            async fn get(&self, url: &str) -> Result<String> {
                let n = {
                    let mut g = self.0.lock().unwrap();
                    g.0 += 1;
                    g.0
                };
                match n {
                    2 | 3 => Err(ArgosError::Feed("HTTP 429 Too Many Requests : ralentis".into())),
                    _ => PagedBinance(Default::default()).get(url).await,
                }
            }
            async fn post_json(&self, _: &str, _: &str) -> Result<String> {
                unreachable!()
            }
            async fn wait(&self, ms: u64) {
                self.0.lock().unwrap().1.push(ms);
            }
        }
        let feed = ExchangeFeed::new(Exchange::Binance, Occupe(Default::default()));
        let bars = feed.candles("BTC", Timeframe::H1, 2500).await.unwrap();
        assert_eq!(bars.len(), 2500, "la 2e page arrive après les attentes");
        let w = feed.http.0.lock().unwrap().1.clone();
        assert_eq!(w[0], 60, "pause de page");
        assert!(
            (2_000..=2_500).contains(&w[1]) && (4_000..=5_000).contains(&w[2]),
            "2 s puis 4 s, + 0 à 25 % : {w:?}"
        );
    }

    /// L'exchange dit combien attendre (Retry-After) : on attend ce délai (+ un peu de hasard), pas l'attente prévue.
    #[tokio::test]
    async fn retry_after_respecte() {
        struct Dit(std::sync::Mutex<(usize, Vec<u64>)>);
        impl Http for Dit {
            async fn get(&self, url: &str) -> Result<String> {
                let n = {
                    let mut g = self.0.lock().unwrap();
                    g.0 += 1;
                    g.0
                };
                match n {
                    2 => Err(ArgosError::Feed("HTTP 429 Too Many Requests (Retry-After 7 s) : ralentis".into())),
                    _ => PagedBinance(Default::default()).get(url).await,
                }
            }
            async fn post_json(&self, _: &str, _: &str) -> Result<String> {
                unreachable!()
            }
            async fn wait(&self, ms: u64) {
                self.0.lock().unwrap().1.push(ms);
            }
        }
        let feed = ExchangeFeed::new(Exchange::Binance, Dit(Default::default()));
        assert_eq!(feed.candles("BTC", Timeframe::H1, 2500).await.unwrap().len(), 2500);
        let w = feed.http.0.lock().unwrap().1.clone();
        assert!(w.len() == 2 && (7_000..=8_750).contains(&w[1]), "7 s demandées : {w:?}");
    }

    #[test]
    fn retry_after_et_hasard() {
        use http::{jitter, parse_retry_after, retry_after_ms};
        let now = 1_793_000_000; // lun. 26 oct. 2026 07:33:20 UTC
        assert_eq!(parse_retry_after("12", now), Some(12));
        assert_eq!(parse_retry_after("3600", now), Some(60), "60 s au plus");
        assert_eq!(parse_retry_after("Mon, 26 Oct 2026 07:33:50 GMT", now), Some(30), "date HTTP : 30 s plus tard");
        assert_eq!(parse_retry_after("Mon, 26 Oct 2026 07:00:00 GMT", now), Some(0), "date passée : tout de suite");
        assert_eq!(parse_retry_after("bientôt", now), None);
        assert_eq!(retry_after_ms("HTTP 429 (Retry-After 9 s) : x"), Some(9_000));
        assert_eq!(retry_after_ms("HTTP 429 : x"), None);
        for _ in 0..50 {
            let j = jitter(2_000);
            assert!((2_000..=2_500).contains(&j), "{j}");
        }
        assert_eq!(jitter(0), 0);
    }
}
