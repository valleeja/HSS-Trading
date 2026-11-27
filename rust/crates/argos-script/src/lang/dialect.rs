//! # 🌐 Les langues d'Argos Script
//! On colle un script dans n'importe laquelle de ces langues ; Argos la reconnaît et la traduit en **Pine v6**, que le
//! moteur fait rouler :
//! - **Pine v6** : tel quel ;
//! - **Pine v2 à v5** (dont les scripts LuxAlgo) : `study` → `indicator`, `ema(` → `ta.ema(`, `security` →
//!   `request.security`, `input(…, type=…)` → `input.int(…)`, `iff`, `transp=` → `color.new`, et en v2 / v3 les
//!   couleurs sans `color.` (`aqua` → `color.aqua`)… ;
//! - **ancien Argos Script (v1)** : `rsi(close, 14)` sans `ta.`, `signal()`, `shape()`, `fill(a, b, #c1, #c2)`,
//!   `barcolor(cond, #c)` — ligne par ligne, les numéros de ligne restent les mêmes ;
//! - **wrun** (ex-kScript) : `param()`, `output()`, `onStart()` / `onBar()`, classes de `ta`
//!   (`new Ema(14)` + `.update(x)`), `out_<nom>(valeur)`, `alert()`, `fill()`, `render.shape/bgcolor/barcolor`.
//!
//! Ce qui n'a pas d'équivalent devient un commentaire, avec un message pour sa ligne (`notes`).

/// Langue d'un script.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialect {
    Pine6,
    /// Pine v4 ou v5 (le numéro).
    PineOld(u8),
    ArgosV1,
    Wrun,
}

impl Dialect {
    /// Nom affiché dans le terminal de l'éditeur.
    pub fn name(self) -> String {
        match self {
            Dialect::Pine6 => "Pine v6".into(),
            Dialect::PineOld(v) => format!("Pine v{v}"),
            Dialect::ArgosV1 => "Argos Script v1".into(),
            Dialect::Wrun => "wrun".into(),
        }
    }
}

/// Script traduit en Pine v6.
#[derive(Debug, Clone, PartialEq)]
pub struct Converted {
    pub dialect: Dialect,
    pub pine: String,
    /// Ligne d'origine de chaque ligne traduite (1 = première ligne), pour que les erreurs pointent la bonne ligne.
    pub lines: Vec<u32>,
    /// (ligne d'origine, message) : ce qui a été changé de façon approximative ou laissé de côté.
    pub notes: Vec<(u32, String)>,
}

impl Converted {
    /// Ligne d'origine de la ligne `n` du texte traduit.
    pub fn origin(&self, n: u32) -> u32 {
        self.lines.get(n.saturating_sub(1) as usize).copied().unwrap_or(n)
    }
}

/// Reconnaît la langue d'un script.
pub fn detect(src: &str) -> Dialect {
    if let Some(v) = version(src) {
        return if v >= 6 { Dialect::Pine6 } else { Dialect::PineOld(v) };
    }
    let has = |s: &str| src.contains(s);
    if has("onBar(") || (has("output(") && (has("param(") || has("param.")) && has("bar.")) || has("bar.close()") {
        return Dialect::Wrun;
    }
    // sans //@version : ancien Argos Script si on voit ses fonctions sans « ta. » ou ses signal() / shape()
    let code: String = src.lines().map(|l| l.split("//").next().unwrap_or("")).collect::<Vec<_>>().join("\n");
    let v1_call = ["signal(", "shape("].iter().any(|f| starts_call(&code, f));
    let bare_ta = [
        "rsi(",
        "ema(",
        "sma(",
        "atr(",
        "stoch(",
        "crossover(",
        "crossunder(",
        "highest(",
        "lowest(",
        "vwap(",
        "obv(",
        "wma(",
        "rma(",
        "hma(",
        "stdev(",
    ]
    .iter()
    .any(|f| starts_call(&code, f));
    if v1_call || (bare_ta && !code.contains("ta.")) {
        return Dialect::ArgosV1;
    }
    Dialect::Pine6
}

/// Traduit `src` en Pine v6 (rien à faire pour du Pine v6).
pub fn to_pine(src: &str) -> Converted {
    let dialect = detect(src);
    let same = |pine: String| Converted {
        dialect,
        lines: (1..=pine.lines().count() as u32).collect(),
        pine,
        notes: vec![],
    };
    match dialect {
        Dialect::Pine6 => same(source_inputs(src)),
        Dialect::PineOld(v) => old_pine(src, v),
        Dialect::ArgosV1 => argos_v1(src),
        Dialect::Wrun => wrun(src),
    }
}

/// Pine v6: `input(close, …)` (the generic `input` with a price as its default) is a Source input, as in TradingView
/// (TradingView's SMA, WMA… declare their Source this way): written `input.source(close, …)` so it gets the Source
/// list and not a number. Lines and columns do not move (same line, the name only gets longer).
fn source_inputs(src: &str) -> String {
    const PRIX: [&str; 9] = ["open", "high", "low", "close", "hl2", "hlc3", "ohlc4", "hlcc4", "volume"];
    let mut out = String::with_capacity(src.len());
    let mut rest = src;
    while let Some(k) = rest.find("input(") {
        let before = rest[..k].chars().next_back();
        let apres = rest[k + 6..].trim_start();
        let defval = apres
            .strip_prefix("defval")
            .map(|x| x.trim_start().trim_start_matches('=').trim_start())
            .unwrap_or(apres);
        let prix = PRIX.iter().any(|p| {
            defval
                .strip_prefix(p)
                .is_some_and(|x| x.trim_start().starts_with(',') || x.trim_start().starts_with(')'))
        });
        out.push_str(&rest[..k]);
        if prix && !before.is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '.') {
            out.push_str("input.source(");
        } else {
            out.push_str("input(");
        }
        rest = &rest[k + 6..];
    }
    out.push_str(rest);
    out
}

/// `//@version=N` en tête.
fn version(src: &str) -> Option<u8> {
    src.lines()
        .find_map(|l| l.trim().strip_prefix("//@version=").and_then(|v| v.trim().parse().ok()))
}

/// Un appel à `f` (qui finit par « ( ») qui n'est pas la fin d'un autre nom (`ta.ema(`, `myema(`).
fn starts_call(code: &str, f: &str) -> bool {
    let mut from = 0;
    while let Some(k) = code[from..].find(f) {
        let at = from + k;
        let before = code[..at].chars().next_back();
        if !before.is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '.') {
            return true;
        }
        from = at + f.len();
    }
    false
}

// ---------------------------------------------------------------------------------------------------------------
// Outils de texte : arguments, parenthèses, appels
// ---------------------------------------------------------------------------------------------------------------

/// Index de la parenthèse / du crochet qui ferme celui à `open` (en respectant les textes entre guillemets).
fn close_of(s: &str, open: usize) -> Option<usize> {
    let b = s.as_bytes();
    let (mut depth, mut quote) = (0i32, None::<u8>);
    for (i, &c) in b.iter().enumerate().skip(open) {
        match quote {
            Some(q) => {
                if c == q && b[i.saturating_sub(1)] != b'\\' {
                    quote = None;
                }
            }
            None => match c {
                b'"' | b'\'' => quote = Some(c),
                b'(' | b'[' | b'{' => depth += 1,
                b')' | b']' | b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(i);
                    }
                }
                _ => {}
            },
        }
    }
    None
}

