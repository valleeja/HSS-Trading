//! 🖱️ ⌨️ Événements de la fenêtre : l'aiguillage vers la souris (`mouse`) et le clavier (`keyboard`).

mod focus;
mod keyboard;
mod modules;
mod mouse;
pub(crate) mod touch;

use focus::Focus;

use crate::*;

impl App {
    /// 🖱 The pointer is a real mouse: no finger for 800 ms (the browser's « mouse » events right after a touch are
    /// the finger's) and none just lifted — help bubbles show on hover only, never on a touch (phone or tablet).
    pub(crate) fn souris_seule(&self) -> bool {
        #[cfg(target_arch = "wasm32")]
        let vu = platform::web::doigt_vu();
        #[cfg(not(target_arch = "wasm32"))]
        let vu = f64::NEG_INFINITY;
        !self.pointer.doigt_leve && touch::now_ms() - self.pointer.dernier_doigt.max(vu) >= 800.0
    }

    pub(crate) fn on_window_event(&mut self, el: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        // navigateur : positions en pixels de la page (comme `ui_size`), et le doigt fait la souris
        #[cfg(target_arch = "wasm32")]
        let event = {
            // la position notée par la page (pixels de la page) ; sinon celle de winit ramenée à la page
            // 📐 au doigt sur tablette, l'interface est agrandie : on ramène les positions à sa taille
            let k = self.window.as_ref().map_or(1.0, |w| w.scale_factor()) * platform::web::view().0;
            let at = |p: winit::dpi::PhysicalPosition<f64>| {
                let (x, y) = platform::web::pointer().unwrap_or((p.x / k, p.y / k));
                winit::dpi::PhysicalPosition::new(x, y)
            };
            // 👆 juste après le doigt : pas de « souris » (le navigateur la place à l'ancienne position du doigt)
            let doigt = touch::now_ms() - self.pointer.dernier_doigt.max(platform::web::doigt_vu()) < 800.0;
            match event {
                WindowEvent::CursorMoved { .. } | WindowEvent::CursorLeft { .. } | WindowEvent::MouseInput { .. } if doigt => return,
                WindowEvent::CursorMoved { device_id, position } => WindowEvent::CursorMoved {
                    device_id,
                    position: at(position),
                },
                // 👆 les doigts sont lus par la page (`platform::web::listen_touch`) : winit perd les mouvements du doigt
                // sur Safari iOS (liste « coalescée » vide), et rien ne défilait
                WindowEvent::Touch(_) => return,
                e => e,
            }
        };
        // 🖥 desktop app: positions in points (window pixels / the compact factor)
        #[cfg(not(target_arch = "wasm32"))]
        let event = match event {
            WindowEvent::CursorMoved { device_id, position } => {
                let k = argos_render::px::scale() as f64;
                WindowEvent::CursorMoved {
                    device_id,
                    position: winit::dpi::PhysicalPosition::new(position.x / k, position.y / k),
                }
            }
            e => e,
        };
        let _ = id;
        // 🐢 un geste : les données en direct redessinent à nouveau à chaque arrivée
        if matches!(
            event,
            WindowEvent::CursorMoved { .. }
                | WindowEvent::MouseInput { .. }
                | WindowEvent::MouseWheel { .. }
                | WindowEvent::KeyboardInput { .. }
                | WindowEvent::Touch(_)
                | WindowEvent::Resized(_)
                | WindowEvent::Focused(_)
        ) {
            self.ui.derniere_entree = Instant::now();
        }
        match event {
            WindowEvent::CloseRequested => {
                self.save();
                self.write_cache(); // garde aussi les bougies du direct
                self.save_watch();
                self.save_whales(true);
                el.exit()
            }
            WindowEvent::Resized(_) => self.redraw(),
            WindowEvent::RedrawRequested => self.draw(),
            WindowEvent::CursorMoved { position, .. } => {
                self.pointer.doigt_leve = false;
                self.mouse_moved(position);
            }
            WindowEvent::CursorLeft { .. } => {
                self.mouse_left();
            }
            // 🖱 fenêtre ou onglet quitté pendant un glisser : le geste s'arrête (plus de main fermée collée)
            WindowEvent::Focused(false) if self.pointer.drag.is_some() || self.pointer.grab.is_some() => {
                self.fin_du_glisser();
                self.redraw();
            }
            // clic droit : le menu de ce qui est sous la souris
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Right,
                ..
            } => {
                // (boîte de clé API ou des 24 mots ouverte : rien derrière)
                if self.session.key_box.is_none() && self.session.words_box.is_none() {
                    self.right_click();
                }
            }
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => {
                self.left_button(state);
            }
            // (boîte de clé ouverte : la molette fait défiler la chart et les volets comme d'habitude ; boîte des 24 mots : rien)
            WindowEvent::MouseWheel { delta, .. } if self.session.words_box.is_none() => {
                self.mouse_wheel(delta);
            }
            WindowEvent::Touch(t) => self.touch(t.id, t.phase, (t.location.x, t.location.y)),
            WindowEvent::ModifiersChanged(m) => self.pointer.mods = m.state(),
            // ⌨️ Ctrl / Alt / Maj suivis tout de suite : dans le navigateur, winit annonce le changement *après* la
            // touche (Ctrl + K pouvait arriver sans Ctrl, et Ctrl restait « collé » : les lettres tapées ensuite
            // étaient jetées par la recherche)
            // 🖥 Échap pendant un geste de la colonne de modules : il est annulé
            WindowEvent::KeyboardInput { ref event, .. }
                if event.state == ElementState::Pressed && matches!(event.logical_key, Key::Named(NamedKey::Escape)) && self.dock.geste.is_some() =>
            {
                self.dock_annuler();
                self.redraw();
            }
            WindowEvent::KeyboardInput { ref event, .. } if Self::mod_of(&event.logical_key).is_some() => {
                let m = Self::mod_of(&event.logical_key).unwrap_or_default();
                self.pointer.mods.set(m, event.state == ElementState::Pressed);
            }
            // ⌨️ une touche va à une seule fenêtre : la première ouverte dans l'ordre de `Focus::pick`
            // 🔎 Ctrl + K (recherche) et ⌨️ Ctrl + / (raccourcis) : partout, sauf pendant la saisie d'une clé
            WindowEvent::KeyboardInput { event, .. }
                if event.state == ElementState::Pressed && self.session.key_box.is_none() && self.global_key(&event).is_some() =>
            {
                match self.global_key(&event) {
                    Some(true) => self.open_palette(),
                    _ => self.toggle_raccourcis(),
                }
            }
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => match self.focus(&event.logical_key) {
                Focus::KeyBox => self.key_box(event),
                Focus::Palette => self.key_palette(event),
                Focus::Marches => self.key_marches(event),
                Focus::Accueil => self.key_accueil(&event.logical_key),
                Focus::Raccourcis => {
                    self.sim_echap();
                    self.ui.raccourcis = None;
                    self.redraw();
                }
                Focus::Assistant => self.key_assistant(event),
                Focus::Support => self.key_support(event),
                Focus::Editor => self.key_editor(event),
                Focus::Market => self.key_market(event),
                Focus::Academy => {
                    if matches!(event.logical_key, Key::Named(NamedKey::Escape)) && self.ui.ac_page.as_mut().is_some_and(|p| p.escape()) {
                        self.ui.ac_page = None;
                    }
                    self.redraw();
                }
                Focus::Portfolio => self.key_portfolio(event),
                Focus::Journal => self.key_journal(event),
                Focus::IndHere => self.key_ind_here(),
                Focus::IndDlg => self.key_ind_dlg(event),
                Focus::ObjDlg => self.key_obj_dlg(event),
                Focus::DrawAlert => self.key_draw_alert(event),
                Focus::NameBox => self.key_name_box(event),
                Focus::IndSearch => self.key_ind_search(&event.logical_key, event.text.as_deref()),
                Focus::EmojiSearch => self.key_emoji_search(&event.logical_key, event.text.as_deref()),
                Focus::FxSearch => self.key_fx_search(&event.logical_key, event.text.as_deref()),
                Focus::GoTo => self.key_goto(event),
                Focus::Actu => self.key_actu(&event.logical_key, event.text.as_deref()),
                Focus::Hl => self.key_hl(&event.logical_key, event.text.as_deref()),
                Focus::Alertes => self.key_alertes(&event.logical_key, event.text.as_deref()),
                Focus::Chart => self.key_chart(&event.logical_key),
            },
            _ => {}
        }
    }
}

