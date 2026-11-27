//! 🪜 How deep a script may nest (B-074). The parser and the interpreter call themselves once per level, the web app
//! has 1 MB of stack and no way to catch its end: before the fix a sum of 1 344 terms on one line, or 476
//! parentheses one in the other, stopped the whole app. A script now gets an error with its line. Pure: the bars of
//! the fixtures, no network.

use super::tests::bars;
use super::tests_intrabar::{avec, grouper, minutes};
use super::*;

/// `corps` as the third line of an indicator, run on a few bars.
fn essai(corps: &str) -> Result<Output, Error> {
    run(
        &format!("//@version=6\nindicator('t')\n{corps}\nplot(close)"),
        &bars()[..5],
        3600,
        &HashMap::new(),
    )
}

/// `n` blocks of `mot` one in the other, `fond` in the last one; the first line is indented `marge` times.
fn blocs(mot: &str, n: usize, marge: usize, fond: &str) -> String {
    let mut s = String::new();
    for k in 0..n {
        s += &format!("{}{mot}\n", "    ".repeat(marge + k));
    }
    s + &format!("{}{fond}", "    ".repeat(marge + n))
}

/// `n` functions named `nom`0, `nom`1…: the first one gives `fond`, each of the others calls the one before from
/// inside `m` nested `if`.
fn chaine(nom: &str, n: usize, m: usize, fond: &str) -> String {
    let mut s = format!("{nom}0(x) => {fond}\n");
    for f in 1..n {
        s += &format!(
            "{nom}{f}(x) =>\n    y = 0.0\n{}\n    y\n",
            blocs("if x > -1", m, 1, &format!("y := {nom}{}(x)", f - 1))
        );
    }
    s
}

/// What the parser reads without a parenthesis is as deep as it is long, and what it reads in parentheses takes
/// stack at each level: past the limit, an error with the line, and never the end of the stack. Each case stopped
/// the app before the fix (a sum of 1 344 terms, 476 parentheses, measured in a web build).
#[test]
fn une_expression_trop_longue_ou_trop_imbriquee_est_refusee_avec_sa_ligne() {
    let n = 2_000;
    let longues = [
        ("sum", format!("x = {}", vec!["1"; n].join(" + "))),
        ("texts", format!("x = 'a'{}", " + 'a'".repeat(n))),
        ("and", format!("x = true{}", " and true".repeat(n))),
        ("not", format!("x = {}true", "not ".repeat(n))),
        ("minus", format!("x = {}1", "-".repeat(n))),
        ("history", format!("x = close{}", "[1]".repeat(n))),
        ("member", format!("x = a{}", ".b".repeat(n))),
        ("calls in a row", format!("x = f{}", "()".repeat(n))),
        ("ternaries", format!("x = {}0", "true ? 1 : ".repeat(n))),
    ];
    for (nom, corps) in &longues {
        // (refused where it is read, before any tree that deep exists: never by the calculation)
        let e = essai(corps).expect_err(nom);
        let lu = e.msg.contains("expression trop longue") || e.msg.contains("(parenthèses, appels, blocs ou ? :)");
        assert!(e.line == 3 && lu, "{nom}: {e}");
    }
    let imbriquees = [
        ("parentheses", format!("x = {}1{}", "(".repeat(n), ")".repeat(n))),
        ("calls", format!("x = {}1{}", "math.abs(".repeat(n), ")".repeat(n))),
        ("lists", format!("x = {}1{}", "[".repeat(n), "]".repeat(n))),
        ("indexes", format!("x = {}1{}", "close[".repeat(n), "]".repeat(n))),
        ("lambdas", format!("x = {}1", "(a) => ".repeat(n))),
        ("functions", format!("{}1", (0..n).map(|k| format!("g{k}() => ")).collect::<String>())),
    ];
    for (nom, corps) in &imbriquees {
        let e = essai(corps).expect_err(nom);
        assert!(
            e.line == 3 && e.msg.contains("trop de niveaux l'un dans l'autre (parenthèses, appels, blocs ou ? :)"),
            "{nom}: {e}"
        );
    }
    for mot in ["if close > 0", "for i = 0 to 0", "while false", "switch\n", "for v in array.from(1)"] {
        // (a `switch` holds its cases one line down: the block of a case is the next `switch`)
        let corps = if mot.starts_with("switch") {
            let mut s = String::new();
            for k in 0..300 {
                s += &format!("{}switch\n{}close > 0 =>\n", "    ".repeat(2 * k), "    ".repeat(2 * k + 1));
            }
            s + &format!("{}x = 1", "    ".repeat(600))
        } else {
            blocs(mot, 300, 0, "x = 1")
        };
        let e = essai(&corps).expect_err(mot);
        assert!(
            e.msg.contains("trop de niveaux l'un dans l'autre (parenthèses, appels, blocs ou ? :)"),
            "{mot}: {e}"
        );
        // (the error is at the line where the limit is passed: a block in a block is two levels)
        assert!((40..=110).contains(&e.line), "{mot}: {e}");
    }
}

