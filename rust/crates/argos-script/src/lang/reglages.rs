//! ⚙ Step 3 of the Argos Script engine: the settings window of a user script, made from its `input()` calls. The
//! inputs become the same `Param` table the built-in indicators use (groups = the `group` of each input, key = its
//! title), so the generic settings window shows them; the values chosen there go back to the script as the
//! title → value map `run` takes.

use super::{InVal, InputSpec};
use crate::inputs::{Kind, Param};
use std::collections::HashMap;
use std::sync::Mutex;

/// Script source names, in the order of `inputs::SOURCES`.
const NOMS_SOURCES: [&str; 9] = ["close", "open", "high", "low", "hl2", "hlc3", "ohlc4", "volume", "hlcc4"];

/// A string that lives as long as the program (the tables are made once per distinct set of inputs).
fn fixe(s: &str) -> &'static str {
    Box::leak(s.to_string().into_boxed_str())
}

/// The settings table of a script: (groups, inputs).
pub type Table = (&'static [&'static str], &'static [Param]);

/// What the tables of the user scripts may keep in one session, in bytes (B-071). A table lives as long as the
/// program: the settings window reads it like the table of a built-in indicator, which is part of the program. One
/// is made per distinct set of inputs opened in ⚙, so a script edited many times makes one per version: a few KB
/// each, 32 MB is thousands of versions. Before, nothing bounded them: a script whose inputs change with the chart
/// (a tooltip made from the price, an input per bar) left up to 60 MB for good at each opening of ⚙.
const MAX_TABLES: usize = 32 << 20;

/// The tables made so far, by the text of their inputs, and the bytes they keep for good.
#[derive(Default)]
struct Tables {
    par_cle: HashMap<String, Table>,
    garde: usize,
}

static TABLES: Mutex<Option<Tables>> = Mutex::new(None);

/// Why a user script gets no settings table (B-071).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Plein {
    /// The table of this script alone is more than all the tables may keep.
    Script,
    /// The tables made since the app was started keep all they may.
    Session,
}

/// What an input is in the settings window, read from the input alone.
enum Sorte<'s> {
    Num(f64),
    Bool,
    Color,
    Price,
    Time,
    Session,
    Symbol,
    Source,
    Choice(&'s [String]),
    Text { multi: bool },
}

fn sorte(s: &InputSpec) -> Sorte<'_> {
    match s.kind {
        "int" => Sorte::Num(1.0),
        "float" => Sorte::Num(0.1),
        "price" => Sorte::Price,
        "time" => Sorte::Time,
        "session" => Sorte::Session,
        "bool" => Sorte::Bool,
        "color" => Sorte::Color,
        "symbol" => Sorte::Symbol,
        "source" => Sorte::Source,
        "string" if !s.options.is_empty() => Sorte::Choice(&s.options),
        "text_area" => Sorte::Text { multi: true },
        _ => Sorte::Text { multi: false },
    }
}

/// The value the window shows for the text `x` of an input of this sort (a list or a source: its index); `None`
/// when the text is kept as a text.
fn valeur_du_texte(sorte: &Sorte, x: &str) -> Option<f64> {
    match sorte {
        Sorte::Choice(o) => Some(o.iter().position(|v| v == x).unwrap_or(0) as f64),
        Sorte::Source => Some(NOMS_SOURCES.iter().position(|v| *v == x).unwrap_or(0) as f64),
        Sorte::Session => Some(crate::inputs::session_val(x)),
        _ => None,
    }
}

/// The default of an input as the window holds it.
fn defaut(s: &InputSpec) -> f64 {
    match &s.def {
        InVal::Num(v) => *v,
        InVal::Bool(b) => *b as u8 as f64,
        InVal::Color(c) => *c as f64,
        InVal::Text(x) => valeur_du_texte(&sorte(s), x).unwrap_or(0.0),
    }
}

/// The group of the inputs that name none.
const SANS_GROUPE: &str = "Inputs";

