//! 🧭 Menus et barres : barre du haut, rail, listes, langue, fuseau, fiche, dock, barre du cell.

use crate::*;

/// 🫧 Ligne « Style verre » du menu Thème.
const GLASS_ID: i64 = 1000;

/// ↶ ↷ Lignes Défaire / Refaire du menu des dessins sur le cell (loin des index des outils).
const UNDO_ID: i64 = 9000;
const REDO_ID: i64 = 9001;

/// ⏱ Id de « ➕ Ajouter un intervalle personnalisé… » (négatif : jamais un intervalle).
pub(crate) const ADD_INTERVAL: i64 = -7;

impl App {
    /// **Le seul endroit où les actions sont faites** (clavier, barre de navigation, barre du bas, menus).
    pub(crate) fn run_action(&mut self, a: ChartAction) {
        let cur = self.scale_now();
        let view = chart_actions::apply_view(a, &mut self.chart.viewport, &self.chart.slots);
        // zoom (boutons − +, Ctrl + ↑ / ↓) en mode libre : le prix se recadre sur les bougies maintenant visibles,
        // pour qu'elles reviennent au centre au lieu de sortir de l'écran
        if matches!(a, ChartAction::ZoomIn | ChartAction::ZoomOut) && !self.chart.settings.scale.auto() {
            self.start_free();
            return self.redraw();
        }
        let set = chart_actions::apply_settings(a, &mut self.chart.settings, self.chart.viewport.bar_spacing, cur);
        if !view && !set {
            match a {
                ChartAction::Reload => return self.reload(),
                ChartAction::WatchPrev | ChartAction::WatchNext => {
                    let dir = if a == ChartAction::WatchPrev { -1 } else { 1 };
                    if let Some(coin) = self.watch.step(dir) {
                        return self.open_coin(coin);
                    }
                }
                ChartAction::GoTo => {
                    if let Some(last) = self.chart.bars.last() {
                        self.ui.goto = Some(GoTo::open(last.time, now_s(), self.tz_offset_s()));
                        self.ui.goto_replay = false;
                    }
                }
                ChartAction::CopyDrawing => {
                    if let Some(i) = self.selected {
                        self.obj_action(i, ObjAction::Copy);
                    }
                }
                ChartAction::PasteDrawing => {
                    if let Some(d) = self.obj_clip.clone() {
                        self.add_copy(d);
                    }
                }
                ChartAction::NextStyle => self.switch_style(1),
                ChartAction::PrevStyle => self.switch_style(-1),
                ChartAction::Undo | ChartAction::Redo => {
                    // ↶ ↷ : un dessin en cours est d'abord abandonné ; puis l'historique des dessins de cette chart
                    if self.draft.take().is_none() {
                        self.track_drawings();
                        let now = self.chart.drawings.clone();
                        let mut guard = self.chart.hist.borrow_mut();
                        let h = &mut *guard;
                        let (from, to) = if a == ChartAction::Undo {
                            (&mut h.undo, &mut h.redo)
                        } else {
                            (&mut h.redo, &mut h.undo)
                        };
                        if let Some(prev) = from.pop() {
                            to.push(now);
                            h.base = Some(prev.clone());
                            drop(guard);
                            self.chart.drawings = prev;
                        }
                    }
                    self.selected = None;
                }
                ChartAction::CloseMenus => {
                    // Échap : quitte l'outil d'un coup (comme TradingView) : dessin en cours annulé, menus fermés, retour au curseur ;
                    // sans outil actif, il ferme les menus puis désélectionne l'objet
                    if self.session.words_box.is_some() {
                        self.words_click(WordsClick::Cancel);
                        return self.redraw();
                    }
                    // ✅ the confirmation box: cancelled; 🔔 an alert's label: it goes
                    if self.ui.confirmer.take().is_some() || self.ui.alerte_sel.take().is_some() {
                        return self.redraw();
                    }
                    let menus = self.ui.list.is_some()
                        || self.ui.pop.is_some()
                        || self.ui.sheet.is_some()
                        || self.ui.argos.is_some()
                        || self.ui.obj_tree.is_some()
                        || self.ui.split_menu.is_some()
                        || self.ui.context.open.is_some();
                    let drafting = self.draft.take().is_some();
                    if menus {
                        self.ui.context = ContextMenu::default();
                        (self.ui.list, self.ui.pop, self.ui.sheet, self.ui.argos, self.ui.obj_tree, self.ui.split_menu) = (None, None, None, None, None, None);
                    }
                    if self.tool != "cursor" {
                        self.set_tool("cursor");
                    } else if !menus && !drafting && self.selected.is_none() && self.ui.plein_ecran.is_some() {
                        return self.quitter_plein_ecran(); // ⛶ Échap : sortie du plein écran
                    } else if !menus && !drafting && self.selected.is_none() && self.watch.prefs.open && !self.top.phone {
                        // 🖥 rien d'autre à fermer : le volet de droite ouvert se ferme
                        self.watch.prefs.open = false;
                        self.save_watch();
                    } else if !menus && !drafting && self.ui.zen && self.selected.is_none() {
                        return self.toggle_zen(); // 🧘 Échap : sortie du mode Zen
                    } else if !menus && !drafting {
                        self.selected = None;
                    }
                    return self.redraw();
                }
                ChartAction::HLineAtMouse => {
                    if let Some(price) = self.price_at_mouse() {
                        self.chart.drawings.push(Drawing::hline(price));
                    }
                }
                ChartAction::HLine(price) => self.chart.drawings.push(Drawing::hline(price)),
                ChartAction::AddAlert(price) => self.add_price_alert(price),
                ChartAction::Brouillon(price, buy) => return self.brouillon_nouveau(price, buy),
                ChartAction::ObjectTree => return self.open_tree(),
                ChartAction::PleinEcran => return self.basculer_plein_ecran(),
                ChartAction::ClearDrawings => {
                    self.chart.drawings.clear();
                    self.selected = None;
                }
                ChartAction::CopyPrice(p) => {
                    let text = format_price(p, data_decimals(&self.chart.bars)).replace('\u{202f}', " ");
                    let ok = self.clipboard.as_mut().is_some_and(|c| c.set_text(text).is_ok());
                    if !ok {
                        eprintln!("⚠️ Presse-papiers indisponible");
                    }
                }
                _ => {}
            }
        }
        self.redraw();
    }

