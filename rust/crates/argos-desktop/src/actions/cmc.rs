//! 📈 CoinMarketCap, seulement avec la clé de l'utilisateur (🔑 Clés API) : relevé une fois par jour par défaut
//! (réglable), ⟳ manuel, compteur de crédits du mois. Il s'ajoute alors aux sources gratuites (marché global, USDC.D,
//! peur & avidité, saison des altcoins), sans rien en retirer ni remplacer (règle de Jee).
//! Sans clé : tout vient des sources gratuites (CoinGecko, alternative.me) ; les séries `CRYPTOCAP:USDT.D`,
//! `CRYPTOCAP:TOTAL3`… se remplissent avec le marché de CoinGecko (un point toutes les 5 min au plus).
//! 🌐 Navigateur (PC et cell) : la clé tapée part une seule fois au serveur du PC (`/api/cles/cmc`), qui la range dans
//! son trousseau ; la page n'en garde rien (`session.cmc` y vaut une chaîne vide = « clé au PC ») et passe par le
//! relais du serveur (`cmcap::RELAIS`).

use crate::*;
use argos_data::cmcap;

/// CoinMarketCap : gratuit par défaut (CoinGecko, alternative.me, GeckoTerminal), et seulement si tu ajoutes ta clé
/// (bouton 🔑 Clé API du panneau Marché, ou 🔑 Clés API). `true` le couperait complètement.
pub(crate) const CMC_OFF: bool = false;

/// 🌐 Route du serveur du PC pour la clé CoinMarketCap (navigateur).
#[cfg(target_arch = "wasm32")]
const CLE_SERVEUR: &str = "/api/cles/cmc";

/// Message affiché tant qu'il n'y a pas de clé.
pub(crate) const NO_KEY: &str = "📈 Ajoute ta clé CoinMarketCap dans 🔑 Clés API (☰ Appli ou Réglages) ▸ CoinMarketCap";

impl App {
    /// Règle le fil de fond selon la clé et le lien (`None` sans clé : il attend).
    pub(crate) fn cmc_apply(&mut self) {
        let base = self.cmc_base().unwrap_or_else(|_| cmcap::DEFAULT_URL.into());
        // 🦎 chandelles DEX et 🪙 logos de CoinMarketCap : seulement avec la clé (à la demande, comptés dans le compteur)
        cmcap::regler(self.session.cmc.clone().map(|k| (base.clone(), k, self.feeds.cmc_credits.clone())));
        if let Ok(mut w) = self.feeds.cmc_wanted.lock() {
            *w = self.session.cmc.clone().map(|k| (base, k));
        }
    }

    /// 📈 Avec la clé : le dernier relevé CoinMarketCap (moins de 2 intervalles) s'ajoute au panneau, à côté des
    /// sources gratuites qui restent telles quelles. Seule la présence de la clé est notée, jamais la clé.
    pub(crate) fn cmc_over(&self, m: &mut argos_data::market::MarketInfo) {
        m.cmc_cle = self.session.cmc.is_some();
        let every = self.feeds.cmc_ctl.every_s.load(Ordering::SeqCst);
        m.cmc = self.feeds.cmc_last.clone().filter(|p| m.cmc_cle && now_s() - p.t <= 2 * every);
    }

    /// 🌐 With the key: the market caps of the last CoinMarketCap reading (less than 2 intervals old, like the panel).
    pub(crate) fn cmc_caps(&self) -> Option<&std::collections::HashMap<String, f64>> {
        let every = self.feeds.cmc_ctl.every_s.load(Ordering::SeqCst);
        self.feeds
            .cmc_last
            .as_ref()
            .filter(|p| self.session.cmc.is_some() && now_s() - p.t <= 2 * every && !p.caps.is_empty())
            .map(|p| &p.caps)
    }

    /// Série CRYPTOCAP: ouverte : ses bougies refaites avec l'historique.
    pub(crate) fn cap_refresh(&mut self) {
        if let Source::Cap { series } = &self.chart.source {
            self.chart.raw = cmcap::bars(&self.feeds.cmc_hist, series);
            self.rebuild();
        }
    }

