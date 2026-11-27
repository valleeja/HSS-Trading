//! 🔝 Menus de la barre du haut d'Argos v1 (copiés de localhost) : menu Argos ▾ (carte : trading, marchés, compte,
//! gagner, Argos),
//! Mises en page ▾ (onglets, sauvegarder, sauvegarde automatique, renommer, modèles Argos) et 🌐 Langue et format des
//! nombres. Une table (`TopAct`) : nom, icône (Argos ▾ : icône Phosphor), prêt ou « bientôt » ; la
//! fenêtre de l'app fait l'action.

use crate::listmenu::{MenuGroup, MenuItem};
use crate::palette as pal;
use crate::shapes::Rect;
use crate::text::{Align, Rgb, Text};
use tiny_skia::Pixmap;

/// Une ligne d'un menu du haut.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopAct {
    // Argos ▾ · navigation
    Trade = 1,
    Portfolio,
    Backtest,
    Journal,
    Agenda,
    News,
    Hub,
    // compte
    MyAccount,
    Wallet,
    DemoAccount,
    // gagner
    Mining,
    Earn,
    Staking,
    Referral,
    Leaderboard,
    // outils
    Funding,
    ApiKeys,
    AiConnector,
    Explorer,
    Stats,
    Announcements,
    Manual,
    Academy,
    /// 👥 Communauté (top, académie, marketplace, idées).
    Community,
    /// 🧩 Modules (bac à sable WASM : installer, droits, démarrer).
    Modules,
    // appli : mises à jour, installation, affichage
    Updates,
    Install,
    CopyUrl,
    Fullscreen,
    ZoomOut,
    ZoomReset,
    ZoomIn,
    ClearCache,
    About,
    Home,
    ScreenTest,
    /// 🌐 Langue et format des nombres (☰ Appli ; avant : 🌐 dans la barre du haut).
    Langue,
    /// ⌂ Ouvrir l'Accueil ; « Accueil au lancement » ; 📱 barre du haut auto / toujours visible.
    Accueil,
    AccueilLancement,
    /// Menu ARGOS de la maquette (tel-13) : 🌍 Marchés, 🛒 Marketplace, 🛡 Gardien (le Simulateur), ⏳ Bientôt (la
    /// liste de ce qui arrive).
    Marches,
    Marketplace,
    Gardien,
    Bientot,
    /// 🫥 PC : estomper les barres sur la chart (allumé / éteint).
    Estomper,
    // mises en page
    NewLayout,
    Save,
    AutoSave,
    Copy,
    Rename,
    Publish,
    Download,
    TemplateSmc,
    TemplateCipher,
    TemplateTrend,
    TemplateTbo,
    TemplateOrderFlow,
    // langue : formats des nombres (100 + index), langues (200 + index)
    Reset,
    /// ⌨ Raccourcis du clavier (menu ARGOS du cell, comme la maquette).
    Raccourcis,
    /// 💻 Version ordi: the desktop app's download page (☰ Appli; before, a button of the welcome page only, which
    /// no longer opens at the start — Jee, 25 Nov 2026).
    Desktop,
}

