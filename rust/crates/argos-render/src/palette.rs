//! 🎨 Couleurs du thème d'origine d'Argos Rust (gris), partagées par toutes les barres, fenêtres et panneaux.
//! Toute nouvelle partie de l'interface prend ses couleurs ici.

use crate::text::Rgb;

/// Fond des barres et des fenêtres.
pub const PANEL: Rgb = Rgb(0x1b, 0x1d, 0x24);
/// Fond le plus sombre (champs, barre du bas).
pub const FIELD: Rgb = Rgb(0x13, 0x15, 0x1b);
/// Bordure.
pub const EDGE: Rgb = Rgb(0x34, 0x38, 0x44);
/// Bordure douce (cases).
pub const EDGE_SOFT: Rgb = Rgb(0x2e, 0x32, 0x3d);
/// Case au repos.
pub const TILE: Rgb = Rgb(0x22, 0x25, 0x2e);
/// Survol.
pub const HOVER: Rgb = Rgb(0x2a, 0x2e, 0x39);
/// Actif / sélectionné.
pub const ON: Rgb = Rgb(0x31, 0x36, 0x44);
/// Icônes et texte.
pub const ICON: Rgb = Rgb(0xd1, 0xd4, 0xdc);
pub const WHITE: Rgb = Rgb(0xe6, 0xe8, 0xee);
/// Texte secondaire.
pub const GREY: Rgb = Rgb(0x8a, 0x8e, 0x9a);
pub const DIM: Rgb = Rgb(0x6a, 0x6e, 0x7a);
/// Texte éteint (indications, champs vides), plus sombre que `DIM`.
pub const MUTED: Rgb = Rgb(0x5d, 0x60, 0x6b);
/// Texte courant des listes et des icônes au repos (gris clair).
pub const TEXT: Rgb = Rgb(0xb2, 0xb5, 0xbe);
/// Fond des cartes et des lignes choisies dans les panneaux.
pub const CARD: Rgb = Rgb(0x1a, 0x1d, 0x25);
/// Fond des bulles et petits menus flottants.
pub const POPUP: Rgb = Rgb(0x1e, 0x21, 0x2a);
/// Rail des curseurs, séparateurs.
pub const TRACK: Rgb = Rgb(0x3a, 0x3e, 0x4a);
/// Fond des étiquettes et des boutons « + ».
pub const LABEL: Rgb = Rgb(0x36, 0x3a, 0x45);
/// Survol des étiquettes et des boutons « + ».
pub const KNOB: Rgb = Rgb(0x50, 0x55, 0x62);
/// Poignée des barres de défilement.
pub const THUMB: Rgb = Rgb(0x4a, 0x4e, 0x5a);
/// Bouton clair (« D'accord », élément choisi sur fond clair).
pub const SNOW: Rgb = Rgb(0xf0, 0xf1, 0xf4);
/// Texte foncé posé sur un bouton clair.
pub const INK: Rgb = Rgb(0x13, 0x17, 0x22);
/// Fond presque noir (barre du bas, bouton ✨).
pub const NIGHT: Rgb = Rgb(0x0b, 0x0c, 0x10);
/// Fond des grandes pages (Marketplace, Portefeuille).
pub const BACKDROP: Rgb = Rgb(0x0b, 0x0e, 0x14);
/// Bordure sombre (barre du haut).
pub const EDGE_DARK: Rgb = Rgb(0x23, 0x26, 0x30);
/// Gris argent (texte secondaire du calendrier « Aller à »).
pub const SILVER: Rgb = Rgb(0x9a, 0xa0, 0xad);
/// Accent bleu Argos (thème 🔷 Argos).
pub const ACCENT: Rgb = Rgb(0x29, 0x62, 0xff);

/// 🎨 Teinte des fonds et bordures selon le thème : 🥇 Or les réchauffe, 🟢 Matrix les verdit. Défaut, Argos et
/// ⚫ Classique les gardent tels quels. On garde la luminosité du gris et on change seulement sa couleur.
pub fn tone(c: Rgb) -> Rgb {
    let k = match crate::theme_index() {
        3 => (1.10, 1.0, 0.80),
        4 => (0.80, 1.10, 0.85),
        _ => return c,
    };
    let l = 0.299 * c.0 as f64 + 0.587 * c.1 as f64 + 0.114 * c.2 as f64;
    let f = |m: f64| (l * m).round().clamp(0.0, 255.0) as u8;
    Rgb(f(k.0), f(k.1), f(k.2))
}

