//! 🌊 Oscillateurs en bande sous la chart : TBO, AI Trend, Cipher (et sa confluence), aires autour de zéro, sous-fenêtres des classiques.

use super::*;

/// 📏 The automatic scale of a panel: the lowest and highest visible values ((∞, −∞) when there is none), without
/// NaN nor ±∞, and without absurd values — farther than 1 000 × the normal spread (between the 5th and 95th
/// percentiles) from it (Jee, 19 Nov: « toujours bien visible peu importe ce qui arrive à la chart »). A real peak
/// (10 or 100 × the spread) stays in the scale; with no spread (a flat series), nothing is left out.
pub(super) fn plage(vals: impl Iterator<Item = f64>) -> (f64, f64) {
    let mut v: Vec<f64> = vals.filter(|x| x.is_finite()).collect();
    let tout = |v: &[f64]| v.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &x| (a.min(x), b.max(x)));
    if v.len() < 20 {
        return tout(&v);
    }
    v.sort_by(f64::total_cmp);
    let (p5, p95) = (v[v.len() * 5 / 100], v[(v.len() * 95 / 100).min(v.len() - 1)]);
    let ecart = p95 - p5;
    if !(ecart > 0.0) {
        return tout(&v);
    }
    let (bas, haut) = (p5 - 1000.0 * ecart, p95 + 1000.0 * ecart);
    let gardees: Vec<f64> = v.into_iter().filter(|x| (bas..=haut).contains(x)).collect();
    tout(&gardees)
}