/// Ligne d'un menu en liste : (action, émoji, nom, sous-titre, prête dans Argos Rust).
type Row = (TopAct, &'static str, &'static str, &'static str, bool);

/// (action, icône Phosphor, nom, sous-titre, prête dans Argos Rust).
type Tile = (TopAct, &'static str, &'static str, &'static str, bool);

/// Argos ▾, regroupé : trader, suivre les marchés, son compte, gagner, Argos lui-même.
const ARGOS: [(&str, &[Tile]); 5] = [
    (
        "TRADING",
        &[
            (TopAct::Trade, "chart-line-up", "Trade", "chart et ordres", true),
            (TopAct::Portfolio, "briefcase", "Portfolio", "positions et historique", true),
            (TopAct::Journal, "notebook", "Journal", "tes trades", true),
            (TopAct::Backtest, "flask", "Backtest", "tester une stratégie", true),
        ],
    ),
    (
        "MARCHÉS",
        &[
            (TopAct::News, "newspaper", "Nouvelles", "actus marchés", true),
            (TopAct::Agenda, "calendar", "Agenda", "calendrier éco", true),
            (TopAct::Funding, "percent", "Funding", "taux du marché", false),
            (TopAct::Explorer, "compass", "Explorer", "adresse Hyperliquid", true),
            (TopAct::Stats, "chart-bar", "Stats", "volume Hyperliquid", true),
        ],
    ),
    (
        "COMPTE",
        &[
            (TopAct::DemoAccount, "game-controller", "Compte démo", "trader sans risque", true),
            (TopAct::MyAccount, "user-circle", "Mon compte", "profil, clé, connexion", true),
            (TopAct::Wallet, "wallet", "Wallet", "lecture seule, ton adresse", true),
            (TopAct::ApiKeys, "key", "Clés API", "exchanges, dans ton trousseau", true),
            (TopAct::AiConnector, "plugs-connected", "Connecteur AI", "brancher Claude, ChatGPT…", false),
        ],
    ),
    (
        "GAGNER",
        &[
            (TopAct::Mining, "hammer", "Minage", "récompenses Argos", false),
            (TopAct::Earn, "coins", "Earn", "intérêts", false),
            (TopAct::Staking, "lock-simple", "Staking", "bloquer pour gagner", false),
            (TopAct::Referral, "handshake", "Parrainage", "inviter des amis", false),
            (TopAct::Leaderboard, "trophy", "Classement", "meilleurs traders démo", false),
        ],
    ),
    (
        "ARGOS",
        &[
            (TopAct::Hub, "squares-four", "Panneaux", "panneaux et tous les outils", true),
            (TopAct::Community, "users-three", "Communauté", "top 24 h, académie, idées", true),
            (TopAct::Modules, "puzzle-piece", "Modules", "bac à sable : installer, droits", true),
            (TopAct::Academy, "graduation-cap", "Académie", "parcours en 8 étapes, quiz", true),
            (TopAct::Announcements, "megaphone", "Annonces", "nouveautés d'Argos", false),
            (TopAct::Manual, "book-open", "Manuel d'Argos", "comment utiliser chaque section", false),
        ],
    ),
];

/// ☰ Appli : mises à jour, installation, affichage (ouvert par le bouton ☰ de l'en-tête d'Argos ▾).
const APP_MENU: [(&str, &[Tile]); 1] = [(
    "APPLI",
    &[
        (TopAct::Updates, "arrows-clockwise", "Mises à jour", "version et date du build", true),
        (TopAct::Install, "download-simple", "Installer l'appli", "écran d'accueil, iPhone, iPad", true),
        (TopAct::Desktop, "export", "Version ordi", "l'app pour ton ordi", true),
        (TopAct::CopyUrl, "copy", "Copier l'URL", "lien de cette chart", true),
        (TopAct::Fullscreen, "layout", "Plein écran", "entrer / sortir", true),
        (TopAct::ZoomOut, "minus", "Zoom −", "interface plus petite", true),
        (TopAct::ZoomIn, "plus", "Zoom +", "interface plus grande", true),
        (TopAct::ZoomReset, "magnifying-glass", "Zoom 100 %", "taille normale", true),
        (TopAct::ClearCache, "trash", "Vider le cache", "et recharger", true),
        (TopAct::About, "seal-check", "Infos", "version, commit, licences", true),
        (TopAct::Home, "squares-four", "Page d'accueil", "la présentation d'Argos", true),
        (TopAct::ScreenTest, "ruler", "Test écran", "hauteur sur iPhone", true),
        (TopAct::ApiKeys, "key", "Clés API", "ajouter une API (optionnel)", true),
        (TopAct::Langue, "globe", "Langue", "langue et format des nombres", true),
        (TopAct::Accueil, "squares-four", "⌂ Accueil", "tes cartes et mises en page (Alt+H)", true),
        (TopAct::AccueilLancement, "squares-four", "Accueil au lancement", "allumer / éteindre", true),
        (TopAct::Estomper, "eye-slash", "Estomper les barres", "quand tu travailles sur la chart", true),
    ],
)];

const LAYOUTS: [(&str, &[Row]); 2] = [
    (
        "",
        &[
            (TopAct::NewLayout, "➕", "Nouvelle mise en page", "", true),
            (TopAct::Save, "💾", "Sauvegarder", "Ctrl + S", true),
            (TopAct::AutoSave, "🔄", "Sauvegarde automatique", "", true),
            (TopAct::Copy, "📄", "Créer une copie", "", true),
            (TopAct::Rename, "✏️", "Renommer", "", true),
            (TopAct::Publish, "📤", "Publier la mise en page (lien)", "", false),
            (TopAct::Download, "⬇️", "Télécharger les données", "", false),
        ],
    ),
    (
        "MODÈLES ARGOS",
        &[
            (TopAct::TemplateSmc, "🥇", "Argos SMC Or", "XAUUSD 4h", false),
            (TopAct::TemplateCipher, "🟠", "Argos Cipher + TVD", "BTC 4h", false),
            (TopAct::TemplateTrend, "🟠", "Argos AI Trend + OB/FVG", "BTC 4h", false),
            (TopAct::TemplateTbo, "🔷", "Argos Momentum TBO", "ETH 4h", false),
            (TopAct::TemplateOrderFlow, "🟠", "Argos Order Flow", "BTC 15m", false),
        ],
    ),
];

impl TopAct {
    pub fn from_id(id: i64) -> Option<Self> {
        ARGOS
            .iter()
            .chain(APP_MENU.iter())
            .flat_map(|(_, r)| r.iter().map(|t| t.0))
            .chain(LAYOUTS.iter().flat_map(|(_, r)| r.iter().map(|r| r.0)))
            // (the essential rows, the phone's lines and the account card's Gardien pill: Marchés, Marketplace and
            // Gardien were left out, so their click did nothing — /app2/ 22 Nov)
            .chain(ESSENTIEL.iter().map(|e| e.0))
            .chain(LIGNES_CELL.iter().map(|l| l.0))
            .chain([TopAct::Reset, TopAct::Raccourcis, TopAct::Gardien])
            .find(|a| *a as i64 == id)
    }

    /// Nom et prête ?
    pub fn info(self) -> (&'static str, bool) {
        ARGOS
            .iter()
            .chain(APP_MENU.iter())
            .flat_map(|(_, r)| r.iter().map(|t| (t.0, t.2, t.4)))
            .chain(LAYOUTS.iter().flat_map(|(_, r)| r.iter().map(|r| (r.0, r.2, r.4))))
            .find(|r| r.0 == self)
            .map_or(
                if self == TopAct::Raccourcis {
                    ("Raccourcis du clavier", true)
                } else {
                    ("Réinitialiser", true)
                },
                |r| (r.1, r.2),
            )
    }
}

fn rows(sections: &[(&str, &[Row])], auto_save: bool) -> Vec<MenuGroup> {
    sections
        .iter()
        .map(|(title, list)| MenuGroup {
            title: title.to_string(),
            items: list
                .iter()
                .map(|(a, emoji, name, sub, ready)| MenuItem {
                    id: *a as i64,
                    label: if *ready { name.to_string() } else { format!("{name} (bientôt)") },
                    icon: Some(emoji),
                    // (📱 phone view: no « Ctrl + S », there is no keyboard)
                    hint: crate::actions::sans_raccourci(sub),
                    checked: *a == TopAct::AutoSave && auto_save,
                    enabled: true,
                    ..Default::default()
                })
                .collect(),
            strip: false,
        })
        .collect()
}

/// Ce qu'un clic dans le menu Argos ▾ demande.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArgosClick {
    None,
    Close,
    Act(TopAct),
    /// ☰ Appli ↔ ‹ Argos.
    Switch,
}

/// La carte du compte en haut du menu ARGOS (maquette tel-13) : remplie par l'app.
#[derive(Debug, Clone, PartialEq)]
pub struct CompteCarte {
    /// Compte Argos connecté (`@nom`) ; `None` : « Aucun compte connecté » et « Se connecter ».
    pub compte: Option<String>,
    /// Compte démo actif, solde, P&L du jour (texte et couleur).
    pub demo: String,
    pub solde: String,
    pub pnl: (String, Rgb),
    /// 🛡 Pilule du Gardien (« 0,8 % / 5 % », couleur) ; `None` : Gardien éteint.
    pub gardien: Option<(String, Rgb)>,
    /// Wallet : l'adresse courte, ou « non connecté ».
    pub wallet: String,
}

impl Default for CompteCarte {
    fn default() -> Self {
        Self {
            compte: None,
            demo: String::new(),
            solde: String::new(),
            pnl: (String::new(), crate::jetons::TEXTE),
            gardien: None,
            wallet: "non connecté".into(),
        }
    }
}

/// Le menu ARGOS épuré de la maquette (tel-13) : la carte du compte, puis l'essentiel ; le reste dans ⏳ Bientôt.
const ESSENTIEL: [(TopAct, &str, &str); 10] = [
    (TopAct::Journal, "notebook", "Journal"),
    (TopAct::Portfolio, "briefcase", "Portfolio"),
    (TopAct::Backtest, "rewind", "Backtest"),
    (TopAct::Marches, "globe", "Marchés"),
    (TopAct::MyAccount, "user-circle", "Compte"),
    (TopAct::Academy, "graduation-cap", "Académie"),
    (TopAct::Marketplace, "shopping-cart", "Marketplace"),
    (TopAct::Hub, "gear", "Réglages"),
    // (maquette : ⌨ Raccourcis du clavier, Ctrl /, avant ⏳ Bientôt)
    (TopAct::Raccourcis, "⌨️", "Raccourcis du clavier"),
    (TopAct::Bientot, "clock", "Bientôt"),
];

/// Menu ARGOS ▾ : la maquette (carte du compte, l'essentiel, ⏳ Bientôt) ; ☰ Appli dans l'en-tête.
#[derive(Debug, Clone, PartialEq)]
pub struct ArgosMenu {
    pub rect: Rect,
    /// ☰ Appli (mises à jour, installation, affichage) au lieu des sections d'Argos.
    pub app: bool,
    /// ⏳ La liste de ce qui arrive bientôt (au lieu de l'essentiel).
    pub bientot: bool,
    /// Hauteur de tout le contenu, et défilement (le menu plus haut que l'écran défile, au doigt comme à la molette).
    pub full: f64,
    pub scroll: f64,
    /// 📱 Cell : la feuille du bas de la maquette (toute la largeur, collée en bas, en verre).
    pub feuille: bool,
}

// 📱 Feuille ARGOS du cell, mesurée dans la maquette (390 × 844, #menua) : feuille de 548,8 de haut, coins du haut de
// 26, marge 8 ; carte du compte à 9 du bord, 188 de haut, rayon 16, marge 12 ; puis 10 lignes de 33,55 (marge 8 / 10,
// rayon 10), la première 6 sous la carte. 📱 Sans « ⌨ Raccourcis du clavier » (pas de clavier, Jee 20 nov.) : une ligne
// de moins.
const F_H: f64 = 548.8 - F_LIGNE;
const F_RAYON: f64 = 26.0;
const F_BORD: f64 = 9.0;
const F_CARTE: f64 = 188.1;
const F_LIGNE: f64 = 33.55;
const F_TETE: f64 = 46.0;

/// Les lignes de la feuille du cell, dans l'ordre de la maquette (📱 sans « ⌨ Raccourcis du clavier » : pas de clavier).
const LIGNES_CELL: [(TopAct, &str, &str); 9] = [
    (TopAct::Journal, "📓", "Journal"),
    (TopAct::Portfolio, "💼", "Portfolio"),
    (TopAct::Backtest, "⏪", "Backtest"),
    (TopAct::Marches, "🌍", "Marchés"),
    (TopAct::MyAccount, "👤", "Compte"),
    (TopAct::Academy, "🎓", "Académie"),
    (TopAct::Marketplace, "🛒", "Marketplace"),
    (TopAct::Hub, "⚙️", "Réglages"),
    (TopAct::Bientot, "⏳", "Bientôt"),
];

/// 📱 Rows that only make sense with a keyboard and a PC window (Jee, 20 Nov), and (Jee, 21 Nov, list 2) ⏳ « Bientôt »
/// (tools that do nothing yet) and the « Compte » row (the account card above already opens it): left out in phone
/// view (phone, or tablet used by touch).
fn pc_seulement(a: TopAct) -> bool {
    crate::actions::tactile()
        && matches!(
            a,
            TopAct::Raccourcis
                | TopAct::ZoomOut
                | TopAct::ZoomIn
                | TopAct::ZoomReset
                | TopAct::Fullscreen
                | TopAct::ScreenTest
                | TopAct::Bientot
                | TopAct::MyAccount
        )
}

/// The essential rows of ARGOS ▾ (📱 phone view: without « ⌨ Raccourcis du clavier »).
fn essentiel() -> Vec<&'static (TopAct, &'static str, &'static str)> {
    ESSENTIEL.iter().filter(|e| !pc_seulement(e.0)).collect()
}

