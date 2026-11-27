//! 📱 Coque Tauri 2 d'Argos HSS Trading.
//! - T0 : une page minimale (`index.html`) qui affiche le module EMA calculé dans le cœur Rust (`noyau`).
//! - T1 : la chart complète (`app/index.html`, la version WASM de `/app/`). La page n'a aucun réseau : ses requêtes,
//!   son direct et ses modules passent par le cœur (`passerelle`), avec la liste blanche d'Argos et la bulle du kit.
//! - Session partagée : les mêmes réglages que l'app de bureau, lus et écrits par le cœur (`reglages`).
//!
//! Seulement les commandes listées dans `build.rs`, permises par `capabilities/default.json` ; aucun plugin.

#![deny(unsafe_code)]

pub mod noyau;
pub mod passerelle;
pub mod reglages;

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use tauri::ipc::Channel;

/// 🧩 T0 : la page demande la courbe EMA (longueur 1 à 500) ; `en_ligne` : bougies réelles si possible.
#[tauri::command]
async fn module_ema(longueur: i64, en_ligne: bool) -> Result<noyau::Vue, String> {
    tauri::async_runtime::spawn_blocking(move || noyau::vue_ema(longueur, en_ligne))
        .await
        .map_err(|e| e.to_string())?
}

/// 🌐 T1 : une requête de la page (liste blanche d'Argos), faite par le cœur.
#[tauri::command]
async fn reseau(url: String, corps: Option<String>) -> Result<String, String> {
    passerelle::reseau(&url, corps.as_deref()).await
}

/// 🧩 T1 : un module du kit Argos lancé dans la bulle du cœur (aucun droit dans la coque).
#[tauri::command]
async fn module_lancer(
    etat: tauri::State<'_, Arc<passerelle::Modules>>,
    cle: String,
    wasm: String,
    manifeste: String,
    entree: String,
) -> Result<String, String> {
    let mods = Arc::clone(&etat);
    tauri::async_runtime::spawn_blocking(move || mods.lancer(&cle, &wasm, &manifeste, &entree))
        .await
        .map_err(|e| e.to_string())?
}

/// Les directs ouverts : leur drapeau « arrête-toi ».
#[derive(Default)]
struct Directs {
    suivant: AtomicU32,
    ouverts: Mutex<HashMap<u32, Arc<AtomicBool>>>,
}

/// Un message du direct pour la page : `c` connecté, `t` un texte de l'exchange, `d` coupé (la raison).
#[derive(Clone, serde::Serialize)]
struct Msg {
    k: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    t: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    r: Option<String>,
}

/// 📡 T1 : le cœur ouvre le direct (WebSocket de la liste blanche), s'abonne, pingue, répond aux contrôles et se
/// reconnecte seul ; chaque message part vers la page par `canal`. Rend le numéro à donner à `direct_fermer`.
#[tauri::command]
fn direct_ouvrir(etat: tauri::State<'_, Arc<Directs>>, abonnement: String, canal: Channel<Msg>) -> Result<u32, String> {
    let sub: argos_data::live::LiveSub = serde_json::from_str(&abonnement).map_err(|e| e.to_string())?;
    if !argos_data::net::autorise(&sub.url) {
        return Err(format!("adresse hors de la liste blanche : {}", sub.url));
    }
    let id = etat.suivant.fetch_add(1, Ordering::Relaxed);
    let stop = Arc::new(AtomicBool::new(false));
    etat.ouverts.lock().map_err(|_| "direct indisponible")?.insert(id, Arc::clone(&stop));
    let (stop2, canal2, directs) = (Arc::clone(&stop), canal.clone(), Arc::clone(&etat));
    // le texte part tel quel vers la page, qui le lit avec le même code qu'avant ; une page fermée (envoi impossible)
    // ou `direct_fermer` arrêtent le fil
    let parse = move |t: &str| {
        if canal2
            .send(Msg {
                k: "t",
                t: Some(t.to_string()),
                r: None,
            })
            .is_err()
        {
            stop2.store(true, Ordering::Relaxed);
        }
        stop2.load(Ordering::Relaxed).then(|| argos_data::live::LiveEvent::Disconnected("fermé".into()))
    };
    let on_event = move |ev: argos_data::live::LiveEvent| {
        if stop.load(Ordering::Relaxed) {
            if let Ok(mut o) = directs.ouverts.lock() {
                o.remove(&id);
            }
            return false;
        }
        let m = match ev {
            argos_data::live::LiveEvent::Connected => Msg { k: "c", t: None, r: None },
            argos_data::live::LiveEvent::Disconnected(r) => Msg { k: "d", t: None, r: Some(r) },
            _ => return true,
        };
        canal.send(m).is_ok()
    };
    argos_data::live::spawn(sub, parse, on_event);
    Ok(id)
}

/// 📡 T1 : la page n'a plus besoin de ce direct.
#[tauri::command]
fn direct_fermer(etat: tauri::State<'_, Arc<Directs>>, id: u32) {
    if let Some(s) = etat.ouverts.lock().ok().and_then(|mut o| o.remove(&id)) {
        s.store(true, Ordering::Relaxed);
    }
}

/// 🔄 Les réglages partagés (dossier de l'app de bureau), ou aucun si le dossier est introuvable.
type Reglages = Arc<Option<reglages::Journal>>;

fn journal(etat: &Reglages) -> Result<&reglages::Journal, String> {
    etat.as_ref().as_ref().ok_or_else(|| "pas de dossier de réglages".to_string())
}

/// 🔄 Tous les réglages de session (au démarrage de la page).
#[tauri::command]
async fn reglages_tout(etat: tauri::State<'_, Reglages>) -> Result<reglages::Reponse, String> {
    let r = Arc::clone(&etat);
    tauri::async_runtime::spawn_blocking(move || journal(&r).map(|j| j.tout()))
        .await
        .map_err(|e| e.to_string())?
}

/// 🔄 Un réglage changé dans la page.
#[tauri::command]
async fn reglage_ecrire(etat: tauri::State<'_, Reglages>, nom: String, texte: String) -> Result<u64, String> {
    let r = Arc::clone(&etat);
    tauri::async_runtime::spawn_blocking(move || {
        let j = journal(&r)?;
        reglages::ecrire(&j.dossier, &nom, &texte)?;
        Ok(j.scanner(false))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// 🔄 Ce qui a changé ailleurs depuis `depuis` (attend jusqu'à 25 s).
#[tauri::command]
async fn reglages_changements(etat: tauri::State<'_, Reglages>, e: String, depuis: u64) -> Result<reglages::Reponse, String> {
    let r = Arc::clone(&etat);
    tauri::async_runtime::spawn_blocking(move || journal(&r).map(|j| j.changements(&e, depuis, std::time::Duration::from_secs(25))))
        .await
        .map_err(|e| e.to_string())?
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(Arc::new(passerelle::Modules::default()))
        .manage(Arc::new(Directs::default()))
        .manage::<Reglages>(Arc::new(reglages::dossier().map(reglages::Journal::new)))
        .invoke_handler(tauri::generate_handler![
            module_ema,
            reseau,
            module_lancer,
            direct_ouvrir,
            direct_fermer,
            reglages_tout,
            reglage_ecrire,
            reglages_changements
        ])
        .run(tauri::generate_context!())
        .expect("coque Tauri d'Argos");
}
