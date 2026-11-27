//! 🫧 Jetons du style verre, relevés dans la maquette validée par Jee (`croquis/maquette-verre/index.html`, :root et
//! les règles `.verre`, `.puce`, `.acc-*`, `.ap-*`). Un seul endroit : chaque écran refait (verre-2) les prend ici au
//! lieu d'écrire ses couleurs, rayons et tailles en dur. Les noms suivent ceux de la maquette.

use crate::text::Rgb;

// — couleurs (:root) —
/// `--bg` / fond des pages plein écran (`body` : #0a0d14).
pub const FOND: Rgb = Rgb(0x0a, 0x0d, 0x14);
/// `--fg` : texte principal.
pub const TEXTE: Rgb = Rgb(0xe8, 0xea, 0xf0);
/// `--dim` : texte secondaire.
pub const DIM: Rgb = Rgb(0x8d, 0x93, 0xa1);
/// `--acc` : accent bleu.
pub const ACCENT: Rgb = Rgb(0x4c, 0x8d, 0xff);
/// `--cyan` et violet des dégradés (bande de l'Accueil, lives).
pub const CYAN: Rgb = Rgb(0x38, 0xd6, 0xff);
pub const VIOLET: Rgb = Rgb(0x9b, 0x59, 0xff);
/// `--dn` : baisse, alerte forte (Tier 1).
pub const BAS: Rgb = Rgb(0xea, 0x39, 0x43);
/// Orange des annonces Tier 2 (`.tier.t2`).
pub const TIER2: Rgb = Rgb(0xf7, 0xa5, 0x31);
pub const BLANC: Rgb = Rgb(0xff, 0xff, 0xff);

// — verre (`.verre`) : (couleur, opacité) —
/// `--verre` : fond des panneaux en verre.
pub const VERRE: (Rgb, f32) = (Rgb(26, 30, 42), 0.6);
/// `--verre2` : puces, survol.
pub const VERRE2: (Rgb, f32) = (BLANC, 0.07);
/// `--lis` : liseré clair du bord.
pub const LISERE: (Rgb, f32) = (BLANC, 0.16);
/// Reflet du haut (`inset 0 1px 0 rgba(255,255,255,.12)`).
pub const REFLET: (Rgb, f32) = (BLANC, 0.12);
/// Ombre portée (`0 10px 30px rgba(0,0,0,.35)`) : décalage, flou, opacité.
pub const OMBRE: (f64, f64, f32) = (10.0, 30.0, 0.35);
/// Fond d'une grande carte de l'Accueil (`.acc-c`) et au survol.
pub const CARTE: (Rgb, f32) = (BLANC, 0.025);
pub const CARTE_SURVOL: (Rgb, f32) = (BLANC, 0.045);
/// Ligne d'aperçu (`.ap-l`).
pub const LIGNE: (Rgb, f32) = (BLANC, 0.04);
/// Puce allumée (`.puce.on`) : l'accent à 35 %.
pub const PUCE_ON: (Rgb, f32) = (ACCENT, 0.35);
/// Champ de recherche de l'Accueil (`.acc-q`).
pub const CHAMP: (Rgb, f32) = (BLANC, 0.06);

/// 🫧 Pills with round ends everywhere (Jee, 16 and 17 Nov: « des pastilles menu pilule bout rond partout »): the
/// radius of a bar, a row, a rail, a button's highlight or a help bubble is half its short side (an icon button's
/// square highlight becomes a circle).
pub fn pilule(r: crate::shapes::Rect) -> f64 {
    r.w.min(r.h) / 2.0
}

/// 🫧 Up to this short side (points), a rounded shape is a bar, a button, a chip, a field or a bubble: a pill (an
/// icon button's square: a circle). Taller ones are cards and windows: they keep their radius.
pub const PILULE_MAX: f64 = 48.0;
/// 🫧 Below this radius the corner is a tick box's or a colour swatch's: kept square-ish.
pub const R_CASE: f64 = 5.0;

/// 🫧 The one rule for every rounded shape (shapes::round_fill, round_stroke, autour): a short side up to
/// `PILULE_MAX` with a radius from `R_CASE` → a pill; otherwise the radius asked.
pub fn rayon(r: crate::shapes::Rect, radius: f64) -> f64 {
    if r.w.min(r.h) <= PILULE_MAX && radius >= R_CASE {
        pilule(r)
    } else {
        radius
    }
}

