//! 🖍 The bar of a selected drawing (Jee's mockup, 17 Nov): one glass pill with round ends above the drawing, in the
//! navigation pill's style (zoompad), grey icons, regular spacing:
//! ⠿ handle · template · line colour · text colour · width « — 1px » · line style · ⚙ · alert · padlock · 🗑 · ⋯.
//! Only the buttons that apply to the drawing's tool; the same bar on PC, tablet and phone (bigger touch zones).

use crate::drawings::Drawing;
use crate::shapes::{lines, round_fill, Rect};
use crate::text::{Align, Rgb, Text};
use crate::zoompad;
use tiny_skia::Pixmap;

/// A button of the bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DbBtn {
    /// ⠿ drag the bar.
    Handle,
    /// Drawing templates (save as…, apply default, the saved ones).
    Template,
    LineColor,
    TextColor,
    /// « — 1px »: 1, 2, 3, 4 px.
    Width,
    /// Solid, dashed, dotted.
    Style,
    /// ⚙ the settings window.
    Settings,
    /// Alarm with + : an alert on the drawing.
    Alert,
    Lock,
    /// 🗑 at once (undo brings it back).
    Trash,
    /// ⋯ more.
    More,
}

/// Tools that have no line to colour, thicken or dash (texts, labels, markers, images, positions).
const NO_LINE: &[&str] = &[
    "text",
    "note",
    "pricenote",
    "pin",
    "callout",
    "comment",
    "pricelabel",
    "signpost",
    "flag",
    "table",
    "image",
    "icon",
    "arrowup",
    "arrowdn",
    "amup",
    "amdn",
    "amleft",
    "amright",
    "long",
    "short",
];
/// Tools that never show a text.
const NO_TEXT: &[&str] = &["image", "icon", "brush", "highlighter"];

/// Tools an alert can follow (a price line through time): the lines and channels.
pub fn alertable(tool: &str) -> bool {
    crate::rail::group_of(tool) == Some("lines") && tool != "vline"
}

/// The buttons of the bar for a drawing of `tool`, in order.
pub fn buttons(tool: &str) -> Vec<DbBtn> {
    use DbBtn::*;
    let line = !NO_LINE.contains(&tool);
    let mut v = vec![Handle, Template];
    if line {
        v.push(LineColor);
    }
    if !NO_TEXT.contains(&tool) {
        v.push(TextColor);
    }
    if line {
        v.extend([Width, Style]);
    }
    v.push(Settings);
    if alertable(tool) {
        v.push(Alert);
    }
    v.extend([Lock, Trash, More]);
    v
}

/// The tooltip of a button (`locked`: the drawing is locked).
pub fn tip(b: DbBtn, locked: bool) -> &'static str {
    match b {
        DbBtn::Handle => "Déplacer la barre",
        DbBtn::Template => "Modèle de dessin",
        DbBtn::LineColor => "Couleur des outils de ligne",
        DbBtn::TextColor => "Couleur du texte des outils de ligne",
        DbBtn::Width => "Largeur de l'outil de ligne",
        DbBtn::Style => "Style",
        DbBtn::Settings => "Paramètres",
        DbBtn::Alert => "Ajouter une alerte",
        DbBtn::Lock if locked => "Déverrouiller",
        DbBtn::Lock => "Verrouiller",
        DbBtn::Trash => "Supprimer",
        DbBtn::More => "Plus",
    }
}

/// The bar laid out: the pill and each button (chart coordinates).
#[derive(Debug, Clone, PartialEq)]
pub struct Bar {
    pub rect: Rect,
    pub btns: Vec<(Rect, DbBtn)>,
    /// 📱 Buttons moved into ⋯ when the bar would not fit the screen's width (template, text colour).
    pub folded: Vec<DbBtn>,
}

impl Bar {
    /// The button under (`x`, `y`).
    pub fn hit(&self, x: f64, y: f64) -> Option<DbBtn> {
        self.rect
            .contains(x, y)
            .then(|| self.btns.iter().find(|(r, _)| r.contains(x, y)).map(|(_, b)| *b))
            .flatten()
    }

