//! 📥 Charger et changer ce qu'on regarde : symbole, intervalle, type de bougies, cache.

use crate::*;

/// 📜 Bougies demandées par page d'historique plus ancien, et au plus en mémoire (au-delà, on ne remonte plus).
pub(crate) const OLDER_PAGE: usize = 3000;
pub(crate) const MAX_RAW: usize = 200_000;
/// 📜 Pause entre deux pages (ms) : assez court pour que le dézoom ne bute pas, jamais une rafale.
const OLDER_PAUSE_MS: u128 = 700;
/// ⚡ Premier écran (étape 2b du plan de collecte) : le même nombre que la couche de données (et la coque Tauri).
pub(crate) const FIRST_SCREEN: usize = argos_data::FIRST_SCREEN;

impl App {
    /// 📜 La vue arrive au début de ce qui est chargé (dézoom ou défilement vers le passé) : on demande la page
    /// d'avant à l'exchange, une à la fois ; plus rien de plus ancien = début de l'historique (le dézoom s'arrête là).
    pub(crate) fn maybe_load_older(&mut self) {
        let Source::Live { exchange, base } = &self.chart.source else { return };
        let Some(tf) = self.source_tf() else { return };
        let (exchange, base) = (*exchange, base.clone());
        let key = format!("{}|{}", self.main_coin(), tf.seconds());
        if self.chart.older_key != key {
            (self.chart.older_key, self.chart.older_busy, self.chart.history_start) = (key.clone(), false, false);
            (self.chart.older_via, self.chart.debut_propre) = (None, None);
        }
        let exchange = self.chart.older_via.unwrap_or(exchange);
        // pas pendant un geste (glisser, doigt posé), et au plus une page toutes les 2 s : jamais de rafale de requêtes
        let gesture = self.pointer.drag.is_some() || self.pointer.finger_scroll.is_some() || self.pointer.touch.waiting();
        let recent = self.chart.older_at.is_some_and(|t| t.elapsed().as_millis() < OLDER_PAUSE_MS);
        if gesture
            || recent
            || self.chart.older_busy
            || self.chart.history_start
            || self.replay.is_some()
            || self.chart.loading.is_some()
            || self.chart.raw.len() >= MAX_RAW
            || !self.chart.viewport.near_start(&self.chart.slots)
        {
            return;
        }
        let Some(before) = self.chart.raw.first().map(|b| b.time) else { return };
        let Some(proxy) = self.proxy.clone() else { return };
        self.chart.older_busy = true;
        self.chart.older_at = Some(web_time::Instant::now());
        argos_data::rt::spawn(move || async move {
            let feed = argos_data::ExchangeFeed::new(exchange, argos_data::rt::Client::default());
            // 🚦 file commune, basse priorité
            let res = argos_data::file::basse(exchange, || feed.older(&base, tf, before, OLDER_PAGE))
                .await
                .map_err(|e| e.to_string());
            let _ = proxy.send_event(UserEvent::Older(key, res));
        });
    }

    /// ⚡ Après le premier écran : remplit le passé en arrière-plan jusqu'à `DEFAULT_CANDLES` (une demande de pages vers
    /// le passé, comme le dézoom), sans bloquer la chart ni le direct. Rien si on a déjà assez (cache).
    pub(crate) fn fill_history(&mut self) {
        let Source::Live { exchange, base } = &self.chart.source else { return };
        let Some(tf) = self.source_tf() else { return };
        let need = DEFAULT_CANDLES.saturating_sub(self.chart.raw.len());
        let (Some(before), Some(proxy)) = (self.chart.raw.first().map(|b| b.time), self.proxy.clone()) else {
            return;
        };
        if need == 0 || self.chart.older_busy || self.replay.is_some() {
            return;
        }
        let (exchange, base) = (*exchange, base.clone());
        let key = format!("{}|{}", self.main_coin(), tf.seconds());
        if self.chart.older_key != key {
            (self.chart.older_key, self.chart.history_start) = (key.clone(), false);
            (self.chart.older_via, self.chart.debut_propre) = (None, None);
        }
        self.chart.older_busy = true;
        self.chart.older_at = Some(web_time::Instant::now());
        argos_data::rt::spawn(move || async move {
            let feed = argos_data::ExchangeFeed::new(exchange, argos_data::rt::Client::default());
            // 🚦 file commune, basse priorité : après le premier écran, une demande à la fois par exchange
            let res = argos_data::file::basse(exchange, || feed.older(&base, tf, before, need))
                .await
                .map_err(|e| e.to_string());
            let _ = proxy.send_event(UserEvent::Older(key, res));
        });
    }

