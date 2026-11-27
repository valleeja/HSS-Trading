//! 😀 Tout le jeu Twemoji pour l'outil « Icônes et émojis » : les images sont copiées dans `emoji/` à la racine du
//! dépôt (servies à côté de l'app, chargées à la demande) ; seul un petit index entre dans l'app :
//! `crates/argos-render/src/emoji_tout.rs`.
//!
//! Usage : `argos-outils emoji-tout <dossier des .svg> [emoji-test.txt] --noms <UnicodeData.txt | unicore/Name.pl>`
//!
//! - avec emoji-test.txt (unicode.org/Public/emoji/15.0/) : les catégories officielles d'Unicode ;
//! - sans : rangement par blocs de points de code (moins fin) ;
//! - `--noms` : les noms des caractères (pour la recherche) ; par défaut le Name.pl de Perl s'il est là.
//!
//! Les variantes de teinte de peau restent servies mais ne sont pas listées dans le sélecteur.

use crate::Args;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub const GROUPES: [&str; 10] = [
    "Trading",
    "Visages",
    "Personnes",
    "Animaux et nature",
    "Nourriture",
    "Voyages et lieux",
    "Activités",
    "Objets",
    "Symboles",
    "Drapeaux",
];

fn unicode_groupe(g: &str) -> Option<u8> {
    Some(match g {
        "Smileys & Emotion" => 1,
        "People & Body" | "Component" => 2,
        "Animals & Nature" => 3,
        "Food & Drink" => 4,
        "Travel & Places" => 5,
        "Activities" => 6,
        "Objects" => 7,
        "Symbols" => 8,
        "Flags" => 9,
        _ => return None,
    })
}

fn peau(c: u32) -> bool {
    (0x1F3FB..0x1F400).contains(&c)
}

