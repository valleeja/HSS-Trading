//! 📅 Calendrier économique et annonces sur la chart.

use crate::*;

impl App {
    /// 📊 Tickers 24 h des trois exchanges crypto (Binance, OKX, Hyperliquid) : Hyperliquid vient de son flux, Binance
    /// et OKX sont relus au plus une fois par minute (une requête chacun pour tous leurs marchés).
    pub(crate) fn all_tickers(&self) -> Vec<argos_data::tickers::Ticker> {
        use argos_data::tickers::Ticker;
        let mut all: Vec<Ticker> = self
            .feeds
            .quotes
            .values()
            .filter(|q| !q.coin.contains(':'))
            .map(|q| Ticker {
                ex: Exchange::Hyperliquid,
                base: q.coin.clone(),
                price: q.mark.parse().unwrap_or(0.0),
                prev: q.prev,
                vol_usd: q.volume,
            })
            .collect();
        for (_, list) in self.feeds.tickers.values() {
            all.extend(list.iter().cloned());
        }
        all
    }

    /// 📊 Relit les tickers de Binance et d'OKX s'ils ont plus d'une minute.
    pub(crate) fn refresh_tickers(&mut self) {
        let Some(proxy) = self.proxy.clone() else { return };
        for ex in [Exchange::Binance, Exchange::Okx] {
            let fresh = self.feeds.tickers.get(&ex).is_some_and(|(t, _)| t.elapsed().as_secs() < 60);
            if fresh || self.feeds.tickers_busy.contains(&ex) {
                continue;
            }
            self.feeds.tickers_busy.insert(ex);
            let proxy = proxy.clone();
            argos_data::rt::spawn(move || async move {
                let res = argos_data::tickers::fetch(&argos_data::rt::Client::default(), ex)
                    .await
                    .map_err(|e| e.to_string());
                let _ = proxy.send_event(UserEvent::Tickers(ex, res));
            });
        }
    }

    /// 📡 Symboles du scanner : ceux de la watchlist sur Binance, OKX et Hyperliquid (20 au plus), sinon la liste du
    /// scanner ; chacun sur l'exchange où il est le plus liquide (les autres sont notés).
    pub(crate) fn scan_coins(&self) -> Vec<(Exchange, String, Vec<Exchange>)> {
        const CRYPTO: [Exchange; 3] = [Exchange::Hyperliquid, Exchange::Binance, Exchange::Okx];
        let best = argos_data::tickers::best_by_base(&self.all_tickers());
        let pick = |ex: Exchange, base: String| -> (Exchange, String, Vec<Exchange>) {
            let key = argos_data::tickers::norm(&base);
            match best.iter().find(|(t, _)| argos_data::tickers::norm(&t.base) == key) {
                Some((t, others)) => (t.ex, t.base.clone(), others.clone()),
                None => (ex, base, vec![]),
            }
        };
        let mut out: Vec<(Exchange, String, Vec<Exchange>)> = vec![];
        for r in self.watch.sections.iter().flat_map(|s| &s.rows) {
            let ex = match r.coin.split_once(':') {
                Some((e, _)) => match Exchange::parse(e) {
                    Some(x) => x,
                    None => continue, // HIP-3 (xyz:NVDA…) : pas une crypto
                },
                None => Exchange::Hyperliquid,
            };
            if !CRYPTO.contains(&ex) || out.len() >= 20 {
                continue;
            }
            let base = self.base_of(&r.coin);
            let key = argos_data::tickers::norm(&base);
            if !base.is_empty() && !out.iter().any(|(_, b, _)| argos_data::tickers::norm(b) == key) {
                out.push(pick(ex, base));
            }
        }
        if out.is_empty() {
            out = argos_scanner::DEFAULT_COINS
                .iter()
                .map(|c| pick(Exchange::Hyperliquid, c.to_string()))
                .collect();
        }
        out
    }

