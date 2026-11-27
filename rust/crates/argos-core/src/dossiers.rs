//! 📁 Les dossiers propres à Argos HSS Trading (séparé d'Argos HSS Project, décision de Jee du 15 nov.) : les
//! réglages de l'app (`~/.config/argos-hss-trading/charts`), partagés par l'app de bureau, la coque Tauri et le
//! serveur local (`/api/reglages`). Au premier lancement, l'ancien dossier (`~/.config/argos/charts`) est **copié**
//! dans le nouveau, sauf l'ancien journal en clair (`journal.json`, migré à part dans la base chiffrée) ; l'ancien
//! dossier n'est jamais modifié ni effacé.

use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};

/// Nom du dossier d'Argos HSS Trading (comme le cache et la base : `argos-hss-trading`).
pub const APP: &str = "argos-hss-trading";
/// Nom de l'ancien dossier, commun avec d'autres projets Argos : seulement relu (copie), jamais écrit.
pub const ANCIEN_APP: &str = "argos";
/// Jamais recopiés : l'ancien journal en clair (sa migration le lit dans l'ancien dossier, puis l'efface).
pub const JAMAIS_COPIES: [&str; 1] = ["journal.json"];

/// `$XDG_CONFIG_HOME`, sinon `~/.config`.
fn config(xdg: Option<OsString>, home: Option<OsString>) -> Option<PathBuf> {
    xdg.filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| home.filter(|v| !v.is_empty()).map(|h| PathBuf::from(h).join(".config")))
}

/// Le dossier des réglages d'après `XDG_CONFIG_HOME` et `HOME` : `…/argos-hss-trading/charts`.
pub fn reglages_de(xdg: Option<OsString>, home: Option<OsString>) -> Option<PathBuf> {
    Some(config(xdg, home)?.join(APP).join("charts"))
}

/// L'ancien dossier des réglages : `…/argos/charts` (seulement relu).
pub fn reglages_ancien_de(xdg: Option<OsString>, home: Option<OsString>) -> Option<PathBuf> {
    Some(config(xdg, home)?.join(ANCIEN_APP).join("charts"))
}

/// Le dossier des réglages de cette session (`~/.config/argos-hss-trading/charts`).
pub fn reglages() -> Option<PathBuf> {
    reglages_de(std::env::var_os("XDG_CONFIG_HOME"), std::env::var_os("HOME"))
}

/// L'ancien dossier des réglages de cette session (`~/.config/argos/charts`).
pub fn reglages_ancien() -> Option<PathBuf> {
    reglages_ancien_de(std::env::var_os("XDG_CONFIG_HOME"), std::env::var_os("HOME"))
}

/// Premier lancement : si `nouveau` n'existe pas encore et `ancien` existe, copie tout `ancien` dans `nouveau` (sauf
/// [`JAMAIS_COPIES`], les liens symboliques ignorés). La copie se fait dans un dossier à côté puis prend sa place d'un
/// coup : jamais de dossier à moitié copié. `ancien` n'est jamais modifié. Rend le nombre de fichiers copiés (0 : rien
/// à faire).
pub fn preparer(nouveau: &Path, ancien: &Path) -> io::Result<usize> {
    if nouveau.exists() || !ancien.is_dir() {
        return Ok(0);
    }
    let parent = nouveau.parent().ok_or_else(|| io::Error::other("dossier sans parent"))?;
    std::fs::create_dir_all(parent)?;
    let tmp = parent.join(format!(".copie-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    let n = copier(ancien, &tmp, true)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o700));
    }
    match std::fs::rename(&tmp, nouveau) {
        Ok(()) => Ok(n),
        // un autre programme d'Argos HSS Trading l'a fait en même temps : on garde le sien
        Err(_) if nouveau.exists() => {
            let _ = std::fs::remove_dir_all(&tmp);
            Ok(0)
        }
        Err(e) => {
            let _ = std::fs::remove_dir_all(&tmp);
            Err(e)
        }
    }
}

/// Copie récursive de `de` vers `vers` ; au premier niveau, sans [`JAMAIS_COPIES`].
fn copier(de: &Path, vers: &Path, racine: bool) -> io::Result<usize> {
    std::fs::create_dir_all(vers)?;
    let mut n = 0;
    for e in std::fs::read_dir(de)? {
        let e = e?;
        let nom = e.file_name();
        if racine && JAMAIS_COPIES.iter().any(|j| nom == *j) {
            continue;
        }
        let t = e.file_type()?;
        if t.is_dir() {
            n += copier(&e.path(), &vers.join(&nom), false)?;
        } else if t.is_file() {
            std::fs::copy(e.path(), vers.join(&nom))?;
            n += 1;
        }
    }
    Ok(n)
}

/// 🧪 Où les tests de toutes les crates écrivent : `rust/target/tmp` (le dossier de compilation du projet), jamais
/// `/tmp` ni les vrais dossiers de l'utilisateur.
pub fn racine_des_tests() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/tmp")
}

/// 🧪 Le dossier de test `nom` de ce programme de tests : `rust/target/tmp/argos-tests-<nom>-<n° du processus>`. Au
/// premier appel, les dossiers laissés par les essais précédents (processus terminés) sont effacés : il ne reste que
/// ceux du dernier essai, dans le dossier de compilation.
pub fn de_test(nom: &str) -> PathBuf {
    static MENAGE: std::sync::Once = std::sync::Once::new();
    let racine = racine_des_tests();
    MENAGE.call_once(|| menage_des_tests(&racine, std::process::id()));
    racine.join(format!("argos-tests-{nom}-{}", std::process::id()))
}

