//! 📚 Catalogue unique des indicateurs (étape 1 du moteur Argos Script) : une fiche par indicateur intégré — son
//! identifiant stable, son nom, sa famille, son origine, sa confiance, sa section, ses réglages — et les listes qui en
//! dépendent (catégories de la page ƒx, fiches du Marketplace). Tous les écrans lisent ce catalogue ; il n'y a plus de
//! liste d'indicateurs en double ailleurs. Généré à partir des tables d'avant (mêmes noms, même ordre, mêmes réglages).
//!
//! Les fiches sont dans l'ordre de `Ind::ALL` (argos-render) : `FICHES[k]` est l'indicateur `Ind::ALL[k]` (vérifié par
//! un test). Trois indicateurs ont leurs réglages hors de ce crate (Lens, carte du carnet, carte des liquidations) :
//! leur crate les inscrit au démarrage (`inscrire_complement`) ; un oubli fait échouer un test.

use crate::inputs::Param;
use std::sync::OnceLock;

/// D'où vient un indicateur (plan Argos Script : 4 familles).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Famille {
    /// La gamme Argos (TBO, Cipher, TVD, AI Trend, Vector Zone, Lens…).
    Gamme,
    /// Les ordinaires (RSI, EMA, MACD…).
    Classique,
    /// Créé par l'utilisateur (Argos Script).
    Utilisateur,
    /// Venu du Marketplace (module signé du kit Argos).
    Marketplace,
    /// 📜 A TradingView community script run as it is, under its own licence (LuxAlgo's Smart Money Concepts).
    Communaute,
}

/// Où vit son code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origine {
    /// Livré dans Argos.
    Integre,
    /// Un script sur cet appareil.
    Local,
    /// Un module WASM.
    Module,
}

/// Quelle confiance lui accorder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Confiance {
    Integre,
    Local,
    ModuleVerifie,
}

/// Ses réglages : une table de ce crate, ou inscrite au démarrage par le crate qui la tient.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Reglages {
    Table(&'static [&'static str], &'static [Param]),
    Complement,
    /// 📜 A built-in indicator written in Argos Script (its source): its settings are its `input()` calls, as for a
    /// user script (groups, inline, tooltips), and its calculation is the script itself.
    Script(&'static str),
}

/// La fiche d'un indicateur.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fiche {
    /// Identifiant stable (le nom de sa variante `Ind`).
    pub id: &'static str,
    pub nom: &'static str,
    pub famille: Famille,
    pub origine: Origine,
    pub confiance: Confiance,
    /// Section du panneau Indicateurs (« CLASSIQUES », « ARGOS »).
    pub section: &'static str,
    pub reglages: Reglages,
}

const fn integre(id: &'static str, nom: &'static str, famille: Famille, section: &'static str, reglages: Reglages) -> Fiche {
    Fiche {
        id,
        nom,
        famille,
        origine: Origine::Integre,
        confiance: Confiance::Integre,
        section,
        reglages,
    }
}

use Famille::{Classique, Communaute, Gamme};
use Reglages::{Complement, Script, Table};