const AW: f64 = 440.0;
const APAD: f64 = 8.0;
const AHEAD: f64 = 52.0;
const ATITLE: f64 = 26.0;
const AROW: f64 = 42.0;
/// Carte du compte et lignes de la liste (menu ARGOS de la maquette).
const CARTE_H: f64 = 178.0;
const LIGNE_H_PC: f64 = 38.0;

/// Height of a row of the list (📱 phone view: 46, a row of 44 for the finger, Jee 21 Nov, list 2).
fn ligne_h() -> f64 {
    if crate::actions::tactile() {
        46.0
    } else {
        LIGNE_H_PC
    }
}

/// ⏳ Le reste du menu complet (ni dans l'essentiel, ni dans la carte du compte), dans l'ordre des anciennes sections :
/// ce qui est prêt s'ouvre, le reste est marqué « bientôt ». Les 27 lignes de l'ancien menu restent ainsi toutes
/// accessibles.
fn reste() -> Vec<&'static Tile> {
    const AILLEURS: [TopAct; 4] = [TopAct::MyAccount, TopAct::DemoAccount, TopAct::Wallet, TopAct::Gardien];
    ARGOS
        .iter()
        .flat_map(|(_, r)| r.iter())
        .filter(|t| !ESSENTIEL.iter().any(|e| e.0 == t.0) && !AILLEURS.contains(&t.0))
        .collect()
}

impl ArgosMenu {
    /// Sous le logo (`anchor`), dans la fenêtre (`win_w` × `win_h`).
    pub fn new(anchor: Rect, win_w: f64, win_h: f64) -> Self {
        Self::of(false, anchor, win_w, win_h)
    }

    /// ☰ Appli, au même endroit.
    pub fn app(anchor: Rect, win_w: f64, win_h: f64) -> Self {
        Self::of(true, anchor, win_w, win_h)
    }

    /// 📱 Cell : la feuille du bas de la maquette, sur toute la largeur de l'écran (`win_w` × `win_h`, barre du bas
    /// comprise).
    pub fn feuille(win_w: f64, win_h: f64) -> Self {
        let mut m = Self {
            rect: Rect::new(0.0, 0.0, win_w, 0.0),
            app: false,
            bientot: false,
            full: 0.0,
            scroll: 0.0,
            feuille: true,
        };
        m.placer(win_h);
        m
    }

    /// 📱 The cell sheet opened on its ⏳ « Bientôt » list (the « Bientôt » row of the pill's « Argos » sheet).
    pub fn feuille_bientot(win_w: f64, win_h: f64) -> Self {
        let mut m = Self::feuille(win_w, win_h);
        m.bientot = true;
        m.placer(win_h);
        m
    }

    /// ⏳ How many things are coming soon (the « Bientôt » list).
    pub fn bientot_n() -> usize {
        reste().len()
    }

    /// Feuille du cell : hauteur et place (collée en bas), après un changement de contenu.
    fn placer(&mut self, win_h: f64) {
        self.full = self.hauteur();
        let vis = self.full.min((win_h - 40.0).max(120.0));
        self.rect = Rect::new(0.0, win_h - vis, self.rect.w, vis);
        self.scroll = 0.0;
    }

    fn of(app: bool, anchor: Rect, win_w: f64, win_h: f64) -> Self {
        let mut m = Self {
            rect: Rect::new(0.0, 0.0, AW.min(win_w - 16.0), 0.0),
            app,
            bientot: false,
            full: 0.0,
            scroll: 0.0,
            feuille: false,
        };
        m.full = m.hauteur();
        let x = anchor.x.clamp(8.0, (win_w - AW - 8.0).max(8.0));
        let y = (anchor.y + anchor.h + 6.0).min((win_h - m.full - 6.0).max(6.0));
        // plus haut que l'écran (téléphone) : on montre ce qui tient, le reste défile
        let vis = m.full.min((win_h - y - 6.0).max(120.0));
        m.rect = Rect::new(x, y, m.rect.w, vis);
        m
    }

    /// Hauteur de tout le contenu.
    fn hauteur(&self) -> f64 {
        if self.feuille {
            return if self.bientot {
                // (en bas : la place du bouton rond Argos AI, posé par-dessus, pour que la dernière ligne reste visible)
                F_TETE + reste().len() as f64 * F_LIGNE + 70.0
            } else {
                F_H
            };
        }
        if self.app {
            AHEAD + self.secs().iter().map(|(_, r)| ATITLE + r.len().div_ceil(2) as f64 * AROW).sum::<f64>() + APAD
        } else if self.bientot {
            AHEAD + reste().len() as f64 * ligne_h() + APAD
        } else {
            AHEAD + CARTE_H + 8.0 + essentiel().len() as f64 * ligne_h() + APAD
        }
    }

    /// Défile de `dy` points (positif : vers le bas du menu).
    pub fn wheel(&mut self, dy: f64) {
        self.scroll = (self.scroll + dy).clamp(0.0, (self.full - self.rect.h).max(0.0));
    }

    /// ☰ Appli's sections (📱 phone view: without zoom, full screen and screen test, Jee 20 Nov).
    fn secs(&self) -> Vec<(&'static str, Vec<&'static Tile>)> {
        APP_MENU.iter().map(|(t, r)| (*t, r.iter().filter(|t| !pc_seulement(t.0)).collect())).collect()
    }

    /// Zones de la carte du compte (coordonnées de la fenêtre) : Se connecter, compte démo, 🛡 Gardien, wallet,
    /// Compte complet.
    fn zones_carte(&self) -> Vec<(Rect, TopAct)> {
        let r = self.rect;
        let c = Rect::new(r.x + APAD, r.y - self.scroll + AHEAD, r.w - 2.0 * APAD, CARTE_H);
        vec![
            (Rect::new(c.x + c.w - 112.0, c.y + 8.0, 104.0, 30.0), TopAct::MyAccount),
            (Rect::new(c.x, c.y + 44.0, c.w, 24.0), TopAct::DemoAccount),
            (Rect::new(c.x + c.w - 120.0, c.y + 92.0, 112.0, 24.0), TopAct::Gardien),
            (Rect::new(c.x, c.y + 120.0, c.w, 22.0), TopAct::Wallet),
            (Rect::new(c.x, c.y + 144.0, c.w, 30.0), TopAct::MyAccount),
        ]
    }

    /// Bouton ☰ Appli (en-tête d'Argos ▾) ou ‹ Argos (en-tête de ☰ Appli), coordonnées de la fenêtre. 📱 Feuille du
    /// cell : seulement « ‹ Argos » en haut à gauche de la liste ⏳ (la feuille principale n'a pas d'en-tête, comme la
    /// maquette).
    pub fn switch_rect(&self) -> Rect {
        let r = self.rect;
        if self.feuille {
            return if self.bientot {
                Rect::new(r.x + F_BORD, r.y + 8.0 - self.scroll, 92.0, 30.0)
            } else {
                Rect::new(r.x, r.y, 0.0, 0.0)
            };
        }
        Rect::new(r.x + r.w - APAD - 96.0, r.y + 10.0 - self.scroll, 92.0, 32.0)
    }

    /// 📱 La carte du compte de la feuille du cell (coordonnées de la fenêtre).
    fn carte_feuille(&self) -> Rect {
        let r = self.rect;
        Rect::new(r.x + F_BORD, r.y - self.scroll + F_BORD, r.w - 2.0 * F_BORD, F_CARTE)
    }

    /// 📱 Zones de la feuille du cell : la carte du compte (Se connecter, compte démo, 🛡 PropFirm, wallet, Compte
    /// complet), puis les 10 lignes ; ou la liste ⏳.
    fn zones_feuille(&self) -> Vec<(Rect, TopAct)> {
        let r = self.rect;
        let ligne = |k: usize, y0: f64| Rect::new(r.x + F_BORD, y0 + k as f64 * F_LIGNE, r.w - 2.0 * F_BORD, F_LIGNE);
        if self.bientot {
            let y0 = r.y - self.scroll + F_TETE;
            return reste().into_iter().enumerate().map(|(k, t)| (ligne(k, y0), t.0)).collect();
        }
        let c = self.carte_feuille();
        let (x, w) = (c.x + 12.0, c.w - 24.0);
        let mut out = vec![
            (Rect::new(c.x + c.w - 12.0 - 99.0, c.y + 12.0, 99.0, 27.5), TopAct::MyAccount),
            (Rect::new(x, c.y + 39.5, w, 22.2), TopAct::DemoAccount),
            (Rect::new(x, c.y + 61.7, w, 22.2), TopAct::DemoAccount),
            (Rect::new(x, c.y + 83.9, w, 28.8), TopAct::Gardien),
            (Rect::new(x, c.y + 112.8, w, 22.2), TopAct::Wallet),
            (Rect::new(x, c.y + 135.0, w, 41.1), TopAct::MyAccount),
        ];
        let y0 = c.y + c.h + 6.0;
        out.extend(LIGNES_CELL.iter().enumerate().map(|(k, l)| (ligne(k, y0), l.0)));
        out
    }

