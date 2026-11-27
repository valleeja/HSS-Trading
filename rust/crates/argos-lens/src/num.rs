//! Le seul passage des `f64` des indicateurs vers les décimaux du paquet : arrondi fixe (au pair le plus proche),
//! nombre de décimales fixé par champ, valeur non finie (ou hors des bornes d'un décimal) = absente.

use rust_decimal::prelude::FromPrimitive;
use rust_decimal::{Decimal, RoundingStrategy};

/// Décimales des prix (clôture, EMA, MACD, ATR, VWAP, niveaux, setup).
pub(crate) const PRICE_DP: u32 = 8;
/// Décimales des oscillateurs en points (RSI, +DI, −DI, ADX, percentile de l'ATR).
pub(crate) const OSC_DP: u32 = 2;
/// Décimales des rapports (volume relatif).
pub(crate) const RATIO_DP: u32 = 4;

/// `f64` → décimal à `dp` décimales ; `None` si la valeur n'est pas finie.
pub(crate) fn to_dec(x: f64, dp: u32) -> Option<Decimal> {
    if !x.is_finite() {
        return None;
    }
    Decimal::from_f64(x).map(|d| fix(d, dp))
}

/// Arrondi fixe à exactement `dp` décimales (zéros gardés, jamais de « -0 »).
pub(crate) fn fix(d: Decimal, dp: u32) -> Decimal {
    let mut r = d.round_dp_with_strategy(dp, RoundingStrategy::MidpointNearestEven);
    r.rescale(dp);
    if r.is_zero() {
        r.set_sign_positive(true);
    }
    r
}

/// Décimal → `f64`, seulement pour le moteur de backtest et la position à l'écran (jamais pour le paquet).
pub(crate) fn to_f64(d: Decimal) -> f64 {
    rust_decimal::prelude::ToPrimitive::to_f64(&d).unwrap_or(f64::NAN)
}