impl App {
    /// La touche Ctrl, Alt ou Maj (`None` pour les autres).
    pub(crate) fn mod_of(k: &Key) -> Option<winit::keyboard::ModifiersState> {
        use winit::keyboard::ModifiersState as M;
        match k {
            Key::Named(NamedKey::Control) => Some(M::CONTROL),
            Key::Named(NamedKey::Alt) => Some(M::ALT),
            Key::Named(NamedKey::Shift) => Some(M::SHIFT),
            _ => None,
        }
    }

    /// Raccourcis de toute l'app : `Some(true)` Ctrl + K, `Some(false)` Ctrl + / (touche « / » ou sa place sur un
    /// clavier QWERTY, pour les claviers où « / » demande Maj).
    pub(crate) fn global_key(&self, event: &winit::event::KeyEvent) -> Option<bool> {
        use winit::keyboard::{KeyCode, PhysicalKey};
        if !self.pointer.mods.control_key() || self.pointer.mods.alt_key() {
            return None;
        }
        match &event.logical_key {
            Key::Character(c) if c.eq_ignore_ascii_case("k") => Some(true),
            Key::Character(c) if c.as_str() == "/" => Some(false),
            _ if event.physical_key == PhysicalKey::Code(KeyCode::Slash) => Some(false),
            _ => None,
        }
    }

