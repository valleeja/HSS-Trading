//! 📜 Ce qu'un script Argos (Pine v6) produit, dessiné sur la chart comme dans Vela / TradingView : `plot` (ligne,
//! marches, aire, histogramme, colonnes, cercles, croix), `fill`, `hline`, `plotshape` / `plotchar`, `bgcolor`, et les
//! dessins `label` / `line` / `box`. Un script `overlay = true` va sur le prix ; sinon il a sa bande (comme un
//! oscillateur), au-dessus des bandes des indicateurs d'Argos ; son nom et ses boutons sont dans la légende (`indleg`).

use crate::indicators::Geo;
use crate::palette as pal;
use crate::shapes::{lines_alpha, poly_fill, round_fill, Rect};
use crate::text::{Align, Text};
use crate::Rgb;
use argos_script::lang::value::{rgba, HandleKind};
use argos_script::lang::{Drawing, Loc, Output, PlotStyle, Shape, Tracer};
use tiny_skia::Pixmap;

/// Un script sur la chart.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ScriptView {
    /// Nom donné dans l'éditeur.
    pub name: String,
    pub out: Output,
    /// 👁 masqué : calculé mais pas dessiné (sa bande reste, vide).
    pub hidden: bool,
    /// Dernière erreur (le script garde son dernier dessin réussi).
    pub error: Option<String>,
    /// 🎨 Style chosen in the ⚙ of a built-in written in Argos Script (colour, width, line type of its plots / fills /
    /// hlines); empty: the script's own (always for a user script).
    pub style: argos_script::style_integres::Style,
    /// The user script's stable id (`SavedScript::id`): its pane height is kept under it; empty for the others.
    pub id: String,
}

impl ScriptView {
    /// Titre de la légende : `shorttitle`, sinon `title`, sinon le nom.
    pub fn title(&self) -> &str {
        [&self.out.short, &self.out.title, &self.name]
            .into_iter()
            .find(|s| !s.is_empty())
            .map_or("Script", |s| s.as_str())
    }

    /// 📐 Height of its lower pane: the one chosen by dragging its top edge (`set_script_heights`, by id), else
    /// `OSC_H`, or 2 × `OSC_H` for a script with drawings in its own pane or 6 plots or more (like the Cipher); a low
    /// screen squeezes it with the native bands. Every reader of a script pane's height goes through here.
    pub fn band_h(&self) -> f64 {
        use crate::indicators::OSC_H;
        let o = &self.out;
        let riche = o.plots.len() >= 6
            || !o.fills.is_empty()
            || o.drawings.iter().any(|d| !d.deleted && !d.force_overlay)
            || o.polylines.iter().any(|p| !p.deleted && !p.force_overlay)
            || o.linefills.iter().any(|l| !l.deleted);
        let h = crate::indicators::hauteur_script(&self.id).unwrap_or(if riche { 2.0 * OSC_H } else { OSC_H });
        h * crate::indicators::squeeze()
    }

    /// 📜 The key its pane's view is kept under: its id, or its name for a script without one.
    pub fn view_key(&self) -> String {
        if self.id.is_empty() {
            self.name.clone()
        } else {
            self.id.clone()
        }
    }

    /// ⚠ What the legend writes after the name when the script is in trouble: « ⚠ erreur ligne n » (its last run
    /// failed: the drawing before stays), « ⚠ ne trace rien » (no output call in it), `None` when all is well.
    pub fn alerte(&self) -> Option<String> {
        let e = self.error.as_deref()?;
        if e == NE_TRACE_RIEN {
            return Some("⚠ ne trace rien".into());
        }
        let ligne = e.strip_prefix("Ligne ").and_then(|r| r.split(' ').next()).filter(|n| n.parse::<u32>().is_ok());
        Some(match ligne {
            Some(n) => format!("⚠ erreur ligne {n}"),
            None => "⚠".into(),
        })
    }
}

/// `ScriptView::error` of a script without any output call (`Output::has_output_calls` false).
pub const NE_TRACE_RIEN: &str = "ne trace rien";

/// Couleur Pine → (couleur, opacité 0–1).
fn ink(c: u32) -> (Rgb, f32) {
    let (r, g, b, t) = rgba(c);
    (Rgb(r, g, b), 1.0 - t as f32 / 100.0)
}

/// Taille de texte (`size.*`) en pixels.
fn font(size: &str) -> f32 {
    match size {
        "tiny" => 9.0,
        "small" => 10.5,
        "large" => 15.0,
        "huge" => 19.0,
        _ => 12.0,
    }
}

/// Taille d'une forme (`plotshape`) en pixels.
fn shape_px(size: &str) -> f64 {
    match size {
        "tiny" => 6.0,
        "normal" => 12.0,
        "large" => 18.0,
        "huge" => 24.0,
        _ => 9.0,
    }
}

/// Tirets d'un style de trait Pine.
fn pattern(style: &str) -> Option<(f64, f64)> {
    match style {
        "dashed" => Some((6.0, 4.0)),
        "dotted" => Some((1.5, 3.5)),
        _ => None,
    }
}

/// Où dessiner : le prix, ou une bande.
struct Frame<'a> {
    g: &'a Geo<'a>,
    top: f64,
    bot: f64,
    /// Bande : (plus bas, plus haut) affichés ; `None` : échelle de prix.
    range: Option<(f64, f64)>,
    /// Demi-largeur d'une bougie.
    half: f64,
}

impl Frame<'_> {
    fn y(&self, v: f64) -> f64 {
        match self.range {
            Some((lo, hi)) => self.top + 20.0 + (hi - v) / (hi - lo) * (self.bot - self.top - 26.0),
            None => self.g.y(v),
        }
    }
    /// x d'une bougie (fractionnaire, aussi au-delà des bougies chargées).
    fn x(&self, i: f64) -> f64 {
        // la vraie place de chaque bougie (les bougies de volume n'ont pas toutes la même largeur)
        self.g.xf(i)
    }
    fn stroke(&self, pm: &mut Pixmap, pts: &[(f64, f64)], c: u32, width: f64, style: &str) {
        let (rgb, a) = ink(c);
        let f = |v: &[(f64, f64)]| v.iter().map(|q| (q.0 as f32, q.1 as f32)).collect::<Vec<_>>();
        let parts = match pattern(style) {
            Some((on, off)) => crate::toolgeom::dashes(pts, on, off),
            None => vec![pts.to_vec()],
        };
        for p in parts {
            lines_alpha(pm, 0.0, 0.0, &[&f(&p)], rgb, width.max(1.0) as f32, a);
        }
    }
    /// A filled rectangle, cut to the frame: nothing above the frame's top nor below its bottom. A rectangle wholly
    /// outside draws nothing; one that crosses an edge is cut there.
    fn rect(&self, pm: &mut Pixmap, x0: f64, y0: f64, x1: f64, y1: f64, c: u32) {
        let (rgb, a) = ink(c);
        let (haut, bas) = (y0.min(y1), y0.max(y1));
        if a > 0.0 && bas >= self.top && haut <= self.bot {
            let (t, b) = (haut.max(self.top), bas.min(self.bot));
            crate::fill(pm, x0.min(x1), t, (x1 - x0).abs(), b - t, rgb, a);
        }
    }
}

/// Last bar to draw for a plot shifted by `offset`: the last visible bar, or after the last bar for a plot shifted
/// ahead, as far as its values go and the screen shows (TradingView draws there; B-046).
fn fin_devant(g: &Geo, offset: i64) -> usize {
    let last = g.last.min(g.bars.len().saturating_sub(1));
    if last + 1 < g.bars.len() {
        return last;
    }
    last + usize::try_from(offset).unwrap_or(0).min(g.apres())
}

/// Valeur d'un tracé (plot décalé de son `offset`, ou niveau d'un hline) à la bougie affichée `i`.
fn series(out: &Output, t: Tracer, i: usize) -> f64 {
    match t {
        (HandleKind::Hline, k) => out.hlines.get(k).map_or(f64::NAN, |h| h.0),
        (_, k) => out
            .plots
            .get(k)
            .and_then(|p| (i as i64 - p.offset).try_into().ok().and_then(|j: usize| p.values.get(j).copied()))
            .unwrap_or(f64::NAN),
    }
}

/// Plus bas / plus haut des tracés visibles (bande d'un script).
fn range(out: &Output, g: &Geo) -> (f64, f64) {
    let plots = out
        .plots
        .iter()
        .enumerate()
        .filter(|(_, p)| p.visible)
        .flat_map(|(k, _)| (g.first..=g.last).map(move |i| series(out, (HandleKind::Plot, k), i)));
    // (like TradingView's autoscale: the shapes placed at a price, and the base of columns / histograms, count too)
    let formes = out
        .shapes
        .iter()
        .filter(|s| s.loc == Loc::Absolute && (g.first..=g.last).contains(&s.bar))
        .map(|s| s.price);
    let base = out
        .plots
        .iter()
        .filter(|p| p.visible && matches!(p.style, PlotStyle::Columns | PlotStyle::Histogram))
        .map(|_| 0.0);
    let bougies = out
        .candles
        .iter()
        .filter(|c| c.visible)
        .flat_map(|c| (g.first..=g.last).filter_map(|i| c.ohlc.get(i)).flat_map(|o| [o[1], o[2]]));
    let (lo, hi) = plots
        .chain(bougies)
        .chain(out.hlines.iter().map(|h| h.0))
        .chain(formes)
        .chain(base)
        .filter(|v| v.is_finite())
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), v| (a.min(v), b.max(v)));
    if lo < hi {
        (lo, hi)
    } else if lo.is_finite() {
        (lo - 1.0, lo + 1.0)
    } else {
        (0.0, 1.0)
    }
}

/// 🕯 `plotcandle` / `plotbar`: a wick from high to low and a body from open to close (at least 1 px), or an OHLC bar
/// (the line, the open's tick left, the close's tick right), at each bar where the four values are known.
fn draw_candles(pm: &mut Pixmap, f: &Frame, c: &argos_script::lang::Candles, vis: std::ops::RangeInclusive<usize>) {
    for i in vis {
        let (Some(o), Some(Some(body))) = (c.ohlc.get(i), c.color.get(i)) else {
            continue;
        };
        if !o.iter().all(|v| v.is_finite()) {
            continue;
        }
        let x = f.x(i as f64);
        let w = (f.half * 0.8).max(1.0);
        let [yo, yh, yl, yc] = o.map(|v| f.y(v));
        let wick = c.wick.get(i).copied().flatten().unwrap_or(*body);
        if c.bars {
            f.stroke(pm, &[(x, yh), (x, yl)], wick, 1.0, "");
            f.stroke(pm, &[(x - w, yo), (x, yo)], wick, 1.0, "");
            f.stroke(pm, &[(x, yc), (x + w, yc)], wick, 1.0, "");
            continue;
        }
        f.stroke(pm, &[(x, yh), (x, yl)], wick, 1.0, "");
        let (t, b) = (yo.min(yc), yo.max(yc).max(yo.min(yc) + 1.0));
        f.rect(pm, x - w, t, x + w, b, *body);
        if let Some(Some(edge)) = c.border.get(i).filter(|e| **e != Some(*body)) {
            f.stroke(pm, &[(x - w, t), (x + w, t), (x + w, b), (x - w, b), (x - w, t)], *edge, 1.0, "");
        }
    }
}

