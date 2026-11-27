//! 📋 Movable and resizable tables (tableaux-1, Jee 22 Nov; mockup `2026-11-23-maquette-tableaux-deplacables-v1`).
//! (tour 3, Jee 23 Nov) Nothing is drawn around a table: under the mouse or the finger its own border just gets a
//! little brighter. PC: the cursor shows what a press takes: drag the body = move, a corner = scale everything (the
//! opposite corner held), an edge = stretch; double-click = back in place; right-click = its small menu. Touch (cell,
//! tablet): a tap selects it (brighter border, hint pill); then one finger moves it, a drag from near a corner or a
//! pinch scales it, the chart does not pan or zoom; a tap elsewhere or « OK » = done. Each table's layout is kept
//! with the indicator settings (`indicator_settings.json`, `tableaux`); removing the indicator forgets it.

use crate::*;
use argos_render::tableaux::{self, Menu, MenuLigne, Mode, Prise, TableLayout};

/// A table being dragged (mouse or finger): what was taken and where it started (chart coordinates).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Glisse {
    pub(crate) key: String,
    pub(crate) prise: Prise,
    pub(crate) x0: f64,
    pub(crate) y0: f64,
    pub(crate) rect0: Rect,
    pub(crate) area: Rect,
    pub(crate) lay0: TableLayout,
    pub(crate) bouge: bool,
}

/// The tables' pointer state.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct TabState {
    /// 🖱 The table (and what of it) under the mouse.
    pub(crate) survol: Option<(String, Prise)>,
    /// 👆 The table selected by the finger.
    pub(crate) choisi: Option<String>,
    /// The table's right-click menu.
    pub(crate) menu: Option<Menu>,
    pub(crate) glisse: Option<Glisse>,
    /// 👆 A press outside the selected table: where (a tap there ends the selection, a drag does nothing).
    pub(crate) hors: Option<(f64, f64)>,
    /// The release of a press the tables took (nothing else sees it).
    pub(crate) avale: bool,
    /// 🖱 The last press on a table (a second one soon after, at the same place: back in place).
    pub(crate) dernier: Option<Instant>,
    /// 👆 Pinched since the selection: kept when it ends.
    pub(crate) a_garder: bool,
}

impl App {
    /// 📋 What the drawing needs: the layouts, the table shown hovered / selected and its menu, and the legend's open
    /// list as a place no table goes under (T6 on the cell; tour 3, H1, on the PC and the tablets).
    pub(crate) fn tableaux_pour_dessin(&self) {
        tableaux::set_layouts(&self.ind.tables);
        let t = &self.pointer.tab;
        let actif = if let Some(k) = &t.choisi {
            Some((k.clone(), Mode::Choisi))
        } else if let Some(g) = &t.glisse {
            Some((g.key.clone(), Mode::Survol))
        } else {
            t.survol.as_ref().map(|(k, _)| (k.clone(), Mode::Survol))
        };
        tableaux::set_actif(actif);
        tableaux::set_menu(t.menu.clone());
        let liste = if self.top.phone && self.ind.leg.0 {
            // (the cell's rows are taller than the PC's: down to the cell list's real bottom, plus room for a
            // selected row's action bar)
            argos_render::indleg::lignes_liste(&self.ind.out)
                .map(|(haut, _)| {
                    let bas = argos_render::indleg::bas_legende(&self.ind.out, true, true) + 12.0;
                    Rect::new(0.0, haut, self.chart.viewport.plot_w(), bas - haut)
                })
                .into_iter()
                .collect()
        } else if self.ind.leg.0 {
            // (tour 3, H1: 🖥 PC and 📟 tablets) the open list's rows whole, from the left edge to the end of the
            // widest row's buttons: the legend's zones are only its buttons, a table went under the rows' names
            let text = argos_render::text::Text::default();
            let vp = self.chart.viewport;
            argos_render::indleg::liste_rect(&text, &self.ind.on, &self.ind.out, vp.plot_h())
                .map(|r| Rect::new(0.0, r.y, (r.w + 8.0).min(vp.plot_w()), r.h + 4.0))
                .into_iter()
                .collect()
        } else {
            vec![]
        };
        argos_render::set_reserve_tableaux(liste);
    }

