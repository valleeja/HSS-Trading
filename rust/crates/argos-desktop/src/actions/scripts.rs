//! 📝 Argos Script : éditeur, exécution des scripts, clavier du cell.

use crate::*;

impl App {
    /// 🧮 What the user scripts learn beyond OHLCV, the same as the built-in scripts (`integre`): the real delta of
    /// each bar (`reel`: `orderflow_chart`), whether the last bar is still forming, the chart's symbol, and the
    /// lower-timeframe bars already loaded for `request.security_lower_tf`. Built once per refresh.
    pub(crate) fn options_scripts(&self, reel: &[argos_core::orderflow::FlowBar]) -> argos_script::lang::RunOptions {
        let mut o = argos_render::indicators::run_options(&self.chart.bars, self.chart.interval.secs(), now_s(), reel);
        if let Source::Live { exchange, base } = &self.chart.source {
            (o.ticker, o.exchange) = (exchange.ticker(base), exchange.name().to_string());
        }
        // (🔬 the lower-timeframe bars already loaded, shared: never copied by a run)
        o.intrabars = self.feeds.script_bars.clone();
        o
    }

    /// 📜 Fait rouler les scripts de la chart ; un script en erreur garde son dernier dessin. A script that never ran,
    /// or that has no output call, gets no pane: one line of the price legend saying why (« ⚠ erreur ligne n »,
    /// « ⚠ ne trace rien »).
    pub(crate) fn run_scripts(
        &self,
        old: Vec<argos_render::scriptview::ScriptView>,
        opts: &argos_script::lang::RunOptions,
    ) -> Vec<argos_render::scriptview::ScriptView> {
        use argos_render::scriptview::{ScriptView, NE_TRACE_RIEN};
        self.scripts
            .iter()
            .filter(|s| s.on)
            .map(
                // (⚙ with its chosen values, step 3)
                |s| match argos_script::lang::run_with(&s.src, &self.chart.bars, self.chart.interval.secs(), &self.valeurs_du_script(&s.id), opts) {
                    Ok(mut out) => {
                        let rien = !out.has_output_calls;
                        // (no bars yet: the declaration never ran, so no pane — a run that fails later must not leave
                        // an empty one, pc-legende-erreur of editeur-pine-1)
                        out.overlay |= rien || self.chart.bars.is_empty();
                        ScriptView {
                            name: s.name.clone(),
                            out,
                            hidden: s.hidden,
                            error: rien.then(|| NE_TRACE_RIEN.to_string()),
                            style: Default::default(),
                            id: s.id.clone(),
                        }
                    }
                    Err(e) => {
                        let out = old.iter().find(|v| v.name == s.name).map_or_else(
                            || argos_script::lang::Output {
                                overlay: true,
                                ..Default::default()
                            },
                            |v| v.out.clone(),
                        );
                        ScriptView {
                            name: s.name.clone(),
                            out,
                            hidden: s.hidden,
                            error: Some(format!("Ligne {} : {}", e.line, e.msg)),
                            style: Default::default(),
                            id: s.id.clone(),
                        }
                    }
                },
            )
            .collect()
    }

    /// 📜 Fenêtre de l'éditeur : au milieu de la zone de la chart, comme v1.
    pub(crate) fn ed_rect(&self) -> Rect {
        let (aw, ah) = (self.area.0 as f64, self.area.1 as f64);
        // 📖 the Doc on a narrow screen (cell): the whole area, as in the mockup; (tableaux-1 tour 2, F9) the code
        // view and the editor too: from the very top (over the chart's title), never a title cut in half above it
        if aw < argos_render::doc_editeur::ETROIT {
            return Rect::new(self.rw(), self.top_h(), aw, ah);
        }
        let w = (aw - 48.0).clamp(320.0, 1100.0);
        Rect::new(self.rw() + (aw - w) / 2.0, self.top_h() + 20.0, w, (ah - 40.0).max(300.0))
    }

