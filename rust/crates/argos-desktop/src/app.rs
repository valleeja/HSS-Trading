//! 🪟 `App` : tout l'état de la fenêtre (en sous-états, voir `state/`), ses petites aides, et le branchement avec
//! winit (`ApplicationHandler`). Le reste du comportement est rangé dans `actions/`, `input/`, `render.rs`, `events.rs`.

use crate::*;

/// Ce que la fenêtre affiche et l'état de la souris.
pub(crate) struct App {
    // — sous-états (state/) —
    /// 📈 La chart du symbole ouvert : source, bougies, type, intervalle, vue, réglages, dessins, direct.
    pub(crate) chart: state::ChartState,
    /// 📡 Flux de marché des panneaux et des modules (prix, carnet, baleines, macro…) : voir `state::Feeds`.
    pub(crate) feeds: state::Feeds,
    /// 📈 Indicateurs natifs (actifs, résultats, réglages, favoris, légende) : voir `state::IndState`.
    pub(crate) ind: state::IndState,
    /// 🖱️ Souris : positions, glisser en cours, derniers clics, curseur, touches enfoncées : voir `state::PointerState`.
    pub(crate) pointer: state::PointerState,
    /// 🪟 Menus, fenêtres, pages et boîtes ouverts par-dessus l'app : voir `state::UiState`.
    pub(crate) ui: state::UiState,
    /// 🧩 Colonne de modules (place, largeurs, contenu, défilement) : voir `state::DockState`.
    pub(crate) dock: state::DockState,
    /// 🔐 Compte, clés et jetons de l'utilisateur (secrets en mémoire seulement) : voir `state::SessionState`.
    pub(crate) session: state::SessionState,
    // — le reste —
    /// Barre d'outils flottante (position, favoris, dernier intervalle).
    pub(crate) tools: toolbar::Prefs,
    /// Barre du haut (logo, mise en page, bandeau, fiche du symbole).
    pub(crate) top: TopBar,
    /// ✨ Assistant (la conversation reste quand on le ferme).
    pub(crate) ai: argos_render::assistant::Assistant,
    /// 💼 Position dessinée liée au ticket démo (index dans `drawings`).
    pub(crate) ticket_draw: Option<usize>,
    /// Barre de gauche (comme Argos v1) : réglages et outil choisi.
    pub(crate) rail: rail::Prefs,
    pub(crate) tool: &'static str,
    /// ⏪ Replay : l'état, l'heure de toutes les bougies et le moment du dernier pas.
    pub(crate) replay: Option<argos_render::replay::Replay>,
    /// 🪟 Zone de chart entière (la chart principale = sa 1re case), le split et ses autres cases.
    pub(crate) area: (u32, u32),
    pub(crate) split: argos_render::splits::Prefs,
    pub(crate) panes: Vec<Pane>,
    pub(crate) replay_times: Vec<i64>,
    pub(crate) replay_at: Instant,
    /// Dernière sauvegarde automatique.
    pub(crate) auto_saved: Instant,
    pub(crate) title: String,
    pub(crate) banner: Option<String>,
    /// Objet sélectionné (poignées visibles ; Suppr l'efface, Échap le désélectionne).
    pub(crate) selected: Option<usize>,
    /// Objet en cours de placement : ses points cliqués + le dernier qui suit la souris (aperçu).
    pub(crate) draft: Option<Drawing>,
    /// Watchlist et barre d'icônes à droite.
    pub(crate) watch: Watchlist,
    /// Symboles dont le logo a déjà été demandé.
    pub(crate) logos_asked: std::collections::HashSet<String>,
    /// Base locale (catalogues des exchanges, watchlists) ; `None` si elle n'a pas pu s'ouvrir.
    pub(crate) store: Option<Store>,
    /// Watchlists de la base, dans leur ordre.
    pub(crate) lists: Vec<WatchList>,
    /// 💼 Compte démo (gardé dans demo_account.json).
    pub(crate) demo: argos_core::portfolio::Portfolio,
    /// 🔔 Alertes et leur historique (gardés dans alerts.json).
    pub(crate) alerts: argos_core::alerts::Alerts,
    /// 🔔 Options choisies pour la prochaine alerte (index dans les délais, dans les validités).
    pub(crate) alerts_draft: (usize, usize),
    /// 💼 Comptes démo (gardés dans demo_accounts.json ; celui choisi vit dans `demo` et le journal).
    pub(crate) accounts: crate::actions::DemoAccounts,
    /// Dessin copié (Ctrl + C / Copier), collé par Ctrl + V.
    pub(crate) obj_clip: Option<Drawing>,
    /// Modèles de style (⋯ → Sauvegarder sous…) : outil → [(nom, style)].
    pub(crate) templates: std::collections::HashMap<String, Vec<(String, Style)>>,
    /// ✏️ Dernier style validé (D'accord) par outil : le prochain objet de cet outil le reprend (`tool_styles.json`).
    pub(crate) tool_styles: std::collections::HashMap<String, Style>,
    /// Section Exchanges : activés, catalogues en téléchargement, exchanges qui ont une clé, boîte de clé ouverte.
    pub(crate) ex_prefs: ExPrefs,
    /// 📜 Scripts Argos (éditeur), gardés dans scripts.json.
    pub(crate) scripts: Vec<SavedScript>,
    /// 🧩 Modules installés et ceux qui tournent.
    pub(crate) mods: crate::actions::ModState,
    /// 🔐 Le journal partagé (base chiffrée du PC) : dernier état connu, prêt ou non, où il est gardé.
    pub(crate) jsync: crate::actions::JournalSync,
    pub(crate) cat_loading: std::collections::HashSet<Exchange>,
    /// Dernière relecture des bougies OANDA (pas de flux en direct chez OANDA : relu aux 15 s).
    pub(crate) oanda_at: Instant,
    /// Génération du symbole ouvert (change à chaque symbole) et lien vers la fenêtre pour les fils d'arrière-plan.
    pub(crate) gen: Arc<AtomicU64>,
    pub(crate) proxy: Option<EventLoopProxy<UserEvent>>,
    /// Presse-papiers (gardé ouvert : sous Linux, le texte copié vit tant que la fenêtre est ouverte).
    pub(crate) clipboard: Option<platform::Clipboard>,
    pub(crate) window: Option<Rc<Window>>,
    pub(crate) surface: Option<softbuffer::Surface<Rc<Window>, Rc<Window>>>,
}

