//! Sauvegarde de la chart entre deux lancements, **par symbole** : type de chart, zoom, réglages (`ChartSettings`) et
//! objets dessinés. Fichier JSON dans `~/.config/argos-hss-trading/charts/` (ou
//! `$XDG_CONFIG_HOME/argos-hss-trading/charts/`), ou le stockage de
//! la page sur le web (`platform`). Écriture sûre : fichier temporaire puis renommage.

use argos_render::drawings::Drawing;
use argos_render::settings::ChartSettings;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Ce qui est gardé. Un champ ajouté plus tard prend sa valeur par défaut dans les vieux fichiers.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Saved {
    /// Nom du type de chart (« Bougies de volume Argos », « Heikin Ashi »…).
    pub style: String,
    /// Zoom : espace entre deux bougies, en pixels.
    pub bar_spacing: f64,
    pub settings: ChartSettings,
    pub drawings: Vec<Drawing>,
}

/// Dossier des sauvegardes, propre à Argos HSS Trading : `$XDG_CONFIG_HOME/argos-hss-trading/charts`, sinon
/// `~/.config/argos-hss-trading/charts` (l'ancien `~/.config/argos/charts` y est copié au premier lancement :
/// [`preparer`]).
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub fn dir() -> Option<PathBuf> {
    // tests : un dossier de test à eux (rust/target/tmp), pour ne jamais toucher à tes vrais réglages
    if cfg!(test) {
        // (un test peut avoir son propre dossier : aucun autre test n'écrit dans ses fichiers en même temps)
        #[cfg(test)]
        if let Some(d) = DOSSIER_TEST.with(|d| d.borrow().clone()) {
            return Some(d);
        }
        return Some(argos_core::dossiers::de_test("reglages"));
    }
    argos_core::dossiers::reglages()
}

#[cfg(test)]
thread_local! {
    /// 🧪 Dossier de réglages propre au test en cours (`None` : le dossier commun des tests).
    pub static DOSSIER_TEST: std::cell::RefCell<Option<PathBuf>> = const { std::cell::RefCell::new(None) };
}

/// 📁 Premier lancement : l'ancien dossier des réglages (`~/.config/argos/charts`, commun avec d'autres projets Argos)
/// est copié dans celui d'Argos HSS Trading, sans l'ancien journal en clair ; l'ancien n'est jamais modifié.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub fn preparer() {
    if cfg!(test) {
        return;
    }
    if let (Some(n), Some(a)) = (argos_core::dossiers::reglages(), argos_core::dossiers::reglages_ancien()) {
        if let Err(e) = argos_core::dossiers::preparer(&n, &a) {
            eprintln!("⚠️ copie des réglages vers {} : {e}", n.display());
        }
    }
}

/// Nom de fichier sûr pour une clé (« hyperliquid-btc » → « hyperliquid-btc.json »).
pub fn file_name(key: &str) -> String {
    let clean: String = key
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    format!("{}.json", clean.trim_matches('-'))
}

/// Lit la sauvegarde `name` (`None` si absente ou illisible : on repart des valeurs par défaut).
pub fn read(name: &str) -> Option<Saved> {
    serde_json::from_str(&crate::platform::read(name)?).ok()
}

/// Écrit la sauvegarde `name`.
pub fn write(name: &str, s: &Saved) -> Result<(), String> {
    crate::platform::write(name, &serde_json::to_string_pretty(s).map_err(|e| e.to_string())?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use argos_render::settings::ScaleMode;

    #[test]
    fn aller_retour_et_fichiers_abimes() {
        // pendant les tests, `dir()` est un dossier de test dans rust/target/tmp (jamais ~/.config)
        assert!(dir().unwrap().starts_with(argos_core::dossiers::racine_des_tests()));
        let p = file_name("Hyperliquid BTC/USDT");
        assert_eq!(p, "hyperliquid-btc-usdt.json");
        let file = dir().unwrap().join(&p);
        let mut s = Saved {
            style: "Heikin Ashi".into(),
            bar_spacing: 12.5,
            drawings: vec![Drawing::hline(80123.4)],
            ..Default::default()
        };
        s.settings.scale.mode = ScaleMode::Log;
        write(&p, &s).unwrap();
        assert_eq!(read(&p), Some(s));
        // fichier abîmé ou absent : rien (valeurs par défaut au lancement)
        std::fs::write(&file, "{pas du json").unwrap();
        assert_eq!(read(&p), None);
        assert_eq!(read("absent.json"), None);
        // vieux fichier sans certains champs : le reste par défaut
        std::fs::write(&file, r#"{"style":"Ligne"}"#).unwrap();
        let old = read(&p).unwrap();
        assert_eq!((old.style.as_str(), old.drawings.len()), ("Ligne", 0));
        std::fs::remove_file(file).ok();
    }
}
