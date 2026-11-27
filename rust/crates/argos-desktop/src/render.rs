//! 🖼️ Dessin de la fenêtre : chart, panneaux, split, dock, copie dans le tampon d'écran.

use super::*;

impl App {
    /// 🪟 Image de toute la zone : la chart principale (déjà dessinée) dans la 1re case, puis chaque autre case.
    pub(crate) fn compose_split(&self, main: &tiny_skia::Pixmap, cells: &[argos_render::splits::Cell]) -> Option<tiny_skia::Pixmap> {
        let (aw, ah) = (self.area.0.max(1), self.area.1.max(1));
        let mut out = argos_render::px::new(aw, ah)?;
        let th = Theme::current();
        out.fill(tiny_skia::Color::from_rgba8(th.background.0, th.background.1, th.background.2, 255));
        argos_render::px::pose(&mut out, 0, 0, main.as_ref(), 1.0);
        let (range, cross) = self.split_follow();
        // 🖥 the legend offset (under the folded top band, or under the rows opened on hover) belongs to the main chart:
        // a pane's title stays on the row of its click zones (symbol, unit, ⇄, 🔗, ⏱ favourites: pane_title_zones). It
        // was drawn 16 points lower, so a click on « · 1h » missed the unit menu.
        let haut_legende = argos_render::haut_legende();
        argos_render::set_haut_legende(0.0);
        for (k, (cell, p)) in cells.iter().skip(1).zip(&self.panes).enumerate() {
            let (x, y) = (cell[0] * aw as f64, cell[1] * ah as f64);
            let (w, h) = ((cell[2] * aw as f64).round(), (cell[3] * ah as f64).round());
            let (name, title) = self.pane_title(k);
            // 📏 chaque case a sa propre échelle de prix, ajustée à ses bougies visibles (jamais les bornes fixées ni le
            // rapport verrouillé de la chart principale)
            let settings = self.pane_settings();
            // ⏪ during a replay: what is known at its hour (see pane_bars)
            let bars = self.pane_bars(k);
            let (viewport, _) = self.pane_view_de(k, &bars, w, h, range);
            // 📱 case du haut sur cell / tablette : le titre descend sous les pilules de la maquette (voir pane_dy)
            let dy = self.pane_dy(k);
            let mut chart = Chart {
                bars: &bars,
                tf_seconds: p.interval.secs(),
                viewport,
                settings,
                theme: th,
                style: self.chart.styles[self.chart.style].clone(),
                tz_offset_s: self.tz_offset_s(),
                title: (dy == 0.0).then(|| title.clone()),
                crosshair: None,
                banner: bars.is_empty().then(|| {
                    if self.replay_heure().is_some() && !p.charge {
                        "⏪ Pas de bougies à cette heure".to_string()
                    } else {
                        "Chargement…".to_string()
                    }
                }),
                context: ContextMenu::default(),
                // 🖍 the drawings synced in the layout (bar ⋯), on a pane of the same symbol
                drawings: self.pane_drawings(&p.coin),
                selected: None,
                now_s: 0,
                live: None,
                hors_ligne: false,
                legende: false,
                navigation: true,
                goto: None,
                indicators: None,
                mark: None,
                whales: vec![],
                events: vec![],
                alerts: vec![],
                alertes_vue: Default::default(),
                ordres: Default::default(),
                demo: None,
                bid_ask: None,
                book_heat: None,
                book_note: None,
                murs: None,
                liq_map: None,
                pad: None,
                trades: vec![],
                explication: &[],
                garder: vec![],
                garder_pilules: vec![],
                boutons_bas: None,
            };
            // ✚ le réticule : celui de la souris sur cette case, sinon celui de la principale (heure, et prix si même
            // symbole) quand il est synchronisé
            if let Some((cx, cy)) = p.croix.filter(|(cx, cy)| *cx <= chart.viewport.plot_w() && *cy <= chart.viewport.plot_h()) {
                chart.crosshair = Some((cx, cy));
            } else if let (Some((pt, rel)), Some(f)) = (cross, argos_render::frame(&chart)) {
                let g = f.geo();
                let (cx, cy) = g.xy(&pt);
                let same = p.coin == self.main_coin();
                let cy = if same { cy } else { rel * chart.viewport.plot_h() };
                if cx >= 0.0 && cx <= chart.viewport.plot_w() && cy >= 0.0 && cy <= chart.viewport.plot_h() {
                    chart.crosshair = Some((cx, cy));
                }
            }
            let mut pm = render(&chart);
            // titre : le symbole et l'unité se choisissent d'un clic ; ⇄ échange avec la principale ; 🔗 Liée / Libre
            let liee = self.split.sync_symbol || self.split.liees.get(k).copied().unwrap_or(false);
            let (_, _, swap, lien) = Self::pane_title_zones(&title, &name, liee, dy);
            if dy > 0.0 {
                let t = argos_render::text::Text::default();
                let room = (chart.viewport.plot_w() - 20.0).max(40.0);
                let s = t.fit(&title, 16.0, room);
                t.draw(&mut pm, &s, (12.0, (24.0 + dy) as f32), 16.0, th.fg, argos_render::text::Align::Left);
            }
            argos_render::shapes::lines(
                &mut pm,
                swap.cx(),
                swap.cy(),
                &[
                    &[(-5.0, -3.0), (5.0, -3.0)],
                    &[(2.0, -6.0), (5.0, -3.0), (2.0, 0.0)],
                    &[(5.0, 3.0), (-5.0, 3.0)],
                    &[(-2.0, 0.0), (-5.0, 3.0), (-2.0, 6.0)],
                ],
                argos_render::text::Rgb(0x8a, 0x8e, 0x9a),
                1.3,
            );
            let text = argos_render::text::Text::default();
            let accent = argos_render::palette::accent();
            argos_render::shapes::round_fill(
                &mut pm,
                lien,
                6.0,
                if liee { accent } else { argos_render::palette::field() },
                if liee { 0.3 } else { 1.0 },
            );
            argos_render::shapes::round_stroke(&mut pm, lien, 6.0, if liee { accent } else { argos_render::palette::edge() }, 1.0);
            text.draw(
                &mut pm,
                Self::pane_link_label(liee),
                (lien.cx() as f32, (lien.cy() + 4.0) as f32),
                12.0,
                argos_render::palette::WHITE,
                argos_render::text::Align::Center,
            );
            // ⏱ ses favoris en raccourcis (l'unité de la case surlignée)
            for (r, secs) in self.pane_favs(k, w) {
                let on = secs == p.interval.secs();
                if on {
                    argos_render::shapes::round_fill(&mut pm, r, 5.0, accent, 0.25);
                }
                text.draw(
                    &mut pm,
                    &Interval(secs).short(),
                    (r.cx() as f32, (r.cy() + 4.0) as f32),
                    11.5,
                    if on { accent } else { argos_render::text::Rgb(0x8a, 0x8e, 0x9a) },
                    argos_render::text::Align::Center,
                );
            }
            // 🖱️ la case sous la souris (ou qu'on glisse) : bord surligné
            let active = p.croix.is_some() || self.pointer.pane_drag.is_some_and(|d| d.0 == k);
            if active {
                if let Some(r) = tiny_skia::Rect::from_xywh(1.0, 1.0, (w - 2.0).max(1.0) as f32, (h - 2.0).max(1.0) as f32) {
                    let mut paint = tiny_skia::Paint::default();
                    paint.set_color_rgba8(accent.0, accent.1, accent.2, 200);
                    let path = tiny_skia::PathBuilder::from_rect(r);
                    let stroke = tiny_skia::Stroke {
                        width: 2.0,
                        ..Default::default()
                    };
                    pm.stroke_path(&path, &paint, &stroke, argos_render::px::t(), None);
                }
            }
            argos_render::px::put(&mut out, x.round(), y.round(), &pm);
        }
        argos_render::set_haut_legende(haut_legende);
        // traits entre les cases
        for cell in cells {
            // (on whole points, like the panes themselves: a line between two pixels was drawn half-grey on two)
            let (x, y, w, h) = (
                (cell[0] * aw as f64).round(),
                (cell[1] * ah as f64).round(),
                (cell[2] * aw as f64).round(),
                (cell[3] * ah as f64).round(),
            );
            for (rx, ry, rw, rh) in [(x, y, w, 1.0), (x, y, 1.0, h)] {
                if let Some(r) = tiny_skia::Rect::from_xywh(rx as f32, ry as f32, rw as f32, rh as f32) {
                    let mut paint = tiny_skia::Paint::default();
                    paint.set_color_rgba8(0x2e, 0x32, 0x3d, 255);
                    out.fill_rect(r, &paint, argos_render::px::t(), None);
                }
            }
        }
        Some(out)
    }

    /// 🧩 Hauteur du contenu d'un module (il défile dans sa case).
    pub(crate) fn dock_content_h(&self, m: argos_render::dock::Module, w: f64) -> f64 {
        use argos_render::dock::{Module, KV_ROW};
        match m {
            Module::Watchlist => self.watch.mini_h(),
            Module::Agenda => self.watch.eco.content_h(argos_render::eco::EcoKind::Agenda, w, self.watch.now),
            Module::News => self.watch.eco.content_h(argos_render::eco::EcoKind::News, w, self.watch.now),
            Module::Whales => self.watch.whales.content_h(),
            Module::Explorer => self.watch.explorer.content_h(&argos_render::text::Text::default(), w),
            Module::Stats => self.watch.stats.content_h(w),
            Module::Market => argos_render::cmc::HEIGHT,
            Module::Positions => self.demo.positions.len().max(1) as f64 * KV_ROW + 12.0,
            Module::Account => 5.0 * KV_ROW + 12.0,
            Module::Book => 0.0,
            Module::Trades => argos_render::dock::tape_h(self.tape_vue().len(), self.tape_source().1.is_some()),
            Module::Funding => 6.0 * KV_ROW + 12.0 + 90.0,
            Module::World => argos_render::dock::world_h(self.feeds.world.len(), w),
            Module::Sentiment => argos_render::dock::sentiment_h(),
            Module::Liquidations => argos_render::dock::liqs_h(self.feeds.liqs.len()),
            Module::KeyStats => argos_render::dock::key_stats_h(),
            Module::Watch => argos_render::dock::watch_h(),
            Module::Scanner => argos_render::dock::scan_h(self.feeds.scan.len()),
            // 🦎 autant de lignes que la fiche en a (variations, valeur, âge, sécurité du jeton…) : la carte défile
            Module::Dex => match &self.chart.source {
                Source::Dex { name, .. } => {
                    argos_render::dock::wallet_h(argos_render::dock::dex_rows(name, self.feeds.dex_info.as_ref(), &self.feeds.dex_trades, 0).len())
                        .max(argos_render::dock::dex_h())
                }
                _ => argos_render::dock::dex_h(),
            },
            Module::Backtest => argos_render::dock::bt_h(),
            Module::Wallet => argos_render::dock::wallet_h(argos_render::dock::wallet_rows(&self.feeds.wallet_addr, self.feeds.wallet.as_ref(), 0).len()),
        }
    }

    /// 🖥 Colonne de modules à gauche / à droite : place de chaque module (`dock.places`), hauteur totale, défilement
    /// ramené dans ses bornes et zone de contenu de chaque module visible (`dock.bodies`).
    pub(crate) fn dock_mise_en_page(&mut self) {
        use argos_render::dock as dk;
        let Some(r) = self.dock.rect.filter(|_| self.watch.dock.pos != dk::DockPos::Bottom) else {
            return;
        };
        let (places, total) = self.watch.dock.layout_pc(r.w, r.h, &|m| self.dock_content_h(m, r.w), &|m| self.dock.hauteur(m));
        self.dock.defil = self.dock.defil.clamp(0.0, dk::defil_max(r.h, total));
        let d = self.dock.defil;
        self.dock.bodies = places
            .iter()
            .map(|(m, p)| (*m, Rect::new(p.x, p.y - d + dk::HEAD_H, p.w, p.h - dk::HEAD_H)))
            .filter(|(_, b)| b.y + b.h > 0.0 && b.y < r.h)
            .collect();
        (self.dock.places, self.dock.total) = (places, total);
    }

    /// 🖥 Largeur de la colonne de modules à gauche / à droite (réglée en glissant son bord, sinon S M L XL de
    /// dock.json) ; en bas, la hauteur de la rangée.
    pub(crate) fn dock_epaisseur(&self) -> f64 {
        use argos_render::dock::{DockPos, LARGEUR_MAX, LARGEUR_MIN};
        match (self.watch.dock.pos, self.dock.largeur) {
            (DockPos::Bottom, _) | (_, None) => self.watch.dock.thickness(),
            (_, Some(w)) => w.clamp(LARGEUR_MIN, LARGEUR_MAX).round(),
        }
    }

    /// 🧩 Dessine le contenu d'un module dans `pm` (largeur `w`) ; `mouse` : souris dans le contenu.
    pub(crate) fn draw_module(
        &self,
        m: argos_render::dock::Module,
        pm: &mut tiny_skia::Pixmap,
        text: &argos_render::text::Text,
        w: f64,
        mouse: Option<(f64, f64)>,
    ) {
        use argos_render::dock::{draw_kv, Module};
        let th = Theme::current();
        // chaque position à son propre prix (chart ou watchlist), pas seulement le symbole affiché
        let pnl = |p: &argos_core::portfolio::Position| self.demo_price(&p.base).map_or(0.0, |x| p.pnl(x));
        let money = |v: f64| format!("{v:+.2} $");
        let col = |v: f64| Some(if v >= 0.0 { th.bull } else { th.bear });
        match m {
            Module::Watchlist => self.watch.draw_mini(pm, text, w, mouse, &th),
            Module::Agenda => self
                .watch
                .eco
                .draw(argos_render::eco::EcoKind::Agenda, pm, text, w, self.watch.now, mouse, &th, self.tz_offset_s()),
            Module::News => self
                .watch
                .eco
                .draw(argos_render::eco::EcoKind::News, pm, text, w, self.watch.now, mouse, &th, self.tz_offset_s()),
            Module::Whales => self.watch.whales.draw(pm, text, w, self.watch.now * 1000, mouse, &th),
            Module::Explorer => self.watch.explorer.draw(pm, text, w, mouse, th.background),
            Module::Stats => self.watch.stats.draw(pm, text, w, mouse, th.background),
            Module::Market => argos_render::cmc::draw(&self.watch.market, pm, text, w, 0.0, &th),
            Module::Positions => {
                let rows: Vec<_> = self
                    .demo
                    .positions
                    .iter()
                    .map(|p| {
                        (
                            format!("{} {} {:.4}", if p.side == argos_core::Side::Long { "▲" } else { "▼" }, p.base, p.qty),
                            money(pnl(p)),
                            col(pnl(p)),
                        )
                    })
                    .collect();
                draw_kv(pm, text, w, &rows, "Aucune position démo ouverte.");
            }
            Module::Account => {
                let open: f64 = self.demo.positions.iter().map(pnl).sum();
                let rows = vec![
                    ("Solde démo".to_string(), format!("{:.2} $", self.demo.balance), None),
                    ("PnL latent".to_string(), money(open), col(open)),
                    ("Équité".to_string(), format!("{:.2} $", self.demo.balance.to_f64() + open), None),
                    ("Positions ouvertes".to_string(), self.demo.positions.len().to_string(), None),
                    ("Trades au journal".to_string(), self.watch.panels.journal.trades.len().to_string(), None),
                ];
                draw_kv(pm, text, w, &rows, "");
            }
            Module::World => argos_render::dock::draw_world(pm, text, w, &self.feeds.world, &self.feeds.world_note, &th),
            Module::Sentiment => argos_render::dock::draw_sentiment(pm, text, w, &self.watch.market, &th),
            Module::KeyStats => argos_render::dock::draw_kv(
                pm,
                text,
                w,
                &argos_render::dock::key_stats(&self.chart.bars, self.chart.interval.secs()),
                "Pas assez de bougies",
            ),
            Module::Wallet => {
                let rows = if self.feeds.wallet_addr.is_empty() {
                    vec![]
                } else {
                    argos_render::dock::wallet_rows(&self.feeds.wallet_addr, self.feeds.wallet.as_ref(), self.tz_offset_s())
                };
                draw_kv(
                    pm,
                    text,
                    w,
                    &rows,
                    "Menu Argos ▸ Wallet : colle ton adresse publique Hyperliquid (lecture seule)",
                );
            }
            Module::Backtest => {
                let rows = self
                    .ui
                    .bt
                    .as_ref()
                    .map(|v| argos_render::dock::bt_rows(v, self.tz_offset_s()))
                    .unwrap_or_default();
                draw_kv(pm, text, w, &rows, "Panneaux ▸ 🧪 Backtest : choisis une stratégie, puis ▶️ Lancer");
            }
            Module::Dex => {
                let rows = match &self.chart.source {
                    Source::Dex { name, .. } => argos_render::dock::dex_rows(name, self.feeds.dex_info.as_ref(), &self.feeds.dex_trades, self.tz_offset_s()),
                    _ => vec![],
                };
                draw_kv(pm, text, w, &rows, "Ouvre une paire DEX : + ▸ « dex:nom » ou une adresse de contrat");
            }
            Module::Scanner => {
                let mut rows = argos_render::dock::scan_rows(&self.feeds.scan);
                let state = if self.feeds.scan_busy {
                    "scan en cours…".to_string()
                } else {
                    let ago = self.feeds.scan_at.map_or(0, |t| t.elapsed().as_secs() / 60);
                    format!("il y a {ago} min · aux 15 min")
                };
                let state = if self.feeds.scan_note.is_empty() {
                    state
                } else {
                    format!("{state} · {}", self.feeds.scan_note)
                };
                if !rows.is_empty() || !self.feeds.scan_busy {
                    rows.push(("Daily + 4H".to_string(), state, Some(argos_render::palette::GREY)));
                }
                draw_kv(pm, text, w, &rows, "📡 Scan des cryptos en cours (Daily + 4H)…");
            }
            Module::Watch => {
                let live = match &self.chart.source {
                    // ₿ a pair against BTC: not Hyperliquid's coin in dollars
                    Source::Live { base, .. } if argos_data::paire::contre_btc(base) => None,
                    Source::Live { base, .. } => self
                        .feeds
                        .quotes
                        .get(argos_data::whales::coin_of(base).as_str())
                        .map(|q| argos_render::dock::Live {
                            mark: q.mark.parse().unwrap_or(0.0),
                            prev: q.prev,
                            volume: q.volume,
                        }),
                    _ => None,
                };
                let rows = argos_render::dock::watch_rows(
                    &self.chart.bars,
                    self.chart.interval.secs(),
                    live,
                    self.feeds.book.as_ref(),
                    data_decimals(&self.chart.bars),
                );
                draw_kv(pm, text, w, &rows, "En attente des données…");
            }
            Module::Liquidations => argos_render::dock::draw_liqs(pm, text, w, &self.feeds.liqs, now_s() * 1000, self.tz_offset_s(), &th),
            // 📖 the order-book panel (Carnet / Delta / DOM), its tabs and pills; the column's header is its title
            Module::Book => argos_render::carnet::draw(
                pm,
                text,
                (w, argos_render::px::lh(pm)),
                &self.watch.prefs.carnet,
                &self.watch.carnet,
                mouse,
                false,
                false,
            ),
            Module::Trades => {
                let (source, repli) = self.tape_source();
                argos_render::dock::draw_tape(
                    pm,
                    text,
                    w,
                    self.tape_vue(),
                    data_decimals(&self.chart.bars),
                    self.tz_offset_s(),
                    self.orderflow_enregistre(),
                    (source.name(), repli.map(|e| e.name())),
                    &th,
                )
            }
            Module::Funding => {
                let coin = match &self.chart.source {
                    Source::Live { base, .. } if argos_data::paire::contre_btc(base) => String::new(),
                    Source::Live { base, .. } => argos_data::whales::coin_of(base),
                    Source::Oanda { .. } | Source::Ratio { .. } | Source::Synth { .. } | Source::Cap { .. } | Source::Dex { .. } | Source::File(_) => {
                        String::new()
                    }
                };
                let rows = match self.feeds.quotes.get(coin.as_str()) {
                    Some(q) => {
                        let mark: f64 = q.mark.parse().unwrap_or(0.0);
                        let left = 3600 - now_s().rem_euclid(3600);
                        let avg = (!self.feeds.funding_hist.is_empty())
                            .then(|| self.feeds.funding_hist.iter().map(|f| f.rate).sum::<f64>() / self.feeds.funding_hist.len() as f64);
                        vec![
                            ("Taux (1 h)".to_string(), format!("{:.4} %", q.funding * 100.0), col(q.funding)),
                            ("Annualisé".to_string(), format!("{:.2} %", q.funding * 24.0 * 365.0 * 100.0), col(q.funding)),
                            ("Prochain paiement".to_string(), format!("{:02}:{:02}", left / 60, left % 60), None),
                            (
                                "Moyenne 24 h".to_string(),
                                avg.map_or("—".into(), |a| format!("{:.4} %", a * 100.0)),
                                avg.and_then(col),
                            ),
                            ("Intérêt ouvert".to_string(), format!("{} $", argos_render::topbar::big(q.oi * mark)), None),
                            (
                                "Oracle".to_string(),
                                format_price(q.oracle, q.mark.split_once('.').map_or(0, |(_, d)| d.len())),
                                None,
                            ),
                        ]
                    }
                    None => vec![],
                };
                draw_kv(pm, text, w, &rows, "Funding d'Hyperliquid : en attente des chiffres de cette pièce…");
                argos_render::dock::draw_funding_bars(pm, text, w, 6.0 * argos_render::dock::KV_ROW + 14.0, 86.0, &self.feeds.funding_hist, &th);
            }
        }
    }

