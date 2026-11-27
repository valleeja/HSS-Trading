//! ⭐ Watchlist : listes, prix, ajout de symboles, nom des listes.

use crate::*;

impl App {
    /// Liste de l'utilisateur affichée (`None` : la liste Hyperliquid automatique).
    pub(crate) fn user_list(&self) -> Option<&WatchList> {
        let id = self.watch.prefs.list?;
        self.lists.iter().find(|l| l.id == id)
    }

    /// Relit les listes de la base. Première fois : une liste « Mes favoris » pour commencer.
    pub(crate) fn load_lists(&mut self) {
        let Some(st) = self.store.as_mut() else { return };
        let mut lists = st.lists().unwrap_or_default();
        if lists.is_empty() {
            if let Ok(id) = st.create_list("Mes favoris") {
                for c in ["BTC", "ETH", "SOL", "HYPE", "PAXG"] {
                    let _ = st.add_item(id, "", &format!("hl:{c}"));
                }
            }
            lists = st.lists().unwrap_or_default();
        }
        // 🏦 une fois : la liste « OANDA 🥇 » de v1 (sections et symboles dans le même ordre)
        let mut seeds: Vec<String> = Self::read_prefs("seeds.json");
        if !seeds.iter().any(|s| s == "oanda") {
            use argos_data::oanda;
            if !lists.iter().any(|l| l.name == oanda::LIST_NAME) {
                if let Ok(id) = st.create_list(oanda::LIST_NAME) {
                    for (sec, list) in oanda::SEED {
                        let _ = st.add_section(id, sec);
                        for ins in list {
                            let _ = st.add_item(id, sec, &oanda::item(ins));
                        }
                    }
                }
                lists = st.lists().unwrap_or_default();
            }
            seeds.push("oanda".into());
            Self::write_prefs("seeds.json", &seeds);
        }
        // 🔄 les listes partagées (bureau, page web, coque) : écrites à chaque passage ici, seulement si elles ont
        // changé (navigateur : c'est aussi leur seule sauvegarde)
        let j = argos_store::listes::vers_json(&*st);
        if platform::read("watchlists.json").as_deref() != Some(j.as_str()) {
            let _ = platform::write("watchlists.json", &j);
        }
        self.lists = lists;
        if self.watch.prefs.list.is_some() && self.user_list().is_none() {
            self.watch.prefs.list = None; // liste supprimée ailleurs
        }
    }

    /// Remplit la watchlist : la liste Hyperliquid (sections automatiques) ou une liste à toi (prix Hyperliquid pour
    /// ses marchés Hyperliquid ; « — » pour les autres exchanges pour l'instant).
    pub(crate) fn refresh_watch(&mut self) {
        let hl = Exchange::Hyperliquid;
        // 🌐 « Tous les marchés » : son fil ne lit que la plateforme affichée, et seulement quand la vue est là
        let tous = self.watch.prefs.tous.then(|| self.tous_bourse());
        if let Ok(mut w) = self.feeds.tous_wanted.lock() {
            *w = tous;
        }
        self.watch.favoris = self
            .favoris_id()
            .and_then(|id| self.store.as_ref()?.items(id).ok())
            .unwrap_or_default()
            .into_iter()
            // (« hl:BTC » de la liste est aussi « BTC », la ligne Hyperliquid de « Tous les marchés »)
            .flat_map(|(_, c)| [c.strip_prefix("hl:").map(str::to_string), Some(c)])
            .flatten()
            .collect();
        let view: Vec<wl::Section> = match (self.user_list().cloned(), self.store.as_ref()) {
            _ if tous.is_some() => self.vue_tous(),
            (Some(l), Some(st)) => {
                let items = st.items(l.id).unwrap_or_default();
                let mut out: Vec<wl::Section> = std::iter::once(String::new())
                    .chain(st.sections(l.id).unwrap_or_default())
                    .map(|name| wl::Section { name, rows: vec![] })
                    .collect();
                for (sec, id) in items {
                    let (ex, sym) = id.split_once(':').unwrap_or(("", id.as_str()));
                    // symbole OANDA : son nom OANDA (XAUUSD), avec le prix de son équivalent Hyperliquid
                    let oanda = argos_data::oanda::label(&id);
                    let row = match self.feeds.quotes.get(sym).filter(|_| ex == hl.id()) {
                        Some(q) => {
                            let mut row = wl::Row::new(&id, &oanda.clone().unwrap_or_else(|| hl.ticker(sym)), &q.mark, q.prev);
                            if oanda.is_some() {
                                row.note = "OANDA".into();
                            }
                            row
                        }
                        None if argos_data::synth::parse_id(&id).is_some() => {
                            // 🧮 symbole calculé : son prix tiré des prix Hyperliquid HIP-3 (or ÷ livre…), DXY sans prix ici
                            let sym = argos_data::synth::parse_id(&id).unwrap_or_default();
                            let px = |c: &str| self.feeds.quotes.get(c).and_then(|q| Some((q.mark.parse::<f64>().ok()?, q.prev)));
                            let (mark, prev) = argos_data::synth::legs(sym)
                                .and_then(|(a, b)| {
                                    let ((pa, qa), (pb, qb)) = (px(a)?, px(b)?);
                                    (pb > 0.0 && qb > 0.0).then(|| (format!("{:.2}", pa / pb), qa / qb))
                                })
                                .unwrap_or((String::new(), 0.0));
                            let mut row = wl::Row::new(&id, sym, &mark, prev);
                            row.note = "calculé".into();
                            row
                        }
                        None => {
                            let ratio = argos_data::tv::parse_ratio_id(&id);
                            let cap = argos_data::cmcap::parse_id(&id);
                            let label = match (&ratio, cap) {
                                (_, Some(s)) => format!("CRYPTOCAP:{s}"),
                                (Some((_, b, q)), _) => argos_data::tv::ratio_name(b, q),
                                (None, None) => oanda.clone().unwrap_or_else(|| if ex == hl.id() { hl.ticker(sym) } else { sym.to_string() }),
                            };
                            let name = if cap.is_some() {
                                "CoinMarketCap"
                            } else if let Some((e, ..)) = &ratio {
                                e.name()
                            } else if oanda.is_some() {
                                "OANDA"
                            } else {
                                Exchange::parse(ex).map_or(ex, |e| e.name())
                            };
                            // autre exchange : son prix (fil des prix), écrit avec les décimales de son catalogue
                            let (mark, prev) = match cap {
                                // 📈 dernier relevé gardé (et celui d'il y a ~24 h pour la variation)
                                Some(s) => cap_mark(&self.feeds.cmc_hist, s),
                                None => self
                                    .price_of(&id)
                                    .map_or((String::new(), 0.0), |(m, dec)| (format!("{:.*}", dec, m.price), m.prev)),
                            };
                            let mut row = wl::Row::new(&id, &label, &mark, prev);
                            row.note = name.to_string();
                            row
                        }
                    };
                    if let Some(s) = out.iter_mut().find(|s| s.name == sec) {
                        s.rows.push(row);
                    }
                }
                // la section sans nom ne s'affiche que si elle a des symboles
                out.retain(|s| !s.name.is_empty() || !s.rows.is_empty());
                let n: usize = out.iter().map(|s| s.rows.len()).sum();
                (self.watch.title, self.watch.note, self.watch.editable) = (l.name.clone(), format!("{n} symbole(s)"), true);
                out
            }
            _ => {
                (self.watch.title, self.watch.note, self.watch.editable) = ("Hyperliquid".into(), "auto".into(), false);
                self.feeds
                    .hl_sections
                    .iter()
                    .map(|s| wl::Section {
                        name: s.name.clone(),
                        rows: s.quotes.iter().map(|q| wl::Row::new(&q.coin, &hl.ticker(&q.coin), &q.mark, q.prev)).collect(),
                    })
                    .collect()
            }
        };
        // 📱 les listes en pilules sous l'en-tête du cell (mêmes ids que le menu des listes)
        self.watch.listes = [(0, "Hyperliquid".to_string()), (ID_TOUS, TOUS.to_string())]
            .into_iter()
            .chain(self.lists.iter().enumerate().map(|(i, l)| (i as i64 + 1, l.name.clone())))
            .collect();
        let cur_list = self.watch.prefs.list;
        self.watch.liste_active = match tous {
            Some(_) => ID_TOUS,
            None => cur_list.and_then(|id| self.lists.iter().position(|l| l.id == id)).map_or(0, |i| i as i64 + 1),
        };
        self.watch.update(view);
        self.want_prices();
        // logos des nouveaux symboles (une seule demande par symbole) : clé = la ligne, logo cherché par son marché ;
        // avec la clé CoinMarketCap, toute monnaie de base (carte officielle des ids)
        let coins: Vec<String> = self.watch.sections.iter().flat_map(|s| s.rows.iter().map(|r| r.coin.clone())).collect();
        let avec_cle = self.session.cmc.is_some();
        let new: Vec<(String, String)> = coins
            .into_iter()
            .filter_map(|c| {
                // (₿ « XRP/BTC »: XRP's logo)
                let base = argos_data::paire::asset(&self.base_of(&c)).to_string();
                (argos_data::logos::demandable(&base, avec_cle) && self.logos_asked.insert(c.clone())).then_some((c, base))
            })
            .collect();
        // logos : gardés sur le disque du PC ; dans le navigateur, demandés au serveur du PC
        if let (false, Some(proxy)) = (new.is_empty(), self.proxy.clone()) {
            let on_logo = move |coin, png| proxy.send_event(UserEvent::Logo(coin, png)).is_ok();
            #[cfg(not(target_arch = "wasm32"))]
            argos_data::logos::spawn(new, on_logo);
            #[cfg(target_arch = "wasm32")]
            argos_data::logos::par_le_serveur(new, on_logo);
        }
    }

