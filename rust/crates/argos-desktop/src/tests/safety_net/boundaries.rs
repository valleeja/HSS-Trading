//! 🕸 Safety net of the split into modules (step 0, AIP-0001): the boundaries between the boxes of the map.
//!
//! What it protects: the split cuts the links between the modules one lot at a time. A link that was cut must not
//! come back, and no new one may appear, while the others are being cut. So the links are counted, and a count
//! only goes down.
//!
//! - **The map** (`map.txt`, written by hand) puts every source file of the workspace in a box: one of the big
//!   modules (A to W), the drawing kit, the shared data, the base types, the shell, a mixed file, or the tests of the
//!   whole app. A file without a box makes `every_source_file_is_on_the_map` fail.
//! - **A link** is a place where a file names another file ([`World::links`]): by a path (`crate::murs::Legende`,
//!   `super::x`, `argos_render::murs::Legende`, each name between the braces of a `use`), by a module brought in
//!   under its name (`murs::Legende`), by a name the main file of the app brings in for all its files, or by calling
//!   a function of `App` written in another file. The code is read as text, without compiling it: what this reading
//!   cannot see (a method called on a value of another module, where no file is named) the compiler will, the day
//!   the module is a crate of its own.
//! - **What is counted**: the links from one box to another that the plan does not let through ([`free`]). A big
//!   module may use the drawing kit, the shared data, the base types and the modules it declares it needs
//!   ([`NEEDS`]); everything else is to go through the door. These links are kept, pair of boxes by pair of boxes,
//!   in `tests/references/safety-net-boundaries.txt`, with the crates each crate depends on.
//! - **A ratchet**: a count higher than the one kept fails. A lower one fails too, until it is written:
//!   `ARGOS_REFERENCES=maj` writes the lower counts, and never a higher one. A count that has to go up (a piece
//!   that goes back to its module takes its links with it) is written by hand in that file, and the report of the
//!   lot says why.
//!
//! `ARGOS_LIENS=<file>` writes every link there (the file and its line, the file it names, the name, the two
//! boxes): where the links of a pair are, and, written before and after a change, which ones are new.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// The crate of the app: its main file brings names in for all its files (they write `use crate::*`), and its
/// functions are written on `App`, in many files.
const APP: &str = "argos-desktop";

/// The boxes of the map, in the order of the plan: the big modules by category, then the centre.
const BOXES: &[(&str, &str)] = &[
    ("A", "Carnet"),
    ("B", "Flux d'ordres"),
    ("C", "Marchés"),
    ("E", "Watchlist"),
    ("F", "Alertes"),
    ("G", "Actualités et calendrier"),
    ("J", "Chart"),
    ("H", "Argos Script"),
    ("I", "Argos AI"),
    ("D", "Scanner"),
    ("L", "Trading démo"),
    ("M", "Portefeuille et journal"),
    ("N", "Simulateur"),
    ("K", "Bots et stratégies"),
    ("O", "PropFirm"),
    ("P", "Académie"),
    ("Q", "Compte"),
    ("R", "Communauté"),
    ("S", "Support"),
    ("T", "Marketplace"),
    ("U", "Gagner"),
    ("V", "Langues"),
    ("W", "Interface"),
    ("KIT", "the drawing kit"),
    ("DON", "the shared data"),
    ("BASE", "the base types"),
    ("COQ", "the shell"),
    ("MIX", "a mixed file"),
    (TESTS, "the tests of the whole app"),
];

/// The box of the tests of the whole app: they may name everything, their links are not counted.
const TESTS: &str = "TESTS";

/// What each big module declares it needs: the other big modules it may use without going through the door.
const NEEDS: &[(&str, &[&str])] = &[
    ("D", &["H", "B"]),
    ("H", &["J"]),
    ("I", &["J", "H"]),
    ("L", &["J"]),
    ("M", &["L", "J"]),
    ("N", &["L", "J", "H", "O", "I"]),
    ("K", &["H", "I", "L", "N", "J"]),
    ("R", &["M", "Q"]),
    ("T", &["H"]),
    ("U", &["Q"]),
];

/// Names the main file of the app brings in that are also the names of other things in its files (a variant, a
/// field, a function): read as links, they would count what is not one.
const COMMON: &[&str] = &[
    "Hit",
    "Click",
    "Source",
    "Tab",
    "Act",
    "Btn",
    "Stat",
    "Tick",
    "Chip",
    "Found",
    "Point",
    "Style",
    "Frame",
    "Target",
    "Outcome",
    "Anchor",
    "Ctx",
    "Rect",
    "Bar",
    "Interval",
    "Timeframe",
    "Exchange",
    "Theme",
    "render",
    "frame",
    "slots",
    "aggregate",
    "Instrument",
    "Search",
    "SECTIONS",
    "DEFAULT_CANDLES",
    "Chart",
    "CandleStyle",
    "current_scale",
];

/// The folders of the app whose files hold the functions of `App`: `actions::load::open` names `actions/load.rs`.
const APP_FOLDERS: [&str; 5] = ["actions", "input", "persist", "state", "security"];

/// The names an `App` goes by where its functions are called: `self.redraw()`, `a.redraw()`.
const APP_NAMES: [&str; 4] = ["self", "a", "app", "me"];

/// What the plan lets a box use without going through the door.
fn free(from: &str, to: &str) -> bool {
    let centre = |boxes: &[&str]| boxes.contains(&to);
    from == to
        || match from {
            "BASE" => false,
            "DON" => centre(&["BASE"]),
            "KIT" => centre(&["DON", "BASE"]),
            "COQ" | "MIX" => centre(&["COQ", "MIX", "KIT", "DON", "BASE"]),
            module => centre(&["KIT", "DON", "BASE"]) || NEEDS.iter().any(|(m, needs)| *m == module && needs.contains(&to)),
        }
}

/// The place of a box in [`BOXES`].
fn place(name: &str) -> Option<usize> {
    BOXES.iter().position(|(b, _)| *b == name)
}

// ---------------------------------------------------------------------------------------------------------------
// The code, read as words and signs
// ---------------------------------------------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    Word,
    /// `::`
    Path,
    Sign(u8),
    /// A text between quotes, a character, a number: nothing a link is made of.
    Other,
}

/// A word or a sign of the code. The comments are left out.
#[derive(Clone, Copy, Debug)]
struct Tok {
    /// Where it starts and where it ends in the text of its file.
    at: (u32, u32),
    line: u32,
    kind: Kind,
}

fn in_a_word(c: u8) -> bool {
    c == b'_' || c.is_ascii_alphanumeric() || c >= 0x80
}

/// The end of a text between quotes, from just after the quote that opens it. `raw`: the number of `#` that close
/// it with the quote (then a `\` is a character like another).
fn quote_end(b: &[u8], mut i: usize, raw: Option<usize>, line: &mut u32) -> usize {
    while i < b.len() {
        match (b[i], raw) {
            (b'\\', None) => {
                *line += u32::from(b.get(i + 1) == Some(&b'\n'));
                i += 2;
            }
            (b'"', None) => return i + 1,
            (b'"', Some(n)) if b.get(i + 1..i + 1 + n).is_some_and(|h| h.iter().all(|c| *c == b'#')) => return i + 1 + n,
            (c, _) => {
                *line += u32::from(c == b'\n');
                i += 1;
            }
        }
    }
    b.len()
}