    /// 📜 Ouvre l'éditeur sur un script (`None` : brouillon du modèle de départ), avec le message d'ouverture de v1.
    pub(crate) fn editor_on(&mut self, slot: Option<usize>, src: Option<&str>) {
        use argos_render::editor::{Editor, LogKind, TEMPLATE};
        let mut ed = match slot.and_then(|k| self.scripts.get(k)) {
            Some(s) => Editor::new(&s.name, &s.src, slot),
            None => Editor::new("Mon script", src.unwrap_or(TEMPLATE), None),
        };
        let now = now_s() + self.tz_offset_s();
        ed.log(
            now,
            format!("« {} » ouvert. ▶ pour tester sur la chart, 💾 pour sauvegarder.", ed.name),
            LogKind::Info,
        );
        // ⚠ a script of the chart in error: the cursor on its line, the error in red
        let erreur = slot
            .and_then(|k| self.scripts.get(k))
            .and_then(|s| self.ind.out.scripts.iter().find(|v| v.name == s.name))
            .and_then(|v| v.error.clone());
        match erreur.as_deref() {
            Some(argos_render::scriptview::NE_TRACE_RIEN) => ed.log(
                now,
                "❌ Ce script ne trace rien : il lui faut au moins un plot(), un plotshape() ou un dessin.",
                LogKind::Err,
            ),
            Some(e) => {
                if let Some(n) = e.strip_prefix("Ligne ").and_then(|r| r.split(' ').next()).and_then(|n| n.parse().ok()) {
                    ed.goto(n);
                }
                ed.log(now, format!("❌ {e}"), LogKind::Err);
            }
            None => {}
        }
        self.ui.editor = Some(ed);
        self.redraw();
    }

    /// 📜 Panneau Indicateurs : Mes scripts (le dernier), Nouveau script, Importer du Pine Script (presse-papiers collé).
    pub(crate) fn open_editor(&mut self, o: argos_render::panels::ScriptOpen) {
        use argos_render::panels::ScriptOpen;
        match o {
            ScriptOpen::Mine if !self.scripts.is_empty() => self.editor_on(Some(self.scripts.len() - 1), None),
            ScriptOpen::Import => {
                let clip = self.clipboard.as_mut().and_then(|c| c.get_text().ok()).unwrap_or_default();
                self.editor_on(None, Some(&clip));
                let now = now_s() + self.tz_offset_s();
                if let Some(e) = self.ui.editor.as_mut() {
                    e.log(
                        now,
                        if clip.is_empty() {
                            "🌲 Colle ton script Pine (Ctrl + V) : il roule tel quel, puis ▶ Ajouter à la chart"
                        } else {
                            "🌲 Script Pine collé : il roule tel quel, ▶ pour l'ajouter à la chart"
                        },
                        argos_render::editor::LogKind::Info,
                    );
                }
            }
            _ => self.editor_on(None, None),
        }
    }

