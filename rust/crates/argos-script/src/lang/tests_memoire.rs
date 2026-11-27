//! 🧮 The memory budget of a script (B-062): what a script keeps is counted, and past its budget the script stops
//! with an error and its line — the app has no way to catch a failed allocation, so before the fix a few lines
//! stopped all of it. Pure: the bars of the fixtures, no network.

use super::tests::bars;
use super::value::Value;
use super::*;
use std::mem::size_of;

const MO: usize = 1 << 20;

/// `src` on the bars, with a memory budget of `octets` for this run.
fn sous(octets: usize, src: &str, b: &[Bar]) -> Result<Output, Error> {
    memoire::budget_essai(octets);
    run(src, b, 3000, &HashMap::new())
}

/// The error of a script stopped at its budget; panics on a run that ends, or on another error.
#[track_caller]
fn arrete(r: Result<Output, Error>) -> Error {
    match r {
        Ok(_) => panic!("the script ran to its end: nothing stopped it"),
        Err(e) => {
            assert!(e.msg.contains("trop de mémoire"), "{e}");
            e
        }
    }
}

#[track_caller]
fn passe(r: Result<Output, Error>) -> Output {
    r.unwrap_or_else(|e| panic!("{e}"))
}

/// The case measured before the fix: 1 000 arrays of 100 000 elements kept in one array, in one bar: 2.3 GB, where
/// each array alone is allowed. The script now stops at 256 MB, at the line that keeps them.
#[test]
fn mille_tableaux_gardes_arretent_le_script_avec_sa_ligne() {
    let b = &bars()[..5];
    let src = "//@version=6
indicator('t')
type T
    array<float> v
var tout = array.new<T>()
if barstate.isfirst
    for i = 0 to 999
        tout.push(T.new(array.new_float(100000, i)))
plot(tout.size())";
    let e = arrete(run(src, b, 3000, &HashMap::new()));
    assert_eq!(e.line, 8, "{e}");
    assert!(e.msg.contains("256 Mo"), "{e}");
    // the same script keeping 50 of them (120 MB) runs
    let o = passe(run(&src.replace("999", "49"), b, 3000, &HashMap::new()));
    assert_eq!(o.plots[0].values[4], 50.0);
}

/// A matrix made again at each bar and read back with an index the script computes is kept for every bar: 1.5 MB a
/// bar here, 750 MB on these bars before the fix. Read back one bar behind, only one is kept and the script runs.
#[test]
fn une_matrice_relue_a_un_indice_calcule_s_arrete_au_budget() {
    let b = bars();
    let src = "//@version=6
indicator('t')
m = matrix.new<float>(250, 250, close)
v = m[bar_index % 3]
plot(na(v) ? 0 : v.rows())";
    let e = arrete(run(src, &b, 3000, &HashMap::new()));
    assert!((3..=5).contains(&e.line), "{e}");
    let o = passe(run(&src.replace("bar_index % 3", "1"), &b, 3000, &HashMap::new()));
    assert_eq!(o.plots[0].values[b.len() - 1], 250.0);
}

/// One budget for the script and its sub-runs: `request.security` runs the script again on other bars, and what
/// that run keeps adds to what the chart's run keeps (144 MB each here: each alone passes, together they do not).
#[test]
fn un_sous_run_compte_dans_le_budget_du_script() {
    let b = &bars()[..40];
    let src = "//@version=6
indicator('t')
type T
    array<float> v
var tout = array.new<T>()
if barstate.isfirst
    for i = 1 to 60
        tout.push(T.new(array.new_float(100000, i)))
h = request.security(syminfo.tickerid, '100', tout.size())
plot(h)";
    let e = arrete(run(src, b, 3000, &HashMap::new()));
    assert_eq!(e.line, 8, "{e}");
    // 96 MB each: the script runs, and the sub-run gave its values
    let o = passe(run(&src.replace("60", "40"), b, 3000, &HashMap::new()));
    assert_eq!(o.plots[0].values[39], 40.0);
    // without the sub-run, the 144 MB of the chart's run pass
    passe(run(
        &src.replace("request.security(syminfo.tickerid, '100', tout.size())", "tout.size()"),
        b,
        3000,
        &HashMap::new(),
    ));
}

/// The count is the real size, in both directions: an array of 100 000 numbers is 2.4 MB (24 bytes a value).
#[test]
fn le_compte_suit_la_taille_reelle() {
    let b = &bars()[..30];
    let src = "//@version=6
indicator('t')
var a = array.new_float(100000, 1)
var total = 0.0
for i = 0 to 20000
    total += i
plot(a.size() + total)";
    assert_eq!(size_of::<value::Value>(), memoire::V);
    let juste = 100_000 * memoire::V;
    arrete(sous(juste - juste / 8, src, b));
    passe(sous(juste + juste / 2, src, b));
}

/// Everything a script keeps is counted, not only its arrays: each of these scripts keeps a few MB in another place
/// and is stopped under a budget of 1 MB; under 64 MB it runs.
#[test]
fn chaque_endroit_qui_garde_de_la_memoire_est_compte() {
    let b = bars();
    let cas: [(&str, &str); 16] = [
        (
            "texts in an array",
            "var a = array.new_string()\na.push(str.repeat('x', 5000) + str.tostring(bar_index))",
        ),
        ("texts in a map", "var m = map.new<int, string>()\nm.put(bar_index, str.repeat('x', 5000))"),
        ("objects", "type T\n    string s\nvar a = array.new<T>()\na.push(T.new(str.repeat('x', 5000)))"),
        (
            "a matrix that grows",
            "var m = matrix.new<float>(0, 150, 0)\nm.add_row(m.rows(), array.new_float(150, close))",
        ),
        (
            "texts given back in a tuple",
            "f() => [str.repeat('x', 5000), close]\nvar a = array.new_string()\n[s, c] = f()\na.push(s)",
        ),
        (
            "the results of a lambda, while it runs",
            "a = array.new_float(2000, 1)\nb = a.map((x) => str.repeat('y', 2000))",
        ),
        (
            "the history of a variable",
            "s = str.repeat('x', 5000) + str.tostring(bar_index)\nplot(str.length(s[bar_index % 400]))",
        ),
        (
            "the history of an expression",
            "plot(str.length((str.repeat('x', 5000) + str.tostring(bar_index))[bar_index % 400]))",
        ),
        ("the memory of a ta.* call in a loop", "x = 0.0\nfor i = 0 to 999\n    x += ta.sma(close, 5)"),
        ("logs", "log.info(str.repeat('x', 5000))"),
        // (the 500 labels a script keeps; the deleted ones keep nothing since B-064: `tests_dessins`)
        ("drawings", "label.new(bar_index, close, str.repeat('x', 20000))"),
        ("shapes", "for i = 0 to 99\n    plotshape(true, text = 'x')"),
        (
            "table cells",
            "var t = table.new(position.top_right, 20, 50)\nfor i = 0 to 19\n    table.cell(t, i, bar_index % 50, str.repeat('x', 2000))",
        ),
        (
            "inputs in a loop",
            "for i = 0 to 30\n    x = input.int(1, 'n', tooltip = str.repeat('t', 50000))",
        ),
        // (the settings keep a copy of the default text of each input, and its title three times)
        (
            "the default text of an input",
            "var d = str.repeat('x', 50000)\nfor i = 0 to 30\n    x = input.string(d, 't' + str.tostring(i))",
        ),
        (
            "the title of an input",
            "var t = str.repeat('x', 20000)\nfor i = 0 to 30\n    x = input.int(1, t + str.tostring(i))",
        ),
    ];
    for (nom, corps) in cas {
        let src = format!("//@version=6\nindicator('t')\n{corps}\nplot(close)");
        let r = sous(MO, &src, &b);
        assert!(matches!(&r, Err(e) if e.msg.contains("trop de mémoire")), "{nom}: {:?}", r.map(|_| ()));
        if let Err(e) = sous(64 * MO, &src, &b) {
            panic!("{nom}: {e}");
        }
    }
}

