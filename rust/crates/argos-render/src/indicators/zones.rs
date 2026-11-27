//! 🧭 Zones et structure : Vector Zone, figures (patterns), price action, SMC et SMC Pro.

use super::*;

/// Argos Price Action comme v1 : OB prolongés vers la droite (ligne médiane en tirets), breakers arrêtés à leur
/// cassure avec leurs lignes prolongées, changements de polarité ▲▼, tableau résumé (option).
/// 🔺 Patterns auto : la figure (pivots reliés), ses lignes en tirets prolongées jusqu'à la cassure, l'étiquette
/// (▲ haussier, ▼ baissier, ◆ neutre ; ✓ = cassure confirmée) et l'objectif mesuré en pointillés.
/// 🧭 Argos Vector Zone Pro, comme le Pine : EMA, zones notées, trendlines, patterns auto (résistance / support +
/// étiquette), patterns chartistes (figure, neckline, cible), canal de Fibonacci, liquidité, étiquettes (CONF, Sweep,
/// cassures) et tableau Vector Watchlist en haut à droite. `texts` : textes des inputs (niveaux de Fibonacci).
pub(super) fn draw_vector_zone(
    pm: &mut Pixmap,
    text: &Text,
    g: &Geo,
    z: &vector_zone::VectorZone,
    vals: &[f64],
    texts: Option<&std::collections::HashMap<String, String>>,
) {
    use vector_zone::Signal as S;
    let p = argos_script::inputs::Inputs {
        params: &vector_zone::PARAMS,
        vals,
    };
    let on_chart = |c: Rgb| crate::palette::on_chart(c);
    let c = |k: &str| crate::palette::on_chart(p.rgb(k));
    // couleurs de TradingView (color.lime, red, teal, orange, gray, green, yellow)
    let (lime, red, teal, orange, gray, green, yellow) = (
        on_chart(hex(0x00e676)),
        on_chart(hex(0xf23645)),
        on_chart(hex(0x009688)),
        on_chart(hex(0xff9800)),
        on_chart(hex(0x787b86)),
        on_chart(hex(0x4caf50)),
        on_chart(hex(0xffeb3b)),
    );
    let (black, white) = (Rgb(0, 0, 0), Rgb(0xff, 0xff, 0xff));
    let n = g.bars.len();
    if n == 0 {
        return;
    }
    let last = n - 1;
    let dec = crate::viewport::data_decimals(g.bars);
    // index de la bougie qui contient l'heure `t` (lignes en temps : xloc.bar_time)
    let idx = |t: i64| g.bars.partition_point(|b| b.time <= t).saturating_sub(1);
    let trait_plein = |pm: &mut Pixmap, a: (f64, f64), b: (f64, f64), col: Rgb, w: f32, alpha: f32| {
        crate::shapes::lines_alpha(pm, 0.0, 0.0, &[&[(a.0 as f32, a.1 as f32), (b.0 as f32, b.1 as f32)]], col, w, alpha);
    };
    // étiquette : boîte arrondie et texte (plusieurs lignes), au-dessus (`up`) ou sous le point
    let etiquette = |pm: &mut Pixmap, x: f64, y: f64, lignes: &[String], bg: Rgb, alpha: f32, fg: Rgb, up: bool| {
        let w = lignes.iter().map(|l| text.width(l, 10.5) as f64).fold(0.0, f64::max) + 10.0;
        let h = 15.0 * lignes.len() as f64 + 2.0;
        let top = if up { y - h - 4.0 } else { y + 4.0 };
        crate::shapes::round_fill(pm, crate::shapes::Rect::new(x - w / 2.0, top, w, h), 4.0, bg, alpha);
        for (k, l) in lignes.iter().enumerate() {
            text.draw(pm, l, (x as f32, (top + 12.0 + 15.0 * k as f64) as f32), 10.5, fg, Align::Center);
        }
    };
    // EMA 1 à 4 (rouge, lime, vert, teal ; épaisseurs 1, 2, 2, 3)
    if p.on("showEma") {
        for (k, (col, w)) in [(red, 1.0), (lime, 2.0), (green, 2.0), (teal, 3.0)].into_iter().enumerate() {
            let pts: Vec<(f64, f64)> = (g.first..=g.last)
                .filter_map(|i| z.emas[k].get(i).filter(|v| v.is_finite()).map(|&v| (g.x(i), g.y(v))))
                .collect();
            let d = crate::dense_line(pts.into_iter());
            if d.len() > 1 {
                lines(pm, 0.0, 0.0, &[&d], col, w);
            }
        }
    }
    // zones d'offre / de demande : du pivot au bord droit ; couleur et opacité selon la note (forte, faible, en dessous)
    for zn in &z.zones {
        let (y0, y1) = (g.y(zn.top), g.y(zn.bot));
        let x0 = g.x(zn.born.max(g.first)).max(0.0);
        let x1 = g.right();
        if x1 <= x0 {
            continue;
        }
        let (base, alpha) = match (zn.demand, zn.score >= z.strong, zn.score >= z.weak) {
            (true, true, _) => (lime, 0.30),
            (true, false, true) => (teal, 0.18),
            (false, true, _) => (red, 0.30),
            (false, false, true) => (orange, 0.18),
            _ => (gray, 0.10),
        };
        crate::fill(pm, x0, y0.min(y1), x1 - x0, (y1 - y0).abs().max(1.0), base, alpha);
        crate::fill(pm, x0, y0.min(y1), x1 - x0, 1.0, base, 0.6);
        crate::fill(pm, x0, y0.max(y1), x1 - x0, 1.0, base, 0.6);
        let label = format!(
            "{}{}/100 · {}T{}",
            if zn.demand { "DEMANDE " } else { "OFFRE " },
            zn.score,
            zn.touches,
            if zn.vector { " · VECTOR" } else { "" }
        );
        text.draw(pm, &label, ((x1 - 6.0) as f32, ((y0 + y1) / 2.0 + 4.0) as f32), 10.5, base, Align::Right);
    }
    // trendlines auto (sommets en jaune, creux en orange), en tirets prolongés à droite
    if p.on("showTL") {
        for (t, col) in [(z.tl_down, yellow), (z.tl_up, orange)] {
            let Some((i1, p1, i2, p2)) = t else { continue };
            let k = (p2 - p1) / ((i2 as f64 - i1 as f64).max(1.0));
            let pe = p1 + k * (last as f64 - i1 as f64);
            super::tirets(pm, &[(g.x(i1), g.y(p1)), (g.x(last), g.y(pe))], 6.0, 4.0, col, 2.0, 1.0);
        }
    }
    // 🔺 patterns auto : résistance et support prolongés à droite, étiquette du pattern 3 bougies après la dernière
    if let Some(pa) = &z.pat {
        let tl = g.bars[last].time;
        let ligne = |pm: &mut Pixmap, l: &vector_zone::Ligne, col: Rgb| {
            let i1 = idx(l.t1);
            let (a, b) = ((g.x(i1), g.y(l.at(g.bars[i1].time))), (g.x(last), g.y(l.at(tl))));
            // prolongée jusqu'au bord droit (extend.right)
            let k = if b.0 > a.0 { (b.1 - a.1) / (b.0 - a.0) } else { 0.0 };
            let r = g.right();
            trait_plein(pm, a, (r, b.1 + k * (r - b.0)), col, 2.0, 1.0);
        };
        if let Some(r) = &pa.res {
            ligne(pm, r, c("cRes"));
        }
        if let Some(s) = &pa.sup {
            ligne(pm, s, c("cSup"));
        }
        if let (Some(r), Some(s), true) = (&pa.res, &pa.sup, pa.kind > 0) {
            let y = g.y((r.at(tl) + s.at(tl)) / 2.0);
            let biais = match pa.bias {
                1 => " · biais haussier ▲",
                -1 => " · biais baissier ▼",
                _ => " · neutre",
            };
            let lignes = [
                format!("{}{biais}", vector_zone::PAT_NOMS[pa.kind.min(10)]),
                format!(
                    "UT {}{} · Rés. {} · Sup. {} touches",
                    vector_zone::tf_nom(pa.tf),
                    if pa.dom { " (domine)" } else { "" },
                    r.touches,
                    s.touches
                ),
            ];
            let bg = match pa.bias {
                1 => green,
                -1 => red,
                _ => gray,
            };
            let w = lignes.iter().map(|l| text.width(l, 10.5) as f64).fold(0.0, f64::max) + 10.0;
            // style label_left : la boîte part du point vers la droite
            etiquette(pm, g.xf(last as f64 + 3.0) + w / 2.0, y - 17.0, &lignes, bg, 0.8, white, false);
        }
    }
    // Ⓜ️ pattern chartiste : la figure, la neckline en tirets jusqu'à la dernière bougie, l'étiquette et la cible
    if let Some(ch) = &z.chart {
        let col = if ch.bias > 0 { c("cBullP") } else { c("cBearP") };
        let pts: Vec<(f32, f32)> = ch.pts.iter().map(|q| (g.x(q.0) as f32, g.y(q.1) as f32)).collect();
        lines(pm, 0.0, 0.0, &[&pts], col, 2.0);
        let (nb1, np1) = ch.neck;
        super::tirets(pm, &[(g.x(nb1), g.y(np1)), (g.x(last), g.y(ch.neck_now))], 6.0, 4.0, col, 1.0, 1.0);
        if let Some(&(b, pr)) = ch.pts.last() {
            let lignes = [
                format!(
                    "{}{}{}",
                    ch.name,
                    if ch.bias > 0 { " ▲" } else { " ▼" },
                    if ch.confirmed { " · CONFIRMÉ" } else { " · en formation" }
                ),
                format!(
                    "Neckline {} · Cible {}",
                    crate::viewport::format_price(ch.neck_now, dec),
                    crate::viewport::format_price(ch.target, dec)
                ),
            ];
            let fg = if ch.bias > 0 { black } else { white };
            etiquette(pm, g.x(b), g.y(pr), &lignes, col, 0.85, fg, ch.bias < 0);
        }
    }
    // 📏 canal de Fibonacci : un niveau par valeur de l'input, de la 1re bougie de base jusqu'à `fibExt` bougies après
    // la dernière ; 0 et 1 en trait plein, les autres en tirets ; golden pocket 0.5–0.618 en jaune
    if let Some(f) = &z.fib {
        let col = c("cFib");
        let txt = texts.and_then(|t| t.get("fibLevels")).map_or(vector_zone::FIB_NIVEAUX, String::as_str);
        let ext = p.get("fibExt").round().max(0.0);
        let bar_s = if n > 1 { g.bars[last].time - g.bars[last - 1].time } else { 0 };
        let t_end = g.bars[last].time + (ext as i64) * bar_s;
        let i1 = idx(f.base.t1);
        let (x1, x2) = (g.x(i1), g.xf(last as f64 + ext));
        let y_of = |l: f64| (g.y(f.base.at(g.bars[i1].time) + l * f.d), g.y(f.base.at(t_end) + l * f.d));
        let niveaux = vector_zone::fib_niveaux(txt);
        if p.on("fibGP") && niveaux.contains(&0.5) && niveaux.contains(&0.618) {
            let ((a1, a2), (b1, b2)) = (y_of(0.5), y_of(0.618));
            let quad = [(x1 as f32, a1 as f32), (x2 as f32, a2 as f32), (x2 as f32, b2 as f32), (x1 as f32, b1 as f32)];
            crate::shapes::poly_fill(pm, 0.0, 0.0, &quad, yellow, 0.15);
        }
        for l in niveaux {
            let (ya, yb) = y_of(l);
            if l == 0.0 || l == 1.0 {
                trait_plein(pm, (x1, ya), (x2, yb), col, 2.0, 1.0);
            } else {
                super::tirets(pm, &[(x1, ya), (x2, yb)], 6.0, 4.0, col, 1.0, 0.7);
            }
            let v = (l * 100.0 * 100.0).round() / 100.0;
            text.draw(pm, &format!("{v}%"), ((x2 + 4.0) as f32, (yb + 4.0) as f32), 10.5, col, Align::Left);
        }
    }
    // 💧 liquidité : BSL (sommets) / SSL (creux) en tirets prolongés ; pris : pointillés jusqu'à la prise ; cassé : gris
    let (bsl, ssl) = (c("cBSL"), c("cSSL"));
    for l in &z.liqs {
        let col = if l.high { bsl } else { ssl };
        let x0 = g.x(l.from.max(g.first));
        let y = g.y(l.price);
        let w = if l.eq { 2.0 } else { 1.0 };
        match l.end {
            None => super::tirets(pm, &[(x0, y), (g.right(), y)], 6.0, 4.0, col, w, 1.0),
            Some(e) => {
                let x1 = g.x(e.min(g.last));
                if x1 > x0 {
                    let (cc, a) = if l.swept { (col, 1.0) } else { (gray, 0.5) };
                    super::tirets(pm, &[(x0, y), (x1, y)], 1.5, 3.0, cc, w, a);
                }
            }
        }
        if l.eq && l.end.is_none() {
            let lbl = if l.high { "EQH 💧" } else { "EQL 💧" };
            text.draw(pm, lbl, (x0 as f32, (y + if l.high { -5.0 } else { 13.0 }) as f32), 10.5, col, Align::Left);
        }
    }
    // étiquettes : CONF, Sweep BSL / SSL (🔥 avec une vector), zone forte touchée (▲ ▼), cassures (flèches, drapeaux)
    if g.vp.bar_spacing >= 2.0 {
        let fleche = |pm: &mut Pixmap, x: f64, y: f64, up: bool, col: Rgb, s: f64| {
            let k = if up { 1.0 } else { -1.0 };
            let pts = [
                (x as f32, (y - k * s) as f32),
                ((x + s * 0.8) as f32, y as f32),
                ((x + s * 0.3) as f32, y as f32),
                ((x + s * 0.3) as f32, (y + k * s) as f32),
                ((x - s * 0.3) as f32, (y + k * s) as f32),
                ((x - s * 0.3) as f32, y as f32),
                ((x - s * 0.8) as f32, y as f32),
            ];
            crate::shapes::poly_fill(pm, 0.0, 0.0, &pts, col, 1.0);
        };
        for &(i, s) in z.signals.iter().filter(|s| (g.first..=g.last).contains(&s.0)) {
            let b = &g.bars[i];
            let x = g.x(i);
            let (yh, yl) = (g.y(b.high) - 6.0, g.y(b.low) + 6.0);
            match s {
                S::ConfLong => etiquette(pm, x, yl, &["CONF".into()], lime, 1.0, black, false),
                S::ConfShort => etiquette(pm, x, yh, &["CONF".into()], red, 1.0, white, true),
                S::SweepSsl { vec } => etiquette(pm, x, yl, &[format!("{}💧 Sweep SSL", if vec { "🔥" } else { "" })], ssl, 0.8, black, false),
                S::SweepBsl { vec } => etiquette(pm, x, yh, &[format!("{}💧 Sweep BSL", if vec { "🔥" } else { "" })], bsl, 0.8, white, true),
                S::StrongDemand => text.draw(pm, "▲", (x as f32, (yl + 9.0) as f32), 9.0, lime, Align::Center),
                S::StrongSupply => text.draw(pm, "▼", (x as f32, (yh - 1.0) as f32), 9.0, red, Align::Center),
                S::PatBreakUp => fleche(pm, x, yl + 12.0, true, lime, 8.0),
                S::PatBreakDn => fleche(pm, x, yh - 12.0, false, red, 8.0),
                S::NeckUp | S::NeckDn => {
                    let (up, col) = (s == S::NeckUp, if s == S::NeckUp { lime } else { red });
                    let (y0, y1) = if up { (yl, yl + 16.0) } else { (yh - 16.0, yh) };
                    trait_plein(pm, (x, y0), (x, y1), col, 1.5, 1.0);
                    let fy = y0;
                    let flag = [(x as f32, fy as f32), ((x + 9.0) as f32, (fy + 3.5) as f32), (x as f32, (fy + 7.0) as f32)];
                    crate::shapes::poly_fill(pm, 0.0, 0.0, &flag, col, 1.0);
                }
            }
        }
    }
    // 📊 Vector Watchlist, en haut à droite de la zone des bougies
    if p.on("showTab") && !z.table.is_empty() {
        let (blue, fuchsia, silver) = (on_chart(hex(0x2196f3)), on_chart(hex(0xe040fb)), on_chart(hex(0xb2b5be)));
        // 📋 (tableaux-1) moved / resized by the user like every table (top right by default)
        let id = crate::tableaux::Id::native(Ind::VectorZone, "", crate::tableaux::interrupteur(p.params, ""));
        let lay = crate::tableaux::layout(&id.key);
        let (rw1, cols1) = (18.0, [54.0, 32.0, 74.0, 48.0, 84.0]);
        let n = z.table.len() as f64 + 2.0;
        let mesure = |s: f64| ((cols1.iter().sum::<f64>() + 8.0) * s * lay.wx, rw1 * s * lay.hy * n + 6.0 * s);
        let defaut = |w: f64, h: f64| crate::shapes::Rect::new(g.right() - w - 8.0, 34.0, w, h);
        let Some((r, s)) = super::placer(&id, g.price_area(), &mesure, &defaut) else {
            return;
        };
        // 🎨 (tour 2, F4) on the tables' layer: over the lines and the walls
        crate::tableaux::dessiner(pm, r, |pm| {
            let (rw, cols) = (rw1 * s * lay.hy, cols1.map(|c| c * s * lay.wx));
            let (x0, y0) = (r.x, r.y);
            crate::shapes::round_fill(pm, r, 6.0, Rgb(0x13, 0x17, 0x22), 0.95);
            let fs = 11.0 * s as f32;
            let cell = |pm: &mut Pixmap, col: usize, row: usize, s2: &str, c: Rgb| {
                let x = x0 + 4.0 * s + cols[..col].iter().sum::<f64>();
                text.draw(
                    pm,
                    s2,
                    ((x + 4.0 * s) as f32, (y0 + rw * 0.5 + 6.0 * s + row as f64 * rw) as f32),
                    fs,
                    c,
                    Align::Left,
                );
            };
            for (k, head) in ["VECTOR", "DIR", "STATUS", "VOL %", "CLOSE"].iter().enumerate() {
                cell(pm, k, 0, head, white);
            }
            for (r, row) in z.table.iter().enumerate() {
                cell(pm, 0, r + 1, &row.name, white);
                match row.cell {
                    Some((up, v, pct, left)) => {
                        cell(pm, 1, r + 1, if up { "▲" } else { "▼" }, if up { lime } else { red });
                        let (st, sc) = match v {
                            Some(vector_zone::VecKind::Climax) => ("CLIMAX", if up { lime } else { red }),
                            Some(vector_zone::VecKind::Vector) => ("VECTOR", if up { blue } else { fuchsia }),
                            None => ("NONE", gray),
                        };
                        cell(pm, 2, r + 1, st, sc);
                        cell(pm, 3, r + 1, &format!("{pct:.0}%"), white);
                        let (d, hh, mm, ss) = (left / 86_400, left % 86_400 / 3600, left % 3600 / 60, left % 60);
                        let t = if d > 0 {
                            format!("{d}d {hh:02}:{mm:02}:{ss:02}")
                        } else {
                            format!("{hh:02}:{mm:02}:{ss:02}")
                        };
                        cell(pm, 4, r + 1, &t, silver);
                    }
                    None => {
                        for k in 1..5 {
                            cell(pm, k, r + 1, "—", gray);
                        }
                    }
                }
            }
            let k = vector_zone::table_total(&z.table);
            cell(pm, 0, z.table.len() + 1, "TOTAL", white);
            let tot = if k > 0 { format!("{k} VECTOR") } else { "NO VECTOR".to_string() };
            cell(pm, 2, z.table.len() + 1, &tot, if k > 0 { lime } else { gray });
            cell(pm, 3, z.table.len() + 1, &format!("{k}/{}", z.table.len()), white);
        });
    }
}

