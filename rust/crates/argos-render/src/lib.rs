//! # Argos Render — les bougies maison d'Argos (aucune dépendance à TradingView)
//! Dessine la chart dans une image en pixels : grille, bougies classiques ou bougies de volume, échelle de prix et de temps,
//! ligne du dernier prix. Le même rendu sert partout : fenêtre desktop, canvas web (WASM), cell, et images PNG pour les tests.
//!
//! Le cœur de la chart : `lib.rs` (`Chart`, `render`), `theme` (couleurs), `style` (types de bougies), `series` (lignes
//! et aires), `timefmt` (textes du temps), `scale` (échelle de prix), `indicators/` (indicateurs) ; le reste des
//! fichiers dessine l'interface autour (barres, menus, panneaux, fenêtres).

#![forbid(unsafe_code)]

pub mod academy;
pub mod accueil;
pub mod actions;
pub mod actualites;
pub mod aibtn;
pub mod alerte_etiquette;
pub mod alertes;
pub mod assistant;
pub mod avis;
pub mod bookheat;
pub mod bottombar;
pub mod btpage;
pub mod calendrier;
pub mod carnet;
pub mod catalogue;
pub mod cmc;
pub mod colorpick;
pub mod confirmer;
pub mod context_menu;
pub mod doc_editeur;
pub mod dock;
pub mod drawalert;
pub mod drawbar;
pub mod drawings;
pub mod eclair;
pub mod eco;
pub mod econ;
pub mod ecrans;
pub mod editor;
pub mod emoji;
mod emoji_tout;
pub mod emojipick;
pub mod entete;
pub mod exchanges;
pub mod explication;
pub mod formfactor;
pub mod fpmulti;
pub mod fxpage;
pub mod glass;
pub mod goto;
pub mod hilo;
pub mod hlexplorer;
pub mod hlstats;
pub mod i18n;
pub mod icons;
pub mod inddlg;
pub mod indicators;
pub mod indleg;
pub mod inmark;
pub mod jetons;
pub mod journal;
pub mod liqview;
pub mod listmenu;
pub mod marches;
pub mod marketplace;
#[cfg(test)]
mod menus_ecran;
pub mod month;
pub mod murs;
pub mod navbar;
pub mod notifs;
pub mod objdlg;
pub mod objmenu;
pub mod objtree;
pub mod ordres;
pub mod palette;
pub mod panels;
pub mod phonebar;
pub mod phosphor;
pub mod plan;
pub mod portfolio;
pub mod position;
pub mod px;
pub mod raccourcis;
pub mod rail;
pub mod recherche;
pub mod reglages;
pub mod replay;
pub mod scriptview;
pub mod sessions;
pub mod settings;
pub mod shapes;
pub mod sheet;
pub mod simulateur;
pub mod snapshot;
pub mod sources;
pub mod splits;
pub mod strategie;
pub mod support;
pub mod symboles;
pub mod tableaux;
pub mod text;
pub mod toolbar;
pub mod toolgeom;
pub mod tools;
pub mod topbar;
pub mod topmenu;
pub mod tpo;
mod twemoji;
pub mod viewport;
pub mod watchlist;
pub mod whales;
pub mod wlmenu;
pub mod wordsbox;
pub mod zoompad;

use argos_core::Bar;
use argos_script::indicators::volume_candles;
use settings::{ChartSettings, ScaleMode};
use text::{Align, Rgb, Text};
use tiny_skia::{Color, GradientStop, LinearGradient, Paint, PathBuilder, Pixmap, Point, Rect, SpreadMode, Stroke};
use viewport::{data_decimals, decimals_for, format_price, format_volume, nice_step, time_interval, PriceScale, Slots, Viewport};

// la chart découpée par rôle (même API : tout reste accessible depuis argos_render)
mod scale;
mod series;
mod style;
mod theme;
mod timefmt;
use scale::build_scale;
pub use scale::{current_scale, frame};
use series::{bar_tick, draw_hlc_area, draw_line_series};
use style::{candle_width, close_scaled};
pub use style::{renko_size, series, slots, CandleStyle, RenkoBox};
pub use theme::{set_theme, theme_index, Theme};
pub use timefmt::{civil_day, crosshair_label};
use timefmt::{clock_text, close_of, countdown_text, time_bucket};

/// Ce qu'on demande de dessiner.
#[derive(Debug, Clone, PartialEq)]
pub struct Chart<'a> {
    pub bars: &'a [Bar],
    /// Unité de temps en secondes (pour les étiquettes de temps).
    pub tf_seconds: i64,
    pub viewport: Viewport,
    pub theme: Theme,
    pub style: CandleStyle,
    /// Décalage de l'heure affichée (ex. −4 h pour Toronto en heure d'été).
    pub tz_offset_s: i64,
    /// Titre en haut à gauche (« BTC · 30m · Hyperliquid »).
    pub title: Option<String>,
    /// Position de la souris (réticule), `None` si elle est hors de la chart.
    pub crosshair: Option<(f64, f64)>,
    /// Message d'avertissement (ex. source de secours du superviseur), affiché sous le titre.
    pub banner: Option<String>,
    /// Réglages de la chart (source unique : rendu, menus, fenêtre Réglages, raccourcis).
    pub settings: ChartSettings,
    /// Menu ouvert (clic droit, ⚙, ➕).
    pub context: context_menu::ContextMenu,
    /// Objets dessinés (moteur d'objets : un outil, ses points, son style).
    pub drawings: Vec<drawings::Drawing>,
    /// Objet sélectionné (dessiné avec ses poignées).
    pub selected: Option<usize>,
    /// Moment présent (secondes UTC) pour le compte à rebours et l'horloge ; 0 = inconnu (rien d'affiché).
    pub now_s: i64,
    /// Direct (WebSocket) : `None` = pas de direct pour cette source, `Some(true)` = branché, `Some(false)` = reconnexion.
    pub live: Option<bool>,
    /// 📴 Plus de réseau : étiquette « Hors ligne » sous le titre (la chart reste utilisable avec le cache).
    pub hors_ligne: bool,
    /// 📐 Légende O H L C · Vol toujours montrée (la dernière bougie sans réticule), comme la maquette du cell.
    pub legende: bool,
    /// 🔍 Pilule de navigation (− + ‹ › ↺ ↶ ↷) en bas de la chart ; 📱 pas sur cell ni tablette (pincer / glisser,
    /// Replay dans ⚙).
    pub navigation: bool,
    /// Fenêtre « Aller à » ouverte (calendrier).
    pub goto: Option<goto::GoTo>,
    /// Indicateurs natifs déjà calculés (sous les dessins).
    pub indicators: Option<&'a indicators::IndOut>,
    /// 📍 Repère d'input en train d'être glissé (surligné).
    pub mark: Option<inmark::Mark>,
    /// 🐋 Bulles des baleines (grosses transactions de la pièce du graphique, déjà filtrées par le seuil).
    pub whales: Vec<argos_data::whales::Whale>,
    /// 📅 Annonces économiques à poser au bas de la chart (vide : rien).
    pub events: Vec<econ::Mark>,
    /// 🔔 Prix des alertes de prix allumées du symbole affiché (lignes pointillées avec une cloche).
    pub alerts: Vec<f64>,
    /// 🔔 Paused alerts, the lines' colour and their short form (« Lignes d'alerte » unchecked).
    pub alertes_vue: alerte_etiquette::Vue,
    /// 💼 Positions, ordres en attente et brouillon démo du symbole affiché (lignes à la TradingView).
    pub ordres: ordres::VueOrdres,
    /// 🟠 Pastille DÉMO dans la barre du bas (tablette) : « DÉMO −580,22 $ » et la couleur du montant.
    pub demo: Option<(String, Rgb)>,
    /// 📖 Meilleur achat et meilleure vente du carnet en direct (lignes / étiquettes Bid & Ask).
    pub bid_ask: Option<(f64, f64)>,
    /// 🔥 Heatmap du carnet et ses réglages (indicateur allumé) ; `note` : message discret (carnet indisponible…).
    pub book_heat: Option<&'a bookheat::BookHeat>,
    pub book_note: Option<&'static str>,
    /// 🧱 Argos Murs (indicator on, crypto chart): walls, grey words, book bars, summary.
    pub murs: Option<murs::Ecran>,
    /// 💥 Carte des liquidations estimée (indicateur allumé).
    pub liq_map: Option<&'a liqview::LiqView>,
    /// 🔍 Boutons − + RAZ Zen en bas à gauche (chart principale seulement).
    pub pad: Option<zoompad::Pad>,
    /// 🧪 Trades du dernier backtest (flèches L / S à l'entrée, rond gagnant / perdant à la sortie).
    pub trades: Vec<TradeMark>,
    /// 🤖 Argos AI level 1: the explanation's drawings (already checked by `argos_lens::explain::check`), owned by the
    /// app (never saved, never selectable); empty: none.
    pub explication: &'a [argos_lens::explain::Command],
    /// 📍 Places over the plot (chart coordinates) the app draws on after the chart (the floating toolbar, the
    /// legend's unfolded indicator list): the high / low pills keep clear of them. Empty: none.
    pub garder: Vec<shapes::Rect>,
    /// 🟠 Places over the plot (chart coordinates) that only the pills keep clear of (Argos Murs' walls, the high / low
    /// pills, the Explorer's lines), not the tables: the 📟 DÉMO band, which moves out of a table's way itself (B-018).
    pub garder_pilules: Vec<shapes::Rect>,
    /// 📟 Top (chart y) of the app's buttons floating at the bottom of the chart (tablet by the finger: the symbol
    /// pill and ƒx): Argos Murs' bottom pills sit just above them. `None`: nothing floats there (cell, PC).
    pub boutons_bas: Option<f64>,
}

/// 🧪 Un trade de backtest dessiné sur la chart.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TradeMark {
    pub entry_t: i64,
    pub exit_t: i64,
    pub long: bool,
    pub entry: f64,
    pub exit: f64,
    /// Résultat en % du prix (signe du sens compris).
    pub pct: f64,
}

pub(crate) fn paint(c: Rgb, alpha: f32) -> Paint<'static> {
    let mut p = Paint::default();
    p.set_color(Color::from_rgba8(c.0, c.1, c.2, (alpha.clamp(0.0, 1.0) * 255.0).round() as u8));
    p.anti_alias = false;
    p
}

/// Ligne dense (beaucoup de points par pixel, chart très dézoomée) : un passage par colonne de pixel (premier, plus
/// haut, plus bas, dernier point de la colonne), comme TradingView. Même tracé à l'œil, beaucoup moins de segments, et
/// aucun pic perdu. Peu de points : inchangée.
pub(crate) fn dense_line(pts: impl Iterator<Item = (f64, f64)>) -> Vec<(f32, f32)> {
    let mut out: Vec<(f32, f32)> = vec![];
    // colonne en cours : (x de la colonne, premier y, plus petit, plus grand, dernier, nombre de points)
    let mut col: Option<(i64, f64, f64, f64, f64, usize)> = None;
    let flush = |out: &mut Vec<(f32, f32)>, (cx, first, lo, hi, last, n): (i64, f64, f64, f64, f64, usize)| {
        let x = cx as f32 + 0.5;
        if n == 1 {
            out.push((x, first as f32));
            return;
        }
        // le pic du milieu dans l'ordre où la ligne le traverse
        let (a, b) = if first <= last { (lo, hi) } else { (hi, lo) };
        out.extend([(x, first as f32), (x, a as f32), (x, b as f32), (x, last as f32)]);
    };
    let mut raw: Vec<(f64, f64)> = vec![];
    for (x, y) in pts {
        raw.push((x, y));
        let cx = x.floor() as i64;
        col = match col {
            Some(k) if k.0 == cx => Some((cx, k.1, k.2.min(y), k.3.max(y), y, k.5 + 1)),
            Some(k) => {
                flush(&mut out, k);
                Some((cx, y, y, y, y, 1))
            }
            None => Some((cx, y, y, y, y, 1)),
        };
    }
    if let Some(k) = col {
        flush(&mut out, k);
    }
    // pas dense (au plus un point par colonne) : le tracé exact
    if out.len() >= raw.len() {
        return raw.into_iter().map(|(x, y)| (x as f32, y as f32)).collect();
    }
    out
}

pub(crate) fn fill(pm: &mut Pixmap, x: f64, y: f64, w: f64, h: f64, c: Rgb, alpha: f32) {
    // 🔍 on whole screen pixels, the size from `w` × `h` alone: candles, wicks, grid and bars stay regular
    let (x, y, w, h) = crate::px::snap_rect(x, y, w.max(1.0), h.max(1.0));
    if let Some(r) = Rect::from_xywh(x as f32, y as f32, w as f32, h as f32) {
        pm.fill_rect(r, &paint(c, alpha), crate::px::t(), None);
    }
}

/// Couleurs TradingView des types en ligne.
pub(crate) const LINE_BLUE: Rgb = Rgb(0x29, 0x62, 0xff);

/// Peinture en dégradé vertical : `stops` = (position 0 → 1 entre `y0` et `y1`, couleur, opacité).
pub(crate) fn gradient(y0: f32, y1: f32, stops: &[(f32, Rgb, f32)]) -> Paint<'static> {
    let stops = stops
        .iter()
        .map(|&(pos, c, a)| GradientStop::new(pos, Color::from_rgba8(c.0, c.1, c.2, (a * 255.0).round() as u8)))
        .collect();
    let shader = LinearGradient::new(
        Point::from_xy(0.0, y0),
        Point::from_xy(0.0, y1),
        stops,
        SpreadMode::Pad,
        tiny_skia::Transform::identity(),
    )
    .unwrap_or(tiny_skia::Shader::SolidColor(Color::TRANSPARENT));
    Paint {
        shader,
        anti_alias: true,
        ..Paint::default()
    }
}

