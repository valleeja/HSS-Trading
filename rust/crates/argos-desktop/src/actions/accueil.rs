//! ⌂ Accueil, 🔎 recherche Ctrl + K, ⌨️ raccourcis Ctrl + / et ‹ › repli des barres : le branchement dans la fenêtre.
//! Le dessin et les calculs sont dans `argos_render::{accueil, recherche, raccourcis}`.

use crate::*;
use argos_core::money::Money;
use argos_render::accueil::{AccClick, AccueilPage, Donnees, Section};
use argos_render::recherche::{Cible, Palette, Reglage};

/// Largeur et hauteur des languettes ‹ › qui replient les barres.
pub(crate) const TAB: (f64, f64) = (14.0, 44.0);

impl App {
    /// ⌂ Ouvre l'Accueil (les menus se ferment ; au tout premier lancement, ses 3 écrans).
    pub(crate) fn open_accueil(&mut self) {
        (self.ui.list, self.ui.sheet, self.ui.argos, self.ui.palette) = (None, None, None, None);
        (self.ui.ac_page, self.ui.pf_page, self.ui.mk_page, self.ui.bt_page) = (None, None, None, None);
        let ac = argos_render::academy::AcademyPage::new(Self::read_prefs("academy.json"));
        let (fait, total) = ac.done_count();
        let prochain = ac.next_guide().map(|i| argos_render::academy::titre(i).to_string());
        self.ui.accueil_ac = (fait, total, prochain);
        self.ui.accueil = Some(AccueilPage {
            layouts: self.liste_layouts(),
            favoris: self.favoris_layouts(),
            profil: self.top.prefs.profil.min(argos_render::accueil::PROFILS.len() - 1),
            bienvenue: (!self.top.prefs.bienvenue_vue).then_some(0),
            heure: ((now_s() + self.tz_offset_s()).rem_euclid(86_400) / 3600) as u32,
            ..Default::default()
        });
        self.refresh_eco();
        self.redraw();
    }

    /// 📱 Première image : on sait enfin si c'est un téléphone. Sur cell, on arrive toujours sur la chart (un volet
    /// restauré, ouvert sur le PC, y prendrait tout l'écran ; Argos AI aussi) ; l'Accueil seulement au tout premier
    /// lancement. Sur PC et tablette : rien ne change, tout est restauré (volets compris). Une seule fois.
    pub(crate) fn premiere_image(&mut self, phone: bool) {
        if std::mem::replace(&mut self.ui.depart_vu, true) || !phone {
            return;
        }
        // 👆 un doigt a déjà touché avant la première image (page encore en chargement) : on ne défait pas ce qu'il a
        // ouvert — sinon le premier toucher après l'ouverture était perdu
        if self.pointer.dernier_doigt.is_finite() {
            return;
        }
        self.watch.prefs.open = false;
        (self.ui.ai_open, self.ui.sheet) = (false, None);
        if self.top.prefs.bienvenue_vue && !self.top.prefs.accueil {
            self.ui.accueil = None; // « Accueil au lancement » allumé : il reste
        }
    }

    /// ⌂ Les vraies données des cartes de l'Accueil.
    pub(crate) fn accueil_donnees(&self) -> Donnees {
        use argos_render::journal;
        let now_ms = journal::now_ms();
        // 🛡️ gardien PropFirm (règles du Journal)
        let j = &self.watch.panels.journal;
        let since = if j.cfg.start.is_empty() {
            0
        } else {
            journal::parse_date(&j.cfg.start).unwrap_or(0)
        };
        let f = argos_core::journal::propfirm(&j.trades, &j.cfg, now_ms, since, &|ms| j.propfirm_day(ms), &|v| journal::money(Some(v)));
        let garde = argos_render::accueil::Garde {
            perte_jour: (-f.today_pnl).max(Money::ZERO),
            limite_jour: f.daily_limit,
            pnl: f.pnl,
            objectif: f.target,
            jours: f.days,
            jours_min: j.cfg.min_days,
        };
        // 📅 3 prochaines news
        let news = argos_render::accueil::prochaines_news(&self.watch.eco.agenda, now_s(), 3)
            .into_iter()
            .map(|e| (e.title.clone(), e.country.clone(), e.ts, e.impact == "High"))
            .collect();
        // 💼 compte démo : équité = solde + latent ; jour (net, comme le solde) = trades fermés aujourd'hui (coûts
        // déduits) + latent − coûts d'entrée déjà payés des positions ouvertes
        let d = &self.demo;
        let mut latent = Money::ZERO;
        let mut positions = vec![];
        for p in &d.positions {
            let v = Money::from_f64(self.demo_price(&p.base).map_or(0.0, |px| p.pnl(px)));
            latent += v;
            let sens = if p.side == argos_core::types::Side::Long { "long" } else { "short" };
            positions.push((format!("{} {sens}", p.base), v));
        }
        let today = |ms: i64| journal::date_txt(ms);
        // closed_at est en secondes, date_txt attend des millisecondes
        let closed_today: Money = d.history.iter().filter(|t| today(t.closed_at * 1000) == today(now_ms)).map(|t| t.net()).sum();
        // 💸 ⚡ ce que les positions ouvertes ont déjà payé (coûts d'entrée, funding), déjà retiré du solde
        let entrees: Money = d.positions.iter().map(|p| d.deja_paye(p)).sum();
        let demo = argos_render::accueil::Demo {
            equite: d.balance + latent,
            jour: closed_today + latent - entrees,
            positions,
        };
        // 🌍 4 premiers symboles de la watchlist, avec la variation sur 24 h
        let rows: Vec<&wl::Row> = self.watch.sections.iter().flat_map(|s| &s.rows).collect();
        // 🟩 carte thermique : les 8 premiers symboles de la watchlist et leur variation sur 24 h
        let chaleur = self
            .watch
            .order()
            .iter()
            .filter_map(|c| rows.iter().find(|r| r.coin == *c))
            .take(8)
            .map(|r| {
                let m = r.mark.parse::<f64>().ok().filter(|m| *m > 0.0);
                (r.label.clone(), m.filter(|_| r.prev > 0.0).map(|m| (m - r.prev) / r.prev * 100.0))
            })
            .collect();
        let marches = self
            .watch
            .order()
            .iter()
            .filter_map(|c| rows.iter().find(|r| r.coin == *c))
            .take(4)
            .map(|r| {
                let m = r.mark.parse::<f64>().ok().filter(|m| *m > 0.0);
                let pct = m.filter(|_| r.prev > 0.0).map(|m| (m - r.prev) / r.prev * 100.0);
                (r.label.clone(), if r.mark.is_empty() { "—".into() } else { r.mark.clone() }, pct)
            })
            .collect();
        Donnees {
            academie: self.ui.accueil_ac.clone(),
            garde: Some(garde),
            news,
            demo: Some(demo),
            marches,
            macro_: self.feeds.world.iter().map(|t| (t.name.clone(), t.price, t.chg_pct)).collect(),
            semaine: argos_render::accueil::semaine(&j.trades, now_ms),
            courbe: std::iter::once(d.start).chain(d.curve.iter().map(|c| c.1)).map(|m| m.to_f64()).collect(),
            actus: self.watch.eco.news.iter().take(3).map(|n| (n.title.clone(), n.source.clone(), n.ts)).collect(),
            chaleur,
            maintenant: now_s(),
        }
    }