pub(super) fn draw_patterns(pm: &mut Pixmap, text: &Text, g: &Geo, list: &[patterns::Pattern], vals: &[f64]) {
    let p = argos_script::inputs::Inputs {
        params: &patterns::PARAMS,
        vals,
    };
    let seg = |pm: &mut Pixmap, a: (f64, f64), b: (f64, f64), c: Rgb, w: f32, dash: Option<(f64, f64)>| match dash {
        Some((on, off)) => super::tirets(pm, &[a, b], on, off, c, w, 1.0),
        None => lines(pm, 0.0, 0.0, &[&[(a.0 as f32, a.1 as f32), (b.0 as f32, b.1 as f32)]], c, w),
    };
    for o in list {
        let c = col(
            &p,
            if o.bias > 0 {
                "cBull"
            } else if o.bias < 0 {
                "cBear"
            } else {
                "cNeutral"
            },
        );
        let pts: Vec<(f64, f64)> = o.pts.iter().map(|q| (g.x(q.i), g.y(q.p))).collect();
        // 📐 canal de Fibonacci : les niveaux parallèles à la ligne de base, jusqu'au bord droit, avec leur valeur
        if let Some((a, b, d)) = o.fib {
            let k = (b.p - a.p) / ((b.i as f64 - a.i as f64).max(1.0));
            let end = g.last.max(b.i);
            for lv in patterns::FIB_LEVELS {
                let (p0, p1) = (a.p + lv * d, a.p + lv * d + k * (end as f64 - a.i as f64));
                let edge = lv == 0.0 || lv == 1.0;
                seg(
                    pm,
                    (g.x(a.i), g.y(p0)),
                    (g.x(end), g.y(p1)),
                    c,
                    if edge { 1.4 } else { 1.0 },
                    (!edge).then_some((4.0, 4.0)),
                );
                text.draw(pm, &format!("{lv}"), ((g.x(end) - 4.0) as f32, (g.y(p1) - 3.0) as f32), 10.0, c, Align::Right);
            }
        } else {
            for w in pts.windows(2) {
                seg(pm, w[0], w[1], c, 1.5, None);
            }
        }
        for &(x, y) in &pts {
            crate::fill(pm, x - 2.0, y - 2.0, 4.0, 4.0, c, 1.0);
        }
        for &(a, b) in &o.lines {
            let end = o.end.max(b.i);
            let k = (b.p - a.p) / ((b.i as f64 - a.i as f64).max(1.0));
            let pe = a.p + k * (end as f64 - a.i as f64);
            seg(pm, (g.x(a.i), g.y(a.p)), (g.x(end), g.y(pe)), c, 1.2, Some((5.0, 4.0)));
        }
        if let Some(t) = o.target {
            let from = o.end;
            let x0 = g.x(from);
            seg(pm, (x0, g.y(t)), (x0 + 60.0, g.y(t)), c, 1.0, Some((2.0, 3.0)));
            text.draw(pm, "objectif", ((x0 + 64.0) as f32, (g.y(t) + 4.0) as f32), 10.5, c, Align::Left);
        }
        let top = o.pts.iter().max_by(|a, b| a.p.total_cmp(&b.p)).copied().unwrap_or(o.pts[0]);
        let label = format!(
            "{} {}{}",
            if o.bias > 0 {
                "▲"
            } else if o.bias < 0 {
                "▼"
            } else {
                "◆"
            },
            o.short,
            if o.conf { " ✓" } else { "" }
        );
        let w = text.width(&label, 11.0) as f64 + 10.0;
        let (lx, ly) = (g.x(top.i), g.y(top.p) - 14.0);
        if lx > -w && lx < g.right() + w && ly > 0.0 && ly < g.price_h {
            crate::fill(pm, lx - w / 2.0, ly - 9.0, w, 17.0, c, 0.92);
            text.draw(pm, &label, (lx as f32, (ly + 4.0) as f32), 11.0, crate::palette::ink_on(c), Align::Center);
        }
    }
}