    /// Prix d'une ligne d'un autre exchange (« binance:BTCUSDT ») et ses décimales ; `None` sans prix encore.
    pub(crate) fn price_of(&self, id: &str) -> Option<(argos_core::feed::Market, usize)> {
        let ex = Exchange::parse(id.split_once(':')?.0)?;
        let inst = self.store.as_ref()?.instrument(id).ok().flatten()?;
        let m = self.feeds.ex_prices.get(&ex)?.get(&argos_data::paire::base(&inst.base, &inst.quote))?.clone();
        Some((m, inst.price_decimals as usize))
    }

    /// Exchanges dont il faut les prix : ceux de la liste affichée et celui du symbole ouvert (hors Hyperliquid).
    pub(crate) fn want_prices(&self) {
        let mut want: Vec<Exchange> = self
            .watch
            .sections
            .iter()
            .flat_map(|s| &s.rows)
            .filter_map(|r| r.coin.split_once(':').and_then(|(e, _)| Exchange::parse(e)))
            .collect();
        if let Source::Live { exchange, .. } = &self.chart.source {
            want.push(*exchange);
        }
        want.retain(|e| *e != Exchange::Hyperliquid);
        want.sort_by_key(|e| e.id());
        want.dedup();
        if let Ok(mut w) = self.feeds.price_wanted.lock() {
            *w = want;
        }
        self.want_hl_live();
    }

    /// 📡 Hyperliquid markets for the live feed: those on screen (ticker band, list rows in view, from the last
    /// frame), the chart, the demo positions and the alerts (as `demo_price` reads them). Unknown names are left out
    /// by the feed; lines of other exchanges are not Hyperliquid's.
    pub(crate) fn want_hl_live(&self) {
        let hl = Exchange::Hyperliquid;
        let mut want: Vec<String> = vec![];
        for id in &self.feeds.hl_visibles {
            match id.split_once(':') {
                Some((ex, sym)) if Exchange::parse(ex).is_some() => {
                    if ex == hl.id() {
                        want.push(sym.to_string());
                    }
                }
                _ => want.push(id.clone()),
            }
        }
        if let Source::Live { exchange, base, .. } = &self.chart.source {
            if *exchange == hl {
                want.push(argos_data::exchanges::hyperliquid::coin(base));
            }
        }
        let mut bases: Vec<String> = self.demo.positions.iter().map(|p| p.base.clone()).collect();
        bases.extend(self.alerts.symbols());
        for b in bases {
            if let Some(c) = argos_data::oanda::OANDA.iter().find(|o| o.0.replace('_', "") == b).and_then(|o| o.4) {
                want.push(c.to_string());
            }
            want.push(argos_data::whales::coin_of(&b));
            want.push(b);
        }
        let mut seen = std::collections::HashSet::new();
        want.retain(|c| seen.insert(c.clone()));
        if let Ok(mut w) = self.feeds.hl_wanted.lock() {
            *w = want;
        }
    }

    /// Marché d'une ligne de watchlist : « hl:BTC » → BTC ; « binance:BTCUSDT » → BTC (catalogue) ; « BTC » → BTC.
    pub(crate) fn base_of(&self, coin: &str) -> String {
        match coin.split_once(':') {
            Some((ex, sym)) if Exchange::parse(ex).is_some() => {
                if ex == Exchange::Hyperliquid.id() {
                    return sym.to_string();
                }
                // Bitfinex: its symbol says it all (« XRPBTC » → XRP/BTC, « XRPF0:USTF0 » → XRP), catalogue or not
                if let Some(perp) = Exchange::parse(ex).and_then(|e| e.bitfinex()) {
                    if let Some(b) = argos_data::exchanges::bitfinex::base_de(&format!("t{sym}"), perp) {
                        return b;
                    }
                }
                self.store
                    .as_ref()
                    .and_then(|s| s.instrument(coin).ok().flatten())
                    // ₿ a pair against BTC opens as « XRP/BTC » (the exchange's BTC pair, not its USDT one)
                    .map_or(sym.to_string(), |i| argos_data::paire::base(&i.base, &i.quote))
            }
            _ => coin.to_string(),
        }
    }

    /// Ouvre le symbole d'une ligne de watchlist (n'importe quel exchange).
    pub(crate) fn open_coin(&mut self, coin: String) {
        // 🔎 récents du volet Symboles (le plus récent d'abord)
        self.ui.sym_recents.retain(|c| *c != coin);
        self.ui.sym_recents.insert(0, coin.clone());
        self.ui.sym_recents.truncate(8);
        // 🪟 symbole choisi pour une case du split (clic sur son nom) : seulement cette case
        if let Some(k) = self.ui.pane_symbole.take() {
            if !self.pane_set_coin(k, &coin) {
                self.banner = Some("🪟 Ce symbole ne peut pas encore aller dans une case (exchanges seulement)".into());
            }
            return self.redraw();
        }
        // 🏦 symbole OANDA : avec ton jeton = les vrais prix OANDA ; sans jeton = son équivalent Hyperliquid (s'il en a un)
        if let Some(ins) = argos_data::oanda::instrument_of(&coin) {
            if self.session.oanda.is_some() {
                if !matches!(&self.chart.source, Source::Oanda { instrument } if *instrument == ins) {
                    self.save();
                    self.write_cache();
                    self.open_oanda(ins);
                }
                self.watch.selected = coin;
                return self.redraw();
            }
            if coin.starts_with("oanda:") {
                self.banner = Some(format!(
                    "🏦 {} : ajoute ton jeton OANDA dans 🔌 API pour ce symbole (pas d'équivalent Hyperliquid)",
                    argos_data::oanda::label(&coin).unwrap_or_default()
                ));
                return self.redraw();
            }
        }
        // 📈 série CoinMarketCap (« cap:USDT.D »)
        if let Some(series) = argos_data::cmcap::parse_id(&coin) {
            if !matches!(&self.chart.source, Source::Cap { series: s } if s == series) {
                self.open_cap(series);
            }
            if self.session.cmc.is_some() {
                self.watch.selected = coin;
            }
            return self.redraw();
        }
        // 🦎 paire DEX (« dex:eth:0x…:PEPE/WETH »)
        if let Some((network, pool, name)) = argos_data::dex::parse_id(&coin) {
            if !matches!(&self.chart.source, Source::Dex { pool: p, .. } if *p == pool) {
                self.save();
                self.write_cache();
                self.open_dex(network, pool, name);
            }
            self.watch.selected = coin;
            return self.redraw();
        }
        // 🧮 symbole calculé gratuit (« synth:XAUGBP », « synth:DXY »)
        if let Some(sym) = argos_data::synth::parse_id(&coin) {
            if !matches!(&self.chart.source, Source::Synth { sym: s } if s == sym) {
                self.save();
                self.write_cache();
                self.open_synth(sym.to_string());
            }
            self.watch.selected = coin;
            return self.redraw();
        }
        // 🧮 paire calculée (« ratio:binance:XRP:BTC »)
        if let Some((ex, base, quote)) = argos_data::tv::parse_ratio_id(&coin) {
            if !matches!(&self.chart.source, Source::Ratio { exchange, base: b, quote: q } if *exchange == ex && *b == base && *q == quote) {
                self.save();
                self.write_cache();
                self.open_ratio(ex, base, quote);
            }
            self.watch.selected = coin;
            return self.redraw();
        }
        let ex = coin.split_once(':').and_then(|(e, _)| Exchange::parse(e)).unwrap_or(Exchange::Hyperliquid);
        let base = self.base_of(&coin);
        self.switch_symbol(ex, base);
        self.watch.selected = coin;
        self.redraw();
    }