    /// 👆 Un doigt se pose, bouge ou se lève : les gestes reconnus sont joués avec les fonctions de la souris.
    pub(crate) fn touch(&mut self, id: u64, phase: winit::event::TouchPhase, pos: (f64, f64)) {
        self.touch_at(id, phase, pos, touch::now_ms());
    }

    /// 👆 The same, done by the finger at `te` (its own time on `touch::now_ms`'s clock, read by the page with the touch:
    /// B-048). The gestures' timers (a swipe's speed, the fling, a double tap, the floating bar's held press) count with
    /// it, so a touch read late, after a slow image, keeps its real time; the long press counts from the reading (B-047).
    pub(crate) fn touch_at(&mut self, id: u64, phase: winit::event::TouchPhase, (x, y): (f64, f64), te: f64) {
        use winit::event::TouchPhase;
        let now = touch::now_ms();
        // (never later than now: a clock that does not match is read as now)
        let te = if te.is_finite() && te <= now { te } else { now };
        // 👆 aucune image encore dessinée (le navigateur a retardé le premier dessin) : l'app ne connaît pas encore la
        // vraie mise en page (elle se croit sur PC) ; on dessine d'abord, sinon le premier toucher tombait à côté
        if !self.ui.depart_vu && self.window.is_some() {
            self.draw();
        }
        self.pointer.dernier_doigt = now;
        let t = &mut self.pointer.touch;
        let gestures = match phase {
            TouchPhase::Started => t.down_at(id, x, y, now, te),
            TouchPhase::Moved => t.moved(id, x, y),
            TouchPhase::Ended => t.up(id, x, y, te),
            TouchPhase::Cancelled => t.cancel(),
        };
        let lu = web_time::Instant::now();
        self.pointer.t_geste = Some(lu.checked_sub(web_time::Duration::from_secs_f64((now - te) / 1000.0)).unwrap_or(lu));
        self.play(gestures);
        self.pointer.t_geste = None;
        // 👆 doigt levé : l'état « survolé » se relâche (cell et tablette)
        self.pointer.doigt_leve = matches!(phase, TouchPhase::Ended | TouchPhase::Cancelled);
        if self.pointer.doigt_leve {
            self.redraw();
        }
    }

    /// 🏷️ By the finger, the press about to be played (the mouse already put there) is on a pane's legend button
    /// (👁 ⚙ { } …) but is no real tap: the finger slides (`next` is a move) or lifted [`touch::TAP_STILL`] points or
    /// more from where it went down. Only the finger's gestures come here: the mouse is never concerned.
    fn legende_pas_un_toucher(&self, next: Option<touch::Gesture>) -> bool {
        let Some((x, y)) = self.pointer.mouse else { return false };
        let (wx, wy) = self.pointer.win_mouse;
        if self.couche_haute(wx, wy) || self.legende_bouton(x, y).is_none() {
            return false;
        }
        match next {
            Some(touch::Gesture::Move(..)) => true,
            Some(touch::Gesture::Release(..)) => self.pointer.touch.tap_moved() >= touch::TAP_STILL,
            _ => false,
        }
    }

