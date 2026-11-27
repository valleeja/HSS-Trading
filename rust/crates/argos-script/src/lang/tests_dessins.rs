//! 🧹 The drawings a script deletes (B-064): they leave the list, the oldest one is found without the list being
//! read again, and a handle still names its drawing after the list was packed. Before the fix a deleted drawing kept
//! its place and its texts until the end of the run, and each new drawing read the whole list. Pure: the bars of the
//! fixtures, no network.

use super::tests::bars;
use super::*;

/// `corps` after the declaration of an indicator, run on the first `n` bars.
#[track_caller]
fn lance(corps: &str, n: usize) -> Output {
    let src = format!("//@version=6\nindicator('t', overlay = true)\n{corps}");
    run(&src, &bars()[..n], 3000, &HashMap::new()).unwrap_or_else(|e| panic!("{e}"))
}

/// The live drawings of a kind, in the order of the list.
fn vivants<'o>(o: &'o Output, kind: &str) -> Vec<&'o Drawing> {
    o.drawings.iter().filter(|d| d.kind == kind && !d.deleted).collect()
}

/// A script that deletes what it draws keeps what is alive, not all it ever drew: 10 labels made and deleted at
/// each bar were 3 000 places and 3 000 texts kept on 300 bars. A list now holds its live drawings and at most
/// 1 024 deleted ones (or as many as the live ones), with no text.
#[test]
fn un_dessin_efface_quitte_la_liste() {
    let o = lance(
        "var garde = label.new(0, 1, 'garde')
for i = 0 to 9
    l = label.new(bar_index, close, str.repeat('x', 100))
    label.delete(l)
if barstate.islast
    label.set_text(garde, 'fin')
    label.set_y(garde, 7)",
        300,
    );
    assert!(o.drawings.len() <= 1026, "{} drawings in the list", o.drawings.len());
    let v = vivants(&o, "label");
    assert_eq!(v.len(), 1);
    // (the label of the first bar moved in the list: its handle still names it)
    assert_eq!((v[0].text.as_str(), v[0].y1), ("fin", 7.0));
    let restes: Vec<&Drawing> = o.drawings.iter().filter(|d| d.deleted).collect();
    assert!(!restes.is_empty());
    assert!(restes
        .iter()
        .all(|d| d.text.capacity() + d.style.capacity() + d.extend.capacity() + d.size.capacity() == 0));
}

/// Past the most a kind keeps (500), the oldest drawing goes, found at once: before, each new drawing read the whole
/// list to count the live ones (18 million places read for 6 000 labels, 2 × 10¹⁰ for 200 000). The steps counted
/// here are those of `Rangs` (finding the oldest, packing the list); a reading of the list from anywhere else is
/// seen by the run at scale below (`deux_cent_mille_etiquettes_se_dessinent_sans_attendre`).
#[test]
fn le_plus_vieux_dessin_part_sans_relire_la_liste() {
    rangs::pas();
    let o = lance("for i = 0 to 19\n    label.new(bar_index, i, str.tostring(bar_index * 100 + i))", 300);
    let pas = rangs::pas();
    let v = vivants(&o, "label");
    assert_eq!(v.len(), 500);
    // (the 500 newest: the 20 of each of the last 25 bars, in the order they were made)
    assert!(v.iter().all(|d| d.x1 >= 275.0));
    assert_eq!((v[0].text.as_str(), v[499].text.as_str()), ("27500", "29919"));
    assert!(o.drawings.len() <= 500 + 1025, "{} drawings in the list", o.drawings.len());
    assert!(pas <= 4 * 6000, "{pas} places read for 6 000 labels");
    // each kind keeps its own 500: labels, lines and boxes made together are 500 of each
    let o = lance(
        "for i = 0 to 9\n    label.new(bar_index, i, 'a')\n    line.new(bar_index, i, bar_index + 1, i)\n    box.new(bar_index, i + 1, bar_index + 1, i)",
        300,
    );
    for kind in ["label", "line", "box"] {
        let v = vivants(&o, kind);
        assert!(v.len() == 500 && v.iter().all(|d| d.x1 >= 250.0), "{kind}: {} alive", v.len());
    }
}

