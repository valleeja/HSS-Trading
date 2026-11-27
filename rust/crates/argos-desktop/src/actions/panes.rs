//! 🪟 Split : les cases en plus de la chart principale.

use crate::*;

impl App {
    /// 🪟 Une case par place de la disposition (hors principale) ; synchros ; charge ce qui manque.
    pub(crate) fn sync_panes(&mut self) {
        let n = argos_render::splits::cells(&self.split.layout).len().saturating_sub(1);
        let main = self.main_coin();
        // nouvelles cases : le même symbole, des unités de temps plus grandes (vue multi-UT, comme v1)
        const DEFAULTS: [i64; 6] = [3600, 14_400, 86_400, 900, 300, 604_800];
        while self.panes.len() < n {
            let k = self.panes.len();
            let saved = self.split.panes.get(k).cloned();
            let used: Vec<i64> = std::iter::once(self.chart.interval.secs())
                .chain(self.panes.iter().map(|p| p.interval.secs()))
                .collect();
            let secs = saved
                .as_ref()
                .map(|s| s.1)
                .unwrap_or_else(|| DEFAULTS.into_iter().find(|s| !used.contains(s)).unwrap_or(3600));
            self.panes.push(Pane::new(saved.map_or(main.clone(), |s| s.0), Interval(secs)));
        }
        self.panes.truncate(n);
        for k in 0..self.panes.len() {
            // 🔗 case liée à la principale (ou ⇄ Symbole synchronisé pour toutes) : le même symbole
            if self.split.sync_symbol || self.split.liees.get(k).copied().unwrap_or(false) {
                self.panes[k].coin = main.clone();
            }
            if self.split.sync_interval {
                self.panes[k].interval = self.chart.interval;
            }
            if self.panes[k].asked != self.panes[k].key() {
                self.load_pane(k);
            }
            self.pane_passe(k);
        }
        // gardées pour le prochain lancement (symbole et unité de temps de chaque case)
        let now: Vec<(String, i64)> = self.panes.iter().map(|p| (p.coin.clone(), p.interval.secs())).collect();
        if !now.is_empty() && now != self.split.panes[..now.len().min(self.split.panes.len())] {
            self.split.panes = now;
            Self::write_prefs("split.json", &self.split);
        }
    }

    /// 🪟 Ce que les autres cases suivent : la plage de temps visible de la chart principale (⟷ Temps), et le point du
    /// réticule (heure + prix) avec sa hauteur relative (✚ Réticule ; un autre symbole garde seulement la hauteur).
    pub(crate) fn split_follow(&self) -> (Option<(i64, i64)>, Option<(argos_render::drawings::Point, f64)>) {
        let bars = &self.chart.bars;
        let range = self.split.sync_time.then(|| {
            let s = argos_render::slots(bars, &self.chart.styles[self.chart.style]);
            let (a, b) = self.chart.viewport.visible(&s)?;
            Some((bars.get(a)?.time, bars.get(b)?.time))
        });
        let cross = self.split.sync_crosshair.then(|| {
            let (x, y) = self.pointer.mouse?;
            let vp = &self.chart.viewport;
            if x < 0.0 || y < 0.0 || x > vp.plot_w() || y > vp.plot_h() {
                return None;
            }
            let f = self.frame_now()?;
            Some((f.geo().point_at(x, y), y / vp.plot_h().max(1.0)))
        });
        (range.flatten(), cross.flatten())
    }

