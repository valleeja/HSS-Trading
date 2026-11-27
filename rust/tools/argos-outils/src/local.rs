//! 🛰️ Argos en local (site v1) : sert la racine du dépôt sur `http://localhost:8787` et relaie `/api/...` vers le
//! site Argos en ligne. Remplace `argos_local.py` (mêmes adresses, mêmes réponses).
//!
//! ```text
//! argos-outils local [--racine rust/]                   http://localhost:8787 (seulement cet ordi)
//! argos-outils local --wifi [--nouveau-code]            📱 aussi le cell (même wifi), avec le code secret ?t=…
//! ```
//! - 💾 `/local/backup` (GET / POST) : sauvegarde locale dans `data/argos-sauvegarde.json` (jamais tes clés) ;
//! - 🦙 `/ollama/...` : relais vers Ollama sur cet ordi (127.0.0.1:11434) ;
//! - 📅 `/api/calendar` : calendrier de la semaine (Forex Factory) lu d'ici ;
//! - jamais servis : `/key`, `/data`, les fichiers cachés ; `sw.js` neutre (toujours la dernière version).
//!
//! Port : `$ARGOS_PORT` (par défaut 8787).

use crate::json::{dumps, J};
use crate::serveur::{date_http, date_iso, parse_qs, pourcent, raison, traduire, type_mime};
use crate::version::maintenant;
use crate::Args;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const SITE: &str = "https://argos-smoky.vercel.app";
const MAX_BACKUP: usize = 25 * 1024 * 1024;
const BLOCK: [&str; 3] = ["/key", "/data", "/."];
const ICONES: [&str; 8] = [
    "/icon-180.png",
    "/icon-512.png",
    "/favicon.png",
    "/favicon.ico",
    "/icon.svg",
    "/logo.png",
    "/apple-touch-icon.png",
    "/apple-touch-icon-precomposed.png",
];
/// service worker neutre : en local on veut toujours la dernière version, sans cache
const SW_OFF: &[u8] = b"self.addEventListener('install',()=>self.skipWaiting());self.addEventListener('activate',e=>e.waitUntil(self.registration.unregister().then(()=>self.clients.matchAll()).then(cs=>cs.forEach(c=>c.navigate(c.url)))));";

pub struct Local {
    pub racine: PathBuf,
    pub port: u16,
    pub wifi: bool,
    pub token: String,
}

impl Local {
    fn data(&self) -> PathBuf {
        self.racine.join("data")
    }
    fn backup(&self) -> PathBuf {
        self.data().join("argos-sauvegarde.json")
    }
}

struct Req {
    methode: String,
    chemin: String,
    entetes: Vec<(String, String)>,
    ip: String,
    corps: Vec<u8>,
}

impl Req {
    fn h(&self, nom: &str) -> Option<&str> {
        self.entetes.iter().find(|(k, _)| k.eq_ignore_ascii_case(nom)).map(|(_, v)| v.as_str())
    }
    fn base(&self) -> &str {
        self.chemin.split('?').next().unwrap_or("")
    }
    fn est_local(&self) -> bool {
        matches!(self.ip.as_str(), "127.0.0.1" | "::1" | "::ffff:127.0.0.1")
    }
    fn longueur(&self) -> usize {
        self.h("Content-Length").and_then(|n| n.trim().parse().ok()).unwrap_or(0)
    }
}

struct Rep {
    code: u16,
    /// texte de la ligne d'état (le message des erreurs, comme `send_error`)
    etat: Option<String>,
    entetes: Vec<(String, String)>,
    corps: Vec<u8>,
}

