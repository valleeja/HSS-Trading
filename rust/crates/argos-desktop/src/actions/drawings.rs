//! ✏️ Dessins et outils : barre flottante, brouillon, réglages, actions sur un objet.

use crate::*;

impl App {
    /// Géométrie des objets telle qu'affichée (temps ↔ x, prix ↔ y, aimant).
    /// Outil choisi, rien de commencé : son aperçu suit la souris, prêt à poser (l'émoji choisi pour l'icône).
    pub(crate) fn ghost(&self) -> Option<Drawing> {
        if self.draft.is_some() || matches!(self.tool, "cursor" | "erase") || self.ui.list.is_some() || self.ui.pop.is_some() || self.ui.obj_dlg.is_some() {
            return None;
        }
        argos_render::tools::def(self.tool).filter(|d| d.points > 0)?;
        let (x, y) = self.pointer.mouse?;
        if x < 0.0 || y < 0.0 || x > self.chart.viewport.plot_w() || y > self.chart.viewport.plot_h() {
            return None;
        }
        let at = self.frame_now()?.snap(x, y, self.rail.magnet);
        Some(self.new_drawing(self.tool, at))
    }

    /// Nouvel objet de l'outil `tool` à son 1er point (RR de la barre de gauche, émoji choisi pour l'icône).
    pub(crate) fn new_drawing(&self, tool: &str, at: Point) -> Drawing {
        let mut d = Drawing::new(tool, vec![at]);
        // le style gardé pour cet outil (réglages validés la dernière fois), sinon celui de départ ; sans texte ni verrou
        d.style = self.tool_styles.get(tool).cloned().unwrap_or_else(|| Drawing::default_style(tool));
        (d.style.text, d.style.locked, d.style.hidden) = (String::new(), false, false);
        d.style.rr = self.rail.rr;
        if tool == "icon" {
            d.style.text = self.rail.emoji.clone();
        }
        d
    }

    /// Choisit un outil de dessin (le curseur = aucun). Son aide s'affiche dans le bandeau.
    pub(crate) fn set_tool(&mut self, id: &'static str) {
        self.tool = id;
        self.draft = None;
        self.banner = argos_render::tools::def(id)
            .filter(|d| !d.tip.is_empty() && id != "cursor")
            .map(|d| format!("{} : {}", d.label, d.tip));
        // icône : d'abord la grille d'émojis, puis l'émoji choisi suit la souris (comme TradingView)
        if id == "icon" {
            let text = argos_render::text::Text::default();
            self.sync_toolbar();
            let anchor = [Btn::Fav("icon"), Btn::Group("annot")]
                .into_iter()
                .find_map(|b| toolbar::button_rect(&self.tools, &text, b))
                .unwrap_or(Rect::new(8.0, 8.0, 0.0, 0.0));
            self.ui.list = None;
            self.ui.pop = Some(toolbar::Pop::new(
                toolbar::PopKind::Emoji,
                anchor,
                self.chart.viewport.plot_w(),
                self.chart.viewport.plot_h(),
            ));
        }
    }

    /// Clic sur la chart avec un outil choisi : gomme, ou un point de plus pour l'objet en cours (1er clic = nouvel
    /// objet ; il se termine au bon nombre de points, ou au double-clic / Entrée pour les outils à points libres).
    pub(crate) fn use_tool(&mut self, x: f64, y: f64, double: bool) {
        if self.tool == "erase" {
            if let Some(i) = self.drawing_at(x, y).filter(|i| !self.chart.drawings[*i].locked()) {
                self.chart.drawings.remove(i);
                self.selected = None;
            }
            return self.redraw();
        }
        let Some(def) = argos_render::tools::def(self.tool).filter(|d| d.points > 0) else {
            return;
        };
        let Some(at) = self.frame_now().map(|f| f.snap(x, y, self.rail.magnet)) else {
            return;
        };
        let fresh = self.new_drawing(def.id, at);
        let d = self.draft.get_or_insert(fresh);
        if let Some(last) = d.pts.last_mut() {
            *last = at; // le point d'aperçu devient un point cliqué
        }
        let free = def.points == 99;
        let done = if free {
            double && d.pts.len() > 1
        } else {
            d.pts.len() >= def.points as usize
        };
        if done {
            self.finish_draft();
        } else {
            d.pts.push(at); // nouveau point d'aperçu, il suit la souris
        }
        self.redraw();
    }