/// Dessine le script `v` : sur le prix (`band` = `None`) ou dans sa bande (haut de la bande). In a band it gives back
/// the band's scale (its numbers on the price column, as a native pane such as the RSI; the crosshair reads it).
pub fn draw(pm: &mut Pixmap, text: &Text, g: &Geo, v: &ScriptView, band: Option<(f64, f64)>) -> Option<crate::indicators::BandAxis> {
    draw_view(pm, text, g, v, band, (0.0, 1.0))
}

/// [`draw`] with the pane's view (offset, zoom: dragging in the pane, on its value column) applied to its scale; on
/// the price it changes nothing.
pub fn draw_view(pm: &mut Pixmap, text: &Text, g: &Geo, v: &ScriptView, band: Option<(f64, f64)>, view: (f64, f64)) -> Option<crate::indicators::BandAxis> {
    let half = if g.last > g.first {
        (g.x(g.last) - g.x(g.first)) / (g.last - g.first) as f64 / 2.0
    } else {
        2.0
    };
    let f = match band {
        Some((top, h)) => {
            crate::fill(pm, 0.0, top, g.right(), h, crate::Theme::current().background, 1.0);
            crate::fill(pm, 0.0, top, g.right(), 1.0, pal::edge_soft(), 1.0);
            Frame {
                g,
                top,
                bot: top + h,
                range: Some({
                    let (lo, hi) = range(&v.out, g);
                    // (no view: the scale exactly as before, not one rounding away)
                    if view == (0.0, 1.0) {
                        (lo, hi)
                    } else {
                        crate::indicators::apply_view(view, lo, hi)
                    }
                }),
                half,
            }
        }
        None => Frame {
            g,
            top: 0.0,
            bot: g.price_h,
            range: None,
            half,
        },
    };
    if v.hidden {
        return None;
    }
    // (the same frame as `Frame::y`: 20 px for the header, 6 at the bottom)
    let axe = match (band, f.range) {
        (Some((top, h)), Some((lo, hi))) => Some(crate::indicators::BandAxis {
            top,
            h,
            pad: (20.0, 6.0),
            lo,
            hi,
            last: vec![],
        }),
        _ => None,
    };
    let out = &v.out;
    let vis = || g.first..=g.last.min(g.bars.len().saturating_sub(1));
    // fond (bgcolor) : une colonne par bougie
    for i in vis() {
        if let Some(Some(c)) = out.bgcolor.get(i) {
            let (x0, x1) = g.slot(i);
            f.rect(pm, x0, f.top, x1, f.bot, *c);
        }
    }
    // remplissages entre deux tracés : un trapèze par bougie, de la couleur de la bougie
    for (k, (a, b, colors)) in out.fills.iter().enumerate() {
        let st = v.style.fill(k);
        // between two plots shifted ahead (`offset`), the fill is shifted with them: the colour of the bar it was
        // computed on, and after the last bar as far as they go, as on TradingView (B-046)
        let off = match (a, b) {
            ((HandleKind::Plot, k), _) | (_, (HandleKind::Plot, k)) => out.plots.get(*k).map_or(0, |p| p.offset),
            _ => 0,
        };
        for i in (g.first..=fin_devant(g, off)).filter(|&i| i > 0) {
            let couleur = usize::try_from(i as i64 - off).ok().and_then(|j| colors.get(j));
            let Some(Some(c)) = couleur else { continue };
            let c = st.couleur.unwrap_or(*c);
            let (a0, a1, b0, b1) = (series(out, *a, i - 1), series(out, *a, i), series(out, *b, i - 1), series(out, *b, i));
            if [a0, a1, b0, b1].iter().all(|v| v.is_finite()) {
                let (x0, x1) = (f.x(i as f64 - 1.0), f.x(i as f64));
                let (rgb, al) = ink(c);
                let pts = [(x0, f.y(a0)), (x1, f.y(a1)), (x1, f.y(b1)), (x0, f.y(b0))].map(|(x, y)| (x as f32, y as f32));
                poly_fill(pm, 0.0, 0.0, &pts, rgb, al);
            }
        }
    }
    for (k, (price, _, c, style)) in out.hlines.iter().enumerate() {
        let st = v.style.hline(k);
        let y = f.y(*price);
        if (f.top..=f.bot).contains(&y) {
            f.stroke(
                pm,
                &[(0.0, y), (g.right(), y)],
                st.couleur.unwrap_or(*c),
                st.epaisseur.unwrap_or(1.0),
                st.ligne.unwrap_or(style.as_str()),
            );
        }
    }
    for (k, p) in out.plots.iter().enumerate().filter(|(_, p)| p.visible) {
        draw_plot(pm, &f, out, k, p.style, v.style.plot(k));
    }
    for c in out.candles.iter().filter(|c| c.visible) {
        draw_candles(pm, &f, c, vis());
    }
    // (in a band, the `force_overlay` marks go on the price instead: `draw_forced`)
    let here = |forced: bool| band.is_none() || !forced;
    for s in out.shapes.iter().filter(|s| vis().contains(&s.bar) && here(s.force_overlay)) {
        draw_shape(pm, text, &f, s);
    }
    // (the linefills under the lines, like TradingView)
    for lf in &out.linefills {
        draw_linefill(pm, &f, out, lf);
    }
    for p in out.polylines.iter().filter(|p| !p.deleted && here(p.force_overlay)) {
        draw_polyline(pm, &f, p);
    }
    for d in out
        .drawings
        .iter()
        .filter(|d| !d.deleted && here(d.force_overlay) && !is_vague_gauge_label(&f, v, d))
    {
        draw_drawing(pm, text, &f, d);
    }
    lay_vague_pills(text, &f, v);
    // 📋 tables (step 4), as they are at the last bar, in their corner of the area
    for (k, t) in out.tables.iter().enumerate().filter(|(_, t)| !t.deleted) {
        let _ = draw_table(pm, text, f.top, f.bot, g.right(), t, &crate::tableaux::Id::script(&v.id, k, v.title()));
    }
    axe
}

/// 🧪 Blue of the buy fills and pink of the sell fills of a strategy (TradingView's).
pub const ACHAT: Rgb = Rgb(0x29, 0x62, 0xFF);
pub const VENTE: Rgb = Rgb(0xE9, 0x1E, 0x63);

/// 🧪 A strategy's fills on the price, like TradingView (also for a strategy drawn in its own band): a dash at the fill
/// price across the bar, an arrow under the bar for a buy (blue) or above it for a sell (pink), and next to the arrow
/// the order's name and its signed quantity. Several fills of one bar stack away from it.
pub fn draw_ordres(pm: &mut Pixmap, text: &Text, g: &Geo, v: &ScriptView) {
    let Some(b) = v.out.strategie.as_ref().filter(|_| !v.hidden) else {
        return;
    };
    let last = g.last.min(g.bars.len().saturating_sub(1));
    let fs = font("small");
    let lh = fs as f64 * 1.25;
    const R: f32 = 5.0;
    let mut pile: std::collections::HashMap<(usize, bool), usize> = Default::default();
    for m in b.marques.iter().filter(|m| (g.first..=last).contains(&m.bar)) {
        let bougie = &g.bars[m.bar];
        let (x0, x1) = g.slot(m.bar);
        let x = g.x(m.bar);
        let c = if m.achat { ACHAT } else { VENTE };
        let y = g.y(m.prix);
        crate::fill(pm, x0, y - 0.75, (x1 - x0).max(3.0), 1.5, c, 1.0);
        let n = pile.entry((m.bar, m.achat)).or_insert(0);
        let decale = *n as f64 * (2.0 * R as f64 + 2.0 * lh + 4.0);
        *n += 1;
        let qty = format!("{}{}", if m.achat { "+" } else { "−" }, quantite(m.qty));
        let r = R;
        if m.achat {
            // the arrow's tip just under the bar, the text under the arrow
            let ay = g.y(bougie.low) + 4.0 + R as f64 + decale;
            poly_fill(
                pm,
                x,
                ay,
                &[(0.0, -r), (r, 0.0), (r * 0.4, 0.0), (r * 0.4, r), (-r * 0.4, r), (-r * 0.4, 0.0), (-r, 0.0)],
                c,
                1.0,
            );
            for (k, t) in [m.texte.as_str(), &qty].iter().enumerate() {
                text.draw(pm, t, (x as f32, (ay + R as f64 + lh * (k as f64 + 1.0)) as f32), fs, c, Align::Center);
            }
        } else {
            let ay = g.y(bougie.high) - 4.0 - R as f64 - decale;
            poly_fill(
                pm,
                x,
                ay,
                &[(0.0, r), (r, 0.0), (r * 0.4, 0.0), (r * 0.4, -r), (-r * 0.4, -r), (-r * 0.4, 0.0), (-r, 0.0)],
                c,
                1.0,
            );
            for (k, t) in [m.texte.as_str(), &qty].iter().enumerate() {
                text.draw(pm, t, (x as f32, (ay - R as f64 - 4.0 - lh * (1 - k) as f64) as f32), fs, c, Align::Center);
            }
        }
    }
}

