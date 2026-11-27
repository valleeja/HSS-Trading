//! Tests du langage : les fonctions `ta` donnent les mêmes valeurs que les fonctions natives d'Argos (elles-mêmes
//! vérifiées contre Pine / v1), et la sémantique de Pine (var, historique, mémoire par appel, tuples, tableaux…).

use super::*;
use crate::ta;
use argos_core::Bar;

pub(super) fn bars() -> Vec<Bar> {
    let v: serde_json::Value = serde_json::from_str(include_str!("../../tests/fixtures/hype_hl_50m.json")).unwrap();
    v.as_array()
        .unwrap()
        .iter()
        .map(|b| Bar {
            time: b["time"].as_i64().unwrap(),
            open: b["open"].as_f64().unwrap(),
            high: b["high"].as_f64().unwrap(),
            low: b["low"].as_f64().unwrap(),
            close: b["close"].as_f64().unwrap(),
            volume: b["volume"].as_f64().unwrap(),
        })
        .collect()
}

fn go(src: &str, b: &[Bar]) -> Output {
    run(src, b, 3000, &HashMap::new()).unwrap_or_else(|e| panic!("{e}"))
}

fn same(a: &[f64], b: &[f64]) {
    assert_eq!(a.len(), b.len());
    for (k, (x, y)) in a.iter().zip(b).enumerate() {
        assert!((x.is_nan() && y.is_nan()) || (x - y).abs() < 1e-9 * y.abs().max(1.0), "bougie {k} : {x} ≠ {y}");
    }
}

#[test]
fn ta_comme_les_fonctions_natives() {
    let b = bars();
    let c: Vec<f64> = b.iter().map(|x| x.close).collect();
    let o = go(
        r#"//@version=6
indicator("ta", overlay = true)
plot(ta.sma(close, 20), "sma")
plot(ta.ema(close, 20), "ema")
plot(ta.rma(close, 14), "rma")
plot(ta.wma(close, 9), "wma")
plot(ta.rsi(close, 14), "rsi")
plot(ta.atr(14), "atr")
plot(ta.stdev(close, 20), "stdev")
plot(ta.highest(close, 10), "hh")
plot(ta.lowest(close, 10), "ll")
"#,
        &b,
    );
    assert_eq!(o.title, "ta");
    assert!(o.overlay);
    let p = |t: &str| o.plots.iter().find(|p| p.title == t).unwrap().values.clone();
    same(&p("sma"), &ta::sma(&c, 20));
    same(&p("ema"), &ta::ema(&c, 20));
    same(&p("rma"), &ta::rma(&c, 14));
    same(&p("wma"), &ta::wma(&c, 9));
    same(&p("rsi"), &ta::rsi(&c, 14));
    same(&p("atr"), &ta::atr(&b, 14));
    same(&p("stdev"), &ta::stdev(&c, 20));
    same(&p("hh"), &ta::highest(&c, 10));
    same(&p("ll"), &ta::lowest(&c, 10));
}

#[test]
fn var_historique_et_memoire_par_appel() {
    let b = bars();
    let o = go(
        r#"indicator("sémantique")
var int n = 0
n += 1
x = close - open
plot(n, "n")
plot(x[1], "x1")
plot(close[2], "c2")
f(src) => ta.sma(src, 3)
plot(f(close), "f_close")
plot(f(open), "f_open")
acc = 0.0
for i = 1 to 3
    acc += i
plot(acc, "acc")
lvl = if close > open
    1
else
    -1
plot(lvl, "lvl")
"#,
        &b,
    );
    let p = |t: &str| o.plots.iter().find(|p| p.title == t).unwrap().values.clone();
    assert_eq!(p("n")[..3], [1.0, 2.0, 3.0], "var garde sa valeur");
    let k = 10;
    assert!((p("x1")[k] - (b[k - 1].close - b[k - 1].open)).abs() < 1e-12, "x[1] = x de la bougie d'avant");
    assert!(p("x1")[0].is_nan());
    assert_eq!(p("c2")[k], b[k - 2].close);
    // la même fonction appelée deux fois : deux mémoires différentes
    let c: Vec<f64> = b.iter().map(|x| x.close).collect();
    let op: Vec<f64> = b.iter().map(|x| x.open).collect();
    same(&p("f_close"), &ta::sma(&c, 3));
    same(&p("f_open"), &ta::sma(&op, 3));
    assert_eq!(p("acc")[5], 6.0);
    assert_eq!(p("lvl")[k], if b[k].close > b[k].open { 1.0 } else { -1.0 });
}

#[test]
fn tuples_tableaux_types_lambdas() {
    let b = bars();
    let o = go(
        r#"indicator("structures")
[m, s, h] = ta.macd(close, 12, 26, 9)
plot(h, "hist")
var arr = array.new<float>()
arr.push(close)
if arr.size() > 5
    arr.shift()
plot(arr.size(), "size")
plot(arr.avg(), "avg")
dbl = arr.map((x) => x * 2)
plot(dbl.sum() / arr.sum(), "ratio")
plot(arr.filter((x) => x > 0).size(), "pos")
plot(arr.reduce((acc, x) => acc + 1, 0), "count")
type Zone
    float top = na
    int touches = 0
var Zone z = Zone.new(high)
z.touches += 1
plot(z.touches, "touches")
method grow(Zone zz, float by) =>
    zz.top += by
    zz.top
plot(z.grow(0) == z.top ? 1 : 0, "method")
m2 = map.new<string, float>()
m2.put("a", 1.5)
plot(m2.get("a"), "map")
plot(str.length(str.tostring(42)), "str")
"#,
        &b,
    );
    let p = |t: &str| o.plots.iter().find(|p| p.title == t).unwrap().values.clone();
    let k = 100;
    let c: Vec<f64> = b.iter().map(|x| x.close).collect();
    let (f, s) = (ta::ema(&c, 12), ta::ema(&c, 26));
    let macd: Vec<f64> = f.iter().zip(&s).map(|(a, b)| a - b).collect();
    let sig = ta::ema(&macd, 9);
    assert!((p("hist")[k] - (macd[k] - sig[k])).abs() < 1e-9);
    assert_eq!(p("size")[k], 5.0);
    assert!((p("avg")[k] - c[k - 4..=k].iter().sum::<f64>() / 5.0).abs() < 1e-9);
    assert_eq!(p("ratio")[k], 2.0);
    assert_eq!((p("pos")[k], p("count")[k]), (5.0, 5.0));
    assert_eq!(p("touches")[k], k as f64 + 1.0, "objet var : garde ses champs");
    assert_eq!(p("method")[k], 1.0);
    assert_eq!(p("map")[k], 1.5);
    assert_eq!(p("str")[k], 2.0);
}

#[test]
fn inputs_formes_et_dessins() {
    let b = bars();
    let src = r#"indicator("inputs", overlay = true)
len = input.int(10, "Longueur", minval = 1, group = "Réglages")
src = input.source(close, "Source")
show = input.bool(true, "Flèches")
col = input.color(color.red, "Couleur")
ma = ta.sma(src, len)
plot(ma, "ma", color = col, linewidth = 2)
plotshape(show and ta.crossover(close, ma), "up", shape.triangleup, location.belowbar, color.green)
h = hline(50, "milieu")
if barstate.islast
    label.new(bar_index, high, "fin", style = label.style_label_down)
bgcolor(close > ma ? color.new(color.green, 90) : na)
"#;
    let o = go(src, &b);
    assert_eq!(o.inputs.len(), 4);
    assert_eq!(o.inputs[0].min, Some(1.0));
    assert_eq!(o.inputs[0].group, "Réglages");
    assert_eq!(o.inputs[1].def, InVal::Text("close".into()));
    assert_eq!(o.plots[0].colors[50], Some(0xFF5252));
    assert!(!o.shapes.is_empty() && o.shapes.iter().all(|s| s.loc == Loc::BelowBar && s.shape == "triangleup"));
    assert_eq!(o.hlines.len(), 1);
    assert_eq!(o.drawings.len(), 1, "un seul label (dernière bougie)");
    assert_eq!(o.drawings[0].text, "fin");
    assert!(o.bgcolor.iter().any(|c| c.is_some()));
    // input choisi : longueur 20, source open
    let mut chosen = HashMap::new();
    chosen.insert("Longueur".to_string(), InVal::Num(20.0));
    chosen.insert("Source".to_string(), InVal::Text("open".into()));
    let o2 = run(src, &b, 3000, &chosen).unwrap();
    let op: Vec<f64> = b.iter().map(|x| x.open).collect();
    same(&o2.plots[0].values, &ta::sma(&op, 20));
}

#[test]
fn erreurs_avec_la_ligne() {
    let b = bars();
    let e = run("indicator(\"x\")\nplot(y)", &b, 60, &HashMap::new()).unwrap_err();
    assert_eq!(e.line, 2);
    assert!(e.msg.contains("« y » n'existe pas"), "{}", e.msg);
    let e = run(
        "x = request.security(syminfo.tickerid, \"D\", close, gaps = barmerge.gaps_on)",
        &b,
        60,
        &HashMap::new(),
    )
    .unwrap_err();
    assert!(e.msg.contains("pas encore"));
    let e = run("i = 0\nwhile true\n    i += 1", &b, 60, &HashMap::new()).unwrap_err();
    assert!(e.msg.contains("boucle infinie"));
    // whichever of the two counts stops a loop without an end, the error has its line (« trop d'opérations » said
    // line 0: the terminal then showed no line at all)
    for (src, ligne, msg) in [
        ("i = 0\nwhile true\n    i += 1", 3, "trop d'opérations sur une bougie (boucle infinie ?)"),
        (
            "j = 0\ni = 0\nwhile true\n    i += 1",
            3,
            "trop de tours de boucle sur une bougie (boucle infinie ?)",
        ),
    ] {
        let e = run(src, &b, 60, &HashMap::new()).unwrap_err();
        assert_eq!((e.line, e.msg.as_str()), (ligne, msg), "{src}");
    }
}

