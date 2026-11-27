//! 🖍 The bar of the selected drawing (Jee's mockup, 17 Nov): where it sits, its clicks, its menus.

use crate::*;
use argos_render::colorpick::{ColorPick, CpClick};
use argos_render::drawbar::{self, Bar, DbBtn, DbPick};
use argos_render::objdlg::Which;
use argos_render::objmenu::ObjAction;

/// 🎨 The custom colours of the palette (« + »), kept for the whole app.
pub(crate) const COULEURS_PERSO: &str = "couleurs_perso.json";

impl App {
    /// The selected drawing and its bar (chart coordinates), when it shows: a drawing selected, not hidden, not being
    /// moved, its settings window closed.
    pub(crate) fn drawbar(&self) -> Option<(usize, Bar)> {
        let i = self.selected?;
        if self.ui.obj_dlg.is_some() || self.pointer.grab.is_some() || self.draft.is_some() || self.rail.hide_draw {
            return None;
        }
        // 🏠 filet n° 44: a full-screen page open (Accueil, Académie, Marketplace, PropFirm, Backtest, plan): the bar
        // is neither drawn over it nor given its clicks; the drawing stays selected, the bar comes back with the chart
        // (the Indicateurs page is a side panel on the PC: the chart and its bar stay)
        let u = &self.ui;
        if u.accueil.is_some() || u.ac_page.is_some() || u.mk_page.is_some() || u.pf_page.is_some() || u.bt_page.is_some() || u.plan.is_some() {
            return None;
        }
        let d = self.chart.drawings.get(i).filter(|d| !d.style.hidden)?;
        let f = self.frame_now()?;
        let hs = d.handles(&f.geo());
        if hs.is_empty() {
            return None;
        }
        let vp = &self.chart.viewport;
        let x = (hs.iter().map(|h| h.0).sum::<f64>() / hs.len() as f64).clamp(0.0, vp.plot_w());
        let y = hs.iter().map(|h| h.1).fold(f64::INFINITY, f64::min).clamp(0.0, vp.plot_h());
        let touch = self.top.phone || self.ui.form.tablet();
        Some((i, drawbar::layout(&d.tool, (x, y), self.ui.drawbar_off, vp.width as f64, vp.plot_h(), touch)))
    }

    /// A click (or touch) at (`x`, `y`), chart coordinates: `true` when the bar took it.
    pub(crate) fn drawbar_click(&mut self, x: f64, y: f64) -> bool {
        let Some((i, bar)) = self.drawbar() else { return false };
        let Some(b) = bar.hit(x, y) else {
            return bar.rect.contains(x, y);
        };
        match b {
            DbBtn::Handle => self.pointer.drawbar_drag = Some(((x, y), self.ui.drawbar_off)),
            DbBtn::Lock => self.obj_action(i, ObjAction::Lock),
            DbBtn::Trash => self.obj_action(i, ObjAction::Remove),
            DbBtn::Settings => self.obj_action(i, ObjAction::Settings),
            DbBtn::Alert => self.open_draw_alert(i),
            DbBtn::LineColor | DbBtn::TextColor => {
                let w = if b == DbBtn::TextColor { Which::Text } else { Which::Line };
                if self.ui.colorpick.as_ref().is_some_and(|c| c.1 == w) {
                    self.ui.colorpick = None; // 2nd click: closed
                } else {
                    let anchor = self.win(bar.at(b).unwrap_or(bar.rect), self.rw());
                    self.open_colours(i, w, anchor, false);
                }
            }
            DbBtn::Template | DbBtn::Width | DbBtn::Style | DbBtn::More => {
                let r = bar.at(b).unwrap_or(bar.rect);
                let anchor = self.win(r, self.rw());
                self.open_list(ListKind::DrawBar(i, b), anchor);
            }
        }
        self.redraw();
        true
    }