    /// Barre du haut : le bandeau suit la watchlist affichée, la fiche suit le symbole ouvert.
    pub(crate) fn refresh_top(&mut self) {
        self.top.tape = self
            .watch
            .sections
            .iter()
            .flat_map(|s| &s.rows)
            .map(|r| {
                let price: f64 = r.mark.parse().unwrap_or(0.0);
                let pct = (price > 0.0 && r.prev > 0.0).then(|| (price / r.prev - 1.0) * 100.0);
                Tick {
                    coin: r.coin.clone(),
                    label: r.label.split_whitespace().next().unwrap_or(&r.label).trim_end_matches("-USDC").to_string(),
                    price: r.mark.clone(),
                    pct,
                }
            })
            .collect();
        if let Source::Cap { series } = &self.chart.source {
            // 📈 série CoinMarketCap : historique gardé par Argos
            let n = self.chart.raw.len();
            (self.top.symbol, self.top.source, self.top.stats) = (format!("CRYPTOCAP:{series}"), format!("CoinMarketCap · {n} relevé(s) gardés"), vec![]);
            self.top.coin = self.watch.selected.clone();
            return;
        }
        if let Source::Ratio { exchange, base, quote } = &self.chart.source {
            // 🧮 paire calculée : marquée « calculée » dans l'en-tête
            (self.top.symbol, self.top.source, self.top.stats) = (
                argos_data::tv::ratio_name(base, quote),
                format!("{base} ÷ {quote} · {}", exchange.name()),
                vec![],
            );
            self.top.coin = self.watch.selected.clone();
            return;
        }
        let Source::Live { exchange, base } = &self.chart.source else {
            (self.top.symbol, self.top.source, self.top.stats) = (self.title.split(" · ").next().unwrap_or("").to_string(), "fichier".into(), vec![]);
            return;
        };
        // 🏷️ nom écrit comme TradingView : « BINANCE:BTCUSDT.P »
        self.top.symbol = argos_data::tv::name(*exchange, base);
        self.top.coin = self.watch.selected.clone();
        // (« Bitfinex Spot », « Bitfinex Perpétuel » : the kind said once)
        self.top.source = format!("{} {}", exchange.parent().name(), exchange.market_kind());
        let th = Theme::current();
        let stat = |label: &str, value: String, color: Option<argos_render::text::Rgb>| Stat {
            label: label.into(),
            value,
            color,
        };
        // ⏯ in a replay (the chart's or the Simulateur's): the price and the 24 h change of the bars played, not the
        // live ones (Suivi 9); volume, open interest and funding are today's: not shown
        if self.replay.is_some() {
            let b = &self.chart.bars;
            let dec = data_decimals(b);
            self.top.stats = b.last().map_or(vec![], |l| {
                let mut v = vec![stat("Prix", format_price(l.close, dec), None)];
                if let Some(a) = b.iter().rev().find(|a| a.time <= l.time - 86_400 && a.close > 0.0) {
                    let chg = (l.close / a.close - 1.0) * 100.0;
                    let col = Some(if chg >= 0.0 { th.bull } else { th.bear });
                    v.push(stat(
                        "Variation 24 h",
                        format!("{} ({})", format_price(l.close - a.close, dec), argos_core::nombres::format_pct(chg)),
                        col,
                    ));
                }
                v
            });
            return;
        }
        self.top.stats = match self.feeds.quotes.get(base.as_str()).filter(|_| *exchange == Exchange::Hyperliquid) {
            Some(q) => {
                let mark: f64 = q.mark.parse().unwrap_or(0.0);
                let dec = q.mark.split_once('.').map_or(0, |(_, d)| d.len());
                let chg = if q.prev > 0.0 { (mark / q.prev - 1.0) * 100.0 } else { 0.0 };
                let col = Some(if chg >= 0.0 { th.bull } else { th.bear });
                vec![
                    stat("Prix", format_price(mark, dec), None),
                    stat("Oracle", format_price(q.oracle, dec), None),
                    stat(
                        "Variation 24 h",
                        format!("{} ({})", format_price(mark - q.prev, dec), argos_core::nombres::format_pct(chg)),
                        col,
                    ),
                    stat("Volume 24 h", format!("{} $", topbar::big(q.volume)), None),
                    stat("Intérêt ouvert", format!("{} $", topbar::big(q.oi * mark)), None),
                    stat(
                        "Funding / compte à rebours",
                        topbar::funding_text(q.funding, now_s()),
                        Some(if q.funding >= 0.0 { th.bull } else { th.bear }),
                    ),
                ]
            }
            None => match self.feeds.ex_prices.get(exchange).and_then(|m| m.get(base.as_str())) {
                Some(m) => {
                    let dec = data_decimals(&self.chart.bars);
                    // 📈 « — » when the exchange gives no price of 24 h ago (Kraken), never a change made up
                    let (chg, col) = match m.change_pct() {
                        Some(pct) => (
                            format!("{} ({})", format_price(m.price - m.prev, dec), argos_core::nombres::format_pct(pct)),
                            Some(if pct >= 0.0 { th.bull } else { th.bear }),
                        ),
                        None => ("—".to_string(), None),
                    };
                    vec![stat("Prix", format_price(m.price, dec), None), stat("Variation 24 h", chg, col)]
                }
                None => {
                    let last = self.chart.bars.last().map(|b| b.close);
                    vec![stat(
                        "Dernier",
                        last.map_or("—".into(), |p| format_price(p, data_decimals(&self.chart.bars))),
                        None,
                    )]
                }
            },
        };
        // ⚡ a Bitfinex perpetual: its mark, open interest and funding with the time left before the next payment
        if let Some(s) = self.bitfinex_statut_de(base).filter(|_| *exchange == Exchange::Bitfinex) {
            let dec = data_decimals(&self.chart.bars);
            let (actif, _) = argos_data::paire::parts(base);
            if s.mark.is_finite() && s.mark > 0.0 {
                self.top.stats.push(stat("Mark", format_price(s.mark, dec), None));
            }
            if s.open_interest.is_finite() {
                self.top.stats.push(stat(
                    "Intérêt ouvert",
                    format!("{} {}", topbar::big(s.open_interest), actif.to_uppercase()),
                    None,
                ));
            }
            if s.funding.is_finite() {
                self.top.stats.push(stat(
                    "Funding / compte à rebours",
                    argos_data::exchanges::bitfinex::funding_texte(&s, now_s() * 1000),
                    Some(if s.funding >= 0.0 { th.bull } else { th.bear }),
                ));
            }
        }
        // ₿ a pair against BTC: its prices are in BTC, said where the price is written
        if argos_data::paire::contre_btc(base) {
            for s in self.top.stats.iter_mut().filter(|s| s.label == "Mark") {
                s.value.push_str(" BTC");
            }
            for s in self.top.stats.iter_mut().filter(|s| matches!(s.label.as_str(), "Prix" | "Dernier")) {
                if s.value != "—" {
                    s.value.push_str(" BTC");
                }
            }
        }
    }