/// B-049: one pasted line, `a = array.new_float(1e12, 0.0)`, asked the computer for 24 000 Go and the whole app
/// stopped (in the browser the wasm aborts: every chart, not only the script); `str.repeat('x', 1e12)` took the same
/// path. Sizes were read without a limit. An array holds 100 000 elements at most, as on TradingView, and a text
/// 100 000 characters: past it, an error with its line, whatever makes it grow (new, push, unshift, insert, concat;
/// str.repeat, +, replace, join, format). At the limit itself, everything still runs.
#[test]
fn un_tableau_ou_un_texte_geant_est_une_erreur_avec_sa_ligne() {
    let b = &bars()[..3];
    let lancer = |corps: &str| run(&format!("//@version=6\nindicator('t')\n{corps}"), b, 3000, &HashMap::new());
    for (corps, ligne, f) in [
        ("a = array.new_float(1e12, 0.0)", 3, "array.new_float"),
        ("a = array.new<int>(100001)", 3, "array.new"),
        ("a = array.new_float(100000, 0.0)\na.push(1.0)", 4, "array.push"),
        ("a = array.new_float(100000, 0.0)\na.unshift(1.0)", 4, "array.unshift"),
        ("a = array.new_float(100000, 0.0)\na.insert(5, 1.0)", 4, "array.insert"),
        (
            "a = array.new_float(60000, 0.0)\nb = array.new_float(60000, 0.0)\na.concat(b)",
            5,
            "array.concat",
        ),
        ("var a = array.new_float(99990, 0.0)\nfor i = 1 to 5\n    a.push(1.0)", 5, "array.push"),
    ] {
        let e = lancer(corps).expect_err(corps);
        assert_eq!(
            (e.line, e.msg),
            (ligne, format!("{f} : 100 000 éléments au plus, comme sur TradingView")),
            "{corps}"
        );
    }
    for (corps, ligne, f) in [
        ("s = str.repeat('x', 1e12)", 3, "str.repeat"),
        ("s = str.repeat('abc', 33334)", 3, "str.repeat"),
        ("s = str.repeat('x', 60000)\nt = s + s", 4, "+"),
        ("s = str.repeat('x', 60000)\ns += s", 4, "+"),
        ("s = str.repeat('x', 1000)\nt = str.replace_all(s, 'x', s)", 4, "str.replace_all"),
        ("s = str.repeat('x', 60000)\nt = str.replace(s, 'x', s)", 4, "str.replace"),
        ("a = array.new_string(1000, str.repeat('x', 1000))\nt = a.join(',')", 4, "array.join"),
        ("s = str.repeat('x', 60000)\nt = str.format('{0}{1}', s, s)", 4, "str.format"),
        ("s = str.repeat('x', 100000)\nt = 1 + s", 4, "+"),
        ("s = str.repeat('x', 60000)\nlog.info('{0}{1}', s, s)", 4, "log.info"),
        ("s = str.repeat('x', 60000)\nlog.warning('{0}{1}', s, s)", 4, "log.warning"),
        ("s = str.repeat('x', 60000)\nlog.error('{0}{1}', s, s)", 4, "log.error"),
        ("s = str.repeat('x', 60000)\nalert('{0}{1}', s, s)", 4, "alert"),
    ] {
        let e = lancer(corps).expect_err(corps);
        assert_eq!((e.line, e.msg), (ligne, format!("{f} : 100 000 caractères au plus dans un texte")), "{corps}");
    }
    // the limit itself passes (accents count for one character each, whatever their bytes)
    let o = lancer(
        "a = array.new_float(99999, 0.0)
a.push(1.0)
b = array.new_float(50000, 0.0)
b.concat(array.new_float(50000, 0.0))
s = str.repeat('é', 50000)
t = s + s
plot(a.size(), 'a')
plot(b.size(), 'b')
plot(str.length(t), 't')
plot(str.length(str.replace_all(s, 'é', 'éé')), 'r')",
    )
    .unwrap_or_else(|e| panic!("{e}"));
    for nom in ["a", "b", "t", "r"] {
        let p = o.plots.iter().find(|p| p.title == nom).expect(nom);
        assert_eq!(p.values[2], 100_000.0, "{nom}");
    }
}

/// B-049, B-036: a text is counted while it is written, piece by piece — a collection inside a collection is never
/// built whole to be refused afterwards, an array that holds itself has an end —, and a matrix or a table is refused
/// side by side, before its sides are multiplied.
#[test]
fn un_texte_de_collection_ou_un_cote_impossible_est_refuse_avant_d_etre_construit() {
    let b = &bars()[..3];
    let lancer = |corps: &str| run(&format!("//@version=6\nindicator('t')\n{corps}"), b, 3000, &HashMap::new());
    let texte = |f: &str| format!("{f} : 100 000 caractères au plus dans un texte");
    let elements = |f: &str| format!("{f} : 100 000 éléments au plus, comme sur TradingView");
    let cases = || "table.new : trop de cases (10 000 au plus)".to_string();
    // b: 10 times the 400 texts of a, 4 000 000 characters once written
    let dix = "a = array.new_string(400, str.repeat('x', 1000))\nb = array.from(a, a, a, a, a, a, a, a, a, a)";
    // f: 100 times e, itself 100 times d…: 600 elements in memory, 10^12 characters once written
    let geant = "a = array.new_string(100, 'x')
b = array.new<string>(100, a)
c = array.new<string>(100, b)
d = array.new<string>(100, c)
e = array.new<string>(100, d)
f = array.new<string>(100, e)";
    // a: an array that holds itself, a text without an end
    let boucle = "a = array.new<float>()\na.push(a)";
    for (corps, ligne, msg) in [
        (format!("{dix}\nt = str.tostring(b)"), 5, texte("str.tostring")),
        ("m = matrix.new<float>(1e30, 0)".to_string(), 3, elements("matrix.new")),
        ("m = matrix.new<float>(0, 1e30)".to_string(), 3, elements("matrix.new")),
        ("m = matrix.new<float>(100001, 0)".to_string(), 3, elements("matrix.new")),
        ("m = matrix.new<float>(400, 400)".to_string(), 3, elements("matrix.new")),
        ("m = matrix.new<float>(1000, 101, 0)".to_string(), 3, elements("matrix.new")),
        ("m = matrix.new<float>(1e12, 1e12)".to_string(), 3, elements("matrix.new")),
        ("t = table.new(position.top_right, 1e30, 1e30)".to_string(), 3, cases()),
        ("t = table.new(position.top_right, 4294967296, 4294967296)".to_string(), 3, cases()),
        ("t = table.new(position.top_right, 65536, 65536)".to_string(), 3, cases()),
        ("t = table.new(position.top_right, 101, 100)".to_string(), 3, cases()),
        (format!("{geant}\nt = str.tostring(f)"), 9, texte("str.tostring")),
        (format!("{geant}\nt = f.join('')"), 9, texte("array.join")),
        (format!("{geant}\nt = 'f = ' + f"), 9, texte("+")),
        (format!("{geant}\nt = f + ' = f'"), 9, texte("+")),
        (format!("{geant}\nt = str.format('{{0}}', f)"), 9, texte("str.format")),
        (format!("{boucle}\nt = str.tostring(a)"), 5, texte("str.tostring")),
        (format!("{boucle}\nt = a.join(',')"), 5, texte("array.join")),
        (format!("{boucle}\nt = '' + a"), 5, texte("+")),
    ] {
        let e = lancer(&corps).expect_err(&corps);
        assert_eq!((e.line, e.msg), (ligne, msg), "{corps}");
    }
    // read as the text of a label, such a collection is cut at the limit (its start is shown), never built whole
    for corps in [
        format!("{geant}\nlabel.new(bar_index, close, f)"),
        format!("{boucle}\nlabel.new(bar_index, close, a)"),
    ] {
        let o = lancer(&corps).unwrap_or_else(|e| panic!("{corps} : {e}"));
        let t = &o.drawings.last().expect(&corps).text;
        assert!(
            t.chars().count() <= 100_000 && t.starts_with('[') && t.ends_with('…'),
            "{corps} : {} octets",
            t.len()
        );
    }
    // cut at the characters a text holds, whoever reads it: `string(a)` and `str.upper(a)` gave 300 000 letters
    let o = lancer(
        "a = array.new_string(60000, 'abc')
b = array.new_string(30000, 'é')
plot(str.length(string(a)), 'string')
plot(str.length(str.upper(a)), 'upper')
plot(str.length(string(b)), 'accents')
plot(str.endswith(string(a), '…') ? 1 : 0, 'fin')
plot(str.length(string(array.new_string(33333, 'a'))), 'juste')
c = array.new_string(33333, 'a')
c.set(0, 'aa')
plot(str.length(string(c)), 'au ras')
plot(str.endswith(string(c), 'a]') ? 1 : 0, 'au ras, entier')
c.set(0, 'aaa')
plot(str.length(string(c)), 'un de trop')
plot(str.endswith(string(c), ', …') ? 1 : 0, 'un de trop, coupé')
e = array.from(str.repeat('😀', 100000))
plot(str.length(string(e)), 'quatre octets')
plot(str.endswith(string(e), '😀…') ? 1 : 0, 'quatre octets, coupé')",
    )
    .unwrap_or_else(|e| panic!("{e}"));
    let lu = |nom: &str| o.plots.iter().find(|p| p.title == nom).expect(nom).values[2];
    assert_eq!(
        (lu("string"), lu("upper"), lu("accents"), lu("fin"), lu("juste")),
        (100_000.0, 100_000.0, 90_000.0, 1.0, 99_999.0)
    );
    // a text of 100 000 characters is whole, its last one kept; one more and it is cut
    assert_eq!(
        (lu("au ras"), lu("au ras, entier"), lu("un de trop"), lu("un de trop, coupé")),
        (100_000.0, 1.0, 100_000.0, 1.0)
    );
    // characters of 4 bytes: cut at 100 000 characters too (the piece that did not fit was left out whole: « [… »)
    assert_eq!((lu("quatre octets"), lu("quatre octets, coupé")), (100_000.0, 1.0));
    // under the limits nothing changes: the same texts as before, a matrix and a table of the largest size
    let o = lancer(
        "a = array.from(1, 2.5, na)
b = array.from(a, a)
m = matrix.new<float>(2, 2, 0)
mp = map.new<string, float>()
mp.put('k', 1)
g = matrix.new<float>(100000, 1)
v = matrix.new<float>(100000, 0)
c = matrix.new<float>(1000, 99, 0)
c.add_col()
t = table.new(position.top_right, 100, 100)
label.new(bar_index, close, str.tostring(a) + ' ' + str.tostring(b) + ' ' + str.tostring(m) + ' ' + str.tostring(mp) + ' ' + b.join(' / ') + str.format(' {0}', a))
plot(g.elements_count() + v.rows() + c.elements_count(), 'g')",
    )
    .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        o.drawings.last().unwrap().text,
        "[1, 2.5, NaN] [[1, 2.5, NaN], [1, 2.5, NaN]] [[0, 0], [0, 0]] {k: 1} [1, 2.5, NaN] / [1, 2.5, NaN] [1, 2.5, NaN]"
    );
    assert_eq!(o.plots[0].values[2], 300_000.0);
    assert_eq!((o.tables[0].columns, o.tables[0].rows), (100, 100));
}

