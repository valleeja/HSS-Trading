//! 📊 Marché crypto (panneau CMC d'Argos v1) : données du jeton ouvert, peur & avidité, capitalisation totale
//! (TOTAL, TOTAL2, TOTAL3), dominances, volume 24 h et saison des altcoins (top 50 contre BTC sur 30 jours).
//! Sources publiques, lecture seule : CoinGecko et alternative.me. Mise à jour aux 60 s (le top 80 : aux 15 min).

use serde_json::Value;

/// Le jeton ouvert (CoinGecko `coins/markets`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Coin {
    pub id: String,
    pub name: String,
    pub symbol: String,
    pub rank: Option<u32>,
    pub price: f64,
    /// Variations en % : 1 h, 24 h, 7 j, 30 j, 1 an.
    pub chg: [Option<f64>; 5],
    pub market_cap: f64,
    pub volume: f64,
    pub fdv: Option<f64>,
    pub low_24h: Option<f64>,
    pub high_24h: Option<f64>,
    pub ath: Option<f64>,
    pub ath_chg: Option<f64>,
}

/// Peur & avidité (alternative.me).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Fng {
    pub value: u32,
    /// En français (« Peur », « Avidité extrême »…).
    pub label: String,
    pub yesterday: Option<u32>,
    pub week_ago: Option<u32>,
}

/// Le marché crypto (CoinGecko `global`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Global {
    pub total: f64,
    pub total_chg_24h: Option<f64>,
    pub btc_d: Option<f64>,
    pub eth_d: Option<f64>,
    pub usdt_d: Option<f64>,
    pub usdc_d: Option<f64>,
    pub volume: f64,
}

impl Global {
    /// TOTAL2 : sans BTC.
    pub fn total2(&self) -> Option<f64> {
        self.btc_d.map(|b| self.total * (1.0 - b / 100.0))
    }
    /// TOTAL3 : sans BTC ni ETH.
    pub fn total3(&self) -> Option<f64> {
        self.btc_d.map(|b| self.total * (1.0 - (b + self.eth_d.unwrap_or(0.0)) / 100.0))
    }
}

/// Le marché depuis un relevé CRYPTOCAP: (CoinMarketCap ou CoinGecko).
#[cfg(test)]
pub fn global_of(p: &crate::cmcap::CapPoint) -> Global {
    Global {
        total: p.total,
        total_chg_24h: p.chg_24h,
        btc_d: Some(p.btc_d),
        eth_d: Some(p.eth_d),
        usdt_d: p.usdt_d,
        usdc_d: p.usdc_d,
        volume: p.volume.unwrap_or(0.0),
    }
}

/// Un relevé CRYPTOCAP: depuis le marché de CoinGecko (gratuit, sans clé) ; `None` sans dominance BTC.
pub fn point_of(g: &Global, t: i64) -> Option<crate::cmcap::CapPoint> {
    Some(crate::cmcap::CapPoint {
        t,
        total: g.total,
        btc_d: g.btc_d?,
        eth_d: g.eth_d.unwrap_or(0.0),
        usdt_d: g.usdt_d,
        usdc_d: g.usdc_d,
        volume: (g.volume > 0.0).then_some(g.volume),
        chg_24h: g.total_chg_24h,
        fng: None,
        alt_season: None,
        caps: Default::default(),
    })
}

/// Tout le panneau.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MarketInfo {
    pub coin: Option<Coin>,
    pub fng: Option<Fng>,
    pub global: Option<Global>,
    /// Saison des altcoins : 0 (Bitcoin) à 100 (altcoins).
    pub alt_season: Option<u32>,
    /// Dernière erreur réseau (affichée en petit).
    pub error: Option<String>,
    /// 🔑 Une clé CoinMarketCap est au trousseau (la clé elle-même ne vient jamais ici).
    pub cmc_cle: bool,
    /// 📈 Dernier relevé CoinMarketCap (avec la clé) : affiché EN PLUS des sources gratuites, jamais à leur place.
    pub cmc: Option<crate::cmcap::CapPoint>,
}

