//! **Table d'actions unique** de la chart. Chaque action est écrite une seule fois : son nom, son raccourci et ce
//! qu'elle fait. Le clavier, la barre de navigation, les menus (clic droit, ⚙, ➕) et la liste des raccourcis lisent
//! tous cette table : le raccourci affiché ne peut jamais être différent de la vraie touche.

use crate::settings::{ChartSettings, ScaleMode};
use crate::viewport::{PriceScale, Slots, Viewport};

/// Réglage tout-ou-rien (✓) des menus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flag {
    LastPriceLabel,
    LastPriceLine,
    Grid,
    SymbolName,
    PrevCloseLabel,
    PrevCloseLine,
    HighLowLabel,
    HighLowLine,
    /// 📍 Prix du plus haut et du plus bas sur la chart.
    HighLowChart,
    Countdown,
    PlusButton,
    SessionBreaks,
    /// 📅 Annonces économiques au bas de la chart.
    EconEvents,
    /// 📰 Nouvelles du marché dans la même rangée.
    EconNews,
    /// 🛡️ Fenêtres PropFirm ±2 min autour des annonces à fort impact.
    PropFirmWindows,
    MainOnly,
    /// TPO : lettres, blocs.
    TpoLetters,
    TpoBlocks,
    /// 📖 Bid & Ask : étiquettes, lignes.
    BidAskLabel,
    BidAskLine,
}

/// Toutes les actions de la chart.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ChartAction {
    // affichage
    ResetView,
    ResetTime,
    ZoomIn,
    ZoomOut,
    ScrollLeft,
    ScrollRight,
    ScrollLeftFast,
    ScrollRightFast,
    GoStart,
    GoEnd,
    /// Afficher les bougies `de` à `à` sur toute la largeur (périodes 1D, 5D…, plage de « Aller à »).
    ShowRange(usize, usize),
    /// Bougie `i` au milieu de la chart (« Aller à » une date).
    CenterOn(usize),
    // échelle de prix
    ResetPrice,
    AutoScale,
    LockRatio,
    Invert,
    Mode(ScaleMode),
    TogglePercent,
    ToggleLog,
    Toggle(Flag),
    SetTimezone(&'static str),
    /// TPO : durée d'une lettre (s), zone de valeur (%), initial balance (min).
    TpoBlock(i64),
    TpoVa(u8),
    TpoIb(i64),
    // fenêtre (faites par la fenêtre desktop)
    /// Ouvrir « Aller à » (taper une date).
    GoTo,
    /// Symbole précédent / suivant de la watchlist (↑ / ↓).
    WatchPrev,
    WatchNext,
    Reload,
    NextStyle,
    PrevStyle,
    Undo,
    /// ↷ Refaire ce qui vient d'être défait.
    Redo,
    CloseMenus,
    HLineAtMouse,
    HLine(f64),
    /// 🔔 Alerte de prix à ce prix.
    AddAlert(f64),
    /// 💼 Préparer un ordre démo à ce prix sur la chart (`true` : achat) : un brouillon avec son bouton Buy / Sell.
    Brouillon(f64, bool),
    CopyPrice(f64),
    ClearDrawings,
    /// Copier / coller le dessin sélectionné.
    CopyDrawing,
    PasteDrawing,
    /// 🌳 Arborescence des objets.
    ObjectTree,
    /// ⛶ The chart or the panel under the right-click full screen (or back from it).
    PleinEcran,
}

/// Touche (indépendante de la librairie de fenêtre).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    /// Caractère, en minuscule.
    Char(char),
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    F5,
    Escape,
}

/// Touches de modification enfoncées.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Mods {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
}

const N: Mods = Mods {
    ctrl: false,
    alt: false,
    shift: false,
};
const C: Mods = Mods { ctrl: true, ..N };
const A: Mods = Mods { alt: true, ..N };
const S: Mods = Mods { shift: true, ..N };
const CA: Mods = Mods { ctrl: true, alt: true, ..N };
const CS: Mods = Mods { ctrl: true, shift: true, ..N };

