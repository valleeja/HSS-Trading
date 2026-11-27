//! ⚡ Perpetual futures of the demo account: leverage, isolated margin, liquidation price, funding.
//! Pure computation; each exchange's limits (maximum leverage, maintenance margin, funding interval) live in
//! `argos_data::perp`.
//!
//! Isolated margin: each position has its own margin (its value divided by the leverage), taken out of the available
//! balance while it is open. A position can lose its margin and nothing more: when the price reaches the
//! liquidation price, the exchange closes it and the whole margin is gone. Linear contracts (margined and settled in
//! dollars, like BTCUSDT), as on the exchanges Argos shows.

use crate::money::Money;

/// ⚡ The perpetual terms of a position: leverage, maintenance margin rate and funding interval.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Perp {
    /// Leverage (1 to 125): the position is worth `levier` times its margin.
    pub levier: f64,
    /// Maintenance margin rate (0.005 = 0.5 %): when the margin left falls to it, the position is liquidated.
    pub maintenance: f64,
    /// Hours between two funding payments (8 on most exchanges, 1 on Hyperliquid).
    pub funding_h: f64,
}

/// ⚡ Leverages offered by the ticket, before the exchange's own maximum (see [`leviers`]): six at most, so the row
/// holds on one line, on a phone too.
pub const LEVIERS: [f64; 6] = [1.0, 5.0, 10.0, 20.0, 50.0, 100.0];

/// ⚡ The leverages the ticket offers on a market whose maximum is `max`: the usual ones below it, then the maximum
/// itself (Hyperliquid BTC ×40: ×1 ×5 ×10 ×20 ×40).
pub fn leviers(max: f64) -> Vec<f64> {
    let max = if max.is_finite() { max.clamp(1.0, 125.0) } else { 1.0 };
    let mut v: Vec<f64> = LEVIERS.iter().copied().filter(|l| *l < max).collect();
    v.push(max);
    v
}

/// ⚡ A perpetual market's terms on its exchange: maximum leverage, maintenance margin rate, hours between two
/// funding payments.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bareme {
    pub levier_max: f64,
    pub maintenance: f64,
    pub funding_h: f64,
}

impl Bareme {
    /// The terms of a position at leverage `levier` (capped at the market's maximum).
    pub fn perp(&self, levier: f64) -> Perp {
        Perp::new(levier.min(self.levier_max), self.maintenance, self.funding_h)
    }
}

/// ⚡ Funding rate used when an exchange's real rate is not known: 0.01 % every 8 hours, the base rate of most
/// exchanges when perpetual and spot prices agree.
pub const TAUX_STANDARD_8H: f64 = 0.0001;

impl Perp {
    /// Terms that make sense: leverage between 1 and 125, maintenance between 0 and half the initial margin, funding
    /// interval of at least 1 hour.
    pub fn new(levier: f64, maintenance: f64, funding_h: f64) -> Perp {
        let levier = if levier.is_finite() { levier.clamp(1.0, 125.0) } else { 1.0 };
        let maintenance = if maintenance.is_finite() { maintenance.clamp(0.0, 0.5 / levier) } else { 0.0 };
        let funding_h = if funding_h.is_finite() && funding_h >= 1.0 { funding_h } else { 8.0 };
        Perp {
            levier,
            maintenance,
            funding_h,
        }
    }

    /// 💰 Isolated margin of `qty` units at `prix`: their value divided by the leverage.
    pub fn marge(&self, qty: f64, prix: f64) -> Money {
        Money::produit(&[qty, prix, 1.0 / self.levier.max(1.0)])
    }

    /// 💀 Liquidation price of a position entered at `entry` (buy: below the entry, sell: above): where the loss
    /// eats the margin down to the maintenance margin. Long: entry × (1 − 1/leverage + maintenance); short:
    /// entry × (1 + 1/leverage − maintenance).
    pub fn liquidation(&self, buy: bool, entry: f64) -> f64 {
        let l = self.levier.max(1.0);
        if buy {
            entry * (1.0 - 1.0 / l + self.maintenance)
        } else {
            entry * (1.0 + 1.0 / l - self.maintenance)
        }
    }

    /// ⚡ The funding rate of one payment from a rate given for 8 hours (Hyperliquid pays every hour: an eighth).
    pub fn taux_par_paiement(&self, taux_8h: f64) -> f64 {
        taux_8h * self.funding_h / 8.0
    }
}

