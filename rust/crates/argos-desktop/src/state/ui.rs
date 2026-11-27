//! 🪟 Ce qui est ouvert par-dessus l'app : menus, listes, fenêtres du bas, fenêtres de réglages, pages (Portfolio,
//! Marketplace), éditeur de scripts, Assistant, boîtes de saisie.

use argos_render::context_menu::ContextMenu;
use argos_render::goto::GoTo;
use argos_render::listmenu::ListMenu;
use argos_render::objdlg::ObjDialog;
use argos_render::sheet::{Sheet, SheetKind};
use argos_render::toolbar;
use argos_render::topmenu;
use argos_render::wlmenu::{AddPanel, NameBox};

use crate::ListKind;

pub(crate) struct UiState {
    /// 🧘 Mode Zen : seulement la chart (barres, panneaux et modules cachés).
    pub(crate) zen: bool,
    /// 🤖 Argos AI level 1: the explanation drawn on the chart (never saved, never selectable).
    pub(crate) explication: Option<crate::actions::argos_ai_ecran::Explication>,
    /// 🔔 The alert line whose label is shown (clicked or touched); ✅ the confirmation box open and what it does.
    pub(crate) alerte_sel: Option<u64>,
    /// 🔔 The alert line under the mouse (PC, like TradingView): its label shows while the mouse is on the line or
    /// on the label.
    pub(crate) alerte_survol: Option<u64>,
    pub(crate) confirmer: Option<(argos_render::confirmer::Confirmer, crate::actions::alerts::AConfirmer)>,
    /// ⛶ The chart or one panel full screen; what the menu line would put full screen (under the right-click or the
    /// finger's crosshair); 🖥 Zen before it; the bubble « Double tap pour revenir » and since when.
    pub(crate) plein_ecran: Option<crate::actions::plein_ecran::PleinEcran>,
    pub(crate) plein_cible: Option<crate::actions::plein_ecran::PleinEcran>,
    pub(crate) plein_zen_avant: bool,
    pub(crate) bulle_plein: Option<(&'static str, web_time::Instant)>,
    /// 🖥🧘 In Zen, the top bar shown over the chart while the mouse is at the top edge (or in the bar).
    pub(crate) zen_haut: bool,
    /// 🧠 Mémoire de l'assistant (notes, méthode, Sandbox) et « tout effacer » armé.
    pub(crate) ai_mem: crate::actions::AiMemory,
    pub(crate) ai_clear_armed: bool,
    /// 💡 Idées de la Communauté (gardées dans `ideas.json`), et le mode « effacer ».
    pub(crate) ideas: Vec<crate::actions::Idea>,
    pub(crate) ideas_delete: bool,
    /// 🔔 Alerte sur un indicateur : ligne choisie (son nom) et condition choisie (index de `LINE_CONDS`).
    pub(crate) alert_line: Option<String>,
    pub(crate) alert_op: usize,
    /// 🔎 Idée dont le détail est ouvert (identifiant), et « 📸 publier avec image » choisi.
    pub(crate) idea_open: Option<String>,
    pub(crate) idea_snap: bool,
    /// 🧪 Réglages et dernier résultat du backtest.
    pub(crate) bt_prefs: crate::actions::BtPrefs,
    pub(crate) bt: Option<argos_render::dock::BtView>,
    /// 🚀 Symboles du menu Top gagnants / perdants (l'id d'une ligne = son index ici).
    pub(crate) movers: Vec<(argos_data::Exchange, String)>,
    /// 📐 Mise en page choisie pour l'écran actuel (téléphone, tablette, desktop, double écran), revue à chaque image.
    pub(crate) form: argos_render::formfactor::Choice,
    /// Bandeau d'information affiché et depuis quand (il part tout seul après quelques secondes).
    pub(crate) banner_since: Option<(String, web_time::Instant)>,
    /// 📐 Le panneau était-il ouvert avant de passer en tablette portrait (rouvert en revenant en paysage) ?
    pub(crate) drawer_was_open: bool,
    /// 📐 Double écran l'un au-dessus de l'autre : (bas de la chart, haut du segment du bas), sous la barre du haut.
    pub(crate) rows: Option<(f64, f64)>,
    /// 📱 La feuille du bas est en verre par-dessus la chart entière (Trading, comme la maquette `#panneau`).
    pub(crate) feuille_verre: bool,
    /// 🖥 Souris sur la bande du rail replié : le rail s'ouvre par-dessus la chart (maquette, au survol).
    pub(crate) rail_survol: bool,
    /// 🖥 Rail de gauche gardé ouvert (bouton ‹ › à côté du rail ouvert ; interface-2.json).
    pub(crate) rail_garde: bool,
    /// 📱 Couleurs du haut et du bas de l'image, données à la page (zones sûres de l'iPhone de la même couleur).
    pub(crate) chrome: (u32, u32),
    /// 🫧 Temps d'une image (ms, moyenne glissante) et nombre d'images mesurées ; style verre coupé (appareil trop lent).
    pub(crate) frame_ms: f64,
    /// 🫧 Temps du verre seul dans l'image (ms, moyenne glissante) et celui de l'image en cours.
    pub(crate) glass_ms: f64,
    pub(crate) glass_now: f64,
    /// 📏 Temps des phases d'une image (moyennes glissantes, ms) : avant la chart, chart, pilules et barres, copie
    /// vers l'écran, envoi (mesure de fluidité ; la page les reçoit par `argosPhases` si elle l'écoute).
    pub(crate) phases: [f64; 5],
    /// 📤 L'image envoyée la fois d'avant (web) : seule la bande de rangées qui a changé est renvoyée au canvas.
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub(crate) image_avant: Vec<u32>,
    /// Sa taille (largeur, hauteur) : une autre taille = tout renvoyer (le canvas redimensionné est effacé).
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub(crate) image_taille: (usize, usize),
    /// 🗂 Menus, feuilles et volet ouverts à l'image d'avant (voir `App::un_seul_ouvert`).
    pub(crate) ouverts: [bool; OUVERTS],
    /// Octets RGBA de la bande envoyée (gardés : pas de nouvelle allocation à chaque image).
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub(crate) octets: Vec<u8>,
    pub(crate) frames: u32,
    /// 🐢 Dernier geste (souris, clavier, molette, doigt, taille de la fenêtre) : au repos depuis 2 s, les données en
    /// direct ne redessinent plus qu'une fois par seconde au plus (avec la dernière valeur).
    pub(crate) derniere_entree: web_time::Instant,
    /// 🐢 Dernier redessin demandé par des données (prix, trades, cotations…).
    pub(crate) dernier_redessin_donnees: web_time::Instant,
    /// 🐢 Données arrivées au repos sans redessin : il sera fait à la prochaine seconde.
    pub(crate) redessin_en_attente: bool,
    /// 🖥 Barre de favoris jamais déplacée (interface-2.json n'a pas sa place) : sa place de la maquette, remontée au-dessus
    /// des panneaux d'indicateurs quand la chart est basse (elle ne cache pas leur nom).
    pub(crate) barre_par_defaut: std::cell::Cell<bool>,
    /// 🖥 Barre de favoris repliée d'elle-même (plus large que la chart) : elle se rouvre quand la place revient.
    pub(crate) barre_auto_repliee: bool,
    /// 🖥 Le clic a fermé le volet API (clic à côté) : son relâcher ne fait rien non plus.
    pub(crate) relache_avale: bool,
    /// 🖥 ⤢ Volet large (min(920, 62 % de la fenêtre)) ; gardé dans interface-2.json.
    pub(crate) volet_large: bool,
    /// 🖥 La page Indicateurs (ƒx) est ouverte dans le volet flottant (PC), pas en plein écran.
    pub(crate) fx_dans_volet: bool,
    /// 🖥 🔎 Le volet « Symboles » de la maquette (PC), dans le volet flottant (avec le panneau Watchlist).
    pub(crate) symboles: Option<argos_render::symboles::Symboles>,
    /// 🔎 The market panel of the symbol pill (hover 0.3 s on PC, touch on the phone and the tablet), its hover timer
    /// and the clock that timer reads.
    pub(crate) marches: Option<argos_render::marches::Panneau>,
    /// ⚡ The card (PC) or sheet (phone) of the chart's news, opened by a ⚡.
    pub(crate) eclair: Option<argos_render::eclair::Eclair>,
    /// 🧭 « Voir sur la chart » of the Explorer Hyperliquid: an address's entry and liquidation lines.
    pub(crate) hl_lignes: Option<argos_render::hlexplorer::LignesAdresse>,
    pub(crate) marches_survol: argos_render::marches::Survol,
    pub(crate) marches_horloge: web_time::Instant,
    /// ₿ The catalogues with pairs against BTC were asked once by the market panel (not again at each letter).
    pub(crate) catalogues_btc: bool,
    /// 🖥 🔎 Symboles ouverts récemment (le plus récent d'abord, 8 au plus).
    pub(crate) sym_recents: Vec<String>,
    /// 🖥 ⚙ La fenêtre « Réglages » de la maquette (PC).
    pub(crate) reglages: Option<argos_render::reglages::Reglages>,
    /// 🛟 Support panel (🖥, in the floating panel).
    pub(crate) support: Option<argos_render::support::Support>,
    /// 🛟 Argos replies not read yet (red dot on the Support button), reloaded at startup and on opening.
    pub(crate) support_non_lues: usize,
    /// 🛟 📎 A file pick is in progress in the page (response expected from `argosPick`).
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub(crate) support_choix: bool,
    /// 🛟 The tickets have been reloaded once since startup (button badge).
    pub(crate) support_lance: bool,
    /// 🖱 Curseur choisi au 1er bouton de la barre de gauche (gardé dans interface-2.json).
    pub(crate) curseur: crate::types::CurseurChoisi,
    /// 🖱 Le petit point, préparé à l'ouverture de la fenêtre (`None` : pas encore, ou pas possible : la croix).
    pub(crate) curseur_point: Option<winit::window::CustomCursor>,
    /// 🖍 Traits du curseur Démonstration (le dernier suit la souris tant que le bouton est tenu).
    pub(crate) demo_traits: Vec<crate::types::TraitDemo>,
    /// 🧪 Tests : taille de la fenêtre quand il n'y en a pas (le dessin va alors dans `image_test`, 0xRRGGBB).
    pub(crate) taille_test: Option<winit::dpi::PhysicalSize<u32>>,
    /// 📱 The single pill folded away (kept in interface-2.json), and its sheet open (« Argos », « Dessins »…).
    pub(crate) pilule_repliee: bool,
    pub(crate) pfeuille: Option<crate::actions::pilule::PFeuille>,
    /// 👆 The finger's crosshair (phone, tablet): where it is (chart) and its price; it stays until the next touch.
    pub(crate) croix: Option<(f64, f64, f64)>,
    /// 📱 ⚡ The card of a perpetual's Funding pill is open (phone; `App::funding_press`).
    pub(crate) funding_carte: bool,
    /// 🎯 The chart image kept while only the mouse moves (`fond.rs`).
    pub(crate) fond: Option<crate::fond::Fond>,
    pub(crate) image_test: Vec<u32>,
    /// 🧪 Tests: the order the Argos AI card and the menus were laid in the last image (« menu », « argos_ai »,
    /// « menu_ai »…).
    pub(crate) couches_test: Vec<&'static str>,
    pub(crate) glass_off: bool,
    /// 🫧 Images depuis le dernier changement de qualité du verre (on mesure à nouveau avant de juger).
    pub(crate) glass_since: u32,
    /// 🫧 Taille d'écran au dernier réglage de la qualité du verre.
    pub(crate) glass_screen: (f64, f64),
    /// Fenêtre des outils d'une catégorie / des unités de temps, ouverte depuis la barre flottante.
    pub(crate) pop: Option<toolbar::Pop>,
    /// Menu en liste ouvert (intervalles ou types de chart).
    pub(crate) list: Option<(ListKind, ListMenu)>,
    /// 🪟 Case du split dont on choisit le symbole (watchlist).
    pub(crate) pane_symbole: Option<usize>,
    /// ⏱ Intervalle personnalisé en cours de saisie : pour cette case du split (sinon la chart principale).
    pub(crate) ut_case: Option<usize>,
    /// ✨ Assistant ouvert.
    pub(crate) ai_open: bool,
    /// Fenêtre ouverte au-dessus de la barre du bas (Panneaux, Hub d'analyse, Dessins).
    pub(crate) sheet: Option<Sheet>,
    /// Menu Argos ▾ ouvert (carte comme v1).
    pub(crate) argos: Option<topmenu::ArgosMenu>,
    /// 🌳 Arborescence des objets ouverte.
    pub(crate) obj_tree: Option<argos_render::objtree::ObjTree>,
    /// 🪟 Menu du split ouvert.
    pub(crate) split_menu: Option<argos_render::splits::Menu>,
    /// Fenêtre fermée par ce clic (le même bouton ne la rouvre pas).
    pub(crate) sheet_closed: Option<SheetKind>,
    /// Menu ouvert (clic droit, ⚙, ➕).
    pub(crate) context: ContextMenu,
    /// Fenêtre « Aller à » ouverte (calendrier).
    pub(crate) goto: Option<GoTo>,
    /// Fenêtre de réglages d'un dessin (double-clic sur le dessin).
    pub(crate) obj_dlg: Option<ObjDialog>,
    /// 🖍 The drawing bar moved by its handle (from its place above the drawing), chart coordinates.
    pub(crate) drawbar_off: (f64, f64),
    /// 🎨 The palette open from the drawing bar: (drawing, text colour?, the palette).
    /// 🔔 « Créer une alerte sur » a drawing: (drawing, the window).
    pub(crate) draw_alert: Option<(usize, argos_render::drawalert::DrawAlert)>,
    pub(crate) colorpick: Option<(usize, argos_render::objdlg::Which, argos_render::colorpick::ColorPick)>,
    /// ⚙️ Fenêtre de réglages d'un indicateur.
    pub(crate) ind_dlg: Option<argos_render::inddlg::IndDialog>,
    /// 📓 Fiche d'un trade du journal (par-dessus l'app).
    pub(crate) j_form: Option<argos_render::journal::JForm>,
    /// 💼 Page Portfolio (comme v1).
    pub(crate) pf_page: Option<argos_render::portfolio::PfPage>,
    /// 📊 Full report page of the last backtest (lot 6b).
    pub(crate) bt_page: Option<argos_render::btpage::BtPage>,
    /// ⚖️ TradingView's file is being chosen in the page (response expected from `argosPick`).
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub(crate) bt_tv_choix: bool,
    /// 🎯 The Stratégie volet: the scripts on the chart when it was first looked at (a strategy added after opens the
    /// volet, once), what its last result was made from, and the volet to open at the next frame (🖥).
    pub(crate) strat_vus: Option<std::collections::HashSet<String>>,
    pub(crate) strat_cle: Option<crate::actions::StratCle>,
    pub(crate) strat_ouvrir: bool,
    /// 🛒 Marketplace (comme v1) et ses aperçus, calculés à l'ouverture sur les bougies affichées.
    pub(crate) mk_page: Option<argos_render::marketplace::MkPage>,
    /// 🎓 Page de l'Académie (plein écran).
    pub(crate) ac_page: Option<argos_render::academy::AcademyPage>,
    pub(crate) mk_spark: Vec<(argos_render::indicators::Ind, (Vec<f64>, argos_render::text::Rgb))>,
    /// 📍 Fenêtre de réglages cachée le temps de choisir un point sur la chart (input à remplir).
    pub(crate) ind_here: Option<(argos_render::inddlg::IndDialog, usize)>,
    /// 📜 Éditeur ouvert (fenêtre au-dessus de la chart, comme v1 : il reçoit les clics et le clavier).
    pub(crate) editor: Option<argos_render::editor::Editor>,
    /// Panneau ＋ (ajouter un symbole) et boîte de nom (nouvelle liste, section, renommer).
    pub(crate) add: Option<AddPanel>,
    pub(crate) name_box: Option<NameBox>,
    /// ✏️ Mise en page renommée depuis sa tuile de l'Accueil (sinon : celle ouverte).
    pub(crate) layout_cible: Option<String>,
    /// 📱 La première image est passée (le départ sur cell est réglé une seule fois).
    pub(crate) depart_vu: bool,
    /// « Supprimer la liste » cliqué une fois : le 2e clic confirme.
    pub(crate) delete_armed: bool,
    /// ⌂ Accueil (page plein écran) et sa carte Académie (lue à l'ouverture : terminés, total, prochain guide).
    pub(crate) accueil: Option<argos_render::accueil::AccueilPage>,
    /// ⌂ What this tab last noted about the Accueil (open or not), in its own storage (`platform::onglet`).
    pub(crate) accueil_note: bool,
    pub(crate) accueil_ac: (usize, usize, Option<String>),
    /// 🔎 Recherche universelle Ctrl + K.
    pub(crate) palette: Option<argos_render::recherche::Palette>,
    /// 🅘 Page Indicateurs du cell et de la tablette (ƒx, maquette).
    pub(crate) fx_page: Option<argos_render::fxpage::FxPage>,
    /// 🔔 Feuille des notifications du cell et de la tablette (maquette).
    pub(crate) notifs: Option<argos_render::notifs::Notifs>,
    /// 🗺 Plan de l'app (cell : page pleine).
    pub(crate) plan: Option<argos_render::plan::Plan>,
    /// 🎡 Le doigt sur la pilule « symbole · unité » du cell (la roue).
    pub(crate) roue: Option<crate::actions::roue::Roue>,
    /// 📱 La section de la watchlist dont le menu (ou la boîte de nom) est ouvert.
    pub(crate) section: Option<String>,
    /// ⌨️ Volet des raccourcis du clavier (Ctrl + /).
    pub(crate) raccourcis: Option<argos_render::raccourcis::Raccourcis>,
    /// 🧪 Volet du Simulateur, et le replay en cours est-il le sien ?
    pub(crate) sim: Option<argos_render::simulateur::Simulateur>,
    pub(crate) sim_replay: bool,
    /// 💼 Ordre préparé sur la chart (bouton Buy / Sell), pas encore envoyé.
    pub(crate) brouillon: Option<argos_render::ordres::Brouillon>,
    /// ⏯ Heure de la dernière bougie jouée par le replay du Simulateur.
    pub(crate) sim_last_t: i64,
    /// 📅 Le prochain choix de « Aller à » est la date de départ du replay du Simulateur.
    pub(crate) goto_replay: bool,
    /// ⏯ Replay du Simulateur demandé à une date : le passé se charge jusque-là, puis le replay part.
    pub(crate) replay_vise: Option<crate::actions::simulateur::Visee>,
    /// ⏪ The chart changed its unit during a replay: the past loads back to this bar of the replay (seconds), then
    /// the message of the change is said again.
    pub(crate) replay_ut: Option<(i64, String)>,
    /// 💾 Séance de replay gardée (relue à l'ouverture du Simulateur).
    pub(crate) seance: Option<argos_core::simu::Seance>,
    /// 🤖 Le trade dont la carte « Argos AI · ton trade » est ouverte (lot 7).
    pub(crate) avis: Option<crate::actions::avis::AvisDe>,
    /// 🏦 Le ⚙ du Footprint (bourses fusionnées) est ouvert.
    pub(crate) fp_reglages: bool,
    /// 🎫 📏 Zoom auto of the Trading ticket (mock-up of 22 Nov), while the ticket shows; `None`: no ticket.
    pub(crate) ticket_zoom: Option<TicketZoom>,
    /// 🛡 What covered the chart at the last image, as told to the page (`argosEcran`, [`crate::App::par_dessus`]):
    /// told again only when it changes. `None`: not told yet.
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub(crate) par_dessus_note: Option<String>,
}

/// 🗂 Nombre de sortes de menus flottants suivies par `App::un_seul_ouvert` (liste, fenêtre de cases, feuille, menu
/// ARGOS, ▦ Splits, 🔔, 🗺, 🔎, clic droit, raccourcis, ⚙ Réglages, volet de droite).
pub(crate) const OUVERTS: usize = 13;

/// 🎫 📏 Zoom auto of the Trading ticket (mock-up of 22 Nov 2026): the price scale widened just enough to show its
/// Limite, TP and SL; left alone once moved by hand; the scale from before given back when the ticket closes.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct TicketZoom {
    /// The chart it belongs to (symbol, unit in seconds): another chart starts over, nothing given back.
    pub(crate) cle: (String, i64),
    /// The price scale when the ticket opened (bounds by hand, ratio lock), given back when it closes.
    pub(crate) avant: (Option<(f64, f64)>, Option<f64>),
    /// The bounds the zoom auto set last; `None`: none yet.
    pub(crate) pose: Option<(f64, f64)>,
    /// The scale moved by hand during the ticket: the zoom auto leaves it alone until the next ticket.
    pub(crate) main: bool,
}
