//! 🌐 Petit serveur local de l'app navigateur d'Argos (`/app`, `/app2`, `/maquette`, `/captures`, `/emoji`…) :
//! - compresse (gzip) le moteur WASM et les scripts, garde la compression en mémoire (une fois par version du fichier) ;
//! - le navigateur redemande seulement « ça a changé ? » (ETag → 304) ;
//! - HTTP/1.1 (connexion gardée ouverte), une tâche par connexion ;
//! - 📈 `/api/quote?sym=DXY` : relais du vrai DXY (Yahoo), seul symbole permis, lecture seule, sans clé ;
//! - 📰 `/api/news` et 📅 `/api/calendar` : relais des flux (le navigateur ne peut pas les lire lui-même, CORS),
//!   lecture seule, 60 s en mémoire, une source en panne n'empêche pas les autres ;
//! - 🔄 `/api/reglages` : la session partagée avec l'app de bureau (mêmes fichiers que `~/.config/argos/charts`) ;
//!   GET = tout, PUT `/api/reglages/<nom>` = un réglage, GET `/api/reglages/changements?e=…&depuis=…` = ce qui a
//!   changé (attend jusqu'à 25 s). Seulement depuis cet ordi (127.0.0.1) ou Tailscale (100.64.0.0/10), jamais les
//!   fichiers de compte ni de clés, 1 Mio au plus par fichier ;
//! - 🧩 un programme qui réutilise ce serveur (bibliothèque) peut ajouter des routes JSON privées (`Route`, mêmes
//!   accès que `/api/reglages`) et des réglages jamais synchronisés (`Serveur::interdits`).
//!
//! Usage : `argos-outils serveur [port] [dossier] [/accueil/]` (par défaut 8000 et le dossier courant ; avec
//! `/app/` en 3e, « / » renvoie vers l'app Rust, le vieux site reste sur `/index.html`).

use crate::json::{dumps, J};
use crate::version::{maintenant, utc};
use crate::Args;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{IpAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const COMPRESS: [&str; 10] = [".wasm", ".js", ".mjs", ".html", ".css", ".json", ".svg", ".webmanifest", ".txt", ".md"];
pub const MAX_REGLAGE: usize = 1 << 20;
/// jamais par le réseau : compte, passkey, clés, portefeuille (même chiffrés), crédits payants
const REGLAGES_INTERDITS: [&str; 6] = [
    "account.json",
    "passkey.json",
    "claude-key.json",
    "account-key.json",
    "wallet.json",
    "cmc_credits.json",
];

/// Une compression gardée : (date ns, taille, octets gzip, ETag).
type Gz = (u128, u64, Arc<Vec<u8>>, String);
/// Une réponse de relais gardée (60 s, 5 min pour le calendrier) : (heure, (code, corps)).
type Garde = (Instant, Arc<(u16, Vec<u8>)>);

pub struct Serveur {
    pub racine: PathBuf,
    pub reglages: PathBuf,
    /// « / » renvoie (302) vers cette adresse (par exemple « /app/ ») ; sans elle, « / » sert index.html.
    pub accueil: Option<String>,
    gz: Mutex<HashMap<PathBuf, Gz>>,
    eco: Mutex<HashMap<&'static str, Garde>>,
    quotes: Mutex<HashMap<(String, String), (Instant, u16, String)>>,
    journaux: Mutex<HashMap<PathBuf, Arc<Journal>>>,
    /// 🧩 routes ajoutées par le programme qui réutilise ce serveur
    pub routes: Vec<Route>,
    /// 🔁 GET-only relays under a prefix, added by the program that reuses this server
    pub relais: Vec<RelaisGet>,
    /// 📅 ajouté par le programme qui réutilise ce serveur : le calendrier de Forex Factory lu → le même, son « Réel »
    /// rempli (argos-serveur : BLS et Fed)
    pub calendrier: Option<Calendrier>,
    /// 🧩 réglages jamais servis ni écrits par `/api/reglages`, en plus des fichiers de compte et de clés
    pub interdits: &'static [&'static str],
}

/// 🧩 Une route JSON privée : mêmes accès que `/api/reglages` (cet ordi ou Tailscale, adresse de la page vérifiée),
/// corps d'un POST de `MAX_REGLAGE` au plus. GET : `repondre(None)` ; POST : `repondre(Some(corps))`. `Ok` = le JSON
/// rendu (200) ; `Err((code, message))` = `{"error": message}`.
pub struct Route {
    pub chemin: &'static str,
    pub repondre: Repondre,
}

/// 🔁 A private GET-only relay under a path prefix (`/api/bourse/…`): same access as `/api/reglages` (this computer
/// or Tailscale, page address checked); every other method is refused (405). `repondre` receives what follows the
/// prefix, query string included (« /bitfinex/v2/tickers?symbols=tXRPBTC »), and says what it accepts itself (closed
/// lists); `Ok` = the JSON sent (200), `Err((code, message))` = `{"error": message}`.
pub struct RelaisGet {
    pub prefixe: &'static str,
    pub repondre: RepondreGet,
}

/// The answer of a `RelaisGet`: what follows its prefix (query included) → JSON, or (code, error message).
pub type RepondreGet = Box<dyn Fn(&str) -> Result<Vec<u8>, (u16, String)> + Send + Sync>;

/// 📅 Le calendrier lu (JSON de Forex Factory) → le même, complété.
pub type Calendrier = Box<dyn Fn(&[u8]) -> Vec<u8> + Send + Sync>;

/// La réponse d'une `Route` : corps du POST (ou rien) → JSON, ou (code, message d'erreur).
pub type Repondre = Box<dyn Fn(Option<&[u8]>) -> Result<Vec<u8>, (u16, String)> + Send + Sync>;

impl Serveur {
    pub fn new(racine: PathBuf, reglages: PathBuf) -> Serveur {
        Serveur {
            racine,
            reglages,
            accueil: None,
            gz: Mutex::default(),
            eco: Mutex::default(),
            quotes: Mutex::default(),
            journaux: Mutex::default(),
            routes: vec![],
            relais: vec![],
            calendrier: None,
            interdits: &[],
        }
    }
}

/// Le dossier des réglages de l'app de bureau : `$ARGOS_REGLAGES` (tests), sinon `$XDG_CONFIG_HOME/argos/charts`,
/// sinon `~/.config/argos/charts`.
pub fn reglages_dossier() -> PathBuf {
    if let Some(d) = std::env::var_os("ARGOS_REGLAGES").filter(|d| !d.is_empty()) {
        return d.into();
    }
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|d| !d.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".config"));
    base.join("argos").join("charts")
}

/// Lance le serveur ; `preparer` y ajoute ce qui est propre au programme appelant (routes, réglages interdits).
pub fn main(a: &Args, preparer: impl FnOnce(&mut Serveur)) -> Result<(), String> {
    let port: u16 = a.pos.first().map_or(Ok(8000), |p| p.parse()).map_err(|_| "port invalide")?;
    let racine = PathBuf::from(a.pos.get(1).map(String::as_str).unwrap_or("."));
    let l = TcpListener::bind(("0.0.0.0", port)).map_err(|e| format!("port {port} : {e}"))?;
    println!(
        "🌐 Argos servi sur le port {port} (dossier {}), compressé et mis en cache. Ctrl + C pour arrêter.",
        racine.display()
    );
    let mut s = Serveur::new(racine, reglages_dossier());
    s.accueil = a.pos.get(2).filter(|x| x.starts_with('/')).cloned();
    preparer(&mut s);
    servir(l, Arc::new(s));
    Ok(())
}

