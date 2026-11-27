//! 📈 Indicateurs : calcul, MTF, réglages, légende.

use crate::*;

impl App {
    /// 🧩 Divergences MTF de l'AI Trend (comme `request.security_lower_tf` du Pine) : charge les bougies des slots actifs plus petits
    /// que la chart, à l'UT de l'exchange qui les divise (5h = 1h regroupées ; une semaine = des jours, `lower_source`), assez pour
    /// couvrir la chart (3000 au plus), relues à la minute.
    pub(crate) fn refresh_mtf(&mut self) {
        use argos_render::indicators::{values, Ind};
        use argos_script::indicators::ai_trend;
        let (Some(proxy), Source::Live { exchange, base }) = (self.proxy.clone(), &self.chart.source) else {
            return;
        };
        let chart = self.chart.interval.secs();
        let need = if self.ind.on.contains(&Ind::AiTrend) {
            ai_trend::lower_needed(&ai_trend::Settings::from_inputs(&values(&self.ind.set, Ind::AiTrend)), chart)
        } else {
            vec![]
        };
        if need.is_empty() {
            self.feeds.mtf_bars.clear();
            self.feeds.mtf_at = None;
            return;
        }
        let key = format!("{}-{base}-{chart}-{need:?}", exchange.name());
        if self.feeds.mtf_at.as_ref().is_some_and(|(k, t)| *k == key && t.elapsed().as_secs() < 60) {
            return;
        }
        self.feeds.mtf_at = Some((key.clone(), Instant::now()));
        let (exchange, base, span) = (*exchange, base.clone(), self.chart.bars.len() as i64 * chart);
        argos_data::rt::spawn(move || async move {
            let mut out = ai_trend::LowerBars::new();
            for sec in need {
                let Some(tf) = lower_source(sec) else {
                    continue;
                };
                let limit = (span / tf.seconds()).clamp(100, DEFAULT_CANDLES as i64) as usize;
                if let Ok(sup) = Supervisor::new(argos_data::rt::Client::default()).candles(exchange, &base, tf, limit).await {
                    out.insert(
                        sec,
                        if tf.seconds() == sec {
                            sup.bars
                        } else {
                            argos_core::aggregate(&sup.bars, sec)
                        },
                    );
                }
            }
            let _ = proxy.send_event(UserEvent::MtfBars(key, out));
        });
    }

    /// 🔬 The lower-timeframe bars the scripts ask for (`request.security_lower_tf`, `Output::lower_tf_asked`, plus
    /// `extra`: a script just run in the editor), loaded like AI Trend's (same loader, own key): enough to cover the
    /// last 500 chart bars, at most 10 000 per timeframe; then, every 60 s, the newest 200 merged in. Only from the
    /// chart's own exchange (a fallback's bars would not add up to the chart's). 3 failures for a key: stop until it
    /// changes. Nothing on a chart that is not a live exchange market: the scripts stay on their fallback.
    pub(crate) fn refresh_scripts_mtf(&mut self, extra: &[i64]) {
        use crate::state::{EtatPetites, ScriptMtf};
        let chart = self.chart.interval.secs();
        let mut need: Vec<i64> = self
            .ind
            .out
            .scripts
            .iter()
            .flat_map(|v| v.out.lower_tf_asked.iter().copied())
            .chain(extra.iter().copied())
            .filter(|s| *s > 0 && *s < chart)
            .collect();
        need.sort_unstable();
        need.dedup();
        let (Some(proxy), Source::Live { exchange, base }, false) = (self.proxy.clone(), &self.chart.source, need.is_empty()) else {
            self.feeds.script_bars = Default::default();
            self.feeds.script_mtf = None;
            return;
        };
        let key = format!("{}-{base}-{chart}-{need:?}", exchange.name());
        if self.feeds.script_mtf.as_ref().is_none_or(|m| m.key != key) {
            self.feeds.script_bars = Default::default();
            self.feeds.script_mtf = Some(ScriptMtf {
                key: key.clone(),
                exchange: *exchange,
                at: None,
                en_cours: false,
                echecs: 0,
                etats: need.iter().map(|s| (*s, EtatPetites::Chargement)).collect(),
            });
        }
        let Some(m) = self.feeds.script_mtf.as_mut() else { return };
        if m.en_cours || m.echecs >= 3 || m.at.is_some_and(|t| t.elapsed().as_secs() < 60) {
            return;
        }
        (m.at, m.en_cours) = (Some(Instant::now()), true);
        // (a timeframe already there: only the newest 200; otherwise enough for the last 500 chart bars, 10 000 at most)
        let couvrir = self.chart.bars.len().min(500) as i64 * chart;
        let demandes: Vec<(i64, usize)> = need
            .iter()
            .map(|s| {
                let n = if self.feeds.script_bars.contains_key(s) {
                    200
                } else {
                    (couvrir / s).clamp(1, MAX_PETITES as i64) as usize
                };
                (*s, n)
            })
            .collect();
        let (exchange, base) = (*exchange, base.clone());
        argos_data::rt::spawn(move || async move {
            let mut out = vec![];
            for (sec, n) in demandes {
                let Some(tf) = lower_source(sec) else {
                    out.push((sec, Err("unité".to_string())));
                    continue;
                };
                let limit = n * (sec / tf.seconds()) as usize;
                let r = match Supervisor::new(argos_data::rt::Client::default()).candles(exchange, &base, tf, limit).await {
                    Ok(sup) if sup.source == exchange && !sup.bars.is_empty() => Ok(if tf.seconds() == sec {
                        sup.bars
                    } else {
                        argos_core::aggregate(&sup.bars, sec)
                    }),
                    Ok(sup) => Err(format!("{} a répondu", sup.source.name())),
                    Err(e) => Err(e.to_string()),
                };
                out.push((sec, r));
            }
            let _ = proxy.send_event(UserEvent::ScriptBars(key, out));
        });
    }