/// A quantity as TradingView writes it: whole, or up to 6 decimals without the trailing zeros.
fn quantite(q: f64) -> String {
    let s = format!("{:.6}", q.abs());
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// 📋 A `table.new` table in its corner (`position.*`) of the area between `top` and `bot`, `w` wide: each column as
/// wide as its longest text, each row as high as its largest text; table, cell and border colours as the script set.
/// 📋 (tableaux-1) Moved / resized by the user like every table ([`crate::indicators::placer`], layout of `id`).
fn draw_table(pm: &mut Pixmap, text: &Text, top: f64, bot: f64, w: f64, t: &argos_script::lang::Table, id: &crate::tableaux::Id) -> Option<Rect> {
    let taille = |c: &argos_script::lang::TableCell| if c.size.is_empty() || c.size == "auto" { 12.0 } else { font(&c.size) };
    let mut largeurs = vec![0.0f64; t.columns];
    let mut hauteurs = vec![0.0f64; t.rows];
    for (r, row) in t.cells.iter().enumerate() {
        for (c, cell) in row.iter().enumerate() {
            if let Some(cell) = cell {
                let f = taille(cell);
                largeurs[c] = largeurs[c].max(text.width(&cell.text, f) as f64 + 12.0);
                hauteurs[r] = hauteurs[r].max(f as f64 * 1.8);
            }
        }
    }
    let (tw, th) = (largeurs.iter().sum::<f64>(), hauteurs.iter().sum::<f64>());
    if tw <= 0.0 || th <= 0.0 {
        return None;
    }
    let lay = crate::tableaux::layout(&id.key);
    // (the size at scale `s`: the texts grow with the cells; the width and row stretches on top)
    let mesure = |s: f64| (tw * s * lay.wx, th * s * lay.hy);
    // (the 9 corners of TradingView; the same margins as the indicator tables: below the legend at the top of the
    // price area, above the event row at its bottom)
    let pos = t.position.trim_start_matches("position.");
    let (mt, mb) = if top == 0.0 { (34.0, 40.0) } else { (24.0, 6.0) };
    // (🧱 in the price pane, a right corner goes left while Argos Murs' pills hold the right side)
    let gauche = pos.ends_with("left") || (pos.ends_with("right") && top == 0.0 && crate::indicators::table_corner_with_walls(0) != 0);
    let defaut = |tw: f64, th: f64| {
        let x = if gauche {
            10.0
        } else if pos.ends_with("center") {
            (w - tw) / 2.0
        } else {
            w - tw - 10.0
        };
        let y = if pos.starts_with("top") {
            top + mt
        } else if pos.starts_with("middle") {
            (top + bot - th) / 2.0
        } else {
            bot - th - mb
        };
        Rect::new(x, y, tw, th)
    };
    // (in the indicator tables' list: the legend layer and the zoom pill see it; never over another table)
    // (and off the reserved places: the legend at the top, the navigation pill and Argos Murs' pills at the bottom)
    let (place, s) = crate::indicators::placer(id, Rect::new(0.0, top, w, bot - top), &mesure, &defaut)?;
    // 🎨 (tour 2, F4) on the tables' layer: over the lines and the walls
    crate::tableaux::dessiner(pm, place, |pm| {
        let (x, y) = (place.x, place.y);
        let (kx, ky) = (s * lay.wx, s * lay.hy);
        let largeurs: Vec<f64> = largeurs.iter().map(|l| l * kx).collect();
        let hauteurs: Vec<f64> = hauteurs.iter().map(|h| h * ky).collect();
        if let Some(c) = t.bgcolor {
            let (rgb, al) = ink(c);
            crate::fill(pm, x, y, place.w, place.h, rgb, al);
        }
        let mut cy = y;
        for (r, row) in t.cells.iter().enumerate() {
            let mut cx = x;
            for (c, cell) in row.iter().enumerate() {
                if let Some(cell) = cell {
                    if let Some(bg) = cell.bgcolor {
                        let (rgb, al) = ink(bg);
                        crate::fill(pm, cx, cy, largeurs[c], hauteurs[r], rgb, al);
                    }
                    let f = taille(cell) * s as f32;
                    let (rgb, _) = ink(cell.text_color);
                    let base = (cy + hauteurs[r] / 2.0 + f as f64 * 0.36) as f32;
                    text.draw(pm, &cell.text, ((cx + largeurs[c] / 2.0) as f32, base), f, rgb, Align::Center);
                }
                if let Some(b) = t.border_color {
                    let (rgb, al) = ink(b);
                    crate::fill(pm, cx, cy + hauteurs[r] - 0.5, largeurs[c], 1.0, rgb, al);
                }
                cx += largeurs[c];
            }
            cy += hauteurs[r];
        }
        if let Some(fc) = t.frame_color {
            let (rgb, _) = ink(fc);
            crate::shapes::round_stroke(pm, place, 0.0, rgb, 1.0);
        }
    });
    Some(place)
}

/// Un `plot` selon son style ; `st` : the colour, width and line type chosen in the ⚙ Style (`None`: the script's).
fn draw_plot(pm: &mut Pixmap, f: &Frame, out: &Output, k: usize, style: PlotStyle, st: argos_script::style_integres::Trace) {
    let g = f.g;
    let p = &out.plots[k];
    let width = st.epaisseur.unwrap_or(p.width);
    let ligne = st.ligne.unwrap_or("");
    let val = |i: usize| series(out, (HandleKind::Plot, k), i);
    let color = |i: usize| {
        (i as i64 - p.offset)
            .try_into()
            .ok()
            .and_then(|j: usize| p.colors.get(j).copied().flatten())
            .map(|c| st.couleur.unwrap_or(c))
    };
    let from = g.first.saturating_sub(1);
    // (shifted ahead, a plot goes on after the last bar as far as the screen shows, as on TradingView: B-046)
    let to = if g.last + 1 >= g.bars.len() { fin_devant(g, p.offset) } else { g.last + 1 }.max(from);
    let base = f.y(0.0).clamp(f.top, f.bot);
    let mut prev: Option<(f64, f64)> = None;
    for i in from..=to {
        let v = val(i);
        let (x, y) = (f.x(i as f64), f.y(v));
        let c = color(i);
        if !v.is_finite() {
            if style != PlotStyle::Line {
                prev = None;
            }
            continue;
        }
        match style {
            PlotStyle::Line | PlotStyle::LineBr | PlotStyle::StepLine | PlotStyle::Area => {
                if let (Some((px, py)), Some(c)) = (prev, c) {
                    let pts: Vec<(f64, f64)> = if style == PlotStyle::StepLine {
                        vec![(px, py), (x, py), (x, y)]
                    } else {
                        vec![(px, py), (x, y)]
                    };
                    if style == PlotStyle::Area {
                        let (rgb, a) = ink(c);
                        poly_fill(
                            pm,
                            0.0,
                            0.0,
                            &[(px, py), (x, y), (x, base), (px, base)].map(|(a, b)| (a as f32, b as f32)),
                            rgb,
                            a * 0.35,
                        );
                    }
                    f.stroke(pm, &pts, c, width, ligne);
                }
                prev = Some((x, y));
            }
            PlotStyle::Histogram | PlotStyle::Columns => {
                if let Some(c) = c {
                    let w = if style == PlotStyle::Columns { f.half * 1.4 } else { width.max(1.0) };
                    f.rect(pm, x - w / 2.0, y, x + w / 2.0, base, c);
                }
            }
            PlotStyle::Circles | PlotStyle::Cross => {
                if let Some(c) = c {
                    let (rgb, a) = ink(c);
                    let r = width.max(1.0) + 1.0;
                    if style == PlotStyle::Circles {
                        round_fill(pm, Rect::new(x - r, y - r, 2.0 * r, 2.0 * r), r, rgb, a);
                    } else {
                        let r = r as f32 + 1.0;
                        lines_alpha(pm, x, y, &[&[(-r, 0.0), (r, 0.0)], &[(0.0, -r), (0.0, r)]], rgb, 1.2, a);
                    }
                }
            }
        }
    }
}

/// Forme de `plotshape` / caractère de `plotchar`, avec son texte.
/// 📌 The `force_overlay = true` marks of a script that has its own band (Pine v6): drawn on the price pane, at
/// prices. A script on the price draws them itself in `draw`.
pub fn draw_forced(pm: &mut Pixmap, text: &Text, g: &Geo, v: &ScriptView) {
    if v.hidden || v.out.overlay {
        return;
    }
    let half = if g.last > g.first {
        (g.x(g.last) - g.x(g.first)) / (g.last - g.first) as f64 / 2.0
    } else {
        2.0
    };
    let f = Frame {
        g,
        top: 0.0,
        bot: g.price_h,
        range: None,
        half,
    };
    let last = g.last.min(g.bars.len().saturating_sub(1));
    for s in v.out.shapes.iter().filter(|s| s.force_overlay && (g.first..=last).contains(&s.bar)) {
        draw_shape(pm, text, &f, s);
    }
    for p in v.out.polylines.iter().filter(|p| p.force_overlay && !p.deleted) {
        draw_polyline(pm, &f, p);
    }
    for d in v.out.drawings.iter().filter(|d| d.force_overlay && !d.deleted) {
        draw_drawing(pm, text, &f, d);
    }
}

fn draw_shape(pm: &mut Pixmap, text: &Text, f: &Frame, s: &Shape) {
    let b = &f.g.bars[s.bar];
    let r = shape_px(&s.size) / 2.0;
    let overlay = f.range.is_none();
    let (y, below) = match s.loc {
        Loc::AboveBar if overlay => (f.g.y(b.high) - r - 4.0, false),
        Loc::BelowBar if overlay => (f.g.y(b.low) + r + 4.0, true),
        Loc::Absolute => (f.y(s.price), false),
        Loc::Bottom | Loc::BelowBar => (f.bot - r - 4.0, true),
        Loc::Top | Loc::AboveBar => (f.top + r + 22.0, false),
    };
    let x = f.x(s.bar as f64);
    let (rgb, a) = ink(s.color);
    let rf = r as f32;
    match s.shape.as_str() {
        "triangleup" => poly_fill(pm, x, y, &[(0.0, -rf), (rf, rf), (-rf, rf)], rgb, a),
        "triangledown" => poly_fill(pm, x, y, &[(0.0, rf), (rf, -rf), (-rf, -rf)], rgb, a),
        "arrowup" => poly_fill(
            pm,
            x,
            y,
            &[
                (0.0, -rf),
                (rf, 0.0),
                (rf * 0.4, 0.0),
                (rf * 0.4, rf),
                (-rf * 0.4, rf),
                (-rf * 0.4, 0.0),
                (-rf, 0.0),
            ],
            rgb,
            a,
        ),
        "arrowdown" => poly_fill(
            pm,
            x,
            y,
            &[
                (0.0, rf),
                (rf, 0.0),
                (rf * 0.4, 0.0),
                (rf * 0.4, -rf),
                (-rf * 0.4, -rf),
                (-rf * 0.4, 0.0),
                (-rf, 0.0),
            ],
            rgb,
            a,
        ),
        "circle" => round_fill(pm, Rect::new(x - r, y - r, 2.0 * r, 2.0 * r), r, rgb, a),
        "square" => crate::fill(pm, x - r, y - r, 2.0 * r, 2.0 * r, rgb, a),
        "diamond" => poly_fill(pm, x, y, &[(0.0, -rf), (rf, 0.0), (0.0, rf), (-rf, 0.0)], rgb, a),
        "cross" => lines_alpha(pm, x, y, &[&[(-rf, 0.0), (rf, 0.0)], &[(0.0, -rf), (0.0, rf)]], rgb, 2.0, a),
        "xcross" => lines_alpha(pm, x, y, &[&[(-rf, -rf), (rf, rf)], &[(-rf, rf), (rf, -rf)]], rgb, 2.0, a),
        "flag" => {
            lines_alpha(pm, x, y, &[&[(-rf * 0.6, rf), (-rf * 0.6, -rf)]], rgb, 1.5, a);
            poly_fill(pm, x, y, &[(-rf * 0.6, -rf), (rf, -rf * 0.5), (-rf * 0.6, 0.0)], rgb, a);
        }
        "labelup" | "labeldown" => {
            bubble(
                pm,
                text,
                x,
                y + if s.shape == "labelup" { -r } else { r },
                if s.shape == "labelup" { "label_up" } else { "label_down" },
                &s.text,
                s.color,
                s.text_color,
                &s.size,
            );
            return;
        }
        c => text.draw(pm, c, (x as f32, (y + r * 0.8) as f32), (2.0 * r).max(10.0) as f32, rgb, Align::Center),
    }
    if !s.text.is_empty() {
        let (tc, _) = ink(s.text_color);
        let fs = font("small");
        let rows: Vec<&str> = s.text.split('\n').collect();
        for (k, row) in rows.iter().enumerate() {
            let ty = if below {
                y + r + 4.0 + fs as f64 * (k as f64 + 1.0)
            } else {
                y - r - 4.0 - fs as f64 * (rows.len() - 1 - k) as f64
            };
            text.draw(pm, row, (x as f32, ty as f32), fs, tc, Align::Center);
        }
    }
}

/// A label drawn as a shape (`label.style_circle`, `style_diamond`, `style_square`, `style_triangleup`…): the shape
/// in the label's colour, centred on its point, its text above. `false`: not a shape style (a bubble or text).
fn label_glyph(pm: &mut Pixmap, text: &Text, x: f64, y: f64, d: &Drawing) -> bool {
    let shape = match d.style.as_str() {
        s @ ("circle" | "diamond" | "square" | "triangleup" | "triangledown" | "arrowup" | "arrowdown" | "cross" | "xcross" | "flag") => s,
        _ => return false,
    };
    let r = shape_px(&d.size) / 2.0;
    let (rgb, a) = ink(d.color);
    let rf = r as f32;
    match shape {
        "circle" => round_fill(pm, Rect::new(x - r, y - r, 2.0 * r, 2.0 * r), r, rgb, a),
        "square" => crate::fill(pm, x - r, y - r, 2.0 * r, 2.0 * r, rgb, a),
        "diamond" => poly_fill(pm, x, y, &[(0.0, -rf), (rf, 0.0), (0.0, rf), (-rf, 0.0)], rgb, a),
        "triangleup" | "arrowup" => poly_fill(pm, x, y, &[(0.0, -rf), (rf, rf), (-rf, rf)], rgb, a),
        "triangledown" | "arrowdown" => poly_fill(pm, x, y, &[(0.0, rf), (rf, -rf), (-rf, -rf)], rgb, a),
        "cross" => lines_alpha(pm, x, y, &[&[(-rf, 0.0), (rf, 0.0)], &[(0.0, -rf), (0.0, rf)]], rgb, 2.0, a),
        "xcross" => lines_alpha(pm, x, y, &[&[(-rf, -rf), (rf, rf)], &[(-rf, rf), (rf, -rf)]], rgb, 2.0, a),
        _ => {
            lines_alpha(pm, x, y, &[&[(-rf * 0.6, rf), (-rf * 0.6, -rf)]], rgb, 1.5, a);
            poly_fill(pm, x, y, &[(-rf * 0.6, -rf), (rf, -rf * 0.5), (-rf * 0.6, 0.0)], rgb, a);
        }
    }
    let (tc, ta) = ink(d.text_color);
    if !d.text.is_empty() && ta > 0.0 {
        text.draw(pm, &d.text, (x as f32, (y - r - 4.0) as f32), font(&d.size), tc, Align::Center);
    }
    true
}

/// A script's label is drawn where the script puts it, never slid (Argos Vague turn 2): one anchored right of the last
/// bar (`apres`: that bar's x) that does not fit whole between its anchor and 4 px before the price scale (`right`)
/// is not drawn at all (a cut word looks broken).
#[allow(clippy::too_many_arguments)]
fn bubble_dans(pm: &mut Pixmap, text: &Text, x: f64, y: f64, d: &Drawing, right: f64, apres: f64) {
    if tient(text, x, &d.style, &d.text, &d.size, right, apres) {
        bubble(pm, text, x, y, &d.style, &d.text, d.color, d.text_color, &d.size);
    }
}

/// Whether a label at `x` is drawn: always left of the last bar's x `apres`; right of it, only when its bubble ends
/// at least 4 px before `right`.
fn tient(text: &Text, x: f64, style: &str, s: &str, size: &str, right: f64, apres: f64) -> bool {
    let (bx, w) = bubble_x(text, x, style, s, size);
    x <= apres + 0.5 || bx + w <= right - 4.0
}

/// Left edge and width of a label's bubble with its tip on `x` (the same sums as `bubble`).
fn bubble_x(text: &Text, x: f64, style: &str, s: &str, size: &str) -> (f64, f64) {
    let fs = font(size);
    let tw = s.split('\n').map(|r| text.width(r, fs)).fold(0.0f32, f32::max) as f64;
    let w = tw + 12.0;
    const TIP: f64 = 6.0;
    let bx = match style {
        "label_left" => x + TIP,
        "label_right" => x - TIP - w,
        "label_upper_left" | "label_lower_left" => x,
        "label_upper_right" | "label_lower_right" => x - w,
        _ => x - w / 2.0,
    };
    (bx, w)
}

/// Étiquette : bulle de couleur `bg` avec sa pointe sur `(x, y)` selon le style Pine (`label_down` par défaut).
#[allow(clippy::too_many_arguments)]
fn bubble(pm: &mut Pixmap, text: &Text, x: f64, y: f64, style: &str, s: &str, bg: u32, tc: u32, size: &str) {
    let fs = font(size);
    let rows: Vec<&str> = s.split('\n').collect();
    let tw = rows.iter().map(|r| text.width(r, fs)).fold(0.0f32, f32::max) as f64;
    let lh = fs as f64 * 1.3;
    let (w, h) = (tw + 12.0, rows.len() as f64 * lh + 6.0);
    const TIP: f64 = 6.0;
    let (rgb, a) = ink(bg);
    let (bx, by, tip): (f64, f64, Option<[(f64, f64); 3]>) = match style {
        "label_up" => (x - w / 2.0, y + TIP, Some([(x, y), (x - TIP, y + TIP), (x + TIP, y + TIP)])),
        "label_left" => (x + TIP, y - h / 2.0, Some([(x, y), (x + TIP, y - TIP), (x + TIP, y + TIP)])),
        "label_right" => (x - TIP - w, y - h / 2.0, Some([(x, y), (x - TIP, y - TIP), (x - TIP, y + TIP)])),
        "label_center" | "none" | "text_outline" => (x - w / 2.0, y - h / 2.0, None),
        "label_upper_left" => (x, y, None),
        "label_upper_right" => (x - w, y, None),
        "label_lower_left" => (x, y - h, None),
        "label_lower_right" => (x - w, y - h, None),
        _ => (x - w / 2.0, y - TIP - h, Some([(x, y), (x - TIP, y - TIP), (x + TIP, y - TIP)])),
    };
    if !matches!(style, "none" | "text_outline") && a > 0.0 {
        round_fill(pm, Rect::new(bx, by, w, h), 3.0, rgb, a);
        if let Some(t) = tip {
            poly_fill(pm, 0.0, 0.0, &t.map(|(a, b)| (a as f32, b as f32)), rgb, a);
        }
    }
    let (tcol, ta) = ink(tc);
    if ta > 0.0 {
        for (k, row) in rows.iter().enumerate() {
            text.draw(
                pm,
                row,
                ((bx + w / 2.0) as f32, (by + 3.0 + lh * (k as f64 + 1.0) - lh * 0.25) as f32),
                fs,
                tcol,
                Align::Center,
            );
        }
    }
}

/// Dessin d'un script : `label`, `line` ou `box`.
/// A drawing's two points on the screen: (x1, y1, x2, y2).
fn points(f: &Frame, d: &Drawing) -> (f64, f64, f64, f64) {
    let (x1, y1) = xy(f, d.xtime, d.x1, d.y1);
    let (x2, y2) = xy(f, d.xtime, d.x2, d.y2);
    (x1, y1, x2, y2)
}

/// A script point on screen: x as a bar index, or as a time in ms (`xtime`), carried on after the last bar; y a price.
fn xy(f: &Frame, xtime: bool, x: f64, y: f64) -> (f64, f64) {
    let g = f.g;
    // x en temps (ms) → index de bougie, prolongé après la dernière bougie
    let bar = |x: f64| -> f64 {
        if !xtime || g.bars.len() < 2 {
            return x;
        }
        let t = x / 1000.0;
        let n = g.bars.len();
        let step = (g.bars[n - 1].time - g.bars[n - 2].time).max(1) as f64;
        let last = g.bars[n - 1].time as f64;
        if t > last {
            return (n - 1) as f64 + (t - last) / step;
        }
        g.bars.partition_point(|b| (b.time as f64) < t) as f64
    };
    (f.x(bar(x)), f.y(y))
}

/// 〰️ A `polyline`: its points joined (a smooth Catmull-Rom curve through them when `curved`), back to the first one
/// when `closed`, its inside filled with its fill colour.
fn draw_polyline(pm: &mut Pixmap, f: &Frame, p: &argos_script::lang::Polyline) {
    let pts: Vec<(f64, f64)> = p
        .points
        .iter()
        .map(|&(x, y)| xy(f, p.xtime, x, y))
        .filter(|(x, y)| x.is_finite() && y.is_finite())
        .collect();
    if pts.len() < 2 {
        return;
    }
    let n = pts.len();
    let mut path = if p.curved {
        let at = |k: isize| -> (f64, f64) {
            if p.closed {
                pts[k.rem_euclid(n as isize) as usize]
            } else {
                pts[k.clamp(0, n as isize - 1) as usize]
            }
        };
        let segments = if p.closed { n } else { n - 1 };
        let mut v = vec![pts[0]];
        for k in 0..segments as isize {
            let (p0, p1, p2, p3) = (at(k - 1), at(k), at(k + 1), at(k + 2));
            for s in 1..=12 {
                let t = s as f64 / 12.0;
                let c = |a: f64, b: f64, c: f64, d: f64| {
                    0.5 * (2.0 * b + (c - a) * t + (2.0 * a - 5.0 * b + 4.0 * c - d) * t * t + (3.0 * b - a - 3.0 * c + d) * t * t * t)
                };
                v.push((c(p0.0, p1.0, p2.0, p3.0), c(p0.1, p1.1, p2.1, p3.1)));
            }
        }
        v
    } else {
        pts.clone()
    };
    if p.closed && !p.curved {
        path.push(pts[0]);
    }
    if let Some(c) = p.fill {
        let (rgb, al) = ink(c);
        if al > 0.0 {
            let f32s: Vec<(f32, f32)> = path.iter().map(|q| (q.0 as f32, q.1 as f32)).collect();
            poly_fill(pm, 0.0, 0.0, &f32s, rgb, al);
        }
    }
    f.stroke(pm, &path, p.color, p.width, &p.style);
}

/// A `linefill`: the area between its two lines, from where they are now; nothing once one of them is deleted.
fn draw_linefill(pm: &mut Pixmap, f: &Frame, out: &Output, lf: &argos_script::lang::Linefill) {
    let (Some(a), Some(b)) = (out.drawings.get(lf.line1), out.drawings.get(lf.line2)) else {
        return;
    };
    if lf.deleted || a.deleted || b.deleted || a.kind != "line" || b.kind != "line" {
        return;
    }
    let (a1x, a1y, a2x, a2y) = points(f, a);
    let (b1x, b1y, b2x, b2y) = points(f, b);
    if ![a1x, a1y, a2x, a2y, b1x, b1y, b2x, b2y].iter().all(|v| v.is_finite()) {
        return;
    }
    let (rgb, al) = ink(lf.color);
    let pts = [(a1x, a1y), (a2x, a2y), (b2x, b2y), (b1x, b1y)].map(|(x, y)| (x as f32, y as f32));
    poly_fill(pm, 0.0, 0.0, &pts, rgb, al);
}

fn draw_drawing(pm: &mut Pixmap, text: &Text, f: &Frame, d: &Drawing) {
    let g = f.g;
    let (x1, y1, x2, y2) = points(f, d);
    let right = g.right();
    match d.kind {
        "label" => {
            // (a shape style draws the shape, the others a bubble or text)
            if x1.is_finite() && y1.is_finite() && (-200.0..right + 200.0).contains(&x1) && !label_glyph(pm, text, x1, y1, d) {
                let apres = f.x(g.bars.len().saturating_sub(1) as f64);
                bubble_dans(pm, text, x1, y1, d, right, apres);
            }
        }
        "line" => {
            if ![x1, x2, y1, y2].iter().all(|v| v.is_finite()) {
                return;
            }
            // extend : jusqu'aux bords, dans le sens de la ligne (gauche = côté du x le plus petit)
            let (mut a, mut b) = if x1 <= x2 { ((x1, y1), (x2, y2)) } else { ((x2, y2), (x1, y1)) };
            let dx = b.0 - a.0;
            if dx > 1e-9 {
                let slope = (b.1 - a.1) / dx;
                let at = |x: f64| (x, y1 + (x - x1) * slope);
                if matches!(d.extend.as_str(), "left" | "both") {
                    a = at(-10.0);
                }
                if matches!(d.extend.as_str(), "right" | "both") {
                    b = at(right + 10.0);
                }
            }
            f.stroke(pm, &[a, b], d.color, d.width, &d.style);
            let (rgb, al) = ink(d.color);
            for (tip, from, on) in [
                (b, a, matches!(d.style.as_str(), "arrow_right" | "arrow_both")),
                (a, b, matches!(d.style.as_str(), "arrow_left" | "arrow_both")),
            ] {
                if on {
                    let ang = (tip.1 - from.1).atan2(tip.0 - from.0);
                    let k = 8.0 + d.width;
                    let p = |da: f64| ((tip.0 - k * (ang + da).cos()) as f32, (tip.1 - k * (ang + da).sin()) as f32);
                    poly_fill(pm, 0.0, 0.0, &[(tip.0 as f32, tip.1 as f32), p(0.45), p(-0.45)], rgb, al);
                }
            }
        }
        _ => {
            if ![x1, x2, y1, y2].iter().all(|v| v.is_finite()) {
                return;
            }
            let (mut l, mut r) = (x1.min(x2), x1.max(x2));
            if matches!(d.extend.as_str(), "left" | "both") {
                l = 0.0;
            }
            if matches!(d.extend.as_str(), "right" | "both") {
                r = right;
            }
            let (t, b) = (y1.min(y2), y1.max(y2));
            // 📜 a box wholly above or below its pane draws nothing (no fill, no border, no text)
            if b < f.top || t > f.bot {
                return;
            }
            f.rect(pm, l, t, r, b, d.bg);
            // the visible part: the whole box when it is inside, else cut at the edge it crosses
            let (vt, vb) = (t.max(f.top), b.min(f.bot));
            if d.width > 0.0 {
                if t >= f.top && b <= f.bot {
                    f.stroke(pm, &[(l, t), (r, t), (r, b), (l, b), (l, t)], d.color, d.width, &d.style);
                } else {
                    // crossing an edge: the two sides cut to the pane, the top or bottom edge only when inside
                    f.stroke(pm, &[(l, vt), (l, vb)], d.color, d.width, &d.style);
                    f.stroke(pm, &[(r, vt), (r, vb)], d.color, d.width, &d.style);
                    if t >= f.top {
                        f.stroke(pm, &[(l, t), (r, t)], d.color, d.width, &d.style);
                    }
                    if b <= f.bot {
                        f.stroke(pm, &[(l, b), (r, b)], d.color, d.width, &d.style);
                    }
                }
            }
            if !d.text.is_empty() {
                let (tc, _) = ink(d.text_color);
                text.draw(
                    pm,
                    &d.text,
                    (((l + r) / 2.0) as f32, ((vt + vb) / 2.0 + 4.0) as f32),
                    font(&d.size),
                    tc,
                    Align::Center,
                );
            }
        }
    }
}

/// 🌊 The margin of Argos Vague's pills: from the last bar, the gauge's ruler and the right edge.
pub const VAGUE_PILL_MARGIN: f64 = 6.0;
/// 🌊 Under this line from the pane's top the pane's legend (name, 👁 ⚙ { } …) is: no pill goes there.
const VAGUE_PILL_TOP: f64 = 32.0;
/// 🌊 The space between two pills pushed apart.
const VAGUE_PILL_GAP: f64 = 2.0;

/// 🌊 One of Argos Vague's pills, laid out: its place, its lines (one, or its label cut in two), font and ink.
#[derive(Debug, Clone)]
struct PiluleVague {
    r: Rect,
    lignes: Vec<String>,
    fs: f32,
    ink: u32,
    texte: String,
}

thread_local! {
    /// 🌊 Argos Vague's pills of its last drawing (chart coordinates), drawn after the value columns
    /// ([`draw_vague_pills`]).
    static VAGUE_PILLS: std::cell::RefCell<Vec<PiluleVague>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// 🌊 Argos Vague's pills of its last drawing (chart coordinates) and their text: SURACHAT, TENDANCE HAUSSIÈRE /
/// BAISSIÈRE, the state (« ● BAISSE FAIBLE … »), SURVENTE.
pub fn vague_pills() -> Vec<(Rect, String)> {
    VAGUE_PILLS.with(|p| p.borrow().iter().map(|p| (p.r, p.texte.clone())).collect())
}

/// 🌊 Forgets the pills of the image before (each image lays them out again when Argos Vague is drawn).
pub fn vague_pills_reset() {
    VAGUE_PILLS.with(|p| p.borrow_mut().clear());
}

/// 🌊 One of Argos Vague's gauge labels (its four words and its state, which it anchors right of the last bar, in its
/// pane): drawn as glass pills by [`draw_vague_pills`], not where the script puts them.
fn is_vague_gauge_label(f: &Frame, v: &ScriptView, d: &Drawing) -> bool {
    f.range.is_some()
        && v.out.title == "Argos Vague"
        && d.kind == "label"
        && d.style == "label_left"
        && points(f, d).0 > f.x(f.g.bars.len().saturating_sub(1) as f64) + 0.5
}

/// 🌊 « ● BAISSE FAIBLE (-25 · CVD+) » cut in two lines at the space nearest its middle (`None`: one word).
fn en_deux_lignes(s: &str) -> Option<Vec<String>> {
    let mid = s.chars().count() as i64 / 2;
    let i = s
        .char_indices()
        .filter(|(_, c)| *c == ' ')
        .map(|(i, _)| i)
        .min_by_key(|i| (s[..*i].chars().count() as i64 - mid).abs())?;
    let (a, b) = (s[..i].trim(), s[i..].trim());
    (!a.is_empty() && !b.is_empty()).then(|| vec![a.to_string(), b.to_string()])
}

/// 🌊 Where Argos Vague's gauge labels go (Jee, 24 Nov: « Pilules verre par-dessus, à droite », then « je ne veux pas
/// qu'elle embarque sur l'oscillateur »): each its own glass pill at the height of its level, never left of the last
/// bar (the curves). First right of the gauge's ruler, against the value column; with no room there, starting right
/// of the ruler over the value column; then the same right of the last bar (over the ruler). Of the labels of one
/// level (the state's « ● », short, full) the longest that fits right of the ruler is kept, whole or cut in two
/// lines (over the value column rather than shorter). A level out of
/// the pane's view (its view dragged) has no pill; a pill touching the one above goes down just enough, and back up
/// from the bottom when there is no room. With the last bar off the screen: none.
fn vague_pill_layout(text: &Text, f: &Frame, v: &ScriptView) -> Vec<PiluleVague> {
    let labels: Vec<&Drawing> = v.out.drawings.iter().filter(|d| !d.deleted && is_vague_gauge_label(f, v, d)).collect();
    if labels.is_empty() {
        return vec![];
    }
    let (cr, m) = (f.g.right(), VAGUE_PILL_MARGIN);
    let fr = cr + f.g.vp.price_axis as f64;
    let apres = f.x(f.g.bars.len().saturating_sub(1) as f64);
    if apres + f.half + m >= cr {
        return vec![];
    }
    // the gauge's ruler (its right edge), when it is on the screen
    let regle = v
        .out
        .drawings
        .iter()
        .filter(|d| !d.deleted && d.kind == "line" && d.x1 == d.x2)
        .map(|d| (points(f, d).0, d.width))
        .filter(|(x, _)| *x > apres && *x < cr)
        .map(|(x, w)| x + w / 2.0)
        .fold(f64::NEG_INFINITY, f64::max);
    // (left edge, right edge, right-aligned): right of the ruler, then right of the last bar; in each, against the
    // value column, then over it
    let groupes: Vec<[(f64, f64, bool); 2]> = [regle + m, apres + f.half + m]
        .into_iter()
        .filter(|g| g.is_finite() && *g < cr)
        .map(|g| [(g, cr - m, true), (g, fr - m, false)])
        .collect();
    let taille = |d: &Drawing, lignes: &[String]| {
        let fs = font(&d.size);
        let w = lignes.iter().map(|l| text.width(l, fs) as f64).fold(0.0, f64::max) + 16.0;
        (w, fs as f64 * 1.3 * lignes.len() as f64 + 6.0)
    };
    let mut pills: Vec<PiluleVague> = vec![];
    let mut faits: Vec<(f64, f64)> = vec![];
    for d in &labels {
        if faits.contains(&(d.x1, d.y1)) {
            continue;
        }
        faits.push((d.x1, d.y1));
        let y = f.y(d.y1);
        // (its level out of the pane's view: no pill where the level is not)
        if !(f.top + VAGUE_PILL_TOP..=f.bot).contains(&y) {
            continue;
        }
        let mut same: Vec<&Drawing> = labels.iter().copied().filter(|o| o.x1 == d.x1 && o.y1 == d.y1).collect();
        same.sort_by(|a, b| {
            taille(b, std::slice::from_ref(&b.text))
                .0
                .total_cmp(&taille(a, std::slice::from_ref(&a.text)).0)
        });
        let choix: Vec<(&Drawing, Vec<String>)> = same
            .iter()
            .flat_map(|o| {
                [Some(vec![o.text.clone()]), en_deux_lignes(&o.text)]
                    .into_iter()
                    .flatten()
                    .map(move |l| (*o, l))
            })
            .collect();
        // (the longest text that fits right of the ruler, even over the value column, before a shorter one: the
        // state keeps its words on a PC where only its « ● » fits against the column)
        let pick = groupes.iter().find_map(|zones| {
            choix.iter().find_map(|(o, l)| {
                let (w, h) = taille(o, l);
                zones.iter().find(|(g, dr, _)| w <= dr - g).map(|(g, dr, colle)| {
                    let x = if *colle { dr - w } else { *g };
                    (Rect::new(x, y - h / 2.0, w, h), *o, l.clone())
                })
            })
        });
        if let Some((r, o, lignes)) = pick {
            pills.push(PiluleVague {
                r,
                lignes,
                fs: font(&o.size),
                ink: o.text_color,
                texte: o.text.clone(),
            });
        }
    }
    pills.sort_by(|a, b| a.r.y.total_cmp(&b.r.y));
    let (lo, hi) = (f.top + VAGUE_PILL_TOP, f.bot - 2.0);
    let mut bas = lo - VAGUE_PILL_GAP;
    for p in pills.iter_mut() {
        p.r.y = p.r.y.max(bas + VAGUE_PILL_GAP);
        bas = p.r.y + p.r.h;
    }
    let mut haut = hi + VAGUE_PILL_GAP;
    for p in pills.iter_mut().rev() {
        p.r.y = p.r.y.min(haut - VAGUE_PILL_GAP - p.r.h);
        haut = p.r.y;
    }
    pills
}

/// 🌊 Lays out Argos Vague's gauge labels ([`vague_pill_layout`]) for [`draw_vague_pills`]. Nothing for another
/// script.
fn lay_vague_pills(text: &Text, f: &Frame, v: &ScriptView) {
    if f.range.is_none() || v.out.title != "Argos Vague" {
        return;
    }
    let pills = vague_pill_layout(text, f, v);
    VAGUE_PILLS.with(|p| *p.borrow_mut() = pills);
}

/// 🌊 Draws Argos Vague's pills laid out by its last drawing, over everything of its pane and of its value column
/// (after the value columns): the app's glass, their text and colour as the script gives them.
pub fn draw_vague_pills(pm: &mut Pixmap, text: &Text) {
    let pills = VAGUE_PILLS.with(|p| p.borrow().clone());
    let fond = crate::Theme::current().background;
    for p in &pills {
        // (on the chart's background first: over the value column its numbers do not show through the glass)
        crate::shapes::round_fill(pm, p.r, crate::jetons::pilule(p.r), fond, 1.0);
        crate::jetons::verre(pm, p.r, crate::jetons::pilule(p.r));
        let (tc, ta) = ink(p.ink);
        if ta > 0.0 {
            let lh = p.fs as f64 * 1.3;
            for (k, l) in p.lignes.iter().enumerate() {
                let y = p.r.y + 3.0 + lh * 0.75 + k as f64 * lh;
                text.draw(pm, l, ((p.r.x + p.r.w / 2.0) as f32, y as f32), p.fs, tc, Align::Center);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 📜 Argos Vague turn 2 (R2 a): a label right of the last bar is never slid left: drawn at its anchor when it
    /// fits whole before the price scale (4 px), not drawn at all otherwise; left of the last bar, always drawn.
    #[test]
    fn etiquette_apres_la_derniere_bougie_jamais_glissee() {
        let text = Text::default();
        for right in [1100.0, 330.0] {
            let apres = right - 120.0;
            for (style, s, size) in [
                ("label_left", "TENDANCE HAUSSIÈRE", "tiny"),
                ("label_left", "SURACHAT", "tiny"),
                ("label_left", "● HAUSSE FAIBLE (35 · CVD+)", "small"),
            ] {
                let (_, w) = bubble_x(&text, 0.0, style, s, size);
                // room for it: drawn where it is
                let x = right - 4.0 - 6.0 - w - 1.0;
                assert!(tient(&text, x, style, s, size, right, x - 1.0), "{right} {s}: fits, right of the last bar");
                // no room: not drawn, never moved
                let x = right - 10.0;
                assert!(!tient(&text, x, style, s, size, right, apres), "{right} {s}: no room");
                let mut pm = crate::px::new(right as u32 + 60, 100).unwrap();
                let d = Drawing {
                    kind: "label",
                    x1: 0.0,
                    y1: 0.0,
                    x2: 0.0,
                    y2: 0.0,
                    xtime: false,
                    text: s.into(),
                    color: 0x1E222D,
                    bg: 0,
                    text_color: 0xFFFFFF,
                    style: style.into(),
                    width: 1.0,
                    extend: "none".into(),
                    size: size.into(),
                    deleted: false,
                    force_overlay: false,
                };
                bubble_dans(&mut pm, &text, x, 50.0, &d, right, apres);
                assert!(pm.data().iter().all(|b| *b == 0), "{right} {s}: nothing drawn, nothing slid");
                // left of the last bar: always drawn
                assert!(tient(&text, x, style, s, size, right, x + 1.0));
            }
        }
    }

    /// 🌊 Argos Vague's gauge (Jee, 24 Nov: « Pilules verre par-dessus, à droite », then « je ne veux pas qu'elle
    /// embarque sur l'oscillateur »): its ruler stays right of the last bar; its words and its state are glass pills at
    /// their level, never left of the last bar (on the curves): right of the ruler against the value column, or over
    /// the value column when the chart has no room; below the pane's legend, one apart from the other; none with the
    /// last bar off the screen; at 402, 820 and 1440 points, with a small or a wide margin, and with the pane zoomed
    /// (a level out of its view has no pill).
    #[test]
    fn vague_gauge_labels_as_glass_pills() {
        let bars: Vec<Bar> = (0..300)
            .map(|i| {
                let t = i as f64;
                let c = 100.0 + 8.0 * (t / 17.0).sin() + 4.0 * (t / 5.0).sin();
                Bar {
                    time: 1_700_000_000 + i * 1800,
                    open: c - 0.4,
                    high: c + 1.2,
                    low: c - 1.3,
                    close: c + 0.3,
                    volume: 1000.0 + 300.0 * (t / 7.0).cos(),
                }
            })
            .collect();
        let vue = |jauge: bool| {
            let inputs = HashMap::from([("Jauge de confluence".to_string(), argos_script::lang::InVal::Bool(jauge))]);
            ScriptView {
                name: "Argos Vague".into(),
                out: argos_script::lang::run(argos_script::integres::ARGOS_VAGUE, &bars, 1800, &inputs).unwrap(),
                ..Default::default()
            }
        };
        let (avec, sans) = (vue(true), vue(false));
        let last = bars.len() - 1;
        let (top, h) = (300.0, 300.0);
        for width in [402u32, 820, 1440] {
            for marge in [30.0, 52.0, 300.0, -120.0] {
                let quoi = format!("{width} / {marge}");
                let vp = Viewport {
                    width,
                    height: 600,
                    bar_spacing: 6.0,
                    right_offset: marge / 6.0,
                    ..Default::default()
                };
                let slots = Slots::uniform(bars.len());
                let scale = PriceScale::fit(&bars, 0, last, 300.0);
                let g = Geo {
                    vp: &vp,
                    scale: &scale,
                    slots: &slots,
                    first: last - 40,
                    last,
                    bars: &bars,
                    price_h: 300.0,
                    tz_offset_s: 0,
                };
                let peint = |v: &ScriptView, view: (f64, f64)| {
                    let mut pm = crate::px::new(width, 600).unwrap();
                    vague_pills_reset();
                    draw_view(&mut pm, &Text::default(), &g, v, Some((top, h)), view);
                    draw_vague_pills(&mut pm, &Text::default());
                    (pm, vague_pills())
                };
                let (b, aucune) = peint(&sans, (0.0, 1.0));
                assert!(aucune.is_empty(), "{quoi}: no gauge, no pill");
                let (a, pills) = peint(&avec, (0.0, 1.0));
                let right = g.right();
                let regle = g.xf(last as f64 + 3.0);
                let bougie = g.xf(last as f64) + 3.0;
                // the ruler at its place when it is on the screen
                if regle < right {
                    let x = regle as u32;
                    assert!((top as u32..600).any(|y| a.pixel(x, y) != b.pixel(x, y)), "{quoi}: the ruler");
                }
                if bougie + VAGUE_PILL_MARGIN >= right {
                    assert!(pills.is_empty(), "{quoi}: the last bar off the screen, no pill {pills:?}");
                    continue;
                }
                assert!(pills.len() >= 3, "{quoi}: the words and the state {pills:?}");
                assert!(pills.iter().any(|(_, t)| t.starts_with('●')), "{quoi}: the state");
                for (r, t) in &pills {
                    assert!(r.x >= bougie + VAGUE_PILL_MARGIN - 0.01, "{quoi}: {t} never on the curves {r:?} {bougie}");
                    assert!(
                        r.x + r.w <= right + vp.price_axis as f64 - VAGUE_PILL_MARGIN + 0.01,
                        "{quoi}: {t} on the screen {r:?}"
                    );
                    assert!(
                        r.x + r.w <= right - VAGUE_PILL_MARGIN + 0.01 || r.x >= regle.min(bougie) - 0.01,
                        "{quoi}: {t} against the value column, or starting right of the ruler {r:?}"
                    );
                    assert!(r.y >= top + VAGUE_PILL_TOP && r.y + r.h <= top + h, "{quoi}: {t} in the pane {r:?}");
                    let (x, y) = ((r.x + 3.0) as u32, (r.y + r.h / 2.0) as u32);
                    assert_ne!(a.pixel(x, y), b.pixel(x, y), "{quoi}: {t} drawn");
                }
                // with room right of the ruler: all of them there, against the value column
                if marge >= 300.0 {
                    for (r, t) in &pills {
                        assert!(r.x > regle && (r.x + r.w - (right - VAGUE_PILL_MARGIN)).abs() < 0.01, "{quoi}: {t} {r:?}");
                    }
                }
                for (i, (p, _)) in pills.iter().enumerate() {
                    for (q, _) in &pills[i + 1..] {
                        assert!(p.y + p.h <= q.y + 0.01 || q.y + q.h <= p.y + 0.01, "{quoi}: {p:?} on {q:?}");
                    }
                }
                // the pane zoomed: they follow its scale, still in it and apart; a level out of its view has none
                let (_, zoom) = peint(&avec, (0.0, 3.0));
                for (r, t) in &zoom {
                    assert!(
                        r.y >= top + VAGUE_PILL_TOP - 0.01 && r.y + r.h <= top + h + 0.01,
                        "{quoi}: {t} zoomed, in the pane {r:?}"
                    );
                }
                let (_, loin) = peint(&avec, (1000.0, 1.0));
                assert!(loin.is_empty(), "{quoi}: the pane dragged far away, its levels out of view: {loin:?}");
            }
        }
    }

    /// 📋 Step 4: a script's table is drawn in its corner (top right here) with its cell colours; nothing elsewhere.
    #[test]
    fn tableau_d_un_script_dans_son_coin() {
        let src = "//@version=6\nindicator(\"t\", overlay = true)\nvar t = table.new(position.top_right, 2, 2, bgcolor = color.navy)\nif barstate.islast\n    table.cell(t, 0, 0, \"Trend\", text_color = color.white)\n    table.cell(t, 1, 0, \"UP\", text_color = color.white, bgcolor = color.green)\n    table.cell(t, 0, 1, \"RSI\", text_color = color.white)\n    table.cell(t, 1, 1, \"55\", text_color = color.white)\nplot(close)\n";
        let bars: Vec<argos_core::Bar> = (0..30)
            .map(|i| argos_core::Bar {
                time: 1_700_000_000 + i * 60,
                open: 1.0,
                high: 2.0,
                low: 0.5,
                close: 1.5,
                volume: 1.0,
            })
            .collect();
        let out = argos_script::lang::run(src, &bars, 60, &std::collections::HashMap::new()).unwrap();
        assert_eq!(out.tables.len(), 1);
        let t = &out.tables[0];
        assert_eq!((t.columns, t.rows, t.position.as_str()), (2, 2, "top_right"));
        assert_eq!(t.cells[0][1].as_ref().map(|c| c.text.as_str()), Some("UP"));
        let mut pm = crate::px::new(400, 300).unwrap();
        crate::indicators::fit_bands(&Default::default(), 300.0); // (a new drawing: no table placed yet)
        let r = draw_table(&mut pm, &Text::default(), 0.0, 300.0, 400.0, t, &crate::tableaux::Id::script("t", 0, "t")).unwrap();
        assert!(!crate::indicators::poser_tableau(r), "in the indicator tables' list");
        let peint = |x: u32, y: u32| pm.pixel(x, y).unwrap().alpha() > 0;
        assert!(peint(385, 40), "top right corner (below the legend margin)");
        assert!(!peint(15, 285), "nothing at the bottom left");
    }

    /// 📋 The 9 corners of TradingView (position.top_left … bottom_right): left / center / right, top / middle /
    /// bottom of the area.
    #[test]
    fn tableau_les_9_positions() {
        let cell = argos_script::lang::TableCell {
            text: "AB".into(),
            text_color: 0xffffff,
            ..Default::default()
        };
        let coins = [
            "top_left",
            "top_center",
            "top_right",
            "middle_left",
            "middle_center",
            "middle_right",
            "bottom_left",
            "bottom_center",
            "bottom_right",
        ];
        for (k, pos) in coins.iter().enumerate() {
            let t = argos_script::lang::Table {
                position: pos.to_string(),
                columns: 1,
                rows: 1,
                cells: vec![vec![Some(cell.clone())]],
                ..Default::default()
            };
            let mut pm = crate::px::new(600, 400).unwrap();
            crate::indicators::fit_bands(&Default::default(), 400.0);
            let r = draw_table(&mut pm, &Text::default(), 0.0, 400.0, 600.0, &t, &crate::tableaux::Id::script("t", 0, "t")).unwrap();
            let (col, row) = (k % 3, k / 3);
            let cx = [10.0 + r.w / 2.0, 300.0, 590.0 - r.w / 2.0][col];
            let cy = [34.0 + r.h / 2.0, 200.0, 360.0 - r.h / 2.0][row];
            assert!((r.cx() - cx).abs() < 0.01 && (r.cy() - cy).abs() < 0.01, "{pos}: {r:?}");
        }
    }
    use crate::viewport::{PriceScale, Slots, Viewport};
    use argos_core::Bar;
    use std::collections::HashMap;

    #[test]
    fn script_dessine_sur_le_prix_et_dans_sa_bande() {
        let bars: Vec<Bar> = (0..200)
            .map(|i| {
                let c = 100.0 + (i as f64 / 9.0).sin() * 5.0;
                Bar {
                    time: i * 60,
                    open: c - 0.3,
                    high: c + 1.0,
                    low: c - 1.0,
                    close: c,
                    volume: 100.0,
                }
            })
            .collect();
        let src = "//@version=6\nindicator(\"Test\", overlay = true)\nm = ta.sma(close, 10)\nplot(m, color = color.orange, linewidth = 2)\nbgcolor(close > m ? color.new(color.green, 80) : na)\nplotshape(ta.crossover(close, m), style = shape.triangleup, location = location.belowbar, color = color.lime)\nif barstate.islast\n    label.new(bar_index, high, \"Fin\")\n    line.new(bar_index - 20, low, bar_index, high, extend = extend.right, style = line.style_dashed)\n    box.new(bar_index - 30, high, bar_index - 10, low, bgcolor = color.new(color.blue, 70))";
        let out = argos_script::lang::run(src, &bars, 60, &HashMap::new()).unwrap();
        let v = ScriptView {
            name: "t".into(),
            out,
            ..Default::default()
        };
        let mut pm = crate::px::new(800, 600).unwrap();
        let vp = Viewport {
            width: 800,
            height: 600,
            bar_spacing: 3.5,
            right_offset: 0.0,
            ..Default::default()
        };
        let slots = Slots::uniform(bars.len());
        let scale = PriceScale::fit(&bars, 0, bars.len() - 1, 600.0);
        let g = Geo {
            vp: &vp,
            scale: &scale,
            slots: &slots,
            first: 0,
            last: bars.len() - 1,
            bars: &bars,
            price_h: 600.0,
            tz_offset_s: 0,
        };
        let before = pm.data().to_vec();
        draw(&mut pm, &Text::default(), &g, &v, None);
        assert_ne!(before, pm.data(), "rien de dessiné sur le prix");
        let mut band = crate::px::new(800, 600).unwrap();
        draw(&mut band, &Text::default(), &g, &v, Some((400.0, crate::indicators::OSC_H)));
        // la bande peint son fond en bas, pas en haut
        assert_eq!(band.pixel(5, 100).unwrap().alpha(), 0);
        assert!(band.pixel(5, 450).unwrap().alpha() > 0);
    }

    /// 🧪 A strategy's fills on the price: blue under the bar that buys, pink above the bar that sells, at its bar;
    /// nothing for a hidden strategy or an indicator.
    #[test]
    fn ordres_d_une_strategie_sur_le_prix() {
        let bars: Vec<Bar> = (0..60)
            .map(|i| Bar {
                time: i * 60,
                open: 100.0,
                high: 101.0,
                low: 99.0,
                close: 100.0,
                volume: 1.0,
            })
            .collect();
        let src =
            "//@version=6\nstrategy(\"S\")\nif bar_index == 29\n    strategy.entry(\"L\", strategy.long)\nif bar_index == 39\n    strategy.close(\"L\")\n";
        let out = argos_script::lang::run(src, &bars, 60, &HashMap::new()).unwrap();
        let mut v = ScriptView {
            name: "s".into(),
            out,
            ..Default::default()
        };
        let vp = Viewport {
            width: 600,
            height: 400,
            bar_spacing: 10.0,
            right_offset: 0.0,
            ..Default::default()
        };
        let slots = Slots::uniform(bars.len());
        let scale = PriceScale::fit(&bars, 0, bars.len() - 1, 400.0);
        let g = Geo {
            vp: &vp,
            scale: &scale,
            slots: &slots,
            first: 0,
            last: bars.len() - 1,
            bars: &bars,
            price_h: 400.0,
            tz_offset_s: 0,
        };
        let couleur = |pm: &Pixmap, x: f64, y0: f64, y1: f64, c: Rgb| {
            (y0 as u32..y1 as u32).any(|y| {
                let p = pm.pixel(x.round() as u32, y).unwrap().demultiply();
                (p.red(), p.green(), p.blue()) == (c.0, c.1, c.2) && p.alpha() == 255
            })
        };
        let mut pm = crate::px::new(600, 400).unwrap();
        draw_ordres(&mut pm, &Text::default(), &g, &v);
        let (achat, vente) = (g.x(30), g.x(40));
        assert!(couleur(&pm, achat, g.y(99.0), g.y(99.0) + 20.0, ACHAT), "blue arrow under the buy bar");
        assert!(couleur(&pm, vente, g.y(101.0) - 20.0, g.y(101.0), VENTE), "pink arrow above the sell bar");
        assert!(!couleur(&pm, achat, g.y(101.0) - 20.0, g.y(101.0), VENTE) && !couleur(&pm, g.x(35), 0.0, 400.0, ACHAT));
        v.hidden = true;
        let mut vide = crate::px::new(600, 400).unwrap();
        draw_ordres(&mut vide, &Text::default(), &g, &v);
        assert!(vide.data().iter().all(|b| *b == 0), "hidden: nothing");
        assert_eq!(quantite(2.0), "2");
        assert_eq!(quantite(0.125), "0.125");
    }

    /// 🎨 Argos Vague turn 2 (R1): a `fill` whose colour is `na` on a bar paints nothing there: on those bars the
    /// band is the same as with no fill at all; on the other bars the fill is painted.
    #[test]
    fn fill_na_rien_sur_ces_bougies() {
        let bars: Vec<Bar> = (0..60)
            .map(|i| Bar {
                time: i * 60,
                open: 100.0,
                high: 101.0,
                low: 99.0,
                close: 100.0,
                volume: 1.0,
            })
            .collect();
        let vue = |couleur: &str| ScriptView {
            name: "f".into(),
            out: argos_script::lang::run(
                &format!("//@version=6\nindicator(\"f\", overlay = false)\na = plot(1, color = na)\nb = plot(-1, color = na)\nfill(a, b, color = {couleur})\n"),
                &bars,
                60,
                &HashMap::new(),
            )
            .unwrap(),
            ..Default::default()
        };
        let vp = Viewport {
            width: 600,
            height: 400,
            bar_spacing: 10.0,
            right_offset: 0.0,
            ..Default::default()
        };
        let slots = Slots::uniform(bars.len());
        let scale = PriceScale::fit(&bars, 0, bars.len() - 1, 200.0);
        let g = Geo {
            vp: &vp,
            scale: &scale,
            slots: &slots,
            first: 0,
            last: bars.len() - 1,
            bars: &bars,
            price_h: 200.0,
            tz_offset_s: 0,
        };
        let peint = |v: &ScriptView| {
            let mut pm = crate::px::new(600, 400).unwrap();
            draw(&mut pm, &Text::default(), &g, v, Some((200.0, 200.0)));
            pm
        };
        let (cond, rien) = (peint(&vue("bar_index % 2 == 0 ? color.red : na")), peint(&vue("na")));
        for i in 10..50usize {
            // the middle of the trapezoid from bar i - 1 to bar i, halfway between the two plots
            let x = ((g.x(i - 1) + g.x(i)) / 2.0).round() as u32;
            let pareil = cond.pixel(x, 300).unwrap() == rien.pixel(x, 300).unwrap();
            assert_eq!(pareil, i % 2 == 1, "bar {i}: fill only where its colour is not na");
        }
    }

    /// 📌 `force_overlay = true` (argos-vague-1): a band script's marks so marked are not drawn in its band but on the
    /// price by `draw_forced`; its other marks stay in the band; a label in `style_circle` is drawn as a dot.
    #[test]
    fn force_overlay_sur_le_prix() {
        let bars: Vec<Bar> = (0..200)
            .map(|i| {
                let c = 100.0 + (i as f64 / 9.0).sin() * 5.0;
                Bar {
                    time: i * 60,
                    open: c - 0.3,
                    high: c + 1.0,
                    low: c - 1.0,
                    close: c,
                    volume: 100.0,
                }
            })
            .collect();
        let tracer = |forcer: bool| {
            let f = if forcer { ", force_overlay = true" } else { "" };
            let src = format!(
                "//@version=6\nindicator(\"o\", overlay = false)\nplot(close - open)\nplotshape(bar_index % 20 == 0, style = shape.triangleup, location = location.belowbar, color = color.yellow, size = size.large{f})\nif barstate.islast\n    box.new(bar_index - 30, high, bar_index - 10, low, bgcolor = color.blue{f})\n    label.new(bar_index - 5, close, \"\", style = label.style_circle, color = color.red{f})\n"
            );
            ScriptView {
                name: "o".into(),
                out: argos_script::lang::run(&src, &bars, 60, &HashMap::new()).unwrap(),
                ..Default::default()
            }
        };
        let vp = Viewport {
            width: 800,
            height: 600,
            bar_spacing: 3.5,
            right_offset: 0.0,
            ..Default::default()
        };
        let slots = Slots::uniform(bars.len());
        let scale = PriceScale::fit(&bars, 0, bars.len() - 1, 400.0);
        let g = Geo {
            vp: &vp,
            scale: &scale,
            slots: &slots,
            first: 0,
            last: bars.len() - 1,
            bars: &bars,
            price_h: 400.0,
            tz_offset_s: 0,
        };
        let peint = |v: &ScriptView, sur_prix: bool| {
            let mut pm = crate::px::new(800, 600).unwrap();
            if sur_prix {
                draw_forced(&mut pm, &Text::default(), &g, v);
            } else {
                draw(&mut pm, &Text::default(), &g, v, Some((400.0, 200.0)));
            }
            pm
        };
        let compte = |pm: &Pixmap, y0: u32, y1: u32| {
            (y0..y1)
                .flat_map(|y| (0..800).map(move |x| (x, y)))
                .filter(|&(x, y)| pm.pixel(x, y).unwrap().alpha() > 0)
                .count()
        };
        let (forcee, normale) = (tracer(true), tracer(false));
        assert!(compte(&peint(&forcee, true), 0, 400) > 500, "the forced marks on the price");
        assert_eq!(compte(&peint(&normale, true), 0, 400), 0, "nothing on the price without force_overlay");
        // in the band: the same line, fewer marks once they go to the price
        let (bf, bn) = (peint(&forcee, false), peint(&normale, false));
        assert_ne!(bf.data(), bn.data(), "the forced marks left the band");
    }

    /// 📜 Argos Vague retouch 4: a script box is cut to its pane. Wholly above or below the visible prices it draws
    /// nothing (no band glued to the pane's edge, nothing in the pane below); crossing the top it is cut there; wholly
    /// inside it is drawn between its two prices; a box far above a band's values leaves the band untouched.
    #[test]
    fn boite_coupee_a_son_panneau() {
        let bars: Vec<Bar> = (0..200)
            .map(|i| {
                let c = 100.0 + (i as f64 / 9.0).sin() * 5.0;
                Bar {
                    time: i * 60,
                    open: c - 0.3,
                    high: c + 1.0,
                    low: c - 1.0,
                    close: c,
                    volume: 100.0,
                }
            })
            .collect();
        let vue = |boite: &str| {
            let src = format!("//@version=6\nindicator(\"o\", overlay = false)\nplot(close - open)\nif barstate.islast\n{boite}");
            ScriptView {
                name: "o".into(),
                out: argos_script::lang::run(&src, &bars, 60, &HashMap::new()).unwrap(),
                ..Default::default()
            }
        };
        let forcee = |haut: &str, bas: &str, bord: &str| {
            vue(&format!(
                "    box.new(bar_index, {haut}, bar_index + 1, {bas}, bgcolor = color.new(color.red, 65){bord}, extend = extend.right, force_overlay = true)\n"
            ))
        };
        let vp = Viewport {
            width: 800,
            height: 600,
            bar_spacing: 3.5,
            right_offset: 0.0,
            ..Default::default()
        };
        let slots = Slots::uniform(bars.len());
        let scale = PriceScale::fit(&bars, 0, bars.len() - 1, 400.0);
        let g = Geo {
            vp: &vp,
            scale: &scale,
            slots: &slots,
            first: 0,
            last: bars.len() - 1,
            bars: &bars,
            price_h: 400.0,
            tz_offset_s: 0,
        };
        let sur_prix = |v: &ScriptView| {
            let mut pm = crate::px::new(800, 600).unwrap();
            draw_forced(&mut pm, &Text::default(), &g, v);
            pm
        };
        // the painted rows (compared by their centre, y + 0.5, with the prices' y: the default border is anti-aliased)
        let lignes = |pm: &Pixmap| -> Vec<u32> { (0..600).filter(|&y| (0..800).any(|x| pm.pixel(x, y).unwrap().alpha() > 0)).collect() };
        let last = bars.last().unwrap();
        // wholly above the visible prices: nothing at all, with or without a border
        for bord in ["", ", border_color = color.red"] {
            assert!(
                lignes(&sur_prix(&forcee("high + 1000", "high + 990", bord))).is_empty(),
                "above ({bord}): nothing painted"
            );
            assert!(
                lignes(&sur_prix(&forcee("low - 990", "low - 1000", bord))).is_empty(),
                "below ({bord}): nothing painted"
            );
        }
        // crossing the top: from the pane's top down to the bottom price, nothing below it
        let p = last.low;
        let yb = g.y(p);
        let l = lignes(&sur_prix(&forcee("high + 1000", &format!("{p}"), "")));
        assert_eq!(l.first().copied(), Some(0), "cut at the pane's top");
        assert!(
            (*l.last().unwrap() as f64 + 0.5 - yb).abs() <= 1.0,
            "down to the bottom price: {:?} vs {yb}",
            l.last()
        );
        // wholly inside: between its two prices
        let l = lignes(&sur_prix(&forcee("high", "low", "")));
        let (yh, yl) = (g.y(last.high), g.y(last.low));
        assert!(!l.is_empty(), "a band inside is drawn");
        assert!((*l.first().unwrap() as f64 + 0.5 - yh).abs() <= 1.0, "top at the high: {:?} vs {yh}", l.first());
        assert!((*l.last().unwrap() as f64 + 0.5 - yl).abs() <= 1.0, "bottom at the low: {:?} vs {yl}", l.last());
        // in the band: a box far above the band's values changes no pixel
        let dans_bande = |v: &ScriptView| {
            let mut pm = crate::px::new(800, 600).unwrap();
            draw(&mut pm, &Text::default(), &g, v, Some((400.0, 200.0)));
            pm
        };
        let avec = vue("    box.new(bar_index, high + 1000, bar_index + 1, high + 990, bgcolor = color.new(color.red, 65), border_color = color.red, extend = extend.right)\n");
        let sans = vue("    x = 0\n");
        assert_eq!(dans_bande(&avec).data(), dans_bande(&sans).data(), "nothing of the box in the band");
    }

    /// 🎨 The ⚙ Style of the built-ins written in Argos Script: (a) nothing set — or its defaults written out, as a
    /// saved file gives them back — draws the SMA, the MA ribbon and Spring Tension pixel for pixel as the drawing with
    /// no style at all; (b) a colour, a width or a line type set on « MA » changes the drawn plot, and the script still
    /// runs with its inputs only.
    #[test]
    fn style_des_integres_en_script() {
        use crate::indicators::{compute, Ind, Settings};
        let bars: Vec<Bar> = (0..300)
            .map(|i| {
                let c = 100.0 + (i as f64 / 9.0).sin() * 5.0 + (i as f64 / 3.0).cos();
                Bar {
                    time: i * 60,
                    open: c - 0.3,
                    high: c + 1.0,
                    low: c - 1.0,
                    close: c,
                    volume: 50.0 + (i % 7) as f64 * 10.0,
                }
            })
            .collect();
        let vp = Viewport {
            width: 800,
            height: 600,
            bar_spacing: 2.5,
            right_offset: 0.0,
            ..Default::default()
        };
        let slots = Slots::uniform(bars.len());
        let scale = PriceScale::fit(&bars, 0, bars.len() - 1, 600.0);
        let g = Geo {
            vp: &vp,
            scale: &scale,
            slots: &slots,
            first: 0,
            last: bars.len() - 1,
            bars: &bars,
            price_h: 600.0,
            tz_offset_s: 0,
        };
        let image = |v: &ScriptView| {
            let mut pm = crate::px::new(800, 600).unwrap();
            crate::indicators::fit_bands(&Default::default(), 600.0);
            let band = (!v.out.overlay).then_some((400.0, crate::indicators::OSC_H));
            draw(&mut pm, &Text::default(), &g, v, band);
            pm.data().to_vec()
        };
        let vue = |i: Ind, set: &Settings| {
            let mut o = compute(&bars, 60, &[i], set, None, 0, &Default::default(), &[]);
            o.integres.remove(0).1
        };
        // (a) the script's own colours and widths
        for i in [Ind::Sma, Ind::MaRibbon, Ind::SpringTension, Ind::LuxAlgoSmc] {
            let v = vue(i, &Settings::new());
            assert!(v.error.is_none(), "{i:?}: {:?}", v.error);
            assert_eq!(v.style, argos_script::style_integres::Style::default(), "{i:?}: nothing set, the script's own");
            let sans_style = ScriptView {
                style: Default::default(),
                ..v.clone()
            };
            let mut set = Settings::new();
            set.insert(i, argos_script::inputs::defaults(i.params().1));
            let ecrits = vue(i, &set);
            assert_eq!(image(&v), image(&sans_style), "{i:?}: pixel for pixel");
            assert_eq!(image(&ecrits), image(&sans_style), "{i:?}: the defaults written out, pixel for pixel");
        }
        // (b) « MA · couleur », « MA · épaisseur », « MA · type de ligne » change the drawn plot
        let params = Ind::Sma.params().1;
        let base = vue(Ind::Sma, &Settings::new());
        for (cle, val) in [("MA · couleur", 0xff0000 as f64), ("MA · épaisseur", 4.0), ("MA · type de ligne", 2.0)] {
            let k = params.iter().position(|p| p.key == cle).unwrap_or_else(|| panic!("{cle}: in the ⚙"));
            let mut vals = argos_script::inputs::defaults(params);
            vals[k] = val;
            let mut set = Settings::new();
            set.insert(Ind::Sma, vals);
            let v = vue(Ind::Sma, &set);
            // (NaN at the warm-up: compared value by value, NaN equal to NaN)
            let memes = |a: &[f64], b: &[f64]| a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x == y || (x.is_nan() && y.is_nan()));
            assert!(
                v.out.plots.len() == base.out.plots.len() && v.out.plots.iter().zip(&base.out.plots).all(|(p, q)| memes(&p.values, &q.values)),
                "{cle}: the script runs with its inputs only"
            );
            assert_ne!(image(&v), image(&base), "{cle}: the plot changes");
        }
    }

    /// 🕯 B-033: a script's `plotcandle` / `plotbar` are drawn on the price (they were refused before): candles in their
    /// colour, OHLC bars too, and nothing on a bar whose colour is `na`.
    #[test]
    fn plotcandle_et_plotbar_dessines() {
        let bars: Vec<Bar> = (0..120)
            .map(|i| {
                let c = 100.0 + (i as f64 / 7.0).sin() * 5.0;
                Bar {
                    time: i * 60,
                    open: c - 0.8,
                    high: c + 1.5,
                    low: c - 1.5,
                    close: c,
                    volume: 100.0,
                }
            })
            .collect();
        let vp = Viewport {
            width: 800,
            height: 500,
            bar_spacing: 6.0,
            right_offset: 0.0,
            ..Default::default()
        };
        let slots = Slots::uniform(bars.len());
        let scale = PriceScale::fit(&bars, 0, bars.len() - 1, 400.0);
        let g = Geo {
            vp: &vp,
            scale: &scale,
            slots: &slots,
            first: 0,
            last: bars.len() - 1,
            bars: &bars,
            price_h: 400.0,
            tz_offset_s: 0,
        };
        let encre = |appel: &str| {
            let src = format!("//@version=6\nindicator(\"c\", overlay = true)\n{appel}\n");
            let v = ScriptView {
                name: "c".into(),
                out: argos_script::lang::run(&src, &bars, 60, &HashMap::new()).unwrap(),
                ..Default::default()
            };
            let mut pm = crate::px::new(800, 500).unwrap();
            draw(&mut pm, &Text::default(), &g, &v, None);
            (0..400)
                .flat_map(|y| (0..800).map(move |x| (x, y)))
                .filter(|&(x, y)| pm.pixel(x, y).unwrap().alpha() > 0)
                .count()
        };
        let bougies = encre("plotcandle(open, high, low, close, 'c', color.green)");
        assert!(bougies > 120 * 12, "a body and a wick on each of the 120 bars: {bougies} px");
        let barres = encre("plotbar(open, high, low, close, 'b', color.orange)");
        assert!(barres > 120 * 8, "a line and two ticks on each bar: {barres} px");
        assert!(barres < bougies, "an OHLC bar has no body");
        assert_eq!(encre("plotcandle(open, high, low, close, 'c', color = na)"), 0, "colour na: nothing drawn");
        let moitie = encre("plotcandle(open, high, low, close, 'c', bar_index % 2 == 0 ? na : color.green)");
        assert!(moitie > bougies / 3 && moitie < bougies * 2 / 3, "every other bar: {moitie} px of {bougies}");
    }
}
