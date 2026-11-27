//! 🏦 Exchanges : activés, catalogues de symboles, exchanges perso, OANDA.

use crate::*;

impl App {
    /// Exchanges activés, dans l'ordre d'Argos.
    pub(crate) fn enabled(&self) -> Vec<Exchange> {
        Exchange::ALL
            .into_iter()
            .filter(|e| self.ex_prefs.enabled.iter().any(|id| id == e.id()))
            .collect()
    }

    /// Catalogues des exchanges activés trop vieux (ou jamais téléchargés) : téléchargés en arrière-plan.
    pub(crate) fn refresh_catalogs(&mut self) {
        let Some(st) = self.store.as_ref() else { return };
        let done = st.catalogs().unwrap_or_default();
        let old: Vec<Exchange> = self
            .enabled()
            .into_iter()
            .filter(|e| !done.iter().any(|c| c.exchange == e.id() && now_s() - c.updated_at < CATALOG_MAX_AGE_S))
            .collect();
        self.download_catalogs(old);
        // 🏦 OANDA : son catalogue complet, avec ton jeton (app ordi seulement : le jeton reste au trousseau)
        #[cfg(not(target_arch = "wasm32"))]
        if let (Some((token, live)), Some(proxy)) = (self.session.oanda.clone(), self.proxy.clone()) {
            let fresh = done.iter().any(|c| c.exchange == "oanda" && now_s() - c.updated_at < CATALOG_MAX_AGE_S);
            if !fresh && !self.feeds.oanda_cat_busy {
                self.feeds.oanda_cat_busy = true;
                argos_data::rt::spawn(move || async move {
                    let res = argos_data::oanda::instruments(&token, live).await;
                    let _ = proxy.send_event(UserEvent::OandaCatalog(res.map_err(String::from)));
                });
            }
        }
    }

    /// ＋ ouvert : les catalogues de toutes les plateformes (pas seulement les activées) trop vieux ou jamais
    /// téléchargés, pour ajouter un symbole de n'importe laquelle ; la recherche se refait à chaque arrivée.
    pub(crate) fn catalogues_pour_recherche(&mut self) {
        let Some(st) = self.store.as_ref() else { return };
        let done = st.catalogs().unwrap_or_default();
        let old: Vec<Exchange> = Exchange::ALL
            .into_iter()
            .filter(|e| !done.iter().any(|c| c.exchange == e.id() && now_s() - c.updated_at < CATALOG_MAX_AGE_S))
            .collect();
        self.download_catalogs(old);
    }

    /// Télécharge ces catalogues en arrière-plan (un à la fois) ; chacun arrive à la fenêtre (`UserEvent::Catalog`).
    pub(crate) fn download_catalogs(&mut self, list: Vec<Exchange>) {
        let list: Vec<Exchange> = list.into_iter().filter(|e| !self.cat_loading.contains(e)).collect();
        let Some(proxy) = self.proxy.clone().filter(|_| !list.is_empty()) else {
            return;
        };
        self.cat_loading.extend(list.iter().copied());
        argos_data::rt::spawn(move || async move {
            for ex in list {
                let feed = argos_data::ExchangeFeed::new(ex, argos_data::rt::Client::default());
                let got = match feed.instruments().await {
                    Ok(list) => list,
                    Err(e) => {
                        eprintln!("⚠️ Catalogue {} : {e}", ex.name());
                        vec![]
                    }
                };
                // vide = échec : la fenêtre arrête juste d'afficher « téléchargement… »
                if proxy.send_event(UserEvent::Catalog(ex, got)).is_err() {
                    return;
                }
            }
        });
    }

    /// Lignes du panneau Exchanges (état des catalogues, clés, téléchargements).
    pub(crate) fn exchange_rows(&self) -> Vec<ExRow> {
        let cats = self.store.as_ref().and_then(|s| s.catalogs().ok()).unwrap_or_default();
        Exchange::ALL
            .into_iter()
            .map(|e| {
                let c = cats.iter().find(|c| c.exchange == e.id());
                ExRow {
                    id: e.id().into(),
                    name: e.name().into(),
                    enabled: self.ex_prefs.enabled.iter().any(|id| id == e.id()),
                    symbols: c.map_or(0, |c| c.symbols),
                    age_s: c.map(|c| now_s() - c.updated_at),
                    has_key: self.session.key_set.contains(&e),
                    loading: self.cat_loading.contains(&e),
                    note: String::new(),
                }
            })
            // ➕ tes APIs personnalisées (une carte chacune, la clé dans le trousseau)
            .chain(self.ex_prefs.custom.iter().map(|n| ExRow {
                id: custom_id(n),
                name: n.clone(),
                enabled: true,
                symbols: 0,
                age_s: None,
                has_key: keys::has(&custom_id(n)),
                loading: false,
                note: String::new(),
            }))
            // 🏦 OANDA : forex, or, indices avec ton jeton (sans jeton : équivalents Hyperliquid)
            .chain(std::iter::once(ExRow {
                id: "oanda".into(),
                name: "OANDA (forex, or, indices)".into(),
                enabled: self.session.oanda.is_some(),
                symbols: argos_data::oanda::OANDA.len(),
                age_s: None,
                has_key: self.session.oanda.is_some(),
                loading: false,
                note: String::new(),
            }))
            // 📈 CoinMarketCap : dominances et capitalisations, avec ta clé (historique gardé par Argos)
            .chain((!super::cmc::CMC_OFF).then(|| self.cmc_row()))
            .collect()
    }

