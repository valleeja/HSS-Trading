//! 📋 Movable and resizable tables (tableaux-1, Jee 22 Nov): every table on the chart — the indicators' tables, the
//! multi-UT table, the Vector Zone watchlist, user scripts' and imported Pine `table.new` tables — goes through one
//! path ([`crate::indicators::placer`]) that reads its [`TableLayout`]. Moved by the body, scaled by a corner (around
//! the opposite corner), stretched by a side; double-click (or « ↺ Remettre à sa place ») puts it back. The app keeps
//! one layout per table key (`indicator_tables.json`) and gives them with [`set_layouts`] before each drawing; what
//! was placed is read back with [`poses`]. (tour 3, Jee 23 Nov: « le rebord ne change pas, juste un peu plus
//! visible ») Nothing is drawn around a table under the mouse or the finger: its own border just gets brighter, and
//! the mouse cursor tells what a press takes. The mouse layer ([`dessiner_dessus`], from `finir`) draws that border,
//! the touch hint pill and the right-click menu over the chart.

use crate::indicators::Ind;
use crate::shapes::Rect;
use crate::text::{Align, Rgb, Text};
use serde::{Deserialize, Serialize};
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use tiny_skia::Pixmap;

/// Bounds of the whole-table scale (corner handles, pinch).
pub const SCALE_MIN: f64 = 0.6;
pub const SCALE_MAX: f64 = 2.0;
/// Bounds of the width stretch (left / right side) and of the row height stretch (top / bottom side).
pub const ETIRE_MIN: f64 = 0.8;
pub const ETIRE_MAX: f64 = 2.0;
/// Blue of the hint pill's « OK » (mockup v1).
pub const BLEU: Rgb = Rgb(0x4d, 0xa3, 0xff);
/// Gap kept around a table (the free places, the mouse's reach outside its edge).
pub const MARGE: f64 = 4.0;
/// 🖱 The mouse's zones of a table's edge: the band 2 px inside to `BORD_SOURIS` − 2 px outside its edge stretches,
/// its corners (`BORD_SOURIS` square around them) scale. Nothing drawn: the cursor shows them.
pub const BORD_SOURIS: f64 = 12.0;
/// 👆 A drag starting within `COIN_DOIGT` points of a corner (inside, or a little outside) scales the table.
pub const COIN_DOIGT: f64 = 24.0;
/// 👆 The finger's target of a button (« OK ») and the reach of a pinch around the table.
pub const CIBLE_DOIGT: f64 = 40.0;
/// The table's border: its alpha over the table's own (`edge_soft`) while it is under the mouse or the finger.
pub const BORD_ACTIF: f32 = 0.28;

/// Where a table is, chosen by the user: `moved` false = its automatic place (its corner, the walls' mirror rule…).
/// Once moved, `dx`, `dy` are the offsets of the table's corner `corner` (0 top left, 1 top right, 2 bottom right,
/// 3 bottom left) from the same corner of its area (px of the price pane): it keeps its place when the window or the
/// orientation changes. `scale` scales everything (text and cells), `wx` stretches the width, `hy` the row height.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TableLayout {
    pub moved: bool,
    pub corner: u8,
    pub dx: f64,
    pub dy: f64,
    pub scale: f64,
    pub wx: f64,
    pub hy: f64,
}

impl Default for TableLayout {
    fn default() -> Self {
        Self {
            moved: false,
            corner: 0,
            dx: 0.0,
            dy: 0.0,
            scale: 1.0,
            wx: 1.0,
            hy: 1.0,
        }
    }
}

impl TableLayout {
    /// The layout with its sizes in their bounds (and no NaN from an old or hand-edited file).
    pub fn borne(self) -> Self {
        let f = |v: f64, lo: f64, hi: f64| if v.is_finite() { v.clamp(lo, hi) } else { 1.0 };
        let o = |v: f64| if v.is_finite() { v } else { 0.0 };
        Self {
            corner: self.corner.min(3),
            dx: o(self.dx),
            dy: o(self.dy),
            scale: f(self.scale, SCALE_MIN, SCALE_MAX),
            wx: f(self.wx, ETIRE_MIN, ETIRE_MAX),
            hy: f(self.hy, ETIRE_MIN, ETIRE_MAX),
            ..self
        }
    }

    /// Where a moved `w` × `h` table goes in `area` (kept inside it).
    pub fn place(&self, area: Rect, w: f64, h: f64) -> Rect {
        let (droite, bas) = (matches!(self.corner, 1 | 2), self.corner >= 2);
        let x = if droite { area.x + area.w - self.dx - w } else { area.x + self.dx };
        let y = if bas { area.y + area.h - self.dy - h } else { area.y + self.dy };
        dans(Rect::new(x, y, w, h), area)
    }

    /// The layout that puts the table at `r` in `area`, its corner `coin` held (`None`: the area's corner nearest
    /// the table's centre); the sizes stay.
    pub fn ancre(self, r: Rect, area: Rect, coin: Option<u8>) -> Self {
        let coin = coin.unwrap_or(match (r.cx() > area.cx(), r.cy() > area.cy()) {
            (false, false) => 0,
            (true, false) => 1,
            (true, true) => 2,
            (false, true) => 3,
        });
        let (droite, bas) = (matches!(coin, 1 | 2), coin >= 2);
        Self {
            moved: true,
            corner: coin,
            dx: if droite { area.x + area.w - (r.x + r.w) } else { r.x - area.x },
            dy: if bas { area.y + area.h - (r.y + r.h) } else { r.y - area.y },
            ..self
        }
    }
}

/// `r` moved inside `area` (its top left kept in when it is larger).
pub fn dans(mut r: Rect, area: Rect) -> Rect {
    r.x = r.x.min(area.x + area.w - r.w).max(area.x);
    r.y = r.y.min(area.y + area.h - r.h).max(area.y);
    r
}

/// The two rects overlap (touching edges do not).
pub fn croise(a: &Rect, b: &Rect) -> bool {
    a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h
}

/// 📐 The free place nearest `r` (same size) inside `area` that overlaps none of `obst`; `r` itself (kept inside
/// `area`) when it is already free; `None` when no place fits. The nearest free place always has its x at `r`'s, at
/// an edge of the area or just beside an obstacle (and the same for y): those are the places tried, plus a grid.
pub fn libre(r: Rect, area: Rect, obst: &[Rect]) -> Option<Rect> {
    let r0 = dans(r, area);
    let libre_ici = |c: &Rect| {
        c.x >= area.x - 0.01
            && c.y >= area.y - 0.01
            && c.x + c.w <= area.x + area.w + 0.01
            && c.y + c.h <= area.y + area.h + 0.01
            && !obst.iter().any(|o| croise(o, c))
    };
    if libre_ici(&r0) {
        return Some(r0);
    }
    if r.w > area.w || r.h > area.h {
        return None;
    }
    let mut xs = vec![r0.x, area.x, area.x + area.w - r.w];
    let mut ys = vec![r0.y, area.y, area.y + area.h - r.h];
    for o in obst {
        xs.extend([o.x - r.w - MARGE, o.x + o.w + MARGE]);
        ys.extend([o.y - r.h - MARGE, o.y + o.h + MARGE]);
    }
    let pas = 16.0;
    let (nx, ny) = (((area.w - r.w) / pas) as usize, ((area.h - r.h) / pas) as usize);
    xs.extend((0..=nx).map(|k| area.x + k as f64 * pas));
    ys.extend((0..=ny).map(|k| area.y + k as f64 * pas));
    let mut best: Option<(f64, Rect)> = None;
    for &x in &xs {
        for &y in &ys {
            let c = Rect::new(x, y, r.w, r.h);
            let d = (x - r0.x).hypot(y - r0.y);
            if best.is_none_or(|(bd, _)| d < bd) && libre_ici(&c) {
                best = Some((d, c));
            }
        }
    }
    best.map(|(_, c)| c)
}