    /// 📜 ▶ Tester (`save` = 💾 : garde aussi une version), comme `edRun` de v1 : compile, l'ajoute à la chart, et le
    /// terminal dit combien de tracés et de signaux.
    pub(crate) fn editor_run(&mut self, save: bool) {
        use argos_render::editor::LogKind;
        let Some(mut ed) = self.ui.editor.take() else { return };
        let now = now_s() + self.tz_offset_s();
        ed.log(now, "Compilation en cours…", LogKind::Info);
        let code = ed.text();
        // 🌐 langue reconnue (Pine v4 / v5 / v6, ancien Argos Script, wrun) : traduite en Pine v6 pour le moteur
        let lang = argos_script::lang::dialect::detect(&code);
        if lang != argos_script::lang::dialect::Dialect::Pine6 {
            ed.log(now, format!("🌐 {} reconnu : traduit en Pine v6", lang.name()), LogKind::Info);
        }
        let opts = self.options_scripts(&self.orderflow_chart());
        match argos_script::lang::run_with(&code, &self.chart.bars, self.chart.interval.secs(), &Default::default(), &opts) {
            Err(e) => {
                ed.goto(e.line);
                ed.log(now, format!("❌ Ligne {} : {}", e.line, e.msg), LogKind::Err);
            }
            // (nothing is saved or added to the chart: no bars yet, or a script that draws nothing — a paste cut short)
            Ok(_) if self.chart.bars.is_empty() => {
                ed.log(now, "⏳ Pas encore de bougies sur la chart : réessaie dans un instant.", LogKind::Warn);
            }
            Ok(out) if !out.has_output_calls => {
                ed.log(
                    now,
                    "❌ Ce script ne trace rien : il lui faut au moins un plot(), un plotshape() ou un dessin.",
                    LogKind::Err,
                );
                ed.log(
                    now,
                    format!("Le code a {} lignes : vérifie qu'il a été collé en entier.", code.lines().count()),
                    LogKind::Note,
                );
            }
            Ok(out) => {
                if (ed.name.is_empty() || ed.name == "Mon script") && !out.title.is_empty() {
                    ed.name = out.title.clone();
                }
                let k = match ed.slot.filter(|k| *k < self.scripts.len()) {
                    Some(k) => k,
                    None => {
                        self.scripts.push(SavedScript::default());
                        self.ids_des_scripts();
                        self.scripts.len() - 1
                    }
                };
                let s = &mut self.scripts[k];
                (s.name, s.src, s.on) = (ed.name.clone(), code.clone(), true);
                if save {
                    s.versions.insert(0, (now_s(), code));
                    s.versions.truncate(20);
                }
                ed.slot = Some(k);
                Self::write_prefs("scripts.json", &self.scripts);
                self.ind.key = Default::default();
                // (the verdict last, so that it stays in the terminal's visible lines: the 📋 lines, the notes, the
                // candle lines of request.security_lower_tf, then ✅ and 💾 — editeur-pine-1 R2)
                for (i, m) in out.logs.iter().rev().take(5).rev() {
                    ed.log(now, format!("📋 bougie {i} : {m}"), LogKind::Info);
                }
                for (l, m) in out.notes.iter().take(8) {
                    ed.log(now, ligne_terminal(*l, m), LogKind::Info);
                }
                // 🔬 its lower-timeframe bars: asked now, the terminal says where they are
                let chart = self.chart.interval.secs();
                let petites: Vec<i64> = out.lower_tf_asked.iter().copied().filter(|s| *s < chart).collect();
                if !petites.is_empty() {
                    self.refresh_scripts_mtf(&petites);
                    for sec in petites {
                        if let Some((m, k)) = self.ligne_petites(sec) {
                            ed.log(now, m, k);
                        }
                    }
                }
                let plural = |n: usize, one: &str, many: &str| format!("{n} {}", if n > 1 { many } else { one });
                ed.log(
                    now,
                    format!(
                        "✅ Compilé : {}, {}{}.",
                        plural(out.plots.len(), "tracé", "tracés"),
                        plural(out.shapes.len(), "signal", "signaux"),
                        if out.overlay { " · sur le prix" } else { " · panneau du bas" }
                    ),
                    LogKind::Ok,
                );
                if rien_a_tracer(&out) {
                    ed.log(now, "⚠ Rien à tracer sur ces bougies pour l'instant.", LogKind::Warn);
                }
                if save {
                    ed.log(now, format!("💾 « {} » sauvegardé.", ed.name), LogKind::Ok);
                }
            }
        }
        self.ui.editor = Some(ed);
        self.redraw();
    }

