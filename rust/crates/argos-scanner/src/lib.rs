//! 🦀 Argos Scanner (bibliothèque) : stratégie double structure Daily + 4H. Calcul pur (aucun réseau) : l'outil en
//! ligne de commande (`main.rs`, fonction `cli`) lit l'API Hyperliquid, l'app lui passe ses propres bougies.

#![forbid(unsafe_code)]

pub mod candle;
pub mod scan;
pub mod ta;

/// Watchlist par défaut du scanner.
pub const DEFAULT_COINS: &[&str] = &[
    "BTC", "ZEC", "ETH", "SOL", "XRP", "NEAR", "HYPE", "PUMP", "ENA", "UNI", "ARB", "SUI", "ONDO", "AAVE", "LINK", "ADA", "CRV",
];