    /// 🔬 The scripts' lower-timeframe bars arrived (dropped when the key changed meanwhile): merged by time, the
    /// scripts run again once; the editor open on a script that asks for them says how it went.
    pub(crate) fn script_bars_recues(&mut self, key: String, res: Vec<(i64, Result<Vec<argos_core::Bar>, String>)>) {
        use crate::state::EtatPetites;
        let Some(m) = self.feeds.script_mtf.as_mut().filter(|m| m.key == key) else {
            return;
        };
        m.en_cours = false;
        let tout = Arc::make_mut(&mut self.feeds.script_bars);
        let mut rate = false;
        let mut finies = vec![];
        for (sec, r) in res {
            match r {
                // (the editor's line once per loading: the first answer, or a change from loaded to unavailable — not
                // at every 60 s refresh of the newest 200)
                Ok(v) => {
                    let n = fusionner(tout.entry(sec).or_default(), v);
                    if !matches!(m.etats.insert(sec, EtatPetites::Chargees(n)), Some(EtatPetites::Chargees(_))) {
                        finies.push(sec);
                    }
                }
                Err(_) => {
                    rate = true;
                    if !tout.contains_key(&sec) && m.etats.insert(sec, EtatPetites::Indisponibles) != Some(EtatPetites::Indisponibles) {
                        finies.push(sec);
                    }
                }
            }
        }
        m.echecs += rate as u32;
        self.ind.key = Default::default();
        // (the editor open on a script that asks for them: one line each)
        let demande = |sec: i64| {
            self.ui
                .editor
                .as_ref()
                .and_then(|e| e.slot)
                .and_then(|k| self.scripts.get(k))
                .and_then(|s| self.ind.out.scripts.iter().find(|v| v.name == s.name))
                .is_some_and(|v| v.out.lower_tf_asked.contains(&sec))
        };
        let lignes: Vec<_> = finies.into_iter().filter(|s| demande(*s)).filter_map(|s| self.ligne_petites(s)).collect();
        let now = now_s() + self.tz_offset_s();
        if let Some(e) = self.ui.editor.as_mut() {
            for (m, k) in lignes {
                e.log(now, m, k);
            }
        }
        self.redraw();
    }

    /// 🔬 The editor's line about the bars of one lower timeframe (D6).
    pub(crate) fn ligne_petites(&self, sec: i64) -> Option<(String, argos_render::editor::LogKind)> {
        use crate::state::EtatPetites;
        use argos_render::editor::LogKind;
        let u = unite_petites(sec);
        let etat = self.feeds.script_mtf.as_ref().and_then(|m| m.etats.get(&sec).copied());
        let bourse = match (&self.feeds.script_mtf, &self.chart.source) {
            (Some(m), _) => m.exchange.name().to_string(),
            (None, Source::Live { exchange, .. }) => exchange.name().to_string(),
            (None, _) => "ce marché".to_string(),
        };
        Some(match etat {
            Some(EtatPetites::Chargement) => (format!("⏳ Bougies {u} en chargement pour request.security_lower_tf…"), LogKind::Info),
            Some(EtatPetites::Chargees(n)) => (
                format!("✅ Bougies {u} chargées ({n}) : le script utilise les vraies petites bougies."),
                LogKind::Ok,
            ),
            Some(EtatPetites::Indisponibles) | None => (
                format!("⚠ Bougies {u} indisponibles sur {bourse} : le script reste sur son calcul de repli."),
                LogKind::Warn,
            ),
        })
    }

    /// 📜 The top edge of a user script's pane under `y`, like a native band's (`mouse.rs` asks: the cursor).
    pub(crate) fn script_bord(&self, y: f64) -> bool {
        self.ind.out.script_edge(self.chart.viewport.plot_h(), y).is_some()
    }

    /// 📜 Pressed on a user script pane's top edge: the drag starts (`true`), like a native band's.
    pub(crate) fn script_bord_appui(&mut self, y: f64) -> bool {
        let Some(v) = self.ind.out.script_edge(self.chart.viewport.plot_h(), y).map(|k| &self.ind.out.scripts[k]) else {
            return false;
        };
        if v.id.is_empty() {
            return false;
        }
        self.pointer.script_drag = Some((v.id.clone(), y, v.band_h()));
        self.set_cursor(winit::window::CursorIcon::RowResize);
        true
    }

    /// 📜 The grabbed edge follows the mouse: up = a taller pane, between `BAND_MIN` and `BAND_MAX` (80 % of the plot
    /// at most), kept in the chart settings by script id. `false` when no script edge is held.
    pub(crate) fn script_bord_glisse(&mut self, y: f64) -> bool {
        use argos_render::indicators::{BAND_MAX, BAND_MIN};
        let Some((id, y0, h0)) = self.pointer.script_drag.clone() else {
            return false;
        };
        let max = BAND_MAX.min(self.chart.viewport.plot_h() * 0.8);
        self.chart.settings.script_bands.insert(id, (h0 + y0 - y).clamp(BAND_MIN, max));
        self.set_cursor(winit::window::CursorIcon::RowResize);
        self.redraw();
        true
    }

    /// 📈 Recalcule les indicateurs natifs quand les bougies ou la liste changent (au plus 2 fois par seconde en direct).
    /// 🎯 What the indicator results come from (`fond.rs`: the chart image is kept while it stays the same).
    pub(crate) fn ind_sig(&self) -> crate::fond::IndSig {
        crate::fond::IndSig {
            at: self.ind.at,
            hidden: self.ind.out.hidden.clone(),
            favs: self.ind.out.favs.clone(),
            scripts: self.ind.out.scripts.iter().map(|v| v.hidden).collect(),
        }
    }