// — rayons —
pub const R_CARTE: f64 = 22.0;
pub const R_APERCU: f64 = 16.0;
pub const R_TUILE: f64 = 16.0;
pub const R_PUCE: f64 = 12.0;
pub const R_LIGNE: f64 = 10.0;
// — rayons du cadre PC (dist/maquette/index.html) —
/// 1re rangée du haut (`#haut{border-radius:20px}`, l.54).
#[cfg(test)]
pub const R_HAUT: f64 = 20.0;
/// Rangées 2 et 3 (`#bandeau,#infosym{border-radius:14px}`, l.310).
#[cfg(test)]
pub const R_RANGEE: f64 = 14.0;
/// Rail de gauche (`#rail{border-radius:16px}`, l.65).
pub const R_RAIL: f64 = 16.0;
/// Colonne d'icônes de droite (`#icones{border-radius:18px}`, l.78).
pub const R_COLONNE: f64 = 18.0;
/// Boutons du rail et de la colonne d'icônes (`#rail button`, `#icones button{border-radius:11px}`, l.69 et 80).
pub const R_BOUTON: f64 = 11.0;
/// Barre de favoris (`#favbar{border-radius:22px}`, l.122) et ses boutons (`#favbar button{border-radius:12px}`, l.124).
pub const R_FAVBAR: f64 = 22.0;
pub const R_BOUTON_FAV: f64 = 12.0;
/// Volet de droite (`#panneau{border-radius:20px}`, l.113).
pub const R_VOLET: f64 = 20.0;
/// Menus et listes (`.pop{border-radius:16px}`, l.157 ; `#flyout{border-radius:16px}`, l.73).
pub const R_MENU: f64 = 16.0;
/// Bulle d'aide (`#infobulle{border-radius:9px;padding:6px 9px;font-size:11px}`, l.155).
pub const R_INFOBULLE: f64 = 9.0;
/// Fenêtres et boîtes (`.fen{border-radius:24px}`, l.183).
pub const R_FENETRE: f64 = 24.0;
/// Pied flottant (`#pied{border-radius:14px}`, l.118).
pub const R_PIED: f64 = 14.0;
/// Carte du Gardien (`.gd{border-radius:14px}`, l.734).
pub const R_GARDIEN: f64 = 14.0;
/// Taille des émojis de la colonne d'icônes (`#icones button{font-size:16px}`, l.80).
pub const T_EMOJI_COLONNE: f32 = 16.0;

// — tailles de texte —
pub const T_SALUT: f32 = 28.0;
pub const T_TITRE_CARTE: f32 = 21.0;
pub const T_TITRE_PAGE: f32 = 18.0;
pub const T_TUILE: f32 = 15.0;
pub const T_TEXTE: f32 = 13.0;
pub const T_APERCU: f32 = 12.5;
pub const T_PETIT: f32 = 11.0;

// — marges —
/// Largeur maximale du contenu (`.acc` : 1180 px).
pub const LARGEUR_MAX: f64 = 1180.0;
pub const ESPACE: f64 = 16.0;
pub const ESPACE_TUILES: f64 = 12.0;
/// Intérieur d'une grande carte (`padding: 22px 22px 0`).
pub const PAD_CARTE: f64 = 22.0;
pub const HAUT_CARTE_MIN: f64 = 280.0;
/// Bande dégradée de l'Accueil (`.acc-bande` : 90 px).
pub const BANDE: f64 = 90.0;

// — barre de défilement (une seule partout, maquette : `--defil`) —
/// Fine pilule transparente aux couleurs du thème (jamais une barre grise opaque) ; plus visible au survol ou attrapée.
pub const DEFIL: (Rgb, f32) = (ACCENT, 0.25);
pub const DEFIL_SURVOL: (Rgb, f32) = (ACCENT, 0.55);
/// Largeur de la pilule (points).
pub const DEFIL_L: f64 = 4.0;

