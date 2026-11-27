//! 🧱 Argos Murs: the public order books (no key) of five perp exchanges (Binance Futures, Bybit, OKX, Bitunix,
//! Hyperliquid) and four spot exchanges (Binance, Bybit, OKX, Coinbase), read as snapshots every few seconds.
//!
//! Why snapshots and not WebSocket streams: the public streams are shallow (Binance's partial book is 20 levels, a
//! few dollars on BTC) or need a snapshot plus every delta to stay right; one deep snapshot every 10 s reaches much
//! further from the price and is never "half-applied". Every host here was already in `net::HOTES`.
//!
//! Sizes are converted to coins (OKX perps are in contracts: × `ctVal`, read once from OKX's instrument list).

use crate::exchanges::Request;
use crate::http::Http;
use argos_core::murs::VenueBook;
use argos_core::{ArgosError, Result};
use serde_json::Value;

/// One order book Argos Murs reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Carnet {
    BinancePerp,
    BybitPerp,
    OkxPerp,
    BitunixPerp,
    HyperliquidPerp,
    BinanceSpot,
    BybitSpot,
    OkxSpot,
    CoinbaseSpot,
}

impl Carnet {
    pub const PERP: [Carnet; 5] = [
        Carnet::BinancePerp,
        Carnet::BybitPerp,
        Carnet::OkxPerp,
        Carnet::BitunixPerp,
        Carnet::HyperliquidPerp,
    ];
    pub const SPOT: [Carnet; 4] = [Carnet::BinanceSpot, Carnet::BybitSpot, Carnet::OkxSpot, Carnet::CoinbaseSpot];

