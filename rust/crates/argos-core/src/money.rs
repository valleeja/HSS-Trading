//! 💰 Montants d'argent en **centimes entiers** (`i64`) : solde, P&L, frais, risque, limites PropFirm. Les additions
//! et soustractions sont exactes (aucune dérive après des milliers de trades). Un calcul en nombres à virgule (prix ×
//! quantité, pourcentage) est arrondi **une seule fois** au centime (moitié : loin de zéro).
//!
//! Fichiers : un montant s'écrit comme avant, un nombre JSON (arrondi au centime) — les anciens fichiers, le site et
//! les anciennes versions le relisent ; un ancien nombre à plusieurs décimales est arrondi au centime à la lecture.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::iter::Sum;
use std::ops::{Add, AddAssign, Neg, Sub, SubAssign};

/// Un montant, en centimes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Money(i64);

impl Money {
    pub const ZERO: Money = Money(0);

    pub const fn from_cents(c: i64) -> Self {
        Money(c)
    }

    pub const fn cents(self) -> i64 {
        self.0
    }

    /// Un nombre à virgule → centimes, arrondi une fois (moitié : loin de zéro). `1.005` → 1,01 (et non 1,00 : le
    /// nombre est d'abord ramené à 6 décimales, ce qui efface l'erreur de représentation). Pas un nombre : 0.
    pub fn from_f64(x: f64) -> Self {
        if !x.is_finite() {
            return Money::ZERO;
        }
        let c = ((x * 1e8).round() / 1e6).round();
        Money(c.clamp(i64::MIN as f64, i64::MAX as f64) as i64)
    }

    /// En dollars (pour l'affichage et les calculs de pourcentage).
    pub fn to_f64(self) -> f64 {
        self.0 as f64 / 100.0
    }

    /// Valeur d'une quantité à un prix (prix × quantité), arrondie une seule fois au centime.
    pub fn of(price: f64, qty: f64) -> Self {
        Money::produit(&[price, qty])
    }

    /// 💰 Produit de nombres (prix × quantité × taux…) calculé en **décimales exactes** (`rust_decimal`), arrondi une
    /// seule fois au centime (moitié : loin de zéro). Chaque nombre est pris tel qu'il s'écrit (`0.1` vaut 0,1, pas
    /// 0,1000000000000000055…). Trop grand pour les décimales : calcul à virgule, comme avant.
    pub fn produit(v: &[f64]) -> Self {
        let mut d = rust_decimal::Decimal::ONE;
        for &x in v {
            match dec(x).and_then(|x| d.checked_mul(x)) {
                Some(p) => d = p,
                None => return Money::from_f64(v.iter().product()),
            }
        }
        cents(d).unwrap_or_else(|| Money::from_f64(v.iter().product()))
    }

    /// 💰 Résultat d'une position : (sortie − entrée) × quantité × sens (+1 achat, −1 vente), en décimales exactes,
    /// arrondi une seule fois au centime.
    pub fn resultat(exit: f64, entry: f64, qty: f64, sign: f64) -> Self {
        let exact = || {
            let diff = dec(exit)?.checked_sub(dec(entry)?)?;
            cents(diff.checked_mul(dec(qty)?)?.checked_mul(dec(sign)?)?)
        };
        exact().unwrap_or_else(|| Money::from_f64((exit - entry) * qty * sign))
    }

    /// `pct` % de ce montant, arrondi au centime.
    pub fn pct(self, pct: f64) -> Self {
        Money::from_f64(self.to_f64() * pct / 100.0)
    }

    pub fn abs(self) -> Self {
        Money(self.0.saturating_abs())
    }

    #[cfg(test)]
    pub fn is_negative(self) -> bool {
        self.0 < 0
    }
}

/// Un nombre à virgule tel qu'il s'écrit (le plus court qui se relit pareil), en décimales ; `None` s'il n'est pas un
/// nombre ou dépasse les décimales.
fn dec(x: f64) -> Option<rust_decimal::Decimal> {
    use std::str::FromStr;
    if !x.is_finite() {
        return None;
    }
    rust_decimal::Decimal::from_str(&format!("{x}")).ok()
}

/// Décimales → centimes (moitié : loin de zéro).
fn cents(d: rust_decimal::Decimal) -> Option<Money> {
    let c = d
        .checked_mul(rust_decimal::Decimal::ONE_HUNDRED)?
        .round_dp_with_strategy(0, rust_decimal::RoundingStrategy::MidpointAwayFromZero);
    i64::try_from(c).ok().map(Money)
}

impl Add for Money {
    type Output = Money;
    fn add(self, o: Money) -> Money {
        Money(self.0.saturating_add(o.0))
    }
}

impl Sub for Money {
    type Output = Money;
    fn sub(self, o: Money) -> Money {
        Money(self.0.saturating_sub(o.0))
    }
}

impl Neg for Money {
    type Output = Money;
    fn neg(self) -> Money {
        Money(self.0.saturating_neg())
    }
}

impl AddAssign for Money {
    fn add_assign(&mut self, o: Money) {
        *self = *self + o;
    }
}

impl SubAssign for Money {
    fn sub_assign(&mut self, o: Money) {
        *self = *self - o;
    }
}

impl Sum for Money {
    fn sum<I: Iterator<Item = Money>>(it: I) -> Money {
        it.fold(Money::ZERO, Add::add)
    }
}

impl<'a> Sum<&'a Money> for Money {
    fn sum<I: Iterator<Item = &'a Money>>(it: I) -> Money {
        it.copied().sum()
    }
}