/// Far more than a script written by hand nests is still read and run: 240 terms in a row, 90 parentheses, 45
/// blocks one in the other.
#[test]
fn un_script_tres_imbrique_mais_sous_les_limites_tourne() {
    let o = essai(&format!("x = {}\nplot(x)", vec!["1"; 240].join(" + "))).unwrap();
    assert_eq!(o.plots[0].values[4], 240.0);
    let o = essai(&format!("x = 'a'{}\nplot(str.length(x))", " + 'a'".repeat(239))).unwrap();
    assert_eq!(o.plots[0].values[4], 240.0);
    let o = essai(&format!("x = {}7{}\nplot(x)", "(".repeat(90), ")".repeat(90))).unwrap();
    assert_eq!(o.plots[0].values[4], 7.0);
    let o = essai(&format!("x = {}-7{}\nplot(x)", "math.abs(".repeat(90), ")".repeat(90))).unwrap();
    assert_eq!(o.plots[0].values[4], 7.0);
    let o = essai(&format!("x = {}5\nplot(x)", "close < 0 ? 1 : ".repeat(90))).unwrap();
    assert_eq!(o.plots[0].values[4], 5.0);
    let o = essai(&format!("var x = 0.0\n{}\nplot(x)", blocs("if close > 0", 45, 0, "x := 9"))).unwrap();
    assert_eq!(o.plots[0].values[4], 9.0);
    let o = essai(&format!("x = {}true\ny = {}3\nplot(x ? y : 0)", "not ".repeat(200), "-".repeat(200))).unwrap();
    assert_eq!(o.plots[0].values[4], 3.0);
}

/// The message of a limit gives a number the writer of the script can count, and a way out that works. It said
/// « 100 au plus » where the 100th parenthesis and the 50th block were refused, « 250 niveaux au plus » for a sum of
/// 250 terms, and « coupe-la en plusieurs lignes » where a line break changes nothing.
#[test]
fn le_message_d_une_limite_dit_ce_qu_on_peut_compter() {
    let niveaux = "trop de niveaux l'un dans l'autre (parenthèses, appels, blocs ou ? :) : moins de 100 (un bloc dans un bloc compte pour deux)";
    let longue = "expression trop longue ou trop imbriquée : moins de 250 termes à la suite, moins quand les premiers sont eux-mêmes des calculs, des appels ou des [n] (coupe-la en morceaux, une variable par morceau)";
    let refus = |corps: &str| essai(corps).map(|_| ()).map_err(|e| e.msg);
    let parentheses = |n: usize| format!("x = {}7{}", "(".repeat(n), ")".repeat(n));
    let appels = |n: usize| format!("x = {}-7{}", "math.abs(".repeat(n), ")".repeat(n));
    let ifs = |n: usize| format!("var x = 0.0\n{}", blocs("if close > 0", n, 0, "x := 9"));
    let somme = |n: usize| format!("x = {}", vec!["close"; n].join(" + "));
    let ternaires = |n: usize| format!("x = {}5", "close < 0 ? 1 : ".repeat(n));
    let ternaires_entre_parentheses = |n: usize| format!("x = {}5{}", "close < 0 ? 1 : (".repeat(n), ")".repeat(n));
    for (nom, fait, n, message) in [
        ("parentheses", &parentheses as &dyn Fn(usize) -> String, 100, niveaux),
        ("calls", &appels, 100, niveaux),
        ("blocks", &ifs, 50, niveaux),
        ("terms", &somme, 250, longue),
        ("? :", &ternaires, 100, niveaux),
        ("? : (", &ternaires_entre_parentheses, 50, niveaux),
    ] {
        assert_eq!(refus(&fait(n - 1)), Ok(()), "{nom}: one less than the number of the message is read and run");
        assert_eq!(refus(&fait(n)), Err(message.to_string()), "{nom}");
    }
    // the first terms are the deepest: when they are calculations, calls, members or [n], one term less (as the
    // message says)
    for (terme, entre) in [
        ("nz(close)", " + "),
        ("syminfo.mintick", " + "),
        ("close[1]", " + "),
        ("2 * close", " + "),
        ("close > 0", " and "),
    ] {
        let somme = |n: usize| format!("x = {}", vec![terme; n].join(entre));
        assert_eq!(refus(&somme(248)), Ok(()), "248 × {terme}");
        assert_eq!(refus(&somme(249)), Err(longue.to_string()), "249 × {terme}");
    }
    // in an old Pine, an `iff` is written `(c ? a : …)`: it counts for two levels, 49 pass
    let iff = |n: usize| {
        let src = format!("//@version=4\nstudy('t')\nx = {}0{}\nplot(x)", "iff(close > 0, 1, ".repeat(n), ")".repeat(n));
        run(&src, &bars()[..5], 3600, &HashMap::new()).map(|_| ()).map_err(|e| (e.line, e.msg))
    };
    assert_eq!(iff(49), Ok(()));
    assert_eq!(iff(50), Err((3, niveaux.to_string())));
    // 300 terms over 30 lines are still one expression; the same terms in two variables are read and run
    let dix = ["close"; 10].join(" + ");
    assert_eq!(refus(&format!("x = {}", vec![dix.as_str(); 30].join(" +\n     "))), Err(longue.to_string()));
    let uns = ["1"; 150].join(" + ");
    let o = essai(&format!("a = {uns}\nx = a + {uns}\nplot(x)")).unwrap();
    assert_eq!(o.plots[0].values[4], 300.0);
}