// Fonds et bordures qui suivent le thème (voir `tone`) : à utiliser dans l'interface à la place des constantes.
pub fn panel() -> Rgb {
    tone(PANEL)
}
pub fn field() -> Rgb {
    tone(FIELD)
}
pub fn edge() -> Rgb {
    tone(EDGE)
}
pub fn edge_soft() -> Rgb {
    tone(EDGE_SOFT)
}
pub fn tile() -> Rgb {
    tone(TILE)
}
pub fn hover() -> Rgb {
    tone(HOVER)
}
pub fn on() -> Rgb {
    tone(ON)
}
pub fn card() -> Rgb {
    tone(CARD)
}
pub fn popup() -> Rgb {
    tone(POPUP)
}
pub fn track() -> Rgb {
    tone(TRACK)
}
pub fn label() -> Rgb {
    tone(LABEL)
}
pub fn knob() -> Rgb {
    tone(KNOB)
}
pub fn thumb() -> Rgb {
    tone(THUMB)
}
pub fn night() -> Rgb {
    tone(NIGHT)
}
pub fn backdrop() -> Rgb {
    tone(BACKDROP)
}
pub fn edge_dark() -> Rgb {
    tone(EDGE_DARK)
}

/// Accent du thème choisi : choix, cases cochées, onglets (bleu seulement dans le thème 🔷 Argos ; neutre dans Défaut
/// et Or, vert dans Matrix).
pub fn accent() -> Rgb {
    crate::Theme::current().accent
}

/// Texte lisible sur le fond `c` : foncé sur une couleur pâle, blanc sinon.
pub fn ink_on(c: Rgb) -> Rgb {
    // le plus contrasté des deux (règle WCAG) ; l'encre foncée est un bleu nuit à peine teinté : le thème clair ne la
    // retourne pas (seuls les gris le sont), elle reste foncée sur les boutons colorés
    let white = Rgb(255, 255, 255);
    if contrast(INK_DARK, c) >= contrast(white, c) {
        INK_DARK
    } else {
        white
    }
}

/// Encre foncée des textes posés sur une couleur pâle (bleu nuit, assez teinté pour ne pas être pris pour un gris).
pub const INK_DARK: Rgb = Rgb(0x08, 0x0c, 0x38);