/// Accepte les connexions pour toujours (une tâche par connexion).
pub fn servir(l: TcpListener, s: Arc<Serveur>) {
    for c in l.incoming().flatten() {
        let s = s.clone();
        std::thread::spawn(move || {
            let _ = c.set_read_timeout(Some(Duration::from_secs(300)));
            let _ = connexion(c, &s);
        });
    }
}

// ———————————————————————————— HTTP ————————————————————————————

struct Requete {
    methode: String,
    chemin: String,
    version: String,
    entetes: Vec<(String, String)>,
    ip: String,
}

impl Requete {
    fn entete(&self, nom: &str) -> Option<&str> {
        self.entetes.iter().find(|(k, _)| k.eq_ignore_ascii_case(nom)).map(|(_, v)| v.as_str())
    }
    fn sans_requete(&self) -> &str {
        self.chemin.split('?').next().unwrap_or("")
    }
    fn requete(&self) -> &str {
        let sans_ancre = self.chemin.split('#').next().unwrap_or("");
        sans_ancre.split_once('?').map_or("", |x| x.1)
    }
}

struct Reponse {
    code: u16,
    entetes: Vec<(String, String)>,
    corps: Vec<u8>,
    /// fermer la connexion après (corps de la requête non lu)
    fermer: bool,
}

impl Reponse {
    fn new(code: u16) -> Reponse {
        Reponse {
            code,
            entetes: vec![],
            corps: vec![],
            fermer: false,
        }
    }
    fn h(mut self, k: &str, v: impl Into<String>) -> Reponse {
        self.entetes.push((k.into(), v.into()));
        self
    }
    fn json(code: u16, v: &J, ascii: bool, charset: bool) -> Reponse {
        let corps = dumps(v, ascii).into_bytes();
        let ct = if charset { "application/json; charset=utf-8" } else { "application/json" };
        let mut r = Reponse::new(code).h("Content-Type", ct).h("Content-Length", corps.len().to_string());
        r.corps = corps;
        r
    }
    fn erreur(code: u16, message: &str) -> Reponse {
        // la même page que http.server de Python
        let explication = match code {
            404 => "Nothing matches the given URI",
            501 => "Server does not support this operation",
            _ => raison(code),
        };
        let corps = format!(
            "<!DOCTYPE HTML>\n<html lang=\"en\">\n    <head>\n        <meta charset=\"utf-8\">\n        \
             <style type=\"text/css\">\n            :root {{\n                color-scheme: light dark;\n            }}\n        \
             </style>\n        <title>Error response</title>\n    </head>\n    <body>\n        <h1>Error response</h1>\n        \
             <p>Error code: {code}</p>\n        <p>Message: {message}.</p>\n        \
             <p>Error code explanation: {code} - {explication}.</p>\n    </body>\n</html>\n"
        )
        .into_bytes();
        let mut r = Reponse::new(code)
            .h("Content-Type", "text/html;charset=utf-8")
            .h("Content-Length", corps.len().to_string());
        r.corps = corps;
        r
    }
}

pub(crate) fn raison(code: u16) -> &'static str {
    match code {
        200 => "OK",
        301 => "Moved Permanently",
        302 => "Found",
        304 => "Not Modified",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        411 => "Length Required",
        413 => "Request Entity Too Large",
        500 => "Internal Server Error",
        501 => "Not Implemented",
        502 => "Bad Gateway",
        _ => "",
    }
}

const JOURS: [&str; 7] = ["Thu", "Fri", "Sat", "Sun", "Mon", "Tue", "Wed"];
const MOIS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

/// Date HTTP : « Sat, 14 Nov 2026 23:06:00 GMT ».
pub fn date_http(secs: i64) -> String {
    let (y, mo, d, h, mi, s) = utc(secs);
    format!(
        "{}, {d:02} {} {y} {h:02}:{mi:02}:{s:02} GMT",
        JOURS[secs.div_euclid(86_400).rem_euclid(7) as usize],
        MOIS[mo as usize - 1]
    )
}

/// Jours depuis 1970 d'une date civile.
fn jours(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Date RFC 2822 (`parsedate_to_datetime`) → secondes Unix.
pub fn date_rfc2822(t: &str) -> Option<i64> {
    let t = t.trim();
    let t = t.split_once(',').map_or(t, |x| x.1).trim();
    let p: Vec<&str> = t.split_whitespace().collect();
    if p.len() < 4 {
        return None;
    }
    let d: i64 = p[0].parse().ok()?;
    let m = MOIS.iter().position(|x| p[1].len() >= 3 && x.eq_ignore_ascii_case(&p[1][..3]))? as i64 + 1;
    let mut y: i64 = p[2].parse().ok()?;
    if y < 100 {
        y += if y > 68 { 1900 } else { 2000 };
    }
    let hms: Vec<i64> = p[3].split(':').map(|x| x.parse().ok()).collect::<Option<Vec<_>>>()?;
    let (h, mi, s) = (*hms.first()?, *hms.get(1)?, hms.get(2).copied().unwrap_or(0));
    let z = p.get(4).copied().unwrap_or("GMT");
    let decal = match z.to_ascii_uppercase().as_str() {
        "GMT" | "UT" | "UTC" | "Z" => 0,
        "EST" => -5 * 3600,
        "EDT" => -4 * 3600,
        "CST" => -6 * 3600,
        "CDT" => -5 * 3600,
        "MST" => -7 * 3600,
        "MDT" => -6 * 3600,
        "PST" => -8 * 3600,
        "PDT" => -7 * 3600,
        x if (x.starts_with('+') || x.starts_with('-')) && x.len() == 5 => {
            let v: i64 = x[1..].parse().ok()?;
            let s = (v / 100) * 3600 + (v % 100) * 60;
            if x.starts_with('-') {
                -s
            } else {
                s
            }
        }
        _ => 0,
    };
    Some(jours(y, m, d) * 86_400 + h * 3600 + mi * 60 + s - decal)
}

/// Date ISO 8601 (`fromisoformat`) → secondes Unix (UTC si aucun fuseau n'est donné).
pub fn date_iso(t: &str) -> Option<i64> {
    let t = t.trim();
    let n = |a: usize, b: usize| t.get(a..b)?.parse::<i64>().ok();
    let (y, m, d) = (n(0, 4)?, n(5, 7)?, n(8, 10)?);
    if t.len() == 10 {
        return Some(jours(y, m, d) * 86_400);
    }
    let (h, mi) = (n(11, 13)?, n(14, 16)?);
    let mut i = 16;
    let mut s = 0;
    if t.as_bytes().get(16) == Some(&b':') {
        s = n(17, 19)?;
        i = 19;
        if t.as_bytes().get(19) == Some(&b'.') {
            i = 20 + t[20..].find(|c: char| !c.is_ascii_digit()).unwrap_or(t.len() - 20);
        }
    }
    let z = &t[i..];
    let decal = if z.is_empty() || z == "Z" {
        0
    } else {
        let signe = if z.starts_with('-') { -1 } else { 1 };
        let v = z[1..].replace(':', "");
        let (hh, mm) = (v.get(0..2)?.parse::<i64>().ok()?, v.get(2..4).and_then(|x| x.parse::<i64>().ok()).unwrap_or(0));
        signe * (hh * 3600 + mm * 60)
    };
    Some(jours(y, m, d) * 86_400 + h * 3600 + mi * 60 + s - decal)
}

fn hex(c: u8) -> Option<u8> {
    (c as char).to_digit(16).map(|v| v as u8)
}

/// `%xx` → octets (et `+` → espace si `plus`).
pub fn pourcent(s: &str, plus: bool) -> String {
    let b = s.as_bytes();
    let mut o = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let (Some(a), Some(c)) = (hex(b[i + 1]), hex(b[i + 2])) {
                o.push(a * 16 + c);
                i += 3;
                continue;
            }
        }
        o.push(if plus && b[i] == b'+' { b' ' } else { b[i] });
        i += 1;
    }
    String::from_utf8_lossy(&o).into_owned()
}

