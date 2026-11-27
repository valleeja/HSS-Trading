//! 🧰 Panneaux de droite, **comme Argos v1** (ouverts par la barre d'icônes de la watchlist) :
//! - 📈 **Indicateurs, scripts et modèles** : Personnel (favoris, mes scripts, nouveau, importer du Pine Script),
//!   Intégrés, Communauté ;
//! - 💼 **Trading démo** : compte démo Argos 100K, gardien PropFirm, calcul de lot ($ → lots / lots → $), ticket d'ordre
//!   (démo seulement : aucun ordre réel ne part d'ici) ;
//! - 📓 **Journal de trading** : Trades / Stats / PropFirm, périodes, tableau, export.
//!
//! Chaque panneau = une table de zones (`layout`) lue **à la fois** par le dessin et par le clic : pas de coordonnées
//! écrites deux fois. Le bouton **AI** (icône Argos AI) est le même partout (`ai_button`).

use crate::palette as pal;
use crate::palette::{GOLD, GREY, MUTED as DIM, WHITE};
use crate::shapes::{lines, round_fill, round_stroke, Rect};
use crate::text::{Align, Rgb, Text};
use crate::{fill, Theme};
use tiny_skia::Pixmap;

pub(crate) mod cell;
pub mod ticket;

const TEAL: Rgb = Rgb(0x4f, 0xd1, 0xc5);

/// Couleur du côté achat : turquoise, ou la couleur de hausse en thème daltonien.
fn buy_teal() -> Rgb {
    let k = crate::theme_index();
    if crate::palette::is_dalt(k) {
        crate::palette::up()
    } else {
        TEAL
    }
}

/// Bouton **AI** : l'icône Argos AI ronde (disque sombre, anneau blanc, « AI »), la même partout.
pub fn ai_button(pm: &mut Pixmap, r: Rect, hover: bool) {
    if hover {
        round_fill(pm, r, r.h / 2.0, pal::tile(), 1.0);
    }
    crate::aibtn::ai_icon(pm, r.cx(), r.cy(), r.h - 2.0);
}

/// Largeur du bouton AI.
pub const AI_W: f64 = 30.0;

/// Ce qu'un clic dans un panneau demande à la fenêtre.
#[derive(Debug, Clone, PartialEq)]
pub enum PanelClick {
    None,
    /// L'état du panneau a changé (redessiner, sauvegarder).
    Changed,
    /// Fonction pas encore là : afficher ce message.
    Soon(String),
    /// Bouton AI.
    Ai,
    /// 🖥 ⬓ de la feuille Trading : ses positions dans la colonne de modules.
    Ancrer,
    /// 🌍 Onglet Marchés de l'ancien ticket : le volet Marché crypto.
    Marches,
    /// 🖥 ⤢ de la feuille Trading : volet large ⇄ normal.
    Agrandir,
    /// 📈 Indicateur natif à activer / couper.
    Indicator(crate::indicators::Ind),
    /// ⚙️ Ouvrir les réglages (inputs) d'un indicateur.
    IndSettings(crate::indicators::Ind),
    /// 🛒 Ouvrir le Marketplace Argos.
    Marketplace,
    /// 🚀 Top gagnants / perdants 24 h (Communauté ▸ Top).
    Movers,
    /// 🎓 Ouvrir l'Académie Argos.
    Academy,
    /// 📓 Journal : nouveau trade, modifier, export, import.
    Journal(crate::journal::JClick),
    /// 📜 Ouvrir l'éditeur Argos Script.
    Script(ScriptOpen),
    /// 💼 Le ticket a changé (côté, risque, stop) : la position dessinée suit.
    Ticket,
    /// 💼 Passer l'ordre démo.
    Order,
    /// 💼 Fermer les positions démo ouvertes et annuler les ordres en attente.
    CloseAll,
    /// 🛡️ Réglage du gardien changé (sauver journal.json).
    GuardCfg,
    /// 🔓 Déverrouiller la journée.
    Unlock,
    /// 💼 Annuler cet ordre en attente (son identifiant).
    CancelOrder(u64),
    /// 💼 Menu des comptes démo (changer, créer, renommer, supprimer).
    Accounts,
    /// 📱 ✕ de la feuille : la fermer.
    Fermer,
    /// 📱 🧪 Simulateur (onglet de la feuille Trading).
    Simulateur,
    /// 🖥 🎫 Ticket while the Simulator shows in the sheet's place: the Simulator closes, the ticket comes back.
    FermerSimulateur,
    /// 🎫 VENDRE (`false`) / ACHETER (`true`) of the module 31 ticket: the demo order leaves now.
    Passer(bool),
    /// 🎫 A field of the ticket to type (its box opens).
    Saisir(ticket::Champ),
    /// ⌖ The next touch on the chart gives the order's price.
    Cible,
    /// 📖 « Carnet › »: the order book module of the column.
    Carnet,
    /// 🎚 The % slider pressed: it follows the mouse / the finger until let go ([`Panels::ticket_glisse`]).
    Curseur,
}

/// 📜 Lignes « Personnel » qui ouvrent l'éditeur.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptOpen {
    /// Mes scripts : le dernier script enregistré.
    Mine,
    /// Nouveau script : le modèle de départ.
    New,
    /// Importer du Pine Script : le presse-papiers collé dans un nouveau script.
    Import,
}

/// Les panneaux (hors watchlist).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Indicators,
    Trading,
    Journal,
}

/// Zone cliquable d'un panneau.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Hit {
    Ai,
    /// Ligne du panneau Indicateurs (index dans `INDICATORS`).
    Row(usize),
    /// Section CLASSIQUES / ARGOS : titre, puis un indicateur (index dans `Ind::ALL`).
    NatLabel(&'static str),
    Nat(usize),
    /// ⚙️ au bout de la ligne d'un indicateur qui a des inputs.
    NatGear(usize),
    /// Recherche (Indicateurs).
    Search,
    Guardian,
    /// 🛡️ Réglage du gardien (index dans `GUARD_STEPS`) et sens (+1 / −1).
    GuardStep(usize, i8),
    /// 🛡️ Case du gardien : 0 SL obligatoire, 1 news, 2 fuseau du reset (au clic : le suivant).
    GuardToggle(usize),
    /// 🔓 « Déverrouiller aujourd'hui ».
    GuardUnlock,
    /// 💼 Prix de l'ordre Limit / Stop : ▲ (+1) ou ▼ (−1).
    PxStep(i8),
    /// 💼 ✕ d'un ordre en attente (index dans `pending`).
    CancelOrder(usize),
    /// 💼 Compte démo (en haut du panneau).
    Account,
    /// Calcul de lot : $ → lots (`true`) ou lots → $.
    LotMode(bool),
    /// Calcul de lot : champ (0 risque / lots, 1 stop) et sens (+1 / −1).
    Step(usize, i8),
    Margin(usize),
    Tab(usize),
    Side(bool),
    Order,
    /// Fermer les positions démo et annuler les ordres en attente (seulement s'il y en a).
    CloseAll,
    Bottom(usize),
    /// Texte seul (pas cliquable), dessiné par sa zone.
    Label,
    /// 📱 Feuille Trading du cell (maquette).
    Cell(cell::Cell),
}

/// Une ligne du panneau Indicateurs : section, icône, nom, nombre (vide : rien).
struct IndRow {
    section: &'static str,
    icon: Icon,
    label: &'static str,
}

/// Icône Phosphor d'une ligne du panneau Indicateurs.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Icon {
    Star,
    User,
    Plus,
    Tree,
    Tech,
    Bars,
    Cap,
    Cart,
    Plug,
    Mark,
}

impl Icon {
    fn name(self) -> &'static str {
        match self {
            Icon::Star => "star",
            Icon::User => "user",
            Icon::Plus => "plus",
            Icon::Tree => "tree-evergreen",
            Icon::Tech => "pulse",
            Icon::Bars => "chart-bar",
            Icon::Cap => "graduation-cap",
            Icon::Cart => "shopping-cart",
            Icon::Plug => "plugs",
            Icon::Mark => "seal-check",
        }
    }
}

const INDICATORS: [IndRow; 11] = [
    IndRow {
        section: "PERSONNEL",
        icon: Icon::Star,
        label: "Favoris",
    },
    IndRow {
        section: "PERSONNEL",
        icon: Icon::User,
        label: "Mes scripts",
    },
    IndRow {
        section: "PERSONNEL",
        icon: Icon::Plus,
        label: "Nouveau script",
    },
    IndRow {
        section: "PERSONNEL",
        icon: Icon::Tree,
        label: "Importer du Pine Script",
    },
    IndRow {
        section: "INTÉGRÉS",
        icon: Icon::Tech,
        label: "Données techniques",
    },
    IndRow {
        section: "INTÉGRÉS",
        icon: Icon::Bars,
        label: "Données fondamentales",
    },
    IndRow {
        section: "COMMUNAUTÉ",
        icon: Icon::Cap,
        label: "Académie Argos",
    },
    IndRow {
        section: "COMMUNAUTÉ",
        icon: Icon::Cart,
        label: "Marketplace Argos",
    },
    IndRow {
        section: "COMMUNAUTÉ",
        icon: Icon::Plug,
        label: "Sources d'exchanges",
    },
    IndRow {
        section: "COMMUNAUTÉ",
        icon: Icon::Mark,
        label: "Choix Argos",
    },
    IndRow {
        section: "COMMUNAUTÉ",
        icon: Icon::Bars,
        label: "Top",
    },
];

/// 🚀 Top gagnants et perdants sur 24 h : `rows` = (symbole, variation %, volume 24 h $). Seuls les marchés d'au
/// moins `min_vol` $ comptent (pas de petits marchés sans liquidité) ; `n` de chaque côté, les plus forts d'abord.
pub fn movers(rows: &[(String, f64, f64)], n: usize, min_vol: f64) -> (Vec<(String, f64)>, Vec<(String, f64)>) {
    let mut v: Vec<(String, f64)> = rows
        .iter()
        .filter(|(_, pct, vol)| pct.is_finite() && *vol >= min_vol)
        .map(|(c, p, _)| (c.clone(), *p))
        .collect();
    v.sort_by(|a, b| b.1.total_cmp(&a.1));
    let up: Vec<(String, f64)> = v.iter().filter(|x| x.1 > 0.0).take(n).cloned().collect();
    let down: Vec<(String, f64)> = v.iter().rev().filter(|x| x.1 < 0.0).take(n).cloned().collect();
    (up, down)
}

