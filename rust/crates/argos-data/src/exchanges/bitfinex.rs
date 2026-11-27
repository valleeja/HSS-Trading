//! Bitfinex, spot and perpetuals, read only (Jee, 23 Nov 2026: « ok bitfinex », spot and perp). Public API v2
//! (`api-pub.bitfinex.com`, HTTPS and WSS, no key, never an order), checked live from the PC on 2026-11-23.
//!
//! What Bitfinex does its own way (each one seen on a real answer):
//! - symbols start with « t »; a name longer than 3 letters takes a colon: « tXRPBTC », « tDOGE:USD », « tAVAX:BTC »;
//! - perpetuals are « tXRPF0:USTF0 » (against USDT) and « tETHF0:BTCF0 » (against BTC);
//! - some codes are not the coin's usual name: UST = USDT, UDC = USDC, ALG = ALGO… (`pub:map:currency:sym`);
//! - a candle is `[MTS, OPEN, CLOSE, HIGH, LOW, VOLUME]`: the close comes **before** the high;
//! - no 4 h (nor 2 h, 3 min) candles: they are made from 1 h (1 min) candles ([`source_tf`]);
//! - book `[PRICE, COUNT, AMOUNT]`: amount > 0 is a bid, < 0 an ask; trades `[ID, MTS, AMOUNT, PRICE]`: amount > 0 is a
//!   market buy;
//! - the live stream sends `[CHAN_ID, "hb"]` heartbeats, ignored;
//! - low public limits (about 10 to 90 requests a minute): one request per screen, long pause between pages.
//!
//! Chart bases: spot « XRP » = XRP/USD, « XRP/BTC » = XRP/BTC; perpetual « XRP » = XRP/USDT, « ETH/BTC » = ETH/BTC.

use super::Request;
use crate::parse::{json, num};
use argos_core::feed::Market;
use argos_core::{ArgosError, Bar, Instrument, MarketKind, Result, Timeframe};
use serde_json::Value;

const REST: &str = "https://api-pub.bitfinex.com/v2";
const WS: &str = "wss://api-pub.bitfinex.com/ws/2";

/// Bitfinex code → usual name (from `pub:map:currency:sym`, the coins Argos shows).
const NOMS: [(&str, &str); 8] = [
    ("ALG", "ALGO"),
    ("DSH", "DASH"),
    ("IOT", "IOTA"),
    ("OPX", "OP"),
    ("UDC", "USDC"),
    ("UST", "USDT"),
    ("WBT", "WBTC"),
    ("LBT", "LBTC"),
];

/// The usual name of a Bitfinex code (« UST » → « USDT »).
pub fn nom(code: &str) -> String {
    NOMS.iter().find(|(c, _)| *c == code).map_or(code.to_string(), |(_, n)| n.to_string())
}

/// The Bitfinex code of a usual name (« USDT » → « UST »).
pub fn code(nom: &str) -> String {
    let n = nom.to_uppercase();
    NOMS.iter().find(|(_, x)| *x == n).map_or(n, |(c, _)| c.to_string())
}

/// « XRP » + « BTC » → « XRPBTC »; a name longer than 3 letters takes a colon (« DOGE:USD »).
fn paire(a: &str, q: &str) -> String {
    if a.len() > 3 || q.len() > 3 {
        format!("{a}:{q}")
    } else {
        format!("{a}{q}")
    }
}

/// The API symbol of a chart base: spot « XRP » → « tXRPUSD », « XRP/BTC » → « tXRPBTC »; perpetual « XRP » →
/// « tXRPF0:USTF0 », « ETH/BTC » → « tETHF0:BTCF0 ».
pub fn symbole(base: &str, perp: bool) -> String {
    let (a, q) = crate::paire::parts(base);
    let a = code(a);
    if perp {
        format!("t{a}F0:{}F0", code(q.unwrap_or("USDT")))
    } else {
        format!("t{}", paire(&a, &code(q.unwrap_or("USD"))))
    }
}

/// The ticker shown (« XRPBTC », « XRPUSD »; perpetual « XRPUSDT », « ETHBTC »).
pub fn ticker(base: &str, perp: bool) -> String {
    let (a, q) = crate::paire::parts(base);
    let q = q.unwrap_or(if perp { "USDT" } else { "USD" });
    format!("{}{}", a.to_uppercase(), q.to_uppercase())
}