/// The end of the character written at `i` (`'a'`, `'\n'`, `'\u{1F600}'`), or nothing if the quote at `i` opens a
/// lifetime (`'a`).
fn char_end(text: &str, i: usize) -> Option<usize> {
    let b = text.as_bytes();
    if b.get(i + 1) == Some(&b'\\') {
        return Some(b[(i + 3).min(b.len())..].iter().position(|c| *c == b'\'').map_or(b.len(), |k| i + 3 + k + 1));
    }
    let width = text.get(i + 1..)?.chars().next()?.len_utf8();
    (b.get(i + 1 + width) == Some(&b'\'')).then_some(i + width + 2)
}

/// The words and the signs of a source file.
fn read(text: &str) -> Vec<Tok> {
    let b = text.as_bytes();
    let (mut toks, mut i, mut line) = (Vec::new(), 0usize, 1u32);
    while i < b.len() {
        let (mut start, from) = (i, line);
        let kind = match b[i] {
            b'\n' => {
                line += 1;
                i += 1;
                continue;
            }
            c if c.is_ascii_whitespace() => {
                i += 1;
                continue;
            }
            b'/' if b.get(i + 1) == Some(&b'/') => {
                i += b[i..].iter().position(|c| *c == b'\n').unwrap_or(b.len() - i);
                continue;
            }
            b'/' if b.get(i + 1) == Some(&b'*') => {
                // (a comment may hold another one)
                let mut depth = 0;
                while i < b.len() {
                    if b[i..].starts_with(b"/*") {
                        depth += 1;
                        i += 2;
                    } else if b[i..].starts_with(b"*/") {
                        depth -= 1;
                        i += 2;
                        if depth == 0 {
                            break;
                        }
                    } else {
                        line += u32::from(b[i] == b'\n');
                        i += 1;
                    }
                }
                continue;
            }
            b'"' => {
                i = quote_end(b, i + 1, None, &mut line);
                Kind::Other
            }
            b'\'' => match char_end(text, i) {
                Some(end) => {
                    i = end;
                    Kind::Other
                }
                None => {
                    i += 1 + b[i + 1..].iter().position(|c| !in_a_word(*c)).unwrap_or(b.len() - i - 1);
                    continue;
                }
            },
            c if c.is_ascii_digit() => {
                i += b[i..].iter().position(|c| !in_a_word(*c)).unwrap_or(b.len() - i);
                if b.get(i) == Some(&b'.') && b.get(i + 1).is_some_and(u8::is_ascii_digit) {
                    i += 1;
                    i += b[i..].iter().position(|c| !in_a_word(*c)).unwrap_or(b.len() - i);
                }
                Kind::Other
            }
            c if in_a_word(c) => {
                i += b[i..].iter().position(|c| !in_a_word(*c)).unwrap_or(b.len() - i);
                let word = &b[start..i];
                let raw = matches!(word, b"r" | b"br" | b"cr");
                let hashes = b[i..].iter().take_while(|c| **c == b'#').count();
                if (raw || matches!(word, b"b" | b"c")) && b.get(i) == Some(&b'"') {
                    i = quote_end(b, i + 1, raw.then_some(0), &mut line);
                    Kind::Other
                } else if raw && hashes > 0 && b.get(i + hashes) == Some(&b'"') {
                    i = quote_end(b, i + hashes + 1, Some(hashes), &mut line);
                    Kind::Other
                } else if word == b"r" && hashes == 1 && b.get(i + 1).is_some_and(|c| in_a_word(*c)) {
                    // (`r#type`: the word is `type`)
                    start = i + 1;
                    i = start + b[start..].iter().position(|c| !in_a_word(*c)).unwrap_or(b.len() - start);
                    Kind::Word
                } else if let (b"b", Some(end)) = (word, char_end(text, i).filter(|_| b.get(i) == Some(&b'\''))) {
                    i = end;
                    Kind::Other
                } else {
                    Kind::Word
                }
            }
            b':' if b.get(i + 1) == Some(&b':') => {
                i += 2;
                Kind::Path
            }
            c => {
                i += 1;
                Kind::Sign(c)
            }
        };
        toks.push(Tok {
            at: (start as u32, i as u32),
            line: from,
            kind,
        });
    }
    toks
}

/// What a `use` brings in: the path it writes (without a `self` at its end) and the name it gives it with `as`.
struct Import<'a> {
    path: Vec<&'a str>,
    alias: Option<&'a str>,
    /// A plain `use` at the top of the file: not `pub`, not inside braces.
    top: bool,
}

/// A place where the code names something under a word (`crate`, `super`, a crate): the two names that follow.
struct Named<'a> {
    line: u32,
    first: &'a str,
    second: Option<&'a str>,
}

impl Named<'_> {
    fn name(&self) -> String {
        self.second.map_or(self.first.to_string(), |second| format!("{}::{second}", self.first))
    }
}

/// One source file of the workspace.
struct File {
    krate: usize,
    /// Its path under the `src/` of its crate, and under the folder of the workspace (as the map writes it).
    rel: String,
    path: String,
    /// Its box (nothing: it is not on the map).
    home: Option<usize>,
    text: String,
    toks: Vec<Tok>,
}

impl File {
    fn word(&self, i: usize) -> Option<&str> {
        let t = self.toks.get(i)?;
        (t.kind == Kind::Word).then(|| &self.text[t.at.0 as usize..t.at.1 as usize])
    }

    fn is(&self, i: usize, sign: u8) -> bool {
        self.toks.get(i).is_some_and(|t| t.kind == Kind::Sign(sign))
    }

    fn path(&self, i: usize) -> bool {
        self.toks.get(i).is_some_and(|t| t.kind == Kind::Path)
    }

    /// The word `i` is the end of something longer (`x::word`, or `x.word` with `dot`): not a name on its own.
    fn glued(&self, i: usize, dot: bool) -> bool {
        let start = self.toks[i].at.0 as usize;
        start > 0 && matches!(self.text.as_bytes()[start - 1], c if in_a_word(c) || c == b':' || (dot && c == b'.'))
    }

    /// The brace that closes the one opened at `open`, and the parts between the commas of that brace.
    fn group(&self, open: usize) -> (usize, Vec<(usize, usize)>) {
        let (mut depth, mut parts, mut from, mut i) = (1, Vec::new(), open + 1, open + 1);
        while i < self.toks.len() {
            match self.toks[i].kind {
                Kind::Sign(b'{') => depth += 1,
                Kind::Sign(b'}') if depth == 1 => break,
                Kind::Sign(b'}') => depth -= 1,
                Kind::Sign(b',') if depth == 1 => {
                    parts.push((from, i));
                    from = i + 1;
                }
                _ => {}
            }
            i += 1;
        }
        if from < i {
            parts.push((from, i));
        }
        (i, parts)
    }

    /// For each word and sign: is it inside a module written in the file itself (`mod tests { … }`)? There, `super`
    /// is the file, not its parent.
    fn inner(&self) -> Vec<bool> {
        let (mut inside, mut i) = (vec![false; self.toks.len()], 0);
        while i < self.toks.len() {
            if self.word(i) == Some("mod") && self.word(i + 1).is_some() && self.is(i + 2, b'{') {
                let close = self.group(i + 2).0;
                inside[i + 3..close].fill(true);
                i = close;
            }
            i += 1;
        }
        inside
    }