const CG: &str = "https://api.coingecko.com/api/v3";
pub const FNG_URL: &str = "https://api.alternative.me/fng/?limit=8";

pub fn global_url() -> String {
    format!("{CG}/global")
}

pub fn coin_url(symbol: &str) -> String {
    format!(
        "{CG}/coins/markets?vs_currency=usd&symbols={}&price_change_percentage=1h,24h,7d,30d,1y",
        symbol.to_lowercase()
    )
}

pub fn top_url() -> String {
    format!("{CG}/coins/markets?vs_currency=usd&order=market_cap_desc&per_page=80&page=1&price_change_percentage=30d")
}

fn f(v: &Value) -> Option<f64> {
    v.as_f64().or_else(|| v.as_str().and_then(|s| s.parse().ok()))
}

/// Symbole de base d'un symbole d'exchange (« BTC-USDT » → BTC, « kPEPE » → PEPE, « xyz:NVDA » → aucun).
pub fn base_symbol(s: &str) -> Option<String> {
    if s.contains(':') {
        return None;
    }
    let s = s.trim_start_matches('k');
    let b: String = s
        .split(['-', '/', '_'])
        .next()
        .unwrap_or(s)
        .trim_end_matches("USDT")
        .trim_end_matches("USDC")
        .trim_end_matches("PERP")
        .to_uppercase();
    (!b.is_empty() && b.chars().all(|c| c.is_ascii_alphanumeric())).then_some(b)
}

pub fn parse_global(body: &str) -> Option<Global> {
    let v: Value = serde_json::from_str(body).ok()?;
    let d = &v["data"];
    let dom = &d["market_cap_percentage"];
    Some(Global {
        total: f(&d["total_market_cap"]["usd"])?,
        total_chg_24h: f(&d["market_cap_change_percentage_24h_usd"]),
        btc_d: f(&dom["btc"]),
        eth_d: f(&dom["eth"]),
        usdt_d: f(&dom["usdt"]),
        usdc_d: f(&dom["usdc"]),
        volume: f(&d["total_volume"]["usd"]).unwrap_or(0.0),
    })
}

pub fn parse_fng(body: &str) -> Option<Fng> {
    let v: Value = serde_json::from_str(body).ok()?;
    let d = v["data"].as_array()?;
    let val = |i: usize| d.get(i).and_then(|x| f(&x["value"])).map(|x| x as u32);
    let label = match d.first()?["value_classification"].as_str().unwrap_or("") {
        "Extreme Fear" => "Peur extrême",
        "Fear" => "Peur",
        "Neutral" => "Neutre",
        "Greed" => "Avidité",
        "Extreme Greed" => "Avidité extrême",
        other => other,
    }
    .to_string();
    Some(Fng {
        value: val(0)?,
        label,
        yesterday: val(1),
        week_ago: val(7),
    })
}

/// Le jeton `symbol` (le plus gros s'il y en a plusieurs avec ce symbole).
pub fn parse_coin(body: &str, symbol: &str) -> Option<Coin> {
    let v: Value = serde_json::from_str(body).ok()?;
    let c = v
        .as_array()?
        .iter()
        .filter(|x| x["symbol"].as_str().is_some_and(|s| s.eq_ignore_ascii_case(symbol)))
        .max_by(|a, b| f(&a["market_cap"]).unwrap_or(0.0).total_cmp(&f(&b["market_cap"]).unwrap_or(0.0)))?;
    let pc = |k: &str| f(&c[format!("price_change_percentage_{k}_in_currency")]);
    Some(Coin {
        id: c["id"].as_str().unwrap_or("").into(),
        name: c["name"].as_str().unwrap_or("").into(),
        symbol: c["symbol"].as_str().unwrap_or("").to_uppercase(),
        rank: f(&c["market_cap_rank"]).map(|r| r as u32),
        price: f(&c["current_price"])?,
        chg: [
            pc("1h"),
            pc("24h").or_else(|| f(&c["price_change_percentage_24h"])),
            pc("7d"),
            pc("30d"),
            pc("1y"),
        ],
        market_cap: f(&c["market_cap"]).unwrap_or(0.0),
        volume: f(&c["total_volume"]).unwrap_or(0.0),
        fdv: f(&c["fully_diluted_valuation"]),
        low_24h: f(&c["low_24h"]),
        high_24h: f(&c["high_24h"]),
        ath: f(&c["ath"]),
        ath_chg: f(&c["ath_change_percentage"]),
    })
}

