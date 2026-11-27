//! 📦 Version de l'appli /app/ : écrit `dist/app/version.json` et remplace `__ARGOS_VERSION__`, `__ARGOS_COMMIT__`
//! et `__ARGOS_BUILT__` dans `dist/app/index.html` et `dist/app/sw.js` (nouvelle version = nouveau cache).
//! Version = date UTC AAAAMMJJHHMM (`ARGOS_BUILD` si déjà choisie), commit = `ARGOS_COMMIT` ou `git describe`.
//! Usage : `argos-outils version-app [dossier dist]` (par défaut : `dist`).

use crate::json::{dumps, J};
use crate::Args;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

/// (année, mois, jour, heure, minute, seconde) UTC d'un instant Unix.
pub fn utc(secs: i64) -> (i64, u32, u32, u32, u32, u32) {
    let (jours, s) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    // jours depuis 1970 → date civile (algorithme de Howard Hinnant)
    let z = jours + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d, (s / 3600) as u32, (s % 3600 / 60) as u32, (s % 60) as u32)
}

pub fn maintenant() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

pub fn main(a: &Args) -> Result<(), String> {
    let dist = PathBuf::from(a.pos.first().map(String::as_str).unwrap_or("dist"));
    let (y, mo, d, h, mi, s) = utc(maintenant());
    let version = std::env::var("ARGOS_BUILD")
        .ok()
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| format!("{y:04}{mo:02}{d:02}{h:02}{mi:02}"));
    let commit = std::env::var("ARGOS_COMMIT").ok().filter(|v| !v.is_empty()).unwrap_or_else(|| {
        let parent = std::fs::canonicalize(&dist).ok().and_then(|p| p.parent().map(|p| p.to_path_buf()));
        let mut c = Command::new("git");
        c.args(["describe", "--always", "--dirty=-modifié"]);
        if let Some(p) = parent {
            c.current_dir(p);
        }
        match c.output() {
            Ok(o) => {
                let t = String::from_utf8_lossy(&o.stdout).trim().to_string();
                if t.is_empty() {
                    "?".into()
                } else {
                    t
                }
            }
            Err(_) => "?".into(),
        }
    });
    let built = format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}+00:00");
    let app = dist.join("app");
    let v = J::obj(vec![("version", J::s(&version)), ("commit", J::s(&commit)), ("built", J::s(&built))]);
    std::fs::write(app.join("version.json"), dumps(&v, false) + "\n").map_err(|e| format!("version.json : {e}"))?;
    for nom in ["index.html", "sw.js"] {
        let p = app.join(nom);
        let t = std::fs::read_to_string(&p).map_err(|e| format!("{} : {e}", p.display()))?;
        let t = t
            .replace("__ARGOS_VERSION__", &version)
            .replace("__ARGOS_COMMIT__", &commit)
            .replace("__ARGOS_BUILT__", &built);
        std::fs::write(&p, t).map_err(|e| format!("{} : {e}", p.display()))?;
    }
    println!("📦 /app/ version {version} · commit {commit}");
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn dates_utc() {
        assert_eq!(super::utc(0), (1970, 1, 1, 0, 0, 0));
        assert_eq!(super::utc(1_794_168_000), (2026, 11, 8, 20, 0, 0));
        assert_eq!(super::utc(951_782_400), (2000, 2, 29, 0, 0, 0));
    }
}