    /// 🪟 Charge les bougies d'une case : fichier (regroupé à son unité de temps) ou exchange (cache, puis en arrière-plan).
    pub(crate) fn load_pane(&mut self, k: usize) {
        let p = &mut self.panes[k];
        p.asked = p.key();
        p.bars.clear();
        (p.zone, p.charge) = (None, false);
        let secs = p.interval.secs();
        if p.coin.starts_with("file:") {
            p.bars = if secs > self.chart.file_interval.secs() {
                aggregate(&self.chart.raw, secs)
            } else {
                self.chart.raw.clone()
            };
            return;
        }
        let Some((ex, base)) = p.coin.split_once(':').and_then(|(e, b)| Exchange::parse(e).map(|e| (e, b.to_string()))) else {
            return;
        };
        let Some(tf) = p.interval.source() else { return }; // secondes : pas d'historique pour une case
        if let Some(cached) = cache_dir()
            .map(|d| d.join(cache::file_name(ex, &base, tf)))
            .as_deref()
            .and_then(|f| cache::read(f, tf, now_s()))
        {
            p.bars = if Interval::from(tf) == p.interval { cached } else { aggregate(&cached, secs) };
        }
        let (key, label, iv) = (p.key(), p.interval.short(), p.interval);
        if let Some(proxy) = self.proxy.clone() {
            p.charge = true;
            argos_data::rt::spawn(move || async move {
                // 🚦 file commune, basse priorité ; 🤝 deux cases sur le même marché et la même unité : une seule demande
                let cle = format!("{}|{base}|{}", ex.id(), tf.id());
                let res = argos_data::file::basse(ex, || {
                    argos_data::file::partage(&cle, || async { load_live(ex, &base, tf, &label, 400).await.map(|c| c.bars) })
                })
                .await
                .map(|bars| if Interval::from(tf) == iv { bars } else { aggregate(&bars, iv.secs()) });
                let _ = proxy.send_event(UserEvent::Pane(key, res));
            });
        }
    }

    /// ⏪ During a replay, a pane needs its bars around the replay's hour (300 before it, to read the past), and the
    /// loaded ones (the latest) may not reach that far back: they are asked once per stretch of 100 bars of the pane.
    /// When the replay ends, the pane goes back to the latest bars.
    fn pane_passe(&mut self, k: usize) {
        let heure = self.replay_heure();
        let p = &self.panes[k];
        if p.coin.starts_with("file:") {
            return;
        }
        let Some(h) = heure else {
            if p.zone.is_some() {
                self.load_pane(k);
            }
            return;
        };
        let sec = p.interval.secs();
        let couvert = p.bars.first().is_some_and(|b| b.time <= h - 300 * sec) && p.bars.last().is_some_and(|b| argos_core::bucket_end(b.time, sec) >= h);
        let pas = 100 * sec;
        let avant = (h.div_euclid(pas) + 3) * pas;
        if couvert || p.charge || p.zone == Some(avant) {
            return;
        }
        self.load_pane_zone(k, avant);
    }

    /// ⏪ Loads the bars of a pane before `avant` (about 500 of them: up to 300 before the replay's hour, the rest after).
    fn load_pane_zone(&mut self, k: usize, avant: i64) {
        let p = &mut self.panes[k];
        let Some((ex, base)) = p.coin.split_once(':').and_then(|(e, b)| Exchange::parse(e).map(|e| (e, b.to_string()))) else {
            return;
        };
        let Some(tf) = p.interval.source() else { return };
        p.zone = Some(avant);
        let Some(proxy) = self.proxy.clone() else { return };
        p.charge = true;
        let (key, iv) = (p.demande(), p.interval);
        let n = (500 * iv.secs() / tf.seconds()).clamp(100, DEFAULT_CANDLES as i64) as usize;
        argos_data::rt::spawn(move || async move {
            let feed = argos_data::ExchangeFeed::new(ex, argos_data::rt::Client::default());
            // 🚦 file commune, basse priorité
            let res = argos_data::file::basse(ex, || feed.older(&base, tf, avant, n))
                .await
                .map(|bars| if Interval::from(tf) == iv { bars } else { aggregate(&bars, iv.secs()) })
                .map_err(|e| e.to_string());
            let _ = proxy.send_event(UserEvent::Pane(key, res));
        });
    }

