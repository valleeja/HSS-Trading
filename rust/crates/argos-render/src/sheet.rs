//! 🗂 Fenêtres d'Argos v1 (`Sheet`) ouvertes sous la barre du haut : Panneaux (⚙ : panneaux, mise en page, outils et
//! réglages, chacun une fois) et Compte (Connect). Une table de zones sert au dessin et au clic. Chaque action a son
//! icône Phosphor (`Act::icon`) ; ce qui est ouvert est coché ✓, ce qui n'est pas prêt est pâle.

use crate::phosphor;
use crate::shapes::{lines, round_fill, Rect};
use crate::text::{Align, Text};
use tiny_skia::Pixmap;

use crate::palette as pal;
use crate::palette::{GREY, ICON, MUTED, WHITE};

/// Une action d'une fenêtre (Panneaux / Compte).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Act {
    Indicators,
    Demo,
    Journal,
    Agenda,
    Eco,
    Whales,
    Modules,
    Live,
    Chat,
    New,
    Save,
    Open,
    Replay,
    Tpo,
    Footprint,
    PatternsAuto,
    MultiChart,
    CreateInd,
    Heatmap,
    Backtest,
    Alerts,
    IndTemplates,
    ChartType,
    ObjectTree,
    Exchanges,
    // ⚙ Réglages (barre du haut)
    ChartSettings,
    Timeframe,
    OpenPanels,
    Account,
    Params,
    /// 🎨 Thème.
    Theme,
    /// 📥 Reprendre les données d'Argos v1.
    ImportV1,
    /// ⌂ Accueil au lancement (case à cocher).
    AccueilLancement,
    /// ⌂ Revoir les 3 écrans du premier lancement.
    RevoirAccueil,
    /// ⌨️ Raccourcis du clavier (Ctrl + /).
    Raccourcis,
    /// 🔊 Son des alertes: the sound and the volume (a list).
    SonAlertes,
    // Connect · Compte Argos (barre du haut)
    TryDemo,
    CreateAccount,
    HaveKey,
    Passkey,
}

impl Act {
    /// (émoji, nom).
    pub fn face(self) -> (&'static str, &'static str) {
        match self {
            Act::Indicators => ("📈", "Indicateurs"),
            Act::Demo => ("💼", "Trading démo"),
            Act::Journal => ("📓", "Journal"),
            Act::Agenda => ("📅", "Agenda"),
            Act::Eco => ("🌐", "Économie"),
            Act::Whales => ("🐋", "Baleines"),
            Act::Modules => ("🧩", "Modules"),
            Act::Live => ("📺", "Live"),
            Act::Chat => ("💬", "Chat"),
            Act::New => ("🆕", "Nouveau"),
            Act::Save => ("💾", "Sauvegarder"),
            Act::Open => ("📂", "Ouvrir"),
            Act::Replay => ("⏪", "Replay"),
            Act::Tpo => ("📊", "TPO (Market Profile)"),
            Act::Footprint => ("👣", "Footprint"),
            Act::PatternsAuto => ("🔺", "Patterns auto"),
            Act::MultiChart => ("🪟", "Multi-chart"),
            Act::CreateInd => ("🛠", "Créer un indicateur"),
            Act::Heatmap => ("🔥", "Heatmap"),
            Act::Backtest => ("🧪", "Backtest"),
            Act::Alerts => ("⏰", "Alertes"),
            Act::IndTemplates => ("🧩", "Modèles d'indicateurs"),
            Act::ChartType => ("🕯", "Type de graphique"),
            Act::ObjectTree => ("🌳", "Arborescence des objets"),
            Act::Exchanges => ("🔑", "Clés API"),
            Act::ChartSettings => ("⚙️", "Chart"),
            Act::Timeframe => ("⏱", "Unité de temps"),
            Act::OpenPanels => ("🗂", "Panneaux"),
            Act::Account => ("🔐", "Compte"),
            Act::Params => ("🛠", "Paramètres"),
            Act::Theme => ("🎨", "Thème"),
            Act::ImportV1 => ("📥", "Importer v1"),
            Act::AccueilLancement => ("⌂", "Accueil au lancement"),
            Act::RevoirAccueil => ("⌂", "Revoir l'accueil"),
            Act::Raccourcis => ("⌨", "Raccourcis"),
            Act::SonAlertes => ("🔔", "Son des alertes"),
            Act::TryDemo => ("🎮", "Essayer en démo"),
            Act::CreateAccount => ("✨", "Créer un compte"),
            Act::HaveKey => ("🔑", "J'ai déjà une clé"),
            Act::Passkey => ("🔐", "Face ID / empreinte"),
        }
    }