fn draw_icon(pm: &mut Pixmap, icon: Icon, cx: f64, cy: f64, c: Rgb) {
    crate::phosphor::draw(pm, icon.name(), cx, cy, 22, c);
}

/// État du panneau Trading démo.
#[derive(Debug, Clone, PartialEq)]
pub struct Trading {
    /// Calcul de lot : `true` = $ → lots (risque en $ → taille) ; `false` = lots → $.
    pub to_lots: bool,
    pub risk: f64,
    pub lots: f64,
    pub sl_pips: f64,
    /// Valeur d'un pip pour 1 lot, en $ (BTC : 1 $ / pip / lot).
    pub pip_value: f64,
    pub guardian: bool,
    /// Marge (0 isolée, 1 levier, 2 compte démo), onglet (0 Market, 1 Limit, 2 Pro), achat / vente, onglet du bas.
    pub margin: usize,
    pub tab: usize,
    pub buy: bool,
    pub bottom: usize,
    /// Solde du compte démo.
    pub balance: f64,
    /// 💼 Position dessinée liée au ticket (`Some` : entrée, stop, cible), et le symbole ouvert.
    pub plan: Option<(f64, f64, f64)>,
    pub sym: String,
    /// Positions démo ouvertes : (texte, P&L au dernier prix).
    pub open: Vec<(String, f64)>,
    /// 💼 Prix de l'ordre Limit / Stop (0 : pas encore choisi, pris au dernier prix).
    pub px: f64,
    /// 💼 Ordres en attente : (identifiant, texte).
    pub pending: Vec<(u64, String)>,
    /// 💼 Nom du compte démo choisi.
    pub account: String,
    /// 📱 Dernier prix du symbole (VENDRE / ACHETER au marché), feuille du cell (maquette) et « ⚙ Tous les réglages »
    /// ouvert (l'ancien ticket complet).
    pub prix: f64,
    pub cell: bool,
    pub options: bool,
    /// 🖥 The Simulator shows in this sheet's place, under its header (PC volet: the 🧪 tab of the module 31 mock-up):
    /// its tab is lit and the ticket is not drawn under it. Set by the window.
    pub sim_onglet: bool,
    /// 🎫 Module 31 ticket: the size (quantity) and the % of the buying power it stands for (the slider), TP / SL as
    /// distances in % from the entry (mirrored for a sell) and their box, reduce-only, the validity and its menu.
    pub qty: f64,
    pub pct: f64,
    pub tpsl: bool,
    pub tp_pct: f64,
    pub sl_pct: f64,
    pub reduce_only: bool,
    pub validity: argos_core::simu::Validity,
    pub menu_validite: bool,
    /// 🛡 Gardien card unfolded, 📒 Compte unfolded, « Positions et ordres » shown in the sheet.
    pub garde_ouverte: bool,
    pub compte_ouvert: bool,
    pub positions_vue: bool,
    /// ⌖ Waiting for a touch on the chart: the order's price takes its level.
    pub cible: bool,
    /// ⚡ Leverage of the perpetual orders (the Simulateur's too), ×10 until it is changed.
    pub levier: f64,
    /// Filled by the window each frame (module 31): the market (symbol, exchange), the book's Mid, what is available
    /// and the buying power, the market's maximum leverage (0: spot), the quantity a reduce-only buy / sell can take
    /// off, the position on this market (signed quantity, P&L), the account block, the funding of the day, and the
    /// estimate of the ticket.
    pub marche: (String, String),
    pub mid: f64,
    pub dispo: f64,
    pub pouvoir: f64,
    pub levier_max: f64,
    pub reducible: (f64, f64),
    pub position: Option<(f64, f64)>,
    pub compte: argos_core::portfolio::AccountSummary,
    pub funding_jour: f64,
    pub estimate: argos_core::simu::Estimate,
    /// 📱 Room kept at the right of VENDRE / ACHETER at the bottom of the phone's sheet: the Argos AI round sits there
    /// (filled by the window; 0: nothing there).
    pub place_ai: f64,
    /// ₿ The market is a pair against BTC (filled by the window): the demo account counts in dollars, so no ticket
    /// yet, a message instead ([`TICKET_PAS_BTC`]).
    pub pas_btc: bool,
}

/// ₿ What the Trading sheet says on a pair against BTC (Jee, 23 Nov 2026).
pub const TICKET_PAS_BTC: &str = "Trading démo : pas encore pour les paires contre BTC";

impl Default for Trading {
    fn default() -> Self {
        Self {
            to_lots: true,
            risk: 250.0,
            lots: 0.5,
            sl_pips: 50.0,
            pip_value: 1.0,
            guardian: false,
            margin: 0,
            tab: 0,
            buy: true,
            bottom: 1,
            balance: 100_000.0,
            plan: None,
            sym: String::new(),
            open: vec![],
            px: 0.0,
            pending: vec![],
            account: String::new(),
            prix: 0.0,
            cell: false,
            options: false,
            sim_onglet: false,
            qty: 0.0,
            pct: 0.0,
            tpsl: true,
            tp_pct: 2.0,
            sl_pct: 1.0,
            reduce_only: false,
            validity: argos_core::simu::Validity::Gtc,
            menu_validite: false,
            garde_ouverte: false,
            compte_ouvert: false,
            positions_vue: false,
            cible: false,
            levier: 10.0,
            marche: (String::new(), String::new()),
            mid: 0.0,
            dispo: 0.0,
            pouvoir: 0.0,
            levier_max: 0.0,
            reducible: (0.0, 0.0),
            position: None,
            compte: argos_core::portfolio::AccountSummary::default(),
            funding_jour: 0.0,
            estimate: argos_core::simu::Estimate::default(),
            place_ai: 0.0,
            pas_btc: false,
        }
    }
}

impl Trading {
    /// Résultat du calcul de lot : taille (lots) pour le risque, ou risque ($) pour la taille.
    pub fn result(&self) -> f64 {
        let per_lot = self.sl_pips * self.pip_value;
        if self.to_lots {
            if per_lot > 0.0 {
                self.risk / per_lot
            } else {
                0.0
            }
        } else {
            self.lots * per_lot
        }
    }

    /// Pas du prix de l'ordre : environ un millième du prix, arrondi (80 000 → 10 ; 4 000 → 1).
    pub fn px_step(&self) -> f64 {
        if self.px > 0.0 {
            10f64.powi(self.px.log10().floor() as i32 - 3)
        } else {
            1.0
        }
    }

    /// Place prise par le champ du prix (onglets Limit et Stop) : le reste du ticket descend d'autant.
    fn px_dy(&self) -> f64 {
        if self.tab == 0 {
            0.0
        } else {
            44.0
        }
    }

    fn step(&mut self, field: usize, dir: i8) {
        let d = dir as f64;
        match (field, self.to_lots) {
            (0, true) => self.risk = (self.risk + 50.0 * d).max(0.0),
            (0, false) => self.lots = ((self.lots + 0.1 * d) * 10.0).round().max(0.0) / 10.0,
            _ => self.sl_pips = (self.sl_pips + 5.0 * d).max(0.0),
        }
    }
}

/// Les panneaux et leur état.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Panels {
    pub trading: Trading,
    pub journal: crate::journal::Journal,
    /// Nombre de favoris / de scripts (Indicateurs).
    pub counts: [usize; 2],
    /// Indicateurs natifs actifs (cochés ; remplis par la fenêtre).
    pub ind_on: Vec<crate::indicators::Ind>,
    /// 🔎 Recherche (Indicateurs) : `Some` = le champ reçoit le clavier ; le texte filtre la liste.
    pub search: Option<String>,
}

/// Le nom correspond-il à la recherche ? (sans majuscules ni accents)
fn matches(name: &str, q: &str) -> bool {
    crate::i18n::plain(name).contains(&crate::i18n::plain(q.trim()))
}

impl Panels {
    /// 📱 La feuille Trading du cell (maquette) plutôt que l'ancien ticket complet ?
    fn cell_trading(&self) -> bool {
        self.trading.cell && !self.trading.options
    }

    /// 🖥 La feuille Trading de la maquette est affichée (elle a ses propres ⬓ ⤢ ✕ dans son en-tête).
    pub fn feuille_trading(&self) -> bool {
        self.cell_trading()
    }

    /// Texte tapé dans la recherche (Échap la vide et la ferme ; Retour arrière efface).
    pub fn search_key(&mut self, c: Option<char>) {
        if let Some(q) = self.search.as_mut() {
            match c {
                Some(c) if !c.is_control() => q.push(c),
                Some(_) => {}
                None => _ = q.pop(),
            }
        }
    }
}

const PAD: f64 = 16.0;
const TOP: f64 = 44.0;
/// 👆 Touch area of the « Tout fermer » button and of each pending order's ✕ (40 pt, like every finger target).
const TOUCHER: f64 = crate::indicators::lens::TOUCHER_MIN;
/// 💼 Room kept between the pending orders and the order button below them.
const ECART: f64 = 8.0;

/// 🛡️ Réglages chiffrés du gardien (dans sa carte) : libellé court, unité.
const GUARD_STEPS: [(&str, &str); 8] = [
    ("Jour", " %"),
    ("Max", " %"),
    ("Risque", " %"),
    ("Positions", ""),
    ("Trades", ""),
    ("Alerte", " %"),
    ("Tilt", ""),
    ("Pause", " min"),
];

/// 🛡️ Fuseaux du reset quotidien proposés (au clic : le suivant).
const RESET_TZ: [(&str, &str); 4] = [
    ("Prague", "Europe/Prague"),
    ("New York", "America/New_York"),
    ("Londres", "Europe/London"),
    ("UTC", "UTC"),
];

/// 🛡️ Valeur du réglage `k` du gardien.
fn guard_value(c: &argos_core::journal::JCfg, k: usize) -> f64 {
    match k {
        0 => c.daily,
        1 => c.maxloss,
        2 => c.risk_pct,
        3 => c.max_pos as f64,
        4 => c.max_trades as f64,
        5 => c.warn,
        6 => c.tilt_n as f64,
        _ => c.tilt_min as f64,
    }
}