    /// Filtres du panneau ＋ : Tout, les exchanges (comme v1), puis les groupes du catalogue.
    pub(crate) fn add_chips(&self) -> Vec<Chip> {
        let chip = |key: String, label: String, dot: Option<argos_render::text::Rgb>| Chip { key, label, dot };
        let mut out = vec![chip(String::new(), "Tout".into(), None)];
        // un filtre par exchange (tous : on ajoute un symbole de n'importe quelle plateforme), avec sa pastille (comme
        // v1 : vert Hyperliquid, jaune Binance, blanc WEEX…)
        for e in Exchange::ALL {
            out.push(chip(format!("ex:{}", e.id()), e.name().to_string(), Some(ex_dot(e))));
        }
        let groups = self.store.as_ref().and_then(|s| s.groups().ok()).unwrap_or_default();
        out.extend(groups.into_iter().map(|(g, _)| chip(format!("grp:{g}"), g, None)));
        out
    }

    /// Refait la recherche du panneau ＋ (texte + filtre) ; « déjà là » = déjà dans la liste affichée.
    pub(crate) fn search_add(&mut self) {
        let Some(p) = self.ui.add.as_ref() else { return };
        let f = p.filter();
        let r = self.chercher_marches(&p.query, f.strip_prefix("ex:"), f.strip_prefix("grp:"));
        let found = r.unwrap_or_else(|msg| {
            self.banner = Some(format!("🏷️ {msg}"));
            vec![]
        });
        if let Some(p) = self.ui.add.as_mut() {
            p.set_results(found);
        }
    }

    /// 🔎 The market search engine, shared by the ＋ panel and the market panel of the symbol pill: argos-store base,
    /// DEX, OANDA, computed pairs, TradingView format; « déjà là » = already in the list shown. `Err`: a message.
    pub(crate) fn chercher_marches(&self, query: &str, exchange: Option<&str>, group: Option<&str>) -> Result<Vec<Found>, String> {
        let Some(st) = self.store.as_ref() else { return Ok(vec![]) };
        let q = Search {
            query,
            exchange,
            group,
            limit: 400,
        };
        let have: std::collections::HashSet<String> = match self.watch.prefs.list {
            Some(id) => st.items(id).unwrap_or_default().into_iter().map(|(_, i)| i).collect(),
            None => Default::default(),
        };
        // 🦎 « dex:pepe » ou une adresse de contrat : recherche sur les DEX (Entrée)
        if let Some(found) = self.dex_hint(query) {
            return Ok(found);
        }
        // 🏦 « OANDA:XAUUSD » : les instruments OANDA d'Argos (avec ou sans jeton : sans, le prix de leur équivalent
        // Hyperliquid), jamais « préfixe pas encore géré »
        let oanda = oanda_results(query, &have);
        if query.trim().to_uppercase().starts_with("OANDA:") {
            return Ok(oanda);
        }
        // 🧮 XAUGBP, XAUEUR, XAGEUR, DXY : calculés, gratuits (un seul résultat par symbole, avant le format TradingView
        // qui lirait « XAUGBP » comme une paire XAU ÷ GBP d'un exchange crypto)
        let synth = synth_results(query, &have, self.session.oanda.is_some());
        let exact = argos_data::synth::SYNTH
            .iter()
            .any(|x| x.0.eq_ignore_ascii_case(&query.trim().replace(['/', ' ', '_'], "")));
        if exact {
            return Ok(synth);
        }
        // 🏷️ ticker écrit comme TradingView (« BINANCE:BTCUSDT.P », « XRPBTC », « XRPUSDT/BTCUSDT »)
        if let Some(found) = tv_results(st, query, &have, self.session.cmc.is_some())? {
            return Ok(found);
        }
        let groups: Vec<String> = st.groups().unwrap_or_default().into_iter().map(|(g, _)| g).collect();
        let mut found: Vec<Found> = st
            .search(&q)
            .unwrap_or_default()
            .into_iter()
            .map(|i| {
                let ex = Exchange::parse(&i.exchange);
                let name = if ex == Some(Exchange::Hyperliquid) {
                    Exchange::Hyperliquid.ticker(&i.symbol)
                } else {
                    i.symbol.clone()
                };
                let mut detail = ex.map_or(i.exchange.clone(), |e| e.name().to_string());
                if !i.active {
                    detail.push_str(" · plus listé");
                }
                Found {
                    have: have.contains(&i.id()) || !i.active,
                    id: i.id(),
                    name,
                    detail,
                    group: i.group,
                }
            })
            .collect();
        // regroupés par groupe (le plus gros d'abord), l'ordre de la recherche gardé dans chaque groupe
        found.sort_by_key(|f| groups.iter().position(|g| *g == f.group).unwrap_or(usize::MAX));
        // 🧮 pas de doublon : l'ancien marché « xyz:DXY » (plus listé) cède la place au DXY d'Argos
        if !synth.is_empty() {
            found.retain(|f| !(f.have && argos_data::synth::parse_id(&argos_data::synth::id(f.name.rsplit(':').next().unwrap_or(""))).is_some()));
        }
        found.splice(0..0, synth);
        found.retain(|f| !oanda.iter().any(|o| o.id == f.id));
        found.extend(oanda);
        Ok(found)
    }

    /// Ajoute un symbole à la liste affichée (liste Hyperliquid affichée : dans ta 1re liste, qui s'affiche), à la fin
    /// de la dernière section.
    pub(crate) fn add_symbol(&mut self, id: &str) {
        // 🦎 ligne « chercher sur les DEX » : la recherche part, rien n'est ajouté
        if let Some(q) = id.strip_prefix(crate::actions::DEX_ASK) {
            return self.dex_search(q.to_string());
        }
        if self.watch.prefs.list.is_none() {
            self.watch.prefs.list = self.lists.first().map(|l| l.id);
        }
        let (Some(list), Some(st)) = (self.watch.prefs.list, self.store.as_mut()) else {
            return;
        };
        let section = st.sections(list).unwrap_or_default().pop().unwrap_or_default();
        if let Err(e) = st.add_item(list, &section, id) {
            self.banner = Some(format!("⚠️ {e}"));
        }
        self.save_watch();
        self.refresh_watch();
        self.search_add();
    }

    /// Menu des listes (comme v1) : les listes et leur nombre de symboles, 🚩 voir une couleur, puis les actions.
    pub(crate) fn lists_menu(&self) -> Vec<MenuGroup> {
        let count = |id: i64| self.store.as_ref().and_then(|s| s.items(id).ok()).map_or(0, |v| v.len());
        let cur = self.watch.prefs.list;
        let tous = self.watch.prefs.tous;
        let mut lists = vec![
            MenuItem {
                id: 0,
                label: "Hyperliquid (auto)".into(),
                enabled: true,
                checked: cur.is_none() && !tous,
                ..Default::default()
            },
            MenuItem {
                id: ID_TOUS,
                label: TOUS.into(),
                enabled: true,
                checked: tous,
                ..Default::default()
            },
        ];
        lists.extend(self.lists.iter().enumerate().map(|(i, l)| MenuItem {
            id: i as i64 + 1,
            label: l.name.clone(),
            enabled: true,
            checked: cur == Some(l.id) && !tous,
            hint: count(l.id).to_string(),
            ..Default::default()
        }));
        let ff = self.watch.prefs.flag_filter;
        let names = ["Rouge", "Bleu", "Vert", "Jaune", "Mauve", "Cyan", "Rose"];
        let mut flags = vec![MenuItem {
            id: ID_FLAG_ALL,
            label: "Tout".into(),
            enabled: true,
            checked: ff.is_none(),
            ..Default::default()
        }];
        flags.extend(wl::FLAG_COLORS.iter().zip(names).enumerate().map(|(k, (c, n))| MenuItem {
            id: ID_FLAG_ALL + 1 + k as i64,
            label: n.into(),
            enabled: true,
            checked: ff == Some(k),
            color: Some(*c),
            hint: self.watch.prefs.flags.iter().filter(|(_, f)| *f == k).count().to_string(),
            ..Default::default()
        }));
        let mine = self.user_list().filter(|_| !tous);
        let act = |id: i64, label: String, enabled: bool| MenuItem {
            id,
            label,
            enabled,
            ..Default::default()
        };
        let delete = match mine {
            Some(l) if self.ui.delete_armed => format!("Confirmer : supprimer « {} »", l.name),
            _ => "Supprimer la liste".into(),
        };
        vec![
            MenuGroup {
                title: String::new(),
                items: lists,
                strip: false,
            },
            MenuGroup {
                title: "Voir :".into(),
                items: flags,
                strip: true,
            },
            MenuGroup {
                title: String::new(),
                items: vec![
                    act(ID_NEW, "+  Nouvelle liste".into(), true),
                    act(ID_SECTION, "+  Ajouter une section".into(), mine.is_some()),
                    act(ID_RENAME, "Renommer la liste".into(), mine.is_some()),
                    act(ID_DELETE, delete, mine.is_some() && self.lists.len() > 1),
                ]
                .into_iter()
                // 📱 sur cell, 🔌 n'est plus dans l'en-tête : les sources et clés API sont ici
                .chain(self.top.phone.then(|| act(ID_API, "🔌  Sources et clés API".into(), true)))
                // 📱 au doigt : pas de ligne grisée (Jee, 21 nov., liste 2)
                .filter(|it| it.enabled || !argos_render::actions::tactile())
                .collect(),
                strip: false,
            },
        ]
        .into_iter()
        // 📱 sur cell au doigt : les listes sont déjà en puces sous le menu, pas deux fois (Jee, 21 nov., liste 2)
        .skip(usize::from(self.top.phone && argos_render::actions::tactile()))
        .collect()
    }