/// Argos TBO Pro Div dans sa bande, comme v1 : delta en zone (ou histogramme), ligne Signal, zéro en tirets, points de
/// break (Vector 200+ avec halo, validé, ou filtré pâle), lignes de divergence, tableau de statut (option).
pub(super) fn draw_tbo(pm: &mut Pixmap, text: &Text, g: &Geo, top: f64, t: Option<&tbo::Tbo>, vals: &[f64]) -> Option<BandAxis> {
    let t = t?;
    let p = argos_script::inputs::Inputs { params: &tbo::PARAMS, vals };
    let col = |k: &str| crate::palette::on_chart(p.rgb(k));
    let h = Ind::Tbo.band_h();
    let d = &t.core.delta;
    let vis = (g.first..=g.last).filter(|&i| i < d.len());
    let (a, b) = plage(vis.clone().flat_map(|i| [d[i], t.signal.get(i).copied().unwrap_or(f64::NAN)]));
    let m = [a, b].iter().filter(|v| v.is_finite()).fold(1e-12f64, |m, v| m.max(v.abs()));
    let (vlo, vhi) = view(Ind::Tbo, -m, m);
    let y = |v: f64| top + 20.0 + (vhi - v) / (vhi - vlo) * (h - 26.0);
    let y0 = y(0.0);
    tirets(pm, &[(0.0, y0), (g.right(), y0)], 4.0, 4.0, Rgb(0xe6, 0xe6, 0xeb), 1.0, 1.0);
    let (bull, bear) = (col("cBull"), col("cBear"));
    let hist = p.pick("oscStyle") == 1;
    if hist {
        // histogramme : une barre par bougie, de la largeur de sa place (bougies de volume : places inégales)
        for i in vis.clone().filter(|&i| d[i].is_finite()) {
            let (yv, x) = (y(d[i]), g.x(i));
            let left = if i > 0 { (x - g.x(i - 1)) / 2.0 } else { 2.0 };
            let right = if i + 1 < d.len() { (g.x(i + 1) - x) / 2.0 } else { left };
            let w = (left.min(right) * 1.4).max(1.0);
            crate::fill(
                pm,
                x - w / 2.0,
                yv.min(y0),
                w,
                (yv - y0).abs().max(1.0),
                if d[i] > 0.0 { bull } else { bear },
                0.9,
            );
        }
    } else {
        // zone en dégradé entre la courbe et 0 (comme le Pine : 50 % sur la courbe → transparent sur 0)
        let pts: Vec<(f64, f64)> = vis.clone().map(|i| (g.x(i), d[i])).collect();
        zero_area(pm, &pts, &y, bull, bear, 0.5);
    }
    if !hist {
        let idx: Vec<usize> = vis.clone().collect();
        let pts: Vec<(f64, f64)> = idx.iter().map(|&i| (g.x(i), y(d[i]))).collect();
        seg_couleurs(pm, &pts, &|k| Some(if d[idx[k]] > 0.0 { bull } else { bear }), 2.0);
    }
    if p.on("sig_on") {
        poly(
            pm,
            &vis.clone()
                .map(|i| (g.x(i), y(t.signal.get(i).copied().unwrap_or(f64::NAN))))
                .collect::<Vec<_>>(),
            col("sig_col"),
            2.0,
        );
    }
    if p.on("div_on") {
        for dv in t.divs.iter().filter(|dv| dv.pivot >= g.first && dv.prev <= g.last && dv.pivot < d.len()) {
            let c = col(if dv.bull { "cDivU" } else { "cDivD" });
            let (a, b) = ((g.x(dv.prev), y(d[dv.prev])), (g.x(dv.pivot), y(d[dv.pivot])));
            div_seg(pm, a, b, c, 1.2, if dv.hidden { p.pick("hid_style") } else { 2 });
        }
    }
    let r = (p.get("dotSize") / 2.0 + 1.0).max(2.0);
    // ▲ ▼ masqués : seulement les points (les signaux et les alertes restent, comme le Pine)
    let shown = |b: &&tbo::Break| p.on(if b.up { "dotsU" } else { "dotsD" });
    for b in t
        .breaks
        .iter()
        .filter(shown)
        .filter(|b| (g.first..=g.last).contains(&b.index) && b.index < d.len())
    {
        let (x, yy) = (g.x(b.index), y(d[b.index]));
        match b.kind {
            tbo::BreakKind::Vector => {
                let c = col(if b.up { "cV2u" } else { "cV2d" });
                crate::shapes::round_fill(
                    pm,
                    crate::shapes::Rect::new(x - r - 3.0, yy - r - 3.0, 2.0 * r + 6.0, 2.0 * r + 6.0),
                    r + 3.0,
                    c,
                    0.25,
                );
                dot(pm, x, yy, r, c);
            }
            tbo::BreakKind::Valid => dot(pm, x, yy, r, col(if b.up { "cBrkU" } else { "cBrkD" })),
            tbo::BreakKind::Filtered => crate::shapes::round_fill(
                pm,
                crate::shapes::Rect::new(x - 2.5, yy - 2.5, 5.0, 5.0),
                2.5,
                col(if b.up { "cBrkU" } else { "cBrkD" }),
                0.3,
            ),
        }
    }
    if p.on("dash") {
        // les 8 lignes du tableau du Pine (TBO PRECISION, SWING, ADX, HTF, DELTA, FILTRES, VECTOR, LAST)
        let k = t.core.delta.len().saturating_sub(1);
        let a = &t.active;
        let mode = match a.preset {
            tbo::Preset::Custom => "CUSTOM".to_string(),
            pr => {
                let name = match pr {
                    tbo::Preset::Scalping => "SCALPING",
                    tbo::Preset::Swing => "SWING",
                    _ => "DAY TRADE",
                };
                format!("{name}{}", if p.pick("preset") == 1 { " (AUTO)" } else { "" })
            }
        };
        let adx = t.adx.get(k).copied().filter(|v| v.is_finite());
        let trend_ok = !p.on("adx_on") || adx.is_some_and(|v| v >= a.adx_min);
        let up = g.bars.get(k).is_none_or(|b| b.close >= b.open);
        let arrow = |up: bool| if up { "▲" } else { "▼" };
        let rows = vec![
            ("Mode", mode),
            ("SWING", a.length.to_string()),
            ("ADX", adx.map_or("—".into(), |v| format!("{v:.1}"))),
            (
                "HTF",
                match t.htf {
                    tbo::HtfUsed::Off => "off".to_string(),
                    tbo::HtfUsed::Chart => "Graphique".to_string(),
                    tbo::HtfUsed::Seconds(s) => argos_core::Interval(s).short(),
                },
            ),
            ("DELTA", d.get(k).filter(|v| v.is_finite()).map_or("—".into(), |v| format!("{v:.4}"))),
            ("FILTRES", if trend_ok { "TREND OK" } else { "RANGE" }.to_string()),
            (
                "VECTOR",
                match t.vector.get(k) {
                    Some(2) => format!("200+ {}", if up { "BULL" } else { "BEAR" }),
                    Some(1) => format!("150+ {}", if up { "BULL" } else { "BEAR" }),
                    _ => "-".to_string(),
                },
            ),
            (
                "LAST",
                match t.last {
                    Some(tbo::LastEvent::Break { up, vector }) => {
                        format!("{}{}", if vector { "BO + VEC200 " } else { "BREAK " }, arrow(up))
                    }
                    Some(tbo::LastEvent::Divergence { bull, hidden }) => {
                        format!("{} {}", if hidden { "HID" } else { "REG" }, arrow(bull))
                    }
                    None => "-".to_string(),
                },
            ),
        ];
        let area = if p.pick("tblIn") == 1 {
            crate::shapes::Rect::new(0.0, top, g.right(), h)
        } else {
            g.price_area()
        };
        table(pm, text, area, "📐 TBO PRECISION", &rows, &p, "", Ind::Tbo);
    }
    let last = d.iter().rev().copied().find(|v| v.is_finite());
    Some(BandAxis {
        top,
        h,
        pad: (20.0, 6.0),
        lo: vlo,
        hi: vhi,
        last: last.map(|v| (v, if v > 0.0 { bull } else { bear })).into_iter().collect(),
    })
}

/// Couleur entre `a` et `b` (t de 0 à 1).
pub(super) fn mix(a: Rgb, b: Rgb, t: f64) -> Rgb {
    let t = t.clamp(0.0, 1.0);
    let m = |x: u8, y: u8| (x as f64 + (y as f64 - x as f64) * t).round() as u8;
    Rgb(m(a.0, b.0), m(a.1, b.1), m(a.2, b.2))
}