/// Saison des altcoins (comme v1) : part du top 50 (hors BTC, stablecoins, jetons emballés / stakés) qui fait mieux
/// que BTC sur 30 jours, sur 100.
pub fn parse_alt_season(body: &str) -> Option<u32> {
    const STABLE: [&str; 14] = [
        "usdt", "usdc", "dai", "fdusd", "usde", "tusd", "pyusd", "usds", "usd1", "busd", "rlusd", "frax", "usdd", "usdp",
    ];
    let v: Value = serde_json::from_str(body).ok()?;
    let all = v.as_array()?;
    let b30 = all
        .iter()
        .find(|x| x["id"] == "bitcoin")
        .and_then(|x| f(&x["price_change_percentage_30d_in_currency"]))?;
    let alts: Vec<f64> = all
        .iter()
        .filter(|x| x["id"] != "bitcoin")
        .filter(|x| {
            let s = x["symbol"].as_str().unwrap_or("").to_lowercase();
            let n = x["name"].as_str().unwrap_or("").to_lowercase();
            !STABLE.contains(&s.as_str()) && !["usd", "stak", "wrapped", "bridged"].iter().any(|w| n.contains(w))
        })
        .take(50)
        .filter_map(|x| f(&x["price_change_percentage_30d_in_currency"]))
        .collect();
    (alts.len() >= 30).then(|| (alts.iter().filter(|c| **c > b30).count() as f64 / alts.len() as f64 * 100.0).round() as u32)
}

#[cfg(any(feature = "native", feature = "web"))]
pub use native::spawn;

#[cfg(any(feature = "native", feature = "web"))]
mod native {
    use super::*;
    use crate::http::Http;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use web_time::Instant;

