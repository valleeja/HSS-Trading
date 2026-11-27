//! 🦎 DEX (paires on-chain) : sources **gratuites et sans clé**, vérifiées le 2026-11-05.
//! - **DexScreener** (`api.dexscreener.com`) : recherche d'un jeton, d'un symbole ou d'une adresse de contrat, sur
//!   toutes les chaînes ; prix, liquidité, volume et nombre d'achats / ventes. Pas de chandelles.
//! - **GeckoTerminal** (`api.geckoterminal.com/api/v2`, public : **10 appels par minute**) : chandelles OHLCV d'une
//!   pool, sa fiche (liquidité, volume 24 h, acheteurs / vendeurs) et ses 300 derniers trades (24 h).
//!
//! - **CoinMarketCap DEX** (seulement avec la clé de l'utilisateur, desktop) : chandelles `/v1/k-line/candles` (plan
//!   gratuit « Basic »), 1 crédit par appel, compté dans le compteur du mois. Sans clé : GeckoTerminal.
//!
//! Les deux sources gratuites acceptent le navigateur (CORS `*`). Pour respecter les limites, les appels sont espacés (`reserve_slot`) :
//! GeckoTerminal un appel toutes les 6,5 s au plus, DexScreener un par seconde.

use crate::http::Http;
use argos_core::{ArgosError, Bar, Result, Timeframe};
use serde_json::Value;

pub const GT: &str = "https://api.geckoterminal.com/api/v2";
pub const DS: &str = "https://api.dexscreener.com";
/// Pause minimale entre deux appels GeckoTerminal (10 par minute) et DexScreener.
pub const GT_PAUSE_MS: u64 = 6_500;
pub const DS_PAUSE_MS: u64 = 1_000;

/// Une paire trouvée (DexScreener) : chaîne, DEX, adresse de la pool, jetons, prix, liquidité, volume et
/// transactions sur 24 h.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Pair {
    pub chain: String,
    pub dex: String,
    pub pool: String,
    pub base: String,
    pub base_addr: String,
    pub quote: String,
    pub price_usd: Option<f64>,
    pub liq_usd: Option<f64>,
    pub vol_24h: Option<f64>,
    pub buys_24h: u64,
    pub sells_24h: u64,
}

impl Pair {
    /// Réseau GeckoTerminal de la chaîne DexScreener (les noms diffèrent pour quelques chaînes).
    pub fn network(&self) -> String {
        network_of(&self.chain)
    }
}

/// Fiche d'une pool (GeckoTerminal) : sur 24 h, achats / ventes et acheteurs / vendeurs distincts.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PoolInfo {
    pub name: String,
    /// Adresse du jeton de base (pour les chandelles CoinMarketCap).
    pub base_addr: String,
    pub price_usd: Option<f64>,
    pub liq_usd: Option<f64>,
    pub vol_24h: Option<f64>,
    pub chg_24h: Option<f64>,
    pub buys: u64,
    pub sells: u64,
    pub buyers: u64,
    pub sellers: u64,
    /// Valeur totale diluée et capitalisation (GeckoTerminal).
    pub fdv: Option<f64>,
    pub mcap: Option<f64>,
    /// Création de la pool (secondes Unix).
    pub created_s: Option<i64>,
    /// Variations 5 min, 1 h, 6 h (%) ; volumes 1 h et 6 h ; achats / ventes sur 1 h.
    pub chg_5m: Option<f64>,
    pub chg_1h: Option<f64>,
    pub chg_6h: Option<f64>,
    pub vol_1h: Option<f64>,
    pub vol_6h: Option<f64>,
    pub buys_1h: u64,
    pub sells_1h: u64,
    /// Part de la liquidité verrouillée (%).
    pub locked_pct: Option<f64>,
    /// 🛡️ Fiche du jeton (lue une fois par jeton).
    pub token: Option<TokenInfo>,
}

/// 🛡️ Fiche d'un jeton (GeckoTerminal `tokens/…/info`, gratuit) : détenteurs, score, vérifié, signaux de risque.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TokenInfo {
    pub holders: Option<u64>,
    /// Part des 10 plus gros détenteurs (%).
    pub top10_pct: Option<f64>,
    /// Score GeckoTerminal (0 à 100).
    pub gt_score: Option<f64>,
    pub verified: bool,
    pub honeypot: Option<bool>,
    /// Autorités encore actives (Solana) : le créateur peut créer des jetons / geler des comptes.
    pub mint_authority: bool,
    pub freeze_authority: bool,
    pub twitter: String,
    pub website: String,
}

