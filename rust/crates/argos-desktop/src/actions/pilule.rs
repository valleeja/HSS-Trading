//! 📱 The single pill of the phone (Jee's mock-up, 17 Nov 2026, maquettes/2026-11-17-pilule-unique-cell-texte-v1.md):
//! its sheets and what each tile opens. Every tile opens what already exists (the same function the old icon bar,
//! top pill or ARGOS menu called), so nothing new is reachable only from here and nothing old is lost.

use crate::*;
use argos_render::ecrans::feuille::{Carte, Feuille, Section, Toucher, Tuile};

/// Which sheet of the pill is open.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum PKind {
    /// ⋯ « Argos ».
    Argos,
    /// ✎ « Dessins ».
    Dessins,
    /// 📈 « Trading · DÉMO ».
    Trading,
    /// A tile « X et Y »: the small sheet X | Y (its id in [`DUOS`]).
    Duo(&'static str),
    /// 👆 « ⋯ » of the finger's crosshair: the chart's menu at its price.
    Croix(f64),
    /// ⚙ « Réglages »: the rows of the Réglages window, as tiles (Jee, 21 Nov, list 2: one way in, not « Panneaux »).
    Reglages,
}

/// A sheet of the pill: which one, its scroll and drag, the drawings search (`Some`: typed text) and its tab.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PFeuille {
    pub(crate) kind: PKind,
    pub(crate) defil: f64,
    pub(crate) tire: f64,
    pub(crate) recherche: Option<String>,
    pub(crate) saisie: bool,
    pub(crate) onglet: usize,
    /// Where the finger was put down (the tile acts when it lifts there).
    pub(crate) appui: Option<(f64, f64)>,
}

impl PFeuille {
    pub(crate) fn new(kind: PKind) -> Self {
        Self {
            kind,
            defil: 0.0,
            tire: 0.0,
            recherche: None,
            saisie: false,
            onglet: 0,
            appui: None,
        }
    }
}

/// The tiles « X et Y » of the « Argos » sheet: (id, title, the two tiles (id, icon, name)).
pub(crate) const DUOS: [(&str, &str, [(&str, &str, &str); 2]); 7] = [
    (
        "replay",
        "Replay et Backtest",
        [("replay1", "rewind", "Replay"), ("backtest", "flask", "Backtest")],
    ),
    ("agenda", "Agenda et News", [("agenda1", "calendar", "Agenda"), ("news", "newspaper", "News")]),
    (
        "baleines",
        "Baleines et Liquidations",
        [("baleines1", "fish", "Baleines"), ("liquidations", "fire", "Liquidations")],
    ),
    (
        "modules",
        "Modules et Marketplace",
        [("modules1", "puzzle-piece", "Modules"), ("marketplace", "shopping-cart", "Marketplace")],
    ),
    (
        "communaute",
        "Communauté et Académie",
        [("communaute1", "users-three", "Communauté"), ("academie", "graduation-cap", "Académie")],
    ),
    (
        "marches",
        "Marchés et Macro",
        [("marches1", "globe", "Marchés"), ("macro", "chart-line", "Macro")],
    ),
    // 🧭 📶 the Hyperliquid modules of « Marchés » (on the PC: the Marchés column and the ARGOS menu)
    (
        "hyperliquid",
        "Explorer et Stats Hyperliquid",
        [("explorer", "compass", "Explorer"), ("stats", "chart-bar", "Stats")],
    ),
];

/// The second pair (Portefeuille et Journal) lives apart: its two tiles open a page and a panel.
const DUO_PF: (&str, &str, [(&str, &str, &str); 2]) = (
    "portefeuille",
    "Portefeuille et Journal",
    [("portefeuille1", "briefcase", "Portefeuille"), ("journal", "notebook", "Journal")],
);