/// Each function adds its own levels to those of the function that calls it: 64 functions calling one another from
/// inside 15 nested `if` stopped the app. The calculation stops with an error and the line; 10 such functions run.
#[test]
fn des_fonctions_qui_s_appellent_dans_des_blocs_s_arretent_avec_une_erreur() {
    let trop = "trop de niveaux l'un dans l'autre dans ce calcul";
    let e = essai(&(chaine("f", 64, 15, "x + 1") + "z = f63(close)")).unwrap_err();
    assert!(e.msg.contains(trop) && e.line > 3, "{e}");
    let o = essai(&(chaine("f", 10, 15, "x + 1") + "z = f9(close)\nplot(z - close)")).unwrap();
    assert_eq!(o.plots[0].values[4], 1.0);
    // the levels of a built-in that calls a lambda count too
    let lambdas = format!(
        "var a = array.from(1.0)\ng0(x) => x + 1\n{}z = g39(close)\nplot(z)",
        (1..40)
            .map(|k| format!("g{k}(x) => array.sum(array.map(a, (v) => g{}(v)))\n", k - 1))
            .collect::<String>()
    );
    interp::pic_niveau();
    assert_eq!(essai(&lambdas).unwrap().plots[0].values[4], 2.0);
    assert_eq!(interp::pic_niveau(), 120);
    // a sub-run goes on from the level of its caller: `request.security` asked 80 levels down for a value 80 levels
    // deep runs its 160 levels below the 80 of its caller (247 in all); one function more on each side is too many
    let sous = |n: usize| {
        let fond = format!("request.security(syminfo.tickerid, 'D', f{}(close))", n - 1);
        chaine("f", n, 15, "x + 1") + &chaine("h", n, 15, &fond) + &format!("z = h{}(close)\nplot(z)", n - 1)
    };
    interp::pic_niveau();
    assert!(essai(&sous(6)).unwrap().plots[0].values[4].is_finite());
    assert_eq!(interp::pic_niveau(), 247);
    let e = essai(&sous(7)).unwrap_err();
    assert!(e.msg.contains(trop), "{e}");
    assert_eq!(interp::pic_niveau(), interp::MAX_NIVEAUX + 1);
}

/// The fast path of `request.security_lower_tf` (an expression of bar series and built-ins alone, run on the
/// intrabars without a sub-run) goes on from the level of its caller too. (Started from zero, 11 functions and 90
/// built-ins ran 254 levels down, past the limit that keeps the stack.)
#[test]
fn le_chemin_rapide_des_intrabars_continue_au_niveau_de_son_appelant() {
    let m = minutes(0, 60, 100.0);
    let b = grouper(&m, 300);
    // `n` functions calling one another from inside 15 nested `if`; the last one asks for `k` built-ins one in the
    // other on each intrabar
    let essai = |n: usize, k: usize| {
        let fond = format!(
            "array.size(request.security_lower_tf(syminfo.tickerid, '1', {}close{}))",
            "math.abs(".repeat(k),
            ")".repeat(k)
        );
        let src = format!("//@version=6\nindicator('t')\n{}z = h{}(close)\nplot(z)", chaine("h", n, 15, &fond), n - 1);
        run_with(&src, &b, 300, &HashMap::new(), &avec(&[(60, m.clone())]))
    };
    // 10 functions are 148 levels and the 90 built-ins go on from there: 238 in all; one function more is 254
    interp::pic_niveau();
    assert_eq!(essai(10, 90).unwrap().plots[0].values.last(), Some(&5.0));
    assert_eq!(interp::pic_niveau(), 238);
    let e = essai(11, 90).unwrap_err();
    assert!(e.msg.contains("trop de niveaux l'un dans l'autre dans ce calcul"), "{e}");
    assert_eq!(interp::pic_niveau(), interp::MAX_NIVEAUX + 1);
}

