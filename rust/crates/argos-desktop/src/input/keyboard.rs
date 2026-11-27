//! ⌨️ Clavier : une fonction par fenêtre qui reçoit les touches (essayées dans l'ordre de `on_window_event`).

use super::Focus;
use crate::*;

impl App {
    /// boîte de clé API : on tape ou colle (Ctrl + V), Tab = champ suivant, Entrée = enregistrer, Échap = annuler
    pub(crate) fn key_box(&mut self, event: winit::event::KeyEvent) {
        let paste = self.pointer.mods.control_key() && matches!(&event.logical_key, Key::Character(c) if c.eq_ignore_ascii_case("v"));
        let clip = if paste {
            self.clipboard.as_mut().and_then(|c| c.get_text().ok())
        } else {
            None
        };
        let Some(b) = self.session.key_box.as_mut() else { return };
        match &event.logical_key {
            Key::Named(NamedKey::Escape) => {
                b.wipe();
                self.session.key_box = None;
            }
            Key::Named(NamedKey::Enter) if b.complete() => self.key_save(),
            Key::Named(NamedKey::Tab) => b.next(),
            Key::Named(NamedKey::Backspace) => b.backspace(),
            _ if paste => {
                if let Some(mut t) = clip {
                    b.type_text(&t);
                    t.clear();
                }
            }
            _ => {
                if let Some(t) = event.text.as_ref().filter(|_| !self.pointer.mods.control_key()) {
                    b.type_text(t);
                }
            }
        }
        self.redraw();
    }

    /// ✨ assistant : on tape la demande ; Entrée envoie, Échap ferme, Ctrl + V colle
    pub(crate) fn key_assistant(&mut self, event: winit::event::KeyEvent) {
        let paste = self.pointer.mods.control_key() && matches!(&event.logical_key, Key::Character(c) if c.eq_ignore_ascii_case("v"));
        match &event.logical_key {
            Key::Named(NamedKey::Escape) => return self.toggle_ai(),
            Key::Named(NamedKey::Enter) => {
                if let Some(q) = self.ai.take_input() {
                    return self.ai_send(q);
                }
            }
            Key::Named(NamedKey::Backspace) => _ = self.ai.input.pop(),
            _ if paste => {
                if let Some(t) = self.clipboard.as_mut().and_then(|c| c.get_text().ok()) {
                    self.ai.input.push_str(t.trim_end());
                }
            }
            _ => {
                if let Some(t) = event.text.as_ref().filter(|_| !self.pointer.mods.control_key()) {
                    self.ai.input.extend(t.chars().filter(|c| !c.is_control()));
                }
            }
        }
        self.redraw();
    }