/// What the settings keep of a new input is told when it is kept: on one bar nothing else brings the count. 31
/// inputs with a default text of 50 000 characters, or with a title of 20 000 (kept three times), keep 1.5 MB or
/// more: the script stops in the loop that makes them, before the bar ends.
#[test]
fn les_textes_d_un_input_amenent_le_compte_dans_la_bougie() {
    let b = &bars()[..1];
    for (nom, corps) in [
        (
            "the default text",
            "var d = str.repeat('x', 50000)\nfor i = 0 to 30\n    x = input.string(d, 't' + str.tostring(i))",
        ),
        (
            "the title",
            "var t = str.repeat('x', 20000)\nfor i = 0 to 30\n    x = input.int(1, t + str.tostring(i))",
        ),
    ] {
        let src = format!("//@version=6\nindicator('t')\n{corps}\nplot(close)");
        let e = arrete(sous(MO, &src, b));
        assert!(e.line == 4 || e.line == 5, "{nom}: {e}");
        let o = passe(sous(64 * MO, &src, b));
        assert_eq!(o.inputs.len(), 31, "{nom}");
    }
}

/// What only the interpreter holds in the middle of a calculation is counted like the rest. Before the fix a count saw
/// what the variables and the collections held, nothing else: a tuple of tuples of fresh texts given back by
/// functions, the arguments of a call waiting for the last one, the copy a built-in makes of an array or of a map
/// for the time of a lambda or of a loop. 14 lines kept 839 MB that way with no error; a few more stopped the app.
#[test]
fn ce_que_le_calcul_en_cours_tient_est_compte() {
    let b = &bars()[..30];
    let cas: [(&str, usize, &str); 6] = [
        (
            "texts in a tuple of tuples no variable holds (243 of 20 KB)",
            2,
            "mk() => str.repeat('x', 20000)
t3() => [mk(), mk(), mk()]
t9() => [t3(), t3(), t3()]
t27() => [t9(), t9(), t9()]
t81() => [t27(), t27(), t27()]
t243() => [t81(), t81(), t81()]
if barstate.islast
    t243()",
        ),
        (
            "the same, kept in an array from the first bar on: nothing more is made afterwards",
            2,
            "mk() => str.repeat('x', 20000)
t3() => [mk(), mk(), mk()]
t9() => [t3(), t3(), t3()]
t27() => [t9(), t9(), t9()]
t81() => [t27(), t27(), t27()]
t243() => [t81(), t81(), t81()]
var garde = array.new<float>()
if barstate.isfirst
    array.push(garde, t243())",
        ),
        (
            "fresh texts waiting in the arguments of a call, 30 levels deep (300 of 20 KB)",
            2,
            "mk() => str.repeat('x', 20000)
g(a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, r) => r
f(int n) =>
    n <= 0 ? 0 : g(mk(), mk(), mk(), mk(), mk(), mk(), mk(), mk(), mk(), mk(), f(n - 1))
if barstate.islast
    f(30)",
        ),
        (
            "100 texts joined in one expression, with no statement and no call between them (10 MB, let go of at once)",
            2,
            "var s = str.repeat('x', 50000)
t() => [s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s, s + s]
if barstate.islast
    t()",
        ),
        (
            "the copy of an array a lambda function works on, 2 deep in 10 levels (20 copies of 480 KB)",
            4,
            "var a = array.new<float>(20000, 1.0)
f(int n) =>
    n <= 0 ? true : array.some(a, (x) => array.some(a, (y) => f(n - 1)))
if barstate.islast
    f(10)",
        ),
        (
            "the copy of a map a for … in goes through, 2 deep in 20 levels (40 copies of 240 KB)",
            4,
            "var m = map.new<int, int>()
if barstate.isfirst
    for i = 1 to 5000
        map.put(m, i, i)
g(int n) =>
    int r = 0
    if n > 0
        for [k1, v1] in m
            for [k2, v2] in m
                r := g(n - 1)
                break
            break
    r
if barstate.islast
    g(20)",
        ),
    ];
    for (nom, mo, corps) in cas {
        let src = format!("//@version=6\nindicator('t')\n{corps}\nplot(close)");
        let r = sous(mo * MO, &src, b);
        assert!(matches!(&r, Err(e) if e.msg.contains("trop de mémoire")), "{nom}: {:?}", r.map(|_| ()));
        // (stopped on the way, not once everything was made: never twice the budget alive at once)
        assert!(memoire::pic_vivant() < 2 * mo * MO, "{nom}: {} bytes alive at once", memoire::pic_vivant());
        if let Err(e) = sous(64 * MO, &src, b) {
            panic!("{nom}: {e}");
        }
    }
}

/// A text an output keeps without any call having made it as a value is counted too: the line of a log written
/// from a pattern (100 times longer than its pieces here), the texts of a copied label.
#[test]
fn une_sortie_ecrite_par_un_motif_ou_copiee_est_comptee() {
    let b = &bars()[..30];
    let cas: [(&str, &str); 2] = [
        (
            "log lines written from a pattern (6 MB kept, from pieces of 1 KB)",
            "var p = str.repeat('{0}', 100)
var x = str.repeat('x', 1000)
log.info(p, x)
log.warning(p, x)",
        ),
        (
            "copies of a label (6 MB of texts kept)",
            "var string t = str.repeat('x', 50000)
var label l = label.new(bar_index, close, t)
for k = 1 to 4
    label.copy(l)",
        ),
    ];
    for (nom, corps) in cas {
        let src = format!("//@version=6\nindicator('t')\n{corps}\nplot(close)");
        let r = sous(2 * MO, &src, b);
        assert!(matches!(&r, Err(e) if e.msg.contains("trop de mémoire")), "{nom}: {:?}", r.map(|_| ()));
        if let Err(e) = sous(64 * MO, &src, b) {
            panic!("{nom}: {e}");
        }
    }
}

