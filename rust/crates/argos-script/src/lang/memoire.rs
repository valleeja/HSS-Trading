//! 🧮 The memory a script keeps (B-062).
//!
//! One array holds 100 000 elements at most and one text 100 000 characters (B-049), but nothing limited how many of
//! them a script keeps at once: 1 000 arrays of 100 000 elements, kept in an array, took 2.3 GB in one bar. The app is
//! one program with no way to catch a failed allocation: past the memory of the browser, the whole app stops.
//!
//! So a run has a budget, shared with its sub-runs (`request.security`):
//! - every place that makes memory says how much (`creer`): an upper bound, cheap, never given back. The makers of
//!   values say it themselves (`Value::str`, `Value::arr`, `Value::text`…); a statement, a loop turn and a call of
//!   a built-in are counted a fixed size each, above what they add to a history, a site or an output;
//! - once enough has been made, the interpreter counts what is alive (`Interp::compter_memoire`), in three parts
//!   that never overlap, so nothing is counted twice and nothing has to be followed from holder to holder:
//!   1. the texts and the tuples, known to the byte at every moment (`VIVANT`): each one is added when it is made
//!      and taken off when its last holder lets go of it, whoever holds it meanwhile, a variable, a collection or
//!      the interpreter itself in the middle of a calculation (the values of a tuple being built, the arguments
//!      of a call waiting for the last one);
//!   2. the collections of the list below, each for its own places only, wherever they are held;
//!   3. the places of the interpreter: its histories, its sites, its outputs, its strategy;
//! - over the budget, the script stops with an error and its line. A count comes only when it may be needed: when
//!   the texts and tuples alive, plus what the last count found besides them, plus what was made since, reach the
//!   budget. A script far from its budget is never counted, whatever it makes and lets go of.
//!
//! The list of collections also ends two leaks: a collection that holds itself (`array.push(a, a)`, two objects
//! pointing at each other) is never freed by reference counting, so it is counted while it lives and emptied when
//! the run ends.
//!
//! The state is kept per thread: a run and its sub-runs are on one thread, and the makers of values
//! (`Value::arr`, `Value::str`) have no interpreter at hand.

use super::value::{Dico, Matrice, Matrix, Nuplet, Obj, Objet, Tableau, Value};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// Bytes of one value in an array, a matrix, a history.
pub(super) const V: usize = std::mem::size_of::<Value>();

/// Bytes counted for one block of memory above what it holds: what the allocator adds to it (its own header, the
/// size rounded up, the smallest block it gives).
const BLOC: usize = 32;

/// A shared block (a text, a tuple, a collection): its two counters, in a block of its own.
const PARTAGE: usize = 2 * std::mem::size_of::<usize>() + BLOC;

/// A collection without its places: its shared block, its borrow flag, its list (pointer, length, capacity) and its
/// line in the list of collections, which grows by doubling.
const COLLECTION: usize = PARTAGE + 4 * std::mem::size_of::<usize>() + 2 * std::mem::size_of::<Collection>();

/// The most a script keeps at once, sub-runs included, on a chart of a few bars: collections, texts, histories, the
/// memory of its `ta.*` calls and what it draws. The reference scripts of the tests keep 13 to 25 MB on 5 000 to
/// 50 000 bars.
const BUDGET: usize = 256 << 20;

/// What the budget grows by for each bar of the chart. A script keeps series as long as the chart with no way
/// around it: a plot is 16 bytes a bar, the memory of a `ta.*` call 8 to 16. With a flat budget an ordinary script
/// of 64 moving averages was refused on a chart of 200 000 bars, where it ran before; here that chart gives 451 MB.
const PAR_BOUGIE: usize = 1 << 10;

/// Once a count has found the script close to its budget, the next one comes after this much more: the most a
/// script goes over its budget before it is stopped (plus what one call makes: 24 MB for the largest array of the
/// longest chart).
const PAS_MIN: usize = 8 << 20;

/// A statement or a loop turn is counted this much: more than the places it adds to the tables of the interpreter
/// (a scope entry, a loop variable). Also what brings a count, sooner or later, to a script that only loops.
const OP: usize = 16;

