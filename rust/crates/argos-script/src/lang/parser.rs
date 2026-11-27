//! 🧩 Lecture des jetons en arbre (syntaxe Pine v6, plus les lambdas d'Argos).
//! Priorités, de la plus faible à la plus forte : `?:`, `or`, `and`, `== !=`, `< <= > >=`, `+ -`, `* / %`,
//! unaires `not - +`, puis appel `f()`, membre `a.b` et historique `x[n]`.

use super::ast::*;
use super::lexer::{Error, Tok, T};

/// Types qu'on peut écrire devant une déclaration ou un paramètre (ignorés : Argos Script est typé à l'exécution).
const TYPES: [&str; 13] = [
    "int", "float", "bool", "string", "color", "label", "line", "box", "table", "array", "matrix", "map", "polyline",
];
const QUALIFIERS: [&str; 4] = ["series", "simple", "const", "input"];

/// The most levels the parser goes down at once: a parenthesis in a parenthesis, a call in an argument, a block in
/// a block (a block in a block is two levels). The right side of a statement is one level itself, so what a script
/// may write is less than this number (99 parentheses, 49 blocks), as the message says. Each level is a dozen calls of the parser, 2.2 KB of stack measured
/// in a web build, and the web app has 1 MB of stack with no way to catch its end: 476 parentheses, one in the
/// other, stopped the whole app (B-074). The scripts of the repository go down 21 levels at most.
pub(super) const MAX_NIVEAUX: u32 = 100;

/// The most levels a statement holds from its top to its deepest value. What is chained without a parenthesis counts
/// too: `a + b + c…` is a tree as deep as it is long, and everything that reads the tree afterwards (the
/// interpreter, the freeing of the tree) goes down once per level: a sum of 1 344 terms stopped the whole app. The
/// first terms of a chain are the deepest, so what they hold counts too: 249 names in a row are read, 248 when the
/// first ones are calculations, calls, members or `[n]` (`2 * close`, `close > 0` in a chain of `and`, `nz(close)`,
/// `syminfo.mintick`, `close[1]`), 247 for `math.abs(-1)`. The
/// tallest tree of the scripts of the repository has 25 levels.
pub(super) const MAX_HAUTEUR: u32 = 250;

#[cfg(test)]
thread_local! {
    /// Test hook: the most levels down and the tallest tree of the scripts read on this thread.
    static PIC: std::cell::Cell<(u32, u32)> = const { std::cell::Cell::new((0, 0)) };
}

/// Test hook: the most levels down and the tallest tree since the last call, and back to zero.
#[cfg(test)]
pub(super) fn pic() -> (u32, u32) {
    PIC.with(|p| p.replace((0, 0)))
}

pub struct Parser {
    toks: Vec<Tok>,
    i: usize,
    ids: u32,
    /// The names read with `name[n]` so far, and how far back (`Script::historiques`).
    historiques: std::collections::HashMap<String, usize>,
    /// How many levels down the parser is (`MAX_NIVEAUX`).
    prof: u32,
    /// The height of the tallest tree read since the last `debut` (`MAX_HAUTEUR`).
    haut: u32,
}

pub fn parse(toks: Vec<Tok>) -> Result<Script, Error> {
    let mut p = Parser {
        toks,
        i: 0,
        ids: 0,
        historiques: Default::default(),
        prof: 0,
        haut: 0,
    };
    let mut body = vec![];
    p.skip_newlines();
    while !p.at(&T::Eof) {
        p.stmts(&mut body)?;
        p.skip_newlines();
    }
    Ok(Script {
        body,
        ids: p.ids,
        historiques: p.historiques,
    })
}