/// The id of a trailing exit is shared by the trades it trails. Before the fix every open trade kept its own copy
/// of the id of every trailing exit, all made in one step of the broker, between two bars: 450 trades under 450
/// exits were 202 500 copies, 800 MB. Here 40 trades under 40 exits with ids of 1 000 characters: 1.6 MB of copies
/// before, 40 KB of ids now, and the script runs under a budget of 512 KB.
#[test]
fn l_id_d_une_sortie_suiveuse_est_partage_par_ses_trades() {
    let b = &bars()[..40];
    let src = "//@version=6
strategy('s')
var string long = str.repeat('i', 990)
if bar_index == 10
    for k = 1 to 40
        strategy.order('L' + str.tostring(k), strategy.long, 0.0001)
if bar_index == 11
    for k = 1 to 40
        strategy.exit(str.tostring(k) + long, trail_price = 0, trail_offset = 1000000000)
// (enough is made at each bar for what the broker keeps to be counted)
if bar_index > 12
    for i = 1 to 20
        x = str.repeat('y', 20000)
plot(strategy.opentrades)";
    let o = passe(sous(MO / 2, src, b));
    assert_eq!(o.plots[0].values[39], 40.0);
}

/// The texts and the tuples alive are known to the byte: counted when made, taken off when their last holder lets
/// go of them. Once a run is over and its interpreter gone, none is left, whatever held them: variables,
/// histories, collections, objects, tuples in tuples, collections that hold themselves, a sub-run, a script
/// stopped by an error in the middle of a bar.
#[test]
fn le_compte_des_textes_et_des_tuples_vivants_revient_a_zero() {
    let b = &bars()[..60];
    assert_eq!(memoire::vivant(), 0);
    let scripts = [
        "s = str.tostring(close) + 'x'\nvar a = array.new_string()\na.push(s)\nplot(str.length(s[3]))",
        "f() => [str.tostring(close), [open, 'deux'], close]\n[x, y, z] = f()\nvar m = map.new<string, string>()\nm.put(x, x + x)\nplot(z)",
        "type T\n    string nom\n    T autre\nvar T t = na\nt := T.new(str.tostring(bar_index), t)\nu = T.new('a', na)\nu.autre := u\nplot(close)",
        "var x = 0\nx := [x, 'texte']\na = array.new<float>(3, close)\na.push(a)\nplot(close)",
        "h = request.security(syminfo.tickerid, '100', str.length(str.tostring(close) + 'h'))\nplot(h)",
        "l = label.new(bar_index, close, str.tostring(close))\nlog.info('{0} et {1}', 'a', str.tostring(open))\nplot(close)",
        "a = array.from('a', 'b', 'c')\nn = 0\nfor [k, v] in a\n    n += str.length(v)\nb2 = a.map((x) => x + x)\nplot(n)",
        "s = str.repeat('x', 1000)\nvar a = array.new_string()\na.push(s)\nif bar_index == 30\n    plot(inconnu)\nplot(close)",
        "var a = array.new_string()\nfor i = 0 to 99\n    a.push(str.repeat('x', 100000))\nplot(close)",
    ];
    for (k, corps) in scripts.iter().enumerate() {
        let src = format!("//@version=6\nindicator('t')\n{corps}");
        let r = sous(64 * MO, &src, b);
        // (the two last ones are stopped, by an error of the script and by the budget; the others run)
        assert_eq!(r.is_ok(), k < 7, "script {k}: {:?}", r.as_ref().map(|_| ()));
        drop(r);
        assert_eq!(memoire::vivant(), 0, "script {k}");
    }
}

/// A value that holds a value that holds a value… is freed in a loop, however long the chain (B-070). Left to the
/// language, the freeing went down the chain by recursion: a linked list of 30 000 objects let go of at once,
/// or a tuple wrapped 100 000 times, overflowed the stack, and the whole app stopped.
#[test]
fn une_longue_chaine_de_valeurs_se_libere_sans_recursion() {
    let b = &bars()[..300];
    let cas: [(&str, &str); 9] = [
        (
            "a linked list of 200 000 objects, let go of in the middle of the run",
            "type Noeud
    Noeud suivant
var Noeud tete = na
if bar_index < 200
    for i = 1 to 1000
        tete := Noeud.new(tete)
if bar_index == 250
    tete := na",
        ),
        (
            "the same list, kept until the end of the run",
            "type Noeud
    Noeud suivant
var Noeud tete = na
if bar_index < 200
    for i = 1 to 1000
        tete := Noeud.new(tete)",
        ),
        (
            "an array that holds the array before it, 100 000 deep, replaced in the middle of the run",
            "var x = array.new<float>(0)
if bar_index < 100
    for i = 1 to 1000
        x := array.from(x)
if bar_index == 200
    x := array.new<float>(0)",
        ),
        (
            "a matrix that holds the matrix before it, 100 000 deep, replaced in the middle of the run",
            "var m = matrix.new<float>(1, 1, 0)
var n = matrix.new<float>(1, 1, 0)
if bar_index < 100
    for i = 1 to 1000
        n := matrix.new<float>(1, 1, 0)
        matrix.set(n, 0, 0, m)
        m := n
if bar_index == 200
    n := matrix.new<float>(1, 1, 0)
    m := matrix.new<float>(1, 1, 0)",
        ),
        (
            "a map that holds the map before it, 100 000 deep, replaced in the middle of the run",
            "var m = map.new<int, float>()
var n = map.new<int, float>()
if bar_index < 100
    for i = 1 to 1000
        n := map.new<int, float>()
        map.put(n, 0, m)
        m := n
if bar_index == 200
    n := map.new<int, float>()
    m := map.new<int, float>()",
        ),
        (
            "a tuple that holds the tuple before it, 100 000 deep, kept in a variable until the end",
            "var x = 0
if bar_index < 100
    for i = 1 to 1000
        x := [x]",
        ),
        (
            "the same chain of tuples, kept in an array, then let go of",
            "var a = array.new<float>(1, 0)
if bar_index < 100
    for i = 1 to 1000
        array.set(a, 0, [array.get(a, 0)])
if bar_index == 200
    array.clear(a)",
        ),
        (
            "a map whose key is the map before it, 100 000 deep, let go of in the middle of the run",
            "var m = map.new<int, float>()
var m2 = map.new<int, float>()
if bar_index < 100
    for i = 1 to 1000
        m2 := map.new<int, float>()
        m2.put(m, 0)
        m := m2
if bar_index == 200
    m2 := map.new<int, float>()
    m := map.new<int, float>()",
        ),
        (
            "a matrix that holds the matrix before it, and an object that holds an array that holds an object",
            "type Boite
    array<float> dedans
var mat = matrix.new<float>(1, 1, 0)
var m2 = matrix.new<float>(1, 1, 0)
var t = array.new<float>(0)
var Boite bo = na
if bar_index < 100
    for i = 1 to 500
        m2 := matrix.new<float>(1, 1, 0)
        m2.set(0, 0, mat)
        mat := m2
        t := array.new<float>(0)
        t.push(bo)
        bo := Boite.new(t)
if bar_index == 200
    m2 := matrix.new<float>(1, 1, 0)
    t := array.new<float>(0)
    mat := matrix.new<float>(1, 1, 0)
    bo := na",
        ),
    ];
    for (nom, corps) in cas {
        let src = format!("//@version=6\nindicator('t')\n{corps}\nplot(close)");
        if let Err(e) = run(&src, b, 3000, &HashMap::new()) {
            panic!("{nom}: {e}");
        }
        assert_eq!(memoire::vivant(), 0, "{nom}");
    }
}