    /// 📜 éditeur : on tape le code ; Ctrl + V coller, Ctrl + S sauvegarder, Ctrl + Entrée tester, Échap ferme le
    /// menu / la bande, puis la fenêtre (comme v1)
    pub(crate) fn key_editor(&mut self, event: winit::event::KeyEvent) {
        use argos_render::editor::EdKey;
        let ctrl = self.pointer.mods.control_key();
        let letter = |s: &str| matches!(&event.logical_key, Key::Character(c) if c.eq_ignore_ascii_case(s));
        if ctrl && letter("s") {
            return self.editor_run(true);
        }
        if ctrl && matches!(event.logical_key, Key::Named(NamedKey::Enter)) {
            return self.editor_run(false);
        }
        // Ctrl + A: everything selected; Ctrl + C then copies the whole text (the editor has no other selection)
        if ctrl && letter("a") {
            if let Some(ed) = self.ui.editor.as_mut() {
                ed.tout_selectionner();
            }
            return self.redraw();
        }
        if ctrl && letter("c") {
            if let Some(t) = self.ui.editor.as_ref().filter(|e| e.tout).map(|e| e.text()) {
                if let Some(c) = self.clipboard.as_mut() {
                    let _ = c.set_text(t);
                }
            }
            return self.redraw();
        }
        let clip = if ctrl && letter("v") {
            self.clipboard.as_mut().and_then(|c| c.get_text().ok())
        } else {
            None
        };
        let Some(ed) = self.ui.editor.as_mut() else { return };
        let k = match &event.logical_key {
            Key::Named(NamedKey::Escape) => {
                if ed.escape() {
                    self.ui.editor = None;
                }
                return self.redraw();
            }
            Key::Named(NamedKey::Enter) => Some(EdKey::Enter),
            Key::Named(NamedKey::Backspace) => Some(EdKey::Backspace),
            Key::Named(NamedKey::Delete) => Some(EdKey::Delete),
            Key::Named(NamedKey::ArrowLeft) => Some(EdKey::Left),
            Key::Named(NamedKey::ArrowRight) => Some(EdKey::Right),
            Key::Named(NamedKey::ArrowUp) => Some(EdKey::Up),
            Key::Named(NamedKey::ArrowDown) => Some(EdKey::Down),
            Key::Named(NamedKey::Home) => Some(EdKey::Home),
            Key::Named(NamedKey::End) => Some(EdKey::End),
            Key::Named(NamedKey::PageUp) => Some(EdKey::PageUp),
            Key::Named(NamedKey::PageDown) => Some(EdKey::PageDown),
            Key::Named(NamedKey::Tab) => Some(EdKey::Tab),
            _ => None,
        };
        match (k, clip) {
            (Some(k), _) => {
                let c = ed.key(k);
                if c != argos_render::editor::EdClick::None {
                    return self.editor_click(c);
                }
            }
            (None, Some(t)) => ed.insert(&t),
            (None, None) => {
                if let Some(t) = event.text.as_ref().filter(|_| !ctrl) {
                    ed.insert(t);
                }
            }
        }
        self.redraw();
    }

    /// 🛒 Marketplace : la recherche se tape, Échap ferme les détails / la recherche, puis la page
    pub(crate) fn key_market(&mut self, event: winit::event::KeyEvent) {
        let Some(m) = self.ui.mk_page.as_mut() else { return };
        match &event.logical_key {
            Key::Named(NamedKey::Escape) => {
                if m.escape() {
                    self.ui.mk_page = None;
                }
            }
            Key::Named(NamedKey::Backspace) => m.backspace(),
            _ => {
                if let Some(t) = event.text.as_ref() {
                    t.chars().for_each(|c| m.type_char(c));
                }
            }
        }
        self.redraw();
    }

    /// 💼 page Portfolio : Échap ferme le guide / le choix du montant, puis la page ; le reste du clavier est ignoré
    pub(crate) fn key_portfolio(&mut self, event: winit::event::KeyEvent) {
        if matches!(event.logical_key, Key::Named(NamedKey::Escape)) {
            match self.ui.pf_page.as_mut().map(|p| p.escape()) {
                Some(true) => self.ui.pf_page = None,
                Some(false) => {}
                // 📊 the backtest report page: Échap closes it
                None => self.ui.bt_page = None,
            }
        }
        self.redraw();
    }

    /// 📓 fiche d'un trade : Échap quitte le champ puis ferme, Tab = champ suivant, Entrée = enregistrer
    pub(crate) fn key_journal(&mut self, event: winit::event::KeyEvent) {
        use argos_render::journal::FormClick;
        let Some(mut f) = self.ui.j_form.take() else { return };
        let c = match &event.logical_key {
            Key::Named(NamedKey::Escape) if f.escape() => FormClick::Close,
            Key::Named(NamedKey::Escape) => FormClick::Changed,
            Key::Named(NamedKey::Enter) => f.enter(),
            Key::Named(NamedKey::Tab) => {
                f.tab();
                FormClick::Changed
            }
            Key::Named(NamedKey::Backspace) => {
                f.backspace();
                FormClick::Changed
            }
            _ => {
                if let Some(t) = event.text.as_ref() {
                    t.chars().for_each(|ch| f.type_char(ch));
                }
                FormClick::Changed
            }
        };
        self.j_form_result(f, c);
        self.redraw();
    }