    /// Reads one tree of a `use` from `i` (`a::b`, `a::{b, c as d}`, `a::*`): adds what it brings in, and gives the
    /// place after it.
    fn tree<'a>(&'a self, mut i: usize, path: &mut Vec<&'a str>, top: bool, out: &mut Vec<Import<'a>>) -> usize {
        let depth = path.len();
        i += usize::from(self.path(i));
        loop {
            if self.is(i, b'{') {
                i += 1;
                while i < self.toks.len() && !self.is(i, b'}') {
                    let next = self.tree(i, path, top, out);
                    i = next.max(i + 1);
                    i += usize::from(self.is(i, b','));
                }
                i += 1;
                break;
            }
            let Some(word) = self.word(i) else {
                // (`*`, or something that is not a path: nothing is brought in under a name)
                i += usize::from(self.is(i, b'*'));
                break;
            };
            path.push(word);
            i += 1;
            if self.path(i) {
                i += 1;
                continue;
            }
            let alias = (self.word(i) == Some("as")).then(|| self.word(i + 1)).flatten();
            i += if alias.is_some() { 2 } else { 0 };
            let mut written = path.clone();
            if written.last() == Some(&"self") {
                written.pop();
            }
            out.push(Import { path: written, alias, top });
            break;
        }
        path.truncate(depth);
        i
    }

    /// Everything the `use`s of the file bring in.
    fn imports(&self) -> Vec<Import<'_>> {
        let (mut out, mut depth) = (Vec::new(), 0);
        for i in 0..self.toks.len() {
            match self.toks[i].kind {
                Kind::Sign(b'{') => depth += 1,
                Kind::Sign(b'}') => depth -= 1,
                _ => {}
            }
            if self.word(i) == Some("use") {
                let public = i > 0 && (self.word(i - 1) == Some("pub") || self.is(i - 1, b')'));
                self.tree(i + 1, &mut Vec::new(), depth == 0 && !public, &mut out);
            }
        }
        out
    }

    /// The places where the file names something under `under`: `under::a::b`, and each part between the braces of
    /// a `use under::{a::b, c}`. `skip`: the words and signs not to look at.
    fn named(&self, under: &str, skip: Option<&[bool]>) -> Vec<Named<'_>> {
        let mut out = Vec::new();
        let seen = |i: usize| !skip.is_some_and(|skip| skip[i]) && self.word(i) == Some(under) && self.path(i + 1);
        for i in (0..self.toks.len()).filter(|i| seen(*i)) {
            let line = self.toks[i].line;
            if self.is(i + 2, b'{') {
                if i > 0 && self.word(i - 1) == Some("use") {
                    for (from, to) in self.group(i + 2).1 {
                        let second = (from + 2 < to && self.path(from + 1)).then(|| self.word(from + 2)).flatten();
                        if let Some(first) = self.word(from).filter(|first| *first != "self") {
                            out.push(Named { line, first, second });
                        }
                    }
                }
            } else if let Some(first) = self.word(i + 2).filter(|_| !self.glued(i, false)) {
                let second = self.path(i + 3).then(|| self.word(i + 4)).flatten();
                out.push(Named { line, first, second });
            }
        }
        out
    }

    /// The parts of the file inside an `impl App { … }` or an `impl … for App { … }`: (the brace that opens it, the
    /// one that closes it).
    fn app_blocks(&self) -> Vec<(usize, usize)> {
        let past_angles = |mut i: usize| {
            if self.is(i, b'<') {
                let mut depth = 0;
                while i < self.toks.len() {
                    // (the `>` of a `->` closes nothing)
                    let arrow = self.is(i, b'>') && i > 0 && self.is(i - 1, b'-');
                    depth += i32::from(self.is(i, b'<')) - i32::from(self.is(i, b'>') && !arrow);
                    i += 1;
                    if depth == 0 {
                        break;
                    }
                }
            }
            i
        };
        let mut blocks = Vec::new();
        for i in (0..self.toks.len()).filter(|i| self.word(*i) == Some("impl")) {
            let (mut j, mut last) = (past_angles(i + 1), None);
            loop {
                // (a path: words joined by `::`)
                while let Some(word) = self.word(j) {
                    last = Some(word);
                    j += 1;
                    if !self.path(j) {
                        break;
                    }
                    j += 1;
                }
                j = past_angles(j);
                if self.word(j) != Some("for") {
                    break;
                }
                j += 1;
            }
            let open = (j..self.toks.len()).find(|k| self.is(*k, b'{') || self.is(*k, b';'));
            if let Some(open) = open.filter(|open| last == Some("App") && self.is(*open, b'{')) {
                blocks.push((open, self.group(open).0));
            }
        }
        blocks
    }
}

// ---------------------------------------------------------------------------------------------------------------
// The workspace and its links
// ---------------------------------------------------------------------------------------------------------------

struct Crate {
    name: String,
    /// Its name as the code writes it (`argos_render`).
    word: String,
    /// Its source files: path under `src/` → place in [`World::files`].
    files: BTreeMap<String, usize>,
    /// The crates of the workspace its `Cargo.toml` names.
    needs: BTreeSet<String>,
}

/// A place where a file names another file.
struct Link {
    from: usize,
    to: usize,
    line: u32,
    name: String,
    /// The kind of link: `path`, `module`, `child`, `brought in`, `call`.
    by: &'static str,
}

/// The source files of the crates of the workspace.
struct World {
    crates: Vec<Crate>,
    files: Vec<File>,
}

/// The links found so far.
struct Found<'w> {
    world: &'w World,
    links: Vec<Link>,
}

impl Found<'_> {
    /// A file never links to itself, and the tests of the whole app may name everything.
    fn add(&mut self, from: usize, to: Option<usize>, line: u32, name: impl Into<String>, by: &'static str) {
        let tests = self.world.files[from].home == place(TESTS);
        if let Some(to) = to.filter(|to| *to != from && !tests) {
            self.links.push(Link {
                from,
                to,
                line,
                name: name.into(),
                by,
            });
        }
    }
}

impl World {
    /// The file of a module of a crate, from its path (`["actions", "load"]`: `actions/load.rs`, or
    /// `actions/load/mod.rs`).
    fn module(&self, krate: usize, path: &[&str]) -> Option<usize> {
        let (files, path) = (&self.crates[krate].files, path.join("/"));
        files.get(&format!("{path}.rs")).or_else(|| files.get(&format!("{path}/mod.rs"))).copied()
    }

    /// The file that `krate::first::second` names: the module `first::second`, or the module `first`, or the module
    /// the main file of the crate takes `first` from to give it again (`again`), or that main file (`lib.rs`).
    fn target(&self, krate: usize, named: &Named, again: &[HashMap<String, usize>]) -> Option<usize> {
        let first = [named.first];
        match self.module(krate, &first) {
            Some(file) => named.second.and_then(|second| self.module(krate, &[named.first, second])).or(Some(file)),
            None => again[krate].get(named.first).or_else(|| self.crates[krate].files.get("lib.rs")).copied(),
        }
    }