/// The candle interval asked to Bitfinex for `tf`, and how many of them make one `tf` candle (4 h = 4 × 1 h).
pub fn source_tf(tf: Timeframe) -> (Timeframe, usize) {
    match tf {
        Timeframe::M3 => (Timeframe::M1, 3),
        Timeframe::H2 => (Timeframe::H1, 2),
        Timeframe::H4 => (Timeframe::H1, 4),
        t => (t, 1),
    }
}

fn tf_id(tf: Timeframe) -> Result<&'static str> {
    Ok(match tf {
        Timeframe::M1 => "1m",
        Timeframe::M5 => "5m",
        Timeframe::M15 => "15m",
        Timeframe::M30 => "30m",
        Timeframe::H1 => "1h",
        Timeframe::H12 => "12h",
        Timeframe::D1 => "1D",
        Timeframe::W1 => "1W",
        t => return Err(ArgosError::Feed(format!("Bitfinex : pas de bougies {} (faites depuis 1 h)", t.id()))),
    })
}

/// At most this many candles per request (Bitfinex's limit).
pub const PAGE: usize = 10_000;

/// Candles of `tf` (a native interval: see [`source_tf`]), the newest first; `end_ms`: only those before it.
pub fn candles_request(base: &str, perp: bool, tf: Timeframe, limit: usize, end_ms: Option<i64>) -> Result<Request> {
    let end = end_ms.map_or(String::new(), |e| format!("&end={e}"));
    Ok(Request::Get(format!(
        "{REST}/candles/trade:{}:{}/hist?limit={}&sort=-1{end}",
        tf_id(tf)?,
        symbole(base, perp),
        limit.clamp(1, PAGE)
    )))
}

/// An error answer: `["error", code, "message"]`.
fn erreur(v: &Value) -> Result<()> {
    if v[0] == "error" {
        return Err(ArgosError::Feed(format!("Bitfinex : {}", v[2].as_str().unwrap_or("erreur"))));
    }
    Ok(())
}

/// One candle `[MTS, OPEN, CLOSE, HIGH, LOW, VOLUME]` (the close before the high).
fn bougie(k: &Value) -> Option<Bar> {
    let a = k.as_array().filter(|a| a.len() >= 6)?;
    Some(Bar {
        time: crate::parse::secs(&a[0]),
        open: num(&a[1]),
        close: num(&a[2]),
        high: num(&a[3]),
        low: num(&a[4]),
        volume: num(&a[5]),
    })
}

/// The candles of an answer, oldest first.
pub fn parse_candles(body: &str) -> Result<Vec<Bar>> {
    let v = json(body)?;
    erreur(&v)?;
    let mut out: Vec<Bar> = v.as_array().map(|a| a.iter().filter_map(bougie).collect()).unwrap_or_default();
    out.sort_by_key(|b| b.time);
    Ok(out)
}

/// Every price at once (spot and perpetuals in one answer).
pub fn markets_request() -> Request {
    Request::Get(format!("{REST}/tickers?symbols=ALL"))
}

/// A Bitfinex symbol read back: (asset, quote, perpetual), usual names (« tXRPF0:USTF0 » → XRP, USDT, true).
pub fn lire_symbole(sym: &str) -> Option<(String, String, bool)> {
    let s = sym.strip_prefix('t')?;
    if let Some((a, q)) = s.split_once(':') {
        if let (Some(a), Some(q)) = (a.strip_suffix("F0"), q.strip_suffix("F0")) {
            return Some((nom(a), nom(q), true));
        }
        return Some((nom(a), nom(q), false));
    }
    // 6 letters without a colon: 3 + 3
    (s.len() == 6 && s.is_ascii()).then(|| (nom(&s[..3]), nom(&s[3..]), false))
}

/// The coin of a Bitfinex market id, for its badge (« bitfinex:XRPF0:USTF0 » → XRP, « bitfinex-spot:XRPBTC » →
/// XRP); `None`: not a Bitfinex id. (The last part after « : » was « USTF0 »: the badge said « U ».)
pub fn pastille(id: &str) -> Option<String> {
    let (ex, sym) = id.split_once(':')?;
    if ex != "bitfinex" && ex != "bitfinex-spot" {
        return None;
    }
    lire_symbole(&format!("t{sym}")).map(|(a, _, _)| a)
}

