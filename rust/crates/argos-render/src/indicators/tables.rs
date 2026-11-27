//! 📋 Tableaux et étiquettes : tableau multi-UT, tableaux des indicateurs, étiquettes, couleurs des inputs.

use super::*;

/// Profil de volume comme v1 : rangées haussières / baissières au début de la plage (ou collées à droite), largeur max en
/// % (33 par défaut), Value Area plus opaque, POC avec étiquette, VAH / VAL en tirets ; tout suit les inputs.
/// Couleur d'un input (« #rrggbb » de v1).
pub(super) fn col(p: &argos_script::inputs::Inputs, k: &str) -> Rgb {
    crate::palette::on_chart(p.rgb(k))
}

/// Volume court comme `kf` de v1 (« 950 », « 1.2K », « 3.4M ») : partagé avec Argos AI.
pub(super) use argos_script::indicators::classic::kf;

/// 📊 Tableau multi-UT en bas à droite de la zone des bougies : UT, tendance ▲▼, RSI, variation, clôture, et le
/// total des UT alignées.
pub(super) fn draw_mtf_table(pm: &mut Pixmap, text: &Text, g: &Geo, rows: &[argos_script::indicators::mtf_table::Row]) {
    if rows.is_empty() {
        return;
    }
    let on_chart = crate::palette::on_chart;
    let (bull, bear) = (on_chart(hex(0x26a69a)), on_chart(hex(0xf23645)));
    // 📋 (tableaux-1) moved / resized by the user like every table: sizes at scale `s`, the stretches on top
    let id = crate::tableaux::Id::native(Ind::MtfTable, "", None);
    let lay = crate::tableaux::layout(&id.key);
    let (rh1, cols1) = (18.0, [40.0, 36.0, 44.0, 62.0, 70.0]);
    let mesure = |s: f64| {
        let w: f64 = (cols1.iter().sum::<f64>() + 8.0) * s * lay.wx;
        (w, rh1 * s * lay.hy * (rows.len() as f64 + 2.0) + 6.0 * s)
    };
    // (bottom right; 🧱 bottom left while Argos Murs' pills hold the right side)
    let defaut = |w: f64, h: f64| {
        let x0 = if super::table_corner_with_walls(5) == 5 {
            g.right() - w - 8.0
        } else {
            g.price_area().x + 8.0
        };
        crate::shapes::Rect::new(x0, (g.price_h - h - 8.0).max(34.0), w, h)
    };
    // 📐 placed like every other table: above the reserved places (the navigation pill, Argos Murs' bottom pills),
    // never over a table already there, and listed in `tableaux` (the wall pills and the high / low pills avoid it)
    let Some((r, s)) = super::placer(&id, g.price_area(), &mesure, &defaut) else {
        return;
    };
    // 🎨 (tour 2, F4) on the tables' layer: over the lines and the walls
    crate::tableaux::dessiner(pm, r, |pm| {
        let (rh, cols) = (rh1 * s * lay.hy, cols1.map(|c| c * s * lay.wx));
        let (x0, y0) = (r.x, r.y);
        crate::shapes::round_fill(pm, r, 6.0, pal::popup(), 0.92);
        let fs = 11.0 * s as f32;
        let cell = |pm: &mut Pixmap, col: usize, row: usize, s2: &str, c: Rgb| {
            let x = x0 + 4.0 * s + cols[..col].iter().sum::<f64>();
            text.draw(
                pm,
                s2,
                ((x + 4.0 * s) as f32, (y0 + rh * 0.5 + 6.0 * s + row as f64 * rh) as f32),
                fs,
                c,
                Align::Left,
            );
        };
        for (k, head) in ["UT", "TEND.", "RSI", "VAR.", "CLÔTURE"].iter().enumerate() {
            cell(pm, k, 0, head, crate::palette::WHITE);
        }
        for (r, row) in rows.iter().enumerate() {
            cell(pm, 0, r + 1, row.name, crate::palette::WHITE);
            let Some(c) = row.cell else {
                for k in 1..5 {
                    cell(pm, k, r + 1, "—", GREY);
                }
                continue;
            };
            let (t, tc) = match c.trend {
                1 => ("▲", bull),
                -1 => ("▼", bear),
                _ => ("◆", GREY),
            };
            cell(pm, 1, r + 1, t, tc);
            let rc = if c.rsi >= 70.0 {
                bear
            } else if c.rsi <= 30.0 {
                bull
            } else {
                crate::palette::WHITE
            };
            cell(pm, 2, r + 1, &format!("{:.0}", c.rsi), rc);
            cell(pm, 3, r + 1, &format!("{:+.2} %", c.chg), if c.chg >= 0.0 { bull } else { bear });
            let (d, hh, mm, ss) = (c.left / 86_400, c.left % 86_400 / 3600, c.left % 3600 / 60, c.left % 60);
            let left = if d > 0 {
                format!("{d}j {hh:02}:{mm:02}")
            } else {
                format!("{hh:02}:{mm:02}:{ss:02}")
            };
            cell(pm, 4, r + 1, &left, GREY);
        }
        let (up, down) = argos_script::indicators::mtf_table::score(rows);
        let n = rows.iter().filter(|r| r.cell.is_some()).count();
        cell(pm, 0, rows.len() + 1, "TOTAL", crate::palette::WHITE);
        let (s, c) = if up > down {
            (format!("▲ {up}/{n}"), bull)
        } else if down > up {
            (format!("▼ {down}/{n}"), bear)
        } else {
            (format!("◆ {up}/{n}"), GREY)
        };
        cell(pm, 1, rows.len() + 1, &s, c);
    });
}