/// A call of a built-in that computes is counted this much, above what it says itself: what it adds to the memory
/// of its site or to a collection is told to the byte (`pousser`, `grandi`).
const APPEL_CALCUL: usize = 32;

/// Any other call is counted this much: more than the largest entry a call adds to the outputs (a drawing, a
/// shape, an input, an order of a strategy), without their texts, which `Value::text` counts.
pub(super) const APPEL_SORTIE: usize = 512;

/// The list of collections is cleared of the dead ones when it has doubled, and at least this often.
const MENAGE_MIN: usize = 4096;

/// A collection made during the run, for as long as something holds it.
enum Collection {
    Arr(Weak<Tableau>),
    Map(Weak<Dico>),
    Mat(Weak<Matrice>),
    Obj(Weak<Objet>),
}

/// The same collection, held: it cannot be freed while the run is being closed.
enum Tenue {
    Arr(Rc<Tableau>),
    Map(Rc<Dico>),
    Mat(Rc<Matrice>),
    Obj(Rc<Objet>),
}

impl Collection {
    fn vivante(&self) -> bool {
        match self {
            Collection::Arr(w) => w.strong_count() > 0,
            Collection::Map(w) => w.strong_count() > 0,
            Collection::Mat(w) => w.strong_count() > 0,
            Collection::Obj(w) => w.strong_count() > 0,
        }
    }

    fn tenir(&self) -> Option<Tenue> {
        Some(match self {
            Collection::Arr(w) => Tenue::Arr(w.upgrade()?),
            Collection::Map(w) => Tenue::Map(w.upgrade()?),
            Collection::Mat(w) => Tenue::Mat(w.upgrade()?),
            Collection::Obj(w) => Tenue::Obj(w.upgrade()?),
        })
    }

    /// Adds the places of the collection, as it is now, to `compte`; false when nothing holds it any more. What
    /// its values hold is counted elsewhere: a text or a tuple in `VIVANT`, a collection at its own line of the
    /// list. A collection being changed right now (the call that changes it is still running) is counted without
    /// its places this time.
    fn peser(&self, compte: &mut Compte) -> bool {
        let Some(tenue) = self.tenir() else {
            return false;
        };
        compte.ajouter(COLLECTION);
        match &tenue {
            Tenue::Arr(a) => {
                if let Ok(a) = a.try_borrow() {
                    compte.places(a.capacity(), V);
                }
            }
            Tenue::Map(m) => {
                if let Ok(m) = m.try_borrow() {
                    compte.places(m.capacity(), 2 * V);
                }
            }
            Tenue::Mat(m) => {
                if let Ok(m) = m.try_borrow() {
                    compte.ajouter(octets_matrice(&m));
                }
            }
            Tenue::Obj(o) => {
                if let Ok(o) = o.try_borrow() {
                    compte.ajouter(octets_objet(&o));
                }
            }
        }
        true
    }
}

impl Tenue {
    /// Takes out what the collection holds and frees it. The collections it held are still held by the caller, so
    /// nothing is freed in cascade and a collection that holds itself lets go of itself.
    fn vider(&self) {
        match self {
            Tenue::Arr(a) => {
                let contenu = a.try_borrow_mut().map(|mut a| std::mem::take(&mut *a));
                drop(contenu);
            }
            Tenue::Map(m) => {
                let contenu = m.try_borrow_mut().map(|mut m| std::mem::take(&mut *m));
                drop(contenu);
            }
            Tenue::Mat(m) => {
                let contenu = m.try_borrow_mut().map(|mut m| std::mem::take(&mut m.v));
                drop(contenu);
            }
            Tenue::Obj(o) => {
                let contenu = o.try_borrow_mut().map(|mut o| std::mem::take(&mut o.fields));
                drop(contenu);
            }
        }
    }
}

/// Bytes of a list of `n` places of `octets` bytes: its block, when it has one.
fn octets_liste(n: usize, octets: usize) -> usize {
    if n == 0 {
        0
    } else {
        n.saturating_mul(octets).saturating_add(BLOC)
    }
}