/// Les raccourcis : (touche, modificateurs, action). Le **premier** raccourci d'une action est celui affiché.
pub const BINDINGS: &[(Key, Mods, ChartAction)] = &[
    (Key::F5, N, ChartAction::Reload),
    (Key::Char('g'), A, ChartAction::GoTo),
    (Key::Char('r'), C, ChartAction::Reload),
    (Key::Char('r'), A, ChartAction::ResetView),
    (Key::Char('q'), CA, ChartAction::ResetTime),
    (Key::Up, C, ChartAction::ZoomIn),
    (Key::Up, N, ChartAction::WatchPrev),
    (Key::Char('+'), N, ChartAction::ZoomIn),
    (Key::Char('='), N, ChartAction::ZoomIn),
    (Key::Down, C, ChartAction::ZoomOut),
    (Key::Down, N, ChartAction::WatchNext),
    (Key::Char('-'), N, ChartAction::ZoomOut),
    (Key::Left, N, ChartAction::ScrollLeft),
    (Key::Right, N, ChartAction::ScrollRight),
    (Key::Left, C, ChartAction::ScrollLeftFast),
    (Key::Right, C, ChartAction::ScrollRightFast),
    (Key::Home, N, ChartAction::GoStart),
    (Key::End, N, ChartAction::GoEnd),
    (Key::Char('i'), A, ChartAction::Invert),
    (Key::Char('p'), A, ChartAction::TogglePercent),
    (Key::Char('l'), A, ChartAction::ToggleLog),
    (Key::Char('h'), A, ChartAction::HLineAtMouse),
    (Key::Char('z'), C, ChartAction::Undo),
    (Key::Char('z'), CS, ChartAction::Redo),
    (Key::Char('y'), C, ChartAction::Redo),
    (Key::Char('c'), C, ChartAction::CopyDrawing),
    (Key::Char('v'), C, ChartAction::PasteDrawing),
    (Key::Char('t'), N, ChartAction::NextStyle),
    (Key::Char('t'), S, ChartAction::PrevStyle),
    (Key::Escape, N, ChartAction::CloseMenus),
];

/// Action d'une touche. Maj est ignorée pour les caractères qui ne sont pas des lettres (« + » demande Maj sur
/// certains claviers).
pub fn from_key(k: Key, m: Mods) -> Option<ChartAction> {
    let m = match k {
        Key::Char(c) if !c.is_alphabetic() => Mods { shift: false, ..m },
        _ => m,
    };
    BINDINGS.iter().find(|(bk, bm, _)| *bk == k && *bm == m).map(|(_, _, a)| *a)
}

fn key_text(k: Key) -> String {
    match k {
        Key::Char(c) => c.to_uppercase().to_string(),
        Key::Left => "←".into(),
        Key::Right => "→".into(),
        Key::Up => "↑".into(),
        Key::Down => "↓".into(),
        Key::Home => "Début".into(),
        Key::End => "Fin".into(),
        Key::F5 => "F5".into(),
        Key::Escape => "Échap".into(),
    }
}

/// Texte d'un raccourci : « Ctrl + Alt + Q ».
pub fn binding_text(k: Key, m: Mods) -> String {
    let mut parts = vec![];
    if m.ctrl {
        parts.push("Ctrl".to_string());
    }
    if m.alt {
        parts.push("Alt".to_string());
    }
    if m.shift {
        parts.push("Maj".to_string());
    }
    parts.push(key_text(k));
    parts.join(" + ")
}

