//! 🖱️ La souris (ou le doigt) : sa position sur chaque zone de la fenêtre, ce qu'elle tient en glissant, les derniers
//! clics (pour le double-clic), le curseur affiché et les touches Alt / Ctrl / Maj enfoncées.

use web_time::Instant;
use winit::keyboard::ModifiersState;
use winit::window::CursorIcon;

use crate::{BarPress, Grab};

pub(crate) struct PointerState {
    /// Barre d'outils flottante attrapée : décalage souris / coin de la barre.
    pub(crate) tool_drag: Option<(f64, f64)>,
    /// Fenêtre de réglages (indicateur ou dessin) attrapée par son titre : écart souris → coin haut gauche.
    pub(crate) dlg_drag: Option<(f64, f64)>,
    /// La poignée a bougé depuis qu'on l'a attrapée (sinon, relâcher = clic : rétracter / déplier la barre).
    pub(crate) tool_moved: bool,
    /// Bouton de la barre flottante appuyé : clic au relâcher, clic maintenu = déplacer la barre, glisser un ⭐ = l'ordre.
    pub(crate) bar_press: Option<BarPress>,
    /// Dernière position de la souris dans la fenêtre (menus posés par-dessus la chart et la watchlist).
    pub(crate) win_mouse: (f64, f64),
    /// Souris sur la barre du haut (coordonnées de la fenêtre).
    pub(crate) top_mouse: Option<(f64, f64)>,
    /// 📱 Souris (doigt) sur la barre du bas du cell (coordonnées de cette barre).
    pub(crate) phone_mouse: Option<(f64, f64)>,
    /// Bord gauche de la watchlist attrapé : (x de la souris au départ, largeur au départ).
    pub(crate) wl_resize: Option<(f64, f64)>,
    /// Souris sur la barre de gauche.
    pub(crate) rail_mouse: Option<(f64, f64)>,
    /// Souris sur la chart.
    pub(crate) mouse: Option<(f64, f64)>,
    /// Glisser de la chart en cours : dernière position de la souris.
    pub(crate) drag: Option<(f64, f64)>,
    /// 🪟 Glisser dans une case du split : (case, dernier x) ; dernier clic dans une case (double-clic = vue auto).
    pub(crate) pane_drag: Option<(usize, f64)>,
    pub(crate) pane_click_at: Option<(usize, Instant)>,
    /// Glisser sur l'échelle de prix : dernière hauteur de la souris.
    pub(crate) axis_drag: Option<f64>,
    /// Dernier clic sur l'échelle de prix (pour le double-clic).
    pub(crate) last_axis_click: Option<Instant>,
    /// ≫ F1j: the last press on the time axis (a double tap / click there goes back to live).
    pub(crate) last_time_click: Option<Instant>,
    /// 👆 F1j: a finger put down on an Argos Murs pill or depth bar: (window x, y where it went down, chart x, y);
    /// its tooltip opens when the finger lifts there (a real tap), never after a drag.
    pub(crate) murs_appui: Option<(f64, f64, f64, f64)>,
    /// Dernier clic sur la chart (double-clic = mode normal ⇄ mode libre).
    pub(crate) last_chart_click: Option<Instant>,
    /// 🖥 ⛶ After a double-click on the chart: when, what a third click undoes, what it puts full screen.
    pub(crate) triple: Option<(Instant, crate::actions::plein_ecran::Defaire, crate::actions::plein_ecran::PleinEcran)>,
    /// 👆 Où le dernier appui (souris ou doigt) s'est posé, et celui d'avant : un double-clic / double-toucher ne compte
    /// qu'au même endroit (voir `App::est_double`).
    pub(crate) appui: Option<(f64, f64)>,
    pub(crate) appui_avant: Option<(f64, f64)>,
    /// 👆 This touch only took the finger's crosshair away (its lift does nothing either).
    pub(crate) appui_croix: bool,
    /// Touches Alt / Ctrl / Maj enfoncées (raccourcis).
    pub(crate) mods: ModifiersState,
    /// Objet attrapé à la souris (tout l'objet ou une poignée).
    pub(crate) grab: Option<Grab>,
    /// 🖍 The drawing bar dragged by its ⠿ handle: (mouse at the start, bar offset at the start), chart coordinates.
    pub(crate) drawbar_drag: Option<((f64, f64), (f64, f64))>,
    /// 🎨 The opacity slider of the palette held.
    pub(crate) cp_drag: bool,
    /// 🎚 The % slider of the Trading ticket held: the window x of the panel's left edge (the slider follows the
    /// mouse / the finger, even outside the panel, until let go).
    pub(crate) curseur_ticket: Option<f64>,
    /// Curseur affiché (flèche, main ouverte, main fermée).
    pub(crate) cursor: CursorIcon,
    /// 🖱 Le petit point (curseur dessiné) est posé à la place de `cursor`.
    pub(crate) point_pose: bool,
    /// Souris sur la watchlist et la barre d'icônes à droite (coordonnées de son image).
    pub(crate) side_mouse: Option<(f64, f64)>,
    /// 📍 Repère d'input (prix / heure) attrapé à la souris.
    pub(crate) mark_drag: Option<argos_render::inmark::Mark>,
    /// ↕️ Bord d'une bande d'oscillateur attrapé : (bande, y de la souris au départ, hauteur au départ).
    pub(crate) band_drag: Option<(argos_render::indicators::Ind, f64, f64)>,
    /// 📜 Top edge of a user script's pane grabbed: (script id, y at the start, height at the start).
    pub(crate) script_drag: Option<(String, f64, f64)>,
    /// ↕️ Bande glissée de l'intérieur (bande, x précédent, y au départ, décalage au départ) : elle monte / descend dans
    /// sa case, le temps suit ; ou sa colonne de valeurs glissée (bande, y au départ, zoom au départ).
    pub(crate) band_pan: Option<(argos_render::indicators::Ind, f64, f64, f64)>,
    pub(crate) band_zoom: Option<(argos_render::indicators::Ind, f64, f64)>,
    /// 📜 The same for a script's pane, by its view key (`ScriptView::view_key`): dragged inside (key, previous x, y at
    /// the start, offset at the start), its value column dragged (key, y at the start, zoom at the start).
    pub(crate) script_pan: Option<(String, f64, f64, f64)>,
    pub(crate) script_zoom: Option<(String, f64, f64)>,
    /// 🧩 Souris sur la colonne de modules.
    pub(crate) dock_mouse: Option<(f64, f64)>,
    /// 📱 Doigt posé sur le Marketplace : (y de départ, dernier y, a bougé) — glisser = la grille défile, taper = clic.
    pub(crate) mk_drag: Option<(f64, f64, bool)>,
    /// 👆 Doigts posés (gestes tactiles : voir `input::touch`).
    pub(crate) touch: crate::input::touch::Touches,
    /// 👆 Heure (ms) du dernier événement du doigt : juste après, les événements « souris » du navigateur (position
    /// d'avant) sont ignorés — ils faisaient croire à un glisser et un toucher sur deux était perdu.
    pub(crate) dernier_doigt: f64,
    /// 👆 (B-047) When the event loop's current round of work began (`touch::now_ms`, set at each `new_events`): a long
    /// press whose time came during that work (an image drawn, data read) waits one more look — the finger may have
    /// lifted meanwhile, its lift not read yet.
    pub(crate) tour_debut: Option<f64>,
    /// 👆 (B-048) When the finger did the gesture being played, read by the page with the touch (`timeStamp`): a gesture
    /// read late, after a slow image, keeps its real time. `None` outside a finger's gesture (the mouse, a long press):
    /// now. The clicks take it as it is (a double tap), the finger's gestures through `App::heure_geste_ms`.
    pub(crate) t_geste: Option<Instant>,
    /// 👆 Le doigt vient de se lever : plus de survol (sinon l'icône touchée restait surlignée) jusqu'à la vraie
    /// souris ou au prochain doigt.
    pub(crate) doigt_leve: bool,
    /// ✏️ (B-019) The press being played starts a finger's drag (a move follows it), not a tap: it never makes a
    /// double tap, and it grabs only the drawing already selected (the others: the chart moves under the finger).
    pub(crate) appui_glisse: bool,
    /// ✏️ (B-019) The finger's drag laid a drawing tool's point where it went down: where it lifts lays the next one
    /// (a two-point tool is finished by the drag alone, as it is drawn).
    pub(crate) trace_doigt: bool,
    /// 🅘 Le doigt s'est posé sur la page Indicateurs (le toucher qui l'a ouverte ne compte pas dedans).
    pub(crate) fx_appui: bool,
    /// 👆 Le doigt fait défiler une liste / un panneau / un menu : (x, y du départ, dernier y, heure du dernier y).
    pub(crate) finger_scroll: Option<(f64, f64, f64, f64)>,
    /// 👆 Barre des favoris qui défile sous le doigt : dernière position x (fenêtre).
    pub(crate) bar_swipe: Option<f64>,
    /// 📖 Finger down on a code block of the editor's Doc: last x (window); a sideways move slides the block.
    pub(crate) doc_pan: Option<f64>,
    /// ↕️ Bouton de la barre d'icônes attrapé : (rang actuel, y de départ, déplacé ?, panneau avant l'appui).
    pub(crate) icon_drag: Option<(usize, f64, bool, (bool, argos_render::watchlist::Tool))>,
    /// 👆 Élan après le défilement au doigt : (vitesse en points par ms, x, y où il s'applique).
    pub(crate) fling: (f64, f64, f64),
    /// 🔔 Ligne d'alerte attrapée (son identifiant) : elle suit la souris / le doigt.
    pub(crate) alert_drag: Option<u64>,
    /// 🔔 Where the alert line was pressed (window): released there without moving, its label shows.
    pub(crate) alert_appui: Option<(f64, f64)>,
    /// 💼 Ligne d'ordre attrapée sur la chart (entrée, SL, TP, brouillon) et son prix de départ (retour si refusé).
    pub(crate) order_drag: Option<(argos_render::ordres::Prise, f64)>,
    /// 💼 La ligne attrapée est du mauvais côté de l'entrée : le message (pastille rouge « ! »).
    pub(crate) order_err: Option<&'static str>,
    /// 📋 The tables: hovered, selected by the finger, dragged, their menu (tableaux-1).
    pub(crate) tab: crate::actions::tableaux::TabState,
}