    /// 🔔 « Créer une alerte sur » drawing `i` (its line followed at each bar), on the symbol and timeframe shown.
    pub(crate) fn open_draw_alert(&mut self, i: usize) {
        let Some(d) = self.chart.drawings.get(i) else { return };
        let (Some(a), b) = (d.pts.first(), d.pts.get(1)) else { return };
        let line = match b.filter(|_| !matches!(d.tool.as_str(), "hline" | "hray" | "cross")) {
            Some(b) => argos_core::alerts::Src::Trend {
                t0: a.t,
                p0: a.p,
                t1: b.t,
                p1: b.p,
            },
            None => argos_core::alerts::Src::Trend {
                t0: a.t,
                p0: a.p,
                t1: a.t,
                p1: a.p,
            },
        };
        let name = if d.style.name.is_empty() {
            argos_render::tools::def(&d.tool).map_or(d.tool.clone(), |t| t.label.to_string())
        } else {
            d.style.name.clone()
        };
        let sources = self.alert_lines().into_iter().map(|(n, _)| (argos_core::alerts::line_id(&n), n)).collect();
        let (sym, tf) = (self.symbol_label(), self.chart.interval.secs());
        let (ww, wh) = (self.win_w(), self.area.1 as f64 + self.top_h());
        let w = argos_render::drawalert::DrawAlert::new(&name, line, &sym, tf, &self.chart.interval.short(), sources, now_s(), ww, wh);
        self.ui.list = None;
        self.ui.colorpick = None;
        self.ui.draw_alert = Some((i, w));
        self.redraw();
    }

    /// 🔔 A click while the alert window is open (window coordinates): `true` when it took it.
    pub(crate) fn draw_alert_press(&mut self, x: f64, y: f64) -> bool {
        let Some((i, mut w)) = self.ui.draw_alert.take() else { return false };
        match w.click(x, y) {
            argos_render::drawalert::DaClick::Cancel => {}
            argos_render::drawalert::DaClick::Create {
                kind,
                every,
                until,
                msg,
                lines,
                son,
            } => {
                let sym = self.symbol_label();
                let id = self.add_alert(&sym, kind, every, until, &msg);
                if let Some(a) = self.alerts.list.iter_mut().find(|a| a.id == id) {
                    (a.lines, a.son) = (lines, son);
                }
                self.save_alerts();
                platform::ask_notifications();
                self.banner = Some(format!("🔔 Alerte créée : {msg} (vérifiée à la clôture de chaque bougie, app ouverte)"));
            }
            // 🔔 a price alert edited (right-click on its line ▸ « Éditer… »): the same alert
            argos_render::drawalert::DaClick::Save { id, price, dir, msg, son } => self.alerte_enregistree(id, price, dir, msg, son),
            // 🔊 ▶ the chosen sound (the window stays open)
            argos_render::drawalert::DaClick::Ecouter(son) => {
                self.ecouter_son(son.as_deref());
                self.ui.draw_alert = Some((i, w));
            }
            argos_render::drawalert::DaClick::None => {
                let (ww, wh) = (self.win_w(), self.area.1 as f64 + self.top_h());
                w.refit(ww, wh);
                self.ui.draw_alert = Some((i, w));
            }
        }
        self.redraw();
        true
    }

    /// 🎨 Opens the shared palette for the colour `w` of drawing `i`, under `anchor` (window coordinates); with `line`,
    /// « Épaisseur » and « Style de ligne » under the colours (settings window).
    pub(crate) fn open_colours(&mut self, i: usize, w: Which, anchor: Rect, line: bool) {
        let Some(d) = self.chart.drawings.get(i) else { return };
        let st = &d.style;
        let (cur, def) = match w {
            Which::Line => (&st.color, "#2962ff"),
            Which::Text => (&st.tcolor, "#ffffff"),
            Which::Stop => (&st.stop_color, argos_render::position::DEFAULT_STOP),
            Which::Target => (&st.target_color, argos_render::position::DEFAULT_TARGET),
        };
        let cur = if cur.is_empty() { def.to_string() } else { cur.clone() };
        // the line's opacity shown on the slider
        let cur = match (w, argos_render::colorpick::split(&cur)) {
            (Which::Line, Some((rgb, _))) => argos_render::colorpick::join(rgb, st.opacity),
            _ => cur,
        };
        let dash = if st.dotted { 2 } else { u8::from(st.dash) };
        let line = line.then_some((st.width, dash));
        let custom: Vec<String> = Self::read_prefs(COULEURS_PERSO);
        self.ui.list = None;
        let (ww, wh) = (self.win_w(), self.area.1 as f64 + self.top_h());
        self.ui.colorpick = Some((i, w, ColorPick::with_line(anchor, &cur, custom, line, ww, wh)));
    }