    /// The names the main file of each crate gives again from one of its modules (`pub use murs::{Wall, Legende};`):
    /// name → the file of that module.
    fn given_again(&self) -> Vec<HashMap<String, usize>> {
        let again = |(k, krate): (usize, &Crate)| {
            let mut names = HashMap::new();
            let Some(f) = krate.files.get("lib.rs").map(|id| &self.files[*id]) else {
                return names;
            };
            let starts = |i: &usize| f.word(*i) == Some("pub") && f.word(i + 1) == Some("use") && f.path(i + 3);
            for i in (0..f.toks.len()).filter(starts) {
                let Some(module) = f.word(i + 2).and_then(|module| self.module(k, &[module])) else {
                    continue;
                };
                if f.is(i + 4, b'{') {
                    let (close, parts) = f.group(i + 4);
                    for (from, to) in parts.into_iter().filter(|_| f.is(close + 1, b';')) {
                        let name = match to - from {
                            1 => f.word(from),
                            3 if f.word(from + 1) == Some("as") => f.word(from + 2),
                            _ => None,
                        };
                        names.extend(name.map(|name| (name.to_string(), module)));
                    }
                } else if let Some(name) = f.word(i + 4).filter(|_| f.is(i + 5, b';')) {
                    names.insert(name.to_string(), module);
                }
            }
            names
        };
        self.crates.iter().enumerate().map(again).collect()
    }

    /// The path of the module above the one a file is (`a/b.rs` and `a/b/mod.rs`: `["a"]`; `a.rs`: nothing, the
    /// top of the crate).
    fn above<'a>(&self, f: &'a File) -> Vec<&'a str> {
        let mut path: Vec<&str> = f.rel.trim_end_matches(".rs").split('/').collect();
        if path.last() == Some(&"mod") {
            path.pop();
        }
        path.pop();
        path
    }

    /// The modules a file is the parent of: name → their file.
    fn children(&self, id: usize) -> HashMap<&str, usize> {
        let f = &self.files[id];
        let base = match f.rel.as_str() {
            "lib.rs" | "main.rs" => String::new(),
            rel => rel
                .strip_suffix("/mod.rs")
                .map_or(format!("{}/", rel.trim_end_matches(".rs")), |folder| format!("{folder}/")),
        };
        let mut children = HashMap::new();
        for (rel, file) in self.crates[f.krate].files.iter().filter(|(_, file)| **file != id) {
            let name = rel.strip_prefix(&base).and_then(|rest| match rest.split_once('/') {
                None => rest.strip_suffix(".rs"),
                Some((name, "mod.rs")) => Some(name),
                Some(_) => None,
            });
            children.extend(name.map(|name| (name, *file)));
        }
        children
    }

    /// What the main file of the app brings in for all its files: the modules (`live`, `wl`) and the names
    /// (`Chart`, `Watchlist`), each with the file it comes from.
    fn brought_in(&self, app: usize, again: &[HashMap<String, usize>]) -> (HashMap<&str, usize>, HashMap<&str, usize>) {
        let (mut modules, mut names) = (HashMap::new(), HashMap::new());
        let Some(main) = self.crates[app].files.get("main.rs").map(|id| &self.files[*id]) else {
            return (modules, names);
        };
        for import in main.imports().into_iter().filter(|import| import.top) {
            let from = |k: &usize| *k != app && import.path.first() == Some(&self.crates[*k].word.as_str());
            let (Some(krate), Some(last)) = ((0..self.crates.len()).find(from), import.path.last()) else {
                continue;
            };
            let (path, name) = (&import.path[1..], import.alias.unwrap_or(last));
            let Some(first) = path.first() else {
                continue;
            };
            match self.module(krate, path) {
                Some(file) => modules.insert(name, file),
                None => {
                    let named = Named {
                        line: 0,
                        first,
                        second: path.get(1).copied(),
                    };
                    self.target(krate, &named, again).and_then(|file| names.insert(name, file))
                }
            };
        }
        (modules, names)
    }

    /// The functions of `App` that are written in one file only: name → that file.
    fn functions_of_app(&self, app: usize) -> HashMap<&str, usize> {
        let mut written: HashMap<&str, BTreeSet<usize>> = HashMap::new();
        for (f, id) in self.crates[app].files.values().map(|id| (&self.files[*id], *id)) {
            for (open, close) in f.app_blocks().into_iter().filter(|_| f.home != place(TESTS)) {
                for name in (open..close).filter(|i| f.word(*i) == Some("fn")).filter_map(|i| f.word(i + 1)) {
                    written.entry(name).or_default().insert(id);
                }
            }
        }
        let in_one_file = written.into_iter().filter(|(_, files)| files.len() == 1);
        in_one_file.filter_map(|(name, files)| Some((name, *files.first()?))).collect()
    }

    /// Every place where a file names another file.
    fn links(&self) -> Vec<Link> {
        let again = self.given_again();
        let app = self.crates.iter().position(|k| k.name == APP);
        let (modules, names) = app.map(|app| self.brought_in(app, &again)).unwrap_or_default();
        let functions = app.map(|app| self.functions_of_app(app)).unwrap_or_default();
        let mut found = Found {
            world: self,
            links: Vec::new(),
        };
        for (id, f) in self.files.iter().enumerate() {
            let (k, inner, above) = (f.krate, f.inner(), self.above(f));
            // 1. a path: `crate::a::b`, `other_crate::a::b`, `super::a`
            for named in f.named("crate", None) {
                found.add(id, self.target(k, &named, &again), named.line, named.name(), "path");
            }
            for (other, krate) in self.crates.iter().enumerate().filter(|(other, _)| *other != k) {
                for named in f.named(&krate.word, None) {
                    found.add(id, self.target(other, &named, &again), named.line, named.name(), "path");
                }
            }
            for named in f.named("super", Some(&inner)) {
                // (at the top of the crate, `super` is the crate; lower, it is the module above)
                let to = match above.as_slice() {
                    [] => self.target(k, &named, &again),
                    above => self.module(k, &[above, &[named.first]].concat()).or_else(|| self.module(k, above)),
                };
                found.add(id, to, named.line, named.name(), "path");
            }
            let all = |i: &usize| f.word(*i) == Some("use") && f.word(i + 1) == Some("super") && f.path(i + 2) && f.is(i + 3, b'*');
            if let Some(i) = (0..f.toks.len()).find(|i| !inner[*i] && all(i)).filter(|_| !above.is_empty()) {
                found.add(id, self.module(k, &above), f.toks[i].line, "*", "path");
            }
            // 2. a child module named by its parent: `child::Name`, `use child::{A, B}`, `pub use child::Name`
            let children = self.children(id);
            // 3. a module brought in under its name by a `use`, then used: `murs::Legende`
            let mut brought: HashMap<&str, usize> = HashMap::new();
            for import in f.imports() {
                let Some((first, path)) = import.path.split_first() else {
                    continue;
                };
                let module = match *first {
                    "crate" => self.module(k, path),
                    "super" => self.module(k, &[above.as_slice(), path].concat()),
                    krate => (0..self.crates.len())
                        .find(|o| *o != k && self.crates[*o].word == krate)
                        .and_then(|o| self.module(o, path)),
                };
                let name = import.alias.or(path.last().copied());
                if let (Some(module), Some(name)) = (module.filter(|module| *module != id), name) {
                    brought.insert(name, module);
                }
            }
            if Some(k) == app {
                brought.retain(|name, _| !modules.contains_key(name));
            }
            for i in 0..f.toks.len() {
                let (Some(word), line) = (f.word(i), f.toks[i].line) else {
                    continue;
                };
                let after = f.word(i + 2).filter(|_| f.path(i + 1));
                let alone = !f.glued(i, false);
                if let Some(child) = children
                    .get(word)
                    .filter(|_| alone && (after.is_some() || (f.path(i + 1) && f.is(i + 2, b'{'))))
                {
                    let given_again = i > 0 && f.word(i - 1) == Some("use") && {
                        let scoped = i > 4 && f.is(i - 2, b')') && f.word(i - 3).is_some() && f.is(i - 4, b'(') && f.word(i - 5) == Some("pub");
                        (i > 1 && f.word(i - 2) == Some("pub")) || scoped
                    };
                    if given_again {
                        found.add(id, Some(*child), line, "pub use", "child");
                    } else if f.is(i + 2, b'{') {
                        for (from, _) in f.group(i + 2).1 {
                            if let Some(name) = f.word(from).filter(|name| *name != "self") {
                                found.add(id, Some(*child), line, name, "child");
                            }
                        }
                    } else if let Some(name) = after {
                        found.add(id, Some(*child), line, name, "child");
                    }
                }
                if let (Some(module), Some(name)) = (brought.get(word).filter(|_| alone), after) {
                    found.add(id, Some(*module), line, name, "module");
                }
                if Some(k) != app {
                    continue;
                }
                // 4. the app: what its main file brings in for all its files
                if f.rel != "main.rs" {
                    if let Some(module) = modules.get(word).filter(|_| alone && f.path(i + 1)) {
                        found.add(id, Some(*module), line, after.unwrap_or(word), "brought in");
                    }
                    if let Some(file) = names.get(word).filter(|_| !COMMON.contains(&word) && !f.glued(i, true)) {
                        found.add(id, Some(*file), line, word, "brought in");
                    }
                }
                // 5. the app: a function of `App` written in another file, a function of one of its folders
                let call = |name: Option<&'_ str>| name.and_then(|name| functions.get(name)).copied();
                if APP_NAMES.contains(&word) && f.is(i + 1, b'.') && f.is(i + 3, b'(') {
                    found.add(id, call(f.word(i + 2)), line, f.word(i + 2).unwrap_or_default(), "call");
                }
                if matches!(word, "Self" | "App") && f.path(i + 1) && f.is(i + 3, b'(') {
                    found.add(id, call(f.word(i + 2)), line, f.word(i + 2).unwrap_or_default(), "call");
                }
                let lower = (f.path(i + 3), f.word(i + 4));
                if let (true, Some(module), (true, Some(name))) = (APP_FOLDERS.contains(&word), after, lower) {
                    let whole = i > 1 && f.path(i - 1) && f.word(i - 2) == Some("crate");
                    if !f.glued(if whole { i - 2 } else { i }, false) {
                        found.add(id, self.crates[k].files.get(&format!("{word}/{module}.rs")).copied(), line, name, "call");
                    }
                }
                if let (true, true, Some(module), (true, Some(name))) = (word == "super", f.rel.starts_with("actions/"), after, lower) {
                    if alone {
                        found.add(id, self.crates[k].files.get(&format!("actions/{module}.rs")).copied(), line, name, "call");
                    }
                }
            }
        }
        found.links
    }
}