    /// Icône Phosphor (au trait fin, comme le reste de l'interface).
    pub fn icon(self) -> &'static str {
        match self {
            Act::Indicators => "pulse",
            Act::Demo | Act::TryDemo => "game-controller",
            Act::Journal => "notebook",
            Act::Agenda => "calendar",
            Act::Eco => "globe",
            Act::Whales => "fish",
            Act::Modules => "puzzle-piece",
            Act::Live => "video-camera",
            Act::Chat => "chat-circle",
            Act::New => "plus-square",
            Act::Save => "floppy-disk",
            Act::Open => "folder-open",
            Act::Replay => "rewind",
            Act::Tpo => "chart-bar-horizontal",
            Act::Footprint => "footprints",
            Act::PatternsAuto => "triangle",
            Act::MultiChart => "grid-four",
            Act::CreateInd => "wrench",
            Act::Heatmap => "fire",
            Act::Backtest => "flask",
            Act::Alerts => "bell",
            Act::IndTemplates => "stack",
            Act::ChartType => "chart-line",
            Act::ObjectTree => "tree-structure",
            Act::Exchanges => "plugs",
            Act::ChartSettings => "gear",
            Act::Timeframe => "clock",
            Act::OpenPanels => "squares-four",
            Act::Account => "shield-check",
            Act::Params => "sliders-horizontal",
            Act::Theme => "palette",
            Act::ImportV1 => "download-simple",
            Act::AccueilLancement => "compass",
            Act::RevoirAccueil => "arrow-counter-clockwise",
            Act::Raccourcis => "lightning",
            Act::SonAlertes => "bell",
            Act::CreateAccount => "sparkle",
            Act::HaveKey => "key",
            Act::Passkey => "fingerprint",
        }
    }

    /// Nom court, quand le nom ne tient pas dans sa case (📱 cell, comme v1 : « Indic. », « Démo »).
    pub fn short(self) -> &'static str {
        match self {
            Act::Indicators => "Indic.",
            Act::Demo => "Démo",
            Act::Eco => "Éco",
            Act::Exchanges => "Clés",
            Act::ChartType => "Type",
            Act::ObjectTree => "Objets",
            Act::Timeframe => "UT",
            Act::CreateInd => "Créer",
            Act::IndTemplates => "Modèles",
            Act::PatternsAuto => "Patterns",
            Act::Tpo => "TPO",
            Act::Params => "Param.",
            Act::SonAlertes => "Son",
            other => other.face().1,
        }
    }

    /// Déjà fait dans Argos Rust (sinon : « bientôt »).
    pub fn ready(self) -> bool {
        // 🔐 Face ID / empreinte : navigateur et cell seulement (passkey du navigateur)
        (cfg!(target_arch = "wasm32") && self == Act::Passkey)
            || matches!(
                self,
                Act::Indicators
                    | Act::Demo
                    | Act::MultiChart
                    | Act::Journal
                    | Act::Save
                    | Act::ChartType
                    | Act::Exchanges
                    | Act::ChartSettings
                    | Act::Timeframe
                    | Act::OpenPanels
                    | Act::Account
                    | Act::Theme
                    | Act::ImportV1
                    | Act::AccueilLancement
                    | Act::RevoirAccueil
                    | Act::Raccourcis
                    | Act::SonAlertes
                    | Act::Agenda
                    | Act::Eco
                    | Act::Whales
                    | Act::Modules
                    | Act::TryDemo
                    | Act::ObjectTree
                    | Act::Replay
                    | Act::Backtest
                    | Act::CreateAccount
                    | Act::HaveKey
            )
    }
}

