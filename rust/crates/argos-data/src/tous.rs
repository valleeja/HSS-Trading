//! 🌐 « Tous les marchés » de la watchlist : tous les perps USDT d'une plateforme, triables par volume 24 h,
//! variation 24 h, volatilité 24 h, market cap, open interest ou nom.
//!
//! Une seule requête par plateforme (celle de `Exchange::markets_request`, déjà utilisée pour les prix), relue au
//! plus une fois par minute, et seulement pour la plateforme affichée. Chaque bourse donne prix, prix d'il y a 24 h,
//! volume ; le plus haut / plus bas 24 h et l'open interest quand elle les donne (sinon « — », rangé en bas).
//! La market cap ne vient pas des bourses : CoinGecko (gratuit, sans clé, top 500), relu toutes les 15 min.
//! Partie pure, testée hors ligne.

use crate::Exchange;
use argos_core::{ArgosError, Result};
use serde_json::Value;
use std::collections::HashMap;

/// Les plateformes du 1er lot (choix de Jee, 20 nov. 2026), puis Bitfinex, perps et spot (23 nov. 2026).
pub const BOURSES: [Exchange; 8] = [
    Exchange::Hyperliquid,
    Exchange::Binance,
    Exchange::Bybit,
    Exchange::Okx,
    Exchange::Bitunix,
    Exchange::Weex,
    Exchange::Bitfinex,
    Exchange::BitfinexSpot,
];

/// Un marché et ses chiffres sur 24 h (en dollars).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Ligne {
    /// Jeton de base (« BTC »), celui qu'ouvre la chart de la plateforme.
    pub base: String,
    /// Symbole de la plateforme (« BTCUSDT », « BTC-USDT-SWAP »), celui de son catalogue (`exchange:symbole`).
    pub symbol: String,
    pub price: f64,
    pub prev: f64,
    pub vol_usd: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub oi_usd: Option<f64>,
    pub cap: Option<f64>,
}

impl Ligne {
    /// Variation sur 24 h en %.
    pub fn pct(&self) -> Option<f64> {
        (self.prev > 0.0 && self.price > 0.0).then(|| (self.price / self.prev - 1.0) * 100.0)
    }
    /// Volatilité 24 h en % : (plus haut − plus bas) ÷ prix.
    pub fn volatilite(&self) -> Option<f64> {
        match (self.high, self.low) {
            (Some(h), Some(l)) if self.price > 0.0 && h >= l && l > 0.0 => Some((h - l) / self.price * 100.0),
            _ => None,
        }
    }
}

/// Les tris offerts (l'ordre du menu « Trier par »).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tri {
    #[default]
    Volume,
    Hausse,
    Baisse,
    Volatilite,
    MarketCap,
    OpenInterest,
    Nom,
}

impl Tri {
    pub const ALL: [Tri; 7] = [
        Tri::Volume,
        Tri::Hausse,
        Tri::Baisse,
        Tri::Volatilite,
        Tri::MarketCap,
        Tri::OpenInterest,
        Tri::Nom,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Tri::Volume => "Les plus tradés (volume 24 h)",
            Tri::Hausse => "Plus fortes hausses 24 h",
            Tri::Baisse => "Plus fortes baisses 24 h",
            Tri::Volatilite => "Volatilité 24 h",
            Tri::MarketCap => "Market cap",
            Tri::OpenInterest => "Open interest",
            Tri::Nom => "Nom A → Z",
        }
    }

    /// Nom court, pour la pilule « Trier par » de la watchlist.
    pub fn court(self) -> &'static str {
        match self {
            Tri::Volume => "Volume 24 h",
            Tri::Hausse => "Hausses 24 h",
            Tri::Baisse => "Baisses 24 h",
            Tri::Volatilite => "Volatilité 24 h",
            Tri::MarketCap => "Market cap",
            Tri::OpenInterest => "Open interest",
            Tri::Nom => "Nom A → Z",
        }
    }

    /// Clé courte gardée dans les préférences (ne jamais changer une clé existante).
    pub fn key(self) -> &'static str {
        match self {
            Tri::Volume => "vol",
            Tri::Hausse => "up",
            Tri::Baisse => "down",
            Tri::Volatilite => "volat",
            Tri::MarketCap => "cap",
            Tri::OpenInterest => "oi",
            Tri::Nom => "nom",
        }
    }

    pub fn parse(key: &str) -> Tri {
        Tri::ALL.into_iter().find(|t| t.key() == key).unwrap_or_default()
    }

    /// La valeur triée (plus grande d'abord), `None` = rangé en bas.
    fn valeur(self, l: &Ligne) -> Option<f64> {
        match self {
            Tri::Volume => l.vol_usd,
            Tri::Hausse => l.pct(),
            Tri::Baisse => l.pct().map(|p| -p),
            Tri::Volatilite => l.volatilite(),
            Tri::MarketCap => l.cap,
            Tri::OpenInterest => l.oi_usd,
            Tri::Nom => None,
        }
    }
}