    /// 📍 choix d'un prix / d'une heure sur la chart : Échap revient à la fenêtre sans rien changer
    pub(crate) fn key_ind_here(&mut self) {
        self.ui.ind_dlg = self.ui.ind_here.take().map(|(dl, _)| dl);
        self.banner = None;
        self.redraw();
    }

    /// fenêtre de réglages d'un indicateur : Échap = Annuler, Entrée = valider le champ (ou OK), le reste se tape
    pub(crate) fn key_ind_dlg(&mut self, event: winit::event::KeyEvent) {
        use argos_render::inddlg::IndClick;
        let Some(mut dl) = self.ui.ind_dlg.take() else { return };
        let c = match &event.logical_key {
            Key::Named(NamedKey::Escape) => IndClick::Close(false),
            Key::Named(NamedKey::Enter) => dl.enter(),
            Key::Named(NamedKey::Backspace) => {
                dl.backspace();
                IndClick::None
            }
            _ => {
                if let Some(t) = event.text.as_ref() {
                    t.chars().for_each(|ch| dl.type_char(ch));
                }
                IndClick::None
            }
        };
        self.ind_dlg_result(dl, c);
        self.redraw();
    }

    /// 🔔 the alert window of a drawing: Escape closes it, Backspace erases, the rest is typed in its field
    pub(crate) fn key_draw_alert(&mut self, event: winit::event::KeyEvent) {
        let Some((i, mut w)) = self.ui.draw_alert.take() else { return };
        match &event.logical_key {
            Key::Named(NamedKey::Escape) => return self.redraw(),
            Key::Named(NamedKey::Backspace) => w.backspace(),
            _ => {
                if let Some(t) = event.text.as_ref() {
                    t.chars().for_each(|c| w.type_char(c));
                }
            }
        }
        self.ui.draw_alert = Some((i, w));
        self.redraw();
    }

    /// fenêtre de réglages d'un dessin : Échap = Annuler, Entrée = valider le champ (ou OK), le reste se tape
    pub(crate) fn key_obj_dlg(&mut self, event: winit::event::KeyEvent) {
        let Some(mut dl) = self.ui.obj_dlg.take() else { return };
        let Some(d) = self.chart.drawings.get_mut(dl.index) else {
            return self.redraw();
        };
        match &event.logical_key {
            Key::Named(NamedKey::Escape) => *d = dl.backup.clone(),
            Key::Named(NamedKey::Enter) if !dl.typing() => {}
            Key::Named(NamedKey::Enter) => {
                let c = dl.enter(d);
                self.dlg_result(dl, c);
            }
            Key::Named(NamedKey::Backspace) => {
                dl.backspace(d);
                self.ui.obj_dlg = Some(dl);
            }
            _ => {
                if let Some(t) = event.text.as_ref() {
                    t.chars().for_each(|c| dl.type_char(c, d));
                }
                self.ui.obj_dlg = Some(dl);
            }
        }
        self.redraw();
    }

    /// boîte de nom ou panneau ＋ ouvert : le clavier tape le texte (Entrée = OK / ajouter, Échap = fermer)
    pub(crate) fn key_name_box(&mut self, event: winit::event::KeyEvent) {
        match &event.logical_key {
            Key::Named(NamedKey::Escape) => (self.ui.name_box, self.ui.add) = (None, None),
            Key::Named(NamedKey::Enter) if self.ui.name_box.is_some() => return self.name_ok(),
            Key::Named(NamedKey::Enter) => {
                if let Some(id) = self.ui.add.as_ref().and_then(|p| p.first()) {
                    self.add_symbol(&id);
                }
            }
            Key::Named(NamedKey::Backspace) => {
                if let Some(b) = self.ui.name_box.as_mut() {
                    b.backspace();
                } else if let Some(p) = self.ui.add.as_mut() {
                    p.backspace();
                    self.search_add();
                }
            }
            _ => {
                if let Some(t) = event.text.as_ref().filter(|t| !t.chars().any(char::is_control)) {
                    if let Some(b) = self.ui.name_box.as_mut() {
                        t.chars().for_each(|c| b.type_char(c));
                    } else if let Some(p) = self.ui.add.as_mut() {
                        t.chars().for_each(|c| p.type_char(c));
                        self.search_add();
                    }
                }
            }
        }
        self.redraw();
    }

