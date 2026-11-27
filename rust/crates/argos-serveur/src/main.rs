//! 🛰️ argos-serveur : le serveur local d'Argos HSS Trading (`make web`, port 8000). C'est le serveur générique
//! d'argos-outils (fichiers, relais, session partagée `/api/reglages`), plus ce qui est propre à Trading : le journal
//! chiffré du PC servi au cell (`/api/journal`), la clé CoinMarketCap de la page rangée au trousseau du PC avec son
//! relais et les logos (`/api/cles/cmc`, `/api/cmc/…`, `/api/logos`), `journal.json` jamais synchronisé en clair, et les réglages dans le
//! dossier propre à Argos HSS Trading. 🔁 And the exchanges relay of the page (`/api/bourse/…`: Bitfinex, KuCoin).
//!
//! Usage : `argos-serveur [port] [dossier] [/accueil/]` (comme `argos-outils serveur`) ; `argos-serveur code-secours` et
//! `argos-serveur restaurer-code` : le code de secours du journal chiffré, dans le terminal (`secours`).

mod bourse;
mod calendrier;
mod cmc;
mod journal;
mod retours;
mod secours;

use std::ffi::OsString;
use std::process::ExitCode;

/// 📁 La session partagée (`/api/reglages`) dans le dossier propre à Argos HSS Trading
/// (`~/.config/argos-hss-trading/charts`, l'ancien `~/.config/argos/charts` copié au premier lancement, sans le
/// journal en clair, ancien intact) ; `$ARGOS_REGLAGES` (tests) garde la main.
fn reglages_propres(s: &mut argos_outils::serveur::Serveur) {
    reglages_propres_de(
        s,
        std::env::var_os("ARGOS_REGLAGES"),
        std::env::var_os("XDG_CONFIG_HOME"),
        std::env::var_os("HOME"),
    );
}

/// [`reglages_propres`] d'après `ARGOS_REGLAGES`, `XDG_CONFIG_HOME` et `HOME` (testable sans toucher à
/// l'environnement).
fn reglages_propres_de(s: &mut argos_outils::serveur::Serveur, force: Option<OsString>, xdg: Option<OsString>, home: Option<OsString>) {
    if force.is_some_and(|d| !d.is_empty()) {
        return;
    }
    let (Some(d), Some(a)) = (
        argos_core::dossiers::reglages_de(xdg.clone(), home.clone()),
        argos_core::dossiers::reglages_ancien_de(xdg, home),
    ) else {
        return;
    };
    if let Err(e) = argos_core::dossiers::preparer(&d, &a) {
        eprintln!("⚠️ copie des réglages vers {} : {e}", d.display());
    }
    s.reglages = d;
}

/// 🆘 The rescue code commands of the encrypted journal (`code-secours`, `restaurer-code`), in this terminal only;
/// `None`: not one of them.
fn commande_secours(nom: &str) -> Option<Result<(), String>> {
    let chemin = || argos_store::sensible::chemin().ok_or_else(|| "pas de dossier de données sur cet appareil".to_string());
    let (entree, sortie) = (&mut std::io::stdin().lock(), &mut std::io::stdout());
    Some(match nom {
        "code-secours" => chemin().and_then(|p| secours::afficher(entree, sortie, &p, argos_store::journal::cle_gardee)),
        "restaurer-code" => chemin().and_then(|p| secours::restaurer(entree, sortie, &p, argos_store::journal::cle_gardee, argos_store::journal::garder_cle)),
        _ => return None,
    })
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(r) = args.first().and_then(|n| commande_secours(n)) {
        return match r {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("❌ {e}");
                ExitCode::FAILURE
            }
        };
    }
    let r = argos_outils::lire_args(args).and_then(|a| {
        argos_outils::serveur::main(&a, |s| {
            // l'ancien journal en clair est migré depuis l'ancien dossier (s.reglages n'a pas encore changé)
            journal::brancher(s, argos_store::sensible::chemin(), argos_store::journal::cle_du_trousseau);
            reglages_propres(s);
            // 🔑 la clé CoinMarketCap tapée dans la page va au trousseau du PC ; relais et logos pour la page
            let reglages = s.reglages.clone();
            cmc::brancher(s, &reglages, cmc::VRAIS);
            retours::brancher(s, retours::dossier(&s.racine.clone()));
            // 📅 the « Réel » of the calendar relay (BLS, Fed)
            calendrier::brancher(s);
            // 🔁 Bitfinex and KuCoin for the page (no CORS): closed lists, cache, cap
            bourse::brancher(s, bourse::appeler_vraiment);
        })
    });
    match r {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("❌ {e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    /// (chemin relatif, contenu) de tous les fichiers d'un dossier, triés.
    fn photo(d: &Path) -> Vec<(std::path::PathBuf, Vec<u8>)> {
        let mut v = vec![];
        let mut pile = vec![d.to_path_buf()];
        while let Some(p) = pile.pop() {
            for e in std::fs::read_dir(&p).unwrap() {
                let e = e.unwrap().path();
                if e.is_dir() {
                    pile.push(e);
                } else {
                    v.push((e.strip_prefix(d).unwrap().to_path_buf(), std::fs::read(&e).unwrap()));
                }
            }
        }
        v.sort();
        v
    }

    /// 📁 Le serveur local sert et écrit la session partagée dans `…/argos-hss-trading/charts` : l'ancien
    /// `…/argos/charts` est copié (sans journal.json) puis plus jamais créé ni modifié.
    #[test]
    fn reglages_dans_le_dossier_de_trading() {
        let base = std::env::temp_dir().join(format!("argos-serveur-reglages-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let ancien = base.join("argos/charts");
        std::fs::create_dir_all(&ancien).unwrap();
        std::fs::write(ancien.join("topbar.json"), b"{\"theme\":1}").unwrap();
        std::fs::write(ancien.join("journal.json"), b"{}").unwrap();
        let avant = photo(&ancien);
        let mut s = argos_outils::serveur::Serveur::new(base.join("www"), ancien.clone());
        // brancher le journal d'abord (il garde l'ancien journal en clair à son ancienne place), puis le dossier
        journal::brancher(&mut s, Some(base.join("data/sensible.redb")), || Ok([7; 32]));
        reglages_propres_de(&mut s, None, Some(base.clone().into_os_string()), None);
        let nouveau = base.join("argos-hss-trading/charts");
        assert_eq!(s.reglages, nouveau);
        assert_eq!(std::fs::read(nouveau.join("topbar.json")).unwrap(), b"{\"theme\":1}");
        assert!(!nouveau.join("journal.json").exists(), "le journal en clair n'est jamais recopié");
        assert_eq!(photo(&ancien), avant, "rien n'est créé ni modifié sous …/argos");
        // $ARGOS_REGLAGES (tests, autres usages) garde la main
        let mut t = argos_outils::serveur::Serveur::new(base.join("www"), base.join("force"));
        reglages_propres_de(&mut t, Some("x".into()), Some(base.clone().into_os_string()), None);
        assert_eq!(t.reglages, base.join("force"));
        let _ = std::fs::remove_dir_all(&base);
    }
}
