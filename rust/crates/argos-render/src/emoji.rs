//! 😀 Émojis en couleur (images Twemoji, licence CC-BY 4.0, incluses dans Argos : rien à installer).
//! Servent à l'outil « Icônes et émojis » (comme TradingView) et au texte : `Text` dessine un émoji trouvé dans une
//! phrase (étiquettes, tableaux, inputs) comme une image. Seuls les émojis écrits dans le code sont inclus
//! (`argos-outils emoji` les copie et refait la table).

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use tiny_skia::{Pixmap, Transform};

/// Les émojis offerts, par section (titre, émojis) : trading d'abord, comme la liste de TradingView.
pub const SECTIONS: &[(&str, &[&str])] = &[
    (
        "Trading",
        &["🚀", "📈", "📉", "💰", "💵", "💎", "🔥", "🎯", "🐂", "🐻", "🐋", "🌕", "⚡", "⏰", "🔔", "📌"],
    ),
    (
        "Signaux",
        &["✅", "❌", "⚠️", "⛔", "🟢", "🔴", "🟡", "🔵", "⬆️", "⬇️", "➡️", "⬅️", "⭐", "💡", "❓", "❗"],
    ),
    (
        "Visages",
        &["😀", "😎", "🤑", "😱", "😭", "🤔", "😴", "🤯", "😤", "🥳", "🙏", "👍", "👎", "👀", "💪", "🧠"],
    ),
    (
        "Divers",
        &["🏁", "🚩", "🎉", "💣", "🧨", "🛑", "🔒", "🔓", "📅", "📰", "🏦", "🪙", "🧲", "🌊", "☀️", "🌙"],
    ),
];

/// L'émoji `e` tel qu'offert (sélection d'Argos ou tout Twemoji), en `&'static str` ; `None` s'il n'y est pas.
pub fn statique(e: &str) -> Option<&'static str> {
    SECTIONS
        .iter()
        .flat_map(|(_, l)| l.iter().copied())
        .chain(crate::emoji_tout::TOUT.iter().map(|(_, _, t, _)| *t))
        .find(|x| *x == e)
}

/// Émoji par défaut d'une nouvelle icône.
pub const DEFAULT: &str = "🚀";

/// Nom Twemoji d'une suite de caractères : points de code en hexa, sans FE0F hors des séquences ZWJ.
fn key(e: &str) -> String {
    let zwj = e.contains('\u{200D}');
    e.chars()
        .filter(|c| zwj || *c != '\u{FE0F}')
        .map(|c| format!("{:x}", c as u32))
        .collect::<Vec<_>>()
        .join("-")
}

fn svg(e: &str) -> Option<&'static [u8]> {
    let k = key(e);
    crate::twemoji::SVG
        .binary_search_by(|(n, _)| (*n).cmp(k.as_str()))
        .ok()
        .map(|i| crate::twemoji::SVG[i].1)
        .or_else(|| charges().lock().ok()?.get(&k).copied())
}

// ---------- 😀 tout Twemoji, chargé à la demande (`/emoji/<nom>.svg`, voir `emoji_tout`) ----------

/// Images arrivées du serveur : nom Twemoji → SVG (gardées pour toute la session).
fn charges() -> &'static Mutex<HashMap<String, &'static [u8]>> {
    static C: OnceLock<Mutex<HashMap<String, &'static [u8]>>> = OnceLock::new();
    C.get_or_init(Default::default)
}

/// Noms à aller chercher (vidés par l'app à chaque image) et noms déjà demandés (une seule fois chacun).
fn demandes() -> &'static Mutex<(Vec<String>, std::collections::HashSet<String>)> {
    static D: OnceLock<Mutex<(Vec<String>, std::collections::HashSet<String>)>> = OnceLock::new();
    D.get_or_init(Default::default)
}

/// Les noms Twemoji de tout le jeu (index `emoji_tout`).
fn connus() -> &'static std::collections::HashSet<&'static str> {
    static K: OnceLock<std::collections::HashSet<&'static str>> = OnceLock::new();
    K.get_or_init(|| crate::emoji_tout::TOUT.iter().map(|(_, k, _, _)| *k).collect())
}

/// `e` fait-il partie de Twemoji (inclus dans l'app ou servi à côté) ?
pub fn connu(e: &str) -> bool {
    let k = key(e);
    svg(e).is_some() || connus().contains(k.as_str()) || crate::emoji_tout::VARIANTES.contains(&k.as_str())
}

/// Les images à aller chercher depuis la dernière fois (l'app les charge, puis appelle [`charger`]).
pub fn a_charger() -> Vec<String> {
    demandes().lock().map(|mut d| std::mem::take(&mut d.0)).unwrap_or_default()
}

/// How many emoji images arrived so far: an image kept with another number is drawn again.
pub fn arrivees() -> usize {
    charges().lock().map_or(0, |c| c.len())
}

/// Une image est arrivée : elle sert tout de suite (la prochaine image la dessine).
pub fn charger(nom: &str, svg: Vec<u8>) {
    if let Ok(mut c) = charges().lock() {
        c.insert(nom.to_string(), Box::leak(svg.into_boxed_slice()));
    }
    if let Ok(mut c) = cache().lock() {
        c.retain(|(e, _), p| p.is_some() || key(e) != nom);
    }
}