impl Parser {
    fn peek(&self) -> &T {
        &self.toks[self.i.min(self.toks.len() - 1)].t
    }
    fn peek_at(&self, k: usize) -> &T {
        &self.toks[(self.i + k).min(self.toks.len() - 1)].t
    }
    fn line(&self) -> u32 {
        self.toks[self.i.min(self.toks.len() - 1)].line
    }
    fn at(&self, t: &T) -> bool {
        self.peek() == t
    }
    fn at_op(&self, o: &str) -> bool {
        matches!(self.peek(), T::Op(x) if *x == o)
    }
    fn at_word(&self, w: &str) -> bool {
        matches!(self.peek(), T::Ident(x) if x == w)
    }
    fn next(&mut self) -> T {
        let t = self.peek().clone();
        self.i += 1;
        t
    }
    fn id(&mut self) -> u32 {
        self.ids += 1;
        self.ids
    }
    fn err<X>(&self, msg: impl Into<String>) -> Result<X, Error> {
        Err(Error {
            line: self.line(),
            msg: msg.into(),
        })
    }
    fn expect_op(&mut self, o: &str) -> Result<(), Error> {
        if self.at_op(o) {
            self.i += 1;
            Ok(())
        } else {
            self.err(format!("« {o} » attendu, trouvé {}", show(self.peek())))
        }
    }
    fn ident(&mut self) -> Result<String, Error> {
        match self.next() {
            T::Ident(s) => Ok(s),
            t => {
                self.i -= 1;
                self.err(format!("nom attendu, trouvé {}", show(&t)))
            }
        }
    }
    fn skip_newlines(&mut self) {
        while self.at(&T::Newline) || self.at_op(";") {
            self.i += 1;
        }
    }

    /// One level down (a parenthesis, an argument, a block): refused past `MAX_NIVEAUX`. The caller comes back up
    /// with `self.prof -= 1` when what it read is good; an error ends the reading.
    fn descendre(&mut self) -> Result<(), Error> {
        self.prof += 1;
        #[cfg(test)]
        PIC.with(|p| p.set((p.get().0.max(self.prof), p.get().1)));
        if self.prof > MAX_NIVEAUX {
            return self.err(format!(
                "trop de niveaux l'un dans l'autre (parenthèses, appels, blocs ou ? :) : moins de {MAX_NIVEAUX} (un bloc dans un bloc compte pour deux)"
            ));
        }
        Ok(())
    }

    /// Before the parts of a node are read: the height reached so far is put aside, and what comes is measured
    /// from the ground.
    fn debut(&mut self) -> u32 {
        std::mem::take(&mut self.haut)
    }

    /// A node was built over what was read since `debut`, or over the last node of a chain: one level more, refused
    /// past `MAX_HAUTEUR`.
    fn monter(&mut self) -> Result<(), Error> {
        self.haut += 1;
        #[cfg(test)]
        PIC.with(|p| p.set((p.get().0, p.get().1.max(self.haut))));
        if self.haut > MAX_HAUTEUR {
            return self.err(format!(
                "expression trop longue ou trop imbriquée : moins de {MAX_HAUTEUR} termes à la suite, moins quand les premiers sont eux-mêmes des calculs, des appels ou des [n] (coupe-la en morceaux, une variable par morceau)"
            ));
        }
        Ok(())
    }

    /// What was built since `debut` joins what was put aside: the taller of the two stays.
    fn fin(&mut self, avant: u32) {
        self.haut = self.haut.max(avant);
    }

    /// Bloc indenté (après une ligne d'en-tête) ou, après `=>`, une expression sur la même ligne.
    /// One statement, or several on the same line separated by commas (`a = 1, b = 2`, Pine v6).
    fn stmts(&mut self, b: &mut Block) -> Result<(), Error> {
        b.push(self.stmt()?);
        while self.at(&T::Op(",")) {
            self.i += 1;
            b.push(self.stmt()?);
        }
        Ok(())
    }

    fn block(&mut self) -> Result<Block, Error> {
        self.descendre()?;
        let b = self.bloc_lu()?;
        self.prof -= 1;
        Ok(b)
    }