    /// Termine l'objet en cours : il rejoint les dessins, sélectionné ; l'outil revient au curseur sauf en mode continu.
    pub(crate) fn finish_draft(&mut self) {
        let Some(mut d) = self.draft.take() else { return };
        let need = argos_render::tools::def(&d.tool).map_or(1, |t| t.points as usize);
        if need == 99 && d.pts.len() > 2 {
            d.pts.pop(); // le point d'aperçu du double-clic
        }
        // 🔍 loupe et 📸 capture pour l'assistant : des actions, pas des objets qui restent sur la chart
        if matches!(d.tool.as_str(), "loupe" | "aicapture") && d.pts.len() >= 2 {
            let (t0, t1) = (d.pts[0].t.min(d.pts[1].t), d.pts[0].t.max(d.pts[1].t));
            if d.tool == "loupe" {
                let i = self.chart.bars.partition_point(|b| b.time < t0);
                let j = self.chart.bars.partition_point(|b| b.time <= t1).saturating_sub(1).max(i);
                self.run_action(ChartAction::AutoScale); // prix automatiques : ils se recadrent sur la plage
                self.chart.viewport.show_range(i, j, &self.chart.slots);
                self.banner = None;
            } else if let Some(f) = self.frame_now() {
                let ((x0, y0), (x1, y1)) = (f.geo().xy(&d.pts[0]), f.geo().xy(&d.pts[1]));
                // la chart telle qu'à l'écran (dessins et indicateurs), sans réticule ni bandeau
                let pm = argos_render::render(&Chart {
                    crosshair: None,
                    banner: None,
                    pad: None,
                    trades: vec![],
                    ..self.chart_with(true)
                });
                let r = tiny_skia::IntRect::from_xywh(
                    x0.min(x1) as i32,
                    y0.min(y1) as i32,
                    ((x1 - x0).abs() as u32).max(1),
                    ((y1 - y0).abs() as u32).max(1),
                );
                let name = format!("argos-capture-{}.png", d.pts[1].t);
                let png = r.and_then(|r| pm.clone_rect(r)).and_then(|z| z.encode_png().ok());
                self.banner = Some(match png.map(|b| platform::save_download(&name, &b)) {
                    Some(Ok(dir)) => format!("📸 Zone enregistrée pour l'assistant Argos : {name} dans {dir}"),
                    _ => "📸 Capture impossible ici".into(),
                });
            }
            self.tool = "cursor";
            return;
        }
        let is_pos = argos_render::position::is_position(&d.tool);
        if is_pos {
            argos_render::position::normalize(&mut d); // entrée, stop, cible
                                                       // 💼 le risque de ta stratégie (réglages de prop firm du journal) : même calcul que le ticket
            let cfg = &self.watch.panels.journal.cfg;
            (d.style.account, d.style.risk, d.style.risk_pct, d.style.lot) = (cfg.account, cfg.risk, false, 1.0);
        }
        self.chart.drawings.push(d);
        self.selected = Some(self.chart.drawings.len() - 1);
        if is_pos {
            self.ticket_from_drawing(self.chart.drawings.len() - 1);
        }
        if !self.rail.keep {
            self.tool = "cursor";
            self.banner = None;
        }
    }

    /// Ouvre la fenêtre de réglages d'un dessin (sur un onglet, ou le premier) avec tes modèles pour cet outil.
    pub(crate) fn open_obj_dlg(&mut self, i: usize, tab: Option<DlgTab>) {
        let Some(d) = self.chart.drawings.get(i) else { return };
        let ww = self.win_w();
        let wh = self.area.1 as f64 + self.top_h();
        let mut dl = ObjDialog::new(i, d, self.chart.interval.secs(), ww, wh);
        dl.bar0 = self.chart.bars.last().map_or(0, |b| b.time); // Coordonnées: bar 0 = the last bar
        if let Some(t) = tab {
            dl = dl.with_tab(t, d);
        }
        dl.templates = self
            .templates
            .get(&d.tool)
            .map(|v| v.iter().map(|(n, _)| n.clone()).collect())
            .unwrap_or_default();
        self.ui.obj_dlg = Some(dl);
    }