    /// Lance le rafraîchissement (60 s) dans son propre fil, seulement quand `wanted` a un symbole (panneau ouvert ;
    /// vide = en pause). `on_info` reçoit le panneau à jour ; le fil s'arrête quand il renvoie `false`.
    pub fn spawn(wanted: Arc<Mutex<Option<String>>>, on_info: impl Fn(MarketInfo) -> bool + Send + 'static) {
        crate::rt::spawn(move || async move {
            let http = crate::rt::Client::default();
            let mut info = MarketInfo::default();
            let (mut last, mut last_sym, mut top_at): (Option<Instant>, Option<String>, Option<Instant>) = (None, None, None);
            loop {
                let want = wanted.lock().ok().and_then(|w| w.clone());
                let due = last.is_none_or(|t| t.elapsed() >= Duration::from_secs(60)) || want != last_sym;
                if let (Some(sym), true) = (want.clone(), due) {
                    last = Some(Instant::now());
                    last_sym = Some(sym.clone());
                    info.error = None;
                    match http.get(&global_url()).await {
                        Ok(b) => info.global = parse_global(&b).or(info.global.take()),
                        Err(e) => info.error = Some(format!("CoinGecko : {e}")),
                    }
                    if let Ok(b) = http.get(FNG_URL).await {
                        info.fng = parse_fng(&b).or(info.fng.take());
                    }
                    info.coin = match base_symbol(&sym) {
                        Some(base) => match http.get(&coin_url(&base)).await {
                            Ok(b) => parse_coin(&b, &base),
                            Err(_) => info.coin.take().filter(|c| c.symbol == base),
                        },
                        None => None,
                    };
                    if top_at.is_none_or(|t| t.elapsed() >= Duration::from_secs(900)) {
                        if let Ok(b) = http.get(&top_url()).await {
                            if let Some(a) = parse_alt_season(&b) {
                                info.alt_season = Some(a);
                                top_at = Some(Instant::now());
                            }
                        }
                    }
                    if !on_info(info.clone()) {
                        return;
                    }
                }
                crate::rt::sleep(Duration::from_millis(700)).await;
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // formats vérifiés en direct (CoinGecko et alternative.me, octobre 2026)
    const GLOBAL: &str = r#"{"data":{"total_market_cap":{"usd":2677810088665.9},"total_volume":{"usd":49651814875.2},"market_cap_percentage":{"btc":60.03,"eth":10.57,"usdt":6.80},"market_cap_change_percentage_24h_usd":-4.507}}"#;
    const FNG: &str = r#"{"data":[{"value":"50","value_classification":"Neutral"},{"value":"54","value_classification":"Neutral"},{"value":"40"},{"value":"41"},{"value":"42"},{"value":"43"},{"value":"44"},{"value":"31","value_classification":"Fear"}]}"#;
    const COIN: &str = r#"[{"id":"bitcoin","symbol":"btc","name":"Bitcoin","current_price":80096,"market_cap":1610272363262,"market_cap_rank":1,"fully_diluted_valuation":1610275166718,"total_volume":17950983585,"high_24h":80825,"low_24h":79332,"price_change_percentage_24h":-0.85309,"ath":126080,"ath_change_percentage":-36.47,"price_change_percentage_1h_in_currency":0.6126,"price_change_percentage_1y_in_currency":-28.1,"price_change_percentage_24h_in_currency":-0.853,"price_change_percentage_30d_in_currency":-4.67,"price_change_percentage_7d_in_currency":-2.97},{"id":"batcat","symbol":"btc","name":"Batcat","current_price":0.01,"market_cap":1000}]"#;

    #[test]
    fn lit_les_sources() {
        let g = parse_global(GLOBAL).unwrap();
        assert!((g.total - 2.6778e12).abs() < 1e9 && g.btc_d == Some(60.03));
        assert!((g.total2().unwrap() - g.total * 0.3997).abs() < 1e9);
        assert!(g.total3().unwrap() < g.total2().unwrap());
        let f = parse_fng(FNG).unwrap();
        assert_eq!((f.value, f.label.as_str(), f.yesterday, f.week_ago), (50, "Neutre", Some(54), Some(31)));
        let c = parse_coin(COIN, "BTC").unwrap();
        assert_eq!((c.name.as_str(), c.rank, c.price), ("Bitcoin", Some(1), 80096.0), "le plus gros jeton « btc »");
        assert_eq!(c.chg[0], Some(0.6126));
        assert_eq!(base_symbol("BTC-USDT").as_deref(), Some("BTC"));
        assert_eq!(base_symbol("kPEPE").as_deref(), Some("PEPE"));
        assert_eq!(base_symbol("xyz:NVDA"), None);
        // relevé CRYPTOCAP: gratuit depuis CoinGecko, et retour
        let p = point_of(&g, 600).unwrap();
        assert_eq!((p.btc_d, p.usdt_d, p.volume), (60.03, Some(6.80), Some(g.volume)));
        assert_eq!(global_of(&p).btc_d, g.btc_d);
    }

    #[test]
    fn saison_des_altcoins() {
        let mut top = vec![r#"{"id":"bitcoin","symbol":"btc","name":"Bitcoin","price_change_percentage_30d_in_currency":-5}"#.to_string()];
        for i in 0..40 {
            let c = if i < 30 { -10 } else { 5 }; // 10 sur 40 font mieux que BTC
            top.push(format!(
                r#"{{"id":"a{i}","symbol":"a{i}","name":"Alt {i}","price_change_percentage_30d_in_currency":{c}}}"#
            ));
        }
        top.push(r#"{"id":"tether","symbol":"usdt","name":"Tether","price_change_percentage_30d_in_currency":0}"#.into());
        assert_eq!(parse_alt_season(&format!("[{}]", top.join(","))), Some(25));
    }
}