    /// 👆 (B-048) The time of the gesture being played (milliseconds, `touch::now_ms`'s clock): the finger's own time
    /// when a finger's gesture is played, else now (the mouse, a long press).
    pub(crate) fn heure_geste_ms(&self) -> f64 {
        touch::now_ms() - self.pointer.t_geste.map_or(0.0, |t| t.elapsed().as_secs_f64() * 1000.0)
    }

    /// 👆 (B-048) The same as an `Instant`, the clicks' clock (a double tap, the floating bar's held press).
    pub(crate) fn heure_geste(&self) -> web_time::Instant {
        self.pointer.t_geste.unwrap_or_else(web_time::Instant::now)
    }

    /// 👆 Survol à montrer : aucun juste après un doigt levé.
    pub(crate) fn survol(&self, p: Option<(f64, f64)>) -> Option<(f64, f64)> {
        p.filter(|_| !self.pointer.doigt_leve)
    }

    /// 👆 (B-047) A round of work of the event loop begins (`new_events`): noted, so that a long press whose time comes
    /// during that work waits one more look (`touch_tick`).
    pub(crate) fn debut_du_tour(&mut self) {
        self.pointer.tour_debut = Some(touch::now_ms());
    }

    /// 👆 Appui long : appelé quand la fenêtre se réveille ; l'heure du prochain réveil, si un doigt attend encore.
    pub(crate) fn touch_tick(&mut self) -> Option<web_time::Instant> {
        let now = touch::now_ms();
        // 👆 B-047: the long press's time came while the app was busy (an image drawn, data read): the finger may have
        // lifted meanwhile, its lift waiting in the browser behind that work (winit looks here right after the image,
        // before the page reads the lift). One more look first, a short wait later ([`touch::RELIRE_MS`]), whatever
        // order the browser keeps: the lift, already waiting, is read in that wait and makes a tap; a finger still down
        // makes its long press at that look
        let debut = self.pointer.tour_debut.take();
        if let (Some(d), Some(at)) = (debut, self.pointer.touch.long_at()) {
            if d < at && now >= at {
                return Some(web_time::Instant::now() + web_time::Duration::from_secs_f64(touch::RELIRE_MS / 1000.0));
            }
        }
        let g = self.pointer.touch.tick(now);
        self.play(g);
        self.pointer
            .touch
            .long_at()
            .map(|at| web_time::Instant::now() + web_time::Duration::from_secs_f64(((at - now) / 1000.0).max(0.0)))
    }

    /// 👆 Sous le doigt (déjà placé par `mouse_moved`) : une zone qui défile (panneau, liste, menu, fenêtre), pas la
    /// chart ? Le doigt qui glisse la fait alors défiler au lieu de cliquer / glisser.
    pub(crate) fn touch_scrolls(&self) -> bool {
        let (wx, wy) = self.pointer.win_mouse;
        let ui = &self.ui;
        ui.pfeuille.is_some()
            // 🔎 the market panel's sheet: the finger scrolls its list
            || ui.marches.as_ref().is_some_and(|m| m.rect.contains(wx, wy))
            || ui.list.as_ref().is_some_and(|(_, m)| m.rect.contains(wx, wy))
            || ui.argos.as_ref().is_some_and(|m| m.rect.contains(wx, wy))
            || ui.sheet.as_ref().is_some_and(|s| s.rect.contains(wx, wy))
            || ui.split_menu.as_ref().is_some_and(|m| m.rect.contains(wx, wy))
            || ui.ind_dlg.as_ref().is_some_and(|d| d.rect.contains(wx, wy))
            || ui.add.as_ref().is_some_and(|p| p.rect.contains(wx, wy))
            || ui.obj_tree.as_ref().is_some_and(|t| t.rect.contains(wx, wy))
            || ui.context.is_open()
            || (ui.pop.as_ref().is_some_and(|p| p.pick.is_some())
                && self.pointer.mouse.is_some_and(|(x, y)| ui.pop.as_ref().is_some_and(|p| p.rect.contains(x, y))))
            || (ui.ai_open && self.ai_rect().contains(wx, wy))
            // 🧪 le volet du Simulateur défile au doigt (Jee, 20 nov. : sinon on ne voit pas le bas), 📅 its month report too
            || (ui.sim.is_some() && self.sim_rect().contains(wx, wy))
            || self.month_under(wx, wy)
            || ui.mk_page.is_some()
            || ui.fx_page.is_some()
            || self.sym_rect().is_some_and(|r| r.contains(wx, wy))
            || self.notifs_sous(wx, wy)
            || ui.reglages.as_ref().is_some_and(|g| g.rect().contains(wx, wy))
            || ui.plan.is_some()
            || ui.ac_page.is_some()
            || ui.accueil.is_some()
            || ui.palette.is_some()
            || ui.pf_page.is_some()
            || ui.bt_page.is_some()
            || (ui.editor.is_some() && self.ed_rect().contains(wx, wy))
            // (🎚 not on the Trading ticket's % slider: the finger drags it)
            || (self.pointer.side_mouse.is_some_and(|(sx, sy)| self.watch.tool_at(sx, sy).is_none() && !self.watch.sur_curseur_ticket(sx, sy, self.side_h(sx)))
                && self.watch.prefs.open)
            || self.pointer.dock_mouse.is_some()
    }