/// Coupe une liste d'arguments sur les virgules du premier niveau.
fn split_args(s: &str) -> Vec<String> {
    let (mut out, mut cur, mut depth, mut quote) = (vec![], String::new(), 0i32, None::<char>);
    for c in s.chars() {
        match quote {
            Some(q) => {
                cur.push(c);
                if c == q {
                    quote = None;
                }
                continue;
            }
            None if c == '"' || c == '\'' => quote = Some(c),
            None if "([{".contains(c) => depth += 1,
            None if ")]}".contains(c) => depth -= 1,
            None if c == ',' && depth == 0 => {
                out.push(cur.trim().to_string());
                cur.clear();
                continue;
            }
            None => {}
        }
        cur.push(c);
    }
    if !cur.trim().is_empty() {
        out.push(cur.trim().to_string());
    }
    out
}

/// Calls one inside the other that [`rewrite_calls`] goes into (B-074). Deeper, the text is copied as it is, and
/// the parser, which reads the converted script, refuses it with its own limit and the line: the conversion called
/// itself once for each call inside another, and a line of a few thousand nested calls (`abs(abs(abs(…)))` in a
/// Pine v4 script) ended the stack before the parser saw it. Above the 100 levels the parser takes.
const MAX_APPELS: usize = 128;

/// Réécrit chaque appel `nom(args)` (nom simple, ou pointé comme `Math.max`) : `f` rend le remplacement, ou `None`
/// pour garder l'appel (ses arguments sont réécrits quand même).
fn rewrite_calls(s: &str, f: &dyn Fn(&str, &[String]) -> Option<String>) -> String {
    reecrire_appels(s, f, 0)
}

/// [`rewrite_calls`] at `niveau` calls inside the line.
fn reecrire_appels(s: &str, f: &dyn Fn(&str, &[String]) -> Option<String>, niveau: usize) -> String {
    let b = s.as_bytes();
    let mut out = String::new();
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if c == b'"' || c == b'\'' {
            // texte entre guillemets : copié tel quel
            let end = s[i + 1..].find(c as char).map_or(s.len(), |k| i + 1 + k + 1);
            out.push_str(&s[i..end]);
            i = end;
            continue;
        }
        let starts_name = (c.is_ascii_alphabetic() || c == b'_') && (i == 0 || !(b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'_' || b[i - 1] == b'.'));
        if starts_name {
            let mut j = i;
            while j < b.len() && (b[j].is_ascii_alphanumeric() || b[j] == b'_' || b[j] == b'.') {
                j += 1;
            }
            let name = &s[i..j];
            let mut k = j;
            while k < b.len() && b[k] == b' ' {
                k += 1;
            }
            if k < b.len() && b[k] == b'(' && niveau < MAX_APPELS {
                if let Some(e) = close_of(s, k) {
                    let args: Vec<String> = split_args(&s[k + 1..e]).iter().map(|a| reecrire_appels(a, f, niveau + 1)).collect();
                    out.push_str(&f(name, &args).unwrap_or_else(|| format!("{name}({})", args.join(", "))));
                    i = e + 1;
                    continue;
                }
            }
            out.push_str(name);
            i = j;
            continue;
        }
        out.push(c as char);
        i += 1;
    }
    out
}

/// Remplace un mot entier (pas une partie d'un autre nom).
fn replace_word(s: &str, from: &str, to: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(k) = rest.find(from) {
        let before = rest[..k].chars().next_back();
        let after = rest[k + from.len()..].chars().next();
        let word = |c: Option<char>| c.is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '.');
        out.push_str(&rest[..k]);
        out.push_str(if word(before) || word(after) { from } else { to });
        rest = &rest[k + from.len()..];
    }
    out.push_str(rest);
    out
}

/// Couleur `#rrggbb` parmi les arguments.
fn hex_arg(args: &[String]) -> Option<String> {
    args.iter()
        .find(|a| a.starts_with('#') && a.len() >= 7 && a[1..7].chars().all(|c| c.is_ascii_hexdigit()))
        .cloned()
}

/// Premier texte entre guillemets.
fn quoted(args: &[String]) -> Option<String> {
    args.iter().find(|a| a.len() >= 2 && a.starts_with('"') && a.ends_with('"')).cloned()
}

/// `a && b`, `a || b`, `!a` → `and`, `or`, `not` (sans toucher à `!=`).
fn logic_words(s: &str) -> String {
    let s = s.replace("&&", " and ").replace("||", " or ").replace("===", "==").replace("!==", "!=");
    let mut out = String::new();
    let ch: Vec<char> = s.chars().collect();
    for (i, &c) in ch.iter().enumerate() {
        if c == '!' && ch.get(i + 1) != Some(&'=') {
            out.push_str("not ");
        } else {
            out.push(c);
        }
    }
    // espaces doublés laissés par les remplacements
    while out.contains("  ") {
        out = out.replace("  ", " ");
    }
    out
}

// ---------------------------------------------------------------------------------------------------------------
// Ancien Argos Script (v1)
// ---------------------------------------------------------------------------------------------------------------

/// Fonctions de l'ancien Argos Script → Pine v6.
fn v1_call(name: &str, a: &[String]) -> Option<String> {
    let arg = |k: usize| a.get(k).cloned().unwrap_or_default();
    Some(match name {
        "sma" | "ema" | "rma" | "wma" | "hma" | "rsi" | "stdev" | "highest" | "lowest" | "change" | "roc" | "crossover" | "crossunder" => {
            format!("ta.{name}({})", a.join(", "))
        }
        "atr" => format!("ta.atr({})", arg(0)),
        "stoch" => format!("ta.stoch(close, high, low, {})", arg(0)),
        "vwap" => "ta.vwap(hlc3)".into(),
        "obv" => "ta.obv".into(),
        "macd" => format!("(ta.ema({0}, {1}) - ta.ema({0}, {2}))", arg(0), arg(1), arg(2)),
        "bb_upper" => format!("(ta.sma({0}, {1}) + {2} * ta.stdev({0}, {1}))", arg(0), arg(1), arg(2)),
        "bb_lower" => format!("(ta.sma({0}, {1}) - {2} * ta.stdev({0}, {1}))", arg(0), arg(1), arg(2)),
        "prev" => format!("({})[{}]", arg(0), a.get(1).cloned().unwrap_or_else(|| "1".into())),
        "abs" | "max" | "min" | "sqrt" | "log" => format!("math.{name}({})", a.join(", ")),
        _ => return None,
    })
}

fn v1_expr(e: &str) -> String {
    logic_words(&rewrite_calls(e, &v1_call))
}