    /// Réponse de la fenêtre de réglages : fermer (Annuler remet l'objet), garder, sauver ou appliquer un modèle.
    pub(crate) fn dlg_result(&mut self, mut dl: ObjDialog, c: DlgClick) {
        let Some(d) = self.chart.drawings.get_mut(dl.index) else { return };
        match c {
            DlgClick::Close(false) => return *d = dl.backup.clone(),
            DlgClick::Close(true) => {
                // D'accord : ce style devient celui de l'outil pour les prochains objets (comme « garder en mémoire »)
                self.tool_styles.insert(d.tool.clone(), d.style.clone());
                Self::write_prefs("tool_styles.json", &self.tool_styles);
                return;
            }
            DlgClick::None | DlgClick::Changed => {}
            DlgClick::SaveAs(name) => {
                let list = self.templates.entry(d.tool.clone()).or_default();
                match list.iter_mut().find(|(n, _)| *n == name) {
                    Some(t) => t.1 = d.style.clone(),
                    None => list.push((name, d.style.clone())),
                }
                dl.templates = list.iter().map(|(n, _)| n.clone()).collect();
                Self::write_prefs("templates.json", &self.templates);
            }
            DlgClick::RemoveTemplate(k) => {
                if let Some(v) = self.templates.get_mut(&d.tool).filter(|v| k < v.len()) {
                    v.remove(k);
                }
                Self::write_prefs("templates.json", &self.templates);
            }
            DlgClick::Palette(w, anchor) => {
                // the shared palette, over the window; « Droite » adds width and line style under the colours
                let line =
                    w == argos_render::objdlg::Which::Line && !argos_render::position::is_position(&d.tool) && !matches!(d.tool.as_str(), "fvg" | "ob" | "liq");
                let i = dl.index;
                self.ui.obj_dlg = Some(dl);
                return self.open_colours(i, w, anchor, line);
            }
            DlgClick::Template(k) => {
                if let Some((_, st)) = self.templates.get(&d.tool).and_then(|v| v.get(k)) {
                    // le style du modèle ; le texte, le verrou et le « caché » de l'objet restent (même règle que la barre)
                    argos_render::drawbar::apply_template(&mut d.style, st);
                }
            }
        }
        // 💼 une position modifiée dans sa fenêtre (prix, risque) : le ticket suit
        if argos_render::position::is_position(&d.tool) {
            let i = dl.index;
            self.ui.obj_dlg = Some(dl);
            return self.ticket_from_drawing(i);
        }
        self.ui.obj_dlg = Some(dl);
    }

    /// Ajoute une copie décalée de 3 bougies vers la droite, sélectionnée (Cloner, Coller).
    pub(crate) fn add_copy(&mut self, mut d: Drawing) {
        let dt = 3 * self.chart.interval.secs();
        d.pts.iter_mut().for_each(|q| q.t += dt);
        d.style.locked = false;
        self.chart.drawings.push(d);
        self.selected = Some(self.chart.drawings.len() - 1);
    }

    /// Action du menu du clic droit sur le dessin `i`.
    pub(crate) fn obj_action(&mut self, i: usize, a: ObjAction) {
        // 🔔 « Ajouter une alerte » on a drawing: the alert window (as the drawing bar's alarm)
        if a == ObjAction::Alert {
            return self.open_draw_alert(i);
        }
        if let Some(msg) = a.soon() {
            self.banner = Some(msg.to_string());
            return self.redraw();
        }
        if i >= self.chart.drawings.len() {
            return;
        }
        match a {
            ObjAction::Front => {
                let d = self.chart.drawings.remove(i);
                self.chart.drawings.push(d);
                self.selected = Some(self.chart.drawings.len() - 1);
            }
            ObjAction::Back => {
                let d = self.chart.drawings.remove(i);
                self.chart.drawings.insert(0, d);
                self.selected = Some(0);
            }
            ObjAction::Intervals => self.open_obj_dlg(i, Some(DlgTab::Vis)),
            ObjAction::Settings => self.open_obj_dlg(i, None),
            ObjAction::Clone => self.add_copy(self.chart.drawings[i].clone()),
            ObjAction::Copy => self.obj_clip = Some(self.chart.drawings[i].clone()),
            ObjAction::Invert => argos_render::position::invert(&mut self.chart.drawings[i]),
            ObjAction::Lock => {
                let d = &mut self.chart.drawings[i];
                d.set_locked(!d.locked());
            }
            ObjAction::Tree => return self.open_tree(),
            ObjAction::Pinceau => return self.pinceau_zone(i),
            ObjAction::Hide => {
                self.chart.drawings[i].style.hidden = true;
                self.selected = None;
                self.banner = Some("Dessin caché : barre de gauche → œil → Masquer / Afficher les dessins le fait revenir".into());
            }
            ObjAction::Remove if !self.chart.drawings[i].locked() => {
                self.chart.drawings.remove(i);
                self.selected = None;
            }
            _ => {}
        }
        self.redraw();
    }

