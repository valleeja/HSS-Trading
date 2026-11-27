//! ⚙️ Génère `crates/argos-script/src/v1_inputs.rs` : les tables d'inputs des indicateurs d'Argos v1 (`../app.js`),
//! évaluées par `node` avec les mêmes aides que v1 (P, SEL…), y compris les inputs de tableau que v1 ajoute
//! (nvTblParams). Une seule source : le code de v1. Usage : `argos-outils v1-inputs [--racine rust/]` (node requis).

use crate::json::{dumps, py_float, py_str, J};
use crate::Args;
use serde_json::Value;
use std::io::Write;
use std::process::{Command, Stdio};

/// nom Rust → début (texte exact) de la définition dans app.js (la dernière trouvée gagne, comme dans v1)
const SPECS: [(&str, &str); 15] = [
    ("TVD", "\n      tvd: {"),
    ("EMA_MULTI", "\n      emas: {"),
    ("CIPHER", "\n      cipher: {"),
    ("AI_TREND", "NV.DEF.aitrend = {"),
    ("TBO", "NV.DEF.tbo = {"),
    ("SMC", "NV.DEF.smc = {"),
    ("HEATMAP", "NV.DEF.hmv = {"),
    ("PROFILE", "NV.DEF.vprof = {"),
    ("BANDS", "NV.DEF.bb = {"),
    ("BANDS_OSC", "NV.DEF.bbo = {"),
    ("SMC_PRO", "NV.DEF.smcu = {"),
    ("FOOTPRINT", "NV.DEF.fpe = {"),
    ("BUBBLES", "NV.DEF.bub = {"),
    ("TAPE", "NV.DEF.tape = {"),
    ("PRICE_ACTION", "NV.DEF.pa = {"),
];

/// Texte de l'objet / tableau qui commence à `s[start]` (accolades équilibrées, chaînes ignorées).
pub fn balanced(s: &str, start: usize) -> Option<&str> {
    let b = s.as_bytes();
    let (mut i, mut depth, mut q) = (start, 0i32, None::<u8>);
    while i < b.len() {
        let c = b[i];
        if let Some(qq) = q {
            if c == b'\\' {
                i += 2;
                continue;
            }
            if c == qq {
                q = None;
            }
        } else if matches!(c, b'"' | b'\'' | b'`') {
            q = Some(c);
        } else if matches!(c, b'{' | b'[' | b'(') {
            depth += 1;
        } else if matches!(c, b'}' | b']' | b')') {
            depth -= 1;
            if depth == 0 {
                return s.get(start..=i);
            }
        }
        i += 1;
    }
    None
}

fn mot(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn blancs(s: &str, i: usize) -> usize {
    i + s[i..].len() - s[i..].trim_start().len()
}

/// `\bname:\s*` : la première occurrence, puis l'objet ou le tableau qui suit.
pub fn field<'a>(obj: &'a str, name: &str) -> Option<&'a str> {
    let motif = format!("{name}:");
    let mut depuis = 0;
    while let Some(p) = obj[depuis..].find(&motif) {
        let p = depuis + p;
        if obj[..p].chars().next_back().is_none_or(|c| !mot(c)) {
            let e = blancs(obj, p + motif.len());
            return match obj.as_bytes().get(e) {
                Some(b'[') | Some(b'{') => balanced(obj, e),
                _ => None,
            };
        }
        depuis = p + obj[p..].chars().next().map_or(1, char::len_utf8);
    }
    None
}

/// `(?:const|,)\s*K\s*=\s*` : la première occurrence ; rend l'indice après le `=` et les blancs.
fn constante(app: &str, k: &str) -> Option<usize> {
    let b = app.as_bytes();
    let mut i = 0;
    while i < b.len() {
        let apres = if b[i..].starts_with(b"const") {
            Some(i + 5)
        } else if b[i] == b',' {
            Some(i + 1)
        } else {
            None
        };
        if let Some(j) = apres {
            let j = blancs(app, j);
            if app[j..].starts_with(k) {
                let j = blancs(app, j + k.len());
                if b.get(j) == Some(&b'=') {
                    return Some(blancs(app, j + 1));
                }
            }
        }
        i += 1;
    }
    None
}

