//! 🔒 La page de la coque n'atteint jamais le réseau elle-même : tout passe par le cœur Rust (couche de données
//! d'Argos). Vérifié sur la configuration livrée, sans lancer de fenêtre :
//! - CSP stricte : `connect-src` seulement la page elle-même et l'IPC de Tauri (aucun http(s)://, ws(s)://, *) ;
//! - aucun plugin réseau (http, websocket, shell, opener, upload) dans les dépendances ;
//! - les capacités ne donnent aucune permission de plugin réseau ;
//! - Tauri sans ses options par défaut ;
//! - T1 : la chart complète vient des fichiers de la coque (jamais d'une adresse) ; seulement les commandes prévues.

const CONF: &str = include_str!("../tauri.conf.json");
const CARGO: &str = include_str!("../Cargo.toml");
const CAPS: &str = include_str!("../capabilities/default.json");

fn csp() -> Vec<(String, Vec<String>)> {
    let v: serde_json::Value = serde_json::from_str(CONF).expect("tauri.conf.json");
    let csp = v["app"]["security"]["csp"].as_str().expect("une CSP est définie");
    csp.split(';')
        .filter_map(|d| {
            let mut w = d.split_whitespace();
            Some((w.next()?.to_string(), w.map(str::to_string).collect()))
        })
        .collect()
}

#[test]
fn la_page_n_a_aucun_acces_reseau() {
    let csp = csp();
    let get = |k: &str| csp.iter().find(|(n, _)| n == k).map(|(_, v)| v.clone());
    let default = get("default-src").expect("default-src");
    assert_eq!(default, ["'self'"], "par défaut : rien d'autre que la page elle-même");
    let connect = get("connect-src").expect("connect-src écrit en clair");
    for src in &connect {
        assert!(
            ["'self'", "ipc:", "http://ipc.localhost"].contains(&src.as_str()),
            "connect-src ne doit viser que la page et l'IPC du cœur, pas « {src} »"
        );
    }
    for (name, srcs) in &csp {
        for s in srcs {
            let low = s.to_lowercase();
            let ipc = low == "http://ipc.localhost" || low == "ipc:";
            assert!(
                ipc || !(low.starts_with("http") || low.starts_with("ws") || low == "*" || low == "https:" || low == "wss:"),
                "{name} ouvre le réseau : « {s} »"
            );
        }
    }
}

#[test]
fn aucun_plugin_reseau_ni_option_par_defaut() {
    for bad in [
        "tauri-plugin-http",
        "tauri-plugin-websocket",
        "tauri-plugin-shell",
        "tauri-plugin-opener",
        "tauri-plugin-upload",
        "reqwest",
    ] {
        assert!(!CARGO.contains(bad), "dépendance réseau interdite dans la coque : {bad}");
    }
    assert!(
        CARGO.contains("tauri = { version = \"2\", default-features = false"),
        "Tauri sans ses options par défaut"
    );
    let caps: serde_json::Value = serde_json::from_str(CAPS).unwrap();
    for p in caps["permissions"].as_array().unwrap() {
        let p = p.as_str().unwrap_or_default();
        assert!(
            !(p.starts_with("http:") || p.starts_with("websocket:") || p.starts_with("shell:") || p.starts_with("opener:")),
            "permission réseau dans les capacités : {p}"
        );
    }
}

#[test]
fn la_chart_vient_des_fichiers_de_la_coque() {
    let v: serde_json::Value = serde_json::from_str(CONF).unwrap();
    assert_eq!(v["build"]["frontendDist"], "./web", "pages copiées dans la coque, pas une adresse");
    assert!(v["build"].get("devUrl").is_none(), "aucune adresse de développement");
    for w in v["app"]["windows"].as_array().unwrap() {
        let url = w["url"].as_str().unwrap_or("index.html");
        assert!(!url.contains(':') && !url.starts_with("//"), "fenêtre sur une page de la coque, pas « {url} »");
    }
    // Tauri ajoute ses empreintes aux scripts : 'unsafe-inline' et 'unsafe-eval' jamais pour les scripts
    let csp = csp();
    let script = &csp.iter().find(|(n, _)| n == "script-src").unwrap().1;
    assert!(!script.iter().any(|s| s == "'unsafe-inline'" || s == "'unsafe-eval'"), "{script:?}");
}

#[test]
fn seulement_les_commandes_prevues() {
    let caps: serde_json::Value = serde_json::from_str(CAPS).unwrap();
    let perms: Vec<&str> = caps["permissions"].as_array().unwrap().iter().filter_map(|p| p.as_str()).collect();
    assert_eq!(
        perms,
        [
            "core:event:default",
            "allow-module-ema",
            "allow-reseau",
            "allow-module-lancer",
            "allow-direct-ouvrir",
            "allow-direct-fermer",
            "allow-reglages-tout",
            "allow-reglage-ecrire",
            "allow-reglages-changements"
        ]
    );
}