/// Trie la liste ; les marchés sans la donnée vont en bas, par volume.
pub fn trier(lignes: &mut [Ligne], tri: Tri) {
    if tri == Tri::Nom {
        lignes.sort_by_key(|l| l.base.to_uppercase());
        return;
    }
    let vol = |l: &Ligne| l.vol_usd.unwrap_or(0.0);
    lignes.sort_by(|a, b| match (tri.valeur(a), tri.valeur(b)) {
        (Some(x), Some(y)) => y.total_cmp(&x),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => vol(b).total_cmp(&vol(a)),
    });
}

fn num(v: &Value) -> Option<f64> {
    v.as_f64().or_else(|| v.as_str().and_then(|s| s.parse().ok())).filter(|x: &f64| x.is_finite())
}

/// Le premier champ présent parmi plusieurs noms (les bourses changent parfois de nom).
fn champ(t: &Value, noms: &[&str]) -> Option<f64> {
    noms.iter().find_map(|n| num(&t[*n]))
}

fn json(body: &str, qui: &str) -> Result<Value> {
    serde_json::from_str(body).map_err(|e| ArgosError::Feed(format!("{qui} : {e}")))
}

fn code_ok(v: &Value, cle: &str, qui: &str) -> Result<()> {
    match &v[cle] {
        Value::Null => Ok(()),
        c if c.as_i64() == Some(0) || c.as_str() == Some("0") => Ok(()),
        _ => Err(ArgosError::Feed(format!(
            "{qui} : {}",
            v["msg"].as_str().or(v["retMsg"].as_str()).unwrap_or("erreur")
        ))),
    }
}

fn liste<'a>(v: &'a Value, qui: &str) -> Result<&'a Vec<Value>> {
    v.as_array().ok_or_else(|| ArgosError::Feed(format!("{qui} : liste absente")))
}