/// Les indicateurs intégrés, dans l'ordre de `Ind::ALL`.
pub const FICHES: [Fiche; 56] = [
    integre(
        "Rsi",
        "RSI 14",
        Classique,
        "CLASSIQUES",
        Table(&crate::classic_inputs::LEN_GROUPS, &crate::classic_inputs::RSI_PARAMS),
    ),
    integre(
        "Macd",
        "MACD",
        Classique,
        "CLASSIQUES",
        Table(&crate::classic_inputs::LEN_GROUPS, &crate::classic_inputs::MACD_PARAMS),
    ),
    integre(
        "Bollinger",
        "Bandes de Bollinger",
        Classique,
        "CLASSIQUES",
        // 📜 TradingView's built-in, run by Argos Script (owner's decision « B: replace », 19 Nov 2026)
        Script(crate::integres::BOLLINGER),
    ),
    integre(
        "EmaCross",
        "Croisement EMA 9/21",
        Classique,
        "CLASSIQUES",
        Table(&crate::classic_inputs::LEN_GROUPS, &crate::classic_inputs::EMA_CROSS_PARAMS),
    ),
    integre(
        "Stochastic",
        "Stochastique",
        Classique,
        "CLASSIQUES",
        Table(&crate::classic_inputs::LEN_GROUPS, &crate::classic_inputs::STOCH_PARAMS),
    ),
    integre(
        "Atr",
        "ATR 14",
        Classique,
        "CLASSIQUES",
        Table(&crate::classic_inputs::LEN_GROUPS, &crate::classic_inputs::ATR_PARAMS),
    ),
    integre(
        "Vwap",
        "VWAP",
        Classique,
        "CLASSIQUES",
        Table(&crate::classic_inputs::LEN_GROUPS, &crate::classic_inputs::VWAP_PARAMS),
    ),
    integre(
        "RelVolume",
        "Volume relatif",
        Classique,
        "CLASSIQUES",
        Table(&crate::classic_inputs::LEN_GROUPS, &crate::classic_inputs::REL_VOLUME_PARAMS),
    ),
    integre(
        "Tvd",
        "Argos TVD Pro",
        Gamme,
        "ARGOS",
        Table(&crate::indicators::tvd::GROUPS, &crate::indicators::tvd::PARAMS),
    ),
    integre(
        "EmaMulti",
        "EMA multiples",
        Gamme,
        "ARGOS",
        Table(&crate::v1_inputs::EMA_MULTI_GROUPS, &crate::v1_inputs::EMA_MULTI_PARAMS),
    ),
    integre(
        "Cipher",
        "Argos Cipher Pro",
        Gamme,
        "ARGOS",
        Table(&crate::indicators::cipher::GROUPS, &crate::indicators::cipher::PARAMS),
    ),
    integre(
        "AiTrend",
        "Argos AI Trend V Pro",
        Gamme,
        "ARGOS",
        Table(&crate::indicators::ai_trend::GROUPS, &crate::indicators::ai_trend::PARAMS),
    ),
    integre(
        "Tbo",
        "Argos TBO Pro Div",
        Gamme,
        "ARGOS",
        Table(&crate::indicators::tbo::GROUPS, &crate::indicators::tbo::PARAMS),
    ),
    integre(
        "Smc",
        "Argos OB · BB · FVG · iFVG",
        Gamme,
        "ARGOS",
        Table(&crate::indicators::smc::GROUPS, &crate::indicators::smc::PARAMS),
    ),
    integre(
        "Heatmap",
        "Argos Heatmap Volume",
        Gamme,
        "ARGOS",
        Table(&crate::v1_inputs::HEATMAP_GROUPS, &crate::v1_inputs::HEATMAP_PARAMS),
    ),
    integre(
        "Profile",
        "Argos Volume Profile",
        Gamme,
        "ARGOS",
        Table(&crate::v1_inputs::PROFILE_GROUPS, &crate::v1_inputs::PROFILE_PARAMS),
    ),
    integre(
        "Bands",
        "Argos Bollinger Bands",
        Gamme,
        "ARGOS",
        Table(&crate::v1_inputs::BANDS_GROUPS, &crate::v1_inputs::BANDS_PARAMS),
    ),
    integre(
        "BandsOsc",
        "Argos Bollinger Oscillator",
        Gamme,
        "ARGOS",
        Table(&crate::v1_inputs::BANDS_OSC_GROUPS, &crate::v1_inputs::BANDS_OSC_PARAMS),
    ),
    integre(
        "SmcPro",
        "Argos SMC",
        Gamme,
        "ARGOS",
        Table(&crate::indicators::smc_pro::GROUPS, &crate::indicators::smc_pro::PARAMS),
    ),
    integre(
        "SwingVwap",
        "Argos Pivot VWAP",
        Gamme,
        "ARGOS",
        Table(&crate::classic_inputs::LEN_GROUPS, &crate::classic_inputs::PIVOT_VWAP_PARAMS),
    ),
    integre(
        "SmartSwingVwap",
        "Argos Pivot VWAP Multi",
        Gamme,
        "ARGOS",
        Table(&crate::classic_inputs::LEN_GROUPS, &crate::classic_inputs::PIVOT_VWAP_MULTI_PARAMS),
    ),
    integre(
        "Footprint",
        "Argos Footprint",
        Gamme,
        "ARGOS",
        Table(&crate::indicators::flow::FOOTPRINT_GROUPS, &crate::indicators::flow::FOOTPRINT_PARAMS),
    ),
    integre(
        "Bubbles",
        "Argos Bubbles · Trades & Liquidité",
        Gamme,
        "ARGOS",
        Table(&crate::indicators::flow::BUBBLES_GROUPS, &crate::indicators::flow::BUBBLES_PARAMS),
    ),
    integre(
        "Tape",
        "Argos Tape",
        Gamme,
        "ARGOS",
        Table(&crate::indicators::flow::TAPE_GROUPS, &crate::indicators::flow::TAPE_PARAMS),
    ),
    integre(
        "PriceAction",
        "Argos Price Action",
        Gamme,
        "ARGOS",
        Table(&crate::indicators::order_blocks::GROUPS, &crate::indicators::order_blocks::PARAMS),
    ),
    integre(
        "AnchoredVwap",
        "Argos Anchored VWAP",
        Gamme,
        "ARGOS",
        Table(&crate::indicators::anchored_vwap::GROUPS, &crate::indicators::anchored_vwap::PARAMS),
    ),
    integre(
        "Patterns",
        "Argos Patterns auto",
        Gamme,
        "ARGOS",
        Table(&crate::indicators::patterns::GROUPS, &crate::indicators::patterns::PARAMS),
    ),
    integre("BookHeat", "Argos Heatmap du carnet", Gamme, "ARGOS", Complement),
    integre("LiqMap", "Argos Carte des liquidations", Gamme, "ARGOS", Complement),
    integre(
        "VectorZone",
        "Argos Vector Zone Pro",
        Gamme,
        "ARGOS",
        Table(&crate::indicators::vector_zone::GROUPS, &crate::indicators::vector_zone::PARAMS),
    ),
    integre(
        "MtfTable",
        "Argos Tableau multi-UT",
        Gamme,
        "ARGOS",
        Table(&crate::indicators::mtf_table::GROUPS, &crate::indicators::mtf_table::PARAMS),
    ),
    integre(
        "StochRsi",
        "RSI stochastique",
        Classique,
        "CLASSIQUES",
        Table(&crate::classic_inputs::LEN_GROUPS, &crate::classic_inputs::STOCH_RSI_PARAMS),
    ),
    integre(
        "Ichimoku",
        "Ichimoku",
        Classique,
        "CLASSIQUES",
        Table(&crate::classic_inputs::LEN_GROUPS, &crate::classic_inputs::ICHIMOKU_PARAMS),
    ),
    integre(
        "Keltner",
        "Canal de Keltner",
        Classique,
        "CLASSIQUES",
        Table(&crate::classic_inputs::LEN_GROUPS, &crate::classic_inputs::KELTNER_PARAMS),
    ),
    integre(
        "Donchian",
        "Canal de Donchian",
        Classique,
        "CLASSIQUES",
        Table(&crate::classic_inputs::LEN_GROUPS, &crate::classic_inputs::DONCHIAN_PARAMS),
    ),
    integre(
        "Psar",
        "SAR parabolique",
        Classique,
        "CLASSIQUES",
        Table(&crate::classic_inputs::LEN_GROUPS, &crate::classic_inputs::PSAR_PARAMS),
    ),
    integre(
        "Dmi",
        "DMI / ADX 14",
        Classique,
        "CLASSIQUES",
        Table(&crate::classic_inputs::LEN_GROUPS, &crate::classic_inputs::DMI_PARAMS),
    ),
    integre(
        "Cci",
        "CCI 20",
        Classique,
        "CLASSIQUES",
        Table(&crate::classic_inputs::LEN_GROUPS, &crate::classic_inputs::CCI_PARAMS),
    ),
    integre(
        "WilliamsR",
        "Williams %R",
        Classique,
        "CLASSIQUES",
        Table(&crate::classic_inputs::LEN_GROUPS, &crate::classic_inputs::WILLIAMS_PARAMS),
    ),
    integre(
        "Obv",
        "OBV",
        Classique,
        "CLASSIQUES",
        Table(&crate::classic_inputs::LEN_GROUPS, &crate::classic_inputs::OBV_PARAMS),
    ),
    integre(
        "Supertrend",
        "Supertrend",
        Classique,
        "CLASSIQUES",
        Table(&crate::classic_inputs::LEN_GROUPS, &crate::classic_inputs::SUPERTREND_PARAMS),
    ),
    integre(
        "Aroon",
        "Aroon 25",
        Classique,
        "CLASSIQUES",
        Table(&crate::classic_inputs::LEN_GROUPS, &crate::classic_inputs::AROON_PARAMS),
    ),
    integre(
        "Mfi",
        "MFI 14",
        Classique,
        "CLASSIQUES",
        Table(&crate::classic_inputs::LEN_GROUPS, &crate::classic_inputs::MFI_PARAMS),
    ),
    integre(
        "Cvd",
        "CVD",
        Classique,
        "CLASSIQUES",
        Table(&crate::classic_inputs::LEN_GROUPS, &crate::classic_inputs::OBV_PARAMS),
    ),
    integre("Lens", "Argos Lens", Gamme, "ARGOS", Complement),
    // 📜 built-in indicators written in Argos Script (from here on: never written to the old by-position file)
    integre("SpringTension", "Spring Tension", Gamme, "ARGOS", Script(crate::integres::SPRING_TENSION)),
    // 📜 TradingView's built-in moving averages, their exact code (lot « moyennes », Jee 18 Nov)
    integre("Sma", "SMA", Classique, "CLASSIQUES", Script(crate::integres::SMA)),
    integre("Wma", "WMA", Classique, "CLASSIQUES", Script(crate::integres::WMA)),
    integre("Hma", "HMA", Classique, "CLASSIQUES", Script(crate::integres::HMA)),
    integre("Vwma", "VWMA", Classique, "CLASSIQUES", Script(crate::integres::VWMA)),
    integre("Smma", "Smoothed MA", Classique, "CLASSIQUES", Script(crate::integres::SMMA)),
    integre("MaRibbon", "MA ribbon", Classique, "CLASSIQUES", Script(crate::integres::MA_RIBBON)),
    // 📜 Argos Bollinger + RSI (Jee, 19 Nov): our own script, the idea of ChartArt's double strategy
    integre(
        "BollingerRsi",
        "Argos Bollinger + RSI",
        Classique,
        "CLASSIQUES",
        Script(crate::integres::BOLLINGER_RSI),
    ),
    integre("ArgosMurs", "Argos Murs", Gamme, "ARGOS", Complement),
    // 📜 Argos Vague (Jee, 20 Nov): our own oscillator, the ideas of BigBeluga's « Nautilus PRO+ » (none of its code)
    integre("ArgosVague", "Argos Vague", Gamme, "ARGOS", Script(crate::integres::ARGOS_VAGUE)),
    // 📜 LuxAlgo's Smart Money Concepts (Jee, 25 Nov: first of the SMC list, drawn as TradingView draws it)
    integre(
        "LuxAlgoSmc",
        "Smart Money Concepts [LuxAlgo]",
        Communaute,
        "ARGOS",
        Script(crate::integres::LUXALGO_SMC),
    ),
];

