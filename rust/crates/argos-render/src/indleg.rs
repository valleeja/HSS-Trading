//! 🏷️ Légende des indicateurs sur la chart, comme Argos v1 : sous la ligne OHLC, un bouton « ⌄ N » (nombre
//! d'indicateurs actifs) ouvre la liste des indicateurs sur le prix ; chaque ligne a les boutons de LuxAlgo : 👁 masquer,
//! ⚙️ réglages, ✥ déplacer, A assistant Argos (à la place de l'étincelle), ☆ favori, </> code, ✕ enlever. Un oscillateur a son nom et les mêmes boutons dans l'en-tête de sa bande. La flèche replie tout :
//! la liste et les en-têtes des bandes.
//! Une seule table de zones (`zones`) sert au dessin et au clic.

use crate::indicators::{legend_title, Ind, IndOut, BAND_FONT};
use crate::palette as pal;
use crate::palette::{GREY, WHITE};
use crate::shapes::{lines, round_fill, round_stroke, Rect};
use crate::text::{Align, Text};
use tiny_skia::Pixmap;

/// Ce qu'un clic dans la légende demande.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegClick {
    /// Ouvrir / replier la liste.
    Toggle,
    Eye(Ind),
    Settings(Ind),
    Move(Ind),
    Ai(Ind),
    Fav(Ind),
    Code(Ind),
    Remove(Ind),
    /// 🖥 ↑ ↓ : monter / descendre le panneau parmi les autres ; ▾ : le replier (22 de haut) / le rouvrir.
    Monter(Ind),
    Descendre(Ind),
    Replier(Ind),
    /// 📜 Script de la chart (index dans `IndOut::scripts`) : 👁, </> ouvrir dans l'éditeur, ✕ enlever de la chart.
    ScriptEye(usize),
    /// ⚙ The settings window made from the script's input() calls (step 3).
    ScriptSettings(usize),
    ScriptCode(usize),
    ScriptRemove(usize),
    /// 📱 Le nom d'un indicateur (cell) : sa barre d'actions s'ouvre (encore : elle se ferme).
    Nom(Ind),
    NomScript(usize),
    /// 📱 ⋯ de la barre d'actions : le menu complet de l'indicateur.
    Plus(Ind),
    PlusScript(usize),
    /// 🧱 Argos Murs « Cadrer sur les murs »: the price scale on the walls (±3 % around the last price), once.
    Cadrer(Ind),
}

/// 📱 Légende du cell (photos de Jee, TradingView mobile) : les noms seuls, en gras ; un nom touché devient une barre
/// d'actions 👁 ⚙ 🗑 ⋯ (fond de verre, fin bord bleu). `sel` : l'indicateur dont la barre est ouverte.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Cell {
    pub actif: bool,
    pub sel: Option<Result<Ind, usize>>,
    /// Largeur de la zone des prix (sans l'échelle) : la barre d'actions s'arrête avant l'échelle.
    pub pw: f64,
}

/// 📱 Taille du nom (photos) et hauteur de la barre d'actions.
const NOM_CELL: f32 = 17.0;
const BARRE_H: f64 = 48.0;
const PAS_CELL: f64 = 34.0;

fn nom_clic(r: Result<Ind, usize>) -> LegClick {
    match r {
        Ok(i) => LegClick::Nom(i),
        Err(k) => LegClick::NomScript(k),
    }
}

/// 📱 Les lignes de la légende du cell : (indicateur ou script, haut de sa ligne, nom) — les superposés sous ⌄ N, et
/// les indicateurs en panneau en haut de leur panneau.
fn lignes_cell(out: &IndOut, on: &[Ind], plot_h: f64) -> Vec<(Result<Ind, usize>, f64, String)> {
    let mut v: Vec<(Result<Ind, usize>, f64, String)> = rows(on, out)
        .into_iter()
        .enumerate()
        .map(|(k, r)| (r, top() + 30.0 + k as f64 * PAS_CELL, titre_cell(out, r)))
        .collect();
    v.extend(out.bands(plot_h).into_iter().map(|(i, top)| (Ok(i), top + 4.0, legend_title(i, out))));
    v.extend(out.script_bands(plot_h).into_iter().map(|(k, top)| (Err(k), top + 4.0, script_title(out, k))));
    v
}

/// 📱 La barre d'actions d'une ligne (haut `y`, largeur de la chart `pw`) : la barre, puis (bouton, action).
fn barre_cell(r: Result<Ind, usize>, y: f64, pw: f64) -> (Rect, Vec<(Rect, LegClick)>) {
    let w = (pw - 16.0).min(360.0);
    let b = Rect::new(8.0, y - (BARRE_H - 28.0) / 2.0, w, BARRE_H);
    let acts: Vec<LegClick> = match r {
        Ok(i) => {
            let mut a = vec![LegClick::Eye(i)];
            if !i.params().1.is_empty() {
                a.push(LegClick::Settings(i));
            }
            if i == Ind::ArgosMurs {
                a.push(LegClick::Cadrer(i));
            }
            a.extend([LegClick::Remove(i), LegClick::Plus(i)]);
            a
        }
        Err(k) => vec![LegClick::ScriptEye(k), LegClick::ScriptRemove(k), LegClick::PlusScript(k)],
    };
    let n = acts.len() as f64;
    let mut out = vec![(Rect::new(b.x, b.y, b.w - n * 46.0 - 6.0, b.h), nom_clic(r))];
    for (k, c) in acts.into_iter().enumerate() {
        out.push((Rect::new(b.x + b.w - (n - k as f64) * 46.0 - 4.0, b.y + 2.0, 44.0, b.h - 4.0), c));
    }
    (b, out)
}

/// 📱 Zones de la légende du cell.
fn zones_cell(_text: &Text, on: &[Ind], out: &IndOut, open: bool, plot_h: f64, pw: f64, cell: Cell) -> Vec<(Rect, LegClick)> {
    let gras = Text::graisse(crate::text::Graisse::Bold);
    let mut z = vec![(Rect::new(8.0, top(), 52.0, 22.0), LegClick::Toggle)];
    if !open {
        return z;
    }
    for (r, y, nom) in lignes_cell(out, on, plot_h) {
        if cell.sel == Some(r) {
            z.extend(barre_cell(r, y, pw).1);
        } else {
            let w = (gras.width(&nom, NOM_CELL) as f64 + 16.0).min(pw - 16.0);
            z.push((Rect::new(8.0, y - 2.0, w, 32.0), nom_clic(r)));
        }
    }
    z
}

