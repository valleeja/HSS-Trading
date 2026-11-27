//! 💲 Prix des autres exchanges (dernier prix + prix d'il y a 24 h, pour tous leurs marchés en une requête) : la
//! watchlist et la fiche du symbole s'en servent pour les symboles hors Hyperliquid (Hyperliquid a son propre flux,
//! module `watchlist`). Seuls les exchanges demandés (`wanted`, changé par la fenêtre) sont rafraîchis.

#[cfg(any(feature = "native", feature = "web"))]
pub use native::spawn;

/// Pause entre deux tours (secondes).
pub const EVERY_S: u64 = 5;

#[cfg(any(feature = "native", feature = "web"))]
mod native {
    use super::EVERY_S;
    use crate::exchanges::Request;
    use crate::http::Http;
    use crate::Exchange;
    use argos_core::feed::Market;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    /// Lance le rafraîchissement dans son propre fil : `on_prices` reçoit les marchés de chaque exchange demandé dans
    /// `wanted` (Hyperliquid ignoré). Le fil s'arrête quand `on_prices` renvoie `false`.
    pub fn spawn(wanted: Arc<Mutex<Vec<Exchange>>>, on_prices: impl Fn(Exchange, Vec<Market>) -> bool + Send + 'static) {
        crate::rt::spawn(move || async move {
            let http = crate::rt::Client::default();
            loop {
                let list: Vec<Exchange> = wanted.lock().map(|w| w.clone()).unwrap_or_default();
                for ex in list.into_iter().filter(|e| *e != Exchange::Hyperliquid) {
                    let body = match ex.markets_request() {
                        Request::Get(url) => http.get(&url).await,
                        Request::PostJson { url, body } => http.post_json(&url, &body).await,
                    };
                    if let Ok(m) = body.and_then(|b| ex.parse_markets(&b)) {
                        if !on_prices(ex, m) {
                            return;
                        }
                    }
                }
                crate::rt::sleep(Duration::from_secs(if crate::rt::phone() { 60 } else { EVERY_S })).await;
                // 📱 téléphone : à la minute
            }
        });
    }
}
