//! # 🦀 Le langage Argos Script
//! Compatible avec la syntaxe de **Pine Script v6** (comme LuxAlgo fait rouler le vrai Pine dans Vela) : on colle un
//! script Pine, il roule tel quel, bougie par bougie. Plus quelques extras d'Argos (lambdas sur les tableaux :
//! `map`, `filter`, `reduce`, `some`, `every`, `count`, `find`).
//!
//! `lexer` (texte → jetons) → `parser` (jetons → arbre) → `interp` (l'arbre roule sur chaque bougie) ; `builtins` :
//! les fonctions de Pine (`ta`, `math`, `array`, `map`, `str`, `color`, `input`, `plot`…).
//!
//! **`and` / `or` comme la version du script** : en Pine v5 et avant (`//@version=5`), les deux côtés sont toujours
//! calculés ; en v6, le côté droit est sauté quand le gauche décide déjà. Une fonction à mémoire (`ta.crossover`,
//! `ta.sma`…) placée à droite d'un `and` / `or` ou dans une branche (`?:`, `if`, `switch`) peut alors manquer des
//! bougies : Argos le signale comme TradingView, et le conseil est de la calculer sur sa propre ligne.

pub mod ast;
pub mod builtins;
pub mod comparer;
pub mod dialect;
pub mod interp;
pub mod lexer;
mod memoire;
pub mod parser;
mod rangs;
pub mod rapport;
pub mod reglages;
pub mod sorties;
pub mod strategie;
pub mod value;

use argos_core::Bar;
pub use lexer::Error;
use std::collections::HashMap;
/// Un tracé qu'on peut remplir : `plot` (index dans `plots`) ou `hline` (index dans `hlines`).
pub type Tracer = (value::HandleKind, usize);

/// Un input déclaré par le script (`input.int(14, "Longueur")`).
#[derive(Debug, Clone, PartialEq)]
pub struct InputSpec {
    /// Titre (ou « Input N » sans titre) : c'est la clé des valeurs choisies.
    pub title: String,
    /// int, float, bool, string, source, color, price, time, session, symbol, text_area, timeframe.
    pub kind: &'static str,
    pub def: InVal,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub step: Option<f64>,
    pub options: Vec<String>,
    pub tooltip: String,
    pub group: String,
    pub inline: String,
    pub confirm: bool,
}

/// Valeur d'un input (choisie ou par défaut).
#[derive(Debug, Clone, PartialEq)]
pub enum InVal {
    Num(f64),
    Bool(bool),
    Text(String),
    Color(u32),
}

/// Style d'un tracé.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlotStyle {
    #[default]
    Line,
    LineBr,
    StepLine,
    Histogram,
    Columns,
    Circles,
    Cross,
    Area,
}

/// Un `plot` : une valeur et une couleur par bougie.
#[derive(Debug, Clone, PartialEq)]
pub struct Plot {
    pub title: String,
    pub values: Vec<f64>,
    /// Couleur à chaque bougie (`None` : pas de couleur, le point n'est pas dessiné).
    pub colors: Vec<Option<u32>>,
    pub width: f64,
    pub style: PlotStyle,
    pub offset: i64,
    /// Affiché sur la chart (`display = display.none` le cache).
    pub visible: bool,
}

/// Où se met une forme de `plotshape` / `plotchar`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Loc {
    AboveBar,
    BelowBar,
    Absolute,
    Top,
    Bottom,
}

/// Une forme posée à une bougie.
#[derive(Debug, Clone, PartialEq)]
pub struct Shape {
    pub bar: usize,
    /// Prix (pour `location.absolute`, sinon le haut / bas de la bougie).
    pub price: f64,
    pub loc: Loc,
    /// Forme (`shape.triangleup`…) ou caractère de `plotchar`.
    pub shape: String,
    pub color: u32,
    pub text: String,
    pub text_color: u32,
    pub size: String,
    pub title: String,
    /// `force_overlay = true` (Pine v6): drawn on the price pane even when the script has its own band.
    pub force_overlay: bool,
}