/// Haut du bouton « ⌄ N » (sous le titre, la ligne OHLC et le volume).
/// Haut du bouton ⌄ (sous le titre et la légende O H L C · Vol), décalé avec la légende.
fn top() -> f64 {
    76.0 + crate::haut_legende()
}

/// 📱 The « ⌄ N » button of the phone's legend (chart coordinates): what sits on its line (the ⚡ Funding pill of a
/// perpetual) starts right of it.
pub fn bouton_cell() -> Rect {
    Rect::new(8.0, top(), 52.0, 22.0)
}
const ROW: f64 = 28.0;
const BTN: f64 = 24.0;
const NAME: f32 = 12.5;

/// Boutons d'un indicateur à partir de `x` (en `y`), dans l'ordre de LuxAlgo (⚙️ seulement s'il a des inputs).
fn buttons(ind: Ind, x: f64, y: f64, bande: bool) -> Vec<(Rect, LegClick)> {
    // maquette : 👁 ⚙ (↑ ↓ ▾ pour un panneau du bas) ✕ ; gardés en plus : ✨ Argos AI (comment le lire) et ⭐ favori
    let mut v = vec![LegClick::Eye(ind)];
    if !ind.params().1.is_empty() {
        v.push(LegClick::Settings(ind));
    }
    // 🧱 « Cadrer sur les murs »
    if ind == Ind::ArgosMurs {
        v.push(LegClick::Cadrer(ind));
    }
    // { } le code source (comme TradingView : 👁 ⚙ { }) : Pine, Argos Script ou (B1) son vrai code Rust
    if crate::sources::a_un_code(ind) {
        v.push(LegClick::Code(ind));
    }
    if bande {
        v.extend([LegClick::Monter(ind), LegClick::Descendre(ind), LegClick::Replier(ind)]);
    }
    v.extend([LegClick::Ai(ind), LegClick::Fav(ind), LegClick::Remove(ind)]);
    v.into_iter()
        .enumerate()
        .map(|(k, c)| (Rect::new(x + k as f64 * (BTN + 2.0), y, BTN, BTN), c))
        .collect()
}

/// Boutons d'un script (pas encore d'inputs ni de favori : 👁, </>, ✕).
fn script_buttons(k: usize, x: f64, y: f64) -> Vec<(Rect, LegClick)> {
    [
        LegClick::ScriptEye(k),
        LegClick::ScriptSettings(k),
        LegClick::ScriptCode(k),
        LegClick::ScriptRemove(k),
    ]
    .into_iter()
    .enumerate()
    .map(|(j, c)| (Rect::new(x + j as f64 * (BTN + 2.0), y, BTN, BTN), c))
    .collect()
}

/// Nom d'un script dans la légende (⚠ : la dernière version a une erreur, le dessin d'avant reste).
fn script_title(out: &IndOut, k: usize) -> String {
    let v = &out.scripts[k];
    match v.alerte() {
        Some(a) => format!("{} {a}", v.title()),
        None => v.title().to_string(),
    }
}

/// A script in error (or that draws nothing): its name in the legend's red.
fn script_en_erreur(out: &IndOut, r: Result<Ind, usize>) -> bool {
    r.is_err_and(|k| out.scripts.get(k).is_some_and(|v| v.error.is_some()))
}

/// Lignes de la liste ouverte : indicateurs sur le prix, puis scripts sur le prix (`Err` = index du script).
fn rows(on: &[Ind], out: &IndOut) -> Vec<Result<Ind, usize>> {
    let inds = Ind::ALL.iter().filter(|i| on.contains(i) && !i.oscillator()).map(|i| Ok(*i));
    inds.chain(out.scripts.iter().enumerate().filter(|(_, v)| v.out.overlay).map(|(k, _)| Err(k)))
        .collect()
}

/// 📱 Nom d'une ligne du cell : Argos Murs en court (« Argos Murs 4/5 · Perp + Compt. »).
fn titre_cell(out: &IndOut, r: Result<Ind, usize>) -> String {
    match (r, out.murs.as_ref()) {
        (Ok(Ind::ArgosMurs), Some(m)) => format!("{} {} · {}", Ind::ArgosMurs.name(), m.court, m.carnets_court),
        _ => row_title(out, r),
    }
}

/// 📱 Height of Argos Murs' glass pill in the phone's legend (inside its row of `PAS_CELL`).
const PILULE_CELL: f64 = 28.0;

/// 📱 Argos Murs' row on the phone: what follows the name (« 4/5 · Perp + Compt. »); `None`: not Argos Murs.
fn murs_cell(out: &IndOut, r: Result<Ind, usize>) -> Option<String> {
    match (r, out.murs.as_ref()) {
        (Ok(Ind::ArgosMurs), Some(m)) => Some(format!(" {} · {}", m.court, m.carnets_court)),
        _ => None,
    }
}

/// 🧱 The second pill after Argos Murs' row (mockup v5): « Carnets » and the books read; `None`: not Argos Murs.
fn carnets_pill(out: &IndOut, r: Result<Ind, usize>) -> Option<&str> {
    match (r, out.murs.as_ref()) {
        (Ok(Ind::ArgosMurs), Some(m)) => Some(m.carnets.as_str()),
        _ => None,
    }
}

/// 🏦 The multi-exchange footprint of a legend row (`None`: another row, or the footprint of one exchange).
fn fp_multi(out: &IndOut, r: Result<Ind, usize>) -> Option<&crate::fpmulti::FpMulti> {
    out.fp_multi
        .as_deref()
        .filter(|_| r == Ok(Ind::Footprint) && crate::indicators::vrais_trades_fp(out))
}

/// 🖥 Width of what follows a row's title (the exchanges' dots and « ■ réel » of the multi-exchange footprint).
fn suite_w(text: &Text, out: &IndOut, r: Result<Ind, usize>) -> f64 {
    fp_multi(out, r).map_or(0.0, |m| crate::fpmulti::legende_suite(text, m, true))
}

/// 🖥 Top of a price-pane indicator's row in the open list (chart coordinates); `None`: not in the list.
pub fn haut_ligne(out: &IndOut, ind: Ind) -> Option<f64> {
    rows(&out.on, out)
        .iter()
        .position(|r| *r == Ok(ind))
        .map(|k| top() + 28.0 + k as f64 * ROW - 1.0)
}