/// What one table takes in the table of tables, on the high side: the block of four places (its key and its table,
/// and a control byte each) and of a control group. The table of tables keeps up to 2.3 places for each table as it
/// grows, and 4 places for the first one (a block of 256 bytes on a 64-bit machine).
const PLACE: usize = bloc(4 * (std::mem::size_of::<(String, Table)>() + 1) + 16);

/// A text of `n` bytes kept for good: the block the allocator gives it, counted on the high side on every build (none
/// for an empty text). A block has a header and is rounded: on a 64-bit machine (glibc) 8 bytes of header, rounded to
/// 16, at least 32 (a text of one byte takes 32); in the web build (dlmalloc, 32 bits) 4 bytes, rounded to 8, at
/// least 16. Counted here as two words of header, rounded to two words (so at least four).
const fn bloc(n: usize) -> usize {
    const MOT: usize = std::mem::size_of::<usize>();
    if n == 0 {
        return 0;
    }
    match n.saturating_add(2 * MOT).checked_next_multiple_of(2 * MOT) {
        Some(x) => x,
        None => usize::MAX,
    }
}

/// The bytes a table of these inputs keeps for good besides its key (B-071): its texts, a block each (the title
/// twice: key and label; the group, « Inputs » when the input names none), its two lists and its place among the
/// tables. At least what is kept: it said 1.14 to 1.5 times less (a list of 2 000 options of one character, 500
/// inputs with a title of one character, a help text of 10 000 characters), so the ceiling let that much more be
/// kept; and on a 64-bit machine a block of a short text takes 32 bytes, where 16 more than the text were counted.
fn poids_sans_cle(specs: &[InputSpec]) -> usize {
    use std::mem::size_of;
    let textes = |s: &InputSpec| {
        let def = if let InVal::Text(x) = &s.def { bloc(x.len()) } else { 0 };
        let options = s
            .options
            .iter()
            .map(|o| bloc(o.len()))
            .fold(bloc(s.options.len() * size_of::<&str>()), usize::saturating_add);
        [
            bloc(s.title.len()),
            bloc(s.title.len()),
            bloc(s.group.len().max(SANS_GROUPE.len())),
            bloc(s.tooltip.len()),
            bloc(s.inline.len()),
            def,
            options,
        ]
        .into_iter()
        .fold(0, usize::saturating_add)
    };
    let listes = bloc(specs.len().saturating_mul(size_of::<Param>())).saturating_add(bloc(specs.len().saturating_mul(size_of::<&str>())));
    specs.iter().map(textes).fold(listes.saturating_add(PLACE), usize::saturating_add)
}

