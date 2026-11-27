//! 🖥 Colonne de modules (gauche / droite) à la souris : molette (la liste d'un module d'abord, puis la colonne), barre
//! de défilement, poignée sous chaque module (hauteur ; double-clic = automatique), ✕ du titre (le
//! module se décoche), titre glissé (ordre, comme ↑ ↓),
//! bord côté chart (largeur). Échap annule le geste en cours.

use argos_render::dock::{self as dk, DockPos};

use crate::state::GesteDock;
use crate::*;

/// Distance (px) avant qu'un titre appuyé se mette à glisser : un clic simple ne déplace rien.
const SEUIL_GLISSE: f64 = 5.0;
/// Largeur de la zone du bord qui règle la largeur de la colonne (de chaque côté du bord).
const BORD: f64 = 4.0;

impl App {
    /// 🖥 Colonne verticale affichée (gauche ou droite) ?
    fn dock_vertical(&self) -> bool {
        self.dock.rect.is_some() && self.watch.dock.pos != DockPos::Bottom
    }

    /// 🖥 La souris (fenêtre) est-elle sur la colonne, bord côté chart compris ? Coordonnées dans la colonne.
    pub(crate) fn dock_zone(&self, wx: f64, wy: f64) -> Option<(f64, f64)> {
        let r = self.dock.rect?;
        let (gauche, droite) = match self.watch.dock.pos {
            DockPos::Right => (BORD, 0.0),
            DockPos::Left => (0.0, BORD),
            DockPos::Bottom => (0.0, 0.0),
        };
        (wx >= r.x - gauche && wx < r.x + r.w + droite && wy >= r.y && wy < r.y + r.h).then_some((wx - r.x, wy - r.y))
    }

    /// 🖥 Sur le bord côté chart (`x` dans la colonne) ?
    fn dock_sur_bord(&self, x: f64) -> bool {
        let Some(r) = self.dock.rect.filter(|_| self.dock_vertical()) else {
            return false;
        };
        match self.watch.dock.pos {
            DockPos::Right => (-BORD..=BORD).contains(&x),
            DockPos::Left => (r.w - BORD..=r.w + BORD).contains(&x),
            DockPos::Bottom => false,
        }
    }

    /// 🖥 Curseur sur la colonne : ↔ sur le bord, ↕ sur une poignée, main sur un titre.
    pub(crate) fn dock_curseur(&self, x: f64, y: f64) -> CursorIcon {
        if !self.dock_vertical() {
            return CursorIcon::Default;
        }
        if self.dock_sur_bord(x) {
            CursorIcon::EwResize
        } else if dk::poignee_a(&self.dock.places, self.dock.defil, y).is_some() {
            CursorIcon::NsResize
        } else if dk::croix_a(&self.dock.places, self.dock.defil, x, y).is_some() {
            CursorIcon::Pointer
        } else if dk::titre_a(&self.dock.places, self.dock.defil, x, y).is_some() {
            CursorIcon::Grab
        } else {
            CursorIcon::Default
        }
    }

    /// 🖥 Appui dans la colonne (`x`, `y` dans la colonne) : un geste commence (bord, barre, poignée, titre) ? Sinon le
    /// clic va au module.
    pub(crate) fn dock_appui(&mut self, x: f64, y: f64) -> bool {
        let Some(r) = self.dock.rect.filter(|_| self.dock_vertical()) else {
            return false;
        };
        if self.dock_sur_bord(x) {
            self.dock.geste = Some(GesteDock::Bord {
                x0: self.pointer.win_mouse.0,
                w0: r.w,
                avant: self.dock.largeur,
            });
            return true;
        }
        if let Some(b) = dk::barre_defil(self.watch.dock.pos, r.w, r.h, self.dock.total, self.dock.defil) {
            if Rect::new(b.x - 4.0, 0.0, b.w + 8.0, r.h).contains(x, y) {
                // sur le pouce : il suit la souris ; à côté : la colonne saute à cet endroit
                if !(b.y..=b.y + b.h).contains(&y) {
                    let k = ((y - b.h / 2.0) / (r.h - b.h).max(1.0)).clamp(0.0, 1.0);
                    self.dock.defil = k * dk::defil_max(r.h, self.dock.total);
                }
                self.dock.geste = Some(GesteDock::Barre {
                    y0: y,
                    defil0: self.dock.defil,
                });
                return true;
            }
        }
        if let Some(m) = dk::poignee_a(&self.dock.places, self.dock.defil, y) {
            let now = Instant::now();
            // double-clic sur la poignée : la hauteur automatique revient
            if self.dock.poignee_avant.is_some_and(|(p, t)| p == m && now.duration_since(t).as_millis() < 350) {
                self.dock.poignee_avant = None;
                self.dock.regler_hauteur(m, None);
                self.dock_garder_hauteurs();
                return true;
            }
            self.dock.poignee_avant = Some((m, now));
            let h0 = self.dock.places.iter().find(|(x, _)| *x == m).map_or(dk::hauteur_min(), |(_, r)| r.h);
            self.dock.geste = Some(GesteDock::Poignee {
                m,
                y0: y,
                h0,
                avant: self.dock.hauteur(m),
            });
            return true;
        }
        // ✕ de l'en-tête : le module se décoche (le dernier masque la colonne), gardé dans dock.json
        if let Some(m) = dk::croix_a(&self.dock.places, self.dock.defil, x, y) {
            self.watch.dock.fermer(m);
            Self::write_prefs("dock.json", &self.watch.dock);
            return true;
        }
        if let Some(m) = dk::titre_a(&self.dock.places, self.dock.defil, x, y) {
            self.dock.geste = Some(GesteDock::Titre { m, y0: y, y, bouge: false });
            return true;
        }
        false
    }

