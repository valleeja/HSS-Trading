//! 🎲 Hasard déterministe d'Argos (SplitMix64, sans dépendance) : la même graine donne toujours la même suite.
//! Sert aux tests « contre le hasard » du backtest et à tout ce qui a besoin de tirages rejouables.
//!
//! ```
//! use argos_core::alea::{Alea, GRAINE};
//! let mut a = Alea::new(GRAINE);
//! let de = a.entier(6) + 1; // un dé : 1 à 6
//! assert!((1..=6).contains(&de));
//! ```

/// Graine fixe par défaut, la seule du projet (« ARGOS HS » en ASCII).
pub const GRAINE: u64 = 0x4152_474F_5320_4853;

/// Générateur SplitMix64 : rapide, 2⁶⁴ valeurs avant de boucler, assez bon pour des tirages (pas pour la crypto).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Alea(u64);

impl Alea {
    /// Un générateur qui part de `graine`.
    pub fn new(graine: u64) -> Self {
        Self(graine)
    }

    /// L'entier 64 bits suivant.
    pub fn suivant(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Un entier dans `[0, n)` (multiplication 128 bits : biais de l'ordre de n / 2⁶⁴, négligeable) ; 0 si `n` = 0.
    pub fn entier(&mut self, n: u64) -> u64 {
        ((u128::from(self.suivant()) * u128::from(n)) >> 64) as u64
    }

    /// Un réel dans `[0, 1)` (53 bits de précision).
    pub fn reel(&mut self) -> f64 {
        (self.suivant() >> 11) as f64 / (1u64 << 53) as f64
    }
}

impl Default for Alea {
    /// Le générateur de la graine fixe `GRAINE`.
    fn default() -> Self {
        Self::new(GRAINE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valeurs_de_reference_splitmix64() {
        // suite publiée de SplitMix64 pour la graine 0
        let mut a = Alea::new(0);
        assert_eq!(
            [a.suivant(), a.suivant(), a.suivant()],
            [0xE220_A839_7B1D_CDAF, 0x6E78_9E6A_A1B9_65F4, 0x06C4_5D18_8009_454F]
        );
    }

    #[test]
    fn meme_graine_meme_suite() {
        let (mut a, mut b) = (Alea::default(), Alea::new(GRAINE));
        let sa: Vec<u64> = (0..100).map(|_| a.suivant()).collect();
        let sb: Vec<u64> = (0..100).map(|_| b.suivant()).collect();
        assert_eq!(sa, sb);
        let mut c = Alea::new(GRAINE + 1);
        assert_ne!(sa[0], c.suivant(), "autre graine, autre suite");
    }

    #[test]
    fn bornes_respectees() {
        let mut a = Alea::default();
        let mut vus = [0u32; 7];
        for _ in 0..7_000 {
            let k = a.entier(7) as usize;
            vus[k] += 1;
            let x = a.reel();
            assert!((0.0..1.0).contains(&x));
        }
        assert!(vus.iter().all(|&v| v > 800), "chaque valeur sort à peu près 1 000 fois : {vus:?}");
        assert_eq!(a.entier(0), 0);
        assert_eq!(a.entier(1), 0);
        assert!((0..1_000).all(|_| a.entier(u64::MAX) < u64::MAX));
    }
}