/// 🛡️ ▲ ▼ sur le réglage `k` du gardien (bornes : jamais sous 0, alerte entre 5 et 100 %, anti-tilt de 0 (éteint) à
/// 10 pertes, pause de 5 à 240 min).
fn guard_step(c: &mut argos_core::journal::JCfg, k: usize, dir: i8) {
    let d = dir as f64;
    match k {
        0 => c.daily = (c.daily + 0.5 * d).max(0.5),
        1 => c.maxloss = (c.maxloss + 0.5 * d).max(0.5),
        2 => c.risk_pct = (c.risk_pct + 0.25 * d).max(0.0),
        3 => c.max_pos = c.max_pos.saturating_add_signed(dir as i32),
        4 => c.max_trades = c.max_trades.saturating_add_signed(dir as i32),
        5 => c.warn = (c.warn + 5.0 * d).clamp(5.0, 100.0),
        6 => c.tilt_n = c.tilt_n.saturating_add_signed(dir as i32).min(10),
        _ => c.tilt_min = (c.tilt_min as i32 + 5 * dir as i32).clamp(5, 240) as u32,
    }
}

/// 🛡️ Ville du fuseau du reset (le fuseau tel quel s'il n'est pas dans la liste).
fn reset_city(tz: &str) -> &str {
    RESET_TZ.iter().find(|r| r.1 == tz).map_or(tz, |r| r.0)
}

fn title(kind: Kind) -> &'static str {
    match kind {
        Kind::Indicators => "Indicateurs, scripts et modèles",
        Kind::Trading => "Trading démo",
        Kind::Journal => "Journal de trading",
    }
}

/// Rangée de boutons égaux sur la largeur `w` (à partir de `x`).
fn row(x: f64, y: f64, w: f64, h: f64, n: usize, gap: f64) -> Vec<Rect> {
    let bw = (w - gap * (n as f64 - 1.0)) / n as f64;
    (0..n).map(|i| Rect::new(x + i as f64 * (bw + gap), y, bw, h)).collect()
}

/// 💼 Label of « Tout fermer »: the demo positions (and their P&L), the pending orders, or both.
fn close_all_txt(positions: usize, pnl: f64, ordres: usize) -> String {
    let s = |n: usize| if n > 1 { "s" } else { "" };
    let pos = format!("{positions} position{} démo ({pnl:+.2} $)", s(positions));
    let ord = format!("{ordres} ordre{} en attente", s(ordres));
    match (positions, ordres) {
        (_, 0) => format!("Fermer {pos}"),
        (0, _) => format!("Annuler {ord}"),
        _ => format!("Tout fermer : {pos} + {ord}"),
    }
}

impl Panels {
    /// 🛡️ Carte du gardien (au-dessus du ticket, s'il est actif) et ses zones ; `None` si le gardien est éteint.
    fn guard_layout(&self, pw: f64) -> Option<(Rect, Vec<(Rect, Hit)>)> {
        if !self.trading.guardian {
            return None;
        }
        let inner = pw - 2.0 * PAD;
        let (x, y0) = (PAD, TOP + 80.0);
        // en-tête 28 (☀ Routine), perte du jour et sa jauge 36, Perte max / Positions / Risque max 34, ⚠ prudence,
        // règles (📋 Mon plan) 22 — comme la carte de la maquette (gdCarte)
        let mut out = vec![
            (Rect::new(x + inner - 82.0, y0 + 5.0, 72.0, 20.0), Hit::Cell(cell::Cell::Routine)),
            (
                Rect::new(x + 58.0, y0 + 28.0 + 36.0 + 34.0 + self.guard_msg_h(), 96.0, 22.0),
                Hit::Cell(cell::Cell::Plan),
            ),
        ];
        let mut y = y0 + 28.0 + 36.0 + 34.0 + self.guard_msg_h() + 22.0;
        if self.journal.locked(crate::journal::now_ms()) {
            y += 22.0;
            out.push((Rect::new(x + 10.0, y, inner - 20.0, 28.0), Hit::GuardUnlock));
            y += 36.0;
        }
        for k in 0..2 {
            for (i, r) in row(x + 10.0, y, inner - 20.0, 26.0, 3, 6.0).into_iter().enumerate() {
                let f = k * 3 + i;
                out.push((Rect::new(r.x + r.w - 20.0, r.y, 20.0, 13.0), Hit::GuardStep(f, 1)));
                out.push((Rect::new(r.x + r.w - 20.0, r.y + 13.0, 20.0, 13.0), Hit::GuardStep(f, -1)));
            }
            y += 32.0;
        }
        for (i, r) in row(x + 10.0, y, inner - 20.0, 26.0, 3, 6.0).into_iter().enumerate() {
            out.push((r, Hit::GuardToggle(i)));
        }
        y += 34.0;
        // 🧊 pause anti-tilt : pertes de suite, minutes, case « revenge »
        let tilt = row(x + 10.0, y, inner - 20.0, 26.0, 3, 6.0);
        for (f, r) in [6, 7].into_iter().zip(&tilt) {
            out.push((Rect::new(r.x + r.w - 20.0, r.y, 20.0, 13.0), Hit::GuardStep(f, 1)));
            out.push((Rect::new(r.x + r.w - 20.0, r.y + 13.0, 20.0, 13.0), Hit::GuardStep(f, -1)));
        }
        out.push((tilt[2], Hit::GuardToggle(3)));
        y += 34.0;
        Some((Rect::new(x, y0, inner, y - y0), out))
    }

    /// 🛡️ Place du message ⚠ prudence dans la carte (0 sans message).
    fn guard_msg_h(&self) -> f64 {
        self.cell_garde().msg_h()
    }

    /// 🛡️ Place prise par la carte du gardien (0 s'il est éteint) : le reste du ticket descend d'autant.
    fn guard_dy(&self, pw: f64) -> f64 {
        self.guard_layout(pw).map_or(0.0, |(r, _)| r.h + 10.0)
    }

    /// 📐 Titre du volet `kind` (largeur `pw`) et ce qui est à sa droite (le bouton AI, sinon la place des boutons du
    /// volet) : le titre se coupe (…) avant.
    pub fn entete(text: &Text, kind: Kind, pw: f64) -> (crate::entete::Entete, Rect) {
        let b = if kind == Kind::Indicators {
            Rect::new(pw - crate::watchlist::FERME_PLACE, 9.0, crate::watchlist::FERME_PLACE, 28.0)
        } else {
            Self::ai_rect(pw)
        };
        (crate::entete::placer(text, title(kind), 16.0, (14.0, 29.0), &[b]), b)
    }

    /// Bouton AI de l'en-tête (Trading démo et Journal ; pas dans Indicateurs).
    pub fn ai_rect(pw: f64) -> Rect {
        Rect::new(pw - AI_W - 12.0 - crate::watchlist::FERME_PLACE, 9.0, AI_W, 28.0)
    }

    /// Hauteur dont `kind` a besoin pour ne rien chevaucher (le panneau défile en dessous).
    pub fn content_h(&self, kind: Kind, pw: f64) -> f64 {
        match kind {
            // calcul de lot, marge, onglets, achat / vente, solde, avis démo, puis l'ordre et les onglets du bas
            // (🧪 the Simulator in the sheet's place: only the header is the sheet's)
            Kind::Trading if self.cell_trading() && self.trading.sim_onglet => ticket::ENTETE,
            Kind::Trading if self.cell_trading() => self.cell_haut(pw),
            Kind::Trading => {
                let t = &self.trading;
                700.0
                    + self.guard_dy(pw)
                    + t.px_dy()
                    + if t.pending.is_empty() {
                        0.0
                    } else {
                        22.0 + TOUCHER * t.pending.len() as f64 + ECART
                    }
            }
            Kind::Journal => self.journal.content_h(pw),
            _ => self.layout(kind, pw, 0.0).iter().map(|(r, _)| r.y + r.h).fold(0.0, f64::max) + 16.0,
        }
    }

