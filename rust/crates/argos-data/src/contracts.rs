//! 📏 Contract sizes: how many coins one contract holds, on the perps that count in contracts (MEXC, BloFin, Gate.io).
//!
//! Shared data: the candles of MEXC and Gate.io (read and live) are turned into coins with them, and so are the trades
//! of the order flow (`whales`). Read by API (`whaletop::top` for a whole list, `ExchangeFeed::coins_per_contract`
//! for one coin) and kept here; without an answer, the size noted by hand.

use std::collections::HashMap;
use std::sync::Mutex;

/// The sizes read by API: « MEXC:PEPE » → coins in one contract.
static SIZES: Mutex<Option<HashMap<String, f64>>> = Mutex::new(None);

/// Keeps the contract sizes of an exchange (read with its top, or for one coin).
pub fn set_contracts(exchange: &str, rows: Vec<(String, f64)>) {
    if let Ok(mut m) = SIZES.lock() {
        let m = m.get_or_insert_with(Default::default);
        for (b, k) in rows {
            m.insert(format!("{exchange}:{b}"), k);
        }
    }
}

/// The size of one contract (in coins) of MEXC's, BloFin's and Gate.io's perps: the one read by API, else the one
/// noted on 2026-11-05 (Gate.io: on 2026-11-15).
pub(crate) fn contract(exchange: &str, coin: &str) -> Option<f64> {
    if let Some(k) = SIZES
        .lock()
        .ok()
        .and_then(|m| m.as_ref().and_then(|m| m.get(&format!("{exchange}:{coin}")).copied()))
    {
        return Some(k);
    }
    Some(match (exchange, coin) {
        ("MEXC", "BTC") => 0.0001,
        ("MEXC", "ETH") => 0.01,
        ("MEXC", "SOL") => 0.1,
        ("MEXC", "XRP") => 1.0,
        ("BloFin", "BTC") => 0.001,
        ("BloFin", "ETH") => 0.01,
        ("BloFin", "SOL") => 1.0,
        ("BloFin", "XRP") => 100.0,
        ("Gate.io", "BTC") => 0.0001,
        ("Gate.io", "ETH") => 0.01,
        ("Gate.io", "SOL") => 1.0,
        ("Gate.io", "XRP") => 10.0,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A size read by API comes before the one noted by hand; an exchange's size is its own; a coin never noted
    /// nor read has none (its volume is never shown in contracts as if they were coins).
    #[test]
    fn a_size_read_comes_before_the_one_noted() {
        assert_eq!(contract("MEXC", "BTC"), Some(0.0001), "noted by hand");
        assert_eq!(contract("MEXC", "ZZSIZEONE"), None);
        set_contracts("MEXC", vec![("ZZSIZEONE".into(), 0.5)]);
        assert_eq!(contract("MEXC", "ZZSIZEONE"), Some(0.5), "read by API");
        assert_eq!(contract("Gate.io", "ZZSIZEONE"), None, "the size of another exchange");
    }
}
