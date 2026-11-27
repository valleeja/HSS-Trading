//! 🚦 File commune des demandes de bougies (étape 4 du plan de collecte), pour toute l'app :
//! - **haute priorité** : le premier écran d'une chart (`haute`). Jamais en file : il part tout de suite. Le direct
//!   (WebSocket) n'est pas une demande et ne passe jamais par ici ;
//! - **basse priorité** (`basse`) : remplir le passé, les pages plus anciennes, réparer les trous, les cases du split.
//!   **Une seule à la fois par exchange**, et seulement quand aucun premier écran n'est en cours sur cet exchange ;
//! - **partage** (`partage`) : deux fenêtres qui veulent le même marché à la même unité (cases du split, chart
//!   principale) font **une seule** demande ; la deuxième reçoit le résultat de la première.
//!
//! Même code sur le desktop et dans le navigateur : l'attente est une petite pause (`rt::sleep`), sans dépendance de plus.

use crate::Exchange;
use argos_core::Bar;
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::sync::Mutex;
use std::time::Duration;

#[derive(Default)]
struct Etat {
    /// Exchanges qui ont une demande basse priorité en cours.
    basse: HashSet<Exchange>,
    /// Premiers écrans en cours, par exchange.
    hautes: HashMap<Exchange, usize>,
    /// Demandes partagées : en cours (`None`) ou finies (résultat, moment en ms).
    partage: HashMap<String, Option<(Result<Vec<Bar>, String>, u128)>>,
}

static ETAT: Mutex<Option<Etat>> = Mutex::new(None);

fn avec<T>(f: impl FnOnce(&mut Etat) -> T) -> T {
    let mut g = ETAT.lock().unwrap_or_else(|e| e.into_inner());
    f(g.get_or_insert_with(Etat::default))
}

/// Pause entre deux regards à la file (ms).
const PAS_MS: u64 = 40;
/// Un résultat partagé sert encore pendant ce temps (ms) à une fenêtre qui arrive juste après.
const PARTAGE_MS: u128 = 10_000;

fn maintenant_ms() -> u128 {
    web_time::SystemTime::now().duration_since(web_time::UNIX_EPOCH).map_or(0, |d| d.as_millis())
}

/// ⚡ Un premier écran sur `ex` : part tout de suite ; pendant ce temps, la basse priorité de cet exchange attend.
pub async fn haute<T, F: Future<Output = T>>(ex: Exchange, f: F) -> T {
    avec(|e| *e.hautes.entry(ex).or_default() += 1);
    let out = f.await;
    avec(|e| {
        if let Some(n) = e.hautes.get_mut(&ex) {
            *n = n.saturating_sub(1);
        }
    });
    out
}

/// 🐢 Une demande basse priorité sur `ex` : attend qu'aucune autre ne tourne sur cet exchange et qu'aucun premier
/// écran n'y soit en cours, puis part. Les autres exchanges ne sont pas bloqués.
pub async fn basse<T, F: Future<Output = T>>(ex: Exchange, f: impl FnOnce() -> F) -> T {
    loop {
        let pris = avec(|e| {
            let libre = !e.basse.contains(&ex) && e.hautes.get(&ex).copied().unwrap_or(0) == 0;
            if libre {
                e.basse.insert(ex);
            }
            libre
        });
        if pris {
            break;
        }
        crate::rt::sleep(Duration::from_millis(PAS_MS)).await;
    }
    let out = f().await;
    avec(|e| e.basse.remove(&ex));
    out
}

/// 🤝 Une demande partagée par clé (« exchange|symbole|unité ») : si la même est déjà en cours (ou vient de finir), on
/// attend son résultat au lieu d'en refaire une.
pub async fn partage<F: Future<Output = Result<Vec<Bar>, String>>>(cle: &str, f: impl FnOnce() -> F) -> Result<Vec<Bar>, String> {
    loop {
        let deja = avec(|e| match e.partage.get(cle) {
            Some(Some((r, at))) if maintenant_ms().saturating_sub(*at) < PARTAGE_MS => Some(Some(r.clone())),
            Some(None) => Some(None),
            _ => {
                e.partage.insert(cle.to_string(), None);
                None
            }
        });
        match deja {
            Some(Some(r)) => return r,
            Some(None) => crate::rt::sleep(Duration::from_millis(PAS_MS)).await,
            None => break,
        }
    }
    let r = f().await;
    avec(|e| {
        let now = maintenant_ms();
        e.partage.retain(|_, v| v.as_ref().is_none_or(|(_, at)| now.saturating_sub(*at) < PARTAGE_MS));
        e.partage.insert(cle.to_string(), Some((r.clone(), now)));
    });
    r
}

#[cfg(all(test, feature = "native"))]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    /// Une demande qui dure `ms` et compte combien tournent en même temps sur son exchange.
    async fn travail(en_cours: Arc<AtomicUsize>, max: Arc<AtomicUsize>, ms: u64) {
        let n = en_cours.fetch_add(1, Ordering::SeqCst) + 1;
        max.fetch_max(n, Ordering::SeqCst);
        tokio::time::sleep(Duration::from_millis(ms)).await;
        en_cours.fetch_sub(1, Ordering::SeqCst);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn une_seule_basse_priorite_par_exchange() {
        let (en_cours, max) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
        let mut tasks = vec![];
        for _ in 0..4 {
            let (c, m) = (en_cours.clone(), max.clone());
            tasks.push(tokio::spawn(async move { basse(Exchange::Bitget, || travail(c, m, 60)).await }));
        }
        for t in tasks {
            t.await.unwrap();
        }
        assert_eq!(max.load(Ordering::SeqCst), 1, "jamais deux en même temps sur Bitget");
        // deux exchanges différents : en même temps
        let (en_cours, max) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
        let t0 = web_time::Instant::now();
        let (a, b) = (en_cours.clone(), max.clone());
        let x = tokio::spawn(async move { basse(Exchange::Gate, || travail(a, b, 150)).await });
        let y = tokio::spawn(async move { basse(Exchange::Mexc, || async { tokio::time::sleep(Duration::from_millis(150)).await }).await });
        x.await.unwrap();
        y.await.unwrap();
        assert!(t0.elapsed() < Duration::from_millis(280), "Gate et MEXC ne s'attendent pas");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn le_premier_ecran_passe_avant() {
        let ordre = Arc::new(Mutex::new(vec![]));
        let o = ordre.clone();
        let h = tokio::spawn(async move {
            haute(Exchange::Kucoin, async {
                tokio::time::sleep(Duration::from_millis(150)).await;
                o.lock().unwrap().push("premier écran");
            })
            .await
        });
        tokio::time::sleep(Duration::from_millis(20)).await; // le premier écran a commencé
        let o = ordre.clone();
        let b = tokio::spawn(async move { basse(Exchange::Kucoin, || async move { o.lock().unwrap().push("passé") }).await });
        h.await.unwrap();
        b.await.unwrap();
        assert_eq!(*ordre.lock().unwrap(), vec!["premier écran", "passé"]);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn deux_fenetres_une_seule_demande() {
        let appels = Arc::new(AtomicUsize::new(0));
        let mut tasks = vec![];
        for _ in 0..3 {
            let a = appels.clone();
            tasks.push(tokio::spawn(async move {
                partage("essai|BTC|3600", || async move {
                    a.fetch_add(1, Ordering::SeqCst);
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    Ok(vec![])
                })
                .await
            }));
        }
        for t in tasks {
            assert_eq!(t.await.unwrap(), Ok(vec![]));
        }
        assert_eq!(appels.load(Ordering::SeqCst), 1, "3 fenêtres, une seule demande à l'exchange");
    }
}
