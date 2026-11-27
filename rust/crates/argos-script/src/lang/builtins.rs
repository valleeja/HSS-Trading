//! 🧰 Les fonctions et constantes de Pine v6 dans Argos Script : variables de bougie, `ta`, `math`, `str`, `color`,
//! `array`, `map`, `input`, `plot` / `plotshape` / `hline` / `fill` / `bgcolor` / `barcolor`, `label` / `line` /
//! `box`, plus les lambdas d'Argos sur les tableaux. Les fonctions `ta` gardent leur mémoire à leur endroit du script.

use super::ast::Expr;
use super::interp::{Interp, Site, Smooth};
use super::lexer::Error;
use super::memoire;
use super::value::*;
use super::*;
use std::rc::Rc;

/// Couleurs de Pine (`color.red`…).
const COLORS: [(&str, u32); 17] = [
    ("aqua", 0x00BCD4),
    ("black", 0x363A45),
    ("blue", 0x2196F3),
    ("fuchsia", 0xE040FB),
    ("gray", 0x787B86),
    ("green", 0x4CAF50),
    ("lime", 0x00E676),
    ("maroon", 0x880E4F),
    ("navy", 0x311B92),
    ("olive", 0x808000),
    ("orange", 0xFF9800),
    ("purple", 0x9C27B0),
    ("red", 0xFF5252),
    ("silver", 0xB2B5BE),
    ("teal", 0x00897B),
    ("white", 0xFFFFFF),
    ("yellow", 0xFFEB3B),
];

/// Couleur des tracés sans couleur (bleu de TradingView).
const PLOT_BLUE: u32 = 0x2962FF;
/// No colour: transparency 100 (a drawing's colour given as `na`).
const NO_COLOR: u32 = 100 << 24;

const NAMESPACES: [&str; 26] = [
    "ta",
    "math",
    "str",
    "color",
    "array",
    "map",
    "input",
    "request",
    "label",
    "line",
    "box",
    "table",
    "strategy",
    "timeframe",
    "barstate",
    "syminfo",
    "log",
    "matrix",
    "chart",
    "polyline",
    "linefill",
    "runtime",
    "session",
    "ticker",
    "plot",
    "hline",
];

pub fn is_namespace(ns: &str) -> bool {
    NAMESPACES.contains(&ns)
}

/// Arguments d'un appel : positionnels (dans l'ordre) puis nommés.
struct A<'x> {
    v: Vec<(Option<&'x str>, Value)>,
}

impl A<'_> {
    fn get(&self, k: usize, name: &str) -> Option<&Value> {
        self.v
            .iter()
            .find(|(n, _)| *n == Some(name))
            .map(|x| &x.1)
            .or_else(|| self.v.iter().filter(|(n, _)| n.is_none()).nth(k).map(|x| &x.1))
    }
    fn val(&self, k: usize, name: &str) -> Value {
        self.get(k, name).cloned().unwrap_or(Value::Na)
    }
    fn num(&self, k: usize, name: &str) -> f64 {
        self.get(k, name).map_or(f64::NAN, Value::num)
    }
    fn num_or(&self, k: usize, name: &str, def: f64) -> f64 {
        self.get(k, name).filter(|v| !v.is_na()).map_or(def, Value::num)
    }
    fn len(&self, k: usize, name: &str) -> usize {
        self.num(k, name).max(0.0) as usize
    }
    fn text(&self, k: usize, name: &str) -> String {
        match self.get(k, name) {
            Some(Value::Na) | None => String::new(),
            Some(v) => v.text(),
        }
    }
    fn color(&self, k: usize, name: &str) -> Option<u32> {
        match self.get(k, name) {
            Some(Value::Color(c)) => Some(*c),
            _ => None,
        }
    }
    /// A colour argument: `def` when it is not given, no colour at all (fully transparent) when it is `na` — Pine's
    /// `color = na` / `color(na)` hides the background of a label, the border of a box…
    fn color_or(&self, k: usize, name: &str, def: u32) -> u32 {
        match self.get(k, name) {
            None => def,
            Some(Value::Color(c)) => *c,
            Some(_) => NO_COLOR,
        }
    }
    fn flag(&self, k: usize, name: &str, def: bool) -> bool {
        self.get(k, name).map_or(def, Value::truthy)
    }
    fn count(&self) -> usize {
        self.v.iter().filter(|(n, _)| n.is_none()).count()
    }
}

fn bar_val(b: &argos_core::Bar, name: &str, i: usize, tf: i64) -> Option<f64> {
    Some(match name {
        "open" => b.open,
        "high" => b.high,
        "low" => b.low,
        "close" => b.close,
        "volume" => b.volume,
        "time" => b.time as f64 * 1000.0,
        "time_close" => (b.time + tf) as f64 * 1000.0,
        "hl2" => (b.high + b.low) / 2.0,
        "hlc3" => (b.high + b.low + b.close) / 3.0,
        "ohlc4" => (b.open + b.high + b.low + b.close) / 4.0,
        "hlcc4" => (b.high + b.low + 2.0 * b.close) / 4.0,
        "bar_index" => i as f64,
        _ => return None,
    })
}

/// Prix d'une bougie passée : `close[3]`.
pub fn series_back(it: &Interp, name: &str, k: usize) -> Option<Value> {
    bar_val(&it.bars[0], name, 0, it.tf)?;
    Some(match it.i.checked_sub(k) {
        Some(j) => Value::from_f64(bar_val(&it.bars[j], name, j, it.tf).unwrap_or(f64::NAN)),
        None => Value::Na,
    })
}

fn date(it: &Interp) -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::from_timestamp(it.bars[it.i].time, 0).unwrap_or_default()
}

/// Variable sans espace de noms : prix de la bougie, `bar_index`, date…
pub fn variable(it: &mut Interp, name: &str) -> Option<Value> {
    use chrono::{Datelike, Timelike};
    if let Some(v) = bar_val(&it.bars[it.i], name, it.i, it.tf) {
        return Some(Value::Num(v));
    }
    let d = date(it);
    Some(match name {
        "last_bar_index" => Value::Num(it.bars.len() as f64 - 1.0),
        // ⏱ the opening time of the chart's last bar, in milliseconds (Pine's `last_bar_time`).
        "last_bar_time" => Value::Num(it.bars.last().map_or(0, |b| b.time) as f64 * 1000.0),
        "year" => Value::Num(d.year() as f64),
        "month" => Value::Num(d.month() as f64),
        "dayofmonth" => Value::Num(d.day() as f64),
        "dayofweek" => Value::Num(d.weekday().number_from_sunday() as f64),
        "hour" => Value::Num(d.hour() as f64),
        "minute" => Value::Num(d.minute() as f64),
        "second" => Value::Num(d.second() as f64),
        "weekofyear" => Value::Num(d.iso_week().week() as f64),
        // ⏱ « now »: the newest time the data reaches — the last bar's opening, or a later intrabar's (a daily chart with
        // 5-minute intrabars: their last one), so an age `timenow - time` of an intrabar event is never negative.
        "timenow" => {
            let t = it.bars.last().map_or(0, |b| b.time);
            let t = it.intrabars.values().filter_map(|v| v.last()).map(|b| b.time).fold(t, i64::max);
            Value::Num(t as f64 * 1000.0)
        }
        _ => return None,
    })
}

/// Unité de temps comme Pine (« 30 », « 240 », « D », « W », « M », « 15S »).
fn tf_text(tf: i64) -> String {
    match tf {
        t if t % 2_592_000 == 0 && t >= 2_592_000 => format!("{}M", t / 2_592_000).replace("1M", "M"),
        t if t % 604_800 == 0 => format!("{}W", t / 604_800).replace("1W", "W"),
        t if t % 86_400 == 0 => format!("{}D", t / 86_400).replace("1D", "D"),
        t if t % 60 == 0 => format!("{}", t / 60),
        t => format!("{t}S"),
    }
}

/// ⏱ Seconds of a Pine timeframe string (`""` = the chart's, « 60 » = minutes, « 30S », « D », « 2W », « 3M »…);
/// `None` when it is not one (step 4: `timeframe.in_seconds`).
pub(crate) fn tf_secs(s: &str, cur: i64) -> Option<i64> {
    let s = s.trim();
    if s.is_empty() {
        return Some(cur);
    }
    let (n, u) = s.split_at(s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len()));
    let n: i64 = if n.is_empty() { 1 } else { n.parse().ok()? };
    let k = match u {
        "" => 60,
        "S" => 1,
        "D" => 86_400,
        "W" => 604_800,
        "M" => 2_592_000,
        _ => return None,
    };
    // (B-051: a number of units past what the seconds hold is not a timeframe — the product overflowed)
    n.checked_mul(k).filter(|_| n > 0)
}

/// `ns.m` sans appel : constantes (`color.red`, `shape.circle`…) et variables (`barstate.islast`, `ta.tr`…).
pub fn member(it: &mut Interp, ns: &str, m: &str, e: &Expr) -> Result<Option<Value>, Error> {
    let s = |t: &str| Some(Value::str(t));
    let n = it.bars.len();
    Ok(match (ns, m) {
        ("color", c) => COLORS.iter().find(|(k, _)| *k == c).map(|(_, v)| Value::Color(*v)),
        ("barstate", "isfirst") => Some(Value::Bool(it.i == 0)),
        ("barstate", "islast") | ("barstate", "isrealtime") | ("barstate", "islastconfirmedhistory") => Some(Value::Bool(it.i + 1 == n)),
        ("barstate", "ishistory") => Some(Value::Bool(it.i + 1 < n)),
        // (false on the last bar only when the app says it is still forming)
        ("barstate", "isconfirmed") => Some(Value::Bool(!(it.last_open && it.i + 1 == n))),
        // 🧮 Argos: buys minus sells of the bar from the real trades, `na` without them
        ("argos", "real_delta") => Some(Value::from_f64(it.real_delta.get(it.i).copied().unwrap_or(f64::NAN))),
        ("barstate", "isnew") => Some(Value::Bool(true)),
        ("syminfo", "mintick") => Some(Value::Num(mintick(it.bars))),
        ("syminfo", "tickerid") => s(&tickerid(it)),
        ("syminfo", "ticker" | "root" | "description") => s(&ticker(it)),
        ("syminfo", "currency" | "basecurrency") => s("USD"),
        ("syminfo", "type") => s("crypto"),
        ("syminfo", "timezone") => s("Etc/UTC"),
        ("syminfo", "pointvalue") => Some(Value::Num(1.0)),
        ("timeframe", "period") => s(&tf_text(it.tf)),
        ("timeframe", "multiplier") => Some(Value::Num(match it.tf {
            t if t % 86_400 == 0 => (t / 86_400) as f64,
            t if t % 60 == 0 => (t / 60) as f64,
            t => t as f64,
        })),
        ("timeframe", "isintraday") => Some(Value::Bool(it.tf < 86_400)),
        ("timeframe", "isdaily") => Some(Value::Bool(it.tf >= 86_400 && it.tf < 604_800)),
        ("timeframe", "isweekly") => Some(Value::Bool(it.tf >= 604_800 && it.tf < 2_592_000)),
        ("timeframe", "ismonthly") => Some(Value::Bool(it.tf >= 2_592_000)),
        ("timeframe", "isminutes") => Some(Value::Bool(it.tf % 60 == 0 && it.tf < 86_400)),
        ("timeframe", "isseconds") => Some(Value::Bool(it.tf < 60)),
        ("math", "pi") => Some(Value::Num(std::f64::consts::PI)),
        ("math", "e") => Some(Value::Num(std::f64::consts::E)),
        ("math", "phi") => Some(Value::Num((1.0 + 5f64.sqrt()) / 2.0)),
        ("math", "rphi") => Some(Value::Num(0.618_033_988_749_895)),
        ("ta", "tr") => Some(Value::from_f64(true_range(it))),
        ("ta", "vwap") => Some(Value::from_f64(vwap(it, f64::NAN))),
        ("ta", "obv") => {
            let (c, p, v) = (it.bars[it.i].close, it.bar_back(1).map(|b| b.close), it.bars[it.i].volume);
            let id = expr_id(e);
            let site = it.site(id);
            site.p[0] += p.map_or(0.0, |p| {
                if c > p {
                    v
                } else if c < p {
                    -v
                } else {
                    0.0
                }
            });
            Some(Value::Num(site.p[0]))
        }
        ("ta", "accdist") => {
            let b = it.bars[it.i];
            let r = b.high - b.low;
            let mfv = if r > 0.0 {
                ((b.close - b.low) - (b.high - b.close)) / r * b.volume
            } else {
                0.0
            };
            let site = it.site(expr_id(e));
            site.p[0] += mfv;
            Some(Value::Num(site.p[0]))
        }
        ("chart", "bg_color") => Some(Value::Color(0x131722)),
        ("chart", "fg_color") => Some(Value::Color(0xD1D4DC)),
        ("dayofweek", d) => ["sunday", "monday", "tuesday", "wednesday", "thursday", "friday", "saturday"]
            .iter()
            .position(|x| *x == d)
            .map(|k| Value::Num(k as f64 + 1.0)),
        ("order", "ascending") => Some(Value::Num(0.0)),
        ("order", "descending") => Some(Value::Num(1.0)),
        ("strategy", "long") => s("long"),
        ("strategy", "short") => s("short"),
        ("strategy", c @ ("fixed" | "cash" | "percent_of_equity")) => s(c),
        ("strategy.commission" | "strategy.oca" | "strategy.direction", c) => s(c),
        // 🧪 what a strategy reads at the bar's close (a sub-run of request.security has no broker: na)
        ("strategy", v) => match &it.strat {
            Some(c) if v == "position_entry_name" => s(c.nom_entree()),
            Some(c) => c.lire(v).map(Value::from_f64),
            None if it.sous => Some(Value::Na),
            None => return it.err(e.line, format!("strategy.{v} : seulement dans un script strategy(...)")),
        },
        (
            "shape"
            | "location"
            | "size"
            | "plot"
            | "hline"
            | "line"
            | "extend"
            | "xloc"
            | "yloc"
            | "label"
            | "text"
            | "display"
            | "format"
            | "barmerge"
            | "position"
            | "font"
            | "currency"
            | "scale"
            | "adjustment"
            | "alert"
            | "dividends"
            | "earnings"
            | "splits"
            | "settlement_as_close"
            | "backadjustment",
            v,
        ) => s(v.trim_start_matches("style_")),
        _ => None,
    })
}

/// `syminfo.ticker`: the chart's symbol as its header shows it (« ARGOS » when the app has not said).
pub(crate) fn ticker(it: &Interp) -> String {
    if it.ticker.is_empty() {
        "ARGOS".into()
    } else {
        it.ticker.clone()
    }
}

/// `syminfo.tickerid`: « EXCHANGE:symbol » (« ARGOS » when the app has not said).
pub(crate) fn tickerid(it: &Interp) -> String {
    match (it.exchange.is_empty(), it.ticker.is_empty()) {
        (_, true) => "ARGOS".into(),
        (true, false) => it.ticker.clone(),
        (false, false) => format!("{}:{}", it.exchange.to_uppercase(), it.ticker),
    }
}

/// Identifiant de mémoire d'une expression sans `id` (variables comme `ta.obv`).
fn expr_id(e: &Expr) -> u64 {
    e as *const Expr as u64 | 1 << 43
}