pub(super) fn tag(pm: &mut Pixmap, text: &Text, x: f64, y: f64, s: &str, fg: Rgb, bg: Rgb) {
    let w = text.width(s, 10.5) as f64 + 10.0;
    crate::shapes::round_fill(pm, crate::shapes::Rect::new(x - w / 2.0, y - 8.0, w, 16.0), 3.0, bg, 1.0);
    text.draw(pm, s, (x as f32, (y + 4.0) as f32), 10.5, fg, Align::Center);
}

/// Shown instead of a table that does not fit its place even at the smallest size.
pub(crate) const TROP_GRAND: &str = "Tableau trop grand pour cette place";

/// Top left of a `w` × `h` table at position `pos` (tr, tl, tc, mr, ml, br, bl, bc) in `area` (dans la zone de prix :
/// sous la légende en haut, au-dessus du zoom en bas).
fn coin(area: crate::shapes::Rect, pos: usize, w: f64, h: f64) -> (f64, f64) {
    let (mt, mb) = if area.y == 0.0 { (34.0, 40.0) } else { (24.0, 6.0) };
    let (l, c, r) = (area.x + 10.0, area.x + (area.w - w) / 2.0, area.x + area.w - w - 10.0);
    let (t, m, b) = (area.y + mt, area.y + (area.h - h) / 2.0, area.y + area.h - h - mb);
    [(r, t), (l, t), (c, t), (r, m), (l, m), (r, b), (l, b), (c, b)][pos.min(7)]
}

/// 📐 One line « Tableau trop grand pour cette place » in the table's corner (font `f`), instead of skipping the table
/// silently; kept inside `area` and, like a table, never over another table.
fn trop_grand(pm: &mut Pixmap, text: &Text, area: crate::shapes::Rect, pos: usize, f: f32) {
    let (w, h) = (text.width(TROP_GRAND, f) as f64 + 16.0, f as f64 * 1.8 + 8.0);
    let (x, y) = coin(area, pos, w, h);
    let y = y.min(area.y + area.h - h).max(area.y);
    let r = super::hors_reserve(crate::shapes::Rect::new(x, y, w, h), area.y).unwrap_or(crate::shapes::Rect::new(x, y, w, h));
    let overlaps = |a: &crate::shapes::Rect| a.x < r.x + r.w && r.x < a.x + a.w && a.y < r.y + r.h && r.y < a.y + a.h;
    if TABLES.with(|t| t.borrow().iter().any(overlaps)) {
        return;
    }
    TABLES.with(|t| t.borrow_mut().push(r));
    crate::tableaux::dessiner(pm, r, |pm| {
        let alpha = if crate::glass::on() { 0.82 } else { 0.97 };
        crate::shapes::round_fill(pm, r, 6.0, pal::panel(), alpha);
        crate::shapes::round_stroke(pm, r, 6.0, pal::edge_soft(), 1.0);
        let base = (r.y + h / 2.0 + f as f64 * 0.35) as f32;
        text.draw(pm, TROP_GRAND, ((r.x + 8.0) as f32, base), f, crate::palette::ICON, Align::Left);
    });
}

