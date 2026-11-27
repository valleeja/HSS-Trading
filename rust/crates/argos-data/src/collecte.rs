//! 🧺 La collecte des bougies (étapes 1 à 7 du plan) pour un **cœur Rust sans fenêtre**, comme celui de la coque
//! Tauri (étape 8). La page web n'y touche jamais : elle reçoit seulement les bougies prêtes, par l'IPC.
//!
//! 1. `premier_ecran` : le cache (bougies fermées, empreinte, 0600), puis seulement ce qui manque (500 au plus), en
//!    haute priorité, par le superviseur (vérification, 5 %, secours, Retry-After) ; le cache est mis à jour.
//!    Sans réseau : les bougies du cache, marquées « hors ligne ».
//! 2. `completer` : en basse priorité (file commune, une demande à la fois par exchange), le passé jusqu'à 5000
//!    bougies, puis la réparation des trous (marché fermé laissé de côté ; un trou que l'exchange n'avait pas est
//!    réessayé une fois par semaine) ; le cache est mis à jour.
//!
//! Même code que l'app de bureau : `cache`, `file`, `supervisor`, `trous`.

use crate::supervisor::Supervisor;
use crate::{cache, file, trous, Exchange, ExchangeFeed, Http, DEFAULT_CANDLES, FIRST_SCREEN};
use argos_core::{Bar, Timeframe};
use std::path::{Path, PathBuf};

/// Les bougies d'un marché et d'où elles viennent.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Collecte {
    pub bars: Vec<Bar>,
    /// Exchange qui a fourni les bougies fraîches (ou « cache »).
    pub source: String,
    /// Combien venaient du cache.
    pub depuis_cache: usize,
    /// Le réseau n'a pas répondu : seulement le cache.
    pub hors_ligne: bool,
    /// Heures des bougies refusées par la vérification (trous à réparer).
    pub a_reparer: Vec<i64>,
}

fn chemin(dossier: Option<&Path>, ex: Exchange, base: &str, tf: Timeframe) -> Option<PathBuf> {
    dossier.map(|d| d.join(cache::file_name(ex, base, tf)))
}

/// ⚡ Premier écran : le cache, puis ce qui manque depuis (500 bougies au plus), en haute priorité. `dossier` : le
/// dossier du cache (`cache::dir()`, ou un dossier temporaire dans les tests ; `None` = sans cache).
pub async fn premier_ecran<H: Http + Clone>(http: H, ex: Exchange, base: &str, tf: Timeframe, dossier: Option<&Path>, now_s: i64) -> Collecte {
    let path = chemin(dossier, ex, base, tf);
    let cached = path.as_deref().and_then(|p| cache::read(p, tf, now_s)).unwrap_or_default();
    let limit = cache::missing(&cached, now_s, tf, DEFAULT_CANDLES).min(FIRST_SCREEN);
    match file::haute(ex, Supervisor::new(http).candles(ex, base, tf, limit)).await {
        Ok(s) => {
            let primaire = !s.is_fallback();
            // les fraîches remplacent le cache à partir de leur première heure ; un secours ne se mélange pas au cache
            let bars = if primaire {
                cache::merge(&cached, s.bars, tf, DEFAULT_CANDLES)
            } else {
                s.bars
            };
            if let (true, Some(p)) = (primaire, &path) {
                let _ = cache::write(p, &bars, tf, now_s);
            }
            Collecte {
                bars,
                source: s.source.name().to_string(),
                depuis_cache: if primaire { cached.len() } else { 0 },
                hors_ligne: false,
                a_reparer: s.a_reparer,
            }
        }
        Err(_) => Collecte {
            depuis_cache: cached.len(),
            bars: cached,
            source: "cache".into(),
            hors_ligne: true,
            a_reparer: vec![],
        },
    }
}