fn argos_v1(src: &str) -> Converted {
    let mut pine = vec![];
    let mut notes = vec![];
    let mut tail: Vec<(String, u32)> = vec![];
    for (i, raw) in src.lines().enumerate() {
        let ln = i as u32 + 1;
        let (code, comment) = match raw.find("//") {
            Some(k) => (&raw[..k], &raw[k..]),
            None => (raw, ""),
        };
        let line = code.trim();
        let call = |f: &str| {
            line.strip_prefix(f)
                .and_then(|r| r.trim_start().strip_prefix('('))
                .and_then(|r| r.trim_end().strip_suffix(')'))
                .map(split_args)
        };
        let out = if line.is_empty() {
            comment.to_string()
        } else if let Some(a) = call("indicator") {
            let keep: Vec<String> = a.into_iter().filter(|x| !x.replace(' ', "").starts_with("alerts=")).collect();
            format!("indicator({})", keep.join(", "))
        } else if let Some(a) = call("plot") {
            let expr = v1_expr(a.first().map_or("na", |s| s.as_str()));
            let title = quoted(&a[1.min(a.len())..]).unwrap_or_else(|| "\"\"".into());
            let color = hex_arg(&a).unwrap_or_else(|| "#2962ff".into());
            let width = a
                .iter()
                .skip(1)
                .find(|x| matches!(x.as_str(), "1" | "2" | "3" | "4"))
                .cloned()
                .unwrap_or_else(|| "2".into());
            let style = if a.iter().any(|x| x == "hist") { ", style=plot.style_histogram" } else { "" };
            format!("plot({expr}, {title}, color={color}, linewidth={width}{style})")
        } else if let Some(a) = call("signal") {
            let cond = v1_expr(a.first().map_or("false", |s| s.as_str()));
            let sell = a.get(1).is_some_and(|s| s == "sell");
            let text = a.get(2).cloned().unwrap_or_else(|| "\"\"".into());
            let (shape, loc, color) = if sell {
                ("shape.triangledown", "location.abovebar", "#ef4444")
            } else {
                ("shape.triangleup", "location.belowbar", "#22c55e")
            };
            format!("plotshape({cond}, {text}, {shape}, {loc}, {color}, text={text}, size=size.small)")
        } else if let Some(a) = call("shape") {
            let cond = v1_expr(a.first().map_or("false", |s| s.as_str()));
            let kind = a.get(1).map_or("triup".to_string(), |s| s.to_lowercase());
            let (shape, up) = match kind.as_str() {
                "tridown" => ("shape.triangledown", false),
                "circle" | "circleup" => ("shape.circle", true),
                "circledown" => ("shape.circle", false),
                "diamond" | "diamondup" => ("shape.diamond", true),
                "diamonddown" => ("shape.diamond", false),
                "labelup" => ("shape.labelup", true),
                "labeldown" => ("shape.labeldown", false),
                _ => ("shape.triangleup", true),
            };
            let below = match a.iter().find(|x| *x == "above" || *x == "below") {
                Some(p) => p == "below",
                None => up,
            };
            let color = hex_arg(&a).unwrap_or_else(|| (if up { "#22c55e" } else { "#ef4444" }).into());
            let text = quoted(&a[1.min(a.len())..]).unwrap_or_else(|| "\"\"".into());
            format!(
                "plotshape({cond}, {text}, {shape}, {}, {color}, text={text})",
                if below { "location.belowbar" } else { "location.abovebar" }
            )
        } else if let Some(a) = call("fill") {
            // fill(a, b, #c1, #c2) : deux tracés cachés + le remplissage, ajoutés en bas du script
            let k = tail.len() / 3 + 1;
            let (x, y) = (v1_expr(a.first().map_or("na", |s| s.as_str())), v1_expr(a.get(1).map_or("na", |s| s.as_str())));
            let cs: Vec<String> = a.iter().filter(|s| s.starts_with('#')).cloned().collect();
            let (c1, c2) = (
                cs.first().cloned().unwrap_or_else(|| "#2962ff".into()),
                cs.get(1).or(cs.first()).cloned().unwrap_or_else(|| "#2962ff".into()),
            );
            tail.push((format!("__fa{k} = plot({x}, display=display.none)"), ln));
            tail.push((format!("__fb{k} = plot({y}, display=display.none)"), ln));
            tail.push((
                format!("fill(__fa{k}, __fb{k}, color=({x}) >= ({y}) ? color.new({c1}, 82) : color.new({c2}, 82))"),
                ln,
            ));
            "// fill() : en bas du script".to_string()
        } else if let Some(a) = call("barcolor") {
            let cond = v1_expr(a.first().map_or("false", |s| s.as_str()));
            format!("barcolor({cond} ? {} : na)", hex_arg(&a).unwrap_or_else(|| "#f0b90b".into()))
        } else if let Some(a) = call("hline") {
            format!("hline({})", a.join(", "))
        } else if let Some((name, expr)) = line
            .split_once('=')
            .filter(|(n, e)| !n.contains(['<', '>', '!']) && !e.starts_with('=') && n.trim().chars().all(|c| c.is_alphanumeric() || c == '_'))
        {
            format!("{} = {}", name.trim(), v1_expr(expr.trim()))
        } else {
            notes.push((
                ln,
                format!("instruction inconnue en Argos Script v1 : « {} »", line.chars().take(40).collect::<String>()),
            ));
            line.to_string()
        };
        pine.push((out + if line.is_empty() { "" } else { comment }).trim_end().to_string());
    }
    let mut lines: Vec<u32> = (1..=pine.len() as u32).collect();
    for (t, ln) in tail {
        pine.push(t);
        lines.push(ln);
    }
    Converted {
        dialect: Dialect::ArgosV1,
        pine: format!("{}\n", pine.join("\n")),
        lines,
        notes,
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Pine v4 / v5 (dont LuxAlgo)
// ---------------------------------------------------------------------------------------------------------------

/// Fonctions de Pine v4 qui sont dans `ta.` en v5 / v6.
const V4_TA: [&str; 46] = [
    "sma",
    "ema",
    "rma",
    "wma",
    "vwma",
    "hma",
    "alma",
    "swma",
    "rsi",
    "stoch",
    "atr",
    "cci",
    "mfi",
    "mom",
    "roc",
    "change",
    "highest",
    "lowest",
    "highestbars",
    "lowestbars",
    "stdev",
    "variance",
    "dev",
    "cross",
    "crossover",
    "crossunder",
    "pivothigh",
    "pivotlow",
    "valuewhen",
    "barssince",
    "cum",
    "falling",
    "rising",
    "linreg",
    "sar",
    "supertrend",
    "macd",
    "bb",
    "bbw",
    "kc",
    "kcw",
    "dmi",
    "wpr",
    "correlation",
    "percentrank",
    "median",
];
/// Fonctions de Pine v4 qui sont dans `math.`.
const V4_MATH: [&str; 13] = [
    "abs", "max", "min", "pow", "sqrt", "log", "log10", "exp", "round", "floor", "ceil", "sign", "avg",
];

fn v4_call(name: &str, a: &[String], notes: &std::cell::RefCell<Vec<String>>) -> Option<String> {
    let arg = |k: usize| a.get(k).cloned().unwrap_or_default();
    Some(match name {
        n if V4_TA.contains(&n) => format!("ta.{n}({})", a.join(", ")),
        n if V4_MATH.contains(&n) => format!("math.{n}({})", a.join(", ")),
        "sum" => format!("math.sum({})", a.join(", ")),
        "study" => format!("indicator({})", a.join(", ")),
        "security" => format!("request.security({})", a.join(", ")),
        "tostring" => format!("str.tostring({})", a.join(", ")),
        "tonumber" => format!("str.tonumber({})", a.join(", ")),
        "iff" => format!("({} ? {} : {})", arg(0), arg(1), arg(2)),
        "vwap" if !a.is_empty() => format!("ta.vwap({})", a.join(", ")),
        "input" => {
            // input(14, title="…", type=input.integer) → input.int(14, title="…")
            let kind = a.iter().find_map(|x| x.replace(' ', "").strip_prefix("type=").map(str::to_string));
            let rest: Vec<String> = a.iter().filter(|x| !x.replace(' ', "").starts_with("type=")).cloned().collect();
            let def = arg(0);
            let k = match kind.as_deref() {
                Some("input.integer") | Some("integer") => "int",
                Some("input.float") | Some("float") => "float",
                Some("input.bool") | Some("bool") => "bool",
                Some("input.source") | Some("source") => "source",
                Some("input.resolution") | Some("resolution") => "timeframe",
                Some("input.session") | Some("session") => "session",
                Some("input.symbol") | Some("symbol") => "symbol",
                Some("input.string") | Some("string") => "string",
                Some("input.color") | Some("color") => "color",
                _ if def == "true" || def == "false" => "bool",
                _ if ["open", "high", "low", "close", "hl2", "hlc3", "ohlc4", "volume"].contains(&def.as_str()) => "source",
                _ if def.starts_with('"') => "string",
                _ if def.starts_with('#') || def.starts_with("color.") => "color",
                _ if def.contains('.') => "float",
                _ => "int",
            };
            format!("input.{k}({})", rest.join(", "))
        }
        "plot" | "plotshape" | "plotchar" | "bgcolor" | "barcolor" | "fill" | "hline" | "plotarrow"
            if a.iter().any(|x| x.replace(' ', "").starts_with("transp=")) =>
        {
            let transp = a
                .iter()
                .find_map(|x| x.replace(' ', "").strip_prefix("transp=").map(str::to_string))
                .unwrap_or_default();
            let mut keep: Vec<String> = a.iter().filter(|x| !x.replace(' ', "").starts_with("transp=")).cloned().collect();
            // 🎨 the transparency goes into the colour, as v6 wants it: `color=` if it is named, else its place
            let named = keep.iter().position(|x| x.replace(' ', "").starts_with("color="));
            let place = match name {
                "bgcolor" | "barcolor" => Some(0),
                "plot" | "hline" | "fill" => Some(2),
                "plotshape" | "plotchar" => Some(4),
                _ => None,
            };
            let named_ok = |x: &String| !x.contains('=') || x.replace(' ', "").starts_with("color=");
            match (named, place) {
                (Some(k), _) => {
                    let c = keep[k].split_once('=').map(|(_, c)| c.trim().to_string()).unwrap_or_default();
                    keep[k] = format!("color=color.new({c}, {transp})");
                    notes.borrow_mut().push(format!("{name}() : « transp={transp} » mis dans color.new (Pine v6)"));
                }
                (None, Some(k)) if keep.get(k).is_some_and(named_ok) && keep[..k].iter().all(|x| !x.contains('=')) => {
                    keep[k] = format!("color.new({}, {transp})", keep[k].trim());
                    notes.borrow_mut().push(format!("{name}() : « transp={transp} » mis dans color.new (Pine v6)"));
                }
                _ => notes
                    .borrow_mut()
                    .push(format!("{name}() : « transp » retiré (en v6, la transparence va dans color.new)")),
            }
            format!("{name}({})", keep.join(", "))
        }
        _ => return None,
    })
}

/// Pine v1 to v3 wrote the styles bare: `style=circles` (plot) and `linestyle=dashed` (hline); v4 put them in
/// `plot.style_*` / `hline.style_*`. Only a bare word right after `style=` / `linestyle=` changes.
fn styles_nus(l: &str) -> String {
    const PLOT: [&str; 8] = ["line", "linebr", "stepline", "histogram", "cross", "area", "columns", "circles"];
    const HLINE: [&str; 3] = ["solid", "dotted", "dashed"];
    let mut out = String::with_capacity(l.len() + 16);
    let mut rest = l;
    while let Some(k) = rest.find("style") {
        let (avant, apres) = rest.split_at(k);
        out.push_str(avant);
        let mot_avant = avant.chars().last().filter(|c| c.is_alphanumeric() || *c == '_' || *c == '.');
        let hline = avant.ends_with("line") && !avant.ends_with(".line");
        let suite = &apres["style".len()..];
        let egal = suite.trim_start();
        let nom_ok =
            mot_avant.is_none() || (hline && avant.len() >= 4 && !avant[..avant.len() - 4].ends_with(|c: char| c.is_alphanumeric() || c == '_' || c == '.'));
        if nom_ok && egal.starts_with('=') && !egal.starts_with("==") {
            let val = egal[1..].trim_start();
            let fin = val.find(|c: char| !(c.is_alphanumeric() || c == '_')).unwrap_or(val.len());
            let (mot, reste) = val.split_at(fin);
            let libre = !reste.starts_with(['.', '(']);
            let (table, ns): (&[&str], &str) = if hline { (&HLINE, "hline") } else { (&PLOT, "plot") };
            if libre && table.contains(&mot) {
                out.push_str(&format!("style={ns}.style_{mot}"));
                rest = reste;
                continue;
            }
        }
        out.push_str("style");
        rest = suite;
    }
    out.push_str(rest);
    out
}

/// The colours of Pine v1 to v3, written without `color.` (v4 put them in `color.`).
const COULEURS_NUES: [&str; 17] = [
    "aqua", "black", "blue", "fuchsia", "gray", "green", "lime", "maroon", "navy", "olive", "orange", "purple", "red", "silver", "teal", "white", "yellow",
];

/// The names the script gives a value to (`name = …`, `name := …`), at the start of a line.
fn noms_assignes(src: &str) -> std::collections::HashSet<String> {
    src.lines()
        .filter_map(|l| {
            let t = l.trim_start();
            let end = t.find(|c: char| !(c.is_alphanumeric() || c == '_'))?;
            let rest = t[end..].trim_start();
            ((rest.starts_with('=') && !rest.starts_with("==")) || rest.starts_with(":=")).then(|| t[..end].to_string())
        })
        .collect()
}

/// Like [`replace_word`], only in the code of the line: the texts between quotes and the `//` comment stay as they are.
fn replace_word_in_code(s: &str, from: &str, to: &str) -> String {
    let mut out = String::new();
    let mut code = String::new();
    let mut quote = None::<char>;
    let mut escaped = false;
    for (i, c) in s.char_indices() {
        match quote {
            Some(q) => {
                out.push(c);
                if escaped {
                    escaped = false;
                } else if c == '\\' {
                    escaped = true;
                } else if c == q {
                    quote = None;
                }
            }
            None if c == '"' || c == '\'' => {
                out.push_str(&replace_word(&code, from, to));
                code.clear();
                out.push(c);
                quote = Some(c);
            }
            None if s[i..].starts_with("//") => {
                out.push_str(&replace_word(&code, from, to));
                out.push_str(&s[i..]);
                return out;
            }
            None => code.push(c),
        }
    }
    out.push_str(&replace_word(&code, from, to));
    out
}

fn old_pine(src: &str, v: u8) -> Converted {
    let mut notes = vec![];
    let mut pine = vec![];
    let propres = noms_assignes(src);
    for (i, raw) in src.lines().enumerate() {
        let ln = i as u32 + 1;
        let t = raw.trim();
        if t.starts_with("//@version=") {
            pine.push("//@version=6".to_string());
            continue;
        }
        if t.starts_with("import ") {
            notes.push((
                ln,
                "import : les bibliothèques (ex. LuxAlgo/…) ne sont pas encore dans Argos Script, la ligne est mise en commentaire".to_string(),
            ));
            pine.push(format!("// {raw}"));
            continue;
        }
        let line = if v <= 4 {
            let n = std::cell::RefCell::new(vec![]);
            let mut l = rewrite_calls(raw, &|name, a| v4_call(name, a, &n));
            // 🎨 Pine v1 to v3: the colours have no `color.` (`aqua`, `red`…), unless the script names a variable so
            if v <= 3 {
                for c in COULEURS_NUES.iter().filter(|c| !propres.contains(**c)) {
                    l = replace_word_in_code(&l, c, &format!("color.{c}"));
                }
                // ✏️ and the styles have no `plot.` / `hline.` (`style=circles`, `linestyle=dashed`)
                l = styles_nus(&l);
            }
            // variables de v4 devenues des noms pointés
            for (from, to) in [
                ("tr", "ta.tr"),
                ("obv", "ta.obv"),
                ("vwap", "ta.vwap"),
                ("accdist", "ta.accdist"),
                ("iii", "ta.iii"),
                ("nvi", "ta.nvi"),
                ("pvi", "ta.pvi"),
                ("wad", "ta.wad"),
                ("wvad", "ta.wvad"),
            ] {
                l = replace_word(&l, from, to);
            }
            for m in n.into_inner() {
                notes.push((ln, m));
            }
            l
        } else {
            raw.to_string()
        };
        pine.push(line);
    }
    let lines = (1..=pine.len() as u32).collect();
    Converted {
        dialect: Dialect::PineOld(v),
        pine: format!("{}\n", pine.join("\n")),
        lines,
        notes,
    }
}

// ---------------------------------------------------------------------------------------------------------------
// wrun
// ---------------------------------------------------------------------------------------------------------------

/// Une sortie déclarée par `output(…)`.
#[derive(Debug, Clone)]
struct Out {
    name: String,
    plot: String,
    lower: bool,
    color: Option<String>,
    width: Option<String>,
    used: bool,
}

/// Un objet de `ta` (`new Ema(14)`) : sa classe et ses arguments (déjà traduits).
#[derive(Debug, Clone)]
struct TaObj {
    class: String,
    args: Vec<String>,
}

/// Nom de variable comme wrun le génère (`fast.len` → `fast_len`).
fn wname(s: &str) -> String {
    let mut out = String::new();
    for c in s.to_lowercase().chars() {
        if c.is_ascii_alphanumeric() || c == '_' {
            out.push(c);
        } else if !out.ends_with('_') {
            out.push('_');
        }
    }
    out
}

/// Valeur d'une clé dans un objet `{ color: "#f00", width: 2 }`.
fn opt(obj: &str, key: &str) -> Option<String> {
    let inner = obj.trim().trim_start_matches('{').trim_end_matches('}');
    split_args(inner).into_iter().find_map(|kv| {
        let (k, v) = kv.split_once(':')?;
        (k.trim() == key).then(|| v.trim().to_string())
    })
}

/// Couleur wrun → Pine (`"#ff0000"` → `#ff0000`, `theme.up` → `color.green`).
fn wcolor(c: &str) -> String {
    let c = c.trim().trim_matches(|q| q == '"' || q == '\'');
    match c {
        "theme.up" => "color.green".into(),
        "theme.down" => "color.red".into(),
        "theme.text" | "theme.muted" => "color.gray".into(),
        "theme.accent" => "color.blue".into(),
        c if c.starts_with('#') => c.to_string(),
        c => format!("color.{c}"),
    }
}

/// Une expression wrun → Pine (barres, paramètres, maths, objets de `ta`, NaN, opérateurs).
fn wexpr(e: &str, objs: &std::collections::HashMap<String, TaObj>) -> String {
    let call = |name: &str, a: &[String]| -> Option<String> {
        let arg = |k: usize| a.get(k).cloned().unwrap_or_default();
        if let Some(field) = name.strip_prefix("bar.") {
            return Some(
                match field {
                    "open" | "high" | "low" | "close" | "volume" => field.to_string(),
                    "time" => "(time / 1000)".into(),
                    "isLast" => "barstate.islast".into(),
                    "count" => "(last_bar_index + 1)".into(),
                    _ => return None,
                }
                .to_string(),
            );
        }
        if let Some(p) = name
            .strip_prefix("p_")
            .or_else(|| name.strip_prefix("pb_"))
            .or_else(|| name.strip_prefix("pt_"))
        {
            if a.is_empty() {
                return Some(format!("p_{p}"));
            }
        }
        if let Some(f) = name.strip_prefix("Math.") {
            let f = match f {
                "abs" | "max" | "min" | "sqrt" | "pow" | "log" | "exp" | "round" | "floor" | "ceil" | "sign" => f,
                "log10" => "log10",
                _ => return None,
            };
            return Some(format!("math.{f}({})", a.join(", ")));
        }
        match name {
            "isNaN" => return Some(format!("na({})", arg(0))),
            "i32" | "i64" | "u32" => return Some(format!("int({})", arg(0))),
            "f64" | "f32" => return Some(format!("float({})", arg(0))),
            _ => {}
        }
        // objet.update(…) → la fonction de ta
        let (obj, method) = name.rsplit_once('.')?;
        let o = objs.get(obj)?;
        if method != "update" {
            return None;
        }
        let n = |k: usize| o.args.get(k).cloned().unwrap_or_else(|| "14".into());
        Some(match o.class.as_str() {
            "Ema" | "Sma" | "Rsi" | "Wma" | "Rma" | "Hma" | "Vwma" => format!("ta.{}({}, {})", o.class.to_lowercase(), arg(0), n(0)),
            "Highest" | "Lowest" => format!("ta.{}({}, {})", o.class.to_lowercase(), arg(0), n(0)),
            "Atr" => format!("ta.atr({})", n(0)),
            "Vwap" => "ta.vwap(hlc3)".into(),
            "Cross" | "PineCross" => format!("(ta.crossover({0}, {1}) ? 1 : ta.crossunder({0}, {1}) ? -1 : 0)", arg(0), arg(1)),
            "PivotHigh" => format!("ta.pivothigh({}, {}, {})", arg(0), n(0), o.args.get(1).cloned().unwrap_or_else(|| n(0))),
            "PivotLow" => format!("ta.pivotlow({}, {}, {})", arg(0), n(0), o.args.get(1).cloned().unwrap_or_else(|| n(0))),
            "Macd" => format!("{obj}_macd"),
            "Stoch" | "Stochastic" => format!("{obj}_k"),
            "Bb" => format!("{obj}_mid"),
            "History" => return None,
            _ => return None,
        })
    };
    let mut s = rewrite_calls(e, &call);
    // History : h.ago(k) → h_src[k]
    for (name, o) in objs {
        if o.class == "History" {
            s = s.replace(&format!("{name}.ago("), &format!("__hist_{name}["));
        }
    }
    s = fix_hist_brackets(&s);
    // champs des objets (m.signal → m_signal)
    for (name, o) in objs {
        for f in ["macd", "signal", "hist", "k", "d", "mid", "upper", "lower"] {
            s = s.replace(&format!("{name}.{f}"), &format!("{name}_{f}"));
        }
        let _ = o;
    }
    s = replace_word(&s, "NaN", "na");
    s = replace_word(&s, "Math.PI", "math.pi");
    logic_words(&s)
}

/// `__hist_h[3)` (laissé par le remplacement de `h.ago(3)`) → `__hist_h[3]`.
fn fix_hist_brackets(s: &str) -> String {
    let mut out = s.to_string();
    let mut from = 0;
    while let Some(k) = out[from..].find("__hist_") {
        let at = from + k;
        if let Some(open) = out[at..].find('[').map(|o| at + o) {
            let mut depth = 0;
            for (i, c) in out[open + 1..].char_indices() {
                match c {
                    '(' => depth += 1,
                    ')' if depth == 0 => {
                        out.replace_range(open + 1 + i..open + 2 + i, "]");
                        break;
                    }
                    ')' => depth -= 1,
                    _ => {}
                }
            }
        }
        from = at + 7;
    }
    out
}

fn wrun(src: &str) -> Converted {
    use std::collections::HashMap;
    let mut notes: Vec<(u32, String)> = vec![];
    // 1. code sans commentaires, une instruction / accolade par ligne (en gardant le numéro de la ligne d'origine)
    let mut toks: Vec<(String, u32)> = vec![];
    let mut in_block_comment = false;
    for (i, raw) in src.lines().enumerate() {
        let ln = i as u32 + 1;
        let mut line = raw.to_string();
        if in_block_comment {
            match line.find("*/") {
                Some(k) => {
                    line = line[k + 2..].to_string();
                    in_block_comment = false;
                }
                None => continue,
            }
        }
        if let Some(k) = line.find("/*") {
            match line[k..].find("*/") {
                Some(e) => line.replace_range(k..k + e + 2, ""),
                None => {
                    line.truncate(k);
                    in_block_comment = true;
                }
            }
        }
        if let Some(k) = find_outside_quotes(&line, "//") {
            line.truncate(k);
        }
        // coupe sur { } ; hors guillemets / parenthèses
        let (mut cur, mut depth, mut quote) = (String::new(), 0i32, None::<char>);
        for c in line.chars() {
            if let Some(q) = quote {
                cur.push(c);
                if c == q {
                    quote = None;
                }
                continue;
            }
            match c {
                '"' | '\'' | '`' => {
                    quote = Some(c);
                    cur.push(c);
                }
                '(' | '[' => {
                    depth += 1;
                    cur.push(c);
                }
                ')' | ']' => {
                    depth -= 1;
                    cur.push(c);
                }
                '{' if depth == 0 && !cur.trim_end().ends_with([':', ',', '=', '(']) && !cur.trim().is_empty()
                    || c == '{' && depth == 0 && cur.trim().is_empty() =>
                {
                    toks.push((format!("{} {{", cur.trim()), ln));
                    cur.clear();
                }
                '{' => {
                    depth += 1; // objet { … } dans une expression
                    cur.push(c);
                }
                '}' if depth > 0 => {
                    depth -= 1;
                    cur.push(c);
                }
                '}' => {
                    if !cur.trim().is_empty() {
                        toks.push((cur.trim().to_string(), ln));
                    }
                    toks.push(("}".into(), ln));
                    cur.clear();
                }
                ';' if depth == 0 => {
                    if !cur.trim().is_empty() {
                        toks.push((cur.trim().to_string(), ln));
                    }
                    cur.clear();
                }
                _ => cur.push(c),
            }
        }
        if !cur.trim().is_empty() {
            toks.push((cur.trim().to_string(), ln));
        }
    }
    // 2. déclarations du haut : param, output, objets de ta, état du module
    let mut inputs: Vec<(String, u32)> = vec![];
    let mut outs: Vec<Out> = vec![];
    let mut objs: HashMap<String, TaObj> = HashMap::new();
    let mut tail: Vec<(String, u32)> = vec![];
    let mut module_vars: Vec<String> = vec![];
    let mut body: Vec<(String, u32)> = vec![]; // onBar (et fonctions) traduits
    let mut start: Vec<(String, u32)> = vec![]; // onStart
                                                // (sorte de bloc, indentation) : « bar », « start », « fn », « if », « for », « while », « skip »
    let mut stack: Vec<&'static str> = vec![];
    let mut declared: Vec<String> = vec![];
    let new_obj = |rhs: &str| -> Option<TaObj> {
        let r = rhs.trim().strip_prefix("new ")?;
        let (class, rest) = r.split_once('(')?;
        let args = split_args(rest.trim_end().strip_suffix(')')?);
        Some(TaObj {
            class: class.trim().to_string(),
            args,
        })
    };
    for (t, ln) in toks {
        let t = t.trim().trim_start_matches("export ").to_string();
        let depth = stack.len();
        let in_body = stack.iter().any(|k| matches!(*k, "bar" | "fn"));
        let in_start = stack.contains(&"start");
        let indent = "    ".repeat(stack.iter().filter(|k| matches!(**k, "if" | "for" | "while" | "else" | "fn")).count());
        let emit = |v: &mut Vec<(String, u32)>, s: String| v.push((format!("{indent}{s}"), ln));
        if t == "}" {
            stack.pop();
            continue;
        }
        if t.starts_with("import ") {
            continue; // les imports de la SDK : rien à faire en Pine
        }
        // fonctions
        if let Some(rest) = t.strip_prefix("function ") {
            let name = rest.split('(').next().unwrap_or("").trim();
            match name {
                "onBar" => stack.push("bar"),
                "onStart" => stack.push("start"),
                "init" | "state" | "finalize" | "reset" => {
                    notes.push((ln, format!("{name}() : forme à 4 fonctions pas traduite, utilise onStart() / onBar()")));
                    stack.push("skip");
                }
                _ => {
                    // fonction d'aide → fonction Pine
                    let params = rest.split_once('(').and_then(|(_, r)| r.rsplit_once(')')).map_or("", |(p, _)| p);
                    let names: Vec<String> = split_args(params)
                        .iter()
                        .map(|p| p.split(':').next().unwrap_or("").trim().to_string())
                        .collect();
                    body.push((format!("{name}({}) =>", names.join(", ")), ln));
                    stack.push("fn");
                }
            }
            continue;
        }
        if stack.contains(&"skip") {
            if t.ends_with('{') {
                stack.push("skip");
            }
            continue;
        }
        // blocs de contrôle
        let open_block = |kw: &str| t.strip_prefix(kw).map(|r| r.trim().trim_end_matches('{').trim().to_string());
        if let Some(c) = open_block("} else if").or_else(|| open_block("else if")) {
            let c = c.trim().trim_start_matches('(').trim_end_matches(')').to_string();
            let ind = "    ".repeat(stack.iter().filter(|k| matches!(**k, "if" | "for" | "while" | "else" | "fn")).count());
            let target = if in_start { &mut start } else { &mut body };
            target.push((format!("{ind}else if {}", wexpr(&c, &objs)), ln));
            stack.push("else");
            continue;
        }
        if t == "else {" || t == "} else {" {
            let ind = "    ".repeat(stack.iter().filter(|k| matches!(**k, "if" | "for" | "while" | "else" | "fn")).count());
            let target = if in_start { &mut start } else { &mut body };
            target.push((format!("{ind}else"), ln));
            stack.push("else");
            continue;
        }
        if let Some(c) = open_block("if") {
            let c = c.trim().strip_prefix('(').and_then(|x| x.strip_suffix(')')).unwrap_or(&c).to_string();
            let target = if in_start { &mut start } else { &mut body };
            emit(target, format!("if {}", wexpr(&c, &objs)));
            stack.push("if");
            continue;
        }
        if let Some(c) = open_block("while") {
            let c = c.trim().strip_prefix('(').and_then(|x| x.strip_suffix(')')).unwrap_or(&c).to_string();
            emit(&mut body, format!("while {}", wexpr(&c, &objs)));
            stack.push("while");
            continue;
        }
        if let Some(c) = open_block("for") {
            // for (let i = 0; i < n; i++) → for i = 0 to n - 1
            let inner = c.trim().strip_prefix('(').and_then(|x| x.strip_suffix(')')).unwrap_or(&c).to_string();
            let parts: Vec<&str> = inner.split(';').map(str::trim).collect();
            let pine = (|| {
                let init = parts.first()?.trim_start_matches("let ").trim_start_matches("const ");
                let (var, from) = init.split_once('=')?;
                let var = var.split(':').next()?.trim();
                let cond = parts.get(1)?;
                let to = if let Some(n) = cond.strip_prefix(&format!("{var} <=")) {
                    wexpr(n.trim(), &objs)
                } else {
                    format!("{} - 1", wexpr(cond.strip_prefix(&format!("{var} <"))?.trim(), &objs))
                };
                Some(format!("for {var} = {} to {to}", wexpr(from.trim(), &objs)))
            })();
            match pine {
                Some(p) => emit(&mut body, p),
                None => {
                    notes.push((ln, "boucle for pas reconnue (forme attendue : for (let i = 0; i < n; i++))".into()));
                    emit(&mut body, "for __i = 0 to 0".into());
                }
            }
            stack.push("for");
            continue;
        }
        if t.ends_with('{') {
            notes.push((ln, format!("bloc pas traduit : « {} »", t.chars().take(40).collect::<String>())));
            stack.push("skip");
            continue;
        }
        // déclarations du haut du fichier
        if depth == 0 {
            if let Some(rest) = t.strip_prefix("param") {
                // param("len", 14, {…}) ou param.int("len", 14, {…})
                let (kind, args) = match rest.split_once('(') {
                    Some((k, a)) => (k.trim_start_matches('.').to_string(), split_args(a.trim_end().strip_suffix(')').unwrap_or(a))),
                    None => continue,
                };
                let name = args.first().map(|s| s.trim_matches('"').to_string()).unwrap_or_default();
                let def = args.get(1).cloned().unwrap_or_else(|| "0".into());
                let o = args.get(2).cloned().unwrap_or_default();
                let title = opt(&o, "description").or_else(|| opt(&o, "label")).unwrap_or_else(|| format!("\"{name}\""));
                let mut extra = String::new();
                if let Some(v) = opt(&o, "min") {
                    extra += &format!(", minval={v}");
                }
                if let Some(v) = opt(&o, "max") {
                    extra += &format!(", maxval={v}");
                }
                let pine_kind = match kind.as_str() {
                    "bool" => "bool",
                    "color" => "color",
                    "source" => "source",
                    "text" | "choice" => "string",
                    "timeframe" => "timeframe",
                    "number" | "price" => "float",
                    "int" => "int",
                    _ if def.contains('.') => "float",
                    _ => "int",
                };
                let def = match pine_kind {
                    "source" => def.trim_matches('"').replace("bar.", "").replace("()", ""),
                    "color" => wcolor(&def),
                    _ => def,
                };
                if kind == "choice" {
                    if let Some(opts) = opt(&o, "options") {
                        extra += &format!(", options={opts}");
                    }
                }
                inputs.push((format!("p_{} = input.{pine_kind}({def}, {title}{extra})", wname(&name)), ln));
                continue;
            }
            if let Some(rest) = t.strip_prefix("output(").or_else(|| t.split_once("= output(").map(|(_, r)| r)) {
                let a = split_args(rest.trim_end().strip_suffix(')').unwrap_or(rest));
                let name = a.first().map(|s| s.trim_matches('"').to_string()).unwrap_or_default();
                let o = a.iter().find(|x| x.starts_with('{')).cloned().unwrap_or_default();
                outs.push(Out {
                    name: wname(&name),
                    plot: a.get(1).cloned().unwrap_or_else(|| "line".into()),
                    lower: a.iter().any(|x| x == "lower"),
                    color: opt(&o, "color").map(|c| wcolor(&c)),
                    width: opt(&o, "width"),
                    used: false,
                });
                // `const h = output(…)` : la poignée garde le nom de la sortie
                if let Some((lhs, _)) = t.split_once("= output(") {
                    let h = lhs
                        .trim()
                        .trim_start_matches("const ")
                        .trim_start_matches("let ")
                        .split(':')
                        .next()
                        .unwrap_or("")
                        .trim()
                        .to_string();
                    if h != wname(&name) {
                        tail.push((format!("// poignée {h} = sortie {}", wname(&name)), ln));
                        objs.insert(
                            h,
                            TaObj {
                                class: format!("Out:{}", wname(&name)),
                                args: vec![],
                            },
                        );
                    }
                }
                continue;
            }
            if let Some(rest) = t.strip_prefix("alert(") {
                let a = split_args(rest.trim_end().strip_suffix(')').unwrap_or(rest));
                let title = a.first().cloned().unwrap_or_else(|| "\"Alerte\"".into());
                let o = a.get(1).cloned().unwrap_or_default();
                let when = opt(&o, "when").map(|h| out_of(&h, &objs)).unwrap_or_else(|| "false".into());
                let msg = opt(&o, "message").or_else(|| opt(&o, "text")).unwrap_or_else(|| title.clone());
                tail.push((format!("alertcondition(not na(__o_{when}) and __o_{when} != 0, {title}, {msg})"), ln));
                continue;
            }
            if let Some(rest) = t.strip_prefix("fill(").or_else(|| t.strip_prefix("range(")) {
                let a = split_args(rest.trim_end().strip_suffix(')').unwrap_or(rest));
                let (u, l) = (
                    out_of(a.first().map_or("", |s| s.as_str()), &objs),
                    out_of(a.get(1).map_or("", |s| s.as_str()), &objs),
                );
                let color = a
                    .get(2)
                    .and_then(|o| opt(o, "color"))
                    .map(|c| wcolor(&c))
                    .unwrap_or_else(|| "color.new(color.blue, 85)".into());
                tail.push((format!("fill(__p_{u}, __p_{l}, color={color})"), ln));
                continue;
            }
            if let Some(rest) = t.strip_prefix("render.") {
                let (kind, args) = rest.split_once('(').unwrap_or((rest, ""));
                let a = split_args(args.trim_end().strip_suffix(')').unwrap_or(args));
                let o = a.get(1).cloned().unwrap_or_default();
                match kind {
                    "shape" => {
                        let out = opt(&o, "output").map(|h| out_of(&h, &objs)).unwrap_or_default();
                        let shape = match opt(&o, "shape").unwrap_or_default().trim_matches('"') {
                            "triangle_down" => "shape.triangledown",
                            "circle" => "shape.circle",
                            "cross" => "shape.xcross",
                            "diamond" => "shape.diamond",
                            "arrow_up" => "shape.arrowup",
                            "arrow_down" => "shape.arrowdown",
                            "flag" => "shape.flag",
                            "square" => "shape.square",
                            _ => "shape.triangleup",
                        };
                        let color = opt(&o, "color").map(|c| wcolor(&c)).unwrap_or_else(|| "color.yellow".into());
                        tail.push((
                            format!(
                                "plotshape(not na(__o_{out}) and __o_{out} != 0, {}, {shape}, location.abovebar, {color})",
                                a.first().cloned().unwrap_or_default()
                            ),
                            ln,
                        ));
                    }
                    "bgcolor" | "barcolor" => {
                        let w = opt(&o, "where").map(|h| out_of(&h, &objs)).unwrap_or_default();
                        let color = opt(&o, "color").map(|c| wcolor(&c)).unwrap_or_else(|| "color.new(color.yellow, 85)".into());
                        tail.push((format!("{kind}(not na(__o_{w}) and __o_{w} != 0 ? {color} : na)"), ln));
                    }
                    _ => notes.push((ln, format!("render.{kind} : pas encore traduit"))),
                }
                continue;
            }
            if t.starts_with("draw.")
                || t.starts_with("pane(")
                || t.starts_with("box(")
                || t.starts_with("segment(")
                || t.starts_with("string(")
                || t.starts_with("input(")
                || t.starts_with("hover(")
            {
                notes.push((ln, format!("« {} » : pas encore traduit en Pine", t.split('(').next().unwrap_or(&t))));
                continue;
            }
            if ["page", "section", "divider", "note", "presets", "legend"]
                .iter()
                .any(|k| t.starts_with(&format!("{k}(")))
            {
                continue; // mise en page des réglages : rien à faire
            }
        }
        // let / const / var
        let decl = ["let ", "const ", "var "].iter().find_map(|k| t.strip_prefix(k));
        if let Some(d) = decl {
            let (lhs, rhs) = match d.split_once('=') {
                Some((l, r)) => (l.trim(), Some(r.trim())),
                None => (d.trim(), None),
            };
            let name = lhs.split(':').next().unwrap_or("").trim().to_string();
            // `let e: Ema;` : l'objet sera créé dans onStart
            if rhs.is_none() && lhs.split(':').nth(1).is_some_and(|ty| ty.trim().starts_with(|c: char| c.is_ascii_uppercase())) {
                continue;
            }
            if let Some(o) = rhs.and_then(new_obj) {
                let args = o.args.iter().map(|x| wexpr(x, &objs)).collect();
                objs.insert(name, TaObj { class: o.class, args });
                continue;
            }
            let value = rhs.map(|r| wexpr(r, &objs)).unwrap_or_else(|| "na".into());
            if depth == 0 {
                module_vars.push(name.clone());
                inputs.push((format!("var {name} = {value}"), ln));
            } else if in_start {
                emit(&mut start, format!("{name} := {value}"));
            } else {
                declared.push(name.clone());
                emit(&mut body, format!("{name} = {value}"));
            }
            continue;
        }
        // onStart : objets créés avec les paramètres
        if let Some((lhs, rhs)) = t.split_once('=').filter(|(l, r)| !l.ends_with(['!', '<', '>', '=']) && !r.starts_with('=')) {
            let name = lhs.trim().trim_end_matches(['+', '-', '*', '/']).trim().to_string();
            if let Some(o) = new_obj(rhs) {
                let args = o.args.iter().map(|x| wexpr(x, &objs)).collect();
                objs.insert(name, TaObj { class: o.class, args });
                continue;
            }
            let op = if lhs.trim().ends_with(['+', '-', '*', '/']) {
                format!("{}=", &lhs.trim()[lhs.trim().len() - 1..])
            } else {
                ":=".into()
            };
            let op = if op == ":=" && in_body && !module_vars.contains(&name) && !declared.contains(&name) {
                "=".to_string()
            } else {
                op
            };
            let target = if in_start { &mut start } else { &mut body };
            emit(target, format!("{name} {op} {}", wexpr(rhs.trim(), &objs)));
            continue;
        }
        if let Some(v) = t.strip_suffix("++").or_else(|| t.strip_suffix("--")) {
            emit(&mut body, format!("{} {}= 1", v.trim(), if t.ends_with("++") { "+" } else { "-" }));
            continue;
        }
        // out_x(valeur) → la valeur de la sortie (tracée en bas)
        if let Some(rest) = t.strip_prefix("out_") {
            if let Some((name, arg)) = rest.split_once('(') {
                let arg = arg.trim_end().strip_suffix(')').unwrap_or(arg);
                if let Some(o) = outs.iter_mut().find(|o| o.name == name) {
                    o.used = true;
                }
                emit(&mut body, format!("__o_{name} := {}", wexpr(arg, &objs)));
                continue;
            }
        }
        // objet.update(…) seul sur sa ligne : Macd / Stoch / Bb / History donnent plusieurs valeurs
        if let Some((obj, args)) = t.split_once(".update(") {
            if let Some(o) = objs.get(obj.trim()).cloned() {
                let a: Vec<String> = split_args(args.trim_end().strip_suffix(')').unwrap_or(args))
                    .iter()
                    .map(|x| wexpr(x, &objs))
                    .collect();
                let n = |k: usize, d: &str| o.args.get(k).cloned().unwrap_or_else(|| d.into());
                let x = a.first().cloned().unwrap_or_else(|| "close".into());
                let obj = obj.trim();
                let line = match o.class.as_str() {
                    "Macd" => format!(
                        "[{obj}_macd, {obj}_signal, {obj}_hist] = ta.macd({x}, {}, {}, {})",
                        n(0, "12"),
                        n(1, "26"),
                        n(2, "9")
                    ),
                    "Stoch" | "Stochastic" => format!(
                        "{obj}_k = ta.sma(ta.stoch(close, high, low, {}), {})\n{indent}{obj}_d = ta.sma({obj}_k, {})",
                        n(0, "14"),
                        n(2, "1"),
                        n(1, "3")
                    ),
                    "Bb" => format!("[{obj}_mid, {obj}_upper, {obj}_lower] = ta.bb({x}, {}, {})", n(0, "20"), n(1, "2")),
                    "History" => format!("__hist_{obj} = {x}"),
                    _ => format!("// {obj}.update() : la valeur est calculée là où elle sert"),
                };
                body.push((format!("{indent}{line}"), ln));
                continue;
            }
        }
        if let Some((obj, args)) = t.split_once(".push(") {
            if objs.get(obj.trim()).is_some_and(|o| o.class == "History") {
                let x = wexpr(args.trim_end().strip_suffix(')').unwrap_or(args), &objs);
                body.push((format!("{indent}__hist_{} = {x}", obj.trim()), ln));
                continue;
            }
        }
        if let Some(r) = t.strip_prefix("return") {
            // dernière valeur d'une fonction d'aide
            if stack.last() == Some(&"fn") || stack.contains(&"fn") {
                emit(&mut body, wexpr(r.trim(), &objs));
            }
            continue;
        }
        if t == "break" || t == "continue" {
            emit(&mut body, t.clone());
            continue;
        }
        notes.push((ln, format!("instruction pas traduite : « {} »", t.chars().take(40).collect::<String>())));
    }
    // 3. assemblage : en-tête, paramètres et état, onStart (1re bougie), onBar, puis les tracés et le reste
    let overlay = !outs.is_empty() && outs.iter().all(|o| !o.lower);
    let mut pine: Vec<(String, u32)> = vec![("//@version=6".into(), 1), (format!("indicator(\"Script wrun\", overlay={overlay})"), 1)];
    pine.extend(inputs);
    for o in &outs {
        pine.push((format!("float __o_{} = na", o.name), 1));
    }
    if !start.is_empty() {
        pine.push(("if barstate.isfirst".into(), start[0].1));
        pine.extend(start.into_iter().map(|(s, l)| (format!("    {s}"), l)));
    }
    pine.extend(body);
    for o in &outs {
        let style = match o.plot.trim_matches('"') {
            "histogram" | "bar" => ", style=plot.style_histogram",
            "area" => ", style=plot.style_area",
            "scatter" | "shape" => ", style=plot.style_circles",
            "none" => ", display=display.none",
            _ => "",
        };
        let color = o.color.clone().map(|c| format!(", color={c}")).unwrap_or_default();
        let width = o.width.clone().map(|w| format!(", linewidth={w}")).unwrap_or_default();
        pine.push((format!("__p_{0} = plot(__o_{0}, \"{0}\"{color}{width}{style})", o.name), 1));
    }
    pine.extend(tail);
    for o in outs.iter().filter(|o| !o.used) {
        notes.push((1, format!("sortie « {} » jamais remplie (out_{}(…))", o.name, o.name)));
    }
    let lines = pine.iter().map(|(_, l)| *l).collect();
    let text: Vec<String> = pine.into_iter().map(|(s, _)| s).collect();
    Converted {
        dialect: Dialect::Wrun,
        pine: format!("{}\n", text.join("\n")),
        lines,
        notes,
    }
}

/// Nom de la sortie derrière une poignée (`const golden = output(…)` → `golden`).
fn out_of(h: &str, objs: &std::collections::HashMap<String, TaObj>) -> String {
    let h = h.trim().trim_matches('"');
    match objs.get(h).and_then(|o| o.class.strip_prefix("Out:")) {
        Some(n) => n.to_string(),
        None => wname(h),
    }
}

/// Position de `pat` hors des textes entre guillemets.
fn find_outside_quotes(s: &str, pat: &str) -> Option<usize> {
    let mut quote = None::<char>;
    for (i, c) in s.char_indices() {
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) => {}
            None if c == '"' || c == '\'' || c == '`' => quote = Some(c),
            None if s[i..].starts_with(pat) => return Some(i),
            None => {}
        }
    }
    None
}
