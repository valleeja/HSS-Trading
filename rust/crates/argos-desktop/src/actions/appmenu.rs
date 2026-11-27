//! ☰ Appli : mises à jour, installation, lien, plein écran, zoom, cache, infos.
//! Dans le navigateur (/app/), la page fait le travail (`argosApp` de app/index.html : service worker, version,
//! aide d'installation iPhone / iPad / Android, zoom gardé dans la page). Sur l'ordi : l'équivalent local (la mise à
//! jour se signale seulement si une version plus récente a été construite sur cette machine).

use crate::*;
use argos_render::topmenu::TopAct;

/// Version de ce build (`AAAAMMJJHHMM`, écrite par la construction), « dev » sinon.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))] // dans le navigateur, la page s'en charge
pub(crate) const BUILD: &str = match option_env!("ARGOS_BUILD") {
    Some(v) => v,
    None => "dev",
};
/// Commit de ce build (court), « ? » sinon.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))] // dans le navigateur, la page s'en charge
pub(crate) const COMMIT: &str = match option_env!("ARGOS_COMMIT") {
    Some(v) => v,
    None => "?",
};

/// `other` est-elle une version plus récente que `cur` ? Les versions sont des dates `AAAAMMJJHHMM` (plus grand =
/// plus récent) ; « dev » ou vide : jamais plus récent, et une version datée l'emporte sur « dev ».
#[cfg_attr(target_arch = "wasm32", allow(dead_code))] // dans le navigateur, la page s'en charge
pub(crate) fn newer(cur: &str, other: &str) -> bool {
    let date = |s: &str| (s.len() >= 12 && s[..12].bytes().all(|b| b.is_ascii_digit())).then(|| s[..12].to_string());
    match (date(cur), date(other)) {
        (Some(a), Some(b)) => b > a,
        (None, Some(_)) => true,
        _ => false,
    }
}

/// Version de `version.json` (`{"version": "…"}`).
#[cfg_attr(target_arch = "wasm32", allow(dead_code))] // dans le navigateur, la page s'en charge
pub(crate) fn version_of(raw: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(raw).ok()?["version"].as_str().map(str::to_string)
}

impl App {
    pub(crate) fn app_act(&mut self, a: TopAct) {
        #[cfg(target_arch = "wasm32")]
        {
            let name = match a {
                TopAct::Updates => "update",
                TopAct::Install => "install",
                TopAct::Desktop => "desktop",
                TopAct::CopyUrl => "copyurl",
                TopAct::Fullscreen => "fullscreen",
                TopAct::ZoomOut => "zoom-out",
                TopAct::ZoomReset => "zoom-reset",
                TopAct::ZoomIn => "zoom-in",
                TopAct::ClearCache => "clearcache",
                TopAct::Home => "home",
                TopAct::ScreenTest => "screentest",
                _ => "about",
            };
            let url = self.share_query();
            platform::web::call_page("argosApp", &[name, &url]);
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.banner = Some(match a {
                TopAct::Updates => self.local_update_note(),
                TopAct::Install => "💻 L'appli desktop est déjà installée. Sur iPhone / iPad : ouvre /app/ dans Safari → Partager → « Sur l'écran d'accueil ». Sur Android : Chrome → ⋮ → « Installer l'appli ».".into(),
                TopAct::Desktop => "💻 Tu es déjà dans la version ordi. Pour l'installer sur un autre ordi : /telecharger.html".into(),
                TopAct::CopyUrl => {
                    let url = format!("http://localhost:8000/app/{}", self.share_query());
                    let ok = self.clipboard.as_mut().is_some_and(|c| c.set_text(url.clone()).is_ok());
                    if ok {
                        format!("🔗 Lien copié : {url}")
                    } else {
                        format!("🔗 {url}")
                    }
                }
                TopAct::Fullscreen => {
                    if let Some(w) = &self.window {
                        let on = w.fullscreen().is_none();
                        w.set_fullscreen(on.then_some(winit::window::Fullscreen::Borderless(None)));
                    }
                    "🖥 Plein écran (F11 ou Échap pour revenir)".into()
                }
                TopAct::ZoomOut | TopAct::ZoomIn | TopAct::ZoomReset => {
                    "🔍 Sur l'ordi, la taille suit la mise à l'échelle de l'écran du système (le zoom est dans /app/)".into()
                }
                TopAct::ScreenTest => "📏 Le test d'écran sert à l'appli installée sur iPhone (/app/)".into(),
                TopAct::Home => "🏠 La page d'accueil est dans l'appli web (/app/) ; sur l'ordi, l'app s'ouvre sur la chart".into(),
                TopAct::ClearCache => "🧹 Rien à vider ici : le cache de pages n'existe que dans le navigateur (/app/)".into(),
                _ => format!("ℹ️ Argos HSS Trading · version {BUILD} · commit {COMMIT} · icônes Phosphor (licence MIT)"),
            });
            self.redraw();
        }
    }

