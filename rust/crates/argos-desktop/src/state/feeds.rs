//! 📡 Flux de marché qui nourrissent les panneaux et les modules 🧩 : prix des watchlists, carnet, derniers
//! trades, fundings, baleines, marchés macro, liquidations, Agenda. Chaque flux a sa génération (`*_gen`) :
//! la changer arrête le fil d'arrière-plan en cours.

use std::sync::atomic::AtomicU64;
use std::sync::Arc;

use argos_data::watchlist as hl_watch;
use argos_data::Exchange;
use web_time::Instant;

/// 🔬 The loading of the scripts' lower-timeframe bars for one key (market, chart unit, wanted timeframes).
#[derive(Debug, Clone)]
pub(crate) struct ScriptMtf {
    pub(crate) key: String,
    /// The exchange asked (the one of the chart).
    pub(crate) exchange: Exchange,
    /// Last request started (`None`: none yet), one in flight, failed requests for this key (3 at most).
    pub(crate) at: Option<Instant>,
    pub(crate) en_cours: bool,
    pub(crate) echecs: u32,
    pub(crate) etats: std::collections::HashMap<i64, EtatPetites>,
}

/// Where the bars of one lower timeframe are (the editor's terminal says it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EtatPetites {
    Chargement,
    Chargees(usize),
    Indisponibles,
}