/// Un trade récent d'une pool.
#[derive(Debug, Clone, PartialEq)]
pub struct Trade {
    pub time_s: i64,
    pub buy: bool,
    pub usd: f64,
    pub price_usd: f64,
}

/// Réseau GeckoTerminal d'une chaîne DexScreener.
pub fn network_of(chain: &str) -> String {
    match chain {
        "ethereum" => "eth",
        "polygon" => "polygon_pos",
        "avalanche" => "avax",
        "bsc" => "bsc",
        "arbitrum" => "arbitrum",
        "optimism" => "optimism",
        "base" => "base",
        "solana" => "solana",
        other => other,
    }
    .to_string()
}

/// Unité de temps GeckoTerminal (`minute` / `hour` / `day` et agrégat) la plus proche **en dessous** de `tf` ; l'app
/// regroupe ensuite elle-même (ex. 30m depuis 15m, 1W depuis 1D).
pub fn gt_tf(tf: Timeframe) -> (&'static str, u32, i64) {
    gt_step(tf.seconds())
}

/// Comme `gt_tf`, depuis une durée en secondes (unité de la chart).
pub fn gt_step(secs: i64) -> (&'static str, u32, i64) {
    match secs {
        s if s < 300 => ("minute", 1, 60),
        s if s < 900 => ("minute", 5, 300),
        s if s < 3600 => ("minute", 15, 900),
        s if s < 14_400 => ("hour", 1, 3600),
        s if s < 43_200 => ("hour", 4, 14_400),
        s if s < 86_400 => ("hour", 12, 43_200),
        _ => ("day", 1, 86_400),
    }
}

/// Unité de temps d'Argos d'un pas GeckoTerminal (secondes de `gt_step`).
pub fn tf_of(step_s: i64) -> Timeframe {
    match step_s {
        60 => Timeframe::M1,
        300 => Timeframe::M5,
        900 => Timeframe::M15,
        3600 => Timeframe::H1,
        14_400 => Timeframe::H4,
        43_200 => Timeframe::H12,
        _ => Timeframe::D1,
    }
}

/// Identifiant d'une paire DEX dans une watchlist : « dex:eth:0xa43f…:PEPE/WETH ».
pub fn id(network: &str, pool: &str, name: &str) -> String {
    format!("dex:{network}:{pool}:{name}")
}

/// (réseau, pool, nom) d'un identifiant `id`.
pub fn parse_id(id: &str) -> Option<(String, String, String)> {
    let mut it = id.strip_prefix("dex:")?.splitn(3, ':');
    let (n, p) = (it.next()?, it.next()?);
    (!n.is_empty() && !p.is_empty()).then(|| (n.to_string(), p.to_string(), it.next().unwrap_or(p).to_string()))
}

/// Texte tapé dans le panneau ＋ qui demande une recherche DEX : « dex:pepe », « dex pepe », ou une adresse de
/// contrat (0x… sur 40 chiffres hexadécimaux, ou une adresse Solana). `None` : recherche normale.
pub fn query_of(text: &str) -> Option<String> {
    let t = text.trim();
    let low = t.to_lowercase();
    if let Some(q) = low.strip_prefix("dex:").or_else(|| low.strip_prefix("dex ")) {
        let q = t[t.len() - q.len()..].trim();
        return (q.chars().count() >= 2).then(|| q.to_string());
    }
    let hex = t.len() == 42 && t.starts_with("0x") && t[2..].chars().all(|c| c.is_ascii_hexdigit());
    let sol = (32..=44).contains(&t.len()) && t.chars().all(|c| c.is_ascii_alphanumeric() && !"0OIl".contains(c));
    (hex || sol).then(|| t.to_string())
}

fn f(v: &Value) -> Option<f64> {
    v.as_f64().or_else(|| v.as_str().and_then(|s| s.parse().ok()))
}

fn u(v: &Value) -> u64 {
    v.as_u64().or_else(|| f(v).map(|x| x.max(0.0) as u64)).unwrap_or(0)
}

pub fn search_url(query: &str) -> String {
    let q: String = query
        .trim()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '$'))
        .take(80)
        .collect();
    format!("{DS}/latest/dex/search?q={q}")
}

pub fn ohlcv_url(network: &str, pool: &str, tf: Timeframe, before_s: Option<i64>) -> String {
    let (unit, agg, _) = gt_tf(tf);
    let before = before_s.map(|b| format!("&before_timestamp={b}")).unwrap_or_default();
    format!("{GT}/networks/{network}/pools/{pool}/ohlcv/{unit}?aggregate={agg}&limit=1000&currency=usd{before}")
}