/// Nom d'une ligne.
fn row_title(out: &IndOut, r: Result<Ind, usize>) -> String {
    match r {
        Ok(i) => legend_title(i, out),
        Err(k) => script_title(out, k),
    }
}

/// Boutons d'une ligne à partir de `x`.
fn row_buttons(r: Result<Ind, usize>, x: f64, y: f64) -> Vec<(Rect, LegClick)> {
    match r {
        Ok(i) => buttons(i, x, y, false),
        Err(k) => script_buttons(k, x, y),
    }
}

/// Bas de la liste des indicateurs sur le prix une fois dépliée (PC, coordonnées de la chart) : ce qui se pose en haut
/// à gauche de la chart (carte Argos Lens) se met dessous, que la liste soit dépliée ou repliée.
pub fn bas_liste(out: &IndOut) -> f64 {
    top() + 28.0 + rows(&out.on, out).len() as f64 * ROW
}

/// Bottom of the legend drawn at the top left (closed: the ⌄ N button; open: its last row; 📱 `cell`: the phone's
/// rows, which span the whole width). Argos Murs keeps its pills under it on a phone.
pub fn bas_legende(out: &IndOut, open: bool, cell: bool) -> f64 {
    let n = rows(&out.on, out).len() as f64;
    if !open {
        top() + 22.0
    } else if cell {
        top() + 30.0 + n * PAS_CELL
    } else {
        top() + 28.0 + n * ROW
    }
}

/// 🖥 Haut et bas des lignes de la liste ouverte (coordonnées de la chart) ; `None` : aucune ligne.
pub fn lignes_liste(out: &IndOut) -> Option<(f64, f64)> {
    let n = rows(&out.on, out).len();
    (n > 0).then(|| (top() + 28.0, top() + 28.0 + n as f64 * ROW))
}

/// 🖥📟 The open list's rows whole (chart coordinates): from the chart's left edge to the end of the widest row's
/// buttons, the names included (`zones` gives only the buttons); `None`: no row. What lays itself over the chart
/// keeps clear of it: the tables (tableaux-1 tour 3, H1), the DÉMO pill.
pub fn liste_rect(text: &Text, on: &[Ind], out: &IndOut, plot_h: f64) -> Option<Rect> {
    let (haut, bas) = lignes_liste(out)?;
    let droite = zones(text, on, out, true, plot_h)
        .into_iter()
        .map(|(r, _)| r)
        .filter(|r| r.y >= haut - 1.0 && r.y < bas)
        .map(|r| r.x + r.w)
        .fold(0.0f64, f64::max);
    Some(Rect::new(0.0, haut, droite, bas - haut))
}

/// Zones cliquables (coordonnées de la chart) : le bouton, les lignes de la liste ouverte, les en-têtes des bandes.
/// `on` : indicateurs actifs (ordre du panneau) ; `plot_h` : hauteur de la zone de prix avec les bandes.
pub fn zones(text: &Text, on: &[Ind], out: &IndOut, open: bool, plot_h: f64) -> Vec<(Rect, LegClick)> {
    zones_de(text, on, out, open, plot_h, 0.0, Cell::default())
}

/// Zones (📱 `cell.actif` : la légende du cell, dans une chart de largeur `pw`).
pub fn zones_de(text: &Text, on: &[Ind], out: &IndOut, open: bool, plot_h: f64, pw: f64, cell: Cell) -> Vec<(Rect, LegClick)> {
    if on.is_empty() && out.scripts.is_empty() {
        return vec![];
    }
    if cell.actif {
        return zones_cell(text, on, out, open, plot_h, pw, cell);
    }
    let mut z = vec![(Rect::new(8.0, top(), 52.0, 22.0), LegClick::Toggle)];
    if open {
        for (k, r) in rows(on, out).into_iter().enumerate() {
            let y = top() + 28.0 + k as f64 * ROW;
            z.extend(row_buttons(
                r,
                18.0 + text.width(&row_title(out, r), NAME) as f64 + suite_w(text, out, r) + 10.0,
                y,
            ));
        }
        for (k, top) in out.script_bands(plot_h) {
            z.extend(script_buttons(k, 8.0 + text.width(&script_title(out, k), BAND_FONT) as f64 + 10.0, top + 3.0));
        }
    }
    // 🖥 panneaux du bas : leurs boutons, liste ouverte ou au survol du panneau (maquette)
    for (ind, top) in out.bands(plot_h) {
        if boutons_visibles(ind, open) {
            let x = match vague_pilule_w(text, ind, out) {
                // (inside the pill, right after its text: the pill grows around them)
                Some(w) => VAGUE_X + w,
                None => 8.0 + text.width(&legend_title(ind, out), BAND_FONT) as f64 + 10.0,
            };
            z.extend(buttons(ind, x, top + 3.0, true));
        }
    }
    z
}

/// 🌊 Left edge of Argos Vague's legend pill in its pane.
const VAGUE_X: f64 = 6.0;

/// 🌊 Argos Vague's legend pill (mockup v7): its parts (name, « <style> · <engine> », last value with its colour) and
/// width; `None` for another pane or before the first computation.
fn vague_parts(text: &Text, ind: Ind, out: &IndOut) -> Option<(String, String, Option<(String, crate::text::Rgb)>, f64)> {
    let l = out.vague.as_ref().filter(|_| ind == Ind::ArgosVague)?;
    let gras = Text::graisse(crate::text::Graisse::Bold);
    let v = crate::indicators::vague_valeur(out).map(|(x, c)| (crate::viewport::format_number(x, 1), c));
    let w = 10.0
        + gras.width(ind.name(), BAND_FONT) as f64
        + 8.0
        + text.width(l, BAND_FONT) as f64
        + v.as_ref().map_or(0.0, |(t, _)| 8.0 + gras.width(t, BAND_FONT) as f64)
        + 10.0;
    Some((ind.name().to_string(), l.clone(), v, w))
}

fn vague_pilule_w(text: &Text, ind: Ind, out: &IndOut) -> Option<f64> {
    vague_parts(text, ind, out).map(|p| p.3)
}