thread_local! {
    /// 🖥 Décalage de la légende (titre, O H L C, Vol, ⌄ indicateurs) vers le bas : la bande des rangées du haut repliées
    /// passe au-dessus d'elle, jamais dessus.
    static HAUT_LEGENDE: std::cell::Cell<f64> = const { std::cell::Cell::new(0.0) };
    /// 🖥🧘 Décalage du bandeau d'information vers le bas (Zen : sous la légende, qui est alors tout en haut).
    static BAS_BANDEAU: std::cell::Cell<f64> = const { std::cell::Cell::new(0.0) };
    /// The legend's lines (title, O H L C, Vol, « ⌄ N ») and its unfolded list at the last drawing (chart
    /// coordinates): the information banner goes under them, never on them.
    static LEGENDE_BANDEAU: std::cell::RefCell<Vec<shapes::Rect>> = const { std::cell::RefCell::new(Vec::new()) };
    /// ≫ F1j: the back-to-live button of the last drawing (chart coordinates); `None`: live, not shown.
    static DIRECT: std::cell::Cell<Option<shapes::Rect>> = const { std::cell::Cell::new(None) };
    /// 📋 (tableaux-1) Places the app keeps free of tables only (the cell's unfolded legend list): see
    /// [`set_reserve_tableaux`].
    static RESERVE_TABLEAUX: std::cell::RefCell<Vec<shapes::Rect>> = const { std::cell::RefCell::new(Vec::new()) };
    /// 📍 The high / low pills of the last drawing (tableaux-1 tour 2, F10: checked against the legend's open list).
    static HILO_DESSINES: std::cell::RefCell<Vec<shapes::Rect>> = const { std::cell::RefCell::new(Vec::new()) };
    /// ≫ Where it was last drawn (kept while live): it stays there while that place is free.
    static DIRECT_AVANT: std::cell::Cell<Option<shapes::Rect>> = const { std::cell::Cell::new(None) };
    /// 🧱 F1k: the price scale's labels drawn and Argos Murs' price tags of the last drawing (chart coordinates).
    static ECHELLE: std::cell::RefCell<(Vec<shapes::Rect>, Vec<shapes::Rect>)> = const { std::cell::RefCell::new((Vec::new(), Vec::new())) };
}

/// 🧱 F1k: the price scale's labels drawn (their rects) and Argos Murs' price tags on the scale, at the last drawing.
pub fn echelle_dessinee() -> (Vec<shapes::Rect>, Vec<shapes::Rect>) {
    ECHELLE.with(|e| e.borrow().clone())
}

/// ≫ F1j: the back-to-live button of the last drawing (chart coordinates), shown only off live.
pub fn bouton_direct() -> Option<shapes::Rect> {
    DIRECT.with(|d| d.get())
}

/// 📋 (tableaux-1, T6) Places no table goes under, besides `Chart::garder` (chart coordinates): the legend's list
/// unfolded on the cell, which the walls' and high / low pills may sit beside (so not in `garder`).
pub fn set_reserve_tableaux(v: Vec<shapes::Rect>) {
    RESERVE_TABLEAUX.with(|r| {
        if *r.borrow() != v {
            *r.borrow_mut() = v;
            // (the kept chart image is drawn again: the tables move)
            tableaux::changer();
        }
    });
}

/// 📍 The high / low price pills drawn in the last drawing (chart coordinates).
pub fn hilo_dessines() -> Vec<shapes::Rect> {
    HILO_DESSINES.with(|h| h.borrow().clone())
}

/// ≫ Where a press goes back to live: the drawn button, and 👆 by the finger at least 44 × 44 points around its
/// centre (drawn at 36 on the phone: a touch a little beside it still counts; Jee's iPhone, 24 Nov).
pub fn bouton_direct_zone() -> Option<shapes::Rect> {
    let r = bouton_direct()?;
    if !actions::tactile() {
        return Some(r);
    }
    let (w, h) = (r.w.max(44.0), r.h.max(44.0));
    Some(shapes::Rect::new(r.cx() - w / 2.0, r.cy() - h / 2.0, w, h))
}

/// 🖥🧘 Décale le bandeau d'information de la chart vers le bas (`d` points ; 0 : à sa place).
pub fn set_bas_bandeau(d: f64) {
    BAS_BANDEAU.with(|c| c.set(d.max(0.0)));
}

/// 🖥 Décale la légende de la chart vers le bas (`d` points ; 0 : à sa place).
pub fn set_haut_legende(d: f64) {
    HAUT_LEGENDE.with(|c| c.set(d.max(0.0)));
}

/// 📱 Under this plot height, the finger's chart shows no O H L C · Vol legend (it would cover the candles).
const LEGENDE_MIN_H: f64 = 360.0;

/// 🖥 Décalage de la légende vers le bas.
pub fn haut_legende() -> f64 {
    HAUT_LEGENDE.with(|c| c.get())
}

/// The legend's two lines for the bar `b` (`real`: the real bar, not Heikin Ashi): « O … H … L … C … ±x % », then
/// « Vol … », « Acheteurs x % », « Vendeurs y % ».
fn legende_textes(b: &Bar, real: &Bar, exact: usize) -> (String, [String; 3]) {
    let pct = if b.open != 0.0 { (b.close / b.open - 1.0) * 100.0 } else { 0.0 };
    let info = format!(
        "O {}  H {}  L {}  C {}  {:+.2} %",
        format_price(b.open, exact),
        format_price(b.high, exact),
        format_price(b.low, exact),
        format_price(b.close, exact),
        pct,
    );
    let (buy, sell) = volume_candles::buyers_sellers(real);
    (
        info,
        [
            format!("Vol {}", format_volume(b.volume)),
            format!("Acheteurs {buy} %"),
            format!("Vendeurs {sell} %"),
        ],
    )
}

/// 🏷️ The legend's lines over the chart as [`render`] draws them (chart coordinates): no table is placed under them.
pub fn legende_zones(c: &Chart) -> Vec<shapes::Rect> {
    let shown = series(c.bars, &c.style);
    let src: &[Bar] = if matches!(c.style, CandleStyle::HeikinAshi { .. }) { c.bars } else { &shown };
    let (pw, ph) = (c.viewport.plot_w(), c.viewport.plot_h());
    legende_rects(c, &Text::default(), &shown, src, data_decimals(c.bars), pw, ph)
}

/// 📍 The legend at the top left, one rectangle per line (title, O H L C, Vol · Acheteurs · Vendeurs, « ⌄ N »), each
/// as wide as its own text: a pill can stay at its wick's height right of a short line.
fn legende_rects(c: &Chart, text: &Text, bars: &[Bar], src: &[Bar], exact: usize, pw: f64, ph: f64) -> Vec<shapes::Rect> {
    let dl = haut_legende();
    let ligne = |y: f64, h: f64, w: f64| shapes::Rect::new(0.0, y, 12.0 + w + 12.0, h);
    let mut v = vec![];
    if let Some(t) = &c.title {
        let (gras, tp) = if c.navigation {
            (Text::graisse(crate::text::Graisse::Bold), 12.5)
        } else {
            (Text::default(), 16.0)
        };
        let room = (pw - 12.0 - if c.live.is_some() { 34.0 } else { 8.0 }).max(40.0);
        let w = gras.width(&gras.fit(t, tp, room), tp) as f64 + if c.live.is_some() { 26.0 } else { 0.0 };
        // (from the top: the title moves down with the legend)
        v.push(ligne(0.0, 30.0 + dl, w));
    }
    let petite = actions::tactile() && ph < LEGENDE_MIN_H;
    if let (Some(b), Some(r), true) = (bars.last(), src.last(), c.legende && !petite) {
        let (info, [vol, buy, sell]) = legende_textes(b, r, exact);
        let ligne2 = text.width(&vol, 12.5) + 14.0 + text.width(&buy, 12.5) + 10.0 + text.width(&sell, 12.5);
        v.push(ligne(30.0 + dl, 22.0, text.width(&info, 12.5) as f64));
        v.push(ligne(52.0 + dl, 20.0, ligne2 as f64));
    }
    // « ⌄ N » of the indicators (its unfolded list: `Chart::garder`)
    // (👆 by the finger it is there with none, « ⌄ 0 »: n° 40, a wide banner hid it)
    if c.indicators.is_some_and(|o| !o.on.is_empty() || !o.scripts.is_empty()) || actions::tactile() {
        v.push(ligne(72.0 + dl, 28.0, 52.0));
    }
    v
}

/// 🧱 The bottom Argos Murs draws its bottom pills against: the plot's bottom, or (📟 tablet by the finger) just above
/// the buttons floating there (`Chart::boutons_bas`), 6 points between the pills and them.
fn murs_bas(c: &Chart, pw: f64, ph: f64) -> f64 {
    // (murs::bas leaves 6 / 14 points under its pills: a narrow / wide plot)
    let sous = if pw < 560.0 { 6.0 } else { 14.0 };
    let bas = match c.boutons_bas {
        Some(y) => (y - 6.0 + sous).min(ph),
        None => ph,
    };
    // 📅 the economic calendar's rounds at the bottom of the price pane: the pills stay above their row, 6 points
    // between them (the rounds are never covered)
    if c.events.is_empty() {
        bas
    } else {
        bas.min(econ::haut_rangee(ph) - 6.0 + sous)
    }
}

/// 🧱 Where Argos Murs' two bottom pills are (chart coordinates, the plot's width).
pub fn murs_pilules_bas(c: &Chart) -> shapes::Rect {
    let (pw, ph) = (c.viewport.plot_w(), c.viewport.plot_h());
    let bas = murs_bas(c, pw, ph);
    let h = if pw < 560.0 { 22.0 } else { 32.0 };
    shapes::Rect::new(0.0, bas - murs::bas_hauteur(pw), pw, h)
}

/// Dessine la chart. L'image fait `viewport.width × viewport.height`.
/// 🧱 Bottom of the price pane: the plot height `ph` less the indicator panels under it.
fn prix_bas(ph: f64, ind: Option<&indicators::IndOut>) -> f64 {
    (ph - ind.map_or(0.0, |o| o.osc_h())).max(0.0)
}

pub fn render(c: &Chart) -> Pixmap {
    render_inner(c, true).0
}

/// 🎯 The chart without its mouse layer and without what is drawn over it, and that mouse layer apart
/// ([`Reticule`]); [`finir`] draws both. The caller can keep this image while only the mouse moves, and run
/// [`finir`] on a copy for each new mouse position.
pub fn render_fond(c: &Chart) -> (Pixmap, Option<Reticule>) {
    render_inner(c, false)
}