/// Lit la réponse de `ex.markets_request()` (contrats USDT des perps, comme la chart ; ₿ Bitfinex : aussi ses paires
/// contre BTC, que sa chart ouvre).
pub fn parse(ex: Exchange, body: &str) -> Result<Vec<Ligne>> {
    let qui = ex.name();
    let v = json(body, qui)?;
    let lignes: Vec<Ligne> = match ex {
        Exchange::Hyperliquid => {
            let (meta, ctx) = (liste(&v[0]["universe"], qui)?, &v[1]);
            meta.iter()
                .enumerate()
                .filter(|(_, u)| !u["isDelisted"].as_bool().unwrap_or(false))
                .filter_map(|(i, u)| {
                    let c = &ctx[i];
                    let price = num(&c["markPx"])?;
                    let name = u["name"].as_str()?;
                    Some(Ligne {
                        base: name.to_string(),
                        symbol: name.to_string(),
                        price,
                        prev: num(&c["prevDayPx"]).unwrap_or(0.0),
                        vol_usd: num(&c["dayNtlVlm"]),
                        oi_usd: num(&c["openInterest"]).map(|oi| oi * price),
                        ..Default::default()
                    })
                })
                .collect()
        }
        Exchange::Binance => liste(&v, qui)?
            .iter()
            .filter_map(|t| {
                Some(Ligne {
                    base: t["symbol"].as_str()?.strip_suffix("USDT")?.to_string(),
                    symbol: t["symbol"].as_str()?.to_string(),
                    price: num(&t["lastPrice"])?,
                    prev: num(&t["openPrice"]).unwrap_or(0.0),
                    vol_usd: num(&t["quoteVolume"]),
                    high: num(&t["highPrice"]),
                    low: num(&t["lowPrice"]),
                    ..Default::default()
                })
            })
            .collect(),
        Exchange::Bybit => {
            code_ok(&v, "retCode", qui)?;
            liste(&v["result"]["list"], qui)?
                .iter()
                .filter_map(|t| {
                    Some(Ligne {
                        base: t["symbol"].as_str()?.strip_suffix("USDT")?.to_string(),
                        symbol: t["symbol"].as_str()?.to_string(),
                        price: num(&t["lastPrice"])?,
                        prev: num(&t["prevPrice24h"]).unwrap_or(0.0),
                        vol_usd: num(&t["turnover24h"]),
                        high: num(&t["highPrice24h"]),
                        low: num(&t["lowPrice24h"]),
                        oi_usd: num(&t["openInterestValue"]),
                        ..Default::default()
                    })
                })
                .collect()
        }
        Exchange::Okx => {
            code_ok(&v, "code", qui)?;
            liste(&v["data"], qui)?
                .iter()
                .filter_map(|t| {
                    let last = num(&t["last"])?;
                    Some(Ligne {
                        base: t["instId"].as_str()?.strip_suffix("-USDT-SWAP")?.to_string(),
                        symbol: t["instId"].as_str()?.to_string(),
                        price: last,
                        prev: num(&t["open24h"]).unwrap_or(0.0),
                        // volCcy24h est en jeton de base : × dernier prix = dollars
                        vol_usd: num(&t["volCcy24h"]).map(|q| q * last),
                        high: num(&t["high24h"]),
                        low: num(&t["low24h"]),
                        ..Default::default()
                    })
                })
                .collect()
        }
        Exchange::Bitunix => {
            code_ok(&v, "code", qui)?;
            liste(&v["data"], qui)?
                .iter()
                .filter_map(|t| {
                    let price = champ(t, &["lastPrice", "last"])?;
                    Some(Ligne {
                        base: t["symbol"].as_str()?.strip_suffix("USDT")?.to_string(),
                        symbol: t["symbol"].as_str()?.to_string(),
                        price,
                        prev: champ(t, &["open", "openPrice"]).unwrap_or(0.0),
                        vol_usd: champ(t, &["quoteVol", "quoteVolume"]).or_else(|| champ(t, &["baseVol", "volume"]).map(|b| b * price)),
                        high: champ(t, &["high", "highPrice"]),
                        low: champ(t, &["low", "lowPrice"]),
                        ..Default::default()
                    })
                })
                .collect()
        }
        Exchange::Weex => {
            let rows = if v.is_array() { &v } else { &v["data"] };
            liste(rows, qui)?
                .iter()
                .filter_map(|t| {
                    let price = num(&t["lastPrice"])?;
                    let prev = num(&t["openPrice"])
                        // (WEEX écrit la variation en fraction : -0.15 = −15 %, vu sur une vraie réponse le 20 nov. 2026)
                        .or_else(|| num(&t["priceChangePercent"]).map(|p| price / (1.0 + p)))
                        .unwrap_or(0.0);
                    Some(Ligne {
                        base: t["symbol"].as_str()?.strip_suffix("USDT")?.to_string(),
                        symbol: t["symbol"].as_str()?.to_string(),
                        price,
                        prev,
                        vol_usd: champ(t, &["quoteVolume", "turnover"]).or_else(|| num(&t["volume"]).map(|b| b * price)),
                        high: num(&t["highPrice"]),
                        low: num(&t["lowPrice"]),
                        ..Default::default()
                    })
                })
                .collect()
        }
        // Bitfinex spot (USD and ₿ BTC pairs) or perpetuals (USDT and BTC): its tickers, volume in coins turned into
        // dollars (× price, × BTC's dollar price for a pair against BTC)
        Exchange::Bitfinex | Exchange::BitfinexSpot => {
            let rows = liste(&v, qui)?;
            let btc_usd = rows.iter().find(|t| t[0] == "tBTCUSD").and_then(|t| num(&t[7]));
            rows.iter()
                .filter_map(|t| {
                    let sym = t[0].as_str()?;
                    let base = crate::exchanges::bitfinex::base_de(sym, ex == Exchange::Bitfinex)?;
                    let price = num(&t[7])?;
                    let usd = if crate::paire::contre_btc(&base) { btc_usd } else { Some(1.0) };
                    Some(Ligne {
                        symbol: sym.trim_start_matches('t').to_string(),
                        base,
                        price,
                        prev: num(&t[5]).map_or(0.0, |c| price - c),
                        vol_usd: num(&t[8]).zip(usd).map(|(v, u)| v * price * u),
                        high: num(&t[9]),
                        low: num(&t[10]),
                        ..Default::default()
                    })
                })
                .collect()
        }
        other => return Err(ArgosError::Feed(format!("{} : pas encore dans « Tous les marchés »", other.name()))),
    };
    Ok(lignes.into_iter().filter(|l| l.price > 0.0 && !l.base.is_empty()).collect())
}