    pub(crate) fn refresh_indicators(&mut self) {
        let last = self.chart.bars.last();
        let key = (
            self.chart.bars.len(),
            last.map_or(0, |b| b.time),
            last.map_or(0, |b| b.close.to_bits()),
            self.ind.on.clone(),
        );
        if key == self.ind.key {
            return;
        }
        // 📱 cell : la chart d'abord (les 2 premières images sans indicateurs), puis les indicateurs
        if self.top.phone && self.ui.frames < 2 && !self.chart.bars.is_empty() {
            if let Some(w) = &self.window {
                w.request_redraw();
            }
            return;
        }
        let same_shape = key.0 == self.ind.key.0 && key.1 == self.ind.key.1 && key.3 == self.ind.key.3;
        // la dernière bougie bouge : on recalcule au plus toutes les 0,5 s (📱 cell : toutes les 1,5 s, moins de calcul)
        let pas_ms = if self.top.phone { 1500 } else { 500 };
        if same_shape && self.ind.at.elapsed().as_millis() < pas_ms {
            if let Some(w) = &self.window {
                w.request_redraw(); // la dernière bougie bouge : on recalcule un peu plus tard
            }
            return;
        }
        let now = now_s();
        self.refresh_mtf();
        let old = std::mem::take(&mut self.ind.out.scripts);
        let reel = self.orderflow_chart();
        self.ind.out = argos_render::indicators::compute(
            &self.chart.bars,
            self.chart.interval.secs(),
            &self.ind.on,
            &self.ind.set,
            self.ind.out.tape.take(),
            now,
            &self.feeds.mtf_bars,
            &reel,
        );
        self.fp_calculer();
        self.ind.out.hidden = self.ind.leg.1.clone();
        self.ind.out.favs = self.ind.favs.clone();
        self.ind.out.texts = self.ind.texts.clone();
        // 🧱 Argos Murs: its legend (exchanges, count, books); not on a chart without an order book (not crypto)
        self.ind.out.murs = self.murs_titre();
        // 🔥 the books the heatmap reads, in its legend
        self.ind.out.carnets = self.heat_on().then(|| self.carnets_titre()).flatten();
        if self.book_coin().is_none() {
            self.ind.out.on.retain(|i| *i != argos_render::indicators::Ind::ArgosMurs);
        }
        let opts = self.options_scripts(&reel);
        self.ind.out.scripts = self.run_scripts(old, &opts);
        // (🔬 the lower-timeframe bars the scripts asked for: loaded in the background, the scripts run again then)
        self.refresh_scripts_mtf(&[]);
        let mods = self.run_modules(); // 🧩 après les scripts : leurs courbes et leur légende
        self.ind.out.scripts.extend(mods);
        self.plein_ecran_sans_panneaux();
        self.explication_a_jour();
        self.ind.key = key;
        self.ind.at = Instant::now();
        // 🎯 the Stratégie volet follows the strategies just run
        self.strategie_a_jour();
    }

    /// ⚙️ Ouvre les réglages d'un indicateur (valeurs gardées, sinon celles de v1).
    pub(crate) fn open_ind_dlg(&mut self, i: argos_render::indicators::Ind) {
        let vals = argos_render::indicators::values(&self.ind.set, i);
        // sources possibles : les sorties des autres indicateurs de la chart (input.source)
        let on: Vec<_> = self.ind.on.iter().copied().filter(|x| *x != i).collect();
        let sources = argos_render::indicators::source_choices(&on);
        let texts = self.ind.texts.get(&i).cloned().unwrap_or_default();
        let mut dl = argos_render::inddlg::IndDialog::new(i, vals, self.win_w(), self.area.1 as f64 + self.top_h())
            .with_texts(texts)
            .with_sources(sources)
            .with_mes_defauts(Self::mes_defauts(i));
        // 📱 cell et tablette : la feuille plein écran du modèle de Jee (TradingView mobile)
        if self.top.phone {
            let h = self.top_h() + self.area.1 as f64 + argos_render::ecrans::dock_h();
            dl = dl.en_cell(self.win_w(), h);
        }
        self.ui.ind_dlg = Some(dl);
    }

    /// Réponse de la fenêtre de réglages d'indicateur : les valeurs s'appliquent en direct, Annuler remet celles d'avant.
    pub(crate) fn ind_dlg_result(&mut self, dl: argos_render::inddlg::IndDialog, c: argos_render::inddlg::IndClick) {
        use argos_render::inddlg::IndClick;
        // 📍 la fenêtre se cache : le prochain clic sur la chart donne le prix / l'heure
        if let IndClick::Here(i) = c {
            self.banner = Some("📍 Clique sur la chart pour choisir le point (Échap = annuler)".into());
            self.ui.ind_here = Some((dl, i));
            return;
        }
        // Défauts ▸ Enregistrer comme défaut : repris chaque fois que l'indicateur est ajouté
        if c == IndClick::SaveDefaults && dl.script.is_none() {
            Self::enregistrer_mes_defauts(dl.ind, &dl.vals, &dl.texts);
            self.banner = Some(format!("✓ Réglages enregistrés comme défaut pour {}", dl.ind.name()));
        }
        let (vals, texts) = match c {
            IndClick::Close(false) => (dl.backup.clone(), dl.texts_backup.clone()),
            _ => (dl.vals.clone(), dl.texts.clone()),
        };
        // ⚙ a user script (step 3): its values by name under its id
        if let Some((id, specs)) = dl.script.clone() {
            let choisies = argos_script::lang::reglages::choisies(&specs, &vals, &texts);
            let m: std::collections::BTreeMap<String, argos_render::catalogue::ValeurScript> = choisies.iter().map(|(k, v)| (k.clone(), v.into())).collect();
            // (the titles this version of the script no longer has stay: changing the code keeps the rest)
            let deja = self.ind.scripts_set.entry(id).or_default();
            if m.iter().any(|(k, v)| deja.get(k) != Some(v)) {
                deja.extend(m);
                self.ind.key = Default::default();
                self.garder_reglages();
            }
            if !matches!(c, IndClick::Close(_)) {
                self.ui.ind_dlg = Some(dl);
            }
            return;
        }
        if self.ind.texts.get(&dl.ind).cloned().unwrap_or_default() != texts {
            self.ind.texts.insert(dl.ind, texts);
            self.ind.key = Default::default();
            Self::write_prefs("indicator_texts.json", &self.ind.texts);
        }
        if argos_render::indicators::values(&self.ind.set, dl.ind) != vals {
            self.ind.set.insert(dl.ind, vals);
            self.ind.key = Default::default(); // recalcul tout de suite
            self.garder_reglages();
        }
        if !matches!(c, IndClick::Close(_)) {
            self.ui.ind_dlg = Some(dl);
        }
    }

