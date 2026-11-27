//! 🖱️ Souris (et doigt, déjà converti en souris) : déplacement, clics, molette.

use crate::*;

impl App {
    pub(crate) fn mouse_moved(&mut self, position: winit::dpi::PhysicalPosition<f64>) {
        let (wx, wy) = (position.x, position.y);
        self.pointer.win_mouse = (wx, wy);
        // 🎨 the palette's opacity slider held: it follows the mouse
        if self.colorpick_drag(wx) {
            return;
        }
        // 🎚 the Trading ticket's % slider held: it follows the mouse / the finger, even outside the panel
        if let Some(x0) = self.pointer.curseur_ticket {
            self.watch.glisse_ticket(wx - x0);
            return self.redraw();
        }
        // 🎡 le doigt glisse sur la pilule du cell : la roue suit, rien d'autre (ni la chart, ni les menus)
        if self.ui.roue.is_some() {
            return self.roue_bouger(wy);
        }
        // 🖥 geste dans la colonne de modules (poignée, titre, barre, bord) : la souris est à lui, même dehors
        if self.dock.geste.is_some() {
            self.dock_bouge(wx, wy);
            return self.redraw();
        }
        // 🛟 ✎ Circle: the ellipse follows the mouse; outside the Support panel (modal layer), the chart doesn't see it
        if let Some(g) = self.ui.support.as_mut() {
            if g.trace_en_cours() {
                g.glisse(wx, wy);
                return self.redraw();
            }
            if wy >= crate::render::VOLET_HAUT && !g.rect().contains(wx, wy) && !self.sur_argos_ai() {
                (self.pointer.mouse, self.pointer.side_mouse, self.pointer.rail_mouse, self.pointer.dock_mouse) = (None, None, None, None);
                self.set_cursor(CursorIcon::Default);
                return self.redraw();
            }
        }
        // 🖥 volet flottant : la souris est à lui (ses coordonnées : depuis son coin haut gauche), pas à la barre du haut
        let libre = self.pointer.drag.is_none() && self.pointer.grab.is_none() && self.pointer.wl_resize.is_none();
        if let Some(v) = self.volet_rect().filter(|v| libre && v.contains(wx, wy)) {
            let (sx, sy) = (wx - v.x, wy - v.y);
            self.pointer.side_mouse = Some((sx, sy));
            (self.pointer.mouse, self.pointer.rail_mouse, self.pointer.top_mouse, self.pointer.dock_mouse) = (None, None, None, None);
            let edge = self.watch.edge_rect(v.h).is_some_and(|r| r.contains(sx, sy));
            self.set_cursor(if edge { CursorIcon::ColResize } else { CursorIcon::Default });
            return self.redraw();
        }
        // 📊 fenêtre de la barre (Type…) ouverte par-dessus les rangées du haut : la souris est à elle (son survol)
        if let Some((cx, cy)) = self.sur_pop_du_haut(wx, wy) {
            (self.pointer.mouse, self.pointer.top_mouse, self.pointer.side_mouse, self.pointer.rail_mouse) = (Some((cx, cy)), None, None, None);
            self.set_cursor(CursorIcon::Default);
            return self.redraw();
        }
        // tout ce qui est sous la barre du haut a ses coordonnées depuis le bas de cette barre
        let y = wy - self.top_h();
        (self.pointer.top_mouse, self.pointer.phone_mouse) = (None, None);
        // coordonnées de la chart (elle commence après la barre de gauche)
        let x = wx - self.rw();
        // 🖥 panneau d'indicateur sous la souris : ses boutons se montrent dans sa légende (maquette, au survol)
        let vp = &self.chart.viewport;
        let dans = x >= 0.0 && x <= vp.plot_w() && y >= 0.0 && y <= vp.plot_h();
        argos_render::indicators::set_survol(if dans {
            self.ind
                .out
                .bands(vp.plot_h())
                .into_iter()
                .find(|(i, top)| y >= *top && y < top + i.band_h())
                .map(|(i, _)| i)
        } else {
            None
        });
        // 🪟 glisser dans une case du split : seulement ses bougies suivent ; sinon, son réticule suit la souris
        if let Some((k, last)) = self.pointer.pane_drag {
            let cells = argos_render::splits::cells(&self.split.layout);
            if let Some(c) = cells.get(k + 1) {
                let (w, h) = ((c[2] * self.area.0 as f64).round(), (c[3] * self.area.1 as f64).round());
                self.pointer.pane_drag = Some((k, x));
                return self.pane_pan(k, x - last, w, h);
            }
        }
        if !self.panes.is_empty() {
            self.pane_hover(x, y);
        }
        // 💼 ligne d'ordre attrapée (entrée, SL, TP, brouillon) : son prix suit
        if self.pointer.order_drag.is_some() {
            if let Some(sc) = self.scale_now() {
                self.ordre_glisse(sc.price_of(y));
            }
            self.set_cursor(CursorIcon::NsResize);
            return self.redraw();
        }
        // 🔔 ligne d'alerte attrapée : son prix suit la souris / le doigt
        if let Some(id) = self.pointer.alert_drag {
            // (a click that trembles does not move it: 5 points first)
            if self.pointer.alert_appui.is_some_and(|(x0, y0)| (wx - x0).hypot(wy - y0) < 5.0) {
                return;
            }
            self.pointer.alert_appui = None;
            if let Some(sc) = self.scale_now() {
                self.alerts.move_line(id, sc.price_of(y));
            }
            self.set_cursor(CursorIcon::NsResize);
            return self.redraw();
        }
        // 📋 a table held (its body, a corner or a side): it follows the mouse / the finger
        if self.pointer.tab.glisse.is_some() {
            self.pointer.mouse = Some((x, y));
            return self.tab_glisse(x, y);
        }
        // 🎓 doigt (ou souris enfoncée) qui glisse sur l'Académie : la page suit
        if let (Some((y0, last, moved)), Some(p)) = (self.pointer.mk_drag.as_mut(), self.ui.ac_page.as_mut()) {
            *moved |= (wy - *y0).abs() > 8.0;
            if *moved {
                let size = self.window.as_deref().map(ui_size).unwrap_or_default();
                p.wheel(*last - wy, &argos_render::text::Text::default(), size.width as f64, size.height as f64);
            }
            *last = wy;
            return self.redraw();
        }
        // ⌂ 🔎 Accueil ou recherche ouverts : la souris est à eux (rien dessous ne réagit) ; le doigt qui glisse sur
        // l'Accueil le fait défiler
        if self.ui.accueil.is_some() || self.ui.palette.is_some() {
            let cartes = argos_render::accueil::cartes(&self.accueil_donnees());
            let size = self.window.as_deref().map(ui_size).unwrap_or_default();
            if let (Some((y0, last, moved)), Some(p), None) = (self.pointer.mk_drag.as_mut(), self.ui.accueil.as_mut(), &self.ui.palette) {
                *moved |= (wy - *y0).abs() > 8.0;
                if *moved {
                    p.wheel(*last - wy, &argos_render::text::Text::default(), size.width as f64, size.height as f64, &cartes);
                }
                *last = wy;
            }
            (self.pointer.mouse, self.pointer.side_mouse, self.pointer.rail_mouse, self.pointer.top_mouse) = (None, None, None, None);
            self.set_cursor(CursorIcon::Default);
            return self.redraw();
        }
        // 🛒 doigt (ou souris enfoncée) qui glisse sur le Marketplace : la grille suit, sans sauter
        if let (Some((y0, last, moved)), Some(m)) = (self.pointer.mk_drag.as_mut(), self.ui.mk_page.as_mut()) {
            *moved |= (wy - *y0).abs() > 8.0;
            if *moved {
                let size = self.window.as_deref().map(ui_size).unwrap_or_default();
                m.wheel(*last - wy, size.width as f64, size.height as f64, &self.ind.on);
            }
            *last = wy;
            return self.redraw();
        }
        // fenêtre de réglages attrapée par son titre : elle suit la souris (le titre reste dans la fenêtre de l'app)
        if let Some((dx, dy)) = self.pointer.dlg_drag {
            let (ww, wh) = (self.win_w(), self.area.1 as f64 + self.top_h());
            let place = |r: &mut Rect| {
                r.x = (wx - dx).clamp(60.0 - r.w, ww - 60.0);
                r.y = (wy - dy).clamp(0.0, wh - 40.0);
            };
            if let Some(d) = self.ui.ind_dlg.as_mut() {
                place(&mut d.rect);
            } else if let Some(d) = self.ui.obj_dlg.as_mut() {
                place(&mut d.rect);
            }
            return self.redraw();
        }
        // bord de la watchlist attrapé : la largeur suit la souris (vers la gauche = plus large)
        if let Some((x0, w0)) = self.pointer.wl_resize {
            self.watch.set_width(w0 + (x0 - wx));
            return self.redraw();
        }
        // bouton de la barre flottante appuyé : clic maintenu = déplacer la barre, glisser un ⭐ = changer l'ordre
        if let Some(mut bp) = self.pointer.bar_press {
            let text = argos_render::text::Text::default();
            let along = if self.tools.vertical { y - bp.at.1 } else { x - bp.at.0 };
            let dist = (x - bp.at.0).hypot(y - bp.at.1);
            if bp.mode == PressMode::Wait {
                let across = (dist * dist - along * along).max(0.0).sqrt();
                let fav = matches!(bp.btn, Btn::Fav(_));
                // (B-048: by the finger, its own time — a move read late after a slow image is no held press)
                bp.mode = if self.heure_geste().duration_since(bp.since).as_millis() >= 380 {
                    PressMode::Lift(bp.at.0 - self.tools.x, bp.at.1 - self.tools.y)
                } else if fav && along.abs() >= 14.0 {
                    PressMode::Sort
                } else if (fav && across > 8.0) || (!fav && dist > 8.0) {
                    PressMode::Cancel
                } else {
                    PressMode::Wait
                };
            }
            match (bp.mode, bp.btn) {
                (PressMode::Lift(dx, dy), _) => {
                    (self.tools.x, self.tools.y) = (x - dx, y - dy);
                    toolbar::clamp(&mut self.tools, &text, self.chart.viewport.plot_w(), self.chart.viewport.plot_h());
                    self.ui.pop = None;
                }
                (PressMode::Sort, Btn::Fav(t)) => {
                    if let Some(over) = toolbar::fav_at(&self.tools, &text, x, y).filter(|o| *o != t) {
                        if let (Some(from), Some(to)) = (self.rail.favs.iter().position(|f| f == t), self.rail.favs.iter().position(|f| f == over)) {
                            let id = self.rail.favs.remove(from);
                            self.rail.favs.insert(to, id);
                            self.sync_toolbar();
                        }
                    }
                }
                _ => {}
            }
            self.pointer.bar_press = Some(bp);
            self.pointer.mouse = Some((x, y));
            return self.redraw();
        }
        // barre flottante attrapée par sa poignée : elle suit la souris
        if let Some((dx, dy)) = self.pointer.tool_drag {
            let to = (x - dx, y - dy);
            self.pointer.tool_moved |= (to.0 - self.tools.x).abs() + (to.1 - self.tools.y).abs() > 3.0;
            if !self.pointer.tool_moved {
                return;
            }
            (self.tools.x, self.tools.y) = to;
            let text = argos_render::text::Text::default();
            toolbar::clamp(&mut self.tools, &text, self.chart.viewport.plot_w(), self.chart.viewport.plot_h());
            return self.redraw();
        }
        // menu en liste ouvert : il suit la souris (survol), rien d'autre
        if self.ui.list.as_ref().is_some_and(|(_, m)| m.rect.contains(wx, wy)) {
            self.pointer.mouse = None;
            self.set_cursor(CursorIcon::Default);
            return self.redraw();
        }
        // ↕️ bouton de la barre d'icônes attrapé : il suit la souris (au-delà de 10 points : c'est un déplacement)
        if let Some((i, y0, moved, prev)) = self.pointer.icon_drag {
            if moved || (y - y0).abs() > 10.0 {
                if !moved {
                    // le clic d'appui avait ouvert / fermé un panneau : on remet comme avant, on déplace seulement
                    (self.watch.prefs.open, self.watch.prefs.panel) = prev;
                }
                let to = self.watch.tool_slot(y);
                if to != i {
                    self.watch.move_tool(i, to);
                }
                self.pointer.icon_drag = Some((to, y0, true, prev));
                self.set_cursor(CursorIcon::Grabbing);
                return self.redraw();
            }
        }
        // barre de défilement de la watchlist attrapée : elle suit la souris, même hors du panneau
        if self.watch.dragging() {
            self.watch.drag_to(y, self.area.1 as f64);
            return self.redraw();
        }
        // fenêtre ouverte (Panneaux, Compte) : la souris est à elle
        let free = self.pointer.tool_drag.is_none() && self.pointer.drag.is_none() && self.pointer.grab.is_none() && self.pointer.bar_press.is_none();
        if free
            && ((self.ui.ai_open && self.ai_rect().contains(wx, wy))
                || self.ui.sheet.as_ref().is_some_and(|s| s.rect.contains(wx, wy))
                || self.ui.argos.as_ref().is_some_and(|m| m.rect.contains(wx, wy))
                || self.ui.obj_tree.as_ref().is_some_and(|t| t.rect.contains(wx, wy))
                || self.ui.split_menu.as_ref().is_some_and(|m| m.rect.contains(wx, wy)))
        {
            if let Some(m) = self.ui.split_menu.as_mut() {
                m.hover(wx, wy);
            }
            (self.pointer.mouse, self.pointer.side_mouse, self.pointer.rail_mouse, self.pointer.top_mouse) = (None, None, None, None);
            self.set_cursor(CursorIcon::Default);
            return self.redraw();
        }
        // 📐 cell : les pilules de la maquette (rangée du haut, DÉMO, Flux, symbole · unité, ƒx, barre d'icônes) et ce
        // qui est en dehors de la chart (haut et bas de l'écran)
        let libre = self.pointer.tool_drag.is_none() && self.pointer.drag.is_none() && self.pointer.grab.is_none();
        if self.top.phone && libre && self.ui.accueil.is_none() {
            let e = self.cell_etat();
            let dans_cell = argos_render::ecrans::hit(self.win_w(), self.cell_h(), &e, wx, wy).is_some();
            if dans_cell || y < 0.0 || y >= self.area.1 as f64 {
                (
                    self.pointer.phone_mouse,
                    self.pointer.top_mouse,
                    self.pointer.mouse,
                    self.pointer.side_mouse,
                    self.pointer.rail_mouse,
                ) = (Some((wx, wy)), None, None, None, None);
                self.set_cursor(CursorIcon::Default);
                return self.redraw();
            }
        }
        // 🖥 survol des bandes (maquette) : le rail de gauche et la colonne repliée s'ouvrent par-dessus la chart
        if libre && !self.top.phone {
            // (une barre ouverte au survol reste ouverte tant que le menu qu'elle a ouvert est là)
            let garder = self.menu_ouvert();
            // 🧘 Zen: the hidden top bar comes back at the top edge (the same way as the folded rows)
            if let Some(r) = self.zen_haut_zone() {
                let dedans = r.contains(wx, wy);
                if dedans || !garder {
                    self.ui.zen_haut = dedans;
                }
                if dedans {
                    (self.pointer.top_mouse, self.pointer.mouse, self.pointer.side_mouse, self.pointer.rail_mouse) = (Some((wx, wy)), None, None, None);
                    self.set_cursor(CursorIcon::Default);
                    return self.redraw();
                }
            }
            // 🖥 rangées du haut repliées : leur bande, puis toute la barre du haut une fois ouverte
            if let Some(r) = self.haut_survol_zone() {
                let dedans = r.contains(wx, wy);
                if dedans || !garder {
                    self.top.survol_bandeaux = dedans;
                }
                if dedans {
                    (self.pointer.top_mouse, self.pointer.mouse, self.pointer.side_mouse, self.pointer.rail_mouse) = (Some((wx, wy)), None, None, None);
                    self.set_cursor(CursorIcon::Default);
                    return self.redraw();
                }
            }
            if let Some(r) = self.rail_survol_zone() {
                let dedans = r.contains(wx, wy);
                if (dedans || !garder) && !self.ui.rail_garde {
                    self.ui.rail_survol = dedans;
                }
                if dedans {
                    (self.pointer.rail_mouse, self.pointer.mouse, self.pointer.side_mouse, self.pointer.top_mouse) =
                        (Some((wx - crate::actions::RAIL_X, wy - self.rail_y0())), None, None, None);
                    self.set_cursor(CursorIcon::Default);
                    return self.redraw();
                }
            }
            if let Some(r) = self.col_survol_zone() {
                let dedans = r.contains(wx, wy);
                if dedans || !garder {
                    self.watch.col_survol = dedans;
                }
                if dedans {
                    (self.pointer.side_mouse, self.pointer.mouse, self.pointer.rail_mouse, self.pointer.top_mouse) =
                        (Some((wx - self.side_x(), wy - self.top_h())), None, None, None);
                    self.set_cursor(CursorIcon::Default);
                    return self.redraw();
                }
            }
        }
        // 🖥 colonne d'icônes PC : elle monte à côté de la barre du haut, la souris est à elle (y < 0 dans la watchlist)
        if y < 0.0 && self.watch.colonne.is_some() && wx >= self.top_w() && self.pointer.drag.is_none() && self.pointer.grab.is_none() {
            self.pointer.side_mouse = Some((wx - self.side_x(), y));
            (self.pointer.mouse, self.pointer.rail_mouse, self.pointer.top_mouse) = (None, None, None);
            self.set_cursor(CursorIcon::Default);
            return self.redraw();
        }
        // barre du haut
        if y < 0.0 && self.pointer.tool_drag.is_none() && self.pointer.drag.is_none() && self.pointer.grab.is_none() {
            (self.pointer.top_mouse, self.pointer.mouse, self.pointer.side_mouse, self.pointer.rail_mouse) = (Some((wx, wy)), None, None, None);
            self.set_cursor(CursorIcon::Default);
            return self.redraw();
        }

        // 🧩 colonne de modules : la souris est à elle
        let busy = self.pointer.drag.is_some()
            || self.pointer.grab.is_some()
            || self.pointer.band_drag.is_some()
            || self.pointer.axis_drag.is_some()
            || self.pointer.band_pan.is_some()
            || self.pointer.band_zoom.is_some()
            || self.pointer.script_pan.is_some()
            || self.pointer.script_zoom.is_some();
        self.pointer.dock_mouse = None;
        if let Some((dx, dy)) = self.dock_zone(wx, wy).filter(|_| !busy) {
            self.pointer.dock_mouse = Some((dx, dy));
            (self.pointer.mouse, self.pointer.side_mouse, self.pointer.rail_mouse) = (None, None, None);
            self.set_cursor(self.dock_curseur(dx, dy));
            return self.redraw();
        }
        // 📐 deux segments l'un au-dessus de l'autre : la chart en haut, le panneau en bas, rien sur la charnière
        if let Some((top, bot)) = self.ui.rows.filter(|_| self.top.phone) {
            if y >= bot && self.watch.prefs.open {
                self.pointer.side_mouse = Some((wx, y - bot));
                (self.pointer.mouse, self.pointer.rail_mouse) = (None, None);
                self.set_cursor(CursorIcon::Default);
                return self.redraw();
            }
            if y >= top {
                (self.pointer.mouse, self.pointer.side_mouse, self.pointer.rail_mouse) = (None, None, None);
                return self.redraw();
            }
        }
        // (deux segments : au-dessus de la charnière, c'est la chart, même panneau ouvert)
        let rows_chart = self.top.phone && self.ui.rows.is_some();
        // watchlist / barre d'icônes / menu ⠿ : la souris est à eux, pas à la chart
        if !rows_chart && self.on_side(wx) {
            let sx = wx - self.side_x();
            // (volet flottant : sous lui, la bande du bas n'est à personne)
            if self.volet_rect().is_some() && sx < self.watch.pw() {
                (self.pointer.mouse, self.pointer.side_mouse, self.pointer.rail_mouse) = (None, None, None);
                return self.redraw();
            }
            self.pointer.side_mouse = Some((sx, y));
            (self.pointer.mouse, self.pointer.rail_mouse) = (None, None);
            let edge = self.watch.edge_rect(self.area.1 as f64).is_some_and(|r| r.contains(sx, y));
            self.set_cursor(if edge { CursorIcon::ColResize } else { CursorIcon::Default });
            return self.redraw();
        }
        // barre de gauche
        if wx < self.rw() {
            (self.pointer.rail_mouse, self.pointer.mouse, self.pointer.side_mouse) = (Some((wx, y)), None, None);
            self.set_cursor(CursorIcon::Default);
            return self.redraw();
        }
        (self.pointer.side_mouse, self.pointer.rail_mouse) = (None, None);
        self.pointer.mouse = Some((x, y));
        // 🖍 Démonstration : le trait en cours suit la souris
        if let Some(t) = self.ui.demo_traits.last_mut().filter(|t| t.fini.is_none()) {
            t.pts.push((x, y));
            return self.redraw();
        }
        if self.ui.context.is_open() {
            let ctx = self.ctx();
            let mut cm = self.ui.context;
            cm.on_move(&self.chart.viewport, &ctx, x, y);
            self.ui.context = cm;
            self.set_cursor(CursorIcon::Default);
            return self.redraw();
        }
        // objet en cours de placement : son dernier point suit la souris (aperçu)
        if self.draft.is_some() {
            if let Some(at) = self.frame_now().map(|f| f.snap(x, y, self.rail.magnet)) {
                if let Some(q) = self.draft.as_mut().and_then(|d| d.pts.last_mut()) {
                    *q = at;
                }
            }
        }
        // 📍 repère d'input attrapé (prix / heure d'un indicateur) : la valeur suit la souris, recalcul tout de suite
        if let (Some(m), Some(f)) = (self.pointer.mark_drag, self.frame_now()) {
            let mut vals = argos_render::indicators::values(&self.ind.set, m.ind);
            for (k, v) in argos_render::inmark::drag(&m, &f.geo(), x, y) {
                vals[k] = v;
            }
            self.ind.set.insert(m.ind, vals);
            self.ind.key = Default::default();
            self.pointer.mouse = Some((x, y));
            return self.redraw();
        }
        // ↕️ bande glissée de l'intérieur : elle monte / descend dans sa case, le temps suit la souris
        if let Some((ind, lx, y0, off0)) = self.pointer.band_pan {
            self.chart.viewport.scroll_px(x - lx, &self.chart.slots);
            let z = self.chart.settings.band_view.get(&ind).map_or(1.0, |v| v.1);
            let h = (ind.band_h() - 26.0).max(10.0);
            self.chart.settings.band_view.insert(ind, (off0 + (y - y0) / h * z, z));
            self.pointer.band_pan = Some((ind, x, y0, off0));
            return self.redraw();
        }
        // colonne de valeurs d'une bande glissée : vers le bas = échelle plus grande (courbe écrasée)
        if let Some((ind, y0, z0)) = self.pointer.band_zoom {
            let off = self.chart.settings.band_view.get(&ind).map_or(0.0, |v| v.0);
            self.chart
                .settings
                .band_view
                .insert(ind, (off, (z0 * ((y - y0) * 0.005).exp()).clamp(0.05, 20.0)));
            self.set_cursor(CursorIcon::RowResize);
            return self.redraw();
        }
        // 📜 the same in a script's pane: dragged inside it moves up / down (time follows), its value column zooms
        if let Some((key, lx, y0, off0)) = self.pointer.script_pan.clone() {
            self.chart.viewport.scroll_px(x - lx, &self.chart.slots);
            let z = self.chart.settings.script_view.get(&key).map_or(1.0, |v| v.1);
            let h = self
                .ind
                .out
                .scripts
                .iter()
                .find(|s| s.view_key() == key)
                .map_or(10.0, |s| (s.band_h() - 26.0).max(10.0));
            self.chart.settings.script_view.insert(key.clone(), (off0 + (y - y0) / h * z, z));
            self.pointer.script_pan = Some((key, x, y0, off0));
            return self.redraw();
        }
        if let Some((key, y0, z0)) = self.pointer.script_zoom.clone() {
            let off = self.chart.settings.script_view.get(&key).map_or(0.0, |v| v.0);
            self.chart
                .settings
                .script_view
                .insert(key, (off, (z0 * ((y - y0) * 0.005).exp()).clamp(0.05, 20.0)));
            self.set_cursor(CursorIcon::RowResize);
            return self.redraw();
        }
        // ↕️ bord de bande attrapé : vers le haut = bande plus haute
        if self.drawbar_drag_to(x, y) {
            return;
        }
        if self.script_bord_glisse(y) {
            return;
        }
        if let Some((ind, y0, h0)) = self.pointer.band_drag {
            let max = argos_render::indicators::BAND_MAX.min(self.chart.viewport.plot_h() * 0.8);
            self.chart
                .settings
                .bands
                .insert(ind, (h0 + y0 - y).clamp(argos_render::indicators::BAND_MIN, max));
            self.set_cursor(CursorIcon::RowResize);
            return self.redraw();
        }
        // objet attrapé : il suit la souris (même code pour tous les objets)
        if let (Some(g), Some(f)) = (self.pointer.grab.clone(), self.frame_now()) {
            match g {
                Grab::Move { i, x0, y0, from } => {
                    if let Some(d) = self.chart.drawings.get_mut(i) {
                        d.move_from(&from, x - x0, y - y0, &f.geo());
                    }
                }
                Grab::Handle { i, h } => {
                    let at = f.snap(x, y, self.rail.magnet);
                    if let Some(d) = self.chart.drawings.get_mut(i) {
                        d.move_point(h, at);
                    }
                }
            }
        }
        // 🔔 the alert line under the mouse: its label (like TradingView)
        self.survol_alerte();
        // 📋 the table under the mouse: its border brighter; its menu's line
        self.tab_survol(x, y);
        let c = self.curseur_chart(x, y);
        self.poser_curseur(c);
        if let Some((lx, ly)) = self.pointer.drag {
            self.chart.viewport.scroll_px(x - lx, &self.chart.slots);
            // mode libre (Auto décoché) : la chart suit aussi la souris en haut / bas ;
            // mode normal : seulement dans le temps, le prix s'ajuste tout seul
            if let Some(s) = self.scale_now().filter(|_| y != ly && !self.chart.settings.scale.auto()) {
                let (lo, hi) = self.chart.settings.scale.manual.unwrap_or((s.min, s.max));
                let d = (y - ly) / s.height.max(1.0) * (hi - lo) * if s.invert { -1.0 } else { 1.0 };
                self.chart.settings.scale.manual = Some((lo + d, hi + d));
            }
            self.pointer.drag = Some((x, y));
        }
        if let Some(last) = self.pointer.axis_drag {
            // vers le bas = écart de prix plus grand = bougies écrasées (comme TradingView)
            if let Some(s) = self.scale_now() {
                let k = ((y - last) * 0.005).exp();
                let (mid, half) = ((s.min + s.max) / 2.0, (s.max - s.min) / 2.0 * k);
                self.chart.settings.scale.manual = Some((mid - half, mid + half));
                if let Some(r) = self.chart.settings.scale.lock_ratio {
                    self.chart.settings.scale.lock_ratio = Some(r * k);
                }
            }
            self.pointer.axis_drag = Some(y);
        }
        self.redraw();
    }