    fn bloc_lu(&mut self) -> Result<Block, Error> {
        if self.at(&T::Newline) {
            self.skip_newlines();
            if !self.at(&T::Indent) {
                return self.err("bloc indenté attendu (4 espaces)");
            }
            self.i += 1;
            let mut b = vec![];
            self.skip_newlines();
            while !self.at(&T::Dedent) && !self.at(&T::Eof) {
                self.stmts(&mut b)?;
                self.skip_newlines();
            }
            if self.at(&T::Dedent) {
                self.i += 1;
            }
            Ok(b)
        } else {
            Ok(vec![self.stmt()?])
        }
    }

    /// Saute les mots de type devant un nom (`series float`, `array<float>`, `Zone`) ; vrai s'il y en avait.
    fn skip_type(&mut self) -> Option<String> {
        let start = self.i;
        while matches!(self.peek(), T::Ident(w) if QUALIFIERS.contains(&w.as_str())) {
            self.i += 1;
        }
        let ty = match self.peek().clone() {
            T::Ident(w) => w,
            _ => {
                self.i = start;
                return None;
            }
        };
        let mut k = 1;
        // type pointé : chart.point
        if ty == "chart" && matches!(self.peek_at(1), T::Op(".")) && matches!(self.peek_at(2), T::Ident(w) if w == "point") {
            k = 3;
        }
        // type générique : array<float>, map<string, float>
        if matches!(self.peek_at(1), T::Op("<")) {
            k = 2;
            while !matches!(self.peek_at(k), T::Op(">") | T::Newline | T::Eof) {
                k += 1;
            }
            k += 1;
        }
        // tableau écrit à l'ancienne : float[], int[], Zone[] (= array<float>…)
        if matches!(self.peek_at(k), T::Op("[")) && matches!(self.peek_at(k + 1), T::Op("]")) {
            k += 2;
        }
        // un type est suivi d'un nom ; un type maison en minuscules (`pivot p = …`, LuxAlgo) aussi, quand le nom est
        // suivi de ce qui suit une déclaration ou un paramètre
        let maison = !is_keyword(&ty)
            && matches!(self.peek_at(k), T::Ident(n) if !is_keyword(n))
            && matches!(self.peek_at(k + 1), T::Op("=" | "," | ")") | T::Newline | T::Dedent | T::Eof);
        if matches!(self.peek_at(k), T::Ident(_)) && (TYPES.contains(&ty.as_str()) || ty.chars().next().is_some_and(char::is_uppercase) || k > 1 || maison) {
            self.i += k;
            Some(if k == 3 && ty == "chart" { "chart.point".to_string() } else { ty })
        } else {
            self.i = start;
            None
        }
    }

    /// Le début d'une ligne est-il `nom(...) =>` (une fonction) ?
    fn is_fn_def(&self) -> bool {
        if !matches!(self.peek(), T::Ident(_)) || !matches!(self.peek_at(1), T::Op("(")) {
            return false;
        }
        let mut depth = 0;
        let mut k = 1;
        loop {
            match self.peek_at(k) {
                T::Op("(") => depth += 1,
                T::Op(")") => {
                    depth -= 1;
                    if depth == 0 {
                        return matches!(self.peek_at(k + 1), T::Op("=>"));
                    }
                }
                T::Newline | T::Eof => return false,
                _ => {}
            }
            k += 1;
        }
    }

    fn stmt(&mut self) -> Result<Stmt, Error> {
        let avant = self.debut();
        let st = self.instruction()?;
        self.monter()?;
        self.fin(avant);
        Ok(st)
    }