pub(super) fn draw_pa(pm: &mut Pixmap, text: &Text, g: &Geo, ob: &order_blocks::OrderBlocks, vals: &[f64]) {
    let p = argos_script::inputs::Inputs {
        params: &order_blocks::PARAMS,
        vals,
    };
    let right = g.right();
    let hline = |pm: &mut Pixmap, from: usize, y: f64, c: Rgb, alpha: f32, dash: bool| {
        let x0 = g.x(from).max(0.0);
        if x0 >= right || y < 0.0 || y > g.price_h {
            return;
        }
        if dash {
            super::tirets(pm, &[(x0, y), (right, y)], 2.0, 3.0, c, 1.0, 1.0);
        } else {
            crate::fill(pm, x0, y, right - x0, 1.0, c, alpha);
        }
    };
    // une zone active va jusqu'au bord droit ; une zone cassée s'arrête à la cassure et ses bords continuent
    let show = |pm: &mut Pixmap, z: &order_blocks::Zone, fill_c: Rgb, broken_c: Rgb| {
        let mid = (z.high + z.low) / 2.0;
        match z.broken_at {
            Some(b) => {
                zone(pm, g, z.origin, Some(b), z.high, z.low, fill_c, 0.2);
                hline(pm, b, g.y(z.high), broken_c, 0.6, false);
                hline(pm, b, g.y(z.low), broken_c, 0.6, false);
                hline(pm, z.origin, g.y(mid), broken_c, 1.0, true);
            }
            None => {
                if z.origin <= g.last {
                    let x0 = g.x(z.origin).max(0.0);
                    let (y0, y1) = (g.y(z.high).min(g.y(z.low)), g.y(z.high).max(g.y(z.low)));
                    crate::fill(pm, x0, y0, (right - x0).max(0.0), (y1 - y0).max(1.0), fill_c, 0.2);
                }
                hline(pm, z.origin, g.y(mid), fill_c, 1.0, true);
            }
        }
    };
    // couleurs : celles des réglages, ou vert / mauve (option « Couleurs »)
    let couleur = |k: usize, key: &str| {
        if p.pick("palette") == 1 {
            crate::palette::on_chart(hex(order_blocks::VERT_MAUVE[k]))
        } else {
            col(&p, key)
        }
    };
    let (c_up, c_up_b, c_dn, c_dn_b) = (
        couleur(0, "coulHaut"),
        couleur(1, "coulHautCasse"),
        couleur(2, "coulBas"),
        couleur(3, "coulBasCasse"),
    );
    // étiquette d'une zone active, au bord droit : « BUY · Pwr 58 % » / « SELL · Pwr 72 % »
    let etiquette = |pm: &mut Pixmap, z: &order_blocks::Zone, buy: bool, c: Rgb| {
        if z.broken_at.is_some() || z.origin > g.last {
            return;
        }
        let mut s = vec![];
        if p.on("buySell") {
            s.push(if buy { "BUY".to_string() } else { "SELL".to_string() });
        }
        if p.on("pwr") {
            s.push(format!("Pwr {:.0} %", z.pwr));
        }
        if s.is_empty() {
            return;
        }
        // pastille au-dessus de la zone, collée au bord droit (comme la maquette) : fond de la couleur de la zone
        let s = s.join(" · ");
        let w = text.width(&s, 10.5) as f64 + 10.0;
        let y = g.y(z.high).min(g.y(z.low)) - 18.0;
        crate::shapes::round_fill(pm, crate::shapes::Rect::new(right - 4.0 - w, y, w, 16.0), 4.0, c, 0.9);
        text.draw(
            pm,
            &s,
            ((right - 4.0 - w / 2.0) as f32, (y + 12.0) as f32),
            10.5,
            crate::palette::ink_on(c),
            Align::Center,
        );
    };
    let (nu, nd) = (p.get("nHaut").max(0.0) as usize, p.get("nBas").max(0.0) as usize);
    for z in ob.rising.iter().take(nu) {
        show(pm, z, c_up, c_up_b);
        etiquette(pm, z, true, c_up);
    }
    for z in ob.falling.iter().take(nd) {
        show(pm, z, c_dn, c_dn_b);
        etiquette(pm, z, false, c_dn);
    }
    // BULL / BEAR n/4 à l'entrée du prix dans une zone : sous la bougie (BULL) ou au-dessus (BEAR)
    if p.on("n4") && g.vp.bar_spacing >= 2.0 {
        for t in ob.notes.iter().filter(|t| (g.first..=g.last).contains(&t.at)) {
            let b = &g.bars[t.at];
            let (c, s) = if t.bull {
                (c_up, format!("BULL {}/4", t.n))
            } else {
                (c_dn, format!("BEAR {}/4", t.n))
            };
            let w = text.width(&s, 10.5) as f64 + 10.0;
            let x = g.x(t.at);
            let y = if t.bull { g.y(b.low) + 6.0 } else { g.y(b.high) - 6.0 - 16.0 };
            crate::shapes::round_fill(pm, crate::shapes::Rect::new(x - w / 2.0, y, w, 16.0), 4.0, c, 0.9);
            text.draw(pm, &s, (x as f32, (y + 12.0) as f32), 10.5, crate::palette::ink_on(c), Align::Center);
        }
    }
    if p.on("polarite") {
        for f in ob.flips.iter().filter(|f| (g.first..=g.last).contains(&f.at)) {
            let (c, y) = if f.support {
                (c_up, g.y(f.price) + 22.0)
            } else {
                (c_dn, g.y(f.price) - 14.0)
            };
            text.draw(pm, if f.support { "▲" } else { "▼" }, (g.x(f.at) as f32, y as f32), 10.0, c, Align::Center);
        }
    }
    if p.on("tableau") {
        let count = |v: &[order_blocks::Zone]| {
            (
                v.iter().filter(|z| z.broken_at.is_none()).count(),
                v.iter().filter(|z| z.broken_at.is_some()).count(),
            )
        };
        let ((ua, ub), (da, db)) = (count(&ob.rising), count(&ob.falling));
        let pol = ob
            .flips
            .last()
            .map_or("—", |f| if f.support { "▲ support confirmé" } else { "▼ résistance confirmée" });
        let rows = [
            ("Zones haussières", format!("{ua} actives (+ {ub} cassées)")),
            ("Zones baissières", format!("{da} actives (+ {db} cassées)")),
            ("Polarité", pol.to_string()),
        ];
        table(pm, text, g.price_area(), "Argos Price Action", &rows, &p, "", Ind::PriceAction);
    }
}

