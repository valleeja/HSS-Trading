//! 🚀 Démarrage de l'app et chargement des bougies (fichier, exchange, OANDA, direct).

use super::*;

/// Bougies chargées.
#[derive(Debug)]
pub(crate) struct Candles {
    pub(crate) bars: Vec<Bar>,
    pub(crate) title: String,
    /// Bandeau du superviseur (source de secours, trous…).
    pub(crate) banner: Option<String>,
    /// Viennent de l'exchange choisi (sinon d'un secours : on ne les mélange pas avec le cache).
    pub(crate) primary: bool,
    /// 📡 L'exchange de secours dont elles viennent quand l'exchange choisi n'a pas répondu (`None` : l'exchange
    /// choisi, ou une source qui n'est pas un exchange).
    pub(crate) secours: Option<Exchange>,
    /// 🕳️ Heures des bougies refusées par la vérification : des trous à réparer (étape 3).
    pub(crate) a_reparer: Vec<i64>,
}

pub(crate) type Loaded = Result<Candles, String>;

/// D'où viennent les bougies (pour recharger avec F5).
#[derive(Clone)]
pub(crate) enum Source {
    Live {
        exchange: Exchange,
        base: String,
    },
    /// 🏦 Instrument OANDA (`XAU_USD`), avec le jeton de l'utilisateur.
    Oanda {
        instrument: String,
    },
    /// 🧮 Paire calculée bougie par bougie : `base` ÷ `quote` sur un exchange (« XRP ÷ BTC » = XRPBTC).
    Ratio {
        exchange: Exchange,
        base: String,
        quote: String,
    },
    /// 🧮 Symbole calculé gratuit (« XAUGBP », « DXY ») : voir `argos_data::synth`.
    Synth {
        sym: String,
    },
    /// 📈 Série CoinMarketCap (« USDT.D », « TOTAL3 »), dessinée avec l'historique gardé par Argos.
    Cap {
        series: String,
    },
    /// 🦎 Paire DEX on-chain (GeckoTerminal) : réseau (« eth », « solana »…), adresse de la pool, nom (« PEPE/WETH »).
    Dex {
        network: String,
        pool: String,
        name: String,
    },
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    File(String),
}

impl Source {
    /// Clé de sauvegarde : un fichier par symbole (les dessins de BTC ne vont pas sur ETH).
    pub(crate) fn key(&self) -> String {
        match self {
            Source::Live { exchange, base } => format!("{}-{base}", exchange.name()),
            Source::Oanda { instrument } => format!("oanda-{instrument}"),
            Source::Ratio { exchange, base, quote } => format!("ratio-{}-{base}-{quote}", exchange.name()),
            Source::Cap { series } => format!("cmc-{series}"),
            Source::Synth { sym } => format!("synth-{sym}"),
            Source::Dex { network, pool, .. } => format!("dex-{network}-{pool}"),
            Source::File(path) => {
                let name = std::path::Path::new(path)
                    .file_stem()
                    .map_or(path.clone(), |f| f.to_string_lossy().into_owned());
                format!("fichier-{name}")
            }
        }
    }
}

/// Bougies d'un fichier (lu en entier).
pub(crate) fn load_file(path: &str, label: &str) -> Result<Candles, String> {
    let bars = load_json(path).map_err(|e| format!("{path} : {e}"))?;
    let name = std::path::Path::new(path)
        .file_name()
        .map_or(path.to_string(), |f| f.to_string_lossy().into_owned());
    Ok(Candles {
        bars,
        title: format!("{name} · {label} · fichier"),
        banner: None,
        primary: true,
        secours: None,
        a_reparer: vec![],
    })
}

/// Bougies d'un exchange (`limit` = combien demander), avec le superviseur (source de secours si besoin).
pub(crate) async fn load_live(exchange: Exchange, base: &str, tf: Timeframe, label: &str, limit: usize) -> Result<Candles, String> {
    println!("Argos HSS Trading · chargement de {base} {tf} sur {} ({limit} bougies)…", exchange.name());
    let supervised = Supervisor::new(argos_data::rt::Client::default())
        .candles(exchange, base, tf, limit)
        .await
        .map_err(|e| e.to_string())?;
    let banner = supervised.banner();
    if let Some(b) = &banner {
        println!("{b}");
    }
    println!("✅ {} bougies reçues de {}", supervised.bars.len(), supervised.source.name());
    Ok(Candles {
        title: titre_charge(supervised.primary, supervised.source, base, label),
        primary: !supervised.is_fallback(),
        secours: supervised.is_fallback().then_some(supervised.source),
        a_reparer: supervised.a_reparer,
        bars: supervised.bars,
        banner,
    })
}