    /// 📡 Lance un scan : bougies Daily et 4H de chaque symbole sur son exchange (une requête à la fois, sans presser
    /// les API), puis la stratégie double structure d'argos-scanner (calcul pur, le même que l'outil en ligne de
    /// commande).
    fn start_scan(&mut self, proxy: EventLoopProxy<UserEvent>) {
        use argos_core::feed::Feed;
        use argos_core::Timeframe;
        self.refresh_tickers();
        let coins = self.scan_coins();
        argos_data::rt::spawn(move || async move {
            let (mut lines, mut missing) = (vec![], vec![]);
            let candles = |bars: Vec<argos_core::Bar>| -> Vec<argos_scanner::candle::Candle> {
                bars.iter()
                    .map(|b| argos_scanner::candle::Candle {
                        t: b.time * 1000,
                        o: b.open,
                        h: b.high,
                        l: b.low,
                        c: b.close,
                    })
                    .collect()
            };
            for (ex, coin, others) in coins {
                let feed = argos_data::ExchangeFeed::new(ex, argos_data::rt::Client::default());
                let d = feed.candles(&coin, Timeframe::D1, 200).await;
                argos_data::rt::sleep(std::time::Duration::from_millis(250)).await;
                let h4 = feed.candles(&coin, Timeframe::H4, 300).await;
                argos_data::rt::sleep(std::time::Duration::from_millis(250)).await;
                let (Ok(d), Ok(h4)) = (d, h4) else {
                    missing.push(coin);
                    continue;
                };
                match argos_scanner::scan::analyse(&coin, &candles(d), &candles(h4), 3) {
                    Some(r) => {
                        use argos_scanner::scan::Verdict;
                        let dir = match r.verdict {
                            Verdict::Long => 1,
                            Verdict::Short => -1,
                            Verdict::Watch => 2,
                            Verdict::None => 0,
                        };
                        lines.push(argos_render::dock::ScanLine {
                            coin,
                            dir,
                            rr: r.plan.and_then(|p| p.rr),
                            ex: ex.name().to_string(),
                            also: others.iter().map(|e| e.name().to_string()).collect(),
                        });
                    }
                    None => missing.push(coin),
                }
            }
            let _ = proxy.send_event(UserEvent::Scan(lines, missing));
        });
    }