fn render_inner(c: &Chart, with_layer: bool) -> (Pixmap, Option<Reticule>) {
    let mut reticule = None;
    let mut murs_barres: Vec<murs::Barre> = vec![];
    let mut murs_survol: Option<murs::Survol> = None;
    let vp = c.viewport;
    let th = c.theme;
    let mut pm = crate::px::new(vp.width.max(1), vp.height.max(1)).expect("taille d'image");
    pm.fill(Color::from_rgba8(th.background.0, th.background.1, th.background.2, 255));
    let text = Text::default();
    LEGENDE_BANDEAU.with(|l| l.borrow_mut().clear());
    // 📴 « Hors ligne » under the title, with or without candles (👆 by the finger: the failed loading's banner, now
    // shown there without its « F5 pour réessayer », went over it)
    if c.hors_ligne && actions::tactile() {
        let w = text.width("📴 Hors ligne · bougies du cache, reconnexion automatique", 12.5) as f64 + 20.0;
        let r = shapes::Rect::new(0.0, 34.0, 10.0 + w.min(vp.plot_w() - 20.0).max(60.0) + 6.0, 24.0);
        LEGENDE_BANDEAU.with(|l| l.borrow_mut().push(r));
    }
    DIRECT.with(|d| d.set(None));
    indicators::set_heights(&c.settings.bands);
    indicators::set_script_heights(&c.settings.script_bands);
    indicators::set_views(&c.settings.band_view, &c.settings.script_view);
    let (pw, ph) = (vp.plot_w(), vp.plot_h());
    // 📐 écran bas : les oscillateurs rétrécissent, la chart des bougies garde au moins 45 % de la hauteur
    match c.indicators {
        Some(o) => indicators::fit_bands(o, ph),
        None => indicators::fit_bands(&indicators::IndOut::default(), ph),
    }
    // (🔍 the navigation pill is always there: the indicator tables keep clear of it, like of each other)
    if c.navigation {
        indicators::reserver(navbar::rect(&vp, ph, c.pad.is_some()));
    }
    // (🧱 Argos Murs' bottom pills, « Carnets en direct » and its « Perp · Comptant » key: the tables keep above them,
    // the multi-UT table included); its wall pills hold the right side: right-corner tables go left
    // (`indicators::table_corner_with_walls`)
    indicators::set_murs_a_droite(c.murs.is_some() && c.indicators.is_some());
    if let (Some(m), Some(o)) = (c.murs.as_ref(), c.indicators) {
        let v = indicators::values(&o.inputs, indicators::Ind::ArgosMurs);
        let p = argos_script::inputs::Inputs {
            params: &murs::PARAMS,
            vals: &v,
        };
        for r in murs::bas_rects(&text, m, &p, pw, murs_bas(c, pw, prix_bas(ph, c.indicators))) {
            indicators::reserver(r);
        }
    }
    let shown = series(c.bars, &c.style);
    let bars: &[Bar] = &shown;
    // vraies bougies alignées sur l'affichage (HA garde les mêmes heures ; Renko : les briques elles-mêmes)
    let src: &[Bar] = if matches!(c.style, CandleStyle::HeikinAshi { .. }) { c.bars } else { bars };
    let n = bars.len();
    let slots = slots(c.bars, &c.style);

    let line_style = matches!(
        c.style,
        CandleStyle::Line | CandleStyle::Area | CandleStyle::Baseline | CandleStyle::LineMarkers | CandleStyle::StepLine | CandleStyle::HlcArea
    );

    if let Some((first, last)) = vp.visible(&slots) {
        let scale = build_scale(c, bars, first, last);
        let o = c.settings.scale;
        let base_y = (ph / 2.0) as f32;
        // prix exacts (dernier prix, réticule, O / H / L / C) : la précision du symbole
        let exact = data_decimals(c.bars);
        // 📋 the tables start below the legend's lines (O H L C, Vol…) and never go under what the app lays over the
        // chart (its unfolded list, the floating toolbar): reserved before they are placed
        for r in legende_rects(c, &text, bars, src, exact, pw, ph).into_iter().chain(c.garder.iter().copied()) {
            indicators::reserver(r);
            LEGENDE_BANDEAU.with(|l| l.borrow_mut().push(r));
        }
        // 📋 (tableaux-1) the cell's unfolded legend list: no table under it (T6); the news row and Argos Murs' pills
        // at full length (the last frame's: they are drawn after the tables): a moved table keeps clear of them too
        for r in RESERVE_TABLEAUX.with(|v| v.borrow().clone()) {
            indicators::reserver(r);
        }
        let bas_prix = prix_bas(ph, c.indicators);
        tableaux::set_zone_prix(shapes::Rect::new(0.0, 0.0, pw, bas_prix));
        // 👆 (tour 2, F2) a table selected by the finger: its hint pill takes the row of Argos Murs' « Carnets » pill
        // (hidden meanwhile); nothing goes under it (no table, wall pill, high / low pill, round)
        let bulle = tableaux::bulle_place(&text, pw, murs_bas(c, pw, bas_prix));
        tableaux::set_bulle(bulle);
        let bulle_vue: Option<shapes::Rect> = tableaux::choisi().then_some(bulle);
        if let Some(b) = bulle_vue {
            indicators::reserver(b);
            tableaux::obstacle(b);
        }
        // (F10: the cell's unfolded legend list, which the tables keep clear of, the high / low pills too)
        let liste_ouverte: Vec<shapes::Rect> = RESERVE_TABLEAUX.with(|v| v.borrow().clone());
        if !c.events.is_empty() {
            if let Some(r) = econ::rangee_zone(bas_prix, pw) {
                tableaux::obstacle(r);
            }
        }
        if c.murs.is_some() {
            for r in murs::pilules_pleines() {
                tableaux::obstacle(r);
            }
        }
        // Pourcent / Indexé sur 100 : par rapport à la clôture de la première bougie visible
        let base = bars[first].close;
        let to_lab = |p: f64| match o.mode {
            ScaleMode::Percent => (p / base - 1.0) * 100.0,
            ScaleMode::Indexed => p / base * 100.0,
            _ => p,
        };
        let from_lab = |l: f64| match o.mode {
            ScaleMode::Percent => base * (1.0 + l / 100.0),
            ScaleMode::Indexed => base * l / 100.0,
            _ => l,
        };
        let price_text = |p: f64, dec: usize| scale_text(o.mode, base, p, dec);
        let (step, ticks) = match o.mode {
            ScaleMode::Percent | ScaleMode::Indexed => {
                let (lo, hi) = scale.price_range();
                let (a, b) = (to_lab(lo).min(to_lab(hi)), to_lab(lo).max(to_lab(hi)));
                let st = nice_step((b - a) * 60.0 / ph);
                let mut v = (a / st).ceil() * st;
                let mut t = vec![];
                while v <= b {
                    t.push(from_lab(v));
                    v += st;
                }
                (st, t)
            }
            _ => scale.ticks(60.0),
        };
        let dec = decimals_for(step);

        // grille horizontale + prix ; comme TradingView, pas de chiffre de l'échelle sous l'étiquette du dernier prix
        // (prix + compte à rebours : jusqu'à 37 px de haut, plus une marge)
        let label_band = c
            .bars
            .last()
            .filter(|_| o.last_price_label)
            .map(|b| scale.y_of(b.close))
            .map(|y| (y - 10.0 - 8.0, y - 10.0 + 37.0 + 6.0));
        // 🧱 F1k: with Argos Murs on, the scale's labels wait for the walls' price tags (known once the walls are
        // drawn): a label a tag touches is left out, the others are drawn under everything else on the scale
        let murs_axe = c.murs.is_some() && c.indicators.is_some();
        let mut graduations: Vec<(shapes::Rect, String)> = vec![];
        let mut axe_pris: Vec<shapes::Rect> = vec![];
        for &p in &ticks {
            let y = scale.y_of(p).round();
            if o.grid {
                fill(&mut pm, 0.0, y, pw, 1.0, th.grid, 1.0);
            }
            if label_band.is_some_and(|(a, b)| y >= a && y <= b) {
                continue;
            }
            let s = price_text(p, dec);
            graduations.push((shapes::Rect::new(pw + 6.0, y - 8.0, text.width(&s, 12.0) as f64 + 4.0, 16.0), s));
        }
        if !murs_axe {
            for (r, s) in &graduations {
                text.draw(&mut pm, s, ((r.x + 2.0) as f32, (r.y + 12.0) as f32), 12.0, th.text, Align::Left);
            }
        }
        ECHELLE.with(|e| *e.borrow_mut() = (if murs_axe { vec![] } else { graduations.iter().map(|g| g.0).collect() }, vec![]));

        // jour civil (dans le fuseau affiché) d'une heure UTC
        let day = |t: i64| (t + c.tz_offset_s).div_euclid(86_400);

        // arrêts de session : ligne verticale à chaque changement de jour
        if c.settings.session_breaks {
            for i in (first + 1)..=last {
                if day(bars[i].time) != day(bars[i - 1].time) {
                    let x = ((vp.x_of(i - 1, &slots) + vp.x_of(i, &slots)) / 2.0).round();
                    let mut k = 0.0;
                    while k < ph {
                        fill(&mut pm, x, k, 1.0, 4.0, Rgb(0x42, 0x6b, 0xd4), 0.9);
                        k += 8.0;
                    }
                }
            }
        }

        // grille verticale + heures / jours
        let interval = time_interval(c.tf_seconds, vp.bar_spacing, 90.0);
        let mut prev_bucket = None;
        // au moins 80 px entre deux étiquettes (Renko : les briques ne suivent pas le temps régulièrement)
        let mut last_x = f64::NEG_INFINITY;
        for (i, b) in bars.iter().enumerate().take(last + 1).skip(first) {
            let t = b.time + c.tz_offset_s;
            let (bucket, label) = time_bucket(t, interval);
            if prev_bucket.is_some_and(|b| b != bucket) {
                let x = vp.x_of(i, &slots).round();
                if x >= 0.0 && x <= pw && x - last_x >= 80.0 {
                    last_x = x;
                    if o.grid {
                        fill(&mut pm, x, 0.0, 1.0, ph, th.grid, 1.0);
                    }
                    // (🖥 never past the plot edge: nothing slides under the corner cell and its Argos AI pill)
                    if x + text.width(&label, 12.0) as f64 / 2.0 <= pw {
                        text.draw(&mut pm, &label, (x as f32, (ph + 18.0) as f32), 12.0, th.text, Align::Center);
                    }
                }
            }
            prev_bucket = Some(bucket);
        }
        // le futur (à droite de la dernière bougie) : les heures à venir, une place par unité de temps (sauf Renko)
        if last + 1 == n && !matches!(c.style, CandleStyle::Renko(_)) {
            for j in 1.. {
                let x = future_x(&vp, j).round();
                if x > pw {
                    break;
                }
                let t = bars[last].time + j as i64 * c.tf_seconds + c.tz_offset_s;
                let (bucket, label) = time_bucket(t, interval);
                if prev_bucket.is_some_and(|b| b != bucket) && x >= 0.0 && x - last_x >= 80.0 {
                    last_x = x;
                    if o.grid {
                        fill(&mut pm, x, 0.0, 1.0, ph, th.grid, 1.0);
                    }
                    // (🖥 never past the plot edge: nothing slides under the corner cell and its Argos AI pill)
                    if x + text.width(&label, 12.0) as f64 / 2.0 <= pw {
                        text.draw(&mut pm, &label, (x as f32, (ph + 18.0) as f32), 12.0, th.text, Align::Center);
                    }
                }
                prev_bucket = Some(bucket);
            }
        }

        // barre de temps, à droite : horloge (dans la barre du bas quand elle est là)
        // (📱 not by the finger: ƒx and the round Argos AI button sit there; Jee, 21 Nov, list 2)
        if c.now_s > 0 && vp.footer == 0 && !actions::tactile() {
            let clock = clock_text(c.now_s, c.tz_offset_s);
            let x0 = pw - text.width(&clock, 12.0) as f64 - 16.0;
            fill(&mut pm, x0, ph + 1.0, pw - x0, vp.time_axis as f64 - 1.0, th.background, 1.0);
            text.draw(&mut pm, &clock, ((pw - 8.0) as f32, (ph + 18.0) as f32), 12.0, th.text, Align::Right);
        }
        // 🧱 l'axe du temps est gardé tel quel : les bougies, l'orderflow et les courbes qui sortent par le bas (prix
        // déplacés ou zoomés à la main) ne débordent plus dessus ; il est reposé après les indicateurs
        let axe_temps = crate::px::int_rect(0.0, ph + 1.0, pw, vp.time_axis as f64 - 1.0)
            .filter(|_| vp.time_axis > 1)
            .and_then(|r| pm.clone_rect(r));

        // bougies
        let vc = match &c.style {
            CandleStyle::Volume(s) => Some(volume_candles::compute(c.bars, s)),
            _ => None,
        };
        let sp = vp.bar_spacing;
        let ohlc = c.style == CandleStyle::Bars;
        if c.style == CandleStyle::HlcArea {
            let pts: Vec<(f32, f32, f32, f32)> = (first..=last)
                .map(|i| {
                    let b = &bars[i];
                    let y = |p: f64| scale.y_of(p) as f32;
                    (vp.x_of(i, &slots) as f32, y(b.high), y(b.close), y(b.low))
                })
                .collect();
            draw_hlc_area(&mut pm, &pts, ph as f32);
        } else if line_style {
            let pts: Vec<(f32, f32)> = (first..=last).map(|i| (vp.x_of(i, &slots) as f32, scale.y_of(bars[i].close) as f32)).collect();
            draw_line_series(&mut pm, &pts, &c.style, base_y, ph as f32);
        }
        // 🔥 heatmap du carnet : derrière les bougies, une bande par niveau, plus vive pour les gros murs
        if let (Some(h), Some(o)) = (c.book_heat, c.indicators) {
            let v = indicators::values(&o.inputs, indicators::Ind::BookHeat);
            let p = argos_script::inputs::Inputs {
                params: &bookheat::PARAMS,
                vals: &v,
            };
            let top = h.ceiling(p.get("ceil"));
            let (thr, keep, op) = (p.get("thr"), p.get("levels").max(1.0) as usize, (p.get("op") / 100.0).clamp(0.05, 1.0) as f32);
            let hp = (scale.y_of(0.0) - scale.y_of(h.tick)).abs().max(1.0);
            for (&t, col) in h.cols.range(bars.get(first).map_or(i64::MIN, |b| b.time)..) {
                let Ok(i) = bars.binary_search_by_key(&t, |b| b.time) else { continue };
                if i > last {
                    break;
                }
                let (x, w) = (vp.x_of(i, &slots), (vp.bar_spacing * slots.size(i)).max(1.0));
                let mut lv: Vec<(i64, f64)> = col.iter().map(|(&k, &u)| (k, u)).collect();
                lv.sort_by(|a, b| b.1.total_cmp(&a.1));
                for (k, usd) in lv.into_iter().take(keep) {
                    let a = bookheat::BookHeat::intensity(usd, top, thr);
                    if a <= 0.0 {
                        continue;
                    }
                    let y = scale.y_of(k as f64 * h.tick);
                    fill(&mut pm, x - w / 2.0, y - hp / 2.0, w, hp, bookheat::color(a), op * (0.35 + 0.65 * a as f32));
                }
            }
        }
        // 🧱 Argos Murs: thin bars of the summed book against the price scale, behind the candles
        if let (Some(m), Some(o)) = (c.murs.as_ref(), c.indicators) {
            let v = indicators::values(&o.inputs, indicators::Ind::ArgosMurs);
            let p = argos_script::inputs::Inputs {
                params: &murs::PARAMS,
                vals: &v,
            };
            murs_barres = murs::barres(&mut pm, m, &p, pw, prix_bas(ph, c.indicators), &|px: f64| scale.y_of(px));
        }
        // 💥 carte des liquidations (estimation) : derrière les bougies
        if let (Some(v), Some(o)) = (c.liq_map, c.indicators) {
            let vals = indicators::values(&o.inputs, indicators::Ind::LiqMap);
            let times: Vec<i64> = bars.iter().map(|b| b.time).collect();
            let x_of_time = |t: i64| -> Option<f64> {
                let i = match times.binary_search(&t) {
                    Ok(i) => i,
                    Err(0) => return None,
                    Err(i) => i - 1,
                };
                Some(vp.x_of(i, &slots))
            };
            let y_of = |p: f64| scale.y_of(p);
            let now_x = vp.x_of(n - 1, &slots) + vp.bar_spacing / 2.0;
            let f = liqview::Frame {
                x_of_time: &x_of_time,
                y_of: &y_of,
                pw,
                ph,
                now_x,
            };
            liqview::draw(&mut pm, &text, &f, v, &vals);
            // (its tooltip under the mouse is in the mouse layer, `Reticule`)
        }
        if let Some(note) = c.book_note {
            text.draw(&mut pm, note, (8.0, (ph - 8.0) as f32), 11.5, th.text, Align::Left);
        }
        // TPO : les profils de lettres remplacent les bougies
        // 🔭 très dézoomé (moins de 1,5 px par bougie) : une bougie par colonne de pixel, faite des bougies qui y
        // tombent (ouverture de la première, plus haut, plus bas, clôture de la dernière), comme TradingView ; net et
        // rapide même avec des dizaines de milliers de bougies
        let dense = sp < 1.5 && !line_style && c.style != CandleStyle::Tpo;
        if dense {
            let mut col: Option<(i64, f64, f64, f64, f64)> = None;
            let draw_col = |pm: &mut Pixmap, (cx, o, h, l, cl): (i64, f64, f64, f64, f64)| {
                let (yh, yl) = (scale.y_of(h), scale.y_of(l));
                let (yh, yl) = (yh.min(yl).round(), yh.max(yl).round());
                fill(pm, cx as f64, yh, 1.0, (yl - yh).max(1.0), if cl >= o { th.bull } else { th.bear }, 1.0);
            };
            for (i, b) in bars.iter().enumerate().take(last + 1).skip(first) {
                let cx = vp.x_of(i, &slots).floor() as i64;
                col = match col {
                    Some(k) if k.0 == cx => Some((cx, k.1, k.2.max(b.high), k.3.min(b.low), b.close)),
                    Some(k) => {
                        draw_col(&mut pm, k);
                        Some((cx, b.open, b.high, b.low, b.close))
                    }
                    None => Some((cx, b.open, b.high, b.low, b.close)),
                };
            }
            if let Some(k) = col {
                draw_col(&mut pm, k);
            }
        }
        for i in (first..=last).filter(|_| !dense && !line_style && c.style != CandleStyle::Tpo) {
            let b = &bars[i];
            let x = vp.x_of(i, &slots);
            let bull = b.close >= b.open;
            let (color, alpha, width, hollow) = match &vc {
                Some(v) => (
                    palette::on_chart(Rgb::hex(&v[i].color)),
                    v[i].opacity as f32,
                    candle_width(sp * slots.size(i)),
                    v[i].hollow,
                ),
                None if c.style == CandleStyle::Hollow => {
                    let up = b.close >= if i > 0 { bars[i - 1].close } else { b.open };
                    (if up { th.bull } else { th.bear }, 1.0, (sp * 0.7).round().max(1.0), b.close > b.open)
                }
                // footprint : bougie fine au milieu, les rangées de chaque côté
                None if c.style == CandleStyle::Footprint => (if bull { th.bull } else { th.bear }, 1.0, (sp * 0.12).round().max(1.0), false),
                None => (if bull { th.bull } else { th.bear }, 1.0, (sp * 0.7).round().max(1.0), false),
            };
            // bougies vector (TVD Pro) : couleur imposée par l'indicateur
            let color = c.indicators.and_then(|o| o.bar_color(i)).unwrap_or(color);
            // haut / bas à l'écran (échelle inversée : le haut du prix est en bas)
            let (yh, yl) = (scale.y_of(b.high), scale.y_of(b.low));
            let (yh, yl) = (yh.min(yl), yh.max(yl));
            let (yo, yc) = (scale.y_of(b.open), scale.y_of(b.close));
            let cx = x.round();
            let up_prev = b.close >= if i > 0 { bars[i - 1].close } else { b.open };
            let prev_color = if up_prev { th.bull } else { th.bear };
            match c.style {
                CandleStyle::Columns => {
                    let w = if sp >= 3.0 { sp.floor() - 1.0 } else { sp.floor().max(1.0) };
                    let top = yc.round();
                    // la colonne part du bas de la zone (du haut si l'échelle est inversée)
                    let (y0, y1) = if o.invert { (0.0, top) } else { (top, ph) };
                    fill(&mut pm, (cx - (w / 2.0).floor()).round(), y0, w, y1 - y0, prev_color, 0.5);
                    continue;
                }
                CandleStyle::HiLo => {
                    let w = (sp * 0.7).round().max(1.0);
                    let top = yh.round();
                    fill(&mut pm, (cx - (w / 2.0).floor()).round(), top, w, (yl - yh).round().max(1.0), LINE_BLUE, 1.0);
                    let (hs, ls) = (format_price(b.high, exact), format_price(b.low, exact));
                    if sp >= text.width(&hs, 11.0).max(text.width(&ls, 11.0)) as f64 + 6.0 {
                        text.draw(&mut pm, &hs, (cx as f32, (top - 4.0) as f32), 11.0, LINE_BLUE, Align::Center);
                        text.draw(&mut pm, &ls, (cx as f32, (yl.round() + 13.0) as f32), 11.0, LINE_BLUE, Align::Center);
                    }
                    continue;
                }
                CandleStyle::HlcBars => {
                    let tick = bar_tick(sp);
                    fill(&mut pm, cx, yh.round(), 1.0, (yl - yh).round(), prev_color, 1.0);
                    fill(&mut pm, cx + 1.0, yc.round(), tick, 1.0, prev_color, 1.0);
                    continue;
                }
                _ => {}
            }
            let top = yo.min(yc).round();
            let h = ((yo - yc).abs().round()).max(1.0);
            let w = width.max(1.0);
            let left = (cx - (w / 2.0).floor()).round();
            if ohlc {
                let tick = bar_tick(sp);
                fill(&mut pm, cx, yh.round(), 1.0, (yl - yh).round(), color, alpha);
                fill(&mut pm, cx - tick, yo.round(), tick, 1.0, color, alpha);
                fill(&mut pm, cx + 1.0, yc.round(), tick, 1.0, color, alpha);
            } else if hollow && w >= 3.0 && h >= 3.0 {
                // corps vide : contour seulement, la mèche s'arrête au contour
                fill(&mut pm, cx, yh.round(), 1.0, top - yh.round(), color, alpha);
                fill(&mut pm, cx, top + h, 1.0, yl.round() - (top + h), color, alpha);
                fill(&mut pm, left, top, w, 1.0, color, alpha);
                fill(&mut pm, left, top + h - 1.0, w, 1.0, color, alpha);
                fill(&mut pm, left, top, 1.0, h, color, alpha);
                fill(&mut pm, left + w - 1.0, top, 1.0, h, color, alpha);
            } else {
                fill(&mut pm, cx, yh.round(), 1.0, (yl - yh).round(), color, alpha);
                fill(&mut pm, left, top, w, h, color, alpha);
            }
        }

        // footprint et TPO (types de chart comme LuxAlgo)
        if matches!(c.style, CandleStyle::Footprint | CandleStyle::Tpo) {
            let g = indicators::Geo {
                vp: &vp,
                scale: &scale,
                slots: &slots,
                first,
                last,
                bars,
                price_h: ph,
                tz_offset_s: c.tz_offset_s,
            };
            if c.style == CandleStyle::Tpo {
                let t = c.settings.tpo;
                let set = argos_script::indicators::tpo::TpoSet {
                    block: t.block,
                    va: t.va as f64,
                    ib: t.ib,
                };
                tpo::draw(
                    &mut pm,
                    &text,
                    &g,
                    &argos_script::indicators::tpo::compute(&bars[..=last], c.tf_seconds, set),
                    t.letters,
                    t.blocks,
                );
            } else {
                use argos_script::indicators::flow;
                let mut vals = argos_script::inputs::defaults(&flow::FOOTPRINT_PARAMS);
                if let Some(k) = flow::FOOTPRINT_PARAMS.iter().position(|p| p.key == "nb") {
                    vals[k] = (last - first + 1) as f64;
                }
                let reel = c.indicators.map_or(&[][..], |o| o.reel.as_slice());
                indicators::draw_footprint(&mut pm, &text, &g, &flow::footprint(&bars[..=last], &vals, reel), &vals);
            }
        }

        // indicateurs natifs (sous les dessins) ; l'échelle de chaque bande sert aussi au réticule
        let mut band_axes: Vec<indicators::BandAxis> = vec![];
        if let Some(out) = c.indicators {
            let g = indicators::Geo {
                vp: &vp,
                scale: &scale,
                slots: &slots,
                first,
                last,
                bars: src,
                price_h: vp.plot_h() - out.osc_h(),
                tz_offset_s: c.tz_offset_s,
            };
            // (tour 2, F4: the tables on their own layer, put over everything plotted once Argos Murs is drawn)
            tableaux::differer(true);
            band_axes = indicators::draw(&mut pm, &text, &g, out);
            indicators::draw_axes(&mut pm, &text, &g, &band_axes, th.background);
        }
        if let Some(a) = &axe_temps {
            crate::px::put(&mut pm, 0.0, ph + 1.0, a);
        }

        // dernier prix : où se pose son étiquette (les murs et les autres prix de l'échelle l'évitent), et si le compte
        // à rebours l'allonge
        let real = matches!(c.style, CandleStyle::HeikinAshi { real_last_price: true });
        let lb = if real { &src[n - 1] } else { &bars[n - 1] };
        let last_y = scale.y_of(lb.close).round();
        let left = c.bars.last().map_or(0, |b| close_of(b.time, c.tf_seconds) - c.now_s);
        let span = c.bars.last().map_or(0, |b| close_of(b.time, c.tf_seconds) - b.time);
        let countdown =
            c.settings.labels.countdown && o.last_price_label && c.now_s > 0 && !matches!(c.style, CandleStyle::Renko(_)) && left > 0 && left <= span;
        // place prise par cette étiquette sur l'échelle (1 px de marge)
        let (band_top, band_bot) = if o.last_price_label && (0.0..ph).contains(&last_y) {
            (last_y - 11.0, last_y - 9.0 + if countdown { 37.0 } else { 20.0 })
        } else {
            (last_y, last_y)
        };
        // 🏷 the price labels of the scale, drawn together once laid out (see `ecarter`)
        let mut tags: Vec<Tag> = vec![];

        // objets dessinés (moteur d'objets)
        let times: Vec<i64> = bars.iter().map(|b| b.time).collect();
        let geo = drawings::Geo {
            mouse: c.crosshair,
            vp: &vp,
            scale: &scale,
            slots: &slots,
            times: &times,
            bars,
            tf: c.tf_seconds,
        };
        // les dessins se font sur une couche à part, coupée à la zone des bougies (rien ne déborde sur les échelles) ;
        // leurs prix sur l'échelle (lignes horizontales, positions) se posent ensuite
        if !c.drawings.is_empty() {
            let mut layer = Pixmap::new(pm.width(), pm.height()).unwrap_or_else(|| pm.clone()); // même taille en pixels
            for (i, d) in c.drawings.iter().enumerate() {
                d.draw(&mut layer, &text, &geo, &|p| price_text(p, exact), c.selected == Some(i));
            }
            let plot = crate::px::int_rect(0.0, 0.0, pw.max(1.0), ph.max(1.0)).and_then(|r| layer.clone_rect(r));
            if let Some(plot) = plot {
                crate::px::put(&mut pm, 0.0, 0.0, &plot);
            }
            for (price, col) in c.drawings.iter().flat_map(|d| d.axis_labels(&geo)) {
                let y = scale.y_of(price).round();
                if (0.0..ph).contains(&y) {
                    tags.push(Tag::plein(y, col, price_text(price, exact)));
                }
            }
        }
        // 🐋 bulles des baleines (couche coupée à la zone des bougies, comme les dessins)
        if !c.whales.is_empty() {
            let mut layer = Pixmap::new(pm.width(), pm.height()).unwrap_or_else(|| pm.clone()); // même taille en pixels
            whales::bubbles(&mut layer, &text, &geo, &c.whales);
            if let Some(plot) = crate::px::int_rect(0.0, 0.0, pw.max(1.0), ph.max(1.0)).and_then(|r| layer.clone_rect(r)) {
                crate::px::put(&mut pm, 0.0, 0.0, &plot);
            }
        }
        // 🧪 trades du backtest : ▲ L sous l'entrée (long) ou ▼ S au-dessus (short), en bleu ; rond vert / rouge et %
        // à la sortie (couche coupée à la zone des bougies)
        if !c.trades.is_empty() {
            let mut layer = Pixmap::new(pm.width(), pm.height()).unwrap_or_else(|| pm.clone());
            let blue = palette::on_chart(Rgb(0x29, 0x62, 0xff));
            for t in c.trades.iter().rev().take(250) {
                let (xe, ye) = (geo.x_of(t.entry_t), geo.scale.y_of(t.entry));
                if xe >= -20.0 && xe <= pw + 20.0 {
                    let (tip, base) = if t.long { (ye + 6.0, ye + 16.0) } else { (ye - 6.0, ye - 16.0) };
                    let tri = [
                        (xe as f32, tip as f32),
                        ((xe - 6.0) as f32, base as f32),
                        ((xe + 6.0) as f32, base as f32),
                        (xe as f32, tip as f32),
                    ];
                    shapes::lines(&mut layer, 0.0, 0.0, &[&tri], blue, 2.0);
                    let ly = if t.long { base + 12.0 } else { base - 3.0 };
                    text.draw(&mut layer, if t.long { "L" } else { "S" }, (xe as f32, ly as f32), 11.0, blue, Align::Center);
                }
                let (xx, yx) = (geo.x_of(t.exit_t), geo.scale.y_of(t.exit));
                if xx >= -20.0 && xx <= pw + 20.0 {
                    let col = if t.pct >= 0.0 { palette::up() } else { palette::down() };
                    shapes::round_fill(&mut layer, shapes::Rect::new(xx - 5.0, yx - 5.0, 10.0, 10.0), 5.0, col, 0.9);
                    text.draw(
                        &mut layer,
                        &format!("{:+.1} %", t.pct),
                        ((xx + 8.0) as f32, (yx + 4.0) as f32),
                        10.5,
                        col,
                        Align::Left,
                    );
                }
            }
            if let Some(plot) = crate::px::int_rect(0.0, 0.0, pw.max(1.0), ph.max(1.0)).and_then(|r| layer.clone_rect(r)) {
                crate::px::put(&mut pm, 0.0, 0.0, &plot);
            }
        }
        // 📅 annonces économiques au bas de la zone des bougies (their rounds are drawn after Argos Murs, below; the
        // bubble of the round under the mouse is in the mouse layer)
        let econ_bottom = ph - c.indicators.map_or(0.0, |o| o.osc_h());
        // 📍 inputs prix / heure des indicateurs (lignes et points à glisser)
        if let Some(out) = c.indicators {
            inmark::draw(&mut pm, &text, &geo, out, c.mark);
        }

        // 📍 high / low prices on the chart, next to their wick (where candles have a high and a low)
        if c.settings.labels.high_low_chart && !(line_style || c.style == CandleStyle::Tpo) {
            let glass = glass::over(th.background, glass::lum(th.background) > 0.5);
            let label_w = |p: f64| hilo::pill_w(&text, &price_text(p, exact));
            // (a layer cut to the candles' zone: nothing over the oscillator bands)
            let bas = ph - c.indicators.map_or(0.0, |o| o.osc_h());
            let mut layer = Pixmap::new(pm.width(), pm.height()).unwrap_or_else(|| pm.clone());
            // (kept clear: the legend at the top left, the bottom pills of Argos Murs, the indicators' tables and Lens card)
            let mut eviter: Vec<shapes::Rect> = legende_rects(c, &text, bars, src, exact, pw, ph);
            // (and what the app lays over the chart: the floating toolbar, the legend's unfolded list)
            eviter.extend(c.garder.iter().chain(&c.garder_pilules).copied());
            // (F10: the cell's unfolded legend list; F2: the touch hint pill)
            eviter.extend(liste_ouverte.iter().copied());
            eviter.extend(bulle_vue);
            // (the real rects Argos Murs drew: its wall pills, « Carnets en direct » and its colour key)
            if c.murs.is_some() {
                eviter.extend(murs::zones_dessinees());
            }
            eviter.extend(indicators::tableaux());
            eviter.extend(indicators::carte_lens());
            // (the row of ⚡ and flags at the bottom of the candles: no high / low pill over it)
            if !c.events.is_empty() {
                eviter.extend(econ::rangee_zone(econ_bottom, pw));
            }
            let mut dessines = vec![];
            for m in hilo::marks(bars, first, last, &|i| vp.x_of(i, &slots), &|p| scale.y_of(p), &label_w, pw, &eviter) {
                // (a pill with no free place left is not drawn: it would lie over the title, the legend…)
                if (0.0..bas).contains(&m.tip.1) && !m.sans_place(&eviter) {
                    let ink = if m.high { palette::up() } else { palette::down() };
                    hilo::draw(&mut layer, &text, &m, &price_text(m.price, exact), pw, glass, ink, th.fg);
                    dessines.push(m.pill);
                }
            }
            HILO_DESSINES.with(|h| *h.borrow_mut() = dessines);
            if let Some(plot) = crate::px::int_rect(0.0, 0.0, pw.max(1.0), bas.max(1.0)).and_then(|r| layer.clone_rect(r)) {
                crate::px::put(&mut pm, 0.0, 0.0, &plot);
            }
        }
        // 🧭 « Voir sur la chart » of the Explorer Hyperliquid: an address's entry and liquidation lines, their pills
        // clear of the legend, the app's places, the walls' pills and the row of ⚡ and flags
        if let Some(l) = hlexplorer::lignes_pour(c.bars.as_ptr() as usize) {
            let mut ev: Vec<shapes::Rect> = legende_rects(c, &text, bars, src, exact, pw, ph);
            // (the pills never go up to the title and the legend, whatever their width: under them, edge ↑ included)
            let haut = ev.iter().map(|r| r.y + r.h + 4.0).fold(0.0, f64::max);
            ev.extend(c.garder.iter().chain(&c.garder_pilules).copied());
            ev.extend(liste_ouverte.iter().copied());
            ev.extend(bulle_vue);
            if c.murs.is_some() {
                ev.extend(murs::zones_dessinees());
            }
            if !c.events.is_empty() {
                ev.extend(econ::rangee_zone(econ_bottom, pw));
            }
            ev.extend(indicators::tableaux());
            ev.extend(indicators::carte_lens());
            let p = hlexplorer::lignes_draw(&mut pm, &text, &l, pw, econ_bottom, haut, &|p| scale.y_of(p), &ev);
            hlexplorer::set_pilules(p);
        }
        // ligne pointillée + étiquette sur l'échelle (haut / bas, clôture de la veille)
        let mut level = |p: f64, col: Rgb, line: bool, label: bool| {
            let y = scale.y_of(p).round();
            if !(0.0..ph).contains(&y) {
                return;
            }
            if line {
                let mut x = 0.0;
                while x < pw {
                    fill(&mut pm, x, y, 4.0, 1.0, col, 0.85);
                    x += 8.0;
                }
            }
            if label {
                tags.push(Tag::plein(y, col, price_text(p, exact)));
            }
        };
        // haut et bas de la partie visible
        let (lb_on, ll_on) = (c.settings.labels.high_low, c.settings.lines.high_low);
        if lb_on || ll_on {
            let hi = bars[first..=last].iter().map(|b| b.high).fold(f64::NEG_INFINITY, f64::max);
            let lo = bars[first..=last].iter().map(|b| b.low).fold(f64::INFINITY, f64::min);
            level(hi, Rgb(0x08, 0x99, 0x81), ll_on, lb_on);
            level(lo, Rgb(0xb2, 0x2b, 0x37), ll_on, lb_on);
        }
        // clôture du jour précédent (vraies bougies)
        let (pc_l, pc_line) = (c.settings.labels.prev_close, c.settings.lines.prev_close);
        if pc_l || pc_line {
            let today = c.bars.last().map(|b| day(b.time));
            if let Some(prev) = c.bars.iter().rev().find(|b| Some(day(b.time)) != today) {
                level(prev.close, Rgb(0x5d, 0x60, 0x6b), pc_line, pc_l);
            }
        }

        // 🔔 alertes de prix : ligne pointillée (dorée, ou la couleur choisie), cloche au bord de la chart, prix sur
        // l'échelle ; en pause : grise et pâle ; « Lignes d'alerte » décoché : ni ligne ni cloche (la forme courte,
        // collée à l'échelle, est dessinée par l'app avec son nom)
        let vue = &c.alertes_vue;
        for (&p, pause) in c.alerts.iter().map(|p| (p, false)).chain(vue.pause.iter().map(|p| (p, true))) {
            let y = scale.y_of(p).round();
            if !(0.0..ph).contains(&y) {
                continue;
            }
            let col = vue.ligne(pause);
            if !vue.courtes {
                let mut x = 0.0;
                while x < pw {
                    fill(&mut pm, x, y, 6.0, 1.0, col, if pause { 0.5 } else { 0.85 });
                    x += 10.0;
                }
                phosphor::draw(&mut pm, "bell", pw - 12.0, y - 9.0, 14, col);
            }
            tags.push(Tag {
                y,
                h: 18.0,
                rayon: 4.0,
                col,
                alpha: if pause { 0.6 } else { 1.0 },
                texte: price_text(p, exact),
                taille: 11.5,
                encre: palette::ink_on(col),
            });
        }

        // 🤖 Argos AI: the explanation layer, after the user's drawings and the alert lines (no line
        // runs through its labels), before the orders, the last price and the crosshair
        // (the alert bells at the right edge: no label under them)
        let cloches: Vec<f64> = if c.alertes_vue.courtes {
            vec![]
        } else {
            c.alerts.iter().chain(&c.alertes_vue.pause).map(|&p| scale.y_of(p).round()).collect()
        };
        explication::draw(&mut pm, &text, &geo, c.explication, &cloches);

        // 💼 positions, ordres en attente et brouillon démo : entrée, TP, SL, étiquettes, ✕ (comme TradingView)
        ordres::draw(
            &mut pm,
            &text,
            &|p| scale.y_of(p),
            &|p| price_text(p, exact),
            pw,
            ph,
            vp.price_axis as f64,
            &c.ordres,
        );
        axe_pris.extend(ordres::etiquettes_echelle(&|p| scale.y_of(p), pw, ph, vp.price_axis as f64, &c.ordres));

        // 📖 Bid & Ask : lignes fines et prix sur l'échelle (vente en haut, achat en bas)
        if let Some((bid, ask)) = c.bid_ask {
            for (p, col) in [(bid, palette::up()), (ask, palette::down())] {
                let y = scale.y_of(p).round();
                if !(0.0..ph).contains(&y) {
                    continue;
                }
                if c.settings.lines.bid_ask {
                    let mut x = 0.0;
                    while x < pw {
                        fill(&mut pm, x, y, 3.0, 1.0, col, 0.8);
                        x += 5.0;
                    }
                }
                if c.settings.labels.bid_ask {
                    tags.push(Tag {
                        y,
                        h: 16.0,
                        rayon: 3.0,
                        col,
                        alpha: 0.9,
                        texte: price_text(p, exact),
                        taille: 10.5,
                        encre: palette::ink_on(col),
                    });
                }
            }
        }

        // 🏷 the price labels on the scale (drawings, high / low, previous close, alerts, bid & ask), laid out together:
        // none covers another nor the last price's label (Jee, 19 Nov: « tout reste lisible »)
        for (y, tag) in ecarter(&tags, (band_top < band_bot).then_some((band_top, band_bot))).into_iter().zip(&tags) {
            tag.draw(&mut pm, &text, y, pw, vp.price_axis as f64);
            axe_pris.push(shapes::Rect::new(pw, y - tag.h / 2.0, vp.price_axis as f64, tag.h));
        }

        // 🧱 order-book walls (heatmap option): a faint line across the chart and a SELL / BUY pill on the price scale,
        // under the last-price label (drawn next)
        if let (Some(h), Some(o)) = (c.book_heat, c.indicators) {
            let v = indicators::values(&o.inputs, indicators::Ind::BookHeat);
            let p = argos_script::inputs::Inputs {
                params: &bookheat::PARAMS,
                vals: &v,
            };
            if p.on("walls") {
                let mid = bars[n - 1].close;
                let dec = viewport::data_decimals(bars);
                let walls: Vec<(f64, bookheat::Wall)> = h
                    .walls(mid, p.get("wallsN").max(1.0) as usize, p.get("wallsMult"), p.get("wallsDepth"))
                    .into_iter()
                    .map(|w| (scale.y_of(w.price).round(), w))
                    .filter(|(y, _)| (0.0..ph).contains(y))
                    .collect();
                // pills never overlap each other nor the last-price label (drawn next): the ones above it go up from
                // it, the ones below go down, each at least one pill height from the previous one
                let (mut up, mut down): (Vec<_>, Vec<_>) = walls.into_iter().partition(|(y, _)| *y < last_y);
                up.sort_by(|a, b| b.0.total_cmp(&a.0));
                down.sort_by(|a, b| a.0.total_cmp(&b.0));
                let mut placed = vec![];
                let mut limit = band_top - 9.0;
                for (y, w) in up {
                    let ly = y.min(limit);
                    limit = ly - 19.0;
                    placed.push((y, ly, w));
                }
                let mut limit = band_bot + 9.0;
                for (y, w) in down {
                    let ly = y.max(limit);
                    limit = ly + 19.0;
                    placed.push((y, ly, w));
                }
                for (y, ly, w) in placed {
                    let c = palette::on_chart(p.rgb(if w.sell { "wallsSell" } else { "wallsBuy" }));
                    fill(&mut pm, 0.0, y, pw, 1.0, c, 0.35);
                    let label = w.label(dec);
                    let tw = text.width(&label, 11.0) as f64 + 12.0;
                    let x1 = pw + vp.price_axis as f64 - 2.0;
                    shapes::round_fill(&mut pm, shapes::Rect::new(x1 - tw, ly - 9.0, tw, 18.0), 4.0, c, 0.95);
                    axe_pris.push(shapes::Rect::new(x1 - tw, ly - 9.0, tw, 18.0));
                    text.draw(
                        &mut pm,
                        &label,
                        ((x1 - tw / 2.0) as f32, (ly + 4.0) as f32),
                        11.0,
                        palette::ink_on(c),
                        Align::Center,
                    );
                }
            }
        }

        // (F1i: where the last-price label goes, the walls' price tags keep clear of it)
        let dernier_tag = (o.last_price_label && (0.0..ph).contains(&last_y)).then_some((last_y - 10.0, last_y - 10.0 + if countdown { 37.0 } else { 20.0 }));
        // 🧱 Argos Murs: book bars, grey words and wall pills in one column against the price scale (mockup v5),
        // under the last-price label (drawn next)
        if let (Some(m), Some(o)) = (c.murs.as_ref(), c.indicators) {
            let v = indicators::values(&o.inputs, indicators::Ind::ArgosMurs);
            let p = argos_script::inputs::Inputs {
                params: &murs::PARAMS,
                vals: &v,
            };
            let times: Vec<i64> = bars.iter().map(|b| b.time).collect();
            let x_of = |t_ms: i64| -> Option<f64> {
                let i = match times.binary_search(&(t_ms.div_euclid(1000))) {
                    Ok(i) => i,
                    Err(0) => return None,
                    Err(i) => i - 1,
                };
                Some(vp.x_of(i, &slots))
            };
            let y_of = |px: f64| scale.y_of(px);
            // the bottom pills sit at the bottom of the PRICE pane, above the indicator panels (never over them), and
            // (📟 tablet by the finger) above the buttons floating there
            // (F2: the touch hint pill too, and the « Carnets » row hidden while it is shown)
            murs::set_garder(&c.garder.iter().chain(&c.garder_pilules).copied().chain(bulle_vue).collect::<Vec<_>>());
            murs::set_sans_bas(bulle_vue.is_some());
            // (lot pilules-1: the candles a pill must not touch, wicks included, and the scale's way to write a price)
            let pane = prix_bas(ph, c.indicators);
            // (🧱 H7, 24 Nov: no band is kept for Argos Murs any more, the candles go under the pills; each pill keeps
            // the place and the length it had with the band of before — the candles are given where they were then)
            let avant = (murs::marge_juste(m, &p, pw, m.marge, actions::tactile()) - vp.marge.max(0.0)).max(0.0);
            murs::set_bougies(
                (first..=last)
                    .map(|i| {
                        let (x, w) = (vp.x_of(i, &slots) - avant, (vp.bar_spacing * slots.size(i)).max(1.0));
                        let (yh, yl) = (scale.y_of(bars[i].high), scale.y_of(bars[i].low));
                        shapes::Rect::new(x - w / 2.0, yh.min(yl), w, (yl - yh).abs().max(1.0))
                    })
                    .filter(|r| r.x + r.w >= 0.0 && r.x <= pw && r.y <= pane)
                    .collect(),
            );
            murs::set_axe(c.settings.scale.mode, base, exact);
            let zones = murs::draw(&mut pm, &text, m, &p, pw, murs_bas(c, pw, prix_bas(ph, c.indicators)), &y_of, &x_of);
            // (the bars and pills drawn: the tooltip under the mouse, in the mouse layer)
            murs_survol = Some(murs::Survol {
                bas: prix_bas(ph, c.indicators),
                ..murs::Survol::new(m, &p, std::mem::take(&mut murs_barres), zones, pw)
            });
            // F1i (Jee, 22 Nov: « prix dans la colonne de prix ») each wall's price on the scale, in its colour (two
            // halves for a perp wall and its spot twin), white bold like the last price; never on the last-price label
            // (drawn next, as the crosshair's) nor on another wall's tag (the bigger wall's stays)
            let gras = Text::graisse(crate::text::Graisse::Bold);
            let mis = murs::tags_axe(&murs::etiquettes_axe(), &y_of, 20.0, prix_bas(ph, c.indicators), dernier_tag);
            let tag_rect = |y: f64| shapes::Rect::new(pw + 2.0, y - 10.0, vp.price_axis as f64 - 4.0, 20.0);
            let tag_rects: Vec<shapes::Rect> = mis.iter().map(|(y, _)| tag_rect(*y)).collect();
            // F1k: the scale's labels now, under what is already on the scale; none a wall's tag touches
            let croise = |a: &shapes::Rect, b: &shapes::Rect| a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h;
            let mut graduees = vec![];
            for (r, s) in &graduations {
                if tag_rects.iter().any(|t| croise(t, r)) {
                    continue;
                }
                if axe_pris.iter().any(|t| croise(t, r)) {
                    continue;
                }
                graduees.push(*r);
                text.draw(&mut pm, s, ((r.x + 2.0) as f32, (r.y + 12.0) as f32), 12.0, th.text, Align::Left);
            }
            ECHELLE.with(|e| *e.borrow_mut() = (graduees, tag_rects));
            for (y, t) in mis {
                let r = tag_rect(y);
                let (a, b) = (t.couleurs[0], *t.couleurs.last().unwrap_or(&t.couleurs[0]));
                shapes::round_fill(&mut pm, r, 4.0, a, 1.0);
                if b != a {
                    fill(&mut pm, r.cx(), r.y, r.w / 2.0 - 4.0, r.h, b, 1.0);
                    shapes::round_fill(&mut pm, shapes::Rect::new(r.x + r.w - 8.0, r.y, 8.0, r.h), 4.0, b, 1.0);
                }
                // (a wall off screen, on the row of its pill: « ▲ 65 000 », the arrow drawn like the pill's)
                let mut x = pw + 8.0;
                if let Some((_, haut)) = t.bord {
                    let (a, h) = (4.0f32, 3.5f32);
                    let pts: [(f32, f32); 3] = if haut { [(0.0, -h), (a, h), (-a, h)] } else { [(0.0, h), (a, -h), (-a, -h)] };
                    shapes::poly_fill(&mut pm, x + 4.0, y, &pts, Rgb(255, 255, 255), 1.0);
                    x += 12.0;
                }
                // (F1l: the thousands apart as on the scale's labels, whatever the bold font's narrow space)
                murs::draw_prix(&mut pm, &gras, &text, &t.texte, x, (y + 4.0) as f32, 12.0, Rgb(255, 255, 255));
            }
        } else {
            murs::oublier();
        }
        // 📋 (tour 2, F4) the tables now: over the candles, the indicator lines, the walls' lines and the drawings;
        // under the scale's tags, the legend and what the app lays over the chart (their pills keep clear of them)
        tableaux::poser_calque(&mut pm);
        // 📅 the economic calendar's rounds (⚡ and flags) at the bottom of the candles, after Argos Murs: a round under
        // one of its pills drawn just now is left out (the pill would hide it)
        let econ_spots = if c.events.is_empty() {
            vec![]
        } else {
            let murs = if c.murs.is_some() { murs::zones_dessinees() } else { vec![] };
            econ::draw_marks(&mut pm, &text, &c.events, &|t| geo.x_of(t), pw, econ_bottom, &murs)
        };

        // ≫ F1j (Jee's iPad, 22 Nov: the view far in the future and no way back by the finger): off live, a round
        // glass button at the bottom right of the price pane, left of Argos Murs' band, on no candle, pill or table
        if vp.hors_direct() {
            let d = if pw < 560.0 { 36.0 } else { 44.0 };
            let bas = murs_bas(c, pw, prix_bas(ph, c.indicators)) - 8.0;
            let droite = if vp.marge > 0.0 { vp.bars_w() } else { pw } - 8.0;
            let mut obstacles: Vec<shapes::Rect> = (first..=last)
                .map(|i| {
                    let (x, w) = (vp.x_of(i, &slots), (vp.bar_spacing * slots.size(i)).max(1.0));
                    let (yh, yl) = (scale.y_of(bars[i].high), scale.y_of(bars[i].low));
                    shapes::Rect::new(x - w / 2.0, yh.min(yl), w, (yl - yh).abs().max(1.0))
                })
                .collect();
            obstacles.extend(murs::zones_dessinees());
            obstacles.extend(indicators::tableaux());
            obstacles.extend(c.garder.iter().chain(&c.garder_pilules).copied());
            obstacles.extend(bulle_vue);
            let croise = |a: &shapes::Rect, b: &shapes::Rect| a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h;
            let defaut = shapes::Rect::new(droite - d, bas - d, d, d);
            // (Jee's iPhone, 24 Nov: « la flèche du bas ne marche pas » — it hopped from place to place as Argos Murs'
            // pills and its bottom summary changed, and the finger came down where it had just been: it stays where it
            // is while that place is still free)
            let candidats = (0..6)
                .flat_map(|col| (0..8).map(move |row| (col, row)))
                .map(|(col, row)| shapes::Rect::new(droite - d - col as f64 * (d + 8.0), bas - d - row as f64 * (d + 8.0), d, d))
                .filter(|r| r.x >= 4.0 && r.y >= 40.0);
            let r = DIRECT_AVANT
                .with(|a| a.get())
                .filter(|a| candidats.clone().any(|r| r == *a))
                .into_iter()
                .chain(candidats)
                .find(|r| !obstacles.iter().any(|o| croise(o, r)))
                .unwrap_or(defaut);
            DIRECT_AVANT.with(|a| a.set(Some(r)));
            shapes::round_fill(&mut pm, r, d / 2.0, palette::popup(), 0.92);
            shapes::round_stroke(&mut pm, r, d / 2.0, palette::accent(), 1.6);
            let k = (d / 44.0) as f32;
            let chevron = |dx: f32| [(dx - 3.0 * k, -5.5 * k), (dx + 2.5 * k, 0.0), (dx - 3.0 * k, 5.5 * k)];
            let (a, b) = (chevron(-2.5 * k), chevron(3.5 * k));
            shapes::lines(&mut pm, r.cx(), r.cy(), &[&a[..], &b[..]], palette::WHITE, 2.0);
            DIRECT.with(|x| x.set(Some(r)));
        }

        // ligne et étiquette du dernier prix
        let y = last_y;
        let lc = match c.style {
            CandleStyle::Line | CandleStyle::Area | CandleStyle::LineMarkers | CandleStyle::StepLine => LINE_BLUE,
            CandleStyle::Baseline if y <= base_y as f64 => th.bull,
            CandleStyle::Baseline => th.bear,
            _ if lb.close >= lb.open => th.bull,
            _ => th.bear,
        };
        if (0.0..ph).contains(&y) {
            if o.last_price_line {
                let mut x = 0.0;
                while x < pw {
                    fill(&mut pm, x, y, 2.0, 1.0, lc, 0.9);
                    x += 5.0;
                }
            }
            // nom du symbole à côté du dernier prix (le titre jusqu'au premier « · »)
            if c.settings.labels.symbol_name {
                if let Some(name) = c.title.as_deref().and_then(|t| t.split(" · ").next()) {
                    let w = text.width(name, 12.0) as f64 + 14.0;
                    fill(&mut pm, pw - w - 2.0, y - 10.0, w, 20.0, lc, 1.0);
                    text.draw(&mut pm, name, ((pw - w + 5.0) as f32, (y + 4.0) as f32), 12.0, Rgb(255, 255, 255), Align::Left);
                }
            }
            // (compte à rebours jusqu'à la fermeture de la bougie, dans l'étiquette du dernier prix, sous le prix)
            if o.last_price_label {
                // texte lisible sur la couleur de la bougie (foncé sur une couleur pâle, blanc sinon)
                let ink = palette::ink_on(lc);
                // une étiquette aux coins arrondis dans l'échelle : le prix en gras, le temps restant dessous, plus petit et atténué
                let h = if countdown { 37.0 } else { 20.0 };
                let r = shapes::Rect::new(pw + 2.0, y - 10.0, vp.price_axis as f64 - 4.0, h);
                shapes::round_fill(&mut pm, r, 4.0, lc, 1.0);
                let price = price_text(lb.close, exact);
                for dx in [0.0, 0.6] {
                    text.draw(&mut pm, &price, ((pw + 8.0 + dx) as f32, (y + 4.0) as f32), 12.0, ink, Align::Left);
                }
                if countdown {
                    let mix = |a: u8, b: u8| ((a as u16 * 4 + b as u16) / 5) as u8;
                    let soft = Rgb(mix(ink.0, lc.0), mix(ink.1, lc.1), mix(ink.2, lc.2));
                    text.draw(&mut pm, &countdown_text(left), ((pw + 8.0) as f32, (y + 22.0) as f32), 11.0, soft, Align::Left);
                }
            }
        }

        // 🎯 what follows the mouse (crosshair, O / H / L / C legend, tooltips), on a layer the caller can redraw alone
        let r = Reticule {
            vp,
            slots: slots.clone(),
            scale,
            band_axes,
            bars: bars.to_vec(),
            src: (!std::ptr::eq(src, bars)).then(|| src.to_vec()),
            renko: matches!(c.style, CandleStyle::Renko(_)),
            tf_seconds: c.tf_seconds,
            tz_offset_s: c.tz_offset_s,
            exact,
            dec,
            mode: o.mode,
            base,
            theme: th,
            legende: c.legende,
            liq: c.liq_map.cloned(),
            econ: econ_spots,
            econ_bottom,
            murs: murs_survol.take(),
            fp: c.indicators.and_then(|o| {
                let m = o.fp_multi.clone()?;
                let f = o.footprint.clone().filter(|_| o.shown(indicators::Ind::Footprint))?;
                Some(FpSurvol {
                    m,
                    f,
                    bas: prix_bas(ph, c.indicators),
                })
            }),
        };
        reticule = Some(r);
    }
    if with_layer {
        finir(&mut pm, c, reticule.as_ref());
    }
    (pm, reticule)
}

