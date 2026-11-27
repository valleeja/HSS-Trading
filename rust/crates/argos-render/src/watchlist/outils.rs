//! Outils de la barre d'icônes de la watchlist, réglages gardés entre deux lancements et couleurs des drapeaux.

use super::*;

/// Outils de la barre d'icônes, comme Argos v1 : watchlist, indicateurs, trading démo, journal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Tool {
    #[default]
    Watchlist,
    Indicators,
    Trading,
    Journal,
    Exchanges,
    /// 🛒 Marketplace Argos.
    Marketplace,
    /// 📊 Marché crypto (CMC).
    Market,
    /// Argos AI (gardé pour les réglages déjà sauvegardés ; l'assistant s'ouvre par ✨).
    Ai,
    /// 📅 Agenda (calendrier économique, news PropFirm) — dans la barre d'icônes et dans Panneaux.
    Agenda,
    /// 📰 Éco / News — dans la barre d'icônes et dans Panneaux.
    News,
    /// 🐋 Baleines (grosses transactions) — ouvert par Panneaux.
    Whales,
    /// 🧩 Réglages des modules (colonne du grand écran) — ouvert par Réglages.
    Modules,
    /// 🖥 🔔 Notifications (volet de la maquette `#alertes`, ouvert par 🔔 de la barre du haut). Hors de la barre
    /// d'icônes ; enregistré comme `Watchlist` (l'ancienne /app/ lit le même fichier et ne le connaît pas).
    Alertes,
    /// 🖥 🛟 Support (panel opened by 🛟 Support in the top bar). Outside the icon bar; saved as
    /// `Watchlist`, like Alertes.
    Support,
    /// Boutons de la maquette qui ouvrent autre chose qu'un volet (barre de droite, Jee 2026-11-09) : ✏ outils de
    /// dessin, 🛡 Gardien PropFirm (Simulateur), ✎ Argos Script, 🎓 Académie, 👥 Communauté, 💼 Portefeuille,
    /// 💥 Liquidations (l'indicateur sur la chart).
    Dessin,
    Gardien,
    Script,
    Academie,
    Communaute,
    Portfolio,
    Liquidations,
    /// ⚙ Réglages (les Panneaux), tout en bas de la barre.
    Reglages,
    /// 🧪 Démo (le Simulateur : compte démo, séance en replay) et 📖 Carnet d'ordres (son volet : Carnet / Delta / DOM),
    /// comme la maquette.
    Demo,
    Carnet,
    /// 🧭 Explorer Hyperliquid and 📶 Stats Hyperliquid (menu Argos ▾ « Marchés », Ctrl + K, column of modules):
    /// outside the icon bar; saved as `Watchlist`, like Alertes.
    Explorer,
    Stats,
    /// 🎯 Stratégie: the result of the strategy on the chart (Jee, 23 Nov). In the bar only while a strategy is on the
    /// chart (not in `ALL`); saved as `Watchlist`, like Alertes.
    Strategie,
}

impl Tool {
    /// Barre d'icônes par défaut, de haut en bas, comme la capture de Jee et la maquette (2026-11-09) : ☆ favoris,
    /// 📈 Trading, 🧪 Démo, 🛡 PropFirm, baleines, modules, Argos Script, Académie, Communauté, 🌐 marchés, Marketplace |
    /// calendrier, news, journal, portefeuille, 📖 carnet d'ordres, liquidations | ⚙ réglages (masqués par défaut :
    /// Indicateurs — ƒx est en haut — et les outils de dessin — barre de gauche). Les API s'ouvrent par 🔌 à côté du ＋.
    pub const ALL: [Tool; 20] = [
        Tool::Watchlist,
        Tool::Trading,
        Tool::Demo,
        Tool::Gardien,
        Tool::Whales,
        Tool::Modules,
        Tool::Script,
        Tool::Academie,
        Tool::Communaute,
        Tool::Market,
        Tool::Marketplace,
        Tool::Agenda,
        Tool::News,
        Tool::Journal,
        Tool::Portfolio,
        Tool::Carnet,
        Tool::Liquidations,
        Tool::Reglages,
        Tool::Indicators,
        Tool::Dessin,
    ];