/// How many built-in indicators existed before the ones written in Argos Script (the old /app/ knows only these: the
/// by-position settings file never gets the others).
pub const AVANT_SCRIPTS: usize = 45;

/// 📜 A native built-in replaced by its Argos Script source (same catalogue id, same place in `FICHES`): how the
/// settings saved for the native one become the script's. `cles[k]` = (old key, new key) of the native table's
/// position `k`: the by-position file (`indicator_inputs.json`) has the value of `cles[k]` at position `k`, the
/// by-name file has it under the old key. A new key is an input title or a style key (`style_integres`); the script's
/// settings missing here keep their defaults.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Conversion {
    pub id: &'static str,
    pub cles: &'static [(&'static str, &'static str)],
}

/// The natives replaced by their script so far (owner's decision « B: replace », 19 Nov 2026; one row per converted
/// indicator). Bollinger: « Basis MA Type » SMA, « Source » close and « Offset » 0 are new (their defaults: the
/// native's own calculation).
pub const CONVERSIONS: [Conversion; 1] = [Conversion {
    id: "Bollinger",
    cles: &[
        ("len", "Length"),
        ("mult", "StdDev"),
        ("col", "Basis · couleur"),
        ("ep1", "Upper · épaisseur"),
        ("ty1", "Upper · type de ligne"),
        ("ep2", "Basis · épaisseur"),
        ("ty2", "Basis · type de ligne"),
        ("ep3", "Lower · épaisseur"),
        ("ty3", "Lower · type de ligne"),
    ],
}];