// ───────────────────────── relevé en arrière-plan ─────────────────────────

/// Pause entre deux relevés de la plateforme affichée (secondes).
pub const EVERY_S: u64 = 60;
/// Pause entre deux relevés de la market cap (CoinGecko, secondes).
pub const CAPS_EVERY_S: u64 = 15 * 60;

#[cfg(any(feature = "native", feature = "web"))]
pub use fil::spawn;

#[cfg(any(feature = "native", feature = "web"))]
mod fil {
    use super::{Ligne, CAPS_EVERY_S, EVERY_S};
    use crate::exchanges::Request;
    use crate::http::Http;
    use crate::Exchange;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    /// Après un échec de la market cap, nouvel essai au bout de 2 min.
    const CAPS_RETRY_S: u64 = 2 * 60;

    /// Lance le relevé dans son propre fil : seulement la plateforme de `wanted` (`None` : vue fermée, rien n'est
    /// lu), tout de suite quand elle change, puis toutes les `EVERY_S` secondes ; la market cap (CoinGecko) toutes les
    /// `CAPS_EVERY_S`. Le fil s'arrête quand un rappel renvoie `false`.
    pub fn spawn(
        wanted: Arc<Mutex<Option<Exchange>>>,
        on_lignes: impl Fn(Exchange, Result<Vec<Ligne>, String>) -> bool + Send + 'static,
        on_caps: impl Fn(HashMap<String, f64>) -> bool + Send + 'static,
    ) {
        crate::rt::spawn(move || async move {
            let http = crate::rt::Client::default();
            let (mut last, mut since, mut caps_in): (Option<Exchange>, u64, u64) = (None, 0, 0);
            loop {
                let ex = wanted.lock().ok().and_then(|w| *w);
                if let Some(ex) = ex {
                    if last != Some(ex) || since >= EVERY_S {
                        let body = match ex.markets_request() {
                            Request::Get(url) => http.get(&url).await,
                            Request::PostJson { url, body } => http.post_json(&url, &body).await,
                        };
                        let lignes = body.and_then(|b| super::parse(ex, &b)).map_err(|e| e.to_string());
                        if !on_lignes(ex, lignes) {
                            return;
                        }
                        (last, since) = (Some(ex), 0);
                    }
                    if caps_in == 0 {
                        let mut caps = HashMap::new();
                        for u in super::caps_urls() {
                            if let Ok(b) = http.get(&u).await {
                                let _ = super::parse_caps(&b, &mut caps);
                            }
                        }
                        caps_in = if caps.is_empty() { CAPS_RETRY_S } else { CAPS_EVERY_S };
                        if !caps.is_empty() && !on_caps(caps) {
                            return;
                        }
                    }
                } else {
                    last = None;
                }
                crate::rt::sleep(Duration::from_secs(1)).await;
                since += 1;
                caps_in = caps_in.saturating_sub(1);
            }
        });
    }
}