/// 🌊 Draws Argos Vague's legend as ONE glass pill at the top left of its pane: « Argos Vague » in bold, the style and
/// engine in grey, the last wave value in the wave's colour, then (when shown: list open or mouse on the pane) its
/// buttons inside the same pill, drawn like a legend row of the price pane (height `BTN + 2`, round ends, the same
/// glass, padding and spacing); `false`: not Argos Vague (drawn as text by the caller).
fn vague_pilule(pm: &mut Pixmap, text: &Text, ind: Ind, out: &IndOut, top: f64, avec_boutons: bool) -> bool {
    let Some((nom, l, v, w)) = vague_parts(text, ind, out) else {
        return false;
    };
    let gras = Text::graisse(crate::text::Graisse::Bold);
    let r = vague_rect(ind, w, top, avec_boutons);
    round_fill(pm, r, r.h / 2.0, pal::panel(), 0.85);
    round_stroke(pm, r, r.h / 2.0, pal::edge(), 1.0);
    let y = (r.y + 18.0) as f32;
    let mut x = r.x + 10.0;
    let attenue = out.hidden.contains(&ind);
    gras.draw(pm, &nom, (x as f32, y), BAND_FONT, if attenue { GREY } else { WHITE }, Align::Left);
    x += gras.width(&nom, BAND_FONT) as f64 + 8.0;
    text.draw(pm, &l, (x as f32, y), BAND_FONT, GREY, Align::Left);
    x += text.width(&l, BAND_FONT) as f64 + 8.0;
    if let Some((t, c)) = v {
        gras.draw(pm, &t, (x as f32, y), BAND_FONT, if attenue { GREY } else { c }, Align::Left);
    }
    true
}

/// 🌊 Argos Vague's pill (text width `w`): one pixel above its buttons (`top + 3`) and `BTN + 2` high, as the price
/// pane's legend rows; with its buttons it ends 8 px after the last one (same padding as those rows).
fn vague_rect(ind: Ind, w: f64, top: f64, avec_boutons: bool) -> Rect {
    let n = if avec_boutons { buttons(ind, 0.0, 0.0, true).len() as f64 } else { 0.0 };
    let extra = if n > 0.0 { n * (BTN + 2.0) + 8.0 } else { 0.0 };
    Rect::new(VAGUE_X, top + 2.0, w + extra, BTN + 2.0)
}

/// 🖥 Are a pane's buttons shown (list open, or the mouse on the pane)? Same rule for the drawing and the zones.
fn boutons_visibles(ind: Ind, open: bool) -> bool {
    open || crate::indicators::survol() == Some(ind)
}

/// 🖥 Le nom d'un panneau du bas est-il dessiné ? Liste ouverte : toujours ; fermée (⌄) : seulement au survol du panneau
/// ou s'il est replié.
pub fn nom_visible(i: Ind, open: bool) -> bool {
    open || i.replie() || crate::indicators::survol() == Some(i)
}

/// Dessine la légende dans l'image de la chart.
pub fn draw(pm: &mut Pixmap, text: &Text, on: &[Ind], out: &IndOut, open: bool, plot_h: f64, mouse: Option<(f64, f64)>) {
    draw_de(pm, text, on, out, open, plot_h, mouse, Cell::default())
}