impl App {
    /// 🐢 Redessin demandé par des données en direct : tout de suite après un geste (moins de 2 s) ; au repos, une fois
    /// par seconde au plus (le redessin attendu se fait avec la dernière valeur). Onglet caché : le navigateur ne
    /// dessine rien (pas d'image demandée).
    pub(crate) fn redraw_donnees(&mut self) {
        if !redessin_donnees_retenu(self.ui.derniere_entree.elapsed(), self.ui.dernier_redessin_donnees.elapsed()) {
            self.redraw_donnees_maintenant();
        } else {
            self.ui.redessin_en_attente = true;
        }
    }

    fn redraw_donnees_maintenant(&mut self) {
        self.ui.redessin_en_attente = false;
        self.ui.dernier_redessin_donnees = Instant::now();
        self.redraw();
    }

    pub(crate) fn redraw(&self) {
        if let Some(w) = &self.window {
            w.request_redraw();
        }
        // panneau Marché crypto ouvert (ou module 🧩) : le fil charge le symbole affiché
        if let Ok(mut m) = self.feeds.market_wanted.lock() {
            let docked = self.dock.rect.is_some()
                && self
                    .watch
                    .dock
                    .shown()
                    .iter()
                    .any(|i| matches!(i.module, argos_render::dock::Module::Market | argos_render::dock::Module::Sentiment));
            // 📈 série CRYPTOCAP: ouverte : le marché de CoinGecko (gratuit) continue son historique
            let cap = matches!(self.chart.source, Source::Cap { .. });
            *m = if (self.watch.prefs.open && self.watch.prefs.panel == wl::Tool::Market) || docked {
                Some(self.symbol_label())
            } else {
                cap.then(|| "BTC".to_string())
            };
        }
        // pas pendant un glisser : on sauvegarde quand on relâche
        if self.pointer.drag.is_none()
            && self.pointer.grab.is_none()
            && self.pointer.axis_drag.is_none()
            && self.pointer.band_drag.is_none()
            && self.pointer.band_pan.is_none()
            && self.pointer.band_zoom.is_none()
            && self.pointer.script_pan.is_none()
            && self.pointer.script_zoom.is_none()
        {
            self.track_drawings();
            self.save();
        }
    }

    /// ↶ ↷ Relève les dessins : s'ils ont changé depuis le dernier relevé, l'état d'avant va dans « défaire »
    /// (au plus `HIST_MAX`) et « refaire » est vidé. Appelé quand rien n'est en train d'être glissé : un déplacement
    /// compte pour un seul pas.
    pub(crate) fn track_drawings(&self) {
        use crate::state::HIST_MAX;
        let mut h = self.chart.hist.borrow_mut();
        let now = &self.chart.drawings;
        match &h.base {
            None => h.base = Some(now.clone()),
            Some(b) if b != now => {
                let before = h.base.replace(now.clone()).unwrap_or_default();
                h.undo.push(before);
                if h.undo.len() > HIST_MAX {
                    h.undo.remove(0);
                }
                h.redo.clear();
            }
            Some(_) => {}
        }
    }

    /// Ce qui est gardé entre deux lancements.
    pub(crate) fn to_saved(&self) -> saved::Saved {
        saved::Saved {
            style: self.chart.styles[self.chart.style].name().to_string(),
            bar_spacing: self.chart.viewport.bar_spacing,
            settings: self.chart.settings.clone(),
            drawings: self.chart.drawings.clone(),
        }
    }

    /// Écrit la sauvegarde du symbole, seulement si quelque chose a changé.
    pub(crate) fn save(&self) {
        let Some(name) = &self.chart.save_path else { return };
        let now = self.to_saved();
        if self.chart.last_saved.borrow().as_ref() == Some(&now) {
            return;
        }
        match saved::write(name, &now) {
            Ok(()) => *self.chart.last_saved.borrow_mut() = Some(now),
            Err(e) => eprintln!("⚠️ Sauvegarde impossible ({name}) : {e}"),
        }
        // 🗂 sauvegarde automatique : la mise en page ouverte suit
        if self.top.prefs.auto_save {
            self.garder_layout();
        }
        // 🖍 « Synchronisation globale » (bar ⋯): the other layouts of this symbol get the same synced drawings
        self.sync_globaux();
    }