/// A script of an older Pine (or of the first Argos Script) is converted before it is read, and the conversion
/// called itself once for each call inside another: 20 000 calls nested on one line ended the stack before the
/// parser saw the script. The conversion now stops going down, and the parser refuses the line like any other.
#[test]
fn un_vieux_dialecte_trop_imbrique_est_refuse_avec_sa_ligne() {
    let n = 20_000;
    let trop = "trop de niveaux l'un dans l'autre (parenthèses, appels, blocs ou ? :) : moins de 100 (un bloc dans un bloc compte pour deux)";
    let b = &bars()[..5];
    for (nom, src, ligne) in [
        (
            "Pine v4",
            format!("//@version=4\nstudy('t')\nx = {}1{}\nplot(x)", "abs(".repeat(n), ")".repeat(n)),
            3,
        ),
        (
            "Pine v2",
            format!("//@version=2\nstudy('t')\nx = {}1{}\nplot(x)", "abs(".repeat(n), ")".repeat(n)),
            3,
        ),
        (
            "Argos Script 1",
            format!("indicator('t')\nplot({}close{})", "ema(".repeat(n), ", 3)".repeat(n)),
            2,
        ),
    ] {
        let e = run(&src, b, 3600, &HashMap::new()).expect_err(nom);
        assert_eq!((e.line, e.msg.as_str()), (ligne, trop), "{nom}");
    }
    // under the limit nothing changes: 90 calls one in the other are converted (`abs` is `math.abs`) and run
    let src = format!("//@version=4\nstudy('t')\nx = {}-2{}\nplot(x)", "abs(".repeat(90), ")".repeat(90));
    let o = run(&src, b, 3600, &HashMap::new()).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(o.plots[0].values[4], 2.0);
}

/// The number of values an expression gives to `request.security` is read from the script, without running it: for
/// a call, from the last line of the function called. A function whose last line calls itself (or another that calls
/// it back) had no end there, and the stack ended before the script ran. It is now an error with its line, the one
/// the same function gives when it is called.
#[test]
fn une_fonction_qui_s_appelle_elle_meme_donnee_a_request_security_s_arrete_avec_une_erreur() {
    let m = minutes(0, 60, 100.0);
    let b = grouper(&m, 300);
    let lancer = |corps: &str| {
        let src = format!("//@version=6\nindicator('t')\n{corps}\nplot(close)");
        run_with(&src, &b, 300, &HashMap::new(), &avec(&[(60, m.clone())]))
    };
    let soi = |f: &str| format!("« {f} » s'appelle elle-même trop de fois (pas de récursion en Pine)");
    for (corps, ligne, msg) in [
        ("f() =>\n    f()\nx = request.security_lower_tf(syminfo.tickerid, '1', f())", 4, soi("f")),
        ("f() =>\n    f()\nx = request.security(syminfo.tickerid, 'D', f())", 4, soi("f")),
        (
            "g() =>\n    f()\nf() =>\n    g()\nx = request.security_lower_tf(syminfo.tickerid, '1', f())",
            6,
            soi("g"),
        ),
    ] {
        let e = lancer(corps).expect_err(corps);
        assert_eq!((e.line, e.msg), (ligne, msg), "{corps}");
    }
    // nothing to calculate (no intrabars of 5 seconds here), or a function that gives `na` before the last line
    // where it calls itself: the shape is read all the same, and the script runs
    for corps in [
        "f() =>\n    f()\nx = request.security_lower_tf(syminfo.tickerid, '5S', f())",
        "f() =>\n    break\n    f()\nx = request.security(syminfo.tickerid, 'D', f())",
        "g() =>\n    continue\n    f()\nf() =>\n    g()\nx = request.security(syminfo.tickerid, 'D', f())",
    ] {
        lancer(corps).unwrap_or_else(|e| panic!("{corps} : {e}"));
    }
    // a chain of 65 functions, as many as a calculation runs one in the other, gives its tuple as it does when it is
    // called (the reading of the shape stopped one function short); 66 are refused as when called
    let chaine = |n: usize, appel: &str| {
        let mut s = format!("f{n}() =>\n    [close, open]\n");
        for k in (1..n).rev() {
            s += &format!("f{k}() =>\n    f{}()\n", k + 1);
        }
        s + &format!("[x, y] = {appel}")
    };
    for appel in [
        "request.security(syminfo.tickerid, 'D', f1())",
        "request.security_lower_tf(syminfo.tickerid, '1', f1())",
        "f1()",
    ] {
        lancer(&chaine(65, appel)).unwrap_or_else(|e| panic!("65, {appel} : {e}"));
        let e = lancer(&chaine(66, appel)).expect_err(appel);
        assert_eq!(e.msg, soi("f66"), "66, {appel}");
    }
    // and the shape is still read through functions: a tuple at the end of a chain of three
    let o = lancer("a() =>\n    [close, open]\nb() =>\n    a()\nc() =>\n    b()\n[x, y] = request.security(syminfo.tickerid, 'D', c())\nplot(x - y, 'd')")
        .unwrap_or_else(|e| panic!("{e}"));
    assert!(o.plots.iter().any(|p| p.title == "d"));
}