    /// 📅 📰 Recharge l'Agenda (aux 10 min ; aussi pour le blocage news du gardien PropFirm) et les nouvelles (chaque minute, en direct) quand leur panneau est ouvert ; 🐋 fait
    /// tourner les flux des baleines (Hyperliquid, Binance) tant que leur panneau est ouvert ou que les bulles sont sur
    /// la chart, pour la pièce du graphique.
    pub(crate) fn refresh_eco(&mut self) {
        self.liq_refresh(); // 💥 carte des liquidations (si l'indicateur est allumé)
        self.hl_refresh(); // 🧭 📶 Explorer and Stats Hyperliquid (open), ⭐ followed addresses
        self.murs_refresh(); // 🧱 Argos Murs : carnets aux 10 s (si l'indicateur est allumé, crypto)
        self.carnets_refresh(); // 📖 🔥 carnets choisis de la heatmap et du carnet d'ordres (aux 5 s, sauf Hyperliquid seul)
        self.carnet_suivre(); // 📖 le panneau Carnet / Delta / DOM refait chaque seconde (et à chaque carnet)
        self.orderflow_suivre(); // 🧮 orderflow réel du marché Hyperliquid de la chart
        #[cfg(not(target_arch = "wasm32"))]
        self.journal_suivre(); // 🔐 journal changé par le cell (serveur local) : relu
        self.watch.whales.now_ms = now_s() * 1000; // 🐋 la période se compte depuis maintenant
        self.watch.whales.tz_offset_s = self.tz_offset_s();
        let Some(proxy) = self.proxy.clone() else { return };
        let stale = |at: Option<Instant>, s: u64| at.is_none_or(|t| t.elapsed().as_secs() >= s);
        // ouvert dans le panneau de droite, ou module affiché dans la colonne 🧩
        use argos_render::dock::Module;
        let in_dock: Vec<Module> = if self.dock.rect.is_some() {
            self.watch.dock.shown().iter().map(|i| i.module).collect()
        } else {
            vec![]
        };
        let docked = |m: Module| in_dock.contains(&m);
        let shown = |t: wl::Tool, m: Module| (self.watch.prefs.open && self.watch.prefs.panel == t) || docked(m);
        // 📅 l'Agenda se charge aussi pour les annonces sur la chart
        let (agenda, news, whales_on) = (
            shown(wl::Tool::Agenda, Module::Agenda)
                || self.chart.settings.econ_events
                || (self.watch.panels.trading.guardian && self.watch.panels.journal.cfg.news)
                || self.ui.accueil.is_some(), // ⌂ carte Calendrier de l'Accueil
            shown(wl::Tool::News, Module::News),
            shown(wl::Tool::Whales, Module::Whales),
        );
        // 📰 ⚙ Actualités: the sources, the language and « ⚡ sur la chart » kept in the settings
        (self.watch.eco.sources_off, self.watch.eco.langue, self.watch.eco.eclairs) = (
            self.watch.prefs.news_sources_off.clone(),
            self.watch.prefs.news_langue.min(2),
            self.chart.settings.econ_news,
        );
        // ⚡ on the chart (crypto chart): the news read again every 5 min even with the panel closed (every minute while
        // it is open)
        let eclairs = self.chart.settings.econ_news && self.chart_coin().is_some();
        let news_every = if news { 60 } else { 300 };
        // 📅 « Réel » of the day's USD releases (BLS, Fed), from their time to +60 min
        if !self.feeds.reel_busy && !self.feeds.reel.due(&self.watch.eco.agenda, now_s()).is_empty() {
            self.feeds.reel_busy = true;
            let (mut reel, mut list, proxy) = (self.feeds.reel.clone(), self.watch.eco.agenda.clone(), proxy.clone());
            argos_data::rt::spawn(move || async move {
                if argos_data::direct() {
                    reel.read(&argos_data::rt::Client::default(), &mut list, now_s()).await;
                } else {
                    // (in the browser the /api/calendar relay fills it: the calendar is read again)
                    for e in reel.due(&list.clone(), now_s()) {
                        reel.last.insert((e.title.clone(), e.ts), now_s());
                    }
                    if let Ok(l) = argos_data::eco::calendar().await {
                        list = l;
                    }
                }
                let _ = proxy.send_event(UserEvent::Reel(reel, list));
            });
        }
        // 📈 the chart's reaction under the Calendrier's past releases
        if agenda && shown(wl::Tool::Agenda, Module::Agenda) {
            self.cal_minutes();
        }
        match (agenda, news || eclairs) {
            (true, _) if stale(self.feeds.eco_at.0, 600) => {
                self.feeds.eco_at.0 = Some(Instant::now());
                let proxy = proxy.clone();
                argos_data::rt::spawn(move || async move {
                    let _ = proxy.send_event(UserEvent::Agenda(argos_data::eco::calendar().await.map_err(String::from)));
                });
            }
            (_, true) if stale(self.feeds.eco_at.1, news_every) => {
                self.feeds.eco_at.1 = Some(Instant::now());
                let proxy = proxy.clone();
                argos_data::rt::spawn(move || async move {
                    let _ = proxy.send_event(UserEvent::News(argos_data::eco::news().await.map_err(String::from)));
                });
            }
            _ => {}
        }
        use argos_data::whales;
        let base = match &self.chart.source {
            // ₿ a pair against BTC: not Hyperliquid's coin in dollars (its book, trades and funding); Bitfinex: its
            // own book (`bitfinex_carnet`)
            Source::Live { base, exchange } if argos_data::paire::contre_btc(base) || exchange.bitfinex().is_some() => String::new(),
            Source::Live { base, .. } => whales::coin_of(base),
            Source::Oanda { .. } | Source::Ratio { .. } | Source::Synth { .. } | Source::Cap { .. } | Source::Dex { .. } | Source::File(_) => String::new(),
        };
        // 👁 baleine suivie : ses positions, relues aux 30 s tant que le panneau est ouvert
        if let Some(addr) = self.watch.whales.follow.clone().filter(|_| whales_on) {
            if self.feeds.whale_acc_at.as_ref().is_none_or(|(a, t)| *a != addr || t.elapsed().as_secs() >= 30) {
                self.feeds.whale_acc_at = Some((addr.clone(), Instant::now()));
                let proxy = proxy.clone();
                argos_data::rt::spawn(move || async move {
                    let res = whales::account(&addr).await;
                    let _ = proxy.send_event(UserEvent::WhaleAccount(addr, res.map_err(String::from)));
                });
            }
        }
        // 👛 wallet en lecture seule : relu aux 30 s tant que sa carte est dans la colonne ou son menu ouvert
        if docked(Module::Wallet) || matches!(self.ui.list, Some((ListKind::Wallet, _))) {
            self.wallet_refresh(false);
        }
        // 🦎 carte DEX : fiche et derniers trades de la pool affichée, à la minute (GeckoTerminal : 10 appels / min)
        if let Source::Dex { network, pool, .. } = &self.chart.source {
            if docked(Module::Dex) && stale(self.feeds.dex_at, 60) {
                self.feeds.dex_at = Some(Instant::now());
                let (network, pool, key, proxy) = (network.clone(), pool.clone(), self.chart.source.key(), proxy.clone());
                argos_data::rt::spawn(move || async move {
                    let http = argos_data::rt::Client::default();
                    let info = argos_data::dex::pool(&http, &network, &pool).await.ok().map(Box::new);
                    let trades = argos_data::dex::trades(&http, &network, &pool).await.unwrap_or_default();
                    let _ = proxy.send_event(UserEvent::DexPool(key, info, trades));
                });
            }
        }
        // 📡 scanner double structure (Daily + 4H, Hyperliquid) : aux 15 min tant que son module est dans la colonne
        // (d'abord les tickers des trois exchanges, pour scanner chaque symbole là où il est le plus liquide)
        let tickers_ready = docked(Module::Scanner) && {
            self.refresh_tickers();
            self.feeds.tickers_busy.is_empty() && !self.feeds.quotes.is_empty()
        };
        if tickers_ready && !self.feeds.scan_busy && stale(self.feeds.scan_at, 900) {
            self.feeds.scan_at = Some(Instant::now());
            self.feeds.scan_busy = true;
            self.start_scan(proxy.clone());
        }
        // 📖 ⚡ 👁 carnet et trades : le flux tourne tant que leur module (ou Surveillance) est dans la colonne ; 💸 fundings 24 h aux 10 min
        // 🔥 📖 aussi pour la heatmap du carnet et les Bid & Ask (crypto Hyperliquid seulement)
        let wants_book = self.heat_on() || self.chart.settings.labels.bid_ask || self.chart.settings.lines.bid_ask;
        // 🧮 et dès que la chart montre un marché Hyperliquid : ses trades nourrissent l'orderflow réel (même connexion)
        // (📖 its volet too)
        let flow = (docked(Module::Book)
            || self.carnet_volet_ouvert()
            || docked(Module::Trades)
            || docked(Module::Watch)
            || ((wants_book || self.orderflow_cle().is_some()) && !base.is_empty()))
        .then(|| base.clone());
        // heatmap allumée : le carnet regroupé (3 chiffres significatifs en HD, 4 en 4K), large autour du prix
        let sig = self.heat_on().then(|| {
            let v = argos_render::indicators::values(&self.ind.set, argos_render::indicators::Ind::BookHeat);
            if (argos_script::inputs::Inputs {
                params: &argos_render::bookheat::PARAMS,
                vals: &v,
            })
            .pick("res")
                == 1
            {
                4
            } else {
                3
            }
        });
        // 📖 the order-book panel without the heatmap: Hyperliquid's 20 levels grouped near its step (more depth)
        let sig = sig.or_else(|| {
            let px = self.chart.bars.last().map_or(0.0, |b| b.close);
            (self.carnet_montre() && px > 0.0)
                .then(|| argos_core::carnet::sig_hyperliquid(px, self.carnet_pas_choisi(px)))
                .flatten()
        });
        // 🧮 marchés Hyperliquid suivis (chart et ● enregistrés en continu) : leurs trades, sur la même connexion
        let autres: Vec<String> = self.orderflow_pieces(Exchange::Hyperliquid);
        if flow != self.feeds.flow_coin || sig != self.feeds.flow_sig || autres != self.feeds.flow_autres {
            let gen = self.feeds.flow_gen.fetch_add(1, Ordering::SeqCst) + 1;
            // (⚡ the trades list starts again by itself when its exchange or coin changes: `App::tape_ajouter`)
            (self.feeds.flow_coin, self.feeds.book, self.feeds.flow_sig) = (flow.clone(), None, sig);
            if flow.is_some() || !autres.is_empty() {
                let (proxy, current) = (proxy.clone(), self.feeds.flow_gen.clone());
                live::spawn(
                    argos_data::book::sub(flow.as_deref(), sig, &autres),
                    |t| {
                        argos_data::book::parse_book(t)
                            .map(LiveEvent::Book)
                            .or_else(|| Some(whales::parse_all(t)).filter(|l| !l.is_empty()).map(LiveEvent::Tape))
                    },
                    move |ev| current.load(Ordering::SeqCst) == gen && proxy.send_event(UserEvent::Flow(ev)).is_ok(),
                );
            }
            self.feeds.flow_autres = autres;
        }
        if docked(Module::Funding) && !base.is_empty() && self.feeds.funding_at.as_ref().is_none_or(|(c, t)| *c != base || t.elapsed().as_secs() >= 600) {
            self.feeds.funding_at = Some((base.clone(), Instant::now()));
            let (proxy, coin) = (proxy.clone(), base.clone());
            argos_data::rt::spawn(move || async move {
                let res = argos_data::book::funding_history(&coin, now_s() * 1000).await;
                let _ = proxy.send_event(UserEvent::FundingHist(coin, res.map_err(String::from)));
            });
        }
        // ⚡ taux de funding réel du perpétuel de la chart (bourses lues par Argos ; Hyperliquid : avec ses prix), aux
        // 5 min, pour le compte démo : seulement quand le Simulateur est ouvert ou qu'une position perpétuelle l'est
        if let Source::Live { exchange, base: b } = &self.chart.source {
            let (ex, b) = (*exchange, b.clone());
            let veut = self.ui.sim.is_some() || self.demo.positions.iter().any(|p| p.perp.is_some());
            if veut
                && argos_data::perp::url_funding(ex, &b).is_some()
                && self
                    .feeds
                    .funding_demo_at
                    .as_ref()
                    .is_none_or(|(e, c, t)| *e != ex || *c != b || t.elapsed().as_secs() >= 300)
            {
                self.feeds.funding_demo_at = Some((ex, b.clone(), Instant::now()));
                let proxy = proxy.clone();
                argos_data::rt::spawn(move || async move {
                    let t = argos_data::perp::taux_8h(&argos_data::rt::Client::default(), ex, &b).await;
                    let _ = proxy.send_event(UserEvent::FundingDemo(ex, b, t));
                });
            }
        }
        // 🌍 marchés macro aux 60 s ; 💥 liquidations : le flux tourne tant que leur module est dans la colonne
        if (docked(Module::World) || self.ui.accueil.is_some()) && stale(self.feeds.world_at, 60) {
            self.feeds.world_at = Some(Instant::now());
            let proxy = proxy.clone();
            argos_data::rt::spawn(move || async move {
                let _ = proxy.send_event(UserEvent::World(argos_data::world::tiles().await.map_err(String::from)));
            });
        }
        if docked(Module::Liquidations) != self.feeds.liq_on {
            let gen = self.feeds.liq_gen.fetch_add(1, Ordering::SeqCst) + 1;
            self.feeds.liq_on = !self.feeds.liq_on;
            if self.feeds.liq_on {
                // 💥 deux sources : Binance (tous les marchés) et Bybit (cryptos principales)
                let (proxy2, current2) = (proxy.clone(), self.feeds.liq_gen.clone());
                live::spawn(
                    argos_data::liq::bybit_sub(argos_data::liq::BYBIT_COINS),
                    |t| Some(argos_data::liq::parse_bybit(t)).filter(|l| !l.is_empty()).map(LiveEvent::Liqs),
                    move |ev| current2.load(Ordering::SeqCst) == gen && proxy2.send_event(UserEvent::Liq(ev)).is_ok(),
                );
                let (proxy, current) = (proxy.clone(), self.feeds.liq_gen.clone());
                live::spawn(
                    argos_data::liq::sub(),
                    |t| Some(argos_data::liq::parse(t)).filter(|l| !l.is_empty()).map(LiveEvent::Liqs),
                    move |ev| current.load(Ordering::SeqCst) == gen && proxy.send_event(UserEvent::Liq(ev)).is_ok(),
                );
            }
        }
        // 🏦 un flux par exchange coché (filtre du panneau), seulement panneau ou bulles allumés
        let exs = self.watch.whales.exchanges_on();
        let active = whales_on || self.watch.whales.bubbles;
        // 🔝 top 30 de chaque exchange coché (sa liste de tickers, une fois par jour ; d'abord Binance, qui range les autres)
        let missing: Vec<&'static str> = exs.iter().copied().filter(|e| !self.feeds.whale_top.contains_key(*e)).collect();
        let day_old = self.feeds.whale_top_at.is_some_and(|t| t.elapsed().as_secs() >= 86_400);
        if active && !self.feeds.whale_top_busy && (!missing.is_empty() || day_old) {
            self.feeds.whale_top_busy = true;
            if day_old {
                self.feeds.whale_top_at = None;
            }
            let todo: Vec<&'static str> = if day_old { exs.clone() } else { missing };
            let proxy = proxy.clone();
            argos_data::rt::spawn(move || async move {
                let global = argos_data::whaletop::top("Binance", &[]).await.unwrap_or_default();
                let mut out = vec![];
                for e in todo {
                    let list = if e == "Binance" && !global.is_empty() {
                        Ok(global.clone())
                    } else {
                        argos_data::whaletop::top(e, &global).await.map_err(String::from)
                    };
                    out.push((e.to_string(), list));
                }
                let _ = proxy.send_event(UserEvent::WhaleTop(out));
            });
        }
        // la liste de chaque exchange (son top, sinon les 4 pièces de base) + la pièce de la chart, sans doublon
        let coins_for = |e: &str| -> Vec<String> {
            let mut l = self.feeds.whale_top.get(e).cloned().unwrap_or_else(|| whales::coins(""));
            let b = base.to_uppercase();
            if !b.is_empty() && !l.contains(&b) {
                l.push(b);
            }
            l
        };
        let sig: usize = exs.iter().map(|e| self.feeds.whale_top.get(*e).map_or(0, |l| l.len())).sum();
        let want = active.then(|| format!("{base}|{}|{sig}", exs.join(",")));
        if want != self.feeds.whale_coin {
            let gen = self.feeds.whale_gen.fetch_add(1, Ordering::SeqCst) + 1;
            self.feeds.whale_coin = want.clone();
            if want.is_some() {
                self.watch.whales.coin = base.clone();
                // KuCoin : son adresse demande un jeton public (sans clé), redemandé à chaque reconnexion
                if exs.contains(&"KuCoin") {
                    let (proxy, current, coins) = (proxy.clone(), self.feeds.whale_gen.clone(), coins_for("KuCoin"));
                    live::spawn_jeton(
                        move || whales::kucoin_preparer(coins.clone()),
                        |t| Some(whales::parse(t)).filter(|l| !l.is_empty()).map(LiveEvent::Whales),
                        move |ev| current.load(Ordering::SeqCst) == gen && proxy.send_event(UserEvent::Whales(ev)).is_ok(),
                    );
                }
                for sub in exs.iter().filter_map(|e| whales::sub_of(e, &coins_for(e))) {
                    let (proxy, current) = (proxy.clone(), self.feeds.whale_gen.clone());
                    live::spawn(
                        sub,
                        |t| Some(whales::parse(t)).filter(|l| !l.is_empty()).map(LiveEvent::Whales),
                        move |ev| current.load(Ordering::SeqCst) == gen && proxy.send_event(UserEvent::Whales(ev)).is_ok(),
                    );
                }
            }
        }
        // 📊 flux de tous les ordres (bougies Binance : 7 j en 1 h, 24 h en 5 min) : aux 5 min, panneau Baleines ouvert
        if whales_on {
            let coin = self.watch.whales.coin.clone();
            let coin = if coin.is_empty() { "BTC".to_string() } else { coin };
            if coin != self.watch.whales.flow_coin || stale(self.feeds.whale_flow_at, 300) {
                self.feeds.whale_flow_at = Some(Instant::now());
                self.watch.whales.flow_coin = coin.clone();
                let proxy = proxy.clone();
                argos_data::rt::spawn(move || async move {
                    let res = whales::flow(&coin).await;
                    let _ = proxy.send_event(UserEvent::WhaleFlow(coin, res.map_err(String::from)));
                });
            }
        }
    }

