//! Un fichier par exchange. Chacun donne seulement 4 choses, sans réseau (donc testables hors ligne) :
//!
//! - `candles_request` : quelle requête faire pour les bougies
//! - `parse_candles`   : lire la réponse → bougies
//! - `markets_request` : quelle requête faire pour la liste des marchés
//! - `parse_markets`   : lire la réponse → marchés
//!
//! Ajouter un exchange = créer son fichier + une ligne dans `Exchange` (lib.rs). Rien d'autre à toucher.

pub mod aster;
pub mod binance;
pub mod bitfinex;
pub mod bitget;
pub mod bitstamp;
pub mod bitunix;
pub mod blofin;
pub mod bybit;
pub mod coinbase;
pub mod gate;
pub mod hyperliquid;
pub mod kraken;
pub mod kucoin;
pub mod mexc;
pub mod okx;
pub mod spot;
pub mod weex;

/// Une requête HTTP à faire (le client HTTP injecté l'exécute).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    Get(String),
    PostJson { url: String, body: String },
}
