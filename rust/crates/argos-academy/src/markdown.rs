//! 📝 Mini-lecteur Markdown maison, limité à ce que les leçons et la doc de l'éditeur utilisent : titres (`#`, `##`,
//! `###`), paragraphes, gras (`**…**`), listes (`- `, `1. `, cases `- [ ] `), tableaux (`| a | b |`), blocs de code
//! (```` ``` ````) et blocs repliables (`<details><summary>…</summary> … </details>`). Tout le reste est gardé comme
//! texte simple.

/// Un morceau de texte, en gras ou non.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Morceau {
    pub texte: String,
    pub gras: bool,
}

/// Un bloc d'une section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Bloc {
    /// Un paragraphe (une ligne du fichier).
    Para(Vec<Morceau>),
    /// Une liste : numérotée ou à puces ; chaque élément est une ligne de texte.
    Liste { numerotee: bool, elements: Vec<Vec<Morceau>> },
    /// Un tableau : la première ligne est l'en-tête (texte simple, sans `**`).
    Tableau(Vec<Vec<String>>),
    /// Un bloc repliable : son titre et son contenu.
    Details { resume: String, blocs: Vec<Bloc> },
    /// A fenced code block (```` ``` ````): its lines exactly as written (indentation and spaces kept, no `**`).
    Code(Vec<String>),
    /// A heading inside a part (`###` and deeper; `#` / `##` when the caller did not split on them): level, text.
    Titre(u8, String),
}

/// A Markdown file cut into parts at its `## ` headings (outside code blocks): (title, blocks of the part). What
/// comes before the first `## ` (the `# ` title, the date line) is left out.
pub fn parties(src: &str) -> Vec<(String, Vec<Bloc>)> {
    let lignes: Vec<&str> = src.lines().collect();
    let mut debuts = Vec::new();
    let mut dans_code = false;
    for (i, l) in lignes.iter().enumerate() {
        if l.trim_start().starts_with("```") {
            dans_code = !dans_code;
        } else if !dans_code && l.starts_with("## ") {
            debuts.push(i);
        }
    }
    debuts
        .iter()
        .enumerate()
        .map(|(k, &d)| {
            let fin = debuts.get(k + 1).copied().unwrap_or(lignes.len());
            (sans_gras(lignes[d][3..].trim()), blocs(&lignes[d + 1..fin]))
        })
        .collect()
}

/// `### Titre` → (3, "Titre"); `None` when the line is not a heading (`#` then a space).
fn titre(t: &str) -> Option<(u8, &str)> {
    let n = t.chars().take_while(|c| *c == '#').count();
    (1..=6).contains(&n).then(|| t[n..].strip_prefix(' ').map(|r| (n as u8, r.trim()))).flatten()
}

/// Découpe une ligne en morceaux gras / pas gras (`**…**`). Un `**` sans fin reste du texte.
pub fn morceaux(ligne: &str) -> Vec<Morceau> {
    let mut out = Vec::new();
    let mut reste = ligne;
    while let Some(d) = reste.find("**") {
        let Some(f) = reste[d + 2..].find("**") else { break };
        if d > 0 {
            out.push(Morceau {
                texte: reste[..d].to_string(),
                gras: false,
            });
        }
        out.push(Morceau {
            texte: reste[d + 2..d + 2 + f].to_string(),
            gras: true,
        });
        reste = &reste[d + 2 + f + 2..];
    }
    if !reste.is_empty() {
        out.push(Morceau {
            texte: reste.to_string(),
            gras: false,
        });
    }
    out
}

/// Le texte simple d'une suite de morceaux (sans les `**`).
pub fn simple(m: &[Morceau]) -> String {
    m.iter().map(|x| x.texte.as_str()).collect()
}

/// Retire les `**` d'une ligne.
pub fn sans_gras(s: &str) -> String {
    simple(&morceaux(s))
}

/// `1. texte` → `texte` (numéro de 1 à 3 chiffres).
pub fn element_numerote(l: &str) -> Option<&str> {
    let n = l.chars().take_while(char::is_ascii_digit).count();
    (1..=3).contains(&n).then(|| l[n..].strip_prefix(". ")).flatten()
}

/// Les cellules d'une ligne de tableau `| a | b |`.
fn cellules(l: &str) -> Vec<String> {
    let l = l.trim().trim_start_matches('|').trim_end_matches('|');
    l.split('|').map(|c| sans_gras(c.trim())).collect()
}

/// Ligne de séparation `|---|---|`.
fn separation(l: &str) -> bool {
    let t = l.trim();
    t.starts_with('|') && t.chars().all(|c| matches!(c, '|' | '-' | ':' | ' '))
}