    /// ✏️ (B) A finger on the screen drags or pinches the chart itself: the chart is drawn as a draft this image.
    pub(crate) fn brouillon_du_doigt(&self) -> bool {
        let doigt = &self.pointer.touch;
        (self.pointer.drag.is_some() || doigt.pince()) && doigt.doigt_pose()
    }

    /// 📐 Revoit la mise en page pour l'écran actuel (taille, échelle, doigt / souris, segments, posture) : rotation,
    /// redimensionnement, plein écran et pliage s'appliquent à l'image suivante, sans rien perdre.
    pub(crate) fn update_form(&mut self) {
        use argos_render::formfactor::{choose, Kind, Screen};
        let Some(window) = &self.window else { return };
        #[cfg(target_arch = "wasm32")]
        let screen = {
            use winit::platform::web::WindowExtWebSys;
            let v: serde_json::Value = platform::web::screen_info().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
            let n = |k: &str, d: f64| v[k].as_f64().unwrap_or(d);
            let (cw, ch, ox, oy) = window.canvas().map_or((n("w", 1280.0), n("h", 800.0), 0.0, 0.0), |c| {
                let r = c.get_bounding_client_rect();
                (r.width(), r.height(), r.left(), r.top())
            });
            let seg = v["segments"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|s| Some((s[0].as_f64()? - ox, s[1].as_f64()? - oy, s[2].as_f64()?, s[3].as_f64()?)))
                        .collect()
                })
                .unwrap_or_default();
            (
                Screen {
                    w: cw,
                    h: ch,
                    scale: n("dpr", 1.0),
                    coarse: v["coarse"].as_bool().unwrap_or(false),
                    segments: seg,
                    folded: v["folded"].as_bool(),
                },
                ox,
                oy,
            )
        };
        #[cfg(not(target_arch = "wasm32"))]
        let screen = {
            let (s, k) = (window.inner_size(), window.scale_factor().max(0.5));
            (
                Screen {
                    w: s.width as f64 / k,
                    h: s.height as f64 / k,
                    scale: k,
                    ..Default::default()
                },
                0.0,
                0.0,
            )
        };
        let (screen, _ox, _oy) = screen;
        #[allow(unused_mut)]
        let mut form = choose(&screen);
        // 🔍 zoom choisi dans ☰ Appli (gardé par la page), en plus de l'agrandissement au doigt
        #[cfg(target_arch = "wasm32")]
        {
            let z: f64 = platform::web::screen_info()
                .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
                .and_then(|v| v["zoom"].as_f64())
                .unwrap_or(1.0);
            form.zoom *= z.clamp(0.6, 1.8);
        }
        let before = self.ui.form;
        self.ui.form = form;
        #[cfg(target_arch = "wasm32")]
        platform::web::set_view(form.zoom, _ox, _oy);
        // 🔍 rendu net : autant de pixels que l'écran en a (DPR × agrandissement au doigt), au plus ×3 ; sur l'ordi,
        // l'app dessine déjà en pixels de la fenêtre (échelle 1)
        // (the exact density of the screen; the limit: the pixel budget alone — lot A, no speed cap any more)
        let want = screen.scale * form.zoom;
        let area = (screen.w * screen.h / (form.zoom * form.zoom)).max(1.0);
        let budget = limite_echelle(form.kind == argos_render::formfactor::Kind::Desktop, area, self.geste_en_cours());
        // 🖐 pendant un geste : ×2 au plus (2,25× moins de pixels qu'à ×3 sur iPhone) ; au lâcher, pleine densité à
        // l'image suivante (positions en points inchangées)
        // (🖥 desktop app: one window pixel per point, times the compact factor of `formfactor::compacte`: the layout
        // gets window / factor points, drawn at `factor` pixels per point)
        let scale = if cfg!(target_arch = "wasm32") {
            echelle_rendu(want, budget, self.geste_en_cours())
        } else {
            form.zoom
        };
        argos_render::px::set_scale(scale as f32);
        // 🫧 taille d'écran détectée : la qualité du verre part du bon niveau (grand écran : grille plus grossière),
        // puis s'ajuste à la vitesse de l'appareil (`frame_timing`)
        let (before_w, before_h) = self.ui.glass_screen;
        if form.kind != before.kind || (screen.w - before_w).abs() > 1.0 || (screen.h - before_h).abs() > 1.0 {
            self.ui.glass_screen = (screen.w, screen.h);
            argos_render::glass::set_level(argos_render::glass::level_for(screen.w * screen.h * scale * scale));
            self.ui.glass_since = 0;
        }
        // en passant en tablette portrait, le panneau se replie (il reste un toucher plus loin) ; en revenant en
        // paysage, il se rouvre s'il était ouvert avant
        if form.drawer() && !before.drawer() && before.kind != Kind::Phone {
            self.ui.drawer_was_open = self.watch.prefs.open;
            self.watch.prefs.open = false;
        } else if before.drawer() && !form.drawer() && self.ui.drawer_was_open {
            self.watch.prefs.open = true;
        }
    }

    /// 🗂 Un seul menu flottant à la fois (Jee, 15 nov. ; comme `cacher()` de la maquette) : à chaque image, si un menu,
    /// une feuille, une fenêtre de cases ou le volet de droite vient de s'ouvrir, ceux qui étaient déjà ouverts se
    /// ferment. Restent à part : les boîtes de confirmation (nom, clé), Argos AI, les modules ancrés et les pages pleines ;
    /// et un menu ouvert depuis l'intérieur du volet (son sous-menu) garde le volet ouvert.
    pub(crate) fn un_seul_ouvert(&mut self) {
        // 🔔 le volet des notifications aussi ; et sans eux (fermés par un de leurs choix), le volet 🔔 vide se ferme
        if self.ui.notifs.as_ref().is_some_and(|n| n.volet) && !(self.watch.prefs.open && self.watch.prefs.panel == wl::Tool::Alertes) {
            self.ui.notifs = None;
        }
        if self.watch.prefs.open && self.watch.prefs.panel == wl::Tool::Alertes && self.ui.notifs.is_none() {
            self.watch.prefs.open = false;
        }
        // ⚙ le volet Réglages suit le volet de droite (un autre volet ouvert à sa place le remplace)
        if self.ui.reglages.is_some() && !(self.watch.prefs.open && self.watch.prefs.panel == wl::Tool::Reglages) {
            self.ui.reglages = None;
        }
        // 🛟 the Support panel too
        if self.ui.support.is_some() && !(self.watch.prefs.open && self.watch.prefs.panel == wl::Tool::Support) {
            self.ui.support = None;
        }
        const VOLET: usize = crate::state::OUVERTS - 1;
        let u = &self.ui;
        let maintenant = [
            u.list.is_some(),
            u.pop.is_some(),
            u.sheet.is_some(),
            u.argos.is_some(),
            u.split_menu.is_some(),
            // (🔔 dans le volet : c'est le volet de droite, compté avec lui)
            u.notifs.as_ref().is_some_and(|n| !n.volet),
            u.plan.is_some(),
            u.palette.is_some(),
            u.context.is_open(),
            u.raccourcis.is_some(),
            // (⚙ dans le volet : c'est le volet de droite, compté avec lui)
            u.reglages.as_ref().is_some_and(|g| !g.volet),
            u.pfeuille.is_some(),
            self.watch.prefs.open,
        ];
        let avant = self.ui.ouverts;
        let nouveaux: Vec<usize> = (0..maintenant.len()).filter(|&k| maintenant[k] && !avant[k]).collect();
        if !nouveaux.is_empty() {
            // ouvert depuis l'intérieur du volet (pas sa barre d'icônes) : c'est son sous-menu, le volet reste
            let dans_volet = self.pointer.side_mouse.is_some_and(|(sx, _)| sx < self.watch.pw());
            for k in (0..maintenant.len()).filter(|&k| maintenant[k] && !nouveaux.contains(&k)) {
                match k {
                    0 => self.ui.list = None,
                    1 => self.ui.pop = None,
                    2 => self.ui.sheet = None,
                    3 => self.ui.argos = None,
                    4 => self.ui.split_menu = None,
                    5 => self.ui.notifs = None,
                    6 => self.ui.plan = None,
                    7 => self.ui.palette = None,
                    8 => self.ui.context = argos_render::context_menu::ContextMenu::default(),
                    9 => self.ui.raccourcis = None,
                    10 => self.ui.reglages = None,
                    11 => self.ui.pfeuille = None,
                    VOLET if !dans_volet => {
                        self.watch.prefs.open = false;
                        self.save_watch();
                    }
                    _ => {}
                }
            }
        }
        let u = &self.ui;
        self.ui.ouverts = [
            u.list.is_some(),
            u.pop.is_some(),
            u.sheet.is_some(),
            u.argos.is_some(),
            u.split_menu.is_some(),
            // (🔔 dans le volet : c'est le volet de droite, compté avec lui)
            u.notifs.as_ref().is_some_and(|n| !n.volet),
            u.plan.is_some(),
            u.palette.is_some(),
            u.context.is_open(),
            u.raccourcis.is_some(),
            // (⚙ dans le volet : c'est le volet de droite, compté avec lui)
            u.reglages.as_ref().is_some_and(|g| !g.volet),
            u.pfeuille.is_some(),
            self.watch.prefs.open,
        ];
    }

    /// 🛡 What covers the chart now: the full-screen pages, the welcome pages of the first launch, the floating menus,
    /// sheets and windows, the dialogs, and the finger's crosshair (laid by a long press: a finger's drag would move it,
    /// not the chart) (short names, in this order); empty: the chart alone. The page is told at each
    /// change (`argosEcran`): the quality gate measures the chart only when nothing covers it (bug B-041, 25 Nov 2026:
    /// its gestures, played on the welcome page left open, were taken for the chart's). A list kept by hand: every new
    /// window, sheet or page is added here, or the gate would not see it covering the chart.
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub(crate) fn par_dessus(&self) -> Vec<&'static str> {
        let u = &self.ui;
        let bienvenue = u.accueil.as_ref().is_some_and(|p| p.bienvenue.is_some());
        [
            (bienvenue, "bienvenue"),
            (u.accueil.is_some() && !bienvenue, "accueil"),
            (u.ac_page.is_some(), "academie"),
            (u.mk_page.is_some(), "marketplace"),
            (u.pf_page.is_some(), "propfirm"),
            (u.bt_page.is_some(), "backtest"),
            (u.fx_page.is_some(), "indicateurs"),
            (u.plan.is_some(), "plan"),
            (u.list.is_some(), "menu"),
            (u.pop.is_some(), "fenetre"),
            (u.sheet.is_some(), "feuille"),
            (u.argos.is_some(), "menu-argos"),
            (u.split_menu.is_some(), "splits"),
            (u.notifs.is_some(), "notifications"),
            (u.palette.is_some(), "recherche"),
            (u.context.is_open(), "clic-droit"),
            (u.raccourcis.is_some(), "raccourcis"),
            (u.reglages.is_some(), "reglages"),
            (u.pfeuille.is_some(), "feuille-pilule"),
            (self.watch.prefs.open, "volet"),
            (u.ai_open, "argos-ai"),
            (u.marches.is_some(), "marches"),
            (u.symboles.is_some(), "symboles"),
            (u.add.is_some(), "ajout"),
            (u.name_box.is_some(), "nom"),
            (u.confirmer.is_some(), "confirmer"),
            (
                u.obj_dlg.is_some() || u.ind_dlg.is_some() || u.colorpick.is_some() || u.draw_alert.is_some(),
                "dialogue",
            ),
            (u.j_form.is_some(), "journal"),
            (u.editor.is_some(), "editeur"),
            (u.goto.is_some(), "aller-a"),
            (u.support.is_some(), "support"),
            (u.plein_ecran.is_some(), "plein-ecran"),
            (u.croix.is_some(), "reticule"),
        ]
        .into_iter()
        .filter_map(|(ouvert, nom)| ouvert.then_some(nom))
        .collect()
    }

    pub(crate) fn draw(&mut self) {
        let t0 = Instant::now();
        self.un_seul_ouvert(); // 🗂 un seul menu flottant à la fois
                               // ⌂ open or closed, the Accueil is noted for this tab only: a reload (F5) reopens what was open here
        self.accueil_noter();
        // 📍 Argos AI dit où l'on est (chart ou Accueil)
        self.ai.lieu = self.ai_lieu();
        self.ai.iv = self.chart.interval.short();
        self.ai.unite_sup = argos_ai::higher_timeframe(self.chart.interval.secs()).map(argos_ai::timeframe_label);
        // 🔔 pastille des alertes et 🛡 pilule du Gardien (barre du haut)
        self.top.alertes = self.alerts.lines(&self.symbol_label()).len();
        self.top.gardien = self.gardien_pilule();
        // 👆 élan du défilement au doigt : un pas par image, jusqu'à l'arrêt
        if self.pointer.fling.0 != 0.0 {
            self.fling_step();
            if self.pointer.fling.0 != 0.0 {
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
        }
        self.draw_frame();
        let ms = t0.elapsed().as_secs_f64() * 1000.0;
        frame_done(ms);
        self.frame_timing(ms);
        // 🛡 what covers the chart, told to the page when it changes (the quality gate reads it: B-041)
        #[cfg(target_arch = "wasm32")]
        {
            let e = self.par_dessus().join(",");
            if self.ui.par_dessus_note.as_deref() != Some(e.as_str()) {
                platform::web::call_page("argosEcran", &[&e]);
                self.ui.par_dessus_note = Some(e);
            }
        }
    }

    /// 🖐 Un geste est-il en cours (glisser la chart, défiler au doigt, pincer) ?
    pub(crate) fn geste_en_cours(&self) -> bool {
        self.pointer.drag.is_some() || self.pointer.finger_scroll.is_some() || self.pointer.touch.pince()
    }

    /// 📏 Temps des phases de l'image (début, fin des indicateurs, fin de la chart, fin des pilules et barres, fin de
    /// la copie) ; l'envoi à l'écran va jusqu'à maintenant.
    fn phases_image(&mut self, debut: Instant, ind: Instant, chart: Instant, ui: Instant, copie: Instant) {
        let fin = Instant::now();
        let ms = |a: Instant, b: Instant| b.duration_since(a).as_secs_f64() * 1000.0;
        let p = [ms(debut, ind), ms(ind, chart), ms(chart, ui), ms(ui, copie), ms(copie, fin)];
        for (k, v) in p.into_iter().enumerate() {
            self.ui.phases[k] = if self.ui.frames <= 1 { v } else { self.ui.phases[k] * 0.9 + v * 0.1 };
        }
        #[cfg(target_arch = "wasm32")]
        if self.ui.frames.is_multiple_of(10) {
            let t: Vec<String> = self.ui.phases.iter().map(|v| format!("{v:.2}")).collect();
            let r: Vec<&str> = t.iter().map(String::as_str).collect();
            platform::web::call_page("argosPhases", &r);
        }
    }

    /// 🫧 Temps de chaque image et du verre seul (moyennes glissantes) : si le verre coûte plus de
    /// [`GLASS_MAX_MS`] par image (téléphone lent…), il se coupe tout seul pour cette session. Dans le navigateur, la
    /// page garde les mesures (`argosFrameMs`) pour comparer avec et sans verre.
    pub(crate) fn frame_timing(&mut self, ms: f64) {
        let ui = &mut self.ui;
        ui.frames = ui.frames.saturating_add(1);
        let ema = |old: f64, new: f64| if ui.frames == 1 { new } else { old * 0.9 + new * 0.1 };
        ui.frame_ms = ema(ui.frame_ms, ms);
        ui.glass_ms = ema(ui.glass_ms, ui.glass_now);
        ui.glass_now = 0.0;
        let glass_wanted = self.top.prefs.glass || argos_render::Theme::is_glass(self.top.prefs.theme);
        ui.glass_since = ui.glass_since.saturating_add(1);
        // 🫧 trop lent : d'abord un verre plus grossier (deux crans), et seulement ensuite coupé
        let lvl = argos_render::glass::level();
        if glass_wanted && !ui.glass_off && ui.glass_since > LENT_IMAGES && ui.glass_ms > GLASS_MAX_MS && lvl < argos_render::glass::MAX_LEVEL {
            argos_render::glass::set_level(lvl + 1);
            (ui.glass_since, ui.glass_ms) = (0, 0.0);
        } else if glass_wanted && !ui.glass_off && ui.glass_since > 300 && ui.glass_ms < GLASS_MAX_MS / 3.0 && lvl > 0 {
            // bien en dessous (fenêtre plus petite, appareil reposé) : un cran plus fin
            argos_render::glass::set_level(lvl - 1);
            (ui.glass_since, ui.glass_ms) = (0, 0.0);
        } else if glass_wanted && !ui.glass_off && ui.glass_since > LENT_IMAGES && ui.glass_ms > GLASS_MAX_MS {
            ui.glass_off = true;
            self.banner = Some(format!(
                "🫧 Verre allégé pour garder l'app fluide ({:.0} ms par image) · Thème ▸ Style verre pour le rallumer",
                ui.glass_ms
            ));
        }
        #[cfg(target_arch = "wasm32")]
        if ui.frames.is_multiple_of(10) {
            platform::web::call_page(
                "argosFrameMs",
                &[
                    &format!("{:.2}", ui.frame_ms),
                    &format!("{:.2}", ui.glass_ms),
                    if ui.glass_off { "coupé" } else { "" },
                ],
            );
        }
    }

    pub(crate) fn draw_frame(&mut self) {
        let t_debut = Instant::now();
        // 🖐 geste en cours (glisser la chart, défiler au doigt, pincer) : le verre garde le flou de la dernière image au
        // repos ; au lâcher, l'image suivante le recalcule (au repos, identique au pixel près)
        argos_render::glass::set_fige(self.geste_en_cours());
        // ✏️ (B) a finger drags or pinches the chart itself: the chart's shapes drawn as a draft, without smooth edges
        // (×2 faster); the image at rest, drawn again as soon as the finger lifts, is sharp. The bars and panels around
        // the chart, the mouse, a finger scrolling a list or a sheet: unchanged.
        let brouillon = self.brouillon_du_doigt();
        // 🛟 red dot on the Support button: Argos replies not read yet
        self.top.support = self.ui.support_non_lues;
        // 🛟 the Support button is placed after the DÉMO pill (without moving it)
        self.top.support_gauche = {
            let (d, _) = self.bandeau_place();
            if d.w > 0.0 && d.y < crate::render::VOLET_HAUT {
                d.x + d.w
            } else {
                0.0
            }
        };
        // bandeau d'information : il part tout seul après 8 s
        match (&self.banner, &self.ui.banner_since) {
            (Some(b), Some((seen, t))) if b == seen => {
                if t.elapsed().as_secs() >= 8 {
                    self.banner = None;
                    self.ui.banner_since = None;
                }
            }
            (Some(b), _) => self.ui.banner_since = Some((b.clone(), web_time::Instant::now())),
            (None, _) => self.ui.banner_since = None,
        }
        self.update_form();
        // 🎫 📏 zoom auto : l'échelle d'avant le ticket revient à sa fermeture
        self.ticket_echelle();
        self.maybe_load_older();
        // (🧪 tests : sans fenêtre, à la taille `taille_test` ; l'image va dans `image_test`)
        let size = match (&self.window, self.ui.taille_test) {
            (Some(window), _) => ui_size(window),
            (None, Some(s)) => s,
            (None, None) => return,
        };
        // 🔍 l'image fait `scale` fois la taille en points (pixels de l'écran : net sur iPhone)
        let sc = argos_render::px::scale() as f64;
        let dev = |v: f64| (v * sc).round() as usize;
        let (Some(w), Some(h)) = (
            NonZeroU32::new((size.width as f64 * sc).round() as u32),
            NonZeroU32::new((size.height as f64 * sc).round() as u32),
        ) else {
            return;
        };
        // la chart à gauche, la watchlist (et sa barre d'icônes) collée à droite, le menu ⠿ par-dessus
        // barre de gauche | chart | watchlist et sa barre d'icônes
        // 📱 cell : barre du haut d'une rangée, pas de barre de gauche, la watchlist par-dessus la chart (toute la
        // largeur), et les deux rangées du bas (symbole, Panneaux / Market / CMC / Argos AI)
        // 🎯 a strategy just added: its volet opens (before the layout)
        self.strategie_ouvrir();
        let Placement {
            form,
            phone,
            side_w,
            cells,
            rw,
            top_h,
        } = self.placer(size);
        self.refresh_indicators();
        let t_ind = Instant::now();
        // 🫧 glass on and a single chart: the right-click menu and « Aller à » on their own layer, laid in glass below
        let mut fond = self.ui.fond.take();
        let ind_sig = self.ind_sig();
        let chart = self.to_chart();
        argos_render::set_menus_a_part(argos_render::glass::on() && cells.len() <= 1);
        argos_render::px::set_brouillon(brouillon);
        let mut pm = crate::fond::image(&mut fond, &chart, &ind_sig);
        argos_render::px::set_brouillon(false);
        self.bulle_plein_ecran(&mut pm);
        self.dessiner_etiquette_alerte(&mut pm);
        let menus_calque = argos_render::menus_calque(&chart).filter(|_| argos_render::glass::on() && cells.len() <= 1);
        argos_render::set_menus_a_part(false);
        drop(chart);
        self.ui.fond = fond;
        // 🖍 traits du curseur Démonstration, par-dessus la chart
        if !self.ui.demo_traits.is_empty() {
            self.dessiner_demo(&mut pm);
        }
        let t_chart = Instant::now();
        argos_render::indleg::draw_de(
            &mut pm,
            &argos_render::text::Text::default(),
            &self.ind.on,
            &self.ind.out,
            self.ind.leg.0,
            self.chart.viewport.plot_h(),
            self.pointer.mouse,
            argos_render::indleg::Cell {
                actif: phone,
                sel: self.ind.sel,
                pw: self.chart.viewport.plot_w(),
            },
        );
        // 🪟 split : la chart principale dans sa case, les autres cases autour (bougies de leur symbole)
        self.sync_panes();
        let mut area_pm = (cells.len() > 1).then(|| self.compose_split(&pm, &cells)).flatten();
        let text = argos_render::text::Text::default();
        // ⏪ the replay bar: over the main chart, or over the panes when it is too narrow for it (Suivi 7)
        if let Some(rp) = &self.replay {
            let date = argos_render::crosshair_label(rp.t + self.tz_offset_s(), self.chart.interval.secs());
            let (zone_w, ph) = self.replay_zone();
            argos_render::replay::draw(area_pm.as_mut().unwrap_or(&mut pm), &text, rp, zone_w, ph, &date, self.pointer.mouse);
        }
        // 📐 deux segments l'un au-dessus de l'autre : le panneau n'a que celui du bas
        self.ui.feuille_verre = phone
            && self.watch.prefs.open
            && self.watch.prefs.panel == wl::Tool::Trading
            && argos_render::glass::on()
            && !matches!(form.split, Some(argos_render::formfactor::Split::Rows { .. }));
        let ch = self.ui.rows.map_or(self.area.1, |(_, bot)| self.area.1.saturating_sub(bot as u32));
        self.refresh_top(); // chaque image : le compte à rebours du funding avance à la seconde
        self.watch.now = now_s();
        self.refresh_eco();
        self.charger_emojis();
        {
            // P&L de chaque position à son propre prix (chart ou watchlist)
            let open = self
                .demo
                .positions
                .iter()
                .map(|p| {
                    (
                        format!("{} {} {:.4}", if p.side == argos_core::Side::Long { "▲" } else { "▼" }, p.base, p.qty),
                        self.demo_price(&p.base).map_or(0.0, |x| p.pnl(x)),
                    )
                })
                .collect();
            let sym = self.symbol_label();
            let pending = self
                .demo
                .pending
                .iter()
                .map(|p| {
                    let kind = if p.kind == argos_core::portfolio::OrderKind::Limit { "Limit" } else { "Stop" };
                    let side = if p.order.side == argos_core::Side::Long { "Buy" } else { "Sell" };
                    (
                        p.id,
                        format!("{side} {kind} {} @ {}", p.order.base, argos_render::viewport::format_price(p.order.price, 2)),
                    )
                })
                .collect();
            let last = self.chart.bars.last().map(|b| b.close);
            let account = self.demo.name.clone();
            let t = &mut self.watch.panels.trading;
            (t.balance, t.sym, t.open, t.pending, t.account) = (self.demo.balance.to_f64(), sym, open, pending, account);
            // 📱 la feuille Trading de la maquette sur cell et tablette ; « ⋯ Toutes les options » se replie avec la feuille
            // (🖥 PC : la même feuille de la maquette dans le volet flottant ; ⋯ Toutes les options = l'ancien ticket complet)
            (t.prix, t.cell) = (last.unwrap_or(0.0), phone || self.watch.volet_flottant);
            if !self.watch.prefs.open {
                t.options = false;
            }
            // Limit / Stop : le prix de l'ordre part du dernier prix (ou de l'entrée de la position dessinée)
            if t.tab != 0 && t.px <= 0.0 {
                t.px = t.plan.map(|p| p.0).or(last).unwrap_or(0.0);
            }
        }
        // 🎫 module 31: the ticket of the open Trading sheet, from the demo account and the market
        if self.watch.prefs.open && self.watch.prefs.panel == wl::Tool::Trading {
            self.ticket_sync();
        }
        if phone {
            self.watch.fiche = self.top.stats.clone(); // 📱 fiche du symbole en bas de la watchlist
        }
        self.top.layout = self.top.prefs.layout_name.clone();
        // (🧪 images de référence : une sauvegarde d'un autre test ne change pas « ✓ Sauvegardé »)
        self.top.saving = platform::sauvegarde_en_cours() && self.ui.taille_test.is_none();
        self.top.iv = (self.chart.interval.secs(), self.chart.interval.short());
        self.top.iv_favs = self.tools.favorites.iter().map(|&s| (s, Interval(s).short())).collect();
        self.top.split = self.split.layout.clone();
        argos_render::viewport::set_number_format(self.top.prefs.number_format);
        argos_render::i18n::set_lang(self.top.prefs.language);
        argos_render::set_theme(self.top.prefs.theme);
        // 🫧 la case « Style verre », ou un thème verre (Verre sombre / Verre clair) : coupé tout seul si trop lent
        let glass_wanted = self.top.prefs.glass || argos_render::Theme::is_glass(self.top.prefs.theme);
        argos_render::glass::set(glass_wanted && !self.ui.glass_off);
        // Mises en page ▸ Sauvegarde automatique : au plus toutes les 10 s
        if self.top.prefs.auto_save && self.auto_saved.elapsed().as_secs() >= 10 {
            self.auto_saved = Instant::now();
            self.save();
        }
        // panneau API : ses lignes à jour avant de le dessiner
        if self.watch.prefs.open && self.watch.prefs.panel == wl::Tool::Exchanges {
            self.watch.exchanges = self.exchange_rows();
        }
        self.watch.panels.ind_on = self.ind.on.clone();
        self.watch.panels.counts = [self.ind.favs.len(), self.scripts.len()];
        // molette : le défilement glisse vers sa cible, une image à la fois
        if self.watch.glide() {
            if let Some(w) = &self.window {
                w.request_redraw();
            }
        }
        // 🖥 colonne d'icônes PC (maquette) : du haut au bas de la fenêtre, dessinée à part
        (self.watch.colonne, self.watch.col_h) = (self.colonne_pc(), ch as f64);
        let mut side = self.watch.draw(&text, ch, self.pointer.side_mouse, &Theme::current());
        // 🖥 volet flottant : dessiné à part, posé sous la 1re rangée (la souris du volet est dans ses coordonnées)
        let volet = self.volet_rect().and_then(|v| {
            let m = self.pointer.side_mouse.filter(|(x, _)| *x < self.watch.pw());
            Some((self.watch.draw_volet(&text, v.h as u32, m, &Theme::current())?, v))
        });
        let colonne = self.watch.draw_colonne(&text, self.pointer.side_mouse, &Theme::current());
        // 🧩 colonne de modules : cadre et en-têtes, puis le contenu de chaque module
        let dock_pm = self.dock.rect.and_then(|r| {
            // 🖥 gauche / droite : chaque module garde sa hauteur, la colonne défile (molette, barre, poignées, titres
            // glissés) ; en bas : la rangée d'avant
            let (mut pm, bodies) = if self.watch.dock.pos != argos_render::dock::DockPos::Bottom {
                use argos_render::dock as dk;
                self.dock_mise_en_page();
                let (places, total, defil) = (std::mem::take(&mut self.dock.places), self.dock.total, self.dock.defil);
                let glisse = match self.dock.geste {
                    Some(crate::state::GesteDock::Titre { m, y, bouge: true, .. }) => Some((m, dk::depot(&places, m, defil, y).1)),
                    _ => None,
                };
                let mouse = self.pointer.dock_mouse.filter(|_| self.dock.geste.is_none());
                let out = dk::frame_pc(
                    &self.watch.dock,
                    &text,
                    (r.w as u32, r.h as u32),
                    (&places, total, defil),
                    mouse,
                    glisse,
                    &Theme::current(),
                )?;
                self.dock.places = places;
                out
            } else {
                argos_render::dock::frame(&self.watch.dock, &text, r.w as u32, r.h as u32, self.pointer.dock_mouse, &Theme::current())?
            };
            for (m, b) in &bodies {
                let h = self.dock_content_h(*m, b.w).max(b.h);
                let off = self.dock.scroll.get(m).copied().unwrap_or(0.0).clamp(0.0, (h - b.h).max(0.0));
                let Some(mut sub) = argos_render::px::new(b.w.max(1.0) as u32, h as u32) else {
                    continue;
                };
                let mouse = self
                    .pointer
                    .dock_mouse
                    .filter(|&(x, y)| b.contains(x, y))
                    .map(|(x, y)| (x - b.x, y - b.y + off));
                self.draw_module(*m, &mut sub, &text, b.w, mouse);
                if let Some(mut part) = argos_render::px::int_rect(0.0, off, b.w.max(1.0), b.h.max(1.0)).and_then(|ir| sub.clone_rect(ir)) {
                    // 📰 the Actualités' bottom stays in view, the list scrolls above it
                    if *m == argos_render::dock::Module::News {
                        let k = argos_render::eco::EcoKind::News;
                        self.watch.eco.draw_pied(k, &mut part, &text, b.w, b.h, self.watch.now, &Theme::current());
                    }
                    argos_render::px::put(&mut pm, b.x, b.y, &part);
                }
            }
            self.dock.bodies = bodies;
            Some((pm, r))
        });
        let flagged = |c: &str| self.watch.flag(c).is_some();
        // 📡 the markets on screen (ticker band, list rows in view): the live prices are asked for those only
        let mut vis = self.top.ticks_visibles(self.top_w(), &text, &flagged);
        if self.watch.prefs.open && self.watch.prefs.panel == wl::Tool::Watchlist {
            let h = self.volet_rect().map_or(ch as f64, |v| v.h);
            vis.extend(self.watch.coins_visibles(h));
        }
        if vis != self.feeds.hl_visibles {
            self.feeds.hl_visibles = vis;
            self.want_hl_live();
        }
        // 📐 cell : le haut est le fond de la chart ; la pilule de la maquette se pose dessus (avec les autres pilules)
        // 🖥 rangées 2 et 3 ouvertes au survol : entre le rail de gauche et le volet ouvert (ou la colonne d'icônes)
        let rangs = self.rangs_haut();
        self.top.rangs = rangs;
        // 🧘 Zen: the top bar brought back lies over the chart — its pills alone
        self.top.flottante = !phone && self.ui.zen;
        let mut top = if phone {
            let mut p = argos_render::px::new(self.top_w().max(1.0) as u32, top_h.max(1.0) as u32).expect("haut du cell");
            p.fill(tiny_skia::Color::from_rgba8(
                Theme::current().background.0,
                Theme::current().background.1,
                Theme::current().background.2,
                255,
            ));
            p
        } else {
            self.top.draw(
                &text,
                self.top_w() as u32,
                self.pointer.top_mouse,
                &self.watch.logos,
                &flagged,
                &Theme::current(),
            )
        };
        // barre flottante (dans la chart), sa bulle, puis le menu en liste ouvert
        self.sync_toolbar();
        let tb = self.tools.clone();
        // barre du bas, sa bulle et sa fenêtre
        // ✓ dans les menus : le panneau ouvert (et Trade quand aucun panneau ne cache la chart)
        let open_tool = self.watch.prefs.open.then_some(self.watch.prefs.panel);
        let sheet_open = |a: Act| {
            open_tool.is_some_and(|t| match a {
                Act::Indicators => t == wl::Tool::Indicators,
                Act::Demo => t == wl::Tool::Trading,
                Act::Journal => t == wl::Tool::Journal,
                Act::Exchanges => t == wl::Tool::Exchanges,
                Act::Agenda => t == wl::Tool::Agenda,
                Act::Eco => t == wl::Tool::News,
                Act::Whales => t == wl::Tool::Whales,
                Act::Modules => t == wl::Tool::Modules,
                _ => false,
            })
        };
        let argos_open = |a: TopAct| match a {
            TopAct::Trade => open_tool.is_none(),
            TopAct::Journal => sheet_open(Act::Journal),
            TopAct::DemoAccount => sheet_open(Act::Demo),
            TopAct::ApiKeys => sheet_open(Act::Exchanges),
            _ => false,
        };
        let mut sheet = self
            .ui
            .sheet
            .as_ref()
            .and_then(|s| s.draw(&text, &self.top.account, self.pointer.win_mouse, &sheet_open));
        let mut argos = self
            .ui
            .argos
            .as_ref()
            .and_then(|m| m.draw(&text, &self.compte_carte(), self.pointer.win_mouse, &argos_open));
        let mut split_menu = self.ui.split_menu.as_ref().and_then(|m| m.draw(&text, &self.split, self.pointer.win_mouse));
        let mut tree = self
            .ui
            .obj_tree
            .as_ref()
            .and_then(|t| t.draw(&text, &self.tree_rows(), &self.symbol_label(), self.pointer.win_mouse));
        let lifted =
            self.pointer.tool_drag.is_some() && self.pointer.tool_moved || self.pointer.bar_press.is_some_and(|p| matches!(p.mode, PressMode::Lift(..)));
        let opened = self.ui.pop.as_ref().map(|p| match p.kind {
            toolbar::PopKind::Tf => Btn::Intervals,
            toolbar::PopKind::Group(g) => Btn::Group(g),
            toolbar::PopKind::Emoji => Btn::Group("annot"),
            toolbar::PopKind::Style => Btn::Style,
            toolbar::PopKind::Ind => Btn::Indicators,
        });
        // 📱 la watchlist ouverte cache aussi la barre flottante
        // 📋 menu du clic droit / appui long ouvert : la barre se cache (elle ne passe plus par-dessus ses lignes)
        // 🫥 PC : la barre flottante disparaît quand la souris n'est pas près d'elle (maquette : elle ne cache plus les
        // bougies) ; elle revient dès qu'on s'en approche, qu'on la déplace ou qu'une de ses fenêtres est ouverte
        let proche = self.barre_outils_proche(lifted || opened.is_some());
        let mut bar = toolbar::draw(&tb, &text, self.pointer.mouse, lifted, opened)
            .filter(|_| !(phone && side_w > 0) && !self.ui.context.is_open() && proche && !self.pilule());
        let mut pop = self.ui.pop.as_ref().and_then(|p| p.draw(&text, &tb, self.pointer.mouse));
        // 🔔 the alert window of a drawing
        let draw_alert_pm = self
            .ui
            .draw_alert
            .as_ref()
            .and_then(|(_, w)| Some((w.draw(&text, self.pointer.win_mouse)?, w.rect)));
        // 🎨 its palette
        let colorpick_pm = self
            .ui
            .colorpick
            .as_ref()
            .and_then(|(_, _, cp)| Some((cp.draw(&text, Some(self.pointer.win_mouse))?, cp.rect)));
        // 🖍 the bar of the selected drawing
        let drawbar_pm = self.drawbar().and_then(|(i, bar)| {
            let open = match self.ui.list.as_ref().map(|(k, _)| *k) {
                Some(ListKind::DrawBar(_, b)) => Some(b),
                _ => None,
            };
            let d = self.chart.drawings.get(i)?;
            Some((argos_render::drawbar::draw(&bar, d, &text, self.pointer.mouse, open)?, bar.rect))
        });
        let mut tip = toolbar::tooltip(
            &tb,
            &text,
            self.pointer
                .mouse
                .filter(|_| self.ui.list.is_none() && self.ui.pop.is_none() && self.pointer.tool_drag.is_none() && self.pointer.bar_press.is_none()),
        )
        .and_then(|(t, r)| Some((toolbar::tip_pixmap(&text, &t)?, r)));
        let mut list = self
            .ui
            .list
            .as_ref()
            .and_then(|(_, m)| Some((m.draw(&text, Some(self.pointer.win_mouse))?, m.rect)));
        let mut add = self.ui.add.as_ref().and_then(|p| p.draw(&text, self.pointer.win_mouse, &self.watch.logos));
        let mut marches = self
            .ui
            .marches
            .as_ref()
            .and_then(|p| p.draw(&text, Some(self.pointer.win_mouse), &|b: &str| self.watch.logos.get(b).cloned()));
        let mut name_box = self.ui.name_box.as_ref().and_then(|b| b.draw(&text, self.pointer.win_mouse));
        // ⚡ the card (PC) or sheet (phone) of the chart's news; 📅 the card of a release
        let mut actu_cartes: Vec<(tiny_skia::Pixmap, Rect)> = self.eclair_draw(&text).into_iter().chain(self.cal_carte_draw(&text)).collect();
        let mut key_box = self.session.key_box.as_ref().and_then(|b| b.draw(&text, self.pointer.win_mouse));
        let confirmer_pm = self.ui.confirmer.as_ref().and_then(|(c, _)| {
            let (w, h) = self.taille_points();
            let mut p = argos_render::px::new(w.max(1.0) as u32, h.max(1.0) as u32)?;
            c.draw(&mut p, &text, w, h, Some(self.pointer.win_mouse));
            Some(p)
        });
        let words_box = self.session.words_box.as_ref().and_then(|b| b.draw(&text, self.pointer.win_mouse));
        let mut obj_dlg = self
            .ui
            .obj_dlg
            .as_ref()
            .and_then(|dl| Some(dl.draw(&text, self.chart.drawings.get(dl.index)?, self.pointer.win_mouse)))
            .unwrap_or_default();
        let mut ind_dlg = self.ui.ind_dlg.as_ref().and_then(|dl| dl.draw(&text, self.pointer.win_mouse));
        let mut j_form = self.ui.j_form.as_ref().and_then(|f| f.draw(&text, self.pointer.win_mouse));
        let er = self.ed_rect();
        let mut editor = self.ui.editor.as_ref().and_then(|e| {
            Some((
                e.draw(&text, er.w, er.h, Some((self.pointer.win_mouse.0 - er.x, self.pointer.win_mouse.1 - er.y)))?,
                er,
            ))
        });
        let locked = !self.chart.drawings.is_empty() && self.chart.drawings.iter().all(|d| d.locked());
        // ‹ barre de gauche repliée : pas d'image
        let mut left = rail::draw(&self.rail, ch, self.tool, locked, self.pointer.rail_mouse).filter(|_| !self.rail.folded);
        let mut rail_tip = self
            .pointer
            .rail_mouse
            .filter(|_| self.ui.list.is_none() && !phone) // (📱 pas de barre de gauche sur cell : pas de bulle)
            .and_then(|(x, y)| rail::hit(x, y, ch as f64))
            .and_then(|(b, r)| Some((toolbar::tip_pixmap(&text, &rail::tip(&self.rail, b))?, r)));
        // 🖥 the right icon bar's help bubble (PC): the tool's name, beside its button on the chart's side
        let mut col_tip = self
            .pointer
            .side_mouse
            .filter(|_| self.ui.list.is_none() && !phone && self.pointer.icon_drag.is_none() && self.souris_seule())
            .and_then(|(x, y)| self.watch.bulle_outil(x, y))
            .and_then(|(t, r)| Some((toolbar::tip_pixmap(&text, t)?, r)));
        let list_tip = self
            .ui
            .list
            .as_ref()
            .and_then(|(_, m)| m.tip_at(self.pointer.win_mouse.0, self.pointer.win_mouse.1).map(|t| (t, m.rect)));
        let mut list_tip = list_tip.and_then(|((t, top), r)| Some((toolbar::tip_pixmap(&text, &t)?, r.x + r.w + 6.0, top + 8.0)));
        // 💼 Portfolio : page plein écran par-dessus tout
        let price = |base: &str| self.demo_price(base);
        let mut pf_pm = self
            .ui
            .pf_page
            .as_ref()
            .and_then(|p| p.draw(&text, size.width, size.height, &self.demo, now_s(), &price, self.pointer.win_mouse));
        let spark = |i: argos_render::indicators::Ind| {
            self.ui
                .mk_spark
                .iter()
                .find(|(k, _)| *k == i)
                .map(|(_, v)| v.clone())
                .unwrap_or((vec![], argos_render::palette::GREY))
        };
        // 📊 the backtest report: a full-screen page too
        let mut bt_pm = self
            .ui
            .bt_page
            .as_ref()
            .zip(self.ui.bt.as_ref())
            .and_then(|(p, v)| p.draw(&text, size.width, size.height, v, self.tz_offset_s(), self.pointer.win_mouse));
        let mut mk_pm = self
            .ui
            .mk_page
            .as_ref()
            .and_then(|m| m.draw(&text, size.width, size.height, &self.ind.on, &spark, self.pointer.win_mouse));
        let mut ac_pm = self
            .ui
            .ac_page
            .as_ref()
            .and_then(|p| p.draw(&text, size.width, size.height, self.pointer.win_mouse));
        // ⌂ Accueil (plein écran, en verre), 🔎 recherche Ctrl + K, ⌨️ volet des raccourcis, ‹ › languettes
        let acc_pm = self.ui.accueil.as_ref().and_then(|p| {
            let cartes = argos_render::accueil::cartes(&self.accueil_donnees());
            p.draw(&text, size.width, size.height, self.pointer.win_mouse, &cartes)
        });
        // 🫧 the information banner of the Accueil's actions, over it (a glass pill at the bottom)
        let acc_band = self.ui.accueil.as_ref().and(self.banner.as_ref()).and_then(|b| {
            let bas = size.height as f64 - if phone { argos_render::ecrans::dock_h() } else { 0.0 };
            argos_render::bandeau_verre(&text, b, size.width as f64, bas)
        });
        let pal_pm = self
            .ui
            .palette
            .as_ref()
            .and_then(|p| p.draw(&text, size.width as f64, size.height as f64, self.pointer.win_mouse));
        // (🖥 dans le volet flottant : à sa place et à sa taille ; sinon toute la fenêtre)
        let fx_r = self.fx_rect().unwrap_or(Rect::new(0.0, 0.0, size.width as f64, size.height as f64));
        let fx_pm = self.ui.fx_page.as_ref().and_then(|p| {
            let m = (!self.pointer.doigt_leve).then_some((self.pointer.win_mouse.0 - fx_r.x, self.pointer.win_mouse.1 - fx_r.y));
            p.draw(&text, fx_r.w, fx_r.h, &self.fx_donnees(), m)
        });
        // 🔎 volet Symboles (🖥) : à la place et à la taille du volet flottant
        let sym_pm = self.sym_rect().and_then(|r| {
            let m = (!self.pointer.doigt_leve).then_some((self.pointer.win_mouse.0 - r.x, self.pointer.win_mouse.1 - r.y));
            Some((self.ui.symboles.as_ref()?.draw(&text, r.w, r.h, &self.sym_donnees(), m)?, r))
        });
        // ⚙ fenêtre Réglages (🖥)
        let reg_pm = self
            .ui
            .reglages
            .as_ref()
            .and_then(|g| g.draw(&text, (!self.pointer.doigt_leve).then_some(self.pointer.win_mouse)));
        // 🛟 Support panel (🖥)
        let sup_pm = self
            .ui
            .support
            .as_ref()
            .and_then(|g| g.draw(&text, (!self.pointer.doigt_leve).then_some(self.pointer.win_mouse)));
        let plan_pm = self.ui.plan.as_ref().and_then(|p| {
            let m = (!self.pointer.doigt_leve).then_some(self.pointer.win_mouse);
            p.draw(&text, size.width as f64, size.height as f64, m)
        });
        // 📱 the open sheet of the single pill (« Argos », « Dessins »…), stuck to the bottom, in glass
        let pfeuille_pm = self.pfeuille_vue().and_then(|f| {
            let (w, h) = self.pfeuille_fenetre();
            let survol = (!self.pointer.doigt_leve).then_some(self.pointer.win_mouse);
            Some((f.draw(&text, w, h, survol)?, f.rect(&text, w, h)))
        });
        let notif_pm = self.ui.notifs.as_ref().and_then(|p| {
            let m = (!self.pointer.doigt_leve).then_some(self.pointer.win_mouse);
            // 🔔 the alerts panel (glass, on the PC too: like the modules of Marchés et suivi)
            if !p.fil {
                p.alertes.dessine.set(None);
                let r = p.alertes_place(size.width as f64, size.height as f64);
                p.alertes.dessine.set(Some(r));
                let logo = |b: &str| self.watch.logos.get(b).cloned();
                let pm = p
                    .alertes
                    .draw(&text, r, &self.alertes_donnees(), now_s(), m, argos_render::glass::on(), &logo)?;
                return Some((pm, r));
            }
            p.draw(
                &text,
                size.width as f64,
                size.height as f64,
                &self.notif_evenements(),
                now_s(),
                m,
                // (🖥 dans le volet : opaque comme les autres volets ; la feuille du cell reste en verre)
                argos_render::glass::on() && !p.volet,
            )
        });
        let sim_pm = self.sim_draw(&text);
        let month_pm = self.month_draw(&text);
        let avis_pm = self.avis_draw(&text);
        let fp_pm = self.fp_draw(&text);
        let band_pm = self.bandeau_draw(&text);
        let rac_pm = self.ui.raccourcis.as_ref().and_then(|p| {
            let r = self.raccourcis_rect();
            Some((p.draw(&text, r, self.pointer.win_mouse)?, r))
        });
        // 🖥 « Afficher prix et symbole » (rangées repliées) : petit rond en verre, liseré violet, plus net au survol
        // 🖥 la bande des rangées du haut repliées (verre, poignée couchée)
        let deplier = self.bande_haut().and_then(|r| {
            let mut p = argos_render::px::new(r.w as u32, r.h as u32)?;
            argos_render::watchlist::dessiner_bande(&mut p, r.w, r.h, &Theme::current());
            Some((p, r))
        });
        let haut_ouvert = self.top.sans_bandeaux && self.top.survol_bandeaux;
        let rail_y0 = self.rail_y0();
        let volet_ouvert = self.watch.prefs.open;
        // 🖥 ‹ › à côté du rail ouvert : le garder ouvert / le replier
        let rail_garde = self.rail_garde_rect().and_then(|r| {
            let mut p = argos_render::px::new(r.w as u32, r.h as u32)?;
            let c = Rect::new(0.5, 0.5, r.w - 1.0, r.h - 1.0);
            // (its own glass pill on the chart, never a dark fill: Jee's permanent rule, 20 Nov)
            argos_render::jetons::verre(&mut p, c, r.w / 2.0);
            if r.contains(self.pointer.win_mouse.0, self.pointer.win_mouse.1) {
                argos_render::shapes::round_fill(&mut p, c, r.w / 2.0, argos_render::jetons::VERRE2.0, argos_render::jetons::VERRE2.1);
            }
            let d = if self.ui.rail_garde { 2.5f32 } else { -2.5 };
            argos_render::shapes::lines(
                &mut p,
                r.w / 2.0,
                r.h / 2.0,
                &[&[(d, -5.0), (-d, 0.0), (d, 5.0)]],
                argos_render::jetons::DIM,
                1.6,
            );
            Some((p, r))
        });
        // 🖥 le pied flottant (au survol du bas de la chart)
        let pied = self.pied_visible().and_then(|r| {
            let m = self.pointer.mouse.map(|(x, y)| (x - r.x, y - r.y));
            let pm = argos_render::bottombar::draw_pied(&text, r.w, m, now_s(), self.tz_offset_s(), &Theme::current())?;
            Some((pm, Rect::new(rw + r.x, top_h + r.y, r.w, r.h)))
        });
        // 🖥 la bande du rail replié (verre, coins ronds) ; plus claire au survol
        // (au survol : le rail entier par-dessus la chart, à la place de la bande)
        let bande = self.bande_rail().filter(|_| !self.ui.rail_survol).and_then(|r| {
            let mut p = argos_render::px::new(r.w as u32, r.h as u32)?;
            argos_render::watchlist::dessiner_bande(&mut p, r.w, r.h, &Theme::current());
            Some((p, r))
        });
        let rail_ouvert = (self.ui.rail_survol && self.bande_rail().is_some())
            .then(|| {
                rail::draw(
                    &self.rail,
                    ch.saturating_sub((self.rail_y0() - top_h) as u32),
                    self.tool,
                    locked,
                    self.pointer.rail_mouse,
                )
            })
            .flatten();
        let tabs: Vec<(tiny_skia::Pixmap, Rect)> = self
            .fold_tabs()
            .into_iter()
            .filter_map(|(r, left)| {
                let folded = if left {
                    self.rail.folded
                } else {
                    self.watch.prefs.folded && !self.watch.prefs.open
                };
                let hover = r.contains(self.pointer.win_mouse.0, self.pointer.win_mouse.1);
                Some((Self::fold_tab_pm(left, folded, hover)?, r))
            })
            .collect();
        let ai_r = self.ai_rect();
        // 📱 zones où on tape : y toucher ouvre le clavier du cell
        #[cfg(target_arch = "wasm32")]
        {
            let mut z: Vec<(f64, f64, f64, f64)> = vec![];
            if let Some(r) = self.marches_zone_saisie() {
                // 🔎 the market panel (PC, tablet) or sheet (phone) lies over the rest: its field « Chercher un marché »
                z.push((r.x, r.y, r.w, r.h));
            } else if let Some((r, _)) = self.pfeuille_vue().filter(|f| f.saisie).and_then(|f| {
                let (w, h) = self.pfeuille_fenetre();
                f.zones(&argos_render::text::Text::default(), w, h)
                    .into_iter()
                    .find(|(_, t)| *t == argos_render::ecrans::feuille::Toucher::Champ)
            }) {
                // ✎ the drawings search of the pill's sheet
                z.push((r.x, r.y, r.w, r.h));
            } else if self.ui.fx_page.as_ref().is_some_and(|p| p.recherche.is_some()) {
                // 🅘 recherche de la page Indicateurs
                let r = argos_render::fxpage::FxPage::champ(size.width as f64);
                z.push((r.x, r.y, r.w, r.h));
            } else if let Some(p) = self.emoji_pick().filter(|p| p.recherche.is_some()) {
                // 😀 recherche du sélecteur d'émojis (coordonnées de la chart → fenêtre)
                let r = p.champ();
                z.push((r.x + self.rw(), r.y + self.top_h(), r.w, r.h));
            } else if let Some(b) = &self.session.key_box {
                z.push((b.rect.x, b.rect.y, b.rect.w, b.rect.h));
            } else if self.ui.palette.is_some() {
                let r = argos_render::recherche::Palette::champ(size.width as f64, size.height as f64);
                z.push((r.x, r.y, r.w, r.h));
            } else if self.ui.ai_open {
                let f = argos_render::assistant::Assistant::champ(ai_r.w, ai_r.h);
                z.push((ai_r.x + f.x, ai_r.y + f.y, f.w, f.h));
            } else if let Some(g) = &self.ui.support {
                // 🛟 the Support fields (subject, description, reply)
                z.extend(g.champs().into_iter().map(|r| (r.x, r.y, r.w, r.h)));
            } else if let Some(b) = &self.ui.name_box {
                z.push((b.rect.x, b.rect.y, b.rect.w, b.rect.h));
            } else if let Some(e) = &self.ui.editor {
                // (📖 Doc: only its search field opens the keyboard)
                let r = self.ed_rect();
                let f = e.zone_saisie(&text, r.w, r.h);
                z.push((r.x + f.x, r.y + f.y, f.w, f.h));
            } else if let Some(p) = &self.ui.add {
                // ＋ / feuille Symbole : « Symbole, nom… »
                let r = p.field_rect();
                z.push((r.x, r.y, r.w, r.h));
            } else if let Some(d) = &self.ui.ind_dlg {
                // (📱 feuille du cell : seulement les champs où l'on tape ; un toucher sur une case n'ouvre pas le clavier)
                z.extend(d.champs_saisie(&text).into_iter().map(|r| (r.x, r.y, r.w, r.h)));
            } else if let Some(d) = &self.ui.obj_dlg {
                z.push((d.rect.x, d.rect.y, d.rect.w, d.rect.h));
            } else if let Some(f) = &self.ui.j_form {
                z.push((f.rect.x, f.rect.y, f.rect.w, f.rect.h));
            } else if self.watch.explorer.saisie.is_some() && self.watch.prefs.open && self.watch.prefs.panel == wl::Tool::Explorer {
                // 🧭 the Explorer Hyperliquid's address field (its panel): the phone's keyboard, and Ctrl + V pastes
                // (📱 the sheet: the whole window)
                let v = self.volet_rect().unwrap_or(Rect::new(0.0, 0.0, size.width as f64, size.height as f64));
                z.push((v.x, v.y, v.w, v.h));
            } else if self.watch.eco.recherche.is_some() && self.watch.prefs.open && self.watch.prefs.panel == wl::Tool::News {
                // 📰 the Actualités' field « Chercher dans les nouvelles » (its panel), like the Explorer's address:
                // the phone's keyboard opens, Ctrl + V pastes (📱 the sheet: the whole window)
                let v = self.volet_rect().unwrap_or(Rect::new(0.0, 0.0, size.width as f64, size.height as f64));
                z.push((v.x, v.y, v.w, v.h));
            } else if let Some(n) = self.ui.notifs.as_ref().filter(|n| !n.fil && n.alertes.recherche.is_some()) {
                // 🔔 the alerts panel's field « Chercher une alerte »: the phone's keyboard opens, Ctrl + V pastes
                let v = n.alertes_place(size.width as f64, size.height as f64);
                z.push((v.x, v.y, v.w, v.h));
            }
            platform::web::set_text_zones(z);
        }
        let ai_m = Some((self.pointer.win_mouse.0 - ai_r.x, self.pointer.win_mouse.1 - ai_r.y));
        let mut ai_pm = self.ui.ai_open.then(|| self.ai.draw(&text, ai_r.w as u32, ai_r.h as u32, ai_m)).flatten();
        // 📐 cell : la barre d'icônes, la rangée du haut et les pilules mesurées dans la maquette
        let cell_e = phone.then(|| self.cell_etat());
        let dock_y = self.top_h() + self.area.1 as f64;
        let mut phone_pm = cell_e.as_ref().filter(|_| !self.bas_cache()).and_then(|e| {
            let m = self.survol(self.pointer.phone_mouse).map(|(x, y)| (x, y - dock_y));
            argos_render::ecrans::dock(&text, size.width as f64, e, Theme::current().background, m)
        });
        // 🎡 la roue de la pilule (le doigt glisse dessus)
        let mut roue_pm = self.ui.roue.as_ref().filter(|r| r.ouverte).and_then(|r| {
            let rr = r.rect();
            let lignes: Vec<String> = r.lignes.iter().map(|(t, _)| t.clone()).collect();
            argos_render::ecrans::roue(&text, &lignes, r.ligne(), r.fraction(), rr.w, rr.h, crate::actions::roue::LIGNE).map(|p| (p, rr))
        });
        let mut cell_ov: Vec<(tiny_skia::Pixmap, Rect)> = match &cell_e {
            Some(e) if self.ui.accueil.is_none() && !self.ui.zen => {
                let (w, h) = (self.win_w(), self.cell_h());
                let m = self.survol(self.pointer.phone_mouse);
                let mut v: Vec<_> = (!e.haut_cache)
                    .then(|| argos_render::ecrans::haut(&text, w, e, None, m))
                    .flatten()
                    .map(|p| {
                        let r = Rect::new(0.0, 0.0, argos_render::px::lw(&p), argos_render::ecrans::haut_h());
                        (p, r)
                    })
                    .into_iter()
                    .collect();
                // Argos AI : seulement s'il est à sa place (une feuille ouverte le cache)
                let ai = self.ai_bouton_rect();
                v.extend(
                    argos_render::ecrans::flottants(&text, w, h, e, m)
                        .into_iter()
                        .filter(|(_, r)| ai.w > 0.0 || *r != argos_render::ecrans::ai_rect(w, h, e.bas_cache)),
                );
                // 👆 « ⋯ » of the finger's crosshair, beside its price label (not under a sheet)
                if let Some(r) = self.croix_bouton().filter(|_| self.ui.pfeuille.is_none()) {
                    v.extend(argos_render::ecrans::bouton_points(r.w, m.is_some_and(|(x, y)| r.contains(x, y))).map(|pm| (pm, r)));
                }
                v
            }
            _ => vec![],
        };
        // 📱 ⚡ a perpetual's Funding pill, right of « ⌄ N », and its open card (laid on the chart under the « ton
        // trade » card, below)
        let mut funding_pm = if phone && self.ui.accueil.is_none() && !self.ui.zen {
            self.funding_images(&text)
        } else {
            vec![]
        };
        // 🌞 thème clair : menus, barres et panneaux en clair (la chart a déjà ses couleurs)
        if argos_render::palette::is_light(self.top.prefs.theme) {
            use argos_render::palette::light_ui;
            for pm in [&mut top, &mut side]
                .into_iter()
                .chain(left.iter_mut())
                .chain(pf_pm.iter_mut())
                .chain(bt_pm.iter_mut())
                .chain(mk_pm.iter_mut())
                .chain(ac_pm.iter_mut())
                .chain(phone_pm.iter_mut())
            {
                light_ui(pm);
                argos_render::palette::on_white(pm);
            }
            for (pm, _) in sheet
                .iter_mut()
                .chain(argos.iter_mut())
                .chain(split_menu.iter_mut())
                .chain(tree.iter_mut())
                .chain(bar.iter_mut())
                .chain(pop.iter_mut())
                .chain(tip.iter_mut())
                .chain(list.iter_mut())
            {
                light_ui(pm);
            }
            for (pm, _) in add
                .iter_mut()
                .chain(marches.iter_mut())
                .chain(actu_cartes.iter_mut())
                .chain(name_box.iter_mut())
                .chain(key_box.iter_mut())
                .chain(obj_dlg.iter_mut())
                .chain(ind_dlg.iter_mut())
                .chain(j_form.iter_mut())
                .chain(editor.iter_mut())
                .chain(rail_tip.iter_mut())
                .chain(col_tip.iter_mut())
            {
                light_ui(pm);
            }
            for (pm, _, _) in list_tip.iter_mut() {
                light_ui(pm);
            }
            for pm in ai_pm.iter_mut() {
                light_ui(pm);
            }
            for (pm, _) in cell_ov.iter_mut().chain(funding_pm.iter_mut()) {
                light_ui(pm);
            }
            for (pm, _) in roue_pm.iter_mut() {
                light_ui(pm);
            }
        }
        let light = argos_render::palette::is_light(self.top.prefs.theme);
        let side_xp = self.side_x(); // 🫧 bord gauche du panneau de droite (verre)
                                     // (📱 the single pill: no mini DÉMO pill, the « ● DÉMO » chip sits beside the title)
        let barres_cachees = self.haut_cache() && !self.pilule();
        // (pas par-dessus une feuille plein écran : réglages d'un indicateur, page Indicateurs)
        let plein_ecran = self.ui.fx_page.is_some() || self.ui.ind_dlg.as_ref().is_some_and(|d| d.cell);
        // (📱 feuille ARGOS ouverte : pas de poignée par-dessus ses lignes)
        let poignees = (phone && !self.pilule() && self.ui.accueil.is_none() && !plein_ecran && self.ui.argos.is_none())
            .then(|| (self.poignee_haut(), self.poignee_bas()));
        let estompe = self.barres_estompees();
        let (mini_demo, mini_ai, ai_case) = (self.mini_demo_rect(), self.ai_rond(), self.ai_bouton_rect());
        let ai_page = self.page_ouverte();
        let mini_demo_txt = (Self::MINI_DEMO_TXT.to_string(), argos_render::palette::GOLD);
        let top_wd = dev(self.top_w()); // largeur de la barre du haut (double écran : l'écran de gauche)
        let t_ui = Instant::now();
        // la surface de la fenêtre ; 🧪 sans fenêtre (tests) : une image en mémoire
        let mut tampon_test: Vec<u32> = vec![];
        let mut surface_buf = None;
        if let Some(surface) = &mut self.surface {
            if surface.resize(w, h).is_err() {
                return;
            }
            let Ok(b) = surface.buffer_mut() else { return };
            surface_buf = Some(b);
        } else {
            tampon_test = vec![0; w.get() as usize * h.get() as usize];
        }
        let buf: &mut [u32] = match surface_buf.as_mut() {
            Some(b) => b,
            None => &mut tampon_test,
        };
        let (ww, cw, rwu) = (w.get() as usize, dev(self.area.0 as f64), dev(rw));
        // rangée par rangée (pixels de l'écran) : barre du haut, puis barre de gauche | chart | watchlist (📱 puis la
        // barre du bas)
        let th_ = dev(top_h);
        // 🖥 à droite de la barre du haut (avant la colonne d'icônes) : le fond de la barre, sans bord ni bande noire
        let fond_haut = {
            let c = Theme::current().chrome;
            (c.0 as u32) << 16 | (c.1 as u32) << 8 | c.2 as u32
        };
        let chart_end = th_ + dev(self.area.1 as f64);
        let chart_pm = area_pm.as_ref().unwrap_or(&pm);
        for (wy, row) in buf.chunks_mut(ww).enumerate() {
            if wy < th_ {
                // (double écran : la barre ne couvre que l'écran de gauche ; à droite, le fond du thème)
                copy_row_on(row, Some(&top), wy, if light { 0x00f3_f4f8 } else { fond_haut });
                continue;
            }
            if phone {
                match (&phone_pm, self.ui.rows) {
                    (Some(b), _) if wy >= chart_end => copy_row(row, Some(b), wy - chart_end),
                    // 🫧 feuille en verre : la chart entière dessous (la feuille est posée en verre après)
                    (_, Some(_)) if self.ui.feuille_verre => copy_row(row, Some(chart_pm), wy - th_),
                    // segment du haut : la chart ; charnière : rien ; segment du bas : le panneau ouvert
                    (_, Some((top, bot))) => {
                        let y = wy - th_;
                        if y < dev(top) {
                            copy_row(row, Some(chart_pm), y);
                        } else if side_w > 0 && y >= dev(bot) {
                            copy_row(row, Some(&side), y - dev(bot));
                        } else {
                            row.fill(if light { 0x00f3_f4f8 } else { 0 });
                        }
                    }
                    // 🫧 style verre : la chart dessous, le panneau posé en verre après
                    _ => copy_row(row, Some(if side_w > 0 && !argos_render::glass::on() { &side } else { chart_pm }), wy - th_),
                }
                continue;
            }
            let y = wy - th_;
            let (l, rest) = row.split_at_mut(rwu.min(ww));
            let (c, rest) = rest.split_at_mut(cw.min(rest.len()));
            let (d, s) = rest.split_at_mut(dev(self.dock.right).min(rest.len()));
            copy_row(l, left.as_ref(), y);
            copy_row(c, Some(chart_pm), y);
            // 🌞 thème clair : le fond de la colonne de modules en clair (sinon noir)
            if light {
                d.fill(0x00f3_f4f8);
            } else {
                copy_row(d, None, y);
            }
            copy_row_on(s, Some(&side), y, if light { 0x00f3_f4f8 } else { 0 });
        }
        if let Some((pm, r)) = &dock_pm {
            blit(buf, ww, h.get() as usize, pm, r.x, r.y);
        }
        let wh = h.get() as usize;
        // 🖥 grand écran : les gros blocs (barre du haut, barre de gauche, panneau de droite) sont posés tels quels. Avant,
        // posés « en verre », leur image rectangulaire recevait le reflet du haut et le liseré de 1 px tout autour : un
        // grand cadre à coins droits autour des rangées du haut et une bande droite à droite (captures de Jee, 16 nov.).
        // Les volets ont leur propre carte arrondie ; les rangées, la colonne et le rail n'ont plus de cadre (règle de Jee).
        if !phone && argos_render::glass::on() {
            let t = Instant::now();
            // (🧘 Zen: the top bar only while the mouse brings it back)
            if !self.ui.zen || self.ui.zen_haut {
                blit(buf, ww, wh, &top, 0.0, 0.0);
            }
            if let Some(l) = &left {
                blit(buf, ww, wh, l, 0.0, top_h);
            }
            if side_w > 0 {
                blit(buf, ww, wh, &side, side_xp, top_h);
            }
            self.ui.glass_now += t.elapsed().as_secs_f64() * 1000.0;
        }
        // 🧘 Zen without the glass: the top bar brought back at the top edge, over the chart
        if !phone && !argos_render::glass::on() && self.ui.zen && self.ui.zen_haut {
            blit(buf, ww, wh, &top, 0.0, 0.0);
        }
        // 📱 ⚡ a perpetual's Funding pill, right of « ⌄ N », and its open card: on the chart, under everything laid over
        // it (the « ton trade » card, the sheets, the menus, the windows)
        for (pm, r) in &funding_pm {
            argos_render::glass::blit(buf, ww, wh, pm, r.x, r.y, light);
        }
        // 🤖 la carte « Argos AI · ton trade » (lot 7), en verre sur la chart (🖥 en bas à gauche, 📱 au-dessus de la
        // barre du bas) : sous tout ce qui s'ouvre (volets, barres, menus, fenêtres), qui prend aussi le clic avant elle
        if let Some((pm, r)) = &avis_pm {
            if argos_render::glass::on() {
                argos_render::glass::blit(buf, ww, wh, pm, r.x, r.y, light);
            } else {
                blit(buf, ww, wh, pm, r.x, r.y);
            }
        }
        // 🖥 le volet flottant, en verre, par-dessus la barre du haut et la watchlist (sous la colonne d'icônes)
        if let Some((pm, v)) = &volet {
            if argos_render::glass::on() {
                argos_render::glass::blit(buf, ww, wh, pm, v.x, v.y, light);
            } else {
                blit(buf, ww, wh, pm, v.x, v.y);
            }
        }
        // 🖥 la colonne d'icônes de la maquette PC, par-dessus (du haut au bas de la fenêtre) : posée telle quelle (son
        // image est déjà la longue pilule de verre de sa bande repliée, Jee 19 nov.), jamais avec le verre rectangulaire
        if let Some((pm, r)) = &colonne {
            blit(buf, ww, wh, pm, side_xp + r.x, top_h + r.y);
        }
        // 🫥 PC : la rangée du haut et la barre de gauche s'estompent quand on travaille sur la chart (maquette :
        // « Estomper les barres sur la chart ») ; elles reviennent dès que la souris s'en approche
        if estompe {
            let fond = if light { 0x00f3_f4f8u32 } else { 0x000a_0d14 };
            let mix = |p: u32| -> u32 {
                let c = |v: u32, s: u32| ((s >> v & 0xff) as f64, (fond >> v & 0xff) as f64);
                let m = |v: u32| {
                    let (a, b) = c(v, p);
                    ((b + (a - b) * ESTOMPE) as u32) << v
                };
                m(16) | m(8) | m(0)
            };
            for (wy, row) in buf.chunks_mut(ww).enumerate() {
                if wy < th_ {
                    row[..top_wd.min(ww)].iter_mut().for_each(|p| *p = mix(*p));
                } else if wy < chart_end {
                    row[..rwu.min(ww)].iter_mut().for_each(|p| *p = mix(*p));
                }
            }
        }
        // 🫧 📱 panneau ouvert sur cell en style verre : par-dessus la chart floutée
        // 📐 cell : la rangée du haut et les pilules de la maquette, posées sur la chart (sous les feuilles et menus)
        // (en verre : la chart dessous est floutée, comme le backdrop-filter de la maquette)
        let ai_cell = mini_ai;
        for (pm, r) in &cell_ov {
            if *r == ai_cell {
                blit(buf, ww, wh, pm, r.x, r.y); // Argos AI : son propre verre (translucide)
            } else {
                argos_render::glass::blit(buf, ww, wh, pm, r.x, r.y, light);
            }
        }
        if let Some((pm, r)) = &roue_pm {
            argos_render::glass::blit(buf, ww, wh, pm, r.x, r.y, light);
        }
        if let (true, true, Some((_, bot))) = (self.ui.feuille_verre, side_w > 0, self.ui.rows) {
            argos_render::glass::blit(buf, ww, wh, &side, 0.0, top_h + bot, light);
        }
        if phone && side_w > 0 && self.ui.rows.is_none() && argos_render::glass::on() {
            let t = Instant::now();
            argos_render::glass::blit(buf, ww, wh, &side, 0.0, top_h, light);
            self.ui.glass_now += t.elapsed().as_secs_f64() * 1000.0;
        }
        if let Some((pm, at)) = &bar {
            // coupée avant l'échelle de prix : jamais par-dessus les prix ni la barre d'icônes (fenêtre étroite, cell)
            let edge = (cw as f64).min(self.chart.viewport.plot_w());
            let vis = (dev((edge - at.x).max(0.0)) as u32).min(pm.width());
            if let Some(part) = tiny_skia::IntRect::from_xywh(0, 0, vis.max(1), pm.height()).and_then(|ir| pm.clone_rect(ir)) {
                // 🫧 style verre : la pilule laisse voir la chart floutée
                if argos_render::glass::on() {
                    let t = Instant::now();
                    argos_render::glass::blit(buf, ww, wh, &part, rw + at.x, at.y + top_h, light);
                    self.ui.glass_now += t.elapsed().as_secs_f64() * 1000.0;
                } else {
                    blit(buf, ww, wh, &part, rw + at.x, at.y + top_h);
                }
            }
        }
        if let Some((pm, r)) = &deplier {
            if argos_render::glass::on() {
                argos_render::glass::blit(buf, ww, wh, pm, r.x, r.y, light);
            } else {
                blit(buf, ww, wh, pm, r.x, r.y);
            }
        }
        if let Some((pm, r)) = &pied {
            if argos_render::glass::on() {
                argos_render::glass::blit(buf, ww, wh, pm, r.x, r.y, light);
            } else {
                blit(buf, ww, wh, pm, r.x, r.y);
            }
        }
        if let Some(pm) = &rail_ouvert {
            if argos_render::glass::on() {
                argos_render::glass::blit(buf, ww, wh, pm, crate::actions::RAIL_X, rail_y0, light);
            } else {
                blit(buf, ww, wh, pm, crate::actions::RAIL_X, rail_y0);
            }
        }
        if let Some((pm, r)) = &rail_garde {
            blit(buf, ww, wh, pm, r.x, r.y);
        }
        // 🖥 rangées repliées ouvertes au survol : toute la barre du haut par-dessus la chart (et par-dessus le rail) ;
        // un volet ouvert passe au-dessus d'elles (maquette : #panneau devant les rangées) : elles s'arrêtent à son bord
        if haut_ouvert {
            // (seulement les rangées 2 et 3, entre leurs bords : ni le rail, ni le volet, ni la colonne dessous)
            let (g, d) = rangs.unwrap_or((0.0, argos_render::px::lw(&top)));
            let d = if volet_ouvert { d.min(side_xp) } else { d }.min(argos_render::px::lw(&top));
            let part = argos_render::px::int_rect(g, top_h, (d - g).max(1.0), (argos_render::px::lh(&top) - top_h).max(1.0)).and_then(|r| top.clone_rect(r));
            // (🫧 their pills alone, over the chart: posed as they are — the glass pose of their rectangle drew a
            // square frame and a straight light line down its left edge, Jee's capture of 17 Nov)
            if let Some(p) = part {
                blit(buf, ww, wh, &p, g, top_h);
            }
        }
        if let Some((pm, r)) = &bande {
            if argos_render::glass::on() {
                argos_render::glass::blit(buf, ww, wh, pm, r.x, r.y, light);
            } else {
                blit(buf, ww, wh, pm, r.x, r.y);
            }
        }
        // 🏦 le ⚙ du Footprint (bourses), en verre : 🖥 sous la légende, 📱 la feuille en bas (au-dessus de la carte, des barres et de la barre flottante)
        if let Some((pm, r)) = &fp_pm {
            if argos_render::glass::on() {
                argos_render::glass::blit(buf, ww, wh, pm, r.x, r.y, light);
            } else {
                blit(buf, ww, wh, pm, r.x, r.y);
            }
        }
        // 🔊 « Son des alertes » opens over the Réglages panel: laid after it, never under it
        let reg_list = list.take_if(|_| reg_pm.is_some() && matches!(self.ui.list.as_ref().map(|(k, _)| k), Some(ListKind::SonAlertes)));
        // 🧠 what the Argos AI card opens goes over the card, never under it (Jee, 19 Nov: the memory menu opened
        // behind the card): its memory menu, the box of a note or of the method, the settings of « Tous les réglages »
        // (the 🔑 box is already over everything); every other menu keeps its place
        let (ai_list, ai_name, ai_dlg) = if ai_pm.is_some() {
            let du_ai = |k: Option<&ListKind>| matches!(k, Some(ListKind::AiMemory));
            (
                list.take_if(|_| du_ai(self.ui.list.as_ref().map(|(k, _)| k))),
                name_box.take_if(|_| {
                    self.ui
                        .name_box
                        .as_ref()
                        .is_some_and(|b| matches!(b.kind, argos_render::wlmenu::NameFor::AiNote | argos_render::wlmenu::NameFor::AiMethod))
                }),
                ind_dlg.take_if(|_| {
                    self.ui
                        .ind_dlg
                        .as_ref()
                        .is_some_and(|d| d.script.is_none() && Some(d.ind) == self.ai.tous_reglages)
                }),
            )
        } else {
            (None, None, None)
        };
        // 🫧 style verre : les menus et fenêtres flottantes laissent voir la chart floutée derrière eux
        let glass = argos_render::glass::on();
        let spent = std::cell::Cell::new(0.0);
        let card = |buf: &mut [u32], pm: &tiny_skia::Pixmap, x: f64, y: f64| {
            if glass {
                let t = Instant::now();
                argos_render::glass::blit(buf, ww, wh, pm, x, y, light);
                spent.set(spent.get() + t.elapsed().as_secs_f64() * 1000.0);
            } else {
                blit(buf, ww, wh, pm, x, y);
            }
        };
        // 🫧 the tabs of the folded bars and the tooltips, in glass too (Jee, 17 Nov: everything glass)
        for (pm, r) in &tabs {
            card(buf, pm, r.x, r.y);
        }
        if let Some((pm, r)) = &tip {
            let x = (r.cx() - argos_render::px::lw(pm) / 2.0).max(2.0);
            card(buf, pm, rw + x, r.y + r.h + 10.0 + top_h);
        }
        // ⌂ l'Accueil en verre : son fond laisse voir la chart floutée ; les menus, boîtes et Argos AI passent par-dessus
        if let Some(pm) = &acc_pm {
            card(buf, pm, 0.0, 0.0);
        }
        if let Some((pm, r)) = &acc_band {
            card(buf, pm, r.x, r.y);
        }
        if let Some((pm, r)) = &pop {
            card(buf, pm, rw + r.x, r.y + top_h);
        }
        // 🖍 the bar of the selected drawing, in glass, above the chart (under the menus)
        if let Some((pm, r)) = &drawbar_pm {
            card(buf, pm, rw + r.x, top_h + r.y);
        }
        // 🫧 the right-click menu and « Aller à », in glass (drawn apart from the chart image)
        if let Some((pm, r)) = &menus_calque {
            card(buf, pm, rw + r.x, r.y + top_h);
        }
        // (📱 la feuille ▦ du cell est posée plus bas, comme la feuille 🔔 : chart floutée dessous)
        let split_feuille = self.ui.split_menu.as_ref().is_some_and(|m| m.cell);
        for (pm, r) in sheet.iter().chain(&argos).chain(&tree).chain(split_menu.iter().filter(|_| !split_feuille)) {
            card(buf, pm, r.x, r.y);
        }
        let mut couches = vec![];
        // 🔔 the alerts panel (volet or phone sheet, the chart blurred under it): under the windows it opens — a line's
        // menu, ⚙, the colour, « Éditer… », the confirmation
        let alertes_tot = self.ui.notifs.as_ref().is_some_and(|n| !n.fil);
        if let Some((pm, r)) = notif_pm.as_ref().filter(|_| alertes_tot) {
            if argos_render::glass::on() {
                flou_dessous(buf, ww, wh, pm, *r);
            }
            blit(buf, ww, wh, pm, r.x, r.y);
            couches.push("alertes");
        }
        if let Some((m, r)) = &list {
            card(buf, m, r.x, r.y);
            couches.push("menu");
        }
        // 🫧 the editor, an indicator's settings, add a symbol and rename: in glass
        for (m, r) in editor.iter().chain(&add).chain(&marches).chain(&actu_cartes).chain(&name_box).chain(&ind_dlg) {
            card(buf, m, r.x, r.y);
        }
        // 🫧 boîtes en verre (réglages d'un dessin, formulaire du journal) : la chart floutée derrière
        for (m, r) in obj_dlg.iter().chain(&j_form) {
            card(buf, m, r.x, r.y);
        }
        if let Some((pm, r)) = &draw_alert_pm {
            card(buf, pm, r.x, r.y);
        }
        // 🎨 the palette (drawing bar or settings window), over them
        if let Some((pm, r)) = &colorpick_pm {
            card(buf, pm, r.x, r.y);
        }
        if let Some((pm, r)) = &rail_tip {
            card(buf, pm, rw + 4.0, r.cy() - 13.0 + top_h);
        }
        if let Some((pm, x, y)) = &list_tip {
            card(buf, pm, *x, *y);
        }
        if let Some((pm, r)) = &col_tip {
            card(buf, pm, side_xp + r.x - argos_render::px::lw(pm) - 6.0, top_h + r.cy() - 13.0);
        }
        for pm in pf_pm.iter().chain(&bt_pm).chain(&mk_pm).chain(&ac_pm) {
            blit(buf, ww, wh, pm, 0.0, 0.0);
        }
        // ✨ Argos AI : une carte en verre qui flotte à droite (maquette pc-36), par-dessus les pages aussi
        if let Some(pm) = &ai_pm {
            card(buf, pm, ai_r.x, ai_r.y);
            couches.push("argos_ai");
        }
        for ((m, r), nom) in ai_list
            .iter()
            .map(|x| (x, "menu_ai"))
            .chain(ai_name.iter().map(|x| (x, "nom_ai")))
            .chain(ai_dlg.iter().map(|x| (x, "reglages_ai")))
        {
            card(buf, m, r.x, r.y);
            couches.push(nom);
        }
        // 🔑 la boîte de clé : par-dessus tout (assistant, volets, tableaux des indicateurs), opaque (rien ne se voit au
        // travers) ; la boîte des 24 mots, en verre
        for (m, r) in key_box.iter() {
            blit(buf, ww, wh, m, r.x, r.y);
        }
        // ✅ the confirmation box (« Supprimer cette alerte? »): over everything, the window darkened under it
        if let Some(p) = &confirmer_pm {
            blit(buf, ww, wh, p, 0.0, 0.0);
        }
        for (m, r) in words_box.iter() {
            card(buf, m, r.x, r.y);
        }
        if let Some((pm, r)) = &rac_pm {
            card(buf, pm, r.x, r.y);
        }
        if let Some((pm, r)) = &sim_pm {
            // (🖥 in the Trading volet: over its glass, without a glass of its own)
            if self.watch.panels.trading.sim_onglet {
                blit(buf, ww, wh, pm, r.x, r.y);
            } else {
                card(buf, pm, r.x, r.y);
            }
        }
        // (the DÉMO band stays as it is: its gold outline and dot are its most frequent colour, the glass would take
        // them for the background and hollow them out)
        if let Some((pm, r)) = &band_pm {
            blit(buf, ww, wh, pm, r.x, r.y);
        }
        // 📱 barres cachées : il ne reste que la mini-pastille DÉMO (P&L du jour)
        if barres_cachees {
            let text = argos_render::text::Text::default();
            if let Some(mut p) = argos_render::px::new(mini_demo.w as u32 + 1, mini_demo.h as u32 + 1) {
                let r = Rect::new(0.0, 0.0, mini_demo.w, mini_demo.h);
                argos_render::phonebar::pastille_demo(&mut p, &text, r, &mini_demo_txt.0, mini_demo_txt.1);
                blit(buf, ww, wh, &p, mini_demo.x, mini_demo.y);
            }
        }
        // 📱 les deux poignées (comme les barres latérales du PC) : une fine pilule, discrète, au bord de chaque barre
        if let Some((h, b)) = poignees {
            for r in [h, b] {
                if let Some(mut p) = argos_render::px::new(r.w as u32, r.h as u32) {
                    let l = Rect::new(r.w / 2.0 - 18.0, r.h / 2.0 - 2.5, 36.0, 5.0);
                    argos_render::shapes::round_fill(&mut p, l, 2.5, argos_render::jetons::DIM, 0.8);
                    blit(buf, ww, wh, &p, r.x, r.y);
                }
            }
        }
        // 📅 the month report (lot 8): over the chart and the Simulator panel (📱 the whole screen, over the handles);
        // in glass, the chart under it blurred, then the page laid as it is (the glass pose would take its teal
        // « Imprimer en PDF » for the background and hollow it out)
        if let Some((pm, r)) = &month_pm {
            if glass {
                flou_dessous(buf, ww, wh, pm, *r);
            }
            blit(buf, ww, wh, pm, r.x, r.y);
        }
        // 🅘 page Indicateurs (cell, tablette) : par-dessus tout, sous Argos AI et la recherche
        // (posée telle quelle : le flou du verre prenait le blanc du texte pour le fond quand la liste est courte)
        // 🫧 the Symbols window and the PC Settings window: in glass
        if let Some((pm, r)) = &sym_pm {
            card(buf, pm, r.x, r.y);
        }
        if let Some((pm, r)) = &reg_pm {
            card(buf, pm, r.x, r.y);
            couches.push("reglages");
        }
        if let Some((m, r)) = &reg_list {
            card(buf, m, r.x, r.y);
            couches.push("menu_reglages");
        }
        self.ui.couches_test = couches;
        if let Some((pm, r)) = &sup_pm {
            blit(buf, ww, wh, pm, r.x, r.y);
        }
        if let Some(pm) = &fx_pm {
            blit(buf, ww, wh, pm, fx_r.x, fx_r.y);
        }
        // 🗺 plan de l'app (cell) : page pleine en verre, la chart floutée derrière (comme la maquette) : le fond teinté
        // passe par le verre, le contenu est posé net par-dessus
        if let Some(pm) = &plan_pm {
            if glass {
                if let Some(f) = argos_render::plan::fond(size.width as f64, size.height as f64) {
                    card(buf, &f, 0.0, 0.0);
                }
            }
            blit(buf, ww, wh, pm, 0.0, 0.0);
        }
        // 🔔 feuille des notifications et 📱 ▦ feuille Splits du cell, posées en bas, en verre (la chart floutée
        // derrière, comme la maquette)
        for (pm, r) in notif_pm
            .iter()
            .filter(|_| !alertes_tot)
            .chain(split_menu.iter().filter(|_| split_feuille))
            .chain(pfeuille_pm.iter())
        {
            // 🌑 ▦ l'ombre portée de la maquette, au-dessus du bord du haut de la feuille
            if (split_feuille && split_menu.as_ref().is_some_and(|(_, q)| q == r)) || pfeuille_pm.as_ref().is_some_and(|(_, q)| q == r) {
                let sc = argos_render::px::dev(1.0);
                let (y_bas, haut) = (argos_render::px::dev(r.y).round().max(0.0) as usize, (40.0 * sc) as usize);
                for j in y_bas.saturating_sub(haut)..y_bas.min(wh) {
                    let a = argos_render::jetons::ombre_au_dessus((y_bas - j) as f64 / sc);
                    for c in &mut buf[j * ww..(j + 1) * ww] {
                        let f = |v: u32| ((v & 0xff) as f32 * (1.0 - a)) as u32;
                        *c = f(*c >> 16) << 16 | f(*c >> 8) << 8 | f(*c);
                    }
                }
            }
            // la chart sous la feuille est floutée, puis la feuille (teinte de la maquette à 60 %) posée dessus
            if argos_render::glass::on() {
                flou_dessous(buf, ww, wh, pm, *r);
            }
            blit(buf, ww, wh, pm, r.x, r.y);
        }
        // ✨ bouton de l'assistant (logo ÂI) : par-dessus tout
        // 📱 sur cell, Argos AI est dans la barre du bas
        let ai = mini_ai;
        // (📱 sur la chart du cell, Argos AI est une des pilules de la maquette, posée plus haut ; sur une page —
        // bienvenue, Accueil, Académie… — il est ici, au coin, par-dessus tout)
        // (choix B de Jee : partout, même par-dessus un menu ou une feuille ouverts sur la chart du cell)
        let par_dessus = plein_ecran
            || self.ui.list.is_some()
            || self.ui.sheet.is_some()
            || self.ui.context.is_open()
            || self.ui.pop.is_some()
            || self.watch.prefs.open
            || self.ui.sim.is_some()
            || self.ui.notifs.as_ref().is_some_and(|n| !n.volet)
            || split_feuille
            || self.ui.argos.is_some();
        if let Some(pm) = (ai.w > 0.0 && (!phone || ai_page || par_dessus))
            .then(|| {
                // (🖥 hover: the whole corner cell; the Argos AI pill fills it)
                let survol = !self.pointer.doigt_leve && ai_case.contains(self.pointer.win_mouse.0, self.pointer.win_mouse.1);
                if phone {
                    argos_render::aibtn::image(ai.w, survol)
                } else {
                    argos_render::aibtn::pastille_image(ai.w, ai.h, survol)
                }
            })
            .flatten()
        {
            blit(buf, ww, wh, &pm, ai.x, ai.y);
        }
        // 🔎 la recherche par-dessus tout
        if let Some((pm, r)) = &pal_pm {
            // voile de la maquette (`#palette` : rgba(5, 7, 12, 0.45)) sur tout l'écran, sous la boîte
            voile(buf, (5, 7, 12), 0.45);
            card(buf, pm, r.x, r.y);
        }
        // tout le verre de l'image compte (Argos AI, l'Accueil, la recherche… compris)
        self.ui.glass_now += spent.get();
        // 📐 double écran côte à côte : rien sous la charnière (ni texte ni bouton), de haut en bas
        if let Some(argos_render::formfactor::Split::Columns { at, gap }) = form.split.filter(|_| !phone) {
            let z = form.zoom.max(0.5);
            let (x0, x1) = (dev((at / z).floor()).min(ww), dev(((at + gap) / z).ceil()).min(ww));
            let bg = if light { 0x00f3_f4f8 } else { 0 };
            for row in buf.chunks_mut(ww) {
                row[x0..x1].fill(bg);
            }
        }
        // 📱 zones sûres (encoche, barre du bas de l'iPhone) : la page les peint de la couleur du fond du haut et du bas
        // de l'app, selon le thème. Fixed colours (Jee, 24 Nov: « quand je bouge la chart les couleurs changent en
        // haut »): before, the first and last pixels of the image — an indicator's line or zone in the corner coloured
        // the iPhone's status bar.
        let chrome = chrome_colours(&Theme::current(), phone, light);
        if chrome != self.ui.chrome {
            self.ui.chrome = chrome;
            #[cfg(target_arch = "wasm32")]
            platform::web::call_page("argosChrome", &[&format!("#{:06x}", chrome.0), &format!("#{:06x}", chrome.1)]);
        }
        let t_copie = Instant::now();
        // 📤 web : seule la bande de rangées qui a changé depuis l'image d'avant est convertie et envoyée au canvas (rien si
        // rien n'a changé) ; ailleurs, l'image entière (softbuffer y alterne ses tampons)
        #[cfg(target_arch = "wasm32")]
        {
            // l'image gardée prend la nouvelle (seulement la bande changée) avant l'envoi, qui consomme le tampon
            let taille = (ww, h.get() as usize);
            let bande = if self.ui.image_taille != taille {
                Bande::Tout
            } else {
                bande_changee(&self.ui.image_avant, buf, ww)
            };
            self.ui.image_taille = taille;
            // 📤 envoi direct : la bande est mise en octets RGBA d'un seul passage (tampon gardé d'une image à l'autre) et
            // posée par un seul putImageData, sans passer par la conversion de softbuffer
            let rangees = match bande {
                Bande::Rien => None,
                Bande::Tout => {
                    self.ui.image_avant.clear();
                    self.ui.image_avant.extend_from_slice(buf);
                    Some((0, taille.1.saturating_sub(1)))
                }
                Bande::Rangees(y0, y1) => {
                    let (a, b) = (y0 * ww, (y1 + 1) * ww);
                    self.ui.image_avant[a..b].copy_from_slice(&buf[a..b]);
                    Some((y0, y1))
                }
            };
            if let (Some((y0, y1)), Some(win)) = (rangees, self.window.as_ref()) {
                use winit::platform::web::WindowExtWebSys;
                if let Some(c) = win.canvas() {
                    rgba_de(&self.ui.image_avant[y0 * ww..(y1 + 1) * ww], &mut self.ui.octets);
                    platform::web::poser_image(&c, &self.ui.octets, ww as u32, (y1 + 1 - y0) as u32, y0 as f64);
                }
            }
        }
        // (la surface est rendue avant la suite : l'emprunt de la fenêtre s'arrête ici)
        #[allow(clippy::let_unit_value)]
        let presentee = surface_buf.map(|b| {
            #[cfg(not(target_arch = "wasm32"))]
            let _ = b.present();
            #[cfg(target_arch = "wasm32")]
            let _ = b;
        });
        if presentee.is_none() {
            self.ui.image_test = tampon_test;
        }
        self.phases_image(t_debut, t_ind, t_chart, t_ui, t_copie);
    }
}

