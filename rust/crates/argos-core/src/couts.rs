//! 💸 Realistic costs of a demo order: the exchange's fees (maker / taker), the bid-ask spread and slippage.
//! Pure computation; each exchange's schedule lives in `argos_data::couts`.
//!
//! - A resting order (Limit entry, take profit, partial target) pays the **maker** fee, at its own price.
//! - An order that takes the book (Market entry, Stop entry, stop loss hit, manual exit) pays the **taker** fee,
//!   plus half the spread (it buys at the ask, sells at the bid) and the slippage.
//!
//! Spread and slippage are counted as a cost in money, like the fees: the entry and exit prices shown on the chart
//! stay the order's prices, so the R, the break-even and the trailing stop keep working on clean levels.

use crate::money::Money;

/// 💸 An exchange's costs, as fractions of the traded value (0.0005 = 0.05 %).
#[derive(Debug, Clone, Copy, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub struct Couts {
    /// Fee of an order that waits in the book (Limit, take profit).
    pub maker: f64,
    /// Fee of an order that takes the book (Market, Stop, stop loss hit, manual exit).
    pub taker: f64,
    /// Full bid-ask spread: an order that takes the book pays half of it.
    pub ecart: f64,
    /// Slippage of an order that takes the book (the price moves while it fills).
    pub glissement: f64,
}

/// 💸 What one fill pays: the exchange's fee, and (taking the book) half the spread plus the slippage.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Taux {
    pub frais: f64,
    pub glissement: f64,
}

/// A rate that makes sense: finite and positive, else 0.
fn propre(v: f64) -> f64 {
    if v.is_finite() && v > 0.0 {
        v
    } else {
        0.0
    }
}

impl Taux {
    /// Fee and spread + slippage of `qty` units at `prix`, each rounded once to the cent.
    pub fn montants(self, qty: f64, prix: f64) -> (Money, Money) {
        (Money::produit(&[qty, prix, self.frais]), Money::produit(&[qty, prix, self.glissement]))
    }

    /// Everything one fill of `qty` units at `prix` costs.
    pub fn total(self, qty: f64, prix: f64) -> Money {
        let (f, g) = self.montants(qty, prix);
        f + g
    }
}

impl Couts {
    /// « Sans frais »: nothing paid.
    pub const SANS: Couts = Couts {
        maker: 0.0,
        taker: 0.0,
        ecart: 0.0,
        glissement: 0.0,
    };

    /// What a fill pays: taker (`true`) or maker.
    pub fn taux(&self, taker: bool) -> Taux {
        if taker {
            Taux {
                frais: propre(self.taker),
                glissement: propre(self.ecart) / 2.0 + propre(self.glissement),
            }
        } else {
            Taux {
                frais: propre(self.maker),
                glissement: 0.0,
            }
        }
    }

    /// Nothing to pay at all.
    pub fn nul(&self) -> bool {
        self.taux(true) == Taux::default() && self.taux(false) == Taux::default()
    }
}

/// 💸 What a ticket would cost before it is sent: the entry, then the whole trade (entry + exit) if the stop loss is
/// hit, and if the take profit is.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Estimation {
    pub entree: Money,
    pub au_sl: Money,
    pub au_tp: Money,
}

/// 💸 Estimate for `qty` units entered at `entry` (`entree_taker`: Market or Stop order; Limit: maker), out at the
/// stop loss `sl` (taker: a stop order fills at market) or at the take profit `tp` (maker: a resting order).
pub fn estimer(c: &Couts, entree_taker: bool, qty: f64, entry: f64, sl: f64, tp: f64) -> Estimation {
    let entree = c.taux(entree_taker).total(qty, entry);
    Estimation {
        entree,
        au_sl: entree + c.taux(true).total(qty, sl),
        au_tp: entree + c.taux(false).total(qty, tp),
    }
}

/// « 0,05 % » / « 0,055 % »: a rate written as a short percentage (at most 4 decimals, no useless zeros).
pub fn pct_txt(v: f64) -> String {
    let t = format!("{:.4}", propre(v) * 100.0);
    let t = t.trim_end_matches('0').trim_end_matches('.');
    format!("{t} %").replace('.', ",")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Binance-like perps: maker 0.02 %, taker 0.05 %, spread 0.01 %, slippage 0.02 %.
    fn perp() -> Couts {
        Couts {
            maker: 0.0002,
            taker: 0.0005,
            ecart: 0.0001,
            glissement: 0.0002,
        }
    }

    #[test]
    fn maker_ou_taker() {
        let c = perp();
        assert_eq!(
            c.taux(false),
            Taux {
                frais: 0.0002,
                glissement: 0.0
            },
            "a resting order: maker, no slippage"
        );
        let t = c.taux(true);
        assert_eq!(t.frais, 0.0005);
        assert!((t.glissement - 0.00025).abs() < 1e-12, "half the spread plus the slippage");
        // 1 BTC at 100 000 $: 50 $ of fee, 25 $ of spread and slippage
        assert_eq!(t.montants(1.0, 100_000.0), (Money::from_cents(5_000), Money::from_cents(2_500)));
        assert!(Couts::SANS.nul() && !c.nul());
        // a broken rate costs nothing rather than paying back
        let bad = Couts {
            maker: -0.001,
            taker: f64::NAN,
            ecart: 0.0,
            glissement: f64::INFINITY,
        };
        assert!(bad.nul());
    }

    #[test]
    fn estimation_avant_envoi() {
        // market buy of 0.5 BTC at 100 000, SL 99 000, TP 102 000
        let e = estimer(&perp(), true, 0.5, 100_000.0, 99_000.0, 102_000.0);
        // entry: 0.5 × 100 000 × 0.075 % = 37.50 $
        assert_eq!(e.entree, Money::from_cents(3_750));
        // stop hit (taker): 0.5 × 99 000 × 0.075 % = 37.125 → 37.13 $
        assert_eq!(e.au_sl, Money::from_cents(3_750 + 3_713));
        // take profit (maker): 0.5 × 102 000 × 0.02 % = 10.20 $
        assert_eq!(e.au_tp, Money::from_cents(3_750 + 1_020));
        // a Limit entry pays the maker fee only
        assert_eq!(estimer(&perp(), false, 0.5, 100_000.0, 99_000.0, 102_000.0).entree, Money::from_cents(1_000));
        assert_eq!(estimer(&Couts::SANS, true, 0.5, 100_000.0, 99_000.0, 102_000.0), Estimation::default());
    }

    #[test]
    fn pourcentages_courts() {
        assert_eq!(pct_txt(0.0005), "0,05 %");
        assert_eq!(pct_txt(0.00055), "0,055 %");
        assert_eq!(pct_txt(0.006), "0,6 %");
        assert_eq!(pct_txt(0.0), "0 %");
    }
}