/// A table placed in the last drawing: its key (the layout's), the name shown in its menu, its rect and its area
/// (chart coordinates), the scale it was drawn at, and the indicator setting that shows it (« Masquer le tableau »).
#[derive(Debug, Clone, PartialEq)]
pub struct Pose {
    pub key: String,
    pub titre: String,
    pub rect: Rect,
    pub area: Rect,
    pub scale: f64,
    pub ind: Option<Ind>,
    pub interrupteur: Option<&'static str>,
}

/// What a table is, given by whoever draws it.
#[derive(Debug, Clone, PartialEq)]
pub struct Id {
    pub key: String,
    pub titre: String,
    pub ind: Option<Ind>,
    pub interrupteur: Option<&'static str>,
}

impl Id {
    /// A native indicator's table (`part`: « g » for the Cipher's Golden Dot table, else empty).
    pub fn native(ind: Ind, part: &str, interrupteur: Option<&'static str>) -> Self {
        Self {
            key: cle(ind, part),
            titre: ind.name().to_string(),
            ind: Some(ind),
            interrupteur,
        }
    }

    /// The `k`-th `table.new` table of the user script `id` (named `nom`).
    pub fn script(id: &str, k: usize, nom: &str) -> Self {
        Self {
            key: format!("script:{id}:{k}"),
            titre: nom.to_string(),
            ind: None,
            interrupteur: None,
        }
    }
}

/// The layout key of an indicator's table.
pub fn cle(ind: Ind, part: &str) -> String {
    format!("{ind:?}{part}")
}

/// The layout keys that belong to an indicator (its tables): forgotten when it is removed.
pub fn est_de(key: &str, ind: Ind) -> bool {
    let base = format!("{ind:?}");
    key == base || (key.starts_with(&base) && key[base.len()..].chars().all(|c| c.is_ascii_lowercase()))
}

/// The layout keys that belong to the user script `id`.
pub fn est_du_script(key: &str, id: &str) -> bool {
    key.strip_prefix("script:").and_then(|k| k.strip_prefix(id)).is_some_and(|r| r.starts_with(':'))
}

/// The setting that shows an indicator's table: a check box among its inputs (« Dashboard », « Tableau résumé »,
/// « Afficher table Golden Dot »…). `part` « g »: the Golden Dot's own.
pub fn interrupteur(params: &[argos_script::inputs::Param], part: &str) -> Option<&'static str> {
    use argos_script::inputs::Kind;
    let est = |k: &str| params.iter().find(|p| p.key == k && matches!(p.kind, Kind::Bool)).map(|p| p.key);
    if part == "g" {
        return est("gold_tbl");
    }
    ["showTable", "dash", "conf_tbl", "showTab", "tableau", "htf_box"].into_iter().find_map(est)
}

/// How the table is shown under the pointer: hovered by the mouse (its border brighter) or selected by the finger
/// (its border brighter, the hint pill).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Survol,
    Choisi,
}

/// The table's right-click menu: its key, where it opened (chart coordinates), the row under the mouse.
#[derive(Debug, Clone, PartialEq)]
pub struct Menu {
    pub key: String,
    pub x: f64,
    pub y: f64,
    pub survol: Option<MenuLigne>,
}

/// A line of the table's menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuLigne {
    Titre,
    Remettre,
    Taille,
    Masquer,
}

impl MenuLigne {
    fn cliquable(self) -> bool {
        matches!(self, Self::Remettre | Self::Masquer)
    }
}

/// What a press on a table takes (chart coordinates): the body (move), a corner (0 top left, 1 top right, 2 bottom
/// right, 3 bottom left: scale), a side (0 top, 1 right, 2 bottom, 3 left: stretch).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Prise {
    Corps,
    Coin(u8),
    Cote(u8),
}

thread_local! {
    static LAYOUTS: RefCell<BTreeMap<String, TableLayout>> = const { RefCell::new(BTreeMap::new()) };
    static VERSION: Cell<u64> = const { Cell::new(0) };
    static POSES: RefCell<Vec<Pose>> = const { RefCell::new(Vec::new()) };
    /// Places a moved table keeps clear of besides the reserved ones (the news row, Argos Murs' pills).
    static OBSTACLES: RefCell<Vec<Rect>> = const { RefCell::new(Vec::new()) };
    /// The price pane of the last drawing (chart coordinates): the touch hint pill sits at its bottom.
    static PRIX: Cell<Rect> = const { Cell::new(Rect::new(0.0, 0.0, 0.0, 0.0)) };
    static ACTIF: RefCell<Option<(String, Mode)>> = const { RefCell::new(None) };
    static MENU: RefCell<Option<Menu>> = const { RefCell::new(None) };
    /// (tour 2, F2) The touch hint pill of the last drawing (the row of Argos Murs' « Carnets » pill).
    static BULLE_PLACE: Cell<Rect> = const { Cell::new(Rect::new(0.0, 0.0, 0.0, 0.0)) };
    /// (tour 2, F4) While the chart is drawn, the tables go on their own layer, put over the plotted things later.
    static DIFFERER: Cell<bool> = const { Cell::new(false) };
    /// (kept from one drawing to the next: only the places the tables took are put on the chart, then cleared)
    static CALQUE: RefCell<Option<Pixmap>> = const { RefCell::new(None) };
    /// (tableaux-vitesse-1) The places drawn on the layer in this drawing, in device pixels, merged when they touch.
    static PLACES: RefCell<Vec<tiny_skia::IntRect>> = const { RefCell::new(Vec::new()) };
    /// The layer already used in this drawing (its size is then the one of the drawing's first table).
    static PRIS: Cell<bool> = const { Cell::new(false) };
}

/// The tables' layouts (the app's, by key): a change draws the chart again ([`version`]).
pub fn set_layouts(m: &BTreeMap<String, TableLayout>) {
    LAYOUTS.with(|l| {
        if *l.borrow() != *m {
            l.borrow_mut().clone_from(m);
            VERSION.with(|v| v.set(v.get() + 1));
        }
    });
}

/// Something the tables' places depend on changed: the kept chart image is drawn again.
pub(crate) fn changer() {
    VERSION.with(|v| v.set(v.get() + 1));
}

/// Changes each time the layouts change (the kept chart image is drawn again).
pub fn version() -> u64 {
    VERSION.with(|v| v.get())
}

/// The layout of the table `key` (default: its automatic place, size 100 %).
pub fn layout(key: &str) -> TableLayout {
    LAYOUTS.with(|l| l.borrow().get(key).copied()).unwrap_or_default().borne()
}

