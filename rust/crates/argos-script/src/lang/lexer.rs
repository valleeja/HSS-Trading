//! ✂️ Lecture du texte d'un script en jetons, comme Pine v6 : les blocs se font par l'indentation (4 espaces ou une
//! tabulation = un niveau). Une ligne indentée d'un nombre d'espaces qui n'est pas un multiple de 4, une ligne dans des
//! parenthèses ou crochets ouverts, ou qui suit un opérateur ou une virgule, continue la ligne d'avant.

/// Un jeton et sa ligne (pour les messages d'erreur).
#[derive(Debug, Clone, PartialEq)]
pub struct Tok {
    pub t: T,
    pub line: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum T {
    Num(f64),
    Str(String),
    /// Couleur littérale `#RRGGBB` ou `#RRGGBBAA` : 0xRRGGBB et la transparence (0–100) dans les bits 24–30.
    Color(u32),
    Ident(String),
    /// Ponctuation et opérateurs : `( ) [ ] , . ? : = := += -= *= /= %= => + - * / % == != < <= > >=`.
    Op(&'static str),
    Newline,
    Indent,
    Dedent,
    Eof,
}

/// Erreur avec sa ligne.
#[derive(Debug, Clone, PartialEq)]
pub struct Error {
    pub line: u32,
    pub msg: String,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ligne {} : {}", self.line, self.msg)
    }
}

const OPS: [&str; 30] = [
    ":=", "+=", "-=", "*=", "/=", "%=", "=>", "==", "!=", "<=", ">=", "(", ")", "[", "]", ",", ".", "?", ":", "=", "+", "-", "*", "/", "%", "<", ">", "{", "}",
    ";",
];

/// Un opérateur à la fin d'une ligne : la ligne continue en dessous (sauf `=>`, suivi du bloc d'une fonction).
fn continues(t: &T) -> bool {
    matches!(t, T::Op(o) if !matches!(*o, ")" | "]" | "}" | "=>")) || matches!(t, T::Ident(w) if w == "and" || w == "or" || w == "not")
}

/// Couleur `#RRGGBB(AA)` → 0xRRGGBB + transparence (alpha 255 = 0 %, 0 = 100 %) dans les bits 24–30.
fn hex_color(h: &str) -> Option<u32> {
    let v = u32::from_str_radix(h, 16).ok()?;
    match h.len() {
        6 => Some(v),
        8 => {
            let alpha = v & 0xff;
            let transp = (100 - (alpha * 100 + 127) / 255).min(100);
            Some((v >> 8) | (transp << 24))
        }
        _ => None,
    }
}

pub fn lex(src: &str) -> Result<Vec<Tok>, Error> {
    let mut out: Vec<Tok> = vec![];
    let mut levels: Vec<usize> = vec![0];
    let mut depth = 0i32; // parenthèses / crochets ouverts
    for (ln, raw) in src.lines().enumerate() {
        let line = ln as u32 + 1;
        // indentation (tabulation = 4 espaces)
        let mut ind = 0;
        for c in raw.chars() {
            match c {
                ' ' => ind += 1,
                '\t' => ind += 4,
                _ => break,
            }
        }
        let body = raw.trim_start();
        let code = strip_comment(body);
        if code.trim().is_empty() {
            continue;
        }
        let prev_cont = out.last().is_some_and(|t| t.t != T::Newline && continues(&t.t));
        let wrapped = depth > 0 || prev_cont || (ind % 4 != 0 && out.last().is_some_and(|t| t.t != T::Newline));
        if wrapped {
            // continuation : on retire le saut de ligne d'avant
            if out.last().is_some_and(|t| t.t == T::Newline) {
                out.pop();
            }
        } else if !out.is_empty() {
            let cur = *levels.last().unwrap_or(&0);
            let lvl = ind / 4 * 4;
            if lvl > cur {
                levels.push(lvl);
                out.push(Tok { t: T::Indent, line });
            } else {
                while lvl < *levels.last().unwrap_or(&0) {
                    levels.pop();
                    out.push(Tok { t: T::Dedent, line });
                }
                if lvl != *levels.last().unwrap_or(&0) {
                    return Err(Error {
                        line,
                        msg: "indentation incohérente".into(),
                    });
                }
            }
        } else if ind > 0 {
            return Err(Error {
                line,
                msg: "la première ligne ne doit pas être indentée".into(),
            });
        }
        let cs: Vec<char> = code.chars().collect();
        let mut i = 0;
        while i < cs.len() {
            let c = cs[i];
            if c.is_whitespace() {
                i += 1;
                continue;
            }
            // nombre : 12, 1.5, .5, 1e-3
            if c.is_ascii_digit() || (c == '.' && cs.get(i + 1).is_some_and(|d| d.is_ascii_digit())) {
                let s = i;
                while i < cs.len() && (cs[i].is_ascii_digit() || cs[i] == '.' || cs[i] == '_') {
                    i += 1;
                }
                if i < cs.len() && (cs[i] == 'e' || cs[i] == 'E') {
                    i += 1;
                    if i < cs.len() && (cs[i] == '-' || cs[i] == '+') {
                        i += 1;
                    }
                    while i < cs.len() && cs[i].is_ascii_digit() {
                        i += 1;
                    }
                }
                let txt: String = cs[s..i].iter().filter(|c| **c != '_').collect();
                let v = txt.parse::<f64>().map_err(|_| Error {
                    line,
                    msg: format!("nombre invalide « {txt} »"),
                })?;
                out.push(Tok { t: T::Num(v), line });
                continue;
            }
            if c == '"' || c == '\'' {
                let q = c;
                i += 1;
                let mut s = String::new();
                while i < cs.len() && cs[i] != q {
                    if cs[i] == '\\' && i + 1 < cs.len() {
                        i += 1;
                        s.push(match cs[i] {
                            'n' => '\n',
                            't' => '\t',
                            o => o,
                        });
                    } else {
                        s.push(cs[i]);
                    }
                    i += 1;
                }
                if i >= cs.len() {
                    return Err(Error {
                        line,
                        msg: "texte pas fermé (guillemet manquant)".into(),
                    });
                }
                i += 1;
                out.push(Tok { t: T::Str(s), line });
                continue;
            }
            if c == '#' {
                let s = i + 1;
                i += 1;
                while i < cs.len() && cs[i].is_ascii_hexdigit() {
                    i += 1;
                }
                let h: String = cs[s..i].iter().collect();
                let v = hex_color(&h).ok_or(Error {
                    line,
                    msg: format!("couleur invalide « #{h} »"),
                })?;
                out.push(Tok { t: T::Color(v), line });
                continue;
            }
            if c.is_alphabetic() || c == '_' {
                let s = i;
                while i < cs.len() && (cs[i].is_alphanumeric() || cs[i] == '_') {
                    i += 1;
                }
                out.push(Tok {
                    t: T::Ident(cs[s..i].iter().collect()),
                    line,
                });
                continue;
            }
            let rest: String = cs[i..cs.len().min(i + 2)].iter().collect();
            let Some(op) = OPS.iter().find(|o| rest.starts_with(**o)) else {
                return Err(Error {
                    line,
                    msg: format!("caractère inattendu « {c} »"),
                });
            };
            match *op {
                "(" | "[" | "{" => depth += 1,
                ")" | "]" | "}" => depth -= 1,
                _ => {}
            }
            i += op.len();
            out.push(Tok { t: T::Op(op), line });
        }
        if depth == 0 && !out.last().is_some_and(|t| continues(&t.t)) {
            out.push(Tok { t: T::Newline, line });
        }
    }
    let line = src.lines().count() as u32;
    if !out.last().is_some_and(|t| t.t == T::Newline) {
        out.push(Tok { t: T::Newline, line });
    }
    while levels.len() > 1 {
        levels.pop();
        out.push(Tok { t: T::Dedent, line });
    }
    out.push(Tok { t: T::Eof, line });
    Ok(out)
}

/// Enlève le commentaire `//` (pas dans un texte entre guillemets).
fn strip_comment(s: &str) -> &str {
    let mut quote: Option<char> = None;
    let b: Vec<(usize, char)> = s.char_indices().collect();
    for (k, &(i, c)) in b.iter().enumerate() {
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) => {}
            None if c == '"' || c == '\'' => quote = Some(c),
            None if c == '/' && b.get(k + 1).is_some_and(|n| n.1 == '/') => return &s[..i],
            None => {}
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(src: &str) -> Vec<T> {
        lex(src).unwrap().into_iter().map(|t| t.t).collect()
    }

    #[test]
    fn blocs_par_indentation_et_continuations() {
        let k = kinds("x = 1 // commentaire\nif x > 0\n    y = 2\n    z = x +\n         y\nplot(x,\n  color=#FF000080)\n");
        assert!(k.contains(&T::Indent) && k.contains(&T::Dedent));
        // « z = x + y » sur deux lignes : un seul saut de ligne
        let z = k.iter().position(|t| *t == T::Ident("z".into())).unwrap();
        assert_eq!(
            k[z..z + 6],
            [
                T::Ident("z".into()),
                T::Op("="),
                T::Ident("x".into()),
                T::Op("+"),
                T::Ident("y".into()),
                T::Newline
            ]
        );
        assert!(k.contains(&T::Color(0xFF0000 | (50 << 24))), "alpha 0x80 ≈ 50 % de transparence");
        assert_eq!(kinds("a := 'b // pas un commentaire'")[2], T::Str("b // pas un commentaire".into()));
        assert!(lex("  x = 1").is_err());
    }
}
