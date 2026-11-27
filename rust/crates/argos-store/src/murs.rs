//! 🧱 Argos Murs recordings: the wall events of each market (`argos_core::murs::Event`, compact JSON), so the
//! indicator can draw the same walls in replay. Order books have no history at the exchanges: only what Argos saw
//! while it was open exists. Its own file `murs.redb` beside `argos.redb`; kept a number of days (30 by default).

use argos_core::murs::Event;
use argos_core::{ArgosError, Result};
use redb::backends::InMemoryBackend;
use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
use std::path::{Path, PathBuf};

/// (market « BTC|perp », time ms, number within that ms) → event (JSON).
const EVENTS: TableDefinition<(&str, i64, u32), &str> = TableDefinition::new("murs_events");

fn err(e: impl Into<redb::Error>) -> ArgosError {
    ArgosError::Invalid(format!("murs enregistrés : {}", e.into()))
}

/// The recordings.
pub struct MursDb {
    db: Database,
}

/// `murs.redb` beside the main base.
pub fn default_path() -> Option<PathBuf> {
    Some(crate::default_path()?.with_file_name("murs.redb"))
}

impl MursDb {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(d) = path.parent() {
            std::fs::create_dir_all(d).map_err(|e| ArgosError::Invalid(format!("dossier des murs : {e}")))?;
        }
        Self::setup(Database::create(path).map_err(err)?)
    }

    /// In memory (tests).
    pub fn memory() -> Result<Self> {
        Self::setup(Database::builder().create_with_backend(InMemoryBackend::new()).map_err(err)?)
    }

    fn setup(db: Database) -> Result<Self> {
        let tx = db.begin_write().map_err(err)?;
        tx.open_table(EVENTS).map_err(err)?;
        tx.commit().map_err(err)?;
        Ok(Self { db })
    }

    /// Adds the events of `market` (one write for all).
    pub fn add(&self, market: &str, events: &[Event]) -> Result<()> {
        if events.is_empty() {
            return Ok(());
        }
        let tx = self.db.begin_write().map_err(err)?;
        {
            let mut t = tx.open_table(EVENTS).map_err(err)?;
            for e in events {
                let j = serde_json::to_string(e).map_err(|x| ArgosError::Invalid(x.to_string()))?;
                // several events in the same millisecond: the next free number
                let mut n = 0u32;
                while t.get((market, e.t_ms, n)).map_err(err)?.is_some() {
                    n += 1;
                }
                t.insert((market, e.t_ms, n), j.as_str()).map_err(err)?;
            }
        }
        tx.commit().map_err(err)
    }

    /// Events of `market` from `from_ms` to `to_ms` (both included), in time order.
    pub fn read(&self, market: &str, from_ms: i64, to_ms: i64) -> Result<Vec<Event>> {
        let rx = self.db.begin_read().map_err(err)?;
        let t = rx.open_table(EVENTS).map_err(err)?;
        let mut out = vec![];
        for row in t.range((market, from_ms, 0)..=(market, to_ms, u32::MAX)).map_err(err)? {
            let (_, v) = row.map_err(err)?;
            if let Ok(e) = serde_json::from_str::<Event>(v.value()) {
                out.push(e);
            }
        }
        Ok(out)
    }

    /// Forgets every event older than `before_ms` (all markets); gives how many went.
    pub fn purge(&self, before_ms: i64) -> Result<usize> {
        let tx = self.db.begin_write().map_err(err)?;
        let n;
        {
            let mut t = tx.open_table(EVENTS).map_err(err)?;
            let old: Vec<(String, i64, u32)> = t
                .iter()
                .map_err(err)?
                .filter_map(|r| r.ok())
                .map(|(k, _)| {
                    let (m, ts, i) = k.value();
                    (m.to_string(), ts, i)
                })
                .filter(|k| k.1 < before_ms)
                .collect();
            n = old.len();
            for (m, ts, i) in old {
                t.remove((m.as_str(), ts, i)).map_err(err)?;
            }
        }
        tx.commit().map_err(err)?;
        Ok(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use argos_core::murs::{Fin, Kind};

    fn ev(t_ms: i64, kind: Kind) -> Event {
        Event {
            t_ms,
            key: 605,
            step: 100.0,
            sell: true,
            spot: false,
            usd: 3.0e6,
            kind,
            venues: 3,
        }
    }

    #[test]
    fn enregistre_relit_et_oublie() {
        let db = MursDb::memory().unwrap();
        db.add("BTC|perp", &[ev(1_000, Kind::Show), ev(1_000, Kind::Size), ev(5_000, Kind::End(Fin::Spoof(2)))])
            .unwrap();
        db.add("ETH|perp", &[ev(2_000, Kind::Show)]).unwrap();
        let all = db.read("BTC|perp", 0, i64::MAX).unwrap();
        assert_eq!(all.len(), 3, "two events in the same millisecond are both kept");
        assert_eq!(all[2].kind, Kind::End(Fin::Spoof(2)));
        assert_eq!(db.read("BTC|perp", 0, 4_999).unwrap().len(), 2, "never past the clock");
        assert_eq!(db.read("ETH|perp", 0, i64::MAX).unwrap().len(), 1);
        assert_eq!(db.purge(3_000).unwrap(), 3);
        assert_eq!(db.read("BTC|perp", 0, i64::MAX).unwrap().len(), 1);
        assert!(db.read("SOL|perp", 0, i64::MAX).unwrap().is_empty());
    }
}
