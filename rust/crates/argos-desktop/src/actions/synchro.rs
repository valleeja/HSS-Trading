//! 🔄 Synchro en direct (étape 2 de synchro-direct) : un réglage changé sur un autre appareil (page web, téléphone,
//! app de bureau) est relu tout de suite et l'affichage suit, sans redémarrer.
//! - App de bureau : un fil regarde le dossier des réglages toutes les 2 s (`surveiller`) ; ses propres écritures
//!   sont reconnues (`platform::ecrits`) et ignorées.
//! - Page web : `app/synchro.js` reçoit les changements du serveur local et réveille l'app
//!   (`argosSynchroReveil`).
//! - Le dernier marché ouvert (`session.json` : exchange, symbole, unité de temps) se rouvre au démarrage et suit en
//!   direct.

use crate::*;

/// Le dernier marché ouvert, partagé entre les appareils.
pub(crate) const SESSION: &str = "session.json";

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub(crate) struct Derniere {
    /// Identifiant de l'exchange (`hl`, `binance`…).
    pub(crate) ex: String,
    pub(crate) base: String,
    /// Unité de temps (secondes).
    pub(crate) interval: i64,
}

impl Derniere {
    /// Le marché noté, s'il est lisible.
    pub(crate) fn marche(&self) -> Option<(Exchange, String, Option<Interval>)> {
        let ex = Exchange::parse(&self.ex)?;
        let base = self.base.trim();
        if base.is_empty() || base.len() > 40 || !base.chars().all(|c| c.is_ascii_alphanumeric() || "-_:./".contains(c)) {
            return None;
        }
        Some((ex, base.to_string(), (self.interval > 0).then_some(Interval(self.interval))))
    }
}

/// Le dernier marché ouvert (au démarrage, quand rien d'autre n'est demandé).
pub(crate) fn derniere() -> Option<(Exchange, String, Option<Interval>)> {
    App::read_prefs::<Derniere>(SESSION).marche()
}

impl App {
    /// Note le marché ouvert (seulement s'il a changé : pas d'écriture inutile, pas de boucle).
    pub(crate) fn noter_marche(&self) {
        let Source::Live { exchange, base } = &self.chart.source else { return };
        let d = Derniere {
            ex: exchange.id().into(),
            base: base.clone(),
            interval: self.chart.interval.secs(),
        };
        if Self::read_prefs::<Derniere>(SESSION) != d {
            Self::write_prefs(SESSION, &d);
        }
    }

    /// 🔄 Des réglages ont changé ailleurs : les relire et mettre l'affichage à jour. Un nom inconnu est ignoré.
    pub(crate) fn recharger(&mut self, noms: &[String]) {
        let mut rebuild = false;
        for nom in noms {
            match nom.as_str() {
                SESSION => {
                    if let Some((ex, base, iv)) = derniere() {
                        let iv = iv.unwrap_or(self.chart.interval);
                        let meme = matches!(&self.chart.source, Source::Live { exchange, base: b } if *exchange == ex && *b == base);
                        if !meme || iv != self.chart.interval {
                            self.save();
                            self.write_cache();
                            self.chart.interval = iv;
                            self.tools.interval = iv.secs();
                            self.open_live(ex, base);
                        }
                    }
                }
                "toolbar.json" => {
                    let iv = self.tools.interval;
                    self.tools = Self::read_prefs("toolbar.json");
                    self.tools.interval = iv; // l'unité de temps suit `session.json`
                    self.passer_au_cadre();
                }
                "watchlist.json" => self.watch.prefs = Self::read_prefs(nom),
                "dock.json" => {
                    self.watch.dock = Self::read_prefs(nom);
                    self.watch.dock.fix();
                }
                "topbar.json" => {
                    let vue = self.top.prefs.bienvenue_vue;
                    self.top.prefs = Self::read_prefs(nom);
                    // 👋 (B-042) the first-launch screens seen: this app's own (from interface-2.json), never the top
                    // bar file's (an older version writes its own flag there; ⌂ then showed the 3 screens again)
                    self.top.prefs.bienvenue_vue = vue;
                    self.passer_au_cadre();
                }
                "rail.json" => {
                    self.rail = Self::read_prefs(nom);
                    self.passer_au_cadre();
                }
                "split.json" => {
                    self.split = Self::read_prefs(nom);
                    self.panes.clear();
                    self.sync_panes();
                }
                "demo_account.json" => self.demo = Self::read_prefs(nom),
                "alerts.json" => self.alerts = Self::read_prefs(nom),
                "indicator_legend.json" => self.ind.leg = Self::read_prefs(nom),
                "indicators.json" => (self.ind.on, rebuild) = (Self::read_prefs(nom), true),
                "market_fav.json" => self.ind.favs = Self::read_prefs(nom),
                // (📚 by name: the source; the old by-position file is only written for /app/, never read back)
                "indicator_settings.json" => {
                    (self.ind.set, self.ind.scripts_set) = Self::lire_reglages();
                    rebuild = true;
                }
                "indicator_texts.json" => self.ind.texts = Self::read_prefs(nom),
                "templates.json" => self.templates = Self::read_prefs(nom),
                "tool_styles.json" => self.tool_styles = Self::read_prefs(nom),
                "scripts.json" => {
                    self.scripts = Self::read_prefs(nom);
                    self.ids_des_scripts();
                }
                crate::actions::layouts::FICHIER => self.layouts_recus(),
                // ⭐ les listes : remplacées par celles de l'autre appareil
                "watchlists.json" => {
                    let raw = platform::read(nom).unwrap_or_default();
                    if let (Some(st), Some(l)) = (self.store.as_mut(), argos_store::listes::lire(&raw)) {
                        argos_store::listes::appliquer(st, &l, true);
                    }
                    self.load_lists();
                    self.refresh_watch();
                }
                _ if self.chart.save_path.as_deref() == Some(nom.as_str()) => {
                    // la sauvegarde du symbole ouvert : type, zoom, réglages, dessins
                    *self.chart.last_saved.borrow_mut() = None;
                    self.restore_session();
                    rebuild = true;
                }
                _ => {}
            }
        }
        if rebuild {
            self.rebuild();
        }
        self.redraw();
    }
}