/// Argos AI Trend V Pro comme v1 : niveaux colorés, fonds des zones extrêmes, ligne en dégradé (survente → neutre →
/// surachat), ligne signal, points, divergences, points multi-UT par slot, boîte HTF. Tout suit les inputs.
pub(super) fn draw_ai(pm: &mut Pixmap, text: &Text, g: &Geo, top: f64, a: Option<&ai_trend::AiTrend>, vals: &[f64]) -> Option<BandAxis> {
    let a = a?;
    let p = argos_script::inputs::Inputs {
        params: &ai_trend::PARAMS,
        vals,
    };
    let col = |k: &str| crate::palette::on_chart(p.rgb(k));
    let h = Ind::AiTrend.band_h();
    let (vlo, vhi) = view(Ind::AiTrend, 0.0, 100.0);
    let y = |v: f64| top + 20.0 + (vhi - v) / (vhi - vlo) * (h - 26.0);
    let (ob, os, xh, xl, mid) = (p.get("ob"), p.get("os"), p.get("xh"), p.get("xl"), p.get("mid"));
    // fond gris entre suracheté et survendu (fill du Pine, 92 % transparent)
    if p.on("lv_show") {
        crate::fill(pm, 0.0, y(ob), g.right(), y(os) - y(ob), hex(0x787b86), 0.08);
    }
    // Zone Extrême BG (comme TradingView) : colonne sur toute la bande pour chaque bougie au-dessus du Very Red (rouge)
    // ou sous le Very Blue (bleu), 80 % transparente
    if p.on("bg_show") {
        for (list, c) in [(&a.extreme_high, col("c_bgh")), (&a.extreme_low, col("c_bgl"))] {
            for &i in list.iter().filter(|&&i| (g.first..=g.last).contains(&i)) {
                let x = g.x(i);
                let l = if i > 0 { (x - g.x(i - 1)) / 2.0 } else { 2.0 };
                let r = if i + 1 < a.ai.len() { (g.x(i + 1) - x) / 2.0 } else { l };
                crate::fill(pm, x - l, top, l + r + 0.5, h, c, 0.2);
            }
        }
    }
    // niveaux comme les hline du Pine : suracheté / survendu en tirets, extrêmes en trait plein, midline en pointillé
    if p.on("lv_show") {
        for (v, k, dash, alpha) in [
            (ob, "c_ob", Some((4.0, 4.0)), 0.6),
            (xh, "c_xh", None, 0.8),
            (mid, "c_mid", Some((1.5, 3.0)), 0.6),
            (os, "c_os", Some((4.0, 4.0)), 0.6),
            (xl, "c_xl", None, 0.8),
        ] {
            let yy = y(v);
            match dash {
                Some((on, off)) => tirets(pm, &[(0.0, yy), (g.right(), yy)], on, off, col(k), 1.0, alpha),
                None => crate::shapes::lines_alpha(pm, 0.0, 0.0, &[&[(0.0, yy as f32), (g.right() as f32, yy as f32)]], col(k), 1.0, alpha),
            }
        }
    }
    let vis = (g.first..=g.last).filter(|&i| i < a.ai.len());
    if p.on("sig_show") {
        let pts: Vec<(f32, f32)> = vis
            .clone()
            .filter(|&i| a.signal[i].is_finite())
            .map(|i| (g.x(i) as f32, y(a.signal[i]) as f32))
            .collect();
        crate::shapes::lines_alpha(pm, 0.0, 0.0, &[&pts], col("sig_col"), p.get("sig_w").max(1.0) as f32, p.opacity("sig_col"));
    }
    // ligne AI : dégradé selon la valeur (comme v1), sinon couleur neutre
    let grad = p.on("grad");
    let (lo, ne, hi) = (col("c_low"), col("c_neu"), col("c_high"));
    for i in vis.clone().skip(1).filter(|&i| a.ai[i].is_finite() && a.ai[i - 1].is_finite()) {
        let v = a.ai[i];
        let c = if !grad {
            ne
        } else if v < mid {
            mix(lo, ne, (v - os) / (mid - os).max(1e-9))
        } else {
            mix(ne, hi, (v - mid) / (ob - mid).max(1e-9))
        };
        seg(pm, (g.x(i - 1), y(a.ai[i - 1])), (g.x(i), y(v)), c, 2.0);
    }
    if p.on("dots_show") {
        for (list, c) in [(&a.buy, crate::palette::up()), (&a.sell, crate::palette::down())] {
            for &i in list.iter().filter(|i| (g.first..=g.last).contains(*i)) {
                dot(pm, g.x(i), y(a.ai[i]), 3.0, c);
            }
        }
    }
    // comme le Pine : 🤑 (« Show the money face »), lignes régulières (« Show Divergence Lines ») et cachées
    // (« Show Hidden Divergences ») ont chacune leur bouton
    let w = p.get("div_w") as f32;
    // 🤑 money face sur les divergences régulières (sous la ligne en haussier, au-dessus en baissier)
    if p.on("face_show") {
        let (off, size) = (p.get("face_off"), [16u32, 20, 26, 32][p.pick("face_size").min(3)]);
        for (list, sign) in [(&a.divs.bull, -1.0), (&a.divs.bear, 1.0)] {
            for d in list.iter().filter(|d| (g.first..=g.last).contains(&d.pivot) && d.pivot < a.ai.len()) {
                let yy = y(a.ai[d.pivot] + sign * off) + sign * -(size as f64) / 2.0;
                crate::icons::draw_emoji(pm, "🤑", g.x(d.pivot), yy, size);
            }
        }
    }
    // régulières en trait plein, cachées (hidden) en tirets par défaut (input « Style des divergences cachées »)
    let (reg, hid) = (p.on("div_show"), p.on("div_hid"));
    for (list, k, hidden) in [
        (&a.divs.bull, "c_dbull", false),
        (&a.divs.bear, "c_dbear", false),
        (&a.divs.hidden_bull, "c_hbull", true),
        (&a.divs.hidden_bear, "c_hbear", true),
    ] {
        if (hidden && !hid) || (!hidden && !reg) {
            continue;
        }
        for d in list.iter().filter(|d| d.pivot >= g.first && d.prev <= g.last && d.pivot < a.ai.len()) {
            let (from, to) = ((g.x(d.prev), y(a.ai[d.prev])), (g.x(d.pivot), y(a.ai[d.pivot])));
            div_seg(pm, from, to, col(k), w, if hidden { p.pick("hid_style") } else { 2 });
        }
    }
    // divergences multi-UT : un point par slot, à « Écart entre slots » × n sous la ligne (haussier) ou au-dessus (baissier)
    let gap = p.get("mtf_off");
    for m in a.mtf.iter().filter(|m| (g.first..=g.last).contains(&m.index) && m.index < a.ai.len()) {
        let n = m.slot; // emplacement 1 à 4
                        // rond de la taille du Pine (cercle `linewidth` = Taille : environ 2,5 × Taille de diamètre)
        let (c, r) = (col(&format!("m{n}_{}", if m.bull { "cb" } else { "cr" })), p.get(&format!("m{n}_sz")) * 1.25);
        let v = a.ai[m.index] + if m.bull { -gap } else { gap } * n as f64;
        crate::shapes::round_fill(
            pm,
            crate::shapes::Rect::new(g.x(m.index) - r, y(v) - r, 2.0 * r, 2.0 * r),
            r,
            c,
            p.opacity(&format!("m{n}_{}", if m.bull { "cb" } else { "cr" })),
        );
    }
    // boîte HTF : dans la bande (ou sur la chart) selon « Table : afficher dans »
    if let (true, Some(b)) = (p.on("htf_box"), &a.htf) {
        let tf = match b.seconds {
            s if s % 86_400 == 0 => format!("{}D", s / 86_400),
            s if s % 3_600 == 0 => format!("{}H", s / 3_600),
            s => format!("{}m", s / 60),
        };
        // comme la boîte du Pine : signal de l'UT, BUY / SELL / NEUTRAL, valeur, preset
        let zone = match b.zone {
            ai_trend::HtfZone::Buy => "BUY 🟢",
            ai_trend::HtfZone::Sell => "SELL 🔴",
            ai_trend::HtfZone::Neutral => "NEUTRAL ⚪",
        };
        let mut rows = vec![("Signal", tf), ("Zone", zone.to_string())];
        if p.on("htf_val") {
            rows.push(("AI", format!("{:.2}", b.value)));
        }
        rows.push(("Preset", a.preset.to_string()));
        let area = if p.pick("tblIn") == 1 {
            crate::shapes::Rect::new(0.0, top, g.right(), h)
        } else {
            g.price_area()
        };
        table(pm, text, area, "🤖 Argos AI Trend V Pro", &rows, &p, "", Ind::AiTrend);
    }
    let last = |v: &[f64], c: Rgb| v.iter().rev().copied().find(|x| x.is_finite()).map(|x| (x, c));
    Some(BandAxis {
        top,
        h,
        pad: (20.0, 6.0),
        lo: vlo,
        hi: vhi,
        last: [last(&a.ai, ne), last(&a.signal, col("sig_col")).filter(|_| p.on("sig_show"))]
            .into_iter()
            .flatten()
            .collect(),
    })
}