    /// 🖱 Le curseur sur la chart, d'après ce qui est sous la souris (`x`, `y` : coordonnées de la chart), avec les
    /// mêmes tests que le clic ; comme TradingView : main fermée seulement en glissant (clic tenu), double flèche
    /// verticale sur une ligne d'alerte ou d'ordre (SL / TP), main qui pointe sur un dessin ou un repère cliquable,
    /// déplacement sur une poignée, ↕ sur le bord d'un panneau, et sinon le curseur choisi (Croix, Point, Flèche),
    /// même en main libre.
    pub(crate) fn curseur_chart(&self, x: f64, y: f64) -> crate::types::Curseur {
        use crate::types::Curseur::Icone;
        if self.pointer.grab.is_some() || self.pointer.drag.is_some() {
            return Icone(CursorIcon::Grabbing);
        }
        if self.pointer.alert_drag.is_some() || self.pointer.order_drag.is_some() {
            return Icone(CursorIcon::NsResize);
        }
        // 🖍 the drawing bar: the hand on its buttons, the open hand on its handle, the arrow on the pill
        if self.pointer.drawbar_drag.is_some() {
            return Icone(CursorIcon::Grabbing);
        }
        if let Some((_, bar)) = self.drawbar().filter(|(_, b)| b.rect.contains(x, y) && self.ui.list.is_none()) {
            return Icone(match bar.hit(x, y) {
                Some(argos_render::drawbar::DbBtn::Handle) => CursorIcon::Grab,
                Some(_) => CursorIcon::Pointer,
                None => CursorIcon::Default,
            });
        }
        // ✨ la case du coin d'Argos AI (toute la case se clique) : la main qui pointe
        if !self.modale_ouverte() && self.ai_bouton_rect().contains(x + self.rw(), y + self.top_h()) {
            return Icone(CursorIcon::Pointer);
        }
        // 📅 the month report (nothing over it): the hand on its buttons, otherwise the arrow
        if let Some(main) = self.month_hand(x + self.rw(), y + self.top_h()) {
            return Icone(if main { CursorIcon::Pointer } else { CursorIcon::Default });
        }
        // 🏦 le ⚙ du Footprint : la main sur ce qui se clique, sinon la flèche
        if let Some(main) = self.fp_main(x + self.rw(), y + self.top_h()) {
            return Icone(if main { CursorIcon::Pointer } else { CursorIcon::Default });
        }
        // 🤖 la carte Argos AI d'un trade (rien au-dessus d'elle) : la main sur ✕ et sur la leçon, sinon la flèche
        if let Some(main) = self.avis_main(x + self.rw(), y + self.top_h()) {
            return Icone(if main { CursorIcon::Pointer } else { CursorIcon::Default });
        }
        // 📋 a table: move on its body, the double arrows on its corners and edges; its menu: the hand
        if let Some(g) = &self.pointer.tab.glisse {
            return Icone(Self::tab_curseur(g.prise));
        }
        if let Some(m) = &self.pointer.tab.menu {
            if argos_render::tableaux::menu_rect(m).is_some_and(|r| r.contains(x, y)) {
                return Icone(if argos_render::tableaux::menu_sous(m, x, y).is_some() {
                    CursorIcon::Pointer
                } else {
                    CursorIcon::Default
                });
            }
        }
        if let Some((_, p)) = self.tab_prise_sous(x, y) {
            return Icone(Self::tab_curseur(p));
        }
        // 🧱 une fenêtre, un menu ou un bouton au-dessus : son curseur, jamais celui de ce qui est dessous ; 🏷️ sur les
        // légendes et les tableaux : la main sur un bouton, sinon la flèche (jamais la croix)
        if self.couche_au_dessus(x + self.rw(), y + self.top_h()) {
            if !self.couche_haute(x + self.rw(), y + self.top_h()) && (self.legende_bouton(x, y).is_some() || self.pad_bouton(x, y).is_some()) {
                return Icone(CursorIcon::Pointer);
            }
            return Icone(CursorIcon::Default);
        }
        let dans = x <= self.chart.viewport.plot_w() && y <= self.chart.viewport.plot_h();
        let over = self.drawing_at(x, y).is_some();
        // (outil de dessin : la croix ; gomme : la main qui pointe sur ce qu'elle effacerait)
        if self.tool == "erase" {
            return Icone(if over { CursorIcon::Pointer } else { CursorIcon::Crosshair });
        }
        if self.tool != "cursor" {
            return Icone(CursorIcon::Crosshair);
        }
        if x <= self.chart.viewport.plot_w() && (self.ind.out.band_edge(self.chart.viewport.plot_h(), y).is_some() || self.script_bord(y)) {
            return Icone(CursorIcon::RowResize);
        }
        if self.handle_at(x, y).is_some() {
            return Icone(CursorIcon::Move);
        }
        if dans && (self.alert_at().is_some() || self.ordre_prise().is_some()) {
            return Icone(CursorIcon::NsResize);
        }
        let on_mark = self
            .frame_now()
            .is_some_and(|f| argos_render::inmark::hit(&self.ind.out, &f.geo(), x, y).is_some());
        if on_mark || over {
            return Icone(CursorIcon::Pointer);
        }
        if bottombar::hit(&self.chart.viewport, self.pointer.mouse, self.chart_demo().as_deref()).is_some() || self.pied_hit().is_some() {
            return Icone(CursorIcon::Pointer);
        }
        if dans {
            self.ui.curseur.curseur()
        } else {
            Icone(CursorIcon::Default)
        }
    }

    /// 🖱 Fin d'un glisser lâché ailleurs que sur la chart (ou fenêtre quittée) : l'objet tenu reste où il est,
    /// la chart arrête de suivre, le curseur redevient normal.
    pub(crate) fn fin_du_glisser(&mut self) {
        if let Some(Grab::Move { i, .. } | Grab::Handle { i, .. }) = self.pointer.grab.take() {
            if self.chart.drawings.get(i).is_some_and(|d| argos_render::position::is_position(&d.tool)) {
                self.ticket_from_drawing(i);
            }
        }
        (self.pointer.drag, self.pointer.axis_drag) = (None, None);
        self.set_cursor(CursorIcon::Default);
    }

    /// 📊 La fenêtre de la barre (Type…) ouverte sous un bouton du haut couvre-t-elle ce point de la fenêtre, au-dessus
    /// de la chart (sur les rangées du haut) ? Ses coordonnées de chart, si oui.
    pub(crate) fn sur_pop_du_haut(&self, wx: f64, wy: f64) -> Option<(f64, f64)> {
        let (cx, cy) = (wx - self.rw(), wy - self.top_h());
        self.ui.pop.as_ref().filter(|p| cy < 0.0 && p.rect.contains(cx, cy)).map(|_| (cx, cy))
    }

    /// 🧱 Une fenêtre modale est ouverte (réglages d'un indicateur ou d'un dessin, boîte de nom, de clé API, des 24
    /// mots) : comme TradingView, la chart ne reçoit alors ni survol, ni clic, ni molette, ni glisser, partout.
    pub(crate) fn modale_ouverte(&self) -> bool {
        self.ui.ind_dlg.is_some()
            || self.ui.confirmer.is_some()
            || self.ui.obj_dlg.is_some()
            || self.ui.name_box.is_some()
            || self.session.key_box.is_some()
            || self.session.words_box.is_some()
            || self.ui.support.is_some()
    }