/// `parse_qs` : clés → valeurs (les valeurs vides sont ignorées).
pub fn parse_qs(q: &str) -> HashMap<String, Vec<String>> {
    let mut m: HashMap<String, Vec<String>> = HashMap::new();
    for part in q.split('&') {
        let Some((k, v)) = part.split_once('=') else { continue };
        if v.is_empty() {
            continue;
        }
        m.entry(pourcent(k, true)).or_default().push(pourcent(v, true));
    }
    m
}

fn lire_requete(r: &mut BufReader<TcpStream>, ip: &str) -> Option<Requete> {
    let mut l = String::new();
    loop {
        l.clear();
        if r.read_line(&mut l).ok()? == 0 {
            return None;
        }
        if !l.trim().is_empty() {
            break;
        }
    }
    let mut p = l.split_whitespace();
    let (methode, chemin, version) = (p.next()?.to_string(), p.next()?.to_string(), p.next().unwrap_or("HTTP/0.9").to_string());
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
    Some(Requete {
        methode,
        chemin,
        version,
        entetes,
        ip: ip.to_string(),
    })
}

fn connexion(c: TcpStream, s: &Serveur) -> std::io::Result<()> {
    let ip = c.peer_addr().map(|a| a.ip().to_string()).unwrap_or_default();
    let mut w = c.try_clone()?;
    let mut r = BufReader::new(c);
    while let Some(q) = lire_requete(&mut r, &ip) {
        let rep = match q.methode.as_str() {
            "GET" | "HEAD" => tete(s, &q, &mut r),
            "POST" if route(s, &q).is_some() => route_privee(s, &q, &mut r),
            _ if relais(s, &q).is_some() => relais_get(s, &q),
            "PUT" => {
                if q.sans_requete().starts_with("/api/reglages/") {
                    reglages(s, &q, &mut r)
                } else {
                    let mut x = Reponse::json(405, &J::obj(vec![("error", J::s("méthode refusée"))]), false, true);
                    x.fermer = true;
                    x
                }
            }
            m => {
                let mut x = Reponse::erreur(501, &format!("Unsupported method ({m:?})"));
                x.fermer = true;
                x
            }
        };
        eprintln!(
            "{ip} - - [{}] \"{} {} {}\" {} -",
            date_http(maintenant()),
            q.methode,
            q.chemin,
            q.version,
            rep.code
        );
        let garder = !rep.fermer
            && match q.entete("Connection").map(str::to_ascii_lowercase) {
                Some(c) if c == "close" => false,
                Some(c) if c == "keep-alive" => true,
                _ => q.version == "HTTP/1.1",
            };
        let mut tete_ = format!(
            "HTTP/1.1 {} {}\r\nServer: argos-outils\r\nDate: {}\r\n",
            rep.code,
            raison(rep.code),
            date_http(maintenant())
        );
        for (k, v) in &rep.entetes {
            tete_.push_str(&format!("{k}: {v}\r\n"));
        }
        // toujours vérifier auprès du serveur (304 si rien n'a changé) : jamais une vieille version après make web
        tete_.push_str("Cache-Control: no-cache\r\n");
        if !garder {
            tete_.push_str("Connection: close\r\n");
        }
        tete_.push_str("\r\n");
        w.write_all(tete_.as_bytes())?;
        if q.methode != "HEAD" {
            w.write_all(&rep.corps)?;
        }
        w.flush()?;
        if !garder {
            break;
        }
    }
    Ok(())
}

// ———————————————————————————— fichiers ————————————————————————————

pub fn type_mime(p: &Path) -> &'static str {
    match p.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase).as_deref() {
        Some("html" | "htm") => "text/html",
        Some("js" | "mjs") => "text/javascript",
        Some("css") => "text/css",
        Some("json") => "application/json",
        Some("wasm") => "application/wasm",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("ico") => "image/vnd.microsoft.icon",
        Some("webmanifest") => "application/manifest+json",
        Some("txt") => "text/plain",
        Some("md") => "text/markdown",
        Some("pdf") => "application/pdf",
        Some("woff") => "font/woff",
        Some("woff2") => "font/woff2",
        Some("ttf") => "font/ttf",
        Some("otf") => "font/otf",
        Some("mp4") => "video/mp4",
        Some("webm") => "video/webm",
        Some("mp3") => "audio/mpeg",
        Some("wav") => "audio/x-wav",
        Some("zip") => "application/zip",
        Some("gz") => "application/gzip",
        Some("xml") => "text/xml",
        Some("csv") => "text/csv",
        Some("py") => "text/x-python",
        Some("sh") => "application/x-sh",
        _ => "application/octet-stream",
    }
}

/// Chemin de la requête → fichier sous la racine (sans `..` ni `.`, comme `translate_path`).
pub fn traduire(racine: &Path, chemin: &str) -> PathBuf {
    let c = chemin.split('?').next().unwrap_or("");
    let c = c.split('#').next().unwrap_or("");
    let fin_slash = c.trim_end().ends_with('/');
    let c = pourcent(c, false);
    let mut p = racine.to_path_buf();
    for mot in c.split('/') {
        if mot.is_empty() || mot == "." || mot == ".." {
            continue;
        }
        p.push(mot);
    }
    if fin_slash {
        p.push("");
    }
    p
}

fn mtime(m: &std::fs::Metadata) -> (u128, i64) {
    let d = m.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).unwrap_or_default();
    (d.as_nanos(), d.as_secs() as i64)
}

fn tete(s: &Serveur, q: &Requete, r: &mut BufReader<TcpStream>) -> Reponse {
    let base = q.sans_requete();
    if base == "/api/quote" {
        return quote(s, q);
    }
    if base == "/api/news" || base == "/api/calendar" {
        return eco(s, q);
    }
    if base.starts_with("/api/reglages") {
        return reglages(s, q, r);
    }
    if route(s, q).is_some() {
        return route_privee(s, q, r);
    }
    if relais(s, q).is_some() {
        return relais_get(s, q);
    }
    if let Some(a) = s.accueil.as_deref().filter(|_| base == "/") {
        return Reponse::new(302).h("Location", a).h("Content-Length", "0");
    }
    let mut p = traduire(&s.racine, &q.chemin);
    let slash = base.ends_with('/');
    if p.is_dir() {
        if !slash {
            let (debut, reste) = q.chemin.split_at(q.chemin.find(['?', '#']).unwrap_or(q.chemin.len()));
            return Reponse::new(301).h("Location", format!("{debut}/{reste}")).h("Content-Length", "0");
        }
        let index = ["index.html", "index.htm"].iter().map(|i| p.join(i)).find(|i| i.is_file());
        match index {
            Some(i) => p = i,
            None => return liste(&p, base),
        }
    } else if slash || !p.is_file() {
        return Reponse::erreur(404, "File not found");
    }
    let Ok(m) = std::fs::metadata(&p) else {
        return Reponse::erreur(404, "File not found");
    };
    let (ns, secs) = mtime(&m);
    let ext = p
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{}", e.to_ascii_lowercase()))
        .unwrap_or_default();
    let gzip = COMPRESS.contains(&ext.as_str()) && q.entete("Accept-Encoding").is_some_and(|a| a.contains("gzip"));
    if gzip {
        let Some((data, etag)) = gz(s, &p, ns, m.len()) else {
            return Reponse::erreur(404, "File not found");
        };
        if q.entete("If-None-Match").is_some_and(|x| x.contains(&etag)) {
            return Reponse::new(304).h("ETag", etag).h("Vary", "Accept-Encoding");
        }
        let mut rep = Reponse::new(200)
            .h("Content-Type", type_mime(&p))
            .h("Content-Encoding", "gzip")
            .h("Content-Length", data.len().to_string())
            .h("ETag", etag)
            .h("Vary", "Accept-Encoding")
            .h("Last-Modified", date_http(secs));
        rep.corps = data.to_vec();
        return rep;
    }
    // sans gzip : comme SimpleHTTPRequestHandler (If-Modified-Since → 304)
    if q.entete("If-None-Match").is_none() {
        if let Some(ims) = q.entete("If-Modified-Since").and_then(date_rfc2822) {
            if secs <= ims {
                return Reponse::new(304);
            }
        }
    }
    let Ok(corps) = std::fs::read(&p) else {
        return Reponse::erreur(404, "File not found");
    };
    let mut rep = Reponse::new(200)
        .h("Content-type", type_mime(&p))
        .h("Content-Length", corps.len().to_string())
        .h("Last-Modified", date_http(secs));
    rep.corps = corps;
    rep
}