/// `format.volume`: thousands as K, millions as M, billions as B, at most 3 decimals and no trailing zero (TradingView's
/// « 16.213K », « 4.39K »); under 1 000, at most 2 decimals. A number that rounds up to 1 000 of its unit takes the
/// unit above.
fn volume_text(x: f64) -> String {
    if !x.is_finite() {
        return "NaN".into();
    }
    let trim = |s: String| {
        if s.contains('.') {
            s.trim_end_matches('0').trim_end_matches('.').to_string()
        } else {
            s
        }
    };
    // the first unit whose rounded number stays under 1 000: 999 999.6 is « 1M », never « 1000K »
    for (div, u, dec) in [(1.0, "", 2), (1e3, "K", 3), (1e6, "M", 3), (1e9, "B", 3)] {
        let txt = format!("{:.dec$}", x / div);
        if txt.trim_start_matches('-').parse::<f64>().is_ok_and(|v| v < 1000.0) {
            return format!("{}{u}", trim(txt));
        }
    }
    format!("{}T", trim(format!("{:.3}", x / 1e12)))
}

/// Plus petit pas de prix vu dans les bougies (≈ `syminfo.mintick`).
fn mintick(bars: &[argos_core::Bar]) -> f64 {
    let dec = bars.iter().rev().take(50).map(|b| {
        let s = format!("{}", b.close);
        s.split('.').nth(1).map_or(0, |d| d.len().min(8))
    });
    10f64.powi(-(dec.max().unwrap_or(2) as i32))
}

fn true_range(it: &Interp) -> f64 {
    let b = &it.bars[it.i];
    match it.bar_back(1) {
        Some(p) => (b.high - b.low).max((b.high - p.close).abs()).max((b.low - p.close).abs()),
        None => b.high - b.low,
    }
}

/// VWAP de la séance (jour UTC) jusqu'à la bougie en cours ; `src` NaN = hlc3.
fn vwap(it: &Interp, src: f64) -> f64 {
    let day = it.bars[it.i].time.div_euclid(86_400);
    let (mut pv, mut v) = (0.0, 0.0);
    for j in (0..=it.i).rev() {
        let b = &it.bars[j];
        if b.time.div_euclid(86_400) != day {
            break;
        }
        let s = if j == it.i && src.is_finite() {
            src
        } else {
            (b.high + b.low + b.close) / 3.0
        };
        pv += s * b.volume;
        v += b.volume;
    }
    if v > 0.0 {
        pv / v
    } else {
        f64::NAN
    }
}

/// Garde la valeur dans la mémoire de l'endroit et rend les `len` dernières.
fn window(site: &mut Site, x: f64, len: usize) -> Option<&[f64]> {
    memoire::pousser(&mut site.a, x);
    let n = site.a.len();
    (len > 0 && n >= len).then(|| &site.a[n - len..])
}

fn tuple(v: Vec<f64>) -> Value {
    Value::tuple(v.into_iter().map(Value::from_f64).collect())
}

fn arr_of(v: &Value) -> Option<Rc<Tableau>> {
    match v {
        Value::Arr(a) => Some(a.clone()),
        _ => None,
    }
}

/// The decimals a number pattern asks (« #.## »: 2). At most what Rust's formatting takes: a pattern of 70 000 « # »
/// stopped the app (B-051).
fn decimales(motif: &str) -> usize {
    motif.split('.').nth(1).map_or(0, |d| d.len()).min(u16::MAX as usize)
}

/// `str.format("{0} et {1}", a, b)`; `None` when the text would pass `MAX_TEXT` characters.
fn format_text(fmt: &str, args: &[Value]) -> Option<String> {
    let mut out = fmt.to_string();
    for (k, v) in args.iter().enumerate() {
        let txt = match v {
            Value::Num(x) => {
                // {0,number,#.##} : on garde 2 décimales
                let pat = format!("{{{k},number,");
                if let Some(p) = out.find(&pat) {
                    if let Some(end) = out[p..].find('}') {
                        let spec = out[p + pat.len()..p + end].to_string();
                        let dec = decimales(&spec);
                        out.replace_range(p..p + end + 1, &format!("{x:.dec$}"));
                    }
                }
                v.text()
            }
            // (B-049: the text of a collection is counted while it is written)
            _ => v.texte_entier()?,
        };
        let cle = format!("{{{k}}}");
        let octets = out.len().saturating_add(out.matches(cle.as_str()).count().saturating_mul(txt.len()));
        out = texte_borne(octets, || out.replace(&cle, &txt))?;
    }
    texte_borne(out.len(), || out)
}

/// The text `faire` builds, when it holds `MAX_TEXT` characters at most. `octets` is the most bytes it can take, known
/// before it is built: past 4 bytes a character nothing is built, so a size the script chose is never asked from the
/// computer (`str.repeat("x", 1e12)` stopped the whole app).
fn texte_borne(octets: usize, faire: impl FnOnce() -> String) -> Option<String> {
    if octets > MAX_TEXT * 4 {
        return None;
    }
    let s = faire();
    (s.len() <= MAX_TEXT || s.chars().count() <= MAX_TEXT).then_some(s)
}

/// `texte_borne` as a script value, or the error of `f` (« str.repeat », « + »…) with its line.
pub(crate) fn valeur_texte(it: &Interp, line: u32, f: &str, octets: usize, faire: impl FnOnce() -> String) -> Result<Value, Error> {
    match texte_borne(octets, faire) {
        Some(s) => Ok(Value::str(&s)),
        None => it.err(line, trop_de_texte(f)),
    }
}

/// The whole text of `v` (`Value::texte_entier`: counted while it is written), or the error of `f` with its line.
pub(crate) fn texte_de(it: &Interp, line: u32, f: &str, v: &Value) -> Result<String, Error> {
    match v.texte_entier() {
        Some(s) => Ok(s),
        None => it.err(line, trop_de_texte(f)),
    }
}

/// The error of a text asked past its limit, named by the function that asked.
fn trop_de_texte(f: &str) -> String {
    format!("{f} : 100 000 caractères au plus dans un texte")
}