    /// 📱 Le menu d'une section de ta liste : renommer, monter, descendre, supprimer.
    pub(crate) fn section_menu(&self) -> Vec<MenuGroup> {
        let it = |id: i64, label: &str| MenuItem {
            id,
            label: label.into(),
            enabled: true,
            ..Default::default()
        };
        vec![MenuGroup {
            title: self.ui.section.clone().unwrap_or_default(),
            items: vec![
                it(1, "✏️  Renommer la section"),
                it(2, "⬆  Monter"),
                it(3, "⬇  Descendre"),
                it(4, "🗑  Supprimer la section"),
            ],
            strip: false,
        }]
    }

    /// 📱 Choix dans le menu d'une section.
    pub(crate) fn section_pick(&mut self, id: i64, anchor: Rect) {
        let (Some(name), Some(l)) = (self.ui.section.clone(), self.watch.prefs.list) else {
            return;
        };
        if id == 1 {
            let ww = self.win_w();
            self.ui.name_box = Some(NameBox::new(NameFor::RenameSection, &name, anchor, ww));
            return self.redraw();
        }
        if let Some(st) = self.store.as_mut() {
            let _ = match id {
                2 => st.move_section(l, &name, -1),
                3 => st.move_section(l, &name, 1),
                4 => st.remove_section(l, &name),
                _ => Ok(()),
            };
        }
        self.ui.section = None;
        self.refresh_watch();
        self.redraw();
    }

    /// Ligne choisie dans le menu des listes.
    pub(crate) fn lists_pick(&mut self, id: i64, anchor: Rect) {
        let armed = std::mem::take(&mut self.ui.delete_armed);
        let ww = self.win_w();
        match id {
            0 => (self.watch.prefs.list, self.watch.prefs.tous) = (None, false),
            ID_TOUS => self.watch.prefs.tous = true,
            ID_FLAG_ALL => self.watch.prefs.flag_filter = None,
            ID_NEW => self.ui.name_box = Some(NameBox::new(NameFor::NewList, "", anchor, ww)),
            ID_SECTION => self.ui.name_box = Some(NameBox::new(NameFor::Section, "", anchor, ww)),
            ID_RENAME => {
                let name = self.user_list().map(|l| l.name.clone()).unwrap_or_default();
                self.ui.name_box = Some(NameBox::new(NameFor::Rename, &name, anchor, ww));
            }
            ID_DELETE if !armed => {
                // 1er clic : le menu reste ouvert et demande de confirmer (comme v1)
                self.ui.delete_armed = true;
                return self.open_list(ListKind::Lists, anchor);
            }
            ID_API => {
                (self.watch.prefs.open, self.watch.prefs.panel) = (true, wl::Tool::Exchanges);
                self.save_watch();
                return self.redraw();
            }
            ID_DELETE => {
                if let (Some(list), Some(st)) = (self.watch.prefs.list, self.store.as_mut()) {
                    let _ = st.delete_list(list);
                }
                self.watch.prefs.list = None;
                self.load_lists();
            }
            k if k > ID_FLAG_ALL && k <= ID_FLAG_ALL + wl::FLAG_COLORS.len() as i64 => {
                self.watch.prefs.flag_filter = Some((k - ID_FLAG_ALL - 1) as usize);
            }
            k => (self.watch.prefs.list, self.watch.prefs.tous) = (self.lists.get(k as usize - 1).map(|l| l.id), false),
        }
        self.save_watch();
        self.refresh_watch();
    }

    /// OK dans la boîte de nom.
    pub(crate) fn name_ok(&mut self) {
        let Some(b) = self.ui.name_box.take() else { return };
        if b.kind == NameFor::Wallet {
            return self.wallet_typed(&b.value);
        }
        if b.kind == NameFor::Idea {
            self.idea_typed(&b.value);
            return self.redraw();
        }
        if b.kind == NameFor::DrawingTemplate {
            return self.template_typed(&b.value);
        }
        if b.kind == NameFor::Raison {
            self.guard_unlock(&b.value);
            return self.redraw();
        }
        if b.kind == NameFor::AlertValue {
            self.alert_value_typed(&b.value);
            return self.redraw();
        }
        if b.kind == NameFor::ModNumber {
            self.mod_number_typed(&b.value);
            return self.redraw();
        }
        if let NameFor::Ticket(c) = b.kind {
            self.ticket_typed(c, &b.value);
            return self.redraw();
        }
        if matches!(b.kind, NameFor::AiNote | NameFor::AiMethod) {
            self.ai_memory_typed(b.kind, &b.value);
            return self.redraw();
        }
        // ⏱ intervalle personnalisé : ajouté au menu et à la barre (favori), puis choisi
        if b.kind == NameFor::Interval {
            let case = self.ui.ut_case.take();
            match Interval::parse(&b.value).filter(|iv| self.tools.add_custom(iv.secs())) {
                Some(iv) => {
                    self.save_tools();
                    match case {
                        Some(k) => return self.pane_set_interval(k, iv),
                        None => return self.set_interval(iv),
                    }
                }
                None => self.banner = Some(format!("⏱️ « {} » : écris un nombre et une unité (ex. 90m, 2h, 10D, 3W, 2M)", b.value.trim())),
            }
            return self.redraw();
        }
        if b.kind == NameFor::Lang {
            match argos_render::i18n::find(&b.value) {
                Some(k) => return self.ask_lang(k, b.rect),
                None => self.banner = Some(format!("🔎 « {} » : cette langue n'est pas encore dans Argos", b.value.trim())),
            }
            return self.redraw();
        }
        if b.kind == NameFor::Account {
            if let Some(name) = b.name() {
                self.demo.name = name;
                self.save_accounts();
            }
            return self.redraw();
        }
        if b.kind == NameFor::Layout {
            let cible = self.ui.layout_cible.take();
            if let Some(name) = b.name() {
                return match cible {
                    Some(ancien) => self.renommer_layout_de(&ancien, name),
                    None => self.renommer_layout(name),
                };
            }
            return self.redraw();
        }
        let (Some(name), Some(st)) = (b.name(), self.store.as_mut()) else {
            return self.redraw();
        };
        let list = self.watch.prefs.list;
        match (b.kind, list) {
            (NameFor::NewList, _) => {
                if let Ok(id) = st.create_list(&name) {
                    (self.watch.prefs.list, self.watch.prefs.tous) = (Some(id), false);
                }
            }
            (NameFor::Section, Some(l)) => {
                let _ = st.add_section(l, &name);
            }
            (NameFor::RenameSection, Some(l)) => {
                if let Some(old) = self.ui.section.take() {
                    let _ = st.rename_section(l, &old, &name);
                }
            }
            (NameFor::Rename, Some(l)) => {
                if let Err(e) = st.rename_list(l, &name) {
                    self.banner = Some(format!("⚠️ {e}"));
                }
            }
            _ => {}
        }
        self.load_lists();
        self.save_watch();
        self.refresh_watch();
        self.redraw();
    }

    pub(crate) fn save_watch(&self) {
        Self::write_prefs("watchlist.json", &self.watch.prefs);
    }
}

/// 🏦 Instruments OANDA d'Argos qui répondent à la recherche (« XAU », « xauusd », « OANDA:XAUUSD », « Or »), sous leur
/// nom OANDA ; l'id est celui de la liste OANDA de départ (l'équivalent Hyperliquid quand il y en a un).
pub(crate) fn oanda_results(query: &str, have: &std::collections::HashSet<String>) -> Vec<Found> {
    let q = query.trim().to_uppercase();
    let q = q.strip_prefix("OANDA:").unwrap_or(&q).replace(['_', '/', ' '], "");
    if q.len() < 2 {
        return vec![];
    }
    argos_data::oanda::OANDA
        .iter()
        .filter(|o| o.0.replace('_', "").starts_with(&q) || o.1.to_uppercase().replace(' ', "").contains(&q))
        .map(|o| {
            let id = argos_data::oanda::item(o.0);
            Found {
                have: have.contains(&id),
                name: format!("OANDA:{}", o.0.replace('_', "")),
                detail: format!("{} · OANDA", o.1),
                group: "OANDA".into(),
                id,
            }
        })
        .collect()
}