/// Catégorie d'un émoji d'après son premier point de code (quand la liste d'Unicode n'est pas donnée).
#[rustfmt::skip]
pub fn groupe_bloc(cps: &[u32]) -> u8 {
    let c = cps[0];
    let r = |a: u32, b: u32| (a..=b).contains(&c);
    let dans = |l: &[u32]| l.contains(&c);
    if r(0x1F1E6, 0x1F1FF) || dans(&[0x1F3C1, 0x1F6A9, 0x1F38C, 0x1F3F3, 0x1F3F4]) {
        return 9;
    }
    if r(0x1F600, 0x1F637) || r(0x1F641, 0x1F644) || r(0x1F910, 0x1F917) || r(0x1F920, 0x1F925) || r(0x1F927, 0x1F92F)
        || r(0x1F970, 0x1F97A) || dans(&[0x1F9D0, 0x263A, 0x2639, 0x1F4A9, 0x1F47B, 0x1F47D, 0x1F47E, 0x1F47F, 0x1F480,
                                        0x1F479, 0x1F47A, 0x1F608, 0x1F916, 0x1F921, 0x1F925, 0x1F92B, 0x1F92D])
        || r(0x1F638, 0x1F640) || r(0x1F648, 0x1F64A) || r(0x1FAE0, 0x1FAE8) || r(0x1F493, 0x1F49F) || r(0x2763, 0x2764)
        || r(0x1F5A4, 0x1F5A4) || r(0x1F90D, 0x1F90F) || r(0x1F4AF, 0x1F4AF) || r(0x1F4A2, 0x1F4A6) || r(0x1F4AB, 0x1F4AD)
        || r(0x1F48B, 0x1F48C) || dans(&[0x1FA75, 0x1FA76, 0x1FA77]) {
        return 1;
    }
    if r(0x1F442, 0x1F450) || r(0x1F466, 0x1F487) || r(0x1F574, 0x1F57A) || r(0x1F590, 0x1F596) || r(0x1F645, 0x1F647)
        || r(0x1F64B, 0x1F64F) || r(0x1F6A3, 0x1F6A3) || r(0x1F6B4, 0x1F6B6) || r(0x1F6C0, 0x1F6C0) || r(0x1F6CC, 0x1F6CC)
        || r(0x1F918, 0x1F91F) || c == 0x1F926 || r(0x1F930, 0x1F939) || r(0x1F93C, 0x1F93E) || r(0x1F9B0, 0x1F9B9)
        || r(0x1F9BB, 0x1F9BB) || r(0x1F9CD, 0x1F9DF) || r(0x1FAC3, 0x1FAC5) || r(0x1FAF0, 0x1FAF8) || dans(&[0x261D, 0x26F9])
        || r(0x270A, 0x270D) || r(0x1F440, 0x1F441) || r(0x1F463, 0x1F465) || r(0x1F48F, 0x1F491) || r(0x1F4AA, 0x1F4AA)
        || r(0x1F3C2, 0x1F3C4) || r(0x1F3C7, 0x1F3C7) || r(0x1F3CA, 0x1F3CC) || r(0x1F9BE, 0x1F9BF) || r(0x1F9E0, 0x1F9E0)
        || r(0x1FAC0, 0x1FAC2) || r(0x1F9B4, 0x1F9B7) || c == 0x1F5E3 {
        return 2;
    }
    if r(0x1F400, 0x1F43F) || r(0x1F330, 0x1F343) || r(0x1F980, 0x1F9AE) || r(0x1FAB0, 0x1FABD) || r(0x1F577, 0x1F578)
        || r(0x1F490, 0x1F490) || r(0x1F4AE, 0x1F4AE) || r(0x1F3F5, 0x1F3F5) || r(0x1F940, 0x1F940) || r(0x1FAB7, 0x1FABF)
        || r(0x1FAE9, 0x1FAEF) || dans(&[0x2618, 0x1F54A, 0x1F43F]) {
        return 3;
    }
    if r(0x1F344, 0x1F37F) || r(0x1F950, 0x1F96F) || r(0x1F9C0, 0x1F9CB) || r(0x1FAD0, 0x1FADB)
        || dans(&[0x2615, 0x1F942, 0x1F943, 0x1F944, 0x1F945, 0x1F9C3]) {
        return 4;
    }
    if r(0x1F680, 0x1F6FF) || r(0x1F3D4, 0x1F3DF) || r(0x1F3E0, 0x1F3F0) || r(0x1F300, 0x1F32F) || r(0x1F5FA, 0x1F5FF)
        || r(0x26F0, 0x26FD) || dans(&[0x2708, 0x2600, 0x2601, 0x2602, 0x2603, 0x2604, 0x2614, 0x26A1, 0x26C4, 0x26C5,
                                       0x26C8, 0x2744, 0x1F6E9, 0x1F9F1, 0x1F6D1, 0x1FA90, 0x1F3A0, 0x1F3A1, 0x1F3A2,
                                       0x1F488, 0x1F3AA]) || r(0x1F550, 0x1F567) || r(0x231A, 0x231B) || r(0x23F0, 0x23F3)
        || r(0x1F9ED, 0x1F9ED) || r(0x1F6DE, 0x1F6DF) {
        return 5;
    }
    if r(0x1F380, 0x1F393) || r(0x1F396, 0x1F3D3) || r(0x1F93A, 0x1F94F) || r(0x1FA80, 0x1FA88)
        || dans(&[0x26BD, 0x26BE, 0x26F3, 0x26F8, 0x265F, 0x2660, 0x2663, 0x2665, 0x2666, 0x1F0CF, 0x1F004, 0x1F3B4,
                  0x1F9E9, 0x1F9F8, 0x1FA81, 0x1F9FF, 0x1FAA9, 0x1F6F7, 0x1F94C]) {
        return 6;
    }
    if r(0x1F4A0, 0x1F4FF) || r(0x1F56F, 0x1F5F9) || r(0x1F9E0, 0x1F9FF) || r(0x1FA70, 0x1FA7F) || r(0x1FA90, 0x1FAAF)
        || r(0x1F50A, 0x1F53D) || dans(&[0x2328, 0x260E, 0x2702, 0x2709, 0x270F, 0x2712, 0x2692, 0x2694, 0x2696, 0x2697,
                                         0x2699, 0x26B0, 0x26B1, 0x26CF, 0x26D1, 0x26D3, 0x1F393, 0x1F3F7, 0x1F3F9, 0x1F3FA,
                                         0x1F451, 0x1F452, 0x1F453, 0x1F454, 0x1F455, 0x1F456, 0x1F457, 0x1F458, 0x1F459,
                                         0x1F45A, 0x1F45B, 0x1F45C, 0x1F45D, 0x1F45E, 0x1F45F, 0x1F460, 0x1F461, 0x1F462,
                                         0x1F484, 0x1F48D, 0x1F48E, 0x1F97B, 0x1F97C, 0x1F97D, 0x1F97E, 0x1F97F, 0x1FA70,
                                         0x1F9AF, 0x1F9BA, 0x1F6CD, 0x1F6D2]) {
        return 7;
    }
    8
}