// ---------------------------------------------------------------------------------------------------------------
// The map, the workspace on the disk
// ---------------------------------------------------------------------------------------------------------------

/// The map: its lines (a path under the folder of the workspace, the place of its box in [`BOXES`]).
struct Map(Vec<(String, usize)>);

impl Map {
    fn read(text: &str) -> Result<Map, String> {
        let mut lines: Vec<(String, usize)> = Vec::new();
        for line in text.lines().map(str::trim).filter(|line| !line.is_empty() && !line.starts_with('#')) {
            let (Some(path), Some(home), None) = (line.split(' ').next(), line.split(' ').nth(1).and_then(place), line.split(' ').nth(2)) else {
                return Err(format!("the map has a line that is not `path box`, or with a box that does not exist: {line}"));
            };
            if lines.iter().any(|(written, _)| written == path) {
                return Err(format!("the map has two lines for {path}"));
            }
            lines.push((path.to_string(), home));
        }
        Ok(Map(lines))
    }

    /// The line that holds for a file: its own, or the closest folder above it.
    fn line(&self, path: &str) -> Option<usize> {
        let holds = |(_, (written, _)): &(usize, &(String, usize))| written == path || (written.ends_with('/') && path.starts_with(written.as_str()));
        self.0
            .iter()
            .enumerate()
            .filter(holds)
            .max_by_key(|(_, (written, _))| written.len())
            .map(|(line, _)| line)
    }
}

/// The folder of the workspace (`rust/`): the first one, from this crate up, whose `Cargo.toml` lists members.
fn workspace() -> PathBuf {
    let mut folder = Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf();
    while !std::fs::read_to_string(folder.join("Cargo.toml")).is_ok_and(|toml| toml.contains("[workspace]")) {
        assert!(folder.pop(), "no workspace above this crate");
    }
    folder
}

/// The source files under a folder, with their path from it (`actions/load.rs`).
fn sources(folder: &Path, under: &str, out: &mut Vec<(String, PathBuf)>) {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(folder).into_iter().flatten().flatten().map(|e| e.path()).collect();
    entries.sort();
    for path in entries {
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        if path.is_dir() {
            sources(&path, &format!("{under}{name}/"), out);
        } else if name.ends_with(".rs") {
            out.push((format!("{under}{name}"), path));
        }
    }
}

impl World {
    /// The crates of the workspace as they are on the disk: the members its `Cargo.toml` lists, their `src/`.
    fn on_the_disk(map: &Map) -> World {
        let workspace = workspace();
        let toml = std::fs::read_to_string(workspace.join("Cargo.toml")).expect("the Cargo.toml of the workspace");
        let listed = toml
            .lines()
            .find(|line| line.trim_start().starts_with("members"))
            .expect("the members of the workspace");
        let mut folders: Vec<String> = Vec::new();
        for member in listed.split('"').skip(1).step_by(2) {
            match member.strip_suffix("/*") {
                // (`crates/*`: every folder there that is a crate)
                Some(folder) => {
                    let inside = std::fs::read_dir(workspace.join(folder)).into_iter().flatten().flatten();
                    let crates = inside.filter(|e| e.path().join("Cargo.toml").is_file());
                    folders.extend(crates.map(|e| format!("{folder}/{}", e.file_name().to_string_lossy())));
                }
                None => folders.push(member.to_string()),
            }
        }
        folders.sort();
        let (mut crates, mut files, mut tomls) = (Vec::new(), Vec::new(), Vec::new());
        for folder in &folders {
            let toml = std::fs::read_to_string(workspace.join(folder).join("Cargo.toml")).unwrap_or_else(|e| panic!("{folder}/Cargo.toml: {e}"));
            let package = toml.lines().skip_while(|line| line.trim() != "[package]");
            let name = package
                .filter_map(|line| line.trim().strip_prefix("name")?.trim_start().strip_prefix('='))
                .next();
            let name = name
                .unwrap_or_else(|| panic!("{folder}/Cargo.toml has no name"))
                .trim()
                .trim_matches('"')
                .to_string();
            let mut found = Vec::new();
            sources(&workspace.join(folder).join("src"), "", &mut found);
            let mut of_the_crate = BTreeMap::new();
            for (rel, path) in found {
                let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
                let path = format!("{folder}/src/{rel}");
                of_the_crate.insert(rel.clone(), files.len());
                files.push(File {
                    krate: crates.len(),
                    rel,
                    home: map.line(&path).map(|line| map.0[line].1),
                    path,
                    toks: read(&text),
                    text,
                });
            }
            crates.push(Crate {
                word: name.replace('-', "_"),
                name,
                files: of_the_crate,
                needs: BTreeSet::new(),
            });
            tomls.push(toml);
        }
        // (a crate needs another one: its `Cargo.toml` has a line `other = …`, or a table `[dependencies.other]`)
        let names: Vec<String> = crates.iter().map(|k: &Crate| k.name.clone()).collect();
        for (krate, toml) in crates.iter_mut().zip(&tomls) {
            let named = |line: &str| {
                let table = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')).and_then(|l| l.rsplit('.').next());
                let entry = line.split_once('=').map(|(name, _)| name.trim().trim_matches('"'));
                let name = table.or(entry)?.split('.').next()?.to_string();
                names.contains(&name).then_some(name)
            };
            krate.needs = toml.lines().map(str::trim).filter_map(named).filter(|name| *name != krate.name).collect();
        }
        World { crates, files }
    }