thread_local! {
    /// 📱 Phone view (phone, or tablet used by touch): no keyboard, so no shortcut is written and the PC-only rows
    /// (zoom, full screen, screen test, keyboard shortcuts) are left out. Set by the app with its layout.
    static TACTILE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// 📱 Phone view on or off (the app calls it each time it lays out the screen).
pub fn set_tactile(on: bool) {
    TACTILE.with(|t| t.set(on));
}

/// 📱 Phone view (phone, or tablet used by touch)?
pub fn tactile() -> bool {
    TACTILE.with(|t| t.get())
}

/// ⌨ Keyboard shortcuts are shown (PC); `false` in phone view (phone, or tablet used by touch): there is no keyboard,
/// so no shortcut text is drawn anywhere (Jee, 20 Nov: « les raccourcis sur tablette cell on enlève tout »).
pub fn montrer_raccourcis() -> bool {
    !tactile()
}

/// Key names as written in the app (French, English, Spanish).
const TOUCHES: [&str; 19] = [
    "Ctrl", "Alt", "⌘", "Cmd", "Maj", "Shift", "Mayús", "Échap", "Esc", "Entrée", "Enter", "Intro", "Suppr", "Del", "Supr", "Tab", "F11", "F5", "Ctrl+K",
];

/// A word of `s` is a key name?
fn nomme_une_touche(s: &str) -> bool {
    s.split(|c: char| !(c.is_alphanumeric() || c == '⌘' || c == '+'))
        .any(|m| TOUCHES.contains(&m) || m.starts_with("Ctrl+") || m.starts_with("Alt+"))
}

/// A key shortcut (« Ctrl + S », « Alt + H », « Suppr », « Échap ») written as a menu hint or in a tooltip?
pub fn est_raccourci(s: &str) -> bool {
    let s = s.trim();
    // only keys: key names, « + », « / », one-sign keys (K, ↑, /), and « glisser »-like words (« Ctrl + glisser »)
    let touche = |m: &str| {
        TOUCHES.contains(&m)
            || m.starts_with("Ctrl+")
            || m.starts_with("Alt+")
            || m.chars().count() == 1
            || ["glisser", "Drag", "drag", "arrastrar", "ou", "or", "o"].contains(&m)
    };
    nomme_une_touche(s) && s.split_whitespace().all(touche)
}

/// 📱 A sentence without its keyboard parts in phone view (the PC keeps it whole): a part in brackets that names a
/// key goes (« Plein écran (F11 ou Échap pour revenir) » → « Plein écran »), so does a « · » part that names one
/// (« PNG ou JPEG · ou Ctrl+V pour coller » → « PNG ou JPEG »); a sentence that is only keys becomes empty.
pub fn sans_touches(s: &str) -> String {
    if montrer_raccourcis() {
        return s.to_string();
    }
    // (in the user's language first: the keys are named there too)
    let s = &*crate::i18n::tr(s);
    // brackets that name a key
    let mut out = String::new();
    let mut reste = s;
    while let Some(a) = reste.find('(') {
        let Some(b) = reste[a..].find(')').map(|b| a + b) else { break };
        if nomme_une_touche(&reste[a + 1..b]) {
            out.push_str(reste[..a].trim_end());
        } else {
            out.push_str(&reste[..=b]);
        }
        reste = &reste[b + 1..];
    }
    out.push_str(reste);
    // a « — » part that names a key: « Chargement impossible : … — F5 pour réessayer », the banner of a failed
    // loading (first, and only where there is a « — »: the « · » parts are then read in what is left)
    let out = if out.contains(" — ") {
        out.split(" — ").filter(|p| !nomme_une_touche(p)).collect::<Vec<_>>().join(" — ")
    } else {
        out
    };
    // « · » parts that name a key
    let parts: Vec<&str> = out.split(" · ").filter(|p| !nomme_une_touche(p)).collect();
    let r = parts.join(" · ");
    let r = r.trim();
    if est_raccourci(r) {
        return String::new();
    }
    au_doigt(r.trim_end_matches([':', ',']).trim_end())
}

/// 📱 Mouse words said for the finger, longest first (the long press does the right click, two taps the double click).
const AU_DOIGT: [(&str, &str); 9] = [
    ("il suit ta souris, clique sur la chart pour le poser", "touche la chart pour le poser"),
    ("il suit ta souris : clique sur la chart pour le poser", "touche la chart pour le poser"),
    ("elle revient au survol", "touche ‹ pour la rouvrir"),
    ("double-clic", "touche deux fois"),
    ("clic droit", "appui long"),
    ("Clique sur ", "Touche "),
    ("clique sur ", "touche "),
    ("Clique ", "Touche "),
    ("clique ", "touche "),
];

/// 📱 A sentence written for the mouse, said for the finger (Jee, 21 Nov, list 2: no « clic droit », « survol »,
/// « souris » or « double-clic » in phone view).
fn au_doigt(s: &str) -> String {
    AU_DOIGT.iter().fold(s.to_string(), |s, (souris, doigt)| s.replace(souris, doigt))
}

/// 📱 A hint without its keyboard shortcut in phone view: « Ctrl + S » → « », « tes cartes (Alt+H) » → « tes cartes ».
pub fn sans_raccourci(hint: &str) -> String {
    if !tactile() {
        return hint.to_string();
    }
    if est_raccourci(hint) {
        return String::new();
    }
    if let Some(k) = hint.rfind(" (") {
        if hint.ends_with(')') && est_raccourci(&hint[k + 2..hint.len() - 1]) {
            return hint[..k].to_string();
        }
    }
    hint.to_string()
}

/// 📱 The shortcut shown next to a name (tooltip, menu row): none in phone view (the name stays).
pub fn shortcut_bulle(a: ChartAction) -> Option<String> {
    if tactile() {
        return None;
    }
    shortcut(a)
}

/// Raccourci affiché d'une action (le premier de la table). Les modes Pourcent / Log montrent celui de leur bascule.
pub fn shortcut(a: ChartAction) -> Option<String> {
    // (🖥 ⛶ full screen: three clicks on the chart or the panel, no key)
    if a == ChartAction::PleinEcran {
        return Some("3 clics".into());
    }
    let a = match a {
        ChartAction::Mode(ScaleMode::Percent) => ChartAction::TogglePercent,
        ChartAction::Mode(ScaleMode::Log) => ChartAction::ToggleLog,
        ChartAction::ResetPrice => ChartAction::ResetView,
        x => x,
    };
    BINDINGS.iter().find(|(_, _, b)| *b == a).map(|(k, m, _)| binding_text(*k, *m))
}

/// Nom d'une action (bulles de la barre de navigation, liste des raccourcis).
pub fn label(a: ChartAction) -> &'static str {
    match a {
        ChartAction::ResetView => "Réinitialiser l'affichage du graphique",
        ChartAction::ResetTime => "Réinitialiser l'échelle de temps",
        ChartAction::ZoomIn => "Grossissement",
        ChartAction::ZoomOut => "Réduction",
        ChartAction::ScrollLeft => "Faire défiler vers la gauche",
        ChartAction::ScrollRight => "Faire défiler vers la droite",
        ChartAction::ScrollLeftFast => "Défiler de 10 bougies vers la gauche",
        ChartAction::ScrollRightFast => "Défiler de 10 bougies vers la droite",
        ChartAction::GoStart => "Aller au début de l'historique",
        ChartAction::GoEnd => "Revenir au présent",
        ChartAction::ShowRange(..) => "Afficher la période",
        ChartAction::CenterOn(_) => "Aller à la bougie",
        ChartAction::GoTo => "Aller à…",
        ChartAction::WatchPrev => "Symbole précédent de la watchlist",
        ChartAction::WatchNext => "Symbole suivant de la watchlist",
        ChartAction::ResetPrice => "Réinitialiser l'échelle de prix",
        ChartAction::AutoScale => "Auto (adapte les données à l'écran)",
        ChartAction::LockRatio => "Verrouiller le rapport prix / barre",
        ChartAction::Invert => "Inverser l'échelle",
        ChartAction::Mode(ScaleMode::Normal) => "Normal",
        ChartAction::Mode(ScaleMode::Percent) | ChartAction::TogglePercent => "Pourcent",
        ChartAction::Mode(ScaleMode::Indexed) => "Indexé sur 100",
        ChartAction::Mode(ScaleMode::Log) | ChartAction::ToggleLog => "Logarithmique",
        ChartAction::Toggle(_) | ChartAction::SetTimezone(_) => "Réglage",
        ChartAction::TpoBlock(_) | ChartAction::TpoVa(_) | ChartAction::TpoIb(_) => "Réglage TPO",
        ChartAction::Reload => "Recharger les bougies",
        ChartAction::NextStyle => "Type de chart suivant",
        ChartAction::PrevStyle => "Type de chart précédent",
        ChartAction::Undo => "Défaire (dessins)",
        ChartAction::Redo => "Refaire (dessins)",
        ChartAction::CloseMenus => "Fermer les menus",
        ChartAction::HLineAtMouse | ChartAction::HLine(_) => "Tracer une ligne horizontale",
        ChartAction::AddAlert(_) => "Ajouter une alerte de prix",
        ChartAction::Brouillon(_, true) => "Préparer un achat démo",
        ChartAction::Brouillon(_, false) => "Préparer une vente démo",
        ChartAction::CopyPrice(_) => "Copier le prix",
        ChartAction::ClearDrawings => "Effacer tous les dessins",
        ChartAction::CopyDrawing => "Copier le dessin",
        ChartAction::PasteDrawing => "Coller le dessin",
        ChartAction::ObjectTree => "Arborescence des objets",
        ChartAction::PleinEcran => "Plein écran",
    }
}