    /// 🔌 Après une coupure (réseau, veille) : redemande les bougies manquées depuis la dernière bougie fermée (en haute
    /// priorité, comme un premier écran), puis la suite habituelle : remplissage et réparation des trous.
    pub(crate) fn catch_up(&mut self) {
        let Source::Live { .. } = &self.chart.source else { return };
        let (Some(tf), Some(proxy)) = (self.source_tf(), self.proxy.clone()) else {
            return;
        };
        if self.chart.loading.is_some() || self.chart.raw.is_empty() {
            return;
        }
        let limit = cache::missing(&self.chart.raw, now_s(), tf, DEFAULT_CANDLES);
        self.chart.loading = Some(vec![]);
        let label = self.chart.interval.short();
        start_load(self.chart.source.clone(), tf, &label, limit, None, proxy, self.gen.load(Ordering::SeqCst));
    }

    /// 🕳️ Répare les trous de la chart (étape 3 du plan) : bougies manquantes et bougies refusées, sauf un marché
    /// fermé (week-end, fêtes pour l'or, les indices, les actions) et les trous déjà essayés sans succès. Une seule
    /// réparation à la fois, au plus 10 trous par passage, jamais pendant un chargement ou un remplissage.
    pub(crate) fn repair_gaps(&mut self) {
        use argos_data::trous;
        let Source::Live { exchange, base } = &self.chart.source else { return };
        let Some(tf) = self.source_tf() else { return };
        let Some(proxy) = self.proxy.clone() else { return };
        if self.chart.repair_busy || self.chart.older_busy || self.chart.loading.is_some() || self.replay.is_some() {
            return;
        }
        let (exchange, base) = (*exchange, base.clone());
        let key = format!("{}|{}", self.main_coin(), tf.seconds());
        if self.chart.trous_connus.0 != key {
            // les trous connus de cette série, gardés à côté de son cache (ils survivent à un redémarrage)
            let connus = self
                .chart
                .cache_path
                .as_deref()
                .map(|p| trous::lire_connus(&trous::chemin_connus(p)))
                .unwrap_or_default();
            self.chart.trous_connus = (key.clone(), connus);
        }
        let cal = trous::calendrier(exchange, &base);
        // seulement chez l'exchange de la chart (pas dans le passé venu d'un exchange d'appoint) ; un trou que
        // l'exchange n'avait pas est laissé de côté une semaine, puis réessayé
        let eviter = trous::a_eviter(&self.chart.trous_connus.1, now_s());
        let todo = trous::trouver(self.bougies_propres(), tf, &self.chart.a_reparer, cal, &eviter);
        if todo.is_empty() {
            return;
        }
        self.chart.repair_busy = true;
        argos_data::rt::spawn(move || async move {
            let feed = argos_data::ExchangeFeed::new(exchange, argos_data::rt::Client::default());
            let r = argos_data::file::basse(exchange, || trous::reparer(&feed, &base, tf, &todo, 10)).await;
            let _ = proxy.send_event(UserEvent::Repaired(key, r));
        });
    }

    /// 🕳️ Trous réparés : les bougies retrouvées entrent dans la chart et le cache ; les trous que l'exchange n'a pas
    /// sont gardés comme « connus » (réessayés dans une semaine, notés à côté du cache). S'il en reste d'autres, un
    /// nouveau passage suit.
    pub(crate) fn gaps_repaired(&mut self, key: String, r: argos_data::trous::Reparation) {
        self.chart.repair_busy = false;
        if key != self.chart.trous_connus.0 {
            return; // un autre marché entre-temps
        }
        if !r.irreparables.is_empty() {
            argos_data::trous::noter(&mut self.chart.trous_connus.1, &r.irreparables, now_s());
            if let Some(p) = &self.chart.cache_path {
                let _ = argos_data::trous::ecrire_connus(&argos_data::trous::chemin_connus(p), &self.chart.trous_connus.1);
            }
        }
        if !r.bars.is_empty() {
            self.chart.raw = argos_data::trous::combler(&self.chart.raw, &r.bars);
            let raw = &self.chart.raw;
            self.chart.a_reparer.retain(|t| raw.binary_search_by_key(t, |b| b.time).is_err());
            self.rebuild();
            self.write_cache();
            println!("🕳️ {} bougie(s) retrouvée(s) dans les trous", r.bars.len());
        }
        if !r.bars.is_empty() || !r.irreparables.is_empty() {
            self.repair_gaps();
        }
        self.redraw();
    }

    /// 📜 Page d'historique plus ancien reçue : ajoutée au début (la vue, ancrée à droite, ne bouge pas).
    pub(crate) fn older_arrived(&mut self, key: String, res: Result<Vec<Bar>, String>) {
        if key != self.chart.older_key {
            return; // un autre marché entre-temps
        }
        self.chart.older_busy = false;
        let first = self.chart.raw.first().map_or(i64::MAX, |b| b.time);
        match res {
            Ok(bars) => {
                let older: Vec<Bar> = bars.into_iter().filter(|b| b.time < first).collect();
                if older.is_empty() {
                    if !self.older_fallback() {
                        self.chart.history_start = true;
                        self.banner = Some("📜 Début de l'historique disponible".into());
                    }
                } else {
                    // 🧾 passé d'un exchange d'appoint : on note où commencent les bougies de l'exchange de la chart
                    if self.chart.older_via.is_some() && self.chart.debut_propre.is_none() && first != i64::MAX {
                        self.chart.debut_propre = Some(first);
                    }
                    self.chart.raw = [older, std::mem::take(&mut self.chart.raw)].concat();
                    self.rebuild();
                    // le passé rempli est gardé : le prochain lancement n'a plus à le redemander
                    self.write_cache();
                    // 🕳️ les trous du passé rempli
                    self.repair_gaps();
                }
            }
            Err(e) => {
                if !self.older_fallback() {
                    self.chart.history_start = true;
                    self.banner = Some(format!("📜 Début de l'historique (plus ancien indisponible : {e})"));
                }
            }
        }
        // ⏯ un replay attend ce passé
        self.sim_replay_poursuivre();
        self.redraw();
    }