pub fn pool_url(network: &str, pool: &str) -> String {
    format!("{GT}/networks/{network}/pools/{pool}")
}

pub fn token_info_url(network: &str, token: &str) -> String {
    format!("{GT}/networks/{network}/tokens/{token}/info")
}

/// 🛡️ Fiche d'un jeton GeckoTerminal.
pub fn parse_token_info(body: &str) -> Result<TokenInfo> {
    let v: Value = serde_json::from_str(body).map_err(|e| ArgosError::Feed(format!("GeckoTerminal : {e}")))?;
    let a = &v["data"]["attributes"];
    if !a.is_object() {
        return Err(ArgosError::Feed("GeckoTerminal : jeton introuvable".into()));
    }
    let auth = |k: &str| a[k].as_str().is_some_and(|s| !s.is_empty() && s != "no");
    Ok(TokenInfo {
        holders: a["holders"]["count"].as_u64(),
        top10_pct: f(&a["holders"]["distribution_percentage"]["top_10"]),
        gt_score: f(&a["gt_score"]),
        verified: a["gt_verified"].as_bool().unwrap_or(false),
        honeypot: a["is_honeypot"]
            .as_bool()
            .or_else(|| a["is_honeypot"].as_str().map(|s| s == "yes" || s == "true")),
        mint_authority: auth("mint_authority"),
        freeze_authority: auth("freeze_authority"),
        twitter: a["twitter_handle"].as_str().unwrap_or("").to_string(),
        website: a["websites"][0].as_str().unwrap_or("").to_string(),
    })
}

pub fn trades_url(network: &str, pool: &str) -> String {
    format!("{GT}/networks/{network}/pools/{pool}/trades")
}