    /// ⏪ The bars a pane shows: during a replay, only what is known at the replay's hour (the bar in progress rebuilt
    /// from the main chart's finer bars when it shows the same market), otherwise all of them.
    pub(crate) fn pane_bars(&self, k: usize) -> std::borrow::Cow<'_, [Bar]> {
        let p = &self.panes[k];
        let Some(heure) = self.replay_heure() else {
            return std::borrow::Cow::Borrowed(&p.bars);
        };
        let (sec, fin) = (p.interval.secs(), self.chart.interval.secs());
        let fines: &[Bar] = if p.coin == self.main_coin() && argos_core::se_regroupe(fin, sec) {
            &self.chart.bars
        } else {
            &[]
        };
        std::borrow::Cow::Owned(argos_core::au_replay(&p.bars, sec, fines, heure))
    }

    /// 🪟 La case (hors principale) sous ce point de la zone : (case, x et y dans la case, largeur, hauteur).
    pub(crate) fn pane_at(&self, x: f64, y: f64) -> Option<(usize, f64, f64, f64, f64)> {
        let cells = argos_render::splits::cells(&self.split.layout);
        let (aw, ah) = (self.area.0 as f64, self.area.1 as f64);
        cells
            .iter()
            .skip(1)
            .enumerate()
            .take(self.panes.len())
            .find(|(_, c)| x >= c[0] * aw && x < (c[0] + c[2]) * aw && y >= c[1] * ah && y < (c[1] + c[3]) * ah)
            .map(|(k, c)| (k, x - c[0] * aw, y - c[1] * ah, (c[2] * aw).round(), (c[3] * ah).round()))
    }

    /// 🪟 La vue d'une case de `w` × `h` : sa vue à elle (zoom, défilement), sinon la plage de la principale (⟷ Temps),
    /// sinon les dernières bougies. Rend aussi les places des bougies.
    pub(crate) fn pane_view(&self, k: usize, w: f64, h: f64, range: Option<(i64, i64)>) -> (Viewport, argos_render::viewport::Slots) {
        self.pane_view_de(k, &self.pane_bars(k), w, h, range)
    }

    /// 🪟 [`Self::pane_view`] on the bars the pane shows (`bars`, from [`Self::pane_bars`]).
    pub(crate) fn pane_view_de(&self, k: usize, bars: &[Bar], w: f64, h: f64, range: Option<(i64, i64)>) -> (Viewport, argos_render::viewport::Slots) {
        let p = &self.panes[k];
        let mut vp = Viewport {
            width: (w as u32).max(1),
            height: (h as u32).max(1),
            ..Viewport::default()
        };
        let s = argos_render::slots(bars, &self.chart.styles[self.chart.style]);
        if let Some((sp, off)) = p.vue {
            (vp.bar_spacing, vp.right_offset) = (sp, off);
        } else if let Some((t0, t1)) = range.filter(|_| !bars.is_empty()) {
            let i = bars.partition_point(|b| b.time < t0);
            let j = bars.partition_point(|b| b.time <= t1).saturating_sub(1).max(i);
            vp.show_range(i, j, &s);
        }
        (vp, s)
    }

    /// 📏 Les réglages d'une case : ceux de la principale, mais l'échelle de prix toujours ajustée à ses bougies (pas
    /// les bornes fixées ni le rapport verrouillé de la principale, qui écrasaient les petites unités de temps).
    pub(crate) fn pane_settings(&self) -> argos_render::settings::ChartSettings {
        let mut s = self.chart.settings.clone();
        (s.scale.manual, s.scale.lock_ratio) = (None, None);
        s
    }

    /// 🖱️ Molette sur une case : zoom autour de la souris, dans cette case seulement.
    pub(crate) fn pane_wheel(&mut self, k: usize, steps: f64, lx: f64, w: f64, h: f64) {
        let (range, _) = self.split_follow();
        let (mut vp, s) = self.pane_view(k, w, h, range);
        vp.zoom_at(1.15f64.powf(steps), lx, &s);
        self.panes[k].vue = Some((vp.bar_spacing, vp.right_offset));
        self.redraw();
    }

    /// 🖱️ Glisser dans une case : les bougies de cette case seulement suivent la souris.
    pub(crate) fn pane_pan(&mut self, k: usize, dx: f64, w: f64, h: f64) {
        let (range, _) = self.split_follow();
        let (mut vp, s) = self.pane_view(k, w, h, range);
        vp.scroll_px(dx, &s);
        self.panes[k].vue = Some((vp.bar_spacing, vp.right_offset));
        self.redraw();
    }

    /// ✚ Souris sur la zone : le réticule de la case survolée (les autres l'oublient).
    pub(crate) fn pane_hover(&mut self, x: f64, y: f64) {
        let at = self.pane_at(x, y);
        let mut changed = false;
        for (k, p) in self.panes.iter_mut().enumerate() {
            let c = at.filter(|a| a.0 == k).map(|a| (a.1, a.2));
            changed |= p.croix != c;
            p.croix = c;
        }
        if changed {
            self.redraw();
        }
    }

    /// 📱 Cell et tablette : de combien le titre d'une case du HAUT descend (0 ailleurs). La rangée du haut de la chart
    /// porte les pilules de la maquette (DÉMO, lignes d'infos, Flux) : le titre, ⇄, 🔗 Liée / Libre et les favoris de
    /// la case passent dessous (bas de la pilule Flux + 6), sinon ils se chevauchent.
    pub(crate) fn pane_dy(&self, k: usize) -> f64 {
        if !(self.top.phone || self.ui.form.tablet()) {
            return 0.0;
        }
        let haut = argos_render::splits::cells(&self.split.layout).get(k + 1).is_some_and(|c| c[1] == 0.0);
        if !haut {
            return 0.0;
        }
        let z = argos_render::ecrans::zones(self.win_w(), self.cell_h(), &self.cell_etat());
        z.iter()
            .find(|(_, c)| *c == argos_render::ecrans::CellHit::Flux)
            .map_or(0.0, |(r, _)| (r.y + r.h - self.top_h() + 6.0).max(0.0))
    }

    /// Les zones du titre d'une case, `dy` plus bas (voir `pane_dy`) : (fin du symbole, fin de l'unité, ⇄ échanger,
    /// 🔗 lier).
    pub(crate) fn pane_title_zones(title: &str, name: &str, liee: bool, dy: f64) -> (f64, f64, Rect, Rect) {
        let text = argos_render::text::Text::default();
        let (wn, wt) = (text.width(name, 16.0) as f64, text.width(title, 16.0) as f64);
        let swap = Rect::new(18.0 + wt, 8.0 + dy, 22.0, 22.0);
        let lw = text.width(Self::pane_link_label(liee), 12.0) as f64 + 16.0;
        (12.0 + wn, 12.0 + wt, swap, Rect::new(swap.x + swap.w + 6.0, 8.0 + dy, lw, 22.0))
    }

    /// ⏱ Les favoris ★ d'une case de largeur `w`, en raccourcis compacts après « 🔗 Liée / Libre » (tant qu'il y a
    /// la place avant l'échelle de prix ; sinon ils restent dans le menu « · 1h »).
    pub(crate) fn pane_favs(&self, k: usize, w: f64) -> Vec<(Rect, i64)> {
        let liee = self.split.sync_symbol || self.split.liees.get(k).copied().unwrap_or(false);
        let (name, title) = self.pane_title(k);
        let dy = self.pane_dy(k);
        let (_, _, _, lien) = Self::pane_title_zones(&title, &name, liee, dy);
        let text = argos_render::text::Text::default();
        let fin = w - Viewport::default().price_axis as f64 - 6.0;
        let mut x = lien.x + lien.w + 8.0;
        let mut out = vec![];
        for &s in &self.tools.favorites {
            let cw = text.width(&Interval(s).short(), 11.5) as f64 + 10.0;
            if x + cw > fin {
                break;
            }
            out.push((Rect::new(x, 9.0 + dy, cw, 20.0), s));
            x += cw + 2.0;
        }
        out
    }

    pub(crate) fn pane_link_label(liee: bool) -> &'static str {
        if liee {
            "🔗 Liée"
        } else {
            "Libre"
        }
    }

    /// Le nom court d'une case et son titre (« ETH · 1h »).
    pub(crate) fn pane_title(&self, k: usize) -> (String, String) {
        let p = &self.panes[k];
        let name = p.coin.rsplit(':').next().unwrap_or(&p.coin);
        let name = std::path::Path::new(name)
            .file_stem()
            .map_or(name.to_string(), |s| s.to_string_lossy().into_owned());
        let title = format!("{name} · {}", p.interval.short());
        (name, title)
    }

    /// 🪟 Clic dans une autre case (coordonnées de la zone). Titre : le symbole ouvre la recherche de symbole pour cette
    /// case, l'unité ouvre le menu des intervalles pour cette case, ⇄ l'échange avec la principale, 🔗 / Libre la lie
    /// (ou la délie) au symbole de la principale. Ailleurs : la case devient active (glisser, double-clic = vue
    /// automatique). `true` : le clic était pour une case.
    pub(crate) fn pane_click(&mut self, x: f64, y: f64) -> bool {
        let Some((k, lx, ly, _, _)) = self.pane_at(x, y) else {
            return false;
        };
        // ⏱ un favori de la case : son unité de temps change (cette case seulement)
        let w = self.pane_at(x, y).map_or(0.0, |p| p.3);
        if let Some((_, secs)) = self.pane_favs(k, w).into_iter().find(|(r, _)| r.contains(lx, ly)) {
            self.pane_set_interval(k, Interval(secs));
            return true;
        }
        let liee = self.split.liees.get(k).copied().unwrap_or(false);
        let (name, title) = self.pane_title(k);
        let dy = self.pane_dy(k);
        let (fin_nom, fin_ut, swap, lien) = Self::pane_title_zones(&title, &name, liee, dy);
        if ly >= dy && ly <= 32.0 + dy && lx <= lien.x + lien.w {
            if swap.contains(lx, ly) {
                let (coin, iv) = (self.panes[k].coin.clone(), self.panes[k].interval);
                (self.panes[k].coin, self.panes[k].interval) = (self.main_coin(), self.chart.interval);
                self.panes[k].vue = None;
                if let Some((ex, base)) = coin.split_once(':').and_then(|(e, b)| Exchange::parse(e).map(|e| (e, b.to_string()))) {
                    self.switch_symbol(ex, base);
                }
                self.set_interval(iv);
            } else if lien.contains(lx, ly) {
                if self.split.liees.len() <= k {
                    self.split.liees.resize(k + 1, false);
                }
                self.split.liees[k] = !liee;
                Self::write_prefs("split.json", &self.split);
                self.banner = Some(if liee {
                    format!("🔓 Case {} libre : choisis son symbole en cliquant sur son nom", k + 2)
                } else {
                    format!("🔗 Case {} liée : elle suit le symbole de la chart principale", k + 2)
                });
                self.sync_panes();
            } else if lx <= fin_nom {
                // 🔎 le symbole de cette case : la watchlist (toute la recherche, tous les exchanges)
                self.ui.pane_symbole = Some(k);
                (self.watch.prefs.open, self.watch.prefs.panel) = (true, argos_render::watchlist::Tool::Watchlist);
                self.banner = Some(format!("🔎 Choisis le symbole de la case {} (Échap : annuler)", k + 2));
            } else if lx <= fin_ut {
                // ⏱ l'unité de temps de cette case
                let r = Rect::new(x - lx + fin_nom, y - ly + 32.0 + dy, 10.0, 1.0);
                self.open_list(ListKind::PaneIntervals(k), r);
                return true;
            }
            self.redraw();
            return true;
        }
        // double-clic dans une case : retour à la vue automatique
        // (B-048: by the finger, the time it tapped, not when the app read it)
        let now = self.pointer.t_geste.unwrap_or_else(Instant::now);
        if matches!(self.pointer.pane_click_at, Some((j, t)) if j == k && self.est_double(Some(t), now)) {
            self.panes[k].vue = None;
            self.pointer.pane_click_at = None;
        } else {
            self.pointer.pane_click_at = Some((k, now));
            self.pointer.pane_drag = Some((k, x));
        }
        self.redraw();
        true
    }

    /// 🔎 Le symbole choisi pour une case (watchlist) : la case seule change ; elle n'est plus liée à la principale.
    pub(crate) fn pane_set_coin(&mut self, k: usize, coin: &str) -> bool {
        // les symboles d'exchange seulement (OANDA, calculés, DEX… : pas encore dans une case)
        let speciaux = ["oanda:", "synth:", "ratio:", "cap:", "dex:", "file:"];
        if k >= self.panes.len() || speciaux.iter().any(|p| coin.starts_with(p)) {
            return false;
        }
        let ex = coin.split_once(':').and_then(|(e, _)| Exchange::parse(e)).unwrap_or(Exchange::Hyperliquid);
        self.panes[k].coin = format!("{}:{}", ex.id(), self.base_of(coin));
        self.panes[k].vue = None;
        if let Some(l) = self.split.liees.get_mut(k) {
            *l = false;
        }
        self.banner = None;
        self.sync_panes();
        Self::write_prefs("split.json", &self.split);
        true
    }

    /// ⏱ L'unité de temps choisie pour une case.
    pub(crate) fn pane_set_interval(&mut self, k: usize, iv: Interval) {
        if let Some(p) = self.panes.get_mut(k) {
            p.interval = iv;
            p.vue = None;
            self.sync_panes();
        }
        self.redraw();
    }
}