/// 👀 App de bureau : regarde le dossier des réglages toutes les 2 s ; rend les fichiers changés par quelqu'un
/// d'autre (pas ceux que cette app vient d'écrire) à `envoyer` (qui rend `false` quand la fenêtre est fermée).
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn surveiller(envoyer: impl Fn(Vec<String>) -> bool + Send + 'static) {
    let Some(dir) = crate::persist::saved::dir() else { return };
    std::thread::spawn(move || {
        let mut vus = instantane(&dir);
        loop {
            std::thread::sleep(std::time::Duration::from_secs(2));
            let now = instantane(&dir);
            let noms = changes(&vus, &now, |nom| {
                std::fs::read_to_string(dir.join(nom)).is_ok_and(|t| !platform::ecrits::est_le_mien(nom, &t))
            });
            vus = now;
            if !noms.is_empty() && !envoyer(noms) {
                return;
            }
        }
    });
}

/// Date et taille de chaque réglage du dossier.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn instantane(dir: &std::path::Path) -> std::collections::BTreeMap<String, (std::time::SystemTime, u64)> {
    std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let nom = e.file_name().into_string().ok()?;
            let m = e.metadata().ok()?;
            let t = m.modified().ok()?;
            (nom.ends_with(".json") && m.is_file()).then_some((nom, (t, m.len())))
        })
        .collect()
}

/// Les réglages qui ont changé entre deux instantanés (et que `autre` attribue à quelqu'un d'autre).
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub(crate) fn changes<V: PartialEq>(
    avant: &std::collections::BTreeMap<String, V>,
    apres: &std::collections::BTreeMap<String, V>,
    autre: impl Fn(&str) -> bool,
) -> Vec<String> {
    apres
        .iter()
        .filter(|(nom, v)| avant.get(*nom) != Some(*v))
        .map(|(nom, _)| nom.clone())
        .filter(|nom| autre(nom))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn dernier_marche_lu_avec_prudence() {
        let d = Derniere {
            ex: "binance".into(),
            base: "ETH".into(),
            interval: 900,
        };
        assert_eq!(d.marche(), Some((Exchange::Binance, "ETH".into(), Some(Interval(900)))));
        let hip3 = Derniere {
            ex: "hl".into(),
            base: "xyz:GOLD".into(),
            interval: 0,
        };
        assert_eq!(hip3.marche(), Some((Exchange::Hyperliquid, "xyz:GOLD".into(), None)));
        for (ex, base) in [("inconnu", "BTC"), ("hl", ""), ("hl", "BTC<script>"), ("hl", &"A".repeat(60))] {
            let d = Derniere {
                ex: ex.into(),
                base: base.into(),
                interval: 60,
            };
            assert_eq!(d.marche(), None, "{ex} {base}");
        }
    }

    #[test]
    fn seulement_ce_qui_a_change_ailleurs() {
        let avant: BTreeMap<String, u32> = [("a.json".into(), 1), ("b.json".into(), 1)].into();
        let apres: BTreeMap<String, u32> = [("a.json".into(), 2), ("b.json".into(), 1), ("c.json".into(), 1)].into();
        assert_eq!(changes(&avant, &apres, |_| true), ["a.json", "c.json"]);
        // ce que l'app vient d'écrire elle-même : ignoré (pas de boucle)
        assert_eq!(changes(&avant, &apres, |n| n != "a.json"), ["c.json"]);
        assert!(changes(&apres, &apres, |_| true).is_empty());
    }

    /// 💾 « Sauvegarde… » juste après une écriture, puis « ✓ Sauvegardé ».
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn indicateur_de_sauvegarde() {
        platform::write("synchro-indic.json", "{}").unwrap();
        assert!(platform::sauvegarde_en_cours());
        assert_eq!(argos_render::topbar::save_label(true), "Sauvegarde…");
        assert_eq!(argos_render::topbar::save_label(false), "✓ Sauvegardé");
        let _ = std::fs::remove_file(crate::persist::saved::dir().unwrap().join("synchro-indic.json"));
    }

    /// 👀 Le fil de l'app de bureau voit un fichier écrit par quelqu'un d'autre, pas ceux qu'elle écrit elle-même.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn ses_propres_ecritures_sont_ignorees() {
        let dir = crate::persist::saved::dir().unwrap();
        std::fs::create_dir_all(&dir).unwrap();
        platform::write("synchro-moi.json", "{\"a\":1}").unwrap();
        std::fs::write(dir.join("synchro-autre.json"), "{\"b\":2}").unwrap();
        let avant = BTreeMap::new();
        let apres = instantane(&dir);
        let noms = changes(&avant, &apres, |nom| {
            std::fs::read_to_string(dir.join(nom)).is_ok_and(|t| !platform::ecrits::est_le_mien(nom, &t))
        });
        assert!(
            noms.contains(&"synchro-autre.json".to_string()) && !noms.contains(&"synchro-moi.json".to_string()),
            "{noms:?}"
        );
        for n in ["synchro-moi.json", "synchro-autre.json"] {
            let _ = std::fs::remove_file(dir.join(n));
        }
    }
}
