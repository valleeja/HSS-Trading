//! JSON écrit exactement comme `json.dumps` de Python (séparateurs « , » et « : », ordre des clés gardé, nombres
//! à virgule comme `repr`) : les fichiers générés restent identiques à ceux des anciens scripts.

use std::fmt::Write;

/// Une valeur JSON dont les objets gardent l'ordre des clés.
#[derive(Debug, Clone, PartialEq)]
pub enum J {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    Arr(Vec<J>),
    Obj(Vec<(String, J)>),
}

impl J {
    pub fn s(v: impl Into<String>) -> J {
        J::Str(v.into())
    }
    pub fn obj(v: Vec<(&str, J)>) -> J {
        J::Obj(v.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
    }
}

/// Depuis serde_json (les objets y sont triés par clé : à n'utiliser que là où l'ordre ne compte pas).
impl From<&serde_json::Value> for J {
    fn from(v: &serde_json::Value) -> J {
        use serde_json::Value as V;
        match v {
            V::Null => J::Null,
            V::Bool(b) => J::Bool(*b),
            V::Number(n) => match n.as_i64() {
                Some(i) => J::Int(i),
                None => J::Float(n.as_f64().unwrap_or(f64::NAN)),
            },
            V::String(s) => J::Str(s.clone()),
            V::Array(a) => J::Arr(a.iter().map(J::from).collect()),
            V::Object(o) => J::Obj(o.iter().map(|(k, v)| (k.clone(), J::from(v))).collect()),
        }
    }
}

/// `repr(float)` de Python : le plus court qui redonne le même nombre ; notation scientifique hors de 1e-4 ≤ |x| < 1e16.
pub fn py_float(f: f64) -> String {
    if f.is_nan() {
        return "nan".into();
    }
    if f.is_infinite() {
        return if f > 0.0 { "inf".into() } else { "-inf".into() };
    }
    if f == 0.0 {
        return if f.is_sign_negative() { "-0.0".into() } else { "0.0".into() };
    }
    // comme Python : le moins de chiffres (arrondis correctement) qui redonnent le même nombre
    let e = (1..=17)
        .map(|p| format!("{:.*e}", p - 1, f.abs()))
        .find(|s| s.parse::<f64>().ok() == Some(f.abs()))
        .unwrap_or_else(|| format!("{:e}", f.abs()));
    let (mant, exp) = e.split_once('e').unwrap_or((&e, "0"));
    let exp: i32 = exp.parse().unwrap_or(0);
    let digits: String = mant.chars().filter(|c| c.is_ascii_digit()).collect();
    let signe = if f < 0.0 { "-" } else { "" };
    let n = digits.len() as i32;
    if (-4..16).contains(&exp) {
        let decpt = exp + 1;
        let corps = if decpt <= 0 {
            format!("0.{}{}", "0".repeat((-decpt) as usize), digits)
        } else if decpt >= n {
            format!("{}{}.0", digits, "0".repeat((decpt - n) as usize))
        } else {
            format!("{}.{}", &digits[..decpt as usize], &digits[decpt as usize..])
        };
        format!("{signe}{corps}")
    } else {
        let reste = if n > 1 { format!(".{}", &digits[1..]) } else { String::new() };
        let s = if exp < 0 { '-' } else { '+' };
        format!("{signe}{}{reste}e{s}{:02}", &digits[..1], exp.abs())
    }
}

/// Une chaîne entre guillemets, comme `json.dumps` (`ascii` = `ensure_ascii`, le réglage par défaut de Python).
pub fn py_str(s: &str, ascii: bool) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            '\u{8}' => o.push_str("\\b"),
            '\u{c}' => o.push_str("\\f"),
            c if (c as u32) < 0x20 => {
                let _ = write!(o, "\\u{:04x}", c as u32);
            }
            c if ascii && !(' '..='~').contains(&c) => {
                let mut buf = [0u16; 2];
                for u in c.encode_utf16(&mut buf) {
                    let _ = write!(o, "\\u{:04x}", u);
                }
            }
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

/// `json.dumps(v)` (sur une ligne) ; `ascii` = `ensure_ascii`.
pub fn dumps(v: &J, ascii: bool) -> String {
    let mut o = String::new();
    ecrire(v, ascii, &mut o);
    o
}

fn ecrire(v: &J, ascii: bool, o: &mut String) {
    match v {
        J::Null => o.push_str("null"),
        J::Bool(b) => o.push_str(if *b { "true" } else { "false" }),
        J::Int(i) => {
            let _ = write!(o, "{i}");
        }
        J::Float(f) => o.push_str(&match py_float(*f).as_str() {
            "nan" => "NaN".to_string(),
            "inf" => "Infinity".to_string(),
            "-inf" => "-Infinity".to_string(),
            x => x.to_string(),
        }),
        J::Str(s) => o.push_str(&py_str(s, ascii)),
        J::Arr(a) => {
            o.push('[');
            for (i, x) in a.iter().enumerate() {
                if i > 0 {
                    o.push_str(", ");
                }
                ecrire(x, ascii, o);
            }
            o.push(']');
        }
        J::Obj(m) => {
            o.push('{');
            for (i, (k, x)) in m.iter().enumerate() {
                if i > 0 {
                    o.push_str(", ");
                }
                o.push_str(&py_str(k, ascii));
                o.push_str(": ");
                ecrire(x, ascii, o);
            }
            o.push('}');
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floats_comme_python() {
        for (f, s) in [
            (1.0, "1.0"),
            (0.1, "0.1"),
            (12.345, "12.345"),
            (1e16, "1e+16"),
            (1.5e-5, "1.5e-05"),
            (0.0001, "0.0001"),
            (-1000000.0, "-1000000.0"),
            (123456789012345.6, "123456789012345.6"),
            (2.5e20, "2.5e+20"),
            (-0.0, "-0.0"),
            (f64::from(100.22_f32), "100.22000122070312"), // égalité parfaite au 17e chiffre : arrondi au pair
        ] {
            assert_eq!(py_float(f), s, "{f}");
        }
    }

    #[test]
    fn chaines_comme_python() {
        assert_eq!(py_str("é\"\\\n\u{1}", false), "\"é\\\"\\\\\\n\\u0001\"");
        assert_eq!(py_str("é😀", true), "\"\\u00e9\\ud83d\\ude00\"");
        let v = J::obj(vec![("a", J::Int(1)), ("b", J::Arr(vec![J::Float(2.0), J::Null]))]);
        assert_eq!(dumps(&v, false), "{\"a\": 1, \"b\": [2.0, null]}");
    }
}
