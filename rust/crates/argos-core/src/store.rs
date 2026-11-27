//! 🗄️ Contrat de stockage des symboles et des watchlists. Le desktop le remplit avec redb (`argos-store`) ; la
//! version web pourra brancher son propre stockage (navigateur) sans rien changer au reste d'Argos.

use crate::{Instrument, Result};

/// État du catalogue d'un exchange.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogInfo {
    pub exchange: String,
    /// Symboles actifs.
    pub symbols: usize,
    /// Dernière mise à jour (secondes UTC).
    pub updated_at: i64,
}

/// Une watchlist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WatchList {
    pub id: i64,
    pub name: String,
}

/// Une recherche dans les catalogues : texte (vide : tout), exchange et groupe facultatifs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Search<'a> {
    pub query: &'a str,
    /// Identifiant de l'exchange (« hl », « binance »…).
    pub exchange: Option<&'a str>,
    /// Groupe (« Crypto », « TradFi », « HIP-3 xyz »…).
    pub group: Option<&'a str>,
    pub limit: usize,
}

pub trait SymbolStore {
    /// Remplace le catalogue d'un exchange. Un symbole qui n'y est plus reste (les listes s'y réfèrent), marqué inactif.
    fn save_catalog(&mut self, exchange: &str, list: &[Instrument], now_s: i64) -> Result<()>;
    /// État du catalogue de chaque exchange déjà téléchargé.
    fn catalogs(&self) -> Result<Vec<CatalogInfo>>;
    /// Recherche (« btc », « nvda ») dans les catalogues : symbole exact d'abord, actifs avant inactifs. Texte vide :
    /// tous les symboles (du filtre), actifs d'abord.
    fn search(&self, s: &Search) -> Result<Vec<Instrument>>;
    /// Groupes des symboles actifs et leur nombre (« Crypto » 512, « TradFi » 40…).
    fn groups(&self) -> Result<Vec<(String, usize)>>;
    /// Un symbole par son identifiant `exchange:symbole`.
    fn instrument(&self, id: &str) -> Result<Option<Instrument>>;

    /// Les watchlists, dans leur ordre.
    fn lists(&self) -> Result<Vec<WatchList>>;
    /// Crée une watchlist (le nom existe déjà : renvoie celle-là).
    fn create_list(&mut self, name: &str) -> Result<i64>;
    fn rename_list(&mut self, list: i64, name: &str) -> Result<()>;
    /// Supprime une watchlist et ses symboles.
    fn delete_list(&mut self, list: i64) -> Result<()>;
    /// Sections d'une watchlist, dans l'ordre (sans la section sans nom du début).
    fn sections(&self, list: i64) -> Result<Vec<String>>;
    /// Ajoute une section à la fin (déjà là : rien ne change).
    fn add_section(&mut self, list: i64, name: &str) -> Result<()>;
    /// Retire une section : ses symboles remontent dans la section sans nom.
    fn remove_section(&mut self, list: i64, name: &str) -> Result<()>;
    /// Renomme une section (ses symboles la suivent) ; le nouveau nom existe déjà : rien ne change.
    fn rename_section(&mut self, list: i64, name: &str, new: &str) -> Result<()>;
    /// Déplace une section d'un cran vers le haut (`-1`) ou le bas (`+1`) ; au bout : rien ne change.
    fn move_section(&mut self, list: i64, name: &str, delta: i32) -> Result<()>;
    /// Symboles d'une watchlist, dans l'ordre : (section, identifiant).
    fn items(&self, list: i64) -> Result<Vec<(String, String)>>;
    /// Ajoute un symbole à la fin d'une section (déjà là : rien ne change).
    fn add_item(&mut self, list: i64, section: &str, id: &str) -> Result<()>;
    fn remove_item(&mut self, list: i64, id: &str) -> Result<()>;
}
