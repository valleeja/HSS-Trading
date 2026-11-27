//! 🐋 Top par volume de chaque exchange pour le panneau Baleines : les 30 marchés les plus échangés, lus dans la liste
//! de tickers publique de chaque exchange (sans clé), avec les noms de cet exchange. Hyperliquid y ajoute sa tradfi
//! HIP-3 (or, argent, actions, indices). MEXC et BloFin : la taille de leurs contrats est lue en même temps.
//! Réutilise `tickers::parse_binance` (Binance) et `hyperliquid::parse_dexes` (dex HIP-3).

use serde_json::Value;

/// Marchés écoutés par exchange.
pub const TOP: usize = 30;
/// Jetons stables : jamais dans le top (on suit les vrais marchés).
const STABLES: [&str; 8] = ["USDT", "USDC", "FDUSD", "DAI", "TUSD", "USD1", "USDE", "PYUSD"];

fn f(v: &Value) -> f64 {
    v.as_f64().or_else(|| v.as_str().and_then(|s| s.parse().ok())).unwrap_or(0.0)
}

/// Garde les `TOP` plus gros volumes, sans stables ni doublon (une seule fois par exchange).
pub fn rank(mut rows: Vec<(String, f64)>) -> Vec<String> {
    rows.retain(|(b, v)| !b.is_empty() && *v > 0.0 && !STABLES.contains(&b.as_str()));
    rows.sort_by(|a, b| b.1.total_cmp(&a.1));
    let mut out: Vec<String> = vec![];
    for (b, _) in rows {
        if !out.contains(&b) {
            out.push(b);
        }
        if out.len() == TOP {
            break;
        }
    }
    out
}

/// Liste de tickers d'un exchange (adresse publique).
pub fn url(exchange: &str) -> Option<&'static str> {
    Some(match exchange {
        "Binance" => "https://fapi.binance.com/fapi/v1/ticker/24hr",
        "OKX" => "https://www.okx.com/api/v5/market/tickers?instType=SPOT",
        "Bybit" => "https://api.bybit.com/v5/market/tickers?category=linear",
        "Bitget" => "https://api.bitget.com/api/v2/mix/market/tickers?productType=USDT-FUTURES",
        "Gate.io" => "https://api.gateio.ws/api/v4/spot/tickers",
        "MEXC" => "https://contract.mexc.com/api/v1/contract/ticker",
        "BloFin" => "https://openapi.blofin.com/api/v1/market/tickers",
        "Bitunix" => "https://fapi.bitunix.com/api/v1/futures/market/tickers",
        "KuCoin" => "https://api.kucoin.com/api/v1/market/allTickers",
        "HTX" => "https://api.huobi.pro/market/tickers",
        "Crypto.com" => "https://api.crypto.com/exchange/v1/public/get-tickers",
        "Coinbase" => "https://api.exchange.coinbase.com/products",
        "Kraken" => "https://api.kraken.com/0/public/AssetPairs",
        _ => return None,
    })
}