/// B-051: a length, an occurrence, a timeframe or a format out of reach never stops the app — every `ta.*` function
/// with every impossible length (0, negative, na, a fraction, 1e30, infinity, the largest whole numbers), then the
/// timeframes and the format patterns. An error with its line or `na` is fine; a panic is the whole app stopped.
#[test]
fn une_longueur_impossible_ne_fait_jamais_tomber_l_app() {
    let b = &bars()[..40];
    let longueurs = [
        "0",
        "-1",
        "-1e30",
        "na",
        "0.5",
        "1e30",
        "1e400",
        "-1e400",
        "4294967295",
        "4294967296",
        "9223372036854775807",
        "18446744073709551615",
    ];
    let ta = [
        "ta.sma(close, L)",
        "ta.ema(close, L)",
        "ta.rma(close, L)",
        "ta.wma(close, L)",
        "ta.vwma(close, L)",
        "ta.hma(close, L)",
        "ta.rsi(close, L)",
        "ta.atr(L)",
        "ta.stdev(close, L)",
        "ta.variance(close, L)",
        "ta.dev(close, L)",
        "ta.highest(close, L)",
        "ta.highest(L)",
        "ta.lowest(close, L)",
        "ta.lowest(L)",
        "ta.highestbars(close, L)",
        "ta.highestbars(L)",
        "ta.lowestbars(close, L)",
        "ta.lowestbars(L)",
        "ta.change(close, L)",
        "ta.mom(close, L)",
        "ta.roc(close, L)",
        "ta.rising(close, L)",
        "ta.falling(close, L)",
        "ta.valuewhen(close > open, close, L)",
        "ta.pivothigh(L, 2)",
        "ta.pivothigh(2, L)",
        "ta.pivothigh(L, L)",
        "ta.pivotlow(close, L, 2)",
        "ta.pivotlow(close, 2, L)",
        "ta.pivotlow(close, L, L)",
        "ta.macd(close, L, 26, 9)",
        "ta.macd(close, 12, L, 9)",
        "ta.macd(close, 12, 26, L)",
        "ta.bb(close, L, 2)",
        "ta.bb(close, 20, L)",
        "ta.bbw(close, L, 2)",
        "ta.kc(close, L, 2)",
        "ta.kc(close, 20, L)",
        "ta.stoch(close, high, low, L)",
        "ta.cci(close, L)",
        "ta.mfi(hlc3, L)",
        "ta.linreg(close, L, 0)",
        "ta.linreg(close, 14, L)",
        "ta.median(close, L)",
        "ta.percentrank(close, L)",
        "ta.range(close, L)",
        "ta.dmi(L, 14)",
        "ta.dmi(14, L)",
        "ta.supertrend(L, 10)",
        "ta.supertrend(3, L)",
        "ta.sar(L, 0.02, 0.2)",
        "ta.sar(0.02, L, 0.2)",
        "ta.sar(0.02, 0.02, L)",
        "ta.correlation(close, open, L)",
        "ta.vwap(close, true, L)",
        "ta.tr(L)",
    ];
    let unites = [
        "",
        "0",
        "-5",
        "1e30",
        "x",
        "153722867280912930",
        "153722867280912931",
        "4611686018427387904",
        "9223372036854775807",
        "99999999999999999999",
        "9223372036854775807S",
        "106751991167301D",
        "9223372036854775807D",
        "15250284452472W",
        "9223372036854775807W",
        "3558399705577M",
        "9223372036854775807M",
    ];
    let temps = [
        "timeframe.change('U')",
        "timeframe.in_seconds('U')",
        "time('U')",
        "time_close('U')",
        "request.security(syminfo.tickerid, 'U', close)",
    ];
    let motifs = [
        "str.tostring(close, '#.' + str.repeat('#', 70000))",
        "str.tostring(close, '0.' + str.repeat('0', 70000))",
        "str.tostring(close, '#.' + str.repeat('#', 65536))",
        "str.tostring(close, '#.' + str.repeat('#', 400))",
        "str.format('{0,number,#.' + str.repeat('#', 70000) + '}', close)",
        "str.format('{0,number,#.' + str.repeat('#', 400) + '}', close)",
    ];
    let mut appels: Vec<String> = vec![];
    for f in ta {
        appels.extend(longueurs.iter().map(|l| f.replace('L', l)));
    }
    for f in temps {
        appels.extend(unites.iter().map(|u| f.replace('U', u)));
    }
    appels.extend(motifs.iter().map(|m| m.to_string()));
    let tombes: Vec<&String> = appels
        .iter()
        .filter(|appel| {
            let src = format!("//@version=6\nindicator('t')\n{appel}\nplot(close)");
            std::panic::catch_unwind(|| run(&src, b, 3000, &HashMap::new()).is_ok()).is_err()
        })
        .collect();
    assert!(
        tombes.is_empty(),
        "{} appels sur {} font tomber l'app : {tombes:#?}",
        tombes.len(),
        appels.len()
    );
    // a pattern with more decimals than a number holds still writes the number
    let o = go(
        "//@version=6\nindicator('t')\nlabel.new(bar_index, close, str.tostring(1.5, '#.' + str.repeat('#', 400)))",
        &b[..1],
    );
    assert!(
        o.drawings[0].text.starts_with("1.5") && o.drawings[0].text.len() <= 402,
        "{}",
        o.drawings[0].text.len()
    );
}

/// B-050: the history of a variable keeps a collection, an object or a text only when the script reads that name
/// back with `[n]`. An array, a matrix or a text made again at each bar stayed in memory for every bar of the chart
/// (1 157 MB for a matrix of 100 × 100 on 5 000 bars), where nothing could read it.
#[test]
fn l_historique_ne_garde_un_tableau_que_si_le_script_le_relit() {
    let b = &bars()[..200];
    let gardes = |src: &str| lourds_gardes(src, b, 3000).unwrap_or_else(|e| panic!("{e}"));
    // every kind of variable: a declaration, a tuple, a `var`, a parameter, a loop variable, a field
    let sans = "//@version=6
indicator('t')
type T
    float p
    array<float> v
taille(x) => x.size()
deux() => [array.from(close), str.tostring(close)]
a = array.new_float(50, close)
m = matrix.new<float>(5, 5, close)
s = str.tostring(close)
mp = map.new<string, float>()
o = T.new(low, a)
n = taille(a)
[t1, t2] = deux()
var k = array.new_float()
k.push(close)
for e in array.from(a, a)
    n += e.size()
plot(n + m.rows() + str.length(s) + mp.size() + o.p + t1.size() + str.length(t2) + k.size())";
    assert_eq!(gardes(sans), 0);
    // the same script reading two of its names back: these two are kept as far back as the script reads them
    // (B-062: one bar for `a[1]`, two for `s[2]`), the others are not
    let avec = format!("{sans}\na1 = a[1]\nplot(na(a1) ? 0 : a1.size())\nplot(str.length(s[2]))");
    assert_eq!(gardes(&avec), 1 + 2);
    // a name read back inside a function keeps the variables of that name everywhere (the careful rule)
    let dans_f = format!("{sans}\nhier(m) => m[1]\nh = hier(close)");
    assert_eq!(gardes(&dans_f), 1);
    // an index the script computes can reach any bar: everything is kept
    let calcule = format!("{sans}\nplot(str.length(s[bar_index % 7]))");
    assert_eq!(gardes(&calcule), 199);
}

/// B-069: a script that reads another timeframe (`request.security`) runs once more on the bars of that timeframe.
/// There, an input kept the value chosen in the settings at the first bar only, then took its default value again
/// (and an untitled one a new name at each bar): the higher-timeframe line was computed with the default length.
#[test]
fn un_reglage_choisi_compte_a_chaque_bougie_d_une_autre_unite() {
    let b = bars();
    let src =
        |len: &str| format!("//@version=6\nindicator('t')\nlen = {len}\nh = request.security(syminfo.tickerid, '100', ta.sma(close, len))\nplot(h)\nplot(len)");
    let fixe = go(&src("20"), &b);
    assert!(fixe.plots[0].values.iter().filter(|v| v.is_finite()).count() > 400);
    for (titre, script) in [("Longueur", "input.int(5, 'Longueur')"), ("Input 1", "input(5)")] {
        let choisi: HashMap<String, InVal> = [(titre.to_string(), InVal::Num(20.0))].into();
        let o = run(&src(script), &b, 3000, &choisi).unwrap_or_else(|e| panic!("{e}"));
        same(&o.plots[0].values, &fixe.plots[0].values);
        same(&o.plots[1].values, &fixe.plots[1].values);
        assert_eq!(o.inputs.iter().map(|i| i.title.as_str()).collect::<Vec<_>>(), [titre]);
        // the same in a strategy, whose seven properties come first in the settings: the untitled input was
        // « Input 8 » on the chart and « Input 1 » on the other timeframe, which kept the default value
        let strategie = |s: &str| s.replace("indicator('t')", "strategy('t')");
        let fixe = go(&strategie(&src("20")), &b);
        let o = run(&strategie(&src(script)), &b, 3000, &choisi).unwrap_or_else(|e| panic!("{e}"));
        same(&o.plots[0].values, &fixe.plots[0].values);
        same(&o.plots[1].values, &fixe.plots[1].values);
        assert_eq!(o.inputs.len(), fixe.inputs.len() + 1, "the properties and this input");
        assert_eq!(o.inputs.iter().filter(|i| i.title == titre).count(), 1);
    }
    // eight untitled inputs in a strategy: on the other timeframe the eighth took the value chosen for the first
    let mut src = String::from("//@version=6\nstrategy('t')\n");
    for k in 1..=8 {
        src += &format!("a{k} = input({k})\n");
    }
    src += "plot(request.security(syminfo.tickerid, '100', a8))\nplot(a1)\nplot(a8)";
    let choisi = |titre: &str, v: f64| HashMap::from([(titre.to_string(), InVal::Num(v))]);
    let o = run(&src, &b, 3000, &choisi("Input 8", 99.0)).unwrap_or_else(|e| panic!("{e}"));
    let finis = |k: usize| o.plots[k].values.iter().copied().filter(|v| v.is_finite()).collect::<Vec<_>>();
    assert!(
        finis(0).len() > 400 && finis(0).iter().all(|v| *v == 99.0),
        "the eighth input on the other timeframe"
    );
    assert!(finis(1).iter().all(|v| *v == 1.0) && finis(2).iter().all(|v| *v == 99.0));
    // the two runs do not reach the same inputs first: each input keeps its own name and its own chosen value
    let src = "//@version=6
indicator('t')
haut = timeframe.in_seconds() > 3000
a = haut ? 0 : input(1)
b = input(2)
c = haut ? input(3) : 0
plot(request.security(syminfo.tickerid, '100', b * 10 + c))
plot(a * 100 + b)";
    let o = run(src, &b, 3000, &choisi("Input 2", 7.0)).unwrap_or_else(|e| panic!("{e}"));
    let finis = |k: usize| o.plots[k].values.iter().copied().filter(|v| v.is_finite()).collect::<Vec<_>>();
    assert!(finis(0).len() > 400 && finis(0).iter().all(|v| *v == 73.0), "{:?}", &finis(0)[..3]);
    assert!(finis(1).iter().all(|v| *v == 107.0));
}