/// Un dessin fait par le script (`label.new`, `line.new`, `box.new`).
#[derive(Debug, Clone, PartialEq)]
pub struct Drawing {
    pub kind: &'static str,
    /// Coordonnées : x en index de bougie (ou en temps ms si `xloc.bar_time`), y en prix.
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
    pub xtime: bool,
    pub text: String,
    pub color: u32,
    pub bg: u32,
    pub text_color: u32,
    pub style: String,
    pub width: f64,
    /// Ligne prolongée : "none", "left", "right", "both".
    pub extend: String,
    pub size: String,
    pub deleted: bool,
    /// `force_overlay = true` (Pine v6): drawn on the price pane even when the script has its own band.
    pub force_overlay: bool,
}

/// 〰️ A `polyline.new` path: its points (x as bar index, or time in ms with `xloc.bar_time`; y as price), joined by
/// straight segments (or a smooth curve when `curved`), back to the first one when `closed`, filled with `fill`.
#[derive(Debug, Clone, PartialEq)]
pub struct Polyline {
    pub points: Vec<(f64, f64)>,
    pub xtime: bool,
    pub curved: bool,
    pub closed: bool,
    pub color: u32,
    /// Fill colour (`None`: no fill).
    pub fill: Option<u32>,
    pub style: String,
    pub width: f64,
    pub deleted: bool,
    pub force_overlay: bool,
}

/// 🕯 `plotcandle` / `plotbar`: candles (or OHLC bars) of the script's own, one per bar; a bar where any of the four
/// values is `na` draws nothing (as in Pine).
#[derive(Debug, Clone, PartialEq)]
pub struct Candles {
    pub title: String,
    /// open, high, low, close at each bar.
    pub ohlc: Vec<[f64; 4]>,
    /// Body colour at each bar (`None`: nothing drawn on that bar).
    pub color: Vec<Option<u32>>,
    pub wick: Vec<Option<u32>>,
    pub border: Vec<Option<u32>>,
    /// `plotbar`: an OHLC bar (a vertical line, ticks left for the open and right for the close) instead of a candle.
    pub bars: bool,
    pub visible: bool,
}

/// A fill between two lines (`linefill.new`): the indices of the two `line` drawings in `Output::drawings`, the
/// colour. Not drawn once deleted, or once one of its lines is deleted (like TradingView).
#[derive(Debug, Clone, PartialEq)]
pub struct Linefill {
    pub line1: usize,
    pub line2: usize,
    pub color: u32,
    pub deleted: bool,
}

/// 📋 A table of the script (`table.new`), drawn in a corner of the chart.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Table {
    /// « top_right », « bottom_left », « middle_center »…
    pub position: String,
    pub columns: usize,
    pub rows: usize,
    pub bgcolor: Option<u32>,
    pub border_color: Option<u32>,
    pub frame_color: Option<u32>,
    /// Cells by row then column (`None`: empty).
    pub cells: Vec<Vec<Option<TableCell>>>,
    pub deleted: bool,
}

/// A table cell.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TableCell {
    pub text: String,
    pub bgcolor: Option<u32>,
    pub text_color: u32,
    /// « tiny », « small », « normal », « large », « huge », « auto ».
    pub size: String,
}

