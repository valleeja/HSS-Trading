//! 📜 Point 3 (« indicateurs = TradingView », tradingview-1): the Pine features Jee's pasted scripts needed (LuxAlgo
//! SMC and Liquidity Swings, Smart Swing VWAP, SMC MTF, Fib OTE, AI Moving Average, MA ribbon, an old v3 breakout),
//! each one kept by a small script here. Every test names the bug of the register it keeps closed (B-030 to B-040,
//! `rust/tools/porte/registre.txt`). Pure: synthetic bars, no network.

use super::*;
use argos_core::Bar;
use std::sync::Arc;

/// 2026-01-05 07:00 UTC (a Monday, not at midnight: the day, week and month groups start before the first bar).
const T0: i64 = 1_767_596_400;

/// `n` bars of `tf` seconds from `t0`: a deterministic random walk around 100, prices with 2 decimals (so
/// `syminfo.mintick` is 0.01), volumes 1..10.
fn walk(t0: i64, tf: i64, n: usize) -> Vec<Bar> {
    let mut x: u64 = 0x2545_F491_4F6C_DD1D;
    let mut alea = move || {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        (x >> 11) as f64 / (1u64 << 53) as f64
    };
    let r2 = |v: f64| (v * 100.0).round() / 100.0;
    let mut p = 100.0;
    (0..n)
        .map(|k| {
            let o = p;
            let c = r2(o * (1.0 + (alea() - 0.5) * 0.01));
            let (h, l) = (r2(o.max(c) * (1.0 + alea() * 0.003)), r2(o.min(c) * (1.0 - alea() * 0.003)));
            p = c;
            Bar {
                time: t0 + tf * k as i64,
                open: o,
                high: h,
                low: l,
                close: c,
                volume: 1.0 + (alea() * 9.0).round(),
            }
        })
        .collect()
}

/// The bars grouped by `tf` seconds, like the chart's own candles.
fn grouper(b: &[Bar], tf: i64) -> Vec<Bar> {
    let mut out: Vec<Bar> = vec![];
    for x in b {
        let t = x.time.div_euclid(tf) * tf;
        match out.last_mut() {
            Some(g) if g.time == t => {
                g.high = g.high.max(x.high);
                g.low = g.low.min(x.low);
                g.close = x.close;
                g.volume += x.volume;
            }
            _ => out.push(Bar { time: t, ..*x }),
        }
    }
    out
}

fn go_with(src: &str, b: &[Bar], tf: i64, opts: &RunOptions) -> Output {
    run_with(src, b, tf, &HashMap::new(), opts).unwrap_or_else(|e| panic!("line {} : {}", e.line, e.msg))
}

fn go(src: &str, b: &[Bar], tf: i64) -> Output {
    go_with(src, b, tf, &RunOptions::default())
}

fn plot<'a>(o: &'a Output, title: &str) -> &'a [f64] {
    &o.plots
        .iter()
        .find(|p| p.title == title)
        .unwrap_or_else(|| panic!("no plot « {title} »: {:?}", o.plots.iter().map(|p| &p.title).collect::<Vec<_>>()))
        .values
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-9 * b.abs().max(1.0)
}

/// A colour that draws nothing: transparency 100 (Pine's `na` colour).
fn invisible(c: u32) -> bool {
    (c >> 24) & 0x7f == 100
}

