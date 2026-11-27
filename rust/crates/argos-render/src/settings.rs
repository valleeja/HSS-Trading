//! Réglages de la chart : **une seule source de vérité**. Le rendu, les menus (clic droit, ⚙), la fenêtre Réglages et
//! les raccourcis lisent et modifient tous cette même structure. Elle est sauvegardée telle quelle (JSON) : un réglage
//! ajouté plus tard prend sa valeur par défaut dans les vieux fichiers (`#[serde(default)]`).

use serde::{Deserialize, Serialize};

/// Tous les réglages de la chart.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ChartSettings {
    /// Échelle de prix (menu ⚙ / clic droit sur la colonne de prix).
    pub scale: ScaleOptions,
    /// Étiquettes sur l'échelle de prix (sous-menu « Étiquettes »).
    pub labels: Labels,
    /// Lignes de prix (sous-menu « Lignes »).
    pub lines: Lines,
    /// Bouton ➕ à côté du prix du réticule.
    pub plus_button: bool,
    /// Lignes verticales à chaque changement de jour (« Arrêts de session »).
    pub session_breaks: bool,
    /// Fuseau horaire (nom IANA, ex. « America/New_York ») ; vide = celui de l'ordinateur.
    pub timezone: String,
    /// Type de chart TPO (clic droit → TPO, comme LuxAlgo).
    pub tpo: TpoOptions,
    /// Hauteur choisie de chaque bande d'oscillateur (glisser son bord ; double-clic sur le bord = par défaut).
    pub bands: std::collections::HashMap<crate::indicators::Ind, f64>,
    /// 📜 Chosen height of each user script's lower pane, by script id (dragging its top edge, like a band).
    pub script_bands: std::collections::HashMap<String, f64>,
    /// 📅 Annonces économiques (impact fort et moyen) au bas de la chart, avec le drapeau de la devise.
    pub econ_events: bool,
    /// 📰 Nouvelles du marché dans la rangée des annonces (survol = titre et source).
    pub econ_news: bool,
    /// 🛡️ Fenêtres PropFirm : ±2 min autour des annonces à fort impact (pas d'ordre dedans si le gardien le demande).
    pub propfirm_windows: bool,
    /// Vue de chaque bande d'oscillateur : (décalage en fraction de son échelle, zoom) ; glisser dedans / sur sa colonne
    /// de valeurs, double-clic sur sa colonne = remise à zéro.
    pub band_view: std::collections::HashMap<crate::indicators::Ind, (f64, f64)>,
    /// 📜 The same view for each script's lower pane, by its key (`ScriptView::view_key`).
    pub script_view: std::collections::HashMap<String, (f64, f64)>,
}

/// Réglages du type de chart TPO (défauts de LuxAlgo).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TpoOptions {
    /// Durée d'une lettre (secondes).
    pub block: i64,
    /// Zone de valeur (%).
    pub va: u8,
    /// Initial balance (minutes).
    pub ib: i64,
    /// Lettres et blocs (les deux se cochent séparément).
    pub letters: bool,
    pub blocks: bool,
}

impl Default for TpoOptions {
    fn default() -> Self {
        Self {
            block: 1800,
            va: 70,
            ib: 60,
            letters: true,
            blocks: true,
        }
    }
}

impl Default for ChartSettings {
    fn default() -> Self {
        Self {
            scale: ScaleOptions::default(),
            labels: Labels::default(),
            lines: Lines::default(),
            plus_button: true,
            session_breaks: false,
            timezone: String::new(),
            tpo: TpoOptions::default(),
            bands: Default::default(),
            script_bands: Default::default(),
            econ_events: true,
            econ_news: true,
            propfirm_windows: true,
            band_view: Default::default(),
            script_view: Default::default(),
        }
    }
}