/// Luminance relative (WCAG) d'une couleur.
pub fn luminance(c: Rgb) -> f64 {
    let f = |v: u8| {
        let v = v as f64 / 255.0;
        if v <= 0.03928 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * f(c.0) + 0.7152 * f(c.1) + 0.0722 * f(c.2)
}

/// Rapport de contraste WCAG entre deux couleurs (1 à 21).
pub fn contrast(a: Rgb, b: Rgb) -> f64 {
    let (x, y) = (luminance(a) + 0.05, luminance(b) + 0.05);
    x.max(y) / x.min(y)
}

/// La même couleur, foncée juste assez pour un contraste d'au moins `min` sur `bg` (texte : 4,5 ; icône : 3).
pub fn readable_on(c: Rgb, bg: Rgb, min: f64) -> Rgb {
    let mut k = 1.0;
    let mut out = c;
    while contrast(out, bg) < min && k > 0.05 {
        k -= 0.04;
        out = Rgb((c.0 as f64 * k) as u8, (c.1 as f64 * k) as u8, (c.2 as f64 * k) as u8);
    }
    out
}

/// 🌞 Gris retourné du thème clair : le fond sombre devient clair, le texte clair devient foncé, et les gris moyens
/// (texte secondaire, icônes) sont foncés plus fort pour rester lisibles sur le clair.
pub fn light_gray(l: i32) -> i32 {
    if l < 40 {
        255 - l
    } else {
        255 - (40 + (l - 40) * 2).min(255)
    }
}

/// Couleur vive du thème choisi : bouton Connect, éléments allumés, logo texte (cyan seulement dans le thème Argos).
pub fn cyan() -> Rgb {
    crate::Theme::current().cyan
}
/// « bientôt », étoiles.
pub const GOLD: Rgb = Rgb(0xf0, 0xb9, 0x0b);
pub const GREEN: Rgb = Rgb(0x08, 0x99, 0x81);
pub const RED: Rgb = Rgb(0xf2, 0x36, 0x45);
/// 📈 Hausse et 📉 baisse du thème (gains / pertes, achat / vente, positions) : vert et rouge, ou les couleurs des
/// thèmes daltoniens (bleu / orange, turquoise / magenta).
pub fn up() -> Rgb {
    crate::Theme::current().bull
}
pub fn down() -> Rgb {
    crate::Theme::current().bear
}
/// 💰 Couleur d'un gain ou d'une perte : la hausse du thème si positif, la baisse si négatif, gris à zéro.
pub fn pnl(v: f64) -> Rgb {
    if v > 0.0 {
        up()
    } else if v < 0.0 {
        down()
    } else {
        GREY
    }
}
/// Avertissement (gardien PropFirm : prudence).
pub const ORANGE: Rgb = Rgb(0xff, 0xab, 0x00);

/// 📜 Couleurs de syntaxe de l'éditeur Argos Script : mots-clés, chaînes, nombres, commentaires, fonctions.
pub const CODE_KW: Rgb = Rgb(0xc6, 0x78, 0xdd);
pub const CODE_STR: Rgb = Rgb(0x98, 0xc3, 0x79);
pub const CODE_NUM: Rgb = Rgb(0xd1, 0x9a, 0x66);
pub const CODE_COM: Rgb = Rgb(0x6a, 0x73, 0x80);
pub const CODE_FN: Rgb = Rgb(0x61, 0xaf, 0xef);

/// 🎨 Catégorie d'une icône d'interface : chacune a sa couleur vive, comme les icônes d'une app.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconCat {
    /// Outils de dessin (lignes, Fibonacci, formes, texte).
    Draw,
    /// Analyse : indicateurs, types de chart, unités de temps.
    Analyse,
    /// Trading : ticket, positions, journal, portefeuille.
    Trading,
    /// Marchés : watchlist, marché, Marketplace, baleines, exchanges.
    Market,
    /// Infos : agenda, news, alertes.
    Info,
    /// Assistant et académie.
    Assistant,
    /// Réglages et outils de vue (aimant, cadenas, œil, gomme, poubelle).
    Settings,
}

/// Couleurs des catégories (dans l'ordre de `IconCat`) pour le thème `k` : vives sur fond sombre, plus foncées en
/// clair, réchauffées dans 🥇 Or, en nuances de vert dans 🟢 Matrix.
fn icon_table(k: usize) -> [Rgb; 7] {
    match k {
        1 => [
            Rgb(0x1d, 0x4e, 0xd8),
            Rgb(0x0e, 0x74, 0x90),
            Rgb(0x04, 0x78, 0x57),
            Rgb(0xb4, 0x53, 0x09),
            Rgb(0xbe, 0x12, 0x3c),
            Rgb(0x7e, 0x22, 0xce),
            Rgb(0x43, 0x38, 0xca),
        ],
        3 => [
            Rgb(0xff, 0xcf, 0x5a),
            Rgb(0xf0, 0xa9, 0x3b),
            Rgb(0xb7, 0xd3, 0x6b),
            Rgb(0xff, 0x9f, 0x43),
            Rgb(0xff, 0x7a, 0x6b),
            Rgb(0xf2, 0x9b, 0xd1),
            Rgb(0xe8, 0xd5, 0xa8),
        ],
        // daltoniens : couleurs d'Okabe et Ito (choisies pour rester distinctes quand on voit mal certaines couleurs)
        6 | 7 => [
            Rgb(0x56, 0xb4, 0xe9),
            Rgb(0xf0, 0xe4, 0x42),
            Rgb(0x00, 0xb8, 0x8a),
            Rgb(0xe6, 0x9f, 0x00),
            Rgb(0xcc, 0x79, 0xa7),
            Rgb(0xd5, 0x5e, 0x00),
            Rgb(0xa8, 0xf8, 0xc8),
        ],
        4 => [
            Rgb(0x3d, 0xff, 0x8c),
            Rgb(0x7d, 0xff, 0xb0),
            Rgb(0x00, 0xe6, 0x76),
            Rgb(0xc6, 0xff, 0x4d),
            Rgb(0x5e, 0xff, 0xd8),
            Rgb(0xa8, 0xff, 0x60),
            Rgb(0x8f, 0xd9, 0xa8),
        ],
        _ => [
            Rgb(0x5b, 0x9c, 0xff),
            Rgb(0x22, 0xd3, 0xee),
            Rgb(0x34, 0xd3, 0x99),
            Rgb(0xfb, 0xbf, 0x24),
            Rgb(0xfb, 0x71, 0x85),
            Rgb(0xc0, 0x84, 0xfc),
            Rgb(0xa5, 0xb4, 0xfc),
        ],
    }
}