    /// Objet dessiné sous la souris (aucun menu ouvert). 👆 By the finger, within a finger's reach (B-019).
    pub(crate) fn drawing_at(&self, x: f64, y: f64) -> Option<usize> {
        if self.ui.context.is_open() {
            return None;
        }
        if self.rail.hide_draw {
            return None;
        }
        let f = self.frame_now()?;
        let tol = if argos_render::actions::tactile() {
            argos_render::drawings::HIT_DOIGT
        } else {
            argos_render::drawings::HIT_PX
        };
        argos_render::drawings::hit_index_r(&self.chart.drawings, x, y, &f.geo(), tol)
    }

    /// Poignée de l'objet sélectionné sous la souris : (objet, point). 👆 By the finger, 44 points across (B-019).
    pub(crate) fn handle_at(&self, x: f64, y: f64) -> Option<(usize, usize)> {
        let i = self.selected.filter(|_| !self.rail.hide_draw)?;
        let f = self.frame_now()?;
        let r = if argos_render::actions::tactile() {
            argos_render::drawings::HANDLE_DOIGT
        } else {
            argos_render::drawings::HANDLE_PX
        };
        self.chart.drawings.get(i)?.handle_at_r(x, y, &f.geo(), r).map(|h| (i, h))
    }

    /// ✏️ A drawing or the selected one's handle under (x, y) (B-019: a double tap there is the drawing's — its
    /// settings —, not the chart's full screen).
    pub(crate) fn sur_un_dessin(&self, x: f64, y: f64) -> bool {
        self.handle_at(x, y).is_some() || self.drawing_at(x, y).is_some()
    }

    /// 🌳 Ouvre l'arborescence des objets (les autres fenêtres se ferment).
    pub(crate) fn open_tree(&mut self) {
        let ww = self.win_w();
        (self.ui.list, self.ui.sheet, self.ui.argos, self.ui.pop) = (None, None, None, None);
        self.ui.obj_tree = Some(argos_render::objtree::ObjTree::new(self.top_h(), ww, self.top_h() + self.area.1 as f64));
        self.redraw();
    }

    /// Lignes de l'arborescence : un dessin par ligne, le plus récent en haut.
    pub(crate) fn tree_rows(&self) -> Vec<argos_render::objtree::TreeRow> {
        use argos_render::indicators::Ind;
        let dec = data_decimals(&self.chart.bars);
        // les indicateurs de la chart d'abord (comme v1), puis les dessins
        let inds = Ind::ALL
            .iter()
            .enumerate()
            .filter(|(_, i)| self.ind.on.contains(i))
            .map(|(k, i)| argos_render::objtree::TreeRow {
                index: argos_render::objtree::IND + k,
                tool: "📈".into(),
                label: i.name().into(),
                sub: if i.oscillator() { "oscillateur".into() } else { "sur le prix".into() },
                hidden: self.ind.leg.1.contains(i),
                ..Default::default()
            });
        inds.chain(self.chart.drawings.iter().enumerate().rev().map(|(i, d)| argos_render::objtree::TreeRow {
            index: i,
            tool: d.tool.clone(),
            label: argos_render::tools::def(&d.tool).map_or(d.tool.clone(), |t| t.label.to_string()),
            sub: if d.style.text.is_empty() {
                d.pts.first().map_or(String::new(), |p| format_price(p.p, dec))
            } else {
                d.style.text.clone()
            },
            hidden: d.style.hidden,
            locked: d.locked(),
            selected: self.selected == Some(i),
        }))
        .collect()
    }