    /// 🏷️ Clic dans la légende des indicateurs.
    pub(crate) fn legend_click(&mut self, c: argos_render::indleg::LegClick) {
        use argos_render::indleg::LegClick;
        match c {
            LegClick::Toggle => {
                self.ind.leg.0 = !self.ind.leg.0;
                self.ind.sel = None;
            }
            // 📱 un nom : sa barre d'actions (encore : fermée)
            LegClick::Nom(i) => self.ind.sel = if self.ind.sel == Some(Ok(i)) { None } else { Some(Ok(i)) },
            LegClick::NomScript(k) => self.ind.sel = if self.ind.sel == Some(Err(k)) { None } else { Some(Err(k)) },
            LegClick::Plus(_) | LegClick::PlusScript(_) => {
                let r = Rect::new(8.0, self.top_h() + 100.0, self.win_w() - 16.0, 1.0);
                return self.open_list(ListKind::IndPlus, r);
            }
            LegClick::Eye(i) => {
                match self.ind.leg.1.iter().position(|x| *x == i) {
                    Some(k) => _ = self.ind.leg.1.remove(k),
                    None => self.ind.leg.1.push(i),
                }
                self.ind.out.hidden = self.ind.leg.1.clone();
            }
            // 🏦 the Footprint's ⚙: its exchanges panel (« ⚙ Tous les réglages » in it opens the full window)
            LegClick::Settings(argos_render::indicators::Ind::Footprint) => return self.fp_basculer(),
            LegClick::Settings(i) => self.open_ind_dlg(i),
            // (📱 the action bar closes after the tap, framed or not: the notice says which)
            LegClick::Cadrer(_) => {
                self.ind.sel = None;
                self.murs_cadrer();
            }
            LegClick::Remove(i) => self.toggle_indicator(i),
            // ☆ favori : le même que le cœur du Marketplace
            LegClick::Fav(i) => {
                match self.ind.favs.iter().position(|x| *x == i) {
                    Some(k) => _ = self.ind.favs.remove(k),
                    None => self.ind.favs.push(i),
                }
                self.ind.out.favs = self.ind.favs.clone();
                Self::write_prefs("market_fav.json", &self.ind.favs);
            }
            // ✨ Argos AI sur cet indicateur : comment le lire et à quoi sert chacun de ses réglages (valeurs actuelles)
            LegClick::Ai(i) => {
                // 🤖 without a Claude key: the rules (level 1) — Lens explains itself; another indicator shows its name,
                // its settings and its rule-based reading, never another card
                if self.sans_cle_claude() {
                    return if i == argos_render::indicators::Ind::Lens {
                        self.expliquer_regles()
                    } else {
                        self.regles_indicateur(i)
                    };
                }
                let q = Self::question_indicateur(i, &argos_render::indicators::values(&self.ind.out.inputs, i));
                if !self.ui.ai_open {
                    self.toggle_ai();
                }
                return self.ai_send(q);
            }
            // { } son code (Pine de Jee) dans l'éditeur, en lecture seule
            LegClick::Code(i) => return self.code_indicateur(i),
            LegClick::Move(i) => self.banner = Some(format!("✥ Déplacer {} vers un autre panneau : bientôt", i.name())),
            // 🖥 ↑ ↓ : le panneau monte / descend parmi les autres ; ▾ : replié (22) / rouvert (interface-2.json)
            LegClick::Monter(i) | LegClick::Descendre(i) => {
                let mut v: Vec<argos_render::indicators::Ind> = self.ind.out.bands(0.0).into_iter().map(|(b, _)| b).collect();
                if let Some(p) = v.iter().position(|x| *x == i) {
                    // (du bas vers le haut : monter = un rang plus loin)
                    let q = if matches!(c, LegClick::Monter(_)) {
                        (p + 1).min(v.len() - 1)
                    } else {
                        p.saturating_sub(1)
                    };
                    v.swap(p, q);
                }
                argos_render::indicators::set_ordre(&v);
                Self::interface2_changer(|s| s.panneaux_ordre = v);
            }
            LegClick::Replier(i) => {
                let mut v = argos_render::indicators::replies();
                match v.iter().position(|x| *x == i) {
                    Some(k) => _ = v.remove(k),
                    None => v.push(i),
                }
                argos_render::indicators::set_replies(&v);
                Self::interface2_changer(|s| s.panneaux_replies = v);
            }
            // 📜 script : index parmi les scripts de la chart → sa case dans Mes scripts
            LegClick::ScriptEye(k) | LegClick::ScriptSettings(k) | LegClick::ScriptCode(k) | LegClick::ScriptRemove(k) => {
                let Some(slot) = self.scripts.iter().enumerate().filter(|(_, s)| s.on).nth(k).map(|(j, _)| j) else {
                    if matches!(c, LegClick::ScriptSettings(_)) {
                        self.banner = Some("⚙ Ce module n'a pas de réglages ici".into());
                        return self.redraw();
                    }
                    // 🧩 après les scripts : les modules démarrés
                    let n = self.scripts.iter().filter(|s| s.on).count();
                    return self.mod_legend(c, k.saturating_sub(n));
                };
                match c {
                    LegClick::ScriptEye(_) => {
                        self.scripts[slot].hidden = !self.scripts[slot].hidden;
                        if let Some(v) = self.ind.out.scripts.get_mut(k) {
                            v.hidden = self.scripts[slot].hidden;
                        }
                    }
                    LegClick::ScriptCode(_) => self.editor_on(Some(slot), None),
                    LegClick::ScriptSettings(_) => return self.reglages_du_script(slot, k),
                    _ => {
                        self.scripts[slot].on = false;
                        self.ind.key = Default::default();
                        // (📋 its tables' places and sizes go with it)
                        let id = self.scripts[slot].id.clone();
                        self.tab_oublier_script(&id);
                    }
                }
                Self::write_prefs("scripts.json", &self.scripts);
            }
        }
        self.legende_garder();
        self.redraw();
    }

