//! 🪙 Spot de Binance, OKX, Bybit et Bitget (« BINANCE:BTCUSDT » sans .P ouvre le spot, plus le perp). Un seul module
//! pour les quatre : mêmes formats de bougies que leurs perps (on réutilise leurs lecteurs), autres adresses.
//! API publiques sans clé, testées en direct le 2026-11-05.

use super::Request;
use crate::parse::{decimals, json, num, text};
use argos_core::feed::Market;
use argos_core::{ArgosError, Bar, Result, Timeframe};
use argos_core::{Instrument, MarketKind};

/// Exchange spot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Venue {
    Binance,
    Okx,
    Bybit,
    Bitget,
}

impl Venue {
    /// Identifiant du catalogue (« binance-spot »).
    pub fn id(self) -> &'static str {
        match self {
            Venue::Binance => "binance-spot",
            Venue::Okx => "okx-spot",
            Venue::Bybit => "bybit-spot",
            Venue::Bitget => "bitget-spot",
        }
    }
}

fn okx_bar(tf: Timeframe) -> &'static str {
    match tf {
        Timeframe::H1 => "1H",
        Timeframe::H2 => "2H",
        Timeframe::H4 => "4H",
        // 12 h, days and weeks from 00:00 UTC, as OKX's perps (its own « 12H » begins at 00:00 in Hong Kong)
        Timeframe::H12 => "12Hutc",
        Timeframe::D1 => "1Dutc",
        Timeframe::W1 => "1Wutc",
        t => t.id(),
    }
}

fn bybit_int(tf: Timeframe) -> &'static str {
    match tf {
        Timeframe::M1 => "1",
        Timeframe::M3 => "3",
        Timeframe::M5 => "5",
        Timeframe::M15 => "15",
        Timeframe::M30 => "30",
        Timeframe::H1 => "60",
        Timeframe::H2 => "120",
        Timeframe::H4 => "240",
        Timeframe::H12 => "720",
        Timeframe::D1 => "D",
        Timeframe::W1 => "W",
    }
}

fn bitget_gran(tf: Timeframe) -> Result<&'static str> {
    Ok(match tf {
        Timeframe::M1 => "1min",
        Timeframe::M3 => "3min",
        Timeframe::M5 => "5min",
        Timeframe::M15 => "15min",
        Timeframe::M30 => "30min",
        Timeframe::H1 => "1h",
        Timeframe::H4 => "4h",
        // 12 h, days and weeks from 00:00 UTC: Bitget's own « 1day » begins at 00:00 in Asia (16:00 UTC, seen on
        // 23 Nov. 2026)
        Timeframe::H12 => "12Hutc",
        Timeframe::D1 => "1Dutc",
        Timeframe::W1 => "1Wutc",
        Timeframe::H2 => return Err(ArgosError::Feed("Bitget spot : pas de bougies 2 h".into())),
    })
}

/// `end_ms` : seulement les bougies d'avant ce moment (ms) ; `None` = les plus récentes.
pub fn candles_request(v: Venue, base: &str, tf: Timeframe, limit: usize, end_ms: Option<i64>) -> Result<Request> {
    // ₿ « XRP/BTC »: the BTC pair; else the USDT pair, as always
    let (b, d) = (crate::paire::symbole(base, "", "USDT"), crate::paire::symbole(base, "-", "USDT"));
    Ok(Request::Get(match v {
        Venue::Binance => {
            let end = end_ms.map_or(String::new(), |e| format!("&endTime={e}"));
            format!(
                "https://api.binance.com/api/v3/klines?symbol={b}&interval={}&limit={}{end}",
                tf.id(),
                limit.min(1000)
            )
        }
        Venue::Okx => match end_ms {
            None => format!(
                "https://www.okx.com/api/v5/market/candles?instId={d}&bar={}&limit={}",
                okx_bar(tf),
                limit.min(300)
            ),
            Some(e) => format!(
                "https://www.okx.com/api/v5/market/history-candles?instId={d}&bar={}&after={e}&limit={}",
                okx_bar(tf),
                limit.min(100)
            ),
        },
        Venue::Bybit => {
            let end = end_ms.map_or(String::new(), |e| format!("&end={e}"));
            format!(
                "https://api.bybit.com/v5/market/kline?category=spot&symbol={b}&interval={}&limit={}{end}",
                bybit_int(tf),
                limit.min(1000)
            )
        }
        Venue::Bitget => {
            let end = end_ms.map_or(String::new(), |e| format!("&endTime={e}"));
            format!(
                "https://api.bitget.com/api/v2/spot/market/candles?symbol={b}&granularity={}&limit={}{end}",
                bitget_gran(tf)?,
                limit.min(1000)
            )
        }
    }))
}