/// Tableau d'un indicateur comme v1 : en-tête « nom | Value », placé et dimensionné par les inputs « Table : position »
/// et « Table : taille » (`tblPos`, `tblSize` de l'indicateur ; absents : haut droite, petit).
/// Tableau d'un indicateur ; sa position et sa taille viennent des inputs `{prefix}tblPos` / `{prefix}tblSize` (le Cipher
/// en a deux : « g » pour le Golden Dot). 📋 (tableaux-1) Moved / resized by the user: [`super::placer`] with the
/// layout of `ind` (and `prefix`).
#[allow(clippy::too_many_arguments)]
pub(super) fn table(
    pm: &mut Pixmap,
    text: &Text,
    area: crate::shapes::Rect,
    title: &str,
    rows: &[(&str, String)],
    inp: &argos_script::inputs::Inputs,
    prefix: &str,
    ind: Ind,
) {
    let has = |k: &str| inp.params.iter().any(|p| p.key == k);
    let (kp, ks) = (format!("{prefix}tblPos"), format!("{prefix}tblSize"));
    let pos = if has(&kp) { inp.pick(&kp) } else { 0 };
    // (🧱 in the price pane, a right corner goes left while Argos Murs' pills hold the right side)
    let pos = if area.y == 0.0 { super::table_corner_with_walls(pos) } else { pos };
    // tailles de v1 : tiny 7, small 10, normal 13, large 20, huge 32 (en px de police) ; ici un peu plus grand pour l'écran
    const SIZES: [f32; 5] = [9.0, 11.0, 13.0, 18.0, 26.0];
    let chosen = if has(&ks) { inp.pick(&ks).min(4) } else { 1 };
    let id = crate::tableaux::Id::native(ind, prefix, crate::tableaux::interrupteur(inp.params, prefix));
    let lay = crate::tableaux::layout(&id.key);
    // 📐 bande rétrécie (écran bas) : la taille choisie, sinon une plus petite qui tient ; si même la plus petite ne
    // tient pas, une seule ligne le dit à la place du tableau (le tableau ne déborde jamais sur la bande voisine)
    let fits = |f: f32| (f as f64 * lay.scale * 1.8 * lay.hy) * (rows.len() + 1) as f64 + 8.0 + 30.0 <= area.h;
    let Some(f) = SIZES[..=chosen].iter().rev().copied().find(|&f| fits(f)) else {
        trop_grand(pm, text, area, pos, SIZES[0]);
        return;
    };
    // (the size at scale `s`: the font, the row height and the width grow together; the stretches on top)
    let mesure = |s: f64| {
        let fs = f * s as f32;
        let wide = |t: &str| text.width(t, fs) as f64;
        let w = rows.iter().map(|(k, v)| wide(k) + wide(v)).fold(wide(title) + wide("Value"), f64::max) + 40.0 * s;
        (w * lay.wx, fs as f64 * 1.8 * lay.hy * (rows.len() + 1) as f64 + 8.0 * s)
    };
    let defaut = |w: f64, h: f64| {
        let (x, y) = coin(area, pos, w, h);
        crate::shapes::Rect::new(x, y, w, h)
    };
    // 🔍 never over the navigation pill: only a table that covers it moves up, just enough; 📐 jamais un tableau
    // par-dessus un autre (bande rétrécie : on garde le premier posé)
    let Some((r, s)) = super::placer(&id, area, &mesure, &defaut) else {
        return;
    };
    // 🎨 (tour 2, F4) on the tables' layer: over the lines and the walls
    crate::tableaux::dessiner(pm, r, |pm| {
        let f = f * s as f32;
        let (w, rh) = (r.w, f as f64 * 1.8 * lay.hy);
        // 🫧 (tour 2, F11) the same glass as the multi-UT table (mockup): the candles no longer show through
        crate::shapes::round_fill(pm, r, 6.0, pal::popup(), 0.92);
        crate::shapes::round_stroke(pm, r, 6.0, pal::edge_soft(), 1.0);
        let pad = 4.0 * s;
        let base = |k: usize| (r.y + pad + rh * (k as f64 + 0.5) + f as f64 * 0.35) as f32;
        let m = 8.0 * s;
        text.draw(pm, title, ((r.x + m) as f32, base(0)), f, crate::palette::WHITE, Align::Left);
        text.draw(pm, "Value", ((r.x + w - m) as f32, base(0)), f, crate::palette::WHITE, Align::Right);
        crate::fill(pm, r.x, r.y + pad + rh, w, 1.0, pal::edge_soft(), 1.0);
        for (k, (name, val)) in rows.iter().enumerate() {
            // libellés en gris clair (le gris foncé se lisait mal, surtout sur une chart claire derrière)
            text.draw(pm, name, ((r.x + m) as f32, base(k + 1)), f, crate::palette::ICON, Align::Left);
            text.draw(pm, val, ((r.x + w - m) as f32, base(k + 1)), f, crate::palette::WHITE, Align::Right);
        }
    });
}