/// `motif:\s*"(\w+)"` : le mot entre guillemets de la première occurrence.
fn chaine_apres(obj: &str, motif: &str) -> Option<String> {
    let mut depuis = 0;
    while let Some(p) = obj[depuis..].find(motif) {
        let e = blancs(obj, depuis + p + motif.len());
        if obj[e..].starts_with('"') {
            let fin = obj[e + 1..].find(|c: char| !mot(c)).map(|f| e + 1 + f)?;
            if fin > e + 1 && obj[fin..].starts_with('"') {
                return Some(obj[e + 1..fin].to_string());
            }
        }
        depuis += p + obj[depuis + p..].chars().next().map_or(1, char::len_utf8);
    }
    None
}

fn consts(app: &str) -> Result<String, String> {
    let mut out = vec![];
    for k in ["SRCO", "TFSEC", "TFO", "HM5", "TFO4"] {
        let i = constante(app, k).ok_or(format!("constante {k} introuvable dans app.js"))?;
        out.push(format!("var {k} = {};", balanced(app, i).ok_or(format!("{k} : fin introuvable"))?));
    }
    out.push("var TFOPT = Object.keys(TFSEC).map(k => [k, k]);".into());
    Ok(out.join("\n"))
}

const EVAL: &str = include_str!("v1_inputs_eval.js");

fn num(v: f64) -> String {
    if v == v.trunc() && v.is_finite() {
        format!("{v:.1}")
    } else {
        py_float(v)
    }
}

/// `str(x)` de Python pour une valeur JSON simple.
fn py_texte(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Bool(b) => if *b { "True" } else { "False" }.into(),
        Value::Null => "None".into(),
        Value::Number(n) => match n.as_i64() {
            Some(i) => i.to_string(),
            None => py_float(n.as_f64().unwrap_or(f64::NAN)),
        },
        x => dumps(&J::from(x), false),
    }
}

fn verite(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64() != Some(0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
    }
}

fn flottant(v: &Value) -> f64 {
    match v {
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Value::String(s) => s.trim().parse().unwrap_or(f64::NAN),
        Value::Bool(b) => f64::from(u8::from(*b)),
        _ => f64::NAN,
    }
}