    /// 🏷️ Garde la légende : les indicateurs masqués (👁) dans indicator_legend.json (partagé) ; la liste ouverte ou fermée
    /// (⌄) du nouvel écran dans interface-2.json (indicator_legend.json garde celle de l'ancienne /app/).
    pub(crate) fn legende_garder(&self) {
        let ancien: (bool, Vec<argos_render::indicators::Ind>) = Self::read_prefs("indicator_legend.json");
        Self::write_prefs("indicator_legend.json", &(ancien.0, self.ind.leg.1.clone()));
        let ouverte = self.ind.leg.0;
        Self::interface2_changer(|i| i.legende_ouverte = Some(ouverte));
    }

    /// 📈 Active / coupe un indicateur natif (gardé pour le prochain lancement).
    pub(crate) fn toggle_indicator(&mut self, i: argos_render::indicators::Ind) {
        if let Some(k) = self.ind.on.iter().position(|x| *x == i) {
            self.ind.on.remove(k);
            // (📋 its tables' places and sizes go with it)
            self.tab_oublier_ind(i);
        } else {
            self.ind.on.push(i);
            // Défauts ▸ Enregistrer comme défaut : l'indicateur ajouté part de ces réglages
            if let Some((vals, texts)) = Self::mes_defauts(i) {
                self.ind.set.insert(i, vals);
                self.ind.texts.insert(i, texts);
                Self::write_prefs("indicator_texts.json", &self.ind.texts);
                self.garder_reglages();
                self.ind.key = Default::default();
            }
            // inputs à confirmer (Pine `confirm = true`) : la fenêtre « Confirmer les inputs » s'ouvre tout de suite
            let vals = argos_render::indicators::values(&self.ind.set, i);
            if let Some(dl) = argos_render::inddlg::IndDialog::confirm(i, vals, self.win_w(), self.area.1 as f64 + self.top_h()) {
                self.ui.ind_dlg = Some(dl);
            }
        }
        Self::write_prefs("indicators.json", &self.ind.on);
        self.redraw();
    }