    /// 🖥 Colonne PC par défaut : la barre de la maquette (DEFAUT), dans son ordre ; les autres outils s'ajoutent par ⭐
    /// dans « Personnaliser la barre ».
    pub const MAQUETTE: [Tool; 10] = [
        Tool::Trading,
        Tool::Demo,
        Tool::Gardien,
        Tool::Whales,
        Tool::Modules,
        Tool::Script,
        Tool::Academie,
        Tool::Communaute,
        Tool::Market,
        Tool::Marketplace,
    ];

    /// 🖥 L'icône de la maquette (ONGLETS : émojis, comme la barre du bas du cell).
    pub fn emoji(self) -> &'static str {
        match self {
            Tool::Watchlist => "☆",
            Tool::Trading => "📈",
            Tool::Demo => "🧪",
            Tool::Gardien => "🛡",
            Tool::Whales => "🐋",
            Tool::Modules => "🧩",
            Tool::Script => "✎",
            Tool::Academie => "🎓",
            Tool::Communaute => "👥",
            Tool::Market => "🌐",
            Tool::Marketplace => "🛒",
            Tool::Agenda => "📅",
            Tool::News => "📰",
            Tool::Alertes => "🔔",
            Tool::Support => "🛟",
            Tool::Journal => "📓",
            Tool::Portfolio => "💼",
            Tool::Carnet => "📖",
            Tool::Liquidations => "💥",
            Tool::Reglages => "⚙",
            Tool::Indicators => "📊",
            Tool::Dessin => "✏",
            Tool::Exchanges => "🔑",
            Tool::Ai => "✨",
            Tool::Explorer => "🧭",
            Tool::Stats => "📶",
            Tool::Strategie => "🎯",
        }
    }

    /// Masqués par défaut (pas dans la barre de la maquette ; toujours dans le menu de la barre).
    pub const CACHES: [Tool; 2] = [Tool::Indicators, Tool::Dessin];

    /// Un trait de séparation après ce bouton (capture de Jee : après la Marketplace et après les liquidations).
    pub fn separe(self) -> bool {
        matches!(self, Tool::Marketplace | Tool::Liquidations)
    }

    /// Ouvre autre chose qu'un volet (l'app s'en charge).
    pub fn action(self) -> bool {
        matches!(
            self,
            Tool::Marketplace
                | Tool::Dessin
                | Tool::Gardien
                | Tool::Script
                | Tool::Academie
                | Tool::Communaute
                | Tool::Portfolio
                | Tool::Liquidations
                | Tool::Reglages
                | Tool::Demo
        )
    }

    /// Couleur de l'icône (maquette : icônes colorées par famille).
    pub(super) fn couleur(self) -> Rgb {
        use crate::palette::IconCat as C;
        crate::palette::icon_color(match self {
            Tool::Watchlist | Tool::Agenda | Tool::News | Tool::Alertes | Tool::Support => C::Info,
            Tool::Trading | Tool::Journal | Tool::Portfolio | Tool::Strategie => C::Trading,
            Tool::Dessin => C::Draw,
            Tool::Gardien | Tool::Modules | Tool::Reglages => C::Settings,
            Tool::Demo => C::Trading,
            Tool::Carnet => C::Market,
            Tool::Indicators | Tool::Script | Tool::Liquidations => C::Analyse,
            Tool::Academie | Tool::Communaute | Tool::Ai => C::Assistant,
            Tool::Whales | Tool::Market | Tool::Marketplace | Tool::Exchanges | Tool::Explorer | Tool::Stats => C::Market,
        })
    }

    /// Panneau (hors watchlist) de cet outil.
    pub(super) fn panel(self) -> Option<Kind> {
        match self {
            Tool::Watchlist => None,
            Tool::Indicators => Some(Kind::Indicators),
            Tool::Trading => Some(Kind::Trading),
            Tool::Journal => Some(Kind::Journal),
            _ => None,
        }
    }
    /// Nom de l'outil (menu de la barre d'icônes).
    pub fn name(self) -> &'static str {
        match self {
            Tool::Watchlist => "Watchlist",
            Tool::Indicators => "Indicateurs",
            Tool::Trading => "Trading démo",
            Tool::Journal => "Journal",
            Tool::Exchanges => "API",
            Tool::Marketplace => "Marketplace",
            Tool::Market => "Marché crypto",
            Tool::Ai => "Argos AI",
            Tool::Agenda => "Agenda",
            Tool::News => "News",
            Tool::Alertes => "Alertes",
            Tool::Support => "Support",
            Tool::Whales => "Baleines",
            Tool::Modules => "Modules",
            Tool::Dessin => "Outils de dessin",
            Tool::Gardien => "PropFirm",
            Tool::Script => "Argos Script",
            Tool::Academie => "Académie",
            Tool::Communaute => "Communauté",
            Tool::Portfolio => "Portefeuille",
            Tool::Liquidations => "Liquidations",
            Tool::Reglages => "Réglages",
            Tool::Demo => "Démo",
            Tool::Carnet => "Carnet d'ordres",
            Tool::Explorer => "Explorer Hyperliquid",
            Tool::Stats => "Stats Hyperliquid",
            Tool::Strategie => "Stratégie",
        }
    }

    /// Icône Phosphor au trait fin (le même dessin que dans Panneaux et le menu Argos).
    fn icon_name(self) -> &'static str {
        match self {
            Tool::Watchlist => "star",
            Tool::Indicators => "pulse",
            Tool::Trading => "chart-line-up",
            Tool::Journal => "notebook",
            Tool::Exchanges => "plugs",
            Tool::Marketplace => "shopping-cart",
            Tool::Market => "globe",
            Tool::Ai => crate::icons::ARGOS_A,
            Tool::Agenda => "calendar",
            Tool::News => "newspaper",
            Tool::Alertes => "bell",
            Tool::Support => "chat-circle",
            Tool::Whales => "fish",
            Tool::Modules => "puzzle-piece",
            Tool::Dessin => "pencil-simple",
            Tool::Gardien => "shield-check",
            Tool::Script => "sliders-horizontal",
            Tool::Academie => "graduation-cap",
            Tool::Communaute => "users-three",
            Tool::Portfolio => "briefcase",
            Tool::Liquidations => "fire",
            Tool::Reglages => "gear",
            Tool::Demo => "flask",
            Tool::Carnet => "book-open",
            Tool::Explorer => "compass",
            Tool::Stats => "chart-bar",
            Tool::Strategie => "cible",
        }
    }

    /// Icône centrée sur `(cx, cy)`, de couleur `c`.
    pub(super) fn icon(self, pm: &mut Pixmap, cx: f64, cy: f64, c: Rgb) {
        match self {
            Tool::Ai => crate::aibtn::ai_icon(pm, cx, cy, 24.0),
            // (🎯 Argos' own target: Phosphor's set here has none)
            Tool::Strategie => crate::icons::draw(pm, "cible", cx, cy, 22, c),
            t => {
                crate::phosphor::draw(pm, t.icon_name(), cx, cy, 22, c);
            }
        }
    }
}

