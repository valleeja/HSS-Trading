//! 🌐 Packs de langue d'Argos : `lang/<code>.json` du site (phrase française → traduction).
//! Téléchargés au clic sur une langue, puis gardés dans le dossier de config (`lang/<code>.json`).

/// Où sont publiés les packs (le site d'Argos) ; `ARGOS_LANG_URL` le remplace (tests, autre hébergement).
const BASE: &str = "https://argos-smoky.vercel.app/lang";

/// Adresse du pack `code`.
pub fn url(code: &str) -> String {
    let base = std::env::var("ARGOS_LANG_URL").unwrap_or_else(|_| BASE.to_string());
    format!("{}/{code}.json", base.trim_end_matches('/'))
}

/// Télécharge le pack `code` (desktop ou navigateur, en arrière-plan avec `rt::spawn`).
#[cfg(any(feature = "native", feature = "web"))]
pub async fn fetch(code: &str) -> argos_core::Result<String> {
    use crate::http::Http;
    crate::rt::Client::default().get(&url(code)).await
}