    /// 🅘 recherche de la page Indicateurs : elle reçoit le clavier ; Échap la ferme
    pub(crate) fn key_fx_search(&mut self, key: &Key, text: Option<&str>) {
        // ✎ the drawings search of the pill's sheet (the same way)
        if let Some(p) = self.ui.pfeuille.as_mut().filter(|p| p.saisie) {
            match key {
                Key::Named(NamedKey::Escape | NamedKey::Enter) => p.saisie = false,
                Key::Named(NamedKey::Backspace) => self.pfeuille_touche(None),
                _ => {
                    if let Some(t) = text.filter(|t| !t.chars().any(char::is_control)) {
                        t.chars().for_each(|c| self.pfeuille_touche(Some(c)));
                    }
                }
            }
            return self.redraw();
        }
        // 🔎 (la recherche du volet Symboles passe par le même chemin)
        if let Some(p) = self.ui.symboles.as_mut().filter(|p| p.recherche.is_some()) {
            match key {
                Key::Named(NamedKey::Escape) => p.recherche = None,
                Key::Named(NamedKey::Backspace) => p.key(None),
                _ => {
                    if let Some(t) = text.filter(|t| !t.chars().any(char::is_control)) {
                        t.chars().for_each(|c| p.key(Some(c)));
                    }
                }
            }
            return self.redraw();
        }
        let Some(p) = self.ui.fx_page.as_mut() else { return };
        match key {
            Key::Named(NamedKey::Escape) => p.recherche = None,
            Key::Named(NamedKey::Backspace) => p.key(None),
            _ => {
                if let Some(t) = text.filter(|t| !t.chars().any(char::is_control)) {
                    t.chars().for_each(|c| p.key(Some(c)));
                }
            }
        }
        self.redraw();
    }

    /// 😀 recherche du sélecteur d'émojis : elle reçoit le clavier ; Échap la ferme
    pub(crate) fn key_emoji_search(&mut self, key: &Key, text: Option<&str>) {
        let Some(p) = self.ui.pop.as_mut().and_then(|p| p.pick.as_mut()) else {
            return;
        };
        match key {
            Key::Named(NamedKey::Escape) => p.recherche = None,
            Key::Named(NamedKey::Backspace) => p.key(None),
            _ => {
                if let Some(t) = text.filter(|t| !t.chars().any(char::is_control)) {
                    t.chars().for_each(|c| p.key(Some(c)));
                }
            }
        }
        self.redraw();
    }

    /// 🔎 recherche du panneau Indicateurs : elle reçoit le clavier ; Échap la vide et la ferme
    pub(crate) fn key_ind_search(&mut self, key: &Key, text: Option<&str>) {
        let p = &mut self.watch.panels;
        match key {
            Key::Named(NamedKey::Escape) => p.search = None,
            Key::Named(NamedKey::Backspace) => p.search_key(None),
            _ => text
                .into_iter()
                .flat_map(|t| t.chars())
                .filter(|c| !c.is_control())
                .for_each(|c| p.search_key(Some(c))),
        }
        self.redraw();
    }

