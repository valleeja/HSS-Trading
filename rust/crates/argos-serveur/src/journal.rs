//! 🔐 `/api/journal` : le journal chiffré du PC, pour le cell (qui n'en garde aucune copie). GET = le lire ; POST = y
//! écrire des changements entrée par entrée (`argos_store::journal`, même logique que l'app de bureau), puis le
//! relire. Mêmes accès que la synchro (route privée d'argos-outils) ; le journal ne passe jamais par un fichier en
//! clair, et l'ancien `journal.json` en clair n'est plus jamais servi ni écrit par `/api/reglages`.

use argos_outils::serveur::{Route, Serveur};
use argos_store::journal as jr;
use argos_store::sensible::Cle;
use std::path::{Path, PathBuf};

/// Réglages jamais synchronisés : l'ancien journal en clair.
const INTERDITS: [&str; 1] = [jr::ANCIEN];

/// Ajoute `/api/journal` au serveur : la base sensible `base`, et d'où vient sa clé (le trousseau ; une clé fixe dans
/// les tests).
pub fn brancher(s: &mut Serveur, base: Option<PathBuf>, cle: fn() -> Result<Cle, String>) {
    s.interdits = &INTERDITS;
    let ancien = s.reglages.join(jr::ANCIEN);
    s.routes.push(Route {
        chemin: "/api/journal",
        repondre: Box::new(move |corps| repondre(base.as_deref(), cle, &ancien, corps)),
    });
}

