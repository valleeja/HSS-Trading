//! 🔐 Le journal chiffré vu de l'app de bureau : où est la base sensible, avec quelle clé, et l'ancien fichier en clair
//! à migrer. La logique (lire, écrire entrée par entrée, migrer, ouvrir le temps d'une opération) est celle
//! d'`argos_store::journal`, partagée avec le serveur local qui sert le cell. Dans les tests : une clé fixe et un
//! dossier dans le dossier de compilation du projet, jamais le vrai trousseau ni ~/.local/share.

use argos_core::journal::{Changements, Donnees};
use argos_store::journal as j;
use std::path::{Path, PathBuf};

/// La base sensible.
pub(crate) fn chemin() -> Option<PathBuf> {
    if cfg!(test) {
        Some(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../target/tmp")
                .join(format!("journal-app-{}", std::process::id()))
                .join("sensible.redb"),
        )
    } else {
        argos_store::sensible::chemin()
    }
}

/// La clé de l'appareil (tests : une clé fixe).
fn cle() -> Result<argos_store::sensible::Cle, String> {
    if cfg!(test) {
        Ok([42; 32])
    } else {
        j::cle_du_trousseau()
    }
}

/// Le journal de la base `p` (l'ancien `journal.json` en clair du dossier des réglages y est migré une fois).
pub(crate) fn charger(p: &Path) -> Result<Donnees, String> {
    // l'ancien journal en clair reste dans l'ancien dossier des réglages (`~/.config/argos/charts`) : il n'est jamais
    // recopié dans celui d'Argos HSS Trading, sa migration le lit là
    let dossier = if cfg!(test) {
        crate::persist::saved::dir()
    } else {
        argos_core::dossiers::reglages_ancien()
    };
    let ancien = dossier.map(|d| d.join(j::ANCIEN));
    j::un_moment(p, &cle()?, |b| {
        if let Some(a) = &ancien {
            j::migrer(b, a)?;
        }
        j::lire(b)
    })
}

/// Écrit des changements (entrée par entrée) ; rend le journal de la base.
pub(crate) fn ecrire(p: &Path, ch: &Changements) -> Result<Donnees, String> {
    j::un_moment(p, &cle()?, |b| j::appliquer(b, ch))
}