/// Zoom (facteur d'espace entre bougies) d'un pas de zoom.
/// Fort exprès (× 2) : quelques clics passent d'une vue de près à une vue générale.
const ZOOM_STEP: f64 = 2.0;

/// Applique les actions d'affichage (zoom, défilement, réinitialiser). `true` si l'action en était une.
pub fn apply_view(a: ChartAction, vp: &mut Viewport, slots: &Slots) -> bool {
    let mid = vp.plot_w() / 2.0;
    let sp = vp.bar_spacing;
    match a {
        ChartAction::ResetView | ChartAction::ResetTime => {
            let d = Viewport::default();
            (vp.bar_spacing, vp.right_offset) = (d.bar_spacing, d.right_offset);
        }
        ChartAction::ZoomIn => vp.zoom_at(ZOOM_STEP, mid, slots),
        ChartAction::ZoomOut => vp.zoom_at(1.0 / ZOOM_STEP, mid, slots),
        ChartAction::ScrollLeft => vp.scroll_px(sp, slots),
        ChartAction::ScrollRight => vp.scroll_px(-sp, slots),
        ChartAction::ScrollLeftFast => vp.scroll_px(sp * 10.0, slots),
        ChartAction::ScrollRightFast => vp.scroll_px(-sp * 10.0, slots),
        ChartAction::GoStart => {
            // la première bougie collée au bord gauche
            vp.right_offset = vp.bars_w() / sp - slots.total();
            vp.scroll_px(0.0, slots);
        }
        ChartAction::GoEnd => vp.right_offset = Viewport::default().right_offset,
        ChartAction::ShowRange(i, j) => vp.show_range(i, j, slots),
        ChartAction::CenterOn(i) => vp.center_on(i, slots),
        _ => return false,
    }
    true
}