    /// 👆 Sous le doigt : la barre des favoris qui déborde de la chart (hors poignée, qui sert à la déplacer) ?
    fn bar_swipes(&self) -> bool {
        let text = argos_render::text::Text::default();
        self.pointer.mouse.is_some_and(|(x, y)| {
            toolbar::max_scroll(&self.tools, &text) > 0.0
                && toolbar::contains(&self.tools, &text, x, y)
                && toolbar::hit(&self.tools, &text, x, y) != Some(Btn::Handle)
                && !self.ui.context.is_open()
        })
    }

    /// 👆 Défile de `dy` points (doigt vers le haut : positif) là où le doigt s'est posé.
    fn finger_wheel(&mut self, x: f64, y: f64, dy: f64) {
        self.mouse_moved(winit::dpi::PhysicalPosition::new(x, y));
        // 40 points par cran de molette (listes, panneaux) : le contenu suit le doigt
        self.mouse_wheel(winit::event::MouseScrollDelta::PixelDelta(winit::dpi::PhysicalPosition::new(0.0, -dy * 1.5)));
    }

    /// 👆 Élan après un défilement au doigt : appelé à chaque image tant qu'il reste de la vitesse.
    pub(crate) fn fling_step(&mut self) {
        let (v, x, y) = self.pointer.fling;
        if v.abs() < 0.02 || self.pointer.finger_scroll.is_some() {
            self.pointer.fling.0 = 0.0;
            return;
        }
        self.pointer.fling.0 = v * 0.94;
        self.finger_wheel(x, y, v * 16.0);
    }

