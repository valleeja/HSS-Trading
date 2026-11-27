//! 📐 Choix de la mise en page selon l'écran réel : téléphone, tablette (portrait / paysage), desktop, et écrans en
//! deux segments (pliables à moitié ouverts, doubles écrans). Une seule fonction, [`choose`], sans état : l'app la
//! rappelle à chaque image, donc la rotation, le redimensionnement, le plein écran et le pliage s'appliquent en direct
//! sans recharger ni perdre l'état (chart, panneaux, dessins).
//!
//! Tout est en pixels logiques (pixels CSS dans le navigateur, pixels physiques ÷ facteur d'échelle sur le desktop),
//! jamais selon le nom de l'appareil. Seuils :
//! - téléphone : moins de [`PHONE_MAX`] px de large, ou (au doigt) le petit côté sous [`PHONE_MAX`] (téléphone couché) ;
//! - tablette : jusqu'à [`TABLET_MAX`] px de large, à la souris comme au doigt (Jee, 23 nov. : une tablette tactile
//!   prend les barres du PC à sa taille, iPad 13 pouces couché = desktop) ; portrait si plus haut que large ;
//! - desktop : au-delà.
//!
//! Au doigt (`pointer: coarse`), l'interface est agrandie ([`Choice::zoom`]) pour que les boutons fassent au moins
//! [`TOUCH_TARGET`] px ; à la souris, elle reste compacte.

/// Largeur (px logiques) sous laquelle c'est un téléphone.
pub const PHONE_MAX: f64 = 600.0;
/// Largeur (px logiques) jusqu'à laquelle c'est une tablette.
pub const TABLET_MAX: f64 = 1024.0;
/// Au doigt : grand côté jusqu'auquel c'est encore une tablette (iPad 13 pouces couché : 1366 px).
pub const TABLET_TOUCH_MAX: f64 = 1400.0;
/// Taille minimale d'un bouton au doigt (px logiques).
pub const TOUCH_TARGET: f64 = 40.0;
/// Hauteur des petits boutons de l'interface à la souris (barre du haut, outils) : le zoom au doigt les amène à
/// [`TOUCH_TARGET`].
pub const MOUSE_TARGET: f64 = 32.0;

/// Ce que l'app sait de l'écran.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Screen {
    /// Largeur et hauteur en px logiques (zones sûres déjà retirées : encoche, barre du bas d'iOS).
    pub w: f64,
    pub h: f64,
    /// Pixels physiques par px logique (`devicePixelRatio`, facteur d'échelle de l'écran).
    pub scale: f64,
    /// Pointeur principal au doigt (`pointer: coarse`, écran tactile).
    pub coarse: bool,
    /// Segments de l'écran (x, y, largeur, hauteur) quand il y en a plusieurs (`window.viewport.segments`) ; vide sinon.
    pub segments: Vec<(f64, f64, f64, f64)>,
    /// Posture annoncée (`navigator.devicePosture`) : `Some(true)` plié en livre / tente, `Some(false)` à plat.
    pub folded: Option<bool>,
}

/// Famille de mise en page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Phone,
    TabletPortrait,
    TabletLandscape,
    Desktop,
}

/// Écran coupé par une charnière : rien d'important ne se dessine dessous.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Split {
    /// Côte à côte : la chart à gauche de `at`, le panneau actif à droite de `at + gap`.
    Columns { at: f64, gap: f64 },
    /// L'un au-dessus de l'autre (« tente », « portable ») : la chart en haut, les contrôles sous `at + gap`.
    Rows { at: f64, gap: f64 },
}

/// 🔎 Compact interface (Jee, 17 Nov): the size factor of the whole interface, one per device (the single source of
/// the sizes), towards TradingView's sizes on each device. Phone: 0.8 (indicator names 18 → 14.4, price scale 14 →
/// 11.2, bars and buttons alike). Tablet and PC: 1 for now (measured next).
pub const COMPACTE_CELL: f64 = 0.8;

/// 👆 Smallest side of a touch target by the finger, in points: 44 real px at the 0.8 touch scale (55 × 0.8 = 44;
/// Jee, 21 Nov). Only the invisible zone grows: icons and text keep their drawn size. The PC keeps its own sizes.
pub const TOUCH_MIN_PT: f64 = 55.0;