/// Applique les actions qui changent les réglages. `bar_spacing` sert au verrou prix / barre ; `current` = l'échelle
/// affichée (pour passer d'auto à manuel sans saut). `true` si l'action en était une.
pub fn apply_settings(a: ChartAction, st: &mut ChartSettings, bar_spacing: f64, current: Option<PriceScale>) -> bool {
    let o = &mut st.scale;
    let now = current.map(|s| (s.min, s.max));
    let set_mode = |o: &mut crate::settings::ScaleOptions, md: ScaleMode| {
        // passer en / sortir du logarithmique change l'espace des bornes : on revient en auto
        if (md == ScaleMode::Log) != (o.mode == ScaleMode::Log) {
            (o.manual, o.lock_ratio) = (None, None);
        }
        o.mode = md;
    };
    match a {
        ChartAction::ResetView | ChartAction::ResetPrice => (o.manual, o.lock_ratio) = (None, None),
        ChartAction::AutoScale => {
            if o.auto() {
                o.manual = now;
            } else {
                (o.manual, o.lock_ratio) = (None, None);
            }
        }
        ChartAction::LockRatio => {
            if o.lock_ratio.is_some() {
                o.lock_ratio = None;
                o.manual = now;
            } else if let Some(s) = current {
                o.lock_ratio = Some((s.max - s.min) / (s.height.max(1.0) * bar_spacing));
                o.manual = now;
            }
        }
        ChartAction::Invert => o.invert = !o.invert,
        ChartAction::Mode(md) => set_mode(o, md),
        ChartAction::TogglePercent => {
            let md = if o.mode == ScaleMode::Percent {
                ScaleMode::Normal
            } else {
                ScaleMode::Percent
            };
            set_mode(o, md)
        }
        ChartAction::ToggleLog => {
            let md = if o.mode == ScaleMode::Log { ScaleMode::Normal } else { ScaleMode::Log };
            set_mode(o, md)
        }
        ChartAction::Toggle(f) => {
            let b = flag_mut(st, f);
            *b = !*b;
        }
        ChartAction::SetTimezone(tz) => st.timezone = tz.to_string(),
        ChartAction::TpoBlock(s) => st.tpo.block = s,
        ChartAction::TpoVa(v) => st.tpo.va = v,
        ChartAction::TpoIb(m) => st.tpo.ib = m,
        _ => return false,
    }
    true
}