/// 🏷️ Résultats d'un ticker TradingView dans le panneau ＋ : `Ok(None)` = recherche normale, `Err` = message clair
/// (préfixe pas encore géré, marché introuvable).
/// 🧮 Résultats « symboles calculés » d'une recherche (XAUGBP, XAUEUR, XAGEUR, DXY), un par symbole : la source
/// est écrite à droite (OANDA avec ton jeton, sinon calculé gratuitement).
pub(crate) fn synth_results(query: &str, have: &std::collections::HashSet<String>, oanda: bool) -> Vec<Found> {
    use argos_data::synth;
    synth::search(query)
        .into_iter()
        .map(|s| {
            let id = synth::id(s);
            let detail = match (synth::oanda_of(s), synth::legs(s)) {
                (Some(_), _) if oanda => format!("{} · OANDA (ton jeton)", synth::name(s)),
                (_, Some((a, b))) => format!("{} · calculé, approximatif : {a} ÷ {b} (Hyperliquid, gratuit)", synth::name(s)),
                _ if cfg!(target_arch = "wasm32") => format!("{} · Yahoo, sinon calculé (5 devises)", synth::name(s)),
                _ => format!("{} · Yahoo (gratuit)", synth::name(s)),
            };
            Found {
                have: have.contains(&id),
                name: s.to_string(),
                detail,
                group: "Forex & métaux calculés".into(),
                id,
            }
        })
        .collect()
}

pub(crate) fn tv_results(st: &Store, query: &str, have: &std::collections::HashSet<String>, cmc_on: bool) -> Result<Option<Vec<Found>>, String> {
    use argos_data::cmcap;
    use argos_data::tv::{self, Tv};
    const GROUP: &str = "Format TradingView";
    // 📈 CRYPTOCAP:USDT.D, TOTAL3… : séries CoinMarketCap (avec ta clé ; sans clé, le message pour l'ajouter)
    let caps = cmcap::series_matching(query);
    if !caps.is_empty() {
        return Ok(Some(
            caps.into_iter()
                .map(|s| {
                    let id = cmcap::id(s);
                    let desc = cmcap::SERIES.iter().find(|(n, _)| *n == s).map_or("", |(_, d)| d);
                    Found {
                        have: have.contains(&id),
                        name: format!("CRYPTOCAP:{s}"),
                        detail: if cmc_on {
                            format!("{desc} · CoinMarketCap")
                        } else {
                            format!("{desc} · historique local (CoinGecko, gratuit)")
                        },
                        group: "CoinMarketCap".into(),
                        id,
                    }
                })
                .collect(),
        ));
    }
    if query.trim().to_uppercase().starts_with("CRYPTOCAP:") {
        let all: Vec<&str> = cmcap::SERIES.iter().map(|(n, _)| *n).collect();
        return Err(format!(
            "{} : pas encore géré (séries offertes : {})",
            query.trim().to_uppercase(),
            all.join(", ")
        ));
    }
    let ratio = |ex: Exchange, base: &str, quote: &str| {
        let id = tv::ratio_id(ex, base, quote);
        Found {
            have: have.contains(&id),
            name: tv::ratio_name(base, quote),
            detail: format!("{} · calculée ({base} ÷ {quote})", ex.name()),
            group: "Paires calculées".into(),
            id,
        }
    };
    let t = match tv::parse(query) {
        Tv::Plain => return Ok(None),
        Tv::NotYet(msg) => return Err(msg),
        Tv::Ratio(a, b) => {
            if a.exchange.is_some() && b.exchange.is_some() && a.exchange != b.exchange {
                return Err("une paire calculée se fait sur un seul exchange (ex. BINANCE:XRPUSDT.P/BINANCE:BTCUSDT.P)".into());
            }
            let ex = a.exchange.or(b.exchange).unwrap_or(tv::RATIO_DEFAULT);
            return Ok(Some(vec![ratio(ex, &a.base, &b.base)]));
        }
        // ₿ « XRPBTC », « xrp/btc » : les vraies paires XRP contre BTC de chaque bourse, puis la paire calculée
        // d'avant (en dernier, toujours là)
        Tv::Ticker(t) if t.is_btc_pair() => {
            let mut found = paires_btc(st, &t.base, t.exchange, have);
            found.push(ratio(t.exchange.map_or(tv::RATIO_DEFAULT, |e| e.parent()), &t.base, "BTC"));
            return Ok(Some(found));
        }
        Tv::Ticker(t) => t,
    };
    // 🪙 spot demandé (« BINANCE:BTCUSDT ») : le spot d'abord, puis le perpétuel du même exchange en secours
    let exchanges: Vec<Exchange> = match t.exchange {
        Some(e) if e.spot_venue().is_some() => vec![e, e.parent()],
        Some(e) => vec![e],
        None => Exchange::ALL.to_vec(),
    };
    let mut found = vec![];
    for ex in exchanges {
        let q = Search {
            query: &t.base,
            exchange: Some(ex.id()),
            group: None,
            limit: 60,
        };
        // le perpétuel de secours seulement si le spot n'a rien donné
        if t.exchange.is_some_and(|e| e.spot_venue().is_some() && e != ex) && !found.is_empty() {
            continue;
        }
        for i in st.search(&q).unwrap_or_default() {
            let hl = ex == Exchange::Hyperliquid;
            let same_base = i.base.eq_ignore_ascii_case(&t.base) || (hl && i.symbol.eq_ignore_ascii_case(&t.base));
            let same_quote = (hl && t.exchange.is_some()) || t.quote.as_deref().is_none_or(|q| i.quote.eq_ignore_ascii_case(q));
            let kind_ok = !t.perp || i.kind == argos_core::MarketKind::Perp;
            if !(same_base && same_quote && kind_ok) {
                continue;
            }
            // spot demandé sur un exchange qui n'a que des perpétuels dans Argos : on propose le perpétuel, en le disant
            let spot_asked = !t.perp && i.kind == argos_core::MarketKind::Perp && !hl;
            let mut detail = ex.name().to_string();
            if spot_asked {
                detail.push_str(" · perpétuel (spot pas encore chargé)");
            }
            if !i.active {
                detail.push_str(" · plus listé");
            }
            found.push(Found {
                have: have.contains(&i.id()) || !i.active,
                id: i.id(),
                name: tv::name(ex, if hl { &i.symbol } else { &i.base }),
                detail,
                group: GROUP.into(),
            });
        }
    }
    if found.is_empty() {
        let wher = t.exchange.map_or("aucun exchange".to_string(), |e| e.name().to_string());
        return Err(format!(
            "{} introuvable sur {wher} (catalogue pas encore chargé ?)",
            query.trim().to_uppercase()
        ));
    }
    Ok(Some(found))
}

/// ₿ The exchanges whose catalogue has spot pairs against BTC.
pub(crate) const BOURSES_BTC: [Exchange; 10] = [
    Exchange::BitfinexSpot,
    Exchange::Bitfinex,
    Exchange::BinanceSpot,
    Exchange::Kraken,
    Exchange::OkxSpot,
    Exchange::Kucoin,
    Exchange::Bitstamp,
    Exchange::Coinbase,
    Exchange::BybitSpot,
    Exchange::BitgetSpot,
];

/// ₿ The exchanges in the order their BTC pairs are shown (Jee's mock-up, 23 Nov): Bitfinex first, then Binance,
/// Kraken, OKX, KuCoin, Bitstamp, then the others in the app's usual order.
pub(crate) fn ordre_btc() -> Vec<Exchange> {
    let d = [
        Exchange::BitfinexSpot,
        Exchange::Bitfinex,
        Exchange::BinanceSpot,
        Exchange::Kraken,
        Exchange::OkxSpot,
        Exchange::Kucoin,
        Exchange::Bitstamp,
    ];
    d.into_iter().chain(Exchange::ALL.into_iter().filter(|e| !d.contains(e))).collect()
}