    /// The place of a button (to open its menu under it).
    pub fn at(&self, b: DbBtn) -> Option<Rect> {
        self.btns.iter().find(|(_, k)| *k == b).map(|(r, _)| *r)
    }
}

/// Width of a button: the handle is narrow, « — 1px » wider; on touch (`touch`), at least 40 points.
fn width_of(b: DbBtn, touch: bool) -> f64 {
    let base = if touch { 40.0 } else { 34.0 };
    match b {
        DbBtn::Handle => {
            if touch {
                32.0
            } else {
                20.0
            }
        }
        DbBtn::Width => base + 30.0,
        _ => base,
    }
}

/// Lays the bar for a drawing of `tool`: centred above `anchor` (the top of the drawing, chart coordinates), moved by
/// the handle's `offset`, always whole inside the chart (`pw` × `ph`; above the drawing, or below it if no room).
pub fn layout(tool: &str, anchor: (f64, f64), offset: (f64, f64), pw: f64, ph: f64, touch: bool) -> Bar {
    let mut bs = buttons(tool);
    let h = if touch { 44.0 } else { 38.0 };
    let pad = 6.0;
    let width = |bs: &[DbBtn]| bs.iter().map(|b| width_of(*b, touch)).sum::<f64>() + 2.0 * pad;
    // 📱 too wide for the screen (phone): no handle, then the template and the text colour go into ⋯ — one row,
    // never off the screen, every action still there
    let mut folded = vec![];
    if width(&bs) > pw - 12.0 {
        bs.retain(|b| *b != DbBtn::Handle);
    }
    for b in [DbBtn::Template, DbBtn::TextColor] {
        if width(&bs) > pw - 12.0 && bs.contains(&b) {
            bs.retain(|x| *x != b);
            folded.push(b);
        }
    }
    let w = width(&bs);
    let mut x = anchor.0 - w / 2.0 + offset.0;
    let mut y = anchor.1 - h - 16.0 + offset.1;
    if offset == (0.0, 0.0) && y < 8.0 {
        y = anchor.1 + 24.0;
    }
    x = x.min(pw - w - 6.0).max(6.0);
    y = y.min(ph - h - 6.0).max(6.0);
    let mut cx = x + pad;
    let btns = bs
        .into_iter()
        .map(|b| {
            let bw = width_of(b, touch);
            let r = Rect::new(cx, y + 3.0, bw, h - 6.0);
            cx += bw;
            (r, b)
        })
        .collect();
    Bar {
        rect: Rect::new(x, y, w, h),
        btns,
        folded,
    }
}

/// The colour of a style field (« #rrggbb »), or `def`.
fn colour(s: &str, def: Rgb) -> Rgb {
    crate::colorpick::split(s).map_or(def, |p| p.0)
}