/// Tout ce que le script a produit.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Output {
    pub title: String,
    pub short: String,
    pub overlay: bool,
    pub inputs: Vec<InputSpec>,
    pub plots: Vec<Plot>,
    /// Lignes horizontales : prix, titre, couleur, style.
    pub hlines: Vec<(f64, String, u32, String)>,
    pub shapes: Vec<Shape>,
    /// Couleur de fond / des bougies à chaque bougie.
    pub bgcolor: Vec<Option<u32>>,
    pub barcolor: Vec<Option<u32>>,
    /// Remplissages entre deux tracés (plot ou hline) : (a, b, couleur par bougie).
    pub fills: Vec<(Tracer, Tracer, Vec<Option<u32>>)>,
    pub drawings: Vec<Drawing>,
    /// `alert()` et `log.info()` : (bougie, message).
    pub logs: Vec<(usize, String)>,
    /// 🔔 `alertcondition(condition, title, message)` (step 4): title, message, and 1 on the bars where the condition
    /// is true (0 elsewhere). The app offers them as named signals in the alerts window.
    pub alertconditions: Vec<(String, String, Vec<f64>)>,
    /// 📋 `table.new` / `table.cell` (step 4): the tables as they are at the last bar (that is what TradingView shows).
    pub tables: Vec<Table>,
    /// `linefill.new` (step 4): fills between two `line` drawings, drawn from where the lines are at the end.
    pub linefills: Vec<Linefill>,
    /// 〰️ `polyline.new`: paths through `chart.point`s.
    pub polylines: Vec<Polyline>,
    /// 🕯 `plotcandle` / `plotbar`.
    pub candles: Vec<Candles>,
    /// 🌐 Langue reconnue du script (traduit en Pine v6 s'il le faut) et ce qui a été traduit à peu près.
    pub lang: Option<dialect::Dialect>,
    pub notes: Vec<(u32, String)>,
    /// 🧪 A `strategy(...)` script: its trades, its fills (arrows on the chart) and its equity, bar by bar.
    pub strategie: Option<strategie::Bilan>,
    /// 🔬 The lower timeframes (seconds) that `request.security_lower_tf` asked for and that [`RunOptions::intrabars`]
    /// did not hold (the script got empty arrays there): the app loads them and runs the script again.
    pub lower_tf_missing: Vec<i64>,
    /// Every lower timeframe (seconds) that `request.security_lower_tf` asked for, given or not: the app keeps loading
    /// them while the script asks.
    pub lower_tf_asked: Vec<i64>,
    /// The script holds at least one call to an output function (plot, shapes, drawings, tables, alerts, strategy
    /// orders), anywhere — read from the parsed script, see [`sorties::has_output_calls`].
    pub has_output_calls: bool,
}

impl Output {
    /// 🧮 The bytes this output holds (B-062): its lists and their texts. Every field is named: a new one has to be
    /// placed, counted or not.
    pub(crate) fn poids(&self) -> usize {
        use std::mem::size_of;
        let Output {
            title,
            short,
            overlay: _,
            inputs,
            plots,
            hlines,
            shapes,
            bgcolor,
            barcolor,
            fills,
            drawings,
            logs,
            alertconditions,
            tables,
            linefills,
            polylines,
            candles,
            // (read once from the source)
            lang: _,
            notes,
            // (made after the last bar, from the broker the interpreter counts)
            strategie: _,
            lower_tf_missing,
            lower_tf_asked,
            has_output_calls: _,
        } = self;
        let t = String::capacity;
        let couleurs = |v: &Vec<Option<u32>>| v.capacity().saturating_mul(size_of::<Option<u32>>());
        // (a list: its places, then what each entry holds besides its place)
        fn liste<T>(v: &Vec<T>, dedans: impl Fn(&T) -> usize) -> usize {
            v.capacity().saturating_mul(size_of::<T>()).saturating_add(v.iter().map(dedans).sum())
        }
        let textes = |v: &Vec<String>| liste(v, t);
        [
            t(title),
            t(short),
            liste(inputs, |s| {
                let def = if let InVal::Text(x) = &s.def { t(x) } else { 0 };
                t(&s.title) + t(&s.tooltip) + t(&s.group) + t(&s.inline) + textes(&s.options) + def
            }),
            liste(plots, |p| {
                t(&p.title) + p.values.capacity().saturating_mul(size_of::<f64>()) + couleurs(&p.colors)
            }),
            liste(hlines, |h| t(&h.1) + t(&h.3)),
            liste(shapes, |s| t(&s.shape) + t(&s.text) + t(&s.size) + t(&s.title)),
            couleurs(bgcolor),
            couleurs(barcolor),
            liste(fills, |f| couleurs(&f.2)),
            liste(drawings, |d| t(&d.text) + t(&d.style) + t(&d.extend) + t(&d.size)),
            liste(logs, |l| t(&l.1)),
            liste(alertconditions, |a| t(&a.0) + t(&a.1) + a.2.capacity().saturating_mul(size_of::<f64>())),
            liste(tables, |x| {
                let cellule = |c: &Option<TableCell>| c.as_ref().map_or(0, |c| t(&c.text) + t(&c.size));
                t(&x.position) + liste(&x.cells, |ligne| liste(ligne, cellule))
            }),
            liste(linefills, |_| 0),
            liste(polylines, |p| t(&p.style) + p.points.capacity().saturating_mul(size_of::<(f64, f64)>())),
            liste(candles, |c| {
                t(&c.title) + c.ohlc.capacity().saturating_mul(size_of::<[f64; 4]>()) + couleurs(&c.color) + couleurs(&c.wick) + couleurs(&c.border)
            }),
            liste(notes, |n| t(&n.1)),
            liste(lower_tf_missing, |_| 0),
            liste(lower_tf_asked, |_| 0),
        ]
        .iter()
        .fold(0, |a, b| a.saturating_add(*b))
    }
}