    /// 📜 Demande une page de `n` bougies plus anciennes que la première (file basse priorité, exchange d'appoint
    /// compris) ; la réponse arrive dans `older_arrived`. `false` si rien n'est parti (pas un marché crypto, pas de
    /// bougie, pas de canal).
    pub(crate) fn request_older(&mut self, n: usize) -> bool {
        let Source::Live { exchange, base } = &self.chart.source else { return false };
        let Some(tf) = self.source_tf() else { return false };
        let (Some(before), Some(proxy)) = (self.chart.raw.first().map(|b| b.time), self.proxy.clone()) else {
            return false;
        };
        let exchange = self.chart.older_via.unwrap_or(*exchange);
        let (base, key) = (base.clone(), self.older_key_sync());
        self.chart.older_busy = true;
        self.chart.older_at = Some(web_time::Instant::now());
        argos_data::rt::spawn(move || async move {
            let feed = argos_data::ExchangeFeed::new(exchange, argos_data::rt::Client::default());
            let res = argos_data::file::basse(exchange, || feed.older(&base, tf, before, n))
                .await
                .map_err(|e| e.to_string());
            let _ = proxy.send_event(UserEvent::Older(key, res));
        });
        true
    }

    /// 📜 Clé du passé du marché ouvert (symbole | unité) ; un autre marché qu'avant : on repart de zéro (pas de
    /// demande en cours, début de l'historique pas encore vu).
    pub(crate) fn older_key_sync(&mut self) -> String {
        let tf = self.source_tf().map_or(0, |tf| tf.seconds());
        let key = format!("{}|{}", self.main_coin(), tf);
        if self.chart.older_key != key {
            (self.chart.older_key, self.chart.older_busy, self.chart.history_start) = (key.clone(), false, false);
            (self.chart.older_via, self.chart.debut_propre) = (None, None);
        }
        key
    }

    /// 📜 L'exchange de la chart n'a plus rien de plus ancien : la suite vient de Binance Futures (même perp), une
    /// seule fois. `false` s'il n'y a pas d'appoint (déjà essayé, ou la chart est déjà sur Binance).
    fn older_fallback(&mut self) -> bool {
        let Source::Live { exchange, .. } = &self.chart.source else { return false };
        if self.chart.older_via.is_some() || *exchange == argos_data::Exchange::Binance {
            return false;
        }
        self.chart.older_via = Some(argos_data::Exchange::Binance);
        self.chart.older_at = None;
        self.banner = Some("📜 Plus ancien : la suite de l'historique vient de Binance Futures".into());
        true
    }

    /// Change de type (`step` = +1 suivant, −1 précédent) et recalcule la place des bougies.
    pub(crate) fn switch_style(&mut self, step: isize) {
        let n = self.chart.styles.len() as isize;
        self.chart.style = (self.chart.style as isize + step).rem_euclid(n) as usize;
        self.chart.slots = slots(&self.chart.bars, &self.chart.styles[self.chart.style]);
        self.chart.viewport.scroll_px(0.0, &self.chart.slots); // garde des bougies à l'écran
        if let Some(w) = &self.window {
            w.set_title(&format!("Argos HSS Trading — {} — {}", self.title, self.chart.styles[self.chart.style].name()));
        }
    }

    /// Recharge les bougies (F5 / Ctrl + R) en gardant le type, le zoom, l'échelle et les lignes.
    /// En cas d'échec, les bougies actuelles restent et un bandeau l'indique.
    pub(crate) fn reload(&mut self) {
        // ⟳ Actualiser (Ctrl + R, Ctrl + K, ↻) : un marché en direct est rouvert comme si on le choisissait de nouveau
        // (direct reconnecté, bougies rechargées, vue sur la dernière bougie) ; la mise en page est gardée avant
        if let Source::Live { exchange, base } = self.chart.source.clone() {
            self.save();
            self.open_live(exchange, base);
            self.banner = Some("⟳ Actualisé : direct reconnecté, bougies rechargées".into());
            return self.redraw();
        }
        self.chart.viewport.right_offset = Viewport::default().right_offset;
        let Some(tf) = self.source_tf() else {
            self.banner = Some("Bougies en secondes : construites en direct, rien à recharger".into());
            return self.redraw();
        };
        // en arrière-plan : la réponse arrive comme un chargement (`UserEvent::Loaded`), les bougies actuelles restent
        if let Some(proxy) = self.proxy.clone() {
            self.banner = Some("⟳ Actualisation : bougies rechargées, vue sur la dernière bougie".into());
            start_load(
                self.chart.source.clone(),
                tf,
                &self.chart.interval.short(),
                DEFAULT_CANDLES,
                self.session.oanda.clone(),
                proxy,
                self.gen.load(Ordering::SeqCst),
            );
        }
        self.redraw();
    }