    /// 📱 ⋯ d'un indicateur (photos de Jee, TradingView mobile) : le menu complet ; ce qui n'existe pas encore est
    /// grisé « bientôt ».
    pub(crate) fn ind_plus_menu(&self) -> Vec<MenuGroup> {
        let nom = match self.ind.sel {
            Some(Ok(i)) => i.name().to_string(),
            Some(Err(k)) => self.ind.out.scripts.get(k).map_or_else(String::new, |v| v.title().to_string()),
            None => String::new(),
        };
        let natif = matches!(self.ind.sel, Some(Ok(_)));
        let fav = matches!(self.ind.sel, Some(Ok(i)) if self.ind.favs.contains(&i));
        let cache = match self.ind.sel {
            Some(Ok(i)) => self.ind.out.hidden.contains(&i),
            Some(Err(k)) => self.ind.out.scripts.get(k).is_some_and(|v| v.hidden),
            None => false,
        };
        let it = |id: i64, icon: &'static str, label: String, ok: bool| MenuItem {
            id,
            label: if ok { label } else { format!("{label} · bientôt") },
            icon: Some(icon),
            enabled: ok,
            ..Default::default()
        };
        vec![
            MenuGroup {
                title: nom.clone(),
                items: vec![
                    // ✨ Argos AI on the phone and the tablet: the same as the legend's ✨ on the PC
                    it(12, "✨", format!("Argos AI : lire {nom}"), natif),
                    // { } its code (Pine, Argos Script or its Rust source, read only), as the PC legend's { }
                    it(
                        13,
                        "braces",
                        format!("Code {}", argos_render::text::de(&nom)),
                        matches!(self.ind.sel, Some(Ok(i)) if argos_render::sources::a_un_code(i)),
                    ),
                    it(1, "⏰", format!("Ajouter une alerte pour {nom}"), true),
                    it(2, "📈", format!("Ajouter un indicateur/une stratégie à {nom}"), false),
                    it(
                        3,
                        if fav { "★" } else { "☆" },
                        if fav {
                            "Supprimer cet indicateur des favoris".into()
                        } else {
                            "Ajouter cet indicateur aux favoris".into()
                        },
                        natif,
                    ),
                ],
                strip: false,
            },
            MenuGroup {
                title: String::new(),
                items: vec![
                    it(4, "↕", "Ordre de visualisation ›".into(), false),
                    it(5, "⏱", "Visibilité des intervalles ›".into(), false),
                    it(6, "⇕", "Déplacer vers ›".into(), natif),
                    it(7, "📌", "Épingler à l'échelle (maintenant à droite) ›".into(), false),
                ],
                strip: false,
            },
            MenuGroup {
                title: String::new(),
                items: vec![
                    it(8, "📄", "Copier".into(), false),
                    it(9, "👁", if cache { "Afficher".into() } else { "Cacher".into() }, true),
                    it(10, "🗑", "Retirer".into(), true),
                    it(11, "🌳", "Arborescence d'objets…".into(), true),
                ],
                strip: false,
            },
        ]
    }

    /// 📱 Choix dans le menu ⋯ d'un indicateur.
    pub(crate) fn ind_plus_pick(&mut self, id: i64) {
        use argos_render::indleg::LegClick;
        let Some(sel) = self.ind.sel else { return };
        let c = match (id, sel) {
            (1, _) => {
                let r = Rect::new(8.0, self.top_h() + 60.0, self.win_w() - 16.0, 1.0);
                return self.open_list(ListKind::Alerts, r);
            }
            (12, Ok(i)) => LegClick::Ai(i),
            (13, Ok(i)) => LegClick::Code(i),
            (3, Ok(i)) => LegClick::Fav(i),
            (6, Ok(i)) => LegClick::Move(i),
            (9, Ok(i)) => LegClick::Eye(i),
            (9, Err(k)) => LegClick::ScriptEye(k),
            (10, Ok(i)) => LegClick::Remove(i),
            (10, Err(k)) => LegClick::ScriptRemove(k),
            (11, _) => return self.open_tree(),
            _ => return self.redraw(),
        };
        if matches!(id, 10) {
            self.ind.sel = None;
        }
        self.legend_click(c);
    }

    /// 🅘 Ce que montre la page Indicateurs.
    pub(crate) fn fx_donnees(&self) -> argos_render::fxpage::Donnees {
        argos_render::fxpage::Donnees {
            actifs: self.ind.on.clone(),
            favoris: self.ind.favs.clone(),
        }
    }

    /// 🅘 Toucher dans la page Indicateurs (position de la fenêtre).
    /// 🖥 PC : ƒx ouvre (ou ferme) le volet Indicateurs de la maquette — la page Indicateurs (recherche, Personnel,
    /// Intégrés) dans le volet flottant ; la chart s'arrête à son bord comme pour les autres volets.
    pub(crate) fn fx_volet(&mut self) {
        let ouvert = self.ui.fx_page.is_some() && self.watch.prefs.open && self.watch.prefs.panel == wl::Tool::Indicators;
        if ouvert {
            self.ui.fx_page = None;
            self.watch.prefs.open = false;
        } else {
            self.ui.fx_page = Some(argos_render::fxpage::FxPage {
                volet: true,
                ..Default::default()
            });
            self.ui.fx_dans_volet = true;
            (self.watch.prefs.open, self.watch.prefs.panel) = (true, wl::Tool::Indicators);
            self.watch.haut_du_volet();
        }
        self.save_watch();
        self.redraw();
    }

    /// 🖥 Place de la page Indicateurs : le volet flottant sur PC ; `None` : toute la fenêtre (cell, tablette).
    pub(crate) fn fx_rect(&self) -> Option<Rect> {
        self.volet_rect()
            .filter(|_| self.ui.fx_dans_volet && self.watch.prefs.panel == wl::Tool::Indicators)
    }

    pub(crate) fn fx_click(&mut self) {
        use argos_render::fxpage::FxClick;
        // (🖥 dans le volet : ses coordonnées et sa largeur)
        let (x, y) = self.pointer.win_mouse;
        let (x, y, w) = match self.fx_rect() {
            Some(v) => (x - v.x, y - v.y, v.w),
            None => (x, y, self.win_w()),
        };
        let d = self.fx_donnees();
        let volet = self.fx_rect().is_some();
        let Some(p) = self.ui.fx_page.as_mut() else { return };
        match p.click(x, y, w, &d) {
            FxClick::None | FxClick::Changed => {}
            FxClick::Fermer => {
                self.ui.fx_page = None;
                if volet {
                    self.watch.prefs.open = false;
                    self.save_watch();
                }
            }
            FxClick::Ajouter(i) => return self.toggle_indicator(i),
            FxClick::Favori(i) => return self.legend_click(argos_render::indleg::LegClick::Fav(i)),
            FxClick::Script(o) => {
                self.ui.fx_page = None;
                if volet {
                    self.watch.prefs.open = false;
                }
                self.open_editor(o);
            }
            FxClick::Bientot(m) => self.banner = Some(m.to_string()),
        }
        self.redraw();
    }
}

impl App {
    /// ✨ La question pour Argos AI sur un indicateur (bouton rond de sa légende, demande de Jee) : comment le lire, et
    /// à quoi sert chacun de ses réglages, avec ses valeurs actuelles (et l'aide « ? » de chaque réglage).
    pub(crate) fn question_indicateur(i: argos_render::indicators::Ind, vals: &[f64]) -> String {
        let mut q = format!(
            "Explique-moi l'indicateur « {} » tel qu'il est sur ma chart : 1) comment l'interpréter (ses lignes, couleurs, signaux) ; 2) à quoi sert chacun de ses réglages, avec mes valeurs actuelles :",
            i.name()
        );
        let params = i.params().1;
        if params.is_empty() {
            q.push_str("\n- (aucun réglage)");
        }
        for (k, p) in params.iter().enumerate() {
            let v = vals.get(k).copied().unwrap_or(p.def);
            let v = if v.fract() == 0.0 { format!("{v:.0}") } else { format!("{v}") };
            q.push_str(&format!("\n- {} = {v}", p.label));
            if !p.tip.is_empty() {
                q.push_str(&format!(" (aide : {})", p.tip.replace('\n', " ")));
            }
        }
        q
    }
}