/// A strategy: its orders, its trades and its marks are counted too, and the texts of an order stay short (the
/// broker copies them at each fill, between two statements of the script).
#[test]
fn les_ordres_d_une_strategie_sont_comptes() {
    let b = bars();
    let src = "//@version=6
strategy('s', pyramiding = 100)
for i = 0 to 9
    strategy.order(str.repeat('e', 900) + str.tostring(i), strategy.long, 1, comment = str.repeat('c', 900))
if bar_index % 2 == 1
    strategy.close_all(comment = str.repeat('s', 900))";
    arrete(sous(2 * MO, src, &b));
    passe(sous(256 * MO, src, &b));
    let e = run(&src.replace("'e', 900", "'e', 1001"), &b, 3000, &HashMap::new()).unwrap_err();
    assert!(e.msg.contains("1 000 caractères") && e.line == 4, "{e}");
    let e = run(&src.replace("'s', 900", "'s', 1001"), &b, 3000, &HashMap::new()).unwrap_err();
    assert!(e.msg.contains("1 000 caractères") && e.line == 6, "{e}");
    // every text the broker keeps, by name or by place, and none of the others: an `alert_message` is a long line
    // of JSON on many scripts, and nothing keeps it
    let l = "str.repeat('x', 1001)";
    for (appel, nom) in [
        (format!("strategy.entry({l}, strategy.long)"), "id"),
        (
            format!("strategy.entry('a', strategy.long, oca_name = {l}, oca_type = strategy.oca.cancel)"),
            "oca_name",
        ),
        (format!("strategy.entry('a', strategy.long, comment = {l})"), "comment"),
        (format!("strategy.exit({l}, 'a', profit = 10)"), "id"),
        (format!("strategy.exit('x', {l}, profit = 10)"), "from_entry"),
        (format!("strategy.exit('x', 'a', profit = 10, comment = {l})"), "comment"),
        (format!("strategy.close({l})"), "id"),
        (format!("strategy.close('a', {l})"), "comment"),
        (format!("strategy.close_all({l})"), "comment"),
        (format!("strategy.cancel({l})"), "id"),
        ("strategy.entry(array.new<float>(3, 0), strategy.long)".to_string(), "id"),
    ] {
        let e = run(&format!("//@version=6\nstrategy('s')\n{appel}"), &b, 3000, &HashMap::new()).unwrap_err();
        assert!(
            e.msg.contains("1 000 caractères") && e.msg.contains(&format!("« {nom} »")) && e.line == 3,
            "{appel}: {e}"
        );
    }
    let libre = format!(
        "//@version=6\nstrategy('s')\nif bar_index % 4 == 0\n    strategy.entry('a', strategy.long, alert_message = {l})\n    \
         strategy.exit('x', 'a', profit = 10, alert_message = {l}, comment_profit = 'tp')\nif bar_index % 4 == 2\n    \
         strategy.close('a', alert_message = {l})\n    strategy.close_all(alert_message = {l})"
    );
    passe(run(&libre, &b, 3000, &HashMap::new()));
}

/// A map holds 50 000 pairs at most, as on TradingView (it had no limit), and the pieces of `str.split` are an array
/// with the limit of an array (100 002 pieces came out of a text of 100 000 characters).
#[test]
fn une_map_et_les_morceaux_d_un_texte_ont_leur_limite() {
    let b = &bars()[..2];
    let src = "//@version=6
indicator('t')
var m = map.new<int, float>()
if barstate.isfirst
    for i = 1 to 50000
        m.put(i, close)
if barstate.islast
    m.put(1, open)
    m.put(50001, open)
plot(m.size())";
    let e = run(src, b, 3000, &HashMap::new()).unwrap_err();
    assert_eq!((e.line, e.msg.as_str()), (9, "map.put : 50 000 paires au plus, comme sur TradingView"));
    let o = passe(run(&src.replace("    m.put(50001, open)\n", ""), b, 3000, &HashMap::new()));
    assert_eq!(o.plots[0].values[1], 50_000.0);
    let src = "//@version=6
indicator('t')
s = str.repeat('a', 100000)
plot(str.split(s, '').size())";
    let e = run(src, b, 3000, &HashMap::new()).unwrap_err();
    assert_eq!((e.line, e.msg.as_str()), (4, "str.split : 100 000 éléments au plus, comme sur TradingView"));
    let o = passe(run(&src.replace("100000", "99998"), b, 3000, &HashMap::new()));
    assert_eq!(o.plots[0].values[1], 100_000.0);
}

/// A collection that holds itself, two objects that hold each other: nothing frees them while they hold on, so the
/// run empties them when it ends. Before the fix they stayed in memory until the app was closed.
#[test]
fn une_collection_qui_se_tient_elle_meme_est_liberee_a_la_fin() {
    let b = &bars()[..20];
    let src = "//@version=6
indicator('t')
type N
    N autre
    array<float> v
a = array.new<float>(1000, close)
a.push(a)
m = map.new<string, float>()
m.put('moi', m)
x = N.new(na, a)
y = N.new(x, a)
x.autre := y
mat = matrix.new<float>(2, 2, 0)
mat.set(0, 0, mat)
plot(a.size())";
    passe(run(src, b, 3000, &HashMap::new()));
    // (the 5 collections of the last bar were held when the run was closed, by its variables and by each other;
    // the ones of the bars before held each other only)
    let (tenues, encore) = memoire::restes();
    assert!(tenues >= 5 * b.len(), "{tenues}");
    assert_eq!(encore, 0);
    // (and a script stopped by an error leaves nothing either)
    let e = run(&format!("{src}\nplot(inconnu)"), b, 3000, &HashMap::new()).unwrap_err();
    assert!(e.msg.contains("inconnu"), "{e}");
    assert_eq!(memoire::restes().1, 0);
}