/// Paires d'une recherche DexScreener, la plus liquide d'abord.
pub fn parse_search(body: &str) -> Result<Vec<Pair>> {
    let v: Value = serde_json::from_str(body).map_err(|e| ArgosError::Feed(format!("DexScreener : {e}")))?;
    let mut out: Vec<Pair> = v["pairs"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|p| {
                    Some(Pair {
                        chain: p["chainId"].as_str()?.to_string(),
                        dex: p["dexId"].as_str().unwrap_or("").to_string(),
                        pool: p["pairAddress"].as_str()?.to_string(),
                        base: p["baseToken"]["symbol"].as_str().unwrap_or("?").to_string(),
                        base_addr: p["baseToken"]["address"].as_str().unwrap_or("").to_string(),
                        quote: p["quoteToken"]["symbol"].as_str().unwrap_or("?").to_string(),
                        price_usd: f(&p["priceUsd"]),
                        liq_usd: f(&p["liquidity"]["usd"]),
                        vol_24h: f(&p["volume"]["h24"]),
                        buys_24h: u(&p["txns"]["h24"]["buys"]),
                        sells_24h: u(&p["txns"]["h24"]["sells"]),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    out.sort_by(|a, b| b.liq_usd.unwrap_or(0.0).total_cmp(&a.liq_usd.unwrap_or(0.0)));
    Ok(out)
}

/// Chandelles GeckoTerminal (`[t, o, h, l, c, v]`, les plus récentes d'abord) → bougies dans l'ordre du temps.
pub fn parse_ohlcv(body: &str) -> Result<Vec<Bar>> {
    let v: Value = serde_json::from_str(body).map_err(|e| ArgosError::Feed(format!("GeckoTerminal : {e}")))?;
    let list = v["data"]["attributes"]["ohlcv_list"]
        .as_array()
        .ok_or_else(|| ArgosError::Feed("GeckoTerminal : pas de chandelles".into()))?;
    let mut bars: Vec<Bar> = list
        .iter()
        .filter_map(|c| {
            let c = c.as_array()?;
            Some(Bar {
                time: f(c.first()?)? as i64,
                open: f(c.get(1)?)?,
                high: f(c.get(2)?)?,
                low: f(c.get(3)?)?,
                close: f(c.get(4)?)?,
                volume: f(c.get(5)?).unwrap_or(0.0),
            })
        })
        .filter(|b| b.open > 0.0 && b.close > 0.0)
        .collect();
    bars.sort_by_key(|b| b.time);
    bars.dedup_by_key(|b| b.time);
    Ok(bars)
}

/// Fiche d'une pool GeckoTerminal.
pub fn parse_pool(body: &str) -> Result<PoolInfo> {
    let v: Value = serde_json::from_str(body).map_err(|e| ArgosError::Feed(format!("GeckoTerminal : {e}")))?;
    let a = &v["data"]["attributes"];
    if !a.is_object() {
        return Err(ArgosError::Feed("GeckoTerminal : pool introuvable".into()));
    }
    let t = &a["transactions"]["h24"];
    Ok(PoolInfo {
        name: a["name"].as_str().unwrap_or("").to_string(),
        // « eth_0x6982… » : le réseau, puis l'adresse du jeton
        base_addr: v["data"]["relationships"]["base_token"]["data"]["id"]
            .as_str()
            .and_then(|i| i.split_once('_'))
            .map(|(_, a)| a.to_string())
            .unwrap_or_default(),
        price_usd: f(&a["base_token_price_usd"]),
        liq_usd: f(&a["reserve_in_usd"]),
        vol_24h: f(&a["volume_usd"]["h24"]),
        chg_24h: f(&a["price_change_percentage"]["h24"]),
        buys: u(&t["buys"]),
        sells: u(&t["sells"]),
        buyers: u(&t["buyers"]),
        sellers: u(&t["sellers"]),
        fdv: f(&a["fdv_usd"]),
        mcap: f(&a["market_cap_usd"]),
        created_s: a["pool_created_at"]
            .as_str()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|d| d.timestamp()),
        chg_5m: f(&a["price_change_percentage"]["m5"]),
        chg_1h: f(&a["price_change_percentage"]["h1"]),
        chg_6h: f(&a["price_change_percentage"]["h6"]),
        vol_1h: f(&a["volume_usd"]["h1"]),
        vol_6h: f(&a["volume_usd"]["h6"]),
        buys_1h: u(&a["transactions"]["h1"]["buys"]),
        sells_1h: u(&a["transactions"]["h1"]["sells"]),
        locked_pct: f(&a["locked_liquidity_percentage"]),
        token: None,
    })
}

/// Derniers trades d'une pool GeckoTerminal (les plus récents d'abord) ; le prix est celui du jeton de base.
pub fn parse_trades(body: &str) -> Result<Vec<Trade>> {
    let v: Value = serde_json::from_str(body).map_err(|e| ArgosError::Feed(format!("GeckoTerminal : {e}")))?;
    Ok(v["data"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|t| {
                    let a = &t["attributes"];
                    let buy = a["kind"].as_str()? == "buy";
                    // achat : on reçoit le jeton de base (to) ; vente : on le donne (from)
                    let price = if buy { f(&a["price_to_in_usd"]) } else { f(&a["price_from_in_usd"]) }?;
                    Some(Trade {
                        time_s: parse_time(a["block_timestamp"].as_str()?)?,
                        buy,
                        usd: f(&a["volume_in_usd"]).unwrap_or(0.0),
                        price_usd: price,
                    })
                })
                .collect()
        })
        .unwrap_or_default())
}

/// « 2026-11-05T03:05:23Z » → secondes Unix.
fn parse_time(s: &str) -> Option<i64> {
    let (d, t) = s.trim_end_matches('Z').split_once('T')?;
    let mut dp = d.split('-').map(|x| x.parse::<i64>());
    let (y, m, day) = (dp.next()?.ok()?, dp.next()?.ok()?, dp.next()?.ok()?);
    let mut tp = t.split(':').map(|x| x.split('.').next().unwrap_or("0").parse::<i64>());
    let (hh, mm, ss) = (tp.next()?.ok()?, tp.next()?.ok()?, tp.next().and_then(|r| r.ok()).unwrap_or(0));
    // jours depuis 1970 (algorithme de Howard Hinnant)
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    Some(days * 86_400 + hh * 3600 + mm * 60 + ss)
}

/// Heure du prochain appel permis, par source (0 : GeckoTerminal, 1 : DexScreener).
static NEXT: std::sync::Mutex<[Option<web_time::Instant>; 2]> = std::sync::Mutex::new([None, None]);

/// Délai d'attente avant un appel à la source `slot` (pause minimale `pause_ms`), et réservation du créneau.
pub fn reserve_slot(slot: usize, pause_ms: u64, now: web_time::Instant) -> u64 {
    let Ok(mut n) = NEXT.lock() else { return pause_ms };
    let at = n[slot].filter(|t| *t > now).unwrap_or(now);
    n[slot] = Some(at + std::time::Duration::from_millis(pause_ms));
    at.duration_since(now).as_millis() as u64
}

/// Appel espacé (limites de débit de la source).
async fn paced<H: Http>(http: &H, slot: usize, pause_ms: u64, url: &str) -> Result<String> {
    let wait = reserve_slot(slot, pause_ms, web_time::Instant::now());
    if wait > 0 {
        http.wait(wait).await;
    }
    http.get(url).await
}

/// Adresse du jeton de base de chaque pool déjà vue (recherche, fiche) : clé = adresse de la pool.
static TOKENS: std::sync::Mutex<Option<std::collections::HashMap<String, String>>> = std::sync::Mutex::new(None);

/// Retient le jeton de base d'une pool.
pub fn remember_token(pool: &str, token: &str) {
    if token.is_empty() {
        return;
    }
    if let Ok(mut t) = TOKENS.lock() {
        t.get_or_insert_with(Default::default).insert(pool.to_lowercase(), token.to_string());
    }
}

/// Jeton de base connu d'une pool.
pub fn token_of(pool: &str) -> Option<String> {
    TOKENS.lock().ok()?.as_ref()?.get(&pool.to_lowercase()).cloned()
}

/// Plateforme CoinMarketCap d'un réseau GeckoTerminal (vérifié : `ethereum` ; les autres suivent les noms de CMC).
pub fn cmc_platform(network: &str) -> String {
    match network {
        "eth" => "ethereum",
        "polygon_pos" => "polygon",
        "avax" => "avalanche",
        other => other,
    }
    .to_string()
}

/// Intervalle CoinMarketCap d'un pas (secondes) : « 1min », « 1h », « 4h », « 1d »… (« 1m » = un mois chez CMC).
pub fn cmc_interval(step_s: i64) -> &'static str {
    match step_s {
        60 => "1min",
        300 => "5min",
        900 => "15min",
        3600 => "1h",
        14_400 => "4h",
        43_200 => "12h",
        _ => "1d",
    }
}