/// Couleur de la catégorie `cat` dans le thème choisi.
pub fn icon_color(cat: IconCat) -> Rgb {
    icon_table(crate::theme_index())[cat as usize]
}

/// Même couleur, un peu plus claire (survol, icône active).
pub fn icon_hot(c: Rgb) -> Rgb {
    let f = |v: u8| (v as f64 + (255.0 - v as f64) * 0.35).round() as u8;
    Rgb(f(c.0), f(c.1), f(c.2))
}

/// Catégorie d'une icône d'après son nom (icône Phosphor ou dessin d'Argos v1). Les dessins d'Argos v1 qui ne sont pas
/// listés sont des outils de dessin ; les icônes Phosphor pas listées sont des réglages.
pub fn icon_cat(name: &str) -> IconCat {
    use IconCat::*;
    match name {
        "pulse"
        | "chart-line"
        | "chart-line-up"
        | "chart-bar"
        | "chart-bar-horizontal"
        | "candles"
        | "bars"
        | "area"
        | "baseline"
        | "columns"
        | "line"
        | "hollow"
        | "heikin"
        | "renko"
        | "fx"
        | "clock"
        | "gauge"
        | "waves"
        | "stack"
        | "flask"
        | "tree-structure"
        | "triangle"
        | "percent"
        | "magnifying-glass"
        | "tree-evergreen" => Analyse,
        "game-controller" | "notebook" | "briefcase" | "wallet" | "coin" | "coins" | "handshake" | "trophy" | "shield-check" | "lock-key" | "key"
        | "fingerprint" | "user" | "user-circle" | "users-three" => Trading,
        "bookmark-simple" | "book-bookmark" | "shopping-cart" | "chart-pie-slice" | "fish" | "plugs" | "plugs-connected" | "globe" | "fire" | "star"
        | "star-fill" | "lightning" | "compass" | "map-pin" => Market,
        "calendar" | "newspaper" | "bell" | "alarm" | "megaphone" | "chat-circle" | "comment" | "video-camera" => Info,
        "sparkle" | "brain" | "graduation-cap" | "book-open" | "seal-check" => Assistant,
        "gear"
        | "sliders-horizontal"
        | "wrench"
        | "hammer"
        | "magnet"
        | "lock-simple"
        | "eye"
        | "eye-slash"
        | "eye2"
        | "eyeoff2"
        | "erase"
        | "trash"
        | "undo"
        | "redo"
        | "arrow-counter-clockwise"
        | "arrows-clockwise"
        | "puzzle-piece"
        | "layout"
        | "squares-four"
        | "grid-four"
        | "palette"
        | "floppy-disk"
        | "folder-open"
        | "download-simple"
        | "export"
        | "copy"
        | "minus"
        | "plus"
        | "plus-square"
        | "caret-down"
        | "caret-up"
        | "arrows-down-up"
        | "rewind"
        | "scissors"
        | "close" => Settings,
        _ if crate::phosphor::has(name) => Settings,
        _ => Draw,
    }
}

/// 🎨 Teinte d'une icône `name` demandée en couleur `c` : le gris neutre d'interface prend la couleur de sa catégorie,
/// le blanc (survol, icône active) cette couleur plus claire ; le grisé (indisponible) et les couleurs choisies
/// (rouge, vert, accent…) restent comme elles sont.
pub fn icon_tint(name: &str, c: Rgb) -> Rgb {
    if c == ICON || c == GREY || c == TEXT || c == SILVER {
        icon_color(icon_cat(name))
    } else if c == WHITE || c == SNOW {
        icon_hot(icon_color(icon_cat(name)))
    } else {
        c
    }
}