fn duo(id: &str) -> Option<(&'static str, &'static str, [(&'static str, &'static str, &'static str); 2])> {
    DUOS.iter().chain([&DUO_PF]).find(|d| d.0 == id).copied()
}

/// Short name of a drawing tools category (its tab).
fn nom_categorie(g: &argos_render::tools::RailGroup) -> String {
    g.label.split([' ', ',']).next().unwrap_or(g.label).to_string()
}

/// The categories of drawing tools, in the left bar's order, then « Outils » (measure, eraser, cursor): (group id,
/// tab name, tools).
pub(crate) fn categories() -> Vec<(&'static str, String, Vec<&'static str>)> {
    use argos_render::tools::{RailItem, RAIL};
    let mut out: Vec<(&'static str, String, Vec<&'static str>)> = RAIL
        .iter()
        .filter_map(|i| match i {
            RailItem::Group(g) => Some((g.id, nom_categorie(g), g.sections.iter().flat_map(|(_, t)| t.iter().copied()).collect())),
            RailItem::Tool(_) => None,
        })
        .collect();
    out.push((argos_render::toolbar::MISC, "Outils".into(), argos_render::toolbar::MISC_TOOLS.to_vec()));
    out
}

impl App {
    /// 📱 The phone shows the single pill (a phone-width screen; the tablet keeps its bars).
    pub(crate) fn pilule(&self) -> bool {
        self.top.phone && self.watch.phone.is_some_and(|w| (w as f64) < argos_render::formfactor::PHONE_MAX)
    }

    /// 📱 The pill folded: by its fold, or ⛶ full screen (the same « barres cachées » state).
    pub(crate) fn pilule_pliee(&self) -> bool {
        self.ui.pilule_repliee || self.ui.plein_ecran.is_some()
    }

    /// 📱 The chart's title on the phone with the pill: the name, cut to leave room for the « ● DÉMO » chip.
    pub(crate) fn pilule_titre_texte(&self) -> String {
        let t = self.title.split(" · ").next().unwrap_or(&self.title);
        argos_render::text::Text::default().fit(t, 16.0, (self.chart.viewport.plot_w() - 12.0 - 112.0).max(60.0))
    }

    /// 📱 Where the chart's title ends and its middle height (window): the « ● DÉMO » chip goes right after it.
    pub(crate) fn pilule_titre(&self) -> (f64, f64) {
        let w = argos_render::text::Text::default().width(&self.pilule_titre_texte(), 16.0) as f64;
        (self.rw() + 12.0 + w, self.top_h() + 24.0 + argos_render::haut_legende() - 6.0)
    }

    /// 📱 Fold or unfold the pill (kept in interface-2.json); the chart takes the room.
    pub(crate) fn pilule_replier(&mut self, repliee: bool) {
        self.ui.pilule_repliee = repliee;
        Self::interface2_changer(|i| i.pilule_repliee = repliee);
        self.redraw();
    }

    /// 📱 Open a sheet of the pill (the same one again: closed).
    pub(crate) fn pfeuille_ouvrir(&mut self, kind: PKind) {
        if self.ui.pfeuille.as_ref().is_some_and(|f| f.kind == kind) {
            self.ui.pfeuille = None;
        } else {
            (self.ui.list, self.ui.pop, self.ui.sheet, self.ui.argos) = (None, None, None, None);
            self.ui.pfeuille = Some(PFeuille::new(kind));
        }
        self.redraw();
    }

    /// The window (`w` × `h`) the sheets of the pill take their place in.
    pub(crate) fn pfeuille_fenetre(&self) -> (f64, f64) {
        (self.win_w(), self.cell_h())
    }

    /// 📱 The sheet of the pill that is open, as it is drawn and touched.
    pub(crate) fn pfeuille_vue(&self) -> Option<Feuille> {
        let p = self.ui.pfeuille.as_ref()?;
        let mut f = match &p.kind {
            PKind::Argos => self.feuille_argos(),
            PKind::Trading => Feuille {
                titre: "Trading · DÉMO".into(),
                sections: vec![Section {
                    titre: String::new(),
                    cols: 2,
                    tuiles: vec![
                        Tuile::new("trading", "chart-line-up", "Trading"),
                        Tuile::new("simulateur", "game-controller", "Simulateur"),
                    ],
                }],
                ..Default::default()
            },
            PKind::Duo(id) => {
                let (_, titre, t) = duo(id)?;
                Feuille {
                    titre: titre.replace(" et ", " | "),
                    sections: vec![Section {
                        titre: String::new(),
                        cols: 2,
                        tuiles: t.iter().map(|(i, ic, n)| Tuile::new(i, ic, n)).collect(),
                    }],
                    ..Default::default()
                }
            }
            PKind::Dessins => self.feuille_dessins(p),
            PKind::Croix(prix) => self.feuille_croix(*prix),
            PKind::Reglages => self.feuille_reglages(),
        };
        (f.defil, f.tire) = (p.defil, p.tire);
        Some(f)
    }

    fn feuille_argos(&self) -> Feuille {
        let t = |id: &str, ic: &'static str, n: &str| Tuile::new(id, ic, n);
        let mut notifs = t("notifs", "bell", "Notifications et alertes");
        notifs.pastille = self.top.alertes;
        let solde = argos_render::viewport::format_price(self.demo.balance.to_f64(), 2);
        let mut outils = vec![
            notifs,
            t("replay", "rewind", "Replay et Backtest"),
            t("type", "chart-bar", "Type de graphique"),
            t("agenda", "calendar", "Agenda et News"),
            t("baleines", "fish", "Baleines et Liquidations"),
            t("carnet", "stack", "Carnet d'ordres"),
            t("zen", "eye-slash", "Mode Zen"),
        ];
        // 🎯 a strategy on the chart: its result, first of the tools
        if !self.watch.strategie.noms.is_empty() {
            outils.insert(0, t("strategie", "cible", "Stratégie"));
        }
        Feuille {
            titre: "Argos".into(),
            partager: true,
            carte: Some(Carte {
                titre: format!("DÉMO · Argos {}K", (self.demo.start.to_f64() / 1000.0).round()),
                texte: format!("Aucun ordre réel · solde {solde} $ · P&L, PropFirm, Wallet"),
            }),
            sections: vec![
                Section {
                    titre: String::new(),
                    cols: 3,
                    tuiles: vec![
                        t("accueil", "compass", "Accueil"),
                        t("recherche", "magnifying-glass", "Recherche"),
                        t("plan", "map-pin", "Plan"),
                        t("layouts", "layout", &format!("{} ✓", self.top.layout)),
                        t("reglages", "gear", "Réglages"),
                    ],
                },
                Section {
                    titre: "OUTILS".into(),
                    cols: 2,
                    tuiles: outils,
                },
                Section {
                    titre: "ESPACES".into(),
                    cols: 3,
                    tuiles: vec![
                        t("propfirm", "shield-check", "PropFirm"),
                        t("script", "lightning", "Argos Script"),
                        t("modules", "puzzle-piece", "Modules et Marketplace"),
                        t("communaute", "users-three", "Communauté et Académie"),
                        t("marches", "globe", "Marchés et Macro"),
                        t("hyperliquid", "compass", "Explorer et Stats Hyperliquid"),
                        t("portefeuille", "briefcase", "Portefeuille et Journal"),
                        t("exchanges", "plugs-connected", "Exchanges"),
                        t("raccourcis", "key", "Raccourcis"),
                    ]
                    .into_iter()
                    // 📱 phone view: no « Raccourcis » (the keyboard shortcuts' list: there is no keyboard)
                    .filter(|t| argos_render::actions::montrer_raccourcis() || t.id != "raccourcis")
                    .collect(),
                },
            ],
            ..Default::default()
        }
    }

    /// ⚙ The « Réglages » sheet: the rows of the Réglages window that work (no « bientôt »), one tile each.
    fn feuille_reglages(&self) -> Feuille {
        Feuille {
            titre: "Réglages".into(),
            sections: vec![Section {
                titre: String::new(),
                cols: 2,
                tuiles: argos_render::reglages::lignes()
                    .into_iter()
                    .filter(|l| !matches!(l.3, argos_render::reglages::Cible::Bientot(_)))
                    .map(|(ic, nom, _, _)| Tuile::new(nom, ic, nom))
                    .collect(),
            }],
            ..Default::default()
        }
    }

    fn feuille_dessins(&self, p: &PFeuille) -> Feuille {
        let cats = categories();
        let tuile = |id: &'static str| {
            let d = argos_render::tools::def(id);
            let mut t = Tuile::new(id, id, d.map_or(id, |d| d.label));
            t.etoile = Some(self.rail.favs.iter().any(|f| f == id));
            t.on = self.tool == id;
            t
        };
        let mut onglets = vec![format!("★ Favoris {}", self.rail.favs.len())];
        onglets.extend(cats.iter().map(|(_, n, t)| format!("{n} {}", t.len())));
        let cherche = p.recherche.as_deref().unwrap_or("").trim().to_lowercase();
        let ids: Vec<&'static str> = if !cherche.is_empty() {
            let plain = |s: &str| argos_render::i18n::plain(s).to_lowercase();
            argos_render::tools::TOOLS
                .iter()
                .filter(|d| plain(d.label).contains(&plain(&cherche)) || d.id.contains(&cherche))
                .map(|d| d.id)
                .collect()
        } else if p.onglet == 0 {
            argos_render::tools::TOOLS
                .iter()
                .filter(|d| self.rail.favs.iter().any(|f| f == d.id))
                .map(|d| d.id)
                .collect()
        } else {
            cats.get(p.onglet - 1).map(|c| c.2.clone()).unwrap_or_default()
        };
        Feuille {
            titre: "Dessins".into(),
            champ: Some(p.recherche.clone().unwrap_or_default()),
            saisie: p.saisie,
            onglets: if cherche.is_empty() { onglets } else { vec![] },
            onglet: p.onglet,
            sections: vec![Section {
                titre: String::new(),
                cols: 3,
                tuiles: ids.into_iter().map(tuile).collect(),
            }],
            vide: Some(
                if cherche.is_empty() {
                    "Pas encore de favoris : touche ☆ sur un outil"
                } else {
                    "Aucun outil de ce nom"
                }
                .into(),
            ),
            ..Default::default()
        }
    }

    /// 📱 A touch on the open sheet of the pill (window coordinates).
    pub(crate) fn pfeuille_toucher(&mut self, x: f64, y: f64) {
        let Some(f) = self.pfeuille_vue() else { return };
        let (w, h) = self.pfeuille_fenetre();
        let text = argos_render::text::Text::default();
        let r = f
            .zones(&text, w, h)
            .into_iter()
            .find(|(r, _)| r.contains(x, y))
            .map_or(Rect::new(x, y, 1.0, 1.0), |(r, _)| r);
        let t = f.toucher(&text, w, h, x, y);
        let kind = self.ui.pfeuille.as_ref().map(|p| p.kind.clone());
        match t {
            Toucher::Fermer => self.ui.pfeuille = None,
            Toucher::Rien => {}
            Toucher::Champ => {
                if let Some(p) = self.ui.pfeuille.as_mut() {
                    p.saisie = true;
                    p.recherche.get_or_insert_with(String::new);
                }
            }
            Toucher::Onglet(k) => {
                if let Some(p) = self.ui.pfeuille.as_mut() {
                    (p.onglet, p.defil) = (k, 0.0);
                }
            }
            Toucher::Etoile(id) => {
                if let Some(d) = argos_render::tools::def(&id) {
                    self.rail.toggle_fav(d.id);
                    self.save_rail();
                }
            }
            Toucher::Partager => {
                self.ui.pfeuille = None;
                return self.cell_click(argos_render::ecrans::CellHit::Partager, r);
            }
            Toucher::Carte => {
                self.ui.pfeuille = None;
                return self.cell_click(argos_render::ecrans::CellHit::Argos, r);
            }
            Toucher::Pied => {
                self.ui.pfeuille = None;
                let (w, h) = self.pfeuille_fenetre();
                self.ui.argos = Some(argos_render::topmenu::ArgosMenu::feuille_bientot(w, h));
            }
            Toucher::Tuile(id) => {
                if kind == Some(PKind::Dessins) {
                    return self.pfeuille_outil(&id);
                }
                if let Some(PKind::Croix(prix)) = kind {
                    return self.croix_ligne(prix, &id);
                }
                if kind == Some(PKind::Reglages) {
                    self.ui.pfeuille = None;
                    if let Some(l) = argos_render::reglages::lignes().into_iter().find(|l| l.1 == id) {
                        return self.reglages_cible(l.3, r);
                    }
                    return self.redraw();
                }
                self.ui.pfeuille = None;
                return self.tuile_ouvrir(&id, r);
            }
        }
        self.redraw();
    }

    /// ✎ A drawing tool chosen in the sheet: the sheet closes, the tool is on (its category remembers it, like the
    /// left bar), a banner says where to touch.
    fn pfeuille_outil(&mut self, id: &str) {
        self.ui.pfeuille = None;
        let Some(d) = argos_render::tools::def(id) else { return self.redraw() };
        if let Some((g, _, _)) = categories()
            .into_iter()
            .find(|(g, _, t)| *g != argos_render::toolbar::MISC && t.contains(&d.id))
        {
            self.rail.set_last(g, d.id);
            self.save_rail();
        }
        self.set_tool(d.id);
        if d.id != "cursor" {
            self.banner = Some(format!("{} · touche le graphique", d.label));
        }
        self.redraw();
    }

    /// 📱 What a tile of the pill's sheets opens: always what already exists (the old icon bar, top pill, ARGOS menu).
    pub(crate) fn tuile_ouvrir(&mut self, id: &str, r: Rect) {
        use argos_render::ecrans::CellHit as C;
        use argos_render::phonebar::{Icone, PhoneHit};
        use argos_render::sheet::Act;
        let volet = |me: &mut Self, t: wl::Tool| {
            (me.watch.prefs.open, me.watch.prefs.panel) = (true, t);
            me.save_watch();
        };
        if duo(id).is_some() {
            return self.pfeuille_ouvrir(PKind::Duo(duo(id).map_or("", |d| d.0)));
        }
        match id {
            "accueil" => return self.cell_click(C::Accueil, r),
            "recherche" => return self.cell_click(C::Recherche, r),
            "plan" => return self.cell_click(C::Plan, r),
            "layouts" => return self.cell_click(C::MiseEnPage, r),
            "reglages" => return self.phone_click(PhoneHit::Outil(Icone::Reglages), r),
            "notifs" => return self.cell_click(C::Alertes, r),
            "type" => return self.sheet_act(Act::ChartType),
            "carnet" => volet(self, wl::Tool::Carnet),
            "strategie" => volet(self, wl::Tool::Strategie),
            "zen" => return self.toggle_zen(),
            "propfirm" => return self.ouvrir_propfirm(),
            "script" => return self.phone_click(PhoneHit::Outil(Icone::Script), r),
            "exchanges" => volet(self, wl::Tool::Exchanges),
            "raccourcis" => return self.toggle_raccourcis(),
            // the small sheets X | Y
            "replay1" => return self.replay_start(),
            "backtest" => return self.sheet_act(Act::Backtest),
            "agenda1" | "macro" => return self.phone_click(PhoneHit::Outil(Icone::Macro), r),
            "news" => {
                volet(self, wl::Tool::News);
                self.refresh_eco();
            }
            "baleines1" => return self.phone_click(PhoneHit::Outil(Icone::Baleines), r),
            "liquidations" => return self.toggle_indicator(argos_render::indicators::Ind::LiqMap),
            "modules1" => return self.phone_click(PhoneHit::Outil(Icone::Modules), r),
            "marketplace" => return self.open_market(),
            "communaute1" => return self.phone_click(PhoneHit::Outil(Icone::Communaute), r),
            "academie" => return self.open_academy(),
            "marches1" => volet(self, wl::Tool::Market),
            "explorer" => volet(self, wl::Tool::Explorer),
            "stats" => volet(self, wl::Tool::Stats),
            "portefeuille1" => self.ui.pf_page = Some(argos_render::portfolio::PfPage::new()),
            "journal" => volet(self, wl::Tool::Journal),
            // 📈 Trading · DÉMO
            "trading" => return self.phone_click(PhoneHit::Outil(Icone::Trading), r),
            "simulateur" => return self.toggle_simulateur(),
            _ => {}
        }
        self.redraw();
    }

    /// 📱 The finger (or the wheel) scrolls the open sheet; pulled down at its top, it follows the finger and closes
    /// past a third of its height (`dy` < 0: the finger goes down).
    pub(crate) fn pfeuille_defiler(&mut self, dy: f64) {
        let Some(f) = self.pfeuille_vue() else { return };
        let (w, h) = self.pfeuille_fenetre();
        let text = argos_render::text::Text::default();
        let rh = f.rect(&text, w, h).h;
        let max = f.defil_max(&text, w, h);
        let Some(p) = self.ui.pfeuille.as_mut() else { return };
        if dy < 0.0 && p.defil <= 0.0 {
            p.tire += -dy;
            if p.tire > rh / 3.0 {
                self.ui.pfeuille = None;
            }
        } else if p.tire > 0.0 {
            p.tire = (p.tire - dy).max(0.0);
        } else {
            p.defil = (p.defil + dy).clamp(0.0, max);
        }
        self.redraw();
    }

    /// 📱 The finger lifted: a sheet only pulled a little comes back up.
    pub(crate) fn pfeuille_lacher(&mut self) {
        if let Some(p) = self.ui.pfeuille.as_mut().filter(|p| p.tire > 0.0) {
            p.tire = 0.0;
            self.redraw();
        }
    }

    /// ✎ The drawings search: a typed letter (`None`: erase one).
    pub(crate) fn pfeuille_touche(&mut self, c: Option<char>) {
        if let Some(s) = self.ui.pfeuille.as_mut().filter(|p| p.saisie).and_then(|p| p.recherche.as_mut()) {
            match c {
                Some(c) => s.push(c),
                None => _ = s.pop(),
            }
            if let Some(p) = self.ui.pfeuille.as_mut() {
                p.defil = 0.0;
            }
        }
        self.redraw();
    }

    /// 👆 Lays (or moves) the finger's crosshair where the finger is, with its price.
    pub(crate) fn croix_poser(&mut self) {
        if let (Some((x, y)), Some(p)) = (self.pointer.mouse, self.price_at_mouse()) {
            self.ui.croix = Some((x, y, p));
            self.ui.plein_cible = Some(self.plein_cible_en(y));
            (self.ui.list, self.ui.context) = (None, argos_render::context_menu::ContextMenu::default());
        }
        self.redraw();
    }

    /// 👆 The round « ⋯ » beside the crosshair's price label (window): stuck to the price scale, it follows the
    /// crosshair; 40 points.
    pub(crate) fn croix_bouton(&self) -> Option<Rect> {
        let (_, y, _) = self.ui.croix?;
        let vp = &self.chart.viewport;
        // (the same box as the Argos Murs tooltip avoids: argos_render::murs::bouton_croix)
        let b = argos_render::murs::bouton_croix(vp.plot_w(), vp.plot_h(), y);
        Some(Rect::new(self.rw() + b.x, self.top_h() + b.y, b.w, b.h))
    }

    /// 👆 Where a touch takes « ⋯ »: its drawing grown to 40 screen points at the compact factor (TOUCHER) around its
    /// middle, the drawing unchanged.
    pub(crate) fn croix_zone(&self) -> Option<Rect> {
        let r = self.croix_bouton()?;
        let t = argos_render::ecrans::feuille::TOUCHER;
        Some(Rect::new(r.cx() - t / 2.0, r.cy() - t / 2.0, t, t))
    }

    /// 👆 « ⋯ » touched: the sheet « À 64 850 », the chart's menu at the crosshair's price.
    pub(crate) fn croix_menu(&mut self) {
        if let Some((_, _, p)) = self.ui.croix {
            self.pfeuille_ouvrir(PKind::Croix(p));
        }
    }

    /// The chart's menu at `prix`, as the right-click opens it (the same lines, the same order).
    fn croix_entrees(&self, prix: f64) -> Vec<argos_render::context_menu::Entry> {
        use argos_render::context_menu::{entries, Anchor, OpenMenu, Source};
        let ctx = self.ctx();
        let om = OpenMenu {
            source: Source::Chart,
            anchor: Anchor::Cursor(0.0, 0.0),
            price: prix,
            decimals: argos_render::viewport::data_decimals(&self.chart.bars),
            hover: None,
            sub: None,
            sub_scroll: 0,
        };
        entries(Source::Chart, &ctx, &om)
    }

    /// The lines of the chart's menu flat, with their tile id (« k » or « k.j » for a sub-menu's line) and section
    /// (a separator starts a new one, a sub-menu is a section of its own with its name).
    fn croix_lignes(&self, prix: f64) -> Vec<(String, String, argos_render::context_menu::Entry)> {
        use argos_render::context_menu::Entry;
        let mut out = vec![];
        let mut section = String::new();
        for (k, e) in self.croix_entrees(prix).into_iter().enumerate() {
            match e {
                Entry::Sep => section = format!("{k}"),
                Entry::Sub { label, entries } => {
                    let titre = label.to_uppercase();
                    for (j, s) in entries.into_iter().enumerate() {
                        if !matches!(s, Entry::Sep) {
                            out.push((titre.clone(), format!("{k}.{j}"), s));
                        }
                    }
                    section = format!("{k}+");
                }
                e => out.push((section.clone(), format!("{k}"), e)),
            }
        }
        out
    }

    fn feuille_croix(&self, prix: f64) -> Feuille {
        use argos_render::context_menu::Entry;
        let mut sections: Vec<Section> = vec![];
        let mut cle = None;
        for (s, id, e) in self.croix_lignes(prix) {
            let Entry::Item { icon, label, checked, action } = e else { continue };
            if cle.as_ref() != Some(&s) {
                // (a sub-menu's name stays as the section's title; a separator: a new group without one)
                let titre = if s.chars().next().is_some_and(|c| c.is_ascii_digit()) || s.is_empty() {
                    String::new()
                } else {
                    s.clone()
                };
                sections.push(Section {
                    titre,
                    cols: 1,
                    tuiles: vec![],
                });
                cle = Some(s);
            }
            let mut t = Tuile::new(&id, icon, &label);
            (t.on, t.grise) = (checked, action.is_none());
            if let Some(sec) = sections.last_mut() {
                sec.tuiles.push(t);
            }
        }
        Feuille {
            titre: format!(
                "À {}",
                argos_render::viewport::format_price(prix, argos_render::viewport::data_decimals(&self.chart.bars))
            ),
            sections,
            ..Default::default()
        }
    }

    /// A line of the sheet « À … »: what the chart's menu does with it (a grey « bientôt » line does nothing, as in
    /// the menu); the sheet and the crosshair go.
    fn croix_ligne(&mut self, prix: f64, id: &str) {
        use argos_render::context_menu::Entry;
        let action = self.croix_lignes(prix).into_iter().find(|(_, i, _)| i == id).and_then(|(_, _, e)| match e {
            Entry::Item { action, .. } => action,
            _ => None,
        });
        let Some(a) = action else { return self.redraw() };
        (self.ui.pfeuille, self.ui.croix) = (None, None);
        self.run_action(a);
    }
}