/// Noms des caractères : `UnicodeData.txt` (code;NOM;…) ou le `unicore/Name.pl` de Perl (code, puis nom).
pub fn lire_noms(t: &str) -> HashMap<u32, String> {
    let mut m = HashMap::new();
    if let Some(corps) = t.split_once("<<'END';\n").map(|x| x.1) {
        let corps = corps.split("\nEND\n").next().unwrap_or("");
        for bloc in corps.split("\n\n") {
            let mut l = bloc.trim().lines();
            let (Some(code), Some(nom)) = (l.next(), l.next()) else { continue };
            if code.contains(' ') {
                continue; // séquence nommée
            }
            if let Ok(c) = u32::from_str_radix(code, 16) {
                m.entry(c).or_insert_with(|| nom.to_string());
            }
        }
    } else {
        for l in t.lines() {
            let mut p = l.split(';');
            let (Some(code), Some(nom)) = (p.next(), p.next()) else { continue };
            if nom.starts_with('<') {
                continue;
            }
            if let Ok(c) = u32::from_str_radix(code, 16) {
                m.insert(c, nom.to_string());
            }
        }
    }
    m
}

fn nom(cps: &[u32], noms: &HashMap<u32, String>) -> String {
    cps.iter()
        .filter(|&&c| !(c == 0x200D || c == 0xFE0F || c == 0x20E3 || (0xE0020..=0xE007F).contains(&c)))
        .filter_map(|c| noms.get(c).map(|n| n.to_lowercase()))
        .collect::<Vec<_>>()
        .join(" ")
}

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Catégories officielles (emoji-test.txt) : nom sans FE0F → (groupe, rang dans la liste, nom court CLDR).
pub fn officiel(t: &str) -> HashMap<String, (u8, usize, String)> {
    let mut m = HashMap::new();
    let mut g = None;
    for (rang, ligne) in t.lines().enumerate() {
        if let Some(x) = ligne.strip_prefix("# group:") {
            g = unicode_groupe(x.trim());
        } else if !ligne.is_empty() && !ligne.starts_with('#') && ligne.contains(';') {
            let Some(g) = g else { continue };
            let cps: Vec<u32> = ligne
                .split(';')
                .next()
                .unwrap_or("")
                .split_whitespace()
                .filter_map(|x| u32::from_str_radix(x, 16).ok())
                .collect();
            let k = cps.iter().filter(|&&c| c != 0xFE0F).map(|c| format!("{c:x}")).collect::<Vec<_>>().join("-");
            // le nom court : après « # <émoji> E15.0 »
            let court = ligne
                .split_once('#')
                .map(|(_, d)| d.splitn(4, ' ').last().unwrap_or("").trim().to_string())
                .unwrap_or_default();
            m.entry(k).or_insert((g, rang, court));
        }
    }
    m
}

