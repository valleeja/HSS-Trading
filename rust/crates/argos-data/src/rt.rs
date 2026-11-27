//! ⚙️ Tâches en arrière-plan, écrites une seule fois pour le desktop et le navigateur :
//! - desktop (`native`) : un fil à part avec tokio ;
//! - navigateur (`web`, WASM) : la boucle de la page (`spawn_local`), minuterie `setTimeout`.
//!
//! `Client` est le client HTTP de la plateforme (reqwest ou `fetch`).

use std::future::Future;
use std::time::Duration;

#[cfg(feature = "native")]
pub type Client = crate::http::NativeHttp;
#[cfg(all(feature = "web", not(feature = "native")))]
pub type Client = crate::http::FetchHttp;

/// Lance une tâche en arrière-plan (`task` fabrique le futur dans le fil ou la boucle qui le fait rouler).
pub fn spawn<F: Future<Output = ()> + 'static>(task: impl FnOnce() -> F + Send + 'static) {
    #[cfg(feature = "native")]
    std::thread::spawn(move || {
        if let Ok(rt) = tokio::runtime::Builder::new_current_thread().enable_all().build() {
            rt.block_on(task());
        }
    });
    #[cfg(all(feature = "web", not(feature = "native")))]
    wasm_bindgen_futures::spawn_local(task());
}

/// Fait rouler un futur jusqu'au bout dans ce fil (le serveur local, qui répond fil par fil) ; `None` si tokio ne
/// démarre pas.
#[cfg(feature = "native")]
pub fn bloquer<F: Future>(f: F) -> Option<F::Output> {
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().ok()?;
    Some(rt.block_on(f))
}

/// Attend `d` sans bloquer.
pub async fn sleep(d: Duration) {
    #[cfg(feature = "native")]
    tokio::time::sleep(d).await;
    #[cfg(all(feature = "web", not(feature = "native")))]
    {
        let ms = d.as_millis().min(i32::MAX as u128) as i32;
        timer(ms).await;
        // 📱 app en arrière-plan (autre app, écran éteint) : les relevés attendent son retour au lieu de tirer
        // des données et la batterie pour rien
        while hidden() {
            timer(2_000).await;
        }
    }
}

/// 🙈 The page is hidden (another tab, another app, screen off). Always `false` on the desktop.
pub fn hidden() -> bool {
    #[cfg(all(feature = "web", not(feature = "native")))]
    return web_sys::window().and_then(|w| w.document()).is_some_and(|d| d.hidden());
    #[cfg(not(all(feature = "web", not(feature = "native"))))]
    false
}

/// 📱 Ouvert sur un téléphone (navigateur d'iPhone ou d'Android) : les relevés de fond s'espacent pour
/// économiser les données mobiles et la batterie. Toujours `false` sur le desktop.
pub fn phone() -> bool {
    #[cfg(all(feature = "web", not(feature = "native")))]
    {
        let ua = web_sys::window().and_then(|w| w.navigator().user_agent().ok()).unwrap_or_default();
        ["iPhone", "iPad", "iPod", "Android", "Mobile"].iter().any(|k| ua.contains(k))
    }
    #[cfg(not(all(feature = "web", not(feature = "native"))))]
    false
}

/// Minuterie `setTimeout` de la page.
#[cfg(all(feature = "web", not(feature = "native")))]
async fn timer(ms: i32) {
    let p = js_sys::Promise::new(&mut |done, _| {
        if let Some(w) = web_sys::window() {
            let _ = w.set_timeout_with_callback_and_timeout_and_arguments_0(&done, ms);
        }
    });
    let _ = wasm_bindgen_futures::JsFuture::from(p).await;
}
