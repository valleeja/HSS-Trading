//! 📊 Volume et flux sur le prix : VWAP ancré, bulles, ruban, profil de volume, volume AI Trend, TVD.

use super::*;

/// Argos Anchored VWAP : la ligne depuis l'ancrage, ses bandes (cochées) et le remplissage entre la paire #1.
pub(super) fn draw_avwap(pm: &mut Pixmap, g: &Geo, a: &anchored_vwap::AnchoredVwap, vals: &[f64]) {
    let p = argos_script::inputs::Inputs {
        params: &anchored_vwap::PARAMS,
        vals,
    };
    let vis = || (g.first.max(a.from)..=g.last).filter(|&i| i < a.vwap.len());
    let line = |pm: &mut Pixmap, f: &dyn Fn(usize) -> f64, c: Rgb, w: f32| poly(pm, &vis().map(|i| (g.x(i), g.y(f(i)))).collect::<Vec<_>>(), c, w);
    let (cb, cf) = (col(&p, "cb"), col(&p, "cf"));
    if p.on("b1") {
        // un seul polygone (bande haute puis bande basse à l'envers) : un rectangle par bougie se chevauchait ou
        // laissait des trous au pixel près, d'où des stries verticales dans le fond
        let m = p.get("m1");
        let up = vis().map(|i| (g.x(i) as f32, g.y(a.vwap[i] + m * a.dev[i]) as f32));
        let down = vis().rev().map(|i| (g.x(i) as f32, g.y(a.vwap[i] - m * a.dev[i]) as f32));
        let pts: Vec<(f32, f32)> = up.chain(down).collect();
        if pts.len() > 2 {
            crate::shapes::poly_fill(pm, 0.0, 0.0, &pts, cf, p.opacity("cf"));
        }
    }
    for (on, k) in [("b1", "m1"), ("b2", "m2"), ("b3", "m3")] {
        if p.on(on) {
            let m = p.get(k);
            let cb2 = Rgb(
                (cb.0 as f32 * p.opacity("cb")) as u8,
                (cb.1 as f32 * p.opacity("cb")) as u8,
                (cb.2 as f32 * p.opacity("cb")) as u8,
            );
            line(pm, &|i| a.vwap[i] + m * a.dev[i], cb2, 1.0);
            line(pm, &|i| a.vwap[i] - m * a.dev[i], cb2, 1.0);
        }
    }
    line(pm, &|i| a.vwap[i], col(&p, "cv"), 2.0);
}

/// Radius (px) of Argos Bubbles' POC dot (« point jaune = POC, la rangée la plus chargée de la bougie »).
pub(super) const POC_R: f64 = 3.0;

/// Argos Bubbles comme v1 : une bulle par déséquilibre (taille ou opacité selon le volume), POC en point jaune cerclé
/// de sombre, tableau résumé. 🧮 Vrais trades seulement : où ils commencent est dit dans la légende (`bubbles_statut`).
pub(super) fn draw_bubbles(pm: &mut Pixmap, text: &Text, g: &Geo, b: &flow::Bubbles, vals: &[f64]) {
    let p = argos_script::inputs::Inputs {
        params: &flow::BUBBLES_PARAMS,
        vals,
    };
    let (cb, cs) = (col(&p, "cb"), col(&p, "cs"));
    for x in b.items.iter().filter(|x| (g.first..=g.last).contains(&x.i)) {
        let (cx, cy) = (g.x(x.i), g.y(x.price));
        let c = if x.buy { cb } else { cs };
        crate::shapes::round_fill(pm, crate::shapes::Rect::new(cx - x.r, cy - x.r, 2.0 * x.r, 2.0 * x.r), x.r, c, x.alpha as f32);
        if x.r > 4.0 {
            crate::shapes::round_stroke(pm, crate::shapes::Rect::new(cx - x.r, cy - x.r, 2.0 * x.r, 2.0 * x.r), x.r, c, 1.0);
        }
    }
    // the POC: a clearly visible yellow dot with a thin dark outline (it was a 1.8 px dot nobody saw)
    for &(i, price) in b.pocs.iter().filter(|(i, _)| (g.first..=g.last).contains(i)) {
        let (cx, cy) = (g.x(i), g.y(price));
        dot(pm, cx, cy, POC_R, hex(0xffd600));
        let r = crate::shapes::Rect::new(cx - POC_R, cy - POC_R, 2.0 * POC_R, 2.0 * POC_R);
        crate::shapes::round_stroke(pm, r, POC_R, hex(0x202020), 1.0);
    }
    if p.on("showTable") {
        let rows: Vec<(&str, String)> = b.dash.iter().map(|(k, v)| (*k, v.clone())).collect();
        table(pm, text, g.price_area(), "Argos Bubbles", &rows, &p, "", Ind::Bubbles);
    }
}