/// The tables placed in the last drawing.
pub fn poses() -> Vec<Pose> {
    POSES.with(|p| p.borrow().clone())
}

/// The table placed at `key` in the last drawing.
pub fn pose(key: &str) -> Option<Pose> {
    POSES.with(|p| p.borrow().iter().find(|t| t.key == key).cloned())
}

/// The table under (`x`, `y`) (chart coordinates), its rect grown by `marge` (its edge's zones outside it).
pub fn pose_sous(x: f64, y: f64, marge: f64) -> Option<Pose> {
    POSES.with(|p| {
        p.borrow()
            .iter()
            .rev()
            .find(|t| Rect::new(t.rect.x - marge, t.rect.y - marge, t.rect.w + 2.0 * marge, t.rect.h + 2.0 * marge).contains(x, y))
            .cloned()
    })
}

/// The price pane of the last drawing.
pub fn zone_prix() -> Rect {
    PRIX.with(|p| p.get())
}

pub(crate) fn set_zone_prix(r: Rect) {
    PRIX.with(|p| p.set(r));
}

/// A new drawing: no table placed yet, no extra obstacle.
pub(crate) fn nouveau_dessin() {
    POSES.with(|p| p.borrow_mut().clear());
    OBSTACLES.with(|o| o.borrow_mut().clear());
}

/// A place a moved table keeps clear of in this drawing (besides the reserved ones).
pub(crate) fn obstacle(r: Rect) {
    OBSTACLES.with(|o| o.borrow_mut().push(r));
}

pub(crate) fn obstacles() -> Vec<Rect> {
    OBSTACLES.with(|o| o.borrow().clone())
}

pub(crate) fn poser(id: &Id, rect: Rect, area: Rect, scale: f64) {
    POSES.with(|p| {
        p.borrow_mut().push(Pose {
            key: id.key.clone(),
            titre: id.titre.clone(),
            rect,
            area,
            scale,
            ind: id.ind,
            interrupteur: id.interrupteur,
        })
    });
}

/// 🎨 (tour 2, F4) The tables of the drawing being made are kept on their own layer (`true`, by `render` around the
/// indicators) and put over the candles, the indicator lines, the walls and the drawings by [`poser_calque`]; `false`:
/// drawn at once (a table drawn outside a chart's drawing).
pub(crate) fn differer(on: bool) {
    DIFFERER.with(|d| d.set(on));
    PRIS.with(|p| p.set(false));
    // (a drawing that never put its layer: its places cleared, the layer stays clear for the next one)
    let places = PLACES.with(|p| std::mem::take(&mut *p.borrow_mut()));
    CALQUE.with(|c| {
        if let Some(calque) = c.borrow_mut().as_mut() {
            for ir in places {
                effacer(calque, ir);
            }
        }
    });
}

/// Draws a table with `f` inside `r` (chart points): on the tables' layer while the chart's drawing defers them, else
/// on `pm`.
pub(crate) fn dessiner(pm: &mut Pixmap, r: Rect, f: impl FnOnce(&mut Pixmap)) {
    if !DIFFERER.with(|d| d.get()) {
        return f(pm);
    }
    // (tableaux-vitesse-1) the layer is kept from one drawing to the next (no screen-sized image made each time);
    // made again only when a drawing starts at another size
    let deja = PRIS.with(|p| p.replace(true));
    let garde = CALQUE
        .with(|c| c.borrow_mut().take())
        .filter(|c| deja || (c.width(), c.height()) == (pm.width(), pm.height()));
    match garde.or_else(|| Pixmap::new(pm.width(), pm.height())) {
        Some(mut calque) => {
            f(&mut calque);
            if let Some(ir) = place_px(r, calque.width(), calque.height()) {
                PLACES.with(|p| ajouter_place(&mut p.borrow_mut(), ir));
            }
            CALQUE.with(|c| *c.borrow_mut() = Some(calque));
        }
        None => f(pm),
    }
}

/// 🎨 (tour 2, F4) The tables' layer put on `pm` (after everything plotted in the price pane, before the pills, the
/// tags, the legend and the floating bar); the tables are drawn at once again. (tableaux-vitesse-1) Only the places
/// the tables took are put, then cleared on the layer: the same pixels as the whole layer, at the cost of the tables.
pub(crate) fn poser_calque(pm: &mut Pixmap) {
    DIFFERER.with(|d| d.set(false));
    let places = PLACES.with(|p| std::mem::take(&mut *p.borrow_mut()));
    CALQUE.with(|c| {
        let mut c = c.borrow_mut();
        let Some(calque) = c.as_mut() else { return };
        for ir in places {
            // (lot A: laid straight from the layer, no copy, same pixels)
            crate::px::pose_zone(pm, ir.x(), ir.y(), calque.as_ref(), ir, 1.0);
            effacer(calque, ir);
        }
    });
}

/// (tableaux-vitesse-1) Room kept around a table's place for its border's antialiasing, in points.
const MARGE_CALQUE: f64 = 3.0;

/// The place `r` (chart points) with [`MARGE_CALQUE`] around it, in device pixels, inside a `w` × `h` layer.
fn place_px(r: Rect, w: u32, h: u32) -> Option<tiny_skia::IntRect> {
    let d = crate::px::dev;
    let x0 = d(r.x - MARGE_CALQUE).floor().clamp(0.0, w as f64) as i32;
    let y0 = d(r.y - MARGE_CALQUE).floor().clamp(0.0, h as f64) as i32;
    let x1 = d(r.x + r.w + MARGE_CALQUE).ceil().clamp(0.0, w as f64) as i32;
    let y1 = d(r.y + r.h + MARGE_CALQUE).ceil().clamp(0.0, h as f64) as i32;
    tiny_skia::IntRect::from_ltrb(x0, y0, x1, y1)
}

/// Adds `ir` to `places`, merged with every place it touches (a pixel is never put twice).
fn ajouter_place(places: &mut Vec<tiny_skia::IntRect>, mut ir: tiny_skia::IntRect) {
    let touche = |a: &tiny_skia::IntRect, b: &tiny_skia::IntRect| a.left() < b.right() && b.left() < a.right() && a.top() < b.bottom() && b.top() < a.bottom();
    while let Some(i) = places.iter().position(|o| touche(o, &ir)) {
        let o = places.swap_remove(i);
        let u = tiny_skia::IntRect::from_ltrb(
            o.left().min(ir.left()),
            o.top().min(ir.top()),
            o.right().max(ir.right()),
            o.bottom().max(ir.bottom()),
        );
        ir = u.unwrap_or(ir);
    }
    places.push(ir);
}

/// The place `ir` of the layer made transparent again.
fn effacer(calque: &mut Pixmap, ir: tiny_skia::IntRect) {
    let w = calque.width() as usize;
    let (x0, x1) = (ir.left().max(0) as usize, (ir.right().max(0) as usize).min(w));
    let (y0, y1) = (ir.top().max(0) as usize, (ir.bottom().max(0) as usize).min(calque.height() as usize));
    if x0 >= x1 {
        return;
    }
    let px = calque.pixels_mut();
    for y in y0..y1 {
        px[y * w + x0..y * w + x1].fill(tiny_skia::PremultipliedColorU8::TRANSPARENT);
    }
}