/// 🏷️ Chart title of a load: the exchange the user chose, like the symbol bar; when the supervisor fell back to
/// another exchange, the legend says it: « XBTUSDTM Perpétuel · 1h · KuCoin (données Binance) ».
pub(crate) fn titre_charge(choisie: Exchange, source: Exchange, base: &str, label: &str) -> String {
    let t = choisie.chart_title(base, label);
    if source == choisie {
        t
    } else {
        format!("{t} (données {})", source.name())
    }
}

/// 🧮 Titre d'une paire calculée : « XRPBTC (calculée, approximative) · 1h · Binance ».
pub(crate) fn ratio_title(exchange: Exchange, base: &str, quote: &str, label: &str) -> String {
    format!("{} · {label} · {}", argos_data::tv::ratio_name(base, quote), exchange.name())
}

/// 🧮 Bougies d'une paire calculée : celles de `base` et de `quote` sur l'exchange, divisées bougie par bougie.
/// 🧮 Titre d'un symbole calculé : « XAUGBP (calculé, approximatif) · 1h · Hyperliquid HIP-3 » (revue R&D, point 5 :
/// marqué approximatif, et jamais mis en cache comme une donnée d'exchange).
pub(crate) fn synth_title(sym: &str, label: &str) -> String {
    match argos_data::synth::legs(sym) {
        Some(_) => format!("{sym} (calculé, approximatif) · {label} · Hyperliquid HIP-3"),
        None => format!("{sym} · {label}"),
    }
}

/// 🧮 Bougies d'un symbole calculé : or / argent ÷ livre / euro (Hyperliquid HIP-3), ou le DXY (Yahoo, sinon calculé
/// avec 5 devises de Kraken).
pub(crate) async fn load_synth(sym: &str, tf: Timeframe, label: &str, limit: usize) -> Result<Candles, String> {
    if let Some((a, b)) = argos_data::synth::legs(sym) {
        let x = load_live(Exchange::Hyperliquid, a, tf, label, limit).await.map_err(|e| format!("{a} : {e}"))?;
        let y = load_live(Exchange::Hyperliquid, b, tf, label, limit).await.map_err(|e| format!("{b} : {e}"))?;
        let bars = argos_data::tv::ratio_bars(&x.bars, &y.bars);
        if bars.is_empty() {
            return Err(format!("{a} et {b} n'ont aucune bougie à la même heure"));
        }
        return Ok(Candles {
            title: synth_title(sym, label),
            banner: Some(format!(
                "🧮 {sym} calculé, approximatif : {a} ÷ {b} bougie par bougie (Hyperliquid HIP-3, sans clé, pas de volume)"
            )),
            primary: false,
            secours: None,
            a_reparer: vec![],
            bars,
        });
    }
    match argos_data::synth::dxy_yahoo(tf.seconds()).await {
        Ok(bars) => Ok(Candles {
            title: format!("DXY · {label} · Yahoo"),
            banner: None,
            primary: false,
            secours: None,
            a_reparer: vec![],
            bars,
        }),
        Err(_) => {
            let bars = argos_data::synth::dxy_calc(tf.seconds()).await?;
            Ok(Candles {
                title: format!("DXY (calculé, approximatif) · {label} · Kraken"),
                banner: Some(
                    "🧮 DXY calculé, approximatif : formule de l'ICE avec EUR, JPY, GBP, CAD, CHF (Kraken) ; la couronne \
                     suédoise suit l'euro (le vrai DXY vient de Yahoo, injoignable ici)"
                        .into(),
                ),
                primary: false,
                secours: None,
                a_reparer: vec![],
                bars,
            })
        }
    }
}

pub(crate) async fn load_ratio(exchange: Exchange, base: &str, quote: &str, tf: Timeframe, label: &str, limit: usize) -> Result<Candles, String> {
    let a = load_live(exchange, base, tf, label, limit).await.map_err(|e| format!("{base} : {e}"))?;
    let b = load_live(exchange, quote, tf, label, limit).await.map_err(|e| format!("{quote} : {e}"))?;
    let bars = argos_data::tv::ratio_bars(&a.bars, &b.bars);
    if bars.is_empty() {
        return Err(format!("{base} et {quote} n'ont aucune bougie à la même heure sur {}", exchange.name()));
    }
    Ok(Candles {
        title: ratio_title(exchange, base, quote, label),
        // 🧮 toujours dit sur la chart : une paire calculée est approximative (pas une donnée d'exchange)
        banner: a.banner.or(b.banner).or_else(|| {
            Some(format!(
                "🧮 Paire calculée, approximative : {} ÷ {} bougie par bougie sur {} (pas de volume)",
                base.to_uppercase(),
                quote.to_uppercase(),
                exchange.name()
            ))
        }),
        primary: false,
        secours: None,
        a_reparer: vec![],
        bars,
    })
}