/// Argos Tape comme v1 : marqueurs de poussées de delta sur les bougies, et le ruban (heure · prix · volume) en tableau.
pub(super) fn draw_tape(pm: &mut Pixmap, text: &Text, g: &Geo, t: &flow::Tape, vals: &[f64]) {
    let p = argos_script::inputs::Inputs {
        params: &flow::TAPE_PARAMS,
        vals,
    };
    for m in t.marks(vals) {
        let Some(i) = g.bars.iter().position(|b| b.time == m.time).filter(|i| (g.first..=g.last).contains(i)) else {
            continue;
        };
        let b = &g.bars[i];
        let c = if m.up { hex(0x00e676) } else { hex(0xff4081) };
        let y = if m.up { g.y(b.low) + 16.0 } else { g.y(b.high) - 6.0 };
        text.draw(pm, m.text, (g.x(i) as f32, y as f32), 12.0, c, Align::Center);
    }
    if !p.on("showTable") {
        return;
    }
    let off = crate::settings::tz_offset("America/New_York", t.lines.first().map_or(0, |l| l.ts)).unwrap_or(0);
    let hms = |ts: i64| {
        let s = (ts + off).rem_euclid(86_400);
        format!("{:02}:{:02}:{:02}", s / 3600, s % 3600 / 60, s % 60)
    };
    // prices with the decimals of the chart's price scale
    let dec = crate::viewport::data_decimals(g.bars);
    let mut rows: Vec<(String, String)> = t
        .lines
        .iter()
        .take(p.get("nl").round() as usize)
        .map(|l| {
            let mv = match l.mv {
                1 => "+",
                -1 => "−",
                _ => "",
            };
            (
                format!(
                    "{}{}{mv}",
                    if p.on("tm") { format!("{}  ", hms(l.ts)) } else { String::new() },
                    crate::viewport::format_price(l.px, dec)
                ),
                format!("{}{} {}", if l.nb { "↻ " } else { "" }, if l.up { "🟢" } else { "🔴" }, kf(l.v)),
            )
        })
        .collect();
    if rows.is_empty() {
        rows.push(("En attente des mises à jour…".into(), "direct".into()));
    }
    if let Some(b) = g.bars.last() {
        rows.extend(tape_deltas(t.d, b, dec));
    }
    let rows: Vec<(&str, String)> = rows.iter().map(|(k, v)| (k.as_str(), v.clone())).collect();
    table(pm, text, g.price_area(), "Argos Tape", &rows, &p, "", Ind::Tape);
}

/// A signed number from its written magnitude `abs`: « + » / « − » (the true minus) only when what is written isn't
/// zero, so never « -0 » nor « +-0 ».
fn signe(v: f64, abs: String) -> String {
    if !abs.chars().any(|c| c.is_ascii_digit() && c != '0') {
        abs
    } else if v < 0.0 {
        format!("−{abs}")
    } else {
        format!("+{abs}")
    }
}

/// The last two rows of the Argos Tape table: the candle's volume delta `dv` and its price change (`dec` decimals,
/// those of the chart's price scale).
fn tape_deltas(dv: f64, b: &Bar, dec: usize) -> [(String, String); 2] {
    let dp = b.close - b.open;
    [
        ("Δ volume (bougie)".into(), signe(dv, kf(dv.abs()))),
        ("Δ prix (bougie)".into(), signe(dp, crate::viewport::format_price(dp.abs(), dec))),
    ]
}