impl Rep {
    fn new(code: u16, ct: &str, corps: Vec<u8>) -> Rep {
        Rep {
            code,
            etat: None,
            entetes: vec![("Content-Type".into(), ct.into()), ("Content-Length".into(), corps.len().to_string())],
            corps,
        }
    }
    fn json(code: u16, v: &J) -> Rep {
        Rep::new(code, "application/json", dumps(v, true).into_bytes())
    }
    fn erreur(code: u16, message: &str) -> Rep {
        let explication = match code {
            403 => "Request forbidden -- authorization will not help",
            404 => "Nothing matches the given URI",
            405 => "Specified method is invalid for this resource",
            501 => "Server does not support this operation",
            _ => raison(code),
        };
        let corps = format!(
            "<!DOCTYPE HTML>\n<html lang=\"en\">\n    <head>\n        <meta charset=\"utf-8\">\n        \
             <style type=\"text/css\">\n            :root {{\n                color-scheme: light dark;\n            }}\n        \
             </style>\n        <title>Error response</title>\n    </head>\n    <body>\n        <h1>Error response</h1>\n        \
             <p>Error code: {code}</p>\n        <p>Message: {message}.</p>\n        \
             <p>Error code explanation: {code} - {explication}.</p>\n    </body>\n</html>\n"
        );
        let mut r = Rep::new(code, "text/html;charset=utf-8", corps.into_bytes());
        r.entetes.insert(0, ("Connection".into(), "close".into()));
        r.etat = Some(message.to_string());
        r
    }
}

fn erreur_json(code: u16, msg: &str) -> Rep {
    Rep::json(code, &J::obj(vec![("error", J::s(msg))]))
}

/// Le code secret du mode wifi, gardé dans `data/.wifi-code` (jamais servi) ; `nouveau` en crée un autre.
pub fn code_wifi(data: &Path, nouveau: bool) -> String {
    let f = data.join(".wifi-code");
    if !nouveau {
        if let Ok(t) = std::fs::read_to_string(&f) {
            let t = t.trim().to_string();
            if t.len() >= 10 {
                return t;
            }
        }
    }
    let t = base64_url(&hasard(9));
    let _ = std::fs::create_dir_all(data);
    if std::fs::write(&f, &t).is_ok() {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&f, std::fs::Permissions::from_mode(0o600));
    }
    t
}

fn hasard(n: usize) -> Vec<u8> {
    let mut b = vec![0u8; n];
    if std::fs::File::open("/dev/urandom").and_then(|mut f| f.read_exact(&mut b)).is_err() {
        let x = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
        b.iter_mut().enumerate().for_each(|(i, v)| *v = (x >> (8 * (i % 16))) as u8);
    }
    b
}

/// base64 « url » sans `=` (comme `secrets.token_urlsafe`).
pub fn base64_url(b: &[u8]) -> String {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut o = String::new();
    for c in b.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        for i in 0..=c.len() {
            o.push(A[(n >> (18 - 6 * i) & 63) as usize] as char);
        }
    }
    o
}

/// Égalité en temps constant (comme `secrets.compare_digest`).
fn egal(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0u8, |x, (p, q)| x | (p ^ q)) == 0
}

/// Chemin jamais servi (clés, sauvegarde, fichiers cachés).
pub fn bloque(chemin: &str) -> bool {
    let c = chemin.split('?').next().unwrap_or("");
    let c = c.split('#').next().unwrap_or("");
    let p = normaliser(&pourcent(c, false)).to_lowercase();
    BLOCK.iter().any(|b| p.starts_with(b)) || p.contains("/.")
}

/// `posixpath.normpath`
fn normaliser(p: &str) -> String {
    let abs = p.starts_with('/');
    let mut v: Vec<&str> = vec![];
    for m in p.split('/') {
        match m {
            "" | "." => {}
            ".." => {
                if v.last().is_some_and(|x| *x != "..") {
                    v.pop();
                } else if !abs {
                    v.push("..");
                }
            }
            x => v.push(x),
        }
    }
    let j = v.join("/");
    match (abs, j.is_empty()) {
        (true, _) => format!("/{j}"),
        (false, true) => ".".into(),
        _ => j,
    }
}