    /// 📅 Rangée d'annonces de la chart : annonces de l'Agenda (impact fort et moyen) avec, si demandé, la fenêtre
    /// PropFirm ±2 min des annonces à fort impact (quand le gardien bloque les news), et 📰 les nouvelles du marché.
    pub(crate) fn econ_marks(&self) -> Vec<argos_render::econ::Mark> {
        let st = &self.chart.settings;
        let window = if st.propfirm_windows && self.watch.panels.journal.cfg.news { 120 } else { 0 };
        let mut out: Vec<argos_render::econ::Mark> = if st.econ_events { self.agenda_marks(window) } else { vec![] };
        // ⚡ the news of the chart's crypto, one per candle (the flags keep their places: `econ::spots`)
        if st.econ_news {
            out.extend(self.eclair_marks());
        }
        out
    }

    /// 📅 Annonces de l'Agenda (impact fort et moyen) ; `window` : fenêtre PropFirm des annonces à fort impact (s).
    fn agenda_marks(&self, window: i64) -> Vec<argos_render::econ::Mark> {
        self.watch
            .eco
            .agenda
            .iter()
            .filter_map(|e| {
                let impact = match e.impact.as_str() {
                    "High" => 2,
                    "Medium" => 1,
                    _ => return None,
                };
                let numbers = [("Réel", &e.actual), ("Prévu", &e.forecast), ("Préc.", &e.previous)]
                    .iter()
                    .filter(|(_, v)| !v.is_empty())
                    .map(|(k, v)| format!("{k} {v}"))
                    .collect::<Vec<_>>()
                    .join("   ");
                Some(argos_render::econ::Mark {
                    ts: e.ts,
                    currency: e.country.clone(),
                    impact,
                    title: e.title.clone(),
                    numbers,
                    window_s: if impact >= 2 { window } else { 0 },
                    ..Default::default()
                })
            })
            .collect()
    }
}

