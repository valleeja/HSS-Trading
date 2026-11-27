//! ▶️ L'interpréteur : le script roule une fois par bougie, de la plus vieille à la plus récente, comme Pine.
//! - Chaque variable garde son historique (`x[1]` = sa valeur à la bougie d'avant) ; `var` garde sa valeur d'une
//!   bougie à l'autre, sinon la variable repart de `na` à chaque bougie.
//! - Chaque endroit du script (un `ta.sma`, un `x[n]` sur une expression) a sa mémoire, aussi par appel de fonction :
//!   une fonction appelée à deux endroits a deux mémoires (comme Pine).

use super::ast::*;
use super::lexer::Error;
use super::memoire::{self, Compte};
use super::rangs::Rangs;
use super::value::*;
use super::{InVal, Output};
use argos_core::Bar;
use std::collections::{HashMap, HashSet, VecDeque};

/// The `str.*` functions, called as methods on a text (`s.length()`): every other call on a text variable named like a
/// namespace goes to that namespace.
const STR_FNS: &[&str] = &[
    "tostring",
    "tonumber",
    "length",
    "upper",
    "lower",
    "contains",
    "startswith",
    "endswith",
    "pos",
    "replace_all",
    "replace",
    "substring",
    "split",
    "trim",
    "repeat",
    "format",
];

/// Une variable : valeur de la bougie en cours, valeurs des bougies passées.
#[derive(Debug, Clone)]
struct Var {
    cur: Value,
    /// The values of the bars before `bar`, the last `loin` of them at most; `hist[0]` is the value of bar `debut`.
    hist: VecDeque<Value>,
    debut: usize,
    /// Bougie de `cur`.
    bar: usize,
    keep: bool,
    init: bool,
    /// How many past bars this variable keeps: as far as the script reads its name back with `[n]`
    /// (`Script::historiques`: the largest `n` written, every bar when one `n` is computed), every bar for a series
    /// `run_vars` was asked for, none otherwise. Nothing else reads a history: on 200 000 bars, 200 variables that
    /// kept every bar took 932 MB (B-062).
    loin: usize,
    /// The script reads this name back with `[n]`: its history keeps its collections and its texts too. Otherwise
    /// nothing can read them (`run_vars` takes numbers), and an array made again at each bar would stay in memory
    /// for every bar of the chart (B-050).
    lourd: bool,
}

impl Var {
    /// The value of the next bar goes to the history; the oldest one leaves when `loin` bars are kept.
    fn pousser(&mut self, v: Value) {
        let avant = self.hist.capacity();
        self.hist.push_back(v);
        memoire::grandi::<Value>(avant, self.hist.capacity());
        if self.hist.len() > self.loin {
            self.hist.pop_front();
            self.debut += 1;
        }
    }

    /// The bar after the last one of the history.
    fn fin(&self) -> usize {
        self.debut + self.hist.len()
    }

    /// The value at bar `j`, when the history still holds it.
    fn passe(&self, j: usize) -> Option<&Value> {
        j.checked_sub(self.debut).and_then(|k| self.hist.get(k))
    }
}

/// Lissage exponentiel (`ta.ema`, `ta.rma`) démarré par la moyenne des `len` premières valeurs, comme `ta::ema`.
#[derive(Debug, Clone, Default)]
pub struct Smooth {
    pub len: usize,
    pub alpha: f64,
    seed: Vec<f64>,
    pub prev: f64,
    started: bool,
}

impl Smooth {
    pub fn new(len: usize, alpha: f64) -> Self {
        Smooth {
            len: len.max(1),
            alpha,
            seed: vec![],
            prev: f64::NAN,
            started: false,
        }
    }
    pub fn ema(len: usize) -> Self {
        Self::new(len, 2.0 / (len.max(1) as f64 + 1.0))
    }
    pub fn rma(len: usize) -> Self {
        Self::new(len, 1.0 / len.max(1) as f64)
    }
    pub fn next(&mut self, x: f64) -> f64 {
        if !self.started {
            if x.is_finite() {
                memoire::pousser(&mut self.seed, x);
            }
            if self.seed.len() == self.len {
                self.prev = self.seed.iter().sum::<f64>() / self.len as f64;
                self.started = true;
                return self.prev;
            }
            return f64::NAN;
        }
        if x.is_finite() {
            self.prev = self.alpha * x + (1.0 - self.alpha) * self.prev;
        }
        self.prev
    }
}

/// Mémoire d'un endroit du script (fonctions `ta`, historique d'une expression, `fixnan`…) : chaque fonction se
/// sert des champs dont elle a besoin.
#[derive(Debug, Clone, Default)]
pub struct Site {
    pub a: Vec<f64>,
    pub b: Vec<f64>,
    pub c: Vec<f64>,
    pub v: Vec<Value>,
    pub s: Vec<Smooth>,
    pub p: [f64; 8],
    /// Drapeau / compteur de l'endroit ; aussi l'état du `math.random` (64 bits : le même nombre sur desktop et en WASM).
    pub n: u64,
}

enum Flow {
    Val(Value),
    Break,
    Continue,
}

/// 🔬 `request.security_lower_tf`'s full path (the whole script on the intrabars) reads the newest 10 000 intrabars
/// at most: 30 000 one-minute bars took about 4 s with Argos Vague's TradingView edition (release build).
const MAX_INTRABARS_COMPLET: usize = 10_000;

/// Limite d'opérations par bougie (boucles infinies).
const MAX_OPS: u64 = 2_000_000;

/// Script functions one inside the other while a script runs: past it a function calls itself (Pine has no recursion).
const MAX_APPELS_SCRIPT: usize = 64;

/// The most levels a calculation goes down at once: an expression in an expression, a block in a block, and the
/// same again in each function a function calls. The interpreter calls itself once per level and the web app has
/// 1 MB of stack with no way to catch its end: 64 functions calling one another inside 15 nested `if` stopped the
/// whole app (B-074). Measured in a web build, a level takes 570 to 1 100 bytes of that stack (the most through the
/// lambda of `array.map`): 250 levels are 270 KB at most. The scripts of the repository go down 18 levels at most.
pub(super) const MAX_NIVEAUX: u32 = 250;

