//! 🎨 Icônes Phosphor (MIT, `assets/phosphor/LICENSE`) : le style unique, au trait fin, des icônes de l'interface
//! (barre d'icônes, menus, panneaux, modules). Dessinées avec resvg à la couleur du thème, puis gardées en mémoire.
//! Un émoji posé comme icône d'interface (« 📅 ») est remplacé par son icône Phosphor (`from_emoji`) ; les émojis de
//! contenu (sélecteur d'émojis, drapeaux, notes) restent des émojis (`icons::draw_emoji`).

use crate::shapes::{lines, round_fill, Rect};
use crate::text::Rgb;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use tiny_skia::Pixmap;

/// nom Phosphor → dessin SVG (256 × 256, style « regular », sauf `star-fill`).
const SVG: [(&str, &str); 88] = [
    ("alarm", include_str!("../assets/phosphor/alarm.svg")),
    ("arrow-counter-clockwise", include_str!("../assets/phosphor/arrow-counter-clockwise.svg")),
    ("arrows-clockwise", include_str!("../assets/phosphor/arrows-clockwise.svg")),
    ("arrows-down-up", include_str!("../assets/phosphor/arrows-down-up.svg")),
    ("bell", include_str!("../assets/phosphor/bell.svg")),
    ("book-bookmark", include_str!("../assets/phosphor/book-bookmark.svg")),
    ("book-open", include_str!("../assets/phosphor/book-open.svg")),
    ("bookmark-simple", include_str!("../assets/phosphor/bookmark-simple.svg")),
    ("brain", include_str!("../assets/phosphor/brain.svg")),
    ("briefcase", include_str!("../assets/phosphor/briefcase.svg")),
    ("calendar", include_str!("../assets/phosphor/calendar.svg")),
    ("caret-down", include_str!("../assets/phosphor/caret-down.svg")),
    ("caret-up", include_str!("../assets/phosphor/caret-up.svg")),
    ("chart-bar", include_str!("../assets/phosphor/chart-bar.svg")),
    ("chart-bar-horizontal", include_str!("../assets/phosphor/chart-bar-horizontal.svg")),
    ("chart-line", include_str!("../assets/phosphor/chart-line.svg")),
    ("chart-line-up", include_str!("../assets/phosphor/chart-line-up.svg")),
    ("chart-pie-slice", include_str!("../assets/phosphor/chart-pie-slice.svg")),
    ("chat-circle", include_str!("../assets/phosphor/chat-circle.svg")),
    ("clock", include_str!("../assets/phosphor/clock.svg")),
    ("coin", include_str!("../assets/phosphor/coin.svg")),
    ("coins", include_str!("../assets/phosphor/coins.svg")),
    ("compass", include_str!("../assets/phosphor/compass.svg")),
    ("copy", include_str!("../assets/phosphor/copy.svg")),
    ("download-simple", include_str!("../assets/phosphor/download-simple.svg")),
    ("export", include_str!("../assets/phosphor/export.svg")),
    ("eye", include_str!("../assets/phosphor/eye.svg")),
    ("eye-slash", include_str!("../assets/phosphor/eye-slash.svg")),
    ("fingerprint", include_str!("../assets/phosphor/fingerprint.svg")),
    ("fire", include_str!("../assets/phosphor/fire.svg")),
    ("fish", include_str!("../assets/phosphor/fish.svg")),
    ("flask", include_str!("../assets/phosphor/flask.svg")),
    ("floppy-disk", include_str!("../assets/phosphor/floppy-disk.svg")),
    ("folder-open", include_str!("../assets/phosphor/folder-open.svg")),
    ("footprints", include_str!("../assets/phosphor/footprints.svg")),
    ("game-controller", include_str!("../assets/phosphor/game-controller.svg")),
    ("gauge", include_str!("../assets/phosphor/gauge.svg")),
    ("gear", include_str!("../assets/phosphor/gear.svg")),
    ("globe", include_str!("../assets/phosphor/globe.svg")),
    ("graduation-cap", include_str!("../assets/phosphor/graduation-cap.svg")),
    ("grid-four", include_str!("../assets/phosphor/grid-four.svg")),
    ("hammer", include_str!("../assets/phosphor/hammer.svg")),
    ("handshake", include_str!("../assets/phosphor/handshake.svg")),
    ("key", include_str!("../assets/phosphor/key.svg")),
    ("layout", include_str!("../assets/phosphor/layout.svg")),
    ("lightning", include_str!("../assets/phosphor/lightning.svg")),
    ("lock-key", include_str!("../assets/phosphor/lock-key.svg")),
    ("lock-simple", include_str!("../assets/phosphor/lock-simple.svg")),
    ("magnet", include_str!("../assets/phosphor/magnet.svg")),
    ("magnifying-glass", include_str!("../assets/phosphor/magnifying-glass.svg")),
    ("map-pin", include_str!("../assets/phosphor/map-pin.svg")),
    ("megaphone", include_str!("../assets/phosphor/megaphone.svg")),
    ("minus", include_str!("../assets/phosphor/minus.svg")),
    ("newspaper", include_str!("../assets/phosphor/newspaper.svg")),
    ("notebook", include_str!("../assets/phosphor/notebook.svg")),
    ("palette", include_str!("../assets/phosphor/palette.svg")),
    ("pencil-simple", include_str!("../assets/phosphor/pencil-simple.svg")),
    ("percent", include_str!("../assets/phosphor/percent.svg")),
    ("plugs", include_str!("../assets/phosphor/plugs.svg")),
    ("plugs-connected", include_str!("../assets/phosphor/plugs-connected.svg")),
    ("plus", include_str!("../assets/phosphor/plus.svg")),
    ("plus-square", include_str!("../assets/phosphor/plus-square.svg")),
    ("pulse", include_str!("../assets/phosphor/pulse.svg")),
    ("puzzle-piece", include_str!("../assets/phosphor/puzzle-piece.svg")),
    ("rewind", include_str!("../assets/phosphor/rewind.svg")),
    ("ruler", include_str!("../assets/phosphor/ruler.svg")),
    ("scissors", include_str!("../assets/phosphor/scissors.svg")),
    ("seal-check", include_str!("../assets/phosphor/seal-check.svg")),
    ("shield-check", include_str!("../assets/phosphor/shield-check.svg")),
    ("shopping-cart", include_str!("../assets/phosphor/shopping-cart.svg")),
    ("sliders-horizontal", include_str!("../assets/phosphor/sliders-horizontal.svg")),
    ("sparkle", include_str!("../assets/phosphor/sparkle.svg")),
    ("squares-four", include_str!("../assets/phosphor/squares-four.svg")),
    ("stack", include_str!("../assets/phosphor/stack.svg")),
    ("star", include_str!("../assets/phosphor/star.svg")),
    ("star-fill", include_str!("../assets/phosphor/star-fill.svg")),
    ("trash", include_str!("../assets/phosphor/trash.svg")),
    ("tree-evergreen", include_str!("../assets/phosphor/tree-evergreen.svg")),
    ("tree-structure", include_str!("../assets/phosphor/tree-structure.svg")),
    ("triangle", include_str!("../assets/phosphor/triangle.svg")),
    ("trophy", include_str!("../assets/phosphor/trophy.svg")),
    ("user", include_str!("../assets/phosphor/user.svg")),
    ("user-circle", include_str!("../assets/phosphor/user-circle.svg")),
    ("users-three", include_str!("../assets/phosphor/users-three.svg")),
    ("video-camera", include_str!("../assets/phosphor/video-camera.svg")),
    ("wallet", include_str!("../assets/phosphor/wallet.svg")),
    ("waves", include_str!("../assets/phosphor/waves.svg")),
    ("wrench", include_str!("../assets/phosphor/wrench.svg")),
];