    /// A made-up workspace, to show what the reading sees: (crate, path under its `src/`, box, text).
    fn made_up(sources: &[(&str, &str, &str, &str)]) -> World {
        let (mut crates, mut files): (Vec<Crate>, Vec<File>) = (Vec::new(), Vec::new());
        for (krate, rel, home, text) in sources {
            let k = crates.iter().position(|k| k.name == *krate).unwrap_or_else(|| {
                crates.push(Crate {
                    name: krate.to_string(),
                    word: krate.replace('-', "_"),
                    files: BTreeMap::new(),
                    needs: BTreeSet::new(),
                });
                crates.len() - 1
            });
            crates[k].files.insert(rel.to_string(), files.len());
            files.push(File {
                krate: k,
                rel: rel.to_string(),
                path: format!("{krate}/{rel}"),
                home: place(home),
                text: text.to_string(),
                toks: read(text),
            });
        }
        World { crates, files }
    }
}

fn map() -> &'static Map {
    static MAP: OnceLock<Map> = OnceLock::new();
    MAP.get_or_init(|| Map::read(include_str!("map.txt")).unwrap_or_else(|e| panic!("{e}")))
}

fn world() -> &'static World {
    static WORLD: OnceLock<World> = OnceLock::new();
    WORLD.get_or_init(|| World::on_the_disk(map()))
}

/// The source files of the app (every crate of the workspace, without its tests): their path and their text.
pub(super) fn code_of_the_app() -> impl Iterator<Item = (&'static str, &'static str)> {
    let tests = |f: &&File| f.rel.split('/').any(|part| part.starts_with("tests"));
    world().files.iter().filter(move |f| !tests(f)).map(|f| (f.path.as_str(), f.text.as_str()))
}

// ---------------------------------------------------------------------------------------------------------------
// The counts and the ratchet
// ---------------------------------------------------------------------------------------------------------------

/// What is kept: for each pair of boxes the plan does not let through, how many links; and the crates each crate
/// needs.
#[derive(Default, PartialEq, Eq, Debug)]
struct Counts {
    pairs: BTreeMap<(usize, usize), u32>,
    crates: BTreeSet<(String, String)>,
}

const KEPT_HEAD: &str = "\
# The boundaries between the boxes of the map (safety net of the split into modules, AIP-0001).
#
# `A -> B 12`: 12 places where a file of the box A names a file of the box B, although the plan does not let A use
# B directly. These are the links still to cut: a number here only goes down.
# `crate x -> y`: the crate x needs the crate y (its Cargo.toml says so).
#
# ARGOS_REFERENCES=maj writes the numbers that went down, and never one that went up. A number that has to go up
# (a piece that goes back to its module takes its links with it; a feature merged before its module is cut out
# brings its own) is written here by hand, and the report of the lot says why: the test gives the lines to write.
# The boxes and what counts as a link: src/tests/safety_net/boundaries.rs and its map.txt.
";

impl Counts {
    fn of(world: &World, links: &[Link]) -> Counts {
        let mut counts = Counts::default();
        for link in links {
            if let (Some(from), Some(to)) = (world.files[link.from].home, world.files[link.to].home) {
                if !free(BOXES[from].0, BOXES[to].0) {
                    *counts.pairs.entry((from, to)).or_default() += 1;
                }
            }
        }
        let needs = |krate: &Crate| krate.needs.iter().map(|need| (krate.name.clone(), need.clone())).collect::<Vec<_>>();
        counts.crates = world.crates.iter().flat_map(needs).collect();
        counts
    }

    fn read(text: &str) -> Result<Counts, String> {
        let mut counts = Counts::default();
        for line in text.lines().map(str::trim).filter(|line| !line.is_empty() && !line.starts_with('#')) {
            let words: Vec<&str> = line.split(' ').collect();
            match words.as_slice() {
                ["crate", from, "->", to] => {
                    counts.crates.insert((from.to_string(), to.to_string()));
                }
                [from, "->", to, links] => match (place(from), place(to), links.parse()) {
                    (Some(from), Some(to), Ok(links)) => {
                        counts.pairs.insert((from, to), links);
                    }
                    _ => return Err(format!("a box that does not exist, or a number that is not one: {line}")),
                },
                _ => return Err(format!("neither `A -> B 12` nor `crate x -> y`: {line}")),
            }
        }
        Ok(counts)
    }

    fn write(&self) -> String {
        let mut text = format!("{KEPT_HEAD}\n");
        for ((from, to), links) in self.pairs.iter().filter(|(_, links)| **links > 0) {
            let _ = writeln!(text, "{} -> {} {links}", BOXES[*from].0, BOXES[*to].0);
        }
        text.push('\n');
        for (from, to) in &self.crates {
            let _ = writeln!(text, "crate {from} -> {to}");
        }
        text
    }
}

/// `J Chart -> H Argos Script`
fn pair((from, to): (usize, usize)) -> String {
    format!("{} {} -> {} {}", BOXES[from].0, BOXES[from].1, BOXES[to].0, BOXES[to].1)
}

/// Where the links of a pair of boxes are today: file to file, the most links first.
fn places(world: &World, links: &[Link], of: (usize, usize)) -> String {
    let mut by_files: BTreeMap<(&str, &str), u32> = BTreeMap::new();
    for link in links {
        let (from, to) = (&world.files[link.from], &world.files[link.to]);
        if (from.home, to.home) == (Some(of.0), Some(of.1)) {
            *by_files.entry((&from.path, &to.path)).or_default() += 1;
        }
    }
    let mut by_files: Vec<_> = by_files.into_iter().collect();
    by_files.sort_by_key(|(files, links)| (std::cmp::Reverse(*links), *files));
    let more = by_files.len().saturating_sub(6);
    let mut text: String = by_files
        .iter()
        .take(6)
        .map(|((from, to), links)| format!("\n      {from} -> {to}: {links}"))
        .collect();
    if more > 0 {
        let _ = write!(text, "\n      (and {more} other pairs of files)");
    }
    text
}

