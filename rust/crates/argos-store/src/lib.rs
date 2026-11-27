//! 🗄️ Argos Store — la base de données locale du desktop (redb, un seul fichier, Rust pur) :
//! `~/.local/share/argos-hss-trading/argos.redb` (ou `$XDG_DATA_HOME/argos-hss-trading/argos.redb`).
//! - **catalogue** : tous les symboles de chaque exchange (infos fixes, sans prix) ;
//! - **watchlists** : tes listes et leurs symboles (par identifiant `exchange:symbole`).
//! - 🔐 **sensible** (`sensible.redb`, à côté) : les données personnelles (journal), chiffrées valeur par valeur.
//!   Le journal (`journal`) y est rangé entrée par entrée, partagé avec le serveur local qui le sert au cell.
//!
//! Respecte le contrat `argos_core::store::SymbolStore`. La base a un numéro de version (table `meta`).
//! Dans le navigateur (WASM) : `MemStore`, le même contrat en mémoire (redb ne tourne pas dans une page).

#![forbid(unsafe_code)]

pub mod listes;
pub mod mem;
pub use mem::MemStore;

#[cfg(not(target_arch = "wasm32"))]
mod disque;
#[cfg(not(target_arch = "wasm32"))]
pub use disque::{default_path, Sorte, Store};
#[cfg(not(target_arch = "wasm32"))]
pub mod cles;
#[cfg(not(target_arch = "wasm32"))]
pub mod journal;
#[cfg(not(target_arch = "wasm32"))]
pub mod murs;
#[cfg(not(target_arch = "wasm32"))]
pub mod retours;
#[cfg(not(target_arch = "wasm32"))]
pub mod secours;
#[cfg(not(target_arch = "wasm32"))]
pub mod sensible;