/// 🏷 A price label on the scale (drawing, high / low, previous close, alert, bid & ask), laid out with the others by
/// [`ecarter`] before it is drawn.
struct Tag {
    /// Its price's height (points).
    y: f64,
    h: f64,
    /// 0: a square box over the whole scale; else a rounded pill 2 points inside it.
    rayon: f64,
    col: Rgb,
    alpha: f32,
    texte: String,
    taille: f32,
    encre: Rgb,
}

impl Tag {
    /// The square white-on-colour label (drawings, high / low, previous close).
    fn plein(y: f64, col: Rgb, texte: String) -> Self {
        Tag {
            y,
            h: 20.0,
            rayon: 0.0,
            col,
            alpha: 1.0,
            texte,
            taille: 12.0,
            encre: Rgb(255, 255, 255),
        }
    }

    /// Draws it centred at `y` (its place given by [`ecarter`]).
    fn draw(&self, pm: &mut Pixmap, text: &Text, y: f64, pw: f64, axis: f64) {
        let top = y - self.h / 2.0;
        if self.rayon <= 0.0 {
            fill(pm, pw, top, axis, self.h, self.col, self.alpha);
        } else {
            shapes::round_fill(pm, shapes::Rect::new(pw + 2.0, top, axis - 4.0, self.h), self.rayon, self.col, self.alpha);
        }
        text.draw(pm, &self.texte, ((pw + 8.0) as f32, (y + 4.0) as f32), self.taille, self.encre, Align::Left);
    }
}