/// A copy counts like a new drawing in the most its kind keeps: 600 copies of a label were 601 live labels.
#[test]
fn une_copie_compte_dans_le_maximum_de_sa_sorte() {
    let o = lance(
        "var l = label.new(0, 0, 'a')\nif barstate.isfirst\n    for i = 0 to 599\n        label.copy(label.new(1, i, 'b'))",
        3,
    );
    assert_eq!(vivants(&o, "label").len(), 500);
    let o = lance(
        "var l = line.new(0, 0, 1, 1)\nif barstate.isfirst\n    for i = 0 to 599\n        line.copy(line.new(1, i, 2, i))",
        3,
    );
    assert_eq!(vivants(&o, "line").len(), 500);
}

/// A handle names its drawing wherever the drawing is in the list: lines kept from the first bar are changed at the
/// last one, after 6 000 others were made and deleted; a fill between two lines follows them to their new places,
/// and a fill whose line is gone is gone with it.
#[test]
fn une_poignee_nomme_son_dessin_apres_le_tassement() {
    let o = lance(
        "var a = line.new(0, 1, 1, 1)
var b = line.new(0, 2, 1, 2)
var c = line.new(0, 4, 1, 4)
var f = linefill.new(a, b, color.red)
var g = linefill.new(a, c, color.red)
if bar_index == 1
    line.delete(c)
for i = 0 to 19
    t = line.new(bar_index, 5, bar_index + 1, 5)
    line.delete(t)
if barstate.islast
    line.set_y1(a, 10)
    line.set_y1(b, 20)
    x = line.new(0, 3, 1, 3)
    linefill.new(b, x, color.blue)
    line.set_y2(linefill.get_line1(f), 11)",
        300,
    );
    assert!(o.drawings.len() <= 1100, "{} drawings in the list", o.drawings.len());
    let lignes: Vec<(f64, f64)> = vivants(&o, "line").iter().map(|d| (d.y1, d.y2)).collect();
    assert_eq!(lignes, [(10.0, 11.0), (20.0, 2.0), (3.0, 3.0)]);
    // (the output names the lines of a fill by their places in `drawings`)
    let y = |k: usize| o.drawings.get(k).filter(|d| !d.deleted).map(|d| d.y1);
    let fills: Vec<_> = o.linefills.iter().map(|f| (f.deleted, y(f.line1), y(f.line2))).collect();
    assert_eq!(fills, [(false, Some(10.0), Some(20.0)), (true, None, None), (false, Some(20.0), Some(3.0))]);
}

/// A deleted drawing is `na` for every function, as in Pine, whether it is still in the list or not: nothing to
/// read, nothing to change, no error. (Before, it could still be read and changed until the end of the run.)
#[test]
fn un_dessin_efface_vaut_na_pour_toutes_les_fonctions() {
    let o = lance(
        "var vieux = label.new(0, 0, 'v')
if barstate.isfirst
    label.delete(vieux)
for i = 0 to 9
    label.delete(label.new(bar_index, close, 't'))
l = label.new(bar_index, close, 'a')
label.delete(l)
x = label.get_x(l)
label.set_text(l, 'b')
label.set_point(l, chart.point.from_index(bar_index, close + 1000))
label.set_y(vieux, 2000)
c = label.copy(l)
label.delete(l)
label.set_text(vieux, 'z')
t = label.get_text(vieux)
plot(na(x) and na(c) and na(t) and na(label.get_y(vieux)) ? 1 : 0)",
        300,
    );
    assert!(o.plots[0].values.iter().all(|v| *v == 1.0), "{:?}", &o.plots[0].values[..5]);
    assert!(o.drawings.iter().all(|d| d.deleted && d.text.is_empty()));
    assert!(o.drawings.len() <= 1026, "{} drawings in the list", o.drawings.len());
    // (the deleted ones still in the list were not moved by `set_point` or `set_y`)
    assert!(o.drawings.len() > 100, "{} deleted drawings still in the list", o.drawings.len());
    assert!(o.drawings.iter().all(|d| d.y1 < 1000.0));
}