/// Dessine la légende (📱 `cell.actif` : celle du cell — les noms en gras, la barre d'actions de l'indicateur choisi).
#[allow(clippy::too_many_arguments)]
pub fn draw_de(pm: &mut Pixmap, text: &Text, on: &[Ind], out: &IndOut, open: bool, plot_h: f64, mouse: Option<(f64, f64)>, cell: Cell) {
    if cell.actif {
        return draw_cell(pm, text, on, out, open, plot_h, cell);
    }
    let hover = |q: &Rect| mouse.is_some_and(|(x, y)| q.contains(x, y));
    // fond des lignes de la liste (nom + boutons), comme les pastilles de v1
    if open {
        let dim = |r: Result<Ind, usize>| match r {
            Ok(i) => out.hidden.contains(&i),
            Err(k) => out.scripts[k].hidden,
        };
        for (k, r) in rows(on, out).into_iter().enumerate() {
            let y = top() + 28.0 + k as f64 * ROW;
            let title = row_title(out, r);
            let w = text.width(&title, NAME) as f64 + suite_w(text, out, r) + 10.0 + row_buttons(r, 0.0, 0.0).len() as f64 * (BTN + 2.0) + 18.0;
            let q = Rect::new(8.0, y - 1.0, w, BTN + 2.0);
            round_fill(pm, q, q.h / 2.0, pal::panel(), 0.85);
            round_stroke(pm, q, q.h / 2.0, pal::edge(), 1.0);
            match carnets_pill(out, r) {
                // 🧱 mockup v5: the name in white, the exchanges in grey; then the « Carnets » pill
                Some(c) => {
                    let nom = Ind::ArgosMurs.name();
                    let reste = title.strip_prefix(nom).unwrap_or("");
                    // (hidden above 4 h: the whole legend in grey)
                    let gris = dim(r) || out.murs.as_ref().is_some_and(|m| m.cache);
                    text.draw(pm, nom, (18.0, (y + 17.0) as f32), NAME, if gris { GREY } else { WHITE }, Align::Left);
                    text.draw(
                        pm,
                        reste,
                        ((18.0 + text.width(nom, NAME) as f64) as f32, (y + 17.0) as f32),
                        NAME,
                        GREY,
                        Align::Left,
                    );
                    let (a, b) = ("Carnets", format!("  {c}"));
                    let cw = text.width(a, NAME) as f64 + text.width(&b, NAME) as f64 + 24.0;
                    let k = Rect::new(q.x + q.w + 8.0, q.y, cw, q.h);
                    round_fill(pm, k, k.h / 2.0, pal::panel(), 0.85);
                    round_stroke(pm, k, k.h / 2.0, pal::edge(), 1.0);
                    text.draw(pm, a, ((k.x + 12.0) as f32, (y + 17.0) as f32), NAME, WHITE, Align::Left);
                    text.draw(
                        pm,
                        &b,
                        ((k.x + 12.0 + text.width(a, NAME) as f64) as f32, (y + 17.0) as f32),
                        NAME,
                        GREY,
                        Align::Left,
                    );
                }
                None => match fp_multi(out, r) {
                    // 🏦 maquette v1 : le nom en blanc, « · 5 bourses » en gris, les points des bourses, « ■ réel »
                    Some(m) => {
                        let nom = Ind::Footprint.name();
                        let reste = title.strip_prefix(nom).unwrap_or("");
                        text.draw(pm, nom, (18.0, (y + 17.0) as f32), NAME, if dim(r) { GREY } else { WHITE }, Align::Left);
                        let x = 18.0 + text.width(nom, NAME) as f64;
                        text.draw(pm, reste, (x as f32, (y + 17.0) as f32), NAME, GREY, Align::Left);
                        let x = 18.0 + text.width(&title, NAME) as f64;
                        crate::fpmulti::legende_dessin(pm, text, m, x, y + 12.5, true);
                    }
                    None => text.draw(
                        pm,
                        &title,
                        (18.0, (y + 17.0) as f32),
                        NAME,
                        if script_en_erreur(out, r) {
                            crate::palette::down()
                        } else if dim(r) {
                            GREY
                        } else {
                            WHITE
                        },
                        Align::Left,
                    ),
                },
            }
        }
        // nom de chaque bande d'oscillateur et de script (ses boutons suivent), aux couleurs du thème (clair : foncé)
        let th = crate::Theme::current();
        for (ind, top) in out.bands(plot_h) {
            if vague_pilule(pm, text, ind, out, top, boutons_visibles(ind, open)) {
                continue;
            }
            text.draw(
                pm,
                &legend_title(ind, out),
                (8.0, (top + 17.0) as f32),
                BAND_FONT,
                if dim(Ok(ind)) { th.text } else { th.fg },
                Align::Left,
            );
        }
        for (k, top) in out.script_bands(plot_h) {
            text.draw(
                pm,
                &script_title(out, k),
                (8.0, (top + 17.0) as f32),
                BAND_FONT,
                if script_en_erreur(out, Err(k)) {
                    crate::palette::down()
                } else if dim(Err(k)) {
                    th.text
                } else {
                    th.fg
                },
                Align::Left,
            );
        }
    } else {
        // 🖥 liste fermée (⌄) : les noms des panneaux du bas se cachent aussi (demande de Jee) ; seulement celui sous la
        // souris (avec ses boutons) et ceux repliés (22 de haut : sinon on ne sait plus ce que c'est)
        let th = crate::Theme::current();
        for (ind, top) in out.bands(plot_h).into_iter().filter(|(i, _)| nom_visible(*i, false)) {
            if vague_pilule(pm, text, ind, out, top, boutons_visibles(ind, open)) {
                continue;
            }
            let c = if out.hidden.contains(&ind) { th.text } else { th.fg };
            text.draw(pm, &legend_title(ind, out), (8.0, (top + 17.0) as f32), BAND_FONT, c, Align::Left);
        }
    }
    for (q, c) in zones(text, on, out, open, plot_h) {
        if hover(&q) {
            round_fill(pm, q, q.h / 2.0, pal::hover(), 1.0);
        }
        let (cx, cy) = (q.cx(), q.cy());
        match c {
            LegClick::Toggle => {
                if !hover(&q) {
                    round_fill(pm, q, q.h / 2.0, pal::panel(), 0.85);
                }
                round_stroke(pm, q, q.h / 2.0, pal::edge(), 1.0);
                let chev: &[(f32, f32)] = if open {
                    &[(-4.0, 2.0), (0.0, -2.0), (4.0, 2.0)]
                } else {
                    &[(-4.0, -2.0), (0.0, 2.0), (4.0, -2.0)]
                };
                lines(pm, q.x + 14.0, cy, &[chev], WHITE, 1.5);
                text.draw(
                    pm,
                    &(on.len() + out.scripts.len()).to_string(),
                    ((q.x + 34.0) as f32, (cy + 5.0) as f32),
                    13.0,
                    WHITE,
                    Align::Center,
                );
            }
            LegClick::Eye(i) => crate::icons::draw(pm, if out.hidden.contains(&i) { "eyeoff2" } else { "eye2" }, cx, cy, 16, GREY),
            LegClick::Settings(_) => crate::icons::draw(pm, "gear", cx, cy, 16, GREY),
            LegClick::Move(_) => crate::icons::draw(pm, "move", cx, cy, 16, GREY),
            LegClick::Monter(_) => lines(pm, cx, cy, &[&[(-4.0, 1.5), (0.0, -2.5), (4.0, 1.5)], &[(0.0, -2.5), (0.0, 5.0)]], GREY, 1.5),
            LegClick::Descendre(_) => lines(pm, cx, cy, &[&[(-4.0, -1.5), (0.0, 2.5), (4.0, -1.5)], &[(0.0, 2.5), (0.0, -5.0)]], GREY, 1.5),
            LegClick::Replier(i) => {
                let chev: &[(f32, f32)] = if i.replie() {
                    &[(-2.0, -4.0), (2.0, 0.0), (-2.0, 4.0)]
                } else {
                    &[(-4.0, -2.0), (0.0, 2.0), (4.0, -2.0)]
                };
                lines(pm, cx, cy, &[chev], GREY, 1.6)
            }
            LegClick::Ai(_) => crate::icons::draw(pm, crate::icons::ARGOS_A, cx, cy, 18, GREY),
            LegClick::Fav(i) if out.favs.contains(&i) => crate::icons::draw(pm, "⭐", cx, cy, 18, GREY),
            LegClick::Fav(_) => crate::icons::draw(pm, "istar", cx, cy, 16, GREY),
            LegClick::Code(_) => crate::icons::draw(pm, "braces", cx, cy, 16, GREY),
            LegClick::Remove(_) | LegClick::ScriptRemove(_) => crate::icons::draw(pm, "close", cx, cy, 16, GREY),
            LegClick::ScriptEye(k) => crate::icons::draw(pm, if out.scripts[k].hidden { "eyeoff2" } else { "eye2" }, cx, cy, 16, GREY),
            LegClick::ScriptCode(_) => crate::icons::draw(pm, "code", cx, cy, 16, GREY),
            LegClick::ScriptSettings(_) => crate::icons::draw(pm, "gear", cx, cy, 16, GREY),
            LegClick::Cadrer(_) => cadrer_icone(pm, cx, cy, 1.0, GREY),
            // (📱 seulement dans la légende du cell)
            LegClick::Nom(_) | LegClick::NomScript(_) | LegClick::Plus(_) | LegClick::PlusScript(_) => {}
        }
    }
}

