//! 🔑 📈 🪙 CoinMarketCap pour la page (PC et cell) : la clé reste au trousseau du PC, la page n'en garde jamais.
//! - `/api/cles/cmc` : GET = `{"present": bool}` (jamais la valeur) ; POST `{"cle": "…"}` = la ranger au trousseau,
//!   POST `{"oublier": true}` = l'effacer ;
//! - `/api/cmc/<chemin>` : relais GET d'une liste fermée d'appels (`cmcap::relais_permis`) ; le serveur y met la clé
//!   du trousseau et les paramètres d'Argos, et refuse si le plafond de crédits du mois est atteint ;
//! - `/api/logos` : POST `{"bases": ["BTC", …]}` = `{"BTC": [octets du PNG], …}` (table intégrée, carte des ids
//!   avec la clé, cache des logos du PC).
//!
//! Mêmes accès que `/api/journal` (cet ordi ou Tailscale, adresse de la page vérifiée) : une page d'un autre site
//! est refusée. La clé n'est jamais écrite dans un fichier, un journal ni une réponse.

use argos_data::cmcap::{self, CreditBox, Credits};
use argos_data::logos::{MAX_PAR_DEMANDE as MAX_LOGOS, ROUTE_LOGOS};
use argos_outils::serveur::{Route, Serveur};
use serde_json::{json, Value};
use std::path::Path;
use std::sync::Arc;

/// Ce que les routes utilisent : la clé du trousseau, l'appel à CoinMarketCap et la recherche des logos (des
/// fonctions de test dans les tests : jamais le vrai trousseau ni le réseau).
#[derive(Clone, Copy)]
pub struct Outils {
    pub lire_cle: fn() -> Option<String>,
    pub ranger_cle: fn(&str) -> Result<(), String>,
    pub oublier_cle: fn(),
    /// GET (adresse complète, clé) → corps.
    pub appeler: fn(&str, &str) -> Result<String, String>,
    /// (bases, compteur) → (base, PNG) trouvés.
    pub logos: fn(Vec<String>, CreditBox) -> Vec<(String, Vec<u8>)>,
    /// Attente la plus longue d'une réponse du trousseau.
    pub delai: std::time::Duration,
}

/// Le trousseau du PC (service argos-hss-trading, entrée « cmc.key »), CoinMarketCap et le cache des logos.
pub const VRAIS: Outils = Outils {
    lire_cle: || argos_store::cles::lire("cmc", "key"),
    ranger_cle: |v| argos_store::cles::ranger("cmc", "key", v),
    oublier_cle: || argos_store::cles::oublier("cmc"),
    appeler: |url, cle| argos_data::rt::bloquer(cmcap::get(url, cle)).unwrap_or_else(|| Err("tokio indisponible".into())),
    logos: logos_du_pc,
    delai: std::time::Duration::from_secs(15),
};

/// Ajoute les routes au serveur ; `reglages` : le dossier du compteur de crédits (`cmc_credits.json`, partagé avec
/// l'app).
pub fn brancher(s: &mut Serveur, reglages: &Path, o: Outils) {
    let credits = Arc::new(reglages.join("cmc_credits.json"));
    s.routes.push(Route {
        chemin: "/api/cles/cmc",
        repondre: Box::new(move |corps| cle(o, corps)),
    });
    for (chemin, complete) in cmcap::relais_permis() {
        let c = credits.clone();
        s.routes.push(Route {
            chemin: Box::leak(chemin.into_boxed_str()),
            repondre: Box::new(move |corps| relais(o, &c, &complete, corps)),
        });
    }
    s.routes.push(Route {
        chemin: ROUTE_LOGOS,
        repondre: Box::new(move |corps| logos(o, &credits, corps)),
    });
}

/// Le trousseau ne répond pas dans le délai (verrouillé, ou son démon vient de redémarrer) : une erreur claire pour la
/// page, au lieu d'une demande qui attend sans fin (la page restait sur « envoi au trousseau de ton PC… »).
fn trousseau_muet() -> (u16, String) {
    (
        503,
        "le trousseau du PC ne répond pas : ouvre ta session du PC (trousseau déverrouillé) puis réessaie".into(),
    )
}