    fn instruction(&mut self) -> Result<Stmt, Error> {
        let line = self.line();
        if self.at_word("break") {
            self.i += 1;
            return Ok(Stmt::Break);
        }
        if self.at_word("continue") {
            self.i += 1;
            return Ok(Stmt::Continue);
        }
        if self.at_word("import") || self.at_word("export") {
            return self.err("import / export : pas encore dans Argos Script");
        }
        if self.at_word("type") && matches!(self.peek_at(1), T::Ident(_)) {
            self.i += 1;
            let name = self.ident()?;
            let mut fields = vec![];
            if self.at(&T::Newline) {
                self.skip_newlines();
                if self.at(&T::Indent) {
                    self.i += 1;
                    self.skip_newlines();
                    while !self.at(&T::Dedent) && !self.at(&T::Eof) {
                        self.skip_type();
                        let f = self.ident()?;
                        let def = if self.at_op("=") {
                            self.i += 1;
                            Some(self.expr()?)
                        } else {
                            None
                        };
                        fields.push((f, def));
                        self.skip_newlines();
                    }
                    if self.at(&T::Dedent) {
                        self.i += 1;
                    }
                }
            }
            return Ok(Stmt::Type { name, fields });
        }
        let method = self.at_word("method") && matches!(self.peek_at(1), T::Ident(_)) && matches!(self.peek_at(2), T::Op("("));
        if method {
            self.i += 1;
        }
        if method || self.is_fn_def() {
            let id = self.id();
            let name = self.ident()?;
            self.expect_op("(")?;
            let mut params = vec![];
            while !self.at_op(")") {
                self.skip_type();
                let p = self.ident()?;
                let def = if self.at_op("=") {
                    self.i += 1;
                    Some(self.expr()?)
                } else {
                    None
                };
                params.push((p, def));
                if !self.at_op(")") {
                    self.expect_op(",")?;
                }
            }
            self.expect_op(")")?;
            self.expect_op("=>")?;
            let body = self.block()?;
            return Ok(Stmt::Fn {
                id,
                name,
                params,
                body,
                method,
            });
        }
        // déclaration : [var|varip] [type] nom = … ; [a, b] = …
        let save = self.i;
        let keep = if self.at_word("var") {
            self.i += 1;
            Keep::Var
        } else if self.at_word("varip") {
            self.i += 1;
            Keep::Varip
        } else {
            Keep::Bar
        };
        let ty = self.skip_type();
        if self.at_op("[") {
            // tuple : [a, b] = f()
            let mut k = 1;
            let mut names = vec![];
            while let T::Ident(n) = self.peek_at(k) {
                names.push(n.clone());
                match self.peek_at(k + 1) {
                    T::Op(",") => k += 2,
                    T::Op("]") => {
                        if matches!(self.peek_at(k + 2), T::Op("=")) {
                            self.i += k + 3;
                            let value = self.expr()?;
                            let id = self.id();
                            return Ok(Stmt::Decl {
                                id,
                                keep,
                                ty,
                                names,
                                value,
                                line,
                            });
                        }
                        break;
                    }
                    _ => break,
                }
            }
        }
        if let (T::Ident(n), T::Op("=")) = (self.peek().clone(), self.peek_at(1).clone()) {
            if !is_keyword(&n) {
                self.i += 2;
                let value = self.expr()?;
                let id = self.id();
                return Ok(Stmt::Decl {
                    id,
                    keep,
                    ty,
                    names: vec![n],
                    value,
                    line,
                });
            }
        }
        if keep != Keep::Bar || ty.is_some() {
            return self.err("déclaration attendue : nom = valeur");
        }
        self.i = save;
        let e = self.expr()?;
        if let T::Op(op @ (":=" | "+=" | "-=" | "*=" | "/=" | "%=")) = self.peek().clone() {
            if !matches!(e.k, Ek::Ident(_) | Ek::Member(..) | Ek::Index { .. }) {
                return self.err("on ne peut pas assigner à cette expression");
            }
            self.i += 1;
            let value = self.expr()?;
            return Ok(Stmt::Assign { target: e, op, value, line });
        }
        Ok(Stmt::Expr(e))
    }

    pub fn expr(&mut self) -> Result<Expr, Error> {
        self.descendre()?;
        let e = if ["if", "switch", "for", "while"].iter().any(|w| self.at_word(w)) {
            let avant = self.debut();
            let e = self.structure()?;
            self.monter()?;
            self.fin(avant);
            e
        } else {
            self.ternary()?
        };
        self.prof -= 1;
        Ok(e)
    }