    /// 🧱 Ce qui est au-dessus de la chart en ce point de la fenêtre (`wx`, `wy`), du haut vers le bas : boîtes
    /// modales, menus et fenêtres (liste, ARGOS, Panneaux, ▦, arbre, outils de la barre), éditeur, panneau ＋, volet
    /// flottant, raccourcis, Argos AI, barre de favoris ; la carte Argos AI d'un trade ; les légendes. `true` : la
    /// chart ne reçoit rien à cet endroit (curseur, clic, molette, glisser), même si une ligne d'alerte, un dessin ou un
    /// repère est dessous.
    pub(crate) fn couche_au_dessus(&self, wx: f64, wy: f64) -> bool {
        self.couche_haute(wx, wy) || self.fp_sous(wx, wy) || self.avis_sous(wx, wy) || self.sur_legende(wx, wy)
    }

    /// 🏷️ Les légendes (coordonnées de la fenêtre) : celle du prix (titre, O H L C · Vol, ⌄), la liste ouverte des
    /// indicateurs sur le prix et le nom (et les valeurs) de chaque panneau du bas. Une couche au-dessus de la chart
    /// (`couche_au_dessus`) : la flèche dessus, la main sur leurs boutons (`curseur_chart`).
    pub(crate) fn legendes(&self) -> Vec<(String, Rect)> {
        let (rw, top_h) = (self.rw(), self.top_h());
        let vp = &self.chart.viewport;
        let (pw, ph) = (vp.plot_w(), vp.plot_h());
        let lw = crate::render::LEGENDE_W.min(pw - 16.0);
        let mut v = vec![(
            "légende du prix".to_string(),
            Rect::new(rw + 8.0, top_h + 6.0 + argos_render::haut_legende(), lw, 92.0),
        )];
        if let Some((haut, bas)) = argos_render::indleg::lignes_liste(&self.ind.out).filter(|_| self.ind.leg.0) {
            v.push(("liste des indicateurs".to_string(), Rect::new(rw + 8.0, top_h + haut, lw, bas - haut)));
        }
        for (i, top) in self.ind.out.bands(ph) {
            v.push((format!("légende {i:?}"), Rect::new(rw + 4.0, top_h + top + 2.0, 260.0f64.min(pw - 8.0), 18.0)));
        }
        v
    }

    /// 🏷️ Sur une légende (prix, liste ouverte, panneaux du bas) ou un tableau d'indicateur, en ce point de la fenêtre.
    pub(crate) fn sur_legende(&self, wx: f64, wy: f64) -> bool {
        let (rw, th) = (self.rw(), self.top_h());
        self.legendes().iter().any(|(_, r)| r.contains(wx, wy))
            || argos_render::indicators::tableaux().iter().any(|r| r.contains(wx - rw, wy - th))
            || self.legende_bouton(wx - rw, wy - th).is_some()
            || self.pad_bouton(wx - rw, wy - th).is_some()
    }

    /// 🔍 The button of the navigation pill under (`x`, `y`) (chart coordinates; PC only, always visible).
    pub(crate) fn pad_bouton(&self, x: f64, y: f64) -> Option<argos_render::navbar::NavBtn> {
        let vp = &self.chart.viewport;
        (!self.top.phone)
            .then(|| argos_render::navbar::hit(vp, vp.plot_h(), Some((x, y)), true))
            .flatten()
    }

    /// 🏷️ Le bouton de légende sous (`x`, `y`) (coordonnées de la chart) : ⌄ N, 👁 ⚙ { } ↑ ↓ ▾ ✕, rond AI…
    pub(crate) fn legende_bouton(&self, x: f64, y: f64) -> Option<argos_render::indleg::LegClick> {
        let vp = &self.chart.viewport;
        if x < 0.0 || y < 0.0 || x >= vp.plot_w() || y >= vp.plot_h() {
            return None;
        }
        let cell = argos_render::indleg::Cell {
            actif: self.top.phone,
            sel: self.ind.sel,
            pw: vp.plot_w(),
        };
        argos_render::indleg::click_de(
            &argos_render::text::Text::default(),
            &self.ind.on,
            &self.ind.out,
            self.ind.leg.0,
            vp.plot_h(),
            vp.plot_w(),
            cell,
            x,
            y,
        )
    }

    /// 🧱 Les couches hautes (sans les légendes) : boîtes modales, menus, fenêtres, volet, Argos AI, barre de favoris.
    pub(crate) fn couche_haute(&self, wx: f64, wy: f64) -> bool {
        let u = &self.ui;
        let dedans = |r: Rect| r.contains(wx, wy);
        let (rw, th) = (self.rw(), self.top_h());
        self.modale_ouverte()
            || u.list.as_ref().is_some_and(|(_, m)| dedans(m.rect))
            || u.pop
                .as_ref()
                .is_some_and(|p| dedans(Rect::new(rw + p.rect.x, th + p.rect.y, p.rect.w, p.rect.h)))
            || u.sheet.as_ref().is_some_and(|s| dedans(s.rect))
            || u.argos.as_ref().is_some_and(|m| dedans(m.rect))
            || u.split_menu.as_ref().is_some_and(|m| dedans(m.rect))
            || u.obj_tree.as_ref().is_some_and(|t| dedans(t.rect))
            || (u.editor.is_some() && dedans(self.ed_rect()))
            || u.add.as_ref().is_some_and(|p| dedans(p.rect))
            || self.volet_rect().is_some_and(dedans)
            || (u.raccourcis.is_some() && dedans(self.raccourcis_rect()))
            || (u.ai_open && dedans(self.ai_rect()))
            || dedans(self.ai_bouton_rect())
            || toolbar::contains(&self.tools, &argos_render::text::Text::default(), wx - rw, wy - th)
            || self.month_under(wx, wy)
            || self.funding_sous(wx, wy)
    }

    /// 🖍 Le curseur Démonstration est-il choisi (et l'outil Curseur pris) ?
    pub(crate) fn demo_active(&self) -> bool {
        self.tool == "cursor" && self.ui.curseur == crate::types::CurseurChoisi::Demonstration
    }

    /// 🖍 Dessine les traits de démonstration sur l'image de la chart (couleur d'accent, 3 px, fondu en 3 s après le
    /// relâcher) et oublie ceux qui sont effacés.
    pub(crate) fn dessiner_demo(&mut self, pm: &mut tiny_skia::Pixmap) {
        let fondu = crate::types::DEMO_FONDU.as_secs_f64();
        self.ui.demo_traits.retain(|t| t.fini.is_none_or(|f| f.elapsed().as_secs_f64() < fondu));
        let sc = argos_render::px::scale() as f32;
        // (le bleu d'accent de la maquette, bien visible ; l'accent du thème Défaut est un gris neutre)
        let c = argos_render::jetons::ACCENT;
        for t in &self.ui.demo_traits {
            let a = t.fini.map_or(1.0, |f| (1.0 - f.elapsed().as_secs_f64() / fondu).clamp(0.0, 1.0)) as f32;
            let mut paint = tiny_skia::Paint::default();
            paint.set_color_rgba8(c.0, c.1, c.2, (a * 255.0) as u8);
            paint.anti_alias = true;
            if t.pts.len() == 1 {
                let (x, y) = t.pts[0];
                if let Some(p) = tiny_skia::PathBuilder::from_circle(x as f32 * sc, y as f32 * sc, 3.0 * sc) {
                    pm.fill_path(&p, &paint, tiny_skia::FillRule::Winding, tiny_skia::Transform::identity(), None);
                }
                continue;
            }
            let mut pb = tiny_skia::PathBuilder::new();
            for (k, (x, y)) in t.pts.iter().enumerate() {
                if k == 0 {
                    pb.move_to(*x as f32 * sc, *y as f32 * sc);
                } else {
                    pb.line_to(*x as f32 * sc, *y as f32 * sc);
                }
            }
            if let Some(p) = pb.finish() {
                let st = tiny_skia::Stroke {
                    width: 3.0 * sc,
                    line_cap: tiny_skia::LineCap::Round,
                    line_join: tiny_skia::LineJoin::Round,
                    ..Default::default()
                };
                pm.stroke_path(&p, &paint, &st, tiny_skia::Transform::identity(), None);
            }
        }
    }

    pub(crate) fn mouse_left(&mut self) {
        self.pointer.mouse = None;
        self.pointer.tab.survol = None;
        self.pointer.drag = None;
        self.pointer.axis_drag = None;
        self.pointer.band_drag = None;
        self.pointer.band_pan = None;
        self.pointer.band_zoom = None;
        self.pointer.script_pan = None;
        self.pointer.script_zoom = None;
        self.pointer.grab = None;
        self.redraw();
    }

    /// 👆 The long press on a pane's top edge (a native band's or a script's, ±14 points by the finger): the edge is
    /// held, as a mouse press on it does (`true`); elsewhere nothing changes (`false`: the crosshair, as before).
    pub(crate) fn edge_grab(&mut self) -> bool {
        let Some((x, y)) = self.pointer.mouse else { return false };
        if self.modale_ouverte() || !(0.0..=self.chart.viewport.plot_w()).contains(&x) || self.couche_haute(self.pointer.win_mouse.0, self.pointer.win_mouse.1)
        {
            return false;
        }
        if self.script_bord_appui(y) {
            return true;
        }
        let Some(ind) = self.ind.out.band_edge(self.chart.viewport.plot_h(), y) else {
            return false;
        };
        self.pointer.band_drag = Some((ind, y, ind.band_h()));
        self.set_cursor(CursorIcon::RowResize);
        true
    }

    /// 📜 The view key of the script pane under `y` (chart points): inside it, and its top edge too when `edge`.
    fn script_pane_at(&self, y: f64, edge: bool) -> Option<String> {
        let ph = self.chart.viewport.plot_h();
        let out = &self.ind.out;
        out.script_at(ph, y)
            .or_else(|| edge.then(|| out.script_edge(ph, y)).flatten())
            .map(|k| out.scripts[k].view_key())
    }

    pub(crate) fn right_click(&mut self) {
        // barre de gauche : menu de l'outil sous la souris et de la barre d'outils (comme Argos v1)
        if let Some((rx, ry)) = self.pointer.rail_mouse {
            let t = match rail::hit(rx, ry, self.area.1 as f64) {
                Some((rail::Btn::Tool(id), _)) => Some(id),
                Some((rail::Btn::Group(g), _)) => Some(self.rail.current(g)),
                _ => None,
            }
            .filter(|t| !matches!(*t, "cursor" | "erase"));
            self.ui.list = None;
            let (wx, wy) = self.pointer.win_mouse;
            return self.open_list(ListKind::ToolMenu(t), Rect::new(wx, wy, 0.0, 0.0));
        }
        // ↕️ barre d'icônes de droite : remettre l'ordre, masquer / réafficher un bouton
        if let Some((sx, sy)) = self
            .pointer
            .side_mouse
            .filter(|(sx, _)| !self.top.phone && *sx >= if self.watch.prefs.open { self.watch.pw() } else { 0.0 })
        {
            let t = self.watch.tool_at(sx, sy).map(|(_, t)| t);
            self.ui.list = None;
            let (wx, wy) = self.pointer.win_mouse;
            return self.open_list(ListKind::IconBar(t), Rect::new(wx - 240.0, wy, 0.0, 0.0));
        }
        // 🤖 la carte Argos AI d'un trade, 📅 the month report: no menu of what is under them
        let (wx, wy) = self.pointer.win_mouse;
        if self.fp_sous(wx, wy) || self.avis_sous(wx, wy) || self.month_under(wx, wy) {
            return;
        }
        let Some((x, y)) = self.pointer.mouse else { return };
        // 📋 a table (PC): its small menu (back in place, size, hide)
        if self.tab_menu_ouvrir(x, y) {
            return;
        }
        // barre flottante (ou sa fenêtre d'outils) : menu de l'outil / de la barre d'outils (comme Argos v1)
        let text = argos_render::text::Text::default();
        // 📱 au doigt, un dessin caché sous la barre (ou sous sa zone de toucher de 44) : son menu passe avant celui de
        // la barre (sinon la partie cachée du dessin n'avait plus de menu)
        let dessin_dessous = self.top.phone && self.ui.pop.is_none() && self.drawing_at(x, y).is_some();
        // (📱 the single pill: no floating bar, its tools are in the « Dessins » sheet)
        if !dessin_dessous && !self.pilule() && (toolbar::contains(&self.tools, &text, x, y) || self.ui.pop.as_ref().is_some_and(|p| p.rect.contains(x, y))) {
            let t = match toolbar::hit(&self.tools, &text, x, y) {
                Some(Btn::Fav(t)) => Some(t),
                _ => None,
            };
            self.ui.pop = None;
            self.ui.list = None;
            return self.open_list(ListKind::ToolMenu(t), Rect::new(x + self.rw(), y + self.top_h(), 0.0, 0.0));
        }
        // 🔔 an alert line (or its short form): its menu — pause, edit, delete, « Lignes d'alerte », colour (before the
        // drawings; 👆 a long press on phone and tablet comes here too)
        if let Some(id) = self.alert_at() {
            (self.ui.list, self.ui.alerte_sel) = (None, None);
            return self.open_list(ListKind::AlertLine(id), Rect::new(x + self.rw(), y + self.top_h(), 0.0, 0.0));
        }
        if let Some(i) = self.drawing_at(x, y) {
            self.selected = Some(i);
            self.ui.list = None;
            // 👆 phone and tablet (Jee, 17 Nov): a long press on a drawing shows its bar (it is selected)
            if self.top.phone {
                return self.redraw();
            }
            return self.open_list(ListKind::Drawing(i), Rect::new(x + self.rw(), y + self.top_h(), 0.0, 0.0));
        }
        let (pw, ph) = (self.chart.viewport.plot_w(), self.chart.viewport.plot_h());
        // 👆 phone and tablet: a long press on the chart lays the crosshair (its menu: the « ⋯ » beside the price);
        // on the scales, their menus as before
        if self.top.phone && x <= pw && y <= ph {
            return self.croix_poser();
        }
        let source = if x > pw {
            MenuSource::PriceAxis
        } else if y > ph {
            MenuSource::TimeAxis
        } else {
            // (⛶ what its last line puts full screen: the panel or the chart under the right-click)
            self.ui.plein_cible = Some(self.plein_cible_en(y));
            MenuSource::Chart
        };
        self.open_menu(source, Anchor::Cursor(x, y));
    }

    /// ✨ La souris est sur le bouton rond d'Argos AI ou sur son volet ouvert (ils passent par-dessus l'Accueil).
    fn sur_argos_ai(&self) -> bool {
        let (x, y) = self.pointer.win_mouse;
        self.ai_bouton_rect().contains(x, y) || (self.ui.ai_open && self.ai_rect().contains(x, y))
    }

    /// Un bouton de la barre du haut (📐 cell : aussi les boutons de la pilule de la maquette).
    pub(crate) fn top_click_do(&mut self, c: TopClick) {
        let w = self.top_w();
        match c {
            TopClick::Open(coin) => return self.open_coin(coin),
            TopClick::Changed => self.save_top(),
            TopClick::Home => return self.open_accueil(),
            TopClick::Recherche => return self.open_palette(),
            // 📊 fx : la même fenêtre que les boutons de la barre d'outils, en haut de la chart, sous le bouton
            TopClick::Style(r) => return self.pop_du_haut(toolbar::PopKind::Style, r),
            // 🖥 PC : ƒx ouvre le volet Indicateurs de la maquette (la grille « Gamme Argos » reste au ƒx de la barre de favoris)
            TopClick::Indicateurs(_) if !self.top.phone => return self.fx_volet(),
            TopClick::Indicateurs(r) => return self.pop_du_haut(toolbar::PopKind::Ind, r),
            // 🖥 PC : 🔔 ouvre la fenêtre des notifications de la maquette (fil, filtres, « + Nouvelle alerte » qui
            // ouvre le menu des alertes d'avant) ; 📱 le menu des alertes, comme avant
            TopClick::Alertes(r) if !self.top.phone => return self.notifs_pop(r),
            TopClick::Alertes(r) => return self.open_list(ListKind::Alerts, Rect::new(r.x, r.y + r.h, r.w, 1.0)),
            TopClick::Partager => self.banner = Some(self.partager_chart()),
            TopClick::Support => return self.support_ouvrir(),
            // 🖥 ▾ : les deux rangées du haut se replient (le bouton rond en bas à droite de la chart les rouvre)
            TopClick::ReplierBandeaux => self.bandeaux(!self.top.sans_bandeaux),
            // 🛡 / le symbole : leur volet (Trading avec la fiche Gardien / Watchlist) ; 2e clic : fermé (demande de Jee)
            // 🖱 PC (Jee, 22 Nov): the click on the pill opens the market panel, pinned (the Symboles panel: its
            // « Ma watchlist › »)
            TopClick::Symbol if !self.top.phone && !argos_render::actions::tactile() => return self.marches_clic_pastille(),
            // 🖥 tablette : le symbole ⌄ ouvre le volet « Symboles » de la maquette (⋯ Toutes les options : la watchlist d'avant)
            TopClick::Symbol if !self.top.phone => {
                self.ui.marches = None;
                return self.symboles_volet();
            }
            TopClick::Gardien | TopClick::Symbol => {
                let t = if c == TopClick::Gardien { wl::Tool::Trading } else { wl::Tool::Watchlist };
                let deja = self.watch.prefs.open && self.watch.prefs.panel == t;
                (self.watch.prefs.open, self.watch.prefs.panel) = (!deja, t);
                // 🛡 Gardien : la feuille Trading en haut, sa carte du Gardien visible tout de suite (même éteint : « Activer »)
                if c == TopClick::Gardien && !deja {
                    self.watch.panels.trading.options = false;
                    self.watch.haut_du_volet();
                }
                self.save_watch();
            }
            // 🖥 PC : ⚙ ouvre la fenêtre « Réglages » de la maquette (⋯ Toutes les options : les Panneaux d'avant)
            TopClick::Menu(TopMenu::Settings, _) if !self.top.phone => return self.reglages_ouvrir(),
            // ⚙ (les Panneaux) : sa fenêtre sous la barre (2e clic sur le même bouton : fermée plus haut)
            TopClick::Menu(TopMenu::Settings, r) => {
                let kind = SheetKind::Panels;
                if self.ui.sheet_closed != Some(kind) {
                    self.ui.sheet = Some(Sheet::below(kind, r, r.y + r.h, w));
                }
            }
            TopClick::Menu(TopMenu::Split, r) => {
                let wh = self.top_h() + self.area.1 as f64;
                (self.ui.list, self.ui.sheet, self.ui.argos) = (None, None, None);
                self.ui.split_menu = if self.ui.split_menu.is_some() {
                    None
                } else {
                    Some(argos_render::splits::Menu::new(r, self.win_w(), wh))
                };
            }
            // ☰ Appli : directement le menu de l'appli (2e clic : fermé)
            TopClick::Menu(TopMenu::App, r) => {
                let (ww, wh) = (self.win_w(), self.top_h() + self.area.1 as f64);
                (self.ui.list, self.ui.sheet) = (None, None);
                self.ui.argos = if self.ui.argos.as_ref().is_some_and(|m| m.app) {
                    None
                } else {
                    Some(topmenu::ArgosMenu::app(Rect::new(r.x + r.w - 440.0, r.y, 440.0, r.h), ww, wh))
                };
            }
            TopClick::Menu(TopMenu::Argos, r) => {
                let (ww, wh) = (self.win_w(), self.top_h() + self.area.1 as f64);
                (self.ui.list, self.ui.sheet) = (None, None);
                // 2e clic sur ARGOS ▾ : fermé (comme ☰)
                if self.ui.argos.as_ref().is_some_and(|m| !m.app) {
                    self.ui.argos = None;
                    return self.redraw();
                }
                // 📱 cell : la feuille du bas de la maquette, toute la largeur, jusqu'au bas de l'écran
                self.ui.argos = Some(if self.top.phone {
                    topmenu::ArgosMenu::feuille(ww, wh + argos_render::ecrans::dock_h())
                } else {
                    topmenu::ArgosMenu::new(r, ww, wh)
                });
            }
            TopClick::Menu(m, r) => return self.open_list(ListKind::Top(m), r),
            // ⏱ à droite du symbole : le menu des intervalles, ou un favori
            TopClick::Intervals(r) => return self.open_list(ListKind::Intervals, r),
            TopClick::Interval(secs) => return self.set_interval(Interval(secs)),
            TopClick::None => {}
        }
        self.redraw()
    }