/// Ce qui est gardé entre deux lancements.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Prefs {
    pub open: bool,
    /// Symboles marqués d'un drapeau.
    pub flags: Vec<(String, usize)>,
    /// Sections repliées.
    pub collapsed: Vec<String>,
    /// Tri : (colonne, croissant).
    pub sort: Option<(Col, bool)>,
    /// Liste affichée : `None` = Hyperliquid (automatique), sinon l'identifiant de la liste dans la base.
    pub list: Option<i64>,
    /// 🚩 Voir seulement une couleur de drapeau.
    pub flag_filter: Option<usize>,
    /// Largeur du panneau (pixels).
    pub width: u32,
    /// Panneau ouvert (quand `open`). (🔔 Alertes s'enregistre comme Watchlist : voir `Tool::Alertes`.)
    #[serde(serialize_with = "panel_enregistre")]
    pub panel: Tool,
    /// ↕️ Ordre des boutons de la barre d'icônes (vide : l'ordre par défaut, `Tool::ALL`).
    pub order: Vec<Tool>,
    /// 🙈 Boutons masqués de la barre d'icônes.
    pub hidden_tools: Vec<Tool>,
    /// ‹ › Barre d'icônes repliée (panneau fermé : rien à droite de la chart).
    pub folded: bool,
    /// Réglages déjà passés à la barre de la maquette (2026-11-09) ; absent d'un fichier : `false`.
    #[serde(default)]
    pub barre_v2: bool,
    /// Réglages déjà passés à la barre maquette-4 (🧪 Démo à la place du ✏ dessin, 2026-11-09).
    #[serde(default)]
    pub barre_v3: bool,
    /// 🌐 La vue « Tous les marchés » est affichée (à la place d'une liste), sa plateforme (id : « binance ») et son tri
    /// (clé de `argos_data::tous::Tri`).
    pub tous: bool,
    pub tous_bourse: String,
    pub tous_tri: String,
    /// 📰 Actualités: the sources unticked and the language (index of `actualites::LANGUES`).
    pub news_sources_off: Vec<String>,
    pub news_langue: usize,
    /// 📖 The order-book panel (volet and column): its tab, step, window and ⚙.
    pub carnet: crate::carnet::CarnetPrefs,
}