/// Émoji ou signe d'interface (« 📅 », « ↺ ») → icône Phosphor.
const EMOJI: [(&str, &str); 76] = [
    ("↺", "arrow-counter-clockwise"),
    ("⌃", "caret-up"),
    ("⌄", "caret-down"),
    ("◷", "clock"),
    ("✚", "plus"),
    ("⭐", "star-fill"),
    ("🐋", "fish"),
    ("📅", "calendar"),
    ("📈", "chart-line-up"),
    ("📉", "chart-line"),
    ("📰", "newspaper"),
    ("🧩", "puzzle-piece"),
    ("🔐", "lock-key"),
    ("🔄", "arrows-clockwise"),
    ("⬇️", "download-simple"),
    ("🟠", "layout"),
    ("🔷", "layout"),
    ("🥇", "layout"),
    ("🔑", "key"),
    ("📖", "book-open"),
    ("⚡", "lightning"),
    ("💼", "briefcase"),
    ("🪙", "coin"),
    ("😱", "gauge"),
    ("📤", "export"),
    ("✏️", "pencil-simple"),
    ("🔒", "lock-simple"),
    ("🧲", "magnet"),
    ("⚙️", "gear"),
    ("⚙", "gear"),
    ("➕", "plus"),
    ("➖", "minus"),
    ("💾", "floppy-disk"),
    ("🗂", "squares-four"),
    ("🛠", "wrench"),
    ("🔥", "fire"),
    ("⏰", "alarm"),
    ("✂️", "scissors"),
    ("📍", "map-pin"),
    ("💸", "percent"),
    ("👤", "user"),
    ("🌍", "globe"),
    ("🌐", "globe"),
    ("💥", "lightning"),
    ("↕️", "arrows-down-up"),
    ("👁", "eye"),
    ("🙈", "eye-slash"),
    ("🗑", "trash"),
    ("📄", "copy"),
    ("🔎", "magnifying-glass"),
    ("↩️", "arrow-counter-clockwise"),
    ("📓", "notebook"),
    ("📺", "video-camera"),
    ("💬", "chat-circle"),
    ("🆕", "plus-square"),
    ("📂", "folder-open"),
    ("⏪", "rewind"),
    ("📊", "chart-bar"),
    ("👣", "footprints"),
    ("🔺", "triangle"),
    ("🪟", "grid-four"),
    ("🧪", "flask"),
    ("🕯", "chart-line"),
    ("🌳", "tree-structure"),
    ("🔌", "plugs"),
    ("⏱", "clock"),
    ("🎨", "palette"),
    ("📥", "download-simple"),
    ("🎮", "game-controller"),
    ("✨", "sparkle"),
    ("🌊", "waves"),
    ("📐", "ruler"),
    ("📏", "ruler"),
    ("🧠", "brain"),
    ("👥", "users-three"),
    ("🛒", "shopping-cart"),
];