    /// Exchange name shown in the legend.
    pub fn name(self) -> &'static str {
        match self {
            Carnet::BinancePerp | Carnet::BinanceSpot => "Binance",
            Carnet::BybitPerp | Carnet::BybitSpot => "Bybit",
            Carnet::OkxPerp | Carnet::OkxSpot => "OKX",
            Carnet::BitunixPerp => "Bitunix",
            Carnet::HyperliquidPerp => "Hyperliquid",
            Carnet::CoinbaseSpot => "Coinbase",
        }
    }

    pub fn spot(self) -> bool {
        Carnet::SPOT.contains(&self)
    }

    /// Hyperliquid groups its book by significant figures: the coarsest grouping that still fits in `step` (more
    /// depth around the price, never coarser than Argos Murs' buckets).
    pub fn hl_sig(price: f64, step: f64) -> u8 {
        if !(price > 0.0 && step > 0.0) {
            return 5;
        }
        let e = price.log10().floor() as i32;
        (2..=5u8).find(|s| 10f64.powi(e + 1 - *s as i32) <= step * (1.0 + 1e-9)).unwrap_or(5)
    }

    /// The snapshot request for `coin` (« BTC »), USDT pairs (Hyperliquid: USDC).
    pub fn request(self, coin: &str, sig: u8) -> Request {
        self.request_q(coin, sig, Monnaie::Usdt)
    }

    /// Quote currencies of this perp book (spot books: USDT, Coinbase USD).
    pub fn monnaies(self) -> &'static [Monnaie] {
        match self {
            Carnet::BinancePerp | Carnet::BybitPerp | Carnet::OkxPerp => &[Monnaie::Usdt, Monnaie::Usdc],
            Carnet::HyperliquidPerp => &[Monnaie::Usdc],
            _ => &[Monnaie::Usdt],
        }
    }

    /// The perp instrument read (« BTCUSDC », « BTC-USDC-SWAP », « BTCPERP »…).
    pub fn instrument(self, coin: &str, m: Monnaie) -> String {
        let c = coin.to_uppercase();
        let usdc = m == Monnaie::Usdc && self.monnaies().contains(&Monnaie::Usdc);
        match (self, usdc) {
            (Carnet::BybitPerp, true) => format!("{c}PERP"),
            (Carnet::OkxPerp | Carnet::OkxSpot, _) => format!(
                "{c}-{}{}",
                if usdc { "USDC" } else { "USDT" },
                if self == Carnet::OkxPerp { "-SWAP" } else { "" }
            ),
            (Carnet::HyperliquidPerp, _) => c,
            (Carnet::CoinbaseSpot, _) => format!("{c}-USD"),
            (_, true) => format!("{c}USDC"),
            _ => format!("{c}USDT"),
        }
    }

    /// The snapshot request for `coin` in quote `m` (a quote the book does not have: its own).
    pub fn request_q(self, coin: &str, sig: u8, m: Monnaie) -> Request {
        let c = coin.to_uppercase();
        let i = self.instrument(coin, m);
        Request::Get(match self {
            Carnet::BinancePerp => format!("https://fapi.binance.com/fapi/v1/depth?symbol={i}&limit=1000"),
            Carnet::BinanceSpot => format!("https://api.binance.com/api/v3/depth?symbol={c}USDT&limit=5000"),
            Carnet::BybitPerp => format!("https://api.bybit.com/v5/market/orderbook?category=linear&symbol={i}&limit=500"),
            Carnet::BybitSpot => format!("https://api.bybit.com/v5/market/orderbook?category=spot&symbol={c}USDT&limit=200"),
            Carnet::OkxPerp => format!("https://www.okx.com/api/v5/market/books-full?instId={i}&sz=5000"),
            Carnet::OkxSpot => format!("https://www.okx.com/api/v5/market/books-full?instId={c}-USDT&sz=5000"),
            Carnet::BitunixPerp => format!("https://fapi.bitunix.com/api/v1/futures/market/depth?symbol={c}USDT&limit=max"),
            Carnet::CoinbaseSpot => format!("https://api.exchange.coinbase.com/products/{c}-USD/book?level=2"),
            Carnet::HyperliquidPerp => {
                return Request::PostJson {
                    url: "https://api.hyperliquid.xyz/info".into(),
                    body: format!(
                        r#"{{"type":"l2Book","coin":"{}","nSigFigs":{}}}"#,
                        crate::exchanges::hyperliquid::coin(&c),
                        sig.clamp(2, 5)
                    ),
                }
            }
        })
    }

    /// Reads a snapshot; `contract`: coins per contract (OKX perps), 1 elsewhere. A level that cannot be (a price or
    /// a size that is not a number above zero, `feed::price_size_ok`) is left out.
    pub fn parse(self, body: &str, contract: f64) -> Result<VenueBook> {
        let bad = |what: &str| ArgosError::Feed(format!("{} : carnet illisible ({what})", self.name()));
        if self == Carnet::HyperliquidPerp {
            // the same reader as the Book module's live l2Book (book.rs), on the REST answer
            let b = crate::book::parse_book(&format!(r#"{{"channel":"l2Book","data":{body}}}"#)).ok_or_else(|| bad("l2Book"))?;
            let side = |l: &[crate::book::Level]| l.iter().map(|l| (l.px, l.sz)).collect();
            return Ok(VenueBook {
                venue: self.name().into(),
                bids: side(&b.bids),
                asks: side(&b.asks),
            });
        }
        let v: Value = serde_json::from_str(body).map_err(|_| bad("JSON"))?;
        let root = match self {
            Carnet::BybitPerp | Carnet::BybitSpot => {
                if v["retCode"].as_i64() != Some(0) {
                    return Err(bad(v["retMsg"].as_str().unwrap_or("retCode")));
                }
                &v["result"]
            }
            Carnet::OkxPerp | Carnet::OkxSpot => {
                if v["code"].as_str() != Some("0") {
                    return Err(bad(v["msg"].as_str().unwrap_or("code")));
                }
                &v["data"][0]
            }
            Carnet::BitunixPerp => &v["data"],
            _ => &v,
        };
        let (kb, ka) = match self {
            Carnet::BybitPerp | Carnet::BybitSpot => ("b", "a"),
            _ => ("bids", "asks"),
        };
        let k = if self == Carnet::OkxPerp { contract } else { 1.0 };
        let side = |key: &str| -> Vec<(f64, f64)> {
            root[key]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|l| {
                    let n = |i: usize| match &l[i] {
                        Value::String(s) => s.parse::<f64>().ok(),
                        x => x.as_f64(),
                    };
                    Some((n(0)?, n(1)? * k))
                })
                .filter(|&(px, sz)| argos_core::feed::price_size_ok(px, sz))
                .collect()
        };
        let (bids, asks) = (side(kb), side(ka));
        if bids.is_empty() && asks.is_empty() {
            return Err(bad("vide"));
        }
        Ok(VenueBook {
            venue: self.name().into(),
            bids,
            asks,
        })
    }
}

/// Quote currency of a perp book.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Monnaie {
    Usdt,
    Usdc,
}

impl Monnaie {
    pub fn name(self) -> &'static str {
        match self {
            Monnaie::Usdt => "USDT",
            Monnaie::Usdc => "USDC",
        }
    }
}