    /// Les zones touchables (coordonnées de la fenêtre) et leur action.
    pub fn zones(&self) -> Vec<(Rect, TopAct)> {
        if self.feuille {
            return self.zones_feuille();
        }
        let r = self.rect;
        let ligne = |k: usize, y0: f64| Rect::new(r.x + APAD, y0 + k as f64 * ligne_h(), r.w - 2.0 * APAD, ligne_h() - 2.0);
        if !self.app && self.bientot {
            let y0 = r.y - self.scroll + AHEAD;
            return reste().into_iter().enumerate().map(|(k, t)| (ligne(k, y0), t.0)).collect();
        }
        if !self.app {
            let y0 = r.y - self.scroll + AHEAD + CARTE_H + 8.0;
            let mut out = self.zones_carte();
            out.extend(essentiel().into_iter().enumerate().map(|(k, e)| (ligne(k, y0), e.0)));
            return out;
        }
        let cw = (r.w - 2.0 * APAD) / 2.0;
        let mut y = r.y - self.scroll + AHEAD;
        let mut out = vec![];
        for (_, rows) in self.secs() {
            y += ATITLE;
            for (i, row) in rows.iter().enumerate() {
                out.push((
                    Rect::new(r.x + APAD + (i % 2) as f64 * cw, y + (i / 2) as f64 * AROW, cw - 2.0, AROW - 2.0),
                    row.0,
                ));
            }
            y += rows.len().div_ceil(2) as f64 * AROW;
        }
        out
    }

    /// Clic (coordonnées de la fenêtre de l'app) ; ⏳ Bientôt ouvre sa liste dans le menu, ‹ revient à l'essentiel.
    pub fn click(&mut self, x: f64, y: f64) -> ArgosClick {
        if !self.rect.contains(x, y) {
            return ArgosClick::Close;
        }
        // 📱 feuille du cell : elle reste collée en bas quand sa hauteur change (⏳ ↔ ‹ Argos)
        let bas = self.rect.y + self.rect.h;
        if self.switch_rect().contains(x, y) {
            if self.bientot {
                self.bientot = false;
                (self.full, self.scroll) = (self.hauteur(), 0.0);
                if self.feuille {
                    self.placer(bas);
                }
                return ArgosClick::None;
            }
            return ArgosClick::Switch;
        }
        match self.zones().into_iter().rev().find(|(q, _)| q.contains(x, y)).map(|(_, a)| a) {
            Some(TopAct::Bientot) => {
                self.bientot = true;
                (self.full, self.scroll) = (self.hauteur(), 0.0);
                if self.feuille {
                    self.placer(bas);
                }
                ArgosClick::None
            }
            Some(a) => ArgosClick::Act(a),
            None => ArgosClick::None,
        }
    }

    /// Image du menu ; `compte` : la carte du compte (menu ARGOS) ; `open` : ce qui est déjà ouvert (coché ✓).
    pub fn draw(&self, text: &Text, compte: &CompteCarte, mouse: (f64, f64), open: &dyn Fn(TopAct) -> bool) -> Option<(Pixmap, Rect)> {
        if self.feuille {
            return self.dessin_feuille(text, compte, mouse);
        }
        use crate::palette::{GREY, ICON, MUTED, WHITE};
        use crate::phosphor;
        // tout le contenu (hauteur `full`), puis la partie visible selon le défilement
        let r = Rect::new(self.rect.x, self.rect.y - self.scroll, self.rect.w, self.full);
        let mut pm = crate::px::new(r.w as u32, r.h as u32)?;
        let loc = |q: Rect| Rect::new(q.x - r.x, q.y - r.y, q.w, q.h);
        crate::shapes::menu_card(&mut pm, loc(r));
        let (titre, sous) = if self.app {
            ("Appli", "mises à jour, installation, affichage")
        } else if self.bientot {
            ("Bientôt", "ce qui arrive dans Argos")
        } else {
            ("Argos", "l'essentiel")
        };
        text.draw(&mut pm, titre, ((APAD + 8.0) as f32, 24.0), 15.0, WHITE, Align::Left);
        text.draw(&mut pm, sous, ((APAD + 8.0) as f32, 41.0), 12.0, GREY, Align::Left);
        // ☰ Appli (‹ Argos pour revenir) : bouton de l'en-tête, au doigt aussi (32 px)
        let sw = loc(self.switch_rect());
        crate::shapes::menu_row(&mut pm, sw, false, self.switch_rect().contains(mouse.0, mouse.1));
        crate::shapes::round_stroke(&mut pm, sw, sw.h / 2.0, pal::edge(), 1.0);
        let bouton = if self.app || self.bientot { "‹ Argos" } else { "☰ Appli" };
        text.draw(&mut pm, bouton, (sw.cx() as f32, (sw.cy() + 5.0) as f32), 13.5, WHITE, Align::Center);
        // (sous l'en-tête : un espace, sans trait, comme la maquette)
        if !self.app {
            self.dessin_argos(&mut pm, text, compte, mouse, r);
        } else {
            let mut y = AHEAD;
            for (title, rows) in self.secs() {
                text.draw(&mut pm, title, ((APAD + 8.0) as f32, (y + 17.0) as f32), 10.5, GREY, Align::Left);
                y += ATITLE + rows.len().div_ceil(2) as f64 * AROW;
            }
            let tiles: Vec<&Tile> = self.secs().into_iter().flat_map(|(_, r)| r).collect();
            for (q, a) in self.zones() {
                let Some((_, icon, name, sub, ready)) = tiles.iter().find(|t| t.0 == a) else {
                    continue;
                };
                let lq = loc(q);
                crate::shapes::menu_row(&mut pm, lq, false, q.contains(mouse.0, mouse.1));
                phosphor::draw(&mut pm, icon, lq.x + 20.0, lq.cy(), 22, if *ready { ICON } else { MUTED });
                // à droite : ✓ si déjà ouvert, « bientôt » si pas encore prêt
                let right = lq.x + lq.w - 6.0;
                let badge = if open(a) {
                    phosphor::check(&mut pm, right - 8.0, lq.y + 13.0, 16.0);
                    22.0
                } else if !ready {
                    phosphor::soon(&mut pm, text, right, lq.y + 13.0);
                    text.width("bientôt", 9.5) as f64 + 16.0
                } else {
                    0.0
                };
                text.draw(
                    &mut pm,
                    &text.fit(name, 13.5, lq.w - 48.0 - badge),
                    ((lq.x + 42.0) as f32, (lq.y + 17.0) as f32),
                    13.5,
                    if *ready { WHITE } else { GREY },
                    Align::Left,
                );
                text.draw(
                    &mut pm,
                    &text.fit(&crate::actions::sans_raccourci(sub), 11.0, lq.w - 48.0),
                    ((lq.x + 42.0) as f32, (lq.y + 32.0) as f32),
                    11.0,
                    GREY,
                    Align::Left,
                );
            }
        }
        let v = self.rect;
        if self.full > v.h {
            // barre de défilement fine à droite
            let k = v.h / self.full;
            crate::jetons::barre_defilement(&mut pm, v.w, self.scroll + self.scroll * k + 4.0, (v.h * k - 8.0).max(16.0), false);
            let part = crate::px::int_rect(0.0, self.scroll, v.w, v.h).and_then(|ir| pm.clone_rect(ir))?;
            return Some((part, v));
        }
        Some((pm, v))
    }