    /// Action du panneau Exchanges.
    pub(crate) fn exchange_action(&mut self, a: ExAction) {
        let id = match &a {
            ExAction::Add => {
                // ＋ Ajouter une API : les exchanges, ceux qui ont déjà une clé cochés ; un choix ouvre la boîte de clé
                let items = Exchange::ALL
                    .into_iter()
                    .enumerate()
                    .map(|(i, e)| MenuItem {
                        id: i as i64,
                        label: e.name().to_string(),
                        checked: self.session.key_set.contains(&e),
                        enabled: true,
                        ..Default::default()
                    })
                    .chain(std::iter::once(MenuItem {
                        id: Exchange::ALL.len() as i64,
                        label: "OANDA (jeton)".into(),
                        checked: self.session.oanda.is_some(),
                        enabled: true,
                        ..Default::default()
                    }))
                    .chain(std::iter::once(MenuItem {
                        id: Exchange::ALL.len() as i64 + 2,
                        label: "CoinMarketCap (clé)".into(),
                        checked: self.session.cmc.is_some(),
                        enabled: true,
                        ..Default::default()
                    }))
                    .chain(std::iter::once(MenuItem {
                        id: Exchange::ALL.len() as i64 + 1,
                        label: "API personnalisée…".into(),
                        icon: Some("➕"),
                        enabled: true,
                        ..Default::default()
                    }))
                    .collect();
                let groups = vec![MenuGroup {
                    title: "AJOUTER UNE API · LECTURE SEULE".into(),
                    items,
                    strip: false,
                }];
                let (ww, wh) = (self.win_w(), self.area.1 as f64);
                let anchor = Rect::new(self.side_x() + 14.0, self.top_h() + 108.0, self.watch.pw() - 28.0, 0.0);
                self.ui.list = Some((ListKind::AddApi, ListMenu::new(groups, vec![], anchor, ww, wh)));
                return;
            }
            ExAction::Toggle(id) | ExAction::Refresh(id) | ExAction::Key(id) | ExAction::Forget(id) => id.clone(),
        };
        if id == "oanda" {
            return self.oanda_action(a);
        }
        if id == "cmc" {
            return self.cmc_action(a);
        }
        if id.starts_with("custom-") {
            return self.custom_action(&id, a);
        }
        let Some(ex) = Exchange::parse(&id) else { return };
        match a {
            ExAction::Add => {}
            ExAction::Toggle(_) => {
                if let Some(k) = self.ex_prefs.enabled.iter().position(|e| *e == id) {
                    self.ex_prefs.enabled.remove(k);
                } else {
                    self.ex_prefs.enabled.push(id);
                    self.refresh_catalogs();
                }
                Self::write_prefs("exchanges.json", &self.ex_prefs);
            }
            ExAction::Refresh(_) => self.download_catalogs(vec![ex]),
            ExAction::Key(_) => {
                let ww = self.win_w();
                self.session.key_box = Some(KeyBox::new(ex.id(), ex.name(), ww, self.area.1 as f64 + self.top_h()));
            }
            ExAction::Forget(_) => {
                keys::forget(ex.id());
                self.session.key_set.remove(&ex);
                self.banner = Some(format!("🔑 Clé {} oubliée (retirée du trousseau)", ex.name()));
            }
        }
    }

    /// ➕ API personnalisée dans le panneau : sa clé (boîte, même nom), oublier (✕) ; pas de catalogue ni d'interrupteur.
    pub(crate) fn custom_action(&mut self, id: &str, a: ExAction) {
        let name = self.ex_prefs.custom.iter().find(|n| custom_id(n) == id).cloned().unwrap_or_default();
        match a {
            ExAction::Key(_) => {
                let (w, h) = (self.win_w(), self.area.1 as f64 + self.top_h());
                let mut b = KeyBox::new("custom", &name, w, h);
                if let Some(f) = b.fields.iter_mut().find(|f| f.id == "name") {
                    f.value = name;
                }
                self.session.key_box = Some(b);
            }
            ExAction::Forget(_) => {
                keys::forget(id);
                self.ex_prefs.custom.retain(|n| custom_id(n) != id);
                Self::write_prefs("exchanges.json", &self.ex_prefs);
                self.banner = Some(format!("🔑 {name} : clé oubliée (retirée du trousseau)"));
            }
            _ => self.banner = Some(format!("➕ {name} : API personnalisée, sa clé est gardée pour tes scripts et tes outils")),
        }
    }

    /// 🏦 OANDA dans le panneau API : jeton (boîte), tester (⟳), oublier (✕) ; l'interrupteur suit le jeton.
    pub(crate) fn oanda_action(&mut self, a: ExAction) {
        match a {
            ExAction::Add => {}
            ExAction::Key(_) => {
                let ww = self.win_w();
                self.session.key_box = Some(KeyBox::new("oanda", "OANDA", ww, self.area.1 as f64 + self.top_h()));
            }
            ExAction::Toggle(_) if self.session.oanda.is_none() => self.oanda_action(ExAction::Key(String::new())),
            ExAction::Toggle(_) => self.banner = Some("🏦 OANDA reste branché tant que ton jeton est là : ✕ pour l'oublier".into()),
            ExAction::Refresh(_) => self.oanda_test(),
            ExAction::Forget(_) => {
                keys::forget("oanda");
                self.session.oanda = None;
                self.banner = Some("🏦 Jeton OANDA oublié : les symboles OANDA reprennent leurs équivalents Hyperliquid".into());
            }
        }
    }

    /// Teste le jeton OANDA en arrière-plan (la réponse arrive en bandeau).
    pub(crate) fn oanda_test(&mut self) {
        let (Some(proxy), Some((token, live))) = (self.proxy.clone(), self.session.oanda.clone()) else {
            return;
        };
        self.banner = Some("🏦 Test du jeton OANDA…".into());
        argos_data::rt::spawn(move || async move {
            let _ = proxy.send_event(UserEvent::OandaTest(argos_data::oanda::test(&token, live).await.map_err(String::from)));
        });
    }
}