/// The results do not change under a budget the script never reaches, however small the steps between two counts.
#[test]
fn compter_ne_change_aucun_resultat() {
    let b = bars();
    let src = "//@version=6
indicator('t')
type Z
    float haut
    array<float> prix
var zones = array.new<Z>()
if bar_index % 10 == 0
    zones.push(Z.new(high, array.from(open, close)))
if zones.size() > 20
    zones.shift()
s = 0.0
for z in zones
    s += z.haut + z.prix.get(1)
m = ta.ema(close, 9)
t = str.tostring(m, '#.##')
plot(s)
plot(m[3] + str.length(t[1]))";
    let libre = passe(run(src, &b, 3000, &HashMap::new()));
    // (1 MB: about a hundred counts on these bars; the script keeps a few hundred KB)
    let serre = passe(sous(MO, src, &b));
    assert_eq!(libre.plots.len(), 2);
    for (p, q) in libre.plots.iter().zip(&serre.plots) {
        assert_eq!(format!("{:?}", p.values), format!("{:?}", q.values));
    }
}

/// The budget follows the chart: 256 MB plus 1 KB a bar. A script keeps series as long as the chart with no way
/// around it (a plot is 16 bytes a bar), and with a flat budget 64 plotted averages were refused on 200 000 bars,
/// where they ran before the budget existed. The run gets the budget of its own chart: the error names it.
#[test]
fn le_budget_suit_la_longueur_de_la_chart() {
    assert_eq!(memoire::budget(0), 256 * MO);
    assert_eq!(memoire::budget(200_000), 256 * MO + 200_000 * 1024);
    // 64 plots of 200 000 bars with their averages: 307 MB measured
    assert!(memoire::budget(200_000) > 400 * MO);
    let b = bars();
    let longue: Vec<Bar> = (0..4096)
        .map(|k| Bar {
            time: b[0].time + 3000 * k as i64,
            ..b[k % b.len()]
        })
        .collect();
    let src = "//@version=6
indicator('t')
type B
    array<float> v
var tout = array.new<B>()
if barstate.isfirst
    for i = 0 to 400
        tout.push(B.new(array.new_float(100000, 1.0)))
plot(close)";
    let e = arrete(run(src, &b[..5], 3000, &HashMap::new()));
    assert!(e.msg.contains("plus de 256 Mo") && e.line == 8, "{e}");
    let e = arrete(run(src, &longue, 3000, &HashMap::new()));
    assert!(e.msg.contains("plus de 260 Mo") && e.line == 8, "{e}");
}

/// A script that sits under its budget is not counted again and again. A count goes through everything the script
/// keeps: it comes only when what is alive may have reached the budget, and what is made and let go of at once (a
/// text read by a calculation, an array of one bar) does not bring it closer. Before, 200 texts measured at each
/// bar, or 2 000 calls of `int()`, brought a count every few bars: a script ran 3 to 10 times slower.
#[test]
fn un_script_sous_son_budget_n_est_pas_recompte() {
    let b = &bars()[..200];
    // 4.8 MB kept under a budget of 8 MB, then at each bar:
    let cas = [
        ("texts read by a calculation", "s = 0.0\nfor i = 0 to 19\n    t = str.repeat('a', 100000)\n    s += str.length(t)", 2),
        ("arrays of one bar", "s = 0.0\nfor i = 0 to 4\n    t = array.new_float(50000, 0.0)\n    s += t.size()", 2),
        ("maps, matrices and objects of one bar", "s = 0.0\nfor i = 0 to 99\n    m = map.new<int, float>()\n    x = matrix.new<float>(20, 20, 0.0)\n    p = chart.point.from_index(i, close)\n    s += m.size()", 2),
        ("calls that compute", "s = 0.0\nfor i = 0 to 1999\n    s += int(i) + math.abs(i)", 12),
        ("texts joined after an output", "t = str.repeat('a', 50000)\nplotshape(false, title = 'x')\ns = 0.0\nfor i = 0 to 19\n    u = 'b' + t\n    s += 1", 2),
        ("a drawing read back", "var l = label.new(0, 0, 'x')\ns = 0.0\nfor i = 0 to 999\n    s += label.get_x(l)", 6),
        ("texts kept from the first bar", "var t = array.new_string()\nif barstate.isfirst\n    for i = 0 to 19\n        t.push(str.repeat('a', 99980 + i))\ns = 0.0\nfor i = 0 to 999\n    s += i", 4),
    ];
    for (nom, corps, plus) in cas {
        let src = format!("//@version=6\nindicator('t')\nvar g1 = array.new_float(100000, 0.0)\nvar g2 = array.new_float(100000, 0.0)\n{corps}\nplot(s)");
        passe(sous(8 * MO, &src, b));
        assert!(memoire::comptes() <= plus, "{nom}: counted {} times in 200 bars", memoire::comptes());
    }
}

/// A list that grows tells what it took to the byte: a full list takes as much room again in one step. 98 full
/// arrays of 100 000 values, one `push` on each, took 235 MB more for 12 KB told, and nothing counted them.
#[test]
fn une_liste_qui_grandit_dit_ce_qu_elle_prend() {
    let b = &bars()[..3];
    // (what a run told as made besides the collections it made, with the line that makes a full list grow and
    // with a line that changes the same list in place)
    let dit = |debut: &str, ligne: &str, temoin: &str| {
        let src = |l: &str| format!("//@version=6\nindicator('t')\n{debut}\nif barstate.islast\n    {l}\nplot(close)");
        passe(sous(512 * MO, &src(temoin), b));
        let sans = memoire::cree();
        passe(sous(512 * MO, &src(ligne), b));
        assert_eq!(memoire::comptes(), 0);
        memoire::cree().saturating_sub(sans)
    };
    let v = memoire::V;
    let plein = "var a = array.new_float(99999, 1.0)";
    for ligne in ["a.push(2.0)", "a.unshift(2.0)", "a.insert(5, 2.0)", "a.concat(array.from(2.0))"] {
        let plus = dit(plein, ligne, "a.set(5, 2.0)");
        assert!((99_999 * v..100_100 * v).contains(&plus), "{ligne}: {plus} bytes told");
    }
    // the first push on an empty list: its first 4 places and the block that holds them
    let plus = dit("var a = array.new_float(0)", "a.push(2.0)", "a.clear()");
    assert_eq!(plus, 4 * v + 32, "a first push");
    // a map and a matrix: the same, with their own places
    let paires = "var m = map.new<int, float>()\nif barstate.isfirst\n    for i = 0 to 4095\n        m.put(i, 1.0)";
    let plus = dit(paires, "m.put(5000, 1.0)", "m.put(5, 2.0)");
    assert!((4096 * 2 * v..4200 * 2 * v).contains(&plus), "map.put: {plus} bytes told");
    let plus = dit("var x = matrix.new<float>(999, 100, 1.0)", "x.add_row()", "x.set(0, 0, 2.0)");
    assert!((99_900 * v..100_100 * v).contains(&plus), "matrix.add_row: {plus} bytes told");
    let plus = dit("var x = matrix.new<float>(100, 999, 1.0)", "x.add_col()", "x.set(0, 0, 2.0)");
    assert!(plus >= 100_000 * v, "matrix.add_col: {plus} bytes told");
    // the results of a lambda, one by one (the 5 000 turns of the lambda tell 16 bytes each)
    let debut = "var a = array.new_float(5000, 1.0)\nvar r = array.new_float(0)";
    let plus = dit(debut, "r := a.filter((x) => x > 0)", "r := a");
    assert!((5000 * (v + 16)..17_000 * v).contains(&plus), "array.filter: {plus} bytes told");
    // end to end: 4 full arrays are 9.6 MB under a budget of 12 MB; one push on each makes them 19.2 MB
    let src = "//@version=6
indicator('t')
type B
    array<float> v
var tout = array.new<B>()
if barstate.isfirst
    for i = 0 to 3
        tout.push(B.new(array.new_float(99999, 1.0)))
    for bo in tout
        bo.v.push(2.0)
plot(close)";
    let e = arrete(sous(12 * MO, src, b));
    assert!(e.line >= 9, "{e}");
    passe(sous(12 * MO, &src.replace("bo.v.push(2.0)", "bo.v.set(0, 2.0)"), b));
}