    /// 🖥 La souris bouge pendant un geste de la colonne (`wx`, `wy` dans la fenêtre).
    pub(crate) fn dock_bouge(&mut self, wx: f64, wy: f64) {
        let Some(r) = self.dock.rect else { return };
        let y = wy - r.y;
        self.pointer.dock_mouse = Some((wx - r.x, y));
        match self.dock.geste {
            Some(GesteDock::Poignee { m, y0, h0, avant }) => {
                let h = (h0 + y - y0).max(dk::hauteur_min()).round();
                self.dock.regler_hauteur(m, Some(h));
                self.dock.geste = Some(GesteDock::Poignee { m, y0, h0, avant });
                // le bas de la colonne suit (la poignée reste sous la souris quand on agrandit le dernier module)
            }
            Some(GesteDock::Titre { m, y0, bouge, .. }) => {
                let bouge = bouge || (y - y0).abs() > SEUIL_GLISSE;
                self.dock.geste = Some(GesteDock::Titre { m, y0, y, bouge });
                // près du haut / du bas : la colonne défile pour atteindre les places cachées
                let max = dk::defil_max(r.h, self.dock.total);
                if bouge && y < 24.0 {
                    self.dock.defil = (self.dock.defil - 12.0).max(0.0);
                } else if bouge && y > r.h - 24.0 {
                    self.dock.defil = (self.dock.defil + 12.0).min(max);
                }
            }
            Some(GesteDock::Barre { y0, defil0 }) => {
                let max = dk::defil_max(r.h, self.dock.total);
                if let Some(b) = dk::barre_defil(self.watch.dock.pos, r.w, r.h, self.dock.total, defil0) {
                    let k = max / (r.h - b.h).max(1.0);
                    self.dock.defil = (defil0 + (y - y0) * k).clamp(0.0, max);
                }
            }
            Some(GesteDock::Bord { x0, w0, avant }) => {
                let dx = wx - x0;
                let w = if self.watch.dock.pos == DockPos::Left { w0 + dx } else { w0 - dx };
                self.dock.largeur = Some(w.clamp(dk::LARGEUR_MIN, dk::LARGEUR_MAX).round());
                self.dock.geste = Some(GesteDock::Bord { x0, w0, avant });
            }
            None => {}
        }
    }

    /// 🖥 Fin du geste (bouton relâché) : l'ordre, la hauteur ou la largeur sont gardés.
    pub(crate) fn dock_lacher(&mut self) {
        match self.dock.geste.take() {
            Some(GesteDock::Titre { m, y, bouge: true, .. }) => {
                let (i, _) = dk::depot(&self.dock.places, m, self.dock.defil, y);
                self.watch.dock.deplacer(m, i);
                Self::write_prefs("dock.json", &self.watch.dock);
            }
            Some(GesteDock::Poignee { .. }) => self.dock_garder_hauteurs(),
            Some(GesteDock::Bord { .. }) => {
                let l = self.dock.largeur;
                Self::interface2_changer(|i| i.modules_largeur = l);
            }
            _ => {}
        }
    }

    /// 🖥 ⬓ du volet : son module équivalent est coché et allumé dans la colonne de modules (gardé dans dock.json, comme
    /// une case du volet Modules), puis le volet se ferme.
    pub(crate) fn volet_en_module(&mut self, m: argos_render::dock::Module) {
        if let Some(i) = self.watch.dock.items.iter_mut().find(|i| i.module == m) {
            i.on = true;
        }
        self.watch.dock.on = true;
        Self::write_prefs("dock.json", &self.watch.dock);
        self.watch.prefs.open = false;
        self.save_watch();
    }

    /// 🖥 Échap pendant un geste de la colonne : tout revient comme avant (rien n'est gardé).
    pub(crate) fn dock_annuler(&mut self) -> bool {
        match self.dock.geste.take() {
            Some(GesteDock::Poignee { m, avant, .. }) => self.dock.regler_hauteur(m, avant),
            Some(GesteDock::Bord { avant, .. }) => self.dock.largeur = avant,
            Some(GesteDock::Barre { defil0, .. }) => self.dock.defil = defil0,
            Some(GesteDock::Titre { .. }) => {}
            None => return false,
        }
        true
    }

    fn dock_garder_hauteurs(&self) {
        let h = self.dock.hauteurs.clone();
        Self::interface2_changer(|i| i.modules_hauteurs = h);
    }

    /// 🖥 Molette sur la colonne (`x`, `y` dans la colonne ; `steps` > 0 : vers le haut) : la liste du module sous la
    /// souris défile tant qu'elle le peut, puis la colonne.
    pub(crate) fn dock_molette(&mut self, x: f64, y: f64, steps: f64) {
        let pas = -steps * 40.0;
        let vertical = self.dock_vertical();
        if let Some((m, b)) = self.dock.bodies.iter().copied().find(|(_, b)| b.contains(x, y)) {
            let max = (self.dock_content_h(m, b.w) - b.h).max(0.0);
            let v = self.dock.scroll.entry(m).or_insert(0.0);
            let nv = (*v + pas).clamp(0.0, max);
            if (nv - *v).abs() > 0.01 || !vertical {
                *v = nv;
                return;
            }
        }
        if let Some(r) = self.dock.rect.filter(|_| self.dock_vertical()) {
            self.dock.defil = (self.dock.defil + pas).clamp(0.0, dk::defil_max(r.h, self.dock.total));
        }
    }
}
