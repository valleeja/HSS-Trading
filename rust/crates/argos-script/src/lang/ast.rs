//! 🌳 L'arbre d'un script Argos Script (syntaxe Pine v6). Chaque appel, historique `x[n]` et déclaration a un `id`
//! unique : l'interpréteur y garde l'état propre à cet endroit du script (comme Pine : chaque `ta.sma` a sa mémoire).

/// Une expression et sa ligne.
#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub k: Ek,
    pub line: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Ek {
    Num(f64),
    Str(String),
    Bool(bool),
    Na,
    Color(u32),
    Ident(String),
    /// `a.b` : espace de noms (`ta.sma`), champ d'un objet ou d'une constante (`color.red`).
    Member(Box<Expr>, String),
    /// Appel : fonction, arguments (nommés ou non), type générique (`array.new<float>`).
    Call {
        id: u32,
        f: Box<Expr>,
        gen: Option<String>,
        args: Vec<(Option<String>, Expr)>,
    },
    /// Historique `x[n]` (et l'accès à un élément d'un tuple n'existe pas en Pine).
    Index {
        id: u32,
        e: Box<Expr>,
        n: Box<Expr>,
    },
    Unary(&'static str, Box<Expr>),
    Binary(&'static str, Box<Expr>, Box<Expr>),
    Ternary(Box<Expr>, Box<Expr>, Box<Expr>),
    /// `[a, b]` : tuple (retour de fonction) ou liste (`options = [...]`).
    Tuple(Vec<Expr>),
    /// `if` qui donne une valeur : (condition, bloc) puis le `else`.
    If(Vec<(Expr, Block)>, Option<Block>),
    /// `switch [sujet]` : cas (valeur ou condition, bloc) et le cas par défaut.
    Switch(Option<Box<Expr>>, Vec<(Expr, Block)>, Option<Block>),
    /// `for i = de to à [by pas]`.
    For {
        var: String,
        from: Box<Expr>,
        to: Box<Expr>,
        by: Option<Box<Expr>>,
        body: Block,
    },
    /// `for x in coll` ou `for [i, x] in coll`.
    ForIn {
        vars: Vec<String>,
        coll: Box<Expr>,
        body: Block,
    },
    While(Box<Expr>, Block),
    /// Extra Argos (pas dans Pine) : `(x, acc) => expr`, pour `array.map / filter / reduce`.
    Lambda(Vec<String>, Box<Expr>),
}

/// Un bloc : ses instructions ; sa valeur est celle de la dernière.
pub type Block = Vec<Stmt>;

/// Mot-clé de déclaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keep {
    /// Recalculée à chaque bougie.
    Bar,
    /// `var` : initialisée une seule fois, garde sa valeur d'une bougie à l'autre.
    Var,
    /// `varip` : comme `var` (pas de différence sans tick en direct).
    Varip,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    /// `[var] [type] nom = valeur` ou `[a, b] = valeur`.
    Decl {
        id: u32,
        keep: Keep,
        ty: Option<String>,
        names: Vec<String>,
        value: Expr,
        line: u32,
    },
    /// `cible := valeur` ou `cible += valeur` (op : ":=", "+=", …).
    Assign {
        target: Expr,
        op: &'static str,
        value: Expr,
        line: u32,
    },
    Expr(Expr),
    /// Fonction (ou méthode) : nom, paramètres (nom, valeur par défaut), corps.
    Fn {
        id: u32,
        name: String,
        params: Vec<(String, Option<Expr>)>,
        body: Block,
        method: bool,
    },
    /// `type Nom` et ses champs (nom, valeur par défaut).
    Type {
        name: String,
        fields: Vec<(String, Option<Expr>)>,
    },
    Break,
    Continue,
}

/// Un script lu : la déclaration `indicator(...)` / `strategy(...)` est une instruction comme les autres.
#[derive(Debug, Clone, PartialEq)]
pub struct Script {
    pub body: Block,
    /// Nombre d'`id` donnés (taille des tables de l'interpréteur).
    pub ids: u32,
    /// The names the script reads back with `name[n]`, anywhere (in a function too), and how far back: the largest
    /// `n` written as a number, or `usize::MAX` as soon as one `n` is computed. Only a variable with one of these
    /// names has a history, of that many bars (B-050, B-062).
    pub historiques: std::collections::HashMap<String, usize>,
}