    /// 📜 Clic (ou Entrée dans un champ) dans l'éditeur, comme les boutons de v1.
    pub(crate) fn editor_click(&mut self, c: argos_render::editor::EdClick) {
        use argos_render::editor::{EdBtn, EdClick, EdItem, Extra, LogKind};
        let now = now_s() + self.tz_offset_s();
        let tz = self.tz_offset_s();
        let Some(ed) = self.ui.editor.as_mut() else { return };
        let say = |ed: &mut argos_render::editor::Editor, m: &str| ed.log(now, m, LogKind::Info);
        // { } B1: a Rust indicator's code: read only, and no copy either (it is not a script)
        if ed.rust
            && matches!(
                c,
                EdClick::Btn(EdBtn::Run) | EdClick::Item(EdItem::Save | EdItem::Copy | EdItem::Rename | EdItem::History | EdItem::Delete)
            )
        {
            say(ed, "{ } Code Rust en lecture seule : l'indicateur est calculé en Rust.");
            return self.redraw();
        }
        // { } code d'un indicateur intégré : lecture seule (« Créer une copie » pour le modifier ou le tester)
        if ed.lecture_seule
            && matches!(
                c,
                EdClick::Btn(EdBtn::Run) | EdClick::Item(EdItem::Save | EdItem::Rename | EdItem::History | EdItem::Delete)
            )
        {
            say(ed, "{ } Code en lecture seule : ▾ Créer une copie pour le modifier ou le tester.");
            return self.redraw();
        }
        match c {
            EdClick::Btn(EdBtn::Run) | EdClick::Item(EdItem::Save) => return self.editor_run(c == EdClick::Item(EdItem::Save)),
            EdClick::Btn(EdBtn::Close) => self.ui.editor = None,
            // 📖 the Doc in place of the code (again: back to the code)
            EdClick::Btn(EdBtn::Help) => ed.basculer_doc(),
            // ⧉ Copier (a code block, exactly as written) / ✨ Copier pour l'IA (the tab's file for an AI)
            EdClick::Doc(a) => {
                use argos_render::doc_editeur::DocAct;
                let (texte, avis) = match a {
                    DocAct::CopierBloc(k) => (ed.doc.texte_bloc(k), None),
                    DocAct::CopierIa => (
                        Some(ed.doc.onglet.llms().to_string()),
                        Some(format!("Copié : fichier pour l'IA ({})", ed.doc.onglet.nom())),
                    ),
                    DocAct::Code | DocAct::Fermer => (None, None),
                };
                if let Some(t) = texte {
                    let ok = self.clipboard.as_mut().is_some_and(|c| c.set_text(t).is_ok());
                    // (the notice on the Doc itself: the chart's banner is under the editor)
                    let avis = if ok { avis } else { Some("⚠️ Presse-papiers indisponible".into()) };
                    if let Some(s) = avis {
                        ed.doc.avis(s);
                    }
                }
            }
            EdClick::Btn(EdBtn::Claude) => say(
                ed,
                "✨ Claude qui écrit le script : arrive avec l'assistant Argos de la version Rust (comme v1)",
            ),
            EdClick::Btn(EdBtn::Pine) => say(
                ed,
                "🌲 Argos Script lit le Pine v6 tel quel : colle ton script (Ctrl + V), puis ▶ Ajouter à la chart",
            ),
            EdClick::Btn(EdBtn::Share) => say(
                ed,
                "📤 Pour publier, crée un compte Argos (bouton Connect) : chaque indicateur est signé avec ta clé",
            ),
            EdClick::Item(EdItem::Copy) => ed.extra = Extra::Prompt(EdItem::Copy, format!("{} (copie)", ed.name)),
            EdClick::Item(EdItem::Rename) => ed.extra = Extra::Prompt(EdItem::Rename, ed.name.clone()),
            EdClick::Item(EdItem::History) => {
                let v = ed.slot.and_then(|k| self.scripts.get(k)).map(|s| s.versions.clone()).unwrap_or_default();
                // « 26 oct. 12:20 », comme v1
                const MOIS: [&str; 12] = [
                    "janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.", "oct.", "nov.", "déc.",
                ];
                let when = |t: i64| {
                    chrono::DateTime::from_timestamp(t + tz, 0).map_or_else(String::new, |d| {
                        use chrono::Datelike;
                        format!("{} {} {}", d.day(), MOIS[d.month0() as usize], d.format("%H:%M"))
                    })
                };
                ed.extra = Extra::History(v.iter().map(|(t, code)| (when(*t), code.lines().count())).collect());
            }
            EdClick::Version(i) => {
                if let Some((_, code)) = ed.slot.and_then(|k| self.scripts.get(k)).and_then(|s| s.versions.get(i)) {
                    ed.set_text(code);
                    ed.extra = Extra::None;
                    say(ed, "🕘 Version restaurée (pas encore sauvegardée).");
                }
            }
            EdClick::PromptOk => {
                let Extra::Prompt(kind, t) = std::mem::replace(&mut ed.extra, Extra::None) else {
                    return;
                };
                let t = t.trim().to_string();
                if kind == EdItem::Rename {
                    if !t.is_empty() {
                        ed.name = t.clone();
                    }
                    if let Some(s) = ed.slot.and_then(|k| self.scripts.get_mut(k)) {
                        s.name = ed.name.clone();
                        Self::write_prefs("scripts.json", &self.scripts);
                        self.ind.key = Default::default();
                    }
                    let m = format!("✏️ Renommé en « {} ».", ed.name);
                    say(ed, &m);
                } else {
                    let name = if t.is_empty() { format!("{} (copie)", ed.name) } else { t };
                    self.scripts.push(SavedScript {
                        name,
                        src: ed.text(),
                        ..Default::default()
                    });
                    Self::write_prefs("scripts.json", &self.scripts);
                    self.editor_on(Some(self.scripts.len() - 1), None);
                    if let Some(e) = self.ui.editor.as_mut() {
                        e.log(now, "📄 Copie créée.", LogKind::Info);
                    }
                }
            }
            EdClick::Item(EdItem::New) => return self.editor_on(None, None),
            // 🗑️ : le 1er clic arme, le 2e supprime
            EdClick::Item(EdItem::Delete) if !ed.armed => ed.armed = true,
            EdClick::Item(EdItem::Delete) => {
                if let Some(k) = ed.slot.filter(|k| *k < self.scripts.len()) {
                    // (📐 its pane height goes with it)
                    let id = self.scripts.remove(k).id;
                    self.chart.settings.script_bands.remove(&id);
                    self.tab_oublier_script(&id);
                    Self::write_prefs("scripts.json", &self.scripts);
                    self.ind.key = Default::default();
                }
                self.ui.editor = None;
                self.banner = Some("Script supprimé".into());
            }
            EdClick::Btn(EdBtn::Menu) | EdClick::Caret | EdClick::None => {}
        }
        self.redraw();
    }