/// Draws the bar: the pill (zoompad style), hover and open button on a round patch, the locked padlock on the light
/// patch. The image covers `bar.rect`.
pub fn draw(bar: &Bar, d: &Drawing, text: &Text, mouse: Option<(f64, f64)>, open: Option<DbBtn>) -> Option<Pixmap> {
    let r0 = bar.rect;
    let mut pm = crate::px::new(r0.w.ceil() as u32 + 1, r0.h.ceil() as u32 + 1)?;
    let loc = |r: Rect| Rect::new(r.x - r0.x, r.y - r0.y, r.w, r.h);
    zoompad::fond(&mut pm, loc(r0));
    let ink = zoompad::ENCRE;
    let hovered = mouse.and_then(|(x, y)| bar.hit(x, y));
    for (r, b) in &bar.btns {
        let lr = loc(*r);
        let on = open == Some(*b) || (*b == DbBtn::Lock && d.locked());
        if on {
            round_fill(&mut pm, lr, lr.h / 2.0, zoompad::CHOISI.0, zoompad::CHOISI.1);
        } else if hovered == Some(*b) && *b != DbBtn::Handle {
            zoompad::survol(&mut pm, lr);
        }
        let (cx, cy) = (lr.cx(), lr.cy());
        match b {
            DbBtn::Handle => {
                for k in 0..6 {
                    let (dx, dy) = ((k % 2) as f64 * 5.0 - 2.5, (k / 2) as f64 * 5.0 - 5.0);
                    round_fill(&mut pm, Rect::new(cx + dx - 1.2, cy + dy - 1.2, 2.4, 2.4), 1.2, crate::palette::DIM, 1.0);
                }
            }
            DbBtn::Template => {
                // four squares, the last one a « + »
                for (k, (dx, dy)) in [(-5.0, -5.0), (5.0, -5.0), (-5.0, 5.0)].iter().enumerate() {
                    let _ = k;
                    crate::shapes::round_stroke(&mut pm, Rect::new(cx + dx - 3.5, cy + dy - 3.5, 7.0, 7.0), 2.0, ink, 1.3);
                }
                lines(&mut pm, cx + 5.0, cy + 5.0, &[&[(-3.5, 0.0), (3.5, 0.0)], &[(0.0, -3.5), (0.0, 3.5)]], ink, 1.3);
            }
            DbBtn::LineColor | DbBtn::TextColor => {
                if *b == DbBtn::LineColor {
                    crate::phosphor::draw(&mut pm, "pencil-simple", cx, cy - 3.0, 18, ink);
                } else {
                    text.draw(&mut pm, "T", (cx as f32, (cy + 3.5) as f32), 17.0, ink, Align::Center);
                }
                let c = if *b == DbBtn::LineColor {
                    colour(&d.style.color, Rgb(0x29, 0x62, 0xff))
                } else {
                    colour(&d.style.tcolor, Rgb(0xff, 0xff, 0xff))
                };
                round_fill(&mut pm, Rect::new(cx - 9.0, cy + 8.0, 18.0, 3.5), 1.75, c, 1.0);
            }
            DbBtn::Width => {
                let w = d.style.width.round().clamp(1.0, 4.0) as f64;
                crate::fill(&mut pm, lr.x + 9.0, cy - w / 2.0, 14.0, w, ink, 1.0);
                text.draw(
                    &mut pm,
                    &format!("{}px", w as i32),
                    ((lr.x + 29.0) as f32, (cy + 5.0) as f32),
                    14.0,
                    ink,
                    Align::Left,
                );
            }
            DbBtn::Style => {
                let (seg, gap): (f64, f64) = if d.style.dotted {
                    (2.0, 3.0)
                } else if d.style.dash {
                    (5.0, 3.0)
                } else {
                    (20.0, 0.0)
                };
                let mut x = cx - 10.0;
                while x < cx + 10.0 {
                    crate::fill(&mut pm, x, cy - 0.75, seg.min(cx + 10.0 - x), 1.5, ink, 1.0);
                    x += seg + gap.max(0.01);
                }
            }
            DbBtn::Settings => {
                crate::phosphor::draw(&mut pm, "gear", cx, cy, 19, ink);
            }
            DbBtn::Alert => {
                crate::phosphor::draw(&mut pm, "alarm", cx - 1.0, cy, 19, ink);
                lines(&mut pm, cx + 7.0, cy + 6.0, &[&[(-3.0, 0.0), (3.0, 0.0)], &[(0.0, -3.0), (0.0, 3.0)]], ink, 1.4);
            }
            DbBtn::Lock => {
                crate::icons::draw(&mut pm, if d.locked() { "lock2" } else { "unlock2" }, cx, cy, 20, ink);
            }
            DbBtn::Trash => {
                crate::phosphor::draw(&mut pm, "trash", cx, cy, 19, ink);
            }
            DbBtn::More => {
                for dx in [-6.0, 0.0, 6.0] {
                    crate::shapes::round_stroke(&mut pm, Rect::new(cx + dx - 2.0, cy - 2.0, 4.0, 4.0), 2.0, ink, 1.2);
                }
            }
        }
    }
    Some(pm)
}