/// A script of more than 1 MB is refused before it is read, at the line where it passes 1 MB. A line of 15 MB of
/// calls one in the other, in an old Pine, took 1.4 GB in its conversion and the allocation failed; the reader takes
/// about 40 bytes for each byte of a script of any language. A script of nearly 1 MB is read.
#[test]
fn un_script_de_plus_d_un_mo_est_refuse_avant_d_etre_lu() {
    let b = &bars()[..5];
    let trop = "script trop long : 1 Mo au plus (il dépasse à cette ligne)";
    let n = 250_000;
    for src in [
        format!("//@version=4\nstudy('t')\nx = {}1{}\nplot(x)", "abs(".repeat(n), ")".repeat(n)),
        format!("//@version=6\nindicator('t')\nx = 1{}\nplot(x)", " + 1".repeat(n)),
    ] {
        let e = run(&src, b, 3600, &HashMap::new()).expect_err("over 1 MB");
        assert_eq!((e.line, e.msg.as_str()), (3, trop));
        let e = run_avec_variables(&src, b, 3600, &HashMap::new(), &["x"]).expect_err("over 1 MB");
        assert_eq!((e.line, e.msg.as_str()), (3, trop));
    }
    let presque = format!("//@version=6\nindicator('t')\n// {}\nplot(close)", "x".repeat(super::MAX_SOURCE - 60));
    assert!(presque.len() <= super::MAX_SOURCE);
    run(&presque, b, 3600, &HashMap::new()).unwrap_or_else(|e| panic!("{e}"));
    let juste_trop = format!("{presque}\n// {}", "x".repeat(100));
    assert_eq!(run(&juste_trop, b, 3600, &HashMap::new()).expect_err("1 MB and 100 bytes").line, 5);
}

/// Every `.pine` file of the repository: the scripts built into the app and the reference scripts.
fn scripts_du_depot() -> Vec<(String, String)> {
    fn lire(dossier: &std::path::Path, out: &mut Vec<(String, String)>) {
        let mut noms: Vec<_> = std::fs::read_dir(dossier).unwrap().map(|e| e.unwrap().path()).collect();
        noms.sort();
        for p in noms {
            if p.is_dir() {
                lire(&p, out);
            } else if p.extension().is_some_and(|x| x == "pine") {
                out.push((p.file_name().unwrap().to_string_lossy().into_owned(), std::fs::read_to_string(&p).unwrap()));
            }
        }
    }
    let mut out = vec![];
    lire(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../pine"), &mut out);
    out
}

/// The limits are far above what a real script does: every script of the repository (30 of them, the LuxAlgo and
/// Zeiierman references among them) stays under a quarter of each limit.
#[test]
fn les_scripts_du_depot_restent_loin_des_limites() {
    let b = bars();
    let scripts = scripts_du_depot();
    assert!(scripts.len() >= 30, "{}", scripts.len());
    for (nom, src) in scripts {
        parser::pic();
        interp::pic_niveau();
        if let Err(e) = run(&src, &b[..b.len().min(600)], 3600, &HashMap::new()) {
            panic!("{nom}: {e}");
        }
        let ((prof, haut), niveau) = (parser::pic(), interp::pic_niveau());
        assert!(prof > 0 && 4 * prof <= parser::MAX_NIVEAUX, "{nom}: the parser went down {prof} levels");
        assert!(haut > 0 && 4 * haut <= parser::MAX_HAUTEUR, "{nom}: a tree of {haut} levels");
        assert!(
            niveau > 0 && 4 * niveau <= interp::MAX_NIVEAUX,
            "{nom}: the calculation went down {niveau} levels"
        );
    }
}