    /// 🎨 A press while the palette is open (window coordinates): `true` when the palette took it.
    pub(crate) fn colorpick_press(&mut self, x: f64, y: f64) -> bool {
        let Some((i, which, mut cp)) = self.ui.colorpick.take() else { return false };
        match cp.click(x, y) {
            CpClick::Close => {
                // closed; on its own colour button the click stops here (a 2nd click closes), elsewhere it goes on
                let (cx, cy) = (x - self.rw(), y - self.top_h());
                let mine = if which == Which::Text { DbBtn::TextColor } else { DbBtn::LineColor };
                let again = matches!(which, Which::Line | Which::Text) && self.drawbar().is_some_and(|(_, bar)| bar.hit(cx, cy) == Some(mine));
                self.redraw();
                return again;
            }
            CpClick::Pick(c) => self.set_drawing_colour(i, which, c),
            CpClick::Width(w) => {
                if let Some(d) = self.chart.drawings.get_mut(i) {
                    d.style.width = w;
                }
            }
            CpClick::Dash(k) => {
                if let Some(d) = self.chart.drawings.get_mut(i) {
                    (d.style.dash, d.style.dotted) = (k == 1, k == 2);
                }
            }
            CpClick::Add => {
                let mut v: Vec<String> = Self::read_prefs(COULEURS_PERSO);
                let base = argos_render::colorpick::split(&cp.current).map(|p| argos_render::colorpick::join(p.0, 1.0));
                if let Some(c) = base.filter(|c| !v.contains(c)) {
                    v.push(c);
                    let n = v.len().saturating_sub(argos_render::colorpick::CUSTOM_MAX);
                    v.drain(..n);
                    Self::write_prefs(COULEURS_PERSO, &v);
                }
                cp.custom = v;
            }
            CpClick::None => {}
        }
        self.pointer.cp_drag = cp.on_slider(x, y);
        self.ui.colorpick = Some((i, which, cp));
        self.redraw();
        true
    }

    /// 🎨 The opacity slider dragged to `x` (window coordinates).
    pub(crate) fn colorpick_drag(&mut self, x: f64) -> bool {
        if !self.pointer.cp_drag {
            return false;
        }
        let Some((i, which, mut cp)) = self.ui.colorpick.take() else { return false };
        if let Some(c) = cp.drag(x) {
            self.set_drawing_colour(i, which, c);
        }
        self.ui.colorpick = Some((i, which, cp));
        self.redraw();
        true
    }

    pub(crate) fn set_drawing_colour(&mut self, i: usize, which: Which, c: String) {
        // 🔔 the palette open for the alerts: one colour for all alert lines (topbar.json)
        if i == crate::actions::alerts::COULEUR_ALERTES {
            if let Some((rgb, _)) = argos_render::colorpick::split(&c) {
                self.top.prefs.couleur_alertes = argos_render::colorpick::join(rgb, 1.0);
                self.save_top();
            }
            return;
        }
        if let Some(d) = self.chart.drawings.get_mut(i) {
            // (the colour always « #rrggbb », older versions read it; the opacity of the line in its own field)
            let Some((rgb, a)) = argos_render::colorpick::split(&c) else { return };
            let c = argos_render::colorpick::join(rgb, 1.0);
            match which {
                Which::Line => (d.style.color, d.style.opacity) = (c, a),
                Which::Text => d.style.tcolor = c,
                Which::Stop => d.style.stop_color = c,
                Which::Target => d.style.target_color = c,
            }
        }
    }

    /// The names of the saved templates of `tool`.
    pub(crate) fn template_names(&self, tool: &str) -> Vec<String> {
        self.templates.get(tool).map_or(vec![], |v| v.iter().map(|(n, _)| n.clone()).collect())
    }