/// (base, volume en $) d'une liste de tickers. Coinbase et Kraken ne donnent pas de volume : la liste de leurs paires
/// en dollars, rangée ensuite selon le top général (`global`).
pub fn parse(exchange: &str, body: &str, global: &[String]) -> Vec<(String, f64)> {
    let Ok(v) = serde_json::from_str::<Value>(body) else { return vec![] };
    let arr = |x: &Value| x.as_array().cloned().unwrap_or_default();
    let by_global = |b: &str| global.iter().position(|g| g == b).map_or(0.0, |k| (global.len() - k) as f64);
    let strip = |s: &str, q: &str| s.strip_suffix(q).map(str::to_uppercase);
    match exchange {
        "Binance" => crate::tickers::parse_binance(body)
            .unwrap_or_default()
            .into_iter()
            .map(|t| (t.base, t.vol_usd))
            .collect(),
        "OKX" => arr(&v["data"])
            .iter()
            .filter_map(|t| Some((strip(t["instId"].as_str()?, "-USDT")?, f(&t["volCcy24h"]))))
            .collect(),
        "Bybit" => arr(&v["result"]["list"])
            .iter()
            .filter_map(|t| Some((strip(t["symbol"].as_str()?, "USDT")?, f(&t["turnover24h"]))))
            .collect(),
        "Bitget" => arr(&v["data"])
            .iter()
            .filter_map(|t| Some((strip(t["symbol"].as_str()?, "USDT")?, f(&t["usdtVolume"]))))
            .collect(),
        "Gate.io" => arr(&v)
            .iter()
            .filter_map(|t| Some((strip(t["currency_pair"].as_str()?, "_USDT")?, f(&t["quote_volume"]))))
            .collect(),
        "MEXC" => arr(&v["data"])
            .iter()
            .filter_map(|t| Some((strip(t["symbol"].as_str()?, "_USDT")?, f(&t["amount24"]))))
            .collect(),
        "BloFin" => arr(&v["data"])
            .iter()
            .filter_map(|t| Some((strip(t["instId"].as_str()?, "-USDT")?, f(&t["volCurrency24h"]) * f(&t["last"]))))
            .collect(),
        "Bitunix" => arr(&v["data"])
            .iter()
            .filter_map(|t| Some((strip(t["symbol"].as_str()?, "USDT")?, f(&t["quoteVol"]))))
            .collect(),
        "KuCoin" => arr(&v["data"]["ticker"])
            .iter()
            .filter_map(|t| Some((strip(t["symbol"].as_str()?, "-USDT")?, f(&t["volValue"]))))
            .collect(),
        "HTX" => arr(&v["data"])
            .iter()
            .filter_map(|t| Some((strip(t["symbol"].as_str()?, "usdt")?, f(&t["vol"]))))
            .collect(),
        "Crypto.com" => arr(&v["result"]["data"])
            .iter()
            .filter_map(|t| Some((strip(t["i"].as_str()?, "_USDT")?, f(&t["vv"]))))
            .collect(),
        "Coinbase" => arr(&v)
            .iter()
            .filter(|p| p["quote_currency"] == "USD" && p["status"] == "online")
            .filter_map(|p| {
                let b = p["base_currency"].as_str()?.to_uppercase();
                let s = by_global(&b);
                Some((b, s))
            })
            .collect(),
        "Kraken" => v["result"]
            .as_object()
            .map(|o| {
                o.values()
                    .filter_map(|p| {
                        let b = p["wsname"].as_str()?.strip_suffix("/USD")?;
                        let b = match b {
                            "XBT" => "BTC",
                            "XDG" => "DOGE",
                            b => b,
                        }
                        .to_string();
                        let s = by_global(&b);
                        Some((b, s))
                    })
                    .collect()
            })
            .unwrap_or_default(),
        _ => vec![],
    }
}

/// Hyperliquid (crypto ou un dex HIP-3) : (nom, volume du jour en $) de `metaAndAssetCtxs`.
pub fn parse_hl(body: &str) -> Vec<(String, f64)> {
    let Ok(v) = serde_json::from_str::<Value>(body) else { return vec![] };
    let names = v[0]["universe"].as_array().cloned().unwrap_or_default();
    let ctx = v[1].as_array().cloned().unwrap_or_default();
    names
        .iter()
        .zip(ctx.iter())
        .filter(|(u, _)| u["isDelisted"] != true)
        .filter_map(|(u, c)| Some((u["name"].as_str()?.to_string(), f(&c["dayNtlVlm"]))))
        .collect()
}

/// Tailles de contrat (MEXC `contract/detail`, BloFin `instruments`) : (base, taille en pièces).
pub fn parse_contracts(exchange: &str, body: &str) -> Vec<(String, f64)> {
    let Ok(v) = serde_json::from_str::<Value>(body) else { return vec![] };
    // MEXC et BloFin : sous « data » ; Gate.io : la liste elle-même
    let rows = v["data"].as_array().or(v.as_array()).cloned().unwrap_or_default();
    match exchange {
        "MEXC" => rows
            .iter()
            .filter_map(|r| Some((r["symbol"].as_str()?.strip_suffix("_USDT")?.to_uppercase(), f(&r["contractSize"]))))
            .collect(),
        "BloFin" => rows
            .iter()
            .filter_map(|r| Some((r["instId"].as_str()?.strip_suffix("-USDT")?.to_uppercase(), f(&r["contractValue"]))))
            .collect(),
        "Gate.io" => rows
            .iter()
            .filter_map(|r| Some((r["name"].as_str()?.strip_suffix("_USDT")?.to_uppercase(), f(&r["quanto_multiplier"]))))
            .collect(),
        _ => vec![],
    }
    .into_iter()
    .filter(|(_, k)| *k > 0.0)
    .collect()
}