/// 📋 Argos Volume Profile's summary table (the shared rows of `classic::profile_tableau`), its prices written as on
/// the chart's scale.
pub(super) fn profile_tableau(p: &classic::Profile, bars: &[Bar]) -> Vec<(&'static str, String)> {
    let d = crate::viewport::data_decimals(bars);
    classic::profile_tableau(p, bars, &|v| crate::viewport::format_price(v, d))
}

pub(super) fn draw_profile(pm: &mut Pixmap, text: &Text, g: &Geo, p: &classic::Profile, vals: &[f64]) {
    let inp = argos_script::inputs::Inputs {
        params: &v1::PROFILE_PARAMS,
        vals,
    };
    let col = |k: &str| crate::palette::on_chart(inp.rgb(k));
    let (poc_c, wid) = (col("cpoc"), inp.get("wid") / 100.0);
    let right = inp.pick("place") == 1;
    let xa = g.x(p.from);
    let unit = if right {
        g.right() * wid
    } else {
        ((g.x(g.bars.len() - 1) - xa) * wid).max(10.0)
    };
    let max = p.up.iter().zip(&p.down).map(|(u, d)| u + d).fold(0.0, f64::max);
    if max <= 0.0 {
        return;
    }
    let (oin, oout) = ((inp.get("oin") / 100.0) as f32, (inp.get("oout") / 100.0) as f32);
    for (k, (u, d)) in p.up.iter().zip(&p.down).enumerate() {
        let (y0, y1) = (g.y(p.bot + p.step * (k + 1) as f64), g.y(p.bot + p.step * k as f64));
        let (y, h) = (y0.min(y1) + 0.5, ((y1 - y0).abs() - 1.0).max(1.0));
        let alpha = if (p.va_lo..=p.va_hi).contains(&k) { oin } else { oout };
        let (wu, wd) = (u / max * unit, d / max * unit);
        let segs = if right {
            [(g.right() - wu - wd, wu, col("cup")), (g.right() - wd, wd, col("cdn"))]
        } else {
            [(xa, wu, col("cup")), (xa + wu, wd, col("cdn"))]
        };
        for (x, w, c) in segs {
            let (l, r) = (x.max(0.0), (x + w).min(g.right()));
            if r > l {
                crate::fill(pm, l, y, r - l, h, c, alpha);
            }
        }
    }
    let x0 = if right { 0.0 } else { xa.max(0.0) };
    if inp.on("valn") {
        for v in [p.bot + p.step * (p.va_hi + 1) as f64, p.bot + p.step * p.va_lo as f64] {
            let y = g.y(v);
            super::tirets(pm, &[(x0, y), (g.right(), y)], 4.0, 4.0, poc_c, 1.0, 1.0);
        }
    }
    let poc = p.bot + p.step * (p.poc as f64 + 0.5);
    let yp = g.y(poc);
    crate::fill(pm, x0, yp - 1.0, g.right() - x0, 2.0, poc_c, 1.0);
    if inp.on("lbl") {
        let label = format!("POC {}", crate::viewport::format_price(poc, crate::viewport::data_decimals(g.bars)));
        let tw = text.width(&label, 10.5) as f64 + 10.0;
        crate::fill(pm, g.right() - tw - 4.0, yp - 8.0, tw, 16.0, poc_c, 1.0);
        text.draw(
            pm,
            &label,
            ((g.right() - tw / 2.0 - 4.0) as f32, (yp + 4.0) as f32),
            10.5,
            crate::palette::WHITE,
            Align::Center,
        );
    }
}