/// Lit le journal (`corps` absent) ou y écrit des changements ; l'ancien journal en clair est migré une fois. Trousseau
/// ou base indisponible : 503 « journal indisponible ».
fn repondre(base: Option<&Path>, cle: fn() -> Result<Cle, String>, ancien: &Path, corps: Option<&[u8]>) -> Result<Vec<u8>, (u16, String)> {
    let ch = corps
        .map(serde_json::from_slice::<jr::Changements>)
        .transpose()
        .map_err(|e| (400, format!("changements illisibles : {e}")))?;
    let indisponible = |e: String| (503, format!("journal indisponible : {e}"));
    let base = base.ok_or_else(|| indisponible("pas de dossier de données".into()))?;
    let d = jr::un_moment(base, &cle().map_err(indisponible)?, |b| {
        jr::migrer(b, ancien)?;
        match &ch {
            None => jr::lire(b),
            Some(ch) => jr::appliquer(b, ch),
        }
    })
    .map_err(indisponible)?;
    serde_json::to_vec(&d).map_err(|e| (500, e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::Arc;

    /// 🧪 La clé du journal dans les tests (jamais le vrai trousseau).
    fn cle_test() -> Result<Cle, String> {
        Ok([7; 32])
    }

    fn sans_trousseau() -> Result<Cle, String> {
        Err("trousseau absent (pas de session)".into())
    }

    /// Un serveur de test sur un port libre.
    fn lancer(charts: &Path, base: PathBuf, cle: fn() -> Result<Cle, String>) -> u16 {
        let l = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = l.local_addr().unwrap().port();
        let mut s = Serveur::new(charts.join("../www"), charts.to_path_buf());
        brancher(&mut s, Some(base), cle);
        std::thread::spawn(move || argos_outils::serveur::servir(l, Arc::new(s)));
        port
    }

    /// Une requête HTTP/1.1 (Host de cet ordi, sauf si donné) : (code, corps).
    fn req(port: u16, methode: &str, chemin: &str, corps: &[u8], entetes: &[(&str, String)]) -> (u16, Vec<u8>) {
        let mut c = TcpStream::connect(("127.0.0.1", port)).unwrap();
        let mut h = format!("{methode} {chemin} HTTP/1.1\r\nConnection: close\r\nContent-Length: {}\r\n", corps.len());
        if !entetes.iter().any(|(k, _)| *k == "Host") {
            h.push_str(&format!("Host: 127.0.0.1:{port}\r\n"));
        }
        for (k, v) in entetes {
            h.push_str(&format!("{k}: {v}\r\n"));
        }
        c.write_all(format!("{h}\r\n").as_bytes()).unwrap();
        let _ = c.write_all(corps);
        let mut tout = vec![];
        let _ = c.read_to_end(&mut tout);
        let fin = tout.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
        let code = String::from_utf8_lossy(&tout[..fin]).split(' ').nth(1).unwrap().parse().unwrap();
        (code, tout[fin + 4..].to_vec())
    }

    fn json(b: &[u8]) -> serde_json::Value {
        serde_json::from_slice(b).unwrap()
    }

    /// 🔐 Le cell lit et écrit le journal chiffré du PC (entrée par entrée) ; l'ancien journal.json en clair est migré
    /// puis effacé ; l'ancien chemin en clair est refusé ; rien en clair sur le disque ; mêmes accès que la synchro ;
    /// trousseau absent : « journal indisponible ».
    #[test]
    fn journal_chiffre_pour_le_cell() {
        // dans le dossier de compilation du projet (rust/target/tmp), jamais dans /tmp
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../target/tmp/argos-serveur-journal-{}", std::process::id()));
        let charts = base.join("charts");
        std::fs::create_dir_all(&charts).unwrap();
        std::fs::write(
            charts.join("journal.json"),
            r#"{"trades":[{"id":"vieux","sym":"BTC","notes":"VIEUX-SECRET"}],"cfg":{"tilt_n":2}}"#,
        )
        .unwrap();
        let port = lancer(&charts, base.join("data/sensible.redb"), cle_test);
        // l'ancien journal en clair n'est jamais servi par la synchro, même avant la migration
        assert_eq!(req(port, "GET", "/api/reglages/journal.json", b"", &[]).0, 403);
        // 1re lecture : l'ancien journal en clair est migré, puis effacé
        let (st, b) = req(port, "GET", "/api/journal", b"", &[]);
        assert_eq!(st, 200, "{}", String::from_utf8_lossy(&b));
        let j = json(&b);
        assert_eq!(
            (j["trades"][0]["id"].as_str(), j["cfg"]["tilt_n"].as_u64(), j["version"].as_u64()),
            (Some("vieux"), Some(2), Some(1))
        );
        assert!(!charts.join("journal.json").exists(), "ancien fichier en clair effacé après migration");
        // une écriture du cell : seulement ses changements, le reste n'est pas touché
        let ch = br#"{"mettre":[{"id":"cell-1","sym":"ETH","notes":"CELL-SECRET"}]}"#;
        let origine = [("Origin", format!("http://127.0.0.1:{port}"))];
        let (st, b) = req(port, "POST", "/api/journal", ch, &origine);
        assert_eq!(st, 200, "{}", String::from_utf8_lossy(&b));
        let j = json(&b);
        assert_eq!((j["trades"].as_array().unwrap().len(), j["version"].as_u64()), (2, Some(2)), "rien de perdu");
        // sur le disque : rien du journal en clair
        let brut = std::fs::read(base.join("data/sensible.redb")).unwrap();
        for secret in [&b"VIEUX-SECRET"[..], b"CELL-SECRET"] {
            assert!(!brut.windows(secret.len()).any(|w| w == secret), "en clair sur le disque");
        }
        // l'ancien chemin en clair est refusé ; changements illisibles ; autre page
        assert_eq!(req(port, "PUT", "/api/reglages/journal.json", b"{}", &[]).0, 403);
        assert!(!charts.join("journal.json").exists());
        assert_eq!(req(port, "POST", "/api/journal", b"pas du json", &[]).0, 400);
        assert_eq!(req(port, "POST", "/api/journal", b"{}", &[("Origin", "https://evil.com".into())]).0, 403);
        assert_eq!(req(port, "GET", "/api/journal", b"", &[("Host", "evil.com".into())]).0, 403);
        // trousseau indisponible (serveur lancé sans session) : réponse propre
        let p2 = lancer(&charts, base.join("data2/sensible.redb"), sans_trousseau);
        let (st, b) = req(p2, "GET", "/api/journal", b"", &[]);
        assert_eq!(
            (st, json(&b)["error"].as_str()),
            (503, Some("journal indisponible : trousseau absent (pas de session)"))
        );
        let _ = std::fs::remove_dir_all(&base);
    }
}