    /// 🧮 Garde le compteur de crédits du mois.
    pub(crate) fn save_credits(&self) {
        if let Ok(c) = self.feeds.cmc_credits.lock().map(|c| *c) {
            Self::write_prefs("cmc_credits.json", &c);
        }
    }

    /// Ligne CoinMarketCap du panneau 🔌 API.
    pub(crate) fn cmc_row(&self) -> ExRow {
        ExRow {
            id: "cmc".into(),
            name: "CoinMarketCap (BTC.D, USDT.D, TOTAL3…)".into(),
            enabled: self.session.cmc.is_some(),
            symbols: cmcap::SERIES.len(),
            age_s: self.feeds.cmc_hist.last().map(|p| now_s() - p.t),
            has_key: self.session.cmc.is_some(),
            loading: false,
            note: match self.session.cmc {
                Some(_) => format!(
                    "{} · relevé toutes les {} h",
                    self.feeds.cmc_credits.lock().map(|c| c.label()).unwrap_or_default(),
                    self.ex_prefs.cmc_every_h.clamp(1, 168)
                ),
                None => "facultatif · sans clé : CoinGecko et alternative.me (gratuits)".into(),
            },
        }
    }

    /// Boîte de la clé : la clé (masquée) et le lien (prérempli, modifiable).
    pub(crate) fn cmc_key_box(&mut self) {
        let (w, h) = (self.win_w(), self.area.1 as f64 + self.top_h());
        let mut b = KeyBox::new("cmc", "CoinMarketCap", w, h);
        if let Some(f) = b.fields.iter_mut().find(|f| f.id == "url") {
            f.value = cmcap::base_url(&self.ex_prefs.cmc_url).unwrap_or_else(|_| cmcap::DEFAULT_URL.into());
        }
        if let Some(f) = b.fields.iter_mut().find(|f| f.id == "every") {
            f.value = self.ex_prefs.cmc_every_h.clamp(1, 168).to_string();
        }
        self.session.key_box = Some(b);
    }

    /// Action du panneau : clé (boîte), ⟳ = tester, ✕ = oublier ; l'interrupteur suit la clé.
    pub(crate) fn cmc_action(&mut self, a: ExAction) {
        match a {
            ExAction::Add => {}
            ExAction::Key(_) => self.cmc_key_box(),
            ExAction::Toggle(_) if self.session.cmc.is_none() => self.cmc_key_box(),
            ExAction::Toggle(_) => self.banner = Some("📈 CoinMarketCap reste branché tant que ta clé est là : ✕ pour l'oublier".into()),
            ExAction::Refresh(_) => {
                // ⟳ : un relevé tout de suite (4 crédits, jamais au-delà du plafond du mois)
                self.feeds.cmc_ctl.force.store(true, Ordering::SeqCst);
                self.banner = Some("📈 Relevé CoinMarketCap demandé (4 crédits)".into());
            }
            ExAction::Forget(_) => {
                // 🌐 navigateur : le serveur du PC l'efface de son trousseau (la réponse confirme)
                #[cfg(target_arch = "wasm32")]
                self.cle_serveur(Some(serde_json::json!({ "oublier": true }).to_string()));
                #[cfg(not(target_arch = "wasm32"))]
                {
                    keys::forget("cmc");
                    self.session.cmc = None;
                    self.watch.market.cmc_cle = false;
                    self.watch.market.cmc = None;
                    self.cmc_apply();
                    self.banner = Some("📈 Clé CoinMarketCap oubliée (retirée du trousseau) ; l'historique déjà gardé reste".into());
                }
            }
        }
    }

