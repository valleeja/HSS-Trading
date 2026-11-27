//! 📈 Indicateurs natifs de la chart : ceux qui sont actifs, leurs résultats et la clé du dernier calcul, leurs
//! réglages et textes, les favoris et la légende.

use web_time::Instant;

pub(crate) struct IndState {
    /// 🏷️ Légende des indicateurs : liste ouverte ? et indicateurs masqués (👁), gardés dans indicator_legend.json.
    pub(crate) leg: (bool, Vec<argos_render::indicators::Ind>),
    /// 📱 L'indicateur (ou le script) dont la barre d'actions est ouverte dans la légende du cell.
    pub(crate) sel: Option<Result<argos_render::indicators::Ind, usize>>,
    /// 📈 Indicateurs natifs actifs (gardés), leurs résultats et ce qui a servi au calcul.
    pub(crate) on: Vec<argos_render::indicators::Ind>,
    pub(crate) out: argos_render::indicators::IndOut,
    pub(crate) key: (usize, i64, u64, Vec<argos_render::indicators::Ind>),
    /// ☆ Indicateurs favoris (Marketplace et légende), gardés dans market_fav.json.
    pub(crate) favs: Vec<argos_render::indicators::Ind>,
    /// Inputs des indicateurs (fenêtre ⚙️ Réglages), gardés dans indicator_inputs.json.
    pub(crate) set: argos_render::indicators::Settings,
    /// ⚙ Chosen values of the user scripts' inputs (step 3): script id → input title → value (indicator_settings.json).
    pub(crate) scripts_set: std::collections::BTreeMap<String, std::collections::BTreeMap<String, argos_render::catalogue::ValeurScript>>,
    /// Textes des inputs texte et symbole (indicator_texts.json).
    pub(crate) texts: argos_render::indicators::Texts,
    /// 📋 Place and size of each table moved / resized by the user, by table key (tableaux-1; kept in
    /// indicator_settings.json, `tableaux`).
    pub(crate) tables: std::collections::BTreeMap<String, argos_render::tableaux::TableLayout>,
    /// Moment du dernier calcul (pas deux calculs de même forme en moins de 500 ms).
    pub(crate) at: Instant,
}