// ───────────────────────── market cap (CoinGecko) ─────────────────────────

const CG: &str = "https://api.coingecko.com/api/v3";

/// Les 2 pages du top 500 par market cap (une requête toutes les 15 min au plus).
pub fn caps_urls() -> [String; 2] {
    [1, 2].map(|p| format!("{CG}/coins/markets?vs_currency=usd&order=market_cap_desc&per_page=250&page={p}"))
}

/// Symbole (majuscules) → market cap. Un symbole porté par plusieurs jetons : la plus grosse market cap gagne.
pub fn parse_caps(body: &str, caps: &mut HashMap<String, f64>) -> Result<()> {
    let v = json(body, "CoinGecko")?;
    for c in liste(&v, "CoinGecko")? {
        let (Some(s), Some(m)) = (c["symbol"].as_str(), num(&c["market_cap"])) else {
            continue;
        };
        let e = caps.entry(s.to_uppercase()).or_insert(0.0);
        *e = e.max(m);
    }
    Ok(())
}

/// Clé market cap d'un jeton de bourse : « kPEPE » / « 1000PEPE » → PEPE.
pub fn cle_cap(base: &str) -> String {
    let b = base
        .strip_prefix('k')
        .filter(|r| r.chars().next().is_some_and(|c| c.is_ascii_uppercase()))
        .unwrap_or(base);
    let b = b
        .strip_prefix("1000000")
        .or_else(|| b.strip_prefix("1000"))
        .filter(|r| !r.is_empty())
        .unwrap_or(b);
    b.to_uppercase()
}