    /// Enregistre ce qui a été tapé dans la boîte : la clé au trousseau, le lien dans les réglages, puis un test.
    pub(crate) fn cmc_key_save(&mut self, key: String, url: String, every: String) {
        // fréquence du relevé (heures, 1 à 168 ; vide ou illisible : 24)
        self.ex_prefs.cmc_every_h = every.trim().parse::<u32>().unwrap_or(24).clamp(1, 168);
        self.feeds.cmc_ctl.every_s.store(self.ex_prefs.cmc_every_h as i64 * 3600, Ordering::SeqCst);
        let base = match cmcap::base_url(&url) {
            Ok(b) => b,
            Err(e) => {
                self.banner = Some(format!("📈 {e}"));
                return;
            }
        };
        let key = key.trim().to_string();
        if !cmcap::cle_valide(&key) {
            self.banner = Some("📈 Clé CoinMarketCap invalide : copie-la au complet depuis pro.coinmarketcap.com ▸ API Key".into());
            return;
        }
        // 🌐 navigateur : la clé part au serveur du PC, une seule fois ; la page n'en garde rien
        #[cfg(target_arch = "wasm32")]
        {
            let _ = base;
            Self::write_prefs("exchanges.json", &self.ex_prefs);
            self.cle_serveur(Some(serde_json::json!({ "cle": key }).to_string()));
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Err(e) = keys::save("cmc", "key", &key) {
                self.banner = Some(format!("⚠️ CoinMarketCap : {e} (le trousseau du système est-il ouvert ?)"));
                return;
            }
            self.ex_prefs.cmc_url = if base == cmcap::DEFAULT_URL { String::new() } else { base };
            Self::write_prefs("exchanges.json", &self.ex_prefs);
            self.session.cmc = Some(key);
            self.watch.market.cmc_cle = true;
            self.cmc_apply();
            self.cmc_test();
        }
    }

    /// Teste la clé en arrière-plan (réponse en bandeau : OK ou l'erreur de CoinMarketCap).
    pub(crate) fn cmc_test(&mut self) {
        let Some(key) = self.session.cmc.clone() else {
            self.banner = Some(NO_KEY.into());
            return;
        };
        let base = match self.cmc_base() {
            Ok(b) => b,
            Err(e) => {
                self.banner = Some(format!("📈 {e}"));
                return;
            }
        };
        if !argos_data::cmcap::reserve(&self.feeds.cmc_credits, 1, now_s()) {
            self.banner = Some("📈 Crédits CoinMarketCap du mois épuisés (10 000) : test remis au 1er du mois".into());
            return;
        }
        self.save_credits();
        self.banner = Some("📈 Test de la clé CoinMarketCap…".into());
        if let Some(proxy) = self.proxy.clone() {
            argos_data::rt::spawn(move || async move {
                let _ = proxy.send_event(UserEvent::CmcTest(cmcap::test(&base, &key).await.map_err(String::from)));
            });
        }
    }

    /// Adresse de l'API : celle des réglages (ordi), ou le relais du serveur du PC (navigateur).
    fn cmc_base(&self) -> Result<String, String> {
        #[cfg(target_arch = "wasm32")]
        return Ok(cmcap::RELAIS.into());
        #[cfg(not(target_arch = "wasm32"))]
        cmcap::base_url(&self.ex_prefs.cmc_url).map_err(String::from)
    }

    /// 🌐 Navigateur : demande au serveur du PC si la clé est rangée (`corps` absent), ou lui envoie `corps`
    /// (`{"cle": …}` ou `{"oublier": true}`) ; la réponse arrive en [`UserEvent::CleCmc`]. Le corps (et la clé) ne
    /// vit que le temps de l'envoi.
    #[cfg(target_arch = "wasm32")]
    pub(crate) fn cle_serveur(&mut self, corps: Option<String>) {
        let Some(proxy) = self.proxy.clone() else { return };
        let envoi = corps.is_some();
        if envoi {
            self.banner = Some("🔑 CoinMarketCap : envoi au trousseau de ton PC…".into());
        }
        argos_data::rt::spawn(move || async move {
            use argos_data::http::Http;
            let c = argos_data::rt::Client::default();
            let rep = match corps {
                Some(b) => c.post_json(CLE_SERVEUR, &b).await,
                None => c.get(CLE_SERVEUR).await,
            };
            let res = rep
                .map_err(|e| cmcap::erreur_du_relais(&e.to_string()))
                .and_then(|b| serde_json::from_str::<serde_json::Value>(&b).map_err(|e| e.to_string()))
                .and_then(|v| v["present"].as_bool().ok_or_else(|| "réponse du PC illisible".to_string()));
            let _ = proxy.send_event(UserEvent::CleCmc(res, envoi));
        });
    }