/// 📱 ⚡ The Funding pill of a perpetual on the phone (Jee, 23 Nov 2026, choice A « pilule Funding »): what it says
/// at rest, and its card. Every figure is the PC's row's (`App::refresh_top`, the same texts): nothing is worked out
/// again here.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FundingCell {
    /// « Funding 0.0100 % 05:49:44 »; « Funding — » while the rate is not known.
    pub(crate) pilule: String,
    /// « XRPUSDT Perpétuel · Bitfinex ».
    pub(crate) titre: String,
    /// (name, value, colour): Prix, Variation 24 h, Mark, Intérêt ouvert, Funding, Prochain paiement, Source.
    pub(crate) lignes: Vec<(String, String, Option<argos_render::text::Rgb>)>,
}

/// 📱 ⚡ Where the Funding pill and its card are (window): the pill, the zone a finger takes it by (the pill grown to
/// 34 points high), the card when it is open.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct FundingZones {
    pub(crate) pilule: Rect,
    pub(crate) toucher: Rect,
    pub(crate) carte: Option<Rect>,
}

impl App {
    /// 📱 ⚡ The Funding pill's figures: on the phone, on a perpetual's chart only (not a spot market, not a pair
    /// against BTC), read from the PC's row of the same chart.
    pub(crate) fn funding_cell(&self) -> Option<FundingCell> {
        if !self.top.phone {
            return None;
        }
        let Source::Live { exchange, base } = &self.chart.source else { return None };
        if exchange.market_kind() != "Perpétuel" || argos_data::paire::contre_btc(base) {
            return None;
        }
        let stat = |l: &str| self.top.stats.iter().find(|s| s.label == l);
        let funding = stat("Funding / compte à rebours");
        // (the PC writes « 0.0100 % · 05:49:44 »: the rate, then the time left; « — » when it is not known)
        let (taux, reste) = funding.map_or(("—".to_string(), "—".to_string()), |s| {
            s.value
                .split_once(" · ")
                .map_or((s.value.clone(), "—".to_string()), |(t, r)| (t.to_string(), r.to_string()))
        });
        let pilule = match (taux.as_str(), reste.as_str()) {
            ("—", _) => "Funding —".to_string(),
            (t, "—") => format!("Funding {t}"),
            (t, r) => format!("Funding {t} {r}"),
        };
        let valeur = |s: Option<&Stat>| s.map_or(("—".to_string(), None), |s| (s.value.clone(), s.color));
        // (Hyperliquid's row writes its mark as « Prix »)
        let mark = stat("Mark").or_else(|| stat("Prix").filter(|_| *exchange == Exchange::Hyperliquid));
        let ligne = |nom: &str, (v, c): (String, Option<argos_render::text::Rgb>)| (nom.to_string(), v, c);
        let lignes = vec![
            ligne("Prix", valeur(stat("Prix").or_else(|| stat("Dernier")))),
            ligne("Variation 24 h", valeur(stat("Variation 24 h"))),
            ligne("Mark", valeur(mark)),
            ligne("Intérêt ouvert", valeur(stat("Intérêt ouvert"))),
            ligne("Funding", (taux, funding.and_then(|s| s.color))),
            ligne("Prochain paiement", (if reste == "—" { reste } else { format!("dans {reste}") }, None)),
            ligne("Source", (self.top.source.clone(), None)),
        ];
        Some(FundingCell {
            pilule,
            titre: format!("{} {} · {}", exchange.ticker(base), exchange.market_kind(), exchange.parent().name()),
            lignes,
        })
    }