impl Prefs {
    /// La barre de la maquette (Jee, 2026-11-09) pour des réglages d'avant : une barre jamais réordonnée prend
    /// l'ordre et les boutons masqués par défaut ; un ordre choisi est gardé (les nouveaux boutons vont à la fin).
    /// Une seule fois ; `true` si ça change (à réécrire).
    pub fn passer_barre_v2(&mut self) -> bool {
        let mut change = false;
        if !self.barre_v2 {
            self.barre_v2 = true;
            if self.order.is_empty() {
                self.hidden_tools = Tool::CACHES.to_vec();
            }
            change = true;
        }
        // maquette-4 : le ✏ dessin quitte la barre (la barre d'outils flottante le remplace) si elle n'a jamais
        // été réordonnée
        if !self.barre_v3 {
            self.barre_v3 = true;
            if self.order.is_empty() && !self.hidden_tools.contains(&Tool::Dessin) {
                self.hidden_tools.push(Tool::Dessin);
            }
            change = true;
        }
        change
    }
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            open: false,
            flags: vec![],
            collapsed: vec![],
            sort: None,
            list: None,
            flag_filter: None,
            width: PANEL,
            panel: Tool::Watchlist,
            order: vec![],
            hidden_tools: Tool::CACHES.to_vec(),
            folded: false,
            barre_v2: true,
            barre_v3: true,
            tous: false,
            tous_bourse: String::new(),
            tous_tri: String::new(),
            news_sources_off: vec![],
            news_langue: 0,
            carnet: Default::default(),
        }
    }
}

/// Couleurs des drapeaux (comme TradingView) : rouge, bleu, vert, jaune, violet, cyan, rose.
pub const FLAG_COLORS: [Rgb; 7] = [
    Rgb(0xf2, 0x36, 0x45),
    Rgb(0x29, 0x62, 0xff),
    Rgb(0x6f, 0xc2, 0x8b),
    Rgb(0xfb, 0xc0, 0x2d),
    Rgb(0xab, 0x47, 0xbc),
    Rgb(0x00, 0xbc, 0xd4),
    Rgb(0xf4, 0x8f, 0xb1),
];