    /// 🖍 The drawings shown on a split pane of `coin`: those synced in the layout or globally (bar ⋯), when the pane
    /// shows the main chart's symbol; « Pas de sync » keeps a drawing on its own chart.
    pub(crate) fn pane_drawings(&self, coin: &str) -> Vec<Drawing> {
        if coin != self.main_coin() {
            return vec![];
        }
        self.chart.drawings.iter().filter(|d| d.style.sync >= 1).cloned().collect()
    }

    /// Décalage du fuseau choisi (réglage), sinon celui de l'ordinateur.
    /// Décalage du fuseau choisi ; jamais choisi : New York (l'heure des marchés et de l'Académie, comme la
    /// maquette).
    pub(crate) fn tz_offset_s(&self) -> i64 {
        let local = || {
            use chrono::Offset;
            chrono::Local::now().offset().fix().local_minus_utc() as i64
        };
        settings::tz_offset(self.fuseau(), now_s()).unwrap_or_else(local)
    }

    /// Le fuseau de la chart (vide, jamais choisi : New York).
    pub(crate) fn fuseau(&self) -> &str {
        let tz = self.chart.settings.timezone.as_str();
        if tz.is_empty() {
            "America/New_York"
        } else {
            tz
        }
    }

    /// Ce qu'on dessine en ce moment.
    pub(crate) fn to_chart(&self) -> Chart<'_> {
        // 🖥 la carte Argos Lens évite la barre de favoris
        let barre = (!self.top.phone && !self.tools.hidden).then(|| toolbar::layout(&self.tools, &argos_render::text::Text::default()).0);
        argos_render::indicators::set_eviter(barre.into_iter().collect());
        // 🖥 la légende commence sous la bande des rangées du haut repliées (2 à 14 sous la barre du haut) ; 📱 sous la
        // pilule DÉMO
        argos_render::set_haut_legende(self.decalage_legende());
        // 🖥 the navigation pill shows only when the mouse comes into its zone (Jee, 17 Nov); 📱 cell, tablet: unchanged
        argos_render::navbar::set_au_survol(!self.top.phone);
        // 🧘 PC Zen: the information banner under the legend (title, O H L C, Vol), which then starts at the top
        // 📱 cell, the legend's list unfolded (its rows are not reserved on the phone): the banner goes under them
        // (F1, 22 Nov: the ⇕ reason sat on the « Argos Murs » row)
        argos_render::set_bas_bandeau(if self.ui.zen && !self.top.phone {
            40.0 + self.decalage_legende()
        } else if self.top.phone && self.ind.leg.0 {
            (argos_render::indleg::bas_legende(&self.ind.out, true, true) + 6.0 - 40.0).max(0.0)
        } else {
            0.0
        });
        self.plein_ecran_par_image();
        let mut c = self.chart_with(true);
        // ⛶ a panel full screen: it alone, without the price legend
        // (nothing of the prices: the last price, the announcements, alerts, orders, drawings, whales, book)
        if matches!(self.ui.plein_ecran, Some(crate::actions::plein_ecran::PleinEcran::Panneau(_))) {
            (c.title, c.legende) = (None, false);
            (c.settings.scale.last_price_label, c.settings.scale.last_price_line, c.settings.labels.countdown) = (false, false, false);
            (c.events, c.alerts, c.drawings, c.whales) = (vec![], vec![], vec![], vec![]);
            c.alertes_vue.pause.clear();
            (c.ordres, c.bid_ask, c.book_heat, c.liq_map) = (Default::default(), None, None, None);
        }
        c
    }

    /// 📟 Tablet by the finger: top (chart y) of the buttons floating at the bottom of the chart (the symbol pill and
    /// ƒx), where Argos Murs' bottom pills must stay above. `None` on PC, on the phone (its single pill) and when
    /// those buttons are not shown (bottom bar hidden, a sheet open).
    pub(crate) fn boutons_bas_y(&self) -> Option<f64> {
        if !self.top.phone || self.pilule() || !argos_render::actions::tactile() {
            return None;
        }
        let e = self.cell_etat();
        if e.bas_cache || e.feuille {
            return None;
        }
        let m = argos_render::ecrans::cell();
        let (w, h) = (self.win_w(), self.cell_h());
        let y = ["pg", "pd"]
            .iter()
            .filter_map(|id| m.place(id, w, h))
            .map(|r| r.y)
            .fold(f64::INFINITY, f64::min);
        y.is_finite().then(|| y - self.top_h())
    }

    /// 📍 What the app lays over the chart that the high / low pills keep clear of (chart coordinates): the floating
    /// toolbar when it is on the chart (even faded on PC: the pills do not jump as the mouse comes near), and the
    /// legend's unfolded indicator list.
    pub(crate) fn hilo_garder(&self) -> Vec<Rect> {
        let text = argos_render::text::Text::default();
        let mut v = vec![];
        if !self.tools.hidden && !self.pilule() {
            let (bar, btns) = toolbar::layout(&self.tools, &text);
            if !btns.is_empty() {
                v.push(bar);
            }
        }
        if self.ind.leg.0 && !self.top.phone {
            let bas = self.chart.viewport.plot_h() - self.ind.out.osc_h();
            v.extend(
                argos_render::indleg::zones(&text, &self.ind.on, &self.ind.out, true, self.chart.viewport.plot_h())
                    .into_iter()
                    .map(|(r, _)| r)
                    .filter(|r| r.y < bas),
            );
        }
        // 📱 ⚡ a perpetual's Funding pill and its open card
        v.extend(self.funding_garder());
        v
    }

    /// 🟠 What only the pills keep clear of (chart coordinates): the DÉMO band laid over the chart under the top bar
    /// (📟 upright tablet). No wall pill nor high / low pill under it (bug B-018, found with B-017's captures, 25 Nov
    /// 2026: a pill of Argos Murs went half under it); the tables do not move for it: it moves out of their way.
    pub(crate) fn pilules_garder(&self) -> Vec<Rect> {
        if self.ui.accueil.is_some() || self.ui.zen {
            return vec![];
        }
        let (ox, oy) = (self.rw(), self.top_h());
        self.demo_sur_la_chart()
            .filter(|r| r.w > 0.0 && r.y + r.h > oy)
            .map(|r| Rect::new(r.x - ox, r.y - oy, r.w, r.h))
            .into_iter()
            .collect()
    }

    /// 🏷️ The chart's title as drawn: 📱📟 the name alone (« BTC-USDC Perpétuel »), as the mock-up; 🖥 the name and the
    /// candles' type.
    pub(crate) fn titre_chart(&self) -> String {
        if self.pilule() {
            // (📱 the single pill: room kept after the name for the flux dot and the DÉMO label)
            self.pilule_titre_texte()
        } else if self.top.phone {
            self.title.split(" · ").next().unwrap_or(&self.title).to_string()
        } else if argos_render::actions::montrer_raccourcis() {
            format!("{} — {}   (T = changer de type)", self.title, self.chart.styles[self.chart.style].name())
        } else {
            // (👆 a tablet by the finger: no key to press, no key hint)
            format!("{} — {}", self.title, self.chart.styles[self.chart.style].name())
        }
    }

    /// La chart à dessiner ; sans les dessins (`with_drawings` = false) pour les calculs de géométrie (souris), qui
    /// n'en ont pas besoin : on évite de copier tous les dessins à chaque mouvement (ni la place du bandeau DÉMO).
    pub(crate) fn chart_with(&self, with_drawings: bool) -> Chart<'_> {
        // 📋 the tables' layouts, the one hovered / selected, its menu
        self.tableaux_pour_dessin();
        // 🧭 « Voir sur la chart »: the address's lines, for this chart only (its candles)
        argos_render::hlexplorer::set_lignes(self.hl_lignes_vue(), self.chart.bars.as_ptr() as usize);
        Chart {
            bars: &self.chart.bars,
            tf_seconds: self.chart.interval.secs(),
            viewport: self.chart.viewport,
            settings: self.chart.settings.clone(),
            theme: Theme::current(),
            style: self.chart.styles[self.chart.style].clone(),
            tz_offset_s: self.tz_offset_s(),
            // 📱📟 cell et tablette : le nom seul (« BTC-USDC Perpétuel »), comme la maquette
            title: Some(self.titre_chart()),
            // sous la barre flottante ou sa fenêtre, sous la carte Argos AI d'un trade : pas de réticule (ni
            // d'étiquettes de position) (👆 the finger's crosshair stays where it was laid)
            // (n° 35: a finger scrolling a sheet — the alerts' on the phone — is the sheet's: the chart behind does not
            // follow it)
            crosshair: self
                .ui
                .croix
                .map(|(x, y, _)| (x, y))
                .or(self.pointer.mouse.filter(|&(x, y)| {
                    // (under the alerts' sheet or window: what `notifs_sous` says)
                    let sous_alertes = self
                        .ui
                        .notifs
                        .as_ref()
                        .is_some_and(|n| !n.volet || n.pop.is_some_and(|p| p.contains(x + self.rw(), y + self.top_h())));
                    self.pointer.finger_scroll.is_none() && !sous_alertes
                }))
                .filter(|&(x, y)| {
                    x < self.chart.viewport.width as f64
                        && y < self.chart.viewport.height as f64
                        && !toolbar::contains(&self.tools, &argos_render::text::Text::default(), x, y)
                        && !self.ui.pop.as_ref().is_some_and(|p| p.rect.contains(x, y))
                        && !self.avis_sous(x + self.rw(), y + self.top_h())
                        && !self.fp_sous(x + self.rw(), y + self.top_h())
                        && !self.month_under(x + self.rw(), y + self.top_h())
                        // 📋 (tableaux-1 tour 2, F3) a table selected, held, or under the mouse: no crosshair through it
                        && !self.tab_sans_croix(x, y)
                }),
            banner: self.banner.clone(),
            context: self.ui.context,
            drawings: if self.rail.hide_draw || !with_drawings {
                vec![]
            } else {
                self.chart.drawings.iter().chain(&self.draft).cloned().chain(self.ghost()).collect()
            },
            selected: if self.rail.hide_draw {
                None
            } else if self.draft.is_some() {
                Some(self.chart.drawings.len())
            } else {
                self.selected
            },
            now_s: now_s(),
            live: self.chart.live,
            hors_ligne: self.chart.hors_ligne,
            legende: true, // 📐 cell et 🖥 PC (maquette) : O H L C · Vol · Acheteurs / Vendeurs toujours là
            navigation: !self.top.phone,
            goto: self.ui.goto.clone(),
            indicators: Some(&self.ind.out),
            mark: self.pointer.mark_drag,
            whales: self.watch.whales.on_chart(self.top.phone),
            events: self.econ_marks(),
            alerts: self.alerts.lines(&self.symbol_label()).into_iter().map(|(_, p)| p).collect(),
            alertes_vue: self.alertes_vue(),
            bid_ask: (self.chart.settings.labels.bid_ask || self.chart.settings.lines.bid_ask)
                .then_some(self.feeds.book.as_ref())
                .flatten()
                .filter(|b| Some(&b.coin) == self.feeds.flow_coin.as_ref())
                .and_then(|b| {
                    let side = |l: &[argos_data::book::Level]| l.iter().map(|x| (x.px, x.sz)).collect::<Vec<_>>();
                    argos_render::bookheat::best(&side(&b.bids), &side(&b.asks))
                }),
            book_heat: self.heat_on().then_some(self.feeds.heat.as_ref()).flatten(),
            liq_map: self.liq_on().then_some(self.feeds.liq_view.as_ref()).flatten(),
            // 📱 cell : les boutons − + RAZ Zen sont dans la barre des favoris
            pad: (!self.top.phone).then(|| self.pad()),
            trades: self.bt_marks(),
            explication: self.explication_vue(),
            garder: self.hilo_garder(),
            // (🟠 B-018: the DÉMO band's place, for a chart to draw only — the screen's, a capture's: the mouse's
            // geometry (`with_drawings` = false) never needs it; counted at each mouse move, several times, it made the
            // PC measure the top bar's texts again and again: +68 % of work for the PC's chart dragged, gate of 25 Nov)
            garder_pilules: if with_drawings { self.pilules_garder() } else { vec![] },
            boutons_bas: self.boutons_bas_y(),
            // (a Bitfinex chart has its own book: `bitfinex_carnet`)
            book_note: ((self.heat_on() || self.chart.settings.labels.bid_ask || self.chart.settings.lines.bid_ask)
                && self.book_coin().is_none()
                && !matches!(&self.chart.source, Source::Live { exchange, .. } if exchange.bitfinex().is_some()))
            .then_some("📖 Carnet indisponible pour ce symbole"),
            murs: self.murs_ecran(),
            ordres: self.vue_ordres(),
            demo: (self.ui.form.tablet() && !self.top.phone).then(|| self.demo_pastille()),
        }
    }

    /// Ce dont les menus ont besoin.
    pub(crate) fn ctx(&self) -> Ctx<'_> {
        Ctx {
            settings: &self.chart.settings,
            drawings: self.chart.drawings.len(),
            now: now_s(),
            tpo: self.chart.styles[self.chart.style] == argos_render::CandleStyle::Tpo,
        }
    }

    pub(crate) fn frame_now(&self) -> Option<Frame> {
        frame(&self.chart_with(false))
    }

    /// L'échelle de prix affichée.
    pub(crate) fn scale_now(&self) -> Option<PriceScale> {
        current_scale(&self.chart_with(false))
    }

    /// La souris (x de la fenêtre) est-elle sur la watchlist ou la barre d'icônes ?
    pub(crate) fn on_side(&self, x: f64) -> bool {
        x >= self.side_x()
    }

    /// Réglages de l'interface gardés en JSON (dossier de config, ou stockage de la page sur le web).
    pub(crate) fn read_prefs<T: serde::de::DeserializeOwned + Default>(name: &str) -> T {
        platform::read(name).and_then(|raw| serde_json::from_str(&raw).ok()).unwrap_or_default()
    }

    /// Écrit `name` en gardant tels quels les champs `garder` de l'ancien fichier (ceux que l'ancienne /app/ lit et
    /// qu'interface-2 range ailleurs) ; un champ absent de l'ancien fichier reste absent.
    pub(crate) fn write_prefs_garde<T: serde::Serialize>(name: &str, v: &T, garder: &[&str]) {
        let Ok(mut nv) = serde_json::to_value(v) else { return };
        let old: Option<serde_json::Value> = platform::read(name).and_then(|r| serde_json::from_str(&r).ok());
        if let Some(o) = nv.as_object_mut() {
            for k in garder {
                match old.as_ref().and_then(|x| x.get(*k)) {
                    Some(x) => {
                        o.insert((*k).to_string(), x.clone());
                    }
                    None => {
                        o.remove(*k);
                    }
                }
            }
        }
        Self::write_prefs(name, &nv);
    }

    pub(crate) fn write_prefs<T: serde::Serialize>(name: &str, v: &T) {
        let res = serde_json::to_string_pretty(v)
            .map_err(|e| e.to_string())
            .and_then(|raw| platform::write(name, &raw));
        if let Err(e) = res {
            eprintln!("⚠️ Sauvegarde impossible ({name}) : {e}");
        }
    }

    /// Ancre (coordonnées de la fenêtre) d'un rectangle de l'image de la watchlist.
    pub(crate) fn side_rect(&self, r: Rect) -> Rect {
        self.win(r, self.side_x())
    }

    /// Largeur de la barre de gauche, la chart commence après (📱 cell : pas de barre de gauche).
    pub(crate) fn rw(&self) -> f64 {
        if self.top.phone || self.ui.zen {
            0.0
        } else if self.rail.folded {
            // ‹ barre de gauche repliée : 🖥 la place de sa bande de 12 (à 10 du bord), la chart commence après
            self.dock.left + BANDE_W
        } else {
            RAIL_W as f64 + self.dock.left
        }
    }

    /// Height of the right panel's image at `sx` (from its left edge): 🖥 the floating volet's, 📱 the sheet's under
    /// the hinge (two segments one above the other), else the area under the top bar.
    pub(crate) fn side_h(&self, sx: f64) -> f64 {
        match self.volet_rect().filter(|_| sx < self.watch.pw()) {
            Some(v) => v.h,
            None => self
                .ui
                .rows
                .filter(|_| self.top.phone)
                .map_or(self.area.1 as f64, |(_, bot)| self.area.1 as f64 - bot),
        }
    }

    /// Bord gauche du panneau de droite (watchlist) dans la fenêtre : après la chart et la colonne de droite.
    pub(crate) fn side_x(&self) -> f64 {
        self.rw() + self.area.0 as f64 + self.dock.right
    }

    /// 📐 Largeur de la barre du haut : sur un double écran côte à côte (Surface Duo, pliable ouvert), seulement l'écran
    /// de gauche (rien ne passe sous la charnière, rien n'y est coupé) ; sinon toute la fenêtre.
    pub(crate) fn top_w(&self) -> f64 {
        match self.ui.form.split.filter(|_| !self.ui.form.phone()) {
            Some(argos_render::formfactor::Split::Columns { at, .. }) => (at / self.ui.form.zoom.max(0.5)).floor().min(self.win_w()),
            // 🖥 colonne d'icônes de la maquette PC : la barre du haut s'arrête avant elle
            _ if self.colonne_pc().is_some() => self.win_w() - argos_render::watchlist::ICON_BAR as f64,
            _ => self.win_w(),
        }
    }

    /// 🖥 Colonne d'icônes de la maquette PC (du haut au bas de la fenêtre, à droite) : de combien elle monte au-dessus
    /// de la watchlist (jusqu'à 10 du haut). `None` sur cell, en Zen, barres cachées, double écran, barre repliée.
    pub(crate) fn colonne_pc(&self) -> Option<f64> {
        let deux = matches!(self.ui.form.split, Some(argos_render::formfactor::Split::Columns { .. }));
        let ok = !self.top.phone && !self.ui.zen && !self.haut_cache() && !deux && self.watch.fixed_w.is_none() && self.watch.width() > 0;
        ok.then(|| (self.top_h() - 10.0).max(0.0))
    }

    /// Hauteur de la barre du haut : la chart, la barre de gauche et la watchlist commencent en dessous.
    pub(crate) fn top_h(&self) -> f64 {
        if self.ui.zen || self.haut_cache() {
            0.0
        } else if self.top.phone {
            // 📐 cell : la chart commence sous la pilule du haut de la maquette (qui flotte dessus)
            argos_render::ecrans::chart_y()
        } else {
            self.top.height() as f64
        }
    }

    /// 🔍 What the navigation pill needs: Zen mode on (its Zen button lit).
    pub(crate) fn pad(&self) -> argos_render::zoompad::Pad {
        argos_render::zoompad::Pad { zen: self.ui.zen }
    }

    /// 🧘 Mode Zen : allumé / éteint (bouton Zen, Échap pour sortir).
    pub(crate) fn toggle_zen(&mut self) {
        // (⛶ full screen on the PC lies on Zen: the Zen button brings everything back)
        if self.ui.plein_ecran.is_some() {
            return self.quitter_plein_ecran();
        }
        self.ui.zen = !self.ui.zen;
        self.ui.zen_haut = false;
        (self.ui.list, self.ui.pop, self.ui.sheet, self.ui.argos, self.ui.split_menu) = (None, None, None, None, None);
        // (📱 phone view: no keyboard, so no « Échap »)
        let message = if argos_render::actions::montrer_raccourcis() {
            "Mode Zen : Échap ou le bouton Zen pour revenir"
        } else {
            "Mode Zen : le bouton Zen pour revenir"
        };
        self.banner = self.ui.zen.then(|| argos_render::i18n::tr(message).into_owned());
        self.redraw();
    }

    /// Rectangle d'une image sous la barre du haut (barre de gauche `dx` = 0, chart `dx` = rw…) → fenêtre.
    pub(crate) fn win(&self, r: Rect, dx: f64) -> Rect {
        Rect::new(r.x + dx, r.y + self.top_h(), r.w, r.h)
    }

    pub(crate) fn set_window_title(&self) {
        if let Some(w) = &self.window {
            w.set_title(&format!("Argos HSS Trading — {} — {}", self.title, self.chart.styles[self.chart.style].name()));
        }
    }

    /// 🪟 Symbole de la chart principale, écrit comme une case (« hl:BTC », « file:… »).
    pub(crate) fn main_coin(&self) -> String {
        match &self.chart.source {
            Source::Live { exchange, base } => format!("{}:{base}", exchange.id()),
            Source::Oanda { instrument } => format!("oanda:{instrument}"),
            Source::Ratio { exchange, base, quote } => argos_data::tv::ratio_id(*exchange, base, quote),
            Source::Synth { sym } => argos_data::synth::id(sym),
            Source::Cap { series } => argos_data::cmcap::id(series),
            Source::Dex { network, pool, name } => argos_data::dex::id(network, pool, name),
            Source::File(p) => format!("file:{p}"),
        }
    }

    /// Largeur de la fenêtre (barre de gauche + zone de chart + watchlist).
    pub(crate) fn win_w(&self) -> f64 {
        // (🖥🧘 Zen: no side panel, the chart is the whole window — the top bar brought back fits in it)
        if self.ui.zen && !self.top.phone {
            return self.side_x();
        }
        self.side_x() + self.watch.width() as f64
    }

    /// Nom court du symbole pour la barre du bas (« BTC », ou le nom du fichier).
    pub(crate) fn symbol_label(&self) -> String {
        match &self.chart.source {
            Source::Live { base, .. } => base.clone(),
            Source::Oanda { instrument } => instrument.replace('_', ""),
            Source::Ratio { base, quote, .. } => format!("{base}{quote}"),
            Source::Synth { sym } => sym.clone(),
            Source::Cap { series } => series.clone(),
            Source::Dex { name, .. } => name.clone(),
            Source::File(p) => std::path::Path::new(p).file_stem().map_or(p.clone(), |s| s.to_string_lossy().into_owned()),
        }
    }

    /// Change le curseur seulement s'il est différent (évite de le redemander au système à chaque mouvement).
    pub(crate) fn set_cursor(&mut self, c: CursorIcon) {
        if self.pointer.cursor != c || self.pointer.point_pose {
            if let Some(w) = &self.window {
                w.set_cursor(c);
            }
            self.pointer.cursor = c;
            self.pointer.point_pose = false;
        }
    }

    /// 🖱 Pose un curseur : une icône du système, ou le petit point (la croix tant qu'il n'est pas prêt).
    pub(crate) fn poser_curseur(&mut self, c: crate::types::Curseur) {
        match c {
            crate::types::Curseur::Icone(i) => self.set_cursor(i),
            crate::types::Curseur::Point => {
                if !self.pointer.point_pose {
                    if let Some(w) = &self.window {
                        match &self.ui.curseur_point {
                            Some(p) => w.set_cursor(p.clone()),
                            None => w.set_cursor(CursorIcon::Crosshair),
                        }
                    }
                    self.pointer.point_pose = true;
                }
            }
        }
    }

    /// 🖱 Ce que la souris montre en ce moment (pour les tests).
    #[cfg(test)]
    pub(crate) fn curseur_pose(&self) -> crate::types::Curseur {
        if self.pointer.point_pose {
            crate::types::Curseur::Point
        } else {
            crate::types::Curseur::Icone(self.pointer.cursor)
        }
    }

    /// Prix sous la souris (dans la zone des bougies).
    pub(crate) fn price_at_mouse(&self) -> Option<f64> {
        let (x, y) = self.pointer.mouse?;
        let inside = x >= 0.0 && x <= self.chart.viewport.plot_w() && y >= 0.0 && y <= self.chart.viewport.plot_h();
        inside.then(|| self.scale_now().map(|s| s.price_of(y))).flatten()
    }
}