    /// 🖱 The table and its part under the mouse at (`x`, `y`) (chart coordinates): PC only, nothing over it.
    pub(crate) fn tab_prise_sous(&self, x: f64, y: f64) -> Option<(tableaux::Pose, Prise)> {
        if argos_render::actions::tactile() || self.modale_ouverte() || self.couche_haute(x + self.rw(), y + self.top_h()) {
            return None;
        }
        // (a legend's button next to a table stays the legend's)
        if self.legende_bouton(x, y).is_some() || self.pad_bouton(x, y).is_some() {
            return None;
        }
        // (its edge's zones reach a little outside it)
        let p = tableaux::pose_sous(x, y, tableaux::BORD_SOURIS)?;
        let pr = tableaux::prise_pose(&p, x, y, false)?;
        Some((p, pr))
    }

    /// 🎯 (tour 2, F3) No crosshair at (`x`, `y`) (chart coordinates): a table selected by the finger or held (mouse
    /// or finger), its menu under the mouse, or (PC) the mouse over a table or its edge (the move / resize cursor
    /// shows); outside the table the crosshair comes back.
    pub(crate) fn tab_sans_croix(&self, x: f64, y: f64) -> bool {
        let t = &self.pointer.tab;
        if t.choisi.is_some() || t.glisse.is_some() {
            return true;
        }
        if t.menu.as_ref().and_then(tableaux::menu_rect).is_some_and(|r| r.contains(x, y)) {
            return true;
        }
        !argos_render::actions::tactile() && self.tab_prise_sous(x, y).is_some()
    }

    /// 🖱 The cursor of a table's part: move on the body, the matching double arrow on a corner or a side.
    pub(crate) fn tab_curseur(p: Prise) -> CursorIcon {
        match p {
            Prise::Corps => CursorIcon::Move,
            Prise::Coin(0 | 2) => CursorIcon::NwseResize,
            Prise::Coin(_) => CursorIcon::NeswResize,
            Prise::Cote(1 | 3) => CursorIcon::EwResize,
            Prise::Cote(_) => CursorIcon::NsResize,
        }
    }

    /// 🖱 The mouse moved over the chart at (`x`, `y`): the table under it (shown hovered), the menu's line.
    pub(crate) fn tab_survol(&mut self, x: f64, y: f64) {
        let s = self.tab_prise_sous(x, y).map(|(p, pr)| (p.key, pr));
        if s != self.pointer.tab.survol {
            self.pointer.tab.survol = s;
        }
        if let Some(m) = self.pointer.tab.menu.as_mut() {
            m.survol = tableaux::menu_sous(m, x, y);
        }
    }

    /// The table `key`'s layout now.
    fn tab_layout(&self, key: &str) -> TableLayout {
        self.ind.tables.get(key).copied().unwrap_or_default().borne()
    }

    /// Starts dragging `p` taken by `prise` at (`x`, `y`).
    fn tab_prendre(&mut self, p: tableaux::Pose, prise: Prise, x: f64, y: f64) {
        self.pointer.tab.glisse = Some(Glisse {
            lay0: self.tab_layout(&p.key),
            key: p.key,
            prise,
            x0: x,
            y0: y,
            rect0: p.rect,
            area: p.area,
            bouge: false,
        });
    }

    /// The dragged table follows the mouse / the finger at (`x`, `y`) (chart coordinates).
    pub(crate) fn tab_glisse(&mut self, x: f64, y: f64) {
        let Some(g) = self.pointer.tab.glisse.as_mut() else { return };
        let (dx, dy) = (x - g.x0, y - g.y0);
        if !g.bouge && dx.hypot(dy) < 3.0 {
            return;
        }
        g.bouge = true;
        let lay = tableaux::glisser(g.lay0, g.rect0, g.area, g.prise, dx, dy);
        let (key, prise) = (g.key.clone(), g.prise);
        self.ind.tables.insert(key, lay);
        if !argos_render::actions::tactile() {
            self.set_cursor(Self::tab_curseur(prise));
        }
        self.redraw();
    }

    /// The dragged table let go: it snaps to the nearest free place (the drawing does it) and that place is kept.
    pub(crate) fn tab_lacher(&mut self) {
        let Some(g) = self.pointer.tab.glisse.take() else { return };
        if g.bouge {
            self.tab_fixer(&g.key);
        }
        self.redraw();
    }

    /// Keeps the table `key` where the drawing put it (snapped off the legend, the pills…) and saves the layouts.
    pub(crate) fn tab_fixer(&mut self, key: &str) {
        self.tab_poser();
        if let (Some(p), Some(lay)) = (tableaux::pose(key), self.ind.tables.get(key).copied()) {
            let n = TableLayout { scale: p.scale, ..lay }.ancre(p.rect, p.area, Some(lay.corner));
            self.ind.tables.insert(key.to_string(), n);
        }
        self.garder_reglages();
    }