/// Bytes of an object of a script type, without what its fields hold: the name of its type, its fields and their
/// names, each in a block of its own.
pub(super) fn octets_objet(o: &Obj) -> usize {
    let noms: usize = o.fields.iter().map(|(n, _)| octets_liste(n.capacity(), 1)).sum();
    octets_liste(o.ty.capacity(), 1)
        .saturating_add(octets_liste(o.fields.capacity(), std::mem::size_of::<(String, Value)>()))
        .saturating_add(noms)
}

/// Bytes of a matrix, without what its values hold: its sizes and its places.
pub(super) fn octets_matrice(m: &Matrix) -> usize {
    (2 * std::mem::size_of::<usize>()).saturating_add(octets_liste(m.v.capacity(), V))
}

/// Bytes of a tuple of `places` values, without what they hold: its shared block, its list and its places.
fn octets_nuplet(places: usize) -> usize {
    (PARTAGE + std::mem::size_of::<Nuplet>()).saturating_add(octets_liste(places, V))
}

struct Registre {
    /// Runs in progress on this thread: the main one and, inside it, its sub-runs.
    runs: u32,
    budget: usize,
    collections: Vec<Collection>,
    /// The length of `collections` at which the dead ones are dropped.
    menage: usize,
    /// What the runs above the one in progress keep outside the collections (a sub-run does not see its parent's
    /// histories).
    dessus: usize,
}

thread_local! {
    /// Bytes made since the last count, texts and tuples left out (an upper bound: nothing is ever taken off).
    static CREE: Cell<usize> = const { Cell::new(0) };
    /// Bytes of the texts and the tuples alive on this thread, and of the lists a built-in copied to work on while
    /// the script runs (`Pret`): exact, whoever holds them.
    static VIVANT: Cell<usize> = const { Cell::new(0) };
    /// A count is due when `CREE + VIVANT` reaches this (never, outside a run): the budget, less what the last
    /// count found besides the texts and the tuples. Until then the script is under its budget for sure.
    static LIMITE: Cell<usize> = const { Cell::new(usize::MAX) };
    /// Whether the text a value is written as may be kept by who asked for it (`appel`, `texte`).
    static GARDE: Cell<bool> = const { Cell::new(true) };
    /// The number of the count in progress: it changes each time `CREE` goes back to zero (`Naissance`).
    static EPOQUE: Cell<u32> = const { Cell::new(0) };
    /// Of `CREE`, the bytes told by lists that grew (`grandi`): a list let go of before the next count takes back
    /// what its own growth told, never more than this.
    static GRANDI: Cell<usize> = const { Cell::new(0) };
    static REGISTRE: RefCell<Registre> = const {
        RefCell::new(Registre { runs: 0, budget: BUDGET, collections: Vec::new(), menage: MENAGE_MIN, dessus: 0 })
    };
}

/// `octets` more were just made (or may have been: an upper bound is fine, it only brings the next count closer).
#[inline]
pub(super) fn creer(octets: usize) {
    CREE.with(|c| c.set(c.get().saturating_add(octets)));
}

/// `octets` more were made; true when the script may have reached its budget: the interpreter then counts what
/// it keeps.
#[inline]
fn fait(octets: usize) -> bool {
    let cree = CREE.with(|c| {
        let n = c.get().saturating_add(octets);
        c.set(n);
        n
    });
    cree.saturating_add(VIVANT.with(Cell::get)) >= LIMITE.with(Cell::get)
}

/// A statement or a loop turn was run; true when what the script keeps is to be counted now.
#[inline]
pub(super) fn op() -> bool {
    fait(OP)
}

/// A built-in is called: what a call may add to what the script keeps besides what it says itself, and whether the
/// texts it reads may stay (see `texte`). True when what the script keeps is to be counted.
#[inline]
pub(super) fn appel(nom: &str) -> bool {
    let (octets, garde) = match classe(nom) {
        Classe::Calcul => (APPEL_CALCUL, false),
        Classe::Reglage => (APPEL_CALCUL, true),
        Classe::Sortie => (APPEL_SORTIE, true),
    };
    GARDE.with(|g| g.set(garde));
    fait(octets)
}