/// Panneaux ⇅ (et ⚙ en haut) : panneaux, outils du hub d'analyse et réglages, une seule fois chacun (section,
/// actions). Les Dessins sont dans la barre de gauche et la barre flottante.
pub const PANELS: [(&str, &[Act]); 4] = [
    (
        "PANNEAUX",
        &[
            Act::Indicators,
            Act::Demo,
            Act::Journal,
            Act::Exchanges,
            Act::Agenda,
            Act::Eco,
            Act::Live,
            Act::Chat,
        ],
    ),
    ("MISE EN PAGE", &[Act::New, Act::Save, Act::Open, Act::ChartType]),
    (
        "OUTILS",
        &[
            Act::Replay,
            Act::Alerts,
            Act::Backtest,
            Act::MultiChart,
            Act::Tpo,
            Act::Footprint,
            Act::Heatmap,
            Act::Whales,
            Act::PatternsAuto,
            Act::CreateInd,
            Act::IndTemplates,
            Act::ObjectTree,
        ],
    ),
    (
        "RÉGLAGES",
        &[
            Act::ChartSettings,
            Act::Timeframe,
            Act::Theme,
            Act::Modules,
            Act::ImportV1,
            Act::Params,
            Act::Account,
            Act::AccueilLancement,
            Act::RevoirAccueil,
            Act::Raccourcis,
            Act::SonAlertes,
        ],
    ),
];

/// 🖥 Tools of a PC with several windows (layouts, multi-chart, import from v1): left out on the phone, in « AVANCÉ »
/// on the tablet (Jee, 21 Nov, list 2).
const AVANCE: [Act; 3] = [Act::Save, Act::MultiChart, Act::ImportV1];

/// 📱 A tile kept in phone view (Jee, 20-21 Nov, lists 1 and 2): no grey tile (not ready), no keyboard shortcuts' list,
/// nothing that is elsewhere for the finger (the objects' tree is in the long press, « Revoir l'accueil » in
/// Réglages); on the phone also nothing already in the ⋯ Argos sheet (alerts, replay, backtest, chart type, whales,
/// modules).
fn au_doigt(a: Act, telephone: bool) -> bool {
    a.ready()
        && !AVANCE.contains(&a)
        && !matches!(a, Act::Raccourcis | Act::ObjectTree | Act::RevoirAccueil)
        && !(telephone && matches!(a, Act::Alerts | Act::Replay | Act::Backtest | Act::ChartType | Act::Whales | Act::Modules))
}

/// Taille des cases des Panneaux (4 colonnes) et hauteur d'un titre de section.
const PCOL: f64 = 110.0;
const PROW: f64 = 74.0;
const PTITLE: f64 = 24.0;

/// Connect · Compte Argos (comme v1) ; seule la démo est faite (le compte et les clés : plus tard, jamais tes 24 mots ici).
pub const ACCOUNT: [Act; 4] = [Act::TryDemo, Act::CreateAccount, Act::HaveKey, Act::Passkey];

/// Ce que montre la fenêtre.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SheetKind {
    Panels,
    /// 📱 Panneaux ⇅ du cell : seulement la section PANNEAUX (comme v1).
    Quick,
    /// Connect · Compte Argos (sous la barre du haut).
    Account,
}

/// Résultat d'un clic dans la fenêtre.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SheetClick {
    None,
    Close,
    Act(Act),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Zone {
    Close,
    Act(Act),
}

/// La fenêtre ouverte (coordonnées de la fenêtre de l'app).
#[derive(Debug, Clone, PartialEq)]
pub struct Sheet {
    pub kind: SheetKind,
    pub rect: Rect,
    /// Largeur d'une case (plus petite sur un cell, pour que les 4 colonnes entrent).
    col: f64,
    /// Hauteur de tout le contenu et défilement (plus haute que l'écran du téléphone : elle défile).
    pub full: f64,
    pub scroll: f64,
    /// Compte Argos : hauteur de la phrase d'intro (toutes ses lignes) ; les boutons commencent dessous.
    intro: f64,
    /// 📱 A phone-width window (less than 700): in phone view, fewer tiles than on the tablet.
    telephone: bool,
}

const PAD: f64 = 14.0;
const HEAD: f64 = 48.0;
/// Compte Argos : taille et interligne de la phrase d'intro, hauteur et pas des boutons.
const INTRO_PT: f32 = 11.5;
const INTRO_LH: f64 = 16.0;
const BTN_H: f64 = 46.0;
const BTN_PAS: f64 = 52.0;

/// Compte Argos : la phrase d'intro (le compte de cet appareil, sinon l'explication) et sa couleur.
fn intro_texte(account: &str) -> (String, crate::text::Rgb) {
    if account.is_empty() {
        (
            "Le trading démo marche sans compte. Ton compte et ta clé : c'est toi qui les entres, jamais tes 24 mots ici.".to_string(),
            GREY,
        )
    } else {
        (format!("✅ Connecté : {account} — clés chiffrées avec ton code sur cet appareil"), WHITE)
    }
}

