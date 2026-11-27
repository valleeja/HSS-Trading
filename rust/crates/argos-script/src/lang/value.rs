//! 💎 Les valeurs d'Argos Script, comme Pine : `na`, nombres (int et float sont des f64), booléens, textes, couleurs,
//! tableaux, maps, objets (`type`), tuples, et les lambdas d'Argos. Les tableaux / maps / objets sont partagés
//! (une copie de la variable pointe sur le même tableau, comme Pine).

use super::memoire;
use std::cell::RefCell;
use std::fmt;
use std::ops::Deref;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub enum Value {
    Na,
    Num(f64),
    Bool(bool),
    Str(Texte),
    /// 0xRRGGBB, transparence 0–100 dans les bits 24–30.
    Color(u32),
    Arr(Rc<Tableau>),
    Map(Rc<Dico>),
    /// 🔢 A `matrix.new` matrix (shared like arrays).
    Mat(Rc<Matrice>),
    Obj(Rc<Objet>),
    Tuple(Rc<Nuplet>),
    /// Lambda (extra Argos) : index dans la table des lambdas de l'interpréteur, and the number of the run that made
    /// it (B-076: the table is filled as each run meets its lambdas — a lambda given back by a sub-run of
    /// `request.security` named the chart's lambda of the same index).
    Lambda(u32, u32),
    /// Poignée d'un tracé (`plot` / `hline`) pour `fill`, ou d'un dessin (`label`, `line`, `box`).
    /// The third field is the number of the run that made it (`Interp::passage`): for any other run it names nothing,
    /// and every function and method takes it for `na` (B-076: a sub-run of `request.security` gives its values back
    /// to the chart's run, with the handles they hold at any depth).
    Handle(HandleKind, usize, u32),
}

// (B-076: the number of a handle's run fits in the room a value already had — 24 bytes on 64 bits, 16 in the web app;
// a bigger value would make every array, history and tuple of every script bigger)
const _: () = assert!(std::mem::size_of::<Value>() == if cfg!(target_pointer_width = "64") { 24 } else { 16 });

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandleKind {
    Plot,
    Hline,
    /// A label, a line or a box: its id, the rank among all the drawings the script ever made (B-064). `Rangs`
    /// tells where it is in `Output::drawings`, if it still is.
    Drawing,
    /// 📋 A `table.new` table (index in `Output::tables`).
    Table,
    /// A `linefill.new` fill between two lines: its id among the fills the script ever made (B-064).
    Linefill,
    /// A `polyline.new` path through `chart.point`s: its id among the paths the script ever made (B-064).
    Polyline,
}

/// 🔢 A Pine matrix: `rows` × `cols` values, row after row.
#[derive(Debug, Clone)]
pub struct Matrix {
    pub rows: usize,
    pub cols: usize,
    pub v: Vec<Value>,
}

impl Matrix {
    pub fn row(&self, r: usize) -> &[Value] {
        &self.v[r * self.cols..(r + 1) * self.cols]
    }
}

/// A matrix of a script, shared by every value that points at it. Made by `Value::mat` only.
pub struct Matrice {
    m: RefCell<Matrix>,
    ne: memoire::Naissance,
}

impl Deref for Matrice {
    type Target = RefCell<Matrix>;

    fn deref(&self) -> &Self::Target {
        &self.m
    }
}

impl Drop for Matrice {
    fn drop(&mut self) {
        self.ne.finir();
        liberer(std::mem::take(&mut self.m.get_mut().v));
    }
}

impl fmt::Debug for Matrice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.m.fmt(f)
    }
}

/// 🧮 The text of a script value: shared, never changed. Made by `Value::str` only, and counted in the memory of the
/// run from there until its last holder lets go of it, whoever holds it meanwhile: a variable, a collection, or
/// the interpreter itself in the middle of a calculation (B-062).
#[derive(Clone)]
pub struct Texte(Rc<str>);

impl Texte {
    fn nouveau(s: &str) -> Texte {
        memoire::texte_vivant(s.len());
        Texte(Rc::from(s))
    }
}

impl Deref for Texte {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0
    }
}

