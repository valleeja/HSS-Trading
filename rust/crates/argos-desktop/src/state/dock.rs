//! 🧩 Colonne de modules : sa place dans la fenêtre, sa largeur de chaque côté de la chart, la zone de contenu de
//! chaque module et son défilement.

use std::collections::HashMap;

use argos_render::dock::Module;
use argos_render::shapes::Rect;

pub(crate) struct DockState {
    /// Place de la colonne dans la fenêtre (`None` : pas affichée).
    pub(crate) rect: Option<Rect>,
    /// Largeur à gauche / à droite de la chart (la chart commence après celle de gauche ; à droite, aussi la charnière
    /// d'un double écran et la place sous le volet du Simulateur).
    pub(crate) left: f64,
    pub(crate) right: f64,
    /// 🧪 PC : dans `right`, la largeur de la chart passée sous le volet du Simulateur (Suivi 3 : la chart s'arrête à
    /// son bord) ; le bouton Argos AI garde sa place, au coin bas droit, sur la bande du bas du volet.
    pub(crate) sous_sim: f64,
    /// Zone de contenu de chaque module.
    pub(crate) bodies: Vec<(Module, Rect)>,
    /// Défilement de chaque module (molette dessus).
    pub(crate) scroll: HashMap<Module, f64>,
    /// 🖥 Colonne PC : place de chaque module (du haut de la colonne, sans défilement), hauteur totale, défilement de la
    /// colonne (gardé pendant la session).
    pub(crate) places: Vec<(Module, Rect)>,
    pub(crate) total: f64,
    pub(crate) defil: f64,
    /// 🖥 Hauteurs réglées à la main (poignées) et largeur réglée (bord) : interface-2.json.
    pub(crate) hauteurs: Vec<(Module, f64)>,
    pub(crate) largeur: Option<f64>,
    /// 🖥 Geste en cours dans la colonne (Échap l'annule).
    pub(crate) geste: Option<GesteDock>,
    /// 🖥 Dernier appui sur une poignée (double-clic : hauteur automatique).
    pub(crate) poignee_avant: Option<(Module, web_time::Instant)>,
}

/// 🖥 Geste dans la colonne de modules.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum GesteDock {
    /// Poignée sous `m` : sa hauteur suit la souris (départ `y0`, hauteur de départ `h0`).
    Poignee { m: Module, y0: f64, h0: f64, avant: Option<f64> },
    /// Titre de `m` glissé (au-delà de 5 px : il se déplace) ; `y` : souris dans la colonne affichée.
    Titre { m: Module, y0: f64, y: f64, bouge: bool },
    /// Barre de défilement glissée.
    Barre { y0: f64, defil0: f64 },
    /// Bord de la colonne côté chart : la largeur suit la souris.
    Bord { x0: f64, w0: f64, avant: Option<f64> },
}

impl DockState {
    /// Hauteur réglée à la main de `m`.
    pub(crate) fn hauteur(&self, m: Module) -> Option<f64> {
        self.hauteurs.iter().find(|(x, _)| *x == m).map(|(_, h)| *h)
    }

    pub(crate) fn regler_hauteur(&mut self, m: Module, h: Option<f64>) {
        self.hauteurs.retain(|(x, _)| *x != m);
        if let Some(h) = h {
            self.hauteurs.push((m, h));
        }
    }
}