/// ⚙ Chosen values of the user scripts' inputs: script id → input title → value.
pub(crate) type ScriptsSet = std::collections::BTreeMap<String, std::collections::BTreeMap<String, argos_render::catalogue::ValeurScript>>;

/// 📚 Indicator settings saved by name (step 2 of the Argos Script engine).
/// - `indicator_settings.json` (by name, versioned) is the source.
/// - On the first launch without it, it is made from `indicator_inputs.json` (by position); that old file is first
///   copied as it is to `indicator_inputs.avant-noms.json` (never erased).
/// - While the old /app/ exists, `indicator_inputs.json` is written too (by position), so /app/ keeps working. One way
///   only: interface-2 is the source; a change /app/ makes to `indicator_inputs.json` after the conversion is not
///   imported back.
pub(crate) const REGLAGES_NOMS: &str = "indicator_settings.json";
pub(crate) const REGLAGES_POSITIONS: &str = "indicator_inputs.json";
pub(crate) const REGLAGES_AVANT: &str = "indicator_inputs.avant-noms.json";
/// ⚙ Défauts ▸ « Enregistrer comme défaut » : les réglages de départ choisis par l'utilisateur, par indicateur (id du
/// catalogue) puis par nom d'input, comme `indicator_settings.json` ; les textes à part.
pub(crate) const MES_DEFAUTS: &str = "indicator_defaults.json";

/// Contenu de `indicator_defaults.json` (les ids et noms inconnus de cette version restent quand on le réécrit).
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub(crate) struct MesDefauts {
    pub(crate) format: u32,
    pub(crate) indicateurs: std::collections::BTreeMap<String, std::collections::BTreeMap<String, f64>>,
    pub(crate) textes: std::collections::BTreeMap<String, std::collections::HashMap<String, String>>,
}

/// 📜 The name of the dated copy made before a conversion: `indicator_inputs.json`, « 2026-11-19 » →
/// `indicator_inputs.avant-conversion-2026-11-19.json`.
/// The exchange's candles that make a lower timeframe of `sec` seconds (AI Trend's slots, the scripts'
/// `request.security_lower_tf`): the chart's own rule, `Interval::source`. A week is made of days, from Monday, like
/// the chart's (the weeks of Hyperliquid, Gate.io, Kraken and KuCoin begin on Thursday: seen on 24 Nov. 2026);
/// otherwise the exchange's interval that divides it (5 h = 1 h regrouped).
pub(crate) fn lower_source(sec: i64) -> Option<Timeframe> {
    Interval(sec).source()
}

pub(crate) fn nom_avant_conversion(nom: &str, date: &str) -> String {
    let base = nom.strip_suffix(".json").unwrap_or(nom);
    format!("{base}.avant-conversion-{date}.json")
}

impl App {
    /// The indicator settings (by position, for the engine) from `indicator_settings.json`, converting the old file
    /// the first time.
    pub(crate) fn lire_reglages() -> (argos_render::indicators::Settings, ScriptsSet) {
        use argos_render::catalogue::ReglagesParNom;
        if let Some(raw) = platform::read(REGLAGES_NOMS) {
            if let Ok(mut r) = serde_json::from_str::<ReglagesParNom>(&raw) {
                // 📜 natives replaced by their script: their old keys renamed once, after a dated copy of both files
                if r.convertir() {
                    Self::copie_avant_conversion(REGLAGES_NOMS, &raw);
                    if let Some(p) = platform::read(REGLAGES_POSITIONS) {
                        Self::copie_avant_conversion(REGLAGES_POSITIONS, &p);
                    }
                    Self::write_prefs(REGLAGES_NOMS, &r);
                }
                return (r.par_position(), r.scripts);
            }
        }
        let ancien = platform::read(REGLAGES_POSITIONS);
        if let Some(raw) = &ancien {
            if platform::read(REGLAGES_AVANT).is_none() {
                let _ = platform::write(REGLAGES_AVANT, raw);
            }
        }
        let mut set: argos_render::indicators::Settings = ancien.as_ref().and_then(|raw| serde_json::from_str(raw).ok()).unwrap_or_default();
        if argos_render::catalogue::convertir_positions(&mut set) {
            if let Some(raw) = &ancien {
                Self::copie_avant_conversion(REGLAGES_POSITIONS, raw);
            }
        }
        Self::write_prefs(REGLAGES_NOMS, &ReglagesParNom::default().avec(&set));
        (set, Default::default())
    }

    /// 📜 Before the settings of a native replaced by its script are converted: the file `nom` copied as it is next to
    /// it, with the date (`indicator_inputs.json` → `indicator_inputs.avant-conversion-2026-11-19.json`); an existing
    /// copy is never overwritten.
    fn copie_avant_conversion(nom: &str, raw: &str) {
        let copie = nom_avant_conversion(nom, &chrono::Local::now().format("%Y-%m-%d").to_string());
        if platform::read(&copie).is_none() {
            let _ = platform::write(&copie, raw);
        }
    }

    /// ⚙ The settings saved with Défauts ▸ « Enregistrer comme défaut » for `i` (by position), and its texts.
    pub(crate) fn mes_defauts(i: argos_render::indicators::Ind) -> Option<(Vec<f64>, std::collections::HashMap<String, String>)> {
        let f: MesDefauts = Self::read_prefs(MES_DEFAUTS);
        let id = argos_render::catalogue::fiche(i).id;
        let noms = f.indicateurs.get(id)?;
        let vals = argos_script::catalogue::par_position(argos_render::catalogue::reglages(i).1, noms);
        Some((vals, f.textes.get(id).cloned().unwrap_or_default()))
    }

