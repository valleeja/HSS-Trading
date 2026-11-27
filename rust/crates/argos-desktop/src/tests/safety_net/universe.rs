//! 🕸 Every screen the app can show, from the code's own lists, and what is on screen in an app.
//!
//! A screen has an id (« panel:Trading », « module:Book », « ui:editor », « list:Styles »…). [`all`] gives the ids
//! each form must have a scene for; [`shown`] reads the same ids on a drawn app. The lists are the code's: its
//! `ALL` constants where it has them, and for the others a `match` or a destructuring that names every variant or
//! field, so a new one does not compile until it is listed here. The messages the app receives from its other
//! threads have ids too (« answer:Wallet », [`ANSWERS`]): a scene shows one when its app received it
//! (`furniture::deliver`).

use super::Form;
use crate::actions::hyperliquid::HlLu;
use crate::state::{ChartState, DockState, Feeds, IndState, PointerState, SessionState, UiState};
use crate::*;
use argos_render::actions::ChartAction;
use argos_render::ecrans::CellHit;
use argos_render::topbar::TopClick;
use std::collections::BTreeSet;

/// An id: the family, then the name without spaces.
pub(super) fn id(family: &str, name: &str) -> String {
    format!("{family}:{}", name.split_whitespace().collect::<Vec<_>>().join("_"))
}

/// An id from a value's `Debug` name.
pub(super) fn id_of(family: &str, value: &impl std::fmt::Debug) -> String {
    id(family, &format!("{value:?}"))
}

/// The variants of an enum without data, kept complete by the compiler.
macro_rules! every {
    ($ty:ty: $($variant:ident),+ $(,)?) => {{
        // (a variant added to the enum does not compile here until it is listed)
        let complete = |v: $ty| match v {
            $(<$ty>::$variant => {}),+
        };
        let all = [$(<$ty>::$variant),+];
        all.iter().for_each(|v| complete(*v));
        all
    }};
}
pub(super) use every;

/// A name for each variant of an enum with data (the data left out), and the list of those names.
macro_rules! named {
    ($(#[$doc:meta])* $names:ident, $name_of:ident, $ty:ty: $($pattern:pat => $name:literal),+ $(,)?) => {
        $(#[$doc])*
        pub(super) const $names: &[&str] = &[$($name),+];
        pub(super) fn $name_of(v: &$ty) -> &'static str {
            match v {
                $($pattern => $name),+
            }
        }
    };
}

/// What makes a field of the app's states a screen: an `Option` that is set, a `bool` that is on.
trait Lit {
    fn lit(&self) -> bool;
}

impl<T> Lit for Option<T> {
    fn lit(&self) -> bool {
        self.is_some()
    }
}

impl Lit for bool {
    fn lit(&self) -> bool {
        *self
    }
}

/// The fields of one of the app's states, written as the pattern that takes the state apart: a field that is a
/// screen when it is set (an `Option`) or on (a `bool`) by its name alone, any other field as `name: _`.
///
/// Every field is named, so a new one does not compile until it is written here: the compiler says « pattern does
/// not mention field », names the field and shows the line to add (`name: _` when it is not a screen). `..` is
/// refused: it would let the next new field through without a word.
///
/// (The pattern goes through the macro token by token, so the compiler reads it where it is written. Put together
/// inside the macro, a missing field only gave « pattern requires `..` due to inaccessible fields », without its
/// name.)
macro_rules! fields {
    ($screens:ident, $shown:ident, $family:literal, $($pattern:tt)+) => {
        fields!(@all $screens, $shown, $family, [$($pattern)+] $($pattern)+);
    };
    (@all $screens:ident, $shown:ident, $family:literal, [$($pattern:tt)+]
     $ty:tt { $($field:ident $(: $not:tt)?),* $(,)? }) => {
        fn $screens() -> Vec<&'static str> {
            let mut names = vec![];
            $(fields!(@name names, $field $(: $not)?);)*
            names
        }
        fn $shown(state: &$ty, shown: &mut BTreeSet<String>) {
            let $($pattern)+ = state;
            $(fields!(@shown shown, $family, $field $(: $not)?);)*
        }
    };
    (@name $names:ident, $field:ident: _) => {};
    (@name $names:ident, $field:ident) => {
        $names.push(stringify!($field));
    };
    (@shown $shown:ident, $family:literal, $field:ident: _) => {};
    (@shown $shown:ident, $family:literal, $field:ident) => {
        if Lit::lit($field) {
            $shown.insert(id($family, stringify!($field)));
        }
    };
}

