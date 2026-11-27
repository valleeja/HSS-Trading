//! 🧪 Tests du serveur (reprise de l'ancien argos_serve_test.py) : toujours dans un dossier temporaire, jamais le
//! vrai ~/.config/argos.

use super::*;
use std::os::unix::fs::PermissionsExt;

fn tmp(nom: &str) -> PathBuf {
    // dans le dossier de compilation du projet (rust/target/tmp), jamais dans /tmp
    let d =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/tmp")
            .join(format!("argos-outils-{nom}-{}-{}", std::process::id(), hasard_hex(4)));
    std::fs::create_dir_all(&d).unwrap();
    d
}

struct Menage(PathBuf);
impl Drop for Menage {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn rss() {
    let x = "<rss><item><title><![CDATA[Or &amp; dollar]]></title><link>https://x.y/a</link>\
             <pubDate>Sun, 08 Nov 2026 20:00:00 GMT</pubDate></item><item><title>sans lien</title></item></rss>";
    let n = parse_rss(x, "T", 0);
    let attendu = J::obj(vec![
        ("title", J::s("Or & dollar")),
        ("link", J::s("https://x.y/a")),
        ("ts", J::Int(1_794_168_000_000)),
        ("source", J::s("T")),
        ("lang", J::s("en")),
    ]);
    assert_eq!(n, vec![attendu]);
    let atom = r#"<feed><entry><title>A</title><link href="https://a.b/c"/><updated>2026-11-08T20:00:00Z</updated></entry></feed>"#;
    let n = parse_rss(atom, "X", 0);
    assert_eq!(
        dumps(&n[0], false),
        r#"{"title": "A", "link": "https://a.b/c", "ts": 1794168000000, "source": "X", "lang": "en"}"#
    );
    assert_eq!(parse_rss(atom, "Journal du Coin", 0)[0], {
        let J::Obj(mut v) = parse_rss(atom, "X", 0).remove(0) else { panic!() };
        v[3].1 = J::s("Journal du Coin");
        v[4].1 = J::s("fr");
        J::Obj(v)
    });
}

#[test]
fn une_source_en_404() {
    let faux = |url: &str| -> Result<Vec<u8>, String> {
        if url.contains("coindesk") {
            return Err("HTTPError".into());
        }
        Ok(if url.contains("cointelegraph") {
            b"<rss><item><title>Bonne</title><link>https://ok.y/1</link></item></rss>".to_vec()
        } else {
            b"<rss></rss>".to_vec()
        })
    };
    let n = eco_news(&faux);
    assert_eq!(n.len(), 1);
    assert!(dumps(&n[0], false).contains("\"title\": \"Bonne\"") && dumps(&n[0], false).contains("Cointelegraph"));
}

#[test]
fn noms() {
    for ok in ["split.json", "dock.json", "hyperliquid-btc.json", "indicator_inputs.json", "a1.json"] {
        assert!(nom_permis(ok), "{ok}");
    }
    let long = "a".repeat(120) + ".json";
    for non in [
        "account.json",
        "passkey.json",
        "claude-key.json",
        "account-key.json",
        "wallet.json",
        "cmc_credits.json",
        "../x.json",
        "a/b.json",
        "modules/x.wasm.b64",
        "lang/de.json",
        ".json",
        "-x.json",
        "x.txt",
        "x.json.tmp",
        "é.json",
        "x y.json",
        long.as_str(),
        "",
    ] {
        assert!(!nom_permis(non), "{non}");
    }
}

#[test]
fn clients() {
    for ok in ["127.0.0.1", "::1", "100.64.0.1", "100.101.102.103", "100.127.255.254", "::ffff:127.0.0.1"] {
        assert!(client_permis(ok), "{ok}");
    }
    for non in ["192.168.1.20", "10.0.0.5", "100.128.0.1", "100.63.255.255", "8.8.8.8", "fe80::1", "", "abc"] {
        assert!(!client_permis(non), "{non}");
    }
}

#[test]
fn hotes() {
    for (h, o) in [
        ("localhost:8000", None),
        ("127.0.0.1:8000", Some("http://127.0.0.1:8000")),
        ("100.90.1.2:8000", None),
        ("pc.tail1234.ts.net:8000", Some("http://pc.tail1234.ts.net:8000")),
        ("[::1]:8000", None),
    ] {
        assert!(hote_permis(h, o), "{h}");
    }
    for (h, o) in [
        ("evil.com", None),
        ("localhost.evil.com:8000", None),
        ("192.168.1.2:8000", None),
        ("localhost:8000", Some("https://evil.com")),
        ("localhost:8000", Some("http://localhost:9999")),
        ("localhost:8000", Some("null")),
        ("", None),
    ] {
        assert!(!hote_permis(h, o), "{h} {o:?}");
    }
}

#[test]
fn ecrire_lire_et_droits() {
    let base = tmp("reglages");
    let _m = Menage(base.clone());
    let d = base.join("charts");
    let t = reglages_ecrire(&d, "split.json", br#"{"n": 2}"#).unwrap();
    assert_eq!(reglages_lire(&d, "split.json"), Some((r#"{"n": 2}"#.into(), t)));
    assert_eq!(std::fs::metadata(d.join("split.json")).unwrap().permissions().mode() & 0o777, 0o600);
    assert_eq!(std::fs::metadata(&d).unwrap().permissions().mode() & 0o777, 0o700);
    reglages_ecrire(&d, "split.json", br#"{"n": 3}"#).unwrap();
    assert_eq!(reglages_lire(&d, "split.json").unwrap().0, r#"{"n": 3}"#);
    let noms: Vec<_> = std::fs::read_dir(&d).unwrap().flatten().map(|e| e.file_name()).collect();
    assert_eq!(noms, ["split.json"], "aucun fichier temporaire ne reste");
    reglages_ecrire(&d, "split.json", b"").unwrap();
    assert_eq!(reglages_lire(&d, "split.json").unwrap().0, "", "vide = permis (l'app efface ainsi)");
}

#[test]
fn refus() {
    let base = tmp("refus");
    let _m = Menage(base.clone());
    let d = base.join("charts");
    let gros = vec![b' '; MAX_REGLAGE + 1];
    let imbrique = [b"[".repeat(10), b"1".to_vec(), b"]".repeat(10), b"x".to_vec()].concat();
    for (nom, data) in [
        ("account.json", &b"{}"[..]),
        ("../x.json", b"{}"),
        ("x.json", b"pas du json"),
        ("x.json", b"\xff\xfe"),
        ("x.json", &imbrique),
        ("x.json", &gros),
    ] {
        assert!(reglages_ecrire(&d, nom, data).is_err(), "{nom}");
    }
    assert!(!base.join("x.json").exists(), "jamais hors du dossier");
}

#[test]
fn lire_tout_sans_les_fichiers_interdits() {
    let d = tmp("tout");
    let _m = Menage(d.clone());
    for (n, t) in [
        ("dock.json", "{}"),
        ("account.json", r#"{"secret": 1}"#),
        ("claude-key.json", "{}"),
        ("notes.txt", "x"),
    ] {
        std::fs::write(d.join(n), t).unwrap();
    }
    std::fs::create_dir(d.join("modules")).unwrap();
    std::fs::write(d.join("gros.json"), " ".repeat(MAX_REGLAGE + 1)).unwrap();
    let noms: Vec<String> = reglages_lire_tout(&d).into_iter().map(|x| x.0).collect();
    assert_eq!(noms, ["dock.json"]);
    assert!(reglages_lire_tout(&d.join("absent")).is_empty());
}

#[test]
fn changements() {
    let d = tmp("journal");
    let _m = Menage(d.clone());
    reglages_ecrire(&d, "dock.json", b"{}").unwrap();
    let j = Journal::new(d.clone());
    let v0 = j.scanner();
    let (_, f, r) = j.changements(&j.epoque, v0, 0.0);
    assert!(f.is_empty() && r.is_empty(), "rien de neuf");
    assert_eq!(j.changements("vieille", v0, 0.0).1[0].0, "dock.json", "autre époque : tout");
    std::fs::write(d.join("split.json"), r#"{"n": 4}"#).unwrap();
    let (v1, f, _) = j.changements(&j.epoque, v0, 0.0);
    assert_eq!(f, [("split.json".to_string(), r#"{"n": 4}"#.to_string())]);
    assert!(v1 > v0);
    reglages_ecrire(&d, "split.json", br#"{"n": 2, "x": 1}"#).unwrap();
    std::fs::remove_file(d.join("dock.json")).unwrap();
    let (v2, f, r) = j.changements(&j.epoque, v1, 0.0);
    assert_eq!(
        (f, r),
        (
            vec![("split.json".to_string(), r#"{"n": 2, "x": 1}"#.to_string())],
            vec!["dock.json".to_string()]
        )
    );
    std::fs::write(d.join("account.json"), "{}").unwrap();
    assert!(j.changements(&j.epoque, v2, 0.0).1.is_empty(), "un fichier interdit n'apparaît jamais");
}

#[test]
fn attente() {
    let d = tmp("attente");
    let _m = Menage(d.clone());
    reglages_ecrire(&d, "dock.json", b"{}").unwrap();
    let j = Journal::new(d.clone());
    let v0 = j.scanner();
    let t0 = Instant::now();
    assert!(j.changements(&j.epoque, v0, 1.2).1.is_empty());
    assert!(t0.elapsed() >= Duration::from_millis(1100), "attend avant de répondre « rien »");
    let d2 = d.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(300));
        reglages_ecrire(&d2, "alerts.json", b"[]").unwrap();
    });
    let t0 = Instant::now();
    let (_, f, _) = j.changements(&j.epoque, v0, 10.0);
    assert_eq!(f, [("alerts.json".to_string(), "[]".to_string())]);
    assert!(t0.elapsed() < Duration::from_secs(3));
}

/// Une requête HTTP brute ; rend (code, entêtes, corps).
fn req(port: u16, methode: &str, chemin: &str, corps: &[u8], entetes: &[(&str, String)]) -> (u16, String, Vec<u8>) {
    let mut c = TcpStream::connect(("127.0.0.1", port)).unwrap();
    let mut h = format!("{methode} {chemin} HTTP/1.1\r\nConnection: close\r\n");
    if !entetes.iter().any(|(k, _)| *k == "Host") {
        h.push_str(&format!("Host: 127.0.0.1:{port}\r\n"));
    }
    if !entetes.iter().any(|(k, _)| *k == "Content-Length") && (methode == "PUT" || !corps.is_empty()) {
        h.push_str(&format!("Content-Length: {}\r\n", corps.len()));
    }
    for (k, v) in entetes {
        h.push_str(&format!("{k}: {v}\r\n"));
    }
    h.push_str("\r\n");
    c.write_all(h.as_bytes()).unwrap();
    let _ = c.write_all(corps);
    let mut tout = vec![];
    let _ = c.read_to_end(&mut tout);
    let fin = tout.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
    let tete = String::from_utf8_lossy(&tout[..fin]).into_owned();
    let code = tete.split(' ').nth(1).unwrap().parse().unwrap();
    (code, tete, tout[fin + 4..].to_vec())
}

fn json(b: &[u8]) -> serde_json::Value {
    serde_json::from_slice(b).unwrap()
}

fn lancer() -> (u16, PathBuf, Menage) {
    let base = tmp("serveur");
    std::fs::create_dir_all(base.join("www/app")).unwrap();
    std::fs::write(base.join("www/app/index.html"), "<p>Argos</p>").unwrap();
    std::fs::write(base.join("www/app/a.png"), [137u8, 80, 78, 71]).unwrap();
    let l = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let port = l.local_addr().unwrap().port();
    let s = Arc::new(Serveur::new(base.join("www"), base.join("charts")));
    std::thread::spawn(move || servir(l, s));
    (port, base.join("charts"), Menage(base))
}

#[test]
fn aller_retour() {
    let (port, _, _m) = lancer();
    let origine = [("Origin", format!("http://127.0.0.1:{port}"))];
    let (st, _, b) = req(port, "PUT", "/api/reglages/dock.json", br#"{"on": true}"#, &origine);
    assert_eq!(st, 200, "{}", String::from_utf8_lossy(&b));
    assert!(json(&b).get("t").is_some());
    let (st, _, b) = req(port, "GET", "/api/reglages", b"", &[]);
    assert_eq!((st, json(&b)["fichiers"]["dock.json"].as_str()), (200, Some(r#"{"on": true}"#)));
    let (st, _, b) = req(port, "GET", "/api/reglages/dock.json", b"", &[]);
    assert_eq!((st, json(&b)["texte"].as_str()), (200, Some(r#"{"on": true}"#)));
    let tout = json(&req(port, "GET", "/api/reglages", b"", &[]).2);
    let b = json(&req(port, "PUT", "/api/reglages/split.json", br#"{"n": 2}"#, &[]).2);
    assert!(b["v"].as_i64() > tout["v"].as_i64());
    let (e, v) = (tout["e"].as_str().unwrap(), tout["v"].as_i64().unwrap());
    let (st, _, c) = req(port, "GET", &format!("/api/reglages/changements?e={e}&depuis={v}&attente=0"), b"", &[]);
    assert_eq!((st, json(&c)["fichiers"].clone()), (200, serde_json::json!({"split.json": "{\"n\": 2}"})));
    assert_eq!(req(port, "GET", "/api/reglages/changements?depuis=abc", b"", &[]).0, 400);
    assert_eq!(req(port, "GET", "/api/reglages/absent.json", b"", &[]).0, 404);
}

#[test]
fn refus_http() {
    let (port, d, _m) = lancer();
    assert_eq!(req(port, "PUT", "/api/reglages/account.json", b"{}", &[]).0, 403);
    assert_eq!(req(port, "PUT", "/api/reglages/x.json", b"pas json", &[]).0, 400);
    assert_eq!(req(port, "PUT", "/api/reglages/x.json", b"{}", &[("Origin", "https://evil.com".into())]).0, 403);
    assert_eq!(req(port, "GET", "/api/reglages", b"", &[("Host", "evil.com".into())]).0, 403);
    assert_eq!(
        req(port, "PUT", "/api/reglages/x.json", b"{}", &[("Content-Length", (MAX_REGLAGE + 1).to_string())]).0,
        413
    );
    assert_eq!(req(port, "PUT", "/autre", b"{}", &[]).0, 405);
    assert!(!d.join("account.json").exists());
}

#[test]
fn fichiers_statiques() {
    let (port, _, _m) = lancer();
    let (st, h, b) = req(port, "GET", "/app", b"", &[]);
    assert_eq!(st, 301);
    assert!(h.contains("Location: /app/"));
    let gz = [("Accept-Encoding", "gzip".to_string())];
    let (st, h, b2) = req(port, "GET", "/app/", b"", &gz);
    assert_eq!(st, 200);
    assert!(h.contains("Content-Encoding: gzip") && h.contains("Content-Type: text/html"));
    let mut txt = String::new();
    flate2::read::GzDecoder::new(&b2[..]).read_to_string(&mut txt).unwrap();
    assert_eq!(txt, "<p>Argos</p>");
    let etag = h.lines().find_map(|l| l.strip_prefix("ETag: ")).unwrap().to_string();
    assert_eq!(req(port, "GET", "/app/index.html", b"", &[gz[0].clone(), ("If-None-Match", etag)]).0, 304);
    let (st, h, b3) = req(port, "GET", "/app/a.png?v=1", b"", &gz);
    assert_eq!((st, b3.len()), (200, 4));
    assert!(h.contains("Content-type: image/png") && !h.contains("gzip"));
    assert_eq!(req(port, "GET", "/app/../../etc/passwd", b"", &[]).0, 404);
    assert_eq!(req(port, "GET", "/rien.js", b"", &[]).0, 404);
    assert_eq!(req(port, "DELETE", "/app/", b"", &[]).0, 501);
    assert!(b.is_empty());
}

#[test]
fn dates() {
    assert_eq!(date_http(1_794_168_000), "Sun, 08 Nov 2026 20:00:00 GMT");
    assert_eq!(date_rfc2822("Sun, 08 Nov 2026 20:00:00 GMT"), Some(1_794_168_000));
    assert_eq!(date_rfc2822("Sun, 08 Nov 2026 15:00:00 -0500"), Some(1_794_168_000));
    assert_eq!(date_iso("2026-11-08T21:00:00.250+01:00"), Some(1_794_168_000));
    assert_eq!(entites("a &amp; b &#39; &#x2019; &rsquo; &zz;"), "a & b ' ’ ’ &zz;");
}

/// « / » → l'app Rust quand on le demande (3e argument `/app/`, comme l'ancien serveur-local.py) ; sinon index.html.
#[test]
fn accueil_vers_app() {
    let base = tmp("accueil");
    std::fs::create_dir_all(base.join("www/app")).unwrap();
    std::fs::write(base.join("www/index.html"), "<p>vieux site</p>").unwrap();
    std::fs::write(base.join("www/app/index.html"), "<p>Argos</p>").unwrap();
    let l = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let port = l.local_addr().unwrap().port();
    let mut s = Serveur::new(base.join("www"), base.join("charts"));
    s.accueil = Some("/app/".into());
    std::thread::spawn(move || servir(l, Arc::new(s)));
    let _m = Menage(base);
    let (st, h, _) = req(port, "GET", "/", b"", &[]);
    assert_eq!(st, 302);
    assert!(h.to_ascii_lowercase().contains("location: /app/"), "{h}");
    assert_eq!(req(port, "GET", "/index.html", b"", &[]).0, 200, "le vieux site reste sur /index.html");
    assert_eq!(req(port, "GET", "/app/", b"", &[]).0, 200);
    let (port2, _, _m2) = lancer();
    assert_ne!(req(port2, "GET", "/", b"", &[]).0, 302, "sans accueil : « / » sert le dossier");
}

/// 🧩 Un programme qui réutilise le serveur : sa route privée (GET, POST avec corps, erreur rendue en JSON, mêmes
/// accès que la synchro) et ses réglages interdits (jamais servis, écrits ni annoncés comme changés).
#[test]
fn routes_et_interdits_ajoutes() {
    let base = tmp("ajouts");
    let charts = base.join("charts");
    std::fs::create_dir_all(&charts).unwrap();
    std::fs::write(charts.join("prive.json"), r#"{"secret": 1}"#).unwrap();
    let l = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let port = l.local_addr().unwrap().port();
    let mut s = Serveur::new(base.join("www"), charts.clone());
    s.interdits = &["prive.json"];
    s.routes.push(Route {
        chemin: "/api/echo",
        repondre: Box::new(|corps| match corps {
            None => Ok(b"{\"lu\": true}".to_vec()),
            Some(b"") => Err((400, "vide".into())),
            Some(d) => Ok(d.to_vec()),
        }),
    });
    std::thread::spawn(move || servir(l, Arc::new(s)));
    let _m = Menage(base);
    assert_eq!(json(&req(port, "GET", "/api/echo", b"", &[]).2)["lu"], true);
    let origine = [("Origin", format!("http://127.0.0.1:{port}"))];
    let (st, _, b) = req(port, "POST", "/api/echo", br#"{"x": 2}"#, &origine);
    assert_eq!((st, json(&b)["x"].as_u64()), (200, Some(2)));
    let (st, _, b) = req(port, "POST", "/api/echo", b"", &[("Content-Length", "0".into())]);
    assert_eq!((st, json(&b)["error"].as_str()), (400, Some("vide")));
    assert_eq!(req(port, "POST", "/api/echo", b"{}", &[("Origin", "https://evil.com".into())]).0, 403);
    assert_eq!(req(port, "GET", "/api/echo", b"", &[("Host", "evil.com".into())]).0, 403);
    assert_eq!(req(port, "POST", "/api/autre", b"{}", &[]).0, 501);
    // le réglage interdit : ni lu, ni écrit, ni dans la liste complète, ni dans les changements
    assert_eq!(req(port, "GET", "/api/reglages/prive.json", b"", &[]).0, 403);
    assert_eq!(req(port, "PUT", "/api/reglages/prive.json", b"{}", &[]).0, 403);
    let tout = json(&req(port, "GET", "/api/reglages", b"", &[]).2);
    assert!(tout["fichiers"].get("prive.json").is_none(), "{tout}");
    std::fs::write(charts.join("prive.json"), r#"{"secret": 2}"#).unwrap();
    let e = tout["e"].as_str().unwrap();
    let ch = json(&req(port, "GET", &format!("/api/reglages/changements?e={e}&depuis=0&attente=0"), b"", &[]).2);
    assert!(ch["fichiers"].get("prive.json").is_none(), "{ch}");
    assert_eq!(std::fs::read_to_string(charts.join("prive.json")).unwrap(), r#"{"secret": 2}"#);
}

/// 📅 The calendar relay reads Forex Factory at most every 5 min (it answered 429 when read every minute); a failed
/// reading keeps the last good table; the « Réel » is filled at each request.
#[test]
fn calendrier_garde_cinq_minutes() {
    let mut s = Serveur::new(PathBuf::from("/nulle-part"), PathBuf::from("/nulle-part"));
    let lus = std::sync::atomic::AtomicUsize::new(0);
    let remplis = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let r2 = remplis.clone();
    s.calendrier = Some(Box::new(move |b: &[u8]| {
        r2.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        b.to_vec()
    }));
    let lire = || {
        lus.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(b"[1]".to_vec())
    };
    for _ in 0..5 {
        assert_eq!(calendrier_relais(&s, &lire), (200, b"[1]".to_vec()));
    }
    assert_eq!(lus.load(std::sync::atomic::Ordering::SeqCst), 1, "one Forex Factory reading");
    assert_eq!(remplis.load(std::sync::atomic::Ordering::SeqCst), 5, "the « Réel » at each request");
    // 5 min later Forex Factory answers 429: the last good table stays
    if let Ok(mut m) = s.eco.lock() {
        let v = m.get("calendar").unwrap().1.clone();
        m.insert("calendar", (Instant::now() - Duration::from_secs(GARDE_CALENDRIER_S + 1), v));
    }
    let refus = || Err("HTTP 429".to_string());
    assert_eq!(calendrier_relais(&s, &refus), (200, b"[1]".to_vec()));
    // nothing read yet and an error: 502
    let vide = Serveur::new(PathBuf::from("/nulle-part"), PathBuf::from("/nulle-part"));
    assert_eq!(calendrier_relais(&vide, &refus).0, 502);
}
