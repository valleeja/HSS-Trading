//! 🛟 Support tickets on disk (the tickets folder, usually `Argos HSS Trading/retours/`): create, list, reply,
//! change the state, mark as read. Shared by the local server (`/api/retours…`, for the page) and the desktop app
//! (which writes here directly). The format and validation live in `argos_core::retours`.
//! One write at a time in this program (two simultaneous sends never get the same number).

use argos_core::retours::{self as rt, Billet, Nouveau};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// An error: (HTTP code, message in French).
pub type Erreur = (u16, String);

static UNE_A_LA_FOIS: Mutex<()> = Mutex::new(());

/// The tickets folder: `$ARGOS_RETOURS`, otherwise `Argos HSS Trading/retours` above `depart` (the served folder,
/// the executable), otherwise `<data>/argos-hss-trading/retours`.
pub fn dossier(depart: Option<&Path>) -> PathBuf {
    let depart = depart.map(|d| std::fs::canonicalize(d).unwrap_or_else(|_| d.to_path_buf()));
    rt::dossier_de(
        std::env::var_os("ARGOS_RETOURS"),
        depart.as_deref(),
        std::env::var_os("XDG_DATA_HOME"),
        std::env::var_os("HOME"),
    )
    .unwrap_or_else(|| PathBuf::from("retours"))
}

/// The current time, in ISO 8601 (UTC).
pub fn maintenant() -> String {
    let s = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64);
    rt::date_iso(s)
}

/// The ticket folders (name, path), from newest to oldest.
fn dossiers(d: &Path) -> Vec<(String, PathBuf)> {
    let mut v: Vec<(u32, String, PathBuf)> = std::fs::read_dir(d)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| {
            let nom = e.file_name().to_string_lossy().into_owned();
            Some((rt::numero_du_dossier(&nom)?, nom, e.path()))
        })
        .collect();
    v.sort_by_key(|x| std::cmp::Reverse(x.0));
    v.into_iter().map(|(_, n, p)| (n, p)).collect()
}

fn lire(p: &Path) -> Option<Billet> {
    serde_json::from_slice(&std::fs::read(p.join(rt::FICHIER)).ok()?).ok()
}

/// Writes a file in one go (temporary file then renamed: never half-written).
fn ecrire(p: &Path, o: &[u8]) -> Result<(), Erreur> {
    let t = p.with_extension("tmp");
    std::fs::write(&t, o)
        .and_then(|_| std::fs::rename(&t, p))
        .map_err(|e| (500, format!("écriture de {} : {e}", p.display())))
}

/// Writes `billet.json` and `note.txt` in folder `p`.
fn garder(p: &Path, b: &Billet) -> Result<(), Erreur> {
    let j = serde_json::to_vec_pretty(b).map_err(|e| (500, e.to_string()))?;
    ecrire(&p.join(rt::FICHIER), &j)?;
    ecrire(&p.join(rt::NOTE), b.note().as_bytes())
}

/// The tickets in folder `d`, newest first.
pub fn liste(d: &Path) -> Vec<Billet> {
    dossiers(d).iter().filter_map(|(_, p)| lire(p)).collect()
}

/// Creates a ticket (validated; PNG / JPEG screenshot checked on its bytes): next number, folder
/// `<numéro>-<date>-<type>/` with `billet.json`, the screenshot and `note.txt`.
pub fn creer(d: &Path, n: Nouveau) -> Result<Billet, Erreur> {
    n.valider().map_err(|m| (400, m))?;
    let image = n.capture().map_err(|m| (415, m))?;
    let _g = UNE_A_LA_FOIS.lock().unwrap_or_else(|e| e.into_inner());
    std::fs::create_dir_all(d).map_err(|e| (500, format!("dossier {} : {e}", d.display())))?;
    let noms: Vec<String> = dossiers(d).into_iter().map(|(n, _)| n).collect();
    let numero = rt::prochain_numero(noms.iter().map(String::as_str));
    let mut id = [0u8; 16];
    getrandom::fill(&mut id).map_err(|e| (500, format!("hasard du système : {e}")))?;
    let mut b = Billet::nouveau(n, rt::uuid(id), numero, maintenant()).map_err(|m| (400, m))?;
    let p = d.join(b.dossier());
    std::fs::create_dir(&p).map_err(|e| (500, format!("dossier {} : {e}", p.display())))?;
    if let Some((o, ext)) = image {
        let nom = format!("capture.{ext}");
        ecrire(&p.join(&nom), &o)?;
        b.capture = Some(nom);
    }
    garder(&p, &b)?;
    Ok(b)
}

/// Applies `f` to ticket `id` then writes it; returns the ticket.
pub fn changer(d: &Path, id: &str, f: impl FnOnce(&mut Billet) -> Result<(), String>) -> Result<Billet, Erreur> {
    let _g = UNE_A_LA_FOIS.lock().unwrap_or_else(|e| e.into_inner());
    let (p, mut b) = dossiers(d)
        .into_iter()
        .find_map(|(_, p)| lire(&p).filter(|b| b.id == id).map(|b| (p, b)))
        .ok_or((404, format!("billet inconnu : {id}")))?;
    f(&mut b).map_err(|m| (400, m))?;
    garder(&p, &b)?;
    Ok(b)
}

/// All Argos replies are seen.
pub fn tous_lus(d: &Path) -> Result<(), Erreur> {
    let _g = UNE_A_LA_FOIS.lock().unwrap_or_else(|e| e.into_inner());
    for (_, p) in dossiers(d) {
        if let Some(mut b) = lire(&p).filter(|b| b.non_lues() > 0) {
            b.marquer_lu();
            garder(&p, &b)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use argos_core::retours::Etat;

    /// 🛟 Create (consecutive numbers, screenshot checked), list, reply, state, read; a rejection creates nothing.
    #[test]
    fn billets_sur_le_disque() {
        let d = std::env::temp_dir().join(format!("argos-store-retours-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let png = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 1];
        let n = |objet: &str, image: &[u8]| Nouveau {
            objet: objet.into(),
            image: rt::base64_ecrire(image),
            ..Default::default()
        };
        let a = creer(&d, n("un", &png)).unwrap();
        let b = creer(&d, n("deux", &[])).unwrap();
        assert_eq!((a.numero, b.numero), (1, 2));
        assert_eq!(std::fs::read(d.join(a.dossier()).join("capture.png")).unwrap(), png);
        assert!(d.join(b.dossier()).join("note.txt").exists());
        assert_eq!(creer(&d, n(" ", &[])).unwrap_err().0, 400);
        assert_eq!(creer(&d, n("x", b"GIF89a")).unwrap_err().0, 415);
        assert_eq!(liste(&d).iter().map(|b| b.numero).collect::<Vec<_>>(), [2, 1]);
        let r = changer(&d, &a.id, |b| b.repondre("argos", "ok", "d")).unwrap();
        assert_eq!(r.non_lues(), 1);
        tous_lus(&d).unwrap();
        assert_eq!(liste(&d)[1].non_lues(), 0);
        let e = changer(&d, &a.id, |b| b.changer_etat(Etat::Resolu, "admin", "d")).unwrap();
        assert_eq!(e.etat, Etat::Resolu);
        assert_eq!(changer(&d, "nope", |_| Ok(())).unwrap_err().0, 404);
        let _ = std::fs::remove_dir_all(&d);
    }
}