#[cfg(test)]
thread_local! {
    /// Test hooks: the keys started, and the longest room a key's writing asked for.
    static CLES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static PLUS_LONGUE: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// The key of a table: the text of all its inputs, or `None` as soon as it would pass `max` bytes (its writing stops
/// there, and never asks for more room than that).
fn cle_de(specs: &[InputSpec], max: usize) -> Option<String> {
    struct Borne {
        s: String,
        max: usize,
    }
    impl std::fmt::Write for Borne {
        fn write_str(&mut self, x: &str) -> std::fmt::Result {
            let n = self.s.len().saturating_add(x.len());
            if n > self.max {
                return Err(std::fmt::Error);
            }
            if n > self.s.capacity() {
                // (the room doubles as the text grows, but never past `max`)
                let place = self.s.capacity().saturating_mul(2).clamp(n, self.max);
                self.s.reserve_exact(place - self.s.len());
            }
            self.s.push_str(x);
            Ok(())
        }
    }
    #[cfg(test)]
    CLES.with(|c| c.set(c.get() + 1));
    let mut b = Borne { s: String::new(), max };
    let fait = std::fmt::write(&mut b, format_args!("{specs:?}"));
    #[cfg(test)]
    PLUS_LONGUE.with(|c| c.set(c.get().max(b.s.capacity())));
    fait.ok().map(|_| b.s)
}

/// The table of `specs` in `t`: the one already made, or a new one when it fits under `max` bytes kept in all.
fn faire(t: &mut Tables, specs: &[InputSpec], max: usize) -> Result<Table, Plein> {
    // (a table that is too much without its key is refused before the key is written, and the key is written only as
    // far as the table can still fit with it: the key is a text of all the inputs, up to 6 times their size when they
    // hold line breaks, quotes or control characters, and writing it whole asked for 189 MB, only to say no)
    let sans_cle = poids_sans_cle(specs);
    if sans_cle > max {
        return Err(Plein::Script);
    }
    let Some(mut cle) = cle_de(specs, max - sans_cle) else {
        return Err(Plein::Script);
    };
    if let Some(x) = t.par_cle.get(&cle) {
        return Ok(*x);
    }
    // (the key is kept with the table: without the room its writing left behind, and counted as it is kept)
    cle.shrink_to_fit();
    let poids = sans_cle.saturating_add(bloc(cle.capacity()));
    let garde = t.garde.saturating_add(poids);
    if garde > max {
        return Err(if poids > max { Plein::Script } else { Plein::Session });
    }
    let mut groupes: Vec<&'static str> = vec![];
    let mut params = vec![];
    for s in specs {
        let g = if s.group.is_empty() { SANS_GROUPE } else { s.group.as_str() };
        let gi = match groupes.iter().position(|x| *x == g) {
            Some(k) => k,
            None => {
                groupes.push(fixe(g));
                groupes.len() - 1
            }
        };
        let kind = match sorte(s) {
            Sorte::Num(pas) => Kind::Num(s.min.unwrap_or(-1e9), s.max.unwrap_or(1e9), s.step.unwrap_or(pas)),
            Sorte::Price => Kind::Price,
            Sorte::Time => Kind::Time,
            Sorte::Session => Kind::Session,
            Sorte::Bool => Kind::Bool,
            Sorte::Color => Kind::Color,
            Sorte::Symbol => Kind::Symbol,
            Sorte::Source => Kind::Source,
            Sorte::Choice(o) => Kind::Choice(Box::leak(o.iter().map(|o| fixe(o)).collect::<Vec<_>>().into_boxed_slice())),
            Sorte::Text { multi } => Kind::Text { multi },
        };
        params.push(Param {
            key: fixe(&s.title),
            group: gi,
            // (an untitled input on a line gets a key « MA #1 · 2 » from the interpreter, but no label, like TradingView)
            // (and a title met twice gets the key « title@k », shown « title »)
            label: if !s.inline.is_empty() && s.title.rsplit_once(" · ").is_some_and(|(_, n)| n.parse::<usize>().is_ok()) {
                ""
            } else if let Some((t, _)) = s.title.rsplit_once('@').filter(|(_, k)| k.parse::<usize>().is_ok()) {
                fixe(t)
            } else {
                fixe(&s.title)
            },
            // (a list or a source: the default is an index)
            def: defaut(s),
            kind,
            tip: fixe(&s.tooltip),
            inline: fixe(&s.inline),
            active: "",
            display: !matches!(kind, Kind::Bool | Kind::Color),
            confirm: s.confirm,
            text: if let InVal::Text(x) = &s.def { fixe(x) } else { "" },
        });
    }
    let x: Table = (Box::leak(groupes.into_boxed_slice()), Box::leak(params.into_boxed_slice()));
    t.garde = garde;
    t.par_cle.insert(cle, x);
    Ok(x)
}

/// The settings table of a script that is part of the program (a built-in script, a rule of Argos AI): made once
/// and kept. For a script the user wrote, see `table_du_script`.
pub fn table(specs: &[InputSpec]) -> Table {
    let mut t = TABLES.lock().unwrap_or_else(|e| e.into_inner());
    faire(t.get_or_insert_with(Tables::default), specs, usize::MAX).unwrap_or((&[], &[]))
}

/// The settings table of a user script: made once per distinct set of inputs and kept (a script edited many times
/// makes one table per version opened in ⚙). Refused once the tables of this session keep `MAX_TABLES`: the window
/// says so instead of keeping more.
pub fn table_du_script(specs: &[InputSpec]) -> Result<Table, Plein> {
    let mut t = TABLES.lock().unwrap_or_else(|e| e.into_inner());
    faire(t.get_or_insert_with(Tables::default), specs, MAX_TABLES)
}

/// The window's values (by position) and texts for the chosen values `choisies` (missing: the default). Read from
/// the inputs alone: no table is made, nothing is kept.
pub fn valeurs(specs: &[InputSpec], choisies: &HashMap<String, InVal>) -> (Vec<f64>, HashMap<String, String>) {
    let mut textes = HashMap::new();
    let vals = specs
        .iter()
        .map(|s| match choisies.get(&s.title) {
            None => defaut(s),
            Some(InVal::Text(x)) => valeur_du_texte(&sorte(s), x).unwrap_or_else(|| {
                textes.insert(s.title.clone(), x.clone());
                0.0
            }),
            Some(InVal::Num(v)) => *v,
            Some(InVal::Bool(b)) => *b as u8 as f64,
            Some(InVal::Color(c)) => *c as f64,
        })
        .collect();
    (vals, textes)
}

/// The chosen values for the script (title → value) from the window's values and texts. Read from the inputs alone,
/// like `valeurs`.
pub fn choisies(specs: &[InputSpec], vals: &[f64], textes: &HashMap<String, String>) -> HashMap<String, InVal> {
    specs
        .iter()
        .zip(vals)
        .map(|(s, v)| {
            let x = match sorte(s) {
                Sorte::Bool => InVal::Bool(*v != 0.0),
                Sorte::Color => InVal::Color(*v as u32),
                Sorte::Choice(o) => InVal::Text(o.get(*v as usize).cloned().unwrap_or_default()),
                Sorte::Source => InVal::Text(NOMS_SOURCES.get(*v as usize).copied().unwrap_or("close").to_string()),
                Sorte::Text { .. } | Sorte::Symbol => InVal::Text(textes.get(&s.title).cloned().unwrap_or_else(|| match &s.def {
                    InVal::Text(x) => x.clone(),
                    _ => String::new(),
                })),
                Sorte::Session => InVal::Text(crate::inputs::session_txt(*v)),
                Sorte::Num(_) | Sorte::Price | Sorte::Time => InVal::Num(*v),
            };
            (s.title.clone(), x)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCRIPT: &str = r#"//@version=6
indicator("Mes réglages", overlay = true)
len = input.int(10, "Length", minval = 1, group = "Calcul")
k = input.float(1.5, "Factor", step = 0.5, group = "Calcul")
show = input.bool(true, "Show", group = "Style")
mode = input.string("Fast", "Mode", options = ["Fast", "Slow"])
src = input.source(close, "Source")
col = input.color(color.red, "Colour", group = "Style")
tf = input.timeframe("60", "Timeframe")
plot(show ? ta.sma(src, len) * k : na, color = col)
"#;

    fn bars() -> Vec<argos_core::Bar> {
        (0..60)
            .map(|i| argos_core::Bar {
                time: 1_700_000_000 + i * 60,
                open: 100.0 + i as f64,
                high: 102.0 + i as f64,
                low: 99.0 + i as f64,
                close: 101.0 + i as f64,
                volume: 1.0,
            })
            .collect()
    }

    /// ⚙ The inputs of a script become its settings table (groups, kinds, defaults); a changed value changes the
    /// result; the window's values go back to the same chosen values.
    #[test]
    fn reglages_d_un_script() {
        let b = bars();
        let out = super::super::run(SCRIPT, &b, 60, &HashMap::new()).unwrap();
        let (groupes, params) = table(&out.inputs);
        assert_eq!(groupes, ["Calcul", "Style", "Inputs"]);
        let p = |k: &str| params.iter().find(|p| p.key == k).unwrap();
        assert_eq!((p("Length").def, p("Length").group), (10.0, 0));
        assert!(matches!(p("Factor").kind, Kind::Num(_, _, s) if s == 0.5));
        assert!(matches!(p("Show").kind, Kind::Bool) && p("Show").def == 1.0);
        assert!(matches!(p("Mode").kind, Kind::Choice(o) if o == ["Fast", "Slow"]));
        assert!(matches!(p("Source").kind, Kind::Source) && p("Source").def == 0.0);
        assert!(matches!(p("Colour").kind, Kind::Color));
        assert!(matches!(p("Timeframe").kind, Kind::Text { .. }) && p("Timeframe").text == "60");
        assert!(std::ptr::eq(table(&out.inputs).1, params), "one table per set of inputs");
        // defaults: the window shows the script's defaults
        let (vals, textes) = valeurs(&out.inputs, &HashMap::new());
        assert_eq!(vals, params.iter().map(|p| p.def).collect::<Vec<_>>());
        // a changed value: the result changes, and goes back and forth unchanged
        let mut v2 = vals.clone();
        v2[0] = 5.0; // Length
        v2[4] = 2.0; // Source: high
        let c = choisies(&out.inputs, &v2, &textes);
        assert_eq!(c["Source"], InVal::Text("high".into()));
        assert_eq!(valeurs(&out.inputs, &c).0, v2);
        let avant = &out.plots[0].values;
        let apres = super::super::run(SCRIPT, &b, 60, &c).unwrap();
        assert_ne!(apres.plots[0].values.last(), avant.last(), "the chosen values are used");
    }

    /// One input whose help text is `k` followed by 10 000 characters: a list of inputs seen nowhere else.
    fn version(k: usize) -> Vec<InputSpec> {
        vec![InputSpec {
            title: "Longueur".into(),
            kind: "int",
            def: InVal::Num(14.0),
            min: None,
            max: None,
            step: None,
            options: vec![],
            tooltip: format!("{k}{}", "x".repeat(10_000)),
            group: String::new(),
            inline: String::new(),
            confirm: false,
        }]
    }

    /// A table lives as long as the program, so the tables of the user scripts have a ceiling (B-071): past it no
    /// new one is made, and the ones already made stay. Before, each distinct list of inputs opened in ⚙ left its
    /// table for good, with no end.
    #[test]
    fn les_tables_des_scripts_ont_un_plafond() {
        let mut t = Tables::default();
        let max = 250_000;
        let faites = (0..100).filter(|k| faire(&mut t, &version(*k), max).is_ok()).count();
        // (a table of these inputs keeps 20 KB: its help text, and its key, which holds the help text too)
        assert!((10..=12).contains(&faites), "{faites} tables made under {max} bytes");
        assert!(t.garde <= max && t.garde >= faites * 20_000, "{} bytes kept", t.garde);
        // (one made before the ceiling is still given, the same one; no new one is)
        let a = faire(&mut t, &version(0), max).expect("made before the ceiling");
        let b = faire(&mut t, &version(0), max).expect("still there");
        assert!(std::ptr::eq(a.1, b.1));
        assert_eq!(
            faire(&mut t, &version(1000), max).map(|_| ()),
            Err(Plein::Session),
            "no new table past the ceiling"
        );
        // (a table that is more than the ceiling on its own is told apart: starting the app again would not help)
        assert_eq!(faire(&mut Tables::default(), &version(0), 15_000).map(|_| ()), Err(Plein::Script));
        assert!(faire(&mut Tables::default(), &version(0), 25_000).is_ok());
        // the ceiling of the app is thousands of versions of an ordinary script
        let inputs = super::super::run(SCRIPT, &bars(), 60, &HashMap::new()).unwrap().inputs;
        let mut t = Tables::default();
        faire(&mut t, &inputs, usize::MAX).expect("no ceiling");
        let une = t.garde;
        assert!(une > 0 && MAX_TABLES / une > 2000, "{une} bytes for the table of an ordinary script");
    }

    /// What a block of `n` bytes really takes: one word of header, rounded to two words, at least four (glibc on a
    /// 64-bit machine; dlmalloc in the web build). Written here on its own, to check `bloc` against it.
    fn pris(n: usize) -> usize {
        let mot = std::mem::size_of::<usize>();
        if n == 0 {
            0
        } else {
            (n + mot).next_multiple_of(2 * mot).max(4 * mot)
        }
    }

    /// What was asked for and never given back for the one table of `t`, read from the table itself, each block
    /// counted by `bloc`: every text it points to, its two lists, its key as it is kept; and `place`.
    fn garde_pour_de_bon(t: &Tables, bloc: &dyn Fn(usize) -> usize, place: usize) -> usize {
        use std::mem::size_of_val;
        assert_eq!(t.par_cle.len(), 1);
        let (cle, (groupes, params)) = t.par_cle.iter().next().unwrap();
        assert_eq!(cle.capacity(), cle.len(), "the key is kept without the room its writing left behind");
        let mut garde = bloc(cle.capacity()) + place + bloc(size_of_val(*groupes)) + bloc(size_of_val(*params));
        garde += groupes.iter().map(|g| bloc(g.len())).sum::<usize>();
        for p in params.iter() {
            garde += [p.key, p.label, p.tip, p.inline, p.text].iter().map(|x| bloc(x.len())).sum::<usize>();
            if let Kind::Choice(o) = p.kind {
                garde += bloc(size_of_val(o)) + o.iter().map(|x| bloc(x.len())).sum::<usize>();
            }
        }
        garde
    }

    /// B-071: the ceiling counts what a table says it keeps, so a table says at least what it keeps. It counted its
    /// key by its length (kept: the room the writing took, up to twice as much), no block, no group « Inputs » and
    /// no place among the tables: under the ceiling of 32 MB, 1.14 to 1.5 times as much was kept for good; then a
    /// block of a short text was counted 16 bytes more than the text, where it takes 32 on a 64-bit machine (1.39
    /// times as much kept for 2 000 options of one character).
    #[test]
    fn une_table_dit_au_moins_ce_qu_elle_garde() {
        // a block is never counted less than it takes, whatever its size
        for n in (0..=4096).chain([65_535, 65_536, 1 << 20, 10_000_001]) {
            assert!(bloc(n) >= pris(n), "a block of {n} bytes: {} counted, {} taken", bloc(n), pris(n));
        }
        assert_eq!((bloc(0), bloc(usize::MAX)), (0, usize::MAX));
        // the places of the table of tables, measured: never more than PLACE for each table
        let mut toutes: HashMap<String, Table> = HashMap::new();
        for n in 1..=3000 {
            toutes.insert(format!("{n}"), (&[], &[]));
            let c = toutes.capacity();
            let places = if c < 8 { c + 1 } else { (c * 8 / 7).next_power_of_two() };
            let octets = pris(places * (std::mem::size_of::<(String, Table)>() + 1) + 16);
            assert!(octets <= n * PLACE, "{n} tables: a block of {octets} bytes of places, {} counted", n * PLACE);
        }
        let un = |titre: &str, def: InVal, kind: &'static str, tooltip: String, options: Vec<String>| InputSpec {
            title: titre.into(),
            kind,
            def,
            min: None,
            max: None,
            step: None,
            options,
            tooltip,
            group: String::new(),
            inline: String::new(),
            confirm: false,
        };
        let lettres = |n: usize| (0..n).map(|k| char::from(b'a' + (k % 26) as u8).to_string()).collect::<Vec<_>>();
        let exacts = [
            (
                "a help text of 10 000 characters",
                vec![un("Longueur", InVal::Num(14.0), "int", "x".repeat(10_000), vec![])],
            ),
            (
                "2 000 options of one character",
                vec![un("Mode", InVal::Text("a".into()), "string", String::new(), lettres(2_000))],
            ),
            ("a text", vec![un("Nom", InVal::Text("BTC".into()), "string", String::new(), vec![])]),
            (
                "an input on a line",
                vec![InputSpec {
                    inline: "Ligne 1".into(),
                    ..un("Longueur", InVal::Num(14.0), "int", String::new(), vec![])
                }],
            ),
        ];
        for (nom, specs) in &exacts {
            let mut t = Tables::default();
            faire(&mut t, specs, usize::MAX).expect("no ceiling");
            // (every block counted once, and no fewer bytes than the allocator takes, with the first table's places)
            assert_eq!(t.garde, garde_pour_de_bon(&t, &bloc, PLACE), "{nom}: one input, each block counted once");
            let premiere = pris(4 * (std::mem::size_of::<(String, Table)>() + 1) + 16);
            let garde = garde_pour_de_bon(&t, &pris, premiere);
            assert!(t.garde >= garde, "{nom}: {} said, {garde} kept", t.garde);
        }
        let plusieurs = [
            (
                "500 inputs with a title of one character",
                (0..500).map(|k| un(&k.to_string(), InVal::Num(1.0), "int", String::new(), vec![])).collect(),
            ),
            ("an ordinary script", super::super::run(SCRIPT, &bars(), 60, &HashMap::new()).unwrap().inputs),
        ];
        for (nom, specs) in &plusieurs {
            let mut t = Tables::default();
            faire(&mut t, specs, usize::MAX).expect("no ceiling");
            let (dit, garde) = (t.garde, garde_pour_de_bon(&t, &bloc, PLACE));
            // (above only by the group, its text and its place in the list: counted once per input, kept once per group)
            let groupe = bloc(SANS_GROUPE.len()) + std::mem::size_of::<&str>();
            assert!(dit >= garde && dit <= garde + specs.len() * groupe, "{nom}: {dit} said, {garde} counted");
            let pris = garde_pour_de_bon(&t, &pris, PLACE);
            assert!(dit >= pris, "{nom}: {dit} said, {pris} kept");
        }
    }

    /// A table that is too much on its own is refused before its key is written, or as soon as its key would pass
    /// what is left: the key is a text of all the inputs, and writing it whole asked for 2.4 times their size outside
    /// any budget, only to say no; 6 times when the texts hold control characters (written « \u{7f} »).
    #[test]
    fn une_table_trop_grosse_est_refusee_sans_ecrire_sa_cle() {
        let cles = || CLES.with(std::cell::Cell::get);
        let longue = || PLUS_LONGUE.with(std::cell::Cell::take);
        let avant = cles();
        assert_eq!(faire(&mut Tables::default(), &version(0), 5_000).map(|_| ()), Err(Plein::Script));
        assert_eq!(cles(), avant, "refused on the weight of its texts: no key written");
        // with its key only it is too much: the key is started, and stopped where the table can no longer fit
        let sans_cle = poids_sans_cle(&version(0));
        longue();
        assert_eq!(faire(&mut Tables::default(), &version(0), 15_000).map(|_| ()), Err(Plein::Script));
        assert_eq!(cles(), avant + 1);
        let ecrit = longue();
        assert!(
            ecrit > 0 && ecrit <= 15_000 - sans_cle,
            "{ecrit} bytes asked for the key, {} left",
            15_000 - sans_cle
        );
        // the same with 2 000 control characters in the help text: its key would be 6 times as long
        let mut specs = version(0);
        specs[0].tooltip = "\u{7f}".repeat(2_000);
        let sans_cle = poids_sans_cle(&specs);
        assert!(format!("{specs:?}").len() > 12_000 && sans_cle < 3_000);
        assert_eq!(faire(&mut Tables::default(), &specs, 8_000).map(|_| ()), Err(Plein::Script));
        let ecrit = longue();
        assert!(
            ecrit > 0 && ecrit <= 8_000 - sans_cle,
            "{ecrit} bytes asked for the key, {} left",
            8_000 - sans_cle
        );
        // and one that fits has its whole key, the same as before
        let mut t = Tables::default();
        faire(&mut t, &specs, 20_000).expect("fits");
        assert_eq!(t.par_cle.keys().next(), Some(&format!("{specs:?}")));
    }

    /// The values of the window are read from the inputs alone: no table is made for them, nothing is kept.
    #[test]
    fn les_valeurs_ne_font_pas_de_table() {
        let specs = version(987_654);
        let (vals, textes) = valeurs(&specs, &HashMap::new());
        assert_eq!(vals, [14.0]);
        assert_eq!(choisies(&specs, &vals, &textes)["Longueur"], InVal::Num(14.0));
        let cle = format!("{specs:?}");
        let t = TABLES.lock().unwrap_or_else(|e| e.into_inner());
        assert!(!t.as_ref().is_some_and(|t| t.par_cle.contains_key(&cle)));
    }
}
