//! 🚩 Watchlist (comme TradingView), à droite de la chart : une barre d'icônes (pour l'instant : le bouton watchlist)
//! et le panneau qu'elle ouvre. Colonnes Symbole · Dernier · Chg · Chg% (clic sur l'en-tête = trier ↑ / ↓ / sans tri),
//! sections repliables (Crypto, HIP-3), drapeaux, chiffres qui viennent de changer en vert / rouge, symbole choisi
//! encadré. Clic sur une ligne = ouvrir ce symbole ; ↑ / ↓ = symbole précédent / suivant dans l'ordre affiché.
//! Partie pure (état, clics, ordre : testée) + dessin dans sa propre image, collée à droite de la chart.

use crate::palette as pal;
use crate::palette::{GREY, ICON, MUTED, TEXT, WHITE};
use crate::panels::{Kind, PanelClick, Panels};
use crate::shapes::{lines, poly_fill, round_fill, round_stroke, Rect};
use crate::text::{Align, Graisse, Rgb, Text};
use crate::viewport::format_price;
use crate::{fill, Theme};
use serde::{Deserialize, Serialize};
use tiny_skia::Pixmap;

/// Largeur de la barre d'icônes (toujours là).
pub const ICON_BAR: u32 = 56;
/// Largeur du panneau ouvert, par défaut ; on la change en glissant son bord gauche (entre `MIN_W` et `MAX_W`).
pub const PANEL: u32 = 420;
/// 🖥 Place gardée à droite de l'en-tête d'un volet pour ⬓ ⤢ ✕ (les boutons de l'en-tête sont à leur gauche).
/// 🖥 Le module de la colonne de modules qui montre la même chose qu'un volet (⬓ ancrer) ; `None` : pas d'équivalent.
pub fn module_du_volet(t: Tool) -> Option<crate::dock::Module> {
    use crate::dock::Module as M;
    Some(match t {
        Tool::Watchlist => M::Watchlist,
        Tool::Agenda => M::Agenda,
        Tool::News => M::News,
        Tool::Whales => M::Whales,
        Tool::Explorer => M::Explorer,
        Tool::Stats => M::Stats,
        Tool::Market => M::Market,
        Tool::Trading | Tool::Demo => M::Positions,
        _ => return None,
    })
}

pub const FERME_PLACE: f64 = 102.0;
pub const MIN_W: u32 = 300;
pub const MAX_W: u32 = 640;
const TOP: f64 = 44.0;
const HEAD_H: f64 = 30.0;
const ROW_H: f64 = 36.0;
/// Rangée de la watchlist compacte (module de la colonne).
const MINI_ROW: f64 = 26.0;
const SEC_H: f64 = 30.0;
const FOOT_H: f64 = 48.0;
/// 📱 Pied plus haut sur cell : la fiche du symbole sous son nom.
const FICHE_H: f64 = 96.0;
/// Bord droit des colonnes Dernier, Chg, Chg%.
/// Distance du bord droit du panneau aux colonnes Dernier, Chg, Chg% (elles suivent la largeur).
const COLS_FROM_RIGHT: [f64; 3] = [140.0, 74.0, 12.0];

/// Colonne de tri.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Col {
    Symbol,
    Last,
    Chg,
    Pct,
}

impl Col {
    fn name(self) -> &'static str {
        match self {
            Col::Symbol => "Symbole",
            Col::Last => "Dernier",
            Col::Chg => "Chg",
            Col::Pct => "Chg%",
        }
    }
}

/// 🔔 Le panneau enregistré : `Alertes` devient `Watchlist` (les anciennes versions ne lisent pas `Alertes`).
fn panel_enregistre<S: serde::Serializer>(t: &Tool, s: S) -> Result<S::Ok, S::Error> {
    let t = if matches!(t, Tool::Alertes | Tool::Support | Tool::Explorer | Tool::Stats | Tool::Strategie) {
        Tool::Watchlist
    } else {
        *t
    };
    serde::Serialize::serialize(&t, s)
}

mod outils;
pub use outils::*;

const DOT: f64 = 24.0;
/// Barre d'icônes : pas entre deux boutons, et l'espace d'un trait de séparation (maquette).
const PAS_OUTIL: f64 = 42.0;
/// 🖥 Colonne PC de la maquette : 44 de large à 6 du bord de `ICON_BAR`, 10 du haut et du bas de la fenêtre, boutons
/// de 36 au pas de 40 ; › et ⚙ en bas (83 et 43 au-dessus du bas de la colonne).
const COL_X: f64 = 6.0;
const COL_W: f64 = 44.0;
const COL_MARGE: f64 = 10.0;
const COL_PAS: f64 = COL_BTN + 4.0;
const COL_BTN: f64 = crate::formfactor::BOUTON_PC;
/// 🖥 Place de la colonne repliée à droite de la chart (maquette `colrep` : --col 22).
const COL_REPLIEE: f64 = 22.0;

/// 🖥 La bande d'une barre repliée (maquette : rail de gauche, colonne de droite repliée) : verre de 12 de large, coins
/// ronds, et la poignée au milieu (trait de 4 × 40, couleur du liseré). Sert aussi au rail (bureau).
pub fn dessiner_bande(pm: &mut Pixmap, w: f64, h: f64, th: &Theme) {
    pilule_bande(pm, w, h, th);
    // (bande couchée, comme celle des rangées du haut : la poignée aussi)
    let p = if w > h {
        Rect::new((w - 40.0) / 2.0, (h - 4.0) / 2.0, 40.0, 4.0)
    } else {
        Rect::new((w - 4.0) / 2.0, (h - 40.0) / 2.0, 4.0, 40.0)
    };
    round_fill(pm, p, 2.0, crate::jetons::sur_fond(crate::jetons::LISERE), 1.0);
}

/// 🫧 The band's pill (folded bar) on the whole image `w` × `h`: round ends (half the short side), the glass fill
/// (almost opaque without the glass style) and the light outline. 🖥 The right column opened (unfolded, or shown on
/// hover over the chart) keeps exactly this shape (Jee, 19 Nov: « quand on ouvre, garder la même forme »).
pub fn pilule_bande(pm: &mut Pixmap, w: f64, h: f64, th: &Theme) {
    let c = Rect::new(0.5, 0.5, w - 1.0, h - 1.0);
    let rayon = w.min(h) / 2.0;
    // (no dark fill under it, glass style or not: Jee's permanent rule, 20 Nov: whatever lies on the chart is its own
    // glass pill; « sans cadre » never means « sans fond », and never a dark rectangle)
    let _ = th;
    crate::jetons::verre(pm, c, rayon);
}

/// Un bouton de la barre d'icônes : fond « choisi », puis l'icône dessinée petite et agrandie (1,3 ×) — plus facile à
/// voir et à viser.
fn icone_outil(pm: &mut Pixmap, t: Tool, ir: Rect, on: bool) {
    if on {
        round_fill(pm, ir, crate::jetons::pilule(ir), crate::jetons::PUCE_ON.0, crate::jetons::PUCE_ON.1);
    }
    let c = if on { WHITE } else { t.couleur() };
    if let Some(mut ic) = crate::px::new(30, 30) {
        t.icon(&mut ic, 15.0, 15.0, c);
        let k = 1.3 * (ir.w / 38.0).min(1.0);
        let tr = tiny_skia::Transform::from_scale(k as f32, k as f32)
            .post_translate(crate::px::dev(ir.cx() - 15.0 * k) as f32, crate::px::dev(ir.cy() - 15.0 * k) as f32);
        pm.draw_pixmap(
            0,
            0,
            ic.as_ref(),
            &tiny_skia::PixmapPaint {
                quality: tiny_skia::FilterQuality::Bicubic,
                ..Default::default()
            },
            tr,
            None,
        );
    }
}
const TRAIT_OUTIL: f64 = 10.0;

/// Une ligne : un marché.
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    /// Nom pour l'API (« BTC », « xyz:NVDA »).
    pub coin: String,
    /// Nom affiché (« BTC-USDC »).
    pub label: String,
    /// En petit et en gris après le nom, s'il reste de la place (ex. l'exchange : « Binance »).
    pub note: String,
    /// Prix tel que l'exchange l'écrit.
    pub mark: String,
    /// Prix précédent (pour colorer les chiffres qui ont changé) et sens du dernier changement.
    last_mark: String,
    up: bool,
    /// Prix d'il y a 24 h.
    pub prev: f64,
}