/// 📱 La légende du cell : ⌄ N, puis les noms en gras blanc (gris si caché) ; la ligne choisie devient une barre de
/// verre au bord bleu avec 👁 ⚙ 🗑 ⋯.
fn draw_cell(pm: &mut Pixmap, text: &Text, on: &[Ind], out: &IndOut, open: bool, plot_h: f64, cell: Cell) {
    let pw = if cell.pw > 0.0 { cell.pw } else { pm.width() as f64 / crate::px::dev(1.0) };
    let gras = Text::graisse(crate::text::Graisse::Bold);
    let dim = |r: Result<Ind, usize>| match r {
        Ok(i) => out.hidden.contains(&i),
        Err(k) => out.scripts[k].hidden,
    };
    // ⌄ N
    let q = bouton_cell();
    // (pilule à bouts ronds, comme les pastilles de la maquette)
    round_fill(pm, q, q.h / 2.0, pal::panel(), 0.85);
    round_stroke(pm, q, q.h / 2.0, pal::edge(), 1.0);
    let chev: &[(f32, f32)] = if open {
        &[(-4.0, 2.0), (0.0, -2.0), (4.0, 2.0)]
    } else {
        &[(-4.0, -2.0), (0.0, 2.0), (4.0, -2.0)]
    };
    lines(pm, q.x + 14.0, q.cy(), &[chev], WHITE, 1.5);
    text.draw(
        pm,
        &(on.len() + out.scripts.len()).to_string(),
        ((q.x + 34.0) as f32, (q.cy() + 5.0) as f32),
        13.0,
        WHITE,
        Align::Center,
    );
    if !open {
        return;
    }
    for (r, y, nom) in lignes_cell(out, on, plot_h) {
        let c = if script_en_erreur(out, r) {
            crate::palette::down()
        } else if dim(r) {
            GREY
        } else {
            WHITE
        };
        if cell.sel == Some(r) {
            let (b, btns) = barre_cell(r, y, pw);
            crate::jetons::verre(pm, b, 10.0);
            round_stroke(pm, b, 10.0, crate::jetons::ACCENT, 1.4);
            let room = btns[0].0.w - 16.0;
            gras.draw(
                pm,
                &gras.fit(&nom, NOM_CELL, room),
                ((b.x + 12.0) as f32, (b.cy() + 6.0) as f32),
                NOM_CELL,
                c,
                Align::Left,
            );
            for (q, a) in btns.into_iter().skip(1) {
                let (cx, cy) = (q.cx(), q.cy());
                match a {
                    LegClick::Eye(i) => crate::icons::draw(pm, if out.hidden.contains(&i) { "eyeoff2" } else { "eye2" }, cx, cy, 22, WHITE),
                    LegClick::ScriptEye(k) => crate::icons::draw(pm, if out.scripts[k].hidden { "eyeoff2" } else { "eye2" }, cx, cy, 22, WHITE),
                    LegClick::Settings(_) => crate::icons::draw(pm, "gear", cx, cy, 22, WHITE),
                    LegClick::Cadrer(_) => cadrer_icone(pm, cx, cy, 1.4, WHITE),
                    LegClick::Remove(_) | LegClick::ScriptRemove(_) => {
                        crate::phosphor::draw(pm, "trash", cx, cy, 22, WHITE);
                    }
                    _ => {
                        for d in [-8.0, 0.0, 8.0] {
                            round_fill(pm, Rect::new(cx + d - 2.0, cy - 2.0, 4.0, 4.0), 2.0, WHITE, 1.0);
                        }
                    }
                }
            }
        } else if let Some(reste) = murs_cell(out, r) {
            // 🧱 Jee's rule: every element on the chart has its own glass pill, nothing bare on the price lines; the
            // name in bold, the count and the books in grey, normal weight (as the other legend pills)
            let nom = Ind::ArgosMurs.name();
            let wn = gras.width(nom, NOM_CELL) as f64;
            let reste = text.fit(&reste, NOM_CELL, (pw - 16.0 - 24.0 - wn).max(0.0));
            let q = Rect::new(8.0, y, (wn + text.width(&reste, NOM_CELL) as f64 + 24.0).min(pw - 16.0), PILULE_CELL);
            round_fill(pm, q, q.h / 2.0, pal::panel(), 0.85);
            round_stroke(pm, q, q.h / 2.0, pal::edge(), 1.0);
            let base = (q.cy() + 6.0) as f32;
            let c = if out.murs.as_ref().is_some_and(|m| m.cache) { GREY } else { c };
            gras.draw(pm, nom, ((q.x + 12.0) as f32, base), NOM_CELL, c, Align::Left);
            text.draw(pm, &reste, ((q.x + 12.0 + wn) as f32, base), NOM_CELL, GREY, Align::Left);
        } else if let Some(m) = fp_multi(out, r) {
            // 🏦 maquette v1 (cell) : une pilule de verre, le nom en gras, « · 5 bourses » en gris, les points des bourses
            let suite = crate::fpmulti::legende_suite(text, m, false);
            let reste = format!(" · {}", m.nombre());
            let place = pw - 16.0 - 24.0 - suite - text.width(&reste, NOM_CELL) as f64;
            // (the phone's width: « Footprint » alone when « Argos Footprint » leaves no room, as the mock-up)
            let nom = Some(Ind::Footprint.name())
                .filter(|n| gras.width(n, NOM_CELL) as f64 <= place)
                .unwrap_or("Footprint");
            let wn = gras.width(nom, NOM_CELL) as f64;
            let reste = text.fit(&reste, NOM_CELL, (pw - 16.0 - 24.0 - wn - suite).max(0.0));
            let q = Rect::new(8.0, y, (wn + text.width(&reste, NOM_CELL) as f64 + suite + 24.0).min(pw - 16.0), PILULE_CELL);
            round_fill(pm, q, q.h / 2.0, pal::panel(), 0.85);
            round_stroke(pm, q, q.h / 2.0, pal::edge(), 1.0);
            let base = (q.cy() + 6.0) as f32;
            gras.draw(pm, nom, ((q.x + 12.0) as f32, base), NOM_CELL, c, Align::Left);
            let x = q.x + 12.0 + wn;
            text.draw(pm, &reste, (x as f32, base), NOM_CELL, GREY, Align::Left);
            crate::fpmulti::legende_dessin(pm, text, m, x + text.width(&reste, NOM_CELL) as f64, q.cy(), false);
        } else {
            gras.draw(pm, &gras.fit(&nom, NOM_CELL, pw - 24.0), (8.0, (y + 20.0) as f32), NOM_CELL, c, Align::Left);
        }
    }
}