pub fn cmc_kline_url(base: &str, network: &str, token: &str, tf: Timeframe) -> String {
    let (_, _, step) = gt_tf(tf);
    format!(
        "{base}/v1/k-line/candles?platform={}&address={token}&interval={}&limit=500",
        cmc_platform(network),
        cmc_interval(step)
    )
}

/// Chandelles CoinMarketCap `[open, high, low, close, volume, heure en ms, traders]` → bougies dans l'ordre du temps.
pub fn parse_cmc_kline(body: &str) -> Result<Vec<Bar>> {
    if let Some(e) = crate::cmcap::error_of(body) {
        return Err(ArgosError::Feed(format!("CoinMarketCap : {e}")));
    }
    let v: Value = serde_json::from_str(body).map_err(|e| ArgosError::Feed(format!("CoinMarketCap : {e}")))?;
    let list = v["data"]
        .as_array()
        .ok_or_else(|| ArgosError::Feed("CoinMarketCap : pas de chandelles".into()))?;
    let mut bars: Vec<Bar> = list
        .iter()
        .filter_map(|c| {
            let c = c.as_array()?;
            Some(Bar {
                time: f(c.get(5)?)? as i64 / 1000,
                open: f(c.first()?)?,
                high: f(c.get(1)?)?,
                low: f(c.get(2)?)?,
                close: f(c.get(3)?)?,
                volume: f(c.get(4)?).unwrap_or(0.0),
            })
        })
        .filter(|b| b.open > 0.0 && b.close > 0.0)
        .collect();
    bars.sort_by_key(|b| b.time);
    bars.dedup_by_key(|b| b.time);
    Ok(bars)
}

/// Fichier du cache local des chandelles CoinMarketCap d'une pool (dans le dossier du cache des bougies).
pub fn cmc_cache_name(pool: &str, tf: Timeframe) -> String {
    let p: String = pool.chars().filter(|c| c.is_ascii_alphanumeric()).take(64).collect();
    format!("cmc-dex-{}-{}.bin", p.to_lowercase(), tf.id())
}