    /// ⌂ Clic dans l'Accueil.
    pub(crate) fn accueil_click(&mut self) {
        let size = self.window.as_deref().map(ui_size).unwrap_or_default();
        let (x, y) = self.pointer.win_mouse;
        let text = argos_render::text::Text::default();
        let cartes = argos_render::accueil::cartes(&self.accueil_donnees());
        let Some(p) = self.ui.accueil.as_mut() else { return };
        let c = p.click(&text, size.width as f64, size.height as f64, x, y, &cartes);
        self.accueil_result(c);
    }

    /// ⌂ Ce que demande un clic (ou une touche) de l'Accueil.
    pub(crate) fn accueil_result(&mut self, c: AccClick) {
        match c {
            AccClick::None => {}
            AccClick::Close => self.ui.accueil = None,
            AccClick::Profil => {
                if let Some(p) = &self.ui.accueil {
                    self.top.prefs.profil = p.profil;
                    self.save_top();
                }
            }
            AccClick::BienvenueFin => {
                self.top.prefs.bienvenue_vue = true;
                Self::bienvenue_ecrire(true);
                self.save_top();
            }
            AccClick::Section(s) => return self.accueil_section(s),
            AccClick::Reprendre => {
                self.ui.accueil = None;
                self.open_academy();
                if let Some(p) = self.ui.ac_page.as_mut() {
                    if p.resume() == argos_render::academy::AcClick::Changed {
                        Self::write_prefs("academy.json", &p.progress);
                    }
                }
            }
            AccClick::Simulateur => {
                self.ui.accueil = None;
                return self.toggle_simulateur();
            }
            AccClick::Recherche => return self.open_palette(),
            AccClick::NouvelleLayout => {
                self.ui.accueil = None;
                return self.nouvelle_layout(false);
            }
            AccClick::Layout(k) => {
                self.ui.accueil = None;
                return self.clic_layout(k);
            }
            AccClick::Favori(k) => {
                if let Some(nom) = self.nom_layout(k) {
                    self.basculer_favori(&nom);
                }
            }
            AccClick::MenuLayout(k, r) => return self.menu_tuile(k, r),
            // 🫧 premier lancement : Verre sombre ou Verre clair, tout de suite
            AccClick::Theme(clair) => {
                self.top.prefs.theme = if clair { 9 } else { 8 };
                argos_render::set_theme(self.top.prefs.theme);
                self.save_top();
            }
            AccClick::CleApi => {
                self.top.prefs.bienvenue_vue = true;
                Self::bienvenue_ecrire(true);
                self.save_top();
                self.ui.accueil = None;
                (self.watch.prefs.open, self.watch.prefs.panel) = (true, wl::Tool::Exchanges);
                self.save_watch();
            }
        }
        self.redraw();
    }

    /// ⌂ Ouvre la section d'une carte (clic ou lettre) ; l'Accueil se ferme.
    pub(crate) fn accueil_section(&mut self, s: Section) {
        self.ui.accueil = None;
        let panel = |me: &mut Self, t: wl::Tool| {
            (me.watch.prefs.open, me.watch.prefs.panel) = (true, t);
            me.save_watch();
        };
        match s {
            Section::Communaute => {
                let r = Rect::new(self.win_w() / 2.0 - 160.0, self.top_h(), 320.0, 1.0);
                return self.open_list(ListKind::Community, r);
            }
            Section::Academie => return self.open_academy(),
            Section::PropFirm => {
                self.watch.panels.journal.tab = 2;
                panel(self, wl::Tool::Journal);
            }
            Section::Calendrier => {
                panel(self, wl::Tool::Agenda);
                self.refresh_eco();
            }
            Section::Portefeuille => self.ui.pf_page = Some(argos_render::portfolio::PfPage::new()),
            Section::Marches => panel(self, wl::Tool::Watchlist),
            Section::Macro => {
                panel(self, wl::Tool::News);
                self.refresh_eco();
            }
            Section::Journal => {
                self.watch.panels.journal.tab = 1;
                panel(self, wl::Tool::Journal);
            }
            Section::Actualites => {
                panel(self, wl::Tool::News);
                self.refresh_eco();
            }
            Section::Thermique => panel(self, wl::Tool::Watchlist),
        }
        self.redraw();
    }

    /// ⌂ Clavier de l'Accueil : la lettre d'une carte ouvre sa section, Échap ferme (ou finit le premier lancement).
    pub(crate) fn key_accueil(&mut self, key: &Key) {
        let first_run = self.ui.accueil.as_ref().is_some_and(|p| p.bienvenue.is_some());
        match key {
            Key::Named(NamedKey::Escape) => {
                if let Some(p) = self.ui.accueil.as_mut() {
                    let c = p.escape();
                    return self.accueil_result(c);
                }
            }
            Key::Character(c) if !first_run && !self.pointer.mods.control_key() && !self.pointer.mods.alt_key() => {
                if let Some(s) = c.chars().next().and_then(Section::depuis_lettre) {
                    return self.accueil_section(s);
                }
            }
            _ => {}
        }
        self.redraw();
    }

    /// 🔎 Ouvre (ou ferme) la recherche Ctrl + K, avec les symboles de la watchlist.
    pub(crate) fn open_palette(&mut self) {
        if self.ui.palette.take().is_some() {
            return self.redraw();
        }
        (self.ui.list, self.ui.sheet, self.ui.argos) = (None, None, None);
        let syms: Vec<(String, String)> = self
            .watch
            .sections
            .iter()
            .flat_map(|s| &s.rows)
            .map(|r| (r.coin.clone(), r.label.clone()))
            .collect();
        self.ui.palette = Some(Palette::new(&syms));
        self.redraw();
    }

    /// 🔎 Clavier de la recherche : on tape, ↑ ↓ choisissent, Entrée ouvre, Échap ferme.
    pub(crate) fn key_palette(&mut self, event: winit::event::KeyEvent) {
        let Some(p) = self.ui.palette.as_mut() else { return };
        match &event.logical_key {
            Key::Named(NamedKey::Escape) => self.ui.palette = None,
            Key::Named(NamedKey::ArrowUp) => p.bouge(-1),
            Key::Named(NamedKey::ArrowDown) => p.bouge(1),
            Key::Named(NamedKey::Backspace) => p.efface(),
            Key::Named(NamedKey::Enter) => {
                if let Some(c) = p.choisie() {
                    return self.palette_cible(c);
                }
            }
            _ => {
                if let Some(t) = event.text.as_ref().filter(|_| !self.pointer.mods.control_key()) {
                    p.tape(t);
                }
            }
        }
        self.redraw();
    }

    /// 🔎 Clic : sur une ligne = l'ouvrir ; à côté de la boîte = fermer.
    pub(crate) fn palette_click(&mut self) {
        let size = self.window.as_deref().map(ui_size).unwrap_or_default();
        let (x, y) = self.pointer.win_mouse;
        let (w, h) = (size.width as f64, size.height as f64);
        let Some(p) = self.ui.palette.as_mut() else { return };
        if let Some(c) = p.clic(w, h, x, y) {
            return self.palette_cible(c);
        }
        if !p.boite(w, h).contains(x, y) {
            self.ui.palette = None;
        }
        self.redraw();
    }