fields! {
    ui_screens, ui_shown, "ui",
    UiState {
        // a screen when it is set (a new window, sheet or page is also added to `App::par_dessus` in render.rs: the
        // quality gate's browser step would not see it covering the chart otherwise, bug B-041)
        explication, alerte_sel, confirmer, plein_ecran, bulle_plein, idea_open, symboles, marches, reglages, support,
        pfeuille, croix, pop, list, sheet, argos, obj_tree, split_menu, goto, obj_dlg, draw_alert, colorpick, ind_dlg,
        j_form, pf_page, bt_page, mk_page, ac_page, editor, add, name_box, accueil, palette, fx_page, notifs, plan,
        roue, raccourcis, sim, brouillon, avis, eclair, hl_lignes,
        // a screen when it is on
        zen, zen_haut, ai_open, rail_survol, volet_large, pilule_repliee, fp_reglages, funding_carte,
        // not a screen
        alerte_survol: _, plein_cible: _, plein_zen_avant: _, ai_mem: _, ai_clear_armed: _, ideas: _, ideas_delete: _,
        alert_line: _, alert_op: _, idea_snap: _, bt_prefs: _, bt: _, movers: _, form: _, banner_since: _,
        drawer_was_open: _, rows: _, feuille_verre: _, rail_garde: _, chrome: _, frame_ms: _, glass_ms: _,
        glass_now: _, phases: _, image_avant: _, image_taille: _, ouverts: _, octets: _, frames: _,
        derniere_entree: _, dernier_redessin_donnees: _, redessin_en_attente: _, barre_par_defaut: _,
        barre_auto_repliee: _, relache_avale: _, fx_dans_volet: _, marches_survol: _, marches_horloge: _,
        sym_recents: _, support_non_lues: _, support_choix: _, support_lance: _, curseur: _, curseur_point: _,
        demo_traits: _, taille_test: _, fond: _, image_test: _, couches_test: _, glass_off: _, glass_since: _,
        glass_screen: _, pane_symbole: _, ut_case: _, sheet_closed: _, context: _, drawbar_off: _, mk_spark: _,
        ind_here: _, layout_cible: _, depart_vu: _, delete_armed: _, accueil_note: _, accueil_ac: _, section: _,
        sim_replay: _, sim_last_t: _, goto_replay: _, replay_vise: _, replay_ut: _, seance: _, bt_tv_choix: _,
        ticket_zoom: _, strat_vus: _, strat_cle: _, strat_ouvrir: _, catalogues_btc: _, par_dessus_note: _,
    }
}

fields! {
    app_screens, app_shown, "app",
    App {
        // a screen when it is set
        replay, banner, selected, draft, ticket_draw,
        // not a screen
        chart: _, feeds: _, ind: _, pointer: _, ui: _, dock: _, session: _, tools: _, top: _, ai: _, rail: _, tool: _,
        area: _, split: _, panes: _, replay_times: _, replay_at: _, auto_saved: _, title: _, watch: _, logos_asked: _,
        store: _, lists: _, demo: _, alerts: _, alerts_draft: _, accounts: _, obj_clip: _, templates: _,
        tool_styles: _, ex_prefs: _, scripts: _, mods: _, jsync: _, cat_loading: _, oanda_at: _, gen: _, proxy: _,
        clipboard: _, window: _, surface: _,
    }
}

fields! {
    session_screens, session_shown, "session",
    SessionState {
        // a screen when it is set
        key_box, words_box,
        // not a screen
        claude_key: _, key_set: _, oanda: _, cmc: _, account: _, acc_draft: _,
    }
}