    pub(crate) fn save_tools(&self) {
        // 🖥 la place et l'état de la barre sont à interface-2 (l'ancienne /app/ garde les siens)
        Self::write_prefs_garde("toolbar.json", &self.tools, &crate::actions::accueil::BARRE_A_ELLE);
        self.barre_garder();
    }

    /// La barre flottante suit la barre de gauche : tes outils ⭐, le dernier outil de chaque groupe, l'outil choisi.
    pub(crate) fn sync_toolbar(&mut self) {
        self.tools.favs = self.rail.favs.iter().filter_map(|t| argos_render::tools::def(t)).map(|d| d.id).collect();
        let misc = self
            .rail
            .last
            .iter()
            .find(|(k, _)| k == toolbar::MISC)
            .and_then(|(_, t)| toolbar::MISC_TOOLS.iter().find(|m| *m == t))
            .copied();
        self.tools.groups = rail::group_ids()
            .map(|g| (g, self.rail.current(g)))
            .chain([(toolbar::MISC, misc.unwrap_or("measure"))])
            .collect();
        self.tools.active = self.tool;
        // 📱 cell : − + RAZ Zen au bout de la barre, et la barre défile si elle dépasse la chart
        self.tools.pad = self.top.phone;
        self.tools.zen = self.ui.zen;
        self.tools.max_w = if self.top.phone { self.chart.viewport.plot_w() } else { 0.0 };
        let text = argos_render::text::Text::default();
        self.tools.scroll = self.tools.scroll.clamp(0.0, toolbar::max_scroll(&self.tools, &text));
        self.tools.style_icon = self.chart.styles[self.chart.style].icon();
        self.tools.style = self.chart.style;
        self.tools.inds_on = self.ind.on.clone();
        self.tools.ind_favs = self.ind.favs.clone();
        self.tools.emoji = argos_render::emoji::statique(&self.rail.emoji).unwrap_or(argos_render::emoji::DEFAULT);
        self.tools.interval = self.chart.interval.secs();
        // la barre a pu s'allonger : elle reste dans la chart, là où la souris la touche aussi
        let text = argos_render::text::Text::default();
        toolbar::clamp(&mut self.tools, &text, self.chart.viewport.plot_w(), self.chart.viewport.plot_h());
    }

    /// Choisit un outil depuis la barre flottante : il devient le dernier de sa catégorie (son icône dans la barre).
    pub(crate) fn pick_tool(&mut self, t: &'static str) {
        let group = rail::group_of(t).or_else(|| toolbar::MISC_TOOLS.contains(&t).then_some(toolbar::MISC));
        if let Some(g) = group {
            self.rail.set_last(g, t);
            self.save_rail();
        }
        self.set_tool(t);
    }

    /// Menu du clic droit sur la barre flottante / la barre de gauche (comme Argos v1).
    pub(crate) fn tool_menu(&self, t: Option<&'static str>) -> Vec<MenuGroup> {
        let magnet = match self.rail.magnet {
            rail::Magnet::Off => "désactivé",
            rail::Magnet::Weak => "faible",
            rail::Magnet::Strong => "fort",
        };
        toolbar::context_menu(&toolbar::MenuState {
            tool: t,
            fav: t.is_some_and(|t| self.rail.favs.iter().any(|f| f == t)),
            keep: self.rail.keep,
            magnet,
            vertical: self.tools.vertical,
            hidden: self.tools.hidden,
            drawings_hidden: self.rail.hide_draw,
        })
    }