fn gz(s: &Serveur, p: &Path, ns: u128, taille: u64) -> Option<(Arc<Vec<u8>>, String)> {
    if let Some((a, b, d, e)) = s.gz.lock().ok()?.get(p) {
        if (*a, *b) == (ns, taille) {
            return Some((d.clone(), e.clone()));
        }
    }
    let brut = std::fs::read(p).ok()?;
    let mut enc = flate2::GzBuilder::new().mtime(0).write(Vec::new(), flate2::Compression::new(6));
    enc.write_all(&brut).ok()?;
    let data = Arc::new(enc.finish().ok()?);
    let h = {
        use sha1::Digest;
        let d = sha1::Sha1::digest(&brut);
        d.iter().map(|b| format!("{b:02x}")).collect::<String>()
    };
    let etag = format!("\"{}-gz\"", &h[..16]);
    s.gz.lock().ok()?.insert(p.to_path_buf(), (ns, taille, data.clone(), etag.clone()));
    Some((data, etag))
}

fn html_esc(t: &str) -> String {
    t.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn liste(d: &Path, base: &str) -> Reponse {
    let Ok(rd) = std::fs::read_dir(d) else {
        return Reponse::erreur(404, "No permission to list directory");
    };
    let mut noms: Vec<String> = rd
        .flatten()
        .map(|e| {
            let n = e.file_name().to_string_lossy().into_owned();
            if e.path().is_dir() {
                n + "/"
            } else {
                n
            }
        })
        .collect();
    noms.sort_by_key(|n| n.to_lowercase());
    let titre = html_esc(&pourcent(base, false));
    let items: String = noms
        .iter()
        .map(|n| format!("<li><a href=\"{}\">{}</a></li>\n", html_esc(n), html_esc(n)))
        .collect();
    let corps = format!(
        "<!DOCTYPE HTML>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n<title>Directory listing for {titre}</title>\n\
         </head>\n<body>\n<h1>Directory listing for {titre}</h1>\n<hr>\n<ul>\n{items}</ul>\n<hr>\n</body>\n</html>\n"
    )
    .into_bytes();
    let mut r = Reponse::new(200)
        .h("Content-type", "text/html; charset=utf-8")
        .h("Content-Length", corps.len().to_string());
    r.corps = corps;
    r
}

// ———————————————————————————— 📈 📰 📅 relais ————————————————————————————

const UA: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 Chrome/129.0 Safari/537.36 ArgosChart";
/// 📰 les flux de l'app de bureau (argos-data/src/eco.rs, FEEDS : (source, adresse, langue) ; argos-serveur vérifie
/// par un test que les deux listes sont égales, argos-outils ne dépendant d'aucune crate d'Argos)
pub const NEWS_FEEDS: [(&str, &str, &str); 11] = [
    ("CoinDesk", "https://www.coindesk.com/arc/outboundfeeds/rss/", "en"),
    ("Cointelegraph", "https://cointelegraph.com/rss", "en"),
    ("Decrypt", "https://decrypt.co/feed", "en"),
    ("Bitcoin Magazine", "https://bitcoinmagazine.com/.rss/full/", "en"),
    ("CryptoSlate", "https://cryptoslate.com/feed/", "en"),
    ("FXStreet", "https://www.fxstreet.com/rss/news", "en"),
    ("MarketWatch", "https://feeds.content.dowjones.io/public/rss/mw_topstories", "en"),
    ("Cointribune", "https://www.cointribune.com/feed/", "fr"),
    ("BeInCrypto FR", "https://fr.beincrypto.com/feed/", "fr"),
    ("Journal du Coin", "https://journalducoin.com/feed/", "fr"),
    ("Invezz FR", "https://invezz.com/fr/feed/", "fr"),
];
const CALENDAR_URL: &str = "https://nfs.faireconomy.media/ff_calendar_thisweek.json";
/// 📈 relais Yahoo : seulement ces symboles et ces intervalles (comme api/quote.mjs du site)
const QUOTE_SYMBOLS: [(&str, &str); 1] = [("DXY", "DX-Y.NYB")];
const QUOTE_IV: [(&str, &str); 7] = [
    ("1m", "7d"),
    ("5m", "60d"),
    ("15m", "60d"),
    ("30m", "60d"),
    ("60m", "730d"),
    ("1d", "10y"),
    ("1wk", "max"),
];

fn agent(secs: u64) -> &'static ureq::Agent {
    static A6: OnceLock<ureq::Agent> = OnceLock::new();
    static A8: OnceLock<ureq::Agent> = OnceLock::new();
    let mk = || ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(secs))).build().into();
    if secs <= 6 {
        A6.get_or_init(mk)
    } else {
        A8.get_or_init(mk)
    }
}

/// Une adresse lue (6 s au plus) ; l'erreur : son genre, comme le nom d'exception de l'ancien script.
pub fn fetch(url: &str) -> Result<Vec<u8>, String> {
    let r = agent(6)
        .get(url)
        .header("User-Agent", UA)
        .header("Accept", "application/rss+xml,application/xml,text/xml,application/json,*/*")
        .call();
    match r {
        Ok(mut r) => r.body_mut().with_config().limit(20 << 20).read_to_vec().map_err(|_| "URLError".into()),
        Err(e) => Err(genre(&e).into()),
    }
}

fn genre(e: &ureq::Error) -> &'static str {
    match e {
        ureq::Error::StatusCode(_) => "HTTPError",
        ureq::Error::Timeout(_) => "TimeoutError",
        _ => "URLError",
    }
}

/// Retire les CDATA et les balises, décode les entités HTML.
fn dec(s: &str) -> String {
    let mut t = String::new();
    let mut reste = s;
    while let Some(i) = reste.find("<![CDATA[") {
        t.push_str(&reste[..i]);
        let apres = &reste[i + 9..];
        match apres.find("]]>") {
            Some(j) => {
                t.push_str(&apres[..j]);
                reste = &apres[j + 3..];
            }
            None => {
                t.push_str(&reste[i..i + 9]);
                reste = apres;
            }
        }
    }
    t.push_str(reste);
    // balises <…>
    let mut u = String::new();
    let mut r = t.as_str();
    while let Some(i) = r.find('<') {
        u.push_str(&r[..i]);
        match r[i + 1..].find('>') {
            Some(j) if j > 0 => r = &r[i + 1 + j + 1..],
            _ => {
                u.push('<');
                r = &r[i + 1..];
            }
        }
    }
    u.push_str(r);
    entites(&u).trim().to_string()
}