/// B-062: a variable keeps its past values only as far back as the script reads its name with `[n]`. Every variable
/// kept one value a bar for the whole chart, read or not: 200 number variables on 200 000 bars held 932 MB.
#[test]
fn une_variable_ne_garde_que_le_passe_que_le_script_relit() {
    let b = &bars()[..300];
    let garde = |src: &str| passe_garde(src, b, 3000).unwrap_or_else(|e| panic!("{e}"));
    let sans = "//@version=6
indicator('t')
f(x) => x * 2
a = close + 1
var b = 0.0
b += a
c = f(a)
[d, e] = [a, b]
plot(a + b + c + d + e)";
    assert_eq!(garde(sans), 0, "nothing is read back: nothing is kept");
    // read back with written numbers: the furthest one counts, for every variable of that name
    assert_eq!(garde(&format!("{sans}\nplot(a[1] + a[3] + b[2])")), 3 + 2);
    // read back with a computed number: every closed bar
    assert_eq!(garde(&format!("{sans}\nplot(a[bar_index % 5])")), 299);
    // the results are the ones of a full history, at every bar
    let src = "//@version=6
indicator('t')
a = close - open
var s = 0.0
s += a
g(x) => x[2]
plot(a[1])
plot(a[4] + s[3])
plot(g(a))
plot(s[bar_index % 6])
plot(bar_index > 250 ? a[1] : na)";
    let o = go(src, b);
    let (c, op): (Vec<f64>, Vec<f64>) = (b.iter().map(|x| x.close).collect(), b.iter().map(|x| x.open).collect());
    let a: Vec<f64> = c.iter().zip(&op).map(|(c, o)| c - o).collect();
    let s: Vec<f64> = a
        .iter()
        .scan(0.0, |t, x| {
            *t += x;
            Some(*t)
        })
        .collect();
    let pareil = |x: f64, y: f64| (x.is_nan() && y.is_nan()) || (x - y).abs() < 1e-9;
    for i in 0..b.len() {
        let hier = |v: &[f64], k: usize| if i >= k { v[i - k] } else { f64::NAN };
        assert!(pareil(o.plots[0].values[i], hier(&a, 1)), "a[1] at {i}");
        assert!(pareil(o.plots[1].values[i], hier(&a, 4) + hier(&s, 3)), "a[4] + s[3] at {i}");
        assert!(pareil(o.plots[2].values[i], hier(&a, 2)), "g(a) at {i}");
        assert!(pareil(o.plots[3].values[i], hier(&s, i % 6)), "s[i % 6] at {i}");
        assert!(pareil(o.plots[4].values[i], if i > 250 { hier(&a, 1) } else { f64::NAN }), "late a[1] at {i}");
    }
}

/// B-050: a script that reads a collection, an object or a text back with `[n]` keeps exactly its results — in a
/// function too, `var` or not.
#[test]
fn un_tableau_relu_avec_son_historique_garde_ses_resultats() {
    let b = &bars()[..60];
    let o = go(
        "//@version=6
indicator('t')
type T
    float p
passe(x) => x[1]
a = array.from(close, open)
m = matrix.new<float>(1, 1, high)
s = str.tostring(bar_index)
o = T.new(low)
var v = array.new_float()
v.push(close)
a1 = a[1]
a2 = a[2]
m1 = m[1]
o1 = o[1]
pa = passe(a)
v1 = v[1]
plot(na(a1) ? na : a1.get(0), 'a1')
plot(na(a2) ? na : a2.get(1), 'a2')
plot(na(m1) ? na : m1.get(0, 0), 'm1')
plot(str.tonumber(s[3]), 's3')
plot(na(o1) ? na : o1.p, 'o1')
plot(na(pa) ? na : pa.get(0), 'pa')
plot(na(v1) ? na : v1.size(), 'v1')",
        b,
    );
    let p = |t: &str| o.plots.iter().find(|p| p.title == t).unwrap().values.clone();
    let avant = |f: fn(&Bar) -> f64, k: usize| -> Vec<f64> { (0..b.len()).map(|i| if i < k { f64::NAN } else { f(&b[i - k]) }).collect() };
    same(&p("a1"), &avant(|x| x.close, 1));
    same(&p("a2"), &avant(|x| x.open, 2));
    same(&p("m1"), &avant(|x| x.high, 1));
    same(&p("o1"), &avant(|x| x.low, 1));
    same(&p("pa"), &avant(|x| x.close, 1));
    let s3: Vec<f64> = (0..b.len()).map(|i| if i < 3 { f64::NAN } else { (i - 3) as f64 }).collect();
    same(&p("s3"), &s3);
    // (a `var` array is shared: its past value is the same array, with today's size)
    let v1: Vec<f64> = (0..b.len()).map(|i| if i < 1 { f64::NAN } else { (i + 1) as f64 }).collect();
    same(&p("v1"), &v1);
}

/// B-049: the limit follows the chart. An array fed once a bar (LuxAlgo SMC keeps its highs, lows and times so) still
/// runs on a chart longer than TradingView's: 100 000 elements, as there, up to 20 000 bars; 5 a bar past them (a
/// chart holds 200 000 bars here). The limit is the chart's in a `request.security` sub-run too.
#[test]
fn un_tableau_nourri_a_chaque_bougie_suit_une_longue_chart() {
    let serie = |n: i64| -> Vec<Bar> {
        (0..n)
            .map(|i| Bar {
                time: 1_600_000_000 + i * 60,
                open: 1.0,
                high: 2.0,
                low: 0.5,
                close: 1.0,
                volume: 1.0,
            })
            .collect()
    };
    let lancer = |b: &[Bar], corps: &str| run(&format!("//@version=6\nindicator('t')\n{corps}"), b, 60, &HashMap::new());
    let dernier = |o: &Output, nom: &str| *o.plots.iter().find(|p| p.title == nom).expect(nom).values.last().unwrap();
    let tv = |f: &str| format!("{f} : 100 000 éléments au plus, comme sur TradingView");
    let par_bougie = |f: &str, n: &str| format!("{f} : {n} éléments au plus (5 par bougie de la chart)");

    // 100 001 bars: one push a bar ends at 100 001 elements
    let b = serie(100_001);
    let o = lancer(&b, "var a = array.new_float()\na.push(high)\nplot(a.size(), 'a')").unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(dernier(&o, "a"), 100_001.0);
    // the limit there is 500 005: for a matrix that grows by 2 a bar, for an array made in one go, and in the hourly
    // sub-run of `request.security` too (1 667 bars of its own)
    let o = lancer(
        &b,
        "var m = matrix.new<float>(0, 2)
m.add_row(m.rows(), array.from(high, low))
var g = array.new_float(500005)
plot(m.rows(), 'm')
plot(g.size(), 'g')
plot(request.security(syminfo.tickerid, '60', g.size()), 'h')",
    )
    .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!((dernier(&o, "m"), dernier(&o, "g"), dernier(&o, "h")), (100_001.0, 500_005.0, 500_005.0));
    for (corps, ligne, f) in [
        ("a = array.new_float(500006)", 3, "array.new_float"),
        ("a = array.new_float(1e12, 0.0)", 3, "array.new_float"),
        ("var a = array.new_float()\nfor i = 1 to 6\n    a.push(high)", 5, "array.push"),
        ("m = matrix.new<float>(1000, 501)", 3, "matrix.new"),
    ] {
        let e = lancer(&b, corps).expect_err(corps);
        assert_eq!((e.line, e.msg), (ligne, par_bougie(f, "500 005")), "{corps}");
    }

    // 20 000 bars: TradingView's own limit
    let b = serie(20_000);
    let o = lancer(&b, "var a = array.new_float(100000)\nplot(a.size(), 'a')").unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(dernier(&o, "a"), 100_000.0);
    for corps in ["a = array.new_float(100001)", "a = array.new_float(1e12)"] {
        let e = lancer(&b, corps).expect_err(corps);
        assert_eq!((e.line, e.msg), (3, tv("array.new_float")), "{corps}");
    }

    // 200 000 bars, the longest chart: 1 000 000
    let b = serie(200_000);
    let o = lancer(&b, "var a = array.new_float(1000000)\nplot(a.size(), 'a')").unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(dernier(&o, "a"), 1_000_000.0);
    for corps in ["a = array.new_float(1000001)", "a = array.new_float(1e12)"] {
        let e = lancer(&b, corps).expect_err(corps);
        assert_eq!((e.line, e.msg), (3, par_bougie("array.new_float", "1 000 000")), "{corps}");
    }
}

/// Scripts Pine v6 tels qu'on les trouve sur TradingView : ils roulent sans changement.
#[test]
fn scripts_pine_reels() {
    let b = bars();
    // Supertrend (exemple officiel de TradingView, Pine v6)
    let st = go(
        r#"//@version=6
indicator("Supertrend", overlay = true, timeframe = "", timeframe_gaps = true)
atrPeriod = input.int(10, "ATR Length", minval = 1)
factor = input.float(3.0, "Factor", minval = 0.01, step = 0.01)
[supertrend, direction] = ta.supertrend(factor, atrPeriod)
supertrend := barstate.isfirst ? na : supertrend
upTrend = plot(direction < 0 ? supertrend : na, "Up Trend", color = color.green, style = plot.style_linebr)
downTrend = plot(direction < 0 ? na : supertrend, "Down Trend", color = color.red, style = plot.style_linebr)
bodyMiddle = plot(barstate.isfirst ? na : (open + close) / 2, "Body Middle", display = display.none)
fill(bodyMiddle, upTrend, title = "Uptrend background", color = color.new(color.green, 90), fillgaps = false)
fill(bodyMiddle, downTrend, title = "Downtrend background", color = color.new(color.red, 90), fillgaps = false)
alertcondition(direction[1] > direction, title = "Downtrend to Uptrend", message = "The Supertrend value switched from Downtrend to Uptrend ")
"#,
        &b,
    );
    assert_eq!(st.plots.len(), 3);
    assert_eq!(st.fills.len(), 2);
    assert!(!st.plots[2].visible, "display.none");
    let (up, dn) = (&st.plots[0].values, &st.plots[1].values);
    let k = 300;
    assert!(up[k].is_nan() != dn[k].is_nan(), "une seule des deux lignes à la fois");
    // le supertrend est sous le prix en tendance haussière, au-dessus en baissière
    for k in 50..b.len() {
        if up[k].is_finite() {
            assert!(up[k] <= b[k].high, "bougie {k}");
        }
    }
    // Squeeze Momentum (LazyBear), réécrit en v6
    let sq = go(
        r#"//@version=6
indicator("Squeeze Momentum Indicator [LazyBear]", shorttitle = "SQZMOM_LB", overlay = false)
length = input.int(20, title = "BB Length")
mult = input.float(2.0, title = "BB MultFactor")
lengthKC = input.int(20, title = "KC Length")
multKC = input.float(1.5, title = "KC MultFactor")
useTrueRange = input.bool(true, title = "Use TrueRange (KC)")
source = close
basis = ta.sma(source, length)
dev = multKC * ta.stdev(source, length)
upperBB = basis + dev
lowerBB = basis - dev
ma = ta.sma(source, lengthKC)
range_1 = useTrueRange ? ta.tr : high - low
rangema = ta.sma(range_1, lengthKC)
upperKC = ma + rangema * multKC
lowerKC = ma - rangema * multKC
sqzOn = lowerBB > lowerKC and upperBB < upperKC
sqzOff = lowerBB < lowerKC and upperBB > upperKC
noSqz = sqzOn == false and sqzOff == false
val = ta.linreg(source - math.avg(math.avg(ta.highest(high, lengthKC), ta.lowest(low, lengthKC)), ta.sma(close, lengthKC)), lengthKC, 0)
bcolor = val > 0 ? (val > nz(val[1]) ? color.lime : color.green) : (val < nz(val[1]) ? color.red : color.maroon)
scolor = noSqz ? color.blue : sqzOn ? color.black : color.gray
plot(val, color = bcolor, style = plot.style_histogram, linewidth = 4)
plot(0, color = scolor, style = plot.style_cross, linewidth = 2)
"#,
        &b,
    );
    assert_eq!(sq.short, "SQZMOM_LB");
    assert_eq!(sq.plots[0].style, PlotStyle::Histogram);
    assert!(sq.plots[0].values[100].is_finite());
    assert!(sq.plots[0]
        .colors
        .iter()
        .skip(60)
        .all(|c| matches!(c, Some(0x00E676 | 0x4CAF50 | 0xFF5252 | 0x880E4F))));
    // pivots avec tableau de lignes (style SMC) et labels
    let pv = go(
        r#"//@version=6
indicator("Pivots", overlay = true, max_lines_count = 50)
left = input.int(5, "Left")
right = input.int(5, "Right")
var lines = array.new<line>()
ph = ta.pivothigh(left, right)
pl = ta.pivotlow(left, right)
if not na(ph)
    l = line.new(bar_index - right, ph, bar_index, ph, color = color.red, extend = extend.right)
    lines.push(l)
    if lines.size() > 5
        line.delete(lines.shift())
if not na(pl)
    label.new(bar_index - right, pl, "PL", style = label.style_label_up, color = color.green)
"#,
        &b,
    );
    let live = pv.drawings.iter().filter(|d| d.kind == "line" && !d.deleted).count();
    assert_eq!(live, 5, "les plus vieilles lignes sont effacées");
    assert!(pv.drawings.iter().any(|d| d.kind == "label" && d.text == "PL"));
}

