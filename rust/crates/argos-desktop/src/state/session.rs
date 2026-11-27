//! 🔐 La session de l'utilisateur : compte Argos de l'appareil, clés des exchanges et de Claude, jeton OANDA,
//! et les boîtes où on les saisit. Les secrets ne sont gardés qu'en mémoire (ou dans le coffre chiffré).

use std::collections::HashSet;

use argos_data::Exchange;
use argos_render::exchanges::KeyBox;
use argos_render::wordsbox::WordsBox;

use crate::security::account;

pub(crate) struct SessionState {
    /// ✨ Clé Claude ouverte pour la session (jamais gardée en clair : seulement le coffre chiffré).
    pub(crate) claude_key: Option<String>,
    /// Exchanges qui ont une clé d'API enregistrée.
    pub(crate) key_set: HashSet<Exchange>,
    /// 🏦 Jeton OANDA de l'utilisateur et compte réel (`true`) ou démo : relu du trousseau sur desktop ; dans le
    /// navigateur, gardé en mémoire le temps de la session seulement (jamais écrit).
    pub(crate) oanda: Option<(String, bool)>,
    /// 📈 Clé CoinMarketCap : relue du trousseau (desktop), jamais écrite dans un fichier ni affichée.
    pub(crate) cmc: Option<String>,
    /// Boîte de saisie d'une clé d'API ouverte.
    pub(crate) key_box: Option<KeyBox>,
    /// ✨ Compte Argos de cet appareil (public), la création en cours (mots en mémoire) et la boîte des 24 mots.
    pub(crate) account: Option<account::Account>,
    pub(crate) acc_draft: Option<account::Draft>,
    pub(crate) words_box: Option<WordsBox>,
}
