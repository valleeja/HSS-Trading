//! 🧰 argos-outils : les petits outils d'Argos en Rust pur (remplacent les anciens scripts Python).
//!
//! ```text
//! argos-outils serveur [port] [dossier]                 serveur local de l'app (port 8000, dossier dist/)
//! argos-outils version-app [dossier dist]               version de /app/ (version.json, index.html, sw.js)
//! argos-outils langue modele|verifier [--racine R]      packs de langue (lang/*.json)
//! argos-outils emoji <dossier svg> [--racine R]         émojis du code → assets/twemoji + twemoji.rs
//! argos-outils emoji-tout <dossier svg> [emoji-test.txt] --noms <UnicodeData.txt | Name.pl> [--racine R]
//! argos-outils maquette <vue> [--racine R]              ecrans/maquette-<vue>.json → maquette_<vue>.rs
//! argos-outils v1-inputs [--racine R]                   app.js (v1) → argos-script/src/v1_inputs.rs (node)
//! argos-outils assemble [--check|--test] [--racine R]   src/* (site v1) → index.html + app.js
//! argos-outils local [--wifi [--nouveau-code]] [--racine R]  site v1 en local (port 8787, relais /api)
//! argos-outils publier-capture <fichier.png> [nom.png] [adresse…] [--racine R]  capture servie sur /captures/, vérifiée
//! ```
//! `--racine` : le dossier `rust/` du dépôt (par défaut : le dossier courant).

use argos_outils::{assemble, capture, emoji, emoji_tout, langue, lire_args, local, maquette, serveur, v1_inputs, version};
use std::process::ExitCode;

const AIDE: &str = "argos-outils <serveur|local|assemble|version-app|langue|emoji|emoji-tout|maquette|v1-inputs|publier-capture> … (voir README.md)";

fn main() -> ExitCode {
    let mut brut: Vec<String> = std::env::args().skip(1).collect();
    if brut.is_empty() || brut[0] == "-h" || brut[0] == "--help" {
        println!("{AIDE}");
        return ExitCode::SUCCESS;
    }
    let cmd = brut.remove(0);
    let r = lire_args(brut).and_then(|a| match cmd.as_str() {
        "serveur" => serveur::main(&a, |_| ()),
        "version-app" => version::main(&a),
        "langue" => langue::main(&a),
        "emoji" => emoji::main(&a),
        "emoji-tout" => emoji_tout::main(&a),
        "maquette" => maquette::main(&a),
        "v1-inputs" => v1_inputs::main(&a),
        "assemble" => assemble::main(&a),
        "local" => local::main(&a),
        "publier-capture" => capture::main(&a),
        _ => Err(format!("commande inconnue « {cmd} ». {AIDE}")),
    });
    match r {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("❌ {e}");
            ExitCode::FAILURE
        }
    }
}