#[test]
fn assez_rapide() {
    // 1000 bougies, une douzaine de fonctions : bien sous une demi-seconde (même en debug)
    let v: serde_json::Value = serde_json::from_str(include_str!("../../tests/fixtures/btc_hl_30m.json")).unwrap();
    let b: Vec<Bar> = v
        .as_array()
        .unwrap()
        .iter()
        .map(|x| Bar {
            time: x["time"].as_i64().unwrap(),
            open: x["open"].as_f64().unwrap(),
            high: x["high"].as_f64().unwrap(),
            low: x["low"].as_f64().unwrap(),
            close: x["close"].as_f64().unwrap(),
            volume: x["volume"].as_f64().unwrap(),
        })
        .collect();
    let t = std::time::Instant::now();
    go(
        r#"indicator("vitesse")
[m, s, h] = ta.macd(close, 12, 26, 9)
r = ta.rsi(close, 14)
[mid, up, dn] = ta.bb(close, 20, 2)
[st, dir] = ta.supertrend(3, 10)
plot(ta.ema(close, 200))
plot(ta.sma(close, 50))
plot(r)
plot(h)
var arr = array.new<float>()
arr.push(close)
if arr.size() > 100
    arr.shift()
plot(arr.avg())
"#,
        &b,
    );
    assert!(t.elapsed().as_millis() < 500, "{:?}", t.elapsed());
}

/// `ta.highest(high, n)` appelé seulement dans un `if` : il lit l'historique des bougies, comme Pine.
#[test]
fn highest_dans_un_if_lit_les_bougies() {
    let b = bars();
    let o = go(
        "indicator(\"h\")\nvar float h = na\nif barstate.islast\n    h := ta.highest(high, 40)\nplot(h)\nplot(ta.highest(high, 40))\nplot(ta.lowest(40))",
        &b,
    );
    let n = b.len() - 1;
    assert!(o.plots[0].values[n].is_finite());
    assert_eq!(o.plots[0].values[n], o.plots[1].values[n]);
    assert_eq!(o.plots[2].values[n], b[n - 39..].iter().map(|x| x.low).fold(f64::INFINITY, f64::min));
}

// ---------- 🌐 langues : ancien Argos Script, Pine v4, wrun → mêmes valeurs qu'en Pine v6 ----------

fn plot_of<'a>(o: &'a Output, title: &str) -> &'a [f64] {
    &o.plots
        .iter()
        .find(|p| p.title == title)
        .unwrap_or_else(|| panic!("tracé {title} absent : {:?}", o.plots.iter().map(|p| &p.title).collect::<Vec<_>>()))
        .values
}

#[test]
fn ancien_argos_script_v1() {
    let b = bars();
    let v1 = "// Argos Script\nindicator(\"Mon script\", overlay=false, alerts=true)\n\nr = rsi(close, 14)\nm = macd(close, 12, 26)\nplot(r, \"RSI\", #7e57c2, 2)\nplot(m, \"MACD\", #2962ff, 1, hist)\nhline(70)\nhline(30)\nfill(r, 50, #22c55e, #ef4444)\nbarcolor(r > 70, #f23645)\nsignal(crossover(r, 30), buy, \"Achat\")\nsignal(crossunder(r, 70), sell, \"Vente\")\nshape(r > 80 && close > open, diamond, above, #f0b90b, \"!\")\n";
    assert_eq!(dialect::detect(v1), dialect::Dialect::ArgosV1);
    let o = go(v1, &b);
    let p = go(
        "//@version=6\nindicator(\"x\")\nplot(ta.rsi(close, 14), \"RSI\")\nplot(ta.ema(close, 12) - ta.ema(close, 26), \"MACD\")\n",
        &b,
    );
    same(plot_of(&o, "RSI"), plot_of(&p, "RSI"));
    same(plot_of(&o, "MACD"), plot_of(&p, "MACD"));
    assert_eq!(o.lang, Some(dialect::Dialect::ArgosV1));
    assert_eq!(o.hlines.len(), 2);
    assert_eq!(o.fills.len(), 1, "fill(a, b, #c1, #c2)");
    assert!(
        o.shapes.iter().any(|s| s.text == "Achat") && o.shapes.iter().any(|s| s.text == "Vente"),
        "signal()"
    );
    assert!(o.barcolor.iter().any(|c| c.is_some()), "barcolor()");
    // une erreur pointe la ligne du script d'origine
    let e = run(
        "indicator(\"x\")\nr = rsi(close, 14)\nplot(r, \"RSI\")\nzz = inconnu(r)\n",
        &b,
        3000,
        &HashMap::new(),
    )
    .unwrap_err();
    assert_eq!(e.line, 4, "{e}");
}

#[test]
fn pine_v4_et_v5() {
    let b = bars();
    let v4 = "//@version=4\nstudy(\"Vieux\", overlay=true)\nlen = input(9, title=\"Longueur\", type=input.integer)\ne = ema(close, len)\nhi = highest(high, 20)\nplot(e, \"EMA\", color=color.orange, transp=0)\nplot(iff(close > e, hi, na), \"Haut\")\nplot(abs(change(close)), \"Var\")\nplot(atr(14) + tr * 0, \"ATR\")\n";
    assert_eq!(dialect::detect(v4), dialect::Dialect::PineOld(4));
    let o = go(v4, &b);
    let p = go(
        "//@version=6\nindicator(\"x\")\nplot(ta.ema(close, 9), \"EMA\")\nplot(math.abs(ta.change(close)), \"Var\")\nplot(ta.atr(14), \"ATR\")\n",
        &b,
    );
    for t in ["EMA", "Var", "ATR"] {
        same(plot_of(&o, t), plot_of(&p, t));
    }
    assert_eq!(o.inputs[0].kind, "int");
    assert!(o.notes.iter().any(|(_, m)| m.contains("transp")), "{:?}", o.notes);
    // v5 (LuxAlgo) : déjà dans ta., seul le numéro change ; import mis en commentaire avec un message
    let v5 = "//@version=5\nimport LuxAlgo/Lib/1 as lux\nindicator(\"Lux\", overlay=true)\nplot(ta.sma(close, 20), \"SMA\")\n";
    let o = go(v5, &b);
    assert_eq!(o.lang, Some(dialect::Dialect::PineOld(5)));
    assert!(o.notes.iter().any(|(l, m)| *l == 2 && m.contains("import")));
}

/// 🎨 Pine v2 / v3: the colours without `color.` (`aqua`, `red`…) and `transp=` give the same chart and the same
/// trades as the same script written in v6 — but a variable the script names `green` stays its own, and the words
/// in its texts and comments stay as they are.
#[test]
fn pine_v2_couleurs_nues_et_transp() {
    let b = bars();
    let v2 = r#"//@version=2
strategy("V2", overlay=true)
len = input(10, title="Longueur red")
green = #00AA00
m = sma(close, len)
plot(m, color=aqua, title="Moyenne")
plot(m * 1.01, title="Haut", color=red, transp=50) // red line
up = crossover(close, m)
dn = crossunder(close, m)
bgcolor(up ? lime : dn ? maroon : na, transp=80)
barcolor(up ? green : na)
if (up)
    strategy.entry("L", strategy.long, stop=m)
if (dn)
    strategy.entry("S", strategy.short, stop=m)
"#;
    let v6 = r#"//@version=6
strategy("V2", overlay=true)
len = input.int(10, title="Longueur red")
green = #00AA00
m = ta.sma(close, len)
plot(m, color=color.aqua, title="Moyenne")
plot(m * 1.01, title="Haut", color=color.new(color.red, 50))
up = ta.crossover(close, m)
dn = ta.crossunder(close, m)
bgcolor(color.new(up ? color.lime : dn ? color.maroon : na, 80))
barcolor(up ? green : na)
if (up)
    strategy.entry("L", strategy.long, stop=m)
if (dn)
    strategy.entry("S", strategy.short, stop=m)
"#;
    assert_eq!(dialect::detect(v2), dialect::Dialect::PineOld(2));
    let conv = dialect::to_pine(v2);
    assert!(conv.pine.contains("title=\"Longueur red\""), "{}", conv.pine);
    assert!(conv.pine.contains("// red line"), "{}", conv.pine);
    assert!(conv.pine.contains("up ? green : na"), "{}", conv.pine);
    let (o, p) = (go(v2, &b), go(v6, &b));
    for t in ["Moyenne", "Haut"] {
        same(plot_of(&o, t), plot_of(&p, t));
        let couleurs = |x: &Output| x.plots.iter().find(|q| q.title == t).map(|q| q.colors.clone());
        assert_eq!(couleurs(&o), couleurs(&p), "{t}");
    }
    assert_eq!(o.bgcolor, p.bgcolor);
    assert!(o.bgcolor.iter().any(Option::is_some), "a background on the crossings");
    assert_eq!(o.barcolor, p.barcolor);
    assert_eq!(o.inputs[0].title, "Longueur red");
    let trades = |x: &Output| {
        x.strategie
            .as_ref()
            .expect("a strategy")
            .trades
            .iter()
            .map(|t| (t.entree.clone(), t.bar_in, t.bar_out, t.ferme.entry, t.ferme.exit))
            .collect::<Vec<_>>()
    };
    assert!(!trades(&o).is_empty());
    assert_eq!(trades(&o), trades(&p));
    assert!(o.notes.iter().any(|(l, m)| *l == 7 && m.contains("transp=50")), "{:?}", o.notes);
    assert!(o.notes.iter().any(|(l, m)| *l == 10 && m.contains("transp=80")), "{:?}", o.notes);
}