pub fn main(a: &Args) -> Result<(), String> {
    let src = PathBuf::from(a.pos.first().ok_or("il manque le dossier des .svg")?);
    let test = a.pos.get(1).map(PathBuf::from);
    let noms_f = a.noms.clone().or_else(|| {
        let p = Path::new("/usr/share/perl");
        let mut v: Vec<PathBuf> = std::fs::read_dir(p)
            .ok()?
            .flatten()
            .map(|e| e.path().join("unicore/Name.pl"))
            .filter(|p| p.exists())
            .collect();
        v.sort();
        v.pop()
    });
    let noms = match &noms_f {
        Some(f) => lire_noms(&std::fs::read_to_string(f).map_err(|e| format!("{} : {e}", f.display()))?),
        None => return Err("il manque --noms <UnicodeData.txt> (noms des caractères pour la recherche)".into()),
    };
    let root = &a.racine;
    let repo = root.join("..");
    let out = repo.join("emoji");
    let table = root.join("crates/argos-render/src/emoji_tout.rs");
    let off = match &test {
        Some(t) => officiel(&std::fs::read_to_string(t).map_err(|e| format!("{} : {e}", t.display()))?),
        None => HashMap::new(),
    };
    let mut fichiers: Vec<String> = std::fs::read_dir(&src)
        .map_err(|e| format!("{} : {e}", src.display()))?
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter(|n| n.ends_with(".svg"))
        .collect();
    fichiers.sort();
    if out.exists() {
        std::fs::remove_dir_all(&out).map_err(|e| e.to_string())?;
    }
    std::fs::create_dir(&out).map_err(|e| e.to_string())?;
    let (mut lignes, mut variantes) = (vec![], vec![]);
    for f in &fichiers {
        std::fs::copy(src.join(f), out.join(f)).map_err(|e| e.to_string())?;
        let k = f.trim_end_matches(".svg").to_string();
        let cps: Vec<u32> = k.split('-').filter_map(|x| u32::from_str_radix(x, 16).ok()).collect();
        if cps.iter().any(|&c| peau(c)) {
            variantes.push(k);
            continue;
        }
        let mut texte: String = cps.iter().filter_map(|&c| char::from_u32(c)).collect();
        if cps.len() == 1 && cps[0] < 0x1F000 {
            texte.push('\u{FE0F}');
        }
        let cle = cps.iter().filter(|&&c| c != 0xFE0F).map(|c| format!("{c:x}")).collect::<Vec<_>>().join("-");
        let (gr, rang, court) = off.get(&cle).cloned().unwrap_or((groupe_bloc(&cps), 1_000_000_000, String::new()));
        let recherche = format!("{} {}", court, nom(&cps, &noms)).trim().to_lowercase();
        lignes.push((gr, rang, k, texte, recherche));
    }
    std::fs::write(
        out.join("LICENSE.txt"),
        "Images Twemoji 15.0.0 (@twemoji/svg), Copyright 2020 Twitter, Inc and other contributors\n\
         (https://github.com/jdecked/twemoji), licence CC-BY 4.0 : https://creativecommons.org/licenses/by/4.0/\n\
         Servies telles quelles à Argos (outil « Icônes et émojis »), chargées à la demande.\n",
    )
    .map_err(|e| e.to_string())?;
    lignes.sort_by(|x, y| (x.0, x.1, &x.2).cmp(&(y.0, y.1, &y.2)));
    variantes.sort();
    let source = if test.is_some() {
        "liste Unicode emoji-test.txt"
    } else {
        "blocs de points de code"
    };
    let rows: Vec<String> = lignes
        .iter()
        .map(|(g, _, k, t, n)| format!("    ({g}, \"{k}\", \"{}\", \"{}\"),", esc(t), esc(n)))
        .collect();
    let groupes: Vec<String> = GROUPES.iter().map(|g| format!("\"{g}\"")).collect();
    let vars: Vec<String> = variantes.iter().map(|k| format!("    \"{k}\",")).collect();
    std::fs::write(
        &table,
        format!(
            "//! Index généré par argos-outils emoji-tout (ne pas modifier à la main) : tout Twemoji 15.0.0, rangé par\n\
             //! catégorie ({source}). (catégorie, nom Twemoji = fichier `/emoji/<nom>.svg`, émoji, nom anglais).\n\n\
             pub const GROUPES: [&str; {}] = [{}];\n\n\
             pub static TOUT: [(u8, &str, &str, &str); {}] = [\n{}\n];\n\n\
             /// Variantes de teinte de peau : servies (texte), pas listées dans le sélecteur.\n\
             pub static VARIANTES: [&str; {}] = [\n{}\n];\n",
            GROUPES.len(),
            groupes.join(", "),
            lignes.len(),
            rows.join("\n"),
            variantes.len(),
            vars.join("\n")
        ),
    )
    .map_err(|e| e.to_string())?;
    println!("{} images dans emoji, {} dans le sélecteur ({source})", fichiers.len(), lignes.len());
    Ok(())
}