impl Drop for Texte {
    fn drop(&mut self) {
        // (the last holder: the text is freed with it)
        if Rc::strong_count(&self.0) == 1 {
            memoire::texte_mort(self.0.len());
        }
    }
}

impl PartialEq for Texte {
    fn eq(&self, autre: &Texte) -> bool {
        self.0 == autre.0
    }
}

impl fmt::Debug for Texte {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&*self.0, f)
    }
}

impl fmt::Display for Texte {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&*self.0, f)
    }
}

/// The elements of an array, shared by every value that points at it. Made by `Value::arr` only.
pub struct Tableau {
    v: RefCell<Vec<Value>>,
    ne: memoire::Naissance,
}

impl Deref for Tableau {
    type Target = RefCell<Vec<Value>>;

    fn deref(&self) -> &Self::Target {
        &self.v
    }
}

impl Drop for Tableau {
    fn drop(&mut self) {
        let v = self.v.get_mut();
        self.ne.finir_liste::<Value>(v.capacity());
        liberer(std::mem::take(v));
    }
}

impl fmt::Debug for Tableau {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.v.fmt(f)
    }
}

/// The pairs of a map, in the order they were put. Made by `Value::map` only.
pub struct Dico {
    v: RefCell<Vec<(Value, Value)>>,
    ne: memoire::Naissance,
}

impl Deref for Dico {
    type Target = RefCell<Vec<(Value, Value)>>;

    fn deref(&self) -> &Self::Target {
        &self.v
    }
}

impl Drop for Dico {
    fn drop(&mut self) {
        let paires = std::mem::take(self.v.get_mut());
        self.ne.finir_liste::<(Value, Value)>(paires.capacity());
        if paires.iter().any(|(cle, v)| cle.tient() || v.tient()) {
            liberer_un_a_un(paires.into_iter().flat_map(|(cle, v)| [cle, v]));
        }
    }
}

impl fmt::Debug for Dico {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.v.fmt(f)
    }
}

/// The values of a tuple, never changed once made. Made by `Value::tuple` only, and counted in the memory of the
/// run like a text: from there until its last holder lets go of it.
pub struct Nuplet(Vec<Value>);

impl Deref for Nuplet {
    type Target = Vec<Value>;

    fn deref(&self) -> &Vec<Value> {
        &self.0
    }
}

impl Drop for Nuplet {
    fn drop(&mut self) {
        memoire::nuplet_mort(self.0.capacity());
        liberer(std::mem::take(&mut self.0));
    }
}

impl fmt::Debug for Nuplet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

thread_local! {
    /// The collections, objects and tuples that wait to be freed, and whether a freeing is emptying that list now.
    static A_LIBERER: RefCell<(bool, Vec<Value>)> = const { RefCell::new((false, Vec::new())) };
}

/// Frees the values a collection, an object or a tuple held when its last holder let go of it (B-070).
///
/// Left to Rust, a value that holds a value that holds a value… is freed by a recursion as deep as the chain: a
/// linked list of 30 000 objects let go of at once (`head := na`), or `x := [x]` in a loop, overflowed the stack,
/// and the whole app stopped. Here a value that holds values itself is not freed inside the freeing of its holder:
/// it waits on a list, which the first freeing that started empties in a loop.
fn liberer(vals: Vec<Value>) {
    // (numbers and texts, the usual case: freed here, with the list that held them)
    if vals.iter().any(Value::tient) {
        liberer_un_a_un(vals.into_iter());
    }
}

fn liberer_un_a_un(vals: impl Iterator<Item = Value>) {
    // (when the thread itself is ending the list is gone: `vals` is then freed the plain way)
    let premier = A_LIBERER.try_with(|l| {
        let mut l = l.borrow_mut();
        // (a value that holds nothing is freed here; nothing in it comes back to this list)
        l.1.extend(vals.filter(Value::tient));
        !std::mem::replace(&mut l.0, true)
    });
    if premier != Ok(true) {
        return;
    }
    // (freeing one may add what it held to the list: it is taken at a later turn, never from inside this one)
    while let Some(v) = A_LIBERER.with(|l| l.borrow_mut().1.pop()) {
        drop(v);
    }
    A_LIBERER.with(|l| {
        let mut l = l.borrow_mut();
        l.0 = false;
        if l.1.capacity() > 4096 {
            l.1 = Vec::new();
        }
    });
}