    /// 👆 Double-clic / double-toucher : le clic d'avant date de moins de 350 ms ET s'est posé au même endroit (à 24
    /// points près). Deux touchers à des endroits différents (le nom d'un indicateur puis son 👁) ne font plus un double.
    pub(crate) fn est_double(&self, avant: Option<Instant>, now: Instant) -> bool {
        const DOUBLE_MS: u128 = 350;
        const DOUBLE_PX: f64 = 24.0;
        let (mx, my) = self.pointer.win_mouse;
        // (✏️ B-019: a finger that slides is never a double tap — a tap to select a drawing and a drag to move it
        // put the chart full screen)
        !self.pointer.appui_glisse
            && avant.is_some_and(|t| now.duration_since(t).as_millis() < DOUBLE_MS)
            && self.pointer.appui_avant.is_some_and(|(x, y)| (mx - x).hypot(my - y) <= DOUBLE_PX)
    }

    /// 🖥 Volets de réglage (pas de travail) : ils se ferment au clic à côté, comme la maquette (`fermerTout`). Aujourd'hui
    /// le volet API (Bourses) ; les volets de travail (Trading, Démo, Watchlist, Modules…) restent ouverts.
    pub(crate) fn volet_ferme_au_clic_a_cote(&self) -> bool {
        self.watch.prefs.open && !self.top.phone && self.watch.fixed_w.is_none() && self.watch.prefs.panel == wl::Tool::Exchanges
    }

