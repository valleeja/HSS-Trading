//! 🔁 `/api/bourse/<exchange>/<path>`: the exchanges relay for the web page (Bitfinex and KuCoin send no CORS
//! header). The closed lists, the cache and the cap are `argos_data::relais` (shared with the page); here only the
//! server's side: GET only (argos-outils `RelaisGet`), same access as `/api/journal` (this computer or Tailscale, page
//! address checked), the exchange asked with the app's network client (white list, no header nor cookie of the
//! browser, 10 s at most), answers of more than `MAX_REPONSE` refused. `/api/bourse/etat`: the page finds the relay.

use argos_data::relais::{self, Decision, Garde};
use argos_outils::serveur::{RelaisGet, Serveur};
use std::sync::{Arc, Mutex};

/// GET (exchange address) → body, or the error.
pub type Appeler = fn(&str) -> Result<String, String>;

/// The real call: the app's native client (white list, 10 s, « Argos » user agent, nothing else).
pub fn appeler_vraiment(url: &str) -> Result<String, String> {
    use argos_data::http::Http;
    argos_data::rt::bloquer(argos_data::http::NativeHttp::default().get(url))
        .unwrap_or_else(|| Err(argos_core::ArgosError::Feed("tokio indisponible".into())))
        .map_err(|e| e.to_string())
}

fn maintenant_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}

/// One relayed request at `now_ms`.
pub fn repondre(garde: &Mutex<Garde>, appeler: Appeler, suite: &str, now_ms: u64) -> Result<Vec<u8>, (u16, String)> {
    if suite == "/etat" {
        let n = garde.lock().map_or(0, |g| g.envoyees);
        return Ok(relais::etat_json(n));
    }
    let p = relais::permis(suite)?;
    let d = garde.lock().map_err(|_| (500, "relais indisponible".to_string()))?.decider(&p, now_ms);
    match d {
        Decision::Garde(b) => Ok(b),
        Decision::Attendre(s) => Err((429, format!("trop de demandes à la bourse : réessaie dans {s} s (Retry-After {s} s)"))),
        Decision::Demander => {
            let corps = appeler(&p.url).map_err(|e| {
                let code = if e.contains("HTTP 429") { 429 } else { 502 };
                (code, e.chars().take(200).collect::<String>())
            })?;
            if corps.len() > relais::MAX_REPONSE {
                return Err((502, "réponse trop grosse : refusée".into()));
            }
            let corps = corps.into_bytes();
            if let Ok(mut g) = garde.lock() {
                g.garder(&p, now_ms, corps.clone());
            }
            Ok(corps)
        }
    }
}