/// Top d'un exchange (et, pour Hyperliquid, sa tradfi HIP-3 en plus). `global` : le top de Binance, qui range Coinbase
/// et Kraken. Les tailles de contrat lues ici sont gardées pour lire les transactions (`contracts::set_contracts`).
#[cfg(any(feature = "native", feature = "web"))]
pub async fn top(exchange: &str, global: &[String]) -> argos_core::Result<Vec<String>> {
    use crate::http::Http;
    let http = crate::rt::Client::default();
    if exchange == "Hyperliquid" {
        let info = "https://api.hyperliquid.xyz/info";
        let crypto = http.post_json(info, r#"{"type":"metaAndAssetCtxs"}"#).await.map_err(|e| e.to_string())?;
        let mut out = rank(parse_hl(&crypto));
        // 🏛️ tradfi HIP-3 : les dex (xyz, …) et leurs marchés les plus échangés
        let dexes = http.post_json(info, r#"{"type":"perpDexs"}"#).await.ok();
        let dexes = dexes.and_then(|b| crate::exchanges::hyperliquid::parse_dexes(&b).ok()).unwrap_or_default();
        let mut tradfi = vec![];
        for d in dexes.iter().take(8) {
            if let Ok(b) = http.post_json(info, &format!(r#"{{"type":"metaAndAssetCtxs","dex":"{d}"}}"#)).await {
                tradfi.extend(parse_hl(&b));
            }
        }
        out.extend(rank(tradfi));
        return Ok(out);
    }
    let u = url(exchange).ok_or("exchange inconnu")?;
    let body = http.get(u).await.map_err(|e| e.to_string())?;
    let list = rank(parse(exchange, &body, global));
    if list.is_empty() {
        return Err(format!("{exchange} : liste de tickers vide").into());
    }
    // les tailles de contrat, si ses Baleines en lisent : sans elles, ses transactions ne se lisent pas, mais le top
    // reste bon
    if crate::whales::baleines_en_contrats(exchange) {
        let _ = charger_contrats(exchange).await;
    }
    Ok(list)
}

/// Lit et garde (`contracts::set_contracts`) les tailles de contrat de MEXC, BloFin ou Gate.io (perps ; rien à lire
/// ailleurs) : pour lire leurs transactions (🐋 Baleines, 🧮 orderflow réel).
#[cfg(any(feature = "native", feature = "web"))]
pub async fn charger_contrats(exchange: &str) -> Result<(), String> {
    use crate::http::Http;
    let url = match exchange {
        "MEXC" => "https://contract.mexc.com/api/v1/contract/detail",
        "BloFin" => "https://openapi.blofin.com/api/v1/market/instruments",
        "Gate.io" => "https://api.gateio.ws/api/v4/futures/usdt/contracts",
        _ => return Ok(()),
    };
    let body = crate::rt::Client::default().get(url).await.map_err(|e| e.to_string())?;
    let rows = parse_contracts(exchange, &body);
    if rows.is_empty() {
        return Err(format!("{exchange} : liste des contrats vide"));
    }
    crate::contracts::set_contracts(exchange, rows);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn top_par_exchange_sans_doublon_ni_stables() {
        let rows = vec![
            ("BTC".into(), 9.0),
            ("USDC".into(), 99.0),
            ("ETH".into(), 5.0),
            ("BTC".into(), 1.0),
            ("SOL".into(), 0.0),
        ];
        assert_eq!(
            rank(rows),
            vec!["BTC".to_string(), "ETH".to_string()],
            "stables, doublons et volume nul retirés"
        );
        let many: Vec<(String, f64)> = (0..50).map(|k| (format!("C{k}"), 100.0 - k as f64)).collect();
        assert_eq!(rank(many).len(), TOP);
        // formats relevés en direct le 2026-11-05 (raccourcis)
        let g = vec!["BTC".to_string(), "ETH".to_string()];
        assert_eq!(
            rank(parse(
                "Bybit",
                r#"{"result":{"list":[{"symbol":"ETHUSDT","turnover24h":"5"},{"symbol":"BTCUSDT","turnover24h":"9"},{"symbol":"BTCPERP","turnover24h":"99"}]}}"#,
                &g
            )),
            ["BTC", "ETH"]
        );
        assert_eq!(
            rank(parse("HTX", r#"{"data":[{"symbol":"btcusdt","vol":9},{"symbol":"ethbtc","vol":99}]}"#, &g)),
            ["BTC"]
        );
        assert_eq!(
            rank(parse(
                "Coinbase",
                r#"[{"base_currency":"ETH","quote_currency":"USD","status":"online"},{"base_currency":"BTC","quote_currency":"USD","status":"online"},{"base_currency":"BTC","quote_currency":"EUR","status":"online"}]"#,
                &g
            )),
            ["BTC", "ETH"],
            "rangés selon le top général"
        );
        assert_eq!(
            rank(parse(
                "Kraken",
                r#"{"result":{"XXBTZUSD":{"wsname":"XBT/USD"},"XETHZEUR":{"wsname":"ETH/EUR"}}}"#,
                &g
            )),
            ["BTC"],
            "XBT = BTC"
        );
        let hl = r#"[{"universe":[{"name":"xyz:GOLD"},{"name":"xyz:OLD","isDelisted":true}]},[{"dayNtlVlm":"5000"},{"dayNtlVlm":"9"}]]"#;
        assert_eq!(rank(parse_hl(hl)), ["xyz:GOLD"]);
        assert_eq!(
            parse_contracts("MEXC", r#"{"data":[{"symbol":"PEPE_USDT","contractSize":1000}]}"#),
            vec![("PEPE".to_string(), 1000.0)]
        );
    }
}