/// 🏷 The centres of `tags` (same order) so that no two cover each other (1 point apart) and none enters `band`, the
/// last price's label (top, bottom), which keeps its exact place. A label touching nothing keeps its exact place;
/// the others move just enough, away from the last price.
fn ecarter(tags: &[Tag], band: Option<(f64, f64)>) -> Vec<f64> {
    let mut out: Vec<f64> = tags.iter().map(|t| t.y).collect();
    let mut order: Vec<usize> = (0..tags.len()).collect();
    order.sort_by(|&a, &b| tags[a].y.total_cmp(&tags[b].y));
    let (top, bot) = band.unwrap_or((f64::INFINITY, f64::INFINITY));
    let mid = (top + bot) / 2.0;
    let (above, below): (Vec<usize>, Vec<usize>) = order.into_iter().partition(|&i| tags[i].y < mid);
    // above the last price (all of them without its label): from the nearest one upwards
    let mut limit = top - 1.0;
    for &i in above.iter().rev() {
        let half = tags[i].h / 2.0;
        out[i] = tags[i].y.min(limit - half);
        limit = out[i] - half - 1.0;
    }
    // below it: from the nearest one downwards
    let mut limit = bot + 1.0;
    for &i in &below {
        let half = tags[i].h / 2.0;
        out[i] = tags[i].y.max(limit + half);
        limit = out[i] + half + 1.0;
    }
    out
}