/// The chart base of a Bitfinex market, if Argos opens it: spot against USD or BTC, perpetuals against USDT or BTC.
pub fn base_de(sym: &str, perp_voulu: bool) -> Option<String> {
    let (a, q, perp) = lire_symbole(sym)?;
    if perp != perp_voulu || a.starts_with("TEST") || q.starts_with("TEST") {
        return None;
    }
    match q.as_str() {
        "BTC" if a != "BTC" => Some(crate::paire::base(&a, "BTC")),
        "USD" if !perp => Some(a),
        "USDT" if perp => Some(a),
        _ => None,
    }
}

/// `[[SYMBOL, BID, BID_SIZE, ASK, ASK_SIZE, DAILY_CHANGE, DAILY_CHANGE_RELATIVE, LAST_PRICE, VOLUME, HIGH, LOW], …]`
/// (funding rows « f… » are left out): the markets of the spot (`perp` false) or of the perpetuals.
pub fn parse_markets(body: &str, perp: bool) -> Result<Vec<Market>> {
    let v = json(body)?;
    erreur(&v)?;
    let rows = v.as_array().ok_or_else(|| ArgosError::Feed("Bitfinex : liste absente".into()))?;
    Ok(rows
        .iter()
        .filter_map(|t| {
            let base = base_de(t[0].as_str()?, perp)?;
            let (last, change) = (num(&t[7]), num(&t[5]));
            Some(Market {
                base,
                price: last,
                // without the change of the day: the price 24 h ago is not known (0, shown « — »), not a 0 % made up
                prev: if change.is_finite() { last - change } else { 0.0 },
            })
        })
        .collect())
}

/// The catalogue: the spot pairs, the perpetuals, and every price (for the decimals: Bitfinex shows 5 significant
/// digits, at most 8 decimals).
pub fn instruments_requests() -> [Request; 3] {
    [
        Request::Get(format!("{REST}/conf/pub:list:pair:exchange")),
        Request::Get(format!("{REST}/conf/pub:list:pair:futures")),
        markets_request(),
    ]
}

/// Decimals of a price with 5 significant digits (61 721 → 0; 0.00002345 → 8 at most).
pub fn decimales(prix: f64) -> u32 {
    if !(prix.is_finite() && prix > 0.0) {
        return 8;
    }
    let chiffres = prix.log10().floor() as i32 + 1;
    (5 - chiffres).clamp(0, 8) as u32
}

/// The catalogue from the three answers of [`instruments_requests`]: `bitfinex-spot:XRPBTC`, `bitfinex:XRPF0:USTF0`…
pub fn parse_instruments(spot: &str, futures: &str, tickers: &str) -> Result<Vec<Instrument>> {
    let liste = |body: &str| -> Result<Vec<String>> {
        let v = json(body)?;
        erreur(&v)?;
        Ok(v[0]
            .as_array()
            .map(|a| a.iter().filter_map(|s| s.as_str().map(str::to_string)).collect())
            .unwrap_or_default())
    };
    let prix: std::collections::HashMap<String, f64> = json(tickers)
        .ok()
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default()
        .iter()
        .filter_map(|t| Some((t[0].as_str()?.to_string(), num(&t[7]))))
        .collect();
    let mut out = vec![];
    for (body, perp) in [(spot, false), (futures, true)] {
        for p in liste(body)? {
            let sym = format!("t{p}");
            let Some(base) = base_de(&sym, perp) else { continue };
            let (a, q) = crate::paire::parts(&base);
            let quote = q.unwrap_or(if perp { "USDT" } else { "USD" }).to_string();
            let group = if perp && a.ends_with("IX") { "Indices" } else { "Crypto" };
            out.push(Instrument {
                exchange: if perp { "bitfinex" } else { "bitfinex-spot" }.into(),
                symbol: p,
                base: a.to_string(),
                quote,
                kind: if perp { MarketKind::Perp } else { MarketKind::Spot },
                group: group.into(),
                price_decimals: decimales(prix.get(&sym).copied().unwrap_or(f64::NAN)),
                size_decimals: 8,
                growth: false,
                active: true,
            });
        }
    }
    out.sort_by(|a, b| (&a.exchange, &a.symbol).cmp(&(&b.exchange, &b.symbol)));
    out.dedup_by(|a, b| a.exchange == b.exchange && a.symbol == b.symbol);
    Ok(out)
}

/// The order book (100 levels each side, exact prices).
pub fn book_request(base: &str, perp: bool) -> Request {
    Request::Get(format!("{REST}/book/{}/P0?len=100", symbole(base, perp)))
}

