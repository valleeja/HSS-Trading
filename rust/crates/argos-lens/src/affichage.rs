//! Nombres du paquet écrits pour l'écran, en français (« 2 418,40 ») : directement depuis les décimaux, sans `f64`.

use crate::num::fix;
use rust_decimal::Decimal;

/// `d` arrondi à `dp` décimales (au pair le plus proche), milliers séparés par une espace, virgule décimale.
pub fn nombre_fr(d: Decimal, dp: u32) -> String {
    let s = fix(d, dp).to_string();
    let (signe, s) = s.strip_prefix('-').map_or(("", s.as_str()), |r| ("-", r));
    let (entier, decimales) = s.split_once('.').unwrap_or((s, ""));
    let mut groupes = String::new();
    for (k, c) in entier.chars().enumerate() {
        if k > 0 && (entier.len() - k) % 3 == 0 {
            groupes.push(' ');
        }
        groupes.push(c);
    }
    if decimales.is_empty() {
        format!("{signe}{groupes}")
    } else {
        format!("{signe}{groupes},{decimales}")
    }
}

/// Décimales d'un prix à l'écran : 2 à partir de 1 000, plus pour les petits prix (au plus 6).
pub fn decimales_prix(d: Decimal) -> u32 {
    let entier = d.trunc().abs().to_string().len() as u32;
    6u32.saturating_sub(entier).clamp(2, 6)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ecriture_francaise() {
        let d = |s: &str| s.parse::<Decimal>().unwrap();
        assert_eq!(nombre_fr(d("2418.40000000"), 2), "2 418,40");
        assert_eq!(nombre_fr(d("1234567.891"), 2), "1 234 567,89");
        assert_eq!(nombre_fr(d("999"), 0), "999");
        assert_eq!(nombre_fr(d("1000"), 0), "1 000");
        assert_eq!(nombre_fr(d("31.49"), 0), "31");
        assert_eq!(nombre_fr(d("1.35"), 1), "1,4");
        assert_eq!(nombre_fr(d("0.125"), 2), "0,12");
        assert_eq!(nombre_fr(d("-1234.5"), 1), "-1 234,5");
        assert_eq!(nombre_fr(d("-0.001"), 2), "0,00");
        assert_eq!(nombre_fr(d("0.00012345"), 6), "0,000123");
        assert_eq!([d("2418.4"), d("95000"), d("1.0832"), d("0.5")].map(decimales_prix), [2, 2, 5, 5]);
    }
}