/// Compte Argos : la phrase d'intro coupée en lignes entières pour une fenêtre large de `pw`.
fn intro_lignes(text: &Text, account: &str, pw: f64) -> Vec<String> {
    crate::assistant::wrap(text, &intro_texte(account).0, INTRO_PT, pw - 2.0 * PAD)
}

/// Compte Argos : hauteur du bloc d'intro (lignes + écart avant le 1er bouton).
fn intro_h(account: &str, pw: f64) -> f64 {
    intro_lignes(&Text::default(), account, pw).len() as f64 * INTRO_LH + 16.0
}

impl Sheet {
    /// Fenêtre posée sur la barre (`bar_y` = haut de la barre), alignée sur le bouton `anchor` ; fenêtre `w`.
    pub fn new(kind: SheetKind, anchor: Rect, bar_y: f64, w: f64) -> Self {
        let col = PCOL.min((w - 12.0 - 2.0 * PAD) / 4.0);
        let intro = if kind == SheetKind::Account {
            intro_h("", 420.0_f64.min(w - 12.0))
        } else {
            0.0
        };
        let (pw, ph) = match kind {
            SheetKind::Panels | SheetKind::Quick => (
                4.0 * col + 2.0 * PAD,
                HEAD + Self::sections(kind, w < 700.0)
                    .iter()
                    .map(|(_, a)| PTITLE + a.len().div_ceil(4) as f64 * PROW)
                    .sum::<f64>()
                    + PAD,
            ),
            SheetKind::Account => (420.0_f64.min(w - 12.0), HEAD + intro + ACCOUNT.len() as f64 * BTN_PAS + PAD),
        };
        let x = (anchor.cx() - pw / 2.0).clamp(6.0, (w - pw - 6.0).max(6.0));
        // 📱 cell : la fenêtre prend toute la largeur (comme v1)
        let (x, pw, col) = if w < 700.0 && kind != SheetKind::Account {
            (6.0, w - 12.0, (w - 12.0 - 2.0 * PAD) / 4.0)
        } else {
            (x, pw, col)
        };
        // ce qui tient entre le haut de l'écran et la barre ; le reste défile
        let vis = ph.min((bar_y - 12.0).max(120.0));
        Self {
            kind,
            rect: Rect::new(x, (bar_y - vis - 6.0).max(6.0), pw, vis),
            col,
            full: ph,
            scroll: 0.0,
            intro,
            telephone: w < 700.0,
        }
    }

    /// Compte Argos : la phrase d'intro de ce compte (`account`, vide s'il n'y en a pas) ; la fenêtre grandit ou
    /// rapetisse pour que toutes ses lignes tiennent au-dessus des boutons, et reste dans la fenêtre de l'app (haute de
    /// `win_h`) : elle remonte si elle dépasse en bas, et défile si elle est plus haute que la fenêtre.
    pub fn compte(mut self, account: &str, win_h: f64) -> Self {
        if self.kind == SheetKind::Account {
            self.intro = intro_h(account, self.rect.w);
            self.full = HEAD + self.intro + ACCOUNT.len() as f64 * BTN_PAS + PAD;
            self.rect.h = self.full.min((win_h - 12.0).max(120.0));
            self.rect.y = self.rect.y.min(win_h - self.rect.h - 6.0).max(6.0);
        }
        self
    }

    /// Défile de `dy` points (positif : vers le bas de la fenêtre).
    pub fn wheel(&mut self, dy: f64) {
        self.scroll = (self.scroll + dy).clamp(0.0, (self.full - self.rect.h).max(0.0));
    }

    /// Le contenu entier, décalé du défilement (coordonnées de la fenêtre de l'app).
    fn content(&self) -> Rect {
        Rect::new(self.rect.x, self.rect.y - self.scroll, self.rect.w, self.full.max(self.rect.h))
    }

