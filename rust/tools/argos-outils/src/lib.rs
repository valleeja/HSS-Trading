//! 🧰 argos-outils : les petits outils d'Argos en Rust pur (remplacent les anciens scripts Python). Le programme
//! `argos-outils` (main.rs) les lance ; un autre programme peut aussi réutiliser le serveur local comme bibliothèque
//! et y ajouter ses propres routes (`serveur::Route`).

pub mod assemble;
pub mod capture;
pub mod emoji;
pub mod emoji_tout;
pub mod json;
pub mod langue;
pub mod local;
pub mod maquette;
pub mod serveur;
pub mod v1_inputs;
pub mod version;

use std::path::PathBuf;

/// Arguments : les positionnels, et la valeur de `--racine` / `--noms`.
pub struct Args {
    pub pos: Vec<String>,
    pub racine: PathBuf,
    pub noms: Option<PathBuf>,
}

pub fn lire_args(brut: Vec<String>) -> Result<Args, String> {
    let mut a = Args {
        pos: vec![],
        racine: PathBuf::from("."),
        noms: None,
    };
    let mut it = brut.into_iter();
    while let Some(x) = it.next() {
        match x.as_str() {
            "--racine" => a.racine = it.next().ok_or("--racine sans dossier")?.into(),
            "--noms" => a.noms = Some(it.next().ok_or("--noms sans fichier")?.into()),
            _ => a.pos.push(x),
        }
    }
    Ok(a)
}