    /// `if`, `switch`, `for`, `while`: a node over its conditions and its blocks.
    fn structure(&mut self) -> Result<Expr, Error> {
        let line = self.line();
        if self.at_word("if") {
            return self.if_expr();
        }
        if self.at_word("switch") {
            self.i += 1;
            let subject = if self.at(&T::Newline) { None } else { Some(Box::new(self.expr()?)) };
            self.skip_newlines();
            if !self.at(&T::Indent) {
                return self.err("cas du switch attendus (indentés)");
            }
            self.i += 1;
            let (mut cases, mut default) = (vec![], None);
            self.skip_newlines();
            while !self.at(&T::Dedent) && !self.at(&T::Eof) {
                if self.at_op("=>") {
                    self.i += 1;
                    default = Some(self.block()?);
                } else {
                    let c = self.expr()?;
                    self.expect_op("=>")?;
                    cases.push((c, self.block()?));
                }
                self.skip_newlines();
            }
            if self.at(&T::Dedent) {
                self.i += 1;
            }
            return Ok(Expr {
                k: Ek::Switch(subject, cases, default),
                line,
            });
        }
        if self.at_word("for") {
            self.i += 1;
            if self.at_op("[") {
                self.i += 1;
                let mut vars = vec![self.ident()?];
                while self.at_op(",") {
                    self.i += 1;
                    vars.push(self.ident()?);
                }
                self.expect_op("]")?;
                if !self.at_word("in") {
                    return self.err("« in » attendu");
                }
                self.i += 1;
                let coll = Box::new(self.expr()?);
                let body = self.block()?;
                return Ok(Expr {
                    k: Ek::ForIn { vars, coll, body },
                    line,
                });
            }
            // la variable de boucle peut avoir son type : `for int i = 0 to 9`
            if matches!(self.peek(), T::Ident(w) if TYPES.contains(&w.as_str())) && matches!(self.peek_at(1), T::Ident(w) if w != "in") {
                self.i += 1;
            }
            let var = self.ident()?;
            if self.at_word("in") {
                self.i += 1;
                let coll = Box::new(self.expr()?);
                let body = self.block()?;
                return Ok(Expr {
                    k: Ek::ForIn { vars: vec![var], coll, body },
                    line,
                });
            }
            self.expect_op("=")?;
            let from = Box::new(self.expr()?);
            if !self.at_word("to") {
                return self.err("« to » attendu dans le for");
            }
            self.i += 1;
            let to = Box::new(self.expr()?);
            let by = if self.at_word("by") {
                self.i += 1;
                Some(Box::new(self.expr()?))
            } else {
                None
            };
            let body = self.block()?;
            return Ok(Expr {
                k: Ek::For { var, from, to, by, body },
                line,
            });
        }
        if self.at_word("while") {
            self.i += 1;
            let c = Box::new(self.expr()?);
            let body = self.block()?;
            return Ok(Expr { k: Ek::While(c, body), line });
        }
        self.ternary()
    }

    fn if_expr(&mut self) -> Result<Expr, Error> {
        let line = self.line();
        self.i += 1; // if
        let mut arms = vec![];
        let c = self.expr()?;
        arms.push((c, self.block()?));
        let mut otherwise = None;
        loop {
            // le else est au début d'une ligne suivante
            let save = self.i;
            self.skip_newlines();
            if self.at_word("else") {
                self.i += 1;
                if self.at_word("if") {
                    self.i += 1;
                    let c = self.expr()?;
                    arms.push((c, self.block()?));
                    continue;
                }
                otherwise = Some(self.block()?);
            } else {
                self.i = save;
            }
            break;
        }
        Ok(Expr {
            k: Ek::If(arms, otherwise),
            line,
        })
    }