/// The conversion of the indicator `id`, if it was a native replaced by its script.
pub fn conversion(id: &str) -> Option<&'static Conversion> {
    CONVERSIONS.iter().find(|c| c.id == id)
}

impl Conversion {
    /// The values of the native's by-position list, by new key (values beyond its table are dropped: the native had
    /// no more settings).
    pub fn depuis_positions(&self, vals: &[f64]) -> std::collections::BTreeMap<String, f64> {
        self.cles.iter().zip(vals).map(|((_, n), v)| (n.to_string(), *v)).collect()
    }

    /// The by-name settings with the native's old keys renamed to the new ones (a new key already there wins, the old
    /// one is dropped). `None`: no old key, nothing to convert (so converting twice changes nothing).
    pub fn depuis_noms(&self, noms: &std::collections::BTreeMap<String, f64>) -> Option<std::collections::BTreeMap<String, f64>> {
        if !self.cles.iter().any(|(a, _)| noms.contains_key(*a)) {
            return None;
        }
        let mut out = noms.clone();
        for (a, n) in self.cles {
            if let Some(v) = out.remove(*a) {
                out.entry(n.to_string()).or_insert(v);
            }
        }
        Some(out)
    }

    /// The native's by-position list from the script's values (`params` / `vals`: the script's table and values), for
    /// the old /app/ that still reads `indicator_inputs.json`.
    pub fn vers_positions(&self, params: &[Param], vals: &[f64]) -> Vec<f64> {
        let noms = par_nom(params, vals);
        self.cles
            .iter()
            .map(|(_, n)| {
                noms.get(*n)
                    .copied()
                    .or_else(|| params.iter().find(|p| p.key == *n).map(|p| p.def))
                    .unwrap_or(f64::NAN)
            })
            .collect()
    }
}