/// Fait rouler `src` sur les bougies (`tf` : durée d'une bougie en secondes) avec les inputs choisis (par titre).
/// 🌐 Le script peut être en Pine v4 / v5 / v6, en ancien Argos Script (v1) ou en wrun : il est traduit en Pine v6
/// d'abord (`dialect`), et les erreurs pointent la ligne du script d'origine.
pub fn run(src: &str, bars: &[Bar], tf: i64, inputs: &HashMap<String, InVal>) -> Result<Output, Error> {
    run_with(src, bars, tf, inputs, &RunOptions::default())
}

/// 🧮 What the app knows about the bars beyond OHLCV, for [`run_with`]. The default (nothing known) is what [`run`]
/// uses: every bar closed, no real trades.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RunOptions {
    /// `argos.real_delta`: buys minus sells of each bar from the real trades (`argos_core::orderflow::FlowBar`), one
    /// value per bar (NaN: no complete trades for that bar → `na`). Empty: `na` on every bar.
    pub real_delta: Vec<f64>,
    /// The last bar is still forming (not closed yet): `barstate.isconfirmed` is false on it, like TradingView's
    /// realtime bar.
    pub last_open: bool,
    /// 🔬 Bars of lower timeframes of the SAME symbol, for `request.security_lower_tf`: timeframe in seconds → bars,
    /// oldest first. Shared, never copied by a run (the app keeps one table for every script).
    pub intrabars: std::sync::Arc<Intrabars>,
    /// The chart's symbol as its header shows it (`syminfo.ticker`) and its exchange (`syminfo.tickerid` is
    /// « EXCHANGE:symbol »). Empty: « ARGOS », as before.
    pub ticker: String,
    pub exchange: String,
}

/// Bars of lower timeframes: timeframe in seconds → bars, oldest first (the shape of `ai_trend::LowerBars`).
pub type Intrabars = HashMap<i64, Vec<Bar>>;

/// The most bytes a script holds (B-074): the largest scripts known are about 50 KB. Reading a script takes memory
/// outside any budget — the conversion of an old Pine keeps a copy of a line for each call it goes down, the reader
/// about 40 bytes for each byte — and a line of 15 MB took 1.4 GB, past what the app may ask for.
pub const MAX_SOURCE: usize = 1_000_000;

/// A script longer than [`MAX_SOURCE`] is refused, at the line where it passes it.
fn taille(src: &str) -> Result<(), Error> {
    if src.len() <= MAX_SOURCE {
        return Ok(());
    }
    let ligne = src.as_bytes()[..MAX_SOURCE].iter().filter(|b| **b == b'\n').count() + 1;
    Err(Error {
        line: u32::try_from(ligne).unwrap_or(u32::MAX),
        msg: "script trop long : 1 Mo au plus (il dépasse à cette ligne)".into(),
    })
}

/// Like [`run`], with what the app knows beyond OHLCV (real trades, a last bar still forming).
pub fn run_with(src: &str, bars: &[Bar], tf: i64, inputs: &HashMap<String, InVal>, opts: &RunOptions) -> Result<Output, Error> {
    taille(src)?;
    let conv = dialect::to_pine(src);
    let at = |e: Error| Error {
        line: conv.origin(e.line),
        msg: e.msg,
    };
    let script = parser::parse(lexer::lex(&conv.pine).map_err(at)?).map_err(at)?;
    let mut it = interp::Interp::new(&script, bars, tf, inputs);
    regle_et_ou(&mut it, &conv.dialect);
    it.real_delta = opts.real_delta.clone();
    it.last_open = opts.last_open;
    it.intrabars = opts.intrabars.clone();
    (it.ticker, it.exchange) = (opts.ticker.clone(), opts.exchange.clone());
    let mut out = it.run().map_err(at)?;
    out.has_output_calls = sorties::has_output_calls(&script);
    notes_du_script(&mut out, &conv);
    Ok(out)
}