    /// 📱 ⚡ Nothing lies over the chart where the Funding pill is (a sheet of the pill, a box, a menu, a page): it
    /// is drawn and touched; otherwise what is open passes over it.
    fn funding_libre(&self) -> bool {
        let u = &self.ui;
        u.pfeuille.is_none()
            && !self.modale_ouverte()
            && u.list.is_none()
            && u.sheet.is_none()
            && u.argos.is_none()
            && u.split_menu.is_none()
            && u.pop.is_none()
            && !u.context.is_open()
            && u.accueil.is_none()
            && !u.zen
            && u.fx_page.is_none()
            && u.plan.is_none()
    }

    /// 📱 ⚡ The Funding pill and its card (window), with their figures: right of « ⌄ N » on its line, never over the
    /// price scale; the card just under it.
    pub(crate) fn funding_zones(&self) -> Option<(FundingCell, FundingZones)> {
        let f = self.funding_cell()?;
        let pw = self.chart.viewport.plot_w();
        let b = argos_render::indleg::bouton_cell();
        let x = b.x + b.w + 8.0;
        let room = pw - x - 8.0;
        if room < 60.0 {
            return None;
        }
        let text = argos_render::text::Text::default();
        let w = argos_render::ecrans::funding_largeur(&text, &f.pilule, room);
        let (ox, oy) = (self.rw(), self.top_h());
        let pilule = Rect::new(ox + x, oy + b.y, w, argos_render::ecrans::FUNDING_H);
        let toucher = Rect::new(pilule.x, pilule.cy() - 17.0, pilule.w, 34.0);
        let carte = self.ui.funding_carte.then(|| {
            let cw = (pw - 16.0).min(320.0);
            Rect::new(ox + 8.0, pilule.y + pilule.h + 6.0, cw, argos_render::ecrans::funding_carte_h(f.lignes.len()))
        });
        Some((f, FundingZones { pilule, toucher, carte }))
    }