/// The same for the two other lists: polylines (50 kept when the script does not say) and fills between lines (500).
#[test]
fn les_courbes_et_les_remplissages_effaces_quittent_leur_liste() {
    let points = "pts = array.from(chart.point.from_index(bar_index, close), chart.point.from_index(bar_index + 1, close))";
    let o = lance(
        &format!("{points}\nfor i = 0 to 9\n    p = polyline.new(pts)\n    polyline.delete(p)\n    polyline.delete(p)"),
        300,
    );
    assert!(
        o.polylines.len() <= 1026 && o.polylines.iter().all(|p| p.deleted),
        "{} polylines in the list",
        o.polylines.len()
    );
    rangs::pas();
    let o = lance(&format!("{points}\nfor i = 0 to 9\n    polyline.new(pts)"), 300);
    let pas = rangs::pas();
    let v: Vec<_> = o.polylines.iter().filter(|p| !p.deleted).collect();
    // (the 50 newest: the 10 of each of the last 5 bars)
    assert!(v.len() == 50 && v.iter().all(|p| p.points[0].0 >= 295.0));
    assert!(
        o.polylines.len() <= 50 + 1025 && pas <= 4 * 3000,
        "{} polylines in the list, {pas} places read",
        o.polylines.len()
    );
    let lignes = "var a = line.new(0, 1, 1, 1)\nvar b = line.new(0, 2, 1, 2)";
    rangs::pas();
    let o = lance(
        &format!("{lignes}\nfor i = 0 to 9\n    linefill.new(a, b, color.new(color.red, bar_index % 100))"),
        300,
    );
    let pas = rangs::pas();
    let v: Vec<_> = o.linefills.iter().filter(|f| !f.deleted).collect();
    assert_eq!(v.len(), 500);
    assert!(
        o.linefills.len() <= 500 + 1025 && pas <= 4 * 3000,
        "{} fills in the list, {pas} places read",
        o.linefills.len()
    );
    // (they still fill between the two lines of the first bar)
    assert!(v.iter().all(|f| (o.drawings[f.line1].y1, o.drawings[f.line2].y1) == (1.0, 2.0)));
    let o = lance(&format!("{lignes}\nfor i = 0 to 9\n    f = linefill.new(a, b, color.red)\n    linefill.delete(f)\n    linefill.delete(f)\n    linefill.set_color(f, color.blue)"), 300);
    assert!(
        o.linefills.len() <= 1026 && o.linefills.iter().all(|f| f.deleted),
        "{} fills in the list",
        o.linefills.len()
    );
    // (and none of them took the colour given after it was deleted)
    let bleu = lance(&format!("{lignes}\nlinefill.new(a, b, color.blue)"), 1).linefills[0].color;
    assert!(!o.linefills.is_empty() && o.linefills.iter().all(|f| f.color != bleu));
}

/// What a script deletes no longer counts in what it keeps (B-062): 20 labels of 1 000 characters made and deleted
/// at each bar are 7 MB on 300 bars, and run under a budget of 2 MB.
#[test]
fn les_dessins_effaces_ne_comptent_plus_dans_le_budget() {
    let src = "//@version=6\nindicator('t', overlay = true)\nfor i = 0 to 19\n    l = label.new(bar_index, close, str.repeat('x', 1000))\n    label.delete(l)";
    memoire::budget_essai(2 << 20);
    let o = run(src, &bars()[..300], 3000, &HashMap::new()).unwrap_or_else(|e| panic!("{e}"));
    assert!(o.drawings.len() <= 1026 && o.drawings.iter().all(|d| d.deleted));
}