/// 🌞 Thème clair des menus et panneaux : les gris sont retournés (fond sombre → clair, texte blanc → foncé, en gardant
/// leur petite teinte) ; les couleurs (accent, vert / rouge, émojis, logos) restent comme elles sont.
/// Couleur d'un indicateur (input « #rrggbb ») sur la chart : en thème daltonien, vert → hausse et rouge → baisse
/// ([`dalt_map`]) ; en thème clair, foncée jusqu'à 3:1 sur le fond blanc
/// (une ligne blanche par défaut y serait invisible) ; en thème sombre, telle quelle.
/// Thèmes daltoniens (rouge-vert sombre, daltonien clair) : hausse bleue, baisse orange.
pub fn is_dalt(k: usize) -> bool {
    matches!(k, 6 | 7)
}

/// Teinte (0..360) d'une couleur, `None` si elle est presque grise.
fn hue(c: Rgb) -> Option<f64> {
    let (r, g, b) = (c.0 as f64, c.1 as f64, c.2 as f64);
    let (mx, mn) = (r.max(g).max(b), r.min(g).min(b));
    if mx - mn < 40.0 {
        return None;
    }
    let d = mx - mn;
    let h = if mx == r {
        ((g - b) / d).rem_euclid(6.0)
    } else if mx == g {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    Some(h * 60.0)
}

/// Thème daltonien `k` : un vert (ou turquoise) devient la couleur de hausse du thème, un rouge (ou rose) celle de
/// baisse ; l'or, le violet, le bleu et les gris restent. Autres thèmes : inchangée.
pub fn dalt_map(c: Rgb, k: usize) -> Rgb {
    if !is_dalt(k) {
        return c;
    }
    let th = crate::Theme::named(k);
    match hue(c) {
        Some(h) if (75.0..185.0).contains(&h) => th.bull,
        Some(h) if !(20.0..300.0).contains(&h) => th.bear,
        _ => c,
    }
}

pub fn on_chart(c: impl Into<Rgb>) -> Rgb {
    on_chart_for(c.into(), crate::theme_index())
}

/// [`on_chart`] pour le thème `k`.
pub fn on_chart_for(c: Rgb, k: usize) -> Rgb {
    let c = dalt_map(c, k);
    if is_light(k) {
        readable_on(c, crate::Theme::named(k).background, 3.0)
    } else {
        c
    }
}

pub fn light_ui(pm: &mut tiny_skia::Pixmap) {
    let w = pm.width() as usize;
    let src: Vec<tiny_skia::ColorU8> = pm.pixels().iter().map(|p| p.demultiply()).collect();
    let h = src.len() / w.max(1);
    // un pixel coloré est un trait fin (texte, icône, ligne) si un de ses voisins n'a pas la même couleur ; un aplat
    // (bouton, puce) garde sa couleur, le texte posé dessus reste lisible
    let same = |a: &tiny_skia::ColorU8, b: &tiny_skia::ColorU8| {
        (a.red() as i32 - b.red() as i32).abs() + (a.green() as i32 - b.green() as i32).abs() + (a.blue() as i32 - b.blue() as i32).abs() < 24
    };
    let sat = |c: &tiny_skia::ColorU8| {
        let (r, g, b) = (c.red() as i32, c.green() as i32, c.blue() as i32);
        r.max(g).max(b) - r.min(g).min(b) >= 40
    };
    // aplat coloré à gauche ET à droite sur la même rangée (texte posé sur un bouton) : on n'y touche pas
    // fenêtre de recherche de l'aplat : 16 points (en pixels de l'écran)
    let span = (16.0 * crate::px::scale()).round() as usize;
    let rad = (crate::px::scale().ceil() as i32).max(1);
    // un aplat (bouton) est plus large qu'un trait : identique jusqu'à `reach` pixels de chaque côté (à ×2 ou ×3,
    // l'intérieur d'une lettre ne passe plus pour un bouton)
    let reach = if rad <= 1 { 1 } else { rad as usize + 1 };
    let inside_fill = |x: usize, y: usize| {
        // cherche l'aplat dans une direction : trois pixels colorés identiques de suite (le bouton), sinon rien
        let find = |pos: &mut dyn Iterator<Item = (usize, usize)>, horiz: bool| -> Option<tiny_skia::ColorU8> {
            for (px, py) in pos {
                let p = &src[py * w + px];
                if p.alpha() == 0 {
                    return None;
                }
                let (a, b) = if horiz {
                    (px.checked_sub(reach).map(|k| py * w + k), (px + reach < w).then(|| py * w + px + reach))
                } else {
                    (py.checked_sub(reach).map(|k| k * w + px), (py + reach < h).then(|| (py + reach) * w + px))
                };
                if sat(p) && a.zip(b).is_some_and(|(a, b)| same(p, &src[a]) && same(p, &src[b])) {
                    return Some(*p);
                }
                if !sat(p) && (p.red() as i32) < 120 && (p.green() as i32) < 120 {
                    return None; // fond gris : pas sur un bouton
                }
            }
            None
        };
        let pair = |a: Option<tiny_skia::ColorU8>, b: Option<tiny_skia::ColorU8>| match (a, b) {
            (Some(a), Some(b)) if same(&a, &b) => Some(Rgb(a.red(), a.green(), a.blue())),
            _ => None,
        };
        // à gauche et à droite sur la rangée, sinon en haut et en bas dans la colonne (lettres serrées)
        pair(
            find(&mut (x.saturating_sub(span)..x).rev().map(|k| (k, y)), true),
            find(&mut (x + 1..(x + span + 1).min(w)).map(|k| (k, y)), true),
        )
        .or_else(|| {
            pair(
                find(&mut (y.saturating_sub(span)..y).rev().map(|k| (x, k)), false),
                find(&mut (y + 1..(y + span + 1).min(h)).map(|k| (x, k)), false),
            )
        })
    };
    for (i, p) in pm.pixels_mut().iter_mut().enumerate() {
        let c = src[i];
        let (r, g, b) = (c.red() as i32, c.green() as i32, c.blue() as i32);
        let (x, y) = (i % w, i / w);
        if r.max(g).max(b) - r.min(g).min(b) >= 40 {
            // trait fin : un pixel différent à moins de `rad` pixels (écran ×2, ×3 : les lettres ont 2 ou 3 pixels
            // d'épaisseur, leur intérieur doit être assombri comme leur bord)
            let thin = (1..=rad).any(|d| {
                [(-d, 0i32), (d, 0), (0, -d), (0, d)].iter().any(|&(dx, dy)| {
                    let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                    nx < 0 || ny < 0 || nx as usize >= w || ny as usize >= h || !same(&c, &src[ny as usize * w + nx as usize])
                })
            });
            let col = Rgb(c.red(), c.green(), c.blue());
            if thin {
                // sur un aplat (bouton, puce), l'aplat est gardé : le texte, choisi pour lui, et son anticrénelage
                // restent tels quels (pas de halo) ; sur le fond clair, le trait est foncé jusqu'à 4,5:1
                if inside_fill(x, y).is_none() && contrast(col, SNOW) < 4.5 {
                    let d = readable_on(col, SNOW, 4.5);
                    *p = tiny_skia::ColorU8::from_rgba(d.0, d.1, d.2, c.alpha()).premultiply();
                }
            }
            continue;
        }
        let l = (r * 299 + g * 587 + b * 114) / 1000;
        if inside_fill(x, y).is_some() {
            continue; // texte (et son anticrénelage) posé sur un bouton coloré : déjà choisi pour ce fond, on n'y touche pas
        }
        let d = light_gray(l) - l;
        let f = |v: i32| (v + d).clamp(0, 255) as u8;
        *p = tiny_skia::ColorU8::from_rgba(f(r), f(g), f(b), c.alpha()).premultiply();
    }
}

/// 🌞 Le thème `k` est-il clair (fond blanc : menus et panneaux retournés en clair) ?
pub fn is_light(k: usize) -> bool {
    matches!(k, 1 | 7 | 9)
}

/// 📝 Paires texte / fond de l'interface (après le passage en clair pour un thème clair) : (nom, texte, fond, contraste
/// minimal). Vérifiées par un test pour chaque thème.
pub fn pairs(k: usize) -> Vec<(&'static str, Rgb, Rgb, f64)> {
    let light = is_light(k);
    let g = |c: Rgb| {
        if light {
            let l = (c.0 as i32 * 299 + c.1 as i32 * 587 + c.2 as i32 * 114) / 1000;
            let d = light_gray(l) - l;
            let f = |v: u8| (v as i32 + d).clamp(0, 255) as u8;
            Rgb(f(c.0), f(c.1), f(c.2))
        } else {
            c
        }
    };
    let t = crate::Theme::named(k);
    let mut v = vec![
        ("texte sur panneau", g(WHITE), g(PANEL), 4.5),
        ("texte sur carte", g(WHITE), g(CARD), 4.5),
        ("texte secondaire sur panneau", g(TEXT), g(PANEL), 4.5),
        ("gris sur panneau", g(GREY), g(PANEL), 4.5),
        ("icône sur panneau", g(ICON), g(PANEL), 3.0),
        ("texte de la chart", t.text, t.background, 4.5),
    ];
    // texte sur les étiquettes colorées (hausse, baisse, accent, or) : encre choisie par `ink_on`
    for (n, c) in [("hausse", t.bull), ("baisse", t.bear), ("or", GOLD), ("rouge", RED), ("vert", GREEN)] {
        let _ = n;
        v.push(("texte sur étiquette colorée", ink_on(c), c, 4.5));
    }
    // couleurs vives posées en trait fin sur le fond clair : foncées par `light_ui`
    if light {
        for c in [GOLD, GREEN, RED, ORANGE, t.bull, t.bear] {
            v.push(("trait coloré sur fond clair", readable_on(c, SNOW, 4.5), SNOW, 4.5));
        }
    }
    v
}

/// 🌞 Thème clair : une image copiée telle quelle à l'écran (barres, panneau de droite) est posée sur du blanc, pour
/// que ses parties transparentes ne deviennent pas noires.
pub fn on_white(pm: &mut tiny_skia::Pixmap) {
    for p in pm.pixels_mut() {
        let a = p.alpha() as u16;
        if a < 255 {
            let f = |v: u8| (v as u16 + 255 - a).min(255) as u8;
            *p = tiny_skia::ColorU8::from_rgba(f(p.red()), f(p.green()), f(p.blue()), 255).premultiply();
        }
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn teinte() {
        use super::*;
        let k = crate::theme_index();
        crate::set_theme(crate::Theme::CLASSIC);
        assert_eq!(panel(), PANEL, "Classique inchangé");
        crate::set_theme(0);
        assert_eq!(hover(), HOVER, "Défaut inchangé");
        crate::set_theme(3);
        let o = panel();
        assert!(o.0 > o.2, "Or plus chaud");
        crate::set_theme(4);
        let m = panel();
        assert!(m.1 > m.0 && m.1 > m.2, "Matrix vert");
        crate::set_theme(k);
    }

    #[test]
    fn clair() {
        use super::*;
        // fond sombre, texte blanc, un aplat bleu de 9 × 3 avec un texte blanc au milieu, un trait jaune isolé
        let mut pm = tiny_skia::Pixmap::new(12, 3).unwrap();
        let px = |c: Rgb| tiny_skia::ColorU8::from_rgba(c.0, c.1, c.2, 255).premultiply();
        for (i, p) in pm.pixels_mut().iter_mut().enumerate() {
            let x = i % 12;
            *p = match x {
                0 => px(PANEL),
                1 => px(WHITE),
                2 => px(GOLD),
                3 => px(PANEL),
                8 => px(Rgb(255, 255, 255)),
                _ => px(ACCENT),
            };
        }
        light_ui(&mut pm);
        let get = |x: usize| {
            let c = pm.pixels()[12 + x].demultiply();
            Rgb(c.red(), c.green(), c.blue())
        };
        assert!(get(0).0 > 200, "fond clair");
        assert!(get(1).0 < 60, "texte foncé");
        assert!(contrast(get(2), SNOW) >= 4.5, "trait jaune foncé jusqu'à être lisible : {:?}", get(2));
        assert_eq!(get(5), ACCENT, "aplat coloré gardé");
        assert_eq!(get(8), Rgb(255, 255, 255), "texte blanc sur l'aplat : gardé blanc");
        assert_eq!((get(7), get(9)), (ACCENT, ACCENT), "pas de halo autour du texte sur l'aplat");
    }

    #[test]
    fn traits_epais_assombris_a_l_echelle_2() {
        use super::*;
        // écran ×2 : une lettre jaune de 3 pixels d'épaisseur sur le fond du panneau ; son milieu doit être foncé aussi
        crate::px::set_scale(2.0);
        let mut pm = tiny_skia::Pixmap::new(9, 9).unwrap();
        for (i, p) in pm.pixels_mut().iter_mut().enumerate() {
            let (x, y) = (i % 9, i / 9);
            let c = if (3..6).contains(&x) && (3..6).contains(&y) { GOLD } else { PANEL };
            *p = tiny_skia::ColorU8::from_rgba(c.0, c.1, c.2, 255).premultiply();
        }
        light_ui(&mut pm);
        crate::px::set_scale(1.0);
        let mid = pm.pixels()[4 * 9 + 4].demultiply();
        assert!(contrast(Rgb(mid.red(), mid.green(), mid.blue()), SNOW) >= 4.5, "milieu du trait : {mid:?}");
    }

    #[test]
    fn lignes_des_indicateurs_en_theme_clair() {
        use super::*;
        for k in [1, 7] {
            let bg = crate::Theme::named(k).background;
            for c in [Rgb(255, 255, 255), Rgb(0xff, 0xd6, 0x00), Rgb(0xee, 0xee, 0xee)] {
                assert!(contrast(on_chart_for(c, k), bg) >= 3.0, "thème {k}, {c:?}");
            }
            assert_eq!(on_chart_for(Rgb(0x13, 0x17, 0x22), k), Rgb(0x13, 0x17, 0x22), "déjà lisible : inchangée");
        }
        assert_eq!(
            on_chart_for(Rgb(255, 255, 255), crate::Theme::CLASSIC),
            Rgb(255, 255, 255),
            "thème sombre : telle quelle"
        );
    }

    #[test]
    fn verts_et_rouges_en_theme_daltonien() {
        use super::*;
        for k in [6, 7] {
            let th = crate::Theme::named(k);
            assert_eq!(dalt_map(Rgb(0x08, 0x99, 0x81), k), th.bull, "vert → hausse");
            assert_eq!(dalt_map(Rgb(0x00, 0xe6, 0x76), k), th.bull);
            assert_eq!(dalt_map(Rgb(0xf2, 0x36, 0x45), k), th.bear, "rouge → baisse");
            assert_eq!(dalt_map(Rgb(0xff, 0x40, 0x81), k), th.bear, "rose → baisse");
            assert_eq!(dalt_map(Rgb(0xff, 0xd6, 0x00), k), Rgb(0xff, 0xd6, 0x00), "or gardé");
            assert_eq!(dalt_map(Rgb(0x9c, 0x27, 0xb0), k), Rgb(0x9c, 0x27, 0xb0), "violet gardé");
        }
        assert_eq!(dalt_map(Rgb(0x08, 0x99, 0x81), crate::Theme::CLASSIC), Rgb(0x08, 0x99, 0x81));
    }

    #[test]
    fn icones_en_couleur() {
        use super::*;
        let k = crate::theme_index();
        crate::set_theme(crate::Theme::CLASSIC);
        // le gris neutre prend la couleur de la catégorie, le blanc la même en plus clair
        let c = icon_tint("calendar", ICON);
        assert_eq!(c, icon_color(IconCat::Info));
        let hot = icon_tint("calendar", WHITE);
        assert!(hot.0 >= c.0 && hot.1 >= c.1 && hot.2 >= c.2 && hot != c, "survol plus clair");
        // grisé et couleurs choisies : inchangés
        assert_eq!(icon_tint("calendar", DIM), DIM);
        assert_eq!(icon_tint("lock-key", RED), RED);
        // catégories
        assert_eq!(icon_cat("game-controller"), IconCat::Trading);
        assert_eq!(icon_cat("fib"), IconCat::Draw, "dessin d'Argos v1 pas listé : outil de dessin");
        assert_eq!(icon_cat("gear"), IconCat::Settings);
        // chaque thème a 7 couleurs différentes, jamais grises
        for t in 0..crate::Theme::NAMES.len() {
            crate::set_theme(t);
            let tab = icon_table(t);
            for (i, a) in tab.iter().enumerate() {
                assert!(a.0.max(a.1).max(a.2) - a.0.min(a.1).min(a.2) >= 40, "thème {t} : couleur {i} trop grise");
                assert!(tab.iter().skip(i + 1).all(|b| b != a), "thème {t} : couleurs distinctes");
            }
        }
        crate::set_theme(k);
    }
}

#[cfg(test)]
mod tests_pnl {
    use super::*;

    /// 💰 Gain en couleur de hausse, perte en couleur de baisse, zéro en gris (thème courant).
    #[test]
    fn couleur_du_pnl() {
        assert_eq!(pnl(96.7), up());
        assert_eq!(pnl(-0.01), down());
        assert_eq!(pnl(0.0), GREY);
        assert_ne!(up(), down());
    }
}