/// Étiquettes sur l'échelle de prix (comme le sous-menu « Étiquettes » de TradingView).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Labels {
    /// Nom du symbole à côté du dernier prix.
    pub symbol_name: bool,
    /// Clôture du jour précédent.
    pub prev_close: bool,
    /// Plus haut et plus bas de la partie visible.
    pub high_low: bool,
    /// 📍 Prix du plus haut et du plus bas visibles écrits sur la chart, à côté de leur mèche (Jee, 21 nov.).
    pub high_low_chart: bool,
    /// Compte à rebours jusqu'à la fermeture de la bougie (sous le dernier prix).
    pub countdown: bool,
    /// 📖 Meilleur achat et meilleure vente du carnet, sur l'échelle.
    pub bid_ask: bool,
}

impl Default for Labels {
    fn default() -> Self {
        Self {
            symbol_name: false,
            prev_close: false,
            high_low: false,
            high_low_chart: true,
            countdown: true,
            bid_ask: false,
        }
    }
}

/// Lignes de prix (comme le sous-menu « Lignes » de TradingView).
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Lines {
    /// Clôture du jour précédent.
    pub prev_close: bool,
    /// Plus haut et plus bas de la partie visible.
    pub high_low: bool,
    /// 📖 Lignes du meilleur achat et de la meilleure vente du carnet.
    pub bid_ask: bool,
}

/// Fuseaux offerts (comme la liste de TradingView) : (ville affichée, nom IANA). « Marché » = UTC pour la crypto.
pub const TIMEZONES: &[(&str, &str)] = &[
    ("UTC", "UTC"),
    ("Marché", "UTC"),
    ("Honolulu", "Pacific/Honolulu"),
    ("Ancrage", "America/Anchorage"),
    ("Juneau", "America/Juneau"),
    ("Los Angeles", "America/Los_Angeles"),
    ("Phoenix", "America/Phoenix"),
    ("Vancouver", "America/Vancouver"),
    ("Denver", "America/Denver"),
    ("San Salvador", "America/El_Salvador"),
    ("Ville de Mexico", "America/Mexico_City"),
    ("Bogota", "America/Bogota"),
    ("Chicago", "America/Chicago"),
    ("Lima", "America/Lima"),
    ("Caracas", "America/Caracas"),
    ("New York", "America/New_York"),
    ("Toronto", "America/Toronto"),
    ("Buenos Aires", "America/Argentina/Buenos_Aires"),
    ("Halifax", "America/Halifax"),
    ("Santiago", "America/Santiago"),
    ("São Paulo", "America/Sao_Paulo"),
    ("Açores", "Atlantic/Azores"),
    ("Dublin", "Europe/Dublin"),
    ("Lisbonne", "Europe/Lisbon"),
    ("Londres", "Europe/London"),
    ("Paris", "Europe/Paris"),
    ("Berlin", "Europe/Berlin"),
    ("Zurich", "Europe/Zurich"),
    ("Athènes", "Europe/Athens"),
    ("Moscou", "Europe/Moscow"),
    ("Dubaï", "Asia/Dubai"),
    ("Calcutta", "Asia/Kolkata"),
    ("Bangkok", "Asia/Bangkok"),
    ("Singapour", "Asia/Singapore"),
    ("Hong Kong", "Asia/Hong_Kong"),
    ("Shanghai", "Asia/Shanghai"),
    ("Tokyo", "Asia/Tokyo"),
    ("Séoul", "Asia/Seoul"),
    ("Sydney", "Australia/Sydney"),
    ("Auckland", "Pacific/Auckland"),
];

/// Décalage (secondes) du fuseau `tz` au moment `t` (secondes UTC) ; `None` si le nom est inconnu.
pub fn tz_offset(tz: &str, t: i64) -> Option<i64> {
    use chrono::{Offset, TimeZone};
    let zone: chrono_tz::Tz = tz.parse().ok()?;
    let at = chrono::DateTime::from_timestamp(t, 0)?.naive_utc();
    Some(zone.offset_from_utc_datetime(&at).fix().local_minus_utc() as i64)
}