/// Argos Cipher Pro comme v1 : zones WT1 / WT2 / VWAP / MFI autour de 0, RSI coloré par zone, Stoch K / D, lignes de
/// divergence, points (taille selon la force du WT), Gold / Purple, barre de tendance. Tout suit les inputs.
pub(super) fn draw_cipher(pm: &mut Pixmap, text: &Text, g: &Geo, top: f64, c: Option<&cipher::Cipher>, vals: &[f64]) -> Option<BandAxis> {
    let c = c?;
    let p = argos_script::inputs::Inputs { params: &cipher::PARAMS, vals };
    if !p.on("osc_show") {
        return None;
    }
    let col = |k: &str| crate::palette::on_chart(p.rgb(k));
    let (wt, fast, mfi, rsi, st) = (p.on("wt_show"), p.on("fast_show"), p.on("mfi_show"), p.on("rsi_show"), p.on("st_show"));
    let (bz, sz, dd, gold, purple) = (p.on("buy_show"), p.on("sell_show"), p.on("div_dots"), p.on("gold_show"), p.on("purple_show"));
    let h = Ind::Cipher.band_h();
    let vis = (g.first..=g.last).filter(|&i| i < c.wt2.len());
    // échelle automatique (comme v1) : séries affichées, points à ±107 et barre de tendance à −97
    let mut series: Vec<&[f64]> = vec![];
    for (on, v) in [
        (wt, &c.wt1),
        (wt, &c.wt2),
        (fast, &c.fast),
        (mfi, &c.mfi),
        (rsi, &c.rsi),
        (st, &c.stoch_k),
        (st, &c.stoch_d),
    ] {
        if on {
            series.push(v);
        }
    }
    let (mut lo, mut hi) = plage(vis.clone().flat_map(|i| series.iter().filter_map(move |v| v.get(i).copied())));
    if bz || dd || gold || p.on("trend_bar") {
        lo = lo.min(-107.0);
    }
    if sz || dd || purple {
        hi = hi.max(107.0);
    }
    if !(lo < hi) {
        (lo, hi) = (-110.0, 110.0);
    }
    let (lo, hi) = view(Ind::Cipher, lo, hi);
    let y = |v: f64| top + 22.0 + (hi - v) / (hi - lo) * (h - 30.0);
    let pts = |v: &[f64]| vis.clone().map(|i| (g.x(i), y(v[i]))).collect::<Vec<_>>();
    // niveaux : OB1, OS1, 0, OB2, OS2
    for k in ["ob1", "os1", "ob2", "os2"] {
        let yy = y(p.get(k));
        tirets(pm, &[(0.0, yy), (g.right(), yy)], 3.0, 3.0, GREY, 1.0, 1.0);
    }
    crate::fill(pm, 0.0, y(0.0), g.right(), 1.0, GREY, 0.45);
    // zones autour de 0 (baseline de v1) : remplissage par bougie + ligne
    let area = |pm: &mut Pixmap, v: &[f64], up: Rgb, down: Rgb| {
        // un polygone par morceau du même signe (coupé là où la courbe croise 0)
        let y0 = y(0.0) as f32;
        let mut run: Vec<(f32, f32)> = vec![];
        let mut sign = true;
        let flush = |pm: &mut Pixmap, run: &mut Vec<(f32, f32)>, pos: bool| {
            if run.len() > 1 {
                let (a, b) = (run[0].0, run[run.len() - 1].0);
                run.push((b, y0));
                run.push((a, y0));
                crate::shapes::poly_fill(pm, 0.0, 0.0, run, if pos { up } else { down }, 0.38);
            }
            run.clear();
        };
        for i in vis.clone() {
            if !v[i].is_finite() {
                flush(pm, &mut run, sign);
                continue;
            }
            let pos = v[i] >= 0.0;
            if !run.is_empty() && pos != sign {
                // point de passage à 0 entre i − 1 et i
                let (xa, va, xb) = (g.x(i - 1), v[i - 1], g.x(i));
                let xz = (xa + (xb - xa) * va / (va - v[i])) as f32;
                run.push((xz, y0));
                flush(pm, &mut run, sign);
                run.push((xz, y0));
            }
            sign = pos;
            run.push((g.x(i) as f32, y(v[i]) as f32));
        }
        flush(pm, &mut run, sign);
        let idx: Vec<usize> = vis.clone().collect();
        let pts: Vec<(f64, f64)> = idx.iter().map(|&i| (g.x(i), y(v[i]))).collect();
        seg_couleurs(pm, &pts, &|k| Some(if v[idx[k]] >= 0.0 { up } else { down }), 1.0);
    };
    if wt {
        area(pm, &c.wt1, col("pc_wt1"), col("pc_wt1"));
        area(pm, &c.wt2, col("pc_wt2"), col("pc_wt2"));
    }
    if fast {
        area(pm, &c.fast, col("pc_fast"), col("pc_fast"));
    }
    if mfi {
        area(pm, &c.mfi, col("pc_mfi"), col("pc2_mfi"));
    }
    // RSI coloré par zone (rouge en surachat, vert en survente, mauve entre les deux)
    if rsi {
        let (ob, os) = (p.get("rsi_ob"), p.get("rsi_os"));
        let (c_ob, c_os, c_mid) = (col("col_rsi_ob"), col("col_rsi_os"), col("col_rsi_mid"));
        let idx: Vec<usize> = vis.clone().collect();
        let pts: Vec<(f64, f64)> = idx.iter().map(|&i| (g.x(i), y(c.rsi[i]))).collect();
        let couleur = |k: usize| {
            let v = c.rsi[idx[k]];
            Some(if v >= ob {
                c_ob
            } else if v <= os {
                c_os
            } else {
                c_mid
            })
        };
        seg_couleurs(pm, &pts, &couleur, 2.0);
    }
    if st {
        poly(pm, &pts(&c.stoch_k), col("pc_k"), 2.0);
        poly(pm, &pts(&c.stoch_d), col("pc_d"), 1.0);
    }
    // lignes de divergence (du pivot d'avant au pivot)
    let divs = |pm: &mut Pixmap, list: &[cipher::Div], v: &[f64], c: Rgb, w: f32, style: usize| {
        for d in list.iter().filter(|d| d.pivot >= g.first && d.prev <= g.last && d.pivot < v.len()) {
            div_seg(pm, (g.x(d.prev), y(v[d.prev])), (g.x(d.pivot), y(v[d.pivot])), c, w, style);
        }
    };
    let hid = p.pick("hid_style");
    divs(pm, &c.wt_divs.bear, &c.wt2, col("pc_dbear"), 2.0, 2);
    divs(pm, &c.wt_divs.bull, &c.wt2, col("pc_dbull"), 2.0, 2);
    // cachées dans la couleur de leur oscillateur (comme le Pine), au style choisi ; 2e WT : régulières et cachées
    divs(pm, &c.wt2_divs.bear, &c.wt2, col("pc_dbear"), 1.0, 2);
    divs(pm, &c.wt2_divs.bull, &c.wt2, col("pc_dbull"), 1.0, 2);
    divs(pm, &c.wt2_divs.hidden_bear, &c.wt2, col("pc_dbear"), 1.0, hid);
    divs(pm, &c.wt2_divs.hidden_bull, &c.wt2, col("pc_dbull"), 1.0, hid);
    divs(pm, &c.wt_divs.hidden_bear, &c.wt2, col("pc_dbear"), 2.0, hid);
    divs(pm, &c.wt_divs.hidden_bull, &c.wt2, col("pc_dbull"), 2.0, hid);
    if mfi {
        divs(pm, &c.mfi_divs.bear, &c.mfi, col("pc_mbear"), 1.0, 2);
        divs(pm, &c.mfi_divs.bull, &c.mfi, col("pc_mbull"), 1.0, 2);
        divs(pm, &c.mfi_divs.hidden_bear, &c.mfi, col("pc_mbear"), 1.0, hid);
        divs(pm, &c.mfi_divs.hidden_bull, &c.mfi, col("pc_mbull"), 1.0, hid);
    }
    if rsi {
        divs(pm, &c.rsi_divs.bear, &c.rsi, hex(0xe60000), 1.0, 2);
        divs(pm, &c.rsi_divs.bull, &c.rsi, hex(0x38ff42), 1.0, 2);
        divs(pm, &c.rsi_divs.hidden_bear, &c.rsi, hex(0xe60000), 1.0, hid);
        divs(pm, &c.rsi_divs.hidden_bull, &c.rsi, hex(0x38ff42), 1.0, hid);
    }
    if st {
        divs(pm, &c.stoch_divs.bear, &c.stoch_k, hex(0xe60000), 1.0, 2);
        divs(pm, &c.stoch_divs.bull, &c.stoch_k, hex(0x38ff42), 1.0, 2);
        divs(pm, &c.stoch_divs.hidden_bear, &c.stoch_k, hex(0xe60000), 1.0, hid);
        divs(pm, &c.stoch_divs.hidden_bull, &c.stoch_k, hex(0x38ff42), 1.0, hid);
    }
    // barre entre −95 et −99 (comme le Pine : couleur MFI > 0 / < 0 à 75 % de transparence) ; autres sources : vert / rouge
    if p.on("trend_bar") && mfi {
        let src = p.pick("trend_src");
        let (y0, y1) = (y(-95.0), y(-99.0));
        for i in vis.clone().filter(|&i| c.mfi[i].is_finite()) {
            let bull = match src {
                1 => c.wt2[i] > 0.0,
                2 => c.rsi[i] >= 50.0,
                _ => c.mfi[i] > 0.0,
            };
            let cc = match (src, bull) {
                (0, true) => col("pc_mfi"),
                (0, false) => col("pc2_mfi"),
                (_, true) => col("col_tbull"),
                _ => col("col_tbear"),
            };
            let x = g.x(i);
            let left = if i > 0 { x - g.x(i - 1) } else { 2.0 };
            crate::fill(pm, x - left, y0.min(y1), left.max(1.0), (y1 - y0).abs().max(2.0), cc, 0.25);
        }
    }
    // points : taille selon la force du WT (plus il est extrême, plus le point est gros)
    let strength = |i: usize, from: f64, span: f64| ((c.wt2.get(i).map_or(60.0, |v| v.abs()) - from) / span).clamp(0.0, 1.0);
    let in_view = |i: &&usize| (g.first..=g.last).contains(*i) && **i < c.wt2.len();
    if bz {
        for &i in c.buy.iter().filter(in_view) {
            dot(pm, g.x(i), y(-107.0), 1.5, col("pc_buy"));
        }
    }
    if sz {
        for &i in c.sell.iter().filter(in_view) {
            dot(pm, g.x(i), y(107.0), 1.5, col("pc_sell"));
        }
    }
    if wt {
        for (list, k) in [(&c.cross_up, "pc_buy"), (&c.cross_down, "pc_sell")] {
            for &i in list.iter().filter(in_view) {
                dot(pm, g.x(i), y(c.wt2[i]), 2.5 + 3.5 * strength(i, 40.0, 70.0), col(k));
            }
        }
    }
    if dd {
        for (list, v, cc) in [(&c.div_bull_dots, -106.0, hex(0x3fff00)), (&c.div_bear_dots, 106.0, hex(0xe60000))] {
            for &i in list.iter().filter(in_view) {
                dot(pm, g.x(i), y(v), 2.2 + 3.0 * strength(i, 40.0, 70.0), cc);
            }
        }
    }
    for (on, list, v, cc) in [(gold, &c.gold_dots, -106.0, col("pc_gold")), (purple, &c.purple, 106.0, hex(0xc000ff))] {
        if on {
            for &i in list.iter().filter(in_view) {
                let r = 6.5 + 3.0 * strength(i, 60.0, 60.0);
                crate::shapes::round_fill(
                    pm,
                    crate::shapes::Rect::new(g.x(i) - r - 3.0, y(v) - r - 3.0, 2.0 * r + 6.0, 2.0 * r + 6.0),
                    r + 3.0,
                    cc,
                    0.25,
                );
                dot(pm, g.x(i), y(v), r, cc);
            }
        }
    }
    // tableaux du Pine, dans la bande : Confluence des divergences (WT / RSI / Stoch / MFI) et Golden Dot
    let n = c.wt2.len();
    let band = crate::shapes::Rect::new(0.0, top, g.right(), h);
    if p.on("conf_tbl") && n > 0 {
        let mark = |b: bool, e: &str| if b { e.to_string() } else { "-".to_string() };
        let mut rows = vec![];
        let (mut nb, mut ns) = (0, 0);
        // comme le Pine : divergences régulières de la dernière bougie, boutons « Show » ou non
        for (name, (b, s)) in ["WaveTrend", "RSI", "Stoch RSI", "MFI"].into_iter().zip(c.table_last) {
            (nb, ns) = (nb + b as usize, ns + s as usize);
            rows.push((name, format!("{} / {}", mark(b, "🟢"), mark(s, "🔴"))));
        }
        rows.push(("TOTAL", format!("{nb}/4 🟢  {ns}/4 🔴")));
        table(pm, text, band, "DIVERGENCE", &rows, &p, "", Ind::Cipher);
    }
    if p.on("gold_tbl") && n > 0 {
        let since = c.gold.last().map(|&i| n - 1 - i.min(n - 1));
        let rows = vec![
            ("GOLDEN DOT", if since == Some(0) { "🟡 ACTIF".to_string() } else { "-".to_string() }),
            ("Dernier signal", since.map_or("aucun".to_string(), |b| format!("{b} barres"))),
        ];
        table(pm, text, band, "🟡 Golden Dot", &rows, &p, "g", Ind::Cipher);
    }
    let last = c.wt2.iter().rev().copied().find(|v| v.is_finite()).filter(|_| wt);
    Some(BandAxis {
        top,
        h,
        pad: (22.0, 8.0),
        lo,
        hi,
        last: last.map(|v| (v, hex(0x5b9cf6))).into_iter().collect(),
    })
}