/// Image pas encore là : demandée une fois (si elle existe dans le jeu).
fn demander(e: &str) {
    let k = key(e);
    if !(connus().contains(k.as_str()) || crate::emoji_tout::VARIANTES.contains(&k.as_str())) {
        return;
    }
    if let Ok(mut d) = demandes().lock() {
        if d.1.insert(k.clone()) {
            d.0.push(k);
        }
    }
}

/// Caractères du BMP montrés en émoji même sans sélecteur FE0F (présentation émoji par défaut d'Unicode).
const BMP_EMOJI: &str = "⌚⌛⏩⏪⏫⏬⏰⏳◽◾☔☕♿⚓⚡⚪⚫⚽⚾⛄⛅⛎⛔⛪⛲⛳⛵⛺⛽✅✊✋✨❌❎❓❔❕❗➕➖➗➰➿⬛⬜⭐⭕";

/// Longueur (en octets) de l'émoji au début de `s`, s'il est dans la table ; sinon `None` (texte normal).
/// Une flèche ou un symbole simple (« → », « ▲ », « ⚠ ») reste du texte ; « ⚠️ » (avec FE0F) devient un émoji.
pub fn at_start(s: &str) -> Option<usize> {
    let mut it = s.char_indices().peekable();
    let (_, first) = it.next()?;
    let mut end = first.len_utf8();
    let mut fe0f = false;
    while let Some(&(i, c)) = it.peek() {
        // (the tags U+E0020…E007F spell a region after 🏴: England, Scotland, Wales)
        let joins = c == '\u{FE0F}' || c == '\u{20E3}' || ('\u{1F3FB}'..='\u{1F3FF}').contains(&c) || ('\u{E0020}'..='\u{E007F}').contains(&c);
        if joins {
            fe0f |= c == '\u{FE0F}';
            end = i + c.len_utf8();
            it.next();
        } else if c == '\u{200D}' {
            it.next();
            match it.next() {
                Some((j, d)) => end = j + d.len_utf8(),
                None => break,
            }
        } else {
            break;
        }
    }
    let bmp = (first as u32) < 0x1F000;
    if bmp && !fe0f && !BMP_EMOJI.contains(first) && end == first.len_utf8() {
        return None;
    }
    connu(&s[..end]).then_some(end)
}

/// Images déjà faites : (émoji, taille) → image (`None` : pas dans la table).
type Cache = Mutex<HashMap<(String, u32), Option<Pixmap>>>;

fn cache() -> &'static Cache {
    static C: OnceLock<Cache> = OnceLock::new();
    C.get_or_init(Default::default)
}

/// L'émoji `e` en image carrée de `size` pixels.
pub fn image(e: &str, size: u32) -> Option<Pixmap> {
    let k = (e.to_string(), size);
    if let Some(p) = cache().lock().ok()?.get(&k) {
        return p.clone();
    }
    if svg(e).is_none() {
        demander(e); // (pas encore arrivée : rien de gardé, elle sera dessinée à son arrivée)
        return None;
    }
    let out = render(e, size.max(1));
    if let Ok(mut c) = cache().lock() {
        c.insert(k, out.clone());
    }
    out
}

fn render(e: &str, size: u32) -> Option<Pixmap> {
    let tree = resvg::usvg::Tree::from_data(svg(e)?, &resvg::usvg::Options::default()).ok()?;
    let mut pm = Pixmap::new(size, size)?;
    let k = size as f32 / tree.size().width().max(tree.size().height());
    resvg::render(&tree, Transform::from_scale(k, k), &mut pm.as_mut());
    Some(pm)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emoji_en_couleur() {
        let pm = image("🚀", 32).expect("fusée");
        assert_eq!((pm.width(), pm.height()), (32, 32));
        assert!(pm.pixels().iter().any(|p| p.alpha() > 200), "des pixels visibles");
        assert!(image("⚠️", 24).is_some(), "le sélecteur de variante est ignoré");
        assert!(
            SECTIONS.iter().flat_map(|(_, l)| l.iter()).all(|e| image(e, 20).is_some()),
            "chaque émoji offert existe"
        );
    }

    #[test]
    fn emoji_dans_le_texte() {
        assert_eq!(at_start("🚀 GOLD BUY"), Some(4));
        assert_eq!(at_start("⚠️ attention"), Some(6), "⚠ + FE0F");
        assert_eq!(at_start("✅ ok"), Some(3), "présentation émoji par défaut");
        assert_eq!(
            (at_start("→ suite"), at_start("▲"), at_start("⚠ seul"), at_start("abc")),
            (None, None, None, None)
        );
        assert_eq!(at_start("🕯️ Volume"), Some(7));
        // 🏴 + its tags is one flag (it was 🏴 then six boxes)
        let angleterre = "\u{1F3F4}\u{E0067}\u{E0062}\u{E0065}\u{E006E}\u{E0067}\u{E007F}";
        assert_eq!(at_start(&format!("{angleterre} UK")), Some(angleterre.len()), "the England flag");
    }
}
