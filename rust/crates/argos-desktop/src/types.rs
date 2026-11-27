//! Petits types partagés par toute l'app : cases du split, menus en liste, barre flottante, scripts, exchanges,
//! identifiants des lignes de menus, et deux petites aides (heure, touches).

use crate::*;

/// 🪟 Une case du split (hors chart principale) : son symbole (« hl:ETH », « file:… »), son unité de temps, ses bougies.
#[derive(Debug, Clone)]
pub(crate) struct Pane {
    pub(crate) coin: String,
    pub(crate) interval: Interval,
    pub(crate) bars: Vec<Bar>,
    /// Clé déjà demandée (pas deux chargements pareils).
    pub(crate) asked: String,
    /// 🖱️ Vue choisie dans cette case (zoom = espace entre bougies, décalage à droite) ; `None` = automatique (ou la
    /// plage de la principale si ⟷ Temps est synchronisé).
    pub(crate) vue: Option<(f64, f64)>,
    /// ✚ Réticule de cette case (souris dessus), en coordonnées de la case.
    pub(crate) croix: Option<(f64, f64)>,
    /// ⏪ During a replay: the bars asked around the replay's hour (those before this time); `None`: the latest.
    pub(crate) zone: Option<i64>,
    /// ⏳ A load of this pane is on its way.
    pub(crate) charge: bool,
}

impl Pane {
    /// Une case vide sur ce symbole et cette unité.
    pub(crate) fn new(coin: String, interval: Interval) -> Self {
        Pane {
            coin,
            interval,
            bars: vec![],
            asked: String::new(),
            vue: None,
            croix: None,
            zone: None,
            charge: false,
        }
    }

    pub(crate) fn key(&self) -> String {
        format!("{}|{}", self.coin, self.interval.secs())
    }

    /// The load being waited for: the key, and the replay's stretch (an answer for another one is too late).
    pub(crate) fn demande(&self) -> String {
        match self.zone {
            Some(z) => format!("{}|⏪{z}", self.key()),
            None => self.key(),
        }
    }
}