    /// 🧾 Les bougies de l'exchange de la chart seulement (sans le passé venu d'un exchange d'appoint) : ce qui peut aller
    /// dans son cache et être réparé chez lui.
    pub(crate) fn bougies_propres(&self) -> &[Bar] {
        let debut = self.chart.debut_propre.unwrap_or(i64::MIN);
        &self.chart.raw[self.chart.raw.partition_point(|b| b.time < debut)..]
    }

    /// 🕯 Web: the browser's candle cache has been read for the series opened (`gen`): its candles first, then the
    /// download of only what is missing (all of it when the cache had nothing or failed).
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub(crate) fn cache_lu(&mut self, gen: u64, bars: Vec<Bar>) {
        let (Some(tf), Some(proxy)) = (self.chart.interval.source(), self.proxy.clone()) else {
            return;
        };
        if self.chart.raw.is_empty() && !bars.is_empty() {
            self.chart.raw = bars;
            // (🗂 🗑 the banner of a layout opened or deleted stays)
            if !self.banner.as_ref().is_some_and(|b| b.starts_with('🗂') || b.starts_with('🗑')) {
                self.banner = Some("Mise à jour depuis la dernière session…".into());
            }
            self.rebuild();
        }
        let limit = cache::missing(&self.chart.raw, now_s(), tf, DEFAULT_CANDLES).min(FIRST_SCREEN);
        let label = self.chart.interval.short();
        start_load(self.chart.source.clone(), tf, &label, limit, None, proxy, gen);
        self.redraw();
    }

    /// Écrit le cache des bougies (rien pendant le chargement du démarrage, ni pour un fichier ou un secours) :
    /// seulement les bougies fermées, la bougie en cours reste en mémoire.
    pub(crate) fn write_cache(&self) {
        // 🕯 web: the browser's candle cache (its own place, errors ignored)
        #[cfg(target_arch = "wasm32")]
        if let (Source::Live { exchange, base }, Some(tf)) = (&self.chart.source, self.source_tf()) {
            let propres = self.bougies_propres();
            if self.chart.loading.is_none() {
                if let Some(data) = crate::persist::bougies_web::encode(propres, tf, now_s()) {
                    let name = cache::file_name(*exchange, base, tf);
                    wasm_bindgen_futures::spawn_local(crate::persist::bougies_web::web::ecrire(name, data, now_s()));
                }
            }
        }
        let (Some(path), Some(tf)) = (&self.chart.cache_path, self.source_tf()) else {
            return;
        };
        let propres = self.bougies_propres();
        if self.chart.loading.is_none() && !propres.is_empty() {
            if let Err(e) = cache::write(path, propres, tf, now_s()) {
                eprintln!("⚠️ Cache impossible ({}) : {e}", path.display());
            }
        }
    }

    /// Reprend la session sauvegardée du symbole ouvert (type, zoom, réglages, dessins). Sans sauvegarde : pas de
    /// dessins, le reste ne change pas.
    pub(crate) fn restore_session(&mut self) {
        self.chart.save_path = Some(saved::file_name(&self.chart.source.key()));
        let sv = self.chart.save_path.as_deref().and_then(saved::read);
        self.chart.drawings = vec![];
        if let Some(sv) = &sv {
            if let Some(i) = self.chart.styles.iter().position(|s| s.name() == sv.style) {
                self.chart.style = i;
            }
            if sv.bar_spacing.is_finite() && sv.bar_spacing > 0.0 {
                self.chart.viewport.bar_spacing = sv.bar_spacing;
            }
            self.chart.settings = sv.settings.clone();
            self.chart.drawings = sv.drawings.clone();
            println!("↩️ Session reprise ({} dessin(s))", self.chart.drawings.len());
        }
        *self.chart.last_saved.borrow_mut() = sv;
        *self.chart.hist.borrow_mut() = Default::default(); // ↶ ↷ un autre symbole : historique à part
    }

    /// Intervalle de l'exchange à télécharger pour l'intervalle affiché (`None` : secondes, construites en direct).
    pub(crate) fn source_tf(&self) -> Option<Timeframe> {
        match self.chart.source {
            Source::File(_) | Source::Cap { .. } => None,
            Source::Dex { .. } => Some(argos_data::dex::tf_of(argos_data::dex::gt_step(self.chart.interval.secs()).2)),
            Source::Live { .. } | Source::Ratio { .. } | Source::Synth { .. } => self.chart.interval.source(),
            Source::Oanda { .. } => self.chart.interval.source_fx().filter(|tf| argos_data::oanda::granularity(*tf).is_some()),
        }
    }

