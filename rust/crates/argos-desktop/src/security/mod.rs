//! 🔐 Sécurité du compte : 24 mots, clés API, coffre chiffré, Face ID.

pub(crate) mod account;
pub(crate) mod keys;
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) mod passkey;
pub(crate) mod vault;