/// 🎯 Finishes an image of [`render_fond`] for the mouse at `c.crosshair`: the mouse layer, then everything drawn
/// over it (scale borders, title and live dot, « Hors ligne », navigation pill, bottom bar, « Aller à », ⚙ and open
/// menus, banner). [`render`] is exactly `render_fond` then `finir`: the caller can keep the image of `render_fond`
/// while only the mouse (or a hover, a menu, the banner) changes, and finish a copy of it each frame.
pub fn finir(pm: &mut Pixmap, c: &Chart, layer: Option<&Reticule>) {
    let text = Text::default();
    if let Some(r) = layer {
        r.draw(pm, &text, c.crosshair);
    }
    let (vp, th) = (c.viewport, c.theme);
    let (pw, ph) = (vp.plot_w(), vp.plot_h());
    // bordures des échelles (allégées, comme la maquette : un trait discret, pas un cadre)
    fill(pm, pw, 0.0, 1.0, vp.height as f64, th.axis_line, 0.4);
    fill(pm, 0.0, ph, vp.width as f64, 1.0, th.axis_line, 0.4);
    if let Some(t) = &c.title {
        // titre coupé proprement (« … ») avant l'échelle de prix, en gardant la place du voyant du direct
        let room = (pw - 12.0 - if c.live.is_some() { 34.0 } else { 8.0 }).max(40.0);
        // (🖥 maquette PC : 12,5 en gras ; 📱 cell : 16 comme avant)
        let (gras, tp) = if c.navigation {
            (Text::graisse(crate::text::Graisse::Bold), 12.5)
        } else {
            (Text::default(), 16.0)
        };
        let t = &gras.fit(t, tp, room);
        let dl = haut_legende();
        gras.draw(pm, t, (12.0, (24.0 + dl) as f32), tp, th.fg, Align::Left);
        // voyant du direct à droite du titre : vert = branché, orange = reconnexion (comme TradingView)
        // (📱📟 cell et tablette : pas de voyant, la pilule « Flux » de la maquette le dit déjà)
        // (🖥 PC : le voyant reste, même avec la légende toujours là)
        if let Some(on) = c.live.filter(|_| !c.legende || c.navigation) {
            let col = if on { Rgb(0x08, 0x99, 0x81) } else { Rgb(0xf0, 0xa5, 0x00) };
            let (cx, cy) = (12.0 + gras.width(t, tp) as f64 + 14.0, 20.0 + dl);
            let mut p = Paint::default();
            p.set_color_rgba8(col.0, col.1, col.2, 70);
            if let Some(halo) = tiny_skia::PathBuilder::from_circle(cx as f32, cy as f32, 8.0) {
                pm.fill_path(&halo, &p, tiny_skia::FillRule::Winding, crate::px::t(), None);
            }
            p.set_color_rgba8(col.0, col.1, col.2, 255);
            if let Some(dot) = tiny_skia::PathBuilder::from_circle(cx as f32, cy as f32, 4.0) {
                pm.fill_path(&dot, &p, tiny_skia::FillRule::Winding, crate::px::t(), None);
            }
        }
    }
    // 📴 hors ligne : étiquette sous le titre, la chart reste celle du cache
    if c.hors_ligne {
        let t = "📴 Hors ligne · bougies du cache, reconnexion automatique";
        let w = text.width(t, 12.5) as f64 + 20.0;
        let r = crate::shapes::Rect::new(10.0, 34.0, w.min(pw - 20.0).max(60.0), 24.0);
        crate::shapes::round_fill(pm, r, 12.0, Rgb(0xb4, 0x53, 0x09), 0.92);
        text.draw(
            pm,
            &text.fit(t, 12.5, r.w - 20.0),
            ((r.x + 10.0) as f32, (r.y + 16.5) as f32),
            12.5,
            Rgb(0xff, 0xff, 0xff),
            Align::Left,
        );
    }
    // 🔍 the navigation pill, always visible (no menu open), centred at the bottom of the chart
    if !c.context.is_open() && c.navigation {
        navbar::draw(
            pm,
            &text,
            &vp,
            ph, // tout en bas de la pile (chart + panneaux d'indicateurs), juste au-dessus de l'échelle de temps
            c.crosshair,
            &th,
            c.pad.map(|p| p.zen),
        );
    }
    // barre du bas : périodes, « Aller à », horloge
    let clock = (c.now_s > 0).then(|| clock_text(c.now_s, c.tz_offset_s));
    let mouse = c.crosshair.filter(|_| !c.context.is_open() && c.goto.is_none()); // pas de bulle sous « Aller à »
    let chips = if c.now_s > 0 { sessions::chips(c.now_s) } else { vec![] };
    bottombar::draw(pm, &text, &vp, mouse, clock.as_deref(), &chips, &th, c.demo.as_ref());
    // 🌞 thème clair : la barre du bas (dessinée en sombre) passe en clair, comme les menus
    if palette::is_light(theme_index()) && vp.footer > 0 {
        let y0 = vp.plot_h() + vp.time_axis as f64;
        if let Some(mut bar) = crate::px::int_rect(0.0, y0, vp.width as f64, vp.footer as f64).and_then(|r| pm.clone_rect(r)) {
            palette::light_ui(&mut bar);
            crate::px::put(pm, 0.0, y0, &bar);
        }
    }
    if let Some(g) = c.goto.as_ref().filter(|_| !menus_a_part()) {
        goto::draw(pm, &text, &vp, g);
    }
    // bouton ⚙ et menu ouvert (clic droit, ⚙, ➕)
    let om = c.context.open;
    // 📱 pas sur cell ni tablette : la maquette n'a pas ce ⚙ (les réglages sont dans le ⚙ du dock)
    // 🖥 cadre PC (pied flottant, footer 0) : plus de ⚙ dans le coin ; le menu d'échelle reste au clic droit sur l'axe
    if c.navigation && vp.footer > 0 {
        context_menu::draw_gear(pm, &vp, om.is_some_and(|o| o.anchor == context_menu::Anchor::Gear), &th);
    }
    if let Some(context_menu::OpenMenu {
        anchor: context_menu::Anchor::Below(y),
        source: context_menu::Source::Plus,
        ..
    }) = om
    {
        context_menu::draw_plus(pm, &vp, y, true);
    }
    let ctx = context_menu::Ctx {
        settings: &c.settings,
        drawings: c.drawings.len(),
        now: c.now_s,
        tpo: c.style == CandleStyle::Tpo,
    };
    if !menus_a_part() {
        context_menu::draw(pm, &vp, &ctx, &c.context, &th);
    }
    if let Some(b) = &c.banner {
        // une ligne (PC : coupée avec « … » si besoin ; 📱 cell / doigt : sur deux lignes, jamais coupée) et un ✕ : un
        // toucher la ferme, elle part aussi toute seule
        let (r, lignes, size) = banner_lignes(&vp, &text, b);
        // ambre foncé en thème sombre, ambre pâle en thème clair (texte brun foncé)
        let (bg, ink) = if palette::is_light(theme_index()) {
            (Rgb(0xff, 0xf1, 0xcc), Rgb(0x5c, 0x3d, 0x00))
        } else {
            (Rgb(0x5c, 0x3d, 0x0a), Rgb(0xff, 0xd6, 0x8a))
        };
        shapes::round_fill(pm, r, 6.0, bg, 0.95);
        for (k, l) in lignes.iter().enumerate() {
            text.draw(pm, l, ((r.x + 10.0) as f32, (r.y + 17.0 + k as f64 * 18.0) as f32), size, ink, Align::Left);
        }
        text.draw(pm, "✕", ((r.x + r.w - 12.0) as f32, (r.y + 17.0) as f32), 12.0, ink, Align::Center);
    }
    // 📋 (tableaux-1) the hovered / selected table's outline and handles, the touch hint pill, the table's menu
    tableaux::dessiner_dessus(pm, &text);
}