// — the menus of the bar —

/// What a line of a bar menu does.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DbPick {
    /// Width in points (1 to 4).
    Width(f32),
    /// 0 solid, 1 dashed, 2 dotted.
    Dash(u8),
    /// An action of the drawing's menu (front, back, intervals, clone, copy, hide).
    Obj(crate::objmenu::ObjAction),
    /// 0 no sync, 1 in the layout, 2 global.
    Sync(u8),
    /// « Sauvegarder le modèle de dessin sous… » (asks a name).
    SaveTemplate,
    /// « Appliquer le modèle de dessin par défaut ».
    DefaultTemplate,
    /// A saved template of this tool (its rank), applied.
    Template(usize),
    /// ✕ of a saved template: removed.
    RemoveTemplate(usize),
    /// 📱 A folded button (in ⋯): open what it opens (template menu, text colour).
    Open(DbBtn),
}

/// 🖍 Applies a template's style to a drawing: everything but its text, its lock and its hiding.
pub fn apply_template(st: &mut crate::drawings::Style, t: &crate::drawings::Style) {
    let keep = (std::mem::take(&mut st.text), st.locked, st.hidden);
    *st = t.clone();
    (st.text, st.locked, st.hidden) = keep;
}

const OBJ: i64 = 100;
const TPL: i64 = 400;
const FOLD: i64 = 500;
const SYNC: i64 = 200;

/// The menu under a bar button (template, width, style, ⋯), lines as in Jee's captures; `templates`: the names of
/// the saved templates of the drawing's tool.
pub fn menu(b: DbBtn, d: &Drawing, templates: &[String], folded: &[DbBtn]) -> Vec<crate::listmenu::MenuGroup> {
    use crate::listmenu::{MenuGroup, MenuItem};
    let it = |id: i64, label: &str, checked: bool, hint: &str| MenuItem {
        id,
        label: label.into(),
        enabled: true,
        checked,
        hint: hint.into(),
        ..Default::default()
    };
    let group = |items: Vec<MenuItem>| MenuGroup {
        title: String::new(),
        items,
        strip: false,
    };
    let st = &d.style;
    match b {
        DbBtn::Template => {
            let mut v = vec![group(vec![
                it(300, "Sauvegarder le modèle de dessin sous…", false, ""),
                it(301, "Appliquer le modèle de dessin par défaut", false, ""),
            ])];
            if !templates.is_empty() {
                v.push(group(
                    templates
                        .iter()
                        .enumerate()
                        .map(|(k, n)| MenuItem {
                            remove: true,
                            ..it(TPL + k as i64, n, false, "")
                        })
                        .collect(),
                ));
            }
            v
        }
        DbBtn::Width => vec![group(
            (1..=4)
                .map(|w| it(w, &format!("{w}px"), (st.width.round() as i64).clamp(1, 4) == w, ""))
                .collect(),
        )],
        DbBtn::Style => vec![group(vec![
            it(10, "Droite", !st.dash && !st.dotted, ""),
            it(11, "Ligne traitillée", st.dash && !st.dotted, ""),
            it(12, "Ligne pointillée", st.dotted, ""),
        ])],
        DbBtn::More => {
            let mut v = vec![];
            if !folded.is_empty() {
                v.push(group(
                    folded
                        .iter()
                        .map(|f| {
                            if *f == DbBtn::Template {
                                it(FOLD, "Modèle de dessin…", false, "")
                            } else {
                                it(FOLD + 1, "Couleur du texte…", false, "")
                            }
                        })
                        .collect(),
                ));
            }
            v.extend(more_groups(st, d.tool == "rect", &it, &group));
            v
        }
        _ => vec![],
    }
}