/// 🧪 Forgets what the last drawing placed (tests that place tables by hand).
pub fn oublier() {
    nouveau_dessin();
}

/// The table shown hovered (mouse) or selected (finger), if any. A selection made or ended draws the chart again
/// (F2: Argos Murs' « Carnets » row gives its place to the hint pill).
pub fn set_actif(a: Option<(String, Mode)>) {
    let choisi = |a: &Option<(String, Mode)>| matches!(a, Some((_, Mode::Choisi)));
    let change = ACTIF.with(|x| {
        let avant = choisi(&x.borrow());
        let apres = choisi(&a);
        *x.borrow_mut() = a;
        avant != apres
    });
    if change {
        changer();
    }
}

/// 👆 A table is selected by the finger (the hint pill is shown).
pub fn choisi() -> bool {
    matches!(actif(), Some((_, Mode::Choisi)))
}

pub fn actif() -> Option<(String, Mode)> {
    ACTIF.with(|x| x.borrow().clone())
}

/// The table's right-click menu shown, if any.
pub fn set_menu(m: Option<Menu>) {
    MENU.with(|x| *x.borrow_mut() = m);
}

/// 🖱👆 (tour 3, H5 / H6) The zones of a table at `r` (chart coordinates), none of them drawn: mouse = its 4 corners
/// (from 4 px inside to `BORD_SOURIS` − 2 outside) and its 4 edges (a band from 2 px inside to `BORD_SOURIS` − 2
/// outside); finger = its 4 corners only (`COIN_DOIGT` inside, 8 outside). Each with what it takes.
pub fn zones(r: Rect, doigt: bool) -> Vec<(Rect, Prise)> {
    let (x0, x1, y0, y1) = (r.x, r.x + r.w, r.y, r.y + r.h);
    if doigt {
        let (c, o) = (COIN_DOIGT, 8.0);
        let t = c + o;
        return vec![
            (Rect::new(x0 - o, y0 - o, t, t), Prise::Coin(0)),
            (Rect::new(x1 - c, y0 - o, t, t), Prise::Coin(1)),
            (Rect::new(x1 - c, y1 - c, t, t), Prise::Coin(2)),
            (Rect::new(x0 - o, y1 - c, t, t), Prise::Coin(3)),
        ];
    }
    // (outside the edge up to `ext`, inside it by `d` for an edge, by `dc` for a corner)
    let (b, d, dc) = (BORD_SOURIS, 2.0, 4.0);
    let ext = b - d;
    let c = ext + dc;
    vec![
        (Rect::new(x0 - ext, y0 - ext, c, c), Prise::Coin(0)),
        (Rect::new(x1 - dc, y0 - ext, c, c), Prise::Coin(1)),
        (Rect::new(x1 - dc, y1 - dc, c, c), Prise::Coin(2)),
        (Rect::new(x0 - ext, y1 - dc, c, c), Prise::Coin(3)),
        (Rect::new(x0, y0 - ext, r.w, b), Prise::Cote(0)),
        (Rect::new(x1 - d, y0, b, r.h), Prise::Cote(1)),
        (Rect::new(x0, y1 - d, r.w, b), Prise::Cote(2)),
        (Rect::new(x0 - ext, y0, b, r.h), Prise::Cote(3)),
    ]
}

/// What a press at (`x`, `y`) takes on the placed table `p`.
pub fn prise_pose(p: &Pose, x: f64, y: f64, doigt: bool) -> Option<Prise> {
    prise(p.rect, x, y, doigt)
}

/// What a press at (`x`, `y`) takes on the table at `r` (`None`: nothing of it): a corner first, then an edge, then
/// the body.
pub fn prise(r: Rect, x: f64, y: f64, doigt: bool) -> Option<Prise> {
    if let Some((_, p)) = zones(r, doigt).into_iter().find(|(h, _)| h.contains(x, y)) {
        return Some(p);
    }
    r.contains(x, y).then_some(Prise::Corps)
}

/// The rect of a table at `r` after the press `p` moved by (`dx`, `dy`), and its layout from `lay` (bounds held):
/// the body moves; a corner scales everything around the opposite corner; a side stretches the width (left /
/// right) or the row height (top / bottom), the opposite side held.
pub fn glisser(lay: TableLayout, r: Rect, area: Rect, p: Prise, dx: f64, dy: f64) -> TableLayout {
    let lay = lay.borne();
    match p {
        Prise::Corps => {
            let n = dans(Rect::new(r.x + dx, r.y + dy, r.w, r.h), area);
            lay.ancre(n, area, None)
        }
        Prise::Coin(c) => {
            // (the corner follows the mouse: the larger of the two ratios, the opposite corner held)
            let sx = if matches!(c, 1 | 2) { dx } else { -dx };
            let sy = if c >= 2 { dy } else { -dy };
            let k = ((r.w + sx) / r.w.max(1.0)).max((r.h + sy) / r.h.max(1.0));
            let scale = (lay.scale * k).clamp(SCALE_MIN, SCALE_MAX);
            let f = scale / lay.scale;
            let (w, h) = (r.w * f, r.h * f);
            let oppose = (c + 2) % 4;
            let n = match oppose {
                0 => Rect::new(r.x, r.y, w, h),
                1 => Rect::new(r.x + r.w - w, r.y, w, h),
                2 => Rect::new(r.x + r.w - w, r.y + r.h - h, w, h),
                _ => Rect::new(r.x, r.y + r.h - h, w, h),
            };
            TableLayout { scale, ..lay }.ancre(n, area, Some(oppose))
        }
        Prise::Cote(s) => {
            let haut_bas = s % 2 == 0;
            let (d, base) = match s {
                0 => (-dy, r.h),
                1 => (dx, r.w),
                2 => (dy, r.h),
                _ => (-dx, r.w),
            };
            let k = (base + d) / base.max(1.0);
            let mut n = lay;
            if haut_bas {
                n.hy = (lay.hy * k).clamp(ETIRE_MIN, ETIRE_MAX);
            } else {
                n.wx = (lay.wx * k).clamp(ETIRE_MIN, ETIRE_MAX);
            }
            let (w, h) = if haut_bas { (r.w, r.h * n.hy / lay.hy) } else { (r.w * n.wx / lay.wx, r.h) };
            // (the opposite side held: its corner is the anchor; the other direction keeps the nearest corner)
            let bas = r.cy() > area.cy();
            let droite = r.cx() > area.cx();
            let (rect, coin) = match s {
                0 => (Rect::new(r.x, r.y + r.h - h, w, h), if droite { 2 } else { 3 }),
                1 => (Rect::new(r.x, r.y, w, h), if bas { 3 } else { 0 }),
                2 => (Rect::new(r.x, r.y, w, h), if droite { 1 } else { 0 }),
                _ => (Rect::new(r.x + r.w - w, r.y, w, h), if bas { 2 } else { 1 }),
            };
            n.ancre(rect, area, Some(coin))
        }
    }
}

