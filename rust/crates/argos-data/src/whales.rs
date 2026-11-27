//! 🐋 Baleines : les **grosses transactions en direct**, de sources publiques sans clé :
//! - Hyperliquid : chaque trade, avec son côté (achat / vente au marché) et le portefeuille qui l'a passé ;
//! - Binance Futures : les « aggTrades » (trades groupés d'un même ordre).
//!
//! 👁 Suivre une baleine : ses positions ouvertes sur Hyperliquid, lues avec son **adresse publique** seulement.
//!
//! Lecture seule : Argos écoute les flux publics, ne se connecte à aucun compte.

use crate::contracts::contract;
use crate::live::LiveSub;
use serde_json::Value;

/// Pièces suivies en plus de celle du graphique.
pub const COINS: [&str; 4] = ["BTC", "ETH", "SOL", "XRP"];
/// Plancher (dollars) : en dessous, le trade n'est pas gardé (le panneau filtre ensuite selon son seuil).
pub const FLOOR: f64 = 50_000.0;

/// Une grosse transaction.
#[derive(Debug, Clone, PartialEq)]
pub struct Whale {
    pub coin: String,
    /// Moment, en millisecondes UTC.
    pub time_ms: i64,
    pub price: f64,
    pub size: f64,
    /// `true` : achat au marché (le preneur achète) ; `false` : vente au marché.
    pub buy: bool,
    pub src: &'static str,
    /// Portefeuille du preneur (Hyperliquid seulement, vide sinon).
    pub wallet: String,
    /// Numéro du trade donné par l'exchange, quand il est **toujours croissant** pour un marché (Binance : `a` de
    /// l'aggTrade, OKX, Bitget, BloFin et KuCoin : `tradeId`, Bitstamp : `id`, MEXC : `i`, Coinbase et Kraken :
    /// `trade_id`) : sert à ne jamais compter deux fois un trade (orderflow réel). `None` sinon.
    pub id: Option<u64>,
}

impl Whale {
    /// Montant en dollars.
    pub fn usd(&self) -> f64 {
        self.price * self.size
    }
}

/// Pièces écoutées : `COINS` plus `extra` (la pièce du graphique), sans doublon.
pub fn coins(extra: &str) -> Vec<String> {
    let mut l: Vec<String> = COINS.iter().map(|c| c.to_string()).collect();
    let e = extra.trim().to_uppercase();
    if !e.is_empty() && !l.contains(&e) {
        l.push(e);
    }
    l
}

/// Pièce d'un symbole d'exchange : « BTC » pour BTC, BTCUSDT, BTC-USDT-SWAP, BTC/USD…
pub fn coin_of(base: &str) -> String {
    let b = base.split(['-', '/', '_', ':']).next().unwrap_or("").to_uppercase();
    let c = ["USDT", "USDC", "USD"]
        .iter()
        .find_map(|q| b.strip_suffix(q).filter(|c| !c.is_empty()))
        .unwrap_or(&b);
    c.to_string()
}