/// An expression read back with a written `[k]` keeps its last `k + 1` bars, not one value a bar: 200 of them on a
/// long chart kept 24 bytes a bar each for nothing. What it gives is what the place that keeps every bar gives.
#[test]
fn une_expression_relue_avec_un_nombre_ecrit_garde_k_bougies() {
    let b = bars();
    // the same expression read with a written number and with a computed one (which keeps every bar): at every bar,
    // every third bar, and after gaps longer than `k`
    for k in [0usize, 1, 2, 7, 40] {
        for quand in ["true", "bar_index % 3 == 0", "bar_index % 50 < 3", "bar_index > 100 and bar_index % 2 == 1"] {
            let src = format!(
                "//@version=6\nindicator('t')\nk = {k} + (close > 0 ? 0 : 1)\nfloat e = na\nfloat c = na\nfloat t = na\n\
                 if {quand}\n    e := (close * 2 + bar_index)[{k}]\n    c := (close * 2 + bar_index)[k]\n    \
                 h = (array.from(close, open))[{k}]\n    t := na(h) ? na : array.get(h, 1)\nplot(e)\nplot(c)\nplot(t)\nplot(open[{k}])"
            );
            let o = passe(run(&src, &b, 3000, &HashMap::new()));
            assert_eq!(format!("{:?}", o.plots[0].values), format!("{:?}", o.plots[1].values), "k = {k}, {quand}");
            // (passed at every bar, the place gives a value from bar `k` on)
            let finis = o.plots[0].values.iter().filter(|v| v.is_finite()).count();
            assert!(quand != "true" || finis == b.len() - k, "k = {k}, {quand}: {finis}");
            // (a collection read back: the open of `k` bars before, where the place was passed `k` bars before)
            for (i, v) in o.plots[2].values.iter().enumerate() {
                assert!(v.is_nan() || *v == o.plots[3].values[i], "k = {k}, {quand}, bar {i}");
            }
        }
    }
    // an array made again at each bar and read one bar behind: 2 arrays are kept (2.4 MB each), not the 18 the
    // place holds before it trims its list
    let src = "//@version=6\nindicator('t')\na = (array.new_float(100000, close))[1]\nplot(na(a) ? 0 : a.size())";
    let o = passe(sous(16 * MO, src, &b));
    assert_eq!(o.plots[0].values[b.len() - 1], 100000.0);
    // 200 places on 504 bars: 2.4 MB with one value a bar each, 150 KB with what can be read
    let places: String = (0..200).map(|i| format!("s += (close + {i})[1]\n")).collect();
    let src = format!("//@version=6\nindicator('t')\ns = 0.0\n{places}plot(s)");
    let o = passe(sous(MO, &src, &b));
    assert!(o.plots[0].values[100].is_finite());
    let partout = src.replace("[1]", "[un]").replace("s = 0.0", "s = 0.0\nun = close > 0 ? 1 : 2");
    arrete(sous(MO, &partout, &b));
}

/// A drawing changed in place keeps the text it is given: `label.set_text` and the like add no entry to the
/// outputs, but what they read stays (a style, a size or an extension is kept as it is written too).
#[test]
fn un_dessin_change_sur_place_garde_son_texte() {
    let b = &bars()[..5];
    let long = "str.repeat('a', 100000)";
    let dessins = |genre: &str, faire: &str, changer: &str| {
        format!(
            "var ds = array.new<{genre}>()\nif barstate.isfirst\n    for i = 0 to 39\n        ds.push({faire})\n    for d in ds\n        {changer}(d, {long})"
        )
    };
    let table = |changer: &str| {
        format!(
            "var t = table.new(position.top_right, 8, 5)\nif barstate.isfirst\n    for i = 0 to 39\n        table.cell(t, i % 8, i / 8, 'x')\n    for i = 0 to 39\n        {changer}(t, i % 8, i / 8, {long})"
        )
    };
    let cas = [
        dessins("label", "label.new(i, close, 'x')", "label.set_text"),
        dessins("label", "label.new(i, close, 'x')", "label.set_size"),
        dessins("label", "label.new(i, close, 'x')", "label.set_style"),
        dessins("line", "line.new(i, close, i + 1, open)", "line.set_extend"),
        dessins("line", "line.new(i, close, i + 1, open)", "line.set_style"),
        dessins("box", "box.new(i, close, i + 1, open)", "box.set_text"),
        dessins("box", "box.new(i, close, i + 1, open)", "box.set_border_style"),
        table("table.cell_set_text"),
        table("table.cell_set_text_size"),
    ];
    for corps in cas {
        let src = format!("//@version=6\nindicator('t', overlay = true)\n{corps}\nplot(close)");
        let e = arrete(sous(2 * MO, &src, b));
        assert_eq!(e.line, 8, "{e}\n{src}");
        passe(sous(64 * MO, &src, b));
    }
}

/// What a run told as made from its first bar to its last, under a budget it never comes close to (no count takes
/// it back to zero).
#[track_caller]
fn dit_en_tout(src: &str, b: &[Bar]) -> usize {
    passe(sous(512 * MO, &format!("//@version=6\nindicator('t')\n{src}"), b));
    assert_eq!(memoire::comptes(), 0);
    memoire::cree()
}

