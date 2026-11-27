//! 🔒 Jamais de clé d'API ni de jeton dans les journaux : chaque ligne qui écrit dans le terminal (`println!`,
//! `eprintln!`, `print!`, `eprint!`, `dbg!`) de tout le code Rust d'Argos (crates, coque Tauri, outils) est lue ; une
//! ligne qui nomme une clé, un jeton, un secret, un mot de passe ou les 24 mots fait échouer le test.
//! (Le cache, lui, ne contient que des chiffres : voir le test `aucun_texte_libre_dans_le_cache` de `cache.rs`.)

use std::path::Path;

/// Mots interdits dans une ligne de journal (en minuscules).
const INTERDITS: [&str; 14] = [
    "token",
    "jeton",
    "secret",
    "passphrase",
    "password",
    "mot de passe",
    "mnemonic",
    "mots",
    "api_key",
    "apikey",
    "x-api-key",
    "private",
    "prive",
    "key",
];

/// Lignes connues qui ne montrent aucun secret : « key » = nom d'un réglage ou d'une case ; l'essai des bougies dit
/// seulement qu'il n'a pas de jeton OANDA (texte fixe).
const PERMIS: [(&str, &str); 3] = [
    ("inputs.rs", "input inconnu : {key}"),
    ("events.rs", "Case {key} : {e}"),
    ("essai_bougies.rs", "sans jeton, ignoré (l'essai ne lit jamais les clés)"),
];

fn fichiers(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        let nom = p.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_string();
        if p.is_dir() {
            if !["target", "vendor", "gen", "node_modules"].contains(&nom.as_str()) {
                fichiers(&p, out);
            }
        } else if nom.ends_with(".rs") {
            out.push(p);
        }
    }
}

#[test]
fn aucune_cle_ni_jeton_dans_les_journaux() {
    let rust = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files = vec![];
    for d in ["crates", "argos-tauri/src", "tools", "modules"] {
        fichiers(&rust.join(d), &mut files);
    }
    assert!(files.len() > 50, "le code d'Argos est bien lu ({} fichiers)", files.len());
    let mut fautes = vec![];
    for f in &files {
        let nom = f.file_name().and_then(|n| n.to_str()).unwrap_or_default();
        if nom == "aucun_secret_dans_les_journaux.rs" {
            continue;
        }
        let text = std::fs::read_to_string(f).unwrap_or_default();
        for (i, l) in text.lines().enumerate() {
            let t = l.trim_start();
            if t.starts_with("//") || !["println!", "eprintln!", "print!(", "eprint!(", "dbg!("].iter().any(|m| t.contains(m)) {
                continue;
            }
            if PERMIS.iter().any(|(fic, extrait)| nom == *fic && l.contains(extrait)) {
                continue;
            }
            let low = l.to_lowercase();
            if let Some(m) = INTERDITS.iter().find(|m| low.contains(*m)) {
                fautes.push(format!("{}:{} « {m} » : {}", f.display(), i + 1, l.trim()));
            }
        }
    }
    assert!(
        fautes.is_empty(),
        "lignes de journal qui parlent d'une clé ou d'un jeton :\n{}",
        fautes.join("\n")
    );
}