/// Argos OB · BB · FVG · iFVG comme v1 : boîtes depuis la bougie d'origine (N actives par côté, les invalidées en plus
/// pâle si demandé), étiquettes OB+ / BB− / FVG+ / iFVG, BOS / CHoCH (tirets), EQH / EQL (✕ quand pris), tableau.
pub(super) fn draw_smc(pm: &mut Pixmap, text: &Text, g: &Geo, s: &smc::Smc, vals: &[f64]) {
    let p = argos_script::inputs::Inputs { params: &smc::PARAMS, vals };
    let col = |k: &str| crate::palette::on_chart(p.rgb(k));
    // index des zones → bougies de la chart (zones d'une UT supérieure : par l'heure, la fin au bout de la bougie HTF)
    let n = g.bars.len();
    let at = |t: i64| g.bars.partition_point(|b| b.time < t).min(n.saturating_sub(1));
    let (i_of, e_of): (Box<dyn Fn(usize) -> usize>, Box<dyn Fn(usize) -> usize>) = match (&s.htf_bars, s.tf) {
        (Some(hb), Some(tf)) => (
            Box::new(move |i| at(hb[i.min(hb.len() - 1)].time)),
            Box::new(move |i| at(hb[i.min(hb.len() - 1)].time + tf)),
        ),
        _ => (Box::new(|i| i), Box::new(|i| i)),
    };
    let tag = if s.htf_bars.is_some() {
        [" ", ["", "15m", "1h", "4H", "1D"][p.pick("tf").min(4)]].concat()
    } else {
        String::new()
    };
    let op = (p.get("opa") / 100.0) as f32;
    let n_max = |k: &str| p.len(k);
    let keep = |list: Vec<(usize, Option<usize>, f64, f64)>, max: usize| -> Vec<(usize, Option<usize>, f64, f64)> {
        let act: Vec<_> = list.iter().filter(|z| z.1.is_none()).copied().collect();
        let old: Vec<_> = if p.on("show_mit") {
            list.iter().filter(|z| z.1.is_some()).copied().collect()
        } else {
            vec![]
        };
        old[old.len().saturating_sub(max)..]
            .iter()
            .chain(&act[act.len().saturating_sub(max)..])
            .copied()
            .collect()
    };
    let mut boxes = |list: Vec<(usize, Option<usize>, f64, f64)>, c: Rgb, label: &str, dash: bool| {
        for (i0, end, top, bot) in list {
            let (from, to) = (i_of(i0), end.map(&e_of));
            zone(pm, g, from, to, top, bot, c, if end.is_some() { op * 0.45 } else { op });
            if dash {
                let (y0, y1) = (g.y(top), g.y(bot));
                let x1 = to.map_or(g.right(), |t| g.x(t)).min(g.right());
                for yy in [y0, y1] {
                    super::tirets(pm, &[(g.x(from).max(0.0), yy), (x1, yy)], 4.0, 3.0, c, 1.0, 1.0);
                }
            }
            if to.is_none_or(|t| t >= g.first) && from <= g.last {
                let x = g.x(from).max(2.0) + 4.0;
                text.draw(
                    pm,
                    &format!("{label}{tag}"),
                    (x as f32, (g.y(top).min(g.y(bot)) + 11.0) as f32),
                    10.0,
                    c,
                    Align::Left,
                );
            }
        }
    };
    let obs = |up: bool, brk: bool| {
        s.order_blocks
            .iter()
            .filter(|o| o.up == up && o.breaker == brk)
            .map(|o| (o.i0, o.end, o.top, o.bot))
            .collect::<Vec<_>>()
    };
    let fvgs = |up: bool, inv: bool| {
        s.fvgs
            .iter()
            .filter(|f| f.up == up && f.inverse == inv)
            .map(|f| (f.i0, f.end, f.top, f.bot))
            .collect::<Vec<_>>()
    };
    for side in [true, false] {
        if p.on("show_bb") {
            boxes(
                keep(obs(side, true), n_max("ob_n")),
                col(if side { "c_bbu" } else { "c_bbd" }),
                if side { "BB+" } else { "BB−" },
                true,
            );
        }
        if p.on("show_ob") {
            boxes(
                keep(obs(side, false), n_max("ob_n")),
                col(if side { "c_obu" } else { "c_obd" }),
                if side { "OB+" } else { "OB−" },
                false,
            );
        }
        if p.on("show_fvg") {
            boxes(
                keep(fvgs(side, false), n_max("fvg_n")),
                col(if side { "c_fvu" } else { "c_fvd" }),
                if side { "FVG+" } else { "FVG−" },
                false,
            );
            if p.on("show_ifvg") {
                boxes(keep(fvgs(side, true), n_max("fvg_n")), col("c_ifv"), if side { "iFVG−" } else { "iFVG+" }, true);
            }
        }
    }
    // BOS / CHoCH : les N dernières affichées
    let shown: Vec<&smc::StructureBreak> = s
        .breaks
        .iter()
        .filter(|b| {
            if matches!(b.kind, smc::BreakType::Choch) {
                p.on("show_choch")
            } else {
                p.on("show_bos")
            }
        })
        .collect();
    for b in &shown[shown.len().saturating_sub(n_max("st_n"))..] {
        let (i0, i1) = (i_of(b.i0), e_of(b.i1));
        if i1 < g.first || i0 > g.last {
            continue;
        }
        let c = if b.up { col("c_bull") } else { col("c_bear") };
        let (x0, x1, y) = (g.x(i0), g.x(i1), g.y(b.price));
        let choch = matches!(b.kind, smc::BreakType::Choch);
        if choch {
            super::tirets(pm, &[(x0, y), (x1, y)], 4.0, 3.0, c, 1.0, 1.0);
        } else {
            seg(pm, (x0, y), (x1, y), c, 1.0);
        }
        let label = format!("{}{tag}", if choch { "CHoCH" } else { "BOS" });
        text.draw(
            pm,
            &label,
            (((x0 + x1) / 2.0) as f32, (if b.up { y - 3.0 } else { y + 12.0 }) as f32),
            10.0,
            c,
            Align::Center,
        );
    }
    if p.on("show_eq") {
        for e in &s.equal[s.equal.len().saturating_sub(8)..] {
            let (i0, x1) = (i_of(e.i0), e.swept.map_or(g.right(), |k| g.x(e_of(k))));
            let (x0, y) = (g.x(i0), g.y(e.price));
            super::tirets(pm, &[(x0, y), (x1.min(g.right()), y)], 3.0, 3.0, col("c_liq"), 1.0, 1.0);
            let label = format!("{}{}", if e.up { "EQH" } else { "EQL" }, if e.swept.is_some() { " ✕" } else { "" });
            text.draw(
                pm,
                &label,
                ((x0 + 4.0) as f32, (if e.up { y - 3.0 } else { y + 12.0 }) as f32),
                10.0,
                col("c_liq"),
                Align::Left,
            );
        }
    }
    if p.on("showTable") {
        let act_ob: Vec<_> = s.order_blocks.iter().filter(|o| o.end.is_none() && !o.breaker).collect();
        let act_bb: Vec<_> = s.order_blocks.iter().filter(|o| o.end.is_none() && o.breaker).collect();
        let act_f: Vec<_> = s.fvgs.iter().filter(|f| f.end.is_none()).collect();
        let last = g.bars.last().map_or(0.0, |b| b.close);
        let near = act_ob
            .iter()
            .chain(&act_bb)
            .map(|o| (if o.breaker { "BB" } else { "OB" }, o.top, o.bot))
            .chain(act_f.iter().map(|f| (if f.inverse { "iFVG" } else { "FVG" }, f.top, f.bot)))
            .min_by(|a, b| {
                (last - a.1)
                    .abs()
                    .min((last - a.2).abs())
                    .total_cmp(&(last - b.1).abs().min((last - b.2).abs()))
            });
        let rows = vec![
            (
                "Structure",
                match s.trend {
                    1 => "Haussière ⬆️",
                    -1 => "Baissière ⬇️",
                    _ => "—",
                }
                .to_string(),
            ),
            (
                "OB actifs",
                format!("{} + / {} −", act_ob.iter().filter(|o| o.up).count(), act_ob.iter().filter(|o| !o.up).count()),
            ),
            (
                "Breakers (BB)",
                format!("{} + / {} −", act_bb.iter().filter(|o| o.up).count(), act_bb.iter().filter(|o| !o.up).count()),
            ),
            (
                "FVG actifs",
                format!(
                    "{} · iFVG {}",
                    act_f.iter().filter(|f| !f.inverse).count(),
                    act_f.iter().filter(|f| f.inverse).count()
                ),
            ),
            (
                "Zone la plus proche",
                near.map_or("—".into(), |(k, t, b)| {
                    format!(
                        "{k} {} – {}",
                        crate::viewport::format_price(b, crate::viewport::data_decimals(g.bars)),
                        crate::viewport::format_price(t, crate::viewport::data_decimals(g.bars))
                    )
                }),
            ),
        ];
        table(pm, text, g.price_area(), "🧱 Argos OB · BB · FVG · iFVG", &rows, &p, "", Ind::Smc);
    }
}