/// 📐 Ce que la mise en page de l'écran donne au dessin (le reste est rangé dans l'app).
pub(crate) struct Placement {
    pub(crate) form: argos_render::formfactor::Choice,
    pub(crate) phone: bool,
    pub(crate) side_w: u32,
    pub(crate) cells: Vec<argos_render::splits::Cell>,
    pub(crate) rw: f64,
    pub(crate) top_h: f64,
}

impl App {
    /// 📐 Mise en page de l'écran pour une fenêtre de `size` points : colonne de modules, panneau de droite, zone de la
    /// chart, taille de la chart, segments du double écran. Une seule fonction, appelée à chaque image et par les tests
    /// (sans fenêtre).
    pub(crate) fn placer(&mut self, size: winit::dpi::PhysicalSize<u32>) -> Placement {
        let form = self.ui.form;
        let phone = form.phone();
        let was_phone = self.top.phone;
        self.top.phone = phone;
        // 📱 touch mode (phone, and a tablet by the finger with the PC bars, Jee 23 Nov): big targets, no keyboard
        // shortcut written, what opens on hover opens on a touch
        argos_render::actions::set_tactile(phone || form.min_target >= argos_render::formfactor::TOUCH_TARGET);
        self.watch.phone = phone.then_some(size.width);
        self.premiere_image(phone);
        // 🧪 PC: the Simulator is the Trading volet's 🧪 tab (module 31 mock-up): it leaves with it (closed, or another
        // volet open); opened elsewhere (DÉMO banner, tablet's pill, Zen mode left, phone view left), it opens it. The
        // phone keeps it full screen, the Zen mode and the dual screen floating
        if self.ui.sim.is_some()
            && !phone
            && !self.ui.zen
            && self.watch.fixed_w.is_none()
            && !(self.watch.prefs.open && self.watch.prefs.panel == wl::Tool::Trading)
        {
            if self.watch.panels.trading.sim_onglet {
                self.ui.sim = None;
            } else {
                self.sim_ouvre_son_volet();
            }
        }
        // 🖥 PC : le volet de droite flotte sous la 1re rangée (choix de Jee : look de la maquette, prix visibles — la
        // chart s'arrête à son bord gauche, l'échelle de prix reste visible) ; ⤢ large : min(920, 62 % de la fenêtre)
        self.watch.volet_flottant = !phone && !self.ui.zen && self.watch.prefs.open;
        self.watch.large_w = (self.watch.volet_flottant && self.ui.volet_large).then(|| (size.width as f64 * 0.62).min(920.0).round() as u32);
        // 🅘 page Indicateurs ouverte dans le volet : elle part avec lui (fermé, ou un autre volet ouvert)
        if self.ui.fx_dans_volet && !(self.watch.prefs.open && self.watch.prefs.panel == wl::Tool::Indicators) {
            (self.ui.fx_page, self.ui.fx_dans_volet) = (None, false);
        }
        // 🔎 volet Symboles : il part avec le volet (fermé, ou un autre volet ouvert)
        if self.ui.symboles.is_some() && !(self.watch.prefs.open && self.watch.prefs.panel == wl::Tool::Watchlist && !phone) {
            self.ui.symboles = None;
        }
        // 🔔 volet des notifications : il part avec le volet (fermé, ou un autre volet ouvert)
        if self.ui.notifs.as_ref().is_some_and(|n| n.volet) && !(self.watch.prefs.open && self.watch.prefs.panel == wl::Tool::Alertes && !phone) {
            self.ui.notifs = None;
        }
        // ⚙ volet Réglages : il part avec le volet (fermé, ou un autre volet ouvert)
        if self.ui.reglages.is_some() && !(self.watch.prefs.open && self.watch.prefs.panel == wl::Tool::Reglages && !phone) {
            self.ui.reglages = None;
        }
        // 🛟 Support panel: same
        if self.ui.support.is_some() && !(self.watch.prefs.open && self.watch.prefs.panel == wl::Tool::Support && !phone) {
            self.ui.support = None;
        }
        // 📈 volet Trading : la feuille de la maquette sur cell et sur PC (⋯ Toutes les options = l'ancien ticket)
        self.watch.panels.trading.cell = phone || self.watch.volet_flottant;
        // 🧩 colonne de modules (grand écran) : à gauche ou à droite de la chart, ou sous elle ; la chart garde au moins
        // 480 px de large et la moitié de la hauteur
        use argos_render::dock::DockPos;
        self.watch.dock_big = size.width >= argos_render::dock::MIN_WIN_W;
        // (n° 19: `rw()` holds the column of the image before when it is on the left: counted in, the room of the next
        // image was short of it, the column went, came back the image after… it blinked; counted without it)
        let room = size.width as f64 - (self.rw() - self.dock.left) - self.watch.width() as f64;
        let dock = (!phone && self.watch.dock.active(size.width)).then(|| (self.watch.dock.pos, self.dock_epaisseur()));
        // (un volet ouvert : la chart peut descendre à 360 pour garder la colonne de modules qu'on règle dans ce volet)
        let min_chart = if self.watch.prefs.open { 360.0 } else { 480.0 };
        let dock = dock.filter(|(p, t)| !self.ui.zen && (*p == DockPos::Bottom || room - t >= min_chart));
        (self.dock.left, self.dock.right) = match dock {
            Some((DockPos::Left, t)) => (t, 0.0),
            Some((DockPos::Right, t)) => (0.0, t),
            _ => (0.0, 0.0),
        };
        // 📐 double écran côte à côte : la chart dans le segment de gauche, rien sous la charnière (le vide de la
        // colonne de modules), le panneau actif occupe le segment de droite
        use argos_render::formfactor::Split;
        let z = form.zoom.max(0.5);
        self.watch.fixed_w = None;
        if let (Some(Split::Columns { at, gap }), false, None, false) = (form.split, phone, dock, self.ui.zen) {
            let (at, gap) = (at / z, gap / z);
            let right = size.width as f64 - at - gap;
            if right > argos_render::watchlist::ICON_BAR as f64 + 200.0 {
                self.watch.fixed_w = Some((right - argos_render::watchlist::ICON_BAR as f64).round() as u32);
                self.watch.prefs.open = true; // le segment de droite montre toujours le panneau actif
                self.dock.right = gap.round();
            }
        }
        if self.watch.fixed_w.is_some() {
            (self.watch.volet_flottant, self.watch.large_w) = (false, None);
            self.watch.panels.trading.cell = phone;
        }
        // 🧪 the Simulator in the Trading volet's place: its tab lit, the ticket not drawn under it
        self.watch.panels.trading.sim_onglet =
            self.ui.sim.is_some() && self.watch.volet_flottant && self.watch.prefs.panel == wl::Tool::Trading && self.watch.panels.feuille_trading();
        let (rw, top_h) = (self.rw(), self.top_h());
        // 🧘 mode Zen : ni barres ni panneaux, la chart prend tout
        let bottom = if phone { self.bas_reserve() as u32 } else { 0 };
        let side_w = if self.ui.zen {
            0
        } else {
            self.watch.width().min(size.width.saturating_sub(rw as u32))
        };
        // 🧪 PC: the Simulator's panel floats on the right (420 px, mock-up); the chart stops at its left edge, so its
        // price scale and the orders' labels (entry, SL, TP) stay in sight (Suivi 3: they were under the panel). Under
        // the panel: nothing, as under a dual screen's hinge. (A window too narrow: the chart keeps its width.)
        self.dock.sous_sim = 0.0;
        // (🖥 in the Trading volet, it is the volet that the chart stops at)
        if self.ui.sim.is_some() && !phone && !self.watch.panels.trading.sim_onglet {
            let sim_w = argos_render::simulateur::Simulateur::rect(size.width as f64, size.height as f64, top_h).w;
            let sous = sim_w - side_w as f64 - self.dock.right;
            if sous > 0.0 && size.width as f64 - rw - sim_w >= 200.0 {
                self.dock.sous_sim = sous;
                self.dock.right += sous;
            }
        }
        self.area = (
            size.width.saturating_sub(side_w + rw as u32 + self.dock.right as u32),
            size.height.saturating_sub(top_h as u32 + bottom),
        );
        let dock_bottom = match dock {
            Some((DockPos::Bottom, t)) => t.min(self.area.1 as f64 / 2.0).round() as u32,
            _ => 0,
        };
        self.dock.rect = dock.map(|(p, t)| match p {
            DockPos::Left => Rect::new(self.rw() - self.dock.left, top_h, t, self.area.1 as f64),
            DockPos::Right => Rect::new(rw + self.area.0 as f64, top_h, t, self.area.1 as f64),
            DockPos::Bottom => Rect::new(rw, top_h + (self.area.1 - dock_bottom) as f64, self.area.0 as f64, dock_bottom as f64),
        });
        // 📱 la watchlist ouverte cache la chart, qui garde sa taille
        let chart_w = if phone { size.width } else { self.area.0 };
        // 🪟 la chart principale prend la 1re case de la disposition (toute la zone sans split)
        let cells = argos_render::splits::cells(&self.split.layout);
        let main = cells[0];
        self.chart.viewport.width = (main[2] * chart_w as f64).round() as u32;
        self.chart.viewport.height = (main[3] * (self.area.1 - dock_bottom) as f64).round() as u32;
        // 📱 pas de rangée des périodes (1D … Tout, 📅) sur cell : la maquette ne l'a pas
        // 🖥 PC (cadre de la maquette) : plus de barre du bas fixe, un pied flottant au survol (pied_visible)
        self.chart.viewport.footer = 0;
        // 📐 double écran l'un au-dessus de l'autre : la chart dans le segment du haut, le panneau ouvert dans celui du
        // bas (rien sous la charnière)
        self.ui.rows = match form.split {
            Some(Split::Rows { at, gap }) if phone => {
                let (top, bot) = ((at / z - top_h).max(120.0).round(), ((at + gap) / z - top_h).round());
                self.chart.viewport.height = self.chart.viewport.height.min(top as u32);
                if self.ui.rows.is_none() {
                    self.watch.prefs.open = true; // les contrôles en bas, tout de suite
                }
                Some((top, bot))
            }
            // 📱 cell : un volet ouvert est une feuille du bas (maquette tel-04), la chart reste visible au-dessus
            _ if phone && self.watch.prefs.open => {
                let top = self.feuille_volet();
                // 🫧 Trading : la chart garde toute sa hauteur sous la feuille en verre (maquette) ; les autres volets
                // la remontent au-dessus
                if !(self.watch.prefs.panel == wl::Tool::Trading && argos_render::glass::on()) {
                    self.chart.viewport.height = self.chart.viewport.height.min(top as u32);
                }
                Some((top, top))
            }
            _ => None,
        };
        // 📱 en passant sur cell, la barre flottante descend en bas de la chart (comme v1)
        // 📱 en passant sur cell : la barre des favoris repliée en petite pilule ⋮⋮, à sa place de la maquette
        if phone && !was_phone {
            (self.tools.vertical, self.tools.retracted) = (false, true);
            let (w, h) = (self.win_w(), self.cell_h());
            if let Some(r) = argos_render::ecrans::cell().place("favbar", w, h) {
                (self.tools.x, self.tools.y) = (r.x - self.rw(), r.y - self.top_h());
            }
            toolbar::clamp(
                &mut self.tools,
                &argos_render::text::Text::default(),
                self.chart.viewport.plot_w(),
                self.chart.viewport.plot_h(),
            );
        }
        if !phone {
            self.placer_barre();
        }
        // ⚙ volet Réglages : à la place finale du volet (⤢ large, taille de la fenêtre)
        if let (Some(v), Some(g)) = (self.volet_rect(), self.ui.reglages.as_mut()) {
            g.cadre = v;
        }
        // 🛟 Support panel: same
        if let (Some(v), Some(g)) = (self.volet_rect(), self.ui.support.as_mut()) {
            g.cadre = v;
        }
        // ⚙ ses résumés en direct : thème, unité de temps, estompage, son des alertes, accueil au lancement
        if self.ui.reglages.is_some() {
            let k = self.top.prefs.theme;
            let resumes = vec![
                ("Thème", Theme::NAMES.get(k).copied().unwrap_or("").to_string()),
                ("Unité de temps", self.chart.interval.label()),
                (
                    "Estompage des barres",
                    if self.top.prefs.estomper {
                        format!("activé · {} %", (ESTOMPE * 100.0).round())
                    } else {
                        "désactivé".to_string()
                    },
                ),
                (
                    "Accueil",
                    if self.top.prefs.accueil {
                        "s'ouvre au lancement"
                    } else {
                        "ne s'ouvre pas au lancement"
                    }
                    .to_string(),
                ),
                (
                    "Son des alertes",
                    format!("{} · {} %", self.top.prefs.son_alertes, self.top.prefs.volume_alertes),
                ),
            ];
            if let Some(g) = self.ui.reglages.as_mut() {
                g.resumes = resumes;
            }
        }
        // 🔔 volet des notifications : pareil
        if let (Some(v), Some(n)) = (self.volet_rect(), self.ui.notifs.as_mut().filter(|n| n.volet)) {
            n.pop = Some(v);
        }
        // 🔔📱 the alerts panel as the phone's bottom sheet (its handle on top)
        if let Some(n) = self.ui.notifs.as_mut() {
            n.alertes.feuille = n.pop.is_none();
        }
        Placement {
            form,
            phone,
            side_w,
            cells,
            rw,
            top_h,
        }
    }
}