#[test]
fn wrun_dialecte() {
    let b = bars();
    let w = r##"import { Ema, Rsi, Macd } from "./sdk/ta";
param("fast", 9, { min: 1, description: "Rapide" });
param.int("slow", 21, { min: 2 });
output("fast", line, overlay, { color: "#f0b90b", width: 2 });
output("slow", line, overlay, { color: "#2962ff" });
const cross = output("cross", none, overlay);
alert("Golden", { when: cross, message: "croisement" });
let ef: Ema;
let es: Ema;
let n: f64 = 0;
export function onStart(): void {
  ef = new Ema(i32(p_fast()));
  es = new Ema(i32(p_slow()));
}
export function onBar(): void {
  const c = bar.close();
  const f = ef.update(c);
  const s = es.update(c);
  out_fast(f);
  out_slow(s);
  if (f > s && !isNaN(s)) {
    n += 1;
  } else {
    n = 0;
  }
  out_cross(n == 1 ? 1 : 0);
}
"##;
    assert_eq!(dialect::detect(w), dialect::Dialect::Wrun);
    let conv = dialect::to_pine(w);
    let o = run(w, &b, 3000, &HashMap::new()).unwrap_or_else(|e| panic!("{e}\n---\n{}", conv.pine));
    let p = go(
        "//@version=6\nindicator(\"x\", overlay=true)\nplot(ta.ema(close, 9), \"fast\")\nplot(ta.ema(close, 21), \"slow\")\n",
        &b,
    );
    same(plot_of(&o, "fast"), plot_of(&p, "fast"));
    same(plot_of(&o, "slow"), plot_of(&p, "slow"));
    assert!(o.overlay);
    assert_eq!(o.inputs.iter().map(|i| i.title.as_str()).collect::<Vec<_>>(), ["Rapide", "slow"]);
    // le croisement vaut 1 seulement à la 1re bougie où la rapide passe au-dessus
    let fast = plot_of(&p, "fast");
    let slow = plot_of(&p, "slow");
    let cr = plot_of(&o, "cross");
    for k in 1..b.len() {
        let up = fast[k] > slow[k];
        let was = fast[k - 1] > slow[k - 1] && !slow[k - 1].is_nan();
        assert_eq!(cr[k] == 1.0, up && !was && !slow[k].is_nan(), "bougie {k}\n{}", conv.pine);
    }
}