/// ₿ The real markets of `asset` against BTC in the catalogue (`only`: that exchange, its spot first), in
/// [`ordre_btc`] order, as search rows (« BINANCE:XRPBTC », « Binance »).
pub(crate) fn paires_btc(st: &Store, asset: &str, only: Option<Exchange>, have: &std::collections::HashSet<String>) -> Vec<Found> {
    let exchanges: Vec<Exchange> = match only {
        Some(e) => [e.spot_of(), Some(e)].into_iter().flatten().collect(),
        None => ordre_btc(),
    };
    let mut found = vec![];
    for ex in exchanges {
        let q = Search {
            query: asset,
            exchange: Some(ex.id()),
            group: None,
            limit: 60,
        };
        for i in st.search(&q).unwrap_or_default() {
            if !(i.active && i.base.eq_ignore_ascii_case(asset) && i.quote.eq_ignore_ascii_case("BTC")) {
                continue;
            }
            if found.iter().any(|f: &Found| f.id == i.id()) {
                continue;
            }
            found.push(Found {
                have: have.contains(&i.id()),
                id: i.id(),
                name: argos_data::tv::name(ex, &argos_data::paire::base(&i.base, "BTC")),
                detail: ex.name().to_string(),
                group: PAIRES_BTC.into(),
            });
        }
    }
    found
}

/// ₿ Group of the real pairs against BTC in the search.
pub(crate) const PAIRES_BTC: &str = "Paires contre BTC";

/// 📈 Valeur d'une série CoinMarketCap pour la watchlist : le dernier relevé, et celui d'il y a ~24 h (variation).
fn cap_mark(hist: &[argos_data::cmcap::CapPoint], series: &str) -> (String, f64) {
    let Some((t, v)) = hist.iter().rev().find_map(|p| p.value(series).map(|v| (p.t, v))) else {
        return (String::new(), 0.0);
    };
    let prev = hist.iter().rev().find(|p| p.t <= t - 86_400).and_then(|p| p.value(series)).unwrap_or(0.0);
    let mark = if series.ends_with(".D") { format!("{v:.2}") } else { format!("{:.0}", v) };
    (mark, prev)
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use argos_core::MarketKind;

    fn inst(ex: &str, symbol: &str, base: &str, quote: &str, kind: MarketKind) -> Instrument {
        Instrument {
            exchange: ex.into(),
            symbol: symbol.into(),
            base: base.into(),
            quote: quote.into(),
            kind,
            group: "Crypto".into(),
            price_decimals: 2,
            size_decimals: 3,
            growth: false,
            active: true,
        }
    }

    fn tv_results_t(st: &Store, q: &str, have: &std::collections::HashSet<String>) -> Result<Option<Vec<Found>>, String> {
        tv_results(st, q, have, true)
    }

    #[test]
    fn series_coinmarketcap_dans_la_recherche() {
        let (st, have) = (store(), Default::default());
        let r = tv_results(&st, "CRYPTOCAP:USDT.D", &have, true).unwrap().unwrap();
        assert_eq!((r[0].id.as_str(), r[0].name.as_str()), ("cap:USDT.D", "CRYPTOCAP:USDT.D"));
        assert_eq!(tv_results(&st, "CRYPTOCAP:", &have, true).unwrap().unwrap().len(), 7);
        let r = tv_results(&st, "total3", &have, false).unwrap().unwrap();
        assert!(
            !r[0].have && r[0].detail.contains("CoinGecko, gratuit"),
            "sans clé : la série s'ajoute quand même (historique gratuit)"
        );
        assert!(tv_results(&st, "CRYPTOCAP:OTHERS", &have, true).unwrap_err().contains("pas encore géré"));
    }

    fn store() -> Store {
        let mut st = Store::in_memory().unwrap();
        st.save_catalog("binance", &[inst("binance", "BTCUSDT", "BTC", "USDT", MarketKind::Perp)], 0)
            .unwrap();
        st.save_catalog("bitstamp", &[inst("bitstamp", "btcusd", "BTC", "USD", MarketKind::Spot)], 0)
            .unwrap();
        st.save_catalog("hl", &[inst("hl", "BTC", "BTC", "USDC", MarketKind::Perp)], 0).unwrap();
        st
    }

    #[test]
    fn recherche_au_format_tradingview() {
        let (mut st, have) = (store(), Default::default());
        let r = tv_results_t(&st, "BINANCE:BTCUSDT.P", &have).unwrap().unwrap();
        assert_eq!((r[0].id.as_str(), r[0].name.as_str()), ("binance:BTCUSDT", "BINANCE:BTCUSDT.P"));
        let r = tv_results_t(&st, "binance:btcusdt", &have).unwrap().unwrap();
        assert!(
            r[0].detail.contains("spot pas encore chargé"),
            "spot demandé mais pas au catalogue : le perpétuel, en le disant"
        );
        st.save_catalog("binance-spot", &[inst("binance-spot", "BTCUSDT", "BTC", "USDT", MarketKind::Spot)], 0)
            .unwrap();
        let r = tv_results_t(&st, "binance:btcusdt", &have).unwrap().unwrap();
        assert_eq!(
            (r.len(), r[0].id.as_str(), r[0].name.as_str()),
            (1, "binance-spot:BTCUSDT", "BINANCE:BTCUSDT"),
            "spot au catalogue : le spot seul"
        );
        let r = tv_results_t(&st, "BITSTAMP:BTCUSD", &have).unwrap().unwrap();
        assert_eq!(r[0].id, "bitstamp:btcusd");
        let r = tv_results_t(&st, "BTCUSDT.P", &have).unwrap().unwrap();
        assert_eq!(r.len(), 1, "sans préfixe : tous les exchanges qui ont ce perpétuel en USDT");
        let r = tv_results_t(&st, "HL:BTC", &have).unwrap().unwrap();
        assert_eq!(r[0].id, "hl:BTC");
        assert!(tv_results_t(&st, "btc", &have).unwrap().is_none(), "recherche normale");
        assert!(tv_results_t(&st, "HTX:BTCUSDT", &have).unwrap_err().contains("pas encore géré"));
        assert!(tv_results_t(&st, "BYBIT:BTCUSDT.P", &have).unwrap_err().contains("introuvable"));
    }

    /// ₿ « xrpbtc », « xrp/btc », « xrp-btc », « xrp btc » : the real XRP pairs against BTC of each exchange, then the
    /// computed pair of before, last; a market against USDT never comes in.
    #[test]
    fn vraies_paires_contre_btc() {
        let (mut st, have) = (store(), Default::default());
        st.save_catalog(
            "binance-spot",
            &[
                inst("binance-spot", "XRPBTC", "XRP", "BTC", MarketKind::Spot),
                inst("binance-spot", "XRPUSDT", "XRP", "USDT", MarketKind::Spot),
            ],
            0,
        )
        .unwrap();
        st.save_catalog("kraken", &[inst("kraken", "XRP/BTC", "XRP", "BTC", MarketKind::Spot)], 0)
            .unwrap();
        st.save_catalog("bitfinex-spot", &[inst("bitfinex-spot", "XRPBTC", "XRP", "BTC", MarketKind::Spot)], 0)
            .unwrap();
        for q in ["xrpbtc", "xrp/btc", "xrp-btc", "xrp btc", "XRPBTC"] {
            let r = tv_results_t(&st, q, &have).unwrap().unwrap();
            let ids: Vec<&str> = r.iter().map(|f| f.id.as_str()).collect();
            assert_eq!(
                ids,
                ["bitfinex-spot:XRPBTC", "binance-spot:XRPBTC", "kraken:XRP/BTC", "ratio:binance:XRP:BTC"],
                "{q}: Bitfinex first, the computed pair last"
            );
            assert_eq!((r[0].name.as_str(), r[0].detail.as_str()), ("BITFINEX:XRPBTC", "Bitfinex Spot"));
            assert_eq!(r[1].name, "BINANCE:XRPBTC");
            assert_eq!(r[0].group, PAIRES_BTC);
        }
        let r = tv_results_t(&st, "BINANCE:XRPBTC", &have).unwrap().unwrap();
        assert_eq!(
            r.iter().map(|f| f.id.as_str()).collect::<Vec<_>>(),
            ["binance-spot:XRPBTC", "ratio:binance:XRP:BTC"]
        );
    }

    #[test]
    fn paires_calculees_dans_la_recherche() {
        let (st, have) = (store(), Default::default());
        for q in ["XRPBTC", "BINANCE:XRPBTC", "XRPUSDT/BTCUSDT"] {
            let r = tv_results(&st, q, &have, true).unwrap().unwrap();
            assert_eq!(r[0].id, "ratio:binance:XRP:BTC", "{q}");
            assert_eq!(r[0].name, "XRPBTC (calculée, approximative)");
        }
        assert_eq!(tv_results_t(&st, "BYBIT:HYPEBTC", &have).unwrap().unwrap()[0].id, "ratio:bybit:HYPE:BTC");
        assert!(
            tv_results_t(&st, "BINANCE:XRPUSDT.P/BYBIT:BTCUSDT.P", &have).is_err(),
            "deux exchanges : refusé"
        );
    }
}