    pub(crate) fn left_button(&mut self, state: ElementState) {
        let pressed = state == ElementState::Pressed;
        if pressed {
            self.pointer.appui_avant = self.pointer.appui.replace(self.pointer.win_mouse);
        }
        // 📋 the tables: their menu, a press on one (move, scale, stretch; double-click: back in place), its release
        if self.ui.confirmer.is_none() && self.tab_bouton(pressed) {
            return;
        }
        // ✅ a confirmation box open takes every click (outside it: cancelled)
        if self.ui.confirmer.is_some() {
            if pressed {
                let (x, y) = self.pointer.win_mouse;
                self.confirmer_clic(x, y);
            }
            return;
        }
        // 🔎 the market panel of the symbol pill: it takes the click; outside it, it closes and the click goes on
        if pressed && self.ui.marches.is_some() {
            let (x, y) = self.pointer.win_mouse;
            if self.marches_click(x, y) {
                return;
            }
        }
        // 📱 ⚡ a perpetual's Funding pill: it opens or closes its card; the card takes the touch (the chart does not
        // move); elsewhere an open card closes and the touch goes on
        if pressed {
            let (x, y) = self.pointer.win_mouse;
            if self.funding_press(x, y) {
                return;
            }
        }
        // ⚡ the card (PC) or sheet (phone) of the chart's news, then 📅 a release's card: they take the click (the PC
        // card closes on a click outside it, and the click goes on)
        if pressed {
            let (x, y) = self.pointer.win_mouse;
            if self.ui.eclair.is_some() && self.eclair_press(x, y) {
                return;
            }
            if self.watch.eco.carte.is_some() && self.cal_carte_press(x, y) {
                return;
            }
        }
        // ⚡ a click on a ⚡ of the chart: its card (PC) or its sheet (phone, tablet)
        if pressed && !self.modale_ouverte() && !self.couche_haute(self.pointer.win_mouse.0, self.pointer.win_mouse.1) {
            if let Some((cx, lien)) = self.pointer.mouse.and_then(|(x, y)| argos_render::econ::eclair_sous(x, y)) {
                return self.eclair_ouvrir(lien, cx);
            }
            // 🧭 the ✕ of an address's line pill (« Voir sur la chart »): the lines go
            if let Some((x, y)) = self.pointer.mouse {
                if self.hl_lignes_press(x, y) {
                    return;
                }
            }
        }
        // ⌖ the ticket waits for a price: the touch on the chart gives it (that is all the touch does)
        if pressed && self.watch.panels.trading.cible && !self.modale_ouverte() && !self.couche_haute(self.pointer.win_mouse.0, self.pointer.win_mouse.1) {
            if let Some(p) = self.price_at_mouse() {
                return self.ticket_cible(p);
            }
        }
        // 🤖 « Explique » on the Argos Lens card: Argos AI explains with its rules (always, no network)
        if pressed
            && !self.modale_ouverte()
            && !self.couche_haute(self.pointer.win_mouse.0, self.pointer.win_mouse.1)
            && self.pointer.mouse.is_some_and(|(x, y)| self.sur_explique(x, y))
        {
            return self.expliquer_regles();
        }
        // ≫ F1j: the back-to-live button (shown only off live) goes back to the last candles
        if pressed
            && !self.modale_ouverte()
            && !self.couche_haute(self.pointer.win_mouse.0, self.pointer.win_mouse.1)
            && self
                .pointer
                .mouse
                .is_some_and(|(x, y)| argos_render::bouton_direct_zone().is_some_and(|r| r.contains(x, y)))
        {
            self.run_action(ChartAction::GoEnd);
            return self.redraw();
        }
        // 🖍 the drawing bar let go after a drag by its handle; 🎨 the palette's slider let go
        if !pressed && self.pointer.drawbar_drag.take().is_some() {
            return self.redraw();
        }
        if !pressed && std::mem::take(&mut self.pointer.cp_drag) {
            return self.redraw();
        }
        // 🎚 the ticket's % slider let go
        if !pressed && self.pointer.curseur_ticket.take().is_some() {
            return self.redraw();
        }
        // 👆 the finger's crosshair: « ⋯ » opens its sheet; any other touch takes it away (on the chart, that is all
        // the touch does)
        if pressed {
            if let Some(r) = self.croix_zone() {
                let (wx, wy) = self.pointer.win_mouse;
                if r.contains(wx, wy) {
                    return self.croix_menu();
                }
                self.ui.croix = None;
                let sur_chart = self
                    .pointer
                    .mouse
                    .is_some_and(|(x, y)| x >= 0.0 && y >= 0.0 && x <= self.chart.viewport.plot_w() && y <= self.chart.viewport.plot_h());
                if sur_chart && self.ui.pfeuille.is_none() {
                    self.pointer.appui_croix = true;
                    return self.redraw();
                }
            }
        } else if std::mem::take(&mut self.pointer.appui_croix) {
            // (the lift leaves no crosshair where the finger was: the touch only took it away)
            self.pointer.mouse = None;
            return self.redraw();
        }
        // 👆 🧱 a tap on an Argos Murs wall pill (or its dot) or depth bar opens its tooltip when the finger lifts
        // there (F1j: a real tap, see `play`); the next tap closes it (and goes on)
        if pressed && argos_render::actions::tactile() {
            let (wx, wy) = self.pointer.win_mouse;
            let ouvert = argos_render::murs::bulle().is_some();
            argos_render::murs::set_bulle(None);
            self.pointer.murs_appui = None;
            let libre = self.ui.pfeuille.is_none() && !self.modale_ouverte() && !self.couche_haute(wx, wy);
            if let Some((x, y)) = self.pointer.mouse.filter(|_| libre && !ouvert) {
                if argos_render::murs::pilule_en(x, y) || argos_render::murs::barre_en(x, y) {
                    self.pointer.murs_appui = Some((wx, wy, x, y));
                    return self.redraw();
                }
            }
            if ouvert {
                self.redraw();
            }
        }
        // 📱 the open sheet of the single pill takes the touch (outside it: closed); let go: a little pull comes back
        // (a tile acts when the finger lifts where it was put down: a scroll never opens anything, and the lift does
        // not land on what the tile just opened)
        if self.ui.pfeuille.is_some() {
            let (wx, wy) = self.pointer.win_mouse;
            let Some(p) = self.ui.pfeuille.as_mut() else { return };
            if pressed {
                p.appui = Some((wx, wy));
                return self.redraw();
            }
            let (appui, tire) = (p.appui.take(), p.tire > 0.0);
            self.pfeuille_lacher();
            if let Some((x0, y0)) = appui.filter(|(x0, y0)| !tire && (wx - x0).abs() < 14.0 && (wy - y0).abs() < 14.0) {
                return self.pfeuille_toucher(x0, y0);
            }
            return self.redraw();
        }
        // 🎨 the palette of the drawing bar, over everything on the chart; 🔔 the alert window of a drawing
        if pressed && self.colorpick_press(self.pointer.win_mouse.0, self.pointer.win_mouse.1) {
            return;
        }
        if pressed && self.draw_alert_press(self.pointer.win_mouse.0, self.pointer.win_mouse.1) {
            return;
        }
        // 📱 the drawing bar over the phone's pills and chips (top of the chart): it takes the touch first — but never
        // before a menu opened over it (filet n° 41: touching a row of the menu changed the width, locked or deleted
        // the drawing under it)
        let (wx, wy) = self.pointer.win_mouse;
        let u = &self.ui;
        let dans = |r: Rect| r.contains(wx, wy);
        let menu_dessus = u.context.is_open()
            || u.list.as_ref().is_some_and(|(_, m)| dans(m.rect))
            || u.argos.as_ref().is_some_and(|m| dans(m.rect))
            || u.split_menu.as_ref().is_some_and(|m| dans(m.rect))
            || u.sheet.as_ref().is_some_and(|s| dans(s.rect))
            || u.pop
                .as_ref()
                .is_some_and(|p| dans(Rect::new(self.rw() + p.rect.x, self.top_h() + p.rect.y, p.rect.w, p.rect.h)));
        if pressed && self.top.phone && !menu_dessus {
            let (cx, cy) = (self.pointer.win_mouse.0 - self.rw(), self.pointer.win_mouse.1 - self.top_h());
            if self.drawbar().is_some_and(|(_, b)| b.rect.contains(cx, cy)) && self.drawbar_click(cx, cy) {
                return;
            }
        }
        // 🖥 colonne de modules : fin du geste (ordre, hauteur, largeur gardés)
        if !pressed && self.dock.geste.is_some() {
            self.dock_lacher();
            return self.redraw();
        }
        // 📊 fenêtre de la barre ouverte par-dessus les rangées du haut : elle reçoit le clic (pas la barre du haut)
        if pressed {
            if let Some((cx, cy)) = self.sur_pop_du_haut(self.pointer.win_mouse.0, self.pointer.win_mouse.1) {
                self.pointer.mouse = Some((cx, cy));
                self.pop_click(cx, cy);
                return self.redraw();
            }
        }
        // 🖍 Démonstration (curseur choisi) : Alt tenu + clic = un point, Alt tenu + glisser = un trait ; ils
        // s'effacent en 3 s (comme TradingView) ; relâcher finit le trait, où que soit la souris
        if !pressed {
            if let Some(t) = self.ui.demo_traits.last_mut().filter(|t| t.fini.is_none()) {
                t.fini = Some(Instant::now());
                return self.redraw();
            }
        } else if self.demo_active() && self.pointer.mods.alt_key() {
            if let Some(m) = self.pointer.mouse {
                self.ui.demo_traits.push(crate::types::TraitDemo { pts: vec![m], fini: None });
                return self.redraw();
            }
        }
        // 🖱 relâché hors de la chart (volet, rail, menu, hors de la fenêtre) pendant un glisser de la chart ou d'un
        // objet : le geste s'arrête là (plus de main fermée collée au retour sur la chart)
        if !pressed && self.pointer.mouse.is_none() && (self.pointer.drag.is_some() || self.pointer.grab.is_some()) {
            self.fin_du_glisser();
            return self.redraw();
        }
        // 🔐 boîte des 24 mots, 🔑 boîte de clé API : elles passent avant tout (fenêtres bloquantes) ; à côté de la boîte de
        // clé = annuler (tout ce qui a été tapé est effacé), sans que le clic n'aille à la chart, au volet ou à un module ;
        // le relâcher n'agit nulle part ailleurs
        if self.session.words_box.is_some() || self.session.key_box.is_some() {
            if pressed {
                let (x, y) = self.pointer.win_mouse;
                if let Some(b) = self.session.words_box.as_mut() {
                    let c = b.click(x, y);
                    self.words_click(c);
                } else if let Some(b) = self.session.key_box.as_mut() {
                    match b.click(x, y) {
                        KeyClick::Save => self.key_save(),
                        KeyClick::Cancel => {
                            b.wipe();
                            if b.exchange == "account-verify" {
                                self.session.acc_draft = None;
                            }
                            self.session.key_box = None;
                        }
                        KeyClick::None => {}
                    }
                }
            }
            return self.redraw();
        }
        // 🛟 Support panel: a modal layer (the chart receives nothing while it is open); its clicks on release,
        // ✎ Circle: press-and-drag on the preview draws an ellipse; outside it, only the 1st top row (🛟 closes
        // it again) and Argos AI respond
        if self.ui.support.is_some() {
            let (x, y) = self.pointer.win_mouse;
            if !pressed && self.ui.support.as_ref().is_some_and(|g| g.trace_en_cours()) {
                if let Some(g) = self.ui.support.as_mut() {
                    g.lache();
                }
                return self.redraw();
            }
            if self.ui.support.as_ref().is_some_and(|g| g.rect().contains(x, y)) && !self.sur_argos_ai() {
                if pressed {
                    if self.ui.support.as_mut().is_some_and(|g| g.appui(x, y)) {
                        return self.redraw();
                    }
                    self.pointer.fx_appui = true;
                } else if std::mem::take(&mut self.pointer.fx_appui) {
                    return self.support_click();
                }
                return self.redraw();
            }
            if !(y < crate::render::VOLET_HAUT || self.sur_argos_ai()) {
                return self.redraw();
            }
        }
        // 🔌 volet API (réglages, pas un volet de travail) : un clic à côté le ferme, sans rien faire d'autre (ni dessin,
        // ni ordre, ni symbole) ; le volet et la colonne d'icônes gardent leurs clics, un menu ouvert par-dessus d'abord
        if !pressed && std::mem::take(&mut self.ui.relache_avale) {
            return self.redraw();
        }
        if pressed && self.volet_ferme_au_clic_a_cote() {
            let (x, _) = self.pointer.win_mouse;
            let menu = self.ui.list.is_some() || self.ui.pop.is_some() || self.ui.sheet.is_some() || self.ui.argos.is_some() || self.ui.context.is_open();
            if x < self.side_x() && !menu {
                self.watch.prefs.open = false;
                self.save_watch();
                self.ui.relache_avale = true;
                return self.redraw();
            }
        }
        // 🎡 la roue de la pilule : le doigt se lève
        if !pressed && self.ui.roue.is_some() {
            return self.roue_lever();
        }
        #[cfg(target_arch = "wasm32")]
        if pressed {
            let p = &self.pointer;
            crate::platform::web::etat(&format!(
                "win {:?} top {:?} phone {:?} side {:?} chart {:?} rail {:?} dock {:?} | top_h {} area {:?} w {} | watch {} {:?} rows {:?} | sheet {} list {} argos {} pop {} sim {} ai {} acc {} pal {} | drag tool {} bar {} wl {} icon {} grab {} drag {} | liste {:?} {:?}",
                p.win_mouse, p.top_mouse, p.phone_mouse, p.side_mouse, p.mouse, p.rail_mouse, p.dock_mouse,
                self.top_h(), self.area, self.win_w(),
                self.watch.prefs.open, self.watch.prefs.panel, self.ui.rows,
                self.ui.sheet.is_some(), self.ui.list.is_some(), self.ui.argos.is_some(), self.ui.pop.is_some(), self.ui.sim.is_some(), self.ui.ai_open, self.ui.accueil.is_some(), self.ui.palette.is_some(),
                p.tool_drag.is_some(), p.bar_press.is_some(), p.wl_resize.is_some(), p.icon_drag.is_some(), p.grab.is_some(), p.drag.is_some(),
                self.ui.list.as_ref().map(|(k, m)| (*k, m.rect)),
                self.ui.list.as_ref().map(|(_, m)| m.clone().click(p.win_mouse.0, p.win_mouse.1)),
            ));
        }
        // 📱 cell : la poignée d'une barre la cache ou la remontre ; barre du bas cachée : la mini-pastille DÉMO
        if pressed && self.poignee_toucher() {
            return;
        }
        // (rangée du haut cachée : la mini-pastille DÉMO en haut à gauche)
        if pressed
            && self.top.phone
            && self.haut_cache()
            && !self.pilule()
            && self.mini_demo_rect().contains(self.pointer.win_mouse.0, self.pointer.win_mouse.1)
        {
            return self.demo_pastille_toucher();
        }
        // 🔎 recherche Ctrl + K : elle prend le clic (une ligne l'ouvre, à côté la ferme)
        if self.ui.palette.is_some() {
            if pressed {
                self.palette_click();
            }
            return self.redraw();
        }
        // 🗺 plan de l'app (au relâcher : un doigt qui glisse fait défiler)
        if self.ui.plan.is_some() && !self.sur_argos_ai() {
            if pressed {
                self.pointer.fx_appui = true;
            } else if std::mem::take(&mut self.pointer.fx_appui) {
                return self.plan_click();
            }
            return self.redraw();
        }
        // ⚙ fenêtre Réglages (🖥) : ses clics au relâcher ; à côté, elle se ferme (un menu ouvert par-dessus, comme
        // 🔊 « Son des alertes », garde les siens)
        let (mx, my) = self.pointer.win_mouse;
        if self.ui.reglages.as_ref().is_some_and(|g| g.rect().contains(mx, my))
            && !self.ui.list.as_ref().is_some_and(|(_, m)| m.rect.contains(mx, my))
            && !self.sur_argos_ai()
        {
            if pressed {
                self.pointer.fx_appui = true;
            } else if std::mem::take(&mut self.pointer.fx_appui) {
                return self.reglages_click();
            }
            return self.redraw();
        }
        // 🔔 feuille des notifications (au relâcher : un doigt qui glisse fait défiler)
        // (🔔 a window opened from the alerts panel over it keeps its clicks: a line's menu, ⚙, the colour, « Éditer… »;
        // a press elsewhere on the panel closes the menu first)
        let par_dessus = self.ui.list.as_ref().is_some_and(|(_, m)| m.rect.contains(mx, my)) || self.ui.colorpick.is_some() || self.ui.draw_alert.is_some();
        if self.notifs_sous(self.pointer.win_mouse.0, self.pointer.win_mouse.1) && !self.sur_argos_ai() && !par_dessus {
            if pressed && self.ui.list.take().is_some() {
                return self.redraw();
            }
            if pressed {
                self.pointer.fx_appui = true;
            } else if std::mem::take(&mut self.pointer.fx_appui) {
                return self.notif_click();
            }
            return self.redraw();
        }
        // 🔎 volet Symboles (🖥) : ses clics au relâcher ; à côté, le reste répond
        if self.sym_rect().is_some_and(|v| v.contains(self.pointer.win_mouse.0, self.pointer.win_mouse.1)) && !self.sur_argos_ai() {
            if pressed {
                self.pointer.fx_appui = true;
            } else if std::mem::take(&mut self.pointer.fx_appui) {
                return self.sym_click();
            }
            return self.redraw();
        }
        // 🅘 page Indicateurs (au relâcher, comme les pages : un doigt qui glisse fait défiler)
        // (🖥 dans le volet flottant : seulement ses clics ; la chart et le reste répondent à côté)
        let hors_du_volet = self.fx_rect().is_some_and(|v| !v.contains(self.pointer.win_mouse.0, self.pointer.win_mouse.1));
        if self.ui.fx_page.is_some() && !self.sur_argos_ai() && !hors_du_volet {
            if pressed {
                self.pointer.fx_appui = true;
            } else if std::mem::take(&mut self.pointer.fx_appui) {
                return self.fx_click();
            }
            return self.redraw();
        }
        // ⌂ Accueil : il reçoit tous les clics (au relâchement, si le doigt n'a pas glissé), sauf quand un menu ou une
        // boîte de nom est ouvert par-dessus (⋯ d'une tuile : menu, Renommer)
        if self.ui.accueil.is_some() && self.ui.list.is_none() && self.ui.name_box.is_none() && !self.sur_argos_ai() {
            if pressed {
                self.pointer.mk_drag = Some((self.pointer.win_mouse.1, self.pointer.win_mouse.1, false));
            } else if !self.pointer.mk_drag.take().is_some_and(|(_, _, moved)| moved) {
                self.accueil_click();
            }
            return self.redraw();
        }
        let (wx, wy) = self.pointer.win_mouse;
        // 📋 un menu en liste ouvert prend d'abord le clic qui tombe dessus : une ligne d'alerte ou d'ordre de la chart,
        // cachée sous le menu, ne doit pas le voler (cell : « Cacher » du menu ⋯ d'un indicateur ne répondait pas)
        if pressed && self.ui.list.as_ref().is_some_and(|(_, m)| m.rect.contains(wx, wy)) {
            return self.list_click(wx, wy);
        }
        // 📅 the month report, over the chart and the Simulator panel: its buttons; a press elsewhere on it does not
        // reach what is under it, nor its release (a drag begun elsewhere, an order line, is let go over it)
        if pressed && self.month_under(wx, wy) {
            self.ui.relache_avale = true;
            return self.month_click(wx, wy);
        }
        // 🧪 volet du Simulateur, 🟠 bandeau DÉMO (Argos AI posé sur le bas du volet prend son clic)
        if self.ui.sim.is_some() && self.sim_rect().contains(wx, wy) && !self.sur_argos_ai() {
            if pressed {
                self.sim_click(wx, wy);
            }
            return self.redraw();
        }
        // 🤖 la carte « Argos AI · ton trade » : ✕, la leçon de l'Académie ; un appui ailleurs sur elle ne passe pas à la
        // chart, son relâcher non plus (un glisser commencé ailleurs, une ligne d'ordre par exemple, se lâche sur elle)
        // 🏦 le ⚙ du Footprint (ouvert par-dessus la carte Argos AI) : ses bourses, l'unité, ses interrupteurs
        if pressed && self.fp_sous(wx, wy) {
            self.ui.relache_avale = true;
            return self.fp_click(wx, wy);
        }
        if pressed && self.avis_sous(wx, wy) {
            self.ui.relache_avale = true;
            return self.avis_click(wx, wy);
        }
        if pressed && self.bandeau_click(wx, wy) {
            return;
        }
        // ⌨️ volet des raccourcis : ✕ le ferme ; le reste du volet ne fait rien
        let rr = self.raccourcis_rect();
        if self.ui.raccourcis.is_some() && rr.contains(wx, wy) {
            if pressed && argos_render::raccourcis::Raccourcis::fermer(rr).contains(wx - rr.x, wy - rr.y) {
                self.ui.raccourcis = None;
            }
            return self.redraw();
        }
        // ‹ › languettes : replier / déplier la barre de gauche ou la barre d'icônes de droite
        if pressed && (self.fold_click(wx, wy) || self.rail_garde_click(wx, wy)) {
            return;
        }
        // 🔔 ligne d'alerte : on l'attrape à 6 px près, on la relâche où on veut
        if !pressed {
            // 🪟 fin du glisser dans une case du split
            if self.pointer.pane_drag.take().is_some() {
                return self.redraw();
            }
            // ↕️ bouton de la barre d'icônes lâché : le nouvel ordre est gardé
            if let Some((_, _, moved, _)) = self.pointer.icon_drag.take() {
                if moved {
                    // 🖥 colonne d'interface-2 : son ordre à elle (interface-2.json)
                    if self.watch.col_outils.is_some() {
                        self.colonne_garder();
                    } else {
                        self.save_watch();
                    }
                    self.set_cursor(CursorIcon::Default);
                    return self.redraw();
                }
            }
            if let Some(id) = self.pointer.alert_drag.take() {
                // 🔔 pressed and released without moving: its label (again: it goes)
                if self.pointer.alert_appui.take().is_some() {
                    self.ui.alerte_sel = if self.ui.alerte_sel == Some(id) { None } else { Some(id) };
                    return self.redraw();
                }
                self.alert_moved(id);
                return self.redraw();
            }
            if self.pointer.order_drag.is_some() {
                return self.ordre_lache();
            }
        } else if let Some((mx, my)) = self
            .pointer
            .mouse
            .filter(|_| self.etiquette_alerte().is_some() && !self.couche_au_dessus(self.pointer.win_mouse.0, self.pointer.win_mouse.1))
            .filter(|&(mx, my)| self.etiquette_alerte_appui(mx, my))
        {
            // 🔔 the label took it (🗑, the handle, the name)
            let _ = (mx, my);
            return self.redraw();
        } else if let Some(id) = self
            .alert_at()
            .filter(|_| !self.couche_au_dessus(self.pointer.win_mouse.0, self.pointer.win_mouse.1))
        {
            (self.pointer.alert_drag, self.pointer.alert_appui) = (Some(id), Some(self.pointer.win_mouse));
            return self.redraw();
        } else if let Some(p) = self
            .ordre_prise()
            .filter(|_| !self.couche_au_dessus(self.pointer.win_mouse.0, self.pointer.win_mouse.1))
        {
            return self.ordre_clic(p);
        }
        // ✨ bouton de l'assistant : il passe avant tout le reste
        if self.ai_bouton_rect().contains(self.pointer.win_mouse.0, self.pointer.win_mouse.1) {
            if pressed {
                return self.toggle_ai();
            }
            return self.redraw();
        }
        // bandeau d'information : un toucher (ou un clic) le ferme
        if pressed {
            if let (Some(b), Some((mx, my))) = (&self.banner, self.pointer.mouse) {
                if argos_render::banner_rect(&self.chart.viewport, &argos_render::text::Text::default(), b).contains(mx, my) {
                    self.banner = None;
                    return self.redraw();
                }
            }
        }
        // 📐 tablette en portrait : le panneau est un tiroir, toucher la chart le referme
        if pressed && self.ui.form.drawer() && self.watch.prefs.open && self.pointer.mouse.is_some() && !self.ui.ai_open {
            self.watch.prefs.open = false;
            self.save_watch();
            return self.redraw();
        }
        // ✨ assistant ouvert : il reçoit le clic
        if self.ui.ai_open && self.session.key_box.is_none() && self.ai_rect().contains(self.pointer.win_mouse.0, self.pointer.win_mouse.1) {
            if pressed {
                use argos_render::assistant::AsClick;
                let r = self.ai_rect();
                let text = argos_render::text::Text::default();
                let sym = format!("{} {}", self.symbol_label(), self.chart.interval.short());
                match self
                    .ai
                    .click(&text, r.w, r.h, self.pointer.win_mouse.0 - r.x, self.pointer.win_mouse.1 - r.y, &sym)
                {
                    AsClick::Close => return self.toggle_ai(),
                    AsClick::Send(q) => return self.ai_send(q),
                    AsClick::Key => self.session.key_box = Some(self.boite_cle_claude()),
                    AsClick::Unlock => self.session.key_box = Some(KeyBox::new("claude-unlock", "Claude", self.win_w(), self.top_h() + self.area.1 as f64)),
                    AsClick::Forget => {
                        let _ = vault::forget(vault::FILE);
                        self.session.claude_key = None;
                        self.ai.key = argos_render::assistant::KeyState::None;
                        self.banner = Some("🗑 Clé Claude oubliée sur cet appareil".into());
                    }
                    AsClick::Memory => {
                        let anchor = Rect::new(r.x + r.w - 300.0, r.y + 44.0, 260.0, 1.0);
                        return self.open_list(ListKind::AiMemory, anchor);
                    }
                    AsClick::NoteYes => return self.ai_note_answer(true),
                    AsClick::NoteNo => return self.ai_note_answer(false),
                    AsClick::EffacerExplication => {
                        self.effacer_explication();
                        return self.redraw();
                    }
                    AsClick::AutresReglages => return self.deplier_reglages(),
                    AsClick::TousReglages => {
                        if let Some(i) = self.ai.tous_reglages {
                            return self.open_ind_dlg(i);
                        }
                    }
                    AsClick::LireTout => return self.lire_ensemble(crate::actions::argos_ai_ecran::Ensemble::Tout),
                    AsClick::Accord => return self.lire_ensemble(crate::actions::argos_ai_ecran::Ensemble::Accord),
                    AsClick::Volatilite => return self.lire_ensemble(crate::actions::argos_ai_ecran::Ensemble::Volatilite),
                    AsClick::NiveauxJour => return self.lire_ensemble(crate::actions::argos_ai_ecran::Ensemble::NiveauxJour),
                    AsClick::Focus | AsClick::None => {}
                }
            }
            return self.redraw();
        }
        // 🧩 colonne de modules : le clic va au module dessous
        if let Some((x, y)) = self.pointer.dock_mouse {
            if pressed && !self.dock_appui(x, y) {
                self.dock_click(x, y);
            }
            return self.redraw();
        }
        // bouton de la barre flottante relâché : clic (sur le même bouton), ou fin du déplacement / du tri
        if !pressed {
            if let Some(bp) = self.pointer.bar_press.take() {
                match bp.mode {
                    PressMode::Wait => {
                        let text = argos_render::text::Text::default();
                        let (mx, my) = self.pointer.mouse.unwrap_or((-1.0, -1.0));
                        if toolbar::hit(&self.tools, &text, mx, my) == Some(bp.btn) {
                            return self.tool_click(bp.btn, mx, my);
                        }
                    }
                    PressMode::Lift(..) => self.save_tools(),
                    PressMode::Sort => self.save_rail(),
                    PressMode::Cancel => {}
                }
                return self.redraw();
            }
        }
        if !pressed && self.pointer.tool_drag.take().is_some() {
            if !self.pointer.tool_moved {
                self.tools.retracted = !self.tools.retracted; // clic sur la poignée
                                                              // 📱 ouverte sur cell : elle glisse vers la gauche pour tenir dans l'écran (maquette : « garde la barre dans
                                                              // l'écran ») ; sinon on n'en voyait que deux boutons, coincés contre l'échelle de prix
                if self.top.phone && !self.tools.retracted && !self.tools.vertical {
                    let text = argos_render::text::Text::default();
                    self.tools.x = toolbar::place_ouverte(&self.tools, &text);
                    self.tools.scroll = 0.0;
                }
            }
            self.save_tools();
            return self.redraw();
        }
        // 🎓 Académie : elle reçoit tous les clics (au relâchement, si le doigt n'a pas glissé)
        if self.ui.ac_page.is_some() {
            if pressed {
                self.pointer.mk_drag = Some((self.pointer.win_mouse.1, self.pointer.win_mouse.1, false));
            } else if !self.pointer.mk_drag.take().is_some_and(|(_, _, moved)| moved) {
                self.academy_click();
            }
            return self.redraw();
        }
        // 🛒 Marketplace : il reçoit tous les clics ; le clic se fait au relâchement, seulement si le doigt n'a pas glissé
        if self.ui.mk_page.is_some() {
            if pressed {
                self.pointer.mk_drag = Some((self.pointer.win_mouse.1, self.pointer.win_mouse.1, false));
            } else if !self.pointer.mk_drag.take().is_some_and(|(_, _, moved)| moved) {
                self.market_click();
            }
            return self.redraw();
        }
        // 💼 page Portfolio : elle reçoit tous les clics
        if self.ui.pf_page.is_some() {
            if pressed {
                self.portfolio_click();
            }
            return self.redraw();
        }
        // 📊 the backtest report page: it gets every click
        if self.ui.bt_page.is_some() {
            if pressed {
                self.rapport_click();
            }
            return self.redraw();
        }
        // 📓 fiche d'un trade : elle reçoit tous les clics
        if self.ui.j_form.is_some() {
            if pressed {
                if let Some(mut f) = self.ui.j_form.take() {
                    let (x, y) = self.pointer.win_mouse;
                    let c = f.click(x, y, &argos_render::text::Text::default());
                    self.j_form_result(f, c);
                }
            }
            return self.redraw();
        }
        // 📜 éditeur : fenêtre comme v1, elle reçoit les clics (à côté : rien)
        if self.ui.editor.is_some() && self.ui.mk_page.is_none() && self.ui.pf_page.is_none() && self.ui.bt_page.is_none() {
            if pressed {
                let (r, (x, y)) = (self.ed_rect(), self.pointer.win_mouse);
                if r.contains(x, y) {
                    let c = self
                        .ui
                        .editor
                        .as_mut()
                        .map(|e| e.click(&argos_render::text::Text::default(), x - r.x, y - r.y, r.w, r.h));
                    if let Some(c) = c {
                        return self.editor_click(c);
                    }
                }
            }
            return self.redraw();
        }
        // fenêtre de réglages (indicateur ou dessin) : appuyer sur son titre = l'attraper pour la déplacer
        if !pressed && self.pointer.dlg_drag.take().is_some() {
            self.set_cursor(CursorIcon::Default);
            return self.redraw();
        }
        if pressed {
            let (x, y) = self.pointer.win_mouse;
            let r = self
                .ui
                .ind_dlg
                .as_ref()
                .map(|d| (d.title_bar(), d.rect))
                .or_else(|| self.ui.obj_dlg.as_ref().map(|d| (d.title_bar(), d.rect)));
            if let Some((bar, r)) = r.filter(|(bar, _)| bar.contains(x, y)) {
                let _ = bar;
                self.pointer.dlg_drag = Some((x - r.x, y - r.y));
                self.set_cursor(CursorIcon::Grabbing);
                return self.redraw();
            }
        }
        // fenêtre de réglages d'un indicateur : elle reçoit le clic (les changements se voient en direct)
        if pressed {
            if let Some(mut dl) = self.ui.ind_dlg.take() {
                let (x, y) = self.pointer.win_mouse;
                let c = dl.click(x, y, &argos_render::text::Text::default());
                self.ind_dlg_result(dl, c);
                return self.redraw();
            }
        }
        // fenêtre de réglages d'un dessin : elle reçoit le clic (les changements se voient en direct)
        if pressed {
            if let Some(mut dl) = self.ui.obj_dlg.take() {
                let (x, y) = self.pointer.win_mouse;
                let out = self.chart.drawings.get_mut(dl.index).map(|d| dl.click(x, y, d));
                if let Some(c) = out {
                    self.dlg_result(dl, c);
                }
                return self.redraw();
            }
        }
        // boîte de nom, puis panneau ＋ : ils reçoivent le clic (à côté = ils se ferment)
        if pressed {
            let (x, y) = self.pointer.win_mouse;
            if let Some(b) = &self.ui.name_box {
                match b.click(x, y) {
                    Some(true) => return self.name_ok(),
                    Some(false) => self.ui.name_box = None,
                    None => {}
                }
                return self.redraw();
            }
            if let Some(p) = self.ui.add.as_mut() {
                let text = argos_render::text::Text::default();
                match p.click(x, y, &text) {
                    AddClick::Outside => self.ui.add = None,
                    AddClick::Inside => {}
                    AddClick::Filter => self.search_add(),
                    AddClick::Add(id) => self.add_symbol(&id),
                }
                return self.redraw();
            }
        }
        // menu en liste ouvert : il reçoit le clic (à côté = il se ferme)
        if pressed && self.ui.list.is_some() {
            let (x, y) = self.pointer.win_mouse;
            return self.list_click(x, y);
        }
        if !pressed && self.pointer.wl_resize.take().is_some() {
            self.save_watch();
            return self.redraw();
        }
        if !pressed && self.watch.dragging() {
            self.watch.release();
            return self.redraw();
        }
        // 🌳 arborescence : elle reçoit le clic (à côté = fermer, le clic continue sur la chart)
        if pressed {
            if let Some(t) = self.ui.obj_tree.take() {
                use argos_render::objtree::TreeClick;
                let (wx, wy) = self.pointer.win_mouse;
                let rows = self.tree_rows();
                let inside = t.rect.contains(wx, wy);
                let res = t.click(&rows, wx, wy);
                let ind = |i: usize| {
                    i.checked_sub(argos_render::objtree::IND)
                        .and_then(|k| argos_render::indicators::Ind::ALL.get(k).copied())
                };
                match res {
                    TreeClick::Hide(i) if ind(i).is_some() => self.legend_click(argos_render::indleg::LegClick::Eye(ind(i).unwrap())),
                    TreeClick::Remove(i) if ind(i).is_some() => self.legend_click(argos_render::indleg::LegClick::Remove(ind(i).unwrap())),
                    TreeClick::Settings(i) => {
                        if let Some(k) = ind(i) {
                            self.open_ind_dlg(k);
                        }
                    }
                    TreeClick::Select(i) if ind(i).is_some() => {}
                    TreeClick::Select(i) => self.selected = Some(i),
                    TreeClick::Hide(i) => self.chart.drawings[i].style.hidden = !self.chart.drawings[i].style.hidden,
                    TreeClick::Lock(i) => {
                        let d = &mut self.chart.drawings[i];
                        d.set_locked(!d.locked());
                    }
                    TreeClick::Remove(i) if !self.chart.drawings[i].locked() => {
                        self.chart.drawings.remove(i);
                        self.selected = None;
                    }
                    _ => {}
                }
                if inside {
                    if res != TreeClick::Close {
                        self.ui.obj_tree = Some(t); // ✕ : fermée
                    }
                    return self.redraw();
                }
            }
        }
        // 🪟 menu Splits : il reçoit le clic (il reste ouvert pour comparer ; à côté = fermer) ; 📱 feuille du cell :
        // Argos AI, au coin par-dessus, garde son clic
        if pressed && !self.ui.split_menu.as_ref().is_some_and(|m| m.cell && self.sur_argos_ai()) {
            if let Some(mut m) = self.ui.split_menu.take() {
                use argos_render::splits::{MenuClick, Sync};
                let (wx, wy) = self.pointer.win_mouse;
                match m.click(wx, wy) {
                    MenuClick::Close => return self.redraw(),
                    MenuClick::Layout(k) => self.split.layout = k,
                    MenuClick::Sync(Sync::Symbol) => self.split.sync_symbol = !self.split.sync_symbol,
                    MenuClick::Sync(Sync::Interval) => self.split.sync_interval = !self.split.sync_interval,
                    MenuClick::Sync(Sync::Crosshair) => self.split.sync_crosshair = !self.split.sync_crosshair,
                    MenuClick::Sync(Sync::Time) => self.split.sync_time = !self.split.sync_time,
                    // 📱 the touch sheet's « Synchro › » sub-menu
                    MenuClick::Synchro(on) => m.synchro = on,
                    MenuClick::None => {}
                }
                Self::write_prefs("split.json", &self.split);
                self.ui.split_menu = Some(m);
                return self.redraw();
            }
        }
        // menu Argos ▾ : il reçoit le clic (à côté = fermer, le clic s'arrête là)
        // (📱 le bouton rond Argos AI reste par-dessus la feuille : il garde son toucher)
        if pressed && !(self.top.phone && self.sur_argos_ai()) {
            if let Some(mut m) = self.ui.argos.take() {
                let (wx, wy) = self.pointer.win_mouse;
                match m.click(wx, wy) {
                    topmenu::ArgosClick::Act(a) => return self.top_act(a as i64, m.rect),
                    topmenu::ArgosClick::None => self.ui.argos = Some(m),
                    topmenu::ArgosClick::Close => {}
                    // ☰ Appli ↔ ‹ Argos : l'autre menu, au même endroit
                    topmenu::ArgosClick::Switch => {
                        let (ww, wh) = (self.win_w(), self.top_h() + self.area.1 as f64);
                        let at = Rect::new(m.rect.x, m.rect.y - 6.0, 0.0, 0.0);
                        self.ui.argos = Some(if m.app {
                            topmenu::ArgosMenu::new(at, ww, wh)
                        } else {
                            topmenu::ArgosMenu::app(at, ww, wh)
                        });
                    }
                }
                return self.redraw();
            }
        }
        // fenêtre ouverte (Panneaux, Compte) : elle reçoit le clic (à côté = fermer)
        self.ui.sheet_closed = None;
        if pressed {
            if let Some(sh) = self.ui.sheet.take() {
                let (wx, wy) = self.pointer.win_mouse;
                match sh.click(wx, wy) {
                    SheetClick::Close => {
                        self.ui.sheet_closed = Some(sh.kind);
                        // ✕ dans la fenêtre : seulement fermer ; à côté : fermer et le clic continue
                        if sh.rect.contains(wx, wy) {
                            return self.redraw();
                        }
                    }
                    SheetClick::None => {
                        self.ui.sheet = Some(sh);
                        return self.redraw();
                    }
                    SheetClick::Act(a) => return self.sheet_act(a),
                }
            }
        }
        // barre du haut
        if let Some((tx, ty)) = self.pointer.top_mouse {
            if pressed {
                let text = argos_render::text::Text::default();
                let w = self.top_w();
                let flags = self.watch.prefs.flags.clone();
                let flagged = |c: &str| flags.iter().any(|(f, _)| f == c);
                let c = self.top.click(tx, ty, w, &text, &flagged);
                return self.top_click_do(c);
            }
            return self.redraw();
        }
        // 📱 barre du bas du cell
        if let Some((px, py)) = self.pointer.phone_mouse {
            if pressed {
                let e = self.cell_etat();
                if let Some((hit, r)) = argos_render::ecrans::hit(self.win_w(), self.cell_h(), &e, px, py) {
                    // 🎡 la pilule « symbole · unité » : la roue attend un glisser (sinon le toucher, au relâcher)
                    if self.roue_poser(hit, r) {
                        return self.redraw();
                    }
                    return self.cell_click(hit, r);
                }
            }
            return self.redraw();
        }
        // barre de gauche
        if let Some((rx, ry)) = self.pointer.rail_mouse {
            if pressed {
                if let Some((b, r)) = rail::hit(rx, ry, self.area.1 as f64) {
                    return self.rail_click(b, r);
                }
            }
            return;
        }
        // watchlist, barre d'icônes et menu ⠿
        if let Some((sx, sy)) = self.pointer.side_mouse {
            // bord gauche du panneau : on l'attrape pour changer la largeur
            // 🖥 volet flottant : sa hauteur à lui (ses coordonnées depuis son coin)
            let hv = self.volet_rect().filter(|_| sx < self.watch.pw()).map(|v| v.h);
            if pressed && self.watch.edge_rect(hv.unwrap_or(self.area.1 as f64)).is_some_and(|r| r.contains(sx, sy)) {
                self.pointer.wl_resize = Some((self.pointer.win_mouse.0, self.watch.pw()));
                return;
            }
            if pressed {
                // (📱 feuille du bas : sa hauteur à elle, comme au dessin — sinon le bas de la feuille était mal touché)
                let h = self.side_h(sx);
                // ↕️ appui sur un bouton de la barre d'icônes : peut-être le début d'un déplacement
                self.pointer.icon_drag = self
                    .watch
                    .tool_at(sx, sy)
                    .map(|(i, _)| (i, sy, false, (self.watch.prefs.open, self.watch.prefs.panel)));
                match self.watch.click(sx, sy, h) {
                    wl::Click::Open(coin) => return self.open_coin(coin),
                    wl::Click::Dock => Self::write_prefs("dock.json", &self.watch.dock),
                    wl::Click::Changed => {
                        // 📰 ⚙ Actualités (sources, langue, ⚡ sur la chart) : gardé
                        self.actualites_garder();
                        // 🐋 réglage du panneau Baleines (période, seuil…) : gardé
                        if self.watch.prefs.open && self.watch.prefs.panel == wl::Tool::Whales {
                            self.save_whales(true);
                        }
                        self.save_watch();
                        if self.watch.prefs.panel == wl::Tool::Journal {
                            self.journal_ecrire();
                        }
                    }
                    wl::Click::Journal(c) => self.journal_click(c),
                    wl::Click::Explorer(c) => return self.explorer_click(c),
                    wl::Click::Lists(r) => return self.open_list(ListKind::Lists, self.side_rect(r)),
                    wl::Click::TousBourse(r) => return self.open_list(ListKind::TousBourse, self.side_rect(r)),
                    wl::Click::TousTri(r) => return self.open_list(ListKind::TousTri, self.side_rect(r)),
                    wl::Click::Favori(coin) => self.ajouter_favori(&coin),
                    wl::Click::Liste(id) => return self.lists_pick(id, Rect::new(0.0, 0.0, 1.0, 1.0)),
                    wl::Click::CleCmc => self.cmc_key_box(),
                    // (📱 cell : la feuille est déjà pleine largeur, et la colonne de modules est celle du grand écran)
                    wl::Click::AgrandirVolet | wl::Click::AncrerVolet(_) if self.top.phone => {
                        self.banner = Some("⬓ Module et volet large : sur grand écran (PC, tablette en paysage)".into());
                    }
                    wl::Click::AgrandirVolet => {
                        self.ui.volet_large = !self.ui.volet_large;
                        let l = self.ui.volet_large;
                        Self::interface2_changer(|i| i.volet_large = l);
                    }
                    wl::Click::AncrerVolet(m) => self.volet_en_module(m),
                    wl::Click::FermerFeuille => {
                        self.watch.prefs.open = false;
                        self.save_watch();
                    }
                    wl::Click::Add(_) => {
                        // the panel takes the place of the list under the watchlist's header, where the panel's image
                        // is in the window (B-017, Jee 25 Nov 2026: it was laid from under the top bar whatever the
                        // panel, so on the phone it stopped half way down the sheet and cut its last result over the
                        // list behind it, and on the PC it went past the bottom of the floating volet and of the
                        // window)
                        let l = self.watch.list_rect(h);
                        let sheet = self.ui.rows.filter(|_| self.top.phone);
                        let (ox, oy) = match (self.volet_rect().filter(|_| sx < self.watch.pw()), sheet) {
                            (Some(v), _) => (v.x, v.y),
                            (None, Some((_, bot))) => (self.side_x(), self.top_h() + bot),
                            (None, None) => (self.side_x(), self.top_h()),
                        };
                        let mut r = Rect::new(ox + l.x, oy + l.y, l.w, l.h);
                        // 📱 the bottom sheet (not a dual screen's lower segment: nothing across the hinge): up to
                        // under the top bar, the field and the first results above the phone's keyboard
                        if sheet.is_some_and(|(top, bot)| bot <= top) {
                            let haut = self.top_h() + l.y;
                            r = Rect::new(r.x, haut, r.w, r.y + r.h - haut);
                        }
                        self.ui.add = Some(AddPanel::new(r, self.add_chips()));
                        self.catalogues_pour_recherche();
                        self.search_add();
                    }
                    wl::Click::Remove(coin) => {
                        if let (Some(l), Some(st)) = (self.watch.prefs.list, self.store.as_mut()) {
                            let _ = st.remove_item(l, &coin);
                        }
                        self.refresh_watch();
                    }
                    wl::Click::SectionMenu(name, r) => {
                        self.ui.section = Some(name);
                        return self.open_list(ListKind::Section, self.side_rect(r));
                    }
                    wl::Click::RemoveSection(name) => {
                        if let (Some(l), Some(st)) = (self.watch.prefs.list, self.store.as_mut()) {
                            let _ = st.remove_section(l, &name);
                        }
                        self.refresh_watch();
                    }
                    wl::Click::Indicator(i) => self.toggle_indicator(i),
                    wl::Click::IndSettings(i) => self.open_ind_dlg(i),
                    wl::Click::Marketplace => self.open_market(),
                    wl::Click::Academy => return self.open_academy(),
                    wl::Click::AcademyNews(row) => return self.open_academy_news(row),
                    wl::Click::Movers => {
                        let (x, y) = self.pointer.win_mouse;
                        return self.open_list(ListKind::Movers, Rect::new(x - 120.0, y, 240.0, 1.0));
                    }
                    wl::Click::Script(o) => self.open_editor(o),
                    wl::Click::Ai => return self.toggle_ai(),
                    wl::Click::OpenUrl(link) => platform::open_url(&link),
                    wl::Click::Ticket => self.drawing_from_ticket(),
                    wl::Click::Order => self.place_demo_order(),
                    wl::Click::Simulateur => return self.toggle_simulateur(),
                    // 🖥 🎫 Ticket over the Simulator in the Trading volet: the ticket comes back in its place
                    wl::Click::FermerSimulateur => (self.ui.sim, self.watch.panels.trading.sim_onglet) = (None, false),
                    wl::Click::CloseAll => self.close_demo_positions(),
                    wl::Click::GuardCfg => self.journal_ecrire(),
                    wl::Click::Unlock => self.guard_unlock_demander(),
                    wl::Click::CancelOrder(id) => self.cancel_demo_order(id),
                    wl::Click::Accounts => self.open_list(ListKind::Accounts, Rect::new(self.win_w() - self.watch.pw(), 50.0, self.watch.pw(), 1.0)),
                    wl::Click::Soon(msg) => self.banner = Some(msg),
                    wl::Click::Exchange(a) => self.exchange_action(a),
                    wl::Click::Outil(t, r) => return self.outil_barre(t, self.side_rect(r)),
                    // 🖥 › en bas de la colonne : la barre se replie (la languette › la rouvre)
                    // 🖥 interface-2 : la colonne repliée en bande / gardée ouverte (interface-2.json) ; sinon comme avant
                    wl::Click::ReplierBarre if self.watch.col_outils.is_some() => {
                        let r = self.watch.col_repliee;
                        Self::interface2_changer(|i| i.colonne_repliee = r);
                    }
                    wl::Click::ReplierBarre => self.save_watch(),
                    // 🖥 ⚙ en bas de la colonne : « Personnaliser la barre » (le même menu que le clic droit)
                    wl::Click::PersonnaliserBarre(r) => return self.open_list(ListKind::IconBar(None), self.side_rect(r)),
                    // 🎫 module 31: the ticket's order, a field to type, ⌖, « Carnet › », the % slider held
                    wl::Click::Passer(achat) => self.passer_ticket(achat),
                    wl::Click::Saisir(c) => self.saisir_ticket(c),
                    wl::Click::Cible => self.ticket_viser(),
                    wl::Click::Carnet => self.ticket_carnet(),
                    wl::Click::Curseur => self.pointer.curseur_ticket = Some(self.pointer.win_mouse.0 - sx),
                    // 🎯 the Stratégie volet: the full report, TradingView's file, the settings, another strategy
                    wl::Click::Strategie(c) => {
                        self.pointer.side_mouse = None;
                        self.strategie_click(c);
                        return self.redraw();
                    }
                    // 📖 the order-book volet: its menus open next to their pill
                    wl::Click::CarnetPanneau(c) => {
                        self.pointer.side_mouse = None;
                        let c = self.carnet_dans_fenetre(c, |a, r| match a.volet_rect() {
                            Some(v) => Rect::new(v.x + r.x, v.y + r.y, r.w, r.h),
                            None => a.side_rect(r),
                        });
                        self.carnet_click(c);
                        return self.redraw();
                    }
                    wl::Click::None => {}
                }
                // le panneau a pu s'ouvrir / se fermer : la souris est peut-être ailleurs maintenant
                self.pointer.side_mouse = None;
            }
            return self.redraw();
        }
        let (mx, my) = self.pointer.mouse.unwrap_or((-1.0, -1.0));
        // barre flottante
        if pressed && self.ui.goto.is_none() && !self.ui.context.is_open() {
            let text = argos_render::text::Text::default();
            // fenêtre des outils / des unités de temps ouverte : elle reçoit le clic (à côté = fermer)
            if self.pop_click(mx, my) {
                return;
            }
            if !self.pilule() && toolbar::contains(&self.tools, &text, mx, my) {
                match toolbar::hit(&self.tools, &text, mx, my) {
                    Some(Btn::Handle) => return self.tool_click(Btn::Handle, mx, my),
                    Some(b) => {
                        self.pointer.bar_press = Some(BarPress {
                            btn: b,
                            at: (mx, my),
                            since: self.heure_geste(),
                            mode: PressMode::Wait,
                        })
                    }
                    None => {}
                }
                return;
            }
        }
        // « Aller à » ouvert : il reçoit le clic (dehors = fermer)
        if pressed && self.ui.goto.is_some() {
            let tz = self.tz_offset_s();
            let vp = self.chart.viewport;
            let out = self.ui.goto.as_mut().map_or(Outcome::Close, |g| g.click(&vp, mx, my, tz));
            return self.goto_outcome(out);
        }
        if pressed && self.ui.context.is_open() {
            // menu ouvert : une ligne active fait son action ; à côté, le menu se ferme
            let ctx = self.ctx();
            let mut cm = self.ui.context;
            let click = cm.click(&self.chart.viewport, &ctx, mx, my);
            self.ui.context = cm;
            if let Click::Action(a) = click {
                self.run_action(a);
            }
            return self.redraw();
        }
        if pressed && context_menu::on_plus(&self.chart.viewport, &self.chart.settings, self.pointer.mouse, mx, my) {
            return self.open_menu(MenuSource::Plus, Anchor::Below(my));
        }
        if pressed && !self.top.phone && self.chart.viewport.footer > 0 && context_menu::on_gear(&self.chart.viewport, mx, my) {
            return self.open_menu(MenuSource::PriceAxis, Anchor::Gear);
        }
        // ⏪ the replay bar (📱 a main pane too narrow for it: it lies over the other panes, Suivi 7)
        if pressed && self.replay.is_some() {
            let (zone_w, ph) = self.replay_zone();
            if let Some(c) = argos_render::replay::click(zone_w, ph, mx, my) {
                return self.replay_act(c);
            }
        }
        // 🪟 une autre case du split (hors de la chart principale)
        if pressed && (mx >= self.chart.viewport.width as f64 || my >= self.chart.viewport.height as f64) && mx >= 0.0 && my >= 0.0 && self.pane_click(mx, my) {
            return;
        }
        // 🖍 the bar of the selected drawing (above the chart, the legend and the pill)
        if pressed && self.drawbar_click(mx, my) {
            return;
        }
        // 🏷️ légende des indicateurs (⌄ N, 👁 ⚙️ 🗑) ; un clic ailleurs replie la liste, comme v1 (👆 not for a finger
        // sliding from it: `legende_pas_un_toucher`)
        if pressed && !self.pointer.touch.legende_muette() && mx < self.chart.viewport.plot_w() && my < self.chart.viewport.plot_h() {
            let text = argos_render::text::Text::default();
            let cell = argos_render::indleg::Cell {
                actif: self.top.phone,
                sel: self.ind.sel,
                pw: self.chart.viewport.plot_w(),
            };
            let pw = self.chart.viewport.plot_w();
            let hit = argos_render::indleg::click_de(
                &text,
                &self.ind.on,
                &self.ind.out,
                self.ind.leg.0,
                self.chart.viewport.plot_h(),
                pw,
                cell,
                mx,
                my,
            );
            if let Some(c) = hit {
                return self.legend_click(c);
            }
            // 📱 un toucher ailleurs : la barre d'actions se ferme d'abord
            if self.ind.sel.take().is_some() {
                return self.redraw();
            }
            if self.ind.leg.0 {
                self.ind.leg.0 = false;
                self.legende_garder();
            }
        }
        // 🔍 the navigation pill (always visible): − + ‹ › ↺ ↶ ↷, Zen and ⏪ Replay in one group
        // (📱 phone and tablet: no navigation pill — pinch / drag, Replay in ⚙)
        if pressed && !self.top.phone {
            use argos_render::navbar::NavBtn;
            match self.pad_bouton(mx, my) {
                Some(NavBtn::Action(a)) => return self.run_action(a),
                Some(NavBtn::Zen) => return self.toggle_zen(),
                Some(NavBtn::Replay) => return self.replay_start(),
                None => {}
            }
        }
        // 🖥 pied flottant du cadre PC (au survol) : les mêmes actions
        if pressed && self.pied_click() {
            return self.redraw();
        }
        // barre du bas : périodes et « Aller à » (ailleurs dans la barre : rien, pas de glisser)
        if my > self.chart.viewport.plot_h() + self.chart.viewport.time_axis as f64 {
            match (pressed, bottombar::hit(&self.chart.viewport, self.pointer.mouse, self.chart_demo().as_deref())) {
                (true, Some(Hit::Range(r))) => {
                    let i = r.start_index(&self.chart.bars, self.tz_offset_s());
                    self.run_action(ChartAction::ShowRange(i, usize::MAX));
                }
                (true, Some(Hit::GoTo)) => self.run_action(ChartAction::GoTo),
                (true, Some(Hit::Demo)) => return self.demo_pastille_toucher(),
                (true, Some(Hit::Sessions)) => {
                    if let Some(r) = bottombar::sessions_rect(&self.chart.viewport) {
                        return self.open_list(ListKind::Timezone, self.win(r, self.rw()));
                    }
                }
                _ => {}
            }
            return;
        }
        // ⏪ replay : le choix de la bougie de départ
        if pressed && self.replay.is_some() {
            let (pw, ph) = (self.chart.viewport.plot_w(), self.chart.viewport.plot_h() - self.ind.out.osc_h());
            if self.replay.as_ref().is_some_and(|r| r.pick) && mx >= 0.0 && mx <= pw && my <= ph {
                if let Some(t) = self
                    .chart
                    .viewport
                    .index_at(mx, &self.chart.slots)
                    .and_then(|i| self.chart.bars.get(i))
                    .map(|b| b.time)
                {
                    if let Some(rp) = self.replay.as_mut() {
                        (rp.t, rp.pick) = (t, false);
                    }
                    self.banner = Some("✂️ Départ choisi : ▶ pour jouer".into());
                    self.rebuild();
                    return self.redraw();
                }
            }
        }
        // outil choisi dans la barre de gauche : gomme, ligne horizontale (les autres : moteur d'objets, étape suivante)
        if pressed && self.tool != "cursor" && mx >= 0.0 && my <= self.chart.viewport.plot_h() && mx <= self.chart.viewport.plot_w() {
            let now = self.heure_geste();
            // (✏️ B-019: where a finger's drag lifts lays the next point)
            let double = self.est_double(self.pointer.last_chart_click, now);
            self.pointer.last_chart_click = Some(now);
            self.pointer.trace_doigt = self.pointer.appui_glisse && self.tool != "erase";
            return self.use_tool(mx, my, double);
        }
        // 📍 choix demandé par la fenêtre de réglages : ce clic donne le prix et l'heure, la fenêtre revient
        if pressed && self.ui.ind_here.is_some() && mx >= 0.0 && mx <= self.chart.viewport.plot_w() && my >= 0.0 && my <= self.chart.viewport.plot_h() {
            if let (Some((mut dl, i)), Some(f), Some(sc)) = (self.ui.ind_here.take(), self.frame_now(), self.scale_now()) {
                dl.here(i, sc.price_of(my), f.geo().t_of(mx) as f64 * 1000.0);
                self.banner = None;
                self.ind_dlg_result(dl, argos_render::inddlg::IndClick::Changed);
            }
            return self.redraw();
        }
        // 📍 repère d'input : relâcher = garder la valeur ; appuyer dessus = l'attraper (avant les dessins)
        if !pressed && self.pointer.mark_drag.take().is_some() {
            self.garder_reglages();
            self.set_cursor(CursorIcon::Grab);
            return self.redraw();
        }
        // ↕️ bandes d'oscillateurs : glisser le bord du haut = agrandir / rapetisser ; double-clic sur le bord =
        // hauteurs par défaut (comme v1) ; double-clic dans la bande = agrandie ⇄ normale
        if !pressed
            && (self.pointer.band_drag.take().is_some()
                | self.pointer.script_drag.take().is_some()
                | self.pointer.band_pan.take().is_some()
                | self.pointer.band_zoom.take().is_some()
                | self.pointer.script_pan.take().is_some()
                | self.pointer.script_zoom.take().is_some())
        {
            self.set_cursor(CursorIcon::Default);
            return self.redraw();
        }
        // 📜 a script pane's value column: drag = zoom of the pane; double click / double tap = its view reset
        if pressed && mx > self.chart.viewport.plot_w() {
            if let Some(key) = self.script_pane_at(my, true) {
                let now = self.heure_geste();
                if self.est_double(self.pointer.last_axis_click, now) {
                    self.chart.settings.script_view.remove(&key);
                    self.pointer.last_axis_click = None;
                } else {
                    self.pointer.last_axis_click = Some(now);
                    let z0 = self.chart.settings.script_view.get(&key).map_or(1.0, |v| v.1);
                    self.pointer.script_zoom = Some((key, my, z0));
                }
                return self.redraw();
            }
        }
        // colonne de valeurs d'une bande : glisser = zoom de la bande ; double-clic = sa vue remise à zéro
        if pressed && mx > self.chart.viewport.plot_w() {
            if let Some(ind) = self
                .ind
                .out
                .band_at(self.chart.viewport.plot_h(), my)
                .or_else(|| self.ind.out.band_edge(self.chart.viewport.plot_h(), my))
            {
                let now = self.heure_geste();
                if self.est_double(self.pointer.last_axis_click, now) {
                    self.chart.settings.band_view.remove(&ind);
                    self.pointer.last_axis_click = None;
                } else {
                    self.pointer.last_axis_click = Some(now);
                    let z0 = self.chart.settings.band_view.get(&ind).map_or(1.0, |v| v.1);
                    self.pointer.band_zoom = Some((ind, my, z0));
                }
                return self.redraw();
            }
        }
        if pressed && mx >= 0.0 && mx <= self.chart.viewport.plot_w() && my >= 0.0 && my <= self.chart.viewport.plot_h() {
            let now = self.heure_geste();
            // ⛶ 🖥 the third click: full screen of the panel or the chart under the mouse
            if self.triple_clic(now) {
                return;
            }
            // (✏️ B-019: a double tap on a drawing is the drawing's — its settings —, not the chart's full screen)
            let double = self.est_double(self.pointer.last_chart_click, now) && !self.sur_un_dessin(mx, my);
            // ⛶ full screen: a double-click / double tap anywhere on it comes back
            if double && self.ui.plein_ecran.is_some() {
                self.pointer.last_chart_click = None;
                return self.quitter_plein_ecran();
            }
            // ⛶ 📱 phone and tablet: a double tap on the chart or a panel puts it full screen
            if double && self.top.phone && self.plein_possible() {
                self.pointer.last_chart_click = None;
                return self.entrer_plein_ecran(self.plein_cible_en(my));
            }
        }
        if pressed && mx >= 0.0 && mx <= self.chart.viewport.plot_w() {
            let ph = self.chart.viewport.plot_h();
            let now = self.heure_geste();
            let double = self.est_double(self.pointer.last_chart_click, now);
            if self.script_bord_appui(my) {
                return self.redraw();
            }
            if let Some(ind) = self.ind.out.band_edge(ph, my) {
                self.pointer.last_chart_click = Some(now);
                if double {
                    self.chart.settings.bands.clear();
                    self.banner = Some("↕️ Hauteurs des indicateurs remises par défaut".into());
                } else {
                    self.pointer.band_drag = Some((ind, my, ind.band_h()));
                    self.set_cursor(CursorIcon::RowResize);
                }
                return self.redraw();
            }
            if let Some(ind) = self.ind.out.band_at(ph, my).filter(|_| double) {
                self.pointer.last_chart_click = None;
                // (⛶ a third click undoes this and puts the panel full screen)
                let avant = self.chart.settings.bands.get(&ind).copied();
                use crate::actions::plein_ecran::{Defaire, PleinEcran};
                self.pointer.triple = self.plein_possible().then_some((now, Defaire::Panneau(ind, avant), PleinEcran::Panneau(ind)));
                let big = (ph * 0.45).clamp(argos_render::indicators::BAND_MIN, argos_render::indicators::BAND_MAX);
                if ind.band_h() >= big - 1.0 {
                    self.chart.settings.bands.remove(&ind);
                } else {
                    self.chart.settings.bands.insert(ind, big);
                }
                return self.redraw();
            }
            // dans la bande : on glisse l'indicateur dans sa case (pas la chart)
            if let Some(ind) = self.ind.out.band_at(ph, my) {
                self.pointer.last_chart_click = Some(now);
                let off0 = self.chart.settings.band_view.get(&ind).map_or(0.0, |v| v.0);
                self.pointer.band_pan = Some((ind, mx, my, off0));
                self.set_cursor(CursorIcon::Grabbing);
                return self.redraw();
            }
            // 📜 in a script's pane: the same, the script moves up / down in it
            if let Some(key) = self.script_pane_at(my, false) {
                self.pointer.last_chart_click = Some(now);
                let off0 = self.chart.settings.script_view.get(&key).map_or(0.0, |v| v.0);
                self.pointer.script_pan = Some((key, mx, my, off0));
                self.set_cursor(CursorIcon::Grabbing);
                return self.redraw();
            }
        }
        if pressed {
            if let Some(m) = self.frame_now().and_then(|f| argos_render::inmark::hit(&self.ind.out, &f.geo(), mx, my)) {
                self.pointer.mark_drag = Some(m);
                self.set_cursor(CursorIcon::Grabbing);
                return self.redraw();
            }
        }
        // relâcher : on lâche l'objet ; appuyer sur une poignée / un objet : on l'attrape et le sélectionne
        if !pressed {
            if let Some(g) = self.pointer.grab.take() {
                let (Grab::Move { i, .. } | Grab::Handle { i, .. }) = g;
                if self.chart.drawings.get(i).is_some_and(|d| argos_render::position::is_position(&d.tool)) {
                    self.ticket_from_drawing(i); // 💼 le ticket suit la position déplacée
                }
                // (lâché : le curseur de ce qui est sous la souris, la main qui pointe sur l'objet)
                let c = self.curseur_chart(mx, my);
                self.poser_curseur(c);
                return self.redraw();
            }
        }
        if pressed {
            let grab = match self.handle_at(mx, my) {
                Some((i, h)) => Some(Grab::Handle { i, h }),
                None => self.drawing_at(mx, my).map(|i| Grab::Move {
                    i,
                    x0: mx,
                    y0: my,
                    from: self.chart.drawings[i].pts.clone(),
                }),
            };
            // ✏️ (B-019) by the finger, a drag moves only the drawing already selected (tapped first, as on
            // TradingView): over another one the chart moves — with a finger's reach, a drag of the chart near a
            // drawing would carry it away
            let grab = grab.filter(|g| {
                let (Grab::Move { i, .. } | Grab::Handle { i, .. }) = g;
                !self.pointer.appui_glisse || self.selected == Some(*i)
            });
            if let Some(g) = grab {
                let i = match g {
                    Grab::Move { i, .. } | Grab::Handle { i, .. } => i,
                };
                // double-clic sur le dessin : sa fenêtre de réglages (comme Argos v1) ; (✏️ B-019) un doigt qui glisse
                // déplace l'objet, il n'ouvre jamais ses réglages (`est_double`)
                let now = self.heure_geste();
                let double = self.selected == Some(i) && self.est_double(self.pointer.last_chart_click, now);
                self.pointer.last_chart_click = Some(now);
                self.selected = Some(i);
                if double {
                    self.open_obj_dlg(i, None);
                    return self.redraw();
                }
                if !self.chart.drawings[i].locked() {
                    self.pointer.grab = Some(g);
                    self.set_cursor(CursorIcon::Grabbing);
                }
                return self.redraw();
            }
            self.selected = None;
        }
        let on_axis = self
            .pointer
            .mouse
            .is_some_and(|(x, y)| x > self.chart.viewport.plot_w() && y < self.chart.viewport.plot_h());
        (self.pointer.drag, self.pointer.axis_drag) = (None, None);
        // ≫ F1j: a double tap / click on the time axis goes back to live (like the ≫ button)
        let vp = self.chart.viewport;
        let on_time = self
            .pointer
            .mouse
            .is_some_and(|(x, y)| x >= 0.0 && x <= vp.plot_w() && y > vp.plot_h() && y <= vp.plot_h() + vp.time_axis as f64);
        if pressed && on_time {
            let now = self.heure_geste();
            if self.est_double(self.pointer.last_time_click, now) {
                self.pointer.last_time_click = None;
                self.run_action(ChartAction::GoEnd);
                return self.redraw();
            }
            self.pointer.last_time_click = Some(now);
        }
        if pressed && on_axis {
            let now = self.heure_geste();
            if self.est_double(self.pointer.last_axis_click, now) {
                // double-clic : retour à l'ajustement automatique
                self.run_action(ChartAction::ResetPrice);
            }
            self.pointer.last_axis_click = Some(now);
            self.pointer.axis_drag = self.pointer.mouse.map(|m| m.1);
        } else if pressed {
            // double-clic sur la chart : mode libre (au démarrage, toute la chart bouge) ⇄ mode normal (prix auto)
            let now = self.heure_geste();
            if self.est_double(self.pointer.last_chart_click, now) {
                self.pointer.last_chart_click = None;
                self.pointer.drag = None;
                // (⛶ a third click undoes this and puts the chart full screen)
                use crate::actions::plein_ecran::{Defaire, PleinEcran};
                self.pointer.triple = self.plein_possible().then_some((now, Defaire::Echelle, PleinEcran::Prix));
                self.run_action(ChartAction::AutoScale);
                self.banner = Some(if self.chart.settings.scale.auto() {
                    "📐 Prix automatique : la chart bouge dans le temps, le prix se recadre seul · double-clic pour la main libre".into()
                } else {
                    "✋ Main libre : glisse la chart dans tous les sens (temps et prix) · double-clic ou ↺ RAZ pour le prix automatique".into()
                });
                if let Some((mx, my)) = self.pointer.mouse {
                    let c = self.curseur_chart(mx, my);
                    self.poser_curseur(c);
                }
                return self.redraw();
            }
            self.pointer.last_chart_click = Some(now);
            self.pointer.drag = self.pointer.mouse;
            // (✊ la main fermée vient quand la souris glisse, clic tenu : comme TradingView)
        } else {
            // relâché : le curseur de ce qui est sous la souris (le curseur choisi au repos)
            if let Some((mx, my)) = self.pointer.mouse {
                let c = self.curseur_chart(mx, my);
                self.poser_curseur(c);
            }
            self.redraw(); // relâché : sauvegarde
        }
    }