#[test]
fn modeles_argos_v1() {
    let b = bars();
    let o = run(
        r##"indicator("EMA sur indicateur", overlay=false)
src = input.source(close, "Source (choisis un indicateur actif)")
len = input.int(9, "Longueur", minval=1, maxval=200)
plot(src, "Source", #9598a1, 1)
plot(ema(src, len), "EMA", #f0b90b, 2)"##,
        &b,
        3000,
        &HashMap::new(),
    )
    .unwrap_or_else(|e| panic!("🔗 EMA sur indicateur : {e}"));
    assert!(!o.plots.is_empty(), "🔗 EMA sur indicateur");
    let o = run(
        r##"indicator("RSI sur indicateur", overlay=false)
src = input.source(close, "Source (choisis un indicateur actif)")
len = input.int(14, "Longueur", minval=2, maxval=200)
plot(rsi(src, len), "RSI", #7e57c2, 2)
hline(70)
hline(30)"##,
        &b,
        3000,
        &HashMap::new(),
    )
    .unwrap_or_else(|e| panic!("🔗 RSI sur indicateur : {e}"));
    assert!(!o.plots.is_empty(), "🔗 RSI sur indicateur");
    let o = run(
        r##"indicator("OBV", overlay=false)
plot(obv(), "OBV", #26a69a, 2)"##,
        &b,
        3000,
        &HashMap::new(),
    )
    .unwrap_or_else(|e| panic!("OBV : {e}"));
    assert!(!o.plots.is_empty(), "OBV");
    let o = run(
        r##"indicator("Argos TVD Pro", overlay=true, alerts=true)
// à combiner avec le type de graphique « Bougies de volume » (PVSRA : climax ≥ 200 % de la moyenne 10, au-dessus de la moyenne ≥ 150 %)
vr = volume / sma(volume, 20)
plot(ema(close, 20), "EMA 20", #f0b90b, 1)
plot(ema(close, 50), "EMA 50", #2962ff, 1)
plot(ema(close, 200), "EMA 200", #b26cf0, 2)
signal(vr >= 3 and close > open, buy, "GOLD")
signal(vr >= 3 and close < open, sell, "GOLD")"##,
        &b,
        3000,
        &HashMap::new(),
    )
    .unwrap_or_else(|e| panic!("Argos TVD Pro : {e}"));
    assert!(!o.plots.is_empty(), "Argos TVD Pro");
    let o = run(
        r##"indicator("Argos TBO Pro", overlay=false, alerts=true)
// version lite : l'écart EMA 9 / EMA 21 mesuré en ATR (pas de lignes de tendance ni de divergences ici)
osc = (ema(close, 9) - ema(close, 21)) / atr(14) * 100
sg = ema(osc, 9)
plot(osc - sg, "Delta", #f23645, 1, hist)
plot(osc, "TBO", #f23645, 2)
plot(sg, "Signal", #9598a1, 1)
hline(0)
signal(crossover(osc, sg) and osc < -80, buy, "TBO")
signal(crossunder(osc, sg) and osc > 90, sell, "TBO")"##,
        &b,
        3000,
        &HashMap::new(),
    )
    .unwrap_or_else(|e| panic!("Argos TBO Pro : {e}"));
    assert!(!o.plots.is_empty(), "Argos TBO Pro");
    let o = run(
        r##"indicator("Argos Gold Scalper", overlay=true, alerts=true)
f = ema(close, 9)
s = ema(close, 21)
v = vwap()
plot(f, "EMA 9", #f0b90b, 2)
plot(s, "EMA 21", #2962ff, 2)
plot(v, "VWAP", #00bcd4, 1)
signal(crossover(f, s) and close > v and rsi(close, 7) > 50, buy, "Scalp")
signal(crossunder(f, s) and close < v and rsi(close, 7) < 50, sell, "Scalp")"##,
        &b,
        3000,
        &HashMap::new(),
    )
    .unwrap_or_else(|e| panic!("Argos Gold Scalper : {e}"));
    assert!(!o.plots.is_empty(), "Argos Gold Scalper");
    let o = run(
        r##"indicator("Argos Gold Momentum", overlay=false, alerts=true)
mo = (rsi(close, 7) + sma(stoch(9), 3)) / 2
plot(mo, "Momentum", #f23645, 2)
plot(sma(mo, 5), "Signal", #9598a1, 1)
hline(80)
hline(20)
signal(crossover(mo, 20), buy, "Mom")
signal(crossunder(mo, 80), sell, "Mom")"##,
        &b,
        3000,
        &HashMap::new(),
    )
    .unwrap_or_else(|e| panic!("Argos Gold Momentum : {e}"));
    assert!(!o.plots.is_empty(), "Argos Gold Momentum");
}

/// ⏱ Step 4: `timeframe.in_seconds` (the chart's, and given strings) and `ta.swma` (weights 1 2 2 1 over 4 values),
/// against the definitions of Pine v6.
#[test]
fn timeframe_in_seconds_et_swma() {
    let b = bars();
    let c: Vec<f64> = b.iter().map(|x| x.close).collect();
    let o = go(
        r#"//@version=6
indicator("tf", overlay = true)
plot(timeframe.in_seconds(), "cur")
plot(timeframe.in_seconds("15"), "m15")
plot(timeframe.in_seconds("1D"), "d")
plot(timeframe.in_seconds("W"), "w")
plot(timeframe.in_seconds("30S"), "s30")
plot(timeframe.in_seconds("3M"), "m3")
plot(ta.swma(close), "swma")
"#,
        &b,
    );
    let p = |t: &str| o.plots.iter().find(|p| p.title == t).unwrap().values.clone();
    let last = |t: &str| *p(t).last().unwrap();
    assert_eq!(last("cur"), 3000.0, "the test chart is 50 minutes");
    assert_eq!(
        (last("m15"), last("d"), last("w"), last("s30"), last("m3")),
        (900.0, 86_400.0, 604_800.0, 30.0, 7_776_000.0)
    );
    let attendu: Vec<f64> = (0..c.len())
        .map(|i| {
            if i < 3 {
                f64::NAN
            } else {
                (c[i - 3] + 2.0 * c[i - 2] + 2.0 * c[i - 1] + c[i]) / 6.0
            }
        })
        .collect();
    same(&p("swma"), &attendu);
}

/// 🔭 Step 4: `request.security` on the chart's symbol, higher timeframe, against a hand computation: daily bars made
/// from the chart's, the expression (close, ta.sma on the daily closes) taken at each day; never the future (a bar
/// sees the last closed day, the day's own value on the bar that closes it, the day in progress on the last bar);
/// lookahead_on gives the day in progress; another symbol or a lower timeframe is refused with a clear message.
#[test]
fn request_security_meme_symbole() {
    let b = bars();
    let o = go(
        r#"//@version=6
indicator("sec", overlay = true)
plot(request.security(syminfo.tickerid, "D", close), "dc")
plot(request.security(syminfo.tickerid, "D", ta.sma(close, 3)), "dsma")
plot(request.security(syminfo.tickerid, "D", close, lookahead = barmerge.lookahead_on), "ahead")
plot(request.security(syminfo.tickerid, "D", close[1], lookahead = barmerge.lookahead_on), "prev")
"#,
        &b,
    );
    let p = |t: &str| o.plots.iter().find(|p| p.title == t).unwrap().values.clone();
    // daily groups by hand
    let jour: Vec<i64> = b.iter().map(|x| x.time.div_euclid(86_400)).collect();
    let mut closes: Vec<f64> = vec![];
    let mut idx = vec![];
    for (i, x) in b.iter().enumerate() {
        if i == 0 || jour[i] != jour[i - 1] {
            closes.push(x.close);
        } else {
            *closes.last_mut().unwrap() = x.close;
        }
        idx.push(closes.len() - 1);
    }
    let sma3: Vec<f64> = (0..closes.len())
        .map(|h| if h < 2 { f64::NAN } else { (closes[h] + closes[h - 1] + closes[h - 2]) / 3.0 })
        .collect();
    let n = b.len();
    let vu = |v: &[f64], i: usize| {
        let h = idx[i];
        let ferme = i + 1 == n || jour[i + 1] != jour[i];
        if ferme {
            v[h]
        } else if h > 0 {
            v[h - 1]
        } else {
            f64::NAN
        }
    };
    let attendu: Vec<f64> = (0..n).map(|i| vu(&closes, i)).collect();
    same(&p("dc"), &attendu);
    same(&p("dsma"), &(0..n).map(|i| vu(&sma3, i)).collect::<Vec<_>>());
    same(&p("ahead"), &(0..n).map(|i| closes[idx[i]]).collect::<Vec<_>>());
    // the classic no-future idiom: close[1] with lookahead_on = the previous closed day on every bar of the day
    same(
        &p("prev"),
        &(0..n).map(|i| if idx[i] > 0 { closes[idx[i] - 1] } else { f64::NAN }).collect::<Vec<_>>(),
    );
    // never the future: before the bar that closes a day, the value is not that day's close
    let k = (1..n - 1).find(|&i| jour[i + 1] == jour[i] && idx[i] > 0).unwrap();
    assert_eq!(p("dc")[k], closes[idx[k] - 1]);
    // refused for now, with a clear message
    let e = run(
        "//@version=6\nindicator(\"x\")\nplot(request.security(\"BINANCE:ETHUSDT\", \"D\", close))\n",
        &b,
        3000,
        &HashMap::new(),
    )
    .unwrap_err();
    assert!(e.msg.contains("autre symbole"), "{}", e.msg);
    // (a lower timeframe reads the intrabars, `tests_references`: none given here, so na, and the app is told)
    let o = run(
        "//@version=6\nindicator(\"x\")\nplot(request.security(syminfo.tickerid, \"5\", close))\n",
        &b,
        3000,
        &HashMap::new(),
    )
    .unwrap();
    assert!(o.plots[0].values.iter().all(|v| v.is_nan()));
    assert_eq!(o.lower_tf_missing, [300]);
    let e = run(
        "//@version=6\nindicator(\"x\")\nplot(request.security(syminfo.tickerid, \"D\", close, gaps = barmerge.gaps_on))\n",
        &b,
        3000,
        &HashMap::new(),
    )
    .unwrap_err();
    assert!(e.msg.contains("gaps_on"), "{}", e.msg);
}

/// F2 (22 Nov, Argos Vector Zone Pro): a tuple through `request.security` — returned by a script function or written
/// as `[a, b]` — gives a tuple of na before the first higher bar closes (as in Pine), never « N noms à gauche, mais
/// une seule valeur à droite »; then the day's values.
#[test]
fn request_security_tuple_na_au_debut() {
    let b = bars();
    let o = go(
        r#"//@version=6
indicator("sec tuple", overlay = true)
f() =>
    a = close
    h = high
    [a, h]
[x, y] = request.security(syminfo.tickerid, "D", f())
[u, v] = request.security(syminfo.tickerid, "D", [close, low])
plot(x, "x")
plot(y, "y")
plot(u, "u")
plot(v, "v")
"#,
        &b,
    );
    let p = |t: &str| o.plots.iter().find(|p| p.title == t).unwrap().values.clone();
    let jour: Vec<i64> = b.iter().map(|x| x.time.div_euclid(86_400)).collect();
    assert!(
        p("x")[0].is_nan() && p("y")[0].is_nan() && p("u")[0].is_nan() && p("v")[0].is_nan(),
        "na before a closed day"
    );
    // the bar that closes the first day: that day's close in x and u
    let k = (0..b.len() - 1).find(|&i| jour[i + 1] != jour[i]).unwrap();
    assert_eq!((p("x")[k], p("u")[k]), (b[k].close, b[k].close));
    assert!(p("y")[k] >= p("x")[k] && p("v")[k] <= p("u")[k]);
}

/// 🔔 Step 4: `alertcondition(condition, title, message)` gives a named signal, 1 on the bars where the condition is
/// true, like the signals TradingView offers in its alert window (the app lists it with the built-in signals).
#[test]
fn alertcondition_donne_un_signal_nomme() {
    let b = bars();
    let o = go(
        r#"//@version=6
indicator("al", overlay = true)
up = ta.crossover(close, ta.sma(close, 10))
alertcondition(up, "Cassure", "Prix au-dessus de la moyenne")
alertcondition(close > open)
plot(close)
"#,
        &b,
    );
    assert_eq!(o.alertconditions.len(), 2);
    let (t, m, sig) = &o.alertconditions[0];
    assert_eq!((t.as_str(), m.as_str()), ("Cassure", "Prix au-dessus de la moyenne"));
    let c: Vec<f64> = b.iter().map(|x| x.close).collect();
    let sma = ta::sma(&c, 10);
    let attendu: Vec<f64> = (0..c.len())
        .map(|i| (i > 0 && sma[i].is_finite() && sma[i - 1].is_finite() && c[i] > sma[i] && c[i - 1] <= sma[i - 1]) as u8 as f64)
        .collect();
    assert_eq!(sig, &attendu);
    assert!(sig.contains(&1.0));
    assert_eq!(o.alertconditions[1].0, "Alerte 2");
    assert_eq!(o.alertconditions[1].2, b.iter().map(|x| (x.close > x.open) as u8 as f64).collect::<Vec<_>>());
}

/// 📋 Step 4: `table.new` / `table.cell` / `cell_set_*` / `table.clear` / `table.delete`, the table as it is at the
/// last bar (what TradingView shows); a `var` table is made once.
#[test]
fn tables_du_script() {
    let b = bars();
    let o = go(
        r#"//@version=6
indicator("tab", overlay = true)
var t = table.new(position.bottom_left, 3, 2, bgcolor = color.black, border_color = color.gray)
table.cell(t, 0, 0, "Close", text_color = color.white, text_size = size.small)
table.cell(t, 1, 0, str.tostring(close), text_color = color.white, bgcolor = close > open ? color.green : color.red)
table.cell(t, 2, 1, "x")
table.cell_set_text(t, 2, 1, "y")
table.clear(t, 0, 1, 1, 1)
var d = table.new(position.top_left, 1, 1)
table.delete(d)
plot(close)
"#,
        &b,
    );
    assert_eq!(o.tables.len(), 2, "one table per call site, even when called on every bar");
    let t = &o.tables[0];
    assert_eq!((t.position.as_str(), t.columns, t.rows), ("bottom_left", 3, 2));
    let last = b.last().unwrap();
    let c = t.cells[0][1].as_ref().unwrap();
    assert_eq!(c.text, format!("{}", last.close));
    assert!(c.bgcolor.is_some());
    assert_eq!(t.cells[0][0].as_ref().unwrap().size, "small");
    assert_eq!(t.cells[1][2].as_ref().unwrap().text, "y");
    assert!(t.cells[1][0].is_none() && t.cells[1][1].is_none(), "cleared");
    assert!(o.tables[1].deleted);
}

/// 📜 Step 4 (TBO line 8): several declarations on one line, separated by commas, like Pine v6
/// (`NONE = "None", RMA = "RMA"`), at the top and in a block.
#[test]
fn plusieurs_declarations_sur_une_ligne() {
    let b = bars();
    let o = go(
        "//@version=6\nindicator(\"d\")\na = 1, b = 2.5, c = \"x\"\nif true\n    d = a + b, e = c\n    plot(d, e)\nplot(a + b)\n",
        &b,
    );
    assert_eq!(o.plots.len(), 2);
    assert_eq!(*o.plots[0].values.last().unwrap(), 3.5);
    assert_eq!(o.plots[0].title, "x");
    assert_eq!(*o.plots[1].values.last().unwrap(), 3.5);
    // (TBO line 211) the type cast color(na): an empty colour, the point is not drawn
    let o = go(
        "//@version=6\nindicator(\"c\")\nplot(0, \"base\", color = color(na))\nplot(1, color = color(color.red))\n",
        &b,
    );
    assert!(o.plots[0].colors.iter().all(Option::is_none));
    assert!(o.plots[1].colors.iter().all(Option::is_some));
}

/// ⏱ Step 4 (ob_fvg_xau line 240): `timeframe.change(tf)`, true on the first chart bar of each new bar of `tf`
/// (UTC days here), against a hand computation.
#[test]
fn timeframe_change_comme_pine() {
    let b = bars();
    let o = go(
        "//@version=6\nindicator(\"tc\")\nplot(timeframe.change(\"D\") ? 1 : 0)\nplot(timeframe.change(\"240\") ? 1 : 0)\n",
        &b,
    );
    let attendu = |tf: i64| -> Vec<f64> {
        (0..b.len())
            .map(|i| (i == 0 || b[i].time.div_euclid(tf) != b[i - 1].time.div_euclid(tf)) as u8 as f64)
            .collect()
    };
    assert_eq!(o.plots[0].values, attendu(86_400));
    assert_eq!(o.plots[1].values, attendu(14_400));
    assert!(o.plots[0].values.iter().skip(1).any(|v| *v == 1.0));
}

/// Step 4 (Vector Zone line 631): `linefill.new` between two lines, like TradingView: it keeps its two lines (and
/// follows them, since it is drawn from where they are at the end), set_color / get_line1 / get_line2 / delete; at
/// most 500, the oldest go.
#[test]
fn linefill_entre_deux_lignes() {
    let b = bars();
    let o = go(
        r#"//@version=6
indicator("lf", overlay = true, max_lines_count = 500)
var l1 = line.new(0, 1.0, 10, 1.0)
var l2 = line.new(0, 2.0, 10, 2.0)
var f = linefill.new(l1, l2, color.new(color.green, 70))
if barstate.islast
    line.set_y1(l1, 3.0)
    linefill.set_color(f, color.red)
    g = linefill.new(linefill.get_line1(f), linefill.get_line2(f), color.blue)
    linefill.delete(g)
for i = 0 to 600
    if barstate.islast
        linefill.new(l1, l2)
plot(close)
"#,
        &b,
    );
    let f = &o.linefills[0];
    assert_eq!((f.line1, f.line2), (0, 1));
    assert_eq!(f.color & 0xFFFFFF, 0xFF5252, "set_color: red");
    assert_eq!(o.drawings[0].y1, 3.0, "the line moved: the fill follows it when drawn");
    assert!(o.linefills[1].deleted, "linefill.delete");
    let vivants = o.linefills.iter().filter(|x| !x.deleted).count();
    assert_eq!(vivants, 500, "at most 500, the oldest go");
}

/// 📜 `run_avec_variables`: the series of a top-level variable, at every bar (not only the last ones), equal to what
/// the script computes (Spring Tension's `rsiVal` = `ta.rsi(close, 14)` of this interpreter on every bar); a name the
/// script does not declare at its top level is absent, without an error; `run` gives the same output.
#[test]
fn variables_lues_par_nom() {
    let b = bars();
    let src = crate::integres::SPRING_TENSION;
    let (out, vars) = run_avec_variables(src, &b, 60, &HashMap::new(), &["rsiVal", "mfiVal", "inconnue"]).unwrap();
    let rsi = &vars["rsiVal"];
    assert_eq!(rsi.len(), b.len(), "every bar");
    let reference = go("plot(ta.rsi(close, 14))", &b).plots[0].values.clone();
    same(rsi, &reference);
    assert_eq!(vars["mfiVal"].len(), b.len());
    assert!(vars["mfiVal"].iter().any(|v| v.is_finite()), "the money flow is computed");
    assert!(!vars.contains_key("inconnue"), "an unknown name is absent");
    assert_eq!(out, run(src, &b, 60, &HashMap::new()).unwrap(), "same output as run");
}

/// 🧮 `and` as in the script's Pine version (19 Nov): in v5, « faux and ta.crossover(...) » gives the same crosses as the
/// cross computed on its own line (both sides always evaluated); in v6 the right side is skipped when the left one is
/// false (like TradingView v6), and the script gets TradingView's warning.
#[test]
fn et_ou_selon_la_version_de_pine() {
    let bars: Vec<Bar> = (0..120)
        .map(|i| {
            let c = 100.0 + if i % 4 < 2 { 1.0 } else { -1.0 } * (1.0 + (i % 7) as f64 * 0.1);
            Bar {
                time: i * 60,
                open: c,
                high: c + 0.5,
                low: c - 0.5,
                close: c,
                volume: 1.0,
            }
        })
        .collect();
    let corps = "indicator(\"t\")\ncr = ta.crossover(close, 100.0)\nplot((bar_index % 3 != 0) and ta.crossover(close, 100.0) ? 1 : 0, \"dedans\")\nplot((bar_index % 3 != 0) and cr ? 1 : 0, \"seul\")\n";
    let tracer = |v: &str| {
        let out = run(&format!("//@version={v}\n{corps}"), &bars, 60, &Default::default()).unwrap();
        let p = |t: &str| out.plots.iter().find(|x| x.title == t).unwrap().values.clone();
        (p("dedans"), p("seul"), out.notes.clone())
    };
    let (dedans5, seul5, notes5) = tracer("5");
    assert_eq!(dedans5, seul5, "v5: both sides always evaluated");
    assert!(!notes5.iter().any(|(_, m)| m.contains("chaque bougie")), "no warning in v5");
    let (dedans6, seul6, notes6) = tracer("6");
    assert_ne!(dedans6, seul6, "v6: the skipped right side misses crosses, like TradingView v6");
    let avert: Vec<&(u32, String)> = notes6
        .iter()
        .filter(|(_, m)| m.contains("ta.crossover") && m.contains("chaque bougie"))
        .collect();
    assert_eq!(avert.len(), 1, "one warning, at its line: {notes6:?}");
    assert_eq!(avert[0].0, 4, "the line of the « and » in the script");
}

/// 🧮 `argos.real_delta` (argos-vague-1): the value the app gives for each bar, `na` where it has none (NaN, or no
/// value at all); `run` gives none (`na` everywhere).
#[test]
fn delta_reel_des_trades() {
    let bars: Vec<Bar> = (0..4)
        .map(|i| Bar {
            time: i * 60,
            open: 1.0,
            high: 2.0,
            low: 0.5,
            close: 1.5,
            volume: 3.0,
        })
        .collect();
    let src = "//@version=6\nindicator(\"d\")\nplot(argos.real_delta, \"d\")\nplot(na(argos.real_delta) ? 1 : 0, \"na\")\n";
    let opts = RunOptions {
        real_delta: vec![5.0, f64::NAN, -2.0],
        last_open: false,
        ..Default::default()
    };
    let out = run_with(src, &bars, 60, &Default::default(), &opts).unwrap();
    let d = &out.plots[0].values;
    assert_eq!((d[0], d[2]), (5.0, -2.0));
    assert!(d[1].is_nan() && d[3].is_nan(), "na without a value");
    assert_eq!(out.plots[1].values, [0.0, 1.0, 0.0, 1.0]);
    let sans = run(src, &bars, 60, &Default::default()).unwrap();
    assert!(sans.plots[0].values.iter().all(|v| v.is_nan()), "run: no real trades");
}

/// ⏳ `barstate.isconfirmed` (argos-vague-1): true on every closed bar, false on the last one only when the app says
/// it is still forming; `force_overlay = true` is kept on the shapes and drawings that ask for it.
#[test]
fn bougie_en_cours_et_force_overlay() {
    let bars: Vec<Bar> = (0..5)
        .map(|i| Bar {
            time: i * 60,
            open: 1.0,
            high: 2.0,
            low: 0.5,
            close: 1.5,
            volume: 3.0,
        })
        .collect();
    let src = "//@version=6\nindicator(\"c\")\nplot(barstate.isconfirmed ? 1 : 0, \"c\")\nplotshape(barstate.isconfirmed, \"s\", force_overlay = true)\nplotshape(true, \"t\")\nif barstate.islast\n    line.new(0, 1, 4, 2, force_overlay = true)\n    label.new(4, 1, \"x\", style = label.style_circle)\n";
    let ouverte = RunOptions {
        real_delta: vec![],
        last_open: true,
        ..Default::default()
    };
    let out = run_with(src, &bars, 60, &Default::default(), &ouverte).unwrap();
    assert_eq!(out.plots[0].values, [1.0, 1.0, 1.0, 1.0, 0.0], "the forming bar is not confirmed");
    let fermee = run(src, &bars, 60, &Default::default()).unwrap();
    assert_eq!(fermee.plots[0].values, [1.0; 5], "all closed by default");
    let s: Vec<(usize, bool)> = out.shapes.iter().filter(|s| s.title == "s").map(|s| (s.bar, s.force_overlay)).collect();
    assert_eq!(s, [(0, true), (1, true), (2, true), (3, true)]);
    assert!(out.shapes.iter().filter(|s| s.title == "t").all(|s| !s.force_overlay));
    assert!(out.drawings[0].force_overlay && out.drawings[0].kind == "line");
    assert!(!out.drawings[1].force_overlay);
    assert_eq!(out.drawings[1].style, "circle");
}

/// 🎨 Argos Vague turn 2 (R1): a `fill` whose colour is `na` on a bar draws nothing on that bar (Pine), and a `fill`
/// without a `color` keeps the default blue; same for a `plot` whose colour is `na` (no segment there).
#[test]
fn fill_et_plot_na_ne_dessinent_rien() {
    let bars: Vec<Bar> = (0..6)
        .map(|i| Bar {
            time: i * 60,
            open: 1.0,
            high: 2.0,
            low: 0.5,
            close: 1.5,
            volume: 3.0,
        })
        .collect();
    let src = "//@version=6\nindicator(\"f\")\na = plot(1, \"a\", color = bar_index % 2 == 0 ? color.red : na)\nb = plot(0, \"b\")\nfill(a, b, color = bar_index % 2 == 0 ? color.red : na)\nfill(a, b)\n";
    let out = run(src, &bars, 60, &Default::default()).unwrap();
    for (i, c) in out.fills[0].2.iter().enumerate() {
        assert_eq!(c.is_some(), i % 2 == 0, "bar {i}: na → no fill");
    }
    assert!(
        out.fills[1].2.iter().all(|c| c.map(|c| c & 0xffffff) == Some(0x2962FF)),
        "no color: the default blue"
    );
    for (i, c) in out.plots[0].colors.iter().enumerate() {
        assert_eq!(c.is_some(), i % 2 == 0, "bar {i}: plot colour na");
    }
}

/// An untitled input (`len = input(14)`, the usual way of old Pine scripts) took a new name at each bar, « Input 1 »
/// then « Input 2 »… : the settings listed one input per bar, and the value chosen there counted on the first bar only.
/// Its name is now kept by its place in the source.
#[test]
fn un_input_sans_titre_garde_son_nom_a_chaque_bougie() {
    let b = bars();
    let titres = |o: &Output| o.inputs.iter().map(|i| i.title.clone()).collect::<Vec<_>>();
    let src = "indicator(\"x\")\nlen = input(5)\nmult = input.float(2.0)\nplot(len * mult)";
    let o = go(src, &b);
    assert_eq!(titres(&o), ["Input 1", "Input 2"], "one input in the settings for each one in the script");
    let choisi = |titre: &str, v: f64| HashMap::from([(titre.to_string(), InVal::Num(v))]);
    let o = run(src, &b, 3000, &choisi("Input 1", 7.0)).unwrap();
    assert!(o.plots[0].values.iter().all(|v| *v == 14.0), "the chosen value counts at every bar");
    // two untitled inputs on the same line keep their own name
    let src = "indicator(\"x\")\nf(a, b) => a * 10 + b\nplot(f(input(1), input(2)))";
    let o = run(src, &b, 3000, &choisi("Input 2", 5.0)).unwrap();
    assert_eq!(titres(&o), ["Input 1", "Input 2"]);
    assert!(o.plots[0].values.iter().all(|v| *v == 15.0));
    // an input reached at some bars only neither takes nor gives its neighbour's name: each one is named after its
    // place among the inputs written in the script (the settings list them as the script reaches them)
    let src = "indicator(\"x\")\na = input(1)\nb = 0\nif bar_index % 2 == 1\n    b := input(2)\nc = input(3)\nplot(a * 100 + c)\nplot(b)";
    let o = run(src, &b, 3000, &choisi("Input 3", 9.0)).unwrap();
    assert_eq!(titres(&o), ["Input 1", "Input 3", "Input 2"]);
    assert!(o.plots[0].values.iter().all(|v| *v == 109.0), "« Input 3 » is the input of line 6 at every bar");
    let o = run(src, &b, 3000, &choisi("Input 2", 9.0)).unwrap();
    assert!(o.plots[0].values.iter().all(|v| *v == 103.0));
    assert!(o.plots[1].values.iter().enumerate().all(|(i, v)| *v == if i % 2 == 1 { 9.0 } else { 0.0 }));
    // two untitled inputs of one line that are never reached in the same bar are two inputs: they shared one name,
    // so the settings listed one input for the two and a chosen value went to both
    let src = "indicator(\"x\")\nplot(bar_index % 2 == 0 ? input(4) : input.float(5.5))";
    let o = run(src, &b, 3000, &choisi("Input 2", 9.0)).unwrap();
    assert_eq!(titres(&o), ["Input 1", "Input 2"]);
    assert!(o.plots[0].values.iter().enumerate().all(|(i, v)| *v == if i % 2 == 0 { 4.0 } else { 9.0 }));
    // after the properties of a strategy, and next to titled inputs: still one name each
    let sans = go("strategy(\"s\")\nplot(close)", &b).inputs.len();
    let o = go("strategy(\"s\")\nlen = input.int(5, \"Longueur\")\nk = input(3)\nplot(close * k + len)", &b);
    assert_eq!(o.inputs.len(), sans + 2, "{:?}", titres(&o));
    assert_eq!(titres(&o).iter().filter(|t| t.starts_with("Input ")).count(), 1, "{:?}", titres(&o));
    assert!(
        titres(&o).iter().any(|t| t == "Input 2"),
        "the second input written in the script: {:?}",
        titres(&o)
    );
}