#[derive(Default)]
pub(crate) struct Feeds {
    /// 👛 Wallet en lecture seule : adresse publique, dernier état, dernière lecture, lecture en cours, erreur.
    pub(crate) wallet_addr: String,
    pub(crate) wallet: Option<argos_data::wallet::Snapshot>,
    pub(crate) wallet_at: Option<Instant>,
    pub(crate) wallet_busy: bool,
    pub(crate) wallet_note: String,
    /// 🐋 Dernière écriture de `whales.json`.
    pub(crate) whales_saved: Option<Instant>,
    /// 📊 Dernière lecture du flux acheteur / vendeur (bougies Binance).
    pub(crate) whale_flow_at: Option<Instant>,
    /// 🔝 Top 30 de chaque exchange (Baleines), lu une fois par jour.
    pub(crate) whale_top: std::collections::HashMap<String, Vec<String>>,
    pub(crate) whale_top_at: Option<Instant>,
    pub(crate) whale_top_busy: bool,
    /// 🏦 Catalogue OANDA en téléchargement.
    pub(crate) oanda_cat_busy: bool,
    /// 📊 Tickers 24 h de Binance et OKX (relus au plus à la minute) et ceux en cours de chargement.
    pub(crate) tickers: std::collections::HashMap<Exchange, (Instant, Vec<argos_data::tickers::Ticker>)>,
    pub(crate) tickers_busy: std::collections::HashSet<Exchange>,
    /// 📈 1-minute candles of the chart's crypto for the Calendrier's price reactions (crypto, candles) and their last
    /// reading (crypto, time).
    pub(crate) minutes: Option<(String, Vec<argos_core::Bar>)>,
    pub(crate) minutes_at: Option<(String, Instant)>,
    /// 📅 « Réel » of the calendar (BLS, Fed): the readings kept, a reading under way.
    pub(crate) reel: argos_data::reel::Reel,
    pub(crate) reel_busy: bool,
    /// 🦎 DEX : dernière recherche (texte, paires) ; carte du dock : dernier chargement, fiche et trades de la pool.
    pub(crate) dex_found: Option<(String, Vec<argos_data::dex::Pair>)>,
    pub(crate) dex_at: Option<Instant>,
    pub(crate) dex_info: Option<argos_data::dex::PoolInfo>,
    pub(crate) dex_trades: Vec<argos_data::dex::Trade>,
    /// 📡 Scanner double structure : dernières lignes, moment du dernier scan, scan en cours, état (« 12 / 17… »).
    pub(crate) scan: Vec<argos_render::dock::ScanLine>,
    pub(crate) scan_at: Option<Instant>,
    pub(crate) scan_busy: bool,
    pub(crate) scan_note: String,
    /// 📅 📰 Dernier chargement de l'Agenda et des nouvelles (rafraîchis aux 10 min / chaque minute quand le panneau est ouvert).
    pub(crate) eco_at: (Option<Instant>, Option<Instant>),
    /// 🐋 Pièce des flux des baleines en cours (`None` : arrêtés) ; changer `whale_gen` arrête les flux en cours.
    pub(crate) whale_coin: Option<String>,
    pub(crate) whale_gen: Arc<AtomicU64>,
    /// 👁 Dernière lecture du compte de la baleine suivie (adresse, moment) : relu aux 30 s, panneau ouvert.
    pub(crate) whale_acc_at: Option<(String, Instant)>,
    /// 🧭 Explorer Hyperliquid: when each reading was asked; ⭐ the alerts of the followed addresses (time, text).
    pub(crate) hlx: crate::actions::hyperliquid::HlTemps,
    pub(crate) hl_alertes: Vec<(i64, String)>,
    /// 🧩 Bougies des slots MTF de l'AI Trend, la clé (marché + UT + durées) et l'heure du dernier chargement.
    pub(crate) mtf_bars: argos_script::indicators::ai_trend::LowerBars,
    pub(crate) mtf_at: Option<(String, Instant)>,
    /// 🔬 Lower-timeframe bars loaded for the scripts' `request.security_lower_tf` (seconds → bars, oldest first),
    /// shared with every script run (`RunOptions::intrabars`), and how their loading goes. Apart from AI Trend's.
    pub(crate) script_bars: Arc<argos_script::lang::Intrabars>,
    pub(crate) script_mtf: Option<ScriptMtf>,
    /// Dernières sections de la liste Hyperliquid et tous les prix Hyperliquid (par nom de marché).
    pub(crate) hl_sections: Vec<hl_watch::Section>,
    pub(crate) quotes: std::collections::HashMap<String, hl_watch::Quote>,
    /// Prix des autres exchanges (par exchange, puis par marché) et les exchanges à rafraîchir (lu par le fil des prix).
    pub(crate) ex_prices: std::collections::HashMap<Exchange, std::collections::HashMap<String, argos_core::feed::Market>>,
    pub(crate) price_wanted: Arc<std::sync::Mutex<Vec<Exchange>>>,
    /// 🌐 « Tous les marchés » : la plateforme affichée (lue par son fil ; `None` : vue fermée, rien n'est lu), son
    /// dernier relevé (ou l'erreur) et les market caps CoinGecko (symbole → $).
    pub(crate) tous_wanted: Arc<std::sync::Mutex<Option<Exchange>>>,
    pub(crate) tous: Option<(Exchange, Result<Vec<argos_data::tous::Ligne>, String>)>,
    pub(crate) tous_caps: std::collections::HashMap<String, f64>,
    /// 📡 Hyperliquid markets whose prices the app wants live (its list shown, the chart, demo positions, alerts).
    pub(crate) hl_wanted: Arc<std::sync::Mutex<Vec<String>>>,
    /// 📡 Watchlist lines on screen at the last frame (ticker band, list rows in view).
    pub(crate) hl_visibles: Vec<String>,
    /// Symbole du panneau Marché crypto quand il est ouvert (sinon le fil est en pause).
    pub(crate) market_wanted: Arc<std::sync::Mutex<Option<String>>>,
    /// 📖 ⚡ 💸 Carnet, derniers trades (le plus récent d'abord) et fundings 24 h de la pièce du graphique (Hyperliquid) ;
    /// pièce du flux en cours (`None` : arrêté, changer `flow_gen` l'arrête) ; dernière lecture du funding.
    pub(crate) book: Option<argos_data::book::Book>,
    /// 🔥 Heatmap du carnet (historique en mémoire) et dernier redessin dû au carnet (au plus ~10 par seconde).
    pub(crate) heat: Option<argos_render::bookheat::BookHeat>,
    pub(crate) heat_draw: Option<Instant>,
    /// Regroupement du carnet demandé au flux (chiffres significatifs ; `None` : détaillé).
    pub(crate) flow_sig: Option<u8>,
    /// 💥 Carte des liquidations : données reçues (clé, par bourse), relevés de l'intérêt ouvert de Hyperliquid
    /// (heure, $), dernière demande, carte calculée et réglages avec lesquels elle l'a été.
    pub(crate) liq_key: String,
    pub(crate) liq_data: Vec<(argos_data::liqdata::Venue, Result<Vec<argos_core::liqmap::OiBar>, String>)>,
    pub(crate) liq_hl: Option<crate::LiqHl>,
    pub(crate) liq_at: Option<Instant>,
    pub(crate) liq_view: Option<argos_render::liqview::LiqView>,
    /// 🧱 Argos Murs: books, walls and this session's events (indicator on, crypto chart).
    pub(crate) murs: Option<Box<crate::actions::murs::MursState>>,
    /// 📖 Argos Murs: Binance perp's diff depth stream (the coin it follows; changing `murs_flux_gen` stops it).
    pub(crate) murs_flux: Option<String>,
    pub(crate) murs_flux_gen: Arc<AtomicU64>,
    /// 📖 🔥 Books of the heatmap and the order-book panel read as snapshots (a choice other than Hyperliquid USDC).
    pub(crate) carnets: Option<crate::actions::carnets::CarnetsState>,
    /// 📖 The order-book panel: Delta's moves, the middle price before, the DOM's centre.
    pub(crate) carnet: crate::actions::carnet::CarnetPanneau,
    /// ⚡ The Bitfinex perpetual of the chart: mark, funding, open interest.
    pub(crate) bitfinex_statut: Option<crate::actions::carnets::StatutBitfinex>,
    /// 🧱 Argos Murs recordings (desktop: `murs.redb`), opened the first time they are needed.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) murs_db: Option<crate::actions::murs::MursDb>,
    pub(crate) liq_sig: Vec<f64>,
    /// Relevés de l'intérêt ouvert de Hyperliquid par pièce (gardés dans liq_hl.json, bornés).
    pub(crate) liq_hl_oi: std::collections::HashMap<String, Vec<(i64, f64)>>,
    pub(crate) tape: Vec<argos_data::whales::Whale>,
    /// ⚡ The exchange and coin the trades of `tape` come from (`App::tape_source`).
    pub(crate) tape_de: Option<(Exchange, String)>,
    /// ⚡ The stream of `tape` was just cut or (re)connected: its next lot may give again trades the list holds
    /// (`App::tape_ajouter`).
    pub(crate) tape_apres_connexion: bool,
    pub(crate) funding_hist: Vec<argos_data::book::Funding>,
    pub(crate) flow_coin: Option<String>,
    /// 🧮 Orderflow réel des marchés suivis (celui de la chart et ● les enregistrés en continu) : trades rangés par
    /// minute et par prix ; la liste ● (gardée dans `orderflow.json`) ; ses pièces abonnées au flux en cours.
    pub(crate) orderflow: Vec<crate::actions::orderflow::FluxReel>,
    pub(crate) enregistres: argos_core::orderflow::Enregistres,
    pub(crate) flow_autres: Vec<String>,
    /// 🏦 Flux de trades des autres bourses branchées (bourse, pièces abonnées) ; changer `orderflow_gen` les arrête.
    pub(crate) orderflow_flux: Vec<(Exchange, Vec<String>)>,
    pub(crate) orderflow_gen: Arc<AtomicU64>,
    /// 🧮 Bourses à contrats dont les tailles de contrat ont déjà été demandées pour l'orderflow réel.
    pub(crate) contrats_demandes: std::collections::HashSet<Exchange>,
    /// 🏦 Pièces dont la taille du contrat OKX a déjà été demandée pour le footprint multi-bourses.
    pub(crate) fp_ctval: std::collections::HashSet<String>,
    /// 🧮 Bourses dont le flux ne s'ouvre pas pour l'instant (jeton refusé…) : l'estimé, jusqu'à la prochaine connexion.
    pub(crate) flux_indisponibles: std::collections::HashSet<Exchange>,
    pub(crate) flow_gen: Arc<AtomicU64>,
    pub(crate) funding_at: Option<(String, Instant)>,
    /// ⚡ Taux de funding réels des perpétuels démo (pièce → bourse et taux pour 8 h), lus à la bourse de la chart aux
    /// 5 min ; la dernière lecture (bourse, pièce, heure).
    pub(crate) funding_demo: std::collections::HashMap<String, (Exchange, f64)>,
    pub(crate) funding_demo_at: Option<(Exchange, String, Instant)>,
    /// 🌍 Marchés macro (tuiles, message, dernière lecture) ; 💥 liquidations (la plus récente d'abord), flux en marche,
    /// changer `liq_gen` l'arrête.
    pub(crate) world: Vec<argos_data::world::Tile>,
    pub(crate) world_note: String,
    pub(crate) world_at: Option<Instant>,
    pub(crate) liqs: Vec<argos_data::liq::Liq>,
    pub(crate) liq_on: bool,
    pub(crate) liq_gen: Arc<AtomicU64>,
    /// 📈 CoinMarketCap : réglage du fil de fond (adresse, clé ; `None` sans clé), relevés gardés (un toutes les
    /// 5 min, dans `cmc_history.json`) et la dernière erreur.
    pub(crate) cmc_wanted: argos_data::cmcap::Wanted,
    pub(crate) cmc_hist: Vec<argos_data::cmcap::CapPoint>,
    /// 🧮 Crédits CoinMarketCap du mois (partagés avec les fils de fond, gardés dans `cmc_credits.json`).
    pub(crate) cmc_credits: argos_data::cmcap::CreditBox,
    /// 📈 Intervalle du relevé et ⟳ manuel (partagés avec le fil) ; dernier relevé CoinMarketCap (gardé dans
    /// `cmc_last.json` : peur & avidité, saison des altcoins et marché de CMC, même après un redémarrage).
    pub(crate) cmc_ctl: std::sync::Arc<argos_data::cmcap::Ctl>,
    pub(crate) cmc_last: Option<argos_data::cmcap::CapPoint>,
    pub(crate) cmc_note: String,
}