impl App {
    /// 🖥 Barre de favoris (PC) : toujours dans la chart ; repliée d'elle-même quand elle est plus large que la chart (elle
    /// se rouvre quand la place revient) ; jamais déplacée, elle reste au-dessus des panneaux d'indicateurs.
    fn placer_barre(&mut self) {
        // (la légende sous la bande du haut : son décalage d'abord, la barre se place sous elle)
        argos_render::set_haut_legende(self.decalage_legende());
        let text = argos_render::text::Text::default();
        let vp = self.chart.viewport;
        let (pw, ph) = (vp.plot_w(), vp.plot_h());
        let large = {
            let mut t = self.tools.clone();
            t.retracted = false;
            toolbar::layout(&t, &text).0
        };
        // (jamais déplacée : elle doit aussi tenir au-dessus des panneaux d'indicateurs, sinon elle se replie)
        let prix_h = ph - self.ind.out.osc_h();
        // (jamais déplacée : entre la légende — titre, O H L C · Vol, ⌄ — et les panneaux d'indicateurs)
        let sous_legende = 104.0 + argos_render::haut_legende();
        // (liste des indicateurs ouverte, ⌄ : la barre se pose aussi sous ses lignes)
        let sous_legende = match argos_render::indleg::lignes_liste(&self.ind.out).filter(|_| self.ind.leg.0) {
            Some((_, bas)) => sous_legende.max(bas + 8.0),
            None => sous_legende,
        };
        let au_dessus = !self.ui.barre_par_defaut.get() || self.tools.vertical || prix_h >= sous_legende + large.h + 8.0;
        let tient = large.w + 8.0 <= pw && large.h + 8.0 <= ph && au_dessus;
        if !self.tools.retracted && !tient {
            (self.tools.retracted, self.ui.barre_auto_repliee) = (true, true);
        } else if self.ui.barre_auto_repliee && tient {
            (self.tools.retracted, self.ui.barre_auto_repliee) = (false, false);
        }
        if self.ui.barre_par_defaut.get() && !self.tools.vertical {
            let bh = toolbar::layout(&self.tools, &text).0.h;
            self.tools.y = crate::actions::BARRE_MAQUETTE.1.max(sous_legende).min(prix_h - bh - 8.0).max(4.0);
            // (chart trop basse pour passer sous la légende : la barre repliée se pose à sa droite)
            self.tools.x = if self.tools.y < sous_legende {
                LEGENDE_W + 16.0
            } else {
                crate::actions::BARRE_MAQUETTE.0
            };
        }
        toolbar::clamp(&mut self.tools, &text, pw, ph);
        // (rangées du haut repliées : jamais sous leur bande, 2 à 14 sous la barre du haut)
        if self.top.sans_bandeaux && self.tools.y < 18.0 {
            let bh = toolbar::layout(&self.tools, &text).0.h;
            self.tools.y = 18.0f64.min((ph - bh - 4.0).max(4.0));
        }
    }
}