    /// 📋 Texte collé ou tapé dans le champ caché de la page (navigateur : clavier du cell, accents composés) : il
    /// va là où on tape — boîte de clé, 🔎 recherche Ctrl + K, assistant, éditeur.
    pub(crate) fn paste_text(&mut self, text: String) {
        // 🔑 la boîte de clé est devant tout (elle prend aussi les clics) : le texte collé va d'abord à elle, même si
        // une recherche est restée ouverte derrière
        if let Some(b) = self.session.key_box.as_mut() {
            b.type_text(&text);
            return self.redraw();
        }
        // (🔔 🧭 📰 the searches — alerts, the Explorer's address, the Actualités, the Indicators, the emojis, the
        // Symbols, the pill's sheet — took the text before this: `App::touche_page`, the computer keyboard's way)
        if let Some(g) = self.ui.support.as_mut().filter(|g| g.focus.is_some()) {
            g.taper(&text);
            return self.redraw();
        }
        if let Some(p) = self.ui.palette.as_mut() {
            p.tape(&text);
        } else if self.ui.ai_open && self.ai.typing {
            self.ai.input.push_str(text.trim_end());
        } else if let Some(e) = self.ui.editor.as_mut() {
            e.insert(&text);
        } else if let Some(b) = self.ui.name_box.as_mut() {
            text.chars().filter(|c| !c.is_control()).for_each(|c| b.type_char(c));
        } else if let Some(p) = self.ui.add.as_mut() {
            // 📱 feuille Symbole : « Symbole, nom… » (le clavier du cell)
            text.chars().filter(|c| !c.is_control()).for_each(|c| p.type_char(c));
            self.search_add();
        } else if let Some(mut f) = self.ui.j_form.take() {
            // 📓 the journal's trade sheet (notes, prices…): same as typing
            text.chars().filter(|c| !c.is_control()).for_each(|c| f.type_char(c));
            self.j_form_result(f, argos_render::journal::FormClick::Changed);
        } else if let Some(mut dl) = self.ui.ind_dlg.take() {
            text.chars().for_each(|c| dl.type_char(c));
            self.ui.ind_dlg = Some(dl);
        } else if let Some(mut dl) = self.ui.obj_dlg.take() {
            if let Some(d) = self.chart.drawings.get_mut(dl.index) {
                text.chars().for_each(|c| dl.type_char(c, d));
            }
            self.ui.obj_dlg = Some(dl);
        }
        self.redraw();
    }