/// `f` (un appel au trousseau) dans son propre fil, au plus `o.delai` ; `None` s'il ne répond pas à temps.
fn au_trousseau<T: Send + 'static>(o: Outils, f: impl FnOnce() -> T + Send + 'static) -> Option<T> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(f());
    });
    rx.recv_timeout(o.delai).ok()
}

fn present(o: Outils) -> Result<Vec<u8>, (u16, String)> {
    let la = au_trousseau(o, move || (o.lire_cle)().is_some()).ok_or_else(trousseau_muet)?;
    Ok(json!({ "present": la }).to_string().into_bytes())
}

/// `/api/cles/cmc` : présence, ou ranger / oublier (la valeur n'est jamais renvoyée).
fn cle(o: Outils, corps: Option<&[u8]>) -> Result<Vec<u8>, (u16, String)> {
    let Some(corps) = corps else { return present(o) };
    let v: Value = serde_json::from_slice(corps).map_err(|_| (400, "demande illisible".to_string()))?;
    if v["oublier"].as_bool() == Some(true) {
        au_trousseau(o, move || (o.oublier_cle)()).ok_or_else(trousseau_muet)?;
        return present(o);
    }
    let cle = v["cle"].as_str().map(str::trim).unwrap_or("").to_string();
    if !cmcap::cle_valide(&cle) {
        return Err((
            400,
            "clé CoinMarketCap invalide : copie-la au complet depuis pro.coinmarketcap.com ▸ API Key".into(),
        ));
    }
    au_trousseau(o, move || (o.ranger_cle)(&cle))
        .ok_or_else(trousseau_muet)?
        .map_err(|e| (503, format!("{e} (le trousseau du PC est-il ouvert ?)")))?;
    present(o)
}

/// Le compteur de crédits du mois (`cmc_credits.json`), ou un compteur neuf.
fn lire_credits(f: &Path) -> Credits {
    std::fs::read(f).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}

fn maintenant_s() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}

/// `/api/cmc/…` : relais GET d'un appel permis, avec la clé du trousseau (la page compte ses crédits, comme l'app).
fn relais(o: Outils, credits: &Path, complete: &str, corps: Option<&[u8]>) -> Result<Vec<u8>, (u16, String)> {
    if corps.is_some() {
        return Err((405, "relais CoinMarketCap : GET seulement".into()));
    }
    let cle = (o.lire_cle)().ok_or((401, "pas de clé CoinMarketCap sur le PC : ajoute-la (🔑 Clé API)".to_string()))?;
    let mut c = lire_credits(credits);
    c.roll(maintenant_s());
    if !c.can_spend(1) {
        return Err((429, format!("plafond de {} crédits CoinMarketCap du mois atteint", cmcap::CREDITS_CAP)));
    }
    (o.appeler)(complete, &cle).map(String::into_bytes).map_err(|e| (502, e))
}

/// `/api/logos` : les logos de ces bases (au plus [`MAX_LOGOS`], symboles simples) ; le compteur gardé s'il a bougé.
fn logos(o: Outils, credits: &Path, corps: Option<&[u8]>) -> Result<Vec<u8>, (u16, String)> {
    let corps = corps.ok_or((405, "logos : POST seulement".to_string()))?;
    let v: Value = serde_json::from_slice(corps).map_err(|_| (400, "demande illisible".to_string()))?;
    let bases: Vec<String> = v["bases"]
        .as_array()
        .ok_or((400, "logos : liste « bases » attendue".to_string()))?
        .iter()
        .filter_map(|b| b.as_str())
        .filter(|b| (1..=32).contains(&b.len()) && b.chars().all(|c| c.is_ascii_alphanumeric()))
        .map(String::from)
        .collect();
    if bases.len() > MAX_LOGOS {
        return Err((413, format!("logos : {MAX_LOGOS} au plus d'un coup")));
    }
    let avant = lire_credits(credits);
    let boite: CreditBox = Arc::new(std::sync::Mutex::new(avant));
    let trouves = (o.logos)(bases, boite.clone());
    if let Ok(apres) = boite.lock().map(|c| *c) {
        if apres != avant {
            let _ = std::fs::write(credits, serde_json::to_string_pretty(&apres).unwrap_or_default());
        }
    }
    let carte: serde_json::Map<String, Value> = trouves.into_iter().map(|(b, png)| (b, json!(png))).collect();
    Ok(Value::Object(carte).to_string().into_bytes())
}

