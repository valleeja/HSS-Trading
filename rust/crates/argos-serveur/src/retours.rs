//! 🛟 `/api/retours…`: the Support tickets (problem, idea, question) sent by the page, stored in the tickets folder
//! (`Argos HSS Trading/retours/<numéro>-<date>-<type>/`: `billet.json`, the screenshot, `note.txt`).
//! Same access as sync (argos-outils private routes: this computer or Tailscale, page origin checked,
//! body of at most `MAX_REGLAGE`). Nothing goes out to the Internet. The format lives in `argos_core::retours`.
//!
//! - `GET /api/retours`: the tickets, newest first (without the screenshot).
//! - `POST /api/retours`: a new ticket (`Nouveau`); the server picks the number and the folder name.
//! - `POST /api/retours/lu`: `{"id"}` (or `{}`: all) — Argos replies are marked as seen.
//! - `POST /api/retours/repondre`: `{"id", "texte", "auteur"?}` (`admin` by default, or `argos`).
//! - `POST /api/retours/etat`: `{"id", "etat", "auteur"?}` (`Ouvert`, `En cours`, `Résolu`).

use argos_core::retours::{Billet, Etat, Nouveau};
use argos_outils::serveur::{Route, Serveur};
use argos_store::retours as disque;
use std::path::{Path, PathBuf};

/// Adds the `/api/retours…` routes; tickets go into `dossier` (written to the server log).
pub fn brancher(s: &mut Serveur, dossier: PathBuf) {
    println!("🛟 Support : billets dans {}", dossier.display());
    let mut route = |chemin: &'static str, f: fn(&Path, Option<&[u8]>) -> Result<Vec<u8>, (u16, String)>| {
        let d = dossier.clone();
        s.routes.push(Route {
            chemin,
            repondre: Box::new(move |corps| f(&d, corps)),
        });
    };
    route("/api/retours", liste_ou_nouveau);
    route("/api/retours/lu", lu);
    route("/api/retours/repondre", repondre);
    route("/api/retours/etat", etat);
}

/// This server's tickets folder: `$ARGOS_RETOURS`, otherwise `Argos HSS Trading/retours` above the served folder,
/// otherwise the data folder.
pub fn dossier(racine: &Path) -> PathBuf {
    disque::dossier(Some(racine))
}

fn json(x: &Billet) -> Result<Vec<u8>, (u16, String)> {
    serde_json::to_vec(x).map_err(|e| (500, e.to_string()))
}

fn liste_ou_nouveau(d: &Path, corps: Option<&[u8]>) -> Result<Vec<u8>, (u16, String)> {
    let Some(corps) = corps else {
        return serde_json::to_vec(&disque::liste(d)).map_err(|e| (500, e.to_string()));
    };
    let n: Nouveau = serde_json::from_slice(corps).map_err(|e| (400, format!("billet illisible : {e}")))?;
    json(&disque::creer(d, n)?)
}

/// A request `{"id", "texte"?, "auteur"?, "etat"?}`.
struct Demande {
    id: String,
    texte: String,
    auteur: String,
    etat: Option<Etat>,
}

fn demande(corps: Option<&[u8]>) -> Result<Demande, (u16, String)> {
    let c = corps.ok_or((405, "POST seulement".to_string()))?;
    let v: serde_json::Value = serde_json::from_slice(c).map_err(|e| (400, format!("demande illisible : {e}")))?;
    let s = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
    let etat = match v.get("etat") {
        None => None,
        Some(e) => Some(serde_json::from_value(e.clone()).map_err(|_| (400, format!("état inconnu : {e}")))?),
    };
    let auteur = Some(s("auteur")).filter(|a| !a.is_empty()).unwrap_or_else(|| "admin".into());
    Ok(Demande {
        id: s("id"),
        texte: s("texte"),
        auteur,
        etat,
    })
}