    /// Sections montrées (📱 Panneaux ⇅ : la 1re seulement).
    /// 📱 Phone view: only the tiles that work and are not elsewhere (see [`au_doigt`]); on the tablet, the PC's
    /// tools go in « AVANCÉ » at the end.
    fn sections(kind: SheetKind, telephone: bool) -> Vec<(&'static str, Vec<Act>)> {
        let s: &[(&str, &[Act])] = match kind {
            SheetKind::Quick => &PANELS[..1],
            _ => &PANELS,
        };
        let doigt = crate::actions::tactile();
        let mut out: Vec<(&'static str, Vec<Act>)> = s
            .iter()
            .map(|(t, a)| (*t, a.iter().copied().filter(|a| !doigt || au_doigt(*a, telephone)).collect()))
            .collect();
        if doigt && !telephone && kind == SheetKind::Panels {
            out.push(("AVANCÉ", AVANCE.to_vec()));
        }
        out.retain(|(_, a)| !a.is_empty());
        out
    }

    /// Fenêtre posée sous une barre (`top` = bas de la barre du haut).
    pub fn below(kind: SheetKind, anchor: Rect, top: f64, w: f64) -> Self {
        let mut s = Self::new(kind, anchor, 10_000.0, w);
        s.rect.y = top + 6.0;
        s
    }

    fn zones(&self) -> Vec<(Rect, Zone)> {
        let r = self.content();
        let mut out = vec![(Rect::new(r.x + r.w - 40.0, r.y + 10.0, 28.0, 28.0), Zone::Close)];
        let top = r.y + HEAD;
        match self.kind {
            SheetKind::Panels | SheetKind::Quick => {
                let mut y = top;
                for (_, acts) in Self::sections(self.kind, self.telephone) {
                    y += PTITLE;
                    for (i, a) in acts.iter().enumerate() {
                        out.push((
                            Rect::new(r.x + PAD + (i % 4) as f64 * self.col, y + (i / 4) as f64 * PROW, self.col - 6.0, PROW - 6.0),
                            Zone::Act(*a),
                        ));
                    }
                    y += acts.len().div_ceil(4) as f64 * PROW;
                }
            }
            SheetKind::Account => {
                for (i, a) in ACCOUNT.iter().enumerate() {
                    out.push((
                        Rect::new(r.x + PAD, top + self.intro + i as f64 * BTN_PAS, r.w - 2.0 * PAD, BTN_H),
                        Zone::Act(*a),
                    ));
                }
            }
        }
        out
    }

    /// Clic (coordonnées de la fenêtre de l'app).
    pub fn click(&self, x: f64, y: f64) -> SheetClick {
        if !self.rect.contains(x, y) {
            return SheetClick::Close;
        }
        match self.sous(x, y) {
            Some(Zone::Close) => SheetClick::Close,
            Some(Zone::Act(a)) => SheetClick::Act(a),
            None => SheetClick::None,
        }
    }

    /// La zone sous (`x`, `y`) : la même pour le survol et le clic, seulement dans la partie visible de la fenêtre.
    fn sous(&self, x: f64, y: f64) -> Option<Zone> {
        if !self.rect.contains(x, y) {
            return None;
        }
        self.zones().into_iter().rev().find(|(r, _)| r.contains(x, y)).map(|(_, z)| z)
    }

    /// Compte Argos : le bouton surligné sous la souris (aucun hors des boutons).
    pub fn survol(&self, x: f64, y: f64) -> Option<Act> {
        match self.sous(x, y) {
            Some(Zone::Act(a)) => Some(a),
            _ => None,
        }
    }

    /// `account` : l'ID du compte de cet appareil (`@nom#tag`), vide s'il n'y en a pas ; `open` : ce qui est déjà
    /// ouvert (coché ✓).
    pub fn draw(&self, text: &Text, account: &str, mouse: (f64, f64), open: &dyn Fn(Act) -> bool) -> Option<(Pixmap, Rect)> {
        let r = self.content();
        let mut pm = crate::px::new(r.w as u32, r.h as u32)?;
        let loc = |q: Rect| Rect::new(q.x - r.x, q.y - r.y, q.w, q.h);
        crate::shapes::menu_card(&mut pm, loc(r));
        let title = match self.kind {
            SheetKind::Panels | SheetKind::Quick => "Panneaux",
            SheetKind::Account => "Compte Argos",
        };
        text.draw(&mut pm, title, ((r.w / 2.0) as f32, 31.0), 16.5, WHITE, Align::Center);
        if let SheetKind::Account = self.kind {
            // le compte de cet appareil (créé ou retrouvé avec les 24 mots), sinon l'explication : en entier, sur autant
            // de lignes qu'il faut (la place est prise par `compte`) ; une ligne qui ne tiendrait pas n'est pas dessinée
            let c = intro_texte(account).1;
            let n = ((self.intro - 16.0) / INTRO_LH).round().max(0.0) as usize;
            for (i, l) in intro_lignes(text, account, r.w).iter().take(n).enumerate() {
                text.draw(&mut pm, l, (PAD as f32, (HEAD + 14.0 + i as f64 * INTRO_LH) as f32), INTRO_PT, c, Align::Left);
            }
        }
        let survol = self.sous(mouse.0, mouse.1);
        if matches!(self.kind, SheetKind::Panels | SheetKind::Quick) {
            let mut y = HEAD;
            for (title, acts) in Self::sections(self.kind, self.telephone) {
                text.draw(&mut pm, title, (PAD as f32, (y + 15.0) as f32), 11.0, GREY, Align::Left);
                y += PTITLE + acts.len().div_ceil(4) as f64 * PROW;
            }
        }
        for (q, z) in self.zones() {
            let lq = loc(q);
            let over = survol == Some(z);
            match z {
                Zone::Close => {
                    round_fill(&mut pm, lq, 8.0, if over { pal::hover() } else { pal::tile() }, 1.0);
                    lines(
                        &mut pm,
                        lq.cx(),
                        lq.cy(),
                        &[&[(-4.5, -4.5), (4.5, 4.5)], &[(4.5, -4.5), (-4.5, 4.5)]],
                        ICON,
                        1.5,
                    );
                }
                Zone::Act(a) if self.kind == SheetKind::Account => {
                    let (_, name) = a.face();
                    round_fill(&mut pm, lq, 12.0, if over { pal::hover() } else { pal::tile() }, 1.0);
                    phosphor::draw(&mut pm, a.icon(), lq.x + 24.0, lq.cy(), 24, if a.ready() { ICON } else { MUTED });
                    text.draw(
                        &mut pm,
                        name,
                        ((lq.x + 50.0) as f32, (lq.cy() + 5.0) as f32),
                        14.0,
                        if a.ready() { WHITE } else { GREY },
                        Align::Left,
                    );
                    if open(a) {
                        phosphor::check(&mut pm, lq.x + lq.w - 18.0, lq.cy(), 18.0);
                    } else if !a.ready() {
                        phosphor::soon(&mut pm, text, lq.x + lq.w - 10.0, lq.cy());
                    }
                }
                Zone::Act(a) => {
                    // comme l'écran d'accueil d'un cell : l'icône, le nom dessous ; fond seulement au survol
                    let (_, name) = a.face();
                    if over {
                        round_fill(&mut pm, lq, 12.0, pal::hover(), 1.0);
                    }
                    phosphor::draw(&mut pm, a.icon(), lq.cx(), lq.y + 26.0, 30, if a.ready() { ICON } else { MUTED });
                    let col = if a.ready() { WHITE } else { GREY };
                    // 📱 phone view: the whole name, on two lines if it must (« Accueil au lancement », not cut)
                    let lignes = crate::assistant::wrap(text, name, 11.0, lq.w - 6.0);
                    if crate::actions::tactile() && lignes.len() == 2 {
                        for (k, l) in lignes.iter().enumerate() {
                            text.draw(
                                &mut pm,
                                l,
                                (lq.cx() as f32, (lq.y + lq.h - 19.0 + 12.0 * k as f64) as f32),
                                11.0,
                                col,
                                Align::Center,
                            );
                        }
                    } else {
                        let label = text.fit(if text.width(name, 11.5) as f64 > lq.w - 8.0 { a.short() } else { name }, 11.5, lq.w - 8.0);
                        text.draw(&mut pm, &label, (lq.cx() as f32, (lq.y + lq.h - 8.0) as f32), 11.5, col, Align::Center);
                    }
                    if open(a) {
                        phosphor::check(&mut pm, lq.cx() + 18.0, lq.y + 9.0, 16.0);
                    }
                }
            }
        }
        // plus haute que l'écran : la partie visible, avec une barre de défilement fine
        let v = self.rect;
        if self.full > v.h {
            let k = v.h / self.full;
            crate::jetons::barre_defilement(&mut pm, v.w, self.scroll + self.scroll * k + 4.0, (v.h * k - 8.0).max(16.0), false);
            let part = crate::px::int_rect(0.0, self.scroll, v.w, v.h).and_then(|ir| pm.clone_rect(ir))?;
            return Some((part, v));
        }
        Some((pm, v))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fenetres() {
        let t = Text::default();
        let anchor = Rect::new(900.0, 800.0, 170.0, 38.0);
        let p = Sheet::new(SheetKind::Panels, anchor, 792.0, 1400.0);
        assert!(p.rect.y + p.rect.h <= 792.0, "au-dessus de la barre");
        let (q, _) = p.zones().into_iter().find(|(_, z)| *z == Zone::Act(Act::Journal)).unwrap();
        assert_eq!(p.click(q.cx(), q.cy()), SheetClick::Act(Act::Journal));
        assert_eq!(p.click(0.0, 0.0), SheetClick::Close);
        let st = Sheet::below(SheetKind::Panels, Rect::new(1300.0, 10.0, 30.0, 30.0), 50.0, 1400.0);
        assert!(st.rect.y > 50.0 && st.rect.x + st.rect.w <= 1400.0);
        let (q, _) = st.zones().into_iter().find(|(_, z)| *z == Zone::Act(Act::Timeframe)).unwrap();
        assert_eq!(st.click(q.cx(), q.cy()), SheetClick::Act(Act::Timeframe));
        for k in [SheetKind::Panels, SheetKind::Quick, SheetKind::Account] {
            assert!(Sheet::new(k, anchor, 792.0, 1400.0).draw(&t, "", (0.0, 0.0), &|a| a == Act::Journal).is_some());
            // 📱 sur un cell de 390 de large, la fenêtre entre dans l'écran
            let ph = Sheet::new(k, Rect::new(10.0, 740.0, 180.0, 44.0), 740.0, 390.0);
            assert!(ph.rect.x >= 6.0 && ph.rect.x + ph.rect.w <= 384.0, "{k:?} {:?}", ph.rect);
        }
    }

    /// 📱 Jee (21 Nov, list 2): Panneaux by the finger shows no grey tile and nothing that is elsewhere; the PC's tools
    /// are gone on the phone and in « AVANCÉ » on the tablet; the PC keeps every tile.
    #[test]
    fn panneaux_au_doigt() {
        let anchor = Rect::new(10.0, 740.0, 180.0, 44.0);
        let tuiles = |w: f64| -> Vec<Act> {
            Sheet::new(SheetKind::Panels, anchor, 740.0, w)
                .zones()
                .into_iter()
                .filter_map(|(_, z)| match z {
                    Zone::Act(a) => Some(a),
                    Zone::Close => None,
                })
                .collect()
        };
        crate::actions::set_tactile(false);
        let pc = tuiles(1440.0);
        assert_eq!(pc.len(), PANELS.iter().map(|(_, a)| a.len()).sum::<usize>(), "PC: every tile");
        assert!(pc.iter().any(|a| !a.ready()), "PC: the grey tiles stay");
        crate::actions::set_tactile(true);
        let (tel, tab) = (tuiles(402.0), tuiles(820.0));
        for (nom, v) in [("phone", &tel), ("tablet", &tab)] {
            assert!(v.iter().all(|a| a.ready()), "{nom}: no grey tile");
            assert!(!v.contains(&Act::ObjectTree) && !v.contains(&Act::RevoirAccueil), "{nom}: once only");
            assert!(v.contains(&Act::AccueilLancement) && v.contains(&Act::Indicators));
        }
        assert!(!AVANCE.iter().any(|a| tel.contains(a)), "phone: no PC tools");
        assert!(!tel.contains(&Act::Replay) && !tel.contains(&Act::ChartType), "phone: not twice with ⋯ Argos");
        let t = Sheet::new(SheetKind::Panels, anchor, 740.0, 820.0);
        let s = Sheet::sections(t.kind, t.telephone);
        assert_eq!(
            s.last().map(|(n, a)| (*n, a.clone())),
            Some(("AVANCÉ", AVANCE.to_vec())),
            "tablet: in « AVANCÉ »"
        );
        assert!(s.iter().all(|(n, a)| *n == "AVANCÉ" || !AVANCE.iter().any(|x| a.contains(x))), "and not twice");
        // the whole name, on two lines, in a phone tile
        let tel = Sheet::new(SheetKind::Panels, anchor, 740.0, 402.0);
        let q = tel.zones().into_iter().find(|(_, z)| *z == Zone::Act(Act::AccueilLancement)).unwrap().0;
        let l = crate::assistant::wrap(&Text::default(), "Accueil au lancement", 11.0, q.w - 6.0);
        assert!(l.len() <= 2 && l.join(" ") == "Accueil au lancement", "{l:?}");
        crate::actions::set_tactile(false);
    }
}

#[cfg(test)]
mod tests_compte {
    use super::*;

    /// 🔐 Compte Argos, de 320 à 1400 de large : la phrase d'intro revient à la ligne en entier (aucun « … »), le
    /// 1er bouton commence sous sa dernière ligne, rien ne se chevauche ; le survol = le clic = le bouton dessiné,
    /// en haut et en bas de chaque bouton, et aucun entre deux boutons ni sur l'intro.
    #[test]
    fn compte_argos_texte_entier_et_survol_exact() {
        let t = Text::default();
        for w in [320.0, 390.0, 402.0, 700.0, 1440.0] {
            for account in ["", "@jee#4821"] {
                let s = Sheet::below(SheetKind::Account, Rect::new(w - 200.0, 10.0, 60.0, 30.0), 48.0, w).compte(account, 900.0);
                let lignes = intro_lignes(&t, account, s.rect.w);
                assert!(lignes.iter().all(|l| !l.contains('…')), "{w} : {lignes:?}");
                assert_eq!(lignes.join(" "), intro_texte(account).0, "{w} : texte entier");
                assert!(
                    lignes.iter().all(|l| t.width(l, INTRO_PT) as f64 <= s.rect.w - 2.0 * PAD),
                    "{w} : dans la fenêtre"
                );
                let derniere = s.rect.y + HEAD + 14.0 + (lignes.len() - 1) as f64 * INTRO_LH + 4.0;
                let boutons: Vec<Rect> = s.zones().into_iter().filter(|(_, z)| matches!(z, Zone::Act(_))).map(|(r, _)| r).collect();
                assert_eq!(boutons.len(), ACCOUNT.len());
                assert!(
                    boutons[0].y >= derniere + 6.0,
                    "{w} {account:?} : 1er bouton {} sous l'intro {derniere}",
                    boutons[0].y
                );
                let b = boutons.last().unwrap();
                assert!(b.y + b.h <= s.rect.y + s.rect.h, "{w} : la fenêtre grandit");
                for (i, (q, a)) in boutons.iter().zip(ACCOUNT).enumerate() {
                    for y in [q.y + 1.0, q.cy(), q.y + q.h - 1.0] {
                        assert_eq!(s.survol(q.cx(), y), Some(a), "{w} : bouton {i}, y {y}");
                        assert_eq!(s.click(q.cx(), y), SheetClick::Act(a));
                    }
                    if let Some(n) = boutons.get(i + 1) {
                        let entre = (q.y + q.h + n.y) / 2.0;
                        assert_eq!(s.survol(q.cx(), entre), None, "{w} : entre deux boutons");
                        assert_eq!(s.click(q.cx(), entre), SheetClick::None);
                    }
                }
                assert_eq!(s.survol(s.rect.cx(), s.rect.y + HEAD + 10.0), None, "{w} : rien sur l'intro");
                assert_eq!(s.survol(s.rect.x - 5.0, boutons[0].cy()), None, "{w} : rien hors de la fenêtre");
                assert!(s.draw(&t, account, (boutons[1].cx(), boutons[1].cy()), &|_| false).is_some());
            }
        }
    }
}

#[cfg(test)]
mod tests_compte_dans_l_ecran {
    use super::*;

    /// 🔐 Compte Argos ouvert depuis le menu ARGOS (« Se connecter », bas de la fenêtre) : elle remonte pour rester
    /// entière dans l'écran ; dans une fenêtre trop basse, elle prend la hauteur de l'écran et défile.
    #[test]
    fn compte_argos_reste_dans_l_ecran() {
        let ancre = Rect::new(60.0, 640.0, 400.0, 30.0);
        let s = Sheet::below(SheetKind::Account, ancre, ancre.y + ancre.h, 1440.0).compte("", 900.0);
        assert!(s.rect.y >= 6.0 && s.rect.y + s.rect.h <= 894.0, "{:?}", s.rect);
        assert_eq!(s.rect.h, s.full, "entière");
        let b = Sheet::below(SheetKind::Account, ancre, ancre.y + ancre.h, 1440.0).compte("", 300.0);
        assert!(b.rect.y >= 6.0 && b.rect.y + b.rect.h <= 294.0, "{:?}", b.rect);
        assert!(b.full > b.rect.h, "elle défile");
    }
}