    /// Zones de `kind` (coordonnées du panneau, largeur `pw`, hauteur `h`).
    fn layout(&self, kind: Kind, pw: f64, h: f64) -> Vec<(Rect, Hit)> {
        if kind == Kind::Trading && self.cell_trading() {
            return self.cell_zones(pw);
        }
        let mut out = if kind == Kind::Indicators {
            vec![]
        } else {
            vec![(Self::ai_rect(pw), Hit::Ai)]
        };
        let inner = pw - 2.0 * PAD;
        match kind {
            Kind::Indicators => {
                out.push((Rect::new(PAD, TOP + 6.0, inner, 40.0), Hit::Search));
                let mut y = TOP + 62.0;
                // les indicateurs d'Argos en premier, par section comme v1 : un clic les met sur la chart
                let q = self.search.as_deref().unwrap_or("");
                let mut sec = "";
                // les classiques d'abord, puis Argos (l'index `k` reste celui de `Ind::ALL`)
                // (Argos grouped by ƒx category: Argos Murs first of « Order flow / carnet »)
                let order = crate::catalogue::ordre_liste();
                for (k, ind) in order.into_iter().filter(|(_, i)| matches(&crate::catalogue::texte_recherche(*i), q)) {
                    if ind.section() != sec {
                        sec = ind.section();
                        out.push((Rect::new(PAD, y, inner, 34.0), Hit::NatLabel(sec)));
                        y += 34.0;
                    }
                    out.push((Rect::new(6.0, y, pw - 12.0, 40.0), Hit::Nat(k)));
                    // ⚙️ par-dessus la ligne (dessiné après, prioritaire au clic)
                    if !ind.params().1.is_empty() {
                        out.push((Rect::new(pw - 46.0, y + 4.0, 32.0, 32.0), Hit::NatGear(k)));
                    }
                    y += 40.0;
                }
                let mut section = "";
                for (i, r) in INDICATORS.iter().enumerate().filter(|(_, r)| matches(r.label, q)) {
                    if r.section != section {
                        section = r.section;
                        out.push((Rect::new(PAD, y, inner, 34.0), Hit::Label));
                        y += 34.0;
                    }
                    out.push((Rect::new(6.0, y, pw - 12.0, 46.0), Hit::Row(i)));
                    y += 46.0;
                }
            }
            Kind::Trading => {
                let t = &self.trading;
                out.push((Rect::new(PAD, TOP + 50.0, inner, 22.0), Hit::Guardian));
                // 🛡️ carte du gardien (s'il est actif) ; le reste descend d'autant
                if let Some((_, zones)) = self.guard_layout(pw) {
                    out.extend(zones);
                }
                // calcul de lot : carte, 2 modes, 2 champs avec ▲ ▼
                let card_y = TOP + 80.0 + self.guard_dy(pw);
                let modes = row(PAD + 12.0, card_y + 36.0, 170.0, 26.0, 2, 8.0);
                out.push((modes[0], Hit::LotMode(true)));
                out.push((modes[1], Hit::LotMode(false)));
                let fy = card_y + 72.0;
                for (k, x) in [(0usize, PAD + 12.0), (1, PAD + 12.0 + inner / 2.0)] {
                    let fw = inner / 2.0 - 24.0;
                    out.push((Rect::new(x + fw - 22.0, fy, 22.0, 13.0), Hit::Step(k, 1)));
                    out.push((Rect::new(x + fw - 22.0, fy + 13.0, 22.0, 13.0), Hit::Step(k, -1)));
                }
                let mut y = card_y + 168.0;
                for (i, r) in row(PAD, y, inner, 40.0, 3, 8.0).into_iter().enumerate() {
                    out.push((r, Hit::Margin(i)));
                }
                y += 56.0;
                for (i, r) in row(PAD, y, 200.0, 34.0, 3, 0.0).into_iter().enumerate() {
                    out.push((r, Hit::Tab(i)));
                }
                y += 50.0;
                // Limit / Stop : le prix de l'ordre, avec ▲ ▼
                if t.tab != 0 {
                    out.push((Rect::new(PAD + inner - 26.0, y, 26.0, 16.0), Hit::PxStep(1)));
                    out.push((Rect::new(PAD + inner - 26.0, y + 16.0, 26.0, 16.0), Hit::PxStep(-1)));
                    y += t.px_dy();
                }
                let sides = row(PAD, y, inner, 40.0, 2, 4.0);
                out.push((sides[0], Hit::Side(true)));
                out.push((sides[1], Hit::Side(false)));
                // sous « Disponible / Position actuelle », s'il reste la place au-dessus du bouton d'ordre (40 pt de
                // haut, et `ECART` avant le bouton d'ordre)
                let ordre = Rect::new(PAD, h - 112.0, inner, 42.0);
                let close = Rect::new(PAD, y + 72.0 + 74.0, inner, TOUCHER);
                let mut ly = close.y;
                // (« Tout fermer » also when there are only pending orders: it cancels them)
                if (!t.open.is_empty() || !t.pending.is_empty()) && close.y + close.h + ECART <= ordre.y {
                    out.push((close, Hit::CloseAll));
                    ly += TOUCHER + ECART;
                }
                // ordres en attente : une ligne de 40 pt chacun, son ✕ (zone de 40 pt) pour l'annuler ; seulement les
                // lignes qui tiennent au-dessus du bouton d'ordre (les autres : le compte du titre)
                ly += 22.0; // titre « Ordres en attente »
                for (i, _) in t.pending.iter().enumerate() {
                    if ly + TOUCHER + ECART > ordre.y {
                        break;
                    }
                    out.push((Rect::new(PAD + inner - TOUCHER, ly, TOUCHER, TOUCHER), Hit::CancelOrder(i)));
                    ly += TOUCHER;
                }
                out.push((Rect::new(pw / 2.0 - 130.0, TOP + 6.0, 260.0, 36.0), Hit::Account));
                out.push((ordre, Hit::Order));
                for (i, r) in row(PAD, h - 58.0, inner, 36.0, 3, 8.0).into_iter().enumerate() {
                    out.push((r, Hit::Bottom(i)));
                }
            }
            Kind::Journal => {}
        }
        out
    }

    /// What a click at (`x`, `y`) of the panel `kind` lands on (a ⚙ beside a name wins over its row).
    fn zone_at(&self, kind: Kind, x: f64, y: f64, pw: f64, h: f64) -> Option<Hit> {
        self.layout(kind, pw, h)
            .into_iter()
            .filter(|(r, _)| r.contains(x, y))
            .min_by_key(|(_, h)| !matches!(h, Hit::NatGear(_)))
            .map(|(_, h)| h)
    }

    /// 🎚 (`x`, `y`) of the Trading sheet is on the ticket's % slider: a finger put there drags it (no scrolling).
    pub fn sur_curseur(&self, x: f64, y: f64, pw: f64, h: f64) -> bool {
        self.feuille_trading() && self.zone_at(Kind::Trading, x, y, pw, h) == Some(Hit::Cell(cell::Cell::Curseur))
    }

    /// Clic dans le panneau `kind` (coordonnées du panneau).
    pub fn click(&mut self, kind: Kind, x: f64, y: f64, pw: f64, h: f64) -> PanelClick {
        if kind == Kind::Journal && !Self::ai_rect(pw).contains(x, y) {
            return match self.journal.click(x, y, pw, crate::journal::now_ms()) {
                crate::journal::JClick::None => PanelClick::None,
                crate::journal::JClick::Changed => PanelClick::Changed,
                c => PanelClick::Journal(c),
            };
        }
        let Some(hit) = self.zone_at(kind, x, y, pw, h) else {
            return PanelClick::None;
        };
        let t = &mut self.trading;
        match hit {
            Hit::Ai => return PanelClick::Ai,
            Hit::Label => return PanelClick::None,
            Hit::Search => {
                self.search.get_or_insert_with(String::new);
                return PanelClick::Changed;
            }
            Hit::NatLabel(_) => return PanelClick::None,
            Hit::Nat(k) => return PanelClick::Indicator(crate::indicators::Ind::ALL[k]),
            Hit::NatGear(k) => return PanelClick::IndSettings(crate::indicators::Ind::ALL[k]),
            Hit::Row(i) if INDICATORS[i].label == "Marketplace Argos" => return PanelClick::Marketplace,
            Hit::Row(i) if INDICATORS[i].label == "Top" => return PanelClick::Movers,
            Hit::Row(i) if INDICATORS[i].label == "Académie Argos" => return PanelClick::Academy,
            Hit::Row(i) if INDICATORS[i].label == "Mes scripts" => return PanelClick::Script(ScriptOpen::Mine),
            Hit::Row(i) if INDICATORS[i].label == "Nouveau script" => return PanelClick::Script(ScriptOpen::New),
            Hit::Row(i) if INDICATORS[i].label == "Importer du Pine Script" => return PanelClick::Script(ScriptOpen::Import),
            Hit::Row(i) => return PanelClick::Soon(format!("{} : arrive avec Argos Script (couche 2B)", INDICATORS[i].label)),
            Hit::Guardian => t.guardian = !t.guardian,
            Hit::GuardStep(k, d) => {
                guard_step(&mut self.journal.cfg, k, d);
                return PanelClick::GuardCfg;
            }
            Hit::GuardToggle(i) => {
                let c = &mut self.journal.cfg;
                match i {
                    0 => c.need_sl = !c.need_sl,
                    1 => c.news = !c.news,
                    3 => c.tilt_revenge = !c.tilt_revenge,
                    _ => {
                        let k = RESET_TZ.iter().position(|r| r.1 == c.reset_tz).map_or(0, |k| (k + 1) % RESET_TZ.len());
                        c.reset_tz = RESET_TZ[k].1.into();
                    }
                }
                return PanelClick::GuardCfg;
            }
            Hit::GuardUnlock => return PanelClick::Unlock,
            Hit::PxStep(d) => {
                let st = t.px_step();
                t.px = ((t.px + st * d as f64) / st).round() * st;
                return PanelClick::Ticket;
            }
            Hit::CancelOrder(i) => {
                if let Some(&(id, _)) = t.pending.get(i) {
                    return PanelClick::CancelOrder(id);
                }
                return PanelClick::None;
            }
            Hit::Account => return PanelClick::Accounts,
            Hit::LotMode(m) => t.to_lots = m,
            Hit::Step(f, d) => {
                t.step(f, d);
                return PanelClick::Ticket;
            }
            // ⚡ the leverage: the market's next one (perpetual; after the largest, the smallest again)
            Hit::Margin(1) if t.levier_max > 0.0 => {
                let l = argos_core::perp::leviers(t.levier_max);
                let k = l.iter().position(|x| *x > t.levier.min(t.levier_max)).unwrap_or(0);
                (t.levier, t.margin) = (l[k], 1);
            }
            Hit::Margin(1) => return PanelClick::Soon("⚡ Au comptant : pas de levier (seulement sur un perpétuel)".into()),
            Hit::Margin(i) => t.margin = i,
            Hit::Tab(i) => t.tab = i,
            Hit::Side(b) => {
                t.buy = b;
                return PanelClick::Ticket;
            }
            Hit::Order => return PanelClick::Order,
            Hit::CloseAll => return PanelClick::CloseAll,
            // (l'ancien ticket, « ⋯ Toutes les options » : de vrais onglets, plus seulement un surlignage)
            Hit::Bottom(0) => return PanelClick::Marches,
            Hit::Bottom(1) => t.options = false, // Trade : retour à la feuille de la maquette
            Hit::Bottom(_) => return PanelClick::Accounts,
            // 🎚 the % slider: where it is touched (then it follows the finger: `ticket_glisse`)
            Hit::Cell(cell::Cell::Curseur) => {
                self.ticket_glisse(x, pw);
                return PanelClick::Curseur;
            }
            Hit::Cell(c) => return self.cell_click(c),
        }
        PanelClick::Changed
    }

