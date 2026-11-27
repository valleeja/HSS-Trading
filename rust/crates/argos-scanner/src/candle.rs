//! Une chandelle OHLC (indépendante de la source : API Hyperliquid, fichier, ou bougies de l'app).

/// Une chandelle OHLC.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Candle {
    /// Ouverture, en millisecondes.
    pub t: i64,
    pub o: f64,
    pub h: f64,
    pub l: f64,
    pub c: f64,
}