fn lu(d: &Path, corps: Option<&[u8]>) -> Result<Vec<u8>, (u16, String)> {
    let q = demande(corps)?;
    if q.id.is_empty() {
        disque::tous_lus(d)?;
        return Ok(br#"{"ok":true}"#.to_vec());
    }
    json(&disque::changer(d, &q.id, |b| {
        b.marquer_lu();
        Ok(())
    })?)
}

fn repondre(d: &Path, corps: Option<&[u8]>) -> Result<Vec<u8>, (u16, String)> {
    let q = demande(corps)?;
    json(&disque::changer(d, &q.id, |b| b.repondre(&q.auteur, &q.texte, &disque::maintenant()))?)
}

fn etat(d: &Path, corps: Option<&[u8]>) -> Result<Vec<u8>, (u16, String)> {
    let q = demande(corps)?;
    let e = q.etat.ok_or((400, "état manquant".to_string()))?;
    json(&disque::changer(d, &q.id, |b| b.changer_etat(e, &q.auteur, &disque::maintenant()))?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use argos_core::retours as rt;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::Arc;

    fn lancer(d: &Path) -> u16 {
        let l = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = l.local_addr().unwrap().port();
        let mut s = Serveur::new(d.join("www"), d.join("charts"));
        brancher(&mut s, d.join("retours"));
        std::thread::spawn(move || argos_outils::serveur::servir(l, Arc::new(s)));
        port
    }

    /// An HTTP/1.1 request: (code, body).
    fn req(port: u16, methode: &str, chemin: &str, corps: &[u8], host: &str) -> (u16, serde_json::Value) {
        let mut c = TcpStream::connect(("127.0.0.1", port)).unwrap();
        let h = format!(
            "{methode} {chemin} HTTP/1.1\r\nConnection: close\r\nHost: {host}\r\nContent-Length: {}\r\n\r\n",
            corps.len()
        );
        c.write_all(h.as_bytes()).unwrap();
        c.write_all(corps).unwrap();
        let mut r = vec![];
        c.read_to_end(&mut r).unwrap();
        let t = String::from_utf8_lossy(&r);
        let code = t[9..12].parse().unwrap();
        let corps = t.split_once("\r\n\r\n").map_or("", |x| x.1);
        (code, serde_json::from_str(corps).unwrap_or(serde_json::Value::Null))
    }

    const PNG: [u8; 12] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 13];

    /// 🛟 Sending a ticket with a screenshot, list, reply, state, read; and the rejections.
    #[test]
    fn billets_par_le_serveur() {
        let d = std::env::temp_dir().join(format!("argos-retours-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let port = lancer(&d);
        let host = format!("127.0.0.1:{port}");
        let nouveau = |objet: &str, image: &[u8]| {
            serde_json::json!({ "type": "probleme", "objet": objet, "description": "détail",
                "contexte": { "version": "v", "ecran": "chart", "symbole": "BTC", "ut": "30m", "taille": "1440×900" },
                "appareil": "pc", "image": rt::base64_ecrire(image) })
            .to_string()
        };
        let (code, b) = req(port, "POST", "/api/retours", nouveau("Le bouton AI", &PNG).as_bytes(), &host);
        assert_eq!(code, 200, "{b}");
        assert_eq!(b["numéro"], 1);
        assert_eq!(b["capture"], "capture.png");
        let id = b["id"].as_str().unwrap().to_string();
        let jour = &b["historique"][0]["date"].as_str().unwrap()[..10];
        let dos = d.join("retours").join(format!("0001-{jour}-probleme"));
        assert_eq!(std::fs::read(dos.join("capture.png")).unwrap(), PNG);
        assert!(dos.join("billet.json").exists() && dos.join("note.txt").exists());
        // the 2nd ticket gets number 2; the list, newest first
        let (code, _) = req(port, "POST", "/api/retours", nouveau("Une question", &[]).as_bytes(), &host);
        assert_eq!(code, 200);
        let (_, l) = req(port, "GET", "/api/retours", b"", &host);
        assert_eq!((l[0]["numéro"].as_u64(), l[1]["numéro"].as_u64()), (Some(2), Some(1)));
        // Argos reply, badge, read; state
        let r = serde_json::json!({ "id": id, "texte": "Je regarde.", "auteur": "argos" }).to_string();
        let (code, b) = req(port, "POST", "/api/retours/repondre", r.as_bytes(), &host);
        assert_eq!((code, b["réponses"][0]["auteur"].as_str()), (200, Some("argos")));
        assert_eq!(b["vues"], 0);
        let (_, b) = req(port, "POST", "/api/retours/lu", serde_json::json!({ "id": id }).to_string().as_bytes(), &host);
        assert_eq!(b["vues"], 1);
        let e = serde_json::json!({ "id": id, "etat": "Résolu" }).to_string();
        let (_, b) = req(port, "POST", "/api/retours/etat", e.as_bytes(), &host);
        assert_eq!((b["etat"].as_str(), b["historique"][1]["auteur"].as_str()), (Some("Résolu"), Some("admin")));
        // rejections: empty subject, not an image, unknown author, unknown ticket, other page origin
        assert_eq!(req(port, "POST", "/api/retours", nouveau(" ", &[]).as_bytes(), &host).0, 400);
        assert_eq!(req(port, "POST", "/api/retours", nouveau("x", b"GIF89a").as_bytes(), &host).0, 415);
        let r = serde_json::json!({ "id": id, "texte": "x", "auteur": "pirate" }).to_string();
        assert_eq!(req(port, "POST", "/api/retours/repondre", r.as_bytes(), &host).0, 400);
        let r = serde_json::json!({ "id": "nope", "texte": "x" }).to_string();
        assert_eq!(req(port, "POST", "/api/retours/repondre", r.as_bytes(), &host).0, 404);
        assert_eq!(req(port, "GET", "/api/retours", b"", "exemple.com").0, 403);
        assert_eq!(req(port, "GET", "/api/retours/lu", b"", &host).0, 405);
        assert_eq!(disque::liste(&d.join("retours")).len(), 2, "aucun dossier pour un envoi refusé");
        let _ = std::fs::remove_dir_all(&d);
    }
}