/// 🫧 Dessine LA barre de défilement : une pilule de `DEFIL_L` de large, bord droit à `droite`, de `y` sur `h` (pas de
/// rail). `actif` : survolée ou attrapée.
pub fn barre_defilement(pm: &mut tiny_skia::Pixmap, droite: f64, y: f64, h: f64, actif: bool) {
    let (c, a) = if actif { DEFIL_SURVOL } else { DEFIL };
    let r = crate::shapes::Rect::new(droite - DEFIL_L - 2.0, y, DEFIL_L, h.max(DEFIL_L));
    crate::shapes::round_fill(pm, r, DEFIL_L / 2.0, c, a);
}

/// Une couleur translucide `(c, a)` posée sur le fond : pour les traits (liserés) qu'on ne peut pas peindre en
/// transparence.
pub fn sur_fond((c, a): (Rgb, f32)) -> Rgb {
    let m = |x: u8, f: u8| (f as f32 + (x as f32 - f as f32) * a).round() as u8;
    Rgb(m(c.0, FOND.0), m(c.1, FOND.1), m(c.2, FOND.2))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::palette::contrast;

    /// Le texte principal et le texte secondaire restent lisibles sur le fond (WCAG : 4,5:1 au moins).
    #[test]
    fn contraste_des_textes_sur_le_fond() {
        assert!(contrast(TEXTE, FOND) >= 4.5, "texte {}", contrast(TEXTE, FOND));
        assert!(contrast(DIM, FOND) >= 4.5, "dim {}", contrast(DIM, FOND));
        assert!(contrast(ACCENT, FOND) >= 4.5, "accent {}", contrast(ACCENT, FOND));
    }

    #[test]
    fn lisere_pose_sur_le_fond() {
        assert_eq!(sur_fond((BLANC, 0.0)), FOND);
        assert_eq!(sur_fond((BLANC, 1.0)), BLANC);
        let l = sur_fond(LISERE);
        assert!(l.0 > FOND.0 && l.0 < 0x40, "{l:?}");
    }
}

#[cfg(test)]
mod tests_defilement {
    use super::*;

    /// La barre de défilement unique (Jee, 2026-11-09) : une fine pilule translucide à l'accent du thème, sans rail,
    /// plus visible au survol ; jamais grise ni opaque.
    #[test]
    fn barre_de_defilement_pilule_translucide() {
        let mut pm = crate::px::new(40, 100).unwrap();
        barre_defilement(&mut pm, 40.0, 10.0, 60.0, false);
        let p = pm.pixel(36, 40).unwrap();
        assert!(p.alpha() > 0 && p.alpha() < 128, "translucide : {}", p.alpha());
        assert!(p.demultiply().blue() > p.demultiply().red(), "à l'accent (bleu), pas gris");
        assert_eq!(pm.pixel(10, 40).unwrap().alpha(), 0, "pas de rail");
        let mut actif = crate::px::new(40, 100).unwrap();
        barre_defilement(&mut actif, 40.0, 10.0, 60.0, true);
        assert!(actif.pixel(36, 40).unwrap().alpha() > p.alpha(), "plus visible au survol");
    }
}

/// 🫧 Fond d'une fenêtre (boîte de clé, formulaire, réglages d'un dessin…) : le verre de la maquette ; sans le style
/// verre, un fond presque plein dessous (lisible sur la chart).
pub fn fond_fenetre(pm: &mut tiny_skia::Pixmap, r: crate::shapes::Rect, rayon: f64) {
    if !crate::glass::on() {
        crate::shapes::round_fill(pm, r, rayon, FOND, 0.94);
    }
    verre(pm, r, rayon);
}

/// 🫧 LE style verre de la maquette (`.verre`), défini une seule fois : fond translucide, coins arrondis de `rayon`
/// (pilule : `r.h / 2`), fin liseré clair. Chaque menu, volet, feuille, boîte et barre passe par lui.
pub fn verre(pm: &mut tiny_skia::Pixmap, r: crate::shapes::Rect, rayon: f64) {
    crate::shapes::round_fill(pm, r, rayon, VERRE.0, VERRE.1);
    crate::shapes::round_stroke(pm, r, rayon, sur_fond(LISERE), 1.0);
}