/// 🖥 Haut du volet flottant dans la fenêtre (maquette `#panneau.ouvert{top:66px;bottom:10px}`, l.244) et sa marge du bas.
pub(crate) const VOLET_HAUT: f64 = 8.0 + argos_render::formfactor::RANGEE1_PC + 14.0;
pub(crate) const VOLET_BAS: f64 = 10.0;

impl App {
    /// 🖥 Place du volet flottant dans la fenêtre (à gauche de la colonne d'icônes, sous la 1re rangée, 10 au-dessus du
    /// bas) ; `None` : volet fermé ou pas sur PC.
    pub(crate) fn volet_rect(&self) -> Option<Rect> {
        self.watch.volet_flottant.then(|| {
            let bas = self.top_h() + self.area.1 as f64 - VOLET_BAS;
            Rect::new(self.side_x(), VOLET_HAUT, self.watch.pw(), (bas - VOLET_HAUT).max(40.0))
        })
    }
}

/// 🏷️ Largeur gardée pour la légende de la chart (titre, O H L C · Vol, ⌄) à gauche.
pub(crate) const LEGENDE_W: f64 = 300.0;

/// 🫥 Opacité des barres estompées (maquette : 20 %).
pub(crate) const ESTOMPE: f64 = 0.2;

impl App {
    /// 📱 Haut de la feuille du volet ouvert (dans la zone de la chart) : Trading comme la maquette (`#panneau` : 70 %
    /// de l'écran, posé sur la barre d'icônes) ; les autres volets gardent `feuille_haut`.
    pub(crate) fn feuille_volet(&self) -> f64 {
        let h = self.area.1 as f64;
        // (🎯 the Stratégie sheet as high as the Trading one: its mock-up)
        if matches!(self.watch.prefs.panel, wl::Tool::Trading | wl::Tool::Strategie) {
            let ecran = self.top_h() + h + argos_render::ecrans::dock_h();
            (h - 0.7 * ecran).max(0.0).round()
        } else {
            feuille_haut(h)
        }
    }
}