/// Efface dans `racine` les dossiers `argos-tests-…-<n°>` d'un autre processus que `moi` qui ne tourne plus (Linux :
/// `/proc/<n°>` absent).
fn menage_des_tests(racine: &Path, moi: u32) {
    let Ok(rd) = std::fs::read_dir(racine) else { return };
    for e in rd.flatten() {
        let nom = e.file_name().to_string_lossy().into_owned();
        let Some(n) = nom
            .strip_prefix("argos-tests-")
            .and_then(|r| r.rsplit('-').next())
            .and_then(|p| p.parse::<u32>().ok())
        else {
            continue;
        };
        if n != moi && !Path::new("/proc").join(n.to_string()).exists() {
            let _ = std::fs::remove_dir_all(e.path());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 🧪 Les dossiers de test sont dans le dossier de compilation ; ceux des essais terminés sont effacés, pas ceux
    /// d'un processus qui tourne encore, ni le reste.
    #[test]
    fn dossiers_de_test_et_menage() {
        let d = de_test("essai");
        assert!(d.starts_with(racine_des_tests()) && !d.starts_with(std::env::temp_dir()), "{}", d.display());
        let r = racine_des_tests().join(format!("menage-{}", std::process::id()));
        // 4 000 000 000 : au-delà de tout numéro de processus ; 1 : init, toujours là
        for n in ["argos-tests-cache-4000000000", "argos-tests-a-b-1", "argos-tests-moi-7", "autre-4000000000"] {
            std::fs::create_dir_all(r.join(n).join("x")).unwrap();
        }
        menage_des_tests(&r, 7);
        let mut reste: Vec<String> = std::fs::read_dir(&r)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        reste.sort();
        assert_eq!(reste, ["argos-tests-a-b-1", "argos-tests-moi-7", "autre-4000000000"]);
        let _ = std::fs::remove_dir_all(&r);
    }

    fn tmp(nom: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("argos-dossiers-{nom}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// Empreinte simple d'un dossier : (chemin relatif, contenu) triés.
    fn photo(d: &Path) -> Vec<(PathBuf, Vec<u8>)> {
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

    #[test]
    fn noms_des_dossiers() {
        let h = Some(OsString::from("/home/x"));
        assert_eq!(reglages_de(None, h.clone()), Some(PathBuf::from("/home/x/.config/argos-hss-trading/charts")));
        assert_eq!(reglages_ancien_de(None, h.clone()), Some(PathBuf::from("/home/x/.config/argos/charts")));
        assert_eq!(
            reglages_de(Some("/x/cfg".into()), h.clone()),
            Some(PathBuf::from("/x/cfg/argos-hss-trading/charts"))
        );
        assert_eq!(reglages_de(Some("".into()), h), Some(PathBuf::from("/home/x/.config/argos-hss-trading/charts")));
        assert_eq!(reglages_de(None, None), None);
    }

    /// Premier lancement : tout est recopié sauf journal.json ; l'ancien dossier reste identique ; au lancement
    /// suivant, rien n'est recopié (le nouveau dossier est à lui, même s'il diverge).
    #[test]
    fn copie_au_premier_lancement_sans_le_journal_et_ancien_intact() {
        let base = tmp("copie");
        let (ancien, nouveau) = (base.join("argos/charts"), base.join("argos-hss-trading/charts"));
        std::fs::create_dir_all(ancien.join("dessins")).unwrap();
        std::fs::write(ancien.join("topbar.json"), b"{\"theme\":1}").unwrap();
        std::fs::write(ancien.join("watchlists.json"), b"[1,2]").unwrap();
        std::fs::write(ancien.join("journal.json"), b"{\"trades\":[\"secret\"]}").unwrap();
        std::fs::write(ancien.join("dessins/BTC.json"), b"[\"rect\"]").unwrap();
        let avant = photo(&ancien);
        assert_eq!(preparer(&nouveau, &ancien).unwrap(), 3);
        assert_eq!(std::fs::read(nouveau.join("topbar.json")).unwrap(), b"{\"theme\":1}");
        assert_eq!(std::fs::read(nouveau.join("dessins/BTC.json")).unwrap(), b"[\"rect\"]");
        assert!(!nouveau.join("journal.json").exists(), "le journal en clair n'est jamais recopié");
        assert_eq!(photo(&ancien), avant, "l'ancien dossier n'a pas bougé");
        // le nouveau vit sa vie : un 2e lancement ne recopie rien et n'écrit rien dans l'ancien
        std::fs::write(nouveau.join("topbar.json"), b"{\"theme\":2}").unwrap();
        assert_eq!(preparer(&nouveau, &ancien).unwrap(), 0);
        assert_eq!(std::fs::read(nouveau.join("topbar.json")).unwrap(), b"{\"theme\":2}");
        assert_eq!(photo(&ancien), avant);
        // aucun dossier de copie laissé derrière
        let restes: Vec<_> = std::fs::read_dir(base.join("argos-hss-trading"))
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(restes, vec![OsString::from("charts")]);
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn rien_a_copier() {
        let base = tmp("vide");
        let (ancien, nouveau) = (base.join("argos/charts"), base.join("argos-hss-trading/charts"));
        assert_eq!(preparer(&nouveau, &ancien).unwrap(), 0, "pas d'ancien dossier : rien");
        assert!(!nouveau.exists(), "le nouveau sera créé à la première écriture");
        let _ = std::fs::remove_dir_all(&base);
    }
}