/// Étiquettes de confluence du Cipher sur le prix, comme le Pine : « n/4 » sous la bougie (bull, vert) ou au-dessus
/// (bear, rouge), dorée à 4/4.
pub(super) fn draw_confluence(pm: &mut Pixmap, text: &Text, g: &Geo, c: &cipher::Cipher, vals: &[f64]) {
    let p = argos_script::inputs::Inputs { params: &cipher::PARAMS, vals };
    if !p.on("conf_lbl") {
        return;
    }
    let col = |k: &str| crate::palette::on_chart(p.rgb(k));
    let (black, white) = (Rgb(0, 0, 0), Rgb(0xff, 0xff, 0xff));
    for (list, below) in [(&c.confluence_buy, true), (&c.confluence_sell, false)] {
        for &(i, n) in list.iter().filter(|(i, _)| (g.first..=g.last).contains(i) && *i < g.bars.len()) {
            let b = &g.bars[i];
            let (bg, fg) = match (n >= 4, below) {
                (true, _) => (col("col_cgold"), black),
                (false, true) => (col("col_cbull"), black),
                (false, false) => (col("col_cbear"), white),
            };
            let s = format!("{n}/4");
            let (w, h) = (text.width(&s, 10.0) as f64 + 10.0, 16.0);
            let y = if below { g.y(b.low) + 8.0 } else { g.y(b.high) - 8.0 - h };
            crate::shapes::round_fill(pm, crate::shapes::Rect::new(g.x(i) - w / 2.0, y, w, h), 3.0, bg, 1.0);
            text.draw(pm, &s, (g.x(i) as f32, (y + 12.0) as f32), 10.0, fg, Align::Center);
        }
    }
}
/// Zone entre une courbe (`(x, valeur)`) et 0 en dégradé, comme le `fill` du Pine TBO : opacité `alpha` sur la courbe,
/// transparente sur la ligne 0 (`up` au-dessus, `down` en dessous). Une tranche par paire de points, coupée où la courbe
/// croise 0 ; les trous (NaN) interrompent la zone.
pub(super) fn zero_area(pm: &mut Pixmap, pts: &[(f64, f64)], y: &dyn Fn(f64) -> f64, up: Rgb, down: Rgb, alpha: f32) {
    let y0 = y(0.0) as f32;
    let slice = |pm: &mut Pixmap, (xa, va): (f64, f64), (xb, vb): (f64, f64)| {
        let pos = va + vb >= 0.0;
        let peak = if pos { va.max(vb) } else { va.min(vb) };
        let top = y(peak) as f32;
        if (top - y0).abs() < 0.5 {
            return;
        }
        let mut pb = tiny_skia::PathBuilder::new();
        pb.move_to(xa as f32, y(va) as f32);
        pb.line_to(xb as f32, y(vb) as f32);
        pb.line_to(xb as f32, y0);
        pb.line_to(xa as f32, y0);
        pb.close();
        if let Some(path) = pb.finish() {
            let c = if pos { up } else { down };
            let mut paint = crate::gradient(top, y0, &[(0.0, c, alpha), (1.0, c, 0.0)]);
            // tranches collées sans couture
            paint.anti_alias = false;
            pm.fill_path(&path, &paint, tiny_skia::FillRule::Winding, crate::px::t(), None);
        }
    };
    for w in pts.windows(2) {
        let ((xa, va), (xb, vb)) = (w[0], w[1]);
        if !va.is_finite() || !vb.is_finite() {
            continue;
        }
        if (va >= 0.0) != (vb >= 0.0) {
            // passage à 0 entre les deux points : deux tranches, une de chaque côté
            let xc = xa + (xb - xa) * (va / (va - vb));
            slice(pm, (xa, va), (xc, 0.0));
            slice(pm, (xc, 0.0), (xb, vb));
        } else {
            slice(pm, (xa, va), (xb, vb));
        }
    }
}