    /// Ligne choisie dans le menu du clic droit de la barre.
    pub(crate) fn tool_menu_action(&mut self, t: Option<&'static str>, a: toolbar::MenuAction) {
        use toolbar::MenuAction::*;
        match a {
            Use => {
                if let Some(t) = t {
                    self.pick_tool(t);
                }
            }
            Fav => {
                if let Some(t) = t {
                    let on = self.rail.toggle_fav(t);
                    self.save_rail();
                    self.banner = Some(if on { "⭐ Ajouté aux favoris" } else { "☆ Retiré des favoris" }.into());
                }
            }
            Ai => self.banner = Some("✨ Assistant Argos (AI) : il arrive dans la version Rust, comme dans Argos v1".into()),
            Keep => {
                self.rail.keep = !self.rail.keep;
                self.save_rail();
                self.banner = Some(
                    if self.rail.keep {
                        "🔒 L'outil reste actif après chaque dessin"
                    } else {
                        "L'outil se désactive après chaque dessin"
                    }
                    .into(),
                );
            }
            Magnet => {
                self.rail.magnet = match self.rail.magnet {
                    rail::Magnet::Off => rail::Magnet::Weak,
                    rail::Magnet::Weak => rail::Magnet::Strong,
                    rail::Magnet::Strong => rail::Magnet::Off,
                };
                self.save_rail();
            }
            Vertical => {
                self.tools.vertical = !self.tools.vertical;
                self.save_tools();
            }
            Hide => {
                self.tools.hidden = !self.tools.hidden;
                self.ui.pop = None;
                self.save_tools();
                if self.tools.hidden {
                    self.banner = Some("Barre de favoris masquée : clic droit sur la barre de gauche ▸ Afficher la barre de favoris".into());
                }
            }
            LockAll => return self.rail_action(rail::MenuAction::LockDraw),
            HideDrawings => return self.rail_action(rail::MenuAction::HideDraw),
            // « Supprimer… » : le menu de la poubelle (dessins, indicateurs, tout), comme le bouton de la barre de gauche
            DeleteAll => {
                if let Some((r, _)) = rail::layout(self.area.1 as f64).into_iter().find(|(_, b)| *b == rail::Btn::Trash) {
                    return self.open_list(ListKind::RailButton(rail::Btn::Trash), self.win(r, 0.0));
                }
            }
        }
        self.sync_toolbar();
        self.redraw();
    }

    /// Clic dans la fenêtre des outils / des unités de temps (coordonnées de la chart). `true` : le clic est pris.
    pub(crate) fn pop_click(&mut self, x: f64, y: f64) -> bool {
        // 😀 sélecteur d'émojis : catégories, recherche, une case
        if let Some(p) = self.ui.pop.as_mut().and_then(|p| p.pick.as_mut()) {
            use argos_render::emojipick::PickClick;
            let dedans = p.rect.contains(x, y);
            match p.click(x, y) {
                PickClick::None | PickClick::Changed => {}
                PickClick::Close => {
                    self.ui.pop = None;
                    let text = argos_render::text::Text::default();
                    return dedans || toolbar::hit(&self.tools, &text, x, y) == Some(Btn::Group("annot"));
                }
                PickClick::Emoji(e) => {
                    self.ui.pop = None;
                    self.rail.emoji = e.to_string();
                    self.save_rail();
                    self.sync_toolbar();
                    self.banner = Some("Émoji prêt : il suit ta souris, clique sur la chart pour le poser".into());
                }
            }
            self.redraw();
            return true;
        }
        let Some(pop) = self.ui.pop.clone() else { return false };
        match pop.click(x, y) {
            toolbar::PopClick::None => {}
            toolbar::PopClick::Close => {
                self.ui.pop = None;
                // à côté : la fenêtre se ferme et le clic continue (sauf sur le bouton qui l'a ouverte : il la referme)
                let text = argos_render::text::Text::default();
                let opener = match pop.kind {
                    toolbar::PopKind::Tf => Btn::Intervals,
                    toolbar::PopKind::Group(g) => Btn::Group(g),
                    toolbar::PopKind::Emoji => Btn::Group("annot"),
                    toolbar::PopKind::Style => Btn::Style,
                    toolbar::PopKind::Ind => Btn::Indicators,
                };
                return pop.rect.contains(x, y) || toolbar::hit(&self.tools, &text, x, y) == Some(opener);
            }
            toolbar::PopClick::Tool(t) => {
                self.ui.pop = None;
                self.replier_barre_apres_choix();
                self.pick_tool(t);
            }
            toolbar::PopClick::Star(t) => {
                self.rail.toggle_fav(t);
                self.save_rail();
                self.sync_toolbar();
            }
            toolbar::PopClick::Emoji(e) => {
                self.ui.pop = None;
                self.replier_barre_apres_choix();
                self.rail.emoji = e.to_string();
                self.save_rail();
                self.banner = Some("Émoji prêt : il suit ta souris, clique sur la chart pour le poser".into());
            }
            toolbar::PopClick::Tf(s) => {
                self.ui.pop = None;
                self.replier_barre_apres_choix();
                self.set_interval(Interval(s));
                return true;
            }
            toolbar::PopClick::Style(k) => {
                self.ui.pop = None;
                self.replier_barre_apres_choix();
                let step = k as isize - self.chart.style as isize;
                self.switch_style(step);
                return true;
            }
            toolbar::PopClick::Ind(i) => {
                self.toggle_indicator(i);
                self.sync_toolbar(); // la case reste ouverte, allumée ou éteinte
            }
            toolbar::PopClick::StarTf(s) => {
                self.tools.toggle_favorite(s);
                self.save_tools();
                self.sync_toolbar();
            }
            toolbar::PopClick::StarInd(i) => {
                match self.ind.favs.iter().position(|x| *x == i) {
                    Some(k) => {
                        self.ind.favs.remove(k);
                    }
                    None => self.ind.favs.push(i),
                }
                self.ind.out.favs = self.ind.favs.clone();
                Self::write_prefs("market_fav.json", &self.ind.favs);
                self.sync_toolbar();
            }
        }
        self.redraw();
        true
    }