/// Catégories de la page ƒx (maquette) : (icône, nom, identifiants des indicateurs, dans l'ordre affiché).
pub const CATEGORIES_FX: [(&str, &str, &[&str]); 8] = [
    (
        "◔",
        "Classiques",
        &[
            "Rsi",
            "Macd",
            "Bollinger",
            "BollingerRsi",
            "EmaCross",
            "Stochastic",
            "Atr",
            "Vwap",
            "RelVolume",
            "EmaMulti",
            "Sma",
            "Wma",
            "Hma",
            "Vwma",
            "Smma",
            "MaRibbon",
        ],
    ),
    (
        "↗",
        "Tendance",
        &["Lens", "Ichimoku", "Supertrend", "Psar", "Dmi", "Aroon", "Donchian", "AiTrend"],
    ),
    (
        "∿",
        "Oscillateurs",
        &[
            "StochRsi",
            "Cci",
            "WilliamsR",
            "Mfi",
            "Cipher",
            "Tbo",
            "BandsOsc",
            "Tvd",
            "SpringTension",
            "ArgosVague",
        ],
    ),
    (
        "▮",
        "Volume",
        &["Obv", "Cvd", "Heatmap", "Profile", "SwingVwap", "SmartSwingVwap", "AnchoredVwap"],
    ),
    ("≋", "Volatilité", &["Bands", "Keltner"]),
    (
        "▭",
        "SMC / Price action",
        &["LuxAlgoSmc", "SmcPro", "Smc", "PriceAction", "Patterns", "VectorZone"],
    ),
    ("⇅", "Order flow / carnet", &["ArgosMurs", "Footprint", "Bubbles", "Tape", "BookHeat", "LiqMap"]),
    ("⊞", "Multi-UT", &["MtfTable"]),
];

/// Words that also find an indicator in the pickers' search (besides its name and its category's name), e.g. « order
/// book », « walls », « wave ». (An id absent from this branch is simply never matched.)
pub const MOTS_CLES: [(&str, &str); 5] = [
    ("ArgosMurs", "murs mur walls wall carnet order book orderbook blocs ordres profondeur depth"),
    ("BookHeat", "heatmap carnet order book orderbook profondeur depth"),
    ("LiqMap", "liquidations carte heatmap"),
    ("ArgosVague", "vague wave vagues waves"),
    ("LuxAlgoSmc", "luxalgo smart money concepts bos choch order blocks structure"),
];

/// The text an indicator is searched in: its name, its category's name and its search words.
pub fn texte_recherche(id: &str, nom: &str) -> String {
    let cat = CATEGORIES_FX.iter().find(|c| c.2.contains(&id)).map_or("", |c| c.1);
    let mots = MOTS_CLES.iter().find(|m| m.0 == id).map_or("", |m| m.1);
    format!("{nom} {cat} {mots}")
}

/// Place of an indicator in the ƒx categories: (category rank, rank inside it); `None`: in no category.
pub fn rang_fx(id: &str) -> Option<(usize, usize)> {
    CATEGORIES_FX
        .iter()
        .enumerate()
        .find_map(|(k, c)| c.2.iter().position(|x| *x == id).map(|i| (k, i)))
}

/// Catégories du Marketplace (comme v1) : (clé, nom).
pub const MARCHE_CATEGORIES: [(&str, &str); 10] = [
    ("all", "Toutes"),
    ("off", "⭐ Officiel Argos"),
    ("trend", "📈 Tendance"),
    ("osc", "🌊 Oscillateurs"),
    ("brk", "📐 Cassures"),
    ("vol", "📊 Volume & flux"),
    ("smc", "🧠 SMC / ICT"),
    ("vlt", "📏 Volatilité"),
    ("gold", "🥇 Or / Scalping"),
    ("com", "👥 Communauté"),
];

