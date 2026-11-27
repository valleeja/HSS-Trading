//! 📨 Messages des autres fils (chargement, direct, prix, réponses) et leur traitement.

use super::*;

/// Messages envoyés à la fenêtre par les autres fils (le chargement, le direct).
#[derive(Debug)]
/// Le numéro (`u64`) est la génération du symbole : une réponse d'un symbole qu'on a quitté est ignorée.
pub(crate) enum UserEvent {
    /// Le chargement des bougies est fini (la fenêtre est déjà ouverte).
    Loaded(u64, Loaded),
    /// 📈 1-minute candles of a crypto (Calendrier: price reactions).
    Minutes(String, Vec<Bar>),
    /// 📅 « Réel » read (the readings kept, the week with them).
    Reel(argos_data::reel::Reel, Vec<argos_data::eco::Event>),
    /// 📜 Bougies plus anciennes (dézoom au-delà de ce qui est chargé) : marché, bougies (vide = début de l'historique).
    Older(String, Result<Vec<Bar>, String>),
    /// 🕳️ Trous réparés (marché « symbole|unité ») : bougies retrouvées et trous que l'exchange n'a pas.
    Repaired(String, argos_data::trous::Reparation),
    Live(u64, LiveEvent),
    /// Nouveaux prix de la watchlist : sections de la liste Hyperliquid, et tous les prix (pour les listes à toi).
    Quotes(Vec<hl_watch::Section>, Vec<hl_watch::Quote>),
    /// Prix (dernier + 24 h) de tous les marchés d'un autre exchange.
    Prices(Exchange, Vec<argos_core::feed::Market>),
    /// 🌐 « Tous les marchés » : tous les marchés de la plateforme affichée (ou l'erreur), et les market caps.
    Tous(Exchange, Result<Vec<argos_data::tous::Ligne>, String>),
    TousCaps(std::collections::HashMap<String, f64>),
    /// Catalogue d'un exchange téléchargé (tous ses symboles).
    Catalog(Exchange, Vec<Instrument>),
    /// Logo d'un symbole (PNG de CoinMarketCap).
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    Logo(String, Vec<u8>),
    /// Panneau Marché crypto à jour (CoinGecko, alternative.me).
    Market(Box<argos_data::market::MarketInfo>),
    /// 🪟 Bougies d'une case du split (clé « symbole|secondes »).
    Pane(String, Result<Vec<Bar>, String>),
    /// 🌐 Pack de langue téléchargé (index dans `i18n::CATALOG`, contenu JSON).
    /// Pack de langue téléchargé : (langue, réponse, en silence = mise à jour d'un pack déjà installé).
    LangPack(usize, Result<String, String>, bool),
    /// 😀 Image Twemoji arrivée du serveur (`/emoji/<nom>.svg`) : nom, SVG (la page seulement ; l'ordinateur lit le
    /// dossier tout de suite).
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    Emoji(String, Result<String, String>),
    /// 📋 Texte collé dans la page (navigateur : coller arrive par l'événement de la page).
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    Paste(String),
    /// 👆 Doigt sur le canvas, lu par la page (navigateur) : (doigt, 0 posé / 1 bouge / 2 levé / 3 annulé, x, y, heure
    /// du doigt sur l'horloge de `touch::now_ms`, B-048).
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    Finger(u64, u8, f64, f64, f64),
    /// ✨ Réponse de Claude (ou l'erreur).
    Ai(Result<argos_data::claude::Answer, String>),
    /// 📅 Calendrier économique de la semaine (ou l'erreur).
    Agenda(Result<Vec<argos_data::eco::Event>, String>),
    /// Bougies des slots MTF de l'AI Trend (UT plus petites que la chart) : clé (marché, durées), bougies par durée.
    MtfBars(String, argos_script::indicators::ai_trend::LowerBars),
    /// 🔬 Lower-timeframe bars for the scripts: key, and for each timeframe (seconds) its bars or the failure.
    ScriptBars(String, Vec<(i64, Result<Vec<argos_core::Bar>, String>)>),
    /// Test du jeton OANDA : nombre de comptes, ou l'erreur d'OANDA.
    OandaTest(Result<usize, String>),
    /// 📈 Relevé CoinMarketCap ou l'erreur (navigateur : par le relais du serveur du PC).
    Cmc(Result<argos_data::cmcap::CapPoint, String>),
    /// 📈 Test de la clé CoinMarketCap : capitalisation totale, ou l'erreur.
    CmcTest(Result<f64, String>),
    /// 🔑 Navigateur : réponse du serveur du PC sur la clé CoinMarketCap (présente ou non, ou l'erreur) ; `true` :
    /// après un envoi ou un oubli (sinon une simple vérification).
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    CleCmc(Result<bool, String>, bool),
    /// 🛟 Browser: PC server response to a Support request (route, with a body, JSON text or the error).
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    Support(&'static str, bool, Result<String, String>),
    /// 🔐 Face ID : activée (message) ou connexion (le compte et ses clés, JSON), ou l'erreur.
    Passkey(Result<String, String>, bool),
    /// 📰 Nouvelles (ou l'erreur).
    News(Result<Vec<argos_data::eco::News>, String>),
    /// 🐋 Flux des baleines (grosses transactions, connexion, coupure).
    Whales(LiveEvent),
    /// 📖 ⚡ Flux du carnet et des trades (modules de la colonne).
    Flow(LiveEvent),
    /// 🧮 Flux de trades d'une bourse branchée à l'orderflow réel (sauf Hyperliquid, qui passe par `Flow`).
    Orderflow(Exchange, LiveEvent),
    /// 🧮 Tailles de contrat d'une bourse lues (ou l'erreur), pour l'orderflow réel.
    OrderflowContrats(Exchange, Result<(), String>),
    /// 🌍 Tuiles des marchés macro (ou l'erreur).
    World(Result<Vec<argos_data::world::Tile>, String>),
    /// 💥 Flux des liquidations.
    Liq(LiveEvent),
    /// 💸 Fundings des dernières 24 h (pièce, liste ou erreur).
    FundingHist(String, Result<Vec<argos_data::book::Funding>, String>),
    /// ⚡ Taux de funding réel (pour 8 h) d'un perpétuel démo, lu à sa bourse (`None` : pas de réponse).
    FundingDemo(Exchange, String, Option<f64>),
    /// 🔐 🌐 Le journal lu sur le PC (cell : serveur local), ou l'erreur.
    #[cfg(target_arch = "wasm32")]
    JournalLu(Result<argos_core::journal::Donnees, String>),
    /// 🔐 🌐 Réponse du PC à des changements envoyés (et s'il faut effacer l'ancienne copie de la page).
    #[cfg(target_arch = "wasm32")]
    JournalEcrit(Result<argos_core::journal::Donnees, String>, argos_core::journal::Changements, bool),
    /// 💥 Carte des liquidations : clé (pièce | période), données de chaque bourse, et pour Hyperliquid ses bougies et
    /// l'intérêt ouvert du moment.
    LiqMap(
        String,
        Vec<(argos_data::liqdata::Venue, Result<Vec<argos_core::liqmap::OiBar>, String>)>,
        Option<LiqHl>,
    ),
    /// 🧱 Argos Murs: the books of a coin (each exchange: its book, or the error), and the far book when it was asked
    /// (lot loin-1: Hyperliquid grouped by 2 figures, every third photo).
    Murs(
        String,
        Vec<(argos_data::murs::Carnet, Result<argos_core::murs::VenueBook, String>)>,
        Option<Result<argos_core::murs::VenueBook, String>>,
        Option<u64>,
    ),
    /// 📖 Argos Murs: Binance perp's diff depth stream (its generation, the event).
    MursFlux(u64, LiveEvent),
    /// 📖 🔥 The chosen books of the heatmap and the order-book panel (choice key, each book or its error).
    Carnets(
        String,
        Vec<(argos_data::murs::Carnet, argos_data::murs::Monnaie, Result<argos_core::murs::VenueBook, String>)>,
    ),
    /// 📖 Bitfinex's order book of the chart (its key, the book or the error).
    CarnetBitfinex(String, Result<argos_data::book::Book, String>),
    /// ⚡ A Bitfinex perpetual's figures (its base, the figures or the error).
    StatutBitfinex(String, Result<Option<argos_data::exchanges::bitfinex::Statut>, String>),
    /// 🔁 The PC server's exchanges relay answered (`true`) or not (browser).
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    RelaisBourse(bool),
    /// 🧭 A reading of the Explorer Hyperliquid (its address).
    Hl(String, crate::actions::hyperliquid::HlLu),
    /// 👁 Compte d'une baleine suivie (adresse, compte ou erreur).
    WhaleAccount(String, Result<argos_data::whales::Account, String>),
    /// 🏦 Catalogue OANDA complet (avec le jeton de l'utilisateur), ou l'erreur.
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    OandaCatalog(Result<Vec<Instrument>, String>),
    /// 🔝 Top 30 de chaque exchange (Baleines) : (exchange, liste ou erreur).
    WhaleTop(Vec<(String, Result<Vec<String>, String>)>),
    /// 📊 Flux acheteur / vendeur (bougies Binance) d'une pièce : (1 h sur 7 j, 5 min sur 24 h).
    WhaleFlow(String, Result<(Vec<argos_data::whales::Flow>, Vec<argos_data::whales::Flow>), String>),
    /// 👛 État public d'une adresse (wallet en lecture seule).
    Wallet(String, Result<argos_data::wallet::Snapshot, String>),
    /// 📊 Tickers 24 h d'un exchange (Binance, OKX).
    Tickers(argos_data::Exchange, Result<Vec<argos_data::tickers::Ticker>, String>),
    /// 🦎 Paires DEX trouvées (texte cherché, paires).
    DexFound(String, Result<Vec<argos_data::dex::Pair>, String>),
    /// 🦎 Fiche et derniers trades de la pool affichée (clé de la pool).
    DexPool(String, Option<Box<argos_data::dex::PoolInfo>>, Vec<argos_data::dex::Trade>),
    /// 📡 Résultat du scanner double structure (lignes, symboles sans données).
    Scan(Vec<argos_render::dock::ScanLine>, Vec<String>),
    /// 🔄 Des réglages ont changé sur un autre appareil (noms des fichiers).
    Reglages(Vec<String>),
    /// 📱 Touche du clavier du cell (Effacer, Entrée, Échap).
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    SoftKey(&'static str),
    /// 🕯 The browser's candle cache read for the series opened (empty: none or an error): then the download.
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    CacheLu(u64, Vec<Bar>),
}

impl App {
    /// Le direct : la dernière bougie est remplacée, ou une nouvelle commence (l'historique ne change jamais).
    pub(crate) fn on_user_event(&mut self, ev: UserEvent) {
        match ev {
            UserEvent::Loaded(g, _) | UserEvent::Live(g, _) | UserEvent::CacheLu(g, _) if g != self.gen.load(Ordering::SeqCst) => return,
            UserEvent::CacheLu(g, bars) => return self.cache_lu(g, bars),
            UserEvent::Quotes(sections, all) => {
                self.feeds.hl_sections = sections;
                self.feeds.quotes = all.into_iter().map(|q| (q.coin.clone(), q)).collect();
                self.refresh_watch();
                let closed = self.demo_exits();
                self.alerts_watch();
                if !self.watch.prefs.open && !closed {
                    return; // panneau fermé : rien à redessiner
                }
            }
            UserEvent::Market(m) => {
                let mut m = *m;
                // 📈 marché de CoinGecko (gratuit) : un point d'historique CRYPTOCAP: toutes les 5 min au plus
                if let Some(p) = m.global.as_ref().and_then(|g| argos_data::market::point_of(g, now_s())) {
                    if self.feeds.cmc_hist.last().is_none_or(|l| p.t - l.t >= argos_data::cmcap::EVERY_S) {
                        argos_data::cmcap::push(&mut self.feeds.cmc_hist, p);
                        Self::write_prefs("cmc_history.json", &self.feeds.cmc_hist);
                        self.cap_refresh();
                    }
                }
                self.cmc_over(&mut m);
                self.watch.market = m;
            }
            // ⌨ 📱 the page's text and keys: in a search, where the computer's keyboard goes; else the page's own list
            UserEvent::Paste(text) => {
                if !self.touche_page(None, Some(&text)) {
                    self.paste_text(text);
                }
            }
            UserEvent::Finger(id, ph, x, y, te) => {
                use winit::event::TouchPhase as P;
                let phase = [P::Started, P::Moved, P::Ended, P::Cancelled][ph.min(3) as usize];
                return self.touch_at(id, phase, (x, y), te);
            }
            UserEvent::SoftKey(k) => {
                if !self.touche_page(Some(k), None) {
                    self.soft_key(k);
                }
                return;
            }
            UserEvent::ScriptBars(key, res) => self.script_bars_recues(key, res),
            UserEvent::MtfBars(key, bars) => {
                // seulement si c'est encore le même marché et les mêmes slots ; on recalcule tout de suite
                if self.feeds.mtf_at.as_ref().is_some_and(|(k, _)| *k == key) {
                    self.feeds.mtf_bars = bars;
                    self.ind.key = Default::default();
                }
            }
            UserEvent::Passkey(res, login) => match (res, login) {
                (Ok(m), false) => self.banner = Some(m),
                (Ok(json), true) => {
                    // connexion Face ID : le compte (public) revient ; ses clés restent dans le paquet chiffré
                    match serde_json::from_str::<serde_json::Value>(&json)
                        .ok()
                        .and_then(|v| serde_json::from_value::<account::Account>(v["account"].clone()).ok())
                    {
                        Some(a) => {
                            let _ = platform::write(account::FILE, &serde_json::to_string_pretty(&a).unwrap_or_default());
                            self.banner = Some(format!("🔐 Connecté avec Face ID : {}", a.id_full()));
                            self.top.account = a.id_full();
                            self.session.account = Some(a);
                        }
                        None => self.banner = Some("🔐 Paquet Face ID illisible : connecte-toi avec tes 24 mots".into()),
                    }
                }
                (Err(e), _) => self.banner = Some(format!("🔐 {e}")),
            },
            UserEvent::Cmc(res) => self.cmc_point(res),
            UserEvent::CleCmc(res, envoi) => self.cmc_cle_recue(res, envoi),
            UserEvent::Support(route, envoi, rep) => self.support_recu(route, envoi, rep),
            UserEvent::CmcTest(res) => {
                self.banner = Some(match res {
                    Ok(total) => format!(
                        "📈 CoinMarketCap OK ✅ (capitalisation totale {:.2} T$) : un point est gardé toutes les 5 min",
                        total / 1e12
                    ),
                    Err(e) => format!("📈 CoinMarketCap : erreur — {e}"),
                });
            }
            UserEvent::OandaTest(res) => {
                self.banner = Some(match res {
                    Ok(n) => format!(
                        "🏦 OANDA branché ✅ ({n} compte{}) : les symboles OANDA prennent maintenant les vrais prix",
                        if n > 1 { "s" } else { "" }
                    ),
                    Err(e) => format!("🏦 OANDA refuse le jeton : {e} (compte démo ou réel ?)"),
                });
            }
            UserEvent::Minutes(coin, bars) => self.cal_minutes_lues(coin, bars),
            UserEvent::Reel(reel, list) => {
                self.feeds.reel_busy = false;
                self.feeds.reel = reel;
                if list.iter().map(|e| (&e.title, e.ts)).eq(self.watch.eco.agenda.iter().map(|e| (&e.title, e.ts))) {
                    self.watch.eco.agenda = list;
                } else {
                    self.feeds.reel.fill(&mut self.watch.eco.agenda);
                }
            }
            UserEvent::Agenda(res) => match res {
                Ok(mut list) => {
                    self.feeds.reel.fill(&mut list);
                    self.watch.eco.carte = None;
                    (self.watch.eco.agenda, self.watch.eco.note) = (list, String::new())
                }
                Err(e) => self.watch.eco.note = agenda_note(&e.to_string()),
            },
            UserEvent::News(res) => match res {
                Ok(list) => {
                    self.watch.eco.news_at = Some(now_s());
                    (self.watch.eco.news, self.watch.eco.news_note) = (list, String::new())
                }
                Err(e) => self.watch.eco.news_note = format!("⚠️ Nouvelles indisponibles : {}", argos_data::eco::erreur_lisible(&e.to_string())),
            },
            UserEvent::Orderflow(ex, ev) => {
                self.orderflow_evenement(ex, ev);
                return;
            }
            UserEvent::OrderflowContrats(ex, res) => self.orderflow_contrats_lus(ex, res),
            UserEvent::Flow(ev) => match ev {
                LiveEvent::Connected | LiveEvent::Disconnected(_) => {
                    self.orderflow_coupure(Exchange::Hyperliquid);
                    return;
                }
                LiveEvent::Book(b) if Some(&b.coin) == self.feeds.flow_coin.as_ref() => {
                    // other books chosen: they feed the heatmap and the panel, not Hyperliquid's stream
                    if self.carnets_instantanes() {
                        return;
                    }
                    self.heat_add(&b);
                    self.feeds.book = Some(b);
                    self.carnet_livre();
                    // le carnet arrive souvent : on redessine au plus ~10 fois par seconde
                    if self.feeds.heat_draw.is_some_and(|t| t.elapsed().as_millis() < 100) {
                        return;
                    }
                    self.feeds.heat_draw = Some(Instant::now());
                }
                LiveEvent::Tape(list) => {
                    self.orderflow_trades(Exchange::Hyperliquid, &list);
                    self.tape_ajouter(Exchange::Hyperliquid, list);
                }
                _ => return,
            },
            UserEvent::World(res) => match res {
                Ok(t) => (self.feeds.world, self.feeds.world_note) = (t, String::new()),
                Err(e) => self.feeds.world_note = format!("⚠️ Marchés indisponibles : {e}"),
            },
            UserEvent::Liq(LiveEvent::Liqs(mut l)) => {
                l.sort_by_key(|q| -q.time_ms);
                l.append(&mut self.feeds.liqs);
                l.truncate(150);
                self.feeds.liqs = l;
            }
            UserEvent::Liq(_) => return,
            UserEvent::LiqMap(key, venues, hl) => self.liq_loaded(key, venues, hl),
            UserEvent::Murs(coin, res, loin, l) => self.murs_recu(coin, res, loin, l),
            UserEvent::MursFlux(gen, ev) => self.murs_flux_recu(gen, ev),
            UserEvent::Carnets(key, res) => self.carnets_recus(key, res),
            UserEvent::CarnetBitfinex(key, res) => self.bitfinex_carnet_recu(key, res),
            UserEvent::RelaisBourse(oui) => self.relais_bourse_recu(oui),
            UserEvent::StatutBitfinex(base, res) => self.bitfinex_statut_recu(base, res),
            #[cfg(target_arch = "wasm32")]
            UserEvent::JournalLu(res) => self.journal_lu(res),
            #[cfg(target_arch = "wasm32")]
            UserEvent::JournalEcrit(res, envoye, effacer) => self.journal_ecrit(res, envoye, effacer),
            UserEvent::FundingHist(coin, res) => match res {
                Ok(l) if self.feeds.funding_at.as_ref().is_some_and(|(c, _)| *c == coin) => self.feeds.funding_hist = l,
                _ => return,
            },
            UserEvent::FundingDemo(ex, base, t) => match t {
                Some(t) => {
                    self.feeds.funding_demo.insert(base, (ex, t));
                }
                None => return,
            },
            UserEvent::DexFound(q, res) => return self.dex_found(q, res),
            UserEvent::Wallet(addr, res) => return self.wallet_loaded(addr, res),
            UserEvent::OandaCatalog(res) => {
                self.feeds.oanda_cat_busy = false;
                match res {
                    Ok(list) if !list.is_empty() => {
                        if let Some(st) = self.store.as_mut() {
                            match st.save_catalog("oanda", &list, now_s()) {
                                Ok(()) => println!("📇 Catalogue OANDA : {} instrument(s)", list.len()),
                                Err(e) => eprintln!("⚠️ Catalogue OANDA : {e}"),
                            }
                        }
                    }
                    Ok(_) => {}
                    Err(e) => eprintln!("⚠️ Catalogue OANDA : {e}"),
                }
            }
            UserEvent::WhaleTop(lists) => {
                self.feeds.whale_top_busy = false;
                self.feeds.whale_top_at = Some(Instant::now());
                for (e, res) in lists {
                    match res {
                        Ok(l) if !l.is_empty() => {
                            self.feeds.whale_top.insert(e, l);
                        }
                        // échec : les 4 pièces de base, retenté au prochain jour
                        _ => {
                            self.feeds.whale_top.insert(e, argos_data::whales::coins(""));
                        }
                    }
                }
            }
            UserEvent::WhaleFlow(coin, res) => {
                let w = &mut self.watch.whales;
                if coin != w.flow_coin {
                    return;
                }
                match res {
                    Ok((h, m)) => (w.flow_h, w.flow_m) = (h, m),
                    Err(e) => {
                        (w.flow_h, w.flow_m) = (vec![], vec![]);
                        w.note = format!("📊 Flux de tous les ordres indisponible : {e}");
                    }
                }
            }
            UserEvent::Tickers(ex, res) => {
                self.feeds.tickers_busy.remove(&ex);
                if let Ok(list) = res {
                    self.feeds.tickers.insert(ex, (Instant::now(), list));
                }
                // 🔎 the market panel: its FAVORIS get the 24 h change of these tickers
                if self.ui.marches.is_some() {
                    self.marches_remplir();
                }
                // 🔎 the market panel: its FAVORIS get the 24 h change of these tickers
                if self.ui.marches.is_some() {
                    self.marches_remplir();
                }
                // menu Top ouvert : refait avec les nouveaux chiffres (même place, même défilement)
                if let Some((ListKind::Movers, m)) = self.ui.list.take() {
                    let (anchor, scroll) = (m.anchor, m.scroll);
                    let (ww, wh) = (self.win_w(), self.area.1 as f64);
                    let mut n = ListMenu::new(self.movers_menu(), vec![], anchor, ww, wh);
                    n.scroll = scroll;
                    self.ui.list = Some((ListKind::Movers, n));
                } else if let Some(l) = self.ui.list.take() {
                    self.ui.list = Some(l);
                }
            }
            UserEvent::DexPool(key, info, trades) => {
                if key != self.chart.source.key() {
                    return;
                }
                if info.is_some() {
                    self.feeds.dex_info = info.map(|b| *b);
                }
                if !trades.is_empty() {
                    self.feeds.dex_trades = trades;
                }
            }
            UserEvent::Scan(lines, missing) => {
                self.feeds.scan_busy = false;
                self.feeds.scan_note = if missing.is_empty() {
                    String::new()
                } else {
                    format!("sans données : {}", missing.join(", "))
                };
                self.feeds.scan = lines;
            }
            UserEvent::Hl(a, lu) => return self.hl_lu(a, lu),
            UserEvent::WhaleAccount(addr, res) => {
                let w = &mut self.watch.whales;
                if w.follow.as_deref() != Some(addr.as_str()) {
                    return; // plus suivie
                }
                match res {
                    Ok(a) => (w.account, w.follow_note) = (Some(a), String::new()),
                    Err(e) => w.follow_note = format!("⚠️ Positions indisponibles : {e}"),
                }
            }
            UserEvent::Whales(ev) => {
                let w = &mut self.watch.whales;
                match ev {
                    LiveEvent::Whales(list) => {
                        let min = argos_render::whales::LEVELS[w.level.min(3)].1;
                        let seen = list.iter().any(|x| x.usd() >= min && (!w.only || x.coin == w.coin));
                        // 🔔 alerte : 1 M$ et plus sur la pièce du graphique
                        if let Some(big) = list
                            .iter()
                            .filter(|x| x.coin == w.coin && x.usd() >= 1e6)
                            .max_by(|a, b| a.usd().total_cmp(&b.usd()))
                        {
                            self.banner = Some(format!(
                                "🐋 Baleine {} : {} {} ({})",
                                big.coin,
                                if big.buy { "ACHAT" } else { "VENTE" },
                                argos_render::whales::money(big.usd()),
                                big.src
                            ));
                        }
                        w.push(list);
                        self.save_whales(false);
                        if !seen && self.banner.is_none() {
                            return; // rien de nouveau à l'écran
                        }
                    }
                    LiveEvent::Disconnected(e) => w.note = format!("⚠️ Flux coupé ({e}) : reconnexion…"),
                    LiveEvent::Indisponible(e) => w.note = format!("⚠️ Flux indisponible ({e}) : nouvel essai bientôt…"),
                    LiveEvent::Connected => w.note.clear(),
                    _ => return,
                }
            }
            UserEvent::Ai(res) => {
                self.ai.busy = false;
                let msg = match res {
                    Ok(a) if a.sources.is_empty() => a.text,
                    Ok(a) => format!(
                        "{}\n\nSources :\n{}",
                        a.text,
                        a.sources.iter().take(6).map(|(t, u)| format!("• {t} — {u}")).collect::<Vec<_>>().join("\n")
                    ),
                    Err(e) => format!("❌ {e}"),
                };
                // 🧠 ligne « 🧠 RETENIR : … » de Claude : retirée du texte, proposée (retenue seulement avec ton accord)
                let (msg, note) = crate::actions::aimem_split(&msg);
                if !self.ui.ai_mem.sandbox {
                    if let Some(n) = note {
                        self.ai.pending_note = Some(n);
                    }
                }
                self.ai.msgs.push((false, msg));
                self.ai.scroll = 0.0;
            }
            UserEvent::Emoji(nom, res) => {
                if let Ok(svg) = res {
                    argos_render::emoji::charger(&nom, svg.into_bytes());
                    self.redraw();
                }
            }
            UserEvent::LangPack(k, res, silent) => {
                let name = argos_render::i18n::CATALOG.get(k).map_or("", |l| l.1);
                let pack = res
                    .ok()
                    .filter(|raw| serde_json::from_str::<std::collections::HashMap<String, String>>(raw).is_ok_and(|m| !m.is_empty()));
                let code = argos_render::i18n::CATALOG[k].0;
                match (pack, silent) {
                    // mise à jour d'un pack installé : seulement s'il a changé, et sans message
                    (Some(raw), true) => {
                        if platform::read(&format!("lang/{code}.json")).as_deref() != Some(raw.as_str()) {
                            let _ = platform::write(&format!("lang/{code}.json"), &raw);
                            if self.top.prefs.language == k {
                                Self::load_lang_pack(k);
                            }
                        }
                    }
                    (Some(raw), false) => {
                        let _ = platform::write(&format!("lang/{code}.json"), &raw);
                        self.pick_lang(k);
                        self.banner = Some(format!("✅ {name} téléchargé"));
                    }
                    (None, true) => {}
                    (None, false) => self.banner = Some(format!("⬇️ {name} : pack pas encore publié (ou pas d'Internet)")),
                }
                self.redraw();
            }
            UserEvent::Pane(key, res) => {
                for p in self.panes.iter_mut().filter(|p| p.demande() == key) {
                    p.charge = false;
                    match &res {
                        Ok(b) => p.bars = b.clone(),
                        Err(e) => eprintln!("⚠️ Case {key} : {e}"),
                    }
                }
            }
            UserEvent::Reglages(noms) => self.recharger(&noms),
            UserEvent::Prices(ex, list) => {
                self.feeds.ex_prices.insert(ex, list.into_iter().map(|m| (m.base.clone(), m)).collect());
                self.refresh_watch();
            }
            UserEvent::Tous(ex, lignes) => {
                self.feeds.tous = Some((ex, lignes));
                self.refresh_watch();
            }
            UserEvent::TousCaps(caps) => {
                self.feeds.tous_caps = caps;
                self.refresh_watch();
            }
            UserEvent::Catalog(ex, list) => {
                self.cat_loading.remove(&ex);
                if let Some(st) = self.store.as_mut().filter(|_| !list.is_empty()) {
                    match st.save_catalog(ex.id(), &list, now_s()) {
                        Ok(()) => println!("📇 Catalogue {} : {} symbole(s)", ex.name(), list.len()),
                        Err(e) => eprintln!("⚠️ Catalogue {} : {e}", ex.name()),
                    }
                }
                // 🔎 the market panel open: its FAVORIS may now find their market on the chart's exchange
                if self.ui.marches.is_some() {
                    self.marches_remplir();
                }
                if self.ui.add.is_some() {
                    let chips = self.add_chips();
                    if let Some(p) = self.ui.add.as_mut() {
                        p.chips = chips;
                    }
                    self.search_add();
                } else {
                    return;
                }
            }
            UserEvent::Logo(coin, png) => {
                if let Ok(pm) = tiny_skia::Pixmap::decode_png(&png) {
                    self.watch.logos.insert(coin, pm);
                }
                if !self.watch.prefs.open {
                    return;
                }
            }
            UserEvent::Loaded(_, Ok(c)) => {
                self.chart.hors_ligne = false;
                // 📡 bougies d'un secours sur une chart sans direct ici (🔑 KuCoin dans une page web) : son direct
                if let (Some((ex, direct)), Some(proxy)) = (self.direct_de_secours(c.secours), self.proxy.clone()) {
                    self.chart.live = Some(false);
                    crate::run::start_live(ex, direct, proxy, self.gen.clone(), self.gen.load(Ordering::SeqCst));
                }
                if matches!(self.chart.source, Source::Dex { .. }) {
                    self.save_credits(); // 🧮 chandelles CoinMarketCap éventuelles
                }
                let first = self.chart.bars.is_empty();
                println!("✅ {} bougie(s) reçue(s)", c.bars.len());
                let tf = self.source_tf().unwrap_or(Timeframe::M1);
                if c.primary {
                    // cache de la dernière session + ce qui manquait (les fraîches remplacent la dernière du cache)
                    self.chart.raw = cache::merge(&self.chart.raw, c.bars, tf, DEFAULT_CANDLES);
                    self.chart.a_reparer = c.a_reparer;
                } else {
                    self.chart.raw = c.bars;
                    self.chart.cache_path = None;
                }
                // le direct arrivé pendant le chargement : même règle (remplace la dernière ou en ajoute une)
                for b in self.chart.loading.take().unwrap_or_default() {
                    live::apply(&mut self.chart.raw, b);
                }
                self.title = c.title;
                // (🗂 🗑 the banner of a layout opened or deleted stays: the candles of its market arrive after it)
                let mise_en_page = self.banner.take().filter(|b| b.starts_with('🗂') || b.starts_with('🗑'));
                self.banner = c.banner.or(mise_en_page);
                self.rebuild();
                self.chart.viewport.scroll_px(0.0, &self.chart.slots);
                if first {
                    self.start_free(); // avec le cache, c'est déjà fait au démarrage
                }
                self.write_cache();
                self.set_window_title();
                // ⚡ premier écran affiché, direct branché : le reste du passé se remplit maintenant, en arrière-plan
                if c.primary {
                    self.fill_history();
                    // 🕳️ puis les trous (s'il n'y a pas de passé à remplir ; sinon après le remplissage)
                    self.repair_gaps();
                }
                // ⏯ un replay du Simulateur attend ces bougies (date choisie, séance reprise)
                self.sim_replay_poursuivre();
            }
            UserEvent::Older(key, res) => return self.older_arrived(key, res),
            UserEvent::Repaired(key, r) => return self.gaps_repaired(key, r),
            UserEvent::Loaded(_, Err(e)) => {
                eprintln!("❌ {e}");
                self.chart.loading = None;
                // 📴 rien ne passe et le direct n'est pas branché : hors ligne (la chart garde le cache)
                if self.chart.live != Some(true) {
                    self.chart.hors_ligne = true;
                }
                let what = if self.chart.bars.is_empty() {
                    "Chargement impossible"
                } else {
                    "Mise à jour impossible (bougies de la dernière session)"
                };
                // (👆 phone and tablet by the finger: the banner leaves « F5 pour réessayer » out, `actions::sans_touches`)
                self.banner = Some(format!("{what} : {e} — F5 pour réessayer"));
                self.sim_replay_poursuivre();
            }
            UserEvent::Live(_, LiveEvent::Connected) => {
                self.chart.live = Some(true);
                self.chart.hors_ligne = false;
                // 🔌 retour après une coupure : on redemande ce qui a été manqué, puis les trous sont réparés
                if std::mem::take(&mut self.chart.coupe) {
                    self.catch_up();
                }
            }
            UserEvent::Live(_, LiveEvent::Bar(b)) if self.chart.loading.is_some() => {
                self.chart.live = Some(true);
                if let Some(wait) = &mut self.chart.loading {
                    live::apply(wait, b);
                }
                // ⚡ le direct bouge tout de suite sur les bougies du cache (il sera réappliqué après le chargement)
                if !self.chart.raw.is_empty() {
                    self.live_bar(b);
                }
            }
            UserEvent::Live(_, LiveEvent::Bar(b)) => {
                (self.chart.live, self.chart.hors_ligne) = (Some(true), false);
                self.live_bar(b);
                self.demo_exits();
                self.alerts_watch();
            }
            // trades : bougies en secondes, ou bougies de l'unité pour un exchange sans canal de bougies (Bitstamp,
            // Coinbase) ; le compte démo et les alertes suivent, comme avec une bougie du direct
            UserEvent::Live(_, LiveEvent::Trades(list)) => {
                (self.chart.live, self.chart.hors_ligne) = (Some(true), false);
                let first = self.chart.raw.is_empty();
                self.live_trades(list);
                if first && !self.chart.bars.is_empty() {
                    self.start_free();
                }
                if self.chart.loading.is_none() {
                    self.demo_exits();
                    self.alerts_watch();
                }
            }
            UserEvent::Live(
                _,
                LiveEvent::Whales(_)
                | LiveEvent::Book(_)
                | LiveEvent::Tape(_)
                | LiveEvent::Liqs(_)
                | LiveEvent::Profondeur(_)
                | LiveEvent::Indisponible(_)
                | LiveEvent::Ctx(_),
            ) => return,
            UserEvent::Live(_, LiveEvent::Disconnected(why)) => {
                self.chart.live = Some(false);
                (self.chart.hors_ligne, self.chart.coupe) = (true, true);
                eprintln!("⚠️ Direct coupé ({why}) : reconnexion…");
            }
        }
        self.redraw_donnees();
    }
}

/// 📅 Message du calendrier qui ne répond pas : la source absente (HTTP 404, le serveur local n'a pas encore `/api`)
/// n'est pas une panne, c'est « bientôt » (règle des écrans pas encore branchés) ; une vraie erreur reste dite.
pub(crate) fn agenda_note(e: &str) -> String {
    if e.contains("404") {
        "📅 Calendrier économique : bientôt".into()
    } else {
        format!("⚠️ Calendrier indisponible : {}", argos_data::eco::erreur_lisible(e))
    }
}

impl App {
    /// 😀 Va chercher les images Twemoji demandées par le dessin (sélecteur, émojis posés, texte) : sur la page, à côté
    /// de l'app (`/emoji/<nom>.svg`) ; sur l'ordinateur, dans le dossier `emoji/` du dépôt (ou `ARGOS_EMOJI`).
    pub(crate) fn charger_emojis(&mut self) {
        for nom in argos_render::emoji::a_charger() {
            #[cfg(target_arch = "wasm32")]
            if let Some(proxy) = self.proxy.clone() {
                argos_data::rt::spawn(move || async move {
                    use argos_data::http::Http;
                    let res = argos_data::rt::Client::default()
                        .get(&format!("/emoji/{nom}.svg"))
                        .await
                        .map_err(|e| e.to_string());
                    let _ = proxy.send_event(UserEvent::Emoji(nom, res));
                });
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let dossiers = std::env::var("ARGOS_EMOJI").into_iter().map(std::path::PathBuf::from).chain(
                    ["emoji", "../emoji", "../../emoji", concat!(env!("CARGO_MANIFEST_DIR"), "/../../../emoji")]
                        .into_iter()
                        .map(std::path::PathBuf::from),
                );
                for d in dossiers {
                    if let Ok(svg) = std::fs::read(d.join(format!("{nom}.svg"))) {
                        argos_render::emoji::charger(&nom, svg);
                        self.redraw();
                        break;
                    }
                }
            }
        }
    }
}