impl Row {
    pub fn new(coin: &str, label: &str, mark: &str, prev: f64) -> Self {
        Self {
            coin: coin.into(),
            label: label.into(),
            note: String::new(),
            mark: mark.into(),
            last_mark: mark.into(),
            up: true,
            prev,
        }
    }
    fn price(&self) -> f64 {
        self.mark.parse().unwrap_or(0.0)
    }
    /// 📈 Its change over 24 h, in price and in %; `None` without a price or without the price 24 h ago
    /// (0: an exchange that does not give it, as Kraken): the row then shows « — », never a figure made up.
    fn change(&self) -> Option<(f64, f64)> {
        let price = self.price();
        (!self.mark.is_empty() && self.prev > 0.0 && price > 0.0).then(|| (price - self.prev, (price / self.prev - 1.0) * 100.0))
    }
    fn decimals(&self) -> usize {
        self.mark.split_once('.').map_or(0, |(_, d)| d.len())
    }
    /// 💲 The price as the watchlist writes it (« 61 373,0 »: the chosen number format, the exchange's decimals);
    /// empty while no price came.
    pub fn prix_affiche(&self) -> String {
        if self.mark.is_empty() {
            String::new()
        } else {
            format_price(self.price(), self.decimals())
        }
    }
    /// 📈 Its 24 h change in % (`None` without a price or the price 24 h ago).
    pub fn variation(&self) -> Option<f64> {
        self.change().map(|(_, pct)| pct)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Section {
    pub name: String,
    pub rows: Vec<Row>,
}

/// Résultat d'un clic.
#[derive(Debug, Clone, PartialEq)]
pub enum Click {
    /// Rien d'important (ou à côté).
    None,
    /// 🖥 ⤢ : volet large ⇄ normal.
    AgrandirVolet,
    /// 🖥 ⬓ : le volet devient ce module de la colonne de modules.
    AncrerVolet(crate::dock::Module),
    /// 🖥 › en bas de la colonne : la barre d'icônes se replie (comme la languette ›).
    ReplierBarre,
    /// 🖥 ⚙ en bas de la colonne : le menu « Personnaliser la barre » (ordre, masquer, afficher), sous ce bouton.
    PersonnaliserBarre(Rect),
    /// 📱 Une pilule de liste (id du menu des listes).
    Liste(i64),
    /// 📱 La mini-pilule du symbole ouvert : la feuille se referme, la chart revient.
    FermerFeuille,
    /// 📱 Le nom d'une section (liste à toi) : son menu (renommer, monter, descendre, supprimer), sous ce nom.
    SectionMenu(String, Rect),
    /// Un bouton de la barre qui ouvre autre chose qu'un volet (dessin, Gardien, Script…), sous ce bouton
    /// (coordonnées de l'image de la watchlist).
    Outil(Tool, Rect),
    /// 📈 Indicateur natif à activer / couper (panneau Indicateurs).
    Indicator(crate::indicators::Ind),
    /// ⚙️ Réglages d'un indicateur (panneau Indicateurs).
    IndSettings(crate::indicators::Ind),
    /// 🛒 Ouvrir le Marketplace Argos.
    Marketplace,
    /// 🚀 Top gagnants / perdants 24 h.
    Movers,
    /// 🎓 Académie Argos.
    Academy,
    /// 📖 Académie, fiche news, à cette ligne du tableau (depuis l'Agenda).
    AcademyNews(usize),
    /// 🧭 Explorer Hyperliquid: read an address, follow it, its lines on the chart, a market's chart, a tab.
    Explorer(crate::hlexplorer::ExClick),
    /// 📜 Ouvrir l'éditeur Argos Script.
    Script(crate::panels::ScriptOpen),
    /// 📓 Journal : nouveau trade, modifier, export, import.
    Journal(crate::journal::JClick),
    /// Quelque chose a changé dans la watchlist (tri, section, drapeau, panneau ouvert / fermé).
    Changed,
    /// Ouvrir ce symbole.
    Open(String),
    /// Nom de la liste (en-tête) : ouvrir le menu des listes, sous ce bouton (coordonnées de l'image).
    Lists(Rect),
    /// ＋ : ouvrir l'ajout de symboles, sous ce bouton.
    Add(Rect),
    /// ✕ d'une ligne : retirer ce symbole de la liste.
    Remove(String),
    /// ✕ d'une section : la retirer.
    RemoveSection(String),
    /// Bouton AI (logo A d'Argos).
    Ai,
    /// 📰 Ouvrir cet article dans le navigateur.
    OpenUrl(String),
    /// 💼 Ticket changé : la position dessinée suit.
    Ticket,
    /// 💼 Passer l'ordre démo.
    Order,
    /// 💼 Fermer les positions démo ouvertes.
    CloseAll,
    /// 🛡️ Réglage du gardien changé (sauver journal.json).
    GuardCfg,
    /// 🔓 Déverrouiller la journée du gardien.
    Unlock,
    /// 💼 Annuler un ordre en attente.
    CancelOrder(u64),
    /// 💼 Menu des comptes démo.
    Accounts,
    /// 📱 🧪 Simulateur (feuille Trading du cell).
    Simulateur,
    /// 🖥 🎫 Ticket while the Simulator shows in the Trading volet: the Simulator closes, the ticket comes back.
    FermerSimulateur,
    /// Fonction pas encore là : afficher ce message.
    Soon(String),
    /// Panneau Exchanges : interrupteur, ⟳, clé, oublier la clé.
    Exchange(crate::exchanges::ExAction),
    /// 🧩 Réglages des modules changés (la fenêtre les garde et refait la mise en page).
    Dock,
    /// 🔑 Ajouter la clé CoinMarketCap (ligne du panneau Marché, sans clé).
    CleCmc,
    /// 🌐 « Tous les marchés » : la pilule Plateforme ▾ (son menu, sous elle).
    TousBourse(Rect),
    /// 🌐 « Tous les marchés » : la pilule Trier par ▾ (son menu, sous elle).
    TousTri(Rect),
    /// 🌐 « Tous les marchés » : ☆ d'une ligne, ajouter ce symbole à « Mes favoris » (★ : il y est déjà).
    Favori(String),
    /// 🎫 VENDRE (`false`) / ACHETER (`true`) of the module 31 ticket: the demo order leaves.
    Passer(bool),
    /// 🎫 A field of the ticket to type (its box opens).
    Saisir(crate::panels::ticket::Champ),
    /// ⌖ The next touch on the chart gives the order's price.
    Cible,
    /// 📖 « Carnet › » of the ticket: the order book module of the column.
    Carnet,
    /// 🎚 The ticket's % slider pressed: it follows the mouse / the finger until let go ([`Watchlist::glisse_ticket`]).
    Curseur,
    /// 🎯 The Stratégie volet asks the app (the full report, TradingView's file, the settings, another strategy).
    Strategie(crate::strategie::StratClick),
    /// 📖 The order-book volet asks the app (a menu, a tab, the DOM's centre); `Rect`s in the window's volet.
    CarnetPanneau(crate::carnet::CarnetClick),
}

/// Ligne affichée.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Line {
    Section(usize),
    Row(usize, usize),
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Watchlist {
    pub prefs: Prefs,
    pub sections: Vec<Section>,
    /// Symbole ouvert sur la chart.
    pub selected: String,
    scroll: f64,
    /// Défilement des panneaux (Trading démo, Indicateurs, Journal, Marché crypto, Exchanges).
    panel_scroll: f64,
    /// Où la molette veut aller (liste, panneau) : le défilement y glisse image par image (`glide`).
    scroll_to: f64,
    panel_to: f64,
    /// Barre de défilement attrapée : distance entre la souris et le haut du curseur.
    bar_grab: Option<f64>,
    /// Choix de couleur du drapeau ouvert pour ce symbole.
    picker: Option<String>,
    /// Logos des symboles (CoinMarketCap) ; sans logo : pastille de couleur.
    pub logos: std::collections::HashMap<String, Pixmap>,
    /// Nom de la liste affichée (en-tête) et ce qu'il y a à côté en gris (« auto », nombre de symboles).
    pub title: String,
    pub note: String,
    /// Liste de l'utilisateur : ✕ au survol pour retirer un symbole ou une section.
    pub editable: bool,
    /// 📱 Les listes en pilules sous l'en-tête du cell : (id du menu des listes, nom) et l'id de la liste ouverte.
    pub listes: Vec<(i64, String)>,
    pub liste_active: i64,
    /// Panneaux Indicateurs, Trading démo, Journal.
    pub panels: Panels,
    /// Lignes du panneau Exchanges (remplies par la fenêtre).
    pub exchanges: Vec<crate::exchanges::ExRow>,
    /// Panneau Marché crypto (rempli par la fenêtre).
    pub market: argos_data::market::MarketInfo,
    /// 📱 Sur cell : toute la largeur de l'écran quand le panneau est ouvert, rien quand il est fermé (sans barre
    /// d'icônes : on passe d'un panneau à l'autre par Panneaux, comme v1).
    pub phone: Option<u32>,
    /// 📐 Double écran côte à côte : le panneau prend toute la largeur du segment de droite (ouvert, sans le
    /// redimensionner).
    pub fixed_w: Option<u32>,
    /// 🖥 Colonne d'icônes de la maquette PC (haut → bas de la fenêtre) : de combien elle monte au-dessus de l'image
    /// (la barre du haut s'arrête avant elle). `None` : la barre d'icônes de toujours, à côté de la chart.
    pub colonne: Option<f64>,
    /// 🖥 Hauteur de l'image de la watchlist quand la colonne est là (pour placer › et ⚙ en bas).
    pub col_h: f64,
    /// 🖥 Outils ⭐ de la colonne PC d'interface-2, dans l'ordre (gardés dans interface-2.json) ; `None` : la barre
    /// d'avant (ordre et masqués de watchlist.json).
    pub col_outils: Option<Vec<Tool>>,
    /// 🖥 Colonne repliée en bande (› en bas ; gardé dans interface-2.json).
    pub col_repliee: bool,
    /// 🖥 Souris sur la bande de la colonne repliée (elle s'élargit par-dessus la chart).
    pub col_survol: bool,
    /// 📱 Fiche du symbole ouvert (prix, variation, volume, OI, funding), en bas de la watchlist du cell.
    pub fiche: Vec<crate::topbar::Stat>,
    /// 📅 Agenda et 📰 Éco / News : données et filtres ; `now` : l'heure (secondes Unix), donnée par la fenêtre.
    pub eco: crate::eco::Eco,
    pub now: i64,
    /// 🐋 Baleines : transactions et filtres.
    pub whales: crate::whales::Whales,
    /// 🧭 Explorer Hyperliquid and 📶 Stats Hyperliquid.
    pub explorer: crate::hlexplorer::Explorer,
    pub stats: crate::hlstats::Stats,
    /// 🧩 Modules : réglages de la colonne ; `dock_big` : la fenêtre est assez large pour l'afficher.
    pub dock: crate::dock::DockPrefs,
    pub dock_big: bool,
    /// 🖥 PC : le volet flotte sous la 1re rangée (dessiné à part, `draw_volet`) ; la watchlist ne le peint plus.
    pub volet_flottant: bool,
    /// 🖥 ⤢ Volet large : sa largeur (donnée par la fenêtre) ; `None` : la largeur normale.
    pub large_w: Option<u32>,
    /// 🌐 « Tous les marchés » : les symboles déjà dans « Mes favoris » (★ au lieu de ☆).
    pub favoris: std::collections::HashSet<String>,
    /// 🎯 Stratégie: the result of the strategy on the chart (its 🎯 is in the bar while there is one).
    pub strategie: crate::strategie::VoletStrategie,
    /// 📖 What the order-book volet shows (built by the app from the book read).
    pub carnet: crate::carnet::CarnetVue,
}

/// 📱 Hauteur de la bande de la poignée, en haut de la feuille du cell (l'en-tête et les panneaux commencent dessous).
pub const POIGNEE_CELL: f64 = 14.0;

impl Watchlist {
    /// Hauteur du pied (📱 cell : avec la fiche du symbole).
    fn foot_h(&self) -> f64 {
        if self.phone.is_some() {
            // 📱 la mini-pilule du symbole ouvert (photos de Jee) et sa marge
            crate::ecrans::boite_de("wl-mini").map_or(FOOT_H + FICHE_H, |b| b[3] + 16.0)
        } else {
            FOOT_H
        }
    }

    /// Largeur à droite de la chart.
    pub fn width(&self) -> u32 {
        match self.phone {
            Some(w) => {
                if self.prefs.open {
                    w
                } else {
                    0
                }
            }
            None => match self.fixed_w {
                Some(w) => ICON_BAR + w,
                // ‹ › barre repliée et panneau fermé : la chart va jusqu'au bord
                // 🖥 interface-2 : la colonne repliée garde sa bande (prefs.folded est celui de l'ancienne /app/)
                None if self.col_outils.is_some() && self.col_repliee && !self.prefs.open => COL_REPLIEE as u32,
                None if self.col_outils.is_some() && !self.prefs.open => ICON_BAR,
                None if self.prefs.folded && !self.prefs.open => 0,
                None => ICON_BAR + if self.prefs.open { self.pw() as u32 } else { 0 },
            },
        }
    }

    /// Colonne sous `x` dans l'en-tête.
    fn header_col(&self, x: f64) -> Col {
        let c = self.cols();
        if x < c[0] - 80.0 {
            Col::Symbol
        } else if x < c[0] + 4.0 {
            Col::Last
        } else if x < c[1] + 4.0 && c[1] > c[0] {
            Col::Chg
        } else {
            Col::Pct
        }
    }

    /// Largeur du panneau ouvert.
    pub fn pw(&self) -> f64 {
        match self.phone {
            Some(w) => w as f64,
            None => match (self.fixed_w, self.large_w) {
                (Some(w), _) => w as f64,
                // 🖥 ⤢ volet large (min(920, 62 % de la fenêtre), donné par la fenêtre)
                (None, Some(w)) => w as f64,
                (None, None) => self.prefs.width.clamp(MIN_W, MAX_W) as f64,
            },
        }
    }

    /// Bords droits des colonnes Dernier, Chg, Chg%. Quand la place manque pour les noms entiers, Chg se masque (comme
    /// TradingView) : le prix prend sa place, Dernier et Chg% restent toujours.
    fn cols(&self) -> [f64; 3] {
        let c = COLS_FROM_RIGHT.map(|d| self.pw() - d);
        if self.sans_chg() {
            [c[1], c[1], c[2]]
        } else {
            c
        }
    }

    /// Chg masquée : avec elle, le plus long nom de la liste ne tiendrait pas entier devant son prix (PC).
    fn sans_chg(&self) -> bool {
        if self.phone.is_some() {
            return false;
        }
        let t = Text::default();
        let place = |r: &Row| {
            let prix = if r.mark.is_empty() {
                14.0
            } else {
                t.width(&format_price(r.price(), r.decimals()), 13.5) as f64
            };
            self.pw() - COLS_FROM_RIGHT[0] - prix - 10.0 - 48.0
        };
        self.sections.iter().flat_map(|s| &s.rows).any(|r| t.width(&r.label, 13.5) as f64 > place(r))
    }

    /// Bord gauche du panneau (on l'attrape pour changer la largeur) ; `None` panneau fermé.
    pub fn edge_rect(&self, h: f64) -> Option<Rect> {
        (self.prefs.open && self.phone.is_none()).then(|| Rect::new(0.0, 0.0, 6.0, h))
    }

    /// Nouvelle largeur (glisser le bord gauche), gardée entre `MIN_W` et `MAX_W`.
    pub fn set_width(&mut self, w: f64) {
        self.prefs.width = (w.round().max(0.0) as u32).clamp(MIN_W, MAX_W);
    }

    /// Nouvelles données : garde le prix précédent de chaque marché pour colorer ce qui change.
    pub fn update(&mut self, sections: Vec<Section>) {
        let old: std::collections::HashMap<String, Row> = self.sections.drain(..).flat_map(|s| s.rows).map(|r| (r.coin.clone(), r)).collect();
        self.sections = sections
            .into_iter()
            .map(|mut s| {
                for r in &mut s.rows {
                    if let Some(o) = old.get(&r.coin) {
                        if o.mark != r.mark {
                            r.last_mark = o.mark.clone();
                            r.up = r.price() >= o.price();
                        } else {
                            r.last_mark = o.last_mark.clone();
                            r.up = o.up;
                        }
                    }
                }
                s
            })
            .collect();
    }

    /// Ordre des lignes d'une section (selon le tri).
    fn sorted(&self, s: usize) -> Vec<usize> {
        let rows = &self.sections[s].rows;
        let mut idx: Vec<usize> = (0..rows.len()).collect();
        // (🌐 « Tous les marchés » : l'ordre de « Trier par », déjà fait par la fenêtre)
        if let Some((col, asc)) = self.prefs.sort.filter(|_| !self.prefs.tous) {
            idx.sort_by(|&a, &b| {
                let (a, b) = (&rows[a], &rows[b]);
                // 📈 a change not known (« — ») goes last, whichever the direction
                if matches!(col, Col::Chg | Col::Pct) {
                    match (a.change(), b.change()) {
                        (Some(_), None) => return std::cmp::Ordering::Less,
                        (None, Some(_)) => return std::cmp::Ordering::Greater,
                        (None, None) => return std::cmp::Ordering::Equal,
                        _ => {}
                    }
                }
                let (ca, cb) = (a.change().unwrap_or_default(), b.change().unwrap_or_default());
                let o = match col {
                    Col::Symbol => a.label.cmp(&b.label),
                    Col::Last => a.price().total_cmp(&b.price()),
                    Col::Chg => ca.0.total_cmp(&cb.0),
                    Col::Pct => ca.1.total_cmp(&cb.1),
                };
                if asc {
                    o
                } else {
                    o.reverse()
                }
            });
        }
        idx
    }

    fn lines(&self) -> Vec<Line> {
        let mut out = vec![];
        for (s, sec) in self.sections.iter().enumerate() {
            out.push(Line::Section(s));
            if !self.prefs.collapsed.contains(&sec.name) {
                let shown = |&r: &usize| self.prefs.flag_filter.is_none_or(|k| self.flag(&sec.rows[r].coin) == Some(k));
                out.extend(self.sorted(s).into_iter().filter(shown).map(|r| Line::Row(s, r)));
            }
        }
        out
    }

    /// Symboles dans l'ordre affiché (sections repliées exclues).
    pub fn order(&self) -> Vec<String> {
        self.lines()
            .into_iter()
            .filter_map(|l| match l {
                Line::Row(s, r) => Some(self.sections[s].rows[r].coin.clone()),
                Line::Section(_) => None,
            })
            .collect()
    }

    /// 🧩 Rangées de la watchlist compacte (module de la colonne), dans l'ordre affiché.
    fn mini_rows(&self) -> Vec<&Row> {
        let all: Vec<&Row> = self.sections.iter().flat_map(|s| &s.rows).collect();
        self.order().iter().filter_map(|c| all.iter().find(|r| r.coin == *c).copied()).collect()
    }

    /// Hauteur de la watchlist compacte.
    pub fn mini_h(&self) -> f64 {
        self.mini_rows().len() as f64 * MINI_ROW + 8.0
    }

    /// 🧩 Watchlist compacte (module de la colonne) : nom, dernier prix, variation 24 h ; le symbole ouvert surligné.
    pub fn draw_mini(&self, pm: &mut Pixmap, text: &Text, w: f64, mouse: Option<(f64, f64)>, th: &Theme) {
        for (k, r) in self.mini_rows().into_iter().enumerate() {
            let y = k as f64 * MINI_ROW;
            let over = mouse.is_some_and(|(_, my)| (y..y + MINI_ROW).contains(&my));
            if r.coin == self.selected || over {
                fill(pm, 0.0, y, w, MINI_ROW, pal::hover(), if r.coin == self.selected { 1.0 } else { 0.6 });
            }
            text.draw(pm, &text.fit(&r.label, 12.5, w * 0.42), (10.0, (y + 17.0) as f32), 12.5, WHITE, Align::Left);
            text.draw(
                pm,
                &crate::viewport::format_number(r.price(), r.decimals()),
                ((w * 0.74) as f32, (y + 17.0) as f32),
                12.5,
                ICON,
                Align::Right,
            );
            let (pct, c) = match r.variation() {
                Some(pct) => (format!("{pct:+.2} %"), if pct >= 0.0 { th.bull } else { th.bear }),
                None => ("—".to_string(), MUTED),
            };
            text.draw(pm, &pct, ((w - 10.0) as f32, (y + 17.0) as f32), 12.0, c, Align::Right);
        }
    }

    /// Symbole sous `y` dans la watchlist compacte.
    pub fn mini_click(&self, y: f64) -> Option<String> {
        self.mini_rows().get((y / MINI_ROW).floor().max(0.0) as usize).map(|r| r.coin.clone())
    }

    /// Symbole précédent (`-1`) ou suivant (`+1`) du symbole ouvert ; `None` au bout de la liste.
    pub fn step(&self, dir: isize) -> Option<String> {
        let order = self.order();
        // le symbole ouvert peut être noté « BTC » (liste Hyperliquid) ou « hl:BTC » (tes listes) : les deux comptent
        let same = |c: &str| c == self.selected || c.strip_prefix("hl:") == Some(self.selected.as_str()) || self.selected.strip_prefix("hl:") == Some(c);
        let next = match order.iter().position(|c| same(c)) {
            Some(i) => i.checked_add_signed(dir).filter(|&j| j < order.len())?,
            None => 0,
        };
        order.get(next).cloned()
    }

    /// Hauteur d'une ligne.
    /// 📱 Décalage des mesures des photos (en-tête à y 63 sur l'écran) vers le haut de la feuille (sous sa poignée).
    fn dy_cell() -> f64 {
        crate::ecrans::boite_de("wl-entete.0").map_or(49.0, |b| b[1] - 14.0)
    }

    /// 📱 En-tête du cell : ⋯ (menu des listes), le titre au centre, 🔎 (chercher / ajouter un symbole).
    pub fn entete_cell(&self) -> (Rect, Rect, Rect) {
        let b = |k: usize| crate::ecrans::boite_de(&format!("wl-entete.{k}")).unwrap_or([17.0, 63.0, 45.0, 45.0]);
        let (p, t, l) = (b(0), b(1), b(2));
        let dy = Self::dy_cell();
        let droite = self.pw() - (390.0 - l[0]);
        (
            Rect::new(p[0], p[1] - dy, p[2], p[3]),
            Rect::new(t[0], t[1] - dy, (droite - 4.0 - t[0]).max(40.0), t[3]),
            Rect::new(droite, l[1] - dy, l[2], l[3]),
        )
    }

    /// 📱 Les pilules des listes (feuille), coupées au bord : (place, id, nom).
    pub fn pilules_cell(&self, text: &Text) -> Vec<(Rect, i64, String)> {
        let b = crate::ecrans::boite_de("wl-listes.0").unwrap_or([17.0, 128.0, 90.0, 32.0]);
        let dy = Self::dy_cell();
        let mut x = b[0];
        let mut out = vec![];
        for (id, nom) in &self.listes {
            let w = text.width(nom, 15.0) as f64 + 28.0;
            if x > self.pw() {
                break;
            }
            out.push((Rect::new(x, b[1] - dy, w, b[3]), *id, nom.clone()));
            x += w + 8.0;
        }
        out
    }

    /// 📱 La mini-pilule du symbole ouvert, en bas de la feuille de hauteur `h` (photos de Jee).
    pub fn mini_cell(&self, h: f64) -> Rect {
        let b = crate::ecrans::boite_de("wl-mini").unwrap_or([22.0, 705.0, 346.0, 55.0]);
        Rect::new(b[0], h - b[3] - 8.0, self.pw() - 2.0 * b[0], b[3])
    }

    /// Haut de la liste des symboles dans la feuille (📱 sous l'en-tête et les pilules).
    fn haut_liste(&self) -> f64 {
        self.haut_tous() + self.bande_tous() + if self.phone.is_some() { 0.0 } else { HEAD_H }
    }

    /// Haut de la rangée des pilules de « Tous les marchés » (sous l'en-tête ; 📱 sous les pilules des listes).
    fn haut_tous(&self) -> f64 {
        if self.phone.is_some() {
            let b = crate::ecrans::boite_de("wl-listes.0").unwrap_or([17.0, 128.0, 90.0, 32.0]);
            b[1] - Self::dy_cell() + b[3] + 8.0
        } else {
            TOP
        }
    }

    /// 🌐 Hauteur de la rangée des pilules Plateforme ▾ / Trier par ▾ (0 hors de « Tous les marchés »).
    fn bande_tous(&self) -> f64 {
        match (self.prefs.tous, self.phone.is_some()) {
            (false, _) => 0.0,
            (true, true) => 44.0,
            (true, false) => 40.0,
        }
    }

    /// 🌐 Les deux pilules de verre de « Tous les marchés » : (place, `true` = Plateforme, texte sans le ⌄).
    pub fn pilules_tous(&self, text: &Text) -> Vec<(Rect, bool, String)> {
        if !self.prefs.tous {
            return vec![];
        }
        let ex = argos_data::Exchange::parse(&self.prefs.tous_bourse).unwrap_or(argos_data::Exchange::Hyperliquid);
        let tri = argos_data::tous::Tri::parse(&self.prefs.tous_tri);
        let (x0, size, h) = if self.phone.is_some() { (17.0, 14.0, 32.0) } else { (12.0, 13.0, 30.0) };
        let y = self.haut_tous() + (self.bande_tous() - h) / 2.0;
        let a = ex.name().to_string();
        let wa = text.width(&a, size) as f64 + 40.0;
        let room = (self.pw() - x0 - wa - 8.0 - 12.0 - 40.0).max(40.0);
        let b = text.fit(&format!("Trier par : {}", tri.court()), size, room);
        let wb = text.width(&b, size) as f64 + 40.0;
        vec![(Rect::new(x0, y, wa, h), true, a), (Rect::new(x0 + wa + 8.0, y, wb, h), false, b)]
    }

    /// 🌐 Dessine les pilules de « Tous les marchés » (verre, ⌄ à droite).
    fn pilules_tous_draw(&self, pm: &mut Pixmap, text: &Text, mouse: Option<(f64, f64)>) {
        let size = if self.phone.is_some() { 14.0 } else { 13.0 };
        for (r, _, t) in self.pilules_tous(text) {
            crate::jetons::verre(pm, r, r.h / 2.0);
            if mouse.is_some_and(|(x, y)| r.contains(x, y)) {
                round_fill(pm, r, r.h / 2.0, crate::jetons::VERRE2.0, crate::jetons::VERRE2.1);
            }
            text.draw(pm, &t, ((r.x + 14.0) as f32, (r.cy() + size * 0.36) as f32), size as f32, WHITE, Align::Left);
            lines(pm, r.x + r.w - 16.0, r.cy(), &[&[(-3.5, -1.5), (0.0, 2.0), (3.5, -1.5)]], WHITE, 1.3);
        }
    }

    /// 📱 🌐 ☆ d'une ligne de « Tous les marchés » sur cell, à droite (le prix se pousse à sa gauche).
    fn etoile_cell(&self, top: f64) -> Rect {
        Rect::new(self.pw() - 52.0, top + (self.row_h() - 40.0) / 2.0, 40.0, 40.0)
    }

    /// Hauteur d'une rangée de symbole (📱 plus haute sur cell).
    fn row_h(&self) -> f64 {
        // 📱 cell : la ligne des photos de Jee (modèle TradingView mobile), mesurée
        match (self.phone, crate::ecrans::boite_de("wl-ligne")) {
            (Some(_), Some(b)) => b[3],
            _ => ROW_H,
        }
    }

    fn line_h(&self, l: Line) -> f64 {
        if matches!(l, Line::Section(_)) {
            match (self.phone, crate::ecrans::boite_de("wl-section")) {
                (Some(_), Some(b)) => b[3],
                _ => SEC_H,
            }
        } else {
            self.row_h()
        }
    }

    /// Lignes visibles avec leur haut (en pixels).
    fn placed(&self) -> Vec<(f64, Line)> {
        let mut y = self.haut_liste() - self.scroll;
        self.lines()
            .into_iter()
            .map(|l| {
                let top = y;
                y += self.line_h(l);
                (top, l)
            })
            .collect()
    }

    /// 📡 Markets of the list rows inside the height `h` (the scroll taken into account): the live prices are asked
    /// for those only.
    pub fn coins_visibles(&self, h: f64) -> Vec<String> {
        self.placed()
            .into_iter()
            .filter(|(top, l)| *top + self.line_h(*l) > 0.0 && *top < h)
            .filter_map(|(_, l)| match l {
                Line::Row(s, r) => Some(self.sections[s].rows[r].coin.clone()),
                _ => None,
            })
            .collect()
    }

    fn content_h(&self) -> f64 {
        self.lines().into_iter().map(|l| self.line_h(l)).sum()
    }

    /// Nom de la liste dans l'en-tête (clic : menu des listes) : il s'arrête avant 🔌 (sinon, en volet étroit, le clic
    /// sur 🔌 ouvrait le menu des listes).
    pub fn title_rect(&self) -> Rect {
        Rect::new(6.0, 8.0, (self.api_rect().x - 4.0 - 6.0).clamp(0.0, 210.0), 30.0)
    }

    /// 🖥 ✕ du volet ouvert (PC), en haut à droite comme la maquette (`#panneau header … ✕`) : il ferme le volet ; le rond
    /// Argos AI et les boutons de l'en-tête sont juste à sa gauche.
    pub fn fermer_rect(&self) -> Option<Rect> {
        // (la feuille Trading de la maquette a ses propres ⬓ ⤢ ✕ dans son en-tête : pas de doublon)
        // (🎯 the Stratégie volet too: its ⤢ ✕ in its header, lower, as in its mock-up; 📖 the Carnet's ✕ in its header)
        let feuille = (self.prefs.panel == Tool::Trading && self.panels.feuille_trading()) || matches!(self.prefs.panel, Tool::Strategie | Tool::Carnet);
        (self.prefs.open && self.phone.is_none() && self.fixed_w.is_none() && !feuille).then(|| Rect::new(self.pw() - 12.0 - 30.0, 8.0, 30.0, 30.0))
    }

    /// La liste revient en haut (défilement à zéro).
    pub fn haut_de_liste(&mut self) {
        (self.scroll, self.scroll_to) = (0.0, 0.0);
    }

    /// Le volet revient en haut (défilement à zéro).
    pub fn haut_du_volet(&mut self) {
        (self.panel_scroll, self.panel_to) = (0.0, 0.0);
    }

    /// 🖥 ⤢ du volet ouvert (PC) : volet large ⇄ normal (maquette `#plarge`, l.836), à gauche du ✕.
    pub fn agrandir_rect(&self) -> Option<Rect> {
        self.fermer_rect().map(|r| Rect::new(r.x - 34.0, r.y, r.w, r.h))
    }

    /// 🖥 ⬓ du volet ouvert (PC) : le volet devient un module de la colonne de modules (maquette `#pancre`, l.836) ;
    /// seulement s'il a un module équivalent.
    pub fn ancrer_rect(&self) -> Option<Rect> {
        module_du_volet(self.prefs.panel)?;
        self.fermer_rect().map(|r| Rect::new(r.x - 68.0, r.y, r.w, r.h))
    }

    /// 🔌 dans l'en-tête, à côté du ＋ : le panneau des API.
    pub fn api_rect(&self) -> Rect {
        Rect::new(self.pw() - 78.0 - FERME_PLACE, 8.0, 30.0, 30.0)
    }

    /// ＋ dans l'en-tête (ajouter un symbole).
    pub fn add_rect(&self) -> Rect {
        Rect::new(self.pw() - 44.0 - FERME_PLACE, 8.0, 30.0, 30.0)
    }

    /// Place de la liste sous l'en-tête (le panneau ＋ s'y ouvre, comme Argos v1), coordonnées de l'image.
    pub fn list_rect(&self, h: f64) -> Rect {
        Rect::new(0.0, TOP, self.pw(), h - TOP - self.foot_h())
    }

    /// ✕ au bout d'une ligne (liste de l'utilisateur), au survol.
    fn remove_rect(&self, top: f64) -> Rect {
        Rect::new(self.pw() - 34.0, top + (self.row_h() - 22.0) / 2.0, 22.0, 22.0)
    }

    /// Couleur du drapeau d'un symbole (`None` : pas de drapeau).
    pub fn flag(&self, coin: &str) -> Option<usize> {
        self.prefs.flags.iter().find(|(c, _)| c == coin).map(|(_, k)| *k)
    }

    /// Bulle du choix de couleur, à droite du drapeau de la ligne du symbole.
    fn picker_rect(&self, coin: &str) -> Option<Rect> {
        let (top, _) = self.placed().into_iter().find(|(_, l)| match l {
            Line::Row(s, r) => self.sections[*s].rows[*r].coin == coin,
            Line::Section(_) => false,
        })?;
        Some(Rect::new(20.0, top + 2.0, FLAG_COLORS.len() as f64 * DOT + 12.0, self.row_h() - 4.0))
    }

    /// Défilement maximum (0 : tout tient dans le panneau).
    fn max_scroll(&self, h: f64) -> f64 {
        (self.content_h() - (h - TOP - HEAD_H - self.foot_h())).max(0.0)
    }

    /// Molette dans le panneau (`steps` > 0 = vers le haut).
    pub fn wheel(&mut self, steps: f64, h: f64) {
        match self.panel_h() {
            Some(ch) => self.panel_to = (self.panel_to - steps * 56.0).clamp(0.0, (ch - h).max(0.0)),
            None => self.scroll_to = (self.scroll_to - steps * self.row_h()).clamp(0.0, self.max_scroll(h)),
        }
    }

    /// Une image du glissement doux vers la cible de la molette ; `true` : encore en mouvement (redessiner).
    pub fn glide(&mut self) -> bool {
        let step = |v: &mut f64, to: f64| {
            let d = to - *v;
            *v = if d.abs() < 0.5 { to } else { *v + d * 0.35 };
            *v != to
        };
        let a = step(&mut self.scroll, self.scroll_to);
        let b = step(&mut self.panel_scroll, self.panel_to);
        a || b
    }

    /// Hauteur du contenu du panneau ouvert ; `None` pour la liste des symboles (elle a son propre défilement).
    fn panel_h(&self) -> Option<f64> {
        if !self.prefs.open {
            return None;
        }
        match (self.prefs.panel.panel(), self.prefs.panel) {
            (Some(k), _) => Some(self.panels.content_h(k, self.pw())),
            (None, Tool::Market) => Some(crate::cmc::HEIGHT),
            (None, Tool::Agenda) => Some(self.eco.content_h(crate::eco::EcoKind::Agenda, self.pw(), self.now)),
            (None, Tool::News) => Some(self.eco.content_h(crate::eco::EcoKind::News, self.pw(), self.now)),
            (None, Tool::Whales) => Some(self.whales.content_h()),
            (None, Tool::Explorer) => Some(self.explorer.content_h(&Text::default(), self.pw())),
            (None, Tool::Stats) => Some(self.stats.content_h(self.pw())),
            (None, Tool::Modules) => Some(crate::dock::content_h(&self.dock)),
            (None, Tool::Exchanges) => Some(crate::exchanges::content_h(&self.exchanges)),
            (None, Tool::Strategie) => Some(self.strategie.content_h(self.pw(), self.phone.is_some())),
            // 📖 never scrolled: the panel takes the height it is given (its footer and ⚙ always in view)
            (None, Tool::Carnet) => Some(self.bande_ai(0.0).h),
            _ => None,
        }
    }

    /// 📶📱 The band kept free at the bottom of the open Stats Hyperliquid sheet (visible height `h`) for the round
    /// Argos AI button, by touch (the same room as the Calendrier's bottom); 📖📱 the phone's Carnet sheet too (its
    /// footer and ⚙ above it); empty elsewhere.
    pub fn bande_ai(&self, h: f64) -> Rect {
        let m = crate::eco::marge_ai();
        let carnet = self.prefs.panel == Tool::Carnet && self.phone.is_some();
        if self.prefs.open && (self.prefs.panel == Tool::Stats || carnet) && m > 0.0 {
            Rect::new(0.0, h - m, self.pw(), m)
        } else {
            Rect::new(0.0, h, 0.0, 0.0)
        }
    }

    /// Défilement du panneau, borné à la hauteur `h` visible.
    fn panel_off(&self, h: f64) -> f64 {
        self.panel_h().map_or(0.0, |ch| self.panel_scroll.min((ch - h).max(0.0)))
    }

    /// Barre de défilement : (rail, curseur) ; `None` si tout tient dans le panneau.
    fn scrollbar(&self, h: f64) -> Option<(Rect, Rect)> {
        let max = self.max_scroll(h);
        if max <= 0.0 {
            return None;
        }
        let (top, view) = (self.haut_liste(), h - TOP - HEAD_H - self.foot_h());
        let track = Rect::new(self.pw() - 10.0, top, 10.0, view);
        let th = (view * view / self.content_h()).max(30.0);
        let ty = top + self.scroll / max * (view - th);
        Some((track, Rect::new(track.x + 2.0, ty, 6.0, th)))
    }

    /// Barre de défilement attrapée ?
    pub fn dragging(&self) -> bool {
        self.bar_grab.is_some()
    }

    /// Glisser la barre de défilement (souris à la hauteur `y`).
    pub fn drag_to(&mut self, y: f64, h: f64) {
        let (Some(grab), Some((track, thumb))) = (self.bar_grab, self.scrollbar(h)) else {
            return;
        };
        let room = (track.h - thumb.h).max(1.0);
        self.scroll = ((y - grab - track.y) / room * self.max_scroll(h)).clamp(0.0, self.max_scroll(h));
        self.scroll_to = self.scroll;
    }

    /// Lâcher la barre de défilement.
    pub fn release(&mut self) {
        self.bar_grab = None;
    }

    /// Largeur du volet ouvert (0 fermé) : la colonne d'icônes commence après.
    pub fn bar_x(&self) -> f64 {
        if self.prefs.open {
            self.pw()
        } else {
            0.0
        }
    }

    /// Bouton du `i`-ième outil dans la barre d'icônes.
    pub fn tool_rect(&self, i: usize) -> Rect {
        if let Some(c) = self.col_cadre() {
            let (pas, b) = self.col_pas();
            return Rect::new(c.x + (COL_W - b) / 2.0, c.y + 9.0 + i as f64 * pas, b, b);
        }
        let traits = self.bar_tools().iter().take(i).filter(|t| t.separe()).count() as f64;
        Rect::new(self.bar_x() + 9.0, 8.0 + i as f64 * PAS_OUTIL + traits * TRAIT_OUTIL, 38.0, 38.0)
    }

    /// 🖥 Pas et taille des boutons de la colonne : 40 et 36 comme la maquette, serrés si tous les outils ne tiennent
    /// pas au-dessus de ›.
    fn col_pas(&self) -> (f64, f64) {
        let c = self.col_cadre().unwrap_or_default();
        let place = c.y + c.h - 83.0 - 4.0 - (c.y + 9.0);
        let n = self.bar_tools().len().max(1) as f64;
        let pas = (place / n).clamp(28.0, COL_PAS);
        (pas, (pas - 4.0).min(COL_BTN))
    }

    /// 🖥 La colonne repliée (maquette `colrep`) : une bande de 12 avec sa poignée, sous les rangées du haut ; au
    /// survol (`col_survol`) elle s'élargit à 44 par-dessus la chart. Un volet ouvert la montre en entier.
    fn col_repliee_ici(&self) -> bool {
        self.col_repliee && !self.prefs.open
    }

    /// 🖥 Cadre de la colonne avec ses icônes (coordonnées de l'image ; dépliée, elle commence au-dessus, à `-haut`) ;
    /// `None` sans colonne ou repliée en bande.
    pub fn col_cadre(&self) -> Option<Rect> {
        let haut = self.colonne?;
        if self.col_repliee_ici() {
            // par-dessus la chart, collée à 6 du bord droit comme dépliée
            return self.col_survol.then(|| Rect::new(COL_REPLIEE - COL_X - COL_W, 0.0, COL_W, self.col_h - 40.0));
        }
        Some(Rect::new(self.bar_x() + COL_X, -haut, COL_W, haut + self.col_h - COL_MARGE))
    }

    /// 🖥 La bande de la colonne repliée (12 de large, à 6 du bord droit, sous les rangées du haut, jusqu'à 40 du bas).
    pub fn col_bande(&self) -> Option<Rect> {
        self.colonne?;
        (self.col_repliee_ici() && !self.col_survol).then(|| Rect::new(COL_REPLIEE - COL_X - 12.0, 0.0, 12.0, self.col_h - 40.0))
    }

    /// 🖥 › / ‹ (replier la barre, ou la garder ouverte) et ⚙ (personnaliser la barre) en bas de la colonne.
    pub fn col_boutons(&self, _h: f64) -> Option<[(Rect, Click); 2]> {
        let c = self.col_cadre()?;
        let x = c.x + (COL_W - COL_BTN) / 2.0;
        let bas = c.y + c.h;
        let (r1, r2) = (
            Rect::new(x, bas - 2.0 * COL_PAS - 3.0, COL_BTN, COL_BTN),
            Rect::new(x, bas - COL_PAS - 3.0, COL_BTN, COL_BTN),
        );
        Some([(r1, Click::ReplierBarre), (r2, Click::PersonnaliserBarre(r2))])
    }

    /// 🖥 Ce que la colonne occupe (ses icônes, ou sa bande repliée), coordonnées de l'image.
    pub fn col_rect(&self, _h: f64) -> Option<Rect> {
        self.col_cadre().or_else(|| self.col_bande())
    }

    /// ↕️ Boutons de la barre d'icônes, de haut en bas : ton ordre (les outils nouveaux à la fin), sans les masqués.
    pub fn bar_tools(&self) -> Vec<Tool> {
        // 🎯 at the end while a strategy is on the chart (never saved in the bar's order)
        let strategie = !self.strategie.noms.is_empty();
        if let Some(v) = &self.col_outils {
            let mut v: Vec<Tool> = v.iter().copied().filter(|t| Tool::ALL.contains(t)).collect();
            if strategie {
                v.push(Tool::Strategie);
            }
            return v;
        }
        let mut v: Vec<Tool> = self.prefs.order.iter().copied().filter(|t| Tool::ALL.contains(t)).collect();
        v.dedup();
        for t in Tool::ALL {
            if !v.contains(&t) {
                v.push(t);
            }
        }
        v.retain(|t| !self.prefs.hidden_tools.contains(t));
        if strategie {
            v.push(Tool::Strategie);
        }
        v
    }

    /// 🖥 The help bubble of the icon bar under the mouse (PC, like the drawing bar on the left: Jee, 19 Nov): the
    /// name of the tool really at that place (`Tool::name`, as in « Personnaliser la barre ») and its button; `None`
    /// elsewhere and on the phone.
    pub fn bulle_outil(&self, x: f64, y: f64) -> Option<(&'static str, Rect)> {
        let (i, t) = self.tool_at(x, y)?;
        Some((t.name(), self.tool_rect(i)))
    }

    /// Bouton de la barre d'icônes sous `(x, y)` : (rang, outil).
    pub fn tool_at(&self, x: f64, y: f64) -> Option<(usize, Tool)> {
        if self.phone.is_some() {
            return None;
        }
        self.bar_tools().into_iter().enumerate().find(|(i, _)| self.tool_rect(*i).contains(x, y))
    }

    /// Rang visé quand on glisse un bouton à la hauteur `y` (borné à la barre).
    pub fn tool_slot(&self, y: f64) -> usize {
        let n = self.bar_tools().len();
        (0..n)
            .min_by(|a, b| (self.tool_rect(*a).cy() - y).abs().total_cmp(&(self.tool_rect(*b).cy() - y).abs()))
            .unwrap_or(0)
    }

    /// ↕️ Déplace le bouton du rang `from` au rang `to` (l'ordre est gardé dans les réglages).
    pub fn move_tool(&mut self, from: usize, to: usize) {
        if let Some(v) = self.col_outils.as_mut() {
            if from < v.len() && to < v.len() && from != to {
                let t = v.remove(from);
                v.insert(to, t);
            }
            return;
        }
        let mut all: Vec<Tool> = self.prefs.order.iter().copied().filter(|t| Tool::ALL.contains(t)).collect();
        for t in Tool::ALL {
            if !all.contains(&t) {
                all.push(t);
            }
        }
        let vis = self.bar_tools();
        let (Some(&a), Some(&b)) = (vis.get(from), vis.get(to)) else { return };
        // (🎯 stays at the end: it is not in the saved order)
        if a == b || a == Tool::Strategie || b == Tool::Strategie {
            return;
        }
        let ia = all.iter().position(|t| *t == a).unwrap_or(0);
        all.remove(ia);
        let ib = all.iter().position(|t| *t == b).unwrap_or(all.len());
        all.insert(if to > from { ib + 1 } else { ib }, a);
        self.prefs.order = all;
    }

    /// Ordre par défaut et tous les boutons visibles.
    pub fn reset_tools(&mut self) {
        (self.prefs.order, self.prefs.hidden_tools) = (vec![], Tool::CACHES.to_vec());
    }

    /// 🙈 Masque / 👁 réaffiche un bouton (la watchlist reste toujours là, pour retrouver la barre).
    pub fn set_tool_hidden(&mut self, t: Tool, hidden: bool) {
        self.prefs.hidden_tools.retain(|x| *x != t);
        if hidden && t != Tool::Watchlist {
            self.prefs.hidden_tools.push(t);
            if self.prefs.panel == t {
                self.prefs.open = false;
            }
        }
    }

    /// Ouvre un outil (son panneau) ; le même encore : ferme le panneau.
    pub fn toggle_tool(&mut self, t: Tool) {
        if self.tool_on(t) {
            self.prefs.open = false;
        } else {
            (self.prefs.open, self.prefs.panel) = (true, t);
        }
    }

    fn tool_on(&self, t: Tool) -> bool {
        self.prefs.open && self.prefs.panel == t
    }

    /// What a click in a panel (Indicateurs, Trading démo, Journal) asks the window.
    fn depuis_panneau(&mut self, c: PanelClick) -> Click {
        match c {
            PanelClick::None => Click::None,
            PanelClick::Changed => Click::Changed,
            PanelClick::Soon(s) => Click::Soon(s),
            PanelClick::Ticket => Click::Ticket,
            PanelClick::Order => Click::Order,
            PanelClick::CloseAll => Click::CloseAll,
            PanelClick::GuardCfg => Click::GuardCfg,
            PanelClick::Unlock => Click::Unlock,
            PanelClick::CancelOrder(id) => Click::CancelOrder(id),
            PanelClick::Accounts => Click::Accounts,
            PanelClick::Fermer => Click::FermerFeuille,
            PanelClick::Ancrer => Click::AncrerVolet(crate::dock::Module::Positions),
            PanelClick::Marches => {
                self.toggle_tool(Tool::Market);
                (self.panel_scroll, self.panel_to) = (0.0, 0.0);
                Click::Changed
            }
            PanelClick::Agrandir => Click::AgrandirVolet,
            PanelClick::Simulateur => Click::Simulateur,
            PanelClick::FermerSimulateur => Click::FermerSimulateur,
            PanelClick::Indicator(i) => Click::Indicator(i),
            PanelClick::IndSettings(i) => Click::IndSettings(i),
            PanelClick::Marketplace => Click::Marketplace,
            PanelClick::Movers => Click::Movers,
            PanelClick::Academy => Click::Academy,
            PanelClick::Journal(c) => Click::Journal(c),
            PanelClick::Script(o) => Click::Script(o),
            PanelClick::Ai => Click::Ai,
            PanelClick::Passer(achat) => Click::Passer(achat),
            PanelClick::Saisir(champ) => Click::Saisir(champ),
            PanelClick::Cible => Click::Cible,
            PanelClick::Carnet => Click::Carnet,
            PanelClick::Curseur => Click::Curseur,
        }
    }

    /// 🎚 (`x`, `y`) of the panel (`h` high) is on the % slider of the Trading sheet: a finger put there drags it
    /// instead of scrolling the sheet.
    pub fn sur_curseur_ticket(&self, x: f64, y: f64, h: f64) -> bool {
        let pw = self.pw();
        let colle = self.panels.ticket_boutons_colles(pw, h).is_some() && y >= h - crate::panels::ticket::BAS_COLLE;
        if !self.prefs.open || self.prefs.panel != Tool::Trading || x >= pw || y < self.panels.entete_fixe() || colle {
            return false;
        }
        let (y, h) = match self.panel_h() {
            Some(ch) => (y + self.panel_off(h), ch.max(h)),
            None => (y, h),
        };
        self.panels.sur_curseur(x, y, pw, h)
    }

    /// 🎚 The ticket's % slider held: it follows `x` (from the panel's left edge).
    pub fn glisse_ticket(&mut self, x: f64) {
        let pw = self.pw();
        self.panels.ticket_glisse(x, pw);
    }

    /// Clic dans l'image de la watchlist (coordonnées depuis son coin haut-gauche).
    pub fn click(&mut self, x: f64, y: f64, h: f64) -> Click {
        if let Some(b) = self.col_boutons(h) {
            if let Some((_, c)) = b.into_iter().find(|(r, _)| r.contains(x, y)) {
                if c == Click::ReplierBarre {
                    if self.col_outils.is_some() {
                        // 🖥 interface-2 : repliée en bande / gardée ouverte (watchlist.json n'est pas touché)
                        (self.col_repliee, self.col_survol) = (!self.col_repliee, false);
                    } else {
                        (self.prefs.folded, self.prefs.open) = (true, false);
                    }
                }
                return c;
            }
        }
        if let Some((i, t)) = self.tool_at(x, y) {
            if t == Tool::Marketplace {
                return Click::Marketplace;
            }
            if t.action() {
                return Click::Outil(t, self.tool_rect(i));
            }
            self.toggle_tool(t);
            (self.panel_scroll, self.panel_to) = (0.0, 0.0);
            return Click::Changed;
        }
        if !self.prefs.open || x >= self.pw() {
            return Click::None;
        }
        // 📱 cell / tablette : toucher la poignée en haut de la feuille (la bande au-dessus de son contenu) la referme
        // (Jee, 20 nov. : « tous les volets ne se referment pas » avec la poignée)
        if self.phone.is_some() && y < POIGNEE_CELL {
            return Click::FermerFeuille;
        }
        // 🖥 ✕ : le volet se ferme ; ⤢ : large ⇄ normal ; ⬓ : il devient un module (ils passent avant le contenu)
        if self.fermer_rect().is_some_and(|r| r.contains(x, y)) {
            return Click::FermerFeuille;
        }
        if self.agrandir_rect().is_some_and(|r| r.contains(x, y)) {
            return Click::AgrandirVolet;
        }
        if let Some(m) = module_du_volet(self.prefs.panel).filter(|_| self.ancrer_rect().is_some_and(|r| r.contains(x, y))) {
            return Click::AncrerVolet(m);
        }
        // 📌📱 the Trading sheet: its header stays at the top, and on the phone VENDRE / ACHETER stay at the bottom (not
        // scrolled, so touched where they are seen)
        if self.prefs.panel == Tool::Trading && self.panels.feuille_trading() {
            let (pw, ch) = (self.pw(), self.panel_h().unwrap_or(h).max(h));
            let c = if y < self.panels.entete_fixe() {
                Some(self.panels.click(Kind::Trading, x, y, pw, ch))
            } else {
                self.panels.click_colle(x, y, pw, h)
            };
            if let Some(c) = c {
                return self.depuis_panneau(c);
            }
        }
        // 🎯 Stratégie (its content scrolls: the menu of the pill goes where the pill is seen)
        if self.prefs.panel == Tool::Strategie {
            use crate::strategie::StratClick as S;
            let (pw, cell, off) = (self.pw(), self.phone.is_some(), self.panel_off(h));
            return match self.strategie.click(&Text::default(), pw, x, y + off, cell) {
                S::None => Click::None,
                S::Changed => Click::Changed,
                S::Fermer => Click::FermerFeuille,
                S::Choisir(r) => Click::Strategie(S::Choisir(Rect::new(r.x, r.y - off, r.w, r.h))),
                c => Click::Strategie(c),
            };
        }
        // 📖 Carnet (its content scrolls too: a menu opens where its pill is seen)
        if self.prefs.panel == Tool::Carnet {
            use crate::carnet::CarnetClick as C;
            let (pw, cell, off) = (self.pw(), self.phone.is_some(), self.panel_off(h));
            let ch = self.panel_h().unwrap_or(h).max(h) - self.bande_ai(h).h;
            let vu = |r: Rect| Rect::new(r.x, r.y - off, r.w, r.h);
            return match crate::carnet::click(&Text::default(), (pw, ch), &self.prefs.carnet, &self.carnet, cell, true, x, y + off) {
                C::Rien => Click::None,
                C::Fermer => Click::FermerFeuille,
                C::Bourse(r) => Click::CarnetPanneau(C::Bourse(vu(r))),
                C::Pas(r) => Click::CarnetPanneau(C::Pas(vu(r))),
                C::Echange(r) => Click::CarnetPanneau(C::Echange(vu(r))),
                C::Reglages(r) => Click::CarnetPanneau(C::Reglages(vu(r))),
                c => Click::CarnetPanneau(c),
            };
        }
        // (📶📱 the room of the round Argos AI button at the bottom of the Stats sheet: no row under it)
        if self.bande_ai(h).contains(x, y) {
            return Click::None;
        }
        // panneaux qui défilent : la souris dans le contenu, qui est au moins aussi haut que sa hauteur utile
        let (y, h) = match self.panel_h() {
            Some(ch) => (y + self.panel_off(h), ch.max(h)),
            None => (y, h),
        };
        // 📅 Agenda / 📰 News
        if let Some(kind) = match self.prefs.panel {
            Tool::Agenda => Some(crate::eco::EcoKind::Agenda),
            Tool::News => Some(crate::eco::EcoKind::News),
            _ => None,
        } {
            return match self.eco.click(kind, &Text::default(), self.pw(), x, y, self.now) {
                crate::eco::EcoClick::Changed | crate::eco::EcoClick::Carte => Click::Changed,
                crate::eco::EcoClick::Close => {
                    self.prefs.open = false;
                    Click::Changed
                }
                crate::eco::EcoClick::Open(link) => Click::OpenUrl(link),
                crate::eco::EcoClick::Learn(row) => Click::AcademyNews(row),
                crate::eco::EcoClick::None => Click::None,
            };
        }
        // 🧩 Modules
        if self.prefs.panel == Tool::Modules {
            let pw = self.pw();
            return if crate::dock::click(&mut self.dock, &Text::default(), pw, x, y) {
                Click::Dock
            } else {
                Click::None
            };
        }
        // 🧭 Explorer Hyperliquid, 📶 Stats Hyperliquid
        if self.prefs.panel == Tool::Explorer {
            return match self.explorer.click(&Text::default(), self.pw(), x, y) {
                crate::hlexplorer::ExClick::None => Click::None,
                // (⚙ choices are kept by the app)
                c => Click::Explorer(c),
            };
        }
        if self.prefs.panel == Tool::Stats {
            return match self.stats.click(self.pw(), h, x, y) {
                crate::hlstats::StClick::None => Click::None,
                crate::hlstats::StClick::Changed => Click::Changed,
                crate::hlstats::StClick::Chart(c) => Click::Open(c),
            };
        }
        // 🐋 Baleines
        if self.prefs.panel == Tool::Whales {
            return match self.whales.click(&Text::default(), self.pw(), x, y) {
                crate::whales::WhaleClick::Changed | crate::whales::WhaleClick::Follow(_) => Click::Changed,
                crate::whales::WhaleClick::Open(link) => Click::OpenUrl(link),
                crate::whales::WhaleClick::None => Click::None,
            };
        }
        // 🪙 Marché, sans clé CoinMarketCap : la ligne « 🔑 Ajoute ta clé… » ouvre la boîte de clé
        if self.prefs.panel == Tool::Market && !self.market.cmc_cle && crate::cmc::cle_rect(self.pw(), FERME_PLACE).contains(x, y) {
            return Click::CleCmc;
        }
        // 🪙 Marché : 🔑 Clé API ouvre directement la boîte de la clé CoinMarketCap (demande de Jee), sans la liste des API
        if self.prefs.panel == Tool::Market && crate::cmc::key_rect(self.pw(), FERME_PLACE).contains(x, y) {
            return Click::CleCmc;
        }
        // panneau Exchanges
        if self.prefs.panel == Tool::Exchanges {
            return match crate::exchanges::click(&self.exchanges, x, y, self.pw()) {
                crate::exchanges::ExClick::None => Click::None,
                crate::exchanges::ExClick::Ai => Click::Ai,
                crate::exchanges::ExClick::Action(a) => Click::Exchange(a),
            };
        }
        // panneau Indicateurs / Trading démo / Journal
        if let Some(k) = self.prefs.panel.panel() {
            let c = self.panels.click(k, x, y, self.pw(), h);
            return self.depuis_panneau(c);
        }
        if let Some((r, bourse, _)) = self.pilules_tous(&Text::default()).into_iter().find(|(r, _, _)| r.contains(x, y)) {
            return if bourse { Click::TousBourse(r) } else { Click::TousTri(r) };
        }
        if self.phone.is_some() {
            if self.mini_cell(h).contains(x, y) {
                return Click::FermerFeuille;
            }
            let (dots, titre, loupe) = self.entete_cell();
            if dots.contains(x, y) || titre.contains(x, y) {
                return Click::Lists(dots);
            }
            if loupe.contains(x, y) {
                return Click::Add(loupe);
            }
            if let Some((_, id, _)) = self.pilules_cell(&Text::default()).into_iter().find(|(r, _, _)| r.contains(x, y)) {
                return Click::Liste(id);
            }
            if y < self.haut_liste() {
                return Click::None;
            }
        }
        if self.title_rect().contains(x, y) {
            return Click::Lists(self.title_rect());
        }
        if self.add_rect().contains(x, y) {
            return Click::Add(self.add_rect());
        }
        if self.api_rect().contains(x, y) {
            self.toggle_tool(Tool::Exchanges);
            (self.panel_scroll, self.panel_to) = (0.0, 0.0);
            return Click::Changed;
        }
        if (TOP..self.haut_liste()).contains(&y) {
            // (🌐 « Tous les marchés » : on trie avec la pilule Trier par)
            if self.prefs.tous {
                return Click::None;
            }
            let col = self.header_col(x);
            self.prefs.sort = match self.prefs.sort {
                Some((c, true)) if c == col => Some((col, false)),
                Some((c, false)) if c == col => None,
                _ => Some((col, true)),
            };
            return Click::Changed;
        }
        // choix de couleur du drapeau ouvert : une pastille = cette couleur (la même encore = enlever le drapeau)
        if let Some(coin) = self.picker.take() {
            if let Some(p) = self.picker_rect(&coin).filter(|p| p.contains(x, y)) {
                let k = (((x - p.x - 6.0) / DOT).floor().max(0.0) as usize).min(FLAG_COLORS.len() - 1);
                let had = self.flag(&coin);
                self.prefs.flags.retain(|(c, _)| *c != coin);
                if had != Some(k) {
                    self.prefs.flags.push((coin, k));
                }
            }
            return Click::Changed;
        }
        // barre de défilement : attraper le curseur, ou cliquer sur le rail pour y sauter
        if let Some((track, thumb)) = self.scrollbar(h).filter(|(t, _)| t.contains(x, y)) {
            if !thumb.contains(x, y) {
                self.bar_grab = Some(thumb.h / 2.0);
                self.drag_to(y, h);
            }
            self.bar_grab = Some(y - self.scrollbar(h).map_or(track.y, |(_, t)| t.y));
            return Click::Changed;
        }
        if y < self.haut_liste() || y > h - self.foot_h() {
            return Click::None;
        }
        let Some((top, line)) = self.placed().into_iter().find(|(top, l)| y >= *top && y < top + self.line_h(*l)) else {
            return Click::None;
        };
        let lift = if matches!(line, Line::Section(_)) {
            (self.row_h() - SEC_H) / 2.0
        } else {
            0.0
        };
        // (📱 cell : pas de ✕ caché au bord droit — il était sous le prix et retirait le symbole au moindre toucher)
        let on_x = self.editable && self.phone.is_none() && self.remove_rect(top - lift).contains(x, y);
        match line {
            Line::Section(s) => {
                let name = self.sections[s].name.clone();
                if on_x && !name.is_empty() {
                    return Click::RemoveSection(name);
                }
                // 📱 le nom d'une section de ta liste : son menu ; ⌄ à droite : la replier
                if self.phone.is_some() && self.editable && !name.is_empty() && x < self.pw() - 56.0 {
                    let (top, _) = self.placed().into_iter().find(|(t, l)| y >= *t && y < t + self.line_h(*l)).unwrap_or((y, line));
                    return Click::SectionMenu(name, Rect::new(16.0, top, 200.0, self.line_h(line)));
                }
                toggle(&mut self.prefs.collapsed, name);
                Click::Changed
            }
            Line::Row(s, r) => {
                let coin = self.sections[s].rows[r].coin.clone();
                // 🌐 « Tous les marchés » : ☆ (🖥 à gauche, à la place du drapeau ; 📱 à droite)
                let etoile = match self.phone {
                    Some(_) => self.etoile_cell(top).contains(x, y),
                    None => x < 26.0,
                };
                if self.prefs.tous && etoile {
                    Click::Favori(coin)
                } else if on_x {
                    Click::Remove(coin)
                } else if x < 22.0 && !self.prefs.tous {
                    self.picker = Some(coin); // ouvre le choix de couleur
                    Click::Changed
                } else {
                    self.selected = coin.clone();
                    Click::Open(coin)
                }
            }
        }
    }

    /// 🖥 Volet flottant (PC) : le volet ouvert seul (`pw` × `h`), en verre arrondi, avec son ✕ ; la fenêtre le pose à
    /// sa place (sous la 1re rangée, à gauche de la colonne d'icônes). `None` : volet fermé.
    pub fn draw_volet(&self, text: &Text, h: u32, mouse: Option<(f64, f64)>, th: &Theme) -> Option<Pixmap> {
        if !self.prefs.open {
            return None;
        }
        let mut pm = crate::px::new(self.pw().max(1.0) as u32, h.max(1))?;
        fill(&mut pm, 0.0, 0.0, self.pw(), h as f64, th.chrome, 1.0);
        self.peindre_volet(&mut pm, text, h as f64, mouse, th);
        Some(pm)
    }

    /// Le volet ouvert (contenu, carte en verre, ✕) dans `pm`, sur la hauteur `hf`.
    fn peindre_volet(&self, pm: &mut Pixmap, text: &Text, hf: f64, mouse: Option<(f64, f64)>, th: &Theme) {
        if let (true, Some(ch)) = (self.prefs.open, self.panel_h()) {
            // panneau dessiné sur toute sa hauteur, puis montré à partir du défilement
            let (ch, off) = (ch.max(hf), self.panel_off(hf));
            if let Some(mut sub) = crate::px::new(self.pw() as u32, ch as u32) {
                fill(&mut sub, 0.0, 0.0, self.pw(), ch, th.chrome, 1.0);
                let m = mouse.filter(|(x, _)| *x < self.pw()).map(|(x, y)| (x, y + off));
                match self.prefs.panel.panel() {
                    Some(k) => self.panels.draw(k, &mut sub, text, (self.pw(), ch), m, th),
                    None if self.prefs.panel == Tool::Market => crate::cmc::draw(&self.market, &mut sub, text, self.pw(), FERME_PLACE, th),
                    None if self.prefs.panel == Tool::Agenda => {
                        self.eco
                            .draw(crate::eco::EcoKind::Agenda, &mut sub, text, self.pw(), self.now, m, th, self.whales.tz_offset_s)
                    }
                    None if self.prefs.panel == Tool::News => {
                        self.eco
                            .draw(crate::eco::EcoKind::News, &mut sub, text, self.pw(), self.now, m, th, self.whales.tz_offset_s)
                    }
                    None if self.prefs.panel == Tool::Whales => self.whales.draw(&mut sub, text, self.pw(), self.now * 1000, m, th),
                    None if self.prefs.panel == Tool::Explorer => self.explorer.draw(&mut sub, text, self.pw(), m, th.background),
                    None if self.prefs.panel == Tool::Stats => self.stats.draw(&mut sub, text, self.pw(), m, th.background),
                    None if self.prefs.panel == Tool::Modules => crate::dock::draw_settings(&self.dock, &mut sub, text, self.pw(), self.dock_big, m, th),
                    None if self.prefs.panel == Tool::Strategie => self.strategie.draw(&mut sub, text, self.pw(), m, self.phone.is_some()),
                    None if self.prefs.panel == Tool::Carnet => {
                        // (📱 its bottom left to the round Argos AI's band, drawn below)
                        let hc = ch - self.bande_ai(hf).h;
                        crate::carnet::draw(&mut sub, text, (self.pw(), hc), &self.prefs.carnet, &self.carnet, m, self.phone.is_some(), true)
                    }
                    None => crate::exchanges::draw(&self.exchanges, &mut sub, text, self.pw(), m, th.grid),
                }
                crate::px::put(pm, 0.0, -(off.round()), &sub);
                // 📶📱 Stats Hyperliquid: the bottom of the sheet keeps the room of the round Argos AI button (laid over
                // its bottom right) at every scroll, the rows scroll above it (capture 202611222137-15: it covered the
                // last row's « 24 h »)
                let ai = self.bande_ai(hf);
                if ai.h > 0.0 {
                    fill(pm, ai.x, ai.y, ai.w, ai.h, th.chrome, 1.0);
                }
                // 📰 the Actualités' bottom stays in view, the list scrolls above it
                if self.prefs.panel == Tool::News {
                    self.eco.draw_pied(crate::eco::EcoKind::News, pm, text, self.pw(), hf, self.now, th);
                }
                // 📌📱 Trading sheet: the header at the top, VENDRE / ACHETER at the bottom of the phone (in place)
                if self.prefs.panel == Tool::Trading {
                    let m = mouse.filter(|(x, _)| *x < self.pw());
                    self.panels.trading_fixe_draw(pm, text, (self.pw(), hf), m, th.chrome);
                }
            }
            // petite barre de défilement à droite quand ça ne tient pas
            if ch > hf {
                let th_h = (hf * hf / ch).max(30.0);
                let ty = off / (ch - hf) * (hf - th_h);
                crate::jetons::barre_defilement(pm, self.pw(), ty + 2.0, th_h - 4.0, false);
            }
        } else if self.prefs.open {
            self.draw_panel(pm, text, hf, mouse, th);
        }
        // 🫧 volet ouvert : une carte en verre arrondie (📱 sur cell : plein écran, sans marge)
        if self.prefs.open && self.phone.is_none() {
            let (tout, carte) = (Rect::new(0.0, 0.0, self.pw(), hf), Rect::new(4.0, 4.0, self.pw() - 6.0, (hf - 8.0).max(1.0)));
            crate::shapes::autour(pm, tout, carte, crate::jetons::R_VOLET, crate::glass::hors_verre(th.chrome));
            round_stroke(pm, carte, crate::jetons::R_VOLET, crate::jetons::sur_fond(crate::jetons::LISERE), 1.0);
        }
        // 🖥 ✕ en haut à droite du volet
        if let Some(r) = self.fermer_rect() {
            let sur = mouse.is_some_and(|(x, y)| r.contains(x, y));
            // (maquette `.x` : rond de 30, fond var(--verre2), l.187)
            let (c, a) = if sur { (pal::hover(), 1.0) } else { crate::jetons::VERRE2 };
            round_fill(pm, r, r.h / 2.0, c, a);
            let t = if sur { WHITE } else { crate::jetons::TEXTE };
            text.draw(pm, "✕", (r.cx() as f32, (r.cy() + 5.0) as f32), 14.0, t, Align::Center);
        }
        // 🖥 ⤢ et ⬓ à sa gauche, mêmes ronds
        for (r, g) in [(self.agrandir_rect(), "⤢"), (self.ancrer_rect(), "⬓")] {
            let Some(r) = r else { continue };
            let sur = mouse.is_some_and(|(x, y)| r.contains(x, y));
            let (c, a) = if sur { (pal::hover(), 1.0) } else { crate::jetons::VERRE2 };
            round_fill(pm, r, r.h / 2.0, c, a);
            let t = if sur { WHITE } else { crate::jetons::TEXTE };
            if g == "⤢" {
                // (⤢ n'est pas dans la police : deux flèches en diagonale, dessinées ; vers le centre quand il est large)
                let k: f32 = if self.large_w.is_some() { -1.0 } else { 1.0 };
                crate::shapes::lines(
                    pm,
                    r.cx(),
                    r.cy(),
                    &[
                        &[(-5.0, 5.0), (5.0, -5.0)],
                        &[(k, -5.0 * k), (5.0 * k, -5.0 * k), (5.0 * k, -k)],
                        &[(-5.0 * k, k), (-5.0 * k, 5.0 * k), (-k, 5.0 * k)],
                    ],
                    t,
                    1.3,
                );
            } else {
                text.draw(pm, g, (r.cx() as f32, (r.cy() + 5.0) as f32), 14.0, t, Align::Center);
            }
        }
    }

    /// Dessine la watchlist (largeur `width()`, hauteur `h`). `mouse` : position dans cette image.
    pub fn draw(&self, text: &Text, h: u32, mouse: Option<(f64, f64)>, th: &Theme) -> Pixmap {
        let w = self.width();
        let mut pm = crate::px::new(w.max(1), h.max(1)).expect("taille de la watchlist");
        let hf = h as f64;
        let bar_x = if self.prefs.open { self.pw() } else { 0.0 };
        fill(&mut pm, 0.0, 0.0, w as f64, hf, th.chrome, 1.0);
        // 🖥 volet flottant (PC) : dessiné à part (`draw_volet`), posé sous la 1re rangée par la fenêtre
        if !self.volet_flottant {
            self.peindre_volet(&mut pm, text, hf, mouse, th);
        }
        // barre d'icônes : tous les outils, toujours (📱 pas sur cell)
        if self.phone.is_some() {
            // 📱 feuille du bas en verre : coins du haut arrondis et poignée (maquette tel-04)
            if self.prefs.open {
                // (le cadre extérieur couvre toute la feuille : avec la règle pair-impair, un cadre plus petit que la carte
                // repeignait tout le contenu sous 30 points — feuille vide sur cell)
                let (tout, carte) = (Rect::new(0.0, 0.0, w as f64, hf), Rect::new(0.0, 0.0, w as f64, hf + 40.0));
                crate::shapes::autour(&mut pm, tout, carte, crate::jetons::R_CARTE, crate::glass::hors_verre(th.chrome));
                round_stroke(&mut pm, carte, crate::jetons::R_CARTE, crate::jetons::sur_fond(crate::jetons::LISERE), 1.0);
                round_fill(&mut pm, Rect::new(w as f64 / 2.0 - 18.0, 7.0, 36.0, 4.0), 2.0, crate::jetons::DIM, 0.7);
            }
            return pm;
        }
        if self.colonne.is_some() {
            // 🖥 la colonne est dessinée à part, sur toute la hauteur de la fenêtre (draw_colonne)
            return pm;
        }
        // 🚫 colonne d'icônes : no frame around it (Jee's rule, 16 Nov; it overrides the maquette's glass card): the
        // app's background, and the icons' pills laid directly on it
        fill(&mut pm, bar_x, 0.0, ICON_BAR as f64, hf, th.chrome, 1.0);
        for (i, t) in self.bar_tools().iter().enumerate() {
            let ir = self.tool_rect(i);
            let on = self.tool_on(*t);
            if !on && mouse.is_some_and(|(x, y)| ir.contains(x, y)) {
                round_fill(&mut pm, ir, crate::jetons::pilule(ir), crate::jetons::VERRE2.0, crate::jetons::VERRE2.1);
            }
            // (no line between the groups: the gap left in their place stays)
            icone_outil(&mut pm, *t, ir, on);
        }
        pm
    }

    /// 🖥 La colonne d'icônes de la maquette PC, en verre, du haut au bas de la fenêtre : son image et sa place
    /// (coordonnées de l'image de la watchlist, le haut à `-haut`). `None` sans colonne.
    pub fn draw_colonne(&self, text: &Text, mouse: Option<(f64, f64)>, th: &Theme) -> Option<(Pixmap, Rect)> {
        // 🖥 repliée : la bande de 12 en verre et sa poignée (comme le rail de gauche)
        if let Some(b) = self.col_bande() {
            let mut pm = crate::px::new(b.w as u32, b.h.max(1.0) as u32)?;
            dessiner_bande(&mut pm, b.w, b.h, th);
            return Some((pm, b));
        }
        let c = self.col_rect(self.col_h)?;
        let mut pm = crate::px::new(c.w as u32, c.h.max(1.0) as u32)?;
        let loc = |r: Rect| Rect::new(r.x - c.x, r.y - c.y, r.w, r.h);
        // 🫧 opened, the column keeps its folded band's shape (Jee, 19 Nov): the same long pill (round ends, glass fill,
        // light outline), only wider; the icons' pills inside it (the window pastes this image as it is)
        pilule_bande(&mut pm, c.w, c.h, th);
        let survol = |r: Rect| mouse.is_some_and(|(x, y)| r.contains(x, y));
        for (i, t) in self.bar_tools().iter().enumerate() {
            let ir = self.tool_rect(i);
            let on = self.tool_on(*t);
            if !on && survol(ir) {
                round_fill(
                    &mut pm,
                    loc(ir),
                    crate::jetons::pilule(loc(ir)),
                    crate::jetons::VERRE2.0,
                    crate::jetons::VERRE2.1,
                );
            }
            // 🖥 l'icône de la maquette (émoji, comme la barre du bas du cell)
            let b = loc(ir);
            if on {
                round_fill(&mut pm, b, crate::jetons::pilule(b), crate::jetons::PUCE_ON.0, crate::jetons::PUCE_ON.1);
            }
            let tp = crate::jetons::T_EMOJI_COLONNE;
            text.draw(
                &mut pm,
                t.emoji(),
                (b.cx() as f32, (b.cy() + tp as f64 * 0.36) as f32),
                tp,
                crate::jetons::TEXTE,
                Align::Center,
            );
        }
        for (r, k) in self.col_boutons(self.col_h).into_iter().flatten() {
            if survol(r) {
                round_fill(&mut pm, loc(r), crate::jetons::pilule(loc(r)), crate::jetons::VERRE2.0, crate::jetons::VERRE2.1);
            }
            let l = loc(r);
            if k == Click::ReplierBarre {
                lines(
                    &mut pm,
                    l.cx(),
                    l.cy(),
                    if self.col_repliee {
                        &[&[(2.5, -5.0), (-2.5, 0.0), (2.5, 5.0)]]
                    } else {
                        &[&[(-2.5, -5.0), (2.5, 0.0), (-2.5, 5.0)]]
                    },
                    crate::jetons::DIM,
                    1.6,
                );
            } else {
                crate::phosphor::draw(&mut pm, "gear", l.cx(), l.cy(), 18, crate::jetons::DIM);
            }
        }
        Some((pm, c))
    }

    fn draw_panel(&self, pm: &mut Pixmap, text: &Text, h: f64, mouse: Option<(f64, f64)>, th: &Theme) {
        let cols = self.cols();
        let pw = self.pw();
        let white = WHITE;
        let grey = GREY;
        let hovered = mouse.filter(|(x, _)| *x < pw);
        // lignes (dessinées d'abord ; l'en-tête et le pied passent par-dessus)
        for (top, line) in self.placed() {
            if top + self.line_h(line) < self.haut_liste() || top > h - self.foot_h() {
                continue;
            }
            match line {
                Line::Section(s) => {
                    let sec = &self.sections[s];
                    let closed = self.prefs.collapsed.contains(&sec.name);
                    let chev: [(f32, f32); 3] = if closed {
                        [(-2.0, -4.0), (2.0, 0.0), (-2.0, 4.0)]
                    } else {
                        [(-4.0, -2.0), (0.0, 2.0), (4.0, -2.0)]
                    };
                    if self.phone.is_some() {
                        // 📱 en-tête de section des photos : texte gras gris de 13 points à x 17, et ⌄ à droite
                        let b = crate::ecrans::boite_de("wl-section").unwrap_or([17.0, 0.0, 356.0, 40.0]);
                        let label = format!("{} · {}", sec.name, sec.rows.len());
                        Text::graisse(Graisse::Bold).draw(pm, &label, (b[0] as f32, (top + b[3] / 2.0 + 4.7) as f32), 13.0, grey, Align::Left);
                        lines(pm, pw - 24.0, top + b[3] / 2.0, &[&chev], grey, 1.4);
                    } else {
                        lines(pm, 30.0, top + SEC_H / 2.0, &[&chev], grey, 1.4);
                        let label = format!("{}  ·  {}", sec.name.to_uppercase(), sec.rows.len());
                        text.draw(pm, &label, (46.0, (top + 20.0) as f32), 11.5, grey, Align::Left);
                    }
                    if self.editable && !sec.name.is_empty() && hovered.is_some_and(|(_, y)| y >= top && y < top + SEC_H) {
                        cross(pm, self.remove_rect(top - (self.row_h() - SEC_H) / 2.0), hovered);
                    }
                }
                Line::Row(s, r) => {
                    let row = &self.sections[s].rows[r];
                    let rect = Rect::new(2.0, top + 1.0, pw - 8.0, self.row_h() - 2.0);
                    let hover = hovered.is_some_and(|(x, y)| rect.contains(x, y));
                    if hover {
                        fill(pm, 0.0, top, pw, self.row_h(), pal::card(), 1.0);
                    }
                    if self.phone.is_some() {
                        // 📱 le symbole ouvert : fond de verre léger (pas de cadre)
                        if row.coin == self.selected {
                            fill(pm, 0.0, top, pw, self.row_h(), crate::jetons::VERRE2.0, crate::jetons::VERRE2.1);
                        }
                    } else {
                        if row.coin == self.selected {
                            round_stroke(pm, rect, 6.0, TEXT, 1.2);
                        }
                        fill(pm, 0.0, top + self.row_h() - 1.0, pw, 1.0, th.grid, 1.0);
                    }
                    self.draw_row(pm, text, row, top, hover, th);
                    if hover && self.editable {
                        let xr = self.remove_rect(top);
                        fill(pm, xr.x - 6.0, top + 1.0, pw - xr.x, self.row_h() - 2.0, pal::card(), 1.0);
                        cross(pm, xr, hovered);
                    }
                }
            }
        }
        if self.sections.iter().all(|s| s.rows.is_empty()) {
            // (🌐 « Tous les marchés » : sa note dit ce qui se passe, chargement ou erreur de la plateforme)
            let msg = if self.editable {
                "Liste vide : clique + pour ajouter un symbole."
            } else if self.prefs.tous && !self.note.is_empty() {
                self.note.as_str()
            } else {
                "Chargement…"
            };
            text.draw(
                pm,
                &text.fit(msg, 13.0, pw - 24.0),
                ((pw / 2.0) as f32, (self.haut_liste() + 40.0 + self.sections.len() as f64 * SEC_H) as f32),
                13.0,
                grey,
                Align::Center,
            );
        }
        // choix de couleur du drapeau
        if let Some((coin, p)) = self.picker.as_ref().and_then(|c| Some((c, self.picker_rect(c)?))) {
            round_fill(pm, p, p.h / 2.0, pal::hover(), 1.0);
            let current = self.flag(coin);
            for (k, c) in FLAG_COLORS.iter().enumerate() {
                let (cx, cy) = (p.x + 6.0 + (k as f64 + 0.5) * DOT, p.cy());
                if current == Some(k) {
                    // couleur choisie : un anneau autour
                    round_stroke(pm, Rect::new(cx - 9.0, cy - 9.0, 18.0, 18.0), 9.0, *c, 1.6);
                    round_fill(pm, Rect::new(cx - 5.0, cy - 5.0, 10.0, 10.0), 5.0, *c, 1.0);
                } else {
                    round_fill(pm, Rect::new(cx - 8.0, cy - 8.0, 16.0, 16.0), 8.0, *c, 1.0);
                }
            }
        }
        // barre de défilement (plus large et plus claire au survol ou attrapée)
        if let Some((track, thumb)) = self.scrollbar(h) {
            let active = self.bar_grab.is_some() || hovered.is_some_and(|(x, y)| track.contains(x, y));
            crate::jetons::barre_defilement(pm, thumb.x + thumb.w + 2.0, thumb.y, thumb.h, active);
        }
        // en-tête
        if self.phone.is_some() {
            self.entete_cell_draw(pm, text, mouse, th);
        } else {
            fill(pm, 0.0, 0.0, pw, self.haut_liste(), th.chrome, 1.0);
            // nom de la liste ⌄ (menu des listes) · note grise · ＋ à droite
            let tr = self.title_rect();
            let flag_dot = self.prefs.flag_filter.map(|k| FLAG_COLORS[k % FLAG_COLORS.len()]);
            if hovered.is_some_and(|(x, y)| tr.contains(x, y)) {
                let w = text.width(&self.title, 16.0) as f64 + 40.0 + if flag_dot.is_some() { 18.0 } else { 0.0 };
                round_fill(pm, Rect::new(tr.x, tr.y, w.min(tr.w), tr.h), 6.0, pal::hover(), 1.0);
            }
            text.draw(pm, &self.title, (14.0, 29.0), 16.0, white, Align::Left);
            let mut nx = 14.0 + text.width(&self.title, 16.0) as f64 + 8.0;
            if let Some(c) = flag_dot {
                round_fill(pm, Rect::new(nx, 18.0, 10.0, 10.0), 5.0, c, 1.0);
                nx += 16.0;
            }
            lines(pm, nx + 4.0, 23.0, &[&[(-3.5, -1.5), (0.0, 2.0), (3.5, -1.5)]], grey, 1.3);
            // le nombre de symboles seulement s'il tient avant 🔌
            if nx + 16.0 + text.width(&self.note, 12.5) as f64 <= self.api_rect().x - 6.0 {
                text.draw(pm, &self.note, ((nx + 16.0) as f32, 29.0), 12.5, grey, Align::Left);
            }
            let ar = self.add_rect();
            if hovered.is_some_and(|(x, y)| ar.contains(x, y)) {
                round_fill(pm, ar, 6.0, pal::hover(), 1.0);
            }
            lines(pm, ar.cx(), ar.cy(), &[&[(-7.0, 0.0), (7.0, 0.0)], &[(0.0, -7.0), (0.0, 7.0)]], white, 1.6);
            let pr = self.api_rect();
            if hovered.is_some_and(|(x, y)| pr.contains(x, y)) {
                round_fill(pm, pr, 6.0, pal::hover(), 1.0);
            }
            Tool::Exchanges.icon(pm, pr.cx(), pr.cy(), white);
            fill(pm, 0.0, self.haut_liste() - 1.0, pw, 1.0, th.grid, 1.0);
        }
        self.pilules_tous_draw(pm, text, mouse);
        let head_y = (TOP + self.bande_tous() + 20.0) as f32;
        // (📱 cell : pas de titres de colonnes, comme les photos de Jee)
        let sans_chg = self.sans_chg();
        for col in [Col::Symbol, Col::Last, Col::Chg, Col::Pct]
            .into_iter()
            .filter(|c| self.phone.is_none() && !(*c == Col::Chg && sans_chg))
        {
            let arrow = match self.prefs.sort.filter(|_| !self.prefs.tous) {
                Some((c, asc)) if c == col => {
                    if asc {
                        " ↑"
                    } else {
                        " ↓"
                    }
                }
                _ => "",
            };
            let label = format!("{}{arrow}", col.name());
            match col {
                Col::Symbol => text.draw(pm, &label, (30.0, head_y), 13.0, grey, Align::Left),
                Col::Last => text.draw(pm, &label, (cols[0] as f32, head_y), 13.0, grey, Align::Right),
                Col::Chg => text.draw(pm, &label, (cols[1] as f32, head_y), 13.0, grey, Align::Right),
                Col::Pct => text.draw(pm, &label, (cols[2] as f32, head_y), 13.0, grey, Align::Right),
            }
        }
        // 📱 pied du cell : la mini-pilule du symbole ouvert (photos de Jee) à la place de la fiche
        if self.phone.is_some() {
            fill(pm, 0.0, h - self.foot_h(), pw, self.foot_h(), th.chrome, 1.0);
            self.mini_cell_draw(pm, text, h, th);
            fill(pm, 0.0, 0.0, 1.0, h, th.grid, 1.0);
            return;
        }
        // pied : symbole ouvert
        fill(pm, 0.0, h - self.foot_h(), pw, self.foot_h(), th.chrome, 1.0);
        fill(pm, 0.0, h - self.foot_h(), pw, 1.0, th.grid, 1.0);
        if let Some(row) = self.sections.iter().flat_map(|s| &s.rows).find(|r| r.coin == self.selected) {
            let mid = h - self.foot_h() + FOOT_H / 2.0;
            coin_badge(pm, text, &row.coin, self.logos.get(&row.coin), 26.0, mid, 12.0);
            text.draw(pm, &row.label, (46.0, (mid + 6.0) as f32), 16.0, white, Align::Left);
        }
        // 📱 fiche du symbole : 3 colonnes × 2 rangées (nom gris souligné, chiffre dessous)
        if self.phone.is_some() {
            let (cw, y0) = ((pw - 28.0) / 3.0, h - FICHE_H);
            for (k, st) in self.fiche.iter().take(6).enumerate() {
                let (x, y) = (14.0 + (k % 3) as f64 * cw, y0 + (k / 3) as f64 * 44.0);
                text.draw(pm, &text.fit(&st.label, 11.0, cw - 6.0), (x as f32, (y + 14.0) as f32), 11.0, grey, Align::Left);
                text.draw(
                    pm,
                    &text.fit(&st.value, 13.0, cw - 6.0),
                    (x as f32, (y + 32.0) as f32),
                    13.0,
                    st.color.unwrap_or(white),
                    Align::Left,
                );
            }
        }
        // bulle « Cliquez pour trier par … ↑ » au survol de l'en-tête
        if let Some((x, y)) = hovered.filter(|(_, y)| !self.prefs.tous && (TOP..self.haut_liste()).contains(y)) {
            let col = self.header_col(x);
            let tip = format!("Cliquez pour trier par {} ↑", col.name());
            let tw = text.width(&tip, 12.0) as f64 + 16.0;
            let bx = (x - tw / 2.0).clamp(2.0, pw - tw - 2.0);
            let by = (y - 34.0).max(2.0);
            round_fill(pm, Rect::new(bx, by, tw, 24.0), 12.0, pal::hover(), 1.0);
            text.draw(pm, &tip, ((bx + 8.0) as f32, (by + 17.0) as f32), 12.0, white, Align::Left);
        }
        // (🖥 volet flottant : sa carte arrondie suffit, pas de trait droit au bord)
        if !self.volet_flottant {
            fill(pm, 0.0, 0.0, 1.0, h, th.grid, 1.0);
        }
    }

    fn draw_row(&self, pm: &mut Pixmap, text: &Text, row: &Row, top: f64, hover: bool, th: &Theme) {
        if self.phone.is_some() {
            return self.draw_row_cell(pm, text, row, top, th);
        }
        let cols = self.cols();
        let base = (top + 23.0) as f32;
        // drapeau vert à gauche (fanion, comme TradingView) ; au survol, son contour pour montrer où cliquer
        let flag: [(f32, f32); 5] = [(0.0, -8.0), (14.0, -8.0), (8.0, 0.0), (14.0, 8.0), (0.0, 8.0)];
        if self.prefs.tous {
            // 🌐 ☆ : ajouter à « Mes favoris » (★ jaune : déjà dedans)
            let fav = self.favoris.contains(&row.coin);
            let (nom, c) = if fav {
                ("star-fill", FLAG_COLORS[3])
            } else {
                ("star", if hover { WHITE } else { MUTED })
            };
            crate::phosphor::draw(pm, nom, 12.0, top + self.row_h() / 2.0, 16, c);
        } else if let Some(k) = self.flag(&row.coin) {
            poly_fill(pm, 2.0, top + self.row_h() / 2.0, &flag, FLAG_COLORS[k % FLAG_COLORS.len()], 1.0);
        } else if hover {
            let outline = [flag[0], flag[1], flag[2], flag[3], flag[4], flag[0]];
            lines(pm, 2.0, top + self.row_h() / 2.0, &[&outline], MUTED, 1.2);
        }
        coin_badge(pm, text, &row.coin, self.logos.get(&row.coin), 32.0, top + self.row_h() / 2.0, 10.0);
        let white = WHITE;
        // le nom ne déborde jamais sur le prix : coupé avec « … » ; la note seulement s'il reste de la place
        let price_w = if row.mark.is_empty() {
            14.0
        } else {
            text.width(&format_price(row.price(), row.decimals()), 13.5) as f64
        };
        let room = cols[0] - price_w - 10.0 - 48.0;
        let name = text.fit(&row.label, 13.5, room);
        text.draw(pm, &name, (48.0, base), 13.5, white, Align::Left);
        let nw = text.width(&name, 13.5) as f64 + 6.0;
        if !row.note.is_empty() && name == row.label && (text.width(&row.note, 11.0) as f64) <= room - nw {
            text.draw(pm, &row.note, ((48.0 + nw) as f32, base), 11.0, GREY, Align::Left);
        }
        if row.mark.is_empty() {
            // pas encore de prix pour cet exchange
            for x in cols {
                text.draw(pm, "—", (x as f32, base), 13.5, MUTED, Align::Right);
            }
            return;
        }
        let dec = row.decimals();
        let now = format_price(row.price(), dec);
        let before = format_price(row.last_mark.parse().unwrap_or(0.0), dec);
        // chiffres qui viennent de changer : en vert (hausse) ou rouge (baisse)
        let same = now.chars().zip(before.chars()).take_while(|(a, b)| a == b).count();
        let (keep, changed): (String, String) = (now.chars().take(same).collect(), now.chars().skip(same).collect());
        let move_col = if row.up { th.bull } else { th.bear };
        let wc = text.width(&changed, 13.5);
        text.draw(pm, &changed, (cols[0] as f32, base), 13.5, move_col, Align::Right);
        text.draw(pm, &keep, (cols[0] as f32 - wc, base), 13.5, white, Align::Right);
        let Some((chg, pct)) = row.change() else {
            // 📈 no price of 24 h ago from this exchange: « — », not a change made up
            if cols[1] > cols[0] {
                text.draw(pm, "—", (cols[1] as f32, base), 13.5, MUTED, Align::Right);
            }
            text.draw(pm, "—", (cols[2] as f32, base), 13.5, MUTED, Align::Right);
            return;
        };
        let col = if chg >= 0.0 { th.bull } else { th.bear };
        let sign = |v: f64| if v < 0.0 { "−" } else { "" };
        // variation : moins de décimales quand le nombre est grand (sinon « 2,745.380 » mord sur le prix) ; plus
        // petit s'il ne tient toujours pas dans sa colonne
        let chg_t = format!("{}{}", sign(chg), format_price(chg.abs(), chg_decimals(chg, dec)));
        if cols[1] > cols[0] {
            let room = (cols[1] - cols[0]) - 8.0;
            let size = if text.width(&chg_t, 13.5) as f64 > room { 11.5 } else { 13.5 };
            text.draw(pm, &chg_t, (cols[1] as f32, base), size, col, Align::Right);
        }
        let pct_t = format!("{}{}%", sign(pct), format_price(pct.abs(), 2));
        text.draw(pm, &pct_t, (cols[2] as f32, base), 13.5, col, Align::Right);
    }
}

impl Watchlist {
    /// 📱 L'en-tête du cell (photos de Jee) : ⋯ et 🔎 en boutons ronds de verre, le titre de la liste au centre en
    /// gras ; dessous, les listes en pilules (l'ouverte en clair).
    fn entete_cell_draw(&self, pm: &mut Pixmap, text: &Text, mouse: Option<(f64, f64)>, th: &Theme) {
        let pw = self.pw();
        fill(pm, 0.0, 0.0, pw, self.haut_liste(), th.chrome, 1.0);
        let gras = Text::graisse(Graisse::Bold);
        let (dots, titre, loupe) = self.entete_cell();
        for r in [dots, loupe] {
            crate::jetons::verre(pm, r, r.h / 2.0);
            if mouse.is_some_and(|(x, y)| r.contains(x, y)) {
                round_fill(pm, r, r.h / 2.0, crate::jetons::VERRE2.0, crate::jetons::VERRE2.1);
            }
        }
        for k in [-7.0, 0.0, 7.0] {
            round_fill(pm, Rect::new(dots.cx() + k - 2.0, dots.cy() - 2.0, 4.0, 4.0), 2.0, WHITE, 1.0);
        }
        crate::phosphor::draw(pm, "magnifying-glass", loupe.cx(), loupe.cy(), 20, WHITE);
        let t = gras.fit(&self.title, 17.0, titre.w - 8.0);
        gras.draw(pm, &t, (titre.cx() as f32, (titre.cy() + 6.0) as f32), 17.0, WHITE, Align::Center);
        for (r, id, nom) in self.pilules_cell(text) {
            if id == self.liste_active {
                round_fill(pm, r, r.h / 2.0, WHITE, 1.0);
                text.draw(pm, &nom, (r.cx() as f32, (r.cy() + 5.4) as f32), 15.0, Rgb(0, 0, 0), Align::Center);
            } else {
                crate::jetons::verre(pm, r, r.h / 2.0);
                text.draw(pm, &nom, (r.cx() as f32, (r.cy() + 5.4) as f32), 15.0, WHITE, Align::Center);
            }
        }
    }

    /// 📱 La mini-pilule : logo rond, « SYMBOLE prix » en gras, « +x % aujourd'hui » en couleur, ∧ à droite.
    fn mini_cell_draw(&self, pm: &mut Pixmap, text: &Text, h: f64, th: &Theme) {
        let r = self.mini_cell(h);
        crate::jetons::verre(pm, r, r.h / 2.0);
        let Some(row) = self.sections.iter().flat_map(|s| &s.rows).find(|x| x.coin == self.selected) else {
            return;
        };
        let gras = Text::graisse(Graisse::Bold);
        coin_badge(pm, text, &row.coin, self.logos.get(&row.coin), r.x + 8.0 + 18.5, r.cy(), 18.5);
        let x = r.x + 8.0 + 37.0 + 10.0;
        let prix = if row.mark.is_empty() {
            String::new()
        } else {
            format_price(row.price(), row.decimals())
        };
        let haut = gras.fit(&format!("{} {}", row.label, prix), 15.0, r.w - (x - r.x) - 44.0);
        gras.draw(pm, &haut, (x as f32, (r.y + 23.0) as f32), 15.0, WHITE, Align::Left);
        if !row.mark.is_empty() {
            let (t, c) = match row.variation() {
                Some(pct) => {
                    let sign = if pct < 0.0 { "−" } else { "+" };
                    (
                        format!("{sign}{} % aujourd'hui", format_price(pct.abs(), 2)),
                        if pct >= 0.0 { th.bull } else { th.bear },
                    )
                }
                // 📈 no price of 24 h ago from this exchange
                None => ("—".to_string(), MUTED),
            };
            text.draw(pm, &t, (x as f32, (r.y + 42.0) as f32), 13.0, c, Align::Left);
        }
        lines(pm, r.x + r.w - 26.0, r.cy(), &[&[(-6.0, 3.0), (0.0, -3.0), (6.0, 3.0)]], WHITE, 1.6);
    }

    /// 📱 Une ligne sur cell, comme les photos de Jee (TradingView mobile, mesures dans ecrans/maquette-cell.json) :
    /// logo rond, SYMBOLE en gras et son nom gris dessous ; à droite le prix en gras et, dessous, la variation et le
    /// pourcentage en vert ou rouge ; un fin séparateur.
    fn draw_row_cell(&self, pm: &mut Pixmap, text: &Text, row: &Row, top: f64, th: &Theme) {
        let b = |k: usize| crate::ecrans::boite_de(&format!("wl-ligne.{k}")).unwrap_or([0.0; 4]);
        let (logo, sym, nom, prix, var, sep) = (b(0), b(1), b(2), b(3), b(4), b(5));
        let gras = Text::graisse(Graisse::Bold);
        let mut droite = (self.pw() - (390.0 - (prix[0] + prix[2]))).max(prix[0]);
        if self.prefs.tous {
            // 🌐 ☆ à droite (au doigt), le prix juste à sa gauche
            let e = self.etoile_cell(top);
            let fav = self.favoris.contains(&row.coin);
            let (nom, c) = if fav { ("star-fill", FLAG_COLORS[3]) } else { ("star", GREY) };
            crate::phosphor::draw(pm, nom, e.cx(), e.cy(), 20, c);
            droite = e.x - 4.0;
        }
        if let Some(k) = self.flag(&row.coin) {
            // petite marque de couleur au bord gauche (drapeau de la liste)
            round_fill(
                pm,
                Rect::new(0.0, top + 14.0, 3.0, self.row_h() - 28.0),
                1.5,
                FLAG_COLORS[k % FLAG_COLORS.len()],
                1.0,
            );
        }
        coin_badge(
            pm,
            text,
            &row.coin,
            self.logos.get(&row.coin),
            logo[0] + logo[2] / 2.0,
            top + logo[1] + logo[3] / 2.0,
            logo[2] / 2.0,
        );
        let base = |z: [f64; 4], t: f64| (top + z[1] + z[3] / 2.0 + t * 0.36) as f32;
        let (ts, tn, tp, tv) = (17.0f32, 13.0f32, 17.0f32, 13.0f32);
        // le prix d'abord : le symbole et le nom s'arrêtent avant lui
        let (prix_t, chg_t, col) = match (row.mark.is_empty(), row.change()) {
            (true, _) => ("—".to_string(), String::new(), GREY),
            // 📈 no price of 24 h ago from this exchange: « — », not a change made up
            (false, None) => (format_price(row.price(), row.decimals()), "—".to_string(), GREY),
            (false, Some((chg, pct))) => {
                let dec = row.decimals();
                let sign = |v: f64| if v < 0.0 { "−" } else { "+" };
                let chg_s = format!("{}{}", sign(chg), format_price(chg.abs(), chg_decimals(chg, dec)));
                let pct_s = format!("{}{} %", sign(pct), format_price(pct.abs(), 2));
                (
                    format_price(row.price(), dec),
                    format!("{chg_s}  {pct_s}"),
                    if chg >= 0.0 { th.bull } else { th.bear },
                )
            }
        };
        let pw_ = gras.width(&prix_t, tp) as f64;
        let room = (droite - pw_ - 10.0 - sym[0]).max(40.0);
        gras.draw(
            pm,
            &gras.fit(&row.label, ts, room),
            (sym[0] as f32, base(sym, ts as f64)),
            ts,
            WHITE,
            Align::Left,
        );
        let desc = if row.note.is_empty() { self.title.clone() } else { row.note.clone() };
        let room_n = (droite - text.width(&chg_t, tv) as f64 - 10.0 - nom[0]).max(40.0);
        text.draw(pm, &text.fit(&desc, tn, room_n), (nom[0] as f32, base(nom, tn as f64)), tn, GREY, Align::Left);
        gras.draw(pm, &prix_t, (droite as f32, base(prix, tp as f64)), tp, WHITE, Align::Right);
        text.draw(pm, &chg_t, (droite as f32, base(var, tv as f64)), tv, col, Align::Right);
        crate::fill(
            pm,
            sep[0],
            top + sep[1],
            (self.pw() - sep[0] - (390.0 - sep[0] - sep[2])).max(0.0),
            sep[3],
            th.grid,
            1.0,
        );
    }
}

/// Décimales de la variation : celles du prix, mais au plus 0 au-delà de 1000, 1 au-delà de 100, 2 au-delà de 1.
pub fn chg_decimals(chg: f64, price_dec: usize) -> usize {
    let a = chg.abs();
    let max = if a >= 1000.0 {
        0
    } else if a >= 100.0 {
        1
    } else if a >= 1.0 {
        2
    } else {
        price_dec
    };
    price_dec.min(max)
}

/// ✕ dans un carré arrondi (plus clair au survol).
fn cross(pm: &mut Pixmap, r: Rect, mouse: Option<(f64, f64)>) {
    if mouse.is_some_and(|(x, y)| r.contains(x, y)) {
        round_fill(pm, r, 4.0, pal::hover(), 1.0);
    }
    lines(pm, r.cx(), r.cy(), &[&[(-4.5, -4.5), (4.5, 4.5)], &[(4.5, -4.5), (-4.5, 4.5)]], TEXT, 1.4);
}

fn toggle(list: &mut Vec<String>, v: String) {
    match list.iter().position(|x| *x == v) {
        Some(i) => {
            list.remove(i);
        }
        None => list.push(v),
    }
}

/// Logo du symbole (réduit au diamètre `2r`) ; sans logo : pastille ronde avec sa première lettre (couleur tirée du nom).
pub(crate) fn coin_badge(pm: &mut Pixmap, text: &Text, coin: &str, logo: Option<&Pixmap>, cx: f64, cy: f64, r: f64) {
    // symbole OANDA : sa pastille émoji (comme v1)
    if let Some(e) = argos_data::oanda::emoji(coin) {
        crate::icons::draw_emoji(pm, e, cx, cy, (2.0 * r + 2.0) as u32);
        return;
    }
    if let Some(logo) = logo.filter(|l| l.width() > 0) {
        let k = (2.0 * r / logo.width() as f64) as f32;
        let at = tiny_skia::Transform::from_row(k, 0.0, 0.0, k, (cx - r) as f32, (cy - r) as f32);
        let paint = tiny_skia::PixmapPaint {
            quality: tiny_skia::FilterQuality::Bicubic,
            ..Default::default()
        };
        pm.draw_pixmap(0, 0, logo.as_ref(), &paint, crate::px::t().pre_concat(at), None);
        return;
    }
    // (a Bitfinex perpetual « bitfinex:XRPF0:USTF0 »: XRP, not « USTF0 »)
    let bitfinex = argos_data::exchanges::bitfinex::pastille(coin);
    let name = bitfinex.as_deref().unwrap_or_else(|| coin.rsplit(':').next().unwrap_or(coin));
    let hash = name.bytes().fold(7u32, |h, b| h.wrapping_mul(31).wrapping_add(b as u32));
    const PALETTE: [Rgb; 8] = [
        Rgb(0xf7, 0x93, 0x1a),
        Rgb(0x62, 0x7e, 0xea),
        Rgb(0x26, 0xa1, 0x7b),
        Rgb(0xe8, 0x41, 0x42),
        Rgb(0x9b, 0x59, 0xb6),
        Rgb(0x00, 0xa3, 0xe0),
        Rgb(0xf0, 0xb9, 0x0b),
        Rgb(0x14, 0xb8, 0xa6),
    ];
    round_fill(pm, Rect::new(cx - r, cy - r, 2.0 * r, 2.0 * r), r, PALETTE[hash as usize % PALETTE.len()], 1.0);
    let letter: String = name.chars().next().map(|c| c.to_uppercase().collect()).unwrap_or_default();
    text.draw(
        pm,
        &letter,
        (cx as f32, (cy + r * 0.4) as f32),
        (r * 1.1) as f32,
        Rgb(255, 255, 255),
        Align::Center,
    );
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod colonne;

#[cfg(test)]
mod noms_entiers;
