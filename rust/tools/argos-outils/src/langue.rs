//! 🌐 Packs de langue.
//!
//! `argos-outils langue modele`   → écrit `../lang/_modele.json` (phrase française → anglais, à traduire)
//! `argos-outils langue verifier` → pour chaque pack `../lang/<code>.json` : phrases manquantes ou en trop
//!
//! Les phrases viennent du dictionnaire `DICT` de `crates/argos-render/src/i18n.rs` (français, English, Español) ;
//! `--racine` = le dossier `rust/`.

use crate::json::py_str;
use crate::Args;

/// Une chaîne Rust entre guillemets à partir de `s[i]` (qui vaut `"`) : (contenu brut, indice après le guillemet).
fn chaine(s: &[u8], i: usize) -> Option<(usize, usize)> {
    if s.get(i) != Some(&b'"') {
        return None;
    }
    let mut j = i + 1;
    while j < s.len() {
        match s[j] {
            b'\\' => j += 2,
            b'"' => return Some((i + 1, j)),
            _ => j += 1,
        }
    }
    None
}

fn blancs(s: &[u8], mut i: usize) -> usize {
    while i < s.len() && s[i].is_ascii_whitespace() {
        i += 1;
    }
    i
}

/// Les triplets `( "fr" , "en" , "es" )` du texte, dans l'ordre (comme `re.findall`).
pub fn triplets(src: &str) -> Vec<(String, String)> {
    let s = src.as_bytes();
    let mut out = vec![];
    let mut i = 0;
    while i < s.len() {
        if s[i] == b'(' {
            let essai = (|| {
                let mut j = blancs(s, i + 1);
                let mut v = vec![];
                for k in 0..3 {
                    let (a, b) = chaine(s, j)?;
                    v.push((a, b));
                    j = blancs(s, b + 1);
                    let attendu = if k < 2 { b',' } else { b')' };
                    if s.get(j) != Some(&attendu) {
                        return None;
                    }
                    j = if k < 2 { blancs(s, j + 1) } else { j + 1 };
                }
                Some((v, j))
            })();
            if let Some((v, fin)) = essai {
                out.push((src[v[0].0..v[0].1].to_string(), src[v[1].0..v[1].1].to_string()));
                i = fin;
                continue;
            }
        }
        i += 1;
    }
    out
}

/// Les échappements d'une chaîne (comme `unicode_escape` de Python sur les octets UTF-8).
pub fn desechapper(s: &str) -> String {
    let b = s.as_bytes();
    let mut o: Vec<u8> = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'\\' && i + 1 < b.len() {
            let c = b[i + 1];
            let r = match c {
                b'\\' => Some(b'\\'),
                b'\'' => Some(b'\''),
                b'"' => Some(b'"'),
                b'n' => Some(b'\n'),
                b't' => Some(b'\t'),
                b'r' => Some(b'\r'),
                b'a' => Some(7),
                b'b' => Some(8),
                b'f' => Some(12),
                b'v' => Some(11),
                b'0' => Some(0),
                _ => None,
            };
            if let Some(r) = r {
                o.push(r);
                i += 2;
                continue;
            }
        }
        o.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&o).into_owned()
}

/// Les phrases : français (sans échappements) → anglais (tel quel dans le code), ordre du dictionnaire ; une phrase
/// en double garde sa première place et sa dernière traduction (comme un dict Python).
pub fn phrases(src: &str) -> Vec<(String, String)> {
    let corps = &src[src.find("const DICT").unwrap_or(0)..];
    let mut out: Vec<(String, String)> = vec![];
    let mut place = std::collections::HashMap::new();
    for (fr, en) in triplets(corps) {
        let k = desechapper(&fr);
        match place.get(&k) {
            Some(&i) => out[i] = (k, en),
            None => {
                place.insert(k.clone(), out.len());
                out.push((k, en));
            }
        }
    }
    out
}

/// `json.dump(…, ensure_ascii=False, indent=1)` d'un objet de chaînes.
pub fn json_indent1(v: &[(String, String)]) -> String {
    if v.is_empty() {
        return "{}".into();
    }
    let lignes: Vec<String> = v.iter().map(|(k, x)| format!(" {}: {}", py_str(k, false), py_str(x, false))).collect();
    format!("{{\n{}\n}}", lignes.join(",\n"))
}

pub fn main(a: &Args) -> Result<(), String> {
    let i18n = a.racine.join("crates/argos-render/src/i18n.rs");
    let lang = a.racine.join("..").join("lang");
    let src = std::fs::read_to_string(&i18n).map_err(|e| format!("{} : {e}", i18n.display()))?;
    let reference = phrases(&src);
    match a.pos.first().map(String::as_str).unwrap_or("verifier") {
        "modele" => {
            std::fs::create_dir_all(&lang).map_err(|e| e.to_string())?;
            std::fs::write(lang.join("_modele.json"), json_indent1(&reference)).map_err(|e| e.to_string())?;
            println!("{} phrases dans lang/_modele.json", reference.len());
        }
        _ => {
            let mut noms: Vec<String> = std::fs::read_dir(&lang)
                .map_err(|e| format!("{} : {e}", lang.display()))?
                .filter_map(|e| e.ok()?.file_name().into_string().ok())
                .filter(|n| n.ends_with(".json") && !n.starts_with('_'))
                .collect();
            noms.sort();
            for nom in noms {
                let t = std::fs::read_to_string(lang.join(&nom)).map_err(|e| format!("{nom} : {e}"))?;
                let pack: serde_json::Map<String, serde_json::Value> = serde_json::from_str(&t).map_err(|e| format!("{nom} : {e}"))?;
                let vrai = |v: &serde_json::Value| match v {
                    serde_json::Value::Null => false,
                    serde_json::Value::Bool(b) => *b,
                    serde_json::Value::String(s) => !s.is_empty(),
                    serde_json::Value::Array(x) => !x.is_empty(),
                    serde_json::Value::Object(x) => !x.is_empty(),
                    serde_json::Value::Number(n) => n.as_f64() != Some(0.0),
                };
                let manque: Vec<&String> = reference.iter().map(|(k, _)| k).filter(|k| !pack.get(*k).is_some_and(vrai)).collect();
                let en_trop = pack.keys().filter(|k| !reference.iter().any(|(r, _)| r == *k)).count();
                let trop = if en_trop > 0 { format!(", {en_trop} en trop") } else { String::new() };
                println!("{nom}: {}/{} phrases{trop}", reference.len() - manque.len(), reference.len());
                for k in manque.iter().take(10) {
                    println!("   manque : {k}");
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dictionnaire() {
        let src = r#"x ("a", "b", "c") const DICT: &[(&str, &str, &str)] = &[
            ("Ouvrir \"x\"", "Open \"x\"", "Abrir"),
            ( "Fermer" , "Close" , "Cerrar" ),
            ("Ouvrir \"x\"", "Open again", "Abrir"),
        ];"#;
        let p = phrases(src);
        assert_eq!(p, [("Ouvrir \"x\"".into(), "Open again".into()), ("Fermer".into(), "Close".into())]);
        assert_eq!(json_indent1(&p), "{\n \"Ouvrir \\\"x\\\"\": \"Open again\",\n \"Fermer\": \"Close\"\n}");
    }
}