impl Local {
    /// 📱 mode wifi : un appareil du réseau doit avoir ouvert l'adresse avec le code secret (gardé ensuite dans un
    /// cookie). `Err` = refusé ; `Ok(true)` = poser le cookie.
    fn porte(&self, q: &Req) -> Result<bool, Rep> {
        if q.est_local() {
            return Ok(false);
        }
        if self.wifi && q.methode == "GET" && ICONES.contains(&q.base()) {
            return Ok(false);
        }
        if !self.wifi {
            return Err(Rep::erreur(403, "Forbidden"));
        }
        let qs = q.chemin.split_once('?').map_or("", |x| x.1);
        let t = parse_qs(qs).get("t").and_then(|v| v.first().cloned()).unwrap_or_default();
        if egal(&t, &self.token) {
            return Ok(true);
        }
        if q.h("Cookie").unwrap_or("").contains(&format!("argos_t={}", self.token)) {
            return Ok(false);
        }
        Err(Rep::new(
            403,
            "text/plain; charset=utf-8",
            "🔒 Argos : ouvre l'adresse complète affichée dans le terminal de ton ordi (avec ?t=…)"
                .as_bytes()
                .to_vec(),
        ))
    }

    fn origines(&self) -> [String; 2] {
        [format!("http://localhost:{}", self.port), format!("http://127.0.0.1:{}", self.port)]
    }

    /// seulement Argos lui-même : en-tête maison + bonne origine
    fn local_ok(&self, q: &Req) -> bool {
        let o = q.h("Origin").unwrap_or("");
        q.h("X-Argos") == Some("1") && (o.is_empty() || self.origines().iter().any(|x| x == o))
    }

    fn repondre(&self, q: &Req) -> (Rep, bool) {
        let cookie = match self.porte(q) {
            Ok(c) => c,
            Err(r) => return (r, false),
        };
        let r = match q.methode.as_str() {
            "GET" | "HEAD" => self.get(q),
            "POST" => self.post(q),
            _ => Rep::erreur(501, &format!("Unsupported method ({:?})", q.methode)),
        };
        (r, cookie)
    }

    fn get(&self, q: &Req) -> Rep {
        if bloque(&q.chemin) {
            return Rep::erreur(404, "Not Found");
        }
        let b = q.base();
        if !q.est_local() && (q.chemin.starts_with("/ollama/") || b == "/local/backup") {
            return erreur_json(403, "seulement sur l'ordi");
        }
        if q.methode == "GET" {
            if b == "/api/calendar" {
                return calendrier();
            }
            if q.chemin.starts_with("/ollama/") {
                return self.ollama(q);
            }
            if b == "/local/backup" {
                return self.sauvegarde(q);
            }
            if q.chemin.starts_with("/api/") {
                return relais(q);
            }
        }
        if self.wifi && !q.est_local() && b == "/manifest.webmanifest" {
            let texte = std::fs::read_to_string(self.racine.join("manifest.webmanifest")).unwrap_or_default();
            let Ok(v) = serde_json::from_str::<serde_json::Value>(&texte) else {
                return Rep::erreur(404, "Not Found");
            };
            let mut m = manifest_ordonne(&v, &texte);
            if let J::Obj(v) = &mut m {
                let u = J::Str(format!("/?t={}", self.token));
                match v.iter_mut().find(|(k, _)| k == "start_url") {
                    Some(x) => x.1 = u,
                    None => v.push(("start_url".into(), u)),
                }
            }
            return Rep::new(200, "application/manifest+json", dumps(&m, true).into_bytes());
        }
        if b == "/sw.js" {
            let mut r = Rep::new(200, "application/javascript", SW_OFF.to_vec());
            r.entetes.retain(|(k, _)| k != "Content-Length");
            return r;
        }
        let chemin = if b == "/favicon.ico" { "/favicon.png" } else { q.chemin.as_str() };
        fichier(&self.racine, chemin)
    }