/// B-031: the Pine of LuxAlgo and of the older scripts was refused by the reader: `var float[] x` (arrays written
/// the old way), `for int i = 0 to …` (a typed loop variable), a user type in lower case (`pivot p = pivot.new(…)`),
/// `box(na)` / `line(na)` (an empty drawing), and Pine v1–v3's bare styles (`style=circles`, `linestyle=dashed`).
#[test]
fn les_ecritures_de_luxalgo_et_du_vieux_pine_passent() {
    let b = walk(T0, 3600, 60);
    let o = go(
        "//@version=5
indicator('t')
type pivot
    float level
    int bar
var float[] hauts = array.new_float()
var box bo = box(na)
var line li = line(na)
pivot p = pivot.new(high, bar_index)
array.push(hauts, p.level)
s = 0
for int i = 0 to array.size(hauts) - 1
    s += 1
plot(s, 'n')
plot(na(bo) and na(li) ? 1 : 0, 'vides')
plot(p.bar, 'bar')",
        &b,
        3600,
    );
    assert_eq!(plot(&o, "n").last(), Some(&60.0), "one pivot pushed per bar, counted by the typed loop");
    assert!(plot(&o, "vides").iter().all(|v| *v == 1.0), "box(na) and line(na) are empty drawings");
    assert_eq!(plot(&o, "bar")[59], 59.0);
    let o = go(
        "//@version=3
study('t')
plot(close, style=circles)
hline(0, linestyle=dashed)",
        &b,
        3600,
    );
    assert_eq!(o.plots[0].style, PlotStyle::Circles);
    assert_eq!(o.hlines[0].3, "dashed");
}

/// B-032: `chart.point` and `polyline.new` were unknown (Smart Swing VWAP draws its VWAPs as polylines through
/// points; LuxAlgo gives its lines points).
#[test]
fn chart_point_et_polyline() {
    let b = walk(T0, 3600, 30);
    let o = go(
        "//@version=6
indicator('t', overlay = true)
if barstate.islast
    pts = array.new<chart.point>()
    for i = 0 to 4
        pts.push(chart.point.from_index(bar_index - i, low[i]))
    polyline.new(pts, curved = false, closed = true, line_color = color.red)
    line.new(chart.point.from_index(bar_index - 5, high), chart.point.now(low))",
        &b,
        3600,
    );
    assert_eq!(o.polylines.len(), 1);
    let p = &o.polylines[0];
    assert!(p.closed && !p.curved && !p.xtime);
    assert_eq!(p.points.len(), 5);
    assert_eq!(p.points[0], (29.0, b[29].low));
    assert_eq!(p.points[4], (25.0, b[25].low));
    let l = o.drawings.iter().find(|d| d.kind == "line").expect("the line");
    assert_eq!((l.x1, l.y1, l.x2, l.y2), (24.0, b[29].high, 29.0, b[29].low));
}

/// B-032 (found by the Contrôle before the merge, 25 Nov): a deleted polyline kept all its points, whether deleted by
/// `polyline.delete` or pushed out by `max_polylines_count`; a script that draws its curve again at every bar made
/// the memory grow with the square of the curve. A deleted polyline keeps no point; the live ones keep theirs.
#[test]
fn une_polyline_effacee_ne_garde_aucun_point() {
    let b = walk(T0, 3600, 30);
    let o = go(
        "//@version=6
indicator('t', overlay = true, max_polylines_count = 3)
pts = array.new<chart.point>()
for i = 0 to 4
    pts.push(chart.point.from_index(bar_index + i, close))
p = polyline.new(pts)
if barstate.islast
    polyline.delete(p)",
        &b,
        3600,
    );
    assert_eq!(o.polylines.len(), 30, "one polyline per bar");
    let (vivantes, effacees): (Vec<_>, Vec<_>) = o.polylines.iter().partition(|p| !p.deleted);
    assert_eq!(vivantes.len(), 2, "3 kept by max_polylines_count, the last one deleted by the script");
    assert!(vivantes.iter().all(|p| p.points.len() == 5), "a live polyline keeps its points");
    assert_eq!(effacees.len(), 28);
    assert_eq!(
        effacees.iter().map(|p| p.points.len()).sum::<usize>(),
        0,
        "points kept by the deleted polylines"
    );
    assert!(o.polylines[29].deleted && o.polylines[26].deleted && !o.polylines[27].deleted);
}