/// 🔎 PC bar heights (Jee, 17 Nov), towards TradingView web: the text keeps its size (factor 1), only the bars get
/// thinner. Single source for the top bar rows, the right icon column and the rail buttons.
/// 1st row of the top bar (was 44).
pub const RANGEE1_PC: f64 = 36.0;
/// 2nd row of the top bar, tickers (was 32).
pub const RANGEE2_PC: f64 = 26.0;
/// 3rd row of the top bar, the symbol sheet (was 38).
pub const RANGEE3_PC: f64 = 30.0;
/// Buttons of the right icon column (was 36) ; the rail buttons are this plus 4 (was 40).
pub const BOUTON_PC: f64 = 30.0;

/// The compact factor of a device kind.
pub fn compacte(kind: Kind) -> f64 {
    match kind {
        Kind::Phone => COMPACTE_CELL,
        _ => 1.0,
    }
}

/// Mise en page choisie.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Choice {
    pub kind: Kind,
    pub split: Option<Split>,
    /// Agrandissement de toute l'interface (1 = taille normale).
    pub zoom: f64,
    /// Plus petite taille de bouton visée (px logiques, avant zoom).
    pub min_target: f64,
}

impl Choice {
    pub fn phone(&self) -> bool {
        self.kind == Kind::Phone
    }
    pub fn tablet(&self) -> bool {
        matches!(self.kind, Kind::TabletPortrait | Kind::TabletLandscape)
    }
    /// Les panneaux latéraux se replient en tiroir (ouverts à la demande, par-dessus / refermés en touchant la chart).
    pub fn drawer(&self) -> bool {
        self.kind == Kind::TabletPortrait && self.split.is_none()
    }
}

/// Deux segments → la charnière (côte à côte si le second commence à droite du premier).
fn split_of(seg: &[(f64, f64, f64, f64)]) -> Option<Split> {
    let [a, b] = seg else { return None };
    let (a, b) = if (b.0, b.1) < (a.0, a.1) { (b, a) } else { (a, b) };
    if b.0 >= a.0 + a.2 - 1.0 {
        Some(Split::Columns {
            at: a.0 + a.2,
            gap: (b.0 - (a.0 + a.2)).max(0.0),
        })
    } else if b.1 >= a.1 + a.3 - 1.0 {
        Some(Split::Rows {
            at: a.1 + a.3,
            gap: (b.1 - (a.1 + a.3)).max(0.0),
        })
    } else {
        None
    }
}