/// The ⋯ menu's own lines.
fn more_groups(
    st: &crate::drawings::Style,
    rect: bool,
    it: &dyn Fn(i64, &str, bool, &str) -> crate::listmenu::MenuItem,
    group: &dyn Fn(Vec<crate::listmenu::MenuItem>) -> crate::listmenu::MenuGroup,
) -> Vec<crate::listmenu::MenuGroup> {
    use crate::listmenu::{MenuGroup, MenuItem};
    use crate::objmenu::ObjAction;
    vec![
        MenuGroup {
            title: "ORDRE DE VISUALISATION".into(),
            items: vec![
                it(OBJ + ObjAction::Front as i64, "Mettre au premier plan", false, ""),
                it(OBJ + ObjAction::Back as i64, "Mettre en arrière-plan", false, ""),
            ],
            strip: false,
        },
        group(vec![it(OBJ + ObjAction::Intervals as i64, "Visibilité des intervalles…", false, "")]),
        // (📱 the long press on a drawing shows this bar: the rest of the drawing's menu is here too)
        group(
            [
                Some(it(OBJ + ObjAction::Clone as i64, "Cloner", false, "Ctrl + Drag")),
                Some(it(OBJ + ObjAction::Copy as i64, "Copier", false, "Ctrl + C")),
                Some(it(OBJ + ObjAction::Invert as i64, "Inverser", false, "")),
                Some(MenuItem {
                    enabled: false,
                    ..it(OBJ + ObjAction::LimitOrder as i64, "Créer un ordre limite…  (bientôt)", false, "")
                }),
                rect.then(|| it(OBJ + ObjAction::Pinceau as i64, "🖌 Pinceau de zone", false, "")),
            ]
            .into_iter()
            .flatten()
            .collect(),
        ),
        group(vec![it(OBJ + ObjAction::Tree as i64, "Arborescence des objets…", false, "")]),
        group(vec![
            it(SYNC, "Pas de sync", st.sync == 0, ""),
            it(SYNC + 1, "Synchroniser dans la mise en page", st.sync == 1, ""),
            it(SYNC + 2, "Synchronisation globale", st.sync == 2, ""),
        ]),
        group(vec![MenuItem {
            icon: Some("eyeoff2"),
            ..it(OBJ + ObjAction::Hide as i64, "Cacher", false, "")
        }]),
    ]
}

/// The ✕ of the line `id` of the template menu: the template removed.
pub fn remove(b: DbBtn, id: i64) -> Option<DbPick> {
    (b == DbBtn::Template && id >= TPL).then(|| DbPick::RemoveTemplate((id - TPL) as usize))
}

