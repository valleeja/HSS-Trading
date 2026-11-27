//! 🗄️ La même base en mémoire (contrat `SymbolStore`) : pour le navigateur (WASM, où redb ne tourne pas). Les catalogues se
//! retéléchargent ; les watchlists se gardent avec `lists_json` / `from_lists_json` (stockage de la page).

use argos_core::instrument::split_id;
use argos_core::store::{CatalogInfo, Search, SymbolStore, WatchList};
use argos_core::{ArgosError, Instrument, Result};
use serde_json::Value;

/// 🔎 A name as the symbol search compares it: upper case, no « / - _ . : » nor spaces (« SOL-USDT-SWAP » →
/// « SOLUSDTSWAP », « sol usdt » → « SOLUSDT »).
fn sans_separateurs(s: &str) -> String {
    s.chars()
        .filter(|c| !matches!(c, '/' | '-' | '_' | '.' | ':') && !c.is_whitespace())
        .flat_map(char::to_uppercase)
        .collect()
}

/// La base en mémoire.
#[derive(Debug, Clone, Default)]
pub struct MemStore {
    pub(crate) instruments: Vec<Instrument>,
    /// (exchange, mise à jour en secondes).
    pub(crate) exchanges: Vec<(String, i64)>,
    lists: Vec<WatchList>,
    /// (liste, section), dans l'ordre.
    sections: Vec<(i64, String)>,
    /// (liste, section, identifiant), dans l'ordre d'ajout.
    items: Vec<(i64, String, String)>,
}

impl MemStore {
    /// Les watchlists (noms, sections, symboles) en JSON, pour les garder entre deux visites (même format que l'app
    /// de bureau : `listes::vers_json`).
    pub fn lists_json(&self) -> String {
        crate::listes::vers_json(self)
    }

    /// Base vide avec les watchlists gardées par `lists_json` (texte illisible : base vide).
    pub fn from_lists_json(raw: &str) -> Self {
        let v: Value = serde_json::from_str(raw).unwrap_or_default();
        let rows = |k: &str| v[k].as_array().cloned().unwrap_or_default();
        let s = |x: &Value, k: usize| x[k].as_str().unwrap_or_default().to_string();
        let n = |x: &Value| x[0].as_i64().unwrap_or(0);
        Self {
            lists: rows("lists").iter().map(|x| WatchList { id: n(x), name: s(x, 1) }).collect(),
            sections: rows("sections").iter().map(|x| (n(x), s(x, 1))).collect(),
            items: rows("items").iter().map(|x| (n(x), s(x, 1), s(x, 2))).collect(),
            ..Self::default()
        }
    }
}

impl SymbolStore for MemStore {
    fn save_catalog(&mut self, exchange: &str, list: &[Instrument], now_s: i64) -> Result<()> {
        // tout passe inactif, puis ce que l'exchange liste encore reprend son état
        for i in self.instruments.iter_mut().filter(|i| i.exchange == exchange) {
            i.active = false;
        }
        for n in list.iter().filter(|i| i.exchange == exchange) {
            match self.instruments.iter_mut().find(|i| i.exchange == n.exchange && i.symbol == n.symbol) {
                Some(i) => *i = n.clone(),
                None => self.instruments.push(n.clone()),
            }
        }
        match self.exchanges.iter_mut().find(|e| e.0 == exchange) {
            Some(e) => e.1 = now_s,
            None => self.exchanges.push((exchange.to_string(), now_s)),
        }
        Ok(())
    }

    fn catalogs(&self) -> Result<Vec<CatalogInfo>> {
        let mut v: Vec<CatalogInfo> = self
            .exchanges
            .iter()
            .map(|(e, t)| CatalogInfo {
                exchange: e.clone(),
                updated_at: *t,
                symbols: self.instruments.iter().filter(|i| &i.exchange == e && i.active).count(),
            })
            .collect();
        v.sort_by(|a, b| a.exchange.cmp(&b.exchange));
        Ok(v)
    }

    fn search(&self, s: &Search) -> Result<Vec<Instrument>> {
        let q = s.query.trim().to_uppercase();
        // 🔎 also without case nor separators: « solusdt » finds SOL-USDT-SWAP, « btcusdc » Hyperliquid's BTC (USDC)
        let qn = sans_separateurs(&q);
        let mut found: Vec<&Instrument> = self
            .instruments
            .iter()
            .filter(|i| {
                let net = |s: &str| !qn.is_empty() && sans_separateurs(s).contains(&qn);
                q.is_empty()
                    || i.base.to_uppercase().starts_with(&q)
                    || i.symbol.to_uppercase().contains(&q)
                    || net(&i.symbol)
                    || net(&format!("{}{}", i.base, i.quote))
            })
            .filter(|i| s.exchange.is_none_or(|e| i.exchange == e) && s.group.is_none_or(|g| i.group == g))
            .collect();
        // exact d'abord, actifs avant inactifs, début du nom, plus court, puis exchange et symbole
        found.sort_by_key(|i| {
            let b = i.base.to_uppercase();
            (b != q, !i.active, !b.starts_with(&q), i.symbol.len(), i.exchange.clone(), i.symbol.clone())
        });
        Ok(found.into_iter().take(s.limit).cloned().collect())
    }