    fn post(&self, q: &Req) -> Rep {
        let b = q.base();
        if !q.est_local() && (q.chemin.starts_with("/ollama/") || b == "/local/backup") {
            return erreur_json(403, "seulement sur l'ordi");
        }
        if q.chemin.starts_with("/ollama/") {
            return self.ollama(q);
        }
        if b == "/local/backup" {
            return self.sauvegarde(q);
        }
        if q.chemin.starts_with("/api/") {
            return relais(q);
        }
        Rep::erreur(405, "Method Not Allowed")
    }

    /// 💾 sauvegarde locale : modèles, mises en page, dessins, trades démo… dans un fichier sur TON ordi
    fn sauvegarde(&self, q: &Req) -> Rep {
        if !self.local_ok(q) {
            return erreur_json(403, "refusé");
        }
        let f = self.backup();
        if q.methode == "GET" {
            return match std::fs::read(&f) {
                Ok(d) => Rep::new(200, "application/json", d),
                Err(_) => Rep::json(200, &J::Obj(vec![])),
            };
        }
        let n = q.longueur();
        if n == 0 || n > MAX_BACKUP {
            return erreur_json(413, "taille invalide");
        }
        let ok = std::str::from_utf8(&q.corps)
            .ok()
            .and_then(|t| serde_json::from_str::<serde_json::Value>(t).ok())
            .is_some_and(|v| v.get("keys").is_some_and(|k| k.is_object()));
        if !ok {
            return erreur_json(400, "JSON invalide");
        }
        let _ = std::fs::create_dir_all(self.data());
        let tmp = f.with_extension("json.tmp");
        if let Err(e) = std::fs::write(&tmp, &q.corps) {
            return erreur_json(500, &format!("écriture impossible : {e}"));
        }
        if f.exists() {
            let _ = std::fs::rename(&f, f.with_extension("json.avant")); // garde la version d'avant, au cas où
        }
        let _ = std::fs::rename(&tmp, &f);
        Rep::json(
            200,
            &J::obj(vec![
                ("ok", J::Bool(true)),
                ("ts", J::Int(ms())),
                ("path", J::Str(f.to_string_lossy().into_owned())),
            ]),
        )
    }

    /// 🦙 relais vers Ollama sur CE ordi (127.0.0.1:11434)
    fn ollama(&self, q: &Req) -> Rep {
        let o = q.h("Origin");
        if !self.local_ok(q) && !o.is_none_or(|o| self.origines().iter().any(|x| x == o)) {
            return erreur_json(403, "refusé");
        }
        let url = format!("http://127.0.0.1:11434{}", &q.chemin["/ollama".len()..]);
        let ct = q.h("Content-Type").unwrap_or("application/json").to_string();
        match envoyer(&q.methode, &url, vec![("Content-Type".into(), ct)], &q.corps, 600) {
            Ok(r) => r,
            Err(e) => Rep::json(
                502,
                &J::obj(vec![(
                    "error",
                    J::obj(vec![(
                        "message",
                        J::Str(format!("Ollama ne répond pas sur ce ordi (127.0.0.1:11434) : {}", court(&e, 120))),
                    )]),
                )]),
            ),
        }
    }
}