/// The table pinched by two fingers: its scale times `k`, its centre held.
pub fn pincer(lay: TableLayout, r: Rect, area: Rect, k: f64) -> TableLayout {
    let lay = lay.borne();
    let scale = (lay.scale * k).clamp(SCALE_MIN, SCALE_MAX);
    let f = scale / lay.scale;
    let (w, h) = (r.w * f, r.h * f);
    let n = dans(Rect::new(r.cx() - w / 2.0, r.cy() - h / 2.0, w, h), area);
    TableLayout { scale, ..lay }.ancre(n, area, None)
}

/// Text of the touch hint pill.
pub const BULLE: &str = "Glisser · pincer pour la taille";

/// The hint pill's sizes on a plot `pw` wide: height (Argos Murs' « Carnets » pill's), font, gap under it.
fn bulle_tailles(pw: f64) -> (f64, f32, f64) {
    if pw < 560.0 {
        (22.0, 11.0, 6.0)
    } else {
        (32.0, 13.0, 14.0)
    }
}

/// (tour 2, F2) Where the touch hint pill goes on a plot `pw` wide whose bottom pills sit on `bas`: the row of Argos
/// Murs' « Carnets » pill (8 points from the left, its height, never taller), as wide as its text and « OK ».
pub(crate) fn bulle_place(text: &Text, pw: f64, bas: f64) -> Rect {
    let (h, f, sous) = bulle_tailles(pw);
    let ok_w = if pw < 560.0 { 34.0 } else { 44.0 };
    let w = (text.width(BULLE, f) as f64 + 14.0 + 10.0 + ok_w + 3.0).min((pw - 16.0).max(0.0));
    Rect::new(8.0, bas - h - sous, w, h)
}

pub(crate) fn set_bulle(r: Rect) {
    BULLE_PLACE.with(|b| b.set(r));
}

/// The touch hint pill (where the last drawing put it, Argos Murs' « Carnets » row) and its « OK » button.
pub fn bulle_doigt() -> (Rect, Rect) {
    let r = BULLE_PLACE.with(|b| b.get());
    let ok_w = if r.h < 30.0 { 34.0 } else { 44.0 };
    (r, Rect::new(r.x + r.w - ok_w - 3.0, r.y + 3.0, ok_w, (r.h - 6.0).max(0.0)))
}

/// The finger's target of « OK »: the button grown to 40 points high at least.
pub fn ok_cible() -> Rect {
    let ok = bulle_doigt().1;
    let g = ((CIBLE_DOIGT - ok.h) / 2.0).max(0.0);
    Rect::new(ok.x - 4.0, ok.y - g, ok.w + 8.0, ok.h + 2.0 * g)
}

const MENU_W: f64 = 220.0;
const MENU_LIGNE: f64 = 30.0;

/// The lines of the table's menu (chart coordinates), kept inside the price pane.
pub fn menu_lignes(m: &Menu) -> Vec<(Rect, MenuLigne)> {
    let Some(t) = pose(&m.key) else { return vec![] };
    let mut lignes = vec![MenuLigne::Titre, MenuLigne::Remettre, MenuLigne::Taille];
    if t.interrupteur.is_some() {
        lignes.push(MenuLigne::Masquer);
    }
    let h = lignes.len() as f64 * MENU_LIGNE + 12.0;
    let r = menu_place(&t, m.y, h);
    lignes
        .into_iter()
        .enumerate()
        .map(|(k, l)| (Rect::new(r.x + 6.0, r.y + 6.0 + k as f64 * MENU_LIGNE, r.w - 12.0, MENU_LIGNE), l))
        .collect()
}

/// (tour 2, F6) The menu card beside the table (mockup panel 2): on its right, on its left when there is no room,
/// at the height of the click; never over the table (and its edge's zones), Argos Murs' pills, the bottom pills or
/// another table; inside the pane.
fn menu_place(t: &Pose, y: f64, h: f64) -> Rect {
    let zp = zone_prix();
    // (the pane: the price pane, or the band the table is in)
    let p = if zp.w > 0.0 && t.area.y < zp.y + zp.h { zp } else { t.area };
    let p = Rect::new(p.x, p.y, p.w.max(MENU_W), p.h.max(h));
    let m = MARGE + BORD_SOURIS;
    let tab = Rect::new(t.rect.x - m, t.rect.y - m - 12.0, t.rect.w + 2.0 * m, t.rect.h + 2.0 * m + 12.0);
    let mut obst: Vec<Rect> = vec![tab];
    obst.extend(obstacles());
    obst.extend(crate::indicators::reserves());
    obst.extend(poses().into_iter().filter(|q| q.key != t.key).map(|q| q.rect));
    let y = (y - MENU_LIGNE).clamp(p.y, (p.y + p.h - h).max(p.y));
    let droite = Rect::new(tab.x + tab.w + 4.0, y, MENU_W, h);
    let gauche = Rect::new(tab.x - 4.0 - MENU_W, y, MENU_W, h);
    let dedans = |c: &Rect| c.x >= p.x && c.x + c.w <= p.x + p.w;
    [droite, gauche]
        .into_iter()
        .filter(dedans)
        .find_map(|c| libre(c, p, &obst).filter(|r| !croise(r, &tab)))
        .or_else(|| libre(droite, p, &obst))
        .or_else(|| libre(droite, p, &[tab]))
        .unwrap_or_else(|| dans(droite, p))
}

/// The whole menu card (chart coordinates).
pub fn menu_rect(m: &Menu) -> Option<Rect> {
    let l = menu_lignes(m);
    let (a, b) = (l.first()?.0, l.last()?.0);
    Some(Rect::new(a.x - 6.0, a.y - 6.0, a.w + 12.0, b.y + b.h - a.y + 12.0))
}

/// The clickable line of the menu under (`x`, `y`).
pub fn menu_sous(m: &Menu, x: f64, y: f64) -> Option<MenuLigne> {
    menu_lignes(m).into_iter().find(|(r, l)| l.cliquable() && r.contains(x, y)).map(|(_, l)| l)
}

fn texte_menu(l: MenuLigne, t: &Pose) -> String {
    match l {
        MenuLigne::Titre => format!("Tableau {}", crate::text::de(&t.titre)),
        MenuLigne::Remettre => "↺ Remettre à sa place".to_string(),
        MenuLigne::Taille => format!("Taille : {:.0} %", t.scale * 100.0),
        MenuLigne::Masquer => "Masquer le tableau".to_string(),
    }
}

/// (tour 3, H5) The table's own border, brighter: the same 1 px line and radius as its frame (square for a script's
/// `table.new`), white at [`BORD_ACTIF`] over it.
fn bord_actif(pm: &mut Pixmap, t: &Pose) {
    let rayon = if t.key.starts_with("script:") { 0.0 } else { 6.0 };
    crate::shapes::round_stroke(pm, t.rect, rayon, crate::palette::WHITE, BORD_ACTIF);
}

