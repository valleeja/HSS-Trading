//! ⌨️ Panneau « Raccourcis du clavier » (Ctrl + /) : la liste groupée des **vrais** raccourcis de l'app. La partie
//! chart est lue dans la table d'actions (`actions::BINDINGS`) : le texte affiché ne peut pas être différent de la
//! vraie touche. Les autres groupes sont ceux du code des fenêtres (`argos-desktop/src/input/keyboard.rs`), vérifiés
//! un par un ; un test ici les garde alignés sur la table.

use crate::accueil::Section;
use crate::actions::{binding_text, label, ChartAction, BINDINGS};
use crate::palette::{GREY, WHITE};
use crate::shapes::{round_fill, round_stroke, Rect};
use crate::text::{Align, Text};
use tiny_skia::Pixmap;

/// Groupe de la chart de chaque action de la table.
fn groupe(a: ChartAction) -> &'static str {
    use ChartAction::*;
    match a {
        ZoomIn | ZoomOut | ScrollLeft | ScrollRight | ScrollLeftFast | ScrollRightFast | GoStart | GoEnd | GoTo | ResetView | ResetTime => {
            "Chart : se déplacer"
        }
        WatchPrev | WatchNext | Reload | NextStyle | PrevStyle => "Chart : symbole et type",
        Invert | TogglePercent | ToggleLog => "Échelle de prix",
        Undo | Redo | CopyDrawing | PasteDrawing | HLineAtMouse => "Dessins",
        _ => "Général",
    }
}

/// Tous les raccourcis : (groupe, action, touches), dans l'ordre d'affichage.
pub fn liste() -> Vec<(&'static str, String, String)> {
    let mut v: Vec<(&'static str, String, String)> = vec![
        ("Général", "Recherche universelle".into(), "Ctrl + K".into()),
        ("Général", "Raccourcis du clavier (cette liste)".into(), "Ctrl + /".into()),
    ];
    // une action à plusieurs touches (Grossissement : Ctrl + ↑ / + / =) : une seule ligne
    for (k, m, a) in BINDINGS {
        let (g, n, t) = (groupe(*a), label(*a), binding_text(*k, *m));
        match v.iter_mut().find(|(vg, vn, _)| *vg == g && vn == n) {
            Some(l) => l.2 = format!("{} / {t}", l.2),
            None => v.push((g, n.into(), t)),
        }
    }
    // ⌂ Accueil ouvert : la lettre en coin de chaque carte
    for s in Section::ALL {
        v.push(("Accueil (page ouverte)", format!("Ouvrir {}", s.nom()), s.lettre().to_string()));
    }
    v.push(("Accueil (page ouverte)", "Fermer l'Accueil".into(), "Échap".into()));
    for (a, k) in [
        ("Sauvegarder et lancer le script", "Ctrl + S"),
        ("Tester le script", "Ctrl + Entrée"),
        ("Coller", "Ctrl + V"),
        ("Fermer le menu, puis l'éditeur", "Échap"),
    ] {
        v.push(("Éditeur de script", a.into(), k.into()));
    }
    for (a, k) in [
        ("Champ suivant (clé API, fiche de trade)", "Tab"),
        ("Valider", "Entrée"),
        ("Annuler / fermer", "Échap"),
        ("Envoyer la demande à Argos AI", "Entrée"),
    ] {
        v.push(("Fenêtres et formulaires", a.into(), k.into()));
    }
    // dans l'ordre des groupes
    const ORDRE: [&str; 8] = [
        "Général",
        "Accueil (page ouverte)",
        "Chart : se déplacer",
        "Chart : symbole et type",
        "Échelle de prix",
        "Dessins",
        "Éditeur de script",
        "Fenêtres et formulaires",
    ];
    v.sort_by_key(|(g, _, _)| ORDRE.iter().position(|o| o == g).unwrap_or(99));
    v
}

const TETE: f64 = 58.0;
const LIGNE: f64 = 30.0;
const TITRE: f64 = 40.0;

/// Le panneau ouvert (volet de 420 px à droite, plein écran sur téléphone).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Raccourcis {
    pub scroll: f64,
}

impl Raccourcis {
    /// Le volet dans une fenêtre `w` × `h` (sous la barre du haut `top`).
    pub fn rect(w: f64, h: f64, top: f64) -> Rect {
        let vw = if w < 600.0 { w } else { 420.0f64.min(w) };
        Rect::new(w - vw, top, vw, (h - top).max(100.0))
    }

    fn hauteur() -> f64 {
        let l = liste();
        let mut groupes = l.iter().map(|x| x.0).collect::<Vec<_>>();
        groupes.dedup();
        groupes.len() as f64 * TITRE + l.len() as f64 * LIGNE + 30.0
    }

    pub fn molette(&mut self, dy: f64, h: f64) {
        self.scroll = (self.scroll + dy).clamp(0.0, (Self::hauteur() - (h - TETE)).max(0.0));
    }

    /// Le ✕ (coordonnées du volet).
    pub fn fermer(r: Rect) -> Rect {
        Rect::new(r.w - 48.0, 11.0, 36.0, 36.0)
    }