/// ⚡ One funding payment of a position worth `qty` × `prix`: a positive rate makes the buyers pay the sellers.
/// Returns what the position pays (negative: what it receives), rounded once to the cent.
pub fn funding(buy: bool, qty: f64, prix: f64, taux: f64) -> Money {
    let sens = if buy { 1.0 } else { -1.0 };
    if !(taux.is_finite() && qty.is_finite() && prix.is_finite()) {
        return Money::ZERO;
    }
    Money::produit(&[qty, prix, taux, sens])
}

/// ⚡ Funding times after `depuis` and up to `jusqua` (seconds, UTC), every `toutes_h` hours from midnight UTC
/// (8 hours: 00:00, 08:00, 16:00). At most 1 000 (a position left open for years does not freeze the app).
pub fn echeances(depuis: i64, jusqua: i64, toutes_h: f64) -> Vec<i64> {
    let pas = (toutes_h.max(1.0) * 3600.0).round() as i64;
    let mut t = depuis.div_euclid(pas) * pas + pas;
    let mut v = vec![];
    while t <= jusqua && v.len() < 1_000 {
        v.push(t);
        t += pas;
    }
    v
}

/// « ×10 » / « ×2,5 »: a leverage written short.
pub fn levier_txt(l: f64) -> String {
    let t = format!("{l:.1}");
    format!("×{}", t.trim_end_matches('0').trim_end_matches('.')).replace('.', ",")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marge_et_liquidation() {
        // 1 BTC at 100 000 $, ×10, maintenance 0.5 %
        let p = Perp::new(10.0, 0.005, 8.0);
        assert_eq!(p.marge(1.0, 100_000.0), Money::from_cents(1_000_000));
        // long: 100 000 × (1 − 0.1 + 0.005) = 90 500; short: 109 500
        assert!((p.liquidation(true, 100_000.0) - 90_500.0).abs() < 1e-6);
        assert!((p.liquidation(false, 100_000.0) - 109_500.0).abs() < 1e-6);
        // ×1: a long is only liquidated near zero
        assert!(Perp::new(1.0, 0.005, 8.0).liquidation(true, 100.0) < 1.0);
        // nonsense is made safe
        let x = Perp::new(f64::NAN, 2.0, 0.0);
        assert_eq!((x.levier, x.funding_h), (1.0, 8.0));
        assert!(x.maintenance <= 0.5);
        assert_eq!(Perp::new(500.0, 0.004, 8.0).levier, 125.0);
    }

    #[test]
    fn funding_paye_ou_recu() {
        // 2 BTC at 50 000 $, rate 0.01 %: the long pays 10 $, the short receives them
        assert_eq!(funding(true, 2.0, 50_000.0, 0.0001), Money::from_cents(1_000));
        assert_eq!(funding(false, 2.0, 50_000.0, 0.0001), Money::from_cents(-1_000));
        // a negative rate: the short pays
        assert_eq!(funding(false, 2.0, 50_000.0, -0.0001), Money::from_cents(1_000));
        assert_eq!(funding(true, 1.0, 1.0, f64::NAN), Money::ZERO);
        // Hyperliquid pays every hour: an eighth of the 8-hour rate
        assert!((Perp::new(5.0, 0.01, 1.0).taux_par_paiement(0.0008) - 0.0001).abs() < 1e-12);
    }

    #[test]
    fn echeances_de_8_heures() {
        // 2026-11-20 07:30 UTC → 17:00: the 08:00 and 16:00 payments
        let jour = 1_795_132_800; // 2026-11-20 00:00 UTC
        let v = echeances(jour + 7 * 3600 + 1800, jour + 17 * 3600, 8.0);
        assert_eq!(v, vec![jour + 8 * 3600, jour + 16 * 3600]);
        // exactly on a payment time: that one is already done
        assert_eq!(echeances(jour + 8 * 3600, jour + 9 * 3600, 8.0), Vec::<i64>::new());
        assert_eq!(echeances(jour, jour + 3 * 3600, 1.0).len(), 3);
        assert_eq!(echeances(0, i64::MAX / 2, 1.0).len(), 1_000);
        assert_eq!(levier_txt(10.0), "×10");
        assert_eq!(leviers(40.0), vec![1.0, 5.0, 10.0, 20.0, 40.0]);
        assert_eq!(leviers(125.0).last(), Some(&125.0));
        assert_eq!(leviers(1.0), vec![1.0]);
        let b = Bareme {
            levier_max: 20.0,
            maintenance: 0.005,
            funding_h: 1.0,
        };
        assert_eq!((b.perp(50.0).levier, b.perp(5.0).levier, b.perp(5.0).funding_h), (20.0, 5.0, 1.0));
        assert_eq!(levier_txt(2.5), "×2,5");
    }
}
