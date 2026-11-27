//! La base du desktop sur disque (redb, un seul fichier, Rust pur). Toute la base vit aussi en mémoire (`MemStore`,
//! la même logique que dans le navigateur) ; chaque changement est écrit dans le fichier en une transaction (tout ou
//! rien). Tables : `instruments` (`exchange:symbole` → instrument), `exchanges` (id → mise à jour en secondes),
//! `meta` (`version`, `watchlists` au format partagé de `listes`).

use crate::MemStore;
use argos_core::store::{CatalogInfo, Search, SymbolStore, WatchList};
use argos_core::{ArgosError, Instrument, MarketKind, Result};
use redb::backends::InMemoryBackend;
use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};

mod catalogue;
pub use catalogue::Sorte;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// Version du format de la base.
const VERSION: &str = "1";

const INSTRUMENTS: TableDefinition<&str, &str> = TableDefinition::new("instruments");
const EXCHANGES: TableDefinition<&str, i64> = TableDefinition::new("exchanges");
const META: TableDefinition<&str, &str> = TableDefinition::new("meta");

/// Emplacement de la base, propre à Argos HSS Trading : `$XDG_DATA_HOME/argos-hss-trading/argos.redb`, sinon
/// `~/.local/share/argos-hss-trading/argos.redb`.
pub fn default_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_DATA_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local").join("share")))?;
    Some(base.join("argos-hss-trading").join("argos.redb"))
}

fn err(e: impl Into<redb::Error>) -> ArgosError {
    ArgosError::Invalid(format!("base de données : {}", e.into()))
}

fn encode(i: &Instrument) -> String {
    json!([i.base, i.quote, i.kind.id(), i.group, i.price_decimals, i.size_decimals, i.growth, i.active]).to_string()
}

fn decode(id: &str, raw: &str) -> Option<Instrument> {
    let (exchange, symbol) = argos_core::instrument::split_id(id)?;
    let v: Value = serde_json::from_str(raw).ok()?;
    let s = |k: usize| v[k].as_str().map(str::to_string);
    let n = |k: usize| v[k].as_u64().and_then(|x| u32::try_from(x).ok());
    Some(Instrument {
        exchange: exchange.to_string(),
        symbol: symbol.to_string(),
        base: s(0)?,
        quote: s(1)?,
        kind: MarketKind::parse(v[2].as_str()?).unwrap_or(MarketKind::Perp),
        group: s(3)?,
        price_decimals: n(4)?,
        size_decimals: n(5)?,
        growth: v[6].as_bool()?,
        active: v[7].as_bool()?,
    })
}

/// La base de données.
pub struct Store {
    mem: MemStore,
    db: Database,
}

impl Store {
    /// Ouvre (ou crée) la base à cet endroit.
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(d) = path.parent() {
            std::fs::create_dir_all(d).map_err(|e| ArgosError::Invalid(format!("dossier de la base : {e}")))?;
        }
        Self::setup(Database::create(path).map_err(err)?)
    }

    /// Base en mémoire (tests).
    pub fn in_memory() -> Result<Self> {
        Self::setup(Database::builder().create_with_backend(InMemoryBackend::new()).map_err(err)?)
    }

    fn setup(db: Database) -> Result<Self> {
        // les tables existent toujours (une base neuve les crée, vides)
        let tx = db.begin_write().map_err(err)?;
        {
            tx.open_table(INSTRUMENTS).map_err(err)?;
            tx.open_table(EXCHANGES).map_err(err)?;
            let mut meta = tx.open_table(META).map_err(err)?;
            if meta.get("version").map_err(err)?.is_none() {
                meta.insert("version", VERSION).map_err(err)?;
            }
        }
        tx.commit().map_err(err)?;

        let rx = db.begin_read().map_err(err)?;
        let meta = rx.open_table(META).map_err(err)?;
        let lists = meta.get("watchlists").map_err(err)?.map(|g| g.value().to_string()).unwrap_or_default();
        let mut mem = MemStore::from_lists_json(&lists);
        for row in rx.open_table(INSTRUMENTS).map_err(err)?.iter().map_err(err)? {
            let (k, v) = row.map_err(err)?;
            // une ligne illisible est sautée : le catalogue se retélécharge
            mem.instruments.extend(decode(k.value(), v.value()));
        }
        for row in rx.open_table(EXCHANGES).map_err(err)?.iter().map_err(err)? {
            let (k, v) = row.map_err(err)?;
            mem.exchanges.push((k.value().to_string(), v.value()));
        }
        drop((meta, rx));
        Ok(Self { mem, db })
    }

    /// Change la base en mémoire puis écrit les listes dans le fichier (une transaction).
    fn lists_change<T>(&mut self, f: impl FnOnce(&mut MemStore) -> Result<T>) -> Result<T> {
        let out = f(&mut self.mem)?;
        let j = crate::listes::vers_json(&self.mem);
        let tx = self.db.begin_write().map_err(err)?;
        tx.open_table(META).map_err(err)?.insert("watchlists", j.as_str()).map_err(err)?;
        tx.commit().map_err(err)?;
        Ok(out)
    }
}