/// What the line `id` of the menu of `b` does.
pub fn pick(b: DbBtn, id: i64) -> Option<DbPick> {
    match b {
        DbBtn::Template => match id {
            300 => Some(DbPick::SaveTemplate),
            301 => Some(DbPick::DefaultTemplate),
            k if k >= TPL => Some(DbPick::Template((k - TPL) as usize)),
            _ => None,
        },
        DbBtn::Width => (1..=4).contains(&id).then_some(DbPick::Width(id as f32)),
        DbBtn::Style => (10..=12).contains(&id).then(|| DbPick::Dash((id - 10) as u8)),
        DbBtn::More if id == FOLD => Some(DbPick::Open(DbBtn::Template)),
        DbBtn::More if id == FOLD + 1 => Some(DbPick::Open(DbBtn::TextColor)),
        DbBtn::More if (SYNC..=SYNC + 2).contains(&id) => Some(DbPick::Sync((id - SYNC) as u8)),
        DbBtn::More => crate::objmenu::ObjAction::from_id(id - OBJ).map(DbPick::Obj),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::TOOLS;

    /// Every drawing tool has its bar: the handle first, ⚙ padlock 🗑 ⋯ always, each button once; the lines have the
    /// whole bar (Jee's mockup); texts and markers have no width or line style.
    #[test]
    fn une_barre_pour_chaque_outil() {
        for t in TOOLS.iter().filter(|t| t.points > 0) {
            let b = buttons(t.id);
            assert_eq!(b[0], DbBtn::Handle, "{}", t.id);
            for k in [DbBtn::Template, DbBtn::Settings, DbBtn::Lock, DbBtn::Trash, DbBtn::More] {
                assert!(b.contains(&k), "{} : {k:?}", t.id);
            }
            for k in &b {
                assert_eq!(b.iter().filter(|x| *x == k).count(), 1, "{} : {k:?} twice", t.id);
            }
            if NO_LINE.contains(&t.id) {
                assert!(!b.contains(&DbBtn::Width) && !b.contains(&DbBtn::Style), "{}", t.id);
            }
        }
        use DbBtn::*;
        assert_eq!(
            buttons("hray"),
            vec![Handle, Template, LineColor, TextColor, Width, Style, Settings, Alert, Lock, Trash, More],
            "Rayon horizontal: the mockup's bar"
        );
        assert_eq!(tip(Lock, false), "Verrouiller");
        // the menus: each line does something
        let d = Drawing::hline(100.0);
        for b in [Template, Width, Style, More] {
            for g in menu(b, &d, &["Mon rayon or".to_string()], &[Template, TextColor]) {
                for i in g.items {
                    assert!(pick(b, i.id).is_some(), "{b:?} : {}", i.label);
                }
            }
        }
        assert_eq!(pick(Width, 3), Some(DbPick::Width(3.0)));
        assert_eq!(pick(Style, 12), Some(DbPick::Dash(2)));
        assert_eq!(pick(Template, TPL + 1), Some(DbPick::Template(1)));
        assert_eq!(remove(Template, TPL), Some(DbPick::RemoveTemplate(0)));
        // a template: everything but the text, the lock and the hiding
        let mut st = Drawing::hline(1.0).style;
        st.text = "mine".into();
        st.locked = true;
        let mut t = st.clone();
        t.text = "theirs".into();
        t.locked = false;
        t.width = 4.0;
        t.color = "#ff0000".into();
        apply_template(&mut st, &t);
        assert_eq!((st.text.as_str(), st.locked, st.width, st.color.as_str()), ("mine", true, 4.0, "#ff0000"));
        assert_eq!(tip(Lock, true), "Déverrouiller");
    }

    /// The bar stays whole in the chart, above the drawing (below if no room), and each button is found where it is.
    #[test]
    fn barre_dans_la_chart_et_clics() {
        let b = layout("hray", (400.0, 300.0), (0.0, 0.0), 1200.0, 700.0, false);
        assert!(b.rect.y + b.rect.h <= 300.0 && (b.rect.cx() - 400.0).abs() < 0.5, "{:?}", b.rect);
        let up = layout("hray", (400.0, 10.0), (0.0, 0.0), 1200.0, 700.0, false);
        assert!(up.rect.y > 10.0, "no room above: below the drawing");
        let edge = layout("hray", (5.0, 300.0), (0.0, 0.0), 1200.0, 700.0, false);
        assert!(edge.rect.x >= 6.0);
        for (r, k) in &b.btns {
            assert_eq!(b.hit(r.cx(), r.cy()), Some(*k));
        }
        // 📱 iPhone 17 Pro (402 points): one row inside the screen, the template and text colour in ⋯
        let p = layout("hray", (200.0, 400.0), (0.0, 0.0), 402.0, 700.0, true);
        assert!(p.rect.x >= 6.0 && p.rect.x + p.rect.w <= 396.0, "{:?}", p.rect);
        assert_eq!(p.folded, vec![DbBtn::Template, DbBtn::TextColor]);
        assert_eq!(pick(DbBtn::More, FOLD), Some(DbPick::Open(DbBtn::Template)));
        // a tablet keeps the whole bar
        assert!(layout("hray", (400.0, 400.0), (0.0, 0.0), 1100.0, 700.0, true).folded.is_empty());
        // touch: zones at least 40 points wide (the handle 32), 38 high
        let t = layout("hray", (200.0, 400.0), (0.0, 0.0), 402.0, 700.0, true);
        for (r, k) in &t.btns {
            assert!(r.h >= 38.0 && (r.w >= 40.0 || *k == DbBtn::Handle), "{k:?} {r:?}");
        }
    }
}