/// 📱 Haut d'une feuille du bas sur cell (volet, Argos AI) dans une zone de `h` points : la chart garde un bon tiers
/// de l'écran au-dessus, au moins 160 points.
pub(crate) fn feuille_haut(h: f64) -> f64 {
    (h * 0.36).max(160.0).min(h - 120.0).max(0.0).round()
}

/// 🖐 Échelle de rendu la plus haute pendant un geste (glisser, pincer).
const GESTE_SCALE: f64 = 2.0;

/// 🔍 Rendering scale: `want` (the screen's exact density × zoom) when it fits in `limit` (speed cap, pixel budget),
/// at least 1; during a gesture, at most [`GESTE_SCALE`]. When the limit bites, an exact fraction of `want` (½, ⅓…)
/// so the browser enlarges the image by a whole factor, never by a skewed one like ×1.11 (blur).
pub(crate) fn echelle_rendu(want: f64, limit: f64, geste: bool) -> f64 {
    // (the limit bites: the sharper of a whole scale under the limit — iPhone ×3 → ×2 as before — and an exact
    // fraction of `want` — PC ×2 → ×1, never ×1.8)
    let s = if want <= limit + 1e-9 {
        want
    } else {
        limit.floor().max(want / (want / limit.max(1e-9)).ceil())
    };
    if geste { s.min(GESTE_SCALE) } else { s }.max(1.0)
}

