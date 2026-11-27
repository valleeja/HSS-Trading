//! 🎬 Ce que fait l'app : des blocs `impl App` rangés par famille (les champs restent dans `App`).

mod account;
mod accueil;
mod actu;
mod aimem;
pub(crate) mod alerts;
mod appmenu;
pub(crate) mod argos_ai_ecran;
mod assistant;
pub(crate) mod avis;
mod backtest;
pub(crate) mod carnet;
pub(crate) mod carnets;
mod catalog;
pub(crate) mod cmc;
mod community;
mod demo_accounts;
mod dex;
mod drawbar;
mod drawings;
mod eco;
mod exchanges;
pub(crate) mod fpmulti;
pub(crate) mod hyperliquid;
mod import;
pub(crate) mod indicators;
mod journal;
pub(crate) mod layouts;
mod liqmap;
mod load;
mod marches;
mod menus;
pub(crate) mod modules;
pub(crate) mod month;
pub(crate) mod murs;
mod notifs;
pub(crate) mod orderflow;
mod ordres;
mod panes;
pub(crate) mod pilule;
pub(crate) mod plein_ecran;
mod replay;
pub(crate) mod roue;
mod scripts;
pub(crate) mod simulateur;
pub(crate) mod support;
pub(crate) mod synchro;
pub(crate) mod tableaux;
mod ticket;
mod trading;
mod wallet;
mod watchlist;
#[cfg(test)]
pub(crate) mod zones;
#[cfg(test)]
pub(crate) use load::FIRST_SCREEN;
#[cfg(test)]
pub(crate) use marches::bloquee_si;
#[cfg(test)]
pub(crate) use menus::ADD_INTERVAL;
#[cfg(test)]
pub(crate) use notifs::alerte_marche_de_la_chart;
#[cfg(test)]
pub(crate) use watchlist::{synth_results, TOUS};
mod whalestore;

pub(crate) use aimem::{remember_request as aimem_request, split_proposal as aimem_split, AiMemory};
pub(crate) use backtest::{BtPrefs, StratCle};
pub(crate) use cmc::cap_title;
pub(crate) use community::Idea;
pub(crate) use journal::JournalSync;
pub(crate) use modules::ModState;
#[cfg(test)]
pub(crate) fn modules_example_json() -> &'static str {
    modules::EXAMPLE_JSON
}
#[cfg(test)]
pub(crate) fn modules_example_wasm() -> &'static [u8] {
    modules::EXAMPLE_WASM
}

#[cfg(test)]
pub(crate) use accueil::Interface2;
#[cfg(test)]
pub(crate) use accueil::ACCUEIL_ONGLET;
pub(crate) use accueil::BARRE_MAQUETTE;
pub(crate) use accueil::RAIL_X;
pub(crate) use demo_accounts::DemoAccounts;
pub(crate) use dex::{dex_title, ASK as DEX_ASK};
pub(crate) use wallet::WalletPrefs;
pub(crate) use whalestore::WhaleStore;