/// B-033: `plotcandle` / `plotbar` were refused (Jee's « Volume candles » and the SMC scripts colour their own
/// candles); a colour given as `na` draws nothing on that bar, no colour given is Pine's blue.
#[test]
fn plotcandle_et_plotbar() {
    let b = walk(T0, 3600, 40);
    let o = go(
        "//@version=6
indicator('t')
plotcandle(open, high, low, close, 'c', close > open ? color.green : color.red, wickcolor = color.gray)
plotbar(open, high, low, close, 'b', color = bar_index % 2 == 0 ? na : color.orange)
plotcandle(open, high, low, close, 'd')",
        &b,
        3600,
    );
    assert_eq!(o.candles.len(), 3);
    let (c, bb, d) = (&o.candles[0], &o.candles[1], &o.candles[2]);
    assert_eq!(c.title, "c");
    assert!(!c.bars && bb.bars);
    for (i, x) in b.iter().enumerate() {
        assert_eq!(c.ohlc[i], [x.open, x.high, x.low, x.close]);
        assert!(c.color[i].is_some() && c.wick[i].is_some());
        assert_eq!(bb.color[i].is_some(), i % 2 == 1, "bar {i}: na draws nothing");
        assert!(d.color[i].is_some(), "no colour given: drawn in Pine's blue");
    }
    assert_ne!(c.color[0], d.color[0]);
}