/// Indicateurs officiels du Marketplace (ordre de v1 ; l'index est gardé dans les favoris) : (identifiant,
/// catégorie, « or / scalping », description).
pub const MARCHE: [(&str, &str, bool, &str); 19] = [
    ("Tvd", "trend", true, "AI Index (RSI + Stoch + momentum) · 4 EMA + nuage · bougies vector · divergences avec lignes · signaux Trend / AI / GOLD / retournements · tableau de bord"),
    ("EmaMulti", "trend", false, "Jusqu'à 6 EMA sur le prix (ex. 13, 50, 100, 200)"),
    ("Cipher", "osc", true, "WaveTrend + divergences WT/RSI/MFI/Stoch + Gold/Purple dots + confluence n/4"),
    ("AiTrend", "osc", true, "AI Trend Index (RSI + Stoch + Momentum) · dégradé · signal · divergences + 🤑 · divergences MTF · zones extrêmes · boîte HTF · étiquettes sur la chart"),
    ("Tbo", "brk", true, "Trend Break Oscillator d'Argos : force des cassures de trendlines · filtres ATR / ADX / HTF / volume · Vector 150+/200+ · divergences régulières et cachées"),
    ("Smc", "smc", false, "OB (order blocks) · BB (breaker blocks : OB cassé qui change de camp) · FVG (fair value gaps) · iFVG (FVG traversé = polarité inversée, invalidé après 2 touches) · structure BOS / CHoCH · liquidité EQH / EQL · option UT supérieure (4H)"),
    ("Heatmap", "vol", false, "Chaleur du volume d'Argos : écart de chaque volume à sa moyenne, en écarts-types · bougies colorées · seuils très fort / fort / moyen / normal / faible"),
    ("Profile", "vol", false, "Profil de volume d'Argos sur les N dernières bougies · POC · zone de valeur (68 % par défaut) · hausse / baisse séparées"),
    ("Footprint", "vol", false, "Achats × ventes par rangée de prix dans chaque bougie · delta · POC · déséquilibres diagonaux (estimé à partir des bougies ; pour le vrai flux intrabar, utilise « Footprint intrabar »)"),
    ("Bubbles", "vol", false, "Bulles style Bookmap : achats / ventes agressifs (déséquilibre ≥ ratio) à chaque prix, taille ou transparence selon le volume · point jaune = POC, la rangée la plus chargée de la bougie (port de Footprint Imbalance Bubbles, vrais trades seulement)"),
    ("Tape", "vol", false, "Ruban des mises à jour en temps réel (heure · prix · volume) et delta de volume de la bougie · marqueurs de poussées de delta (port de Tape LucF ; se remplit en direct seulement)"),
    ("PriceAction", "smc", false, "Order blocks d'Argos : dernière bougie opposée avant une impulsion qui laisse une FVG et casse le swing (portée courte / moyenne / longue), zones cassées (breakers), ligne médiane, changements de polarité ▲▼"),
    ("AnchoredVwap", "vol", false, "VWAP qui démarre à la bougie choisie sur la chart (ligne à glisser) · jusqu'à 3 paires de bandes d'écart-type · source au choix, même la sortie d'un autre indicateur"),
    ("Bands", "vlt", false, "Bandes de Bollinger (moyenne ± k écarts-types) · nuage · squeeze (bandes serrées) · touches des bandes"),
    ("BandsOsc", "osc", false, "Force des cassures d'Argos : de combien la clôture sort de sa bande, au-dessus (haussier) ou en dessous (baissier), lissée de 0 à 100 · ligne 50"),
    ("VectorZone", "brk", true, "4 EMA · bougies vector PVSRA (climax / vector) · zones d'offre et de demande notées sur 100 · trendlines auto + CONF · liquidité BSL / SSL, EQH / EQL et sweeps · tableau vector 15m / 1H / 4H / D / W"),
    ("MtfTable", "trend", false, "Tableau multi-UT en bas à droite : tendance (EMA 20 / 50 et prix) · RSI · variation · temps avant clôture en 15m / 1H / 4H / D / W, regroupés depuis la chart · total des UT alignées"),
    ("SmcPro", "smc", true, "Structure HH/HL/LH/LL · BOS / CHoCH · OB · Breakers · FVG / iFVG · liquidité EQH/EQL + sweeps · sessions et killzones ICT · Premium / Discount · biais multi-UT · plan entrée / SL / TP et score de confiance /10"),
    ("Lens", "trend", true, "Lecture du marché pour Argos AI : régime (tendance ↑ / ↓, range, haute volatilité) · scores long et short sur 100 (tendance, momentum, force, volume, structure) · setup sur bougie clôturée (entrée, stop 1,5 ATR, cibles 1,5R et 3R) · affichage seul, aucun ordre"),
];