/// `[[PRICE, COUNT, AMOUNT], …]`: amount > 0 a bid, < 0 an ask; bids from the best down, asks from the best up.
pub fn parse_book(body: &str, coin: &str, time_ms: i64) -> Result<crate::book::Book> {
    let v = json(body)?;
    erreur(&v)?;
    let rows = v.as_array().ok_or_else(|| ArgosError::Feed("Bitfinex : carnet absent".into()))?;
    let mut b = crate::book::Book {
        coin: coin.to_string(),
        time_ms,
        ..Default::default()
    };
    for r in rows {
        let (px, n, amount) = (num(&r[0]), num(&r[1]), num(&r[2]));
        // 🧹 a level that cannot be (a price of zero or under, a size of zero, NaN, infinite) is left out
        if !argos_core::feed::price_size_ok(px, amount.abs()) {
            continue;
        }
        let l = crate::book::Level {
            px,
            sz: amount.abs(),
            n: if n.is_finite() { n as u32 } else { 0 },
        };
        if amount > 0.0 {
            b.bids.push(l);
        } else {
            b.asks.push(l);
        }
    }
    b.bids.sort_by(|x, y| y.px.total_cmp(&x.px));
    b.asks.sort_by(|x, y| x.px.total_cmp(&y.px));
    Ok(b)
}

/// The last trades (100 at most).
pub fn trades_request(base: &str, perp: bool) -> Request {
    Request::Get(format!("{REST}/trades/{}/hist?limit=100", symbole(base, perp)))
}