/// Les entités HTML courantes (`&amp;`, `&#39;`, `&#x2019;`, `&rsquo;`…).
pub fn entites(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    let mut r = s;
    while let Some(i) = r.find('&') {
        o.push_str(&r[..i]);
        let apres = &r[i + 1..];
        let fin = apres.find(';').filter(|&j| j <= 10);
        let rempl = fin.and_then(|j| {
            let n = &apres[..j];
            let c = if let Some(h) = n.strip_prefix("#x").or_else(|| n.strip_prefix("#X")) {
                u32::from_str_radix(h, 16).ok().and_then(char::from_u32).map(String::from)
            } else if let Some(d) = n.strip_prefix('#') {
                d.parse().ok().and_then(char::from_u32).map(String::from)
            } else {
                let v = match n {
                    "amp" => "&",
                    "lt" => "<",
                    "gt" => ">",
                    "quot" => "\"",
                    "apos" => "'",
                    "nbsp" => "\u{a0}",
                    "rsquo" => "’",
                    "lsquo" => "‘",
                    "rdquo" => "”",
                    "ldquo" => "“",
                    "hellip" => "…",
                    "ndash" => "–",
                    "mdash" => "—",
                    "euro" => "€",
                    "pound" => "£",
                    "laquo" => "«",
                    "raquo" => "»",
                    "eacute" => "é",
                    "egrave" => "è",
                    "agrave" => "à",
                    "ccedil" => "ç",
                    "copy" => "©",
                    "reg" => "®",
                    "trade" => "™",
                    _ => return None,
                };
                Some(v.to_string())
            }?;
            Some((c, j))
        });
        match rempl {
            Some((c, j)) => {
                o.push_str(&c);
                r = &apres[j + 1..];
            }
            None => {
                o.push('&');
                r = apres;
            }
        }
    }
    o.push_str(r);
    o
}

/// Cherche `motif` sans tenir compte de la casse (ASCII), à partir de `depuis`.
fn trouver(bas: &str, motif: &str, depuis: usize) -> Option<usize> {
    bas.get(depuis..)?.find(motif).map(|i| i + depuis)
}

/// `<t[^>]*>(…?)</t>` : le contenu de la première balise `t`.
fn balise(it: &str, t: &str) -> String {
    let bas = it.to_ascii_lowercase();
    let (o, f) = (format!("<{}", t.to_ascii_lowercase()), format!("</{}>", t.to_ascii_lowercase()));
    let mut depuis = 0;
    while let Some(i) = trouver(&bas, &o, depuis) {
        if let Some(g) = trouver(&bas, ">", i + o.len()) {
            if let Some(j) = trouver(&bas, &f, g + 1) {
                return dec(&it[g + 1..j]);
            }
        }
        depuis = i + 1;
    }
    String::new()
}

/// Les 30 premiers articles d'un flux RSS / Atom : {title, link, ts (ms), source}.
pub fn parse_rss(xml: &str, src: &str, maintenant_ms: i64) -> Vec<J> {
    let bas = xml.to_ascii_lowercase();
    let mut items = vec![];
    let mut i = 0;
    while items.len() < 30 && i < bas.len() {
        let pi = trouver(&bas, "<item", i);
        let pe = trouver(&bas, "<entry", i);
        let (p, fin) = match (pi, pe) {
            (Some(a), Some(b)) if b < a => (b, "</entry>"),
            (Some(a), _) => (a, "</item>"),
            (None, Some(b)) => (b, "</entry>"),
            (None, None) => break,
        };
        let debut = p + if fin == "</item>" { 5 } else { 6 };
        match trouver(&bas, fin, debut) {
            Some(j) => {
                items.push(&xml[p..j + fin.len()]);
                i = j + fin.len();
            }
            None => i = p + 1,
        }
    }
    let mut out = vec![];
    for it in items {
        let mut link = balise(it, "link");
        if link.is_empty() {
            let b = it.to_ascii_lowercase();
            let mut d = 0;
            while let Some(k) = trouver(&b, "<link", d) {
                let g = trouver(&b, ">", k).unwrap_or(b.len());
                if let Some(h) = b[k..g].find("href=\"") {
                    let a = k + h + 6;
                    if let Some(e) = it[a..].find('"').filter(|&e| e > 0) {
                        link = it[a..a + e].to_string();
                        break;
                    }
                }
                d = k + 1;
            }
        }
        let mut ts = maintenant_ms;
        for d in [balise(it, "pubDate"), balise(it, "dc:date"), balise(it, "updated")] {
            if !d.is_empty() {
                if let Some(x) = date_rfc2822(&d).or_else(|| date_iso(&d.replace('Z', "+00:00"))) {
                    ts = x * 1000;
                }
                break;
            }
        }
        let title = balise(it, "title");
        if !title.is_empty() && (link.starts_with("http://") || link.starts_with("https://")) {
            out.push(J::obj(vec![
                ("title", J::Str(title)),
                ("link", J::Str(link)),
                ("ts", J::Int(ts)),
                ("source", J::s(src)),
                ("lang", J::s(NEWS_FEEDS.iter().find(|f| f.0 == src).map_or("en", |f| f.2))),
            ]));
        }
    }
    out
}

fn ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

/// Les articles de toutes les sources (une en panne n'empêche pas les autres), du plus récent : 200 au plus.
pub fn eco_news(lire: &dyn Fn(&str) -> Result<Vec<u8>, String>) -> Vec<J> {
    let mut items = vec![];
    for (src, url, _) in NEWS_FEEDS {
        if let Ok(x) = lire(url) {
            items.extend(parse_rss(&String::from_utf8_lossy(&x), src, ms()));
        }
    }
    let ts = |j: &J| match j {
        J::Obj(v) => v
            .iter()
            .find(|(k, _)| k == "ts")
            .and_then(|(_, x)| if let J::Int(i) = x { Some(*i) } else { None })
            .unwrap_or(0),
        _ => 0,
    };
    items.sort_by_key(|j| std::cmp::Reverse(ts(j)));
    items.truncate(200);
    items
}

/// 📰 The news relay is kept 60 s; 📅 the Forex Factory table 5 min (it answered 429 for ~10 min when read every
/// minute, 22 Nov 2026).
const GARDE_NEWS_S: u64 = 60;
const GARDE_CALENDRIER_S: u64 = 300;

fn en_cache(s: &Serveur, cle: &'static str, garde_s: u64, f: impl FnOnce() -> (u16, Vec<u8>)) -> Arc<(u16, Vec<u8>)> {
    if let Some((t, v)) = s.eco.lock().ok().and_then(|m| m.get(cle).cloned()) {
        if t.elapsed() < Duration::from_secs(garde_s) {
            return v;
        }
    }
    let v = Arc::new(f());
    if let Ok(mut m) = s.eco.lock() {
        m.insert(cle, (Instant::now(), v.clone()));
    }
    v
}

/// 📅 The calendar relay: the Forex Factory table read at most every 5 min (`lire`; a failed reading keeps the last
/// good table, tried again 5 min later), then its « Réel » filled at each request (`Serveur::calendrier`, its own
/// rhythm: +1 / +3 / +10 / +30 min after a release, 20 requests a day at most).
pub(crate) fn calendrier_relais(s: &Serveur, lire: &dyn Fn() -> Result<Vec<u8>, String>) -> (u16, Vec<u8>) {
    let ancien = s.eco.lock().ok().and_then(|m| m.get("calendar").map(|(_, v)| v.clone())).filter(|v| v.0 == 200);
    let v = en_cache(s, "calendar", GARDE_CALENDRIER_S, || match lire() {
        Ok(x) => (200, x),
        Err(e) => match ancien {
            Some(a) => (200, a.1.clone()),
            None => (
                502,
                dumps(&J::obj(vec![("error", J::Str(format!("calendrier indisponible ({e})")))]), true).into_bytes(),
            ),
        },
    });
    if v.0 != 200 {
        return (v.0, v.1.clone());
    }
    (200, s.calendrier.as_ref().map_or_else(|| v.1.clone(), |f| f(&v.1)))
}