    /// 📊 fx de la barre du haut (`r` : le bouton, coordonnées de la fenêtre) : la fenêtre des types de graphique ou
    /// des indicateurs en haut de la chart, sous le bouton ; encore = fermer.
    pub(crate) fn pop_du_haut(&mut self, kind: toolbar::PopKind, r: Rect) {
        if self.ui.pop.as_ref().is_some_and(|p| p.kind == kind) {
            self.ui.pop = None;
        } else {
            self.ui.list = None;
            let ancre = Rect::new(r.x - self.rw(), -6.0, r.w, 0.0);
            let mut p = toolbar::Pop::new(kind, ancre, self.chart.viewport.plot_w(), self.chart.viewport.plot_h());
            // 🖥 PC (maquette) : sous le bouton du haut, par-dessus les rangées 2 et 3 (comme les autres menus du haut) ;
            // sa place reste en coordonnées de la chart (le haut est alors au-dessus d'elle)
            if !self.top.phone && p.pick.is_none() {
                p.rect.y = crate::render::VOLET_HAUT - self.top_h();
            }
            self.ui.pop = Some(p);
        }
        self.redraw();
    }

    /// 🫥 PC : la rangée du haut et la barre de gauche sont-elles estompées ? Oui quand la souris travaille sur la
    /// chart, loin d'elles (24 points), sans menu ni fenêtre ouverte, et si le réglage est allumé (☰ Appli).
    /// 📅 The month report open over the chart: the mouse reads it, it does not work on the chart.
    /// 👆 By the finger (a tablet with the PC bars): never faded, there is no hover to bring them back.
    pub(crate) fn barres_estompees(&self) -> bool {
        if self.top.phone || argos_render::actions::tactile() || !self.top.prefs.estomper || self.ui.zen {
            return false;
        }
        let u = &self.ui;
        let ouvert = u.list.is_some()
            || u.argos.is_some()
            || u.sheet.is_some()
            || u.pop.is_some()
            || u.palette.is_some()
            || u.accueil.is_some()
            || u.split_menu.is_some()
            || u.name_box.is_some()
            || u.ind_dlg.is_some()
            || u.context.is_open()
            || u.sim.as_ref().is_some_and(|s| s.month.is_some());
        let Some((x, y)) = self.pointer.mouse else { return false };
        !ouvert && y > 24.0 && x > 24.0 && self.pointer.top_mouse.is_none() && self.pointer.rail_mouse.is_none()
    }