/// The lists the interpreter keeps for the script tell what they take as they grow, like the lists of the script: the
/// history of a variable read back at a computed distance (one value a bar), the history of an expression, the
/// first values of a moving average. Each pair of scripts runs the same statements and the same calls: only the
/// list differs.
#[test]
fn les_listes_de_l_interprete_disent_ce_qu_elles_prennent() {
    let b = &bars()[..300];
    let (n, v) = (b.len(), memoire::V);
    let k = "k = 1 + (close > 0 ? 0 : 1)";
    // a variable read back at a computed distance keeps every bar; at a written one, that many bars
    let plus = dit_en_tout(&format!("x = close\n{k}\nplot(x[k])"), b).saturating_sub(dit_en_tout(&format!("x = close\n{k}\nplot(x[1] + k)"), b));
    assert!(((n - 16) * v..2 * n * v + 64).contains(&plus), "history of a variable: {plus} bytes told");
    // the same for an expression
    let plus = dit_en_tout(&format!("{k}\nplot((close * 2)[k])"), b).saturating_sub(dit_en_tout(&format!("{k}\nplot((close * 2)[1] + k)"), b));
    assert!(((n - 64) * v..2 * n * v + 64).contains(&plus), "history of an expression: {plus} bytes told");
    // an expression read back 40 bars behind keeps 41 to 98 values, 1 to 18 when read at the bar itself
    let plus = dit_en_tout("plot((close * 2)[40])", b).saturating_sub(dit_en_tout("plot((close * 2)[0])", b));
    assert!(
        (40 * v..300 * v).contains(&plus),
        "history of an expression, written distance: {plus} bytes told"
    );
    // an average of 200 bars keeps its first 200 values until it starts, an average of 1 bar one value
    let plus = dit_en_tout("plot(ta.ema(close, 200))", b).saturating_sub(dit_en_tout("plot(ta.ema(close, 1))", b));
    assert!((190 * 8..600 * 8).contains(&plus), "first values of an average: {plus} bytes told");
}

/// What the making of a collection told is taken back when the collection is let go of before the next count, and
/// only then: once a count went through it, or when it comes from another run, its end changes nothing. (Taken back
/// twice, what the script made since the count was hidden, up to a whole budget.)
#[test]
fn une_collection_lachee_apres_un_compte_ne_rend_rien() {
    let (d_avant, d_avant_2) = (Value::tableau(vec![Value::Na; 1000]), Value::tableau(vec![Value::Na; 1000]));
    let _run = memoire::Session::entrer(64 * MO);
    let depart = memoire::cree();
    let a = Value::tableau(vec![Value::Na; 1000]);
    let fait = memoire::cree();
    assert!(fait >= depart + 1000 * memoire::V);
    // (a run starts a new count: what was made before it is not in what this run made)
    drop(d_avant);
    assert_eq!(memoire::cree(), fait, "made before this run: nothing to take back");
    drop(a);
    assert_eq!(memoire::cree(), depart, "made and let go of between two counts: as if never made");
    let a = Value::tableau(vec![Value::Na; 1000]);
    memoire::suite(0).unwrap();
    assert_eq!(memoire::cree(), 0, "a count starts from zero");
    memoire::creer(5000);
    drop(a);
    assert_eq!(memoire::cree(), 5000, "counted since its making: nothing to take back");
    drop(d_avant_2);
    assert_eq!(memoire::cree(), 5000, "made before this run: nothing to take back");
}

/// A list let go of before the next count takes back what its growth told with what its making told: a buffer made
/// full and pushed once told the doubling of its list, and stayed told after its end. Growth told before the last
/// count, or never told, is not taken back.
#[test]
fn une_liste_lachee_rend_aussi_ce_que_sa_croissance_a_dit() {
    let v = memoire::V;
    let pousser = |a: &Value, n: usize| {
        let Value::Arr(t) = a else { panic!() };
        (0..n).for_each(|_| memoire::pousser(&mut t.borrow_mut(), Value::Na));
    };
    let _run = memoire::Session::entrer(64 * MO);
    memoire::creer(5000);
    let depart = memoire::cree();
    // made full, pushed once (its list doubles), let go of: as if never made
    let a = Value::arr(vec![Value::Na; 1000]);
    pousser(&a, 1);
    assert!(memoire::cree() >= depart + 2000 * v, "{} bytes told", memoire::cree() - depart);
    drop(a);
    assert_eq!(memoire::cree(), depart, "making and growth taken back");
    // an empty one that grows step by step, and a map: the same
    let a = Value::arr(vec![]);
    pousser(&a, 100);
    let m = Value::map(vec![]);
    let Value::Map(d) = &m else { panic!() };
    (0..100).for_each(|k| memoire::pousser(&mut d.borrow_mut(), (Value::Num(k as f64), Value::Na)));
    assert!(memoire::cree() >= depart + 300 * v);
    drop((a, m));
    assert_eq!(memoire::cree(), depart, "an array and a map that grew from nothing");
    // two lists grow, one is let go of: only its own growth is taken back
    let (a, b) = (Value::arr(vec![Value::Na; 1000]), Value::arr(vec![Value::Na; 3000]));
    pousser(&a, 1);
    pousser(&b, 1);
    let deux = memoire::cree();
    drop(a);
    let reste = memoire::cree();
    assert!(reste >= depart + 6000 * v && deux - reste >= 2000 * v && deux - reste < 2100 * v);
    drop(b);
    assert_eq!(memoire::cree(), depart);
    // a list that grew without telling takes back its making alone (nothing of what others told)
    let a = Value::arr(vec![Value::Na; 1000]);
    let fait = memoire::cree();
    if let Value::Arr(t) = &a {
        t.borrow_mut().reserve(5000);
    }
    drop(a);
    assert!(
        fait > depart + 1000 * v && memoire::cree() == depart,
        "growth never told: {} left",
        memoire::cree() - depart
    );
    // growth told before a count is not taken back after it
    let a = Value::arr(vec![Value::Na; 1000]);
    pousser(&a, 1);
    memoire::suite(0).unwrap();
    memoire::creer(7000);
    pousser(&a, 1001);
    let avant = memoire::cree();
    drop(a);
    assert_eq!(memoire::cree(), avant, "made before the count: nothing to take back");
    // and what lists told as they grew before a count gives nothing, after it, to a list that grew without telling
    memoire::suite(0).unwrap();
    memoire::creer(7000);
    let depart = memoire::cree();
    let a = Value::arr(vec![Value::Na; 1000]);
    if let Value::Arr(t) = &a {
        t.borrow_mut().reserve(5000);
    }
    drop(a);
    assert_eq!(memoire::cree(), depart, "growth told before the count, taken back after it");
}

/// Lists of one bar that grow do not bring a count: 20 buffers of 50 000 values made full and pushed once at each
/// bar told 24 MB a bar and were counted again every few bars; a script that also kept many objects ran 1.4 to 5
/// times slower.
#[test]
fn des_listes_de_passage_qui_grandissent_n_amenent_pas_de_compte() {
    let b = &bars()[..200];
    let cas = [
        ("arrays", "for i = 0 to 19\n    t = array.new_float(50000, close)\n    t.push(close)"),
        (
            "arrays that grow from nothing",
            "for i = 0 to 19\n    t = array.new_float()\n    t.concat(g1)",
        ),
        (
            "maps",
            "var gm = map.new<int, float>()\nif barstate.isfirst\n    for k = 0 to 4095\n        gm.put(k, 1.0)\nfor i = 0 to 19\n    m = gm.copy()\n    m.put(-1, close)",
        ),
    ];
    for (nom, corps) in cas {
        let src = format!("//@version=6\nindicator('t')\nvar g1 = array.new_float(100000, 0.0)\nvar g2 = array.new_float(100000, 0.0)\n{corps}\nplot(close)");
        // (4.8 MB kept, 8 MB at most with the list of the moment, under a budget of 16 MB)
        passe(sous(16 * MO, &src, b));
        assert!(memoire::comptes() <= 2, "{nom}: counted {} times in 200 bars", memoire::comptes());
    }
}

