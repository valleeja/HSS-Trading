//! ⚡ 📰 📅 The news of the chart's crypto (⚡ on the chart, its card or sheet, « Voir tout › »), the Actualités
//! module's settings, and the Calendrier's card and price reactions.

use crate::*;
use argos_render::calendrier::{Carte, CarteClick};
use argos_render::eclair::{Eclair, EclairClick};

impl App {
    /// 🪙 The crypto of the chart (« BTC » for BTCUSDT.P), `None` for a chart that isn't one of the news' cryptos.
    pub(crate) fn chart_coin(&self) -> Option<&'static str> {
        match &self.chart.source {
            Source::Live { base, .. } => argos_data::actu::coin_of_base(&argos_data::whales::coin_of(base)),
            _ => None,
        }
    }

    /// 📰 The news naming `coin`, newest first (language and sources of the module's settings).
    pub(crate) fn news_of(&self, coin: &str) -> Vec<argos_data::eco::News> {
        let e = &self.watch.eco;
        let mut v: Vec<argos_data::eco::News> = e
            .news
            .iter()
            .filter(|n| n.ts > 0 && !e.sources_off.contains(&n.source))
            .filter(|n| match e.langue {
                1 => n.lang == "fr",
                2 => n.lang != "fr",
                _ => true,
            })
            .filter(|n| argos_data::actu::coins_of(&n.title).contains(&coin))
            .cloned()
            .collect();
        v.sort_by_key(|n| std::cmp::Reverse(n.ts));
        v
    }

    /// ⚡ The marks of the chart's news: one per candle of the timeframe shown (the newest of that candle), only the
    /// news naming the chart's crypto; none for a chart that isn't a crypto.
    pub(crate) fn eclair_marks(&self) -> Vec<argos_render::econ::Mark> {
        let Some(coin) = self.chart_coin() else { return vec![] };
        let tf = self.source_tf().map_or(3600, |t| t.seconds()).max(60);
        let choisi = self.ui.eclair.as_ref().map(|e| e.lien.clone());
        let mut seen = std::collections::HashSet::new();
        self.news_of(coin)
            .into_iter()
            .filter(|n| seen.insert(n.ts - n.ts.rem_euclid(tf)))
            .map(|n| argos_render::econ::Mark {
                ts: n.ts - n.ts.rem_euclid(tf),
                currency: argos_render::econ::NEWS.into(),
                impact: 0,
                choisi: choisi.as_deref() == Some(n.link.as_str()),
                title: n.title,
                numbers: n.source,
                link: n.link,
                window_s: 0,
            })
            .collect()
    }

    /// ⚡ A click on a ⚡ (chart points): its card (PC) or its sheet (phone, tablet).
    pub(crate) fn eclair_ouvrir(&mut self, lien: String, cx: f64) {
        let Some(coin) = self.chart_coin() else { return };
        let symbole = self.top.symbol.rsplit(':').next().unwrap_or(&self.top.symbol).to_string();
        self.ui.eclair = Some(Eclair {
            coin: coin.to_string(),
            symbole,
            news: self.news_of(coin),
            lien,
            ancre: (cx, argos_render::econ::rangee_cy()),
            haut_rangee: argos_render::econ::haut_rangee(argos_render::econ::rangee_bas()),
            plot_w: self.chart.viewport.plot_w(),
            feuille: self.ui.form.phone(),
            scroll: 0.0,
            tz: self.tz_offset_s(),
            now: now_s(),
        });
        self.redraw();
    }

    /// ⚡ The card or sheet and its place (window points).
    pub(crate) fn eclair_draw(&self, text: &argos_render::text::Text) -> Option<(tiny_skia::Pixmap, Rect)> {
        let e = self.ui.eclair.as_ref()?;
        let (w, h) = self.taille_points();
        let m = (!self.pointer.doigt_leve).then_some(self.pointer.win_mouse);
        if e.feuille {
            return e.draw(text, w, h, m);
        }
        let (rw, th) = (self.rw(), self.top_h());
        let (pm, r) = e.draw(text, w, h, m.map(|(x, y)| (x - rw, y - th)))?;
        Some((pm, Rect::new(r.x + rw, r.y + th, r.w, r.h)))
    }

    /// ⚡ A press while the card or sheet is open (window points): `true` if it took it (on the PC a click outside the
    /// card closes it and goes on; the sheet takes every touch).
    pub(crate) fn eclair_press(&mut self, wx: f64, wy: f64) -> bool {
        let Some(e) = self.ui.eclair.as_ref() else { return false };
        let text = argos_render::text::Text::default();
        let (w, h) = self.taille_points();
        let feuille = e.feuille;
        let (x, y) = if feuille { (wx, wy) } else { (wx - self.rw(), wy - self.top_h()) };
        let c = e.click(&text, x, y, w, h);
        let pris = match c {
            EclairClick::None => true,
            EclairClick::Fermer => {
                self.ui.eclair = None;
                feuille
            }
            EclairClick::Ouvrir(l) => {
                platform::open_url(&l);
                true
            }
            EclairClick::VoirTout(c) => {
                self.ui.eclair = None;
                self.actualites_filtrees(&c);
                true
            }
        };
        self.redraw();
        pris
    }

    /// 📰 « Voir tout › »: the Actualités module (panel) filtered on `coin`.
    pub(crate) fn actualites_filtrees(&mut self, coin: &str) {
        let e = &mut self.watch.eco;
        (e.filtre_coin, e.onglet, e.recherche, e.reglages_news) = (Some(coin.to_string()), 0, None, false);
        (self.watch.prefs.open, self.watch.prefs.panel) = (true, wl::Tool::News);
        self.save_watch();
        self.redraw();
    }

    /// 📰 The module's settings changed (sources, language, ⚡ on the chart): kept with the panel's settings, the ⚡
    /// switch with the chart's.
    pub(crate) fn actualites_garder(&mut self) {
        let e = &self.watch.eco;
        let (off, langue, eclairs) = (e.sources_off.clone(), e.langue, e.eclairs);
        if (self.watch.prefs.news_sources_off.clone(), self.watch.prefs.news_langue) != (off.clone(), langue) {
            (self.watch.prefs.news_sources_off, self.watch.prefs.news_langue) = (off, langue);
            self.save_watch();
        }
        if self.chart.settings.econ_news != eclairs {
            self.chart.settings.econ_news = eclairs;
            self.save();
        }
    }

    /// 📅 The card of the release chosen in the Calendrier, and its place (window points): PC, left of the panel;
    /// phone, a sheet.
    pub(crate) fn cal_carte(&self) -> Option<Carte> {
        let i = self.watch.eco.carte?;
        let ev = self.watch.eco.agenda.get(i)?.clone();
        let coin = self.chart_coin().unwrap_or("").to_string();
        let symbole = self.top.symbol.rsplit(':').next().unwrap_or(&self.top.symbol).to_string();
        let (reaction, courbe) = match self.feeds.minutes.as_ref() {
            Some((c, bars)) if !coin.is_empty() && *c == coin => (
                argos_data::reel::reaction(bars, ev.ts),
                bars.iter()
                    .filter(|b| (ev.ts - 1800..=ev.ts + 3600).contains(&b.time))
                    .map(|b| (b.time + 60, b.close))
                    .collect(),
            ),
            _ => (None, vec![]),
        };
        Some(Carte {
            ev,
            coin,
            symbole,
            reaction,
            courbe,
            now: now_s(),
        })
    }

    fn cal_carte_place(&self, c: &Carte, text: &argos_render::text::Text) -> Rect {
        let (w, h) = self.taille_points();
        if self.ui.form.phone() {
            let (cw, ch) = c.taille(text, w);
            // (n° 21: above the Argos AI round, which stays over the sheets: it covered « Voir sur la chart › »)
            let ai = self.ai_bouton_rect();
            let bas = if ai.w > 0.0 && ai.y < h { (ai.y - 6.0).min(h) } else { h };
            return Rect::new(0.0, (bas - ch).max(40.0), cw, ch);
        }
        let (cw, ch) = c.taille(text, argos_render::calendrier::CARTE_W);
        let v = self.volet_rect().unwrap_or(Rect::new(w - 440.0, 80.0, 420.0, 600.0));
        Rect::new((v.x - cw - 12.0).max(8.0), v.y.max(8.0).min((h - ch - 8.0).max(8.0)), cw, ch)
    }

    pub(crate) fn cal_carte_draw(&self, text: &argos_render::text::Text) -> Option<(tiny_skia::Pixmap, Rect)> {
        let c = self.cal_carte()?;
        let r = self.cal_carte_place(&c, text);
        let m = (!self.pointer.doigt_leve).then_some((self.pointer.win_mouse.0 - r.x, self.pointer.win_mouse.1 - r.y));
        Some((c.draw(text, r.w, m)?, r))
    }

    /// 📅 A press on the card (window points): `true` if it took it.
    pub(crate) fn cal_carte_press(&mut self, wx: f64, wy: f64) -> bool {
        let Some(c) = self.cal_carte() else { return false };
        let text = argos_render::text::Text::default();
        let r = self.cal_carte_place(&c, &text);
        if !r.contains(wx, wy) {
            return false;
        }
        match c.click(&text, r.w, wx - r.x, wy - r.y) {
            CarteClick::None => {}
            CarteClick::Fermer => self.watch.eco.carte = None,
            CarteClick::VoirChart(ts) => {
                self.watch.eco.carte = None;
                if self.ui.form.phone() {
                    self.watch.prefs.open = false;
                }
                self.goto_outcome(argos_render::goto::Outcome::Go(argos_render::goto::Target::At(ts)));
            }
        }
        self.redraw();
        true
    }

    /// 📈 The 1-minute candles of the chart's crypto around the past high-impact releases (10 at most, the last day),
    /// read again every 5 min while the Calendrier is shown; then each release's reaction.
    pub(crate) fn cal_minutes(&mut self) {
        let Some(proxy) = self.proxy.clone() else { return };
        let Some(coin) = self.chart_coin() else { return };
        let Source::Live { exchange, base } = &self.chart.source else { return };
        let now = now_s();
        let Some(first) = self.watch.eco.cal_reactions(now).into_iter().find(|t| now - t < 86_400) else {
            return;
        };
        let fresh = self.feeds.minutes_at.as_ref().is_some_and(|(c, t)| c == coin && t.elapsed().as_secs() < 300);
        if fresh {
            return;
        }
        self.feeds.minutes_at = Some((coin.to_string(), Instant::now()));
        let limit = (((now - first + 1800) / 60) as usize + 5).min(1500);
        let (ex, base, coin) = (*exchange, base.clone(), coin.to_string());
        argos_data::rt::spawn(move || async move {
            use argos_core::feed::Feed;
            let feed = argos_data::ExchangeFeed::new(ex, argos_data::rt::Client::default());
            if let Ok(bars) = feed.candles(&base, argos_core::Timeframe::M1, limit).await {
                let _ = proxy.send_event(UserEvent::Minutes(coin, bars));
            }
        });
    }

    /// 📈 The 1-minute candles arrived: each past release's reaction.
    pub(crate) fn cal_minutes_lues(&mut self, coin: String, bars: Vec<argos_core::Bar>) {
        let now = now_s();
        let e = &mut self.watch.eco;
        e.reactions.clear();
        for ts in e.cal_reactions(now) {
            if let Some(r) = argos_data::reel::reaction(&bars, ts) {
                e.reactions.insert(ts, r);
            }
        }
        e.reaction_coin = coin.clone();
        self.feeds.minutes = Some((coin, bars));
        self.redraw();
    }
}