fn eco(s: &Serveur, q: &Requete) -> Reponse {
    if !q.requete().is_empty() {
        return Reponse::json(400, &J::obj(vec![("error", J::s("paramètre inconnu"))]), false, true);
    }
    if q.sans_requete() == "/api/news" {
        let v = en_cache(s, "news", GARDE_NEWS_S, || {
            let sources = J::Arr(NEWS_FEEDS.iter().map(|f| J::s(f.0)).collect());
            (
                200,
                dumps(&J::obj(vec![("items", J::Arr(eco_news(&fetch))), ("sources", sources)]), false).into_bytes(),
            )
        });
        let mut r = Reponse::new(200)
            .h("Content-Type", "application/json; charset=utf-8")
            .h("Content-Length", v.1.len().to_string());
        r.corps = v.1.clone();
        return r;
    }
    let v = calendrier_relais(s, &|| fetch(CALENDAR_URL));
    let mut r = Reponse::new(v.0)
        .h("Content-Type", "application/json; charset=utf-8")
        .h("Content-Length", v.1.len().to_string());
    r.corps = v.1.clone();
    r
}

fn yahoo(s: &Serveur, sym: &str, iv: &str, rng: &str) -> (u16, String) {
    let cle = (sym.to_string(), iv.to_string());
    if let Some((t, c, b)) = s.quotes.lock().ok().and_then(|m| m.get(&cle).cloned()) {
        if t.elapsed() < Duration::from_secs(60) {
            return (c, b);
        }
    }
    let url = format!(
        "https://query1.finance.yahoo.com/v8/finance/chart/{}?interval={iv}&range={rng}",
        sym.replace('^', "%5E")
    );
    let lu = agent(8)
        .get(&url)
        .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) ArgosChart")
        .call()
        .map_err(|e| genre(&e).to_string())
        .and_then(|mut r| r.body_mut().with_config().limit(50 << 20).read_to_vec().map_err(|_| "URLError".to_string()));
    let out = lu.and_then(|b| {
        let x: serde_json::Value = serde_json::from_slice(&b).map_err(|_| "JSONDecodeError".to_string())?;
        let x = x.get("chart").and_then(|c| c.get("result")).and_then(|r| r.get(0)).ok_or("KeyError")?;
        let o = x.get("indicators").and_then(|i| i.get("quote")).and_then(|q| q.get(0)).ok_or("KeyError")?;
        let col = |k: &str| o.get(k).and_then(|v| v.as_array()).ok_or("KeyError");
        let (op, hi, lo, cl, vo) = (col("open")?, col("high")?, col("low")?, col("close")?, col("volume")?);
        let mut bars = vec![];
        for (i, t) in x.get("timestamp").and_then(|t| t.as_array()).ok_or("KeyError")?.iter().enumerate() {
            let v = [&op[i], &hi[i], &lo[i], &cl[i]];
            if v.iter().any(|x| x.is_null()) {
                continue;
            }
            let vol = match &vo[i] {
                serde_json::Value::Null => J::Int(0),
                serde_json::Value::Number(n) if n.as_f64() == Some(0.0) => J::Int(0),
                x => J::from(x),
            };
            bars.push(J::Arr(vec![J::from(t), J::from(v[0]), J::from(v[1]), J::from(v[2]), J::from(v[3]), vol]));
        }
        Ok(dumps(
            &J::obj(vec![("bars", J::Arr(bars)), ("src", J::Str(format!("Yahoo Finance ({sym})")))]),
            true,
        ))
    });
    let (c, b) = match out {
        Ok(b) => (200, b),
        Err(e) => (502, dumps(&J::obj(vec![("error", J::Str(format!("source de prix indisponible ({e})")))]), true)),
    };
    if let Ok(mut m) = s.quotes.lock() {
        m.insert(cle, (Instant::now(), c, b.clone()));
    }
    (c, b)
}

fn quote(s: &Serveur, q: &Requete) -> Reponse {
    let p = parse_qs(q.requete());
    let un = |k: &str| p.get(k).and_then(|v| v.first()).cloned();
    let (sym, iv) = (un("sym").unwrap_or_default(), un("iv").unwrap_or_else(|| "60m".into()));
    let mut res = (
        400,
        dumps(&J::obj(vec![("error", J::s("symbole ou paramètre refusé (seulement sym=DXY)"))]), true),
    );
    let ysym = QUOTE_SYMBOLS.iter().find(|x| x.0 == sym).map(|x| x.1);
    let rng = QUOTE_IV.iter().find(|x| x.0 == iv).map(|x| x.1);
    if let Some(ys) = ysym {
        if (rng.is_some() || iv == "spark") && p.keys().all(|k| k == "sym" || k == "iv") {
            let (yiv, r) = if iv == "spark" { ("30m", "5d") } else { (iv.as_str(), rng.unwrap_or("730d")) };
            res = yahoo(s, ys, yiv, r);
        }
    }
    let mut r = Reponse::new(res.0)
        .h("Content-Type", "application/json")
        .h("Content-Length", res.1.len().to_string());
    r.corps = res.1.into_bytes();
    r
}

// ———————————————————————————— 🔄 session partagée ————————————————————————————

/// Un réglage de session : « nom.json » simple (lettres, chiffres, - et _), pas un fichier interdit, pas de
/// sous-dossier.
pub fn nom_permis(nom: &str) -> bool {
    let n = nom.chars().count();
    if !(nom.ends_with(".json") && 5 < n && n <= 100) || REGLAGES_INTERDITS.contains(&nom) {
        return false;
    }
    let tige = &nom[..nom.len() - 5];
    tige.chars().next().is_some_and(char::is_alphanumeric) && tige.chars().all(|c| c.is_ascii() && (c.is_ascii_alphanumeric() || c == '-' || c == '_'))
}

/// Cet ordi (127.0.0.1, ::1) ou Tailscale (100.64.0.0/10) ; jamais le Wi-Fi local ni Internet.
pub fn client_permis(ip: &str) -> bool {
    let Ok(a) = ip.split('%').next().unwrap_or("").parse::<IpAddr>() else {
        return false;
    };
    let a = match a {
        IpAddr::V6(v) => v.to_ipv4_mapped().map_or(IpAddr::V6(v), IpAddr::V4),
        x => x,
    };
    match a {
        IpAddr::V4(v) => v.is_loopback() || (v.octets()[0] == 100 && (64..128).contains(&v.octets()[1])),
        IpAddr::V6(v) => v.is_loopback(),
    }
}

/// Le nom d'hôte d'un en-tête Host (« pc:8000 » → « pc », « [::1]:8000 » → « ::1 ») ; `None` si mal formé.
fn hote(host: &str) -> Option<String> {
    let net = host.split(['/', '?', '#']).next().unwrap_or("");
    if net.contains('[') != net.contains(']') {
        return None;
    }
    let h = net.rsplit_once('@').map_or(net, |x| x.1);
    let n = if let Some(r) = h.strip_prefix('[') {
        r.split(']').next().unwrap_or("")
    } else {
        h.split(':').next().unwrap_or("")
    };
    Some(n.to_lowercase())
}

/// Contre une page d'un autre site qui viserait ce serveur : l'adresse demandée (Host) doit être cet ordi, une
/// adresse Tailscale ou un nom Tailscale (….ts.net), et une page (Origin) doit venir de la même adresse.
pub fn hote_permis(host: &str, origin: Option<&str>) -> bool {
    let Some(h) = hote(host) else { return false };
    let mut ok = matches!(h.as_str(), "localhost" | "127.0.0.1" | "::1") || h.ends_with(".ts.net");
    if !ok {
        ok = matches!(h.parse::<IpAddr>(), Ok(IpAddr::V4(v)) if v.octets()[0] == 100 && (64..128).contains(&v.octets()[1]));
    }
    if !ok {
        return false;
    }
    let Some(o) = origin else { return true };
    let Some((schema, reste)) = o.split_once("://") else { return false };
    let net = reste.split(['/', '?', '#']).next().unwrap_or("");
    matches!(schema.to_ascii_lowercase().as_str(), "http" | "https") && net.to_lowercase() == host.to_lowercase()
}