    /// Refait les bougies affichées à partir des bougies brutes (regroupées si l'intervalle n'est pas celui de la
    /// source) et leur place.
    pub(crate) fn rebuild(&mut self) {
        self.chart.bars = if self.data_interval() == self.chart.interval {
            self.chart.raw.clone()
        } else {
            aggregate(&self.chart.raw, self.chart.interval.secs())
        };
        // ⏪ replay : la chart s'arrête à l'heure choisie (toutes les bougies pendant qu'on choisit le départ)
        if let Some(rp) = &mut self.replay {
            self.replay_times = self.chart.bars.iter().map(|b| b.time).collect();
            rp.left = self.replay_times.iter().filter(|&&t| t > rp.t).count();
            if !rp.pick {
                self.chart.bars.retain(|b| b.time <= rp.t);
            }
        }
        self.chart.slots = slots(&self.chart.bars, &self.chart.styles[self.chart.style]);
        self.sim_tick(); // 🧪 les bougies finies jouent les ordres du Simulateur
    }

    /// Intervalle des bougies reçues (celui affiché, ou sa source quand il est regroupé, ex. 3h depuis 1h).
    fn data_interval(&self) -> Interval {
        match self.chart.source {
            Source::File(_) => self.chart.file_interval,
            Source::Cap { .. } => Interval(argos_data::cmcap::EVERY_S),
            Source::Dex { .. } => Interval(argos_data::dex::gt_step(self.chart.interval.secs()).2),
            Source::Live { .. } | Source::Ratio { .. } | Source::Synth { .. } => self.chart.interval.source().map_or(self.chart.interval, Interval::from),
            Source::Oanda { .. } => self.chart.interval.source_fx().map_or(self.chart.interval, Interval::from),
        }
    }

    /// ⚡ Bougie du direct : si elle ne fait que bouger la bougie en cours, seule la dernière bougie affichée change (pas
    /// de copie ni de regroupement de tout l'historique à chaque tick) ; une nouvelle bougie, un regroupement, le replay
    /// ou un type à largeur variable refont tout.
    pub(crate) fn live_bar(&mut self, b: Bar) {
        let new = live::apply(&mut self.chart.raw, b);
        self.live_affiche(new);
    }

    /// ⚡ Trades du direct : ils remplissent les bougies téléchargées (celles de la source, regroupées ensuite comme les
    /// autres ; Bitstamp et Coinbase n'ont pas de canal de bougies), ou les bougies en secondes ; puis comme une bougie
    /// du direct. (Ils remplissaient des bougies de l'unité affichée : en 1 semaine, faite de jours, rien ne bougeait.)
    pub(crate) fn live_trades(&mut self, list: Vec<live::Trade>) {
        let secs = self.data_interval().secs();
        let mut new = false;
        for t in list {
            new |= live::fold_trade(&mut self.chart.raw, t, secs);
        }
        self.live_affiche(new);
    }

    /// ⚡ Le direct arrivé dans les bougies brutes (`new` : une bougie a commencé) passe à l'écran.
    fn live_affiche(&mut self, new: bool) {
        let simple = self.replay.is_none()
            && self.data_interval() == self.chart.interval
            && !matches!(self.chart.styles[self.chart.style], CandleStyle::Volume(_) | CandleStyle::Renko(_));
        match (self.chart.bars.last_mut(), self.chart.raw.last()) {
            (Some(last), Some(r)) if simple && !new && last.time == r.time => *last = *r,
            _ => self.rebuild(),
        }
    }