    /// « Sauvegarder le modèle de dessin sous… » typed: the selected drawing's style kept under `name` (the same
    /// name replaces it).
    pub(crate) fn template_typed(&mut self, name: &str) {
        let name = name.trim();
        if let Some(d) = self.selected.and_then(|i| self.chart.drawings.get(i)).filter(|_| !name.is_empty()) {
            // (the templates of the settings window too: one list, templates.json)
            let v = self.templates.entry(d.tool.clone()).or_default();
            match v.iter_mut().find(|(n, _)| n == name) {
                Some(t) => t.1 = d.style.clone(),
                None => v.push((name.to_string(), d.style.clone())),
            }
            Self::write_prefs("templates.json", &self.templates);
            self.banner = Some(format!("Modèle « {name} » enregistré"));
        }
        self.redraw();
    }

    /// ✕ of a saved template: removed, the menu stays open.
    pub(crate) fn drawbar_remove(&mut self, i: usize, b: DbBtn, id: i64, anchor: Rect) {
        if let (Some(DbPick::RemoveTemplate(k)), Some(d)) = (drawbar::remove(b, id), self.chart.drawings.get(i)) {
            if let Some(v) = self.templates.get_mut(&d.tool) {
                if k < v.len() {
                    v.remove(k);
                }
            }
            Self::write_prefs("templates.json", &self.templates);
            self.ui.list = None;
            self.open_list(ListKind::DrawBar(i, b), anchor);
        }
        self.redraw();
    }

    /// The bar dragged by its handle: it follows the mouse (chart coordinates).
    pub(crate) fn drawbar_drag_to(&mut self, x: f64, y: f64) -> bool {
        let Some(((x0, y0), (ox, oy))) = self.pointer.drawbar_drag else {
            return false;
        };
        self.ui.drawbar_off = (ox + x - x0, oy + y - y0);
        self.redraw();
        true
    }

    /// A line of a bar menu chosen.
    pub(crate) fn drawbar_pick(&mut self, i: usize, b: DbBtn, id: i64) {
        let Some(p) = drawbar::pick(b, id) else { return };
        if let DbPick::Obj(a) = p {
            return self.obj_action(i, a);
        }
        // 📱 a button folded into ⋯ (narrow screen): what it opens, under ⋯
        if let DbPick::Open(b) = p {
            let anchor = self
                .drawbar()
                .and_then(|(_, bar)| bar.at(DbBtn::More))
                .map_or(Rect::new(100.0, 100.0, 10.0, 10.0), |r| self.win(r, self.rw()));
            self.ui.list = None;
            return match b {
                DbBtn::TextColor => {
                    self.open_colours(i, Which::Text, anchor, false);
                    self.redraw();
                }
                _ => self.open_list(ListKind::DrawBar(i, DbBtn::Template), anchor),
            };
        }
        if let Some(d) = self.chart.drawings.get_mut(i) {
            match p {
                DbPick::Width(w) => d.style.width = w,
                DbPick::Dash(k) => {
                    d.style.dash = k == 1;
                    d.style.dotted = k == 2;
                }
                DbPick::Sync(k) => d.style.sync = k,
                DbPick::DefaultTemplate => drawbar::apply_template(&mut d.style, &argos_render::drawings::Style::default()),
                DbPick::Template(k) => {
                    let tool = d.tool.clone();
                    if let Some((_, t)) = self.templates.get(&tool).and_then(|v| v.get(k)).cloned() {
                        drawbar::apply_template(&mut self.chart.drawings[i].style, &t);
                    }
                }
                DbPick::SaveTemplate => {
                    let anchor = self
                        .drawbar()
                        .map_or(Rect::new(200.0, 200.0, 10.0, 10.0), |(_, bar)| self.win(bar.rect, self.rw()));
                    self.ui.name_box = Some(argos_render::wlmenu::NameBox::new(
                        argos_render::wlmenu::NameFor::DrawingTemplate,
                        "",
                        anchor,
                        self.win_w(),
                    ));
                }
                DbPick::Obj(_) | DbPick::RemoveTemplate(_) | DbPick::Open(_) => {}
            }
        }
        self.redraw();
    }
}