fn t_ms(p: &Path) -> Option<i64> {
    let m = std::fs::metadata(p).ok()?;
    Some(m.modified().ok()?.duration_since(UNIX_EPOCH).ok()?.as_millis() as i64)
}

/// {texte, t (dernière écriture, ms)} d'un réglage.
pub fn reglages_lire(dossier: &Path, nom: &str) -> Option<(String, i64)> {
    let p = dossier.join(nom);
    if std::fs::metadata(&p).ok()?.len() > MAX_REGLAGE as u64 {
        return None;
    }
    let t = String::from_utf8(std::fs::read(&p).ok()?).ok()?;
    Some((t, t_ms(&p)?))
}

/// Tous les réglages de session (sans les fichiers interdits), triés par nom.
pub fn reglages_lire_tout(dossier: &Path) -> Vec<(String, String, i64)> {
    let Ok(rd) = std::fs::read_dir(dossier) else { return vec![] };
    let mut noms: Vec<String> = rd.flatten().filter_map(|e| e.file_name().into_string().ok()).collect();
    noms.sort();
    noms.into_iter()
        .filter(|n| nom_permis(n) && dossier.join(n).is_file())
        .filter_map(|n| reglages_lire(dossier, &n).map(|(t, ms)| (n, t, ms)))
        .collect()
}

/// Écrit un réglage (JSON valide ou vide) : fichier temporaire puis renommage, 0600, dossier 0700 ; la dernière
/// écriture gagne. Rend l'heure de l'écriture (ms).
pub fn reglages_ecrire(dossier: &Path, nom: &str, data: &[u8]) -> Result<i64, String> {
    if !nom_permis(nom) {
        return Err("réglage refusé".into());
    }
    if data.len() > MAX_REGLAGE {
        return Err("réglage trop gros".into());
    }
    let texte = std::str::from_utf8(data).map_err(|_| "ce n'est pas un réglage JSON".to_string())?;
    if !texte.trim().is_empty() && serde_json::from_str::<serde_json::Value>(texte).is_err() {
        return Err("ce n'est pas un réglage JSON".into());
    }
    ecrire_fichier(dossier, nom, data).map_err(|e| format!("E/S:{e}"))?;
    t_ms(&dossier.join(nom)).ok_or_else(|| "E/S".into())
}

fn ecrire_fichier(dossier: &Path, nom: &str, data: &[u8]) -> std::io::Result<()> {
    use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
    std::fs::DirBuilder::new().recursive(true).mode(0o700).create(dossier)?;
    let p = dossier.join(nom);
    let tmp = dossier.join(format!("{nom}.{}.{:?}.tmp", std::process::id(), std::thread::current().id()).replace(['(', ')'], ""));
    let r = (|| {
        let mut f = std::fs::OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(&tmp)?;
        f.write_all(data)?;
        f.sync_all()?;
        std::fs::rename(&tmp, &p)
    })();
    if r.is_err() && tmp.exists() {
        let _ = std::fs::remove_file(&tmp);
    }
    r
}

/// 🔢 Un numéro de version par réglage, pour la synchro en direct. Le dossier est relu (taille et date de chaque
/// fichier) à chaque question : un fichier écrit par l'app de bureau a un nouveau numéro. `epoque` change à chaque
/// démarrage du serveur : une page qui a une autre époque reçoit tout.
pub struct Journal {
    pub dossier: PathBuf,
    pub epoque: String,
    etat: Mutex<(i64, HashMap<String, (u128, u64)>, HashMap<String, i64>)>,
}

fn hasard_hex(n: usize) -> String {
    let mut b = vec![0u8; n];
    if std::fs::File::open("/dev/urandom").and_then(|mut f| f.read_exact(&mut b)).is_err() {
        let x = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
        b.iter_mut().enumerate().for_each(|(i, v)| *v = (x >> (8 * (i % 16))) as u8);
    }
    b.iter().map(|x| format!("{x:02x}")).collect()
}

impl Journal {
    pub fn new(dossier: PathBuf) -> Journal {
        let j = Journal {
            dossier,
            epoque: hasard_hex(6),
            etat: Mutex::new((0, HashMap::new(), HashMap::new())),
        };
        j.scanner_(true);
        j
    }

    /// Relit le dossier ; rend le numéro de version courant.
    pub fn scanner(&self) -> i64 {
        self.scanner_(false)
    }

    fn scanner_(&self, premier: bool) -> i64 {
        let mut vus = HashMap::new();
        if let Ok(rd) = std::fs::read_dir(&self.dossier) {
            for e in rd.flatten() {
                let Ok(nom) = e.file_name().into_string() else { continue };
                if !nom_permis(&nom) {
                    continue;
                }
                let Ok(m) = std::fs::metadata(e.path()) else { continue };
                if m.len() <= MAX_REGLAGE as u64 {
                    vus.insert(nom, (mtime(&m).0, m.len()));
                }
            }
        }
        let mut g = self.etat.lock().unwrap_or_else(|e| e.into_inner());
        let (v, anciens, versions) = &mut *g;
        let mut noms: Vec<String> = vus.keys().chain(anciens.keys()).cloned().collect();
        noms.sort();
        noms.dedup();
        for nom in noms {
            if vus.get(&nom) != anciens.get(&nom) {
                if !premier {
                    *v += 1;
                }
                versions.insert(nom, *v);
            }
        }
        *anciens = vus;
        *v
    }

    /// Ce qui a changé après `depuis` : (v, fichiers {nom: texte}, retirés) ; attend au plus `attente` secondes
    /// qu'il se passe quelque chose. Une autre époque (serveur relancé) : tout.
    pub fn changements(&self, epoque: &str, depuis: i64, attente: f64) -> (i64, Vec<(String, String)>, Vec<String>) {
        let fin = Instant::now() + Duration::from_secs_f64(attente.max(0.0));
        loop {
            let v = self.scanner();
            let (mut noms, present) = {
                let g = self.etat.lock().unwrap_or_else(|e| e.into_inner());
                let tout = epoque != self.epoque;
                let noms: Vec<String> = g.2.iter().filter(|(_, k)| tout || **k > depuis).map(|(n, _)| n.clone()).collect();
                (noms, g.1.keys().cloned().collect::<std::collections::HashSet<_>>())
            };
            if !noms.is_empty() || Instant::now() >= fin {
                noms.sort();
                let (mut fichiers, mut retires) = (vec![], vec![]);
                for nom in noms {
                    let r = if present.contains(&nom) { reglages_lire(&self.dossier, &nom) } else { None };
                    match r {
                        Some((t, _)) => fichiers.push((nom, t)),
                        None if !present.contains(&nom) => retires.push(nom),
                        None => {}
                    }
                }
                return (v, fichiers, retires);
            }
            std::thread::sleep(Duration::from_secs_f64(1.0f64.min(fin.saturating_duration_since(Instant::now()).as_secs_f64())));
        }
    }
}

fn journal(s: &Serveur, dossier: &Path) -> Arc<Journal> {
    let mut m = s.journaux.lock().unwrap_or_else(|e| e.into_inner());
    m.entry(dossier.to_path_buf())
        .or_insert_with(|| Arc::new(Journal::new(dossier.to_path_buf())))
        .clone()
}