/// What a script deletes itself leaves the count of its sort: drawings kept from the first bar stay while others are
/// made and deleted at each bar. (A deletion not told to the count, and the kept ones would go as soon as the count
/// passed the most their sort keeps: 400 labels and 20 deleted at each bar, 40 polylines, 400 fills.)
#[test]
fn ce_que_le_script_efface_ne_compte_plus_dans_le_maximum() {
    let o = lance(
        "var garde = array.new<label>()
if barstate.isfirst
    for i = 0 to 399
        garde.push(label.new(0, i, 'g'))
for i = 0 to 19
    label.delete(label.new(bar_index, close, 't'))",
        300,
    );
    let v = vivants(&o, "label");
    assert!(v.len() == 400 && v.iter().all(|d| d.text == "g"), "{} labels alive", v.len());
    let o = lance(
        "pts = array.from(chart.point.from_index(bar_index, close), chart.point.from_index(bar_index + 1, close))
var garde = array.new<polyline>()
if barstate.isfirst
    for i = 0 to 39
        garde.push(polyline.new(pts))
for i = 0 to 9
    polyline.delete(polyline.new(pts))",
        300,
    );
    let v: Vec<_> = o.polylines.iter().filter(|p| !p.deleted).collect();
    assert!(v.len() == 40 && v.iter().all(|p| p.points[0].0 == 0.0), "{} polylines alive", v.len());
    let o = lance(
        "var a = line.new(0, 1, 1, 1)
var b = line.new(0, 2, 1, 2)
var garde = array.new<linefill>()
if barstate.isfirst
    for i = 0 to 399
        garde.push(linefill.new(a, b, color.red))
for i = 0 to 9
    linefill.delete(linefill.new(a, b, color.blue))",
        300,
    );
    assert_eq!(o.linefills.iter().filter(|f| !f.deleted).count(), 400);
    // deleted twice, a drawing leaves the count once: of the 10 kept at each bar the newest stay, as many as their
    // sort keeps (50 polylines, 500 fills; one less here, the one made last took a place before it was deleted) —
    // counted out twice, none of them would ever go
    let o = lance(
        "pts = array.from(chart.point.from_index(bar_index, close), chart.point.from_index(bar_index + 1, close))
for i = 0 to 9
    polyline.new(pts)
    p = polyline.new(pts)
    polyline.delete(p)
    polyline.delete(p)",
        300,
    );
    let v: Vec<_> = o.polylines.iter().filter(|p| !p.deleted).collect();
    assert!(v.len() == 49 && v.iter().all(|p| p.points[0].0 >= 295.0), "{} polylines alive", v.len());
    let o = lance(
        "var a = line.new(0, 1, 1, 1)
var b = line.new(0, 2, 1, 2)
float lu = 0
for i = 0 to 9
    linefill.new(a, b, color.red)
    f = linefill.new(a, b, color.blue)
    linefill.delete(f)
    linefill.delete(f)
    lu += na(linefill.get_line1(f)) ? 0 : 1
plot(lu)",
        300,
    );
    assert_eq!(o.linefills.iter().filter(|f| !f.deleted).count(), 499);
    assert!(o.plots[0].values.iter().all(|v| *v == 0.0), "a deleted fill has no line to give");
}

/// The ids of the drawings are part of what a script keeps (B-062): each count asks the three lists what they hold.
#[test]
fn les_rangs_des_dessins_sont_dans_le_compte() {
    let src = "//@version=6\nindicator('t', overlay = true)\nfor i = 0 to 19\n    l = label.new(bar_index, close, str.repeat('x', 1000))\n    label.delete(l)";
    memoire::budget_essai(2 << 20);
    rangs::peses();
    run(src, &bars()[..300], 3000, &HashMap::new()).unwrap_or_else(|e| panic!("{e}"));
    let (comptes, peses) = (memoire::comptes(), rangs::peses());
    assert!(comptes >= 1 && peses >= 3 * comptes, "{comptes} counts, {peses} lists of ids asked");
}

/// A drawing made inside `request.security` belongs to that other run: the chart's run has no such drawing, and the
/// handle it gets is `na`, alone or in a tuple (B-076). Given as it was, it named the chart's own drawing of the same
/// rank: half the labels of the chart got a text written for the sub-run's, or were deleted.
#[test]
fn une_poignee_d_un_autre_passage_vaut_na() {
    let o = lance(
        "var miens = array.new<label>()
sx = request.security(syminfo.tickerid, '120', label.new(bar_index, close, 'sub'))
[a, t, l2] = request.security(syminfo.tickerid, '120', [close, table.new(position.top_right, 1, 1), line.new(0, 0, 1, 1)])
miens.push(label.new(bar_index, high, 'chart'))
label.set_text(sx, 'autre')
table.cell(t, 0, 0, 'autre')
line.delete(l2)
if bar_index % 2 == 0
    label.delete(sx)
intacts = 0
if barstate.islast
    for l in miens
        if label.get_text(l) == 'chart'
            intacts += 1
plot(intacts, 'intacts')
plot(na(sx) and na(t) and na(l2) ? 1 : 0, 'na')
plot(a, 'a')",
        300,
    );
    let lu = |nom: &str| &o.plots.iter().find(|p| p.title == nom).expect(nom).values;
    assert_eq!(lu("intacts")[299], 300.0);
    assert!(lu("na").iter().all(|v| *v == 1.0) && lu("a")[299].is_finite());
    assert!(vivants(&o, "label").len() == 300 && o.tables.is_empty() && vivants(&o, "line").is_empty());
}

