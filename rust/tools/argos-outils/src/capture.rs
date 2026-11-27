//! 🖼 `publier-capture` : une capture à montrer, copiée dans le dossier stable des captures servies (hors des côtés A / B
//! de /app2/, servi sur /captures/), puis vérifiée par une vraie requête sur chaque adresse (code 200 et image) ;
//! sinon échec, et aucune adresse n'est donnée. Une adresse de capture n'est écrite dans une note qu'après ce ✅.
//!
//! ```text
//! argos-outils publier-capture <fichier.png> [nom.png] [adresse…] [--racine R]
//! ```
//! Le dossier : `$ARGOS_CAPTURES_DOSSIER`, sinon `captures-servies` à côté du dépôt (`<racine>/../../captures-servies`).
//! Les adresses (sans `/captures/…`) : celles données, sinon `http://127.0.0.1:8000`.

use crate::Args;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Adresse vérifiée quand aucune n'est donnée : le serveur local de /app2/.
pub const ADRESSE_LOCALE: &str = "http://127.0.0.1:8000";

pub fn main(a: &Args) -> Result<(), String> {
    let (fichier, nom, adresses) = lire(&a.pos)?;
    let dossier = std::env::var_os("ARGOS_CAPTURES_DOSSIER")
        .map(PathBuf::from)
        .unwrap_or_else(|| a.racine.join("../../captures-servies"));
    let liens = publier(Path::new(&fichier), &nom, &dossier, &adresses)?;
    for l in liens {
        println!("✅ {l}");
    }
    Ok(())
}

/// Fichier, nom publié (par défaut celui du fichier) et adresses (par défaut l'adresse locale).
fn lire(pos: &[String]) -> Result<(String, String, Vec<String>), String> {
    let fichier = pos.first().ok_or("publier-capture <fichier.png> [nom.png] [adresse…]")?.clone();
    let (mut nom, mut adresses) = (None, vec![]);
    for p in &pos[1..] {
        if p.starts_with("http://") || p.starts_with("https://") {
            adresses.push(p.trim_end_matches('/').to_string());
        } else if nom.is_none() {
            nom = Some(p.clone());
        } else {
            return Err(format!("argument en trop : « {p} »"));
        }
    }
    let nom = match nom {
        Some(n) => n,
        None => Path::new(&fichier)
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or("nom de fichier illisible")?
            .to_string(),
    };
    if nom.contains('/') || nom.contains("..") || nom.is_empty() {
        return Err(format!("nom de capture refusé : « {nom} »"));
    }
    if adresses.is_empty() {
        adresses.push(ADRESSE_LOCALE.to_string());
    }
    Ok((fichier, nom, adresses))
}

/// Copie `fichier` sous `dossier/nom`, puis vérifie `adresse/captures/nom` pour chaque adresse ; renvoie les liens.
pub fn publier(fichier: &Path, nom: &str, dossier: &Path, adresses: &[String]) -> Result<Vec<String>, String> {
    std::fs::create_dir_all(dossier).map_err(|e| format!("{} : {e}", dossier.display()))?;
    std::fs::copy(fichier, dossier.join(nom)).map_err(|e| format!("{} : {e}", fichier.display()))?;
    adresses
        .iter()
        .map(|a| {
            let url = format!("{a}/captures/{nom}");
            verifier(&url).map(|()| url)
        })
        .collect()
}

/// Vraie requête : il faut le code 200 et un type image.
pub fn verifier(url: &str) -> Result<(), String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(10)))
        .http_status_as_error(false)
        .build()
        .into();
    let r = agent.get(url).call().map_err(|e| format!("{url} : {e}"))?;
    let code = r.status().as_u16();
    let genre = r.headers().get("Content-Type").and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
    if code != 200 || !genre.starts_with("image/") {
        return Err(format!("{url} : {code} {genre} (il faut 200 et une image)"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};

    /// Petit serveur d'une seule réponse par connexion : /captures/bon.png → 200 image/png, le reste → 404.
    fn serveur() -> String {
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let adr = format!("http://{}", l.local_addr().unwrap());
        std::thread::spawn(move || {
            for c in l.incoming().flatten().take(8) {
                let mut c = c;
                let mut buf = [0u8; 2048];
                let n = c.read(&mut buf).unwrap_or(0);
                let req = String::from_utf8_lossy(&buf[..n]).to_string();
                let rep = if req.starts_with("GET /captures/bon.png ") {
                    "HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nContent-Length: 3\r\nConnection: close\r\n\r\nPNG"
                } else {
                    "HTTP/1.1 404 Not Found\r\nContent-Type: text/html\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                };
                let _ = c.write_all(rep.as_bytes());
            }
        });
        adr
    }

    #[test]
    fn publie_et_verifie_chaque_adresse() {
        let adr = serveur();
        let d = std::env::temp_dir().join(format!("argos-captures-{}", std::process::id()));
        let f = d.join("source.png");
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(&f, b"PNG").unwrap();
        let dossier = d.join("servies");
        // servie : 200 image/png → le lien
        let liens = publier(&f, "bon.png", &dossier, std::slice::from_ref(&adr)).unwrap();
        assert_eq!(liens, vec![format!("{adr}/captures/bon.png")]);
        assert!(dossier.join("bon.png").exists(), "copiée dans le dossier stable");
        // absente côté serveur : 404 → échec, pas de lien
        let e = publier(&f, "absent.png", &dossier, &[adr]).unwrap_err();
        assert!(e.contains("404"), "{e}");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn arguments() {
        let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        let (f, n, a) = lire(&s(&["x/gardien.png"])).unwrap();
        assert_eq!((f.as_str(), n.as_str(), a), ("x/gardien.png", "gardien.png", vec![ADRESSE_LOCALE.to_string()]));
        let (_, n, a) = lire(&s(&["x.png", "autre.png", "https://h:8443/"])).unwrap();
        assert_eq!((n.as_str(), a), ("autre.png", vec!["https://h:8443".to_string()]));
        assert!(lire(&s(&["x.png", "../evil.png"])).is_err(), "pas de chemin");
        assert!(lire(&s(&[])).is_err());
    }
}