/// Bougies de la pool. **CoinMarketCap seulement au premier chargement** d'une paire et d'une unité de temps (clé
/// réglée, jeton connu, crédit disponible) : ses chandelles vont dans un cache local. Ensuite, les relectures passent
/// par GeckoTerminal (gratuit) et les nouvelles bougies sont fusionnées avec cet historique. Sans clé : GeckoTerminal.
/// Renvoie aussi la source affichée.
pub async fn candles_any<H: Http>(http: &H, network: &str, pool: &str, tf: Timeframe) -> Result<(Vec<Bar>, &'static str)> {
    #[cfg(feature = "native")]
    {
        let path = crate::cache::dir().map(|d| d.join(cmc_cache_name(pool, tf)));
        let now = crate::system_now_ms() / 1000;
        let cached = path.as_deref().and_then(|p| crate::cache::read(p, tf, now));
        match cached {
            // déjà un historique CoinMarketCap : seulement les nouvelles bougies, gratuites
            Some(old) => {
                let fresh = candles(http, network, pool, tf, None).await.unwrap_or_default();
                let all = crate::cache::merge(&old, fresh, tf, 5000);
                if let Some(p) = &path {
                    let _ = crate::cache::write(p, &all, tf, now);
                }
                return Ok((all, "CoinMarketCap + GeckoTerminal"));
            }
            None => {
                let cmc = crate::cmcap::reglage();
                if let (Some((base, key, credits)), Some(token)) = (cmc, token_of(pool)) {
                    if crate::cmcap::reserve(&credits, 1, now) {
                        if let Ok(bars) = crate::cmcap::get(&cmc_kline_url(&base, network, &token, tf), &key)
                            .await
                            .map_err(ArgosError::Feed)
                            .and_then(|b| parse_cmc_kline(&b))
                        {
                            if bars.len() > 1 {
                                if let Some(p) = &path {
                                    let _ = crate::cache::write(p, &bars, tf, now);
                                }
                                return Ok((bars, "CoinMarketCap"));
                            }
                        }
                    }
                }
            }
        }
    }
    Ok((candles(http, network, pool, tf, None).await?, "GeckoTerminal"))
}

/// Recherche (DexScreener) : un symbole, un nom ou une adresse de contrat.
pub async fn search<H: Http>(http: &H, query: &str) -> Result<Vec<Pair>> {
    let pairs = parse_search(&paced(http, 1, DS_PAUSE_MS, &search_url(query)).await?)?;
    for p in &pairs {
        remember_token(&p.pool, &p.base_addr);
    }
    Ok(pairs)
}

/// Bougies de la pool à l'unité de temps GeckoTerminal la plus proche (jusqu'à 1000 ; `before_s` : plus anciennes).
pub async fn candles<H: Http>(http: &H, network: &str, pool: &str, tf: Timeframe, before_s: Option<i64>) -> Result<Vec<Bar>> {
    parse_ohlcv(&paced(http, 0, GT_PAUSE_MS, &ohlcv_url(network, pool, tf, before_s)).await?)
}

pub async fn pool<H: Http>(http: &H, network: &str, pool: &str) -> Result<PoolInfo> {
    let mut info = parse_pool(&paced(http, 0, GT_PAUSE_MS, &pool_url(network, pool)).await?)?;
    remember_token(pool, &info.base_addr);
    // 🛡️ fiche du jeton : un seul appel par jeton (gardée ensuite), pour rester sous 10 appels / min
    if !info.base_addr.is_empty() {
        let key = format!("{network}_{}", info.base_addr.to_lowercase());
        let cached = TOKEN_INFO.lock().ok().and_then(|m| m.as_ref().and_then(|m| m.get(&key).cloned()));
        info.token = match cached {
            Some(t) => Some(t),
            None => {
                let t = paced(http, 0, GT_PAUSE_MS, &token_info_url(network, &info.base_addr))
                    .await
                    .ok()
                    .and_then(|b| parse_token_info(&b).ok());
                if let (Some(t), Ok(mut m)) = (&t, TOKEN_INFO.lock()) {
                    m.get_or_insert_with(Default::default).insert(key, t.clone());
                }
                t
            }
        };
    }
    Ok(info)
}

/// Fiches des jetons déjà lues (réseau_adresse → fiche).
static TOKEN_INFO: std::sync::Mutex<Option<std::collections::HashMap<String, TokenInfo>>> = std::sync::Mutex::new(None);