    /// 🗺 Toucher dans le plan de l'app (position de la fenêtre) : ‹ Retour / ✕ le ferment ; un bloc, une tuile ou
    /// « Voir » ferme le plan et ouvre sa partie de l'app.
    pub(crate) fn plan_click(&mut self) {
        use argos_render::plan::PlanClick;
        let (x, y) = self.pointer.win_mouse;
        let (w, h) = (self.win_w(), self.top_h() + self.area.1 as f64 + argos_render::ecrans::dock_h());
        let Some(p) = self.ui.plan.as_ref() else { return };
        match p.click(x, y, w, h) {
            PlanClick::Rien => self.redraw(),
            PlanClick::Fermer => {
                self.ui.plan = None;
                self.redraw();
            }
            PlanClick::Ouvrir(l) => {
                self.ui.plan = None;
                self.plan_ouvrir(l);
            }
        }
    }

    /// 🗺 Ouvre une partie de l'app depuis le plan (comme son bouton à l'écran).
    pub(crate) fn plan_ouvrir(&mut self, l: argos_render::plan::Lieu) {
        use argos_render::ecrans::CellHit as C;
        use argos_render::phonebar::Icone;
        use argos_render::plan::Lieu as L;
        // le bouton de l'écran du cell (sa place sert d'ancre à ce qu'il ouvre)
        let bouton = |me: &mut Self, hit: C| {
            let z = argos_render::ecrans::zones(me.win_w(), me.cell_h(), &me.cell_etat());
            let r = z.iter().find(|(_, x)| *x == hit).map_or(Rect::new(0.0, 0.0, me.win_w(), 46.0), |(r, _)| *r);
            me.cell_click(hit, r);
        };
        match l {
            L::Chart => self.redraw(),
            // ✏️ la barre des outils ⋮⋮ s'ouvre (elle se replie après le choix)
            L::Outils => {
                if self.tools.retracted {
                    self.tools.retracted = false;
                    if self.top.phone && !self.tools.vertical {
                        self.tools.x = argos_render::toolbar::place_ouverte(&self.tools, &argos_render::text::Text::default());
                        self.tools.scroll = 0.0;
                    }
                    self.save_tools();
                }
                self.redraw();
            }
            L::Indicateurs => bouton(self, C::Fx),
            L::Alertes => bouton(self, C::Alertes),
            L::Trading => bouton(self, C::Icone(Icone::Trading)),
            L::Watchlist => bouton(self, C::Symbole),
            L::Recherche => bouton(self, C::Recherche),
            L::Reglages => bouton(self, C::Icone(Icone::Reglages)),
            L::Argos => bouton(self, C::Argos),
            L::Appli => {
                let r = Rect::new(self.win_w() - 60.0, 16.0, 30.0, 28.0);
                self.top_click_do(argos_render::topbar::TopClick::Menu(argos_render::topbar::TopMenu::App, r));
            }
            L::Splits => bouton(self, C::Disposition),
            L::Accueil => bouton(self, C::Accueil),
            L::Section(s) => self.palette_cible(Cible::Section(s)),
            L::Raccourcis => self.palette_cible(Cible::Raccourcis),
            L::Simulateur => self.palette_cible(Cible::Simulateur),
        }
    }

    /// 🔎 Ouvre ce qu'une ligne de la recherche désigne.
    pub(crate) fn palette_cible(&mut self, c: Cible) {
        self.ui.palette = None;
        match c {
            Cible::Symbole(coin) => {
                self.ui.accueil = None;
                return self.open_coin(coin);
            }
            Cible::Indicateur(i) => {
                self.ui.accueil = None;
                if !self.ind.on.contains(&i) {
                    self.toggle_indicator(i);
                }
                self.banner = Some(format!("📈 {} sur la chart", i.name()));
            }
            Cible::Lecon(id) => {
                self.ui.accueil = None;
                self.open_academy();
                let w = self.window.as_deref().map(ui_size).unwrap_or_default().width as f64;
                if let Some(p) = self.ui.ac_page.as_mut() {
                    if p.open_at(&id, 0, usize::MAX, &argos_render::text::Text::default(), w) == argos_render::academy::AcClick::Changed {
                        Self::write_prefs("academy.json", &p.progress);
                    }
                }
            }
            Cible::Section(s) => return self.accueil_section(s),
            Cible::Accueil => return self.open_accueil(),
            Cible::Raccourcis => return self.toggle_raccourcis(),
            Cible::Simulateur => return self.toggle_simulateur(),
            Cible::Actualiser => return self.reload(),
            Cible::Support => return self.support_ouvrir(),
            Cible::Outil(t) => {
                (self.watch.prefs.open, self.watch.prefs.panel) = (true, t);
                self.save_watch();
            }
            Cible::Reglage(r) => match r {
                Reglage::StyleVerre => {
                    self.top.prefs.glass = !self.top.prefs.glass;
                    self.ui.glass_off = false;
                    self.save_top();
                    self.banner = Some(format!("🫧 Style verre {}", if self.top.prefs.glass { "allumé" } else { "éteint" }));
                }
                Reglage::AccueilAuLancement => self.toggle_accueil_lancement(),
                Reglage::RevoirAccueil => return self.revoir_accueil(),
                Reglage::Theme => {
                    self.ui.accueil = None;
                    return self.sheet_act(argos_render::sheet::Act::Theme);
                }
            },
        }
        self.redraw();
    }

    /// 📱 Cell : la barre du haut est-elle cachée par sa poignée ? (la chart monte jusqu'en haut)
    pub(crate) fn haut_cache(&self) -> bool {
        // (📱 the single pill: no top bar any more, the flux dot and DÉMO sit beside the chart's title)
        // (⛶ full screen: hidden too)
        self.top.phone && (self.top.prefs.cell_haut_cache || self.pilule() || self.ui.plein_ecran.is_some())
    }

    /// 📱 Cell : les deux rangées du bas sont-elles cachées par leur poignée ? (la chart descend jusqu'en bas ; il
    /// reste la mini-pastille DÉMO et le bouton rond Argos AI)
    pub(crate) fn bas_cache(&self) -> bool {
        // (📱 the single pill: no icon bar any more, the pill floats at the bottom)
        self.top.phone && (self.top.prefs.cell_bas_cache || self.pilule() || self.ui.plein_ecran.is_some())
    }

    /// 📱 A side panel is open as a bottom sheet (Argos AI aside): the single pill hides under it.
    pub(crate) fn volet_feuille(&self) -> bool {
        self.watch.prefs.open && !self.ui.ai_open
    }

    /// 📱 Room kept at the bottom under the chart: the icon bar, or the single pill (none when folded, hidden, or
    /// under an open panel's sheet: the sheet then goes down to the bottom of the screen, no black band).
    pub(crate) fn bas_reserve(&self) -> f64 {
        if self.ui.zen {
            0.0
        } else if self.pilule() && !self.volet_feuille() {
            argos_render::ecrans::pilule_reserve(self.pilule_pliee())
        } else if self.bas_cache() {
            0.0
        } else {
            argos_render::ecrans::dock_h()
        }
    }

    /// 👋 interface-2 : les pages de bienvenue vues, dans son propre fichier (les anciennes versions synchronisées
    /// réécrivent topbar.json sans connaître la nouvelle interface : les pages revenaient à chaque lancement).
    pub(crate) fn bienvenue_lire() -> bool {
        Self::interface2().bienvenue_vue
    }

    pub(crate) fn bienvenue_ecrire(vue: bool) {
        Self::interface2_changer(|i| i.bienvenue_vue = vue);
    }

    /// ⌂ At launch: opens the Accueil the very first time, when « Accueil au lancement » is on, or when it was open in
    /// this same tab before a reload (F5). Never because of another tab or device.
    pub(crate) fn accueil_au_depart(&mut self) {
        let ici = platform::onglet::lire(ACCUEIL_ONGLET).as_deref() == Some("1");
        self.ui.accueil_note = ici;
        if self.top.prefs.accueil_au_lancement(ici) {
            self.open_accueil();
        }
    }