/// B-076, the rest of it: a handle of the other run reached the chart inside a tuple inside a tuple, and through the
/// fast path of a lower timeframe (`request.security` below the chart, `request.security_lower_tf`), which kept the
/// expression's values without the sub-run's cleaning: the chart's own labels got the texts meant for the others.
#[test]
fn une_poignee_d_un_autre_passage_vaut_na_dans_un_tuple_de_tuple_et_sur_une_unite_plus_fine() {
    let o = lance(
        "f() => [label.new(bar_index, high, 'sub'), 1]
g() => [f(), 2]
[x, y] = request.security(syminfo.tickerid, '240', g())
label.new(bar_index, close, 'chart')
float hna = na
if not na(y)
    [h, k] = x
    label.set_text(h, 'autre')
    hna := na(h) ? 1 : 0
plot(hna, 'na')",
        300,
    );
    let na = &o.plots.iter().find(|p| p.title == "na").expect("na").values;
    assert!(na.iter().filter(|v| v.is_finite()).count() > 200 && na.iter().all(|v| !v.is_finite() || *v == 1.0));
    assert!(vivants(&o, "label").len() == 300 && vivants(&o, "label").iter().all(|d| d.text == "chart"));
    // a lower timeframe: 5-minute bars, the 1-minute ones given
    let m = super::tests_intrabar::minutes(0, 600, 100.0);
    let b = super::tests_intrabar::grouper(&m, 300);
    let src = "//@version=6
indicator('t', overlay = true)
label.new(bar_index, close, 'chart')
l = request.security(syminfo.tickerid, '1', label.new(bar_index, close, 'sub'))
label.set_text(l, 'x')
ls = request.security_lower_tf(syminfo.tickerid, '1', label.new(bar_index, close, 'sub'))
for h in ls
    label.set_text(h, 'y')
plot(na(l) ? 1 : 0, 'na')
plot(ls.size(), 'n')";
    let o = run_with(src, &b, 300, &HashMap::new(), &super::tests_intrabar::avec(&[(60, m.clone())])).unwrap_or_else(|e| panic!("{e}"));
    let lu = |nom: &str| &o.plots.iter().find(|p| p.title == nom).expect(nom).values;
    assert!(lu("na").iter().all(|v| *v == 1.0) && lu("n")[b.len() - 1] == 5.0);
    assert!(vivants(&o, "label").len() == b.len() && vivants(&o, "label").iter().all(|d| d.text == "chart"));
}

/// B-076 through a collection: a sub-run's handle given back inside an array, a map, a matrix, an object of a type
/// of the script (a `label` and a `box` field, as in the scripts of zones), and deeper (an array and a map of such
/// objects) still named the chart's drawing of the same rank — the Contrôle's script of 9 lines lost 18 of its
/// 400 chart labels, never deleted. It now names nothing on the chart: every function and method takes it for `na`.
#[test]
fn une_poignee_d_un_autre_passage_vaut_na_dans_une_collection() {
    let o = lance(
        "type Zone
    label lbl
    box bx
f() =>
    a = array.new<label>()
    array.push(a, label.new(bar_index, close, 'sub'))
    m = map.new<string, label>()
    m.put('k', label.new(bar_index, close, 'sub'))
    mx = matrix.new<label>(1, 1)
    mx.set(0, 0, label.new(bar_index, close, 'sub'))
    z = Zone.new(label.new(bar_index, close, 'sub'), box.new(bar_index, high, bar_index + 1, low))
    zs = array.new<Zone>()
    zs.push(Zone.new(label.new(bar_index, close, 'sub'), box.new(bar_index, high, bar_index + 1, low)))
    mz = map.new<string, Zone>()
    mz.put('k', Zone.new(label.new(bar_index, close, 'sub')))
    [a, m, mx, z, zs, mz]
[a, m, mx, z, zs, mz] = request.security(syminfo.tickerid, '120', f())
label.new(bar_index, close, 'chart')
box.new(bar_index, high, bar_index + 1, low, text = 'chart')
float vus = na
if not na(a)
    label.delete(array.get(a, 0))
    label.set_text(m.get('k'), 'autre')
    label.delete(mx.get(0, 0))
    label.set_text(z.lbl, 'autre')
    box.delete(z.bx)
    zs.get(0).lbl.delete()
    box.set_text(zs.get(0).bx, 'autre')
    label.set_text(mz.get('k').lbl, 'autre')
    x = array.get(a, 0)
    vus := na(x) and na(m.get('k')) and na(mx.get(0, 0)) and na(z.lbl) and na(z.bx) and na(zs.get(0).lbl) ? 1 : 0
plot(vus, 'na')
plot(na(label.get_text(z.lbl)) ? 1 : 0, 'texte')",
        300,
    );
    let lu = |nom: &str| &o.plots.iter().find(|p| p.title == nom).expect(nom).values;
    assert!(lu("na").iter().filter(|v| v.is_finite()).count() > 200 && lu("na").iter().all(|v| !v.is_finite() || *v == 1.0));
    assert!(lu("texte").iter().all(|v| *v == 1.0));
    let (labels, boxes) = (vivants(&o, "label"), vivants(&o, "box"));
    assert!(labels.len() == 300 && labels.iter().all(|d| d.text == "chart"), "{} labels", labels.len());
    assert!(boxes.len() == 300 && boxes.iter().all(|d| d.text == "chart"), "{} boxes", boxes.len());
}