#[cfg(test)]
thread_local! {
    /// Test hook: the deepest level a calculation reached on this thread since `pic_niveau_zero`.
    static PIC_NIVEAU: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

/// Test hook: the deepest level reached since the last call, and back to zero.
#[cfg(test)]
pub(super) fn pic_niveau() -> u32 {
    PIC_NIVEAU.with(|p| p.replace(0))
}

/// Bytes counted for each bar of the chart when a `request.security*` site keeps its results: the bar of the other
/// timeframe (or its range of intrabars) and whether it closes it. Its values are counted as places.
const PAR_BOUGIE_D_UN_SOUS_RUN: usize = 24;

/// The key of no site: where `expression_sur` keeps its values (`capturer` never writes there).
const SANS_SITE: (u64, u64) = (u64::MAX, u64::MAX);

/// Bytes counted for one entry of a table of the interpreter (a variable, a site, an input seen), besides what the
/// entry holds: its key, its place and the room the table keeps free.
const PLACE_DE_TABLE: usize = 64;

thread_local! {
    /// The number of the last run made on this thread (B-076): each run numbers the handles it makes with its own.
    static PASSAGES: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

/// The number of a new run: one more than the last one on this thread (B-076). A sub-run of `request.security` gives
/// its values back to the chart's run with the handles they hold — alone, in a tuple, in an array, a map, a matrix or
/// an object, at any depth — and a handle names an output of the run that made it (a drawing, a table, a plot): with
/// its run's number, it names nothing in any other run, where every function and method takes it for `na`. (Given as
/// it was, it named whatever the chart's run had made at the same rank: a label of the chart was deleted, or got the
/// text meant for the sub-run's.) A lambda names an entry of its run's table of lambdas the same way: in another run,
/// the functions that call lambdas stop with an error and its line, instead of calling the chart's lambda of that
/// index. Nothing is copied or read through. The number wraps after 4 billion runs: a run and the sub-runs it makes,
/// one per `request.security` site, never come near that.
fn nouveau_passage() -> u32 {
    PASSAGES.with(|p| {
        let n = p.get().wrapping_add(1);
        p.set(n);
        n
    })
}

/// The line of a statement (0: `break`, `continue`, a function or a type, which have none).
fn ligne_de(st: &Stmt) -> u32 {
    match st {
        Stmt::Decl { line, .. } | Stmt::Assign { line, .. } => *line,
        Stmt::Expr(e) => e.line,
        _ => 0,
    }
}

pub struct Interp<'a> {
    /// 🧮 The run in progress, for the memory count (B-062). First field: it is closed first when the interpreter
    /// goes, and empties what the collections still hold one by one — then nothing is freed in cascade.
    _session: memoire::Session,
    pub bars: &'a [Bar],
    pub tf: i64,
    /// Bougie en cours.
    pub i: usize,
    script: &'a Script,
    fns: HashMap<&'a str, &'a Stmt>,
    methods: HashMap<&'a str, &'a Stmt>,
    types: HashMap<&'a str, &'a [(String, Option<Expr>)]>,
    vars: Vec<Var>,
    var_at: HashMap<(u64, u64, u8), usize>,
    scopes: Vec<HashMap<&'a str, usize>>,
    frame: usize,
    /// Chemin des appels de fonctions en cours (une mémoire par endroit d'appel).
    pub path: u64,
    depth: usize,
    /// How many levels down the calculation is (`MAX_NIVEAUX`); a sub-run goes on from the level of its caller.
    niveau: u32,
    pub sites: HashMap<(u64, u64), Site>,
    pub out: Output,
    pub inputs: &'a HashMap<String, InVal>,
    pub seen_inputs: HashSet<String>,
    /// The variables whose value at every bar `run_vars` was asked for, by name: they keep every bar.
    series: HashSet<String>,
    /// Inputs met on each line (`inline`) during this bar: (title of the first titled one, how many so far). Gives the
    /// stable key of an untitled input on a line (« MA #1 · 2 »), the same on every bar.
    pub lignes_inputs: HashMap<String, (String, usize)>,
    /// Inputs met during this bar: how many so far, and how many times each title. A title seen a second time gets
    /// the key « title@k » (k: its rank among the inputs), like `catalogue::par_nom`: two inputs with the same title
    /// (TradingView's SMA: « Length » and the smoothing's « Length ») keep their own value.
    pub inputs_vus: (usize, HashMap<String, usize>),
    /// Untitled inputs are named after their place among the inputs written in the script (« Input 3 »): the place
    /// of each input call, by its id, read from the parsed script the first time one is needed (`place_input`).
    pub places_inputs: Option<HashMap<u32, usize>>,
    pub plot_at: HashMap<(u64, u64), usize>,
    /// 🔔 The `alertcondition` of each call site (index in `out.alertconditions`).
    pub alertcond_at: HashMap<(u64, u64), usize>,
    /// 📋 The `table.new` of each call site (index in `out.tables`).
    pub table_at: HashMap<(u64, u64), usize>,
    lambdas: Vec<&'a Expr>,
    lambda_at: HashMap<usize, u32>,
    ops: u64,
    /// 1er argument de l'appel en cours quand c'est une série de la bougie (`high`, `close`…) : `ta.highest` lit alors
    /// l'historique des bougies, juste même s'il n'est appelé que dans un `if` (comme Pine).
    pub src: Option<&'a str>,
    /// 🔭 `request.security` (step 4): in a sub-run on higher-timeframe bars, the call site whose expression is
    /// captured, and its value at each of those bars.
    capture: Option<((u64, u64), Vec<Value>)>,
    /// In such a sub-run: the other `request.security` calls just evaluate their expression (no sub-run in a sub-run),
    /// and a strategy gives no order.
    pub(crate) sous: bool,
    /// 🔖 The number of this run among the runs of its thread (B-076). Its handles carry it; a handle that carries
    /// another one was made by a sub-run of `request.security` and given back inside what the call returns: here it
    /// names nothing, and every function and method takes it for `na`.
    pub(crate) passage: u32,
    /// 🧮 Pine v5 and before: `and` / `or` evaluate both sides, always (Pine v6 skips the right side when the left one
    /// decides — a function with memory there, `ta.crossover`, would miss bars).
    pub(crate) strict_et_ou: bool,
    /// ⚠️ Pine v6: warn (like TradingView) when a function with memory is called where it may be skipped — right of
    /// an `and` / `or`, in a branch of `?:`, `if` or `switch`.
    pub(crate) avertir_branches: bool,
    /// 🔁 Pine v6 loops: `for i = a to b` reads `b` again before each turn, and `for … in` an array reads its size and
    /// its element again at each turn (the body may remove or insert); v5 and older fix both before the loop starts.
    pub(crate) boucles_dynamiques: bool,
    /// How deep we are in such a place, the built-in being called (name, line) and the warnings already given.
    cond: u32,
    appel: (String, u32),
    avertis: HashSet<(u32, String)>,
    /// Main run: each `request.security` site's higher-timeframe data, computed once.
    secs: HashMap<(u64, u64), Securite>,
    /// 🧪 The broker of a `strategy(...)` script (made by the declaration, on the first bar).
    pub(crate) strat: Option<super::strategie::Courtier>,
    /// 🧮 `argos.real_delta` of each bar (NaN or missing: `na`), from `RunOptions`.
    pub real_delta: Vec<f64>,
    /// The last bar is still forming: `barstate.isconfirmed` is false on it.
    pub last_open: bool,
    /// 🔬 Lower-timeframe bars of the same symbol (`RunOptions::intrabars`), shared with the app.
    pub intrabars: std::sync::Arc<super::Intrabars>,
    /// Main run: each `request.security_lower_tf` site's intrabar values, computed once.
    ltfs: HashMap<(u64, u64), Intrabar>,
    /// The chart's symbol and exchange (`syminfo.ticker`, `syminfo.tickerid`); empty: « ARGOS ».
    pub ticker: String,
    pub exchange: String,
    /// Most drawings kept per kind (label, line, box): `max_labels_count`, `max_lines_count`, `max_boxes_count` of the
    /// declaration (1..=500), 500 when not given.
    /// Drawings kept by kind: labels, lines, boxes, polylines (`max_*_count` of `indicator()`).
    pub max_dessins: [usize; 4],
    /// The ids of the drawings (B-064): labels, lines and boxes (three sorts of one list), linefills, polylines.
    pub(super) ids_dessins: Rangs,
    pub(super) ids_linefills: Rangs,
    pub(super) ids_polylines: Rangs,
    /// The most elements an array or a matrix holds (B-049): 100 000 as on TradingView, or 5 a bar on a chart of more
    /// than 20 000 bars — TradingView's own ratio (100 000 elements, 20 000 bars), so that an array fed at each bar
    /// runs on the 200 000 bars a chart holds here. The chart's limit in a sub-run too, whatever its own bars.
    pub max_elements: usize,
}

/// One `request.security_lower_tf` site: for each chart bar, the range of its intrabars in `valeurs` (empty when the
/// intrabars do not cover it from its first second), the expression's value at each intrabar, and how many arrays
/// it gives (`Some(n)`: a tuple of n arrays). `na_partout`: an ignored invalid timeframe (`na` on every bar).
struct Intrabar {
    plages: Vec<(usize, usize)>,
    valeurs: Vec<Value>,
    tuple: Option<usize>,
    na_partout: bool,
}

/// One `request.security` site: for each chart bar, its higher-timeframe bar and whether it closes it; the
/// expression's value at each higher-timeframe bar.
struct Securite {
    haut: Vec<usize>,
    ferme: Vec<bool>,
    valeurs: Vec<Value>,
}

/// The chart bars grouped into bars of `tf` seconds (same symbol): the grouped bars and, for each chart bar, its
/// group. Days and below: by time; weeks: from Monday; months: by calendar month (UTC).
/// The group (bar of `tf` seconds) a time falls in: minutes / hours / days by time, weeks from Monday, months by
/// calendar month (UTC). Shared by `request.security` and `timeframe.change`.
pub(crate) fn groupe_ut(t: i64, tf: i64) -> i64 {
    if tf % 2_592_000 == 0 {
        use chrono::Datelike;
        let d = chrono::DateTime::from_timestamp(t, 0).unwrap_or_default();
        (d.year() as i64 * 12 + d.month0() as i64) / (tf / 2_592_000)
    } else if tf % 604_800 == 0 {
        (t + 3 * 86_400).div_euclid(tf)
    } else {
        t.div_euclid(tf)
    }
}

/// ⏱ The opening time (seconds) of the `tf` period numbered `g` by [`groupe_ut`].
pub(crate) fn debut_groupe(g: i64, tf: i64) -> i64 {
    if tf % 2_592_000 == 0 {
        let m = g * (tf / 2_592_000);
        let d = chrono::NaiveDate::from_ymd_opt(m.div_euclid(12) as i32, m.rem_euclid(12) as u32 + 1, 1).unwrap_or_default();
        d.and_hms_opt(0, 0, 0).unwrap_or_default().and_utc().timestamp()
    } else if tf % 604_800 == 0 {
        g * tf - 3 * 86_400
    } else {
        g * tf
    }
}

fn regrouper(bars: &[Bar], tf: i64) -> (Vec<Bar>, Vec<usize>) {
    let cle = |t: i64| groupe_ut(t, tf);
    let (mut out, mut idx): (Vec<Bar>, Vec<usize>) = (vec![], vec![]);
    let mut derniere = None;
    for b in bars {
        let k = cle(b.time);
        if derniere == Some(k) {
            let g = out.last_mut().expect("a group");
            g.high = g.high.max(b.high);
            g.low = g.low.min(b.low);
            g.close = b.close;
            g.volume += b.volume;
        } else {
            out.push(*b);
            derniere = Some(k);
        }
        idx.push(out.len() - 1);
    }
    (out, idx)
}

/// For each chart bar, the range of `petites` (lower-timeframe bars, oldest first) in [its time, the next chart bar's
/// time) — the last one: everything from its time on. Empty when the intrabars do not cover the bar from its first
/// second (they start after it).
fn plages(bars: &[Bar], petites: &[Bar]) -> Vec<(usize, usize)> {
    let debut = petites.first().map_or(i64::MAX, |b| b.time);
    let pos = |t: i64| petites.partition_point(|b| b.time < t);
    (0..bars.len())
        .map(|i| {
            if bars[i].time < debut {
                return (0, 0);
            }
            let a = pos(bars[i].time);
            let b = bars.get(i + 1).map_or(petites.len(), |n| pos(n.time));
            (a, b.max(a))
        })
        .collect()
}

/// Fresh arrays from intrabar values: one array, or (a tuple expression) a tuple of `n` arrays, element k of each.
fn tableaux(vals: &[Value], tuple: Option<usize>) -> Value {
    match tuple {
        None => Value::arr(vals.to_vec()),
        Some(n) => Value::tuple(
            (0..n)
                .map(|k| {
                    Value::arr(
                        vals.iter()
                            .map(|v| match v {
                                Value::Tuple(t) => t.get(k).cloned().unwrap_or(Value::Na),
                                v => v.clone(),
                            })
                            .collect(),
                    )
                })
                .collect(),
        ),
    }
}

fn mix(a: u64, b: u64) -> u64 {
    (a ^ b.wrapping_mul(0x9E37_79B9_7F4A_7C15)).rotate_left(23).wrapping_mul(0xBF58_476D_1CE4_E5B9)
}

impl<'a> Interp<'a> {
    pub fn new(script: &'a Script, bars: &'a [Bar], tf: i64, inputs: &'a HashMap<String, InVal>) -> Self {
        let n = bars.len();
        let mut it = Interp {
            _session: memoire::Session::entrer(memoire::budget(bars.len())),
            bars,
            tf,
            i: 0,
            script,
            fns: HashMap::new(),
            methods: HashMap::new(),
            types: HashMap::new(),
            vars: vec![],
            var_at: HashMap::new(),
            scopes: vec![HashMap::new()],
            frame: 0,
            path: 0,
            depth: 0,
            niveau: 0,
            sites: HashMap::new(),
            out: Output {
                bgcolor: vec![None; n],
                barcolor: vec![None; n],
                ..Default::default()
            },
            inputs,
            seen_inputs: HashSet::new(),
            series: HashSet::new(),
            lignes_inputs: HashMap::new(),
            inputs_vus: (0, HashMap::new()),
            places_inputs: None,
            plot_at: HashMap::new(),
            alertcond_at: HashMap::new(),
            table_at: HashMap::new(),
            lambdas: vec![],
            lambda_at: HashMap::new(),
            ops: 0,
            src: None,
            capture: None,
            sous: false,
            passage: nouveau_passage(),
            strict_et_ou: false,
            avertir_branches: false,
            boucles_dynamiques: false,
            cond: 0,
            appel: (String::new(), 0),
            avertis: HashSet::new(),
            secs: HashMap::new(),
            strat: None,
            real_delta: vec![],
            last_open: false,
            intrabars: Default::default(),
            ltfs: HashMap::new(),
            ticker: String::new(),
            exchange: String::new(),
            max_dessins: [
                super::builtins::MAX_DRAWINGS,
                super::builtins::MAX_DRAWINGS,
                super::builtins::MAX_DRAWINGS,
                super::builtins::MAX_POLYLINES,
            ],
            ids_dessins: Rangs::new(3),
            ids_linefills: Rangs::new(1),
            ids_polylines: Rangs::new(1),
            max_elements: super::builtins::MAX_ELEMENTS.max(n.saturating_mul(5)),
        };
        for st in &script.body {
            match st {
                Stmt::Fn { name, method: true, .. } => _ = it.methods.insert(name.as_str(), st),
                Stmt::Fn { name, .. } => _ = it.fns.insert(name.as_str(), st),
                Stmt::Type { name, fields } => _ = it.types.insert(name.as_str(), fields.as_slice()),
                _ => {}
            }
        }
        it
    }

    pub fn run(self) -> Result<Output, Error> {
        self.run_vars(&[]).map(|(out, _)| out)
    }

    /// The place of the input called at `id` among the inputs written in the script (1 for the first). Read from the
    /// text of the script, not from the run (B-063, B-069): an untitled input has the same name at every bar, in the
    /// chart's run and in a run on another timeframe, in a strategy (whose properties come first in the settings) as
    /// in an indicator, and two inputs of one line never share a name.
    pub(super) fn place_input(&mut self, id: u64) -> usize {
        let script = self.script;
        let places = self.places_inputs.get_or_insert_with(|| super::sorties::places_des_inputs(script));
        places.get(&(id as u32)).copied().unwrap_or(0)
    }

    /// Bar `i` starts: the scopes, the operations and what the inputs counted during the bar before start again.
    /// The chart's run and a sub-run on another timeframe start a bar the same way (B-069: a sub-run kept the count
    /// of the inputs from one bar to the next — from its second bar a titled input took the key of a second input
    /// with the same title, « Longueur@1 », so the value chosen in the settings was lost and the default came back).
    fn debut_bougie(&mut self, i: usize) {
        self.i = i;
        self.scopes = vec![HashMap::new()];
        self.lignes_inputs.clear();
        self.inputs_vus = (0, HashMap::new());
        self.frame = 0;
        self.ops = 0;
    }

    /// The script on every bar of the chart, in order.
    pub(super) fn tourner(&mut self) -> Result<(), Error> {
        for i in 0..self.bars.len() {
            self.debut_bougie(i);
            // 🧪 a strategy's orders fill at this bar's open and along its path, before the script reads its state
            if let Some(c) = self.strat.as_mut() {
                c.avant(i, &self.bars[i]);
            }
            let script = self.script;
            self.block(&script.body, false)?;
            if let Some(c) = self.strat.as_mut() {
                c.apres(&self.bars[i]);
            }
        }
        Ok(())
    }

    /// 🧪 For the tests: the collections, objects and texts the histories of the variables hold.
    #[cfg(test)]
    pub(super) fn lourds_gardes(&self) -> usize {
        self.vars.iter().flat_map(|v| &v.hist).filter(|x| x.lourd()).count()
    }

    /// 🧪 For the tests: the values the histories of the variables hold, whatever they are.
    #[cfg(test)]
    pub(super) fn passe_garde(&self) -> usize {
        self.vars.iter().map(|v| v.hist.len()).sum()
    }

    /// Runs the script, then reads the series of the top-level variables `names` (their value at every bar, as the
    /// script left it; a name the script does not declare at its top level is absent from the table).
    pub fn run_vars(mut self, names: &[&str]) -> Result<(Output, HashMap<String, Vec<f64>>), Error> {
        // (a series asked for keeps every bar; the other variables only what the script reads back)
        self.series = names.iter().map(|n| n.to_string()).collect();
        self.tourner()?;
        if let Some(c) = self.strat.take() {
            self.out.strategie = Some(c.bilan());
            // ⚙ the strategy's properties after the script's own inputs
            let (proprietes, autres) = std::mem::take(&mut self.out.inputs)
                .into_iter()
                .partition::<Vec<_>, _>(|s| s.group == super::strategie::PROPRIETES);
            self.out.inputs = autres.into_iter().chain(proprietes).collect();
        }
        let mut series = HashMap::new();
        // (closing the last bar: its value goes to the history like the others)
        self.i = self.bars.len();
        for name in names {
            let Some(vid) = self.scopes.first().and_then(|s| s.get(name).copied()) else {
                continue;
            };
            self.sync(vid);
            let var = &self.vars[vid];
            let mut v = vec![f64::NAN; var.debut.min(self.bars.len())];
            v.extend(var.hist.iter().map(Value::num));
            v.resize(self.bars.len(), f64::NAN);
            series.insert((*name).to_string(), v);
        }
        self.ranger_linefills();
        Ok((self.out, series))
    }

    /// While the script runs, a fill between two lines names them by their ids; the output names them by their
    /// places in `drawings` (B-064). A fill whose line left the list is deleted, like its line.
    fn ranger_linefills(&mut self) {
        for lf in &mut self.out.linefills {
            match (self.ids_dessins.place(lf.line1), self.ids_dessins.place(lf.line2)) {
                (Some(a), Some(b)) => (lf.line1, lf.line2) = (a, b),
                _ => {
                    lf.deleted = true;
                    (lf.line1, lf.line2) = (usize::MAX, usize::MAX);
                }
            }
        }
    }

    pub fn err<X>(&self, line: u32, msg: impl Into<String>) -> Result<X, Error> {
        Err(Error { line, msg: msg.into() })
    }

    /// La mémoire d'un endroit du script (créée au premier passage).
    pub fn site(&mut self, id: u64) -> &mut Site {
        // ⚠️ a function with memory where it may be skipped (Pine v6): TradingView's warning, once per place (the editor's
        // terminal writes the ⚠️ before each note)
        let (nom, ligne) = (&self.appel.0, self.appel.1);
        if self.avertir_branches
            && self.cond > 0
            && (nom.starts_with("ta.") || nom == "fixnan" || nom == "math.sum")
            && self.avertis.insert((ligne, nom.clone()))
        {
            self.out.notes.push((
                ligne,
                format!(
                    "La fonction « {nom} » doit être appelée à chaque bougie pour rester juste. Conseil : sortez cet \
                         appel de cette condition (mettez-le sur sa propre ligne)."
                ),
            ));
        }
        self.sites.entry((id, self.path)).or_insert_with(|| {
            memoire::creer(std::mem::size_of::<Site>() + PLACE_DE_TABLE);
            Site::default()
        })
    }

    /// Evaluates `f` in a place that may be skipped on some bars (right of `and` / `or`, a branch).
    fn branche<T>(&mut self, f: impl FnOnce(&mut Self) -> Result<T, Error>) -> Result<T, Error> {
        self.cond += 1;
        let r = f(self);
        self.cond -= 1;
        r
    }

    /// Bougie `k` bougies en arrière (`None` avant la première).
    pub fn bar_back(&self, k: usize) -> Option<&Bar> {
        self.i.checked_sub(k).map(|j| &self.bars[j])
    }

    // ── variables ────────────────────────────────────────────────────────────────────────────────────────────────

    /// Ferme les bougies passées de la variable (sa valeur va dans l'historique).
    fn sync(&mut self, vid: usize) {
        let i = self.i;
        let v = &mut self.vars[vid];
        if v.bar < i {
            // (B-050: a collection, an object or a text goes to the history only when the script reads this name
            // back with `[n]`; nothing else reads it there, `run_vars` takes numbers)
            // (B-062: and only as many bars as the script reads back; none for a name it never reads with `[n]`)
            if v.loin > 0 {
                let garde = |v: &Var| if v.lourd || !v.cur.lourd() { v.cur.clone() } else { Value::Na };
                // (bars the history never saw: `na`; when more than it keeps, it starts again `loin` bars before)
                if v.bar.saturating_sub(v.fin()) > v.loin {
                    v.hist.clear();
                    v.debut = v.bar - v.loin;
                }
                while v.fin() < v.bar {
                    v.pousser(Value::Na);
                }
                if v.fin() == v.bar {
                    let x = garde(v);
                    v.pousser(x);
                }
                // (a `var` keeps its value on the bars where the script does not pass; the others are `na` there)
                if i.saturating_sub(v.fin()) > v.loin {
                    v.hist.clear();
                    v.debut = i - v.loin;
                }
                while v.fin() < i {
                    let x = if v.keep { garde(v) } else { Value::Na };
                    v.pousser(x);
                }
            }
            v.bar = i;
            if !v.keep {
                v.cur = Value::Na;
            }
        }
    }

    /// La variable de cet endroit (déclaration, paramètre, variable de boucle), créée au premier passage ; `nom` : le
    /// nom que le script lui donne (un seul par endroit).
    fn var_for(&mut self, key: (u64, u64, u8), keep: bool, nom: &str) -> usize {
        if let Some(&v) = self.var_at.get(&key) {
            self.sync(v);
            return v;
        }
        let lu = self.script.historiques.get(nom).copied().unwrap_or(0);
        memoire::creer(std::mem::size_of::<Var>() + PLACE_DE_TABLE);
        self.vars.push(Var {
            cur: Value::Na,
            hist: VecDeque::new(),
            debut: 0,
            bar: self.i,
            keep,
            init: false,
            loin: if self.series.contains(nom) { usize::MAX } else { lu },
            lourd: lu > 0,
        });
        let v = self.vars.len() - 1;
        self.var_at.insert(key, v);
        v
    }

    fn bind(&mut self, name: &'a str, vid: usize) {
        if let Some(s) = self.scopes.last_mut() {
            s.insert(name, vid);
        }
    }

    fn lookup(&self, name: &str) -> Option<usize> {
        for s in self.scopes[self.frame..].iter().rev() {
            if let Some(&v) = s.get(name) {
                return Some(v);
            }
        }
        if self.frame > 0 {
            return self.scopes[0].get(name).copied();
        }
        None
    }

    fn declare(&mut self, key: (u64, u64, u8), name: &'a str, val: Value) {
        let vid = self.var_for(key, false, name);
        self.vars[vid].cur = val;
        self.vars[vid].init = true;
        self.bind(name, vid);
    }

    // ── instructions ─────────────────────────────────────────────────────────────────────────────────────────────

    fn block(&mut self, b: &'a Block, scoped: bool) -> Result<Flow, Error> {
        if scoped {
            self.scopes.push(HashMap::new());
        }
        let mut last = Flow::Val(Value::Na);
        for st in b {
            last = self.stmt(st)?;
            if !matches!(last, Flow::Val(_)) {
                break;
            }
        }
        if scoped {
            self.scopes.pop();
        }
        Ok(last)
    }

    fn stmt(&mut self, st: &'a Stmt) -> Result<Flow, Error> {
        self.ops += 1;
        if self.ops > MAX_OPS {
            return self.err(ligne_de(st), "trop d'opérations sur une bougie (boucle infinie ?)");
        }
        if memoire::op() {
            self.compter_memoire(ligne_de(st))?;
        }
        match st {
            Stmt::Fn { .. } | Stmt::Type { .. } => Ok(Flow::Val(Value::Na)),
            Stmt::Break => Ok(Flow::Break),
            Stmt::Continue => Ok(Flow::Continue),
            Stmt::Expr(e) => self.eval_flow(e),
            Stmt::Decl {
                id, keep, names, value, line, ..
            } => {
                let keep_it = *keep != Keep::Bar;
                let keys: Vec<(u64, u64, u8)> = (0..names.len()).map(|k| (*id as u64, self.path, k as u8)).collect();
                let vids: Vec<usize> = keys.iter().zip(names).map(|(k, n)| self.var_for(*k, keep_it, n)).collect();
                let fresh = !keep_it || !self.vars[vids[0]].init;
                if fresh {
                    let v = self.eval(value)?;
                    if names.len() == 1 {
                        let v = match v {
                            Value::Tuple(_) => return self.err(*line, "cette fonction donne plusieurs valeurs : [a, b] = …"),
                            v => v,
                        };
                        self.vars[vids[0]].cur = v;
                    } else {
                        let Value::Tuple(t) = v else {
                            return self.err(*line, format!("{} noms à gauche, mais une seule valeur à droite", names.len()));
                        };
                        if t.len() != names.len() {
                            return self.err(*line, format!("{} noms à gauche, {} valeurs à droite", names.len(), t.len()));
                        }
                        for (k, vid) in vids.iter().enumerate() {
                            self.vars[*vid].cur = t[k].clone();
                        }
                    }
                    for vid in &vids {
                        self.vars[*vid].init = true;
                    }
                }
                for (k, n) in names.iter().enumerate() {
                    self.bind(n.as_str(), vids[k]);
                }
                Ok(Flow::Val(self.vars[vids[0]].cur.clone()))
            }
            Stmt::Assign { target, op, value, line } => {
                let v = self.eval(value)?;
                match &target.k {
                    Ek::Ident(n) => {
                        let Some(vid) = self.lookup(n) else {
                            return self.err(*line, format!("« {n} » n'existe pas (déclare-la d'abord avec « {n} = … »)"));
                        };
                        self.sync(vid);
                        let nv = self.compound(op, &self.vars[vid].cur.clone(), v, *line)?;
                        self.vars[vid].cur = nv.clone();
                        Ok(Flow::Val(nv))
                    }
                    Ek::Member(base, field) => {
                        let Value::Obj(o) = self.eval(base)? else {
                            return self.err(*line, format!("« .{field} » : ce n'est pas un objet"));
                        };
                        let old = o.borrow().fields.iter().find(|f| f.0 == *field).map(|f| f.1.clone());
                        let Some(old) = old else {
                            return self.err(*line, format!("le champ « {field} » n'existe pas"));
                        };
                        let nv = self.compound(op, &old, v, *line)?;
                        if let Some(f) = o.borrow_mut().fields.iter_mut().find(|f| f.0 == *field) {
                            f.1 = nv.clone();
                        }
                        Ok(Flow::Val(nv))
                    }
                    _ => self.err(*line, "assignation impossible ici (array.set pour un tableau)"),
                }
            }
        }
    }

    fn compound(&self, op: &str, old: &Value, v: Value, line: u32) -> Result<Value, Error> {
        Ok(match op {
            ":=" => v,
            "+=" => self.binop("+", old.clone(), v, line)?,
            "-=" => self.binop("-", old.clone(), v, line)?,
            "*=" => self.binop("*", old.clone(), v, line)?,
            "/=" => self.binop("/", old.clone(), v, line)?,
            _ => self.binop("%", old.clone(), v, line)?,
        })
    }

    // ── expressions ──────────────────────────────────────────────────────────────────────────────────────────────

    pub fn eval(&mut self, e: &'a Expr) -> Result<Value, Error> {
        match self.eval_flow(e)? {
            Flow::Val(v) => Ok(v),
            _ => Ok(Value::Na),
        }
    }

    fn eval_flow(&mut self, e: &'a Expr) -> Result<Flow, Error> {
        self.niveau += 1;
        #[cfg(test)]
        PIC_NIVEAU.with(|p| p.set(p.get().max(self.niveau)));
        let r = if self.niveau > MAX_NIVEAUX {
            self.err(
                e.line,
                format!("trop de niveaux l'un dans l'autre dans ce calcul (expressions, blocs, fonctions qui s'appellent) : {MAX_NIVEAUX} au plus"),
            )
        } else {
            self.calculer(e)
        };
        self.niveau -= 1;
        r
    }

    fn calculer(&mut self, e: &'a Expr) -> Result<Flow, Error> {
        let v = match &e.k {
            Ek::Num(v) => Value::Num(*v),
            Ek::Str(s) => Value::str(s),
            Ek::Bool(b) => Value::Bool(*b),
            Ek::Na => Value::Na,
            Ek::Color(c) => Value::Color(*c),
            Ek::Ident(n) => match self.lookup(n) {
                Some(vid) => {
                    self.sync(vid);
                    self.vars[vid].cur.clone()
                }
                None => match super::builtins::variable(self, n) {
                    Some(v) => v,
                    None => return self.err(e.line, format!("« {n} » n'existe pas")),
                },
            },
            Ek::Member(base, m) => {
                // espace de noms (`color.red`, `barstate.islast`, `ta.tr`) ou champ d'un objet
                if let Ek::Ident(ns) = &base.k {
                    if self.lookup(ns).is_none() {
                        if let Some(v) = super::builtins::member(self, ns, m, e)? {
                            return Ok(Flow::Val(v));
                        }
                    }
                }
                // (two levels: `strategy.commission.percent`, `strategy.oca.cancel`)
                if let Ek::Member(b0, m0) = &base.k {
                    if let Ek::Ident(ns) = &b0.k {
                        if self.lookup(ns).is_none() && super::builtins::is_namespace(ns) {
                            if let Some(v) = super::builtins::member(self, &format!("{ns}.{m0}"), m, e)? {
                                return Ok(Flow::Val(v));
                            }
                        }
                    }
                }
                match self.eval(base)? {
                    Value::Obj(o) => match o.borrow().fields.iter().find(|f| f.0 == *m) {
                        Some(f) => f.1.clone(),
                        None => return self.err(e.line, format!("le champ « {m} » n'existe pas")),
                    },
                    Value::Na => Value::Na,
                    v => return self.err(e.line, format!("« .{m} » sur un {}", v.kind())),
                }
            }
            Ek::Call { id, f, gen, args } => self.call(*id, f, gen.as_deref(), args, e.line)?,
            Ek::Index { id, e: inner, n } => {
                let k = self.eval(n)?.num();
                if !(k >= 0.0) {
                    return self.err(e.line, "l'historique [n] demande un nombre positif");
                }
                self.history(*id, inner, k as usize, matches!(n.k, Ek::Num(_)))?
            }
            Ek::Unary(op, a) => {
                let v = self.eval(a)?;
                match *op {
                    "not" => Value::Bool(!v.truthy()),
                    "-" => Value::from_f64(-v.num()),
                    _ => v,
                }
            }
            // (Pine v5 and before: both sides always; v6: the right one only when needed)
            Ek::Binary(op @ ("and" | "or"), a, b) => {
                let x = self.eval(a)?.truthy();
                if self.strict_et_ou {
                    let y = self.eval(b)?.truthy();
                    Value::Bool(if *op == "and" { x && y } else { x || y })
                } else if (*op == "and") == x {
                    Value::Bool(self.branche(|s| s.eval(b))?.truthy())
                } else {
                    Value::Bool(x)
                }
            }
            Ek::Binary(op, a, b) => {
                let (x, y) = (self.eval(a)?, self.eval(b)?);
                let v = self.binop(op, x, y, e.line)?;
                // (B-062: texts joined in one long expression pass no statement and no call; counted here)
                if matches!(v, Value::Str(_)) && memoire::a_compter() {
                    self.compter_memoire(e.line)?;
                }
                v
            }
            Ek::Ternary(c, a, b) => {
                if self.eval(c)?.truthy() {
                    self.branche(|s| s.eval(a))?
                } else {
                    self.branche(|s| s.eval(b))?
                }
            }
            Ek::Tuple(items) => {
                let mut v = Vec::with_capacity(items.len());
                for it in items {
                    v.push(self.eval(it)?);
                }
                Value::tuple(v)
            }
            Ek::If(arms, otherwise) => {
                for (c, b) in arms {
                    if self.eval(c)?.truthy() {
                        return self.branche(|s| s.block(b, true));
                    }
                }
                match otherwise {
                    Some(b) => return self.branche(|s| s.block(b, true)),
                    None => Value::Na,
                }
            }
            Ek::Switch(subject, cases, default) => {
                let s = match subject {
                    Some(s) => Some(self.eval(s)?),
                    None => None,
                };
                for (c, b) in cases {
                    let v = self.eval(c)?;
                    let hit = match &s {
                        Some(s) => s.same(&v),
                        None => v.truthy(),
                    };
                    if hit {
                        return self.branche(|s| s.block(b, true));
                    }
                }
                match default {
                    Some(b) => return self.branche(|s| s.block(b, true)),
                    None => Value::Na,
                }
            }
            Ek::For { var, from, to, by, body } => {
                let (a, z) = (self.eval(from)?.num(), self.eval(to)?.num());
                if a.is_nan() || z.is_nan() {
                    return Ok(Flow::Val(Value::Na));
                }
                let step = match by {
                    Some(b) => self.eval(b)?.num().abs(),
                    None => 1.0,
                };
                if !(step > 0.0) {
                    return self.err(e.line, "le pas du for doit être plus grand que 0");
                }
                let step = if z < a { -step } else { step };
                let key = (e as *const Expr as u64, self.path, 0);
                let mut x = a;
                let mut z = z;
                let mut last = Value::Na;
                let mut premier = true;
                loop {
                    // (v6: the bound again before each turn after the first; the direction stays the first one)
                    if self.boucles_dynamiques && !premier {
                        z = self.eval(to)?.num();
                        if z.is_nan() {
                            break;
                        }
                    }
                    premier = false;
                    if !((step > 0.0 && x <= z + 1e-9) || (step < 0.0 && x >= z - 1e-9)) {
                        break;
                    }
                    self.scopes.push(HashMap::new());
                    self.declare(key, var.as_str(), Value::Num(x));
                    let f = self.block(body, false)?;
                    self.scopes.pop();
                    self.tick(e.line)?;
                    match f {
                        Flow::Break => break,
                        Flow::Continue => {}
                        Flow::Val(v) => last = v,
                    }
                    x += step;
                }
                last
            }
            Ek::ForIn { vars, coll, body } => {
                let col = self.eval(coll)?;
                // (v6, an array: its size and element read again at each turn, as TradingView does when the body
                // removes or inserts — `for [i, ob] in blocks` … `blocks.remove(i)` goes on with the next one's new index)
                if let (true, Value::Arr(arr)) = (self.boucles_dynamiques, &col) {
                    let base = e as *const Expr as u64;
                    let mut last = Value::Na;
                    let mut k = 0usize;
                    loop {
                        let Some(v) = arr.borrow().get(k).cloned() else { break };
                        self.scopes.push(HashMap::new());
                        if vars.len() == 2 {
                            self.declare((base, self.path, 0), vars[0].as_str(), Value::Num(k as f64));
                            self.declare((base, self.path, 1), vars[1].as_str(), v);
                        } else {
                            self.declare((base, self.path, 0), vars[0].as_str(), v);
                        }
                        let f = self.block(body, false)?;
                        self.scopes.pop();
                        self.tick(e.line)?;
                        match f {
                            Flow::Break => break,
                            Flow::Continue => {}
                            Flow::Val(v) => last = v,
                        }
                        k += 1;
                    }
                    return Ok(Flow::Val(last));
                }
                let items: Vec<(Value, Value)> = match col {
                    Value::Arr(a) => a.borrow().iter().enumerate().map(|(k, v)| (Value::Num(k as f64), v.clone())).collect(),
                    Value::Map(m) => m.borrow().clone(),
                    Value::Na => vec![],
                    v => return self.err(e.line, format!("for … in : un tableau ou une map, pas un {}", v.kind())),
                };
                // (B-062: this copy is held while the body runs, and no variable of the script shows it)
                let _pret = memoire::Pret::de(items.capacity(), 2 * memoire::V);
                let base = e as *const Expr as u64;
                let mut last = Value::Na;
                for (k, v) in items {
                    self.scopes.push(HashMap::new());
                    if vars.len() == 2 {
                        self.declare((base, self.path, 0), vars[0].as_str(), k);
                        self.declare((base, self.path, 1), vars[1].as_str(), v);
                    } else {
                        self.declare((base, self.path, 0), vars[0].as_str(), v);
                    }
                    let f = self.block(body, false)?;
                    self.scopes.pop();
                    self.tick(e.line)?;
                    match f {
                        Flow::Break => break,
                        Flow::Continue => {}
                        Flow::Val(v) => last = v,
                    }
                }
                last
            }
            Ek::While(c, body) => {
                let mut last = Value::Na;
                while self.eval(c)?.truthy() {
                    let f = self.block(body, true)?;
                    self.tick(e.line)?;
                    match f {
                        Flow::Break => break,
                        Flow::Continue => {}
                        Flow::Val(v) => last = v,
                    }
                }
                last
            }
            Ek::Lambda(..) => {
                let key = e as *const Expr as usize;
                let idx = match self.lambda_at.get(&key) {
                    Some(&k) => k,
                    None => {
                        self.lambdas.push(e);
                        let k = self.lambdas.len() as u32 - 1;
                        self.lambda_at.insert(key, k);
                        k
                    }
                };
                Value::Lambda(idx, self.passage)
            }
        };
        Ok(Flow::Val(v))
    }

    fn tick(&mut self, line: u32) -> Result<(), Error> {
        self.ops += 1;
        if self.ops > MAX_OPS {
            return self.err(line, "trop de tours de boucle sur une bougie (boucle infinie ?)");
        }
        if memoire::op() {
            self.compter_memoire(line)?;
        }
        Ok(())
    }

    /// 🧮 B-062: counts what the script keeps now, against its budget: the texts and tuples alive (known at every
    /// moment, whoever holds them), the places of the collections made by the run and its sub-runs, the places of
    /// this interpreter, and those of the run above while this one is its sub-run. Over the budget: the error of
    /// the script, at `line`.
    pub(super) fn compter_memoire(&self, line: u32) -> Result<(), Error> {
        let mut compte = Compte::default();
        compte.ajouter(memoire::vivant());
        memoire::collections(&mut compte);
        self.peser(&mut compte);
        compte.ajouter(memoire::dessus());
        match memoire::suite(compte.octets) {
            Ok(()) => Ok(()),
            Err(budget) => self.err(
                line,
                format!(
                    "le script garde trop de mémoire : plus de {} Mo (tableaux, textes, objets, dessins ou historiques \
                     gardés d'une bougie à l'autre)",
                    budget >> 20
                ),
            ),
        }
    }

    /// The places this interpreter holds: its variables and their histories, the memory of its sites, its outputs,
    /// the results of its sub-runs, its strategy. What the values in those places hold is not followed: a text or
    /// a tuple is in the count of the live ones, a collection in the list of collections. Every field is named
    /// here: a new one has to be placed, kept or not.
    fn peser(&self, c: &mut Compte) {
        use std::mem::size_of;
        let Interp {
            _session: _,
            bars: _,
            tf: _,
            i: _,
            script: _,
            fns: _,
            methods: _,
            types: _,
            vars,
            var_at,
            scopes,
            frame: _,
            path: _,
            depth: _,
            niveau: _,
            sites,
            out,
            inputs: _,
            seen_inputs,
            series: _,
            lignes_inputs,
            inputs_vus,
            places_inputs,
            plot_at,
            alertcond_at,
            table_at,
            lambdas,
            lambda_at,
            ops: _,
            src: _,
            capture,
            sous: _,
            passage: _,
            strict_et_ou: _,
            avertir_branches: _,
            boucles_dynamiques: _,
            cond: _,
            appel: _,
            avertis,
            secs,
            strat,
            // (given by the app, not made by the script)
            real_delta: _,
            last_open: _,
            intrabars: _,
            ltfs,
            ticker: _,
            exchange: _,
            max_dessins: _,
            ids_dessins,
            ids_linefills,
            ids_polylines,
            max_elements: _,
        } = self;
        // (a table: its places, of `t` bytes and one more each)
        let table = |n: usize, t: usize| n.saturating_mul(t + 1);
        let texte = String::capacity;
        c.ajouter(vars.capacity().saturating_mul(size_of::<Var>()));
        for v in vars {
            c.places(v.hist.capacity(), memoire::V);
        }
        c.ajouter(table(var_at.capacity(), size_of::<((u64, u64, u8), usize)>()));
        c.ajouter(scopes.iter().map(|s| table(s.capacity(), size_of::<(&str, usize)>())).sum());
        c.ajouter(table(sites.capacity(), size_of::<((u64, u64), Site)>()));
        for s in sites.values() {
            for l in [&s.a, &s.b, &s.c] {
                c.places(l.capacity(), size_of::<f64>());
            }
            c.places(s.v.capacity(), memoire::V);
            c.places(s.s.capacity(), size_of::<Smooth>());
            for l in &s.s {
                c.places(l.seed.capacity(), size_of::<f64>());
            }
        }
        c.ajouter(out.poids());
        c.ajouter(ids_dessins.poids() + ids_linefills.poids() + ids_polylines.poids());
        c.ajouter(seen_inputs.iter().map(|t| texte(t) + PLACE_DE_TABLE).sum());
        c.ajouter(lignes_inputs.iter().map(|(k, (t, _))| texte(k) + texte(t) + PLACE_DE_TABLE).sum());
        c.ajouter(inputs_vus.1.keys().map(|k| texte(k) + PLACE_DE_TABLE).sum());
        c.ajouter(places_inputs.as_ref().map_or(0, |p| table(p.capacity(), size_of::<(u32, usize)>())));
        for t in [plot_at, alertcond_at, table_at] {
            c.ajouter(table(t.capacity(), size_of::<((u64, u64), usize)>()));
        }
        c.ajouter(lambdas.capacity().saturating_mul(size_of::<&Expr>()));
        c.ajouter(table(lambda_at.capacity(), size_of::<(usize, u32)>()));
        if let Some((_, vals)) = capture {
            c.places(vals.capacity(), memoire::V);
        }
        c.ajouter(avertis.iter().map(|(_, nom)| texte(nom) + PLACE_DE_TABLE).sum());
        for s in secs.values() {
            c.places(s.haut.capacity(), size_of::<usize>());
            c.places(s.ferme.capacity(), 1);
            c.places(s.valeurs.capacity(), memoire::V);
        }
        for s in ltfs.values() {
            c.places(s.plages.capacity(), size_of::<(usize, usize)>());
            c.places(s.valeurs.capacity(), memoire::V);
        }
        if let Some(courtier) = strat {
            c.ajouter(courtier.poids());
        }
    }

    /// Whether values made by another run reached this one: a `request.security` site keeps what a sub-run or the fast
    /// path gave back (`secs`, `ltfs`: the only ways in, and nothing leaves them). Only then can a handle of another run
    /// be here (B-076).
    #[inline]
    pub(crate) fn lit_ailleurs(&self) -> bool {
        !self.secs.is_empty() || !self.ltfs.is_empty()
    }

    /// What this interpreter holds besides the collections, in bytes (what its sub-run adds to each of its counts).
    fn poids_propre(&self) -> usize {
        let mut compte = Compte::default();
        self.peser(&mut compte);
        compte.octets
    }

    pub fn binop(&self, op: &str, x: Value, y: Value, line: u32) -> Result<Value, Error> {
        // (B-049: a text that doubles in a loop stops at its limit, with an error)
        if let (Value::Str(a), "+") = (&x, op) {
            memoire::calcul();
            let b = super::builtins::texte_de(self, line, "+", &y)?;
            return super::builtins::valeur_texte(self, line, "+", a.len() + b.len(), || format!("{a}{b}"));
        }
        if let (Value::Str(b), "+") = (&y, op) {
            memoire::calcul();
            let a = super::builtins::texte_de(self, line, "+", &x)?;
            return super::builtins::valeur_texte(self, line, "+", a.len() + b.len(), || format!("{a}{b}"));
        }
        match op {
            "==" | "!=" => {
                if x.is_na() || y.is_na() {
                    return Ok(Value::Bool(false));
                }
                let eq = x.same(&y);
                return Ok(Value::Bool(if op == "==" { eq } else { !eq }));
            }
            _ => {}
        }
        if matches!(x, Value::Str(_)) || matches!(y, Value::Str(_)) {
            return self.err(line, format!("« {op} » entre un {} et un {}", x.kind(), y.kind()));
        }
        let (a, b) = (x.num(), y.num());
        Ok(match op {
            "+" => Value::from_f64(a + b),
            "-" => Value::from_f64(a - b),
            "*" => Value::from_f64(a * b),
            "/" => Value::from_f64(if b == 0.0 { f64::NAN } else { a / b }),
            "%" => Value::from_f64(if b == 0.0 { f64::NAN } else { a % b }),
            "<" => Value::Bool(a < b),
            "<=" => Value::Bool(a <= b),
            ">" => Value::Bool(a > b),
            ">=" => Value::Bool(a >= b),
            _ => return self.err(line, format!("opérateur inconnu « {op} »")),
        })
    }

    /// `e[k]` : variable → son historique ; prix de la bougie → bougie d'avant ; autre expression → mémoire de cet
    /// endroit (l'expression est calculée à chaque bougie où on passe ici). `ecrit` : `k` is a number written in the
    /// script, the same at every bar: that place then lets go of the collections and texts older than `k` bars.
    fn history(&mut self, id: u32, inner: &'a Expr, k: usize, ecrit: bool) -> Result<Value, Error> {
        if let Ek::Ident(n) = &inner.k {
            if let Some(vid) = self.lookup(n) {
                self.sync(vid);
                let v = &self.vars[vid];
                return Ok(if k == 0 {
                    v.cur.clone()
                } else {
                    self.i.checked_sub(k).and_then(|j| v.passe(j)).cloned().unwrap_or(Value::Na)
                });
            }
            if let Some(v) = super::builtins::series_back(self, n, k) {
                return Ok(v);
            }
        }
        let v = self.eval(inner)?;
        let i = self.i;
        let s = self.site(id as u64 | 1 << 40);
        let avant = s.v.capacity();
        if !ecrit {
            // `k` is computed: any bar may be read back, the place keeps one value a bar
            while s.v.len() < i {
                s.v.push(Value::Na);
            }
            if s.v.len() == i {
                s.v.push(v);
            } else {
                s.v[i] = v;
            }
            memoire::grandi::<Value>(avant, s.v.capacity());
            return Ok(i.checked_sub(k).and_then(|j| s.v.get(j)).cloned().unwrap_or(Value::Na));
        }
        // (B-062: nothing here reads further back than `k`, so the place keeps the last `k + 1` bars and no more:
        // one value a bar for every `(a + b)[1]` of a script was 24 bytes a bar each, for nothing. `s.n` is the bar
        // of `s.v[0]`.)
        let mut debut = s.n as usize;
        if i < debut || debut + s.v.len() + k <= i {
            // not passed here for `k` bars or more: nothing kept can be read again
            s.v.clear();
            debut = i;
        }
        let deja = s.v.len().saturating_sub(k + 1);
        while debut + s.v.len() < i {
            s.v.push(Value::Na);
        }
        if debut + s.v.len() == i {
            s.v.push(v);
        } else {
            s.v[i - debut] = v;
        }
        memoire::grandi::<Value>(avant, s.v.capacity());
        // what went out of reach lets go of its collections and its texts at once, and leaves the list by blocks
        let trop = s.v.len().saturating_sub(k + 1);
        for x in &mut s.v[deja.min(trop)..trop] {
            if x.lourd() {
                *x = Value::Na;
            }
        }
        if trop > k + 16 {
            s.v.drain(..trop);
            debut += trop;
        }
        s.n = debut as u64;
        Ok(i.checked_sub(k)
            .and_then(|j| j.checked_sub(debut))
            .and_then(|j| s.v.get(j))
            .cloned()
            .unwrap_or(Value::Na))
    }

    // ── appels ───────────────────────────────────────────────────────────────────────────────────────────────────

    fn args(&mut self, args: &'a [(Option<String>, Expr)]) -> Result<Vec<(Option<&'a str>, Value)>, Error> {
        let mut out = Vec::with_capacity(args.len());
        for (n, e) in args {
            out.push((n.as_deref(), self.eval(e)?));
        }
        Ok(out)
    }

    fn call(&mut self, id: u32, f: &'a Expr, gen: Option<&str>, args: &'a [(Option<String>, Expr)], line: u32) -> Result<Value, Error> {
        match &f.k {
            Ek::Ident(name) => {
                if let Some(st) = self.fns.get(name.as_str()).copied() {
                    let a = self.args(args)?;
                    return self.call_user(st, id, a, line);
                }
                let a = self.args(args)?;
                self.appel = (name.clone(), line);
                super::builtins::call(self, name, a, id as u64, gen, line)
            }
            Ek::Member(base, m) => {
                if let Ek::Ident(ns) = &base.k {
                    if ns == "request" && m == "security" && self.lookup(ns).is_none() {
                        return self.securite(id as u64, args, line);
                    }
                    if ns == "request" && m == "security_lower_tf" && self.lookup(ns).is_none() {
                        return self.securite_ltf(id as u64, args, line);
                    }
                    if self.lookup(ns).is_none() && super::builtins::is_namespace(ns) {
                        let a = self.args(args)?;
                        self.src = args
                            .first()
                            .filter(|(n, _)| n.as_deref().is_none_or(|n| n == "source"))
                            .and_then(|(_, e)| match &e.k {
                                Ek::Ident(v) if self.lookup(v).is_none() => Some(v.as_str()),
                                _ => None,
                            });
                        self.appel = (format!("{ns}.{m}"), line);
                        return super::builtins::call(self, &format!("{ns}.{m}"), a, id as u64, gen, line);
                    }
                    if m == "new" && self.lookup(ns).is_none() {
                        if let Some(fields) = self.types.get(ns.as_str()).copied() {
                            let a = self.args(args)?;
                            return self.construct(ns, fields, a, line);
                        }
                    }
                }
                // chart.point.new / from_index / from_time / now / copy
                if let Ek::Member(b2, p) = &base.k {
                    if matches!(&b2.k, Ek::Ident(ns) if ns == "chart" && p == "point" && self.lookup(ns).is_none()) {
                        let a = self.args(args)?;
                        let nom = format!("chart.point.{m}");
                        self.appel = (nom.clone(), line);
                        return super::builtins::call(self, &nom, a, id as u64, gen, line);
                    }
                }
                // méthode : la valeur passe en premier argument
                let this = self.eval(base)?;
                // A text variable named like a built-in namespace (`timeframe`, `ticker`…) does not hide that
                // namespace's functions, as in Pine: LuxAlgo's `higherTimeframe(string timeframe) =>
                // timeframe.in_seconds() > timeframe.in_seconds(timeframe)` compares the chart's timeframe with the given one.
                if let (Ek::Ident(ns), Value::Str(_)) = (&base.k, &this) {
                    if ns != "str" && super::builtins::is_namespace(ns) && !STR_FNS.contains(&m.as_str()) {
                        let a = self.args(args)?;
                        self.appel = (format!("{ns}.{m}"), line);
                        return super::builtins::call(self, &format!("{ns}.{m}"), a, id as u64, gen, line);
                    }
                }
                let mut a = vec![(None, this.clone())];
                a.extend(self.args(args)?);
                if let Some(st) = self.methods.get(m.as_str()).copied() {
                    return self.call_user(st, id, a, line);
                }
                let ns = match &this {
                    Value::Arr(_) => "array",
                    Value::Map(_) => "map",
                    Value::Mat(_) => "matrix",
                    Value::Str(_) => "str",
                    Value::Color(_) => "color",
                    Value::Handle(HandleKind::Drawing, k, _) => self.ids_dessins.place(*k).and_then(|p| self.out.drawings.get(p)).map_or("label", |d| d.kind),
                    Value::Handle(HandleKind::Table, ..) => "table",
                    Value::Handle(HandleKind::Linefill, ..) => "linefill",
                    Value::Handle(HandleKind::Polyline, ..) => "polyline",
                    Value::Obj(o) if o.borrow().ty == "chart.point" => "chart.point",
                    Value::Na => return Ok(Value::Na),
                    v => return self.err(line, format!("pas de méthode « {m} » pour un {}", v.kind())),
                };
                super::builtins::call(self, &format!("{ns}.{m}"), a, id as u64, gen, line)
            }
            _ => self.err(line, "on ne peut appeler que des fonctions"),
        }
    }

    /// 🔭 `request.security(symbol, timeframe, expression, gaps, lookahead)` (step 4): the same symbol on a higher (or
    /// equal) timeframe, made from the chart's bars (a lower one: from the intrabars, the value at each chart bar's last
    /// intrabar). The whole script runs once more on the grouped bars and the
    /// expression's value is taken at each of them. Never the future: on a chart bar, the value of the last closed
    /// higher bar (that bar's own value on the chart bar that closes it, and on the last bar, the one in progress), as
    /// Pine does with `lookahead_off`; `lookahead_on` (explicitly asked) gives the bar in progress everywhere.
    fn securite(&mut self, id: u64, args: &'a [(Option<String>, Expr)], line: u32) -> Result<Value, Error> {
        let arg = |k: usize, nom: &str| {
            args.iter()
                .find(|(n, _)| n.as_deref() == Some(nom))
                .or_else(|| args.iter().filter(|(n, _)| n.is_none()).nth(k))
                .map(|(_, e)| e)
        };
        let Some(expr) = arg(2, "expression") else {
            return self.err(line, "request.security : il manque l'expression");
        };
        let cle = (id, self.path);
        if self.sous {
            // (in a sub-run: the captured site keeps its value at each higher bar; the others are just evaluated)
            let v = self.eval(expr)?;
            self.capturer(cle, &v);
            return Ok(v);
        }
        if !self.secs.contains_key(&cle) {
            self.meme_symbole("request.security", arg(0, "symbol"), line)?;
            let t = match arg(1, "timeframe") {
                Some(e) => self.eval(e)?.text(),
                None => String::new(),
            };
            let Some(tf) = super::builtins::tf_secs(&t, self.tf) else {
                return self.err(line, format!("request.security : unité de temps inconnue « {t} »"));
            };
            let n = self.bars.len();
            if tf < self.tf {
                // 🔬 a lower timeframe, as on TradingView: on each chart bar, the expression's value at the LAST of its
                // intrabars (`na` where the intrabars do not cover the bar, or none were given: listed in
                // `Output::lower_tf_missing` so the app fetches them); the same paths as `request.security_lower_tf`.
                if !self.out.lower_tf_asked.contains(&tf) {
                    self.out.lower_tf_asked.push(tf);
                }
                let ib = self.intrabars.clone();
                let valeurs = match ib.get(&tf).filter(|b| !b.is_empty()) {
                    None => {
                        if !self.out.lower_tf_missing.contains(&tf) {
                            self.out.lower_tf_missing.push(tf);
                        }
                        vec![Value::Na; n]
                    }
                    Some(toutes) => {
                        let rapide = self.expression_seule(expr);
                        let petites = if rapide {
                            &toutes[..]
                        } else {
                            &toutes[toutes.len().saturating_sub(MAX_INTRABARS_COMPLET)..]
                        };
                        let v = if rapide {
                            self.expression_sur(expr, petites, tf)?
                        } else {
                            let mut v = self.sous_run(petites, tf, cle)?;
                            v.resize(petites.len(), Value::Na);
                            v
                        };
                        plages(self.bars, petites)
                            .into_iter()
                            .map(|(a, b)| if b > a { v[b - 1].clone() } else { Value::Na })
                            .collect()
                    }
                };
                memoire::par_bougie(n, PAR_BOUGIE_D_UN_SOUS_RUN);
                self.secs.insert(
                    cle,
                    Securite {
                        haut: (0..n).collect(),
                        ferme: vec![true; n],
                        valeurs,
                    },
                );
            } else {
                let (barres, haut) = regrouper(self.bars, tf);
                let mut valeurs = self.sous_run(&barres, tf, cle)?;
                valeurs.resize(barres.len(), Value::Na);
                let ferme = (0..n).map(|i| i + 1 < n && haut[i + 1] != haut[i]).collect();
                memoire::par_bougie(n, PAR_BOUGIE_D_UN_SOUS_RUN);
                self.secs.insert(cle, Securite { haut, ferme, valeurs });
            }
        }
        // (gaps_on: not yet; refused rather than a wrong result)
        if let Some(e) = arg(3, "gaps") {
            if self.eval(e)?.text().ends_with("gaps_on") {
                return self.err(line, "request.security : gaps = barmerge.gaps_on : pas encore dans Argos Script");
            }
        }
        let lookahead = match arg(4, "lookahead") {
            Some(e) => self.eval(e)?.text().ends_with("on"),
            None => false,
        };
        let s = &self.secs[&cle];
        let (i, n) = (self.i, self.bars.len());
        let h = s.haut[i];
        let v = if lookahead || s.ferme[i] || i + 1 == n {
            s.valeurs[h].clone()
        } else if h > 0 {
            s.valeurs[h - 1].clone()
        } else {
            Value::Na
        };
        // (F2, 22 Nov: no closed higher bar yet, or the site not reached there: a tuple expression gives a tuple of
        // na, as in Pine — `[a, …, q] = request.security(…, f())` failed with « 17 noms à gauche, mais une seule
        // valeur à droite » on the first chart bars)
        if matches!(v, Value::Na) {
            if let Some(k) = self.forme_tuple(expr) {
                return Ok(Value::tuple(vec![Value::Na; k]));
            }
        }
        Ok(v)
    }

    /// In a sub-run: keeps `v` as the captured site's value at this bar.
    fn capturer(&mut self, cle: (u64, u64), v: &Value) {
        let i = self.i;
        if let Some((_, vals)) = self.capture.as_mut().filter(|(c, _)| *c == cle) {
            if vals.len() <= i {
                vals.resize(i + 1, Value::Na);
            }
            vals[i] = v.clone();
        }
    }

    /// The whole script once more on `barres` (`tf` seconds) in a sub-run, capturing the site `cle`: its value at
    /// each of those bars (shorter when the site is not reached on the last ones).
    fn sous_run(&self, barres: &[Bar], tf: i64, cle: (u64, u64)) -> Result<Vec<Value>, Error> {
        // (B-062: one budget for the script: the sub-run counts what this run holds with what it holds itself)
        let _dessus = memoire::Dessus::ajouter(self.poids_propre());
        let mut sub = Interp::new(self.script, barres, tf, self.inputs);
        sub.sous = true;
        sub.niveau = self.niveau;
        sub.max_elements = self.max_elements;
        memoire::places(barres.len());
        sub.capture = Some((cle, vec![]));
        (sub.ticker, sub.exchange) = (self.ticker.clone(), self.exchange.clone());
        for i in 0..barres.len() {
            sub.debut_bougie(i);
            let script = sub.script;
            sub.block(&script.body, false)?;
        }
        Ok(sub.capture.take().map(|c| c.1).unwrap_or_default())
    }

    /// `request.security*` reads the chart's own symbol only: `syminfo.tickerid`, `syminfo.ticker` or nothing.
    fn meme_symbole(&mut self, fonction: &str, symbole: Option<&'a Expr>, line: u32) -> Result<(), Error> {
        let Some(e) = symbole else {
            return Ok(());
        };
        let sym = self.eval(e)?.text();
        let (ticker, tickerid) = (super::builtins::ticker(self), super::builtins::tickerid(self));
        if !sym.is_empty() && sym != tickerid && sym != ticker {
            return self.err(
                line,
                format!("{fonction} : un autre symbole ({sym}) : pas encore dans Argos Script (seulement celui de la chart)"),
            );
        }
        Ok(())
    }

    /// 🔬 `request.security_lower_tf(symbol, timeframe, expression, ignore_invalid_symbol, currency,
    /// ignore_invalid_timeframe, calc_bars_count)`, like TradingView: on each chart bar, an ARRAY with the expression's
    /// value at each lower-timeframe bar whose time falls in [this chart bar's time, the next chart bar's time) — the
    /// last chart bar takes every intrabar from its time on (the bar in progress: its intrabars so far). A tuple
    /// expression gives a tuple of arrays. A chart bar the intrabars do not cover from its first second gets empty
    /// arrays, never an error (TradingView outside its intrabar history); a timeframe the app has not given
    /// (`RunOptions::intrabars`) gives empty arrays everywhere and is listed in `Output::lower_tf_missing`.
    ///
    /// Fast path, the normal case: an expression that reads no script variable and calls no script function
    /// (`[open, close, volume]`) is evaluated alone on each intrabar. Otherwise the whole script runs once on the
    /// intrabars, capturing this site, like `request.security`. `currency` and `calc_bars_count` are ignored.
    fn securite_ltf(&mut self, id: u64, args: &'a [(Option<String>, Expr)], line: u32) -> Result<Value, Error> {
        const F: &str = "request.security_lower_tf";
        let arg = |k: usize, nom: &str| {
            args.iter()
                .find(|(n, _)| n.as_deref() == Some(nom))
                .or_else(|| args.iter().filter(|(n, _)| n.is_none()).nth(k))
                .map(|(_, e)| e)
        };
        let Some(expr) = arg(2, "expression") else {
            return self.err(line, format!("{F} : il manque l'expression"));
        };
        let cle = (id, self.path);
        let litteral = self.forme_tuple(expr);
        if self.sous {
            // (in a sub-run: the value on this bar, captured when it is the captured site, as a one-element array)
            let v = self.eval(expr)?;
            self.capturer(cle, &v);
            let tuple = litteral.or(match &v {
                Value::Tuple(t) => Some(t.len()),
                _ => None,
            });
            return Ok(tableaux(std::slice::from_ref(&v), tuple));
        }
        if !self.ltfs.contains_key(&cle) {
            self.meme_symbole(F, arg(0, "symbol"), line)?;
            let t = match arg(1, "timeframe") {
                Some(e) => self.eval(e)?.text(),
                None => String::new(),
            };
            let Some(tf) = super::builtins::tf_secs(&t, self.tf) else {
                return self.err(line, format!("{F} : unité de temps inconnue « {t} »"));
            };
            let ignorer = match arg(5, "ignore_invalid_timeframe") {
                Some(e) => self.eval(e)?.truthy(),
                None => false,
            };
            let n = self.bars.len();
            let vide = |na_partout| Intrabar {
                plages: vec![(0, 0); n],
                valeurs: vec![],
                tuple: litteral,
                na_partout,
            };
            let site = if tf > self.tf {
                if !ignorer {
                    return self.err(line, format!("{F} : l'unité demandée ({t}) est plus grande que celle de la chart"));
                }
                vide(true)
            } else if tf == self.tf {
                // (the chart's own timeframe: the expression on the chart bar itself, one element)
                let v = self.eval(expr)?;
                let tuple = litteral.or(match &v {
                    Value::Tuple(t) => Some(t.len()),
                    _ => None,
                });
                return Ok(tableaux(std::slice::from_ref(&v), tuple));
            } else {
                if !self.out.lower_tf_asked.contains(&tf) {
                    self.out.lower_tf_asked.push(tf);
                }
                let ib = self.intrabars.clone();
                match ib.get(&tf).filter(|b| !b.is_empty()) {
                    None => {
                        if !self.out.lower_tf_missing.contains(&tf) {
                            self.out.lower_tf_missing.push(tf);
                        }
                        vide(false)
                    }
                    Some(toutes) => {
                        let rapide = self.expression_seule(expr);
                        // (the full path runs the whole script on the intrabars: the newest MAX_INTRABARS_COMPLET only,
                        // older chart bars get empty arrays, as outside the intrabar history)
                        let petites = if rapide {
                            &toutes[..]
                        } else {
                            &toutes[toutes.len().saturating_sub(MAX_INTRABARS_COMPLET)..]
                        };
                        let valeurs = if rapide {
                            self.expression_sur(expr, petites, tf)?
                        } else {
                            let mut v = self.sous_run(petites, tf, cle)?;
                            v.resize(petites.len(), Value::Na);
                            v
                        };
                        let tuple = litteral.or(valeurs.iter().find_map(|v| match v {
                            Value::Tuple(t) => Some(t.len()),
                            _ => None,
                        }));
                        Intrabar {
                            plages: plages(self.bars, petites),
                            valeurs,
                            tuple,
                            na_partout: false,
                        }
                    }
                }
            };
            memoire::par_bougie(self.bars.len(), PAR_BOUGIE_D_UN_SOUS_RUN);
            self.ltfs.insert(cle, site);
        }
        let s = &self.ltfs[&cle];
        if s.na_partout {
            return Ok(match s.tuple {
                Some(n) => Value::tuple(vec![Value::Na; n]),
                None => Value::Na,
            });
        }
        let (a, b) = s.plages[self.i];
        Ok(tableaux(&s.valeurs[a..b], s.tuple))
    }

    /// How many values an expression gives, read from the script without running it: a tuple `[a, b]`, or a call to a
    /// script function whose last line is one (`f_mtfDivs()`). `None`: a single value (or not known).
    fn forme_tuple(&self, e: &Expr) -> Option<usize> {
        // (a loop with an end, B-075: it called itself once per function, and a function whose last line calls
        // itself, `f() => f()`, has no end — the stack ended before the script ran. Past the calls one in the other
        // a script may make, no shape: the calculation then refuses the script with its line. One pass for each of
        // the 65 functions a calculation runs one in the other, and one to read what the last one gives)
        let mut e = e;
        for _ in 0..=MAX_APPELS_SCRIPT + 1 {
            match &e.k {
                Ek::Tuple(items) => return Some(items.len()),
                Ek::Call { f, .. } => {
                    let Ek::Ident(n) = &f.k else { return None };
                    let Some(Stmt::Fn { body, .. }) = self.fns.get(n.as_str()) else { return None };
                    let Some(Stmt::Expr(x)) = body.last() else { return None };
                    e = x;
                }
                _ => return None,
            }
        }
        None
    }

    /// The fast path of `request.security_lower_tf` applies: the expression reads no script variable and calls no
    /// script function (only bar series, literals, operators, tuples and built-ins). Decided from the expression.
    fn expression_seule(&self, e: &Expr) -> bool {
        let ok = |x: &Expr| self.expression_seule(x);
        match &e.k {
            Ek::Num(_) | Ek::Str(_) | Ek::Bool(_) | Ek::Na | Ek::Color(_) => true,
            Ek::Ident(n) => self.lookup(n).is_none() && !self.fns.contains_key(n.as_str()),
            Ek::Member(b, _) => match &b.k {
                Ek::Ident(ns) => self.lookup(ns).is_none() && super::builtins::is_namespace(ns),
                _ => false,
            },
            Ek::Call { f, args, .. } => {
                let fonction = match &f.k {
                    Ek::Ident(n) => !self.fns.contains_key(n.as_str()) && self.lookup(n).is_none(),
                    Ek::Member(b, m) => {
                        matches!(&b.k, Ek::Ident(ns) if self.lookup(ns).is_none() && super::builtins::is_namespace(ns))
                            && !(m.starts_with("security") || self.methods.contains_key(m.as_str()))
                    }
                    _ => false,
                };
                fonction && args.iter().all(|a| ok(&a.1))
            }
            Ek::Index { e, n, .. } => ok(e) && ok(n),
            Ek::Unary(_, a) => ok(a),
            Ek::Binary(_, a, b) => ok(a) && ok(b),
            Ek::Ternary(c, a, b) => ok(c) && ok(a) && ok(b),
            Ek::Tuple(v) => v.iter().all(ok),
            _ => false,
        }
    }

    /// The expression alone on each of `petites` (`tf` seconds), in order, in its own interpreter (its `ta.*`
    /// memory runs along the intrabars).
    fn expression_sur(&self, e: &'a Expr, petites: &[Bar], tf: i64) -> Result<Vec<Value>, Error> {
        let _dessus = memoire::Dessus::ajouter(self.poids_propre());
        let mut sub = Interp::new(self.script, petites, tf, self.inputs);
        sub.sous = true;
        sub.niveau = self.niveau;
        sub.max_elements = self.max_elements;
        (sub.ticker, sub.exchange) = (self.ticker.clone(), self.exchange.clone());
        // (B-062: the values are kept where the memory count sees them, and each intrabar is a turn: nothing else
        // counts on this path, which runs no statement)
        memoire::places(petites.len());
        sub.capture = Some((SANS_SITE, Vec::with_capacity(petites.len())));
        for i in 0..petites.len() {
            sub.debut_bougie(i);
            let v = sub.eval(e)?;
            if let Some((_, out)) = sub.capture.as_mut() {
                out.push(v);
            }
            sub.tick(e.line)?;
        }
        Ok(sub.capture.take().map(|c| c.1).unwrap_or_default())
    }

    fn construct(&mut self, ty: &str, fields: &'a [(String, Option<Expr>)], args: Vec<(Option<&str>, Value)>, line: u32) -> Result<Value, Error> {
        let mut vals = Vec::with_capacity(fields.len());
        for (k, (name, def)) in fields.iter().enumerate() {
            let given = args
                .iter()
                .find(|(n, _)| *n == Some(name.as_str()))
                .or_else(|| args.get(k).filter(|(n, _)| n.is_none()));
            let v = match (given, def) {
                (Some((_, v)), _) => v.clone(),
                (None, Some(d)) => self.eval(d)?,
                (None, None) => Value::Na,
            };
            vals.push((name.clone(), v));
        }
        if args.len() > fields.len() {
            return self.err(line, format!("{ty}.new : {} champs, {} valeurs", fields.len(), args.len()));
        }
        Ok(Value::obj(Obj {
            ty: ty.to_string(),
            fields: vals,
        }))
    }

    /// Appel d'une fonction du script : ses paramètres sont des variables de cet appel (avec historique).
    pub fn call_user(&mut self, st: &'a Stmt, id: u32, args: Vec<(Option<&str>, Value)>, line: u32) -> Result<Value, Error> {
        let Stmt::Fn {
            id: fid, name, params, body, ..
        } = st
        else {
            return Ok(Value::Na);
        };
        if self.depth > MAX_APPELS_SCRIPT {
            return self.err(line, format!("« {name} » s'appelle elle-même trop de fois (pas de récursion en Pine)"));
        }
        if args.len() > params.len() {
            return self.err(line, format!("« {name} » prend {} paramètres, {} donnés", params.len(), args.len()));
        }
        // valeurs des paramètres : positionnels, nommés, puis défauts
        let mut vals = Vec::with_capacity(params.len());
        for (k, (p, def)) in params.iter().enumerate() {
            let given = args
                .iter()
                .find(|(n, _)| *n == Some(p.as_str()))
                .or_else(|| args.get(k).filter(|(n, _)| n.is_none()));
            vals.push(match (given, def) {
                (Some((_, v)), _) => v.clone(),
                (None, Some(d)) => self.eval(d)?,
                (None, None) => return self.err(line, format!("« {name} » : il manque « {p} »")),
            });
        }
        let (old_path, old_frame) = (self.path, self.frame);
        self.path = mix(old_path, id as u64);
        self.frame = self.scopes.len();
        self.depth += 1;
        self.scopes.push(HashMap::new());
        for (k, ((p, _), v)) in params.iter().zip(vals).enumerate() {
            self.declare((*fid as u64 | 1 << 41, self.path, k as u8), p.as_str(), v);
        }
        let r = self.block(body, false);
        self.scopes.truncate(self.frame);
        self.depth -= 1;
        (self.path, self.frame) = (old_path, old_frame);
        Ok(match r? {
            Flow::Val(v) => v,
            _ => Value::Na,
        })
    }

    /// Appel d'une lambda (extra Argos) : elle voit les variables autour d'elle.
    pub fn call_lambda(&mut self, idx: u32, args: &[Value], line: u32) -> Result<Value, Error> {
        let Some(e) = self.lambdas.get(idx as usize).copied() else {
            return self.err(line, "lambda inconnue");
        };
        let Ek::Lambda(ps, body) = &e.k else {
            return self.err(line, "lambda inconnue");
        };
        self.scopes.push(HashMap::new());
        for (k, p) in ps.iter().enumerate() {
            let v = args.get(k).cloned().unwrap_or(Value::Na);
            self.declare((e as *const Expr as u64 | 1 << 42, self.path, k as u8), p.as_str(), v);
        }
        let r = self.eval(body);
        self.scopes.pop();
        self.tick(line)?;
        r
    }
}