    /// ⌂ Notes in this tab's own storage whether the Accueil is open (only when it changes).
    pub(crate) fn accueil_noter(&mut self) {
        let ouvert = self.ui.accueil.is_some();
        if ouvert != self.ui.accueil_note {
            self.ui.accueil_note = ouvert;
            platform::onglet::ecrire(ACCUEIL_ONGLET, if ouvert { "1" } else { "0" });
        }
    }

    /// 📐 Réglages propres à interface-2.
    pub(crate) fn interface2() -> Interface2 {
        Self::read_prefs::<Interface2>("interface-2.json")
    }

    /// 📐 Change un réglage d'interface-2 (les autres restent).
    pub(crate) fn interface2_changer(f: impl FnOnce(&mut Interface2)) {
        let mut i = Self::interface2();
        f(&mut i);
        Self::write_prefs("interface-2.json", &i);
    }

    /// 📱 Hauteur de la fenêtre du cell (rangée du haut, chart, barre d'icônes).
    pub(crate) fn cell_h(&self) -> f64 {
        self.top_h() + self.area.1 as f64 + self.bas_reserve()
    }

    /// 📱 Poignée de la rangée du haut (fenêtre) : au bord du haut, au milieu (le vide de la pilule de la maquette).
    pub(crate) fn poignee_haut(&self) -> Rect {
        Rect::new(self.win_w() / 2.0 - 32.0, 0.0, 64.0, 14.0)
    }

    /// 📱 Poignée du bas (fenêtre) : juste au-dessus de la barre d'icônes, entre la pilule du symbole et ƒx (cachée :
    /// tout en bas de l'écran).
    pub(crate) fn poignee_bas(&self) -> Rect {
        let h = self.cell_h();
        let y = if self.bas_cache() {
            h - 14.0
        } else {
            h - argos_render::ecrans::dock_h() - 12.0
        };
        Rect::new(self.win_w() / 2.0 - 32.0, y, 64.0, 12.0)
    }

    /// 📱 Une page plein écran est ouverte (Accueil et bienvenue, Académie, Marketplace, PropFirm).
    pub(crate) fn page_ouverte(&self) -> bool {
        self.ui.accueil.is_some()
            || self.ui.ac_page.is_some()
            || self.ui.mk_page.is_some()
            || self.ui.pf_page.is_some()
            || self.ui.bt_page.is_some()
            || self.ui.fx_page.is_some()
            || self.ui.plan.is_some()
    }

    /// 📐 Ce que montre l'écran principal du cell (maquette mesurée), tiré du moteur.
    pub(crate) fn cell_etat(&self) -> argos_render::ecrans::Etat {
        let court = |c: String| c.rsplit(':').next().unwrap_or(&c).to_string();
        // le nom de la chart (« BTC-USDC » de « BTC-USDC Perpétuel · 4h · Hyperliquid »), comme la maquette
        let symbole = self
            .title
            .split([' ', '·'])
            .next()
            .filter(|t| !t.is_empty())
            .map_or_else(|| self.symbol_label(), str::to_string);
        let (sa, sp) = self.voisins_symbole();
        let (ia, ip) = self.voisins_intervalle();
        let iv = |s: Option<i64>| s.map(|s| Interval(s).short());
        let flux = match (self.chart.hors_ligne, self.chart.live) {
            (true, _) => ("Hors ligne".to_string(), true),
            (false, Some(true)) => ("Flux à l'heure".to_string(), false),
            (false, Some(false)) => ("Flux 1 en retard".to_string(), true),
            (false, None) => ("Flux en pause".to_string(), true),
        };
        argos_render::ecrans::Etat {
            mise_en_page: self.top.layout.clone(),
            sauvegarde: self.top.saving,
            alertes: self.top.alertes,
            compte: format!("Argos {}K", (self.demo.start.to_f64() / 1000.0).round()),
            flux,
            symbole,
            sym_voisins: (sa.map(court), sp.map(court)),
            unite: self.chart.interval.short(),
            unite_voisines: (iv(ia), iv(ip)),
            allume: match self.phone_allume() {
                Some(argos_render::phonebar::PhoneHit::Outil(i)) => Some(i),
                _ => None,
            },
            haut_cache: self.haut_cache(),
            bas_cache: self.bas_cache(),
            feuille: self.volet_feuille() || self.ui.pfeuille.is_some(),
            pilule: self.pilule(),
            repliee: self.pilule_pliee(),
            logo: self.watch.logos.get(&self.watch.selected).cloned(),
            coin: self.watch.selected.clone(),
            pastille: self.top.alertes,
            demo: true,
            titre: self.pilule_titre(),
        }
    }

    /// 📱 Toucher une poignée : sa barre se cache ou revient (gardé dans les réglages). `true` si c'était une poignée.
    pub(crate) fn poignee_toucher(&mut self) -> bool {
        // (📱 the single pill has its own fold bar instead)
        if !self.top.phone || self.pilule() || self.ui.accueil.is_some() || self.ui.plan.is_some() || self.ui.argos.is_some() {
            return false;
        }
        let (x, y) = self.pointer.win_mouse;
        // (📅 the month report covers them)
        if self.month_under(x, y) {
            return false;
        }
        if self.poignee_haut().contains(x, y) {
            self.top.prefs.cell_haut_cache = !self.top.prefs.cell_haut_cache;
        } else if self.poignee_bas().contains(x, y) {
            self.top.prefs.cell_bas_cache = !self.top.prefs.cell_bas_cache;
        } else {
            return false;
        }
        self.save_top();
        self.redraw();
        true
    }

    /// 🟠 Le texte de la mini pastille DÉMO (rangée du haut cachée).
    pub(crate) const MINI_DEMO_TXT: &'static str = MINI_DEMO;

    /// 📱 Mini-pastille DÉMO (fenêtre) : la rangée du haut cachée, en haut à gauche (la pastille de la maquette part
    /// avec la rangée).
    pub(crate) fn mini_demo_rect(&self) -> Rect {
        let text = argos_render::text::Text::default();
        let w = argos_render::phonebar::pastille_w(&text, MINI_DEMO);
        Rect::new(12.0, 16.0, w, 22.0)
    }

    /// ✨ Bouton Argos AI (fenêtre) : sa zone cliquable. 🖥 PC (choix A de Jee, 16 nov.) : toute la case du coin, sous
    /// l'échelle des prix et à droite de l'axe du temps (plus rien des barres ni des colonnes dessous) ; 📱 sur cell,
    /// à sa place de la maquette (collé au coin, au-dessus de la barre d'icônes) ; une feuille ouverte passe devant.
    /// ✨ The drawn Argos AI button: 🖥 the pill, the whole corner cell minus 2 points on each side; elsewhere, the
    /// button area.
    pub(crate) fn ai_rond(&self) -> Rect {
        let r = self.ai_bouton_rect();
        if self.top.phone {
            return r;
        }
        // (🖥 the Argos AI pill: the whole cell, 2-point margin on each side)
        Rect::new(r.x + 2.0, r.y + 2.0, (r.w - 4.0).max(0.0), (r.h - 4.0).max(0.0))
    }