/// 🫧 The information banner over a full-screen page (⌂ Accueil: created, renamed, ★, duplicated, deleted,
/// refused): a glass pill centred at the bottom, its bottom at `bas` (window). Its image and place (window).
pub fn bandeau_verre(text: &Text, b: &str, ww: f64, bas: f64) -> Option<(Pixmap, shapes::Rect)> {
    let line = actions::sans_touches(b);
    let w = (text.width(&line, 13.0) as f64 + 36.0).min((ww - 32.0).max(80.0));
    let line = text.fit(&line, 13.0, w - 36.0);
    let r = shapes::Rect::new(((ww - w) / 2.0).round(), (bas - 36.0 - 16.0).round(), w, 36.0);
    let mut pm = px::new(w.ceil() as u32 + 1, 37)?;
    let q = shapes::Rect::new(0.0, 0.0, w, 36.0);
    shapes::round_fill(&mut pm, q, 18.0, jetons::sur_fond(jetons::VERRE), 1.0);
    shapes::round_stroke(&mut pm, q, 18.0, jetons::sur_fond(jetons::LISERE), 1.0);
    text.draw(&mut pm, &line, (18.0, 23.0), 13.0, jetons::TEXTE, Align::Left);
    Some((pm, r))
}

/// Place du bandeau d'information de la chart (en haut au centre ; coordonnées de la chart).
pub fn banner_rect(vp: &Viewport, text: &Text, b: &str) -> shapes::Rect {
    banner_lignes(vp, text, b).0
}

/// The information banner: its place (chart coordinates), its lines and their size. PC: one line, cut with « … »
/// when too long. 📱 Cell / finger (F1, 22 Nov: the ⇕ notice sat on « O … H … L … C … » and its reason was cut):
/// as wide as the screen less 32 points at most, on two lines when needed, never cut. Never on the legend's lines
/// (title, O H L C, Vol, « ⌄ N », its unfolded list) nor on an indicator's table: under them.
pub fn banner_lignes(vp: &Viewport, text: &Text, b: &str) -> (shapes::Rect, Vec<String>, f32) {
    let s = actions::sans_touches(b);
    let etroit = actions::tactile() || vp.plot_w() < 560.0;
    // en haut au centre, sous la bulle DÉMO : il ne couvre plus le sélecteur d'indicateurs ni la légende
    // (🧘 Zen : sous la légende — titre, O H L C, Vol — qui commence alors en haut de la fenêtre)
    let bas = BAS_BANDEAU.with(|c| c.get());
    let (mut r, mut lignes, mut size) = if etroit {
        let room = (vp.width as f64 - 32.0).max(80.0);
        let w = (text.width(&s, 13.0) as f64 + 44.0).min(room);
        // (two lines at most: a smaller size before a third line, and never « … »)
        let mut size = 13.0;
        let mut lignes = crate::assistant::wrap(text, &s, size, w - 34.0);
        if lignes.len() > 2 {
            size = 12.0;
            lignes = crate::assistant::wrap(text, &s, size, w - 34.0);
        }
        let h = 8.0 + 18.0 * lignes.len() as f64;
        (shapes::Rect::new(((vp.width as f64 - w) / 2.0).max(16.0), 40.0 + bas, w, h), lignes, size)
    } else {
        let room = (vp.plot_w() - 24.0).max(80.0);
        let w = (text.width(&s, 13.0) as f64 + 44.0).min(room);
        let line = text.fit(&s, 13.0, w - 34.0);
        (shapes::Rect::new(((vp.plot_w() - w) / 2.0).max(8.0), 40.0 + bas, w, 26.0), vec![line], 13.0)
    };
    // 📋 never over an indicator's table (the backtest's summary lay over Argos TVD Pro, Suivi 10): under it
    let croise = |t: &shapes::Rect, r: &shapes::Rect| t.x < r.x + r.w && r.x < t.x + t.w && t.y < r.y + r.h && r.y < t.y + t.h;
    let legende = LEGENDE_BANDEAU.with(|l| l.borrow().clone());
    let sous_la_legende = |r: &mut shapes::Rect| {
        for _ in 0..8 {
            match legende.iter().chain(&indicators::tableaux()).find(|t| croise(t, r)) {
                Some(t) => r.y = t.y + t.h + 6.0,
                None => break,
            }
        }
    };
    sous_la_legende(&mut r);
    // 🧱 F1n (cell-cadrer-apres-v1: the ⇕ notice cut the top wall pill): never on an Argos Murs pill — from the left
    // gutter to the pill column's left edge less 8 points, on two lines (smaller before a third), never cut
    let pilules: Vec<shapes::Rect> = murs::pilules_dessinees().into_iter().map(|(p, _)| p).collect();
    if pilules.iter().any(|p| croise(p, &r)) {
        let gauche = if etroit { 16.0 } else { 8.0 };
        let colonne = pilules.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
        let w = (colonne - 8.0 - gauche).max(80.0);
        size = 13.0;
        lignes = crate::assistant::wrap(text, &s, size, w - 34.0);
        for petit in [12.0, 11.0] {
            if lignes.len() <= 2 {
                break;
            }
            size = petit;
            lignes = crate::assistant::wrap(text, &s, size, w - 34.0);
        }
        r = shapes::Rect::new(gauche, r.y, w, 8.0 + 18.0 * lignes.len() as f64);
        sous_la_legende(&mut r);
    }
    (r, lignes, size)
}

/// Centre de la `j`-ième place à venir après la dernière bougie (une place par unité de temps).
fn future_x(vp: &Viewport, j: usize) -> f64 {
    vp.bars_w() - (vp.right_offset - j as f64 + 0.5) * vp.bar_spacing
}

/// Price as written on the scale: percent or indexed on 100 (from `base`, the close of the first visible bar), or
/// the price with `dec` decimals.
pub(crate) fn scale_text(mode: ScaleMode, base: f64, p: f64, dec: usize) -> String {
    match mode {
        ScaleMode::Percent => {
            let l = (p / base - 1.0) * 100.0;
            format!("{}{} %", if l < -0.0049 { "−" } else { "" }, format_price(l.abs(), 2))
        }
        ScaleMode::Indexed => format_price(p / base * 100.0, 2),
        _ => format_price(p, dec),
    }
}