    /// 🌐 Réponse du serveur du PC : la clé y est (ou non) ; la page note seulement sa présence.
    pub(crate) fn cmc_cle_recue(&mut self, res: Result<bool, String>, envoi: bool) {
        match res {
            Ok(present) => {
                self.session.cmc = present.then(String::new);
                self.watch.market.cmc_cle = present;
                if !present {
                    self.watch.market.cmc = None;
                }
                self.cmc_apply();
                if envoi {
                    self.banner = Some(if present {
                        "🔑 CoinMarketCap : clé rangée dans le trousseau de ton PC (la page n'en garde rien)".into()
                    } else {
                        "📈 Clé CoinMarketCap oubliée (retirée du trousseau du PC) ; l'historique déjà gardé reste".into()
                    });
                    if present {
                        self.cmc_test();
                    }
                }
                self.refresh_watch();
            }
            Err(e) if envoi => self.banner = Some(format!("⚠️ CoinMarketCap : {e}")),
            Err(_) => {}
        }
        self.redraw();
    }

    /// Un relevé arrive : gardé (fichier local), et la chart CRYPTOCAP: ouverte se met à jour.
    pub(crate) fn cmc_point(&mut self, res: Result<cmcap::CapPoint, String>) {
        match res {
            Ok(p) => {
                self.save_credits();
                self.feeds.cmc_last = Some(p.clone());
                Self::write_prefs("cmc_last.json", &self.feeds.cmc_last);
                let mut m = std::mem::take(&mut self.watch.market);
                self.cmc_over(&mut m);
                self.watch.market = m;
                cmcap::push(&mut self.feeds.cmc_hist, p);
                self.feeds.cmc_note.clear();
                Self::write_prefs("cmc_history.json", &self.feeds.cmc_hist);
                self.cap_refresh();
                self.refresh_watch();
            }
            Err(e) => {
                self.feeds.cmc_note = e.clone();
                if matches!(self.chart.source, Source::Cap { .. }) {
                    self.banner = Some(format!("📈 CoinMarketCap : {e}"));
                }
            }
        }
        self.redraw();
    }

    /// Ouvre une série CRYPTOCAP: (« USDT.D ») avec l'historique gardé ; sans clé, le message pour l'ajouter.
    pub(crate) fn open_cap(&mut self, series: &str) {
        let Some(series) = cmcap::series_of(series) else { return };
        if self.feeds.cmc_hist.is_empty() {
            self.banner = Some(
                "📈 Historique encore vide : il se remplit tout seul (marché de CoinGecko, gratuit), ou avec une clé CoinMarketCap dans 🔑 Clés API".into(),
            );
        }
        self.save();
        self.write_cache();
        self.gen.fetch_add(1, Ordering::SeqCst);
        self.chart.source = Source::Cap { series: series.to_string() };
        // l'historique est en 5 min : on ne descend pas plus bas
        if self.chart.interval.secs() < cmcap::EVERY_S || self.chart.interval.secs() % cmcap::EVERY_S != 0 {
            self.chart.interval = Interval(cmcap::EVERY_S);
        }
        self.restore_session();
        self.title = cap_title(series, &self.chart.interval.short());
        (self.chart.live, self.ui.goto, self.pointer.grab, self.ui.list) = (None, None, None, None);
        (self.selected, self.draft) = (None, None);
        self.ui.context = ContextMenu::default();
        self.chart.viewport.right_offset = Viewport::default().right_offset;
        (self.chart.cache_path, self.chart.loading) = (None, None);
        self.chart.raw = cmcap::bars(&self.feeds.cmc_hist, series);
        let n = self.chart.raw.len();
        self.banner = Some(if n == 0 {
            format!("📈 CRYPTOCAP:{series} : historique vide pour l'instant, un point arrive toutes les 5 min (l'historique est payant chez CoinMarketCap, Argos le garde lui-même)")
        } else {
            format!("📈 CRYPTOCAP:{series} : {n} bougie(s) de 5 min gardées par Argos")
        });
        self.rebuild();
        self.start_free();
        self.set_window_title();
        self.redraw();
    }
}

/// Titre d'une série : « CRYPTOCAP:USDT.D · 1h · CoinMarketCap ».
pub(crate) fn cap_title(series: &str, label: &str) -> String {
    format!("CRYPTOCAP:{series} · {label} · CoinMarketCap")
}