/// Fonction qui donne les réglages tenus hors de ce crate (inscrite une fois au démarrage).
pub type ComplementFn = fn(&str) -> Option<(&'static [&'static str], &'static [Param])>;

static COMPLEMENT: OnceLock<ComplementFn> = OnceLock::new();

/// Inscrit la fonction des réglages tenus ailleurs (Lens, carte du carnet, carte des liquidations).
pub fn inscrire_complement(f: ComplementFn) {
    let _ = COMPLEMENT.set(f);
}

/// La fiche d'un identifiant.
pub fn fiche(id: &str) -> Option<&'static Fiche> {
    FICHES.iter().find(|f| f.id == id)
}

/// Les réglages d'une fiche : (groupes, table). Un complément manquant est une erreur de programmation : elle se
/// voit tout de suite (et un test la vérifie pour chaque fiche).
pub fn reglages(f: &Fiche) -> (&'static [&'static str], &'static [Param]) {
    match f.reglages {
        Table(g, p) => (g, p),
        Complement => COMPLEMENT
            .get()
            .and_then(|c| c(f.id))
            .unwrap_or_else(|| panic!("📚 catalogue : réglages de « {} » non inscrits (inscrire_complement)", f.id)),
        // (its inputs, then the style of its plots / fills / hlines: `style_integres`)
        Script(src) => {
            let t = crate::style_integres::table(src, &entrees(src));
            (t.groupes, t.params)
        }
    }
}

/// 🎨 Is the setting at position `k` of the fiche's table a style setting of a built-in script (colour, width or
/// line type of a plot / fill / hline, applied when drawing, never an input of the script)?
pub fn est_style(f: &Fiche, k: usize) -> bool {
    source(f).is_some_and(|src| {
        let t = crate::style_integres::table(src, &entrees(src));
        k >= t.entrees && k < t.params.len()
    })
}

/// 📜 The source of a built-in indicator written in Argos Script (`None`: a built-in written in Rust).
pub fn source(f: &Fiche) -> Option<&'static str> {
    match f.reglages {
        Script(src) => Some(src),
        _ => None,
    }
}

/// 📜 The `input()` calls of a built-in script (read once per script, from a run on 3 flat bars).
pub fn entrees(src: &'static str) -> Vec<crate::lang::InputSpec> {
    use std::collections::HashMap;
    use std::sync::Mutex;
    static VUES: Mutex<Option<HashMap<usize, Vec<crate::lang::InputSpec>>>> = Mutex::new(None);
    let mut v = VUES.lock().unwrap_or_else(|e| e.into_inner());
    v.get_or_insert_with(HashMap::new)
        .entry(src.as_ptr() as usize)
        .or_insert_with(|| {
            // (a few flat bars: enough for the script to run once and declare its inputs)
            let bars: Vec<argos_core::Bar> = (0..3)
                .map(|i| argos_core::Bar {
                    time: i * 60,
                    open: 1.0,
                    high: 1.0,
                    low: 1.0,
                    close: 1.0,
                    volume: 1.0,
                })
                .collect();
            crate::lang::run(src, &bars, 60, &HashMap::new()).map(|o| o.inputs).unwrap_or_default()
        })
        .clone()
}

/// Settings of one indicator by name (step 2 of the Argos Script engine): `Param::key` → value, in table order. A key
/// that appears twice in the same table is written `key@k` the second time (`k`: its position), and values beyond
/// the table (an older, longer table) are kept as `#k`: nothing is ever lost, and `par_position` gives back exactly
/// the same values.
pub fn par_nom(params: &[Param], vals: &[f64]) -> std::collections::BTreeMap<String, f64> {
    let mut out = std::collections::BTreeMap::new();
    for (k, v) in vals.iter().enumerate() {
        let nom = match params.get(k) {
            Some(p) if !out.contains_key(p.key) => p.key.to_string(),
            Some(p) => format!("{}@{k}", p.key),
            None => format!("#{k}"),
        };
        out.insert(nom, *v);
    }
    out
}