/// The books the heatmap and the order-book panel read, and what to say about the choice (a fallback, a book left
/// out).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Choix {
    pub carnets: Vec<(Carnet, Monnaie)>,
    pub note: Option<String>,
}

impl Choix {
    /// Only Hyperliquid's USDC book: the live stream (finer than snapshots) is enough.
    pub fn hyperliquid_seul(&self) -> bool {
        self.carnets == [(Carnet::HyperliquidPerp, Monnaie::Usdc)]
    }
}

/// The perp book of a chart exchange (`None`: Argos reads no book there).
pub fn carnet_de(ex: crate::Exchange) -> Option<Carnet> {
    use crate::Exchange as E;
    Some(match ex {
        E::Binance => Carnet::BinancePerp,
        E::Bybit => Carnet::BybitPerp,
        E::Okx => Carnet::OkxPerp,
        E::Bitunix => Carnet::BitunixPerp,
        E::Hyperliquid => Carnet::HyperliquidPerp,
        _ => return None,
    })
}

/// The book of the chart's market, perp or spot (`None`: Argos Murs reads no book there): the trades of the chart
/// are that exchange's, so REMPLI weighs them against its own part of a wall.
pub fn carnet_du_chart(ex: crate::Exchange) -> Option<Carnet> {
    use crate::Exchange as E;
    carnet_de(ex).or(match ex {
        E::BinanceSpot => Some(Carnet::BinanceSpot),
        E::BybitSpot => Some(Carnet::BybitSpot),
        E::OkxSpot => Some(Carnet::OkxSpot),
        E::Coinbase => Some(Carnet::CoinbaseSpot),
        _ => None,
    })
}

/// 📖 Which books to read (shared by the heatmap and the order-book panel). `chart`: the chart's exchange and
/// whether its pair is in USDC; `coches`: Binance, Bybit, OKX, Bitunix, Hyperliquid (used when `auto` is off);
/// `monnaie`: 0 the chart's, 1 USDT, 2 USDC, 3 both (summed); `web`: in the browser (Bitunix refuses it).
/// Nothing readable: Hyperliquid USDC, said in the note.
pub fn carnets_choisis(chart: Option<(crate::Exchange, bool)>, auto: bool, coches: [bool; 5], monnaie: usize, web: bool) -> Choix {
    let mut notes: Vec<String> = vec![];
    let mut l: Vec<Carnet> = if auto {
        match chart.and_then(|(e, _)| carnet_de(e)) {
            Some(c) => vec![c],
            None => {
                if let Some((e, _)) = chart {
                    notes.push(format!("pas de carnet {} : Hyperliquid", e.name()));
                }
                vec![]
            }
        }
    } else {
        Carnet::PERP.iter().zip(coches).filter(|(_, on)| *on).map(|(c, _)| *c).collect()
    };
    if web && l.contains(&Carnet::BitunixPerp) {
        l.retain(|c| *c != Carnet::BitunixPerp);
        notes.push("Bitunix bloqué sur le web".into());
    }
    let chart_usdc = chart.is_some_and(|(e, u)| u || e == crate::Exchange::Hyperliquid);
    let voulues: &[Monnaie] = match monnaie {
        1 => &[Monnaie::Usdt],
        2 => &[Monnaie::Usdc],
        3 => &[Monnaie::Usdt, Monnaie::Usdc],
        _ if chart_usdc => &[Monnaie::Usdc],
        _ => &[Monnaie::Usdt],
    };
    let mut carnets = vec![];
    for c in &l {
        let mut m: Vec<Monnaie> = voulues.iter().copied().filter(|m| c.monnaies().contains(m)).collect();
        // the chart's own currency on an exchange that has only the other one: its own (auto never reads nothing)
        if m.is_empty() && (monnaie == 0 || auto) {
            m = c.monnaies().to_vec();
        }
        if m.is_empty() {
            notes.push(format!("{} : {} seulement", c.name(), c.monnaies()[0].name()));
        }
        carnets.extend(m.into_iter().map(|m| (*c, m)));
    }
    if carnets.is_empty() {
        if !notes.iter().any(|n| n.ends_with(": Hyperliquid")) {
            notes.push("repli sur Hyperliquid".into());
        }
        carnets.push((Carnet::HyperliquidPerp, Monnaie::Usdc));
    }
    Choix {
        carnets,
        note: (!notes.is_empty()).then(|| notes.join(" · ")),
    }
}