/// Mêmes formats que les perps : on réutilise leurs lecteurs.
pub fn parse_candles(v: Venue, body: &str) -> Result<Vec<Bar>> {
    match v {
        Venue::Binance => super::binance::parse_candles(body),
        // a spot pair gives its volume in coins at index 5 (a perp, at 6)
        Venue::Okx => super::okx::parse_candles_in(body, super::okx::COINS_SPOT),
        Venue::Bybit => super::bybit::parse_candles(body),
        Venue::Bitget => super::bitget::parse_candles(body),
    }
}

pub fn markets_request(v: Venue) -> Request {
    Request::Get(
        match v {
            Venue::Binance => "https://api.binance.com/api/v3/ticker/24hr",
            Venue::Okx => "https://www.okx.com/api/v5/market/tickers?instType=SPOT",
            Venue::Bybit => "https://api.bybit.com/v5/market/tickers?category=spot",
            Venue::Bitget => "https://api.bitget.com/api/v2/spot/market/tickers",
        }
        .into(),
    )
}

pub fn parse_markets(v: Venue, body: &str) -> Result<Vec<Market>> {
    let j = json(body)?;
    let m = |base: Option<&str>, price: &serde_json::Value, prev: &serde_json::Value| {
        Some(Market {
            base: base?.to_string(),
            price: num(price),
            prev: num(prev),
        })
    };
    let rows = |x: &serde_json::Value| x.as_array().cloned().unwrap_or_default();
    // the USDT pairs (base « XRP ») and ₿ the BTC pairs (base « XRP/BTC »)
    let base = |s: Option<&str>, sep: &str| -> Option<String> {
        let s = s?;
        match s.strip_suffix(&format!("{sep}USDT")) {
            Some(b) => Some(b.to_string()),
            None => s
                .strip_suffix(&format!("{sep}BTC"))
                .filter(|b| !b.is_empty())
                .map(|b| crate::paire::base(b, "BTC")),
        }
    };
    let m = |b: Option<String>, price: &serde_json::Value, prev: &serde_json::Value| m(b.as_deref(), price, prev);
    Ok(match v {
        Venue::Binance => rows(&j)
            .iter()
            .filter_map(|t| m(base(t["symbol"].as_str(), ""), &t["lastPrice"], &t["openPrice"]))
            .collect(),
        Venue::Okx => rows(&j["data"])
            .iter()
            .filter_map(|t| m(base(t["instId"].as_str(), "-"), &t["last"], &t["open24h"]))
            .collect(),
        Venue::Bybit => rows(&j["result"]["list"])
            .iter()
            .filter_map(|t| m(base(t["symbol"].as_str(), ""), &t["lastPrice"], &t["prevPrice24h"]))
            .collect(),
        Venue::Bitget => rows(&j["data"])
            .iter()
            .filter_map(|t| m(base(t["symbol"].as_str(), ""), &t["lastPr"], &t["open"]))
            .collect(),
    })
}

pub fn instruments_request(v: Venue) -> Request {
    Request::Get(
        match v {
            // ₿ the pairs that trade, without their permission sets: about 2.6 MB, not 18 MB (over `MAX_BODY`, the
            // whole catalogue was refused and Binance's BTC pairs never reached the search)
            Venue::Binance => "https://api.binance.com/api/v3/exchangeInfo?permissions=SPOT&showPermissionSets=false&symbolStatus=TRADING",
            Venue::Okx => "https://www.okx.com/api/v5/public/instruments?instType=SPOT",
            Venue::Bybit => "https://api.bybit.com/v5/market/instruments-info?category=spot&limit=1000",
            Venue::Bitget => "https://api.bitget.com/api/v2/spot/public/symbols",
        }
        .into(),
    )
}