/// Appel d'une fonction de Pine (`name` : « plot », « ta.sma »…).
pub fn call(it: &mut Interp, name: &str, args: Vec<(Option<&str>, Value)>, id: u64, gen: Option<&str>, line: u32) -> Result<Value, Error> {
    // (B-062: every built-in passes here; when enough was made, what the script keeps is counted before the call)
    if memoire::appel(name) {
        it.compter_memoire(line)?;
    }
    // (B-076: a handle that another run made — a sub-run of `request.security` gave it back, alone or inside a tuple,
    // a collection or an object — names nothing of this run: every function, and every method, which comes here with
    // its value first, takes it for `na`. Only a run that read another timeframe can hold one: the others skip the look.)
    let mut args = args;
    if it.lit_ailleurs() {
        for (_, v) in &mut args {
            if matches!(v, Value::Handle(_, _, p) if *p != it.passage) {
                *v = Value::Na;
            }
        }
    }
    let a = A { v: args };
    let i = it.i;
    let n = it.bars.len();
    let nan = Value::Na;
    Ok(match name {
        // ── déclaration ──────────────────────────────────────────────────────────────────────────────────────────
        "indicator" | "strategy" | "study" => {
            if i == 0 {
                it.out.title = a.text(0, "title");
                it.out.short = a.text(1, "shorttitle");
                it.out.overlay = a.flag(2, "overlay", false);
                // (max_labels_count / max_lines_count / max_boxes_count, by name: 1..=500; 500 when not given)
                for (k, nom) in ["max_labels_count", "max_lines_count", "max_boxes_count", "max_polylines_count"]
                    .iter()
                    .enumerate()
                {
                    let v = a.num(usize::MAX, nom);
                    if v.is_finite() {
                        it.max_dessins[k] = (v as usize).clamp(1, if k == 3 { MAX_POLYLINES_PINE } else { MAX_DRAWINGS });
                    }
                }
                // 🧪 a strategy: its broker, with its properties (TradingView's defaults otherwise)
                if name == "strategy" && !it.sous {
                    let r = reglages_strategie(it, &a, line);
                    it.strat = Some(super::strategie::Courtier::new(r));
                }
            }
            nan
        }
        "library" => return it.err(line, "library() : pas encore dans Argos Script"),
        // ── na ───────────────────────────────────────────────────────────────────────────────────────────────────
        "nz" => {
            let v = a.val(0, "source");
            if v.is_na() {
                a.get(1, "replacement").cloned().unwrap_or(Value::Num(0.0))
            } else {
                v
            }
        }
        "na" => Value::Bool(a.val(0, "x").is_na()),
        "fixnan" => {
            let v = a.num(0, "source");
            let s = it.site(id);
            if v.is_finite() {
                s.p[0] = v;
                s.n = 1;
            }
            if s.n == 1 {
                Value::Num(s.p[0])
            } else {
                nan
            }
        }
        "int" => Value::from_f64(a.num(0, "x").trunc()),
        "float" => Value::from_f64(a.num(0, "x")),
        // (type cast, like Pine: color(na) is an empty colour, a colour stays itself, a number is a 0xRRGGBB colour)
        "color" => match a.val(0, "x") {
            Value::Color(c) => Value::Color(c),
            Value::Na => Value::Na,
            v => Value::Color(v.num() as u32),
        },
        "bool" => Value::Bool(a.val(0, "x").truthy()),
        // (casts of the drawing types, like Pine: `var bo = box(na)` is an empty box until one is made)
        "box" | "line" | "label" | "table" | "linefill" | "polyline" => a.val(0, "x"),
        "string" | "str" => Value::str(&a.text(0, "x")),
        "max_bars_back" => nan,
        // 🔔 alertcondition(condition, title, message): a named signal (1 where the condition is true), one per call site
        "alertcondition" => {
            let cle = (id, it.path);
            let k = match it.alertcond_at.get(&cle) {
                Some(&k) => k,
                None => {
                    let titre = a.text(1, "title");
                    let titre = if titre.is_empty() {
                        format!("Alerte {}", it.out.alertconditions.len() + 1)
                    } else {
                        titre
                    };
                    memoire::par_bougie(n, std::mem::size_of::<f64>());
                    it.out.alertconditions.push((titre, a.text(2, "message"), vec![0.0; n]));
                    it.alertcond_at.insert(cle, it.out.alertconditions.len() - 1);
                    it.out.alertconditions.len() - 1
                }
            };
            let vrai = a.get(0, "condition").is_some_and(|v| v.truthy());
            it.out.alertconditions[k].2[it.i] = vrai as u8 as f64;
            nan
        }
        "alert" | "log.info" | "log.warning" | "log.error" => {
            let msg = if a.count() > 1 {
                match format_text(&a.text(0, "message"), &a.v.iter().skip(1).map(|x| x.1.clone()).collect::<Vec<_>>()) {
                    // (B-062: the text the pattern gives is kept by the log, far longer than its pieces)
                    Some(t) => {
                        memoire::texte(t.capacity());
                        t
                    }
                    None => return it.err(line, format!("{name} : 100 000 caractères au plus dans un texte")),
                }
            } else {
                a.text(0, "message")
            };
            it.out.logs.push((i, msg));
            nan
        }
        "timestamp" => {
            // timestamp([fuseau,] année, mois, jour, heure, minute, seconde)
            let off = if matches!(a.get(0, "timezone"), Some(Value::Str(_))) { 1 } else { 0 };
            let g = |k: usize, def: f64| a.v.iter().filter(|x| x.0.is_none()).nth(k + off).map_or(def, |x| x.1.num());
            let d = chrono::NaiveDate::from_ymd_opt(g(0, 1970.0) as i32, g(1, 1.0) as u32, g(2, 1.0) as u32)
                .and_then(|d| d.and_hms_opt(g(3, 0.0) as u32, g(4, 0.0) as u32, g(5, 0.0) as u32));
            d.map_or(nan, |d| Value::Num(d.and_utc().timestamp() as f64 * 1000.0))
        }
        // ── inputs ───────────────────────────────────────────────────────────────────────────────────────────────
        "input" | "input.int" | "input.float" | "input.bool" | "input.string" | "input.source" | "input.color" | "input.price" | "input.time"
        | "input.session" | "input.symbol" | "input.text_area" | "input.timeframe" | "input.enum" => input(it, name, &a, id)?,
        // ── tracés ───────────────────────────────────────────────────────────────────────────────────────────────
        "plot" => {
            let key = (id, it.path);
            let k = match it.plot_at.get(&key) {
                Some(&k) => k,
                None => {
                    let style = match a.text(4, "style").as_str() {
                        "linebr" => PlotStyle::LineBr,
                        "stepline" | "steplinebr" | "stepline_diamond" => PlotStyle::StepLine,
                        "histogram" => PlotStyle::Histogram,
                        "columns" => PlotStyle::Columns,
                        "circles" => PlotStyle::Circles,
                        "cross" => PlotStyle::Cross,
                        "area" | "areabr" => PlotStyle::Area,
                        _ => PlotStyle::Line,
                    };
                    let title = a.text(1, "title");
                    let title = if title.is_empty() {
                        format!("Plot {}", it.out.plots.len() + 1)
                    } else {
                        title
                    };
                    memoire::par_bougie(n, std::mem::size_of::<(f64, Option<u32>)>());
                    it.out.plots.push(Plot {
                        title,
                        values: vec![f64::NAN; n],
                        colors: vec![None; n],
                        width: a.num_or(3, "linewidth", 1.0),
                        style,
                        offset: a.num_or(7, "offset", 0.0) as i64,
                        visible: a.text(11, "display") != "none",
                    });
                    it.plot_at.insert(key, it.out.plots.len() - 1);
                    it.out.plots.len() - 1
                }
            };
            let p = &mut it.out.plots[k];
            p.values[i] = a.num(0, "series");
            p.colors[i] = match a.get(2, "color") {
                None => Some(PLOT_BLUE),
                Some(Value::Color(c)) => Some(*c),
                Some(_) => None,
            };
            Value::Handle(HandleKind::Plot, k, it.passage)
        }
        // 🕯 plotcandle(open, high, low, close, title, color, wickcolor, editable, show_last, bordercolor, display)
        //    plotbar(open, high, low, close, title, color, editable, show_last, display)
        "plotcandle" | "plotbar" => {
            let bars = name == "plotbar";
            let key = (id, it.path);
            let k = match it.plot_at.get(&key) {
                Some(&k) => k,
                None => {
                    let title = a.text(4, "title");
                    let title = if title.is_empty() {
                        format!("{} {}", if bars { "Bars" } else { "Candles" }, it.out.candles.len() + 1)
                    } else {
                        title
                    };
                    memoire::par_bougie(n, std::mem::size_of::<([f64; 4], [Option<u32>; 3])>());
                    it.out.candles.push(super::Candles {
                        title,
                        ohlc: vec![[f64::NAN; 4]; n],
                        color: vec![None; n],
                        wick: vec![None; n],
                        border: vec![None; n],
                        bars,
                        visible: a.text(if bars { 8 } else { 10 }, "display") != "none",
                    });
                    it.plot_at.insert(key, it.out.candles.len() - 1);
                    it.out.candles.len() - 1
                }
            };
            let ohlc = [a.num(0, "open"), a.num(1, "high"), a.num(2, "low"), a.num(3, "close")];
            // (no `color`: Pine's blue; a `color` that is na on this bar: nothing drawn on it)
            let color = match a.get(5, "color") {
                None => Some(PLOT_BLUE),
                Some(_) => a.color(5, "color"),
            };
            let other = |k: usize, nom: &str| match a.get(k, nom) {
                None => color,
                Some(_) => a.color(k, nom),
            };
            let (wick, border) = if bars {
                (color, color)
            } else {
                (other(6, "wickcolor"), other(9, "bordercolor"))
            };
            let c = &mut it.out.candles[k];
            c.ohlc[i] = ohlc;
            c.color[i] = color;
            c.wick[i] = wick;
            c.border[i] = border;
            nan
        }
        "hline" => {
            let s = it.site(id);
            if s.n == 0 {
                s.n = 1;
                let k = it.out.hlines.len();
                it.site(id).p[0] = k as f64;
                it.out.hlines.push((
                    a.num(0, "price"),
                    a.text(1, "title"),
                    a.color(2, "color").unwrap_or(0x787B86),
                    a.text(3, "linestyle"),
                ));
            }
            Value::Handle(HandleKind::Hline, it.site(id).p[0] as usize, it.passage)
        }
        "fill" => {
            let h = |v: Value| match v {
                Value::Handle(k @ (HandleKind::Plot | HandleKind::Hline), x, _) => Some((k, x)),
                _ => None,
            };
            let (Some(x), Some(y)) = (h(a.val(0, "plot1").clone()), h(a.val(1, "plot2").clone())) else {
                return it.err(line, "fill() : deux plot ou hline");
            };
            let s = it.site(id);
            if s.n == 0 {
                s.n = 1;
                let k = it.out.fills.len();
                it.site(id).p[0] = k as f64;
                memoire::par_bougie(n, std::mem::size_of::<Option<u32>>());
                it.out.fills.push((x, y, vec![None; n]));
            }
            let k = it.site(id).p[0] as usize;
            // Pine: no `color` → the default blue; a `color` that is `na` on this bar → nothing drawn on this bar
            it.out.fills[k].2[i] = match a.get(2, "color") {
                None => Some(0x2962FF | (90 << 24)),
                Some(_) => a.color(2, "color"),
            };
            nan
        }
        "bgcolor" => {
            if let Some(c) = a.color(0, "color") {
                let j = (i as i64 + a.num_or(1, "offset", 0.0) as i64).clamp(0, n as i64 - 1) as usize;
                it.out.bgcolor[j] = Some(c);
            }
            nan
        }
        "barcolor" => {
            if let Some(c) = a.color(0, "color") {
                let j = (i as i64 + a.num_or(1, "offset", 0.0) as i64).clamp(0, n as i64 - 1) as usize;
                it.out.barcolor[j] = Some(c);
            }
            nan
        }
        "plotshape" | "plotchar" | "plotarrow" => {
            let v = a.val(0, "series");
            let (loc_k, color_k, offset_k, text_k, tc_k, size_k) = match name {
                "plotshape" => (3, 4, 5, 6, 7, 9),
                "plotchar" => (3, 4, 5, 6, 7, 9),
                _ => (9, 2, 4, 9, 9, 9),
            };
            let up = v.num() > 0.0;
            let on = if name == "plotarrow" {
                v.num() != 0.0 && !v.is_na()
            } else {
                v.truthy() || (matches!(v, Value::Num(_)) && !v.is_na() && a.text(loc_k, "location") == "absolute")
            };
            if on {
                let loc = match a.text(loc_k, "location").as_str() {
                    "belowbar" => Loc::BelowBar,
                    "absolute" => Loc::Absolute,
                    "top" => Loc::Top,
                    "bottom" => Loc::Bottom,
                    _ if name == "plotarrow" => {
                        if up {
                            Loc::BelowBar
                        } else {
                            Loc::AboveBar
                        }
                    }
                    _ => Loc::AboveBar,
                };
                let shape = match name {
                    "plotshape" => {
                        let s = a.text(2, "style");
                        if s.is_empty() {
                            "xcross".to_string()
                        } else {
                            s
                        }
                    }
                    "plotchar" => {
                        let c = a.text(2, "char");
                        if c.is_empty() {
                            "★".to_string()
                        } else {
                            c
                        }
                    }
                    _ => {
                        if up {
                            "arrowup".into()
                        } else {
                            "arrowdown".into()
                        }
                    }
                };
                let j = (i as i64 + a.num_or(offset_k, "offset", 0.0) as i64).clamp(0, n as i64 - 1) as usize;
                let def = if name == "plotarrow" {
                    if up {
                        0x4CAF50
                    } else {
                        0xFF5252
                    }
                } else {
                    PLOT_BLUE
                };
                it.out.shapes.push(Shape {
                    bar: j,
                    price: v.num(),
                    loc,
                    shape,
                    color: a
                        .color(color_k, "color")
                        .or(a.color(9, if up { "colorup" } else { "colordown" }))
                        .unwrap_or(def),
                    text: a.text(text_k, "text"),
                    text_color: a.color(tc_k, "textcolor").unwrap_or(0xD1D4DC),
                    size: a.text(size_k, "size"),
                    title: a.text(1, "title"),
                    force_overlay: a.flag(usize::MAX, "force_overlay", false),
                });
            }
            nan
        }
        // ⏱ timeframe.change(timeframe): true on the first chart bar of a new bar of that timeframe (UTC grouping, like
        // request.security), and on the first bar
        "timeframe.change" => {
            let t = a.text(0, "timeframe");
            let Some(tf) = tf_secs(&t, it.tf) else {
                return it.err(line, format!("timeframe.change : unité de temps inconnue « {t} »"));
            };
            let g = |k: usize| super::interp::groupe_ut(it.bars[k].time, tf);
            Value::Bool(it.i == 0 || g(it.i) != g(it.i - 1))
        }
        // ⏱ time(timeframe) / time_close(timeframe): when the `timeframe` period holding this bar opens / closes, in
        //    milliseconds (Pine; `time("M")` = the first of the month at 00:00 UTC). No timeframe, or one smaller than
        //    the chart's: the bar's own. A session is not handled yet (crypto trades all day).
        "time" | "time_close" => {
            let t = if a.get(0, "timeframe").is_some() {
                a.text(0, "timeframe")
            } else {
                String::new()
            };
            let Some(tf) = tf_secs(&t, it.tf) else {
                return it.err(line, format!("{name} : unité de temps inconnue « {t} »"));
            };
            if !a.text(1, "session").is_empty() {
                return it.err(line, format!("{name} : la séance (session) : pas encore dans Argos Script"));
            }
            let b = it.bars[it.i].time;
            let (debut, fin) = if tf <= it.tf {
                (b, b + it.tf)
            } else {
                let g = super::interp::groupe_ut(b, tf);
                (super::interp::debut_groupe(g, tf), super::interp::debut_groupe(g + 1, tf))
            };
            Value::Num(if name == "time" { debut } else { fin } as f64 * 1000.0)
        }
        // ⏱ timeframe.in_seconds([timeframe]): seconds of a timeframe (the chart's without one)
        "timeframe.in_seconds" => {
            let t = if a.get(0, "timeframe").is_some() {
                a.text(0, "timeframe")
            } else {
                String::new()
            };
            tf_secs(&t, it.tf).map_or(Value::Na, |v| Value::Num(v as f64))
        }
        // ── ta ───────────────────────────────────────────────────────────────────────────────────────────────────
        n if n.starts_with("ta.") => ta(it, &n[3..], &a, id, line)?,
        // ── math ─────────────────────────────────────────────────────────────────────────────────────────────────
        n if n.starts_with("math.") => {
            let x = a.num(0, "number");
            let f = |v: f64| Value::from_f64(v);
            match &n[5..] {
                "abs" => f(x.abs()),
                "ceil" => f(x.ceil()),
                "floor" => f(x.floor()),
                "round" => {
                    let p = a.num_or(1, "precision", 0.0);
                    let m = 10f64.powi(p as i32);
                    f((x * m).round() / m)
                }
                "round_to_mintick" => {
                    let t = mintick(it.bars);
                    f((x / t).round() * t)
                }
                "sqrt" => f(x.sqrt()),
                "exp" => f(x.exp()),
                "log" => f(x.ln()),
                "log10" => f(x.log10()),
                "pow" => f(x.powf(a.num(1, "exponent"))),
                "sign" => f(if x.is_nan() {
                    x
                } else if x > 0.0 {
                    1.0
                } else if x < 0.0 {
                    -1.0
                } else {
                    0.0
                }),
                "sin" => f(x.sin()),
                "cos" => f(x.cos()),
                "tan" => f(x.tan()),
                "asin" => f(x.asin()),
                "acos" => f(x.acos()),
                "atan" => f(x.atan()),
                "todegrees" => f(x.to_degrees()),
                "toradians" => f(x.to_radians()),
                "max" | "min" | "avg" => {
                    let xs: Vec<f64> = a.v.iter().map(|v| v.1.num()).collect();
                    if xs.iter().any(|v| v.is_nan()) {
                        nan
                    } else {
                        match &n[5..] {
                            "max" => f(xs.iter().copied().fold(f64::MIN, f64::max)),
                            "min" => f(xs.iter().copied().fold(f64::MAX, f64::min)),
                            _ => f(xs.iter().sum::<f64>() / xs.len().max(1) as f64),
                        }
                    }
                }
                "sum" => {
                    let len = a.len(1, "length");
                    match window(it.site(id), x, len) {
                        Some(w) => f(w.iter().sum()),
                        None => nan,
                    }
                }
                "random" => {
                    // pseudo-aléatoire reproductible (le même à chaque calcul)
                    let (lo, hi) = (a.num_or(0, "min", 0.0), a.num_or(1, "max", 1.0));
                    let s = it.site(id);
                    s.n =
                        s.n.wrapping_mul(6_364_136_223_846_793_005)
                            .wrapping_add(1_442_695_040_888_963_407 + a.num_or(2, "seed", 0.0) as u64);
                    f(lo + (hi - lo) * ((s.n >> 11) as f64 / (1u64 << 53) as f64))
                }
                m => return it.err(line, format!("math.{m} : pas encore dans Argos Script")),
            }
        }
        // ── str ──────────────────────────────────────────────────────────────────────────────────────────────────
        n if n.starts_with("str.") => {
            let s = a.text(0, "source");
            match &n[4..] {
                "tostring" => {
                    let v = a.val(0, "value");
                    match (a.text(1, "format").as_str(), &v) {
                        (f, Value::Num(x)) if f.starts_with('#') || f.contains('.') => {
                            let dec = decimales(f);
                            Value::str(&format!("{x:.dec$}"))
                        }
                        ("percent", Value::Num(x)) => Value::str(&format!("{x:.2}%")),
                        // format.volume, as TradingView writes it: « 950 », « 4.39K », « 16.213K », « 1.2M »
                        ("volume", Value::Num(x)) => Value::str(&volume_text(*x)),
                        // format.mintick / format.price: rounded to the symbol's tick
                        ("mintick" | "price", Value::Num(x)) => {
                            let t = mintick(it.bars);
                            let dec = (0..10).find(|&d| (t * 10f64.powi(d)).fract().abs() < 1e-9).unwrap_or(8) as usize;
                            Value::str(&format!("{:.dec$}", (x / t).round() * t))
                        }
                        // (B-049: the text of a collection had no limit)
                        _ => {
                            let t = texte_de(it, line, "str.tostring", &v)?;
                            valeur_texte(it, line, "str.tostring", t.len(), || t)?
                        }
                    }
                }
                "tonumber" => Value::from_f64(s.trim().parse().unwrap_or(f64::NAN)),
                "length" => Value::Num(s.chars().count() as f64),
                "upper" => Value::str(&s.to_uppercase()),
                "lower" => Value::str(&s.to_lowercase()),
                "contains" => Value::Bool(s.contains(&a.text(1, "str"))),
                "startswith" => Value::Bool(s.starts_with(&a.text(1, "str"))),
                "endswith" => Value::Bool(s.ends_with(&a.text(1, "str"))),
                "pos" => Value::from_f64(s.find(&a.text(1, "str")).map_or(f64::NAN, |p| s[..p].chars().count() as f64)),
                // (B-049: a text never grows past its limit, an error with its line in place of a giant allocation)
                "replace_all" => {
                    let (de, par) = (a.text(1, "target"), a.text(2, "replacement"));
                    let octets = s.len().saturating_add(s.matches(de.as_str()).count().saturating_mul(par.len()));
                    valeur_texte(it, line, "str.replace_all", octets, || s.replace(&de, &par))?
                }
                "replace" => {
                    let (de, par) = (a.text(1, "target"), a.text(2, "replacement"));
                    valeur_texte(it, line, "str.replace", s.len().saturating_add(par.len()), || s.replacen(&de, &par, 1))?
                }
                "substring" => {
                    let cs: Vec<char> = s.chars().collect();
                    let b = a.len(1, "begin_pos").min(cs.len());
                    let e = a.get(2, "end_pos").map_or(cs.len(), |v| (v.num().max(0.0) as usize).min(cs.len()));
                    Value::str(&cs[b..e.max(b)].iter().collect::<String>())
                }
                "split" => {
                    // (B-062: the pieces are an array like any other: its limit, counted before it is made)
                    let sep = a.text(1, "separator");
                    if s.split(&sep).count() > it.max_elements {
                        return it.err(line, trop_d_elements(it, "str.split"));
                    }
                    Value::arr(s.split(&sep).map(Value::str).collect())
                }
                "trim" => Value::str(s.trim()),
                "repeat" => {
                    let n = a.len(1, "repeat");
                    valeur_texte(it, line, "str.repeat", s.len().saturating_mul(n), || s.repeat(n))?
                }
                "format" => match format_text(&a.text(0, "formatString"), &a.v.iter().skip(1).map(|x| x.1.clone()).collect::<Vec<_>>()) {
                    Some(t) => Value::str(&t),
                    None => return it.err(line, "str.format : 100 000 caractères au plus dans un texte"),
                },
                m => return it.err(line, format!("str.{m} : pas encore dans Argos Script")),
            }
        }
        // ── color ────────────────────────────────────────────────────────────────────────────────────────────────
        "color.new" => match a.get(0, "color") {
            Some(Value::Color(c)) => Value::Color(color(
                ((c >> 16) & 0xff) as f64,
                ((c >> 8) & 0xff) as f64,
                (c & 0xff) as f64,
                a.num_or(1, "transp", 0.0),
            )),
            _ => nan,
        },
        "color.rgb" => Value::Color(color(a.num(0, "red"), a.num(1, "green"), a.num(2, "blue"), a.num_or(3, "transp", 0.0))),
        "color.r" | "color.g" | "color.b" | "color.t" => match a.get(0, "color") {
            Some(Value::Color(c)) => {
                let (r, g, b, t) = rgba(*c);
                Value::Num(match name {
                    "color.r" => r,
                    "color.g" => g,
                    "color.b" => b,
                    _ => t,
                } as f64)
            }
            _ => nan,
        },
        "color.from_gradient" => {
            let (v, lo, hi) = (a.num(0, "value"), a.num(1, "bottom_value"), a.num(2, "top_value"));
            match (a.color(3, "bottom_color"), a.color(4, "top_color")) {
                (Some(c1), Some(c2)) if v.is_finite() && hi != lo => {
                    let t = ((v - lo) / (hi - lo)).clamp(0.0, 1.0);
                    let (p, q) = (rgba(c1), rgba(c2));
                    let m = |x: u8, y: u8| x as f64 + (y as f64 - x as f64) * t;
                    Value::Color(color(m(p.0, q.0), m(p.1, q.1), m(p.2, q.2), m(p.3, q.3)))
                }
                (Some(c1), _) => Value::Color(c1),
                _ => nan,
            }
        }
        // ── tableaux et maps ─────────────────────────────────────────────────────────────────────────────────────
        n if n.starts_with("array.") => array(it, &n[6..], &a, gen, line)?,
        n if n.starts_with("map.") => map(&n[4..], &a, it, line)?,
        n if n.starts_with("matrix.") => matrix(&n[7..], &a, it, line)?,
        // ── dessins ──────────────────────────────────────────────────────────────────────────────────────────────
        n if n.starts_with("chart.point.") => chart_point(it, &n["chart.point.".len()..], &a, line)?,
        n if n.starts_with("polyline.") => polyline(it, &n["polyline.".len()..], &a, line)?,
        n if n.starts_with("label.") || n.starts_with("line.") || n.starts_with("box.") => drawing(it, n, &a),
        // ── pas encore ───────────────────────────────────────────────────────────────────────────────────────────
        n if n.starts_with("request.") => return it.err(line, format!("{n} : pas encore dans Argos Script (bientôt : multi-unités de temps)")),
        n if n.starts_with("strategy.") => strategy(it, &n[9..], &a, line)?,
        n if n.starts_with("table.") => table(it, n, &a, id, line)?,
        n if n.starts_with("linefill.") => linefill(it, n, &a),
        n => return it.err(line, format!("« {n} » n'existe pas (ou pas encore dans Argos Script)")),
    })
}