/// Menu en liste ouvert.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ListKind {
    /// 📟 ⋯ of the tablet's dock: the other buttons, with their names.
    DockAutres,
    /// ⌂ 📱 The « Accueil » row of Réglages by the finger: au lancement (oui / non), revoir l'accueil.
    Accueil,
    /// 🖱 Groupe du curseur (1er bouton de la barre de gauche) : Croix, Point, Flèche, Gomme (comme TradingView).
    Curseurs,
    Intervals,
    /// 📱 Le menu d'une section de la watchlist (son nom : `UiState.section`).
    Section,
    /// 📱 ⋯ de la barre d'actions d'un indicateur (lequel : `IndState.sel`).
    IndPlus,
    /// ⏱ Le menu des intervalles pour une case du split (seulement cette case change).
    PaneIntervals(usize),
    Styles,
    /// 🎨 Thème (Réglages).
    Theme,
    /// Sous-menu d'un groupe de la barre de gauche.
    RailGroup(&'static str),
    /// Menu d'un bouton du bas de la barre de gauche (aimant, cadenas, œil, poubelle).
    RailButton(rail::Btn),
    /// Menu des watchlists (nom de la liste dans l'en-tête).
    Lists,
    /// 🌐 « Tous les marchés » : menus des pilules Plateforme ▾ et Trier par ▾.
    TousBourse,
    TousTri,
    /// 🎯 📜 ▾ of the Stratégie volet: the strategies on the chart.
    Strategies,
    /// 📖 A pill of the order-book panel (exchange, step, coins traded, ⚙).
    Carnet(crate::actions::carnet::CarnetMenu),
    /// Fuseau horaire et sessions (en bas à droite).
    Timezone,
    /// Clic droit sur un dessin (son index).
    Drawing(usize),
    /// 🖍 A menu of the selected drawing's bar (the drawing, the button).
    DrawBar(usize, argos_render::drawbar::DbBtn),
    /// ＋ Ajouter une API : choisir l'exchange.
    AddApi,
    /// Clic droit sur la barre flottante ou la barre de gauche : l'outil (s'il y en a un) et la barre d'outils.
    ToolMenu(Option<&'static str>),
    /// Menus de la barre du haut : Argos ▾, Mises en page ▾, 🌐 Langue.
    Top(TopMenu),
    /// 🌐 « Télécharger cette langue ? » (index dans `i18n::CATALOG`).
    LangConfirm(usize),
    /// 🔔 Menu des alertes.
    Alerts,
    /// 💼 Comptes démo.
    Accounts,
    /// 🚀 Top gagnants / perdants 24 h (Communauté ▸ Top).
    Movers,
    /// 🧪 Backtest : stratégie, sens, stop, objectif.
    Backtest,
    /// 🧠 Mémoire de l'assistant.
    AiMemory,
    /// 👛 Wallet en lecture seule.
    Wallet,
    /// ↕️ Clic droit / appui long sur la barre d'icônes de droite (le bouton dessous, s'il y en a un).
    IconBar(Option<argos_render::watchlist::Tool>),
    /// 👥 Communauté.
    Community,
    /// 🧩 Modules (bac à sable WASM).
    Modules,
    /// ⌂ ⋯ d'une tuile de mise en page de l'Accueil (n° dans la liste), et sa confirmation « Supprimer ? ».
    TuileLayout(usize),
    SupprimerLayout(usize),
    /// 🔔 Right-click (or long press) on an alert line (its id): pause / resume, edit, delete, « Lignes d'alerte »,
    /// the alerts' colour, « Son… ».
    AlertLine(u64),
    /// 🔊 « Son… » of an alert line (its id): « Par défaut », the 8 sounds, « Son coupé ».
    SonAlerte(u64),
    /// 🔊 « Son des alertes » (Réglages): the default sound, ▶ Écouter, the volume.
    SonAlertes,
    /// 🔔 ⚙ of the alerts panel: every alert setting (sound, lines, colour, options, the notifications feed).
    ReglagesAlertes,
}

/// Ce que fait un bouton appuyé de la barre flottante pendant que la souris bouge (comme Argos v1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum PressMode {
    /// Pas encore décidé : au relâcher, c'est un clic.
    Wait,
    /// Clic maintenu ~0,4 s : la barre suit la souris (décalage souris → coin de la barre).
    Lift(f64, f64),
    /// Glisser un outil ⭐ : on change l'ordre des favoris.
    Sort,
    /// Bougé trop tôt : ni clic, ni déplacement.
    Cancel,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct BarPress {
    pub(crate) btn: Btn,
    pub(crate) at: (f64, f64),
    pub(crate) since: Instant,
    pub(crate) mode: PressMode,
}

/// Menu Fuseau horaire : les lignes des sessions commencent ici (pas des choix, juste l'info).
pub(crate) const ID_SESSION: i64 = 500;

/// Catalogue rafraîchi s'il a plus d'un jour.
pub(crate) const CATALOG_MAX_AGE_S: i64 = 86_400;

/// 📜 Un script Argos enregistré (scripts.json), comme v1 : nom, code, sur la chart ou non, masqué (👁), versions
/// sauvegardées (💾 : heure en secondes, code ; 20 au plus, la plus récente d'abord).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub(crate) struct SavedScript {
    /// ⚙ Stable identifier (step 3): its settings are saved under it in indicator_settings.json, whatever its name.
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) src: String,
    pub(crate) on: bool,
    pub(crate) hidden: bool,
    pub(crate) versions: Vec<(i64, String)>,
}

/// Exchanges activés (section Exchanges) : leur catalogue est téléchargé et rafraîchi chaque jour, ils ont leur filtre
/// dans le panneau ＋. Au départ : ceux d'Argos v1 (Hyperliquid, Binance, WEEX).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub(crate) struct ExPrefs {
    pub(crate) enabled: Vec<String>,
    /// ➕ APIs personnalisées : les noms choisis (la clé de chacune est dans le trousseau, sous « custom-<nom> »).
    pub(crate) custom: Vec<String>,
    /// 📈 Lien de l'API CoinMarketCap (vide = https · pro-api.coinmarketcap.com). La clé, elle, est au trousseau.
    pub(crate) cmc_url: String,
    /// 📈 Relevé CoinMarketCap toutes les … heures (24 par défaut : une fois par jour).
    pub(crate) cmc_every_h: u32,
}