/// Les logos du PC : la clé du trousseau (s'il y en a une) règle la carte des ids, puis chaque base est cherchée.
fn logos_du_pc(bases: Vec<String>, credits: CreditBox) -> Vec<(String, Vec<u8>)> {
    cmcap::regler(argos_store::cles::lire("cmc", "key").map(|k| (cmcap::DEFAULT_URL.to_string(), k, credits)));
    argos_data::rt::bloquer(async move {
        let mut ch = argos_data::logos::Chercheur::default();
        let mut out = vec![];
        for b in bases {
            if let Some(png) = ch.png(&b).await {
                out.push((b, png));
            }
        }
        out
    })
    .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::Mutex;

    /// 🧪 Le « trousseau » des tests : une valeur en mémoire (jamais le vrai trousseau).
    static CLE: Mutex<Option<String>> = Mutex::new(None);
    /// 🧪 Le dernier appel relayé : (adresse, clé).
    static APPEL: Mutex<Option<(String, String)>> = Mutex::new(None);

    const TEST: Outils = Outils {
        lire_cle: || CLE.lock().unwrap().clone(),
        ranger_cle: |v| {
            *CLE.lock().unwrap() = Some(v.to_string());
            Ok(())
        },
        oublier_cle: || *CLE.lock().unwrap() = None,
        appeler: |url, cle| {
            *APPEL.lock().unwrap() = Some((url.to_string(), cle.to_string()));
            Ok(r#"{"status":{"error_code":0},"data":{}}"#.into())
        },
        logos: |bases, credits| {
            credits.lock().unwrap().spend(1);
            bases.into_iter().filter(|b| b == "BTC").map(|b| (b, vec![137, 80, 78, 71])).collect()
        },
        delai: std::time::Duration::from_secs(5),
    };

    fn lancer(charts: &Path) -> u16 {
        let l = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = l.local_addr().unwrap().port();
        let mut s = Serveur::new(charts.join("../www"), charts.to_path_buf());
        brancher(&mut s, charts, TEST);
        std::thread::spawn(move || argos_outils::serveur::servir(l, Arc::new(s)));
        port
    }

    fn req(port: u16, methode: &str, chemin: &str, corps: &[u8], entetes: &[(&str, &str)]) -> (u16, Value) {
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
        (code, serde_json::from_slice(&tout[fin + 4..]).unwrap_or(Value::Null))
    }

    /// 🔑 La clé se range, se dit présente sans jamais revenir, s'oublie ; une clé invalide est refusée ; une page
    /// d'un autre site ou une autre adresse sont refusées. Puis le relais (liste fermée, clé du trousseau, plafond)
    /// et les logos (compteur gardé).
    #[test]
    fn cle_relais_et_logos() {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../target/tmp/argos-serveur-cmc-{}", std::process::id()));
        let charts = base.join("charts");
        std::fs::create_dir_all(&charts).unwrap();
        let port = lancer(&charts);
        let page = [("Origin", &*format!("http://127.0.0.1:{port}"))];
        assert_eq!(req(port, "GET", "/api/cles/cmc", b"", &[]), (200, json!({"present": false})));
        // pas de clé : le relais le dit, rien ne part
        let (st, b) = req(port, "GET", "/api/cmc/v1/global-metrics/quotes/latest?convert=EUR", b"", &[]);
        assert_eq!(st, 401, "{b}");
        assert!(APPEL.lock().unwrap().is_none());
        // clé invalide refusée ; bonne clé rangée, jamais renvoyée
        assert_eq!(req(port, "POST", "/api/cles/cmc", br#"{"cle":"court"}"#, &page).0, 400);
        let (st, b) = req(port, "POST", "/api/cles/cmc", br#"{"cle":"  cle-de-test-1234  "}"#, &page);
        assert_eq!((st, &b), (200, &json!({"present": true})));
        assert!(!b.to_string().contains("cle-de-test"), "la valeur ne revient jamais");
        assert_eq!(CLE.lock().unwrap().as_deref(), Some("cle-de-test-1234"), "rangée sans les espaces");
        // autre site, autre adresse : refusés
        assert_eq!(
            req(port, "POST", "/api/cles/cmc", br#"{"cle":"cle-pirate-123"}"#, &[("Origin", "https://evil.com")]).0,
            403
        );
        assert_eq!(req(port, "GET", "/api/cles/cmc", b"", &[("Host", "evil.com")]).0, 403);
        assert_eq!(CLE.lock().unwrap().as_deref(), Some("cle-de-test-1234"));
        // relais : les paramètres d'Argos (pas ceux de la page), la clé du trousseau ; hors liste : 404 ; POST : 405
        let (st, _) = req(port, "GET", "/api/cmc/v1/global-metrics/quotes/latest?convert=EUR", b"", &[]);
        assert_eq!(st, 200);
        assert_eq!(
            APPEL.lock().unwrap().clone(),
            Some((cmcap::global_url(cmcap::DEFAULT_URL), "cle-de-test-1234".to_string()))
        );
        assert_eq!(req(port, "GET", "/api/cmc/v1/key/info", b"", &[]).0, 404);
        assert_eq!(req(port, "POST", "/api/cmc/v3/fear-and-greed/latest", b"{}", &page).0, 405);
        // plafond du mois atteint : refusé
        let plein = Credits {
            month: cmcap::month_of(maintenant_s()),
            used: cmcap::CREDITS_CAP,
        };
        std::fs::write(charts.join("cmc_credits.json"), serde_json::to_string(&plein).unwrap()).unwrap();
        assert_eq!(req(port, "GET", "/api/cmc/v3/fear-and-greed/latest", b"", &[]).0, 429);
        std::fs::remove_file(charts.join("cmc_credits.json")).unwrap();
        // logos : trouvés en octets ; compteur gardé ; liste illisible ou trop longue refusée
        let (st, b) = req(port, "POST", "/api/logos", br#"{"bases":["BTC","XAUT","../x"]}"#, &page);
        assert_eq!((st, b), (200, json!({"BTC": [137, 80, 78, 71]})));
        assert_eq!(lire_credits(&charts.join("cmc_credits.json")).used, 1, "compteur gardé");
        assert_eq!(req(port, "POST", "/api/logos", b"{}", &page).0, 400);
        let trop = json!({ "bases": vec!["BTC"; MAX_LOGOS + 1] }).to_string();
        assert_eq!(req(port, "POST", "/api/logos", trop.as_bytes(), &page).0, 413);
        // oublier
        assert_eq!(
            req(port, "POST", "/api/cles/cmc", br#"{"oublier":true}"#, &page),
            (200, json!({"present": false}))
        );
        assert!(CLE.lock().unwrap().is_none());
        let _ = std::fs::remove_dir_all(&base);
    }
}

#[cfg(test)]
mod tests_trousseau_muet {
    use super::*;

    /// 🧪 Un trousseau qui ne répond pas (verrouillé, démon qui redémarre) : jamais le vrai trousseau.
    const MUET: Outils = Outils {
        lire_cle: || {
            std::thread::sleep(std::time::Duration::from_secs(3));
            None
        },
        ranger_cle: |_| {
            std::thread::sleep(std::time::Duration::from_secs(3));
            Ok(())
        },
        oublier_cle: || std::thread::sleep(std::time::Duration::from_secs(3)),
        appeler: |_, _| Err("hors ligne".into()),
        logos: |_, _| vec![],
        delai: std::time::Duration::from_millis(200),
    };

    /// 🔑 Le trousseau du PC ne répond pas : ranger, oublier et la présence répondent vite une erreur 503 claire (la
    /// page l'affiche) au lieu d'attendre sans fin.
    #[test]
    fn trousseau_qui_ne_repond_pas() {
        for corps in [Some(&br#"{"cle":"cle-de-test-0000-aaaa"}"#[..]), Some(&br#"{"oublier":true}"#[..]), None] {
            let t = std::time::Instant::now();
            let r = cle(MUET, corps);
            assert!(t.elapsed() < std::time::Duration::from_secs(2), "réponse sans attendre le trousseau");
            let (code, msg) = r.expect_err("erreur");
            assert_eq!(code, 503);
            assert!(msg.contains("ne répond pas"), "{msg}");
        }
    }
}