/// Argos AI Trend sur le prix (sections 2 et 3 du Pine) : EMA 13 / 50 / 200 avec glow, patterns de retournement (▲ orange / ▼ rose
/// + nom), divergence Volume / Chaos RSI (fond orange + ▲ jaune) et 🚀 Ignition. Les bougies Volume Vector passent par `bar_color`.
pub(super) fn draw_ai_chart(pm: &mut Pixmap, text: &Text, g: &Geo, v: &argos_script::indicators::ai_trend_volume::Volume, vals: &[f64]) {
    let p = argos_script::inputs::Inputs {
        params: &ai_trend::PARAMS,
        vals,
    };
    let vis = |i: usize| (g.first..=g.last).contains(&i);
    // fond orange (80 % transparent) sur toute la hauteur du prix aux divergences Volume / Chaos RSI
    if p.on("ign_div") {
        for &i in v.vol_chaos_bull.iter().filter(|&&i| vis(i)) {
            let (x0, x1) = g.slot(i);
            crate::fill(pm, x0, 0.0, x1 - x0, g.price_h, hex(0xff9800), 0.2);
        }
    }
    // EMAs : couleur selon la clôture au-dessus / en dessous (comme le Pine), glow = traits larges transparents dessous
    if p.on("ema_show") {
        let close: Vec<f64> = g.bars.iter().map(|b| b.close).collect();
        let lime_red = |up: bool| if up { hex(0x00ff00) } else { hex(0xff0000) };
        let blue_purple = |up: bool| if up { hex(0x2196f3) } else { hex(0xba68c8) };
        let ema_line = |pm: &mut Pixmap, e: &[f64], pal: &dyn Fn(bool) -> Rgb, w: f32, glow: &[(f32, f32)]| {
            // morceaux de même couleur ; chaque morceau reprend le point d'avant pour rester collé
            let mut runs: Vec<(Rgb, Vec<(f32, f32)>)> = vec![];
            for i in g.first.max(1)..=g.last.min(e.len().saturating_sub(1)) {
                if !e[i].is_finite() || !e[i - 1].is_finite() {
                    continue;
                }
                let c = pal(close[i] >= e[i]);
                let (a, b) = ((g.x(i - 1) as f32, g.y(e[i - 1]) as f32), (g.x(i) as f32, g.y(e[i]) as f32));
                match runs.last_mut() {
                    Some((rc, pts)) if *rc == c && pts.last() == Some(&a) => pts.push(b),
                    _ => runs.push((c, vec![a, b])),
                }
            }
            for &(gw, ga) in glow {
                for (c, pts) in &runs {
                    crate::shapes::lines_alpha(pm, 0.0, 0.0, &[pts], *c, gw, ga);
                }
            }
            for (c, pts) in &runs {
                lines(pm, 0.0, 0.0, &[pts], *c, w);
            }
        };
        if p.on("e200_on") {
            let glow: &[(f32, f32)] = if p.on("glow200") { &[(20.0, 0.12), (9.0, 0.28)] } else { &[] };
            ema_line(pm, &v.ema200, &blue_purple, 3.0, glow);
        }
        if p.on("e50_on") {
            let glow: &[(f32, f32)] = if p.on("glow") { &[(10.0, 0.16), (5.0, 0.32)] } else { &[] };
            ema_line(pm, &v.ema50, &lime_red, 2.0, glow);
        }
        if p.on("e13_on") {
            ema_line(pm, &v.ema13, &lime_red, 1.0, &[]);
        }
    }
    // patterns : ▲ sous la bougie / ▼ au-dessus, puis le nom dans une étiquette de la même couleur
    let tri = |pm: &mut Pixmap, x: f64, y: f64, up: bool, c: Rgb| {
        let pts: [(f32, f32); 3] = if up {
            [(0.0, -5.0), (5.0, 4.0), (-5.0, 4.0)]
        } else {
            [(0.0, 5.0), (5.0, -4.0), (-5.0, -4.0)]
        };
        crate::shapes::poly_fill(pm, x, y, &pts, c, 1.0);
    };
    let mut below = vec![0.0f64; g.last + 1];
    let mut above = vec![0.0f64; g.last + 1];
    for (list, bull, show, ck) in [
        (&v.bull_patterns, true, p.on("pat_bull"), "pat_cbu"),
        (&v.bear_patterns, false, p.on("pat_bear"), "pat_cbe"),
    ] {
        if !show {
            continue;
        }
        let c = col(&p, ck);
        for &(i, name) in list.iter().filter(|x| vis(x.0)) {
            let b = &g.bars[i];
            let x = g.x(i);
            if bull {
                let y = g.y(b.low) + 10.0;
                tri(pm, x, y, true, c);
                if p.on("pat_lbl") {
                    tag(pm, text, x, y + 16.0, name, Rgb(0xff, 0xff, 0xff), c);
                }
                below[i] = if p.on("pat_lbl") { 34.0 } else { 18.0 };
            } else {
                let y = g.y(b.high) - 10.0;
                tri(pm, x, y, false, c);
                if p.on("pat_lbl") {
                    tag(pm, text, x, y - 16.0, name, Rgb(0xff, 0xff, 0xff), c);
                }
                above[i] = if p.on("pat_lbl") { 34.0 } else { 18.0 };
            }
        }
    }
    if p.on("ign_div") {
        for &i in v.vol_chaos_bull.iter().filter(|&&i| vis(i)) {
            let y = g.y(g.bars[i].low) + 10.0 + below[i];
            tri(pm, g.x(i), y, true, hex(0xffeb3b));
            below[i] += 16.0;
        }
    }
    if p.on("ign_show") {
        for &i in v.ignition.iter().filter(|&&i| vis(i)) {
            let (x, y) = (g.x(i), g.y(g.bars[i].low) + 22.0 + below[i]);
            crate::shapes::round_fill(pm, crate::shapes::Rect::new(x - 15.0, y - 13.0, 30.0, 26.0), 5.0, hex(0xff9800), 1.0);
            crate::shapes::poly_fill(pm, x, y - 13.0, &[(-5.0, 0.0), (0.0, -6.0), (5.0, 0.0)], hex(0xff9800), 1.0);
            text.draw(pm, "🚀", (x as f32, (y + 7.0) as f32), 17.0, Rgb(0xff, 0xff, 0xff), Align::Center);
        }
    }
}