    /// 📱 ⚡ Is (`wx`, `wy`) (window) on the Funding pill or its open card? The chart then gets nothing there.
    pub(crate) fn funding_sous(&self, wx: f64, wy: f64) -> bool {
        self.funding_libre()
            && !self.sous_avis(wx, wy)
            && self
                .funding_zones()
                .is_some_and(|(_, z)| z.toucher.contains(wx, wy) || z.carte.is_some_and(|c| c.contains(wx, wy)))
    }

    /// 📱 ⚡ Is (`wx`, `wy`) (window) on the « ton trade » card, laid over the pill? (Its place only: `avis_sous` asks
    /// what lies over the chart, and the pill is one of those.)
    fn sous_avis(&self, wx: f64, wy: f64) -> bool {
        self.ui.avis.is_some() && self.avis_place(&argos_render::text::Text::default()).is_some_and(|(_, r)| r.contains(wx, wy))
    }

    /// 📱 ⚡ A press (window): on the pill, its card opens or closes; on the card, nothing (the chart does not move);
    /// anywhere else, an open card closes and the press goes on. `true`: the press was taken.
    pub(crate) fn funding_press(&mut self, wx: f64, wy: f64) -> bool {
        // (the « ton trade » card lies over the pill: it takes the press)
        let zones = self.funding_zones().filter(|_| self.funding_libre() && !self.sous_avis(wx, wy)).map(|(_, z)| z);
        match zones {
            Some(z) if z.toucher.contains(wx, wy) => {
                self.ui.funding_carte = !self.ui.funding_carte;
                self.redraw();
                true
            }
            Some(z) if z.carte.is_some_and(|c| c.contains(wx, wy)) => true,
            _ => {
                if std::mem::take(&mut self.ui.funding_carte) {
                    self.redraw();
                }
                false
            }
        }
    }

