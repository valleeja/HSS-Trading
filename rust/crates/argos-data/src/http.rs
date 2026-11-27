//! Client HTTP injecté : la couche 2A ne choisit pas comment aller sur Internet.
//! - desktop / serveur : `NativeHttp` (option `native`, reqwest)
//! - navigateur / cell : un client WASM fourni par la couche 4 (fetch du navigateur)
//! - tests : un faux client qui renvoie des réponses enregistrées

use argos_core::Result;

#[allow(async_fn_in_trait)]
pub trait Http {
    async fn get(&self, url: &str) -> Result<String>;
    async fn post_json(&self, url: &str, body: &str) -> Result<String>;
    /// Pause entre deux pages de bougies (limites de requêtes des exchanges). Par défaut : aucune (tests).
    async fn wait(&self, _ms: u64) {}
}

#[cfg(feature = "native")]
pub use native::NativeHttp;
#[cfg(feature = "web")]
pub use web::FetchHttp;

/// 🛡️ Taille maximale d'une réponse d'exchange (16 Mio) : une réponse folle est refusée, sans faire planter l'app.
/// (La plus grosse réponse normale d'Argos, l'historique du DXY, fait environ 2 Mio.)
pub const MAX_BODY: usize = 16 * 1024 * 1024;

/// ⏳ Attente demandée par l'exchange (en-tête `Retry-After`) : au plus 60 s.
pub const MAX_RETRY_AFTER_S: u64 = 60;

/// La valeur de l'en-tête `Retry-After` en secondes d'attente (au plus `MAX_RETRY_AFTER_S`) : un nombre de secondes, ou
/// une date HTTP (« Wed, 21 Oct 2026 07:28:00 GMT ») comparée à `now_s`. `None` si illisible.
pub fn parse_retry_after(value: &str, now_s: i64) -> Option<u64> {
    let v = value.trim();
    let s = match v.parse::<u64>() {
        Ok(n) => n,
        Err(_) => {
            let t = chrono::DateTime::parse_from_rfc2822(&v.replace("GMT", "+0000")).ok()?.timestamp();
            (t - now_s).max(0) as u64
        }
    };
    Some(s.min(MAX_RETRY_AFTER_S))
}

/// L'attente demandée par l'exchange, lue dans un message d'erreur « … (Retry-After 12 s) … » (en ms).
pub fn retry_after_ms(err: &str) -> Option<u64> {
    let rest = err.split("(Retry-After ").nth(1)?;
    rest.split(" s)").next()?.trim().parse::<u64>().ok().map(|s| s.min(MAX_RETRY_AFTER_S) * 1000)
}