/// 🧱 « Cadrer sur les murs »: two price lines and a double arrow between them (the scale fitted on the walls).
fn cadrer_icone(pm: &mut Pixmap, cx: f64, cy: f64, k: f32, c: crate::text::Rgb) {
    let p = |x: f32, y: f32| (x * k, y * k);
    lines(
        pm,
        cx,
        cy,
        &[
            &[p(-6.0, -6.0), p(6.0, -6.0)],
            &[p(-6.0, 6.0), p(6.0, 6.0)],
            &[p(0.0, -3.5), p(0.0, 3.5)],
            &[p(-2.5, -1.0), p(0.0, -3.5), p(2.5, -1.0)],
            &[p(-2.5, 1.0), p(0.0, 3.5), p(2.5, 1.0)],
        ],
        c,
        1.4,
    );
}

/// 📱 Smallest touch zone height on the screen, in points (the drawing may be smaller).
const TOUCHER_MIN: f64 = 32.0;

/// Tolérance au doigt (📱) : un toucher jusqu'à cette distance d'un bouton de 24 px le prend (le plus proche).
const DOIGT: f64 = 12.0;

/// Clic en `(x, y)` (coordonnées de la chart) ; `None` : à côté. Sur un bouton : lui ; sinon le bouton le plus proche à
/// moins de `DOIGT` (un doigt couvre plus qu'un bouton de 24 px, Jee 2026-11-09 : ⚙ ne répondait pas au toucher).
pub fn click(text: &Text, on: &[Ind], out: &IndOut, open: bool, plot_h: f64, x: f64, y: f64) -> Option<LegClick> {
    click_de(text, on, out, open, plot_h, 0.0, Cell::default(), x, y)
}