pub async fn trades<H: Http>(http: &H, network: &str, pool: &str) -> Result<Vec<Trade>> {
    parse_trades(&paced(http, 0, GT_PAUSE_MS, &trades_url(network, pool)).await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fiche_du_jeton_et_details_de_la_pool() {
        let p = parse_pool(r#"{"data":{"attributes":{"name":"PEPE / WETH","pool_created_at":"2023-04-14T17:21:11Z","fdv_usd":"2044249538.7","market_cap_usd":"2078426309.1","price_change_percentage":{"m5":"0.1","h1":"-0.5","h6":"2","h24":"1.5"},"volume_usd":{"h1":"1000","h6":"9000","h24":"40000"},"transactions":{"h1":{"buys":12,"sells":8},"h24":{"buys":1,"sells":2,"buyers":1,"sellers":1}},"reserve_in_usd":"3","locked_liquidity_percentage":"0.0"}}}"#).unwrap();
        assert_eq!((p.chg_1h, p.vol_6h, p.buys_1h, p.sells_1h), (Some(-0.5), Some(9000.0), 12, 8));
        assert_eq!(p.created_s, Some(1_681_492_871));
        assert!(p.fdv.is_some() && p.mcap.is_some() && p.locked_pct == Some(0.0));
        let t = parse_token_info(r#"{"data":{"attributes":{"gt_score":90.47,"gt_verified":true,"holders":{"count":600369,"distribution_percentage":{"top_10":"35.2"}},"mint_authority":null,"freeze_authority":"no","is_honeypot":false,"twitter_handle":"pepecoineth","websites":["https://www.pepe.vip"]}}}"#).unwrap();
        assert_eq!((t.holders, t.top10_pct, t.verified, t.honeypot), (Some(600369), Some(35.2), true, Some(false)));
        assert!(!t.mint_authority && !t.freeze_authority, "null / « no » : pas d'autorité");
        assert_eq!(t.twitter, "pepecoineth");
    }

    // formats relevés en direct le 2026-11-05 (raccourcis)
    const SEARCH: &str = r#"{"schemaVersion":"1.0.0","pairs":[
        {"chainId":"robinhood","dexId":"uniswap","pairAddress":"0x113c","baseToken":{"address":"0x9B09","name":"THE CHOSEN FROG","symbol":"PEPE"},"quoteToken":{"symbol":"WETH"},"priceUsd":"0.0000001","txns":{"h24":{"buys":3,"sells":1}},"volume":{"h24":120.5},"liquidity":{"usd":900}},
        {"chainId":"ethereum","dexId":"uniswap","pairAddress":"0xa43fe16908251ee70ef74718545e4fe6c5ccec9f","baseToken":{"address":"0x6982508145454ce325ddbe47a25d4ec3d2311933","name":"Pepe","symbol":"PEPE"},"quoteToken":{"symbol":"WETH"},"priceUsd":"0.000005057","txns":{"h24":{"buys":647,"sells":787}},"volume":{"h24":4236311.6},"liquidity":{"usd":33163716.2}}]}"#;
    const OHLCV: &str = r#"{"data":{"id":"x","type":"ohlcv_request_response","attributes":{"ohlcv_list":[[1793847600,5.06e-06,5.08e-06,5.03e-06,5.05e-06,34487.9],[1793844000,5.02e-06,5.06e-06,4.99e-06,5.06e-06,106563.0]]}},"meta":{}}"#;
    const POOL: &str = r#"{"data":{"id":"eth_0xa43f","type":"pool","relationships":{"base_token":{"data":{"id":"eth_0x6982508145454ce325ddbe47a25d4ec3d2311933","type":"token"}}},"attributes":{"name":"PEPE / WETH","base_token_price_usd":"0.00000505766","reserve_in_usd":"33163716.2468","price_change_percentage":{"h24":"1.5"},"volume_usd":{"h24":"4236311.61"},"transactions":{"h24":{"buys":647,"sells":787,"buyers":274,"sellers":264}}}}}"#;
    const TRADES: &str = r#"{"data":[{"id":"t1","type":"trade","attributes":{"block_timestamp":"2026-11-05T03:05:23Z","kind":"sell","from_token_amount":"372.32","to_token_amount":"0.00000077","price_from_in_usd":"0.00000505766","price_to_in_usd":"2425.68","volume_in_usd":"0.00188"}},{"id":"t2","type":"trade","attributes":{"block_timestamp":"2026-11-05T03:04:00Z","kind":"buy","price_from_in_usd":"2425.68","price_to_in_usd":"0.0000051","volume_in_usd":"1500.5"}}]}"#;

    #[test]
    fn recherche_la_plus_liquide_d_abord() {
        let p = parse_search(SEARCH).unwrap();
        assert_eq!(p.len(), 2);
        assert_eq!((p[0].chain.as_str(), p[0].network().as_str(), p[0].base.as_str()), ("ethereum", "eth", "PEPE"));
        assert_eq!((p[0].buys_24h, p[0].sells_24h), (647, 787));
        assert!(
            search_url("pepe & co; drop").ends_with("q=pepecodrop"),
            "rien d'autre que des lettres et chiffres"
        );
        assert!(search_url("0x6982508145454ce325ddbe47a25d4ec3d2311933").ends_with("0x6982508145454ce325ddbe47a25d4ec3d2311933"));
    }

    #[test]
    fn chandelles_dans_l_ordre_du_temps() {
        let b = parse_ohlcv(OHLCV).unwrap();
        assert_eq!(b.len(), 2);
        assert!(b[0].time < b[1].time);
        assert_eq!(b[1].close, 5.05e-06);
        assert_eq!(gt_tf(Timeframe::M30), ("minute", 15, 900), "30m regroupé depuis 15m");
        assert_eq!(gt_tf(Timeframe::W1), ("day", 1, 86_400));
        let u = ohlcv_url("eth", "0xa43f", Timeframe::H4, Some(1_793_800_000));
        assert!(
            u.contains("/ohlcv/hour?aggregate=4&limit=1000") && u.ends_with("&before_timestamp=1793800000"),
            "{u}"
        );
    }

    #[test]
    fn fiche_et_trades() {
        let p = parse_pool(POOL).unwrap();
        assert_eq!((p.buyers, p.sellers, p.buys, p.sells), (274, 264, 647, 787));
        assert_eq!(p.base_addr, "0x6982508145454ce325ddbe47a25d4ec3d2311933");
        assert_eq!(p.liq_usd, Some(33_163_716.246_8));
        let t = parse_trades(TRADES).unwrap();
        assert_eq!(t.len(), 2);
        assert!(!t[0].buy && t[1].buy);
        assert_eq!(t[0].time_s, 1_793_847_923, "2026-11-05T03:05:23Z");
        assert_eq!(t[1].price_usd, 0.0000051, "achat : prix du jeton reçu");
        assert!(parse_pool(r#"{"errors":[{"status":"404"}]}"#).is_err());
    }

    #[test]
    fn identifiants_et_recherche() {
        let i = id("eth", "0xa43f", "PEPE/WETH");
        assert_eq!(parse_id(&i), Some(("eth".into(), "0xa43f".into(), "PEPE/WETH".into())));
        assert_eq!(parse_id("hl:BTC"), None);
        assert_eq!(query_of("dex:pepe").as_deref(), Some("pepe"));
        assert_eq!(query_of("DEX Pepe").as_deref(), Some("Pepe"));
        assert_eq!(
            query_of("0x6982508145454ce325ddbe47a25d4ec3d2311933").as_deref(),
            Some("0x6982508145454ce325ddbe47a25d4ec3d2311933")
        );
        assert!(query_of("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").is_some(), "adresse Solana");
        assert_eq!(query_of("BTC"), None);
        assert_eq!(query_of("BINANCE:BTCUSDT"), None);
        assert_eq!(tf_of(gt_step(1800).2), Timeframe::M15);
        assert_eq!(tf_of(gt_step(604_800).2), Timeframe::D1);
    }

    #[test]
    fn chandelles_coinmarketcap() {
        // format relevé en direct (2026-11-05) : [open, high, low, close, volume, heure ms, traders]
        let body = r#"{"data":[[0.0000049556,0.0000050370,0.0000049556,0.0000050370,92157.58,1793840400000,44],[0.0000050370,0.0000050678,0.0000049950,0.0000050678,108954.26,1793844000000,36]],"status":{"error_code":"0","credit_count":1}}"#;
        let b = parse_cmc_kline(body).unwrap();
        assert_eq!((b.len(), b[0].time, b[1].close), (2, 1_793_840_400, 0.000_005_067_8));
        assert!(parse_cmc_kline(r#"{"status":{"error_code":"1002","error_message":"API key missing."}}"#).is_err());
        let u = cmc_kline_url("https://pro-api.coinmarketcap.com", "eth", "0x6982", Timeframe::H4);
        assert!(u.ends_with("/v1/k-line/candles?platform=ethereum&address=0x6982&interval=4h&limit=500"), "{u}");
        assert_eq!((cmc_interval(60), cmc_interval(86_400)), ("1min", "1d"), "« 1m » serait un mois chez CMC");
        assert_eq!(cmc_cache_name("0xA43F/../x", Timeframe::H1), "cmc-dex-0xa43fx-1h.bin", "nom de fichier sûr");
        remember_token("0xABC", "0xtok");
        assert_eq!(token_of("0xabc").as_deref(), Some("0xtok"));
    }

    #[test]
    fn appels_espaces() {
        let t0 = web_time::Instant::now();
        // DexScreener (créneau 1) : le 1er part tout de suite, le 2e attend la pause
        let a = reserve_slot(1, 1000, t0);
        let b = reserve_slot(1, 1000, t0);
        assert!(a <= 1000 && b >= a + 999, "{a} {b}");
    }

    #[test]
    fn adresses_dans_la_liste_blanche() {
        for u in [
            search_url("pepe"),
            ohlcv_url("eth", "0x1", Timeframe::H1, None),
            pool_url("eth", "0x1"),
            trades_url("eth", "0x1"),
        ] {
            assert!(crate::net::verifier(&u).is_ok(), "{u}");
        }
    }
}