fn objet(v: Vec<(String, String)>) -> J {
    J::Obj(v.into_iter().map(|(k, t)| (k, J::Str(t))).collect())
}

/// Un refus en JSON (`{"error": …}`) ; la connexion est fermée si le corps de la requête n'a pas été lu.
fn refus_json(q: &Requete, code: u16, m: &str) -> Reponse {
    let mut x = Reponse::json(code, &J::obj(vec![("error", J::s(m))]), false, true);
    x.fermer = matches!(q.methode.as_str(), "PUT" | "POST");
    x
}

/// Les accès privés (`/api/reglages`, routes ajoutées) : cet ordi ou Tailscale, et l'adresse de la page vérifiée.
fn acces_prive(q: &Requete) -> Result<(), Reponse> {
    if !client_permis(&q.ip) {
        return Err(refus_json(q, 403, "seulement depuis cet ordi ou Tailscale"));
    }
    if !hote_permis(q.entete("Host").unwrap_or(""), q.entete("Origin")) {
        return Err(refus_json(q, 403, "adresse de la page refusée"));
    }
    Ok(())
}

/// Le corps d'un PUT / POST privé : `MAX_REGLAGE` octets au plus.
fn lire_corps(q: &Requete, r: &mut BufReader<TcpStream>) -> Result<Vec<u8>, Reponse> {
    let Some(n) = q.entete("Content-Length").and_then(|x| x.trim().parse::<i64>().ok()) else {
        return Err(refus_json(q, 411, "taille inconnue"));
    };
    if n < 0 || n as usize > MAX_REGLAGE {
        return Err(refus_json(q, 413, &format!("trop gros (maximum {} Mio)", MAX_REGLAGE >> 20)));
    }
    let mut data = vec![0u8; n as usize];
    if r.read_exact(&mut data).is_err() {
        return Err(refus_json(q, 400, "corps incomplet"));
    }
    Ok(data)
}

/// La route ajoutée qui sert ce chemin.
fn route<'a>(s: &'a Serveur, q: &Requete) -> Option<&'a Route> {
    s.routes.iter().find(|x| x.chemin == q.sans_requete())
}

/// 🧩 Une route ajoutée (`Route`) : accès privés, puis sa réponse.
fn route_privee(s: &Serveur, q: &Requete, r: &mut BufReader<TcpStream>) -> Reponse {
    let Some(rt) = route(s, q) else {
        return Reponse::erreur(404, "File not found");
    };
    if let Err(x) = acces_prive(q) {
        return x;
    }
    let corps = if q.methode == "POST" {
        match lire_corps(q, r) {
            Ok(d) => Some(d),
            Err(x) => return x,
        }
    } else {
        None
    };
    match (rt.repondre)(corps.as_deref()) {
        Ok(corps) => {
            let mut x = Reponse::new(200)
                .h("Content-Type", "application/json; charset=utf-8")
                .h("Content-Length", corps.len().to_string());
            x.corps = corps;
            x
        }
        Err((code, m)) => Reponse::json(code, &J::obj(vec![("error", J::Str(m))]), false, true),
    }
}

/// The GET relay whose prefix starts this path (« /api/bourse » serves « /api/bourse/… », never « /api/boursex »).
fn relais<'a>(s: &'a Serveur, q: &Requete) -> Option<&'a RelaisGet> {
    let base = q.sans_requete();
    s.relais.iter().find(|x| base.strip_prefix(x.prefixe).is_some_and(|r| r.starts_with('/')))
}

/// 🔁 A GET relay (`RelaisGet`): private access, GET only (anything else 405, connection closed: body not read),
/// then its answer, never kept by the browser.
fn relais_get(s: &Serveur, q: &Requete) -> Reponse {
    let Some(rl) = relais(s, q) else {
        return Reponse::erreur(404, "File not found");
    };
    if q.methode != "GET" {
        let mut x = Reponse::json(405, &J::obj(vec![("error", J::s("méthode refusée"))]), false, true);
        x.fermer = true;
        return x;
    }
    if let Err(x) = acces_prive(q) {
        return x;
    }
    let sans_ancre = q.chemin.split('#').next().unwrap_or("");
    match (rl.repondre)(&sans_ancre[rl.prefixe.len()..]) {
        Ok(corps) => {
            let mut x = Reponse::new(200)
                .h("Content-Type", "application/json; charset=utf-8")
                .h("Content-Length", corps.len().to_string());
            x.corps = corps;
            x
        }
        Err((code, m)) => Reponse::json(code, &J::obj(vec![("error", J::Str(m))]), false, true),
    }
}

fn reglages(s: &Serveur, q: &Requete, r: &mut BufReader<TcpStream>) -> Reponse {
    let refus = |code: u16, m: &str| refus_json(q, code, m);
    let dossier = &s.reglages;
    if let Err(x) = acces_prive(q) {
        return x;
    }
    let permis = |n: &str| !s.interdits.contains(&n);
    let nom = pourcent(q.sans_requete()["/api/reglages".len()..].trim_start_matches('/'), false);
    let j = journal(s, dossier);
    let lire = q.methode != "PUT";
    if lire && nom.is_empty() {
        let tout: Vec<_> = reglages_lire_tout(dossier).into_iter().filter(|(n, _, _)| permis(n)).collect();
        let v = j.scanner();
        let fichiers = J::Obj(tout.iter().map(|(n, t, _)| (n.clone(), J::Str(t.clone()))).collect());
        let ts = J::Obj(tout.iter().map(|(n, _, ms)| (n.clone(), J::Int(*ms))).collect());
        return Reponse::json(
            200,
            &J::obj(vec![("fichiers", fichiers), ("t", ts), ("e", J::s(&j.epoque)), ("v", J::Int(v))]),
            false,
            true,
        );
    }
    if lire && nom == "changements" {
        let p = parse_qs(q.requete());
        let un = |k: &str, d: &str| p.get(k).and_then(|v| v.first()).cloned().unwrap_or_else(|| d.into());
        let (Ok(depuis), Ok(attente)) = (un("depuis", "0").trim().parse::<i64>(), un("attente", "25").trim().parse::<f64>()) else {
            return refus(400, "paramètre refusé");
        };
        let (v, f, ret) = j.changements(&un("e", ""), depuis, attente.clamp(0.0, 25.0));
        let corps = J::obj(vec![
            ("e", J::s(&j.epoque)),
            ("v", J::Int(v)),
            ("fichiers", objet(f.into_iter().filter(|(n, _)| permis(n)).collect())),
            ("retires", J::Arr(ret.into_iter().filter(|n| permis(n)).map(J::Str).collect())),
        ]);
        return Reponse::json(200, &corps, false, true);
    }
    if !nom_permis(&nom) || !permis(&nom) {
        return refus(403, "réglage refusé");
    }
    if lire {
        return match reglages_lire(dossier, &nom) {
            Some((t, ms)) => Reponse::json(200, &J::obj(vec![("texte", J::Str(t)), ("t", J::Int(ms))]), false, true),
            None => Reponse::json(404, &J::obj(vec![("error", J::s("absent"))]), false, true),
        };
    }
    let data = match lire_corps(q, r) {
        Ok(d) => d,
        Err(x) => return x,
    };
    match reglages_ecrire(dossier, &nom, &data) {
        Ok(t) => Reponse::json(200, &J::obj(vec![("t", J::Int(t)), ("v", J::Int(j.scanner()))]), false, true),
        Err(e) if e.starts_with("E/S") => Reponse::json(500, &J::obj(vec![("error", J::s("écriture impossible"))]), false, true),
        Err(e) => Reponse::json(400, &J::obj(vec![("error", J::Str(e))]), false, true),
    }
}

#[cfg(test)]
mod tests;