    fn ternary(&mut self) -> Result<Expr, Error> {
        let avant = self.debut();
        let mut e = self.binary(0)?;
        if self.at_op("?") {
            let line = self.line();
            self.i += 1;
            let a = self.expr()?;
            self.expect_op(":")?;
            let b = self.expr()?;
            self.monter()?;
            e = Expr {
                k: Ek::Ternary(Box::new(e), Box::new(a), Box::new(b)),
                line,
            };
        }
        self.fin(avant);
        Ok(e)
    }

    fn binary(&mut self, lvl: usize) -> Result<Expr, Error> {
        const LEVELS: [&[&str]; 6] = [&["or"], &["and"], &["==", "!="], &["<", "<=", ">", ">="], &["+", "-"], &["*", "/", "%"]];
        if lvl == LEVELS.len() {
            return self.unary();
        }
        let avant = self.debut();
        let mut a = self.binary(lvl + 1)?;
        loop {
            let op = match self.peek() {
                T::Op(o) if LEVELS[lvl].contains(o) => *o,
                T::Ident(w) if LEVELS[lvl].contains(&w.as_str()) => {
                    if w == "or" {
                        "or"
                    } else {
                        "and"
                    }
                }
                _ => break,
            };
            let line = self.line();
            self.i += 1;
            let b = self.binary(lvl + 1)?;
            self.monter()?;
            a = Expr {
                k: Ek::Binary(op, Box::new(a), Box::new(b)),
                line,
            };
        }
        self.fin(avant);
        Ok(a)
    }

    /// `not x`, `-x`, `+x`, and several in a row (`not not x`, `- -x`): read in a loop, the last one applies first.
    fn unary(&mut self) -> Result<Expr, Error> {
        let mut ops = vec![];
        loop {
            let op = if self.at_word("not") {
                "not"
            } else if self.at_op("-") {
                "-"
            } else if self.at_op("+") {
                "+"
            } else {
                break;
            };
            ops.push((op, self.line()));
            self.i += 1;
        }
        let avant = self.debut();
        let mut e = self.postfix()?;
        while let Some((op, line)) = ops.pop() {
            self.monter()?;
            e = Expr {
                k: Ek::Unary(op, Box::new(e)),
                line,
            };
        }
        self.fin(avant);
        Ok(e)
    }

    /// Après un membre `a.b` : `<type, …>(` est un appel générique, pas une comparaison.
    fn generic(&mut self) -> Option<String> {
        if !self.at_op("<") {
            return None;
        }
        let mut k = 1;
        let mut parts = vec![];
        loop {
            match self.peek_at(k) {
                T::Ident(w) => parts.push(w.clone()),
                _ => return None,
            }
            // a dotted type: array.new<chart.point>()
            while matches!(self.peek_at(k + 1), T::Op(".")) && matches!(self.peek_at(k + 2), T::Ident(_)) {
                if let (Some(last), T::Ident(w)) = (parts.last_mut(), self.peek_at(k + 2)) {
                    *last = format!("{last}.{w}");
                }
                k += 2;
            }
            match self.peek_at(k + 1) {
                T::Op(",") => k += 2,
                T::Op(">") if matches!(self.peek_at(k + 2), T::Op("(")) => {
                    self.i += k + 2;
                    return Some(parts.join(","));
                }
                _ => return None,
            }
        }
    }