/// Flux Hyperliquid : un abonnement « trades » par pièce (un message par ligne).
pub fn hyperliquid(coins: &[String]) -> LiveSub {
    LiveSub {
        url: "wss://api.hyperliquid.xyz/ws".into(),
        subscribe: coins
            .iter()
            .map(|c| format!(r#"{{"method":"subscribe","subscription":{{"type":"trades","coin":"{c}"}}}}"#))
            .collect::<Vec<_>>()
            .join("\n"),
        ping: r#"{"method":"ping"}"#.into(),
        ..Default::default()
    }
}

/// Flux Binance Futures : les aggTrades de chaque pièce en un abonnement (Binance pingue lui-même).
pub fn binance(coins: &[String]) -> LiveSub {
    aggtrades("wss://fstream.binance.com/market/ws", coins)
}

/// Les aggTrades (format Binance Futures, aussi celui d'Aster) de chaque pièce, sur le flux `url`, en un abonnement.
pub fn aggtrades(url: &str, coins: &[String]) -> LiveSub {
    let params: Vec<String> = coins.iter().map(|c| format!(r#""{}usdt@aggTrade""#, c.to_lowercase())).collect();
    LiveSub {
        url: url.into(),
        subscribe: format!(r#"{{"method":"SUBSCRIBE","params":[{}],"id":7}}"#, params.join(",")),
        ping: String::new(),
        ..Default::default()
    }
}

/// 🏦 Exchanges des grosses transactions (flux publics sans clé, testés en direct le 2026-11-05) : nom affiché.
/// Les 5 premiers (les plus gros) sont écoutés par défaut ; les autres se cochent dans le panneau.
/// KuCoin demande d'abord un jeton public (sans clé), HTX envoie ses messages en gzip, Crypto.com veut une réponse à
/// ses battements de cœur : tout ça est géré. Pas de flux public utilisable : WEEX (le serveur ferme la connexion).
pub const EXCHANGES: [&str; 14] = [
    "Binance",
    "OKX",
    "Bybit",
    "Bitget",
    "Hyperliquid",
    "Coinbase",
    "Kraken",
    "Gate.io",
    "MEXC",
    "BloFin",
    "Bitunix",
    "KuCoin",
    "HTX",
    "Crypto.com",
];
/// Exchanges écoutés par défaut.
pub const DEFAULT_ON: usize = 5;

/// Exchanges dont les trades de perps sont en contrats (convertis en pièces par `contract`) ; Gate.io : ses perps de
/// l'orderflow réel (ses Baleines écoutent le spot, en pièces).
const EN_CONTRATS: [&str; 3] = ["MEXC", "BloFin", "Gate.io"];

/// La taille des trades de `coin` sur `exchange` se lit-elle en pièces ? Toujours, sauf sur un exchange à contrats
/// tant que la taille du contrat de cette pièce n'est pas connue (`whaletop::charger_contrats`).
pub fn taille_lisible(exchange: &str, coin: &str) -> bool {
    !EN_CONTRATS.contains(&exchange) || contract(exchange, coin).is_some()
}

/// Les 🐋 Baleines de cet exchange lisent-elles des contrats ? MEXC et BloFin oui ; Gate.io non (ses Baleines écoutent
/// le spot : seule l'orderflow réel lit ses contrats).
pub fn baleines_en_contrats(exchange: &str) -> bool {
    EN_CONTRATS.contains(&exchange) && exchange != "Gate.io"
}

/// Pièces d'un exchange à contrats : seulement celles dont on connaît la taille du contrat.
fn known(exchange: &str, coins: &[String]) -> Vec<String> {
    coins.iter().filter(|c| contract(exchange, c).is_some()).cloned().collect()
}

/// Millisecondes d'une date ISO 8601 (« 2026-11-05T20:07:08Z »).
pub(crate) fn iso_ms(s: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(s).ok().map(|d| d.timestamp_millis())
}

/// Flux OKX : trades spot de chaque pièce (taille en pièces ; OKX attend « ping » en texte).
pub fn okx(coins: &[String]) -> LiveSub {
    okx_trades(coins, "-USDT")
}

/// Flux OKX des trades de chaque pièce, sur le marché `instId` = pièce + `suffixe` (« -USDT » : spot, taille en
/// pièces ; « -USDT-SWAP » : perps, taille en contrats) ; une seule connexion.
pub fn okx_trades(coins: &[String], suffixe: &str) -> LiveSub {
    let args: Vec<String> = coins.iter().map(|c| format!(r#"{{"channel":"trades","instId":"{c}{suffixe}"}}"#)).collect();
    LiveSub {
        url: "wss://ws.okx.com:8443/ws/v5/public".into(),
        subscribe: format!(r#"{{"op":"subscribe","args":[{}]}}"#, args.join(",")),
        ping: "ping".into(),
        ..Default::default()
    }
}

/// Flux Bybit : trades des perps USDT (taille en pièces).
pub fn bybit(coins: &[String]) -> LiveSub {
    bybit_trades("wss://stream.bybit.com/v5/public/linear", coins)
}

/// Flux Bybit des trades de chaque pièce sur le marché `url` (« …/linear » : perps ; « …/spot » : spot).
pub fn bybit_trades(url: &str, coins: &[String]) -> LiveSub {
    let args: Vec<String> = coins.iter().map(|c| format!(r#""publicTrade.{c}USDT""#)).collect();
    // Bybit : 10 marchés au plus par demande d'abonnement
    LiveSub {
        url: url.into(),
        subscribe: args
            .chunks(10)
            .map(|a| format!(r#"{{"op":"subscribe","args":[{}]}}"#, a.join(",")))
            .collect::<Vec<_>>()
            .join("\n"),
        ping: r#"{"op":"ping"}"#.into(),
        ..Default::default()
    }
}

/// Flux Bitget : trades des perps USDT (taille en pièces ; Bitget attend « ping » en texte).
pub fn bitget(coins: &[String]) -> LiveSub {
    bitget_trades("USDT-FUTURES", coins)
}

/// Flux Bitget des trades de chaque pièce sur le marché `inst_type` (« USDT-FUTURES » : perps ; « SPOT » : spot).
pub fn bitget_trades(inst_type: &str, coins: &[String]) -> LiveSub {
    let args: Vec<String> = coins
        .iter()
        .map(|c| format!(r#"{{"instType":"{inst_type}","channel":"trade","instId":"{c}USDT"}}"#))
        .collect();
    LiveSub {
        url: "wss://ws.bitget.com/v2/ws/public".into(),
        subscribe: format!(r#"{{"op":"subscribe","args":[{}]}}"#, args.join(",")),
        ping: "ping".into(),
        ..Default::default()
    }
}

/// Flux d'un exchange (nom de `EXCHANGES`).
pub fn sub_of(exchange: &str, coins: &[String]) -> Option<LiveSub> {
    Some(match exchange {
        "Binance" => binance(coins),
        "OKX" => okx(coins),
        "Bybit" => bybit(coins),
        "Bitget" => bitget(coins),
        "Hyperliquid" => hyperliquid(coins),
        // Coinbase : paires en dollars ; « matches » = chaque transaction (le même flux que ses bougies)
        "Coinbase" => crate::exchanges::coinbase::trades_sub(coins),
        "Kraken" => LiveSub {
            url: "wss://ws.kraken.com/v2".into(),
            subscribe: format!(
                r#"{{"method":"subscribe","params":{{"channel":"trade","symbol":[{}]}}}}"#,
                coins.iter().map(|c| format!(r#""{c}/USD""#)).collect::<Vec<_>>().join(",")
            ),
            ping: r#"{"method":"ping"}"#.into(),
            ..Default::default()
        },
        "Gate.io" => LiveSub {
            url: "wss://api.gateio.ws/ws/v4/".into(),
            subscribe: format!(
                r#"{{"time":0,"channel":"spot.trades","event":"subscribe","payload":[{}]}}"#,
                coins.iter().map(|c| format!(r#""{c}_USDT""#)).collect::<Vec<_>>().join(",")
            ),
            ping: r#"{"time":0,"channel":"spot.ping"}"#.into(),
            ..Default::default()
        },
        "MEXC" => LiveSub {
            url: "wss://contract.mexc.com/edge".into(),
            subscribe: known("MEXC", coins)
                .iter()
                .map(|c| format!(r#"{{"method":"sub.deal","param":{{"symbol":"{c}_USDT"}}}}"#))
                .collect::<Vec<_>>()
                .join("\n"),
            ping: r#"{"method":"ping"}"#.into(),
            ..Default::default()
        },
        "BloFin" => LiveSub {
            url: "wss://openapi.blofin.com/ws/public".into(),
            subscribe: format!(
                r#"{{"op":"subscribe","args":[{}]}}"#,
                known("BloFin", coins)
                    .iter()
                    .map(|c| format!(r#"{{"channel":"trades","instId":"{c}-USDT"}}"#))
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            ping: "ping".into(),
            ..Default::default()
        },
        // HTX : un abonnement par pièce, messages en gzip, répondre « pong » à chaque « ping »
        "HTX" => LiveSub {
            url: "wss://api.huobi.pro/ws".into(),
            subscribe: coins
                .iter()
                .map(|c| format!(r#"{{"sub":"market.{}usdt.trade.detail","id":"{c}"}}"#, c.to_lowercase()))
                .collect::<Vec<_>>()
                .join("\n"),
            gzip: true,
            reply: crate::live::Reply::Htx,
            ..Default::default()
        },
        // Crypto.com : répondre à chaque battement de cœur, sinon il coupe
        "Crypto.com" => LiveSub {
            url: "wss://stream.crypto.com/exchange/v1/market".into(),
            subscribe: format!(
                r#"{{"id":1,"method":"subscribe","params":{{"channels":[{}]}}}}"#,
                coins.iter().map(|c| format!(r#""trade.{c}_USDT""#)).collect::<Vec<_>>().join(",")
            ),
            reply: crate::live::Reply::CryptoCom,
            ..Default::default()
        },
        // KuCoin : l'adresse vient d'un jeton public demandé juste avant (`kucoin_preparer`, à chaque connexion)
        "KuCoin" => return None,
        "Bitunix" => LiveSub {
            url: "wss://fapi.bitunix.com/public/".into(),
            subscribe: format!(
                r#"{{"op":"subscribe","args":[{}]}}"#,
                coins
                    .iter()
                    .map(|c| format!(r#"{{"symbol":"{c}USDT","ch":"trade"}}"#))
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            ping: r#"{"op":"ping","ping":0}"#.into(),
            // 💤 nothing comes back to this ping (`LiveSub::ping_unanswered`)
            ping_unanswered: true,
            ..Default::default()
        },
        _ => return None,
    })
}

/// KuCoin : abonnement aux transactions, une fois l'adresse (avec son jeton) connue.
pub fn kucoin_sub_at(url: &str, coins: &[String]) -> LiveSub {
    LiveSub {
        url: url.to_string(),
        subscribe: format!(
            r#"{{"id":"1","type":"subscribe","topic":"/market/match:{}","response":true}}"#,
            // (₿ the chart of a BTC pair « XRP/BTC »: XRP-BTC)
            coins.iter().map(|c| crate::paire::symbole(c, "-", "USDT")).collect::<Vec<_>>().join(",")
        ),
        ping: r#"{"id":"argos","type":"ping"}"#.into(),
        reply: crate::live::Reply::KuCoin,
        ..Default::default()
    }
}

/// KuCoin : l'adresse du flux et son jeton, dans la réponse de `POST /api/v1/bullet-public` (public, sans clé).
pub fn parse_kucoin_bullet(body: &str) -> Option<String> {
    let v: Value = serde_json::from_str(body).ok()?;
    let d = &v["data"];
    let endpoint = d["instanceServers"][0]["endpoint"].as_str()?;
    let token = d["token"].as_str()?;
    Some(format!("{endpoint}?token={token}&connectId=argos"))
}

/// KuCoin : demande un jeton public neuf, puis l'abonnement prêt à ouvrir (`live::spawn_jeton` l'appelle à chaque
/// connexion : un jeton expire).
#[cfg(any(feature = "native", feature = "web"))]
pub async fn kucoin_preparer(coins: Vec<String>) -> Result<LiveSub, String> {
    use crate::http::Http;
    let body = crate::rt::Client::default()
        .post_json("https://api.kucoin.com/api/v1/bullet-public", "{}")
        .await
        .map_err(|e| e.to_string())?;
    let url = parse_kucoin_bullet(&body).ok_or("jeton KuCoin illisible")?;
    Ok(kucoin_sub_at(&url, &coins))
}

/// Grosses transactions d'un message (Hyperliquid ou Binance) : seulement celles d'au moins `FLOOR` dollars.
pub fn parse(msg: &str) -> Vec<Whale> {
    parse_all(msg).into_iter().filter(|w| w.usd() >= FLOOR).collect()
}

/// Every trade of a message, small ones included (the ⚡ Trades module and the order flow), whatever the exchange. A
/// trade that cannot be (a price or a size that is not a number above zero, `feed::price_size_ok`) is left out: one
/// rule, at the one way out of every decoder.
pub fn parse_all(msg: &str) -> Vec<Whale> {
    let Ok(v) = serde_json::from_str::<Value>(msg) else { return vec![] };
    let num = |t: &Value, k: &str| t[k].as_str().and_then(|s| s.parse::<f64>().ok());
    let mut list: Vec<Whale> = if v["channel"] == "trades" {
        v["data"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|t| {
                let buy = t["side"] == "B";
                // users = [acheteur, vendeur] : le preneur est l'acheteur si achat au marché
                let wallet = t["users"][if buy { 0 } else { 1 }].as_str().unwrap_or("").to_string();
                Some(Whale {
                    coin: t["coin"].as_str()?.to_string(),
                    time_ms: t["time"].as_i64()?,
                    price: num(t, "px")?,
                    size: num(t, "sz")?,
                    buy,
                    src: "Hyperliquid",
                    id: None,
                    wallet,
                })
            })
            .collect()
    } else if (v["e"] == "trade" || v["e"] == "tradeSnapshot") && v["d"].is_array() {
        // WEEX (perps) : son décodeur
        crate::exchanges::weex::trades(&v)
    } else if v["e"] == "aggTrade" {
        // Binance Futures, et Aster (même format : marqué « Binance », l'orderflow range par la bourse de son flux)
        (|| {
            let coin = v["s"].as_str()?.strip_suffix("USDT")?.to_string();
            // m = l'acheteur a fourni la liquidité : le preneur a vendu
            Some(Whale {
                coin,
                time_ms: v["T"].as_i64()?,
                price: num(&v, "p")?,
                size: num(&v, "q")?,
                buy: v["m"] == false,
                src: "Binance",
                wallet: String::new(),
                id: v["a"].as_u64(),
            })
        })()
        .into_iter()
        .collect()
    } else if v["arg"]["channel"] == "trades" && v["data"][0]["price"].is_string() {
        // BloFin : comme OKX, mais « price » / « size » (en contrats, convertis en pièces) ; side : côté du preneur ;
        // tradeId : toujours croissant
        let coin = coin_of(v["arg"]["instId"].as_str().unwrap_or(""));
        let k = contract("BloFin", &coin).unwrap_or(0.0);
        v["data"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|t| {
                Some(Whale {
                    coin: coin.clone(),
                    time_ms: t["ts"].as_str()?.parse().ok()?,
                    price: num(t, "price")?,
                    size: num(t, "size")? * k,
                    buy: t["side"] == "buy",
                    src: "BloFin",
                    id: t["tradeId"].as_str().and_then(|s| s.parse().ok()),
                    wallet: String::new(),
                })
            })
            .collect()
    } else if v["arg"]["channel"] == "trades" {
        // OKX : {"arg":{"channel":"trades",…},"data":[{"instId":"BTC-USDT","tradeId":"…","px","sz","side":"buy","ts":"…"}]}
        // (side : côté du preneur ; tradeId : toujours croissant pour un marché)
        v["data"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|t| {
                Some(Whale {
                    coin: coin_of(t["instId"].as_str()?),
                    time_ms: t["ts"].as_str()?.parse().ok()?,
                    price: num(t, "px")?,
                    size: num(t, "sz")?,
                    buy: t["side"] == "buy",
                    src: "OKX",
                    id: t["tradeId"].as_str().and_then(|s| s.parse().ok()),
                    wallet: String::new(),
                })
            })
            .collect()
    } else if v["topic"].as_str().is_some_and(|t| t.starts_with("publicTrade.")) {
        // Bybit : {"topic":"publicTrade.BTCUSDT","data":[{"T":ms,"s":"BTCUSDT","S":"Buy","v","p"}]}
        v["data"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|t| {
                Some(Whale {
                    coin: coin_of(t["s"].as_str()?),
                    time_ms: t["T"].as_i64()?,
                    price: num(t, "p")?,
                    size: num(t, "v")?,
                    buy: t["S"] == "Buy",
                    src: "Bybit",
                    // i : un numéro toujours croissant sur le spot ; un texte (UUID) sur les perps, ignoré
                    id: t["i"].as_str().and_then(|s| s.parse().ok()),
                    wallet: String::new(),
                })
            })
            .collect()
    } else if v["arg"]["channel"] == "trade" && (v["arg"]["instType"] == "USDT-FUTURES" || v["arg"]["instType"] == "SPOT") {
        // Bitget : {"arg":{"instType":"USDT-FUTURES","channel":"trade","instId":"BTCUSDT"},"data":[{"ts","price","size","side",
        // "tradeId"}]} (side : côté du preneur ; tradeId : toujours croissant, mais le lot va du plus récent au plus ancien)
        let coin = coin_of(v["arg"]["instId"].as_str().unwrap_or(""));
        v["data"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|t| {
                Some(Whale {
                    coin: coin.clone(),
                    time_ms: t["ts"].as_str()?.parse().ok()?,
                    price: num(t, "price")?,
                    size: num(t, "size")?,
                    buy: t["side"] == "buy",
                    src: "Bitget",
                    id: t["tradeId"].as_str().and_then(|s| s.parse().ok()),
                    wallet: String::new(),
                })
            })
            .filter(|w| !w.coin.is_empty())
            .collect()
    } else if v["type"] == "match" || v["type"] == "last_match" {
        // Coinbase : son décodeur, partagé avec ses bougies
        crate::exchanges::coinbase::trade(&v).into_iter().collect()
    } else if v["channel"] == "trade" && v["data"].is_array() && v["data"][0]["qty"].is_number() {
        // Kraken v2 : {"channel":"trade","data":[{"symbol":"BTC/USD","side","price","qty","timestamp"}]}
        // (side : côté du preneur ; trade_id : toujours croissant)
        v["data"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|t| {
                Some(Whale {
                    coin: coin_of(t["symbol"].as_str()?),
                    time_ms: iso_ms(t["timestamp"].as_str()?)?,
                    price: t["price"].as_f64()?,
                    size: t["qty"].as_f64()?,
                    buy: t["side"] == "buy",
                    src: "Kraken",
                    id: t["trade_id"].as_u64(),
                    wallet: String::new(),
                })
            })
            .collect()
    } else if v["channel"] == "futures.trades" {
        // Gate.io (perps) : son décodeur
        crate::exchanges::gate::trades(&v)
    } else if v["channel"] == "spot.trades" && v["event"] == "update" {
        // Gate.io : {"channel":"spot.trades","result":{"side","amount","price","currency_pair","create_time_ms":"….920000"}}
        let r = &v["result"];
        (|| {
            Some(Whale {
                coin: coin_of(r["currency_pair"].as_str()?),
                time_ms: r["create_time_ms"].as_str()?.split('.').next()?.parse().ok()?,
                price: num(r, "price")?,
                size: num(r, "amount")?,
                buy: r["side"] == "buy",
                src: "Gate.io",
                id: None,
                wallet: String::new(),
            })
        })()
        .into_iter()
        .collect()
    } else if v["channel"] == "push.deal" {
        // MEXC (perps) : taille en contrats ; T = 1 achat preneur, 2 vente preneur ; i : numéro toujours croissant (un lot va
        // du plus récent au plus ancien : l'agrégateur les range)
        let coin = coin_of(v["symbol"].as_str().unwrap_or(""));
        let k = contract("MEXC", &coin).unwrap_or(0.0);
        let rows: Vec<&Value> = match &v["data"] {
            Value::Array(a) => a.iter().collect(),
            d => vec![d],
        };
        rows.into_iter()
            .filter_map(|t| {
                Some(Whale {
                    coin: coin.clone(),
                    time_ms: t["t"].as_i64()?,
                    price: t["p"].as_f64()?,
                    size: t["v"].as_f64()? * k,
                    buy: t["T"] == 1,
                    src: "MEXC",
                    id: t["i"].as_str().and_then(|s| s.parse().ok()),
                    wallet: String::new(),
                })
            })
            .collect()
    } else if v["event"] == "trade" && v["channel"].as_str().is_some_and(|c| c.starts_with("live_trades_")) {
        // Bitstamp : son décodeur, partagé avec ses bougies
        crate::exchanges::bitstamp::trade(&v).into_iter().collect()
    } else if v["type"] == "message" && v["subject"] == "trade.l3match" {
        // KuCoin : son décodeur, partagé avec ses bougies
        crate::exchanges::kucoin::trade(&v).into_iter().collect()
    } else if v["ch"].as_str().is_some_and(|c| c.starts_with("market.") && c.ends_with(".trade.detail")) {
        // HTX : {"ch":"market.btcusdt.trade.detail","tick":{"data":[{"amount","price","direction":"buy","ts"}]}}
        let ch = v["ch"].as_str().unwrap_or("");
        let coin = ch
            .trim_start_matches("market.")
            .split('.')
            .next()
            .unwrap_or("")
            .trim_end_matches("usdt")
            .to_uppercase();
        v["tick"]["data"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|t| {
                Some(Whale {
                    coin: coin.clone(),
                    time_ms: t["ts"].as_i64()?,
                    price: t["price"].as_f64()?,
                    size: t["amount"].as_f64()?,
                    buy: t["direction"] == "buy",
                    src: "HTX",
                    id: None,
                    wallet: String::new(),
                })
            })
            .collect()
    } else if v["result"]["channel"] == "trade" {
        // Crypto.com : {"result":{"channel":"trade","data":[{"t","p","q","s":"BUY","i":"BTC_USDT"}]}}
        v["result"]["data"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|t| {
                Some(Whale {
                    coin: coin_of(t["i"].as_str()?),
                    time_ms: t["t"].as_i64()?,
                    price: num(t, "p")?,
                    size: num(t, "q")?,
                    buy: t["s"] == "BUY",
                    src: "Crypto.com",
                    id: None,
                    wallet: String::new(),
                })
            })
            .collect()
    } else if v["ch"] == "trade" && v["symbol"].is_string() {
        // Bitunix : {"ch":"trade","symbol":"BTCUSDT","data":[{"t":"…Z","p","v","s":"buy"}]} (s : côté du preneur,
        // relevé avec le carnet le 2026-11-15 ; pas de numéro de trade ; t à la seconde)
        let coin = coin_of(v["symbol"].as_str().unwrap_or(""));
        v["data"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|t| {
                Some(Whale {
                    coin: coin.clone(),
                    time_ms: iso_ms(t["t"].as_str()?).or_else(|| v["ts"].as_i64())?,
                    price: num(t, "p")?,
                    size: num(t, "v")?,
                    buy: t["s"] == "buy",
                    src: "Bitunix",
                    id: None,
                    wallet: String::new(),
                })
            })
            .collect()
    } else {
        vec![]
    };
    list.retain(|w| argos_core::feed::price_size_ok(w.price, w.size));
    list
}

/// Une position ouverte d'un portefeuille Hyperliquid.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Position {
    pub coin: String,
    /// Quantité : positive = long, négative = short.
    pub size: f64,
    pub entry: f64,
    /// Valeur en dollars.
    pub value: f64,
    /// Gain / perte latent (dollars).
    pub pnl: f64,
    pub leverage: f64,
    /// Prix de liquidation (0 : aucun).
    pub liq: f64,
}

/// Compte d'un portefeuille Hyperliquid.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Account {
    /// Valeur du compte (dollars).
    pub value: f64,
    /// Positions, la plus grosse d'abord.
    pub positions: Vec<Position>,
}

/// Une adresse publique Hyperliquid (0x + 40 caractères hexadécimaux).
pub fn is_address(a: &str) -> bool {
    a.len() == 42 && a.starts_with("0x") && a[2..].chars().all(|c| c.is_ascii_hexdigit())
}

/// Lit la réponse `clearinghouseState` d'Hyperliquid.
pub fn parse_account(raw: &str) -> Option<Account> {
    let v: Value = serde_json::from_str(raw).ok()?;
    let f = |x: &Value| x.as_str().and_then(|s| s.parse::<f64>().ok()).or_else(|| x.as_f64()).unwrap_or(0.0);
    let mut positions: Vec<Position> = v["assetPositions"]
        .as_array()?
        .iter()
        .map(|a| &a["position"])
        .filter_map(|p| {
            Some(Position {
                coin: p["coin"].as_str()?.to_string(),
                size: f(&p["szi"]),
                entry: f(&p["entryPx"]),
                value: f(&p["positionValue"]),
                pnl: f(&p["unrealizedPnl"]),
                leverage: f(&p["leverage"]["value"]),
                liq: f(&p["liquidationPx"]),
            })
        })
        .filter(|p| p.size != 0.0)
        .collect();
    positions.sort_by(|a, b| b.value.total_cmp(&a.value));
    Some(Account {
        value: f(&v["marginSummary"]["accountValue"]),
        positions,
    })
}

/// 📊 Flux acheteur / vendeur d'une bougie Binance Futures (tous les ordres, pas seulement les baleines) : le volume en
/// dollars et sa part « taker buy » (achats au marché) ; le reste = ventes au marché.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Flow {
    pub t_ms: i64,
    pub buy_usd: f64,
    pub sell_usd: f64,
}

/// Bougies Binance Futures de `coin` (`interval` : « 5m », « 1h »…), les `limit` dernières.
pub fn flow_url(coin: &str, interval: &str, limit: usize) -> String {
    let c: String = coin.chars().filter(|c| c.is_ascii_alphanumeric()).take(15).collect::<String>().to_uppercase();
    format!(
        "https://fapi.binance.com/fapi/v1/klines?symbol={c}USDT&interval={interval}&limit={}",
        limit.min(1500)
    )
}

/// Lit les bougies : champ 7 = volume en dollars, champ 10 = part acheteuse (taker buy) en dollars.
pub fn parse_flow(body: &str) -> Option<Vec<Flow>> {
    let v: Value = serde_json::from_str(body).ok()?;
    let f = |x: &Value| x.as_str().and_then(|s| s.parse::<f64>().ok()).or_else(|| x.as_f64());
    Some(
        v.as_array()?
            .iter()
            .filter_map(|k| {
                let quote = f(&k[7])?;
                let buy = f(&k[10])?;
                Some(Flow {
                    t_ms: k[0].as_i64()?,
                    buy_usd: buy,
                    sell_usd: (quote - buy).max(0.0),
                })
            })
            .collect(),
    )
}

/// 📊 Flux de `coin` : 7 jours en bougies 1 h, et 24 h en bougies 5 min (2 appels publics, sans clé).
#[cfg(any(feature = "native", feature = "web"))]
pub async fn flow(coin: &str) -> argos_core::Result<(Vec<Flow>, Vec<Flow>)> {
    use crate::http::Http;
    let http = crate::rt::Client::default();
    let h1 = http.get(&flow_url(coin, "1h", 168)).await.map_err(|e| e.to_string())?;
    let m5 = http.get(&flow_url(coin, "5m", 288)).await.map_err(|e| e.to_string())?;
    let h1 = parse_flow(&h1).ok_or("bougies illisibles")?;
    let m5 = parse_flow(&m5).ok_or("bougies illisibles")?;
    if h1.is_empty() {
        return Err(format!("{coin} n'est pas sur Binance Futures").into());
    }
    Ok((h1, m5))
}

/// Positions ouvertes d'une adresse publique (aucune clé : c'est public sur Hyperliquid).
#[cfg(any(feature = "native", feature = "web"))]
pub async fn account(address: &str) -> argos_core::Result<Account> {
    use crate::http::Http;
    if !is_address(address) {
        return Err("adresse invalide".into());
    }
    let body = format!(r#"{{"type":"clearinghouseState","user":"{address}"}}"#);
    let raw = crate::rt::Client::default()
        .post_json("https://api.hyperliquid.xyz/info", &body)
        .await
        .map_err(|e| e.to_string())?;
    parse_account(&raw).ok_or_else(|| "réponse illisible".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grosses_transactions_okx_bybit_bitget() {
        let okx = r#"{"arg":{"channel":"trades","instId":"BTC-USDT"},"data":[{"instId":"BTC-USDT","tradeId":"1","px":"76319.2","sz":"10","side":"buy","ts":"1793908804337"}]}"#;
        let w = parse(okx);
        assert_eq!((w[0].coin.as_str(), w[0].src, w[0].buy, w[0].time_ms), ("BTC", "OKX", true, 1_793_908_804_337));
        let by = r#"{"topic":"publicTrade.ETHUSDT","type":"snapshot","data":[{"T":1793908804262,"s":"ETHUSDT","S":"Sell","v":"100","p":"2400"}]}"#;
        let w = parse(by);
        assert_eq!((w[0].coin.as_str(), w[0].src, w[0].buy), ("ETH", "Bybit", false));
        assert!((w[0].usd() - 240_000.0).abs() < 1e-6);
        let bg = r#"{"action":"update","arg":{"instType":"USDT-FUTURES","channel":"trade","instId":"SOLUSDT"},"data":[{"ts":"1793908804390","price":"150","size":"1000","side":"sell","tradeId":"9"}]}"#;
        let w = parse(bg);
        assert_eq!((w[0].coin.as_str(), w[0].src, w[0].buy), ("SOL", "Bitget", false));
        // petites transactions : filtrées
        assert!(parse(&okx.replace("\"sz\":\"10\"", "\"sz\":\"0.01\"")).is_empty());
        for e in EXCHANGES.iter().filter(|e| **e != "KuCoin") {
            assert!(sub_of(e, &["BTC".into()]).is_some(), "{e}");
        }
        let k = kucoin_sub_at("wss://ws-api-spot.kucoin.com/?token=x&connectId=argos", &["BTC".into(), "ETH".into()]);
        assert!(k.subscribe.contains("/market/match:BTC-USDT,ETH-USDT"));
        assert_eq!(
            parse_kucoin_bullet(
                r#"{"code":"200000","data":{"token":"abc","instanceServers":[{"endpoint":"wss://ws-api-spot.kucoin.com/","pingInterval":18000}]}}"#
            )
            .as_deref(),
            Some("wss://ws-api-spot.kucoin.com/?token=abc&connectId=argos")
        );
        // réponses aux messages de contrôle
        use crate::live::Reply;
        assert_eq!(
            Reply::CryptoCom
                .answer(r#"{"id":1587523073344,"method":"public/heartbeat","code":0}"#)
                .as_deref(),
            Some(r#"{"id":1587523073344,"method":"public/respond-heartbeat"}"#)
        );
        assert_eq!(Reply::Htx.answer(r#"{"ping":1492420473027}"#).as_deref(), Some(r#"{"pong":1492420473027}"#));
        assert_eq!(Reply::Htx.answer(r#"{"ch":"x"}"#), None);
        // messages réels (relevés le 2026-11-05), gonflés au-dessus du plancher
        let cases = [
            (
                r#"{"type":"match","side":"sell","size":"5","price":"76093.63","product_id":"BTC-USD","time":"2026-11-05T20:06:40.006907Z"}"#,
                "Coinbase",
                true,
            ),
            (
                r#"{"channel":"trade","type":"update","data":[{"symbol":"BTC/USD","side":"sell","price":76126.0,"qty":5.0,"timestamp":"2026-11-05T20:07:07.660699Z"}]}"#,
                "Kraken",
                false,
            ),
            (
                r#"{"time":1,"channel":"spot.trades","event":"update","result":{"side":"buy","currency_pair":"BTC_USDT","amount":"5","price":"76000","create_time_ms":"1793909228853.920000"}}"#,
                "Gate.io",
                true,
            ),
            (
                r#"{"symbol":"BTC_USDT","data":[{"p":76231,"v":50000,"T":2,"t":1793909229110}],"channel":"push.deal","ts":1}"#,
                "MEXC",
                false,
            ),
            (
                r#"{"arg":{"channel":"trades","instId":"BTC-USDT"},"data":[{"instId":"BTC-USDT","price":"76223","size":"5000","side":"buy","ts":"1793909206560"}]}"#,
                "BloFin",
                true,
            ),
            (
                r#"{"ch":"trade","symbol":"BTCUSDT","ts":1793909228787,"data":[{"t":"2026-11-05T20:07:08Z","p":"76236.8","v":"5","s":"sell"}]}"#,
                "Bitunix",
                false,
            ),
            (
                r#"{"topic":"/market/match:BTC-USDT","type":"message","subject":"trade.l3match","data":{"price":"76380.5","side":"buy","size":"5","symbol":"BTC-USDT","time":"1793912124957000000"}}"#,
                "KuCoin",
                true,
            ),
            (
                r#"{"ch":"market.btcusdt.trade.detail","ts":1,"tick":{"id":1,"ts":1,"data":[{"id":1,"ts":1793909187232,"amount":5.0,"price":76250.01,"direction":"sell"}]}}"#,
                "HTX",
                false,
            ),
            (
                r#"{"id":-1,"method":"subscribe","code":0,"result":{"instrument_name":"BTC_USDT","subscription":"trade.BTC_USDT","channel":"trade","data":[{"d":"1","t":1793912124957,"p":"76238.77","q":"5","s":"BUY","i":"BTC_USDT"}]}}"#,
                "Crypto.com",
                true,
            ),
        ];
        for (raw, src, buy) in cases {
            let w = parse(raw);
            assert_eq!(w.len(), 1, "{src}");
            assert_eq!((w[0].coin.as_str(), w[0].src, w[0].buy), ("BTC", src, buy), "{src}");
            assert!(w[0].usd() > 300_000.0 && w[0].time_ms > 1_700_000_000_000, "{src} : {:?}", w[0]);
        }
    }

    #[test]
    fn flux_acheteur_vendeur_des_bougies() {
        let raw = r#"[[1793901600000,"75914.4","76248.4","75669.4","76067.2","8821.4",1793905199999,"670468067.2",265757,"4620.4","351223394.3","0"]]"#;
        let f = parse_flow(raw).unwrap();
        assert_eq!(f[0].t_ms, 1_793_901_600_000);
        assert!((f[0].buy_usd - 351_223_394.3).abs() < 1.0);
        assert!((f[0].sell_usd - (670_468_067.2 - 351_223_394.3)).abs() < 1.0);
        assert!(flow_url("btc", "1h", 168).ends_with("symbol=BTCUSDT&interval=1h&limit=168"));
    }

    #[test]
    fn grosses_transactions() {
        let hl = r#"{"channel":"trades","data":[
            {"coin":"BTC","side":"A","px":"100000.0","sz":"2.5","time":1793020000000,"users":["0xaaa","0xbbb"]},
            {"coin":"BTC","side":"B","px":"100000.0","sz":"0.01","time":1793020000001,"users":["0xccc","0xddd"]}]}"#;
        let w = parse(hl);
        assert_eq!(w.len(), 1, "le petit trade (1 000 $) est laissé de côté");
        assert_eq!(
            (w[0].coin.as_str(), w[0].buy, w[0].wallet.as_str(), w[0].usd()),
            ("BTC", false, "0xbbb", 250_000.0)
        );
        let bn = r#"{"e":"aggTrade","E":1793020000100,"s":"ETHUSDT","a":1,"p":"4000.00","q":"30.000","f":1,"l":2,"T":1793020000099,"m":false}"#;
        let w = parse(bn);
        assert_eq!((w[0].coin.as_str(), w[0].buy, w[0].src, w[0].usd()), ("ETH", true, "Binance", 120_000.0));
        assert!(parse(r#"{"channel":"pong"}"#).is_empty());
        assert!(parse(r#"{"result":null,"id":7}"#).is_empty());
    }

    #[test]
    fn positions_d_une_baleine() {
        let raw = r#"{"marginSummary":{"accountValue":"12500000.5"},"assetPositions":[
            {"type":"oneWay","position":{"coin":"ETH","szi":"-800.0","entryPx":"4100.0","positionValue":"3200000.0","unrealizedPnl":"80000.0","leverage":{"type":"cross","value":10},"liquidationPx":"5200.0"}},
            {"type":"oneWay","position":{"coin":"BTC","szi":"50.0","entryPx":"100000.0","positionValue":"5600000.0","unrealizedPnl":"600000.0","leverage":{"type":"cross","value":20},"liquidationPx":null}}]}"#;
        let a = parse_account(raw).unwrap();
        assert_eq!(a.value, 12_500_000.5);
        assert_eq!(a.positions[0].coin, "BTC", "la plus grosse d'abord");
        assert_eq!((a.positions[1].size, a.positions[1].leverage, a.positions[1].liq), (-800.0, 10.0, 5200.0));
        assert_eq!(a.positions[0].liq, 0.0);
        assert!(is_address("0x1234567890abcdef1234567890ABCDEF12345678"));
        assert!(!is_address("0x123") && !is_address("1234567890abcdef1234567890abcdef12345678xx"));
    }

    #[test]
    fn abonnements() {
        assert_eq!(
            [coin_of("BTCUSDT"), coin_of("eth-usdt-swap"), coin_of("SOL"), coin_of("XRP/USD")],
            ["BTC", "ETH", "SOL", "XRP"]
        );
        let c = coins("doge");
        assert_eq!(c, ["BTC", "ETH", "SOL", "XRP", "DOGE"]);
        assert_eq!(coins("ETH").len(), 4, "pas de doublon");
        let h = hyperliquid(&c);
        assert_eq!(h.subscribe.lines().count(), 5, "un abonnement par pièce");
        assert!(h.subscribe.lines().all(|l| serde_json::from_str::<Value>(l).is_ok()));
        let b = binance(&c);
        assert!(b.subscribe.contains(r#""btcusdt@aggTrade","ethusdt@aggTrade""#));
        assert!(serde_json::from_str::<Value>(&b.subscribe).is_ok());
    }

    /// 💤 The trades, as the live candles (`live::tests::une_page_ne_juge_que_les_liens_qui_repondent_au_ping`): a page
    /// judges the silence of a connection only where the exchange answers the app's pings with a message
    /// (`LiveSub::judged`).
    #[test]
    fn une_page_ne_juge_que_les_trades_qui_repondent_au_ping() {
        let coins = ["BTC".to_string()];
        // pinged by the app, and the exchange answers: judged
        for name in ["OKX", "Bybit", "Bitget", "Hyperliquid", "Kraken", "Gate.io", "MEXC", "BloFin"] {
            assert!(sub_of(name, &coins).expect("trades").judged(), "{name}");
        }
        // the exchange pings by itself, or asks the app to answer its own pings: not judged
        for name in ["Binance", "Coinbase", "HTX", "Crypto.com"] {
            assert!(!sub_of(name, &coins).expect("trades").judged(), "{name}");
        }
        // pinged by the app, but nothing comes back (Bitunix): still pinged, to keep the link open, and not judged
        let sub = sub_of("Bitunix", &coins).expect("trades");
        assert!(!sub.ping.is_empty() && sub.ping_unanswered && !sub.judged(), "{sub:?}");
    }
}