/// The settings by position from the settings by name: each table entry takes its value by name (missing: its
/// default), then the kept extra values `#k` follow, in order. Unknown names are ignored here (the caller keeps
/// them in the file).
pub fn par_position(params: &[Param], noms: &std::collections::BTreeMap<String, f64>) -> Vec<f64> {
    let mut vu = std::collections::BTreeSet::new();
    let mut out: Vec<f64> = params
        .iter()
        .enumerate()
        .map(|(k, p)| {
            let nom = if vu.insert(p.key) { p.key.to_string() } else { format!("{}@{k}", p.key) };
            noms.get(&nom).copied().unwrap_or(p.def)
        })
        .collect();
    let mut extra: Vec<(usize, f64)> = noms
        .iter()
        .filter_map(|(n, v)| Some((n.strip_prefix('#')?.parse::<usize>().ok()?, *v)))
        .filter(|(k, _)| *k >= params.len())
        .collect();
    extra.sort_by_key(|x| x.0);
    out.extend(extra.into_iter().map(|x| x.1));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 📚 Step 2: by position → by name → by position gives back the same values (duplicate keys, extra values kept);
    /// a missing name takes its default; an unknown name is ignored.
    #[test]
    fn reglages_par_nom_aller_retour() {
        let p = crate::classic_inputs::MACD_PARAMS.as_slice();
        let vals: Vec<f64> = (0..p.len() + 2).map(|k| k as f64 + 0.5).collect();
        let n = par_nom(p, &vals);
        assert_eq!(n.len(), vals.len(), "one name per value");
        assert!(n.contains_key(&format!("#{}", p.len() + 1)), "extra values kept");
        assert_eq!(par_position(p, &n), vals, "round trip");
        let mut seul = std::collections::BTreeMap::new();
        seul.insert(p[1].key.to_string(), 42.0);
        seul.insert("inconnu".to_string(), 7.0);
        let v = par_position(p, &seul);
        assert_eq!(v.len(), p.len());
        assert_eq!(v[1], 42.0);
        assert_eq!(v[0], p[0].def, "missing: the default");
    }

    /// 📜 Every conversion: a fiche now written in Argos Script, among the 45 of the by-position file, each new key a
    /// key of its table (once), each old key once. Old names → new names once (a second time changes nothing); by
    /// position → by name → back to the native's positions gives the same values.
    #[test]
    fn conversions_des_natifs_remplaces() {
        for c in &CONVERSIONS {
            let f = fiche(c.id).unwrap();
            assert!(source(f).is_some(), "{}: written in Argos Script", c.id);
            assert!(FICHES.iter().position(|g| g.id == c.id).unwrap() < AVANT_SCRIPTS, "{}", c.id);
            let p = reglages(f).1;
            for (k, (a, n)) in c.cles.iter().enumerate() {
                assert!(p.iter().any(|x| x.key == *n), "{}: « {n} » is not a key of its table", c.id);
                assert!(c.cles[k + 1..].iter().all(|(a2, n2)| a2 != a && n2 != n), "{}: « {a} » / « {n} » twice", c.id);
            }
            let anciens: Vec<f64> = (0..c.cles.len()).map(|k| k as f64 + 3.0).collect();
            let noms: std::collections::BTreeMap<String, f64> = c.cles.iter().map(|(a, _)| a.to_string()).zip(anciens.iter().copied()).collect();
            let nouveaux = c.depuis_noms(&noms).expect("converted");
            assert_eq!(nouveaux, c.depuis_positions(&anciens), "by name and by position agree");
            assert_eq!(c.depuis_noms(&nouveaux), None, "once");
            let vals = par_position(p, &nouveaux);
            assert_eq!(c.vers_positions(p, &vals), anciens, "back to the native's positions");
        }
        let b = conversion("Bollinger").unwrap();
        let olds: Vec<&str> = b.cles.iter().map(|x| x.0).collect();
        assert_eq!(
            olds,
            ["len", "mult", "col", "ep1", "ty1", "ep2", "ty2", "ep3", "ty3"],
            "the native's table, in order"
        );
    }

    /// 📚 Fiches uniques, catégories ƒx et Marketplace qui ne citent que des fiches existantes, chaque fiche dans une
    /// seule catégorie ƒx.
    #[test]
    fn catalogue_coherent() {
        for (k, f) in FICHES.iter().enumerate() {
            assert!(FICHES.iter().skip(k + 1).all(|g| g.id != f.id), "{} en double", f.id);
        }
        let fx: Vec<&str> = CATEGORIES_FX.iter().flat_map(|c| c.2.iter().copied()).collect();
        for f in &FICHES {
            assert_eq!(fx.iter().filter(|x| **x == f.id).count(), 1, "{} : une seule catégorie ƒx", f.id);
        }
        assert!(MARCHE.iter().all(|m| fiche(m.0).is_some() && MARCHE_CATEGORIES.iter().any(|c| c.0 == m.1)));
    }
}