impl ApplicationHandler<UserEvent> for App {
    /// Le direct : la dernière bougie est remplacée, ou une nouvelle commence (l'historique ne change jamais).
    fn user_event(&mut self, _: &ActiveEventLoop, ev: UserEvent) {
        self.on_user_event(ev);
    }

    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title(format!("Argos HSS Trading — {} — {}", self.title, self.chart.styles[self.chart.style].name()))
            .with_window_icon(window_icon());
        // desktop : fenêtre de 1400 × 800 ; navigateur : un canvas ajouté à la page, qui prend toute la page (CSS)
        #[cfg(not(target_arch = "wasm32"))]
        let attrs = attrs.with_inner_size(winit::dpi::LogicalSize::new(1400.0, 800.0));
        #[cfg(target_arch = "wasm32")]
        let attrs = {
            use winit::platform::web::WindowAttributesExtWebSys;
            attrs.with_append(true)
        };
        let window = Rc::new(el.create_window(attrs).expect("fenêtre"));
        let context = softbuffer::Context::new(window.clone()).expect("contexte d'affichage");
        self.surface = Some(softbuffer::Surface::new(&context, window.clone()).expect("surface d'affichage"));
        self.window = Some(window);
        // 🖱 le petit point du curseur « Point » (comme TradingView) : un rond de 5 px, liseré sombre, au centre
        self.ui.curseur_point = winit::window::CustomCursor::from_rgba(point_rgba(), 11, 11, 5, 5)
            .ok()
            .map(|s| el.create_custom_cursor(s));
    }

    /// Compte à rebours : on redessine chaque seconde (seulement s'il est affiché).
    fn new_events(&mut self, _: &ActiveEventLoop, cause: StartCause) {
        self.debut_du_tour();
        if matches!(cause, StartCause::ResumeTimeReached { .. }) {
            self.redraw();
        }
    }

    fn about_to_wait(&mut self, el: &ActiveEventLoop) {
        // 🧩 navigateur : réponses des modules (Worker) et fichiers choisis ; réveil rapide tant qu'on attend
        let busy = self.mods_tick();
        // 🛟 Support: tickets reloaded at startup; image chosen or pasted in the page
        let busy = self.support_tick() || busy;
        // ⚖️ Backtest report: TradingView's file chosen in the page
        let busy = self.rapport_tv_tick() || busy;
        // 🔎 the market panel's hover timer (PC): wake up soon while it waits its 0.3 s
        let busy = self.marches_tick() || busy;
        // 📖 « Copié » on a code block of the Doc: wake up to take it away after 1.5 s
        let busy = self.ui.editor.as_ref().is_some_and(|e| e.doc_ouverte && e.doc.anime()) || busy;
        // 🧱 a tapped Argos Murs tooltip closes by itself after 4 s (F1j): wake up to take it away
        let busy = argos_render::murs::bulle().is_some() || busy;
        // 💾 « Sauvegarde… » affiché : redessiner dès qu'elle est finie (« ✓ Sauvegardé »), réveil bientôt d'ici là
        let saving = platform::sauvegarde_en_cours();
        if self.top.saving != saving {
            self.redraw();
        }
        let busy = busy || saving;
        self.wait_next(el);
        // 🐢 données arrivées au repos : un seul redessin, une seconde après le précédent
        if self.ui.redessin_en_attente {
            let due = self.ui.dernier_redessin_donnees + REPOS_DONNEES;
            if Instant::now() >= due {
                self.redraw_donnees_maintenant();
            } else if !matches!(el.control_flow(), ControlFlow::WaitUntil(t) if t <= due) {
                el.set_control_flow(ControlFlow::WaitUntil(due));
            }
        }
        if busy {
            let soon = Instant::now() + Duration::from_millis(120);
            if !matches!(el.control_flow(), ControlFlow::WaitUntil(t) if t <= soon) {
                el.set_control_flow(ControlFlow::WaitUntil(soon));
            }
        }
    }

    fn window_event(&mut self, el: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        self.on_window_event(el, id, event);
    }
}