/// ↕️ Menu de la barre d'icônes : 0 = ordre par défaut, 100 + k = masquer `Tool::ALL[k]`, 200 + k = le réafficher.
const ICON_RESET: i64 = 0;
const ICON_HIDE: i64 = 100;
const ICON_SHOW: i64 = 200;
/// 🖥 Colonne d'interface-2 : 300 + k = ⭐ / ☆ `Tool::ALL[k]` (dans la colonne ou non) ; 299 = la barre de la maquette.
const ICON_ETOILE: i64 = 300;
const ICON_MAQUETTE: i64 = 299;

impl App {
    pub(crate) fn icon_bar_menu(&self, t: Option<wl::Tool>) -> Vec<argos_render::listmenu::MenuGroup> {
        use argos_render::listmenu::{MenuGroup, MenuItem};
        let idx = |t: wl::Tool| wl::Tool::ALL.iter().position(|x| *x == t).unwrap_or(0) as i64;
        let item = |id: i64, label: String| MenuItem {
            id,
            label,
            enabled: true,
            ..Default::default()
        };
        // 🖥 colonne d'interface-2 : une étoile par outil (⭐ = dans la colonne), comme demandé par Jee
        if let Some(col) = &self.watch.col_outils {
            let mut items = vec![item(ICON_MAQUETTE, "↺ Remettre la barre de la maquette".into())];
            for t in wl::Tool::ALL {
                let on = col.contains(&t);
                items.push(MenuItem {
                    id: ICON_ETOILE + idx(t),
                    label: format!("{} {}", t.emoji(), t.name()),
                    enabled: true,
                    star: Some(on),
                    ..Default::default()
                });
            }
            items.push(MenuItem {
                id: -1,
                label: "⭐ = dans la barre · glisse une icône pour la déplacer".into(),
                enabled: false,
                ..Default::default()
            });
            return vec![MenuGroup {
                title: "PERSONNALISER LA BARRE".into(),
                items,
                strip: false,
            }];
        }
        let mut items = vec![item(ICON_RESET, "↺ Remettre l'ordre par défaut (et tout afficher)".into())];
        // (🎯 Stratégie comes and goes with the chart's strategy: not hidden here)
        if let Some(t) = t.filter(|t| !matches!(t, wl::Tool::Watchlist | wl::Tool::Strategie)) {
            items.push(item(ICON_HIDE + idx(t), format!("🙈 Masquer « {} »", t.name())));
        }
        for h in &self.watch.prefs.hidden_tools {
            items.push(item(ICON_SHOW + idx(*h), format!("👁 Afficher « {} »", h.name())));
        }
        items.push(MenuItem {
            id: -1,
            label: "Astuce : glisse un bouton pour le déplacer".into(),
            enabled: false,
            ..Default::default()
        });
        vec![MenuGroup {
            title: "PERSONNALISER LA BARRE".into(),
            items,
            strip: false,
        }]
    }

    pub(crate) fn icon_bar_pick(&mut self, id: i64) {
        let tool = |k: i64| wl::Tool::ALL.get(k as usize).copied();
        match id {
            // 🖥 colonne d'interface-2 (interface-2.json ; watchlist.json n'est pas touché)
            ICON_MAQUETTE if self.watch.col_outils.is_some() => {
                self.watch.col_outils = Some(wl::Tool::MAQUETTE.to_vec());
                self.colonne_garder();
                self.banner = Some("↺ Barre de la maquette".into());
                self.ui.list = None;
                return;
            }
            k if (ICON_ETOILE..ICON_ETOILE + 50).contains(&k) && self.watch.col_outils.is_some() => {
                if let (Some(t), Some(col)) = (tool(k - ICON_ETOILE), self.watch.col_outils.as_mut()) {
                    if let Some(p) = col.iter().position(|x| *x == t) {
                        col.remove(p);
                        if self.watch.prefs.open && self.watch.prefs.panel == t {
                            self.watch.prefs.open = false;
                        }
                    } else {
                        col.push(t);
                    }
                    self.colonne_garder();
                }
                return; // le menu reste ouvert (cocher plusieurs étoiles d'affilée) : list_click le refait
            }
            ICON_RESET => {
                self.watch.reset_tools();
                self.banner = Some("↺ Barre d'icônes : ordre par défaut".into());
            }
            k if (ICON_SHOW..ICON_SHOW + 50).contains(&k) => {
                if let Some(t) = tool(k - ICON_SHOW) {
                    self.watch.set_tool_hidden(t, false);
                }
            }
            k if (ICON_HIDE..ICON_HIDE + 50).contains(&k) => {
                if let Some(t) = tool(k - ICON_HIDE) {
                    self.watch.set_tool_hidden(t, true);
                    self.banner = Some(format!("🙈 « {} » masqué : clic droit sur la barre pour le réafficher", t.name()));
                }
            }
            _ => return,
        }
        self.ui.list = None;
        self.save_watch();
    }
}

/// 🌐 Nom de la vue « Tous les marchés » (menu des listes, pilule du cell, en-tête).
pub(crate) const TOUS: &str = "🌐 Tous les marchés";
/// Nom de la liste où ☆ ajoute un symbole.
pub(crate) const FAVORIS: &str = "Mes favoris";

/// 🌐 « Tous les marchés » de la watchlist : tous les perps d'une plateforme, triés (choix de Jee, 20 nov. 2026).
impl App {
    /// La plateforme affichée (Hyperliquid par défaut).
    pub(crate) fn tous_bourse(&self) -> Exchange {
        Exchange::parse(&self.watch.prefs.tous_bourse)
            .filter(|e| argos_data::tous::BOURSES.contains(e))
            .unwrap_or(Exchange::Hyperliquid)
    }

    /// Les lignes de la vue : le dernier relevé de la plateforme affichée, avec les market caps, dans l'ordre de
    /// « Trier par » (données absentes en bas). En-tête : le nombre de marchés, ou ce qui ne va pas.
    pub(crate) fn vue_tous(&mut self) -> Vec<wl::Section> {
        let ex = self.tous_bourse();
        let tri = argos_data::tous::Tri::parse(&self.watch.prefs.tous_tri);
        (self.watch.title, self.watch.editable) = (TOUS.to_string(), false);
        let mut lignes = match &self.feeds.tous {
            Some((e, Ok(l))) if *e == ex => l.clone(),
            Some((e, Err(msg))) if *e == ex => {
                self.watch.note = format!("{} : {msg}", ex.name());
                return vec![];
            }
            _ => {
                self.watch.note = format!("{} : chargement…", ex.name());
                return vec![];
            }
        };
        // 📈 with the CMC key: CoinMarketCap's caps (last reading, top 80) for the coins CoinGecko doesn't give
        match self.cmc_caps() {
            Some(cmc) => argos_data::tous::avec_caps(&mut lignes, &argos_data::cmcap::caps_avec_cmc(&self.feeds.tous_caps, cmc)),
            None => argos_data::tous::avec_caps(&mut lignes, &self.feeds.tous_caps),
        }
        argos_data::tous::trier(&mut lignes, tri);
        self.watch.note = format!("{} · {} marchés", ex.name(), lignes.len());
        let rows = lignes
            .iter()
            .map(|l| {
                // (Hyperliquid : le nom du marché seul, comme la liste Hyperliquid et la chart ouverte)
                let id = if ex == Exchange::Hyperliquid {
                    l.symbol.clone()
                } else {
                    format!("{}:{}", ex.id(), l.symbol)
                };
                let mut row = wl::Row::new(&id, &l.base, &prix_tous(l.price), l.prev);
                row.note = note_tous(l, tri);
                row
            })
            .collect();
        vec![wl::Section {
            name: ex.name().to_string(),
            rows,
        }]
    }

    /// Menu d'une pilule : les 6 plateformes, ou les tris.
    pub(crate) fn tous_menu(&self, kind: ListKind) -> Vec<MenuGroup> {
        let items = if kind == ListKind::TousBourse {
            let cur = self.tous_bourse();
            argos_data::tous::BOURSES
                .iter()
                .enumerate()
                .map(|(k, e)| MenuItem {
                    id: ID_TOUS_BOURSE + k as i64,
                    label: e.name().into(),
                    enabled: true,
                    checked: *e == cur,
                    color: Some(ex_dot(*e)),
                    ..Default::default()
                })
                .collect()
        } else {
            let cur = argos_data::tous::Tri::parse(&self.watch.prefs.tous_tri);
            argos_data::tous::Tri::ALL
                .iter()
                .enumerate()
                .map(|(k, t)| MenuItem {
                    id: ID_TOUS_TRI + k as i64,
                    label: t.label().into(),
                    enabled: true,
                    checked: *t == cur,
                    ..Default::default()
                })
                .collect()
        };
        vec![MenuGroup {
            title: String::new(),
            items,
            strip: false,
        }]
    }