    /// 🫥 La barre flottante est-elle visible ? Sur PC (souris) : seulement quand la souris est à moins de 24 points
    /// d'elle, ou si `occupee` (on la déplace, une de ses fenêtres est ouverte) ; au doigt : toujours.
    pub(crate) fn barre_outils_proche(&self, occupee: bool) -> bool {
        if self.top.phone || occupee || self.rail.folded {
            return true;
        }
        let Some((x, y)) = self.pointer.mouse else { return false };
        let (r, _) = toolbar::layout(&self.tools, &argos_render::text::Text::default());
        let m = 24.0;
        x > r.x - m && x < r.x + r.w + m && y > r.y - m && y < r.y + r.h + m
    }

    /// Clic sur un bouton de la barre flottante.
    /// 📱 Cell / tablette (choix de Jee, 15 nov.) : la barre flottante ouverte se replie toute seule en ⋮⋮, à sa place
    /// de la maquette, dès qu'un de ses boutons a été choisi (elle ne reste plus en travers de la chart).
    pub(crate) fn replier_barre_apres_choix(&mut self) {
        if !self.tools.pad || self.tools.retracted || self.tools.vertical {
            return;
        }
        self.tools.retracted = true;
        self.tools.scroll = 0.0;
        let (w, h) = (self.win_w(), self.cell_h());
        if let Some(r) = argos_render::ecrans::cell().place("favbar", w, h) {
            (self.tools.x, self.tools.y) = (r.x - self.rw(), r.y - self.top_h());
        }
        toolbar::clamp(
            &mut self.tools,
            &argos_render::text::Text::default(),
            self.chart.viewport.plot_w(),
            self.chart.viewport.plot_h(),
        );
        self.save_tools();
    }

    pub(crate) fn tool_click(&mut self, b: Btn, x: f64, y: f64) {
        let text = argos_render::text::Text::default();
        // un bouton qui fait son action tout de suite (pas ⋮⋮, pas ⇅, pas un bouton qui ouvre sa fenêtre de cases) :
        // la barre se replie
        if matches!(b, Btn::Favorite(_) | Btn::Fav(_) | Btn::Pad(_)) {
            self.replier_barre_apres_choix();
        }
        match b {
            Btn::Handle => {
                self.pointer.tool_drag = Some((x - self.tools.x, y - self.tools.y));
                self.pointer.tool_moved = false;
            }
            Btn::Favorite(secs) => return self.set_interval(Interval(secs)),
            // 🕐 unités de temps, 🕯️ types de graphique, fx gamme Argos, une catégorie d'outils : en cases avec icônes (encore = fermer)
            Btn::Intervals | Btn::Style | Btn::Indicators | Btn::Group(_) => {
                let kind = match b {
                    Btn::Group(g) => toolbar::PopKind::Group(g),
                    Btn::Style => toolbar::PopKind::Style,
                    Btn::Indicators => toolbar::PopKind::Ind,
                    _ => toolbar::PopKind::Tf,
                };
                if self.ui.pop.as_ref().is_some_and(|p| p.kind == kind) {
                    self.ui.pop = None;
                } else if let Some(r) = toolbar::button_rect(&self.tools, &text, b) {
                    self.ui.list = None;
                    self.ui.pop = Some(toolbar::Pop::new(kind, r, self.chart.viewport.plot_w(), self.chart.viewport.plot_h()));
                }
            }
            // outil ⭐ : le choisir
            Btn::Fav(t) => self.pick_tool(t),
            // 📱 − + RAZ Zen (cell : au bout de la barre)
            Btn::Pad(k) => match toolbar::pad_action(k) {
                argos_render::zoompad::PadBtn::Action(a) => return self.run_action(a),
                argos_render::zoompad::PadBtn::Zen => return self.toggle_zen(),
                argos_render::zoompad::PadBtn::Replay => return self.replay_start(),
            },
            Btn::Rotate => {
                self.tools.vertical = !self.tools.vertical;
                self.ui.pop = None;
                self.sync_toolbar();
                self.save_tools();
            }
            // 🔔 le menu des alertes, sous le bouton
            Btn::Alerts => {
                if let Some(r) = toolbar::button_rect(&self.tools, &text, b) {
                    let anchor = Rect::new(r.x + self.rw(), r.y + r.h, r.w, 1.0);
                    self.replier_barre_apres_choix();
                    return self.open_list(ListKind::Alerts, anchor);
                }
            }
        }
        self.redraw();
    }
}