impl App {
    /// 🔥 La heatmap du carnet est-elle allumée (indicateur coché) ?
    pub(crate) fn heat_on(&self) -> bool {
        self.ind.on.contains(&argos_render::indicators::Ind::BookHeat)
    }

    /// 📖 Pièce Hyperliquid dont on lit le carnet pour la chart ; `None` : pas de carnet public (OANDA, fichier…).
    pub(crate) fn book_coin(&self) -> Option<String> {
        match &self.chart.source {
            // ₿ a pair against BTC: no other market's book in dollars under its prices in BTC
            Source::Live { base, .. } if argos_data::paire::contre_btc(base) => None,
            Source::Live { base, .. } => Some(argos_data::whales::coin_of(base)).filter(|c| !c.is_empty()),
            _ => None,
        }
    }

    /// 📖 Where the Book module's order book comes from, said as it is: Hyperliquid's own; Hyperliquid's for the same
    /// coin when the chart is on another exchange; or none (the coin is not on Hyperliquid — known once Hyperliquid's
    /// list of markets has arrived — or the market has no public book).
    pub(crate) fn carnet_source(&self) -> argos_render::dock::CarnetSource {
        use argos_render::dock::CarnetSource;
        let Source::Live { exchange, .. } = &self.chart.source else {
            return CarnetSource::Indisponible("Pas de carnet d'ordres public pour ce marché".into());
        };
        // Bitfinex: its own book, read every 5 s
        if let Some(perp) = exchange.bitfinex() {
            // its last reading answered with a refusal: not readable, like the other exchanges out of order
            let refuse = self.feeds.carnets.as_ref().is_some_and(|s| s.at.is_some() && !s.busy && s.ok == 0);
            if refuse && self.feeds.book.is_none() {
                return CarnetSource::Indisponible(format!("Carnet indisponible pour {}", exchange.name()));
            }
            let n = if self.feeds.book.is_some() { 1 } else { 0 };
            return CarnetSource::Choisis(format!("Bitfinex {} {n}/1", if perp { "Perpétuel" } else { "Spot" }));
        }
        if self.carnets_instantanes() {
            if let Some(t) = self.carnets_titre() {
                return CarnetSource::Choisis(t);
            }
        }
        match self.book_coin() {
            _ if *exchange == Exchange::Hyperliquid => CarnetSource::Hyperliquid,
            Some(c) if self.feeds.quotes.is_empty() || self.feeds.quotes.contains_key(&c) => CarnetSource::MemePiece,
            _ => CarnetSource::Indisponible(format!("Carnet indisponible pour {}", exchange.name())),
        }
    }

