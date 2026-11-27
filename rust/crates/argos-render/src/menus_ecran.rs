//! 📱 Tous les menus et fenêtres tiennent dans l'écran visible d'un téléphone (iPhone : 375 × 812, zones sûres
//! 61 / 30 ⇒ 375 × 721 points ; 390 × 844) et défilent quand ils sont plus hauts.

use crate::listmenu::{ListMenu, MenuGroup, MenuItem};
use crate::shapes::Rect;

const SCREENS: [(f64, f64); 2] = [(375.0, 721.0), (390.0, 844.0)];

fn inside(r: Rect, w: f64, h: f64, what: &str) {
    assert!(
        r.x >= 0.0 && r.y >= 0.0 && r.x + r.w <= w + 0.5 && r.y + r.h <= h + 0.5,
        "{what} sort de l'écran {w}×{h} : {r:?}"
    );
}

#[test]
fn menus_dans_l_ecran_du_telephone() {
    for (w, h) in SCREENS {
        // Argos ▾ et ☰ Appli : sous le logo ; plus hauts que l'écran → ils défilent
        for app in [false, true] {
            let at = Rect::new(4.0, 6.0, 72.0, 38.0);
            let mut m = if app {
                crate::topmenu::ArgosMenu::app(at, w, h)
            } else {
                crate::topmenu::ArgosMenu::new(at, w, h)
            };
            inside(m.rect, w, h, "menu Argos");
            if m.full > m.rect.h {
                m.wheel(10_000.0);
                assert!(m.scroll > 0.0 && m.scroll <= m.full - m.rect.h + 0.5, "le menu Argos défile jusqu'au bout");
            }
        }
        // une longue liste (unités de temps : 40 lignes), ouverte en haut ou en bas de l'écran
        let items: Vec<MenuItem> = (0..40)
            .map(|k| MenuItem {
                id: k,
                label: format!("{k} minutes"),
                enabled: true,
                ..Default::default()
            })
            .collect();
        for anchor in [Rect::new(100.0, 50.0, 60.0, 30.0), Rect::new(100.0, h - 60.0, 60.0, 30.0)] {
            let l = ListMenu::new(
                vec![MenuGroup {
                    title: "MINUTES".into(),
                    items: items.clone(),
                    strip: false,
                }],
                vec![],
                anchor,
                w,
                h,
            );
            inside(l.rect, w, h, "liste");
        }
        // Panneaux (feuille au-dessus de la barre du bas), dispositions, arborescence
        for kind in [
            crate::sheet::SheetKind::Panels,
            crate::sheet::SheetKind::Quick,
            crate::sheet::SheetKind::Account,
        ] {
            let s = crate::sheet::Sheet::new(kind, Rect::new(10.0, h - 50.0, 100.0, 40.0), h - 60.0, w);
            inside(s.rect, w, h, "feuille Panneaux");
        }
        inside(
            crate::splits::Menu::new(Rect::new(200.0, 6.0, 40.0, 30.0), w, h).rect,
            w,
            h,
            "menu des dispositions",
        );
        inside(crate::objtree::ObjTree::new(50.0, w, h).rect, w, h, "arborescence");
    }
}