    pub(crate) fn ai_bouton_rect(&self) -> Rect {
        // 📱 the single pill: Argos AI at its right end (folded: none, the ⌃ tab brings it back)
        if self.pilule() {
            if self.pilule_pliee() || self.ui.accueil.as_ref().is_some_and(|p| p.bienvenue.is_some()) {
                return Rect::new(0.0, 0.0, 0.0, 0.0);
            }
            return argos_render::ecrans::pilule_parts(self.win_w(), self.cell_h())[6].0;
        }
        if !self.top.phone {
            // (🧪 the chart stopped at the Simulator panel's edge: the pill keeps its place, at the bottom right, over the
            // panel's bottom band where nothing of its content goes — the mock-up's)
            let vp = &self.chart.viewport;
            let x = self.rw() + vp.plot_w() + self.dock.sous_sim;
            return Rect::new(x, self.top_h() + vp.plot_h(), vp.price_axis as f64, vp.time_axis as f64);
        }
        // une page plein écran (bienvenue, Accueil, Académie, Marketplace, PropFirm) : tout en bas, collé à droite
        // (les 3 pages de bienvenue couvrent tout, Argos AI compris, comme la maquette)
        if self.ui.accueil.as_ref().is_some_and(|p| p.bienvenue.is_some()) {
            return Rect::new(0.0, 0.0, 0.0, 0.0);
        }
        // A (Jee, 2026-11-09) : la même place partout — au-dessus de la barre d'icônes, à droite (pages comprises)
        if self.page_ouverte() {
            let h = self.top_h() + self.area.1 as f64 + argos_render::ecrans::dock_h();
            return argos_render::ecrans::ai_rect(self.win_w(), if self.bas_cache() { h } else { self.cell_h() }, false);
        }
        // choix B (Jee) : même avec la watchlist, une feuille, un menu ou le Simulateur ouverts (maquette « Symboles »)
        argos_render::ecrans::ai_rect(self.win_w(), self.cell_h(), self.bas_cache())
    }

    /// 📱 Le bouton de la barre du bas dont le volet est ouvert (surligné).
    pub(crate) fn phone_allume(&self) -> Option<argos_render::phonebar::PhoneHit> {
        use argos_render::phonebar::{Icone, PhoneHit};
        if self.ui.sim.is_some() {
            return Some(PhoneHit::Outil(Icone::Demo));
        }
        if !self.watch.prefs.open {
            return None;
        }
        Some(match self.watch.prefs.panel {
            wl::Tool::Watchlist => PhoneHit::Symbol,
            wl::Tool::Trading => PhoneHit::Outil(Icone::Trading),
            wl::Tool::Whales => PhoneHit::Outil(Icone::Baleines),
            wl::Tool::Agenda => PhoneHit::Outil(Icone::Macro),
            _ => return None,
        })
    }

    /// ⌂ Réglage « Accueil au lancement » : allumé / éteint.
    pub(crate) fn toggle_accueil_lancement(&mut self) {
        self.top.prefs.accueil = !self.top.prefs.accueil;
        self.save_top();
        self.banner = Some(format!("⌂ Accueil au lancement : {}", if self.top.prefs.accueil { "oui" } else { "non" }));
    }

    /// ⌂ « Revoir l'accueil » : les 3 écrans du premier lancement, de nouveau.
    pub(crate) fn revoir_accueil(&mut self) {
        self.top.prefs.bienvenue_vue = false;
        Self::bienvenue_ecrire(false);
        self.open_accueil();
    }

    /// ⌨️ Ouvre / ferme le volet des raccourcis.
    pub(crate) fn toggle_raccourcis(&mut self) {
        self.ui.raccourcis = match self.ui.raccourcis.take() {
            Some(_) => None,
            None => Some(argos_render::raccourcis::Raccourcis::default()),
        };
        self.redraw();
    }

    /// ⌨️ Le volet des raccourcis dans la fenêtre.
    pub(crate) fn raccourcis_rect(&self) -> Rect {
        // (🧪 without a window: the test's image size, the one the drawing uses)
        let size = self.window.as_deref().map(ui_size).or(self.ui.taille_test).unwrap_or_default();
        let (w, h) = if size.width > 0 {
            (size.width as f64, size.height as f64)
        } else {
            (self.win_w(), self.top_h() + self.area.1 as f64)
        };
        argos_render::raccourcis::Raccourcis::rect(w, h, self.top_h())
    }

    /// ‹ › Les languettes qui replient la barre de gauche (`true`) et la barre d'icônes de droite (`false`).
    pub(crate) fn fold_tabs(&self) -> Vec<(Rect, bool)> {
        if self.top.phone || self.ui.zen || self.page_ouverte() {
            return vec![];
        }
        let y = self.top_h() + self.area.1 as f64 / 2.0 - TAB.1 / 2.0;
        let right = self.side_x() - TAB.0;
        // 🖥 colonne d'icônes là : son › en bas la replie (pas de languette › de plus) ; repliée, la languette la rouvre
        let mut v = if self.colonne_pc().is_some() {
            vec![]
        } else {
            vec![(Rect::new(right, y, TAB.0, TAB.1), false)]
        };
        // 🖥 rail replié : sa bande de 12 le rouvre (pas de languette ‹ par-dessus)
        if !self.rail.folded && !self.ui.rail_garde {
            v.insert(0, (Rect::new(self.rw(), y, TAB.0, TAB.1), true));
        }
        v
    }

    /// 🖥 Bande du rail replié (maquette PC : 12 de large à 10 du bord, du haut de la chart à 40 du bas, en verre) ;
    /// un clic la déplie. `None` rail déplié, sur cell, en Zen ou sur une page.
    pub(crate) fn bande_rail(&self) -> Option<Rect> {
        let page = self.page_ouverte();
        (self.rail.folded && !self.top.phone && !self.ui.zen && !page).then(|| Rect::new(10.0, self.top_h() - 6.0, 12.0, (self.area.1 as f64 - 34.0).max(40.0)))
    }

    /// ‹ › Clic sur une languette : replie / déplie sa barre. `true` si le clic était pour elle.
    pub(crate) fn fold_click(&mut self, x: f64, y: f64) -> bool {
        let Some((_, left)) = self.fold_tabs().into_iter().find(|(r, _)| r.contains(x, y)) else {
            return false;
        };
        if left {
            self.rail.folded = !self.rail.folded;
            let deplie = !self.rail.folded;
            Self::interface2_changer(|i| i.rail_deplie = deplie);
        } else {
            // panneau ouvert : le geste le ferme et replie la barre ; sinon il replie / déplie la barre
            let w = &mut self.watch.prefs;
            w.folded = !w.folded || w.open;
            w.open = false;
            self.save_watch();
        }
        self.redraw();
        true
    }

    /// ‹ › Image d'une languette (chevron vers où la barre va).
    pub(crate) fn fold_tab_pm(left: bool, folded: bool, hover: bool) -> Option<tiny_skia::Pixmap> {
        use argos_render::shapes::{lines, round_fill, round_stroke};
        let mut pm = argos_render::px::new(TAB.0 as u32, TAB.1 as u32)?;
        let r = Rect::new(0.0, 0.0, TAB.0, TAB.1);
        round_fill(&mut pm, r, 5.0, argos_render::text::Rgb(20, 22, 30), if hover { 0.95 } else { 0.7 });
        round_stroke(
            &mut pm,
            Rect::new(0.5, 0.5, TAB.0 - 1.0, TAB.1 - 1.0),
            5.0,
            argos_render::text::Rgb(0x3a, 0x3d, 0x48),
            1.0,
        );
        // ‹ = vers la gauche ; la barre de gauche se replie vers la gauche, celle de droite vers la droite
        let vers_gauche = left != folded;
        let d = if vers_gauche { 2.5f32 } else { -2.5 };
        lines(
            &mut pm,
            TAB.0 / 2.0,
            TAB.1 / 2.0,
            &[&[(d, -5.0), (-d, 0.0), (d, 5.0)]],
            argos_render::palette::GREY,
            1.5,
        );
        Some(pm)
    }
}