    fn play(&mut self, gestures: Vec<touch::Gesture>) {
        use touch::Gesture as G;
        let at = |x: f64, y: f64| winit::dpi::PhysicalPosition::new(x, y);
        for (i, &g) in gestures.iter().enumerate() {
            // 📋 a table selected by the finger takes every gesture (the chart neither pans nor zooms); a tap on one
            // selects it
            if self.tab_geste(g, gestures.get(i + 1).copied()) {
                continue;
            }
            // 👆 défilement au doigt en cours : le doigt fait défiler, il ne clique pas
            if let Some((sx, sy, ly, lt)) = self.pointer.finger_scroll {
                match g {
                    G::Move(x, y) => {
                        // (B-048: the finger's own time, the speed of moves read together after a slow image is true)
                        let now = self.heure_geste_ms();
                        let dy = ly - y;
                        // 📖 on a code block of the Doc, a mostly sideways move slides the block (no wrap)
                        if let Some(lx) = self.pointer.doc_pan {
                            self.pointer.doc_pan = Some(x);
                            if (lx - x).abs() > dy.abs() {
                                let er = self.ed_rect();
                                if let Some(e) = self.ui.editor.as_mut() {
                                    e.doc.pan(sx - er.x, sy - er.y, lx - x);
                                }
                                self.pointer.finger_scroll = Some((sx, sy, y, now));
                                self.redraw();
                                continue;
                            }
                        }
                        self.finger_wheel(sx, sy, dy);
                        let v = dy / (now - lt).max(1.0);
                        self.pointer.fling = (self.pointer.fling.0 * 0.6 + v * 0.4, sx, sy);
                        // (B-048: a move that does not move — the lift's own, a pressure change — keeps the time of
                        // the last real move: a finger held still before lifting has no fling)
                        let lt = if dy == 0.0 { lt } else { now };
                        self.pointer.finger_scroll = Some((sx, sy, y, lt));
                        continue;
                    }
                    G::Release(..) => {
                        self.pointer.finger_scroll = None;
                        self.pointer.doc_pan = None;
                        // doigt immobile avant de se lever : pas d'élan (B-048 : à l'heure du doigt, pas de la lecture)
                        if self.heure_geste_ms() - lt > 80.0 {
                            self.pointer.fling.0 = 0.0;
                        }
                        self.redraw();
                        continue;
                    }
                    _ => {}
                }
            }
            // 👆 la barre des favoris défile de gauche à droite sous le doigt (cell : plus longue que la chart)
            if let Some(lx) = self.pointer.bar_swipe {
                match g {
                    G::Move(x, _) => {
                        let text = argos_render::text::Text::default();
                        let max = toolbar::max_scroll(&self.tools, &text);
                        self.tools.scroll = (self.tools.scroll + lx - x).clamp(0.0, max);
                        self.pointer.bar_swipe = Some(x);
                        self.redraw();
                        continue;
                    }
                    G::Release(..) => {
                        self.pointer.bar_swipe = None;
                        self.redraw();
                        continue;
                    }
                    _ => {}
                }
            }
            match g {
                // 👆 glisser sur la barre des favoris (pas sur sa poignée) : elle défile au lieu de cliquer
                G::Press(x, y)
                    if matches!(gestures.get(i + 1), Some(G::Move(..))) && {
                        self.mouse_moved(at(x, y));
                        self.bar_swipes()
                    } =>
                {
                    self.pointer.bar_swipe = Some(x);
                }
                // le doigt glisse (un mouvement suit l'appui) sur une zone qui défile : défilement, pas de clic
                G::Press(x, y)
                    if matches!(gestures.get(i + 1), Some(G::Move(..))) && {
                        self.mouse_moved(at(x, y));
                        self.touch_scrolls()
                    } =>
                {
                    self.pointer.finger_scroll = Some((x, y, y, self.heure_geste_ms()));
                    self.pointer.fling = (0.0, x, y);
                    // (🧱 F1j: a drag closes the tapped tooltip and never opens one)
                    argos_render::murs::set_bulle(None);
                    self.pointer.murs_appui = None;
                    let er = self.ed_rect();
                    let sur_code = self
                        .ui
                        .editor
                        .as_ref()
                        .is_some_and(|e| e.doc_ouverte && e.doc.bloc_sous(x - er.x, y - er.y).is_some());
                    self.pointer.doc_pan = sur_code.then_some(x);
                }
                // 🏷️ a legend button acts only on a real tap (the finger lifted where it went down): a finger that
                // slides from it catches a pane's edge when it went down within its reach, else it is played as before
                // but past the legend (the chart scrolls…)
                G::Press(x, y)
                    if {
                        self.mouse_moved(at(x, y));
                        self.legende_pas_un_toucher(gestures.get(i + 1).copied())
                    } =>
                {
                    if !(matches!(gestures.get(i + 1), Some(G::Move(..))) && self.edge_grab()) {
                        self.pointer.touch.set_legende_muette(true);
                        self.left_button(ElementState::Pressed);
                        self.pointer.touch.set_legende_muette(false);
                    }
                }
                G::Move(x, y) => self.mouse_moved(at(x, y)),
                G::Press(x, y) => {
                    self.mouse_moved(at(x, y));
                    // (✏️ B-019: a press that starts a drag, not a tap)
                    self.pointer.appui_glisse = matches!(gestures.get(i + 1), Some(G::Move(..)));
                    self.left_button(ElementState::Pressed);
                    self.pointer.appui_glisse = false;
                }
                G::Release(x, y) => {
                    self.mouse_moved(at(x, y));
                    self.left_button(ElementState::Released);
                    // ✏️ B-019: a drawing tool's point laid where the finger went down, the drag ends where it lifts:
                    // its next point (a line, a box, a Fibonacci retracement finished by the drag alone; before, it
                    // waited for one more touch, and the touch meant to select it finished it there, crushed)
                    if std::mem::take(&mut self.pointer.trace_doigt) && self.draft.is_some() {
                        let vp = self.chart.viewport;
                        let (mx, my) = ((x - self.rw()).clamp(0.0, vp.plot_w()), (y - self.top_h()).clamp(0.0, vp.plot_h()));
                        self.use_tool(mx, my, false);
                    }
                    // 🧱 F1j: the finger lifts where it went down on a wall pill or depth bar (moved < 8 points): its
                    // tooltip opens; and no hover is left where the finger was
                    if let Some((ax, ay, cx, cy)) = self.pointer.murs_appui.take() {
                        if (x - ax).hypot(y - ay) < 8.0 {
                            argos_render::murs::set_bulle(Some((cx, cy)));
                        }
                    }
                    if argos_render::actions::tactile() && self.ui.croix.is_none() {
                        self.pointer.mouse = None;
                    }
                    self.redraw();
                }
                G::RightClick(x, y) => {
                    self.mouse_moved(at(x, y));
                    // ↕️ a long press on a pane's top edge catches the edge (no crosshair, no menu): the finger then
                    // makes the pane taller or shorter, lifting it lets go
                    if self.edge_grab() {
                        self.pointer.touch.grab();
                        self.redraw();
                    } else {
                        self.right_click();
                    }
                }
                // 👆 the crosshair follows the finger held down after the long press
                G::Hold(x, y) => {
                    if self.ui.croix.is_some() {
                        self.mouse_moved(at(x, y));
                        self.croix_poser();
                    }
                }
                // pincer : comme la molette au même endroit (sur l'échelle de prix : le prix, ailleurs : le temps)
                G::Pinch { cx, cy, sx, sy } => {
                    if self.ui.mk_page.is_some() {
                        continue;
                    }
                    self.mouse_moved(at(cx, cy));
                    argos_render::murs::set_bulle(None);
                    self.pointer.murs_appui = None;
                    let on_axis = self.pointer.mouse.is_some_and(|(mx, _)| mx > self.chart.viewport.plot_w());
                    let s = if on_axis { sy } else { sx };
                    if (s - 1.0).abs() > 1e-6 {
                        let steps = s.ln() / 1.12f64.ln();
                        self.mouse_wheel(winit::event::MouseScrollDelta::PixelDelta(at(0.0, steps * 60.0)));
                    }
                }
                G::Swipe { x, y, down } => self.swipe_close(x, y, down),
            }
        }
    }