    fn groups(&self) -> Result<Vec<(String, usize)>> {
        let mut g: Vec<(String, usize)> = vec![];
        for i in self.instruments.iter().filter(|i| i.active) {
            match g.iter_mut().find(|x| x.0 == i.group) {
                Some(x) => x.1 += 1,
                None => g.push((i.group.clone(), 1)),
            }
        }
        g.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        Ok(g)
    }

    fn instrument(&self, id: &str) -> Result<Option<Instrument>> {
        let Some((ex, sym)) = split_id(id) else { return Ok(None) };
        Ok(self.instruments.iter().find(|i| i.exchange == ex && i.symbol == sym).cloned())
    }

    fn lists(&self) -> Result<Vec<WatchList>> {
        Ok(self.lists.clone())
    }

    fn create_list(&mut self, name: &str) -> Result<i64> {
        if let Some(l) = self.lists.iter().find(|l| l.name == name) {
            return Ok(l.id);
        }
        let id = self.lists.iter().map(|l| l.id).max().unwrap_or(0) + 1;
        self.lists.push(WatchList { id, name: name.to_string() });
        Ok(id)
    }

    fn rename_list(&mut self, list: i64, name: &str) -> Result<()> {
        if let Some(l) = self.lists.iter_mut().find(|l| l.id == list) {
            l.name = name.to_string();
        }
        Ok(())
    }

    fn delete_list(&mut self, list: i64) -> Result<()> {
        self.lists.retain(|l| l.id != list);
        self.sections.retain(|s| s.0 != list);
        self.items.retain(|i| i.0 != list);
        Ok(())
    }

    fn sections(&self, list: i64) -> Result<Vec<String>> {
        Ok(self.sections.iter().filter(|s| s.0 == list).map(|s| s.1.clone()).collect())
    }

    fn add_section(&mut self, list: i64, name: &str) -> Result<()> {
        if !self.sections.iter().any(|s| s.0 == list && s.1 == name) {
            self.sections.push((list, name.to_string()));
        }
        Ok(())
    }

    fn remove_section(&mut self, list: i64, name: &str) -> Result<()> {
        for i in self.items.iter_mut().filter(|i| i.0 == list && i.1 == name) {
            i.1.clear();
        }
        self.sections.retain(|s| !(s.0 == list && s.1 == name));
        Ok(())
    }

    fn rename_section(&mut self, list: i64, name: &str, new: &str) -> Result<()> {
        if new.is_empty() || self.sections.iter().any(|s| s.0 == list && s.1 == new) {
            return Ok(());
        }
        for s in self.sections.iter_mut().filter(|s| s.0 == list && s.1 == name) {
            s.1 = new.to_string();
        }
        for i in self.items.iter_mut().filter(|i| i.0 == list && i.1 == name) {
            i.1 = new.to_string();
        }
        Ok(())
    }

    fn move_section(&mut self, list: i64, name: &str, delta: i32) -> Result<()> {
        let idx: Vec<usize> = (0..self.sections.len()).filter(|&k| self.sections[k].0 == list).collect();
        if let Some(p) = idx.iter().position(|&k| self.sections[k].1 == name) {
            let q = p as i64 + delta as i64;
            if q >= 0 && (q as usize) < idx.len() {
                self.sections.swap(idx[p], idx[q as usize]);
            }
        }
        Ok(())
    }

    fn items(&self, list: i64) -> Result<Vec<(String, String)>> {
        // section sans nom d'abord, puis les sections dans leur ordre ; dans chaque section, l'ordre d'ajout
        let pos = |sec: &str| self.sections.iter().filter(|s| s.0 == list).position(|s| s.1 == sec).map_or(-1, |p| p as i64);
        let mut v: Vec<(i64, usize, String, String)> = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, i)| i.0 == list)
            .map(|(k, i)| (pos(&i.1), k, i.1.clone(), i.2.clone()))
            .collect();
        v.sort_by_key(|x| (x.0, x.1));
        Ok(v.into_iter().map(|x| (x.2, x.3)).collect())
    }

    fn add_item(&mut self, list: i64, section: &str, id: &str) -> Result<()> {
        if split_id(id).is_none() {
            return Err(ArgosError::Invalid(format!("identifiant de symbole invalide : « {id} » (exchange:symbole)")));
        }
        if !section.is_empty() {
            self.add_section(list, section)?;
        }
        if !self.items.iter().any(|i| i.0 == list && i.2 == id) {
            self.items.push((list, section.to_string(), id.to_string()));
        }
        Ok(())
    }

    fn remove_item(&mut self, list: i64, id: &str) -> Result<()> {
        self.items.retain(|i| !(i.0 == list && i.2 == id));
        Ok(())
    }
}