/// 🟠 Mini pastille DÉMO de la maquette (barres du cell cachées).
const MINI_DEMO: &str = "DÉMO";

/// 🖥 Place et état de la barre de favoris d'interface-2.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub(crate) struct BarreI2 {
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) hidden: bool,
    pub(crate) retracted: bool,
    pub(crate) vertical: bool,
}

/// 🖥 Champs de toolbar.json et rail.json que l'ancienne /app/ lit et qu'interface-2 range dans interface-2.json :
/// /app2/ les recopie tels quels quand elle réécrit ces fichiers.
pub(crate) const BARRE_A_ELLE: [&str; 5] = ["x", "y", "hidden", "retracted", "vertical"];
pub(crate) const RAIL_A_ELLE: [&str; 1] = ["folded"];
pub(crate) const TOP_A_ELLE: [&str; 1] = ["pct"];

/// ⌂ Name of the tab state « the Accueil is open » (`platform::onglet`, never in the shared settings).
pub(crate) const ACCUEIL_ONGLET: &str = "accueil-ouvert";

/// 🖥 Barre de favoris de la maquette PC : 893 × 48 à 64 du bord gauche et 110 sous le haut de la chart.
pub(crate) const BARRE_MAQUETTE: (f64, f64) = (64.0, 110.0);

impl App {
    /// 🖥 Cadre PC d'interface-2 (au lancement et après une synchro) : rail replié en bande de 12 sauf si tu l'as
    /// déplié, barre de favoris à sa place à elle (la maquette la première fois, visible). Rien n'est écrit : rail.json
    /// et toolbar.json restent ceux de l'ancienne /app/.
    pub(crate) fn passer_au_cadre(&mut self) {
        self.appliquer_cadre(&Self::interface2());
    }

    /// 🖥 Le cadre d'interface-2 d'après ces réglages (sans rien lire ni écrire).
    pub(crate) fn appliquer_cadre(&mut self, i: &Interface2) {
        // 🖥 le rail est toujours une bande (maquette) : il s'ouvre au survol, par-dessus la chart ; ‹ › le garde ouvert
        self.rail.folded = !i.rail_deplie;
        self.ui.curseur = i.curseur;
        self.curseur_icone_rail();
        (self.ui.rail_garde, self.ui.rail_survol) = (i.rail_deplie, false);
        self.watch.col_repliee = i.colonne_repliee;
        argos_render::indicators::set_replies(&i.panneaux_replies);
        argos_render::indicators::set_ordre(&i.panneaux_ordre);
        let b = i.barre.unwrap_or(BarreI2 {
            x: BARRE_MAQUETTE.0,
            y: BARRE_MAQUETTE.1,
            hidden: false,
            retracted: false,
            vertical: false,
        });
        let t = &mut self.tools;
        (t.x, t.y, t.hidden, t.retracted, t.vertical) = (b.x, b.y, b.hidden, b.retracted, b.vertical);
        self.top.prefs.pct = i.tickers_pct.unwrap_or(true);
        self.top.sans_bandeaux = i.sans_bandeaux;
        self.ui.pilule_repliee = i.pilule_repliee;
        self.watch.col_outils = Some(i.colonne_outils.clone().unwrap_or_else(|| wl::Tool::MAQUETTE.to_vec()));
        self.ui.barre_par_defaut.set(i.barre.is_none());
        self.dock.hauteurs = i.modules_hauteurs.clone();
        self.dock.largeur = i.modules_largeur;
        self.ui.volet_large = i.volet_large;
        if let Some(o) = i.legende_ouverte {
            self.ind.leg.0 = o;
        }
    }

    /// 🖥 Garde les réglages de la barre du haut : le choix $ / % des tickers est à interface-2 (topbar.json garde celui de
    /// l'ancienne /app/).
    pub(crate) fn save_top(&self) {
        Self::write_prefs_garde("topbar.json", &self.top.prefs, &TOP_A_ELLE);
        let pct = self.top.prefs.pct;
        Self::interface2_changer(|i| i.tickers_pct = Some(pct));
    }

    /// 🖥 Garde les outils ⭐ de la colonne (et leur ordre) dans interface-2.json.
    pub(crate) fn colonne_garder(&self) {
        let v = self.watch.col_outils.clone();
        Self::interface2_changer(|i| i.colonne_outils = v);
    }

    /// 🖥 Garde la place et l'état de la barre de favoris d'interface-2.
    pub(crate) fn barre_garder(&self) {
        self.ui.barre_par_defaut.set(false);
        let t = &self.tools;
        let b = BarreI2 {
            x: t.x,
            y: t.y,
            hidden: t.hidden,
            retracted: t.retracted,
            vertical: t.vertical,
        };
        Self::interface2_changer(|i| i.barre = Some(b));
    }
}

/// 📐 Réglages propres à interface-2 (fichier « interface-2.json »).
/// Règle : tout réglage de mise en page du NOUVEL écran (rail replié, place des barres, colonne, cadre, pied) va ici ;
/// les anciens fichiers de mise en page (rail.json, toolbar.json, champs de mise en page de topbar.json /
/// watchlist.json) ne sont réécrits qu'avec leurs anciennes valeurs. Les données (listes, dessins, indicateurs,
/// compte démo, journal, thème) restent communes avec l'ancienne /app/.
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub(crate) struct Interface2 {
    #[serde(default)]
    pub(crate) bienvenue_vue: bool,
    /// ⛶ How many times the « Double tap pour revenir » bubble was shown (at most 3).
    #[serde(default)]
    pub(crate) plein_ecran_bulles: u8,
    /// 🖥 Rail de gauche déplié (cadre PC de la maquette : replié en une bande de 12 par défaut). Gardé ici, pas dans
    /// rail.json : l'ancienne /app/ (même stockage) garde son rail à elle.
    #[serde(default)]
    pub(crate) rail_deplie: bool,
    /// 🖥 Barre de favoris d'interface-2 : sa place et son état (gardés ici, pas dans toolbar.json que l'ancienne /app/
    /// lit). `None` : pas encore bougée, à sa place de la maquette PC.
    #[serde(default)]
    pub(crate) barre: Option<BarreI2>,
    /// 🖥 Tickers de la barre du haut en % (maquette) ou en prix ; `None` : en %, comme la maquette.
    #[serde(default)]
    pub(crate) tickers_pct: Option<bool>,
    /// 🖥 Les deux rangées du bas de la barre du haut repliées (▾ de la maquette).
    #[serde(default)]
    pub(crate) sans_bandeaux: bool,
    /// 📱 The phone's single pill folded away (the ⌃ tab brings it back).
    #[serde(default)]
    pub(crate) pilule_repliee: bool,
    /// 🖥 Outils ⭐ de la colonne d'icônes PC, dans l'ordre ; `None` : ceux de la maquette.
    #[serde(default)]
    pub(crate) colonne_outils: Option<Vec<wl::Tool>>,
    /// 🖥 Colonne d'icônes repliée en bande (› en bas).
    #[serde(default)]
    pub(crate) colonne_repliee: bool,
    /// 🖥 Panneaux d'indicateurs repliés (▾) et leur ordre choisi (↑ ↓), du bas vers le haut.
    #[serde(default)]
    pub(crate) panneaux_replies: Vec<argos_render::indicators::Ind>,
    #[serde(default)]
    pub(crate) panneaux_ordre: Vec<argos_render::indicators::Ind>,
    /// 🖥 Colonne de modules : hauteur réglée à la main de chaque module (poignée ; double-clic = automatique) et largeur
    /// réglée en glissant son bord (`None` : S / M / L / XL de dock.json, que l'ancienne /app/ garde).
    #[serde(default)]
    pub(crate) modules_hauteurs: Vec<(argos_render::dock::Module, f64)>,
    #[serde(default)]
    pub(crate) modules_largeur: Option<f64>,
    /// 🖥 ⤢ Volet de droite large.
    #[serde(default)]
    pub(crate) volet_large: bool,
    /// 💡 Support: the « Qu'est-ce qu'on construit ensuite ? » card hidden until this time (seconds; its ✕, 30 days).
    #[serde(default)]
    pub(crate) idee_cachee: Option<i64>,
    /// 🖱 Curseur choisi au 1er bouton de la barre de gauche (Croix par défaut, comme TradingView).
    #[serde(default)]
    pub(crate) curseur: crate::types::CurseurChoisi,
    /// 🏷️ Liste des indicateurs ouverte ou fermée (⌄) ; `None` : celle d'indicator_legend.json.
    #[serde(default)]
    pub(crate) legende_ouverte: Option<bool>,
}