/// The most characters a text of a script holds (B-049), far above the texts TradingView itself accepts.
pub(super) const MAX_TEXT: usize = 100_000;

/// How deep a collection inside a collection is written: deeper, its text is cut (an array that holds itself would
/// be written without an end).
const MAX_PROFONDEUR: u32 = 16;

/// Adds `s` to `out`; false when the text would pass 4 bytes for each of the `MAX_TEXT` characters a text holds, and
/// then only what fits of `s`, cut on a character (a piece left out whole cut the text of a collection holding
/// 100 000 characters of 4 bytes at its first character, « [… »).
fn pousser(out: &mut String, s: &str) -> bool {
    let reste = (MAX_TEXT * 4).saturating_sub(out.len());
    if s.len() <= reste {
        out.push_str(s);
        return true;
    }
    out.push_str(&s[..s.floor_char_boundary(reste)]);
    false
}

/// Adds the texts of `vals` to `out`, `sep` between them; false as soon as one does not fit (see `Value::ecrire`).
fn ecrire_liste<'v>(out: &mut String, vals: impl Iterator<Item = &'v Value>, sep: &str, niveau: u32) -> bool {
    let mut vals = vals.enumerate();
    vals.all(|(k, v)| (k == 0 || pousser(out, sep)) && v.ecrire(out, niveau))
}

/// Objet d'un type maison (`type Zone`).
#[derive(Debug, Clone)]
pub struct Obj {
    pub ty: String,
    pub fields: Vec<(String, Value)>,
}

/// An object of a script, shared by every value that points at it. Made by `Value::obj` only.
pub struct Objet {
    o: RefCell<Obj>,
    ne: memoire::Naissance,
}

impl Deref for Objet {
    type Target = RefCell<Obj>;

    fn deref(&self) -> &Self::Target {
        &self.o
    }
}

impl Drop for Objet {
    fn drop(&mut self) {
        self.ne.finir();
        let o = self.o.get_mut();
        if o.fields.iter().any(|(_, v)| v.tient()) {
            liberer_un_a_un(std::mem::take(&mut o.fields).into_iter().map(|(_, v)| v));
        }
    }
}

impl fmt::Debug for Objet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.o.fmt(f)
    }
}

impl Value {
    // 🧮 Every text, collection, object and tuple of a script is made by one of the makers below, and nowhere else
    // (a test reads the sources): each one tells the memory count of the run what it made (B-062).

    pub fn str(s: &str) -> Value {
        Value::Str(Texte::nouveau(s))
    }

    pub fn arr(v: Vec<Value>) -> Value {
        Value::Arr(Value::tableau(v))
    }

    /// A new array to fill before it becomes a value: what is put in it is seen by the memory count at once
    /// (the results of a lambda, one by one).
    pub(super) fn tableau(v: Vec<Value>) -> Rc<Tableau> {
        let ne = memoire::naitre_liste(v.capacity(), memoire::V);
        let a = Rc::new(Tableau { v: RefCell::new(v), ne });
        memoire::tableau(&a);
        a
    }

    pub fn map(v: Vec<(Value, Value)>) -> Value {
        let ne = memoire::naitre_liste(v.capacity(), 2 * memoire::V);
        let m = Rc::new(Dico { v: RefCell::new(v), ne });
        memoire::dictionnaire(&m);
        Value::Map(m)
    }

    pub fn mat(m: Matrix) -> Value {
        let ne = memoire::naitre(memoire::octets_matrice(&m));
        let m = Rc::new(Matrice { m: RefCell::new(m), ne });
        memoire::matrice(&m);
        Value::Mat(m)
    }

    pub fn obj(o: Obj) -> Value {
        let ne = memoire::naitre(memoire::octets_objet(&o));
        let o = Rc::new(Objet { o: RefCell::new(o), ne });
        memoire::objet(&o);
        Value::Obj(o)
    }

    pub fn tuple(v: Vec<Value>) -> Value {
        memoire::nuplet_vivant(v.capacity());
        Value::Tuple(Rc::new(Nuplet(v)))
    }