    /// 📱 Effacer / Entrée / Échap / ↑ ↓ du champ caché de la page : vers la boîte de clé, la recherche Ctrl + K,
    /// l'assistant ou l'éditeur.
    pub(crate) fn soft_key(&mut self, k: &str) {
        use argos_render::editor::EdKey;
        // 🔑 la boîte de clé d'abord (comme pour coller)
        if let Some(b) = self.session.key_box.as_mut() {
            match k {
                "Backspace" => b.backspace(),
                "Enter" if b.complete() => return self.key_save(),
                "Escape" => {
                    b.wipe();
                    self.session.key_box = None;
                }
                _ => {}
            }
            return self.redraw();
        }
        if self.ui.support.as_ref().is_some_and(|g| g.focus.is_some()) {
            return self.key_support_nom(k, None);
        }
        // (the searches took their keys before this: `App::touche_page`, the computer keyboard's way)
        if let Some(p) = self.ui.palette.as_mut() {
            match k {
                "Backspace" => p.efface(),
                "ArrowUp" => p.bouge(-1),
                "ArrowDown" => p.bouge(1),
                "Enter" => {
                    if let Some(c) = p.choisie() {
                        return self.palette_cible(c);
                    }
                }
                _ => self.ui.palette = None,
            }
        } else if self.ui.ai_open && self.ai.typing {
            match k {
                "Backspace" => _ = self.ai.input.pop(),
                "Enter" => {
                    if let Some(q) = self.ai.take_input() {
                        return self.ai_send(q);
                    }
                }
                _ => return self.toggle_ai(),
            }
        } else if let Some(e) = self.ui.editor.as_mut() {
            match k {
                "Backspace" => _ = e.key(EdKey::Backspace),
                "Enter" => _ = e.key(EdKey::Enter),
                // (📖 Doc search: ↑ / ↓ previous / next match, Échap clears)
                "ArrowUp" if e.doc_ouverte => _ = e.key(EdKey::Up),
                "ArrowDown" if e.doc_ouverte => _ = e.key(EdKey::Down),
                "Escape" if e.doc_ouverte => _ = e.escape(),
                _ => {}
            }
        } else if self.ui.name_box.is_some() || self.ui.add.is_some() {
            // 📱 boîte de nom, feuille Symbole : Effacer, Entrée (OK / ajouter le 1er), Échap (fermer)
            match k {
                "Backspace" => {
                    if let Some(b) = self.ui.name_box.as_mut() {
                        b.backspace();
                    } else if let Some(p) = self.ui.add.as_mut() {
                        p.backspace();
                        self.search_add();
                    }
                }
                "Enter" if self.ui.name_box.is_some() => return self.name_ok(),
                "Enter" => {
                    if let Some(id) = self.ui.add.as_ref().and_then(|p| p.first()) {
                        self.add_symbol(&id);
                    }
                }
                "Escape" => (self.ui.name_box, self.ui.add) = (None, None),
                _ => {}
            }
        } else if let Some(mut dl) = self.ui.ind_dlg.take() {
            use argos_render::inddlg::IndClick;
            let c = match k {
                "Backspace" => {
                    dl.backspace();
                    IndClick::None
                }
                "Enter" => dl.enter(),
                "Escape" => IndClick::Close(false),
                _ => IndClick::None,
            };
            self.ind_dlg_result(dl, c);
        }
        self.redraw();
    }
}