fn court(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

fn ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

/// Le manifeste dans l'ordre du fichier (serde_json trie les clés : chaque objet reprend l'ordre où ses clés
/// apparaissent dans le texte, comme `json.load` de Python).
fn manifest_ordonne(v: &serde_json::Value, texte: &str) -> J {
    match v {
        serde_json::Value::Object(o) => {
            let mut v: Vec<(String, J)> = o.iter().map(|(k, x)| (k.clone(), manifest_ordonne(x, texte))).collect();
            v.sort_by_key(|(k, _)| texte.find(&format!("\"{k}\"")).unwrap_or(usize::MAX));
            J::Obj(v)
        }
        serde_json::Value::Array(a) => J::Arr(a.iter().map(|x| manifest_ordonne(x, texte)).collect()),
        x => J::from(x),
    }
}

/// Une requête vers une autre adresse ; la réponse (même 4xx / 5xx) est rendue telle quelle.
fn envoyer(methode: &str, url: &str, entetes: Vec<(String, String)>, corps: &[u8], secs: u64) -> Result<Rep, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(secs)))
        .http_status_as_error(false)
        .build()
        .into();
    let mut b = ureq::http::Request::builder().method(methode).uri(url);
    for (k, v) in &entetes {
        b = b.header(k.as_str(), v.as_str());
    }
    let req = b.body(corps.to_vec()).map_err(|e| e.to_string())?;
    let mut r = agent.run(req).map_err(|e| e.to_string())?;
    let code = r.status().as_u16();
    let ct = r
        .headers()
        .get("Content-Type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/json")
        .to_string();
    let d = r.body_mut().with_config().limit(64 << 20).read_to_vec().map_err(|e| e.to_string())?;
    Ok(Rep::new(code, &ct, d))
}

/// Relais `/api/...` vers le site Argos en ligne.
fn relais(q: &Req) -> Rep {
    let mut h: Vec<(String, String)> = q
        .entetes
        .iter()
        .filter(|(k, _)| {
            matches!(
                k.to_ascii_lowercase().as_str(),
                "content-type" | "x-oanda-token" | "x-ai-key" | "accept" | "authorization"
            )
        })
        .cloned()
        .collect();
    h.push(("Origin".into(), SITE.into()));
    h.push(("User-Agent".into(), "Argos-local/1.0".into()));
    let corps = if q.corps.is_empty() { vec![] } else { q.corps.clone() };
    envoyer(&q.methode, &format!("{SITE}{}", q.chemin), h, &corps, 60).unwrap_or_else(|e| {
        Rep::new(
            502,
            "application/json",
            format!("{{\"error\":\"site Argos injoignable : {}\"}}", court(&e, 100)).into_bytes(),
        )
    })
}

/// 📅 calendrier de la semaine (Forex Factory), lu directement d'ici
fn calendrier() -> Rep {
    let lu = envoyer(
        "GET",
        "https://nfs.faireconomy.media/ff_calendar_thisweek.json",
        vec![("User-Agent".into(), "Argos-local/1.0".into())],
        &[],
        20,
    )
    .and_then(|r| if r.code == 200 { Ok(r.corps) } else { Err(format!("HTTP Error {}", r.code)) })
    .and_then(|d| serde_json::from_slice::<serde_json::Value>(&d).map_err(|e| e.to_string()));
    match lu {
        Ok(v) => Rep::json(200, &J::obj(vec![("items", J::Arr(calendrier_items(&v)))])),
        Err(e) => erreur_json(502, &format!("calendrier indisponible : {}", court(&e, 100))),
    }
}

pub fn calendrier_items(v: &serde_json::Value) -> Vec<J> {
    let s = |e: &serde_json::Value, k: &str| match e.get(k) {
        Some(serde_json::Value::String(x)) => x.clone(),
        Some(serde_json::Value::Null) | None => String::new(),
        Some(x) => x.to_string(),
    };
    v.as_array()
        .into_iter()
        .flatten()
        .filter_map(|e| {
            let ts = date_iso(&s(e, "date")).map_or(0, |x| x * 1000);
            (ts != 0).then(|| {
                J::obj(vec![
                    ("title", J::Str(s(e, "title"))),
                    ("country", J::Str(s(e, "country"))),
                    ("impact", J::Str(s(e, "impact"))),
                    ("ts", J::Int(ts)),
                    ("allDay", J::Bool(false)),
                    ("forecast", J::Str(s(e, "forecast"))),
                    ("previous", J::Str(s(e, "previous"))),
                    ("actual", J::Str(s(e, "actual"))),
                ])
            })
        })
        .collect()
}