/// 🧪 The properties of `strategy(...)` (by name, or at Pine's positions), TradingView's defaults otherwise. What
/// Argos does not do yet is said once in the notes.
fn reglages_strategie(it: &mut Interp, a: &A, line: u32) -> super::strategie::Reglages {
    use super::strategie::{Reglages, Taille, PROPRIETES, TAILLES};
    let d = Reglages::default();
    let num = |k: usize, n: &str, def: f64| a.get(k, n).filter(|v| !v.is_na()).map_or(def, Value::num);
    let commission = a.text(16, "commission_type");
    let pct = commission.is_empty() || commission == "percent";
    if !pct && num(17, "commission_value", 0.0) != 0.0 {
        it.out.notes.push((
            line,
            format!("commission « {commission} » : pas encore dans Argos Script, comptée à 0 (seule « percent » l'est)"),
        ));
    }
    // ⚙ the properties declared in `strategy(...)`, changeable in the settings like TradingView's Properties tab
    let taille = Taille::de(&a.text(11, "default_qty_type"));
    let mut prop = |titre: &str, kind: &'static str, def: InVal, min: Option<f64>, options: &[&str]| {
        if it.seen_inputs.insert(titre.to_string()) {
            it.out.inputs.push(InputSpec {
                title: titre.to_string(),
                kind,
                def: def.clone(),
                min,
                max: None,
                step: None,
                options: options.iter().map(|o| o.to_string()).collect(),
                tooltip: String::new(),
                group: PROPRIETES.into(),
                inline: String::new(),
                confirm: false,
            });
        }
        it.inputs.get(titre).cloned().unwrap_or(def)
    };
    let nombre = |v: InVal, def: f64| match v {
        InVal::Num(x) if x.is_finite() => x,
        _ => def,
    };
    let capital = num(13, "initial_capital", d.capital).max(0.0);
    let capital = nombre(prop("Capital de départ", "float", InVal::Num(capital), Some(0.0), &[]), capital);
    let valeur = num(12, "default_qty_value", d.valeur);
    let valeur = nombre(prop("Taille des ordres", "float", InVal::Num(valeur), Some(0.0), &[]), valeur);
    let taille = match prop("Type de taille", "string", InVal::Text(taille.nom().into()), None, &TAILLES) {
        InVal::Text(t) => Taille::du_nom(&t).unwrap_or(taille),
        _ => taille,
    };
    let pyramiding = num(6, "pyramiding", 0.0).max(0.0);
    let pyramiding = nombre(prop("Pyramiding", "int", InVal::Num(pyramiding), Some(0.0), &[]), pyramiding);
    let commission_pct = if pct { num(17, "commission_value", 0.0).max(0.0) } else { 0.0 };
    let commission_pct = nombre(prop("Commission %", "float", InVal::Num(commission_pct), Some(0.0), &[]), commission_pct);
    let glissement = num(15, "slippage", 0.0).max(0.0);
    let glissement = nombre(prop("Glissement (ticks)", "int", InVal::Num(glissement), Some(0.0), &[]), glissement);
    let cloture = a.flag(18, "process_orders_on_close", false);
    let a_la_cloture = match prop("Ordres à la clôture", "bool", InVal::Bool(cloture), None, &[]) {
        InVal::Bool(b) => b,
        _ => cloture,
    };
    Reglages {
        capital: capital.max(0.0),
        taille,
        valeur,
        pyramiding: pyramiding.max(0.0).round() as u32,
        commission_pct: commission_pct.max(0.0),
        glissement: glissement.max(0.0).round(),
        a_la_cloture,
        tick: mintick(it.bars),
    }
}

/// The most characters a text kept by the broker holds (the id of an order, its comment, its group).
const MAX_TEXTE_ORDRE: usize = 1_000;

/// 🧪 `strategy.*` calls: orders to the broker (they fill from the next price on).
fn strategy(it: &mut Interp, f: &str, a: &A, line: u32) -> Result<Value, Error> {
    use super::strategie::{Entree, Fermeture, Sortie};
    use argos_core::Side;
    // (Pine v4 / v5 `when = false`: the call does nothing)
    if it.sous || a.get(usize::MAX, "when").is_some_and(|w| !w.truthy()) {
        return Ok(Value::Na);
    }
    let opt = |k: usize, n: &str| a.get(k, n).filter(|v| !v.is_na()).map(Value::num).filter(|x| x.is_finite());
    // (B-062: the broker copies the texts of an order at each of its fills, between two statements of the script,
    // where nothing counts the memory: the texts it keeps stay short. The others are free: an `alert_message` is
    // often a long line of JSON, and it is not kept.)
    let gardes: &[(usize, &str)] = match f {
        "entry" | "order" => &[(0, "id"), (5, "oca_name"), (7, "comment")],
        "exit" => &[(0, "id"), (1, "from_entry"), (12, "comment")],
        "close" => &[(0, "id"), (1, "comment")],
        "close_all" => &[(0, "comment")],
        "cancel" => &[(0, "id")],
        _ => &[],
    };
    let long = |v: &Value| match v {
        Value::Str(s) => s.len() > MAX_TEXTE_ORDRE && s.chars().count() > MAX_TEXTE_ORDRE,
        v => v.lourd(),
    };
    if let Some((_, nom)) = gardes.iter().find(|(k, n)| a.get(*k, n).is_some_and(long)) {
        return it.err(line, format!("strategy.{f} : 1 000 caractères au plus dans « {nom} »"));
    }
    let Some(c) = it.strat.as_mut() else {
        return it.err(line, format!("strategy.{f} : seulement dans un script strategy(...)"));
    };
    match f {
        "entry" | "order" => {
            // direction: strategy.long / strategy.short (Pine v4: `long = true`)
            let side = match a.get(1, "direction").or_else(|| a.get(usize::MAX, "long")) {
                Some(Value::Bool(b)) => {
                    if *b {
                        Side::Long
                    } else {
                        Side::Short
                    }
                }
                Some(v) if v.text() == "short" => Side::Short,
                Some(v) if v.text() == "long" => Side::Long,
                _ => return it.err(line, format!("strategy.{f} : le sens (strategy.long ou strategy.short) manque")),
            };
            let oca = if a.text(6, "oca_type") == "cancel" {
                a.text(5, "oca_name")
            } else {
                String::new()
            };
            c.entree(
                Entree {
                    id: a.text(0, "id"),
                    side,
                    qty: opt(2, "qty"),
                    limit: opt(3, "limit"),
                    stop: opt(4, "stop"),
                    brut: f == "order",
                    comment: a.text(7, "comment"),
                    oca,
                },
                false,
            );
        }
        "exit" => c.sortie(Sortie {
            id: a.text(0, "id"),
            de: a.text(1, "from_entry"),
            qty: opt(2, "qty"),
            qty_pct: opt(3, "qty_percent"),
            profit: opt(4, "profit"),
            limit: opt(5, "limit"),
            loss: opt(6, "loss"),
            stop: opt(7, "stop"),
            trail_price: opt(8, "trail_price"),
            trail_points: opt(9, "trail_points"),
            trail_offset: opt(10, "trail_offset"),
            comment: a.text(12, "comment"),
            ..Default::default()
        }),
        "close" => c.fermer(
            Fermeture {
                de: Some(a.text(0, "id")),
                qty: opt(2, "qty"),
                qty_pct: opt(3, "qty_percent"),
                comment: a.text(1, "comment"),
            },
            a.flag(5, "immediately", false),
        ),
        "close_all" => c.fermer(
            Fermeture {
                de: None,
                qty: None,
                qty_pct: None,
                comment: a.text(0, "comment"),
            },
            a.flag(2, "immediately", false),
        ),
        "cancel" => c.annuler(Some(&a.text(0, "id"))),
        "cancel_all" => c.annuler(None),
        // (one account currency: the symbol's)
        "convert_to_account" | "convert_to_symbol" => return Ok(a.val(0, "value")),
        "default_entry_qty" => return Ok(Value::from_f64(c.taille_par_defaut(a.num(0, "fill_price")))),
        _ => return it.err(line, format!("strategy.{f} : pas encore dans Argos Script")),
    }
    Ok(Value::Na)
}

/// `input.*` : la valeur choisie (par titre) ou celle par défaut ; l'input est noté une fois pour la fenêtre de réglages.
fn input(it: &mut Interp, name: &str, a: &A, id: u64) -> Result<Value, Error> {
    let def = a.val(0, "defval");
    let kind: &'static str = match name {
        "input.int" => "int",
        "input.float" => "float",
        "input.bool" => "bool",
        "input.string" | "input.enum" => "string",
        "input.source" => "source",
        "input.color" => "color",
        "input.price" => "price",
        "input.time" => "time",
        "input.session" => "session",
        "input.symbol" => "symbol",
        "input.text_area" => "text_area",
        "input.timeframe" => "timeframe",
        _ => match &def {
            Value::Bool(_) => "bool",
            Value::Str(_) => "string",
            Value::Color(_) => "color",
            _ => "float",
        },
    };
    let title = a.text(1, "title");
    // An untitled input: on a line (`inline`) next to a titled one, a stable key from that line — the first titled
    // input's title and its rank on the line (« MA #1 · 2 »), like TradingView which shows no label there; else
    // « Input N », N being its place among the inputs written in the script. That name comes from the text of the
    // script alone, so it is the same at every bar and in every run of the script.
    let inline_txt = a.text(6, "inline");
    let ligne = (!inline_txt.is_empty()).then(|| {
        let e = it.lignes_inputs.entry(inline_txt.clone()).or_insert_with(|| (String::new(), 0));
        e.1 += 1;
        if e.0.is_empty() && !title.is_empty() {
            e.0 = title.clone();
        }
        e.clone()
    });
    let title = match ligne {
        _ if !title.is_empty() => title,
        Some((base, k)) => format!("{} · {k}", if base.is_empty() { &inline_txt } else { &base }),
        None => format!("Input {}", it.place_input(id)),
    };
    let rang = it.inputs_vus.0;
    it.inputs_vus.0 += 1;
    let fois = it.inputs_vus.1.entry(title.clone()).or_insert(0);
    *fois += 1;
    let title = if *fois > 1 { format!("{title}@{rang}") } else { title };
    // source : le défaut est une série (close…) ; on garde son nom
    let src_name = |v: &Value| -> String {
        let b = &it.bars[it.i];
        for n in ["close", "open", "high", "low", "hl2", "hlc3", "ohlc4", "hlcc4", "volume"] {
            if bar_val(b, n, it.i, it.tf).is_some_and(|x| x == v.num()) {
                return n.to_string();
            }
        }
        "close".into()
    };
    let def_val = match (kind, &def) {
        ("bool", v) => InVal::Bool(v.truthy()),
        ("color", Value::Color(c)) => InVal::Color(*c),
        ("source", v) => InVal::Text(src_name(v)),
        (_, Value::Str(s)) => InVal::Text(s.to_string()),
        (_, v) => InVal::Num(v.num()),
    };
    if it.seen_inputs.insert(title.clone()) {
        let opts = match a.get(9, "options") {
            Some(Value::Tuple(t)) => t.iter().map(Value::text).collect(),
            _ => vec![],
        };
        it.out.inputs.push(InputSpec {
            title: title.clone(),
            kind,
            def: def_val.clone(),
            min: a.get(2, "minval").map(Value::num),
            max: a.get(3, "maxval").map(Value::num),
            step: a.get(4, "step").map(Value::num),
            options: opts,
            tooltip: a.text(5, "tooltip"),
            group: a.text(7, "group"),
            inline: a.text(6, "inline"),
            confirm: a.flag(8, "confirm", false),
        });
        // (B-062: what the settings keep of a new input besides what the call told — a copy of its default text,
        // and its title twice more, in the titles seen and in their count — is told when it is kept: 20 000 inputs
        // with a default text of 100 000 characters were 2 GB kept for 12 MB told, and no count ever came)
        let defaut = match &def_val {
            InVal::Text(s) => s.len(),
            _ => 0,
        };
        memoire::creer(defaut.saturating_add(title.len().saturating_mul(2)));
    }
    let chosen = it.inputs.get(&title).cloned().unwrap_or(def_val);
    Ok(match (kind, chosen) {
        ("source", InVal::Text(s)) => Value::from_f64(bar_val(&it.bars[it.i], &s, it.i, it.tf).unwrap_or(def.num())),
        (_, InVal::Num(v)) => Value::from_f64(if kind == "int" { v.round() } else { v }),
        (_, InVal::Bool(b)) => Value::Bool(b),
        (_, InVal::Text(s)) => Value::str(&s),
        (_, InVal::Color(c)) => Value::Color(c),
    })
}

/// `linefill.*` (step 4), like TradingView: `linefill.new(line1, line2, color)` fills between two lines and follows
/// them (drawn from where they are at the end), is gone when one of them is deleted, at most 500 (the oldest go);
/// `linefill.set_color`, `linefill.delete`, `linefill.get_line1` / `get_line2`.
fn linefill(it: &mut Interp, f: &str, a: &A) -> Value {
    use super::Linefill;
    let ligne = |v: Value| match v {
        Value::Handle(HandleKind::Drawing, k, _) => Some(k),
        _ => None,
    };
    if f == "linefill.new" {
        let (Some(l1), Some(l2)) = (ligne(a.val(0, "line1")), ligne(a.val(1, "line2"))) else {
            return Value::Na; // (na lines: no fill, like Pine)
        };
        it.out.linefills.push(Linefill {
            line1: l1,
            line2: l2,
            color: a.color_or(2, "color", PLOT_BLUE | (80 << 24)),
            deleted: false,
        });
        // (B-064: the oldest go without the list being read again)
        let id = it.ids_linefills.nouveau(0);
        let Interp { ids_linefills, out, .. } = it;
        while let Some(place) = ids_linefills.plus_vieux(0, MAX_DRAWINGS, |p| !out.linefills[p].deleted) {
            out.linefills[place].deleted = true;
            ids_linefills.efface(0);
        }
        ids_linefills.tasser(&mut out.linefills, |l| (!l.deleted).then_some(0));
        return Value::Handle(HandleKind::Linefill, id, it.passage);
    }
    let Value::Handle(HandleKind::Linefill, k, _) = a.val(0, "id") else {
        return Value::Na; // (na.delete() and the like: nothing to do, like Pine)
    };
    // (a deleted fill is `na` for every function: nothing to read, nothing to change)
    let Some(lf) = it.ids_linefills.place(k).and_then(|p| it.out.linefills.get_mut(p)).filter(|l| !l.deleted) else {
        return Value::Na;
    };
    match f {
        "linefill.delete" => {
            lf.deleted = true;
            it.ids_linefills.efface(0);
            it.ids_linefills.tasser(&mut it.out.linefills, |l| (!l.deleted).then_some(0));
            Value::Na
        }
        "linefill.set_color" => {
            if let Some(c) = a.color(1, "color") {
                lf.color = c;
            }
            Value::Na
        }
        "linefill.get_line1" => Value::Handle(HandleKind::Drawing, lf.line1, it.passage),
        "linefill.get_line2" => Value::Handle(HandleKind::Drawing, lf.line2, it.passage),
        _ => Value::Na,
    }
}