/// 🌑 L'ombre portée de la maquette (`box-shadow: 0 10px 30px rgba(0,0,0,.35)`, `OMBRE`) au-dessus du bord du haut
/// d'une feuille posée en bas : son opacité à `d` points au-dessus du bord (flou de 30 px = écart type 15 ; l'ombre
/// est décalée de 10 vers le bas). Environ 0,09 contre le bord, presque rien à 30 points.
pub fn ombre_au_dessus(d: f64) -> f32 {
    let (dy, flou, a) = OMBRE;
    let z = (d.max(0.0) + dy) / (flou / 2.0) / std::f64::consts::SQRT_2;
    // erfc (Abramowitz et Stegun 7.1.26, erreur < 1,5e-7) ; Q(x) = erfc(x / √2) / 2
    let t = 1.0 / (1.0 + 0.327_591_1 * z);
    let poly = t * (0.254_829_592 + t * (-0.284_496_736 + t * (1.421_413_741 + t * (-1.453_152_027 + t * 1.061_405_429))));
    (a as f64 * poly * (-z * z).exp() / 2.0) as f32
}

#[cfg(test)]
mod ombre {
    #[test]
    fn ombre_portee_comme_la_maquette() {
        let o = super::ombre_au_dessus;
        assert!((o(0.0) - 0.0884).abs() < 0.002, "contre le bord : {}", o(0.0));
        assert!(o(10.0) < o(0.0) && o(30.0) < 0.003, "elle s'efface : {} {}", o(10.0), o(30.0));
    }

    /// 🖥 Cadre PC : les coins des éléments ont le rayon de leur jeton (celui de la maquette). Près du coin (à 0,2 rayon du
    /// bord), on est hors de la forme avec le bon rayon ; avec un rayon deux fois plus petit, on serait dedans.
    #[test]
    fn coins_du_cadre_pc_aux_rayons_de_la_maquette() {
        use crate::shapes::Rect;
        let px = |pm: &tiny_skia::Pixmap, x: f64, y: f64| {
            let p = pm.pixel(x as u32, y as u32).unwrap();
            (p.red(), p.green(), p.blue(), p.alpha())
        };
        // (rect, rayon attendu) : le pixel près du coin diffère du pixel au milieu du bord du haut
        let coin = |pm: &tiny_skia::Pixmap, r: Rect, k: f64, quoi: &str| {
            let (dedans, pres) = (px(pm, r.x + r.w / 2.0, r.y + k), px(pm, r.x + 0.2 * k, r.y + 0.2 * k));
            assert_ne!(dedans, pres, "{quoi} : coin arrondi au rayon {k}");
        };
        // help bubble: a pill (see `pilules_partout`; the rail has no card any more: `rail_sans_cadre`)
        let t = crate::text::Text::default();
        let pm = crate::toolbar::tip_pixmap(&t, "Fibonacci et Gann").unwrap();
        let r = Rect::new(0.5, 0.5, crate::px::lw(&pm) - 1.0, 25.0);
        coin(&pm, r, super::pilule(r), "bulle d'aide");
        // menus (`.pop` 16)
        let mut pm = crate::px::new(240, 200).unwrap();
        crate::shapes::menu_card(&mut pm, Rect::new(0.0, 0.0, 240.0, 200.0));
        coin(&pm, Rect::new(0.0, 0.0, 240.0, 200.0), super::R_MENU, "menu");
        // 🫧 pilules à bouts ronds partout (Jee, 16 nov.) : ligne de menu choisie, pastille de choix (rayon = moitié de
        // la hauteur, jamais moins que son jeton)
        let pilule = |f: &dyn Fn(&mut tiny_skia::Pixmap, Rect), h: f64, jeton: f64, quoi: &str| {
            let mut pm = crate::px::new(200, 60).unwrap();
            let r = Rect::new(10.0, 10.0, 160.0, h);
            f(&mut pm, r);
            coin(&pm, r, (h / 2.0).max(jeton), quoi);
        };
        pilule(
            &|pm, r| crate::shapes::menu_row(pm, r, true, false),
            38.0,
            super::R_LIGNE,
            "ligne de menu choisie",
        );
        pilule(
            &|pm, r| crate::shapes::menu_row(pm, r, false, true),
            38.0,
            super::R_LIGNE,
            "ligne de menu survolée",
        );
        pilule(&|pm, r| crate::shapes::pilule(pm, r, true, false), 24.0, super::R_PUCE, "pastille choisie");
        pilule(&|pm, r| crate::shapes::pilule(pm, r, false, false), 24.0, super::R_PUCE, "pastille");
        assert_eq!(
            (
                super::R_HAUT,
                super::R_RANGEE,
                super::R_COLONNE,
                super::R_BOUTON,
                super::R_FAVBAR,
                super::R_BOUTON_FAV
            ),
            (20.0, 14.0, 18.0, 11.0, 22.0, 12.0),
            "#haut, #bandeau, #icones, boutons, #favbar, ses boutons"
        );
        assert_eq!((super::R_VOLET, super::R_PIED, super::R_GARDIEN), (20.0, 14.0, 14.0), "#panneau, #pied, .gd");
    }

