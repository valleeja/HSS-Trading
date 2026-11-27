//! 🔑 Clés API dans le **trousseau du système** (`argos_store::cles` : service « argos-hss-trading », compte
//! « <service>.<champ> », reprise depuis l'ancien service « argos »). Argos ne les écrit jamais dans un fichier, ne
//! les affiche jamais en clair et ne les envoie nulle part d'autre que le service concerné.
//! Seul le jeton OANDA est relu (`read`) : il sert à demander les bougies à OANDA (et la clé CoinMarketCap, pour
//! ses appels).
//!
//! Dans le navigateur (WASM) : la page ne garde aucune clé. La clé CoinMarketCap part au serveur du PC
//! (`actions::cmc`, route `/api/cles/cmc`) qui la range dans son trousseau ; les autres clés : pas dans le navigateur.

/// Range un champ dans le trousseau (`id` : l'exchange, ou « oanda »).
pub fn save(id: &str, field: &str, value: &str) -> Result<(), String> {
    #[cfg(not(target_arch = "wasm32"))]
    return argos_store::cles::ranger(id, field, value);
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (id, field, value);
        Err("clés dans le navigateur : seulement CoinMarketCap, par le serveur de ton PC".into())
    }
}

/// Relit un champ (le jeton OANDA, pour demander les bougies) ; `None` s'il n'est pas rangé.
pub fn read(id: &str, field: &str) -> Option<String> {
    #[cfg(not(target_arch = "wasm32"))]
    return argos_store::cles::lire(id, field);
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (id, field);
        None
    }
}

/// Une clé (ou une adresse) est-elle rangée pour ce service ? (La valeur lue est oubliée tout de suite.)
pub fn has(id: &str) -> bool {
    #[cfg(not(target_arch = "wasm32"))]
    return argos_store::cles::a_une_cle(id);
    #[cfg(target_arch = "wasm32")]
    {
        let _ = id;
        false
    }
}

/// Oublie tout ce qui est rangé pour ce service (l'ancienne entrée sous « argos » reste, sans jamais revenir).
pub fn forget(id: &str) {
    #[cfg(not(target_arch = "wasm32"))]
    argos_store::cles::oublier(id);
    #[cfg(target_arch = "wasm32")]
    let _ = id;
}