/// A count that passes keeps what it found: the next one comes when what was found, plus what is made since, may
/// have reached the budget. 4.8 MB kept, a long loop that brings a count, then 1 MB kept per line: the script stops
/// at the 4th of these lines under a budget of 8 MB, where it holds 8.6 MB. (Forgotten, it went on to the 6th line
/// at least, 10.6 MB: a script could hold twice its budget.)
#[test]
fn un_compte_passe_garde_ce_qu_il_a_trouve() {
    let b = &bars()[..3];
    let mut src = String::from(
        "//@version=6
indicator('t')
type B
    array<float> v
var tout = array.new<B>()
float s = 0
if barstate.isfirst
    for i = 0 to 4
        tout.push(B.new(array.new_float(40000, 1.0)))
    for i = 0 to 393215
        s += 1
",
    );
    for _ in 0..12 {
        src += "    tout.push(B.new(array.new_float(40000, 1.0)))\n";
    }
    src += "plot(s)";
    let e = arrete(sous(8 * MO, &src, b));
    // (the pushes are lines 12 to 23: 5 arrays and 4 more are 8.6 MB)
    assert!((14..=16).contains(&e.line), "{e}");
    assert!(memoire::comptes() >= 2, "a first count passed, during the loop");
}

/// A script that sits just under its budget is not counted at every statement: past a count close to the budget,
/// the next one waits for a step more (8 MB, a sixteenth of a small budget). The smallest budget this script runs
/// under is searched for, then the script is run under it.
#[test]
fn un_script_au_ras_de_son_budget_n_est_pas_recompte_a_chaque_ligne() {
    let b = &bars()[..200];
    let src = "//@version=6
indicator('t')
var g1 = array.new_float(100000, 0.0)
var g2 = array.new_float(100000, 0.0)
s = 0.0
for i = 0 to 999
    s += i
plot(s)";
    // (it keeps 4.8 MB: under 4 MB it stops, under 8 MB it runs)
    let (mut bas, mut haut) = (4 * MO, 8 * MO);
    while haut - bas > 1024 {
        let milieu = bas + (haut - bas) / 2;
        match sous(milieu, src, b) {
            Ok(_) => haut = milieu,
            Err(_) => bas = milieu,
        }
    }
    passe(sous(haut, src, b));
    // (3.2 MB are told as made in these 200 bars, 1 KB under the budget: a count every 64 statements without the step)
    assert!(memoire::comptes() <= 40, "counted {} times in 200 bars", memoire::comptes());
}

/// Every text, collection, object and tuple of a script is made in `value.rs`, where its memory is counted. The
/// compiler holds it: what those values point at (`Texte`, `Tableau`, `Dico`, `Matrice`, `Objet`, `Nuplet`) has
/// private fields, so no other file can build one. This test keeps the other door closed: no shared block is made
/// anywhere else, where no count would see what it holds.
#[test]
fn les_valeurs_qui_gardent_de_la_memoire_sont_faites_a_un_seul_endroit() {
    let sources = [
        ("builtins.rs", include_str!("builtins.rs")),
        ("interp.rs", include_str!("interp.rs")),
        ("strategie.rs", include_str!("strategie.rs")),
        ("reglages.rs", include_str!("reglages.rs")),
        ("comparer.rs", include_str!("comparer.rs")),
        ("rapport.rs", include_str!("rapport.rs")),
        ("sorties.rs", include_str!("sorties.rs")),
        ("dialect.rs", include_str!("dialect.rs")),
        ("parser.rs", include_str!("parser.rs")),
        ("lexer.rs", include_str!("lexer.rs")),
        ("mod.rs", include_str!("mod.rs")),
    ];
    // (written in pieces: this file is not one of the sources, but stays clear of its own search)
    let blocs = ["new(", "from(", "new_cyclic(", "new_uninit(", "<"].map(|f| format!("Rc::{f}"));
    let mut lignes = 0;
    for (fichier, src) in sources {
        for (n, ligne) in src.lines().enumerate() {
            let code = ligne.split("//").next().unwrap_or("");
            lignes += 1;
            for f in &blocs {
                assert!(!code.contains(f.as_str()), "{fichier}:{} makes a shared block outside value.rs: {ligne}", n + 1);
            }
        }
    }
    assert!(lignes > 10_000, "{lignes} lines read");
    let makers = include_str!("value.rs");
    assert!(blocs.iter().any(|f| makers.contains(f.as_str())), "the search finds the makers where they are");
}

/// A call that adds an entry to the outputs is counted for more than the entry (its texts are counted apart).
#[test]
fn le_forfait_d_un_appel_couvre_la_plus_grande_entree_des_sorties() {
    let tailles = [
        size_of::<InputSpec>(),
        size_of::<Plot>(),
        size_of::<Shape>(),
        size_of::<Drawing>(),
        size_of::<Polyline>(),
        size_of::<Candles>(),
        size_of::<Linefill>(),
        size_of::<Table>(),
        size_of::<TableCell>(),
        size_of::<(usize, String)>(),
        size_of::<(f64, String, u32, String)>(),
        size_of::<(String, String, Vec<f64>)>(),
        size_of::<(Tracer, Tracer, Vec<Option<u32>>)>(),
    ];
    let plus_grande = tailles.into_iter().chain(strategie::tailles()).max().unwrap_or(0);
    assert!(plus_grande <= memoire::APPEL_SORTIE, "{plus_grande} > {}", memoire::APPEL_SORTIE);
}

/// B-062: the text of a collection cut at 100 000 characters kept the room its writing took (512 KB for 100 KB of
/// text) and told the budget its length: `log.info(a)` at every bar kept 5 times what was told, and on a long chart
/// the app died of a refused allocation before the budget's count came. The text is kept at its size, and told so.
#[test]
fn un_texte_coupe_ne_garde_pas_de_place_de_trop() {
    for (nom, v) in [
        ("30 000 numbers", Value::arr(vec![Value::Num(1.234_567_89); 30_000])),
        ("500 texts of 1 000 characters", Value::arr(vec![Value::str(&"x".repeat(1000)); 500])),
    ] {
        let s = v.text();
        assert!(s.ends_with('…') && s.chars().count() == 100_000, "{nom}: {} characters", s.chars().count());
        assert_eq!(s.capacity(), s.len(), "{nom}: the cut text keeps no room besides its own");
    }
}
