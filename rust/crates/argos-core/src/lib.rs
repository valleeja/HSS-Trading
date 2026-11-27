//! # Argos Core — Layer 1
//! Compte, structure (types), sécurité / sandbox, contrat de lecture d'API de bougies, backtest et portfolio.
//! Cette couche ne dépend d'aucune autre couche d'Argos. Les couches du dessus l'utilisent ; elle ne les connaît pas.

#![forbid(unsafe_code)]

pub mod account;
pub mod alea;
pub mod alerts;
pub mod backtest;
pub mod carnet;
pub mod catalogue;
pub mod couts;
pub mod dossiers;
pub mod feed;
pub mod instrument;
pub mod interval;
pub mod journal;
pub mod liqmap;
pub mod money;
pub mod month;
pub mod murs;
pub mod nombres;
pub mod orderflow;
pub mod perp;
pub mod portfolio;
pub mod retours;
pub mod security;
pub mod simu;
pub mod store;
pub mod types;

pub use instrument::{Instrument, MarketKind};
pub use interval::Interval;
pub use money::Money;
pub use types::{
    aggregate, au_replay, bucket_end, bucket_start, derniere_finie, fin_en_cours, heikin_ashi, renko, se_regroupe, ArgosError, Bar, Result, Side, Symbol,
    Timeframe, MONTH_SECONDS,
};

/// Empreinte SHA-256 (32 octets) : fichiers du cache des bougies (un fichier modifié ou abîmé est reconnu).
pub fn sha256(data: &[u8]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    Sha256::digest(data).into()
}

/// Empreinte SHA-256 en hexadécimal.
pub fn sha256_hex(data: &[u8]) -> String {
    sha256(data).iter().map(|b| format!("{b:02x}")).collect()
}