/// « -12.34 » (2 décimales, point).
impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign = if self.0 < 0 { "-" } else { "" };
        let a = self.0.unsigned_abs();
        write!(f, "{sign}{}.{:02}", a / 100, a % 100)
    }
}

/// Écrit comme un nombre JSON au centime (même forme qu'avant : lisible par le site et les anciennes versions).
impl Serialize for Money {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_f64(self.to_f64())
    }
}

/// Relit un nombre (ancien ou nouveau, arrondi au centime) ou un texte (« 12.34 »).
impl<'de> Deserialize<'de> for Money {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            N(f64),
            S(String),
        }
        Ok(match Raw::deserialize(d)? {
            Raw::N(x) => Money::from_f64(x),
            Raw::S(s) => Money::from_f64(s.trim().replace(',', ".").parse::<f64>().map_err(serde::de::Error::custom)?),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prix_fois_quantite_en_decimales_exactes() {
        // 0,1 × 3 vaut 0,30 exactement ; 0,145 × 1 s'arrondit à 0,15 (moitié : loin de zéro)
        assert_eq!(Money::of(0.1, 3.0), Money::from_cents(30));
        assert_eq!(Money::of(0.145, 1.0), Money::from_cents(15));
        assert_eq!(Money::of(-0.145, 1.0), Money::from_cents(-15));
        // 1,005 $ × 1 000 : 1 005,00 $ pile
        assert_eq!(Money::of(1.005, 1000.0), Money::from_cents(100_500));
        // frais : 33 333,33 $ × 0,05 % = 16,666665 $ → 16,67 $
        assert_eq!(Money::produit(&[33_333.33, 0.0005]), Money::from_cents(1_667));
        // (sortie − entrée) × quantité × sens : 0,3 − 0,1 = 0,2 exactement
        assert_eq!(Money::resultat(0.3, 0.1, 1_000.0, 1.0), Money::from_cents(20_000));
        assert_eq!(Money::resultat(0.3, 0.1, 1_000.0, -1.0), Money::from_cents(-20_000));
        // pas un nombre : 0 ; trop grand pour les décimales : comme avant (à virgule, arrondi au centime)
        assert_eq!(Money::produit(&[f64::NAN, 2.0]), Money::ZERO);
        assert_eq!(Money::produit(&[1e30, 1e30]), Money::from_f64(1e60));
    }

    #[test]
    fn arrondi_au_centime_une_fois() {
        assert_eq!(Money::from_f64(0.005).cents(), 1, "0,005 → 0,01 (moitié loin de zéro)");
        assert_eq!(Money::from_f64(1.005).cents(), 101, "1,005 → 1,01 malgré l'erreur de représentation");
        assert_eq!(Money::from_f64(2.675).cents(), 268);
        assert_eq!(Money::from_f64(-0.005).cents(), -1, "perte : arrondie loin de zéro aussi");
        assert_eq!(Money::from_f64(0.0049).cents(), 0);
        assert_eq!(Money::from_f64(f64::NAN), Money::ZERO);
        assert_eq!(Money::of(0.1, 3.0).cents(), 30, "0,1 × 3 = 0,30 (pas 0,30000000000000004)");
        assert_eq!(Money::from_cents(10_000_000).pct(1.0).cents(), 100_000, "1 % de 100 000 = 1 000");
    }

    #[test]
    fn tres_gros_comptes_et_pertes() {
        let big = Money::from_f64(9_000_000_000_000.0); // 9 000 milliards : tient dans i64 (centimes)
        assert_eq!(big.cents(), 900_000_000_000_000);
        assert_eq!((big + Money::from_f64(0.01)).to_string(), "9000000000000.01");
        assert_eq!((Money::from_f64(100.0) - Money::from_f64(250.55)).to_string(), "-150.55");
        assert_eq!(
            Money::from_cents(i64::MAX) + Money::from_cents(1),
            Money::from_cents(i64::MAX),
            "pas de débordement"
        );
        assert!(Money::from_f64(-3.0).is_negative() && Money::from_f64(-3.0).abs() == Money::from_f64(3.0));
    }

    #[test]
    fn mille_trades_sans_derive() {
        // 1000 trades de +0,10 $ puis 1000 de −0,10 $ : retour exact au départ (en f64 : 1e-12 près, pas exact)
        let start = Money::from_f64(100_000.0);
        let mut bal = start;
        for _ in 0..1000 {
            bal += Money::from_f64(0.1);
        }
        assert_eq!(bal, Money::from_f64(100_100.0));
        for _ in 0..1000 {
            bal -= Money::from_f64(0.1);
        }
        assert_eq!(bal, start);
        let mut f = 100_000.0f64;
        (0..1000).for_each(|_| f += 0.1);
        assert_ne!(f, 100_100.0, "le même calcul en f64 dérive (preuve que le test a un sens)");
        let total: Money = (0..1000).map(|_| Money::from_f64(0.1)).sum();
        assert_eq!(total.cents(), 10_000);
    }

    #[test]
    fn fichiers_anciens_et_nouveaux() {
        let m: Money = serde_json::from_str("100000.004999").unwrap();
        assert_eq!(m.cents(), 10_000_000, "ancien nombre : arrondi au centime");
        let m: Money = serde_json::from_str("\"12,35\"").unwrap();
        assert_eq!(m.cents(), 1235);
        assert_eq!(serde_json::to_string(&Money::from_f64(-150.55)).unwrap(), "-150.55", "toujours un nombre JSON");
    }
}