    /// Dessine le panneau `kind` dans `pm` (taille `(pw, h)`) ; `mouse` en coordonnées du panneau.
    pub fn draw(&self, kind: Kind, pm: &mut Pixmap, text: &Text, (pw, h): (f64, f64), mouse: Option<(f64, f64)>, th: &Theme) {
        if kind == Kind::Trading && self.cell_trading() {
            return self.cell_draw(pm, text, pw, mouse);
        }
        let hover = |r: &Rect| mouse.is_some_and(|(x, y)| r.contains(x, y));
        // en-tête : titre + AI
        text.draw(pm, &Self::entete(text, kind, pw).0.titre, (14.0, 29.0), 16.0, WHITE, Align::Left);
        let zones = self.layout(kind, pw, h);
        let t = &self.trading;
        if kind == Kind::Trading {
            // fond (carte, champs) d'abord : les boutons passent par-dessus ; les lignes des ordres en attente là où
            // sont leurs ✕
            let lignes: Vec<Rect> = zones.iter().filter(|(_, x)| matches!(x, Hit::CancelOrder(_))).map(|(r, _)| *r).collect();
            self.draw_trading(pm, text, pw, &lignes);
        }
        for (r, hit) in &zones {
            let r = *r;
            match *hit {
                Hit::Cell(_) => {}
                Hit::Ai => ai_button(pm, r, hover(&r)),
                Hit::Search => {
                    round_fill(pm, r, 8.0, pal::field(), 1.0);
                    round_stroke(pm, r, 8.0, pal::edge_soft(), 1.0);
                    let ring = crate::shapes::arc(5.5, 0.0, 360.0, 20);
                    lines(pm, r.x + 18.0, r.cy() - 1.0, &[&ring, &[(4.0, 4.0), (8.0, 8.0)]], GREY, 1.4);
                    match self.search.as_deref() {
                        Some(q) => {
                            round_stroke(pm, r, 8.0, crate::palette::accent(), 1.0);
                            text.draw(pm, &format!("{q}|"), ((r.x + 36.0) as f32, (r.cy() + 5.0) as f32), 14.0, WHITE, Align::Left);
                        }
                        None => text.draw(pm, "Chercher", ((r.x + 36.0) as f32, (r.cy() + 5.0) as f32), 14.0, DIM, Align::Left),
                    }
                }
                Hit::Label => {
                    let i = zones.iter().position(|(z, _)| z.y > r.y).and_then(|k| match zones[k].1 {
                        Hit::Row(i) => Some(i),
                        _ => None,
                    });
                    if let Some(i) = i {
                        text.draw(pm, INDICATORS[i].section, (r.x as f32, (r.y + 24.0) as f32), 11.5, GREY, Align::Left);
                    }
                }
                Hit::NatLabel(sec) => text.draw(pm, sec, (r.x as f32, (r.y + 24.0) as f32), 11.5, GREY, Align::Left),
                Hit::Nat(k) => {
                    let ind = crate::indicators::Ind::ALL[k];
                    let on = self.ind_on.contains(&ind);
                    if hover(&r) {
                        round_fill(pm, r, 6.0, pal::tile(), 1.0);
                    }
                    // case à cocher
                    let b = Rect::new(r.x + 14.0, r.cy() - 9.0, 18.0, 18.0);
                    round_fill(pm, b, 4.0, if on { crate::palette::accent() } else { pal::card() }, 1.0);
                    round_stroke(pm, b, 4.0, if on { crate::palette::accent() } else { pal::edge_soft() }, 1.0);
                    if on {
                        lines(pm, b.cx(), b.cy(), &[&[(-4.5, 0.0), (-1.5, 3.5), (4.5, -3.5)]], WHITE, 1.8);
                    }
                    text.draw(
                        pm,
                        &text.fit(ind.name(), 14.0, r.w - 80.0),
                        ((r.x + 46.0) as f32, (r.cy() + 5.0) as f32),
                        14.0,
                        WHITE,
                        Align::Left,
                    );
                    let gear = if ind.params().1.is_empty() { 0.0 } else { 36.0 };
                    if ind.oscillator() {
                        text.draw(pm, "osc.", ((r.x + r.w - 12.0 - gear) as f32, (r.cy() + 4.0) as f32), 11.0, GREY, Align::Right);
                    }
                }
                Hit::NatGear(_) => {
                    if hover(&r) {
                        round_fill(pm, r, 6.0, pal::tile(), 1.0);
                    }
                    crate::phosphor::draw(pm, "gear", r.cx(), r.cy(), 18, crate::palette::ICON);
                }
                Hit::Row(i) => {
                    let ind = &INDICATORS[i];
                    if hover(&r) {
                        round_fill(pm, r, 6.0, pal::tile(), 1.0);
                    }
                    draw_icon(pm, ind.icon, r.x + 24.0, r.cy(), WHITE);
                    // libellé entier (coupé seulement s'il ne tient vraiment pas avant le compteur et la flèche)
                    let room = r.w - 56.0 - 44.0;
                    text.draw(
                        pm,
                        &text.fit(ind.label, 15.0, room),
                        ((r.x + 56.0) as f32, (r.cy() + 5.0) as f32),
                        15.0,
                        WHITE,
                        Align::Left,
                    );
                    lines(pm, r.x + r.w - 16.0, r.cy(), &[&[(-2.5, -4.5), (2.0, 0.0), (-2.5, 4.5)]], GREY, 1.4);
                    if let Some(n) = [0usize, 1].into_iter().find(|&k| k == i).map(|k| self.counts[k]) {
                        text.draw(pm, &n.to_string(), ((r.x + r.w - 34.0) as f32, (r.cy() + 5.0) as f32), 13.0, GREY, Align::Right);
                    }
                }
                Hit::Guardian => {
                    let label = if t.guardian {
                        "✓ Gardien PropFirm actif sur ce compte"
                    } else {
                        "Activer le gardien PropFirm sur ce compte"
                    };
                    text.draw(pm, label, (r.x as f32, (r.y + 16.0) as f32), 13.5, crate::palette::cyan(), Align::Left);
                }
                Hit::PxStep(d) => {
                    let c = if hover(&r) { WHITE } else { GREY };
                    let up = if d > 0 {
                        [(-3.5, 2.0), (0.0, -2.0), (3.5, 2.0)]
                    } else {
                        [(-3.5, -2.0), (0.0, 2.0), (3.5, -2.0)]
                    };
                    lines(pm, r.cx(), r.cy(), &[&up], c, 1.3);
                }
                Hit::CancelOrder(_) => {
                    // (the ✕ stays small; its touch area is the whole 40 pt square)
                    if hover(&r) {
                        round_fill(pm, r, 6.0, pal::hover(), 1.0);
                    }
                    lines(
                        pm,
                        r.cx(),
                        r.cy(),
                        &[&[(-4.0, -4.0), (4.0, 4.0)], &[(4.0, -4.0), (-4.0, 4.0)]],
                        crate::palette::down(),
                        1.5,
                    );
                }
                Hit::Account => {}
                Hit::GuardToggle(i) => {
                    let c = &self.journal.cfg;
                    round_fill(pm, r, 6.0, if hover(&r) { pal::hover() } else { pal::field() }, 1.0);
                    round_stroke(pm, r, 6.0, pal::edge_soft(), 1.0);
                    let (on, label) = match i {
                        0 => (Some(c.need_sl), "SL obligatoire".to_string()),
                        1 => (Some(c.news), "News ±2 min".to_string()),
                        3 => (Some(c.tilt_revenge), "Revenge".to_string()),
                        _ => (None, format!("Reset {}", reset_city(&c.reset_tz))),
                    };
                    let mut tx = r.x + 8.0;
                    if let Some(on) = on {
                        let b = Rect::new(r.x + 7.0, r.cy() - 6.0, 12.0, 12.0);
                        round_fill(pm, b, 3.0, if on { crate::palette::accent() } else { pal::card() }, 1.0);
                        round_stroke(pm, b, 3.0, if on { crate::palette::accent() } else { pal::edge_soft() }, 1.0);
                        if on {
                            lines(pm, b.cx(), b.cy(), &[&[(-3.0, 0.0), (-1.0, 2.5), (3.0, -2.5)]], WHITE, 1.4);
                        }
                        tx = b.x + b.w + 5.0;
                    }
                    text.draw(
                        pm,
                        &text.fit(&label, 11.5, r.x + r.w - tx - 4.0),
                        (tx as f32, (r.cy() + 4.0) as f32),
                        11.5,
                        WHITE,
                        Align::Left,
                    );
                }
                Hit::GuardUnlock => {
                    round_fill(pm, r, 6.0, if hover(&r) { pal::hover() } else { pal::tile() }, 1.0);
                    round_stroke(pm, r, 6.0, crate::palette::down(), 1.0);
                    crate::phosphor::draw(pm, "lock-key", r.x + 18.0, r.cy(), 16, crate::palette::down());
                    text.draw(
                        pm,
                        "Déverrouiller aujourd'hui",
                        (r.cx() as f32, (r.cy() + 5.0) as f32),
                        13.0,
                        WHITE,
                        Align::Center,
                    );
                }
                Hit::LotMode(m) => {
                    let on = t.to_lots == m;
                    round_fill(pm, r, r.h / 2.0, if on { pal::hover() } else { pal::card() }, 1.0);
                    round_stroke(pm, r, r.h / 2.0, if on { GREY } else { pal::edge_soft() }, 1.0);
                    text.draw(
                        pm,
                        if m { "$ → lots" } else { "lots → $" },
                        (r.cx() as f32, (r.cy() + 5.0) as f32),
                        13.0,
                        WHITE,
                        Align::Center,
                    );
                }
                Hit::Step(_, d) | Hit::GuardStep(_, d) => {
                    let c = if hover(&r) { WHITE } else { GREY };
                    let up = if d > 0 {
                        [(-3.5, 2.0), (0.0, -2.0), (3.5, 2.0)]
                    } else {
                        [(-3.5, -2.0), (0.0, 2.0), (3.5, -2.0)]
                    };
                    lines(pm, r.cx(), r.cy(), &[&up], c, 1.3);
                }
                Hit::Margin(i) => {
                    round_fill(pm, r, 6.0, if t.margin == i || hover(&r) { pal::edge_dark() } else { pal::card() }, 1.0);
                    // the demo's margin is isolated; the leverage of the market (spot: none)
                    let s = match i {
                        0 => "Isolée".to_string(),
                        1 if t.levier_max > 0.0 => argos_core::perp::levier_txt(t.levier.min(t.levier_max)),
                        1 => "Comptant".to_string(),
                        _ => "Démo".to_string(),
                    };
                    text.draw(pm, &s, (r.cx() as f32, (r.cy() + 5.0) as f32), 14.5, WHITE, Align::Center);
                }
                Hit::Tab(i) => {
                    let on = t.tab == i;
                    text.draw(
                        pm,
                        ["Market", "Limit", "Stop"][i],
                        ((r.x + 4.0) as f32, (r.cy() + 4.0) as f32),
                        14.5,
                        if on { WHITE } else { GREY },
                        Align::Left,
                    );
                    if on {
                        fill(pm, r.x, r.y + r.h - 2.0, 60.0, 2.0, WHITE, 1.0);
                    }
                }
                Hit::Side(b) => {
                    let on = t.buy == b;
                    let buy_c = buy_teal();
                    let (bg, fg) = match (on, b) {
                        (true, true) => (buy_c, crate::palette::ink_on(buy_c)),
                        (true, false) => (crate::palette::down(), crate::palette::ink_on(crate::palette::down())),
                        _ => (pal::card(), GREY),
                    };
                    round_fill(pm, r, 6.0, bg, 1.0);
                    text.draw(
                        pm,
                        if b { "Buy / Long" } else { "Sell / Short" },
                        (r.cx() as f32, (r.cy() + 5.0) as f32),
                        14.5,
                        fg,
                        Align::Center,
                    );
                }
                Hit::Order => {
                    // bouton actif : aplat plein de la couleur du côté choisi, texte le plus contrasté
                    let fill_c = if t.buy { buy_teal() } else { crate::palette::down() };
                    round_fill(pm, r, 6.0, fill_c, 1.0);
                    text.draw(
                        pm,
                        ["Passer l'ordre (démo)", "Placer l'ordre Limit (démo)", "Placer l'ordre Stop (démo)"][t.tab.min(2)],
                        (r.cx() as f32, (r.cy() + 5.0) as f32),
                        14.5,
                        crate::palette::ink_on(fill_c),
                        Align::Center,
                    );
                }
                Hit::CloseAll => {
                    // 🫧 a glass pill (red tint under the pointer)
                    crate::jetons::verre(pm, r, crate::jetons::pilule(r));
                    if hover(&r) {
                        round_fill(pm, r, crate::jetons::pilule(r), Rgb(0x4a, 0x1c, 0x22), 0.6);
                    }
                    let pnl = t.open.iter().map(|o| o.1).sum::<f64>();
                    let couleur = if t.open.is_empty() { WHITE } else { crate::palette::pnl(pnl) };
                    text.draw(
                        pm,
                        &text.fit(&close_all_txt(t.open.len(), pnl, t.pending.len()), 13.5, r.w - 16.0),
                        (r.cx() as f32, (r.cy() + 5.0) as f32),
                        13.5,
                        couleur,
                        Align::Center,
                    );
                }
                Hit::Bottom(i) => {
                    if t.bottom == i {
                        round_fill(pm, r, 8.0, pal::hover(), 1.0);
                    }
                    text.draw(
                        pm,
                        ["Marchés", "Trade", "Compte"][i],
                        (r.cx() as f32, (r.cy() + 5.0) as f32),
                        15.0,
                        if t.bottom == i { WHITE } else { GREY },
                        Align::Center,
                    );
                }
            }
        }
        match kind {
            Kind::Trading => {}
            Kind::Journal => self.journal.draw(pm, text, pw, mouse, th.grid, crate::journal::now_ms()),
            Kind::Indicators => {}
        }
    }