impl SymbolStore for Store {
    fn save_catalog(&mut self, exchange: &str, list: &[Instrument], now_s: i64) -> Result<()> {
        self.mem.save_catalog(exchange, list, now_s)?;
        // tout le catalogue de cet exchange réécrit en un seul bloc : tout ou rien
        let tx = self.db.begin_write().map_err(err)?;
        {
            let mut t = tx.open_table(INSTRUMENTS).map_err(err)?;
            for i in self.mem.instruments.iter().filter(|i| i.exchange == exchange) {
                t.insert(i.id().as_str(), encode(i).as_str()).map_err(err)?;
            }
            tx.open_table(EXCHANGES).map_err(err)?.insert(exchange, now_s).map_err(err)?;
        }
        tx.commit().map_err(err)
    }

    fn catalogs(&self) -> Result<Vec<CatalogInfo>> {
        self.mem.catalogs()
    }

    fn search(&self, s: &Search) -> Result<Vec<Instrument>> {
        self.mem.search(s)
    }

    fn groups(&self) -> Result<Vec<(String, usize)>> {
        self.mem.groups()
    }

    fn instrument(&self, id: &str) -> Result<Option<Instrument>> {
        self.mem.instrument(id)
    }

    fn lists(&self) -> Result<Vec<WatchList>> {
        self.mem.lists()
    }

    fn create_list(&mut self, name: &str) -> Result<i64> {
        self.lists_change(|m| m.create_list(name))
    }

    fn rename_list(&mut self, list: i64, name: &str) -> Result<()> {
        self.lists_change(|m| m.rename_list(list, name))
    }

    fn delete_list(&mut self, list: i64) -> Result<()> {
        self.lists_change(|m| m.delete_list(list))
    }

    fn sections(&self, list: i64) -> Result<Vec<String>> {
        self.mem.sections(list)
    }

    fn add_section(&mut self, list: i64, name: &str) -> Result<()> {
        self.lists_change(|m| m.add_section(list, name))
    }

    fn remove_section(&mut self, list: i64, name: &str) -> Result<()> {
        self.lists_change(|m| m.remove_section(list, name))
    }

    fn rename_section(&mut self, list: i64, name: &str, new: &str) -> Result<()> {
        self.lists_change(|m| m.rename_section(list, name, new))
    }

    fn move_section(&mut self, list: i64, name: &str, delta: i32) -> Result<()> {
        self.lists_change(|m| m.move_section(list, name, delta))
    }

    fn items(&self, list: i64) -> Result<Vec<(String, String)>> {
        self.mem.items(list)
    }

    fn add_item(&mut self, list: i64, section: &str, id: &str) -> Result<()> {
        self.lists_change(|m| m.add_item(list, section, id))
    }