    /// « ?ex=hl&s=BTC&tf=5h » : le symbole et l'unité de temps de la chart (lien à partager).
    pub(crate) fn share_query(&self) -> String {
        match &self.chart.source {
            Source::Live { exchange, base } => format!("?ex={}&s={base}&tf={}", exchange.id(), self.chart.interval.short()),
            Source::Dex { network, pool, name } => format!("?dex={network}:{pool}:{name}&tf={}", self.chart.interval.short()),
            _ => String::new(),
        }
    }

    /// 💻 Une version plus récente a-t-elle été construite sur cette machine ? (`dist/app/version.json` du dépôt,
    /// cherché depuis le dossier courant et celui du programme.)
    #[cfg(not(target_arch = "wasm32"))]
    fn local_update_note(&self) -> String {
        let mut dirs: Vec<std::path::PathBuf> = vec![];
        for start in [
            std::env::current_dir().ok(),
            std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.to_path_buf())),
        ]
        .into_iter()
        .flatten()
        {
            dirs.extend(start.ancestors().take(6).map(|d| d.to_path_buf()));
        }
        let found = dirs
            .iter()
            .find_map(|d| std::fs::read_to_string(d.join("dist/app/version.json")).ok().and_then(|raw| version_of(&raw)));
        match found {
            Some(v) if newer(BUILD, &v) => format!("⬆️ Nouvelle version locale disponible ({v}) : relance l'appli après l'avoir reconstruite"),
            Some(_) => format!("✅ Argos est à jour (version {BUILD})"),
            None => format!("✅ Version {BUILD} · aucune version plus récente trouvée sur cette machine"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logique_des_mises_a_jour() {
        assert!(newer("202611041800", "202611041815"));
        assert!(!newer("202611041815", "202611041800"), "plus ancienne");
        assert!(!newer("202611041815", "202611041815"), "la même");
        assert!(newer("dev", "202611041815"), "une version datée l'emporte sur dev");
        assert!(!newer("202611041815", "dev"));
        assert!(!newer("202611041815", ""));
        assert_eq!(version_of(r#"{"version":"202611041815","commit":"abc1234"}"#).as_deref(), Some("202611041815"));
        assert_eq!(version_of("pas du json"), None);
    }
}

#[cfg(test)]
mod tests_pwa {
    /// 📲 L'appli /app/ est installable : manifest complet (nom, icônes 192 / 512 dont « maskable », sans
    /// verrouillage d'orientation), service worker réseau d'abord avec page hors ligne, version écrite à la
    /// construction, balises Apple, et chaque action de ☰ Appli connue de la page.
    #[test]
    fn manifest_service_worker_et_page() {
        let m: serde_json::Value = serde_json::from_str(include_str!("../../../../../app/manifest.webmanifest")).unwrap();
        assert_eq!(m["name"], "Argos HSS Trading");
        assert_eq!(m["short_name"], "HSS Trading");
        assert_eq!(
            (m["start_url"].as_str(), m["scope"].as_str(), m["display"].as_str()),
            (Some("/app/"), Some("/app/"), Some("standalone"))
        );
        assert!(m.get("orientation").is_none(), "portrait et paysage : pas de verrouillage");
        let icons = m["icons"].as_array().unwrap();
        for (size, purpose) in [("192x192", "any"), ("512x512", "any"), ("192x192", "maskable"), ("512x512", "maskable")] {
            assert!(icons.iter().any(|i| i["sizes"] == size && i["purpose"] == purpose), "icône {size} {purpose}");
        }
        let sw = include_str!("../../../../../app/sw.js");
        assert!(sw.contains("\"__ARGOS_VERSION__\""), "version écrite à la construction");
        assert!(
            sw.contains("fetch(req)") && sw.contains("c.match(req"),
            "réseau d'abord, cache (de cette version) sans réseau"
        );
        assert!(
            sw.contains("OFFLINE") && sw.contains("/app/version.json"),
            "page hors ligne ; version.json jamais en cache"
        );
        let page = include_str!("../../../../../app/index.html");
        for tag in [
            "apple-mobile-web-app-capable",
            "apple-touch-icon",
            "/app/manifest.webmanifest",
            "safe-area-inset",
            "__ARGOS_VERSION__",
        ] {
            assert!(page.contains(tag), "page : {tag}");
        }
        for action in [
            "update",
            "install",
            "copyurl",
            "fullscreen",
            "zoom-out",
            "zoom-in",
            "zoom-reset",
            "clearcache",
            "screentest",
            "home",
            "desktop",
        ] {
            assert!(page.contains(&format!("case \"{action}\"")), "action ☰ Appli : {action}");
        }
        // la règle « plus récent » de la page est la même que celle de l'appli desktop
        assert!(page.contains("b > a : true"));
        // 🏠 welcome page: loaded by the page, kept offline, with its button to the chart; it never opens by itself
        // (Jee, 25 Nov 2026: « je veux enlever cette page au début »): the app opens on the chart, ☰ Appli → Page
        // d'accueil opens it (window.__landing, its one use)
        let home = include_str!("../../../../../app/accueil.js");
        assert!(page.contains("/app/accueil.js") && sw.contains("/app/accueil.js"));
        assert!(home.contains("data-lpgo") && home.contains("Commencer à charter") && home.contains("data-lpinstall"));
        assert!(!home.contains("fonts.googleapis"), "pas de service tiers");
        assert_eq!(
            home.matches("openLanding").count(),
            2,
            "defined, and given to ☰ Appli only: never called at the start"
        );
        assert!(home.contains("window.__landing = openLanding;") && page.contains("window.__landing && window.__landing()"));
        assert!(!home.contains("lpoff"), "no box « go straight to the chart next time »: it always does");
        // 💻 « Version ordi » and 📱 « Installer l'appli » (the welcome page's two buttons) are in ☰ Appli; the desktop
        // page by its file (the PC's server has no clean addresses: « /telecharger » was not found there)
        assert!(page.contains("\"/telecharger.html\"") && !home.contains("href=\"/telecharger\""));
        assert!(std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../telecharger.html")).is_file());
        for a in [super::TopAct::Install, super::TopAct::Desktop] {
            assert!(super::TopAct::from_id(a as i64).is_some(), "{a:?} in ☰ Appli");
        }
        // 📱 taille de l'appli installée sur iOS : la page charge la règle, gardée hors ligne ; ses cas sont testés
        // avec node (s'il est là)
        // la règle « hauteur = écran » est retirée : la page suit innerHeight (vraie taille de la vue sur iPhone)
        assert!(!page.contains("/app/ecran.js") && !page.contains("--ag-h"));
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../app");
        if let Ok(out) = std::process::Command::new("node").arg("ecran.test.js").current_dir(dir).output() {
            assert!(out.status.success(), "règle iOS : {}", String::from_utf8_lossy(&out.stderr));
        }
        // 🔄 session partagée : la page la charge avant la chart, hors ligne aussi ; ses garde-fous sont testés avec node
        // (s'il est là) ; ceux du serveur local par les tests d'argos-outils (make check), en dossier temporaire
        assert!(page.contains("/app/synchro.js") && sw.contains("/app/synchro.js") && page.contains("argosSynchro.charger()"));
        if let Ok(out) = std::process::Command::new("node").arg("synchro.test.js").current_dir(dir).output() {
            assert!(out.status.success(), "synchro.js : {}", String::from_utf8_lossy(&out.stderr));
        }
        // 🔍 zoom of the app with the keyboard (Ctrl / Cmd + 0, +, −): the page loads zoom.js before it is used, keeps
        // it offline, and the 3 keys are tested with node (if it is there)
        assert!(page.contains("/app/zoom.js") && sw.contains("/app/zoom.js") && page.contains("toucheZoom(e)"));
        if let Ok(out) = std::process::Command::new("node").arg("zoom.test.js").current_dir(dir).output() {
            assert!(out.status.success(), "zoom.js : {}", String::from_utf8_lossy(&out.stderr));
        }
        // 🔄 page pas démarrée (PC injoignable, pages en reconstruction) : elle réessaie toute seule et se recharge
        // quand version.json et le moteur répondent ; gardée hors ligne (essayé au navigateur sans tête, pas de node)
        let reprise = include_str!("../../../../../app/reprise.js");
        assert!(page.contains("/app/reprise.js") && sw.contains("/app/reprise.js") && page.contains("argosReprise.demarrer()"));
        assert!(reprise.contains("version.json") && reprise.contains("pkg/argos_bg.wasm") && reprise.contains("location.reload()"));
        // même recette que le site pour l'iPhone installé : pas de display_override, viewport du site, corps à 100dvh
        assert!(m.get("display_override").is_none());
        // Safari en onglet : la page retire « viewport-fit=cover » (Safari la garde hors de l'île et de sa barre)
        assert!(page.contains(r#"setAttribute("content", "width=device-width,initial-scale=1")"#));
        assert!(page.contains(r#"content="width=device-width,initial-scale=1,viewport-fit=cover""#) && page.contains("height: 100dvh"));
        // ☰ Appli : la page d'accueil et le test d'écran sont dans le menu
        assert!(super::TopAct::from_id(super::TopAct::Home as i64).is_some() && super::TopAct::from_id(super::TopAct::ScreenTest as i64).is_some());
        let v = include_str!("../../../../tools/argos-outils/src/version.rs");
        assert!(v.contains("__ARGOS_VERSION__") && v.contains("version.json"));
    }
}