    /// 🛡️ Fond de la carte du gardien : état, perte du jour, perte max, positions, risque max, verrou, et les libellés
    /// des réglages (▲ ▼ et cases dessinés par leurs zones).
    fn draw_guard(&self, pm: &mut Pixmap, text: &Text, gc: Rect, zones: &[(Rect, Hit)]) {
        use crate::journal::GuardLevel;
        let open: f64 = self.trading.open.iter().map(|o| o.1).sum();
        let st = self.journal.guard_card(open, self.trading.open.len(), crate::journal::now_ms());
        let (edge, icon, state) = match st.level() {
            GuardLevel::Ok => (pal::edge_soft(), crate::palette::up(), "dans les règles"),
            GuardLevel::Half => (crate::palette::GOLD, crate::palette::GOLD, "moitié d'une limite"),
            GuardLevel::Warn => (crate::palette::ORANGE, crate::palette::ORANGE, "prudence"),
            GuardLevel::Bad if st.locked => (crate::palette::down(), crate::palette::down(), "journée verrouillée"),
            GuardLevel::Bad => (crate::palette::down(), crate::palette::down(), "limite atteinte"),
        };
        round_fill(pm, gc, crate::jetons::R_GARDIEN, pal::card(), 1.0);
        round_stroke(pm, gc, crate::jetons::R_GARDIEN, edge, if st.level() == GuardLevel::Ok { 1.0 } else { 1.6 });
        crate::phosphor::draw(pm, "shield-check", gc.x + 20.0, gc.y + 15.0, 18, icon);
        text.draw(
            pm,
            &text.fit(&format!("Gardien PropFirm · {state}"), 13.0, gc.w - 44.0 - 86.0),
            ((gc.x + 36.0) as f32, (gc.y + 20.0) as f32),
            13.0,
            WHITE,
            Align::Left,
        );
        // ☀ Routine (à droite du titre)
        let routine = Rect::new(gc.x + gc.w - 82.0, gc.y + 5.0, 72.0, 20.0);
        round_fill(pm, routine, 10.0, pal::tile(), 1.0);
        round_stroke(pm, routine, 10.0, pal::edge_soft(), 1.0);
        text.draw(pm, "☀ Routine", (routine.cx() as f32, (routine.cy() + 4.0) as f32), 11.0, WHITE, Align::Center);
        let m = |v: f64| crate::journal::money(Some(v));
        // perte du jour et sa jauge, avec le trait du seuil d'alerte (80 % par défaut)
        let mut y = gc.y + 28.0;
        text.draw(pm, "Perte du jour", ((gc.x + 12.0) as f32, (y + 13.0) as f32), 12.5, WHITE, Align::Left);
        text.draw(
            pm,
            &format!("{} / {}", m(st.day_loss), m(st.day_lim)),
            ((gc.x + gc.w - 12.0) as f32, (y + 13.0) as f32),
            12.5,
            WHITE,
            Align::Right,
        );
        let j = Rect::new(gc.x + 12.0, y + 21.0, gc.w - 24.0, 8.0);
        round_fill(pm, j, 4.0, pal::field(), 1.0);
        let part = if st.day_lim > 0.0 { (st.day_loss / st.day_lim).clamp(0.0, 1.0) } else { 0.0 };
        let seuil = if st.warn > 0.0 { st.warn / 100.0 } else { 0.8 };
        if part > 0.0 {
            let c = if part >= seuil {
                crate::palette::down()
            } else if part >= 0.5 {
                crate::palette::ORANGE
            } else {
                crate::palette::up()
            };
            round_fill(pm, Rect::new(j.x, j.y, (j.w * part).max(8.0), j.h), 4.0, c, 1.0);
        }
        round_fill(
            pm,
            Rect::new(j.x + j.w * seuil.clamp(0.0, 1.0) - 1.0, j.y - 3.0, 2.0, 14.0),
            1.0,
            crate::palette::ORANGE,
            1.0,
        );
        y += 36.0;
        // Perte max / Positions / Risque max : trois cases
        let rows = st.rows();
        let w3 = (gc.w - 24.0) / 3.0;
        for (k, (lab, val)) in rows[1..].iter().enumerate() {
            let cx = gc.x + 12.0 + w3 * k as f64;
            text.draw(pm, lab, (cx as f32, (y + 10.0) as f32), 10.0, GREY, Align::Left);
            text.draw(pm, &text.fit(val, 11.5, w3 - 4.0), (cx as f32, (y + 25.0) as f32), 11.5, WHITE, Align::Left);
        }
        y += 34.0;
        // ⚠ prudence
        // (ou 🔓 la journée déverrouillée aujourd'hui, avec sa raison notée au Journal)
        if let Some((s, alerte)) = self.cell_garde().message() {
            let mh = self.cell_garde().msg_h();
            let r = Rect::new(gc.x + 10.0, y, gc.w - 20.0, mh - 6.0);
            round_fill(pm, r, 8.0, if alerte { crate::palette::ORANGE } else { crate::palette::up() }, 0.14);
            for (k, ligne) in s.lines().enumerate() {
                text.draw(
                    pm,
                    &text.fit(ligne, 11.5, r.w - 16.0),
                    ((r.x + 8.0) as f32, (r.y + 18.5 + 16.0 * k as f64) as f32),
                    11.5,
                    WHITE,
                    Align::Left,
                );
            }
            y += mh;
        }
        // Règles : 📋 Mon plan › · la PropFirm choisie
        let regles = if self.journal.cfg.firm.is_empty() {
            "règles par défaut".to_string()
        } else {
            self.journal.cfg.firm.clone()
        };
        text.draw(pm, "Règles :", ((gc.x + 12.0) as f32, (y + 15.0) as f32), 11.0, DIM, Align::Left);
        text.draw(pm, "📋 Mon plan ›", ((gc.x + 60.0) as f32, (y + 15.0) as f32), 11.0, TEAL, Align::Left);
        text.draw(
            pm,
            &text.fit(&format!("· {regles}"), 11.0, gc.w - 168.0),
            ((gc.x + 156.0) as f32, (y + 15.0) as f32),
            11.0,
            DIM,
            Align::Left,
        );
        y += 22.0;
        if st.locked {
            text.draw(
                pm,
                &text.fit("🔒 Journée verrouillée jusqu'à minuit (heure du reset)", 12.0, gc.w - 24.0),
                ((gc.x + 12.0) as f32, (y + 15.0) as f32),
                12.0,
                crate::palette::down(),
                Align::Left,
            );
        }
        // réglages chiffrés : un champ par réglage, ▲ ▼ à droite (dessinés par leurs zones)
        let c = &self.journal.cfg;
        for (r, hit) in zones {
            if let Hit::GuardStep(k, 1) = *hit {
                let w = (gc.w - 20.0 - 12.0) / 3.0;
                let field = Rect::new(r.x + r.w - w, r.y, w, 26.0);
                round_fill(pm, field, 6.0, pal::field(), 1.0);
                round_stroke(pm, field, 6.0, pal::edge_soft(), 1.0);
                let (lab, unit) = GUARD_STEPS[k];
                let v = guard_value(c, k);
                text.draw(
                    pm,
                    &text.fit(&format!("{lab} {v}{unit}"), 11.5, w - 26.0),
                    ((field.x + 7.0) as f32, (field.cy() + 4.0) as f32),
                    11.5,
                    WHITE,
                    Align::Left,
                );
            }
        }
    }