/// Catalogue spot : une seule ligne par paire (jamais deux fois le même ticker sur le même exchange).
pub fn parse_instruments(v: Venue, body: &str) -> Result<Vec<Instrument>> {
    let j = json(body)?;
    let mk = |symbol: String, base: String, quote: String, pd: u32, sd: u32, active: bool| Instrument {
        exchange: v.id().into(),
        symbol,
        base,
        quote,
        kind: MarketKind::Spot,
        group: "Crypto".into(),
        price_decimals: pd,
        size_decimals: sd,
        growth: false,
        active,
    };
    let rows = |x: &serde_json::Value| x.as_array().cloned().unwrap_or_default();
    let mut out: Vec<Instrument> = match v {
        Venue::Binance => rows(&j["symbols"])
            .iter()
            .map(|s| {
                let tick = s["filters"]
                    .as_array()
                    .and_then(|f| f.iter().find(|x| x["filterType"] == "PRICE_FILTER"))
                    .map(|f| f["tickSize"].clone());
                mk(
                    text(&s["symbol"]),
                    text(&s["baseAsset"]),
                    text(&s["quoteAsset"]),
                    tick.as_ref().map_or(2, decimals),
                    decimals(&s["baseAssetPrecision"]).min(8),
                    s["status"] == "TRADING",
                )
            })
            .collect(),
        Venue::Okx => rows(&j["data"])
            .iter()
            .map(|s| {
                mk(
                    text(&s["instId"]),
                    text(&s["baseCcy"]),
                    text(&s["quoteCcy"]),
                    decimals(&s["tickSz"]),
                    decimals(&s["lotSz"]),
                    s["state"] == "live",
                )
            })
            .collect(),
        Venue::Bybit => rows(&j["result"]["list"])
            .iter()
            .map(|s| {
                mk(
                    text(&s["symbol"]),
                    text(&s["baseCoin"]),
                    text(&s["quoteCoin"]),
                    decimals(&s["priceFilter"]["tickSize"]),
                    decimals(&s["lotSizeFilter"]["basePrecision"]),
                    s["status"] == "Trading",
                )
            })
            .collect(),
        Venue::Bitget => rows(&j["data"])
            .iter()
            .map(|s| {
                mk(
                    text(&s["symbol"]),
                    text(&s["baseCoin"]),
                    text(&s["quoteCoin"]),
                    text(&s["pricePrecision"]).parse().unwrap_or(2),
                    text(&s["quantityPrecision"]).parse().unwrap_or(4),
                    s["status"] == "online",
                )
            })
            .collect(),
    };
    if j["code"].as_str().is_some_and(|c| c != "0" && c != "00000") || j["retCode"].as_i64().is_some_and(|c| c != 0) {
        return Err(ArgosError::Feed(format!("{} : catalogue refusé", v.id())));
    }
    out.retain(|i| !i.symbol.is_empty() && !i.base.is_empty());
    out.sort_by(|a, b| a.symbol.cmp(&b.symbol));
    out.dedup_by(|a, b| a.symbol == b.symbol);
    Ok(out)
}