impl App {
    /// 🖥 Pied flottant du cadre PC (maquette : pilule en verre de 1050 × 32, à 8 du bas de la chart, centrée) : sa
    /// place dans la chart, bulle comprise au-dessus. `None` sur cell ou avec la barre du bas d'avant.
    pub(crate) fn pied_rect(&self) -> Option<Rect> {
        let vp = &self.chart.viewport;
        if self.top.phone || vp.footer > 0 {
            return None;
        }
        // (jamais jusqu'au bouton Argos AI, en bas à droite de la chart : la même place libre des deux côtés)
        let w = bottombar::PIED_W.min(vp.plot_w() - 2.0 * (argos_render::aibtn::SIZE + 16.0)).max(200.0);
        let h = bottombar::PIED_H + bottombar::PIED_BULLE;
        Some(Rect::new(((vp.plot_w() - w) / 2.0).round(), vp.height as f64 - 8.0 - h, w, h))
    }

    /// 🖥 Le pied flottant, s'il se montre : souris dans la chart à moins de 70 de son bas, ou son menu (Fuseau, Aller
    /// à) ouvert.
    pub(crate) fn pied_visible(&self) -> Option<Rect> {
        let r = self.pied_rect()?;
        let vp = &self.chart.viewport;
        let ouvert = self.ui.goto.is_some() || matches!(self.ui.list.as_ref().map(|l| &l.0), Some(ListKind::Timezone));
        let pres = self
            .pointer
            .mouse
            .is_some_and(|(x, y)| x >= 0.0 && x <= vp.width as f64 && y >= vp.height as f64 - 70.0 && y <= vp.height as f64);
        (ouvert || pres).then_some(r)
    }

    /// 🖥 Bouton du pied flottant sous la souris.
    pub(crate) fn pied_hit(&self) -> Option<bottombar::Hit> {
        let r = self.pied_visible()?;
        let (x, y) = self.pointer.mouse?;
        bottombar::hit(&bottombar::pied_vp(r.w), Some((x - r.x, y - r.y)), None)
    }

    /// 🖥 Clic dans le pied flottant : les mêmes actions que la barre du bas. `true` si le clic était pour lui.
    pub(crate) fn pied_click(&mut self) -> bool {
        let (Some(r), Some((x, y))) = (self.pied_visible(), self.pointer.mouse) else {
            return false;
        };
        if !(x >= r.x && x <= r.x + r.w && y >= r.y + bottombar::PIED_BULLE && y <= r.y + r.h) {
            return false;
        }
        match self.pied_hit() {
            Some(Hit::Range(g)) => {
                let i = g.start_index(&self.chart.bars, self.tz_offset_s());
                self.run_action(ChartAction::ShowRange(i, usize::MAX));
            }
            Some(Hit::GoTo) => self.run_action(ChartAction::GoTo),
            Some(Hit::Demo) => self.demo_pastille_toucher(),
            Some(Hit::Sessions) => {
                if let Some(s) = bottombar::sessions_rect(&bottombar::pied_vp(r.w)) {
                    let s = Rect::new(s.x + r.x, s.y + r.y, s.w, s.h);
                    self.open_list(ListKind::Timezone, self.win(s, self.rw()));
                }
            }
            None => {}
        }
        true
    }
}

impl App {
    /// 🖥 Replie (`true`) ou rouvre les deux rangées du bas de la barre du haut (tickers, fiche du symbole) ; gardé dans
    /// interface-2.json.
    pub(crate) fn bandeaux(&mut self, replies: bool) {
        (self.top.sans_bandeaux, self.top.survol_bandeaux) = (replies, false);
        Self::interface2_changer(|i| i.sans_bandeaux = replies);
        self.redraw();
    }

    /// How far the price legend (title, O H L C) starts below the top of the chart: 🖥 under the band of the folded
    /// rows; 📱 just under the DÉMO pill (Jee, 17 Nov: it ran under it), when the top row shows; else 0.
    pub(crate) fn decalage_legende(&self) -> f64 {
        if self.bande_haut().is_some() {
            return 16.0;
        }
        // 🖥 folded rows open on hover: the ticker row and the info row lie over the chart; the legend (and what sits
        // at the top of the chart: ⌄ N, pills) goes down under them, then up again when they fold (Jee, 20 Nov:
        // « jamais réglé »)
        if !self.top.phone && !self.ui.zen && self.top.sans_bandeaux && self.top.survol_bandeaux {
            return (argos_render::topbar::HEIGHT - argos_render::topbar::SANS_BANDEAUX_H) as f64 + 4.0;
        }
        // 🖥🧘 Zen: the chart starts at the top of the window; while the top bar comes back over it, the legend goes
        // down under it (never hidden), then up again
        if let Some(r) = self.zen_haut_rect() {
            return r.h;
        }
        if !self.top.phone || self.haut_cache() || self.ui.zen {
            return 0.0;
        }
        // (the title is drawn 24 below the chart's top, its letters ≈ 13 high: its top 4 under the pill's bottom)
        argos_render::ecrans::cell()
            .place("demoband", self.win_w(), self.cell_h())
            .map_or(0.0, |r| (r.y + r.h + 4.0 - self.top_h() - (24.0 - 13.0)).max(0.0))
    }

    /// 🖥 Rangées du haut repliées (demande de Jee : comme le rail) : une fine bande en verre avec sa poignée, couchée
    /// sous la 1re rangée ; au survol les rangées s'ouvrent par-dessus la chart. `None` dépliées, ouvertes, sur cell.
    pub(crate) fn bande_haut(&self) -> Option<Rect> {
        // (jamais sur la colonne de modules : elle commence sous la barre du haut, la bande s'arrête à son bord)
        // (sur la chart seulement : ni sur le rail, ni sur l'échelle de prix, ni sur un volet ou la colonne de modules)
        let (x0, x1) = (self.rw() + 4.0, self.rw() + self.chart.viewport.plot_w() - 4.0);
        (!self.top.phone && !self.ui.zen && self.top.sans_bandeaux && !self.top.survol_bandeaux)
            .then(|| Rect::new(x0, self.top_h() + 2.0, (x1 - x0).max(40.0), 12.0))
    }