/// La case d'un réglage tout-ou-rien.
pub fn flag_mut(st: &mut ChartSettings, f: Flag) -> &mut bool {
    match f {
        Flag::LastPriceLabel => &mut st.scale.last_price_label,
        Flag::LastPriceLine => &mut st.scale.last_price_line,
        Flag::Grid => &mut st.scale.grid,
        Flag::MainOnly => &mut st.scale.main_only,
        Flag::SymbolName => &mut st.labels.symbol_name,
        Flag::PrevCloseLabel => &mut st.labels.prev_close,
        Flag::PrevCloseLine => &mut st.lines.prev_close,
        Flag::HighLowLabel => &mut st.labels.high_low,
        Flag::HighLowLine => &mut st.lines.high_low,
        Flag::HighLowChart => &mut st.labels.high_low_chart,
        Flag::Countdown => &mut st.labels.countdown,
        Flag::BidAskLabel => &mut st.labels.bid_ask,
        Flag::BidAskLine => &mut st.lines.bid_ask,
        Flag::PlusButton => &mut st.plus_button,
        Flag::SessionBreaks => &mut st.session_breaks,
        Flag::EconEvents => &mut st.econ_events,
        Flag::EconNews => &mut st.econ_news,
        Flag::PropFirmWindows => &mut st.propfirm_windows,
        Flag::TpoLetters => &mut st.tpo.letters,
        Flag::TpoBlocks => &mut st.tpo.blocks,
    }
}

/// Valeur d'un réglage tout-ou-rien (pour le ✓ des menus).
pub fn flag(st: &ChartSettings, f: Flag) -> bool {
    *flag_mut(&mut st.clone(), f)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn touches_et_raccourcis_affiches() {
        assert_eq!(from_key(Key::Char('r'), A), Some(ChartAction::ResetView));
        assert_eq!(from_key(Key::Char('q'), CA), Some(ChartAction::ResetTime));
        assert_eq!(from_key(Key::Up, C), Some(ChartAction::ZoomIn));
        assert_eq!(from_key(Key::Char('+'), S), Some(ChartAction::ZoomIn), "Maj ignorée pour « + »");
        assert_eq!(from_key(Key::Char('t'), S), Some(ChartAction::PrevStyle), "Maj comptée pour les lettres");
        assert_eq!(from_key(Key::Char('x'), N), None);
        // le texte affiché vient de la même table
        assert_eq!(shortcut(ChartAction::ZoomIn).as_deref(), Some("Ctrl + ↑"));
        assert_eq!(shortcut(ChartAction::ResetTime).as_deref(), Some("Ctrl + Alt + Q"));
        assert_eq!(shortcut(ChartAction::Mode(ScaleMode::Log)).as_deref(), Some("Alt + L"));
        assert_eq!(shortcut(ChartAction::ScrollLeft).as_deref(), Some("←"));
        assert_eq!(shortcut(ChartAction::Mode(ScaleMode::Indexed)), None);
        // chaque raccourci affiché redonne bien son action
        for (k, m, a) in BINDINGS {
            assert_eq!(from_key(*k, *m), Some(*a), "{}", binding_text(*k, *m));
        }
    }

    #[test]
    fn actions_affichage_et_reglages() {
        let mut vp = Viewport::default();
        let slots = Slots::uniform(1000);
        let sp = vp.bar_spacing;
        assert!(apply_view(ChartAction::ZoomIn, &mut vp, &slots));
        assert!((vp.bar_spacing - sp * ZOOM_STEP).abs() < 1e-9);
        assert!(apply_view(ChartAction::ResetTime, &mut vp, &slots));
        assert_eq!(vp.bar_spacing, sp);
        assert!(!apply_view(ChartAction::Invert, &mut vp, &slots), "pas une action d'affichage");

        let mut st = ChartSettings::default();
        let cur = Some(PriceScale {
            min: 100.0,
            max: 200.0,
            top: 0.0,
            height: 500.0,
            log: false,
            invert: false,
        });
        assert!(apply_settings(ChartAction::AutoScale, &mut st, 7.0, cur));
        assert_eq!(st.scale.manual, Some((100.0, 200.0)));
        assert!(apply_settings(ChartAction::ResetView, &mut st, 7.0, cur));
        assert!(st.scale.auto());
        apply_settings(ChartAction::TogglePercent, &mut st, 7.0, None);
        assert_eq!(st.scale.mode, ScaleMode::Percent);
        apply_settings(ChartAction::TogglePercent, &mut st, 7.0, None);
        assert_eq!(st.scale.mode, ScaleMode::Normal, "Alt + P deux fois = retour en normal");
        for f in [
            Flag::HighLowLabel,
            Flag::HighLowChart,
            Flag::PrevCloseLine,
            Flag::PlusButton,
            Flag::SessionBreaks,
            Flag::EconEvents,
            Flag::EconNews,
            Flag::PropFirmWindows,
            Flag::Countdown,
        ] {
            let before = flag(&st, f);
            apply_settings(ChartAction::Toggle(f), &mut st, 7.0, None);
            assert_ne!(flag(&st, f), before, "{f:?}");
        }
        apply_settings(ChartAction::SetTimezone("Asia/Tokyo"), &mut st, 7.0, None);
        assert_eq!(st.timezone, "Asia/Tokyo");
        assert!(!apply_settings(ChartAction::Reload, &mut st, 7.0, None), "action de la fenêtre");
    }
}