impl Default for ExPrefs {
    fn default() -> Self {
        Self {
            enabled: ["hl", "binance", "weex"].map(String::from).to_vec(),
            custom: vec![],
            cmc_url: String::new(),
            cmc_every_h: 24,
        }
    }
}

/// Identifiant d'une API personnalisée dans le trousseau et le panneau : « custom-<nom en minuscules> ».
pub(crate) fn custom_id(name: &str) -> String {
    format!("custom-{}", argos_core::account::handle_norm(name))
}

/// Pastille de couleur d'un exchange (filtres du panneau ＋, comme v1).
pub(crate) fn ex_dot(e: Exchange) -> argos_render::text::Rgb {
    let (r, g, b) = match e {
        Exchange::Hyperliquid => (0x50, 0xd2, 0xa0),
        Exchange::Binance => (0xf0, 0xb9, 0x0b),
        Exchange::Weex => (0xe6, 0xe8, 0xee),
        Exchange::Bybit => (0xf7, 0xa6, 0x00),
        Exchange::Okx => (0x8a, 0x8e, 0x9a),
        Exchange::Blofin => (0xff, 0x8a, 0x3d),
        Exchange::Bitunix => (0x3d, 0xc8, 0xf0),
        Exchange::Bitstamp => (0x4c, 0xaf, 0x50),
        Exchange::Bitget => (0x00, 0xf0, 0xff),
        Exchange::Gate => (0x22, 0x55, 0xff),
        Exchange::Mexc => (0x18, 0x8e, 0xff),
        Exchange::Kucoin => (0x23, 0xaf, 0x91),
        Exchange::Coinbase => (0x00, 0x52, 0xff),
        Exchange::Kraken => (0x7a, 0x5c, 0xff),
        Exchange::Aster => (0xe8, 0xa8, 0x7c),
        Exchange::Bitfinex => (0x16, 0xb1, 0x57),
        // le spot a la couleur de son exchange
        e => return ex_dot(e.parent()),
    };
    argos_render::text::Rgb(r, g, b)
}

/// Menu des listes : identifiants des lignes (0 = Hyperliquid automatique, 1.. = listes de la base dans leur ordre).
pub(crate) const ID_FLAG_ALL: i64 = 100;
pub(crate) const ID_NEW: i64 = 200;
pub(crate) const ID_SECTION: i64 = 201;
pub(crate) const ID_RENAME: i64 = 202;
pub(crate) const ID_DELETE: i64 = 203;
/// 📱 « 🔌 Sources et clés API » (menu ⋯ de la watchlist du cell).
pub(crate) const ID_API: i64 = 204;
/// 🌐 « Tous les marchés » dans le menu des listes (et en pilule sur cell) ; ses deux menus (pilules) : les
/// plateformes à partir de `ID_TOUS_BOURSE`, les tris à partir de `ID_TOUS_TRI`.
pub(crate) const ID_TOUS: i64 = 205;
pub(crate) const ID_TOUS_BOURSE: i64 = 300;
pub(crate) const ID_TOUS_TRI: i64 = 400;

/// 💾 Dossier du cache des bougies ; dans les tests : un dossier de test (`rust/target/tmp`), jamais le vrai `~/.cache`.
pub(crate) fn cache_dir() -> Option<std::path::PathBuf> {
    if cfg!(test) {
        return Some(argos_core::dossiers::de_test("cache"));
    }
    argos_data::cache::dir()
}

#[cfg(test)]
thread_local! {
    /// 🧪 Moment figé (images de référence) : `now_s` le renvoie dans ce fil de test.
    pub(crate) static NOW_FIGE: std::cell::Cell<Option<i64>> = const { std::cell::Cell::new(None) };
}