    /// 📱 La feuille ARGOS du cell (maquette #menua, mesurée) : la carte du compte, puis les 10 lignes ; ou la liste ⏳
    /// avec « ‹ Argos ». En verre (posée par l'app comme les autres feuilles), coins du haut de 26.
    fn dessin_feuille(&self, text: &Text, compte: &CompteCarte, mouse: (f64, f64)) -> Option<(Pixmap, Rect)> {
        use crate::jetons as j;
        use crate::shapes::{round_fill, round_stroke};
        let r = Rect::new(self.rect.x, self.rect.y - self.scroll, self.rect.w, self.full);
        let mut pm = crate::px::new(r.w as u32, r.h.ceil() as u32)?;
        let loc = |q: Rect| Rect::new(q.x - r.x, q.y - r.y, q.w, q.h);
        let survol = |q: &Rect| q.contains(mouse.0, mouse.1);
        // la feuille : verre, coins du haut de 26 (le bas va jusqu'au bord de l'écran), liseré
        round_fill(&mut pm, Rect::new(0.0, 0.0, r.w, r.h + F_RAYON), F_RAYON, j::VERRE.0, j::VERRE.1);
        round_stroke(&mut pm, Rect::new(0.5, 0.5, r.w - 1.0, r.h + F_RAYON), F_RAYON, j::sur_fond(j::LISERE), 1.0);
        let gras = Text::graisse(crate::text::Graisse::Bold);
        // une ligne : survol, nom (13), à droite « › » ou ce qu'on donne
        let ligne = |pm: &mut Pixmap, q: Rect, nom: &str, pret: bool| {
            let lq = loc(q);
            if survol(&q) {
                round_fill(pm, lq, j::pilule(lq), j::VERRE2.0, j::VERRE2.1);
            }
            let c = if pret { j::TEXTE } else { j::DIM };
            text.draw(
                pm,
                &text.fit(nom, 13.0, lq.w - 90.0),
                ((lq.x + 10.0) as f32, (lq.cy() + 4.5) as f32),
                13.0,
                c,
                Align::Left,
            );
            lq
        };
        let droite = |pm: &mut Pixmap, lq: Rect, s: &str| {
            text.draw(pm, s, ((lq.x + lq.w - 10.0) as f32, (lq.cy() + 4.5) as f32), 13.0, j::DIM, Align::Right);
        };
        if self.bientot {
            let sw = loc(self.switch_rect());
            round_fill(&mut pm, sw, 12.0, j::BLANC, if survol(&self.switch_rect()) { 0.14 } else { 0.07 });
            text.draw(&mut pm, "‹ Argos", (sw.cx() as f32, (sw.cy() + 4.5) as f32), 13.0, j::TEXTE, Align::Center);
            gras.draw(
                &mut pm,
                "⏳ Bientôt et le reste",
                ((sw.x + sw.w + 12.0) as f32, (sw.cy() + 5.0) as f32),
                14.0,
                j::TEXTE,
                Align::Left,
            );
            for (q, a) in self.zones() {
                let Some(t) = reste().into_iter().find(|t| t.0 == a) else { continue };
                let lq = ligne(&mut pm, q, t.2, t.4);
                droite(&mut pm, lq, if t.4 { "›" } else { "bientôt" });
            }
        } else {
            // la carte du compte (rayon 16, marge 12)
            let cr = self.carte_feuille();
            let c = loc(cr);
            round_fill(&mut pm, c, j::R_TUILE, j::VERRE2.0, 0.09);
            round_stroke(&mut pm, c, j::R_TUILE, j::sur_fond(j::LISERE), 1.0);
            let qui = compte.compte.as_deref().map_or("Aucun compte connecté".to_string(), |n| n.to_string());
            gras.draw(
                &mut pm,
                &text.fit(&format!("👤 {qui}"), 13.0, c.w - 140.0),
                ((c.x + 12.0) as f32, (c.y + 30.0) as f32),
                13.0,
                j::TEXTE,
                Align::Left,
            );
            let z = self.zones_feuille();
            if compte.compte.is_none() {
                let b = loc(z[0].0);
                round_fill(&mut pm, b, 12.0, j::BLANC, if survol(&z[0].0) { 0.14 } else { 0.07 });
                text.draw(&mut pm, "Se connecter", (b.cx() as f32, (b.cy() + 4.5) as f32), 13.0, j::TEXTE, Align::Center);
            }
            // les 4 lignes (12) : à gauche le nom, à droite la valeur
            let rang = |pm: &mut Pixmap, q: Rect, g: &str| {
                let lq = loc(q);
                text.draw(pm, g, (lq.x as f32, (lq.cy() + 4.0) as f32), 12.0, j::TEXTE, Align::Left);
                lq
            };
            let lq = rang(&mut pm, z[1].0, "Compte démo actif");
            gras.draw(
                &mut pm,
                &compte.demo,
                ((lq.x + lq.w) as f32, (lq.cy() + 4.0) as f32),
                12.0,
                j::TEXTE,
                Align::Right,
            );
            let lq = rang(&mut pm, z[2].0, "Solde · P&L du jour");
            let pw = text.width(&compte.pnl.0, 12.0) as f64;
            text.draw(
                &mut pm,
                &compte.pnl.0,
                ((lq.x + lq.w) as f32, (lq.cy() + 4.0) as f32),
                12.0,
                compte.pnl.1,
                Align::Right,
            );
            text.draw(
                &mut pm,
                &format!("{} · ", compte.solde),
                ((lq.x + lq.w - pw) as f32, (lq.cy() + 4.0) as f32),
                12.0,
                j::TEXTE,
                Align::Right,
            );
            let lq = rang(&mut pm, z[3].0, "PropFirm");
            let (gt, gc) = compte.gardien.clone().unwrap_or_else(|| ("Gardien éteint".into(), j::DIM));
            let gt = format!("🛡 {gt}");
            let bw = gras.width(&gt, 11.0) as f64 + 18.0;
            let badge = Rect::new(lq.x + lq.w - bw, lq.cy() - 11.4, bw, 22.8);
            round_fill(&mut pm, badge, 10.0, gc, 0.25);
            gras.draw(&mut pm, &gt, (badge.cx() as f32, (badge.cy() + 4.0) as f32), 11.0, gc, Align::Center);
            let lq = rang(&mut pm, z[4].0, "Wallet");
            text.draw(
                &mut pm,
                &compte.wallet,
                ((lq.x + lq.w) as f32, (lq.cy() + 4.0) as f32),
                12.0,
                j::DIM,
                Align::Right,
            );
            // « Compte complet (…) › » sur deux lignes
            let cq = loc(z[5].0);
            if survol(&z[5].0) {
                round_fill(&mut pm, cq, j::pilule(cq), j::VERRE2.0, j::VERRE2.1);
            }
            let lignes = crate::assistant::wrap(text, "Compte complet (mon compte, clés API, wallet, connecteur AI)", 13.0, cq.w - 24.0);
            for (k, s) in lignes.iter().take(2).enumerate() {
                text.draw(&mut pm, s, (cq.x as f32, (cq.y + 18.0 + k as f64 * 17.55) as f32), 13.0, j::TEXTE, Align::Left);
            }
            text.draw(&mut pm, "›", ((cq.x + cq.w) as f32, (cq.cy() + 5.0) as f32), 13.0, j::TEXTE, Align::Right);
            // les 10 lignes
            let n = reste().len();
            for (q, a) in z.into_iter().skip(6) {
                let Some(l) = LIGNES_CELL.iter().find(|l| l.0 == a) else { continue };
                let nom = format!("{} {}", l.1, l.2);
                match a {
                    TopAct::Bientot => {
                        let lq = ligne(&mut pm, q, &nom, false);
                        droite(&mut pm, lq, &format!("{n} ›"));
                    }
                    TopAct::Raccourcis => {
                        let lq = ligne(&mut pm, q, &nom, true);
                        // touches Ctrl et / (11,5, rayon 6, fond blanc 10 %)
                        let mut x = lq.x + lq.w - 10.0;
                        for k in ["/", "Ctrl"] {
                            let kw = text.width(k, 11.5) as f64 + 12.0;
                            x -= kw;
                            let kr = Rect::new(x, lq.cy() - 9.0, kw.max(20.9), 18.0);
                            round_fill(&mut pm, kr, kr.h / 2.0, j::BLANC, 0.1);
                            round_stroke(&mut pm, kr, kr.h / 2.0, j::sur_fond(j::LISERE), 1.0);
                            text.draw(&mut pm, k, (kr.cx() as f32, (kr.cy() + 4.0) as f32), 11.5, j::TEXTE, Align::Center);
                            x -= 4.0;
                        }
                    }
                    _ => {
                        let lq = ligne(&mut pm, q, &nom, true);
                        droite(&mut pm, lq, "›");
                    }
                }
            }
        }
        let v = self.rect;
        if self.full > v.h {
            let part = crate::px::int_rect(0.0, self.scroll, v.w, v.h).and_then(|ir| pm.clone_rect(ir))?;
            return Some((part, v));
        }
        Some((pm, v))
    }