/// La mise en page pour cet écran.
pub fn choose(s: &Screen) -> Choice {
    let (w, h) = (s.w.max(1.0), s.h.max(1.0));
    let short = w.min(h);
    let split = split_of(&s.segments);
    let kind = match split {
        // double écran : chart d'un côté, panneau de l'autre (paysage), ou chart en haut et contrôles en bas
        Some(Split::Columns { .. }) => Kind::TabletLandscape,
        // l'un au-dessus de l'autre : disposition téléphone (chart en haut, panneau et barres du bas en bas)
        Some(Split::Rows { .. }) => Kind::Phone,
        None if w < PHONE_MAX || (s.coarse && short < PHONE_MAX) => Kind::Phone,
        // 📟 a touch tablet (its small side at least PHONE_MAX: iPad, Android tablet) takes the same layout as the mouse
        // at that size, the PC bars (Jee, 23 Nov: « ok tablette »); only the touch targets grow (`zoom`)
        None if w <= TABLET_MAX => {
            if h > w {
                Kind::TabletPortrait
            } else {
                Kind::TabletLandscape
            }
        }
        None => Kind::Desktop,
    };
    // au doigt : boutons d'au moins 40 px (le téléphone a déjà ses grandes barres) ; à la souris : compact
    let zoom = if s.coarse && kind != Kind::Phone { TOUCH_TARGET / MOUSE_TARGET } else { 1.0 };
    let zoom = zoom * compacte(kind);
    Choice {
        kind,
        split,
        zoom,
        min_target: if s.coarse { TOUCH_TARGET } else { 24.0 },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scr(w: f64, h: f64, scale: f64, coarse: bool) -> Screen {
        Screen {
            w,
            h,
            scale,
            coarse,
            ..Default::default()
        }
    }

    #[test]
    fn tailles_typiques() {
        // iPhone (portrait, couché)
        assert_eq!(choose(&scr(390.0, 844.0, 3.0, true)).kind, Kind::Phone);
        assert_eq!(choose(&scr(844.0, 390.0, 3.0, true)).kind, Kind::Phone, "téléphone couché : reste téléphone");
        // 📟 touch tablets (Jee, 23 Nov: « ok tablette »): the PC bars, the same layout as the mouse at that size, only
        // the touch targets bigger; iPad 13" 1366 × 1024 and 1024 × 1366, iPad Air 820 × 1180, iPad 9.7", Android
        for (w, h, kind) in [
            (1366.0, 1024.0, Kind::Desktop),
            (1024.0, 1366.0, Kind::TabletPortrait),
            (820.0, 1180.0, Kind::TabletPortrait),
            (1180.0, 820.0, Kind::Desktop),
            (768.0, 1024.0, Kind::TabletPortrait),
            (1024.0, 768.0, Kind::TabletLandscape),
            (800.0, 1280.0, Kind::TabletPortrait),
            (1280.0, 800.0, Kind::Desktop),
        ] {
            let doigt = choose(&scr(w, h, 2.0, true));
            assert_eq!(doigt.kind, kind, "tablette au doigt {w} × {h}");
            assert_eq!(doigt.kind, choose(&scr(w, h, 2.0, false)).kind, "{w} × {h}: as with the mouse");
            assert!(
                doigt.zoom * MOUSE_TARGET >= TOUCH_TARGET && doigt.min_target >= TOUCH_TARGET,
                "{w} × {h}: big targets"
            );
        }
        // phones by the finger: unchanged
        for (w, h) in [(390.0, 844.0), (430.0, 932.0)] {
            assert_eq!(choose(&scr(w, h, 3.0, true)).kind, Kind::Phone, "{w} × {h}");
            assert_eq!(choose(&scr(w, h, 3.0, true)).zoom, COMPACTE_CELL);
        }
        // 1080p à la souris, 4K avec facteur 2 (1920 × 1080 logiques)
        assert_eq!(choose(&scr(1920.0, 1080.0, 1.0, false)).kind, Kind::Desktop);
        assert_eq!(choose(&scr(3840.0 / 2.0, 2160.0 / 2.0, 2.0, false)).kind, Kind::Desktop);
        // petite fenêtre à la souris
        assert_eq!(choose(&scr(500.0, 700.0, 1.0, false)).kind, Kind::Phone);
        assert_eq!(choose(&scr(900.0, 700.0, 1.0, false)).kind, Kind::TabletLandscape);
        // grand écran tactile (portable tactile 1440 px) : desktop, mais boutons agrandis
        let c = choose(&scr(1600.0, 900.0, 1.5, true));
        assert_eq!(c.kind, Kind::Desktop);
        assert!(c.zoom * MOUSE_TARGET >= TOUCH_TARGET);
    }

    /// 📱 Tailles CSS (points) des téléphones récents, relevées le 2026-11-05 sur des tableaux de tailles
    /// (yesviz.com, sizzy.co, madegooddesigns.com) ; certaines valeurs Pixel varient d'une source à l'autre.
    const PHONES: [(&str, f64, f64); 22] = [
        ("iPhone SE", 375.0, 667.0),
        ("iPhone 12 / 13 mini", 375.0, 812.0),
        ("iPhone 12 / 13 / 14", 390.0, 844.0),
        ("iPhone 14 Plus", 428.0, 926.0),
        ("iPhone 14 Pro / 15 / 16", 393.0, 852.0),
        ("iPhone 15 / 16 Plus et Pro Max", 430.0, 932.0),
        ("iPhone 16 Pro / 17 / 17 Pro", 402.0, 874.0),
        ("iPhone 16 / 17 Pro Max", 440.0, 956.0),
        ("iPhone Air", 420.0, 912.0),
        ("iPhone (vue courte d'iOS 26.1)", 375.0, 751.0),
        ("Pixel 6 / 7 / 8", 412.0, 915.0),
        ("Pixel 7 Pro / 8 Pro", 448.0, 998.0),
        ("Pixel 9 / 10", 412.0, 924.0),
        ("Pixel 9 Pro / 10 Pro XL", 414.0, 921.0),
        ("Pixel 10 Pro", 427.0, 952.0),
        ("Pixel 9 Pro Fold plié", 412.0, 980.0),
        ("Galaxy S24 / S25", 360.0, 780.0),
        ("Galaxy S24 / S25 Ultra", 384.0, 832.0),
        ("Galaxy Z Fold plié", 344.0, 882.0),
        ("Galaxy Z Flip", 412.0, 1004.0),
        ("Petit Android", 360.0, 640.0),
        ("Surface Duo un écran", 540.0, 720.0),
    ];

    #[test]
    fn tous_les_telephones_recents() {
        use crate::shapes::Rect;
        for (name, w, h) in PHONES {
            assert_eq!(choose(&scr(w, h, 3.0, true)).kind, Kind::Phone, "{name} debout");
            assert_eq!(choose(&scr(h, w, 3.0, true)).kind, Kind::Phone, "{name} couché");
            assert_eq!(choose(&scr(w, h, 3.0, true)).zoom, COMPACTE_CELL, "{name}: no enlargement, the compact factor");
            // les menus tiennent dans l'écran (largeur et hauteur, barre du bas comprise)
            let a = crate::topmenu::ArgosMenu::new(Rect::new(4.0, 4.0, 100.0, 40.0), w, h - 112.0);
            assert!(
                a.rect.x >= 0.0 && a.rect.x + a.rect.w <= w && a.rect.y + a.rect.h <= h - 112.0,
                "{name} : menu Argos ▾ {:?}",
                a.rect
            );
        }
        // pliables ouverts et tablettes, au doigt : the PC bars at their size (Jee, 23 Nov)
        for (name, w, h) in [
            ("Pixel 9 Pro Fold ouvert", 884.0, 1032.0),
            ("Galaxy Z Fold ouvert", 884.0, 1104.0),
            ("iPad mini", 744.0, 1133.0),
            ("iPad Air 13", 1024.0, 1366.0),
        ] {
            assert_eq!(choose(&scr(w, h, 2.0, true)).kind, Kind::TabletPortrait, "{name}");
        }
    }

    #[test]
    fn doigt_ou_souris() {
        // tablette au doigt : the PC bars (Jee, 23 Nov), bigger targets (≥ 40 px)
        let t = choose(&scr(820.0, 1180.0, 2.0, true));
        assert_eq!(
            (t.kind, t.zoom, t.min_target),
            (Kind::TabletPortrait, TOUCH_TARGET / MOUSE_TARGET, TOUCH_TARGET)
        );
        // petite fenêtre à la souris : disposition tablette (portrait : panneaux en tiroir)
        assert!(choose(&scr(820.0, 1000.0, 1.0, false)).drawer(), "fenêtre haute : panneaux en tiroir");
        assert!(!choose(&scr(1000.0, 820.0, 1.0, false)).drawer(), "fenêtre large : panneaux visibles");
        // grand écran tactile : desktop aux boutons agrandis
        let g = choose(&scr(1600.0, 900.0, 1.5, true));
        assert!(g.zoom * MOUSE_TARGET >= TOUCH_TARGET, "portable tactile : boutons ≥ 40 px");
        let d = choose(&scr(1920.0, 1080.0, 1.0, false));
        assert_eq!((d.zoom, d.min_target), (1.0, 24.0), "souris : compact");
        assert_eq!(choose(&scr(390.0, 844.0, 3.0, true)).zoom, COMPACTE_CELL, "phone: the compact factor only");
    }

    #[test]
    fn pliables_et_doubles_ecrans() {
        // Fold plié (≈ 344 × 882) : la vue téléphone ; déplié (≈ 884 × 1104) : a tablet, the PC bars (Jee, 23 Nov)
        assert_eq!(choose(&scr(344.0, 882.0, 2.6, true)).kind, Kind::Phone);
        let open = Screen {
            folded: Some(false),
            ..scr(884.0, 1104.0, 2.6, true)
        };
        assert_eq!(choose(&open).kind, Kind::TabletPortrait);
        // Duo : deux segments de 540 × 720 côte à côte, charnière de 28 px
        let duo = Screen {
            segments: vec![(0.0, 0.0, 540.0, 720.0), (568.0, 0.0, 540.0, 720.0)],
            ..scr(1108.0, 720.0, 2.5, true)
        };
        let c = choose(&duo);
        assert_eq!(c.kind, Kind::TabletLandscape);
        assert_eq!(c.split, Some(Split::Columns { at: 540.0, gap: 28.0 }));
        assert!(!c.drawer());
        // Duo tourné (« portable ») : un segment en haut, un en bas
        let rows = Screen {
            segments: vec![(0.0, 568.0, 720.0, 540.0), (0.0, 0.0, 720.0, 540.0)],
            folded: Some(true),
            ..scr(720.0, 1108.0, 2.5, true)
        };
        assert_eq!(
            choose(&rows).split,
            Some(Split::Rows { at: 540.0, gap: 28.0 }),
            "ordre des segments indifférent"
        );
        // un seul segment : pas de charnière
        assert_eq!(choose(&scr(884.0, 1104.0, 2.6, true)).split, None);
    }

    #[test]
    fn reajustement_en_direct() {
        // rotation, redimensionnement, pliage : seule la taille change, le choix suit (fonction sans état)
        let mut s = scr(820.0, 1000.0, 1.0, false);
        assert_eq!(choose(&s).kind, Kind::TabletPortrait);
        (s.w, s.h) = (s.h, s.w);
        assert_eq!(choose(&s).kind, Kind::TabletLandscape);
        s.w = 344.0;
        assert_eq!(choose(&s).kind, Kind::Phone);
    }
}