    pub fn draw(&self, text: &Text, r: Rect, mouse: (f64, f64)) -> Option<Pixmap> {
        let mut pm = crate::px::new(r.w.max(1.0) as u32, r.h.max(1.0) as u32)?;
        // 🫧 jetons de la maquette (verre-2)
        use crate::jetons as j;
        crate::fill(&mut pm, 0.0, 0.0, r.w, r.h, j::FOND, 0.94);
        crate::fill(&mut pm, 0.0, 0.0, r.w, r.h, j::VERRE.0, j::VERRE.1);
        crate::fill(&mut pm, 0.0, 0.0, 1.0, r.h, j::sur_fond(j::LISERE), 1.0);
        let mut body = crate::px::new(r.w.max(1.0) as u32, (r.h - TETE).max(1.0) as u32)?;
        let mut y = -self.scroll;
        let mut dernier = "";
        for (g, a, k) in liste() {
            if g != dernier {
                y += TITRE;
                text.draw(&mut body, &g.to_uppercase(), (18.0, (y - 12.0) as f32), 11.5, GREY, Align::Left);
                dernier = g;
            }
            if y > -LIGNE && y < r.h {
                let kw = text.width(&k, 12.0) as f64 + 16.0;
                text.draw(
                    &mut body,
                    &text.fit(&a, 13.0, r.w - kw - 50.0),
                    (18.0, (y + 19.0) as f32),
                    13.0,
                    WHITE,
                    Align::Left,
                );
                let p = Rect::new(r.w - 18.0 - kw, y + 4.0, kw, 22.0);
                round_fill(&mut body, p, 6.0, j::VERRE2.0, j::VERRE2.1);
                round_stroke(&mut body, p, 6.0, j::sur_fond((j::BLANC, 0.3)), 1.0);
                text.draw(&mut body, &k, (p.cx() as f32, (p.cy() + 4.0) as f32), 12.0, WHITE, Align::Center);
            }
            y += LIGNE;
        }
        // barre de défilement fine (6 px, sans rail)
        let full = Self::hauteur();
        let vh = r.h - TETE;
        if full > vh {
            let th = (vh * vh / full).max(30.0);
            let ty = self.scroll / (full - vh) * (vh - th);
            crate::jetons::barre_defilement(&mut body, r.w - 2.0, ty, th, false);
        }
        crate::px::put(&mut pm, 0.0, TETE, &body);
        Text::graisse(crate::text::Graisse::Bold).draw(&mut pm, "Raccourcis du clavier", (18.0, 36.0), 17.0, j::TEXTE, Align::Left);
        let x = Self::fermer(r);
        if x.contains(mouse.0 - r.x, mouse.1 - r.y) {
            round_fill(&mut pm, x, 8.0, crate::palette::hover(), 1.0);
        }
        crate::shapes::lines(&mut pm, x.cx(), x.cy(), &[&[(-7.0, -7.0), (7.0, 7.0)], &[(7.0, -7.0), (-7.0, 7.0)]], GREY, 1.6);
        crate::fill(&mut pm, 0.0, TETE - 1.0, r.w, 1.0, j::sur_fond(j::LISERE), 1.0);
        Some(pm)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::{from_key, Key, Mods};

    #[test]
    fn chaque_raccourci_de_la_chart_est_le_vrai() {
        let l = liste();
        // toute la table d'actions y est, avec le même texte de touches
        for (k, m, a) in BINDINGS {
            assert!(
                l.iter().any(|(_, n, t)| n == label(*a) && t.split(" / ").any(|x| x == binding_text(*k, *m))),
                "{}",
                label(*a)
            );
            assert_eq!(from_key(*k, *m), Some(*a));
        }
        // Alt + H est la ligne horizontale (pas l'Accueil, tant que Jee n'a pas tranché)
        assert_eq!(
            from_key(
                Key::Char('h'),
                Mods {
                    alt: true,
                    ..Default::default()
                }
            ),
            Some(ChartAction::HLineAtMouse)
        );
        assert!(l.iter().any(|(_, _, t)| t == "Ctrl + K") && l.iter().any(|(_, _, t)| t == "Ctrl + /"));
        // les groupes se suivent (pas de groupe coupé en deux)
        let mut vus: Vec<&str> = vec![];
        for (g, _, _) in &l {
            if vus.last() != Some(g) {
                assert!(!vus.contains(g), "groupe {g} coupé");
                vus.push(g);
            }
        }
        assert_eq!(l.iter().filter(|x| x.0 == "Accueil (page ouverte)").count(), 11, "10 cartes + Échap");
        let zoom: Vec<_> = l.iter().filter(|x| x.1 == label(ChartAction::ZoomIn)).collect();
        assert_eq!(zoom.len(), 1, "une ligne par action");
        assert_eq!(zoom[0].2, "Ctrl + ↑ / + / =");
    }

    #[test]
    fn volet_420_et_plein_ecran_sur_telephone() {
        let r = Raccourcis::rect(1280.0, 800.0, 100.0);
        assert_eq!((r.x, r.w, r.y), (860.0, 420.0, 100.0));
        let r = Raccourcis::rect(390.0, 800.0, 56.0);
        assert_eq!((r.x, r.w), (0.0, 390.0));
        let mut p = Raccourcis::default();
        p.molette(10_000.0, 700.0);
        assert!(p.scroll > 0.0 && p.scroll < Raccourcis::hauteur());
        assert!(p.draw(&Text::default(), Raccourcis::rect(1280.0, 800.0, 100.0), (0.0, 0.0)).is_some());
    }
}
