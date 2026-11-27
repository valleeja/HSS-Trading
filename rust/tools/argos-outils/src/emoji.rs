//! 😀 Copie les images Twemoji (CC-BY 4.0) des émojis écrits dans le code (et les leçons de l'Académie) vers
//! `crates/argos-render/assets/twemoji/` (seulement ceux utilisés : l'app reste légère) et écrit la table
//! `crates/argos-render/src/twemoji.rs`. Usage : `argos-outils emoji <dossier svg de @twemoji/svg> [--racine rust/]`.

use crate::Args;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Caractère d'émoji (même liste que l'ancien script).
pub fn est_emoji(c: char) -> bool {
    let u = c as u32;
    matches!(u,
        0x1F000..=0x1FAFF | 0x2190..=0x21FF | 0x2300..=0x23FF | 0x25A0..=0x27BF | 0x2900..=0x297F | 0x2B00..=0x2BFF
        | 0x3030 | 0x303D | 0x3297 | 0x3299 | 0xA9 | 0xAE | 0x203C | 0x2049 | 0x2122 | 0x2139)
}

fn regional(c: char) -> bool {
    (0x1F1E6..=0x1F1FF).contains(&(c as u32))
}

fn peau(c: char) -> bool {
    (0x1F3FB..=0x1F3FF).contains(&(c as u32))
}

/// Les grappes d'émojis d'un texte, de gauche à droite (drapeaux, émoji + sélecteur / teinte / touche, séquences
/// ZWJ, touches 0-9 # *).
pub fn grappes(t: &str) -> Vec<String> {
    let c: Vec<char> = t.chars().collect();
    let mut out = vec![];
    let mut i = 0;
    while i < c.len() {
        // drapeau : deux lettres régionales
        if i + 1 < c.len() && regional(c[i]) && regional(c[i + 1]) {
            out.push(c[i..i + 2].iter().collect());
            i += 2;
            continue;
        }
        if est_emoji(c[i]) {
            let mut j = i + 1;
            while j < c.len() && (c[j] == '\u{FE0F}' || peau(c[j]) || c[j] == '\u{20E3}') {
                j += 1;
            }
            // séquences ZWJ : ZWJ + émoji (+ sélecteur / teinte)
            while j + 1 < c.len() && c[j] == '\u{200D}' && est_emoji(c[j + 1]) {
                j += 2;
                while j < c.len() && (c[j] == '\u{FE0F}' || peau(c[j])) {
                    j += 1;
                }
            }
            out.push(c[i..j].iter().collect());
            i = j;
            continue;
        }
        if matches!(c[i], '0'..='9' | '#' | '*') {
            let mut j = i + 1;
            if j < c.len() && c[j] == '\u{FE0F}' {
                j += 1;
            }
            if j < c.len() && c[j] == '\u{20E3}' {
                out.push(c[i..=j].iter().collect());
                i = j + 1;
                continue;
            }
        }
        i += 1;
    }
    out
}

/// Nom de fichier Twemoji : points de code en hexa, sans FE0F hors des séquences ZWJ.
pub fn nom(seq: &str) -> String {
    let zwj = seq.contains('\u{200D}');
    seq.chars()
        .filter(|c| zwj || *c != '\u{FE0F}')
        .map(|c| format!("{:x}", c as u32))
        .collect::<Vec<_>>()
        .join("-")
}

fn brut(seq: &str) -> String {
    seq.chars().map(|c| format!("{:x}", c as u32)).collect::<Vec<_>>().join("-")
}

/// Tous les fichiers d'un dossier (récursif) qui finissent par `ext`.
pub fn fichiers(d: &Path, ext: &str, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(d) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            fichiers(&p, ext, out);
        } else if p.to_string_lossy().ends_with(ext) {
            out.push(p);
        }
    }
}

pub fn main(a: &Args) -> Result<(), String> {
    let src = PathBuf::from(a.pos.first().ok_or("il manque le dossier des .svg de @twemoji/svg")?);
    let root = &a.racine;
    let out = root.join("crates/argos-render/assets/twemoji");
    let mut fs_ = vec![];
    fichiers(&root.join("crates"), ".rs", &mut fs_);
    fichiers(&root.join("crates/argos-academy/contenu"), ".md", &mut fs_);
    let mut trouves = BTreeSet::new();
    for f in fs_ {
        let t = std::fs::read_to_string(&f).map_err(|e| format!("{} : {e}", f.display()))?;
        trouves.extend(grappes(&t));
    }
    std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
    for e in std::fs::read_dir(&out).map_err(|e| e.to_string())?.flatten() {
        if e.path().extension().is_some_and(|x| x == "svg") {
            std::fs::remove_file(e.path()).map_err(|e| e.to_string())?;
        }
    }
    let mut gardes = vec![];
    for seq in &trouves {
        for cand in [nom(seq), brut(seq)] {
            let f = src.join(format!("{cand}.svg"));
            if f.exists() {
                std::fs::copy(&f, out.join(format!("{cand}.svg"))).map_err(|e| e.to_string())?;
                gardes.push(cand);
                break;
            }
        }
    }
    std::fs::write(
        out.join("LICENSE.txt"),
        "Images Twemoji, Copyright 2020 Twitter, Inc and other contributors (https://github.com/jdecked/twemoji),\n\
         licence CC-BY 4.0 : https://creativecommons.org/licenses/by/4.0/\n",
    )
    .map_err(|e| e.to_string())?;
    gardes.sort();
    let rows: Vec<String> = gardes
        .iter()
        .map(|k| format!("    (\"{k}\", include_bytes!(\"../assets/twemoji/{k}.svg\")),"))
        .collect();
    std::fs::write(
        root.join("crates/argos-render/src/twemoji.rs"),
        format!(
            "//! Table générée par argos-outils emoji : nom Twemoji → image SVG (ne pas modifier à la main).\n\n\
             pub const SVG: [(&str, &[u8]); {}] = [\n{}\n];\n",
            gardes.len(),
            rows.join("\n")
        ),
    )
    .map_err(|e| e.to_string())?;
    println!("{} émojis copiés dans crates/argos-render/assets/twemoji", gardes.len());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grappes_et_noms() {
        let g = grappes("a 🇫🇷 b ❤️ 👍🏽 👨‍👩‍👧 1️⃣ 7 ©");
        assert_eq!(g, ["🇫🇷", "❤️", "👍🏽", "👨‍👩‍👧", "1️⃣", "©"]);
        assert_eq!(nom("❤️"), "2764");
        assert_eq!(nom("👁️‍🗨️"), "1f441-fe0f-200d-1f5e8-fe0f");
        assert_eq!(nom("1️⃣"), "31-20e3");
    }
}