/// 📱 Jee (20 Nov): « les raccourcis sur tablette cell on enlève tout ». In phone view no keyboard shortcut is written
/// on the main screens; the PC keeps them.
#[cfg(test)]
mod vue_telephone {
    use super::*;
    use crate::shapes::Rect;
    use crate::text::{ecrits, Text};

    #[test]
    fn phrases_sans_touches() {
        set_tactile(false);
        assert_eq!(sans_touches("Défaire (Ctrl + Z)"), "Défaire (Ctrl + Z)", "PC: unchanged");
        set_tactile(true);
        assert_eq!(sans_touches("Défaire (Ctrl + Z)"), "Défaire");
        assert_eq!(sans_touches("🖥 Plein écran (F11 ou Échap pour revenir)"), "🖥 Plein écran");
        assert_eq!(sans_touches("PNG ou JPEG · ou Ctrl+V pour coller une image"), "PNG ou JPEG");
        // (the banner of a failed loading: its « — F5 pour réessayer » goes, the rest stays)
        assert_eq!(
            sans_touches("Mise à jour impossible (bougies de la dernière session) : HTTP 500 — F5 pour réessayer"),
            "Mise à jour impossible (bougies de la dernière session) : HTTP 500"
        );
        assert_eq!(
            sans_touches("Ctrl + R · direct reconnecté, bougies rechargées"),
            "direct reconnecté, bougies rechargées"
        );
        assert_eq!(
            sans_touches("3. Sur cette source : « Ajouter une clé », colle-la (Ctrl + V)."),
            "3. Sur cette source : « Ajouter une clé », colle-la."
        );
        assert_eq!(sans_touches("Tab = champ suivant · Entrée = enregistrer · Échap = annuler"), "");
        assert_eq!(
            sans_touches("🔎 Choisis le symbole de la case 2 (Échap : annuler)"),
            "🔎 Choisis le symbole de la case 2"
        );
        assert_eq!(sans_touches("Entrée ou clic · DexScreener, gratuit"), "DexScreener, gratuit");
        // what is not a key stays
        assert_eq!(sans_touches("Mises à jour (version et date)"), "Mises à jour (version et date)");
        assert_eq!(
            sans_touches("Entrée 63 000 · Stop 62 500"),
            "Stop 62 500",
            "(only used where « Entrée » is a key)"
        );
        for k in [
            "Ctrl + S",
            "Alt + H",
            "Suppr",
            "Ctrl + glisser",
            "Ctrl + Maj + Z ou Ctrl + Y",
            "Ctrl /",
            "Échap",
            "Ctrl + Drag",
        ] {
            assert!(est_raccourci(k), "{k}");
        }
        for k in ["", "taille normale", "3 clics", "BTC · 30m", "Entrée 63 000", "Supprimer"] {
            assert!(!est_raccourci(k), "{k}");
        }
        set_tactile(false);
    }