/// What a built-in does with the memory, by its name.
#[derive(Debug, PartialEq)]
enum Classe {
    /// It computes: it adds no entry to the outputs, and the texts it reads are let go of at its end or come out as
    /// values of the script (counted alive, to the byte).
    Calcul,
    /// It adds no entry to the outputs but may keep a text it reads: it changes a drawing in place.
    Reglage,
    /// It may add an entry to the outputs and keep the texts it reads: every name not known here.
    Sortie,
}

fn classe(nom: &str) -> Classe {
    const CALCULS: [&str; 11] = [
        "ta.",
        "math.",
        "array.",
        "matrix.",
        "map.",
        "str.",
        "color.",
        "chart.point.",
        "label.get_",
        "line.get_",
        "box.get_",
    ];
    const REGLAGES: [&str; 4] = ["label.set_", "line.set_", "box.set_", "table.cell_set_"];
    if matches!(nom, "nz" | "na" | "fixnan" | "int" | "float" | "bool" | "string") || CALCULS.iter().any(|p| nom.starts_with(p)) {
        Classe::Calcul
    } else if REGLAGES.iter().any(|p| nom.starts_with(p)) {
        Classe::Reglage
    } else {
        Classe::Sortie
    }
}

/// A calculation between texts outside a built-in (`a + b`): the texts it reads are let go of at once.
#[inline]
pub(super) fn calcul() {
    GARDE.with(|g| g.set(false));
}

/// True when enough was made since the last count: asked by the places that make memory between two statements
/// (texts joined by « + », a tuple).
#[inline]
pub(super) fn a_compter() -> bool {
    fait(0)
}

/// A value was written as a text of `octets` bytes. When the built-in that asked may keep it (a label, a title, a
/// line of the log), it is told as made: the output that keeps it is weighed at each count. A calculation lets go
/// of it at once, or gives it back as a value of the script, counted alive: nothing to tell. 200 texts of 100 000
/// characters measured at each bar (`str.length`) were told as 20 MB made, and a script that held many small
/// objects was counted again every 5 bars: it ran 3 times slower.
#[inline]
pub(super) fn texte(octets: usize) {
    if GARDE.with(Cell::get) {
        creer(octets.saturating_add(BLOC));
    }
}

fn vivre(octets: usize) {
    let vivant = VIVANT.with(|v| {
        v.set(v.get().saturating_add(octets));
        v.get()
    });
    #[cfg(test)]
    PIC.with(|p| p.set(p.get().max(vivant)));
    #[cfg(not(test))]
    let _ = vivant;
}

fn mourir(octets: usize) {
    VIVANT.with(|v| v.set(v.get().saturating_sub(octets)));
}

/// The text of a value, of `octets` bytes, was made (`Value::str`).
#[inline]
pub(super) fn texte_vivant(octets: usize) {
    vivre(octets.saturating_add(PARTAGE));
}

/// The last holder of a text of `octets` bytes let go of it.
#[inline]
pub(super) fn texte_mort(octets: usize) {
    mourir(octets.saturating_add(PARTAGE));
}

/// A tuple of `places` values was made (`Value::tuple`).
#[inline]
pub(super) fn nuplet_vivant(places: usize) {
    vivre(octets_nuplet(places));
}

/// The last holder of a tuple of `places` values let go of it.
#[inline]
pub(super) fn nuplet_mort(places: usize) {
    mourir(octets_nuplet(places));
}

/// A text shared by several places of the strategy broker (the id of an exit, by the trades it trails): counted as
/// made, then weighed with the broker at each count.
pub(super) fn cle(s: &str) -> Rc<str> {
    creer(s.len().saturating_add(PARTAGE));
    Rc::from(s)
}

/// Bytes of the texts, tuples and lent lists alive now.
pub(super) fn vivant() -> usize {
    VIVANT.with(Cell::get)
}