/// Fond, bordure et nom d'une bande d'oscillateur.
pub(super) fn band(pm: &mut Pixmap, g: &Geo, top: f64, ind: Ind) {
    // fond de la chart (pas de voile bleuté) : la bande se fond dans le thème
    crate::fill(pm, 0.0, top, g.right(), ind.band_h(), crate::Theme::current().background, 1.0);
    // (trait du haut allégé, comme la maquette : il marque la limite qu'on tire pour redimensionner, sans faire cadre)
    crate::fill(pm, 0.0, top, g.right(), 1.0, pal::edge_soft(), 0.45);
}

/// Bande d'un indicateur simple : niveaux, histogramme depuis 0, lignes, signaux.
pub(super) fn draw_sub(pm: &mut Pixmap, g: &Geo, top: f64, pl: &Plot) -> BandAxis {
    let vis = || (g.first..=g.last).flat_map(|i| pl.lines.iter().filter_map(move |l| l.0.get(i).copied()).chain(pl.hist.get(i).map(|h| h.0)));
    let h = pl.ind.band_h();
    let (lo, hi) = pl.range.unwrap_or_else(|| {
        let (a, b) = plage(vis());
        let (a, b) = if pl.hist.is_empty() { (a, b) } else { (a.min(0.0), b.max(0.0)) };
        if a < b {
            (a, b)
        } else {
            (a - 1.0, a + 1.0)
        }
    });
    let (lo, hi) = view(pl.ind, lo, hi);
    let y = |v: f64| top + 20.0 + (hi - v) / (hi - lo) * (h - 26.0);
    for &(a, b, c, alpha) in &pl.zones {
        let (ya, yb) = (y(a), y(b));
        crate::fill(pm, 0.0, ya.min(yb), g.right(), (ya - yb).abs(), c, alpha);
    }
    for &lv in pl.levels.iter().filter(|l| (lo..=hi).contains(*l)) {
        crate::fill(pm, 0.0, y(lv), g.right(), 1.0, GREY, 0.3);
    }
    if !pl.hist.is_empty() {
        let w = if g.last > g.first {
            ((g.x(g.last) - g.x(g.first)) / (g.last - g.first) as f64 * 0.7).max(1.0)
        } else {
            3.0
        };
        let y0 = y(0.0);
        for i in g.first..=g.last {
            if let Some(&(v, c, alpha)) = pl.hist.get(i).filter(|h| h.0.is_finite()) {
                let yv = y(v);
                crate::fill(pm, g.x(i) - w / 2.0, yv.min(y0), w, (yv - y0).abs().max(1.0), c, alpha);
            }
        }
    }
    for (k, (v, c, w)) in pl.lines.iter().enumerate() {
        poly_trait(
            pm,
            &(g.first..=g.last)
                .map(|i| (g.x(i), y(v.get(i).copied().unwrap_or(f64::NAN))))
                .collect::<Vec<_>>(),
            crate::palette::on_chart(*c),
            *w,
            pl.dash.get(k).copied().unwrap_or(0),
        );
    }
    let at = |i: usize| {
        pl.lines
            .first()
            .and_then(|l| l.0.get(i))
            .or(pl.hist.get(i).map(|h| &h.0))
            .copied()
            .unwrap_or(f64::NAN)
    };
    // très dézoomé (moins de 2 px par bougie) : pas de points de signal, ils couvriraient la ligne
    for &(i, _, c, _) in pl
        .marks
        .iter()
        .filter(|m| g.vp.bar_spacing >= 2.0 && (g.first..=g.last).contains(&m.0) && at(m.0).is_finite())
    {
        dot(pm, g.x(i), y(at(i)), 2.5, crate::palette::on_chart(c));
    }
    let last = pl
        .lines
        .iter()
        .filter_map(|(v, c, _)| v.iter().rev().copied().find(|x| x.is_finite()).map(|x| (x, crate::palette::on_chart(*c))))
        .take(2)
        .collect();
    BandAxis {
        top,
        h,
        pad: (20.0, 6.0),
        lo,
        hi,
        last,
    }
}