pub fn main(a: &Args) -> Result<(), String> {
    let root = &a.racine;
    let app_p = root.join("../app.js");
    let app = std::fs::read_to_string(&app_p).map_err(|e| format!("{} : {e}", app_p.display()))?;
    let mut raw = vec![];
    for (nom, motif) in SPECS {
        let m = app.rfind(motif).ok_or(format!("{nom} introuvable dans app.js"))?;
        let ouv = m + 1 + app[m + 1..].find('{').ok_or("accolade introuvable")?;
        let obj = balanced(&app, ouv).ok_or(format!("{nom} : fin introuvable"))?;
        let s = |x: Option<&str>| x.map_or(J::Null, J::s);
        raw.push((
            nom.to_string(),
            J::obj(vec![
                ("groups", s(field(obj, "groups"))),
                ("params", s(field(obj, "params"))),
                ("pane", chaine_apres(obj, "pane:").map_or(J::Null, J::Str)),
                ("tblDef", chaine_apres(obj, "tblDef:").map_or(J::Null, J::Str)),
            ]),
        ));
    }
    let js = EVAL.replace("__CONSTS__", &consts(&app)?);
    let mut node = Command::new("node")
        .args(["-e", &js])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| format!("node : {e}"))?;
    node.stdin
        .take()
        .ok_or("node : entrée")?
        .write_all(dumps(&J::Obj(raw), true).as_bytes())
        .map_err(|e| e.to_string())?;
    let o = node.wait_with_output().map_err(|e| e.to_string())?;
    if !o.status.success() {
        return Err(format!("node a échoué ({})", o.status));
    }
    let res: Value = serde_json::from_slice(&o.stdout).map_err(|e| format!("sortie de node : {e}"))?;
    let rs = |s: &str| py_str(s, false);
    let mut body = vec![
        "//! ⚙️ Inputs des indicateurs d'Argos v1, générés depuis app.js par argos-outils v1-inputs (ne pas modifier à la main).".to_string(),
        "//! Mêmes clés, groupes, noms, défauts, min, max et pas que v1. Une liste garde l'index de l'option ; une couleur, 0xRRGGBB.".into(),
        String::new(),
        "use crate::inputs::{check, choice, color, num, Param};".into(),
        String::new(),
    ];
    let mut resume = vec![];
    for (nom, _) in SPECS {
        let d = &res[nom];
        let groups = d["groups"].as_array().cloned().unwrap_or_default();
        let gid: Vec<&Value> = groups.iter().map(|g| &g[0]).collect();
        let labels: Vec<String> = groups.iter().map(|g| format!("{} {}", py_texte(&g[1]), py_texte(&g[2]))).collect();
        body.push(format!(
            "pub const {nom}_GROUPS: [&str; {}] = [{}];",
            labels.len(),
            labels.iter().map(|x| rs(x)).collect::<Vec<_>>().join(", ")
        ));
        let params = d["params"].as_array().cloned().unwrap_or_default();
        let mut rows = vec![];
        for p in &params {
            let g = gid.iter().position(|x| **x == p["g"]).ok_or(format!("{nom} : groupe inconnu"))?;
            let (key, lab) = (rs(&py_texte(&p["k"])), rs(&py_texte(&p["label"])));
            match p["type"].as_str().unwrap_or("") {
                "bool" => rows.push(format!("    check({key}, {g}, {lab}, {}),", verite(&p["def"]))),
                "col" => rows.push(format!(
                    "    color({key}, {g}, {lab}, 0x{}),",
                    py_texte(&p["def"]).trim_start_matches('#').to_lowercase()
                )),
                "sel" => {
                    let opts = p["opts"].as_array().cloned().unwrap_or_default();
                    let keys: Vec<String> = opts.iter().map(|o| py_texte(&o[0])).collect();
                    let i = keys
                        .iter()
                        .position(|k| *k == py_texte(&p["def"]))
                        .ok_or(format!("{nom} : défaut absent des options"))?;
                    let o: Vec<String> = opts.iter().map(|o| rs(&py_texte(&o[1]))).collect();
                    rows.push(format!("    choice({key}, {g}, {lab}, {i}, &[{}]),", o.join(", ")));
                }
                _ => {
                    let d0 = flottant(&p["def"]);
                    let step = match p.get("step") {
                        Some(s) if verite(s) => flottant(s),
                        _ => {
                            if d0 == d0.trunc() {
                                1.0
                            } else {
                                0.01
                            }
                        }
                    };
                    let mn = p.get("min").map_or(-1e6, flottant);
                    let mx = p.get("max").map_or(1e6, flottant);
                    rows.push(format!("    num({key}, {g}, {lab}, {}, {}, {}, {}),", num(d0), num(mn), num(mx), num(step)));
                }
            }
        }
        body.push(format!("pub const {nom}_PARAMS: [Param; {}] = [", rows.len()));
        resume.push(format!("{nom} {}", rows.len()));
        body.extend(rows);
        body.push("];".into());
        body.push(String::new());
    }
    std::fs::write(root.join("crates/argos-script/src/v1_inputs.rs"), body.join("\n")).map_err(|e| e.to_string())?;
    println!("crates/argos-script/src/v1_inputs.rs : {}", resume.join(", "));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lecture_du_js() {
        let s = r#"x = { a: "}{", b: [1, (2)], groups: [["g", "⚙", "Réglages"]] } fin"#;
        let o = s.find('{').unwrap();
        let obj = balanced(s, o).unwrap();
        assert!(obj.ends_with("]] }"));
        assert_eq!(field(obj, "groups"), Some(r#"[["g", "⚙", "Réglages"]]"#));
        assert_eq!(field("xgroups: [1]", "groups"), None);
        assert_eq!(chaine_apres(r#"{ pane:  "sub", x }"#, "pane:"), Some("sub".into()));
        assert_eq!(constante("let a, TFO = {x}", "TFO"), Some(13), "juste après « = »");
        assert_eq!(num(5.0), "5.0");
        assert_eq!(num(0.25), "0.25");
    }
}
