//! Indicateurs par défaut d'Argos : un fichier chacun, calcul pur (bougies → résultat), testés contre TradingView.
//! Ordre de construction : bougies de volume → Cipher Pro → AI Trend → TBO → SMC.

pub mod ai_trend;
pub mod ai_trend_volume;
pub mod anchored_vwap;
pub mod cipher;
pub mod classic;
pub mod flow;
pub mod mtf_table;
pub mod order_blocks;
pub mod patterns;
pub mod pivot_vwap;
pub mod smc;
pub mod smc_pro;
pub mod tbo;
pub mod textbook;
pub mod tpo;
pub mod tvd;
pub mod vector_zone;
pub mod volume_candles;