/// 🔍 Échelle de rendu la plus haute sur cell et tablette (iPhone : DPR 3).
const MAX_SCALE: f64 = 3.0;
/// 🔍 Pixels de l'écran dessinés au plus par image (cell, tablette). (lot A) 6 M: the largest tablets get their own
/// density too (iPad Pro 12.9" ×2 ≈ 5.6 M pixels; with 4.2 M it fell to ×1, blurred).
const MAX_PIXELS: f64 = 6_000_000.0;
/// 🔍 PC: up to a 5K screen at its exact density (5120 × 2880 ≈ 14.7 M pixels).
const MAX_PIXELS_PC: f64 = 14_800_000.0;

thread_local! {
    /// (temps moyen d'une image en ms, images comptées)
    static PACE: std::cell::Cell<(f64, u32)> = const { std::cell::Cell::new((0.0, 0)) };
}

/// 🖐 (fluidité, lot A) Pixels drawn at most per image on the phone and the tablet while a finger moves (glisser,
/// pincer, défiler): the image is then an exact fraction of the screen's density (iPhone ×2.4 → ×1.2, ×3 → ×1.5,
/// tablet ×2 → ×1), enlarged by a whole factor, until the finger lifts (sharp again at once).
const GESTE_PIXELS: f64 = 1_000_000.0;

/// 🔍 (fluidité, lot A) The highest rendering scale for a screen of `area` points² (the pixel budget: phone and
/// tablet [`MAX_PIXELS`], at most ×3, [`GESTE_PIXELS`] while a finger moves; PC up to a 5K screen). There is no speed
/// cap any more (Jee, 24 Nov: « 4k ultra »): after a slow start the cell stayed at ×1 on its ×3 screen, all blurred,
/// and almost never came back. A slow device lightens the glass, then turns it off, instead (`frame_timing`).
pub(crate) fn limite_echelle(desktop: bool, area: f64, geste: bool) -> f64 {
    if desktop {
        (MAX_PIXELS_PC / area).sqrt()
    } else if geste {
        (GESTE_PIXELS.min(MAX_PIXELS) / area).sqrt().min(MAX_SCALE)
    } else {
        (MAX_PIXELS / area).sqrt().min(MAX_SCALE)
    }
}

/// Note le temps d'une image (moyenne glissante) ; la page garde la mesure avec l'échelle (`argosPace`) pour vérifier
/// sur l'appareil. (lot A) It no longer lowers the scale ([`limite_echelle`]).
fn frame_done(ms: f64) {
    PACE.with(|p| {
        let (avg, n) = p.get();
        let avg = if n == 0 { ms } else { avg * 0.9 + ms * 0.1 };
        let n = n.saturating_add(1);
        #[cfg(target_arch = "wasm32")]
        if n.is_multiple_of(20) {
            let now = argos_render::px::scale() as f64;
            platform::web::call_page("argosPace", &[&format!("{now}"), &format!("{avg:.1}")]);
        }
        p.set((avg, n));
    });
}

/// 🫧 Coût maximal du verre dans une image (ms) avant qu'il se coupe tout seul.
const GLASS_MAX_MS: f64 = 25.0;
/// 🫧 Images lentes d'affilée (≈ 3 s à 60 images par seconde) avant d'alléger ou de couper le verre : jamais sur un
/// pic, seulement si c'est lent pendant plusieurs secondes.
const LENT_IMAGES: u32 = 180;

/// Taille de la fenêtre en points : ses pixels sur desktop ; dans le navigateur, les pixels de la page divisés par
/// l'agrandissement au doigt (le dessin, lui, est fait `px::scale()` fois plus fin).
pub(crate) fn ui_size(w: &Window) -> winit::dpi::PhysicalSize<u32> {
    // navigateur : la taille du canvas en pixels de la page (la taille de winit est fausse sur les écrans ×2, ×3)
    #[cfg(target_arch = "wasm32")]
    {
        use winit::platform::web::WindowExtWebSys;
        // 📐 au doigt sur tablette, l'interface est dessinée plus petite puis agrandie par la page (boutons ≥ 40 px)
        let z = platform::web::view().0;
        if let Some(c) = w.canvas().filter(|c| c.client_width() > 0) {
            return winit::dpi::PhysicalSize::new((c.client_width() as f64 / z) as u32, (c.client_height().max(1) as f64 / z) as u32);
        }
        let l = w.inner_size().to_logical::<u32>(w.scale_factor());
        winit::dpi::PhysicalSize::new(l.width, l.height)
    }
    // (🖥 in points: window pixels / the render scale, the compact factor)
    #[cfg(not(target_arch = "wasm32"))]
    {
        let (s, k) = (w.inner_size(), argos_render::px::scale() as f64);
        winit::dpi::PhysicalSize::new((s.width as f64 / k).round() as u32, (s.height as f64 / k).round() as u32)
    }
}

/// Copie la rangée `sy` d'une image dans la fenêtre (noir hors de l'image).
pub(crate) fn copy_row(dst: &mut [u32], src: Option<&tiny_skia::Pixmap>, sy: usize) {
    copy_row_on(dst, src, sy, 0);
}

/// Comme `copy_row`, avec la couleur `bg` (0x00RRGGBB) là où l'image ne va pas (thème clair : du clair, pas du noir).
pub(crate) fn copy_row_on(dst: &mut [u32], src: Option<&tiny_skia::Pixmap>, sy: usize, bg: u32) {
    let row = src.and_then(|p| {
        let w = p.width() as usize;
        p.pixels().get(sy * w..(sy + 1) * w)
    });
    // ⏩ (vitesse-1) one pass over the row, no look-up per pixel; past the image: `bg`
    let n = row.map_or(0, |r| r.len().min(dst.len()));
    if let Some(r) = row {
        for (d, p) in dst[..n].iter_mut().zip(r) {
            *d = (p.red() as u32) << 16 | (p.green() as u32) << 8 | p.blue() as u32;
        }
    }
    dst[n..].fill(bg);
}

/// Pose une image par-dessus la fenêtre (avec sa transparence), au coin `(x0, y0)`.
/// Pose une image (pixels de l'écran) au point (`x0`, `y0`) d'un tampon de `ww` × `wh` pixels.
/// 📤 Pixels 0RGB (`u32`) → octets RGBA opaques pour le canvas (même conversion que softbuffer), dans `out` gardé d'une
/// image à l'autre.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub(crate) fn rgba_de(px: &[u32], out: &mut Vec<u8>) {
    out.clear();
    out.reserve(px.len() * 4);
    for &p in px {
        out.extend_from_slice(&[(p >> 16) as u8, (p >> 8) as u8, p as u8, 255]);
    }
}