/// 🎯 The mouse layer of a chart: liquidation tooltip, economic release bubble, crosshair (dotted lines, price and
/// time on the scales) and the O / H / L / C · Vol legend (the bar under the mouse, else the last one). Owned, so
/// it can outlive the [`Chart`] and be drawn again on a copy of the image from [`render_fond`] when only the mouse
/// moves; [`render`] draws it in place.
#[derive(Debug, Clone, PartialEq)]
pub struct Reticule {
    vp: Viewport,
    slots: Slots,
    scale: PriceScale,
    band_axes: Vec<indicators::BandAxis>,
    /// Bars as shown (Heikin Ashi, Renko…).
    bars: Vec<Bar>,
    /// Real bars when they differ from the shown ones (Heikin Ashi): volume buyers / sellers.
    src: Option<Vec<Bar>>,
    renko: bool,
    tf_seconds: i64,
    tz_offset_s: i64,
    exact: usize,
    dec: usize,
    mode: ScaleMode,
    base: f64,
    theme: Theme,
    legende: bool,
    liq: Option<liqview::LiqView>,
    econ: Vec<econ::Spot>,
    econ_bottom: f64,
    /// 🧱 Argos Murs: its depth bars and pills drawn, for the tooltip of the one under the mouse.
    murs: Option<murs::Survol>,
    /// 🏦 The multi-exchange footprint: the detail of the row under the mouse (📱 the finger).
    fp: Option<FpSurvol>,
}

/// 🏦 The multi-exchange footprint drawn, for the bubble of the row under the mouse.
#[derive(Debug, Clone, PartialEq)]
struct FpSurvol {
    m: std::sync::Arc<fpmulti::FpMulti>,
    f: argos_script::indicators::flow::Footprint,
    /// Bottom of the price pane.
    bas: f64,
}

impl Reticule {
    /// 🏦 The bubble of the footprint row under the mouse (`None`: no multi-exchange footprint there, or an empty row).
    pub fn fp_bulle(&self, mx: f64, my: f64) -> Option<fpmulti::Bulle> {
        let s = self.fp.as_ref()?;
        if my > s.bas || s.f.step <= 0.0 {
            return None;
        }
        let i = self.vp.index_at(mx, &self.slots)?;
        let b = s.f.bars.iter().find(|b| b.i == i)?;
        let j = (self.scale.price_of(my) / s.f.step).floor() as i64;
        b.rows.iter().find(|r| r.0 == j)?;
        let t = self.bars.get(i)?.time;
        let hm = (t + self.tz_offset_s).rem_euclid(86_400);
        let heure = format!("{} · {:02}:{:02}", fpmulti::unite_temps(self.tf_seconds), hm / 3600, hm % 3600 / 60);
        let dec = self.dec;
        fpmulti::Bulle::de(&s.m, t, s.f.step, j, &|p| format_price(p, dec), heure)
    }

    /// 🏦 The footprint cell under the mouse (its row, the width of the footprint's cells).
    fn fp_case(&self, mx: f64, my: f64) -> Option<shapes::Rect> {
        let s = self.fp.as_ref()?;
        let i = self.vp.index_at(mx, &self.slots)?;
        let x = |k: usize| self.vp.x_of(k, &self.slots);
        let sp = match (i.checked_sub(1), i + 1 < self.bars.len()) {
            (Some(p), true) => (x(i + 1) - x(p)) / 2.0,
            (Some(p), false) => x(i) - x(p),
            (None, _) => x(i + 1) - x(i),
        };
        let cw = (sp * 0.9).max(2.0);
        let j = (self.scale.price_of(my) / s.f.step).floor();
        let (y0, y1) = (self.scale.y_of((j + 1.0) * s.f.step), self.scale.y_of(j * s.f.step));
        Some(shapes::Rect::new(x(i) - cw / 2.0, y0.min(y1), cw, (y1 - y0).abs()))
    }
}

impl Reticule {
    /// 🧱 The lines of Argos Murs' tooltip for the mouse (or a tap) at `(x, y)` (chart points); empty: nothing there.
    pub fn murs_infobulle(&self, x: f64, y: f64) -> Vec<String> {
        let pw = self.vp.plot_w();
        if let Some((_, t)) = self.murs.as_ref().and_then(|s| s.pression.as_ref()).filter(|(r, _)| r.contains(x, y)) {
            return vec![t.clone()];
        }
        self.murs
            .as_ref()
            .and_then(|s| murs::vise(s, x, y, pw).map(|v| murs::infobulle(s, v)))
            .unwrap_or_default()
    }

    /// Draws the layer for the mouse at `mouse` (chart coordinates; `None` or outside the bars: no crosshair).
    pub fn draw(&self, pm: &mut Pixmap, text: &Text, mouse: Option<(f64, f64)>) {
        let (vp, slots, scale, th) = (&self.vp, &self.slots, &self.scale, &self.theme);
        let (pw, ph) = (vp.plot_w(), vp.plot_h());
        let bars = &self.bars[..];
        let src = self.src.as_deref().unwrap_or(bars);
        let n = bars.len();
        let price_text = |p: f64, dec: usize| scale_text(self.mode, self.base, p, dec);
        let inside = mouse.filter(|(x, y)| *x >= 0.0 && *x <= pw && *y >= 0.0 && *y <= ph);
        // 💥 liquidation band under the mouse
        if let (Some(v), Some((mx, my))) = (&self.liq, inside) {
            let t = vp.index_at(mx, slots).and_then(|i| bars.get(i)).map_or(i64::MAX, |b| b.time);
            if let Some(b) = v.at(scale.price_of(my), t) {
                liqview::tooltip(pm, text, b, mx, my, pw);
            }
        }
        // 🧱 Argos Murs: the pill under the mouse is kept as drawn (no crosshair line through it), its pixels taken
        // now and put back after the crosshair; the tooltip comes last (end of this function)
        let pilule = self
            .murs
            .as_ref()
            .zip(inside)
            .and_then(|(s, (mx, my))| murs::pilule_sous(s, mx, my).map(|z| z.r));
        let copie = pilule.and_then(|r| murs::copie_zone(pm, r));
        // 📅 bubble of the economic release under the mouse
        if !self.econ.is_empty() {
            econ::bubble(pm, text, &self.econ, pw, self.econ_bottom, mouse, self.tz_offset_s);
        }
        // réticule : lignes pointillées, prix et heure sur les échelles
        if let Some((mx, my)) = inside {
            let cross = Rgb(0x9a, 0xa0, 0xad);
            let i = vp.index_at(mx, slots);
            // dans le futur : la place à venir la plus proche (sauf Renko)
            let ahead = (i.is_none() && n > 0 && mx > vp.x_of(n - 1, slots) && !self.renko)
                .then(|| ((mx - vp.x_of(n - 1, slots)) / vp.bar_spacing).round().max(1.0) as usize);
            let cx = match (i, ahead) {
                (Some(i), _) => vp.x_of(i, slots),
                (None, Some(j)) => future_x(vp, j),
                _ => mx,
            }
            .round();
            let mut k = 0.0;
            while k < ph {
                fill(pm, cx, k, 1.0, 3.0, cross, 0.8);
                k += 6.0;
            }
            k = 0.0;
            while k < pw {
                fill(pm, k, my.round(), 3.0, 1.0, cross, 0.8);
                k += 6.0;
            }
            let label = Rgb(0x36, 0x3a, 0x45);
            fill(pm, pw, my.round() - 10.0, vp.price_axis as f64, 20.0, label, 1.0);
            // dans une bande d'oscillateur : la valeur de la bande, pas le prix
            let value = match self.band_axes.iter().find(|a| a.contains(my)) {
                Some(a) => viewport::format_number(a.value_at(my), 2),
                None => price_text(scale.price_of(my), self.exact.max(self.dec)),
            };
            text.draw(
                pm,
                &value,
                ((pw + 8.0) as f32, (my.round() + 4.0) as f32),
                12.0,
                Rgb(255, 255, 255),
                Align::Left,
            );
            let hovered_time = match (i, ahead) {
                (Some(i), _) => Some(bars[i].time),
                (None, Some(j)) => Some(bars[n - 1].time + j as i64 * self.tf_seconds),
                _ => None,
            };
            if let Some(t) = hovered_time {
                let t = t + self.tz_offset_s;
                let when = crosshair_label(t, self.tf_seconds);
                let w = text.width(&when, 12.0) as f64 + 16.0;
                fill(pm, cx - w / 2.0, ph + 1.0, w, vp.time_axis as f64 - 1.0, label, 1.0);
                text.draw(pm, &when, (cx as f32, (ph + 18.0) as f32), 12.0, Rgb(255, 255, 255), Align::Center);
            }
        }
        // légende O H L C · Vol · Acheteurs / Vendeurs : la bougie sous la souris, sinon la dernière (toujours là,
        // comme la maquette et TradingView)
        // (⛶ a panel full screen: nothing of the prices)
        // (📱 by the finger on a small chart, the watchlist open on the phone: not over the candles; Jee, 21 Nov, list 2)
        let petite = actions::tactile() && ph < LEGENDE_MIN_H;
        let sous = inside
            .and_then(|(mx, _)| vp.index_at(mx, slots))
            .filter(|_| indicators::plein_ecran().is_none() && !petite);
        if let Some(i) = sous.or(n.checked_sub(1).filter(|_| self.legende && !petite)) {
            let exact = self.exact;
            let b = &bars[i];
            let up = b.close >= b.open;
            let col = if up { th.bull } else { th.bear };
            // volume total + acheteurs / vendeurs (estimés selon la clôture dans le range de la bougie ; vraie bougie, pas HA)
            let (info, [vol, buy_s, sell_s]) = legende_textes(b, &src[i], exact);
            text.draw(pm, &info, (12.0, (46.0 + haut_legende()) as f32), 12.5, col, Align::Left);
            let y = (64.0 + haut_legende()) as f32;
            text.draw(pm, &vol, (12.0, y), 12.5, th.text, Align::Left);
            let x = 12.0 + text.width(&vol, 12.5) + 14.0;
            text.draw(pm, &buy_s, (x, y), 12.5, th.bull, Align::Left);
            let x = x + text.width(&buy_s, 12.5) + 10.0;
            text.draw(pm, &sell_s, (x, y), 12.5, th.bear, Align::Left);
        }
        if let (Some(r), Some(c)) = (pilule, copie.as_ref()) {
            murs::remet_zone(pm, r, c);
        }
        // (👆 a tapped wall pill keeps its tooltip open, the finger lifted, until the next tap)
        // (F1j: by the finger only a real tap opens it — never where a drag ended)
        let survol = if actions::tactile() { None } else { inside };
        if let (Some(s), Some((mx, my))) = (&self.murs, survol.or(murs::bulle())) {
            murs::infobulle_draw(pm, text, s, mx, my, pw, ph);
        }
        // 🏦 the multi-exchange footprint: the detail of the row under the mouse, exchange by exchange (the cell
        // outlined in white, as the mock-up)
        if let Some((mx, my)) = inside {
            if let Some(b) = self.fp_bulle(mx, my) {
                if let Some(c) = self.fp_case(mx, my) {
                    shapes::round_stroke(pm, c, 2.0, Rgb(255, 255, 255), 1.5);
                }
                let bas = self.fp.as_ref().map_or(ph, |s| s.bas);
                b.draw(pm, text, b.place(mx, my, pw, bas));
            }
        }
        // ≫ F1j: « Revenir au direct » over the button (mouse)
        if let Some(r) = bouton_direct().filter(|r| survol.is_some_and(|(x, y)| r.contains(x, y))) {
            let t = "Revenir au direct";
            let w = text.width(t, 12.0) as f64 + 20.0;
            let b = shapes::Rect::new((r.cx() - w / 2.0).clamp(4.0, (pw - w - 4.0).max(4.0)), r.y - 8.0 - 26.0, w, 26.0);
            shapes::round_fill(pm, b, 8.0, palette::popup(), 0.95);
            shapes::round_stroke(pm, b, 8.0, palette::edge(), 1.0);
            text.draw(pm, t, (b.cx() as f32, (b.y + 17.0) as f32), 12.0, palette::WHITE, Align::Center);
        }
    }
}

thread_local! {
    /// 🫧 The right-click menu and « Aller à » drawn on their own layer (glass), not in the chart image.
    static MENUS_A_PART: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// 🫧 `true`: [`render`] leaves out the right-click menu and « Aller à »; [`menus_calque`] draws them apart.
pub fn set_menus_a_part(on: bool) {
    MENUS_A_PART.with(|c| c.set(on));
}

fn menus_a_part() -> bool {
    MENUS_A_PART.with(|c| c.get())
}

/// 🫧 The right-click menu (and its sub-menu) and « Aller à » of `c`, on a layer of their own (same drawing, same
/// places, chart coordinates) to be laid in glass over the chart: the image and its place, `None` when none is open.
pub fn menus_calque(c: &Chart) -> Option<(Pixmap, shapes::Rect)> {
    if c.goto.is_none() && !c.context.is_open() {
        return None;
    }
    let vp = c.viewport;
    let th = c.theme;
    let mut pm = px::new(vp.width.max(1), vp.height.max(1))?;
    if let Some(g) = &c.goto {
        goto::draw(&mut pm, &text::Text::default(), &vp, g);
    }
    let ctx = context_menu::Ctx {
        settings: &c.settings,
        drawings: c.drawings.len(),
        now: c.now_s,
        tpo: c.style == CandleStyle::Tpo,
    };
    context_menu::draw(&mut pm, &vp, &ctx, &c.context, &th);
    // the smallest rectangle around what was drawn (screen pixels → points)
    let (w, h) = (pm.width() as usize, pm.height() as usize);
    let px = pm.pixels();
    let (mut x0, mut y0, mut x1, mut y1) = (w, h, 0, 0);
    for j in 0..h {
        for i in 0..w {
            if px[j * w + i].alpha() > 0 {
                (x0, y0, x1, y1) = (x0.min(i), y0.min(j), x1.max(i + 1), y1.max(j + 1));
            }
        }
    }
    let part = (x1 > x0 && y1 > y0)
        .then(|| tiny_skia::IntRect::from_xywh(x0 as i32, y0 as i32, (x1 - x0) as u32, (y1 - y0) as u32))
        .flatten()?;
    let s = px::scale() as f64;
    Some((
        pm.clone_rect(part)?,
        shapes::Rect::new(x0 as f64 / s, y0 as f64 / s, (x1 - x0) as f64 / s, (y1 - y0) as f64 / s),
    ))
}

#[cfg(test)]
mod tests;