/// Pose la market cap sur chaque ligne connue de CoinGecko.
pub fn avec_caps(lignes: &mut [Ligne], caps: &HashMap<String, f64>) {
    for l in lignes {
        l.cap = caps.get(&cle_cap(&l.base)).copied();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lit_les_six_bourses() {
        let hl = r#"[{"universe":[{"name":"BTC"},{"name":"OLD","isDelisted":true},{"name":"kPEPE"}]},[{"markPx":"80000","prevDayPx":"79000","dayNtlVlm":"2000000000","openInterest":"30000"},{},{"markPx":"0.01","prevDayPx":"0.011","dayNtlVlm":"5000000","openInterest":"1000"}]]"#;
        let l = parse(Exchange::Hyperliquid, hl).unwrap();
        assert_eq!(l.len(), 2, "marché retiré ignoré");
        assert_eq!((l[0].vol_usd, l[0].oi_usd), (Some(2e9), Some(30000.0 * 80000.0)));
        assert_eq!(l[0].volatilite(), None, "Hyperliquid ne donne ni haut ni bas");

        let bn = r#"[{"symbol":"BTCUSDT","lastPrice":"80000","openPrice":"79000","quoteVolume":"12000000000","highPrice":"81000","lowPrice":"78000"},{"symbol":"BTCUSDC","lastPrice":"1","openPrice":"1"}]"#;
        let b = parse(Exchange::Binance, bn).unwrap();
        assert_eq!(b.len(), 1, "USDT seulement");
        assert!((b[0].volatilite().unwrap() - 3.75).abs() < 1e-9);

        let by = r#"{"retCode":0,"result":{"list":[{"symbol":"SOLUSDT","lastPrice":"100","prevPrice24h":"95","turnover24h":"5e8","highPrice24h":"102","lowPrice24h":"94","openInterestValue":"9e8"}]}}"#;
        let y = parse(Exchange::Bybit, by).unwrap();
        assert_eq!((y[0].vol_usd, y[0].oi_usd), (Some(5e8), Some(9e8)));
        assert!(parse(Exchange::Bybit, r#"{"retCode":10006,"retMsg":"Too many visits"}"#).is_err());

        let ok = r#"{"code":"0","data":[{"instId":"ETH-USDT-SWAP","last":"2000","open24h":"2100","volCcy24h":"1000","high24h":"2150","low24h":"1950"},{"instId":"ETH-USD-SWAP","last":"2000"}]}"#;
        let o = parse(Exchange::Okx, ok).unwrap();
        assert_eq!((o.len(), o[0].vol_usd), (1, Some(2_000_000.0)), "volume en $ = jetons × prix");
        assert_eq!(
            (l[1].symbol.as_str(), b[0].symbol.as_str(), o[0].symbol.as_str()),
            ("kPEPE", "BTCUSDT", "ETH-USDT-SWAP"),
            "symbole du catalogue"
        );

        let bu = r#"{"code":0,"data":[{"symbol":"XRPUSDT","markPrice":"2","lastPrice":"2","open":"1.9","last":"2","quoteVol":"3e7","baseVol":"1.5e7","high":"2.1","low":"1.8"}]}"#;
        assert_eq!(parse(Exchange::Bitunix, bu).unwrap()[0].vol_usd, Some(3e7));

        // noms des champs pris sur de vraies réponses (20 nov. 2026)
        let wx = r#"[{"symbol":"ALABUSDT","priceChange":"-33.10","priceChangePercent":"-0.150708","lastPrice":"186.53","openPrice":"219.63","highPrice":"220.56","lowPrice":"183.06","volume":"4915.99","quoteVolume":"997186.7719","markPrice":"186.56"},{"symbol":"DOGEUSDT","lastPrice":"0.2","priceChangePercent":"-0.2","quoteVolume":"1e7"}]"#;
        let w = parse(Exchange::Weex, wx).unwrap();
        assert_eq!((w[0].prev, w[0].vol_usd, w[0].high), (219.63, Some(997186.7719), Some(220.56)));
        assert!((w[1].prev - 0.25).abs() < 1e-12, "variation en fraction sans openPrice");

        assert!(parse(Exchange::Kraken, "[]").is_err(), "pas encore dans le 1er lot");
    }

    /// ₿ Bitfinex in « Tous les marchés »: its spot (USD and BTC pairs) and its perpetuals (USDT and BTC), volume in
    /// dollars; the other platforms keep their USDT perpetuals only.
    #[test]
    fn bitfinex_spot_perps_et_paires_btc() {
        let t = r#"[["tBTCUSD",1,1,1,1,1000,0.01,60000,10,61000,59000],["tXRPBTC",1,1,1,1,0.0000001,0.004,0.00002,1000,0.000021,0.000019],["tXRPUST",1,1,1,1,0,0,1.2,5,1,1],["tXRPF0:USTF0",1,1,1,1,0.01,0,1.2,100,1.3,1.1],["tETHF0:BTCF0",1,1,1,1,0,0,0.05,2,0.051,0.049]]"#;
        let s = parse(Exchange::BitfinexSpot, t).unwrap();
        assert_eq!(
            s.iter().map(|l| (l.base.as_str(), l.symbol.as_str())).collect::<Vec<_>>(),
            [("BTC", "BTCUSD"), ("XRP/BTC", "XRPBTC")]
        );
        assert_eq!(s[0].vol_usd, Some(600_000.0));
        assert!(
            (s[1].vol_usd.unwrap() - 1000.0 * 0.00002 * 60000.0).abs() < 1e-6,
            "BTC volume × BTC's dollar price"
        );
        assert!((s[1].prev - 0.0000199).abs() < 1e-12);
        let p = parse(Exchange::Bitfinex, t).unwrap();
        assert_eq!(
            p.iter().map(|l| (l.base.as_str(), l.symbol.as_str())).collect::<Vec<_>>(),
            [("XRP", "XRPF0:USTF0"), ("ETH/BTC", "ETHF0:BTCF0")]
        );
        assert!(BOURSES.contains(&Exchange::Bitfinex) && BOURSES.contains(&Exchange::BitfinexSpot));
        let bn = r#"[{"symbol":"ETHBTC","lastPrice":"0.03","openPrice":"0.03"},{"symbol":"ETHUSDT","lastPrice":"2000","openPrice":"2000"}]"#;
        assert_eq!(parse(Exchange::Binance, bn).unwrap().len(), 1, "Binance perps: USDT only, as before");
    }

    fn l(base: &str, vol: Option<f64>, pct: f64, cap: Option<f64>) -> Ligne {
        Ligne {
            base: base.into(),
            price: 100.0 + pct,
            prev: 100.0,
            vol_usd: vol,
            cap,
            ..Default::default()
        }
    }

    #[test]
    fn tris_et_manquants_en_bas() {
        let mut v = vec![
            l("ETH", Some(5.0), -3.0, Some(300.0)),
            l("BTC", Some(9.0), 1.0, Some(1000.0)),
            l("ZZZ", Some(7.0), 8.0, None),
            l("abc", None, 0.0, None),
        ];
        let noms = |v: &[Ligne]| v.iter().map(|x| x.base.clone()).collect::<Vec<_>>().join(",");
        trier(&mut v, Tri::Volume);
        assert_eq!(noms(&v), "BTC,ZZZ,ETH,abc");
        trier(&mut v, Tri::Hausse);
        assert_eq!(noms(&v), "ZZZ,BTC,abc,ETH");
        trier(&mut v, Tri::Baisse);
        assert_eq!(noms(&v), "ETH,abc,BTC,ZZZ");
        trier(&mut v, Tri::MarketCap);
        assert_eq!(noms(&v), "BTC,ETH,ZZZ,abc", "sans market cap : en bas, par volume");
        trier(&mut v, Tri::Nom);
        assert_eq!(noms(&v), "abc,BTC,ETH,ZZZ");
        assert_eq!(Tri::parse("volat"), Tri::Volatilite);
        assert_eq!(Tri::parse("inconnu"), Tri::Volume);
    }

    #[test]
    fn market_cap_coingecko() {
        let mut caps = HashMap::new();
        parse_caps(
            r#"[{"symbol":"btc","market_cap":1.5e12},{"symbol":"pepe","market_cap":4e9},{"symbol":"pepe","market_cap":1e3},{"symbol":"x","market_cap":null}]"#,
            &mut caps,
        )
        .unwrap();
        assert_eq!(caps.len(), 2);
        let mut v = vec![
            l("kPEPE", None, 0.0, None),
            l("1000PEPE", None, 0.0, None),
            l("BTC", None, 0.0, None),
            l("NVDA", None, 0.0, None),
        ];
        avec_caps(&mut v, &caps);
        assert_eq!(v.iter().map(|x| x.cap).collect::<Vec<_>>(), vec![Some(4e9), Some(4e9), Some(1.5e12), None]);
        assert_eq!(cle_cap("1000"), "1000", "jamais vide");
        for u in caps_urls() {
            assert!(crate::net::autorise(&u), "adresse dans la liste blanche");
        }
    }
}
