//! 📐 Mesures de la maquette (`ecrans/maquette-<vue>.json`, sorties de `tools/maquette-mesure.js`) → table Rust
//! (`ecrans/maquette_<vue>.rs`). Usage : `argos-outils maquette cell [--racine rust/]`.

use crate::json::{dumps, py_float, J};
use crate::Args;
use serde_json::Value;

fn s(v: &Value) -> String {
    dumps(&J::from(v), false)
}

fn f(v: &Value) -> String {
    py_float(v.as_f64().unwrap_or(f64::NAN))
}

fn opt(v: Option<&Value>) -> String {
    match v {
        None | Some(Value::Null) => "None".into(),
        Some(x) => format!("Some({})", s(x)),
    }
}

pub fn zone(z: &Value, ind: usize) -> String {
    let p = " ".repeat(ind);
    let enfants: String = z
        .get("enfants")
        .and_then(Value::as_array)
        .map(|e| e.iter().map(|x| zone(x, ind + 8)).collect())
        .unwrap_or_default();
    let boite: Vec<String> = z["boite"].as_array().map(|b| b.iter().map(f).collect()).unwrap_or_default();
    format!(
        "{p}Zone {{\n{p}    id: {},\n{p}    boite: [{}],\n{p}    rayon: {},\n{p}    fond: {},\n{p}    bord: {},\n{p}    texte: {},\n{p}    taille: {},\n{p}    libelle: {},\n{p}    enfants: &[\n{enfants}{p}    ],\n{p}}},\n",
        s(&z["id"]),
        boite.join(", "),
        f(&z["rayon"]),
        s(&z["fond"]),
        opt(z.get("bord")),
        s(&z["texte"]),
        f(&z["taille"]),
        s(&z["libelle"]),
    )
}

pub fn table(vue: &str, d: &Value) -> String {
    let zones: String = d["zones"].as_array().map(|z| z.iter().map(|x| zone(x, 8)).collect()).unwrap_or_default();
    format!(
        "//! Table générée par argos-outils maquette depuis maquette-{vue}.json (mesures de la maquette) :\n\
         //! ne pas modifier à la main.\n\n\
         use super::{{Maquette, Zone}};\n\n\
         pub const MAQUETTE: Maquette = Maquette {{\n    vue: {},\n    zones: &[\n{zones}    ],\n}};\n",
        s(&d["vue"])
    )
}

pub fn main(a: &Args) -> Result<(), String> {
    let vue = a.pos.first().map(String::as_str).unwrap_or("cell");
    let ici = a.racine.join("crates/argos-render/src/ecrans");
    let src = ici.join(format!("maquette-{vue}.json"));
    let t = std::fs::read_to_string(&src).map_err(|e| format!("{} : {e}", src.display()))?;
    let d: Value = serde_json::from_str(&t).map_err(|e| format!("{} : {e}", src.display()))?;
    std::fs::write(ici.join(format!("maquette_{vue}.rs")), table(vue, &d)).map_err(|e| e.to_string())?;
    println!("{vue} : {} blocs", d["zones"].as_array().map_or(0, Vec::len));
    Ok(())
}