    /// 🫧 Jee (17 Nov, « tu n'as pas réglé les coins carrés »): the rail, its buttons' highlight and the help bubbles
    /// are pills or circles — their radius is half their short side. Why the test above missed them: it looked 0.2
    /// radius from the corner, where a corner of 16 on a 44 wide rail is outside too; here the point is 0.24 × half
    /// the short side from the corner: outside a pill, inside any corner smaller than 3/4 of it.
    #[test]
    fn pilules_partout() {
        use crate::shapes::Rect;
        let px = |pm: &tiny_skia::Pixmap, x: f64, y: f64| {
            let p = pm.pixel(x as u32, y as u32).unwrap();
            (p.red(), p.green(), p.blue(), p.alpha())
        };
        let proche =
            |a: (u8, u8, u8, u8), b: (u8, u8, u8, u8)| a.0.abs_diff(b.0) <= 8 && a.1.abs_diff(b.1) <= 8 && a.2.abs_diff(b.2) <= 8 && a.3.abs_diff(b.3) <= 8;
        // `dehors`: a pixel outside the shape; the point near the corner is like it, the middle of the left edge is not
        let pilule = |pm: &tiny_skia::Pixmap, r: Rect, dehors: (f64, f64), quoi: &str| {
            let k = r.w.min(r.h) / 2.0;
            let fond = px(pm, dehors.0, dehors.1);
            for (cx, cy) in [(r.x, r.y), (r.x + r.w, r.y), (r.x, r.y + r.h), (r.x + r.w, r.y + r.h)] {
                let (sx, sy) = (if cx > r.x { -1.0 } else { 1.0 }, if cy > r.y { -1.0 } else { 1.0 });
                let p = px(pm, cx + sx * 0.24 * k, cy + sy * 0.24 * k);
                assert!(
                    proche(p, fond),
                    "{quoi}: corner ({cx}, {cy}) rounded to half its short side ({k}): {p:?} vs {fond:?}"
                );
            }
            assert!(!proche(px(pm, r.x + k, r.cy()), fond), "{quoi}: filled inside");
        };
        // a chosen rail button: a circle (the first button is the chosen cursor)
        let (b, _) = crate::rail::layout(600.0)[0];
        let mut nu = crate::rail::draw(&crate::rail::Prefs::default(), 600, "aucun", false, None).unwrap();
        let on = crate::rail::draw(&crate::rail::Prefs::default(), 600, "cursor", false, None).unwrap();
        // (only the highlight differs between the two images: compare them pixel by pixel near the corner)
        let k = b.w.min(b.h) / 2.0;
        let coin = (b.x + 0.24 * k, b.y + 0.24 * k);
        assert_eq!(px(&nu, coin.0, coin.1), px(&on, coin.0, coin.1), "rail button highlight: a circle");
        assert_ne!(px(&nu, b.x + 2.0, b.cy()), px(&on, b.x + 2.0, b.cy()), "rail button highlight: drawn");
        let _ = &mut nu;
        // a help bubble (`#infobulle`)
        let t = crate::text::Text::default();
        let pm = crate::toolbar::tip_pixmap(&t, "Fibonacci et Gann").unwrap();
        pilule(&pm, Rect::new(0.5, 0.5, crate::px::lw(&pm) - 1.0, 25.0), (0.0, 0.0), "bulle d'aide");
    }
}