    fn draw_trading(&self, pm: &mut Pixmap, text: &Text, pw: f64, lignes: &[Rect]) {
        let t = &self.trading;
        let inner = pw - 2.0 * PAD;
        // compte démo
        let acc = Rect::new(pw / 2.0 - 130.0, TOP + 6.0, 260.0, 36.0);
        round_fill(pm, acc, 18.0, pal::card(), 1.0);
        round_stroke(pm, acc, 18.0, pal::edge_soft(), 1.0);
        round_fill(pm, Rect::new(acc.x + 16.0, acc.cy() - 3.5, 7.0, 7.0), 3.5, TEAL, 1.0);
        let bal = crate::viewport::format_price(t.balance, 0);
        text.draw(
            pm,
            &format!("{} · ${bal} ⌄", if t.account.is_empty() { "Argos 100K" } else { &t.account }),
            ((acc.x + 32.0) as f32, (acc.cy() + 5.0) as f32),
            14.0,
            WHITE,
            Align::Left,
        );
        // 🛡️ carte du gardien (s'il est actif), puis le ticket en dessous
        let dy = self.guard_dy(pw);
        if let Some((gc, zones)) = self.guard_layout(pw) {
            self.draw_guard(pm, text, gc, &zones);
        }
        // carte du calcul de lot
        let card = Rect::new(PAD, TOP + 80.0 + dy, inner, 150.0);
        round_fill(pm, card, 8.0, Rgb(0x14, 0x22, 0x22), 1.0);
        round_stroke(pm, card, 8.0, pal::edge_soft(), 1.0);
        text.draw(
            pm,
            "Calcul de lot · BTC : 1 pip = 1 · 1 $ / pip / lot",
            ((card.x + 12.0) as f32, (card.y + 24.0) as f32),
            13.0,
            WHITE,
            Align::Left,
        );
        let fy = card.y + 72.0;
        let fw = inner / 2.0 - 24.0;
        let (first_label, first) = if t.to_lots {
            ("Risque $", format!("{}", t.risk))
        } else {
            ("Lots", format!("{:.1}", t.lots))
        };
        for (k, (lab, val)) in [(first_label, first), ("SL (pips)", format!("{}", t.sl_pips))].into_iter().enumerate() {
            let x = card.x + 12.0 + k as f64 * inner / 2.0;
            text.draw(pm, lab, (x as f32, (fy + 17.0) as f32), 12.5, GREY, Align::Left);
            let lw = text.width(lab, 12.5) as f64 + 8.0;
            let f = Rect::new(x + lw, fy, fw - lw, 26.0);
            round_fill(pm, f, 4.0, pal::field(), 1.0);
            round_stroke(pm, f, 4.0, pal::edge_soft(), 1.0);
            text.draw(pm, &val, ((f.x + 8.0) as f32, (f.cy() + 5.0) as f32), 13.5, WHITE, Align::Left);
        }
        let res = if t.to_lots {
            format!("= {:.2} lot", t.result())
        } else {
            format!("= {} $ de risque", crate::viewport::format_price(t.result(), 2))
        };
        text.draw(pm, &res, ((card.x + 12.0) as f32, (card.y + 124.0) as f32), 15.0, WHITE, Align::Left);
        // 💼 la position dessinée sur la chart (liée au ticket), sinon le repère PropFirm
        let note = match t.plan {
            Some((e, st, tg)) => {
                let rr = if (e - st).abs() > 0.0 { (tg - e).abs() / (e - st).abs() } else { 0.0 };
                let f = |v: f64| crate::viewport::format_price(v, 2);
                format!(
                    "{} {} · entrée {} · stop {} · cible {} · RR {rr:.2}",
                    if t.buy { "▲ Long" } else { "▼ Short" },
                    t.sym,
                    f(e),
                    f(st),
                    f(tg)
                )
            }
            None => "Dessine une position longue / courte : le ticket se remplit".to_string(),
        };
        text.draw(
            pm,
            &text.fit(&note, 11.5, inner - 24.0),
            ((card.x + 12.0) as f32, (card.y + 143.0) as f32),
            11.5,
            if t.plan.is_some() { GOLD } else { GREY },
            Align::Left,
        );
        // disponible / position (démo)
        // Limit / Stop : le champ du prix de l'ordre (sous les onglets)
        let fy = TOP + 80.0 + dy + 168.0 + 56.0 + 50.0;
        if t.tab != 0 {
            let f = Rect::new(PAD, fy, inner, 32.0);
            round_fill(pm, f, 6.0, pal::field(), 1.0);
            round_stroke(pm, f, 6.0, pal::edge_soft(), 1.0);
            let lab = if t.tab == 1 { "Prix limite" } else { "Prix stop" };
            text.draw(pm, lab, ((f.x + 10.0) as f32, (f.cy() + 5.0) as f32), 13.0, GREY, Align::Left);
            text.draw(
                pm,
                &crate::viewport::format_price(t.px, 2),
                ((f.x + f.w - 34.0) as f32, (f.cy() + 5.0) as f32),
                14.0,
                WHITE,
                Align::Right,
            );
        }
        let y = fy + t.px_dy() + 72.0;
        let avail = crate::viewport::format_price(t.balance, 0);
        let cur = match t.open.len() {
            0 => format!("0 {}", t.sym),
            1 => format!("{} ({:+.2} $)", t.open[0].0, t.open[0].1),
            n => format!("{n} positions ({:+.2} $)", t.open.iter().map(|o| o.1).sum::<f64>()),
        };
        // 💰 la position : verte si elle gagne, rouge si elle perd
        let pos_c = if t.open.is_empty() {
            WHITE
        } else {
            crate::palette::pnl(t.open.iter().map(|o| o.1).sum::<f64>())
        };
        for (k, (a, b, c)) in [("Disponible", format!("{avail} USDC"), WHITE), ("Position actuelle", cur, pos_c)]
            .into_iter()
            .enumerate()
        {
            let yy = y + k as f64 * 28.0;
            text.draw(pm, a, (PAD as f32, yy as f32), 14.0, WHITE, Align::Left);
            text.draw(pm, &b, ((pw - PAD) as f32, yy as f32), 14.0, c, Align::Right);
        }
        text.draw(
            pm,
            "Démo seulement : aucun ordre réel ne part d'ici.",
            (PAD as f32, (y + 62.0) as f32),
            12.0,
            GOLD,
            Align::Left,
        );
        // ordres en attente (le ✕ de chaque ligne est une zone du panneau)
        if let Some(premiere) = lignes.first() {
            text.draw(
                pm,
                &format!("Ordres en attente ({})", t.pending.len()),
                (PAD as f32, (premiere.y - 7.0) as f32),
                12.5,
                GREY,
                Align::Left,
            );
            for (r, (_, s)) in lignes.iter().zip(&t.pending) {
                text.draw(
                    pm,
                    &text.fit(s, 12.5, inner - TOUCHER - 8.0),
                    (PAD as f32, (r.cy() + 4.5) as f32),
                    12.5,
                    WHITE,
                    Align::Left,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn top_gagnants_et_perdants() {
        let r = |c: &str, p: f64, v: f64| (c.to_string(), p, v);
        let rows = vec![
            r("BTC", 2.0, 1e9),
            r("ETH", -3.0, 1e9),
            r("PUMP", 40.0, 1e3),
            r("SOL", 8.0, 5e8),
            r("XRP", -1.0, 2e8),
            r("ZEC", 0.0, 1e8),
        ];
        let (up, down) = super::movers(&rows, 10, 1e6);
        assert_eq!(up, vec![("SOL".to_string(), 8.0), ("BTC".to_string(), 2.0)], "PUMP : trop petit volume");
        assert_eq!(down, vec![("ETH".to_string(), -3.0), ("XRP".to_string(), -1.0)]);
        assert_eq!(super::movers(&rows, 1, 1e6).0.len(), 1);
    }
    use super::*;

    #[test]
    fn recherche_des_indicateurs() {
        let mut p = Panels::default();
        let all = p
            .layout(Kind::Indicators, 320.0, 800.0)
            .iter()
            .filter(|(_, h)| matches!(h, Hit::Nat(_)))
            .count();
        assert_eq!(p.click(Kind::Indicators, 100.0, TOP + 20.0, 320.0, 800.0), PanelClick::Changed);
        for c in "VWÂP".chars() {
            p.search_key(Some(c));
        }
        p.search_key(Some('x'));
        p.search_key(None);
        assert_eq!(p.search.as_deref(), Some("VWÂP"));
        let names: Vec<&str> = p
            .layout(Kind::Indicators, 320.0, 800.0)
            .iter()
            .filter_map(|(_, h)| match h {
                Hit::Nat(k) => Some(crate::indicators::Ind::ALL[*k].name()),
                _ => None,
            })
            .collect();
        assert!(
            !names.is_empty() && names.len() < all && names.iter().all(|n| n.contains("VWAP")),
            "sans majuscules ni accents : {names:?}"
        );
    }

    #[test]
    fn calcul_de_lot() {
        let mut t = Trading::default();
        assert!((t.result() - 5.0).abs() < 1e-9, "250 $ / (50 pips × 1 $) = 5 lots (comme v1)");
        t.step(0, 1);
        assert_eq!(t.risk, 300.0);
        t.step(1, -1);
        assert_eq!(t.sl_pips, 45.0);
        t.to_lots = false;
        t.lots = 0.5;
        t.sl_pips = 50.0;
        assert!((t.result() - 25.0).abs() < 1e-9, "0,5 lot × 50 pips × 1 $ = 25 $");
        t.sl_pips = 0.0;
        t.to_lots = true;
        assert_eq!(t.result(), 0.0, "stop à 0 : pas de division par zéro");
    }

    #[test]
    fn clics_des_panneaux() {
        let mut p = Panels::default();
        let (pw, h) = (340.0, 900.0);
        let ai = Panels::ai_rect(pw);
        for k in [Kind::Trading, Kind::Journal] {
            assert_eq!(p.click(k, ai.cx(), ai.cy(), pw, h), PanelClick::Ai);
        }
        assert_eq!(
            p.click(Kind::Indicators, ai.cx(), ai.cy(), pw, h),
            PanelClick::None,
            "pas de bouton AI dans Indicateurs"
        );
        // Indicateurs : « Mes scripts » ouvre l'éditeur, une ligne pas encore faite = bientôt
        let lay = p.layout(Kind::Indicators, pw, h);
        let row = |k| lay.iter().find(|(_, z)| *z == Hit::Row(k)).unwrap().0;
        let r = row(1);
        assert_eq!(p.click(Kind::Indicators, r.cx(), r.cy(), pw, h), PanelClick::Script(ScriptOpen::Mine));
        let r = row(4);
        assert!(matches!(p.click(Kind::Indicators, r.cx(), r.cy(), pw, h), PanelClick::Soon(s) if s.starts_with("Données techniques")));
        // Trading : lots → $, puis vente, puis gardien
        let z = p.layout(Kind::Trading, pw, h);
        let at = |hit: Hit| z.iter().find(|(_, x)| *x == hit).unwrap().0;
        let r = at(Hit::LotMode(false));
        assert_eq!(p.click(Kind::Trading, r.cx(), r.cy(), pw, h), PanelClick::Changed);
        assert!(!p.trading.to_lots);
        let r = at(Hit::Side(false));
        p.click(Kind::Trading, r.cx(), r.cy(), pw, h);
        assert!(!p.trading.buy);
        let r = at(Hit::Guardian);
        p.click(Kind::Trading, r.cx(), r.cy(), pw, h);
        assert!(p.trading.guardian);
        let r = at(Hit::Order);
        assert_eq!(p.click(Kind::Trading, r.cx(), r.cy(), pw, h), PanelClick::Order, "ordre démo (jamais réel)");
        // « Fermer les positions » : seulement quand il y a des positions démo ouvertes (gardien éteint : sa carte
        // pousserait le ticket plus bas que ce panneau de 900 px)
        p.trading.guardian = false;
        assert!(!z.iter().any(|(_, x)| *x == Hit::CloseAll), "rien à fermer");
        p.trading.open = vec![("▲ BTC 0.1000".into(), 25.0)];
        let z = p.layout(Kind::Trading, pw, h);
        let r = z.iter().find(|(_, x)| *x == Hit::CloseAll).expect("bouton Fermer").0;
        assert!(r.y + r.h <= h - 112.0, "au-dessus du bouton d'ordre");
        assert_eq!(p.click(Kind::Trading, r.cx(), r.cy(), pw, h), PanelClick::CloseAll);
        // 👆 the button and each ✕: 40 pt touch areas, and the orders keep `ECART` above the order button
        let toucher = |z: &[(Rect, Hit)], h: &Hit| z.iter().find(|(_, x)| x == h).map(|(r, _)| *r).unwrap();
        assert!(toucher(&z, &Hit::CloseAll).h >= TOUCHER);
        // only pending orders: the button is there and cancels them
        p.trading.open.clear();
        p.trading.pending = vec![(7, "Limit ▲ BTC 0.1000 @ 90 000".into())];
        let z = p.layout(Kind::Trading, pw, h);
        let r = z.iter().find(|(_, x)| *x == Hit::CloseAll).expect("bouton avec des ordres seulement").0;
        assert_eq!(p.click(Kind::Trading, r.cx(), r.cy(), pw, h), PanelClick::CloseAll);
        // 📸 ARGOS_TOUT_FERMER_PNG=folder: the ticket with only a pending order, to look at it
        if let Ok(dir) = std::env::var("ARGOS_TOUT_FERMER_PNG") {
            let mut pm = crate::px::new(pw as u32, h as u32).unwrap();
            p.draw(Kind::Trading, &mut pm, &Text::default(), (pw, h), None, &Theme::default());
            pm.save_png(format!("{dir}/tout-fermer-ordres.png")).unwrap();
        }
        assert_eq!(close_all_txt(0, 0.0, 1), "Annuler 1 ordre en attente");
        assert_eq!(close_all_txt(2, 25.0, 0), "Fermer 2 positions démo (+25.00 $)");
        assert_eq!(close_all_txt(1, -5.0, 3), "Tout fermer : 1 position démo (-5.00 $) + 3 ordres en attente");
        p.trading.pending.clear();
        // Journal : période 30 j
        p.click(Kind::Journal, PAD + 44.0 + 6.0 + 25.0, TOP + 50.0 + 13.0, pw, h);
        assert_eq!(p.journal.period, 1);
        // dessin des 3 sans planter
        for k in [Kind::Indicators, Kind::Trading, Kind::Journal] {
            let mut pm = crate::px::new(340, 900).unwrap();
            p.draw(k, &mut pm, &Text::default(), (pw, h), Some((100.0, 100.0)), &Theme::default());
        }
    }

    /// 👆 Pending orders on a phone-sized ticket: « Tout fermer » and every ✕ are 40 pt touch areas inside the panel,
    /// none overlaps another, and the last order stays `ECART` above the order button (orders that do not fit are
    /// left out, never drawn under the button).
    #[test]
    fn ordres_en_attente_touchables() {
        for (pw, h) in [(390.0, 760.0), (390.0, 874.0), (340.0, 1200.0)] {
            let mut p = Panels::default();
            p.trading.pending = (0..6).map(|k| (k, format!("Buy Limit BTC @ 62 3{k}0,00"))).collect();
            p.trading.open = vec![("▲ BTC 0.1000".into(), -5.0)];
            let z = p.layout(Kind::Trading, pw, h);
            let ordre = z.iter().find(|(_, x)| *x == Hit::Order).unwrap().0;
            let mut cibles: Vec<Rect> = z
                .iter()
                .filter(|(_, x)| matches!(x, Hit::CloseAll | Hit::CancelOrder(_)))
                .map(|(r, _)| *r)
                .collect();
            assert!(!cibles.is_empty(), "{pw}×{h}");
            cibles.sort_by(|a, b| a.y.total_cmp(&b.y));
            for (k, r) in cibles.iter().enumerate() {
                assert!(r.w >= TOUCHER && r.h >= TOUCHER, "{pw}×{h}: 40 pt touch area {r:?}");
                assert!(r.x >= 0.0 && r.x + r.w <= pw, "{pw}×{h}: inside the panel {r:?}");
                assert!(r.y + r.h + ECART <= ordre.y, "{pw}×{h}: above the order button {r:?} {ordre:?}");
                if let Some(suivante) = cibles.get(k + 1) {
                    assert!(r.y + r.h <= suivante.y, "{pw}×{h}: no overlap {r:?} {suivante:?}");
                }
            }
            let mut pm = crate::px::new(pw as u32, h as u32).unwrap();
            p.draw(Kind::Trading, &mut pm, &Text::default(), (pw, h), None, &Theme::default());
        }
    }

    #[test]
    fn carte_et_reglages_du_gardien() {
        let mut p = Panels::default();
        let (pw, h) = (340.0, 1200.0);
        let order_y = |p: &Panels| p.layout(Kind::Trading, pw, h).iter().find(|(_, x)| *x == Hit::LotMode(true)).unwrap().0.y;
        let before = order_y(&p);
        assert!(p.guard_layout(pw).is_none(), "gardien éteint : pas de carte");
        p.trading.guardian = true;
        assert!(order_y(&p) > before, "la carte pousse le ticket vers le bas");
        assert!(p.content_h(Kind::Trading, pw) > 700.0);
        let at = |p: &Panels, hit: Hit| p.layout(Kind::Trading, pw, h).into_iter().find(|(_, x)| *x == hit).map(|z| z.0);
        // ▲ risque max : 1 % → 1,25 %
        let r = at(&p, Hit::GuardStep(2, 1)).unwrap();
        assert_eq!(p.click(Kind::Trading, r.cx(), r.cy(), pw, h), PanelClick::GuardCfg);
        assert_eq!(p.journal.cfg.risk_pct, 1.25);
        // ▼ positions max : 2 → 1 ; ▲ alerte jamais au-dessus de 100 %
        let r = at(&p, Hit::GuardStep(3, -1)).unwrap();
        p.click(Kind::Trading, r.cx(), r.cy(), pw, h);
        assert_eq!(p.journal.cfg.max_pos, 1);
        for _ in 0..10 {
            guard_step(&mut p.journal.cfg, 5, 1);
        }
        assert_eq!(p.journal.cfg.warn, 100.0);
        // cases : SL obligatoire, news, fuseau du reset
        for i in 0..3 {
            let r = at(&p, Hit::GuardToggle(i)).unwrap();
            assert_eq!(p.click(Kind::Trading, r.cx(), r.cy(), pw, h), PanelClick::GuardCfg);
        }
        assert!(!p.journal.cfg.need_sl && !p.journal.cfg.news);
        assert_eq!(p.journal.cfg.reset_tz, "America/New_York");
        // 🧊 anti-tilt : ▲ pertes de suite 0 → 1, ▼ pause 30 → 25 min, case « Revenge »
        for hit in [Hit::GuardStep(6, 1), Hit::GuardStep(7, -1), Hit::GuardToggle(3)] {
            let r = at(&p, hit).unwrap();
            assert_eq!(p.click(Kind::Trading, r.cx(), r.cy(), pw, h), PanelClick::GuardCfg);
        }
        let c = &p.journal.cfg;
        assert_eq!((c.tilt_n, c.tilt_min, c.tilt_revenge), (1, 25, true));
        for _ in 0..60 {
            guard_step(&mut p.journal.cfg, 7, -1);
            guard_step(&mut p.journal.cfg, 6, 1);
        }
        assert_eq!((p.journal.cfg.tilt_min, p.journal.cfg.tilt_n), (5, 10), "bornes : 5 min, 10 pertes");
        // ☀ Routine et 📋 Mon plan de la carte répondent ; à 82 % de la perte du jour, le message ⚠ fait descendre le
        // reste (règles, réglages, ticket)
        for hit in [Hit::Cell(cell::Cell::Routine), Hit::Cell(cell::Cell::Plan)] {
            let r = at(&p, hit).unwrap();
            assert!(matches!(p.click(Kind::Trading, r.cx(), r.cy(), pw, h), PanelClick::Soon(_)), "{hit:?}");
        }
        let plan = at(&p, Hit::Cell(cell::Cell::Plan)).unwrap().y;
        let lot = order_y(&p);
        p.journal.cfg.warn = 80.0; // monté à 100 % plus haut
        p.trading.open = vec![("BTC".into(), -4_100.0)];
        assert_eq!(at(&p, Hit::Cell(cell::Cell::Plan)).unwrap().y, plan + cell::MSG_H);
        assert_eq!(order_y(&p), lot + cell::MSG_H);
        let mut pm = crate::px::new(340, 1200).unwrap();
        p.draw(Kind::Trading, &mut pm, &Text::default(), (pw, h), None, &Theme::default());
        p.trading.open.clear();
        // journée verrouillée : le bouton « Déverrouiller aujourd'hui » apparaît
        assert!(at(&p, Hit::GuardUnlock).is_none());
        p.journal.cfg.lock = p.journal.propfirm_day(crate::journal::now_ms());
        let r = at(&p, Hit::GuardUnlock).expect("bouton Déverrouiller");
        assert_eq!(p.click(Kind::Trading, r.cx(), r.cy(), pw, h), PanelClick::Unlock);
        let mut pm = crate::px::new(340, 1200).unwrap();
        p.draw(Kind::Trading, &mut pm, &Text::default(), (pw, h), Some((100.0, 200.0)), &Theme::default());
    }
}
