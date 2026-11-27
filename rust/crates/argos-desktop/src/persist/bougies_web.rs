//! 🕯 The candle cache of the browser (the web app had none: every timeframe seen again was downloaded whole). The
//! same bytes as the PC's cache files (`argos_data::cache::file_bytes`, fingerprint checked), kept in the browser's
//! Cache API (cache « argos-bougies »: its own quota, binary data, never the page's storage where the layouts and
//! drawings live), at most [`MAX_TOTAL`] bytes: the least used series go first. Only closed candles; when a series
//! is opened again, only its last candles are asked for (`cache::missing`). Any error reading or writing it only
//! means a download, as before. The PC keeps its own files (`~/.cache/argos-hss-trading/candles`), untouched.

#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code))] // (used by the web app; tested on the PC)

use argos_core::Bar;
use argos_core::Timeframe;
use argos_data::cache;

/// At most this many bytes of candles in the browser's cache.
pub(crate) const MAX_TOTAL: usize = 30_000_000;
/// Candles kept per series (the history the chart asks for).
pub(crate) const MAX_BARS: usize = argos_data::DEFAULT_CANDLES;

/// The bytes kept for a series: its closed candles (the last [`MAX_BARS`]); `None` if there is none.
pub(crate) fn encode(bars: &[Bar], tf: Timeframe, now_s: i64) -> Option<Vec<u8>> {
    let closed = cache::closed(bars, tf.seconds(), now_s);
    let keep = &closed[closed.len().saturating_sub(MAX_BARS)..];
    (!keep.is_empty()).then(|| cache::file_bytes(keep))
}

/// The closed candles read back; `None` when the bytes are damaged or empty (then: a download, as before).
pub(crate) fn decode(bytes: &[u8], tf: Timeframe, now_s: i64) -> Option<Vec<Bar>> {
    let bars = cache::parse_bytes(bytes)?;
    let closed = cache::closed(&bars, tf.seconds(), now_s);
    (!closed.is_empty()).then(|| closed.to_vec())
}

/// The index of the cache: (series, bytes, last use in seconds).
pub(crate) type Index = Vec<(String, usize, i64)>;

/// Writing `name` (`size` bytes) at `now_s`: the new index and the series to remove (the least used first, until
/// all fits in [`MAX_TOTAL`]; never `name`).
pub(crate) fn plan(mut idx: Index, name: &str, size: usize, now_s: i64) -> (Index, Vec<String>) {
    idx.retain(|e| e.0 != name);
    idx.sort_by_key(|e| e.2);
    let mut gone = vec![];
    while !idx.is_empty() && idx.iter().map(|e| e.1).sum::<usize>() + size > MAX_TOTAL {
        gone.push(idx.remove(0).0);
    }
    idx.push((name.to_string(), size, now_s));
    (idx, gone)
}

/// 🌐 The browser's Cache API (web): read, write, remove; every error ends as `None` (a download, as before).
#[cfg(target_arch = "wasm32")]
pub(crate) mod web {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    const NOM: &str = "argos-bougies";

    fn url(name: &str) -> String {
        format!("/argos-bougies/{name}")
    }

    async fn ouvrir() -> Option<web_sys::Cache> {
        let caches = web_sys::window()?.caches().ok()?;
        JsFuture::from(caches.open(NOM)).await.ok()?.dyn_into().ok()
    }

    async fn lire_url(c: &web_sys::Cache, u: &str) -> Option<Vec<u8>> {
        let r = JsFuture::from(c.match_with_str(u)).await.ok()?;
        let r: web_sys::Response = r.dyn_into().ok()?;
        let buf = JsFuture::from(r.array_buffer().ok()?).await.ok()?;
        Some(js_sys::Uint8Array::new(&buf).to_vec())
    }

    async fn ecrire_url(c: &web_sys::Cache, u: &str, data: &[u8]) -> Option<()> {
        let mut copie = data.to_vec();
        let r = web_sys::Response::new_with_opt_u8_array(Some(&mut copie)).ok()?;
        JsFuture::from(c.put_with_str(u, &r)).await.ok()?;
        Some(())
    }

    async fn index(c: &web_sys::Cache) -> super::Index {
        lire_url(c, &url("index"))
            .await
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    /// The bytes of `name`, its use noted (`None`: absent or any error).
    pub(crate) async fn lire(name: &str, now_s: i64) -> Option<Vec<u8>> {
        let c = ouvrir().await?;
        let data = lire_url(&c, &url(name)).await?;
        let mut idx = index(&c).await;
        if let Some(e) = idx.iter_mut().find(|e| e.0 == name) {
            e.2 = now_s;
        }
        if let Ok(b) = serde_json::to_vec(&idx) {
            let _ = ecrire_url(&c, &url("index"), &b).await;
        }
        Some(data)
    }

    /// Keeps `data` as `name`, the least used series removed to stay under the limit (errors ignored).
    pub(crate) async fn ecrire(name: String, data: Vec<u8>, now_s: i64) {
        let Some(c) = ouvrir().await else { return };
        let (idx, gone) = super::plan(index(&c).await, &name, data.len(), now_s);
        for g in gone {
            let _ = JsFuture::from(c.delete_with_str(&url(&g))).await;
        }
        if ecrire_url(&c, &url(&name), &data).await.is_some() {
            if let Ok(b) = serde_json::to_vec(&idx) {
                let _ = ecrire_url(&c, &url("index"), &b).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bars(n: i64, step: i64) -> Vec<Bar> {
        (0..n)
            .map(|i| Bar {
                time: 1_700_000_000 + i * step,
                open: 1.0 + i as f64,
                high: 2.0 + i as f64,
                low: 0.5 + i as f64,
                close: 1.5 + i as f64,
                volume: 3.0,
            })
            .collect()
    }

    /// Kept and read back the same (closed candles only, the last MAX_BARS); damaged or empty bytes give `None`, so
    /// the chart just downloads, as before; the least used series go first past the limit, never the one written.
    #[test]
    fn cache_des_bougies_du_navigateur() {
        let b = bars(500, 3600);
        let now = b.last().unwrap().time + 1800; // the last candle still open
        let data = encode(&b, Timeframe::H1, now).expect("bytes");
        assert_eq!(
            decode(&data, Timeframe::H1, now).expect("read back"),
            b[..499].to_vec(),
            "closed candles, the same"
        );
        // a read error: no candles, never a panic (the download happens as before)
        assert!(decode(b"!! not a cache", Timeframe::H1, now).is_none());
        assert!(decode(&data[..data.len() / 2], Timeframe::H1, now).is_none(), "cut short");
        assert!(encode(&[], Timeframe::H1, now).is_none());
        let long = bars(MAX_BARS as i64 + 300, 3600);
        let d = decode(&encode(&long, Timeframe::H1, i64::MAX / 2).unwrap(), Timeframe::H1, i64::MAX / 2).unwrap();
        assert_eq!(d.len(), MAX_BARS, "the last MAX_BARS");
        // the limit: the least used go first, never the one written
        let mut idx: Index = vec![];
        for k in 0..100 {
            let (n, gone) = plan(idx, &format!("s{k}"), 1_000_000, k);
            assert!(!gone.contains(&format!("s{k}")));
            idx = n;
        }
        assert!(idx.iter().map(|e| e.1).sum::<usize>() <= MAX_TOTAL);
        assert!(idx.iter().any(|e| e.0 == "s99") && !idx.iter().any(|e| e.0 == "s0"), "the oldest gone");
    }
}