    /// 📱 Jee (21 Nov, list 2): the mouse words of the banners and tips are said for the finger; the PC keeps them.
    #[test]
    fn phrases_dites_au_doigt() {
        let souris = [
            (
                "Émoji prêt : il suit ta souris, clique sur la chart pour le poser",
                "Émoji prêt : touche la chart pour le poser",
            ),
            (
                "Choisis l'émoji, il suit ta souris : clique sur la chart pour le poser",
                "Choisis l'émoji, touche la chart pour le poser",
            ),
            (
                "Clique les points, double-clic (ou Entrée) pour terminer",
                "Touche les points, touche deux fois pour terminer",
            ),
            (
                "✋ Main libre : glisse la chart · double-clic ou ↺ RAZ pour le prix automatique",
                "✋ Main libre : glisse la chart · touche deux fois ou ↺ RAZ pour le prix automatique",
            ),
            (
                "🙈 « Règle » masqué : clic droit sur la barre pour le réafficher",
                "🙈 « Règle » masqué : appui long sur la barre pour le réafficher",
            ),
            (
                "‹ Barre d'outils repliée : elle revient au survol",
                "‹ Barre d'outils repliée : touche ‹ pour la rouvrir",
            ),
            ("📍 Clique sur la chart pour choisir le point", "📍 Touche la chart pour choisir le point"),
        ];
        set_tactile(false);
        for (pc, _) in souris {
            assert_eq!(sans_touches(pc), pc, "PC: unchanged");
        }
        set_tactile(true);
        for (pc, doigt) in souris {
            assert_eq!(sans_touches(pc), doigt);
        }
        // (a word that only looks like it stays)
        assert_eq!(sans_touches("Cliquer pour cliquer"), "Cliquer pour cliquer");
        set_tactile(false);
    }

    /// A drawn text that writes a keyboard shortcut.
    fn ecrit_une_touche(s: &str) -> bool {
        nomme_une_touche(s) || s.contains("Touches :") || s.contains("↑ ↓ choisir")
    }

    /// Texts written by the main screens (home, search, map, ARGOS ▾, ☰ Appli, the phone sheet, panels, settings,
    /// layouts menu).
    fn ecrans(w: f64, h: f64) -> Vec<String> {
        let t = Text::default();
        let _ = ecrits();
        let cartes = crate::accueil::cartes(&crate::accueil::Donnees::default());
        let _ = crate::accueil::AccueilPage::default().draw(&t, w as u32, h as u32, (0.0, 0.0), &cartes);
        let _ = crate::recherche::Palette::new(&[]).draw(&t, w, h, (0.0, 0.0));
        let _ = crate::plan::Plan::default().draw(&t, w, h, None);
        let a = Rect::new(10.0, 10.0, 100.0, 36.0);
        let compte = crate::topmenu::CompteCarte::default();
        let _ = crate::topmenu::ArgosMenu::new(a, w, h).draw(&t, &compte, (0.0, 0.0), &|_| false);
        let _ = crate::topmenu::ArgosMenu::app(a, w, h).draw(&t, &compte, (0.0, 0.0), &|_| false);
        let _ = crate::topmenu::ArgosMenu::feuille(w, h).draw(&t, &compte, (0.0, 0.0), &|_| false);
        let _ = crate::sheet::Sheet::new(crate::sheet::SheetKind::Panels, a, 50.0, w).draw(&t, "", (0.0, 0.0), &|_| false);
        let _ = crate::reglages::Reglages::new(w - 10.0, 50.0, h).draw(&t, None);
        let layouts = crate::topmenu::layouts_menu(&[("Argos".into(), "BTC · 30m".into(), true)], false);
        let _ = crate::listmenu::ListMenu::new(layouts, vec![], a, w, h).draw(&t, None);
        ecrits()
    }

    #[test]
    fn aucun_raccourci_dessine_en_vue_telephone() {
        // 🖥 PC: the shortcuts are there
        set_tactile(false);
        let pc = ecrans(1280.0, 900.0);
        for attendu in ["Ctrl", "Échap", "Ctrl + S"] {
            assert!(pc.iter().any(|s| s == attendu), "PC: « {attendu} » drawn");
        }
        assert!(pc.iter().any(|s| s.contains("Touches :")), "PC: the home's « Touches » line");
        // 📱 phone view (phone and touch tablet sizes): none
        set_tactile(true);
        for (w, h) in [(390.0, 844.0), (402.0, 874.0), (834.0, 1194.0)] {
            let tel = ecrans(w, h);
            assert!(!tel.is_empty());
            let touches: Vec<&String> = tel.iter().filter(|s| ecrit_une_touche(s)).collect();
            assert!(touches.is_empty(), "{w}×{h}: {touches:?}");
            // the home's key caps: the letter in each card's corner
            for s in crate::accueil::Section::ALL {
                let l = s.lettre().to_string();
                assert!(!tel.contains(&l) || pc.iter().filter(|x| **x == l).count() > 1, "{w}×{h}: letter {l}");
            }
        }
        set_tactile(false);
    }
}