    /// The tables placed now as the next drawing puts them (`tableaux::pose` gives their real rects).
    fn tab_poser(&self) {
        self.tableaux_pour_dessin();
        let _ = argos_render::render_fond(&self.to_chart());
    }

    /// Back in place: the table `key` takes its automatic place and size again, placed at once; hovered only if it
    /// is still under the mouse at (`x`, `y`), with the cursor of what is there now (tour 3, H8: it was drawn hovered
    /// away from the mouse).
    pub(crate) fn tab_remettre(&mut self, key: &str, (x, y): (f64, f64)) {
        self.ind.tables.remove(key);
        self.garder_reglages();
        self.tab_poser();
        self.tab_survol(x, y);
        let c = self.curseur_chart(x, y);
        self.poser_curseur(c);
        self.redraw();
    }

    /// « Masquer le tableau »: the indicator's own setting that shows its table is turned off.
    fn tab_masquer(&mut self, key: &str) {
        let Some(p) = tableaux::pose(key) else { return };
        let (Some(ind), Some(k)) = (p.ind, p.interrupteur) else { return };
        let mut vals = argos_render::indicators::values(&self.ind.set, ind);
        if let Some(n) = ind.params().1.iter().position(|q| q.key == k) {
            if let Some(v) = vals.get_mut(n) {
                *v = 0.0;
            }
        }
        self.ind.set.insert(ind, vals);
        self.ind.key = Default::default();
        self.garder_reglages();
        self.redraw();
    }

    /// 🖱 Right-click at (`x`, `y`) (chart coordinates): on a table, its menu (`true`).
    pub(crate) fn tab_menu_ouvrir(&mut self, x: f64, y: f64) -> bool {
        let Some((p, _)) = self.tab_prise_sous(x, y) else { return false };
        (self.ui.list, self.ui.pop) = (None, None);
        self.pointer.tab.menu = Some(Menu {
            key: p.key,
            x,
            y,
            survol: None,
        });
        self.redraw();
        true
    }

    /// 🖱 Left button (`pressed` or released) for the tables; `true`: they took it (nothing else sees it).
    pub(crate) fn tab_bouton(&mut self, pressed: bool) -> bool {
        let (wx, wy) = self.pointer.win_mouse;
        let (x, y) = (wx - self.rw(), wy - self.top_h());
        if !pressed {
            if self.pointer.tab.glisse.is_some() {
                self.tab_lacher();
                return true;
            }
            return std::mem::take(&mut self.pointer.tab.avale);
        }
        if let Some(m) = self.pointer.tab.menu.take() {
            match tableaux::menu_sous(&m, x, y) {
                Some(MenuLigne::Remettre) => self.tab_remettre(&m.key, (x, y)),
                Some(MenuLigne::Masquer) => self.tab_masquer(&m.key),
                _ if tableaux::menu_rect(&m).is_some_and(|r| r.contains(x, y)) => {}
                // (outside: the menu closes and the click goes on)
                _ => {
                    self.redraw();
                    return false;
                }
            }
            self.pointer.tab.avale = true;
            self.redraw();
            return true;
        }
        let Some((p, prise)) = self.tab_prise_sous(x, y) else { return false };
        // (B-048: by the finger, the time it tapped, not when the app read it)
        let now = self.pointer.t_geste.unwrap_or_else(Instant::now);
        if self.est_double(self.pointer.tab.dernier, now) {
            self.pointer.tab.dernier = None;
            self.pointer.tab.avale = true;
            self.tab_remettre(&p.key, (x, y));
            return true;
        }
        self.pointer.tab.dernier = Some(now);
        self.tab_prendre(p, prise, x, y);
        self.set_cursor(Self::tab_curseur(prise));
        true
    }