    /// Menu Fuseau horaire (comme v1) : les fuseaux, puis les sessions et leurs heures dans ce fuseau (ouvertes en blanc).
    pub(crate) fn timezone_menu(&self) -> Vec<MenuGroup> {
        use argos_render::sessions::{open_now, zone_name, SESSIONS, ZONES};
        let zones = ZONES
            .iter()
            .enumerate()
            .map(|(i, (name, tz))| MenuItem {
                id: i as i64,
                label: name.to_string(),
                enabled: true,
                checked: self.fuseau() == *tz,
                ..Default::default()
            })
            .collect();
        let (now, off) = (now_s(), self.tz_offset_s());
        let open = open_now(now);
        let mut sessions: Vec<MenuItem> = SESSIONS
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let (a, b) = s.hours_at(off, now);
                MenuItem {
                    id: ID_SESSION + i as i64,
                    label: s.name.into(),
                    enabled: open.contains(&i),
                    color: Some(s.color),
                    hint: format!("{a} → {b}"),
                    ..Default::default()
                }
            })
            .collect();
        sessions.push(MenuItem {
            id: ID_SESSION + 99,
            label: "Plus de volume : Londres + NY".into(),
            ..Default::default()
        });
        let zone = zone_name(self.fuseau()).to_uppercase();
        vec![
            MenuGroup {
                title: "FUSEAU HORAIRE".into(),
                items: zones,
                strip: false,
            },
            MenuGroup {
                title: format!("SESSIONS (HEURE {zone})"),
                items: sessions,
                strip: false,
            },
        ]
    }

    /// 🌐 Packs de langue téléchargés (index dans `i18n::CATALOG`).
    pub(crate) fn lang_installed() -> Vec<usize> {
        (0..argos_render::i18n::CATALOG.len())
            .filter(|&k| platform::read(&format!("lang/{}.json", argos_render::i18n::CATALOG[k].0)).is_some())
            .collect()
    }

    /// Active le pack téléchargé de la langue `k` (lu du dossier de config) ; `false` s'il n'y est pas ou est illisible.
    pub(crate) fn load_lang_pack(k: usize) -> bool {
        let Some(code) = argos_render::i18n::CATALOG.get(k).map(|l| l.0) else {
            return false;
        };
        let pack = platform::read(&format!("lang/{code}.json")).and_then(|raw| serde_json::from_str::<std::collections::HashMap<String, String>>(&raw).ok());
        match pack {
            Some(p) => {
                argos_render::i18n::set_pack(k, p);
                true
            }
            None => false,
        }
    }

    /// 🌐 Langue `k` choisie dans le menu : déjà là = tout de suite ; pas encore téléchargée = on demande d'abord.
    pub(crate) fn ask_lang(&mut self, k: usize, anchor: Rect) {
        use argos_render::i18n;
        if i18n::done(k) || Self::lang_installed().contains(&k) {
            self.pick_lang(k);
            return self.redraw();
        }
        let name = i18n::CATALOG.get(k).map_or("", |l| l.1);
        let items = vec![
            MenuItem {
                id: 1,
                label: format!("Oui · {name}"),
                icon: Some("⬇️"),
                enabled: true,
                ..Default::default()
            },
            MenuItem {
                id: 0,
                label: "Annuler".into(),
                enabled: true,
                ..Default::default()
            },
        ];
        let groups = vec![MenuGroup {
            title: "TÉLÉCHARGER CETTE LANGUE ?".into(),
            items,
            strip: false,
        }];
        let (ww, wh) = (self.win_w(), self.area.1 as f64);
        self.ui.list = Some((ListKind::LangConfirm(k), ListMenu::new(groups, vec![], anchor, ww, wh)));
        self.redraw();
    }

    /// 🌐 Choisir la langue `k` : faite dans le code ou déjà téléchargée = tout de suite ; sinon on la télécharge.
    pub(crate) fn pick_lang(&mut self, k: usize) {
        use argos_render::i18n;
        let Some((code, name, _)) = i18n::CATALOG.get(k).copied() else { return };
        let installed = i18n::done(k) || Self::load_lang_pack(k);
        // pack pas fait dans le code : on va chercher la dernière version (nouvelles phrases) ; déjà installé = en silence
        if !i18n::done(k) {
            if let Some(proxy) = self.proxy.clone() {
                argos_data::rt::spawn(move || async move {
                    let res = argos_data::langpack::fetch(code).await.map_err(|e| e.to_string());
                    let _ = proxy.send_event(UserEvent::LangPack(k, res, installed));
                });
            }
        }
        if !installed {
            self.banner = Some(format!("⬇️ Téléchargement : {name}…"));
            return;
        }
        self.top.prefs.language = k;
        i18n::set_lang(k);
        self.save_top();
    }

    /// Menu des intervalles : Secondes → Jours, ⭐ favoris, l'intervalle affiché surligné.
    pub(crate) fn interval_menu(&self) -> Vec<MenuGroup> {
        let item = |secs: i64| MenuItem {
            id: secs,
            label: Interval(secs).label(),
            enabled: self.interval_ok(Interval(secs)),
            star: Some(self.tools.favorites.contains(&secs)),
            checked: secs == self.chart.interval.secs(),
            ..Default::default()
        };
        // ⏱ tes intervalles personnalisés, et la ligne pour en ajouter un
        let mut perso: Vec<MenuItem> = self.tools.custom.iter().map(|&s| item(s)).collect();
        perso.push(MenuItem {
            id: ADD_INTERVAL,
            label: "Ajouter un intervalle personnalisé…".into(),
            icon: Some("➕"),
            enabled: true,
            ..Default::default()
        });
        let perso = MenuGroup {
            title: "PERSONNALISÉS".into(),
            items: perso,
            strip: false,
        };
        SECTIONS
            .iter()
            .map(|(title, list)| MenuGroup {
                title: title.to_string(),
                items: list
                    .iter()
                    .map(|&secs| MenuItem {
                        id: secs,
                        label: Interval(secs).label(),
                        enabled: self.interval_ok(Interval(secs)),
                        star: Some(self.tools.favorites.contains(&secs)),
                        checked: secs == self.chart.interval.secs(),
                        ..Default::default()
                    })
                    .collect(),
                strip: false,
            })
            .chain(std::iter::once(perso))
            .collect()
    }

    /// Ouvre un menu en liste collé au bouton `anchor` (coordonnées de la fenêtre).
    pub(crate) fn open_list(&mut self, kind: ListKind, anchor: Rect) {
        self.ui.pop = None; // un menu à la fois
        if self.ui.list.as_ref().is_some_and(|(k, _)| *k == kind) {
            self.ui.list = None; // 2e clic sur le même bouton : le menu se ferme
            return self.redraw();
        }
        if matches!(kind, ListKind::RailGroup(_) | ListKind::RailButton(_)) {
            return self.open_rail_list(kind, anchor);
        }
        let (groups, collapsed) = match kind {
            ListKind::Intervals | ListKind::PaneIntervals(_) => (self.interval_menu(), self.tools.menu_collapsed.clone()),
            ListKind::Styles => (
                vec![MenuGroup {
                    title: "TYPE DE GRAPHIQUE".into(),
                    items: self
                        .chart
                        .styles
                        .iter()
                        .enumerate()
                        .map(|(i, st)| MenuItem {
                            id: i as i64,
                            label: st.name().to_string(),
                            enabled: true,
                            checked: i == self.chart.style,
                            ..Default::default()
                        })
                        .collect(),
                    strip: false,
                }],
                vec![],
            ),
            ListKind::Lists => (self.lists_menu(), vec![]),
            ListKind::TousBourse | ListKind::TousTri => (self.tous_menu(kind), vec![]),
            ListKind::Strategies => (self.strategies_menu(), vec![]),
            ListKind::Carnet(m) => (self.carnet_menu(m), vec![]),
            ListKind::Section => (self.section_menu(), vec![]),
            ListKind::IndPlus => (self.ind_plus_menu(), vec![]),
            ListKind::Alerts => (self.alerts_menu(), vec![]),
            ListKind::Accounts => (self.accounts_menu(), vec![]),
            ListKind::Movers => (self.movers_menu(), vec![]),
            ListKind::Backtest => (self.backtest_menu(), vec![]),
            ListKind::AiMemory => (self.ai_memory_menu(), vec![]),
            ListKind::Wallet => (self.wallet_menu(), vec![]),
            ListKind::IconBar(t) => (self.icon_bar_menu(t), vec![]),
            ListKind::Community => (self.community_menu(), vec![]),
            ListKind::DockAutres => (Self::dock_autres_menu(), vec![]),
            ListKind::Accueil => (self.accueil_menu(), vec![]),
            ListKind::Modules => (self.modules_menu(), vec![]),
            ListKind::Timezone => (self.timezone_menu(), vec![]),
            ListKind::Theme => {
                let items = Theme::NAMES
                    .iter()
                    .enumerate()
                    .map(|(k, n)| MenuItem {
                        id: k as i64,
                        label: n.to_string(),
                        enabled: true,
                        checked: k == self.top.prefs.theme,
                        ..Default::default()
                    })
                    .collect();
                (
                    vec![
                        MenuGroup {
                            title: "THÈME".into(),
                            items,
                            strip: false,
                        },
                        // 🫧 par-dessus n'importe quel thème
                        MenuGroup {
                            title: "EFFET".into(),
                            items: vec![MenuItem {
                                id: GLASS_ID,
                                label: "🫧 Style verre".into(),
                                enabled: true,
                                checked: self.top.prefs.glass,
                                ..Default::default()
                            }],
                            strip: false,
                        },
                    ],
                    vec![],
                )
            }
            ListKind::Drawing(i) => match self.chart.drawings.get(i) {
                Some(d) => (objmenu::groups(d), vec![]),
                None => return,
            },
            ListKind::AlertLine(id) => match self.menu_ligne_alerte(id) {
                Some(g) => (g, vec![]),
                None => return,
            },
            ListKind::SonAlerte(id) => match self.menu_son_alerte(id) {
                Some(g) => (g, vec![]),
                None => return,
            },
            ListKind::SonAlertes => (self.menu_son_alertes(), vec![]),
            ListKind::ReglagesAlertes => (self.menu_reglages_alertes(), vec![]),
            ListKind::DrawBar(i, b) => match self.chart.drawings.get(i) {
                Some(d) => {
                    let folded = self.drawbar().map_or(vec![], |(_, bar)| bar.folded);
                    (argos_render::drawbar::menu(b, d, &self.template_names(&d.tool), &folded), vec![])
                }
                None => return,
            },
            ListKind::ToolMenu(t) => (self.tool_menu(t), vec![]),
            ListKind::Top(TopMenu::Layouts) => (topmenu::layouts_menu(&self.liste_layouts(), self.top.prefs.auto_save), vec![]),
            ListKind::Top(_) => (
                topmenu::lang_menu(self.top.prefs.number_format, self.top.prefs.language, &Self::lang_installed()),
                vec![],
            ),
            _ => return,
        };
        let (ww, wh) = (self.win_w(), self.area.1 as f64);
        // 👆 the menus only a touch opens (⋯ of the tablet's dock, « Accueil »): rows of the finger's touch minimum
        let ligne = if matches!(kind, ListKind::DockAutres | ListKind::Accueil) {
            argos_render::formfactor::TOUCH_MIN_PT
        } else {
            argos_render::listmenu::ROW
        };
        let mut menu = ListMenu::new_lignes(groups, collapsed, anchor, ww, wh, ligne);
        // 🩶 an alert line's menu: grey icons, like the drawing bar's
        menu.gris = matches!(kind, ListKind::AlertLine(_));
        self.ui.list = Some((kind, menu));
        self.redraw();
    }

    /// Clic dans le menu en liste ouvert.
    /// 🚀 Menu Top gagnants / perdants sur 24 h : les perps Hyperliquid d'au moins 1 M$ de volume, 10 de chaque côté.
    pub(crate) fn movers_menu(&mut self) -> Vec<MenuGroup> {
        // 📊 Binance + OKX + Hyperliquid, dédoublonnés : le plus liquide de chaque symbole (les autres en petit)
        self.refresh_tickers();
        let best = argos_data::tickers::best_by_base(&self.all_tickers());
        let rows: Vec<(String, f64, f64)> = best
            .iter()
            .enumerate()
            .filter_map(|(k, (t, _))| Some((k.to_string(), t.pct()?, t.vol_usd)))
            .collect();
        let (up, down) = argos_render::panels::movers(&rows, 10, 1e6);
        let at = |k: &String| k.parse::<usize>().ok().and_then(|k| best.get(k));
        self.ui.movers = up.iter().chain(&down).filter_map(|(k, _)| at(k)).map(|(t, _)| (t.ex, t.base.clone())).collect();
        let label = |k: &String| {
            at(k).map_or(String::new(), |(t, others)| {
                let also: Vec<String> = others.iter().map(|e| e.name().to_string()).collect();
                argos_render::dock::venue_label(&t.base, t.ex.name(), &also)
            })
        };
        let up: Vec<(String, f64)> = up.iter().map(|(k, p)| (label(k), *p)).collect();
        let down: Vec<(String, f64)> = down.iter().map(|(k, p)| (label(k), *p)).collect();
        let th = Theme::current();
        let item = |k: usize, (c, p): &(String, f64)| MenuItem {
            id: k as i64,
            label: c.clone(),
            enabled: true,
            hint: format!("{p:+.2} %"),
            color: Some(if *p >= 0.0 { th.bull } else { th.bear }),
            ..Default::default()
        };
        let empty = |t: &str| MenuItem {
            id: -1,
            label: t.into(),
            enabled: false,
            ..Default::default()
        };
        let mut g_up: Vec<MenuItem> = up.iter().enumerate().map(|(k, x)| item(k, x)).collect();
        let mut g_down: Vec<MenuItem> = down.iter().enumerate().map(|(k, x)| item(up.len() + k, x)).collect();
        if g_up.is_empty() {
            g_up.push(empty("En attente des prix (Binance, OKX, Hyperliquid)…"));
        }
        if g_down.is_empty() {
            g_down.push(empty("—"));
        }
        vec![
            MenuGroup {
                title: "🚀 TOP GAGNANTS 24 H".into(),
                items: g_up,
                strip: false,
            },
            MenuGroup {
                title: "📉 TOP PERDANTS 24 H".into(),
                items: g_down,
                strip: false,
            },
        ]
    }

    pub(crate) fn list_click(&mut self, x: f64, y: f64) {
        let Some((kind, mut menu)) = self.ui.list.take() else { return };
        match menu.click(x, y) {
            MenuClick::Outside => self.ui.delete_armed = false,
            MenuClick::Remove(id) => {
                if let ListKind::DrawBar(i, b) = kind {
                    return self.drawbar_remove(i, b, id, menu.anchor);
                }
            }
            // 🖥 ⭐ / ☆ d'un outil de la colonne (l'étoile ou la ligne) : le menu reste ouvert avec ses nouvelles étoiles
            MenuClick::Star(id) | MenuClick::Pick(id) if matches!(kind, ListKind::IconBar(_)) && self.watch.col_outils.is_some() && id >= 300 => {
                self.icon_bar_pick(id);
                menu.groups = self.icon_bar_menu(None);
                self.ui.list = Some((kind, menu));
                return self.redraw();
            }
            MenuClick::Inside => {
                if kind == ListKind::Intervals {
                    self.tools.menu_collapsed = menu.collapsed.clone();
                    self.save_tools();
                } else if matches!(kind, ListKind::RailGroup(_)) {
                    self.rail.collapsed = menu.collapsed.clone();
                    self.save_rail();
                }
                self.ui.list = Some((kind, menu));
            }
            MenuClick::Star(id) if kind == ListKind::Curseurs && id == Self::ID_GOMME => {
                let on = self.rail.toggle_fav("erase");
                self.save_rail();
                menu.set_star(id, on);
                self.ui.list = Some((kind, menu));
            }
            MenuClick::Star(id) if matches!(kind, ListKind::RailGroup(_)) => {
                if let Some(t) = argos_render::tools::TOOLS.get(id as usize) {
                    let on = self.rail.toggle_fav(t.id);
                    self.save_rail();
                    menu.set_star(id, on);
                }
                self.ui.list = Some((kind, menu));
            }
            MenuClick::Star(id) => {
                let on = self.tools.toggle_favorite(id);
                self.save_tools();
                menu.set_star(id, on);
                self.ui.list = Some((kind, menu));
            }
            MenuClick::Pick(id) if kind == ListKind::Accounts => {
                let anchor = menu.anchor;
                if self.accounts_pick(id, anchor) {
                    let (ww, wh) = (self.win_w(), self.area.1 as f64);
                    self.ui.list = Some((kind, ListMenu::new(self.accounts_menu(), vec![], anchor, ww, wh)));
                }
                return self.redraw();
            }
            MenuClick::Pick(id) if kind == ListKind::Community => {
                let anchor = menu.anchor;
                if self.community_pick(id, anchor) {
                    let (ww, wh) = (self.win_w(), self.area.1 as f64);
                    self.ui.list = Some((kind, ListMenu::new(self.community_menu(), vec![], anchor, ww, wh)));
                }
                return self.redraw();
            }
            MenuClick::Pick(id) if kind == ListKind::Modules => {
                let anchor = menu.anchor;
                if self.modules_pick(id) {
                    let (ww, wh) = (self.win_w(), self.area.1 as f64);
                    self.ui.list = Some((kind, ListMenu::new(self.modules_menu(), vec![], anchor, ww, wh)));
                }
                return self.redraw();
            }
            MenuClick::Pick(id) if matches!(kind, ListKind::TuileLayout(_) | ListKind::SupprimerLayout(_)) => {
                return self.tuile_pick(kind, id, menu.anchor);
            }
            MenuClick::Pick(id) if matches!(kind, ListKind::IconBar(_)) => {
                self.icon_bar_pick(id);
                return self.redraw();
            }
            MenuClick::Pick(id) if kind == ListKind::Wallet => {
                let anchor = menu.anchor;
                if self.wallet_pick(id, anchor) {
                    let (ww, wh) = (self.win_w(), self.area.1 as f64);
                    self.ui.list = Some((kind, ListMenu::new(self.wallet_menu(), vec![], anchor, ww, wh)));
                }
                return self.redraw();
            }
            MenuClick::Pick(id) if kind == ListKind::AiMemory => {
                let anchor = menu.anchor;
                if self.ai_memory_pick(id, anchor) {
                    let (ww, wh) = (self.win_w(), self.area.1 as f64);
                    self.ui.list = Some((kind, ListMenu::new(self.ai_memory_menu(), vec![], anchor, ww, wh)));
                }
                return self.redraw();
            }
            MenuClick::Pick(id) if kind == ListKind::Backtest => {
                // un réglage : le menu reste ouvert, refait ; « Lancer » : il se ferme
                if self.backtest_pick(id) {
                    let anchor = menu.anchor;
                    let (ww, wh) = (self.win_w(), self.area.1 as f64);
                    self.ui.list = Some((kind, ListMenu::new(self.backtest_menu(), vec![], anchor, ww, wh)));
                }
                return self.redraw();
            }
            MenuClick::Pick(id) if kind == ListKind::SonAlertes => {
                // 🔊 a sound, ▶, the volume: the menu stays open, made again
                self.son_alertes_choix(id);
                let anchor = menu.anchor;
                let (ww, wh) = (self.win_w(), self.area.1 as f64);
                self.ui.list = Some((kind, ListMenu::new(self.menu_son_alertes(), vec![], anchor, ww, wh)));
                return self.redraw();
            }
            MenuClick::Pick(id) if kind == ListKind::Alerts => {
                // options, pause, suppression : le menu reste ouvert, refait avec les changements
                if self.alerts_pick(id) {
                    let anchor = menu.anchor;
                    let (ww, wh) = (self.win_w(), self.area.1 as f64);
                    let mut m = ListMenu::new(self.alerts_menu(), vec![], anchor, ww, wh);
                    m.scroll = menu.scroll;
                    self.ui.list = Some((kind, m));
                }
                return self.redraw();
            }
            MenuClick::Pick(id) => match kind {
                ListKind::Intervals | ListKind::PaneIntervals(_) if id == ADD_INTERVAL => {
                    self.ui.ut_case = match kind {
                        ListKind::PaneIntervals(k) => Some(k),
                        _ => None,
                    };
                    let ww = self.win_w();
                    self.ui.name_box = Some(NameBox::new(NameFor::Interval, "", menu.anchor, ww));
                    return self.redraw();
                }
                ListKind::Intervals => return self.set_interval(Interval(id)),
                // ⏱ menu ouvert pour une case du split : seulement cette case
                ListKind::PaneIntervals(k) => return self.pane_set_interval(k, Interval(id)),
                ListKind::Styles => {
                    let step = id as isize - self.chart.style as isize;
                    self.switch_style(step);
                }
                ListKind::RailGroup(_) if id == UNDO_ID || id == REDO_ID => {
                    return self.run_action(if id == UNDO_ID { ChartAction::Undo } else { ChartAction::Redo });
                }
                ListKind::Curseurs => return self.curseurs_choix(id),
                ListKind::RailGroup(g) => {
                    if let Some(t) = argos_render::tools::TOOLS.get(id as usize) {
                        self.rail.set_last(g, t.id);
                        self.save_rail();
                        self.set_tool(t.id);
                    }
                }
                ListKind::RailButton(_) => {
                    if let Some(a) = rail::MenuAction::from_id(id) {
                        self.rail_action(a);
                    }
                }
                ListKind::Lists => return self.lists_pick(id, menu.anchor),
                // ⌂ 📱 the two choices of « Accueil » (Réglages by the finger)
                ListKind::Accueil => {
                    use argos_render::sheet::Act;
                    let a = if id == 0 { Act::AccueilLancement } else { Act::RevoirAccueil };
                    return self.reglages_cible(argos_render::reglages::Cible::Act(a), menu.anchor);
                }
                // 📟 ⋯ of the tablet's dock: as if its button had been touched
                ListKind::DockAutres => {
                    if let Some(&(i, _, _)) = argos_render::ecrans::DOCK_AUTRES.get(id as usize) {
                        return self.phone_click(argos_render::phonebar::PhoneHit::Outil(i), menu.anchor);
                    }
                }
                ListKind::TousBourse | ListKind::TousTri => return self.tous_pick(id),
                ListKind::Strategies => return self.strategie_pick(id),
                ListKind::Carnet(m) => return self.carnet_pick(m, id),
                ListKind::Section => return self.section_pick(id, menu.anchor),
                ListKind::IndPlus => return self.ind_plus_pick(id),
                ListKind::Alerts => {
                    self.alerts_pick(id);
                }
                ListKind::Accounts => {}
                ListKind::Backtest
                | ListKind::AiMemory
                | ListKind::Wallet
                | ListKind::IconBar(_)
                | ListKind::Community
                | ListKind::Modules
                | ListKind::TuileLayout(_)
                | ListKind::SupprimerLayout(_) => {}
                ListKind::Movers => {
                    if let Some((ex, base)) = self.ui.movers.get(id as usize).cloned() {
                        return self.switch_symbol(ex, base);
                    }
                }
                ListKind::LangConfirm(k) => {
                    if id == 1 {
                        self.pick_lang(k);
                    }
                }
                ListKind::AddApi => {
                    if let Some(e) = usize::try_from(id).ok().and_then(|i| Exchange::ALL.get(i)) {
                        self.exchange_action(ExAction::Key(e.id().into()));
                    } else if id == Exchange::ALL.len() as i64 {
                        self.exchange_action(ExAction::Key("oanda".into()));
                    } else if id == Exchange::ALL.len() as i64 + 1 {
                        let (w, h) = (self.win_w(), self.area.1 as f64 + self.top_h());
                        self.session.key_box = Some(KeyBox::new("custom", "API personnalisée", w, h));
                    } else if id == Exchange::ALL.len() as i64 + 2 {
                        self.cmc_key_box();
                    }
                }
                ListKind::Drawing(i) => {
                    if let Some(a) = ObjAction::from_id(id) {
                        return self.obj_action(i, a);
                    }
                }
                ListKind::DrawBar(i, b) => return self.drawbar_pick(i, b, id),
                ListKind::AlertLine(a) => return self.ligne_alerte_choix(a, id, menu.anchor),
                ListKind::SonAlerte(a) => self.son_alerte_choix(a, id),
                ListKind::SonAlertes => {}
                ListKind::ReglagesAlertes => return self.reglages_alertes_choix(id, menu.anchor),
                ListKind::ToolMenu(t) => {
                    if let Some(a) = toolbar::MenuAction::from_id(id) {
                        return self.tool_menu_action(t, a);
                    }
                }
                ListKind::Top(_) => return self.top_act(id, menu.anchor),
                ListKind::Theme if id == GLASS_ID => {
                    self.top.prefs.glass = !self.top.prefs.glass;
                    self.ui.glass_off = false;
                    self.save_top();
                }
                ListKind::Theme => {
                    self.top.prefs.theme = id.max(0) as usize;
                    argos_render::set_theme(self.top.prefs.theme);
                    self.save_top();
                }
                ListKind::Timezone => {
                    if let Some((_, tz)) = usize::try_from(id).ok().and_then(|i| argos_render::sessions::ZONES.get(i)) {
                        self.chart.settings.timezone = tz.to_string();
                    }
                }
            },
        }
        self.redraw();
    }

    /// Clic sur un bouton de la barre de gauche (comme Argos v1).
    pub(crate) fn rail_click(&mut self, b: rail::Btn, r: Rect) {
        use rail::Btn;
        match b {
            // 🖱 1er bouton : la petite flèche du coin = le groupe du curseur (Croix, Point, Flèche, Gomme)
            Btn::Tool("cursor") if self.pointer.rail_mouse.is_some_and(|(x, y)| rail::on_arrow(&r, x, y)) => {
                return self.open_rail_list(ListKind::Curseurs, self.win(r, 0.0));
            }
            Btn::Tool(id) => self.set_tool(id),
            Btn::Erase => self.set_tool(if self.tool == "erase" { "cursor" } else { "erase" }),
            Btn::Group(g) => {
                // comme TradingView : clic sur l'icône = l'outil tout de suite ; clic sur la petite flèche du coin = la liste du groupe
                if self.pointer.rail_mouse.is_some_and(|(x, y)| rail::on_arrow(&r, x, y)) {
                    return self.open_list(ListKind::RailGroup(g), self.win(r, 0.0));
                }
                self.ui.list = None;
                self.set_tool(self.rail.current(g));
            }
            Btn::Magnet | Btn::Lock | Btn::Eye | Btn::Trash => return self.open_list(ListKind::RailButton(b), self.win(r, 0.0)),
            Btn::Keep => {
                self.rail.keep = !self.rail.keep;
                self.save_rail();
                self.banner = Some(
                    if self.rail.keep {
                        "✏️ Mode dessin continu : l'outil reste actif"
                    } else {
                        "✏️ L'outil se désactive après chaque dessin"
                    }
                    .into(),
                );
            }
            Btn::Undo => return self.run_action(ChartAction::Undo),
            Btn::Redo => return self.run_action(ChartAction::Redo),
        }
        self.redraw();
    }

    /// Ouvre un sous-menu de la barre de gauche, à droite du bouton.
    pub(crate) fn open_rail_list(&mut self, kind: ListKind, r: Rect) {
        if self.ui.list.as_ref().is_some_and(|(k, _)| *k == kind) {
            self.ui.list = None;
            return self.redraw();
        }
        let locked = !self.chart.drawings.is_empty() && self.chart.drawings.iter().all(|d| d.locked());
        let groups = match kind {
            // 📱 sur le cell (pas de barre de gauche) : ↶ ↷ en tête des outils de dessin
            ListKind::RailGroup(g) if self.top.phone => {
                let item = |id: i64, label: &str, ok: bool| argos_render::listmenu::MenuItem {
                    id,
                    label: label.into(),
                    enabled: ok,
                    ..Default::default()
                };
                let h = self.chart.hist.borrow();
                let mut v = vec![argos_render::listmenu::MenuGroup {
                    title: String::new(),
                    items: vec![
                        item(UNDO_ID, "↶ Défaire", !h.undo.is_empty() || self.draft.is_some()),
                        item(REDO_ID, "↷ Refaire", !h.redo.is_empty()),
                    ],
                    strip: false,
                }];
                drop(h);
                v.extend(rail::group_menu(&self.rail, g, self.tool));
                v
            }
            ListKind::RailGroup(g) => rail::group_menu(&self.rail, g, self.tool),
            ListKind::Curseurs => self.menu_curseurs(),
            ListKind::RailButton(b) => rail::button_menu(&self.rail, b, self.chart.drawings.len(), locked),
            _ => return,
        };
        let (ww, wh) = (self.win_w(), self.area.1 as f64);
        // depuis la barre de gauche : à droite du bouton ; depuis la barre flottante : dessous
        let mut menu = if r.x < self.rw() {
            ListMenu::beside(groups, self.rail.collapsed.clone(), r, ww, wh)
        } else {
            ListMenu::new(groups, self.rail.collapsed.clone(), r, ww, wh)
        };
        menu.gris = true; // 🩶 tool menus: grey, like the drawing bar (Jee, 17 Nov)
        self.ui.list = Some((kind, menu));
        self.redraw();
    }

    /// Ligne choisie dans le menu de l'aimant, du cadenas, de l'œil ou de la poubelle.
    pub(crate) fn rail_action(&mut self, a: rail::MenuAction) {
        use rail::MenuAction::*;
        rail::apply(&mut self.rail, a);
        match a {
            LockDraw | LockAll => {
                let lock = a == LockAll || !self.chart.drawings.iter().all(|d| d.locked());
                self.chart.drawings.iter_mut().for_each(|d| d.set_locked(lock));
            }
            DelDraw | DelAll => {
                self.chart.drawings.clear();
                (self.selected, self.draft) = (None, None);
            }
            // dessins cachés un par un (clic droit → Cacher) : Masquer / Afficher les fait revenir
            HideDraw | HideAll if self.chart.drawings.iter().any(|d| d.style.hidden) => {
                self.chart.drawings.iter_mut().for_each(|d| d.style.hidden = false);
                (self.rail.hide_draw, self.rail.hide_ind, self.rail.hide_pos) = (false, false, false);
            }
            _ => {}
        }
        self.save_rail();
        self.redraw();
    }

    pub(crate) fn save_rail(&self) {
        // 🖥 replié / déplié : à interface-2 (interface-2.json), l'ancienne /app/ garde le sien
        Self::write_prefs_garde("rail.json", &self.rail, &crate::actions::accueil::RAIL_A_ELLE);
    }

    /// ⌂ 📱 The two choices of the « Accueil » row (Réglages by the finger).
    fn accueil_menu(&self) -> Vec<MenuGroup> {
        let item = |id: i64, label: &str, checked: bool| MenuItem {
            id,
            label: label.into(),
            enabled: true,
            checked,
            ..Default::default()
        };
        vec![MenuGroup {
            title: "ACCUEIL".into(),
            items: vec![item(0, "Accueil au lancement", self.top.prefs.accueil), item(1, "Revoir l'accueil", false)],
            strip: false,
        }]
    }

    /// 📟 The small sheet of ⋯ (tablet's dock): 🐋 🧩 🎓 👥 🛒 with their names.
    fn dock_autres_menu() -> Vec<MenuGroup> {
        vec![MenuGroup {
            title: String::new(),
            items: argos_render::ecrans::DOCK_AUTRES
                .iter()
                .enumerate()
                .map(|(k, (_, em, nom))| MenuItem {
                    id: k as i64,
                    label: format!("{em}  {nom}"),
                    enabled: true,
                    ..Default::default()
                })
                .collect(),
            strip: false,
        }]
    }

    /// Action d'une fenêtre de la barre du bas (Panneaux, Hub d'analyse).
    /// ▦ 📱 La feuille Splits du bas (cell et tablette, maquette), par-dessus le reste.
    pub(crate) fn ouvrir_splits_feuille(&mut self) {
        let wh = self.top_h() + self.area.1 as f64 + argos_render::ecrans::dock_h();
        (self.ui.list, self.ui.sheet, self.ui.argos, self.ui.notifs) = (None, None, None, None);
        self.ui.split_menu = Some(argos_render::splits::Menu::feuille(self.win_w(), wh));
    }

    pub(crate) fn sheet_act(&mut self, a: Act) {
        self.ui.sheet = None;
        let panel = |me: &mut Self, t: wl::Tool| {
            (me.watch.prefs.open, me.watch.prefs.panel) = (true, t);
            me.save_watch();
        };
        match a {
            Act::Indicators => panel(self, wl::Tool::Indicators),
            // 🪟 Multi-chart : les Splits (📱 cell et tablette : la feuille du bas ; PC : le menu, sous la barre, à droite)
            Act::MultiChart => {
                if self.top.phone || self.ui.form.tablet() {
                    self.ouvrir_splits_feuille();
                } else {
                    let (ww, wh) = (self.win_w(), self.top_h() + self.area.1 as f64);
                    let ancre = Rect::new(ww - 340.0, 0.0, 40.0, self.top_h());
                    self.ui.split_menu = Some(argos_render::splits::Menu::new(ancre, ww, wh));
                }
            }
            Act::Demo => panel(self, wl::Tool::Trading),
            Act::Journal => panel(self, wl::Tool::Journal),
            Act::Exchanges => panel(self, wl::Tool::Exchanges),
            Act::ImportV1 => return self.import_v1(),
            Act::AccueilLancement => self.toggle_accueil_lancement(),
            Act::RevoirAccueil => return self.revoir_accueil(),
            Act::Raccourcis => return self.toggle_raccourcis(),
            Act::Agenda => {
                panel(self, wl::Tool::Agenda);
                self.refresh_eco();
            }
            Act::Eco => {
                panel(self, wl::Tool::News);
                self.refresh_eco();
            }
            Act::Whales => {
                panel(self, wl::Tool::Whales);
                self.refresh_eco();
            }
            Act::Modules => panel(self, wl::Tool::Modules),
            Act::Save => {
                self.save();
                self.banner = Some("💾 Sauvegardé : dessins, unité de temps et réglages de ce symbole".into());
            }
            Act::ChartType => {
                let w = self.win_w();
                return self.open_list(ListKind::Styles, Rect::new(w - 420.0, self.top_h() + 6.0, 170.0, 0.0));
            }
            Act::ChartSettings => return self.open_menu(MenuSource::PriceAxis, Anchor::Gear),
            // 🔊 📱 Réglages of the cell: the same list as the PC panel's row, at the top
            Act::SonAlertes => {
                let w = self.win_w();
                return self.open_list(ListKind::SonAlertes, Rect::new((w / 2.0 - 120.0).max(6.0), self.top_h() + 6.0, 240.0, 0.0));
            }
            Act::Theme => {
                let w = self.win_w();
                return self.open_list(ListKind::Theme, Rect::new((w / 2.0 - 110.0).max(6.0), self.top_h() + 6.0, 220.0, 0.0));
            }
            Act::Timeframe => {
                let anchor = Rect::new(self.chart.viewport.plot_w() / 2.0, 4.0, 10.0, 10.0);
                self.ui.list = None;
                self.ui.pop = Some(toolbar::Pop::new(
                    toolbar::PopKind::Tf,
                    anchor,
                    self.chart.viewport.plot_w(),
                    self.chart.viewport.plot_h(),
                ));
            }
            Act::OpenPanels => {
                // sous ⚙ de la barre du haut
                let w = self.win_w();
                self.ui.sheet = Some(Sheet::below(SheetKind::Panels, Rect::new(w - 40.0, 10.0, 30.0, 30.0), 48.0, w));
            }
            Act::ObjectTree => return self.open_tree(),
            Act::Replay => return self.replay_start(),
            Act::Backtest => {
                let (x, y) = self.pointer.win_mouse;
                return self.open_list(ListKind::Backtest, Rect::new(x - 120.0, y, 240.0, 1.0));
            }
            Act::Account => {
                let w = self.win_w();
                self.ui.sheet = Some(
                    Sheet::below(SheetKind::Account, Rect::new(w - 200.0, 10.0, 60.0, 30.0), 48.0, w)
                        .compte(&self.top.account, self.area.1 as f64 + self.top_h()),
                );
            }
            Act::TryDemo => panel(self, wl::Tool::Trading),
            Act::Passkey => return self.passkey_start(),
            Act::HaveKey => match &self.session.account {
                Some(a) => self.banner = Some(format!("✨ Tu es déjà connecté sur cet appareil : {}", a.id_full())),
                None => {
                    let (w, h) = (self.win_w(), self.area.1 as f64 + self.top_h());
                    self.session.key_box = Some(KeyBox::new("account-login", "Compte Argos", w, h));
                }
            },
            Act::CreateAccount => match &self.session.account {
                Some(a) => self.banner = Some(format!("✨ Tu as déjà un compte sur cet appareil : {}", a.id_full())),
                None => {
                    let (w, h) = (self.win_w(), self.area.1 as f64 + self.top_h());
                    self.session.key_box = Some(KeyBox::new("account-new", "Compte Argos", w, h));
                }
            },
            other => self.banner = Some(format!("{} : bientôt dans la version Rust (comme dans Argos v1)", other.face().1)),
        }
        self.redraw();
    }

    /// 🎡 Symboles voisins dans la watchlist : (avant, après) le symbole ouvert.
    pub(crate) fn voisins_symbole(&self) -> (Option<String>, Option<String>) {
        let o = self.watch.order();
        match o.iter().position(|c| *c == self.watch.selected) {
            Some(k) => (k.checked_sub(1).and_then(|j| o.get(j).cloned()), o.get(k + 1).cloned()),
            None => (None, o.first().cloned()),
        }
    }

    /// 🎡 Unités de temps favorites voisines : (la plus grande plus courte, la plus petite plus longue).
    pub(crate) fn voisins_intervalle(&self) -> (Option<i64>, Option<i64>) {
        let cur = self.chart.interval.secs();
        let f = &self.tools.favorites;
        (f.iter().copied().filter(|s| *s < cur).max(), f.iter().copied().filter(|s| *s > cur).min())
    }

    /// 🗂 La carte du compte du menu ARGOS (maquette tel-13) : compte connecté, compte démo, solde et P&L du jour,
    /// 🛡 Gardien, wallet.
    pub(crate) fn compte_carte(&self) -> topmenu::CompteCarte {
        let (_, pnl_c) = self.demo_pastille();
        let jour = self.sim_jour();
        let w = &self.feeds.wallet_addr;
        topmenu::CompteCarte {
            compte: self.session.account.as_ref().map(|a| a.id_full()),
            demo: self.demo.name.clone(),
            solde: argos_render::simulateur::argent(self.demo.balance, false),
            pnl: (argos_render::simulateur::argent(jour, true), pnl_c),
            gardien: self.gardien_pilule(),
            wallet: if w.is_empty() {
                "non connecté".into()
            } else {
                format!("{}…{}", &w[..w.len().min(6)], &w[w.len().saturating_sub(4)..])
            },
        }
    }

    /// 🧰 Bouton de la barre d'icônes de droite qui ouvre autre chose qu'un volet (`r` : le bouton, coordonnées de la
    /// fenêtre) : les mêmes actions que les icônes du bas du cell.
    pub(crate) fn outil_barre(&mut self, t: wl::Tool, r: Rect) {
        match t {
            wl::Tool::Dessin => return self.open_rail_list(ListKind::RailGroup("lines"), r),
            wl::Tool::Demo => return self.toggle_simulateur(),
            wl::Tool::Gardien => return self.ouvrir_propfirm(),
            wl::Tool::Script => self.open_editor(argos_render::panels::ScriptOpen::Mine),
            wl::Tool::Academie => return self.open_academy(),
            wl::Tool::Communaute => return self.open_list(ListKind::Community, r),
            wl::Tool::Portfolio => self.ui.pf_page = Some(argos_render::portfolio::PfPage::new()),
            wl::Tool::Liquidations => self.toggle_indicator(argos_render::indicators::Ind::LiqMap),
            wl::Tool::Marketplace => return self.open_market(),
            wl::Tool::Reglages => {
                let w = self.win_w();
                self.ui.sheet = Some(Sheet::below(SheetKind::Panels, r, r.y, w));
            }
            _ => {}
        }
        self.redraw();
    }

    /// 📐 Un toucher sur l'écran principal du cell (zones mesurées dans la maquette ; `r` : la zone, fenêtre).
    pub(crate) fn cell_click(&mut self, hit: argos_render::ecrans::CellHit, r: Rect) {
        use argos_render::ecrans::CellHit as C;
        use argos_render::phonebar::PhoneHit;
        use argos_render::topbar::TopClick;
        let sous = Rect::new(r.x, r.y, r.w, r.h);
        match hit {
            C::Accueil => self.top_click_do(TopClick::Home),
            C::Argos => self.top_click_do(TopClick::Menu(TopMenu::Argos, sous)),
            C::MiseEnPage => self.top_click_do(TopClick::Menu(TopMenu::Layouts, sous)),
            // 🔔 la feuille des notifications (maquette)
            C::Alertes => {
                // (the alerts panel as the bottom sheet, its handle on top)
                let mut n = argos_render::notifs::Notifs::default();
                n.alertes.feuille = true;
                self.ui.notifs = Some(n);
                self.redraw();
            }
            // ▦ la feuille Splits du bas (maquette) ; 2e appui : fermée
            C::Disposition => {
                if self.ui.split_menu.take().is_none() {
                    self.ouvrir_splits_feuille();
                }
                self.redraw();
            }
            C::Partager => self.top_click_do(TopClick::Partager),
            C::Recherche => self.top_click_do(TopClick::Recherche),
            // 🗺 le plan de l'app (choix de Jee, 15 nov.) : une page pleine, un bloc par endroit ; 2e toucher : fermée
            C::Plan => {
                (self.ui.list, self.ui.pop, self.ui.sheet, self.ui.argos) = (None, None, None, None);
                self.ui.plan = if self.ui.plan.is_some() {
                    None
                } else {
                    Some(argos_render::plan::Plan::default())
                };
                self.redraw();
            }
            C::Demo => self.demo_pastille_toucher(),
            C::Flux => {
                let e = self.cell_etat();
                self.banner = Some(format!("📡 {} · {}", e.flux.0, self.symbol_label()));
                self.redraw();
            }
            // 🔎 the market panel: the phone's bottom sheet, or under (above) the pill on the tablet
            C::Symbole => self.marches_ouvrir(r, self.pilule()),
            C::SymbolePrec => self.phone_click(PhoneHit::SymbolePrec, r),
            C::SymboleSuiv => self.phone_click(PhoneHit::SymboleSuiv, r),
            C::Unite => self.phone_click(PhoneHit::Interval, r),
            C::UnitePrec => self.phone_click(PhoneHit::IntervallePrec, r),
            C::UniteSuiv => self.phone_click(PhoneHit::IntervalleSuiv, r),
            C::Fx => self.phone_click(PhoneHit::Fx, r),
            C::Ai => self.toggle_ai(),
            C::Icone(i) => self.phone_click(PhoneHit::Outil(i), r),
            C::Autres => self.open_list(ListKind::DockAutres, r),
            // 📱 the single pill
            C::Dessins => self.pfeuille_ouvrir(crate::actions::pilule::PKind::Dessins),
            C::Trading => self.pfeuille_ouvrir(crate::actions::pilule::PKind::Trading),
            C::Plus => self.pfeuille_ouvrir(crate::actions::pilule::PKind::Argos),
            C::Replier => self.pilule_replier(true),
            C::Rouvrir => self.pilule_replier(false),
        }
    }

    /// 📱 Bouton de la barre du bas du cell (`r` : le bouton, coordonnées de la fenêtre).
    pub(crate) fn phone_click(&mut self, hit: PhoneHit, r: Rect) {
        use argos_render::phonebar::Icone;
        let w = self.win_w();
        // un volet : ouvert (ou fermé si c'est déjà lui)
        let volet = |me: &mut Self, t: wl::Tool| {
            let open = !(me.watch.prefs.open && me.watch.prefs.panel == t);
            (me.watch.prefs.open, me.watch.prefs.panel) = (open, t);
            me.save_watch();
        };
        match hit {
            // le symbole ouvre la watchlist par-dessus la chart ; un 2e toucher la ferme
            PhoneHit::Symbol => volet(self, wl::Tool::Watchlist),
            PhoneHit::Interval => return self.open_list(ListKind::Intervals, r),
            // ƒx : la fenêtre des indicateurs (la même que ƒx de la barre du haut sur PC)
            // 🅘 la page Indicateurs de la maquette (plein écran, en verre)
            PhoneHit::Fx => {
                (self.ui.list, self.ui.pop, self.ui.sheet) = (None, None, None);
                self.ui.fx_page = Some(argos_render::fxpage::FxPage::new());
            }
            PhoneHit::Demo => return self.demo_pastille_toucher(),
            // 🎡 molette : le symbole ou l'unité d'avant (en haut) ou d'après (en bas)
            PhoneHit::SymbolePrec | PhoneHit::SymboleSuiv => {
                let (avant, apres) = self.voisins_symbole();
                if let Some(c) = if hit == PhoneHit::SymbolePrec { avant } else { apres } {
                    return self.open_coin(c);
                }
                // pas de voisin (liste vide, bout de liste) : un toucher ne reste jamais sans effet, la watchlist s'ouvre
                volet(self, wl::Tool::Watchlist);
            }
            PhoneHit::IntervallePrec | PhoneHit::IntervalleSuiv => {
                let (avant, apres) = self.voisins_intervalle();
                if let Some(s) = if hit == PhoneHit::IntervallePrec { avant } else { apres } {
                    return self.set_interval(Interval(s));
                }
                return self.open_list(ListKind::Intervals, r);
            }
            PhoneHit::Outil(i) => match i {
                Icone::Trading => volet(self, wl::Tool::Trading),
                Icone::Demo => return self.toggle_simulateur(),
                Icone::PropFirm => return self.ouvrir_propfirm(),
                Icone::Script => self.open_editor(argos_render::panels::ScriptOpen::Mine),
                Icone::Baleines => {
                    volet(self, wl::Tool::Whales);
                    self.refresh_eco();
                }
                Icone::Modules => return self.open_list(ListKind::Modules, r),
                Icone::Academie => return self.open_academy(),
                // 📟 tablette : 👥 🌐 🛒 de la maquette (les mêmes actions que la barre d'icônes du PC)
                Icone::Communaute => return self.open_list(ListKind::Community, r),
                Icone::Macro => {
                    volet(self, wl::Tool::Agenda);
                    self.refresh_eco();
                }
                Icone::Marketplace => return self.open_market(),
                Icone::Reglages if argos_render::actions::tactile() => return self.reglages_doigt(),
                Icone::Reglages => self.ui.sheet = Some(Sheet::new(SheetKind::Panels, r, r.y, w)),
            },
        }
        self.redraw();
    }

    /// Ligne choisie dans un menu de la barre du haut (Argos ▾, Mises en page ▾, 🌐).
    pub(crate) fn top_act(&mut self, id: i64, anchor: Rect) {
        let panel = |me: &mut Self, t: wl::Tool| {
            (me.watch.prefs.open, me.watch.prefs.panel) = (true, t);
            me.save_watch();
        };
        // 🗂 une mise en page de la liste (ids négatifs)
        if let Some(k) = topmenu::layout_of(id) {
            return self.clic_layout(k);
        }
        // 🌐 🔎 Chercher une langue ; format des nombres (100 + k) ; langues (200 + k)
        if id == topmenu::LANG_SEARCH {
            let ww = self.win_w();
            self.ui.name_box = Some(NameBox::new(NameFor::Lang, "", anchor, ww));
            return self.redraw();
        }
        if (100..100 + argos_render::viewport::NUMBER_FORMATS.len() as i64).contains(&id) {
            self.top.prefs.number_format = (id - 100) as usize;
            argos_render::viewport::set_number_format(self.top.prefs.number_format);
            self.save_top();
            return self.redraw();
        }
        if id >= 200 {
            return self.ask_lang((id - 200) as usize, anchor);
        }
        let Some(a) = TopAct::from_id(id) else { return self.redraw() };
        match a {
            TopAct::Trade => self.watch.prefs.open = false,
            TopAct::Portfolio => self.ui.pf_page = Some(argos_render::portfolio::PfPage::new()),
            TopAct::Backtest => return self.sheet_act(Act::Backtest),
            TopAct::Academy => return self.open_academy(),
            TopAct::Community => return self.open_list(ListKind::Community, anchor),
            TopAct::Modules => return self.open_list(ListKind::Modules, anchor),
            TopAct::Wallet => return self.open_wallet(anchor),
            TopAct::DemoAccount => panel(self, wl::Tool::Trading),
            TopAct::Journal => panel(self, wl::Tool::Journal),
            TopAct::ApiKeys => {
                panel(self, wl::Tool::Exchanges);
                // 🔒 navigateur : aucune clé n'y va ; elles se gèrent dans l'app desktop (trousseau du système)
                if cfg!(target_arch = "wasm32") {
                    self.banner = Some("🔑 Les clés API se gèrent dans l'app desktop (trousseau du système) : aucune clé ne va dans le navigateur. Sans clé, tout marche avec les sources gratuites.".into());
                }
            }
            TopAct::Hub if argos_render::actions::tactile() => return self.reglages_doigt(),
            TopAct::Hub => return self.sheet_act(Act::OpenPanels),
            // 👤 Mon compte (avant : Connect dans la barre du haut) et 🌐 Langue (avant : 🌐) : leur fenêtre habituelle
            TopAct::MyAccount => {
                let w = self.top_w();
                self.ui.sheet =
                    Some(Sheet::below(SheetKind::Account, anchor, anchor.y + anchor.h, w).compte(&self.top.account, self.area.1 as f64 + self.top_h()));
            }
            TopAct::Langue => return self.open_list(ListKind::Top(TopMenu::Lang), anchor),
            TopAct::Accueil => return self.open_accueil(),
            // menu ARGOS de la maquette : 🌍 Marchés (le volet du marché), 🛒 Marketplace, 🛡 Gardien (le Simulateur)
            TopAct::Marches => panel(self, wl::Tool::Market),
            // 🧭 📶 the Hyperliquid modules of « Marchés »
            TopAct::Explorer => panel(self, wl::Tool::Explorer),
            TopAct::Stats => panel(self, wl::Tool::Stats),
            TopAct::Marketplace => return self.open_market(),
            // ⌨ raccourcis du clavier ; 📰 Nouvelles et 📅 Agenda : les pages Actualités et Calendrier
            TopAct::Raccourcis => return self.palette_cible(argos_render::recherche::Cible::Raccourcis),
            TopAct::News => return self.palette_cible(argos_render::recherche::Cible::Section(argos_render::accueil::Section::Actualites)),
            TopAct::Agenda => return self.palette_cible(argos_render::recherche::Cible::Section(argos_render::accueil::Section::Calendrier)),
            TopAct::Gardien => return self.toggle_simulateur(),
            TopAct::Estomper => {
                self.top.prefs.estomper = !self.top.prefs.estomper;
                self.save_top();
                self.banner = Some(format!("🫥 Estomper les barres : {}", if self.top.prefs.estomper { "oui" } else { "non" }));
            }
            TopAct::AccueilLancement => self.toggle_accueil_lancement(),
            TopAct::Save => {
                self.save();
                self.garder_layout();
                self.banner = Some("💾 Mise en page sauvegardée".into());
            }
            TopAct::NewLayout => return self.nouvelle_layout(false),
            TopAct::Copy => return self.nouvelle_layout(true),
            TopAct::AutoSave => {
                self.top.prefs.auto_save = !self.top.prefs.auto_save;
                self.save_top();
            }
            TopAct::Rename => {
                let ww = self.win_w();
                self.ui.name_box = Some(NameBox::new(NameFor::Layout, &self.top.prefs.layout_name.clone(), anchor, ww));
            }
            TopAct::Reset => {
                (self.top.prefs.number_format, self.top.prefs.language) = (0, 0);
                argos_render::viewport::set_number_format(0);
                argos_render::i18n::set_lang(0);
                self.save_top();
            }
            TopAct::Updates
            | TopAct::Install
            | TopAct::Desktop
            | TopAct::CopyUrl
            | TopAct::Fullscreen
            | TopAct::ZoomOut
            | TopAct::ZoomReset
            | TopAct::ZoomIn
            | TopAct::ClearCache
            | TopAct::About
            | TopAct::Home
            | TopAct::ScreenTest => return self.app_act(a),
            other => self.banner = Some(format!("{} : bientôt dans la version Rust (comme dans Argos v1)", other.info().0)),
        }
        self.redraw();
    }

    /// Ouvre un menu ; le prix est celui sous la souris (ou 0 hors des bougies).
    pub(crate) fn open_menu(&mut self, source: MenuSource, anchor: Anchor) {
        let price = self.price_at_mouse().unwrap_or(0.0);
        self.ui.context.show(source, anchor, price, data_decimals(&self.chart.bars));
        self.redraw();
    }

    /// 🧩 Clic dans la colonne de modules (coordonnées de la colonne).
    pub(crate) fn dock_click(&mut self, x: f64, y: f64) {
        use argos_render::dock::Module;
        let Some((m, b)) = self.dock.bodies.iter().copied().find(|(_, b)| b.contains(x, y)) else {
            return;
        };
        let (lx, ly) = (x - b.x, y - b.y + self.dock.scroll.get(&m).copied().unwrap_or(0.0));
        let text = argos_render::text::Text::default();
        let url = match m {
            Module::Watchlist => {
                if let Some(c) = self.watch.mini_click(ly) {
                    return self.open_coin(c);
                }
                None
            }
            Module::Agenda | Module::News => {
                let kind = if m == Module::Agenda {
                    argos_render::eco::EcoKind::Agenda
                } else {
                    argos_render::eco::EcoKind::News
                };
                match self.watch.eco.click(kind, &text, b.w, lx, ly, self.watch.now) {
                    argos_render::eco::EcoClick::Open(u) => Some(u),
                    argos_render::eco::EcoClick::Learn(row) => return self.open_academy_news(row),
                    argos_render::eco::EcoClick::Changed => {
                        self.actualites_garder();
                        None
                    }
                    _ => None,
                }
            }
            // 📖 the order-book panel (without its own title: the column's header is above it)
            Module::Book => {
                let c = argos_render::carnet::click(&text, (b.w, b.h), &self.watch.prefs.carnet, &self.watch.carnet, false, false, lx, ly);
                let d = self.dock.rect.unwrap_or_default();
                let c = self.carnet_dans_fenetre(c, |_, r| Rect::new(d.x + b.x + r.x, d.y + b.y + r.y, r.w, r.h));
                return self.carnet_click(c);
            }
            Module::Trades if self.orderflow_enregistre().is_some() && argos_render::dock::tape_rec(b.w).contains(lx, ly) => {
                return self.orderflow_basculer();
            }
            Module::Explorer => {
                let c = self.watch.explorer.click(&text, b.w, lx, ly);
                return self.explorer_click(c);
            }
            Module::Stats => {
                let h = self.dock_content_h(m, b.w).max(b.h);
                return match self.watch.stats.click(b.w, h, lx, ly) {
                    argos_render::hlstats::StClick::Chart(c) => self.open_coin(c),
                    argos_render::hlstats::StClick::Changed => self.redraw(),
                    argos_render::hlstats::StClick::None => {}
                };
            }
            Module::Whales => {
                let c = self.watch.whales.click(&text, b.w, lx, ly);
                self.save_whales(true);
                match c {
                    argos_render::whales::WhaleClick::Open(u) => Some(u),
                    _ => None,
                }
            }
            // 🔑 Marché crypto : « 🔑 Clé API » et « 🔑 Ajoute ta clé CoinMarketCap pour plus » ouvrent la boîte de la clé
            // (comme dans le volet ; Jee, 20 nov. : le lien ne faisait rien dans la colonne)
            Module::Market
                if argos_render::cmc::key_rect(b.w, 0.0).contains(lx, ly)
                    || (!self.watch.market.cmc_cle && argos_render::cmc::cle_rect(b.w, 0.0).contains(lx, ly)) =>
            {
                self.cmc_key_box();
                return self.redraw();
            }
            _ => None,
        };
        if let Some(u) = url {
            platform::open_url(&u);
        }
    }
}