/// 📋 `table.*` (step 4): `table.new` (one table per call site, made again empty when called again, like a table
/// that is not `var`), `table.cell`, `table.cell_set_text` / `_bgcolor` / `_text_color`, `table.clear`,
/// `table.delete`; the other `table.*` functions change nothing.
fn table(it: &mut Interp, f: &str, a: &A, id: u64, line: u32) -> Result<Value, Error> {
    use super::{Table, TableCell};
    if f == "table.new" {
        // (B-049: compared as numbers, before the cast — as sizes, 2^32 × 2^32 wrapped to 0 and passed)
        let (cols, rows) = (a.num_or(1, "columns", 1.0).max(1.0), a.num_or(2, "rows", 1.0).max(1.0));
        if cols * rows > 10_000.0 {
            return it.err(line, "table.new : trop de cases (10 000 au plus)");
        }
        let (cols, rows) = (cols as usize, rows as usize);
        memoire::par_bougie(cols * rows, std::mem::size_of::<Option<TableCell>>());
        let t = Table {
            position: a.text(0, "position"),
            columns: cols,
            rows,
            bgcolor: a.color(3, "bgcolor"),
            frame_color: a.color(4, "frame_color"),
            border_color: a.color(6, "border_color"),
            cells: vec![vec![None; cols]; rows],
            deleted: false,
        };
        let cle = (id, it.path);
        let k = match it.table_at.get(&cle) {
            Some(&k) => {
                it.out.tables[k] = t;
                k
            }
            None => {
                it.out.tables.push(t);
                it.table_at.insert(cle, it.out.tables.len() - 1);
                it.out.tables.len() - 1
            }
        };
        return Ok(Value::Handle(HandleKind::Table, k, it.passage));
    }
    let Value::Handle(HandleKind::Table, k, _) = a.val(0, "table_id") else {
        return Ok(Value::Na);
    };
    let Some(t) = it.out.tables.get_mut(k) else { return Ok(Value::Na) };
    let (c, r) = (a.num_or(1, "column", 0.0), a.num_or(2, "row", 0.0));
    let place = (c >= 0.0 && r >= 0.0 && (c as usize) < t.columns && (r as usize) < t.rows).then_some((c as usize, r as usize));
    match f {
        "table.cell" => {
            if let Some((c, r)) = place {
                t.cells[r][c] = Some(TableCell {
                    text: a.text(3, "text"),
                    text_color: a.color(6, "text_color").unwrap_or(0x000000),
                    size: a.text(9, "text_size"),
                    bgcolor: a.color(10, "bgcolor"),
                });
            }
        }
        "table.cell_set_text" | "table.cell_set_bgcolor" | "table.cell_set_text_color" | "table.cell_set_text_size" => {
            if let Some((c, r)) = place {
                let cell = t.cells[r][c].get_or_insert_with(Default::default);
                match f {
                    "table.cell_set_text" => cell.text = a.text(3, "text"),
                    "table.cell_set_bgcolor" => cell.bgcolor = a.color(3, "bgcolor"),
                    "table.cell_set_text_color" => cell.text_color = a.color(3, "text_color").unwrap_or(cell.text_color),
                    _ => cell.size = a.text(3, "text_size"),
                }
            }
        }
        "table.clear" => {
            let (c1, r1) = (a.num_or(3, "end_column", c), a.num_or(4, "end_row", r));
            for row in (r.max(0.0) as usize)..=(r1.max(0.0) as usize).min(t.rows - 1) {
                for col in (c.max(0.0) as usize)..=(c1.max(0.0) as usize).min(t.columns - 1) {
                    t.cells[row][col] = None;
                }
            }
        }
        "table.delete" => t.deleted = true,
        _ => {}
    }
    Ok(Value::Na)
}

/// Fonctions `ta.*`, chacune avec la mémoire de son endroit.
fn ta(it: &mut Interp, f: &str, a: &A, id: u64, line: u32) -> Result<Value, Error> {
    let i = it.i;
    let bar = it.bars[i];
    let prev = it.bar_back(1).copied();
    let src = a.num(0, "source");
    let f64v = Value::from_f64;
    Ok(match f {
        "sma" => {
            let len = a.len(1, "length");
            match window(it.site(id), src, len) {
                Some(w) if w.iter().all(|v| v.is_finite()) => f64v(w.iter().sum::<f64>() / len as f64),
                _ => Value::Na,
            }
        }
        "ema" | "rma" => {
            let len = a.len(1, "length");
            let s = it.site(id);
            if s.s.is_empty() {
                memoire::pousser(&mut s.s, if f == "ema" { Smooth::ema(len) } else { Smooth::rma(len) });
            }
            f64v(s.s[0].next(src))
        }
        "wma" => {
            let len = a.len(1, "length");
            match window(it.site(id), src, len) {
                Some(w) => {
                    let den = (len * (len + 1) / 2) as f64;
                    f64v(w.iter().enumerate().map(|(k, v)| v * (k + 1) as f64).sum::<f64>() / den)
                }
                None => Value::Na,
            }
        }
        // symmetrically weighted moving average of the last 4 values (weights 1 2 2 1), like Pine
        "swma" => match window(it.site(id), src, 4) {
            Some(w) => f64v((w[0] + 2.0 * w[1] + 2.0 * w[2] + w[3]) / 6.0),
            None => Value::Na,
        },
        "vwma" => {
            let len = a.len(1, "length");
            let s = it.site(id);
            memoire::pousser(&mut s.b, bar.volume);
            match window(s, src * bar.volume, len) {
                Some(w) => {
                    let pv: f64 = w.iter().sum();
                    let v: f64 = s.b[s.b.len() - len..].iter().sum();
                    f64v(pv / v)
                }
                None => Value::Na,
            }
        }
        "hma" => {
            let len = a.len(1, "length").max(1);
            let s = it.site(id);
            let wma = |xs: &[f64], l: usize| -> f64 {
                if xs.len() < l || l == 0 {
                    return f64::NAN;
                }
                let w = &xs[xs.len() - l..];
                w.iter().enumerate().map(|(k, v)| v * (k + 1) as f64).sum::<f64>() / (l * (l + 1) / 2) as f64
            };
            memoire::pousser(&mut s.a, src);
            let raw = 2.0 * wma(&s.a, len / 2) - wma(&s.a, len);
            memoire::pousser(&mut s.b, raw);
            let sq = (len as f64).sqrt().floor() as usize;
            f64v(wma(&s.b, sq))
        }
        "rsi" => {
            let len = a.len(1, "length");
            let s = it.site(id);
            if s.s.is_empty() {
                s.s = vec![Smooth::rma(len), Smooth::rma(len)];
                s.p[0] = f64::NAN;
            }
            let p = s.p[0];
            s.p[0] = src;
            if p.is_nan() {
                return Ok(Value::Na);
            }
            let d = src - p;
            let (u, dn) = (s.s[0].next(d.max(0.0)), s.s[1].next((-d).max(0.0)));
            f64v(if u.is_nan() || dn.is_nan() {
                f64::NAN
            } else if dn == 0.0 {
                100.0
            } else if u == 0.0 {
                0.0
            } else {
                100.0 - 100.0 / (1.0 + u / dn)
            })
        }
        "atr" => {
            let len = a.len(0, "length");
            let tr = true_range(it);
            let s = it.site(id);
            if s.s.is_empty() {
                memoire::pousser(&mut s.s, Smooth::rma(len));
            }
            f64v(s.s[0].next(tr))
        }
        "tr" => {
            let handle_na = a.flag(0, "handle_na", false);
            f64v(if prev.is_none() && !handle_na { f64::NAN } else { true_range(it) })
        }
        "stdev" | "variance" | "dev" => {
            let len = a.len(1, "length");
            let biased = a.flag(2, "biased", true);
            match window(it.site(id), src, len) {
                Some(w) => {
                    let m = w.iter().sum::<f64>() / len as f64;
                    if f == "dev" {
                        f64v(w.iter().map(|x| (x - m).abs()).sum::<f64>() / len as f64)
                    } else {
                        let d = if biased { len as f64 } else { (len as f64 - 1.0).max(1.0) };
                        let var = w.iter().map(|x| (x - m).powi(2)).sum::<f64>() / d;
                        f64v(if f == "variance" { var } else { var.sqrt() })
                    }
                }
                None => Value::Na,
            }
        }
        "highest" | "lowest" | "highestbars" | "lowestbars" => {
            // ta.highest(len) : sur le haut ; ta.highest(src, len)
            let two = a.count() >= 2 || a.get(9, "source").is_some();
            let (src, len) = if two {
                (src, a.len(1, "length"))
            } else {
                (if f.starts_with("high") { bar.high } else { bar.low }, a.len(0, "length"))
            };
            // source = série de la bougie : la fenêtre vient des bougies (juste même appelé seulement dans un `if`)
            let series = if two {
                it.src.filter(|s| bar_val(&it.bars[0], s, 0, it.tf).is_some())
            } else {
                Some(if f.starts_with("high") { "high" } else { "low" })
            };
            let w: Option<Vec<f64>> = match series {
                Some(s) => {
                    (len > 0 && it.i + 1 >= len).then(|| (it.i + 1 - len..=it.i).map(|j| bar_val(&it.bars[j], s, j, it.tf).unwrap_or(f64::NAN)).collect())
                }
                None => window(it.site(id), src, len).map(<[f64]>::to_vec),
            };
            match w {
                Some(w) => {
                    let high = f.starts_with("high");
                    let mut best = 0;
                    for (k, v) in w.iter().enumerate() {
                        if (high && *v >= w[best]) || (!high && *v <= w[best]) {
                            best = k;
                        }
                    }
                    if f.ends_with("bars") {
                        Value::Num(-((len - 1 - best) as f64))
                    } else {
                        f64v(w[best])
                    }
                }
                None => Value::Na,
            }
        }
        "change" | "mom" | "roc" => {
            let len = if f == "change" {
                a.num_or(1, "length", 1.0) as usize
            } else {
                a.len(1, "length")
            };
            let v = a.val(0, "source");
            let s = it.site(id);
            memoire::pousser(&mut s.a, v.num());
            let nn = s.a.len();
            if nn <= len {
                return Ok(if matches!(v, Value::Bool(_)) { Value::Bool(false) } else { Value::Na });
            }
            let old = s.a[nn - 1 - len];
            if matches!(v, Value::Bool(_)) {
                Value::Bool(old != v.num())
            } else if f == "roc" {
                f64v(100.0 * (src - old) / old)
            } else {
                f64v(src - old)
            }
        }
        "cum" => {
            let s = it.site(id);
            if src.is_finite() {
                s.p[0] += src;
            }
            Value::Num(s.p[0])
        }
        "crossover" | "crossunder" | "cross" => {
            let (x, y) = (src, a.num(1, "source2"));
            let s = it.site(id);
            let (px, py, had) = (s.p[0], s.p[1], s.n > 0);
            (s.p[0], s.p[1], s.n) = (x, y, 1);
            let up = had && x > y && px <= py;
            let dn = had && x < y && px >= py;
            Value::Bool(match f {
                "crossover" => up,
                "crossunder" => dn,
                _ => up || dn,
            })
        }
        "rising" | "falling" => {
            let len = a.len(1, "length");
            let s = it.site(id);
            memoire::pousser(&mut s.a, src);
            let nn = s.a.len();
            if nn <= len {
                return Ok(Value::Bool(false));
            }
            let w = &s.a[nn - 1 - len..nn - 1];
            Value::Bool(if f == "rising" {
                w.iter().all(|v| src > *v)
            } else {
                w.iter().all(|v| src < *v)
            })
        }
        "barssince" => {
            let c = a.val(0, "condition").truthy();
            let s = it.site(id);
            if c {
                s.n = 1;
                s.p[0] = i as f64;
            }
            if s.n == 1 {
                Value::Num(i as f64 - s.p[0])
            } else {
                Value::Na
            }
        }
        "valuewhen" => {
            let c = a.val(0, "condition").truthy();
            let v = a.val(1, "source");
            let occ = a.num_or(2, "occurrence", 0.0) as usize;
            let s = it.site(id);
            if c {
                memoire::pousser(&mut s.v, v);
            }
            // (B-051: an occurrence out of reach, 1e30, is `na`)
            s.v.len().checked_sub(occ.saturating_add(1)).map_or(Value::Na, |k| s.v[k].clone())
        }
        "pivothigh" | "pivotlow" => {
            // ta.pivothigh(left, right) sur le haut, ou (source, left, right)
            let (src, l, r) = if a.count() >= 3 {
                (src, a.len(1, "leftbars"), a.len(2, "rightbars"))
            } else {
                (if f == "pivothigh" { bar.high } else { bar.low }, a.len(0, "leftbars"), a.len(1, "rightbars"))
            };
            let s = it.site(id);
            memoire::pousser(&mut s.a, src);
            let nn = s.a.len();
            match crate::ta::pivot(&s.a, nn - 1, l, r, f == "pivothigh") {
                Some(j) => f64v(s.a[j]),
                None => Value::Na,
            }
        }
        "macd" => {
            let (fl, sl, sg) = (a.len(1, "fastlen"), a.len(2, "slowlen"), a.len(3, "siglen"));
            let s = it.site(id);
            if s.s.is_empty() {
                s.s = vec![Smooth::ema(fl), Smooth::ema(sl), Smooth::ema(sg)];
            }
            let m = s.s[0].next(src) - s.s[1].next(src);
            let sig = s.s[2].next(m);
            tuple(vec![m, sig, m - sig])
        }
        "bb" => {
            let (len, mult) = (a.len(1, "length"), a.num(2, "mult"));
            match window(it.site(id), src, len) {
                Some(w) => {
                    let m = w.iter().sum::<f64>() / len as f64;
                    let sd = (w.iter().map(|x| (x - m).powi(2)).sum::<f64>() / len as f64).sqrt();
                    tuple(vec![m, m + mult * sd, m - mult * sd])
                }
                None => tuple(vec![f64::NAN; 3]),
            }
        }
        "bbw" => {
            let (len, mult) = (a.len(1, "length"), a.num(2, "mult"));
            match window(it.site(id), src, len) {
                Some(w) => {
                    let m = w.iter().sum::<f64>() / len as f64;
                    let sd = (w.iter().map(|x| (x - m).powi(2)).sum::<f64>() / len as f64).sqrt();
                    f64v(2.0 * mult * sd / m * 100.0)
                }
                None => Value::Na,
            }
        }
        "kc" => {
            let (len, mult) = (a.len(1, "length"), a.num(2, "mult"));
            let use_tr = a.flag(3, "useTrueRange", true);
            let range = if use_tr { true_range(it) } else { bar.high - bar.low };
            let s = it.site(id);
            if s.s.is_empty() {
                s.s = vec![Smooth::ema(len), Smooth::ema(len)];
            }
            let (m, r) = (s.s[0].next(src), s.s[1].next(range));
            tuple(vec![m, m + mult * r, m - mult * r])
        }
        "stoch" => {
            let (h, l, len) = (a.num(1, "high"), a.num(2, "low"), a.len(3, "length"));
            let s = it.site(id);
            memoire::pousser(&mut s.b, h);
            memoire::pousser(&mut s.c, l);
            let nn = s.b.len();
            if len == 0 || nn < len {
                return Ok(Value::Na);
            }
            let hh = s.b[nn - len..].iter().copied().fold(f64::MIN, f64::max);
            let ll = s.c[nn - len..].iter().copied().fold(f64::MAX, f64::min);
            f64v(if hh == ll { f64::NAN } else { 100.0 * (src - ll) / (hh - ll) })
        }
        "cci" => {
            let len = a.len(1, "length");
            match window(it.site(id), src, len) {
                Some(w) => {
                    let m = w.iter().sum::<f64>() / len as f64;
                    let dev = w.iter().map(|x| (x - m).abs()).sum::<f64>() / len as f64;
                    f64v(if dev == 0.0 { 0.0 } else { (src - m) / (0.015 * dev) })
                }
                None => Value::Na,
            }
        }
        "mfi" => {
            let len = a.len(1, "length");
            let s = it.site(id);
            let p = s.p[0];
            s.p[0] = src;
            let mf = src * bar.volume;
            memoire::pousser(&mut s.b, if !p.is_nan() && s.n > 0 && src > p { mf } else { 0.0 });
            memoire::pousser(&mut s.c, if !p.is_nan() && s.n > 0 && src < p { mf } else { 0.0 });
            s.n = 1;
            let nn = s.b.len();
            // (B-051: a length out of reach, 1e30, is `na` like a length longer than the chart)
            if nn < len.saturating_add(1) || len == 0 {
                return Ok(Value::Na);
            }
            let (up, dn): (f64, f64) = (s.b[nn - len..].iter().sum(), s.c[nn - len..].iter().sum());
            f64v(if dn == 0.0 { 100.0 } else { 100.0 - 100.0 / (1.0 + up / dn) })
        }
        "linreg" => {
            let (len, off) = (a.len(1, "length"), a.num_or(2, "offset", 0.0));
            match window(it.site(id), src, len) {
                Some(w) => {
                    let n = len as f64;
                    let (sx, sy) = ((0..len).map(|k| k as f64).sum::<f64>(), w.iter().sum::<f64>());
                    let sxy: f64 = w.iter().enumerate().map(|(k, y)| k as f64 * y).sum();
                    let sxx: f64 = (0..len).map(|k| (k * k) as f64).sum();
                    let slope = (n * sxy - sx * sy) / (n * sxx - sx * sx);
                    let icpt = (sy - slope * sx) / n;
                    f64v(icpt + slope * (n - 1.0 - off))
                }
                None => Value::Na,
            }
        }
        // ta.vwap(source, anchor, stdev_mult) like Pine: sums kept by the call site (each bar's own source), started again
        // when `anchor` is true (without it, on a new UTC day: the session of a 24 h market); with `stdev_mult`,
        // [vwap, upper band, lower band] (B-030: before, the anchor was ignored, older bars took hlc3, no bands)
        "vwap" => {
            let anchor = match a.get(1, "anchor") {
                Some(v) => v.truthy(),
                None => prev.is_none_or(|p| p.time.div_euclid(86_400) != bar.time.div_euclid(86_400)),
            };
            let mult = a.get(2, "stdev_mult").map(Value::num);
            let s = it.site(id);
            if anchor || s.n == 0 {
                (s.p[0], s.p[1], s.p[2], s.n) = (0.0, 0.0, 0.0, 1);
            }
            if src.is_finite() && bar.volume.is_finite() {
                s.p[0] += src * bar.volume;
                s.p[1] += bar.volume;
                s.p[2] += src * src * bar.volume;
            }
            let (sw, sv, sw2) = (s.p[0], s.p[1], s.p[2]);
            let v = if sv > 0.0 { sw / sv } else { f64::NAN };
            match mult {
                None => f64v(v),
                Some(m) => {
                    let sd = (sw2 / sv - v * v).max(0.0).sqrt();
                    tuple(vec![v, v + sd * m, v - sd * m])
                }
            }
        }
        "median" | "percentrank" | "range" => {
            let len = a.len(1, "length");
            match window(it.site(id), src, len) {
                Some(w) => match f {
                    "median" => {
                        let mut v = w.to_vec();
                        v.sort_by(f64::total_cmp);
                        f64v(if len % 2 == 1 { v[len / 2] } else { (v[len / 2 - 1] + v[len / 2]) / 2.0 })
                    }
                    "range" => f64v(w.iter().copied().fold(f64::MIN, f64::max) - w.iter().copied().fold(f64::MAX, f64::min)),
                    _ => {
                        let below = w[..len - 1].iter().filter(|x| **x <= src).count();
                        f64v(100.0 * below as f64 / (len - 1).max(1) as f64)
                    }
                },
                None => Value::Na,
            }
        }
        "dmi" => {
            let (dil, adxl) = (a.len(0, "diLength"), a.len(1, "adxSmoothing"));
            let tr = true_range(it);
            let (up, dn) = prev.map_or((f64::NAN, f64::NAN), |p| (bar.high - p.high, p.low - bar.low));
            let pdm = if up > dn && up > 0.0 { up } else { 0.0 };
            let mdm = if dn > up && dn > 0.0 { dn } else { 0.0 };
            let s = it.site(id);
            if s.s.is_empty() {
                s.s = vec![Smooth::rma(dil), Smooth::rma(dil), Smooth::rma(dil), Smooth::rma(adxl)];
            }
            if prev.is_none() {
                return Ok(tuple(vec![f64::NAN; 3]));
            }
            let t = s.s[0].next(tr);
            let (p, m) = (100.0 * s.s[1].next(pdm) / t, 100.0 * s.s[2].next(mdm) / t);
            let sum = p + m;
            let dx = if sum.is_finite() {
                100.0 * (p - m).abs() / if sum == 0.0 { 1.0 } else { sum }
            } else {
                f64::NAN
            };
            tuple(vec![p, m, s.s[3].next(dx)])
        }
        "supertrend" => {
            // [supertrend, direction] ; direction −1 = haussier (comme Pine)
            let (factor, len) = (a.num(0, "factor"), a.len(1, "atrPeriod"));
            let tr = true_range(it);
            let hl2 = (bar.high + bar.low) / 2.0;
            let pc = prev.map_or(f64::NAN, |p| p.close);
            let s = it.site(id);
            if s.s.is_empty() {
                memoire::pousser(&mut s.s, Smooth::rma(len));
                s.p = [f64::NAN; 8];
            }
            let atr = s.s[0].next(tr);
            let (mut up, mut dn) = (hl2 + factor * atr, hl2 - factor * atr);
            let (pu, pd, pst, pdir) = (s.p[0], s.p[1], s.p[2], s.p[3]);
            if !pd.is_nan() && !(dn > pd || pc < pd) {
                dn = pd;
            }
            if !pu.is_nan() && !(up < pu || pc > pu) {
                up = pu;
            }
            let dir = if pst.is_nan() || atr.is_nan() {
                1.0
            } else if pst == pu {
                if bar.close > up {
                    -1.0
                } else {
                    1.0
                }
            } else if bar.close < dn {
                1.0
            } else {
                -1.0
            };
            let _ = pdir;
            let st = if atr.is_nan() {
                f64::NAN
            } else if dir == -1.0 {
                dn
            } else {
                up
            };
            s.p[0] = up;
            s.p[1] = dn;
            s.p[2] = st;
            s.p[3] = dir;
            tuple(vec![st, dir])
        }
        "sar" => {
            // pas à pas comme ta.sar de TradingView (B-092) : même règle que textbook::psar_n, le retournement se lit
            // sur le SAR avant qu'il soit borné par les deux bougies d'avant
            let (start, inc, max) = (a.num(0, "start"), a.num(1, "inc"), a.num(2, "max"));
            let back2 = it.bar_back(2).copied();
            let s = it.site(id);
            // p : 0 sar, 1 extrême, 2 facteur, 3 sous le prix (1/0), 4 lancé
            let Some(p) = prev else {
                return Ok(Value::Na);
            };
            let lance = s.p[4] == 0.0;
            let mut first = lance;
            if lance {
                let below = bar.close > p.close;
                s.p[..5].copy_from_slice(&if below {
                    [p.low, bar.high, start, 1.0, 1.0]
                } else {
                    [p.high, bar.low, start, 0.0, 1.0]
                });
            }
            let (mut sar, mut ext, mut af, mut below) = (s.p[0], s.p[1], s.p[2], s.p[3] == 1.0);
            sar += af * (ext - sar);
            if below && sar > bar.low {
                (first, below, sar, ext, af) = (true, false, bar.high.max(ext), bar.low, start);
            } else if !below && sar < bar.high {
                (first, below, sar, ext, af) = (true, true, bar.low.min(ext), bar.high, start);
            }
            if !first {
                if below && bar.high > ext {
                    (ext, af) = (bar.high, (af + inc).min(max));
                } else if !below && bar.low < ext {
                    (ext, af) = (bar.low, (af + inc).min(max));
                }
            }
            sar = if below { sar.min(p.low) } else { sar.max(p.high) };
            // la 2e bougie d'avant compte dès la 2e bougie du SAR
            if let (false, Some(q)) = (lance, back2) {
                sar = if below { sar.min(q.low) } else { sar.max(q.high) };
            }
            s.p[..4].copy_from_slice(&[sar, ext, af, below as u8 as f64]);
            Value::Num(sar)
        }
        "correlation" => {
            let (y, len) = (a.num(1, "source2"), a.len(2, "length"));
            let s = it.site(id);
            memoire::pousser(&mut s.b, y);
            match window(s, src, len).map(<[f64]>::to_vec) {
                Some(w) => {
                    let v = &s.b[s.b.len() - len..];
                    let n = len as f64;
                    let (mx, my) = (w.iter().sum::<f64>() / n, v.iter().sum::<f64>() / n);
                    let cov: f64 = w.iter().zip(v).map(|(x, y)| (x - mx) * (y - my)).sum();
                    let (sx, sy) = (
                        w.iter().map(|x| (x - mx).powi(2)).sum::<f64>().sqrt(),
                        v.iter().map(|y| (y - my).powi(2)).sum::<f64>().sqrt(),
                    );
                    f64v(cov / (sx * sy))
                }
                None => Value::Na,
            }
        }
        f => return it.err(line, format!("ta.{f} : pas encore dans Argos Script")),
    })
}