    fn remove_item(&mut self, list: i64, id: &str) -> Result<()> {
        self.lists_change(|m| m.remove_item(list, id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inst(ex: &str, symbol: &str, base: &str, active: bool) -> Instrument {
        Instrument {
            exchange: ex.into(),
            symbol: symbol.into(),
            base: base.into(),
            quote: "USDT".into(),
            kind: MarketKind::Perp,
            group: "Crypto".into(),
            price_decimals: 1,
            size_decimals: 3,
            growth: false,
            active,
        }
    }

    /// Les deux bases (fichier redb et mémoire) passent les mêmes tests.
    #[test]
    fn catalogue_et_recherche() {
        catalogue(Store::in_memory().unwrap());
        catalogue(MemStore::default());
    }

    fn catalogue<S: SymbolStore>(mut s: S) {
        let bin = vec![
            inst("binance", "BTCUSDT", "BTC", true),
            inst("binance", "BTCDOMUSDT", "BTCDOM", true),
            inst("binance", "ETHUSDT", "ETH", true),
        ];
        s.save_catalog("binance", &bin, 100).unwrap();
        s.save_catalog("hl", &[inst("hl", "BTC", "BTC", true), inst("hl", "xyz:NVDA", "NVDA", true)], 200)
            .unwrap();
        let find = |s: &S, q: &str, n: usize| {
            s.search(&Search {
                query: q,
                limit: n,
                ..Search::default()
            })
            .unwrap()
        };
        let found = find(&s, "btc", 10);
        assert_eq!(
            found.iter().map(|i| i.id()).collect::<Vec<_>>(),
            vec!["hl:BTC", "binance:BTCUSDT", "binance:BTCDOMUSDT"],
            "exact d'abord, puis plus court"
        );
        assert_eq!(find(&s, "nvda", 5)[0].id(), "hl:xyz:NVDA");
        assert_eq!(find(&s, "  ", 50).len(), 5, "texte vide : tout");
        let only = s
            .search(&Search {
                query: "",
                exchange: Some("hl"),
                group: None,
                limit: 50,
            })
            .unwrap();
        assert_eq!(only.len(), 2);
        let grp = s
            .search(&Search {
                query: "",
                exchange: None,
                group: Some("Crypto"),
                limit: 50,
            })
            .unwrap();
        assert_eq!(grp.len(), 5);
        assert_eq!(s.groups().unwrap(), vec![("Crypto".to_string(), 5)]);
        assert_eq!(s.instrument("hl:xyz:NVDA").unwrap().map(|i| i.base), Some("NVDA".into()));
        assert_eq!(s.instrument("hl:RIEN").unwrap(), None);
        // nouvelle liste de Binance sans ETHUSDT : il reste, inactif ; l'état du catalogue suit
        s.save_catalog("binance", &bin[..2], 300).unwrap();
        assert!(!s.instrument("binance:ETHUSDT").unwrap().unwrap().active);
        let info = s.catalogs().unwrap();
        assert_eq!(
            info[0],
            CatalogInfo {
                exchange: "binance".into(),
                symbols: 2,
                updated_at: 300
            }
        );
        assert_eq!(info[1].symbols, 2);
    }

    /// 🔎 The symbol search ignores case and separators: « solusdt » finds Binance / Bybit SOLUSDT and OKX
    /// SOL-USDT-SWAP, « btcusdc » Hyperliquid's BTC (USDC); « sol » still finds every SOL, in the same order.
    #[test]
    fn recherche_sans_separateurs() {
        recherche_nette(Store::in_memory().unwrap());
        recherche_nette(MemStore::default());
    }

    fn recherche_nette<S: SymbolStore>(mut s: S) {
        let q = |ex: &str, symbol: &str, base: &str, quote: &str| Instrument {
            quote: quote.into(),
            ..inst(ex, symbol, base, true)
        };
        s.save_catalog("binance", &[q("binance", "SOLUSDT", "SOL", "USDT"), q("binance", "BTCUSDT", "BTC", "USDT")], 1)
            .unwrap();
        s.save_catalog("bybit", &[q("bybit", "SOLUSDT", "SOL", "USDT")], 1).unwrap();
        s.save_catalog("okx", &[q("okx", "SOL-USDT-SWAP", "SOL", "USDT")], 1).unwrap();
        s.save_catalog("hl", &[q("hl", "SOL", "SOL", "USDC"), q("hl", "BTC", "BTC", "USDC")], 1)
            .unwrap();
        let ids = |q: &str| {
            s.search(&Search {
                query: q,
                limit: 50,
                ..Search::default()
            })
            .unwrap()
            .iter()
            .map(|i| i.id())
            .collect::<Vec<_>>()
        };
        for q in ["solusdt", "SOL/USDT", "sol-usdt", "Sol Usdt"] {
            let r = ids(q);
            for id in ["binance:SOLUSDT", "bybit:SOLUSDT", "okx:SOL-USDT-SWAP"] {
                assert!(r.contains(&id.to_string()), "{q}: {id} in {r:?}");
            }
            assert!(!r.iter().any(|i| i.starts_with("hl:")), "{q}: not the USDC ones");
        }
        assert_eq!(ids("btcusdc"), ["hl:BTC"]);
        assert_eq!(ids("solusdc"), ["hl:SOL"]);
        assert_eq!(
            ids("sol"),
            ["hl:SOL", "binance:SOLUSDT", "bybit:SOLUSDT", "okx:SOL-USDT-SWAP"],
            "every SOL, as before"
        );
        assert!(!ids("eth").iter().any(|i| i.contains("BTC")));
    }

    #[test]
    fn watchlists() {
        lists(Store::in_memory().unwrap());
        lists(MemStore::default());
        // navigateur : les listes se gardent en JSON et reviennent pareilles
        let mut m = MemStore::default();
        let a = m.create_list("Or").unwrap();
        m.add_item(a, "Métaux", "oanda:XAUUSD").unwrap();
        let back = MemStore::from_lists_json(&m.lists_json());
        assert_eq!(back.lists().unwrap(), m.lists().unwrap());
        assert_eq!(back.items(a).unwrap(), vec![("Métaux".into(), "oanda:XAUUSD".into())]);
        assert_eq!(back.sections(a).unwrap(), vec!["Métaux"]);
    }

    fn lists<S: SymbolStore>(mut s: S) {
        let a = s.create_list("Mes favoris").unwrap();
        assert_eq!(s.create_list("Mes favoris").unwrap(), a, "même nom : même liste");
        let b = s.create_list("Actions").unwrap();
        assert_eq!(
            s.lists().unwrap().iter().map(|l| l.name.as_str()).collect::<Vec<_>>(),
            vec!["Mes favoris", "Actions"]
        );
        s.add_item(a, "", "hl:BTC").unwrap();
        s.add_item(a, "ISO 20022", "binance:XRPUSDT").unwrap();
        s.add_item(a, "", "hl:BTC").unwrap(); // déjà là
        assert_eq!(
            s.items(a).unwrap(),
            vec![("".into(), "hl:BTC".into()), ("ISO 20022".into(), "binance:XRPUSDT".into())]
        );
        assert!(s.add_item(a, "", "BTC").is_err(), "identifiant sans exchange refusé");
        s.remove_item(a, "hl:BTC").unwrap();
        assert_eq!(s.items(a).unwrap().len(), 1);
        assert!(s.items(b).unwrap().is_empty());
        // sections : une section vide existe, l'ordre suit les sections ; la retirer remonte ses symboles
        s.add_section(a, "Altcoins").unwrap();
        s.add_item(a, "", "hl:ETH").unwrap();
        s.add_item(a, "Altcoins", "hl:SOL").unwrap();
        assert_eq!(s.sections(a).unwrap(), vec!["ISO 20022", "Altcoins"]);
        let ids = |s: &S| s.items(a).unwrap().into_iter().map(|(_, i)| i).collect::<Vec<_>>();
        assert_eq!(ids(&s), vec!["hl:ETH", "binance:XRPUSDT", "hl:SOL"]);
        // 📱 sections nommables et réordonnables (Jee, 2026-11-09) : l'ordre des symboles suit ; au bout rien ne bouge ;
        // un nom déjà pris est refusé
        s.move_section(a, "Altcoins", -1).unwrap();
        assert_eq!(s.sections(a).unwrap(), vec!["Altcoins", "ISO 20022"]);
        assert_eq!(ids(&s), vec!["hl:ETH", "hl:SOL", "binance:XRPUSDT"]);
        s.move_section(a, "Altcoins", -1).unwrap();
        assert_eq!(s.sections(a).unwrap(), vec!["Altcoins", "ISO 20022"]);
        s.rename_section(a, "Altcoins", "Métaux").unwrap();
        s.rename_section(a, "Métaux", "ISO 20022").unwrap();
        assert_eq!(s.sections(a).unwrap(), vec!["Métaux", "ISO 20022"]);
        assert_eq!(s.items(a).unwrap()[1], ("Métaux".into(), "hl:SOL".into()));
        s.move_section(a, "Métaux", 1).unwrap();
        s.remove_section(a, "ISO 20022").unwrap();
        assert_eq!(s.items(a).unwrap()[0], ("".into(), "binance:XRPUSDT".into()));
        // renommer, supprimer
        s.rename_list(b, "Actions US").unwrap();
        s.delete_list(a).unwrap();
        assert_eq!(
            s.lists().unwrap(),
            vec![WatchList {
                id: b,
                name: "Actions US".into()
            }]
        );
        assert!(s.items(a).unwrap().is_empty() && s.sections(a).unwrap().is_empty(), "tout part avec la liste");
    }

    /// Témoin : après réouverture du fichier, tout se relit pareil (catalogue, recherche, listes, sections, ordre).
    #[test]
    fn fichier_garde_tout_et_version() {
        let dir = std::env::temp_dir().join(format!("argos-store-test-{}", std::process::id()));
        let path = dir.join("argos.redb");
        let photo = |s: &Store| {
            let all = Search {
                query: "",
                limit: 100,
                ..Search::default()
            };
            let lists = s.lists().unwrap();
            let detail: Vec<_> = lists.iter().map(|l| (s.sections(l.id).unwrap(), s.items(l.id).unwrap())).collect();
            (s.catalogs().unwrap(), s.search(&all).unwrap(), s.groups().unwrap(), lists, detail)
        };
        let avant = {
            let mut s = Store::open(&path).unwrap();
            s.save_catalog("hl", &[inst("hl", "HYPE", "HYPE", true), inst("hl", "xyz:NVDA", "NVDA", false)], 50)
                .unwrap();
            let l = s.create_list("Liste").unwrap();
            s.add_item(l, "Perps", "hl:HYPE").unwrap();
            s.add_item(l, "", "hl:xyz:NVDA").unwrap();
            let m = s.create_list("Or").unwrap();
            s.add_section(m, "Vide").unwrap();
            s.move_section(m, "Vide", 0).unwrap();
            photo(&s)
        };
        let s = Store::open(&path).unwrap(); // réouverture : rien de perdu, rien de recréé
        assert_eq!(photo(&s), avant);
        assert_eq!(s.instrument("hl:xyz:NVDA").unwrap().map(|i| (i.base, i.active)), Some(("NVDA".into(), false)));
        let rx = s.db.begin_read().unwrap();
        assert_eq!(rx.open_table(META).unwrap().get("version").unwrap().unwrap().value(), VERSION);
        drop((rx, s));
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn instrument_encode_decode() {
        let i = inst("hl", "xyz:NVDA", "NVDA", false);
        assert_eq!(decode(&i.id(), &encode(&i)), Some(i));
        assert_eq!(decode("hl:BTC", "pas du json"), None);
        assert_eq!(decode("sans-deux-points", &encode(&inst("hl", "BTC", "BTC", true))), None);
    }

    /// Reprise : une base redb neuve reprend les listes de l'ancienne base par `watchlists.json` (même format pour
    /// toutes les bases), sans rien lire de l'ancienne base ; les catalogues, eux, se retéléchargent.
    #[test]
    fn reprise_des_listes_par_le_json_partage() {
        let mut ancienne = MemStore::default();
        let a = ancienne.create_list("Mes favoris").unwrap();
        ancienne.add_item(a, "", "hl:BTC").unwrap();
        ancienne.add_item(a, "Métaux", "oanda:XAUUSD").unwrap();
        ancienne.add_section(a, "Vide").unwrap();
        let b = ancienne.create_list("Actions").unwrap();
        ancienne.add_item(b, "", "hl:xyz:NVDA").unwrap();
        let json = crate::listes::vers_json(&ancienne);

        let mut neuve = Store::in_memory().unwrap();
        assert!(crate::listes::appliquer(&mut neuve, &crate::listes::lire(&json).unwrap(), false));
        assert_eq!(crate::listes::vers_json(&neuve), json, "mêmes listes, sections, symboles et ordre");
    }
}