/// 🐢 Après le premier écran, en basse priorité : le passé jusqu'à 5000 bougies, puis les trous réparés (sauf un
/// marché fermé). Rend les bougies complétées (et met le cache à jour).
#[allow(clippy::too_many_arguments)]
pub async fn completer<H: Http + Clone>(
    http: H,
    ex: Exchange,
    base: &str,
    tf: Timeframe,
    dossier: Option<&Path>,
    mut bars: Vec<Bar>,
    a_reparer: &[i64],
    now_s: i64,
) -> Vec<Bar> {
    let feed = ExchangeFeed::new(ex, http);
    let need = DEFAULT_CANDLES.saturating_sub(bars.len());
    if let (true, Some(first)) = (need > 0, bars.first().map(|b| b.time)) {
        if let Ok(older) = file::basse(ex, || feed.older(base, tf, first, need)).await {
            let older: Vec<Bar> = older.into_iter().filter(|b| b.time < first).collect();
            bars = [older, bars].concat();
        }
    }
    // trous que l'exchange n'avait pas : laissés de côté une semaine, puis réessayés
    let fichier_connus = chemin(dossier, ex, base, tf).map(|p| trous::chemin_connus(&p));
    let mut connus = fichier_connus.as_deref().map(trous::lire_connus).unwrap_or_default();
    let g = trous::trouver(&bars, tf, a_reparer, trous::calendrier(ex, base), &trous::a_eviter(&connus, now_s));
    if !g.is_empty() {
        let r = file::basse(ex, || trous::reparer(&feed, base, tf, &g, 10)).await;
        bars = trous::combler(&bars, &r.bars);
        // un trou réparé n'est plus « connu » ; un trou encore absent prend la date de cet essai
        connus.retain(|c| !g.contains(&c.trou) || r.irreparables.contains(&c.trou));
        trous::noter(&mut connus, &r.irreparables, now_s);
        if let Some(p) = &fichier_connus {
            let _ = trous::ecrire_connus(p, &connus);
        }
    }
    if let Some(p) = chemin(dossier, ex, base, tf) {
        let _ = cache::write(&p, &bars, tf, now_s);
    }
    bars
}

#[cfg(test)]
mod tests {
    use super::*;
    use argos_core::{ArgosError, Result};
    use std::sync::Mutex;

    const H: i64 = 3600;