/// Clic (📱 avec la légende du cell).
#[allow(clippy::too_many_arguments)]
pub fn click_de(text: &Text, on: &[Ind], out: &IndOut, open: bool, plot_h: f64, pw: f64, cell: Cell, x: f64, y: f64) -> Option<LegClick> {
    let z = zones_de(text, on, out, open, plot_h, pw, cell);
    if let Some((_, c)) = z.iter().find(|(q, _)| q.contains(x, y)) {
        return Some(*c);
    }
    // 📱 (compact interface) touch zones at least 32 points high on the screen, the drawing unchanged: each zone
    // grows to that height around its middle; where two grown zones meet, the nearest middle wins
    if cell.actif {
        let min = TOUCHER_MIN / crate::formfactor::COMPACTE_CELL;
        let pres = z
            .iter()
            .filter(|(q, _)| q.h < min && x >= q.x && x <= q.x + q.w && (y - q.cy()).abs() <= min / 2.0)
            .min_by(|a, b| (y - a.0.cy()).abs().total_cmp(&(y - b.0.cy()).abs()));
        if let Some((_, c)) = pres {
            return Some(*c);
        }
    }
    let dist = |q: &Rect| (x - x.clamp(q.x, q.x + q.w)).hypot(y - y.clamp(q.y, q.y + q.h));
    z.iter()
        .map(|(q, c)| (dist(q), *c))
        .filter(|(d, _)| *d <= DOIGT)
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, c)| c)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indicators::{compute, Settings};
    use argos_core::Bar;

    /// 📱 Compact interface (80 %): on the phone, the legend's touch zones are at least 32 screen points high (40
    /// layout points) even when drawn smaller (⌄ N is 22 high); the nearest wins between two; the drawing unchanged.
    #[test]
    fn zones_a_toucher_du_cell_au_moins_32_points() {
        let t = Text::default();
        let cell = Cell {
            actif: true,
            sel: None,
            pw: 390.0,
        };
        let on = [Ind::Rsi];
        let out = IndOut::default();
        let z = zones_de(&t, &on, &out, false, 600.0, 390.0, cell);
        let (q, c) = z[0];
        assert_eq!(c, LegClick::Toggle);
        assert!(q.h < 32.0, "drawn smaller");
        let min = TOUCHER_MIN / crate::formfactor::COMPACTE_CELL;
        assert!(min * crate::formfactor::COMPACTE_CELL >= 32.0 - 1e-9);
        for dy in [-(min / 2.0) + 0.5, min / 2.0 - 0.5] {
            assert_eq!(
                click_de(&t, &on, &out, false, 600.0, 390.0, cell, q.cx(), q.cy() + dy),
                Some(LegClick::Toggle),
                "{dy}"
            );
        }
        assert_eq!(
            click_de(&t, &on, &out, false, 600.0, 390.0, cell, q.cx() + 200.0, q.cy() + min / 2.0 + 30.0),
            None
        );
    }

    #[test]
    fn legende() {
        let bars: Vec<Bar> = (0..300)
            .map(|i| {
                let c = 100.0 + (i as f64 / 9.0).sin() * 5.0;
                Bar {
                    time: i * 60,
                    open: c - 0.3,
                    high: c + 1.0,
                    low: c - 1.0,
                    close: c,
                    volume: 100.0 + (i % 7) as f64 * 40.0,
                }
            })
            .collect();
        let on = [Ind::Tvd, Ind::Rsi];
        let out = compute(&bars, 60, &on, &Settings::new(), None, 0, &Default::default(), &[]);
        let t = Text::default();
        // repliée : seulement le bouton (les en-têtes des bandes sont repliés aussi)
        let z = zones(&t, &on, &out, false, 600.0);
        assert_eq!(z.len(), 1);
        assert_eq!(z[0].1, LegClick::Toggle);
        // ouverte : les boutons de la bande du RSI (et son ⚙️ : il a ses réglages depuis le 9 nov. 2026)
        let z = zones(&t, &on, &out, true, 600.0);
        assert_eq!(
            z.iter()
                .filter(|(_, c)| matches!(c, LegClick::Eye(Ind::Rsi) | LegClick::Remove(Ind::Rsi)))
                .count(),
            2
        );
        assert!(z.iter().any(|(_, c)| *c == LegClick::Settings(Ind::Rsi)), "⚙️ du RSI");
        // ouverte : TVD Pro (sur le prix) : 👁 ⚙ ✕ de la maquette, plus ✨ Argos AI et ⭐ favori (plus de ✥ ni de </>)
        assert_eq!(
            z.iter()
                .filter(|(_, c)| matches!(
                    c,
                    LegClick::Eye(Ind::Tvd) | LegClick::Settings(Ind::Tvd) | LegClick::Ai(Ind::Tvd) | LegClick::Fav(Ind::Tvd) | LegClick::Remove(Ind::Tvd)
                ))
                .count(),
            5
        );
        let z = zones(&t, &on, &out, true, 600.0);
        let gear = z.iter().find(|(_, c)| *c == LegClick::Settings(Ind::Tvd)).expect("⚙️ de TVD Pro").0;
        assert_eq!(click(&t, &on, &out, true, 600.0, gear.cx(), gear.cy()), Some(LegClick::Settings(Ind::Tvd)));
        assert_eq!(click(&t, &on, &out, true, 600.0, 500.0, 300.0), None);
        let mut pm = crate::px::new(800, 600).unwrap();
        draw(&mut pm, &t, &on, &out, true, 600.0, None);
        // 📜 scripts : un sur le prix (ligne de la liste), un dans sa bande (en-tête), avec 👁 </> ✕
        let mut out = compute(&bars, 60, &[], &Settings::new(), None, 0, &Default::default(), &[]);
        for (k, ov) in [true, false].into_iter().enumerate() {
            let src = format!("//@version=6\nindicator(\"S{k}\", overlay = {ov})\nplot(close)");
            let o = argos_script::lang::run(&src, &bars, 60, &Default::default()).unwrap();
            out.scripts.push(crate::scriptview::ScriptView {
                name: format!("s{k}"),
                out: o,
                ..Default::default()
            });
        }
        let z = zones(&t, &[], &out, true, 600.0);
        assert!(z.iter().any(|(_, c)| *c == LegClick::ScriptCode(0)) && z.iter().any(|(_, c)| *c == LegClick::ScriptRemove(1)));
        let band = z.iter().find(|(_, c)| *c == LegClick::ScriptEye(1)).unwrap().0;
        assert!(band.y > 400.0, "boutons du script sans overlay dans sa bande");
    }

    /// 🌊 Argos Vague (retouch 3): ONE glass pill holds the text and every button of the pane, in the app's usual
    /// order, as a legend row of the price pane (height `BTN + 2`, buttons one pixel under its top, 8 px after the
    /// last one); closed list, mouse elsewhere: the pill holds the text only.
    #[test]
    fn argos_vague_une_pilule_avec_ses_boutons() {
        let bars: Vec<Bar> = (0..400)
            .map(|i| {
                let c = 100.0 + (i as f64 / 9.0).sin() * 5.0;
                Bar {
                    time: i * 1800,
                    open: c - 0.3,
                    high: c + 1.0,
                    low: c - 1.0,
                    close: c,
                    volume: 100.0 + (i % 7) as f64 * 40.0,
                }
            })
            .collect();
        let on = [Ind::ArgosVague];
        let out = compute(&bars, 1800, &on, &Settings::new(), None, 0, &Default::default(), &[]);
        let t = Text::default();
        let (ind, top) = out.bands(700.0)[0];
        assert_eq!(ind, Ind::ArgosVague);
        let w = vague_pilule_w(&t, ind, &out).expect("a pill");
        let z: Vec<(Rect, LegClick)> = zones(&t, &on, &out, true, 700.0).into_iter().filter(|(q, _)| q.y > top).collect();
        let ordre: Vec<LegClick> = z.iter().map(|(_, c)| *c).collect();
        assert_eq!(
            ordre,
            buttons(ind, 0.0, 0.0, true).into_iter().map(|(_, c)| c).collect::<Vec<_>>(),
            "none removed"
        );
        let p = vague_rect(ind, w, top, true);
        for (q, c) in &z {
            assert!(
                q.x >= p.x && q.x + q.w <= p.x + p.w && q.y >= p.y && q.y + q.h <= p.y + p.h,
                "{c:?} inside the pill"
            );
        }
        assert_eq!(z[0].0.x, VAGUE_X + w, "the first button 10 px after the text, as the price pane's rows");
        assert_eq!(p.y + 1.0, z[0].0.y);
        assert_eq!(p.h, BTN + 2.0);
        let last = z.last().unwrap().0;
        assert_eq!(p.x + p.w, last.x + last.w + 2.0 + 8.0);
        // closed list, mouse elsewhere: no button, the pill around the text alone
        assert!(zones(&t, &on, &out, false, 700.0).iter().all(|(q, _)| q.y < top));
        assert_eq!(vague_rect(ind, w, top, false).w, w);
        // a click on the pill's buttons answers them
        let g = z.iter().find(|(_, c)| *c == LegClick::Settings(ind)).unwrap().0;
        assert_eq!(click(&t, &on, &out, true, 700.0, g.cx(), g.cy()), Some(LegClick::Settings(ind)));
        let mut pm = crate::px::new(900, 700).unwrap();
        draw(&mut pm, &t, &on, &out, true, 700.0, None);
    }

    /// 📱 Un doigt posé un peu à côté de ⚙ (moins de 12 px) le prend quand même ; plus loin : rien.
    #[test]
    fn doigt_un_peu_a_cote() {
        let t = Text::default();
        let bars: Vec<argos_core::Bar> = (0..120)
            .map(|i| argos_core::Bar {
                time: i * 60,
                open: 100.0 + i as f64,
                high: 102.0 + i as f64,
                low: 99.0 + i as f64,
                close: 101.0 + i as f64,
                volume: 5.0,
            })
            .collect();
        let on = [Ind::Tvd];
        let out = compute(&bars, 60, &on, &Settings::new(), None, 0, &Default::default(), &[]);
        let z = zones(&t, &on, &out, true, 600.0);
        let g = z.iter().find(|(_, c)| *c == LegClick::Settings(Ind::Tvd)).unwrap().0;
        assert_eq!(click(&t, &on, &out, true, 600.0, g.cx(), g.y + g.h + 8.0), Some(LegClick::Settings(Ind::Tvd)));
        assert_eq!(
            click(&t, &on, &out, true, 600.0, g.cx(), g.y + g.h + 30.0).filter(|c| *c == LegClick::Settings(Ind::Tvd)),
            None
        );
    }
}