/// Moment présent, en secondes UTC.
pub(crate) fn now_s() -> i64 {
    #[cfg(test)]
    if let Some(t) = NOW_FIGE.with(|n| n.get()) {
        return t;
    }
    web_time::SystemTime::now()
        .duration_since(web_time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}

/// Touche de la fenêtre → touche de la table d'actions.
pub(crate) fn to_key(k: &Key) -> Option<chart_actions::Key> {
    Some(match k {
        Key::Named(NamedKey::ArrowLeft) => chart_actions::Key::Left,
        Key::Named(NamedKey::ArrowRight) => chart_actions::Key::Right,
        Key::Named(NamedKey::ArrowUp) => chart_actions::Key::Up,
        Key::Named(NamedKey::ArrowDown) => chart_actions::Key::Down,
        Key::Named(NamedKey::Home) => chart_actions::Key::Home,
        Key::Named(NamedKey::End) => chart_actions::Key::End,
        Key::Named(NamedKey::F5) => chart_actions::Key::F5,
        Key::Named(NamedKey::Escape) => chart_actions::Key::Escape,
        Key::Character(c) => chart_actions::Key::Char(c.chars().next()?.to_lowercase().next()?),
        _ => return None,
    })
}

/// Objet attrapé à la souris.
#[derive(Debug, Clone)]
pub(crate) enum Grab {
    /// Tout l'objet : où on l'a attrapé et ses points à ce moment.
    Move { i: usize, x0: f64, y0: f64, from: Vec<Point> },
    /// Une poignée (un point de l'objet).
    Handle { i: usize, h: usize },
}

/// 🖱 Curseur choisi au 1er bouton de la barre de gauche (comme TradingView : Cross par défaut, Dot, Arrow).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub(crate) enum CurseurChoisi {
    #[default]
    Croix,
    Point,
    Fleche,
    /// Démonstration (TradingView) : Alt tenu + clic ou glisser = des points et des traits qui s'effacent en 3 s.
    Demonstration,
}

impl CurseurChoisi {
    pub(crate) const TOUS: [CurseurChoisi; 4] = [CurseurChoisi::Croix, CurseurChoisi::Point, CurseurChoisi::Fleche, CurseurChoisi::Demonstration];

    pub(crate) fn nom(self) -> &'static str {
        match self {
            CurseurChoisi::Croix => "Croix",
            CurseurChoisi::Point => "Point",
            CurseurChoisi::Fleche => "Flèche",
            CurseurChoisi::Demonstration => "Démonstration",
        }
    }

    /// Le curseur à poser au repos sur la chart.
    pub(crate) fn curseur(self) -> Curseur {
        match self {
            CurseurChoisi::Croix => Curseur::Icone(CursorIcon::Crosshair),
            CurseurChoisi::Point => Curseur::Point,
            CurseurChoisi::Fleche => Curseur::Icone(CursorIcon::Default),
            CurseurChoisi::Demonstration => Curseur::Icone(CursorIcon::Crosshair),
        }
    }
}

/// 🖍 Un trait du curseur Démonstration : ses points (coordonnées de la chart) et son départ (il s'efface en 3 s
/// après le relâcher ; `fini` : relâché à ce moment).
#[derive(Debug, Clone)]
pub(crate) struct TraitDemo {
    pub(crate) pts: Vec<(f64, f64)>,
    pub(crate) fini: Option<Instant>,
}

/// 🖍 Durée du fondu d'un trait de démonstration (comme TradingView : « quelques secondes »).
pub(crate) const DEMO_FONDU: Duration = Duration::from_secs(3);

/// 🖱 Ce que la souris montre : une icône du système, ou le petit point (curseur dessiné, natif et navigateur).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Curseur {
    Icone(CursorIcon),
    Point,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn touches_de_la_table_d_actions() {
        assert_eq!(to_key(&Key::Named(NamedKey::ArrowLeft)), Some(chart_actions::Key::Left));
        assert_eq!(to_key(&Key::Named(NamedKey::Escape)), Some(chart_actions::Key::Escape));
        // une lettre : en minuscule, Maj ou pas
        assert_eq!(to_key(&Key::Character("T".into())), Some(chart_actions::Key::Char('t')));
        // touche sans action
        assert_eq!(to_key(&Key::Named(NamedKey::Tab)), None);
    }

    #[test]
    fn cle_d_une_case_du_split() {
        let p = Pane::new("ETH".into(), Interval::parse("1h").unwrap());
        assert_eq!(p.key(), "ETH|3600");
    }
}