/// ⏩ Runs all `futs` at the same time (one task: no thread, no new crate) and gives their results in the same
/// order. At start-up the catalogue and the prices of the ~10 HIP-3 dexes were asked one after the other.
pub async fn ensemble<T, F: std::future::Future<Output = T>>(futs: Vec<F>) -> Vec<T> {
    use std::task::Poll;
    let mut futs: Vec<std::pin::Pin<Box<F>>> = futs.into_iter().map(Box::pin).collect();
    let mut out: Vec<Option<T>> = futs.iter().map(|_| None).collect();
    std::future::poll_fn(|cx| {
        let mut waiting = false;
        for (f, o) in futs.iter_mut().zip(out.iter_mut()) {
            if o.is_none() {
                match f.as_mut().poll(cx) {
                    Poll::Ready(v) => *o = Some(v),
                    Poll::Pending => waiting = true,
                }
            }
        }
        if waiting {
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    })
    .await;
    out.into_iter().flatten().collect()
}

/// 🎲 Un petit délai au hasard ajouté à une attente (0 à +25 %), pour que des requêtes en attente ne repartent pas
/// toutes au même moment. Le hasard vient de l'horloge (nanosecondes) : aucune dépendance de plus.
pub fn jitter(ms: u64) -> u64 {
    let n = web_time::SystemTime::now()
        .duration_since(web_time::UNIX_EPOCH)
        .map_or(0, |d| d.subsec_nanos() as u64);
    ms + ms * (n % 251) / 1000
}

/// Le message d'erreur d'une réponse trop grosse.
pub fn too_big(n: usize) -> argos_core::ArgosError {
    argos_core::ArgosError::Feed(format!("réponse trop grosse ({} Mio, maximum {} Mio) : refusée", n >> 20, MAX_BODY >> 20))
}

#[cfg(feature = "native")]
mod native {
    use super::Http;
    use argos_core::{ArgosError, Result};

    /// Client natif (rustls), délai de 10 s, identifié « Argos ».
    #[derive(Clone)]
    pub struct NativeHttp(reqwest::Client);

    impl Default for NativeHttp {
        fn default() -> Self {
            let c = reqwest::Client::builder()
                .user_agent("Argos/2.0")
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .expect("client HTTP");
            Self(c)
        }
    }

    async fn text(r: reqwest::Result<reqwest::Response>) -> Result<String> {
        let mut r = r.map_err(|e| ArgosError::Feed(e.to_string()))?;
        let status = r.status();
        // ⏳ « trop de requêtes » / « indisponible » avec l'attente demandée par l'exchange : notée dans l'erreur
        let wait = (status.as_u16() == 429 || status.as_u16() == 503)
            .then(|| r.headers().get("retry-after").and_then(|v| v.to_str().ok()).map(str::to_string))
            .flatten()
            .and_then(|v| super::parse_retry_after(&v, crate::system_now_ms() / 1000))
            .map_or(String::new(), |s| format!(" (Retry-After {s} s)"));
        // taille annoncée trop grosse : refusée avant de lire ; sinon lue morceau par morceau, arrêtée au maximum
        if let Some(n) = r.content_length().filter(|n| *n as usize > super::MAX_BODY) {
            return Err(super::too_big(n as usize));
        }
        let mut bytes: Vec<u8> = vec![];
        while let Some(chunk) = r.chunk().await.map_err(|e| ArgosError::Feed(e.to_string()))? {
            if bytes.len() + chunk.len() > super::MAX_BODY {
                return Err(super::too_big(bytes.len() + chunk.len()));
            }
            bytes.extend_from_slice(&chunk);
        }
        let body = String::from_utf8_lossy(&bytes).into_owned();
        if !status.is_success() {
            return Err(ArgosError::Feed(format!(
                "HTTP {status}{wait} : {}",
                body.chars().take(200).collect::<String>()
            )));
        }
        Ok(body)
    }

    impl Http for NativeHttp {
        async fn get(&self, url: &str) -> Result<String> {
            crate::net::verifier(url)?;
            text(self.0.get(url).send().await).await
        }
        async fn post_json(&self, url: &str, body: &str) -> Result<String> {
            crate::net::verifier(url)?;
            text(self.0.post(url).header("content-type", "application/json").body(body.to_string()).send().await).await
        }
        async fn wait(&self, ms: u64) {
            tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
        }
    }

    /// 🛡️ Une réponse plus grosse que `MAX_BODY` est refusée, annoncée ou non (petit serveur local, sans Internet).
    #[cfg(test)]
    #[tokio::test]
    async fn reponse_trop_grosse_refusee() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = l.local_addr().unwrap().port();
        tokio::spawn(async move {
            for annonce in [true, false, true] {
                let (mut c, _) = l.accept().await.unwrap();
                let mut buf = [0u8; 2048];
                let _ = c.read(&mut buf).await;
                let n = if annonce { super::MAX_BODY + 1 } else { super::MAX_BODY + 64 * 1024 };
                let head = if annonce {
                    format!("HTTP/1.1 200 OK\r\ncontent-length: {n}\r\nconnection: close\r\n\r\n")
                } else {
                    "HTTP/1.1 200 OK\r\nconnection: close\r\n\r\n".to_string()
                };
                let _ = c.write_all(head.as_bytes()).await;
                let chunk = vec![b'x'; 64 * 1024];
                let mut sent = 0;
                while sent < n {
                    if c.write_all(&chunk[..chunk.len().min(n - sent)]).await.is_err() {
                        break;
                    }
                    sent += chunk.len();
                }
            }
        });
        let c = reqwest::Client::new();
        let url = format!("http://127.0.0.1:{port}/");
        for _ in 0..2 {
            let e = text(c.get(&url).send().await).await.unwrap_err().to_string();
            assert!(e.contains("trop grosse"), "{e}");
        }
    }
}

#[cfg(feature = "web")]
mod web {
    use super::Http;
    use argos_core::{ArgosError, Result};
    use wasm_bindgen::{JsCast, JsValue};

    /// Client du navigateur (`fetch`) : mêmes règles que le client natif (texte, erreur si le statut n'est pas OK).
    #[derive(Clone, Default)]
    pub struct FetchHttp;

    fn err(e: JsValue) -> ArgosError {
        ArgosError::Feed(e.as_string().unwrap_or_else(|| "réseau".into()))
    }