/// Direct des bougies spot.
pub fn live_sub(v: Venue, base: &str, tf: Timeframe) -> Option<crate::live::LiveSub> {
    let (b, d) = (crate::paire::symbole(base, "", "USDT"), crate::paire::symbole(base, "-", "USDT"));
    Some(match v {
        Venue::Binance => crate::live::LiveSub {
            url: "wss://stream.binance.com:9443/ws".into(),
            subscribe: format!(r#"{{"method":"SUBSCRIBE","params":["{}@kline_{}"],"id":1}}"#, b.to_lowercase(), tf.id()),
            ..Default::default()
        },
        Venue::Okx => crate::live::LiveSub {
            url: "wss://ws.okx.com:8443/ws/v5/business".into(),
            subscribe: format!(r#"{{"op":"subscribe","args":[{{"channel":"candle{}","instId":"{d}"}}]}}"#, okx_bar(tf)),
            ping: "ping".into(),
            ..Default::default()
        },
        Venue::Bybit => crate::live::LiveSub {
            url: "wss://stream.bybit.com/v5/public/spot".into(),
            subscribe: format!(r#"{{"op":"subscribe","args":["kline.{}.{b}"]}}"#, bybit_int(tf)),
            ping: r#"{"op":"ping"}"#.into(),
            ..Default::default()
        },
        Venue::Bitget => crate::live::LiveSub {
            url: "wss://ws.bitget.com/v2/ws/public".into(),
            subscribe: format!(
                r#"{{"op":"subscribe","args":[{{"instType":"SPOT","channel":"candle{}","instId":"{b}"}}]}}"#,
                spot_ws_tf(tf)?
            ),
            ping: "ping".into(),
            ..Default::default()
        },
    })
}

/// 🧮 Trades spot de chaque pièce (contre l'USDT), sur une connexion : pour l'orderflow réel ; les mêmes flux et
/// décodeurs que leurs perps (whales), à l'adresse ou au marché du spot. Relevé le 2026-11-15 : même sens du preneur.
pub fn trades_sub(v: Venue, coins: &[String]) -> crate::live::LiveSub {
    match v {
        Venue::Binance => crate::whales::aggtrades("wss://stream.binance.com:9443/ws", coins),
        Venue::Okx => crate::whales::okx_trades(coins, "-USDT"),
        Venue::Bybit => crate::whales::bybit_trades("wss://stream.bybit.com/v5/public/spot", coins),
        Venue::Bitget => crate::whales::bitget_trades("SPOT", coins),
    }
}

/// Canal de bougies spot de Bitget (« candle30m », « candle1H »…).
fn spot_ws_tf(tf: Timeframe) -> Option<&'static str> {
    Some(match tf {
        Timeframe::M1 => "1m",
        Timeframe::M5 => "5m",
        Timeframe::M15 => "15m",
        Timeframe::M30 => "30m",
        Timeframe::H1 => "1H",
        Timeframe::H4 => "4H",
        // from 00:00 UTC, as its candles (`bitget_gran`)
        Timeframe::H12 => "12Hutc",
        Timeframe::D1 => "1Dutc",
        Timeframe::W1 => "1Wutc",
        _ => return None,
    })
}

pub fn parse_live(v: Venue, msg: &str) -> Option<Bar> {
    match v {
        Venue::Binance => super::binance::parse_live(msg),
        Venue::Okx => super::okx::parse_live_in(msg, super::okx::COINS_SPOT),
        Venue::Bybit => super::bybit::parse_live(msg),
        Venue::Bitget => super::bitget::parse_live(msg),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spot_des_quatre_exchanges() {
        assert_eq!(
            candles_request(Venue::Binance, "btc", Timeframe::M30, 1, None).unwrap(),
            Request::Get("https://api.binance.com/api/v3/klines?symbol=BTCUSDT&interval=30m&limit=1".into())
        );
        assert!(
            matches!(candles_request(Venue::Okx, "BTC", Timeframe::H1, 5, None).unwrap(), Request::Get(u) if u.contains("instId=BTC-USDT&") && !u.contains("SWAP"))
        );
        assert!(matches!(candles_request(Venue::Bybit, "BTC", Timeframe::H1, 5, None).unwrap(), Request::Get(u) if u.contains("category=spot")));
        let b = parse_candles(
            Venue::Bitget,
            r#"{"code":"00000","data":[["1793912400000","76237.6","76237.6","75760.01","76003.87","124.5","9467712.9","9467712.9"]]}"#,
        )
        .unwrap();
        assert_eq!(b[0].close, 76003.87);
        let m = parse_markets(Venue::Bitget, r#"{"code":"00000","data":[{"symbol":"BTCUSDT","lastPr":"110","open":"100"}]}"#).unwrap();
        assert_eq!((m[0].base.as_str(), m[0].prev), ("BTC", 100.0));
        // une seule fois par paire, même si l'exchange la répète
        let i = parse_instruments(
            Venue::Okx,
            r#"{"code":"0","data":[{"instId":"BTC-USDT","baseCcy":"BTC","quoteCcy":"USDT","tickSz":"0.1","lotSz":"0.00000001","state":"live"},{"instId":"BTC-USDT","baseCcy":"BTC","quoteCcy":"USDT","tickSz":"0.1","lotSz":"0.00000001","state":"live"}]}"#,
        )
        .unwrap();
        assert_eq!(i.len(), 1);
        assert_eq!((i[0].id().as_str(), i[0].kind), ("okx-spot:BTC-USDT", MarketKind::Spot));
    }

    /// 🔒 Reliable data: the spot markets of OKX and Bitget ask their 12 hours, days and weeks from 00:00 UTC (the
    /// candles and their live channel); OKX's spot gives its volume in coins at index 5 (real answer of 23 Nov. 2026).
    #[test]
    fn spot_days_in_utc_and_okx_volume_in_coins() {
        for (tf, name) in [(Timeframe::H12, "12Hutc"), (Timeframe::D1, "1Dutc"), (Timeframe::W1, "1Wutc")] {
            let url = |v| match candles_request(v, "BTC", tf, 5, None).unwrap() {
                Request::Get(u) => u,
                Request::PostJson { url, .. } => url,
            };
            assert!(url(Venue::Okx).contains(&format!("&bar={name}&")), "{}", url(Venue::Okx));
            assert!(url(Venue::Bitget).contains(&format!("&granularity={name}&")), "{}", url(Venue::Bitget));
            for v in [Venue::Okx, Venue::Bitget] {
                let sub = live_sub(v, "BTC", tf).expect("live").subscribe;
                assert!(sub.contains(&format!(r#""channel":"candle{name}""#)), "{sub}");
            }
        }
        let body = r#"{"code":"0","data":[["1795470240000","61288.3","61313","61253.2","61266.2","5.31868739","325962.867454732","325962.867454732","0"]]}"#;
        assert_eq!(parse_candles(Venue::Okx, body).unwrap()[0].volume, 5.31868739);
        let live = r#"{"arg":{"channel":"candle1m","instId":"BTC-USDT"},"data":[["1795470240000","61288.3","61313","61253.2","61266.2","5.31868739","325962.867454732","325962.867454732","0"]]}"#;
        assert_eq!(parse_live(Venue::Okx, live).map(|b| b.volume), Some(5.31868739));
    }

    /// ₿ The BTC pairs of the four spot markets: their candles, their prices (base « XRP/BTC »), the USDT ones unchanged.
    /// ₿ Binance's spot catalogue fits under `MAX_BODY`: its whole answer (18 MB with the permission sets) was refused,
    /// in the browser too, so its BTC pairs never reached the search « xrpbtc ».
    #[test]
    fn catalogue_binance_spot_sous_la_taille_max() {
        let Request::Get(u) = instruments_request(Venue::Binance) else {
            panic!("GET")
        };
        assert!(u.starts_with("https://api.binance.com/api/v3/exchangeInfo?"), "same host: {u}");
        assert!(u.contains("showPermissionSets=false") && u.contains("symbolStatus=TRADING"), "{u}");
        // 🔒 reliable data: Binance's real answer for XRPBTC (recorded 23 Nov 2026, this request) is read as it is:
        // XRP against BTC, prices with the 8 decimals of its tick (0.00000001), quantities in whole XRP
        let body = r#"{"symbols":[{"symbol":"XRPBTC","status":"TRADING","baseAsset":"XRP","baseAssetPrecision":8,"quoteAsset":"BTC","quotePrecision":8,"filters":[{"filterType":"PRICE_FILTER","minPrice":"0.00000001","maxPrice":"1000.00000000","tickSize":"0.00000001"},{"filterType":"LOT_SIZE","minQty":"1.00000000","maxQty":"90000000.00000000","stepSize":"1.00000000"}]}]}"#;
        let i = parse_instruments(Venue::Binance, body).unwrap();
        assert_eq!(i.len(), 1);
        let x = &i[0];
        assert_eq!((x.id().as_str(), x.base.as_str(), x.quote.as_str()), ("binance-spot:XRPBTC", "XRP", "BTC"));
        assert_eq!((x.price_decimals, x.active, x.kind), (8, true, MarketKind::Spot));
    }

    #[test]
    fn paires_contre_btc_du_spot() {
        let url = |v, b| match candles_request(v, b, Timeframe::H4, 5, None).unwrap() {
            Request::Get(u) => u,
            Request::PostJson { url, .. } => url,
        };
        assert!(url(Venue::Binance, "XRP/BTC").contains("symbol=XRPBTC&"));
        assert!(url(Venue::Binance, "XRP").contains("symbol=XRPUSDT&"), "USDT unchanged");
        assert!(url(Venue::Okx, "XRP/BTC").contains("instId=XRP-BTC&"));
        assert!(url(Venue::Bybit, "XRP/BTC").contains("symbol=XRPBTC&"));
        assert!(url(Venue::Bitget, "XRP/BTC").contains("symbol=XRPBTC&"));
        let m = parse_markets(
            Venue::Binance,
            r#"[{"symbol":"XRPUSDT","lastPrice":"2","openPrice":"1"},{"symbol":"XRPBTC","lastPrice":"0.00002345","openPrice":"0.00002"},{"symbol":"XRPEUR","lastPrice":"1","openPrice":"1"}]"#,
        )
        .unwrap();
        let bases: Vec<&str> = m.iter().map(|x| x.base.as_str()).collect();
        assert_eq!(bases, ["XRP", "XRP/BTC"], "USDT and BTC, nothing else");
        let m = parse_markets(Venue::Okx, r#"{"data":[{"instId":"ETH-BTC","last":"0.03","open24h":"0.031"}]}"#).unwrap();
        assert_eq!(m[0].base, "ETH/BTC");
        let l = live_sub(Venue::Okx, "XRP/BTC", Timeframe::H1).unwrap();
        assert!(l.subscribe.contains(r#""instId":"XRP-BTC""#));
        let l = live_sub(Venue::Binance, "XRP/BTC", Timeframe::H1).unwrap();
        assert!(l.subscribe.contains("xrpbtc@kline_1h"));
    }
}