/// « (UTC-4) New York » pour la liste des fuseaux, au moment `t`.
pub fn tz_label(city: &str, tz: &str, t: i64) -> String {
    let off = tz_offset(tz, t).unwrap_or(0);
    let (h, m) = (off / 3600, (off.abs() % 3600) / 60);
    match (h, m) {
        (0, 0) if city == "UTC" || city == "Marché" => city.to_string(),
        (0, 0) => format!("(UTC) {city}"),
        (h, 0) => format!("(UTC{h:+}) {city}"),
        (h, m) => format!("(UTC{h:+}:{m:02}) {city}"),
    }
}

/// Mode de l'échelle de prix (comme TradingView).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ScaleMode {
    #[default]
    Normal,
    /// % de variation depuis la première bougie visible.
    Percent,
    /// Indexé sur 100 à la première bougie visible.
    Indexed,
    /// Logarithmique : même hauteur pour le même % de variation.
    Log,
}

/// Réglages de l'échelle de prix.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ScaleOptions {
    pub mode: ScaleMode,
    /// Échelle inversée (le haut en bas).
    pub invert: bool,
    /// Bornes fixées à la main (dans l'espace de l'échelle : ln du prix en logarithmique). `None` = ajustement automatique.
    pub manual: Option<(f64, f64)>,
    /// Rapport prix / barre verrouillé : écart d'échelle par pixel de hauteur et par pixel d'espace entre bougies.
    pub lock_ratio: Option<f64>,
    /// L'échelle suit seulement la série principale (pas les indicateurs).
    pub main_only: bool,
    /// Étiquette du dernier prix sur l'échelle.
    pub last_price_label: bool,
    /// Ligne pointillée du dernier prix.
    pub last_price_line: bool,
    /// Lignes de la grille.
    pub grid: bool,
}

impl Default for ScaleOptions {
    fn default() -> Self {
        Self {
            mode: ScaleMode::Normal,
            invert: false,
            manual: None,
            lock_ratio: None,
            main_only: true,
            last_price_label: true,
            last_price_line: true,
            grid: true,
        }
    }
}

impl ScaleOptions {
    /// Ajustement automatique (ni bornes à la main, ni rapport verrouillé).
    pub fn auto(&self) -> bool {
        self.manual.is_none() && self.lock_ratio.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aller_retour_json_et_vieux_fichiers() {
        let mut s = ChartSettings::default();
        s.scale.mode = ScaleMode::Log;
        s.scale.invert = true;
        s.scale.manual = Some((1.0, 2.0));
        s.labels.high_low = true;
        s.timezone = "America/New_York".into();
        let j = serde_json::to_string(&s).unwrap();
        assert_eq!(serde_json::from_str::<ChartSettings>(&j).unwrap(), s);
        // fichier d'une vieille version (réglages manquants) : valeurs par défaut pour le reste
        let old: ChartSettings = serde_json::from_str(r#"{"scale":{"invert":true}}"#).unwrap();
        assert!(old.scale.invert && old.scale.grid && old.scale.mode == ScaleMode::Normal);
        assert_eq!(serde_json::from_str::<ChartSettings>("{}").unwrap(), ChartSettings::default());
        // 📍 a file from before the high / low prices on the chart: they are on
        let old: ChartSettings = serde_json::from_str(r#"{"labels":{"high_low":true,"countdown":false}}"#).unwrap();
        assert!(old.labels.high_low && old.labels.high_low_chart && !old.labels.countdown);
    }

    #[test]
    fn fuseaux() {
        // 25 oct. 2026 : heure d'été à New York (−4), pas à Tokyo (+9)
        let t = 1_792_929_600;
        assert_eq!(tz_offset("America/New_York", t), Some(-4 * 3600));
        assert_eq!(tz_offset("Asia/Tokyo", t), Some(9 * 3600));
        assert_eq!(tz_offset("Pas/Une_Ville", t), None);
        assert_eq!(tz_label("New York", "America/New_York", t), "(UTC-4) New York");
        assert_eq!(tz_label("Calcutta", "Asia/Kolkata", t), "(UTC+5:30) Calcutta");
        assert_eq!(tz_label("UTC", "UTC", t), "UTC");
        assert!(TIMEZONES.iter().all(|(_, z)| tz_offset(z, t).is_some()), "tous les fuseaux connus");
    }
}
