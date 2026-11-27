//! Les commandes de l'app sont déclarées ici : Tauri ne les accepte que si une capacité les permet
//! (`capabilities/default.json` → `allow-…` par commande). Refus par défaut pour tout le reste.
fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(tauri_build::AppManifest::new().commands(&[
        "module_ema",
        "reseau",
        "module_lancer",
        "direct_ouvrir",
        "direct_fermer",
        "reglages_tout",
        "reglage_ecrire",
        "reglages_changements",
    ])))
    .expect("configuration Tauri");
}