/// The other states of the app (the chart, the column of modules, the data received, the indicators, the pointer),
/// every field of them written as `name: _`. None is a screen by its name: what they hold reaches the screen
/// through the lists of this file, the presses ([`super::windows`]) and the answers ([`super::answers`]). They are
/// written here so that a field added to one of them stops the build, here, and names itself (« pattern does not
/// mention field … »): who adds it writes it in the list, after asking what shows it. A field that changes what a
/// screen shows needs a scene that sets it (the figures of a Bitfinex perpetual, a table selected by the finger): in
/// [`super::scenes`], or in [`super::answers`] when it comes from a message. `..` is refused.
macro_rules! watched {
    ($(($state:ident: $ty:ident) [$($pattern:tt)+])*) => {
        #[allow(dead_code)]
        fn watched($($state: &$ty),*) {
            $(
                watched!(@fields $($pattern)+);
                let $($pattern)+ = $state;
            )*
        }
    };
    (@fields $ty:tt { $($(#[$only:meta])? $field:ident: _),* $(,)? }) => {};
}

watched! {
    (chart: ChartState) [ChartState {
        source: _, bars: _, slots: _, styles: _, style: _, interval: _, raw: _, file_interval: _, viewport: _,
        settings: _, save_path: _, last_saved: _, drawings: _, hist: _, live: _, loading: _, cache_path: _,
        older_busy: _, history_start: _, older_key: _, older_at: _, older_via: _, a_reparer: _, repair_busy: _,
        trous_connus: _, hors_ligne: _, coupe: _, debut_propre: _,
    }]
    (dock: DockState) [DockState {
        rect: _, left: _, right: _, sous_sim: _, bodies: _, scroll: _, places: _, total: _, defil: _, hauteurs: _,
        largeur: _, geste: _, poignee_avant: _,
    }]
    (feeds: Feeds) [Feeds {
        wallet_addr: _, wallet: _, wallet_at: _, wallet_busy: _, wallet_note: _, whales_saved: _, whale_flow_at: _,
        whale_top: _, whale_top_at: _, whale_top_busy: _, oanda_cat_busy: _, tickers: _, tickers_busy: _, minutes: _,
        minutes_at: _, reel: _, reel_busy: _, dex_found: _, dex_at: _, dex_info: _, dex_trades: _, scan: _,
        scan_at: _, scan_busy: _, scan_note: _, eco_at: _, whale_coin: _, whale_gen: _, whale_acc_at: _, hlx: _,
        hl_alertes: _, mtf_bars: _, mtf_at: _, script_bars: _, script_mtf: _, hl_sections: _, quotes: _,
        ex_prices: _, price_wanted: _, tous_wanted: _, tous: _, tous_caps: _, hl_wanted: _, hl_visibles: _,
        market_wanted: _, book: _, heat: _, heat_draw: _, flow_sig: _, liq_key: _, liq_data: _, liq_hl: _, liq_at: _,
        liq_view: _, murs: _, murs_flux: _, murs_flux_gen: _, carnets: _, carnet: _, bitfinex_statut: _,
        #[cfg(not(target_arch = "wasm32"))]
        murs_db: _,
        liq_sig: _, liq_hl_oi: _, tape: _, tape_de: _, funding_hist: _, flow_coin: _, orderflow: _, enregistres: _,
        tape_apres_connexion: _,
        flow_autres: _, orderflow_flux: _, orderflow_gen: _, contrats_demandes: _, fp_ctval: _,
        flux_indisponibles: _, flow_gen: _, funding_at: _, funding_demo: _, funding_demo_at: _, world: _,
        world_note: _, world_at: _, liqs: _, liq_on: _, liq_gen: _, cmc_wanted: _, cmc_hist: _, cmc_credits: _,
        cmc_ctl: _, cmc_last: _, cmc_note: _,
    }]
    (indicators: IndState) [IndState {
        leg: _, sel: _, on: _, out: _, key: _, favs: _, set: _, scripts_set: _, texts: _, tables: _, at: _,
    }]
    (pointer: PointerState) [PointerState {
        tool_drag: _, dlg_drag: _, tool_moved: _, bar_press: _, win_mouse: _, top_mouse: _, phone_mouse: _,
        wl_resize: _, rail_mouse: _, mouse: _, drag: _, pane_drag: _, pane_click_at: _, axis_drag: _,
        last_axis_click: _, last_time_click: _, murs_appui: _, last_chart_click: _, triple: _, appui: _,
        appui_avant: _, appui_croix: _, mods: _, grab: _, drawbar_drag: _, cp_drag: _, curseur_ticket: _, cursor: _,
        point_pose: _, side_mouse: _, mark_drag: _, band_drag: _, script_drag: _, band_pan: _, band_zoom: _,
        script_pan: _, script_zoom: _, dock_mouse: _, mk_drag: _, touch: _, dernier_doigt: _, doigt_leve: _, fx_appui: _, finger_scroll: _,
        bar_swipe: _, doc_pan: _, icon_drag: _, fling: _, alert_drag: _, alert_appui: _, order_drag: _, order_err: _,
        tab: _, appui_glisse: _, trace_doigt: _, tour_debut: _, t_geste: _,
    }]
}

/// Every tool of the right column and of the phone's sheets (`Tool::ALL` only has the ones of the default bar).
pub(super) fn tools() -> [wl::Tool; 27] {
    every!(wl::Tool: Watchlist, Indicators, Trading, Journal, Exchanges, Marketplace, Market, Ai, Agenda, News, Whales,
        Modules, Alertes, Support, Dessin, Gardien, Script, Academie, Communaute, Portfolio, Liquidations, Reglages,
        Demo, Carnet, Explorer, Stats, Strategie)
}

named! {
    /// The list menus (`ListKind`).
    LISTS, list_name, ListKind:
    ListKind::DockAutres => "DockAutres",
    ListKind::Accueil => "Accueil",
    ListKind::Curseurs => "Curseurs",
    ListKind::Intervals => "Intervals",
    ListKind::Section => "Section",
    ListKind::IndPlus => "IndPlus",
    ListKind::PaneIntervals(_) => "PaneIntervals",
    ListKind::Styles => "Styles",
    ListKind::Theme => "Theme",
    ListKind::RailGroup(_) => "RailGroup",
    ListKind::RailButton(_) => "RailButton",
    ListKind::Lists => "Lists",
    ListKind::TousBourse => "TousBourse",
    ListKind::TousTri => "TousTri",
    ListKind::Timezone => "Timezone",
    ListKind::Drawing(_) => "Drawing",
    ListKind::DrawBar(..) => "DrawBar",
    ListKind::AddApi => "AddApi",
    ListKind::ToolMenu(_) => "ToolMenu",
    ListKind::Top(_) => "Top",
    ListKind::LangConfirm(_) => "LangConfirm",
    ListKind::Alerts => "Alerts",
    ListKind::Accounts => "Accounts",
    ListKind::Movers => "Movers",
    ListKind::Backtest => "Backtest",
    ListKind::AiMemory => "AiMemory",
    ListKind::Wallet => "Wallet",
    ListKind::IconBar(_) => "IconBar",
    ListKind::Community => "Community",
    ListKind::Modules => "Modules",
    ListKind::TuileLayout(_) => "TuileLayout",
    ListKind::SupprimerLayout(_) => "SupprimerLayout",
    ListKind::AlertLine(_) => "AlertLine",
    ListKind::SonAlerte(_) => "SonAlerte",
    ListKind::SonAlertes => "SonAlertes",
    ListKind::Strategies => "Strategies",
    ListKind::Carnet(_) => "Carnet",
    ListKind::ReglagesAlertes => "ReglagesAlertes",
}

named! {
    /// The windows of tiles of the favourites bar and of the top bar (`toolbar::PopKind`).
    POPS, pop_name, toolbar::PopKind:
    toolbar::PopKind::Tf => "Tf",
    toolbar::PopKind::Group(_) => "Group",
    toolbar::PopKind::Emoji => "Emoji",
    toolbar::PopKind::Style => "Style",
    toolbar::PopKind::Ind => "Ind",
}

named! {
    /// The sheets of the phone's pill (`PKind`).
    PILLS, pill_name, crate::actions::pilule::PKind:
    crate::actions::pilule::PKind::Argos => "Argos",
    crate::actions::pilule::PKind::Dessins => "Dessins",
    crate::actions::pilule::PKind::Trading => "Trading",
    crate::actions::pilule::PKind::Duo(_) => "Duo",
    crate::actions::pilule::PKind::Croix(_) => "Croix",
    crate::actions::pilule::PKind::Reglages => "Reglages",
}

named! {
    /// What the chart can be asked to do (`ChartAction`: keyboard, menus of the chart).
    CHART_ACTIONS, chart_action_name, ChartAction:
    ChartAction::ResetView => "ResetView",
    ChartAction::ResetTime => "ResetTime",
    ChartAction::ZoomIn => "ZoomIn",
    ChartAction::ZoomOut => "ZoomOut",
    ChartAction::ScrollLeft => "ScrollLeft",
    ChartAction::ScrollRight => "ScrollRight",
    ChartAction::ScrollLeftFast => "ScrollLeftFast",
    ChartAction::ScrollRightFast => "ScrollRightFast",
    ChartAction::GoStart => "GoStart",
    ChartAction::GoEnd => "GoEnd",
    ChartAction::ShowRange(..) => "ShowRange",
    ChartAction::CenterOn(_) => "CenterOn",
    ChartAction::ResetPrice => "ResetPrice",
    ChartAction::AutoScale => "AutoScale",
    ChartAction::LockRatio => "LockRatio",
    ChartAction::Invert => "Invert",
    ChartAction::Mode(_) => "Mode",
    ChartAction::TogglePercent => "TogglePercent",
    ChartAction::ToggleLog => "ToggleLog",
    ChartAction::Toggle(_) => "Toggle",
    ChartAction::SetTimezone(_) => "SetTimezone",
    ChartAction::TpoBlock(_) => "TpoBlock",
    ChartAction::TpoVa(_) => "TpoVa",
    ChartAction::TpoIb(_) => "TpoIb",
    ChartAction::GoTo => "GoTo",
    ChartAction::WatchPrev => "WatchPrev",
    ChartAction::WatchNext => "WatchNext",
    ChartAction::Reload => "Reload",
    ChartAction::NextStyle => "NextStyle",
    ChartAction::PrevStyle => "PrevStyle",
    ChartAction::Undo => "Undo",
    ChartAction::Redo => "Redo",
    ChartAction::CloseMenus => "CloseMenus",
    ChartAction::HLineAtMouse => "HLineAtMouse",
    ChartAction::HLine(_) => "HLine",
    ChartAction::AddAlert(_) => "AddAlert",
    ChartAction::Brouillon(..) => "Brouillon",
    ChartAction::CopyPrice(_) => "CopyPrice",
    ChartAction::ClearDrawings => "ClearDrawings",
    ChartAction::CopyDrawing => "CopyDrawing",
    ChartAction::PasteDrawing => "PasteDrawing",
    ChartAction::ObjectTree => "ObjectTree",
    ChartAction::PleinEcran => "PleinEcran",
}

named! {
    /// The buttons of the PC's top bar (`TopClick`).
    TOP_CLICKS, top_click_name, TopClick:
    TopClick::None => "None",
    TopClick::ReplierBandeaux => "ReplierBandeaux",
    TopClick::Recherche => "Recherche",
    TopClick::Style(_) => "Style",
    TopClick::Indicateurs(_) => "Indicateurs",
    TopClick::Alertes(_) => "Alertes",
    TopClick::Partager => "Partager",
    TopClick::Gardien => "Gardien",
    TopClick::Support => "Support",
    TopClick::Changed => "Changed",
    TopClick::Open(_) => "Open",
    TopClick::Symbol => "Symbol",
    TopClick::Home => "Home",
    TopClick::Intervals(_) => "Intervals",
    TopClick::Interval(_) => "Interval",
    TopClick::Menu(..) => "Menu",
}

named! {
    /// The zones of the phone's main screen (`CellHit`).
    CELL_HITS, cell_hit_name, CellHit:
    CellHit::Accueil => "Accueil",
    CellHit::Argos => "Argos",
    CellHit::MiseEnPage => "MiseEnPage",
    CellHit::Alertes => "Alertes",
    CellHit::Disposition => "Disposition",
    CellHit::Partager => "Partager",
    CellHit::Recherche => "Recherche",
    CellHit::Plan => "Plan",
    CellHit::Demo => "Demo",
    CellHit::Flux => "Flux",
    CellHit::Symbole => "Symbole",
    CellHit::SymbolePrec => "SymbolePrec",
    CellHit::SymboleSuiv => "SymboleSuiv",
    CellHit::Unite => "Unite",
    CellHit::UnitePrec => "UnitePrec",
    CellHit::UniteSuiv => "UniteSuiv",
    CellHit::Fx => "Fx",
    CellHit::Ai => "Ai",
    CellHit::Icone(_) => "Icone",
    CellHit::Autres => "Autres",
    CellHit::Dessins => "Dessins",
    CellHit::Trading => "Trading",
    CellHit::Plus => "Plus",
    CellHit::Replier => "Replier",
    CellHit::Rouvrir => "Rouvrir",
}

named! {
    /// The messages the app receives from its other threads (`UserEvent`), by what `App::on_user_event` does with
    /// them: the kind of message and, where the app does something else with it, its answer (« -err »: a failure) or
    /// the event it carries (« -other »: the events that kind of message ignores).
    ANSWERS, answer_name, UserEvent:
    UserEvent::Loaded(_, Ok(_)) => "Loaded",
    UserEvent::Loaded(_, Err(_)) => "Loaded-err",
    UserEvent::Older(_, Ok(_)) => "Older",
    UserEvent::Older(_, Err(_)) => "Older-err",
    UserEvent::Repaired(..) => "Repaired",
    UserEvent::Live(_, LiveEvent::Connected) => "Live-Connected",
    UserEvent::Live(_, LiveEvent::Bar(_)) => "Live-Bar",
    UserEvent::Live(_, LiveEvent::Trades(_)) => "Live-Trades",
    UserEvent::Live(_, LiveEvent::Disconnected(_)) => "Live-Disconnected",
    UserEvent::Live(..) => "Live-other",
    UserEvent::Quotes(..) => "Quotes",
    UserEvent::Prices(..) => "Prices",
    UserEvent::Tous(_, Ok(_)) => "Tous",
    UserEvent::Tous(_, Err(_)) => "Tous-err",
    UserEvent::TousCaps(_) => "TousCaps",
    UserEvent::Catalog(..) => "Catalog",
    UserEvent::Logo(..) => "Logo",
    UserEvent::Market(_) => "Market",
    UserEvent::Pane(_, Ok(_)) => "Pane",
    UserEvent::Pane(_, Err(_)) => "Pane-err",
    UserEvent::LangPack(_, Ok(_), _) => "LangPack",
    UserEvent::LangPack(_, Err(_), _) => "LangPack-err",
    UserEvent::Emoji(..) => "Emoji",
    UserEvent::Paste(_) => "Paste",
    UserEvent::Finger(..) => "Finger",
    UserEvent::Ai(Ok(_)) => "Ai",
    UserEvent::Ai(Err(_)) => "Ai-err",
    UserEvent::Agenda(Ok(_)) => "Agenda",
    UserEvent::Agenda(Err(_)) => "Agenda-err",
    UserEvent::MtfBars(..) => "MtfBars",
    UserEvent::ScriptBars(..) => "ScriptBars",
    UserEvent::OandaTest(Ok(_)) => "OandaTest",
    UserEvent::OandaTest(Err(_)) => "OandaTest-err",
    UserEvent::Cmc(Ok(_)) => "Cmc",
    UserEvent::Cmc(Err(_)) => "Cmc-err",
    UserEvent::CmcTest(Ok(_)) => "CmcTest",
    UserEvent::CmcTest(Err(_)) => "CmcTest-err",
    UserEvent::CleCmc(Ok(_), _) => "CleCmc",
    UserEvent::CleCmc(Err(_), _) => "CleCmc-err",
    UserEvent::Support(_, _, Ok(_)) => "Support",
    UserEvent::Support(_, _, Err(_)) => "Support-err",
    UserEvent::Passkey(Ok(_), _) => "Passkey",
    UserEvent::Passkey(Err(_), _) => "Passkey-err",
    UserEvent::News(Ok(_)) => "News",
    UserEvent::News(Err(_)) => "News-err",
    UserEvent::Whales(LiveEvent::Whales(_)) => "Whales",
    UserEvent::Whales(LiveEvent::Connected) => "Whales-Connected",
    UserEvent::Whales(LiveEvent::Disconnected(_)) => "Whales-Disconnected",
    UserEvent::Whales(LiveEvent::Indisponible(_)) => "Whales-Indisponible",
    UserEvent::Whales(_) => "Whales-other",
    UserEvent::Flow(LiveEvent::Book(_)) => "Flow-Book",
    UserEvent::Flow(LiveEvent::Tape(_)) => "Flow-Tape",
    UserEvent::Flow(LiveEvent::Connected | LiveEvent::Disconnected(_)) => "Flow-cut",
    UserEvent::Flow(_) => "Flow-other",
    UserEvent::Orderflow(_, LiveEvent::Whales(_)) => "Orderflow",
    UserEvent::Orderflow(_, LiveEvent::Connected) => "Orderflow-Connected",
    UserEvent::Orderflow(_, LiveEvent::Disconnected(_)) => "Orderflow-Disconnected",
    UserEvent::Orderflow(_, LiveEvent::Indisponible(_)) => "Orderflow-Indisponible",
    UserEvent::Orderflow(..) => "Orderflow-other",
    UserEvent::OrderflowContrats(_, Ok(())) => "OrderflowContrats",
    UserEvent::OrderflowContrats(_, Err(_)) => "OrderflowContrats-err",
    UserEvent::World(Ok(_)) => "World",
    UserEvent::World(Err(_)) => "World-err",
    UserEvent::Liq(LiveEvent::Liqs(_)) => "Liq",
    UserEvent::Liq(_) => "Liq-other",
    UserEvent::FundingHist(_, Ok(_)) => "FundingHist",
    UserEvent::FundingHist(_, Err(_)) => "FundingHist-err",
    UserEvent::FundingDemo(_, _, Some(_)) => "FundingDemo",
    UserEvent::FundingDemo(_, _, None) => "FundingDemo-none",
    UserEvent::LiqMap(..) => "LiqMap",
    UserEvent::Murs(..) => "Murs",
    UserEvent::MursFlux(..) => "MursFlux",
    UserEvent::Carnets(..) => "Carnets",
    UserEvent::CarnetBitfinex(_, Ok(_)) => "CarnetBitfinex",
    UserEvent::CarnetBitfinex(_, Err(_)) => "CarnetBitfinex-err",
    UserEvent::StatutBitfinex(_, Ok(Some(_))) => "StatutBitfinex",
    UserEvent::StatutBitfinex(_, Ok(None)) => "StatutBitfinex-none",
    UserEvent::StatutBitfinex(_, Err(_)) => "StatutBitfinex-err",
    UserEvent::RelaisBourse(true) => "RelaisBourse",
    UserEvent::RelaisBourse(false) => "RelaisBourse-none",
    UserEvent::WhaleAccount(_, Ok(_)) => "WhaleAccount",
    UserEvent::WhaleAccount(_, Err(_)) => "WhaleAccount-err",
    UserEvent::OandaCatalog(Ok(_)) => "OandaCatalog",
    UserEvent::OandaCatalog(Err(_)) => "OandaCatalog-err",
    UserEvent::WhaleTop(_) => "WhaleTop",
    UserEvent::WhaleFlow(_, Ok(_)) => "WhaleFlow",
    UserEvent::WhaleFlow(_, Err(_)) => "WhaleFlow-err",
    UserEvent::Wallet(_, Ok(_)) => "Wallet",
    UserEvent::Wallet(_, Err(_)) => "Wallet-err",
    UserEvent::Tickers(_, Ok(_)) => "Tickers",
    UserEvent::Tickers(_, Err(_)) => "Tickers-err",
    UserEvent::DexFound(_, Ok(_)) => "DexFound",
    UserEvent::DexFound(_, Err(_)) => "DexFound-err",
    UserEvent::DexPool(..) => "DexPool",
    UserEvent::Scan(..) => "Scan",
    UserEvent::Reglages(_) => "Reglages",
    UserEvent::SoftKey(_) => "SoftKey",
    UserEvent::CacheLu(..) => "CacheLu",
    UserEvent::Minutes(..) => "Minutes",
    UserEvent::Reel(..) => "Reel",
    UserEvent::Hl(_, HlLu::Compte(Ok(_))) => "Hl-Compte",
    UserEvent::Hl(_, HlLu::Compte(Err(_))) => "Hl-Compte-err",
    UserEvent::Hl(_, HlLu::Ordres(Ok(_))) => "Hl-Ordres",
    UserEvent::Hl(_, HlLu::Ordres(Err(_))) => "Hl-Ordres-err",
    UserEvent::Hl(_, HlLu::Fills(Ok(_))) => "Hl-Fills",
    UserEvent::Hl(_, HlLu::Fills(Err(_))) => "Hl-Fills-err",
    UserEvent::Hl(_, HlLu::Funding(Ok(_))) => "Hl-Funding",
    UserEvent::Hl(_, HlLu::Funding(Err(_))) => "Hl-Funding-err",
    UserEvent::Hl(_, HlLu::Portfolio(Ok(_))) => "Hl-Portfolio",
    UserEvent::Hl(_, HlLu::Portfolio(Err(_))) => "Hl-Portfolio-err",
    UserEvent::Hl(_, HlLu::Suivi(Ok(_))) => "Hl-Suivi",
    UserEvent::Hl(_, HlLu::Suivi(Err(_))) => "Hl-Suivi-err",
}

/// What the 🔔 shows (`Notifs::fil`): the alerts panel, or the feed of notifications.
const NOTIFS: [&str; 2] = ["alertes", "fil"];

/// Screens the phone does not have, and why: an id, or a whole family (« module: »).
const PC_ONLY: &[(&str, &str)] = &[
    ("module:", "the column of modules is for wide windows (`DockPrefs::active`)"),
    (
        "panel:Alertes",
        "🔔 is a sheet on the phone (`CellHit::Alertes`); the Alertes volet is the PC's (`App::notifs_pop`)",
    ),
    (
        "ui:symboles",
        "the Symboles volet opens from the PC's top bar (`TopClick::Symbol`); the phone has its markets sheet",
    ),
    (
        "ui:reglages",
        "the Réglages window is for the mouse and the tablet; the phone has its Réglages sheet (`App::reglages_doigt`)",
    ),
    ("ui:support", "« Le Support s'ouvre sur l'ordinateur pour l'instant » (`App::support_ouvrir`)"),
    (
        "answer:Support",
        "the phone never asks the PC's server for the tickets of the Support (`App::support_tick`)",
    ),
    ("answer:Support-err", "as answer:Support"),
    (
        "ui:zen_haut",
        "the top bar comes back under the mouse in Zen: the mouse's only (`App::mouse_moved`)",
    ),
    ("ui:rail_survol", "the left bar unfolds under the mouse: the mouse's only (`App::mouse_moved`)"),
    ("pop:Group", "a window of the favourites bar, which the single pill replaces (`App::pilule`)"),
    (
        "pop:Style",
        "a window of the favourites bar and of the top bar, which the single pill replaces (`App::pilule`)",
    ),
    (
        "pop:Ind",
        "a window of the favourites bar and of the top bar, which the single pill replaces (`App::pilule`)",
    ),
];
/// Screens the PC does not have, and why.
const PHONE_ONLY: &[(&str, &str)] = &[
    ("pill:", "the single pill is the phone's (`App::pilule`)"),
    ("ui:pfeuille", "the sheets of the single pill (`App::pfeuille_ouvrir`)"),
    ("ui:croix", "the finger's crosshair, laid by a long press (`App::croix_poser`)"),
    (
        "ui:funding_carte",
        "the card of a perpetual's Funding pill, on the phone only (`App::funding_cell`)",
    ),
    (
        "ui:roue",
        "the wheel of the pill, under a finger held on the symbol or the unit of time (`App::roue_poser`)",
    ),
    ("ui:plan", "the plan of the app opens from the phone's screen (`CellHit::Plan`)"),
];
/// Screens no form has, and messages no scene can deliver, and why.
const NOWHERE: &[(&str, &str)] = &[
    (
        "sheet:Quick",
        "nothing in the app opens this sheet (`argos-render` builds one in its own tests only)",
    ),
    (
        "answer:Emoji",
        "the page's: the computer reads the folder emoji/ at once (`App::charger_emojis`), and an image loaded this way \
         would stay for all the scenes of the test program (`argos_render::emoji::charger`)",
    ),
    (
        "answer:CacheLu",
        "the page's: `App::cache_lu` hands the download that follows to the event loop, which a test has not",
    ),
];

/// Is `id` this exception (an id, or a whole family)?
pub(super) fn is(exception: &str, id: &str) -> bool {
    if exception.ends_with(':') {
        id.starts_with(exception)
    } else {
        id == exception
    }
}

/// The screens a form does not have: (the form, the id or the family, why).
pub(super) fn exceptions() -> Vec<(Form, &'static str, &'static str)> {
    let of = |form: Form, list: &'static [(&'static str, &'static str)]| list.iter().map(move |(what, why)| (form, *what, *why));
    of(Form::Phone, PC_ONLY)
        .chain(of(Form::Pc, PHONE_ONLY))
        .chain(Form::ALL.into_iter().flat_map(|form| of(form, NOWHERE)))
        .collect()
}

/// Does this form have this screen?
pub(super) fn has(form: Form, id: &str) -> bool {
    !exceptions().iter().any(|(without, what, _)| *without == form && is(what, id))
}

/// Every screen, on each form that has it.
pub(super) fn all() -> Vec<(Form, String)> {
    use argos_render::dock::Module;
    use argos_render::indicators::Ind;
    let mut ids: Vec<String> = vec![];
    ids.extend(tools().iter().filter(|t| !t.action()).map(|t| id_of("panel", t)));
    ids.extend(Module::ALL.iter().map(|m| id_of("module", m)));
    ids.extend(Ind::ALL.iter().map(|i| id_of("ind", i)));
    ids.extend(CandleStyle::all().iter().map(|s| id("style", s.name())));
    ids.extend(argos_render::Theme::NAMES.iter().map(|t| id("theme", t)));
    ids.extend(ui_screens().iter().map(|f| id("ui", f)));
    ids.extend(app_screens().iter().map(|f| id("app", f)));
    ids.extend(session_screens().iter().map(|f| id("session", f)));
    ids.extend(LISTS.iter().map(|l| id("list", l)));
    ids.extend(POPS.iter().map(|p| id("pop", p)));
    ids.extend(PILLS.iter().map(|p| id("pill", p)));
    ids.extend(ANSWERS.iter().map(|k| id("answer", k)));
    // 🔔 (the alerts panel, and the feed of notifications behind its ⚙)
    ids.extend(NOTIFS.iter().map(|face| id("notifs", face)));
    ids.extend(every!(argos_render::sheet::SheetKind: Panels, Quick, Account).iter().map(|k| id_of("sheet", k)));
    ids.extend(
        every!(argos_render::context_menu::Source: PriceAxis, TimeAxis, Chart, Plus)
            .iter()
            .map(|k| id_of("menu", k)),
    );
    Form::ALL
        .into_iter()
        .flat_map(|form| ids.iter().filter(move |i| has(form, i)).map(move |i| (form, i.clone())))
        .collect()
}

/// The part of the window that shows the date and the hour it really is, if there is one. The app has a clock the
/// tests cannot freeze (`argos_render::journal::now_ms`), and the form of a new trade of the journal is filled with
/// it (`JClick::New`): a scene is compared without that part of its image.
pub(super) fn real_clock(a: &App) -> Option<Rect> {
    a.ui.j_form.as_ref().filter(|form| form.is_new).map(|form| form.rect)
}

/// What is on screen in `a`, read after it is drawn.
pub(super) fn shown(a: &App) -> BTreeSet<String> {
    let mut s = BTreeSet::new();
    let u = &a.ui;
    if a.watch.prefs.open && !u.zen {
        s.insert(id_of("panel", &a.watch.prefs.panel));
    }
    if a.dock.rect.is_some() {
        s.extend(a.watch.dock.shown().iter().map(|i| id_of("module", &i.module)));
    }
    s.extend(a.ind.on.iter().map(|i| id_of("ind", i)));
    s.insert(id("style", a.chart.styles[a.chart.style].name()));
    s.insert(id("theme", argos_render::Theme::NAMES[a.top.prefs.theme]));
    ui_shown(u, &mut s);
    app_shown(a, &mut s);
    session_shown(&a.session, &mut s);
    if let Some((kind, _)) = &u.list {
        s.insert(id("list", list_name(kind)));
    }
    if let Some(pop) = &u.pop {
        s.insert(id("pop", pop_name(&pop.kind)));
    }
    if let Some(sheet) = &u.pfeuille {
        s.insert(id("pill", pill_name(&sheet.kind)));
    }
    if let Some(panel) = &u.notifs {
        s.insert(id("notifs", NOTIFS[usize::from(panel.fil)]));
    }
    if let Some(sheet) = &u.sheet {
        s.insert(id_of("sheet", &sheet.kind));
    }
    if let Some(menu) = &u.context.open {
        s.insert(id_of("menu", &menu.source));
    }
    s
}