/// Every link, one per line: the file and its line, its box, the file it names, its box, the name, the kind of
/// link, and whether the plan lets it through.
fn listing(world: &World, links: &[Link]) -> String {
    let mut lines: Vec<String> = Vec::new();
    for link in links {
        let (from, to) = (&world.files[link.from], &world.files[link.to]);
        let (Some(a), Some(b)) = (from.home, to.home) else {
            continue;
        };
        let through = if free(BOXES[a].0, BOXES[b].0) { "free" } else { "to cut" };
        let (a, b) = (BOXES[a].0, BOXES[b].0);
        lines.push(format!(
            "{}:{}\t{a}\t{}\t{b}\t{}\t{}\t{through}\n",
            from.path, link.line, to.path, link.name, link.by
        ));
    }
    lines.sort();
    lines.concat()
}

/// 🕸 Every source file of the workspace has its box on the map, and every line of the map still has its files.
#[test]
fn every_source_file_is_on_the_map() {
    let (world, map) = (world(), map());
    let lost: Vec<&str> = world.files.iter().filter(|f| f.home.is_none()).map(|f| f.path.as_str()).collect();
    assert!(
        lost.is_empty(),
        "{} source file(s) without a box. Each one gets a line in src/tests/safety_net/map.txt, in the box of the module it is part of:\n  {}",
        lost.len(),
        lost.join("\n  ")
    );
    let used: BTreeSet<usize> = world.files.iter().filter_map(|f| map.line(&f.path)).collect();
    let empty: Vec<&str> = (0..map.0.len())
        .filter(|line| !used.contains(line))
        .map(|line| map.0[line].0.as_str())
        .collect();
    assert!(
        empty.is_empty(),
        "{} line(s) of src/tests/safety_net/map.txt without a file (moved, renamed or removed: the line follows the file):\n  {}",
        empty.len(),
        empty.join("\n  ")
    );
}

/// 🕸 No boundary is crossed more than before: the links the plan does not let through are counted, pair of boxes by
/// pair of boxes, and a count only goes down. The same for the crates each crate needs.
#[test]
fn no_boundary_is_crossed_more_than_before() {
    let world = world();
    assert!(
        world.files.iter().all(|f| f.home.is_some()),
        "a source file has no box: see `every_source_file_is_on_the_map`"
    );
    let links = world.links();
    if let Some(file) = std::env::var_os("ARGOS_LIENS") {
        std::fs::write(&file, listing(world, &links)).unwrap_or_else(|e| panic!("{}: {e}", Path::new(&file).display()));
    }
    let now = Counts::of(world, &links);
    let between = links.iter().filter(|link| world.files[link.from].home != world.files[link.to].home).count();
    let (to_cut, pairs) = (now.pairs.values().sum::<u32>(), now.pairs.len());
    println!(
        "🕸 safety net: {} links, {between} from one box to another, {to_cut} of them to cut ({pairs} pairs of boxes)",
        links.len()
    );
    let path = super::references_folder().join("safety-net-boundaries.txt");
    let kept = match std::fs::read_to_string(&path) {
        Ok(text) => Counts::read(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display())),
        // (the first time: everything is written as it is)
        Err(_) if super::updating() => return std::fs::write(&path, now.write()).expect("the boundaries are written"),
        Err(e) => panic!("{}: {e}. ARGOS_REFERENCES=maj writes the boundaries as they are today", path.display()),
    };
    let both: BTreeSet<(usize, usize)> = kept.pairs.keys().chain(now.pairs.keys()).copied().collect();
    let count = |counts: &Counts, of: &(usize, usize)| counts.pairs.get(of).copied().unwrap_or(0);
    let (mut up, mut down, mut by_hand) = (Vec::new(), Vec::new(), Vec::new());
    for of in &both {
        let (before, after) = (count(&kept, of), count(&now, of));
        if after > before {
            up.push(format!("\n  {}: {before} kept, {after} now{}", pair(*of), places(world, &links, *of)));
            by_hand.push(format!("\n  {} -> {} {after}", BOXES[of.0].0, BOXES[of.1].0));
        } else if after < before {
            down.push(format!("\n  {}: {before} kept, {after} now", pair(*of)));
        }
    }
    let crates = |a: &BTreeSet<(String, String)>, b: &BTreeSet<(String, String)>| -> Vec<String> {
        a.difference(b).map(|(from, to)| format!("\n  crate {from} -> {to}")).collect()
    };
    let (more, fewer) = (crates(&now.crates, &kept.crates), crates(&kept.crates, &now.crates));
    if super::updating() {
        // (what went down is written; what went up never is)
        let lower = both.iter().map(|of| (*of, count(&now, of).min(count(&kept, of))));
        let written = Counts {
            pairs: lower.collect(),
            crates: kept.crates.intersection(&now.crates).cloned().collect(),
        };
        std::fs::write(&path, written.write()).expect("the boundaries are written");
    }
    let file = "tests/references/safety-net-boundaries.txt";
    assert!(
        up.is_empty() && more.is_empty(),
        "{} boundary count(s) went up: new links between boxes that the plan does not let through. A module uses the kit, the shared data, \
         the base types and the modules it declares it needs; the rest goes through the door.{}{}\n(to find the new links: ARGOS_LIENS=<file> \
         before the change and after it, then compare the two files. A count that has to go up is written by hand in {file}, and the report \
         of the lot says why. Its line(s) as that file writes them:{}{})",
        up.len() + more.len(),
        up.concat(),
        more.concat(),
        by_hand.concat(),
        more.concat()
    );
    assert!(
        super::updating() || (down.is_empty() && fewer.is_empty()),
        "{} boundary count(s) went down: good. Write them (ARGOS_REFERENCES=maj) and commit {file}, so that they cannot go back up:{}{}",
        down.len() + fewer.len(),
        down.concat(),
        fewer.concat()
    );
}