    /// Faux Binance : des bougies d'une heure jusqu'à `fin` (pas après), sauf les heures de `absentes` ; panne si `coupe`.
    struct Faux {
        fin: i64,
        absentes: Vec<i64>,
        coupe: bool,
        /// Réponse fixe (200) : vide, ou un corps d'erreur.
        reponse: Option<&'static str>,
        appels: Mutex<usize>,
    }
    impl Faux {
        fn new(fin: i64) -> Self {
            Faux {
                fin,
                absentes: vec![],
                coupe: false,
                reponse: None,
                appels: Mutex::new(0),
            }
        }
    }
    impl Http for &Faux {
        async fn get(&self, url: &str) -> Result<String> {
            *self.appels.lock().unwrap() += 1;
            if self.coupe {
                return Err(ArgosError::Feed("réseau coupé".into()));
            }
            if let Some(r) = self.reponse {
                return Ok(r.to_string());
            }
            if !url.contains("binance") {
                return Err(ArgosError::Feed("HTTP 503".into()));
            }
            let q = |k: &str| url.split(&format!("{k}=")).nth(1).map(|v| v.split('&').next().unwrap().parse::<i64>().unwrap());
            let limit = q("limit").unwrap();
            let end = q("endTime").map_or(self.fin, |e| (e / 1000).min(self.fin));
            let last = end / H * H;
            let rows: Vec<String> = (0..limit)
                .rev()
                .map(|k| last - k * H)
                .filter(|t| *t > 1_000_000 && !self.absentes.contains(t))
                .map(|t| format!(r#"[{},"10","11","9","10.5","1"]"#, t * 1000))
                .collect();
            Ok(format!("[{}]", rows.join(",")))
        }
        async fn post_json(&self, _: &str, _: &str) -> Result<String> {
            Err(ArgosError::Feed("pas de POST".into()))
        }
    }

    /// Maintenant, sauf dans les 2 premières minutes d'une heure (marge de l'horloge du cache : la bougie qui vient
    /// de finir n'y est pas encore « fermée ») : on recule alors juste avant l'heure, pour que les tests ne dépendent
    /// pas de la minute où ils tournent.
    fn now() -> i64 {
        let r = crate::system_now_ms() / 1000;
        if r % H < cache::MARGE_HORLOGE_S + 60 {
            r - r % H - 60
        } else {
            r
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn premier_ecran_puis_cache_puis_complet() {
        let dir = std::env::temp_dir().join(format!("argos-collecte-{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok(); // un échec passé pouvait en laisser un
        let n = now();
        let faux = Faux::new(n);
        // 1er lancement : rien en cache → 500 bougies (premier écran), écrites dans le cache (bougies fermées)
        let c = premier_ecran(&faux, Exchange::Binance, "BTC", Timeframe::H1, Some(&dir), n).await;
        assert_eq!((c.bars.len(), c.depuis_cache, c.hors_ligne, c.source.as_str()), (500, 0, false, "Binance"));
        // passé et trous : 5000 bougies, cache à jour
        let tout = completer(&faux, Exchange::Binance, "BTC", Timeframe::H1, Some(&dir), c.bars, &c.a_reparer, n).await;
        assert_eq!(tout.len(), DEFAULT_CANDLES);
        // 2e lancement : le cache sert (4999 fermées), seulement ce qui manque est demandé
        let avant = *faux.appels.lock().unwrap();
        let c = premier_ecran(&faux, Exchange::Binance, "BTC", Timeframe::H1, Some(&dir), n).await;
        assert!(c.depuis_cache >= DEFAULT_CANDLES - 1, "{}", c.depuis_cache);
        assert_eq!(c.bars.len(), DEFAULT_CANDLES);
        assert_eq!(*faux.appels.lock().unwrap() - avant, 1, "une seule petite demande");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn hors_ligne_le_cache_suffit() {
        let dir = std::env::temp_dir().join(format!("argos-collecte-hl-{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok(); // un échec passé pouvait en laisser un
        let n = now();
        let mut faux = Faux::new(n);
        let c = premier_ecran(&faux, Exchange::Binance, "BTC", Timeframe::H1, Some(&dir), n).await;
        assert!(!c.hors_ligne && c.bars.len() == 500);
        faux.coupe = true;
        let c = premier_ecran(&faux, Exchange::Binance, "BTC", Timeframe::H1, Some(&dir), n).await;
        assert!(c.hors_ligne, "plus de réseau");
        assert_eq!((c.bars.len(), c.source.as_str()), (499, "cache"), "les bougies fermées du cache");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn trous_repares_en_completant() {
        let dir = std::env::temp_dir().join(format!("argos-collecte-trou-{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok(); // un échec passé pouvait en laisser un
        let n = now();
        let fin = n / H * H;
        // des bougies du milieu manquent dans la 1re réponse, puis l'exchange les a
        let mut faux = Faux::new(n);
        faux.absentes = vec![fin - 100 * H, fin - 101 * H];
        let c = premier_ecran(&faux, Exchange::Binance, "BTC", Timeframe::H1, Some(&dir), n).await;
        assert!(c.bars.iter().all(|b| b.time != fin - 100 * H));
        faux.absentes.clear();
        let tout = completer(&faux, Exchange::Binance, "BTC", Timeframe::H1, Some(&dir), c.bars, &[], n).await;
        assert!(
            tout.iter().any(|b| b.time == fin - 100 * H) && tout.iter().any(|b| b.time == fin - 101 * H),
            "trou réparé"
        );
        assert!(tout.windows(2).all(|w| w[1].time - w[0].time == H), "plus aucun trou");
        std::fs::remove_dir_all(&dir).ok();
    }

    /// 🧾 Revue R&D, point 6 : une réponse vide, ou un « 200 » avec un corps d'erreur, ne vident jamais le cache et ne
    /// passent pas pour « aucune bougie » : le cache reste, le secours est essayé, « hors ligne » si rien ne répond.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn reponse_vide_ou_erreur_ne_vide_pas_le_cache() {
        for (k, mauvaise) in ["[]", r#"{"code":-1121,"msg":"Invalid symbol."}"#].into_iter().enumerate() {
            let dir = std::env::temp_dir().join(format!("argos-collecte-vide-{k}-{}", std::process::id()));
            std::fs::remove_dir_all(&dir).ok(); // un échec passé pouvait en laisser un
            let n = now();
            let mut faux = Faux::new(n);
            let c = premier_ecran(&faux, Exchange::Binance, "BTC", Timeframe::H1, Some(&dir), n).await;
            assert_eq!(c.bars.len(), 500);
            faux.reponse = Some(mauvaise);
            let c = premier_ecran(&faux, Exchange::Binance, "BTC", Timeframe::H1, Some(&dir), n).await;
            assert!(c.hors_ligne, "{mauvaise} : rien d'utilisable");
            assert_eq!((c.bars.len(), c.source.as_str()), (499, "cache"), "{mauvaise} : le cache reste");
            // et le fichier du cache est toujours là, intact
            let p = dir.join(cache::file_name(Exchange::Binance, "BTC", Timeframe::H1));
            assert_eq!(cache::read(&p, Timeframe::H1, n).map(|b| b.len()), Some(499), "{mauvaise}");
            std::fs::remove_dir_all(&dir).ok();
        }
    }
}