    /// ⌨ 📱 A key of the page's hidden field (the phone's keyboard: « Backspace », « Enter », « Escape »…) or its
    /// text, in a search that has the keyboard: it goes where the computer's keys go ([`Focus::pick`]) and does what
    /// they do there. (`App::soft_key` and `App::paste_text` had a list of their own, written apart: the erase key
    /// did nothing in the Actualités, the alerts panel, the Explorer's address nor the Indicators panel, and Enter
    /// emptied the searches of the Indicators page, the Symbols panel and the emojis.) `false`: none of those
    /// searches has the keyboard (the page's own list goes on).
    pub(crate) fn touche_page(&mut self, k: Option<&str>, text: Option<&str>) -> bool {
        let key = match k {
            Some("Backspace") => Key::Named(NamedKey::Backspace),
            Some("Enter") => Key::Named(NamedKey::Enter),
            Some("Escape") => Key::Named(NamedKey::Escape),
            Some("ArrowUp") => Key::Named(NamedKey::ArrowUp),
            Some("ArrowDown") => Key::Named(NamedKey::ArrowDown),
            Some(_) => return false,
            None => Key::Unidentified(winit::keyboard::NativeKey::Unidentified),
        };
        let text: Option<String> = text.map(|t| t.chars().filter(|c| !c.is_control()).collect());
        let text = text.as_deref();
        match self.focus(&key) {
            Focus::IndSearch => self.key_ind_search(&key, text),
            Focus::EmojiSearch => self.key_emoji_search(&key, text),
            Focus::FxSearch => self.key_fx_search(&key, text),
            Focus::Actu => self.key_actu(&key, text),
            Focus::Hl => self.key_hl(&key, text),
            Focus::Alertes => self.key_alertes(&key, text),
            // 🔎 the market panel's search (Jee, 24 Nov: the iPhone's keyboard typed nothing in it)
            Focus::Marches => self.key_marches_with(&key, text),
            _ => return false,
        }
        true
    }

    /// ⚡ 📅 📰 Escape closes the news card or sheet, then a release's card, then empties and leaves the Actualités
    /// search; the other keys type in that search.
    pub(crate) fn key_actu(&mut self, key: &Key, text: Option<&str>) {
        // (Ctrl + V: the clipboard on the PC app; in the browser the page's paste arrives by `paste_text`)
        let paste = self.pointer.mods.control_key() && matches!(key, Key::Character(c) if c.eq_ignore_ascii_case("v"));
        let clip = if paste {
            self.clipboard.as_mut().and_then(|c| c.get_text().ok())
        } else {
            None
        };
        let ctrl = self.pointer.mods.control_key();
        let e = &mut self.watch.eco;
        match key {
            Key::Named(NamedKey::Escape) => {
                if self.ui.eclair.take().is_none() && e.carte.take().is_none() {
                    e.recherche = None;
                }
            }
            Key::Named(NamedKey::Backspace) => e.news_key(None),
            _ if paste => clip.unwrap_or_default().chars().filter(|c| !c.is_control()).for_each(|c| e.news_key(Some(c))),
            _ => text
                .filter(|_| !ctrl)
                .into_iter()
                .flat_map(|t| t.chars())
                .filter(|c| !c.is_control())
                .for_each(|c| e.news_key(Some(c))),
        }
        self.redraw();
    }

    /// 🔔 The alerts panel's field « Chercher une alerte »: Escape empties and leaves it, Ctrl + V pastes (PC app; in
    /// the browser the page's paste arrives by `paste_text`), the other keys type.
    pub(crate) fn key_alertes(&mut self, key: &Key, text: Option<&str>) {
        let ctrl = self.pointer.mods.control_key();
        let paste = ctrl && matches!(key, Key::Character(c) if c.eq_ignore_ascii_case("v"));
        let clip = if paste {
            self.clipboard.as_mut().and_then(|c| c.get_text().ok())
        } else {
            None
        };
        let Some(e) = self.ui.notifs.as_mut().map(|n| &mut n.alertes) else { return };
        match key {
            Key::Named(NamedKey::Escape) => e.recherche = None,
            Key::Named(NamedKey::Backspace) => e.taper(None),
            _ if paste => clip.unwrap_or_default().chars().filter(|c| !c.is_control()).for_each(|c| e.taper(Some(c))),
            _ => text
                .filter(|_| !ctrl)
                .into_iter()
                .flat_map(|t| t.chars())
                .filter(|c| !c.is_control())
                .for_each(|c| e.taper(Some(c))),
        }
        self.redraw();
    }