/// L'icône Phosphor qui remplace cet émoji d'interface.
pub fn from_emoji(e: &str) -> Option<&'static str> {
    EMOJI.iter().find(|(k, _)| *k == e).map(|(_, v)| *v)
}

/// Le contenu d'un SVG Phosphor (sans la balise `<svg>`).
fn body(name: &str) -> Option<&'static str> {
    let s = SVG.iter().find(|(k, _)| *k == name)?.1;
    let start = s.find('>')? + 1;
    let end = s.rfind("</svg>")?;
    Some(&s[start..end])
}

type Key = (&'static str, u32, (u8, u8, u8));

fn cache() -> &'static Mutex<HashMap<Key, Pixmap>> {
    static C: OnceLock<Mutex<HashMap<Key, Pixmap>>> = OnceLock::new();
    C.get_or_init(Default::default)
}

/// `name` est-il une icône Phosphor ?
pub fn has(name: &str) -> bool {
    SVG.iter().any(|(k, _)| *k == name)
}

/// Image de l'icône `name` (`size` pixels de côté), de couleur `c`.
pub fn image(name: &str, size: u32, c: Rgb) -> Option<Pixmap> {
    let name = SVG.iter().find(|(k, _)| *k == name)?.0;
    let key = (name, size, (c.0, c.1, c.2));
    if let Some(p) = cache().lock().ok()?.get(&key) {
        return Some(p.clone());
    }
    let hex = format!("#{:02x}{:02x}{:02x}", c.0, c.1, c.2);
    let doc = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256" width="{size}" height="{size}" fill="{hex}">{}</svg>"##,
        body(name)?.replace("currentColor", &hex)
    );
    let tree = resvg::usvg::Tree::from_str(&doc, &resvg::usvg::Options::default()).ok()?;
    let mut pm = Pixmap::new(size, size)?;
    resvg::render(&tree, tiny_skia::Transform::identity(), &mut pm.as_mut());
    if let Ok(mut c) = cache().lock() {
        c.insert(key, pm.clone());
    }
    Some(pm)
}

/// Pose l'icône `name` centrée sur `(cx, cy)` ; `false` si ce n'est pas une icône Phosphor.
pub fn draw(pm: &mut Pixmap, name: &str, cx: f64, cy: f64, size: u32, c: Rgb) -> bool {
    // gris neutre → couleur de la catégorie de l'icône (voir `palette::icon_tint`)
    // rendue directement à la taille de l'écran (nette sur iPhone), posée centrée en points
    let Some(img) = image(name, crate::px::dev(size as f64).round() as u32, crate::palette::icon_tint(name, c)) else {
        return false;
    };
    let half = size as f64 / 2.0;
    crate::px::put(pm, cx - half, cy - half, &img);
    true
}

/// ✓ dans une pastille (ce qui est déjà ouvert), centrée sur `(cx, cy)`.
pub fn check(pm: &mut Pixmap, cx: f64, cy: f64, d: f64) {
    let c = crate::palette::accent();
    round_fill(pm, Rect::new(cx - d / 2.0, cy - d / 2.0, d, d), d / 2.0, c, 1.0);
    let k = (d / 18.0) as f32;
    lines(
        pm,
        cx,
        cy,
        &[&[(-4.0 * k, 0.0), (-1.2 * k, 3.0 * k), (4.2 * k, -3.0 * k)]],
        crate::palette::ink_on(c),
        1.8 * k,
    );
}

/// Petite étiquette « bientôt » alignée à droite sur `right`, centrée sur `cy`.
pub fn soon(pm: &mut Pixmap, text: &crate::text::Text, right: f64, cy: f64) {
    let w = text.width("bientôt", 9.5) as f64 + 12.0;
    round_fill(pm, Rect::new(right - w, cy - 8.0, w, 16.0), 8.0, crate::palette::label(), 1.0);
    text.draw(
        pm,
        "bientôt",
        ((right - w / 2.0) as f32, (cy + 3.5) as f32),
        9.5,
        crate::palette::GREY,
        crate::text::Align::Center,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icones_phosphor() {
        for (name, _) in SVG {
            let p = image(name, 24, Rgb(255, 255, 255)).expect(name);
            assert!(p.pixels().iter().filter(|px| px.alpha() > 128).count() > 10, "{name} : dessin visible");
        }
        for (e, name) in EMOJI {
            assert!(body(name).is_some(), "{e} → {name}");
        }
        assert!(body("inconnue").is_none() && from_emoji("🦄").is_none());
    }
}