fn array(it: &mut Interp, f: &str, a: &A, gen: Option<&str>, line: u32) -> Result<Value, Error> {
    let _ = gen;
    // créations
    match f {
        "new" | "new_float" | "new_int" | "new_bool" | "new_string" | "new_color" | "new_label" | "new_line" | "new_box" => {
            // (B-049: the size is compared as a number, before it is asked from the computer: `array.new_float(1e12)`
            // stopped the whole app)
            let size = a.num_or(0, "size", 0.0).max(0.0);
            if size > it.max_elements as f64 {
                return it.err(line, trop_d_elements(it, &format!("array.{f}")));
            }
            let init = a.val(1, "initial_value");
            return Ok(Value::arr(vec![init; size as usize]));
        }
        "from" => return Ok(Value::arr(a.v.iter().map(|x| x.1.clone()).collect())),
        _ => {}
    }
    let Some(arr) = arr_of(&a.val(0, "id")) else {
        if a.val(0, "id").is_na() {
            return it.err(line, format!("array.{f} sur un tableau na (pas encore créé)"));
        }
        return it.err(line, format!("array.{f} : le 1er argument doit être un tableau"));
    };
    let idx = |v: &Vec<Value>, k: f64| -> Option<usize> {
        let n = v.len() as i64;
        let k = k as i64;
        let k = if k < 0 { n + k } else { k };
        (k >= 0 && k < n).then_some(k as usize)
    };
    let nums = |v: &Vec<Value>| -> Vec<f64> { v.iter().map(Value::num).filter(|x| !x.is_nan()).collect() };
    let x1 = a.val(1, "value");
    let out = match f {
        "size" => Value::Num(arr.borrow().len() as f64),
        "push" | "unshift" | "insert" if arr.borrow().len() >= it.max_elements => {
            return it.err(line, trop_d_elements(it, &format!("array.{f}")));
        }
        "push" => {
            memoire::pousser(&mut arr.borrow_mut(), x1);
            Value::Na
        }
        "unshift" => {
            let mut v = arr.borrow_mut();
            let avant = v.capacity();
            v.insert(0, x1);
            memoire::grandi::<Value>(avant, v.capacity());
            Value::Na
        }
        "pop" => arr.borrow_mut().pop().unwrap_or(Value::Na),
        "shift" => {
            let mut v = arr.borrow_mut();
            if v.is_empty() {
                Value::Na
            } else {
                v.remove(0)
            }
        }
        "get" | "first" | "last" => {
            let v = arr.borrow();
            let k = match f {
                "first" => 0.0,
                "last" => -1.0,
                _ => a.num(1, "index"),
            };
            match idx(&v, k) {
                Some(k) => v[k].clone(),
                None => return it.err(line, format!("array.{f} : index {k} hors du tableau (taille {})", v.len())),
            }
        }
        "set" => {
            let mut v = arr.borrow_mut();
            let k = a.num(1, "index");
            match idx(&v, k) {
                Some(k) => v[k] = a.val(2, "value"),
                None => return it.err(line, format!("array.set : index {k} hors du tableau (taille {})", v.len())),
            }
            Value::Na
        }
        "insert" => {
            let mut v = arr.borrow_mut();
            let k = (a.num(1, "index").max(0.0) as usize).min(v.len());
            let avant = v.capacity();
            v.insert(k, a.val(2, "value"));
            memoire::grandi::<Value>(avant, v.capacity());
            Value::Na
        }
        "remove" => {
            let mut v = arr.borrow_mut();
            match idx(&v, a.num(1, "index")) {
                Some(k) => v.remove(k),
                None => Value::Na,
            }
        }
        "clear" => {
            arr.borrow_mut().clear();
            Value::Na
        }
        "fill" => {
            let val = a.val(1, "value");
            arr.borrow_mut().iter_mut().for_each(|x| *x = val.clone());
            Value::Na
        }
        "includes" => Value::Bool(arr.borrow().iter().any(|x| x.same(&x1))),
        "indexof" => Value::Num(arr.borrow().iter().position(|x| x.same(&x1)).map_or(-1.0, |k| k as f64)),
        "lastindexof" => Value::Num(arr.borrow().iter().rposition(|x| x.same(&x1)).map_or(-1.0, |k| k as f64)),
        "sum" => Value::from_f64(nums(&arr.borrow()).iter().sum()),
        "avg" => {
            let v = nums(&arr.borrow());
            Value::from_f64(if v.is_empty() { f64::NAN } else { v.iter().sum::<f64>() / v.len() as f64 })
        }
        "max" | "min" => {
            let v = nums(&arr.borrow());
            Value::from_f64(if v.is_empty() {
                f64::NAN
            } else if f == "max" {
                v.iter().copied().fold(f64::MIN, f64::max)
            } else {
                v.iter().copied().fold(f64::MAX, f64::min)
            })
        }
        "range" => {
            let v = nums(&arr.borrow());
            Value::from_f64(v.iter().copied().fold(f64::MIN, f64::max) - v.iter().copied().fold(f64::MAX, f64::min))
        }
        "median" => {
            let mut v = nums(&arr.borrow());
            v.sort_by(f64::total_cmp);
            let n = v.len();
            Value::from_f64(if n == 0 {
                f64::NAN
            } else if n % 2 == 1 {
                v[n / 2]
            } else {
                (v[n / 2 - 1] + v[n / 2]) / 2.0
            })
        }
        "stdev" | "variance" => {
            let v = nums(&arr.borrow());
            let n = v.len() as f64;
            let m = v.iter().sum::<f64>() / n;
            let var = v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / n;
            Value::from_f64(if f == "stdev" { var.sqrt() } else { var })
        }
        "sort" => {
            let desc = a.num_or(1, "order", 0.0) == 1.0;
            arr.borrow_mut().sort_by(|x, y| {
                let o = match (x, y) {
                    (Value::Str(p), Value::Str(q)) => p.cmp(q),
                    _ => x.num().total_cmp(&y.num()),
                };
                if desc {
                    o.reverse()
                } else {
                    o
                }
            });
            Value::Na
        }
        "reverse" => {
            arr.borrow_mut().reverse();
            Value::Na
        }
        "copy" => Value::arr(arr.borrow().clone()),
        "slice" => {
            let v = arr.borrow();
            let (s, e) = (
                (a.num(1, "index_from").max(0.0) as usize).min(v.len()),
                (a.num(2, "index_to").max(0.0) as usize).min(v.len()),
            );
            Value::arr(v[s..e.max(s)].to_vec())
        }
        "concat" => {
            if let Some(o) = arr_of(&a.val(1, "id2")) {
                let extra = o.borrow().clone();
                if arr.borrow().len() + extra.len() > it.max_elements {
                    return it.err(line, trop_d_elements(it, "array.concat"));
                }
                let mut v = arr.borrow_mut();
                let avant = v.capacity();
                v.extend(extra);
                memoire::grandi::<Value>(avant, v.capacity());
            }
            Value::Arr(arr)
        }
        "join" => {
            // (B-049: written piece by piece up to the limit — the texts of all the elements were built first)
            let Some(s) = Value::joindre(&arr.borrow(), &a.text(1, "separator")) else {
                return it.err(line, trop_de_texte("array.join"));
            };
            valeur_texte(it, line, "array.join", s.len(), || s)?
        }
        "abs" => Value::arr(arr.borrow().iter().map(|x| Value::from_f64(x.num().abs())).collect()),
        // extras Argos : lambdas
        "map" | "filter" | "some" | "every" | "count" | "find" | "reduce" | "foreach" => {
            let l = match a.val(1, "f") {
                Value::Lambda(l, p) if p == it.passage => l,
                // (B-076: made by a sub-run of `request.security`, its index named the chart's lambda of that rank)
                Value::Lambda(..) => {
                    return it.err(
                        line,
                        format!("array.{f} : cette lambda a été faite sur une autre unité de temps (request.security), elle n'existe pas ici"),
                    );
                }
                _ => return it.err(line, format!("array.{f} : il faut une lambda, ex. (x) => x * 2")),
            };
            let items = arr.borrow().clone();
            // (B-062: this copy is held while the lambda runs, and no variable of the script shows it)
            let _pret = memoire::Pret::de(items.capacity(), memoire::V);
            match f {
                "map" => {
                    // (B-062: the results go into the new array one by one, where the memory count sees them — a
                    // local list of 100 000 fresh texts was seen by nothing)
                    let out = Value::tableau(Vec::with_capacity(items.len()));
                    for (k, x) in items.into_iter().enumerate() {
                        let y = it.call_lambda(l, &[x, Value::Num(k as f64)], line)?;
                        // (made with a place for each result: nothing more to tell)
                        out.borrow_mut().push(y);
                    }
                    Value::Arr(out)
                }
                "filter" => {
                    let out = Value::tableau(Vec::new());
                    for (k, x) in items.into_iter().enumerate() {
                        if it.call_lambda(l, &[x.clone(), Value::Num(k as f64)], line)?.truthy() {
                            memoire::pousser(&mut out.borrow_mut(), x);
                        }
                    }
                    Value::Arr(out)
                }
                "reduce" => {
                    let mut acc = a.val(2, "initial");
                    for x in items {
                        acc = it.call_lambda(l, &[acc, x], line)?;
                    }
                    acc
                }
                "foreach" => {
                    for (k, x) in items.into_iter().enumerate() {
                        it.call_lambda(l, &[x, Value::Num(k as f64)], line)?;
                    }
                    Value::Na
                }
                _ => {
                    let mut hits = 0usize;
                    let mut found = Value::Na;
                    let total = items.len();
                    for x in items {
                        if it.call_lambda(l, std::slice::from_ref(&x), line)?.truthy() {
                            hits += 1;
                            if found.is_na() {
                                found = x;
                            }
                            if f == "find" || f == "some" {
                                break;
                            }
                        }
                    }
                    match f {
                        "some" => Value::Bool(hits > 0),
                        "every" => Value::Bool(hits == total),
                        "count" => Value::Num(hits as f64),
                        _ => found,
                    }
                }
            }
        }
        f => return it.err(line, format!("array.{f} : pas encore dans Argos Script")),
    };
    Ok(out)
}