    /// 👆 A finger gesture for the tables (phone view); `true`: they took it. A tap on a table selects it; while one
    /// is selected every gesture is theirs (the chart neither pans nor zooms): drag it, pull a corner, pinch it, tap
    /// elsewhere or « OK » to finish. Nothing selected: the gestures go to the chart as before.
    pub(crate) fn tab_geste(&mut self, g: crate::input::touch::Gesture, suivant: Option<crate::input::touch::Gesture>) -> bool {
        use crate::input::touch::Gesture as G;
        if !argos_render::actions::tactile() {
            return false;
        }
        let (rw, th) = (self.rw(), self.top_h());
        // (a selected table no longer drawn: the selection ends)
        if self.pointer.tab.choisi.as_deref().is_some_and(|k| tableaux::pose(k).is_none()) {
            self.tab_fin_choix();
        }
        let choisi = self.pointer.tab.choisi.clone();
        match g {
            G::Press(wx, wy) => {
                let (x, y) = (wx - rw, wy - th);
                self.pointer.win_mouse = (wx, wy);
                let Some(key) = choisi else {
                    // a tap (pressed and released at once) on a table selects it
                    if matches!(suivant, Some(G::Release(..))) && !self.modale_ouverte() && !self.couche_haute(wx, wy) {
                        if let Some(p) = tableaux::pose_sous(x, y, 0.0) {
                            self.pointer.tab.choisi = Some(p.key);
                            self.pointer.tab.avale = true;
                            // (F3: no crosshair laid through the selected table, now or after « OK »)
                            (self.ui.croix, self.pointer.mouse) = (None, None);
                            self.redraw();
                            return true;
                        }
                    }
                    return false;
                };
                if tableaux::ok_cible().contains(x, y) {
                    self.tab_fin_choix();
                    self.pointer.tab.avale = true;
                    // (tour 3, H9) « OK » lays no crosshair where it was
                    (self.ui.croix, self.pointer.mouse) = (None, None);
                    return true;
                }
                if let Some(p) = tableaux::pose(&key) {
                    if let Some(pr) = tableaux::prise(p.rect, x, y, true) {
                        self.tab_prendre(p, pr, x, y);
                        return true;
                    }
                }
                self.pointer.tab.hors = Some((wx, wy));
                true
            }
            G::Move(wx, wy) => {
                if self.pointer.tab.glisse.is_some() {
                    self.tab_glisse(wx - rw, wy - th);
                    return true;
                }
                choisi.is_some() && self.pointer.tab.hors.is_some()
            }
            G::Release(wx, wy) => {
                if self.pointer.tab.glisse.is_some() {
                    self.tab_lacher();
                    return true;
                }
                if let Some((x0, y0)) = self.pointer.tab.hors.take() {
                    if (wx - x0).hypot(wy - y0) < 8.0 {
                        self.tab_fin_choix();
                    }
                    return true;
                }
                std::mem::take(&mut self.pointer.tab.avale)
            }
            G::Pinch { cx, cy, sx, sy } => {
                let Some(key) = choisi else { return false };
                let (x, y) = (cx - rw, cy - th);
                if let Some(p) = tableaux::pose(&key) {
                    let m = tableaux::CIBLE_DOIGT;
                    let sur = Rect::new(p.rect.x - m, p.rect.y - m, p.rect.w + 2.0 * m, p.rect.h + 2.0 * m).contains(x, y);
                    let k = if (sx - 1.0).abs() > (sy - 1.0).abs() { sx } else { sy };
                    if sur && (k - 1.0).abs() > 1e-6 {
                        let lay = tableaux::pincer(self.tab_layout(&key), p.rect, p.area, k);
                        self.ind.tables.insert(key, lay);
                        self.pointer.tab.a_garder = true;
                        self.redraw();
                    }
                }
                true
            }
            G::RightClick(..) | G::Hold(..) | G::Swipe { .. } => choisi.is_some(),
        }
    }

    /// 👆 The selection ends (tap elsewhere, « OK »): a pinched table is kept where it is.
    pub(crate) fn tab_fin_choix(&mut self) {
        if let Some(k) = self.pointer.tab.choisi.take() {
            if std::mem::take(&mut self.pointer.tab.a_garder) {
                self.tab_fixer(&k);
            }
        }
        self.pointer.tab.hors = None;
        self.redraw();
    }

    /// The indicator `i` is removed: its tables' layouts are forgotten.
    pub(crate) fn tab_oublier_ind(&mut self, i: argos_render::indicators::Ind) {
        let n = self.ind.tables.len();
        self.ind.tables.retain(|k, _| !tableaux::est_de(k, i));
        if self.ind.tables.len() != n {
            self.garder_reglages();
        }
    }

    /// The user script `id` is removed: its tables' layouts are forgotten.
    pub(crate) fn tab_oublier_script(&mut self, id: &str) {
        let n = self.ind.tables.len();
        self.ind.tables.retain(|k, _| !tableaux::est_du_script(k, id));
        if self.ind.tables.len() != n {
            self.garder_reglages();
        }
    }
}
