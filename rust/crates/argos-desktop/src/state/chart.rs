//! 📈 La chart du symbole ouvert : d'où viennent les bougies, les bougies elles-mêmes, le type et l'intervalle
//! affichés, la vue (zoom, défilement), les réglages, les dessins, l'état du direct et la sauvegarde du symbole.

use argos_core::{Bar, Interval};
use argos_render::drawings::Drawing;
use argos_render::settings::ChartSettings;
use argos_render::viewport::{Slots, Viewport};
use argos_render::CandleStyle;

use crate::persist::saved;
use crate::run::Source;

pub(crate) struct ChartState {
    /// D'où viennent les bougies (F5 / Ctrl + R = recharger).
    pub(crate) source: Source,
    pub(crate) bars: Vec<Bar>,
    /// Place de chaque bougie (selon le type), pour le zoom et le défilement.
    pub(crate) slots: Slots,
    /// Types offerts et celui affiché.
    pub(crate) styles: Vec<CandleStyle>,
    pub(crate) style: usize,
    /// Intervalle affiché. Les bougies brutes (`raw`) viennent de l'intervalle de l'exchange qui tombe juste dedans
    /// (`Interval::source`) ; `bars` = elles regroupées dans l'intervalle. Secondes : construites avec les trades.
    pub(crate) interval: Interval,
    pub(crate) raw: Vec<Bar>,
    /// Intervalle des bougies d'un fichier (on peut seulement les regrouper plus haut).
    pub(crate) file_interval: Interval,
    pub(crate) viewport: Viewport,
    /// Réglages de la chart (source unique, sauvegardés par symbole).
    pub(crate) settings: ChartSettings,
    /// Fichier de sauvegarde de ce symbole, et le dernier contenu écrit (pour n'écrire que s'il a changé).
    /// Nom de la sauvegarde du symbole ouvert (`saved`).
    pub(crate) save_path: Option<String>,
    pub(crate) last_saved: std::cell::RefCell<Option<saved::Saved>>,
    /// Objets dessinés (moteur d'objets : outil, points temps + prix, style).
    pub(crate) drawings: Vec<Drawing>,
    /// ↶ ↷ Historique des dessins de cette chart (en mémoire seulement, vidé au changement de symbole).
    pub(crate) hist: std::cell::RefCell<DrawHistory>,
    /// Direct : `None` = pas de direct pour cette source, `Some(true)` = branché, `Some(false)` = connexion / reconnexion.
    pub(crate) live: Option<bool>,
    /// Chargement du démarrage en cours : les bougies du direct reçues entre-temps attendent ici.
    pub(crate) loading: Option<Vec<Bar>>,
    /// Fichier du cache des bougies (exchange seulement ; `None` si les bougies viennent d'un secours).
    pub(crate) cache_path: Option<std::path::PathBuf>,
    /// 📜 Historique plus ancien : demande en cours, début atteint (rien de plus ancien), et pour quel marché
    /// (« symbole|unité » : un autre marché repart de zéro).
    pub(crate) older_busy: bool,
    pub(crate) history_start: bool,
    pub(crate) older_key: String,
    /// Dernière demande d'historique plus ancien (au plus une toutes les quelques secondes : pas de rafale).
    pub(crate) older_at: Option<web_time::Instant>,
    /// 📜 Exchange d'appoint pour l'historique plus ancien quand celui de la chart s'arrête (Hyperliquid ne garde que
    /// 5000 bougies par unité) : Binance Futures, le même perp.
    pub(crate) older_via: Option<argos_data::Exchange>,
    /// 🕳️ Trous (étape 3) : heures des bougies refusées à réparer, réparation en cours, et trous que l'exchange n'a
    /// pas (pour ce marché « symbole|unité », avec l'heure de l'essai : réessayés une fois par semaine).
    pub(crate) a_reparer: Vec<i64>,
    pub(crate) repair_busy: bool,
    pub(crate) trous_connus: (String, Vec<argos_data::trous::Connu>),
    /// 📴 Plus de réseau (direct coupé ou chargement en échec) : étiquette sur la chart ; et le direct a été coupé
    /// (au retour : rattrapage des bougies manquées, puis réparation des trous).
    pub(crate) hors_ligne: bool,
    pub(crate) coupe: bool,
    /// 🧾 Revue R&D, point 1 : début des bougies de l'exchange de la chart quand le passé plus ancien vient d'un
    /// exchange d'appoint (`older_via`). Les bougies d'avant ne vont jamais dans son cache (prix d'un autre exchange).
    pub(crate) debut_propre: Option<i64>,
}

/// ↶ ↷ Historique des dessins : chaque état d'avant un changement (ajout, déplacement, réglages, style, suppression).
#[derive(Debug, Default)]
pub(crate) struct DrawHistory {
    /// Les dessins tels qu'au dernier relevé (`None` : pas encore relevés pour ce symbole).
    pub(crate) base: Option<Vec<Drawing>>,
    pub(crate) undo: Vec<Vec<Drawing>>,
    pub(crate) redo: Vec<Vec<Drawing>>,
}

/// Pas d'historique gardés au plus.
pub(crate) const HIST_MAX: usize = 100;