/// `[[ID, MTS, AMOUNT, PRICE], …]`: amount > 0 is a market buy; the newest first.
pub fn parse_trades(body: &str, coin: &str) -> Result<Vec<crate::whales::Whale>> {
    let v = json(body)?;
    erreur(&v)?;
    let mut out: Vec<crate::whales::Whale> = v
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|t| {
                    let (amount, price) = (num(&t[2]), num(&t[3]));
                    // 🧹 a trade that cannot be (a price of zero or under, a size of zero, NaN, infinite) is left out
                    argos_core::feed::price_size_ok(price, amount.abs()).then(|| crate::whales::Whale {
                        coin: coin.to_string(),
                        time_ms: t[1].as_i64().unwrap_or(0),
                        price,
                        size: amount.abs(),
                        buy: amount > 0.0,
                        src: "Bitfinex",
                        wallet: String::new(),
                        id: t[0].as_u64(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    out.sort_by_key(|w| -w.time_ms);
    Ok(out)
}

/// A perpetual's figures (`status/deriv`): mark price, current funding rate, open interest (in coins).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Statut {
    pub mark: f64,
    pub funding: f64,
    pub open_interest: f64,
    /// When the next funding is paid (ms UTC; 0: unknown).
    pub next_funding_ms: i64,
}

/// « 0.0653 % · 03:12:45 »: the current funding rate (Bitfinex gives a fraction) and the time left before its next
/// payment (`now_ms`); « — » for the time when Bitfinex did not say it.
pub fn funding_texte(s: &Statut, now_ms: i64) -> String {
    let taux = format!("{:.4} %", s.funding * 100.0);
    if s.next_funding_ms <= 0 {
        return format!("{taux} · —");
    }
    let r = ((s.next_funding_ms - now_ms).max(0) / 1000) as u64;
    format!("{taux} · {:02}:{:02}:{:02}", r / 3600, r % 3600 / 60, r % 60)
}

/// The figures of one perpetual.
pub fn status_request(base: &str) -> Request {
    Request::Get(format!("{REST}/status/deriv?keys={}", symbole(base, true)))
}

/// `[[KEY, MTS, _, DERIV_PRICE, SPOT_PRICE, _, INSURANCE, _, NEXT_FUNDING_TS, NEXT_FUNDING_ACCRUED, STEP, _,
/// CURRENT_FUNDING, _, _, MARK_PRICE, _, _, OPEN_INTEREST, …]]`.
pub fn parse_status(body: &str) -> Result<Option<Statut>> {
    let v = json(body)?;
    erreur(&v)?;
    Ok(v.get(0).filter(|r| r.is_array()).map(|r| Statut {
        mark: num(&r[15]),
        funding: num(&r[12]),
        open_interest: num(&r[18]),
        next_funding_ms: r[8].as_i64().unwrap_or(0),
    }))
}

/// Live candles (channel `candles`, key « trade:1h:tXRPBTC »); an interval Bitfinex does not have (4 h, 2 h, 3 min:
/// made from 1 h / 1 min candles) follows its live **trades** instead ([`trades_sub`]), put into the last candle by the
/// chart (`live::fold_trade`), so the last 4 h candle moves too.
pub fn live_sub(base: &str, perp: bool, tf: Timeframe) -> Option<crate::live::LiveSub> {
    let Ok(id) = tf_id(tf) else {
        return Some(trades_sub(base, perp));
    };
    Some(crate::live::LiveSub {
        url: WS.into(),
        subscribe: format!(r#"{{"event":"subscribe","channel":"candles","key":"trade:{id}:{}"}}"#, symbole(base, perp)),
        ping: r#"{"event":"ping","cid":1}"#.into(),
        ..Default::default()
    })
}

/// Live trades of one market (channel `trades`; its messages carry no symbol: one market per connection).
pub fn trades_sub(base: &str, perp: bool) -> crate::live::LiveSub {
    crate::live::LiveSub {
        url: WS.into(),
        subscribe: format!(r#"{{"event":"subscribe","channel":"trades","symbol":"{}"}}"#, symbole(base, perp)),
        ping: r#"{"event":"ping","cid":1}"#.into(),
        ..Default::default()
    }
}

/// A new trade of the live stream: `[CHAN_ID, "te", [ID, MTS, AMOUNT, PRICE]]`. The first message (the last trades,
/// already in the downloaded candles), the « tu » copies, heartbeats and events give nothing.
pub fn parse_live_trades(msg: &str) -> Vec<crate::live::Trade> {
    let Ok(v) = json(msg) else { return vec![] };
    if v[1] != "te" {
        return vec![];
    }
    let t = &v[2];
    let (mts, amount, price) = (t[1].as_i64(), num(&t[2]), num(&t[3]));
    match mts {
        Some(time_ms) if amount.is_finite() && price.is_finite() && price > 0.0 && amount != 0.0 => vec![crate::live::Trade {
            time_ms,
            price,
            size: amount.abs(),
        }],
        _ => vec![],
    }
}

/// A live candle: `[CHAN_ID, [MTS, O, C, H, L, V]]` (an update); the first message (a list of candles) gives the newest
/// one; `[CHAN_ID, "hb"]` (heartbeat) and the events (`{"event":…}`) give nothing.
pub fn parse_live(msg: &str) -> Option<Bar> {
    let v = json(msg).ok()?;
    let d = v.as_array()?.get(1)?;
    let k = if d[0].is_array() {
        d.as_array()?.iter().max_by_key(|k| k[0].as_i64().unwrap_or(0))?
    } else {
        d
    };
    crate::parse::valid(bougie(k)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 🔒 Reliable data (Jee, 23 Nov: « données affichées fiables »): Bitfinex's real answers of 23 Nov 2026 17:20 UTC,
    /// read as Bitfinex gives them. The book of XRPBTC: a positive amount is a bid, a negative one an ask (its size
    /// the absolute amount, in XRP), prices in BTC kept whole (8 decimals), the spread from the two best prices.
    /// The XRPUSDT perpetual (`status/deriv`): mark at index 15, the current funding (a fraction, × 100 for %) at 12,
    /// the open interest in XRP (contracts of 1 XRP) at 18, the next payment's time (ms) at 8; the countdown counts
    /// from now to it.
    #[test]
    fn donnees_fiables_carnet_et_statut() {
        let b = parse_book("[[0.00001566,2,7588.09061214],[0.00001567,1,-2060.09889859]]", "XRP/BTC", 1).unwrap();
        assert_eq!((b.bids.len(), b.asks.len()), (1, 1));
        assert_eq!((b.bids[0].px, b.bids[0].sz, b.bids[0].n), (0.00001566, 7588.09061214, 2));
        assert_eq!((b.asks[0].px, b.asks[0].sz, b.asks[0].n), (0.00001567, 2060.09889859, 1));
        let (ecart, _) = b.spread().unwrap();
        assert!((ecart - 0.00000001).abs() < 1e-15, "{ecart}");
        let s = parse_status(
            r#"[["tXRPF0:USTF0",1795454314000,null,0.97122,0.97145,null,59332707.22187445,null,1795478400000,-0.00001989,1500,null,0,null,null,0.971198,null,null,2274368.17012306,null,null,null,0.0005,0.0025]]"#,
        )
        .unwrap()
        .unwrap();
        assert_eq!(
            (s.mark, s.funding, s.open_interest, s.next_funding_ms),
            (0.971198, 0.0, 2274368.17012306, 1795478400000)
        );
        // the countdown: 6 h 40 min 0 s before the payment, then 0 once it is past (never negative)
        assert_eq!(funding_texte(&s, 1795478400000 - (6 * 3600 + 40 * 60) * 1000), "0.0000 % · 06:40:00");
        assert_eq!(funding_texte(&s, 1795478400000 + 5_000), "0.0000 % · 00:00:00");
        let neg = Statut { funding: -0.000125, ..s };
        assert!(funding_texte(&neg, 0).starts_with("-0.0125 %"), "the sign of a negative funding");
    }

    /// 🧹 A level or a trade that cannot be (a price of zero or under, a size of zero, NaN) is left out by both readers
    /// (`argos_core::feed::price_size_ok`), and what came with it stays; the real book of 23 Nov is still read whole.
    #[test]
    fn a_zero_or_negative_price_is_refused() {
        let b = parse_book(
            "[[0.00001566,2,7588.09061214],[0,1,5],[-0.00001,1,5],[0.00001565,1,0],[0.00001567,1,-2060.09889859],[0,1,-3],[-0.00002,1,-3]]",
            "XRP/BTC",
            1,
        )
        .unwrap();
        assert_eq!((b.bids.len(), b.asks.len()), (1, 1));
        assert_eq!((b.bids[0].px, b.asks[0].px), (0.00001566, 0.00001567));
        let real = parse_book("[[0.00001566,2,7588.09061214],[0.00001567,1,-2060.09889859]]", "XRP/BTC", 1).unwrap();
        assert_eq!((real.bids.len(), real.asks.len()), (1, 1), "the real book is read whole");
        let t = parse_trades(
            "[[1,1795454314000,12.5,0.00001566],[2,1795454313000,3,0],[3,1795454312000,-4,-0.00001566],[4,1795454311000,0,0.00001566],[5,1795454310000,-2,0.00001567]]",
            "XRP/BTC",
        )
        .unwrap();
        assert_eq!(
            t.iter().map(|w| (w.id, w.price, w.size, w.buy)).collect::<Vec<_>>(),
            vec![(Some(1), 0.00001566, 12.5, true), (Some(5), 0.00001567, 2.0, false)]
        );
    }

    /// Real answers of 2026-11-23 (shortened).
    #[test]
    fn bougie_cloture_avant_haut() {
        let b = parse_candles("[[1795413600000,61778,61721,61888,61703,1.71504272],[1795410000000,61321,61786,61945,61186,16.4629805]]").unwrap();
        assert_eq!(b.len(), 2);
        assert_eq!(b[0].time, 1_795_410_000, "oldest first");
        let n = &b[1];
        assert_eq!((n.open, n.close, n.high, n.low, n.volume), (61778.0, 61721.0, 61888.0, 61703.0, 1.71504272));
        assert!(parse_candles(r#"["error",10020,"symbol: invalid"]"#)
            .unwrap_err()
            .to_string()
            .contains("invalid"));
    }

    #[test]
    fn symboles_courts_longs_et_perps() {
        assert_eq!(symbole("XRP/BTC", false), "tXRPBTC");
        assert_eq!(symbole("XRP", false), "tXRPUSD");
        assert_eq!(symbole("DOGE/BTC", false), "tDOGE:BTC");
        assert_eq!(symbole("AVAX", false), "tAVAX:USD");
        assert_eq!(symbole("XRP", true), "tXRPF0:USTF0");
        assert_eq!(symbole("ETH/BTC", true), "tETHF0:BTCF0");
        assert_eq!(symbole("ALGO", true), "tALGF0:USTF0", "Bitfinex code");
        assert_eq!(lire_symbole("tXRPF0:USTF0"), Some(("XRP".into(), "USDT".into(), true)), "UST = USDT");
        assert_eq!(lire_symbole("tDOGE:BTC"), Some(("DOGE".into(), "BTC".into(), false)));
        assert_eq!(base_de("tXRPF0:USTF0", true).as_deref(), Some("XRP"), "perp against USDT");
        assert_eq!(base_de("tETHF0:BTCF0", true).as_deref(), Some("ETH/BTC"));
        assert_eq!(base_de("tXRPBTC", false).as_deref(), Some("XRP/BTC"));
        assert_eq!(base_de("tXRPUST", false), None, "spot against USDT: the USD pair is the one shown");
        assert_eq!(base_de("tTESTBTC:TESTUSD", false), None);
        assert_eq!(pastille("bitfinex:XRPF0:USTF0").as_deref(), Some("XRP"), "the coin, not « U »");
        assert_eq!(pastille("bitfinex-spot:XRPBTC").as_deref(), Some("XRP"));
        assert_eq!(pastille("bitfinex-spot:DOGE:BTC").as_deref(), Some("DOGE"));
        assert_eq!(pastille("binance:XRPUSDT"), None);
        assert_eq!(ticker("XRP/BTC", false), "XRPBTC");
        assert_eq!(ticker("XRP", true), "XRPUSDT");
        assert_eq!(
            candles_request("XRP/BTC", false, Timeframe::H1, 50, Some(1000)).unwrap(),
            Request::Get(format!("{REST}/candles/trade:1h:tXRPBTC/hist?limit=50&sort=-1&end=1000"))
        );
        assert!(candles_request("XRP", false, Timeframe::H4, 50, None).is_err(), "no 4 h at Bitfinex");
        assert_eq!(source_tf(Timeframe::H4), (Timeframe::H1, 4));
        assert_eq!(source_tf(Timeframe::H1), (Timeframe::H1, 1));
    }

    #[test]
    fn prix_et_catalogue() {
        let tickers = r#"[["tBTCUSD",61714,2.7,61728,4.6,2473,0.0417,61712,1211.9,62017,59054],["tXRPBTC",0.0000233,1,0.0000234,1,0.0000001,0.004,0.00002345,10,0.0000235,0.000023],["tXRPF0:USTF0",1.43,1,1.44,1,0.01,0.007,1.4321,5,1.5,1.4],["fUSD",0.0001,30,5,1,2,0,0,0,0,0,0,0,0,0,0],["tXRPUST",1,1,1,1,0,0,1.43,1,1,1]]"#;
        let s = parse_markets(tickers, false).unwrap();
        assert_eq!(s.iter().map(|m| m.base.as_str()).collect::<Vec<_>>(), ["BTC", "XRP/BTC"]);
        assert!((s[1].prev - 0.00002335).abs() < 1e-12, "prev = last - daily change");
        let no_change = parse_markets(r#"[["tBTCUSD",61714,2.7,61728,4.6,null,null,61712,1211.9,62017,59054]]"#, false).unwrap();
        assert_eq!(no_change[0].change_pct(), None, "no change given: « — », not 0 %");
        let p = parse_markets(tickers, true).unwrap();
        assert_eq!(p.iter().map(|m| m.base.as_str()).collect::<Vec<_>>(), ["XRP"]);
        let i = parse_instruments(
            r#"[["BTCUSD","XRPBTC","XRPUST","DOGE:BTC","TESTBTC:TESTUSD"]]"#,
            r#"[["XRPF0:USTF0","ETHF0:BTCF0","JAPAN225IXF0:USTF0"]]"#,
            tickers,
        )
        .unwrap();
        let ids: Vec<String> = i.iter().map(|x| x.id()).collect();
        assert_eq!(
            ids,
            [
                "bitfinex:ETHF0:BTCF0",
                "bitfinex:JAPAN225IXF0:USTF0",
                "bitfinex:XRPF0:USTF0",
                "bitfinex-spot:BTCUSD",
                "bitfinex-spot:DOGE:BTC",
                "bitfinex-spot:XRPBTC"
            ]
        );
        let x = i.iter().find(|x| x.symbol == "XRPBTC").unwrap();
        assert_eq!(
            (x.base.as_str(), x.quote.as_str(), x.kind, x.price_decimals),
            ("XRP", "BTC", MarketKind::Spot, 8)
        );
        let p = i.iter().find(|x| x.symbol == "XRPF0:USTF0").unwrap();
        assert_eq!((p.quote.as_str(), p.kind, p.price_decimals), ("USDT", MarketKind::Perp, 4));
        assert_eq!(i.iter().find(|x| x.symbol.starts_with("JAPAN")).unwrap().group, "Indices");
        assert_eq!(decimales(61721.0), 0);
        assert_eq!(decimales(0.00002345), 8);
    }

    #[test]
    fn carnet_signe_trades_statut_direct() {
        let b = parse_book("[[61708,1,0.023],[61707,2,0.5],[61720,1,-0.0001],[61725,3,-2]]", "XRP/BTC", 7).unwrap();
        assert_eq!(
            b.bids.iter().map(|l| l.px).collect::<Vec<_>>(),
            [61708.0, 61707.0],
            "amount > 0: bids, best first"
        );
        assert_eq!(b.asks.iter().map(|l| (l.px, l.sz)).collect::<Vec<_>>(), [(61720.0, 0.0001), (61725.0, 2.0)]);
        assert_eq!(b.bids[1].n, 2);
        let t = parse_trades("[[2017441698,1795413822520,0.00341847,61721],[2017441697,1795413822519,-0.0001,61720]]", "BTC").unwrap();
        assert_eq!((t[0].buy, t[1].buy, t[1].size, t[0].id), (true, false, 0.0001, Some(2017441698)));
        let s = parse_status(r#"[["tGBPF0:USTF0",1795413821000,null,1.3176,1.317,null,59671310.35,null,1795420800000,0.00213555,6965,null,0.0025,null,null,1.314298041222,null,null,706.01206086]]"#)
            .unwrap()
            .unwrap();
        assert_eq!((s.mark, s.funding, s.open_interest), (1.314298041222, 0.0025, 706.01206086));
        assert_eq!(s.next_funding_ms, 1_795_420_800_000);
        // 1 h 2 min 3 s before the next payment (real answer of the PC, 23 Nov: 0.00065314 for BTC)
        let btc = Statut { funding: 0.00065314, ..s };
        assert_eq!(funding_texte(&btc, 1_795_420_800_000 - 3_723_000), "0.0653 % · 01:02:03");
        assert_eq!(funding_texte(&Statut { next_funding_ms: 0, ..s }, 0), "0.2500 % · —");
        let l = live_sub("XRP/BTC", false, Timeframe::H1).unwrap();
        assert!(l.url.starts_with("wss://api-pub.bitfinex.com") && l.subscribe.contains(r#""key":"trade:1h:tXRPBTC""#));
        // 4 h: its live trades, put into the last 4 h candle (real « te » message of the PC, 23 Nov)
        let q = live_sub("XRP/BTC", false, Timeframe::H4).unwrap();
        assert!(q.subscribe.contains(r#""channel":"trades","symbol":"tXRPBTC""#));
        let t = parse_live_trades(r#"[318,"te",[1891234567,1795424162000,-12.5,0.00001531]]"#);
        assert_eq!(
            t,
            vec![crate::live::Trade {
                time_ms: 1_795_424_162_000,
                price: 0.00001531,
                size: 12.5
            }]
        );
        assert!(
            parse_live_trades(r#"[318,"tu",[1891234567,1795424162000,-12.5,0.00001531]]"#).is_empty(),
            "copy"
        );
        assert!(
            parse_live_trades(r#"[318,[[1891234567,1795424162000,-12.5,0.00001531]]]"#).is_empty(),
            "snapshot: already counted"
        );
        assert!(parse_live_trades(r#"[318,"hb"]"#).is_empty());
        assert_eq!(
            parse_live(r#"[318,"te",[1891234567,1795424162000,-12.5,0.00001531]]"#),
            None,
            "a trade is not a candle"
        );
        assert_eq!(parse_live(r#"[318,[[1891234567,1795424162000,-12.5,0.00001531]]]"#), None);
        let mut bars = vec![Bar {
            time: 1_795_420_800,
            open: 1.0,
            high: 2.0,
            low: 0.5,
            close: 1.5,
            volume: 10.0,
        }];
        crate::live::fold_trade(
            &mut bars,
            crate::live::Trade {
                time_ms: 1_795_424_162_000,
                price: 2.5,
                size: 1.0,
            },
            14_400,
        );
        assert_eq!(
            (bars.len(), bars[0].high, bars[0].close, bars[0].volume),
            (1, 2.5, 2.5, 11.0),
            "the 4 h candle moves"
        );
        assert_eq!(parse_live(r#"[17470,"hb"]"#), None, "heartbeat ignored");
        assert_eq!(parse_live(r#"{"event":"info","version":2}"#), None);
        let b = parse_live("[17470,[1795413600000,1,1.5,2,0.5,10]]").unwrap();
        assert_eq!((b.open, b.close, b.high, b.low), (1.0, 1.5, 2.0, 0.5));
        let b = parse_live("[17470,[[1795413600000,1,1.5,2,0.5,10],[1795410000000,1,1,1,1,1]]]").unwrap();
        assert_eq!(b.time, 1_795_413_600, "the snapshot's newest candle");
    }
}