/// « Binance USDT · Bybit USDC 2/2 » (+ the note): the books actually read and how many answered.
pub fn titre_carnets(ch: &Choix, ok: usize) -> String {
    let noms: Vec<String> = ch.carnets.iter().map(|(c, m)| format!("{} {}", c.name(), m.name())).collect();
    let mut t = format!("{} {ok}/{}", noms.join(" · "), ch.carnets.len());
    if let Some(n) = &ch.note {
        t += &format!(" ({n})");
    }
    t
}

/// The books summed into one (same price grouped by `tick`): bids best first, asks best first.
pub fn somme(coin: &str, books: &[VenueBook], tick: f64, time_ms: i64) -> crate::book::Book {
    use std::collections::BTreeMap;
    let t = if tick > 0.0 { tick } else { 1.0 };
    let (mut b, mut a): (BTreeMap<i64, f64>, BTreeMap<i64, f64>) = (BTreeMap::new(), BTreeMap::new());
    for v in books {
        for &(px, sz) in &v.bids {
            *b.entry((px / t).floor() as i64).or_default() += sz;
        }
        for &(px, sz) in &v.asks {
            *a.entry((px / t).ceil() as i64).or_default() += sz;
        }
    }
    let lv = |k: &i64, sz: &f64| crate::book::Level {
        px: *k as f64 * t,
        sz: *sz,
        n: 0,
    };
    crate::book::Book {
        coin: coin.into(),
        time_ms,
        bids: b.iter().rev().map(|(k, s)| lv(k, s)).collect(),
        asks: a.iter().map(|(k, s)| lv(k, s)).collect(),
    }
}

/// Coins per OKX perp contract (`ctVal`), from OKX's instrument list.
pub fn parse_okx_ctval(body: &str) -> Option<f64> {
    let v: Value = serde_json::from_str(body).ok()?;
    v["data"][0]["ctVal"].as_str()?.parse().ok().filter(|x: &f64| *x > 0.0)
}

static CTVAL: std::sync::Mutex<Vec<(String, f64)>> = std::sync::Mutex::new(Vec::new());

/// Coins per OKX perp contract of the instrument `inst` (read once, then kept).
async fn ctval_inst<H: Http>(http: &H, inst: &str) -> Result<f64> {
    if let Some(v) = CTVAL.lock().ok().and_then(|l| l.iter().find(|(x, _)| *x == inst).map(|(_, v)| *v)) {
        return Ok(v);
    }
    let raw = http
        .get(&format!("https://www.okx.com/api/v5/public/instruments?instType=SWAP&instId={inst}"))
        .await?;
    let v = parse_okx_ctval(&raw).ok_or_else(|| ArgosError::Feed("OKX : taille du contrat inconnue".into()))?;
    if let Ok(mut l) = CTVAL.lock() {
        l.push((inst.to_string(), v));
    }
    Ok(v)
}

/// 🏦 Coins per OKX USDT perp contract of `coin` when already read (by Argos Murs or the multi-exchange footprint):
/// OKX perp trades are counted in contracts.
pub fn okx_ctval_connu(coin: &str) -> Option<f64> {
    let inst = Carnet::OkxPerp.instrument(coin, Monnaie::Usdt);
    CTVAL.lock().ok().and_then(|l| l.iter().find(|(x, _)| *x == inst).map(|(_, v)| *v))
}

/// 🏦 Reads (once) the coins per OKX USDT perp contract of `coin`, kept for Argos Murs too.
pub async fn okx_ctval<H: Http>(http: &H, coin: &str) -> Result<f64> {
    ctval_inst(http, &Carnet::OkxPerp.instrument(coin, Monnaie::Usdt)).await
}

/// 🏦 Notes a contract size already known (tests of the multi-exchange footprint).
pub fn okx_ctval_noter(coin: &str, v: f64) {
    if let Ok(mut l) = CTVAL.lock() {
        l.push((Carnet::OkxPerp.instrument(coin, Monnaie::Usdt), v));
    }
}

async fn send<H: Http>(http: &H, r: Request) -> Result<String> {
    match r {
        Request::Get(u) => http.get(&u).await,
        Request::PostJson { url, body } => http.post_json(&url, &body).await,
    }
}

/// Reads one book of `coin`; `step`: Argos Murs' bucket (Hyperliquid's grouping follows it), `price`: about the
/// current price.
pub async fn fetch<H: Http>(http: &H, c: Carnet, coin: &str, price: f64, step: f64) -> Result<VenueBook> {
    fetch_q(http, c, coin, price, step, Monnaie::Usdt).await
}