    /// 👆 Coup de doigt vers la droite ou vers le bas : la page ouverte se ferme (Marketplace, Portfolio, éditeur,
    /// assistant, puis le panneau de droite si le doigt est parti dedans). Vers le bas : seulement en haut de la liste.
    fn swipe_close(&mut self, x: f64, y: f64, down: bool) {
        if self.ui.palette.is_some() {
            return; // 🔎 la recherche se ferme par Échap ou à côté
        }
        // 📅 the month report: a swipe to the right, or down from its top, closes it (the Simulator panel stays)
        if self.month_under(x, y) {
            if !down || self.ui.sim.as_ref().and_then(|s| s.month.as_ref()).is_some_and(|p| p.scroll <= 0.0) {
                self.month_close();
            }
            return self.redraw();
        }
        if let Some(p) = self.ui.plan.as_ref() {
            if !down || p.at_top() {
                self.ui.plan = None;
            }
            return self.redraw();
        }
        if let Some(p) = self.ui.accueil.as_ref() {
            if (!down || p.at_top()) && p.bienvenue.is_none() {
                self.ui.accueil = None;
            }
            return self.redraw();
        }
        if let Some(p) = self.ui.ac_page.as_ref() {
            if !down || p.at_top() {
                self.ui.ac_page = None;
            }
            return self.redraw();
        }
        if let Some(m) = self.ui.mk_page.as_ref() {
            if !down || m.at_top() {
                self.ui.mk_page = None;
            }
        } else if self.ui.pf_page.is_some() {
            self.ui.pf_page = None;
        } else if self.ui.bt_page.is_some() {
            self.ui.bt_page = None;
        } else if self.ui.editor.is_some() {
            return; // on écrit du code : pas de fermeture au doigt (bouton ✕ de l'éditeur)
        } else if self.ui.ai_open {
            return self.toggle_ai();
        } else if self.watch.prefs.open && down && self.top.phone {
            // 📱 glisser vers le bas depuis le haut de la feuille du bas (poignée, en-tête) : elle se referme (Jee, 20 nov.)
            if let Some((_, bas)) = self.ui.rows {
                let haut = self.top_h() + bas;
                if y >= haut - 12.0 && y <= haut + 48.0 {
                    self.watch.prefs.open = false;
                    self.save_watch();
                }
            }
        } else if self.watch.prefs.open && !down {
            let w = self.window.as_deref().map(ui_size).unwrap_or_default().width as f64;
            if x >= w - self.watch.pw() {
                self.watch.prefs.open = false;
                self.save_watch();
            }
        }
        self.redraw();
    }
}