/// A list a built-in copied for itself and holds while the script runs (the elements of an array while a lambda
/// is called on each, the pairs of a map while `for … in` goes through them): counted for as long as it is held,
/// since no variable of the script shows it.
pub(super) struct Pret(usize);

impl Pret {
    /// A list of `n` places of `octets` bytes each.
    pub fn de(n: usize, octets: usize) -> Pret {
        let octets = octets_liste(n, octets);
        vivre(octets);
        Pret(octets)
    }
}

impl Drop for Pret {
    fn drop(&mut self) {
        mourir(self.0);
    }
}

/// A list of `n` entries of `octets` bytes was made at once: an output with a place for every bar (a plot, a fill,
/// candles), the cells of a table, the points of a path.
#[inline]
pub(super) fn par_bougie(n: usize, octets: usize) {
    creer(octets_liste(n, octets));
}

/// `n` places for values were made: a list made whole (the results of a sub-run, a history that catches up).
#[inline]
pub(super) fn places(n: usize) {
    creer(octets_liste(n, V));
}

/// A list went from `avant` places to `apres`: what it took more is told to the byte. A full list takes as much
/// room again in one step, far more than the one place a push adds on average: 98 full arrays of 100 000 values,
/// one push on each, took 235 MB more for 12 KB told.
#[inline]
pub(super) fn grandi<T>(avant: usize, apres: usize) {
    let octets = croissance::<T>(avant, apres);
    if octets > 0 {
        creer(octets);
        GRANDI.with(|g| g.set(g.get().saturating_add(octets)));
    }
}

/// What a list of `T` takes more at `apres` places than at `avant`: the places, and its block when it had none.
fn croissance<T>(avant: usize, apres: usize) -> usize {
    if apres <= avant {
        return 0;
    }
    let bloc = if avant == 0 { BLOC } else { 0 };
    (apres - avant).saturating_mul(std::mem::size_of::<T>()).saturating_add(bloc)
}

/// Adds `x` at the end of `v` and tells what the list took more.
#[inline]
pub(super) fn pousser<T>(v: &mut Vec<T>, x: T) {
    let avant = v.capacity();
    v.push(x);
    grandi::<T>(avant, v.capacity());
}

/// What the making of a collection told the count, and when. A collection let go of before the next count takes it
/// back (`finir`): 20 arrays of 100 000 values made and let go of at each bar were 32 MB told as made, and a script
/// that also held 300 000 small objects was counted again every 5 bars.
pub(super) struct Naissance {
    epoque: u32,
    octets: usize,
    /// The places its list had when it was made (`naitre_liste`); `usize::MAX` for a collection without a list
    /// that grows.
    places: usize,
}

impl Naissance {
    /// The collection is let go of: made since the last count, what its making told is taken back. (Counted since,
    /// it is found gone at the next count.)
    pub(super) fn finir(&self) {
        self.rendre(0);
    }

    /// `finir` for a collection whose list of `T` has `places` places at its end: what its growth told since it was
    /// made is taken back with its making. (A buffer made full then pushed once told the doubling of its list and
    /// kept it told after its end: 400 buffers of 2 500 values at each bar brought two counts a bar to a script
    /// close to its budget, which ran 5 times slower.)
    pub(super) fn finir_liste<T>(&self, places: usize) {
        self.rendre(croissance::<T>(self.places, places));
    }

    fn rendre(&self, grandi: usize) {
        if self.epoque == EPOQUE.with(Cell::get) {
            // (never more than the lists told as they grew since the last count: a list that grew without telling
            // takes nothing back for it)
            let grandi = GRANDI.with(|g| {
                let pris = grandi.min(g.get());
                g.set(g.get() - pris);
                pris
            });
            CREE.with(|c| c.set(c.get().saturating_sub(self.octets.saturating_add(grandi))));
        }
    }
}

/// A collection of `octets` bytes is being made (every collection is made by a maker of `Value`).
pub(super) fn naitre(octets: usize) -> Naissance {
    let octets = octets.saturating_add(COLLECTION);
    creer(octets);
    Naissance {
        epoque: EPOQUE.with(Cell::get),
        octets,
        places: usize::MAX,
    }
}