    /// 🧭 The Explorer Hyperliquid's address field: Escape leaves it, Enter reads the address, the other keys type.
    pub(crate) fn key_hl(&mut self, key: &Key, text: Option<&str>) {
        // (Ctrl + V: the clipboard on the PC app; in the browser the page's paste arrives by `paste_text`)
        let paste = self.pointer.mods.control_key() && matches!(key, Key::Character(c) if c.eq_ignore_ascii_case("v"));
        let clip = if paste {
            self.clipboard.as_mut().and_then(|c| c.get_text().ok())
        } else {
            None
        };
        let ctrl = self.pointer.mods.control_key();
        let e = &mut self.watch.explorer;
        let c = match key {
            Key::Named(NamedKey::Escape) => {
                e.saisie = None;
                argos_render::hlexplorer::ExClick::Changed
            }
            Key::Named(NamedKey::Enter) => e.valider(),
            Key::Named(NamedKey::Backspace) => e.taper(None),
            _ if paste => {
                let mut c = argos_render::hlexplorer::ExClick::Changed;
                for ch in clip.unwrap_or_default().chars() {
                    c = e.taper(Some(ch));
                }
                c
            }
            _ => {
                let mut c = argos_render::hlexplorer::ExClick::Changed;
                for ch in text.filter(|_| !ctrl).into_iter().flat_map(|t| t.chars()).filter(|c| !c.is_control()) {
                    c = e.taper(Some(ch));
                }
                c
            }
        };
        self.explorer_click(c);
        self.redraw();
    }

    /// « Aller à » ouvert : le clavier tape la date (Entrée = aller, Échap = fermer)
    pub(crate) fn key_goto(&mut self, event: winit::event::KeyEvent) {
        let tz = self.tz_offset_s();
        let Some(g) = self.ui.goto.as_mut() else { return };
        let out = match &event.logical_key {
            Key::Named(NamedKey::Escape) => Outcome::Close,
            Key::Named(NamedKey::Enter) => g.submit(tz).map_or(Outcome::Stay, Outcome::Go),
            Key::Named(NamedKey::Backspace) => {
                g.backspace();
                Outcome::Stay
            }
            Key::Character(c) => {
                c.chars().for_each(|ch| g.type_char(ch));
                Outcome::Stay
            }
            _ => Outcome::Stay,
        };
        self.goto_outcome(out);
    }

    /// clavier : la table d'actions dit quelle action fait chaque touche
    /// (seule la touche compte : les tests l'appellent sans événement winit)
    pub(crate) fn key_chart(&mut self, key: &Key) {
        // 🪟 Échap : annule le choix du symbole d'une case du split
        if matches!(key, Key::Named(NamedKey::Escape)) && self.ui.pane_symbole.take().is_some() {
            self.banner = None;
            return self.redraw();
        }
        // ⌖ Échap : the ticket stops waiting for a touch on the chart
        if matches!(key, Key::Named(NamedKey::Escape)) && std::mem::take(&mut self.watch.panels.trading.cible) {
            self.banner = None;
            return self.redraw();
        }
        // objets : Entrée termine un dessin à points libres, Suppr / Retour arrière efface l'objet sélectionné
        match key {
            Key::Named(NamedKey::Enter) if self.draft.is_some() => {
                self.finish_draft();
                return self.redraw();
            }
            Key::Named(NamedKey::Delete | NamedKey::Backspace) if self.selected.is_some() => {
                if let Some(i) = self.selected.take().filter(|&i| self.chart.drawings.get(i).is_some_and(|d| !d.locked())) {
                    self.chart.drawings.remove(i);
                }
                return self.redraw();
            }
            _ => {}
        }
        let mods = chart_actions::Mods {
            ctrl: self.pointer.mods.control_key(),
            alt: self.pointer.mods.alt_key(),
            shift: self.pointer.mods.shift_key(),
        };
        if let Some(a) = to_key(key).and_then(|k| chart_actions::from_key(k, mods)) {
            self.run_action(a);
        }
    }
}