/// Bougies d'un fichier JSON (liste d'objets `time` en secondes, `open`, `high`, `low`, `close`, `volume`).
pub(crate) fn load_json(path: &str) -> Result<Vec<Bar>, String> {
    let raw = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let v: serde_json::Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    let rows = v.as_array().ok_or("le fichier doit contenir une liste de bougies")?;
    let num = |b: &serde_json::Value, k: &str| b[k].as_f64().ok_or(format!("champ « {k} » manquant"));
    let bars = rows
        .iter()
        .map(|b| {
            Ok(Bar {
                time: b["time"].as_i64().ok_or("champ « time » manquant")?,
                open: num(b, "open")?,
                high: num(b, "high")?,
                low: num(b, "low")?,
                close: num(b, "close")?,
                volume: num(b, "volume").unwrap_or(0.0),
            })
        })
        .collect::<Result<Vec<Bar>, String>>()?;
    Ok(argos_core::feed::clean_bars(bars))
}

/// Ouvre la fenêtre tout de suite. Un fichier se lit sur place (instantané) ; un exchange se charge en arrière-plan,
/// en même temps que le direct, et la chart se remplit quand les bougies arrivent.
pub(crate) fn run(source: Source, iv: Option<Interval>) {
    crate::persist::saved::preparer(); // 📁 réglages copiés au premier lancement (avant de les lire)
    let tools: toolbar::Prefs = App::read_prefs("toolbar.json");
    // fichier : l'intervalle de ses bougies (30m sans précision) ; exchange : la commande, sinon le dernier choisi
    let file_interval = iv.unwrap_or(Interval(1800));
    let interval = match source {
        Source::File(_) => file_interval,
        Source::Live { .. } | Source::Oanda { .. } | Source::Ratio { .. } | Source::Synth { .. } | Source::Cap { .. } | Source::Dex { .. } => {
            iv.unwrap_or(Interval(tools.interval))
        }
    };
    // un fichier se lit sur place ; un exchange s'ouvre avec `open_live` (cache tout de suite, le reste en arrière-plan)
    let (bars, title) = match &source {
        Source::File(path) => match load_file(path, &interval.short()) {
            Ok(c) => (c.bars, c.title),
            Err(e) => {
                eprintln!("❌ {e}");
                std::process::exit(1);
            }
        },
        Source::Live { exchange, base } => (vec![], exchange.chart_title(base, &interval.short())),
        Source::Oanda { instrument } => (vec![], format!("{} · OANDA · {}", instrument.replace('_', ""), interval.short())),
        Source::Ratio { exchange, base, quote } => (vec![], ratio_title(*exchange, base, quote, &interval.short())),
        Source::Synth { sym } => (vec![], synth_title(sym, &interval.short())),
        Source::Cap { series } => (vec![], crate::actions::cap_title(series, &interval.short())),
        Source::Dex { network, name, .. } => (vec![], crate::actions::dex_title(name, network, &interval.short())),
    };
    let mut app = App::blank(source.clone(), interval, file_interval, bars, title);
    // réglages gardés entre deux lancements (dossier de config, ou stockage de la page)
    app.tools = tools;
    app.ai.sandbox = app.ui.ai_mem.sandbox; // 🧠 / 🧪 dans l'en-tête de l'assistant
    app.watch.prefs = App::read_prefs("watchlist.json");
    if app.watch.prefs.passer_barre_v2() {
        App::write_prefs("watchlist.json", &app.watch.prefs); // la barre de droite de la maquette (une seule fois)
    }
    app.watch.dock = App::read_prefs("dock.json");
    app.watch.dock.fix();
    app.top.prefs = App::read_prefs("topbar.json");
    if app.top.prefs.passer_au_verre() | app.top.prefs.passer_accueil_v2() {
        App::write_prefs("topbar.json", &app.top.prefs);
    }
    // 👋 interface-2 : les pages de bienvenue vues ou non, de son propre fichier
    app.top.prefs.bienvenue_vue = App::bienvenue_lire();
    // 👋 /app2/?bienvenue=1 : les 3 pages du premier lancement, de nouveau (pour les essais)
    #[cfg(target_arch = "wasm32")]
    if web_sys::window()
        .and_then(|w| w.location().search().ok())
        .and_then(|q| web_sys::UrlSearchParams::new_with_str(&q).ok())
        .and_then(|p| p.get("bienvenue"))
        .is_some_and(|v| v == "1")
    {
        app.top.prefs.bienvenue_vue = false;
    }
    if vault::load(vault::FILE).is_some() {
        app.ai.key = argos_render::assistant::KeyState::Locked;
    }
    app.session.account = account::load();
    app.rail = App::read_prefs("rail.json");
    app.passer_au_cadre(); // 🖥 rail replié, barre de favoris à sa place d'interface-2
    app.split = App::read_prefs("split.json");
    // ⌂ l'Accueil au lancement : la première fois (ses 3 écrans), s'il était ouvert dans cet onglet avant F5, ou si le
    // réglage le demande ; sinon on revient là où on était (mise en page, symbole, volets)
    app.accueil_au_depart();
    app.clipboard = platform::Clipboard::new();
    #[cfg(not(target_arch = "wasm32"))]
    {
        app.store = argos_store::default_path().and_then(|p| Store::open(&p).map_err(|e| eprintln!("⚠️ Base Argos : {e}")).ok());
        // 🔄 les listes changées ailleurs (page web, téléphone) pendant que l'app était fermée : seulement ajoutées
        // (rien ne se perd au démarrage ; la base reste la source si le fichier manque)
        if let (Some(st), Some(l)) = (
            app.store.as_mut(),
            platform::read("watchlists.json").as_deref().and_then(argos_store::listes::lire),
        ) {
            argos_store::listes::appliquer(st, &l, false);
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        app.store = Some(Store::from_lists_json(&platform::read("watchlists.json").unwrap_or_default()));
    }
    app.demo = App::read_prefs("demo_account.json");
    app.ui.seance = App::read_prefs(crate::actions::simulateur::SEANCE);
    app.alerts = App::read_prefs("alerts.json");
    app.ind.leg = App::read_prefs("indicator_legend.json");
    app.ind.on = App::read_prefs("indicators.json");
    // 📈 default indicators: the set of a fresh install, or Argos Murs once on a profile that never had it
    app.indicateurs_par_defaut(platform::read("indicators.json").is_none());
    app.ind.favs = App::read_prefs("market_fav.json");
    (app.ind.set, app.ind.scripts_set) = App::lire_reglages();
    app.ind.tables = App::read_prefs::<argos_render::catalogue::ReglagesParNom>(crate::actions::indicators::REGLAGES_NOMS).tableaux;
    app.ind.texts = App::read_prefs("indicator_texts.json");
    app.templates = App::read_prefs("templates.json");
    app.tool_styles = App::read_prefs("tool_styles.json");
    app.ex_prefs = App::read_prefs("exchanges.json");
    app.scripts = App::read_prefs("scripts.json");
    app.ids_des_scripts();
    app.mods.list = App::read_prefs("modules.json");
    app.mods.catalog = App::read_prefs("modules-catalogue.json");
    app.feeds.enregistres = App::read_prefs("orderflow.json"); // 🧮 ● marchés enregistrés en continu
    let event_loop = EventLoop::<UserEvent>::with_user_event().build().expect("boucle d'événements");
    app.proxy = Some(event_loop.create_proxy());
    match source {
        Source::Live { exchange, base } => app.open_live(exchange, base),
        Source::Oanda { instrument } => app.open_oanda(instrument),
        Source::Ratio { exchange, base, quote } => app.open_ratio(exchange, base, quote),
        Source::Synth { sym } => app.open_synth(sym),
        Source::Cap { series } => app.open_cap(&series),
        Source::Dex { network, pool, name } => app.open_dex(network, pool, name),
        Source::File(_) => {
            app.restore_session();
            app.chart.raw = std::mem::take(&mut app.chart.bars);
            app.rebuild();
            app.start_free();
        }
    }
    // 🔄 synchro en direct : les réglages changés ailleurs (fil qui regarde le dossier, ou page web réveillée par
    // synchro.js) sont relus tout de suite
    let proxy = event_loop.create_proxy();
    #[cfg(not(target_arch = "wasm32"))]
    crate::actions::synchro::surveiller(move |noms| proxy.send_event(UserEvent::Reglages(noms)).is_ok());
    #[cfg(target_arch = "wasm32")]
    platform::web::reveil_synchro(move |noms| {
        let _ = proxy.send_event(UserEvent::Reglages(noms));
    });
    // la watchlist Hyperliquid se rafraîchit en arrière-plan
    let proxy = event_loop.create_proxy();
    hl_watch::spawn(app.feeds.hl_wanted.clone(), move |sections, all| {
        proxy.send_event(UserEvent::Quotes(sections, all)).is_ok()
    });
    // les prix des autres exchanges (ceux de la liste affichée et du symbole ouvert), en arrière-plan
    let proxy = event_loop.create_proxy();
    argos_data::prices::spawn(app.feeds.price_wanted.clone(), move |ex, m| proxy.send_event(UserEvent::Prices(ex, m)).is_ok());
    // 🌐 « Tous les marchés » : la plateforme affichée, à la minute (rien quand la vue est fermée)
    let (p1, p2) = (event_loop.create_proxy(), event_loop.create_proxy());
    argos_data::tous::spawn(
        app.feeds.tous_wanted.clone(),
        move |ex, l| p1.send_event(UserEvent::Tous(ex, l)).is_ok(),
        move |caps| p2.send_event(UserEvent::TousCaps(caps)).is_ok(),
    );
    let proxy = event_loop.create_proxy();
    if let Ok(mut c) = app.feeds.cmc_credits.lock() {
        *c = App::read_prefs("cmc_credits.json");
    }
    argos_data::market::spawn(app.feeds.market_wanted.clone(), move |m| {
        proxy.send_event(UserEvent::Market(Box::new(m))).is_ok()
    });
    // tes watchlists (base locale) et les catalogues des exchanges (rafraîchis chaque jour, en arrière-plan)
    app.load_lists();
    // quels exchanges ont déjà une clé dans le trousseau (les valeurs ne sont pas gardées)
    app.session.key_set = Exchange::ALL.into_iter().filter(|e| keys::has(e.id())).collect();
    app.top.account = app.session.account.as_ref().map(|a| a.id_full()).unwrap_or_default();
    app.session.oanda = keys::read("oanda", "token").map(|t| (t, keys::read("oanda", "env").as_deref() == Some("live")));
    // 📈 CoinMarketCap (seulement avec une clé, 🔑 Clés API) : la clé (trousseau), l'historique gardé, puis le relevé
    // de fond (une fois par jour par défaut ; au démarrage seulement si le dernier est plus vieux)
    app.session.cmc = if crate::actions::cmc::CMC_OFF { None } else { keys::read("cmc", "key") };
    app.watch.market.cmc_cle = app.session.cmc.is_some();
    app.feeds.cmc_hist = App::read_prefs("cmc_history.json");
    app.feeds.wallet_addr = App::read_prefs::<crate::actions::WalletPrefs>("wallet.json").address;
    App::read_prefs::<crate::actions::WhaleStore>("whales.json").apply(&mut app.watch.whales);
    // 🧭 Explorer Hyperliquid: the followed addresses and the reading interval
    let ex = App::read_prefs::<crate::actions::hyperliquid::ExplorerPrefs>("explorer.json");
    (app.watch.explorer.suivies, app.watch.explorer.intervalle) = (ex.suivies, ex.intervalle);
    app.feeds.cmc_last = App::read_prefs::<Option<argos_data::cmcap::CapPoint>>("cmc_last.json");
    app.feeds
        .cmc_ctl
        .every_s
        .store(app.ex_prefs.cmc_every_h.clamp(1, 168) as i64 * 3600, Ordering::SeqCst);
    app.cmc_apply();
    // 🌐 navigateur : la clé est-elle rangée au trousseau du PC ? (réponse : UserEvent::CleCmc)
    #[cfg(target_arch = "wasm32")]
    app.cle_serveur(None);
    // 🔁 navigateur : le relais des bourses du serveur du PC est-il là ? (Bitfinex, KuCoin ; UserEvent::RelaisBourse)
    #[cfg(target_arch = "wasm32")]
    app.relais_bourse_chercher();
    {
        let proxy = event_loop.create_proxy();
        let last = app.feeds.cmc_last.as_ref().map(|p| p.t);
        argos_data::cmcap::spawn(
            app.feeds.cmc_wanted.clone(),
            app.feeds.cmc_credits.clone(),
            app.feeds.cmc_ctl.clone(),
            last,
            now_s,
            move |r| proxy.send_event(UserEvent::Cmc(r)).is_ok(),
        );
    }
    app.refresh_watch();
    app.refresh_catalogs();
    app.journal_charger(); // 🔐 journal chiffré (migré une fois depuis l'ancien journal.json en clair)
    app.accounts = App::read_prefs("demo_accounts.json");
    app.accounts_ready();
    // langue choisie avant ; un pack téléchargé se met à jour en silence (nouvelles phrases)
    let lang = app.top.prefs.language;
    app.pick_lang(lang);

    #[cfg(not(target_arch = "wasm32"))]
    event_loop.run_app(&mut app).expect("fenêtre Argos");
    // navigateur : la boucle de la page fait rouler l'app ; coller arrive par l'événement de la page
    #[cfg(target_arch = "wasm32")]
    {
        use winit::platform::web::EventLoopExtWebSys;
        let proxy = event_loop.create_proxy();
        let (p1, p2, p3) = (proxy.clone(), proxy.clone(), proxy.clone());
        platform::web::listen_touch(move |id, ph, x, y, te| _ = p3.send_event(UserEvent::Finger(id, ph, x, y, te)));
        platform::web::listen_paste(move |t| {
            let _ = proxy.send_event(UserEvent::Paste(t));
        });
        platform::web::listen_pointer();
        platform::web::listen_browser_keys();
        platform::web::listen_keyboard(
            move |t| _ = p1.send_event(UserEvent::Paste(t)),
            move |k| _ = p2.send_event(UserEvent::SoftKey(k)),
        );
        event_loop.spawn_app(app);
    }
}

impl App {
    /// Une app toute neuve, sans fenêtre et sans rien lire du disque (réglages par défaut, pas de base, pas de
    /// presse-papiers). `run` la complète avec les réglages gardés ; les tests s'en servent telle quelle.
    pub(crate) fn blank(source: Source, interval: Interval, file_interval: Interval, bars: Vec<Bar>, title: String) -> App {
        App {
            chart: state::ChartState {
                save_path: None,
                last_saved: Default::default(),
                hist: Default::default(),
                settings: ChartSettings::default(),
                source,
                slots: slots(&bars, &CandleStyle::all()[0]),
                styles: CandleStyle::all(),
                style: 0,
                bars,
                interval,
                raw: vec![],
                file_interval,
                viewport: Viewport {
                    footer: bottombar::HEIGHT, // barre du bas : périodes, « Aller à », horloge
                    ..Viewport::default()
                },
                live: None,
                loading: None,
                cache_path: None,
                older_busy: false,
                history_start: false,
                older_key: String::new(),
                older_at: None,
                older_via: None,
                a_reparer: vec![],
                repair_busy: false,
                trous_connus: (String::new(), vec![]),
                hors_ligne: false,
                coupe: false,
                debut_propre: None,
                drawings: vec![],
            },
            title,
            tools: Default::default(),
            pointer: state::PointerState {
                tool_drag: None,
                dlg_drag: None,
                tool_moved: false,
                bar_press: None,
                win_mouse: (0.0, 0.0),
                top_mouse: None,
                phone_mouse: None,
                wl_resize: None,
                rail_mouse: None,
                mouse: None,
                drag: None,
                pane_drag: None,
                pane_click_at: None,
                axis_drag: None,
                last_axis_click: None,
                last_time_click: None,
                murs_appui: None,
                last_chart_click: None,
                triple: None,
                appui: None,
                appui_avant: None,
                appui_croix: false,
                side_mouse: None,
                mark_drag: None,
                band_drag: None,
                script_drag: None,
                band_pan: None,
                band_zoom: None,
                script_pan: None,
                script_zoom: None,
                dock_mouse: None,
                mods: ModifiersState::empty(),
                grab: None,
                drawbar_drag: None,
                cp_drag: false,
                curseur_ticket: None,
                cursor: CursorIcon::Default,
                point_pose: false,
                mk_drag: None,
                touch: Default::default(),
                dernier_doigt: f64::NEG_INFINITY,
                tour_debut: None,
                t_geste: None,
                doigt_leve: false,
                appui_glisse: false,
                trace_doigt: false,
                fx_appui: false,
                finger_scroll: None,
                bar_swipe: None,
                doc_pan: None,
                icon_drag: None,
                fling: (0.0, 0.0, 0.0),
                alert_drag: None,
                alert_appui: None,
                order_drag: None,
                order_err: None,
                tab: Default::default(),
            },
            ui: state::UiState {
                zen: false,
                explication: None,
                alerte_sel: None,
                alerte_survol: None,
                confirmer: None,
                plein_ecran: None,
                plein_cible: None,
                plein_zen_avant: false,
                bulle_plein: None,
                zen_haut: false,
                ai_mem: App::read_prefs("ai_memory.json"),
                ai_clear_armed: false,
                ideas: App::read_prefs("ideas.json"),
                ideas_delete: false,
                alert_line: None,
                alert_op: 0,
                idea_open: None,
                idea_snap: false,
                bt_prefs: App::read_prefs("backtest.json"),
                bt: None,
                movers: vec![],
                form: argos_render::formfactor::choose(&argos_render::formfactor::Screen {
                    w: 1280.0,
                    h: 800.0,
                    scale: 1.0,
                    ..Default::default()
                }),
                drawer_was_open: false,
                banner_since: None,
                rows: None,
                feuille_verre: false,
                rail_survol: false,
                rail_garde: false,
                chrome: (u32::MAX, u32::MAX),
                frame_ms: 0.0,
                glass_ms: 0.0,
                glass_now: 0.0,
                phases: [0.0; 5],
                image_avant: Vec::new(),
                image_taille: (0, 0),
                ouverts: [false; crate::state::OUVERTS],
                octets: Vec::new(),
                frames: 0,
                derniere_entree: Instant::now(),
                dernier_redessin_donnees: Instant::now(),
                redessin_en_attente: false,
                barre_par_defaut: std::cell::Cell::new(false),
                barre_auto_repliee: false,
                relache_avale: false,
                volet_large: false,
                fx_dans_volet: false,
                symboles: None,
                marches: None,
                eclair: None,
                hl_lignes: None,
                marches_survol: Default::default(),
                marches_horloge: Instant::now(),
                catalogues_btc: false,
                sym_recents: vec![],
                reglages: None,
                support: None,
                support_non_lues: 0,
                support_choix: false,
                bt_tv_choix: false,
                strat_vus: None,
                strat_cle: None,
                strat_ouvrir: false,
                support_lance: false,
                curseur: Default::default(),
                curseur_point: None,
                demo_traits: vec![],
                taille_test: None,
                pilule_repliee: false,
                pfeuille: None,
                croix: None,
                funding_carte: false,
                fond: None,
                image_test: vec![],
                couches_test: vec![],
                glass_off: false,
                glass_since: 0,
                glass_screen: (0.0, 0.0),
                pop: None,
                list: None,
                pane_symbole: None,
                ut_case: None,
                ai_open: false,
                sheet: None,
                argos: None,
                obj_tree: None,
                split_menu: None,
                sheet_closed: None,
                context: ContextMenu::default(),
                goto: None,
                add: None,
                name_box: None,
                layout_cible: None,
                depart_vu: false,
                obj_dlg: None,
                drawbar_off: (0.0, 0.0),
                colorpick: None,
                draw_alert: None,
                ind_dlg: None,
                j_form: None,
                pf_page: None,
                bt_page: None,
                mk_page: None,
                ac_page: None,
                accueil: None,
                accueil_note: false,
                accueil_ac: (0, 0, None),
                palette: None,
                fx_page: None,
                notifs: None,
                plan: None,
                roue: None,
                section: None,
                raccourcis: None,
                sim: None,
                sim_replay: false,
                brouillon: None,
                sim_last_t: 0,
                goto_replay: false,
                replay_vise: None,
                replay_ut: None,
                seance: None,
                avis: None,
                fp_reglages: false,
                ticket_zoom: None,
                par_dessus_note: None,
                mk_spark: vec![],
                ind_here: None,
                editor: None,
                delete_armed: false,
            },
            top: TopBar {
                layout: "Argos".into(),
                ..TopBar::default()
            },
            ai: Default::default(),
            session: state::SessionState {
                claude_key: None,
                key_set: Default::default(),
                oanda: None,
                cmc: None,
                key_box: None,
                account: None,
                acc_draft: None,
                words_box: None,
            },
            feeds: Default::default(),
            ticket_draw: None,
            rail: Default::default(),
            tool: "cursor",
            replay: None,
            area: (1400, 800),
            split: Default::default(),
            panes: vec![],
            replay_times: vec![],
            replay_at: Instant::now(),
            auto_saved: Instant::now(),
            banner: None,
            clipboard: None,
            watch: Watchlist::default(),
            logos_asked: Default::default(),
            store: None,
            lists: vec![],
            demo: Default::default(),
            alerts: Default::default(),
            alerts_draft: (0, 0),
            accounts: Default::default(),
            ind: state::IndState {
                leg: Default::default(),
                sel: None,
                on: vec![],
                out: Default::default(),
                key: (0, 0, 0, vec![]),
                favs: vec![],
                set: Default::default(),
                scripts_set: Default::default(),
                texts: Default::default(),
                tables: Default::default(),
                at: Instant::now(),
            },
            obj_clip: None,
            templates: Default::default(),
            tool_styles: Default::default(),
            ex_prefs: Default::default(),
            dock: state::DockState {
                rect: None,
                left: 0.0,
                right: 0.0,
                sous_sim: 0.0,
                bodies: vec![],
                scroll: Default::default(),
                places: vec![],
                total: 0.0,
                defil: 0.0,
                hauteurs: vec![],
                largeur: None,
                geste: None,
                poignee_avant: None,
            },
            scripts: vec![],
            mods: Default::default(),
            // 🧪 sans base : le journal reste en mémoire (les tests, et l'app jusqu'à `journal_charger`)
            jsync: crate::actions::JournalSync {
                pret: true,
                ..Default::default()
            },
            cat_loading: Default::default(),
            oanda_at: Instant::now(),
            gen: Arc::new(AtomicU64::new(0)),
            proxy: None,
            selected: None,
            draft: None,
            window: None,
            surface: None,
        }
    }
}

/// Charge les bougies en arrière-plan ; le résultat arrive à la fenêtre (`UserEvent::Loaded`).
pub(crate) fn start_load(source: Source, tf: Timeframe, label: &str, limit: usize, oanda: Option<(String, bool)>, proxy: EventLoopProxy<UserEvent>, gen: u64) {
    let label = label.to_string();
    argos_data::rt::spawn(move || async move {
        let res = match &source {
            Source::File(path) => load_file(path, &label),
            // ⚡ premier écran : haute priorité (la file basse priorité de cet exchange attend qu'il soit fini)
            Source::Live { exchange, base } => argos_data::file::haute(*exchange, load_live(*exchange, base, tf, &label, limit)).await,
            Source::Oanda { instrument } => match oanda {
                Some((token, live)) => argos_data::oanda::candles(instrument, tf, &token, live)
                    .await
                    .map(|bars| Candles {
                        bars,
                        title: format!("{} · OANDA · {label}", instrument.replace('_', "")),
                        banner: None,
                        primary: false,
                        secours: None,
                        a_reparer: vec![],
                    })
                    .map_err(|e| format!("OANDA : {e}")),
                None => Err("ajoute ton jeton OANDA dans 🔌 API".into()),
            },
            Source::Ratio { exchange, base, quote } => load_ratio(*exchange, base, quote, tf, &label, limit).await,
            Source::Synth { sym } => load_synth(sym, tf, &label, limit).await,
            Source::Cap { .. } => Err("les séries CoinMarketCap se lisent dans l'historique local".into()),
            // 🦎 GeckoTerminal (gratuit, 10 appels / min) : jusqu'à 1000 bougies de la pool
            // (CoinMarketCap avec la clé de l'utilisateur, sinon GeckoTerminal)
            Source::Dex { network, pool, name } => argos_data::dex::candles_any(&argos_data::rt::Client::default(), network, pool, tf)
                .await
                .map(|(bars, src)| Candles {
                    bars,
                    title: format!("{} · {src}", crate::actions::dex_title(name, network, &label)),
                    banner: None,
                    primary: false,
                    secours: None,
                    a_reparer: vec![],
                })
                .map_err(|e| format!("GeckoTerminal : {e}")),
        };
        let _ = proxy.send_event(UserEvent::Loaded(gen, res));
    });
}

/// Lance le direct en arrière-plan ; il s'arrête tout seul quand la fenêtre est fermée ou qu'on change de symbole.
/// 🔑 KuCoin : l'adresse de son flux demande d'abord un jeton public (sans clé), redemandé à chaque reconnexion.
pub(crate) fn start_live(ex: Exchange, direct: live::DirectChart, proxy: EventLoopProxy<UserEvent>, current: Arc<AtomicU64>, gen: u64) {
    let parse = move |t: &str| live::parse_event(ex, t);
    let on_event = move |ev| current.load(Ordering::SeqCst) == gen && proxy.send_event(UserEvent::Live(gen, ev)).is_ok();
    match direct {
        live::DirectChart::Pret(sub) => {
            live::spawn(sub, parse, on_event);
        }
        live::DirectChart::Jeton(coins) => live::spawn_jeton(move || argos_data::whales::kucoin_preparer(coins.clone()), parse, on_event),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fichier JSON temporaire, effacé à la fin du test.
    struct Tmp(std::path::PathBuf);

    impl Tmp {
        fn new(name: &str, body: &str) -> Tmp {
            let p = std::env::temp_dir().join(format!("argos-test-{}-{name}.json", std::process::id()));
            std::fs::write(&p, body).unwrap();
            Tmp(p)
        }

        fn path(&self) -> &str {
            self.0.to_str().unwrap()
        }
    }

    impl Drop for Tmp {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    #[test]
    fn cle_de_sauvegarde_par_symbole() {
        let live = Source::Live {
            exchange: Exchange::Binance,
            base: "BTC".into(),
        };
        assert_eq!(live.key(), "Binance-BTC");
        let oanda = Source::Oanda { instrument: "XAU_USD".into() };
        assert_eq!(oanda.key(), "oanda-XAU_USD");
        // un fichier : seulement son nom, sans dossier ni extension
        assert_eq!(Source::File("/tmp/bougies/eth.json".into()).key(), "fichier-eth");
    }

    #[test]
    fn json_lu_trie_et_volume_par_defaut() {
        let f = Tmp::new(
            "ok",
            r#"[{"time": 120, "open": 2, "high": 3, "low": 1, "close": 2.5},
                {"time": 60, "open": 1, "high": 2, "low": 0.5, "close": 1.5, "volume": 10}]"#,
        );
        let bars = load_json(f.path()).unwrap();
        assert_eq!(bars.len(), 2);
        // remises dans l'ordre du temps
        assert_eq!((bars[0].time, bars[0].close, bars[0].volume), (60, 1.5, 10.0));
        // volume absent = 0
        assert_eq!((bars[1].time, bars[1].close, bars[1].volume), (120, 2.5, 0.0));
    }

    #[test]
    fn json_sans_champ_obligatoire_refuse() {
        let f = Tmp::new("sans-close", r#"[{"time": 60, "open": 1, "high": 2, "low": 0.5}]"#);
        assert_eq!(load_json(f.path()).unwrap_err(), "champ « close » manquant");
        let f = Tmp::new("objet", r#"{"time": 60}"#);
        assert_eq!(load_json(f.path()).unwrap_err(), "le fichier doit contenir une liste de bougies");
    }

    #[test]
    fn fichier_lu_avec_son_nom_dans_le_titre() {
        let f = Tmp::new("titre", r#"[{"time": 60, "open": 1, "high": 2, "low": 0.5, "close": 1.5}]"#);
        let c = load_file(f.path(), "1 h").unwrap();
        assert_eq!(c.bars.len(), 1);
        assert!(c.title.ends_with(" · 1 h · fichier"), "{}", c.title);
        assert!(c.title.starts_with("argos-test-"), "{}", c.title);
        assert!(load_file("/nexiste/pas.json", "1 h").is_err());
    }
}