    /// ⚙ Défauts ▸ « Enregistrer comme défaut » : `vals` and `texts` become the starting settings of `i`.
    pub(crate) fn enregistrer_mes_defauts(i: argos_render::indicators::Ind, vals: &[f64], texts: &std::collections::HashMap<String, String>) {
        let mut f: MesDefauts = Self::read_prefs(MES_DEFAUTS);
        f.format = 1;
        let id = argos_render::catalogue::fiche(i).id.to_string();
        f.indicateurs
            .insert(id.clone(), argos_script::catalogue::par_nom(argos_render::catalogue::reglages(i).1, vals));
        f.textes.insert(id, texts.clone());
        Self::write_prefs(MES_DEFAUTS, &f);
    }

    /// Saves the indicator settings: by name (keeping what the file has that this version does not know), and by
    /// position for the old /app/.
    pub(crate) fn garder_reglages(&self) {
        use argos_render::catalogue::ReglagesParNom;
        let mut r = Self::read_prefs::<ReglagesParNom>(REGLAGES_NOMS).avec(&self.ind.set);
        r.scripts = self.ind.scripts_set.clone();
        r.tableaux = self.ind.tables.clone();
        Self::write_prefs(REGLAGES_NOMS, &r);
        // (the old /app/ knows only the built-ins before the Argos Script ones: never write the others there)
        let ancien: argos_render::indicators::Settings = self
            .ind
            .set
            .iter()
            .filter(|(i, _)| (**i as usize) < argos_script::catalogue::AVANT_SCRIPTS)
            .map(|(i, v)| (*i, argos_render::catalogue::positions_anciennes(*i, v)))
            .collect();
        Self::write_prefs(REGLAGES_POSITIONS, &ancien);
    }
}

impl App {
    /// ⚙ Gives every user script a stable id (step 3), and saves the list when one was missing.
    pub(crate) fn ids_des_scripts(&mut self) {
        let mut nouveau = false;
        for s in self.scripts.iter_mut().filter(|s| s.id.is_empty()) {
            let mut o = [0u8; 8];
            let _ = getrandom::fill(&mut o);
            s.id = o.iter().map(|b| format!("{b:02x}")).collect();
            nouveau = true;
        }
        if nouveau {
            Self::write_prefs("scripts.json", &self.scripts);
        }
    }

    /// ⚙ Opens the settings window of the user script `slot` (its `k`-th view on the chart), made from its input()
    /// calls (step 3): the same window as the indicators, its values saved by name under the script's id.
    pub(crate) fn reglages_du_script(&mut self, slot: usize, k: usize) {
        let Some(v) = self.ind.out.scripts.get(k) else { return };
        let specs = &v.out.inputs;
        // (B-071: a table is kept for good, so their total has a ceiling)
        let table = if specs.is_empty() {
            Err("⚙ Ce script n'a pas d'input() : rien à régler")
        } else {
            use argos_script::lang::reglages::Plein;
            argos_script::lang::reglages::table_du_script(specs).map_err(|plein| match plein {
                Plein::Script => "⚙ Les réglages de ce script prennent trop de place pour être ouverts",
                Plein::Session => "⚙ Trop de versions de réglages ouvertes depuis le lancement : relance Argos pour régler ce script",
            })
        };
        let (groups, params) = match table {
            Ok(t) => t,
            Err(bandeau) => {
                self.banner = Some(bandeau.into());
                return self.redraw();
            }
        };
        let s = &self.scripts[slot];
        let (vals, textes) = argos_script::lang::reglages::valeurs(specs, &self.valeurs_du_script(&s.id));
        let mut dl = argos_render::inddlg::IndDialog::script(
            s.id.clone(),
            specs.clone(),
            &s.name,
            groups,
            params,
            vals,
            self.win_w(),
            self.area.1 as f64 + self.top_h(),
        )
        .with_texts(textes);
        if self.top.phone {
            let h = self.top_h() + self.area.1 as f64 + argos_render::ecrans::dock_h();
            dl = dl.en_cell(self.win_w(), h);
        }
        self.ui.ind_dlg = Some(dl);
        self.redraw();
    }

    /// ⚙ The chosen values of a user script, as the script engine takes them (input title → value).
    pub(crate) fn valeurs_du_script(&self, id: &str) -> std::collections::HashMap<String, argos_script::lang::InVal> {
        self.ind
            .scripts_set
            .get(id)
            .map_or_else(Default::default, |m| m.iter().map(|(k, v)| (k.clone(), v.into())).collect())
    }
}

/// 🔬 Most lower-timeframe bars kept per timeframe for the scripts (10 000 one-minute bars on Hyperliquid: 2 requests
/// of about 105 points out of the 1 200 a minute of the whole address — do not raise it).
pub(crate) const MAX_PETITES: usize = 10_000;

/// Merges `neuves` (oldest first) into `v` by time: the bars of `v` older than the first new one stay, the new ones
/// follow; the newest `MAX_PETITES` are kept. Gives the count.
pub(crate) fn fusionner(v: &mut Vec<argos_core::Bar>, neuves: Vec<argos_core::Bar>) -> usize {
    if let Some(t) = neuves.first().map(|b| b.time) {
        v.retain(|b| b.time < t);
        v.extend(neuves);
    }
    let trop = v.len().saturating_sub(MAX_PETITES);
    v.drain(..trop);
    v.len()
}

/// « 1 min », « 5 min », « 1 h », « 1 j » : a lower timeframe in the editor's lines.
pub(crate) fn unite_petites(sec: i64) -> String {
    match sec {
        s if s % 86_400 == 0 => format!("{} j", s / 86_400),
        s if s % 3600 == 0 => format!("{} h", s / 3600),
        s if s % 60 == 0 => format!("{} min", s / 60),
        s => format!("{s} s"),
    }
}