    async fn send(url: &str, body: Option<&str>) -> Result<String> {
        crate::net::verifier(url)?;
        // 📱 dans la coque Tauri : la page n'a pas de réseau, le cœur Rust fait la requête (liste blanche, taille max)
        #[cfg(not(feature = "native"))]
        if crate::coque::active() {
            let args = crate::coque::objet(&[("url", JsValue::from_str(url)), ("corps", body.map_or(JsValue::NULL, JsValue::from_str))]);
            return crate::coque::invoke("reseau", &args)
                .await
                .map_err(ArgosError::Feed)?
                .as_string()
                .ok_or_else(|| ArgosError::Feed("réponse du cœur illisible".into()));
        }
        // 🔁 Bitfinex, KuCoin (no CORS): through the PC's server relay when the page found it (GET only)
        // (not known yet at start-up: asked once first, then the request goes the right way; the chart's candles left
        // before the answer and the browser refused them)
        let mut relaye = if body.is_none() { crate::relais::relayer(url) } else { None };
        if relaye.is_some() && !crate::relais::connu() {
            let oui = Box::pin(send(crate::relais::ETAT, None)).await.is_ok_and(|b| crate::relais::lire_etat(&b));
            crate::relais::activer(oui);
        }
        if !crate::relais::actif() {
            relaye = None;
        }
        let url = relaye.as_deref().unwrap_or(url);
        let init = web_sys::RequestInit::new();
        init.set_method(if body.is_some() { "POST" } else { "GET" });
        if let Some(b) = body {
            init.set_body(&JsValue::from_str(b));
            let h = web_sys::Headers::new().map_err(err)?;
            h.set("Content-Type", "application/json").map_err(err)?;
            init.set_headers(&h);
        }
        let w = web_sys::window().ok_or_else(|| ArgosError::Feed("pas de fenêtre".into()))?;
        let resp: web_sys::Response = wasm_bindgen_futures::JsFuture::from(w.fetch_with_str_and_init(url, &init))
            .await
            .map_err(err)?
            .dyn_into()
            .map_err(err)?;
        // taille annoncée trop grosse : refusée avant de lire ; sinon vérifiée après (le navigateur lit tout d'un coup)
        let announced = resp.headers().get("content-length").ok().flatten().and_then(|v| v.parse::<usize>().ok());
        if let Some(n) = announced.filter(|n| *n > super::MAX_BODY) {
            return Err(super::too_big(n));
        }
        let text = wasm_bindgen_futures::JsFuture::from(resp.text().map_err(err)?)
            .await
            .map_err(err)?
            .as_string()
            .unwrap_or_default();
        if text.len() > super::MAX_BODY {
            return Err(super::too_big(text.len()));
        }
        if !resp.ok() {
            return Err(ArgosError::Feed(format!(
                "HTTP {} : {}",
                resp.status(),
                text.chars().take(200).collect::<String>()
            )));
        }
        Ok(text)
    }

    impl Http for FetchHttp {
        async fn get(&self, url: &str) -> Result<String> {
            send(url, None).await
        }
        async fn post_json(&self, url: &str, body: &str) -> Result<String> {
            send(url, Some(body)).await
        }
        async fn wait(&self, ms: u64) {
            crate::rt::sleep(std::time::Duration::from_millis(ms)).await;
        }
    }
}

#[cfg(test)]
mod tests_ensemble {
    /// ⏩ All the futures move together (each is polled while the others wait), the results stay in order, and a
    /// finished one is never polled again.
    #[test]
    fn ensemble_en_meme_temps_dans_l_ordre() {
        use std::cell::RefCell;
        use std::future::Future;
        use std::task::{Context, Poll, Waker};
        let log = RefCell::new(vec![]);
        let step = |id: usize, n: usize| {
            let log = &log;
            let mut left = n;
            std::future::poll_fn(move |cx: &mut Context<'_>| {
                log.borrow_mut().push(id);
                if left == 0 {
                    return Poll::Ready(id * 10);
                }
                left -= 1;
                cx.waker().wake_by_ref();
                Poll::Pending
            })
        };
        let mut f = std::pin::pin!(super::ensemble(vec![step(1, 2), step(2, 0), step(3, 1)]));
        let mut cx = Context::from_waker(Waker::noop());
        let out = loop {
            if let Poll::Ready(v) = f.as_mut().poll(&mut cx) {
                break v;
            }
        };
        assert_eq!(out, [10, 20, 30], "in order");
        assert_eq!(
            *log.borrow(),
            [1, 2, 3, 1, 3, 1],
            "together: 1 and 3 waited side by side; 2 done at once, not polled again"
        );
    }
}