    pub(crate) fn mouse_wheel(&mut self, delta: MouseScrollDelta) {
        let steps = match delta {
            MouseScrollDelta::LineDelta(_, y) => y as f64,
            MouseScrollDelta::PixelDelta(p) => p.y / 60.0,
        };
        // 🧱 fenêtre modale ouverte : la molette ne va qu'à elle (hors d'elle : rien, jamais à la chart)
        if self.modale_ouverte() {
            let (wx, wy) = self.pointer.win_mouse;
            let r = self.ui.ind_dlg.as_ref().map(|d| d.rect).or_else(|| self.ui.obj_dlg.as_ref().map(|d| d.rect));
            if !r.is_some_and(|r| r.contains(wx, wy)) {
                // 🛟 the Support panel scrolls
                if let Some(g) = self.ui.support.as_mut().filter(|g| g.rect().contains(wx, wy)) {
                    g.wheel(-steps * 40.0);
                }
                return self.redraw();
            }
        }
        // 🔎 the market panel (PC) or its sheet (phone): its list scrolls, so its last row (CALCULÉE) comes above
        // « ⚙ Tous les filtres »
        {
            let (wx, wy) = self.pointer.win_mouse;
            if let Some(m) = self.ui.marches.as_mut().filter(|m| m.rect.contains(wx, wy)) {
                m.wheel(-steps * 40.0);
                return self.redraw();
            }
        }
        // 📱 the open sheet of the single pill scrolls (pulled down at its top: it closes)
        if self.ui.pfeuille.is_some() {
            return self.pfeuille_defiler(-steps * 40.0);
        }
        // 🧩 molette sur un module de la colonne : son contenu défile
        if let Some((x, y)) = self.pointer.dock_mouse {
            self.dock_molette(x, y, steps);
            return self.redraw();
        }
        // 🗺 plan de l'app : la page défile
        if self.ui.plan.is_some() {
            let (w, h) = (self.win_w(), self.top_h() + self.area.1 as f64 + argos_render::ecrans::dock_h());
            if let Some(p) = self.ui.plan.as_mut() {
                p.wheel(-steps * 40.0, w, h);
            }
            return self.redraw();
        }
        // ⚙ fenêtre Réglages : la liste défile
        if let Some(g) = self
            .ui
            .reglages
            .as_mut()
            .filter(|g| g.rect().contains(self.pointer.win_mouse.0, self.pointer.win_mouse.1))
        {
            g.wheel(-steps * 40.0);
            return self.redraw();
        }
        // ⚡ the sheet of the chart's news: its list scrolls
        if self.ui.eclair.as_ref().is_some_and(|e| e.feuille) {
            let (w, h) = self.taille_points();
            let text = argos_render::text::Text::default();
            if let Some(e) = self.ui.eclair.as_mut() {
                e.wheel(&text, -steps * 40.0, w, h);
            }
            return self.redraw();
        }
        // 🔔 feuille des notifications : le fil défile
        if self.notifs_sous(self.pointer.win_mouse.0, self.pointer.win_mouse.1) {
            // 🔔 the alerts panel: its list scrolls
            if self.ui.notifs.as_ref().is_some_and(|n| !n.fil) {
                let d = self.alertes_donnees();
                let (w, h) = (self.win_w(), self.top_h() + self.area.1 as f64 + argos_render::ecrans::dock_h());
                if let Some(n) = self.ui.notifs.as_mut() {
                    let r = n.alertes_place(w, h);
                    n.alertes.wheel(-steps * 40.0, r, &d);
                }
                return self.redraw();
            }
            let ev = self.notif_evenements();
            if let Some(p) = self.ui.notifs.as_mut() {
                p.wheel(-steps * 40.0, &ev);
            }
            return self.redraw();
        }
        // 🔎 volet Symboles : la liste défile
        if let Some(v) = self.sym_rect().filter(|v| v.contains(self.pointer.win_mouse.0, self.pointer.win_mouse.1)) {
            let d = self.sym_donnees();
            if let Some(p) = self.ui.symboles.as_mut() {
                p.wheel(-steps * 40.0, v.w, v.h, &d);
            }
            return self.redraw();
        }
        // 🅘 page Indicateurs : la liste défile
        let hors_du_volet = self.fx_rect().is_some_and(|v| !v.contains(self.pointer.win_mouse.0, self.pointer.win_mouse.1));
        if self.ui.fx_page.is_some() && !hors_du_volet {
            let h = self
                .fx_rect()
                .map_or(self.top_h() + self.area.1 as f64 + argos_render::ecrans::dock_h(), |v| v.h);
            let d = self.fx_donnees();
            if let Some(p) = self.ui.fx_page.as_mut() {
                p.wheel(-steps * 40.0, h, &d);
            }
            return self.redraw();
        }
        // 😀 molette (ou doigt) sur le sélecteur d'émojis : la grille défile
        if let (Some(p), Some((mx, my))) = (self.ui.pop.as_mut().and_then(|p| p.pick.as_mut()), self.pointer.mouse) {
            if p.rect.contains(mx, my) {
                p.wheel(-steps * 40.0);
                return self.redraw();
            }
        }
        // molette (ou doigt) sur la fenêtre Panneaux ou le menu des dispositions plus hauts que l'écran : ils défilent
        let (wx, wy) = self.pointer.win_mouse;
        if let Some(s) = self.ui.sheet.as_mut().filter(|s| s.rect.contains(wx, wy)) {
            s.wheel(-steps * 40.0);
            return self.redraw();
        }
        if let Some(m) = self.ui.split_menu.as_mut().filter(|m| m.rect.contains(wx, wy)) {
            m.wheel(-steps * 40.0);
            return self.redraw();
        }
        // molette (ou doigt) sur le menu Argos ▾ / ☰ Appli plus haut que l'écran : il défile
        if let Some(m) = self
            .ui
            .argos
            .as_mut()
            .filter(|m| m.rect.contains(self.pointer.win_mouse.0, self.pointer.win_mouse.1))
        {
            m.wheel(-steps * 40.0);
            return self.redraw();
        }
        // 📅 the month report (over the Argos AI card): it scrolls
        if self.month_wheel(-steps * 40.0) {
            return;
        }
        // molette sur l'assistant : la conversation défile
        if self.ui.ai_open && self.ai_rect().contains(self.pointer.win_mouse.0, self.pointer.win_mouse.1) {
            self.ai.wheel(-steps * 40.0);
            return self.redraw();
        }
        // 🔎 molette sur la recherche : la liste défile ; ⌂ sur l'Accueil : la page ; ⌨️ sur le volet des raccourcis
        if let Some(p) = self.ui.palette.as_mut() {
            p.molette(if steps > 0.0 { -1 } else { 1 });
            return self.redraw();
        }
        if self.ui.accueil.is_some() {
            let cartes = argos_render::accueil::cartes(&self.accueil_donnees());
            let size = self.window.as_deref().map(ui_size).unwrap_or_default();
            if let Some(p) = self.ui.accueil.as_mut() {
                p.wheel(
                    -steps * 60.0,
                    &argos_render::text::Text::default(),
                    size.width as f64,
                    size.height as f64,
                    &cartes,
                );
            }
            return self.redraw();
        }
        if self.sim_wheel(-steps * 40.0) {
            return;
        }
        // 🤖 la molette sur la carte Argos AI d'un trade ne fait pas bouger la chart dessous
        if self.avis_sous(self.pointer.win_mouse.0, self.pointer.win_mouse.1) || self.fp_sous(self.pointer.win_mouse.0, self.pointer.win_mouse.1) {
            return;
        }
        let rr = self.raccourcis_rect();
        if let Some(p) = self
            .ui
            .raccourcis
            .as_mut()
            .filter(|_| rr.contains(self.pointer.win_mouse.0, self.pointer.win_mouse.1))
        {
            p.molette(-steps * 40.0, rr.h);
            return self.redraw();
        }
        // molette sur l'Académie : la page défile
        if let Some(p) = self.ui.ac_page.as_mut() {
            let size = self.window.as_deref().map(ui_size).unwrap_or_default();
            p.wheel(-steps * 60.0, &argos_render::text::Text::default(), size.width as f64, size.height as f64);
            return self.redraw();
        }
        // 💼 molette (ou doigt) sur le Portfolio : la page défile (cell : blocs empilés)
        if let Some(p) = self.ui.pf_page.as_mut() {
            let size = self.window.as_deref().map(ui_size).unwrap_or_default();
            p.wheel(-steps * 60.0, &argos_render::text::Text::default(), size.width as f64, size.height as f64);
            return self.redraw();
        }
        // 📊 wheel (or finger) on the backtest report: the page, then its table
        if self.ui.bt_page.is_some() {
            self.rapport_molette(-steps * 60.0);
            return self.redraw();
        }
        // molette sur le Marketplace : la grille défile
        if let Some(m) = self.ui.mk_page.as_mut() {
            let size = self.window.as_deref().map(ui_size).unwrap_or_default();
            m.wheel(-steps * 60.0, size.width as f64, size.height as f64, &self.ind.on);
            return self.redraw();
        }
        // molette sur l'éditeur : le code défile (3 lignes par cran) ; 📖 la Doc défile au point près, Maj + molette fait
        // glisser un bloc de code de côté
        let er = self.ed_rect();
        if er.contains(self.pointer.win_mouse.0, self.pointer.win_mouse.1) {
            let (x, y) = (self.pointer.win_mouse.0 - er.x, self.pointer.win_mouse.1 - er.y);
            let maj = self.pointer.mods.shift_key();
            if let Some(e) = self.ui.editor.as_mut() {
                if !(maj && e.doc_ouverte && e.doc.pan(x, y, -steps * 40.0)) {
                    e.molette(steps);
                }
                return self.redraw();
            }
        }
        // molette sur la fenêtre de réglages d'un indicateur : la liste des inputs défile
        if let Some(dl) = self
            .ui
            .ind_dlg
            .as_mut()
            .filter(|d| d.rect.contains(self.pointer.win_mouse.0, self.pointer.win_mouse.1))
        {
            dl.wheel(-steps * 42.0, &argos_render::text::Text::default());
            return self.redraw();
        }
        // molette sur l'arborescence : elle défile
        let n = self.chart.drawings.len();
        if let Some(t) = self
            .ui
            .obj_tree
            .as_mut()
            .filter(|t| t.rect.contains(self.pointer.win_mouse.0, self.pointer.win_mouse.1))
        {
            t.wheel(steps, n);
            return self.redraw();
        }
        // molette sur le panneau ＋ : les résultats défilent
        if let Some(p) = self
            .ui
            .add
            .as_mut()
            .filter(|p| p.rect.contains(self.pointer.win_mouse.0, self.pointer.win_mouse.1))
        {
            p.wheel(steps);
            return self.redraw();
        }
        // molette sur le menu en liste : il défile
        if let Some((_, m)) = self
            .ui
            .list
            .as_mut()
            .filter(|(_, m)| m.rect.contains(self.pointer.win_mouse.0, self.pointer.win_mouse.1))
        {
            m.wheel(steps);
            return self.redraw();
        }
        // molette sur la watchlist : elle défile
        if let Some((sx, _)) = self.pointer.side_mouse {
            if self.watch.prefs.open && sx < self.watch.pw() {
                // 📱 feuille du bas : sa hauteur à elle (comme au dessin), sinon la fin du volet restait hors d'atteinte
                // 🖥 volet flottant : sa hauteur à lui
                let h = match self.volet_rect() {
                    Some(v) => v.h,
                    None => self.ui.rows.map_or(self.area.1 as f64, |(_, bot)| self.area.1 as f64 - bot),
                };
                self.watch.wheel(steps, h);
            }
            return self.redraw();
        }
        // molette sur un long sous-menu (ex. fuseaux) : il défile
        if let Some((mx, my)) = self.pointer.mouse.filter(|_| self.ui.context.is_open()) {
            let ctx = self.ctx();
            let mut cm = self.ui.context;
            let used = cm.scroll(&self.chart.viewport, &ctx, mx, my, steps);
            self.ui.context = cm;
            if used {
                return self.redraw();
            }
        }
        // 🪟 molette sur une autre case du split : zoom de cette case seulement
        let (cx, cy) = (self.pointer.win_mouse.0 - self.rw(), self.pointer.win_mouse.1 - self.top_h());
        if self.pointer.side_mouse.is_none() && (cx >= self.chart.viewport.width as f64 || cy >= self.chart.viewport.height as f64) {
            if let Some((k, lx, _, w, h)) = self.pane_at(cx, cy) {
                return self.pane_wheel(k, steps, lx, w, h);
            }
        }
        // molette sur la colonne de prix : zoom du prix (vers le haut = plus près) ; sur la colonne d'une bande
        // d'oscillateur : zoom de cette bande
        if let Some((_, my)) = self
            .pointer
            .mouse
            .filter(|(mx, my)| *mx > self.chart.viewport.plot_w() && *my < self.chart.viewport.plot_h())
        {
            let k = (-steps * 0.1).exp();
            if let Some(ind) = self
                .ind
                .out
                .band_at(self.chart.viewport.plot_h(), my)
                .or_else(|| self.ind.out.band_edge(self.chart.viewport.plot_h(), my))
            {
                let (off, z) = self.chart.settings.band_view.get(&ind).copied().unwrap_or((0.0, 1.0));
                self.chart.settings.band_view.insert(ind, (off, (z * k).clamp(0.05, 20.0)));
            } else if let Some(key) = self.script_pane_at(my, true) {
                // 📜 a script's pane: the same (the wheel, two fingers pinching on its value column)
                let (off, z) = self.chart.settings.script_view.get(&key).copied().unwrap_or((0.0, 1.0));
                self.chart.settings.script_view.insert(key, (off, (z * k).clamp(0.05, 20.0)));
            } else if let Some(sc) = self.scale_now() {
                let (mid, half) = ((sc.min + sc.max) / 2.0, (sc.max - sc.min) / 2.0 * k);
                self.chart.settings.scale.manual = Some((mid - half, mid + half));
                if let Some(r) = self.chart.settings.scale.lock_ratio {
                    self.chart.settings.scale.lock_ratio = Some(r * k);
                }
            }
            return self.redraw();
        }
        let x = self.pointer.mouse.map_or(self.chart.viewport.plot_w(), |m| m.0);
        self.chart.viewport.zoom_at(1.12f64.powf(steps), x, &self.chart.slots);
        self.redraw();
    }
}