    /// Le menu ARGOS de la maquette : la carte du compte puis l'essentiel (ou la liste ⏳ Bientôt). `r` : le contenu
    /// entier (coordonnées de la fenêtre).
    fn dessin_argos(&self, pm: &mut Pixmap, text: &Text, compte: &CompteCarte, mouse: (f64, f64), r: Rect) {
        use crate::jetons as j;
        let loc = |q: Rect| Rect::new(q.x - r.x, q.y - r.y, q.w, q.h);
        let survol = |q: &Rect| q.contains(mouse.0, mouse.1);
        let ligne = |pm: &mut Pixmap, q: Rect, icone: &str, nom: &str, droite: &str, survole: bool, pret: bool| {
            let lq = loc(q);
            if survole {
                crate::shapes::round_fill(pm, lq, j::pilule(lq), j::VERRE2.0, j::VERRE2.1);
            }
            // (pas d'icône Phosphor, comme ⌨️ : l'émoji)
            if !crate::phosphor::draw(
                pm,
                icone,
                lq.x + 18.0,
                lq.cy(),
                18,
                if pret { crate::palette::ICON } else { crate::palette::MUTED },
            ) {
                text.draw(pm, icone, ((lq.x + 18.0) as f32, (lq.cy() + 5.0) as f32), 14.0, j::TEXTE, Align::Center);
            }
            text.draw(
                pm,
                nom,
                ((lq.x + 38.0) as f32, (lq.cy() + 5.0) as f32),
                14.0,
                if pret { j::TEXTE } else { j::DIM },
                Align::Left,
            );
            text.draw(pm, droite, ((lq.x + lq.w - 10.0) as f32, (lq.cy() + 5.0) as f32), 13.0, j::DIM, Align::Right);
        };
        if self.bientot {
            for (q, a) in self.zones() {
                let Some(t) = reste().into_iter().find(|t| t.0 == a) else { continue };
                ligne(pm, q, t.1, t.2, if t.4 { "›" } else { "bientôt" }, survol(&q), t.4);
            }
            return;
        }
        // la carte du compte (verre, liseré)
        let c = loc(Rect::new(r.x + APAD, r.y + AHEAD, r.w - 2.0 * APAD, CARTE_H));
        crate::shapes::round_fill(pm, c, j::R_TUILE, j::VERRE2.0, 0.09);
        crate::shapes::round_stroke(pm, c, j::R_TUILE, j::sur_fond(j::LISERE), 1.0);
        let gras = Text::graisse(crate::text::Graisse::Bold);
        let qui = compte.compte.as_deref().map_or("Aucun compte connecté".to_string(), |n| n.to_string());
        crate::phosphor::draw(pm, "user-circle", c.x + 20.0, c.y + 23.0, 16, j::TEXTE);
        gras.draw(
            pm,
            &text.fit(&qui, 14.0, c.w - 160.0),
            ((c.x + 34.0) as f32, (c.y + 28.0) as f32),
            14.0,
            j::TEXTE,
            Align::Left,
        );
        let zc = self.zones_carte();
        if compte.compte.is_none() {
            let b = loc(zc[0].0);
            crate::shapes::round_fill(pm, b, b.h / 2.0, j::VERRE2.0, if survol(&zc[0].0) { 0.16 } else { 0.1 });
            text.draw(pm, "Se connecter", (b.cx() as f32, (b.cy() + 4.5) as f32), 13.0, j::TEXTE, Align::Center);
        }
        let rang = |pm: &mut Pixmap, y: f64, g: &str, d: &str, cd: Rgb| {
            text.draw(pm, g, ((c.x + 12.0) as f32, (c.y + y) as f32), 13.0, j::DIM, Align::Left);
            text.draw(pm, d, ((c.x + c.w - 12.0) as f32, (c.y + y) as f32), 13.0, cd, Align::Right);
        };
        rang(pm, 61.0, "Compte démo actif", &compte.demo, j::TEXTE);
        // solde · P&L du jour (le P&L à sa couleur)
        text.draw(pm, "Solde · P&L du jour", ((c.x + 12.0) as f32, (c.y + 85.0) as f32), 13.0, j::DIM, Align::Left);
        let pw = text.width(&compte.pnl.0, 13.0) as f64;
        text.draw(
            pm,
            &compte.pnl.0,
            ((c.x + c.w - 12.0) as f32, (c.y + 85.0) as f32),
            13.0,
            compte.pnl.1,
            Align::Right,
        );
        text.draw(
            pm,
            &format!("{} ·", compte.solde),
            ((c.x + c.w - 16.0 - pw) as f32, (c.y + 85.0) as f32),
            13.0,
            j::TEXTE,
            Align::Right,
        );
        // 🛡 PropFirm : la pilule du Gardien
        text.draw(pm, "PropFirm", ((c.x + 12.0) as f32, (c.y + 109.0) as f32), 13.0, j::DIM, Align::Left);
        let (gt, gc) = compte.gardien.clone().unwrap_or_else(|| ("Gardien éteint".into(), j::DIM));
        let gw = text.width(&gt, 12.0) as f64 + 34.0;
        let pil = Rect::new(c.x + c.w - 12.0 - gw, c.y + 94.0, gw, 22.0);
        crate::shapes::round_fill(pm, pil, 11.0, gc, 0.2);
        crate::phosphor::draw(pm, "shield-check", pil.x + 13.0, pil.cy(), 12, gc);
        text.draw(pm, &gt, ((pil.x + 24.0) as f32, (pil.cy() + 4.0) as f32), 12.0, gc, Align::Left);
        rang(pm, 133.0, "Wallet", &compte.wallet, j::DIM);
        let cc = loc(zc[4].0);
        if survol(&zc[4].0) {
            crate::shapes::round_fill(pm, cc, j::pilule(cc), j::VERRE2.0, j::VERRE2.1);
        }
        text.draw(
            pm,
            &text.fit("Compte complet (mon compte, clés API, wallet, connecteur AI)", 13.0, cc.w - 40.0),
            ((cc.x + 12.0) as f32, (cc.cy() + 4.5) as f32),
            13.0,
            j::TEXTE,
            Align::Left,
        );
        text.draw(pm, "›", ((cc.x + cc.w - 12.0) as f32, (cc.cy() + 5.0) as f32), 15.0, j::DIM, Align::Right);
        // l'essentiel
        let n = reste().len();
        let y0 = r.y + AHEAD + CARTE_H + 8.0;
        for (k, e) in essentiel().into_iter().enumerate() {
            let q = Rect::new(r.x + APAD, y0 + k as f64 * ligne_h(), r.w - 2.0 * APAD, ligne_h() - 2.0);
            let droite = match e.0 {
                TopAct::Bientot => format!("{n} ›"),
                TopAct::Raccourcis => "Ctrl /".into(),
                _ => "›".into(),
            };
            ligne(pm, q, e.1, e.2, &droite, survol(&q), true);
        }
    }
}

/// Menu des mises en page : tes mises en page (nom, symbole · unité de temps, celle ouverte cochée), puis les actions et les modèles.
pub fn layouts_menu(liste: &[(String, String, bool)], auto_save: bool) -> Vec<MenuGroup> {
    let mut g = vec![MenuGroup {
        title: "MES MISES EN PAGE".into(),
        items: liste
            .iter()
            .enumerate()
            .map(|(k, (nom, hint, ouverte))| MenuItem {
                id: layout_id(k),
                label: nom.clone(),
                hint: hint.clone(),
                checked: *ouverte,
                enabled: true,
                icon: Some(if *ouverte { "🥇" } else { "🗂" }),
                ..Default::default()
            })
            .collect(),
        strip: false,
    }];
    g.extend(rows(&LAYOUTS, auto_save));
    g
}

/// 🗂 Id d'une mise en page de la liste (négatif : jamais celui d'une action ou d'une langue).
pub fn layout_id(k: usize) -> i64 {
    -1 - k as i64
}

/// La mise en page d'un id du menu (`layout_id`), s'il en est une.
pub fn layout_of(id: i64) -> Option<usize> {
    (id < 0).then(|| (-1 - id) as usize)
}

/// Id de « 🔎 Chercher une langue… » dans le menu 🌐.
pub const LANG_SEARCH: i64 = 190;
/// Nombre de langues de la liste du menu (les autres du catalogue : par la recherche, ou une fois téléchargées).
const MENU_LANGS: usize = 22;