/// ⚙ Fenêtre « Réglages » de la maquette (🖥 PC).
impl App {
    /// ⚙ : ouvre (ou ferme) le volet Réglages (le volet flottant, comme les autres volets ; la chart s'arrête à son bord).
    pub(crate) fn reglages_ouvrir(&mut self) {
        let ouvert = self.ui.reglages.is_some() && self.watch.prefs.open && self.watch.prefs.panel == wl::Tool::Reglages;
        if ouvert {
            self.ui.reglages = None;
            self.watch.prefs.open = false;
        } else {
            (self.ui.list, self.ui.sheet, self.ui.argos, self.ui.notifs) = (None, None, None, None);
            (self.watch.prefs.open, self.watch.prefs.panel) = (true, wl::Tool::Reglages);
            // (sa place exacte est reprise à chaque image par la mise en page)
            let v = self
                .volet_rect()
                .unwrap_or(Rect::new(self.side_x() - 408.0, crate::render::VOLET_HAUT, 400.0, 600.0));
            self.ui.reglages = Some(argos_render::reglages::Reglages::dans_volet(v));
        }
        self.save_watch();
        self.redraw();
    }

    /// 📱 « Réglages » by the finger (Jee, 21 Nov, list 2: one way in, named what it opens): the « Réglages » sheet
    /// on the phone, the Réglages window on the tablet (« Panneaux » stays one of its rows).
    pub(crate) fn reglages_doigt(&mut self) {
        if self.pilule() {
            self.pfeuille_ouvrir(crate::actions::pilule::PKind::Reglages);
        } else if self.ui.reglages.is_none() {
            self.reglages_ouvrir();
        }
    }

