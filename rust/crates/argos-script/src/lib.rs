//! # Argos Script — Layer 2B
//! - `ta` : fonctions d'analyse technique (mêmes résultats que Pine Script)
//! - `indicators` : indicateurs par défaut (bougies de volume TVD Pro, puis Cipher, AI Trend, TBO, SMC)
//! - à venir : le langage Argos Script et la sous-couche `pine` (transcription Pine → Argos Script)
//!
//! Utilise seulement la couche 1 (`argos-core`).

#![forbid(unsafe_code)]

pub mod backtest;
pub mod catalogue;
pub mod classic_inputs;
pub mod indicators;
pub mod inputs;
pub mod integres;
pub mod lang;
pub mod style_integres;
pub mod ta;
pub mod v1_inputs;