/// Lit les lignes d'une section en blocs.
pub fn blocs(lignes: &[&str]) -> Vec<Bloc> {
    let mut out: Vec<Bloc> = Vec::new();
    let mut i = 0;
    while i < lignes.len() {
        let l = lignes[i].trim_end();
        let t = l.trim_start();
        if t.is_empty() {
            i += 1;
            continue;
        }
        // ``` … ``` : the lines as they are (a block without its end runs to the end of the part)
        if t.starts_with("```") {
            let debut = i + 1;
            let mut fin = debut;
            while fin < lignes.len() && !lignes[fin].trim_start().starts_with("```") {
                fin += 1;
            }
            out.push(Bloc::Code(lignes[debut..fin].iter().map(|l| l.to_string()).collect()));
            i = fin + 1;
            continue;
        }
        if let Some((n, s)) = titre(t) {
            out.push(Bloc::Titre(n, sans_gras(s)));
            i += 1;
            continue;
        }
        // <details><summary>…</summary> … </details>
        if let Some(rest) = t.strip_prefix("<details>") {
            let resume = rest
                .trim()
                .strip_prefix("<summary>")
                .and_then(|r| r.split("</summary>").next())
                .unwrap_or("")
                .trim()
                .to_string();
            let debut = i + 1;
            let mut fin = debut;
            while fin < lignes.len() && !lignes[fin].trim().starts_with("</details>") {
                fin += 1;
            }
            out.push(Bloc::Details {
                resume,
                blocs: blocs(&lignes[debut..fin]),
            });
            i = fin + 1;
            continue;
        }
        if t.starts_with('|') {
            let mut rows = Vec::new();
            while i < lignes.len() && lignes[i].trim_start().starts_with('|') {
                if !separation(lignes[i]) {
                    rows.push(cellules(lignes[i]));
                }
                i += 1;
            }
            out.push(Bloc::Tableau(rows));
            continue;
        }
        let puce = |s: &str| s.strip_prefix("- [ ] ").or_else(|| s.strip_prefix("- ")).map(str::to_string);
        if puce(t).is_some() || element_numerote(t).is_some() {
            let numerotee = puce(t).is_none();
            let mut elements = Vec::new();
            while i < lignes.len() {
                let t = lignes[i].trim();
                let e = if numerotee { element_numerote(t).map(str::to_string) } else { puce(t) };
                let Some(e) = e else { break };
                elements.push(morceaux(&e));
                i += 1;
            }
            out.push(Bloc::Liste { numerotee, elements });
            continue;
        }
        out.push(Bloc::Para(morceaux(t)));
        i += 1;
    }
    out
}

/// Le texte simple d'un bloc (pour la recherche et la référence de l'assistant) ; tableaux en lignes `| a | b |`.
pub fn texte_bloc(b: &Bloc) -> String {
    match b {
        Bloc::Para(m) => simple(m),
        Bloc::Liste { numerotee, elements } => elements
            .iter()
            .enumerate()
            .map(|(k, e)| {
                if *numerotee {
                    format!("{}. {}", k + 1, simple(e))
                } else {
                    format!("• {}", simple(e))
                }
            })
            .collect::<Vec<_>>()
            .join("\n"),
        Bloc::Tableau(rows) => rows.iter().map(|r| format!("| {} |", r.join(" | "))).collect::<Vec<_>>().join("\n"),
        Bloc::Code(lignes) => lignes.join("\n"),
        Bloc::Titre(_, t) => t.clone(),
        Bloc::Details { resume, blocs } => {
            let mut s = resume.clone();
            for b in blocs {
                s += "\n";
                s += &texte_bloc(b);
            }
            s
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gras_listes_tableaux_details() {
        assert_eq!(
            morceaux("a **b** c"),
            vec![
                Morceau {
                    texte: "a ".into(),
                    gras: false
                },
                Morceau { texte: "b".into(), gras: true },
                Morceau {
                    texte: " c".into(),
                    gras: false
                }
            ]
        );
        assert_eq!(sans_gras("**x** et **y"), "x et **y");
        let src = [
            "Un paragraphe.",
            "- un",
            "- [ ] deux",
            "1. premier",
            "2. second",
            "| A | B |",
            "|---|---|",
            "| **1** | 2 |",
            "<details><summary>Chiffres</summary>",
            "",
            "- 19 %",
            "</details>",
        ];
        let b = blocs(&src);
        assert_eq!(b.len(), 5);
        assert!(matches!(&b[1], Bloc::Liste { numerotee: false, elements } if elements.len() == 2));
        assert!(matches!(&b[2], Bloc::Liste { numerotee: true, elements } if elements.len() == 2));
        assert_eq!(b[3], Bloc::Tableau(vec![vec!["A".into(), "B".into()], vec!["1".into(), "2".into()]]));
        assert!(matches!(&b[4], Bloc::Details { resume, blocs } if resume == "Chiffres" && blocs.len() == 1));
        assert_eq!(texte_bloc(&b[3]), "| A | B |\n| 1 | 2 |");
        assert_eq!(element_numerote("12. x"), Some("x"));
        assert_eq!(element_numerote("2026. x"), None);
    }

    /// The editor's 📖 Doc: fenced code kept as written (a `## ` or `#` inside it is code, not a heading), `###`
    /// headings, parts cut at `## `.
    #[test]
    fn code_titres_parties() {
        let src =
            "# Titre\n\nintro\n\n## Un\n\n### Sous **titre**\n\n```rust\n#![forbid(unsafe_code)]\n## pas un titre\n    x = 1  \n```\n\n- a\n\n## Deux\n\ntexte";
        let p = parties(src);
        assert_eq!(p.len(), 2);
        assert_eq!(p[0].0, "Un");
        assert_eq!(p[0].1[0], Bloc::Titre(3, "Sous titre".into()));
        assert_eq!(
            p[0].1[1],
            Bloc::Code(vec!["#![forbid(unsafe_code)]".into(), "## pas un titre".into(), "    x = 1  ".into()])
        );
        assert!(matches!(&p[0].1[2], Bloc::Liste { .. }));
        assert_eq!(
            texte_bloc(&p[0].1[1]),
            "#![forbid(unsafe_code)]\n## pas un titre\n    x = 1  ",
            "exactly as written"
        );
        assert_eq!(p[1].0, "Deux");
        assert_eq!(titre("#hashtag"), None);
    }
}