impl App {
    /// { } Le code d'un indicateur intégré (bouton { } de la légende, comme TradingView), dans l'éditeur ✎ en lecture
    /// seule ; rien s'il n'a pas de source (le bouton n'est alors pas affiché).
    pub(crate) fn code_indicateur(&mut self, i: argos_render::indicators::Ind) {
        use argos_render::editor::{Editor, LogKind};
        // { } B1: an indicator kept in Rust shows its real Rust source, read only (no copy: it is not a script)
        if let Some(src) = argos_render::sources::source_rust(i) {
            let mut ed = Editor::new(&format!("{} (code Rust)", i.name()), &src, None);
            (ed.lecture_seule, ed.rust) = (true, true);
            ed.log(
                now_s() + self.tz_offset_s(),
                format!("{{ }} Code Rust de « {} », en lecture seule : l'indicateur est calculé en Rust.", i.name()),
                LogKind::Info,
            );
            self.ui.editor = Some(ed);
            return self.redraw();
        }
        let Some(src) = argos_render::sources::source_de(i) else { return };
        let mut ed = Editor::new(&format!("{} (code)", i.name()), src, None);
        ed.lecture_seule = true;
        ed.log(
            now_s() + self.tz_offset_s(),
            format!(
                "{{ }} Code de « {} » ({}), en lecture seule. ▾ Créer une copie pour le modifier.",
                i.name(),
                if argos_script::catalogue::source(argos_render::catalogue::fiche(i)).is_some() {
                    "Argos Script"
                } else {
                    "Pine"
                }
            ),
            LogKind::Info,
        );
        self.ui.editor = Some(ed);
        self.redraw();
    }
}

/// The run drew nothing on these bars (every plot empty, no shape, line, fill, drawing, table, colour or trade),
/// although the script has output calls.
pub(crate) fn rien_a_tracer(o: &argos_script::lang::Output) -> bool {
    o.plots.iter().all(|p| !p.values.iter().any(|v| v.is_finite()))
        && o.shapes.is_empty()
        && o.hlines.is_empty()
        && o.fills.is_empty()
        && o.drawings.iter().all(|d| d.deleted)
        && o.tables.iter().all(|t| t.deleted)
        && o.linefills.iter().all(|l| l.deleted)
        && o.polylines.iter().all(|p| p.deleted)
        && o.candles.iter().all(|c| c.ohlc.iter().all(|v| !v.iter().all(|x| x.is_finite())))
        && o.bgcolor.iter().chain(&o.barcolor).all(Option::is_none)
        && o.strategie.as_ref().is_none_or(|b| b.trades.is_empty() && b.ouverts.is_empty())
}

/// ⚠️ A script's note as the editor's terminal writes it: one ⚠️, then its line.
pub(crate) fn ligne_terminal(ligne: u32, note: &str) -> String {
    format!("⚠️ Ligne {ligne} : {note}")
}

#[cfg(test)]
mod tests {
    /// ⚠️ Jee (19 Nov): the editor's terminal wrote « ⚠️ Ligne 10 : ⚠️ La fonction… ». Each note line has one ⚠️.
    #[test]
    fn terminal_un_seul_avertissement_par_ligne() {
        let bars: Vec<argos_core::Bar> = (0..300)
            .map(|i| {
                let c = 100.0 + (i as f64 / 7.0).sin() * 5.0;
                argos_core::Bar {
                    time: i * 1800,
                    open: c - 0.5,
                    high: c + 1.0,
                    low: c - 1.0,
                    close: c,
                    volume: 1.0,
                }
            })
            .collect();
        let src = "//@version=6\nindicator(\"t\")\nr = ta.rsi(close, 14)\nplotshape(r > 50 and ta.crossover(close, ta.sma(close, 20)), \"x\", shape.circle)\n";
        let out = argos_script::lang::run(src, &bars, 1800, &Default::default()).unwrap();
        assert!(out.notes.iter().any(|(_, m)| m.contains("chaque bougie")), "{:?}", out.notes);
        for (l, m) in &out.notes {
            let ligne = super::ligne_terminal(*l, m);
            assert_eq!(ligne.matches('⚠').count(), 1, "{ligne}");
        }
    }
}