/// Argos SMC : zones (OB, Breakers, FVG, iFVG), BOS / CHoCH, Premium / Discount, OTE — affichage et couleurs selon les
/// inputs de v1 (Afficher les Order Blocks, FVG, Premium / Discount, OTE…).
pub(super) fn draw_smc_pro(pm: &mut Pixmap, text: &Text, g: &Geo, s: &smc_pro::SmcPro, vals: &[f64]) {
    let p = argos_script::inputs::Inputs {
        params: &smc_pro::PARAMS,
        vals,
    };
    let col = |k: &str| crate::palette::on_chart(p.rgb(k));
    let (bull, bear, inv) = (col("colBull"), col("colBear"), col("colInv"));
    let z = &s.zones;
    let sets = [
        (p.on("showOB"), &z.bull_ob, bull),
        (p.on("showOB"), &z.bear_ob, bear),
        (p.on("showBB"), &z.bull_bb, bull),
        (p.on("showBB"), &z.bear_bb, bear),
        (p.on("showFVG"), &z.bull_fvg, bull),
        (p.on("showFVG"), &z.bear_fvg, bear),
        (p.on("showIFVG"), &z.bull_ifvg, inv),
        (p.on("showIFVG"), &z.bear_ifvg, inv),
    ];
    for (on, list, c) in sets {
        if on {
            for zz in list.iter() {
                zone(pm, g, zz.left, None, zz.top, zz.bot, c, 0.12);
            }
        }
    }
    if p.on("showBos") {
        for e in s.events.iter().filter(|e| e.at >= g.first && e.from <= g.last) {
            let c = if e.up { bull } else { bear };
            let (x0, x1, y) = (g.x(e.from), g.x(e.at), g.y(e.level));
            seg(pm, (x0, y), (x1, y), c, 1.0);
            text.draw(
                pm,
                if e.choch { "CHoCH" } else { "BOS" },
                (((x0 + x1) / 2.0) as f32, (y - 3.0) as f32),
                10.0,
                c,
                Align::Center,
            );
        }
    }
    // Premium / Discount : ligne d'équilibre (et zones si demandé)
    if p.on("showPD") && s.range_top.is_finite() && s.range_bot.is_finite() && s.range_top > s.range_bot {
        let (yt, ye, yb) = (g.y(s.range_top), g.y(s.equilibrium), g.y(s.range_bot));
        crate::fill(pm, 0.0, yt.min(ye), g.right(), (ye - yt).abs(), bear, 0.05);
        crate::fill(pm, 0.0, ye.min(yb), g.right(), (yb - ye).abs(), bull, 0.05);
        crate::fill(pm, 0.0, ye, g.right(), 1.0, col("colEq"), 0.7);
        text.draw(pm, "Équilibre", ((g.right() - 6.0) as f32, (ye - 3.0) as f32), 10.0, col("colEq"), Align::Right);
    }
    if p.on("showOte") && s.ote_top.is_finite() && s.ote_bot.is_finite() && s.ote_top > s.ote_bot {
        let (y0, y1) = (g.y(s.ote_top), g.y(s.ote_bot));
        let c = col("colOte");
        crate::fill(pm, g.right() * 0.75, y0.min(y1), g.right() * 0.25, (y1 - y0).abs().max(1.0), c, 0.12);
        text.draw(pm, "OTE", ((g.right() - 6.0) as f32, (y0.min(y1) + 12.0) as f32), 10.0, c, Align::Right);
    }
}