    /// 🖥 Où la souris garde les rangées repliées ouvertes : leur bande (un peu élargie), puis toute la barre du haut.
    pub(crate) fn haut_survol_zone(&self) -> Option<Rect> {
        if self.top.phone || self.ui.zen || !self.top.sans_bandeaux {
            return None;
        }
        Some(if self.top.survol_bandeaux {
            // (un volet ouvert passe devant : les rangées s'arrêtent à son bord)
            let w = if self.watch.prefs.open {
                self.side_x().min(self.top_w())
            } else {
                self.top_w()
            };
            let w = self.haut_sans_modules(0.0, w).1;
            Rect::new(0.0, 0.0, w, argos_render::topbar::HEIGHT as f64 + 6.0)
        } else {
            let b = self.bande_haut()?;
            Rect::new(b.x, b.y - 6.0, b.w, b.h + 12.0)
        })
    }

    /// 🖥 Bornes (x0, x1) d'une bande du haut posée sur la chart, ramenées hors de la colonne de modules (à gauche ou à
    /// droite) : les rangées et la bande s'arrêtent à son bord, comme pour un volet.
    pub(crate) fn haut_sans_modules(&self, x0: f64, x1: f64) -> (f64, f64) {
        match self.dock.rect {
            Some(r) if self.dock.right > 0.0 => (x0, x1.min(r.x)),
            Some(r) if self.dock.left > 0.0 => (x0.max(r.x + r.w), x1),
            _ => (x0, x1),
        }
    }

    /// 🖥 Un menu est ouvert : la barre qui l'a ouvert au survol reste ouverte tant qu'il est là.
    /// 🖥🧘 In Zen the top bar is hidden; it comes back over the chart while the mouse is at the top edge of the
    /// window, stays while the mouse is in it or one of its menus is open (Jee, 17 Nov: « Zen must hide the top bar
    /// too »). Its place when shown; `None` otherwise, out of Zen, on the phone.
    pub(crate) fn zen_haut_rect(&self) -> Option<Rect> {
        (!self.top.phone && self.ui.zen && self.ui.zen_haut).then(|| Rect::new(0.0, 0.0, self.top_w(), self.top.height() as f64))
    }

    /// 🖥🧘 Where the mouse brings the top bar back in Zen: the top edge (6 high), then the whole bar once shown.
    pub(crate) fn zen_haut_zone(&self) -> Option<Rect> {
        if self.top.phone || !self.ui.zen {
            return None;
        }
        Some(match self.zen_haut_rect() {
            Some(r) => Rect::new(r.x, r.y, r.w, r.h + 6.0),
            None => Rect::new(0.0, 0.0, self.top_w(), 6.0),
        })
    }

    pub(crate) fn menu_ouvert(&self) -> bool {
        self.ui.list.is_some() || self.ui.pop.is_some()
    }
}

/// 🖥 Bord gauche du rail ouvert au survol (par-dessus la chart, comme la maquette).
pub(crate) const RAIL_X: f64 = 6.0;

impl App {
    /// 🖥 Où la souris garde le rail de gauche ouvert : sa bande (un peu élargie), puis le rail entier une fois ouvert.
    /// `None` rail déplié (ancienne mise en page), sur cell, en Zen.
    pub(crate) fn rail_survol_zone(&self) -> Option<Rect> {
        let b = self.bande_rail()?;
        Some(if self.ui.rail_survol {
            // (le rail et son bouton ‹ › à côté)
            Rect::new(
                RAIL_X,
                self.rail_y0(),
                rail::RAIL_W as f64 + 28.0,
                self.area.1 as f64 - (self.rail_y0() - self.top_h()),
            )
        } else {
            Rect::new(b.x - 6.0, b.y, b.w + 12.0, b.h)
        })
    }

    /// 🖥 Où la souris garde la colonne repliée ouverte : sa bande (un peu élargie), puis la colonne entière.
    pub(crate) fn col_survol_zone(&self) -> Option<Rect> {
        if !self.watch.col_repliee || self.watch.prefs.open || self.watch.colonne.is_none() {
            return None;
        }
        let (dx, dy) = (self.side_x(), self.top_h());
        let r = if self.watch.col_survol {
            self.watch.col_rect(self.watch.col_h)?
        } else {
            self.watch.col_bande()?
        };
        let r = if self.watch.col_survol {
            r
        } else {
            Rect::new(r.x - 6.0, r.y, r.w + 12.0, r.h)
        };
        Some(Rect::new(r.x + dx, r.y + dy, r.w, r.h))
    }
}

impl App {
    /// 🖥 Bouton ‹ › à côté du rail ouvert (comme ‹ de la colonne et ▴ des rangées du haut) : › « Garder la barre
    /// ouverte », ‹ « Replier la barre (elle revient au survol) ».
    pub(crate) fn rail_garde_rect(&self) -> Option<Rect> {
        // (en bas, à côté du rail : jamais sur la légende ⌄ N ni sur les rangées du haut)
        let y = self.top_h() + self.area.1 as f64 - 70.0;
        if self.ui.rail_garde && !self.rail.folded && !self.top.phone {
            return Some(Rect::new(self.rw() + 4.0, y, 24.0, 24.0));
        }
        (self.ui.rail_survol && self.bande_rail().is_some()).then(|| Rect::new(RAIL_X + rail::RAIL_W as f64 + 2.0, y, 24.0, 24.0))
    }

    /// 🖥 Haut du rail ouvert au survol : sous la barre du haut (et sous la bande des rangées repliées).
    pub(crate) fn rail_y0(&self) -> f64 {
        self.top_h() + if self.bande_haut().is_some() { 16.0 } else { 0.0 }
    }

    /// 🖥 Clic sur ‹ › du rail : gardé ouvert / replié au survol (interface-2.json).
    pub(crate) fn rail_garde_click(&mut self, x: f64, y: f64) -> bool {
        if !self.rail_garde_rect().is_some_and(|r| r.contains(x, y)) {
            return false;
        }
        let g = !self.ui.rail_garde;
        // gardé ouvert : il prend sa place (la chart et tout ce qui est à gauche se décalent) ; replié : la bande
        (self.ui.rail_garde, self.rail.folded, self.ui.rail_survol) = (g, !g, false);
        Self::interface2_changer(|i| i.rail_deplie = g);
        self.banner = Some(
            if g {
                "› Barre d'outils gardée ouverte"
            } else {
                "‹ Barre d'outils repliée : elle revient au survol"
            }
            .into(),
        );
        self.redraw();
        true
    }
}

impl App {
    /// 🖥 Bords des rangées 2 et 3 ouvertes au survol (demande de Jee : « à la largeur quand les menus s'ouvrent ») :
    /// à droite du rail (sa bande ou le rail gardé ouvert), jusqu'au volet ouvert, sinon jusqu'à la colonne d'icônes.
    pub(crate) fn rangs_haut(&self) -> Option<(f64, f64)> {
        let d = if self.watch.prefs.open { self.side_x() - 4.0 } else { self.top_w() - 8.0 };
        let d = self.haut_sans_modules(0.0, d + 4.0).1 - 4.0;
        // (jamais sur l'échelle de prix : les prix restent lisibles pendant le survol)
        let d = d.min(self.rw() + self.chart.viewport.plot_w() - 4.0);
        (!self.top.phone).then(|| (self.rw() + 4.0, d))
    }
}