    fn postfix(&mut self) -> Result<Expr, Error> {
        let avant = self.debut();
        let mut e = self.primary()?;
        loop {
            let line = self.line();
            if self.at_op(".") {
                self.i += 1;
                let m = self.ident()?;
                self.monter()?;
                e = Expr {
                    k: Ek::Member(Box::new(e), m),
                    line,
                };
            } else if self.at_op("(") || matches!(self.peek(), T::Op("<")) && matches!(e.k, Ek::Member(..) | Ek::Ident(_)) && self.generic_ahead() {
                let gen = self.generic();
                self.expect_op("(")?;
                let mut args = vec![];
                while !self.at_op(")") {
                    let named = match (self.peek().clone(), self.peek_at(1).clone()) {
                        (T::Ident(n), T::Op("=")) => {
                            self.i += 2;
                            Some(n)
                        }
                        _ => None,
                    };
                    args.push((named, self.expr()?));
                    if !self.at_op(")") {
                        self.expect_op(",")?;
                    }
                }
                self.expect_op(")")?;
                let id = self.id();
                self.monter()?;
                e = Expr {
                    k: Ek::Call { id, f: Box::new(e), gen, args },
                    line,
                };
            } else if self.at_op("[") {
                self.i += 1;
                let n = self.expr()?;
                self.expect_op("]")?;
                let id = self.id();
                // (the same test as the interpreter's `history`: the past of a name is read from its variable)
                if let Ek::Ident(nom) = &e.k {
                    // (how far back: the number written there, as the interpreter reads it; a computed `n`: any bar)
                    let loin = match n.k {
                        Ek::Num(k) if k >= 0.0 => k as usize,
                        _ => usize::MAX,
                    };
                    let h = self.historiques.entry(nom.clone()).or_insert(0);
                    *h = (*h).max(loin);
                }
                self.monter()?;
                e = Expr {
                    k: Ek::Index {
                        id,
                        e: Box::new(e),
                        n: Box::new(n),
                    },
                    line,
                };
            } else {
                break;
            }
        }
        self.fin(avant);
        Ok(e)
    }

    fn generic_ahead(&self) -> bool {
        let mut k = 1;
        loop {
            if !matches!(self.peek_at(k), T::Ident(_)) {
                return false;
            }
            // a dotted type: array.new<chart.point>()
            while matches!(self.peek_at(k + 1), T::Op(".")) && matches!(self.peek_at(k + 2), T::Ident(_)) {
                k += 2;
            }
            match self.peek_at(k + 1) {
                T::Op(",") => k += 2,
                T::Op(">") => return matches!(self.peek_at(k + 2), T::Op("(")),
                _ => return false,
            }
        }
    }

    /// `(a, b) =>` : une lambda (extra Argos) ?
    fn lambda_ahead(&self) -> bool {
        let mut k = 1;
        if matches!(self.peek_at(1), T::Op(")")) {
            return matches!(self.peek_at(2), T::Op("=>"));
        }
        loop {
            if !matches!(self.peek_at(k), T::Ident(_)) {
                return false;
            }
            match self.peek_at(k + 1) {
                T::Op(",") => k += 2,
                T::Op(")") => return matches!(self.peek_at(k + 2), T::Op("=>")),
                _ => return false,
            }
        }
    }

    fn primary(&mut self) -> Result<Expr, Error> {
        let avant = self.debut();
        let e = self.valeur()?;
        self.fin(avant);
        Ok(e)
    }

    /// A value: a number, a text, a name, a list `[a, b]`, a lambda, or an expression in parentheses (which is no
    /// node of its own: its levels are counted going down, in `expr`).
    fn valeur(&mut self) -> Result<Expr, Error> {
        let line = self.line();
        let k = match self.next() {
            T::Num(v) => Ek::Num(v),
            T::Str(s) => Ek::Str(s),
            T::Color(c) => Ek::Color(c),
            T::Ident(w) => match w.as_str() {
                "true" => Ek::Bool(true),
                "false" => Ek::Bool(false),
                "na" if !matches!(self.peek(), T::Op("(")) => Ek::Na,
                "if" | "switch" | "for" | "while" => {
                    self.i -= 1;
                    return self.expr();
                }
                _ => Ek::Ident(w),
            },
            T::Op("(") => {
                self.i -= 1;
                if self.lambda_ahead() {
                    self.i += 1;
                    let mut ps = vec![];
                    while !self.at_op(")") {
                        ps.push(self.ident()?);
                        if self.at_op(",") {
                            self.i += 1;
                        }
                    }
                    self.i += 1;
                    self.expect_op("=>")?;
                    let body = self.expr()?;
                    self.monter()?;
                    return Ok(Expr {
                        k: Ek::Lambda(ps, Box::new(body)),
                        line,
                    });
                }
                self.i += 1;
                let e = self.expr()?;
                self.expect_op(")")?;
                return Ok(e);
            }
            T::Op("[") => {
                let mut items = vec![];
                while !self.at_op("]") {
                    items.push(self.expr()?);
                    if !self.at_op("]") {
                        self.expect_op(",")?;
                    }
                }
                self.i += 1;
                Ek::Tuple(items)
            }
            t => {
                self.i -= 1;
                return self.err(format!("valeur attendue, trouvé {}", show(&t)));
            }
        };
        self.monter()?;
        Ok(Expr { k, line })
    }
}

