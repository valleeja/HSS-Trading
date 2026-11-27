//! 💾 Sauvegardes sur l'appareil : charts enregistrées, import des données de la v1, 🔐 journal chiffré (bureau).

pub(crate) mod bougies_web;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod journal;
pub(crate) mod saved;
pub(crate) mod v1import;