/// `naitre` for a list of `n` places of `octets` bytes each.
pub(super) fn naitre_liste(n: usize, octets: usize) -> Naissance {
    Naissance {
        places: n,
        ..naitre(octets_liste(n, octets))
    }
}

/// The collection joins the list the counts go through.
fn inscrire(c: Collection) {
    REGISTRE.with(|r| {
        let mut r = r.borrow_mut();
        if r.runs == 0 {
            return;
        }
        r.collections.push(c);
        if r.collections.len() > r.menage {
            r.collections.retain(Collection::vivante);
            r.menage = (2 * r.collections.len()).max(MENAGE_MIN);
        }
    });
}

/// A new array (every array is made by `Value::arr`).
pub(super) fn tableau(a: &Rc<Tableau>) {
    inscrire(Collection::Arr(Rc::downgrade(a)));
}

/// A new map (`Value::map`).
pub(super) fn dictionnaire(m: &Rc<Dico>) {
    inscrire(Collection::Map(Rc::downgrade(m)));
}

/// A new matrix (`Value::mat`).
pub(super) fn matrice(m: &Rc<Matrice>) {
    inscrire(Collection::Mat(Rc::downgrade(m)));
}

/// A new object of a script type or a `chart.point` (`Value::obj`).
pub(super) fn objet(o: &Rc<Objet>) {
    inscrire(Collection::Obj(Rc::downgrade(o)));
}

/// What is alive, as it is being counted, in bytes.
#[derive(Default)]
pub(super) struct Compte {
    pub octets: usize,
}

impl Compte {
    pub fn ajouter(&mut self, octets: usize) {
        self.octets = self.octets.saturating_add(octets);
    }

    /// A list of `n` places of `octets` bytes each (a history, the memory of a site, the places of a collection).
    pub fn places(&mut self, n: usize, octets: usize) {
        self.ajouter(octets_liste(n, octets));
    }
}

/// Adds to `compte` the places of every collection made by the run, as they are now; the dead ones leave the list.
pub(super) fn collections(compte: &mut Compte) {
    REGISTRE.with(|r| {
        let mut r = r.borrow_mut();
        r.collections.retain(|c| c.peser(compte));
        r.menage = (2 * r.collections.len()).max(MENAGE_MIN);
    });
}

/// What the runs above the one in progress keep outside the collections.
pub(super) fn dessus() -> usize {
    REGISTRE.with(|r| r.borrow().dessus)
}

/// The budget of a script on a chart of `bougies` bars.
pub(super) fn budget(bougies: usize) -> usize {
    BUDGET.saturating_add(bougies.saturating_mul(PAR_BOUGIE))
}

/// The smallest step between two counts under `budget` (under a small budget, in a test: a sixteenth of it).
fn pas_min(budget: usize) -> usize {
    PAS_MIN.min(budget / 16 + 1)
}

/// The script keeps `total` bytes in all: `Err(budget)` when that is over its budget. Else nothing is counted again
/// until what is alive may have reached the budget: the texts and tuples are known at every moment, the rest is
/// what this count found, plus what is made from now on. Close to the budget, the next count comes one step later.
pub(super) fn suite(total: usize) -> Result<(), usize> {
    #[cfg(test)]
    COMPTES.with(|c| c.set(c.get() + 1));
    let budget = REGISTRE.with(|r| r.borrow().budget);
    if total > budget {
        return Err(budget);
    }
    let autres = total.saturating_sub(vivant());
    let seuil = budget.max(total.saturating_add(pas_min(budget)));
    nouvelle_epoque();
    LIMITE.with(|l| l.set(seuil.saturating_sub(autres)));
    Ok(())
}

/// A count starts from zero: nothing made before it is taken back from it.
fn nouvelle_epoque() {
    CREE.with(|c| c.set(0));
    GRANDI.with(|g| g.set(0));
    EPOQUE.with(|e| e.set(e.get().wrapping_add(1)));
}