    /// 🔥 Une photo du carnet entre dans la heatmap (colonne de la bougie en cours) ; l'historique repart de zéro si
    /// le symbole, l'unité de temps ou le regroupement change.
    pub(crate) fn heat_add(&mut self, b: &argos_data::book::Book) {
        if !self.heat_on() {
            self.feeds.heat = None;
            return;
        }
        let (Some(last), Some((bid, ask))) = (self.chart.bars.last(), b.spread().map(|_| (b.bids[0].px, b.asks[0].px))) else {
            return;
        };
        let mid = (bid + ask) / 2.0;
        let v = argos_render::indicators::values(&self.ind.set, argos_render::indicators::Ind::BookHeat);
        let fine = argos_script::inputs::Inputs {
            params: &argos_render::bookheat::PARAMS,
            vals: &v,
        }
        .pick("res")
            == 1;
        let tick = argos_render::bookheat::tick_for(mid, fine);
        let key = format!("{}|{}|{tick}", b.coin, self.chart.interval.secs());
        if self.feeds.heat.as_ref().is_none_or(|h| h.key != key) {
            self.feeds.heat = Some(argos_render::bookheat::BookHeat::new(&key, tick, 2000));
        }
        if let Some(h) = self.feeds.heat.as_mut() {
            h.add(last.time, b.bids.iter().chain(&b.asks).map(|l| (l.px, l.sz)), mid);
        }
    }
}