    /// 📱 ⚡ The Funding pill and its open card as images (window places), to lay over the chart.
    pub(crate) fn funding_images(&self, text: &argos_render::text::Text) -> Vec<(tiny_skia::Pixmap, Rect)> {
        let Some((f, z)) = self.funding_zones() else { return vec![] };
        let libre = self.funding_libre();
        let mut v: Vec<_> = argos_render::ecrans::funding_pilule(text, &f.pilule, z.pilule.w, libre && z.carte.is_some())
            .map(|pm| (pm, z.pilule))
            .into_iter()
            .collect();
        if let Some(c) = z.carte.filter(|_| libre) {
            v.extend(argos_render::ecrans::funding_carte(text, &f.titre, &f.lignes, c.w).map(|pm| (pm, c)));
        }
        v
    }

    /// 📱 ⚡ The Funding pill (chart place): the high / low pills, the banners and the tables keep clear of it. (Not
    /// its card: it lies over the chart a moment, as a menu does, and nothing under it moves or goes.)
    pub(crate) fn funding_garder(&self) -> Vec<Rect> {
        let Some((_, z)) = self.funding_zones() else { return vec![] };
        let (ox, oy) = (self.rw(), self.top_h());
        vec![Rect::new(z.pilule.x - ox, z.pilule.y - oy, z.pilule.w, z.pilule.h)]
    }
}