impl App {
    /// Prochain réveil de la boucle (appui long, replay, relecture des bougies, compte à rebours).
    fn wait_next(&mut self, el: &ActiveEventLoop) {
        // 🖍 traits de démonstration qui s'effacent : une image toutes les 33 ms jusqu'à la fin du fondu
        if !self.ui.demo_traits.is_empty() {
            el.set_control_flow(ControlFlow::WaitUntil(Instant::now() + Duration::from_millis(33)));
            return;
        }
        // 👆 un doigt posé et immobile : réveil pour l'appui long (clic droit)
        if let Some(at) = self.touch_tick() {
            el.set_control_flow(ControlFlow::WaitUntil(at));
            return;
        }
        // ⏪ replay qui joue : une bougie à chaque période
        if let Some(rp) = self.replay.as_ref().filter(|r| r.play) {
            let period = Duration::from_millis(rp.period_ms());
            if self.replay_at.elapsed() >= period {
                self.replay_step(1);
            }
            el.set_control_flow(ControlFlow::WaitUntil(self.replay_at + period));
            return;
        }
        // 🏦 OANDA : pas de flux en direct, les bougies sont relues aux 15 s (sans bandeau ; 📱 téléphone : à la minute)
        // 🦎 DEX : à la minute (GeckoTerminal : 10 appels par minute pour tout le monde)
        let dex = matches!(self.chart.source, Source::Dex { .. });
        let every = Duration::from_secs(if argos_data::rt::phone() || dex { 60 } else { 15 });
        if matches!(
            self.chart.source,
            Source::Oanda { .. } | Source::Ratio { .. } | Source::Synth { .. } | Source::Dex { .. }
        ) {
            if self.oanda_at.elapsed() >= every && self.chart.loading.is_none() {
                self.oanda_at = Instant::now();
                if let (Some(proxy), Some(tf)) = (self.proxy.clone(), self.source_tf()) {
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
            }
            el.set_control_flow(ControlFlow::WaitUntil(self.oanda_at + every));
            return;
        }
        if self.chart.settings.labels.countdown {
            el.set_control_flow(ControlFlow::WaitUntil(Instant::now() + Duration::from_secs(1)));
        } else {
            el.set_control_flow(ControlFlow::Wait);
        }
    }
}

/// 🐢 Au repos (aucun geste depuis `REPOS_APRES`), les données en direct redessinent au plus une fois par `REPOS_DONNEES`.
pub(crate) const REPOS_APRES: Duration = Duration::from_secs(2);
pub(crate) const REPOS_DONNEES: Duration = Duration::from_secs(1);

/// 🐢 Ce redessin de données attend-il ? (au repos depuis `REPOS_APRES` et le dernier date de moins de `REPOS_DONNEES`)
pub(crate) fn redessin_donnees_retenu(depuis_geste: Duration, depuis_redessin: Duration) -> bool {
    depuis_geste >= REPOS_APRES && depuis_redessin < REPOS_DONNEES
}

/// 🖥 Place de la bande du rail replié (10 + 12 + 4), à gauche de la chart.
pub(crate) const BANDE_W: f64 = 26.0;

/// 🖱 Image du curseur « Point » (11 × 11, RGBA) : un rond clair de 5 px entouré de sombre, lisible sur tout fond.
fn point_rgba() -> Vec<u8> {
    let mut v = vec![0u8; 11 * 11 * 4];
    for y in 0..11 {
        for x in 0..11 {
            let d = ((x as f64 - 5.0).powi(2) + (y as f64 - 5.0).powi(2)).sqrt();
            let (c, a) = if d <= 2.5 {
                (255, 255)
            } else if d <= 3.6 {
                (20, 230)
            } else {
                (0, 0)
            };
            let k = (y * 11 + x) * 4;
            v[k..k + 4].copy_from_slice(&[c, c, c, a]);
        }
    }
    v
}