/// 🎯 Over the chart (the mouse layer, `finir`): the hovered / selected table's brighter border (nothing else around
/// it: no outline, no handle, no grip), the touch hint pill, the table's menu. Nothing when no table is active.
pub fn dessiner_dessus(pm: &mut Pixmap, text: &Text) {
    if let Some((key, mode)) = actif() {
        if let Some(t) = pose(&key) {
            bord_actif(pm, &t);
            if mode == Mode::Choisi {
                // (F2: in the row of Argos Murs' « Carnets » pill, its height; that pill is hidden meanwhile)
                let (b, ok) = bulle_doigt();
                let f: f32 = if b.h < 30.0 { 11.0 } else { 13.0 };
                let base = |y: f64| (y + f as f64 * 0.36) as f32;
                crate::shapes::round_fill(pm, b, b.h / 2.0, crate::palette::panel(), 0.95);
                crate::shapes::round_stroke(pm, b, b.h / 2.0, BLEU, 1.0);
                let room = (ok.x - b.x - 14.0 - 8.0).max(0.0);
                let s = text.fit(BULLE, f, room);
                text.draw(pm, &s, ((b.x + 14.0) as f32, base(b.cy())), f, crate::palette::WHITE, Align::Left);
                crate::shapes::round_fill(pm, ok, ok.h / 2.0, BLEU, 1.0);
                text.draw(pm, "OK", (ok.cx() as f32, base(ok.cy())), f, crate::palette::WHITE, Align::Center);
            }
        }
    }
    let Some(m) = MENU.with(|x| x.borrow().clone()) else { return };
    let Some(t) = pose(&m.key) else { return };
    let Some(card) = menu_rect(&m) else { return };
    crate::shapes::menu_card(pm, card);
    for (r, l) in menu_lignes(&m) {
        crate::shapes::menu_row(pm, r, false, m.survol == Some(l) && l.cliquable());
        let c = if l == MenuLigne::Titre { crate::palette::GREY } else { crate::palette::WHITE };
        text.draw(pm, &texte_menu(l, &t), ((r.x + 10.0) as f32, (r.cy() + 4.5) as f32), 13.0, c, Align::Left);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const AREA: Rect = Rect::new(0.0, 0.0, 1000.0, 600.0);

    #[test]
    fn glisser_le_corps_deplace_le_tableau() {
        let r = Rect::new(700.0, 40.0, 200.0, 150.0);
        let l = glisser(TableLayout::default(), r, AREA, Prise::Corps, -500.0, 300.0);
        assert!(l.moved);
        let n = l.place(AREA, 200.0, 150.0);
        assert_eq!((n.x, n.y), (200.0, 340.0));
        // (outside: kept in the area)
        let l = glisser(TableLayout::default(), r, AREA, Prise::Corps, 900.0, 900.0);
        let n = l.place(AREA, 200.0, 150.0);
        assert_eq!((n.x + n.w, n.y + n.h), (1000.0, 600.0));
    }

    #[test]
    fn un_coin_grossit_autour_du_coin_oppose() {
        let r = Rect::new(100.0, 100.0, 200.0, 100.0);
        // the bottom right corner pulled by 100 px: the top left stays
        let l = glisser(TableLayout::default(), r, AREA, Prise::Coin(2), 100.0, 0.0);
        assert!((l.scale - 1.5).abs() < 1e-9, "{l:?}");
        let n = l.place(AREA, 300.0, 150.0);
        assert_eq!((n.x, n.y), (100.0, 100.0), "the opposite corner held");
        // the top left corner pulled in: the bottom right stays
        let l = glisser(TableLayout::default(), r, AREA, Prise::Coin(0), 40.0, 20.0);
        let f = l.scale;
        assert!(f < 1.0);
        let n = l.place(AREA, 200.0 * f, 100.0 * f);
        assert!((n.x + n.w - 300.0).abs() < 1e-9 && (n.y + n.h - 200.0).abs() < 1e-9, "{n:?}");
    }

    #[test]
    fn les_cotes_etirent_et_les_bornes_tiennent() {
        let r = Rect::new(100.0, 100.0, 200.0, 100.0);
        let l = glisser(TableLayout::default(), r, AREA, Prise::Cote(1), 50.0, 0.0);
        assert!((l.wx - 1.25).abs() < 1e-9 && l.hy == 1.0 && l.scale == 1.0);
        let n = l.place(AREA, 250.0, 100.0);
        assert_eq!(n.x, 100.0, "the left side held");
        let l = glisser(TableLayout::default(), r, AREA, Prise::Cote(3), 50.0, 0.0);
        let n = l.place(AREA, 200.0 * l.wx, 100.0);
        assert!((n.x + n.w - 300.0).abs() < 1e-9, "the right side held: {n:?}");
        let l = glisser(TableLayout::default(), r, AREA, Prise::Cote(2), 0.0, 30.0);
        assert!((l.hy - 1.3).abs() < 1e-9 && l.wx == 1.0);
        // bounds
        let big = glisser(TableLayout::default(), r, AREA, Prise::Coin(2), 5000.0, 5000.0);
        assert_eq!(big.scale, SCALE_MAX);
        let small = glisser(TableLayout::default(), r, AREA, Prise::Coin(2), -190.0, -90.0);
        assert_eq!(small.scale, SCALE_MIN);
        let w = glisser(TableLayout::default(), r, AREA, Prise::Cote(1), 5000.0, 0.0);
        assert_eq!(w.wx, ETIRE_MAX);
        let h = glisser(TableLayout::default(), r, AREA, Prise::Cote(2), 0.0, -95.0);
        assert_eq!(h.hy, ETIRE_MIN);
        let p = pincer(TableLayout::default(), r, AREA, 10.0);
        assert_eq!(p.scale, SCALE_MAX);
        let fou = TableLayout {
            scale: f64::NAN,
            wx: 9.0,
            hy: 0.1,
            corner: 7,
            ..Default::default()
        }
        .borne();
        assert_eq!((fou.scale, fou.wx, fou.hy, fou.corner), (1.0, ETIRE_MAX, ETIRE_MIN, 3));
    }

    #[test]
    fn la_place_survit_a_un_changement_de_fenetre() {
        let r = Rect::new(20.0, 400.0, 200.0, 150.0);
        let l = TableLayout::default().ancre(r, AREA, None);
        assert_eq!(l.corner, 3, "bottom left");
        let petit = Rect::new(0.0, 0.0, 390.0, 500.0);
        let n = l.place(petit, 200.0, 150.0);
        assert_eq!((n.x, n.y + n.h), (20.0, 450.0), "still 20 from the left and 50 from the bottom");
    }

    #[test]
    fn libre_evite_les_obstacles() {
        let obst = [Rect::new(0.0, 0.0, 300.0, 100.0), Rect::new(800.0, 0.0, 200.0, 600.0)];
        let r = Rect::new(700.0, 50.0, 200.0, 100.0);
        let n = libre(r, AREA, &obst).unwrap();
        assert!(!obst.iter().any(|o| croise(o, &n)) && n.x + n.w <= 1000.0 && n.y + n.h <= 600.0);
        assert!((n.x - 700.0).hypot(n.y - 50.0) <= 110.0, "the nearest: {n:?}");
        assert_eq!(
            libre(Rect::new(400.0, 300.0, 100.0, 100.0), AREA, &obst),
            Some(Rect::new(400.0, 300.0, 100.0, 100.0))
        );
        assert_eq!(libre(Rect::new(0.0, 0.0, 2000.0, 10.0), AREA, &[]), None, "too wide");
    }

    #[test]
    fn cles_par_indicateur_et_par_script() {
        assert!(est_de(&cle(Ind::Cipher, "g"), Ind::Cipher) && est_de(&cle(Ind::Cipher, ""), Ind::Cipher));
        assert!(!est_de(&cle(Ind::Tvd, ""), Ind::Tbo));
        assert!(!est_de("SmcPro", Ind::Smc), "another indicator whose name starts the same");
        assert!(est_du_script("script:abc:0", "abc") && !est_du_script("script:abcd:0", "abc"));
        let p = &argos_script::v1_inputs::TVD_PARAMS;
        assert_eq!(interrupteur(p, ""), Some("showTable"));
    }

    /// (tour 3, H5 / H6) Mouse: the body moves, a corner scales, an edge (all along it) stretches, just outside it
    /// too; finger: the body moves, a drag starting within 24 points of a corner scales.
    #[test]
    fn prises_souris_et_doigt() {
        let r = Rect::new(100.0, 100.0, 200.0, 100.0);
        assert_eq!(prise(r, 150.0, 150.0, false), Some(Prise::Corps));
        assert_eq!(prise(r, 96.0, 96.0, false), Some(Prise::Coin(0)));
        assert_eq!(prise(r, 302.0, 202.0, false), Some(Prise::Coin(2)), "1 px out of the bottom right corner");
        assert_eq!(prise(r, 304.0, 150.0, false), Some(Prise::Cote(1)));
        assert_eq!(prise(r, 304.0, 120.0, false), Some(Prise::Cote(1)), "the whole edge, not only its middle");
        assert_eq!(prise(r, 140.0, 205.0, false), Some(Prise::Cote(2)));
        assert_eq!(prise(r, 200.0, 92.0, false), Some(Prise::Cote(0)), "no grip any more: the top edge");
        assert_eq!(prise(r, 200.0, 80.0, false), None);
        assert_eq!(prise(r, 500.0, 500.0, false), None);
        // finger: 10 points inside a corner scales, the middle moves, an edge is the body
        assert_eq!(prise(r, 290.0, 190.0, true), Some(Prise::Coin(2)));
        assert_eq!(prise(r, 110.0, 110.0, true), Some(Prise::Coin(0)));
        assert_eq!(prise(r, 305.0, 95.0, true), Some(Prise::Coin(1)), "a little outside");
        assert_eq!(prise(r, 200.0, 150.0, true), Some(Prise::Corps));
        assert_eq!(prise(r, 200.0, 102.0, true), Some(Prise::Corps), "the top edge moves it");
        assert_eq!(prise(r, 270.0, 170.0, true), Some(Prise::Corps), "30 points in: the body");
        assert_eq!(zones(r, true).len(), 4);
    }

    /// 🧪 (tour 2) A table placed by the one path, as a drawing does: `w` × `h` at 100 %, its default place `defaut`.
    fn placer_test(key: &str, area: Rect, w: f64, h: f64, defaut: Rect) -> Option<(Rect, f64)> {
        let id = Id {
            key: key.to_string(),
            titre: key.to_string(),
            ind: None,
            interrupteur: None,
        };
        crate::indicators::placer(&id, area, &|s| (w * s, h * s), &|w, h| Rect::new(defaut.x, defaut.y, w, h))
    }

    /// F5: a table whose own place is taken (another table there) goes to the nearest free place at its size; smaller
    /// (down to 0.6) only when nothing fits; not drawn only when nothing fits even at 0.6.
    #[test]
    fn un_tableau_allume_est_toujours_visible() {
        for pw in [390.0, 820.0, 1440.0] {
            crate::indicators::nouveau_dessin_test();
            let area = Rect::new(0.0, 0.0, pw, 500.0);
            let d = Rect::new(8.0, 300.0, 180.0, 120.0);
            let (a, sa) = placer_test("a", area, 180.0, 120.0, d).expect("the first");
            let (b, sb) = placer_test("b", area, 180.0, 120.0, d).expect("the second: visible too");
            assert!(!croise(&a, &b), "{pw}: no overlap: {a:?} / {b:?}");
            assert_eq!((sa, sb), (1.0, 1.0), "{pw}: room: its size");
            assert!(b.x >= 0.0 && b.y >= 0.0 && b.x + b.w <= pw && b.y + b.h <= 500.0);
        }
        // a pane only one table fits in at 100 %: the second at a smaller scale, then none
        crate::indicators::nouveau_dessin_test();
        let area = Rect::new(0.0, 0.0, 200.0, 220.0);
        let d = Rect::new(0.0, 0.0, 200.0, 120.0);
        let (a, _) = placer_test("a", area, 200.0, 120.0, d).unwrap();
        let (b, s) = placer_test("b", area, 200.0, 120.0, d).expect("smaller");
        assert!((SCALE_MIN - 1e-9..1.0).contains(&s) && !croise(&a, &b), "{s} {b:?}");
        assert_eq!(placer_test("c", area, 200.0, 120.0, d), None, "nothing fits even at 0.6");
    }

    /// F2: the touch hint pill is in the « Carnets » row: its height (22 on a phone, 32 wider), never taller, inside
    /// the plot, its « OK » inside it and a finger target of 40 at least.
    #[test]
    fn la_bulle_au_doigt_prend_la_rangee_des_carnets() {
        let t = Text::default();
        for pw in [390.0, 820.0, 1440.0] {
            let b = bulle_place(&t, pw, 600.0);
            let h = if pw < 560.0 { 22.0 } else { 32.0 };
            assert_eq!(b.h, h, "{pw}");
            assert!(b.x >= 8.0 && b.x + b.w <= pw - 8.0 + 1e-9 && b.y + b.h <= 600.0, "{pw}: {b:?}");
            set_bulle(b);
            let (r, ok) = bulle_doigt();
            assert_eq!(r, b);
            assert!(
                ok.x >= r.x && ok.x + ok.w <= r.x + r.w && ok.y >= r.y && ok.y + ok.h <= r.y + r.h,
                "{pw}: {ok:?}"
            );
            assert!(ok_cible().h >= CIBLE_DOIGT - 1e-9 && ok_cible().contains(ok.cx(), ok.cy()));
        }
    }

    /// F6: the right-click menu opens beside the table (right, left at the right edge), never over it, inside the pane.
    #[test]
    fn le_menu_s_ouvre_a_cote_du_tableau() {
        for pw in [390.0, 820.0, 1440.0] {
            crate::indicators::nouveau_dessin_test();
            let zone = Rect::new(0.0, 0.0, pw, 600.0);
            set_zone_prix(zone);
            let id = Id {
                key: "t".into(),
                titre: "Argos TVD Pro".into(),
                ind: None,
                interrupteur: None,
            };
            for x in [10.0, pw - 150.0 - 10.0] {
                crate::indicators::nouveau_dessin_test();
                let r = Rect::new(x, 80.0, 150.0, 200.0);
                poser(&id, r, zone, 1.0);
                let m = Menu {
                    key: "t".into(),
                    x: r.cx(),
                    y: r.cy(),
                    survol: None,
                };
                let c = menu_rect(&m).expect("the menu");
                let autour = Rect::new(r.x - MARGE, r.y - MARGE, r.w + 2.0 * MARGE, r.h + 2.0 * MARGE);
                assert!(!croise(&c, &autour), "{pw} {x}: the menu {c:?} over the table {r:?}");
                assert!(c.x >= 0.0 && c.y >= 0.0 && c.x + c.w <= pw + 1e-9 && c.y + c.h <= 600.0 + 1e-9, "{pw}: {c:?}");
                if pw >= 1440.0 {
                    assert!(if x < 100.0 { c.x > r.x + r.w } else { c.x + c.w < r.x }, "{pw} {x}: beside it: {c:?}");
                }
            }
        }
    }

    /// (tour 3, H5 / H6) Under the mouse or selected by the finger, a table only gets its border brighter: nothing is
    /// drawn outside it (no outline, no handle, no grip) and its inside is untouched; the finger also gets the hint pill.
    #[test]
    fn seul_le_bord_s_eclaircit() {
        let t = Text::default();
        let fond = Rgb(0x10, 0x12, 0x16);
        let id = Id {
            key: "TvdPro".into(),
            titre: "Argos TVD Pro".into(),
            ind: None,
            interrupteur: None,
        };
        let r = Rect::new(60.0, 40.0, 120.0, 80.0);
        for mode in [Mode::Survol, Mode::Choisi] {
            crate::indicators::nouveau_dessin_test();
            set_zone_prix(Rect::new(0.0, 0.0, 400.0, 300.0));
            set_bulle(Rect::new(8.0, 250.0, 200.0, 32.0));
            poser(&id, r, Rect::new(0.0, 0.0, 400.0, 300.0), 1.0);
            let mut avant = Pixmap::new(400, 300).unwrap();
            crate::fill(&mut avant, 0.0, 0.0, 400.0, 300.0, fond, 1.0);
            crate::shapes::round_fill(&mut avant, r, 6.0, crate::palette::popup(), 0.92);
            crate::shapes::round_stroke(&mut avant, r, 6.0, crate::palette::edge_soft(), 1.0);
            let mut apres = avant.clone();
            set_actif(Some(("TvdPro".into(), mode)));
            dessiner_dessus(&mut apres, &t);
            set_actif(None);
            let w = avant.width() as usize;
            let (a, b) = (avant.pixels(), apres.pixels());
            let mut bord_plus_clair = false;
            for y in 0..240usize {
                for x in 0..400usize {
                    let k = y * w + x;
                    let (xf, yf) = (x as f64 + 0.5, y as f64 + 0.5);
                    let dehors = !Rect::new(r.x - 1.5, r.y - 1.5, r.w + 3.0, r.h + 3.0).contains(xf, yf);
                    // (the rounded corners' arc runs inside the rect: left out of the inside)
                    let coin = (xf < r.x + 7.0 || xf > r.x + r.w - 7.0) && (yf < r.y + 7.0 || yf > r.y + r.h - 7.0);
                    let dedans = !coin && Rect::new(r.x + 1.5, r.y + 1.5, r.w - 3.0, r.h - 3.0).contains(xf, yf);
                    if dehors || dedans {
                        assert_eq!(a[k], b[k], "{mode:?}: ({x}, {y}) changed: only the border may");
                    } else if b[k].red() > a[k].red() + 20 {
                        bord_plus_clair = true;
                    }
                }
            }
            assert!(bord_plus_clair, "{mode:?}: the border is brighter");
            // (the finger's hint pill, in its row)
            let pill = (250..282).any(|y| (8..208).any(|x| a[y * w + x] != b[y * w + x]));
            assert_eq!(pill, mode == Mode::Choisi, "{mode:?}: the hint pill only for the finger");
        }
        set_zone_prix(Rect::new(0.0, 0.0, 0.0, 0.0));
    }

    /// F4: while a chart is drawn, a table goes on its own layer, put over what was drawn meanwhile.
    #[test]
    fn les_tableaux_passent_au_dessus_des_lignes() {
        let mut pm = Pixmap::new(40, 40).unwrap();
        differer(true);
        dessiner(&mut pm, Rect::new(0.0, 0.0, 40.0, 40.0), |l| {
            crate::fill(l, 0.0, 0.0, 40.0, 40.0, Rgb(0, 0, 255), 1.0)
        });
        assert_eq!(pm.pixel(20, 20).map(|p| p.alpha()), Some(0), "not on the chart yet");
        // (a line drawn after the table, on the chart)
        crate::fill(&mut pm, 0.0, 20.0, 40.0, 1.0, Rgb(255, 0, 0), 1.0);
        poser_calque(&mut pm);
        let p = pm.pixel(20, 20).unwrap();
        assert!(p.blue() > 200 && p.red() < 50, "the table over the line: {p:?}");
        // (outside a drawing: at once)
        let mut pm = Pixmap::new(4, 4).unwrap();
        dessiner(&mut pm, Rect::new(0.0, 0.0, 4.0, 4.0), |l| {
            crate::fill(l, 0.0, 0.0, 4.0, 4.0, Rgb(0, 255, 0), 1.0)
        });
        assert_eq!(pm.pixel(1, 1).map(|p| p.green()), Some(255));
    }

    /// (tableaux-vitesse-1) The layer is kept from one drawing to the next and only the tables' places are put: a
    /// table drawn before is never seen again where it was, two tables side by side are each put once.
    #[test]
    fn le_calque_garde_ne_laisse_rien_derriere() {
        let bleu = |l: &mut Pixmap, r: Rect| crate::fill(l, r.x, r.y, r.w, r.h, Rgb(0, 0, 255), 0.5);
        let (a, b) = (Rect::new(5.0, 5.0, 20.0, 10.0), Rect::new(40.0, 30.0, 20.0, 10.0));
        // (1st drawing: a table at `a`)
        let mut pm = Pixmap::new(80, 60).unwrap();
        differer(true);
        dessiner(&mut pm, a, |l| bleu(l, a));
        poser_calque(&mut pm);
        let une = pm.pixel(10, 10).unwrap();
        assert!(une.blue() > 0, "the table at a: {une:?}");
        // (2nd drawing: the table moved to `b`, and its neighbour right next to it)
        let mut pm = Pixmap::new(80, 60).unwrap();
        differer(true);
        let c = Rect::new(61.0, 30.0, 10.0, 10.0);
        dessiner(&mut pm, b, |l| bleu(l, b));
        dessiner(&mut pm, c, |l| bleu(l, c));
        poser_calque(&mut pm);
        assert_eq!(pm.pixel(10, 10).map(|p| p.alpha()), Some(0), "nothing left at a");
        assert_eq!(pm.pixel(45, 35), Some(une), "b put once, like a");
        assert_eq!(pm.pixel(65, 35), Some(une), "c put once, like a");
        // (a drawing that never put its layer leaves nothing either)
        differer(true);
        dessiner(&mut pm, a, |l| bleu(l, a));
        differer(true);
        let mut pm = Pixmap::new(80, 60).unwrap();
        poser_calque(&mut pm);
        assert_eq!(pm.pixel(10, 10).map(|p| p.alpha()), Some(0), "nothing left from the drawing never put");
    }
}