/// A run in progress, from the making of its interpreter to its end. The first one on the thread is the main run:
/// it opens the budget, and at its end what the collections still hold is freed. The others are its sub-runs.
pub(super) struct Session {
    principale: bool,
}

impl Session {
    pub fn entrer(budget: usize) -> Session {
        REGISTRE.with(|r| {
            let mut r = r.borrow_mut();
            let principale = r.runs == 0;
            if principale {
                #[cfg(test)]
                let budget = ESSAI.with(Cell::take).unwrap_or(budget);
                r.budget = budget;
                r.dessus = 0;
                r.collections.clear();
                r.menage = MENAGE_MIN;
                nouvelle_epoque();
                LIMITE.with(|l| l.set(budget));
            }
            r.runs += 1;
            Session { principale }
        })
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let restes = REGISTRE.with(|r| {
            let mut r = r.borrow_mut();
            r.runs = r.runs.saturating_sub(1);
            if !self.principale {
                return Vec::new();
            }
            r.runs = 0;
            LIMITE.with(|l| l.set(usize::MAX));
            std::mem::take(&mut r.collections)
        });
        // (outside the borrow: what is freed here may itself free collections)
        let tenues: Vec<Tenue> = restes.iter().filter_map(Collection::tenir).collect();
        tenues.iter().for_each(Tenue::vider);
        drop(tenues);
        #[cfg(test)]
        RESTES.with(|r| *r.borrow_mut() = restes);
    }
}

/// While a sub-run works: what its parent keeps outside the collections is added to every count.
pub(super) struct Dessus(usize);

impl Dessus {
    pub fn ajouter(octets: usize) -> Dessus {
        REGISTRE.with(|r| {
            let mut r = r.borrow_mut();
            r.dessus = r.dessus.saturating_add(octets);
        });
        Dessus(octets)
    }
}

impl Drop for Dessus {
    fn drop(&mut self) {
        REGISTRE.with(|r| {
            let mut r = r.borrow_mut();
            r.dessus = r.dessus.saturating_sub(self.0);
        });
    }
}

#[cfg(test)]
thread_local! {
    /// 🧪 The budget of the next main run of this thread, when a test chose one.
    static ESSAI: Cell<Option<usize>> = const { Cell::new(None) };
    /// 🧪 The collections the last main run of this thread still held when it was closed.
    static RESTES: RefCell<Vec<Collection>> = const { RefCell::new(Vec::new()) };
    /// 🧪 The most `VIVANT` has been since a test last chose a budget.
    static PIC: Cell<usize> = const { Cell::new(0) };
    /// 🧪 How many times what a script keeps was counted since a test last chose a budget.
    static COMPTES: Cell<usize> = const { Cell::new(0) };
}

/// 🧪 For the tests: the budget of the next run of this thread.
#[cfg(test)]
pub(super) fn budget_essai(octets: usize) {
    ESSAI.with(|e| e.set(Some(octets)));
    PIC.with(|p| p.set(vivant()));
    COMPTES.with(|c| c.set(0));
}

/// 🧪 For the tests: how many times what a script keeps was counted since `budget_essai`.
#[cfg(test)]
pub(super) fn comptes() -> usize {
    COMPTES.with(Cell::get)
}

/// 🧪 For the tests: the bytes told as made since the last count (after a run: during its last stretch).
#[cfg(test)]
pub(super) fn cree() -> usize {
    CREE.with(Cell::get)
}

/// 🧪 For the tests: the most bytes of texts, tuples and lent lists alive at once since `budget_essai`.
#[cfg(test)]
pub(super) fn pic_vivant() -> usize {
    PIC.with(Cell::get)
}

/// 🧪 For the tests: the collections the last run of this thread held when it was closed (how many), and how many
/// of them something still holds now that its interpreter is gone.
#[cfg(test)]
pub(super) fn restes() -> (usize, usize) {
    RESTES.with(|r| (r.borrow().len(), r.borrow().iter().filter(|c| c.vivante()).count()))
}