    /// A value that holds values itself: a collection, an object or a tuple.
    fn tient(&self) -> bool {
        matches!(self, Value::Arr(_) | Value::Map(_) | Value::Mat(_) | Value::Obj(_) | Value::Tuple(_))
    }

    /// Nombre (`na` → NaN, booléen → 1 / 0).
    pub fn num(&self) -> f64 {
        match self {
            Value::Num(v) => *v,
            Value::Bool(b) => *b as u8 as f64,
            Value::Color(c) => *c as f64,
            _ => f64::NAN,
        }
    }

    /// Vrai / faux comme Pine : `na` et 0 sont faux.
    pub fn truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Num(v) => *v != 0.0 && !v.is_nan(),
            Value::Na => false,
            _ => true,
        }
    }

    pub fn is_na(&self) -> bool {
        match self {
            Value::Na => true,
            Value::Num(v) => v.is_nan(),
            _ => false,
        }
    }

    /// A value that holds memory of its own: a collection, an object or a text. The history of a variable keeps
    /// it only when the script reads that name back with `[n]` (B-050).
    pub fn lourd(&self) -> bool {
        matches!(
            self,
            Value::Str(_) | Value::Arr(_) | Value::Map(_) | Value::Mat(_) | Value::Obj(_) | Value::Tuple(_)
        )
    }

    /// Nombre propre : NaN devient `na`.
    pub fn from_f64(v: f64) -> Value {
        if v.is_nan() {
            Value::Na
        } else {
            Value::Num(v)
        }
    }

    /// Texte (pour `str.tostring`, les étiquettes et les messages). The text of a collection stops at the limit of a
    /// text, « … » at its end; what a script keeps as a text goes through [`Value::texte_entier`], which refuses it.
    pub fn text(&self) -> String {
        let s = self.texte_fait();
        // (told as it is kept: its block, not its length)
        memoire::texte(s.capacity());
        s
    }

    fn texte_fait(&self) -> String {
        match self {
            Value::Na => "NaN".into(),
            Value::Num(v) => {
                if v.fract() == 0.0 && v.abs() < 1e15 {
                    format!("{}", *v as i64)
                } else {
                    let s = format!("{v:.8}");
                    s.trim_end_matches('0').trim_end_matches('.').to_string()
                }
            }
            Value::Bool(b) => b.to_string(),
            Value::Str(s) => s.to_string(),
            Value::Color(c) => format!("#{:06X}", c & 0xff_ffff),
            Value::Arr(_) | Value::Map(_) | Value::Mat(_) | Value::Tuple(_) => {
                let mut out = String::new();
                let entier = self.ecrire(&mut out, 0);
                // (`ecrire` stops at 4 bytes for each character a text holds: the text is cut here at the characters)
                match out.char_indices().nth(MAX_TEXT - 1) {
                    Some((k, c)) if !entier || k + c.len_utf8() < out.len() => {
                        out.truncate(k);
                        out.push('…');
                    }
                    None if !entier => out.push('…'),
                    _ => {}
                }
                // (B-062: the writing took room for up to 400 000 bytes, and a text cut at 100 000 characters kept
                // it all — 512 KB for 100 KB: a log of it at every bar kept 5 times what the budget was told, and
                // the app died of a refused allocation before the count came)
                out.shrink_to_fit();
                out
            }
            Value::Obj(o) => format!("{}(…)", o.borrow().ty),
            Value::Lambda(..) => "(lambda)".into(),
            Value::Handle(..) => "(id)".into(),
        }
    }

    /// The whole text of the value, or `None` when it would pass the limit of a text (B-049). The text is counted
    /// while it is written, piece by piece: a collection inside a collection is never built whole to be refused
    /// afterwards (600 elements nested 6 deep write 10^12 characters), and an array that holds itself has an end.
    pub fn texte_entier(&self) -> Option<String> {
        let mut out = String::new();
        let entier = self.ecrire(&mut out, 0);
        memoire::texte(out.capacity());
        entier.then_some(out)
    }

    /// The texts of `vals` with `sep` between them (`array.join`), or `None` like [`Value::texte_entier`].
    pub(super) fn joindre(vals: &[Value], sep: &str) -> Option<String> {
        let mut out = String::new();
        let entier = ecrire_liste(&mut out, vals.iter(), sep, 0);
        memoire::texte(out.capacity());
        entier.then_some(out)
    }

    /// Adds the text of the value to `out`. False as soon as the whole would pass the limit of a text, or when the
    /// collections go deeper than `MAX_PROFONDEUR`: `out` then holds the start of the text only.
    fn ecrire(&self, out: &mut String, niveau: u32) -> bool {
        match self {
            Value::Str(s) => pousser(out, s),
            Value::Arr(_) | Value::Map(_) | Value::Mat(_) | Value::Tuple(_) if niveau >= MAX_PROFONDEUR => false,
            Value::Arr(a) => pousser(out, "[") && ecrire_liste(out, a.borrow().iter(), ", ", niveau + 1) && pousser(out, "]"),
            Value::Tuple(t) => pousser(out, "[") && ecrire_liste(out, t.iter(), ", ", niveau + 1) && pousser(out, "]"),
            Value::Map(m) => {
                let paires = m.borrow();
                let mut paires = paires.iter().enumerate();
                pousser(out, "{")
                    && paires
                        .all(|(k, (cle, v))| (k == 0 || pousser(out, ", ")) && cle.ecrire(out, niveau + 1) && pousser(out, ": ") && v.ecrire(out, niveau + 1))
                    && pousser(out, "}")
            }
            Value::Mat(m) => {
                let m = m.borrow();
                pousser(out, "[")
                    && (0..m.rows).all(|r| {
                        (r == 0 || pousser(out, ", ")) && pousser(out, "[") && ecrire_liste(out, m.row(r).iter(), ", ", niveau + 1) && pousser(out, "]")
                    })
                    && pousser(out, "]")
            }
            v => pousser(out, &v.texte_fait()),
        }
    }

    /// Égalité de Pine (`na == x` est faux ; tableaux / objets : même référence).
    pub fn same(&self, o: &Value) -> bool {
        match (self, o) {
            (Value::Num(a), Value::Num(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Num(_), Value::Bool(_)) | (Value::Bool(_), Value::Num(_)) => self.num() == o.num(),
            (Value::Str(a), Value::Str(b)) => a == b,
            (Value::Color(a), Value::Color(b)) => a == b,
            (Value::Arr(a), Value::Arr(b)) => Rc::ptr_eq(a, b),
            (Value::Map(a), Value::Map(b)) => Rc::ptr_eq(a, b),
            (Value::Mat(a), Value::Mat(b)) => Rc::ptr_eq(a, b),
            (Value::Obj(a), Value::Obj(b)) => Rc::ptr_eq(a, b),
            (Value::Handle(k1, a, p1), Value::Handle(k2, b, p2)) => k1 == k2 && a == b && p1 == p2,
            _ => false,
        }
    }

    /// Nom du type (messages d'erreur).
    pub fn kind(&self) -> &'static str {
        match self {
            Value::Na => "na",
            Value::Num(_) => "nombre",
            Value::Bool(_) => "booléen",
            Value::Str(_) => "texte",
            Value::Color(_) => "couleur",
            Value::Arr(_) => "tableau",
            Value::Map(_) => "map",
            Value::Mat(_) => "matrice",
            Value::Obj(_) => "objet",
            Value::Tuple(_) => "tuple",
            Value::Lambda(..) => "lambda",
            Value::Handle(..) => "id",
        }
    }
}

/// Couleur → (r, g, b, transparence 0–100).
pub fn rgba(c: u32) -> (u8, u8, u8, u8) {
    ((c >> 16) as u8, (c >> 8) as u8, c as u8, ((c >> 24) & 0x7f).min(100) as u8)
}

/// (r, g, b, transparence) → couleur.
pub fn color(r: f64, g: f64, b: f64, t: f64) -> u32 {
    let c = |v: f64| v.round().clamp(0.0, 255.0) as u32;
    (c(r) << 16) | (c(g) << 8) | c(b) | ((t.round().clamp(0.0, 100.0) as u32) << 24)
}