/// 🧪 For the tests (B-050): the collections, objects and texts the variables still hold in their history once the
/// script has run on every bar, as [`run`] runs it.
#[cfg(test)]
pub(crate) fn lourds_gardes(src: &str, bars: &[Bar], tf: i64) -> Result<usize, Error> {
    let conv = dialect::to_pine(src);
    let script = parser::parse(lexer::lex(&conv.pine)?)?;
    let inputs = HashMap::new();
    let mut it = interp::Interp::new(&script, bars, tf, &inputs);
    regle_et_ou(&mut it, &conv.dialect);
    it.tourner()?;
    Ok(it.lourds_gardes())
}

/// 🧪 For the tests (B-062): the values of any kind the variables still hold in their history once the script has
/// run on every bar, as [`run`] runs it.
#[cfg(test)]
pub(crate) fn passe_garde(src: &str, bars: &[Bar], tf: i64) -> Result<usize, Error> {
    let conv = dialect::to_pine(src);
    let script = parser::parse(lexer::lex(&conv.pine)?)?;
    let inputs = HashMap::new();
    let mut it = interp::Interp::new(&script, bars, tf, &inputs);
    regle_et_ou(&mut it, &conv.dialect);
    it.tourner()?;
    Ok(it.passe_garde())
}

/// 📜 Like [`run`], and also gives the series of some top-level variables of the script, by name (their value at
/// every bar): what the script computes without drawing it (Spring Tension's `rsiVal`, `mfiVal`), read without
/// changing the script. A name the script does not declare at its top level is absent from the table.
pub fn run_avec_variables(
    src: &str,
    bars: &[Bar],
    tf: i64,
    inputs: &HashMap<String, InVal>,
    noms: &[&str],
) -> Result<(Output, HashMap<String, Vec<f64>>), Error> {
    taille(src)?;
    let conv = dialect::to_pine(src);
    let at = |e: Error| Error {
        line: conv.origin(e.line),
        msg: e.msg,
    };
    let script = parser::parse(lexer::lex(&conv.pine).map_err(at)?).map_err(at)?;
    let mut it = interp::Interp::new(&script, bars, tf, inputs);
    regle_et_ou(&mut it, &conv.dialect);
    let (mut out, series) = it.run_vars(noms).map_err(at)?;
    out.has_output_calls = sorties::has_output_calls(&script);
    notes_du_script(&mut out, &conv);
    Ok((out, series))
}

/// 🧮 `and` / `or` as in the script's Pine version (kept by `dialect` from its `//@version=`): v5 and before evaluate
/// both sides always, v6 skips the right one when the left decides — and then warns, like TradingView, when a function
/// with memory may be skipped. (Pine v6 is also what a script without a version, or in another language, is run as.)
fn regle_et_ou(it: &mut interp::Interp, d: &dialect::Dialect) {
    it.strict_et_ou = matches!(d, dialect::Dialect::PineOld(v) if *v <= 5);
    it.avertir_branches = matches!(d, dialect::Dialect::Pine6);
    it.boucles_dynamiques = matches!(d, dialect::Dialect::Pine6);
}

/// The notes of a run: what the translation changed, then the interpreter's warnings (at the script's own lines).
fn notes_du_script(out: &mut Output, conv: &dialect::Converted) {
    let avertis: Vec<(u32, String)> = std::mem::take(&mut out.notes).into_iter().map(|(l, m)| (conv.origin(l), m)).collect();
    out.lang = Some(conv.dialect);
    out.notes = conv.notes.clone();
    out.notes.extend(avertis);
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_dessins;
#[cfg(test)]
mod tests_imbrication;
#[cfg(test)]
mod tests_intrabar;
#[cfg(test)]
mod tests_memoire;
#[cfg(test)]
mod tests_recettes;
#[cfg(test)]
mod tests_references;
#[cfg(test)]
mod tests_strategie;