/// A small made-up workspace: one example of each kind of link, and of what is not one.
const MADE_UP: &[(&str, &str, &str, &str)] = &[
    (
        "argos-render",
        "lib.rs",
        "J",
        "pub mod murs; pub mod text; mod kid;
         pub use text::{wrap, Font as Police};
         pub use kid::Thing;
         pub struct Chart;
         fn draw() { kid::go(); murs::Wall::new(); }",
    ),
    (
        "argos-render",
        "murs.rs",
        "A",
        "pub mod legende; pub mod autre;
         use crate::text::Font;
         use super::wrap;
         use crate::{kid, text::measure};
         use argos_data::{live, Exchange};
         // crate::text::in_a_comment /* and crate::text::in_another */
         const TEXT: &str = \"crate::text::in_a_text\";
         const RAW: &str = r#\"crate::text::in_a_raw \"text\"\"#;
         const QUOTE: char = '\"';
         pub struct Wall;
         fn f<'a>(x: &'a str) { kid::go(); crate::Chart; live::start(); }
         mod tests { use super::*; fn t() { super::f(\"\"); } }",
    ),
    (
        "argos-render",
        "murs/legende.rs",
        "A",
        "use super::{Wall, autre};
         use super::*;
         fn g() { super::autre::h(); autre::h(); }",
    ),
    ("argos-render", "murs/autre.rs", "B", "pub fn h() {}"),
    ("argos-render", "text.rs", "KIT", "pub fn wrap() {} pub struct Font; pub fn measure() {}"),
    ("argos-render", "kid.rs", "KIT", "pub fn go() {} pub struct Thing;"),
    ("argos-data", "lib.rs", "DON", "pub mod live; pub struct Exchange;"),
    ("argos-data", "live.rs", "DON", "pub struct LiveEvent; pub fn start() {}"),
    (
        "argos-desktop",
        "main.rs",
        "COQ",
        "mod app; mod actions; mod tests;
use app::App;
use argos_data::live::{self, LiveEvent};
use argos_render::murs::Wall;
use argos_render::text as txt;
use argos_render::Chart;
fn main() {}",
    ),
    (
        "argos-desktop",
        "app.rs",
        "COQ",
        "use crate::*;
         pub struct App;
         impl App {
             pub fn redraw(&mut self) {}
             pub fn width(&self) -> u32 { 0 }
             pub fn keep(name: &str) {}
         }",
    ),
    ("argos-desktop", "actions/mod.rs", "COQ", "pub mod load; pub mod murs;"),
    (
        "argos-desktop",
        "actions/murs.rs",
        "A",
        "use crate::*;
         impl App {
             pub fn murs_open(&mut self) {
                 self.redraw();
                 let wall: Wall = Wall;
                 live::start();
                 txt::wrap();
                 let event = LiveEvent;
                 let chart = Chart;
                 super::load::open(self);
                 crate::actions::load::open(self);
                 App::keep(\"x\");
                 self
                     .width();
                 self.murs_close();
                 other.redraw();
             }
             fn murs_close(&mut self) {}
         }",
    ),
    (
        "argos-desktop",
        "actions/load.rs",
        "J",
        "use crate::*; pub fn open(a: &mut App) { a.murs_open(); }",
    ),
    (
        "argos-desktop",
        "tests.rs",
        TESTS,
        "use crate::*; fn t(a: &mut App) { a.redraw(); crate::actions::load::open(a); }",
    ),
];

/// 🕸 What the reading sees: each kind of link once, in a made-up workspace. Not seen: what is in a comment or
/// between quotes, `super` inside a module written in the file itself, a call on something that is not an `App`,
/// a common name (`Chart`), a file that names itself, and everything the tests of the whole app name.
#[test]
fn the_reading_sees_each_kind_of_link() {
    let world = World::made_up(MADE_UP);
    let seen = |link: &Link| format!("{} > {} {} ({})", world.files[link.from].path, world.files[link.to].path, link.name, link.by);
    let mut links: Vec<String> = world.links().iter().map(seen).collect();
    links.sort();
    let expected = [
        "argos-desktop/actions/load.rs > argos-desktop/actions/murs.rs murs_open (call)",
        "argos-desktop/actions/murs.rs > argos-data/live.rs LiveEvent (brought in)",
        "argos-desktop/actions/murs.rs > argos-data/live.rs start (brought in)",
        "argos-desktop/actions/murs.rs > argos-desktop/actions/load.rs actions::load (path)",
        "argos-desktop/actions/murs.rs > argos-desktop/actions/load.rs load::open (path)",
        "argos-desktop/actions/murs.rs > argos-desktop/actions/load.rs open (call)",
        "argos-desktop/actions/murs.rs > argos-desktop/actions/load.rs open (call)",
        "argos-desktop/actions/murs.rs > argos-desktop/app.rs keep (call)",
        "argos-desktop/actions/murs.rs > argos-desktop/app.rs redraw (call)",
        "argos-desktop/actions/murs.rs > argos-desktop/app.rs width (call)",
        "argos-desktop/actions/murs.rs > argos-render/murs.rs Wall (brought in)",
        "argos-desktop/actions/murs.rs > argos-render/murs.rs Wall (brought in)",
        "argos-desktop/actions/murs.rs > argos-render/text.rs wrap (brought in)",
        "argos-desktop/main.rs > argos-data/live.rs live (path)",
        "argos-desktop/main.rs > argos-desktop/app.rs App (child)",
        "argos-desktop/main.rs > argos-render/lib.rs Chart (path)",
        "argos-desktop/main.rs > argos-render/murs.rs murs::Wall (path)",
        "argos-desktop/main.rs > argos-render/text.rs text (path)",
        "argos-render/lib.rs > argos-render/kid.rs go (child)",
        "argos-render/lib.rs > argos-render/kid.rs pub use (child)",
        "argos-render/lib.rs > argos-render/murs.rs Wall (child)",
        "argos-render/lib.rs > argos-render/text.rs pub use (child)",
        "argos-render/murs.rs > argos-data/lib.rs Exchange (path)",
        "argos-render/murs.rs > argos-data/live.rs live (path)",
        "argos-render/murs.rs > argos-data/live.rs start (module)",
        "argos-render/murs.rs > argos-render/kid.rs go (module)",
        "argos-render/murs.rs > argos-render/kid.rs kid (path)",
        "argos-render/murs.rs > argos-render/lib.rs Chart (path)",
        "argos-render/murs.rs > argos-render/text.rs text::Font (path)",
        "argos-render/murs.rs > argos-render/text.rs text::measure (path)",
        "argos-render/murs.rs > argos-render/text.rs wrap (path)",
        "argos-render/murs/legende.rs > argos-render/murs.rs * (path)",
        "argos-render/murs/legende.rs > argos-render/murs.rs Wall (path)",
        "argos-render/murs/legende.rs > argos-render/murs/autre.rs autre (path)",
        "argos-render/murs/legende.rs > argos-render/murs/autre.rs autre::h (path)",
        "argos-render/murs/legende.rs > argos-render/murs/autre.rs h (module)",
    ];
    assert_eq!(links, expected, "the links read in the made-up workspace:\n{}", links.join("\n"));
    // (counted: only what the plan does not let through. The Carnet may use the kit and the shared data; it may not
    // use the Flux d'ordres, the Chart or the shell without the door)
    let counts = Counts::of(&world, &world.links());
    let counted: Vec<String> = counts
        .pairs
        .iter()
        .map(|((from, to), links)| format!("{} -> {} {links}", BOXES[*from].0, BOXES[*to].0))
        .collect();
    assert_eq!(counted, ["A -> B 3", "A -> J 5", "A -> COQ 3", "J -> A 2", "COQ -> A 1", "COQ -> J 1"]);
    assert_eq!(Counts::read(&counts.write()), Ok(counts), "the counts are read as they are written");
}

/// 🕸 What the plan lets through: the kit, the shared data and the base types for everybody above them, the needs a
/// module declares, and nothing else.
#[test]
fn the_plan_lets_through_what_it_says() {
    for (from, to) in [
        ("A", "KIT"),
        ("A", "DON"),
        ("A", "BASE"),
        ("N", "L"),
        ("M", "J"),
        ("COQ", "MIX"),
        ("MIX", "COQ"),
        ("COQ", "KIT"),
        ("KIT", "DON"),
        ("DON", "BASE"),
    ] {
        assert!(free(from, to), "{from} may use {to}");
    }
    for (from, to) in [
        ("L", "N"),
        ("A", "J"),
        ("A", "COQ"),
        ("A", "MIX"),
        ("COQ", "A"),
        ("MIX", "W"),
        ("KIT", "J"),
        ("DON", "KIT"),
        ("DON", "B"),
        ("BASE", "DON"),
        ("A", TESTS),
    ] {
        assert!(!free(from, to), "{from} may not use {to} without the door");
    }
    for (module, needs) in NEEDS {
        let known = |name: &&str| place(name).is_some();
        assert!(known(module) && needs.iter().all(known), "the needs of {module} are boxes of the map");
    }
}