    /// What a row of Réglages opens (the window, or the phone's sheet); `r`: where it was touched.
    pub(crate) fn reglages_cible(&mut self, c: argos_render::reglages::Cible, r: Rect) {
        use argos_render::reglages::Cible;
        let (x, y) = (r.cx(), r.cy());
        match c {
            // 🔊 son des alertes : sa liste, à côté de la ligne (le volet reste ouvert)
            Cible::Act(argos_render::sheet::Act::SonAlertes) => {
                return self.open_list(ListKind::SonAlertes, Rect::new(x - 120.0, y, 240.0, 1.0));
            }
            Cible::Act(a) => {
                self.ui.reglages = None;
                self.watch.prefs.open = false;
                return self.sheet_act(a);
            }
            Cible::Bientot(m) => self.banner = Some(m.to_string()),
            // ⌂ 📱 « Accueil » by the finger: its two choices, beside the row
            Cible::Accueil => return self.open_list(ListKind::Accueil, Rect::new(x - 120.0, y, 240.0, 1.0)),
            // 🫥 estompage : oui / non (le volet reste ouvert, son résumé change)
            Cible::Estomper => return self.top_act(topmenu::TopAct::Estomper as i64, Rect::new(x, y, 1.0, 1.0)),
            // ☰ Appli : le menu de l'appli (le volet se ferme)
            Cible::Appli => {
                self.ui.reglages = None;
                self.watch.prefs.open = false;
                let (ww, wh) = (self.win_w(), self.top_h() + self.area.1 as f64);
                self.ui.argos = Some(topmenu::ArgosMenu::app(Rect::new(ww - 480.0, 10.0, 440.0, 36.0), ww, wh));
            }
        }
        self.redraw();
    }