/// The most pairs a map holds, as on TradingView.
const MAX_PAIRES: usize = 50_000;

fn map(f: &str, a: &A, it: &mut Interp, line: u32) -> Result<Value, Error> {
    if f == "new" {
        return Ok(Value::map(vec![]));
    }
    let Value::Map(m) = a.val(0, "id") else {
        return it.err(line, format!("map.{f} : le 1er argument doit être une map"));
    };
    let key = a.val(1, "key");
    let pos = m.borrow().iter().position(|(k, _)| k.same(&key));
    Ok(match f {
        "put" => {
            let v = a.val(2, "value");
            let mut mm = m.borrow_mut();
            match pos {
                Some(p) => std::mem::replace(&mut mm[p].1, v),
                // (B-062: a map had no limit at all)
                None if mm.len() >= MAX_PAIRES => return it.err(line, "map.put : 50 000 paires au plus, comme sur TradingView"),
                None => {
                    memoire::pousser(&mut mm, (key, v));
                    Value::Na
                }
            }
        }
        "get" => pos.map_or(Value::Na, |p| m.borrow()[p].1.clone()),
        "contains" => Value::Bool(pos.is_some()),
        "remove" => pos.map_or(Value::Na, |p| m.borrow_mut().remove(p).1),
        "size" => Value::Num(m.borrow().len() as f64),
        "keys" => Value::arr(m.borrow().iter().map(|x| x.0.clone()).collect()),
        "values" => Value::arr(m.borrow().iter().map(|x| x.1.clone()).collect()),
        "clear" => {
            m.borrow_mut().clear();
            Value::Na
        }
        "copy" => Value::map(m.borrow().clone()),
        f => return it.err(line, format!("map.{f} : pas encore dans Argos Script")),
    })
}

/// The most elements a matrix or an array holds, as on TradingView: `matrix.new`, `add_row` / `add_col` that make a
/// matrix grow; `array.new*`, `push`, `unshift`, `insert` and `concat` (B-049). On a chart of more than 20 000 bars
/// the limit is 5 elements a bar (`Interp::max_elements`).
pub(crate) const MAX_ELEMENTS: usize = 100_000;

/// The error of a collection asked past its limit, named by the function that asked (`matrix.add_row`…).
fn trop_d_elements(it: &Interp, f: &str) -> String {
    if it.max_elements == MAX_ELEMENTS {
        return format!("{f} : 100 000 éléments au plus, comme sur TradingView");
    }
    // « 1 000 000 »
    let n = it.max_elements.to_string();
    let milliers: Vec<&str> = n.as_bytes().rchunks(3).rev().filter_map(|c| std::str::from_utf8(c).ok()).collect();
    format!("{f} : {} éléments au plus (5 par bougie de la chart)", milliers.join(" "))
}

/// 🔢 `matrix.*`, as in Pine: a matrix of `rows` × `columns` values (Smart Swing VWAP ranks its anchors in one).
/// An index outside the matrix is an error, as on TradingView.
fn matrix(f: &str, a: &A, it: &mut Interp, line: u32) -> Result<Value, Error> {
    use super::value::Matrix;
    let mat = Value::mat;
    if f == "new" {
        // (B-036: each side and their product compared as numbers, before the cast — 1e30 rows of 0 column passed,
        // and the text of that matrix stopped the app)
        let (rows, cols) = (a.num_or(0, "rows", 0.0).max(0.0), a.num_or(1, "columns", 0.0).max(0.0));
        let max = it.max_elements as f64;
        if rows > max || cols > max || rows * cols > max {
            return it.err(line, trop_d_elements(it, "matrix.new"));
        }
        let (rows, cols) = (rows as usize, cols as usize);
        return Ok(mat(Matrix {
            rows,
            cols,
            v: vec![a.val(2, "initial_value"); rows * cols],
        }));
    }
    let Value::Mat(m) = a.val(0, "id") else {
        if a.val(0, "id").is_na() {
            return it.err(line, format!("matrix.{f} sur une matrice na (pas encore créée)"));
        }
        return it.err(line, format!("matrix.{f} : le 1er argument doit être une matrice"));
    };
    let (rows, cols) = {
        let m = m.borrow();
        (m.rows, m.cols)
    };
    let hors = |k: f64, n: usize| !(k >= 0.0 && (k as usize) < n);
    let r = a.num_or(1, "row", 0.0);
    let nums = |v: &[Value]| -> Vec<f64> { v.iter().map(Value::num).filter(|x| !x.is_nan()).collect() };
    Ok(match f {
        "rows" => Value::Num(rows as f64),
        "columns" => Value::Num(cols as f64),
        "elements_count" => Value::Num((rows * cols) as f64),
        "is_square" => Value::Bool(rows == cols),
        "get" | "set" => {
            let c = a.num(2, "column");
            if hors(r, rows) || hors(c, cols) {
                return it.err(line, format!("matrix.{f} : la case ({r}, {c}) est hors de la matrice ({rows} × {cols})"));
            }
            let k = r as usize * cols + c as usize;
            if f == "get" {
                m.borrow().v[k].clone()
            } else {
                m.borrow_mut().v[k] = a.val(3, "value");
                Value::Na
            }
        }
        "row" => {
            if hors(r, rows) {
                return it.err(line, format!("matrix.row : la ligne {r} est hors de la matrice ({rows} lignes)"));
            }
            Value::arr(m.borrow().row(r as usize).to_vec())
        }
        "col" => {
            let c = a.num_or(1, "column", 0.0);
            if hors(c, cols) {
                return it.err(line, format!("matrix.col : la colonne {c} est hors de la matrice ({cols} colonnes)"));
            }
            let m = m.borrow();
            Value::arr((0..rows).map(|r| m.v[r * cols + c as usize].clone()).collect())
        }
        "copy" => mat(m.borrow().clone()),
        "transpose" => {
            let m = m.borrow();
            let v = (0..cols).flat_map(|c| (0..rows).map(move |r| (r, c))).map(|(r, c)| m.v[r * cols + c].clone());
            mat(Matrix {
                rows: cols,
                cols: rows,
                v: v.collect(),
            })
        }
        "reverse" => {
            m.borrow_mut().v.reverse();
            Value::Na
        }
        // sort(id, column = 0, order = order.ascending): the rows, by the values of one column
        "sort" => {
            let c = a.num_or(1, "column", 0.0);
            if rows > 0 && hors(c, cols) {
                return it.err(line, format!("matrix.sort : la colonne {c} est hors de la matrice ({cols} colonnes)"));
            }
            let desc = a.num_or(2, "order", 0.0) == 1.0;
            let mut mm = m.borrow_mut();
            let mut lignes: Vec<Vec<Value>> = (0..rows).map(|r| mm.row(r).to_vec()).collect();
            lignes.sort_by(|x, y| {
                let (p, q) = (&x[c as usize], &y[c as usize]);
                let o = match (p, q) {
                    (Value::Str(p), Value::Str(q)) => p.cmp(q),
                    _ => p.num().total_cmp(&q.num()),
                };
                if desc {
                    o.reverse()
                } else {
                    o
                }
            });
            mm.v = lignes.concat();
            Value::Na
        }
        // fill(id, value, from_row = 0, to_row = rows, from_column = 0, to_column = columns)
        "fill" => {
            let v = a.val(1, "value");
            let (r0, r1) = (
                a.num_or(2, "from_row", 0.0).max(0.0) as usize,
                (a.num_or(3, "to_row", rows as f64).max(0.0) as usize).min(rows),
            );
            let (c0, c1) = (
                a.num_or(4, "from_column", 0.0).max(0.0) as usize,
                (a.num_or(5, "to_column", cols as f64).max(0.0) as usize).min(cols),
            );
            let mut mm = m.borrow_mut();
            for r in r0..r1 {
                for c in c0..c1 {
                    mm.v[r * cols + c] = v.clone();
                }
            }
            Value::Na
        }
        // add_row(id, row = rows, array_id = na) / add_col(id, column = columns, array_id = na)
        "add_row" | "add_col" => {
            let ligne = f == "add_row";
            let vide = rows * cols == 0;
            let (n, autre) = match (vide, ligne) {
                (true, _) => (0, 0),
                (false, true) => (rows, cols),
                (false, false) => (cols, rows),
            };
            let k = a.num_or(1, if ligne { "row" } else { "column" }, n as f64);
            if !(k >= 0.0 && k as usize <= n) {
                return it.err(line, format!("matrix.{f} : la place {k} est hors de la matrice"));
            }
            let k = k as usize;
            let vals = match a.val(2, "array_id") {
                Value::Arr(v) => v.borrow().clone(),
                _ => vec![Value::Na; autre.max(1)],
            };
            if !vide && vals.len() != autre {
                return it.err(line, format!("matrix.{f} : le tableau a {} valeurs, il en faut {autre}", vals.len()));
            }
            let mut mm = m.borrow_mut();
            let w = vals.len();
            // (B-036: the limit of `matrix.new`, for a matrix that grows too)
            if (n + 1).saturating_mul(w) > it.max_elements {
                return it.err(line, trop_d_elements(it, &format!("matrix.{f}")));
            }
            // (B-062: a row is added in place, a column makes the whole matrix again)
            let avant = if ligne { mm.v.capacity() } else { 0 };
            if ligne {
                mm.v.splice(k * w..k * w, vals);
                (mm.rows, mm.cols) = (n + 1, w);
            } else {
                let old = std::mem::take(&mut mm.v);
                for (r, x) in vals.into_iter().enumerate() {
                    mm.v.extend_from_slice(&old[r * n..r * n + k]);
                    mm.v.push(x);
                    mm.v.extend_from_slice(&old[r * n + k..(r + 1) * n]);
                }
                (mm.rows, mm.cols) = (w, n + 1);
            }
            memoire::grandi::<Value>(avant, mm.v.capacity());
            Value::Na
        }
        // remove_row(id, row = rows - 1) / remove_col(id, column = columns - 1): the removed values, as an array
        "remove_row" | "remove_col" => {
            let ligne = f == "remove_row";
            let n = if ligne { rows } else { cols };
            let k = a.num_or(1, if ligne { "row" } else { "column" }, n as f64 - 1.0);
            if hors(k, n) {
                return it.err(line, format!("matrix.{f} : la place {k} est hors de la matrice"));
            }
            let k = k as usize;
            let mut mm = m.borrow_mut();
            let mut lignes: Vec<Vec<Value>> = (0..rows).map(|r| mm.row(r).to_vec()).collect();
            let parti = if ligne {
                lignes.remove(k)
            } else {
                lignes.iter_mut().map(|l| l.remove(k)).collect()
            };
            mm.rows = lignes.len();
            mm.cols = if ligne { cols } else { cols - 1 };
            if mm.rows == 0 || mm.cols == 0 {
                (mm.rows, mm.cols) = (0, 0);
            }
            mm.v = lignes.concat();
            Value::arr(parti)
        }
        "swap_rows" | "swap_columns" => {
            let ligne = f == "swap_rows";
            let n = if ligne { rows } else { cols };
            let (p, q) = (
                a.num(1, if ligne { "row1" } else { "column1" }),
                a.num(2, if ligne { "row2" } else { "column2" }),
            );
            if hors(p, n) || hors(q, n) {
                return it.err(line, format!("matrix.{f} : ({p}, {q}) hors de la matrice"));
            }
            let mut mm = m.borrow_mut();
            for i in 0..if ligne { cols } else { rows } {
                let (x, y) = if ligne {
                    (p as usize * cols + i, q as usize * cols + i)
                } else {
                    (i * cols + p as usize, i * cols + q as usize)
                };
                mm.v.swap(x, y);
            }
            Value::Na
        }
        "max" | "min" | "avg" => {
            let x = nums(&m.borrow().v);
            if x.is_empty() {
                Value::Na
            } else {
                Value::Num(match f {
                    "max" => x.iter().copied().fold(f64::MIN, f64::max),
                    "min" => x.iter().copied().fold(f64::MAX, f64::min),
                    _ => x.iter().sum::<f64>() / x.len() as f64,
                })
            }
        }
        f => return it.err(line, format!("matrix.{f} : pas encore dans Argos Script")),
    })
}