/// Un fichier du dossier (comme `SimpleHTTPRequestHandler` : `index.html` des dossiers, `/` ajouté par 301).
fn fichier(racine: &Path, chemin: &str) -> Rep {
    let mut p = traduire(racine, chemin);
    if p.is_dir() {
        let base = chemin.split(['?', '#']).next().unwrap_or("");
        if !base.ends_with('/') {
            let reste = &chemin[base.len()..];
            let mut r = Rep::new(301, "", vec![]);
            r.entetes = vec![("Location".into(), format!("{base}/{reste}")), ("Content-Length".into(), "0".into())];
            return r;
        }
        match ["index.html", "index.htm"].iter().map(|i| p.join(i)).find(|x| x.is_file()) {
            Some(x) => p = x,
            None => return Rep::erreur(404, "File not found"),
        }
    }
    match std::fs::read(&p) {
        Ok(d) => {
            let mut r = Rep::new(200, type_mime(&p), d);
            r.entetes[0].0 = "Content-type".into();
            if let Some(t) = std::fs::metadata(&p).ok().and_then(|m| m.modified().ok()) {
                let s = t.duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
                r.entetes.push(("Last-Modified".into(), date_http(s)));
            }
            r
        }
        Err(_) => Rep::erreur(404, "File not found"),
    }
}

fn lire(r: &mut BufReader<TcpStream>, ip: String) -> Option<Req> {
    let mut l = String::new();
    r.read_line(&mut l).ok()?;
    let mut p = l.split_whitespace();
    let (methode, chemin) = (p.next()?.to_string(), p.next()?.to_string());
    let mut entetes = vec![];
    loop {
        let mut h = String::new();
        if r.read_line(&mut h).ok()? == 0 || h.trim().is_empty() {
            break;
        }
        if let Some((k, v)) = h.split_once(':') {
            entetes.push((k.trim().to_string(), v.trim().to_string()));
        }
        if entetes.len() > 100 {
            return None;
        }
    }
    let mut q = Req {
        methode,
        chemin,
        entetes,
        ip,
        corps: vec![],
    };
    let n = q.longueur();
    let estime_backup = q.base() == "/local/backup" && n > MAX_BACKUP;
    if n > 0 && !estime_backup && n <= 64 << 20 {
        let mut c = vec![0u8; n];
        r.read_exact(&mut c).ok()?;
        q.corps = c;
    }
    Some(q)
}

fn connexion(c: TcpStream, s: &Local) -> std::io::Result<()> {
    let ip = c.peer_addr().map(|a| a.ip().to_string()).unwrap_or_default();
    let mut w = c.try_clone()?;
    let mut r = BufReader::new(c);
    let Some(q) = lire(&mut r, ip) else { return Ok(()) };
    let (rep, cookie) = s.repondre(&q);
    let mut t = format!(
        "HTTP/1.0 {} {}\r\nServer: argos-outils\r\nDate: {}\r\n",
        rep.code,
        rep.etat.as_deref().unwrap_or(raison(rep.code)),
        date_http(maintenant())
    );
    for (k, v) in &rep.entetes {
        t.push_str(&format!("{k}: {v}\r\n"));
    }
    t.push_str("Cache-Control: no-store\r\n");
    if cookie {
        t.push_str(&format!(
            "Set-Cookie: argos_t={}; Path=/; HttpOnly; SameSite=Lax; Max-Age=31536000\r\n",
            s.token
        ));
    }
    t.push_str("\r\n");
    w.write_all(t.as_bytes())?;
    if q.methode != "HEAD" {
        w.write_all(&rep.corps)?;
    }
    w.flush()
}

pub fn servir(l: TcpListener, s: Arc<Local>) {
    for c in l.incoming().flatten() {
        let s = s.clone();
        std::thread::spawn(move || {
            let _ = c.set_read_timeout(Some(Duration::from_secs(120)));
            let _ = connexion(c, &s);
        });
    }
}