    /// Choix dans le menu d'une pilule : la vue repart en haut ; une nouvelle plateforme est lue tout de suite.
    pub(crate) fn tous_pick(&mut self, id: i64) {
        let k = |base: i64| usize::try_from(id - base).ok();
        if let Some(e) = k(ID_TOUS_BOURSE).and_then(|k| argos_data::tous::BOURSES.get(k)).filter(|_| id < ID_TOUS_TRI) {
            self.watch.prefs.tous_bourse = e.id().to_string();
        } else if let Some(t) = k(ID_TOUS_TRI).and_then(|k| argos_data::tous::Tri::ALL.get(k)) {
            self.watch.prefs.tous_tri = t.key().to_string();
        }
        self.watch.haut_de_liste();
        self.save_watch();
        self.refresh_watch();
        self.redraw();
    }

    /// La liste « Mes favoris » (créée si elle n'existe plus).
    pub(crate) fn favoris_id(&self) -> Option<i64> {
        self.lists.iter().find(|l| l.name == FAVORIS).map(|l| l.id)
    }

    /// ☆ d'une ligne : le symbole va dans « Mes favoris » (à la fin de sa dernière section).
    pub(crate) fn ajouter_favori(&mut self, coin: &str) {
        // une ligne Hyperliquid (« BTC ») va dans la liste comme « hl:BTC »
        let coin = match coin.split_once(':') {
            Some((e, _)) if Exchange::parse(e).is_some() => coin.to_string(),
            _ => format!("{}:{coin}", Exchange::Hyperliquid.id()),
        };
        let coin = coin.as_str();
        if self.favoris_id().is_none() {
            if let Some(st) = self.store.as_mut() {
                let _ = st.create_list(FAVORIS);
            }
            self.load_lists();
        }
        let (Some(list), Some(st)) = (self.favoris_id(), self.store.as_mut()) else {
            return;
        };
        let nom = coin.split_once(':').map_or(coin, |(_, s)| s).to_string();
        if st.items(list).unwrap_or_default().iter().any(|(_, c)| c == coin) {
            self.banner = Some(format!("★ {nom} est déjà dans {FAVORIS}"));
            return;
        }
        let section = st.sections(list).unwrap_or_default().pop().unwrap_or_default();
        self.banner = Some(match st.add_item(list, &section, coin) {
            Ok(_) => format!("★ {nom} ajouté à {FAVORIS}"),
            Err(e) => format!("⚠️ {e}"),
        });
        self.load_lists();
        self.refresh_watch();
    }
}

/// Prix d'une ligne de « Tous les marchés », avec 4 à 5 chiffres significatifs.
pub(crate) fn prix_tous(p: f64) -> String {
    let dec = (3.0 - p.abs().max(1e-12).log10().floor()).clamp(1.0, 10.0) as usize;
    format!("{p:.dec$}")
}

/// Montant en dollars écrit court, à la française (« 1,2 Md$ »).
pub(crate) fn dollars_court(v: f64) -> String {
    let (x, u) = match v.abs() {
        a if a >= 1e12 => (v / 1e12, " T$"),
        a if a >= 1e9 => (v / 1e9, " Md$"),
        a if a >= 1e6 => (v / 1e6, " M$"),
        a if a >= 1e3 => (v / 1e3, " k$"),
        _ => (v, " $"),
    };
    format!("{x:.1}{u}").replace('.', ",")
}

/// La note grise d'une ligne : le chiffre du tri choisi (le volume pour les tris par variation et par nom).
pub(crate) fn note_tous(l: &argos_data::tous::Ligne, tri: argos_data::tous::Tri) -> String {
    use argos_data::tous::Tri;
    let (nom, v) = match tri {
        Tri::Volatilite => {
            return l
                .volatilite()
                .map_or("Volat. —".into(), |v| format!("Volat. {}", format!("{v:.1} %").replace('.', ",")))
        }
        Tri::MarketCap => ("Cap", l.cap),
        Tri::OpenInterest => ("OI", l.oi_usd),
        Tri::Volume | Tri::Hausse | Tri::Baisse | Tri::Nom => ("Vol", l.vol_usd),
    };
    v.map_or(format!("{nom} —"), |v| format!("{nom} {}", dollars_court(v)))
}

/// 🔎 Volet « Symboles » de la maquette (🖥 PC), dans le volet flottant.
impl App {
    /// Symbole ⌄ : ouvre (ou ferme) le volet Symboles.
    pub(crate) fn symboles_volet(&mut self) {
        let ouvert = self.ui.symboles.is_some() && self.watch.prefs.open && self.watch.prefs.panel == wl::Tool::Watchlist;
        if ouvert {
            self.ui.symboles = None;
            self.watch.prefs.open = false;
        } else {
            self.ui.symboles = Some(argos_render::symboles::Symboles::default());
            (self.watch.prefs.open, self.watch.prefs.panel) = (true, wl::Tool::Watchlist);
        }
        self.save_watch();
        self.redraw();
    }

    /// Place du volet Symboles (fenêtre) ; `None` s'il n'est pas ouvert.
    pub(crate) fn sym_rect(&self) -> Option<Rect> {
        self.ui.symboles.as_ref()?;
        self.volet_rect().filter(|_| self.watch.prefs.panel == wl::Tool::Watchlist)
    }

    /// Ce que le volet montre : la fiche du symbole ouvert, la watchlist, les récents.
    pub(crate) fn sym_donnees(&self) -> argos_render::symboles::Donnees {
        use argos_render::symboles::{Donnees, Ligne};
        // (💲 the price written as in the watchlist: « 61 373,0 », not the exchange's « 61373.0 »)
        let ligne = |r: &wl::Row| Ligne {
            coin: r.coin.clone(),
            label: r.label.clone(),
            prix: r.prix_affiche(),
            chg: r.variation(),
            fav: self.watch.flag(&r.coin).is_some(),
        };
        let lignes: Vec<Ligne> = self.watch.sections.iter().flat_map(|s| s.rows.iter()).map(ligne).collect();
        let recents = self
            .ui
            .sym_recents
            .iter()
            .map(|c| {
                lignes.iter().find(|l| l.coin == *c).cloned().unwrap_or(Ligne {
                    coin: c.clone(),
                    label: c.clone(),
                    ..Default::default()
                })
            })
            .collect();
        let detail = self
            .chart
            .bars
            .last()
            .map(|b| format!("Prix {} · {}", argos_render::viewport::format_price(b.close, 2), self.chart.interval.short()))
            .unwrap_or_default();
        // the header says the list's exchange (not the chart's), nothing when the list mixes exchanges
        let bourses: std::collections::BTreeSet<&str> = lignes
            .iter()
            .map(|l| match l.coin.split_once(':') {
                Some((e, _)) if Exchange::parse(e).is_some() => Exchange::parse(e).map_or("", |e| e.name()),
                // OANDA, computed, CRYPTOCAP, DEX and ratio markets: not one of the exchanges
                Some((e, _)) if ["oanda", "synth", "cap", "dex", "ratio"].contains(&e) => "",
                // « BTC », « xyz:NVDA » (HIP-3): Hyperliquid
                _ => Exchange::Hyperliquid.name(),
            })
            .collect();
        let bourse = match bourses.into_iter().collect::<Vec<_>>()[..] {
            [b] => b.to_string(),
            _ => String::new(),
        };
        Donnees {
            titre: self.symbol_label(),
            source: self.top.source.clone(),
            detail,
            bourse,
            lignes,
            recents,
        }
    }

    /// Clic dans le volet Symboles (position de la fenêtre).
    pub(crate) fn sym_click(&mut self) {
        use argos_render::symboles::SymClick;
        let Some(v) = self.sym_rect() else { return };
        let (x, y) = (self.pointer.win_mouse.0 - v.x, self.pointer.win_mouse.1 - v.y);
        let d = self.sym_donnees();
        let Some(p) = self.ui.symboles.as_mut() else { return };
        match p.click(x, y, v.w, v.h, &d) {
            SymClick::None | SymClick::Changed => {}
            SymClick::Fermer => {
                self.ui.symboles = None;
                self.watch.prefs.open = false;
                self.save_watch();
            }
            SymClick::Ouvrir(c) => return self.open_coin(c),
            // ☆ : le drapeau de la watchlist (1re couleur) ; encore = l'enlever
            SymClick::Favori(c) => {
                let had = self.watch.flag(&c).is_some();
                self.watch.prefs.flags.retain(|(f, _)| *f != c);
                if !had {
                    self.watch.prefs.flags.push((c, 0));
                }
                self.save_watch();
            }
            // ⋯ Toutes les options : la watchlist d'avant (listes, sections, ＋), dans le même volet
            SymClick::Options => self.ui.symboles = None,
            SymClick::Bientot(m) => self.banner = Some(m.to_string()),
        }
        self.redraw();
    }
}