fn is_keyword(w: &str) -> bool {
    matches!(
        w,
        "if" | "else"
            | "for"
            | "to"
            | "by"
            | "in"
            | "while"
            | "switch"
            | "and"
            | "or"
            | "not"
            | "var"
            | "varip"
            | "true"
            | "false"
            | "break"
            | "continue"
            | "type"
            | "method"
    )
}

fn show(t: &T) -> String {
    match t {
        T::Num(v) => format!("« {v} »"),
        T::Str(s) => format!("« \"{s}\" »"),
        T::Color(_) => "une couleur".into(),
        T::Ident(w) => format!("« {w} »"),
        T::Op(o) => format!("« {o} »"),
        T::Newline => "la fin de la ligne".into(),
        T::Indent => "une indentation".into(),
        T::Dedent => "la fin du bloc".into(),
        T::Eof => "la fin du script".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lang::lexer::lex;

    fn p(src: &str) -> Script {
        parse(lex(src).unwrap()).unwrap_or_else(|e| panic!("{e}"))
    }

    #[test]
    fn syntaxe_pine_v6() {
        let s = p(r#"//@version=6
indicator("Test", overlay = true)
len = input.int(14, "Longueur", minval = 1)
var float acc = na
series float x = close
[m, sig, h] = ta.macd(close, 12, 26, 9)
f(a, b = 2) => a * b + 1
g(x) =>
    y = x * 2
    y + 1
acc := nz(acc) + close[1]
if close > open
    acc += 1
else if close < open
    acc -= 1
else
    acc := acc
z = switch
    close > open => 1
    => 0
for i = 0 to 10 by 2
    acc += i
arr = array.new<float>(3, 0.0)
for v in arr
    acc += v
type Zone
    float top = na
    int touches = 0
method height(Zone z) => z.top
c = close > open ? color.green : color.new(#FF0000, 50)
plot(x[1], "x", color = c)
dbl = arr.map((v) => v * 2)
"#);
        assert!(s.body.len() >= 18);
        assert!(matches!(&s.body[1], Stmt::Decl { keep: Keep::Bar, names, .. } if names == &["len"]));
        assert!(matches!(&s.body[2], Stmt::Decl { keep: Keep::Var, ty: Some(t), .. } if t == "float"));
        assert!(matches!(&s.body[4], Stmt::Decl { names, .. } if names.len() == 3));
        assert!(matches!(&s.body[5], Stmt::Fn { params, body, .. } if params.len() == 2 && body.len() == 1));
        assert!(matches!(&s.body[6], Stmt::Fn { body, .. } if body.len() == 2));
        assert!(matches!(&s.body[7], Stmt::Assign { op: ":=", .. }));
        assert!(matches!(&s.body[8], Stmt::Expr(Expr { k: Ek::If(arms, Some(_)), .. }) if arms.len() == 2));
        assert!(s.body.iter().any(|st| matches!(st, Stmt::Type { fields, .. } if fields.len() == 2)));
        assert!(s.body.iter().any(|st| matches!(st, Stmt::Fn { method: true, .. })));
    }

    #[test]
    fn erreurs_lisibles() {
        let e = parse(lex("x = (1 + 2").unwrap()).unwrap_err();
        assert_eq!(e.line, 1);
        let e = parse(lex("if close > open\nx = 1").unwrap()).unwrap_err();
        assert!(e.msg.contains("bloc indenté"), "{}", e.msg);
    }
}