#[cfg(test)]
mod tests_plage {
    use super::plage;

    /// 📏 An absurd value (−25 000 000 among values between −100 and 100) is left out of the scale; a real peak
    /// (10 × the spread, like the 20 000 of a volume panel) stays; NaN and ±∞ never count; a flat series keeps it.
    #[test]
    fn echelle_sans_valeurs_absurdes() {
        let normal: Vec<f64> = (0..200).map(|i| (i as f64 / 7.0).sin() * 100.0).collect();
        let mut v = normal.clone();
        v.extend([-25_000_000.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY]);
        let (lo, hi) = plage(v.into_iter());
        assert!(lo >= -100.0 - 1e-9 && hi <= 100.0 + 1e-9, "({lo}, {hi})");
        let mut pic = normal.clone();
        pic.push(2_000.0);
        assert_eq!(plage(pic.into_iter()).1, 2_000.0, "a real peak stays");
        let plat = vec![5.0; 50];
        assert_eq!(plage(plat.into_iter()), (5.0, 5.0));
        assert_eq!(plage([1.0, f64::NAN, 3.0].into_iter()), (1.0, 3.0), "few values: all of them");
        assert_eq!(plage(std::iter::empty()), (f64::INFINITY, f64::NEG_INFINITY));
    }
}