/// Nombre maximum de dessins gardés par sorte (comme `max_labels_count` de Pine, au maximum).
pub(crate) const MAX_DRAWINGS: usize = 500;
/// Polylines kept when `max_polylines_count` is not given (Pine's default), and the most it can ask for.
pub(crate) const MAX_POLYLINES: usize = 50;
const MAX_POLYLINES_PINE: usize = 100;

/// 📍 A `chart.point`: an object with `time` (ms), `index` (bar) and `price`, like Pine's.
fn point(time: f64, index: f64, price: f64) -> Value {
    Value::obj(super::value::Obj {
        ty: "chart.point".into(),
        fields: vec![
            ("time".into(), Value::from_f64(time)),
            ("index".into(), Value::from_f64(index)),
            ("price".into(), Value::from_f64(price)),
        ],
    })
}

/// The (x, y) of a `chart.point` for a drawing: its time with `xloc.bar_time`, else its bar index. `None`: not a point.
fn point_xy(v: &Value, xtime: bool) -> Option<(f64, f64)> {
    let Value::Obj(o) = v else { return None };
    let o = o.borrow();
    if o.ty != "chart.point" {
        return None;
    }
    let field = |n: &str| o.fields.iter().find(|(k, _)| k == n).map_or(f64::NAN, |(_, v)| v.num());
    Some((field(if xtime { "time" } else { "index" }), field("price")))
}

/// `chart.point.new(time, index, price)`, `from_index(index, price)`, `from_time(time, price)`, `now(price)`, `copy(id)`.
fn chart_point(it: &mut Interp, f: &str, a: &A, line: u32) -> Result<Value, Error> {
    let nan = f64::NAN;
    Ok(match f {
        "new" => point(a.num(0, "time"), a.num(1, "index"), a.num(2, "price")),
        "from_index" => point(nan, a.num(0, "index"), a.num(1, "price")),
        "from_time" => point(a.num(0, "time"), nan, a.num(1, "price")),
        "now" => {
            let b = it.bars[it.i];
            point(b.time as f64 * 1000.0, it.i as f64, a.num_or(0, "price", b.close))
        }
        "copy" => match a.val(0, "id") {
            Value::Obj(o) => Value::obj(o.borrow().clone()),
            v => v,
        },
        f => return it.err(line, format!("chart.point.{f} : pas encore dans Argos Script")),
    })
}

/// A deleted polyline keeps no point (B-032), as TradingView frees it: its place stays in the list (a handle is a
/// place in it), its points go. Smart Swing VWAP deletes its curve and draws it again with all its points at every
/// bar: kept, they grew with the square of the curve (935 000 points for 3 000 bars).
fn effacer_polyline(p: &mut super::Polyline) {
    p.deleted = true;
    p.points = Vec::new();
}

/// 〰️ `polyline.new(points, curved, closed, xloc, line_color, fill_color, line_style, line_width, force_overlay)`,
/// `polyline.delete(id)` and `polyline.all`.
fn polyline(it: &mut Interp, f: &str, a: &A, line: u32) -> Result<Value, Error> {
    match f {
        "new" => {
            let xtime = a.text(3, "xloc") == "bar_time";
            let points: Vec<(f64, f64)> = match a.val(0, "points") {
                Value::Arr(v) => v.borrow().iter().filter_map(|p| point_xy(p, xtime)).collect(),
                _ => vec![],
            };
            memoire::par_bougie(points.len(), std::mem::size_of::<(f64, f64)>());
            it.out.polylines.push(super::Polyline {
                points,
                xtime,
                curved: a.flag(1, "curved", false),
                closed: a.flag(2, "closed", false),
                color: a.color_or(4, "line_color", PLOT_BLUE),
                fill: a.color(5, "fill_color"),
                style: a.text(6, "line_style"),
                width: a.num_or(7, "line_width", 1.0),
                deleted: false,
                force_overlay: a.flag(8, "force_overlay", false),
            });
            // too many: the oldest ones go, like TradingView (B-064: without the list being read again)
            let id = it.ids_polylines.nouveau(0);
            let max = it.max_dessins[3];
            let Interp { ids_polylines, out, .. } = it;
            while let Some(place) = ids_polylines.plus_vieux(0, max, |p| !out.polylines[p].deleted) {
                effacer_polyline(&mut out.polylines[place]);
                ids_polylines.efface(0);
            }
            ids_polylines.tasser(&mut out.polylines, |p| (!p.deleted).then_some(0));
            Ok(Value::Handle(HandleKind::Polyline, id, it.passage))
        }
        "delete" => {
            if let Value::Handle(HandleKind::Polyline, k, _) = a.val(0, "id") {
                let place = it.ids_polylines.place(k);
                if let Some(p) = place.and_then(|p| it.out.polylines.get_mut(p)).filter(|p| !p.deleted) {
                    effacer_polyline(p);
                    it.ids_polylines.efface(0);
                    it.ids_polylines.tasser(&mut it.out.polylines, |p| (!p.deleted).then_some(0));
                }
            }
            Ok(Value::Na)
        }
        f => it.err(line, format!("polyline.{f} : pas encore dans Argos Script")),
    }
}

/// `line.new(first_point, second_point, …)`, `label.new(point, …)`, `box.new(top_left, bottom_right, …)`: the same
/// call with x / y numbers in place of the points (the `xloc` decides between their time and their bar index).
fn points_en_xy<'x>(kind: &str, a: &A<'x>) -> Option<A<'x>> {
    let (n_points, xloc_at, names): (usize, usize, &[&str]) = match kind {
        "line" => (2, 2, &["first_point", "second_point"]),
        "box" => (2, 6, &["top_left", "bottom_right"]),
        _ => (1, 2, &["point"]),
    };
    let is_point = |v: &Value| point_xy(v, false).is_some();
    let first = a.get(0, names[0])?;
    if !is_point(first) {
        return None;
    }
    let xtime = a.text(xloc_at, "xloc") == "bar_time";
    let mut v = vec![];
    for (k, n) in names.iter().enumerate().take(n_points) {
        let (x, y) = point_xy(&a.val(k, n), xtime).unwrap_or((f64::NAN, f64::NAN));
        v.push((None, Value::from_f64(x)));
        v.push((None, Value::from_f64(y)));
    }
    // (a box gives (left, top, right, bottom): its two points are already in that order)
    let positional: Vec<&(Option<&'x str>, Value)> = a.v.iter().filter(|(n, _)| n.is_none()).collect();
    v.extend(positional.iter().skip(n_points).map(|x| (*x).clone()));
    v.extend(a.v.iter().filter(|(n, _)| n.is_some_and(|n| !names.contains(&n))).cloned());
    Some(A { v })
}

/// The sort of a label, a line or a box in the ids of the drawings (B-064), which is also its place in `max_dessins`.
fn sorte_de(kind: &str) -> usize {
    match kind {
        "label" => 0,
        "line" => 1,
        _ => 2,
    }
}

/// A deleted drawing keeps no text (B-064), as a deleted polyline keeps no point: it stays in the list, empty, until
/// the list is packed.
fn effacer_dessin(d: &mut Drawing) {
    d.deleted = true;
    (d.text, d.style, d.extend, d.size) = Default::default();
}

/// The live drawing `id`; `None` once it is deleted, still in the list or not.
fn dessin<'i>(it: &'i mut Interp, id: usize) -> Option<&'i mut Drawing> {
    let place = it.ids_dessins.place(id)?;
    it.out.drawings.get_mut(place).filter(|d| !d.deleted)
}

/// The drawing just put at the end of the list takes its id (B-064). Past the most its kind keeps, the oldest go;
/// the deleted ones leave the list when they are too many.
fn poser_dessin(it: &mut Interp) -> usize {
    let sorte = sorte_de(it.out.drawings.last().map_or("label", |d| d.kind));
    let id = it.ids_dessins.nouveau(sorte);
    let max = it.max_dessins[sorte];
    let Interp { ids_dessins, out, .. } = it;
    while let Some(place) = ids_dessins.plus_vieux(sorte, max, |p| !out.drawings[p].deleted) {
        effacer_dessin(&mut out.drawings[place]);
        ids_dessins.efface(sorte);
    }
    ids_dessins.tasser(&mut out.drawings, |d| (!d.deleted).then(|| sorte_de(d.kind)));
    id
}

fn drawing(it: &mut Interp, name: &str, a: &A) -> Value {
    let (kind, f) = name.split_once('.').unwrap_or(("label", ""));
    if f == "new" {
        if let Some(xy) = points_en_xy(kind, a) {
            return drawing(it, name, &xy);
        }
    }
    // set_first_point / set_second_point / set_point / set_top_left_point / set_bottom_right_point: the point's x / y
    if let Some(second) = match f {
        "set_first_point" | "set_point" | "set_top_left_point" => Some(false),
        "set_second_point" | "set_bottom_right_point" => Some(true),
        _ => None,
    } {
        if let Value::Handle(HandleKind::Drawing, k, _) = a.val(0, "id") {
            if let Some(d) = dessin(it, k) {
                if let Some((x, y)) = point_xy(&a.val(1, "point"), d.xtime) {
                    if second {
                        (d.x2, d.y2) = (x, y);
                    } else {
                        (d.x1, d.y1) = (x, y);
                    }
                }
            }
        }
        return Value::Na;
    }
    let kind: &'static str = match kind {
        "line" => "line",
        "box" => "box",
        _ => "label",
    };
    if f == "new" {
        // (xloc: label.new's 4th parameter, line.new's 5th, box.new's 9th)
        let xtime = a.text(
            match kind {
                "label" => 3,
                "line" => 4,
                _ => 8,
            },
            "xloc",
        ) == "bar_time";
        let mut d = Drawing {
            kind,
            x1: f64::NAN,
            y1: f64::NAN,
            x2: f64::NAN,
            y2: f64::NAN,
            xtime,
            text: String::new(),
            color: PLOT_BLUE,
            bg: PLOT_BLUE | (80 << 24),
            text_color: 0xFFFFFF,
            style: String::new(),
            width: 1.0,
            extend: "none".into(),
            size: "normal".into(),
            deleted: false,
            force_overlay: a.flag(usize::MAX, "force_overlay", false),
        };
        match kind {
            "label" => {
                (d.x1, d.y1) = (a.num(0, "x"), a.num(1, "y"));
                d.text = a.text(2, "text");
                d.color = a.color_or(5, "color", PLOT_BLUE);
                d.style = a.text(6, "style");
                d.text_color = a.color_or(7, "textcolor", 0xFFFFFF);
                d.size = a.text(8, "size");
            }
            "line" => {
                (d.x1, d.y1, d.x2, d.y2) = (a.num(0, "x1"), a.num(1, "y1"), a.num(2, "x2"), a.num(3, "y2"));
                d.extend = a.text(5, "extend");
                d.color = a.color_or(6, "color", PLOT_BLUE);
                d.style = a.text(7, "style");
                d.width = a.num_or(8, "width", 1.0);
            }
            _ => {
                (d.x1, d.y1, d.x2, d.y2) = (a.num(0, "left"), a.num(1, "top"), a.num(2, "right"), a.num(3, "bottom"));
                d.color = a.color_or(4, "border_color", PLOT_BLUE);
                d.width = a.num_or(5, "border_width", 1.0);
                d.style = a.text(6, "border_style");
                d.extend = a.text(7, "extend");
                d.bg = a.color_or(9, "bgcolor", PLOT_BLUE | (80 << 24));
                d.text = a.text(10, "text");
                d.text_color = a.color_or(12, "text_color", 0xFFFFFF);
            }
        }
        it.out.drawings.push(d);
        let k = poser_dessin(it);
        return Value::Handle(HandleKind::Drawing, k, it.passage);
    }
    let Value::Handle(HandleKind::Drawing, k, _) = a.val(0, "id") else {
        return Value::Na; // na.delete() et compagnie : rien à faire, comme Pine
    };
    // (a deleted drawing is `na` for every function, as in Pine: nothing to read, nothing to change)
    let Some(d) = dessin(it, k) else {
        return Value::Na;
    };
    let v1 = a.val(1, "x");
    match f {
        "delete" => {
            let sorte = sorte_de(d.kind);
            effacer_dessin(d);
            it.ids_dessins.efface(sorte);
            it.ids_dessins.tasser(&mut it.out.drawings, |d| (!d.deleted).then(|| sorte_de(d.kind)));
            Value::Na
        }
        "set_x" | "set_x1" | "set_left" => {
            d.x1 = v1.num();
            Value::Na
        }
        "set_y" | "set_y1" | "set_top" => {
            d.y1 = v1.num();
            Value::Na
        }
        "set_x2" | "set_right" => {
            d.x2 = v1.num();
            Value::Na
        }
        "set_y2" | "set_bottom" => {
            d.y2 = v1.num();
            Value::Na
        }
        "set_xy" | "set_xy1" | "set_lefttop" => {
            (d.x1, d.y1) = (v1.num(), a.num(2, "y"));
            Value::Na
        }
        "set_xy2" | "set_rightbottom" => {
            (d.x2, d.y2) = (v1.num(), a.num(2, "y"));
            Value::Na
        }
        "set_text" => {
            d.text = v1.text();
            Value::Na
        }
        // (`na`: no colour, as in Pine)
        "set_color" | "set_border_color" => {
            d.color = if let Value::Color(c) = v1 { c } else { NO_COLOR };
            Value::Na
        }
        "set_bgcolor" => {
            d.bg = if let Value::Color(c) = v1 { c } else { NO_COLOR };
            Value::Na
        }
        "set_textcolor" | "set_text_color" => {
            d.text_color = if let Value::Color(c) = v1 { c } else { NO_COLOR };
            Value::Na
        }
        "set_style" | "set_border_style" => {
            d.style = v1.text();
            Value::Na
        }
        "set_width" | "set_border_width" => {
            d.width = v1.num();
            Value::Na
        }
        "set_extend" => {
            d.extend = v1.text();
            Value::Na
        }
        "set_size" => {
            d.size = v1.text();
            Value::Na
        }
        "get_x" | "get_x1" | "get_left" => Value::from_f64(d.x1),
        "get_y" | "get_y1" | "get_top" => Value::from_f64(d.y1),
        "get_x2" | "get_right" => Value::from_f64(d.x2),
        "get_y2" | "get_bottom" => Value::from_f64(d.y2),
        "get_text" => Value::str(&d.text),
        "get_price" => {
            // prix de la ligne à la bougie x
            let x = v1.num();
            Value::from_f64(if d.x2 == d.x1 {
                d.y1
            } else {
                d.y1 + (d.y2 - d.y1) * (x - d.x1) / (d.x2 - d.x1)
            })
        }
        "copy" => {
            let c = d.clone();
            // (B-062: the copy keeps its own texts)
            memoire::texte(c.text.capacity() + c.style.capacity() + c.extend.capacity() + c.size.capacity());
            it.out.drawings.push(c);
            // (B-064: a copy counts like a new drawing in the most its kind keeps)
            let k = poser_dessin(it);
            Value::Handle(HandleKind::Drawing, k, it.passage)
        }
        _ => Value::Na,
    }
}