    /// Clic dans la fenêtre Réglages (position de la fenêtre de l'app).
    pub(crate) fn reglages_click(&mut self) {
        use argos_render::reglages::RegClick;
        let (x, y) = self.pointer.win_mouse;
        let Some(g) = self.ui.reglages.as_ref() else { return };
        match g.click(x, y) {
            RegClick::None => {}
            RegClick::Fermer => {
                self.ui.reglages = None;
                self.watch.prefs.open = false;
                self.save_watch();
            }
            RegClick::Agrandir => {
                self.ui.volet_large = !self.ui.volet_large;
                let l = self.ui.volet_large;
                Self::interface2_changer(|i| i.volet_large = l);
            }
            RegClick::Ligne(c) => return self.reglages_cible(c, Rect::new(x, y, 1.0, 1.0)),
            RegClick::Options => {
                self.ui.reglages = None;
                self.watch.prefs.open = false;
                return self.sheet_act(argos_render::sheet::Act::OpenPanels);
            }
        }
        self.redraw();
    }
}

/// 🖱 Groupe du curseur (1er bouton de la barre de gauche), comme TradingView.
impl App {
    /// Gomme dans le groupe du curseur.
    const ID_GOMME: i64 = 10;

    /// Le menu : Croix, Point, Flèche (le choisi coché), puis Gomme.
    pub(crate) fn menu_curseurs(&self) -> Vec<argos_render::listmenu::MenuGroup> {
        use crate::types::CurseurChoisi;
        use argos_render::listmenu::{MenuGroup, MenuItem};
        let icone = |c: CurseurChoisi| match c {
            CurseurChoisi::Croix => "cursor",
            CurseurChoisi::Point => "curseur-point",
            CurseurChoisi::Fleche => "curseur-fleche",
            CurseurChoisi::Demonstration => "highlighter",
        };
        let curseurs = CurseurChoisi::TOUS
            .iter()
            .enumerate()
            .map(|(k, c)| MenuItem {
                id: k as i64,
                label: c.nom().into(),
                icon: Some(icone(*c)),
                enabled: true,
                checked: self.tool == "cursor" && self.ui.curseur == *c,
                ..Default::default()
            })
            .collect();
        vec![
            MenuGroup {
                title: String::new(),
                items: curseurs,
                strip: false,
            },
            MenuGroup {
                title: String::new(),
                items: vec![MenuItem {
                    id: Self::ID_GOMME,
                    label: "Gomme".into(),
                    icon: Some("erase"),
                    enabled: true,
                    checked: self.tool == "erase",
                    // ☆ : la gomme dans la barre de favoris (comme les outils de dessin)
                    star: Some(self.rail.favs.iter().any(|f| f == "erase")),
                    ..Default::default()
                }],
                strip: false,
            },
        ]
    }