/// B-076 through a collection, below the chart: the fast path (an expression of built-ins alone, `array.from`), a
/// function of the script on a lower timeframe (a whole sub-run on the intrabars) and `request.security_lower_tf`
/// (an array of what each intrabar gave, itself holding arrays and objects).
#[test]
fn une_poignee_d_un_autre_passage_vaut_na_dans_une_collection_sur_une_unite_plus_fine() {
    let m = super::tests_intrabar::minutes(0, 600, 100.0);
    let b = super::tests_intrabar::grouper(&m, 300);
    let src = "//@version=6
indicator('t', overlay = true)
type Zone
    label lbl
f() =>
    z = Zone.new(label.new(bar_index, close, 'sub'))
    z
g() =>
    array.from(label.new(bar_index, close, 'sub'))
label.new(bar_index, close, 'chart')
a = request.security(syminfo.tickerid, '1', array.from(label.new(bar_index, close, 'sub')))
z = request.security(syminfo.tickerid, '1', f())
ls = request.security_lower_tf(syminfo.tickerid, '1', g())
zs = request.security_lower_tf(syminfo.tickerid, '1', f())
float vus = na
if not na(a)
    label.set_text(array.get(a, 0), 'x')
    vus := na(array.get(a, 0)) ? 1 : 0
if not na(z)
    z.lbl.set_text('y')
    label.delete(z.lbl)
for l in ls
    label.delete(array.get(l, 0))
for q in zs
    label.set_text(q.lbl, 'z')
plot(vus, 'na')
plot(ls.size() + zs.size(), 'n')";
    let o = run_with(src, &b, 300, &HashMap::new(), &super::tests_intrabar::avec(&[(60, m.clone())])).unwrap_or_else(|e| panic!("{e}"));
    let lu = |nom: &str| &o.plots.iter().find(|p| p.title == nom).expect(nom).values;
    assert!(lu("na").iter().all(|v| *v == 1.0) && lu("n")[b.len() - 1] == 10.0);
    let labels = vivants(&o, "label");
    assert!(labels.len() == b.len() && labels.iter().all(|d| d.text == "chart"), "{} labels", labels.len());
    // `request.security_lower_tf` alone (no `request.security` in the script)
    let src = "//@version=6
indicator('t', overlay = true)
label.new(bar_index, close, 'chart')
ls = request.security_lower_tf(syminfo.tickerid, '1', array.from(label.new(bar_index, close, 'sub')))
for l in ls
    label.delete(array.get(l, 0))";
    let o = run_with(src, &b, 300, &HashMap::new(), &super::tests_intrabar::avec(&[(60, m.clone())])).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(vivants(&o, "label").len(), b.len());
}

/// B-076, what a handle compares with: on the chart's own timeframe the sub-run makes its label at each bar with
/// the rank the chart's label gets at the same bar. Compared as ranks, the sub-run's label equalled the chart's —
/// `array.includes`, `array.indexof` and `==` found the chart's label in the array the call gave back.
#[test]
fn une_poignee_d_un_autre_passage_n_egale_aucun_dessin_de_la_chart() {
    let o = lance(
        "f() =>
    array.from(label.new(bar_index, close, 'sub'))
x = request.security(syminfo.tickerid, timeframe.period, f())
mien = label.new(bar_index, close, 'chart')
plot(array.includes(x, mien) or array.indexof(x, mien) >= 0 or array.get(x, 0) == mien ? 1 : 0, 'egal')
plot(array.includes(array.from(mien), mien) and array.from(mien).get(0) == mien ? 1 : 0, 'soi')",
        300,
    );
    let lu = |nom: &str| &o.plots.iter().find(|p| p.title == nom).expect(nom).values;
    assert!(lu("egal").iter().all(|v| *v == 0.0) && lu("soi").iter().all(|v| *v == 1.0));
    assert_eq!(vivants(&o, "label").len(), 300);
}