/// B-034: `last_bar_time`, `timenow` and `time(timeframe)` / `time_close(timeframe)` were missing (the SMC MTF
/// dashboard ages its levels with them); `timenow` counts the intrabars too, else an age read on a lower timeframe
/// came out negative.
#[test]
fn les_heures_de_pine() {
    let b = walk(T0, 3600, 1000);
    let src = "//@version=6
indicator('t')
plot(last_bar_time, 'lbt')
plot(timenow, 'now')
plot(time('60'), 'h')
plot(time('1D'), 'jour')
plot(time_close('1D'), 'fin')
plot(time('W'), 'semaine')
plot(time('M'), 'mois')
plot(time_close('M'), 'mois fin')";
    let o = go(src, &b, 3600);
    let last = b[999].time;
    assert!(plot(&o, "lbt").iter().all(|v| *v == last as f64 * 1000.0));
    assert_eq!(plot(&o, "now")[999], last as f64 * 1000.0);
    let jour = |t: i64| t.div_euclid(86_400) * 86_400;
    let mois = |t: i64| {
        use chrono::Datelike;
        let d = chrono::DateTime::from_timestamp(t, 0).unwrap().date_naive();
        chrono::NaiveDate::from_ymd_opt(d.year(), d.month(), 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp()
    };
    for (i, x) in b.iter().enumerate() {
        let ms = |s: i64| s as f64 * 1000.0;
        assert_eq!(plot(&o, "h")[i], ms(x.time));
        assert_eq!(plot(&o, "jour")[i], ms(jour(x.time)), "bar {i}");
        assert_eq!(plot(&o, "fin")[i], ms(jour(x.time) + 86_400));
        // weeks start on Monday 00:00 UTC (1 Jan 1970 was a Thursday)
        let lundi = jour(x.time) - (x.time.div_euclid(86_400) + 3).rem_euclid(7) * 86_400;
        assert_eq!(plot(&o, "semaine")[i], ms(lundi), "bar {i}");
        assert_eq!(plot(&o, "mois")[i], ms(mois(x.time)), "bar {i}");
        assert_eq!(plot(&o, "mois fin")[i], ms(mois(mois(x.time) + 32 * 86_400)), "bar {i}");
    }
    // (T0 + 1000 h is 15 Feb: one month change seen)
    assert!(plot(&o, "mois").iter().any(|v| *v != plot(&o, "mois")[0]));
    // the newest intrabar is later than the newest chart bar's open: timenow is that one
    let minutes = walk(last, 60, 60);
    let opts = RunOptions {
        intrabars: Arc::new([(60, minutes.clone())].into_iter().collect()),
        ..Default::default()
    };
    let o = go_with(src, &b, 3600, &opts);
    assert_eq!(plot(&o, "now")[999], minutes[59].time as f64 * 1000.0);
}

/// B-035: a text variable named like a namespace (`timeframe = "60"` in LuxAlgo SMC v5) hid that namespace's
/// functions: `timeframe.in_seconds(timeframe)` was read as a method of the text.
#[test]
fn une_variable_au_nom_d_un_espace() {
    let b = walk(T0, 3600, 10);
    let o = go(
        "//@version=5
indicator('t')
timeframe = '240'
plot(timeframe.in_seconds(timeframe), 's')
plot(timeframe.contains('4') ? 1 : 0, 'c')
plot(str.length(timeframe), 'n')",
        &b,
        3600,
    );
    assert_eq!(plot(&o, "s")[9], 14_400.0);
    assert_eq!(plot(&o, "c")[9], 1.0, "a text function stays the text's method");
    assert_eq!(plot(&o, "n")[9], 3.0);
}

/// B-036: `matrix.*` was unknown (Jee's AI Moving Average keeps its learning data in matrices).
#[test]
fn les_matrices() {
    let b = walk(T0, 3600, 5);
    let o = go(
        "//@version=6
indicator('t')
m = matrix.new<float>(2, 3, 0)
m.set(0, 1, 5)
m.set(1, 2, close)
m.add_row(2, array.from(1.0, 2.0, 3.0))
t = m.transpose()
r = m.copy()
r.sort(2, order.descending)
plot(m.rows(), 'rows')
plot(t.rows(), 'trows')
plot(m.get(2, 2), 'g')
plot(m.max(), 'max')
plot(matrix.avg(m), 'avg')
plot(r.get(0, 2), 'tri')
plot(m.elements_count(), 'n')",
        &b,
        3600,
    );
    let c = b[4].close;
    assert_eq!(plot(&o, "rows")[4], 3.0);
    assert_eq!(plot(&o, "trows")[4], 3.0);
    assert_eq!(plot(&o, "g")[4], 3.0);
    assert_eq!(plot(&o, "max")[4], c.max(5.0));
    assert!(near(plot(&o, "avg")[4], (11.0 + c) / 9.0));
    assert_eq!(plot(&o, "tri")[4], c.max(3.0), "rows sorted on column 2, largest first");
    assert_eq!(plot(&o, "n")[4], 9.0);
    // out of the matrix: an error with its line, never a silent value
    let e = run(
        "//@version=6\nindicator('t')\nm = matrix.new<float>(2, 2, 0)\nplot(m.get(5, 0))",
        &b,
        3600,
        &HashMap::new(),
    )
    .expect_err("out of range");
    assert_eq!(e.line, 4);
}

/// B-036 (found by the Contrôle before the merge, 25 Nov): `add_row` / `add_col` grew a matrix past the 100 000
/// elements `matrix.new` refuses (1 500 000 measured in a loop over 50 bars). The same limit with the same words;
/// a matrix that reaches exactly 100 000 elements still passes.
#[test]
fn une_matrice_ne_grandit_pas_au_dela_de_100_000_elements() {
    let b = walk(T0, 3600, 5);
    let essai = |corps: &str| {
        run(
            &format!("//@version=6\nindicator('t')\n{corps}\nplot(m.elements_count(), 'n')"),
            &b,
            3600,
            &HashMap::new(),
        )
    };
    let trop = |f: &str| format!("matrix.{f} : 100 000 éléments au plus, comme sur TradingView");
    // 999 × 100, one more row: 100 000 elements, the limit itself
    let o =
        essai("m = matrix.new<float>(999, 100, 0)\nm.add_row(m.rows(), array.new_float(100, 1.0))").unwrap_or_else(|e| panic!("line {} : {}", e.line, e.msg));
    assert_eq!(plot(&o, "n")[4], 100_000.0);
    // one more row, one more column: refused, with the line
    let e = essai("m = matrix.new<float>(999, 100, 0)\nm.add_row(0, array.new_float(100, 1.0))\nm.add_row()").expect_err("add_row");
    assert_eq!((e.line, e.msg), (5, trop("add_row")));
    let e = essai("m = matrix.new<float>(999, 100, 0)\nm.add_col()").expect_err("add_col");
    assert_eq!((e.line, e.msg), (4, trop("add_col")));
    // the loop the Contrôle measured: 10 000 rows of 3 more at each bar
    let e = essai("var m = matrix.new<float>(0, 0)\nfor i = 0 to 9999\n    m.add_row(m.rows(), array.from(1.0, 2.0, 3.0))").expect_err("the loop");
    assert_eq!((e.line, e.msg), (5, trop("add_row")));
}

/// B-037: `request.security` on a timeframe lower than the chart's was refused (Argos Vector Zone Pro, SMC MTF); like
/// TradingView it gives the expression's value at the last intrabar of each chart bar, `na` until the app has
/// loaded those intrabars (listed in `lower_tf_missing`).
#[test]
fn request_security_plus_petit_que_la_chart() {
    let minutes = walk(T0, 60, 48 * 60);
    let heures = grouper(&minutes, 3600);
    let src = "//@version=6
indicator('t')
plot(request.security(syminfo.tickerid, '1', close), 'c')
plot(request.security(syminfo.tickerid, '1', ta.sma(close, 3)), 'sma')";
    let opts = RunOptions {
        intrabars: Arc::new([(60, minutes.clone())].into_iter().collect()),
        ..Default::default()
    };
    let o = go_with(src, &heures, 3600, &opts);
    assert!(o.lower_tf_missing.is_empty());
    assert_eq!(o.lower_tf_asked, vec![60]);
    for (i, h) in heures.iter().enumerate() {
        assert_eq!(plot(&o, "c")[i], h.close, "hour {i}");
        let k = (i + 1) * 60;
        let sma = (minutes[k - 1].close + minutes[k - 2].close + minutes[k - 3].close) / 3.0;
        assert!(near(plot(&o, "sma")[i], sma), "hour {i}");
    }
    let o = go(src, &heures, 3600);
    assert!(plot(&o, "c").iter().all(|v| v.is_nan()));
    assert_eq!(o.lower_tf_missing, vec![60]);
}

/// B-038: `str.tostring(x, format.volume)` wrote the bare number (TradingView: « 1.235M », « 12K ») and
/// `format.mintick` did not round to the symbol's tick (the SMC labels).
#[test]
fn str_tostring_volume_et_mintick() {
    let b = walk(T0, 3600, 5);
    let o = go(
        "//@version=6
indicator('t', overlay = true)
if barstate.islast
    label.new(bar_index, close, str.tostring(1234567, format.volume))
    label.new(bar_index, close, str.tostring(12000, format.volume))
    label.new(bar_index, close, str.tostring(999.456, format.volume))
    label.new(bar_index, close, str.tostring(2.5e9, format.volume))
    label.new(bar_index, close, str.tostring(123.456789, format.mintick))",
        &b,
        3600,
    );
    let textes: Vec<&str> = o.drawings.iter().map(|d| d.text.as_str()).collect();
    assert_eq!(textes, ["1.235M", "12K", "999.46", "2.5B", "123.46"]);
    // (found by the Contrôle before the merge, 25 Nov) a number that rounds up to 1 000 of its unit takes the unit
    // above, as on TradingView: 999 999.6 is « 1M », never « 1000K »
    let o = go(
        "//@version=6
indicator('t', overlay = true)
if barstate.islast
    label.new(bar_index, close, str.tostring(999999.6, format.volume))
    label.new(bar_index, close, str.tostring(999.996, format.volume))
    label.new(bar_index, close, str.tostring(999999999.6, format.volume))
    label.new(bar_index, close, str.tostring(999999999999.6, format.volume))
    label.new(bar_index, close, str.tostring(-999999.6, format.volume))
    label.new(bar_index, close, str.tostring(999999.4, format.volume))
    label.new(bar_index, close, str.tostring(999.994, format.volume))
    label.new(bar_index, close, str.tostring(1000, format.volume))",
        &b,
        3600,
    );
    let textes: Vec<&str> = o.drawings.iter().map(|d| d.text.as_str()).collect();
    assert_eq!(textes, ["1M", "1K", "1B", "1T", "-1M", "999.999K", "999.99", "1K"]);
}

/// B-039: a drawing colour given as `na` was drawn in blue (LuxAlgo SMC's BOS / CHoCH labels had a blue background
/// under their text); `na` is no colour at all, given at the call or by a setter.
#[test]
fn une_couleur_na_ne_dessine_rien() {
    let b = walk(T0, 3600, 5);
    let o = go(
        "//@version=6
indicator('t', overlay = true)
if barstate.islast
    label.new(bar_index, close, 'BOS', color = na, textcolor = color.white)
    line.new(bar_index - 3, close, bar_index, close, color = na)
    box.new(bar_index - 3, high, bar_index, low, border_color = na, bgcolor = na)
    l2 = label.new(bar_index, close, 'CHoCH')
    l2.set_color(na)
    label.new(bar_index, close, 'blue')",
        &b,
        3600,
    );
    let d = &o.drawings;
    assert!(invisible(d[0].color) && !invisible(d[0].text_color));
    assert!(invisible(d[1].color));
    assert!(invisible(d[2].color) && invisible(d[2].bg));
    assert!(invisible(d[3].color));
    assert!(!invisible(d[4].color), "no colour given: Pine's default, drawn");
}

/// B-040: Pine v6 loops ran as v5's: TradingView v6 reads a `for … in` array's size again at each turn and evaluates
/// a `for` loop's `to` bound before each turn (LuxAlgo's v6 code pushes while it walks); v5 keeps both fixed.
#[test]
fn boucles_v6_relues_a_chaque_tour() {
    let b = walk(T0, 3600, 3);
    let corps = "a = array.from(1, 2, 3)
n = 0
for x in a
    n += 1
    if a.size() < 6
        a.push(x)
k = 0
lim = 3
for i = 0 to lim
    k += 1
    lim := 1
plot(n, 'n')
plot(k, 'k')";
    let v6 = go(&format!("//@version=6\nindicator('t')\n{corps}"), &b, 3600);
    assert_eq!((plot(&v6, "n")[2], plot(&v6, "k")[2]), (6.0, 2.0));
    let v5 = go(&format!("//@version=5\nindicator('t')\n{corps}"), &b, 3600);
    assert_eq!((plot(&v5, "n")[2], plot(&v5, "k")[2]), (3.0, 4.0));
}

/// B-030: `ta.vwap(source, anchor, stdev_mult)` ignored its anchor (always a new UTC day), took hlc3 for older bars
/// and gave no bands (Smart Swing VWAP, the quarter VWAP).
#[test]
fn ta_vwap_ancre_et_bandes() {
    let b = walk(T0, 3600, 100);
    let o = go(
        "//@version=6
indicator('t')
[v, up, dn] = ta.vwap(close, bar_index % 7 == 0, 2)
plot(v, 'v')
plot(up, 'up')
plot(dn, 'dn')
plot(ta.vwap(hlc3), 'jour')",
        &b,
        3600,
    );
    let (mut pv, mut vv, mut p2) = (0.0, 0.0, 0.0);
    let (mut jpv, mut jv) = (0.0, 0.0);
    for (i, x) in b.iter().enumerate() {
        if i % 7 == 0 {
            (pv, vv, p2) = (0.0, 0.0, 0.0);
        }
        pv += x.close * x.volume;
        vv += x.volume;
        p2 += x.close * x.close * x.volume;
        let w = pv / vv;
        let sd = (p2 / vv - w * w).max(0.0).sqrt();
        assert!(near(plot(&o, "v")[i], w), "bar {i}");
        assert!(near(plot(&o, "up")[i], w + 2.0 * sd), "bar {i}");
        assert!(near(plot(&o, "dn")[i], w - 2.0 * sd), "bar {i}");
        if i == 0 || x.time.div_euclid(86_400) != b[i - 1].time.div_euclid(86_400) {
            (jpv, jv) = (0.0, 0.0);
        }
        let hlc3 = (x.high + x.low + x.close) / 3.0;
        jpv += hlc3 * x.volume;
        jv += x.volume;
        assert!(near(plot(&o, "jour")[i], jpv / jv), "bar {i}: no anchor, a new UTC day");
    }
}
