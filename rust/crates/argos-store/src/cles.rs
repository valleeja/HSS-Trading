//! 🔑 Clés API dans le **trousseau du système** : Secret Service (trousseau GNOME / KWallet) sous Linux, Trousseau
//! sous macOS, Gestionnaire d'identifiants sous Windows. Argos ne les écrit jamais dans un fichier, ne les affiche
//! jamais en clair et ne les envoie nulle part d'autre que le service concerné.
//! Une entrée par champ : service « argos-hss-trading » (propre à Argos HSS Trading, comme la clé du journal
//! chiffré), compte « <service>.<champ> » (ex. « binance.secret », « oanda.token »). Une clé rangée avant sous
//! l'ancien service commun « argos » est reprise une fois, à sa première lecture : copiée sous le nouveau nom,
//! l'ancienne entrée laissée telle quelle.
//! Partagé par l'app de bureau et le serveur local (qui range la clé CoinMarketCap tapée dans la page du PC ou du
//! cell, sans jamais la renvoyer).
//! 🧪 `ARGOS_SERVICE_CLES` : un autre service (essais avec un trousseau de test) ; l'ancien service n'est alors jamais
//! relu.

/// Le service d'Argos HSS Trading dans le trousseau.
pub const SERVICE: &str = "argos-hss-trading";
/// L'ancien service, commun avec d'autres projets Argos : seulement relu (reprise), jamais écrit.
pub const ANCIEN_SERVICE: &str = "argos";

/// Tous les champs possibles d'un service (pour savoir s'il a une clé, et pour tout oublier).
pub const CHAMPS: [&str; 6] = ["key", "secret", "passphrase", "address", "token", "env"];

/// Marque « oubliée » d'un service (sous le nouveau nom, sans secret : la valeur est « 1 ») : ses anciennes entrées
/// sous « argos » ne sont plus jamais reprises.
const OUBLIEE: &str = "oubliee";

/// Le service utilisé (`ARGOS_SERVICE_CLES` s'il est donné, sinon [`SERVICE`]) et s'il faut reprendre l'ancien.
fn service() -> (String, bool) {
    match std::env::var("ARGOS_SERVICE_CLES") {
        Ok(s) if !s.is_empty() => (s, false),
        _ => (SERVICE.into(), true),
    }
}

fn entree(service: &str, id: &str, champ: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new(service, &format!("{id}.{champ}")).map_err(|e| format!("trousseau : {e}"))
}

/// La valeur sous le nouveau nom ; sinon (et si le service n'a pas été oublié) celle de l'ancien nom, recopiée sous
/// le nouveau (l'ancienne n'est pas touchée). La logique seule, sans trousseau (testable).
pub fn reprendre(
    nouveau: impl Fn() -> Option<String>,
    oubliee: impl Fn() -> bool,
    ancien: impl Fn() -> Option<String>,
    copier: impl Fn(&str),
) -> Option<String> {
    nouveau().or_else(|| {
        if oubliee() {
            return None;
        }
        let v = ancien()?;
        copier(&v);
        Some(v)
    })
}

/// Lit un champ (nouveau nom, sinon reprise depuis l'ancien).
pub fn lire(id: &str, champ: &str) -> Option<String> {
    let (s, reprise) = service();
    let get = |sv: &str, c: &str| entree(sv, id, c).ok()?.get_password().ok();
    reprendre(
        || get(&s, champ),
        || !reprise || get(&s, OUBLIEE).is_some(),
        || get(ANCIEN_SERVICE, champ),
        |v| {
            if let Ok(e) = entree(&s, id, champ) {
                let _ = e.set_password(v);
            }
        },
    )
}

/// Range un champ (`id` : l'exchange, « oanda », « cmc »…).
pub fn ranger(id: &str, champ: &str, valeur: &str) -> Result<(), String> {
    entree(&service().0, id, champ)?.set_password(valeur).map_err(|e| format!("trousseau : {e}"))
}

/// Une clé (ou une adresse) est-elle rangée pour ce service ? (La valeur lue est oubliée tout de suite.)
pub fn a_une_cle(id: &str) -> bool {
    CHAMPS.iter().any(|c| lire(id, c).is_some())
}

/// Oublie tout ce qui est rangé pour ce service sous le nouveau nom, et le marque « oublié » : ses anciennes entrées
/// sous « argos » (peut-être utiles à un autre projet Argos) restent intactes mais ne sont plus jamais reprises.
pub fn oublier(id: &str) {
    let s = service().0;
    for c in CHAMPS {
        if let Ok(e) = entree(&s, id, c) {
            let _ = e.delete_credential();
        }
    }
    if let Ok(e) = entree(&s, id, OUBLIEE) {
        let _ = e.set_password("1");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    /// Reprise : sous le nouveau nom d'abord ; sinon l'ancienne valeur est recopiée sous le nouveau nom, une fois, et
    /// l'ancienne n'est jamais écrite.
    #[test]
    fn reprise_depuis_l_ancien_service() {
        let nouveau: RefCell<Option<String>> = RefCell::new(None);
        let ancien: RefCell<Option<String>> = RefCell::new(Some("valeur-ancienne".into()));
        let oubliee = RefCell::new(false);
        let lire = || {
            reprendre(
                || nouveau.borrow().clone(),
                || *oubliee.borrow(),
                || ancien.borrow().clone(),
                |v| *nouveau.borrow_mut() = Some(v.to_string()),
            )
        };
        assert_eq!(lire().as_deref(), Some("valeur-ancienne"));
        assert_eq!(nouveau.borrow().as_deref(), Some("valeur-ancienne"), "recopiée sous le nouveau nom");
        assert_eq!(ancien.borrow().as_deref(), Some("valeur-ancienne"), "l'ancienne est intacte");
        // le nouveau nom gagne ensuite, même si l'ancienne change ailleurs
        *ancien.borrow_mut() = Some("autre".into());
        assert_eq!(lire().as_deref(), Some("valeur-ancienne"));
        // rien nulle part : rien
        *nouveau.borrow_mut() = None;
        *ancien.borrow_mut() = None;
        assert_eq!(lire(), None);
        assert!(nouveau.borrow().is_none(), "rien n'est créé");
        // « Oublier » : la nouvelle entrée effacée et la marque posée ; l'ancienne existe toujours, et ne revient pas
        *ancien.borrow_mut() = Some("valeur-ancienne".into());
        *nouveau.borrow_mut() = None;
        *oubliee.borrow_mut() = true;
        assert_eq!(lire(), None, "la clé oubliée ne revient pas");
        assert_eq!(ancien.borrow().as_deref(), Some("valeur-ancienne"), "l'ancienne entrée existe toujours");
        assert!(nouveau.borrow().is_none());
        // une nouvelle clé rangée après l'oubli est bien relue
        *nouveau.borrow_mut() = Some("nouvelle".into());
        assert_eq!(lire().as_deref(), Some("nouvelle"));
    }

    #[test]
    fn noms_des_services() {
        assert_eq!(SERVICE, "argos-hss-trading");
        assert_eq!(ANCIEN_SERVICE, "argos");
    }
}