fn ip_wifi() -> String {
    std::net::UdpSocket::bind("0.0.0.0:0")
        .and_then(|u| u.connect("10.255.255.255:1").map(|_| u))
        .and_then(|u| u.local_addr())
        .map(|a| a.ip().to_string())
        .unwrap_or_else(|_| "ADRESSE-DE-TON-ORDI".into())
}

fn qui(prog: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|p| std::env::split_paths(&p).any(|d| d.join(prog).is_file()))
}

pub fn main(a: &Args) -> Result<(), String> {
    let racine = std::fs::canonicalize(a.racine.join("..")).map_err(|e| format!("dossier : {e}"))?;
    let wifi = a.pos.iter().any(|x| x == "--wifi");
    let nouveau = a.pos.iter().any(|x| x == "--nouveau-code");
    let port: u16 = std::env::var("ARGOS_PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(8787);
    let token = code_wifi(&racine.join("data"), nouveau);
    let s = Local { racine, port, wifi, token };
    let l = TcpListener::bind((if wifi { "0.0.0.0" } else { "127.0.0.1" }, port)).map_err(|e| format!("port {port} : {e}"))?;
    let url = format!("http://localhost:{port}/");
    println!("🛰️  Argos local : {url}   (Ctrl + C pour arrêter)");
    if wifi {
        let murl = format!("http://{}:{port}/?t={}", ip_wifi(), s.token);
        println!("📱 Sur ton cell (même wifi), ouvre :  {murl}");
        if qui("qrencode") {
            let _ = std::process::Command::new("qrencode").args(["-t", "ANSIUTF8", &murl]).status();
        } else {
            println!("   (pour un code QR à scanner : sudo apt install qrencode, puis relance)");
        }
        println!("🔒 Même code à chaque lancement (signet / icône du cell OK). Nouveau code : --wifi --nouveau-code. Ctrl + C = plus personne n'y a accès.");
    }
    // ouvre le navigateur sans afficher ses messages internes dans le terminal
    if let Some(o) = ["xdg-open", "open"].into_iter().find(|p| qui(p)) {
        let _ = std::process::Command::new(o)
            .arg(&url)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();
    }
    println!("👉 Laisse ce terminal ouvert pendant que tu utilises Argos.");
    println!("💾 Sauvegarde locale : {}", s.backup().display());
    servir(l, Arc::new(s));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_comme_token_urlsafe() {
        assert_eq!(base64_url(&[0xfb, 0xff, 0xfe, 0, 1, 2, 3, 4, 5]), "-__-AAECAwQF");
        assert_eq!(base64_url(&hasard(9)).len(), 12);
    }

    #[test]
    fn chemins_bloques() {
        for p in [
            "/key/x",
            "/data/argos-sauvegarde.json",
            "/.git/config",
            "/a/.env",
            "/DATA/x",
            "/x/../data/y",
            "/%64ata/x",
        ] {
            assert!(bloque(p), "{p}");
        }
        for p in ["/", "/index.html", "/app.js?v=1", "/keynote.png?x=/."] {
            assert_eq!(bloque(p), p.contains("/key"), "{p}");
        }
    }

    #[test]
    fn normpath() {
        assert_eq!(normaliser("/a/./b/../c//"), "/a/c");
        assert_eq!(normaliser("/../x"), "/x");
    }

    #[test]
    fn items_du_calendrier() {
        let v: serde_json::Value = serde_json::from_str(
            r#"[{"title":"CPI","country":"USD","date":"2026-11-12T08:30:00-05:00","impact":"High","forecast":null,"previous":"0.3%"},{"title":"x","date":""}]"#,
        )
        .unwrap();
        let it = calendrier_items(&v);
        assert_eq!(it.len(), 1);
        assert_eq!(
            dumps(&it[0], true),
            r#"{"title": "CPI", "country": "USD", "impact": "High", "ts": 1794490200000, "allDay": false, "forecast": "", "previous": "0.3%", "actual": ""}"#
        );
    }
}