/// Argos TVD Pro comme v1 : nuage entre EMA Fast et Slow, 4 EMA, lignes de divergence, repères, tableau de bord.
pub(super) fn draw_tvd(pm: &mut Pixmap, text: &Text, g: &Geo, t: &tvd::Tvd, vals: &[f64]) {
    let vals = &argos_script::inputs::Inputs { params: &tvd::PARAMS, vals };
    // nuage EMA Fast / Slow continu (comme le fill du Pine : 85 % de transparence), vert / rouge / gris selon la tendance
    super::cloud(pm, g, &|i| {
        let c = t.cloud.get(i).map(|k| match k {
            1 => (hex(0x22c55e), 0.15),
            -1 => (hex(0xef4444), 0.15),
            _ => (hex(0x94a3b8), 0.15),
        });
        (t.fast.get(i).copied().unwrap_or(f64::NAN), t.slow.get(i).copied().unwrap_or(f64::NAN), c)
    });
    for (v, c, w) in [
        (&t.fast, hex(0x22d3ee), 1.0),
        (&t.slow, hex(0xf59e0b), 2.0),
        (&t.mid, hex(0x3b82f6), 2.0),
        (&t.trend, hex(0xa855f7), 2.0),
    ] {
        poly(pm, &(g.first..=g.last).map(|i| (g.x(i), g.y(v[i]))).collect::<Vec<_>>(), c, w);
    }
    for d in t.divs.iter().filter(|d| d.i1 >= g.first && d.i0 <= g.last) {
        let (c, w) = match d.kind {
            tvd::MarkKind::Bull => (hex(0x22c55e), 2.0),
            tvd::MarkKind::HBull => (hex(0x86efac), 1.0),
            tvd::MarkKind::Bear => (hex(0xef4444), 2.0),
            _ => (hex(0xfca5a5), 1.0),
        };
        seg(pm, (g.x(d.i0), g.y(d.p0)), (g.x(d.i1), g.y(d.p1)), c, w);
    }
    // repères empilés quand plusieurs tombent sur la même bougie
    let mut stack: HashMap<(usize, bool), f64> = HashMap::new();
    for m in t.marks.iter().filter(|m| g.vp.bar_spacing >= 2.0 && (g.first..=g.last).contains(&m.index)) {
        let (b, below) = (&g.bars[m.index], m.kind.below());
        let off = stack.get(&(m.index, below)).copied().unwrap_or(0.0);
        let y = if below { g.y(b.low) + 9.0 + off } else { g.y(b.high) - 9.0 - off };
        let x = g.x(m.index);
        use tvd::MarkKind::*;
        let step = if m.kind.text().is_empty() { 11.0 } else { 23.0 };
        match m.kind {
            TrendBuy => crate::shapes::poly_fill(pm, x, y, &[(0.0, -5.0), (5.0, 4.0), (-5.0, 4.0)], hex(0x22c55e), 1.0),
            TrendSell => crate::shapes::poly_fill(pm, x, y, &[(0.0, 5.0), (5.0, -4.0), (-5.0, -4.0)], hex(0xef4444), 1.0),
            AiBuy => dot(pm, x, y, 3.5, hex(0x16a34a)),
            AiSell => dot(pm, x, y, 3.5, hex(0xdc2626)),
            RevBull | RevBear => crate::shapes::poly_fill(
                pm,
                x,
                y,
                &[(0.0, -4.5), (4.5, 0.0), (0.0, 4.5), (-4.5, 0.0)],
                if below { hex(0x86efac) } else { hex(0xfca5a5) },
                1.0,
            ),
            k => {
                let (fg, bg) = match k {
                    GoldSell => (crate::palette::WHITE, hex(0x7f1d1d)),
                    GoldBuy => (crate::palette::WHITE, hex(0x14532d)),
                    Bull => (crate::palette::WHITE, hex(0x16a34a)),
                    HBull => (hex(0x052e16), hex(0x86efac)),
                    Bear => (crate::palette::WHITE, hex(0xdc2626)),
                    _ => (hex(0x450a0a), hex(0xfca5a5)),
                };
                let y = if below { y + 6.0 } else { y - 6.0 };
                tag(pm, text, x, y, k.text(), fg, bg);
            }
        }
        *stack.entry((m.index, below)).or_insert(0.0) += step;
    }
    if !t.dash.is_empty() {
        table(pm, text, g.price_area(), "🤖🕯️ Argos TVD Pro", &t.dash, vals, "", Ind::Tvd);
    }
}