/// `fetch` in quote `m` (heatmap and order-book panel).
pub async fn fetch_q<H: Http>(http: &H, c: Carnet, coin: &str, price: f64, step: f64, m: Monnaie) -> Result<VenueBook> {
    let inst = c.instrument(coin, m);
    let mut k = 1.0;
    if c == Carnet::OkxPerp {
        k = ctval_inst(http, &inst).await?;
    }
    let body = send(http, c.request_q(coin, Carnet::hl_sig(price, step), m)).await?;
    c.parse(&body, k)
}

/// 🧱 Far walls: Hyperliquid's perp book grouped by 2 significant figures (BTC: 1 000 $ groups, about ±30 %), one
/// small answer (≈ 1.7 KB). Asked every third photo only (far groups barely move; Hyperliquid answers 429 when one IP
/// asks too much).
pub const LOIN_SIG: u8 = 2;

/// The far book request of `coin` (Hyperliquid perp, USDC).
pub fn requete_loin(coin: &str) -> Request {
    Carnet::HyperliquidPerp.request_q(coin, LOIN_SIG, Monnaie::Usdc)
}

/// Reads the far book of `coin` (see [`LOIN_SIG`]).
pub async fn fetch_loin<H: Http>(http: &H, coin: &str) -> Result<VenueBook> {
    let body = send(http, requete_loin(coin)).await?;
    Carnet::HyperliquidPerp.parse(&body, 1.0)
}

/// One request of a photo: a near book (`Some`), or the far book (`None`, [`fetch_loin`]); one future type, so the far
/// request goes out together with the others. Binance perp's photo also gives its `lastUpdateId` (the merged book).
pub async fn fetch_photo<H: Http>(http: &H, c: Option<Carnet>, coin: &str, price: f64, step: f64) -> Result<(VenueBook, Option<u64>)> {
    match c {
        Some(Carnet::BinancePerp) => {
            let body = send(http, Carnet::BinancePerp.request_q(coin, 5, Monnaie::Usdt)).await?;
            Ok((Carnet::BinancePerp.parse(&body, 1.0)?, photo_id(&body)))
        }
        Some(c) => Ok((fetch(http, c, coin, price, step).await?, None)),
        None => Ok((fetch_loin(http, coin).await?, None)),
    }
}

/// 📖 Lot loin-1 B: the `lastUpdateId` of a Binance depth photo (REST).
pub fn photo_id(body: &str) -> Option<u64> {
    let v: Value = serde_json::from_str(body).ok()?;
    v["lastUpdateId"].as_u64()
}

/// 📖 Binance perp's diff depth stream of `coin` (USDT perp), every 500 ms (the lightest). The depth streams belong to
/// the `/public` route (`/market` carries klines and aggTrade); host already in `net.rs`.
pub fn flux_binance(coin: &str) -> crate::live::LiveSub {
    crate::live::LiveSub {
        url: "wss://fstream.binance.com/public/ws".into(),
        subscribe: format!(
            r#"{{"method":"SUBSCRIBE","params":["{}@depth@500ms"],"id":11}}"#,
            Carnet::BinancePerp.instrument(coin, Monnaie::Usdt).to_lowercase()
        ),
        ping: String::new(),
        ..Default::default()
    }
}

/// 📖 One `depthUpdate` of Binance's diff depth stream: `pu` the previous event's final id, `u` its own, the levels
/// (absolute quantities; 0 = gone), and the message's size (bytes, for the measurements).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MajProfondeur {
    pub pu: u64,
    pub u: u64,
    pub bids: Vec<(f64, f64)>,
    pub asks: Vec<(f64, f64)>,
    pub octets: usize,
}