    /// Ouvre un symbole d'un exchange dans l'intervalle affiché : sa session, son cache (affiché tout de suite), puis
    /// en arrière-plan le téléchargement de ce qui manque et le direct (bougies, ou trades pour les secondes).
    /// Les réponses en retard d'un symbole ou d'un intervalle quitté sont ignorées.
    pub(crate) fn open_live(&mut self, exchange: Exchange, base: String) {
        let gen = self.gen.fetch_add(1, Ordering::SeqCst) + 1;
        self.chart.source = Source::Live { exchange, base: base.clone() };
        // 🔄 le dernier marché ouvert : rouvert au démarrage, et sur les autres appareils en direct
        self.noter_marche();
        if exchange == Exchange::Hyperliquid {
            self.watch.selected = base.clone();
        }
        self.restore_session();
        let label = self.chart.interval.short();
        self.title = exchange.chart_title(&base, &label);
        (self.chart.live, self.ui.goto, self.pointer.grab, self.ui.list) = (None, None, None, None);
        (self.selected, self.draft) = (None, None);
        self.ui.context = ContextMenu::default();
        self.chart.viewport.right_offset = Viewport::default().right_offset;
        match self.chart.interval.source() {
            Some(tf) => {
                self.chart.cache_path = cache_dir().map(|d| d.join(cache::file_name(exchange, &base, tf)));
                self.chart.raw = self.chart.cache_path.as_deref().and_then(|p| cache::read(p, tf, now_s())).unwrap_or_default();
                self.chart.debut_propre = None;
                self.banner = Some(if self.chart.raw.is_empty() {
                    format!("Chargement de {DEFAULT_CANDLES} bougies sur {}…", exchange.name())
                } else {
                    format!("Mise à jour depuis la dernière session sur {}…", exchange.name())
                });
                self.chart.loading = Some(vec![]);
            }
            None => {
                // secondes : pas d'historique, les bougies se construisent avec les trades à partir de maintenant
                (self.chart.cache_path, self.chart.loading) = (None, None);
                self.chart.raw.clear();
                self.banner = Some(format!("Bougies de {label} construites en direct à partir de maintenant (pas d'historique)"));
            }
        }
        self.rebuild();
        self.start_free();
        if let Some(proxy) = self.proxy.clone() {
            if let Some(tf) = self.chart.interval.source() {
                // 🕯 web: the browser's candle cache is read first (async), then only what is missing is asked for
                #[cfg(target_arch = "wasm32")]
                {
                    let (name, p) = (cache::file_name(exchange, &base, tf), proxy.clone());
                    wasm_bindgen_futures::spawn_local(async move {
                        let now = now_s();
                        let bars = crate::persist::bougies_web::web::lire(&name, now)
                            .await
                            .and_then(|d| crate::persist::bougies_web::decode(&d, tf, now))
                            .unwrap_or_default();
                        let _ = p.send_event(crate::events::UserEvent::CacheLu(gen, bars));
                    });
                }
                // ⚡ premier écran d'abord (le reste du passé vient après, en arrière-plan : `fill_history`)
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let limit = cache::missing(&self.chart.raw, now_s(), tf, DEFAULT_CANDLES).min(FIRST_SCREEN);
                    start_load(self.chart.source.clone(), tf, &label, limit, None, proxy.clone(), gen);
                }
            }
            // le direct : bougies, ou trades (bougies en secondes, exchanges sans canal de bougies) ; 🔑 KuCoin : jeton
            match live::direct_chart(exchange, &base, self.chart.interval.source()) {
                Some(direct) => {
                    self.chart.live = Some(false);
                    start_live(exchange, direct, proxy, self.gen.clone(), gen);
                }
                // 🔑 KuCoin dans une page web : le direct de l'exchange de secours suivra ses bougies (`direct_de_secours`)
                None => println!("ℹ️ Pas de direct pour {} ici : celui de l'exchange de secours suivra.", exchange.name()),
            }
        }
        self.set_window_title();
    }

    /// 📡 Le direct à brancher quand des bougies arrivent d'un exchange de secours (`secours`) sur une chart qui n'a
    /// pas de direct ici (🔑 KuCoin dans une page web : son jeton y est refusé) : celui de ce secours, pour que la chart
    /// bouge avec les données qu'elle montre. `None` : la chart a déjà son direct, ou n'est pas celle d'un exchange.
    pub(crate) fn direct_de_secours(&self, secours: Option<Exchange>) -> Option<(Exchange, live::DirectChart)> {
        let (Some(ex), None, Source::Live { base, .. }) = (secours, self.chart.live, &self.chart.source) else {
            return None;
        };
        live::direct_chart(ex, base, self.chart.interval.source()).map(|d| (ex, d))
    }

    /// 🏦 Ouvre un instrument OANDA (`XAU_USD`) avec ton jeton : sa session et ses dessins, puis les bougies en
    /// arrière-plan (relues aux 15 s : OANDA n'a pas de flux en direct ici).
    pub(crate) fn open_oanda(&mut self, instrument: String) {
        let gen = self.gen.fetch_add(1, Ordering::SeqCst) + 1;
        self.chart.source = Source::Oanda {
            instrument: instrument.clone(),
        };
        self.restore_session();
        let label = self.chart.interval.short();
        self.title = format!("{} · OANDA · {label}", instrument.replace('_', ""));
        (self.chart.live, self.ui.goto, self.pointer.grab, self.ui.list) = (None, None, None, None);
        (self.selected, self.draft) = (None, None);
        self.ui.context = ContextMenu::default();
        self.chart.viewport.right_offset = Viewport::default().right_offset;
        (self.chart.cache_path, self.chart.loading) = (None, Some(vec![]));
        self.chart.raw.clear();
        self.banner = Some(format!("🏦 Chargement de {} sur OANDA…", instrument.replace('_', "")));
        self.rebuild();
        self.start_free();
        if let (Some(proxy), Some(tf)) = (self.proxy.clone(), self.source_tf()) {
            self.oanda_at = Instant::now();
            start_load(self.chart.source.clone(), tf, &label, DEFAULT_CANDLES, self.session.oanda.clone(), proxy, gen);
        }
        self.set_window_title();
    }

    /// 🦎 Ouvre une paire DEX (GeckoTerminal, gratuit) : pas de direct, les bougies sont relues à la minute (limite
    /// de 10 appels par minute de GeckoTerminal).
    pub(crate) fn open_dex(&mut self, network: String, pool: String, name: String) {
        let gen = self.gen.fetch_add(1, Ordering::SeqCst) + 1;
        self.chart.source = Source::Dex {
            network: network.clone(),
            pool,
            name: name.clone(),
        };
        self.restore_session();
        let label = self.chart.interval.short();
        self.title = crate::actions::dex_title(&name, &network, &label);
        (self.chart.live, self.ui.goto, self.pointer.grab, self.ui.list) = (None, None, None, None);
        (self.selected, self.draft) = (None, None);
        self.ui.context = ContextMenu::default();
        self.chart.viewport.right_offset = Viewport::default().right_offset;
        (self.chart.cache_path, self.chart.loading) = (None, Some(vec![]));
        self.chart.raw.clear();
        // la carte DEX se recharge pour cette pool
        (self.feeds.dex_at, self.feeds.dex_info) = (None, None);
        self.feeds.dex_trades.clear();
        self.banner = Some(format!("🦎 Chargement de {name} ({network}) sur GeckoTerminal…"));
        self.rebuild();
        self.start_free();
        if let (Some(proxy), Some(tf)) = (self.proxy.clone(), self.source_tf()) {
            self.oanda_at = Instant::now();
            start_load(self.chart.source.clone(), tf, &label, DEFAULT_CANDLES, None, proxy, gen);
        }
        self.set_window_title();
    }

    /// 🧮 Ouvre une paire calculée (`base` ÷ `quote` bougie par bougie) : pas de direct, les bougies sont relues aux
    /// 15 s comme OANDA.
    pub(crate) fn open_ratio(&mut self, exchange: Exchange, base: String, quote: String) {
        let gen = self.gen.fetch_add(1, Ordering::SeqCst) + 1;
        self.chart.source = Source::Ratio {
            exchange,
            base: base.clone(),
            quote: quote.clone(),
        };
        self.restore_session();
        let label = self.chart.interval.short();
        self.title = crate::run::ratio_title(exchange, &base, &quote, &label);
        (self.chart.live, self.ui.goto, self.pointer.grab, self.ui.list) = (None, None, None, None);
        (self.selected, self.draft) = (None, None);
        self.ui.context = ContextMenu::default();
        self.chart.viewport.right_offset = Viewport::default().right_offset;
        (self.chart.cache_path, self.chart.loading) = (None, Some(vec![]));
        self.chart.raw.clear();
        self.banner = Some(format!(
            "🧮 Chargement de {} : {base} ÷ {quote} sur {}, bougie par bougie (pas de volume)…",
            argos_data::tv::ratio_name(&base, &quote),
            exchange.name()
        ));
        self.rebuild();
        self.start_free();
        if let (Some(proxy), Some(tf)) = (self.proxy.clone(), self.source_tf()) {
            self.oanda_at = Instant::now();
            start_load(self.chart.source.clone(), tf, &label, DEFAULT_CANDLES, None, proxy, gen);
        }
        self.set_window_title();
    }

    /// 🧮 Ouvre un symbole calculé gratuit (« XAUGBP », « DXY »). Avec un jeton OANDA (desktop), l'or et l'argent en
    /// livre / euro viennent plutôt d'OANDA (vrais prix) ; pas de direct, les bougies sont relues aux 15 s.
    pub(crate) fn open_synth(&mut self, sym: String) {
        if let (Some(ins), true) = (argos_data::synth::oanda_of(&sym), self.session.oanda.is_some()) {
            return self.open_oanda(ins.to_string());
        }
        let gen = self.gen.fetch_add(1, Ordering::SeqCst) + 1;
        self.chart.source = Source::Synth { sym: sym.clone() };
        self.restore_session();
        let label = self.chart.interval.short();
        self.title = crate::run::synth_title(&sym, &label);
        (self.chart.live, self.ui.goto, self.pointer.grab, self.ui.list) = (None, None, None, None);
        (self.selected, self.draft) = (None, None);
        self.ui.context = ContextMenu::default();
        self.chart.viewport.right_offset = Viewport::default().right_offset;
        (self.chart.cache_path, self.chart.loading) = (None, Some(vec![]));
        self.chart.raw.clear();
        self.banner = Some(format!("🧮 Chargement de {sym} ({})…", argos_data::synth::name(&sym)));
        self.rebuild();
        self.start_free();
        if let (Some(proxy), Some(tf)) = (self.proxy.clone(), self.source_tf()) {
            self.oanda_at = Instant::now();
            start_load(self.chart.source.clone(), tf, &label, DEFAULT_CANDLES, None, proxy, gen);
        }
        self.set_window_title();
    }

    /// Change d'intervalle (barre flottante, menu, watchlist) : garde le symbole, sa session et ses dessins.
    pub(crate) fn set_interval(&mut self, iv: Interval) {
        if !self.interval_ok(iv) {
            self.banner = Some(format!("{} : pas disponible pour cette source", iv.label()));
            return self.redraw();
        }
        self.ui.list = None;
        if iv == self.chart.interval {
            return self.redraw();
        }
        // ⏪ during a replay the hour stays (a bigger unit first finishes its bar in progress)
        let avant = self.replay_heure();
        let heure = self.replay_avant_ut(iv);
        self.save();
        self.write_cache();
        self.chart.interval = iv;
        self.tools.interval = iv.secs();
        self.save_tools();
        if let (Some(h), Some(rp)) = (heure, self.replay.as_mut()) {
            rp.t = argos_core::derniere_finie(h, iv.secs());
        }
        match self.chart.source.clone() {
            Source::Live { exchange, base } => self.open_live(exchange, base),
            Source::Oanda { instrument } => self.open_oanda(instrument),
            Source::Ratio { exchange, base, quote } => self.open_ratio(exchange, base, quote),
            Source::Synth { sym } => self.open_synth(sym),
            Source::Dex { network, pool, name } => self.open_dex(network, pool, name),
            Source::Cap { series } => {
                self.rebuild();
                self.chart.viewport.scroll_px(0.0, &self.chart.slots);
                self.start_free();
                self.title = crate::actions::cap_title(&series, &iv.short());
                self.set_window_title();
            }
            Source::File(path) => {
                self.rebuild();
                self.chart.viewport.scroll_px(0.0, &self.chart.slots);
                self.start_free();
                let name = std::path::Path::new(&path)
                    .file_name()
                    .map_or(path.clone(), |f| f.to_string_lossy().into_owned());
                self.title = format!("{name} · {} · fichier", iv.short());
                self.set_window_title();
            }
        }
        if let Some(h) = heure {
            self.replay_apres_ut(h, avant.is_some_and(|a| a < h));
        }
        self.redraw();
    }

    /// Cet intervalle est-il possible pour la source ouverte ?
    pub(crate) fn interval_ok(&self, iv: Interval) -> bool {
        match &self.chart.source {
            Source::File(_) => iv.secs() >= self.chart.file_interval.secs() && iv.secs() % self.chart.file_interval.secs() == 0,
            Source::Live { exchange, .. } => iv.source().is_some() || live::trades_subscription(*exchange, "").is_some(),
            Source::Oanda { .. } => iv.source_fx().is_some_and(|tf| argos_data::oanda::granularity(tf).is_some()),
            Source::Ratio { .. } | Source::Synth { .. } => iv.source().is_some(),
            Source::Cap { .. } => iv.secs() >= argos_data::cmcap::EVERY_S && iv.secs() % argos_data::cmcap::EVERY_S == 0,
            Source::Dex { .. } => iv.secs() >= 60 && iv.secs() % argos_data::dex::gt_step(iv.secs()).2 == 0,
        }
    }

    /// Change de symbole (watchlist, ↑ / ↓) : sauvegarde l'ancien, ouvre le nouveau.
    pub(crate) fn switch_symbol(&mut self, ex: Exchange, base: String) {
        if matches!(&self.chart.source, Source::Live { exchange, base: b } if *exchange == ex && *b == base) {
            return self.redraw();
        }
        self.save();
        self.write_cache();
        self.open_live(ex, base);
        self.redraw();
    }

    /// Suite d'un clic ou d'une touche dans « Aller à » : rester, fermer, ou aller à la date / la plage.
    pub(crate) fn goto_outcome(&mut self, out: Outcome) {
        match out {
            Outcome::Stay => {}
            Outcome::Close => (self.ui.goto, self.ui.goto_replay) = (None, false),
            Outcome::Go(target) => {
                self.ui.goto = None;
                // 📅 date de départ du replay du Simulateur
                if std::mem::take(&mut self.ui.goto_replay) {
                    let t = match target {
                        Target::At(t) | Target::Range(t, _) => t,
                    };
                    return self.sim_replay_depuis(t, None);
                }
                let at = |t| goto::index_at_time(&self.chart.bars, t);
                let action = match target {
                    Target::At(t) => at(t).map(ChartAction::CenterOn),
                    Target::Range(a, b) => at(a).zip(at(b)).map(|(i, j)| ChartAction::ShowRange(i, j)),
                };
                if let Some(a) = action {
                    return self.run_action(a);
                }
            }
        }
        self.redraw();
    }

    /// Au démarrage : mode libre (toute la chart bouge), cadré sur les bougies d'aujourd'hui (pas sur les bornes de la
    /// dernière session, le prix a pu bouger). Le double-clic sur la chart revient au mode normal (prix auto).
    pub(crate) fn start_free(&mut self) {
        if self.chart.bars.is_empty() {
            return;
        }
        (self.chart.settings.scale.manual, self.chart.settings.scale.lock_ratio) = (None, None);
        self.run_action(ChartAction::AutoScale);
    }
}