#[cfg(test)]
mod tests_tape {
    use super::*;

    /// 📋 Argos Tape (Jee, 22 Nov): never « -0 » nor « +-0 », the true « − », prices at the chart's decimals.
    #[test]
    fn deltas_du_tape() {
        let bar = |o: f64, c: f64| Bar {
            open: o,
            close: c,
            high: o.max(c),
            low: o.min(c),
            time: 0,
            volume: 1.0,
        };
        let v = |dv: f64, b: Bar, d: usize| tape_deltas(dv, &b, d).map(|(_, v)| v);
        assert_eq!(v(-0.3, bar(100.0, 100.0), 2), ["0", "0,00"]);
        assert_eq!(v(0.0, bar(100.004, 100.0), 2), ["0", "0,00"], "rounded to zero: no sign");
        assert_eq!(v(-1500.0, bar(100.0, 99.5), 1), ["−1,5K", "−0,5"]);
        assert_eq!(v(42.0, bar(60_000.0, 60_012.0), 0), ["+42", "+12"]);
        assert!(!v(-0.4, bar(1.0, 0.9999), 2).iter().any(|s| s.contains('-')), "never the ASCII minus");
        assert_eq!(signe(-3.0, "3".into()), "−3");
    }

    /// 📋🧱 The Argos Tape table is never under an Argos Murs pill (capture 202611221633-3): while the walls hold the
    /// right side it goes to the left corner (`table_corner_with_walls`), otherwise it keeps its top right corner.
    #[test]
    fn tableau_du_tape_hors_des_murs() {
        let vals = values(&Settings::new(), Ind::Tape);
        let p = argos_script::inputs::Inputs {
            params: &flow::TAPE_PARAMS,
            vals: &vals,
        };
        let rows = vec![("12:00:00  61 000,0", "🟢 1,2K".to_string())];
        let area = crate::shapes::Rect::new(0.0, 0.0, 1000.0, 600.0);
        let mut pm = crate::px::new(1000, 600).unwrap();
        for (murs, gauche) in [(true, true), (false, false)] {
            TABLES.with(|t| t.borrow_mut().clear());
            set_murs_a_droite(murs);
            table(&mut pm, &Text::default(), area, "Argos Tape", &rows, &p, "", Ind::Tape);
            let r = tableaux()[0];
            assert_eq!(r.x + r.w < 500.0, gauche, "walls {murs}: {r:?}");
        }
        set_murs_a_droite(false);
    }
}