    /// Un choix du menu : le curseur (gardé dans interface-2.json, l'outil Curseur reprend) ou la gomme.
    pub(crate) fn curseurs_choix(&mut self, id: i64) {
        use crate::types::CurseurChoisi;
        if id == Self::ID_GOMME {
            self.set_tool("erase");
        } else if let Some(c) = CurseurChoisi::TOUS.get(id as usize).copied() {
            self.curseur_choisir(c);
            self.set_tool("cursor");
        }
        self.redraw();
    }

    /// 🖱 Le curseur choisi : gardé, montré au 1er bouton de la barre, posé tout de suite sur la chart.
    pub(crate) fn curseur_choisir(&mut self, c: crate::types::CurseurChoisi) {
        self.ui.curseur = c;
        Self::interface2_changer(|i| i.curseur = c);
        self.curseur_icone_rail();
        if let Some((mx, my)) = self.pointer.mouse {
            let k = self.curseur_chart(mx, my);
            self.poser_curseur(k);
        }
    }

    /// 🖱 L'icône du 1er bouton de la barre de gauche suit le curseur choisi.
    pub(crate) fn curseur_icone_rail(&self) {
        use crate::types::CurseurChoisi;
        rail::set_icone_curseur(match self.ui.curseur {
            CurseurChoisi::Croix => "cursor",
            CurseurChoisi::Point => "curseur-point",
            CurseurChoisi::Fleche => "curseur-fleche",
            CurseurChoisi::Demonstration => "highlighter",
        });
    }
}