/// Adds `/api/bourse/…` to the server.
pub fn brancher(s: &mut Serveur, appeler: Appeler) {
    let garde = Arc::new(Mutex::new(Garde::default()));
    s.relais.push(RelaisGet {
        prefixe: relais::PREFIXE,
        repondre: Box::new(move |suite| repondre(&garde, appeler, suite, maintenant_ms())),
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::atomic::{AtomicUsize, Ordering};

    static APPELS: AtomicUsize = AtomicUsize::new(0);

    fn faux(url: &str) -> Result<String, String> {
        APPELS.fetch_add(1, Ordering::SeqCst);
        assert!(
            url.starts_with("https://api-pub.bitfinex.com/") || url.starts_with("https://api.kucoin.com/"),
            "{url}"
        );
        Ok(format!("[\"{url}\"]"))
    }

    fn lancer() -> u16 {
        let l = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = l.local_addr().unwrap().port();
        let base = std::env::temp_dir().join(format!("argos-serveur-bourse-{}", std::process::id()));
        let mut s = Serveur::new(base.join("www"), base.join("charts"));
        brancher(&mut s, faux);
        std::thread::spawn(move || argos_outils::serveur::servir(l, Arc::new(s)));
        port
    }

    fn req(port: u16, methode: &str, chemin: &str, entetes: &[(&str, &str)]) -> (u16, String) {
        let mut c = TcpStream::connect(("127.0.0.1", port)).unwrap();
        let mut h = format!("{methode} {chemin} HTTP/1.1\r\nConnection: close\r\nContent-Length: 0\r\n");
        if !entetes.iter().any(|(k, _)| *k == "Host") {
            h.push_str(&format!("Host: 127.0.0.1:{port}\r\n"));
        }
        for (k, v) in entetes {
            h.push_str(&format!("{k}: {v}\r\n"));
        }
        c.write_all(format!("{h}\r\n").as_bytes()).unwrap();
        let mut tout = vec![];
        let _ = c.read_to_end(&mut tout);
        let fin = tout.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
        let code = String::from_utf8_lossy(&tout[..fin]).split(' ').nth(1).unwrap().parse().unwrap();
        (code, String::from_utf8_lossy(&tout[fin + 4..]).into_owned())
    }

    /// 🔁 The relay: the page finds it, a listed request goes to the fixed host (rebuilt), kept in the cache; another
    /// host, another path, a POST, another site's page → refused, nothing sent.
    #[test]
    fn relais_ferme_et_garde() {
        let port = lancer();
        let (st, b) = req(port, "GET", "/api/bourse/etat", &[]);
        assert_eq!(st, 200);
        assert!(relais::lire_etat(&b));
        let avant = APPELS.load(Ordering::SeqCst);
        let (st, b) = req(port, "GET", "/api/bourse/bitfinex/v2/book/tXRPBTC/P0?len=100", &[]);
        assert_eq!((st, b.as_str()), (200, "[\"https://api-pub.bitfinex.com/v2/book/tXRPBTC/P0?len=100\"]"));
        assert_eq!(req(port, "GET", "/api/bourse/bitfinex/v2/book/tXRPBTC/P0?len=100", &[]).0, 200);
        assert_eq!(APPELS.load(Ordering::SeqCst), avant + 1, "the second one from the cache");
        for (m, c, code) in [
            ("GET", "/api/bourse/evil/x", 404),
            ("GET", "/api/bourse/bitfinex/v2/auth/r/orders", 404),
            ("GET", "/api/bourse/bitfinex//evil.io/v2/tickers", 404),
            ("GET", "/api/bourse/kucoin/api/v1/bullet-public", 404),
            ("GET", "/api/bourse/bitfinex/v2/tickers?symbols=ALL&key=x", 400),
            ("POST", "/api/bourse/bitfinex/v2/tickers?symbols=ALL", 405),
            ("PUT", "/api/bourse/bitfinex/v2/tickers?symbols=ALL", 405),
            ("DELETE", "/api/bourse/bitfinex/v2/tickers?symbols=ALL", 405),
        ] {
            assert_eq!(req(port, m, c, &[]).0, code, "{m} {c}");
        }
        assert_eq!(
            req(port, "GET", "/api/bourse/bitfinex/v2/tickers?symbols=ALL", &[("Origin", "https://evil.com")]).0,
            403
        );
        assert_eq!(req(port, "GET", "/api/bourse/bitfinex/v2/tickers?symbols=ALL", &[("Host", "evil.com")]).0, 403);
        assert_eq!(APPELS.load(Ordering::SeqCst), avant + 1, "nothing sent for the refused ones");
        // the server's other points are unchanged (a file that does not exist: 404 as before, /api/boursex not relayed)
        assert_eq!(req(port, "GET", "/api/boursex/bitfinex/v2/tickers", &[]).0, 404);
    }

    /// 🧊 Over the cap: the kept answer, or 429 with the wait; never more than the cap to the exchange.
    #[test]
    fn plafond_par_minute() {
        static N: AtomicUsize = AtomicUsize::new(0);
        fn compte(_: &str) -> Result<String, String> {
            N.fetch_add(1, Ordering::SeqCst);
            Ok("[]".into())
        }
        let g = Mutex::new(Garde::default());
        let mut refus = 0;
        for i in 0..200u64 {
            let s = format!("/bitfinex/v2/trades/t{}BTC/hist?limit=100", ["XRP", "ETH", "SOL"][(i % 3) as usize]);
            let s = if i >= 60 { format!("/bitfinex/v2/trades/tA{i}BTC/hist?limit=100") } else { s };
            if let Err((c, m)) = repondre(&g, compte, &s, i * 100) {
                assert_eq!(c, 429);
                assert!(m.contains("Retry-After"));
                refus += 1;
            }
        }
        assert_eq!(N.load(Ordering::SeqCst), relais::PLAFOND_MINUTE);
        assert!(refus > 0);
    }
}
