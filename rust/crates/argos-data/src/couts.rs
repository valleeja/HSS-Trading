//! 💸 Each exchange's public base schedule (tier 0: no VIP level, no token discount, no referral): maker / taker fees
//! of the market Argos shows (perps, or spot for a spot venue), plus a typical spread and slippage for a liquid pair.
//!
//! Fees read in November 2026 from the exchanges' fee pages and fee comparisons; an exchange can change them at any
//! time. Spread and slippage are estimates (no exchange publishes them): tighter on the deepest books. These numbers
//! only feed the demo account, never a real order.

use crate::Exchange;
use argos_core::couts::Couts;

/// Percent → fraction (0.05 → 0.0005), written as the exchanges write them.
const fn pct(maker: f64, taker: f64, ecart: f64, glissement: f64) -> Couts {
    Couts {
        maker: maker / 100.0,
        taker: taker / 100.0,
        ecart: ecart / 100.0,
        glissement: glissement / 100.0,
    }
}

/// Deep perps books (Binance, OKX, Bybit, Bitget, Hyperliquid): spread and slippage of a liquid pair.
const PROFOND: (f64, f64) = (0.01, 0.01);
/// Thinner books: twice that.
const MINCE: (f64, f64) = (0.02, 0.02);

/// 💸 The base schedule of `e` for `base` (Hyperliquid HIP-3 markets such as « xyz:NVDA » have their own, higher
/// fees).
pub fn bareme(e: Exchange, base: &str) -> Couts {
    let (p, m) = (PROFOND, MINCE);
    match e {
        Exchange::Hyperliquid if base.contains(':') => pct(0.03, 0.09, m.0, m.1),
        Exchange::Hyperliquid => pct(0.015, 0.045, p.0, p.1),
        Exchange::Binance | Exchange::Okx => pct(0.02, 0.05, p.0, p.1),
        Exchange::Bybit => pct(0.02, 0.055, p.0, p.1),
        Exchange::Bitget => pct(0.02, 0.06, p.0, p.1),
        Exchange::Gate => pct(0.02, 0.05, m.0, m.1),
        Exchange::Bitunix | Exchange::Blofin | Exchange::Kucoin => pct(0.02, 0.06, m.0, m.1),
        Exchange::Weex => pct(0.02, 0.08, m.0, m.1),
        Exchange::Mexc => pct(0.0, 0.02, m.0, m.1),
        Exchange::Aster => pct(0.0, 0.04, m.0, m.1),
        Exchange::BinanceSpot | Exchange::BybitSpot | Exchange::BitgetSpot => pct(0.1, 0.1, p.0, p.1),
        Exchange::OkxSpot => pct(0.08, 0.1, p.0, p.1),
        // spot exchanges, lowest volume tier
        Exchange::Coinbase => pct(0.4, 0.6, m.0, m.1),
        Exchange::Kraken => pct(0.25, 0.4, m.0, m.1),
        Exchange::Bitstamp => pct(0.3, 0.4, m.0, m.1),
        // Bitfinex, lowest tier as its public fee schedule gives it (not re-read from its site on 23 Nov: an
        // estimate, like the others)
        Exchange::Bitfinex => pct(0.02, 0.065, m.0, m.1),
        Exchange::BitfinexSpot => pct(0.1, 0.2, m.0, m.1),
    }
}

/// 💸 OANDA (forex, metals, indices): no commission, the cost is in the spread.
pub const OANDA: Couts = pct(0.0, 0.0, 0.02, 0.0);

/// 💸 A DEX pool (GeckoTerminal pair): the pool fee on every swap (no resting order), and a wider slippage.
pub const DEX: Couts = pct(0.3, 0.3, 0.0, 0.3);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chaque_bourse_a_son_bareme() {
        for e in Exchange::ALL {
            let c = bareme(e, "BTC");
            assert!(!c.nul(), "{}: no costs", e.name());
            assert!(c.maker <= c.taker, "{}: maker above taker", e.name());
            assert!(c.taker < 0.01, "{}: taker above 1 %", e.name());
        }
        assert_eq!(bareme(Exchange::Binance, "BTC").taker, 0.0005);
        assert_eq!(bareme(Exchange::Hyperliquid, "BTC").maker, 0.00015);
        // HIP-3 costs more than the crypto perps
        assert!(bareme(Exchange::Hyperliquid, "xyz:NVDA").taker > bareme(Exchange::Hyperliquid, "BTC").taker);
        assert!(!OANDA.nul() && !DEX.nul());
    }
}