/// A `depthUpdate` message (`None`: anything else, e.g. the subscription's answer `{"result":null,"id":11}`). A level
/// that cannot be is left out: its price is not a number above zero, or its quantity is not a number or is negative
/// (0 can be: the level is gone).
pub fn parse_maj(msg: &str) -> Option<MajProfondeur> {
    let v: Value = serde_json::from_str(msg).ok()?;
    if v["e"] != "depthUpdate" {
        return None;
    }
    let cote = |k: &str| -> Vec<(f64, f64)> {
        v[k].as_array()
            .into_iter()
            .flatten()
            .filter_map(|l| -> Option<(f64, f64)> { Some((l[0].as_str()?.parse().ok()?, l[1].as_str()?.parse().ok()?)) })
            .filter(|&(px, q)| px.is_finite() && px > 0.0 && q.is_finite() && q >= 0.0)
            .collect()
    };
    Some(MajProfondeur {
        pu: v["pu"].as_u64()?,
        u: v["u"].as_u64()?,
        bids: cote("b"),
        asks: cote("a"),
        octets: msg.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // trimmed real answers, read on 2026-11-20 (BTC)
    const BINANCE_PERP: &str = r#"{"lastUpdateId":12185381354840,"E":1795208940206,"T":1795208940199,"bids":[["58408.40","16.568"],["58408.30","0.082"]],"asks":[["58408.50","3.1"],["58409.00","0.5"]]}"#;
    const BINANCE_SPOT: &str =
        r#"{"lastUpdateId":103315442377,"bids":[["58437.38000000","9.40237000"],["58437.37000000","6.21734000"]],"asks":[["58437.39000000","1.00000000"]]}"#;
    const BYBIT: &str = r#"{"retCode":0,"retMsg":"OK","result":{"s":"BTCUSDT","a":[["58413.60","2.235"],["58413.70","0.001"]],"b":[["58413.50","1.5"]],"ts":1795208940000,"u":1},"retExtInfo":{},"time":1795208940001}"#;
    const OKX_PERP: &str =
        r#"{"code":"0","msg":"","data":[{"asks":[["58404.5","904.78","43"],["58404.6","1.98","4"]],"bids":[["58404.4","100","3"]],"ts":"1795208940000"}]}"#;
    const OKX_INST: &str = r#"{"code":"0","data":[{"instId":"BTC-USDT-SWAP","instType":"SWAP","ctVal":"0.01","ctValCcy":"BTC"}],"msg":""}"#;
    const BITUNIX: &str = r#"{"code":0,"data":{"asks":[["58414","2.2513"],["58414.1","3.1208"]],"bids":[["58413.9","4.0"]]},"msg":"Success"}"#;
    const COINBASE: &str = r#"{"bids":[["58317.57","0.1266798",4],["58317.53","0.00017341",1]],"asks":[["58317.58","0.5",2]],"sequence":1,"auction_mode":false,"auction":null,"time":"2026-11-20T21:30:00Z"}"#;
    const HL: &str = r#"{"coin":"BTC","time":1795208945101,"levels":[[{"px":"58400.0","sz":"62.37382","n":150},{"px":"58300.0","sz":"673.08328","n":520}],[{"px":"58500.0","sz":"10.5","n":20}]]}"#;

    /// Lot loin-1: the far request asks Hyperliquid for 2 significant figures; a real trimmed answer (21 Nov, BTC
    /// ≈ 60 230) parses into 1 000 $ groups.
    #[test]
    fn carnet_lointain() {
        match requete_loin("btc") {
            Request::PostJson { url, body } => {
                assert_eq!(url, "https://api.hyperliquid.xyz/info");
                assert!(body.contains(r#""nSigFigs":2"#) && body.contains(r#""coin":"BTC""#), "{body}");
            }
            r => panic!("{r:?}"),
        }
        const LOIN: &str = r#"{"coin":"BTC","time":1795305600000,"levels":[[{"px":"60000.0","sz":"812.4","n":900},{"px":"55000.0","sz":"264.4","n":120},{"px":"53000.0","sz":"463.5","n":80}],[{"px":"61000.0","sz":"590.1","n":700},{"px":"63000.0","sz":"504.8","n":95},{"px":"65000.0","sz":"315.4","n":40}]]}"#;
        let b = Carnet::HyperliquidPerp.parse(LOIN, 1.0).unwrap();
        assert_eq!((b.bids.len(), b.asks.len()), (3, 3));
        assert_eq!((b.bids[1], b.asks[2]), ((55_000.0, 264.4), (65_000.0, 315.4)));
    }

    /// Lot loin-1 B: a real trimmed `depthUpdate` (22 Nov), the subscription's answer ignored, the photo's
    /// `lastUpdateId`, and the stream's address and message.
    #[test]
    fn flux_de_profondeur_binance() {
        const MAJ: &str = r#"{"e":"depthUpdate","E":1795325101000,"T":1795325100995,"s":"BTCUSDT","ps":"BTCUSDT","U":12198206284082,"u":12198206342074,"pu":12198206283959,"b":[["1000.00","13.362"],["58960.10","0.000"]],"a":[["58973.30","0.178"]],"st":0}"#;
        let m = parse_maj(MAJ).unwrap();
        assert_eq!((m.pu, m.u), (12_198_206_283_959, 12_198_206_342_074));
        assert_eq!(m.bids, vec![(1000.0, 13.362), (58_960.1, 0.0)]);
        assert_eq!(m.asks, vec![(58_973.3, 0.178)]);
        assert_eq!(m.octets, MAJ.len());
        assert!(parse_maj(r#"{"result":null,"id":11}"#).is_none());
        assert_eq!(photo_id(BINANCE_PERP), Some(12_185_381_354_840));
        let f = flux_binance("btc");
        assert_eq!(f.url, "wss://fstream.binance.com/public/ws");
        assert_eq!(f.subscribe, r#"{"method":"SUBSCRIBE","params":["btcusdt@depth@500ms"],"id":11}"#);
        assert!(crate::net::autorise(&f.url));
    }

    #[test]
    fn chaque_carnet_se_lit() {
        let b = Carnet::BinancePerp.parse(BINANCE_PERP, 1.0).unwrap();
        assert_eq!((b.bids[0], b.asks.len(), b.venue.as_str()), ((58408.4, 16.568), 2, "Binance"));
        let b = Carnet::BinanceSpot.parse(BINANCE_SPOT, 1.0).unwrap();
        assert_eq!(b.bids[1], (58437.37, 6.21734));
        for c in [Carnet::BybitPerp, Carnet::BybitSpot] {
            let b = c.parse(BYBIT, 1.0).unwrap();
            assert_eq!((b.asks[0], b.bids[0]), ((58413.6, 2.235), (58413.5, 1.5)));
        }
        assert!(Carnet::BybitPerp.parse(r#"{"retCode":10001,"retMsg":"params error"}"#, 1.0).is_err());
        // OKX perps: contracts × ctVal (0.01 BTC) = coins
        let k = parse_okx_ctval(OKX_INST).unwrap();
        let b = Carnet::OkxPerp.parse(OKX_PERP, k).unwrap();
        assert!((b.asks[0].1 - 9.0478).abs() < 1e-9 && (b.bids[0].1 - 1.0).abs() < 1e-9);
        let b = Carnet::OkxSpot.parse(OKX_PERP, k).unwrap();
        assert_eq!(b.asks[0].1, 904.78, "OKX spot is already in coins");
        let b = Carnet::BitunixPerp.parse(BITUNIX, 1.0).unwrap();
        assert_eq!((b.asks[1], b.bids[0]), ((58414.1, 3.1208), (58413.9, 4.0)));
        let b = Carnet::CoinbaseSpot.parse(COINBASE, 1.0).unwrap();
        assert_eq!((b.bids.len(), b.asks[0]), (2, (58317.58, 0.5)));
        let b = Carnet::HyperliquidPerp.parse(HL, 1.0).unwrap();
        assert_eq!((b.bids[1], b.asks[0], b.venue.as_str()), ((58300.0, 673.08328), (58500.0, 10.5), "Hyperliquid"));
        assert!(Carnet::CoinbaseSpot.parse(r#"{"message":"NotFound"}"#, 1.0).is_err());
        assert!(Carnet::HyperliquidPerp.parse("null", 1.0).is_err());
    }

    #[test]
    fn adresses_et_regroupement_hyperliquid() {
        assert_eq!(Carnet::hl_sig(58_400.0, 100.0), 3, "100 $ groups on BTC");
        assert_eq!(Carnet::hl_sig(3_000.0, 5.0), 4, "1 $ groups on ETH (10 $ would be coarser than 5 $)");
        assert_eq!(Carnet::hl_sig(150.0, 0.2), 4);
        assert_eq!(Carnet::PERP.len() + Carnet::SPOT.len(), 9);
        for c in Carnet::PERP.iter().chain(&Carnet::SPOT) {
            let url = match c.request("btc", 3) {
                Request::Get(u) => u,
                Request::PostJson { url, body } => {
                    assert!(body.contains(r#""nSigFigs":3"#) && body.contains(r#""coin":"BTC""#));
                    url
                }
            };
            assert!(crate::net::autorise(&url), "{url}");
            assert!(url.contains("BTC") || url.ends_with("/info"), "{url}");
            assert_eq!(c.spot(), Carnet::SPOT.contains(c));
        }
    }

    /// 📖 Jee, 21 Nov: the heatmap and the order-book panel read the chart's own book by default (its exchange, its
    /// quote), or the books ticked, in USDT, USDC or both; Bitunix falls back in the browser; never nothing.
    #[test]
    fn carnet_du_marche_de_la_chart() {
        use crate::Exchange as E;
        assert_eq!(carnet_du_chart(E::Binance), Some(Carnet::BinancePerp));
        assert_eq!(carnet_du_chart(E::BinanceSpot), Some(Carnet::BinanceSpot));
        assert_eq!(carnet_du_chart(E::Coinbase), Some(Carnet::CoinbaseSpot));
        assert_eq!(carnet_du_chart(E::Kraken), None);
    }

    #[test]
    fn choix_des_carnets_et_replis() {
        use crate::Exchange as E;
        use Monnaie::{Usdc, Usdt};
        let c = |chart, auto, coches, m, web| carnets_choisis(chart, auto, coches, m, web);
        let tout = [true; 5];
        // default: the chart's exchange alone, in the chart's quote
        assert_eq!(c(Some((E::Binance, false)), true, tout, 0, false).carnets, vec![(Carnet::BinancePerp, Usdt)]);
        assert_eq!(c(Some((E::Binance, true)), true, tout, 0, false).carnets, vec![(Carnet::BinancePerp, Usdc)]);
        assert!(c(Some((E::Hyperliquid, false)), true, tout, 0, false).hyperliquid_seul());
        // both quotes summed; Hyperliquid has USDC only, Bitunix USDT only
        let both = c(None, false, tout, 3, false);
        assert_eq!(both.carnets.len(), 3 * 2 + 1 + 1);
        assert!(both.carnets.contains(&(Carnet::HyperliquidPerp, Usdc)) && !both.carnets.contains(&(Carnet::HyperliquidPerp, Usdt)));
        // USDT only with Hyperliquid ticked: Hyperliquid left out, said
        let usdt = c(None, false, [false, false, false, false, true], 1, false);
        assert!(usdt.hyperliquid_seul(), "nothing readable: Hyperliquid fallback");
        assert!(usdt.note.unwrap().contains("Hyperliquid : USDC seulement"));
        // the browser: Bitunix refuses it → Hyperliquid, said
        let web = c(Some((E::Bitunix, false)), true, tout, 0, true);
        assert!(web.hyperliquid_seul());
        assert!(web.note.as_deref().unwrap().contains("Bitunix bloqué sur le web"));
        assert_eq!(c(Some((E::Bitunix, false)), true, tout, 0, false).carnets, vec![(Carnet::BitunixPerp, Usdt)]);
        // an exchange without a book read by Argos: Hyperliquid, said
        let k = c(Some((E::Kraken, false)), true, tout, 0, false);
        assert!(k.hyperliquid_seul() && k.note.unwrap().contains("Kraken"));
        assert_eq!(
            titre_carnets(&c(None, false, [true, true, false, false, false], 1, false), 1),
            "Binance USDT · Bybit USDT 1/2"
        );
        // instruments of the USDC perps
        assert_eq!(Carnet::BybitPerp.instrument("btc", Usdc), "BTCPERP");
        assert_eq!(Carnet::OkxPerp.instrument("BTC", Usdc), "BTC-USDC-SWAP");
        assert_eq!(Carnet::BinancePerp.instrument("BTC", Usdc), "BTCUSDC");
        assert_eq!(Carnet::BinancePerp.instrument("BTC", Usdt), "BTCUSDT");
        assert_eq!(Carnet::HyperliquidPerp.instrument("BTC", Usdt), "BTC");
        let Request::Get(u) = Carnet::BybitPerp.request_q("BTC", 5, Usdc) else {
            panic!()
        };
        assert!(u.ends_with("symbol=BTCPERP&limit=500"));
        // summed into one book, grouped by tick, best first
        let v = |b: Vec<(f64, f64)>, a: Vec<(f64, f64)>| VenueBook {
            venue: "x".into(),
            bids: b,
            asks: a,
        };
        let s = somme(
            "BTC",
            &[
                v(vec![(59_990.0, 1.0), (59_950.0, 2.0)], vec![(60_010.0, 1.0)]),
                v(vec![(59_995.0, 3.0)], vec![(60_005.0, 0.5)]),
            ],
            10.0,
            7,
        );
        assert_eq!(s.bids.iter().map(|l| (l.px, l.sz)).collect::<Vec<_>>(), vec![(59_990.0, 4.0), (59_950.0, 2.0)]);
        assert_eq!(s.asks.iter().map(|l| (l.px, l.sz)).collect::<Vec<_>>(), vec![(60_010.0, 1.5)]);
    }
}
