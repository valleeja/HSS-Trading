//! (temporaire, jamais commité) serveur d'essai de la branche support-1 : 127.0.0.1 seulement.
#[path = "../src/retours.rs"]
#[allow(dead_code)]
mod retours;

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let port: u16 = a[0].parse().unwrap();
    let racine = std::path::PathBuf::from(&a[1]);
    let l = std::net::TcpListener::bind(("127.0.0.1", port)).unwrap();
    let mut s = argos_outils::serveur::Serveur::new(racine.clone(), argos_outils::serveur::reglages_dossier());
    s.accueil = Some("/app/".into());
    retours::brancher(&mut s, retours::dossier(&racine));
    println!("essai sur 127.0.0.1:{port}");
    argos_outils::serveur::servir(l, std::sync::Arc::new(s));
}