/// 📤 Ce qui a changé entre l'image envoyée avant (`avant`) et la nouvelle (`img`), rangées de `w` pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Bande {
    /// Rien : pas d'envoi.
    Rien,
    /// Taille changée (ou première image) : tout envoyer.
    Tout,
    /// Les rangées `y0..=y1` (la première et la dernière qui ont changé).
    Rangees(usize, usize),
}

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub(crate) fn bande_changee(avant: &[u32], img: &[u32], w: usize) -> Bande {
    if w == 0 || avant.len() != img.len() {
        return Bande::Tout;
    }
    let rangees = || avant.chunks_exact(w).zip(img.chunks_exact(w));
    let Some(y0) = rangees().position(|(a, b)| a != b) else { return Bande::Rien };
    let n = img.len() / w;
    let y1 = n - 1 - rangees().rev().position(|(a, b)| a != b).unwrap_or(0);
    Bande::Rangees(y0, y1)
}

/// Voile de couleur `c` et d'opacité `a` sur toute l'image (0RGB).
pub(crate) fn voile(buf: &mut [u32], c: (u8, u8, u8), a: f32) {
    let k = (a.clamp(0.0, 1.0) * 256.0) as u32;
    let mix = |d: u32, s: u8| (d * (256 - k) + s as u32 * k) >> 8;
    for d in buf.iter_mut() {
        *d = mix((*d >> 16) & 255, c.0) << 16 | mix((*d >> 8) & 255, c.1) << 8 | mix(*d & 255, c.2);
    }
}

/// 🫧 Blurs the image under `pm` laid at `r` (where `pm` is not transparent), as the mock-ups' `backdrop-filter:
/// blur(18px)`; `pm` is then laid over it as it is (`blit`). The glass pose (`glass::blit`) takes the most frequent
/// colour of an image for its background: a sheet or a page with a large coloured button would be hollowed out.
pub(crate) fn flou_dessous(buf: &mut [u32], ww: usize, wh: usize, pm: &tiny_skia::Pixmap, r: Rect) {
    let dev = |v: f64| argos_render::px::dev(v).round().max(0.0) as usize;
    let (x0, y0, pw) = (dev(r.x), dev(r.y), pm.width() as usize);
    let (w, h) = (pw.min(ww.saturating_sub(x0)), (pm.height() as usize).min(wh.saturating_sub(y0)));
    let flou = argos_render::glass::blur_region_r(buf, ww, wh, x0, y0, w, h, 5); // ≈ 18 px, comme la maquette (blur(18px))
    for (k, c) in flou.iter().enumerate() {
        let (i, j) = (k % w.max(1), k / w.max(1));
        if pm.pixels()[j * pw + i].alpha() > 0 {
            buf[(y0 + j) * ww + x0 + i] = (c.0 as u32) << 16 | (c.1 as u32) << 8 | c.2 as u32;
        }
    }
}

pub(crate) fn blit(buf: &mut [u32], ww: usize, wh: usize, pm: &tiny_skia::Pixmap, x0: f64, y0: f64) {
    let (x0, y0) = (argos_render::px::dev(x0).round() as i64, argos_render::px::dev(y0).round() as i64);
    let (pw, ph) = (pm.width() as i64, pm.height() as i64);
    // ⏩ (vitesse-1) the columns inside the window worked out once, then row by row: an opaque pixel is copied, a
    // transparent one skipped, the others mixed (the same sum as before)
    let (xa, xb) = (x0.max(0), (x0 + pw).min(ww as i64));
    if xa >= xb {
        return;
    }
    let px = pm.pixels();
    for py in 0..ph {
        let y = y0 + py;
        if y < 0 || y >= wh as i64 {
            continue;
        }
        let (s0, d0) = ((py * pw + xa - x0) as usize, y as usize * ww + xa as usize);
        // (a buffer cut short: the pixels it still has)
        let n = ((xb - xa) as usize).min(buf.len().saturating_sub(d0));
        let (Some(src), Some(dst)) = (px.get(s0..s0 + n), buf.get_mut(d0..d0 + n)) else {
            continue;
        };
        for (d, p) in dst.iter_mut().zip(src) {
            match p.alpha() {
                0 => {}
                255 => *d = (p.red() as u32) << 16 | (p.green() as u32) << 8 | p.blue() as u32,
                a => {
                    let inv = 255 - a as u32;
                    let mix = |src: u8, shift: u32| (src as u32 + ((*d >> shift) & 255) * inv / 255).min(255);
                    *d = mix(p.red(), 16) << 16 | mix(p.green(), 8) << 8 | mix(p.blue(), 0);
                }
            }
        }
    }
}

/// Icône de la fenêtre : le logo ÂI d'Argos.
pub(crate) fn window_icon() -> Option<winit::window::Icon> {
    let pm = tiny_skia::Pixmap::decode_png(include_bytes!("../../argos-render/assets/argos-ai-256.png")).ok()?;
    // tiny-skia garde les couleurs prémultipliées : on les remet en RGBA droit pour l'icône
    let rgba: Vec<u8> = pm
        .pixels()
        .iter()
        .flat_map(|p| {
            let c = p.demultiply();
            [c.red(), c.green(), c.blue(), c.alpha()]
        })
        .collect();
    winit::window::Icon::from_rgba(rgba, pm.width(), pm.height()).ok()
}

/// 📱 The colours the page paints the iPhone's safe zones with (top: status bar, bottom: home bar): the background of
/// the top and of the bottom of the app in the theme `t` — never what is drawn there. Phone: the chart's background
/// lies under the floating top pill and the bottom bar; larger screens: the top bar's (light: #f3f4f8) above, the
/// chart's below.
pub(crate) fn chrome_colours(t: &Theme, phone: bool, light: bool) -> (u32, u32) {
    let u = |c: argos_render::text::Rgb| (c.0 as u32) << 16 | (c.1 as u32) << 8 | c.2 as u32;
    let fond = u(t.background);
    let haut = if phone {
        fond
    } else if light {
        0x00f3_f4f8
    } else {
        u(t.chrome)
    };
    (haut, fond)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 📱 Jee, 24 Nov (« quand je bouge la chart les couleurs changent en haut »): the safe zones' colours come from the
    /// theme only — the same whatever crosses the corner; light themes give light colours, dark themes dark ones.
    #[test]
    fn safe_zone_colours_follow_the_theme_only() {
        let lum = |c: u32| (c >> 16 & 255) + (c >> 8 & 255) + (c & 255);
        for k in 0..Theme::NAMES.len() {
            let t = Theme::named(k);
            let light = argos_render::palette::is_light(k);
            for phone in [true, false] {
                let (haut, bas) = chrome_colours(&t, phone, light);
                assert_eq!((haut, bas), chrome_colours(&t, phone, light), "{k}: fixed");
                assert_eq!(bas, (t.background.0 as u32) << 16 | (t.background.1 as u32) << 8 | t.background.2 as u32);
                if light {
                    assert!(lum(haut) > 3 * 160 && lum(bas) > 3 * 160, "{k} {phone}: light {haut:06x} {bas:06x}");
                } else {
                    assert!(lum(haut) < 3 * 100 && lum(bas) < 3 * 100, "{k} {phone}: dark {haut:06x} {bas:06x}");
                }
            }
        }
    }

    /// ⏩ vitesse-1: the faster blit and row copy give exactly the pixels of the old ones (kept here as reference),
    /// with half-transparent pixels, images partly outside the window and a buffer shorter than a whole window.
    #[test]
    fn copie_rapide_meme_image() {
        fn blit_avant(buf: &mut [u32], ww: usize, wh: usize, pm: &tiny_skia::Pixmap, x0: f64, y0: f64) {
            let (x0, y0) = (argos_render::px::dev(x0).round() as i64, argos_render::px::dev(y0).round() as i64);
            for py in 0..pm.height() {
                let y = y0 + py as i64;
                if y < 0 || y >= wh as i64 {
                    continue;
                }
                for px in 0..pm.width() {
                    let x = x0 + px as i64;
                    if x < 0 || x >= ww as i64 {
                        continue;
                    }
                    let Some(p) = pm.pixel(px, py).filter(|p| p.alpha() > 0) else { continue };
                    let Some(d) = buf.get_mut(y as usize * ww + x as usize) else { continue };
                    let inv = 255 - p.alpha() as u32;
                    let mix = |src: u8, shift: u32| (src as u32 + ((*d >> shift) & 255) * inv / 255).min(255);
                    *d = mix(p.red(), 16) << 16 | mix(p.green(), 8) << 8 | mix(p.blue(), 0);
                }
            }
        }
        fn copy_avant(dst: &mut [u32], src: Option<&tiny_skia::Pixmap>, sy: usize, bg: u32) {
            let row = src.and_then(|p| {
                let w = p.width() as usize;
                p.pixels().get(sy * w..(sy + 1) * w)
            });
            for (i, d) in dst.iter_mut().enumerate() {
                *d = row
                    .and_then(|r| r.get(i))
                    .map_or(bg, |p| (p.red() as u32) << 16 | (p.green() as u32) << 8 | p.blue() as u32);
            }
        }
        // an image with every kind of pixel: opaque, transparent, half
        let mut pm = tiny_skia::Pixmap::new(37, 23).unwrap();
        for (k, p) in pm.pixels_mut().iter_mut().enumerate() {
            let a = [0u8, 255, 128, 7, 250][k % 5];
            let c = |v: usize| ((v * 37 + k * 11) % 256).min(a as usize) as u8;
            *p = tiny_skia::PremultipliedColorU8::from_rgba(c(1), c(2), c(3), a).unwrap();
        }
        let (ww, wh) = (50usize, 30usize);
        let fond: Vec<u32> = (0..ww * wh).map(|i| (i as u32).wrapping_mul(2_654_435_761) & 0x00ff_ffff).collect();
        for (x, y) in [(0.0, 0.0), (20.0, 15.0), (-10.0, -5.0), (40.0, 25.0), (-40.0, 0.0), (60.0, 0.0), (5.0, 29.0)] {
            let (mut a, mut b) = (fond.clone(), fond.clone());
            blit(&mut a, ww, wh, &pm, x, y);
            blit_avant(&mut b, ww, wh, &pm, x, y);
            assert_eq!(a, b, "blit at ({x}, {y})");
            // a buffer shorter than the window (last rows missing): no panic, the same pixels
            let (mut a, mut b) = (fond[..ww * wh - 70].to_vec(), fond[..ww * wh - 70].to_vec());
            blit(&mut a, ww, wh, &pm, x, y);
            blit_avant(&mut b, ww, wh, &pm, x, y);
            assert_eq!(a, b, "short buffer, blit at ({x}, {y})");
        }
        for w in [10usize, 37, 60] {
            for sy in [0usize, 5, 22, 23, 40] {
                for src in [Some(&pm), None] {
                    let (mut a, mut b) = (vec![1u32; w], vec![1u32; w]);
                    copy_row_on(&mut a, src, sy, 0x123456);
                    copy_avant(&mut b, src, sy, 0x123456);
                    assert_eq!(a, b, "row {sy}, width {w}");
                }
            }
        }
    }

    /// Image `w` × `h` remplie d'une couleur.
    fn image(w: u32, h: u32, c: tiny_skia::Color) -> tiny_skia::Pixmap {
        let mut pm = tiny_skia::Pixmap::new(w, h).unwrap();
        pm.fill(c);
        pm
    }

    #[test]
    fn rangee_copiee_en_rgb_et_noir_hors_image() {
        let pm = image(2, 2, tiny_skia::Color::from_rgba8(0x12, 0x34, 0x56, 255));
        let mut row = [7u32; 3];
        copy_row(&mut row, Some(&pm), 1);
        assert_eq!(row, [0x123456, 0x123456, 0]);
        // rangée sous l'image, ou pas d'image : noir
        copy_row(&mut row, Some(&pm), 5);
        assert_eq!(row, [0, 0, 0]);
        copy_row(&mut row, None, 0);
        assert_eq!(row, [0, 0, 0]);
    }

    #[test]
    fn image_opaque_posee_et_coupee_au_bord() {
        let pm = image(2, 2, tiny_skia::Color::from_rgba8(0xff, 0, 0, 255));
        let mut buf = vec![0u32; 3 * 3];
        // à moitié hors de la fenêtre (en bas à droite)
        blit(&mut buf, 3, 3, &pm, 2.0, 2.0);
        assert_eq!(buf[2 * 3 + 2], 0xff0000);
        assert_eq!(buf.iter().filter(|&&p| p != 0).count(), 1);
    }

    /// 🖐 Densité pendant un geste : ×2 au plus ; au repos (lâcher), la pleine densité revient ; jamais sous 1 ; le
    /// budget de pixels reste respecté.
    #[test]
    fn densite_plafonnee_pendant_le_geste() {
        assert_eq!(echelle_rendu(3.0, 9.0, false), 3.0, "iPhone au repos : ×3");
        assert_eq!(echelle_rendu(3.0, 9.0, true), 2.0, "pendant le geste : ×2");
        assert_eq!(echelle_rendu(1.5, 9.0, true), 1.5, "déjà sous ×2 : inchangé");
        assert_eq!(
            echelle_rendu(3.0, 1.2, false),
            1.0,
            "budget de pixels : une fraction exacte (⅓ ici, au moins 1)"
        );
        assert_eq!(echelle_rendu(2.0, 1.8, false), 1.0, "PC ×2 au-dessus du budget : ×1 (agrandi ×2), jamais ×1,8");
        assert_eq!(echelle_rendu(4.0, 2.5, false), 2.0, "×4 limité : ×2 (½)");
        assert_eq!(echelle_rendu(1.25, 3.0, false), 1.25, "DPR 1,25 (PC de Jee) : exact");
        assert_eq!(echelle_rendu(3.0, 2.0, false), 2.0, "iPhone ralenti : ×2 comme avant, pas ×1,5");
        assert_eq!(echelle_rendu(0.5, 9.0, true), 1.0, "jamais sous 1");
        // 🔍 PC de Jee (DPR 1,25) : toujours ×1,25, jamais étiré
        assert_eq!(echelle_rendu(1.25, limite_echelle(true, 1536.0 * 960.0, false), false), 1.25);
        assert_eq!(
            echelle_rendu(1.0, limite_echelle(true, 1536.0 * 960.0, false), false),
            1.0,
            "écran externe DPR 1 : ×1"
        );
        // mêmes positions en points à ×3 et à ×2 (seule la finesse change)
        let e = argos_render::ecrans::Etat::default();
        argos_render::px::set_scale(3.0);
        let a = argos_render::ecrans::zones(390.0, 844.0, &e);
        argos_render::px::set_scale(2.0);
        let b = argos_render::ecrans::zones(390.0, 844.0, &e);
        argos_render::px::set_scale(1.0);
        assert_eq!(format!("{a:?}"), format!("{b:?}"));
    }

    /// 🔍 (fluidité, lot A — garde-fou de netteté ; Jee, 24 Nov: « 4k ultra ») At rest the rendering scale is the
    /// screen's own density — iPhone ×3 (×2.4 with the app's zoom 0.8), large phones, tablets ×2 up to the iPad Pro
    /// 12.9", Jee's PC ×1.25, a Mac ×2 — even after a long run of very slow frames (the cell stayed at ×1 on its ×3
    /// screen, blurred). Never ×1 on a ×2 or ×3 screen at rest. While a finger moves on the phone or the tablet: an
    /// exact fraction of that density, about a million pixels (until the drag is done by the graphics card, lot B);
    /// the PC as at rest.
    #[test]
    fn sharp_at_rest_even_after_slow_frames() {
        for _ in 0..500 {
            frame_done(400.0);
        }
        // (desktop, density, width and height in CSS pixels, zoom, scale while a finger moves, name)
        let ecrans = [
            (false, 3.0, 390.0, 844.0, 1.0, 1.5, "iPhone 13"),
            (false, 3.0, 390.0, 844.0, 0.8, 1.2, "iPhone 13, zoom 0,8"),
            (false, 3.0, 430.0, 932.0, 1.0, 1.5, "iPhone Pro Max"),
            (false, 2.625, 412.0, 915.0, 1.0, 1.3125, "Android"),
            (false, 2.0, 820.0, 1180.0, 1.0, 1.0, "iPad"),
            (false, 2.0, 1024.0, 1366.0, 1.0, 1.0, "iPad Pro 12,9"),
            (true, 1.25, 1536.0, 960.0, 1.0, 1.25, "PC de Jee"),
            (true, 2.0, 1440.0, 900.0, 1.0, 2.0, "Mac ×2"),
        ];
        for (desktop, dpr, w, h, zoom, geste, nom) in ecrans {
            let want: f64 = dpr * zoom;
            let area = w * h / (zoom * zoom);
            assert_eq!(echelle_rendu(want, limite_echelle(desktop, area, false), false), want, "{nom} : au repos");
            let g = echelle_rendu(want, limite_echelle(desktop, area, true), true);
            assert!((g - geste).abs() < 1e-9, "{nom} : pendant un geste, ×{g}");
            assert!(g >= 1.0 && (want / g).fract().abs() < 1e-9 || desktop, "{nom} : une fraction exacte ({g})");
        }
    }

    /// 📤 Envoi direct : les octets RGBA sont ceux que softbuffer aurait posés (rouge, vert, bleu, opaque), au pixel près.
    #[test]
    fn octets_rgba_comme_softbuffer() {
        let px = [0x0012_3456u32, 0x00ff_0080, 0, 0x00ff_ffff, 0xff00_0001];
        let mut o = vec![9u8; 3];
        rgba_de(&px, &mut o);
        let attendu: Vec<u8> = px.iter().flat_map(|&p| [(p >> 16) as u8, (p >> 8) as u8, p as u8, 255]).collect();
        assert_eq!(o, attendu);
        assert_eq!(&o[..4], &[0x12, 0x34, 0x56, 255]);
        rgba_de(&px[..1], &mut o);
        assert_eq!(o.len(), 4, "le tampon gardé est vidé à chaque fois");
    }

    /// 📤 Seule la bande de rangées qui a changé est renvoyée ; rien si rien n'a changé ; tout à la première image.
    #[test]
    fn bande_de_rangees_changees() {
        let w = 4;
        let a = vec![0u32; w * 6];
        assert_eq!(bande_changee(&[], &a, w), Bande::Tout, "première image");
        assert_eq!(bande_changee(&a, &a, w), Bande::Rien, "rien n'a changé : pas d'envoi");
        let mut b = a.clone();
        b[w * 2 + 1] = 7; // rangée 2
        b[w * 4 + 3] = 9; // rangée 4
        assert_eq!(bande_changee(&a, &b, w), Bande::Rangees(2, 4));
        let mut c = a.clone();
        c[0] = 1;
        assert_eq!(bande_changee(&a, &c, w), Bande::Rangees(0, 0), "seulement la première rangée");
        let mut d = a.clone();
        d[w * 6 - 1] = 1;
        assert_eq!(bande_changee(&a, &d, w), Bande::Rangees(5, 5), "seulement la dernière");
        assert_eq!(bande_changee(&a[..w * 5], &a, w), Bande::Tout, "autre taille");
    }

    #[test]
    fn image_transparente_ne_change_rien() {
        let pm = image(2, 2, tiny_skia::Color::TRANSPARENT);
        let mut buf = vec![0x445566u32; 4];
        blit(&mut buf, 2, 2, &pm, 0.0, 0.0);
        assert_eq!(buf, vec![0x445566; 4]);
    }

    #[test]
    fn image_a_moitie_transparente_melangee() {
        // tiny-skia garde les couleurs prémultipliées : rouge à 50 % = (128, 0, 0, 128)
        let pm = image(1, 1, tiny_skia::Color::from_rgba8(255, 0, 0, 128));
        let mut buf = vec![0x0000ffu32];
        blit(&mut buf, 1, 1, &pm, 0.0, 0.0);
        let (r, g, b) = (buf[0] >> 16 & 255, buf[0] >> 8 & 255, buf[0] & 255);
        assert!((127..=129).contains(&r), "{r}");
        assert_eq!(g, 0);
        assert!((126..=128).contains(&b), "{b}");
    }
}