/// B-076, the same for a lambda of Argos: its value is an index in the table of lambdas of the run that made it. Given
/// back by a sub-run of `request.security`, `array.map` called the chart's lambda of the same index (the result was 2,
/// from the chart's `g`, instead of 1 000), or stopped with « lambda inconnue ». It now stops with an error that says
/// where the lambda comes from, and the chart's own lambdas still run.
#[test]
fn une_lambda_d_un_autre_passage_n_appelle_pas_celle_de_la_chart() {
    let essai = |corps: &str| {
        let src = format!("//@version=6\nindicator('t')\n{corps}");
        run(&src, &bars()[..300], 3000, &HashMap::new()).map(|o| o.plots[0].values[299])
    };
    for corps in [
        "k = request.security(syminfo.tickerid, '120', (v) => v * 1000)\ng = (v) => v + 1\na = array.from(1.0)\nplot(na(k) ? na : array.get(array.map(a, k), 0), 'r')",
        "f() =>\n    array.from((v) => v * 1000)\nx = request.security(syminfo.tickerid, timeframe.period, f())\ng = (v) => v + 1\na = array.from(1.0)\nplot(array.get(array.map(a, array.get(x, 0)), 0), 'r')",
    ] {
        let e = essai(corps).expect_err(corps).to_string();
        assert!(e.contains("autre unité de temps") && e.starts_with(&format!("ligne {}", corps.lines().count() + 2)), "{e}");
    }
    assert_eq!(
        essai("g = (v) => v + 1\nplot(array.get(array.map(array.from(1.0), g), 0), 'r')").ok(),
        Some(2.0)
    );
}

/// The handles a function gives back name what they say in the run that made them (B-076: each one carries the
/// run's number): the copy of a label, a line and a box, and the two lines of a fill, are changed through them.
#[test]
fn les_poignees_rendues_par_copy_et_get_line_nomment_leur_dessin() {
    let o = lance(
        "if barstate.islast
    c = label.copy(label.new(bar_index, close, 'a'))
    label.set_text(c, 'copie')
    d = line.copy(line.new(0, 1, 1, 1))
    line.set_y1(d, 7)
    e = box.copy(box.new(0, 2, 1, 1))
    box.set_text(e, 'copie')
    f = linefill.new(line.new(0, 3, 1, 3), line.new(0, 4, 1, 4))
    line.set_y1(linefill.get_line1(f), 8)
    line.set_y1(linefill.get_line2(f), 9)",
        3,
    );
    let (labels, lines, boxes) = (vivants(&o, "label"), vivants(&o, "line"), vivants(&o, "box"));
    assert_eq!(labels.iter().map(|d| d.text.as_str()).collect::<Vec<_>>(), ["a", "copie"]);
    assert_eq!(lines.iter().map(|d| d.y1).collect::<Vec<_>>(), [1.0, 7.0, 8.0, 9.0]);
    assert_eq!(boxes.iter().map(|d| d.text.as_str()).collect::<Vec<_>>(), ["", "copie"]);
}

/// At scale: 20 labels made at each of 10 000 bars, deleted by the script or pushed out by the most their sort
/// keeps, are drawn in a moment (0.4 s). Reading the list again at each new drawing, the same script took 155 s
/// with the labels deleted and 182 s without: the bound is far from both.
#[test]
fn deux_cent_mille_etiquettes_se_dessinent_sans_attendre() {
    let b = super::tests_intrabar::minutes(0, 10_000, 100.0);
    for corps in [
        "for i = 0 to 19\n    l = label.new(bar_index, close, str.repeat('x', 100))\n    label.delete(l)",
        "for i = 0 to 19\n    label.new(bar_index, close + i, 'x')",
    ] {
        let src = format!("//@version=6\nindicator('t', overlay = true)\n{corps}");
        let depart = std::time::Instant::now();
        let o = run(&src, &b, 60, &HashMap::new()).unwrap_or_else(|e| panic!("{e}"));
        let duree = depart.elapsed();
        assert!(o.drawings.len() <= 500 + 1025, "{} drawings in the list", o.drawings.len());
        assert!(duree.as_secs() < 30, "{corps}: {duree:?}");
    }
}