/// 🌐 Langue et format des nombres : formats (id 100 + k), langues (id 200 + k ; `lang` cochée), la recherche,
/// Réinitialiser. Une langue pas encore téléchargée (ni dans le code, ni dans `installed`) a son nom plus foncé :
/// un clic dessus demande si on la télécharge.
pub fn lang_menu(format: usize, lang: usize, installed: &[usize]) -> Vec<MenuGroup> {
    use crate::i18n::{done, CATALOG};
    let formats = crate::viewport::NUMBER_FORMATS
        .iter()
        .enumerate()
        .map(|(k, (ex, _, _))| MenuItem {
            id: 100 + k as i64,
            label: ex.to_string(),
            checked: k == format,
            enabled: true,
            ..Default::default()
        })
        .collect();
    let mut langs: Vec<MenuItem> = (0..CATALOG.len())
        .filter(|&k| k < MENU_LANGS || k == lang || installed.contains(&k))
        .map(|k| {
            let ready = done(k) || installed.contains(&k);
            MenuItem {
                id: 200 + k as i64,
                label: CATALOG[k].1.to_string(),
                checked: k == lang,
                enabled: true,
                dim: !ready,
                ..Default::default()
            }
        })
        .collect();
    langs.push(MenuItem {
        id: LANG_SEARCH,
        label: "Chercher une langue…".into(),
        icon: Some("🔎"),
        enabled: true,
        ..Default::default()
    });
    vec![
        MenuGroup {
            title: "FORMAT DES NOMBRES".into(),
            items: formats,
            strip: false,
        },
        MenuGroup {
            title: "LANGUE".into(),
            items: langs,
            strip: false,
        },
        MenuGroup {
            title: String::new(),
            items: vec![MenuItem {
                id: TopAct::Reset as i64,
                label: "Réinitialiser".into(),
                icon: Some("↩️"),
                enabled: true,
                ..Default::default()
            }],
            strip: false,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menus_du_haut() {
        let mut m = ArgosMenu::new(Rect::new(10.0, 10.0, 100.0, 36.0), 1400.0, 900.0);
        assert!(m.rect.y + m.rect.h <= 900.0 && m.rect.w == 440.0);
        let (q, _) = m.zones().into_iter().find(|(_, a)| *a == TopAct::Journal).unwrap();
        assert_eq!(m.click(q.cx(), q.cy()), ArgosClick::Act(TopAct::Journal));
        assert_eq!(m.click(1300.0, 800.0), ArgosClick::Close);
        assert!(m.draw(&Text::default(), &CompteCarte::default(), (0.0, 0.0), &|a| a == TopAct::Trade).is_some());
        // chaque action une seule fois
        let all: Vec<TopAct> = ARGOS.iter().flat_map(|(_, r)| r.iter().map(|x| x.0)).collect();
        assert!(all.iter().all(|a| all.iter().filter(|b| *b == a).count() == 1));
        let l = layouts_menu(&[("Argos".into(), "BTC · 30m".into(), true), ("Or".into(), "XAU · 4h".into(), false)], true);
        assert!(l[1].items.iter().any(|i| i.id == TopAct::AutoSave as i64 && i.checked));
        assert!(l[0].items[0].checked && !l[0].items[1].checked && l[0].items[1].label == "Or");
        assert_eq!(layout_of(l[0].items[1].id), Some(1));
        assert_eq!(TopAct::from_id(l[0].items[1].id), None, "jamais pris pour une action");
        assert_eq!(layout_of(TopAct::Save as i64), None);
        assert_eq!(TopAct::NewLayout.info(), ("Nouvelle mise en page", true));
        assert_eq!(TopAct::from_id(TopAct::Rename as i64), Some(TopAct::Rename));
        assert_eq!(TopAct::Save.info(), ("Sauvegarder", true));
        let g = lang_menu(1, 5, &[9]);
        // pas de ⬇️ : une langue pas encore téléchargée a son nom plus foncé (toujours cliquable)
        assert!(g[0].items[1].checked && !g[1].items[0].dim && g[1].items[2].dim && g[1].items[2].hint.is_empty());
        assert!(g[1].items[5].checked && g[1].items[5].label == "Español");
        assert!(!g[1].items[9].dim, "Deutsch déjà téléchargé");
        assert_eq!(g[1].items.last().map(|i| i.id), Some(LANG_SEARCH));
        // une langue hors de la liste, choisie : elle s'ajoute au menu
        assert!(lang_menu(0, 23, &[]).iter().any(|g| g.items.iter().any(|i| i.label == "Polski" && i.checked)));
        assert_eq!(crate::i18n::find("allemand"), Some(9));
        assert_eq!(crate::i18n::find("polonais"), Some(23));
        assert_eq!(crate::i18n::find("Portugues"), Some(8));
        assert_eq!(crate::i18n::find("klingon"), None);
    }

    /// Jee (2026-11-08) : plus de « Toutes mes mises en page (Maj + T) » dans le menu des mises en page.
    #[test]
    fn plus_de_toutes_mes_mises_en_page() {
        let g = layouts_menu(&[("Argos".into(), "BTC · 4h".into(), true)], false);
        assert!(g
            .iter()
            .flat_map(|g| &g.items)
            .all(|i| !i.label.contains("Toutes mes mises en page") && i.hint != "Maj + T"));
    }

    /// Menu ARGOS de la maquette (tel-13, Jee 2026-11-09) : la carte du compte (Se connecter, démo, 🛡, wallet,
    /// Compte complet), l'essentiel (9 lignes), ⏳ Bientôt qui ouvre la liste de ce qui arrive, ‹ pour revenir.
    #[test]
    fn menu_argos_epure() {
        let mut m = ArgosMenu::new(Rect::new(10.0, 10.0, 100.0, 36.0), 1400.0, 900.0);
        let z = m.zones();
        let actes: Vec<TopAct> = z.iter().map(|(_, a)| *a).collect();
        for a in [
            TopAct::Journal,
            TopAct::Portfolio,
            TopAct::Backtest,
            TopAct::Marches,
            TopAct::Academy,
            TopAct::Marketplace,
            TopAct::Hub,
            TopAct::Gardien,
            TopAct::DemoAccount,
            TopAct::Wallet,
        ] {
            assert!(actes.contains(&a), "{a:?}");
        }
        assert!(!actes.contains(&TopAct::Mining), "les « bientôt » ne sont plus dans la liste");
        let (b, _) = *z.iter().find(|(_, a)| *a == TopAct::Bientot).unwrap();
        assert_eq!(m.click(b.cx(), b.cy()), ArgosClick::None);
        assert!(m.bientot && m.zones().iter().any(|(_, a)| *a == TopAct::Mining) && m.zones().iter().any(|(_, a)| *a == TopAct::News && a.info().1));
        let s = m.switch_rect();
        assert_eq!(m.click(s.cx(), s.cy()), ArgosClick::None, "‹ revient à l'essentiel");
        assert!(!m.bientot);
        let c = CompteCarte {
            compte: Some("@jee".into()),
            demo: "Argos 100K".into(),
            solde: "100 412 $".into(),
            pnl: ("+142 $".into(), crate::palette::up()),
            gardien: Some(("0,8 % / 5 %".into(), crate::palette::up())),
            wallet: "non connecté".into(),
        };
        let (pm, _) = m.draw(&Text::default(), &c, (0.0, 0.0), &|_| false).unwrap();
        if let Ok(dir) = std::env::var("ARGOS_CAPTURES") {
            pm.save_png(format!("{dir}/app-menu-argos.png")).unwrap();
        }
    }

    /// 📱 Feuille ARGOS du cell, aux mesures de la maquette (390 × 844, #menua) : feuille, carte, lignes ; chaque zone
    /// agit ; ⏳ ouvre le reste (les 27 lignes de l'ancien menu restent accessibles) et « ‹ Argos » revient.
    #[test]
    fn feuille_argos_du_cell() {
        let mut m = ArgosMenu::feuille(390.0, 844.0);
        // (📱 the mock-up's 548.8 minus the « ⌨ Raccourcis du clavier » row: no keyboard on the phone, Jee 20 Nov)
        let fh = 548.8 - F_LIGNE;
        assert_eq!(m.rect, Rect::new(0.0, 844.0 - fh, 390.0, fh), "feuille du bas, toute la largeur");
        let c = m.carte_feuille();
        assert_eq!((c.x, c.y.round(), c.w, c.h), (9.0, 338.0, 372.0, 188.1), "carte du compte");
        let z = m.zones();
        let premiere = z.iter().find(|(_, a)| *a == TopAct::Journal).unwrap().0;
        assert_eq!(
            (premiere.x, premiere.y.round(), premiere.w, premiere.h),
            (9.0, 532.0, 372.0, 33.55),
            "1re ligne"
        );
        let se_connecter = z[0].0;
        assert_eq!((se_connecter.x, se_connecter.w, se_connecter.h), (270.0, 99.0, 27.5));
        // chaque zone agit (dernière ligne : ⏳ Bientôt)
        for (q, a) in m.zones() {
            let mut n = m.clone();
            match n.click(q.cx(), q.cy()) {
                ArgosClick::Act(b) => assert_eq!(b, a),
                ArgosClick::None => assert_eq!(a, TopAct::Bientot),
                autre => panic!("{a:?} : {autre:?}"),
            }
        }
        let actes: Vec<TopAct> = z.iter().map(|(_, a)| *a).collect();
        assert!(!actes.contains(&TopAct::Raccourcis), "no keyboard on the phone");
        assert!(actes.contains(&TopAct::Gardien) && actes.contains(&TopAct::Wallet));
        // ⏳ : le reste, collé en bas ; le nombre affiché = ses lignes
        let (b, _) = *z.iter().find(|(_, a)| *a == TopAct::Bientot).unwrap();
        assert_eq!(m.click(b.cx(), b.cy()), ArgosClick::None);
        assert!(m.bientot && (m.rect.y + m.rect.h - 844.0).abs() < 0.01, "toujours collée en bas");
        let reste: Vec<TopAct> = m.zones().iter().map(|(_, a)| *a).collect();
        assert_eq!(reste.len(), super::reste().len());
        // les 27 lignes de l'ancien menu complet : dans la feuille ou dans ⏳
        for t in ARGOS.iter().flat_map(|(_, r)| r.iter()) {
            assert!(
                actes.contains(&t.0) || reste.contains(&t.0) || t.0 == TopAct::DemoAccount,
                "{:?} accessible",
                t.0
            );
        }
        for (q, a) in m.zones() {
            assert_eq!(m.clone().click(q.cx(), q.cy()), ArgosClick::Act(a));
        }
        let s = m.switch_rect();
        assert_eq!(m.click(s.cx(), s.cy()), ArgosClick::None, "‹ Argos revient");
        assert!(!m.bientot && m.rect.h == fh);
        assert!(m.draw(&Text::default(), &CompteCarte::default(), (0.0, 0.0), &|_| false).is_some());
        m.bientot = true;
        assert!(m.draw(&Text::default(), &CompteCarte::default(), (0.0, 0.0), &|_| false).is_some());
    }

    /// 🧭 Every place of the Argos ▾ menu (PC, ☰ Appli, the phone's sheet, the account card's Gardien pill) gives an
    /// act the app knows back from its id: Marchés, Marketplace and Gardien did nothing (/app2/, 22 Nov).
    #[test]
    fn chaque_acte_du_menu_est_connu() {
        let menus = [
            ArgosMenu::new(Rect::new(10.0, 10.0, 100.0, 40.0), 1440.0, 900.0),
            ArgosMenu::app(Rect::new(10.0, 10.0, 100.0, 40.0), 1440.0, 900.0),
            ArgosMenu::feuille(390.0, 844.0),
        ];
        for m in menus {
            for (_, a) in m.zones() {
                assert_eq!(TopAct::from_id(a as i64), Some(a), "{a:?}");
            }
        }
        for a in [TopAct::Marches, TopAct::Marketplace, TopAct::Gardien] {
            assert_eq!(TopAct::from_id(a as i64), Some(a), "{a:?}");
        }
    }

    /// ⌨ Menu ARGOS PC comme la maquette : « Raccourcis du clavier » (Ctrl /) dans l'essentiel, avant ⏳ Bientôt (et plus
    /// dans la liste Bientôt).
    #[test]
    fn raccourcis_dans_l_essentiel() {
        let k = super::ESSENTIEL
            .iter()
            .position(|e| e.0 == super::TopAct::Raccourcis)
            .expect("dans l'essentiel");
        assert_eq!(super::ESSENTIEL[k + 1].0, super::TopAct::Bientot);
        assert!(!super::reste().iter().any(|t| t.0 == super::TopAct::Raccourcis), "plus dans Bientôt");
    }

    /// 📱 Jee (20 Nov, « ok liste »): on the phone and the touch tablet, no keyboard shortcuts (⌨ row, « Alt+H »,
    /// « Ctrl + S »), no zoom −/+/100 %, no full screen, no screen test; the PC keeps all of them.
    #[test]
    fn cell_sans_ce_qui_est_pour_le_pc() {
        let pc_seul = [TopAct::ZoomOut, TopAct::ZoomIn, TopAct::ZoomReset, TopAct::Fullscreen, TopAct::ScreenTest];
        let actes = |m: &ArgosMenu| m.zones().into_iter().map(|(_, a)| a).collect::<Vec<_>>();
        let ctrl_s = || {
            layouts_menu(&[], false)
                .iter()
                .flat_map(|g| g.items.clone())
                .find(|i| i.id == TopAct::Save as i64)
                .unwrap()
                .hint
        };
        let accueil = |m: &ArgosMenu| {
            m.secs()
                .into_iter()
                .flat_map(|(_, r)| r)
                .find(|t| t.0 == TopAct::Accueil)
                .map(|t| crate::actions::sans_raccourci(t.3))
                .unwrap()
        };
        for tactile in [false, true] {
            crate::actions::set_tactile(tactile);
            let argos = ArgosMenu::new(Rect::new(10.0, 10.0, 100.0, 36.0), 1400.0, 900.0);
            let app = ArgosMenu::app(Rect::new(10.0, 10.0, 100.0, 36.0), 1400.0, 900.0);
            assert_eq!(actes(&argos).contains(&TopAct::Raccourcis), !tactile, "⌨ row, tactile {tactile}");
            for a in pc_seul {
                assert_eq!(actes(&app).contains(&a), !tactile, "{a:?}, tactile {tactile}");
            }
            // the rest of ☰ Appli stays (💻 Version ordi too: the welcome page's button, Jee 25 Nov)
            for a in [
                TopAct::Updates,
                TopAct::Install,
                TopAct::Desktop,
                TopAct::ClearCache,
                TopAct::About,
                TopAct::Accueil,
                TopAct::Langue,
            ] {
                assert!(actes(&app).contains(&a), "{a:?}");
            }
            assert_eq!(accueil(&app).contains("Alt+H"), !tactile);
            assert!(accueil(&app).starts_with("tes cartes et mises en page"));
            assert_eq!(ctrl_s(), if tactile { "" } else { "Ctrl + S" });
            // every row still acts and the menu draws
            for (q, a) in app.zones() {
                assert_eq!(app.clone().click(q.cx(), q.cy()), ArgosClick::Act(a));
            }
            assert!(app.draw(&Text::default(), &CompteCarte::default(), (0.0, 0.0), &|_| false).is_some());
            assert!(argos.draw(&Text::default(), &CompteCarte::default(), (0.0, 0.0), &|_| false).is_some());
        }
        crate::actions::set_tactile(false);
    }

    /// 📱 Jee (21 Nov, list 2): ARGOS ▾ by the finger has rows of 44 at least, one « Compte » only (the account card)
    /// and no ⏳ « Bientôt »; the PC keeps its rows of 38, the « Compte » row and « Bientôt ».
    #[test]
    fn argos_au_doigt_lignes_de_44() {
        for tactile in [false, true] {
            crate::actions::set_tactile(tactile);
            let m = ArgosMenu::new(Rect::new(10.0, 10.0, 100.0, 36.0), 820.0, 1180.0);
            let n = m.zones_carte().len();
            let lignes: Vec<(Rect, TopAct)> = m.zones().into_iter().skip(n).collect();
            assert_eq!(lignes.iter().any(|(_, a)| *a == TopAct::Bientot), !tactile, "⏳, tactile {tactile}");
            assert_eq!(lignes.iter().any(|(_, a)| *a == TopAct::MyAccount), !tactile, "Compte, tactile {tactile}");
            assert!(lignes.iter().any(|(_, a)| *a == TopAct::Journal));
            assert!(lignes.iter().all(|(q, _)| (q.h >= 44.0) == tactile), "tactile {tactile}");
            assert!(m.full <= 1180.0, "{}", m.full);
        }
        crate::actions::set_tactile(false);
    }
}
